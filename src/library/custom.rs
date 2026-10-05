//! Challenges made in the editor, saved as `.gchallenge` files: a scene
//! with the golden ball and a goal, plus a name and an ink budget.

use crate::config::{CHALLENGE_EXT, PPM};
use crate::scene::SceneFile;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FORMAT: &str = "gravity-engine-challenge";
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone)]
pub struct ChallengeFile {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub goal: String,
    /// Length of line (px) the player may draw.
    pub ink: f32,
    /// Size of the world (px) the scene was made in.
    pub arena: [f32; 2],
    pub scene: SceneFile,
}

impl ChallengeFile {
    pub fn new(name: &str, ink: f32, arena: (f32, f32), scene: SceneFile) -> Self {
        let name = name.trim();
        ChallengeFile {
            format: FORMAT.into(),
            version: VERSION,
            name: if name.is_empty() { "Untitled challenge".into() } else { name.into() },
            goal: "Get the ball into the goal".into(),
            ink,
            arena: [arena.0, arena.1],
            scene,
        }
    }

    /// The scene for a world of `aw × ah` px: centred horizontally and
    /// resting on the same floor, like the built-in levels.
    pub fn scene_for(&self, aw: f32, _ah: f32) -> SceneFile {
        let dx = (aw - self.arena[0]) / 2.0 / PPM;
        let mut sc = self.scene.clone();
        if dx.abs() > 1e-4 {
            for o in &mut sc.objects {
                o.x += dx;
            }
            for l in sc.links.iter_mut().filter(|l| l.b.is_none()) {
                l.lb[0] += dx;
            }
            for z in &mut sc.zones {
                z.min[0] += dx;
                z.max[0] += dx;
            }
        }
        sc
    }

    pub fn read(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let c: ChallengeFile = serde_json::from_slice(&bytes).map_err(|e| format!("invalid challenge: {e}"))?;
        if c.format != FORMAT {
            return Err("not a Gravity Engine challenge".into());
        }
        if c.version > VERSION {
            return Err(format!("challenge version {} is newer than supported ({VERSION})", c.version));
        }
        if !c.ink.is_finite() || c.ink <= 0.0 {
            return Err("invalid ink budget".into());
        }
        Ok(c)
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
}

/// A file name for `name` in `dir` that is not taken yet.
pub fn free_path(dir: &Path, name: &str) -> PathBuf {
    let slug: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if slug.is_empty() { "challenge".to_string() } else { slug.chars().take(40).collect() };
    let mut path = dir.join(format!("{slug}.{CHALLENGE_EXT}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{slug}-{n}.{CHALLENGE_EXT}"));
        n += 1;
    }
    path
}

/// Every readable challenge in `dir`, sorted by name.
pub fn list(dir: &Path) -> Vec<(PathBuf, ChallengeFile)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<(PathBuf, ChallengeFile)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| crate::config::ext_of(&p.to_string_lossy()) == CHALLENGE_EXT)
        .filter_map(|p| ChallengeFile::read(&p).ok().map(|c| (p, c)))
        .collect();
    out.sort_by_key(|a| a.1.name.to_lowercase());
    out
}

/// Stars for a solve that used `used` of the ink (0‥1): ★★★ with half the
/// ink or less, ★★ up to 80 %, ★ otherwise.
pub fn stars(used: f32) -> u8 {
    if used <= 0.5 {
        3
    } else if used <= 0.8 {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;
    use crate::physics::zones::{Zone, ZoneKind};

    fn sample() -> ChallengeFile {
        let scene = SceneFile {
            version: 3,
            gravity: -9.8,
            border: BorderMode::Walls,
            objects: vec![],
            links: vec![],
            water: None,
            gadgets: vec![],
            zones: vec![Zone {
                kind: ZoneKind::Goal,
                min: [1.0, 0.0],
                max: [2.0, 1.0],
                angle: 0.0,
                strength: 0.0,
                pair: None,
            }],
            night: None,
            weather: None,
        };
        ChallengeFile::new("  My level! ", 500.0, (1100.0, 720.0), scene)
    }

    #[test]
    fn challenge_files_round_trip_and_recentre() {
        let dir = std::env::temp_dir().join(format!("ge-challenges-{}", std::process::id()));
        let c = sample();
        assert_eq!(c.name, "My level!");
        let path = free_path(&dir, &c.name);
        assert!(path.ends_with("my-level.gchallenge"), "{path:?}");
        c.write(&path).unwrap();
        assert!(free_path(&dir, &c.name).ends_with("my-level-2.gchallenge"));
        let back = ChallengeFile::read(&path).unwrap();
        assert_eq!((back.name.as_str(), back.ink), ("My level!", 500.0));
        assert_eq!(list(&dir).len(), 1);
        // A wider world moves everything right by half the difference.
        let sc = back.scene_for(1100.0 + 2.0 * PPM, 720.0);
        assert!((sc.zones[0].min[0] - 2.0).abs() < 1e-5);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn stars_follow_the_ink_used() {
        assert_eq!([stars(0.2), stars(0.5), stars(0.51), stars(0.8), stars(0.95)], [3, 3, 2, 2, 1]);
    }
}
