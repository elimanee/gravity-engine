//! Drawing in 3D: a lit material (the sun, the sky's light from above and
//! fog in the distance), meshes cut to fit macroquad's draw calls, and
//! simple shapes to build things from.

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Comparison, CullFace, Equation, PipelineParams};
use macroquad::models::Vertex;
use macroquad::prelude::*;

/// Most vertices and indices in a draw call (macroquad's default capacity,
/// with room to spare).
const MOST_VERTS: usize = 9000;
const MOST_INDICES: usize = 4800;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;

varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump vec3 n;
varying mediump vec3 at;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    vec4 w = Model * vec4(position, 1.0);
    at = w.xyz;
    n = (Model * vec4(normal.xyz, 0.0)).xyz;
    gl_Position = Projection * w;
    color = color0 / 255.0;
    uv = texcoord;
}"#;

const FRAGMENT: &str = r#"#version 100
precision mediump float;

varying lowp vec2 uv;
varying lowp vec4 color;
varying mediump vec3 n;
varying mediump vec3 at;

uniform sampler2D Texture;
uniform vec3 Sun;
uniform vec3 Eye;
uniform vec4 Fog;
uniform vec4 Light;

void main() {
    vec4 base = color * texture2D(Texture, uv);
    if (base.a < 0.4) {
        discard;
    }
    vec3 nn = normalize(n);
    // Lit on both sides (thin things are seen from behind).
    if (dot(nn, Eye - at) < 0.0) {
        nn = -nn;
    }
    float sun = max(dot(nn, Sun), 0.0);
    float sky = 0.75 + 0.25 * nn.y;
    // Light.rgb: the ambient colour; Light.a: how much the sun gives.
    vec3 lit = base.rgb * (Light.rgb * sky + Light.a * sun);
    float d = length(at - Eye);
    float f = clamp(1.0 - exp(-d * Fog.a), 0.0, 1.0);
    gl_FragColor = vec4(mix(lit, Fog.rgb, f), base.a);
}"#;

/// The lighting of a world.
#[derive(Clone, Copy, Debug)]
pub struct Sky {
    /// Towards the sun (normalised).
    pub sun: Vec3,
    pub ambient: Color,
    pub sunlight: f32,
    pub fog: Color,
    /// Fog density (per metre).
    pub haze: f32,
    /// The sky's colour overhead and at the horizon.
    pub top: Color,
    pub horizon: Color,
}

impl Sky {
    pub const DAY: Sky = Sky {
        sun: Vec3::new(0.38, 0.8, 0.46),
        ambient: Color::new(0.46, 0.5, 0.6, 1.0),
        sunlight: 0.7,
        fog: Color::new(0.72, 0.84, 0.97, 1.0),
        haze: 0.0042,
        top: Color::new(0.2, 0.46, 0.92, 1.0),
        horizon: Color::new(0.74, 0.86, 0.98, 1.0),
    };
}

pub struct Gfx {
    lit: Material,
    pub white: Texture2D,
}

impl Gfx {
    pub fn new() -> Option<Gfx> {
        let params = PipelineParams {
            depth_test: Comparison::LessOrEqual,
            depth_write: true,
            cull_face: CullFace::Nothing,
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            ..Default::default()
        };
        let lit = load_material(
            ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
            MaterialParams {
                pipeline_params: params,
                uniforms: vec![
                    UniformDesc::new("Sun", UniformType::Float3),
                    UniformDesc::new("Eye", UniformType::Float3),
                    UniformDesc::new("Fog", UniformType::Float4),
                    UniformDesc::new("Light", UniformType::Float4),
                ],
                ..Default::default()
            },
        )
        .ok()?;
        let white = Texture2D::from_rgba8(1, 1, &[255, 255, 255, 255]);
        Some(Gfx { lit, white })
    }

    /// Draw lit from here on, seen from `eye`.
    pub fn begin(&self, sky: &Sky, eye: Vec3) {
        self.lit.set_uniform("Sun", sky.sun.normalize());
        self.lit.set_uniform("Eye", eye);
        self.lit.set_uniform("Fog", vec4(sky.fog.r, sky.fog.g, sky.fog.b, sky.haze));
        self.lit.set_uniform("Light", vec4(sky.ambient.r, sky.ambient.g, sky.ambient.b, sky.sunlight));
        gl_use_material(&self.lit);
    }

    pub fn end(&self) {
        gl_use_default_material();
    }
}

/// Draw `meshes` placed by `model`.
pub fn draw_at(meshes: &[Mesh], model: Mat4) {
    let gl = unsafe { get_internal_gl() }.quad_gl;
    gl.push_model_matrix(model);
    for m in meshes {
        draw_mesh(m);
    }
    let gl = unsafe { get_internal_gl() }.quad_gl;
    gl.pop_model_matrix();
}

/// The sky behind everything: a gradient from the horizon up (screen
/// pass, before the world).
pub fn draw_sky(sky: &Sky, horizon_y: f32) {
    let (w, h) = (screen_width(), screen_height());
    clear_background(sky.horizon);
    let y = horizon_y.clamp(-h, 2.0 * h);
    let top = sky.top;
    let hor = sky.horizon;
    let v = |x: f32, y: f32, c: Color| Vertex::new(x, y, 0.0, 0.0, 0.0, c);
    let vertices = vec![v(0.0, 0.0, top), v(w, 0.0, top), v(w, y, hor), v(0.0, y, hor)];
    draw_mesh(&Mesh { vertices, indices: vec![0, 1, 2, 0, 2, 3], texture: None });
}

/// Triangles being built: positions, normals, texture coordinates and
/// colours; made into meshes small enough for a draw call each.
#[derive(Clone, Default, Debug)]
pub struct Shape {
    pub pos: Vec<Vec3>,
    pub normal: Vec<Vec3>,
    pub uv: Vec<Vec2>,
    pub colour: Vec<[u8; 4]>,
    pub tris: Vec<[u32; 3]>,
}

pub fn rgba(c: Color) -> [u8; 4] {
    [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8, (c.a * 255.0) as u8]
}

impl Shape {
    pub fn vertex(&mut self, p: Vec3, n: Vec3, uv: Vec2, c: Color) -> u32 {
        self.pos.push(p);
        self.normal.push(n);
        self.uv.push(uv);
        self.colour.push(rgba(c));
        (self.pos.len() - 1) as u32
    }

    pub fn tri(&mut self, a: u32, b: u32, c: u32) {
        self.tris.push([a, b, c]);
    }

    /// A flat quad a, b, c, d (in order round it), its normal worked out.
    pub fn quad(&mut self, p: [Vec3; 4], uv: [Vec2; 4], c: Color) {
        let n = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
        let i: Vec<u32> = (0..4).map(|k| self.vertex(p[k], n, uv[k], c)).collect();
        self.tri(i[0], i[1], i[2]);
        self.tri(i[0], i[2], i[3]);
    }

    /// Another shape added, moved by `m` (its normals turned with it).
    pub fn append(&mut self, other: &Shape, m: Mat4) {
        let base = self.pos.len() as u32;
        for k in 0..other.pos.len() {
            self.pos.push(m.transform_point3(other.pos[k]));
            self.normal.push(m.transform_vector3(other.normal[k]).normalize_or_zero());
            self.uv.push(other.uv[k]);
            self.colour.push(other.colour[k]);
        }
        self.tris.extend(other.tris.iter().map(|t| [t[0] + base, t[1] + base, t[2] + base]));
    }

    /// Meshes of it, each small enough for one draw call.
    pub fn meshes(&self, texture: Option<&Texture2D>) -> Vec<Mesh> {
        let mut out = vec![];
        let mut map = vec![u32::MAX; self.pos.len()];
        let mut verts: Vec<Vertex> = vec![];
        let mut idx: Vec<u16> = vec![];
        let mut used: Vec<usize> = vec![];
        let mut flush = |verts: &mut Vec<Vertex>, idx: &mut Vec<u16>, used: &mut Vec<usize>, map: &mut Vec<u32>| {
            if !idx.is_empty() {
                out.push(Mesh {
                    vertices: std::mem::take(verts),
                    indices: std::mem::take(idx),
                    texture: texture.cloned(),
                });
            }
            for k in used.drain(..) {
                map[k] = u32::MAX;
            }
        };
        for t in &self.tris {
            if verts.len() + 3 > MOST_VERTS || idx.len() + 3 > MOST_INDICES {
                flush(&mut verts, &mut idx, &mut used, &mut map);
            }
            for &k in t {
                let k = k as usize;
                if map[k] == u32::MAX {
                    map[k] = verts.len() as u32;
                    used.push(k);
                    let n = self.normal[k];
                    verts.push(Vertex {
                        position: self.pos[k],
                        uv: self.uv[k],
                        color: self.colour[k],
                        normal: vec4(n.x, n.y, n.z, 0.0),
                    });
                }
                idx.push(map[k] as u16);
            }
        }
        flush(&mut verts, &mut idx, &mut used, &mut map);
        out
    }
}

/// A box from `min` to `max`, one colour.
pub fn cuboid(min: Vec3, max: Vec3, c: Color) -> Shape {
    let mut s = Shape::default();
    let (a, b) = (min, max);
    let p = |x: f32, y: f32, z: f32| vec3(x, y, z);
    let uv = [vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0)];
    // Each face counter-clockwise seen from outside.
    s.quad([p(a.x, a.y, b.z), p(b.x, a.y, b.z), p(b.x, b.y, b.z), p(a.x, b.y, b.z)], uv, c);
    s.quad([p(b.x, a.y, a.z), p(a.x, a.y, a.z), p(a.x, b.y, a.z), p(b.x, b.y, a.z)], uv, c);
    s.quad([p(a.x, a.y, a.z), p(a.x, a.y, b.z), p(a.x, b.y, b.z), p(a.x, b.y, a.z)], uv, c);
    s.quad([p(b.x, a.y, b.z), p(b.x, a.y, a.z), p(b.x, b.y, a.z), p(b.x, b.y, b.z)], uv, c);
    s.quad([p(a.x, b.y, b.z), p(b.x, b.y, b.z), p(b.x, b.y, a.z), p(a.x, b.y, a.z)], uv, c);
    s.quad([p(a.x, a.y, a.z), p(b.x, a.y, a.z), p(b.x, a.y, b.z), p(a.x, a.y, b.z)], uv, c);
    s
}

/// A sphere (ellipsoid with `scale`) of `rings` × `segs` facets.
pub fn sphere(centre: Vec3, scale: Vec3, rings: u32, segs: u32, c: Color) -> Shape {
    let mut s = Shape::default();
    for i in 0..=rings {
        let v = i as f32 / rings as f32;
        let th = v * std::f32::consts::PI;
        for j in 0..=segs {
            let u = j as f32 / segs as f32;
            let ph = u * std::f32::consts::TAU;
            let n = vec3(th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin());
            s.vertex(centre + n * scale, (n / scale).normalize_or_zero(), vec2(u, v), c);
        }
    }
    let row = segs + 1;
    for i in 0..rings {
        for j in 0..segs {
            let (a, b) = (i * row + j, (i + 1) * row + j);
            s.tri(a, b, b + 1);
            s.tri(a, b + 1, a + 1);
        }
    }
    s
}

/// A cylinder or cone along y from `y0` (radius `r0`) to `y1` (`r1`), capped.
pub fn cylinder(centre: Vec3, y0: f32, y1: f32, r0: f32, r1: f32, segs: u32, c: Color) -> Shape {
    let mut s = Shape::default();
    let slope = (r0 - r1) / (y1 - y0).abs().max(1e-3);
    for j in 0..=segs {
        let a = j as f32 / segs as f32 * std::f32::consts::TAU;
        let (x, z) = (a.cos(), a.sin());
        let n = vec3(x, slope, z).normalize();
        s.vertex(centre + vec3(x * r0, y0, z * r0), n, vec2(j as f32 / segs as f32, 0.0), c);
        s.vertex(centre + vec3(x * r1, y1, z * r1), n, vec2(j as f32 / segs as f32, 1.0), c);
    }
    for j in 0..segs {
        let (a, b) = (j * 2, j * 2 + 2);
        s.tri(a, a + 1, b + 1);
        s.tri(a, b + 1, b);
    }
    for (y, r, up) in [(y0, r0, -1.0f32), (y1, r1, 1.0)] {
        if r <= 0.0 {
            continue;
        }
        let mid = s.vertex(centre + vec3(0.0, y, 0.0), vec3(0.0, up, 0.0), vec2(0.5, 0.5), c);
        let first = s.pos.len() as u32;
        for j in 0..=segs {
            let a = j as f32 / segs as f32 * std::f32::consts::TAU;
            s.vertex(centre + vec3(a.cos() * r, y, a.sin() * r), vec3(0.0, up, 0.0), vec2(0.5, 0.5), c);
        }
        for j in 0..segs {
            s.tri(mid, first + j, first + j + 1);
        }
    }
    s
}

/// A ring (torus) round the y axis: radius `r`, tube `t`.
pub fn torus(r: f32, t: f32, segs: u32, sides: u32, c: Color) -> Shape {
    let mut s = Shape::default();
    for i in 0..=segs {
        let a = i as f32 / segs as f32 * std::f32::consts::TAU;
        let centre = vec3(a.cos() * r, 0.0, a.sin() * r);
        let out = vec3(a.cos(), 0.0, a.sin());
        for j in 0..=sides {
            let b = j as f32 / sides as f32 * std::f32::consts::TAU;
            let n = out * b.cos() + Vec3::Y * b.sin();
            s.vertex(centre + n * t, n, vec2(i as f32 / segs as f32, j as f32 / sides as f32), c);
        }
    }
    let row = sides + 1;
    for i in 0..segs {
        for j in 0..sides {
            let (a, b) = (i * row + j, (i + 1) * row + j);
            s.tri(a, b, b + 1);
            s.tri(a, b + 1, a + 1);
        }
    }
    s
}

/// A procedural texture: a checker of two colours (n × n squares) with a
/// darker line round each square, `px` pixels a side.
pub fn checker(a: Color, b: Color, n: u32, px: u32) -> Texture2D {
    let mut data = Vec::with_capacity((px * px * 4) as usize);
    let cell = px / n;
    for y in 0..px {
        for x in 0..px {
            let (cx, cy) = (x / cell, y / cell);
            let edge = x % cell == 0 || y % cell == 0;
            let c = if (cx + cy) % 2 == 0 { a } else { b };
            let k = if edge { 0.85 } else { 1.0 };
            data.extend_from_slice(&[(c.r * k * 255.0) as u8, (c.g * k * 255.0) as u8, (c.b * k * 255.0) as u8, 255]);
        }
    }
    let t = Texture2D::from_rgba8(px as u16, px as u16, &data);
    repeat(&t);
    t
}

/// Tiled over the ground (and smoothed into the distance).
pub fn repeat(t: &Texture2D) {
    use macroquad::miniquad::{FilterMode as F, MipmapFilterMode, TextureWrap};
    let id = t.raw_miniquad_id();
    let ctx = unsafe { get_internal_gl() }.quad_context;
    ctx.texture_set_wrap(id, TextureWrap::Repeat, TextureWrap::Repeat);
    ctx.texture_generate_mipmaps(id);
    ctx.texture_set_min_filter(id, F::Linear, MipmapFilterMode::Linear);
    ctx.texture_set_mag_filter(id, F::Nearest);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_shapes_are_cut_into_meshes_that_fit_a_draw_call() {
        // A grid of 120 × 120 quads: 28 800 triangles.
        let mut s = Shape::default();
        let n = 120;
        for i in 0..=n {
            for j in 0..=n {
                s.vertex(vec3(i as f32, 0.0, j as f32), Vec3::Y, Vec2::ZERO, WHITE);
            }
        }
        for i in 0..n {
            for j in 0..n {
                let a = i * (n + 1) + j;
                s.tri(a, a + n + 1, a + n + 2);
                s.tri(a, a + n + 2, a + 1);
            }
        }
        let meshes = s.meshes(None);
        assert!(meshes.len() > 1);
        let tris: usize = meshes.iter().map(|m| m.indices.len() / 3).sum();
        assert_eq!(tris, s.tris.len(), "every triangle drawn once");
        for m in &meshes {
            assert!(m.vertices.len() <= MOST_VERTS && m.indices.len() <= MOST_INDICES);
            assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
        }
        // Positions survive the cut.
        let m = &meshes[0];
        let t = &s.tris[0];
        assert_eq!(m.vertices[m.indices[1] as usize].position, s.pos[t[1] as usize]);
    }

    #[test]
    fn shapes_face_outwards() {
        let b = cuboid(Vec3::splat(-1.0), Vec3::splat(1.0), WHITE);
        for t in &b.tris {
            let (p, n) = (b.pos[t[0] as usize], b.normal[t[0] as usize]);
            let face = (b.pos[t[1] as usize] - p).cross(b.pos[t[2] as usize] - p);
            assert!(face.dot(n) > 0.0, "winding agrees with the normal");
            assert!(n.dot(p) > 0.0, "the normal points out");
        }
        let s = sphere(Vec3::ZERO, Vec3::ONE, 6, 8, WHITE);
        assert!(s.pos.iter().zip(&s.normal).all(|(p, n)| (p.normalize() - *n).length() < 1e-3));
        let r = torus(2.0, 0.5, 12, 6, WHITE);
        assert!(r.pos.iter().all(|p| ((p.x * p.x + p.z * p.z).sqrt() - 2.0).abs() <= 0.5 + 1e-3));
    }
}
