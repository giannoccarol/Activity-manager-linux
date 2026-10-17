//! Vetro vero su Wayland: il compositor (KWin) sfoca ciò che sta dietro la finestra trasparente
//! (protocollo standard ext-background-effect-v1). Tutto gira sul thread di GTK.
use gtk::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use wayland_backend::client::{Backend, ObjectId};
use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_compositor::WlCompositor, wl_region::WlRegion, wl_registry::WlRegistry, wl_surface::WlSurface};
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};
use wayland_protocols::ext::background_effect::v1::client::{
    ext_background_effect_manager_v1::ExtBackgroundEffectManagerV1 as Manager, ext_background_effect_surface_v1::ExtBackgroundEffectSurfaceV1 as Effect,
};

struct St;

macro_rules! no_events {
    ($($t:ty => $d:ty),*) => {$(
        impl Dispatch<$t, $d> for St {
            fn event(_: &mut Self, _: &$t, _: <$t as Proxy>::Event, _: &$d, _: &Connection, _: &QueueHandle<Self>) {}
        }
    )*};
}
no_events!(WlRegistry => GlobalListContents, WlCompositor => (), WlRegion => (), Manager => (), Effect => ());

struct Blur {
    conn: Connection,
    queue: EventQueue<St>,
    compositor: WlCompositor,
    _manager: Manager,
    effect: Effect,
}

thread_local! {
    static BLURS: RefCell<HashMap<String, Blur>> = RefCell::new(HashMap::new());
}

fn attach(win: &gtk::ApplicationWindow) -> Result<Blur, String> {
    let gw = win.window().ok_or("finestra non ancora realizzata")?;
    let display = gw.display();
    let kind = display.type_().name();
    if kind != "GdkWaylandDisplay" {
        return Err(format!("display {kind}, non Wayland"));
    }
    unsafe {
        use gtk::glib::translate::ToGlibPtr;
        let d: *mut gtk::gdk::ffi::GdkDisplay = display.to_glib_none().0;
        let w: *mut gtk::gdk::ffi::GdkWindow = gw.to_glib_none().0;
        let wl_display = gdk_wayland_sys::gdk_wayland_display_get_wl_display(d as *mut _);
        let wl_surface = gdk_wayland_sys::gdk_wayland_window_get_wl_surface(w as *mut _);
        if wl_display.is_null() || wl_surface.is_null() {
            return Err("superficie Wayland assente".into());
        }
        let conn = Connection::from_backend(Backend::from_foreign_display(wl_display as *mut _));
        let id = ObjectId::from_ptr(WlSurface::interface(), wl_surface as *mut _).map_err(|e| format!("id superficie: {e:?}"))?;
        let surface = WlSurface::from_id(&conn, id).map_err(|e| format!("superficie: {e:?}"))?;
        let (globals, queue) = registry_queue_init::<St>(&conn).map_err(|e| format!("registro: {e:?}"))?;
        let qh = queue.handle();
        let manager: Manager = globals.bind(&qh, 1..=1, ()).map_err(|e| format!("ext_background_effect: {e:?}"))?;
        let compositor: WlCompositor = globals.bind(&qh, 1..=4, ()).map_err(|e| format!("compositor: {e:?}"))?;
        let effect = manager.get_background_effect(&surface, &qh, ());
        Ok(Blur { conn, queue, compositor, _manager: manager, effect })
    }
}

impl Blur {
    /// Area sfocata: rettangolo con angoli arrotondati, approssimati con righe di 1 px.
    fn set_region(&self, w: i32, h: i32, radius: i32) {
        let qh = self.queue.handle();
        let region = self.compositor.create_region(&qh, ());
        let r = radius.clamp(0, w.min(h) / 2);
        region.add(0, r, w, h - 2 * r);
        for y in 0..r {
            let dy = r as f64 - y as f64 - 0.5;
            let inset = (r as f64 - (r as f64 * r as f64 - dy * dy).max(0.0).sqrt()).round() as i32;
            region.add(inset, y, w - 2 * inset, 1);
            region.add(inset, h - 1 - y, w - 2 * inset, 1);
        }
        self.effect.set_blur_region(Some(&region));
        region.destroy();
        let _ = self.conn.flush();
    }
}

/// Attiva, aggiorna o toglie la sfocatura dietro una finestra. `radius` segue gli angoli disegnati in CSS.
pub fn apply(label: &str, win: &gtk::ApplicationWindow, on: bool, radius: i32) {
    // all'avvio la finestra può non essere ancora realizzata: riprova poco dopo
    if on && win.window().is_none() {
        let (label, win) = (label.to_string(), win.clone());
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(80), move || apply(&label, &win, on, radius));
        return;
    }
    BLURS.with(|b| {
        let mut map = b.borrow_mut();
        if !on {
            if let Some(x) = map.remove(label) {
                x.effect.destroy();
                let _ = x.conn.flush();
                win.queue_draw();
            }
            return;
        }
        if !map.contains_key(label) {
            match attach(win) {
                Ok(x) => {
                    map.insert(label.to_string(), x);
                }
                Err(e) => {
                    eprintln!("sfocatura non disponibile per {label}: {e}");
                    return;
                }
            }
        }
        if let (Some(x), Some(gw)) = (map.get(label), win.window()) {
            let full = gw.state().intersects(gtk::gdk::WindowState::MAXIMIZED | gtk::gdk::WindowState::FULLSCREEN | gtk::gdk::WindowState::TILED);
            x.set_region(gw.width(), gw.height(), if full { 0 } else { radius });
            // lo stato del blur si applica al prossimo commit della superficie: chiede un nuovo fotogramma
            win.queue_draw();
        }
    });
}
