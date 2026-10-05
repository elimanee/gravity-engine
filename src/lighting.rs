//! Night: the scene is drawn as usual, then multiplied by a *light map*, a
//! texture cleared to a dim ambient colour into which every light adds a
//! soft glow. Lamps light a visibility polygon (rays cast from the lamp
//! stop at objects), so objects cast shadows.

use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
use macroquad::prelude::*;

/// The light map is drawn at this fraction of the window's resolution
/// (softer shadow edges, less work).
const SCALE: f32 = 0.5;
/// Share of the light map added on top as haze.
const HAZE: f32 = 0.22;

const VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

const FRAGMENT: &str = r#"#version 100
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
void main() {
    gl_FragColor = color * texture2D(Texture, uv);
}
"#;

struct Gpu {
    /// Lights add up: rgb × alpha.
    add: Material,
    /// The light map multiplies what is under it.
    multiply: Material,
    /// White in the middle fading to black at the edge.
    glow: Texture2D,
    target: RenderTarget,
}

#[derive(Default)]
pub struct Lights {
    gpu: Option<Gpu>,
}

/// Dim bluish moonlight for a night of `darkness` (0.5‥0.95), brightened
/// by a lightning `flash` (0‥1).
pub fn ambient(darkness: f32, flash: f32) -> Color {
    let k = (1.0 - darkness).clamp(0.0, 1.0);
    let f = flash * 0.8;
    Color::new((k * 0.7 + f).min(1.0), (k * 0.82 + f).min(1.0), (k * 1.2 + f).min(1.0), 1.0)
}

fn material(blend: BlendState) -> Option<Material> {
    load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
        MaterialParams {
            pipeline_params: PipelineParams { color_blend: Some(blend), ..Default::default() },
            ..Default::default()
        },
    )
    .ok()
}

fn glow_texture() -> Texture2D {
    const N: u16 = 128;
    let mut px = Vec::with_capacity(N as usize * N as usize * 4);
    for y in 0..N {
        for x in 0..N {
            let d = vec2(x as f32 + 0.5, y as f32 + 0.5) / (N as f32 / 2.0) - Vec2::ONE;
            let f = (1.0 - d.length()).clamp(0.0, 1.0);
            // Smooth: bright core, long soft tail.
            let v = (f * f * (3.0 - 2.0 * f) * 255.0) as u8;
            px.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let t = Texture2D::from_rgba8(N, N, &px);
    t.set_filter(FilterMode::Linear);
    t
}

impl Lights {
    /// Start drawing lights in world pixels over the visible world rectangle
    /// `visible`. Returns false when the GPU side could not be set up.
    pub fn begin(&mut self, visible: Rect, ambient: Color) -> bool {
        let (w, h) = ((screen_width() * SCALE).max(1.0) as u32, (screen_height() * SCALE).max(1.0) as u32);
        if self.gpu.is_none() {
            let add =
                material(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One));
            let multiply = material(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::DestinationColor),
                BlendFactor::Zero,
            ));
            let (Some(add), Some(multiply)) = (add, multiply) else { return false };
            self.gpu = Some(Gpu { add, multiply, glow: glow_texture(), target: render_target(w, h) });
        }
        let Some(gpu) = &mut self.gpu else { return false };
        if (gpu.target.texture.width() as u32, gpu.target.texture.height() as u32) != (w, h) {
            gpu.target = render_target(w, h);
        }
        gpu.target.texture.set_filter(FilterMode::Linear);
        let mut cam = Camera2D::from_display_rect(visible);
        cam.render_target = Some(gpu.target.clone());
        set_camera(&cam);
        clear_background(ambient);
        gl_use_material(&gpu.add);
        true
    }

    /// A round glow of `radius` px; `color.a` is its strength.
    pub fn glow(&self, at: Vec2, radius: f32, color: Color) {
        let Some(gpu) = &self.gpu else { return };
        draw_texture_ex(
            &gpu.glow,
            at.x - radius,
            at.y - radius,
            color,
            DrawTextureParams { dest_size: Some(vec2(radius, radius) * 2.0), ..Default::default() },
        );
    }

    /// Light from `at` filling the polygon `rim` (a fan around `at`),
    /// fading out at `radius` px.
    pub fn fan(&self, at: Vec2, rim: &[Vec2], radius: f32, color: Color) {
        let Some(gpu) = &self.gpu else { return };
        if rim.len() < 2 || rim.len() > 4000 {
            return;
        }
        let uv = |p: Vec2| (p - at) / (radius * 2.0) + vec2(0.5, 0.5);
        let c: [u8; 4] = color.into();
        let mut vertices = Vec::with_capacity(rim.len() + 1);
        vertices.push(Vertex::new(at.x, at.y, 0.0, 0.5, 0.5, Color::from_rgba(c[0], c[1], c[2], c[3])));
        for &p in rim {
            let t = uv(p);
            vertices.push(Vertex::new(p.x, p.y, 0.0, t.x, t.y, Color::from_rgba(c[0], c[1], c[2], c[3])));
        }
        let mut indices = Vec::with_capacity(rim.len() * 3);
        for k in 1..rim.len() as u16 {
            indices.extend_from_slice(&[0, k, k + 1]);
        }
        draw_mesh(&Mesh { vertices, indices, texture: Some(gpu.glow.clone()) });
    }

    /// Light along a segment: glows every few pixels.
    pub fn line(&self, a: Vec2, b: Vec2, radius: f32, color: Color) {
        let n = ((a.distance(b) / (radius * 0.6)).ceil() as usize).clamp(1, 200);
        for k in 0..=n {
            self.glow(a.lerp(b, k as f32 / n as f32), radius, color);
        }
    }

    /// Stop drawing lights and darken the window with the light map, then
    /// add a little of it back as haze (light in the air shows even over a
    /// black background).
    pub fn end(&self) {
        let Some(gpu) = &self.gpu else { return };
        gl_use_default_material();
        set_default_camera();
        let params = DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            flip_y: true,
            ..Default::default()
        };
        gl_use_material(&gpu.multiply);
        draw_texture_ex(&gpu.target.texture, 0.0, 0.0, WHITE, params.clone());
        gl_use_material(&gpu.add);
        draw_texture_ex(&gpu.target.texture, 0.0, 0.0, Color::new(1.0, 1.0, 1.0, HAZE), params);
        gl_use_default_material();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn darker_nights_and_bright_flashes() {
        let dim = ambient(0.95, 0.0);
        let bright = ambient(0.5, 0.0);
        assert!(dim.r < bright.r && dim.b < bright.b);
        assert!(bright.b > bright.r, "moonlight is bluish");
        let flash = ambient(0.95, 1.0);
        assert!(flash.r > 0.7 && flash.b <= 1.0);
    }
}
