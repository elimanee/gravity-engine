//! Audio facade: one entry point for tracker modules, common formats and
//! playlists.

pub mod analyzer;
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
}

pub struct Audio {
    tracker: TrackerPlayer,
    stream: Option<StreamPlayer>,
    volume: f32,
    tap: Arc<Tap>,
    /// Live analysis of what is playing, for the visualizer.
    pub analyzer: Analyzer,
}

impl Audio {
    pub fn new(volume: f32) -> Self {
        let tap = Arc::new(Tap::default());
        let mut a = Audio {
            tracker: TrackerPlayer::start(tap.clone()),
            stream: StreamPlayer::new(tap.clone()),
            volume: -1.0,
            tap,
            analyzer: Analyzer::default(),
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

    /// Load any supported audio file. Returns a short description on success.
    pub fn load(&mut self, path: &str) -> Result<String, String> {
        let ext = ext_of(path);
        if TRACKER_EXT.contains(&ext.as_str()) {
            if let Some(s) = self.stream.as_mut() {
                s.stop();
            }
            self.tracker.load(path)?;
            return Ok(file_name_of(path));
        }
        let stream = self.stream.as_mut().ok_or("no audio output device")?;
        self.tracker.stop();
        if PLAYLIST_EXT.contains(&ext.as_str()) {
            let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            let dir = std::path::Path::new(path).parent().unwrap_or(std::path::Path::new(""));
            let tracks = stream::parse_pls(&content, dir);
            if tracks.is_empty() {
                return Err("playlist is empty".into());
            }
            let name = format!("{} ({} tracks)", file_name_of(path), tracks.len());
            stream.load_playlist(tracks, name.clone())?;
            Ok(name)
        } else {
            stream.load(path)?;
            Ok(file_name_of(path))
        }
    }

    pub fn toggle_pause(&mut self) -> Option<bool> {
        let t = self.tracker.info();
        if t.loaded {
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

    /// Refresh the visualizer analysis (once per frame).
    pub fn analyze(&mut self, dt: f32, gain: f32) {
        self.analyzer.update(&self.tap, dt, gain);
    }

    pub fn tick(&mut self) {
        if let Some(s) = self.stream.as_mut() {
            s.tick();
        }
    }

    pub fn now_playing(&self) -> Option<NowPlaying> {
        let t = self.tracker.info();
        if t.loaded {
            return Some(NowPlaying {
                fmt: if t.fmt.is_empty() { "MOD".into() } else { t.fmt },
                title: if t.title.trim().is_empty() { t.name } else { t.title },
                playing: t.playing,
                detail: Some(format!("{}/{}", t.order + 1, t.orders.max(1))),
            });
        }
        let s = self.stream.as_ref()?.info();
        s.loaded.then(|| NowPlaying { fmt: s.fmt.clone(), title: s.name.clone(), playing: s.playing, detail: None })
    }
}
