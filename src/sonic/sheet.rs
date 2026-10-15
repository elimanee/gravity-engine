//! Sonic drawn from a sprite sheet the user provides (none ships with the
//! program): a PNG in `~/.config/gravity_engine/sonic/`, cut up by a
//! `sheet.json` beside it, or by the built-in layout of Triangly's Sonic 1
//! sheet (The Spriters Resource) when the PNG is that sheet.
//!
//! Each frame is a cell whose centre is Sonic's centre, facing right, at
//! one sheet pixel per guide pixel (or `scale`).

use super::Sonic;
use macroquad::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Where the sheet lives.
pub fn dir() -> Option<PathBuf> {
    crate::config::config_dir().map(|c| c.join("sonic"))
}

/// How to cut a sheet up.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layout {
    /// Width and height of a cell (px).
    pub cell: [u32; 2],
    /// Background colours made transparent.
    pub transparent: Vec<[u8; 3]>,
    /// Sheet pixels per guide pixel.
    pub scale: f32,
    /// Top-left corners of each animation's cells, in order.
    pub animations: HashMap<String, Vec<[u32; 2]>>,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { cell: [64, 64], transparent: vec![], scale: 1.0, animations: HashMap::new() }
    }
}

/// Triangly's Sonic 1 sheet (690 × 1558): 66 px cells on dark green, the
/// in-game tile space in a lighter green.
fn sonic1() -> Layout {
    let row = |y: u32, xs: &[u32]| xs.iter().map(|&x| [x, y]).collect::<Vec<_>>();
    let mut a = HashMap::new();
    a.insert("idle".into(), row(243, &[24]));
    a.insert("bored".into(), row(243, &[110, 180, 250, 320]));
    a.insert("look_up".into(), row(243, &[406]));
    a.insert("crouch".into(), row(243, &[492]));
    a.insert("spring".into(), row(243, &[578]));
    a.insert("walk".into(), row(334, &[24, 94, 164, 234, 304, 374]));
    a.insert("skid".into(), row(334, &[460, 530]));
    a.insert("balance".into(), row(425, &[460, 530]));
    a.insert("run".into(), row(516, &[24, 94, 164, 234]));
    a.insert("roll".into(), row(607, &[24, 94, 164, 234, 304]));
    a.insert("push".into(), row(607, &[390, 460, 530, 600]));
    a.insert("hurt".into(), row(789, &[24, 94]));
    Layout { cell: [66, 66], transparent: vec![[13, 72, 7], [37, 102, 26], [67, 153, 49]], scale: 1.0, animations: a }
}

/// A loaded sheet.
pub struct Sheet {
    texture: Texture2D,
    layout: Layout,
    pub name: String,
}

/// The PNG to use in `dir`, and its layout.
fn find(dir: &Path) -> Result<(PathBuf, Option<Layout>), String> {
    let mut pngs: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .collect();
    pngs.sort();
    let png = pngs.into_iter().next().ok_or("no PNG sprite sheet")?;
    let json = dir.join("sheet.json");
    let layout = match std::fs::read_to_string(&json) {
        Ok(text) => Some(serde_json::from_str::<Layout>(&text).map_err(|e| format!("sheet.json: {e}"))?),
        Err(_) => None,
    };
    Ok((png, layout))
}

/// Cut out the background colours.
pub fn clear_background(img: &mut image::RgbaImage, colours: &[[u8; 3]]) {
    for p in img.pixels_mut() {
        if colours.iter().any(|c| c[0] == p[0] && c[1] == p[1] && c[2] == p[2]) {
            p[3] = 0;
        }
    }
}

impl Sheet {
    /// The sheet in the Sonic folder, if there is one.
    pub fn load() -> Result<Option<Sheet>, String> {
        let Some(dir) = dir().filter(|d| d.is_dir()) else { return Ok(None) };
        let (png, layout) = match find(&dir) {
            Ok(found) => found,
            Err(e) if e == "no PNG sprite sheet" => return Ok(None),
            Err(e) => return Err(e),
        };
        let mut img = image::open(&png).map_err(|e| e.to_string())?.to_rgba8();
        let layout = match layout {
            Some(l) => l,
            None if img.dimensions() == (690, 1558) => sonic1(),
            None => return Err("unknown sheet layout: add a sheet.json beside it (see the README)".into()),
        };
        if !layout.animations.contains_key("idle") {
            return Err("the sheet has no \"idle\" animation".into());
        }
        clear_background(&mut img, &layout.transparent);
        let texture = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, &img);
        texture.set_filter(FilterMode::Nearest);
        let name = png.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        Ok(Some(Sheet { texture, layout, name }))
    }

    /// Copy a chosen PNG (and its sheet.json, if any) into the Sonic folder.
    pub fn install(png: &Path) -> Result<(), String> {
        let dir = dir().ok_or("no config directory")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        for old in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
            if old.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("png") || e == "json") {
                let _ = std::fs::remove_file(old.path());
            }
        }
        std::fs::copy(png, dir.join("sheet.png")).map_err(|e| e.to_string())?;
        if let Some(json) = png.parent().map(|p| p.join("sheet.json")).filter(|j| j.is_file()) {
            std::fs::copy(json, dir.join("sheet.json")).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn frames(&self, name: &str) -> Option<&Vec<[u32; 2]>> {
        self.layout.animations.get(name).filter(|f| !f.is_empty())
    }

    /// The animation and frame for Sonic's state.
    fn pick(&self, s: &Sonic) -> [u32; 2] {
        let speed = if s.grounded { s.gsp.abs() } else { s.xsp.abs() };
        let name = if s.is_ball() {
            "roll"
        } else if !s.grounded && s.ysp < 0.0 {
            "spring"
        } else if s.skidding {
            "skid"
        } else if s.pushing {
            "push"
        } else if s.crouching {
            "crouch"
        } else if s.looking_up {
            "look_up"
        } else if s.grounded && speed == 0.0 {
            if s.idle > 180 && self.frames("bored").is_some() {
                "bored"
            } else {
                "idle"
            }
        } else if speed >= 6.0 && self.frames("run").is_some() {
            "run"
        } else {
            "walk"
        };
        let frames = self.frames(name).or_else(|| self.frames("walk")).or_else(|| self.frames("idle"));
        let Some(frames) = frames else { return [0, 0] };
        let k = if name == "bored" {
            ((s.idle - 180) / 30) as usize
        } else if name == "push" {
            (s.anim / 8.0) as usize
        } else {
            s.anim as usize
        };
        frames[k % frames.len()]
    }

    /// Draw Sonic from the sheet (world pass).
    pub fn draw(&self, s: &Sonic) {
        let [x, y] = self.pick(s);
        let [w, h] = self.layout.cell;
        let k = super::SCALE / self.layout.scale.max(0.01);
        let size = vec2(w as f32, h as f32) * k;
        let upright = !s.grounded || (s.gsp == 0.0 && super::mode_down(s.angle).y > 0.5 && s.angle.sin().abs() < 0.4);
        let angle = if upright && s.grounded { 0.0 } else { s.angle };
        draw_texture_ex(
            &self.texture,
            s.pos.x - size.x / 2.0,
            s.pos.y - size.y / 2.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(size),
                source: Some(Rect::new(x as f32, y as f32, w as f32, h as f32)),
                rotation: -angle,
                flip_x: s.facing < 0.0,
                ..Default::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_parse_and_backgrounds_clear() {
        let json = r#"{"cell": [48, 48], "transparent": [[0, 128, 0]], "animations": {"idle": [[0, 0]], "walk": [[48, 0], [96, 0]]}}"#;
        let l: Layout = serde_json::from_str(json).unwrap();
        assert_eq!(l.cell, [48, 48]);
        assert_eq!(l.scale, 1.0, "scale defaults to 1");
        assert_eq!(l.animations["walk"].len(), 2);
        let mut img = image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 128, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([10, 20, 200, 255]));
        clear_background(&mut img, &l.transparent);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(1, 0)[3], 255);
        let s1 = sonic1();
        for (name, frames) in &s1.animations {
            for f in frames {
                assert!(f[0] + 66 <= 690 && f[1] + 66 <= 1558, "{name} cell inside the sheet");
            }
        }
    }
}
