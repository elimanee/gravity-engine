//! Window movement tracking for the "window shake" effect.
//!
//! The window position is polled on a background thread (X11, KWin over
//! DBus on KDE Wayland, or Win32) so blocking platform calls never stall a
//! frame.

use crate::config::PPM;
use rapier2d::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

struct Shared {
    pos: Mutex<Option<(f32, f32)>>,
    enabled: AtomicBool,
}

pub struct WindowTracker {
    shared: Arc<Shared>,
    prev: Option<(f32, f32)>,
}

impl WindowTracker {
    pub fn start() -> Self {
        let shared = Arc::new(Shared { pos: Mutex::new(None), enabled: AtomicBool::new(true) });
        #[cfg(target_os = "linux")]
        {
            let weak = Arc::downgrade(&shared);
            std::thread::Builder::new().name("window-tracker".into()).spawn(move || linux::poll_loop(weak)).ok();
        }
        #[cfg(windows)]
        {
            let weak = Arc::downgrade(&shared);
            std::thread::Builder::new().name("window-tracker".into()).spawn(move || win32::poll_loop(weak)).ok();
        }
        WindowTracker { shared, prev: None }
    }

    /// Pause polling while the effect is disabled.
    pub fn set_enabled(&mut self, on: bool) {
        self.shared.enabled.store(on, Ordering::Relaxed);
        if !on {
            self.prev = None;
        }
    }

    /// Push every dynamic body opposite to the window's motion since the last call.
    pub fn apply(&mut self, bodies: &mut RigidBodySet, sensitivity: f32) {
        let Some(cur) = self.shared.pos.lock().ok().and_then(|p| *p) else { return };
        let Some(prev) = self.prev.replace(cur) else { return };
        let (dx, dy) = (cur.0 - prev.0, cur.1 - prev.1);
        if dx.abs() < 0.5 && dy.abs() < 0.5 {
            return;
        }
        // Ignore teleports (workspace switches, maximise…).
        if dx.abs() > 400.0 || dy.abs() > 400.0 {
            return;
        }
        let dv = vector![-dx / PPM * sensitivity, dy / PPM * sensitivity];
        for (_, b) in bodies.iter_mut() {
            if b.is_dynamic() {
                let v = *b.linvel();
                b.set_linvel(v + dv, true);
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::Shared;
    use std::sync::atomic::Ordering;
    use std::sync::Weak;
    use std::time::Duration;

    pub fn poll_loop(shared: Weak<Shared>) {
        let dpy = unsafe { x11::xlib::XOpenDisplay(std::ptr::null()) };
        let kde = if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            zbus::blocking::Connection::session().ok().filter(|c| kde_pos(c).is_some() || kde_active(c))
        } else {
            None
        };
        if dpy.is_null() && kde.is_none() {
            return;
        }

        while let Some(s) = shared.upgrade() {
            if s.enabled.load(Ordering::Relaxed) {
                let pos = kde.as_ref().and_then(kde_pos).or_else(|| unsafe { x11_pos(dpy) });
                if let Ok(mut p) = s.pos.lock() {
                    *p = pos;
                }
            }
            drop(s);
            std::thread::sleep(Duration::from_millis(8));
        }

        if !dpy.is_null() {
            unsafe { x11::xlib::XCloseDisplay(dpy) };
        }
    }

    fn kde_active(conn: &zbus::blocking::Connection) -> bool {
        conn.call_method(Some("org.kde.KWin"), "/KWin", Some("org.kde.KWin"), "activeClient", &()).is_ok()
    }

    fn kde_pos(conn: &zbus::blocking::Connection) -> Option<(f32, f32)> {
        use std::collections::HashMap;
        use zbus::zvariant::OwnedValue;

        let id: u32 = conn
            .call_method(Some("org.kde.KWin"), "/KWin", Some("org.kde.KWin"), "activeClient", &())
            .ok()?
            .body()
            .deserialize()
            .ok()?;
        if id == 0 {
            return None;
        }
        let info: HashMap<String, OwnedValue> = conn
            .call_method(Some("org.kde.KWin"), "/KWin", Some("org.kde.KWin"), "getWindowInfo", &(id,))
            .ok()?
            .body()
            .deserialize()
            .ok()?;
        let x = i32::try_from(info.get("x")?).ok()?;
        let y = i32::try_from(info.get("y")?).ok()?;
        Some((x as f32, y as f32))
    }

    /// Top-level frame position of the focused window.
    unsafe fn x11_pos(dpy: *mut x11::xlib::Display) -> Option<(f32, f32)> {
        use x11::xlib::*;
        if dpy.is_null() {
            return None;
        }
        let mut win: Window = 0;
        let mut revert = 0i32;
        XGetInputFocus(dpy, &mut win, &mut revert);
        if win <= 1 {
            return None;
        }
        let root = XDefaultRootWindow(dpy);
        let mut w = win;
        for _ in 0..8 {
            let (mut parent, mut rw): (Window, Window) = (0, 0);
            let mut children: *mut Window = std::ptr::null_mut();
            let mut n: u32 = 0;
            if XQueryTree(dpy, w, &mut rw, &mut parent, &mut children, &mut n) == 0 {
                break;
            }
            if !children.is_null() {
                XFree(children as *mut _);
            }
            if parent == root || parent == 0 {
                break;
            }
            w = parent;
        }
        let (mut x, mut y) = (0i32, 0i32);
        let mut child: Window = 0;
        XTranslateCoordinates(dpy, w, root, 0, 0, &mut x, &mut y, &mut child);
        Some((x as f32, y as f32))
    }
}

#[cfg(windows)]
mod win32 {
    use super::Shared;
    use std::sync::atomic::Ordering;
    use std::sync::Weak;
    use std::time::Duration;
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowRect};

    pub fn poll_loop(shared: Weak<Shared>) {
        let title: Vec<u16> = crate::config::APP_NAME.encode_utf16().chain(Some(0)).collect();
        while let Some(s) = shared.upgrade() {
            if s.enabled.load(Ordering::Relaxed) {
                let pos = unsafe {
                    let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
                    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
                    (!hwnd.is_null() && GetWindowRect(hwnd, &mut r) != 0).then_some((r.left as f32, r.top as f32))
                };
                if let Ok(mut p) = s.pos.lock() {
                    *p = pos;
                }
            }
            drop(s);
            std::thread::sleep(Duration::from_millis(8));
        }
    }
}
