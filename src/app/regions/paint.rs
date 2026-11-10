//! Painting on the sheet (pencil, eraser, fill, taking a colour) and changing
//! its colours (always, and when Super), with a colour picker for both.
//! What is painted goes into the sheet itself, written again on saving; the
//! colours go with the regions.

use super::*;
use crate::util::hsv_to_rgb;

/// What a press on the sheet does while painting.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Brush {
    Pencil,
    Eraser,
    Fill,
    Dropper,
}

/// A stroke being painted: the sheet before it, the box painted so far
/// (x0, y0, x1, y1) and the last point.
struct Stroke {
    before: image::RgbaImage,
    area: [u32; 4],
    last: Vec2,
}

pub struct Paint {
    brush: Brush,
    colour: [u8; 3],
    /// The brush's side (sheet px).
    size: u32,
    /// The colour picker is open, and where it is (hue, saturation, value).
    picker: bool,
    hsv: [f32; 3],
    /// The sheet's colours, to take one quickly.
    quick: Vec<[u8; 3]>,
    stroke: Option<Stroke>,
}

pub struct Palette {
    /// The Super colours are edited (else the usual ones).
    pub sup: bool,
    /// The colours before the one being dragged in the picker (to undo).
    before: Option<(Vec<sheet::Swap>, Vec<sheet::Swap>)>,
    /// The character's colours, and the one being changed.
    colours: Vec<[u8; 3]>,
    chosen: Option<usize>,
    hsv: [f32; 3],
}

/// A swatch of the tools: the paint colour (opens the picker), a colour of
/// the sheet to paint with, or one of the character's colours to change.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Swatch {
    PaintColour,
    Quick([u8; 3]),
    Colour(usize),
}

/// The biggest brush.
const MOST_SIZE: u32 = 16;
/// Heights of the tools' bars along the bottom of the sheet view, and the
/// colour picker's size.
const PAINT_BAR: f32 = 48.0;
const PALETTE_BAR: f32 = 86.0;
const PICKER: Vec2 = vec2(330.0, 170.0);

fn rgb_to_hsv(c: [u8; 3]) -> [f32; 3] {
    let [r, g, b] = c.map(|v| v as f32 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    [h, if max == 0.0 { 0.0 } else { d / max }, max]
}

fn from_hsv([h, s, v]: [f32; 3]) -> [u8; 3] {
    let (r, g, b) = hsv_to_rgb(h, s, v);
    [r, g, b]
}

fn colour(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

/// The colours most used in `rects` of `img` (opaque pixels; the whole
/// picture when there are none), most used first, at most `most`; for a
/// lossy picture (`soft`), shades close to one already listed are left out.
pub fn colours_used(img: &image::RgbaImage, rects: &[Rect], soft: bool, most: usize) -> Vec<[u8; 3]> {
    let mut count: HashMap<[u8; 3], u32> = HashMap::new();
    let whole = [Rect::new(0.0, 0.0, img.width() as f32, img.height() as f32)];
    let mut seen = std::collections::HashSet::new();
    for r in if rects.is_empty() { &whole[..] } else { rects } {
        if !seen.insert([r.x, r.y, r.w, r.h].map(|v| v as i32)) {
            continue;
        }
        let (x0, y0) = (r.x.max(0.0) as u32, r.y.max(0.0) as u32);
        let (x1, y1) = (((r.x + r.w) as u32).min(img.width()), ((r.y + r.h) as u32).min(img.height()));
        for y in y0..y1 {
            for x in x0..x1 {
                let p = img.get_pixel(x, y);
                if p[3] > 200 {
                    *count.entry([p[0], p[1], p[2]]).or_default() += 1;
                }
            }
        }
    }
    let mut by_use: Vec<([u8; 3], u32)> = count.into_iter().collect();
    by_use.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let near = |a: [u8; 3], b: [u8; 3]| (0..3).map(|k| (a[k] as i32 - b[k] as i32).pow(2)).sum::<i32>() <= 28 * 28;
    let mut out: Vec<[u8; 3]> = vec![];
    for (c, _) in by_use {
        if out.len() >= most {
            break;
        }
        if !soft || !out.iter().any(|&o| near(o, c)) {
            out.push(c);
        }
    }
    out
}

/// Flood `img` from `(x, y)` with `to`: the pixels of the same colour
/// touching it (any see-through one, from a see-through one). Returns the
/// box filled (x0, y0, x1, y1), if any.
pub fn flood(img: &mut image::RgbaImage, x: u32, y: u32, to: image::Rgba<u8>) -> Option<[u32; 4]> {
    let (w, h) = img.dimensions();
    if x >= w || y >= h {
        return None;
    }
    let target = *img.get_pixel(x, y);
    let same = |p: &image::Rgba<u8>| if target[3] == 0 { p[3] == 0 } else { *p == target };
    if same(&to) {
        return None;
    }
    let mut area = [x, y, x + 1, y + 1];
    let mut todo = vec![(x, y)];
    img.put_pixel(x, y, to);
    while let Some((x, y)) = todo.pop() {
        area = [area[0].min(x), area[1].min(y), area[2].max(x + 1), area[3].max(y + 1)];
        let around = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
        for (nx, ny) in around {
            if nx < w && ny < h && same(img.get_pixel(nx, ny)) {
                img.put_pixel(nx, ny, to);
                todo.push((nx, ny));
            }
        }
    }
    Some(area)
}

impl RegionEditor {
    /// The character's colours: those changed first, then the most used by
    /// its pictures.
    fn character_colours(&self, most: usize) -> Vec<[u8; 3]> {
        let rects: Vec<Rect> = self.sheet.anims.values().flat_map(all_pics).map(|p| p.src).collect();
        let soft = self.sheet.soft || self.sheet.layout_soft;
        let changed = self.sheet.recolour.iter().chain(&self.sheet.super_colours).map(|s| s[0]);
        let mut out: Vec<[u8; 3]> = vec![];
        for c in changed.chain(colours_used(&self.sheet.image, &rects, soft, most)) {
            if out.len() < most && !out.contains(&c) {
                out.push(c);
            }
        }
        out
    }

    pub(super) fn toggle_paint(&mut self) {
        if self.paint.take().is_some() {
            return;
        }
        let quick = self.character_colours(24);
        let colour = quick.first().copied().unwrap_or([0, 0, 0]);
        self.paint = Some(Paint {
            brush: Brush::Pencil,
            colour,
            size: 1,
            picker: false,
            hsv: rgb_to_hsv(colour),
            quick,
            stroke: None,
        });
        (self.palette, self.grid, self.picking) = (None, None, false);
        self.notes.push(Note::Info("Paint: what you paint is kept in the sheet itself when you save".into()));
    }

    pub(super) fn toggle_palette(&mut self) {
        if self.palette.take().is_some() {
            return;
        }
        let most = ((layout_parts().1.w - 20.0) / 25.0).floor().max(4.0) as usize;
        let colours = self.character_colours(most);
        self.palette = Some(Palette { sup: false, before: None, colours, chosen: None, hsv: [0.0, 0.0, 0.0] });
        (self.paint, self.grid, self.picking) = (None, None, false);
    }

    /// Esc: the colour picker closes, else the tool in use. Whether one did.
    pub(super) fn close_tool(&mut self) -> bool {
        if let Some(p) = self.paint.as_mut() {
            if p.picker {
                p.picker = false;
            } else {
                self.paint = None;
            }
            return true;
        }
        if let Some(p) = self.palette.as_mut() {
            if p.chosen.is_some() {
                p.chosen = None;
            } else {
                self.palette = None;
            }
            return true;
        }
        false
    }

    pub(super) fn tool_command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::ToolDone => {
                (self.paint, self.palette) = (None, None);
            }
            Cmd::PutBack => {
                let Some(p) = &self.palette else { return };
                let Some(from) = p.chosen.and_then(|k| p.colours.get(k)).copied() else { return };
                let sup = p.sup;
                let before = Step::Colours(self.sheet.recolour.clone(), self.sheet.super_colours.clone());
                self.push_step(before);
                let list = if sup { &mut self.sheet.super_colours } else { &mut self.sheet.recolour };
                list.retain(|s| s[0] != from);
                self.retexture = Some(0.0);
                let now = self.colour_now(from);
                if let Some(p) = self.palette.as_mut() {
                    p.hsv = rgb_to_hsv(now);
                }
            }
            _ => {}
        }
        if let Some(p) = self.palette.as_mut() {
            match cmd {
                Cmd::ColoursUsual | Cmd::ColoursSuper => {
                    p.sup = cmd == Cmd::ColoursSuper;
                    let chosen = p.chosen.and_then(|k| p.colours.get(k)).copied();
                    if let Some(c) = chosen {
                        let now = self.colour_now(c);
                        if let Some(p) = self.palette.as_mut() {
                            p.hsv = rgb_to_hsv(now);
                        }
                    }
                }
                _ => {}
            }
            return;
        }
        let Some(p) = self.paint.as_mut() else { return };
        match cmd {
            Cmd::Brush(b) => p.brush = b,
            Cmd::ColourPicker => p.picker = !p.picker,
            Cmd::SizeLess => p.size = p.size.saturating_sub(1).max(1),
            Cmd::SizeMore => p.size = (p.size + 1).min(MOST_SIZE),
            _ => {}
        }
    }

    /// What `from` becomes in the colours being edited (usual or Super).
    fn colour_now(&self, from: [u8; 3]) -> [u8; 3] {
        let find = |list: &[sheet::Swap]| list.iter().find(|s| s[0] == from).map(|s| s[1]);
        let usual = find(&self.sheet.recolour).unwrap_or(from);
        match &self.palette {
            Some(p) if p.sup => find(&self.sheet.super_colours).unwrap_or(usual),
            _ => usual,
        }
    }

    /// The bar's height along the bottom of the sheet view, for the tool in use.
    fn bar_height(&self) -> Option<f32> {
        if self.paint.is_some() {
            Some(PAINT_BAR)
        } else if self.palette.is_some() {
            Some(PALETTE_BAR)
        } else {
            None
        }
    }

    /// Where the colour picker is, when open.
    fn picker_rect(&self, view: Rect) -> Option<Rect> {
        let open =
            self.paint.as_ref().is_some_and(|p| p.picker) || self.palette.as_ref().is_some_and(|p| p.chosen.is_some());
        let bar = self.bar_height()?;
        open.then(|| Rect::new(view.x + 10.0, view.bottom() - bar - 8.0 - PICKER.y, PICKER.x, PICKER.y))
    }

    /// The picker's parts: its shades (saturation across, value up) and hues.
    fn picker_parts(r: Rect) -> (Rect, Rect) {
        (Rect::new(r.x + 10.0, r.y + 10.0, 140.0, 140.0), Rect::new(r.x + 158.0, r.y + 10.0, 16.0, 140.0))
    }

    /// The tools' buttons, steppers and swatches; returns where the colour
    /// picker is, when open.
    pub(super) fn tool_bars(
        &self,
        view: Rect,
        buttons: &mut Vec<Button>,
        steppers: &mut Vec<Stepper>,
        swatches: &mut Vec<(Rect, Swatch)>,
    ) -> Option<Rect> {
        let mut push = |r: Rect, label: &str, cmd: Cmd, on: bool| {
            buttons.push(Button { r, label: label.to_string(), cmd, on, enabled: true });
        };
        let picker = self.picker_rect(view);
        if let Some(p) = &self.paint {
            let y = view.bottom() - 40.0;
            let mut x = view.x + 10.0;
            for (label, b, w) in [
                ("Pencil", Brush::Pencil, 66.0),
                ("Eraser", Brush::Eraser, 66.0),
                ("Fill", Brush::Fill, 50.0),
                ("Pick", Brush::Dropper, 50.0),
            ] {
                push(Rect::new(x, y, w, 30.0), label, Cmd::Brush(b), p.brush == b);
                x += w + 6.0;
            }
            swatches.push((Rect::new(x, y, 30.0, 30.0), Swatch::PaintColour));
            x += 36.0;
            steppers.push(Stepper {
                r: Rect::new(x, y, 100.0, 30.0),
                label: "Size",
                value: p.size.to_string(),
                less: Cmd::SizeLess,
                more: Cmd::SizeMore,
                enabled: true,
            });
            push(Rect::new(view.right() - 70.0, y, 60.0, 30.0), "Done", Cmd::ToolDone, false);
            if let Some(r) = picker {
                for (i, &c) in p.quick.iter().enumerate() {
                    let (col, row) = ((i % 6) as f32, (i / 6) as f32);
                    let at = vec2(r.x + 186.0 + col * 22.0, r.y + 78.0 + row * 22.0);
                    swatches.push((Rect::new(at.x, at.y, 18.0, 18.0), Swatch::Quick(c)));
                }
            }
        }
        if let Some(p) = &self.palette {
            let y = view.bottom() - 80.0;
            push(Rect::new(view.x + 10.0, y, 70.0, 28.0), "Usual", Cmd::ColoursUsual, !p.sup);
            push(Rect::new(view.x + 86.0, y, 70.0, 28.0), "Super", Cmd::ColoursSuper, p.sup);
            push(Rect::new(view.right() - 70.0, y, 60.0, 28.0), "Done", Cmd::ToolDone, false);
            let swapped = p.chosen.and_then(|k| p.colours.get(k)).is_some_and(|&c| {
                let list = if p.sup { &self.sheet.super_colours } else { &self.sheet.recolour };
                list.iter().any(|s| s[0] == c)
            });
            buttons.push(Button {
                r: Rect::new(view.right() - 160.0, y, 84.0, 28.0),
                label: "Put back".into(),
                cmd: Cmd::PutBack,
                on: false,
                enabled: swapped,
            });
            for k in 0..p.colours.len() {
                let r = Rect::new(view.x + 10.0 + k as f32 * 25.0, view.bottom() - 44.0, 22.0, 34.0);
                if r.right() <= view.right() - 8.0 {
                    swatches.push((r, Swatch::Colour(k)));
                }
            }
        }
        picker
    }

    pub(super) fn tool_swatch(&mut self, sw: Swatch) {
        match sw {
            Swatch::PaintColour => self.tool_command(Cmd::ColourPicker),
            Swatch::Quick(c) => self.set_picked(c),
            Swatch::Colour(k) => {
                let Some(from) = self.palette.as_ref().and_then(|p| p.colours.get(k)).copied() else { return };
                let now = self.colour_now(from);
                if let Some(p) = self.palette.as_mut() {
                    p.chosen = if p.chosen == Some(k) { None } else { Some(k) };
                    p.hsv = rgb_to_hsv(now);
                }
            }
        }
    }

    /// The colour the picker shows.
    fn picked(&self) -> Option<[u8; 3]> {
        if let Some(p) = &self.paint {
            return Some(p.colour);
        }
        let p = self.palette.as_ref()?;
        let from = p.colours.get(p.chosen?)?;
        Some(self.colour_now(*from))
    }

    /// The picker's colour becomes the paint's, or the one the chosen
    /// colour becomes.
    fn set_picked(&mut self, c: [u8; 3]) {
        self.set_picked_hsv(c, rgb_to_hsv(c));
    }

    fn set_picked_hsv(&mut self, c: [u8; 3], hsv: [f32; 3]) {
        if let Some(p) = self.paint.as_mut() {
            p.colour = c;
            p.hsv = hsv;
            if matches!(p.brush, Brush::Eraser | Brush::Dropper) {
                p.brush = Brush::Pencil;
            }
            return;
        }
        let Some(p) = self.palette.as_mut() else { return };
        let Some(&from) = p.chosen.and_then(|k| p.colours.get(k)) else { return };
        p.hsv = hsv;
        let sup = p.sup;
        let list = if sup { &mut self.sheet.super_colours } else { &mut self.sheet.recolour };
        list.retain(|s| s[0] != from);
        // A colour left as it was needs no swap (unless it stays so when Super).
        if sup || c != from {
            list.push([from, c]);
        }
        self.retexture.get_or_insert(get_time());
    }

    fn hsv(&self) -> [f32; 3] {
        match (&self.paint, &self.palette) {
            (Some(p), _) => p.hsv,
            (_, Some(p)) => p.hsv,
            _ => [0.0; 3],
        }
    }

    pub(super) fn picker_press(&mut self, l: &Layout, m: Vec2) {
        let Some(r) = l.picker else { return };
        let colours = (self.sheet.recolour.clone(), self.sheet.super_colours.clone());
        if let Some(p) = self.palette.as_mut() {
            p.before = Some(colours);
        }
        let (shade, hue) = Self::picker_parts(r);
        if shade.contains(m) {
            self.drag = Some(Drag::Shade);
        } else if hue.contains(m) {
            self.drag = Some(Drag::Hue);
        } else {
            return;
        }
        self.picker_drag(m);
    }

    pub(super) fn picker_drag(&mut self, m: Vec2) {
        let Some(r) = self.picker_rect(layout_parts().1) else { return };
        let (shade, hue) = Self::picker_parts(r);
        let mut hsv = self.hsv();
        match self.drag {
            Some(Drag::Shade) => {
                hsv[1] = ((m.x - shade.x) / shade.w).clamp(0.0, 1.0);
                hsv[2] = 1.0 - ((m.y - shade.y) / shade.h).clamp(0.0, 1.0);
            }
            Some(Drag::Hue) => hsv[0] = ((m.y - hue.y) / hue.h).clamp(0.0, 0.999),
            _ => return,
        }
        self.set_picked_hsv(from_hsv(hsv), hsv);
    }

    /// The picker let go: the colours as they were, to undo (when they changed).
    pub(super) fn keep_colours(&mut self) {
        let Some((usual, sup)) = self.palette.as_mut().and_then(|p| p.before.take()) else { return };
        if usual != self.sheet.recolour || sup != self.sheet.super_colours {
            self.push_step(Step::Colours(usual, sup));
        }
    }

    /// The colours changed a while ago (or the picker was let go): the
    /// sheet is drawn with them.
    pub(super) fn retexture_now(&mut self) {
        let Some(since) = self.retexture else { return };
        let dragging = matches!(self.drag, Some(Drag::Shade | Drag::Hue));
        if !dragging || get_time() - since > 0.12 {
            self.sheet.retexture();
            self.retexture = None;
        }
    }

    // ── Painting ────────────────────────────────────────────────

    /// The pointer pressed on the sheet while painting.
    pub(super) fn paint_press(&mut self, p: Vec2) -> Option<Drag> {
        let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        let brush = self.paint.as_ref()?.brush;
        if alt || brush == Brush::Dropper {
            self.take_colour(p);
            return None;
        }
        if brush == Brush::Fill {
            self.fill(p);
            return None;
        }
        let before = self.sheet.original.clone();
        self.paint.as_mut()?.stroke = Some(Stroke { before, area: [u32::MAX, u32::MAX, 0, 0], last: p });
        self.dab(p, p);
        Some(Drag::Paint)
    }

    pub(super) fn paint_to(&mut self, p: Vec2) {
        let Some(last) = self.paint.as_ref().and_then(|t| t.stroke.as_ref()).map(|s| s.last) else { return };
        self.dab(last, p);
    }

    /// Paint along from `a` to `b` (sheet points).
    fn dab(&mut self, a: Vec2, b: Vec2) {
        let Some(paint) = &self.paint else { return };
        let size = paint.size as i64;
        let px = match paint.brush {
            Brush::Eraser => image::Rgba([0, 0, 0, 0]),
            _ => image::Rgba([paint.colour[0], paint.colour[1], paint.colour[2], 255]),
        };
        let (w, h) = self.sheet.original.dimensions();
        let steps = (b - a).length().ceil().max(1.0) as usize;
        let half = (size as f32 - 1.0) / 2.0;
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0u32, 0u32);
        for i in 0..=steps {
            let q = a + (b - a) * (i as f32 / steps as f32);
            let (sx, sy) = ((q.x - half).floor() as i64, (q.y - half).floor() as i64);
            for y in sy.max(0)..(sy + size).min(h as i64) {
                for x in sx.max(0)..(sx + size).min(w as i64) {
                    let (x, y) = (x as u32, y as u32);
                    self.sheet.original.put_pixel(x, y, px);
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        if let Some(s) = self.paint.as_mut().and_then(|t| t.stroke.as_mut()) {
            s.last = b;
            if x0 < x1 {
                s.area = [s.area[0].min(x0), s.area[1].min(y0), s.area[2].max(x1), s.area[3].max(y1)];
            }
        }
        if x0 < x1 {
            self.sheet.repaint([x0, y0, x1 - x0, y1 - y0]);
        }
    }

    /// The stroke done: kept to undo.
    pub(super) fn end_stroke(&mut self) {
        let Some(s) = self.paint.as_mut().and_then(|t| t.stroke.take()) else { return };
        self.keep_pixels(&s.before, s.area);
    }

    /// What was in `area` (x0, y0, x1, y1) of `before` kept to undo.
    fn keep_pixels(&mut self, before: &image::RgbaImage, area: [u32; 4]) {
        let [x0, y0, x1, y1] = area;
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let was = image::imageops::crop_imm(before, x0, y0, x1 - x0, y1 - y0).to_image();
        self.push_step(Step::Pixels((x0, y0), was));
        self.find_sprites();
    }

    /// Put `img` on the sheet at `at`; returns what was there.
    pub(super) fn put_pixels(&mut self, at: (u32, u32), img: &image::RgbaImage) -> image::RgbaImage {
        let (w, h) = img.dimensions();
        let was = image::imageops::crop_imm(&self.sheet.original, at.0, at.1, w, h).to_image();
        image::imageops::replace(&mut self.sheet.original, img, at.0 as i64, at.1 as i64);
        self.sheet.repaint([at.0, at.1, w, h]);
        was
    }

    fn fill(&mut self, p: Vec2) {
        let Some(c) = self.paint.as_ref().map(|t| t.colour) else { return };
        if p.x < 0.0 || p.y < 0.0 {
            return;
        }
        let before = self.sheet.original.clone();
        let to = image::Rgba([c[0], c[1], c[2], 255]);
        let Some(area) = flood(&mut self.sheet.original, p.x as u32, p.y as u32, to) else { return };
        self.sheet.repaint([area[0], area[1], area[2] - area[0], area[3] - area[1]]);
        self.keep_pixels(&before, area);
    }

    /// The colour under `p` to paint with (see-through: the eraser).
    fn take_colour(&mut self, p: Vec2) {
        let (w, h) = self.sheet.image.dimensions();
        if p.x < 0.0 || p.y < 0.0 || p.x >= w as f32 || p.y >= h as f32 {
            return;
        }
        let px = *self.sheet.image.get_pixel(p.x as u32, p.y as u32);
        if px[3] < 128 {
            if let Some(t) = self.paint.as_mut() {
                t.brush = Brush::Eraser;
            }
            self.notes.push(Note::Info("See-through there: the eraser".into()));
            return;
        }
        self.set_picked([px[0], px[1], px[2]]);
    }

    // ── Colours ─────────────────────────────────────────────────

    /// A click on the sheet while changing colours: the colour under it is
    /// the one to change (listed if it was not).
    pub(super) fn palette_pick(&mut self, p: Vec2) {
        let (w, h) = self.sheet.image.dimensions();
        if p.x < 0.0 || p.y < 0.0 || p.x >= w as f32 || p.y >= h as f32 {
            return;
        }
        let px = *self.sheet.image.get_pixel(p.x as u32, p.y as u32);
        if px[3] < 128 {
            self.notes.push(Note::Info("See-through there: click a colour of the character".into()));
            return;
        }
        let c = [px[0], px[1], px[2]];
        let soft = self.sheet.soft || self.sheet.layout_soft;
        let near = |a: [u8; 3]| (0..3).map(|k| (a[k] as i32 - c[k] as i32).pow(2)).sum::<i32>() <= 28 * 28;
        let Some(pal) = self.palette.as_mut() else { return };
        let k = match pal.colours.iter().position(|&o| o == c || (soft && near(o))) {
            Some(k) => k,
            None => {
                let most = pal.colours.len().max(4);
                pal.colours.insert(0, c);
                pal.colours.truncate(most);
                0
            }
        };
        pal.chosen = Some(k);
        let from = pal.colours[k];
        let now = self.colour_now(from);
        if let Some(pal) = self.palette.as_mut() {
            pal.hsv = rgb_to_hsv(now);
        }
    }

    // ── Drawing ─────────────────────────────────────────────────

    pub(super) fn tool_caption(&self) -> Option<String> {
        if let Some(p) = &self.paint {
            return Some(
                match p.brush {
                    Brush::Pencil => {
                        "Paint: click or drag on the sheet  ·  Alt-click takes a colour  ·  P or Esc: done"
                    }
                    Brush::Eraser => "Erase: drag on the sheet to make it see-through  ·  P or Esc: done",
                    Brush::Fill => "Fill: click a patch of one colour  ·  Alt-click takes a colour  ·  P or Esc: done",
                    Brush::Dropper => "Take a colour: click it on the sheet",
                }
                .to_string(),
            );
        }
        let p = self.palette.as_ref()?;
        Some(if p.sup && self.sheet.super_colours.is_empty() {
            "Super colours: none yet (the game makes the character golden)  ·  choose one to change".to_string()
        } else if p.sup {
            "Super colours: what each colour becomes when Super  ·  click one here or on the sheet".to_string()
        } else {
            "Colours: choose one (below or on the sheet), then what it becomes  ·  Esc: done".to_string()
        })
    }

    /// The tools' bars behind their buttons (drawn with the sheet view).
    pub(super) fn draw_tool_bars(&self, v: Rect) {
        if let Some(h) = self.bar_height() {
            let r = Rect::new(v.x + 1.0, v.bottom() - h, v.w - 2.0, h - 1.0);
            draw_rectangle(r.x, r.y, r.w, r.h, alpha(SURFACE, 0.94));
        }
    }

    /// The tools' swatches and the colour picker (over the buttons).
    pub(super) fn draw_tools(&self, l: &Layout, m: Vec2) {
        if let Some(r) = l.picker {
            self.draw_picker(r);
        }
        let chosen = self.palette.as_ref().and_then(|p| p.chosen);
        for &(r, sw) in &l.tool_swatches {
            match sw {
                Swatch::PaintColour => {
                    let Some(p) = &self.paint else { continue };
                    if p.brush == Brush::Eraser {
                        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.30, 0.30, 0.34, 1.0));
                        draw_rectangle(r.x, r.y, r.w / 2.0, r.h / 2.0, Color::new(0.22, 0.22, 0.26, 1.0));
                        draw_rectangle(
                            r.x + r.w / 2.0,
                            r.y + r.h / 2.0,
                            r.w / 2.0,
                            r.h / 2.0,
                            Color::new(0.22, 0.22, 0.26, 1.0),
                        );
                    } else {
                        draw_rectangle(r.x, r.y, r.w, r.h, colour(p.colour));
                    }
                    let edge = if p.picker || r.contains(m) { WHITE } else { BORDER_HI };
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, edge);
                }
                Swatch::Quick(c) => {
                    draw_rectangle(r.x, r.y, r.w, r.h, colour(c));
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, if r.contains(m) { WHITE } else { BORDER });
                }
                Swatch::Colour(k) => {
                    let Some(&from) = self.palette.as_ref().and_then(|p| p.colours.get(k)) else { continue };
                    // What it was above, what it becomes below.
                    let half = r.h / 2.0;
                    draw_rectangle(r.x, r.y, r.w, half, colour(from));
                    draw_rectangle(r.x, r.y + half, r.w, half, colour(self.colour_now(from)));
                    let lit = chosen == Some(k);
                    let edge = if lit {
                        WHITE
                    } else if r.contains(m) {
                        BORDER_HI
                    } else {
                        BORDER
                    };
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, if lit { 2.0 } else { 1.0 }, edge);
                }
            }
        }
    }

    fn draw_picker(&self, r: Rect) {
        rrect(r, RADIUS, SURFACE_2);
        rrect_lines(r, RADIUS, 1.0, BORDER_HI);
        let (shade, hue) = Self::picker_parts(r);
        let hsv = self.hsv();
        // Shades of the hue: saturation across, value up.
        const CELLS: usize = 28;
        let cell = shade.w / CELLS as f32;
        for i in 0..CELLS {
            for j in 0..CELLS {
                let (s, v) = ((i as f32 + 0.5) / CELLS as f32, 1.0 - (j as f32 + 0.5) / CELLS as f32);
                let c = colour(from_hsv([hsv[0], s, v]));
                draw_rectangle(shade.x + i as f32 * cell, shade.y + j as f32 * cell, cell + 0.5, cell + 0.5, c);
            }
        }
        let at = vec2(shade.x + hsv[1] * shade.w, shade.y + (1.0 - hsv[2]) * shade.h);
        draw_circle_lines(at.x, at.y, 5.0, 2.0, if hsv[2] > 0.5 { BLACK } else { WHITE });
        let step = hue.h / 36.0;
        for k in 0..36 {
            let c = colour(from_hsv([k as f32 / 36.0, 1.0, 1.0]));
            draw_rectangle(hue.x, hue.y + k as f32 * step, hue.w, step + 0.5, c);
        }
        let hy = hue.y + hsv[0] * hue.h;
        draw_rectangle_lines(hue.x - 2.0, hy - 2.0, hue.w + 4.0, 4.0, 2.0, WHITE);
        // The colour, and for one being changed, what it was.
        let x = r.x + 186.0;
        let now = self.picked().unwrap_or([0, 0, 0]);
        let was = self.palette.as_ref().and_then(|p| p.colours.get(p.chosen?)).copied();
        match was {
            Some(w) => {
                draw_rectangle(x, r.y + 10.0, 60.0, 34.0, colour(w));
                draw_rectangle(x + 60.0, r.y + 10.0, 60.0, 34.0, colour(now));
                text("was", x + 2.0, r.y + 56.0, 10.0, TEXT_MUTED);
                text("becomes", x + 62.0, r.y + 56.0, 10.0, TEXT_MUTED);
            }
            None => draw_rectangle(x, r.y + 10.0, 120.0, 34.0, colour(now)),
        }
        draw_rectangle_lines(x, r.y + 10.0, 120.0, 34.0, 1.0, BORDER_HI);
        let rgb = format!("{}, {}, {}", now[0], now[1], now[2]);
        if was.is_some() {
            text(&rgb, x, r.y + 72.0, 11.0, TEXT_DIM);
        } else {
            text(&rgb, x, r.y + 58.0, 11.0, TEXT_DIM);
            text("The sheet's colours:", x, r.y + 73.0, 10.0, TEXT_MUTED);
        }
    }

    /// The brush on the sheet, where it would paint.
    pub(super) fn draw_brush(&self, v: Rect, m: Vec2) {
        let Some(p) = &self.paint else { return };
        let bar = v.bottom() - PAINT_BAR;
        let over_picker = self.picker_rect(v).is_some_and(|r| r.contains(m));
        if !v.contains(m) || m.y >= bar || over_picker || matches!(self.drag, Some(Drag::Shade | Drag::Hue)) {
            return;
        }
        let q = self.to_sheet(m);
        match p.brush {
            Brush::Pencil | Brush::Eraser => {
                let half = (p.size as f32 - 1.0) / 2.0;
                let corner = (q - Vec2::splat(half)).floor();
                let r = self.screen_rect(Rect::new(corner.x, corner.y, p.size as f32, p.size as f32));
                if p.brush == Brush::Pencil && self.zoom >= 2.0 {
                    draw_rectangle(r.x, r.y, r.w, r.h, alpha(colour(p.colour), 0.6));
                }
                draw_rectangle_lines(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, 1.0, WHITE);
            }
            Brush::Fill | Brush::Dropper => {
                draw_line(m.x - 9.0, m.y, m.x + 9.0, m.y, 1.0, WHITE);
                draw_line(m.x, m.y - 9.0, m.x, m.y + 9.0, 1.0, WHITE);
                if p.brush == Brush::Fill {
                    draw_rectangle(m.x + 10.0, m.y + 10.0, 14.0, 14.0, colour(p.colour));
                    draw_rectangle_lines(m.x + 10.0, m.y + 10.0, 14.0, 14.0, 1.0, WHITE);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_go_round_hsv_and_back() {
        for c in [[0, 0, 0], [255, 255, 255], [255, 0, 0], [12, 200, 90], [40, 60, 220], [128, 128, 64]] {
            let back = from_hsv(rgb_to_hsv(c));
            for k in 0..3 {
                assert!((back[k] as i32 - c[k] as i32).abs() <= 1, "{c:?} came back as {back:?}");
            }
        }
    }

    #[test]
    fn floods_fill_one_patch_and_the_most_used_colours_are_listed() {
        let mut img = image::RgbaImage::from_pixel(6, 4, image::Rgba([0, 0, 255, 255]));
        // A red wall down the middle splits the blue in two.
        for y in 0..4 {
            img.put_pixel(3, y, image::Rgba([255, 0, 0, 255]));
        }
        img.put_pixel(5, 0, image::Rgba([0, 0, 0, 0]));
        let area = flood(&mut img, 0, 0, image::Rgba([0, 255, 0, 255]));
        assert_eq!(area, Some([0, 0, 3, 4]));
        assert_eq!(img.get_pixel(2, 3).0, [0, 255, 0, 255]);
        assert_eq!(img.get_pixel(4, 0).0, [0, 0, 255, 255], "beyond the wall: not filled");
        // Filling with the colour it is: nothing.
        assert_eq!(flood(&mut img, 0, 0, image::Rgba([0, 255, 0, 255])), None);
        // A see-through patch fills from any see-through pixel.
        assert_eq!(flood(&mut img, 5, 0, image::Rgba([9, 9, 9, 255])), Some([5, 0, 6, 1]));
        // Green (12) over red (4) over blue (7), the see-through one aside.
        let used = colours_used(&img, &[], false, 8);
        assert_eq!(used, [[0, 255, 0], [0, 0, 255], [255, 0, 0], [9, 9, 9]]);
        assert_eq!(colours_used(&img, &[Rect::new(3.0, 0.0, 1.0, 4.0)], false, 8), [[255, 0, 0]]);
    }
}
