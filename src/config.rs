//! Global tuning constants and on-disk locations.

use std::path::PathBuf;

/// Pixels per physics metre.
pub const PPM: f32 = 60.0;
/// Default restitution of every object.
pub const BOUNCE: f32 = 0.45;
/// Default friction of every object.
pub const FRICTION: f32 = 0.5;
/// Default density of every object.
pub const DENSITY: f32 = 1.0;
/// Wall thickness in metres.
pub const WALL_T: f32 = 0.5;
/// Longest side (px) an imported image is scaled down to.
pub const MAX_IMG_PX: u32 = 220;
/// Hard upper bound on trail history per object.
pub const TRAIL_MAX: usize = 40;
/// Physics sub-step length (s). Frames are split into steps no longer than this.
pub const PHYSICS_DT: f32 = 1.0 / 60.0;
/// Objects further than this (m) outside the arena are culled.
pub const CULL_MARGIN: f32 = 60.0;

pub const APP_NAME: &str = "Gravity Engine";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Gravity presets: (label, g in m/s²).
pub const GRAVITY_PRESETS: &[(&str, f32)] = &[
    ("ZERO", 0.0),
    ("MOON", -1.6),
    ("MARS", -3.7),
    ("EARTH", -9.8),
    ("JUPITER", -24.8),
    ("HEAVY", -40.0),
    ("REVERSE", 12.0),
];

pub const IMAGE_EXT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tiff", "tif", "ico", "tga", "qoi", "hdr", "exr", "dds", "ff",
    "ppm", "pgm", "pbm",
];
pub const TRACKER_EXT: &[&str] = &["mod", "xm", "it", "s3m", "mptm", "mo3", "okt", "umx", "669", "far", "mtm"];
pub const AUDIO_EXT: &[&str] = &["mp3", "flac", "wav", "ogg", "opus", "aac", "m4a"];
pub const PLAYLIST_EXT: &[&str] = &["pls"];
pub const SCENE_EXT: &str = "gscene";

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// `$XDG_CONFIG_HOME/gravity_engine` (falls back to `~/.config/gravity_engine`).
pub fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home().map(|h| h.join(".config")))
        .map(|p| p.join("gravity_engine"))
}

/// Where screenshots are written: `~/Pictures/gravity_engine`, or the config dir.
pub fn screenshot_dir() -> Option<PathBuf> {
    let pics = home().map(|h| h.join("Pictures"));
    match pics {
        Some(p) if p.is_dir() => Some(p.join("gravity_engine")),
        _ => config_dir().map(|c| c.join("screenshots")),
    }
}

/// Default folder for scene files.
pub fn scene_dir() -> Option<PathBuf> {
    config_dir().map(|c| c.join("scenes"))
}

/// Lower-case extension of a path-like string.
pub fn ext_of(path: &str) -> String {
    std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// File name component of a path-like string (or the whole string).
pub fn file_name_of(path: &str) -> String {
    std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path).to_string()
}
