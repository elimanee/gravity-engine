//! The Knife tool: drag across the scene. Ropes and springs the blade
//! crosses snap at once, hinges, motors and glue come apart when it passes
//! over them, and objects it goes right through are sliced in two when
//! the button is released.

use super::*;
use crate::audio::sfx::Sound;
use crate::physics::fracture;
use crate::physics::to_screen;
use std::collections::VecDeque;

/// How long the blade's trail stays visible (s).
const TRAIL_SECS: f64 = 0.22;
/// Pivots (hinges, motors, glue) this close to the blade are cut (px).
const PIVOT_REACH: f32 = 10.0;
/// Objects smaller than this are not sliced (px).
const MIN_SIZE: f32 = 14.0;

#[derive(Default)]
pub(super) struct Knife {
    /// Where the current cut started (world px).
    start: Option<Vec2>,
    last: Option<Vec2>,
    trail: VecDeque<(Vec2, f64)>,
    /// An undo step was recorded for this cut.
    recorded: bool,
}

/// Whether segments `a`–`b` and `c`–`d` cross.
pub(super) fn segments_cross(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let cross = |o: Vec2, p: Vec2, q: Vec2| (p - o).perp_dot(q - o);
    let (d1, d2) = (cross(c, d, a), cross(c, d, b));
    let (d3, d4) = (cross(a, b, c), cross(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

impl App {
    pub(super) fn knife_press(&mut self, m: Vec2) {
        self.knife.start = Some(m);
        self.knife.last = Some(m);
        self.knife.recorded = false;
        self.knife.trail.push_back((m, get_time()));
    }

    /// Pointer held: cut the links the blade crossed since the last frame.
    pub(super) fn knife_drag(&mut self, m: Vec2) {
        let Some(last) = self.knife.last else { return };
        if last.distance(m) < 1.5 {
            return;
        }
        self.knife.trail.push_back((m, get_time()));
        self.knife.last = Some(m);
        let cut: Vec<usize> = (0..self.links.len())
            .filter(|&i| {
                let l = &self.links[i];
                let Some((pa, pb)) = l.ends(&self.world) else { return false };
                let (a, b) = (to_screen(pa.x, pa.y), to_screen(pb.x, pb.y));
                if l.kind.is_pivot() {
                    distance_to_segment(a, last, m) < PIVOT_REACH
                } else {
                    segments_cross(last, m, a, b)
                }
            })
            .collect();
        let cloth = self.softs.iter().any(|s| {
            s.kind == crate::physics::soft::SoftKind::Cloth
                && (0..s.edges.len())
                    .any(|e| s.edge_ends(&self.world, e).is_some_and(|(a, b)| segments_cross(last, m, a, b)))
        });
        if cut.is_empty() && !cloth {
            return;
        }
        if !self.knife.recorded {
            self.record("Cut");
            self.knife.recorded = true;
        }
        self.knife_cloth(last, m);
        for &i in cut.iter().rev() {
            let l = self.links.remove(i);
            l.remove(&mut self.world);
        }
        self.sound(Sound::Snap, m, 0.7);
        if self.s.effects {
            self.effects.dust(m, 5.0);
        }
    }

    /// Button released: slice every object the blade went right through.
    pub(super) fn knife_release(&mut self, m: Vec2) {
        let Some(start) = self.knife.start.take() else { return };
        self.knife.last = None;
        if start.distance(m) < 12.0 {
            return;
        }
        let crossed: Vec<RigidBodyHandle> = self
            .objects
            .iter()
            .filter(|o| !o.is_visualizer() && o.size.x.min(o.size.y) >= MIN_SIZE)
            .filter(|o| {
                // Both ends outside, some point of the stroke inside.
                let inside = |p: Vec2| o.contains_px(&self.world, p.x, p.y);
                if inside(start) || inside(m) {
                    return false;
                }
                let n = (start.distance(m) / 3.0).ceil() as usize;
                (1..n).any(|k| inside(start.lerp(m, k as f32 / n as f32)))
            })
            .map(|o| o.body)
            .collect();
        let mut sliced = 0;
        for body in crossed {
            if !self.knife.recorded {
                self.record("Cut");
                self.knife.recorded = true;
            }
            if self.slice_object(body, start, m) {
                sliced += 1;
            }
        }
        if sliced > 0 {
            self.sound(Sound::Slice, m, 0.8);
        }
    }

    /// Cut object `body` along the line through `a` and `b` (world px).
    fn slice_object(&mut self, body: RigidBodyHandle, a: Vec2, b: Vec2) -> bool {
        let Some(i) = self.objects.iter().position(|o| o.body == body) else { return false };
        let o = &self.objects[i];
        let longest = o.size.x.max(o.size.y).ceil() as u32;
        let Some(dec) = o.source.decode(longest.clamp(8, 1024), true) else { return false };
        let img = &dec.frames[o.frame().min(dec.frames.len() - 1)];
        let Some(rb) = self.world.bodies.get(body) else { return false };
        // The cut line in sprite pixels.
        let to_px = |p: Vec2| {
            let (x, y) = to_phys(p.x, p.y);
            let local = rb.position().inverse_transform_point(&Point::new(x, y));
            let u = local.x * PPM / o.size.x + 0.5;
            let v = 0.5 - local.y * PPM / o.size.y;
            (u * img.width() as f32, v * img.height() as f32)
        };
        let pieces = fracture::slice(img, to_px(a), to_px(b));
        if pieces.len() < 2 {
            return false;
        }
        let dir = (b - a).normalize_or_zero();
        let colour = super::impacts::average_colour(img);
        let centre = o.screen_pos(&self.world).0;
        self.replace_with_pieces(i, pieces, Some(vec2(-dir.y, dir.x)));
        if self.s.effects {
            self.effects.debris(centre, 10.0, colour);
        }
        true
    }

    /// The blade's fading trail (world pass).
    pub(super) fn draw_knife(&mut self) {
        let now = get_time();
        while self.knife.trail.front().is_some_and(|(_, t)| now - t > TRAIL_SECS) && self.knife.start.is_none() {
            self.knife.trail.pop_front();
        }
        while self.knife.trail.len() > 64 {
            self.knife.trail.pop_front();
        }
        let pts: Vec<(Vec2, f64)> = self.knife.trail.iter().copied().collect();
        for w in pts.windows(2) {
            let ((a, ta), (b, _)) = (w[0], w[1]);
            let fade =
                if self.knife.start.is_some() { 1.0 } else { (1.0 - (now - ta) / TRAIL_SECS).clamp(0.0, 1.0) as f32 };
            draw_line(a.x, a.y, b.x, b.y, 5.0 * fade + 1.0, Color::new(0.8, 0.9, 1.0, 0.25 * fade));
            draw_line(a.x, a.y, b.x, b.y, 2.0 * fade + 0.5, Color::new(1.0, 1.0, 1.0, 0.9 * fade));
        }
        if self.knife.start.is_none() && self.knife.trail.len() == 1 {
            self.knife.trail.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_geometry() {
        let (a, b) = (vec2(0.0, 0.0), vec2(10.0, 10.0));
        assert!(segments_cross(a, b, vec2(0.0, 10.0), vec2(10.0, 0.0)));
        assert!(!segments_cross(a, b, vec2(20.0, 0.0), vec2(30.0, 10.0)));
        assert!((distance_to_segment(vec2(0.0, 10.0), a, b) - 50f32.sqrt()).abs() < 1e-4);
        assert_eq!(distance_to_segment(vec2(-3.0, -4.0), a, b), 5.0);
    }
}
