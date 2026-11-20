//! The character editor (Library → Sonic → Characters): characters the
//! player makes up. Each starts from a built-in one's drawing (its size and
//! sheet layout too) and has its own name, colours, physics (as multiples
//! of the guide's), moves (blocks on the jump or the action button, tried
//! in order), a 3D model from a model pack and a sprite sheet of its own.
//! It takes the whole screen; *Try* plays the character as edited, saved or
//! not, in 2D (the test track) or in 3D (the zone), and Esc comes back.

use super::regions::RegionTest;
use super::*;
use crate::library::acts;
use crate::sonic::custom::{self, Custom, PREVIEW};
use crate::sonic::kit::{Ability, Block, Button as Key, Kit, Physics, MOST_BLOCKS};
use crate::sonic::sheet::{self, Owner, Sheet};
use crate::sonic::{art, maker, Character, SCALE};
use crate::ui::theme::*;
use crate::ui::widgets::{
    button, clip, draw_button, draw_slider, draw_stepper, draw_toggle_row, section, stepper, SliderSpec, SliderState,
    SLIDER_ROW_H,
};

const TOP: f32 = 56.0;
const LIST_W: f32 = 184.0;
const MID_W: f32 = 400.0;
const GAP: f32 = 12.0;
const PAD: f32 = 14.0;
const LIST_ROW: f32 = 40.0;
const BLOCK_H: f32 = 46.0;
const MOST_NAME: usize = 24;
const MOST_MODEL: usize = 32;

/// Hues to pick colours from (degrees), each light, full and dark.
const HUES: [f32; 11] = [0.0, 24.0, 46.0, 90.0, 135.0, 170.0, 192.0, 216.0, 245.0, 278.0, 322.0];

fn hsv(h: f32, s: f32, v: f32) -> [u8; 3] {
    let c = v * s;
    let hp = (h / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r, g, b].map(|t| ((t + m) * 255.0).round() as u8)
}

/// The colours to pick from: the hues light, full and dark, and greys.
fn palette() -> Vec<[u8; 3]> {
    let mut out = vec![];
    for (s, v, grey) in [(0.35, 1.0, 250u8), (0.85, 0.92, 140), (0.85, 0.5, 36)] {
        out.extend(HUES.iter().map(|&h| hsv(h, s, v)));
        out.push([grey; 3]);
    }
    out
}

const PALETTE_COLS: usize = HUES.len() + 1;

fn colour(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

/// What the editor asks the app to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Ask {
    Close,
    Save,
    Delete,
    PlayAs,
    Try { three: bool },
    MakeSheet,
    ChooseSheet,
    EditRegions,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Cmd {
    New,
    Open(usize),
    Ask(Ask),
    Base(Character),
    Swatch(usize),
    Paint(usize),
    BaseColours,
    Roll,
    SpinDash,
    Magnet,
    Breathe,
    BaseMoves,
    Kind(usize, i32),
    Key(usize),
    Flip(usize),
    Up(usize),
    Remove(usize),
    Add,
    Model(i32),
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Name,
    Model,
}

struct BlockRects {
    kind: Rect,
    key: Rect,
    setting: Rect,
    up: Rect,
    remove: Rect,
}

struct Layout {
    list: Rect,
    rows: Vec<(Rect, Option<usize>)>,
    name: Rect,
    top: Vec<(Rect, Cmd)>,
    mid: Rect,
    preview: Rect,
    tries: [Rect; 2],
    looks_y: f32,
    bases: [Rect; 5],
    colours_y: f32,
    swatches: [Rect; 3],
    base_colours: Rect,
    palette: Vec<Rect>,
    model_y: f32,
    model: Rect,
    model_arrows: [Rect; 2],
    sheet_y: f32,
    sheet_buttons: [Rect; 3],
    right: Rect,
    physics: [Rect; 5],
    toggles: [Rect; 2],
    passives: [Rect; 2],
    moves_y: f32,
    base_moves: Rect,
    blocks: Vec<BlockRects>,
    add: Rect,
    about_y: f32,
}

pub(super) struct CharEditor {
    pub draft: Custom,
    /// The name it is saved under (none: not saved yet).
    pub was: Option<String>,
    /// As it was opened or last saved (to tell unsaved changes).
    baseline: Custom,
    focus: Option<Field>,
    /// The colour the palette sets (main, belly and muzzle, shoes).
    swatch: usize,
    physics: [SliderState; 5],
    settings: [SliderState; MOST_BLOCKS],
    /// List rows scrolled past.
    scroll: usize,
    clock: f32,
    /// A click that would lose unsaved changes, to be made again (and how
    /// long it waits).
    drop_asked: Option<(Cmd, f32)>,
    /// Delete clicked once: seconds left to click it again.
    delete_asked: f32,
    /// The model pack's names (models.dat), for ◀ ▶.
    models: Vec<String>,
    /// For the app to show.
    pub notes: Vec<String>,
    /// Backspace held: seconds until it takes another letter.
    backspace: f32,
    /// Just back from trying: the key that ended it is not for the editor.
    pub fresh: bool,
    /// In the studio: *Play as* is *Use in game* (the game plays as them).
    pub in_studio: bool,
}

/// A name no character has yet.
fn free_name(stem: &str) -> String {
    (1..)
        .map(|k| if k == 1 { stem.to_string() } else { format!("{stem} {k}") })
        .take(500)
        .find(|n| custom::find(n).is_none())
        .unwrap_or_else(|| stem.to_string())
}

/// "Sonic's", "Tails'".
fn whose(name: &str) -> String {
    if name.ends_with('s') {
        format!("{name}'")
    } else {
        format!("{name}'s")
    }
}

fn number(v: f32, unit: &str) -> String {
    match unit {
        "s" => format!("{v:.1} s"),
        "px" => format!("{v:.0} px"),
        "×" => format!("{v:.2}×"),
        _ => format!("{v:.1}"),
    }
}

impl CharEditor {
    /// The editor on the player's character `i`, or on a new one.
    pub fn new(open: Option<usize>) -> CharEditor {
        let mut ed = CharEditor {
            draft: Custom::default(),
            was: None,
            baseline: Custom::default(),
            focus: None,
            swatch: 0,
            physics: Default::default(),
            settings: Default::default(),
            scroll: 0,
            clock: 0.0,
            drop_asked: None,
            delete_asked: 0.0,
            models: crate::three::models::names(),
            notes: vec![],
            backspace: 0.0,
            fresh: false,
            in_studio: false,
        };
        match open.filter(|&i| i < custom::count()) {
            Some(i) => ed.open(i),
            None if custom::count() > 0 => ed.open(0),
            None => ed.start_new(),
        }
        ed
    }

    fn start_new(&mut self) {
        let blocks = vec![Block::new(Ability::DoubleJump { power: 0.8 })];
        self.draft =
            Custom { name: free_name("New character"), kit: Kit { blocks, ..Kit::default() }, ..Custom::default() };
        self.was = None;
        self.baseline = self.draft.clone().sanitized();
        self.swatch = 0;
    }

    fn open(&mut self, i: usize) {
        let Some(c) = custom::get(i as u8) else { return };
        self.was = Some(c.name.clone());
        self.baseline = c.clone();
        self.draft = c;
    }

    /// The one in the list it is (saved under its name).
    pub fn index(&self) -> Option<u8> {
        match self.was.as_deref().and_then(custom::find) {
            Some(Character::Custom(i)) => Some(i),
            _ => None,
        }
    }

    pub fn dirty(&self) -> bool {
        self.draft.clone().sanitized() != self.baseline
    }

    /// Kept as `name` (by the app).
    pub fn saved(&mut self) {
        self.draft = self.draft.clone().sanitized();
        self.was = Some(self.draft.name.clone());
        self.baseline = self.draft.clone();
    }

    /// The one shown was deleted (by the app): on to another.
    pub fn deleted(&mut self) {
        self.delete_asked = 0.0;
        if custom::count() > 0 {
            self.open(0);
        } else {
            self.start_new();
        }
    }

    fn layout(&self) -> Layout {
        let (sw, sh) = (screen_width(), screen_height());
        let total = (sw - 2.0 * GAP).min(LIST_W + MID_W + 2.0 * GAP + 600.0);
        let x0 = ((sw - total) / 2.0).max(GAP);
        let h = (sh - TOP - GAP).max(400.0);
        let list = Rect::new(x0, TOP, LIST_W, h);
        let mid = Rect::new(list.right() + GAP, TOP, MID_W, h);
        let right = Rect::new(mid.right() + GAP, TOP, (x0 + total - mid.right() - GAP).max(360.0), h);

        // The list: a new one, then the player's.
        let fits = ((list.h - 44.0) / LIST_ROW).floor().max(1.0) as usize;
        let mut rows = vec![(Rect::new(list.x + 8.0, list.y + 36.0, list.w - 16.0, LIST_ROW - 4.0), None)];
        for (slot, i) in (self.scroll..custom::count()).take(fits.saturating_sub(1)).enumerate() {
            let y = list.y + 36.0 + (slot + 1) as f32 * LIST_ROW;
            rows.push((Rect::new(list.x + 8.0, y, list.w - 16.0, LIST_ROW - 4.0), Some(i)));
        }

        // The top bar: the name, then what to do.
        let name = Rect::new(mid.x, 12.0, mid.w, 32.0);
        let mut top = vec![];
        let mut x = right.right();
        for (w, cmd) in [
            (76.0, Cmd::Ask(Ask::Close)),
            (76.0, Cmd::Ask(Ask::Save)),
            (76.0, Cmd::Ask(Ask::Delete)),
            (96.0, Cmd::Ask(Ask::PlayAs)),
        ] {
            x -= w;
            top.push((Rect::new(x, 12.0, w, 32.0), cmd));
            x -= 6.0;
        }

        // The middle: the preview, what they look like.
        let (mx, mw) = (mid.x + PAD, mid.w - 2.0 * PAD);
        let mut y = mid.y + PAD;
        let preview = Rect::new(mx, y, mw, 178.0);
        y += 178.0 + 8.0;
        let half = (mw - 8.0) / 2.0;
        let tries = [Rect::new(mx, y, half, 30.0), Rect::new(mx + half + 8.0, y, half, 30.0)];
        y += 30.0 + 14.0;
        let looks_y = y;
        y += 22.0;
        let bw = (mw - 4.0 * 6.0) / 5.0;
        let bases = std::array::from_fn(|k| Rect::new(mx + k as f32 * (bw + 6.0), y, bw, 28.0));
        y += 28.0 + 14.0;
        let colours_y = y;
        y += 22.0;
        let reset_w = 96.0;
        let sw3 = (mw - reset_w - 3.0 * 8.0) / 3.0;
        let swatches = std::array::from_fn(|k| Rect::new(mx + k as f32 * (sw3 + 8.0), y, sw3, 30.0));
        let base_colours = Rect::new(mx + mw - reset_w, y, reset_w, 30.0);
        y += 30.0 + 8.0;
        let cw = (mw - (PALETTE_COLS - 1) as f32 * 3.0) / PALETTE_COLS as f32;
        let palette = (0..PALETTE_COLS * 3)
            .map(|k| {
                let (col, row) = ((k % PALETTE_COLS) as f32, (k / PALETTE_COLS) as f32);
                Rect::new(mx + col * (cw + 3.0), y + row * 21.0, cw, 18.0)
            })
            .collect();
        y += 3.0 * 21.0 + 12.0;
        let model_y = y;
        y += 22.0;
        let model_arrows = [Rect::new(mx, y, 28.0, 28.0), Rect::new(mx + mw - 28.0, y, 28.0, 28.0)];
        let model = Rect::new(mx + 34.0, y, mw - 68.0, 28.0);
        y += 28.0 + 26.0;
        let sheet_y = y;
        y += 22.0 + 20.0;
        let third = (mw - 12.0) / 3.0;
        let sheet_buttons = std::array::from_fn(|k| Rect::new(mx + k as f32 * (third + 6.0), y, third, 28.0));

        // The right: how they move, and their moves.
        let (rx, rw) = (right.x + PAD, right.w - 2.0 * PAD);
        let mut y = right.y + PAD + 22.0;
        let col = (rw - 20.0) / 2.0;
        let physics = std::array::from_fn(|k| {
            let (c, r) = ((k % 2) as f32, (k / 2) as f32);
            Rect::new(rx + c * (col + 20.0), y + r * (SLIDER_ROW_H + 8.0), col, SLIDER_ROW_H)
        });
        // The fifth slider leaves room beside it for the two switches.
        let row3 = y + 2.0 * (SLIDER_ROW_H + 8.0);
        let toggles =
            [Rect::new(rx + col + 20.0, row3 - 4.0, col, 22.0), Rect::new(rx + col + 20.0, row3 + 22.0, col, 22.0)];
        y += 3.0 * (SLIDER_ROW_H + 8.0) + 4.0;
        // What they just are: a ring magnet, breathing underwater.
        let passives = [Rect::new(rx, y, col, 22.0), Rect::new(rx + col + 20.0, y, col, 22.0)];
        y += 22.0 + 16.0;
        let moves_y = y;
        let base_moves = Rect::new(rx + rw - 130.0, y - 2.0, 130.0, 22.0);
        y += 22.0 + 18.0;
        // The moves close up to fit (the add button and a summary under them).
        let n = self.draft.kit.blocks.len();
        let room = right.bottom() - PAD - y - 30.0 - 22.0 - 8.0 - 2.0 * 16.0;
        let block_h = if n > 0 { (room / n as f32).clamp(34.0, BLOCK_H) } else { BLOCK_H };
        let blocks = (0..n)
            .map(|k| {
                let by = y + k as f32 * block_h;
                let kind = Rect::new(rx, by, 168.0, 32.0);
                let key = Rect::new(kind.right() + 6.0, by, 92.0, 32.0);
                let remove = Rect::new(rx + rw - 28.0, by, 28.0, 32.0);
                let up = Rect::new(remove.x - 32.0, by, 28.0, 32.0);
                let setting =
                    Rect::new(key.right() + 12.0, by - 4.0, (up.x - 12.0 - key.right() - 12.0).max(40.0), 36.0);
                BlockRects { kind, key, setting, up, remove }
            })
            .collect();
        y += n as f32 * block_h;
        let add = Rect::new(rx, y, 150.0, 30.0);
        let about_y = y + 30.0 + 22.0;
        Layout {
            list,
            rows,
            name,
            top,
            mid,
            preview,
            tries,
            looks_y,
            bases,
            colours_y,
            swatches,
            base_colours,
            palette,
            model_y,
            model,
            model_arrows,
            sheet_y,
            sheet_buttons,
            right,
            physics,
            toggles,
            passives,
            moves_y,
            base_moves,
            blocks,
            add,
            about_y,
        }
    }

    pub fn update(&mut self, dt: f32, input: &mut Input) -> Option<Ask> {
        self.clock += dt;
        self.delete_asked = (self.delete_asked - dt).max(0.0);
        self.drop_asked = self.drop_asked.and_then(|(c, t)| (t > dt).then_some((c, t - dt)));
        let l = self.layout();

        // The two fields.
        if input.left_pressed {
            let was = self.focus;
            self.focus = if l.name.contains(input.mouse) {
                Some(Field::Name)
            } else if l.model.contains(input.mouse) {
                Some(Field::Model)
            } else {
                None
            };
            if self.focus.is_some() && self.focus != was {
                while get_char_pressed().is_some() {}
            }
        }
        if std::mem::take(&mut self.fresh) {
            // Esc ended the try: not a key for the editor.
        } else if let Some(f) = self.focus {
            let (text, most) = match f {
                Field::Name => (&mut self.draft.name, MOST_NAME),
                Field::Model => (&mut self.draft.model, MOST_MODEL),
            };
            while let Some(c) = get_char_pressed() {
                let fits = text.chars().count() < most;
                match f {
                    Field::Name if fits && !c.is_control() => text.push(c),
                    Field::Model if fits && c.is_ascii_graphic() => text.push(c.to_ascii_uppercase()),
                    _ => {}
                }
            }
            // Held, it goes on; with Ctrl, it clears the field.
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if is_key_pressed(KeyCode::Backspace) {
                if ctrl {
                    text.clear();
                } else {
                    text.pop();
                }
                self.backspace = 0.45;
            } else if is_key_down(KeyCode::Backspace) {
                self.backspace -= dt;
                if self.backspace <= 0.0 {
                    text.pop();
                    self.backspace = 0.04;
                }
            }
            if [KeyCode::Enter, KeyCode::KpEnter, KeyCode::Escape, KeyCode::Tab].iter().any(|k| is_key_pressed(*k)) {
                self.focus = None;
            }
        } else if is_key_pressed(KeyCode::Escape) {
            return self.run(Cmd::Ask(Ask::Close));
        } else if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            && is_key_pressed(KeyCode::S)
        {
            return self.run(Cmd::Ask(Ask::Save));
        }

        if input.wheel != 0.0 && l.list.contains(input.mouse) {
            let most = custom::count().saturating_sub(l.rows.len().saturating_sub(1));
            self.scroll = if input.wheel > 0.0 { self.scroll.saturating_sub(1) } else { (self.scroll + 1).min(most) };
        }

        // Sliders: the physics, and each move's setting.
        for k in 0..Physics::FIELDS.len() {
            let (_, lo, hi) = Physics::FIELDS[k];
            let mut v = self.draft.kit.physics.get(k);
            if self.physics[k].update(l.physics[k], &mut v, lo, hi, input) {
                self.draft.kit.physics.set(k, (v * 20.0).round() / 20.0);
            }
        }
        for (k, r) in l.blocks.iter().enumerate() {
            let b = &mut self.draft.kit.blocks[k];
            let Some(st) = b.ability.setting().filter(|s| s.unit != "toggle") else { continue };
            let mut v = st.value;
            if self.settings[k].update(r.setting, &mut v, st.lo, st.hi, input) {
                let step = if st.unit == "×" {
                    0.05
                } else if st.hi > 40.0 {
                    2.0
                } else {
                    0.5
                };
                b.ability = b.ability.with((v / step).round() * step);
            }
        }

        // Clicks.
        let mut cmd = None;
        let mut on = |r: Rect, c: Cmd, input: &mut Input| {
            if button(r, input) {
                cmd = Some(c);
            }
        };
        for (r, i) in &l.rows {
            on(*r, i.map_or(Cmd::New, Cmd::Open), input);
        }
        for (r, c) in &l.top {
            on(*r, *c, input);
        }
        on(l.tries[0], Cmd::Ask(Ask::Try { three: false }), input);
        on(l.tries[1], Cmd::Ask(Ask::Try { three: true }), input);
        for (k, r) in l.bases.iter().enumerate() {
            on(*r, Cmd::Base(Character::ALL[k]), input);
        }
        for (k, r) in l.swatches.iter().enumerate() {
            on(*r, Cmd::Swatch(k), input);
        }
        on(l.base_colours, Cmd::BaseColours, input);
        for (k, r) in l.palette.iter().enumerate() {
            on(*r, Cmd::Paint(k), input);
        }
        on(l.model_arrows[0], Cmd::Model(-1), input);
        on(l.model_arrows[1], Cmd::Model(1), input);
        for (r, a) in l.sheet_buttons.iter().zip([Ask::MakeSheet, Ask::ChooseSheet, Ask::EditRegions]) {
            on(*r, Cmd::Ask(a), input);
        }
        on(l.toggles[0], Cmd::Roll, input);
        on(l.toggles[1], Cmd::SpinDash, input);
        on(l.passives[0], Cmd::Magnet, input);
        on(l.passives[1], Cmd::Breathe, input);
        on(l.base_moves, Cmd::BaseMoves, input);
        for (k, r) in l.blocks.iter().enumerate() {
            on(r.up, Cmd::Up(k), input);
            on(r.remove, Cmd::Remove(k), input);
            if self.draft.kit.blocks[k].ability.fixed_button().is_none() {
                on(r.key, Cmd::Key(k), input);
            }
            let toggle = self.draft.kit.blocks[k].ability.setting().is_some_and(|s| s.unit == "toggle");
            if toggle {
                on(r.setting, Cmd::Flip(k), input);
            }
        }
        if self.draft.kit.blocks.len() < MOST_BLOCKS {
            on(l.add, Cmd::Add, input);
        }
        for (k, r) in l.blocks.iter().enumerate() {
            match stepper(r.kind, input) {
                0 => {}
                d => cmd = Some(Cmd::Kind(k, d)),
            }
        }
        input.consumed = true;
        input.over_ui = true;
        let ask = cmd.and_then(|c| self.run(c));
        custom::set_preview(Some(self.draft.clone()));
        ask
    }

    fn run(&mut self, cmd: Cmd) -> Option<Ask> {
        let losing = matches!(cmd, Cmd::New | Cmd::Open(_) | Cmd::Ask(Ask::Close));
        if losing && self.dirty() && self.drop_asked.map(|(c, _)| c) != Some(cmd) {
            self.drop_asked = Some((cmd, 4.0));
            self.notes.push("Unsaved changes: again to drop them (or Save first)".into());
            return None;
        }
        let d = &mut self.draft;
        match cmd {
            Cmd::New => self.start_new(),
            Cmd::Open(i) => self.open(i),
            Cmd::Ask(Ask::Delete) if self.was.is_none() => {
                self.notes.push("Not saved yet: nothing to delete".into());
            }
            Cmd::Ask(Ask::Delete) if self.delete_asked <= 0.0 => self.delete_asked = 3.0,
            Cmd::Ask(a @ (Ask::MakeSheet | Ask::EditRegions)) if self.was.is_none() => {
                let _ = a;
                self.notes.push("Save the character first: its sheet goes by its name".into());
            }
            Cmd::Ask(a) => return Some(a),
            Cmd::Base(c) => d.base = c,
            Cmd::Swatch(k) => self.swatch = k,
            Cmd::Paint(i) => {
                let mut cs = d.colours.unwrap_or_else(|| art::base_colours(d.base));
                cs[self.swatch] = palette()[i];
                d.colours = Some(cs);
            }
            Cmd::BaseColours => d.colours = None,
            Cmd::Roll => d.kit.roll = !d.kit.roll,
            Cmd::SpinDash => d.kit.spin_dash = !d.kit.spin_dash,
            Cmd::Magnet => d.kit.magnet = !d.kit.magnet,
            Cmd::Breathe => d.kit.breathe = !d.kit.breathe,
            Cmd::BaseMoves => d.kit = Kit::of(d.base),
            Cmd::Kind(k, dir) => {
                if let Some(b) = d.kit.blocks.get_mut(k) {
                    *b = Block::new(b.ability.cycle(dir));
                }
            }
            Cmd::Key(k) => {
                if let Some(b) = d.kit.blocks.get_mut(k) {
                    b.on = if b.on == Key::Action { Key::JumpAgain } else { Key::Action };
                }
            }
            Cmd::Flip(k) => {
                if let Some(b) = d.kit.blocks.get_mut(k) {
                    let on = b.ability.setting().is_some_and(|s| s.value >= 0.5);
                    b.ability = b.ability.with(if on { 0.0 } else { 1.0 });
                }
            }
            Cmd::Up(k) if k > 0 && k < d.kit.blocks.len() => d.kit.blocks.swap(k, k - 1),
            Cmd::Up(_) => {}
            Cmd::Remove(k) if k < d.kit.blocks.len() => {
                d.kit.blocks.remove(k);
            }
            Cmd::Remove(_) => {}
            Cmd::Add => {
                let next = Ability::ALL.into_iter().find(|a| !d.kit.has(*a)).unwrap_or(Ability::ALL[0]);
                if d.kit.blocks.len() < MOST_BLOCKS {
                    d.kit.blocks.push(Block::new(next));
                }
            }
            Cmd::Model(dir) => {
                if self.models.is_empty() {
                    self.notes.push("No model pack found (models.dat and models/ in the Sonic folder)".into());
                } else {
                    // The base's model first, then the pack's.
                    let all: Vec<&str> = std::iter::once("").chain(self.models.iter().map(|s| s.as_str())).collect();
                    let i = all.iter().position(|m| m.eq_ignore_ascii_case(&d.model)).unwrap_or(0) as i32;
                    d.model = all[(i + dir).rem_euclid(all.len() as i32) as usize].to_string();
                }
            }
        }
        if losing {
            self.drop_asked = None;
        }
        None
    }

    /// What the preview shows in turn: the usual poses, then each move's.
    fn showcase(&self) -> Vec<(&'static str, String, bool)> {
        let k = &self.draft.kit;
        let mut v = vec![
            ("idle", "Standing".to_string(), false),
            ("walk", "Walking".to_string(), false),
            ("run", "Running".to_string(), false),
            ("roll", "Jumping".to_string(), true),
        ];
        if k.spin_dash {
            v.push(("spindash", "Spin dash (↓ + Space)".to_string(), false));
        }
        for b in &k.blocks {
            let Some(&(name, _)) = b.ability.animations().first() else { continue };
            let key = if b.on == Key::Action { "X" } else { "Space in the air" };
            let air = !(b.on == Key::Action && b.ability.on_ground());
            v.push((name, format!("{} ({key})", b.ability.name()), air));
        }
        v
    }

    pub fn draw(&self, mouse: Vec2, sheet: Option<&Sheet>, has_sheet: bool) {
        clear_background(Color::new(0.05, 0.05, 0.09, 1.0));
        let l = self.layout();
        let m = mouse;
        text_bold("Characters", l.list.x + 2.0, baseline(28.0, 18.0), 18.0, TEXT);

        // The name.
        let r = l.name;
        let typing = self.focus == Some(Field::Name);
        rrect(r, 7.0, if typing { SURFACE_HI } else { SURFACE_2 });
        rrect_lines(r, 7.0, 1.0, if typing { ACCENT } else { BORDER });
        text("Name", r.x + 10.0, baseline(r.y + r.h / 2.0, 12.0), 12.0, TEXT_MUTED);
        let nx = r.x + 54.0;
        text(&self.draft.name, nx, baseline(r.y + r.h / 2.0, 15.0), 15.0, TEXT);
        if typing && (get_time() * 2.0) as i64 % 2 == 0 {
            let x = nx + 1.0 + measure(&self.draft.name, 15.0);
            draw_line(x, r.y + 7.0, x, r.y + r.h - 7.0, 1.2, TEXT);
        }
        let state = match (&self.was, self.dirty()) {
            (None, _) => ("not saved yet", WARNING),
            (Some(_), true) => ("changed", WARNING),
            (Some(_), false) => ("saved", SUCCESS),
        };
        text(state.0, r.right() - 10.0 - measure(state.0, 11.0), baseline(r.y + r.h / 2.0, 11.0), 11.0, state.1);

        for (r, c) in &l.top {
            let label = match c {
                Cmd::Ask(Ask::Close) => "Close",
                Cmd::Ask(Ask::Save) => "Save",
                Cmd::Ask(Ask::Delete) if self.delete_asked > 0.0 => "Sure?",
                Cmd::Ask(Ask::Delete) => "Delete",
                _ if self.in_studio => "Use in game",
                _ => "Play as ▶",
            };
            let lit = matches!(c, Cmd::Ask(Ask::Save)) && self.dirty() || matches!(c, Cmd::Ask(Ask::PlayAs));
            let off = matches!(c, Cmd::Ask(Ask::Delete)) && self.was.is_none();
            draw_button(*r, label, r.contains(m) && !off, lit, if off { 0.5 } else { 1.0 });
            if matches!(c, Cmd::Ask(Ask::Delete)) && self.delete_asked > 0.0 {
                rrect_lines(*r, 7.0, 1.5, DANGER);
            }
        }

        self.draw_list(&l, m);
        self.draw_middle(&l, m, sheet, has_sheet);
        self.draw_right(&l, m);
    }

    fn draw_list(&self, l: &Layout, m: Vec2) {
        let p = l.list;
        panel(p, 1.0);
        text_bold("YOURS", p.x + 12.0, p.y + 22.0, 11.0, TEXT_MUTED);
        let open = self.index().map(|i| i as usize);
        for (r, i) in &l.rows {
            let lit = *i == open && (i.is_some() || self.was.is_none());
            let hov = r.contains(m);
            rrect(
                *r,
                8.0,
                if lit {
                    mix(SURFACE_2, ACCENT, 0.35)
                } else if hov {
                    SURFACE_HI
                } else {
                    SURFACE_2
                },
            );
            rrect_lines(*r, 8.0, 1.0, if lit { ACCENT_HI } else { BORDER });
            match i {
                None => {
                    text_bold("+", r.x + 12.0, baseline(r.y + r.h / 2.0, 16.0), 16.0, ACCENT_HI);
                    text("New character", r.x + 30.0, baseline(r.y + r.h / 2.0, 13.0), 13.0, TEXT);
                }
                Some(i) => {
                    let Some(c) = custom::get(*i as u8) else { continue };
                    let dot = colour(c.colours.map_or_else(|| art::base_colours(c.base)[0], |cs| cs[0]));
                    draw_circle(r.x + 16.0, r.y + r.h / 2.0, 7.0, dot);
                    let name = crate::ui::library::fit(&c.name, r.w - 40.0, 13.0);
                    text_bold(&name, r.x + 30.0, r.y + 16.0, 13.0, TEXT);
                    let about = format!("as {}  ·  {} moves", c.base.short(), c.kit.blocks.len());
                    text(&about, r.x + 30.0, r.y + 30.0, 10.5, TEXT_MUTED);
                }
            }
        }
        if custom::count() == 0 {
            let lines = crate::ui::library::wrap("Make one: its looks, how it moves and its moves", p.w - 24.0, 11.0);
            for (k, line) in lines.iter().enumerate() {
                text(line, p.x + 12.0, p.y + 100.0 + k as f32 * 15.0, 11.0, TEXT_MUTED);
            }
        }
        let shown = l.rows.len().saturating_sub(1);
        if self.scroll > 0 || self.scroll + shown < custom::count() {
            text_centered("Scroll for more", p.x + p.w / 2.0, p.bottom() - 10.0, 11.0, TEXT_MUTED);
        }
    }

    fn draw_middle(&self, l: &Layout, m: Vec2, sheet: Option<&Sheet>, has_sheet: bool) {
        let p = l.mid;
        panel(p, 1.0);
        let (mx, mw) = (p.x + PAD, p.w - 2.0 * PAD);
        self.draw_preview(l.preview, sheet);
        draw_button(l.tries[0], "Try in 2D ▶", l.tries[0].contains(m), false, 1.0);
        draw_button(l.tries[1], "Try in 3D ▶", l.tries[1].contains(m), false, 1.0);

        section(mx, l.looks_y, mw, "LOOKS LIKE", 1.0);
        for (k, r) in l.bases.iter().enumerate() {
            let c = Character::ALL[k];
            draw_button(*r, c.short(), r.contains(m), self.draft.base == c, 1.0);
        }

        section(mx, l.colours_y, mw, "COLOURS", 1.0);
        let cs = self.draft.colours.unwrap_or_else(|| art::base_colours(self.draft.base));
        for (k, (r, label)) in l.swatches.iter().zip(["Body", "Belly", "Shoes"]).enumerate() {
            let on = self.swatch == k;
            rrect(*r, 7.0, if r.contains(m) { SURFACE_HI } else { SURFACE_2 });
            rrect_lines(*r, 7.0, if on { 2.0 } else { 1.0 }, if on { ACCENT_HI } else { BORDER });
            let chip = Rect::new(r.x + 6.0, r.y + 6.0, 18.0, r.h - 12.0);
            rrect(chip, 4.0, colour(cs[k]));
            rrect_lines(chip, 4.0, 1.0, alpha(BLACK, 0.4));
            text(label, chip.right() + 7.0, baseline(r.y + r.h / 2.0, 12.0), 12.0, if on { TEXT } else { TEXT_DIM });
        }
        let own = self.draft.colours.is_some();
        draw_button(l.base_colours, "Base's colours", l.base_colours.contains(m), false, if own { 1.0 } else { 0.5 });
        for (k, (r, c)) in l.palette.iter().zip(palette()).enumerate() {
            rrect(*r, 4.0, colour(c));
            let chosen = cs[self.swatch] == c;
            if chosen || r.contains(m) {
                rrect_lines(*r, 4.0, 2.0, if chosen { WHITE } else { alpha(WHITE, 0.6) });
            }
            let _ = k;
        }

        section(mx, l.model_y, mw, "3D MODEL", 1.0);
        for (r, dir) in l.model_arrows.iter().zip([-1.0f32, 1.0]) {
            rrect(*r, 7.0, if r.contains(m) { SURFACE_HI } else { SURFACE_2 });
            rrect_lines(*r, 7.0, 1.0, BORDER);
            let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
            draw_triangle(
                vec2(cx + 4.0 * dir, cy),
                vec2(cx - 3.0 * dir, cy - 5.0),
                vec2(cx - 3.0 * dir, cy + 5.0),
                TEXT_DIM,
            );
        }
        let r = l.model;
        let typing = self.focus == Some(Field::Model);
        rrect(r, 7.0, if typing { SURFACE_HI } else { SURFACE_2 });
        rrect_lines(r, 7.0, 1.0, if typing { ACCENT } else { BORDER });
        let (shown, col) = if self.draft.model.is_empty() && !typing {
            (format!("{} model", whose(self.draft.base.short())), TEXT_MUTED)
        } else {
            (self.draft.model.clone(), TEXT)
        };
        text(&shown, r.x + 10.0, baseline(r.y + r.h / 2.0, 13.0), 13.0, col);
        if typing && (get_time() * 2.0) as i64 % 2 == 0 {
            let x = r.x + 11.0 + measure(&self.draft.model, 13.0);
            draw_line(x, r.y + 6.0, x, r.y + r.h - 6.0, 1.2, TEXT);
        }
        let hint = if self.draft.model.is_empty() {
            "Or a name from your model pack's models.dat (◀ ▶ go through them)".to_string()
        } else if self.models.iter().any(|n| n.eq_ignore_ascii_case(&self.draft.model)) {
            "In your model pack".to_string()
        } else {
            "Not in your model pack: the base's model is used".to_string()
        };
        text(&hint, mx, r.bottom() + 15.0, 11.0, TEXT_MUTED);

        section(mx, l.sheet_y, mw, "SPRITE SHEET (2D)", 1.0);
        let about = match (&self.was, has_sheet) {
            (None, _) => "Save first: a sheet of its own goes by its name".to_string(),
            (Some(_), true) => format!("custom-{}.png  ·  drawn from it", custom::slug(&self.draft.name)),
            (Some(_), false) => "None: the game draws them, as their base, in their colours".to_string(),
        };
        text(&about, mx, l.sheet_y + 36.0, 12.0, if has_sheet { SUCCESS } else { TEXT_DIM });
        let saved = self.was.is_some();
        for (k, (r, label)) in
            l.sheet_buttons.iter().zip(["Make from drawing", "Choose a picture…", "Edit regions"]).enumerate()
        {
            let ok = match k {
                0 => saved,
                1 => !cfg!(target_arch = "wasm32"),
                _ => saved && has_sheet,
            };
            draw_button(*r, label, ok && r.contains(m), false, if ok { 1.0 } else { 0.5 });
        }
    }

    fn draw_preview(&self, pv: Rect, sheet: Option<&Sheet>) {
        gradient_v(pv.x, pv.y, pv.w, pv.h, Color::new(0.16, 0.2, 0.36, 1.0), Color::new(0.07, 0.08, 0.16, 1.0));
        rrect_lines(pv, RADIUS, 1.0, BORDER);
        let show = self.showcase();
        const EACH: f32 = 1.6;
        let n = show.len().max(1);
        let k = (self.clock / EACH) as usize % n;
        let t = self.clock % EACH;
        let (name, caption, air) = &show[k];
        let c = Character::Custom(PREVIEW);
        let z = 2.0;
        let ground = pv.bottom() - 30.0;
        draw_line(pv.x + 16.0, ground, pv.right() - 16.0, ground, 2.0, alpha(Color::new(0.5, 0.8, 0.4, 1.0), 0.7));
        let pics = maker::drawn_animations(c).iter().find(|(n, _)| n == name).map_or(1, |(_, n)| *n);
        let (mut s, _) = maker::posed(c, name, (t * 12.0) as usize % pics, pics, Vec2::ZERO);
        let feet = s.radii().1 * SCALE;
        // In the air: a hop; flying and gliding: hovering.
        let hop = match *name {
            "fly" | "fly_tired" => 30.0 + (self.clock * 4.0).sin() * 4.0,
            // Lying flat, the body is lower than standing.
            "glide" => (self.clock * 4.0).sin() * 4.0 - 4.0,
            _ if *air => (t / EACH * std::f32::consts::PI).sin() * 26.0,
            _ => 0.0,
        };
        let screen = vec2(pv.x + pv.w / 2.0, ground - feet * z - hop);
        // Zoomed in on the screen (`from_display_rect` would flip y: it is
        // meant for render targets).
        let (sw, sh) = (screen_width(), screen_height());
        let r = Rect::new(-screen.x / z, -screen.y / z, sw / z, sh / z);
        let cam = Camera2D { target: r.center(), zoom: vec2(2.0 / r.w, 2.0 / r.h), ..Default::default() };
        clip(Some(pv));
        set_camera(&cam);
        let drawn = sheet.is_some_and(|sh| sh.draw_anim(name, t, 12.0, Vec2::ZERO, 1.0, WHITE));
        s.facing = 1.0;
        if !drawn {
            art::draw_at(&s, self.clock);
        }
        art::move_effects(&s, self.clock);
        set_default_camera();
        clip(None);
        text(caption, pv.x + 12.0, baseline(pv.y + 16.0, 12.0), 12.0, TEXT_DIM);
        let page = format!("{} / {n}", k + 1);
        text(&page, pv.right() - 12.0 - measure(&page, 11.0), baseline(pv.y + 16.0, 11.0), 11.0, TEXT_MUTED);
        let how = if sheet.is_some() { "From its sheet" } else { "The game's drawing" };
        text(how, pv.x + 12.0, baseline(pv.bottom() - 13.0, 11.0), 11.0, TEXT_MUTED);
    }

    fn draw_right(&self, l: &Layout, m: Vec2) {
        let p = l.right;
        panel(p, 1.0);
        let (rx, rw) = (p.x + PAD, p.w - 2.0 * PAD);
        section(rx, p.y + PAD, rw, "HOW THEY MOVE", 1.0);
        let k = &self.draft.kit;
        for (i, r) in l.physics.iter().enumerate() {
            let (name, lo, hi) = Physics::FIELDS[i];
            let v = k.physics.get(i);
            let spec = SliderSpec { label: name, value_text: format!("×{v:.2}"), min: lo, max: hi, accent: ACCENT };
            draw_slider(*r, v, &spec, r.contains(m) || self.physics[i].dragging, 1.0);
        }
        draw_toggle_row(l.toggles[0], "Roll (↓ on the run)", k.roll, l.toggles[0].contains(m), 1.0);
        draw_toggle_row(l.toggles[1], "Spin dash (↓ + Space)", k.spin_dash, l.toggles[1].contains(m), 1.0);
        draw_toggle_row(l.passives[0], "Ring magnet", k.magnet, l.passives[0].contains(m), 1.0);
        draw_toggle_row(l.passives[1], "Breathes underwater", k.breathe, l.passives[1].contains(m), 1.0);

        section(rx, l.moves_y, rw - 140.0, "MOVES", 1.0);
        let copy = format!("{} moves", whose(self.draft.base.short()));
        draw_button(l.base_moves, &copy, l.base_moves.contains(m), false, 1.0);
        let hint = "Tried in order on a press  ·  each once a jump  ·  X is the action button";
        text(hint, rx, l.moves_y + 34.0, 11.0, TEXT_MUTED);
        for (i, (r, b)) in l.blocks.iter().zip(&k.blocks).enumerate() {
            draw_stepper(r.kind, &format!("{}", i + 1), b.ability.name(), ACCENT_HI, m, 1.0);
            // What is held down stays on its button.
            let (key, fixed) = match b.ability.fixed_button() {
                Some(Key::Action) => ("Hold X", true),
                Some(Key::JumpAgain) => ("Hold jump", true),
                None if b.on == Key::Action => ("Action (X)", false),
                None => ("Jump again", false),
            };
            draw_button(r.key, key, r.key.contains(m) && !fixed, false, if fixed { 0.7 } else { 1.0 });
            match b.ability.setting() {
                Some(st) if st.unit == "toggle" => {
                    // Its name above, as the sliders have, and the switch.
                    text(st.name, r.setting.x, r.setting.y + 14.0, 13.0, TEXT_DIM);
                    let row = Rect::new(r.setting.x, r.setting.y + 18.0, 40.0, 22.0);
                    draw_toggle_row(row, "", st.value >= 0.5, r.setting.contains(m), 1.0);
                }
                Some(st) => {
                    let spec = SliderSpec {
                        label: st.name,
                        value_text: number(st.value, st.unit),
                        min: st.lo,
                        max: st.hi,
                        accent: ACCENT,
                    };
                    draw_slider(r.setting, st.value, &spec, r.setting.contains(m) || self.settings[i].dragging, 1.0);
                }
                None => {
                    let s = if b.on == Key::Action && b.ability.on_ground() {
                        "On the ground or in the air"
                    } else {
                        "In the air"
                    };
                    text(s, r.setting.x, baseline(r.kind.y + r.kind.h / 2.0, 12.0), 12.0, TEXT_MUTED);
                }
            }
            for (btn, glyph, ok) in [(r.up, "▲", i > 0), (r.remove, "×", true)] {
                draw_button(btn, glyph, ok && btn.contains(m), false, if ok { 1.0 } else { 0.4 });
            }
        }
        if k.blocks.len() < MOST_BLOCKS {
            draw_button(l.add, "+ Add a move", l.add.contains(m), false, 1.0);
        } else {
            let most = format!("{MOST_BLOCKS} moves at most");
            text(&most, l.add.x, baseline(l.add.y + l.add.h / 2.0, 12.0), 12.0, TEXT_MUTED);
        }
        if k.blocks.is_empty() {
            let s = "No moves of its own: it runs, rolls and jumps";
            text(s, l.add.right() + 12.0, baseline(l.add.y + l.add.h / 2.0, 12.0), 12.0, TEXT_MUTED);
        }
        section(rx, l.about_y - 22.0, rw, "IN SHORT", 1.0);
        for (i, line) in crate::ui::library::wrap(&k.about(), rw, 12.0).iter().take(2).enumerate() {
            text(line, rx, l.about_y + 8.0 + i as f32 * 16.0, 12.0, TEXT_DIM);
        }
    }
}

/// What to come back to after trying a character.
pub(super) struct CharTest {
    /// The scene (in 2D; none in 3D).
    back: Option<RegionTest>,
    character: Character,
}

impl App {
    /// Open the character editor (on the player's character `i`, or the
    /// one played, or the first).
    pub(super) fn open_chars(&mut self, i: Option<usize>) {
        custom::reload();
        let played = match self.s.character {
            Character::Custom(i) if i != PREVIEW => Some(i as usize),
            _ => None,
        };
        self.library.fader.open = false;
        let mut ed = CharEditor::new(i.or(played));
        ed.in_studio = self.studio;
        self.chars = Some(ed);
    }

    /// A frame of the editor, over the whole screen.
    pub(super) fn chars_frame(&mut self, dt: f32) {
        set_default_camera();
        self.sonic_sheets_once();
        let mut input = Input::gather();
        let Some(ed) = self.chars.as_mut() else { return };
        let ask = ed.update(dt, &mut input);
        for n in ed.notes.drain(..) {
            self.toasts.info(n);
        }
        if let Some(ask) = ask {
            self.chars_ask(ask);
        }
        if let Some(ed) = self.chars.as_ref() {
            let i = ed.index();
            let sheet = i.and_then(|i| self.sonic_sheet.get(Character::Custom(i)));
            let has = i.is_some_and(|i| sheet::has_sheet(Character::Custom(i)));
            ed.draw(input.mouse, sheet, has);
        }
        self.toasts.update(dt);
        self.toasts.draw(56.0, &self.jobs, dt);
    }

    fn chars_ask(&mut self, ask: Ask) {
        match ask {
            Ask::Close => {
                self.chars = None;
                custom::set_preview(None);
                if !self.studio {
                    self.library.open_sonic();
                }
            }
            Ask::Save => {
                self.save_char();
            }
            Ask::Delete => self.delete_char(),
            Ask::PlayAs => {
                let dirty = self.chars.as_ref().is_some_and(|ed| ed.dirty() || ed.was.is_none());
                if dirty && !self.save_char() {
                    return;
                }
                let Some(i) = self.chars.as_ref().and_then(|ed| ed.index()) else { return };
                // The studio has no game to play: the game plays as them.
                if self.studio {
                    self.use_in_game(Character::Custom(i));
                    return;
                }
                self.set_character(Character::Custom(i));
                self.chars = None;
                custom::set_preview(None);
                self.library.open_sonic();
            }
            Ask::Try { three } => self.try_char(three),
            Ask::MakeSheet => self.make_char_sheet(),
            Ask::ChooseSheet => {
                let saved = self.chars.as_ref().is_some_and(|ed| ed.was.is_some() && !ed.dirty());
                if !saved && !self.save_char() {
                    return;
                }
                if let Some(i) = self.chars.as_ref().and_then(|ed| ed.index()) {
                    self.choose_sheet(Owner::Char(Character::Custom(i)));
                }
            }
            Ask::EditRegions => {
                if let Some(i) = self.chars.as_ref().and_then(|ed| ed.index()) {
                    self.open_regions(Some(Owner::Char(Character::Custom(i))));
                }
            }
        }
    }

    /// Keep the character edited; whoever is played stays who they were
    /// (the list's order changes with names).
    fn save_char(&mut self) -> bool {
        let Some(ed) = self.chars.as_mut() else { return false };
        let played = self.s.character.is_custom().then(|| self.s.character.key());
        let (old, new) = (ed.was.clone(), ed.draft.clone().sanitized());
        if new.name != old.clone().unwrap_or_default() && custom::find(&new.name).is_some() {
            self.toasts.warn(format!("There is already a character called {}", new.name));
            return false;
        }
        match custom::save(&new, old.as_deref()) {
            Ok(_) => {
                ed.saved();
                if let Some(o) = old.as_ref().filter(|o| custom::slug(o) != custom::slug(&new.name)) {
                    rename_sheet(o, &new.name);
                }
                self.toasts.success(format!("{} kept  ·  Library → Sonic → Play as", new.name));
                self.characters_changed(played, old.map(|o| (o, new.name.clone())));
                true
            }
            Err(e) => {
                self.toasts.error(format!("Not saved: {e}"));
                false
            }
        }
    }

    fn delete_char(&mut self) {
        let Some(ed) = self.chars.as_mut() else { return };
        let Some(name) = ed.was.clone() else { return };
        let played = self.s.character.is_custom().then(|| self.s.character.key());
        match custom::delete(&name) {
            Ok(()) => {
                ed.deleted();
                self.toasts.info(format!("{name} deleted (a sheet of theirs stays in the Sonic folder)"));
                self.characters_changed(played, None);
            }
            Err(e) => self.toasts.error(format!("Not deleted: {e}")),
        }
    }

    /// The list changed: the one played is found again by name (`renamed`:
    /// from what to what), and the sheets are read again.
    fn characters_changed(&mut self, played: Option<String>, renamed: Option<(String, String)>) {
        if let Some(key) = played {
            let key = match renamed {
                Some((old, new)) if key == format!("custom:{old}") => format!("custom:{new}"),
                _ => key,
            };
            let c = Character::from_key(&key);
            self.s.character = c;
            if let Some(run) = self.sonic.as_mut() {
                run.sonic.become_character(c);
            }
        }
        if self.sonic_sheet_tried {
            self.load_sonic_sheet();
        }
    }

    /// A sheet made from the game's drawing of the character edited.
    fn make_char_sheet(&mut self) {
        let saved = self.chars.as_ref().is_some_and(|ed| ed.was.is_some() && !ed.dirty());
        if !saved && !self.save_char() {
            return;
        }
        let Some(i) = self.chars.as_ref().and_then(|ed| ed.index()) else { return };
        let c = Character::Custom(i);
        let made = maker::from_drawing(c);
        match self.keep_made(c, &made).and_then(|png| Sheet::install_for(&png, c)) {
            Ok(name) => {
                self.toasts.success(format!("{name} made from the game's drawing  ·  Edit regions to paint over it"));
                self.sonic_sheet_tried = true;
                self.load_sonic_sheet();
            }
            Err(e) => self.toasts.error(format!("Could not make the sheet: {e}")),
        }
    }

    /// Play the character as edited (saved or not): on the test track in 2D,
    /// or in the 3D zone. Esc comes back.
    fn try_char(&mut self, three: bool) {
        let Some(ed) = self.chars.as_ref() else { return };
        custom::set_preview(Some(ed.draft.clone()));
        let saved = ed.index();
        let c = Character::Custom(PREVIEW);
        self.sonic_sheets_once();
        // Its sheet, when it has one (as saved).
        if let Some(Ok(Some(s))) = saved.map(|i| Sheet::load(Character::Custom(i))) {
            self.sonic_sheet.set(c, s);
        }
        let character = self.s.character;
        if three {
            self.s.character = c;
            self.play_3d(true);
            if self.three.is_none() {
                self.s.character = character;
                return;
            }
            self.char_test = Some(CharTest { back: None, character });
        } else {
            let back = self.scene_to_come_back_to();
            self.char_test = Some(CharTest { back: Some(back), character });
            self.s.character = c;
            self.challenge = None;
            if !self.infinite() {
                self.set_world_size(4);
            }
            let (aw, ah) = self.arena();
            self.apply_scene(acts::TEST_TRACK.scene(aw / 2.0 - 600.0, ah));
            self.camera.zoom = 1.0;
            self.paused = false;
        }
        self.library.fader.open = false;
        let name = custom::label(PREVIEW);
        self.toasts.info(format!("Trying {name} as edited  ·  Esc: back to the editor"));
    }

    /// Esc while trying: back to the editor (and the scene as it was).
    pub(super) fn end_char_test(&mut self) {
        let Some(t) = self.char_test.take() else { return };
        self.three = None;
        match t.back {
            Some(back) => self.come_back_to(back),
            None => self.s.character = t.character,
        }
        self.s.character = t.character;
        if self.sonic_sheet_tried {
            self.load_sonic_sheet();
        }
        self.library.fader.open = false;
        if let Some(ed) = self.chars.as_mut() {
            ed.fresh = true;
        }
    }

    /// While trying in 2D: how to come back.
    pub(super) fn draw_char_test(&self) {
        if self.char_test.is_none() {
            return;
        }
        let s = format!("Trying {} as edited  ·  Esc: back to the editor", custom::label(PREVIEW));
        let w = measure(&s, 13.0) + 28.0;
        let r = Rect::new((screen_width() - w) / 2.0, screen_height() - 52.0, w, 32.0);
        rrect(r, 10.0, alpha(SURFACE, 0.92));
        rrect_lines(r, 10.0, 1.0, BORDER_HI);
        text_centered(&s, r.x + r.w / 2.0, r.y + r.h / 2.0, 13.0, TEXT);
    }
}

/// A character renamed: their sheet and its regions go by the new name.
fn rename_sheet(old: &str, new: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(dir) = sheet::dir() {
        let (o, n) = (custom::slug(old), custom::slug(new));
        for ext in ["png", "regions.json"] {
            let (from, to) = (dir.join(format!("custom-{o}.{ext}")), dir.join(format!("custom-{n}.{ext}")));
            if from.is_file() && !to.exists() {
                let _ = std::fs::rename(from, to);
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (old, new);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_has_light_full_and_dark_of_each_hue_and_greys() {
        let p = palette();
        assert_eq!(p.len(), PALETTE_COLS * 3);
        assert_eq!(hsv(0.0, 1.0, 1.0), [255, 0, 0]);
        assert_eq!(hsv(120.0, 1.0, 1.0), [0, 255, 0]);
        assert_eq!(hsv(240.0, 1.0, 0.5), [0, 0, 128]);
        // Each row ends with a grey, lighter to darker.
        let greys: Vec<[u8; 3]> = (0..3).map(|r| p[r * PALETTE_COLS + PALETTE_COLS - 1]).collect();
        assert!(greys.windows(2).all(|w| w[0][0] > w[1][0]));
        assert!(greys.iter().all(|g| g[0] == g[1] && g[1] == g[2]));
    }

    #[test]
    fn settings_read_well() {
        assert_eq!(number(8.0, "s"), "8.0 s");
        assert_eq!(number(56.0, "px"), "56 px");
        assert_eq!(number(0.8, "×"), "0.80×");
        assert_eq!(number(9.0, ""), "9.0");
    }
}
