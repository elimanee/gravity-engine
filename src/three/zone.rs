//! The open zone: rolling green hills to run about freely (a height field
//! ringed by mountains), with a loop, ramps, platforms reached by springs,
//! dash panels, lines of rings, badniks, trees, and piles of crates and
//! barrels to knock over.

use super::gfx::{cuboid, cylinder, sphere, Shape};
use super::phys::Solid;
use super::world::{Badnik, BadnikKind, Loop, Maker, Panel, Skin, Spring, Stretch};
use macroquad::prelude::*;

/// Cells across, metres a cell.
const CELLS: usize = 96;
const CELL: f32 = 4.0;
const SIZE: f32 = CELLS as f32 * CELL;

pub struct Zone {
    seed: u32,
    made: bool,
}

impl Zone {
    pub fn new(seed: u32) -> Zone {
        Zone { seed, made: false }
    }
}

/// Smooth bumps from a seed.
fn noise(seed: u32, x: f32, z: f32) -> f32 {
    let s = (seed % 1000) as f32 * 0.37;
    (x * 0.021 + s).sin() * (z * 0.017 - s * 0.5).cos() * 9.0
        + (x * 0.043 + z * 0.031 + s * 1.3).sin() * 4.0
        + (z * 0.067 - x * 0.012 + s).sin() * 1.6
        + ((x - z) * 0.009 + s * 0.2).cos() * 6.0
}

/// The ground's height at (x, z): hills, flat in the middle where he
/// starts and where things stand, mountains round the edge.
pub fn height(seed: u32, x: f32, z: f32) -> f32 {
    let c = SIZE / 2.0;
    let (dx, dz) = (x - c, z - c);
    let mid = (dx * dx + dz * dz).sqrt();
    let flat = smooth(((mid - 40.0) / 50.0).clamp(0.0, 1.0));
    let edge = (x.min(z).min(SIZE - x).min(SIZE - z) / 40.0).clamp(0.0, 1.0);
    let wall = (1.0 - smooth(edge)) * 70.0;
    noise(seed, x, z) * flat + wall
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// A loop at `start` going along `forward`, coming out `shift` metres to
/// its right; `width` wide.
pub fn loop_shape(start: Vec3, forward: Vec3, radius: f32, width: f32, shift: f32) -> (Shape, Shape, Loop) {
    let side = forward.cross(Vec3::Y).normalize();
    let mut top = Shape::default();
    let mut under = Shape::default();
    let n = 64;
    let row = |t: f32| {
        let p = start
            + forward * (t.sin() * radius)
            + Vec3::Y * (radius - t.cos() * radius)
            + side * (shift * t / std::f32::consts::TAU);
        let inward = (forward * -t.sin() + Vec3::Y * t.cos()).normalize();
        (p, inward)
    };
    for k in 0..=n {
        let t = k as f32 / n as f32 * std::f32::consts::TAU;
        let (p, inward) = row(t);
        let v = t * radius / 4.0;
        top.vertex(p - side * width / 2.0, inward, vec2(0.0, v), WHITE);
        top.vertex(p + side * width / 2.0, inward, vec2(width / 4.0, v), WHITE);
        under.vertex(p - side * width / 2.0 - inward * 0.6, -inward, vec2(0.0, v), WHITE);
        under.vertex(p + side * width / 2.0 - inward * 0.6, -inward, vec2(width / 4.0, v), WHITE);
    }
    for k in 0..n as u32 {
        let (a, b) = (k * 2, k * 2 + 2);
        top.tri(a, b + 1, b);
        top.tri(a, a + 1, b + 1);
        under.tri(a, b, b + 1);
        under.tri(a, b + 1, a + 1);
    }
    let l = Loop { start, forward, side, radius, width, shift };
    (top, under, l)
}

/// A ramp (a wedge) at `at` going up along `dir`.
fn ramp(at: Vec3, dir: Vec3, len: f32, rise: f32, width: f32) -> Shape {
    let side = dir.cross(Vec3::Y).normalize() * width / 2.0;
    let mut s = Shape::default();
    let a = at - side;
    let b = at + side;
    let c = at + dir * len + Vec3::Y * rise + side;
    let d = at + dir * len + Vec3::Y * rise - side;
    let e = at + dir * len + side - Vec3::Y * 0.5;
    let f = at + dir * len - side - Vec3::Y * 0.5;
    let uv = [vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(1.0, len / 4.0), vec2(0.0, len / 4.0)];
    s.quad([a, d, c, b], uv, WHITE);
    s.quad([d, f, e, c], uv, WHITE);
    s.quad([a - Vec3::Y * 0.5, f, d, a], uv, WHITE);
    s.quad([b, c, e, b - Vec3::Y * 0.5], uv, WHITE);
    s
}

/// A palm-ish tree: a trunk and a crown of leaves.
fn tree(at: Vec3, h: f32) -> (Shape, Shape) {
    let trunk = cylinder(at, -0.5, h, 0.45, 0.3, 8, Color::new(0.55, 0.36, 0.2, 1.0));
    let mut leaves = Shape::default();
    for k in 0..7 {
        let a = k as f32 / 7.0 * std::f32::consts::TAU;
        let dir = vec3(a.cos(), -0.35, a.sin());
        let leaf = sphere(Vec3::ZERO, vec3(2.0, 0.15, 0.6), 4, 8, Color::new(0.2, 0.62, 0.22, 1.0));
        let q = Quat::from_rotation_arc(Vec3::X, dir.normalize());
        leaves.append(&leaf, Mat4::from_rotation_translation(q, at + Vec3::Y * h + dir * 1.6));
    }
    leaves.append(&sphere(at + Vec3::Y * h, Vec3::splat(0.6), 5, 8, Color::new(0.25, 0.55, 0.2, 1.0)), Mat4::IDENTITY);
    (trunk, leaves)
}

impl Maker for Zone {
    fn endless(&self) -> bool {
        false
    }

    fn start(&self) -> (Vec3, Vec3) {
        let c = SIZE / 2.0;
        (vec3(c - 30.0, height(self.seed, c - 30.0, c), c), Vec3::X)
    }

    fn next(&mut self) -> Option<Stretch> {
        if self.made {
            return None;
        }
        self.made = true;
        let seed = self.seed;
        let mut st = Stretch::default();
        // The ground: one solid height map, drawn with grass on gentle
        // slopes and earth on steep ones.
        let mut g = Shape::default();
        let h = |x: f32, z: f32| height(seed, x, z);
        for i in 0..=CELLS {
            for j in 0..=CELLS {
                let (x, z) = (j as f32 * CELL, i as f32 * CELL);
                let y = h(x, z);
                let n = vec3(h(x - 1.0, z) - h(x + 1.0, z), 2.0, h(x, z - 1.0) - h(x, z + 1.0)).normalize();
                let steep = ((1.0 - n.y) * 4.0).clamp(0.0, 1.0);
                let tint = Color::new(1.0, 1.0 - steep * 0.25, 1.0 - steep * 0.6, 1.0);
                let tint = if y > 30.0 { Color::new(0.85, 0.8, 0.75, 1.0) } else { tint };
                g.vertex(vec3(x, y, z), n, vec2(x / 8.0, z / 8.0), tint);
            }
        }
        let row = (CELLS + 1) as u32;
        for i in 0..CELLS as u32 {
            for j in 0..CELLS as u32 {
                let a = i * row + j;
                g.tri(a, a + row + 1, a + 1);
                g.tri(a, a + row, a + row + 1);
            }
        }
        st.ground.push((g, Skin::Grass));
        let c = SIZE / 2.0;
        let base = |x: f32, z: f32| vec3(x, h(x, z), z);

        // A loop ahead of the start, then rings through it.
        let at = base(c + 10.0, c);
        let (top, under, l) = loop_shape(at, Vec3::X, 7.0, 6.0, 7.5);
        st.ground.push((top, Skin::Soil));
        st.decor.push((under, Skin::Soil));
        st.loops.push(l);
        let mut floor = cuboid(vec3(c - 40.0, at.y - 1.0, c - 4.0), vec3(c + 40.0, at.y + 0.02, c + 12.0), WHITE);
        for u in floor.uv.iter_mut() {
            *u *= 6.0;
        }
        st.ground.push((floor, Skin::Soil));
        for k in 0..6 {
            st.rings.push(at + vec3(-18.0 + k as f32 * 2.0, 1.0, 0.0));
        }
        st.panels.push(Panel { at: at + vec3(-22.0, 0.0, 0.0), dir: Vec3::X, up: Vec3::Y, half: 2.7 });
        // Ramps out in the hills, with rings in the air beyond them.
        for (k, (x, z, dir)) in
            [(c - 20.0, c + 50.0, vec3(0.0, 0.0, 1.0)), (c + 40.0, c - 50.0, vec3(1.0, 0.0, -1.0).normalize())]
                .into_iter()
                .enumerate()
        {
            let at = base(x, z);
            st.ground.push((ramp(at, dir, 10.0, 3.5 + k as f32, 6.0), Skin::Soil));
            st.panels.push(Panel { at: at - dir * 6.0 + Vec3::Y * 0.05, dir, up: Vec3::Y, half: 2.7 });
            for r in 0..6 {
                let t = r as f32 * 2.5;
                st.rings.push(at + dir * (13.0 + t) + Vec3::Y * (5.0 + 2.5 - (t - 6.0).powi(2) * 0.07));
            }
        }
        // Platforms on pillars, a spring at the foot of each.
        for (k, (x, z)) in [(c - 30.0, c - 35.0), (c + 60.0, c + 30.0), (c - 70.0, c + 10.0)].into_iter().enumerate() {
            let ground = h(x, z);
            let top = ground + 8.0 + k as f32 * 3.0;
            st.ground.push((cuboid(vec3(x - 5.0, top - 1.0, z - 5.0), vec3(x + 5.0, top, z + 5.0), WHITE), Skin::Soil));
            st.decor.push((
                cylinder(vec3(x, 0.0, z), ground - 1.0, top - 1.0, 1.2, 1.0, 10, Color::new(0.7, 0.5, 0.3, 1.0)),
                Skin::Plain,
            ));
            st.springs.push(Spring {
                at: vec3(x - 8.0, h(x - 8.0, z), z),
                up: Vec3::Y,
                power: 13.0 + k as f32,
                red: true,
                squash: 0.0,
            });
            for r in 0..3 {
                for q in 0..3 {
                    st.rings.push(vec3(x - 2.0 + r as f32 * 2.0, top + 1.0, z - 2.0 + q as f32 * 2.0));
                }
            }
        }
        // Lines of rings about the hills, and yellow springs.
        let mut rng = seed.wrapping_mul(2654435761).max(1);
        let mut next = move || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            (rng % 10_000) as f32 / 10_000.0
        };
        for _ in 0..22 {
            let (x, z) = (60.0 + next() * (SIZE - 120.0), 60.0 + next() * (SIZE - 120.0));
            let a = next() * std::f32::consts::TAU;
            let d = vec3(a.cos(), 0.0, a.sin());
            for k in 0..5 {
                let p = vec3(x, 0.0, z) + d * (k as f32 * 2.2);
                st.rings.push(vec3(p.x, h(p.x, p.z) + 1.0, p.z));
            }
        }
        for _ in 0..6 {
            let (x, z) = (60.0 + next() * (SIZE - 120.0), 60.0 + next() * (SIZE - 120.0));
            st.springs.push(Spring { at: vec3(x, h(x, z), z), up: Vec3::Y, power: 10.0, red: false, squash: 0.0 });
        }
        // Badniks.
        for k in 0..14 {
            let (x, z) = (50.0 + next() * (SIZE - 100.0), 50.0 + next() * (SIZE - 100.0));
            if (vec2(x, z) - vec2(c - 30.0, c)).length() < 25.0 {
                continue;
            }
            let a = next() * std::f32::consts::TAU;
            let at = vec3(x, h(x, z), z);
            let kind = if k % 3 == 2 { BadnikKind::Flyer } else { BadnikKind::Crawler };
            let at = if kind == BadnikKind::Flyer { at + Vec3::Y * 2.5 } else { at };
            st.badniks.push(Badnik { kind, at, home: at, dir: vec3(a.cos(), 0.0, a.sin()), t: next() * 600.0 });
        }
        // Trees (their trunks are solid).
        for _ in 0..40 {
            let (x, z) = (40.0 + next() * (SIZE - 80.0), 40.0 + next() * (SIZE - 80.0));
            if (vec2(x, z) - vec2(c, c)).length() < 45.0 {
                continue;
            }
            let (trunk, leaves) = tree(base(x, z), 5.0 + next() * 3.0);
            st.ground.push((trunk, Skin::Plain));
            st.decor.push((leaves, Skin::Plain));
        }
        // Things to knock over: a stack of crates and a heap of barrels.
        for y in 0..4 {
            for x in 0..(4 - y) {
                let p = vec3(
                    c - 10.0 + x as f32 * 1.05 + y as f32 * 0.5,
                    h(c - 10.0, c - 12.0) + 0.5 + y as f32 * 1.0,
                    c - 12.0,
                );
                st.things.push((Solid::Box(Vec3::splat(0.5)), p));
            }
        }
        for k in 0..6 {
            let p = vec3(c - 5.0 + (k % 3) as f32 * 1.0, h(c - 5.0, c + 14.0) + 0.6 + (k / 3) as f32 * 1.1, c + 14.0);
            st.things.push((Solid::Cylinder(0.5, 0.4), p));
        }
        st.things.push((Solid::Ball(1.2), vec3(c - 20.0, h(c - 20.0, c + 6.0) + 3.0, c + 6.0)));
        Some(st)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_zone_is_flat_where_he_starts_and_walled_in() {
        let mut z = Zone::new(7);
        let (at, _) = z.start();
        let c = SIZE / 2.0;
        assert!((height(7, c, c) - height(7, c - 30.0, c)).abs() < 0.5, "flat in the middle");
        assert!(height(7, 2.0, c) > 50.0 && height(7, c, SIZE - 2.0) > 50.0, "mountains round the edge");
        let st = z.next().unwrap();
        assert!(z.next().is_none(), "made once");
        assert!(st.rings.len() > 60 && !st.loops.is_empty() && !st.badniks.is_empty() && !st.things.is_empty());
        // He starts on the ground, clear of everything else.
        assert!((at.y - height(7, at.x, at.z)).abs() < 1e-3);
    }

    #[test]
    fn the_loop_goes_round_and_comes_out_to_the_side() {
        let (top, _, l) = loop_shape(Vec3::ZERO, Vec3::X, 6.0, 4.0, 6.0);
        let first = (top.pos[0] + top.pos[1]) / 2.0;
        let last = (top.pos[top.pos.len() - 2] + top.pos[top.pos.len() - 1]) / 2.0;
        assert!(first.length() < 1e-4);
        assert!((last - l.side * 6.0).length() < 1e-3, "out to the side: {last:?}");
        let highest = top.pos.iter().map(|p| p.y).fold(0.0, f32::max);
        assert!((highest - 12.0).abs() < 0.1);
        // The surface faces in.
        let mid = top.pos.len() / 4;
        assert!(top.normal[mid].x < -0.9, "the far side faces back: {:?}", top.normal[mid]);
    }
}
