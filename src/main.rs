/*  Gravity Engine — Rust / macroquad + rapier2d
    ================================================
    Drop images (PNG / JPG / GIF) onto the window.
    Each image becomes a rigid body with real physics.

    Controls
    ─────────────────────────────────────────────────
    A key                open file picker
    Left-drag            throw object
    Right-click          context menu
      ├─ Resize ▶        50 % / 75 % / 100 % / 150 % / 200 %
      ├─ Copy & Paste
      ├─ Size All ▶      50 % / 75 % / 100 % / 150 % / 200 %
      └─ Delete
    Gravity buttons      ZERO / MOON / MARS / EARTH / JUPITER / HEAVY / REVERSE
    M                    load tracker module (.mod / .xm / .it / .s3m / …)
    P                    pause / resume module playback
    D                    toggle debug overlay
    R                    clear all
    Space                pause / unpause physics
    Q / Esc              quit
*/

#![allow(clippy::too_many_arguments)]

use macroquad::prelude::*;
use rfd::FileDialog;
use rapier2d::prelude::*;
use rapier2d::na;
use std::sync::{Arc, Mutex};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::BufReader;
use std::collections::VecDeque;

// ═══════════════════════════════════════════════════════════════
// Window movement tracker — shakes rigid bodies when window moves
// Supports X11 and KDE Wayland (via KWin DBus).
// ═══════════════════════════════════════════════════════════════
struct WindowTracker {
    prev_x: f32,
    prev_y: f32,
    #[cfg(target_os = "linux")]
    x11_dpy: *mut x11::xlib::Display,
    #[cfg(target_os = "linux")]
    kde:     Option<zbus::blocking::Connection>, // Some on KDE Wayland
}

unsafe impl Send for WindowTracker {}

impl WindowTracker {
    fn new() -> Self {
        #[cfg(not(target_os = "linux"))]
        return WindowTracker { prev_x: 0.0, prev_y: 0.0 };

        #[cfg(target_os = "linux")]
        {
            let x11_dpy = unsafe { x11::xlib::XOpenDisplay(std::ptr::null()) };

            // On Wayland, try to connect to KWin DBus
            let kde = if std::env::var("WAYLAND_DISPLAY").is_ok() {
                zbus::blocking::Connection::session().ok().filter(|conn| {
                    conn.call_method(
                        Some("org.kde.KWin"), "/KWin",
                        Some("org.kde.KWin"), "activeClient", &(),
                    ).is_ok()
                })
            } else {
                None
            };

            let mut t = WindowTracker { x11_dpy, kde, prev_x: 0.0, prev_y: 0.0 };
            let (px, py) = t.get_pos();
            t.prev_x = px;
            t.prev_y = py;
            t
        }
    }

    fn get_pos(&self) -> (f32, f32) {
        #[cfg(target_os = "linux")]
        {
            if let Some(ref conn) = self.kde {
                if let Some(pos) = Self::kde_pos(conn) { return pos; }
            }
            return unsafe { Self::x11_pos(self.x11_dpy) };
        }
        #[cfg(not(target_os = "linux"))]
        (self.prev_x, self.prev_y)
    }

    #[cfg(target_os = "linux")]
    fn kde_pos(conn: &zbus::blocking::Connection) -> Option<(f32, f32)> {
        use std::collections::HashMap;
        use zbus::zvariant::OwnedValue;

        // Get focused window ID from KWin
        let id: u32 = conn.call_method(
            Some("org.kde.KWin"), "/KWin",
            Some("org.kde.KWin"), "activeClient", &(),
        ).ok()?.body().deserialize().ok()?;
        if id == 0 { return None; }

        // Get window geometry from KWin
        let info: HashMap<String, OwnedValue> = conn.call_method(
            Some("org.kde.KWin"), "/KWin",
            Some("org.kde.KWin"), "getWindowInfo", &(id,),
        ).ok()?.body().deserialize().ok()?;

        let x: i32 = i32::try_from(info.get("x")?).ok()?;
        let y: i32 = i32::try_from(info.get("y")?).ok()?;
        Some((x as f32, y as f32))
    }

    #[cfg(target_os = "linux")]
    unsafe fn x11_pos(dpy: *mut x11::xlib::Display) -> (f32, f32) {
        use x11::xlib::*;
        if dpy.is_null() { return (0.0, 0.0); }
        let mut win: Window = 0;
        let mut revert = 0i32;
        XGetInputFocus(dpy, &mut win, &mut revert);
        if win <= 1 { return (0.0, 0.0); }
        let root = XDefaultRootWindow(dpy);
        let mut w = win;
        for _ in 0..8 {
            let mut parent: Window = 0;
            let mut rw:     Window = 0;
            let mut children: *mut Window = std::ptr::null_mut();
            let mut n: u32 = 0;
            XQueryTree(dpy, w, &mut rw, &mut parent, &mut children, &mut n);
            if !children.is_null() { XFree(children as *mut _); }
            if parent == root || parent == 0 { break; }
            w = parent;
        }
        let mut x = 0i32;
        let mut y = 0i32;
        let mut child: Window = 0;
        XTranslateCoordinates(dpy, w, root, 0, 0, &mut x, &mut y, &mut child);
        (x as f32, y as f32)
    }

    fn tick(&mut self, bodies: &mut RigidBodySet, sensitivity: f32) {
        let (cx, cy) = self.get_pos();
        let dx = cx - self.prev_x;
        let dy = cy - self.prev_y;
        self.prev_x = cx;
        self.prev_y = cy;

        if dx.abs() < 0.5 && dy.abs() < 0.5 { return; }

        let vx = -dx / PPM * sensitivity;
        let vy =  dy / PPM * sensitivity;

        for (_, body) in bodies.iter_mut() {
            if body.is_dynamic() {
                let v = *body.linvel();
                body.set_linvel(vector![v.x + vx, v.y + vy], true);
                body.wake_up(true);
            }
        }
    }
}

impl Drop for WindowTracker {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        unsafe {
            if !self.x11_dpy.is_null() {
                x11::xlib::XCloseDisplay(self.x11_dpy);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════
const PPM:        f32 = 60.0;
const BOUNCE:     f32 = 0.45;
const FRICTION:   f32 = 0.5;
const DENSITY:    f32 = 1.0;
const WALL_T:     f32 = 0.5;
const MAX_IMG_PX: u32 = 220;
const HUD_H:      f32 = 80.0;

const GRAVITY_PRESETS: &[(&str, f32, Color)] = &[
    ("ZERO",     0.0,   Color { r: 0.30, g: 0.28, b: 0.50, a: 1.0 }),
    ("MOON",    -1.6,   Color { r: 0.32, g: 0.30, b: 0.56, a: 1.0 }),
    ("MARS",    -3.7,   Color { r: 0.55, g: 0.30, b: 0.18, a: 1.0 }),
    ("EARTH",   -9.8,   Color { r: 0.20, g: 0.45, b: 0.25, a: 1.0 }),
    ("JUPITER", -24.8,  Color { r: 0.60, g: 0.42, b: 0.16, a: 1.0 }),
    ("HEAVY",   -40.0,  Color { r: 0.62, g: 0.18, b: 0.18, a: 1.0 }),
    ("REVERSE",  12.0,  Color { r: 0.28, g: 0.20, b: 0.65, a: 1.0 }),
];

// ═══════════════════════════════════════════════════════════════
// Coordinate conversion  (physics ↔ screen)
// ═══════════════════════════════════════════════════════════════
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let i = (h * 6.0) as u32;
    let f = h * 6.0 - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i % 6 {
        0 => (v, t, p), 1 => (q, v, p), 2 => (p, v, t),
        3 => (p, q, v), 4 => (t, p, v), _ => (v, p, q),
    };
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn to_phys(px: f32, py: f32) -> (f32, f32) {
    (px / PPM, (screen_height() - py) / PPM)
}

fn to_screen(bx: f32, by: f32) -> (f32, f32) {
    (bx * PPM, screen_height() - by * PPM)
}

// ═══════════════════════════════════════════════════════════════
// GIF frame loader
// ═══════════════════════════════════════════════════════════════
struct GifFrames {
    frames:    Vec<Texture2D>,
    delays_ms: Vec<u16>,
}

fn load_gif(path: &str, max_px: u32) -> Option<GifFrames> {
    let data = std::fs::read(path).ok()?;
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(std::io::Cursor::new(&data)).ok()?;

    let orig_w = decoder.width()  as u32;
    let orig_h = decoder.height() as u32;
    let scale  = (max_px as f32 / orig_w.max(orig_h) as f32).min(1.0);
    let out_w  = ((orig_w as f32 * scale) as u32).max(1);
    let out_h  = ((orig_h as f32 * scale) as u32).max(1);

    let mut frames    = Vec::new();
    let mut delays_ms = Vec::new();
    let mut canvas    = vec![0u8; (orig_w * orig_h * 4) as usize];

    while let Some(frame) = decoder.read_next_frame().ok().flatten() {
        let delay_ms = (frame.delay as u16).max(2) * 10;
        let fx = frame.left  as usize;
        let fy = frame.top   as usize;
        let fw = frame.width as usize;
        for row in 0..frame.height as usize {
            for col in 0..frame.width as usize {
                let src_i = (row * fw + col) * 4;
                let dst_x = fx + col;
                let dst_y = fy + row;
                let dst_i = (dst_y * orig_w as usize + dst_x) * 4;
                if dst_i + 3 < canvas.len() && src_i + 3 < frame.buffer.len()
                    && frame.buffer[src_i + 3] > 0
                {
                    canvas[dst_i..dst_i + 4]
                        .copy_from_slice(&frame.buffer[src_i..src_i + 4]);
                }
            }
        }
        let src_img = image::RgbaImage::from_raw(orig_w, orig_h, canvas.clone())?;
        let resized  = image::imageops::resize(
            &src_img, out_w, out_h, image::imageops::FilterType::Triangle);
        let tex = Texture2D::from_rgba8(out_w as u16, out_h as u16, &resized);
        tex.set_filter(FilterMode::Linear);
        frames.push(tex);
        delays_ms.push(delay_ms);
    }

    if frames.is_empty() { None } else { Some(GifFrames { frames, delays_ms }) }
}

// ═══════════════════════════════════════════════════════════════
// Static image loader
// ═══════════════════════════════════════════════════════════════
fn load_static_image_raw(path: &str, max_px: u32) -> Option<(Texture2D, image::RgbaImage)> {
    let data = std::fs::read(path).ok()?;
    let img  = image::load_from_memory(&data).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    let scale  = (max_px as f32 / w.max(h) as f32).min(1.0);
    let nw = ((w as f32 * scale) as u32).max(1);
    let nh = ((h as f32 * scale) as u32).max(1);
    let resized = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
    let tex = Texture2D::from_rgba8(nw as u16, nh as u16, &resized);
    tex.set_filter(FilterMode::Linear);
    Some((tex, resized))
}

fn load_static_image(path: &str, max_px: u32) -> Option<Texture2D> {
    load_static_image_raw(path, max_px).map(|(t, _)| t)
}

fn decode_image_bytes(data: &[u8], max_px: u32) -> Option<(Texture2D, image::RgbaImage)> {
    let img = image::load_from_memory(data).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    let scale  = (max_px as f32 / w.max(h) as f32).min(1.0);
    let nw = ((w as f32 * scale) as u32).max(1);
    let nh = ((h as f32 * scale) as u32).max(1);
    let resized = image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle);
    let tex = Texture2D::from_rgba8(nw as u16, nh as u16, &resized);
    tex.set_filter(FilterMode::Linear);
    Some((tex, resized))
}

fn decode_gif_bytes(data: &[u8], max_px: u32) -> Option<GifFrames> {
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder.read_info(std::io::Cursor::new(data)).ok()?;
    let orig_w = decoder.width()  as u32;
    let orig_h = decoder.height() as u32;
    let scale  = (max_px as f32 / orig_w.max(orig_h) as f32).min(1.0);
    let out_w  = ((orig_w as f32 * scale) as u32).max(1);
    let out_h  = ((orig_h as f32 * scale) as u32).max(1);
    let mut frames    = Vec::new();
    let mut delays_ms = Vec::new();
    let mut canvas    = vec![0u8; (orig_w * orig_h * 4) as usize];
    while let Some(frame) = decoder.read_next_frame().ok().flatten() {
        let delay_ms = (frame.delay as u16).max(2) * 10;
        let fx = frame.left  as usize;
        let fy = frame.top   as usize;
        let fw = frame.width as usize;
        for row in 0..frame.height as usize {
            for col in 0..frame.width as usize {
                let src_i = (row * fw + col) * 4;
                let dst_x = fx + col;
                let dst_y = fy + row;
                let dst_i = (dst_y * orig_w as usize + dst_x) * 4;
                if dst_i + 3 < canvas.len() && src_i + 3 < frame.buffer.len()
                    && frame.buffer[src_i + 3] > 0
                {
                    canvas[dst_i..dst_i + 4].copy_from_slice(&frame.buffer[src_i..src_i + 4]);
                }
            }
        }
        let src_img = image::RgbaImage::from_raw(orig_w, orig_h, canvas.clone())?;
        let resized  = image::imageops::resize(&src_img, out_w, out_h,
                                               image::imageops::FilterType::Triangle);
        let tex = Texture2D::from_rgba8(out_w as u16, out_h as u16, &resized);
        tex.set_filter(FilterMode::Linear);
        frames.push(tex);
        delays_ms.push(delay_ms);
    }
    if frames.is_empty() { None } else { Some(GifFrames { frames, delays_ms }) }
}

// ── 88×31 button fetcher ────────────────────────────────────────
fn scrape_urls(page_url: &str, base: &str) -> Vec<String> {
    let html = match ureq::get(page_url)
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
    {
        Some(h) => h,
        None    => return Vec::new(),
    };
    let mut urls = Vec::new();
    for chunk in html.split("src=\"") {
        let end = match chunk.find('"') { Some(i) => i, None => continue };
        let src = &chunk[..end];
        if !src.ends_with(".gif") && !src.ends_with(".png") && !src.ends_with(".jpg") {
            continue;
        }
        let url = if src.starts_with("http") {
            src.to_string()
        } else {
            format!("{}{}", base, src.trim_start_matches("./").trim_start_matches('/'))
        };
        if !urls.contains(&url) { urls.push(url); }
    }
    urls
}

fn fetch_88x31(tx: std::sync::mpsc::Sender<(String, Vec<u8>)>, count: usize) {
    std::thread::spawn(move || {
        // Scrape both galleries and merge the lists
        let mut urls = scrape_urls(
            "https://cyber.dabamos.de/88x31/",
            "https://cyber.dabamos.de/88x31/",
        );
        urls.extend(scrape_urls(
            "https://hellnet.work/8831/",
            "https://hellnet.work/8831/",
        ));
        urls.dedup();

        if urls.is_empty() { return; }

        // Simple LCG shuffle (no rand crate available in threads)
        let n = urls.len();
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;
        let mut s = seed;
        let mut indices: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let j = (s >> 33) as usize % (i + 1);
            indices.swap(i, j);
        }

        use std::io::Read;
        for &idx in indices.iter().take(count) {
            let url = &urls[idx];
            if let Ok(resp) = ureq::get(url)
                .timeout(std::time::Duration::from_secs(8))
                .call()
            {
                let mut bytes = Vec::new();
                if resp.into_reader().take(512_000).read_to_end(&mut bytes).is_ok()
                    && !bytes.is_empty()
                {
                    let name = url.split('/').last().unwrap_or("button.gif").to_string();
                    if tx.send((name, bytes)).is_err() { break; }
                }
            }
        }
    });
}

// Rasterise an SVG at up to max_px on the longest side.
// tiny_skia outputs premultiplied RGBA so we unmultiply before handing the
// bytes to macroquad (which expects straight alpha).
fn load_svg(path: &str, max_px: u32) -> Option<(Texture2D, image::RgbaImage)> {
    let data    = std::fs::read(path).ok()?;
    let options = resvg::usvg::Options::default();
    let tree    = resvg::usvg::Tree::from_data(&data, &options).ok()?;
    let sz      = tree.size();
    let (sw, sh) = (sz.width(), sz.height());
    let scale   = (max_px as f32 / sw.max(sh)).min(1.0);
    let ow      = ((sw * scale) as u32).max(1);
    let oh      = ((sh * scale) as u32).max(1);
    let mut pm  = resvg::tiny_skia::Pixmap::new(ow, oh)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(ow as f32 / sw, oh as f32 / sh),
        &mut pm.as_mut(),
    );
    // Unmultiply premultiplied alpha → straight alpha
    let mut bytes = pm.data().to_vec();
    for px in bytes.chunks_exact_mut(4) {
        let a = px[3] as f32 / 255.0;
        if a > 0.0 {
            px[0] = (px[0] as f32 / a).min(255.0) as u8;
            px[1] = (px[1] as f32 / a).min(255.0) as u8;
            px[2] = (px[2] as f32 / a).min(255.0) as u8;
        }
    }
    let rgba = image::RgbaImage::from_raw(ow, oh, bytes.clone())?;
    let tex  = Texture2D::from_rgba8(ow as u16, oh as u16, &bytes);
    tex.set_filter(FilterMode::Linear);
    Some((tex, rgba))
}

// Sample opaque pixel positions from a resized image and return them as
// normalised hull points in the range [-0.5, 0.5] × [-0.5, 0.5].
// Returns None when the image has no meaningful transparency (JPEG etc.)
// so the caller falls back to a plain cuboid collider.
fn compute_hull_pts(img: &image::RgbaImage) -> Option<Vec<[f32; 2]>> {
    let (w, h) = img.dimensions();
    let step   = (w.min(h) / 48).max(1) as usize;
    let pts: Vec<[f32; 2]> = (0..h as usize).step_by(step)
        .flat_map(|y| {
            (0..w as usize).step_by(step).filter_map(move |x| {
                if img.get_pixel(x as u32, y as u32)[3] > 20 {
                    let u = x as f32 / (w as f32 - 1.0) - 0.5;
                    let v = 0.5 - y as f32 / (h as f32 - 1.0);
                    Some([u, v])
                } else {
                    None
                }
            })
        })
        .collect();
    if pts.len() < 3 { None } else { Some(pts) }
}

// Build a collider from hull points (PNG cutout) or fall back to a cuboid.
// Hull points are stored normalised in [-0.5, 0.5]; scaled by 2×half to get
// physics coordinates.  density is passed explicitly so resize can preserve mass.
fn make_shape_collider(
    hull_pts: &Option<Vec<[f32; 2]>>,
    half_w:   f32,
    half_h:   f32,
    density:  f32,
) -> Collider {
    let hw = half_w.max(0.01);
    let hh = half_h.max(0.01);
    let builder = if let Some(ref pts) = hull_pts {
        let phys: Vec<na::Point2<f32>> = pts.iter()
            .map(|&[u, v]| na::Point2::new(u * 2.0 * hw, v * 2.0 * hh))
            .collect();
        ColliderBuilder::convex_hull(&phys)
            .unwrap_or_else(|| ColliderBuilder::cuboid(hw, hh))
    } else {
        ColliderBuilder::cuboid(hw, hh)
    };
    builder.density(density).friction(FRICTION).restitution(BOUNCE).build()
}

// ═══════════════════════════════════════════════════════════════
// PhysicsImage — one image + its rigid body
// ═══════════════════════════════════════════════════════════════
const TRAIL_MAX: usize = 40; // hard upper bound on trail history

struct PhysicsImage {
    path:            String,
    texture:         Texture2D,
    gif:             Option<GifFrames>,
    frame_idx:       usize,
    frame_timer:     f32,
    body_handle:     RigidBodyHandle,
    collider_handle: ColliderHandle,
    half_w:          f32,
    half_h:          f32,
    fixed_mass:      f32,
    trail:           VecDeque<(Vec2, f32, Texture2D, f64)>, // (screen pos, rotation, frame, timestamp)
    hull_pts:        Option<Vec<[f32; 2]>>,                // normalised convex-hull points for PNG cutouts
}

impl PhysicsImage {
    fn new(
        path: &str,
        target_w_px: Option<f32>,
        target_h_px: Option<f32>,
        spawn_px: (f32, f32),
        bodies:    &mut RigidBodySet,
        colliders: &mut ColliderSet,
    ) -> Option<Self> {
        let lp     = path.to_ascii_lowercase();
        let is_gif = lp.ends_with(".gif");
        let is_svg = lp.ends_with(".svg");

        let hull_pts: Option<Vec<[f32; 2]>>;
        let (texture, gif, tex_w, tex_h);
        if is_gif {
            let frames = load_gif(path, MAX_IMG_PX)?;
            tex_w    = frames.frames[0].width();
            tex_h    = frames.frames[0].height();
            texture  = frames.frames[0].clone();
            gif      = Some(frames);
            hull_pts = None;
        } else if is_svg {
            let (t, raw) = load_svg(path, MAX_IMG_PX)?;
            tex_w    = t.width();
            tex_h    = t.height();
            texture  = t;
            gif      = None;
            hull_pts = compute_hull_pts(&raw); // SVGs always have a transparent background
        } else {
            let (t, raw) = load_static_image_raw(path, MAX_IMG_PX)?;
            tex_w   = t.width();
            tex_h   = t.height();
            texture = t;
            gif     = None;
            // Skip hull only for formats that definitively have no alpha channel;
            // everything else (PNG, WebP, BMP, ICO, TGA, QOI, TIFF, EXR, DDS …)
            // may carry transparency, so let compute_hull_pts decide.
            let no_alpha = lp.ends_with(".jpg") || lp.ends_with(".jpeg")
                        || lp.ends_with(".hdr")
                        || lp.ends_with(".ppm") || lp.ends_with(".pgm")
                        || lp.ends_with(".pbm");
            hull_pts = if !no_alpha { compute_hull_pts(&raw) } else { None };
        }

        let (pw, ph) = match (target_w_px, target_h_px) {
            (Some(w), Some(h)) => (w, h),
            _                  => (tex_w, tex_h),
        };

        let half_w = pw / 2.0 / PPM;
        let half_h = ph / 2.0 / PPM;

        let (bx, by) = to_phys(spawn_px.0, spawn_px.1);
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![bx, by])
            .linvel(vector![rand::gen_range(-2.0f32, 2.0), 0.0])
            .angular_damping(0.6)
            .build();
        let body_handle = bodies.insert(body);

        let collider = make_shape_collider(&hull_pts, half_w, half_h, DENSITY);
        let collider_handle =
            colliders.insert_with_parent(collider, body_handle, bodies);

        let fixed_mass = bodies[body_handle].mass();

        Some(PhysicsImage {
            path: path.to_string(),
            texture,
            gif,
            frame_idx:   0,
            frame_timer: 0.0,
            body_handle,
            collider_handle,
            half_w,
            half_h,
            fixed_mass,
            trail: VecDeque::new(),
            hull_pts,
        })
    }

    fn texture_w(&self) -> f32 { self.half_w * 2.0 * PPM }
    fn texture_h(&self) -> f32 { self.half_h * 2.0 * PPM }

    fn from_bytes(
        name:      &str,
        data:      &[u8],
        spawn_px:  (f32, f32),
        bodies:    &mut RigidBodySet,
        colliders: &mut ColliderSet,
    ) -> Option<Self> {
        let is_gif = name.ends_with(".gif") || {
            // Detect by GIF magic bytes
            data.len() >= 6 && (&data[..6] == b"GIF87a" || &data[..6] == b"GIF89a")
        };

        let (texture, gif, tex_w, tex_h, hull_pts) = if is_gif {
            let frames = decode_gif_bytes(data, MAX_IMG_PX)?;
            let tw = frames.frames[0].width();
            let th = frames.frames[0].height();
            let tex = frames.frames[0].clone();
            (tex, Some(frames), tw, th, None)
        } else {
            let (tex, raw) = decode_image_bytes(data, MAX_IMG_PX)?;
            let tw = tex.width();
            let th = tex.height();
            let hull = compute_hull_pts(&raw);
            (tex, None, tw, th, hull)
        };

        let half_w = tex_w / 2.0 / PPM;
        let half_h = tex_h / 2.0 / PPM;
        let (bx, by) = to_phys(spawn_px.0, spawn_px.1);
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![bx, by])
            .linvel(vector![rand::gen_range(-1.0f32, 1.0), 0.0])
            .angular_damping(0.6)
            .build();
        let body_handle = bodies.insert(body);
        let collider = make_shape_collider(&hull_pts, half_w, half_h, DENSITY);
        let collider_handle = colliders.insert_with_parent(collider, body_handle, bodies);
        let fixed_mass = bodies[body_handle].mass();

        Some(PhysicsImage {
            path: name.to_string(),
            texture, gif,
            frame_idx: 0, frame_timer: 0.0,
            body_handle, collider_handle,
            half_w, half_h, fixed_mass,
            trail: VecDeque::new(),
            hull_pts,
        })
    }

    fn update_anim(&mut self, dt_ms: f32) {
        let Some(gif) = &self.gif else { return };
        self.frame_timer += dt_ms;
        loop {
            let delay = gif.delays_ms[self.frame_idx] as f32;
            if self.frame_timer < delay { break; }
            self.frame_timer -= delay;
            self.frame_idx    = (self.frame_idx + 1) % gif.frames.len();
        }
        if let Some(gif) = &self.gif {
            self.texture = gif.frames[self.frame_idx].clone();
        }
    }

    fn update_trail(&mut self, bodies: &RigidBodySet, max_len: usize, fade_secs: f32) {
        let now = get_time();
        // Evict points that have aged out
        while let Some(&(_, _, _, ts)) = self.trail.front() {
            if (now - ts) as f32 > fade_secs { self.trail.pop_front(); } else { break; }
        }
        // Trim if length slider was reduced
        while self.trail.len() > max_len { self.trail.pop_front(); }

        let body = &bodies[self.body_handle];
        let pos  = body.translation();
        let ang  = body.rotation().angle();
        let (sx, sy) = to_screen(pos.x, pos.y);
        let p = vec2(sx, sy);
        let far_enough = self.trail.back()
            .map_or(true, |&(last, _, _, _)| (last.x-p.x).powi(2) + (last.y-p.y).powi(2) >= 9.0);
        if far_enough {
            self.trail.push_back((p, ang, self.texture.clone(), now));
            if self.trail.len() > max_len { self.trail.pop_front(); }
        }
    }

    fn draw_trail(&self, fade_secs: f32) {
        if self.trail.is_empty() { return; }
        let now = get_time();
        let w   = self.texture_w();
        let h   = self.texture_h();
        for (pos, angle, tex, ts) in &self.trail {
            let age   = (now - ts) as f32;
            let youth = 1.0 - (age / fade_secs).clamp(0.0, 1.0); // 1=fresh, 0=expired
            let alpha = youth * youth * 0.40;
            if alpha < 0.005 { continue; }
            draw_texture_ex(
                tex,
                pos.x - w * 0.5,
                pos.y - h * 0.5,
                Color::new(1.0, 1.0, 1.0, alpha),
                DrawTextureParams {
                    dest_size: Some(vec2(w, h)),
                    rotation:  -angle,
                    ..Default::default()
                },
            );
        }
    }

    fn draw(&self, bodies: &RigidBodySet) {
        let body = &bodies[self.body_handle];
        let pos  = body.translation();
        let ang  = body.rotation().angle();
        let (sx, sy) = to_screen(pos.x, pos.y);
        let w = self.texture_w();
        let h = self.texture_h();
        draw_texture_ex(
            &self.texture,
            sx - w / 2.0,
            sy - h / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(w, h)),
                rotation:  -ang,
                ..Default::default()
            },
        );
    }

    fn resize(
        &mut self,
        factor: f32,
        bodies:    &mut RigidBodySet,
        colliders: &mut ColliderSet,
        islands:   &mut IslandManager,
    ) {
        let new_w = (self.texture_w() * factor).max(8.0);
        let new_h = (self.texture_h() * factor).max(8.0);
        self.half_w = new_w / 2.0 / PPM;
        self.half_h = new_h / 2.0 / PPM;

        let lp_r   = self.path.to_ascii_lowercase();
        let is_gif = lp_r.ends_with(".gif");
        let is_svg = lp_r.ends_with(".svg");
        if is_gif {
            if let Some(frames) = load_gif(&self.path, new_w.max(new_h) as u32) {
                self.texture   = frames.frames[0].clone();
                self.frame_idx = 0;
                self.gif       = Some(frames);
            }
        } else if is_svg {
            if let Some((t, _)) = load_svg(&self.path, new_w.max(new_h) as u32) {
                self.texture = t;
            }
        } else if let Some(t) = load_static_image(&self.path, new_w.max(new_h) as u32) {
            self.texture = t;
        }

        // Compute density so rapier derives the right mass AND non-zero angular
        // inertia from the new geometry. density(0) + set_additional_mass gives
        // zero total angular inertia, which snaps the body's rotation to 0.
        let area    = (2.0 * self.half_w.max(0.01)) * (2.0 * self.half_h.max(0.01));
        let density = self.fixed_mass / area;

        colliders.remove(self.collider_handle, islands, bodies, true);
        let new_col = make_shape_collider(&self.hull_pts, self.half_w, self.half_h, density);
        self.collider_handle =
            colliders.insert_with_parent(new_col, self.body_handle, bodies);
    }

    fn destroy(
        &self,
        bodies:           &mut RigidBodySet,
        colliders:        &mut ColliderSet,
        islands:          &mut IslandManager,
        impulse_joints:   &mut ImpulseJointSet,
        multibody_joints: &mut MultibodyJointSet,
    ) {
        bodies.remove(
            self.body_handle, islands, colliders,
            impulse_joints, multibody_joints, true,
        );
    }
}

// ═══════════════════════════════════════════════════════════════
// Context menu
// ═══════════════════════════════════════════════════════════════
#[derive(Debug, Clone, PartialEq)]
enum MenuAction {
    Resize(f32),
    SizeAll(f32),
    CopyPaste,
    Delete,
}

struct ContextMenu {
    visible:     bool,
    fade:        f32,   // 0.0=transparent → 1.0=opaque
    x:           f32,
    y:           f32,
    target_idx:  usize,
    // -1 = no submenu open; >= 0 = index of item whose submenu is open
    show_sub:    i32,
    hovered:     i32,
    sub_hovered: i32,
}

const MENU_W:      f32 = 200.0;
const MENU_ITEM_H: f32 = 32.0;
const MENU_FONT:   u16 = 15;

const RESIZE_STEPS: &[(&str, f32)] = &[
    ("  50%",  0.50),
    ("  75%",  0.75),
    ("  100%", 1.00),
    ("  150%", 1.50),
    ("  200%", 2.00),
];

// (label, has_submenu, action)
const TOP_ITEMS: &[(&str, bool, Option<MenuAction>)] = &[
    ("[R] Resize",       true,  None),
    ("[C] Copy & Paste", false, Some(MenuAction::CopyPaste)),
    ("[S] Size All",     true,  None),
    ("[X] Delete",       false, Some(MenuAction::Delete)),
];

impl ContextMenu {
    fn new() -> Self {
        ContextMenu {
            visible: false, fade: 0.0, x: 0.0, y: 0.0,
            target_idx: 0,
            show_sub: -1, hovered: -1, sub_hovered: -1,
        }
    }

    fn open(&mut self, x: f32, y: f32, idx: usize) {
        self.visible     = true;
        self.fade        = 0.0;
        self.x           = x.min(screen_width()  - MENU_W * 2.0 - 8.0);
        self.y           = y.min(screen_height() - self.total_h() - 4.0);
        self.target_idx  = idx;
        self.show_sub    = -1;
        self.hovered     = -1;
        self.sub_hovered = -1;
    }

    fn close(&mut self) { self.visible = false; self.fade = 0.0; }

    fn update(&mut self, dt: f32) {
        if self.visible {
            self.fade = (self.fade + dt * 10.0).min(1.0);
        }
    }

    fn total_h(&self) -> f32 {
        // items + 1 separator (10 px) + top/bottom padding (12 px)
        MENU_ITEM_H * TOP_ITEMS.len() as f32 + 10.0 + 12.0
    }

    fn item_rect(&self, idx: usize) -> Rect {
        // separator lives before the last item → add 10 px from that point
        let y_off = if idx + 1 == TOP_ITEMS.len() {
            6.0 + MENU_ITEM_H * idx as f32 + 10.0
        } else {
            6.0 + MENU_ITEM_H * idx as f32
        };
        Rect::new(self.x, self.y + y_off, MENU_W, MENU_ITEM_H)
    }

    fn sub_panel_y(&self) -> f32 {
        let sub_h    = RESIZE_STEPS.len() as f32 * MENU_ITEM_H + 12.0;
        let item_idx = self.show_sub.max(0) as usize;
        let ideal    = self.item_rect(item_idx).y;
        ideal.min(screen_height() - sub_h - 4.0)
    }

    fn sub_rect(&self, j: usize) -> Rect {
        Rect::new(
            self.x + MENU_W,
            self.sub_panel_y() + 6.0 + j as f32 * MENU_ITEM_H,
            MENU_W, MENU_ITEM_H,
        )
    }

    fn sub_panel_rect(&self) -> Rect {
        let sub_h = RESIZE_STEPS.len() as f32 * MENU_ITEM_H + 12.0;
        Rect::new(self.x + MENU_W, self.sub_panel_y(), MENU_W, sub_h)
    }

    fn handle_motion(&mut self, mx: f32, my: f32) {
        if !self.visible { return; }

        // Keep submenu open while mouse is over it or in the gap between panels
        if self.show_sub >= 0 {
            let sp  = self.sub_panel_rect();
            let par = self.item_rect(self.show_sub as usize);
            let gap = Rect::new(
                self.x + MENU_W, par.y,
                sp.x - (self.x + MENU_W) + sp.w, sp.h,
            );
            if sp.contains(vec2(mx, my)) || gap.contains(vec2(mx, my)) {
                self.sub_hovered = -1;
                for j in 0..RESIZE_STEPS.len() {
                    if self.sub_rect(j).contains(vec2(mx, my)) {
                        self.sub_hovered = j as i32;
                        break;
                    }
                }
                return;
            }
        }

        self.hovered     = -1;
        self.sub_hovered = -1;
        self.show_sub    = -1;
        for i in 0..TOP_ITEMS.len() {
            if self.item_rect(i).contains(vec2(mx, my)) {
                self.hovered  = i as i32;
                self.show_sub = if TOP_ITEMS[i].1 { i as i32 } else { -1 };
                break;
            }
        }
        if self.show_sub >= 0 {
            for j in 0..RESIZE_STEPS.len() {
                if self.sub_rect(j).contains(vec2(mx, my)) {
                    self.sub_hovered = j as i32;
                    break;
                }
            }
        }
    }

    fn handle_click(&mut self, mx: f32, my: f32) -> Option<(MenuAction, usize)> {
        if !self.visible { return None; }

        if self.show_sub >= 0 {
            for j in 0..RESIZE_STEPS.len() {
                if self.sub_rect(j).contains(vec2(mx, my)) {
                    let factor = RESIZE_STEPS[j].1;
                    let idx    = self.target_idx;
                    let action = if self.show_sub == 0 {
                        MenuAction::Resize(factor)
                    } else {
                        MenuAction::SizeAll(factor)
                    };
                    self.close();
                    return Some((action, idx));
                }
            }
        }

        for i in 0..TOP_ITEMS.len() {
            if self.item_rect(i).contains(vec2(mx, my)) {
                if TOP_ITEMS[i].1 { return None; } // submenu items open on hover
                let action = TOP_ITEMS[i].2.clone()?;
                let idx    = self.target_idx;
                self.close();
                return Some((action, idx));
            }
        }

        self.close();
        None
    }

    fn draw(&self) {
        if !self.visible || self.fade <= 0.001 { return; }

        let f  = self.fade;
        let ca = |r: u8, g: u8, b: u8, a: u8| -> Color {
            Color::from_rgba(r, g, b, (a as f32 * f) as u8)
        };
        let cw = Color { a: f, ..WHITE };

        let total_h = self.total_h();
        draw_rectangle(self.x, self.y, MENU_W, total_h,
                       ca(30, 28, 48, 245));
        draw_rectangle_lines(self.x, self.y, MENU_W, total_h, 1.0,
                             ca(80, 75, 130, 255));

        for i in 0..TOP_ITEMS.len() {
            let r   = self.item_rect(i);
            let hov = self.hovered == i as i32;

            if hov {
                draw_rectangle(r.x + 2.0, r.y + 2.0, r.w - 4.0, r.h - 4.0,
                               ca(60, 55, 100, 220));
            }

            // Separator before the last item (Delete)
            if i + 1 == TOP_ITEMS.len() {
                let sep_y = r.y - 6.0;
                draw_line(self.x + 8.0, sep_y, self.x + MENU_W - 8.0, sep_y,
                          1.0, ca(55, 50, 85, 255));
            }

            let col = if hov { cw } else { ca(180, 175, 210, 255) };
            draw_text(TOP_ITEMS[i].0,
                      r.x + 12.0,
                      r.y + r.h / 2.0 + MENU_FONT as f32 / 2.0 - 2.0,
                      MENU_FONT as f32, col);

            // Arrow for items with submenus
            if TOP_ITEMS[i].1 {
                let ac = if hov { cw } else { ca(120, 115, 160, 255) };
                let ax = self.x + MENU_W - 18.0;
                let ay = r.y + r.h / 2.0;
                draw_triangle(
                    vec2(ax, ay - 5.0),
                    vec2(ax, ay + 5.0),
                    vec2(ax + 8.0, ay),
                    ac,
                );
            }
        }

        // Submenu panel
        if self.show_sub >= 0 {
            let sub_h = RESIZE_STEPS.len() as f32 * MENU_ITEM_H + 12.0;
            let sub_x = self.x + MENU_W;
            let sub_y = self.sub_panel_y();

            draw_rectangle(sub_x, sub_y, MENU_W, sub_h,
                           ca(30, 28, 48, 245));
            draw_rectangle_lines(sub_x, sub_y, MENU_W, sub_h, 1.0,
                                 ca(80, 75, 130, 255));

            for (j, (label, _)) in RESIZE_STEPS.iter().enumerate() {
                let sr   = self.sub_rect(j);
                let shov = self.sub_hovered == j as i32;
                if shov {
                    draw_rectangle(sr.x + 2.0, sr.y + 2.0, sr.w - 4.0, sr.h - 4.0,
                                   ca(60, 55, 100, 220));
                }
                let col = if shov { cw } else { ca(180, 175, 210, 255) };
                draw_text(label,
                          sr.x + 12.0,
                          sr.y + sr.h / 2.0 + MENU_FONT as f32 / 2.0 - 2.0,
                          MENU_FONT as f32, col);
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Gravity preset buttons
// ═══════════════════════════════════════════════════════════════
struct GravityButtons {
    rects:  Vec<Rect>,
    active: i32,
}

impl GravityButtons {
    fn new() -> Self { GravityButtons { rects: vec![], active: 3 } }

    fn sync(&mut self, gravity_y: f32) {
        for (i, (_, g, _)) in GRAVITY_PRESETS.iter().enumerate() {
            if (gravity_y - g).abs() < 0.05 {
                self.active = i as i32;
                return;
            }
        }
        self.active = -1;
    }

    fn handle_click(&mut self, mx: f32, my: f32) -> Option<f32> {
        for (i, r) in self.rects.iter().enumerate() {
            if r.contains(vec2(mx, my)) {
                self.active = i as i32;
                return Some(GRAVITY_PRESETS[i].1);
            }
        }
        None
    }

    fn draw(&mut self, font_size: f32, gravity_y: f32, offset_y: f32) {
        self.rects.clear();
        self.sync(gravity_y);

        let btn_h  = 22.0;
        let pad    = 6.0;
        let gap    = 5.0;
        let margin = 10.0;
        let y0     = HUD_H - btn_h - 6.0 + offset_y;
        let mut x  = margin;

        for (i, (label, _, base_col)) in GRAVITY_PRESETS.iter().enumerate() {
            let txt_w = measure_text(label, None, font_size as u16, 1.0).width;
            let btn_w = txt_w + pad * 2.0;
            let rect  = Rect::new(x, y0, btn_w, btn_h);
            self.rects.push(rect);

            let is_active = self.active == i as i32;
            let bg = if is_active {
                Color::new(
                    (base_col.r + 0.25).min(1.0),
                    (base_col.g + 0.25).min(1.0),
                    (base_col.b + 0.25).min(1.0),
                    1.0,
                )
            } else {
                Color::new(base_col.r * 0.7, base_col.g * 0.7, base_col.b * 0.7, 0.85)
            };
            draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
            let border = if is_active {
                Color::from_rgba(220, 215, 255, 255)
            } else {
                Color::from_rgba(80, 75, 120, 200)
            };
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, border);

            let txt_col = if is_active { WHITE }
                          else { Color::from_rgba(180, 175, 210, 255) };
            draw_text(label,
                      rect.x + pad,
                      rect.y + btn_h / 2.0 + font_size / 2.0 - 2.0,
                      font_size, txt_col);

            x += btn_w + gap;
            if x > screen_width() - margin { break; }
        }

        let val_str = format!("g = {:+.1} m/s²", gravity_y);
        let vw      = measure_text(&val_str, None, font_size as u16, 1.0).width;
        draw_text(&val_str,
                  screen_width() - vw - margin,
                  y0 + btn_h / 2.0 + font_size / 2.0 - 2.0,
                  font_size,
                  Color::from_rgba(130, 125, 175, 255));
    }
}

// ═══════════════════════════════════════════════════════════════
// Side-border mode
// ═══════════════════════════════════════════════════════════════
#[derive(Clone, Copy, PartialEq)]
enum BorderMode { Default, Loop, Kill, Warp, Bounce, Repulse, Portal }

impl BorderMode {
    fn next(self) -> Self {
        match self {
            Self::Default => Self::Loop,
            Self::Loop    => Self::Kill,
            Self::Kill    => Self::Warp,
            Self::Warp    => Self::Bounce,
            Self::Bounce  => Self::Repulse,
            Self::Repulse => Self::Portal,
            Self::Portal  => Self::Default,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Default => "WALLS",
            Self::Loop    => "LOOP",
            Self::Kill    => "KILL",
            Self::Warp    => "WARP",
            Self::Bounce  => "BOUNCE",
            Self::Repulse => "REPULSE",
            Self::Portal  => "PORTAL",
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Physics world wrapper
// ═══════════════════════════════════════════════════════════════
struct PhysWorld {
    bodies:             RigidBodySet,
    colliders:          ColliderSet,
    gravity:            Vector<f32>,
    integration_params: IntegrationParameters,
    physics_pipeline:   PhysicsPipeline,
    island_manager:     IslandManager,
    broad_phase:        DefaultBroadPhase,
    narrow_phase:       NarrowPhase,
    impulse_joints:     ImpulseJointSet,
    multibody_joints:   MultibodyJointSet,
    ccd_solver:         CCDSolver,
    query_pipeline:     QueryPipeline,
    wall_handles:       Vec<RigidBodyHandle>,
    border_mode:        BorderMode,
}

impl PhysWorld {
    fn new(gravity_y: f32) -> Self {
        let mut pw = PhysWorld {
            bodies:             RigidBodySet::new(),
            colliders:          ColliderSet::new(),
            gravity:            vector![0.0, gravity_y],
            integration_params: IntegrationParameters::default(),
            physics_pipeline:   PhysicsPipeline::new(),
            island_manager:     IslandManager::new(),
            broad_phase:        DefaultBroadPhase::new(),
            narrow_phase:       NarrowPhase::new(),
            impulse_joints:     ImpulseJointSet::new(),
            multibody_joints:   MultibodyJointSet::new(),
            ccd_solver:         CCDSolver::new(),
            query_pipeline:     QueryPipeline::new(),
            wall_handles:       vec![],
            border_mode:        BorderMode::Default,
        };
        pw.rebuild_walls();
        pw
    }

    fn set_border_mode(&mut self, mode: BorderMode) {
        self.border_mode = mode;
        self.rebuild_walls();
    }

    fn rebuild_walls(&mut self) {
        for h in self.wall_handles.drain(..) {
            self.bodies.remove(
                h, &mut self.island_manager, &mut self.colliders,
                &mut self.impulse_joints, &mut self.multibody_joints, true);
        }

        let sw = screen_width()  / PPM;
        let sh = screen_height() / PPM;
        let hw = sw / 2.0;
        let hh = sh / 2.0;

        let mut walls = vec![];
        if self.border_mode != BorderMode::Portal {
            walls.push((vector![hw, WALL_T / 2.0], vector![hw + WALL_T, WALL_T / 2.0])); // floor
        }
        if matches!(self.border_mode, BorderMode::Default | BorderMode::Bounce | BorderMode::Repulse) {
            walls.push((vector![WALL_T / 2.0, hh],      vector![WALL_T / 2.0, hh + WALL_T])); // left
            walls.push((vector![sw - WALL_T / 2.0, hh], vector![WALL_T / 2.0, hh + WALL_T])); // right
        }

        for (pos, half_ext) in walls {
            let body = RigidBodyBuilder::fixed().translation(pos).build();
            let bh   = self.bodies.insert(body);
            let col  = ColliderBuilder::cuboid(half_ext.x, half_ext.y)
                .restitution(BOUNCE * 0.6)
                .friction(FRICTION)
                .build();
            self.colliders.insert_with_parent(col, bh, &mut self.bodies);
            self.wall_handles.push(bh);
        }
    }

    fn set_gravity(&mut self, gy: f32) {
        self.gravity = vector![0.0, gy];
        for (_, body) in self.bodies.iter_mut() {
            if body.is_dynamic() { body.wake_up(true); }
        }
    }

    fn step(&mut self) {
        self.physics_pipeline.step(
            &self.gravity,
            &self.integration_params,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            Some(&mut self.query_pipeline),
            &(),
            &(),
        );
    }

    fn body_at_px(&self, objects: &[PhysicsImage], px: f32, py: f32) -> Option<usize> {
        let (bx, by) = to_phys(px, py);
        let pt       = point![bx, by];
        for (i, obj) in objects.iter().enumerate() {
            if let Some(col) = self.colliders.get(obj.collider_handle) {
                if col.shape().contains_point(col.position(), &pt) {
                    return Some(i);
                }
            }
        }
        None
    }
}

// ═══════════════════════════════════════════════════════════════
// Object spawner
// ═══════════════════════════════════════════════════════════════
#[derive(Clone, Copy, PartialEq)]
enum SpawnShape { Circle, Box, Triangle, Pentagon, Star }

impl SpawnShape {
    fn label(self) -> &'static str {
        match self {
            Self::Circle   => "Circle",
            Self::Box      => "Box",
            Self::Triangle => "Triangle",
            Self::Pentagon => "Pentagon",
            Self::Star     => "Star",
        }
    }
    fn all() -> &'static [SpawnShape] {
        &[Self::Circle, Self::Box, Self::Triangle, Self::Pentagon, Self::Star]
    }
}

const SPAWN_COLORS: &[(u8, u8, u8)] = &[
    (220, 80,  80),   // red
    (80,  180, 80),   // green
    (80,  130, 230),  // blue
    (230, 180, 50),   // yellow
    (200, 80,  200),  // purple
    (80,  210, 200),  // cyan
    (240, 130, 50),   // orange
    (200, 200, 200),  // white
];

fn make_spawn_texture(shape: SpawnShape, size: u32, r: u8, g: u8, b: u8) -> (Texture2D, image::RgbaImage) {
    let s = size as i32;
    let cx = s / 2;
    let cy = s / 2;
    let mut img = image::RgbaImage::new(size, size);

    let fill = |img: &mut image::RgbaImage, px: i32, py: i32| {
        if px >= 0 && py >= 0 && px < s && py < s {
            img.put_pixel(px as u32, py as u32, image::Rgba([r, g, b, 255]));
        }
    };

    match shape {
        SpawnShape::Circle => {
            let r2 = (cx * cx) as f32;
            for py in 0..s {
                for px in 0..s {
                    let dx = px - cx;
                    let dy = py - cy;
                    if (dx*dx + dy*dy) as f32 <= r2 * 0.96 {
                        img.put_pixel(px as u32, py as u32, image::Rgba([r, g, b, 255]));
                    }
                }
            }
        }
        SpawnShape::Box => {
            let pad = (s as f32 * 0.05) as i32;
            for py in pad..s-pad {
                for px in pad..s-pad {
                    img.put_pixel(px as u32, py as u32, image::Rgba([r, g, b, 255]));
                }
            }
        }
        SpawnShape::Triangle => {
            // Equilateral triangle
            for py in 0..s {
                let t = py as f32 / (s - 1) as f32;
                let half = (t * cx as f32 * 0.94) as i32;
                let base_y = (s as f32 * 0.08) as i32;
                if py >= base_y {
                    for px in cx-half..=cx+half {
                        fill(&mut img, px, py);
                    }
                }
            }
        }
        SpawnShape::Pentagon => {
            let rf = cx as f32 * 0.9;
            let pts: Vec<(f32, f32)> = (0..5).map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 5.0 - std::f32::consts::FRAC_PI_2;
                (cx as f32 + a.cos() * rf, cy as f32 + a.sin() * rf)
            }).collect();
            for py in 0..s {
                for px in 0..s {
                    if point_in_polygon(px as f32, py as f32, &pts) {
                        img.put_pixel(px as u32, py as u32, image::Rgba([r, g, b, 255]));
                    }
                }
            }
        }
        SpawnShape::Star => {
            let ro = cx as f32 * 0.92;
            let ri = cx as f32 * 0.40;
            let pts: Vec<(f32, f32)> = (0..10).map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 10.0 - std::f32::consts::FRAC_PI_2;
                let rad = if i % 2 == 0 { ro } else { ri };
                (cx as f32 + a.cos() * rad, cy as f32 + a.sin() * rad)
            }).collect();
            for py in 0..s {
                for px in 0..s {
                    if point_in_polygon(px as f32, py as f32, &pts) {
                        img.put_pixel(px as u32, py as u32, image::Rgba([r, g, b, 255]));
                    }
                }
            }
        }
    }

    let tex = Texture2D::from_rgba8(size as u16, size as u16, &img);
    (tex, img)
}

fn point_in_polygon(px: f32, py: f32, pts: &[(f32, f32)]) -> bool {
    let n = pts.len();
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = pts[i];
        let (xj, yj) = pts[j];
        if ((yi > py) != (yj > py)) &&
           (px < (xj - xi) * (py - yi) / (yj - yi) + xi)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

struct Spawner {
    visible:    bool,
    fade:       f32,
    shape:      SpawnShape,
    size:       f32,      // diameter in pixels
    color_idx:  usize,
}

impl Spawner {
    fn new() -> Self {
        Spawner { visible: false, fade: 0.0, shape: SpawnShape::Circle, size: 80.0, color_idx: 0 }
    }

    fn toggle(&mut self) { self.visible = !self.visible; }

    fn update(&mut self, dt: f32) {
        let target = if self.visible { 1.0_f32 } else { 0.0 };
        self.fade += (target - self.fade) * (dt * 14.0).min(1.0);
        if self.fade < 0.005 { self.fade = 0.0; }
    }

    fn panel_rect(&self) -> Rect {
        let pw = 260.0_f32;
        let ph = 220.0_f32;
        Rect::new(
            (screen_width()  - pw) / 2.0,
            (screen_height() - ph) / 2.0 + 80.0,
            pw, ph,
        )
    }

    fn handle_click(&mut self, mx: f32, my: f32) -> bool {
        if self.fade < 0.01 { return false; }
        let p = self.panel_rect();
        if !p.contains(vec2(mx, my)) { return false; }

        // Shape buttons (row of 5)
        let shapes = SpawnShape::all();
        let btn_w = (p.w - 20.0) / shapes.len() as f32;
        for (i, &s) in shapes.iter().enumerate() {
            let r = Rect::new(p.x + 10.0 + i as f32 * btn_w, p.y + 30.0, btn_w - 4.0, 34.0);
            if r.contains(vec2(mx, my)) { self.shape = s; return true; }
        }

        // Color swatches
        let sw = 24.0_f32;
        let gap = 6.0_f32;
        let total = SPAWN_COLORS.len() as f32 * (sw + gap) - gap;
        let sx0 = p.x + (p.w - total) / 2.0;
        for (i, _) in SPAWN_COLORS.iter().enumerate() {
            let r = Rect::new(sx0 + i as f32 * (sw + gap), p.y + 100.0, sw, sw);
            if r.contains(vec2(mx, my)) { self.color_idx = i; return true; }
        }

        // Size slider (simple click → set)
        let track = Rect::new(p.x + 20.0, p.y + 158.0, p.w - 40.0, 10.0);
        if track.contains(vec2(mx, my)) {
            let t = ((mx - track.x) / track.w).clamp(0.0, 1.0);
            self.size = 30.0 + t * 150.0;
            return true;
        }

        true // consumed
    }

    fn draw(&self) {
        if self.fade < 0.01 { return; }
        let f = self.fade;
        let a = |v: u8| (v as f32 * f) as u8;
        let p = self.panel_rect();

        draw_rectangle(p.x, p.y, p.w, p.h, Color::from_rgba(12, 10, 24, a(245)));
        draw_rectangle_lines(p.x, p.y, p.w, p.h, 1.5, Color::from_rgba(80, 75, 140, a(255)));

        let title = "SPAWN OBJECT";
        let tw = measure_text(title, None, 14, 1.0).width;
        draw_text(title, p.x + (p.w - tw) / 2.0, p.y + 18.0, 14.0,
                  Color::from_rgba(170, 165, 220, a(210)));

        // Shape buttons
        let shapes = SpawnShape::all();
        let btn_w = (p.w - 20.0) / shapes.len() as f32;
        for (i, &s) in shapes.iter().enumerate() {
            let r = Rect::new(p.x + 10.0 + i as f32 * btn_w, p.y + 30.0, btn_w - 4.0, 34.0);
            let active = s == self.shape;
            let bg = if active { Color::from_rgba(55, 50, 115, a(240)) }
                     else      { Color::from_rgba(22, 19, 45, a(220)) };
            let border = if active { Color::from_rgba(160, 150, 255, a(255)) }
                         else      { Color::from_rgba(55, 50, 95, a(180)) };
            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, border);
            let lw = measure_text(s.label(), None, 12, 1.0).width;
            let tc = if active { Color::from_rgba(255, 255, 255, a(255)) }
                     else      { Color::from_rgba(140, 135, 190, a(255)) };
            draw_text(s.label(), r.x + (r.w - lw) / 2.0, r.y + 22.0, 12.0, tc);
        }

        // Color label
        draw_text("Color", p.x + 10.0, p.y + 92.0, 13.0,
                  Color::from_rgba(130, 125, 180, a(200)));

        // Color swatches
        let sw = 24.0_f32;
        let gap = 6.0_f32;
        let total = SPAWN_COLORS.len() as f32 * (sw + gap) - gap;
        let sx0 = p.x + (p.w - total) / 2.0;
        for (i, &(cr, cg, cb)) in SPAWN_COLORS.iter().enumerate() {
            let rx = sx0 + i as f32 * (sw + gap);
            let ry = p.y + 100.0;
            draw_rectangle(rx, ry, sw, sw, Color::from_rgba(cr, cg, cb, a(230)));
            if i == self.color_idx {
                draw_rectangle_lines(rx - 1.0, ry - 1.0, sw + 2.0, sw + 2.0, 2.0,
                                     Color::from_rgba(255, 255, 255, a(255)));
            }
        }

        // Size label + slider
        draw_text(&format!("Size  {}px", self.size as i32),
                  p.x + 10.0, p.y + 152.0, 13.0,
                  Color::from_rgba(130, 125, 180, a(200)));
        let track_x = p.x + 20.0;
        let track_y = p.y + 158.0;
        let track_w = p.w - 40.0;
        draw_rectangle(track_x, track_y, track_w, 6.0,
                       Color::from_rgba(40, 38, 70, a(255)));
        let t = (self.size - 30.0) / 150.0;
        draw_rectangle(track_x, track_y, track_w * t, 6.0,
                       Color::from_rgba(130, 120, 230, a(255)));
        draw_circle(track_x + track_w * t, track_y + 3.0, 7.0,
                    Color::from_rgba(180, 170, 255, a(255)));

        // Hint
        let hint = "Click in scene to spawn";
        let hw = measure_text(hint, None, 12, 1.0).width;
        draw_text(hint, p.x + (p.w - hw) / 2.0, p.y + 200.0, 12.0,
                  Color::from_rgba(100, 95, 150, a(180)));
    }

    fn spawn_object(
        &self,
        spawn_px: (f32, f32),
        bodies:    &mut RigidBodySet,
        colliders: &mut ColliderSet,
    ) -> Option<PhysicsImage> {
        let &(cr, cg, cb) = &SPAWN_COLORS[self.color_idx];
        let size = self.size as u32;
        let (texture, raw) = make_spawn_texture(self.shape, size, cr, cg, cb);
        let hull_pts = compute_hull_pts(&raw);

        let half_w = self.size / 2.0 / PPM;
        let half_h = self.size / 2.0 / PPM;

        let (bx, by) = to_phys(spawn_px.0, spawn_px.1);
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![bx, by])
            .angular_damping(0.6)
            .build();
        let body_handle = bodies.insert(body);

        let collider = make_shape_collider(&hull_pts, half_w, half_h, DENSITY);
        let collider_handle = colliders.insert_with_parent(collider, body_handle, bodies);
        let fixed_mass = bodies[body_handle].mass();

        Some(PhysicsImage {
            path: format!("__spawn__{}", self.shape.label()),
            texture,
            gif: None,
            frame_idx: 0,
            frame_timer: 0.0,
            body_handle,
            collider_handle,
            half_w,
            half_h,
            fixed_mass,
            trail: VecDeque::new(),
            hull_pts,
        })
    }
}

// ═══════════════════════════════════════════════════════════════
// Background
// ═══════════════════════════════════════════════════════════════
#[derive(Clone, Copy, PartialEq)]
enum BgMode { Dark, Space, Grid, Sunset, Ocean, Custom }

impl BgMode {
    fn next(self) -> Self {
        match self {
            Self::Dark   => Self::Space,
            Self::Space  => Self::Grid,
            Self::Grid   => Self::Sunset,
            Self::Sunset => Self::Ocean,
            Self::Ocean  => Self::Custom,
            Self::Custom => Self::Dark,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Dark   => "Dark",
            Self::Space  => "Space",
            Self::Grid   => "Grid",
            Self::Sunset => "Sunset",
            Self::Ocean  => "Ocean",
            Self::Custom => "Custom",
        }
    }
}

struct Background {
    mode:    BgMode,
    texture: Option<Texture2D>,
    // star positions (x, y, radius, twinkle-offset) generated once
    stars:   Vec<(f32, f32, f32, f32)>,
}

impl Background {
    fn new() -> Self {
        let stars = (0..220).map(|i| {
            let x = (i as f32 * 137.508).fract() * 4000.0;
            let y = (i as f32 * 97.312).fract()  * 2500.0;
            let r = (i as f32 * 53.1).fract() * 1.4 + 0.3;
            let o = (i as f32 * 23.7).fract() * std::f32::consts::TAU;
            (x, y, r, o)
        }).collect();
        Background { mode: BgMode::Dark, texture: None, stars }
    }

    fn draw(&self, sw: f32, sh: f32) {
        let t = get_time() as f32;
        match self.mode {
            BgMode::Dark => {
                clear_background(Color::from_rgba(15, 14, 25, 255));
                draw_rectangle(0.0, 0.0,      sw, sh / 2.0, Color::from_rgba(12, 12, 22, 255));
                draw_rectangle(0.0, sh / 2.0, sw, sh / 2.0, Color::from_rgba(28, 18, 48, 255));
            }
            BgMode::Space => {
                clear_background(Color::from_rgba(4, 4, 12, 255));
                // Nebula gradient
                draw_rectangle(0.0, 0.0, sw, sh * 0.6,
                               Color::from_rgba(8, 6, 22, 255));
                draw_rectangle(0.0, sh * 0.4, sw, sh * 0.6,
                               Color::from_rgba(18, 5, 30, 255));
                // Stars with twinkle
                for &(sx, sy, sr, off) in &self.stars {
                    let sx = sx % sw;
                    let sy = sy % sh;
                    let twinkle = ((t * 1.8 + off).sin() * 0.4 + 0.6).max(0.1);
                    let a = (twinkle * 220.0) as u8;
                    draw_circle(sx, sy, sr, Color::from_rgba(200, 210, 255, a));
                }
            }
            BgMode::Grid => {
                clear_background(Color::from_rgba(10, 10, 18, 255));
                let step = 48.0_f32;
                let col  = Color::from_rgba(35, 35, 60, 255);
                let col2 = Color::from_rgba(50, 50, 80, 255);
                let mut x = 0.0_f32;
                while x <= sw {
                    let thick = if (x / step) as i32 % 4 == 0 { 1.5 } else { 0.7 };
                    let c = if (x / step) as i32 % 4 == 0 { col2 } else { col };
                    draw_line(x, 0.0, x, sh, thick, c);
                    x += step;
                }
                let mut y = 0.0_f32;
                while y <= sh {
                    let thick = if (y / step) as i32 % 4 == 0 { 1.5 } else { 0.7 };
                    let c = if (y / step) as i32 % 4 == 0 { col2 } else { col };
                    draw_line(0.0, y, sw, y, thick, c);
                    y += step;
                }
            }
            BgMode::Sunset => {
                clear_background(Color::from_rgba(20, 10, 30, 255));
                // Sky gradient: deep purple → orange → warm pink
                draw_rectangle(0.0, 0.0,          sw, sh * 0.35, Color::from_rgba(20,  10,  50, 255));
                draw_rectangle(0.0, sh * 0.35,    sw, sh * 0.25, Color::from_rgba(130, 40,  60, 255));
                draw_rectangle(0.0, sh * 0.60,    sw, sh * 0.20, Color::from_rgba(220, 100, 30, 255));
                draw_rectangle(0.0, sh * 0.80,    sw, sh * 0.20, Color::from_rgba(240, 160, 40, 255));
                // Sun glow
                let sun_x = sw * 0.5;
                let sun_y = sh * 0.72;
                draw_circle(sun_x, sun_y, 60.0, Color::from_rgba(255, 220, 100, 200));
                draw_circle(sun_x, sun_y, 38.0, Color::from_rgba(255, 240, 180, 230));
                draw_circle(sun_x, sun_y, 20.0, Color::from_rgba(255, 255, 220, 255));
            }
            BgMode::Ocean => {
                clear_background(Color::from_rgba(5, 15, 35, 255));
                draw_rectangle(0.0, 0.0,       sw, sh * 0.4, Color::from_rgba(5,  18, 45, 255));
                draw_rectangle(0.0, sh * 0.4,  sw, sh * 0.3, Color::from_rgba(8,  35, 70, 255));
                draw_rectangle(0.0, sh * 0.7,  sw, sh * 0.3, Color::from_rgba(12, 55, 95, 255));
                // Animated caustic ripples
                for i in 0..6u8 {
                    let cx = sw * (0.15 + i as f32 * 0.14);
                    let cy = sh * (0.55 + (i as f32 * 0.07).sin() * 0.15);
                    let r  = 40.0 + (t * 1.2 + i as f32 * 1.1).sin() * 15.0;
                    let a  = (30.0 + (t * 1.5 + i as f32 * 0.9).sin() * 20.0) as u8;
                    draw_circle_lines(cx, cy, r, 1.0, Color::from_rgba(40, 130, 200, a));
                }
            }
            BgMode::Custom => {
                if let Some(ref tex) = self.texture {
                    draw_texture_ex(tex, 0.0, 0.0, WHITE, DrawTextureParams {
                        dest_size: Some(vec2(sw, sh)),
                        ..Default::default()
                    });
                } else {
                    // Fallback when no image loaded yet
                    clear_background(Color::from_rgba(15, 14, 25, 255));
                    let msg = "Press G to load a background image";
                    let mw  = measure_text(msg, None, 18, 1.0).width;
                    draw_text(msg, (sw - mw) / 2.0, sh / 2.0, 18.0,
                              Color::from_rgba(120, 115, 180, 200));
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Drag modes
// ═══════════════════════════════════════════════════════════════
#[derive(Clone, Copy, PartialEq)]
enum DragMode {
    Spring,        // spring force → throw on release
    Slingshot,     // pull back → launch opposite direction on release
    Pull,          // gravitational attraction field
    Push,          // repulsion field
    Vortex,        // rotational spin field around cursor
    Freeze,        // velocity damping field (slow objects down)
    Orbit,         // make objects circle the cursor
    Bomb,          // click to detonate an instant radial explosion
}

impl DragMode {
    fn label(self) -> &'static str {
        match self {
            Self::Spring    => "Spring",
            Self::Slingshot => "Slingshot",
            Self::Pull      => "Pull",
            Self::Push      => "Push",
            Self::Vortex    => "Vortex",
            Self::Freeze    => "Freeze",
            Self::Orbit     => "Orbit",
            Self::Bomb      => "Bomb",
        }
    }
    fn next(self) -> Self {
        match self {
            Self::Spring    => Self::Slingshot,
            Self::Slingshot => Self::Pull,
            Self::Pull      => Self::Push,
            Self::Push      => Self::Vortex,
            Self::Vortex    => Self::Freeze,
            Self::Freeze    => Self::Orbit,
            Self::Orbit     => Self::Bomb,
            Self::Bomb      => Self::Spring,
        }
    }
    fn all() -> &'static [DragMode] {
        &[
            Self::Spring, Self::Slingshot, Self::Pull,  Self::Push,
            Self::Vortex, Self::Freeze,    Self::Orbit, Self::Bomb,
        ]
    }
}

struct GravityBomb {
    x:    f32,
    y:    f32,
    born: f64,
    radius: f32,
}

impl GravityBomb {
    const DURATION: f64 = 0.55;

    fn age(&self) -> f32 { (get_time() - self.born) as f32 }

    fn alive(&self) -> bool { (get_time() - self.born) < Self::DURATION }

    fn draw(&self) {
        let t = (self.age() / Self::DURATION as f32).min(1.0);
        let ease = 1.0 - (1.0 - t) * (1.0 - t);

        // Three expanding rings at different phases
        for offset in [0.0_f32, 0.18, 0.36] {
            let rt = ((t + offset) % 1.0).min(1.0);
            let r  = self.radius * rt * 1.2;
            let a  = ((1.0 - rt) * 255.0) as u8;
            let col = Color::from_rgba(255, 120, 40, a);
            draw_circle_lines(self.x, self.y, r, 2.0 * (1.0 - rt) + 0.5, col);
        }

        // Radial burst lines
        let burst_a = ((1.0 - ease) * 220.0) as u8;
        if burst_a > 10 {
            let br = self.radius * 0.6 * ease;
            for i in 0..12u8 {
                let a = i as f32 * std::f32::consts::TAU / 12.0;
                draw_line(
                    self.x + a.cos() * br * 0.3,
                    self.y + a.sin() * br * 0.3,
                    self.x + a.cos() * br,
                    self.y + a.sin() * br,
                    1.5, Color::from_rgba(255, 200, 80, burst_a),
                );
            }
        }

        // Core flash
        let core_r = 18.0 * (1.0 - ease);
        if core_r > 0.5 {
            draw_circle(self.x, self.y, core_r,
                        Color::from_rgba(255, 240, 180, ((1.0 - ease) * 200.0) as u8));
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Drag-mode grid picker  (Tab to open/close)
// ═══════════════════════════════════════════════════════════════
struct DragModeMenu {
    visible: bool,
    fade:    f32,
    hovered: i32,
}

impl DragModeMenu {
    const COLS:   usize = 4;
    const CELL_W: f32   = 140.0;
    const CELL_H: f32   = 90.0;
    const GAP:    f32   = 10.0;
    const PAD:    f32   = 18.0;

    fn new() -> Self { DragModeMenu { visible: false, fade: 0.0, hovered: -1 } }

    fn open(&mut self)  { self.visible = true; self.hovered = -1; }
    fn close(&mut self) { self.visible = false; }

    fn panel_rect() -> Rect {
        let n    = DragMode::all().len();
        let cols = Self::COLS;
        let rows = (n + cols - 1) / cols;
        let pw   = cols as f32 * Self::CELL_W + (cols - 1) as f32 * Self::GAP + Self::PAD * 2.0;
        let ph   = rows as f32 * Self::CELL_H + (rows - 1) as f32 * Self::GAP + Self::PAD * 2.0 + 24.0;
        Rect::new(
            (screen_width()  - pw) / 2.0,
            (screen_height() - ph) / 2.0,
            pw, ph,
        )
    }

    fn cell_rect(i: usize) -> Rect {
        let p   = Self::panel_rect();
        let col = i % Self::COLS;
        let row = i / Self::COLS;
        Rect::new(
            p.x + Self::PAD + col as f32 * (Self::CELL_W + Self::GAP),
            p.y + Self::PAD + 24.0 + row as f32 * (Self::CELL_H + Self::GAP),
            Self::CELL_W,
            Self::CELL_H,
        )
    }

    fn update(&mut self, dt: f32) {
        let target = if self.visible { 1.0_f32 } else { 0.0 };
        self.fade += (target - self.fade) * (dt * 14.0).min(1.0);
        if self.fade < 0.005 { self.fade = 0.0; }
    }

    fn handle_motion(&mut self, mx: f32, my: f32) {
        if self.fade < 0.01 { return; }
        self.hovered = -1;
        for (i, _) in DragMode::all().iter().enumerate() {
            if Self::cell_rect(i).contains(vec2(mx, my)) {
                self.hovered = i as i32;
                return;
            }
        }
    }

    fn handle_click(&mut self, mx: f32, my: f32) -> Option<DragMode> {
        if self.fade < 0.01 { return None; }
        for (i, &mode) in DragMode::all().iter().enumerate() {
            if Self::cell_rect(i).contains(vec2(mx, my)) {
                self.close();
                return Some(mode);
            }
        }
        if !Self::panel_rect().contains(vec2(mx, my)) {
            self.close();
        }
        None
    }

    fn draw(&self, current: DragMode) {
        if self.fade < 0.01 { return; }
        let f = self.fade;
        let a = |v: u8| (v as f32 * f) as u8;
        let p = Self::panel_rect();

        // Panel backdrop
        draw_rectangle(p.x, p.y, p.w, p.h,
                       Color::from_rgba(12, 10, 24, a(240)));
        draw_rectangle_lines(p.x, p.y, p.w, p.h, 1.5,
                             Color::from_rgba(80, 75, 140, a(255)));

        // Title
        let title = "DRAG MODE";
        let tw = measure_text(title, None, 14, 1.0).width;
        draw_text(title, p.x + (p.w - tw) / 2.0, p.y + Self::PAD + 2.0, 14.0,
                  Color::from_rgba(170, 165, 220, a(210)));

        for (i, &mode) in DragMode::all().iter().enumerate() {
            let r       = Self::cell_rect(i);
            let is_cur  = mode == current;
            let is_hov  = self.hovered == i as i32;

            let (bg, border) = if is_cur {
                (Color::from_rgba(55, 50, 115, a(240)),
                 Color::from_rgba(160, 150, 255, a(255)))
            } else if is_hov {
                (Color::from_rgba(38, 34, 75, a(230)),
                 Color::from_rgba(100, 95, 180, a(210)))
            } else {
                (Color::from_rgba(22, 19, 45, a(220)),
                 Color::from_rgba(55, 50, 95, a(180)))
            };

            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, border);

            let lbl  = mode.label();
            let lw   = measure_text(lbl, None, 15, 1.0).width;
            let tcol = if is_cur || is_hov {
                Color::from_rgba(255, 255, 255, a(255))
            } else {
                Color::from_rgba(150, 145, 200, a(255))
            };
            draw_text(lbl, r.x + (r.w - lw) / 2.0, r.y + r.h / 2.0 + 6.0, 15.0, tcol);

            // Small dot on active mode
            if is_cur {
                draw_circle(r.x + r.w - 10.0, r.y + 10.0, 4.0,
                            Color::from_rgba(160, 150, 255, a(255)));
            }
        }
    }
}

struct DragState {
    object_idx:   usize,
    anchor_world: (f32, f32),
    mode:         DragMode,
}

// ═══════════════════════════════════════════════════════════════
// Slider widget
// ═══════════════════════════════════════════════════════════════
struct Slider {
    value:    f32,
    min:      f32,
    max:      f32,
    dragging: bool,
}

impl Slider {
    fn new(value: f32, min: f32, max: f32) -> Self {
        Self { value, min, max, dragging: false }
    }

    // Returns true if the click hit this slider's track (expanded hit area).
    fn try_start(&mut self, track: Rect, mx: f32, my: f32) -> bool {
        let hit = Rect::new(track.x, track.y - 8.0, track.w, track.h + 16.0);
        if hit.contains(vec2(mx, my)) {
            self.dragging = true;
            self.apply_x(track, mx);
            true
        } else {
            false
        }
    }

    fn continue_drag(&mut self, track: Rect, mx: f32) {
        if self.dragging { self.apply_x(track, mx); }
    }

    fn stop(&mut self) { self.dragging = false; }

    fn apply_x(&mut self, track: Rect, mx: f32) {
        let t = ((mx - track.x) / track.w).clamp(0.0, 1.0);
        self.value = self.min + t * (self.max - self.min);
    }

    fn draw(&self, track: Rect, label: &str) {
        let t    = ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        let hx   = track.x + track.w * t;
        let cy   = track.y + track.h * 0.5;

        draw_rectangle(track.x, track.y, track.w, track.h,
                       Color::from_rgba(35, 32, 58, 255));
        draw_rectangle_lines(track.x, track.y, track.w, track.h, 1.0,
                             Color::from_rgba(75, 70, 125, 255));
        draw_rectangle(track.x, track.y, track.w * t, track.h,
                       Color::from_rgba(90, 80, 175, 255));
        draw_circle(hx, cy, 7.0, Color::from_rgba(215, 210, 255, 255));
        draw_circle_lines(hx, cy, 7.0, 1.0,
                          Color::from_rgba(120, 115, 190, 255));
        draw_text(label, track.x, track.y - 5.0, 12.0,
                  Color::from_rgba(160, 155, 210, 255));
    }
}

// ═══════════════════════════════════════════════════════════════
// Module player  (.mod / .xm / .it / .s3m / …)
// Decoding runs in a background thread via libopenmpt.
// PCM floats are streamed to cpal through a lock-free ring buffer.
// ═══════════════════════════════════════════════════════════════
enum ModCmd {
    Load { data: Vec<u8>, name: String },
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
}

#[derive(Clone, Default)]
struct ModInfo {
    loaded:  bool,
    playing: bool,
    title:   String,
    fmt:     String,   // "XM", "IT", "MOD", "S3M", …
    name:    String,   // file name
    order:   i32,
    orders:  i32,
}

struct ModulePlayer {
    tx:   std::sync::mpsc::SyncSender<ModCmd>,
    info: Arc<Mutex<ModInfo>>,
    _thr: std::thread::JoinHandle<()>,
}

impl ModulePlayer {
    fn start() -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<ModCmd>(4);
        let info     = Arc::new(Mutex::new(ModInfo::default()));
        let info2    = Arc::clone(&info);
        let thr      = std::thread::spawn(move || mod_audio_thread(rx, info2));
        Self { tx, info, _thr: thr }
    }

    fn load(&self, path: &str) {
        if let Ok(data) = std::fs::read(path) {
            let name = std::path::Path::new(path)
                .file_name().and_then(|n| n.to_str())
                .unwrap_or(path).to_string();
            self.tx.send(ModCmd::Load { data, name }).ok();
        }
    }

    fn toggle_pause(&self) {
        let playing = self.info.lock().unwrap().playing;
        let cmd = if playing { ModCmd::Pause } else { ModCmd::Resume };
        self.tx.send(cmd).ok();
    }

    fn stop(&self) { self.tx.send(ModCmd::Stop).ok(); }

    fn set_volume(&self, vol: f32) { self.tx.send(ModCmd::SetVolume(vol)).ok(); }

    fn info(&self) -> ModInfo { self.info.lock().unwrap().clone() }
}

// ═══════════════════════════════════════════════════════════════
// .pls playlist parser
// Returns the resolved file paths listed in the playlist.
// HTTP/RTSP streams and missing files are silently skipped.
// ═══════════════════════════════════════════════════════════════
fn parse_pls(pls_path: &str) -> Vec<String> {
    let content = match std::fs::read_to_string(pls_path) {
        Ok(s)  => s,
        Err(_) => return vec![],
    };
    let dir = std::path::Path::new(pls_path)
        .parent()
        .unwrap_or(std::path::Path::new(""));
    let mut entries: std::collections::BTreeMap<u32, String> = std::collections::BTreeMap::new();
    for line in content.lines() {
        let line  = line.trim();
        let lower = line.to_ascii_lowercase();
        if !lower.starts_with("file") { continue; }
        let rest = &line[4..];
        let eq   = match rest.find('=') { Some(i) => i, None => continue };
        let idx: u32 = match rest[..eq].trim().parse() { Ok(n) => n, Err(_) => continue };
        let raw  = rest[eq + 1..].trim();
        let lp   = raw.to_ascii_lowercase();
        let entry = if lp.starts_with("http://") || lp.starts_with("https://")
                    || lp.starts_with("rtsp://")
        {
            raw.to_string()  // keep network URLs verbatim
        } else if std::path::Path::new(raw).is_absolute() {
            raw.to_string()
        } else {
            dir.join(raw).to_string_lossy().into_owned()
        };
        // Always include network URLs; only include local paths that exist
        let is_url = lp.starts_with("http") || lp.starts_with("rtsp");
        if is_url || std::path::Path::new(&entry).exists() {
            entries.insert(idx, entry);
        }
    }
    entries.into_values().collect()
}

// Thin wrapper that lets an HTTP response body satisfy rodio's Read + Seek
// requirement.  Seek is a no-op (returns the current byte position); symphonia
// can still decode MP3/AAC streams that it never needs to rewind.
struct HttpStream {
    inner: Box<dyn std::io::Read + Send + 'static>,
    pos:   u64,
}

impl HttpStream {
    fn open(url: &str) -> Option<Self> {
        let resp = ureq::get(url)
            .set("User-Agent", "gravity-engine/0.1")
            .set("Icy-MetaData", "0")
            .call()
            .ok()?;
        Some(HttpStream { inner: Box::new(resp.into_reader()), pos: 0 })
    }
}

impl std::io::Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl std::io::Seek for HttpStream {
    fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> { Ok(self.pos) }
}

// SAFETY: HttpStream is only ever accessed from the single rodio decode thread.
unsafe impl Sync for HttpStream {}

// ═══════════════════════════════════════════════════════════════
// Regular audio player  (.mp3 / .flac / .wav / .ogg / .pls / …)
// Uses rodio + symphonia for decoding.
// Single files loop indefinitely; playlists loop as a sequence.
// ═══════════════════════════════════════════════════════════════
#[derive(Clone, Default)]
struct RegularAudioInfo {
    loaded:  bool,
    playing: bool,
    name:    String,
    fmt:     String,
}

struct RegularAudioPlayer {
    _stream:  OutputStream,
    handle:   OutputStreamHandle,
    sink:     Option<Sink>,
    state:    RegularAudioInfo,
    playlist: Vec<String>, // non-empty when a .pls was loaded
    volume:   f32,
}

impl RegularAudioPlayer {
    fn new() -> Option<Self> {
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Self {
            _stream: stream, handle, sink: None,
            state: RegularAudioInfo::default(),
            playlist: vec![],
            volume: 1.0,
        })
    }

    fn load(&mut self, path: &str) {
        let file = match std::fs::File::open(path) { Ok(f) => f, Err(_) => return };
        let decoder = match Decoder::new(BufReader::new(file)) {
            Ok(d)  => d,
            Err(e) => { eprintln!("audio decode: {e}"); return }
        };
        if let Some(ref s) = self.sink { s.stop(); }
        let sink = match Sink::try_new(&self.handle) { Ok(s) => s, Err(_) => return };
        sink.append(decoder.repeat_infinite());
        self.sink  = Some(sink);
        self.state = RegularAudioInfo {
            loaded:  true,
            playing: true,
            name: std::path::Path::new(path)
                      .file_name().and_then(|n| n.to_str())
                      .unwrap_or(path).to_string(),
            fmt: std::path::Path::new(path)
                     .extension().and_then(|e| e.to_str())
                     .unwrap_or("").to_uppercase(),
        };
    }

    fn toggle_pause(&mut self) {
        if let Some(ref sink) = self.sink {
            if self.state.playing { sink.pause(); self.state.playing = false; }
            else                  { sink.play();  self.state.playing = true;  }
        }
    }

    // Load a sequence of file paths / URLs and play them in order, looping forever.
    fn load_playlist(&mut self, paths: Vec<String>, display_name: String) {
        if let Some(ref s) = self.sink { s.stop(); }
        let sink = match Sink::try_new(&self.handle) { Ok(s) => s, Err(_) => return };
        sink.set_volume(self.volume);
        let mut n = 0usize;
        for p in &paths {
            let lp = p.to_ascii_lowercase();
            if lp.starts_with("http://") || lp.starts_with("https://") {
                if let Some(s) = HttpStream::open(p) {
                    if let Ok(d) = Decoder::new(BufReader::new(s)) { sink.append(d); n += 1; }
                    else { eprintln!("stream decode failed: {p}"); }
                } else { eprintln!("stream connect failed: {p}"); }
            } else if let Ok(f) = std::fs::File::open(p) {
                if let Ok(d) = Decoder::new(BufReader::new(f)) { sink.append(d); n += 1; }
            }
        }
        if n == 0 { return; }
        self.sink     = Some(sink);
        self.playlist = paths;
        self.state    = RegularAudioInfo {
            loaded: true, playing: true,
            name: display_name,
            fmt: "PLS".to_string(),
        };
    }

    // Call once per frame: re-queues the playlist when the sink drains.
    fn check_replay(&mut self) {
        if self.playlist.is_empty() || !self.state.playing { return; }
        if self.sink.as_ref().map_or(false, |s| s.empty()) {
            let paths = self.playlist.clone();
            let name  = self.state.name.clone();
            self.load_playlist(paths, name);
        }
    }

    fn stop(&mut self) {
        if let Some(ref s) = self.sink { s.stop(); }
        self.sink     = None;
        self.playlist = vec![];
        self.state    = RegularAudioInfo::default();
    }

    fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 1.0);
        if let Some(ref s) = self.sink { s.set_volume(self.volume); }
    }

    fn info(&self) -> RegularAudioInfo { self.state.clone() }
}

fn mod_audio_thread(
    rx:   std::sync::mpsc::Receiver<ModCmd>,
    info: Arc<Mutex<ModInfo>>,
) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use openmpt::module::{Logger, Module};
    use openmpt::module::metadata::MetadataKey;

    let host   = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None    => { eprintln!("mod: no audio output device"); return; }
    };

    const SR: u32 = 48_000;
    const CH: u16 = 2;
    let cfg = cpal::StreamConfig {
        channels:    CH,
        sample_rate: cpal::SampleRate(SR),
        buffer_size: cpal::BufferSize::Default,
    };

    // 1 second of stereo headroom
    let (mut prod, c) = ringbuf::HeapRb::<f32>::new(SR as usize * CH as usize).split();
    let mut c_audio   = c; // renamed so the move closure captures it as mutable

    let stream = match device.build_output_stream::<f32, _, _>(
        &cfg,
        move |out: &mut [f32], _| {
            for s in out.iter_mut() { *s = c_audio.pop().unwrap_or(0.0); }
        },
        |e| eprintln!("cpal error: {e}"),
        None,
    ) {
        Ok(s)  => s,
        Err(e) => { eprintln!("mod: failed to open audio stream: {e}"); return; }
    };
    if let Err(e) = stream.play() {
        eprintln!("mod: stream.play failed: {e}"); return;
    }

    let mut module:     Option<Module> = None;
    let mut cur_volume: f32            = 1.0;

    loop {
        // Drain command queue
        loop {
            match rx.try_recv() {
                Ok(ModCmd::Load { mut data, name }) => {
                    // create_from_memory uses buf.capacity() to size the read
                    if let Ok(mut m) = Module::create_from_memory(
                        &mut data,
                        Logger::None,
                        &[],
                    ) {
                        m.set_repeat_count(-1); // loop forever
                        let title  = m.get_metadata(MetadataKey::ModuleTitle)
                            .unwrap_or_default();
                        let fmt    = m.get_metadata(MetadataKey::TypeExt)
                            .unwrap_or_default().to_uppercase();
                        let orders = m.get_num_orders();
                        *info.lock().unwrap() = ModInfo {
                            loaded: true, playing: true,
                            title, fmt, name,
                            order: 0, orders,
                        };
                        module = Some(m);
                    }
                }
                Ok(ModCmd::Pause)  => { info.lock().unwrap().playing = false; }
                Ok(ModCmd::Resume) => { info.lock().unwrap().playing = true;  }
                Ok(ModCmd::Stop)   => {
                    module = None;
                    *info.lock().unwrap() = ModInfo::default();
                }
                Ok(ModCmd::SetVolume(v)) => { cur_volume = v.clamp(0.0, 1.0); }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                Err(std::sync::mpsc::TryRecvError::Empty)        => break,
            }
        }

        // Decode into ring buffer while there is room
        if info.lock().unwrap().playing {
            if let Some(ref mut m) = module {
                info.lock().unwrap().order = m.get_current_order();

                let free = prod.free_len() / CH as usize; // free stereo frames
                let want = free.min(4096);
                if want > 0 {
                    // Pre-fill so capacity == want*2; the C API reads buf.capacity()>>1 frames
                    let mut buf = vec![0.0f32; want * CH as usize];
                    let got = m.read_interleaved_float_stereo(SR as i32, &mut buf);
                    for &s in &buf[..got * CH as usize] {
                        prod.push(s * cur_volume).ok();
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

// ═══════════════════════════════════════════════════════════════
// Pixel-out transition (title screen → game)
// ═══════════════════════════════════════════════════════════════
struct PixelOut {
    order:   Vec<usize>, // block indices in random dismissal order
    colors:  Vec<Color>, // one pre-assigned color per block index
    cleared: f32,        // fractional count of blocks removed so far
    cols:    usize,
    block:   f32,        // side length in px
}

// Palette sampled from the title screen's own colors so the initial
// mosaic looks like a pixelated snapshot of the title screen.
const PIXEL_PALETTE: &[(u8, u8, u8)] = &[
    ( 10,  10,  20), // dark background (top half)
    ( 22,  14,  38), // purple background (bottom half)
    ( 25,  23,  42), // controls-box fill
    ( 70,  65, 120), // border purple
    (100,  90, 180), // accent line
    (160, 155, 220), // key text
    (110, 105, 160), // value text
    (210, 205, 255), // title text
];

impl PixelOut {
    fn new(sw: f32, sh: f32) -> Self {
        let block = 16.0_f32;
        let cols  = (sw / block).ceil() as usize + 1;
        let rows  = (sh / block).ceil() as usize + 1;
        let n     = cols * rows;
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = rand::gen_range(0usize, i + 1);
            order.swap(i, j);
        }
        let colors: Vec<Color> = (0..n).map(|_| {
            let (r, g, b) = PIXEL_PALETTE[rand::gen_range(0, PIXEL_PALETTE.len())];
            Color::from_rgba(r, g, b, 255)
        }).collect();
        PixelOut { order, colors, cleared: 0.0, cols, block }
    }

    // Advance; returns true when all blocks have been cleared.
    fn update(&mut self, dt: f32) -> bool {
        let total = self.order.len() as f32;
        self.cleared = (self.cleared + dt * total / 0.45).min(total);
        self.cleared as usize >= self.order.len()
    }

    // Draw every block that hasn't been cleared yet.
    // The caller is responsible for drawing the destination scene underneath first.
    fn draw(&self) {
        let cleared_int = self.cleared as usize;
        for &idx in &self.order[cleared_int..] {
            let col = idx % self.cols;
            let row = idx / self.cols;
            draw_rectangle(
                col as f32 * self.block,
                row as f32 * self.block,
                self.block, self.block,
                self.colors[idx],
            );
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Main
// ═══════════════════════════════════════════════════════════════
#[macroquad::main(window_conf)]
async fn main() {
    let preload: Vec<String> = std::env::args().skip(1).collect();

    let mut pw        = PhysWorld::new(-9.8);
    let mut objects:  Vec<PhysicsImage> = vec![];
    let mut menu      = ContextMenu::new();
    let mut grav_btns = GravityButtons::new();
    let mut paused        = false;
    let mut drag:         Option<DragState> = None;
    let mut drag_mode       = DragMode::Spring;
    let mut drag_mode_menu  = DragModeMenu::new();
    let mut gravity_bombs: Vec<GravityBomb> = Vec::new();
    let mut background = Background::new();
    let mut spawner    = Spawner::new();
    let mut spawner_size_drag = false;
    let mut win_tracker   = WindowTracker::new();
    let mut win_force_slider = Slider::new(8.0, 0.0, 40.0);
    let mut win_shake_on     = true;
    let mut btn_rx: Option<std::sync::mpsc::Receiver<(String, Vec<u8>)>> = None;
    let mut btn_pending = 0usize; // how many still expected
    let mut field_active    = false;
    let mut slider_radius = Slider::new(150.0, 30.0,  450.0); // pixels
    let mut slider_force  = Slider::new(300.0, 20.0, 1500.0);
    let mut vol_slider      = Slider::new(1.0,  0.0,  1.0);
    let mut last_vol        = 1.0f32;
    let mut trail_enabled    = true;
    let mut trail_len_slider  = Slider::new(15.0, 3.0, TRAIL_MAX as f32);
    let mut trail_fade_slider = Slider::new(2.0,  0.2, 8.0);
    let mut trail_slide       = -(188.0_f32 + 16.0); // start off-screen left
    let mut hud_y         = 0.0_f32;   // 0=fully visible, -(HUD_H-4)=peeking
    let mut panel_slide   = 1.0_f32;   // 0=on-screen, 1=slid off right edge
    let mut win_slide     = 1.0_f32;   // window-force panel, same convention
    let mut show_title  = true;
    let mut pixel_out: Option<PixelOut> = None;
    let mut show_debug  = false;
    let mod_player = ModulePlayer::start();
    let mut reg_player = RegularAudioPlayer::new();

    let mut last_w = screen_width();
    let mut last_h = screen_height();

    for path in &preload {
        let spawn = (screen_width() / 2.0,
                     HUD_H + 40.0 + rand::gen_range(0.0f32, 60.0));
        if let Some(obj) = PhysicsImage::new(
            path, None, None, spawn, &mut pw.bodies, &mut pw.colliders)
        {
            objects.push(obj);
        }
    }

    loop {
        let dt       = get_frame_time();
        let dt_ms    = dt * 1000.0;
        let (mx, my) = mouse_position();
        let sw = screen_width();
        let sh = screen_height();

        // ── animation ─────────────────────────────────────────
        menu.update(dt);
        drag_mode_menu.update(dt);
        spawner.update(dt);

        let audio_loaded = mod_player.info().loaded
            || reg_player.as_ref().map_or(false, |p| p.info().loaded);
        // HUD slides up to leave a 4 px peek strip; stays visible when
        // the mouse is near the top, the vol slider is being dragged, or physics is paused.
        let hud_near = paused || vol_slider.dragging || win_force_slider.dragging;
        hud_y += ((if hud_near { 0.0 } else { -(HUD_H - 4.0) }) - hud_y)
                  * (dt * 10.0).min(1.0);

        // Slider panel geometry (bottom-right, shown in Pull/Push mode)
        let show_sliders = matches!(drag_mode, DragMode::Pull | DragMode::Push | DragMode::Vortex | DragMode::Freeze | DragMode::Orbit | DragMode::Bomb);
        panel_slide += ((if show_sliders { 0.0 } else { 1.0 }) - panel_slide)
                        * (dt * 12.0).min(1.0);

        // Trail panel slides in from the left when the mouse is in the bottom-left
        // corner or a trail slider is being dragged.
        let near_trail = paused
                      || trail_len_slider.dragging
                      || trail_fade_slider.dragging;
        trail_slide += ((if near_trail { 0.0_f32 } else { -(188.0 + 16.0) }) - trail_slide)
                        * (dt * 10.0).min(1.0);

        let show_win_panel = paused || win_force_slider.dragging;
        win_slide += ((if show_win_panel { 0.0 } else { 1.0 }) - win_slide)
                      * (dt * 12.0).min(1.0);

        let s_panel_w = 230.0_f32;
        let s_panel_h = 88.0_f32;
        let s_panel_x = sw - s_panel_w - 8.0 + panel_slide * (s_panel_w + 16.0);
        let s_panel_y = sh - s_panel_h - 8.0;
        let s_panel   = Rect::new(s_panel_x, s_panel_y, s_panel_w, s_panel_h);
        let r_track   = Rect::new(s_panel_x + 10.0, s_panel_y + 26.0, s_panel_w - 20.0, 10.0);
        let f_track   = Rect::new(s_panel_x + 10.0, s_panel_y + 64.0, s_panel_w - 20.0, 10.0);

        let vol_track = Rect::new(sw / 2.0 - 70.0, 34.0 + hud_y, 140.0, 6.0);

        // Window-force panel (bottom-right, above drag-mode panel, pause-only)
        let wf_panel_w = 230.0_f32;
        let wf_panel_h = 50.0_f32;
        let wf_panel_x = sw - wf_panel_w - 8.0 + win_slide * (wf_panel_w + 16.0);
        let wf_panel_y = sh - s_panel_h - wf_panel_h - 20.0;
        let wf_track   = Rect::new(wf_panel_x + 10.0, wf_panel_y + 28.0, wf_panel_w - 20.0, 10.0);

        // Trail settings panel (bottom-left, always visible)
        let tp_w   = 188.0_f32;
        let tp_h   = if trail_enabled { 90.0 } else { 32.0 };
        let tp_x   = 8.0_f32 + trail_slide;
        let tp_y   = sh - tp_h - 8.0 - WALL_T * PPM;
        let tp_panel   = Rect::new(tp_x, tp_y, tp_w, tp_h);
        let tp_tog     = Rect::new(tp_x + 4.0,  tp_y + 5.0,  tp_w - 8.0,  22.0);
        let tp_sld     = Rect::new(tp_x + 10.0, tp_y + 38.0, tp_w - 20.0,  8.0);
        let tp_sld2    = Rect::new(tp_x + 10.0, tp_y + 66.0, tp_w - 20.0,  8.0);

        // ── title screen ──────────────────────────────────────
        if show_title {
            // During pixel-out, show the game background so departing blocks
            // reveal the scene underneath; otherwise show the title gradient.
            if pixel_out.is_some() {
                clear_background(Color::from_rgba(15, 14, 25, 255));
                draw_rectangle(0.0, 0.0, sw, sh / 2.0,
                               Color::from_rgba(12, 12, 22, 255));
                draw_rectangle(0.0, sh / 2.0, sw, sh / 2.0,
                               Color::from_rgba(28, 18, 48, 255));
            } else {
                clear_background(Color::from_rgba(15, 14, 25, 255));
                draw_rectangle(0.0, 0.0, sw, sh * 0.5,
                               Color::from_rgba(10, 10, 20, 255));
                draw_rectangle(0.0, sh * 0.5, sw, sh * 0.5,
                               Color::from_rgba(22, 14, 38, 255));
            }

            let cx = sw / 2.0;
            if pixel_out.is_none() {

            // ── Logo ──────────────────────────────────────────
            {
                let lx = cx;
                let ly = sh * 0.14;
                let t  = get_time() as f32;

                // Gravity field lines (8 directions, dashed segments fading out)
                for i in 0..8u8 {
                    let a = i as f32 * std::f32::consts::TAU / 8.0;
                    for seg in 0..3u8 {
                        let f  = seg as f32 / 3.0;
                        let r0 = 18.0 + f * 38.0;
                        let r1 = r0 + 10.0;
                        let al = ((1.0 - f) * 38.0) as u8;
                        draw_line(lx + r0 * a.cos(), ly + r0 * a.sin(),
                                  lx + r1 * a.cos(), ly + r1 * a.sin(),
                                  1.0, Color::from_rgba(120, 100, 210, al));
                    }
                }

                // Orbital path rings (dotted)
                for &orbit_r in &[33.0_f32, 47.0, 61.0] {
                    let n = (orbit_r * 1.1) as usize;
                    for j in 0..n {
                        let a = j as f32 / n as f32 * std::f32::consts::TAU;
                        draw_circle(lx + orbit_r * a.cos(), ly + orbit_r * a.sin(),
                                    0.9, Color::from_rgba(90, 80, 155, 45));
                    }
                }

                // Orbiting image-blocks  (radius, speed, phase, w, h, r, g, b)
                const ORBS: &[(f32,f32,f32,f32,f32,u8,u8,u8)] = &[
                    (33.0,  0.70,  0.00, 12.0,  9.0, 100, 185, 255),
                    (47.0, -0.50,  1.30, 13.0, 11.0, 255, 150,  65),
                    (61.0,  0.38,  3.10, 11.0, 13.0, 170,  95, 255),
                    (40.0,  1.35,  4.70,  9.0,  9.0,  75, 215, 140),
                    (54.0, -0.85,  2.20, 12.0, 10.0, 255,  95, 140),
                ];
                for &(r, spd, phase, w, h, cr, cg, cb) in ORBS {
                    let ang  = t * spd + phase;
                    let ox   = lx + r * ang.cos();
                    let oy   = ly + r * ang.sin();
                    // Motion trail
                    let tang = ang - spd.signum() * 0.28;
                    let tx   = lx + r * tang.cos();
                    let ty   = ly + r * tang.sin();
                    draw_rectangle(tx - w * 0.35, ty - h * 0.35, w * 0.7, h * 0.7,
                                   Color::from_rgba(cr, cg, cb, 55));
                    // Block face
                    draw_rectangle(ox - w * 0.5, oy - h * 0.5, w, h,
                                   Color::from_rgba(cr, cg, cb, 215));
                    draw_rectangle_lines(ox - w * 0.5, oy - h * 0.5, w, h, 1.0,
                                         Color::from_rgba(
                                             cr.saturating_add(55),
                                             cg.saturating_add(55),
                                             cb.saturating_add(55), 255));
                }

                // Central body – layered glow then solid core
                for i in 0..5u8 {
                    let r  = 22.0 - i as f32 * 3.2;
                    let al = ((5 - i) as f32 / 5.0 * 35.0) as u8;
                    draw_circle(lx, ly, r, Color::from_rgba(195, 175, 255, al));
                }
                draw_circle(lx, ly, 11.0, Color::from_rgba(190, 170, 255, 240));
                draw_circle(lx, ly,  7.0, Color::from_rgba(225, 215, 255, 255));
                draw_circle(lx, ly,  3.5, Color::from_rgba(255, 252, 255, 255));
            }

            // Title
            let title = "IMAGE GRAVITY ENGINE";
            let tfs   = 38.0_f32;
            let tw    = measure_text(title, None, tfs as u16, 1.0).width;
            draw_text(title, cx - tw / 2.0, sh * 0.30,
                      tfs, Color::from_rgba(210, 205, 255, 255));
            draw_line(cx - tw / 2.0, sh * 0.30 + 6.0,
                      cx + tw / 2.0, sh * 0.30 + 6.0,
                      1.5, Color::from_rgba(100, 90, 180, 200));

            // Subtitle
            let sub  = "A Rust physics sandbox";
            let subw = measure_text(sub, None, 16, 1.0).width;
            draw_text(sub, cx - subw / 2.0, sh * 0.30 + 28.0,
                      16.0, Color::from_rgba(130, 125, 175, 255));

            // Controls box
            let box_w = (380.0_f32).min(sw - 80.0);
            let box_x = cx - box_w / 2.0;
            let box_y = sh * 0.42;
            let box_h = 288.0;
            draw_rectangle(box_x, box_y, box_w, box_h,
                           Color::from_rgba(25, 23, 42, 230));
            draw_rectangle_lines(box_x, box_y, box_w, box_h, 1.0,
                                 Color::from_rgba(70, 65, 120, 255));

            let entries: &[(&str, &str)] = &[
                ("A",           "Add images (.png .jpg .gif .svg .bmp .tiff .ico …)"),
                ("Left-drag",   "Throw object"),
                ("Right-click", "Context menu"),
                ("Space",       "Pause / unpause physics"),
                ("M",           "Load audio (.mp3 .flac .ogg .wav / .mod .xm .it …)"),
                ("P",           "Pause / resume music"),
                ("B",           "Cycle side borders (walls / loop / kill)"),
                ("R",           "Clear all objects"),
                ("D",           "Toggle debug overlay"),
                ("Q / Esc",     "Quit"),
            ];
            let lh      = 26.0;
            let lfs     = 14.0;
            let lx      = box_x + 20.0;
            let col_key = Color::from_rgba(160, 155, 220, 255);
            let col_val = Color::from_rgba(110, 105, 160, 255);
            for (i, (key, val)) in entries.iter().enumerate() {
                let ly = box_y + 22.0 + i as f32 * lh + lfs;
                draw_text(key, lx, ly, lfs, col_key);
                let kw = measure_text(key, None, lfs as u16, 1.0).width;
                draw_text(val, lx + kw + 16.0, ly, lfs, col_val);
            }

            // Blinking "press any key" prompt
            if ((get_time() / 0.6) as i32 % 2) == 0 {
                let prompt = "Press any key or click to start";
                let pw2    = measure_text(prompt, None, 16, 1.0).width;
                draw_text(prompt, cx - pw2 / 2.0, sh * 0.88,
                          16.0, Color::from_rgba(180, 175, 230, 200));
            }
            } // end: title content drawn only when no pixel-out in progress

            // Quit from title screen (always works, even mid-transition)
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) {
                break;
            }
            // Any other input triggers the pixel-out transition
            if pixel_out.is_none()
                && (get_char_pressed().is_some()
                    || is_key_pressed(KeyCode::Space)
                    || is_key_pressed(KeyCode::Enter)
                    || is_mouse_button_pressed(MouseButton::Left)
                    || is_mouse_button_pressed(MouseButton::Right))
            {
                pixel_out = Some(PixelOut::new(sw, sh));
            }

            // Pixel-out overlay: draw remaining blocks, advance, and finish
            if let Some(ref mut po) = pixel_out {
                po.draw();
                if po.update(dt) {
                    show_title = false;
                    pixel_out  = None;
                }
            }

            next_frame().await;
            continue;
        }

        // ── window resize ─────────────────────────────────────
        if (sw - last_w).abs() > 1.0 || (sh - last_h).abs() > 1.0 {
            pw.rebuild_walls();
            last_w = sw;
            last_h = sh;
        }

        // ── keyboard ──────────────────────────────────────────
        if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::Escape) {
            if menu.visible { menu.close(); } else { break; }
        }
        if is_key_pressed(KeyCode::Space) { paused = !paused; }
        if is_key_pressed(KeyCode::Tab) {
            if drag_mode_menu.visible { drag_mode_menu.close(); }
            else { drag_mode_menu.open(); }
        }
        if is_key_pressed(KeyCode::B)     { pw.set_border_mode(pw.border_mode.next()); }
        if is_key_pressed(KeyCode::W)     { win_shake_on = !win_shake_on; }
        if is_key_pressed(KeyCode::F) && btn_pending == 0 {
            let (tx, rx) = std::sync::mpsc::channel();
            fetch_88x31(tx, 20);
            btn_rx      = Some(rx);
            btn_pending = 20;
        }
        if is_key_pressed(KeyCode::G) {
            if background.mode == BgMode::Custom {
                // Load a custom background image
                if let Some(path) = FileDialog::new()
                    .add_filter("Image", &["png","jpg","jpeg","webp","bmp","gif"])
                    .pick_file()
                {
                    if let Ok(bytes) = std::fs::read(&path) {
                        if let Ok(img) = image::load_from_memory(&bytes) {
                            let img   = img.to_rgba8();
                            let tex   = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, &img);
                            background.texture = Some(tex);
                        }
                    }
                }
            } else {
                background.mode = background.mode.next();
                // If we just switched to Custom, open picker immediately
                if background.mode == BgMode::Custom {
                    if let Some(path) = FileDialog::new()
                        .add_filter("Image", &["png","jpg","jpeg","webp","bmp","gif"])
                        .pick_file()
                    {
                        if let Ok(bytes) = std::fs::read(&path) {
                            if let Ok(img) = image::load_from_memory(&bytes) {
                                let img   = img.to_rgba8();
                                let tex   = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, &img);
                                background.texture = Some(tex);
                            }
                        }
                    }
                }
            }
        }
        if is_key_pressed(KeyCode::T)     { trail_enabled = !trail_enabled; }
        if is_key_pressed(KeyCode::N)     { spawner.toggle(); }
        if is_key_pressed(KeyCode::D)     { show_debug = !show_debug; }
        if is_key_pressed(KeyCode::P) {
            if mod_player.info().loaded  { mod_player.toggle_pause(); }
            else if let Some(ref mut p) = reg_player { p.toggle_pause(); }
        }
        if is_key_pressed(KeyCode::M) {
            const TRACKER_EXT: &[&str] = &["mod","xm","it","s3m","mptm","mo3","okt","umx","669","far","mtm"];
            const REGULAR_EXT: &[&str] = &["mp3","flac","wav","ogg","opus","aac","m4a","pls"];
            if let Some(path) = FileDialog::new()
                .add_filter("All audio",       &["mod","xm","it","s3m","mptm","mo3","okt","umx","669","far","mtm","mp3","flac","wav","ogg","opus","aac","m4a","pls"])
                .add_filter("Audio files",     REGULAR_EXT)
                .add_filter("Playlists",       &["pls"])
                .add_filter("Tracker modules", TRACKER_EXT)
                .add_filter("All files", &["*"])
                .pick_file()
            {
                let path_str = path.to_string_lossy().into_owned();
                let ext      = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if TRACKER_EXT.contains(&ext.as_str()) {
                    if let Some(ref mut p) = reg_player { p.stop(); }
                    mod_player.load(&path_str);
                } else if ext == "pls" {
                    let tracks = parse_pls(&path_str);
                    if !tracks.is_empty() {
                        mod_player.stop();
                        if let Some(ref mut p) = reg_player {
                            let fname = std::path::Path::new(&path_str)
                                .file_name().and_then(|n| n.to_str())
                                .unwrap_or(&path_str);
                            let name = format!("{} ({} tracks)", fname, tracks.len());
                            p.load_playlist(tracks, name);
                            p.set_volume(vol_slider.value);
                        }
                    }
                } else {
                    mod_player.stop();
                    if let Some(ref mut p) = reg_player {
                        p.load(&path_str);
                        p.set_volume(vol_slider.value);
                    }
                }
            }
        }
        if is_key_pressed(KeyCode::R) {
            menu.close();
            for obj in objects.drain(..) {
                obj.destroy(&mut pw.bodies, &mut pw.colliders,
                             &mut pw.island_manager,
                             &mut pw.impulse_joints,
                             &mut pw.multibody_joints);
            }
            drag = None;
        }

        // ── file picker ───────────────────────────────────────
        if is_key_pressed(KeyCode::A) {
            if let Some(paths) = FileDialog::new()
                .add_filter("Images", &[
                    "png","jpg","jpeg","gif","webp","svg",
                    "bmp","tiff","tif","ico","tga","qoi",
                    "hdr","exr","dds","ff","ppm","pgm","pbm",
                ])
                .add_filter("All files", &["*"])
                .pick_files()
            {
                for path in paths {
                    let path_str = path.to_string_lossy().into_owned();
                    let spawn = (
                        sw * 0.5 + rand::gen_range(-80.0f32, 80.0),
                        HUD_H + 40.0 + rand::gen_range(0.0f32, 60.0),
                    );
                    if let Some(obj) = PhysicsImage::new(
                        &path_str, None, None, spawn,
                        &mut pw.bodies, &mut pw.colliders)
                    {
                        objects.push(obj);
                    }
                }
            }
        }

        // ── mouse motion ──────────────────────────────────────
        menu.handle_motion(mx, my);
        drag_mode_menu.handle_motion(mx, my);
        if spawner_size_drag {
            let p = spawner.panel_rect();
            let track_x = p.x + 20.0;
            let track_w = p.w - 40.0;
            let t = ((mx - track_x) / track_w).clamp(0.0, 1.0);
            spawner.size = 30.0 + t * 150.0;
        }
        if show_sliders {
            slider_radius.continue_drag(r_track, mx);
            slider_force.continue_drag(f_track, mx);
        }
        if audio_loaded { vol_slider.continue_drag(vol_track, mx); }
        win_force_slider.continue_drag(wf_track, mx);
        if trail_enabled {
            trail_len_slider.continue_drag(tp_sld,  mx);
            trail_fade_slider.continue_drag(tp_sld2, mx);
        }

        // Spring / Slingshot — single object
        if let Some(ref ds) = drag {
            let (bx, by) = to_phys(mx, my);
            if let Some(body) = pw.bodies.get_mut(objects[ds.object_idx].body_handle) {
                let pos = *body.translation();
                let dx  = bx - pos.x;
                let dy  = by - pos.y;
                let force = match ds.mode {
                    DragMode::Spring => vector![
                        dx * 800.0 - body.linvel().x * 25.0,
                        dy * 800.0 - body.linvel().y * 25.0,
                    ],
                    DragMode::Slingshot => vector![
                        dx * 2000.0 - body.linvel().x * 55.0,
                        dy * 2000.0 - body.linvel().y * 55.0,
                    ],
                    _ => vector![0.0, 0.0],
                };
                body.reset_forces(true);
                body.add_force(force, true);
                body.wake_up(true);
            }
        }

        // Pull / Push — field affects every object within radius
        if field_active {
            let (bx, by)  = to_phys(mx, my);
            let r_phys    = slider_radius.value / PPM;
            let fv        = slider_force.value;
            let handles: Vec<_> = objects.iter().map(|o| o.body_handle).collect();
            for handle in handles {
                if let Some(body) = pw.bodies.get_mut(handle) {
                    let pos  = *body.translation();
                    let dx   = bx - pos.x;
                    let dy   = by - pos.y;
                    let dist = (dx*dx + dy*dy).sqrt().max(0.1);
                    body.reset_forces(true);
                    if dist <= r_phys {
                        let mag = (fv / (dist * dist)).min(fv * 3.0);
                        let force = match drag_mode {
                            DragMode::Pull  => vector![ dx/dist * mag,  dy/dist * mag],
                            DragMode::Push  => vector![-dx/dist * mag, -dy/dist * mag],
                            DragMode::Vortex => {
                                // Tangent force → counterclockwise spin
                                vector![-dy/dist * mag, dx/dist * mag]
                            }
                            DragMode::Orbit => {
                                // Centripetal + tangent: keeps objects circling at current radius
                                let inward = (fv * 0.6 / (dist * dist)).min(fv * 2.0);
                                let tangent = (fv * 0.4 / dist).min(fv * 1.5);
                                vector![
                                    dx/dist * inward + (-dy/dist) * tangent,
                                    dy/dist * inward + ( dx/dist) * tangent,
                                ]
                            }
                            DragMode::Freeze => {
                                // Apply damping as a counter-velocity force
                                let damp = (fv * 0.15).min(fv);
                                vector![
                                    -body.linvel().x * damp,
                                    -body.linvel().y * damp,
                                ]
                            }
                            _ => vector![0.0, 0.0],
                        };
                        body.add_force(force, true);
                        body.wake_up(true);
                    }
                }
            }
        }

        // ── mouse buttons ─────────────────────────────────────
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(g) = grav_btns.handle_click(mx, my) {
                pw.set_gravity(g);
            } else if win_force_slider.try_start(wf_track, mx, my) {
                // win force drag started
            } else if audio_loaded && vol_slider.try_start(vol_track, mx, my) {
                // volume drag started — handled by continue_drag above
            } else if drag_mode_menu.fade > 0.01 {
                if let Some(new_mode) = drag_mode_menu.handle_click(mx, my) {
                    drag_mode = new_mode;
                    drag = None;
                    field_active = false;
                }
            } else if spawner.fade > 0.01 {
                let p = spawner.panel_rect();
                let track = Rect::new(p.x + 20.0, p.y + 158.0, p.w - 40.0, 16.0);
                if track.contains(vec2(mx, my)) {
                    spawner_size_drag = true;
                } else if spawner.handle_click(mx, my) {
                    // panel interaction consumed
                } else {
                    // click outside panel → spawn object
                    if let Some(obj) = spawner.spawn_object(
                        (mx, my), &mut pw.bodies, &mut pw.colliders)
                    {
                        objects.push(obj);
                    }
                }
            } else if menu.visible {
                if let Some((action, idx)) = menu.handle_click(mx, my) {
                    match action {
                        MenuAction::Delete => {
                            drag = None;
                            let obj = objects.remove(idx);
                            obj.destroy(&mut pw.bodies, &mut pw.colliders,
                                        &mut pw.island_manager,
                                        &mut pw.impulse_joints,
                                        &mut pw.multibody_joints);
                        }
                        MenuAction::CopyPaste => {
                            if idx < objects.len() {
                                let src    = &objects[idx];
                                let path   = src.path.clone();
                                let tw     = src.texture_w();
                                let th     = src.texture_h();
                                let (ox, oy) = to_screen(
                                    pw.bodies[src.body_handle].translation().x,
                                    pw.bodies[src.body_handle].translation().y,
                                );
                                let offset = rand::gen_range(20.0f32, 50.0);
                                if let Some(dup) = PhysicsImage::new(
                                    &path, Some(tw), Some(th),
                                    (ox + offset, oy + offset),
                                    &mut pw.bodies, &mut pw.colliders)
                                {
                                    objects.push(dup);
                                }
                            }
                        }
                        MenuAction::Resize(factor) => {
                            if idx < objects.len() {
                                objects[idx].resize(
                                    factor, &mut pw.bodies, &mut pw.colliders,
                                    &mut pw.island_manager);
                            }
                        }
                        MenuAction::SizeAll(factor) => {
                            for obj in &mut objects {
                                obj.resize(factor, &mut pw.bodies, &mut pw.colliders,
                                           &mut pw.island_manager);
                            }
                        }
                    }
                }
            } else if tp_panel.contains(vec2(mx, my)) {
                if tp_tog.contains(vec2(mx, my)) {
                    trail_enabled = !trail_enabled;
                } else if trail_enabled {
                    if !trail_len_slider.try_start(tp_sld, mx, my) {
                        trail_fade_slider.try_start(tp_sld2, mx, my);
                    }
                }
            } else if show_sliders && s_panel.contains(vec2(mx, my)) {
                if !slider_radius.try_start(r_track, mx, my) {
                    slider_force.try_start(f_track, mx, my);
                }
            } else if my > (HUD_H + hud_y).max(0.0) {
                match drag_mode {
                    DragMode::Pull | DragMode::Push | DragMode::Vortex | DragMode::Freeze | DragMode::Orbit => {
                        field_active = true;
                    }
                    DragMode::Bomb => {
                        // Instant radial explosion at cursor position
                        let (bx, by) = to_phys(mx, my);
                        let r_phys   = slider_radius.value / PPM;
                        let fv       = slider_force.value;
                        let handles: Vec<_> = objects.iter().map(|o| o.body_handle).collect();
                        for handle in handles {
                            if let Some(body) = pw.bodies.get_mut(handle) {
                                let pos  = *body.translation();
                                let dx   = pos.x - bx;
                                let dy   = pos.y - by;
                                let dist = (dx*dx + dy*dy).sqrt().max(0.1);
                                if dist <= r_phys {
                                    let mag = (fv * 4.0 / (dist * dist)).min(fv * 15.0);
                                    body.apply_impulse(
                                        vector![dx/dist * mag, dy/dist * mag], true);
                                    body.wake_up(true);
                                }
                            }
                        }
                        gravity_bombs.push(GravityBomb {
                            x: mx, y: my,
                            born: get_time(),
                            radius: slider_radius.value,
                        });
                    }
                    _ => {
                        if let Some(i) = pw.body_at_px(&objects, mx, my) {
                            let pos = *pw.bodies[objects[i].body_handle].translation();
                            drag = Some(DragState {
                                object_idx:   i,
                                anchor_world: (pos.x, pos.y),
                                mode:         drag_mode,
                            });
                        }
                    }
                }
            }
        }

        if is_mouse_button_released(MouseButton::Left) {
            spawner_size_drag = false;
            slider_radius.stop();
            slider_force.stop();
            vol_slider.stop();
            trail_len_slider.stop();
            trail_fade_slider.stop();
            win_force_slider.stop();
            if let Some(ref ds) = drag {
                if let Some(body) = pw.bodies.get_mut(
                        objects[ds.object_idx].body_handle) {
                    body.reset_forces(true);
                    if ds.mode == DragMode::Slingshot {
                        // Launch opposite to the pull direction
                        let (ax, ay) = ds.anchor_world;
                        let (mxp, myp) = to_phys(mx, my);
                        body.set_linvel(
                            vector![(ax - mxp) * 5.5, (ay - myp) * 5.5],
                            true,
                        );
                    }
                }
            }
            drag = None;
            if field_active {
                // clear any forces left on all bodies from the field
                for obj in &objects {
                    if let Some(body) = pw.bodies.get_mut(obj.body_handle) {
                        body.reset_forces(true);
                    }
                }
                field_active = false;
            }
        }

        if is_mouse_button_pressed(MouseButton::Right) {
            if menu.visible { menu.close(); }
            if let Some(i) = pw.body_at_px(&objects, mx, my) {
                menu.open(mx, my, i);
            }
        }

        // ── playlist replay ───────────────────────────────────
        if let Some(ref mut p) = reg_player { p.check_replay(); }

        // ── volume ────────────────────────────────────────────
        let v = vol_slider.value;
        if (v - last_vol).abs() > 0.001 {
            mod_player.set_volume(v);
            if let Some(ref mut p) = reg_player { p.set_volume(v); }
            last_vol = v;
        }

        // ── physics step ──────────────────────────────────────
        if !paused {
            if win_shake_on { win_tracker.tick(&mut pw.bodies, win_force_slider.value); }
            pw.step();
        }

        // ── drain incoming 88x31 buttons ─────────────────────
        if let Some(ref rx) = btn_rx {
            loop {
                match rx.try_recv() {
                    Ok((name, data)) => {
                        btn_pending = btn_pending.saturating_sub(1);
                        let sx = rand::gen_range(50.0_f32, sw - 50.0);
                        let sy = 40.0_f32;
                        if let Some(obj) = PhysicsImage::from_bytes(
                            &name, &data, (sx, sy),
                            &mut pw.bodies, &mut pw.colliders)
                        {
                            objects.push(obj);
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        btn_pending = 0;
                        btn_rx = None;
                        break;
                    }
                }
            }
            if btn_pending == 0 { btn_rx = None; }
        }

        // ── side-border logic ─────────────────────────────────
        if !paused {
            let sw_phys = sw / PPM;
            let sh_phys = sh / PPM;
            match pw.border_mode {
                BorderMode::Loop => {
                    for obj in &objects {
                        if let Some(body) = pw.bodies.get_mut(obj.body_handle) {
                            let pos = *body.translation();
                            if pos.x < -1.0 {
                                body.set_translation(vector![pos.x + sw_phys + 1.0, pos.y], true);
                            } else if pos.x > sw_phys + 1.0 {
                                body.set_translation(vector![pos.x - sw_phys - 1.0, pos.y], true);
                            }
                        }
                    }
                }
                BorderMode::Kill => {
                    let dead: Vec<usize> = objects.iter().enumerate()
                        .filter(|(_, obj)| {
                            let x = pw.bodies[obj.body_handle].translation().x;
                            x < -1.0 || x > sw_phys + 1.0
                        })
                        .map(|(i, _)| i)
                        .collect();
                    for i in dead.into_iter().rev() {
                        if drag.as_ref().map_or(false, |d| d.object_idx == i) {
                            drag = None;
                        }
                        let obj = objects.remove(i);
                        obj.destroy(&mut pw.bodies, &mut pw.colliders,
                                    &mut pw.island_manager,
                                    &mut pw.impulse_joints,
                                    &mut pw.multibody_joints);
                    }
                }
                BorderMode::Warp => {
                    // Wrap all four sides
                    for obj in &objects {
                        if let Some(body) = pw.bodies.get_mut(obj.body_handle) {
                            let pos = *body.translation();
                            let mut nx = pos.x;
                            let mut ny = pos.y;
                            if pos.x < -1.0        { nx = pos.x + sw_phys + 1.0; }
                            else if pos.x > sw_phys + 1.0 { nx = pos.x - sw_phys - 1.0; }
                            if pos.y < -1.0        { ny = pos.y + sh_phys + 1.0; }
                            else if pos.y > sh_phys + 1.0 { ny = pos.y - sh_phys - 1.0; }
                            if nx != pos.x || ny != pos.y {
                                body.set_translation(vector![nx, ny], true);
                            }
                        }
                    }
                }
                BorderMode::Bounce => {
                    // Strong impulse-based repulsion near edges (trampolines)
                    let margin = 1.5_f32;
                    let strength = 120.0_f32;
                    for obj in &objects {
                        if let Some(body) = pw.bodies.get_mut(obj.body_handle) {
                            let pos = *body.translation();
                            let vel = *body.linvel();
                            let mut imp = vector![0.0_f32, 0.0];
                            if pos.x < margin && vel.x < 0.0 {
                                imp.x += strength * (margin - pos.x) / margin;
                            }
                            if pos.x > sw_phys - margin && vel.x > 0.0 {
                                imp.x -= strength * (pos.x - (sw_phys - margin)) / margin;
                            }
                            if imp.x != 0.0 || imp.y != 0.0 {
                                body.apply_impulse(imp, true);
                            }
                        }
                    }
                }
                BorderMode::Repulse => {
                    // Soft continuous force pushing away from all walls
                    let zone = 3.0_f32;
                    let strength = 80.0_f32;
                    for obj in &objects {
                        if let Some(body) = pw.bodies.get_mut(obj.body_handle) {
                            let pos = *body.translation();
                            let mut fx = 0.0_f32;
                            let fy;
                            let dl = (pos.x).max(0.0);
                            let dr = (sw_phys - pos.x).max(0.0);
                            if dl < zone { fx +=  strength * (1.0 - dl / zone); }
                            if dr < zone { fx -= strength * (1.0 - dr / zone); }
                            // Also push off the floor gently
                            let db = (pos.y).max(0.0);
                            fy = if db < zone { strength * (1.0 - db / zone) } else { 0.0 };
                            if fx != 0.0 || fy != 0.0 {
                                body.reset_forces(true);
                                body.add_force(vector![fx, fy], true);
                                body.wake_up(true);
                            }
                        }
                    }
                }
                BorderMode::Portal => {
                    // All 4 edges are portals — wrap in every direction
                    for obj in &objects {
                        if let Some(body) = pw.bodies.get_mut(obj.body_handle) {
                            let pos = *body.translation();
                            let mut nx = pos.x;
                            let mut ny = pos.y;
                            if pos.x < -1.0             { nx = pos.x + sw_phys + 1.0; }
                            else if pos.x > sw_phys + 1.0 { nx = pos.x - sw_phys - 1.0; }
                            if pos.y < -1.0             { ny = pos.y + sh_phys + 1.0; }
                            else if pos.y > sh_phys + 1.0 { ny = pos.y - sh_phys - 1.0; }
                            if nx != pos.x || ny != pos.y {
                                body.set_translation(vector![nx, ny], true);
                            }
                        }
                    }
                }
                BorderMode::Default => {}
            }
        }

        // ── GIF animation + trail ─────────────────────────────
        let trail_len  = trail_len_slider.value as usize;
        let trail_fade = trail_fade_slider.value;
        for obj in &mut objects {
            obj.update_anim(dt_ms);
            obj.update_trail(&pw.bodies, trail_len, trail_fade);
        }

        // ══════════════════════════════════════════════════════
        // Draw
        // ══════════════════════════════════════════════════════
        background.draw(sw, sh);

        let ft  = (WALL_T * PPM) as f32;
        // Floor (always solid)
        draw_rectangle(0.0, sh - ft, sw, ft, Color::from_rgba(50, 50, 80, 255));
        // Side walls — solid in Default, animated indicator strips otherwise
        match pw.border_mode {
            BorderMode::Default => {
                draw_rectangle(0.0,      0.0, ft, sh, Color::from_rgba(50, 50, 80, 255));
                draw_rectangle(sw - ft, 0.0, ft, sh, Color::from_rgba(50, 50, 80, 255));
            }
            BorderMode::Loop => {
                let t       = get_time() as f32;
                let spacing = 52.0_f32;
                let scroll  = (t * 42.0) % spacing;
                let cx_l    = ft * 0.5;
                let cx_r    = sw - ft * 0.5;
                draw_rectangle(0.0,      0.0, ft, sh, Color::from_rgba(50, 110, 255, 45));
                draw_rectangle(sw - ft, 0.0, ft, sh, Color::from_rgba(50, 110, 255, 45));
                // Arrows pointing inward (→ on left, ← on right), scrolling down
                let arrow_col = Color::from_rgba(90, 170, 255, 210);
                let mut y = -scroll;
                while y < sh {
                    draw_triangle(
                        vec2(cx_l - 6.0, y - 5.0),
                        vec2(cx_l - 6.0, y + 5.0),
                        vec2(cx_l + 7.0, y),
                        arrow_col,
                    );
                    draw_triangle(
                        vec2(cx_r + 6.0, y - 5.0),
                        vec2(cx_r + 6.0, y + 5.0),
                        vec2(cx_r - 7.0, y),
                        arrow_col,
                    );
                    y += spacing;
                }
            }
            BorderMode::Kill => {
                let t       = get_time() as f32;
                let pulse   = (t * 3.0).sin() as f32 * 0.5 + 0.5;
                let strip_a = (40.0 + pulse * 40.0) as u8;
                let mark_a  = (130.0 + pulse * 90.0) as u8;
                let cx_l    = ft * 0.5;
                let cx_r    = sw - ft * 0.5;
                draw_rectangle(0.0,      0.0, ft, sh, Color::from_rgba(255, 45, 45, strip_a));
                draw_rectangle(sw - ft, 0.0, ft, sh, Color::from_rgba(255, 45, 45, strip_a));
                let mark_col = Color::from_rgba(255, 80, 60, mark_a);
                let spacing  = 52.0_f32;
                let s        = 5.0_f32;
                let mut y    = spacing * 0.5;
                while y < sh {
                    draw_line(cx_l - s, y - s, cx_l + s, y + s, 1.5, mark_col);
                    draw_line(cx_l + s, y - s, cx_l - s, y + s, 1.5, mark_col);
                    draw_line(cx_r - s, y - s, cx_r + s, y + s, 1.5, mark_col);
                    draw_line(cx_r + s, y - s, cx_r - s, y + s, 1.5, mark_col);
                    y += spacing;
                }
            }
            BorderMode::Warp => {
                // Cyan portals on all four edges, scrolling arrows on sides + top
                let t       = get_time() as f32;
                let spacing = 52.0_f32;
                let scroll  = (t * 42.0) % spacing;
                let col     = Color::from_rgba(0, 220, 200, 55);
                let acol    = Color::from_rgba(0, 220, 200, 200);
                draw_rectangle(0.0,      0.0, ft, sh, col);
                draw_rectangle(sw - ft, 0.0, ft, sh, col);
                draw_rectangle(0.0,      0.0, sw, ft, col);
                let cx_l = ft * 0.5;
                let cx_r = sw - ft * 0.5;
                let cy_t = ft * 0.5;
                let mut y = -scroll;
                while y < sh {
                    draw_triangle(vec2(cx_l - 6.0, y - 5.0), vec2(cx_l - 6.0, y + 5.0), vec2(cx_l + 7.0, y), acol);
                    draw_triangle(vec2(cx_r + 6.0, y - 5.0), vec2(cx_r + 6.0, y + 5.0), vec2(cx_r - 7.0, y), acol);
                    y += spacing;
                }
                let mut x = -scroll;
                while x < sw {
                    draw_triangle(vec2(x - 5.0, cy_t + 6.0), vec2(x + 5.0, cy_t + 6.0), vec2(x, cy_t - 7.0), acol);
                    x += spacing;
                }
            }
            BorderMode::Bounce => {
                // Green neon glow strips on side walls
                let t     = get_time() as f32;
                let pulse = (t * 4.0).sin() * 0.4 + 0.6;
                let a     = (60.0 * pulse) as u8;
                let ga    = (180.0 * pulse) as u8;
                draw_rectangle(0.0,      0.0, ft, sh, Color::from_rgba(60, 255, 100, a));
                draw_rectangle(sw - ft, 0.0, ft, sh, Color::from_rgba(60, 255, 100, a));
                draw_line(ft, 0.0, ft, sh, 1.5, Color::from_rgba(60, 255, 100, ga));
                draw_line(sw - ft, 0.0, sw - ft, sh, 1.5, Color::from_rgba(60, 255, 100, ga));
            }
            BorderMode::Repulse => {
                // Purple gradient glow fading inward on all sides
                let t     = get_time() as f32;
                let pulse = (t * 2.5).sin() * 0.3 + 0.7;
                let a     = (50.0 * pulse) as u8;
                let la    = (160.0 * pulse) as u8;
                draw_rectangle(0.0,      0.0, ft * 3.0, sh, Color::from_rgba(180, 80, 255, a));
                draw_rectangle(sw - ft * 3.0, 0.0, ft * 3.0, sh, Color::from_rgba(180, 80, 255, a));
                draw_rectangle(0.0, sh - ft * 3.0, sw, ft * 3.0, Color::from_rgba(180, 80, 255, a));
                draw_line(ft * 3.0, 0.0, ft * 3.0, sh, 1.0, Color::from_rgba(200, 100, 255, la));
                draw_line(sw - ft * 3.0, 0.0, sw - ft * 3.0, sh, 1.0, Color::from_rgba(200, 100, 255, la));
                draw_line(0.0, sh - ft * 3.0, sw, sh - ft * 3.0, 1.0, Color::from_rgba(200, 100, 255, la));
            }
            BorderMode::Portal => {
                // Animated rainbow portals on all 4 edges
                let t       = get_time() as f32;
                let spacing = 52.0_f32;
                let scroll  = (t * 38.0) % spacing;
                // Hue-cycling border strips
                let hue = (t * 0.4) % 1.0;
                let (pr, pg, pb) = hsv_to_rgb(hue, 0.8, 1.0);
                let (pr2, pg2, pb2) = hsv_to_rgb((hue + 0.5) % 1.0, 0.8, 1.0);
                let strip_a = 55u8;
                draw_rectangle(0.0,      0.0, ft, sh, Color::from_rgba(pr,  pg,  pb,  strip_a));
                draw_rectangle(sw - ft,  0.0, ft, sh, Color::from_rgba(pr2, pg2, pb2, strip_a));
                draw_rectangle(0.0,      0.0, sw, ft, Color::from_rgba(pr,  pg,  pb,  strip_a));
                draw_rectangle(0.0, sh - ft, sw, ft,  Color::from_rgba(pr2, pg2, pb2, strip_a));
                // Bright edge lines
                let line_a = 220u8;
                draw_line(ft, 0.0, ft, sh, 1.5, Color::from_rgba(pr,  pg,  pb,  line_a));
                draw_line(sw - ft, 0.0, sw - ft, sh, 1.5, Color::from_rgba(pr2, pg2, pb2, line_a));
                draw_line(0.0, ft, sw, ft, 1.5, Color::from_rgba(pr,  pg,  pb,  line_a));
                draw_line(0.0, sh - ft, sw, sh - ft, 1.5, Color::from_rgba(pr2, pg2, pb2, line_a));
                // Scrolling arrow pairs on all 4 edges
                let acol  = Color::from_rgba(pr,  pg,  pb,  210);
                let acol2 = Color::from_rgba(pr2, pg2, pb2, 210);
                let cx_l  = ft * 0.5;
                let cx_r  = sw - ft * 0.5;
                let cy_t  = ft * 0.5;
                let cy_b  = sh - ft * 0.5;
                let mut y = -scroll;
                while y < sh {
                    draw_triangle(vec2(cx_l - 5.0, y - 4.0), vec2(cx_l - 5.0, y + 4.0), vec2(cx_l + 6.0, y), acol);
                    draw_triangle(vec2(cx_r + 5.0, y - 4.0), vec2(cx_r + 5.0, y + 4.0), vec2(cx_r - 6.0, y), acol2);
                    y += spacing;
                }
                let mut x = -scroll;
                while x < sw {
                    draw_triangle(vec2(x - 4.0, cy_t + 5.0), vec2(x + 4.0, cy_t + 5.0), vec2(x, cy_t - 6.0), acol);
                    draw_triangle(vec2(x - 4.0, cy_b - 5.0), vec2(x + 4.0, cy_b - 5.0), vec2(x, cy_b + 6.0), acol2);
                    x += spacing;
                }
            }
        }

        if trail_enabled { for obj in &objects { obj.draw_trail(trail_fade); } }
        for obj in &objects { obj.draw(&pw.bodies); }

        if let Some(ref ds) = drag {
            let body = &pw.bodies[objects[ds.object_idx].body_handle];
            let (cx, cy) = to_screen(body.translation().x, body.translation().y);
            let (ax, ay) = to_screen(ds.anchor_world.0, ds.anchor_world.1);

            match ds.mode {
                DragMode::Spring => {
                    // Yellow spring line + cursor ring
                    draw_line(mx, my, cx, cy, 2.0, Color::from_rgba(255, 215, 50, 255));
                    draw_circle_lines(mx, my, 5.0, 1.0,
                                      Color::from_rgba(255, 215, 50, 255));
                }
                DragMode::Slingshot => {
                    // Magenta elastic bands (two parallel lines from anchor to object)
                    let band = Color::from_rgba(230, 80, 230, 255);
                    let hint = Color::from_rgba(255, 160, 255, 160);
                    let vx = cx - ax;
                    let vy = cy - ay;
                    let len = (vx*vx + vy*vy).sqrt().max(1.0);
                    let px = -vy / len * 5.0;
                    let py =  vx / len * 5.0;
                    draw_line(ax + px, ay + py, cx + px, cy + py, 2.0, band);
                    draw_line(ax - px, ay - py, cx - px, cy - py, 2.0, band);
                    draw_circle(ax, ay, 4.0, band);
                    // Dashed arrow showing launch direction
                    let lx = ax - (mx - ax);
                    let ly = ay - (my - ay);
                    draw_line(cx, cy, lx, ly, 1.5, hint);
                    draw_circle_lines(lx, ly, 5.0, 1.0, hint);
                }
                // Field modes — no per-drag visual, drawn below
                DragMode::Pull | DragMode::Push | DragMode::Vortex | DragMode::Freeze | DragMode::Orbit | DragMode::Bomb => {}
            }
        }

        // Radius outline + slider panel (Pull / Push mode)
        if show_sliders {
            let outline_col = if field_active {
                Color::from_rgba(130, 120, 220, 200)
            } else {
                Color::from_rgba(90, 85, 150, 120)
            };
            draw_circle_lines(mx, my, slider_radius.value, 1.5, outline_col);

            draw_rectangle(s_panel_x, s_panel_y, s_panel_w, s_panel_h,
                           Color::from_rgba(18, 16, 32, 230));
            draw_rectangle_lines(s_panel_x, s_panel_y, s_panel_w, s_panel_h, 1.0,
                                 Color::from_rgba(75, 70, 125, 255));
            slider_radius.draw(r_track,
                &format!("Radius  {}px", slider_radius.value as i32));
            slider_force.draw(f_track,
                &format!("Force   {:.0}", slider_force.value));
        }

        // Field mode visual indicator
        if field_active {
            let t = get_time() as f32;
            match drag_mode {
                DragMode::Pull => {
                    let col  = Color::from_rgba(80, 160, 255, 220);
                    let col2 = Color::from_rgba(80, 160, 255, 100);
                    draw_circle_lines(mx, my, 28.0, 1.5, col2);
                    draw_circle_lines(mx, my, 16.0, 1.5, col);
                    draw_circle_lines(mx, my,  6.0, 1.0, col);
                }
                DragMode::Push => {
                    let col = Color::from_rgba(255, 90, 60, 230);
                    for i in 0..8u8 {
                        let a = i as f32 * std::f32::consts::TAU / 8.0;
                        draw_line(mx, my,
                                  mx + a.cos() * 24.0,
                                  my + a.sin() * 24.0,
                                  1.5, col);
                    }
                    draw_circle(mx, my, 4.0, col);
                }
                DragMode::Vortex => {
                    // Spinning arcs around cursor
                    let col = Color::from_rgba(100, 230, 180, 230);
                    for i in 0..6u8 {
                        let base = i as f32 * std::f32::consts::TAU / 6.0 + t * 3.0;
                        let r1 = 10.0 + i as f32 * 3.0;
                        draw_circle_lines(
                            mx + base.cos() * r1,
                            my + base.sin() * r1,
                            3.0, 1.0, col);
                    }
                    draw_circle_lines(mx, my, 22.0, 1.0,
                                      Color::from_rgba(100, 230, 180, 100));
                }
                DragMode::Freeze => {
                    // Snowflake-like asterisk
                    let col = Color::from_rgba(180, 220, 255, 230);
                    for i in 0..6u8 {
                        let a = i as f32 * std::f32::consts::PI / 3.0;
                        draw_line(mx, my,
                                  mx + a.cos() * 22.0,
                                  my + a.sin() * 22.0,
                                  1.5, col);
                        let bx = mx + a.cos() * 14.0;
                        let by = my + a.sin() * 14.0;
                        let perp = a + std::f32::consts::FRAC_PI_2;
                        draw_line(bx - perp.cos() * 5.0, by - perp.sin() * 5.0,
                                  bx + perp.cos() * 5.0, by + perp.sin() * 5.0,
                                  1.0, col);
                    }
                    draw_circle(mx, my, 3.0, col);
                }
                DragMode::Orbit => {
                    // Dashed orbit ring with a spinning dot
                    let col  = Color::from_rgba(255, 200, 80, 200);
                    let col2 = Color::from_rgba(255, 200, 80, 80);
                    draw_circle_lines(mx, my, 24.0, 1.0, col2);
                    let a = t * 4.0;
                    draw_circle(mx + a.cos() * 24.0,
                                my + a.sin() * 24.0,
                                4.0, col);
                    draw_circle(mx, my, 3.0, col);
                }
                _ => {}
            }
        }

        // Bomb cursor indicator (pulsing crosshair)
        if drag_mode == DragMode::Bomb {
            let pulse = (get_time() as f32 * 5.0).sin() * 0.3 + 0.7;
            let col = Color::from_rgba(255, 90, 40, (200.0 * pulse) as u8);
            let r   = slider_radius.value;
            draw_circle_lines(mx, my, r, 1.0,
                              Color::from_rgba(255, 90, 40, 60));
            draw_circle_lines(mx, my, 12.0, 1.5, col);
            draw_line(mx - 18.0, my, mx - 6.0, my, 1.5, col);
            draw_line(mx + 6.0, my, mx + 18.0, my, 1.5, col);
            draw_line(mx, my - 18.0, mx, my - 6.0, 1.5, col);
            draw_line(mx, my + 6.0, mx, my + 18.0, 1.5, col);
        }

        // Draw active bomb animations, expire old ones
        gravity_bombs.retain(|b| b.alive());
        for bomb in &gravity_bombs { bomb.draw(); }

        // HUD bar
        draw_rectangle(0.0, hud_y, sw, HUD_H, Color::from_rgba(0, 0, 0, 130));
        // Peek-strip accent line visible when HUD is sliding away
        if hud_y < -1.0 {
            let t = (-hud_y / (HUD_H - 4.0)).clamp(0.0, 1.0);
            draw_rectangle(0.0, (hud_y + HUD_H).max(0.0), sw, 2.0,
                           Color::from_rgba(100, 90, 180, (t * 180.0) as u8));
        }
        draw_text(
            "A = images  |  N = spawner  |  F = 88x31 buttons  |  W = window shake  |  Left-drag = throw  |  Right-click = menu  |  M = music  |  D = debug",
            10.0, 18.0 + hud_y, 14.0, Color::from_rgba(160, 155, 210, 255));

        let right_str = format!(
            "Tab=drag:{}   B=sides:{}   G=bg:{}   R=clear   Space=pause   Q=quit   obj:{}{}",
            drag_mode.label(),
            pw.border_mode.label(),
            background.mode.label(),
            objects.len(),
            if paused { "  PAUSED" } else { "" },
        );
        let rw = measure_text(&right_str, None, 14, 1.0).width;
        draw_text(&right_str, sw - rw - 10.0, 18.0 + hud_y, 14.0,
                  Color::from_rgba(110, 105, 165, 255));

        grav_btns.draw(14.0, pw.gravity.y, hud_y);

        // ── music badge ───────────────────────────────────────
        {
            let mi = mod_player.info();
            let ri = reg_player.as_ref().map(|p| p.info());

            let badge_content: Option<(String, bool)> = if mi.loaded {
                let state = if mi.playing { "PLAYING" } else { "PAUSED" };
                let disp  = if mi.title.is_empty() { &mi.name } else { &mi.title };
                let short = if disp.len() > 24 { &disp[..24] } else { disp.as_str() };
                Some((format!("[{}] {} ({})  ORD {}/{}",
                    if mi.fmt.is_empty() { "MOD" } else { &mi.fmt },
                    short, state, mi.order + 1, mi.orders), mi.playing))
            } else if let Some(ri) = ri.filter(|r| r.loaded) {
                let state = if ri.playing { "PLAYING" } else { "PAUSED" };
                let short = if ri.name.len() > 28 { &ri.name[..28] } else { ri.name.as_str() };
                Some((format!("[{}] {} ({})", ri.fmt, short, state), ri.playing))
            } else {
                None
            };

            if let Some((badge, playing)) = badge_content {
                let bw  = measure_text(&badge, None, 13, 1.0).width;
                let bx  = sw / 2.0 - bw / 2.0;
                let by  = 6.0 + hud_y;
                let pad = 6.0;
                draw_rectangle(bx - pad, by, bw + pad * 2.0, 18.0,
                               Color::from_rgba(20, 18, 38, 200));
                draw_rectangle_lines(bx - pad, by, bw + pad * 2.0, 18.0, 1.0,
                                     Color::from_rgba(80, 75, 130, 200));
                let col = if playing {
                    Color::from_rgba(160, 220, 160, 255)
                } else {
                    Color::from_rgba(180, 175, 210, 200)
                };
                draw_text(&badge, bx, by + 13.0, 13.0, col);
            }

            if audio_loaded {
                vol_slider.draw(vol_track,
                    &format!("VOL  {:3.0}%", vol_slider.value * 100.0));
            }
        }

        // ── debug overlay ─────────────────────────────────────
        if show_debug {
            let fsize      = 13.0_f32;
            let line_h     = 18.0_f32;
            let pad        = 10.0_f32;
            let panel_w    = 240.0_f32;
            let header_rows = 5; // title + FPS + objects + gravity + paused
            let obj_rows    = objects.len() * 4;
            let total_rows  = header_rows + obj_rows;
            let panel_h     = pad * 2.0 + total_rows as f32 * line_h;
            let panel_x     = sw - panel_w - 8.0;
            let panel_y     = HUD_H + 8.0;

            draw_rectangle(panel_x, panel_y, panel_w, panel_h,
                           Color::from_rgba(15, 13, 28, 215));
            draw_rectangle_lines(panel_x, panel_y, panel_w, panel_h, 1.0,
                                 Color::from_rgba(70, 65, 120, 255));

            let tx      = panel_x + pad;
            let col_hdr = Color::from_rgba(200, 195, 255, 255);
            let col_key = Color::from_rgba(140, 135, 200, 255);
            let col_val = Color::from_rgba(100,  95, 155, 255);
            let mut ly  = panel_y + pad + fsize;

            let dbg_label = "DEBUG";
            draw_text(dbg_label, tx, ly, fsize, col_hdr);
            ly += line_h;

            draw_text(&format!("FPS: {}", get_fps()), tx, ly, fsize, col_key);
            ly += line_h;
            draw_text(&format!("Objects: {}", objects.len()), tx, ly, fsize, col_key);
            ly += line_h;
            draw_text(&format!("Gravity: {:+.1} m/s²", pw.gravity.y),
                      tx, ly, fsize, col_key);
            ly += line_h;
            draw_text(&format!("Paused: {}", if paused { "yes" } else { "no" }),
                      tx, ly, fsize, col_key);
            ly += line_h;

            for (i, obj) in objects.iter().enumerate() {
                let body = &pw.bodies[obj.body_handle];
                let pos  = body.translation();
                let vel  = body.linvel();
                let name = std::path::Path::new(&obj.path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(&obj.path);
                let label = if name.len() > 16 {
                    format!("#{} {}…", i, &name[..15])
                } else {
                    format!("#{} {}", i, name)
                };

                draw_text(&label, tx, ly, fsize, col_hdr);
                ly += line_h;
                draw_text(&format!("  pos ({:.1}, {:.1})", pos.x, pos.y),
                          tx, ly, fsize, col_val);
                ly += line_h;
                draw_text(&format!("  vel ({:.1}, {:.1})", vel.x, vel.y),
                          tx, ly, fsize, col_val);
                ly += line_h;
                draw_text(
                    &format!("  mass {:.2}  {}×{}px",
                             obj.fixed_mass,
                             obj.texture_w() as i32,
                             obj.texture_h() as i32),
                    tx, ly, fsize, col_val);
                ly += line_h;
            }
        }

        // ── trail panel (bottom-left) ─────────────────────────
        {
            let border = Color::from_rgba(75, 70, 125, 255);
            draw_rectangle(tp_x, tp_y, tp_w, tp_h, Color::from_rgba(18, 16, 32, 230));
            draw_rectangle_lines(tp_x, tp_y, tp_w, tp_h, 1.0, border);

            // Toggle button row
            let tog_bg = if trail_enabled {
                Color::from_rgba(55, 50, 95, 210)
            } else {
                Color::from_rgba(30, 28, 50, 210)
            };
            draw_rectangle(tp_tog.x, tp_tog.y, tp_tog.w, tp_tog.h, tog_bg);
            draw_rectangle_lines(tp_tog.x, tp_tog.y, tp_tog.w, tp_tog.h, 1.0, border);
            let tog_label = if trail_enabled { "TRAIL  ON" } else { "TRAIL  OFF" };
            let tog_col   = if trail_enabled {
                Color::from_rgba(150, 215, 150, 255)
            } else {
                Color::from_rgba(140, 135, 195, 200)
            };
            let tw = measure_text(tog_label, None, 13, 1.0).width;
            draw_text(tog_label, tp_tog.x + (tp_tog.w - tw) * 0.5,
                      tp_tog.y + 14.5, 13.0, tog_col);

            // Length + fade sliders (only when trail is on)
            if trail_enabled {
                trail_len_slider.draw(tp_sld,
                    &format!("Length  {}", trail_len_slider.value as usize));
                trail_fade_slider.draw(tp_sld2,
                    &format!("Fade  {:.1}s", trail_fade_slider.value));
            }
        }

        // Window-force panel
        if win_slide < 0.99 {
            draw_rectangle(wf_panel_x, wf_panel_y, wf_panel_w, wf_panel_h,
                           Color::from_rgba(18, 16, 32, 230));
            draw_rectangle_lines(wf_panel_x, wf_panel_y, wf_panel_w, wf_panel_h, 1.0,
                                 Color::from_rgba(75, 70, 125, 255));

            // ON / OFF toggle label (top-right corner of panel)
            let (tog_label, tog_col) = if win_shake_on {
                ("ON",  Color::from_rgba(150, 215, 150, 255))
            } else {
                ("OFF", Color::from_rgba(215, 100, 100, 255))
            };
            let tlw = measure_text(tog_label, None, 12, 1.0).width;
            draw_text(tog_label,
                      wf_panel_x + wf_panel_w - tlw - 8.0,
                      wf_panel_y + 14.0, 12.0, tog_col);

            let label = format!("Window Force  {:.1}  [W]", win_force_slider.value);
            win_force_slider.draw(wf_track, &label);
        }

        // Drag-mode picker (grid, above everything)
        // Fetch status overlay
        if btn_pending > 0 {
            let msg = format!("Fetching 88x31 buttons… {} left", btn_pending);
            let mw  = measure_text(&msg, None, 15, 1.0).width;
            let t   = get_time() as f32;
            let a   = ((t * 4.0).sin() * 0.25 + 0.75) as f32;
            draw_text(&msg, (sw - mw) / 2.0, sh / 2.0,
                      15.0, Color { a, ..Color::from_rgba(180, 200, 255, 255) });
        }

        drag_mode_menu.draw(drag_mode);
        spawner.draw();

        // Context menu is always on top
        menu.draw();

        next_frame().await;
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title:     "Image Gravity — Rust / macroquad".to_string(),
        window_width:     960,
        window_height:    680,
        window_resizable: true,
        ..Default::default()
    }
}
