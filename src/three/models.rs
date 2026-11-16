//! Model packs (Sonic Robo Blast 2's MD3 packs): read at run time from the
//! player's own files, never shipped with the game. Put a pack's
//! `models.dat` and `models/` folder in `~/.config/gravity_engine/sonic/`
//! (or in `…/sonic/models/`); what is found replaces the characters,
//! badniks, rings and springs built from simple shapes.
//!
//! A pack's textures are drawn in a neutral colour with a `_blend` map
//! saying where the character's own colour goes and how light; the colour
//! is put on here.

use super::md3::{self, Anim, Md3};
use crate::sonic::Character;
use macroquad::models::Vertex;
use macroquad::prelude::*;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// Metres per model unit (Sonic's model is about 40 units tall, and stands
/// as tall as the characters' collision, about 1.5 m).
const METRES: f32 = 1.52 / 39.7;
/// The scale packs give most models (models.dat), which is 1:1 here.
const BASE_SCALE: f32 = 3.0;

/// The colour a character's `_blend` map puts on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ramp {
    pub main: Color,
}

impl Ramp {
    pub fn of(c: Character) -> Ramp {
        let main = match c {
            Character::Sonic => Color::new(0.1, 0.32, 0.92, 1.0),
            Character::Tails => Color::new(0.98, 0.62, 0.15, 1.0),
            Character::Knuckles => Color::new(0.86, 0.14, 0.16, 1.0),
            Character::Amy => Color::new(1.0, 0.5, 0.7, 1.0),
            Character::Exe => Color::new(0.05, 0.13, 0.42, 1.0),
        };
        Ramp { main }
    }
}

/// Put the character's colour on the RGBA pixels `base` where the RGBA
/// `blend` map has something, as SRB2 does: the map's grey picks black (0),
/// the colour (127) or white (255) and shades between; its opacity says
/// how much.
pub fn recolour(base: &mut [u8], blend: &[u8], ramp: Ramp) {
    let main = ramp.main;
    for (px, b) in base.as_chunks_mut::<4>().0.iter_mut().zip(blend.as_chunks::<4>().0) {
        let amount = b[3] as f32 / 255.0;
        if amount <= 0.0 {
            continue;
        }
        let grey = b[0] as f32;
        let colour = (254.0 - (grey - 127.0).abs() * 2.0).clamp(0.0, 255.0) / 255.0;
        let white = ((grey - 127.0) * 2.0).clamp(0.0, 255.0) / 255.0;
        for (k, v) in [main.r, main.g, main.b].into_iter().enumerate() {
            let was = px[k] as f32 / 255.0;
            let want = (white + colour * v).min(1.0);
            px[k] = ((was + (want - was) * amount) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// A surface ready to draw: its triangles, texture coordinates, and
/// each frame's positions and normals (metres, x forward, y up, z right).
struct Part {
    uv: Vec<Vec2>,
    frames: Vec<Vec<(Vec3, Vec3)>>,
    /// Cut to fit draw calls: the vertices each piece uses, and its
    /// triangles in those.
    pieces: Vec<(Vec<u32>, Vec<u16>)>,
}

/// Triangles cut into pieces of at most `most` indices (each piece's own
/// vertices numbered from 0).
fn cut(tris: &[[u32; 3]], most: usize) -> Vec<(Vec<u32>, Vec<u16>)> {
    let mut out = vec![];
    let mut verts: Vec<u32> = vec![];
    let mut idx: Vec<u16> = vec![];
    let mut map = std::collections::HashMap::new();
    for t in tris {
        if idx.len() + 3 > most || verts.len() + 3 > u16::MAX as usize {
            out.push((std::mem::take(&mut verts), std::mem::take(&mut idx)));
            map.clear();
        }
        for &k in t {
            let i = *map.entry(k).or_insert_with(|| {
                verts.push(k);
                verts.len() - 1
            });
            idx.push(i as u16);
        }
    }
    if !idx.is_empty() {
        out.push((verts, idx));
    }
    out
}

/// A model loaded and coloured.
pub struct Model {
    parts: Vec<Part>,
    pub anims: BTreeMap<String, Anim>,
    texture: Option<Texture2D>,
    /// Frame 0, made once (for things that do not animate).
    still: Vec<Mesh>,
    pub height: f32,
}

/// The model's axes (x forward, y left, z up) to ours, scaled.
fn ours(p: Vec3, k: f32) -> Vec3 {
    vec3(p.x, p.z, -p.y) * k
}

impl Model {
    /// From an MD3 and its texture's pixels (RGBA, `size`), `scale` as in
    /// models.dat.
    pub fn new(m: &Md3, texture: Option<Texture2D>, scale: f32) -> Option<Model> {
        let k = METRES * scale / BASE_SCALE;
        let mut parts = vec![];
        for s in &m.surfaces {
            if s.frames.is_empty() || s.tris.iter().flatten().any(|&i| i as usize >= s.uv.len()) {
                continue;
            }
            parts.push(Part {
                pieces: cut(&s.tris, 4800),
                uv: s.uv.clone(),
                frames: s
                    .frames
                    .iter()
                    .map(|f| f.iter().map(|(p, n)| (ours(*p, k), ours(*n, 1.0))).collect())
                    .collect(),
            });
        }
        if parts.is_empty() {
            return None;
        }
        let mut anims = m.animations();
        poses(&mut anims);
        let mut model = Model { parts, anims, texture, still: vec![], height: m.height() * k };
        model.still = model.meshes(0, 0, 0.0);
        Some(model)
    }

    fn meshes(&self, a: usize, b: usize, t: f32) -> Vec<Mesh> {
        let mut out = vec![];
        for p in &self.parts {
            let fa = &p.frames[a.min(p.frames.len() - 1)];
            let fb = &p.frames[b.min(p.frames.len() - 1)];
            for (verts, indices) in &p.pieces {
                let vertices = verts
                    .iter()
                    .map(|&i| {
                        let i = i as usize;
                        let (pa, na) = fa[i];
                        let (pb, nb) = fb[i];
                        let n = na.lerp(nb, t).normalize_or_zero();
                        Vertex {
                            position: pa.lerp(pb, t),
                            uv: p.uv[i],
                            color: [255; 4],
                            normal: vec4(n.x, n.y, n.z, 0.0),
                        }
                    })
                    .collect();
                out.push(Mesh { vertices, indices: indices.clone(), texture: self.texture.clone() });
            }
        }
        out
    }

    /// Draw its first frame at `place`.
    pub fn draw_still(&self, place: Mat4) {
        super::gfx::draw_at(&self.still, place);
    }

    /// Which frames to show for `anim` at `time` (frames, going on
    /// growing): two neighbours and how far between, when it blends.
    pub fn frames(&self, anim: &str, time: f32) -> Option<(usize, usize, f32)> {
        let a = self.anims.get(anim)?;
        let n = a.frames.len();
        let t = time.max(0.0);
        let i = t.floor() as usize % n;
        let j = (i + 1) % n;
        let f = if a.blend { t.fract() } else { 0.0 };
        Some((a.frames[i], a.frames[j], f))
    }

    /// Draw `anim` (or the first of its stand-ins it has) at `time`.
    pub fn draw_anim(&self, place: Mat4, anim: &str, time: f32) {
        let mut name = anim;
        let mut tries = 0;
        let found = loop {
            if let Some(f) = self.frames(name, time) {
                break Some(f);
            }
            tries += 1;
            match instead(name) {
                Some(next) if tries < 6 => name = next,
                _ => break None,
            }
        };
        // A model with only animations of its own plays its first.
        let found = found.or_else(|| self.anims.keys().next().and_then(|k| self.frames(k, time)));
        let (a, b, t) = found.unwrap_or((0, 0, 0.0));
        super::gfx::draw_at(&self.meshes(a, b, t), place);
    }

    pub fn has(&self, anim: &str) -> bool {
        self.anims.contains_key(anim)
    }

    /// Its ball is one still pose (so it is spun round as it goes).
    pub fn still_ball(&self) -> bool {
        self.anims.get("ROLL").is_some_and(|a| a.frames.len() == 1)
    }
}

/// Sonic.exe's model (2011X's, "EXE_") is forty poses in one go rather
/// than animations: which are which.
fn poses(anims: &mut BTreeMap<String, Anim>) {
    let Some(all) = anims.get("EXE_").filter(|a| a.frames.len() >= 14 && anims.len() == 1).cloned() else { return };
    let pick = |ks: &[usize], blend: bool| Anim { frames: ks.iter().map(|&k| all.frames[k]).collect(), blend };
    for (name, ks, blend) in [
        ("STND", &[13][..], false),
        ("WAIT", &[5, 6], true),
        ("WALK", &[5, 6], true),
        ("RUN_", &[11, 12], true),
        ("DASH", &[0, 2], true),
        ("ROLL", &[1], false),
        ("SPIN", &[10], false),
        ("SPNG", &[3], false),
        ("FALL", &[4], false),
        ("SKID", &[9], false),
        ("PAIN", &[8], false),
    ] {
        anims.insert(name.to_string(), pick(ks, blend));
    }
}

/// What to show when a model has not got an animation.
fn instead(anim: &str) -> Option<&'static str> {
    Some(match anim {
        "DASH" => "RUN_",
        "RUN_" => "WALK",
        "WALK" => "STND",
        "WAIT" => "STND",
        "SPIN" => "ROLL",
        "FLY_" => "SPNG",
        "GLID" => "FALL",
        "MLEE" => "STND",
        "SKID" => "WALK",
        "PAIN" => "FALL",
        "FALL" => "SPNG",
        "SPNG" => "STND",
        "ROLL" => "STND",
        _ => return None,
    })
}

/// Everything a pack gave.
#[derive(Default)]
pub struct Pack {
    pub character: Option<Model>,
    pub crawler: Option<Model>,
    pub flyer: Option<Model>,
    pub ring: Option<Model>,
    pub spring: Option<Model>,
    pub spring_red: Option<Model>,
}

impl Pack {
    pub fn count(&self) -> usize {
        [&self.character, &self.crawler, &self.flyer, &self.ring, &self.spring, &self.spring_red]
            .iter()
            .filter(|m| m.is_some())
            .count()
    }
}

/// The lines of a models.dat: name → (path, scale).
pub fn parse_dat(text: &str) -> HashMap<String, (String, f32)> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let mut w = line.split_whitespace();
        let (Some(name), Some(path)) = (w.next(), w.next()) else { continue };
        if name.starts_with("//") || name.starts_with('#') || !path.to_ascii_lowercase().ends_with(".md3") {
            continue;
        }
        let scale = w.next().and_then(|s| s.parse::<f32>().ok()).filter(|s| *s > 0.0).unwrap_or(BASE_SCALE);
        out.insert(name.to_ascii_uppercase(), (path.replace('\\', "/"), scale));
    }
    out
}

/// Where packs are looked for.
fn roots() -> Vec<PathBuf> {
    let Some(base) = crate::config::config_dir().map(|c| c.join("sonic")) else { return vec![] };
    vec![base.clone(), base.join("models")]
}

/// A model's file, its scale and its texture's files, by name.
fn find(name: &str, fallback: &str) -> Option<(PathBuf, f32)> {
    for root in roots() {
        let dat = std::fs::read_to_string(root.join("models.dat")).map(|t| parse_dat(&t)).unwrap_or_default();
        let (path, scale) = dat.get(name).cloned().unwrap_or((fallback.to_string(), BASE_SCALE));
        for dir in [root.join("models"), root.clone()] {
            let p = dir.join(&path);
            if p.is_file() {
                return Some((p, scale));
            }
        }
    }
    None
}

fn texture(md3_path: &Path, shader: &str, ramp: Option<Ramp>, glow: f32) -> Option<Texture2D> {
    // The model's own name, else the file its surfaces name.
    let png = std::fs::read(md3_path.with_extension("png"))
        .or_else(|_| std::fs::read(md3_path.with_file_name(Path::new(shader).file_name().unwrap_or_default())))
        .ok()?;
    let mut img = Image::from_file_with_format(&png, Some(ImageFormat::Png)).ok()?;
    if glow != 1.0 {
        for px in img.bytes.as_chunks_mut::<4>().0 {
            for v in &mut px[..3] {
                *v = (*v as f32 * glow).min(255.0) as u8;
            }
        }
    }
    if let Some(ramp) = ramp {
        let stem = md3_path.file_stem()?.to_string_lossy().to_string();
        let blend = md3_path.with_file_name(format!("{stem}_blend.png"));
        if let Some(b) =
            std::fs::read(blend).ok().and_then(|d| Image::from_file_with_format(&d, Some(ImageFormat::Png)).ok())
        {
            if b.width == img.width && b.height == img.height {
                recolour(&mut img.bytes, &b.bytes, ramp);
            }
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Linear);
    Some(t)
}

fn load_one(name: &str, fallback: &str, ramp: Option<Ramp>) -> Option<Model> {
    let (path, scale) = find(name, fallback)?;
    let data = std::fs::read(&path).ok()?;
    let m = md3::parse(&data).ok()?;
    // Rings shine.
    let glow = if name == "RING" { 1.45 } else { 1.0 };
    let shader = m.surfaces.first().map_or("", |s| s.shader.as_str());
    Model::new(&m, texture(&path, shader, ramp, glow), scale)
}

/// The models for `character` and the world's things (none on the web).
pub fn load(character: Character) -> Pack {
    if cfg!(target_arch = "wasm32") {
        return Pack::default();
    }
    // Sonic.exe: his own model when there is one (a pack's EXE_), else
    // Sonic's, darker.
    let exe = matches!(character, Character::Exe).then(|| load_one("EXE_", "EXE_/RAGE.md3", None)).flatten();
    let (name, file) = match character {
        Character::Sonic | Character::Exe => ("SONIC", "PLAY/SONIC.md3"),
        Character::Tails => ("TAILS", "PLAY/TAILS.md3"),
        Character::Knuckles => ("KNUCKLES", "PLAY/KNUCKLES.md3"),
        Character::Amy => ("AMY", "PLAY/AMY.md3"),
    };
    Pack {
        character: exe.or_else(|| load_one(name, file, Some(Ramp::of(character)))),
        crawler: load_one("POSS", "ENMY/POSS.md3", None),
        flyer: load_one("BUZZ", "ENMY/BUZZ.md3", None),
        ring: load_one("RING", "OBJE/RING.md3", None),
        spring: load_one("SPRY", "OBJE/SPRY.md3", None),
        spring_red: load_one("SPRR", "OBJE/SPRR.md3", None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_surfaces_are_cut_to_fit_draw_calls() {
        let tris: Vec<[u32; 3]> = (0..5000u32).map(|k| [k, k + 1, k + 2]).collect();
        let pieces = cut(&tris, 4800);
        assert!(pieces.len() > 1 && pieces.iter().all(|(_, i)| i.len() <= 4800));
        // Every triangle comes back the same.
        let back: Vec<[u32; 3]> = pieces
            .iter()
            .flat_map(|(v, i)| {
                i.chunks(3).map(|t| [v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]]).collect::<Vec<_>>()
            })
            .collect();
        assert_eq!(back, tris);
    }

    #[test]
    fn models_dat_lines_are_read() {
        let d = parse_dat(
            "SONIC PLAY/SONIC.md3 3.0 0.0\n// a note\nRSPR OBJE\\RSPR.md3 2.7 0.0\nBAD line\nTVRI OBJE/TVRI.md3\n",
        );
        assert_eq!(d["SONIC"], ("PLAY/SONIC.md3".to_string(), 3.0));
        assert_eq!(d["RSPR"], ("OBJE/RSPR.md3".to_string(), 2.7));
        assert_eq!(d["TVRI"].1, BASE_SCALE);
        assert_eq!(d.len(), 3);
    }

    #[test]
    fn the_blend_map_puts_the_characters_colour_on() {
        // Two pixels: one the blend map leaves, one it colours fully, light.
        let mut base = vec![0, 200, 0, 255, 0, 200, 0, 255];
        let blend = vec![0, 0, 0, 0, 255, 255, 255, 255];
        let ramp = Ramp::of(Character::Sonic);
        recolour(&mut base, &blend, ramp);
        assert_eq!(&base[..4], &[0, 200, 0, 255], "left alone");
        assert_eq!(&base[4..], &[255, 255, 255, 255], "white in the map: white");
        // Mid-grey: his colour; darker: darker; half see-through: half way.
        let main = ramp.main;
        let mut px = vec![0, 200, 0, 255];
        recolour(&mut px, &[127, 127, 127, 255], ramp);
        let near = |a: u8, b: f32| (a as f32 - b * 255.0).abs() < 3.0;
        assert!(near(px[0], main.r) && near(px[1], main.g) && near(px[2], main.b), "{px:?}");
        let mut px = vec![0, 0, 0, 255];
        recolour(&mut px, &[64, 64, 64, 255], ramp);
        assert!(near(px[2], main.b * 0.5), "{px:?}");
        let mut px = vec![0, 0, 0, 255];
        recolour(&mut px, &[127, 127, 127, 128], ramp);
        assert!(near(px[2], main.b * 0.5), "{px:?}");
    }

    #[test]
    fn a_model_turns_into_our_axes_and_plays_its_frames() {
        let data = md3::tests::sample(&["SPR2_WALK+i00", "SPR2_WALK+i01", "SPR2_STND00"]);
        let m = md3::parse(&data).unwrap();
        let model = Model::new(&m, None, BASE_SCALE).unwrap();
        // The sample's triangle is lifted 1 unit (its z, up) a frame: up is our y.
        let p0 = model.parts[0].frames[0][0].0;
        let p1 = model.parts[0].frames[1][0].0;
        assert!((p1.y - p0.y - METRES).abs() < 1e-5 && (p1.x - p0.x).abs() < 1e-6, "{p0:?} {p1:?}");
        // Walking blends from one frame to the next, round and round.
        assert_eq!(model.frames("WALK", 0.25), Some((0, 1, 0.25)));
        assert_eq!(model.frames("WALK", 1.5), Some((1, 0, 0.5)));
        assert_eq!(model.frames("STND", 7.0), Some((2, 2, 0.0)));
        assert!(model.frames("RUN_", 0.0).is_none() && instead("RUN_") == Some("WALK"));
        // A model of poses (Sonic.exe's) gets animations made of them.
        let names: Vec<String> = (0..40).map(|k| format!("SPR2_EXE_{k:02}")).collect();
        let data = md3::tests::sample(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let exe = Model::new(&md3::parse(&data).unwrap(), None, 1.2).unwrap();
        assert_eq!(exe.frames("STND", 0.0), Some((13, 13, 0.0)));
        assert_eq!(exe.frames("RUN_", 0.5), Some((11, 12, 0.5)));
        assert!(exe.still_ball());
    }
}
