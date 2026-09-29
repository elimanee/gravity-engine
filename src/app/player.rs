//! The classic player window: its skin and what its buttons do.

use super::*;
use crate::skin::{self, Skin};
use crate::ui::PlayerCmd;

impl App {
    /// Load the saved skin (or the first installed one) the first time the
    /// player is shown.
    pub(super) fn ensure_skin(&mut self) {
        if self.skin_tried {
            return;
        }
        self.skin_tried = true;
        let path = match self.s.player_skin.clone() {
            Some(p) => Some(std::path::PathBuf::from(p)),
            None if self.s.player_skin_builtin => None,
            // Prefer Audacious's own default skin when it is installed.
            None => {
                let found = skin::discover();
                found.iter().find(|p| skin::display_name(p) == "Default").or(found.first()).cloned()
            }
        };
        if let Some(p) = path {
            match Skin::load(&p) {
                Ok(s) => self.skin = Some(s),
                Err(e) => self.toasts.error(format!("Skin {}: {e}", skin::display_name(&p))),
            }
        }
        self.drawer.skin_name = self.skin_name();
    }

    fn skin_name(&self) -> String {
        self.skin.as_ref().map_or_else(|| "Built-in".to_string(), |s| s.name.clone())
    }

    /// Use the skin at `path` (a .wsz or a folder) and show the player.
    pub(super) fn use_skin(&mut self, path: &std::path::Path) {
        match Skin::load(path) {
            Ok(s) => {
                self.toasts.status("skin", format!("Skin: {}", s.name));
                self.s.player_skin = Some(path.to_string_lossy().into_owned());
                self.s.player_skin_builtin = false;
                self.skin = Some(s);
                self.skin_tried = true;
                self.s.player = true;
            }
            Err(e) => self.toasts.error(format!("Skin {}: {e}", skin::display_name(path))),
        }
        self.drawer.skin_name = self.skin_name();
    }

    /// Step through the installed skins; the built-in look comes first.
    pub(super) fn cycle_skin(&mut self, d: i32) {
        let found = skin::discover();
        let current = self.skin.as_ref().and_then(|s| s.path.clone());
        let i = current.and_then(|c| found.iter().position(|p| *p == c)).map_or(0, |i| i as i32 + 1);
        let n = found.len() as i32 + 1;
        let next = (i + d).rem_euclid(n);
        if next == 0 {
            self.skin = None;
            self.s.player_skin = None;
            self.s.player_skin_builtin = true;
            let hint = if found.is_empty() { "  ·  no skins found: drop a .wsz file on the window" } else { "" };
            self.toasts.status("skin", format!("Skin: built-in{hint}"));
            self.drawer.skin_name = self.skin_name();
        } else {
            self.use_skin(&found[next as usize - 1]);
        }
    }

    pub(super) fn pick_skin(&mut self) {
        let mut dlg = FileDialog::new().add_filter("Winamp / Audacious skin", &skin::SKIN_EXT);
        if let Some(dir) = skin::skin_dirs().into_iter().find(|d| d.is_dir()) {
            dlg = dlg.set_directory(dir);
        }
        if let Some(p) = dlg.pick_file() {
            self.use_skin(&p);
        }
    }

    pub(super) fn player_cmd(&mut self, cmd: PlayerCmd) {
        let loaded = self.audio.now_playing();
        match cmd {
            PlayerCmd::Play => match loaded {
                Some(n) if !n.playing => {
                    self.audio.toggle_pause();
                }
                Some(_) => self.audio.seek(0.0),
                None => self.apply(Action::LoadAudio, Vec2::ZERO),
            },
            PlayerCmd::Pause => {
                self.audio.toggle_pause();
            }
            PlayerCmd::Stop => self.audio.stop(),
            PlayerCmd::Next => self.audio.next(),
            PlayerCmd::Previous => self.audio.previous(),
            PlayerCmd::Eject => self.apply(Action::LoadAudio, Vec2::ZERO),
            PlayerCmd::Close => self.s.player = false,
            PlayerCmd::DoubleSize => self.s.player_double = !self.s.player_double,
            PlayerCmd::Seek(f) => self.audio.seek(f as f64),
            PlayerCmd::Volume(v) => self.s.volume = v.clamp(0.0, 1.0),
        }
    }
}
