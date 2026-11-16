//! Quake 3 models (`.md3`), as Sonic Robo Blast 2's model packs use them:
//! surfaces of triangles with texture coordinates, and a set of frames,
//! each a pose of every vertex (animation is going from one to the next).
//! SRB2 names the frames after its sprites: `SPR2_WALK+i00`,
//! `SPR2_WALK+i01`… (`+i`: blend from one to the next), `SPR2_STND00`,
//! `SUPERWALK+i00` and so on; [`Md3::animations`] groups them.
//!
//! Only reads what it is given: every count and offset is checked.

use macroquad::prelude::{vec2, vec3, Vec2, Vec3};
use std::collections::BTreeMap;

/// A model: its frames' names and bounds, and its surfaces.
#[derive(Clone, Debug, Default)]
pub struct Md3 {
    pub frames: Vec<Frame>,
    pub surfaces: Vec<Surface>,
}

#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub name: String,
    pub min: Vec3,
    pub max: Vec3,
}

/// Triangles with one texture; each frame a position and normal for each
/// vertex (in the model's own axes: x forward, y left, z up).
#[derive(Clone, Debug, Default)]
pub struct Surface {
    /// Its texture's file name, as the model gives it.
    pub shader: String,
    pub tris: Vec<[u32; 3]>,
    pub uv: Vec<Vec2>,
    pub frames: Vec<Vec<(Vec3, Vec3)>>,
}

/// Frames that play as one animation, and whether they blend into each other.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anim {
    pub frames: Vec<usize>,
    pub blend: bool,
}

struct Reader<'a> {
    d: &'a [u8],
}

impl Reader<'_> {
    fn bytes(&self, at: usize, n: usize) -> Result<&[u8], String> {
        self.d.get(at..at.checked_add(n).ok_or("bad offset")?).ok_or_else(|| format!("cut short at {at}"))
    }

    fn i32(&self, at: usize) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.bytes(at, 4)?.try_into().map_err(|_| "bad int")?))
    }

    fn f32(&self, at: usize) -> Result<f32, String> {
        Ok(f32::from_le_bytes(self.bytes(at, 4)?.try_into().map_err(|_| "bad float")?))
    }

    fn i16(&self, at: usize) -> Result<i16, String> {
        Ok(i16::from_le_bytes(self.bytes(at, 2)?.try_into().map_err(|_| "bad short")?))
    }

    fn name(&self, at: usize, n: usize) -> Result<String, String> {
        let b = self.bytes(at, n)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(n);
        Ok(String::from_utf8_lossy(&b[..end]).into_owned())
    }

    fn vec3(&self, at: usize) -> Result<Vec3, String> {
        Ok(vec3(self.f32(at)?, self.f32(at + 4)?, self.f32(at + 8)?))
    }

    /// A count that has to be sensible.
    fn count(&self, at: usize, most: i32) -> Result<usize, String> {
        let n = self.i32(at)?;
        if !(0..=most).contains(&n) {
            return Err(format!("count {n} out of range"));
        }
        Ok(n as usize)
    }

    fn offset(&self, at: usize, base: usize) -> Result<usize, String> {
        let o = self.i32(at)?;
        if o < 0 {
            return Err("negative offset".into());
        }
        base.checked_add(o as usize).ok_or_else(|| "offset overflow".into())
    }
}

/// A normal packed as latitude and longitude in a byte each.
fn normal(packed: i16) -> Vec3 {
    let p = packed as u16;
    let lat = ((p >> 8) & 255) as f32 * std::f32::consts::TAU / 255.0;
    let lng = (p & 255) as f32 * std::f32::consts::TAU / 255.0;
    vec3(lat.cos() * lng.sin(), lat.sin() * lng.sin(), lng.cos())
}

pub fn parse(data: &[u8]) -> Result<Md3, String> {
    let r = Reader { d: data };
    if r.bytes(0, 4)? != b"IDP3" {
        return Err("not an MD3 model".into());
    }
    if r.i32(4)? != 15 {
        return Err("unknown MD3 version".into());
    }
    let n_frames = r.count(76, 1024)?;
    let n_surfaces = r.count(84, 64)?;
    let ofs_frames = r.offset(92, 0)?;
    let ofs_surfaces = r.offset(100, 0)?;
    let mut md3 = Md3::default();
    for i in 0..n_frames {
        let at = ofs_frames + i * 56;
        md3.frames.push(Frame { min: r.vec3(at)?, max: r.vec3(at + 12)?, name: r.name(at + 40, 16)? });
    }
    let mut at = ofs_surfaces;
    for _ in 0..n_surfaces {
        if r.bytes(at, 4)? != b"IDP3" {
            return Err("bad surface".into());
        }
        let frames = r.count(at + 72, 1024)?;
        let n_shaders = r.count(at + 76, 256)?;
        let n_verts = r.count(at + 80, 65_535)?;
        let n_tris = r.count(at + 84, 262_144)?;
        let ofs_tris = r.offset(at + 88, at)?;
        let ofs_shaders = r.offset(at + 92, at)?;
        let ofs_st = r.offset(at + 96, at)?;
        let ofs_xyz = r.offset(at + 100, at)?;
        let ofs_end = r.offset(at + 104, at)?;
        if frames != n_frames {
            return Err("a surface's frames do not match the model's".into());
        }
        let shader = if n_shaders > 0 { r.name(ofs_shaders, 64)? } else { String::new() };
        let mut tris = Vec::with_capacity(n_tris);
        for t in 0..n_tris {
            let a = ofs_tris + t * 12;
            let tri = [r.i32(a)?, r.i32(a + 4)?, r.i32(a + 8)?];
            if tri.iter().any(|&k| k < 0 || k as usize >= n_verts) {
                return Err("a triangle points past the vertices".into());
            }
            tris.push([tri[0] as u32, tri[1] as u32, tri[2] as u32]);
        }
        let mut uv = Vec::with_capacity(n_verts);
        for v in 0..n_verts {
            uv.push(vec2(r.f32(ofs_st + v * 8)?, r.f32(ofs_st + v * 8 + 4)?));
        }
        let mut poses = Vec::with_capacity(frames);
        for f in 0..frames {
            let mut pose = Vec::with_capacity(n_verts);
            for v in 0..n_verts {
                let a = ofs_xyz + (f * n_verts + v) * 8;
                let p = vec3(r.i16(a)? as f32, r.i16(a + 2)? as f32, r.i16(a + 4)? as f32) / 64.0;
                pose.push((p, normal(r.i16(a + 6)?)));
            }
            poses.push(pose);
        }
        md3.surfaces.push(Surface { shader, tris, uv, frames: poses });
        if ofs_end <= at {
            return Err("bad surface length".into());
        }
        at = ofs_end;
    }
    Ok(md3)
}

/// An animation's name and whether its frames blend, from a frame's name:
/// `SPR2_WALK+i03` is frame 3 of `WALK`, blended; `SUPERRUN_01` of `SUPERRUN_`.
pub fn anim_of(frame: &str) -> (String, bool) {
    // SRB2's own names: a sprite name of four characters (`TAL6` has a
    // digit in it), "SUPER" before it for the super form.
    let sprite = |rest: &str, before: &str| {
        let name: String = rest.chars().take(4).collect();
        let after = &rest[name.len()..];
        let blend = after.starts_with("+i");
        (format!("{before}{name}"), blend)
    };
    if let Some(rest) = frame.strip_prefix("SPR2_") {
        if rest.len() > 4 {
            return sprite(rest, "");
        }
    }
    if let Some(rest) = frame.strip_prefix("SUPER") {
        if rest.len() > 4 && rest.is_char_boundary(4) {
            return sprite(rest, "SUPER");
        }
    }
    let name = frame.trim_end_matches(|c: char| c.is_ascii_digit());
    match name.strip_suffix("+i") {
        Some(n) => (n.to_string(), true),
        None => (name.to_string(), false),
    }
}

impl Md3 {
    /// The frames grouped into animations by name, in order.
    pub fn animations(&self) -> BTreeMap<String, Anim> {
        let mut out: BTreeMap<String, Anim> = BTreeMap::new();
        for (i, f) in self.frames.iter().enumerate() {
            let (name, blend) = anim_of(&f.name);
            let a = out.entry(name).or_insert(Anim { frames: vec![], blend });
            a.frames.push(i);
            a.blend |= blend;
        }
        out
    }

    /// How tall it stands in its first frame (model units).
    pub fn height(&self) -> f32 {
        self.frames.first().map_or(0.0, |f| f.max.z - f.min.z.min(0.0))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A small MD3 made by hand: one surface (a triangle), `frames` frames
    /// each lifting it by 1 unit.
    pub fn sample(frame_names: &[&str]) -> Vec<u8> {
        let n_frames = frame_names.len();
        let mut d = vec![];
        let i32s = |d: &mut Vec<u8>, v: i32| d.extend_from_slice(&v.to_le_bytes());
        let name = |d: &mut Vec<u8>, s: &str, n: usize| {
            let mut b = s.as_bytes().to_vec();
            b.resize(n, 0);
            d.extend_from_slice(&b);
        };
        d.extend_from_slice(b"IDP3");
        i32s(&mut d, 15);
        name(&mut d, "test", 64);
        i32s(&mut d, 0); // flags
        i32s(&mut d, n_frames as i32);
        i32s(&mut d, 0); // tags
        i32s(&mut d, 1); // surfaces
        i32s(&mut d, 0); // skins
        let header = 108;
        let ofs_frames = header;
        let ofs_surf = ofs_frames + n_frames * 56;
        i32s(&mut d, ofs_frames as i32);
        i32s(&mut d, ofs_surf as i32); // tags (none)
        i32s(&mut d, ofs_surf as i32);
        let surf_len = 108 + 68 + 12 + 3 * 8 + n_frames * 3 * 8;
        i32s(&mut d, (ofs_surf + surf_len) as i32);
        for (k, f) in frame_names.iter().enumerate() {
            for v in [0.0f32, 0.0, k as f32, 10.0, 10.0, 10.0 + k as f32, 0.0, 0.0, 0.0, 5.0] {
                d.extend_from_slice(&v.to_le_bytes());
            }
            name(&mut d, f, 16);
        }
        // The surface.
        d.extend_from_slice(b"IDP3");
        name(&mut d, "body", 64);
        i32s(&mut d, 0);
        i32s(&mut d, n_frames as i32);
        i32s(&mut d, 1); // shaders
        i32s(&mut d, 3); // verts
        i32s(&mut d, 1); // tris
        let (o_sh, o_tri) = (108usize, 108usize + 68);
        let o_st = o_tri + 12;
        let o_xyz = o_st + 24;
        i32s(&mut d, o_tri as i32);
        i32s(&mut d, o_sh as i32);
        i32s(&mut d, o_st as i32);
        i32s(&mut d, o_xyz as i32);
        i32s(&mut d, surf_len as i32);
        name(&mut d, "BODY.png", 64);
        i32s(&mut d, 0);
        for k in [0, 1, 2] {
            i32s(&mut d, k);
        }
        for v in [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0] {
            d.extend_from_slice(&v.to_le_bytes());
        }
        for f in 0..n_frames {
            for (x, y) in [(0i16, 0i16), (10, 0), (0, 10)] {
                for c in [x * 64, y * 64, f as i16 * 64, 0] {
                    d.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
        d
    }

    #[test]
    fn a_model_reads_back_with_its_frames_and_poses() {
        let data = sample(&["SPR2_STND00", "SPR2_WALK+i00", "SPR2_WALK+i01", "SUPERRUN_00"]);
        let m = parse(&data).unwrap();
        assert_eq!(m.frames.len(), 4);
        assert_eq!(m.frames[1].name, "SPR2_WALK+i00");
        let s = &m.surfaces[0];
        assert_eq!(s.shader.as_str(), "BODY.png");
        assert_eq!(s.tris, vec![[0, 1, 2]]);
        assert_eq!(s.uv[2], vec2(0.0, 1.0));
        assert_eq!(s.frames[2][1].0, vec3(10.0, 0.0, 2.0), "frame 2 is lifted by 2");
        assert!((s.frames[0][0].1.length() - 1.0).abs() < 1e-3, "normals are unit");
        let anims = m.animations();
        assert_eq!(anims["WALK"], Anim { frames: vec![1, 2], blend: true });
        assert_eq!(anims["STND"], Anim { frames: vec![0], blend: false });
        assert_eq!(anims["SUPERRUN_"].frames, vec![3]);
        assert_eq!(m.height(), 10.0);
    }

    #[test]
    fn broken_files_are_refused_not_trusted() {
        let data = sample(&["A00", "A01"]);
        assert!(parse(b"nope").is_err());
        assert!(parse(&data[..data.len() - 10]).is_err(), "cut short");
        let mut bad = data.clone();
        bad[76..80].copy_from_slice(&100_000i32.to_le_bytes());
        assert!(parse(&bad).is_err(), "absurd count");
        let mut bad = data.clone();
        // The triangle's last corner pointing past the vertices.
        let tri = 108 + 2 * 56 + 108 + 68 + 8;
        bad[tri..tri + 4].copy_from_slice(&7i32.to_le_bytes());
        assert!(parse(&bad).is_err());
    }

    #[test]
    fn frame_names_make_animations() {
        assert_eq!(anim_of("SPR2_RUN_+i02"), ("RUN_".into(), true));
        assert_eq!(anim_of("SPR2_ROLL04"), ("ROLL".into(), false));
        assert_eq!(anim_of("SUPERSTND+i01"), ("SUPERSTND".into(), true));
        assert_eq!(anim_of("POSE00"), ("POSE".into(), false));
        assert_eq!(anim_of("FBOMB01"), ("FBOMB".into(), false));
        assert_eq!(anim_of("SPR2_TAL600"), ("TAL6".into(), false));
        assert_eq!(anim_of("SPR2_TAL0+i03"), ("TAL0".into(), true));
        assert_eq!(anim_of("SUPERRUN_01"), ("SUPERRUN_".into(), false));
    }
}
