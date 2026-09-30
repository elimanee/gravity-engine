//! What collisions and explosions do besides pushing things: particle
//! effects, and breakable objects shattering into pieces.

use super::juice::{HITS_PER_FRAME, HIT_SOUND_SPEED, SLOW_IMPACT};
use super::*;
use crate::audio::sfx::Sound;
use crate::physics::fracture;
use crate::physics::to_screen;
use rapier2d::prelude::ColliderHandle;

/// Impacts (speed change, m/s) above these make dust / sparks.
const DUST_SPEED: f32 = 4.5;
const SPARK_SPEED: f32 = 9.0;
/// Most impacts turned into effects per frame.
const EFFECTS_PER_FRAME: usize = 10;
/// Most objects shattered per frame (each one decodes and encodes images).
const BREAKS_PER_FRAME: usize = 3;
/// New pieces cannot break again for this long (s), so a shatter does not cascade.
const FRESH_SECS: f64 = 0.5;

impl App {
    /// Handle the impacts of this frame's physics steps.
    pub(super) fn process_impacts(&mut self) {
        let impacts = self.world.take_impacts();
        let now = get_time();
        self.fresh.retain(|_, t| now - *t < FRESH_SECS);
        if impacts.is_empty() {
            return;
        }
        let index: HashMap<ColliderHandle, usize> =
            self.objects.iter().enumerate().map(|(i, o)| (o.collider, i)).collect();

        // (speed, point, normal, size, hardness) of each impact, and the objects it breaks.
        let mut hits = Vec::with_capacity(impacts.len());
        let mut breaks: Vec<(RigidBodyHandle, Point<f32>)> = vec![];
        for imp in impacts {
            let sides = [index.get(&imp.collider1).copied(), index.get(&imp.collider2).copied()];
            let mass = |side: Option<usize>| {
                side.and_then(|i| self.world.bodies.get(self.objects[i].body))
                    .filter(|b| b.is_dynamic())
                    .map(|b| b.mass().max(1e-3))
            };
            let masses = [mass(sides[0]), mass(sides[1])];
            // Speed change of each side; a pinned object feels what it did to the other one.
            let dv = |k: usize| masses[k].or(masses[1 - k]).map_or(0.0, |m| imp.impulse / m);
            let speed =
                masses.iter().zip([dv(0), dv(1)]).filter(|(m, _)| m.is_some()).map(|(_, v)| v).fold(0.0, f32::max);
            // What it sounds like: the larger object's size, the bouncier material.
            let objs = sides.iter().flatten().map(|&i| &self.objects[i]);
            let size = objs.clone().map(|o| o.size.x.max(o.size.y)).fold(0.0, f32::max);
            let hard = objs.map(|o| o.material.bounce).fold(0.0, f32::max) * 1.3 + 0.25;
            hits.push((speed, imp.point, imp.normal, size, hard));
            for (k, side) in sides.iter().enumerate() {
                let Some(i) = *side else { continue };
                let o = &self.objects[i];
                if o.material.breakable && dv(k) > o.material.strength && !self.fresh.contains_key(&o.body) {
                    breaks.push((o.body, imp.point));
                }
            }
        }

        hits.sort_by(|a, b| b.0.total_cmp(&a.0));
        for &(speed, point, _, size, hard) in hits.iter().take(HITS_PER_FRAME) {
            if speed < HIT_SOUND_SPEED || size <= 0.0 {
                break;
            }
            let volume = ((speed - HIT_SOUND_SPEED) / 14.0).min(1.0).powf(0.7) * 0.9;
            self.sound(Sound::Hit { size, hard }, to_screen(point.x, point.y), volume);
        }
        if hits.first().is_some_and(|h| h.0 > SLOW_IMPACT) {
            self.slow_motion(0.5);
        }
        if self.s.effects {
            for &(speed, point, normal, ..) in hits.iter().take(EFFECTS_PER_FRAME) {
                let at = to_screen(point.x, point.y);
                let n = vec2(normal.x, -normal.y);
                if speed > SPARK_SPEED {
                    self.effects.sparks(at, n, speed);
                    self.effects.sparks(at, -n, speed * 0.7);
                } else if speed > DUST_SPEED {
                    self.effects.dust(at, speed);
                } else {
                    break;
                }
            }
        }

        let mut done = vec![];
        for (body, point) in breaks {
            if done.len() >= BREAKS_PER_FRAME || done.contains(&body) {
                continue;
            }
            if self.shatter(body, point) {
                done.push(body);
            }
        }
    }

    /// Explosion aftermath: flash, and breakable objects kicked hard enough shatter.
    pub(super) fn bomb_hits(&mut self, at: Vec2, hits: Vec<(RigidBodyHandle, f32)>) {
        let force = (self.s.tool_strength / 600.0).clamp(0.2, 1.0);
        self.sound(Sound::Boom, at, 0.5 + force * 0.5);
        if hits.len() >= 3 {
            self.slow_motion(force);
        }
        if self.s.effects {
            self.effects.explosion(at, self.s.tool_radius * 0.35);
        }
        let blast = to_phys(at.x, at.y);
        let mut broken = 0;
        for (body, speed) in hits {
            let Some(o) = self.objects.iter().find(|o| o.body == body) else { continue };
            if broken < BREAKS_PER_FRAME && o.material.breakable && speed > o.material.strength {
                // Crack on the side facing the blast.
                let c = *self.world.bodies[body].translation();
                let (dx, dy) = vector_to(c.x, c.y, blast.0, blast.1);
                let reach = o.size.x.min(o.size.y) / PPM * 0.35;
                let toward = (dx * reach, dy * reach);
                if self.shatter(body, Point::new(c.x + toward.0, c.y + toward.1)) {
                    broken += 1;
                }
            }
        }
    }

    /// Replace object `i` by `pieces` of its sprite, moving like it did.
    /// They fly apart from its centre, or along ±`apart` (a screen
    /// direction) when given.
    pub(super) fn replace_with_pieces(&mut self, i: usize, pieces: Vec<fracture::Piece>, apart: Option<Vec2>) {
        let o = &self.objects[i];
        let Some(b) = self.world.bodies.get(o.body) else { return };
        let (pos, angle) = o.screen_pos(&self.world);
        let (v, w) = (*b.linvel(), b.angvel());
        let size = o.size;
        let material = o.material;
        let base = o.name();
        let name = format!("{} (piece)", base.trim_end_matches(" (piece)"));
        self.remove_object(i);

        let (sin, cos) = (-angle).sin_cos();
        let now = get_time();
        for p in pieces {
            let (x, y) = (p.offset.0 * size.x, p.offset.1 * size.y);
            let at = pos + vec2(x * cos - y * sin, x * sin + y * cos);
            let piece_size = (p.scale.0 * size.x, p.scale.1 * size.y);
            // Rigid-body velocity at the piece plus a small kick.
            let r = ((at.x - pos.x) / PPM, -(at.y - pos.y) / PPM);
            let kick = match apart {
                Some(n) => {
                    let side = if (at - pos).dot(n) >= 0.0 { 1.0 } else { -1.0 };
                    (n.x * side * 0.8, -n.y * side * 0.8)
                }
                None => {
                    let len = (r.0 * r.0 + r.1 * r.1).sqrt().max(0.05);
                    (r.0 / len * 1.2, r.1 / len * 1.2)
                }
            };
            let linvel = (v.x - w * r.1 + kick.0, v.y + w * r.0 + kick.1);
            let breakable = material.breakable && piece_size.0.min(piece_size.1) > 40.0;
            let Some(png) = fracture::to_png(&p.image) else { continue };
            let placement = Placement {
                pos_px: (at.x, at.y),
                angle,
                linvel,
                angvel: w,
                size_px: Some(piece_size),
                material: Material { breakable, ..material },
                ..Default::default()
            };
            let source = Source::Memory { name: name.clone(), data: Arc::new(png) };
            if let Some(piece) = Object::load(&mut self.world, source, placement) {
                self.fresh.insert(piece.body, now);
                self.objects.push(piece);
            }
        }
    }

    /// Break an object into pieces around `point` (world metres). Returns
    /// whether it broke.
    pub(super) fn shatter(&mut self, body: RigidBodyHandle, point: Point<f32>) -> bool {
        let Some(i) = self.objects.iter().position(|o| o.body == body) else { return false };
        let o = &self.objects[i];
        if o.is_visualizer() || o.size.x.min(o.size.y) < 18.0 {
            return false;
        }
        let longest = o.size.x.max(o.size.y).ceil() as u32;
        let Some(dec) = o.source.decode(longest.clamp(8, 1024), true) else { return false };
        let img = &dec.frames[o.frame().min(dec.frames.len() - 1)];
        let Some(b) = self.world.bodies.get(body) else { return false };
        let local = b.position().inverse_transform_point(&point);
        let impact =
            ((local.x * PPM / o.size.x + 0.5).clamp(0.0, 1.0), (0.5 - local.y * PPM / o.size.y).clamp(0.0, 1.0));
        let seed = (get_time() * 1e6) as u64 ^ body.into_raw_parts().0 as u64;
        let pieces = fracture::shatter(img, impact, fracture::piece_count(o.size.x, o.size.y), seed);
        if pieces.len() < 2 {
            return false;
        }

        let size = o.size;
        let colour = average_colour(img);
        self.replace_with_pieces(i, pieces, None);
        if self.s.effects {
            self.effects.debris(to_screen(point.x, point.y), size.x.max(size.y) / 2.0, colour);
        }
        self.sound(Sound::Shatter, to_screen(point.x, point.y), 0.8);
        self.slow_motion(1.0);
        true
    }
}

/// Unit vector from (x0, y0) toward (x1, y1).
fn vector_to(x0: f32, y0: f32, x1: f32, y1: f32) -> (f32, f32) {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = (dx * dx + dy * dy).sqrt().max(1e-4);
    (dx / len, dy / len)
}

/// Mean colour of the opaque pixels (for debris).
pub(super) fn average_colour(img: &image::RgbaImage) -> Color {
    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    for p in img.pixels().filter(|p| p[3] > 128) {
        r += p[0] as u64;
        g += p[1] as u64;
        b += p[2] as u64;
        n += 1;
    }
    if n == 0 {
        return GRAY;
    }
    Color::from_rgba((r / n) as u8, (g / n) as u8, (b / n) as u8, 255)
}
