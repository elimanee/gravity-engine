//! The player's own Sonic levels: scene files with a level in them, made
//! in the level editor and kept in `~/.config/gravity_engine/levels/`.

use crate::config::SCENE_EXT;
use crate::scene::{self, SceneFile};
use std::path::{Path, PathBuf};

/// The levels in the levels folder, by name: (file, level name).
pub fn list() -> Vec<(PathBuf, String)> {
    let Some(dir) = crate::config::level_dir() else { return vec![] };
    let Ok(entries) = std::fs::read_dir(&dir) else { return vec![] };
    let mut out: Vec<(PathBuf, String)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == SCENE_EXT))
        .filter_map(|p| {
            let sc = scene::read(&p).ok()?;
            let name = sc.game.map(|g| g.name).filter(|n| !n.trim().is_empty());
            let name =
                name.unwrap_or_else(|| p.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned()));
            Some((p, name))
        })
        .collect();
    out.sort_by_key(|(_, name)| name.to_lowercase());
    out
}

/// A file name for a new level called `name` that is not taken yet.
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
    let slug = if slug.is_empty() { "level".to_string() } else { slug.chars().take(40).collect() };
    let mut path = dir.join(format!("{slug}.{SCENE_EXT}"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{slug}-{n}.{SCENE_EXT}"));
        n += 1;
    }
    path
}

/// Save a level (creating the folder).
pub fn write(path: &Path, sc: &SceneFile) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    scene::write(path, sc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_paths_do_not_overwrite() {
        let dir = std::env::temp_dir().join(format!("ge-levels-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = free_path(&dir, "Green Meadow!");
        assert!(a.file_name().unwrap().to_string_lossy().starts_with("green-meadow."));
        std::fs::write(&a, "{}").unwrap();
        let b = free_path(&dir, "Green Meadow!");
        assert_ne!(a, b);
        assert!(free_path(&dir, "  ").file_name().unwrap().to_string_lossy().starts_with("level."));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
