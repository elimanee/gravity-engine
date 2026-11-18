//! The terrain editor (sprite region editor → Zone → *Terrain…*): how a
//! tileset's pieces make the ground. Its columns go side by side, in order;
//! each is its pieces from the top down (tall grass, a tuft, the grass into
//! the earth along the ground's top, the earth and the piece under it).
//! Here they are chosen on the sheet, piece by piece, with the ground they
//! make shown as they change. Kept as `zone.terrain.json`.

use super::skins::{bake_terrain, bake_terrain_ledge};
use super::*;
use crate::sonic::zone::{self, Column, Grid, Piece, Terrain};
use crate::ui::theme::*;
use crate::ui::widgets::{button, clip, draw_button, draw_stepper, section, stepper};
use image::RgbaImage;
use std::rc::Rc;

const SIDE_W: f32 = 330.0;
const PREVIEW_H: f32 = 170.0;
const TOP: f32 = 56.0;
const SLOT_H: f32 = 28.0;

/// A place for a piece: in the column chosen, or the grass under tall grass.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Slot {
    TallTop,
    Tall,
    Tuft,
    Top,
    Earth,
    Under,
    Grass,
}

impl Slot {
    const ALL: [Slot; 7] = [Slot::TallTop, Slot::Tall, Slot::Tuft, Slot::Top, Slot::Earth, Slot::Under, Slot::Grass];

    fn label(self) -> &'static str {
        match self {
            Slot::TallTop => "Tall grass, its top",
            Slot::Tall => "Tall grass",
            Slot::Tuft => "Tuft (above the top)",
            Slot::Top => "Grass into earth: the top",
            Slot::Earth => "Earth (none: a plain colour)",
            Slot::Under => "Under, then earth again…",
            Slot::Grass => "Grass under tall grass (all)",
        }
    }

    /// What it is, in a sentence.
    fn noun(self) -> &'static str {
        match self {
            Slot::TallTop => "the top of the tall grass",
            Slot::Tall => "the tall grass",
            Slot::Tuft => "the tuft",
            Slot::Top => "the top (grass into earth)",
            Slot::Earth => "the earth",
            Slot::Under => "the piece under",
            Slot::Grass => "the grass under tall grass",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Slot::TallTop => "T2",
            Slot::Tall => "T1",
            Slot::Tuft => "Tu",
            Slot::Top => "Top",
            Slot::Earth => "E",
            Slot::Under => "U",
            Slot::Grass => "G",
        }
    }
}

fn slot_of(t: &Terrain, col: usize, s: Slot) -> Option<Piece> {
    let c = t.columns.get(col)?;
    match s {
        Slot::TallTop => c.tall[0],
        Slot::Tall => c.tall[1],
        Slot::Tuft => c.tuft,
        Slot::Top => c.top,
        Slot::Earth => c.earth,
        Slot::Under => c.under,
        Slot::Grass => t.grass,
    }
}

fn set_slot(t: &mut Terrain, col: usize, s: Slot, p: Option<Piece>) {
    if s == Slot::Grass {
        t.grass = p;
        return;
    }
    let Some(c) = t.columns.get_mut(col) else { return };
    match s {
        Slot::TallTop => c.tall[0] = p,
        Slot::Tall => c.tall[1] = p,
        Slot::Tuft => c.tuft = p,
        Slot::Top => c.top = p,
        Slot::Earth => c.earth = p,
        Slot::Under => c.under = p,
        Slot::Grass => {}
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cmd {
    Close,
    Save,
    Reset,
    Column(i32),
    Copy,
    Remove,
    Move(i32),
    Arm(Slot),
    Clear(Slot),
    TallEvery(i32),
    Piece(i32),
    Step(i32),
    OriginX(i32),
    OriginY(i32),
}

pub(super) enum Ask {
    Close,
    /// Kept: the game draws the ground with it again.
    Saved,
}

struct Layout {
    view: Rect,
    side: Rect,
    preview: Rect,
    top: Vec<(Rect, Cmd, &'static str)>,
    column: Rect,
    column_buttons: Vec<(Rect, Cmd, &'static str)>,
    pieces_y: f32,
    slots: Vec<(Rect, Rect, Slot)>,
    settings_y: f32,
    tall: Rect,
    grid: [(Rect, &'static str); 4],
}

pub(super) struct TerrainEditor {
    img: Rc<RgbaImage>,
    /// The sheet on screen (made when first drawn).
    texture: Option<Texture2D>,
    t: Terrain,
    saved: Terrain,
    /// The game's own terrain for this sheet, if it knows one.
    builtin: Option<Terrain>,
    col: usize,
    armed: Option<Slot>,
    /// The sheet's top-left on screen, and screen px a sheet px.
    origin: Vec2,
    zoom: f32,
    placed: bool,
    pan: Option<(Vec2, Vec2)>,
    /// The ground made with the terrain (the one it was made with): the
    /// ground, the ledge, and where each goes (sheet px).
    preview: Option<(Terrain, Texture2D, Texture2D, Rect, Rect)>,
    closing: f32,
    pub notes: Vec<String>,
}

impl TerrainEditor {
    /// On the zone's sheet as drawn (its background cleared).
    pub fn new(img: RgbaImage) -> TerrainEditor {
        let size = [img.width(), img.height()];
        let builtin = zone::ninja_terrain(&img);
        let t = zone::read_terrain(img.dimensions())
            .map(|t| t.fitted(&img))
            .or_else(|| builtin.clone())
            .unwrap_or_else(|| Terrain { size, ..Terrain::default() }.fitted(&img));
        TerrainEditor {
            img: Rc::new(img),
            texture: None,
            saved: t.clone(),
            t,
            builtin,
            col: 0,
            armed: Some(Slot::Top),
            origin: Vec2::ZERO,
            zoom: 2.0,
            placed: false,
            pan: None,
            preview: None,
            closing: 0.0,
            notes: vec![],
        }
    }

    fn dirty(&self) -> bool {
        self.t != self.saved
    }

    fn layout(&self) -> Layout {
        let (sw, sh) = (screen_width(), screen_height());
        let preview = Rect::new(12.0, sh - 12.0 - PREVIEW_H, sw - 24.0, PREVIEW_H);
        let body_h = preview.y - 8.0 - TOP;
        let side = Rect::new(sw - 12.0 - SIDE_W, TOP, SIDE_W, body_h);
        let view = Rect::new(12.0, TOP, side.x - 12.0 - 12.0, body_h);
        let mut top = vec![];
        let mut x = sw - 12.0;
        for (w, cmd, label) in
            [(76.0, Cmd::Close, "Close"), (76.0, Cmd::Save, "Save"), (110.0, Cmd::Reset, "Game's own")]
        {
            x -= w;
            top.push((Rect::new(x, 12.0, w, 32.0), cmd, label));
            x -= 6.0;
        }
        let (sx, w) = (side.x + 12.0, side.w - 24.0);
        let mut y = side.y + 12.0 + 22.0;
        let column = Rect::new(sx, y, w, 28.0);
        y += 34.0;
        let mut column_buttons = vec![];
        let bw = (w - 3.0 * 6.0) / 4.0;
        for (k, (cmd, label)) in
            [(Cmd::Copy, "Copy"), (Cmd::Remove, "Remove"), (Cmd::Move(-1), "◀ Move"), (Cmd::Move(1), "Move ▶")]
                .into_iter()
                .enumerate()
        {
            column_buttons.push((Rect::new(sx + k as f32 * (bw + 6.0), y, bw, 26.0), cmd, label));
        }
        y += 26.0 + 10.0;
        let pieces_y = y;
        y += 22.0;
        let mut slots = vec![];
        for s in Slot::ALL {
            if s == Slot::Grass {
                y += 6.0;
            }
            let row = Rect::new(sx, y, w, SLOT_H - 2.0);
            slots.push((row, Rect::new(row.right() - 26.0, y + 1.0, 26.0, SLOT_H - 4.0), s));
            y += SLOT_H;
        }
        y += 8.0;
        let settings_y = y;
        y += 22.0;
        let tall = Rect::new(sx, y, w, 28.0);
        y += 34.0;
        let half = (w - 6.0) / 2.0;
        let grid = [
            (Rect::new(sx, y, half, 28.0), "Piece"),
            (Rect::new(sx + half + 6.0, y, half, 28.0), "Step"),
            (Rect::new(sx, y + 34.0, half, 28.0), "Grid x"),
            (Rect::new(sx + half + 6.0, y + 34.0, half, 28.0), "Grid y"),
        ];
        Layout { view, side, preview, top, column, column_buttons, pieces_y, slots, settings_y, tall, grid }
    }

    /// Fit the terrain's pieces (or the whole sheet) in the view.
    fn place(&mut self, view: Rect) {
        let mut pieces: Vec<Piece> = self
            .t
            .columns
            .iter()
            .flat_map(|c| [c.tall[0], c.tall[1], c.tuft, c.top, c.earth, c.under])
            .flatten()
            .collect();
        pieces.extend(self.t.grass);
        let n = self.t.grid.piece as f32;
        let area = if pieces.is_empty() {
            Rect::new(0.0, 0.0, self.img.width() as f32, self.img.height() as f32)
        } else {
            let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for (x, y) in pieces {
                (x0, y0) = (x0.min(x as f32), y0.min(y as f32));
                (x1, y1) = (x1.max(x as f32 + n), y1.max(y as f32 + n));
            }
            Rect::new(x0 - 8.0, y0 - 8.0, x1 - x0 + 16.0, y1 - y0 + 16.0)
        };
        self.zoom = (view.w / area.w).min(view.h / area.h).floor().clamp(1.0, 8.0);
        self.origin = vec2(view.x + (view.w - area.w * self.zoom) / 2.0, view.y + (view.h - area.h * self.zoom) / 2.0)
            - vec2(area.x, area.y) * self.zoom;
        self.origin = self.origin.round();
    }

    fn to_sheet(&self, m: Vec2) -> Vec2 {
        (m - self.origin) / self.zoom
    }

    pub fn update(&mut self, dt: f32, input: &mut Input) -> Option<Ask> {
        self.closing = (self.closing - dt).max(0.0);
        let l = self.layout();
        if !self.placed {
            self.place(l.view);
            self.placed = true;
        }
        let m = input.mouse;
        // The view: the wheel zooms about the pointer, a right or middle
        // drag moves the sheet.
        if l.view.contains(m) && input.wheel != 0.0 {
            let at = self.to_sheet(m);
            self.zoom = (self.zoom * if input.wheel > 0.0 { 2.0 } else { 0.5 }).clamp(1.0, 16.0);
            self.origin = (m - at * self.zoom).round();
        }
        let dragging = input.right_down || input.middle_down;
        match (&self.pan, dragging) {
            (None, true) if l.view.contains(m) => self.pan = Some((m, self.origin)),
            (Some((from, o)), true) => self.origin = (*o + (m - *from)).round(),
            (Some(_), false) => self.pan = None,
            _ => {}
        }
        // Keys.
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        let mut cmd = None;
        if is_key_pressed(KeyCode::Escape) {
            cmd = Some(Cmd::Close);
        } else if ctrl && is_key_pressed(KeyCode::S) {
            cmd = Some(Cmd::Save);
        } else if is_key_pressed(KeyCode::Left) {
            cmd = Some(Cmd::Column(-1));
        } else if is_key_pressed(KeyCode::Right) {
            cmd = Some(Cmd::Column(1));
        }
        // Clicks.
        for (r, c, _) in &l.top {
            if button(*r, input) {
                cmd = Some(*c);
            }
        }
        match stepper(l.column, input) {
            0 => {}
            d => cmd = Some(Cmd::Column(d)),
        }
        for (r, c, _) in &l.column_buttons {
            if button(*r, input) {
                cmd = Some(*c);
            }
        }
        for (row, clear, s) in &l.slots {
            if button(*clear, input) {
                cmd = Some(Cmd::Clear(*s));
            } else if button(*row, input) {
                cmd = Some(Cmd::Arm(*s));
            }
        }
        match stepper(l.tall, input) {
            0 => {}
            d => cmd = Some(Cmd::TallEvery(d)),
        }
        for (k, (r, _)) in l.grid.iter().enumerate() {
            match stepper(*r, input) {
                0 => {}
                d => {
                    cmd = Some(match k {
                        0 => Cmd::Piece(d),
                        1 => Cmd::Step(d),
                        2 => Cmd::OriginX(d),
                        _ => Cmd::OriginY(d),
                    })
                }
            }
        }
        // A piece clicked on the sheet: into the place chosen, else the
        // column it is in is chosen.
        if input.left_pressed && !input.consumed && l.view.contains(m) {
            let p = self.to_sheet(m);
            if let Some(piece) = self.t.grid.piece_at(p.x, p.y).filter(|&(x, y)| {
                x + self.t.grid.piece <= self.img.width() && y + self.t.grid.piece <= self.img.height()
            }) {
                match self.armed {
                    Some(s) => set_slot(&mut self.t, self.col, s, Some(piece)),
                    None => {
                        let found = (0..self.t.columns.len())
                            .find(|&i| Slot::ALL[..6].iter().any(|&s| slot_of(&self.t, i, s) == Some(piece)));
                        if let Some(i) = found {
                            self.col = i;
                        }
                    }
                }
                self.t = self.t.clone().fitted(&self.img);
            }
            input.consumed = true;
        }
        // The preview: a column clicked is chosen.
        if input.left_pressed && !input.consumed && l.preview.contains(m) {
            if let Some((_, _, _, r, _)) = &self.preview {
                let u = ((m.x - (l.preview.x + 10.0)) / 2.0 + r.x).floor() as i64;
                let n = self.t.columns.len().max(1) as i64;
                self.col = u.div_euclid(self.t.grid.piece.max(1) as i64).rem_euclid(n) as usize;
            }
            input.consumed = true;
        }
        input.consumed = true;
        cmd.and_then(|c| self.run(c))
    }

    fn run(&mut self, cmd: Cmd) -> Option<Ask> {
        let n = self.t.columns.len();
        let step = |v: u32, d: i32, lo: u32, hi: u32| (v as i32 + d).clamp(lo as i32, hi as i32) as u32;
        match cmd {
            Cmd::Close if self.dirty() && self.closing <= 0.0 => {
                self.closing = 3.0;
                self.notes.push("Not saved: again to leave without it (or Save)".into());
            }
            Cmd::Close => return Some(Ask::Close),
            Cmd::Save => {
                let size = [self.img.width(), self.img.height()];
                let t = Terrain { size, ..self.t.clone() };
                let keep = if t.usable() { Some(&t) } else { None };
                match zone::save_terrain(keep) {
                    Ok(()) => {
                        self.saved = self.t.clone();
                        self.notes.push(if keep.is_some() {
                            "Saved zone.terrain.json: the ground is built with it".into()
                        } else {
                            "No column has a top: the ground is drawn from the zone's tiles".into()
                        });
                        return Some(Ask::Saved);
                    }
                    Err(e) => self.notes.push(format!("Not saved: {e}")),
                }
            }
            Cmd::Reset => {
                let size = [self.img.width(), self.img.height()];
                self.t =
                    self.builtin.clone().unwrap_or_else(|| Terrain { size, ..Terrain::default() }.fitted(&self.img));
                self.col = 0;
                self.notes.push(match self.builtin {
                    Some(_) => "Back to the game's own terrain for this sheet (Save to keep it)".into(),
                    None => "Started again: one empty column (Save to keep it)".into(),
                });
            }
            Cmd::Column(d) => self.col = (self.col as i32 + d).rem_euclid(n.max(1) as i32) as usize,
            Cmd::Copy => {
                let c = self.t.columns.get(self.col).cloned().unwrap_or_default();
                self.t.columns.insert((self.col + 1).min(n), c);
                self.col = (self.col + 1).min(self.t.columns.len() - 1);
            }
            Cmd::Remove if n > 1 => {
                self.t.columns.remove(self.col);
                self.col = self.col.min(self.t.columns.len() - 1);
            }
            Cmd::Remove => self.t.columns = vec![Column::default()],
            Cmd::Move(d) => {
                let to = self.col as i32 + d;
                if to >= 0 && (to as usize) < n {
                    self.t.columns.swap(self.col, to as usize);
                    self.col = to as usize;
                }
            }
            Cmd::Arm(s) => self.armed = if self.armed == Some(s) { None } else { Some(s) },
            Cmd::Clear(s) => set_slot(&mut self.t, self.col, s, None),
            Cmd::TallEvery(d) => self.t.tall_every = step(self.t.tall_every, d, 0, 8),
            Cmd::Piece(d) => {
                let piece = if d > 0 { (self.t.grid.piece * 2).min(32) } else { (self.t.grid.piece / 2).max(4) };
                let gap = self.t.grid.step.saturating_sub(self.t.grid.piece);
                self.t.grid = Grid { piece, step: piece + gap, ..self.t.grid };
            }
            Cmd::Step(d) => self.t.grid.step = step(self.t.grid.step, d, self.t.grid.piece, self.t.grid.piece + 8),
            Cmd::OriginX(d) => self.t.grid.origin.0 = step(self.t.grid.origin.0, d, 0, self.t.grid.step - 1),
            Cmd::OriginY(d) => self.t.grid.origin.1 = step(self.t.grid.origin.1, d, 0, self.t.grid.step - 1),
        }
        if !matches!(cmd, Cmd::Close | Cmd::Save) {
            self.t = self.t.clone().fitted(&self.img);
        }
        None
    }

    /// The ground the terrain makes: hills, a slope, a pit and a ledge (in
    /// sheet px), made again when the terrain changes.
    fn preview(&mut self, width: f32) {
        if self.preview.as_ref().is_some_and(|p| p.0 == self.t) {
            return;
        }
        let w = width.max(120.0);
        let ground = |x0: f32, x1: f32, pts: &[(f32, f32)]| {
            let mut poly: Vec<Vec2> = pts.iter().map(|&(x, y)| vec2(x, y)).collect();
            poly.push(vec2(x1, 200.0));
            poly.push(vec2(x0, 200.0));
            poly
        };
        let pit = w * 0.62;
        let a = ground(
            0.0,
            pit,
            &[(0.0, 46.0), (w * 0.12, 46.0), (w * 0.25, 30.0), (w * 0.36, 30.0), (w * 0.5, 54.0), (pit, 54.0)],
        );
        let b = ground(pit + 34.0, w, &[(pit + 34.0, 40.0), (w, 40.0)]);
        let (ia, ra) = match bake_terrain(&a, 1.0, &self.t, &self.img, &[]) {
            Some(x) => x,
            None => (RgbaImage::new(1, 1), Rect::new(0.0, 0.0, 1.0, 1.0)),
        };
        let (ib, rb) =
            bake_terrain(&b, 1.0, &self.t, &self.img, &[]).unwrap_or((RgbaImage::new(1, 1), Rect::default()));
        // The ledge goes in the second picture's place, drawn with it.
        let ledge = [vec2(pit - 20.0, 12.0), vec2(pit + 60.0, 12.0)];
        let (il, rl) =
            bake_terrain_ledge(&ledge, 6.0, 1.0, &self.t, &self.img).unwrap_or((RgbaImage::new(1, 1), Rect::default()));
        let up = crate::sonic::sheet::upload;
        // Two textures kept: the ground (both pieces side by side), the ledge.
        let mut both = RgbaImage::new(w as u32, 200);
        for (img, r) in [(&ia, ra), (&ib, rb)] {
            for (x, y, p) in img.enumerate_pixels() {
                let (bx, by) = (r.x as i64 + x as i64, r.y as i64 + y as i64);
                if p[3] > 0 && bx >= 0 && by >= 0 && (bx as u32) < both.width() && (by as u32) < both.height() {
                    both.put_pixel(bx as u32, by as u32, *p);
                }
            }
        }
        self.preview = Some((self.t.clone(), up(&both), up(&il), Rect::new(0.0, 0.0, w, 200.0), rl));
    }

    pub fn draw(&mut self, mouse: Vec2) {
        if self.texture.is_none() {
            self.texture = Some(crate::sonic::sheet::upload(&self.img));
        }
        clear_background(Color::new(0.05, 0.05, 0.09, 1.0));
        let l = self.layout();
        text_bold("Terrain", 16.0, baseline(28.0, 18.0), 18.0, TEXT);
        let hint = match self.armed {
            Some(Slot::Grass) => format!("Click a piece on the sheet: {}, for every column", Slot::Grass.noun()),
            Some(s) => format!("Click a piece on the sheet: {} of column {}", s.noun(), self.col + 1),
            None => "Click a piece to find its column  ·  choose a place on the right to change it".to_string(),
        };
        text(&hint, 110.0, baseline(28.0, 13.0), 13.0, TEXT_DIM);
        for (r, c, label) in &l.top {
            let lit = *c == Cmd::Save && self.dirty();
            draw_button(*r, label, r.contains(mouse), lit, 1.0);
        }
        self.draw_view(&l, mouse);
        self.draw_side(&l, mouse);
        self.draw_preview(&l);
    }

    fn draw_view(&self, l: &Layout, mouse: Vec2) {
        let v = l.view;
        rrect(v, RADIUS, Color::new(0.08, 0.08, 0.13, 1.0));
        clip(Some(v));
        // A checkerboard behind what is clear.
        let cell = 16.0;
        let (sw, sh) = (self.img.width() as f32 * self.zoom, self.img.height() as f32 * self.zoom);
        let (x0, y0) = (self.origin.x.max(v.x), self.origin.y.max(v.y));
        let (x1, y1) = ((self.origin.x + sw).min(v.right()), (self.origin.y + sh).min(v.bottom()));
        let mut y = y0;
        while y < y1 {
            let mut x = x0;
            while x < x1 {
                let odd = (((x - self.origin.x) / cell) as i32 + ((y - self.origin.y) / cell) as i32) % 2 == 0;
                let c = if odd { Color::new(0.16, 0.16, 0.2, 1.0) } else { Color::new(0.2, 0.2, 0.25, 1.0) };
                draw_rectangle(x, y, cell.min(x1 - x), cell.min(y1 - y), c);
                x += cell;
            }
            y += cell;
        }
        let params = DrawTextureParams { dest_size: Some(vec2(sw, sh)), ..Default::default() };
        if let Some(t) = &self.texture {
            draw_texture_ex(t, self.origin.x, self.origin.y, WHITE, params);
        }
        let n = self.t.grid.piece as f32 * self.zoom;
        let box_at =
            |p: Piece| Rect::new(self.origin.x + p.0 as f32 * self.zoom, self.origin.y + p.1 as f32 * self.zoom, n, n);
        // Every column's top, numbered in order.
        for (i, c) in self.t.columns.iter().enumerate() {
            if let Some(p) = c.top {
                let r = box_at(p);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, alpha(WHITE, 0.35));
                if self.zoom >= 2.0 && i != self.col {
                    text(&(i + 1).to_string(), r.x + 1.0, r.y - 2.0, 10.0, alpha(WHITE, 0.6));
                }
            }
        }
        // The column chosen, its pieces named.
        for s in Slot::ALL {
            if let Some(p) = slot_of(&self.t, self.col, s) {
                let r = box_at(p);
                let lit = self.armed == Some(s);
                draw_rectangle_lines(
                    r.x - 1.0,
                    r.y - 1.0,
                    r.w + 2.0,
                    r.h + 2.0,
                    2.0,
                    if lit { WARNING } else { ACCENT_HI },
                );
                if self.zoom >= 2.0 {
                    text_bold(s.short(), r.x + 2.0, r.y + 11.0, 10.0, if lit { WARNING } else { ACCENT_HI });
                }
            }
        }
        // The piece under the pointer.
        if v.contains(mouse) {
            let p = self.to_sheet(mouse);
            if let Some(piece) = self.t.grid.piece_at(p.x, p.y) {
                let r = box_at(piece);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, if self.armed.is_some() { WARNING } else { TEXT });
            }
        }
        clip(None);
        rrect_lines(v, RADIUS, 1.0, BORDER);
        let tip = format!("×{}  ·  wheel: zoom  ·  right-drag: move", self.zoom);
        text(&tip, v.x + 10.0, v.bottom() - 10.0, 11.0, TEXT_MUTED);
    }

    fn draw_side(&self, l: &Layout, m: Vec2) {
        let p = l.side;
        panel(p, 1.0);
        let (sx, w) = (p.x + 12.0, p.w - 24.0);
        let n = self.t.columns.len();
        section(sx, p.y + 12.0, w, "COLUMNS (SIDE BY SIDE, IN ORDER)", 1.0);
        draw_stepper(l.column, "Column", &format!("{} / {n}", self.col + 1), ACCENT_HI, m, 1.0);
        for (r, c, label) in &l.column_buttons {
            let ok = match c {
                Cmd::Move(d) => {
                    let to = self.col as i32 + d;
                    to >= 0 && (to as usize) < n
                }
                _ => true,
            };
            draw_button(*r, label, ok && r.contains(m), false, if ok { 1.0 } else { 0.4 });
        }
        section(sx, l.pieces_y, w, "ITS PIECES, FROM THE TOP DOWN", 1.0);
        for (row, clear, s) in &l.slots {
            let armed = self.armed == Some(*s);
            let hov = row.contains(m);
            rrect(
                *row,
                6.0,
                if armed {
                    mix(SURFACE_2, WARNING, 0.25)
                } else if hov {
                    SURFACE_HI
                } else {
                    SURFACE_2
                },
            );
            rrect_lines(*row, 6.0, 1.0, if armed { WARNING } else { BORDER });
            let mini = Rect::new(row.x + 4.0, row.y + 2.0, row.h - 4.0, row.h - 4.0);
            rrect(mini, 3.0, Color::new(0.12, 0.12, 0.18, 1.0));
            match slot_of(&self.t, self.col, *s) {
                Some(piece) => {
                    let n = self.t.grid.piece as f32;
                    let params = DrawTextureParams {
                        source: Some(Rect::new(piece.0 as f32, piece.1 as f32, n, n)),
                        dest_size: Some(mini.size()),
                        ..Default::default()
                    };
                    if let Some(t) = &self.texture {
                        draw_texture_ex(t, mini.x, mini.y, WHITE, params);
                    }
                    draw_button(*clear, "×", clear.contains(m), false, 1.0);
                }
                None => {
                    let none = if *s == Slot::Earth { "plain" } else { "none" };
                    text_centered(none, mini.x + mini.w / 2.0, mini.y + mini.h / 2.0, 8.0, TEXT_MUTED);
                }
            }
            text(
                s.label(),
                mini.right() + 8.0,
                baseline(row.y + row.h / 2.0, 12.0),
                12.0,
                if armed { TEXT } else { TEXT_DIM },
            );
        }
        section(sx, l.settings_y, w, "TALL GRASS AND THE SHEET'S GRID", 1.0);
        let tall = match self.t.tall_every {
            0 => "never".to_string(),
            1 => "every stretch".to_string(),
            k => format!("1 stretch in {k}"),
        };
        draw_stepper(l.tall, "Tall grass", &tall, ACCENT_HI, m, 1.0);
        let g = self.t.grid;
        let values =
            [format!("{} px", g.piece), format!("{} px", g.step), g.origin.0.to_string(), g.origin.1.to_string()];
        for ((r, label), value) in l.grid.iter().zip(values) {
            draw_stepper(*r, label, &value, ACCENT_HI, m, 1.0);
        }
    }

    fn draw_preview(&mut self, l: &Layout) {
        let p = l.preview;
        let inner_w = (p.w - 20.0) / 2.0;
        self.preview(inner_w);
        rrect(p, RADIUS, Color::new(0.36, 0.62, 0.95, 1.0));
        clip(Some(p));
        if let Some((_, ground, ledge, r, rl)) = &self.preview {
            let z = 2.0;
            let at = vec2(p.x + 10.0, p.y - 4.0);
            let params = DrawTextureParams { dest_size: Some(r.size() * z), ..Default::default() };
            draw_texture_ex(ground, at.x, at.y, WHITE, params);
            let params = DrawTextureParams { dest_size: Some(rl.size() * z), ..Default::default() };
            draw_texture_ex(ledge, at.x + rl.x * z, at.y + rl.y * z, WHITE, params);
            // The column chosen, wherever it is.
            let piece = self.t.grid.piece.max(1) as f32;
            let n = self.t.columns.len().max(1) as f32;
            let mut u = self.col as f32 * piece;
            while u < r.w {
                let x = at.x + u * z;
                draw_rectangle_lines(x, p.y + 2.0, piece * z, p.h - 4.0, 1.5, alpha(WARNING, 0.8));
                u += piece * n;
            }
        }
        clip(None);
        rrect_lines(p, RADIUS, 1.0, BORDER);
        text(
            "The ground it makes (click a column to choose it)",
            p.x + 10.0,
            p.bottom() - 10.0,
            11.0,
            alpha(WHITE, 0.9),
        );
    }
}

impl App {
    /// Open the terrain editor on the zone's sheet as edited.
    pub(super) fn open_terrain(&mut self) {
        let Some(ed) = self.regions.as_ref() else { return };
        self.terrain_ed = Some(TerrainEditor::new(ed.sheet.drawn(false)));
    }

    /// A frame of the terrain editor, over the whole screen.
    pub(super) fn terrain_frame(&mut self, dt: f32) {
        set_default_camera();
        let mut input = Input::gather();
        let Some(ed) = self.terrain_ed.as_mut() else { return };
        let ask = ed.update(dt, &mut input);
        for n in ed.notes.drain(..) {
            self.toasts.info(n);
        }
        if let Some(ed) = self.terrain_ed.as_mut() {
            ed.draw(input.mouse);
        }
        match ask {
            Some(Ask::Close) => self.terrain_ed = None,
            Some(Ask::Saved) => {
                // The game builds its ground with it from now on.
                self.sonic_sheet_tried = true;
                self.load_sonic_sheet();
            }
            None => {}
        }
        self.toasts.update(dt);
        self.toasts.draw(56.0, &self.jobs, dt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sheet the size of NinjaCakes' tileset, its terrain's pieces there.
    fn sheet() -> RgbaImage {
        let mut img = RgbaImage::new(zone::NINJA_SIZE.0, zone::NINJA_SIZE.1);
        for i in 0..26 {
            for (y, c) in [(67, [0, 200, 0, 255]), (85, [240, 120, 0, 255])] {
                for py in y..y + 8 {
                    for px in 10 + 9 * i..18 + 9 * i {
                        img.put_pixel(px, py, image::Rgba(c));
                    }
                }
            }
        }
        img
    }

    #[test]
    fn columns_are_changed_copied_moved_and_put_back() {
        let mut ed = TerrainEditor::new(sheet());
        let own = ed.builtin.clone().expect("the game knows this one");
        assert_eq!(ed.t.columns.len(), 26);
        // A piece into the earth of the first column.
        ed.run(Cmd::Arm(Slot::Earth));
        set_slot(&mut ed.t, ed.col, Slot::Earth, Some((37, 85)));
        assert_eq!(ed.t.columns[0].earth, Some((37, 85)));
        assert!(ed.dirty());
        // Copied after itself (and chosen), moved back, removed.
        ed.run(Cmd::Copy);
        assert_eq!((ed.t.columns.len(), ed.col, ed.t.columns[1].earth), (27, 1, Some((37, 85))));
        ed.run(Cmd::Move(1));
        assert_eq!((ed.col, ed.t.columns[2].earth), (2, Some((37, 85))));
        ed.run(Cmd::Remove);
        assert_eq!(ed.t.columns.len(), 26);
        ed.run(Cmd::Clear(Slot::Earth));
        assert_eq!(ed.t.columns[0].earth, Some((37, 85)), "the column chosen only");
        // Tall grass and the grid.
        ed.run(Cmd::TallEvery(-5));
        assert_eq!(ed.t.tall_every, 0, "never");
        ed.run(Cmd::Piece(1));
        assert_eq!((ed.t.grid.piece, ed.t.grid.step), (16, 17), "the gap kept");
        ed.run(Cmd::Piece(-1));
        ed.run(Cmd::OriginX(-3));
        assert_eq!(ed.t.grid.origin.0, 0);
        // The game's own again.
        ed.run(Cmd::Reset);
        assert_eq!(ed.t, own);
        // Leaving with changes asks once.
        ed.run(Cmd::Column(1));
        set_slot(&mut ed.t, ed.col, Slot::Tuft, Some((10, 67)));
        assert!(ed.run(Cmd::Close).is_none());
        assert!(matches!(ed.run(Cmd::Close), Some(Ask::Close)));
    }
}
