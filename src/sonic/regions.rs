//! What a player has made of a character's sheet with the sprite region
//! editor: pictures moved, resized, recentred, added, removed, mirrored or
//! held longer, animations' pace, pieces in front and behind and how they
//! turn, the character's colours changed (always, and when Super), and for
//! a sheet the game does not know, its background colours and scale. They are kept beside the sheet in
//! `<character>.regions.json`, replace the layout's own animations of the
//! same names, and only count for a sheet of the size they were made on
//! (another sheet of that character ignores them).

use super::sheet::{Anim, Pic, Repeat, Swap, Turn};
use super::Character;
use macroquad::prelude::{vec2, Rect};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(v: &usize) -> bool {
    *v == 0
}

fn one() -> f32 {
    1.0
}

fn is_one(v: &f32) -> bool {
    *v == 1.0
}

fn is_ground(t: &Turn) -> bool {
    *t == Turn::Ground
}

/// A picture: where it is on the sheet, where the character's centre is
/// in it, whether it is mirrored and how long it shows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Region {
    /// x, y, width, height on the sheet (px).
    pub rect: [f32; 4],
    /// The character's centre, from the picture's top-left (px).
    pub centre: [f32; 2],
    #[serde(default, skip_serializing_if = "is_false")]
    pub flip: bool,
    /// In pictures: 2 shows twice as long as the others.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub hold: f32,
}

/// An animation's pictures in order, the pieces drawn behind them (Tails'
/// tails) and in front, its pace (pictures a second; none: the games'),
/// whether it plays once, the picture it loops back to (from 0, the first:
/// the ones before it open the animation once), and how it turns with the
/// ground.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnimRegions {
    pub frames: Vec<Region>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub behind: Vec<Region>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub front: Vec<Region>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fps: Option<f32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub once: bool,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub loop_from: usize,
    /// Runs of pictures played several times over on the way.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repeats: Vec<Repeat>,
    #[serde(default, skip_serializing_if = "is_ground")]
    pub turn: Turn,
}

impl AnimRegions {
    fn of(a: &Anim) -> AnimRegions {
        AnimRegions {
            frames: a.pics.iter().map(to_region).collect(),
            behind: a.behind.iter().map(to_region).collect(),
            front: a.front.iter().map(to_region).collect(),
            fps: a.fps,
            once: a.once,
            loop_from: a.loop_from,
            repeats: a.repeats.clone(),
            turn: a.turn,
        }
    }

    fn anim(&self) -> Anim {
        Anim {
            pics: self.frames.iter().map(to_pic).collect(),
            behind: self.behind.iter().map(to_pic).collect(),
            front: self.front.iter().map(to_pic).collect(),
            fps: self.fps,
            once: self.once,
            loop_from: self.loop_from,
            repeats: self.repeats.clone(),
            turn: self.turn,
        }
    }
}

/// A `<character>.regions.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Regions {
    /// The size of the sheet they were made on.
    pub size: [u32; 2],
    /// Background colours cut out besides the layout's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub background: Vec<[u8; 3]>,
    /// Cut them out by closeness (lossy pictures), not only the exact colours.
    #[serde(default, skip_serializing_if = "is_false")]
    pub soft: bool,
    /// Sheet pixels per guide pixel, when not the layout's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scale: Option<f32>,
    /// Colours of the sheet drawn as others: always, and when Super.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recolour: Vec<Swap>,
    #[serde(default, rename = "super", skip_serializing_if = "Vec::is_empty")]
    pub super_colours: Vec<Swap>,
    /// The animations changed (an empty one: the sheet has none of it).
    #[serde(default)]
    pub animations: BTreeMap<String, AnimRegions>,
}

fn to_region(p: &Pic) -> Region {
    Region { rect: [p.src.x, p.src.y, p.src.w, p.src.h], centre: [p.centre.x, p.centre.y], flip: p.flip, hold: p.hold }
}

fn to_pic(r: &Region) -> Pic {
    let [x, y, w, h] = r.rect;
    Pic { src: Rect::new(x, y, w, h), centre: vec2(r.centre[0], r.centre[1]), flip: r.flip, hold: r.hold }
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
                animations.insert(name.clone(), AnimRegions::of(now));
            }
        }
        Regions { size: [size.0, size.1], animations, ..Default::default() }
    }

    /// Nothing differs from the sheet's own layout.
    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
            && self.background.is_empty()
            && !self.soft
            && self.scale.is_none()
            && self.recolour.is_empty()
            && self.super_colours.is_empty()
    }

    /// Put these in place of the layout's animations of the same names.
    pub fn apply(&self, anims: &mut HashMap<String, Anim>) {
        for (name, a) in &self.animations {
            anims.insert(name.clone(), a.anim());
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
        if self.is_empty() {
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
                .map(|r| {
                    let mut line =
                        format!("        {{\"rect\": [{}], \"centre\": [{}]", list(&r.rect), list(&r.centre));
                    if r.flip {
                        line += ", \"flip\": true";
                    }
                    if r.hold != 1.0 {
                        line += &format!(", \"hold\": {}", num(r.hold));
                    }
                    line + "}"
                })
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
                let mut fields = vec![format!("\"frames\": {}", regions(&a.frames))];
                if !a.behind.is_empty() {
                    fields.push(format!("\"behind\": {}", regions(&a.behind)));
                }
                if !a.front.is_empty() {
                    fields.push(format!("\"front\": {}", regions(&a.front)));
                }
                if let Some(fps) = a.fps {
                    fields.push(format!("\"fps\": {}", num(fps)));
                }
                if a.once {
                    fields.push("\"once\": true".to_string());
                }
                if a.loop_from > 0 {
                    fields.push(format!("\"loop_from\": {}", a.loop_from));
                }
                if !a.repeats.is_empty() {
                    let runs: Vec<String> = a
                        .repeats
                        .iter()
                        .map(|r| format!("{{\"from\": {}, \"to\": {}, \"times\": {}}}", r.from, r.to, r.times))
                        .collect();
                    fields.push(format!("\"repeats\": [{}]", runs.join(", ")));
                }
                if a.turn != Turn::Ground {
                    fields.push(format!("\"turn\": {}", serde_json::to_string(&a.turn).unwrap_or_default()));
                }
                format!("    {name}: {{\n      {}\n    }}", fields.join(",\n      "))
            })
            .collect();
        let mut top = vec![format!("\"size\": [{}, {}]", self.size[0], self.size[1])];
        if !self.background.is_empty() {
            let colours: Vec<String> = self.background.iter().map(|[r, g, b]| format!("[{r}, {g}, {b}]")).collect();
            top.push(format!("\"background\": [{}]", colours.join(", ")));
        }
        if self.soft {
            top.push("\"soft\": true".to_string());
        }
        if let Some(scale) = self.scale {
            top.push(format!("\"scale\": {}", num(scale)));
        }
        let swaps = |ss: &[Swap]| {
            let pairs: Vec<String> =
                ss.iter().map(|[[a, b, c], [d, e, f]]| format!("[[{a}, {b}, {c}], [{d}, {e}, {f}]]")).collect();
            format!("[\n    {}\n  ]", pairs.join(",\n    "))
        };
        if !self.recolour.is_empty() {
            top.push(format!("\"recolour\": {}", swaps(&self.recolour)));
        }
        if !self.super_colours.is_empty() {
            top.push(format!("\"super\": {}", swaps(&self.super_colours)));
        }
        top.push(format!("\"animations\": {{\n{}\n  }}", anims.join(",\n")));
        format!("{{\n  {}\n}}\n", top.join(",\n  "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pic(x: f32, cx: f32) -> Pic {
        Pic::new(Rect::new(x, 2.0, 30.0, 40.0), vec2(cx, 21.0))
    }

    fn anim(pics: Vec<Pic>) -> Anim {
        Anim { pics, ..Default::default() }
    }

    #[test]
    fn only_changed_animations_are_kept_and_come_back() {
        let mut defaults = HashMap::new();
        defaults.insert("idle".to_string(), anim(vec![pic(0.0, 15.0)]));
        defaults.insert("walk".to_string(), anim(vec![pic(40.0, 15.0), pic(80.0, 15.0)]));
        defaults.insert("bored".to_string(), anim(vec![pic(120.0, 15.0)]));
        defaults.insert("run".to_string(), anim(vec![pic(240.0, 15.0)]));
        let mut anims = defaults.clone();
        anims.get_mut("walk").unwrap().pics[1].centre.x = 16.5;
        anims.get_mut("bored").unwrap().pics.clear();
        let dash = Anim {
            pics: vec![pic(160.0, 15.0)],
            behind: vec![pic(200.0, 9.0)],
            front: vec![pic(240.0, 5.0)],
            turn: Turn::Steps,
            ..Default::default()
        };
        anims.insert("spindash".to_string(), dash);
        // Paced, played once, a picture mirrored and held twice as long.
        anims.get_mut("walk").unwrap().loop_from = 1;
        anims.get_mut("walk").unwrap().repeats = vec![Repeat { from: 0, to: 1, times: 8 }];
        let run = anims.get_mut("run").unwrap();
        (run.fps, run.once) = (Some(12.0), true);
        (run.pics[0].flip, run.pics[0].hold) = (true, 2.0);
        let mut r = Regions::changes((640, 480), &defaults, &anims);
        assert_eq!(r.animations.keys().collect::<Vec<_>>(), ["bored", "run", "spindash", "walk"]);
        assert!(r.animations["bored"].frames.is_empty(), "a cleared animation is kept as empty");
        (r.background, r.soft, r.scale) = (vec![[0, 128, 128]], true, Some(2.0));
        (r.recolour, r.super_colours) = (vec![[[0, 0, 255], [200, 0, 0]]], vec![[[0, 0, 255], [255, 220, 0]]]);
        // Through the file's text and back over the layout.
        let text = r.to_json();
        assert!(text.contains(r#"{"rect": [160, 2, 30, 40], "centre": [15, 21]}"#), "{text}");
        assert!(text.contains(r#""centre": [16.5, 21]"#), "{text}");
        assert!(text.contains(r#""centre": [15, 21], "flip": true, "hold": 2}"#), "{text}");
        assert!(text.contains(r#""fps": 12"#) && text.contains(r#""once": true"#), "{text}");
        assert!(text.contains(r#""loop_from": 1"#), "{text}");
        assert!(text.contains(r#""repeats": [{"from": 0, "to": 1, "times": 8}]"#), "{text}");
        assert!(text.contains(r#""background": [[0, 128, 128]]"#) && text.contains(r#""scale": 2"#), "{text}");
        assert!(text.contains(r#""front": ["#) && text.contains(r#""turn": "steps""#), "{text}");
        assert!(
            text.contains(
                r#""recolour": [
    [[0, 0, 255], [200, 0, 0]]
  ]"#
            ),
            "{text}"
        );
        assert!(text.contains(r#""super": ["#), "{text}");
        let back: Regions = serde_json::from_str(&text).unwrap();
        assert_eq!(back, r);
        let mut applied = defaults.clone();
        back.apply(&mut applied);
        assert_eq!(applied, anims);
        // Nothing changed: nothing to keep.
        assert!(Regions::changes((640, 480), &anims, &applied).is_empty());
    }

    #[test]
    fn regions_files_are_named_after_the_character_and_older_ones_still_read() {
        assert_eq!(file_name(Character::Exe), "exe.regions.json");
        assert_eq!(file_name(Character::Knuckles), "knuckles.regions.json");
        let older =
            r#"{"size": [10, 10], "animations": {"idle": {"frames": [{"rect": [0, 0, 5, 5], "centre": [2.5, 3]}]}}}"#;
        let r: Regions = serde_json::from_str(older).unwrap();
        let idle = &r.animations["idle"];
        assert!(idle.behind.is_empty() && idle.fps.is_none() && !idle.once && idle.loop_from == 0);
        assert!(idle.repeats.is_empty() && idle.front.is_empty() && idle.turn == Turn::Ground);
        assert_eq!((idle.frames[0].flip, idle.frames[0].hold), (false, 1.0));
        assert!(r.background.is_empty() && !r.soft && r.scale.is_none());
        assert!(r.recolour.is_empty() && r.super_colours.is_empty());
    }
}
