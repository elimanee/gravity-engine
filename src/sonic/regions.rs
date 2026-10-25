//! The pictures a player has moved, resized, recentred, added or removed on
//! a character's sheet with the sprite region editor. They are kept beside
//! the sheet in `<character>.regions.json` and replace the layout's own
//! animations of the same names, only for a sheet of the size they were
//! made on (another sheet of that character ignores them).

use super::sheet::{Anim, Pic};
use super::Character;
use macroquad::prelude::{vec2, Rect};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

/// A picture: where it is on the sheet, and where the character's centre
/// is in it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Region {
    /// x, y, width, height on the sheet (px).
    pub rect: [f32; 4],
    /// The character's centre, from the picture's top-left (px).
    pub centre: [f32; 2],
}

/// An animation's pictures, in order, and the pieces drawn behind them
/// (Tails' tails).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnimRegions {
    pub frames: Vec<Region>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behind: Vec<Region>,
}

/// A `<character>.regions.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Regions {
    /// The size of the sheet they were made on.
    pub size: [u32; 2],
    /// The animations changed (an empty one: the sheet has none of it).
    pub animations: BTreeMap<String, AnimRegions>,
}

fn to_region(p: &Pic) -> Region {
    Region { rect: [p.src.x, p.src.y, p.src.w, p.src.h], centre: [p.centre.x, p.centre.y] }
}

fn to_pic(r: &Region) -> Pic {
    let [x, y, w, h] = r.rect;
    Pic { src: Rect::new(x, y, w, h), centre: vec2(r.centre[0], r.centre[1]) }
}

/// The file `c`'s regions are kept in.
pub fn file_name(c: Character) -> String {
    format!("{}.regions.json", c.short().to_lowercase())
}

fn path(c: Character) -> Option<PathBuf> {
    super::sheet::dir().map(|d| d.join(file_name(c)))
}

impl Regions {
    /// What differs between a layout's animations and the edited ones.
    pub fn changes(size: (u32, u32), defaults: &HashMap<String, Anim>, anims: &HashMap<String, Anim>) -> Regions {
        let none = Anim::default();
        let mut animations = BTreeMap::new();
        for name in defaults.keys().chain(anims.keys()) {
            let (was, now) = (defaults.get(name).unwrap_or(&none), anims.get(name).unwrap_or(&none));
            if was != now {
                let frames = now.pics.iter().map(to_region).collect();
                let behind = now.behind.iter().map(to_region).collect();
                animations.insert(name.clone(), AnimRegions { frames, behind });
            }
        }
        Regions { size: [size.0, size.1], animations }
    }

    /// Put these in place of the layout's animations of the same names.
    pub fn apply(&self, anims: &mut HashMap<String, Anim>) {
        for (name, a) in &self.animations {
            let pics = a.frames.iter().map(to_pic).collect();
            let behind = a.behind.iter().map(to_pic).collect();
            anims.insert(name.clone(), Anim { pics, behind });
        }
    }

    /// `c`'s regions for a sheet of this size (none: no file, or one made
    /// on another sheet).
    pub fn read(c: Character, size: (u32, u32)) -> Result<Option<Regions>, String> {
        let Some(text) = path(c).and_then(|p| std::fs::read_to_string(p).ok()) else { return Ok(None) };
        let r: Regions = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file_name(c)))?;
        Ok((r.size == [size.0, size.1]).then_some(r))
    }

    /// Keep them as `c`'s (no change left: the file goes).
    pub fn save(&self, c: Character) -> Result<(), String> {
        let p = path(c).ok_or("no config directory")?;
        if self.animations.is_empty() {
            return match std::fs::remove_file(&p) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
                _ => Ok(()),
            };
        }
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        }
        std::fs::write(&p, self.to_json()).map_err(|e| e.to_string())
    }

    /// The file's text: a picture per line, whole numbers without decimals.
    pub fn to_json(&self) -> String {
        let num = |v: f32| {
            if v.fract() == 0.0 && v.abs() < 1e7 {
                format!("{v:.0}")
            } else {
                serde_json::to_string(&v).unwrap_or_else(|_| "0".into())
            }
        };
        let list = |vs: &[f32]| vs.iter().map(|&v| num(v)).collect::<Vec<_>>().join(", ");
        let regions = |rs: &[Region]| {
            let lines: Vec<String> = rs
                .iter()
                .map(|r| format!("        {{\"rect\": [{}], \"centre\": [{}]}}", list(&r.rect), list(&r.centre)))
                .collect();
            if lines.is_empty() {
                "[]".to_string()
            } else {
                format!("[\n{}\n      ]", lines.join(",\n"))
            }
        };
        let anims: Vec<String> = self
            .animations
            .iter()
            .map(|(name, a)| {
                let name = serde_json::to_string(name).unwrap_or_default();
                let behind = if a.behind.is_empty() {
                    String::new()
                } else {
                    format!(",\n      \"behind\": {}", regions(&a.behind))
                };
                format!("    {name}: {{\n      \"frames\": {}{behind}\n    }}", regions(&a.frames))
            })
            .collect();
        format!(
            "{{\n  \"size\": [{}, {}],\n  \"animations\": {{\n{}\n  }}\n}}\n",
            self.size[0],
            self.size[1],
            anims.join(",\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pic(x: f32, cx: f32) -> Pic {
        Pic { src: Rect::new(x, 2.0, 30.0, 40.0), centre: vec2(cx, 21.0) }
    }

    #[test]
    fn only_changed_animations_are_kept_and_come_back() {
        let mut defaults = HashMap::new();
        defaults.insert("idle".to_string(), Anim { pics: vec![pic(0.0, 15.0)], behind: vec![] });
        defaults.insert("walk".to_string(), Anim { pics: vec![pic(40.0, 15.0), pic(80.0, 15.0)], behind: vec![] });
        defaults.insert("bored".to_string(), Anim { pics: vec![pic(120.0, 15.0)], behind: vec![] });
        let mut anims = defaults.clone();
        anims.get_mut("walk").unwrap().pics[1].centre.x = 16.5;
        anims.get_mut("bored").unwrap().pics.clear();
        anims.insert("spindash".to_string(), Anim { pics: vec![pic(160.0, 15.0)], behind: vec![pic(200.0, 9.0)] });
        let r = Regions::changes((640, 480), &defaults, &anims);
        assert_eq!(r.animations.keys().collect::<Vec<_>>(), ["bored", "spindash", "walk"]);
        assert!(r.animations["bored"].frames.is_empty(), "a cleared animation is kept as empty");
        // Through the file's text and back over the layout.
        let text = r.to_json();
        assert!(text.contains(r#"{"rect": [160, 2, 30, 40], "centre": [15, 21]}"#), "{text}");
        assert!(text.contains(r#""centre": [16.5, 21]"#), "{text}");
        let back: Regions = serde_json::from_str(&text).unwrap();
        assert_eq!(back, r);
        let mut applied = defaults.clone();
        back.apply(&mut applied);
        assert_eq!(applied, anims);
        // Nothing changed: nothing to keep.
        assert!(Regions::changes((640, 480), &anims, &applied).animations.is_empty());
    }

    #[test]
    fn regions_files_are_named_after_the_character() {
        assert_eq!(file_name(Character::Exe), "exe.regions.json");
        assert_eq!(file_name(Character::Knuckles), "knuckles.regions.json");
        let behindless =
            r#"{"size": [10, 10], "animations": {"idle": {"frames": [{"rect": [0, 0, 5, 5], "centre": [2.5, 3]}]}}}"#;
        let r: Regions = serde_json::from_str(behindless).unwrap();
        assert!(r.animations["idle"].behind.is_empty());
    }
}
