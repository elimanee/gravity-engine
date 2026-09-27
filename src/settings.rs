//! User preferences, persisted as JSON in the config directory.

use crate::background::BgMode;
use crate::physics::borders::BorderMode;
use crate::physics::tools::Tool;
use crate::shapes::Shape;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub gravity: f32,
    pub time_scale: f32,
    pub border: BorderMode,
    pub background: BgMode,
    pub custom_background: Option<String>,

    pub tool: Tool,
    pub tool_radius: f32,
    pub tool_strength: f32,

    pub trails: bool,
    pub trail_length: f32,
    pub trail_fade: f32,

    pub window_shake: bool,
    pub shake_force: f32,

    pub volume: f32,

    pub spawn_shape: Shape,
    /// Index into `shapes::PALETTE`; `PALETTE.len()` means "random".
    pub spawn_color: usize,
    pub spawn_size: f32,

    pub show_title: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            gravity: -9.8,
            time_scale: 1.0,
            border: BorderMode::Walls,
            background: BgMode::Dark,
            custom_background: None,
            tool: Tool::Spring,
            tool_radius: 150.0,
            tool_strength: 300.0,
            trails: true,
            trail_length: 15.0,
            trail_fade: 2.0,
            window_shake: true,
            shake_force: 8.0,
            volume: 1.0,
            spawn_shape: Shape::Circle,
            spawn_color: 0,
            spawn_size: 80.0,
            show_title: true,
        }
    }
}

pub const RADIUS_RANGE: (f32, f32) = (30.0, 450.0);
pub const STRENGTH_RANGE: (f32, f32) = (20.0, 1500.0);
pub const TRAIL_LEN_RANGE: (f32, f32) = (3.0, crate::config::TRAIL_MAX as f32);
pub const TRAIL_FADE_RANGE: (f32, f32) = (0.2, 8.0);
pub const SHAKE_RANGE: (f32, f32) = (0.0, 40.0);
pub const GRAVITY_RANGE: (f32, f32) = (-40.0, 20.0);
pub const TIME_SCALE_RANGE: (f32, f32) = (0.1, 2.0);
pub const SPAWN_SIZE_RANGE: (f32, f32) = (24.0, 200.0);

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        crate::config::config_dir().map(|d| d.join("settings.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .map(Settings::sanitized)
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("no config directory")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Clamp every value into its valid range (hand-edited files, old versions…).
    pub fn sanitized(mut self) -> Self {
        let clamp = |v: f32, (lo, hi): (f32, f32), d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        let d = Settings::default();
        self.gravity = clamp(self.gravity, GRAVITY_RANGE, d.gravity);
        self.time_scale = clamp(self.time_scale, TIME_SCALE_RANGE, d.time_scale);
        self.tool_radius = clamp(self.tool_radius, RADIUS_RANGE, d.tool_radius);
        self.tool_strength = clamp(self.tool_strength, STRENGTH_RANGE, d.tool_strength);
        self.trail_length = clamp(self.trail_length, TRAIL_LEN_RANGE, d.trail_length);
        self.trail_fade = clamp(self.trail_fade, TRAIL_FADE_RANGE, d.trail_fade);
        self.shake_force = clamp(self.shake_force, SHAKE_RANGE, d.shake_force);
        self.volume = clamp(self.volume, (0.0, 1.0), d.volume);
        self.spawn_size = clamp(self.spawn_size, SPAWN_SIZE_RANGE, d.spawn_size);
        self.spawn_color = self.spawn_color.min(crate::shapes::PALETTE.len());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_partial_files() {
        let s = Settings { gravity: -3.7, tool: Tool::Vortex, ..Default::default() };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.gravity, -3.7);
        assert_eq!(back.tool, Tool::Vortex);

        let partial: Settings = serde_json::from_str(r#"{"volume": 0.25}"#).unwrap();
        assert_eq!(partial.volume, 0.25);
        assert_eq!(partial.border, BorderMode::Walls);
    }

    #[test]
    fn sanitize_clamps() {
        let s = Settings { volume: 7.0, gravity: f32::NAN, spawn_color: 999, ..Default::default() }.sanitized();
        assert_eq!(s.volume, 1.0);
        assert_eq!(s.gravity, -9.8);
        assert_eq!(s.spawn_color, crate::shapes::PALETTE.len());
    }
}
