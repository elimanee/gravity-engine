//! Procedural shapes for the object spawner: anti-aliased shaded sprites plus
//! exact collider geometry.

use crate::util::point_in_polygon;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::f32::consts::{FRAC_PI_2, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shape {
    Circle,
    Box,
    Triangle,
    Pentagon,
    Hexagon,
    Star,
    Capsule,
}

impl Shape {
    pub const ALL: &'static [Shape] =
        &[Shape::Circle, Shape::Box, Shape::Triangle, Shape::Pentagon, Shape::Hexagon, Shape::Star, Shape::Capsule];

    pub fn label(self) -> &'static str {
        match self {
            Shape::Circle => "Circle",
            Shape::Box => "Box",
            Shape::Triangle => "Triangle",
            Shape::Pentagon => "Pentagon",
            Shape::Hexagon => "Hexagon",
            Shape::Star => "Star",
            Shape::Capsule => "Capsule",
        }
    }

    /// Sprite aspect ratio (height / width).
    pub fn aspect(self) -> f32 {
        match self {
            Shape::Capsule => 0.5,
            _ => 1.0,
        }
    }

    /// Normalised outline (u, v) ∈ [-0.5, 0.5]², v pointing *up*, counter-clockwise.
    /// `None` for shapes described analytically (circle, box, capsule).
    pub fn polygon(self) -> Option<Vec<[f32; 2]>> {
        let regular = |n: usize, r: f32| -> Vec<[f32; 2]> {
            (0..n)
                .map(|i| {
                    let a = FRAC_PI_2 + i as f32 * TAU / n as f32;
                    [a.cos() * r, a.sin() * r]
                })
                .collect()
        };
        match self {
            Shape::Triangle => Some(vec![[0.0, 0.46], [-0.48, -0.42], [0.48, -0.42]]),
            Shape::Pentagon => Some(regular(5, 0.48)),
            Shape::Hexagon => Some(regular(6, 0.48)),
            Shape::Star => Some(
                (0..10)
                    .map(|i| {
                        let a = FRAC_PI_2 + i as f32 * TAU / 10.0;
                        let r = if i % 2 == 0 { 0.49 } else { 0.21 };
                        [a.cos() * r, a.sin() * r]
                    })
                    .collect(),
            ),
            Shape::Circle | Shape::Box | Shape::Capsule => None,
        }
    }

    /// Whether the normalised point lies inside the shape, for a sprite of
    /// `w × h` px. `inset` shrinks the shape by that many pixels.
    fn contains(self, u: f32, v: f32, w: f32, h: f32, inset: f32, poly: &[(f32, f32)]) -> bool {
        let (x, y) = (u * w, v * h); // pixel units around the centre
        match self {
            Shape::Circle => x * x + y * y <= (w * 0.5 - inset).powi(2),
            Shape::Box => {
                let r = w * BOX_ROUNDING;
                let (hx, hy) = (w * 0.5 - inset - r, h * 0.5 - inset - r);
                let dx = (x.abs() - hx).max(0.0);
                let dy = (y.abs() - hy).max(0.0);
                dx * dx + dy * dy <= r * r
            }
            Shape::Capsule => {
                let r = h * 0.5 - inset;
                let hx = w * 0.5 - h * 0.5;
                let dx = (x.abs() - hx).max(0.0);
                dx * dx + y * y <= r * r
            }
            _ => {
                let s = 1.0 - 2.0 * inset / w;
                point_in_polygon(u / s, v / s, poly)
            }
        }
    }
}

/// Corner radius of `Shape::Box` relative to its width.
pub const BOX_ROUNDING: f32 = 0.12;

/// A pleasant, well-separated palette for spawned objects.
pub const PALETTE: &[(u8, u8, u8)] = &[
    (239, 86, 102),  // coral
    (250, 160, 70),  // tangerine
    (247, 208, 80),  // sunflower
    (98, 204, 120),  // mint
    (64, 196, 214),  // teal
    (84, 140, 245),  // azure
    (150, 110, 250), // violet
    (232, 110, 200), // orchid
    (228, 228, 236), // snow
];

/// Rasterise `shape` at `width` px with 4×4 super-sampling, a darker rim and a
/// soft vertical sheen.
pub fn rasterize(shape: Shape, width: u32, rgb: (u8, u8, u8)) -> RgbaImage {
    let w = width.max(4);
    let h = ((w as f32 * shape.aspect()).round() as u32).max(4);
    let (wf, hf) = (w as f32, h as f32);
    let poly: Vec<(f32, f32)> = shape.polygon().unwrap_or_default().into_iter().map(|[u, v]| (u, v)).collect();
    let rim = (wf * 0.045).clamp(1.5, 5.0);
    const SS: usize = 4;

    let base = [rgb.0 as f32, rgb.1 as f32, rgb.2 as f32];
    let mut img = RgbaImage::new(w, h);
    for py in 0..h {
        for px in 0..w {
            let (mut cov, mut inner) = (0usize, 0usize);
            for sy in 0..SS {
                for sx in 0..SS {
                    let fx = px as f32 + (sx as f32 + 0.5) / SS as f32;
                    let fy = py as f32 + (sy as f32 + 0.5) / SS as f32;
                    let u = fx / wf - 0.5;
                    let v = 0.5 - fy / hf;
                    if shape.contains(u, v, wf, hf, 0.0, &poly) {
                        cov += 1;
                        if shape.contains(u, v, wf, hf, rim, &poly) {
                            inner += 1;
                        }
                    }
                }
            }
            if cov == 0 {
                continue;
            }
            let alpha = cov as f32 / (SS * SS) as f32;
            let inner_frac = inner as f32 / cov as f32;
            let t = py as f32 / hf; // 0 top → 1 bottom
            let sheen = 1.14 - 0.30 * t;
            let mut c = [0u8; 4];
            for i in 0..3 {
                let body = (base[i] * sheen).min(255.0);
                let edge = base[i] * 0.62;
                c[i] = (edge + (body - edge) * inner_frac) as u8;
            }
            c[3] = (alpha * 255.0) as u8;
            img.put_pixel(px, py, Rgba(c));
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_rasterises_with_coverage() {
        for &s in Shape::ALL {
            let img = rasterize(s, 64, (200, 100, 50));
            let opaque = img.pixels().filter(|p| p[3] == 255).count();
            assert!(opaque > 300, "{s:?} has too little coverage");
            assert_eq!(img.get_pixel(0, 0)[3], 0, "{s:?} corner should be clear");
        }
    }

    #[test]
    fn polygons_are_normalised() {
        for &s in Shape::ALL {
            if let Some(p) = s.polygon() {
                assert!(p.iter().all(|q| q[0].abs() <= 0.5 && q[1].abs() <= 0.5));
            }
        }
    }
}
