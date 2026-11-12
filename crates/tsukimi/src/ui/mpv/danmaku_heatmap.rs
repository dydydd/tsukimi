use gtk::{
    glib,
    prelude::*,
};

mod imp {
    use std::cell::RefCell;

    use super::*;
    use glib::Properties;

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
                    let bar_w = width as f64 / n as f64;
                    let max = densities.iter().copied().fold(0.0f32, f32::max).max(1e-6);
                    for (i, &d) in densities.iter().enumerate() {
                        let t = (d / max).clamp(0.0, 1.0) as f64;
                        // color gradient: low -> gray/blue, high -> orange/red
                        let r = 0.3 + 0.7 * t;
                        let g = 0.4 + 0.2 * (1.0 - t);
                        let b = 0.8 * (1.0 - t);
                        cr.set_source_rgba(r, g, b, 0.35 + 0.55 * t);
                        let x = i as f64 * bar_w;
                        let h = height as f64 * t.max(0.05);
                        let _ = cr.rectangle(x, height as f64 - h, bar_w.max(1.0), h);
                        let _ = cr.fill();
                    }
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

// end of file