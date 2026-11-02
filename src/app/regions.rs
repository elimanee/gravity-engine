//! The sprite region editor: on a character's sheet, which pictures each of
//! its animations uses, where the character's centre is in each, how long
//! each shows and at what pace the animation plays. Any picture can be a
//! character's sheet: its background colours are picked here and its
//! sprites found by themselves. It takes the whole screen; what is saved
//! goes beside the sheet as `<character>.regions.json` (see
//! [`crate::sonic::regions`]), the game draws from it at once, and *Try*
//! plays the character as edited on a test track first.

use super::*;
use crate::library::acts;
use crate::sonic::regions;
use crate::sonic::sheet::{self, detect_sprites, find_sprite, Anim, Editable, Pic, Repeat};
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
const ZOOMS: &[f32] = &[0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 12.0, 16.0, 20.0, 24.0, 32.0];

/// The paces an animation can be given (pictures a second), after the
/// games' own.
const PACES: &[f32] = &[4.0, 6.0, 8.0, 10.0, 12.0, 15.0, 20.0, 24.0, 30.0];

/// Sheet pixels per guide pixel a sheet can be given.
const SCALES: &[f32] = &[0.5, 1.0, 1.5, 2.0, 3.0, 4.0];

/// The longest a picture can be held (in pictures), and the most times a
/// run can repeat.
const MOST_HOLD: f32 = 8.0;
const MOST_REPEATS: u32 = 16;

/// The preview plays the games' own pace at this many pictures a second
/// (in the game it follows the character's speed); the pieces behind at
/// their own.
const PREVIEW_FPS: f32 = 10.0;
const BEHIND_FPS: f32 = 8.0;

/// Thumbnails of the animation's pictures.
const THUMB: f32 = 48.0;
const THUMB_GAP: f32 = 6.0;

/// The left panel's sheet settings, under the animations.
const SHEET_BOX: f32 = 160.0;

/// What the editor asks of the app.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Ask {
    Close,
    Save,
    /// Edit this character's sheet instead.
    Open(Character),
    /// Choose a picture to be this character's sheet.
    OpenSheet(Character),
    /// Play the character as edited.
    Try,
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
    /// Every sprite found in a box (Shift), from this sheet point.
    Gather(Vec2),
    /// Moving the chosen pictures, their centres with them, from this
    /// sheet point.
    Move(Vec2),
    /// The selected picture's edges (-1: left / top, 1: right / bottom).
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
    OpenSheet,
    Try,
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
    Slower,
    Faster,
    Once,
    HoldLess,
    HoldMore,
    Mirror,
    LoopHere,
    RepeatLess,
    RepeatMore,
    FitFeet,
    FitMiddle,
    SameCentre,
    Earlier,
    Later,
    Duplicate,
    Copy,
    Paste,
    Delete,
    Pick,
    Unpick(usize),
    Soft,
    Smaller,
    Bigger,
}

struct Button {
    r: Rect,
    label: String,
    cmd: Cmd,
    on: bool,
    enabled: bool,
}

/// `◀ label value ▶`: the left third goes down, the rest up.
struct Stepper {
    r: Rect,
    label: &'static str,
    value: String,
    less: Cmd,
    more: Cmd,
    enabled: bool,
}

/// Where everything is this frame.
struct Layout {
    list: Rect,
    /// The sheet's settings, under the animations.
    sheet_box: Rect,
    view: Rect,
    side: Rect,
    preview: Rect,
    thumbs: Vec<Rect>,
    /// Pictures with no room for a thumbnail.
    unshown: usize,
    /// Top of the selected picture's numbers.
    info: f32,
    buttons: Vec<Button>,
    steppers: Vec<Stepper>,
    /// The background colours: where, which, and the index of a picked one
    /// (the layout's own cannot be taken away).
    swatches: Vec<(Rect, [u8; 3], Option<usize>)>,
}

/// Nudging the chosen pictures with the arrow keys.
#[derive(Clone, Copy)]
enum Nudge {
    Move,
    Size,
    Centre,
}

/// The sheet-wide settings saved with the regions.
#[derive(Clone, PartialEq)]
struct Settings {
    background: Vec<[u8; 3]>,
    soft: bool,
    scale: f32,
}

pub(super) struct RegionEditor {
    pub character: Character,
    /// The characters with a sheet.
    available: Vec<Character>,
    pub sheet: Editable,
    /// The sprites found on the sheet, and whether it has any transparency.
    sprites: Vec<Rect>,
    clear: bool,
    /// The animations listed, and when the game shows each.
    names: Vec<(String, &'static str)>,
    anim: usize,
    /// Editing the pieces drawn behind (Tails' tails).
    behind: bool,
    /// The picture handles are on, and every picture chosen with it
    /// (sorted; it is one of them).
    selected: Option<usize>,
    marked: Vec<usize>,
    /// The sheet's top-left on screen, and screen px per sheet px.
    origin: Vec2,
    zoom: f32,
    /// The view is placed once the screen's size is known.
    placed: bool,
    drag: Option<Drag>,
    /// The chosen pictures as they were when the drag began.
    grabbed: Vec<(usize, Pic)>,
    /// The animations before the drag (to undo it).
    before: Option<HashMap<String, Anim>>,
    undo: Vec<HashMap<String, Anim>>,
    redo: Vec<HashMap<String, Anim>>,
    /// What was last saved (or found), to tell unsaved changes.
    saved: HashMap<String, Anim>,
    saved_settings: Settings,
    /// Pictures copied (Ctrl+C), to paste in any animation.
    clipboard: Vec<Pic>,
    /// The next click on the sheet picks a background colour.
    picking: bool,
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
    /// Back from trying the sprites: the Esc that brought it back is not
    /// for the editor.
    pub fresh: bool,
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

/// "Sonic's", "Tails'".
fn whose(c: Character) -> String {
    let name = c.label();
    if name.ends_with('s') {
        format!("{name}'")
    } else {
        format!("{name}'s")
    }
}

fn plural(n: usize, one: &str) -> String {
    if n == 1 {
        format!("1 {one}")
    } else {
        format!("{n} {one}s")
    }
}

fn rect_between(a: Vec2, b: Vec2, size: Vec2) -> Rect {
    let lo = a.min(b).floor().max(Vec2::ZERO);
    let hi = a.max(b).ceil().min(size);
    Rect::new(lo.x, lo.y, (hi.x - lo.x).max(0.0), (hi.y - lo.y).max(0.0))
}

/// Boxes in reading order: row by row (a box whose middle is level with
/// one of a row's is on that row), left to right.
fn reading_order(mut boxes: Vec<Rect>) -> Vec<Rect> {
    boxes.sort_by(|a, b| a.y.total_cmp(&b.y));
    let mut rows: Vec<Vec<Rect>> = vec![];
    for b in boxes {
        let mid = b.y + b.h / 2.0;
        match rows.last_mut() {
            Some(row) if row.iter().any(|r| mid >= r.y && mid <= r.bottom()) => row.push(b),
            _ => rows.push(vec![b]),
        }
    }
    rows.into_iter()
        .flat_map(|mut row| {
            row.sort_by(|a, b| a.x.total_cmp(&b.x));
            row
        })
        .collect()
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
/// screen px per sheet px; mirrored when it is.
fn draw_pic(texture: &Texture2D, pic: Pic, at: Vec2, zoom: f32, tint: Color) {
    let cx = if pic.flip { pic.src.w - pic.centre.x } else { pic.centre.x };
    let top_left = (at - vec2(cx, pic.centre.y) * zoom).round();
    draw_texture_ex(
        texture,
        top_left.x,
        top_left.y,
        tint,
        DrawTextureParams {
            source: Some(pic.src),
            dest_size: Some(vec2(pic.src.w, pic.src.h) * zoom),
            flip_x: pic.flip,
            ..Default::default()
        },
    );
}

/// An animation's pace as shown.
fn pace(fps: Option<f32>) -> String {
    fps.map_or_else(|| "Game".to_string(), |f| format!("{} /s", num(f)))
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
        if !sheet.known && sheet.anims.values().all(|a| a.pics.is_empty()) {
            let (w, h) = sheet.image.dimensions();
            notes.push(Note::Info(format!(
                "A sheet new to the game ({w} × {h}): pick its background, then give each animation its pictures"
            )));
        }
        let saved = sheet.anims.clone();
        let saved_settings = Settings { background: sheet.background.clone(), soft: sheet.soft, scale: sheet.scale };
        let mut ed = RegionEditor {
            character,
            available,
            sheet,
            sprites: vec![],
            clear: true,
            names,
            anim: 0,
            behind: false,
            selected: None,
            marked: vec![],
            origin: Vec2::ZERO,
            zoom: 1.0,
            placed: false,
            drag: None,
            grabbed: vec![],
            before: None,
            undo: vec![],
            redo: vec![],
            saved,
            saved_settings,
            clipboard: vec![],
            picking: false,
            playing: true,
            onion: false,
            clock: 0.0,
            leaving: None,
            last_click: (0.0, Vec2::ZERO),
            scroll: 0,
            held: HashMap::new(),
            fresh: false,
            notes,
        };
        ed.find_sprites();
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

    fn current(&self) -> Option<&Anim> {
        self.sheet.anims.get(self.name())
    }

    fn current_mut(&mut self) -> &mut Anim {
        let name = self.names[self.anim].0.clone();
        self.sheet.anims.entry(name).or_default()
    }

    fn pics(&self) -> &[Pic] {
        self.current().map_or(&[], |a| if self.behind { &a.behind } else { &a.pics })
    }

    fn pics_mut(&mut self) -> &mut Vec<Pic> {
        let behind = self.behind;
        let a = self.current_mut();
        if behind {
            &mut a.behind
        } else {
            &mut a.pics
        }
    }

    fn settings(&self) -> Settings {
        Settings { background: self.sheet.background.clone(), soft: self.sheet.soft, scale: self.sheet.scale }
    }

    fn dirty(&self) -> bool {
        differ(&self.sheet.anims, &self.saved) || self.settings() != self.saved_settings
    }

    /// The pieces drawn behind are shown (Tails, or a sheet that has some).
    fn layered(&self) -> bool {
        self.character == Character::Tails || self.current().is_some_and(|a| !a.behind.is_empty())
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

    /// Look for the sheet's sprites again (after its background changed).
    fn find_sprites(&mut self) {
        self.sprites = detect_sprites(&self.sheet.image);
        self.clear = self.sheet.image.pixels().any(|p| p[3] < 128);
    }

    fn select_anim(&mut self, i: usize) {
        self.anim = i.min(self.names.len() - 1);
        self.behind = self.behind && self.layered();
        self.select_only((!self.pics().is_empty()).then_some(0));
        self.clock = 0.0;
        if self.placed {
            self.frame_view(layout_parts().1);
        }
    }

    /// Fit the view around the animation's pictures (the whole sheet when
    /// it has none).
    fn frame_view(&mut self, view: Rect) {
        let rects: Vec<Rect> =
            self.current().map(|a| a.pics.iter().chain(&a.behind).map(|p| p.src).collect()).unwrap_or_default();
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

    // ── Choosing pictures ───────────────────────────────────────

    /// The pictures chosen (the selected one at least).
    fn chosen(&self) -> Vec<usize> {
        let n = self.pics().len();
        let mut out: Vec<usize> = self.marked.iter().copied().filter(|&k| k < n).collect();
        if out.is_empty() {
            out.extend(self.selected.filter(|&k| k < n));
        }
        out
    }

    fn select_only(&mut self, k: Option<usize>) {
        self.selected = k;
        self.marked = k.into_iter().collect();
    }

    /// Choose `k` too, or not any more (Shift / Ctrl + click).
    fn toggle(&mut self, k: usize) {
        self.playing = false;
        if let Some(i) = self.marked.iter().position(|&m| m == k) {
            self.marked.remove(i);
            if self.selected == Some(k) {
                self.selected = self.marked.last().copied();
            }
        } else {
            self.marked.push(k);
            self.marked.sort_unstable();
            self.selected = Some(k);
        }
    }

    fn fix_selection(&mut self) {
        let n = self.pics().len();
        self.selected = self.selected.filter(|_| n > 0).map(|k| k.min(n - 1));
        self.marked.retain(|&k| k < n);
        if let Some(k) = self.selected {
            if !self.marked.contains(&k) {
                self.marked.push(k);
                self.marked.sort_unstable();
            }
        }
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

    /// New pictures after the selected one; they become the chosen ones.
    fn add(&mut self, new: Vec<Pic>) {
        if new.is_empty() {
            return;
        }
        let n = self.pics().len();
        let at = self.selected.map_or(n, |k| (k + 1).min(n));
        let count = new.len();
        self.pics_mut().splice(at..at, new);
        // The loop point and repeated runs keep to their pictures.
        if !self.behind {
            self.current_mut().reindex(n, |i| Some(if i < at { i } else { i + count }));
        }
        self.selected = Some(at + count - 1);
        self.marked = (at..at + count).collect();
        self.playing = false;
    }

    /// A picture for `r` as drawn: tight around its sprite when there is
    /// one, centred as the animation is.
    fn fitted(&self, r: Rect, grow: f32) -> Pic {
        find_sprite(&self.sheet.image, r, grow, self.feet()).unwrap_or_else(|| {
            let centre = match self.feet() {
                Some(hr) => vec2(r.w / 2.0, r.h - hr),
                None => r.size() / 2.0,
            };
            Pic::new(r, centre)
        })
    }

    fn set_pic(&mut self, k: usize, pic: Pic) {
        if let Some(p) = self.pics_mut().get_mut(k) {
            *p = pic;
        }
    }

    fn selected_pic(&self) -> Option<Pic> {
        self.selected.and_then(|k| self.pics().get(k).copied())
    }

    /// Change every chosen picture.
    fn each_chosen(&mut self, f: impl Fn(&Self, Pic) -> Pic) {
        let chosen = self.chosen();
        let new: Vec<(usize, Pic)> = chosen.iter().map(|&k| (k, f(self, self.pics()[k]))).collect();
        self.change(|ed| {
            for (k, p) in new {
                ed.set_pic(k, p);
            }
        });
    }

    /// Tighten the chosen pictures around their sprites, centred above the
    /// feet or in the middle.
    fn fit(&mut self, feet: bool) {
        if self.chosen().is_empty() {
            return;
        }
        let hr = body_radii(self.character, false).1 * self.sheet.scale;
        let mut missed = 0;
        let chosen = self.chosen();
        let fitted: Vec<(usize, Option<Pic>)> = chosen
            .iter()
            .map(|&k| {
                let p = self.pics()[k];
                let f = find_sprite(&self.sheet.image, p.src, 8.0, feet.then_some(hr));
                (k, f.map(|f| Pic { flip: p.flip, hold: p.hold, ..f }))
            })
            .collect();
        self.change(|ed| {
            for (k, f) in fitted {
                match f {
                    Some(f) => ed.set_pic(k, f),
                    None => missed += 1,
                }
            }
        });
        if missed > 0 {
            self.notes.push(Note::Warn(format!("No sprite in {}", plural(missed, "picture"))));
        }
    }

    fn nudge(&mut self, d: Vec2, how: Nudge) {
        let size = self.size();
        self.each_chosen(|_, p| match how {
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
        });
    }

    /// The chosen pictures take the selected one's place for the character
    /// (as far from their bottom and middle); with one picture chosen,
    /// the whole animation does.
    fn same_centre(&mut self) {
        let Some(p) = self.selected_pic() else { return };
        let off = p.centre - vec2(p.src.w / 2.0, p.src.h);
        let targets: Vec<usize> =
            if self.chosen().len() > 1 { self.chosen() } else { (0..self.pics().len()).collect() };
        self.change(|ed| {
            for k in targets {
                let q = ed.pics()[k];
                ed.set_pic(k, Pic { centre: vec2(q.src.w / 2.0, q.src.h) + off, ..q });
            }
        });
    }

    fn delete(&mut self) {
        let chosen = self.chosen();
        let Some(&first) = chosen.first() else { return };
        let n = self.pics().len();
        self.change(|ed| {
            for &k in chosen.iter().rev() {
                ed.pics_mut().remove(k);
            }
            // The loop point and repeated runs keep to their pictures (or
            // the ones after them).
            if !ed.behind {
                let gone = |i: usize| chosen.binary_search(&i).is_ok();
                ed.current_mut().reindex(n, |i| (!gone(i)).then(|| i - chosen.iter().filter(|&&k| k < i).count()));
            }
        });
        let n = self.pics().len();
        self.select_only((n > 0).then(|| first.min(n - 1)));
    }

    fn duplicate(&mut self) {
        let chosen = self.chosen();
        let Some(&last) = chosen.last() else { return };
        let copies: Vec<Pic> = chosen.iter().map(|&k| self.pics()[k]).collect();
        self.selected = Some(last);
        self.change(|ed| ed.add(copies));
    }

    fn copy(&mut self) {
        let chosen = self.chosen();
        if chosen.is_empty() {
            return;
        }
        self.clipboard = chosen.iter().map(|&k| self.pics()[k]).collect();
        self.notes.push(Note::Info(format!(
            "Copied {}: Ctrl+V pastes them in any animation",
            plural(chosen.len(), "picture")
        )));
    }

    fn paste(&mut self) {
        let pics = self.clipboard.clone();
        if pics.is_empty() {
            self.notes.push(Note::Info("Nothing copied yet: choose pictures, then Ctrl+C".into()));
            return;
        }
        self.change(|ed| ed.add(pics));
    }

    /// Move the selected picture earlier or later in the animation.
    fn shift(&mut self, by: isize) {
        let Some(k) = self.selected else { return };
        let to = k as isize + by;
        if to < 0 || to as usize >= self.pics().len() {
            return;
        }
        let to = to as usize;
        self.change(|ed| {
            ed.pics_mut().swap(k, to);
            // The loop point goes with its picture.
            if !ed.behind {
                let a = ed.current_mut();
                if a.loop_from > 0 && (a.loop_from == k || a.loop_from == to) {
                    a.loop_from = if a.loop_from == k { to } else { k };
                }
            }
        });
        self.select_only(Some(to));
    }

    /// Where the animation loops back to, when it opens with pictures
    /// before it (and loops: not played once).
    fn loop_point(&self) -> Option<usize> {
        let a = self.current()?;
        (!self.behind && !a.once && a.loop_from > 0 && a.loop_from < a.pics.len()).then_some(a.loop_from)
    }

    /// The chosen pictures, when they follow one another: (first, last).
    fn chosen_run(&self) -> Option<(usize, usize)> {
        let chosen = self.chosen();
        let (&a, &b) = (chosen.first()?, chosen.last()?);
        (b - a + 1 == chosen.len()).then_some((a, b))
    }

    /// How many times the chosen run plays: that of the run it is in, else once.
    fn run_times(&self) -> u32 {
        let Some((a, b)) = self.chosen_run() else { return 1 };
        let run = self.current().and_then(|an| an.repeat_of(a)).filter(|r| b <= r.to);
        run.map_or(1, |r| r.times)
    }

    /// Repeat the chosen run more or fewer times (once: not repeated). In a
    /// run already, that run changes; otherwise the chosen pictures become
    /// one, in place of any they cut across.
    fn step_repeat(&mut self, by: i32) {
        if self.behind {
            return;
        }
        let Some((a, b)) = self.chosen_run() else {
            self.notes.push(Note::Info("Choose pictures that follow one another (Shift-click the last)".into()));
            return;
        };
        let times = (self.run_times() as i32 + by).clamp(1, MOST_REPEATS as i32) as u32;
        self.change(|ed| {
            let runs = &mut ed.current_mut().repeats;
            if let Some(r) = runs.iter_mut().find(|r| r.from <= a && b <= r.to) {
                r.times = times;
            } else {
                runs.retain(|r| r.to < a || r.from > b);
                runs.push(Repeat { from: a, to: b, times });
            }
            runs.retain(|r| r.times >= 2);
            runs.sort_by_key(|r| r.from);
        });
        // From the start, to see it play.
        (self.clock, self.playing) = (0.0, true);
    }

    /// Choose the pictures from the selected one to `k` (Shift-click).
    fn select_run(&mut self, k: usize) {
        self.playing = false;
        let a = self.selected.unwrap_or(k);
        self.marked = (a.min(k)..=a.max(k)).collect();
        self.selected = Some(k);
    }

    /// Loop from the selected picture (again: the whole animation loops).
    fn loop_here(&mut self) {
        let Some(k) = self.selected.filter(|_| !self.behind) else { return };
        let to = if self.loop_point() == Some(k) { 0 } else { k };
        self.change(|ed| ed.current_mut().loop_from = to);
        // From the start, to see it open.
        (self.clock, self.playing) = (0.0, true);
    }

    fn step_picture(&mut self, by: isize) {
        let n = self.pics().len();
        if n == 0 {
            return;
        }
        self.playing = false;
        let k = self.selected.map_or(0, |k| (k as isize + by).rem_euclid(n as isize) as usize);
        self.select_only(Some(k));
    }

    fn reset_anim(&mut self) {
        let name = self.name().to_string();
        let was = self.sheet.defaults.get(&name).cloned().unwrap_or_default();
        self.change(|ed| {
            ed.sheet.anims.insert(name, was);
        });
        self.fix_selection();
        if self.selected.is_none() && !self.pics().is_empty() {
            self.select_only(Some(0));
        }
    }

    /// Slower or faster: the games' own pace, then each of [`PACES`].
    fn step_pace(&mut self, by: isize) {
        let now = self.current().and_then(|a| a.fps);
        let i = now.and_then(|f| PACES.iter().position(|&p| p >= f - 0.01)).map_or(-1, |i| i as isize);
        let to = (i + by).clamp(-1, PACES.len() as isize - 1);
        let fps = (to >= 0).then(|| PACES[to as usize]);
        self.change(|ed| ed.current_mut().fps = fps);
    }

    fn step_hold(&mut self, by: f32) {
        let Some(p) = self.selected_pic() else { return };
        let hold = (p.hold.round() + by).clamp(1.0, MOST_HOLD);
        self.each_chosen(|_, q| Pic { hold, ..q });
    }

    fn mirror(&mut self) {
        let Some(p) = self.selected_pic() else { return };
        self.each_chosen(|_, q| Pic { flip: !p.flip, ..q });
    }

    // ── The sheet ───────────────────────────────────────────────

    /// The sheet's colour at `p`, as it is (before any is cut out).
    fn colour_at(&self, p: Vec2) -> Option<[u8; 3]> {
        let (w, h) = self.sheet.original.dimensions();
        if p.x < 0.0 || p.y < 0.0 || p.x >= w as f32 || p.y >= h as f32 {
            return None;
        }
        let c = self.sheet.original.get_pixel(p.x as u32, p.y as u32);
        Some([c[0], c[1], c[2]])
    }

    fn pick(&mut self, p: Vec2) {
        self.picking = false;
        let Some(c) = self.colour_at(p) else { return };
        if self.sheet.layout_background.contains(&c) || self.sheet.background.contains(&c) {
            self.notes.push(Note::Info("That colour is already cut out".into()));
            return;
        }
        self.sheet.background.push(c);
        self.recolour();
        let found = self.sprites.len();
        self.notes.push(Note::Info(format!(
            "Background {} cut out: {} found",
            format_args!("({}, {}, {})", c[0], c[1], c[2]),
            plural(found, "sprite")
        )));
    }

    fn recolour(&mut self) {
        self.sheet.recolour();
        self.find_sprites();
        self.leaving = None;
    }

    fn step_scale(&mut self, by: isize) {
        let i = SCALES.iter().position(|&s| s >= self.sheet.scale - 0.01).unwrap_or(1) as isize;
        self.sheet.scale = SCALES[(i + by).clamp(0, SCALES.len() as isize - 1) as usize];
        self.leaving = None;
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
        let no_idle = self.sheet.anims.get("idle").is_none_or(|a| a.pics.is_empty());
        if no_idle && self.sheet.known {
            return Err("\"idle\" needs a picture at least".into());
        }
        let r = self.sheet.regions();
        r.save(self.character)?;
        self.saved = self.sheet.anims.clone();
        self.saved_settings = self.settings();
        self.leaving = None;
        let file = regions::file_name(self.character);
        Ok(if r.is_empty() {
            format!("{} sheet is back to its own layout", whose(self.character))
        } else if no_idle {
            format!("Saved {file}: the game will use it once \"idle\" has a picture")
        } else {
            format!("Saved {file} ({} changed)", plural(r.animations.len(), "animation"))
        })
    }

    // ── Layout ──────────────────────────────────────────────────

    fn layout(&self) -> Layout {
        let (top, view) = layout_parts();
        let sw = screen_width();
        let left = Rect::new(12.0, view.y, 180.0, view.h);
        let list = Rect::new(left.x, left.y, left.w, left.h - SHEET_BOX - 8.0);
        let sheet_box = Rect::new(left.x, left.bottom() - SHEET_BOX, left.w, SHEET_BOX);
        let side = Rect::new(sw - 284.0, view.y, 272.0, view.h);
        let mut buttons = vec![];
        let mut steppers = vec![];
        let mut push = |r: Rect, label: &str, cmd: Cmd, on: bool, enabled: bool| {
            buttons.push(Button { r, label: label.to_string(), cmd, on, enabled });
        };
        // The characters, after the title.
        let mut x = 16.0 + measure_bold("Sprite regions", 18.0) + 24.0;
        for c in Character::ALL {
            push(Rect::new(x, 10.0, 74.0, 32.0), c.short(), Cmd::Character(c), c == self.character, true);
            x += 80.0;
        }
        let mut x = top.w - 16.0;
        let dirty = self.dirty();
        for (w, label, cmd, on, enabled) in [
            (76.0, "Close", Cmd::Close, false, true),
            (76.0, "Save", Cmd::Save, dirty, true),
            (70.0, "Try ▶", Cmd::Try, false, true),
            (56.0, "Redo", Cmd::Redo, false, !self.redo.is_empty()),
            (56.0, "Undo", Cmd::Undo, false, !self.undo.is_empty()),
            (80.0, "Reset all", Cmd::ResetAll, false, differ(&self.sheet.anims, &self.sheet.defaults)),
        ] {
            x -= w;
            push(Rect::new(x, 10.0, w, 32.0), label, cmd, on, enabled);
            x -= 6.0;
        }
        // The sheet: background colours, closeness, scale, another picture.
        let b = sheet_box;
        let mut swatches = vec![];
        let mut sx = b.x + 12.0;
        let colours = self.sheet.layout_background.iter().map(|&c| (c, None));
        let picked = self.sheet.background.iter().enumerate().map(|(i, &c)| (c, Some(i)));
        for (c, i) in colours.chain(picked).take(6) {
            swatches.push((Rect::new(sx, b.y + 34.0, 16.0, 16.0), c, i));
            sx += 20.0;
        }
        push(Rect::new(b.right() - 62.0, b.y + 30.0, 50.0, 24.0), "Pick", Cmd::Pick, self.picking, true);
        push(Rect::new(b.x + 10.0, b.y + 62.0, b.w - 20.0, 26.0), "Soft edges", Cmd::Soft, self.sheet.soft, true);
        steppers.push(Stepper {
            r: Rect::new(b.x + 10.0, b.y + 94.0, b.w - 20.0, 26.0),
            label: "Scale",
            value: format!("×{}", num(self.sheet.scale)),
            less: Cmd::Smaller,
            more: Cmd::Bigger,
            enabled: true,
        });
        push(Rect::new(b.x + 10.0, b.y + 126.0, b.w - 20.0, 26.0), "Open a sheet…", Cmd::OpenSheet, false, true);
        // The side, from the bottom up; the preview gets what is left.
        let (w2, w3) = ((side.w - 6.0) / 2.0, (side.w - 12.0) / 3.0);
        let n = self.pics().len();
        let chosen = self.chosen();
        let any = !chosen.is_empty();
        let k = self.selected.unwrap_or(0);
        let mut y = side.bottom() - 28.0;
        let none = Anim::default();
        let changed = self.current().unwrap_or(&none) != self.sheet.defaults.get(self.name()).unwrap_or(&none);
        push(Rect::new(side.x, y, side.w, 28.0), "Reset this animation", Cmd::ResetAnim, false, changed);
        let rows = [
            [("Copy", Cmd::Copy, any), ("Paste", Cmd::Paste, !self.clipboard.is_empty()), ("Delete", Cmd::Delete, any)],
            [
                ("← Earlier", Cmd::Earlier, any && k > 0),
                ("Later →", Cmd::Later, any && k + 1 < n),
                ("Duplicate", Cmd::Duplicate, any),
            ],
            [
                ("Fit: feet", Cmd::FitFeet, any),
                ("Fit: middle", Cmd::FitMiddle, any),
                ("Same centre", Cmd::SameCentre, any && n > 1),
            ],
        ];
        for row in rows {
            y -= 34.0;
            for (c, (label, cmd, enabled)) in row.into_iter().enumerate() {
                push(Rect::new(side.x + c as f32 * (w3 + 6.0), y, w3, 28.0), label, cmd, false, enabled);
            }
        }
        y -= 34.0;
        let pic = self.selected_pic();
        steppers.push(Stepper {
            r: Rect::new(side.x, y, w3, 28.0),
            label: "Hold",
            value: format!("×{}", num(pic.map_or(1.0, |p| p.hold))),
            less: Cmd::HoldLess,
            more: Cmd::HoldMore,
            enabled: any,
        });
        let flipped = pic.is_some_and(|p| p.flip);
        push(Rect::new(side.x + w3 + 6.0, y, w3, 28.0), "Mirror", Cmd::Mirror, flipped, any);
        let looping = self.selected.is_some() && self.loop_point() == self.selected;
        let can_loop = chosen.len() == 1 && !self.behind && !self.current().is_some_and(|a| a.once);
        push(Rect::new(side.x + 2.0 * (w3 + 6.0), y, w3, 28.0), "Loop here", Cmd::LoopHere, looping, can_loop);
        y -= 34.0;
        let run = self.chosen_run().filter(|_| !self.behind);
        let times = self.run_times();
        // Within a run: the whole run.
        let inside = run.and_then(|(a, b)| self.current()?.repeat_of(a).filter(|r| b <= r.to));
        let run = inside.map(|r| (r.from, r.to)).or(run);
        steppers.push(Stepper {
            r: Rect::new(side.x, y, side.w, 28.0),
            label: "Repeat",
            value: match run {
                Some((a, b)) if a != b => format!("pictures {}–{}  ×{times}", a + 1, b + 1),
                Some((a, _)) => format!("picture {}  ×{times}", a + 1),
                None => "×1".to_string(),
            },
            less: Cmd::RepeatLess,
            more: Cmd::RepeatMore,
            enabled: run.is_some(),
        });
        y -= 58.0;
        let info = y;
        // Above: the preview, its controls, the layers, the pace and the
        // pictures.
        let mut t = side.y;
        let fixed = 34.0 + if self.layered() { 34.0 } else { 0.0 } + 34.0;
        let cols = ((side.w + THUMB_GAP) / (THUMB + THUMB_GAP)).floor().max(1.0) as usize;
        let room_rows = (((info - 4.0 - side.y - fixed - 110.0) / (THUMB + THUMB_GAP)).floor().max(1.0)) as usize;
        let thumb_rows = n.div_ceil(cols).clamp(1, room_rows);
        let preview_h =
            (info - 4.0 - side.y - fixed - thumb_rows as f32 * (THUMB + THUMB_GAP) - 8.0).clamp(110.0, 200.0);
        let preview = Rect::new(side.x, t, side.w, preview_h);
        t += preview_h + 8.0;
        let w4 = (side.w - 18.0) / 4.0;
        for (i, (label, cmd, on)) in [
            ("←", Cmd::Previous, false),
            (if self.playing { "Pause" } else { "Play" }, Cmd::Play, self.playing),
            ("→", Cmd::Next, false),
            ("Onion", Cmd::Onion, self.onion),
        ]
        .into_iter()
        .enumerate()
        {
            push(Rect::new(side.x + i as f32 * (w4 + 6.0), t, w4, 28.0), label, cmd, on, n > 0);
        }
        t += 34.0;
        if self.layered() {
            push(Rect::new(side.x, t, w2, 28.0), "Body", Cmd::Body, !self.behind, true);
            push(Rect::new(side.x + w2 + 6.0, t, w2, 28.0), "Behind", Cmd::Behind, self.behind, true);
            t += 34.0;
        }
        let anim = self.current();
        steppers.push(Stepper {
            r: Rect::new(side.x, t, w2, 28.0),
            label: "Pace",
            value: pace(anim.and_then(|a| a.fps)),
            less: Cmd::Slower,
            more: Cmd::Faster,
            enabled: true,
        });
        push(Rect::new(side.x + w2 + 6.0, t, w2, 28.0), "Play once", Cmd::Once, anim.is_some_and(|a| a.once), true);
        t += 34.0;
        let shown = (thumb_rows * cols).min(n);
        let thumbs: Vec<Rect> = (0..shown)
            .map(|k| {
                let (c, r) = ((k % cols) as f32, (k / cols) as f32);
                Rect::new(side.x + c * (THUMB + THUMB_GAP), t + r * (THUMB + THUMB_GAP), THUMB, THUMB)
            })
            .collect();
        Layout { list, sheet_box, view, side, preview, thumbs, unshown: n - shown, info, buttons, steppers, swatches }
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
        if ask.is_none() && self.drag.is_none() && !std::mem::take(&mut self.fresh) {
            ask = self.keys(input, dt, &l);
        }
        self.mouse(input, l.view);
        self.draw(&self.layout(), input);
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
            return if b.enabled { self.command(b.cmd) } else { None };
        }
        if let Some(s) = l.steppers.iter().find(|s| s.r.contains(m)) {
            input.consumed = true;
            let cmd = if m.x < s.r.x + s.r.w / 3.0 { s.less } else { s.more };
            return if s.enabled { self.command(cmd) } else { None };
        }
        if let Some(&(_, _, i)) = l.swatches.iter().find(|s| s.0.contains(m)) {
            input.consumed = true;
            match i {
                Some(i) => return self.command(Cmd::Unpick(i)),
                None => self.notes.push(Note::Info("The sheet's own background (it stays cut out)".into())),
            }
            return None;
        }
        if let Some(k) = l.thumbs.iter().position(|t| t.contains(m)) {
            input.consumed = true;
            if input.shift {
                self.select_run(k);
            } else if input.ctrl {
                self.toggle(k);
            } else {
                self.select_only(Some(k));
                self.playing = false;
            }
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
        if l.side.contains(m) || l.sheet_box.contains(m) {
            input.consumed = true;
        }
        None
    }

    fn command(&mut self, cmd: Cmd) -> Option<Ask> {
        match cmd {
            Cmd::Character(c) if c == self.character => {}
            Cmd::Character(c) if self.available.contains(&c) => return self.leave(Ask::Open(c)),
            Cmd::Character(c) => return self.leave(Ask::OpenSheet(c)),
            Cmd::OpenSheet => return self.leave(Ask::OpenSheet(self.character)),
            Cmd::Try => return Some(Ask::Try),
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
                if self.playing {
                    // From the start: its opening shows again.
                    self.clock = 0.0;
                } else {
                    // Stop on the picture showing.
                    let k = self.shown().or(self.selected);
                    self.select_only(k);
                }
            }
            Cmd::Onion => self.onion = !self.onion,
            Cmd::Body | Cmd::Behind => {
                self.behind = cmd == Cmd::Behind;
                self.select_only((!self.pics().is_empty()).then_some(0));
            }
            Cmd::Slower => self.step_pace(-1),
            Cmd::Faster => self.step_pace(1),
            Cmd::Once => self.change(|ed| {
                let a = ed.current_mut();
                a.once = !a.once;
            }),
            Cmd::HoldLess => self.step_hold(-1.0),
            Cmd::HoldMore => self.step_hold(1.0),
            Cmd::Mirror => self.mirror(),
            Cmd::LoopHere => self.loop_here(),
            Cmd::RepeatLess => self.step_repeat(-1),
            Cmd::RepeatMore => self.step_repeat(1),
            Cmd::FitFeet => self.fit(true),
            Cmd::FitMiddle => self.fit(false),
            Cmd::SameCentre => self.same_centre(),
            Cmd::Earlier => self.shift(-1),
            Cmd::Later => self.shift(1),
            Cmd::Duplicate => self.duplicate(),
            Cmd::Copy => self.copy(),
            Cmd::Paste => self.paste(),
            Cmd::Delete => self.delete(),
            Cmd::Pick => {
                self.picking = !self.picking;
                if self.picking {
                    self.notes.push(Note::Info("Click the sheet's background colour (Esc: never mind)".into()));
                }
            }
            Cmd::Unpick(i) => {
                if i < self.sheet.background.len() {
                    self.sheet.background.remove(i);
                    self.recolour();
                }
            }
            Cmd::Soft => {
                self.sheet.soft = !self.sheet.soft;
                self.recolour();
            }
            Cmd::Smaller => self.step_scale(-1),
            Cmd::Bigger => self.step_scale(1),
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
            if self.picking {
                self.picking = false;
                return None;
            }
            return self.leave(Ask::Close);
        }
        if ctrl {
            if pressed(KeyCode::S) {
                return Some(Ask::Save);
            }
            if pressed(KeyCode::Z) {
                if shift {
                    self.redo();
                } else {
                    self.undo();
                }
            }
            if pressed(KeyCode::Y) {
                self.redo();
            }
            if pressed(KeyCode::A) {
                let n = self.pics().len();
                self.marked = (0..n).collect();
                self.selected = self.selected.or((n > 0).then_some(0));
            }
            if pressed(KeyCode::C) {
                self.copy();
            }
            if pressed(KeyCode::V) {
                self.paste();
            }
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
        let commands = [
            (KeyCode::Delete, Cmd::Delete),
            (KeyCode::Backspace, Cmd::Delete),
            (KeyCode::D, Cmd::Duplicate),
            (KeyCode::F, Cmd::FitFeet),
            (KeyCode::M, Cmd::FitMiddle),
            (KeyCode::C, Cmd::SameCentre),
            (KeyCode::H, Cmd::Mirror),
            (KeyCode::L, Cmd::LoopHere),
            (KeyCode::Equal, Cmd::HoldMore),
            (KeyCode::Minus, Cmd::HoldLess),
            (KeyCode::Space, Cmd::Play),
            (KeyCode::O, Cmd::Onion),
            (KeyCode::I, Cmd::Pick),
            (KeyCode::LeftBracket, Cmd::Previous),
            (KeyCode::Comma, Cmd::Previous),
            (KeyCode::RightBracket, Cmd::Next),
            (KeyCode::Period, Cmd::Next),
        ];
        for (key, cmd) in commands {
            if pressed(key) {
                self.command(cmd);
            }
        }
        if pressed(KeyCode::T) {
            return Some(Ask::Try);
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

    /// The pointer pressed on the sheet: what it starts (none: it chose a
    /// picture, or picked a colour).
    fn grab(&mut self, m: Vec2, p: Vec2, input: &Input) -> Option<Drag> {
        if self.picking {
            self.pick(p);
            return None;
        }
        let under = self.pics().iter().rposition(|q| q.src.contains(p));
        if input.shift || input.ctrl {
            return match under {
                Some(k) if input.shift => {
                    self.select_run(k);
                    None
                }
                Some(k) => {
                    self.toggle(k);
                    None
                }
                None if input.shift => Some(Drag::Gather(p)),
                None => None,
            };
        }
        if let Some(pic) = self.selected_pic() {
            match self.handle_at(m) {
                Some(Handle::Centre) => return Some(Drag::Centre(p, pic)),
                Some(Handle::Edge(ex, ey)) if self.chosen().len() <= 1 => {
                    return Some(Drag::Resize(p, pic, (ex, ey)));
                }
                _ => {}
            }
        }
        let Some(k) = under else { return Some(Drag::New(p)) };
        // Pressing one of the chosen moves them all; another is chosen alone.
        if !self.chosen().contains(&k) {
            self.select_only(Some(k));
        } else {
            self.selected = Some(k);
        }
        self.playing = false;
        self.grabbed = self.chosen().into_iter().map(|k| (k, self.pics()[k])).collect();
        Some(Drag::Move(p))
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

    /// Add every sprite found inside `r`, in reading order.
    fn gather(&mut self, r: Rect) {
        let inside: Vec<Rect> =
            self.sprites.iter().copied().filter(|s| r.contains(s.point() + s.size() / 2.0)).collect();
        if inside.is_empty() {
            self.notes.push(Note::Info("No sprites in that box".into()));
            return;
        }
        let pics: Vec<Pic> = reading_order(inside).into_iter().map(|s| self.fitted(s, 0.0)).collect();
        let n = pics.len();
        self.add(pics);
        self.notes.push(Note::Info(format!("Added {}", plural(n, "picture"))));
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
            self.drag = self.grab(m, p, input);
        }
        match self.drag {
            Some(Drag::Move(from)) => {
                let d = (p - from).round();
                let grabbed = std::mem::take(&mut self.grabbed);
                for &(k, was) in &grabbed {
                    let room = size - was.src.size();
                    let at = vec2(within(was.src.x + d.x, 0.0, room.x), within(was.src.y + d.y, 0.0, room.y));
                    self.set_pic(k, Pic { src: Rect::new(at.x, at.y, was.src.w, was.src.h), ..was });
                }
                self.grabbed = grabbed;
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
                if let Some(k) = self.selected {
                    self.set_pic(k, Pic { src: Rect::new(x0, y0, x1 - x0, y1 - y0), centre, ..was });
                }
            }
            Some(Drag::Centre(from, was)) => {
                let centre = ((was.centre + p - from) * 2.0).round() / 2.0;
                if let Some(k) = self.selected {
                    self.set_pic(k, Pic { centre, ..was });
                }
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
                        self.select_only(Some(k));
                        self.playing = false;
                    }
                }
            }
            return;
        }
        if !input.left_released {
            return;
        }
        match self.drag {
            Some(Drag::New(from)) => {
                let r = rect_between(from, p, size);
                if m.distance(self.to_screen(from)) > 4.0 && r.w >= 2.0 && r.h >= 2.0 {
                    let pic = self.fitted(r, 0.0);
                    self.add(vec![pic]);
                } else if get_time() - self.last_click.0 < 0.4 && m.distance(self.last_click.1) < 6.0 {
                    // A double click: the sprite under it (or touching the pointer).
                    let reach = (6.0 / self.zoom).max(1.0).ceil();
                    let dot = Rect::new(p.x.floor() - reach, p.y.floor() - reach, 2.0 * reach + 1.0, 2.0 * reach + 1.0);
                    match find_sprite(&self.sheet.image, dot, 64.0, self.feet()) {
                        Some(pic) => self.add(vec![pic]),
                        None => self.notes.push(Note::Info("No sprite there: double-click on one".into())),
                    }
                    self.last_click = (0.0, m);
                } else {
                    self.select_only(None);
                    self.last_click = (get_time(), m);
                }
            }
            Some(Drag::Gather(from)) => {
                let r = rect_between(from, p, size);
                self.gather(r);
            }
            _ => {}
        }
        self.drag = None;
        self.grabbed.clear();
        if let Some(before) = self.before.take() {
            self.keep(before);
        }
    }

    /// The picture the preview shows of the layer being edited.
    fn shown(&self) -> Option<usize> {
        let a = self.current()?;
        let n = self.pics().len();
        if n == 0 {
            return None;
        }
        if !self.playing {
            return self.selected.map(|k| k.min(n - 1));
        }
        if self.behind {
            return Some((self.clock * BEHIND_FPS) as usize % n);
        }
        let t = self.clock * a.fps.unwrap_or(PREVIEW_FPS);
        if a.once {
            // Through once, the last picture held a second, and again.
            let rest = a.fps.unwrap_or(PREVIEW_FPS);
            Some(a.at(t.rem_euclid(a.length() + rest), true))
        } else {
            Some(a.at(t, false))
        }
    }

    // ── Drawing ─────────────────────────────────────────────────

    fn draw(&self, l: &Layout, input: &Input) {
        let m = input.mouse;
        clear_background(Color::new(0.045, 0.04, 0.085, 1.0));
        self.draw_view(l.view, input);
        self.draw_list(l.list, m);
        self.draw_sheet_box(l);
        self.draw_preview(l.preview);
        self.draw_thumbs(l, m);
        self.draw_info(l);
        self.draw_top();
        for b in &l.buttons {
            draw_btn(b, m);
        }
        for s in &l.steppers {
            draw_stepper(s, m);
        }
        self.draw_help();
    }

    fn draw_top(&self) {
        text_bold("Sprite regions", 16.0, baseline(22.0, 18.0), 18.0, TEXT);
        let (w, h) = self.sheet.image.dimensions();
        let mark = if self.dirty() { "  ·  unsaved" } else { "" };
        let new = if self.sheet.known { "" } else { "  ·  new to the game" };
        let line = format!("{}  ·  {w} × {h}{new}{mark}", self.sheet.name);
        text(&line, 16.0, baseline(41.0, 11.0), 11.0, TEXT_MUTED);
    }

    fn draw_help(&self) {
        let parts = [
            "Drag on the sheet: new picture",
            "Double-click a sprite: add it",
            "Shift-drag: add every sprite in the box",
            "Shift-click: a run  ·  Ctrl-click: one more",
            "Drag a box, its edges, its cross",
            "Arrows move (Shift ×10, Ctrl size, Alt centre)",
            "Wheel zoom",
            "Right-drag pan",
            "T: try it",
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
            let unsaved = now != self.saved.get(name).unwrap_or(&none);
            if unsaved || now != self.sheet.defaults.get(name).unwrap_or(&none) {
                draw_circle(r.right() - 16.0 - cw, r.y + r.h / 2.0, 3.0, if unsaved { WARNING } else { ACCENT });
            }
        }
        if self.scroll + self.list_rows(list) < self.names.len() {
            text_centered("…", list.x + list.w / 2.0, list.bottom() - 8.0, 13.0, TEXT_MUTED);
        }
    }

    fn draw_sheet_box(&self, l: &Layout) {
        let b = l.sheet_box;
        rrect(b, RADIUS, SURFACE);
        rrect_lines(b, RADIUS, 1.0, BORDER);
        text_bold("SHEET", b.x + 12.0, b.y + 20.0, 11.0, TEXT_MUTED);
        text("background", b.x + 56.0, b.y + 20.0, 11.0, TEXT_MUTED);
        if l.swatches.is_empty() {
            text("none yet", b.x + 12.0, baseline(b.y + 42.0, 12.0), 12.0, TEXT_MUTED);
        }
        for &(r, c, picked) in &l.swatches {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(c[0], c[1], c[2], 255));
            let edge = if picked.is_some() { BORDER_HI } else { BORDER };
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, edge);
        }
        let more = self.sheet.layout_background.len() + self.sheet.background.len();
        if more > l.swatches.len() {
            let x = l.swatches.last().map_or(b.x + 12.0, |s| s.0.right() + 4.0);
            text(&format!("+{}", more - l.swatches.len()), x, baseline(b.y + 42.0, 11.0), 11.0, TEXT_MUTED);
        }
    }

    fn draw_view(&self, v: Rect, input: &Input) {
        let m = input.mouse;
        rrect(v, RADIUS, Color::new(0.03, 0.028, 0.06, 1.0));
        clip(Some(v));
        let size = self.size();
        let sr = self.screen_rect(Rect::new(0.0, 0.0, size.x, size.y));
        // A checkerboard shows through what is cut out.
        draw_rectangle(sr.x, sr.y, sr.w, sr.h, Color::new(0.30, 0.30, 0.34, 1.0));
        let shown = Rect::new(sr.x.max(v.x), sr.y.max(v.y), 0.0, 0.0);
        let (x1, y1) = (sr.right().min(v.right()), sr.bottom().min(v.bottom()));
        const SQUARE: f32 = 12.0;
        let dark = Color::new(0.22, 0.22, 0.26, 1.0);
        let (mut y, mut row) = (shown.y, ((shown.y - sr.y) / SQUARE).floor() as i32);
        while y < y1 {
            let h = (sr.y + (row + 1) as f32 * SQUARE).min(y1) - y;
            let mut col = ((shown.x - sr.x) / SQUARE).floor() as i32;
            let mut x = shown.x;
            while x < x1 {
                let w = (sr.x + (col + 1) as f32 * SQUARE).min(x1) - x;
                if (row + col) % 2 == 0 {
                    draw_rectangle(x, y, w, h, dark);
                }
                x += w;
                col += 1;
            }
            y += h;
            row += 1;
        }
        let texture = &self.sheet.texture;
        draw_texture_ex(
            texture,
            sr.x,
            sr.y,
            WHITE,
            DrawTextureParams { dest_size: Some(sr.size()), ..Default::default() },
        );
        // Pixels, close up.
        if self.zoom >= 8.0 {
            let grid = alpha(WHITE, 0.06);
            let a = self.to_sheet(v.point()).max(Vec2::ZERO).floor();
            let b = self.to_sheet(v.point() + v.size()).min(size).ceil();
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
        // The sprites found, when Shift is held (Shift-drag adds them).
        if input.shift && !self.picking {
            for s in &self.sprites {
                let r = self.screen_rect(*s);
                if r.overlaps(&v) {
                    let hot = matches!(self.drag, Some(Drag::Gather(from)) if rect_between(from, self.to_sheet(m), size).contains(s.point() + s.size() / 2.0));
                    dashed_rect(r, alpha(if hot { SUCCESS } else { TEXT_DIM }, if hot { 0.95 } else { 0.5 }));
                }
            }
        }
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
        if let Some(a) = self.current() {
            let other = if self.behind { &a.pics } else { &a.behind };
            for q in other {
                let r = self.screen_rect(q.src);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, alpha(WARNING, 0.55));
            }
        }
        // Its pictures, numbered (chosen ones lit).
        let chosen = self.chosen();
        for (k, q) in self.pics().iter().enumerate() {
            let r = self.screen_rect(q.src);
            let sel = Some(k) == self.selected;
            let lit = chosen.contains(&k);
            if lit {
                draw_rectangle(r.x, r.y, r.w, r.h, alpha(ACCENT, if sel { 0.14 } else { 0.1 }));
            }
            let c = if lit { ACCENT_HI } else { ACCENT };
            draw_rectangle_lines(r.x, r.y, r.w, r.h, if lit { 2.0 } else { 1.5 }, c);
            let mut label = (k + 1).to_string();
            if q.hold != 1.0 {
                label += &format!(" ×{}", num(q.hold));
            }
            if q.flip {
                label += " ↔";
            }
            if self.loop_point() == Some(k) {
                label += " ↻";
            }
            if let Some(r) = self.current().and_then(|a| a.repeat_of(k)).filter(|r| r.from == k && !self.behind) {
                label += &format!(" ↻×{}", r.times);
            }
            let lw = measure_bold(&label, 11.0) + 8.0;
            let tag = Rect::new(r.x, r.y - 15.0, lw, 15.0);
            draw_rectangle(tag.x, tag.y, tag.w, tag.h, c);
            text_bold(&label, tag.x + 4.0, baseline(tag.y + 7.5, 11.0), 11.0, Color::new(0.05, 0.04, 0.1, 1.0));
        }
        if let Some(q) = self.selected_pic() {
            self.draw_selected(q, m);
        }
        match self.drag {
            Some(Drag::New(from)) => {
                dashed_rect(self.screen_rect(rect_between(from, self.to_sheet(m), size)), ACCENT_HI)
            }
            Some(Drag::Gather(from)) => {
                dashed_rect(self.screen_rect(rect_between(from, self.to_sheet(m), size)), SUCCESS)
            }
            _ => {}
        }
        clip(None);
        rrect_lines(v, RADIUS, 1.0, BORDER);
        // What this animation is.
        let (name, what) = &self.names[self.anim];
        let n = self.pics().len();
        let layer = if self.behind { "  ·  behind" } else { "" };
        let caption = if self.picking {
            "Click the sheet's background colour to cut it out  ·  Esc: never mind".to_string()
        } else {
            format!("{name}  —  {what}  ·  {}{layer}", plural(n, "picture"))
        };
        let bar = Rect::new(v.x + 1.0, v.y + 1.0, v.w - 2.0, 28.0);
        draw_rectangle(bar.x, bar.y, bar.w, bar.h, alpha(SURFACE, 0.92));
        text_left(bar, 12.0, &caption, 13.0, if self.picking { WARNING } else { TEXT });
        let z = format!("×{}", self.zoom);
        text(&z, bar.right() - 12.0 - measure(&z, 12.0), baseline(bar.y + 14.0, 12.0), 12.0, TEXT_MUTED);
        // A sheet with nothing cut out yet.
        if !self.clear && !self.picking {
            let tip = "Nothing is see-through on this sheet yet: Pick (left, under SHEET), then click its background";
            let tw = measure(tip, 12.0) + 20.0;
            let r = Rect::new(v.x + (v.w - tw) / 2.0, v.bottom() - 40.0, tw, 26.0);
            rrect(r, 8.0, alpha(WARNING, 0.9));
            text_left(r, 10.0, tip, 12.0, Color::new(0.1, 0.07, 0.0, 1.0));
        }
        // The colour under the eyedropper.
        if self.picking && v.contains(m) {
            if let Some(c) = self.colour_at(self.to_sheet(m)) {
                let r = Rect::new(m.x + 16.0, m.y + 16.0, 30.0, 30.0);
                draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(c[0], c[1], c[2], 255));
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, WHITE);
                let s = format!("{}, {}, {}", c[0], c[1], c[2]);
                let t = Rect::new(r.right() + 4.0, r.y + 4.0, measure(&s, 11.0) + 12.0, 22.0);
                rrect(t, 6.0, SURFACE_HI);
                text_left(t, 6.0, &s, 11.0, TEXT);
            }
            draw_line(m.x - 9.0, m.y, m.x + 9.0, m.y, 1.0, WHITE);
            draw_line(m.x, m.y - 9.0, m.x, m.y + 9.0, 1.0, WHITE);
        }
        // Which animation a faint picture is.
        if self.drag.is_none() && !self.picking && v.contains(m) && self.handle_at(m).is_none() {
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
        if self.chosen().len() <= 1 {
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
        let Some(a) = self.current().filter(|a| !a.pics.is_empty() || !a.behind.is_empty()) else {
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
        // The layer edited follows the controls; the other plays.
        let shown = self.shown();
        let kb = if self.behind {
            shown
        } else {
            (!a.behind.is_empty()).then(|| (self.clock * BEHIND_FPS) as usize % a.behind.len())
        };
        let kp = if self.behind {
            (!a.pics.is_empty()).then(|| a.at(self.clock * a.fps.unwrap_or(PREVIEW_FPS), false))
        } else {
            shown
        };
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
        let pace = match a.fps {
            Some(f) => format!("{} pictures a second", num(f)),
            None => "the games' pace".to_string(),
        };
        let how = match self.loop_point() {
            _ if a.once => "  ·  once".to_string(),
            Some(k) => format!("  ·  loops from {}", k + 1),
            None => String::new(),
        };
        text(&format!("×{z}  ·  {pace}{how}"), pv.x + 10.0, baseline(pv.y + 14.0, 11.0), 11.0, TEXT_MUTED);
        if let Some(k) = k {
            let s = format!("{} / {}", k + 1, list.len());
            text(&s, pv.right() - 10.0 - measure(&s, 11.0), baseline(pv.bottom() - 12.0, 11.0), 11.0, TEXT_MUTED);
        }
    }

    fn draw_thumbs(&self, l: &Layout, m: Vec2) {
        let shown = if self.playing { self.shown() } else { None };
        let chosen = self.chosen();
        for (k, (t, q)) in l.thumbs.iter().zip(self.pics()).enumerate() {
            let sel = Some(k) == self.selected;
            let lit = chosen.contains(&k);
            rrect(*t, 6.0, if t.contains(m) { SURFACE_HI } else { SURFACE_2 });
            let z = ((t.w - 8.0) / q.src.w).min((t.h - 8.0) / q.src.h);
            let z = if z >= 1.0 { z.floor() } else { z };
            let at = vec2(t.x + (t.w - q.src.w * z) / 2.0, t.y + (t.h - q.src.h * z) / 2.0).round();
            draw_texture_ex(
                &self.sheet.texture,
                at.x,
                at.y,
                WHITE,
                DrawTextureParams {
                    source: Some(q.src),
                    dest_size: Some(q.src.size() * z),
                    flip_x: q.flip,
                    ..Default::default()
                },
            );
            let border = if sel {
                ACCENT_HI
            } else if lit {
                ACCENT
            } else if shown == Some(k) {
                SUCCESS
            } else {
                BORDER
            };
            rrect_lines(*t, 6.0, if lit { 2.0 } else { 1.0 }, border);
            text_bold(&(k + 1).to_string(), t.x + 4.0, t.y + 12.0, 10.0, TEXT_DIM);
            // Repeated runs, along their tops.
            if let Some(r) = self.current().and_then(|a| a.repeat_of(k)).filter(|_| !self.behind) {
                draw_rectangle(t.x + 6.0, t.y + 1.0, t.w - 12.0, 2.0, alpha(ACCENT_HI, 0.9));
                if r.from == k {
                    let s = format!("×{}", r.times);
                    text_bold(&s, t.x + 18.0, t.y + 12.0, 10.0, ACCENT_HI);
                }
            }
            // The opening, played once, and the loop after it.
            if let Some(from) = self.loop_point() {
                let c = if k < from { alpha(WARNING, 0.75) } else { alpha(SUCCESS, 0.85) };
                draw_rectangle(t.x + 6.0, t.bottom() - 3.0, t.w - 12.0, 2.0, c);
                if k == from {
                    text_bold("↻", t.right() - 4.0 - measure_bold("↻", 11.0), t.y + 12.0, 11.0, SUCCESS);
                }
            }
            if q.hold != 1.0 {
                let s = format!("×{}", num(q.hold));
                text_bold(&s, t.right() - 4.0 - measure_bold(&s, 10.0), t.bottom() - 4.0, 10.0, WARNING);
            }
        }
        if l.unshown > 0 {
            let last = l.thumbs.last().copied().unwrap_or(l.preview);
            text(&format!("+{} more", l.unshown), l.side.x, last.bottom() + 10.0, 11.0, TEXT_MUTED);
        }
        if l.thumbs.is_empty() {
            let y = l.preview.bottom() + 8.0 + 34.0 + if self.layered() { 34.0 } else { 0.0 } + 34.0;
            text("No pictures in this animation", l.side.x + 4.0, baseline(y + THUMB / 2.0, 12.0), 12.0, TEXT_MUTED);
        }
    }

    fn draw_info(&self, l: &Layout) {
        let x = l.side.x + 4.0;
        let line = |k: f32, s: &str, c: Color| text(s, x, baseline(l.info + 9.0 + k * 17.0, 12.0), 12.0, c);
        let chosen = self.chosen();
        match (self.selected, self.selected_pic()) {
            (Some(_), Some(_)) if chosen.len() > 1 => {
                line(0.0, &format!("{} chosen", plural(chosen.len(), "picture")), TEXT);
                line(1.0, "Arrows, Fit, Hold, Mirror and Delete", TEXT_DIM);
                line(2.0, "act on them all", TEXT_DIM);
            }
            (Some(k), Some(q)) => {
                let mut head = format!("Picture {} of {}", k + 1, self.pics().len());
                if q.hold != 1.0 {
                    head += &format!("  ·  held ×{}", num(q.hold));
                }
                if q.flip {
                    head += "  ·  mirrored";
                }
                if self.loop_point() == Some(k) {
                    head += "  ·  loops from here";
                }
                if let Some(r) = self.current().and_then(|a| a.repeat_of(k)).filter(|_| !self.behind) {
                    head += &format!("  ·  run ×{}", r.times);
                }
                line(0.0, &head, TEXT);
                let r = q.src;
                line(1.0, &format!("x {}   y {}   ·   {} × {}", num(r.x), num(r.y), num(r.w), num(r.h)), TEXT_DIM);
                let (cx, cy) = (num(q.centre.x), num(q.centre.y));
                line(2.0, &format!("Centre {cx}, {cy}   (from its top-left)"), TEXT_DIM);
            }
            _ => {
                line(0.0, "No picture selected", TEXT_DIM);
                line(1.0, "Click one on the sheet or above", TEXT_MUTED);
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

fn draw_stepper(s: &Stepper, m: Vec2) {
    let hovered = s.enabled && s.r.contains(m);
    rrect(
        s.r,
        7.0,
        if hovered {
            SURFACE_HI
        } else if s.enabled {
            SURFACE_2
        } else {
            alpha(SURFACE_2, 0.5)
        },
    );
    rrect_lines(s.r, 7.0, 1.0, BORDER);
    let cy = s.r.y + s.r.h / 2.0;
    let arrow = if s.enabled { TEXT_DIM } else { alpha(TEXT_MUTED, 0.5) };
    for (x, dir) in [(s.r.x + 12.0, -1.0f32), (s.r.right() - 12.0, 1.0)] {
        draw_triangle(vec2(x + 4.0 * dir, cy), vec2(x - 3.0 * dir, cy - 5.0), vec2(x - 3.0 * dir, cy + 5.0), arrow);
    }
    let (lw, vw) = (measure(s.label, 12.0), measure(&s.value, 13.0));
    let x0 = s.r.x + (s.r.w - lw - 6.0 - vw) / 2.0;
    text(s.label, x0, baseline(cy, 12.0), 12.0, TEXT_MUTED);
    text(&s.value, x0 + lw + 6.0, baseline(cy, 13.0), 13.0, if s.enabled { TEXT } else { TEXT_MUTED });
}

/// The top bar and the sheet view (the rest is laid out around them).
fn layout_parts() -> (Rect, Rect) {
    let (sw, sh) = (screen_width(), screen_height());
    let (y, bottom) = (60.0, sh - 34.0);
    let top = Rect::new(0.0, 0.0, sw, 52.0);
    let view = Rect::new(204.0, y, (sw - 284.0 - 12.0 - 204.0).max(100.0), bottom - y);
    (top, view)
}

/// What to come back to after trying the sprites.
pub(super) struct RegionTest {
    scene: scene::SceneFile,
    world_size: u8,
    character: Character,
    camera: (Vec2, f32),
    act: Option<usize>,
    level: Option<scene::SceneFile>,
}

impl App {
    /// Open the sprite region editor on `c`'s sheet (else the character
    /// played, else the first with a sheet; with none, ask for one).
    pub(super) fn open_regions(&mut self, c: Option<Character>) {
        let available: Vec<Character> = Character::ALL.into_iter().filter(|&c| sheet::has_sheet(c)).collect();
        let wanted = c.unwrap_or(self.s.character);
        let c = if available.contains(&wanted) {
            wanted
        } else if let (None, Some(&first)) = (c, available.first()) {
            first
        } else {
            self.choose_sheet(wanted);
            return;
        };
        match Editable::open(c) {
            Ok(Some(sheet)) => self.regions = Some(RegionEditor::new(c, available, sheet)),
            Ok(None) => self.toasts.warn(format!("{} has no sprite sheet", c.label())),
            Err(e) => self.toasts.error(format!("{} sheet: {e}", whose(c))),
        }
    }

    /// Choose any picture to be `c`'s sheet, then edit it.
    fn choose_sheet(&mut self, c: Character) {
        let types = ["png", "PNG", "webp", "WEBP", "gif", "GIF", "bmp", "BMP", "jpg", "JPG", "jpeg", "JPEG"];
        let Some(path) = FileDialog::new().add_filter("Sprite sheet", &types).pick_file() else { return };
        match sheet::Sheet::install_for(&path, c) {
            Ok(name) => {
                self.toasts.success(format!("{} now uses {name}", c.label()));
                self.sonic_sheet_tried = true;
                self.load_sonic_sheet();
                self.open_regions(Some(c));
            }
            Err(e) => self.toasts.error(format!("Could not use that picture: {e}")),
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
            Some(Ask::OpenSheet(c)) => self.choose_sheet(c),
            Some(Ask::Try) => self.try_regions(),
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

    /// Play the character as edited (saved or not) on the test track; Esc
    /// comes back to the editor, and to the scene as it was.
    fn try_regions(&mut self) {
        let Some(ed) = self.regions.as_ref() else { return };
        if ed.sheet.anims.get("idle").is_none_or(|a| a.pics.is_empty()) {
            self.toasts.warn("Give \"idle\" a picture first");
            return;
        }
        let (c, edited) = (ed.character, ed.sheet.sheet());
        let water =
            self.s.water.then_some(scene::SceneWater { level: self.s.water_level, density: self.s.water_density });
        let mut sc = scene::capture(&self.world, &self.objects, &self.links, &self.zones, water);
        sc.gadgets = scene::capture_gadgets(&self.objects, &self.gadgets);
        sc.night = Some(self.s.night);
        sc.weather = Some(self.s.weather);
        sc.world_size = Some(self.s.world_size);
        sc.sonic =
            self.sonic.as_ref().filter(|_| self.game.is_none()).map(|r| to_phys(r.sonic.pos.x, r.sonic.pos.y).into());
        self.region_test = Some(RegionTest {
            scene: sc,
            world_size: self.s.world_size,
            character: self.s.character,
            camera: (self.camera.center, self.camera.zoom),
            act: self.act,
            level: self.level.clone(),
        });
        // Every sheet as saved, but this one as edited.
        if !self.sonic_sheet_tried {
            self.sonic_sheet_tried = true;
            self.load_sonic_sheet();
        }
        self.sonic_sheet.set(c, edited);
        self.s.character = c;
        self.challenge = None;
        if !self.infinite() {
            self.set_world_size(4);
        }
        let (aw, ah) = self.arena();
        self.apply_scene(acts::TEST_TRACK.scene(aw / 2.0 - 600.0, ah));
        self.camera.zoom = 1.0;
        self.paused = false;
        self.library.fader.open = false;
        self.toasts.info(format!("Trying {} sprites as edited  ·  Esc: back to the editor", whose(c)));
    }

    /// Esc while trying: back to the editor and the scene as it was.
    pub(super) fn end_region_test(&mut self) {
        let Some(t) = self.region_test.take() else { return };
        self.stop_game();
        self.sonic = None;
        self.s.character = t.character;
        if self.s.world_size != t.world_size {
            self.set_world_size(t.world_size);
        }
        match (t.act, t.level) {
            (Some(i), _) => self.open_act(i),
            (None, Some(level)) => {
                self.level_world(&level);
                self.apply_scene(level);
            }
            (None, None) => {
                self.apply_scene(t.scene);
                (self.camera.center, self.camera.zoom) = t.camera;
            }
        }
        self.paused = false;
        // The sheets as saved again.
        self.load_sonic_sheet();
        if let Some(ed) = self.regions.as_mut() {
            ed.fresh = true;
        }
    }

    /// While trying: how to come back.
    pub(super) fn draw_region_test(&self) {
        if self.region_test.is_none() {
            return;
        }
        let c = self.regions.as_ref().map_or(self.s.character, |ed| ed.character);
        let s = format!("Trying {} sprites as edited  ·  Esc: back to the editor", whose(c));
        let w = measure(&s, 13.0) + 28.0;
        let r = Rect::new((screen_width() - w) / 2.0, screen_height() - 52.0, w, 32.0);
        rrect(r, 10.0, alpha(SURFACE, 0.92));
        rrect_lines(r, 10.0, 1.0, BORDER_HI);
        text_centered(&s, r.x + r.w / 2.0, r.y + r.h / 2.0, 13.0, TEXT);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprites_gathered_come_in_reading_order() {
        let r = |x: f32, y: f32, h: f32| Rect::new(x, y, 20.0, h);
        // Two rows; on each, sprites of different heights, out of order.
        let boxes = vec![
            r(60.0, 104.0, 30.0),
            r(0.0, 10.0, 40.0),
            r(30.0, 100.0, 40.0),
            r(90.0, 22.0, 20.0),
            r(40.0, 0.0, 50.0),
        ];
        let order: Vec<(f32, f32)> = reading_order(boxes).iter().map(|b| (b.x, b.y)).collect();
        assert_eq!(order, [(0.0, 10.0), (40.0, 0.0), (90.0, 22.0), (30.0, 100.0), (60.0, 104.0)]);
    }
}
