//! Freehand drawings made with the Draw tool. A stroke recorded in screen
//! pixels becomes either a filled polygon (when it ends near where it
//! started) or a plank that follows the stroke.

use crate::util::point_in_polygon;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

/// Minimum distance (px) between two recorded stroke points.
pub const SAMPLE_SPACING: f32 = 4.0;
/// Most points kept for a filled shape (convex decomposition cost).
const MAX_FILLED_POINTS: usize = 64;
/// Most segments kept for a plank (one capsule collider each).
const MAX_OPEN_POINTS: usize = 40;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    /// Outline (filled) or centre line (open), normalised to the sprite box:
    /// (u, v) ∈ [-0.5, 0.5]², v pointing up.
    pub points: Vec<[f32; 2]>,
    pub closed: bool,
    /// Stroke thickness relative to the longest side of the box (open strokes).
    pub thickness: f32,
    /// Box height / width.
    pub aspect: f32,
    pub rgb: [u8; 3],
}

/// A finished stroke: the drawing plus where it sits on screen.
pub struct Placed {
    pub drawing: Drawing,
    pub center: (f32, f32),
    pub size: (f32, f32),
}

/// Whether a stroke in progress would close into a filled shape.
pub fn closes(pts: &[(f32, f32)], thickness: f32) -> bool {
    if pts.len() < 4 {
        return false;
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let gap = ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt();
    let length: f32 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
    gap <= (thickness * 1.5).max(24.0) && length > gap * 4.0 && length > 80.0
}

/// Turn a stroke (screen pixels) into a drawing. Returns `None` for strokes
/// too small to become an object.
pub fn from_stroke(pts: &[(f32, f32)], thickness: f32, rgb: [u8; 3]) -> Option<Placed> {
    if pts.is_empty() {
        return None;
    }
    let thickness = thickness.max(2.0);
    if closes(pts, thickness) {
        let mut outline = pts.to_vec();
        outline.pop();
        let outline = simplify_to(&outline, true, MAX_FILLED_POINTS);
        if outline.len() >= 3 && polygon_area(&outline).abs() > 300.0 {
            return Some(normalise(&outline, true, 0.0, rgb));
        }
    }
    let line = simplify_to(pts, false, MAX_OPEN_POINTS);
    // A click (or a tiny scribble) makes a dot.
    let line = if line.len() < 2 { vec![line[0], line[0]] } else { line };
    Some(normalise(&line, false, thickness, rgb))
}

fn normalise(pts: &[(f32, f32)], closed: bool, thickness: f32, rgb: [u8; 3]) -> Placed {
    let pad = thickness / 2.0;
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for &(x, y) in pts {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x);
        y1 = y1.max(y);
    }
    let (x0, y0, x1, y1) = (x0 - pad, y0 - pad, x1 + pad, y1 + pad);
    let (w, h) = ((x1 - x0).max(4.0), (y1 - y0).max(4.0));
    let (cx, cy) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let points = pts.iter().map(|&(x, y)| [(x - cx) / w, (cy - y) / h]).collect();
    Placed {
        drawing: Drawing { points, closed, thickness: thickness / w.max(h), aspect: h / w, rgb },
        center: (cx, cy),
        size: (w, h),
    }
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Shoelace area (signed).
pub fn polygon_area(pts: &[(f32, f32)]) -> f32 {
    let n = pts.len();
    (0..n).map(|i| pts[i].0 * pts[(i + 1) % n].1 - pts[(i + 1) % n].0 * pts[i].1).sum::<f32>() / 2.0
}

fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    dist(p, (a.0 + dx * t, a.1 + dy * t))
}

/// Ramer–Douglas–Peucker simplification.
fn rdp(pts: &[(f32, f32)], eps: f32, out: &mut Vec<(f32, f32)>) {
    if pts.len() < 3 {
        out.extend_from_slice(&pts[..pts.len().saturating_sub(1)]);
        return;
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (i, d) = pts[1..pts.len() - 1]
        .iter()
        .enumerate()
        .map(|(i, &p)| (i + 1, seg_dist(p, a, b)))
        .fold((0, 0.0), |m, x| if x.1 > m.1 { x } else { m });
    if d > eps {
        rdp(&pts[..=i], eps, out);
        rdp(&pts[i..], eps, out);
    } else {
        out.push(a);
    }
}

/// Simplify until at most `max` points remain.
fn simplify_to(pts: &[(f32, f32)], closed: bool, max: usize) -> Vec<(f32, f32)> {
    let mut eps = 2.0;
    loop {
        let mut out = vec![];
        if closed {
            // Close the loop so the seam is simplified like any other point.
            let mut ring = pts.to_vec();
            ring.push(pts[0]);
            rdp(&ring, eps, &mut out);
        } else {
            rdp(pts, eps, &mut out);
            out.push(pts[pts.len() - 1]);
        }
        if out.len() <= max || eps > 400.0 {
            return out;
        }
        eps *= 1.5;
    }
}

impl Drawing {
    /// Outline scaled to a sprite of `w × h` pixels, y down, origin top-left.
    fn pixels(&self, w: f32, h: f32) -> Vec<(f32, f32)> {
        self.points.iter().map(|&[u, v]| ((u + 0.5) * w, (0.5 - v) * h)).collect()
    }

    /// Rasterise at `max_px` on the longest side, with an
    /// anti-aliased edge, a darker rim and the same sheen as spawned shapes.
    pub fn rasterize(&self, max_px: u32) -> RgbaImage {
        let aspect = self.aspect;
        let longest = max_px.max(4) as f32;
        let (w, h) = if aspect >= 1.0 { (longest / aspect, longest) } else { (longest, longest * aspect) };
        let (w, h) = (w.round().max(4.0) as u32, h.round().max(4.0) as u32);
        let (wf, hf) = (w as f32, h as f32);
        let pts = self.pixels(wf, hf);
        let radius = self.thickness * wf.max(hf) / 2.0;
        let rim = (wf.max(hf) * 0.03).clamp(1.5, 4.0).min(radius * 0.5).max(1.0);
        let band = if self.closed { rim + 1.5 } else { radius + 1.5 };

        // Distance to the outline / centre line, only computed near it.
        let mut d = vec![f32::MAX; (w * h) as usize];
        let n = pts.len();
        let segs = if self.closed { n } else { n.saturating_sub(1).max(1) };
        for i in 0..segs {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let xa = ((a.0.min(b.0) - band).floor().max(0.0)) as u32;
            let xb = ((a.0.max(b.0) + band).ceil().min(wf - 1.0)) as u32;
            let ya = ((a.1.min(b.1) - band).floor().max(0.0)) as u32;
            let yb = ((a.1.max(b.1) + band).ceil().min(hf - 1.0)) as u32;
            for y in ya..=yb {
                for x in xa..=xb {
                    let p = (x as f32 + 0.5, y as f32 + 0.5);
                    let k = (y * w + x) as usize;
                    d[k] = d[k].min(seg_dist(p, a, b));
                }
            }
        }

        let base = [self.rgb[0] as f32, self.rgb[1] as f32, self.rgb[2] as f32];
        let mut img = RgbaImage::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let dk = d[(y * w + x) as usize];
                // Signed distance, positive inside.
                let inside_d = if self.closed {
                    let p = (x as f32 + 0.5, y as f32 + 0.5);
                    if point_in_polygon(p.0, p.1, &pts) {
                        dk
                    } else {
                        -dk
                    }
                } else {
                    radius - dk
                };
                let alpha = (inside_d + 0.5).clamp(0.0, 1.0);
                if alpha <= 0.0 {
                    continue;
                }
                let inner = ((inside_d - rim + 0.5) / 1.5).clamp(0.0, 1.0);
                let sheen = 1.14 - 0.30 * (y as f32 / hf);
                let mut c = [0u8; 4];
                for i in 0..3 {
                    let body = (base[i] * sheen).min(255.0);
                    let edge = base[i] * 0.62;
                    c[i] = (edge + (body - edge) * inner) as u8;
                }
                c[3] = (alpha * 255.0) as u8;
                img.put_pixel(x, y, Rgba(c));
            }
        }
        img
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn circle(n: usize, r: f32) -> Vec<(f32, f32)> {
        (0..=n)
            .map(|i| {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                (300.0 + a.cos() * r, 200.0 + a.sin() * r)
            })
            .collect()
    }

    #[test]
    fn a_loop_becomes_a_filled_shape() {
        let p = from_stroke(&circle(80, 60.0), 10.0, [255, 0, 0]).unwrap();
        assert!(p.drawing.closed);
        assert!((p.size.0 - 120.0).abs() < 2.0 && (p.size.1 - 120.0).abs() < 2.0);
        assert!((p.center.0 - 300.0).abs() < 1.0 && (p.center.1 - 200.0).abs() < 1.0);
        assert!(p.drawing.points.len() <= MAX_FILLED_POINTS);
        assert!(p.drawing.points.iter().all(|q| q[0].abs() <= 0.5 + 1e-4 && q[1].abs() <= 0.5 + 1e-4));
    }

    #[test]
    fn a_line_becomes_a_plank() {
        let line: Vec<_> = (0..50).map(|i| (100.0 + i as f32 * 5.0, 300.0 + (i as f32 * 0.3).sin())).collect();
        let p = from_stroke(&line, 12.0, [0, 255, 0]).unwrap();
        assert!(!p.drawing.closed);
        // Straight line: simplified to (almost) two points, padded by the thickness.
        assert!(p.drawing.points.len() <= 6);
        assert!((p.size.0 - (245.0 + 12.0)).abs() < 1.0);
        assert!(p.size.1 >= 12.0);
    }

    #[test]
    fn a_click_makes_a_dot() {
        let p = from_stroke(&[(10.0, 10.0)], 16.0, [1, 2, 3]).unwrap();
        assert_eq!(p.size, (16.0, 16.0));
        let img = p.drawing.rasterize(32);
        assert_eq!(img.get_pixel(16, 16)[3], 255);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn simplification_caps_points() {
        let zigzag: Vec<_> = (0..500).map(|i| (i as f32 * 3.0, if i % 2 == 0 { 0.0 } else { 40.0 })).collect();
        assert!(simplify_to(&zigzag, false, MAX_OPEN_POINTS).len() <= MAX_OPEN_POINTS);
    }

    #[test]
    fn filled_raster_has_a_solid_middle() {
        let p = from_stroke(&circle(60, 50.0), 8.0, [200, 100, 50]).unwrap();
        let img = p.drawing.rasterize(100);
        assert_eq!(img.get_pixel(50, 50)[3], 255);
        assert_eq!(img.get_pixel(1, 1)[3], 0);
    }
}
