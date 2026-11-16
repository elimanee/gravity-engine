//! The endless 3D run: a road of grass on earth hanging over the sea,
//! made a stretch at a time as he runs (a new course every run). It bends,
//! climbs and dips, leans into its bends, and has its moments: ramps over
//! gaps, loops (coming out to the side), springs up to a higher road, dash
//! panels, lines of rings and badniks.

use super::gfx::{cylinder, sphere, Shape};
use super::world::{Badnik, BadnikKind, Maker, Panel, Skin, Spring, Stretch};
use super::zone::loop_shape;
use macroquad::prelude::*;

/// The road's width and its points' spacing (m).
pub const WIDTH: f32 = 10.0;
const STEP: f32 = 2.0;
/// How deep the earth under the road goes.
const DEPTH: f32 = 5.0;

/// A point along the way: where, which way, how leaned over.
#[derive(Clone, Copy, Debug)]
struct Frame {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    bank: f32,
}

impl Frame {
    fn forward(&self) -> Vec3 {
        vec3(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos())
    }

    fn flat_forward(&self) -> Vec3 {
        vec3(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    fn right(&self) -> Vec3 {
        let flat = vec3(-self.yaw.sin(), 0.0, self.yaw.cos());
        (flat * self.bank.cos() + Vec3::Y * self.bank.sin()).normalize()
    }

    fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }
}

/// The moments a stretch can have.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    Bends,
    Hills,
    Gap,
    Loop,
    StepUp,
    Dash,
    Badniks,
}

pub struct Course {
    rng: u32,
    at: Frame,
    along: f32,
    yaw_rate: f32,
    /// How fast the road comes round to what is asked of it (0‥1 a step):
    /// gently as a rule, quickly before a ramp, a loop or a step.
    ease: f32,
    count: u32,
    last: Option<Feature>,
}

/// The road's usual ease, and its ease where it must be straight and level
/// soon.
const GENTLE: f32 = 0.08;
const QUICK: f32 = 0.3;

impl Course {
    pub fn new(seed: u32) -> Course {
        Course {
            rng: seed.max(1),
            at: Frame { pos: Vec3::ZERO, yaw: 0.0, pitch: 0.0, bank: 0.0 },
            along: 0.0,
            yaw_rate: 0.0,
            ease: GENTLE,
            count: 0,
            last: None,
        }
    }

    fn next_f(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng % 100_000) as f32 / 100_000.0
    }

    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next_f()
    }

    fn pick(&mut self) -> Feature {
        use Feature::*;
        if self.count < 2 {
            return Bends;
        }
        let all = [Bends, Hills, Gap, Loop, StepUp, Dash, Badniks];
        loop {
            let f = all[(self.next_f() * all.len() as f32) as usize % all.len()];
            if Some(f) != self.last {
                return f;
            }
        }
    }

    /// Road along `frames` (top, earth under it, ends closed).
    fn road(st: &mut Stretch, frames: &[(Frame, f32)]) {
        if frames.len() < 2 {
            return;
        }
        let mut top = Shape::default();
        let mut sides = Shape::default();
        let half = WIDTH / 2.0;
        for (f, d) in frames {
            let (r, u) = (f.right(), f.up());
            let v = d / 4.0;
            top.vertex(f.pos - r * half, u, vec2(0.0, v), WHITE);
            top.vertex(f.pos + r * half, u, vec2(WIDTH / 4.0, v), WHITE);
            // The edges of the earth: from the road's edge down.
            for (edge, out) in [(f.pos - r * half, -r), (f.pos + r * half, r)] {
                sides.vertex(edge, out, vec2(0.0, v), WHITE);
                sides.vertex(edge - Vec3::Y * DEPTH, out, vec2(DEPTH / 4.0, v), WHITE);
            }
        }
        for k in 0..frames.len() as u32 - 1 {
            let (a, b) = (k * 2, k * 2 + 2);
            top.tri(a, b + 1, b);
            top.tri(a, a + 1, b + 1);
            let (a, b) = (k * 4, k * 4 + 4);
            sides.tri(a, b, b + 1);
            sides.tri(a, b + 1, a + 1);
            sides.tri(a + 2, b + 3, b + 2);
            sides.tri(a + 2, a + 3, b + 3);
        }
        // Close the ends (a wall to spring up past, at a step).
        for (f, _) in [frames[0], frames[frames.len() - 1]] {
            let r = f.right() * half;
            let down = Vec3::Y * DEPTH;
            let n = -f.flat_forward();
            let p = [f.pos - r, f.pos + r, f.pos + r - down, f.pos - r - down];
            let uv = [vec2(0.0, 0.0), vec2(2.5, 0.0), vec2(2.5, 1.2), vec2(0.0, 1.2)];
            let i: Vec<u32> = (0..4).map(|k| sides.vertex(p[k], n, uv[k], WHITE)).collect();
            sides.tri(i[0], i[1], i[2]);
            sides.tri(i[0], i[2], i[3]);
        }
        st.ground.push((top, Skin::Grass));
        st.ground.push((sides, Skin::Soil));
    }

    /// Go on `len` metres, turning, climbing and leaning as asked; the
    /// frames passed (to build road on, or not, over a gap).
    fn go(&mut self, st: &mut Stretch, len: f32, yaw_rate: f32, pitch: f32, bank: Option<f32>) -> Vec<(Frame, f32)> {
        let mut out = vec![(self.at, self.along)];
        let steps = (len / STEP).ceil() as usize;
        for _ in 0..steps {
            self.yaw_rate += (yaw_rate - self.yaw_rate) * self.ease;
            self.at.yaw += self.yaw_rate * STEP;
            self.at.pitch += (pitch - self.at.pitch) * self.ease;
            let lean = bank.unwrap_or((-self.yaw_rate * 9.0).clamp(-0.4, 0.4));
            self.at.bank += (lean - self.at.bank) * 0.15;
            self.at.pos += self.at.forward() * STEP;
            self.along += STEP;
            out.push((self.at, self.along));
            st.path.push((self.at.pos, self.along));
        }
        out
    }

    /// Dash panels right across the road (no way round them).
    fn panels(st: &mut Stretch, f: &Frame) {
        st.panels.push(Panel { at: f.pos, dir: f.forward(), up: f.up(), half: WIDTH / 2.0 });
    }

    /// A line of rings over frames `from..to` (every few), `side` metres
    /// to the right.
    fn rings(st: &mut Stretch, frames: &[(Frame, f32)], side: f32) {
        for (f, _) in frames.iter().step_by(2) {
            st.rings.push(f.pos + f.right() * side + f.up() * 1.0);
        }
    }

    fn decor(&mut self, st: &mut Stretch, frames: &[(Frame, f32)]) {
        // Sunflowers and totems along the edges, now and then.
        for (k, (f, _)) in frames.iter().enumerate() {
            if k % 7 != 3 || self.next_f() < 0.5 {
                continue;
            }
            let side = if self.next_f() < 0.5 { -1.0 } else { 1.0 };
            let at = f.pos + f.right() * side * (WIDTH / 2.0 - 0.6);
            let mut s = cylinder(at, 0.0, 1.6, 0.06, 0.06, 6, Color::new(0.2, 0.55, 0.2, 1.0));
            s.append(
                &sphere(at + Vec3::Y * 1.7, vec3(0.45, 0.45, 0.12), 5, 10, Color::new(1.0, 0.82, 0.1, 1.0)),
                Mat4::IDENTITY,
            );
            s.append(
                &sphere(at + Vec3::Y * 1.7, vec3(0.2, 0.2, 0.16), 4, 8, Color::new(0.55, 0.3, 0.1, 1.0)),
                Mat4::IDENTITY,
            );
            st.decor.push((s, Skin::Plain));
        }
    }

    fn feature(&mut self, st: &mut Stretch, f: Feature) {
        match f {
            Feature::Bends => {
                for _ in 0..2 {
                    let rate = self.range(-0.035, 0.035);
                    let pitch = self.range(-0.12, 0.12);
                    let len = self.range(30.0, 50.0);
                    let fr = self.go(st, len, rate, pitch, None);
                    Self::road(st, &fr);
                    let side = self.range(-3.0, 3.0);
                    Self::rings(st, &fr[fr.len() / 3..fr.len() * 2 / 3], side);
                    self.decor(st, &fr);
                }
            }
            Feature::Hills => {
                let mut all = vec![];
                for k in 0..4 {
                    let pitch = if k % 2 == 0 { self.range(0.18, 0.3) } else { -self.range(0.2, 0.35) };
                    let rate = self.range(-0.01, 0.01);
                    let fr = self.go(st, 24.0, rate, pitch, None);
                    all.extend(if all.is_empty() { fr } else { fr[1..].to_vec() });
                }
                Self::road(st, &all);
                Self::rings(st, &all[all.len() / 2..], 0.0);
                self.decor(st, &all);
            }
            Feature::Gap => {
                // Straight and level, dash panels, a ramp, the gap, and the
                // road again a little lower.
                self.ease = QUICK;
                let mut fr = self.go(st, 24.0, 0.0, 0.0, Some(0.0));
                Self::panels(st, &fr[5].0);
                fr.extend(self.go(st, 12.0, 0.0, 0.3, Some(0.0))[1..].iter().copied());
                Self::road(st, &fr);
                let lip = self.at;
                let gap = self.range(8.0, 13.0);
                self.at.pitch = 0.0;
                let over = self.go(st, gap, 0.0, 0.0, Some(0.0));
                // Rings in an arc over the gap.
                for (k, (f, _)) in over.iter().enumerate() {
                    let t = k as f32 / (over.len() - 1).max(1) as f32;
                    st.rings.push(f.pos + Vec3::Y * (2.5 + (t * std::f32::consts::PI).sin() * 3.0));
                }
                self.at.pos.y = lip.pos.y - self.range(1.0, 3.0);
                let land = self.go(st, 36.0, 0.0, 0.0, Some(0.0));
                Self::road(st, &land);
                self.ease = GENTLE;
            }
            Feature::Loop => {
                self.ease = QUICK;
                let mut fr = self.go(st, 30.0, 0.0, 0.0, Some(0.0));
                self.at.pitch = 0.0;
                self.at.bank = 0.0;
                if let Some(last) = fr.last_mut() {
                    last.0 = self.at;
                }
                Self::panels(st, &fr[5].0);
                Self::road(st, &fr);
                Self::rings(st, &fr[7..fr.len() - 2], 0.0);
                let shift = WIDTH + 2.0;
                let (top, under, l) = loop_shape(self.at.pos, self.at.flat_forward(), 7.5, WIDTH, shift);
                st.ground.push((top, Skin::Soil));
                st.decor.push((under, Skin::Soil));
                st.loops.push(l);
                for k in 1..8 {
                    let t = k as f32 / 8.0 * std::f32::consts::TAU;
                    let at = l.start
                        + l.forward * (t.sin() * 6.5)
                        + Vec3::Y * (7.5 - t.cos() * 6.5)
                        + l.side * (shift * t / std::f32::consts::TAU);
                    st.rings.push(at);
                }
                // The path goes round with him (for the distance), then on
                // from beside the way in.
                let r = self.at.right();
                self.at.pos += r * shift;
                self.along += 7.5 * std::f32::consts::TAU;
                st.path.push((self.at.pos, self.along));
                let out = self.go(st, 24.0, 0.0, 0.0, Some(0.0));
                Self::road(st, &out);
                self.ease = GENTLE;
            }
            Feature::StepUp => {
                // Springs right across the road throw him up and on, onto
                // the road that goes on higher up past the wall.
                self.ease = QUICK;
                let fr = self.go(st, 30.0, 0.0, 0.0, Some(0.0));
                let p = fr[fr.len() - 4].0;
                let throw = p.flat_forward() * 4.0 + Vec3::Y * 13.0;
                for k in 0..6 {
                    st.springs.push(Spring {
                        at: p.pos + p.right() * (k as f32 * 1.7 - 4.25),
                        up: throw.normalize(),
                        power: throw.length(),
                        red: true,
                        squash: 0.0,
                    });
                }
                Self::road(st, &fr);
                // The wall: from under the road he is on up to the one above.
                let rise = self.range(5.0, 7.0);
                let (low, r) = (self.at, self.at.right() * (WIDTH / 2.0));
                let mut wall = Shape::default();
                let (a, b) = (low.pos - r - Vec3::Y * DEPTH, low.pos + r - Vec3::Y * DEPTH);
                let up = Vec3::Y * (rise + DEPTH);
                let uv = [vec2(0.0, 0.0), vec2(2.5, 0.0), vec2(2.5, 3.0), vec2(0.0, 3.0)];
                wall.quad([b, a, a + up, b + up], uv, WHITE);
                st.ground.push((wall, Skin::Soil));
                self.at.pos.y += rise;
                self.ease = GENTLE;
                let rate = self.range(-0.01, 0.01);
                let up = self.go(st, 40.0, rate, 0.0, None);
                // Rings up where the springs send him.
                for k in 1..6 {
                    let t = k as f32 * 0.22;
                    let at = p.pos + Vec3::Y * 1.5 + throw * (t * 2.4) - Vec3::Y * (15.75 * t * t);
                    st.rings.push(at);
                }
                Self::road(st, &up);
                self.decor(st, &up);
            }
            Feature::Dash => {
                let rate = self.range(-0.02, 0.02);
                let fr = self.go(st, 60.0, rate, -0.08, None);
                for k in [2, 14] {
                    Self::panels(st, &fr[k].0);
                }
                Self::road(st, &fr);
                Self::rings(st, &fr[4..], -2.5);
                Self::rings(st, &fr[4..], 2.5);
            }
            Feature::Badniks => {
                let rate = self.range(-0.02, 0.02);
                let fr = self.go(st, 60.0, rate, 0.0, None);
                for k in [6, 14, 22] {
                    let (p, _) = fr[k.min(fr.len() - 1)];
                    let kind = if k == 14 { BadnikKind::Flyer } else { BadnikKind::Crawler };
                    let at = if kind == BadnikKind::Flyer { p.pos + p.up() * 1.6 } else { p.pos };
                    st.badniks.push(Badnik { kind, at, home: at, dir: p.right(), t: self.range(0.0, 600.0) });
                }
                Self::road(st, &fr);
                Self::rings(st, &fr[2..fr.len() - 2], self.range(-3.0, 3.0));
                self.decor(st, &fr);
            }
        }
    }
}

impl Maker for Course {
    fn endless(&self) -> bool {
        true
    }

    fn start(&self) -> (Vec3, Vec3) {
        (vec3(4.0, 0.0, 0.0), Vec3::X)
    }

    fn next(&mut self) -> Option<Stretch> {
        let mut st = Stretch::default();
        let f = self.pick();
        self.feature(&mut st, f);
        self.last = Some(f);
        self.count += 1;
        Some(st)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_course_goes_on_with_every_kind_of_stretch_and_he_starts_on_road() {
        let mut c = Course::new(42);
        let mut seen = std::collections::HashSet::new();
        let mut last_along = 0.0;
        for _ in 0..80 {
            let f = c.pick();
            let mut st = Stretch::default();
            c.feature(&mut st, f);
            c.last = Some(f);
            c.count += 1;
            seen.insert(f);
            assert!(!st.ground.is_empty() && !st.path.is_empty());
            // The path goes on.
            for (_, d) in &st.path {
                assert!(*d > last_along);
                last_along = *d;
            }
            for (s, _) in &st.ground {
                assert!(s.tris.iter().all(|t| t.iter().all(|&i| (i as usize) < s.pos.len())));
            }
        }
        assert_eq!(seen.len(), 7, "every kind: {seen:?}");
        // Another seed, another course.
        let mut a = Course::new(1);
        let mut b = Course::new(2);
        let (sa, sb) = (a.next().unwrap(), b.next().unwrap());
        let (pa, pb) = (sa.path.last().unwrap().0, sb.path.last().unwrap().0);
        assert!((pa - pb).length() > 0.5);
    }

    /// Which stretch is at `along` metres of the course from `seed`.
    fn feature_at(seed: u32, along: f32) -> (Feature, f32) {
        let mut c = Course::new(seed);
        loop {
            let from = c.along;
            c.next();
            if c.along >= along {
                return (c.last.unwrap(), along - from);
            }
        }
    }

    #[test]
    fn someone_who_follows_the_road_gets_a_long_way_on_any_course() {
        use super::super::world::{Input, World3};
        use crate::sonic::Character;
        let mut short = vec![];
        for seed in 13..=20 {
            let mut w = World3::new(Box::new(Course::new(seed)), Character::Sonic, 0.0);
            let mut was = false;
            for _ in 0..60 * 150 {
                let stick = w.autopilot(12.0);
                // Jumps on the badniks in the way, as anyone would.
                let jump = w.badnik_ahead();
                w.step(Input { stick, jump, jump_pressed: jump && !was, ..Input::default() });
                was = jump;
                if w.over || w.distance > 2500.0 {
                    break;
                }
            }
            if w.distance < 2500.0 {
                short.push((seed, w.distance, feature_at(seed, w.distance), w.over, w.runner.pos));
            }
        }
        assert!(short.is_empty(), "runs that ended early: {short:#?}");
    }

    #[test]
    fn holding_forward_alone_follows_the_road_a_long_way() {
        use super::super::world::{Input, World3};
        use crate::sonic::Character;
        let mut short = vec![];
        for seed in 1..=12 {
            let mut w = World3::new(Box::new(Course::new(seed)), Character::Sonic, 0.0);
            let mut was = false;
            for _ in 0..60 * 150 {
                let jump = w.badnik_ahead();
                w.step(Input { stick: vec2(0.0, 1.0), jump, jump_pressed: jump && !was, ..Input::default() });
                was = jump;
                if w.over || w.distance > 2500.0 {
                    break;
                }
            }
            if w.distance < 2500.0 {
                short.push((seed, w.distance, feature_at(seed, w.distance), w.runner.pos, w.off_road()));
            }
        }
        assert!(short.is_empty(), "runs that ended early: {short:#?}");
    }

    #[test]
    fn he_runs_along_the_road_and_round_its_loops() {
        use super::super::phys::Phys;
        use super::super::player::{Controls, Runner};
        use crate::sonic::Character;
        // A loop straight away.
        let mut c = Course::new(9);
        let mut st = Stretch::default();
        c.feature(&mut st, Feature::Loop);
        let mut p = Phys::new();
        for (s, _) in &st.ground {
            p.add_mesh(&s.pos, &s.tris);
        }
        p.refresh();
        let mut r = Runner::new(Character::Sonic, vec3(1.0, 0.0, 0.0), Vec3::X);
        r.vel = Vec3::X * 12.0;
        let l = st.loops[0];
        let mut upside_down = false;
        let mut progress: Option<f32> = None;
        for _ in 0..400 {
            r.frame(&p, Controls { dir: Vec3::X, ..Controls::default() });
            upside_down |= r.up.y < -0.9;
            // The world's guide, as it does it.
            if !r.grounded {
                progress = None;
            } else if let Some(now) = progress {
                progress = l.guide(now, r.pos).map(|(now, at)| {
                    r.pos = at;
                    now
                });
            } else {
                progress = l.entering(r.pos, r.vel);
            }
            if upside_down && r.pos.x > l.start.x + 12.0 {
                break;
            }
        }
        assert!(upside_down, "went round");
        assert!(r.grounded && r.pos.x > l.start.x + 12.0, "out the other side: {:?}", r.pos);
        assert!(((r.pos - l.start).dot(l.side) - l.shift).abs() < 1.5, "beside the way in: {:?}", r.pos);
    }
}
