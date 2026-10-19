//! The web version has no music player (no files to play): there is never
//! a stream player, and playlists are empty.

use super::analyzer::Tap;
use super::eq::EqParams;
use std::sync::Arc;
use std::time::Duration;

pub fn probe_duration(_path: &str) -> Option<Duration> {
    None
}

pub fn parse_pls(_content: &str, _dir: &std::path::Path) -> Vec<String> {
    vec![]
}

pub fn parse_m3u(_content: &str, _dir: &std::path::Path) -> Vec<String> {
    vec![]
}

#[derive(Clone, Default)]
pub struct StreamInfo {
    pub loaded: bool,
    pub playing: bool,
    pub name: String,
    pub fmt: String,
}

pub struct StreamPlayer {
    state: StreamInfo,
}

impl StreamPlayer {
    pub fn new(_tap: Arc<Tap>, _eq: Arc<EqParams>) -> Option<Self> {
        None
    }

    pub fn play(&mut self, _path: &str) -> Result<(), String> {
        Err("music needs the desktop version".into())
    }

    pub fn finished(&self) -> bool {
        true
    }

    pub fn position(&self) -> Option<(f64, Option<f64>)> {
        None
    }

    pub fn seek(&mut self, _frac: f64) {}

    pub fn toggle_pause(&mut self) {}

    pub fn stop(&mut self) {}

    pub fn set_volume(&mut self, _vol: f32) {}

    pub fn info(&self) -> &StreamInfo {
        &self.state
    }
}
