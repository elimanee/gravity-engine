//! The classic player window: its skin and what its buttons do.

use super::*;
use crate::audio::NowPlaying;
use crate::skin::{self, Skin};
use crate::ui::skin_player::{EqView, PlayerView};
use crate::ui::PlayerCmd;

/// Equalizer presets: name, preamp, bands (dB).
const EQ_PRESETS: [(&str, f32, [f32; 10]); 8] = [
    ("Flat", 0.0, [0.0; 10]),
    ("Rock", -1.0, [5.0, 3.0, -2.0, -4.0, -1.5, 2.0, 5.0, 6.0, 6.0, 6.0]),
    ("Pop", 0.0, [-1.0, 3.0, 5.0, 5.5, 3.5, -1.0, -1.5, -1.5, -1.0, -1.0]),
    ("Dance", -1.0, [7.0, 5.5, 2.0, 0.0, 0.0, -3.5, -4.5, -4.5, 0.0, 0.0]),
    ("Classical", 0.0, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.5, -4.5, -4.5, -6.0]),
    ("Full bass", -3.0, [7.0, 7.0, 7.0, 4.0, 1.0, -3.5, -5.5, -6.5, -7.0, -7.0]),
    ("Full treble", -3.0, [-7.0, -7.0, -7.0, -3.0, 1.5, 7.0, 10.0, 10.0, 10.0, 10.5]),
    ("Headphones", 0.0, [3.5, 8.0, 4.0, -2.5, -2.0, 1.0, 3.5, 7.0, 9.0, 10.0]),
];

/// What the player windows show.
pub(super) fn view<'a>(
    s: &'a crate::settings::Settings,
    skin: Option<&'a Skin>,
    audio: &'a Audio,
    now: Option<&'a NowPlaying>,
) -> PlayerView<'a> {
    PlayerView {
        skin,
        now,
        volume: s.volume,
        double: s.player_double,
        playlist: &audio.playlist,
        eq: EqView { on: s.eq_on, preamp: s.eq_preamp, bands: s.eq_bands },
        show_eq: s.player_eq,
        show_playlist: s.player_playlist,
        shuffle: s.shuffle,
        repeat: s.repeat,
    }
}

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
                None if self.audio.resume() => {}
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
            PlayerCmd::ToggleEq => self.s.player_eq = !self.s.player_eq,
            PlayerCmd::TogglePlaylist => self.s.player_playlist = !self.s.player_playlist,
            PlayerCmd::Shuffle => {
                self.s.shuffle = !self.s.shuffle;
                self.toasts.status("player", format!("Shuffle {}", if self.s.shuffle { "on" } else { "off" }));
            }
            PlayerCmd::Repeat => {
                self.s.repeat = !self.s.repeat;
                self.toasts.status("player", format!("Repeat {}", if self.s.repeat { "on" } else { "off" }));
            }
            PlayerCmd::EqOn => self.s.eq_on = !self.s.eq_on,
            PlayerCmd::EqGain(0, g) => self.s.eq_preamp = g,
            PlayerCmd::EqGain(i, g) => {
                if let Some(b) = self.s.eq_bands.get_mut(i - 1) {
                    *b = g;
                }
                self.s.eq_on = true;
            }
            PlayerCmd::EqPreset => {
                // The preset after the one matching the sliders (Flat when none does).
                let current = EQ_PRESETS.iter().position(|(_, p, b)| *p == self.s.eq_preamp && *b == self.s.eq_bands);
                let (name, preamp, bands) = EQ_PRESETS[current.map_or(0, |i| (i + 1) % EQ_PRESETS.len())];
                self.s.eq_preamp = preamp;
                self.s.eq_bands = bands;
                self.s.eq_on = true;
                self.toasts.status("player", format!("Equalizer preset: {name}"));
            }
            PlayerCmd::PlayEntry(i) => {
                if let Err(e) = self.audio.play_index(i) {
                    self.toasts.error(format!("Audio: {e}"));
                }
            }
            PlayerCmd::RemoveEntry(i) => self.audio.remove_entries(&[i]),
            PlayerCmd::MoveEntry(from, to) => self.audio.move_entry(from, to),
            PlayerCmd::AddFiles => {
                let mut exts: Vec<&str> = crate::config::AUDIO_EXT.to_vec();
                exts.extend(crate::config::TRACKER_EXT);
                exts.extend(crate::config::PLAYLIST_EXT);
                if let Some(files) = FileDialog::new().add_filter("Music", &exts).pick_files() {
                    let paths: Vec<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
                    let n = self.audio.add(&paths);
                    self.toasts.status("player", format!("Added {n} songs to the playlist"));
                }
            }
            PlayerCmd::SortPlaylist => self.audio.sort_playlist(),
            PlayerCmd::ClearPlaylist => self.audio.clear_playlist(),
        }
    }
}
