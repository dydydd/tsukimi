use gtk::{
    glib,
    prelude::*,
    subclass::prelude::*,
};

mod imp {
    use std::cell::RefCell;

    use super::*;
    use glib::Properties;
    use gtk::subclass::prelude::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::DanmakuHeatmap)]
    pub struct DanmakuHeatmap {
        pub densities: RefCell<Vec<f32>>,
        pub duration: RefCell<f64>,
        pub position: RefCell<f64>,
        pub seek_cb: RefCell<Option<Box<dyn Fn(f64) + 'static>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DanmakuHeatmap {
        const NAME: &'static str = "DanmakuHeatmap";
        type Type = super::DanmakuHeatmap;
        type ParentType = gtk::DrawingArea;
    }

    #[glib::derived_properties]
    impl ObjectImpl for DanmakuHeatmap {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj().clone();
            obj.set_draw_func(glib::clone!(
                #[weak]
                obj,
                move |_, cr, width, height| {
                    let imp = obj.imp();
                    let densities = imp.densities.borrow();
                    let n = densities.len();
                    if n == 0 || width <= 0 || height <= 0 {
                        return;
                    }
                    let max = densities.iter().copied().fold(0.0f32, f32::max).max(1e-6);
                    let step = width as f64 / (n as f64 - 1.0).max(1.0);
                    // smooth curve points
                    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(n);
                    for (i, &d) in densities.iter().enumerate() {
                        let t = (d / max).clamp(0.0, 1.0) as f64;
                        let x = i as f64 * step;
                        let y = height as f64 * (1.0 - t * 0.9);
                        pts.push((x, y));
                    }
                    // fill under curve
                    cr.move_to(pts[0].0, height as f64);
                    for &(x, y) in &pts {
                        cr.line_to(x, y);
                    }
                    cr.line_to(pts.last().unwrap().0, height as f64);
                    cr.close_path();
                    // parse accent color
                    let accent = crate::ui::models::SETTINGS.accent_color_code();
                    let (ar, ag, ab) = parse_hex_color(&accent).unwrap_or((0.68, 0.71, 0.98));
                    let pat = gtk::cairo::LinearGradient::new(0.0, 0.0, 0.0, height as f64);
                    pat.add_color_stop_rgba(0.0, ar, ag, ab, 0.9);
                    pat.add_color_stop_rgba(1.0, ar * 0.35, ag * 0.35, ab * 0.35, 0.25);
                    let _ = cr.set_source(pat);
                    let _ = cr.fill();
                    // stroke curve
                    cr.move_to(pts[0].0, pts[0].1);
                    for &(x, y) in pts.iter().skip(1) {
                        cr.line_to(x, y);
                    }
                    cr.set_source_rgba(ar, ag, ab, 0.95);
                    cr.set_line_width(1.5);
                    let _ = cr.stroke();
                    // position indicator
                    let dur = *imp.duration.borrow();
                    let pos = *imp.position.borrow();
                    if dur > 0.0 {
                        let x = (pos / dur) * width as f64;
                        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
                        let _ = cr.rectangle(x - 1.0, 0.0, 2.0, height as f64);
                        let _ = cr.fill();
                    }
                }
            ));
            // click to seek
            let click = gtk::GestureClick::new();
            click.connect_pressed(glib::clone!(
                #[weak]
                obj,
                move |_, _, x, _| {
                    let imp = obj.imp();
                    let dur = *imp.duration.borrow();
                    let w = obj.width() as f64;
                    if dur > 0.0 && w > 0.0 {
                        let t = (x / w) * dur;
                        if let Some(cb) = obj.imp().seek_cb.borrow().as_ref() {
                            cb(t);
                        }
                    }
                }
            ));
            obj.add_controller(click);
        }
    }

    impl WidgetImpl for DanmakuHeatmap {}
    impl DrawingAreaImpl for DanmakuHeatmap {}
}

glib::wrapper! {
    pub struct DanmakuHeatmap(ObjectSubclass<imp::DanmakuHeatmap>)
        @extends gtk::Widget, gtk::DrawingArea, @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl DanmakuHeatmap {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn set_data(&self, densities: Vec<f32>, duration: f64) {
        let imp = self.imp();
        *imp.densities.borrow_mut() = densities;
        *imp.duration.borrow_mut() = duration;
        self.queue_draw();
    }

    pub fn set_position(&self, position: f64) {
        *self.imp().position.borrow_mut() = position;
        self.queue_draw();
    }

    pub fn clear(&self) {
        self.imp().densities.borrow_mut().clear();
        self.queue_draw();
    }

    pub fn set_seek_callback<F: Fn(f64) + 'static>(&self, f: F) {
        *self.imp().seek_cb.borrow_mut() = Some(Box::new(f));
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

// end of file