//! Audio facade: one entry point for tracker modules, common formats and
//! playlists.

pub mod analyzer;
pub mod eq;
pub mod sfx;
mod stream;
mod tracker;

use crate::config::{ext_of, file_name_of, PLAYLIST_EXT, TRACKER_EXT};
use analyzer::{Analyzer, Tap};
use std::sync::Arc;
use stream::StreamPlayer;
use tracker::TrackerPlayer;

/// What the "now playing" pill shows.
pub struct NowPlaying {
    pub fmt: String,
    pub title: String,
    pub playing: bool,
    /// Extra detail, e.g. the tracker order position.
    pub detail: Option<String>,
    /// Seconds played, and the length when known.
    pub position: f64,
    pub duration: Option<f64>,
    /// Index in the playlist.
    pub index: Option<usize>,
}

/// A song in the playlist.
#[derive(Clone, Debug, PartialEq)]
pub struct PlaylistEntry {
    pub path: String,
    pub title: String,
    /// Seconds, when known.
    pub duration: Option<f64>,
}

impl PlaylistEntry {
    fn new(path: String) -> Self {
        let lower = path.to_ascii_lowercase();
        let remote = lower.starts_with("http://") || lower.starts_with("https://");
        let title = if remote { path.clone() } else { title_of(&path) };
        let local_stream = !remote && !TRACKER_EXT.contains(&ext_of(&path).as_str());
        let duration = if local_stream { stream::probe_duration(&path).map(|d| d.as_secs_f64()) } else { None };
        PlaylistEntry { path, title, duration }
    }
}

/// "Artist - Title" from a file name, without the extension.
fn title_of(path: &str) -> String {
    let name = file_name_of(path);
    match name.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_string(),
        _ => name,
    }
}

pub struct Audio {
    tracker: TrackerPlayer,
    stream: Option<StreamPlayer>,
    volume: f32,
    tap: Arc<Tap>,
    eq: Arc<eq::EqParams>,
    /// Sound effects, on their own output stream (None without a device).
    sfx: Option<sfx::Sfx>,
    /// Live analysis of what is playing, for the visualizer.
    pub analyzer: Analyzer,
    pub playlist: Vec<PlaylistEntry>,
    /// Index of the entry loaded in a player.
    current: Option<usize>,
    /// The entry Play starts after a stop.
    last: usize,
    /// Pick the next song at random.
    pub shuffle: bool,
    /// Start over after the last song (otherwise stop).
    pub repeat: bool,
}

impl Audio {
    pub fn new(volume: f32) -> Self {
        let tap = Arc::new(Tap::default());
        let eq = Arc::new(eq::EqParams::default());
        let mut a = Audio {
            tracker: TrackerPlayer::start(tap.clone(), eq.clone()),
            stream: StreamPlayer::new(tap.clone(), eq.clone()),
            volume: -1.0,
            tap,
            eq,
            sfx: sfx::Sfx::new(),
            analyzer: Analyzer::default(),
            playlist: vec![],
            current: None,
            last: 0,
            shuffle: false,
            repeat: true,
        };
        a.set_volume(volume);
        a
    }

    pub fn is_audio_file(path: &str) -> bool {
        let ext = ext_of(path);
        TRACKER_EXT.contains(&ext.as_str())
            || crate::config::AUDIO_EXT.contains(&ext.as_str())
            || PLAYLIST_EXT.contains(&ext.as_str())
    }

    /// Files or URLs of `path`: the entries of a .pls playlist, or itself.
    fn expand(path: &str) -> Result<Vec<String>, String> {
        if PLAYLIST_EXT.contains(&ext_of(path).as_str()) {
            let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let dir = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(""));
            let tracks =
                if ext_of(path) == "pls" { stream::parse_pls(&content, dir) } else { stream::parse_m3u(&content, dir) };
            if tracks.is_empty() {
                return Err("playlist is empty".into());
            }
            Ok(tracks)
        } else {
            Ok(vec![path.to_string()])
        }
    }

    /// Replace the playlist with `path` (a song or a .pls playlist) and play
    /// it. Returns a short description on success.
    pub fn load(&mut self, path: &str) -> Result<String, String> {
        let entries = Self::expand(path)?;
        let n = entries.len();
        self.stop();
        self.playlist = entries.into_iter().map(PlaylistEntry::new).collect();
        self.play_index(0)?;
        Ok(if n > 1 { format!("{} ({n} tracks)", file_name_of(path)) } else { file_name_of(path) })
    }

    /// Append songs (or playlists) to the playlist; starts playing when
    /// nothing was. Returns how many entries were added.
    pub fn add(&mut self, paths: &[String]) -> usize {
        let first = self.playlist.len();
        for p in paths {
            if let Ok(entries) = Self::expand(p) {
                self.playlist.extend(entries.into_iter().map(PlaylistEntry::new));
            }
        }
        let added = self.playlist.len() - first;
        if added > 0 && self.current.is_none() {
            self.play_index(first).ok();
        }
        added
    }

    /// Put back a saved playlist without playing it.
    pub fn restore_playlist(&mut self, paths: &[String]) {
        self.playlist = paths.iter().filter(|p| !p.is_empty()).cloned().map(PlaylistEntry::new).collect();
    }

    /// Play entry `i` from the start.
    pub fn play_index(&mut self, i: usize) -> Result<(), String> {
        let Some(entry) = self.playlist.get(i) else { return Err("no such entry".into()) };
        let path = entry.path.clone();
        let result = if TRACKER_EXT.contains(&ext_of(&path).as_str()) {
            if let Some(s) = self.stream.as_mut() {
                s.stop();
            }
            self.tracker.load(&path)
        } else {
            self.tracker.stop();
            self.stream.as_mut().ok_or("no audio output device".to_string()).and_then(|s| s.play(&path))
        };
        self.current = result.is_ok().then_some(i);
        self.last = i;
        result
    }

    /// Play again after a stop: the last song played (or the first).
    pub fn resume(&mut self) -> bool {
        let i = if self.last < self.playlist.len() { self.last } else { 0 };
        !self.playlist.is_empty() && self.play_index(i).is_ok()
    }

    pub fn remove_entries(&mut self, remove: &[usize]) {
        let mut keep = Vec::with_capacity(self.playlist.len());
        let mut current = None;
        for (i, e) in self.playlist.drain(..).enumerate() {
            if remove.contains(&i) {
                continue;
            }
            if self.current == Some(i) {
                current = Some(keep.len());
            }
            keep.push(e);
        }
        if self.current.is_some() && current.is_none() {
            // The song playing was removed.
            self.tracker.stop();
            if let Some(s) = self.stream.as_mut() {
                s.stop();
            }
        }
        self.playlist = keep;
        self.current = current;
        self.last = current.unwrap_or(0);
    }

    /// Move entry `from` to position `to`.
    pub fn move_entry(&mut self, from: usize, to: usize) {
        if from >= self.playlist.len() || to >= self.playlist.len() || from == to {
            return;
        }
        let e = self.playlist.remove(from);
        self.playlist.insert(to, e);
        self.current = self.current.map(|c| {
            if c == from {
                to
            } else if from < c && c <= to {
                c - 1
            } else if to <= c && c < from {
                c + 1
            } else {
                c
            }
        });
    }

    /// Sort by title, keeping the current song playing.
    pub fn sort_playlist(&mut self) {
        let current = self.current.map(|i| self.playlist[i].path.clone());
        self.playlist.sort_by_key(|e| e.title.to_lowercase());
        if let Some(p) = current {
            self.current = self.playlist.iter().position(|e| e.path == p);
        }
    }

    pub fn clear_playlist(&mut self) {
        self.stop();
        self.playlist.clear();
    }

    /// The entry after (`d = 1`) or before (`d = -1`) the current one.
    fn step(&self, d: i32) -> Option<usize> {
        let n = self.playlist.len();
        if n == 0 {
            return None;
        }
        let cur = self.current.unwrap_or(0);
        if self.shuffle && n > 1 {
            let mut pick = macroquad::rand::gen_range(0, n - 1);
            if pick >= cur {
                pick += 1;
            }
            return Some(pick);
        }
        let next = cur as i32 + d;
        if (0..n as i32).contains(&next) {
            Some(next as usize)
        } else if self.repeat || d < 0 {
            Some(next.rem_euclid(n as i32) as usize)
        } else {
            None
        }
    }

    pub fn toggle_pause(&mut self) -> Option<bool> {
        let t = self.tracker.info();
        if t.loaded && !t.ended {
            self.tracker.toggle_pause();
            Some(!t.playing)
        } else if let Some(s) = self.stream.as_mut().filter(|s| s.info().loaded) {
            s.toggle_pause();
            Some(s.info().playing)
        } else {
            None
        }
    }

    pub fn set_volume(&mut self, v: f32) {
        let v = v.clamp(0.0, 1.0);
        if (v - self.volume).abs() < 0.001 {
            return;
        }
        self.volume = v;
        self.tracker.set_volume(v);
        if let Some(s) = self.stream.as_mut() {
            s.set_volume(v);
        }
    }

    /// Equalizer settings (gains in dB).
    pub fn set_eq(&self, enabled: bool, preamp: f32, bands: &[f32; 10]) {
        self.eq.set(enabled, preamp, bands);
    }

    /// Stop whatever is playing (the playlist stays).
    pub fn stop(&mut self) {
        self.tracker.stop();
        if let Some(s) = self.stream.as_mut() {
            s.stop();
        }
        self.current = None;
    }

    /// Next song (a lone tracker module: its next pattern).
    pub fn next(&mut self) {
        if self.playlist.len() <= 1 && self.tracker.info().loaded {
            self.tracker.jump_orders(1);
        } else if let Some(i) = self.step(1) {
            self.play_index(i).ok();
        }
    }

    /// Previous song, or back to the start of this one after a few seconds.
    pub fn previous(&mut self) {
        let t = self.tracker.info();
        if self.playlist.len() <= 1 && t.loaded {
            self.tracker.jump_orders(-1);
            return;
        }
        let position = self.now_playing().map_or(0.0, |n| n.position);
        let target = if position > 3.0 { self.current } else { self.step(-1) };
        if let Some(i) = target {
            self.play_index(i).ok();
        }
    }

    /// Jump to `frac` (0‥1) of the song, when its length is known.
    pub fn seek(&mut self, frac: f64) {
        let t = self.tracker.info();
        if t.loaded {
            self.tracker.seek(t.duration * frac.clamp(0.0, 1.0));
        } else if let Some(s) = self.stream.as_mut() {
            s.seek(frac);
        }
    }

    /// Play a sound effect (see [`sfx::Sfx::play`]).
    pub fn play_sfx(&mut self, now: f64, sound: sfx::Sound, volume: f32, pan: f32, pitch: f32) {
        if let Some(s) = self.sfx.as_mut() {
            s.play(now, sound, volume, pan, pitch);
        }
    }

    /// Refresh the visualizer analysis (once per frame).
    pub fn analyze(&mut self, dt: f32, gain: f32) {
        self.analyzer.update(&self.tap, dt, gain);
    }

    /// Once per frame: move on when the song ended.
    pub fn tick(&mut self) {
        let Some(cur) = self.current else { return };
        let t = self.tracker.info();
        // Learn tracker module lengths once they are loaded.
        if t.loaded && t.duration > 0.0 {
            if let Some(e) = self.playlist.get_mut(cur).filter(|e| e.duration.is_none()) {
                e.duration = Some(t.duration);
            }
        }
        let ended = (t.loaded && t.ended) || self.stream.as_ref().is_some_and(|s| s.finished());
        if ended {
            match self.step(1) {
                Some(i) => {
                    if self.play_index(i).is_err() {
                        self.current = None;
                    }
                }
                None => self.stop(),
            }
        }
    }

    pub fn now_playing(&self) -> Option<NowPlaying> {
        let t = self.tracker.info();
        let entry = self.current.and_then(|i| self.playlist.get(i));
        if t.loaded {
            let title = if t.title.trim().is_empty() { entry.map_or(t.name, |e| e.title.clone()) } else { t.title };
            return Some(NowPlaying {
                fmt: if t.fmt.is_empty() { "MOD".into() } else { t.fmt },
                title,
                playing: t.playing,
                detail: Some(format!("{}/{}", t.order + 1, t.orders.max(1))),
                position: t.position,
                duration: (t.duration > 0.0).then_some(t.duration),
                index: self.current,
            });
        }
        let stream = self.stream.as_ref()?;
        let s = stream.info();
        let (position, duration) = stream.position().unwrap_or((0.0, None));
        let title = entry.map_or_else(|| s.name.clone(), |e| e.title.clone());
        s.loaded.then(|| NowPlaying {
            fmt: s.fmt.clone(),
            title,
            playing: s.playing,
            detail: None,
            position,
            duration,
            index: self.current,
        })
    }
}
