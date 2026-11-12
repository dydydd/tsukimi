use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
};

use mutsumi::*;

mod imp {
    use std::cell::{
        Cell,
        RefCell,
    };

    use gtk::{
        glib,
        prelude::*,
        subclass::prelude::*,
    };

    use crate::ui::mpv::sink::MPVPlaySink;

    #[derive(Default, glib::Properties)]
    #[properties(wrapper_type = super::VideoScale)]
    pub struct VideoScale {
        #[property(get, set = Self::set_player, explicit_notify, nullable)]
        pub player: glib::WeakRef<MPVPlaySink>,

        pub is_dragging: Cell<bool>,

        pub heatmap_densities: RefCell<Vec<f32>>,
        pub heatmap_duration: Cell<f64>,
        pub heatmap_position: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VideoScale {
        const NAME: &'static str = "VideoScale";
        type Type = super::VideoScale;
        type ParentType = gtk::Scale;
    }

    #[glib::derived_properties]
    impl ObjectImpl for VideoScale {
        fn constructed(&self) {
            self.parent_constructed();
            // new GestureClick with add_controller is doesn't work for connect_released
            //
            // so we need to iterate through the controllers to get the GestureClick
            // and then connect the signals
            let mut gesture = gtk::GestureClick::new();
            self.obj()
                .observe_controllers()
                .into_iter()
                .for_each(|collection| {
                    if let Ok(event) = collection
                        && event.type_() == gtk::GestureClick::static_type()
                    {
                        gesture = event.downcast::<gtk::GestureClick>().unwrap();
                    }
                });

            gesture.connect_pressed(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, _, _| {
                    imp.on_click_pressed();
                }
            ));

            gesture.connect_released(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, _, _, _| {
                    imp.on_click_released();
                }
            ));
        }
    }
    impl WidgetImpl for VideoScale {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            let densities = self.heatmap_densities.borrow();
            if !densities.is_empty() {
                let width = obj.width() as f64;
                let height = obj.height() as f64;
                if width > 0.0 && height > 0.0 {
                    let max = densities.iter().copied().fold(0.0f32, f32::max).max(1e-6);
                    let n = densities.len();
                    let step = width / (n as f64 - 1.0).max(1.0);
                    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(n);
                    for (i, &d) in densities.iter().enumerate() {
                        let t = (d / max).clamp(0.0, 1.0) as f64;
                        let x = i as f64 * step;
                        let y = height * 0.5 - (height * 0.4 * t);
                        pts.push((x, y));
                    }
                    let accent = crate::ui::models::SETTINGS.accent_color_code();
                    let (ar, ag, ab) = super::parse_hex_color(&accent).unwrap_or((0.68, 0.71, 0.98));
                    let band_top = height * 0.15;
                    let band_bottom = height * 0.85;
                    let cairo_rect = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
                    let cr = snapshot.append_cairo(&cairo_rect);
                    // fill under curve
                    cr.move_to(pts[0].0, band_bottom);
                        for &(x, y) in &pts {
                            cr.line_to(x, y.clamp(band_top, band_bottom));
                        }
                        cr.line_to(pts.last().unwrap().0, band_bottom);
                        cr.close_path();
                        let pat = gtk::cairo::LinearGradient::new(0.0, band_top, 0.0, band_bottom);
                        pat.add_color_stop_rgba(0.0, ar, ag, ab, 0.7);
                        pat.add_color_stop_rgba(1.0, ar * 0.3, ag * 0.3, ab * 0.3, 0.25);
                        let _ = cr.set_source(pat);
                        let _ = cr.fill();
                        // stroke
                        cr.move_to(pts[0].0, pts[0].1.clamp(band_top, band_bottom));
                        for &(x, y) in pts.iter().skip(1) {
                            cr.line_to(x, y.clamp(band_top, band_bottom));
                        }
                        cr.set_source_rgba(ar, ag, ab, 0.9);
                        cr.set_line_width(1.2);
                        let _ = cr.stroke();
                    // position indicator
                    let dur = self.heatmap_duration.get();
                    let pos = self.heatmap_position.get();
                    if dur > 0.0 {
                        let x = (pos / dur) * width;
                        let cairo_rect = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
                        let cr = snapshot.append_cairo(&cairo_rect);
                        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
                        let _ = cr.rectangle(x - 1.0, band_top, 2.0, band_bottom - band_top);
                        let _ = cr.fill();
                    }
                }
            }
            self.parent_snapshot(snapshot);
        }
    }

    impl RangeImpl for VideoScale {}
    impl ScaleImpl for VideoScale {}

    impl VideoScale {
        fn set_player(&self, player: Option<MPVPlaySink>) {
            if self.player.upgrade() == player {
                return;
            }
            self.player.set(player.as_ref());
        }

        fn on_click_pressed(&self) {
            self.is_dragging.set(true);
        }

        fn on_click_released(&self) {
            let obj = self.obj();
            self.on_seek_finished(obj.value());
            self.is_dragging.set(false);
        }

        fn on_seek_finished(&self, value: f64) {
            self.player.upgrade().unwrap().set_position(value);
        }
    }
}

glib::wrapper! {
    pub struct VideoScale(ObjectSubclass<imp::VideoScale>)
        @extends gtk::Widget, gtk::Scale, gtk::Range, @implements gtk::Accessible, gtk::Buildable, gtk::Orientable, gtk::ConstraintTarget;
}

impl Default for VideoScale {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoScale {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_heatmap_data(&self, densities: Vec<f32>, duration: f64) {
        let imp = self.imp();
        *imp.heatmap_densities.borrow_mut() = densities;
        imp.heatmap_duration.set(duration);
        self.queue_draw();
    }

    pub fn set_heatmap_position(&self, position: f64) {
        self.imp().heatmap_position.set(position);
        self.queue_draw();
    }

    pub fn clear_heatmap(&self) {
        self.imp().heatmap_densities.borrow_mut().clear();
        self.queue_draw();
    }

    pub fn update_position_callback(&self) -> glib::ControlFlow {
        let position = &self.player().unwrap().position();
        if *position > 0.0 {
            self.set_value(*position);
        }
        glib::ControlFlow::Continue
    }

    pub fn set_cache_end_time(&self, end_time: i64) {
        self.set_fill_level(end_time as f64);
    }

    pub fn reset_scale(&self) {
        self.set_value(0.0);
        self.set_fill_level(0.0);
    }

    pub fn is_dragging(&self) -> bool {
        self.imp().is_dragging.get()
    }

    pub fn set_chapter_list(&self, chapter_list: ChapterList) {
        self.clear_marks();

        for chapter in chapter_list {
            self.add_mark(chapter.time, gtk::PositionType::Top, None);
        }
    }
}

fn parse_hex_color(hex: &str) -> Option<(f64, f64, f64)> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()? as f64 / 255.0;
    let g = u8::from_str_radix(&h[2..4], 16).ok()? as f64 / 255.0;
    let b = u8::from_str_radix(&h[4..6], 16).ok()? as f64 / 255.0;
    Some((r, g, b))
}
