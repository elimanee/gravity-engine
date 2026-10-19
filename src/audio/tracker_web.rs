//! The web version has no tracker module player (libopenmpt is a desktop
//! library): loading a module says so, everything else does nothing.

use super::analyzer::Tap;
use super::eq::EqParams;
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct TrackerInfo {
    pub loaded: bool,
    pub playing: bool,
    pub title: String,
    pub fmt: String,
    pub name: String,
    pub order: i32,
    pub orders: i32,
    pub position: f64,
    pub duration: f64,
    pub ended: bool,
}

pub struct TrackerPlayer;

impl TrackerPlayer {
    pub fn start(_tap: Arc<Tap>, _eq: Arc<EqParams>) -> Self {
        TrackerPlayer
    }

    pub fn load(&self, _path: &str) -> Result<(), String> {
        Err("music needs the desktop version".into())
    }

    pub fn toggle_pause(&self) {}

    pub fn stop(&self) {}

    pub fn seek(&self, _seconds: f64) {}

    pub fn jump_orders(&self, _delta: i32) {}

    pub fn set_volume(&self, _vol: f32) {}

    pub fn info(&self) -> TrackerInfo {
        TrackerInfo::default()
    }
}
