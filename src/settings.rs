//! User preferences, persisted as JSON in the config directory.

use crate::background::BgMode;
use crate::physics::borders::BorderMode;
use crate::physics::gadgets::{Ammo, GadgetKind, Trigger};
use crate::physics::links::LinkKind;
use crate::physics::tools::Tool;
use crate::physics::zones::ZoneKind;
use crate::shapes::Shape;
use crate::ui::visualizer::VisStyle;
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

    /// Visualizer layer drawn behind the objects.
    pub vis_background: VisStyle,
    /// Style of visualizer objects (never `Off`).
    pub vis_object: VisStyle,
    /// Visualizer sensitivity.
    pub vis_gain: f32,
    /// Objects hop on each detected beat.
    pub vis_dance: bool,

    pub spawn_shape: Shape,
    /// Index into `shapes::PALETTE`; `PALETTE.len()` means "random".
    pub spawn_color: usize,
    pub spawn_size: f32,

    /// Draw tool stroke thickness (px).
    pub draw_thickness: f32,
    /// Drawings are pinned in place (Shift inverts).
    pub draw_pinned: bool,
    pub link_kind: LinkKind,
    /// Motor speed (rad/s, positive = clockwise).
    pub motor_speed: f32,
    /// New motors are driven with ← / → instead of turning by themselves.
    pub motor_drive: bool,

    /// What the Gadget tool places, and when it works.
    pub gadget_kind: GadgetKind,
    pub gadget_trigger: Trigger,
    /// Thrust as a multiple of the object's weight.
    pub thrust: f32,
    /// Cannon muzzle speed (m/s), shots per second and ammunition.
    pub cannon_speed: f32,
    pub cannon_rate: f32,
    pub cannon_ammo: Ammo,

    pub zone_kind: ZoneKind,
    /// Wind direction in degrees (0 = right, 90 = up).
    pub zone_angle: f32,
    /// Wind / lift strength (m/s²).
    pub zone_strength: f32,

    /// Particle effects (sparks, dust, splashes, debris).
    pub effects: bool,
    /// Ids of the challenges already solved.
    pub challenges_done: Vec<String>,
    /// Best stars (1‥3) per challenge id.
    pub challenge_stars: std::collections::BTreeMap<String, u8>,

    pub water: bool,
    /// Water surface height, 0 (floor) ‥ 1 (ceiling).
    pub water_level: f32,
    /// Water density relative to the default object density.
    pub water_density: f32,

    /// World size as a multiple of the window (1‥3).
    pub world_size: u8,

    /// What the Pour tool pours.
    pub grain_kind: crate::physics::grains::GrainKind,

    /// The classic player window is shown.
    pub player: bool,
    /// Drawn at twice the skin's size.
    pub player_double: bool,
    /// Skin file or folder; `None` for the built-in look.
    pub player_skin: Option<String>,
    /// The built-in look was chosen over the installed skins.
    pub player_skin_builtin: bool,
    /// Equalizer and playlist windows shown under the player.
    pub player_eq: bool,
    pub player_playlist: bool,
    /// The player windows are a physical object in the world.
    pub player_physics: bool,
    pub shuffle: bool,
    pub repeat: bool,
    pub eq_on: bool,
    /// Gains in dB (±12).
    pub eq_preamp: f32,
    pub eq_bands: [f32; 10],
    /// Songs in the playlist (files or URLs), restored at start.
    pub playlist: Vec<String>,

    /// Procedural sound effects (hits, breaking glass, explosions…).
    pub sfx: bool,
    pub sfx_volume: f32,
    /// Time slows down for a moment on big impacts and explosions.
    pub slow_motion: bool,

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
            vis_background: VisStyle::Off,
            vis_object: VisStyle::Bars,
            vis_gain: 1.5,
            vis_dance: false,
            spawn_shape: Shape::Circle,
            spawn_color: 0,
            spawn_size: 80.0,
            draw_thickness: 14.0,
            draw_pinned: false,
            link_kind: LinkKind::Rope,
            motor_speed: 4.0,
            motor_drive: false,
            gadget_kind: GadgetKind::Laser,
            gadget_trigger: Trigger::Always,
            thrust: 2.0,
            cannon_speed: 14.0,
            cannon_rate: 2.0,
            cannon_ammo: Ammo::Ball,
            zone_kind: ZoneKind::Wind,
            zone_angle: 0.0,
            zone_strength: 14.0,
            effects: true,
            challenges_done: vec![],
            challenge_stars: Default::default(),
            water: false,
            water_level: 0.3,
            water_density: 1.6,
            world_size: 1,
            grain_kind: Default::default(),
            player: false,
            player_double: true,
            player_skin: None,
            player_skin_builtin: false,
            player_eq: false,
            player_playlist: true,
            player_physics: false,
            shuffle: false,
            repeat: true,
            eq_on: false,
            eq_preamp: 0.0,
            eq_bands: [0.0; 10],
            playlist: vec![],
            sfx: true,
            sfx_volume: 0.6,
            slow_motion: true,
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
pub const VIS_GAIN_RANGE: (f32, f32) = (0.3, 5.0);
pub const DRAW_THICKNESS_RANGE: (f32, f32) = (4.0, 48.0);
pub const WATER_LEVEL_RANGE: (f32, f32) = (0.05, 0.9);
pub const WATER_DENSITY_RANGE: (f32, f32) = (0.3, 4.0);
pub const MOTOR_SPEED_RANGE: (f32, f32) = (-12.0, 12.0);
pub const ZONE_STRENGTH_RANGE: (f32, f32) = (2.0, 40.0);
pub const THRUST_RANGE: (f32, f32) = (0.2, 8.0);
pub const CANNON_SPEED_RANGE: (f32, f32) = (2.0, 40.0);
pub const CANNON_RATE_RANGE: (f32, f32) = (0.3, 10.0);

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
        self.sfx_volume = clamp(self.sfx_volume, (0.0, 1.0), d.sfx_volume);
        let db = crate::audio::eq::MAX_DB;
        self.eq_preamp = clamp(self.eq_preamp, (-db, db), 0.0);
        for b in &mut self.eq_bands {
            *b = clamp(*b, (-db, db), 0.0);
        }
        self.vis_gain = clamp(self.vis_gain, VIS_GAIN_RANGE, d.vis_gain);
        if self.vis_object == VisStyle::Off {
            self.vis_object = VisStyle::Bars;
        }
        self.spawn_size = clamp(self.spawn_size, SPAWN_SIZE_RANGE, d.spawn_size);
        self.spawn_color = self.spawn_color.min(crate::shapes::PALETTE.len());
        self.draw_thickness = clamp(self.draw_thickness, DRAW_THICKNESS_RANGE, d.draw_thickness);
        self.water_level = clamp(self.water_level, WATER_LEVEL_RANGE, d.water_level);
        self.water_density = clamp(self.water_density, WATER_DENSITY_RANGE, d.water_density);
        self.motor_speed = clamp(self.motor_speed, MOTOR_SPEED_RANGE, d.motor_speed);
        self.thrust = clamp(self.thrust, THRUST_RANGE, d.thrust);
        self.cannon_speed = clamp(self.cannon_speed, CANNON_SPEED_RANGE, d.cannon_speed);
        self.cannon_rate = clamp(self.cannon_rate, CANNON_RATE_RANGE, d.cannon_rate);
        self.world_size = self.world_size.clamp(1, 3);
        self.zone_strength = clamp(self.zone_strength, ZONE_STRENGTH_RANGE, d.zone_strength);
        self.zone_angle =
            if self.zone_angle.is_finite() { (self.zone_angle / 90.0).round().rem_euclid(4.0) * 90.0 } else { 0.0 };
        if self.zone_kind == ZoneKind::Goal {
            self.zone_kind = ZoneKind::Wind;
        }
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
