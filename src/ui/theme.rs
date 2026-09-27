//! Visual language: palette, typography and drawing primitives shared by
//! every UI element.

use macroquad::prelude::*;
use std::cell::RefCell;

// ── Palette ─────────────────────────────────────────────────────
pub const SURFACE: Color = Color::new(0.075, 0.068, 0.125, 0.94);
pub const SURFACE_2: Color = Color::new(0.115, 0.105, 0.19, 1.0);
pub const SURFACE_HI: Color = Color::new(0.17, 0.155, 0.28, 1.0);
pub const BORDER: Color = Color::new(0.30, 0.28, 0.48, 0.55);
pub const BORDER_HI: Color = Color::new(0.55, 0.50, 0.90, 0.9);
pub const TEXT: Color = Color::new(0.93, 0.92, 0.99, 1.0);
pub const TEXT_DIM: Color = Color::new(0.68, 0.66, 0.84, 1.0);
pub const TEXT_MUTED: Color = Color::new(0.47, 0.45, 0.63, 1.0);
pub const ACCENT: Color = Color::new(0.55, 0.47, 1.0, 1.0);
pub const ACCENT_HI: Color = Color::new(0.70, 0.64, 1.0, 1.0);
pub const SUCCESS: Color = Color::new(0.45, 0.88, 0.60, 1.0);
pub const WARNING: Color = Color::new(1.0, 0.76, 0.35, 1.0);
pub const DANGER: Color = Color::new(1.0, 0.42, 0.42, 1.0);

pub const RADIUS: f32 = 10.0;

pub fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

pub fn fade(c: Color, f: f32) -> Color {
    Color { a: c.a * f, ..c }
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t)
}

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

// ── Typography ──────────────────────────────────────────────────
struct Fonts {
    regular: Font,
    bold: Font,
}

thread_local! {
    // Leaked on purpose: fonts own GPU textures, and dropping them from a
    // thread-local destructor at exit would touch an already-destroyed GL context.
    static FONTS: RefCell<Option<&'static Fonts>> = const { RefCell::new(None) };
}

/// Load the bundled Inter fonts. Falls back to macroquad's built-in font if
/// they cannot be parsed.
pub fn load_fonts() {
    let regular = load_ttf_font_from_bytes(include_bytes!("../../assets/fonts/Inter-Medium.otf"));
    let bold = load_ttf_font_from_bytes(include_bytes!("../../assets/fonts/InterDisplay-Bold.otf"));
    match (regular, bold) {
        (Ok(regular), Ok(bold)) => {
            let fonts: &'static Fonts = Box::leak(Box::new(Fonts { regular, bold }));
            FONTS.with(|f| *f.borrow_mut() = Some(fonts));
        }
        (r, b) => eprintln!("font load failed: {:?} {:?}", r.err(), b.err()),
    }
}

fn with_font<R>(bold: bool, f: impl FnOnce(Option<&Font>) -> R) -> R {
    FONTS.with(|fonts| {
        let fonts = *fonts.borrow();
        f(fonts.map(|x| if bold { &x.bold } else { &x.regular }))
    })
}

fn draw_str(s: &str, x: f32, y: f32, size: f32, color: Color, bold: bool) {
    with_font(bold, |font| {
        draw_text_ex(
            s,
            x.round(),
            y.round(),
            TextParams { font, font_size: size.round() as u16, color, ..Default::default() },
        );
    });
}

/// Draw text with its baseline at `y`.
pub fn text(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_str(s, x, y, size, color, false);
}

pub fn text_bold(s: &str, x: f32, y: f32, size: f32, color: Color) {
    draw_str(s, x, y, size, color, true);
}

pub fn measure(s: &str, size: f32) -> f32 {
    with_font(false, |font| measure_text(s, font, size.round() as u16, 1.0).width)
}

pub fn measure_bold(s: &str, size: f32) -> f32 {
    with_font(true, |font| measure_text(s, font, size.round() as u16, 1.0).width)
}

/// Baseline that vertically centres a line of text of `size` on `cy`.
pub fn baseline(cy: f32, size: f32) -> f32 {
    cy + size * 0.36
}

pub fn text_centered(s: &str, cx: f32, cy: f32, size: f32, color: Color) {
    text(s, cx - measure(s, size) / 2.0, baseline(cy, size), size, color);
}

pub fn text_bold_centered(s: &str, cx: f32, cy: f32, size: f32, color: Color) {
    text_bold(s, cx - measure_bold(s, size) / 2.0, baseline(cy, size), size, color);
}

/// Text vertically centred in `r`, left-aligned with `pad`.
pub fn text_left(r: Rect, pad: f32, s: &str, size: f32, color: Color) {
    text(s, r.x + pad, baseline(r.y + r.h / 2.0, size), size, color);
}

// ── Shapes ──────────────────────────────────────────────────────
fn vert(x: f32, y: f32, c: Color) -> Vertex {
    Vertex::new(x, y, 0.0, 0.0, 0.0, c)
}

fn rounded_outline(x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Vec2> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let seg = if r < 4.0 { 3 } else { 6 };
    let corners = [
        (x + w - r, y + r, -std::f32::consts::FRAC_PI_2),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, std::f32::consts::FRAC_PI_2),
        (x + r, y + r, std::f32::consts::PI),
    ];
    let mut pts = Vec::with_capacity(4 * (seg + 1));
    for (cx, cy, a0) in corners {
        for i in 0..=seg {
            let a = a0 + std::f32::consts::FRAC_PI_2 * i as f32 / seg as f32;
            pts.push(vec2(cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    pts
}

/// Filled rounded rectangle drawn as a single triangle fan (no overdraw, so
/// translucent colours blend correctly).
pub fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    if w <= 0.0 || h <= 0.0 || color.a <= 0.0 {
        return;
    }
    let pts = rounded_outline(x, y, w, h, r);
    let mut vertices = Vec::with_capacity(pts.len() + 1);
    vertices.push(vert(x + w / 2.0, y + h / 2.0, color));
    vertices.extend(pts.iter().map(|p| vert(p.x, p.y, color)));
    let n = pts.len() as u16;
    let mut indices = Vec::with_capacity(pts.len() * 3);
    for i in 0..n {
        indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % n]);
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

pub fn rounded_rect_lines(x: f32, y: f32, w: f32, h: f32, r: f32, thickness: f32, color: Color) {
    if color.a <= 0.0 {
        return;
    }
    let pts = rounded_outline(x + 0.5, y + 0.5, w - 1.0, h - 1.0, r);
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        draw_line(a.x, a.y, b.x, b.y, thickness, color);
    }
}

pub fn rrect(r: Rect, radius: f32, color: Color) {
    rounded_rect(r.x, r.y, r.w, r.h, radius, color);
}

pub fn rrect_lines(r: Rect, radius: f32, thickness: f32, color: Color) {
    rounded_rect_lines(r.x, r.y, r.w, r.h, radius, thickness, color);
}

/// Soft drop shadow under a rounded rectangle.
pub fn shadow(r: Rect, radius: f32, strength: f32) {
    for i in 1..=4 {
        let g = i as f32 * 3.0;
        rounded_rect(
            r.x - g + 2.0,
            r.y - g + 6.0,
            r.w + g * 2.0 - 4.0,
            r.h + g * 2.0 - 4.0,
            radius + g,
            Color::new(0.0, 0.0, 0.0, 0.07 * strength),
        );
    }
}

/// Standard floating panel: shadow, translucent surface, hairline border.
pub fn panel(r: Rect, f: f32) {
    if f <= 0.001 {
        return;
    }
    shadow(r, RADIUS, f);
    rrect(r, RADIUS, fade(SURFACE, f));
    rrect_lines(r, RADIUS, 1.0, fade(BORDER, f));
}

/// Vertical gradient quad.
pub fn gradient_v(x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    let vertices = vec![vert(x, y, top), vert(x + w, y, top), vert(x + w, y + h, bottom), vert(x, y + h, bottom)];
    draw_mesh(&Mesh { vertices, indices: vec![0, 1, 2, 0, 2, 3], texture: None });
}

/// Horizontal gradient quad.
pub fn gradient_h(x: f32, y: f32, w: f32, h: f32, left: Color, right: Color) {
    let vertices = vec![vert(x, y, left), vert(x + w, y, right), vert(x + w, y + h, right), vert(x, y + h, left)];
    draw_mesh(&Mesh { vertices, indices: vec![0, 1, 2, 0, 2, 3], texture: None });
}

/// Radial gradient disc fading from `inner` at the centre to `outer` at `r`.
pub fn radial(cx: f32, cy: f32, r: f32, inner: Color, outer: Color) {
    const N: u16 = 48;
    let mut vertices = Vec::with_capacity(N as usize + 1);
    vertices.push(vert(cx, cy, inner));
    for i in 0..N {
        let a = i as f32 / N as f32 * std::f32::consts::TAU;
        vertices.push(vert(cx + a.cos() * r, cy + a.sin() * r, outer));
    }
    let mut indices = Vec::with_capacity(N as usize * 3);
    for i in 0..N {
        indices.extend_from_slice(&[0, 1 + i, 1 + (i + 1) % N]);
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

/// Three-stop vertical gradient.
pub fn gradient_v3(x: f32, y: f32, w: f32, h: f32, top: Color, mid: Color, bottom: Color) {
    gradient_v(x, y, w, h / 2.0, top, mid);
    gradient_v(x, y + h / 2.0, w, h / 2.0, mid, bottom);
}

/// Draw a keyboard key cap and return its width.
pub fn keycap(x: f32, cy: f32, label: &str, size: f32, f: f32) -> f32 {
    let tw = measure(label, size);
    let w = (tw + size * 1.0).max(size * 1.7);
    let h = size * 1.7;
    let r = Rect::new(x, cy - h / 2.0, w, h);
    rrect(Rect::new(r.x, r.y + 2.0, r.w, r.h), 5.0, fade(Color::new(0.0, 0.0, 0.0, 0.35), f));
    rrect(r, 5.0, fade(SURFACE_HI, f));
    rrect_lines(r, 5.0, 1.0, fade(BORDER, f));
    text(label, x + (w - tw) / 2.0, baseline(cy, size), size, fade(TEXT, f));
    w
}

/// Small rounded "chip" with text; returns its rectangle.
pub fn chip(x: f32, cy: f32, label: &str, size: f32, fg: Color, bg: Color) -> Rect {
    let w = measure(label, size) + size * 1.4;
    let h = size * 1.9;
    let r = Rect::new(x, cy - h / 2.0, w, h);
    rrect(r, h / 2.0, bg);
    text(label, x + size * 0.7, baseline(cy, size), size, fg);
    r
}

pub fn chip_width(label: &str, size: f32) -> f32 {
    measure(label, size) + size * 1.4
}
