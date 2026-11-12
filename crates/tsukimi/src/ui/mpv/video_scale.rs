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
            let width = obj.width() as f64;
            let height = obj.height() as f64;
            if !densities.is_empty() && width > 0.0 && height > 0.0 {
                let pts = super::heatmap_points(&densities, width, height);
                if pts.len() > 1 {
                    let accent = crate::ui::models::SETTINGS.accent_color_code();
                    let (ar, ag, ab) =
                        super::parse_hex_color(&accent).unwrap_or((0.68, 0.71, 0.98));
                    let baseline = super::heatmap_baseline(height);
                    let cairo_rect =
                        gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
                    let cr = snapshot.append_cairo(&cairo_rect);

                    // filled area between the curve and the seek bar
                    super::trace_curve(&cr, &pts);
                    cr.line_to(pts.last().unwrap().0, baseline);
                    cr.line_to(pts[0].0, baseline);
                    cr.close_path();
                    let pat = gtk::cairo::LinearGradient::new(0.0, 0.0, 0.0, baseline);
                    pat.add_color_stop_rgba(0.0, ar, ag, ab, 0.38);
                    pat.add_color_stop_rgba(1.0, ar, ag, ab, 0.06);
                    let _ = cr.set_source(pat);
                    let _ = cr.fill();

                    // the curve itself
                    super::trace_curve(&cr, &pts);
                    cr.set_source_rgba(ar, ag, ab, 0.95);
                    cr.set_line_width(1.2);
                    let _ = cr.stroke();
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

    pub fn set_heatmap_data(&self, densities: Vec<f32>) {
        *self.imp().heatmap_densities.borrow_mut() = densities;
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

/// Vertical position of the seek bar track inside the widget.
fn heatmap_baseline(height: f64) -> f64 {
    (height * 0.5 - 2.0).max(3.0)
}

/// Turn the per-bucket danmaku counts into a smoothed curve sitting on the seek bar.
fn heatmap_points(densities: &[f32], width: f64, height: f64) -> Vec<(f64, f64)> {
    let n = densities.len();
    if n < 2 {
        return Vec::new();
    }

    let mut values: Vec<f64> = densities.iter().map(|&d| d as f64).collect();
    let max = values.iter().cloned().fold(0.0f64, f64::max).max(1e-6);
    for value in values.iter_mut() {
        *value = (*value / max).clamp(0.0, 1.0);
    }

    // one blur pass turns the histogram into a wave
    let raw = values.clone();
    for (i, value) in values.iter_mut().enumerate() {
        let left = raw[i.saturating_sub(1)];
        let right = raw[(i + 1).min(n - 1)];
        *value = 0.25 * left + 0.5 * raw[i] + 0.25 * right;
    }

    // keep the curve clear of the handle area on both ends
    let inset = 9.0;
    let baseline = heatmap_baseline(height);
    let amplitude = (baseline - 2.0).max(1.0);
    let step = (width - inset * 2.0) / (n as f64 - 1.0);

    values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            // lift quiet passages a little so the wave stays readable
            let h = (0.85 * v + 0.15 * v.sqrt()).clamp(0.0, 1.0);
            (inset + i as f64 * step, baseline - amplitude * h)
        })
        .collect()
}

/// Trace a smooth curve through `pts` using Catmull-Rom control points.
fn trace_curve(cr: &gtk::cairo::Context, pts: &[(f64, f64)]) {
    let n = pts.len();
    cr.move_to(pts[0].0, pts[0].1);
    for i in 0..n - 1 {
        let p0 = pts[i.saturating_sub(1)];
        let p1 = pts[i];
        let p2 = pts[i + 1];
        let p3 = pts[(i + 2).min(n - 1)];
        let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
        cr.curve_to(c1.0, c1.1, c2.0, c2.1, p2.0, p2.1);
    }
}
