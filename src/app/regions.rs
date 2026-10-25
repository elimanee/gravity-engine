//! The sprite region editor: on a character's sheet, which pictures each of
//! its animations uses and where the character's centre is in each. It
//! takes the whole screen; what is saved goes beside the sheet as
//! `<character>.regions.json` (see [`crate::sonic::regions`]), and the game
//! draws from it at once.

use super::*;
use crate::sonic::regions::{self, Regions};
use crate::sonic::sheet::{self, find_sprite, Anim, Editable, Pic};
use crate::sonic::{body_radii, Character};
use crate::ui::theme::*;
use crate::ui::widgets::clip;

/// The animations every sheet can have, in the order listed, and when the
/// game shows each.
const COMMON: &[(&str, &str)] = &[
    ("idle", "Standing still"),
    ("bored", "Waiting a while (after 3 s)"),
    ("look_up", "Holding Up"),
    ("crouch", "Holding Down"),
    ("walk", "Walking, below running speed"),
    ("run", "Running fast"),
    ("skid", "Braking hard"),
    ("push", "Pushing against a wall"),
    ("roll", "Curled up: jumping and rolling"),
    ("spindash", "Charging a spin dash (Down + Jump)"),
    ("spring", "Rising, off a spring"),
    ("hurt", "Knocked back"),
    ("dead", "Losing a life"),
];

/// Each character's own animations.
fn extras(c: Character) -> &'static [(&'static str, &'static str)] {
    match c {
        Character::Sonic => &[],
        Character::Tails => &[("fly", "Flying (Space in the air)"), ("fly_tired", "Flying, tired out")],
        Character::Knuckles => &[
            ("glide", "Gliding (Space in the air)"),
            ("slide", "Sliding along the ground after a glide"),
            ("drop", "Dropping out of a glide"),
            ("climb", "Climbing a wall"),
            ("climb_up", "Climbing over the top of a wall"),
        ],
        Character::Amy => &[("hammer", "Hammer swing on the ground"), ("hammer_air", "Hammer swing in the air")],
        Character::Exe => &[
            ("warp", "Teleporting (Space in the air)"),
            ("chase", "Hunting you (Halloween act)"),
            ("laugh", "Laughing after catching you"),
        ],
    }
}

/// Animations drawn with the character's centre in the middle of the
/// picture (curled up, or in the air) rather than above the feet.
const MIDDLE: &[&str] = &[
    "roll",
    "spindash",
    "spring",
    "hurt",
    "dead",
    "glide",
    "drop",
    "climb",
    "climb_up",
    "fly",
    "fly_tired",
    "warp",
    "hammer_air",
];

/// Animations drawn curled up (the smaller body).
const CURLED: &[&str] = &["roll", "spindash"];

/// Zoom steps of the sheet view (screen px per sheet px).
const ZOOMS: &[f32] = &[0.25, 0.5, 0.75, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0, 20.0, 24.0, 32.0];

/// Pictures per second in the preview (the pieces behind: their own).
const PREVIEW_FPS: f32 = 10.0;
const BEHIND_FPS: f32 = 8.0;

/// Thumbnails of the animation's pictures.
const THUMB: f32 = 56.0;
const THUMB_GAP: f32 = 6.0;

/// What the editor asks of the app.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Ask {
    Close,
    Save,
    /// Edit this character's sheet instead.
    Open(Character),
}

/// Something to tell the player.
pub(super) enum Note {
    Info(String),
    Warn(String),
}

/// The pointer held down on the sheet.
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    /// A new picture, from this sheet point.
    New(Vec2),
    /// Moving the picture, its centre with it, from this sheet point.
    Move(Vec2, Pic),
    /// Its edges (-1: left / top, 1: right / bottom).
    Resize(Vec2, Pic, (i8, i8)),
    /// Its centre.
    Centre(Vec2, Pic),
    /// The view, from this screen point and sheet origin.
    Pan(Vec2, Vec2),
}

/// What of the selected picture is under the pointer.
#[derive(Clone, Copy, PartialEq)]
enum Handle {
    Centre,
    Edge(i8, i8),
    Inside,
}

#[derive(Clone, Copy, PartialEq)]
enum Cmd {
    Character(Character),
    Save,
    Close,
    Undo,
    Redo,
    ResetAll,
    ResetAnim,
    Previous,
    Play,
    Next,
    Onion,
    Body,
    Behind,
    FitFeet,
    FitMiddle,
    Earlier,
    Later,
    Duplicate,
    Delete,
}

struct Button {
    r: Rect,
    label: String,
    cmd: Cmd,
    on: bool,
    enabled: bool,
}

/// Where everything is this frame.
struct Layout {
    list: Rect,
    view: Rect,
    side: Rect,
    preview: Rect,
    thumbs: Vec<Rect>,
    /// Top of the selected picture's numbers.
    info: f32,
    buttons: Vec<Button>,
}

/// Nudging the selected picture with the arrow keys.
#[derive(Clone, Copy)]
enum Nudge {
    Move,
    Size,
    Centre,
}

pub(super) struct RegionEditor {
    character: Character,
    /// The characters with a sheet.
    available: Vec<Character>,
    sheet: Editable,
    /// The animations listed, and when the game shows each.
    names: Vec<(String, &'static str)>,
    anim: usize,
    /// Editing the pieces drawn behind (Tails' tails).
    behind: bool,
    selected: Option<usize>,
    /// The sheet's top-left on screen, and screen px per sheet px.
    origin: Vec2,
    zoom: f32,
    /// The view is placed once the screen's size is known.
    placed: bool,
    drag: Option<Drag>,
    /// The animations before the drag (to undo it).
    before: Option<HashMap<String, Anim>>,
    undo: Vec<HashMap<String, Anim>>,
    redo: Vec<HashMap<String, Anim>>,
    /// What was last saved (or found), to tell unsaved changes.
    saved: HashMap<String, Anim>,
    playing: bool,
    onion: bool,
    clock: f32,
    /// Asked once with unsaved changes: asking again leaves them.
    leaving: Option<Ask>,
    last_click: (f64, Vec2),
    /// First animation shown in the list.
    scroll: usize,
    /// Arrow keys held, and for how long (they repeat).
    held: HashMap<KeyCode, f32>,
    pub notes: Vec<Note>,
}

/// `v` kept between `lo` and `hi` (`lo` wins when they cross).
fn within(v: f32, lo: f32, hi: f32) -> f32 {
    v.min(hi).max(lo)
}

/// Whether two sets of animations draw differently (an empty animation
/// is the same as none).
fn differ(a: &HashMap<String, Anim>, b: &HashMap<String, Anim>) -> bool {
    let none = Anim::default();
    a.keys().chain(b.keys()).any(|n| a.get(n).unwrap_or(&none) != b.get(n).unwrap_or(&none))
}

/// A sheet coordinate as shown: whole, or to a tenth.
fn num(v: f32) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}

fn rect_between(a: Vec2, b: Vec2, size: Vec2) -> Rect {
    let lo = a.min(b).floor().max(Vec2::ZERO);
    let hi = a.max(b).ceil().min(size);
    Rect::new(lo.x, lo.y, (hi.x - lo.x).max(0.0), (hi.y - lo.y).max(0.0))
}

fn dashed_line(a: Vec2, b: Vec2, thickness: f32, color: Color) {
    let len = a.distance(b);
    if len < 1.0 {
        return;
    }
    let dir = (b - a) / len;
    let mut t = 0.0;
    while t < len {
        let e = (t + 5.0).min(len);
        let (p, q) = (a + dir * t, a + dir * e);
        draw_line(p.x, p.y, q.x, q.y, thickness, color);
        t += 9.0;
    }
}

fn dashed_rect(r: Rect, color: Color) {
    let (a, b, c, d) = (r.point(), vec2(r.right(), r.y), vec2(r.right(), r.bottom()), vec2(r.x, r.bottom()));
    for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
        dashed_line(p, q, 1.5, color);
    }
}

/// Picture `pic` with the character's centre at `at` (screen), `zoom`
/// screen px per sheet px.
fn draw_pic(texture: &Texture2D, pic: Pic, at: Vec2, zoom: f32, tint: Color) {
    let top_left = (at - pic.centre * zoom).round();
    draw_texture_ex(
        texture,
        top_left.x,
        top_left.y,
        tint,
        DrawTextureParams {
            source: Some(pic.src),
            dest_size: Some(vec2(pic.src.w, pic.src.h) * zoom),
            ..Default::default()
        },
    );
}

impl RegionEditor {
    pub fn new(character: Character, available: Vec<Character>, mut sheet: Editable) -> Self {
        let mut names: Vec<(String, &'static str)> =
            COMMON.iter().chain(extras(character)).map(|&(n, what)| (n.to_string(), what)).collect();
        let mut more: Vec<String> =
            sheet.anims.keys().filter(|k| !names.iter().any(|(n, _)| n == *k)).cloned().collect();
        more.sort();
        names.extend(more.into_iter().map(|k| (k, "Not drawn by the game")));
        let mut notes = vec![];
        if let Some(w) = sheet.warning.take() {
            notes.push(Note::Warn(format!("{w}: starting again from the sheet's own layout")));
        }
        let saved = sheet.anims.clone();
        let mut ed = RegionEditor {
            character,
            available,
            sheet,
            names,
            anim: 0,
            behind: false,
            selected: None,
            origin: Vec2::ZERO,
            zoom: 1.0,
            placed: false,
            drag: None,
            before: None,
            undo: vec![],
            redo: vec![],
            saved,
            playing: true,
            onion: false,
            clock: 0.0,
            leaving: None,
            last_click: (0.0, Vec2::ZERO),
            scroll: 0,
            held: HashMap::new(),
            notes,
        };
        ed.select_anim(0);
        ed
    }

    fn name(&self) -> &str {
        &self.names[self.anim].0
    }

    fn size(&self) -> Vec2 {
        let (w, h) = self.sheet.image.dimensions();
        vec2(w as f32, h as f32)
    }

    fn pics(&self) -> &[Pic] {
        self.sheet.anims.get(self.name()).map_or(&[], |a| if self.behind { &a.behind } else { &a.pics })
    }

    fn pics_mut(&mut self) -> &mut Vec<Pic> {
        let name = self.names[self.anim].0.clone();
        let a = self.sheet.anims.entry(name).or_default();
        if self.behind {
            &mut a.behind
        } else {
            &mut a.pics
        }
    }

    fn dirty(&self) -> bool {
        differ(&self.sheet.anims, &self.saved)
    }

    /// The pieces drawn behind are shown (Tails, or a sheet that has some).
    fn layered(&self) -> bool {
        self.character == Character::Tails || self.sheet.anims.get(self.name()).is_some_and(|a| !a.behind.is_empty())
    }

    /// How far below the centre the feet are (sheet px), for a standing
    /// animation; none for one centred in the middle.
    fn feet(&self) -> Option<f32> {
        (!MIDDLE.contains(&self.name())).then(|| body_radii(self.character, false).1 * self.sheet.scale)
    }

    /// The character's half width and half height in this animation (sheet px).
    fn radii(&self) -> Vec2 {
        let (w, h) = body_radii(self.character, CURLED.contains(&self.name()));
        vec2(w, h) * self.sheet.scale
    }

    fn select_anim(&mut self, i: usize) {
        self.anim = i.min(self.names.len() - 1);
        self.behind = self.behind && self.layered();
        self.selected = (!self.pics().is_empty()).then_some(0);
        self.clock = 0.0;
        if self.placed {
            self.frame_view(layout_parts().1);
        }
    }

    /// Fit the view around the animation's pictures (the whole sheet when
    /// it has none).
    fn frame_view(&mut self, view: Rect) {
        let rects: Vec<Rect> = self
            .sheet
            .anims
            .get(self.name())
            .map(|a| a.pics.iter().chain(&a.behind).map(|p| p.src).collect())
            .unwrap_or_default();
        let size = self.size();
        let (area, most) = match rects.split_first() {
            Some((first, rest)) => (rest.iter().fold(*first, |acc, r| acc.combine_with(*r)), 6.0),
            None => (Rect::new(0.0, 0.0, size.x, size.y), 4.0),
        };
        let fit = ((view.w - 80.0) / area.w.max(1.0)).min((view.h - 100.0) / area.h.max(1.0)).min(most);
        self.zoom = ZOOMS.iter().rev().copied().find(|&z| z <= fit).unwrap_or(ZOOMS[0]);
        let middle = vec2(view.x + view.w / 2.0, view.y + 14.0 + view.h / 2.0);
        self.origin = (middle - (area.point() + area.size() / 2.0) * self.zoom).round();
    }

    fn to_sheet(&self, m: Vec2) -> Vec2 {
        (m - self.origin) / self.zoom
    }

    fn to_screen(&self, p: Vec2) -> Vec2 {
        self.origin + p * self.zoom
    }

    fn screen_rect(&self, r: Rect) -> Rect {
        let a = self.to_screen(r.point());
        Rect::new(a.x, a.y, r.w * self.zoom, r.h * self.zoom)
    }

    // ── Changes ─────────────────────────────────────────────────

    /// Make a change that can be undone (nothing kept when it changes nothing).
    fn change(&mut self, f: impl FnOnce(&mut Self)) {
        let before = self.sheet.anims.clone();
        f(self);
        self.keep(before);
    }

    fn keep(&mut self, before: HashMap<String, Anim>) {
        if before != self.sheet.anims {
            self.undo.push(before);
            if self.undo.len() > 300 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.leaving = None;
        }
    }

    fn undo(&mut self) {
        if let Some(a) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.sheet.anims, a));
            self.fix_selection();
        }
    }

    fn redo(&mut self) {
        if let Some(a) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.sheet.anims, a));
            self.fix_selection();
        }
    }

    fn fix_selection(&mut self) {
        let n = self.pics().len();
        self.selected = self.selected.filter(|_| n > 0).map(|k| k.min(n - 1));
    }

    /// A new picture after the selected one.
    fn add(&mut self, pic: Pic) {
        let at = self.selected.map_or(self.pics().len(), |k| (k + 1).min(self.pics().len()));
        self.pics_mut().insert(at, pic);
        self.selected = Some(at);
        self.playing = false;
    }

    /// A picture for `r` as drawn, centred as the animation is.
    fn plain(&self, r: Rect) -> Pic {
        let centre = match self.feet() {
            Some(hr) => vec2(r.w / 2.0, r.h - hr),
            None => r.size() / 2.0,
        };
        Pic { src: r, centre }
    }

    fn set_selected(&mut self, pic: Pic) {
        if let Some(k) = self.selected.filter(|&k| k < self.pics().len()) {
            self.pics_mut()[k] = pic;
        }
    }

    fn selected_pic(&self) -> Option<Pic> {
        self.selected.and_then(|k| self.pics().get(k).copied())
    }

    /// Tighten the selected picture around its sprite, centred above the
    /// feet or in the middle.
    fn fit(&mut self, feet: bool) {
        let Some(p) = self.selected_pic() else { return };
        let hr = body_radii(self.character, false).1 * self.sheet.scale;
        match find_sprite(&self.sheet.image, p.src, 8.0, feet.then_some(hr)) {
            Some(fitted) => self.change(|ed| ed.set_selected(fitted)),
            None => self.notes.push(Note::Warn("No sprite in this picture".into())),
        }
    }

    fn nudge(&mut self, d: Vec2, how: Nudge) {
        let Some(p) = self.selected_pic() else { return };
        let size = self.size();
        let moved = match how {
            Nudge::Move => {
                let room = size - p.src.size();
                let at = vec2(within(p.src.x + d.x, 0.0, room.x), within(p.src.y + d.y, 0.0, room.y));
                Pic { src: Rect::new(at.x, at.y, p.src.w, p.src.h), ..p }
            }
            Nudge::Size => {
                let room = size - p.src.point();
                let wh = vec2(within(p.src.w + d.x, 1.0, room.x), within(p.src.h + d.y, 1.0, room.y));
                Pic { src: Rect::new(p.src.x, p.src.y, wh.x, wh.y), ..p }
            }
            Nudge::Centre => Pic { centre: p.centre + d, ..p },
        };
        self.change(|ed| ed.set_selected(moved));
    }

    fn delete(&mut self) {
        let Some(k) = self.selected.filter(|&k| k < self.pics().len()) else { return };
        self.change(|ed| {
            ed.pics_mut().remove(k);
        });
        self.fix_selection();
    }

    fn duplicate(&mut self) {
        let Some(p) = self.selected_pic() else { return };
        self.change(|ed| ed.add(p));
    }

    /// Move the selected picture earlier or later in the animation.
    fn shift(&mut self, by: isize) {
        let Some(k) = self.selected else { return };
        let to = k as isize + by;
        if to < 0 || to as usize >= self.pics().len() {
            return;
        }
        self.change(|ed| ed.pics_mut().swap(k, to as usize));
        self.selected = Some(to as usize);
    }

    fn step_picture(&mut self, by: isize) {
        let n = self.pics().len();
        if n == 0 {
            return;
        }
        self.playing = false;
        let k = self.selected.map_or(0, |k| (k as isize + by).rem_euclid(n as isize) as usize);
        self.selected = Some(k);
    }

    fn reset_anim(&mut self) {
        let name = self.name().to_string();
        let was = self.sheet.defaults.get(&name).cloned().unwrap_or_default();
        self.change(|ed| {
            ed.sheet.anims.insert(name, was);
        });
        self.fix_selection();
        if self.selected.is_none() && !self.pics().is_empty() {
            self.selected = Some(0);
        }
    }

    /// Do `ask`, unless it would leave unsaved changes: then say so, and
    /// do it when asked again.
    fn leave(&mut self, ask: Ask) -> Option<Ask> {
        if !self.dirty() || self.leaving == Some(ask) {
            return Some(ask);
        }
        self.leaving = Some(ask);
        let again = if ask == Ask::Close { "Esc again" } else { "click again" };
        self.notes.push(Note::Warn(format!("Unsaved changes: Ctrl+S saves them ({again} leaves them)")));
        None
    }

    /// Keep the changes beside the sheet; returns what to tell.
    pub fn save(&mut self) -> Result<String, String> {
        if self.sheet.anims.get("idle").is_none_or(|a| a.pics.is_empty()) {
            return Err("\"idle\" needs a picture at least".into());
        }
        let r = Regions::changes(self.sheet.image.dimensions(), &self.sheet.defaults, &self.sheet.anims);
        r.save(self.character)?;
        self.saved = self.sheet.anims.clone();
        self.leaving = None;
        let n = r.animations.len();
        Ok(match n {
            0 => format!("{}'s sheet is back to its own layout", self.character.label()),
            1 => format!("Saved {} (1 animation changed)", regions::file_name(self.character)),
            n => format!("Saved {} ({n} animations changed)", regions::file_name(self.character)),
        })
    }

    // ── Layout ──────────────────────────────────────────────────

    fn layout(&self) -> Layout {
        let (top, view) = layout_parts();
        let sw = screen_width();
        let list = Rect::new(12.0, view.y, 180.0, view.h);
        let side = Rect::new(sw - 284.0, view.y, 272.0, view.h);
        let mut buttons = vec![];
        let mut push = |r: Rect, label: &str, cmd: Cmd, on: bool, enabled: bool| {
            buttons.push(Button { r, label: label.to_string(), cmd, on, enabled });
        };
        // The characters, after the title.
        let mut x = 16.0 + measure_bold("Sprite regions", 18.0) + 24.0;
        for c in Character::ALL {
            push(
                Rect::new(x, 10.0, 78.0, 32.0),
                c.short(),
                Cmd::Character(c),
                c == self.character,
                self.available.contains(&c),
            );
            x += 84.0;
        }
        let mut x = top.w - 16.0;
        let dirty = self.dirty();
        for (w, label, cmd, on, enabled) in [
            (84.0, "Close", Cmd::Close, false, true),
            (84.0, "Save", Cmd::Save, dirty, true),
            (60.0, "Redo", Cmd::Redo, false, !self.redo.is_empty()),
            (60.0, "Undo", Cmd::Undo, false, !self.undo.is_empty()),
            (84.0, "Reset all", Cmd::ResetAll, false, differ(&self.sheet.anims, &self.sheet.defaults)),
        ] {
            x -= w;
            push(Rect::new(x, 10.0, w, 32.0), label, cmd, on, enabled);
            x -= 6.0;
        }
        // The side: preview, play controls, layers, pictures, buttons.
        let preview = Rect::new(side.x, side.y, side.w, 180.0);
        let mut y = preview.bottom() + 8.0;
        let w4 = (side.w - 18.0) / 4.0;
        let n = self.pics().len();
        let has = n > 0;
        for (k, (label, cmd, on)) in [
            ("←", Cmd::Previous, false),
            (if self.playing { "Pause" } else { "Play" }, Cmd::Play, self.playing),
            ("→", Cmd::Next, false),
            ("Onion", Cmd::Onion, self.onion),
        ]
        .into_iter()
        .enumerate()
        {
            push(Rect::new(side.x + k as f32 * (w4 + 6.0), y, w4, 28.0), label, cmd, on, has);
        }
        y += 34.0;
        let w2 = (side.w - 6.0) / 2.0;
        if self.layered() {
            push(Rect::new(side.x, y, w2, 28.0), "Body", Cmd::Body, !self.behind, true);
            push(Rect::new(side.x + w2 + 6.0, y, w2, 28.0), "Behind", Cmd::Behind, self.behind, true);
            y += 34.0;
        }
        let cols = ((side.w + THUMB_GAP) / (THUMB + THUMB_GAP)).floor().max(1.0) as usize;
        let thumbs: Vec<Rect> = (0..n)
            .map(|k| {
                let (c, r) = ((k % cols) as f32, (k / cols) as f32);
                Rect::new(side.x + c * (THUMB + THUMB_GAP), y + r * (THUMB + THUMB_GAP), THUMB, THUMB)
            })
            .collect();
        y += n.div_ceil(cols).max(1) as f32 * (THUMB + THUMB_GAP) + 2.0;
        let info = y;
        y += 58.0;
        let sel = self.selected_pic().is_some();
        let k = self.selected.unwrap_or(0);
        for (row, pair) in [
            [("Fit: feet", Cmd::FitFeet, sel), ("Fit: middle", Cmd::FitMiddle, sel)],
            [("← Earlier", Cmd::Earlier, sel && k > 0), ("Later →", Cmd::Later, sel && k + 1 < n)],
            [("Duplicate", Cmd::Duplicate, sel), ("Delete", Cmd::Delete, sel)],
        ]
        .into_iter()
        .enumerate()
        {
            for (c, (label, cmd, enabled)) in pair.into_iter().enumerate() {
                let r = Rect::new(side.x + c as f32 * (w2 + 6.0), y + row as f32 * 34.0, w2, 28.0);
                push(r, label, cmd, false, enabled);
            }
        }
        y += 3.0 * 34.0;
        let none = Anim::default();
        let changed =
            self.sheet.anims.get(self.name()).unwrap_or(&none) != self.sheet.defaults.get(self.name()).unwrap_or(&none);
        push(Rect::new(side.x, y, side.w, 28.0), "Reset this animation", Cmd::ResetAnim, false, changed);
        Layout { list, view, side, preview, thumbs, info, buttons }
    }

    // ── A frame ─────────────────────────────────────────────────

    pub fn frame(&mut self, dt: f32, input: &mut Input) -> Option<Ask> {
        if !self.placed {
            self.placed = true;
            self.frame_view(layout_parts().1);
        }
        if self.playing {
            self.clock += dt;
        }
        let l = self.layout();
        let mut ask = self.click(&l, input);
        if ask.is_none() && self.drag.is_none() {
            ask = self.keys(input, dt, &l);
        }
        self.mouse(input, l.view);
        self.draw(&self.layout(), input.mouse);
        ask
    }

    fn click(&mut self, l: &Layout, input: &mut Input) -> Option<Ask> {
        let m = input.mouse;
        if input.wheel != 0.0 && l.list.contains(m) {
            let most = self.names.len().saturating_sub(self.list_rows(l.list));
            self.scroll = (self.scroll as isize - input.wheel as isize).clamp(0, most as isize) as usize;
            input.wheel = 0.0;
        }
        if !input.left_pressed || self.drag.is_some() {
            return None;
        }
        if let Some(b) = l.buttons.iter().find(|b| b.r.contains(m)) {
            input.consumed = true;
            if !b.enabled {
                if let Cmd::Character(c) = b.cmd {
                    self.notes.push(Note::Info(format!("{} has no sprite sheet: Sonic sprites… adds one", c.label())));
                }
                return None;
            }
            return self.command(b.cmd);
        }
        if let Some(k) = l.thumbs.iter().position(|t| t.contains(m)) {
            input.consumed = true;
            self.selected = Some(k);
            self.playing = false;
            return None;
        }
        if l.list.contains(m) {
            input.consumed = true;
            let row = ((m.y - l.list.y - 30.0) / 24.0).floor();
            if row >= 0.0 {
                let i = self.scroll + row as usize;
                if i < self.names.len() && (row as usize) < self.list_rows(l.list) {
                    self.select_anim(i);
                }
            }
            return None;
        }
        if l.side.contains(m) {
            input.consumed = true;
        }
        None
    }

    fn command(&mut self, cmd: Cmd) -> Option<Ask> {
        match cmd {
            Cmd::Character(c) if c != self.character => return self.leave(Ask::Open(c)),
            Cmd::Character(_) => {}
            Cmd::Save => return Some(Ask::Save),
            Cmd::Close => return self.leave(Ask::Close),
            Cmd::Undo => self.undo(),
            Cmd::Redo => self.redo(),
            Cmd::ResetAll => {
                let defaults = self.sheet.defaults.clone();
                self.change(|ed| ed.sheet.anims = defaults);
                self.fix_selection();
            }
            Cmd::ResetAnim => self.reset_anim(),
            Cmd::Previous => self.step_picture(-1),
            Cmd::Next => self.step_picture(1),
            Cmd::Play => {
                self.playing = !self.playing;
                if !self.playing {
                    // Stop on the picture showing.
                    self.selected = self.shown().or(self.selected);
                }
            }
            Cmd::Onion => self.onion = !self.onion,
            Cmd::Body | Cmd::Behind => {
                self.behind = cmd == Cmd::Behind;
                self.selected = (!self.pics().is_empty()).then_some(0);
            }
            Cmd::FitFeet => self.fit(true),
            Cmd::FitMiddle => self.fit(false),
            Cmd::Earlier => self.shift(-1),
            Cmd::Later => self.shift(1),
            Cmd::Duplicate => self.duplicate(),
            Cmd::Delete => self.delete(),
        }
        None
    }

    /// An arrow key pressed, or held long enough to repeat.
    fn repeat(&mut self, k: KeyCode, dt: f32) -> bool {
        if is_key_pressed(k) {
            self.held.insert(k, 0.0);
            return true;
        }
        if !is_key_down(k) {
            self.held.remove(&k);
            return false;
        }
        let t = self.held.entry(k).or_insert(0.0);
        let was = *t;
        *t += dt;
        const DELAY: f32 = 0.35;
        const EVERY: f32 = 0.04;
        *t > DELAY && ((was - DELAY) / EVERY).floor() != ((*t - DELAY) / EVERY).floor()
    }

    fn keys(&mut self, input: &Input, dt: f32, l: &Layout) -> Option<Ask> {
        let pressed = |k| is_key_pressed(k);
        let (ctrl, shift) = (input.ctrl, input.shift);
        let alt = is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt);
        if pressed(KeyCode::Escape) {
            return self.leave(Ask::Close);
        }
        if ctrl && pressed(KeyCode::S) {
            return Some(Ask::Save);
        }
        if ctrl && pressed(KeyCode::Z) {
            if shift {
                self.redo();
            } else {
                self.undo();
            }
        }
        if ctrl && pressed(KeyCode::Y) {
            self.redo();
        }
        let step = if shift { 10.0 } else { 1.0 };
        let how = if alt {
            Nudge::Centre
        } else if ctrl {
            Nudge::Size
        } else {
            Nudge::Move
        };
        for (k, d) in [
            (KeyCode::Left, vec2(-1.0, 0.0)),
            (KeyCode::Right, vec2(1.0, 0.0)),
            (KeyCode::Up, vec2(0.0, -1.0)),
            (KeyCode::Down, vec2(0.0, 1.0)),
        ] {
            if self.repeat(k, dt) {
                self.nudge(d * step, how);
            }
        }
        if ctrl || alt {
            return None;
        }
        if pressed(KeyCode::Delete) || pressed(KeyCode::Backspace) {
            self.delete();
        }
        if pressed(KeyCode::D) {
            self.duplicate();
        }
        if pressed(KeyCode::F) {
            self.fit(true);
        }
        if pressed(KeyCode::M) {
            self.fit(false);
        }
        if pressed(KeyCode::Space) {
            self.command(Cmd::Play);
        }
        if pressed(KeyCode::O) {
            self.onion = !self.onion;
        }
        if pressed(KeyCode::B) && self.layered() {
            self.command(if self.behind { Cmd::Body } else { Cmd::Behind });
        }
        if pressed(KeyCode::Home) {
            self.frame_view(l.view);
        }
        if pressed(KeyCode::PageUp) && self.anim > 0 {
            self.select_anim(self.anim - 1);
            self.show_in_list(l.list);
        }
        if pressed(KeyCode::PageDown) && self.anim + 1 < self.names.len() {
            self.select_anim(self.anim + 1);
            self.show_in_list(l.list);
        }
        if pressed(KeyCode::LeftBracket) || pressed(KeyCode::Comma) {
            self.step_picture(-1);
        }
        if pressed(KeyCode::RightBracket) || pressed(KeyCode::Period) {
            self.step_picture(1);
        }
        None
    }

    fn list_rows(&self, list: Rect) -> usize {
        ((list.h - 36.0) / 24.0).floor().max(1.0) as usize
    }

    fn show_in_list(&mut self, list: Rect) {
        let rows = self.list_rows(list);
        if self.anim < self.scroll {
            self.scroll = self.anim;
        } else if self.anim >= self.scroll + rows {
            self.scroll = self.anim + 1 - rows;
        }
    }

    /// The selected picture's part under the pointer.
    fn handle_at(&self, m: Vec2) -> Option<Handle> {
        let p = self.selected_pic()?;
        let r = self.screen_rect(p.src);
        if m.distance(self.to_screen(p.src.point() + p.centre)) <= 8.0 {
            return Some(Handle::Centre);
        }
        // Small on screen: room to grab it inside too.
        let reach = 5.0f32.min(r.w / 4.0).min(r.h / 4.0).max(2.0);
        let near = |a: f32, b: f32| (a - b).abs() <= reach;
        let along_y = m.y >= r.y - reach && m.y <= r.bottom() + reach;
        let along_x = m.x >= r.x - reach && m.x <= r.right() + reach;
        let ex = if along_y && near(m.x, r.x) {
            -1
        } else if along_y && near(m.x, r.right()) {
            1
        } else {
            0
        };
        let ey = if along_x && near(m.y, r.y) {
            -1
        } else if along_x && near(m.y, r.bottom()) {
            1
        } else {
            0
        };
        if ex != 0 || ey != 0 {
            return Some(Handle::Edge(ex, ey));
        }
        r.contains(m).then_some(Handle::Inside)
    }

    fn grab(&mut self, m: Vec2, p: Vec2) -> Drag {
        if let Some(pic) = self.selected_pic() {
            match self.handle_at(m) {
                Some(Handle::Centre) => return Drag::Centre(p, pic),
                Some(Handle::Edge(ex, ey)) => return Drag::Resize(p, pic, (ex, ey)),
                Some(Handle::Inside) => return Drag::Move(p, pic),
                None => {}
            }
        }
        if let Some(k) = self.pics().iter().rposition(|q| q.src.contains(p)) {
            self.selected = Some(k);
            self.playing = false;
            return Drag::Move(p, self.pics()[k]);
        }
        Drag::New(p)
    }

    /// The pictures of other animations under the pointer: (animation, picture).
    fn others_at(&self, p: Vec2) -> Vec<(usize, usize)> {
        let mut out = vec![];
        for i in (1..=self.names.len()).map(|d| (self.anim + d) % self.names.len()) {
            if i == self.anim {
                continue;
            }
            if let Some(a) = self.sheet.anims.get(&self.names[i].0) {
                if let Some(k) = a.pics.iter().position(|q| q.src.contains(p)) {
                    out.push((i, k));
                }
            }
        }
        out
    }

    fn mouse(&mut self, input: &mut Input, view: Rect) {
        let m = input.mouse;
        let p = self.to_sheet(m);
        let size = self.size();
        if input.wheel != 0.0 && view.contains(m) {
            let i = ZOOMS.iter().position(|&z| z >= self.zoom - 0.001).unwrap_or(ZOOMS.len() - 1);
            let j = (i as isize + input.wheel as isize).clamp(0, ZOOMS.len() as isize - 1) as usize;
            let z = ZOOMS[j];
            self.origin = (m - (m - self.origin) * z / self.zoom).round();
            self.zoom = z;
        }
        let panning = is_mouse_button_pressed(MouseButton::Right) || is_mouse_button_pressed(MouseButton::Middle);
        if panning && view.contains(m) && self.drag.is_none() {
            self.drag = Some(Drag::Pan(m, self.origin));
        }
        if input.left_pressed && !input.consumed && view.contains(m) && self.drag.is_none() {
            input.consumed = true;
            self.before = Some(self.sheet.anims.clone());
            self.drag = Some(self.grab(m, p));
        }
        match self.drag {
            Some(Drag::Move(from, was)) => {
                let (d, room) = ((p - from).round(), size - was.src.size());
                let at = vec2(within(was.src.x + d.x, 0.0, room.x), within(was.src.y + d.y, 0.0, room.y));
                self.set_selected(Pic { src: Rect::new(at.x, at.y, was.src.w, was.src.h), ..was });
            }
            Some(Drag::Resize(from, was, (ex, ey))) => {
                let d = (p - from).round();
                let (mut x0, mut y0, mut x1, mut y1) = (was.src.x, was.src.y, was.src.right(), was.src.bottom());
                match ex {
                    -1 => x0 = within(x0 + d.x, 0.0, x1 - 1.0).min(x1 - 1.0),
                    1 => x1 = within(x1 + d.x, x0 + 1.0, size.x),
                    _ => {}
                }
                match ey {
                    -1 => y0 = within(y0 + d.y, 0.0, y1 - 1.0).min(y1 - 1.0),
                    1 => y1 = within(y1 + d.y, y0 + 1.0, size.y),
                    _ => {}
                }
                // The character stays where it is on the sheet.
                let centre = was.centre + was.src.point() - vec2(x0, y0);
                self.set_selected(Pic { src: Rect::new(x0, y0, x1 - x0, y1 - y0), centre });
            }
            Some(Drag::Centre(from, was)) => {
                let centre = ((was.centre + p - from) * 2.0).round() / 2.0;
                self.set_selected(Pic { centre, ..was });
            }
            Some(Drag::Pan(from, o)) => self.origin = (o + m - from).round(),
            _ => {}
        }
        let right_up = is_mouse_button_released(MouseButton::Right) || is_mouse_button_released(MouseButton::Middle);
        if let Some(Drag::Pan(from, _)) = self.drag {
            if right_up {
                self.drag = None;
                // A right click, not a drag: to the picture of another animation under it.
                if m.distance(from) < 4.0 {
                    if let Some(&(i, k)) = self.others_at(p).first() {
                        let keep = (self.origin, self.zoom);
                        self.select_anim(i);
                        (self.origin, self.zoom) = keep;
                        self.selected = Some(k);
                        self.playing = false;
                    }
                }
            }
            return;
        }
        if !input.left_released {
            return;
        }
        if let Some(Drag::New(from)) = self.drag {
            let r = rect_between(from, p, size);
            if m.distance(self.to_screen(from)) > 4.0 && r.w >= 2.0 && r.h >= 2.0 {
                let pic = find_sprite(&self.sheet.image, r, 0.0, self.feet()).unwrap_or_else(|| self.plain(r));
                self.add(pic);
            } else if get_time() - self.last_click.0 < 0.4 && m.distance(self.last_click.1) < 6.0 {
                // A double click: the sprite under it (or touching the pointer).
                let reach = (6.0 / self.zoom).max(1.0).ceil();
                let dot = Rect::new(p.x.floor() - reach, p.y.floor() - reach, 2.0 * reach + 1.0, 2.0 * reach + 1.0);
                match find_sprite(&self.sheet.image, dot, 64.0, self.feet()) {
                    Some(pic) => self.add(pic),
                    None => self.notes.push(Note::Info("No sprite there: double-click on one".into())),
                }
                self.last_click = (0.0, m);
            } else {
                self.selected = None;
                self.last_click = (get_time(), m);
            }
        }
        self.drag = None;
        if let Some(before) = self.before.take() {
            self.keep(before);
        }
    }

    /// The picture the preview shows of the layer being edited.
    fn shown(&self) -> Option<usize> {
        let n = self.pics().len();
        if n == 0 {
            return None;
        }
        if self.playing {
            let fps = if self.behind { BEHIND_FPS } else { PREVIEW_FPS };
            Some((self.clock * fps) as usize % n)
        } else {
            self.selected.map(|k| k.min(n - 1))
        }
    }

    // ── Drawing ─────────────────────────────────────────────────

    fn draw(&self, l: &Layout, m: Vec2) {
        clear_background(Color::new(0.045, 0.04, 0.085, 1.0));
        self.draw_view(l.view, m);
        self.draw_list(l.list, m);
        self.draw_preview(l.preview);
        self.draw_thumbs(l, m);
        self.draw_info(l);
        self.draw_top();
        for b in &l.buttons {
            draw_btn(b, m);
        }
        self.draw_help();
    }

    fn draw_top(&self) {
        text_bold("Sprite regions", 16.0, baseline(22.0, 18.0), 18.0, TEXT);
        let (w, h) = self.sheet.image.dimensions();
        let mark = if self.dirty() { "  ·  unsaved" } else { "" };
        text(&format!("{}  ·  {w} × {h}{mark}", self.sheet.name), 16.0, baseline(41.0, 11.0), 11.0, TEXT_MUTED);
    }

    fn draw_help(&self) {
        let parts = [
            "Drag on the sheet: new picture",
            "Double-click a sprite: add it",
            "Drag a box, its edges, its cross",
            "Arrows move (Shift ×10, Ctrl size, Alt centre)",
            "F / M fit",
            "Wheel zoom",
            "Right-drag pan",
            "Right-click: go to a picture",
            "Ctrl+Z undo",
            "Ctrl+S save",
        ];
        let room = screen_width() - 32.0;
        let mut line = String::new();
        for p in parts {
            let next = if line.is_empty() { p.to_string() } else { format!("{line}  ·  {p}") };
            if measure(&next, 11.0) > room {
                break;
            }
            line = next;
        }
        text(&line, 16.0, baseline(screen_height() - 17.0, 11.0), 11.0, TEXT_MUTED);
    }

    fn draw_list(&self, list: Rect, m: Vec2) {
        rrect(list, RADIUS, SURFACE);
        rrect_lines(list, RADIUS, 1.0, BORDER);
        text_bold("ANIMATIONS", list.x + 12.0, list.y + 20.0, 11.0, TEXT_MUTED);
        let none = Anim::default();
        for (row, i) in (self.scroll..self.names.len()).take(self.list_rows(list)).enumerate() {
            let name = &self.names[i].0;
            let r = Rect::new(list.x + 6.0, list.y + 30.0 + row as f32 * 24.0, list.w - 12.0, 22.0);
            if i == self.anim {
                rrect(r, 6.0, SURFACE_HI);
                draw_rectangle(r.x, r.y + 4.0, 3.0, r.h - 8.0, ACCENT);
            } else if r.contains(m) {
                rrect(r, 6.0, alpha(SURFACE_HI, 0.5));
            }
            let now = self.sheet.anims.get(name).unwrap_or(&none);
            let n = now.pics.len();
            text_left(r, 10.0, name, 13.0, if n > 0 { TEXT } else { TEXT_MUTED });
            let count = n.to_string();
            let cw = measure(&count, 12.0);
            text(&count, r.right() - 8.0 - cw, baseline(r.y + r.h / 2.0, 12.0), 12.0, TEXT_DIM);
            // Changed from the sheet's layout (amber: not saved yet).
            if now != self.sheet.defaults.get(name).unwrap_or(&none) || now != self.saved.get(name).unwrap_or(&none) {
                let unsaved = now != self.saved.get(name).unwrap_or(&none);
                draw_circle(r.right() - 16.0 - cw, r.y + r.h / 2.0, 3.0, if unsaved { WARNING } else { ACCENT });
            }
        }
        if self.scroll + self.list_rows(list) < self.names.len() {
            text_centered("…", list.x + list.w / 2.0, list.bottom() - 8.0, 13.0, TEXT_MUTED);
        }
    }

    fn draw_view(&self, v: Rect, m: Vec2) {
        rrect(v, RADIUS, Color::new(0.03, 0.028, 0.06, 1.0));
        clip(Some(v));
        let size = self.size();
        let sr = self.screen_rect(Rect::new(0.0, 0.0, size.x, size.y));
        draw_rectangle(sr.x, sr.y, sr.w, sr.h, Color::new(0.15, 0.15, 0.19, 1.0));
        draw_texture_ex(
            &self.sheet.texture,
            sr.x,
            sr.y,
            WHITE,
            DrawTextureParams { dest_size: Some(sr.size()), ..Default::default() },
        );
        // Pixels, close up.
        if self.zoom >= 8.0 {
            let grid = alpha(WHITE, 0.06);
            let (a, b) = (
                self.to_sheet(v.point()).max(Vec2::ZERO).floor(),
                self.to_sheet(v.point() + v.size()).min(size).ceil(),
            );
            for x in a.x as i32..=b.x as i32 {
                let sx = self.to_screen(vec2(x as f32, 0.0)).x;
                draw_line(sx, sr.y.max(v.y), sx, sr.bottom().min(v.bottom()), 1.0, grid);
            }
            for y in a.y as i32..=b.y as i32 {
                let sy = self.to_screen(vec2(0.0, y as f32)).y;
                draw_line(sr.x.max(v.x), sy, sr.right().min(v.right()), sy, 1.0, grid);
            }
        }
        draw_rectangle_lines(sr.x, sr.y, sr.w, sr.h, 1.0, BORDER);
        // The other animations' pictures, faintly.
        let name = self.name();
        for (n, a) in &self.sheet.anims {
            if n == name {
                continue;
            }
            for q in &a.pics {
                let r = self.screen_rect(q.src);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, alpha(TEXT_MUTED, 0.3));
            }
        }
        // This animation's other layer.
        if let Some(a) = self.sheet.anims.get(name) {
            let other = if self.behind { &a.pics } else { &a.behind };
            for q in other {
                let r = self.screen_rect(q.src);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, alpha(WARNING, 0.55));
            }
        }
        // Its pictures, numbered.
        for (k, q) in self.pics().iter().enumerate() {
            let r = self.screen_rect(q.src);
            let sel = Some(k) == self.selected;
            if sel {
                draw_rectangle(r.x, r.y, r.w, r.h, alpha(ACCENT, 0.12));
            }
            draw_rectangle_lines(r.x, r.y, r.w, r.h, if sel { 2.0 } else { 1.5 }, if sel { ACCENT_HI } else { ACCENT });
            let label = (k + 1).to_string();
            let lw = measure_bold(&label, 11.0) + 8.0;
            let tag = Rect::new(r.x, r.y - 15.0, lw, 15.0);
            draw_rectangle(tag.x, tag.y, tag.w, tag.h, if sel { ACCENT_HI } else { ACCENT });
            text_bold(&label, tag.x + 4.0, baseline(tag.y + 7.5, 11.0), 11.0, Color::new(0.05, 0.04, 0.1, 1.0));
        }
        if let Some(q) = self.selected_pic() {
            self.draw_selected(q, m);
        }
        if let Some(Drag::New(from)) = self.drag {
            let r = self.screen_rect(rect_between(from, self.to_sheet(m), size));
            dashed_rect(r, ACCENT_HI);
        }
        clip(None);
        rrect_lines(v, RADIUS, 1.0, BORDER);
        // What this animation is.
        let (name, what) = &self.names[self.anim];
        let n = self.pics().len();
        let layer = if self.behind { "  ·  behind" } else { "" };
        let caption = format!("{name}  —  {what}  ·  {n} picture{}{layer}", if n == 1 { "" } else { "s" });
        let bar = Rect::new(v.x + 1.0, v.y + 1.0, v.w - 2.0, 28.0);
        draw_rectangle(bar.x, bar.y, bar.w, bar.h, alpha(SURFACE, 0.92));
        text_left(bar, 12.0, &caption, 13.0, TEXT);
        let z = format!("×{}", self.zoom);
        text(&z, bar.right() - 12.0 - measure(&z, 12.0), baseline(bar.y + 14.0, 12.0), 12.0, TEXT_MUTED);
        // Which animation a faint picture is.
        if self.drag.is_none() && v.contains(m) && self.handle_at(m).is_none() {
            let p = self.to_sheet(m);
            if !self.pics().iter().any(|q| q.src.contains(p)) {
                if let Some(&(i, k)) = self.others_at(p).first() {
                    let tip = format!("{} {}  ·  right-click to edit it", self.names[i].0, k + 1);
                    let tw = measure(&tip, 12.0) + 16.0;
                    let r = Rect::new((m.x + 14.0).min(v.right() - tw - 4.0), m.y + 18.0, tw, 22.0);
                    rrect(r, 6.0, SURFACE_HI);
                    text_left(r, 8.0, &tip, 12.0, TEXT);
                }
            }
        }
    }

    /// The selected picture's handles, the character's body and centre.
    fn draw_selected(&self, q: Pic, m: Vec2) {
        let r = self.screen_rect(q.src);
        let c = self.to_screen(q.src.point() + q.centre);
        let body = self.radii() * self.zoom;
        let b = Rect::new(c.x - body.x, c.y - body.y, body.x * 2.0, body.y * 2.0);
        draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.0, alpha(SUCCESS, 0.85));
        // The ground under the feet.
        dashed_line(vec2(b.x - 16.0, b.bottom()), vec2(b.right() + 16.0, b.bottom()), 1.0, SUCCESS);
        let hover = if self.drag.is_none() { self.handle_at(m) } else { None };
        for ex in [-1i8, 0, 1] {
            for ey in [-1i8, 0, 1] {
                if ex == 0 && ey == 0 {
                    continue;
                }
                let hx = r.x + r.w * (ex as f32 + 1.0) / 2.0;
                let hy = r.y + r.h * (ey as f32 + 1.0) / 2.0;
                let hot = hover == Some(Handle::Edge(ex, ey));
                let s = if hot { 9.0 } else { 7.0 };
                draw_rectangle(hx - s / 2.0, hy - s / 2.0, s, s, if hot { WHITE } else { ACCENT_HI });
                draw_rectangle_lines(hx - s / 2.0, hy - s / 2.0, s, s, 1.0, Color::new(0.05, 0.04, 0.1, 1.0));
            }
        }
        let hot = hover == Some(Handle::Centre) || matches!(self.drag, Some(Drag::Centre(..)));
        let red = if hot { WHITE } else { DANGER };
        draw_circle_lines(c.x, c.y, 6.0, 1.5, red);
        draw_line(c.x - 11.0, c.y, c.x + 11.0, c.y, 1.5, red);
        draw_line(c.x, c.y - 11.0, c.x, c.y + 11.0, 1.5, red);
    }

    /// The preview's zoom: the same for every animation of the sheet, so
    /// they compare.
    fn preview_zoom(&self, pv: Rect) -> f32 {
        let stand = body_radii(self.character, false).1 * self.sheet.scale;
        let all = self.sheet.anims.values().flat_map(|a| a.pics.iter().chain(&a.behind));
        let (half, up) =
            all.fold((0.0f32, 0.0f32), |(h, u), p| (h.max(p.centre.x).max(p.src.w - p.centre.x), u.max(p.centre.y)));
        ((pv.w - 40.0) / (2.0 * half.max(1.0))).min((pv.h - 50.0) / (up + stand)).floor().clamp(1.0, 6.0)
    }

    fn draw_preview(&self, pv: Rect) {
        rrect(pv, RADIUS, SURFACE_2);
        rrect_lines(pv, RADIUS, 1.0, BORDER);
        let Some(a) = self.sheet.anims.get(self.name()).filter(|a| !a.pics.is_empty() || !a.behind.is_empty()) else {
            text_centered("No pictures yet", pv.x + pv.w / 2.0, pv.y + pv.h / 2.0 - 10.0, 13.0, TEXT_DIM);
            let hint = "Drag on the sheet, or double-click a sprite";
            text_centered(hint, pv.x + pv.w / 2.0, pv.y + pv.h / 2.0 + 10.0, 11.0, TEXT_MUTED);
            return;
        };
        let body = self.radii();
        let z = self.preview_zoom(pv);
        let ground = pv.bottom() - 26.0;
        let c = vec2(pv.x + pv.w / 2.0, ground - body.y * z).round();
        clip(Some(pv));
        draw_line(pv.x + 12.0, ground, pv.right() - 12.0, ground, 1.0, alpha(TEXT_MUTED, 0.6));
        let b = body * z;
        draw_rectangle_lines(c.x - b.x, c.y - b.y, b.x * 2.0, b.y * 2.0, 1.0, alpha(SUCCESS, 0.45));
        let tex = &self.sheet.texture;
        let at = |list: &[Pic], editing: bool, fps: f32| -> Option<usize> {
            if list.is_empty() {
                None
            } else if editing {
                self.shown()
            } else {
                Some((self.clock * fps) as usize % list.len())
            }
        };
        let (kb, kp) = (at(&a.behind, self.behind, BEHIND_FPS), at(&a.pics, !self.behind, PREVIEW_FPS));
        // The picture before, faintly, to line them up.
        let (list, k) = if self.behind { (&a.behind, kb) } else { (&a.pics, kp) };
        if let Some(k) = k.filter(|_| self.onion && list.len() > 1) {
            draw_pic(tex, list[(k + list.len() - 1) % list.len()], c, z, alpha(WHITE, 0.35));
        }
        if let Some(k) = kb {
            draw_pic(tex, a.behind[k], c, z, WHITE);
        }
        if let Some(k) = kp {
            draw_pic(tex, a.pics[k], c, z, WHITE);
        }
        draw_circle(c.x, c.y, 2.0, DANGER);
        clip(None);
        text(&format!("Preview  ×{z}"), pv.x + 10.0, baseline(pv.y + 14.0, 11.0), 11.0, TEXT_MUTED);
        if let Some(k) = k {
            let s = format!("{} / {}", k + 1, list.len());
            text(&s, pv.right() - 10.0 - measure(&s, 11.0), baseline(pv.y + 14.0, 11.0), 11.0, TEXT_MUTED);
        }
    }

    fn draw_thumbs(&self, l: &Layout, m: Vec2) {
        let shown = if self.playing { self.shown() } else { None };
        for (k, (t, q)) in l.thumbs.iter().zip(self.pics()).enumerate() {
            let sel = Some(k) == self.selected;
            rrect(*t, 6.0, if t.contains(m) { SURFACE_HI } else { SURFACE_2 });
            let z = ((t.w - 10.0) / q.src.w).min((t.h - 10.0) / q.src.h);
            let z = if z >= 1.0 { z.floor() } else { z };
            let at = vec2(t.x + (t.w - q.src.w * z) / 2.0, t.y + (t.h - q.src.h * z) / 2.0).round();
            draw_texture_ex(
                &self.sheet.texture,
                at.x,
                at.y,
                WHITE,
                DrawTextureParams { source: Some(q.src), dest_size: Some(q.src.size() * z), ..Default::default() },
            );
            let border = if sel {
                ACCENT_HI
            } else if shown == Some(k) {
                SUCCESS
            } else {
                BORDER
            };
            rrect_lines(*t, 6.0, if sel { 2.0 } else { 1.0 }, border);
            text_bold(&(k + 1).to_string(), t.x + 5.0, t.y + 13.0, 10.0, TEXT_DIM);
        }
        if l.thumbs.is_empty() {
            let y = l.preview.bottom() + 8.0 + 34.0 + if self.layered() { 34.0 } else { 0.0 };
            text("No pictures in this animation", l.side.x + 4.0, baseline(y + THUMB / 2.0, 12.0), 12.0, TEXT_MUTED);
        }
    }

    fn draw_info(&self, l: &Layout) {
        let x = l.side.x + 4.0;
        let line = |k: f32, s: &str, c: Color| text(s, x, baseline(l.info + 9.0 + k * 18.0, 12.0), 12.0, c);
        match (self.selected, self.selected_pic()) {
            (Some(k), Some(q)) => {
                line(0.0, &format!("Picture {} of {}", k + 1, self.pics().len()), TEXT);
                let r = q.src;
                line(1.0, &format!("x {}   y {}   ·   {} × {}", num(r.x), num(r.y), num(r.w), num(r.h)), TEXT_DIM);
                let (cx, cy) = (num(q.centre.x), num(q.centre.y));
                line(2.0, &format!("Centre {cx}, {cy}   (from its top-left)"), TEXT_DIM);
            }
            _ => {
                line(0.0, "No picture selected", TEXT_DIM);
                line(1.0, "Click one on the sheet or below", TEXT_MUTED);
            }
        }
    }
}

fn draw_btn(b: &Button, m: Vec2) {
    let hovered = b.enabled && b.r.contains(m);
    let bg = if b.on && b.enabled {
        alpha(ACCENT, 0.9)
    } else if hovered {
        SURFACE_HI
    } else {
        SURFACE_2
    };
    rrect(b.r, 7.0, if b.enabled { bg } else { alpha(SURFACE_2, 0.5) });
    rrect_lines(b.r, 7.0, 1.0, if hovered || (b.on && b.enabled) { BORDER_HI } else { BORDER });
    let c = if b.enabled { TEXT } else { TEXT_MUTED };
    text_centered(&b.label, b.r.x + b.r.w / 2.0, b.r.y + b.r.h / 2.0, 13.0, c);
}

/// The top bar and the sheet view (the rest is laid out around them).
fn layout_parts() -> (Rect, Rect) {
    let (sw, sh) = (screen_width(), screen_height());
    let (y, bottom) = (60.0, sh - 34.0);
    let top = Rect::new(0.0, 0.0, sw, 52.0);
    let view = Rect::new(204.0, y, (sw - 284.0 - 12.0 - 204.0).max(100.0), bottom - y);
    (top, view)
}

impl App {
    /// Open the sprite region editor on `c`'s sheet (else the character
    /// played, else the first with a sheet).
    pub(super) fn open_regions(&mut self, c: Option<Character>) {
        let available: Vec<Character> = Character::ALL.into_iter().filter(|&c| sheet::has_sheet(c)).collect();
        let Some(&first) = available.first() else {
            self.toasts.warn("No character sprite sheet yet: add one with Sonic sprites… first");
            return;
        };
        let c = c.or(Some(self.s.character)).filter(|c| available.contains(c)).unwrap_or(first);
        match Editable::open(c) {
            Ok(Some(sheet)) => self.regions = Some(RegionEditor::new(c, available, sheet)),
            Ok(None) => self.toasts.warn(format!("{} has no sprite sheet", c.label())),
            Err(e) => self.toasts.error(format!("{}'s sheet: {e}", c.label())),
        }
    }

    /// Into the editor from the command line (`--sprite-regions`).
    pub fn edit_regions(&mut self, c: Option<Character>) {
        self.open_regions(c);
    }

    /// A frame of the editor, over the whole screen.
    pub(super) fn regions_frame(&mut self, dt: f32) {
        set_default_camera();
        let mut input = Input::gather();
        let Some(ed) = self.regions.as_mut() else { return };
        let ask = ed.frame(dt, &mut input);
        for n in ed.notes.drain(..) {
            match n {
                Note::Info(t) => self.toasts.info(t),
                Note::Warn(t) => self.toasts.warn(t),
            }
        }
        match ask {
            Some(Ask::Close) => self.regions = None,
            Some(Ask::Save) => self.save_regions(),
            Some(Ask::Open(c)) => self.open_regions(Some(c)),
            None => {}
        }
        self.toasts.update(dt);
        self.toasts.draw(56.0, &self.jobs, dt);
    }

    fn save_regions(&mut self) {
        let Some(ed) = self.regions.as_mut() else { return };
        match ed.save() {
            Ok(said) => {
                self.toasts.success(said);
                // The game draws from them at once.
                self.sonic_sheet_tried = true;
                self.load_sonic_sheet();
            }
            Err(e) => self.toasts.error(format!("Not saved: {e}")),
        }
    }
}
