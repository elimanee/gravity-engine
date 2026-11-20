//! The studio (`gravity_studio`): the character editor and the sprite region
//! editor as a program of their own, opened from a page of its own (closing
//! an editor comes back to it); and, in the game, the buttons that open it.
//!
//! The studio keeps what it makes where the game finds it (`characters/`,
//! `sonic/`), and never writes the game's settings but for *Use in the
//! game*. A game that opened the studio reads the characters and sheets
//! again when it is closed. The web version has no programs to open: its
//! editors stay in the game.

use super::*;
use crate::sonic::sheet::Owner;
use crate::sonic::{art, custom, maker, Character, SCALE};
use crate::ui::theme::*;
use crate::ui::widgets::{button, draw_button};

/// What the studio's page opens.
#[derive(Clone, Copy)]
enum Open {
    Characters,
    Regions(Option<Owner>),
}

/// Where things are on the studio's page.
struct Home {
    characters: Rect,
    regions: Rect,
    owners: Vec<(Rect, Owner)>,
    quit: Rect,
}

fn home_layout() -> Home {
    let (sw, sh) = (screen_width(), screen_height());
    let w = ((sw - 3.0 * 40.0) / 2.0).clamp(260.0, 440.0);
    let h = (sh - 330.0).clamp(220.0, 320.0);
    let x0 = (sw - 2.0 * w - 40.0) / 2.0;
    let y = 150.0;
    let characters = Rect::new(x0, y, w, h);
    let regions = Rect::new(x0 + w + 40.0, y, w, h);
    let n = Owner::ALL.len() as f32;
    let bw = (w - (n - 1.0) * 6.0) / n;
    let owners = Owner::ALL
        .into_iter()
        .enumerate()
        .map(|(k, o)| (Rect::new(regions.x + k as f32 * (bw + 6.0), regions.bottom() + 12.0, bw, 30.0), o))
        .collect();
    let quit = Rect::new(sw - 96.0, 18.0, 80.0, 32.0);
    Home { characters, regions, owners, quit }
}

impl App {
    /// The studio's state: a game's, with its editors opened from the
    /// studio's page.
    pub fn studio() -> App {
        let mut app = App::new(vec![], true);
        app.studio = true;
        app
    }

    /// The studio's page, when no editor is open.
    pub(super) fn studio_frame(&mut self, dt: f32) {
        set_default_camera();
        self.sonic_sheets_once();
        let mut input = Input::gather();
        let l = home_layout();
        let mut open = None;
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) {
            self.quit = true;
        } else if is_key_pressed(KeyCode::C) {
            open = Some(Open::Characters);
        } else if is_key_pressed(KeyCode::R) {
            open = Some(Open::Regions(None));
        }
        if button(l.characters, &mut input) {
            open = Some(Open::Characters);
        }
        if button(l.regions, &mut input) {
            open = Some(Open::Regions(None));
        }
        for &(r, o) in &l.owners {
            if button(r, &mut input) {
                open = Some(Open::Regions(Some(o)));
            }
        }
        if button(l.quit, &mut input) {
            self.quit = true;
        }
        self.draw_home(&l, input.mouse);
        self.toasts.update(dt);
        self.toasts.draw(56.0, &self.jobs, dt);
        match open {
            Some(Open::Characters) => self.open_chars(None),
            Some(Open::Regions(o)) => self.open_regions(o),
            None => {}
        }
    }

    fn draw_home(&self, l: &Home, m: Vec2) {
        let (sw, sh) = (screen_width(), screen_height());
        clear_background(Color::new(0.05, 0.05, 0.09, 1.0));
        gradient_v(0.0, 0.0, sw, sh * 0.5, Color::new(0.11, 0.1, 0.2, 1.0), Color::new(0.05, 0.05, 0.09, 1.0));
        let t = get_time() as f32;
        text_bold_centered(&format!("{} Studio", crate::config::APP_NAME), sw / 2.0, 62.0, 30.0, TEXT);
        let line = "Characters of your own and the sprite sheets: made here, played in the game";
        text_centered(line, sw / 2.0, 98.0, 14.0, TEXT_DIM);
        draw_button(l.quit, "Quit", l.quit.contains(m), false, 1.0);

        // Characters: theirs (or the built-in ones), running.
        let yours: Vec<Character> = Character::every().into_iter().filter(|c| c.is_custom()).take(4).collect();
        let shown = if yours.is_empty() { Character::ALL[..4].to_vec() } else { yours.clone() };
        let r = l.characters;
        self.home_card(r, "Characters", "C", m);
        let ground = r.y + r.h * 0.62;
        let step = r.w / (shown.len() as f32 + 1.0);
        for (k, &c) in shown.iter().enumerate() {
            let pose = if k % 2 == 0 { "run" } else { "walk" };
            self.home_figure(c, pose, vec2(r.x + step * (k as f32 + 1.0), ground), 1.4, t + k as f32 * 0.3);
        }
        draw_line(r.x + 24.0, ground, r.right() - 24.0, ground, 2.0, Color::new(0.5, 0.8, 0.4, 0.6));
        let about = match yours.len() {
            0 => {
                "Their looks and colours, how they run and jump, and up to eight moves. None of yours yet.".to_string()
            }
            _ => format!(
                "Their looks and colours, how they run and jump, and their moves  ·  {} of yours",
                custom::count()
            ),
        };
        for (i, s) in crate::ui::library::wrap(&about, r.w - 48.0, 13.0).iter().take(2).enumerate() {
            text(s, r.x + 24.0, r.bottom() - 46.0 + i as f32 * 18.0, 13.0, TEXT_DIM);
        }

        // Sprite regions: a sheet of pictures, one of them being laid out.
        let r = l.regions;
        self.home_card(r, "Sprite regions", "R", m);
        let cell = ((r.w - 48.0) / 4.0).min(r.h * 0.42 / 2.0);
        let grid = Rect::new(r.x + (r.w - cell * 4.0) / 2.0, r.y + 64.0, cell * 4.0, cell * 2.0);
        rrect(grid, 6.0, Color::new(0.12, 0.13, 0.2, 1.0));
        let poses = ["idle", "walk", "run", "roll", "spring", "skid", "push", "spindash"];
        for (k, pose) in poses.iter().enumerate() {
            let (cx, cy) = ((k % 4) as f32, (k / 4) as f32);
            let c = Rect::new(grid.x + cx * cell, grid.y + cy * cell, cell, cell);
            draw_rectangle_lines(c.x, c.y, c.w, c.h, 1.0, Color::new(0.3, 0.32, 0.45, 0.8));
            let zoom = cell / 60.0;
            self.home_figure(Character::Sonic, pose, vec2(c.x + c.w / 2.0, c.bottom() - 6.0), zoom, t);
            if k == 2 {
                // The one being laid out: its box and its centre.
                rrect_lines(Rect::new(c.x + 3.0, c.y + 3.0, c.w - 6.0, c.h - 6.0), 3.0, 2.0, ACCENT);
                let (px, py) = (c.x + c.w / 2.0, c.y + c.h * 0.55);
                draw_line(px - 6.0, py, px + 6.0, py, 1.5, WARNING);
                draw_line(px, py - 6.0, px, py + 6.0, 1.5, WARNING);
            }
        }
        let about = "Each character's sheet and the zone's tiles: boxes, centres, animations, colours, terrain";
        for (i, s) in crate::ui::library::wrap(about, r.w - 48.0, 13.0).iter().take(2).enumerate() {
            text(s, r.x + 24.0, r.bottom() - 46.0 + i as f32 * 18.0, 13.0, TEXT_DIM);
        }
        for &(b, o) in &l.owners {
            draw_button(b, o.short(), b.contains(m), false, 1.0);
        }

        let folder = crate::config::config_dir().map(|d| d.display().to_string()).unwrap_or_default();
        // The home folder as ~.
        let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
        let folder = match folder.strip_prefix(&home) {
            Some(rest) if !home.is_empty() => format!("~{rest}"),
            _ => folder,
        };
        let foot = format!("Kept in {folder}, where the game finds them  ·  Esc: quit");
        text_centered(&foot, sw / 2.0, sh - 30.0, 12.0, TEXT_MUTED);
        let version = format!("v{}", crate::config::APP_VERSION);
        text(&version, sw - 16.0 - measure(&version, 11.0), sh - 24.0, 11.0, TEXT_MUTED);
    }

    /// A card of the studio's page: its frame (lit under the pointer), its
    /// name and its key.
    fn home_card(&self, r: Rect, name: &str, key: &str, m: Vec2) {
        let hov = r.contains(m);
        rrect(r, 14.0, if hov { SURFACE_HI } else { SURFACE });
        rrect_lines(r, 14.0, if hov { 2.0 } else { 1.0 }, if hov { ACCENT } else { BORDER });
        text_bold(name, r.x + 24.0, r.y + 40.0, 20.0, TEXT);
        keycap(r.right() - 44.0, r.y + 22.0, key, 12.0, 1.0);
    }

    /// Character `c` in `pose`, feet at `at` (screen), `zoom` times bigger:
    /// from their sheet if they have one, else the game's drawing.
    fn home_figure(&self, c: Character, pose: &str, at: Vec2, zoom: f32, t: f32) {
        let pics = maker::drawn_animations(c).iter().find(|(n, _)| *n == pose).map_or(1, |(_, n)| *n);
        let (mut s, _) = maker::posed(c, pose, (t * 10.0) as usize % pics, pics, Vec2::ZERO);
        s.facing = 1.0;
        let feet = s.radii().1 * SCALE;
        let (sw, sh) = (screen_width(), screen_height());
        let screen = at - vec2(0.0, feet * zoom);
        // Zoomed in on the screen (as the character editor's preview).
        let r = Rect::new(-screen.x / zoom, -screen.y / zoom, sw / zoom, sh / zoom);
        set_camera(&Camera2D { target: r.center(), zoom: vec2(2.0 / r.w, 2.0 / r.h), ..Default::default() });
        let drawn = self.sonic_sheet.get(c).is_some_and(|sh| sh.draw_anim(pose, t, 12.0, Vec2::ZERO, 1.0, WHITE));
        if !drawn {
            art::draw_at(&s, t);
        }
        set_default_camera();
    }

    /// Where the studio is: beside the game, as `gravity_studio` (or named
    /// like the game, as the release downloads are).
    #[cfg(not(target_arch = "wasm32"))]
    fn studio_program() -> Option<std::path::PathBuf> {
        let me = std::env::current_exe().ok()?;
        let name = me.file_name()?.to_string_lossy().to_string();
        let mut names = vec![format!("gravity_studio{}", std::env::consts::EXE_SUFFIX)];
        if name.contains("gravity_engine") {
            names.insert(0, name.replace("gravity_engine", "gravity_studio"));
        }
        names.into_iter().map(|n| me.with_file_name(n)).find(|p| p.is_file())
    }

    /// Open the studio on `args` (`--characters`, `--sprite-regions=…`);
    /// false when it cannot be (the web version, or it is not beside the
    /// game), so the editor opens in the game instead.
    pub(super) fn launch_studio(&mut self, args: &[String]) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = args;
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.studio {
                return false;
            }
            if self.studio_child.is_some() {
                self.toasts.info("The studio is open already");
                return true;
            }
            let Some(program) = Self::studio_program() else {
                self.toasts.warn("The studio (gravity_studio) is not beside the game: opened here instead");
                return false;
            };
            let named = Settings::load().character.key();
            match std::process::Command::new(&program).args(args).spawn() {
                Ok(child) => {
                    self.studio_child = Some((child, named));
                    self.library.fader.open = false;
                    self.toasts.info("Opening the studio…  ·  the game reads what it makes when it is closed");
                    true
                }
                Err(e) => {
                    self.toasts.warn(format!("Could not open the studio ({e}): opened here instead"));
                    false
                }
            }
        }
    }

    /// The character editor: in the studio when there is one.
    pub(super) fn edit_characters_or_studio(&mut self) {
        if !self.launch_studio(&["--characters".to_string()]) {
            self.open_chars(None);
        }
    }

    /// The sprite region editor (on `c`'s sheet): in the studio when there
    /// is one.
    pub(super) fn edit_regions_or_studio(&mut self, c: Option<Owner>) {
        let arg = match c {
            Some(Owner::Char(ch)) if ch.is_custom() => format!("--sprite-regions={}", ch.key()),
            Some(o) => format!("--sprite-regions={}", o.short().to_lowercase()),
            None => "--sprite-regions".to_string(),
        };
        if !self.launch_studio(&[arg]) {
            self.open_regions(c);
        }
    }

    /// Once a frame in the game: the studio it opened was closed, so what
    /// it made is read again (characters, sheets, the character chosen
    /// there with *Use in the game*).
    pub(super) fn poll_studio(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some((child, _)) = self.studio_child.as_mut() else { return };
            if !matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
                return;
            }
            let Some((_, named)) = self.studio_child.take() else { return };
            self.studio_closed(&named);
        }
    }

    /// The studio was closed; `named` is the character the settings on
    /// disk named when it was opened.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn studio_closed(&mut self, named: &str) {
        // Made-up characters are known by name: their places may change.
        let played = self.s.character.key();
        custom::reload();
        self.load_sonic_sheet();
        let chosen = Settings::load().character;
        if chosen.key() != named {
            // *Use in the game* chose one.
            self.set_character(chosen);
        } else {
            let c = Character::from_key(&played);
            self.s.character = c;
            if let Some(run) = &mut self.sonic {
                run.sonic.become_character(c);
            }
        }
        self.toasts.success("Studio closed: characters and sprite sheets read again");
    }

    /// *Use in the game* in the studio: the game plays as `c` from now on
    /// (the settings on disk are changed, nothing else of them).
    pub(super) fn use_in_game(&mut self, c: Character) {
        let mut s = Settings::load();
        s.character = c;
        match s.save() {
            Ok(()) => self.toasts.success(format!("The game plays as {} from now on", c.label())),
            Err(e) => self.toasts.error(format!("Could not change the game's settings: {e}")),
        }
    }
}

/// The studio's arguments (or the game's, passed on to it): the editor to
/// open first, if any. `--sprite-regions=WHO` takes a built-in character's
/// short name, `zone`, or one of the player's (`custom:NAME`).
pub fn studio_arg(arg: &str) -> Option<StudioStart> {
    match arg {
        "--characters" => Some(StudioStart::Characters),
        "--sprite-regions" => Some(StudioStart::Regions(None)),
        a => {
            let who = a.strip_prefix("--sprite-regions=")?;
            let name = who.to_lowercase();
            let owner = Owner::ALL
                .into_iter()
                .find(|o| o.short().to_lowercase() == name)
                .or_else(|| who.strip_prefix("custom:").and_then(custom::find).map(Owner::Char));
            Some(StudioStart::Regions(owner))
        }
    }
}

/// What the studio opens first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StudioStart {
    Characters,
    Regions(Option<Owner>),
}

impl App {
    /// Open what the studio was started on.
    pub fn studio_start(&mut self, start: StudioStart) {
        match start {
            StudioStart::Characters => self.open_chars(None),
            StudioStart::Regions(o) => self.open_regions(o),
        }
    }

    /// In the game, from the command line: the studio opened on `start`
    /// (or the editor in the game, without it).
    pub fn open_studio(&mut self, start: StudioStart) {
        match start {
            StudioStart::Characters => self.edit_characters_or_studio(),
            StudioStart::Regions(o) => self.edit_regions_or_studio(o),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_studio_is_started_on_an_editor_by_its_arguments() {
        assert_eq!(studio_arg("--characters"), Some(StudioStart::Characters));
        assert_eq!(studio_arg("--sprite-regions"), Some(StudioStart::Regions(None)));
        assert_eq!(
            studio_arg("--sprite-regions=Tails"),
            Some(StudioStart::Regions(Some(Owner::Char(Character::Tails))))
        );
        assert_eq!(studio_arg("--sprite-regions=zone"), Some(StudioStart::Regions(Some(Owner::Zone))));
        assert_eq!(studio_arg("--sprite-regions=nobody"), Some(StudioStart::Regions(None)));
        assert_eq!(studio_arg("level.gscene"), None);
    }
}
