//! The characters, badniks and things built from simple shapes: what is
//! drawn when no model files are there (and in the docs' pictures). A
//! character is a few parts (body and head, arms, legs) swung about their
//! joints for running; curled up, a spiky ball.
//!
//! Model space: x forward, y up, z to the right, feet at y = 0, a character
//! about 1.5 m tall.

use super::gfx::{cuboid, cylinder, sphere, torus, Shape};
use crate::sonic::Character;
use macroquad::prelude::*;

/// What a character is doing, for the drawing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    /// Through the run cycle (radians, goes on growing).
    pub phase: f32,
    /// Speed (guide px a frame).
    pub speed: f32,
    pub ball: bool,
    /// The ball's spin (radians).
    pub spin: f32,
    pub air: bool,
    pub flying: bool,
    pub gliding: bool,
    pub hammer: Option<u32>,
    pub skidding: bool,
    /// The model pack's animation (SRB2's names) and how far into it.
    pub anim: &'static str,
    pub time: f32,
}

/// A character's parts, ready to draw.
pub struct Figure {
    body: Vec<Mesh>,
    arm: [Vec<Mesh>; 2],
    leg: [Vec<Mesh>; 2],
    ball: Vec<Mesh>,
    extra: Vec<Mesh>,
    shoulder: [Vec3; 2],
    hip: [Vec3; 2],
}

struct Look {
    main: Color,
    skin: Color,
    shoe: Color,
    eye: Color,
    glove: Color,
}

fn look(c: Character) -> Look {
    let skin = Color::new(0.98, 0.78, 0.6, 1.0);
    match c {
        Character::Sonic => Look {
            main: Color::new(0.1, 0.32, 0.92, 1.0),
            skin,
            shoe: Color::new(0.9, 0.12, 0.12, 1.0),
            eye: Color::new(0.1, 0.55, 0.2, 1.0),
            glove: WHITE,
        },
        Character::Tails => Look {
            main: Color::new(0.98, 0.62, 0.15, 1.0),
            skin: Color::new(1.0, 0.96, 0.9, 1.0),
            shoe: Color::new(0.9, 0.15, 0.15, 1.0),
            eye: Color::new(0.15, 0.35, 0.8, 1.0),
            glove: WHITE,
        },
        Character::Knuckles => Look {
            main: Color::new(0.86, 0.14, 0.16, 1.0),
            skin,
            shoe: Color::new(0.2, 0.7, 0.3, 1.0),
            eye: Color::new(0.6, 0.2, 0.6, 1.0),
            glove: WHITE,
        },
        Character::Amy => Look {
            main: Color::new(1.0, 0.5, 0.7, 1.0),
            skin,
            shoe: Color::new(0.9, 0.15, 0.2, 1.0),
            eye: Color::new(0.2, 0.6, 0.3, 1.0),
            glove: WHITE,
        },
        Character::Exe => Look {
            main: Color::new(0.08, 0.2, 0.62, 1.0),
            skin: Color::new(0.9, 0.72, 0.58, 1.0),
            shoe: Color::new(0.55, 0.05, 0.05, 1.0),
            eye: Color::new(0.9, 0.05, 0.05, 1.0),
            glove: Color::new(0.85, 0.85, 0.85, 1.0),
        },
    }
}

/// `s` turned so its y axis points along `dir`, then put at `at`.
fn along(s: &Shape, dir: Vec3, at: Vec3) -> Shape {
    let mut out = Shape::default();
    let q = Quat::from_rotation_arc(Vec3::Y, dir.normalize());
    out.append(s, Mat4::from_rotation_translation(q, at));
    out
}

/// A cone from `at` out along `dir`, `len` long, `r` wide at the base.
fn spike(at: Vec3, dir: Vec3, len: f32, r: f32, c: Color) -> Shape {
    along(&cylinder(Vec3::ZERO, 0.0, len, r, 0.0, 7, c), dir, at)
}

fn eyes(s: &mut Shape, l: &Look, centre: Vec3, black: bool) {
    for side in [-1.0f32, 1.0] {
        let white = if black { Color::new(0.03, 0.03, 0.05, 1.0) } else { WHITE };
        s.append(&sphere(centre + vec3(0.0, 0.0, side * 0.09), vec3(0.06, 0.12, 0.085), 6, 8, white), Mat4::IDENTITY);
        s.append(
            &sphere(centre + vec3(0.05, 0.0, side * 0.075), vec3(0.025, 0.05, 0.035), 4, 6, l.eye),
            Mat4::IDENTITY,
        );
    }
}

impl Figure {
    pub fn new(c: Character) -> Figure {
        let l = look(c);
        let mut body = Shape::default();
        let head = vec3(0.02, 1.12, 0.0);
        // Body and belly.
        if matches!(c, Character::Amy) {
            body.append(
                &cylinder(vec3(0.0, 0.0, 0.0), 0.4, 0.8, 0.36, 0.18, 12, Color::new(0.85, 0.12, 0.2, 1.0)),
                Mat4::IDENTITY,
            );
        } else {
            body.append(&sphere(vec3(0.0, 0.62, 0.0), vec3(0.25, 0.3, 0.25), 8, 10, l.main), Mat4::IDENTITY);
            body.append(&sphere(vec3(0.1, 0.58, 0.0), vec3(0.18, 0.22, 0.19), 6, 8, l.skin), Mat4::IDENTITY);
        }
        // Head, muzzle, nose, eyes.
        body.append(&sphere(head, Vec3::splat(0.34), 10, 12, l.main), Mat4::IDENTITY);
        body.append(&sphere(head + vec3(0.24, -0.08, 0.0), vec3(0.14, 0.11, 0.19), 6, 8, l.skin), Mat4::IDENTITY);
        body.append(&sphere(head + vec3(0.38, -0.04, 0.0), Vec3::splat(0.045), 4, 6, BLACK), Mat4::IDENTITY);
        eyes(&mut body, &l, head + vec3(0.26, 0.08, 0.0), matches!(c, Character::Exe));
        // Ears.
        for side in [-1.0f32, 1.0] {
            body.append(
                &spike(head + vec3(-0.04, 0.24, side * 0.16), vec3(-0.2, 1.0, side * 0.3), 0.2, 0.08, l.main),
                Mat4::IDENTITY,
            );
        }
        // Spikes, quills, dreadlocks or hair.
        let back = head + vec3(-0.18, 0.0, 0.0);
        match c {
            Character::Sonic | Character::Exe => {
                for (dy, dz) in [(0.18, 0.0), (0.0, 0.0), (-0.18, 0.0), (0.06, 0.16), (0.06, -0.16)] {
                    let dir = vec3(-1.0, dy * 2.0 - 0.3, dz * 2.0);
                    body.append(&spike(back + vec3(0.0, dy, dz), dir, 0.55, 0.15, l.main), Mat4::IDENTITY);
                }
                body.append(&spike(vec3(-0.18, 0.72, 0.0), vec3(-1.0, -0.2, 0.0), 0.25, 0.1, l.main), Mat4::IDENTITY);
            }
            Character::Tails => {
                for dz in [-0.1f32, 0.0, 0.1] {
                    body.append(
                        &spike(head + vec3(0.15, 0.3, dz), vec3(0.6, 0.6, dz * 2.0), 0.18, 0.07, l.main),
                        Mat4::IDENTITY,
                    );
                }
            }
            Character::Knuckles => {
                for (dy, dz) in [(0.1, -0.12), (0.1, 0.12), (-0.05, 0.0), (0.2, 0.0)] {
                    let dir = vec3(-0.5, -1.0, dz * 2.0);
                    body.append(&spike(back + vec3(0.0, dy, dz), dir, 0.6, 0.12, l.main), Mat4::IDENTITY);
                }
                body.append(&sphere(vec3(0.16, 0.74, 0.0), vec3(0.06, 0.1, 0.18), 5, 8, WHITE), Mat4::IDENTITY);
            }
            Character::Amy => {
                for dz in [-0.2f32, 0.0, 0.2] {
                    body.append(
                        &spike(back + vec3(0.0, -0.1, dz), vec3(-0.4, -1.0, dz), 0.3, 0.11, l.main),
                        Mat4::IDENTITY,
                    );
                }
                body.append(
                    &torus(0.25, 0.04, 16, 6, Color::new(0.9, 0.1, 0.2, 1.0)),
                    Mat4::from_translation(head + vec3(0.0, 0.22, 0.0)),
                );
                body.append(&sphere(vec3(0.0, 0.62, 0.0), vec3(0.17, 0.2, 0.17), 6, 8, l.main), Mat4::IDENTITY);
            }
        }
        let mut extra = Shape::default();
        if matches!(c, Character::Tails) {
            for side in [-1.0f32, 1.0] {
                extra.append(
                    &spike(vec3(-0.2, 0.45, side * 0.1), vec3(-1.0, 0.5, side * 0.6), 0.6, 0.14, l.main),
                    Mat4::IDENTITY,
                );
            }
        }
        // Arms (pivot at the shoulder, hanging down) and legs (at the hip).
        let shoulder = [vec3(0.0, 0.8, -0.24), vec3(0.0, 0.8, 0.24)];
        let hip = [vec3(0.0, 0.4, -0.12), vec3(0.0, 0.4, 0.12)];
        let fist = if matches!(c, Character::Knuckles) { 0.13 } else { 0.09 };
        let arm = shoulder.map(|s| {
            let mut a = cylinder(s, -0.38, 0.0, 0.045, 0.05, 6, l.skin);
            if matches!(c, Character::Sonic | Character::Exe | Character::Knuckles) {
                a = cylinder(
                    s,
                    -0.38,
                    0.0,
                    0.045,
                    0.05,
                    6,
                    if matches!(c, Character::Knuckles) { l.main } else { l.skin },
                );
            }
            a.append(&sphere(s + vec3(0.02, -0.42, 0.0), Vec3::splat(fist), 6, 8, l.glove), Mat4::IDENTITY);
            a.meshes(None)
        });
        let leg = hip.map(|h| {
            let mut g = cylinder(h, -0.32, 0.0, 0.05, 0.055, 6, l.main);
            g.append(&sphere(h + vec3(0.09, -0.34, 0.0), vec3(0.2, 0.08, 0.1), 6, 8, l.shoe), Mat4::IDENTITY);
            g.append(&cylinder(h + vec3(0.0, -0.33, 0.0), 0.0, 0.06, 0.07, 0.07, 8, WHITE), Mat4::IDENTITY);
            g.meshes(None)
        });
        // Curled up: a ball with the spikes swept round.
        let mut ball = sphere(vec3(0.0, 0.0, 0.0), Vec3::splat(0.46), 10, 12, l.main);
        for k in 0..6 {
            let a = k as f32 / 6.0 * std::f32::consts::TAU;
            let out = vec3(a.cos(), a.sin(), 0.0);
            let tangent = vec3(-a.sin(), a.cos(), 0.0);
            ball.append(&spike(out * 0.36, out * 0.7 - tangent, 0.32, 0.14, l.main), Mat4::IDENTITY);
        }
        ball.append(&sphere(vec3(0.3, 0.1, 0.0), vec3(0.14, 0.12, 0.18), 5, 6, l.skin), Mat4::IDENTITY);
        Figure { body: body.meshes(None), arm, leg, ball: ball.meshes(None), extra: extra.meshes(None), shoulder, hip }
    }

    /// Draw at `place` (model space to world: feet at its origin).
    pub fn draw(&self, place: Mat4, pose: &Pose) {
        use super::gfx::draw_at;
        if pose.ball {
            let centre = place * Mat4::from_translation(vec3(0.0, 0.5, 0.0)) * Mat4::from_rotation_z(-pose.spin);
            draw_at(&self.ball, centre);
            return;
        }
        let run = (pose.speed / 6.0).clamp(0.0, 1.0);
        let lean = if pose.skidding { -0.35 } else { run * 0.3 };
        let base = place * Mat4::from_rotation_z(-lean);
        draw_at(&self.body, base);
        let wag = (pose.phase * 1.5).sin() * 0.25;
        let tails = if pose.flying { (pose.phase * 4.0).sin() * 0.6 } else { wag };
        draw_at(&self.extra, base * pivot(vec3(-0.2, 0.45, 0.0), Mat4::from_rotation_x(tails)));
        let swing = pose.phase.sin() * (0.25 + run * 0.85);
        for k in 0..2 {
            let side = if k == 0 { 1.0 } else { -1.0 };
            let (arm, leg) = if pose.gliding {
                (Mat4::from_rotation_z(2.9), Mat4::from_rotation_z(-0.3))
            } else if pose.air {
                (Mat4::from_rotation_z(0.6 + side * 0.3), Mat4::from_rotation_z(side * 0.4))
            } else if let Some(t) = pose.hammer {
                let a = 2.6 - (t as f32 / 16.0).min(1.0) * 3.4;
                (Mat4::from_rotation_z(a), Mat4::from_rotation_z(side * 0.2))
            } else {
                (Mat4::from_rotation_z(-swing * side), Mat4::from_rotation_z(swing * side))
            };
            draw_at(&self.arm[k], base * pivot(self.shoulder[k], arm));
            draw_at(&self.leg[k], base * pivot(self.hip[k], leg));
        }
    }
}

/// `turn` applied about `at`.
fn pivot(at: Vec3, turn: Mat4) -> Mat4 {
    Mat4::from_translation(at) * turn * Mat4::from_translation(-at)
}

/// Things in the worlds, built once.
pub struct Props {
    pub ring: Vec<Mesh>,
    pub spring: Vec<Mesh>,
    pub spring_red: Vec<Mesh>,
    pub panel: Vec<Mesh>,
    pub crawler: Vec<Mesh>,
    pub flyer: Vec<Mesh>,
    pub shadow: Vec<Mesh>,
    /// What a badnik bursts into, a blast's fire, and a cloud.
    pub puff: Vec<Mesh>,
    pub flame: Vec<Mesh>,
    pub cloud: Vec<Mesh>,
    pub crate_: Vec<Mesh>,
    pub ball: Vec<Mesh>,
    pub barrel: Vec<Mesh>,
}

impl Default for Props {
    fn default() -> Self {
        Self::new()
    }
}

impl Props {
    pub fn new() -> Props {
        let gold = Color::new(1.0, 0.82, 0.15, 1.0);
        let ring = {
            let mut t = torus(0.36, 0.07, 18, 6, gold);
            let mut s = Shape::default();
            s.append(&t, Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2));
            t = s;
            t.meshes(None)
        };
        let spring_of = |top: Color| {
            let mut s = cylinder(Vec3::ZERO, 0.0, 0.2, 0.55, 0.55, 14, Color::new(0.35, 0.36, 0.4, 1.0));
            for k in 0..3 {
                s.append(
                    &torus(0.38, 0.05, 14, 5, Color::new(0.75, 0.75, 0.8, 1.0)),
                    Mat4::from_translation(vec3(0.0, 0.3 + k as f32 * 0.13, 0.0)),
                );
            }
            s.append(&cylinder(Vec3::ZERO, 0.62, 0.8, 0.6, 0.55, 14, top), Mat4::IDENTITY);
            s.meshes(None)
        };
        let panel = {
            let mut s = cuboid(vec3(-1.0, 0.0, -0.8), vec3(1.0, 0.06, 0.8), Color::new(0.3, 0.3, 0.35, 1.0));
            for k in 0..3 {
                let x = -0.6 + k as f32 * 0.5;
                let y = 0.07;
                let c = Color::new(1.0, 0.85, 0.1, 1.0);
                let a = vec3(x + 0.35, y, 0.0);
                let b = vec3(x, y, -0.55);
                let d = vec3(x, y, 0.55);
                let i0 = s.vertex(a, Vec3::Y, Vec2::ZERO, c);
                let i1 = s.vertex(d, Vec3::Y, Vec2::ZERO, c);
                let i2 = s.vertex(b, Vec3::Y, Vec2::ZERO, c);
                let i3 = s.vertex(vec3(x + 0.15, y, 0.0), Vec3::Y, Vec2::ZERO, Color::new(0.3, 0.3, 0.35, 1.0));
                s.tri(i0, i2, i1);
                let _ = i3;
            }
            s.meshes(None)
        };
        let crawler = {
            let red = Color::new(0.85, 0.12, 0.1, 1.0);
            let mut s = sphere(vec3(0.0, 0.55, 0.0), vec3(0.7, 0.42, 0.55), 8, 12, red);
            s.append(
                &sphere(vec3(0.55, 0.5, 0.0), vec3(0.3, 0.28, 0.32), 6, 8, Color::new(0.15, 0.2, 0.55, 1.0)),
                Mat4::IDENTITY,
            );
            for side in [-1.0f32, 1.0] {
                s.append(&sphere(vec3(0.75, 0.6, side * 0.12), Vec3::splat(0.08), 4, 6, WHITE), Mat4::IDENTITY);
                for k in 0..2 {
                    let x = -0.3 + k as f32 * 0.5;
                    s.append(
                        &cylinder(vec3(x, 0.0, side * 0.45), 0.0, 0.25, 0.14, 0.14, 8, Color::new(0.2, 0.2, 0.22, 1.0)),
                        Mat4::IDENTITY,
                    );
                }
            }
            s.meshes(None)
        };
        let flyer = {
            let mut s = sphere(vec3(0.0, 0.0, 0.0), vec3(0.6, 0.3, 0.3), 8, 10, Color::new(0.15, 0.2, 0.6, 1.0));
            s.append(
                &sphere(vec3(-0.3, 0.0, 0.0), vec3(0.3, 0.25, 0.25), 6, 8, Color::new(1.0, 0.75, 0.1, 1.0)),
                Mat4::IDENTITY,
            );
            for side in [-1.0f32, 1.0] {
                s.append(
                    &sphere(vec3(0.0, 0.3, side * 0.35), vec3(0.3, 0.04, 0.35), 4, 8, Color::new(0.85, 0.9, 1.0, 0.8)),
                    Mat4::IDENTITY,
                );
            }
            s.append(
                &spike(vec3(-0.5, 0.0, 0.0), vec3(-0.3, -1.0, 0.0), 0.4, 0.08, Color::new(0.8, 0.8, 0.85, 1.0)),
                Mat4::IDENTITY,
            );
            s.meshes(None)
        };
        let shadow = {
            let mut s = Shape::default();
            let c = Color::new(0.0, 0.0, 0.0, 0.5);
            let mid = s.vertex(Vec3::ZERO, Vec3::Y, Vec2::ZERO, c);
            let n = 16;
            for k in 0..=n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                s.vertex(vec3(a.cos() * 0.5, 0.0, a.sin() * 0.5), Vec3::Y, Vec2::ZERO, c);
            }
            for k in 0..n {
                s.tri(mid, mid + 2 + k, mid + 1 + k);
            }
            s.meshes(None)
        };
        let puff = sphere(Vec3::ZERO, Vec3::splat(0.5), 6, 8, Color::new(1.0, 0.98, 0.9, 1.0)).meshes(None);
        let flame = {
            let mut s = sphere(Vec3::ZERO, Vec3::splat(0.5), 6, 8, Color::new(1.0, 0.55, 0.1, 1.0));
            s.append(
                &sphere(vec3(0.0, 0.1, 0.0), Vec3::splat(0.36), 5, 8, Color::new(1.0, 0.9, 0.35, 1.0)),
                Mat4::IDENTITY,
            );
            s.meshes(None)
        };
        let cloud = {
            let white = Color::new(1.0, 1.0, 1.0, 1.0);
            let mut s = sphere(Vec3::ZERO, vec3(22.0, 5.0, 12.0), 6, 10, white);
            for (at, size) in [
                (vec3(-16.0, -1.0, 4.0), vec3(14.0, 4.0, 9.0)),
                (vec3(15.0, -1.5, -3.0), vec3(16.0, 3.5, 10.0)),
                (vec3(2.0, 3.0, 0.0), vec3(11.0, 4.5, 8.0)),
            ] {
                s.append(&sphere(at, size, 5, 9, white), Mat4::IDENTITY);
            }
            s.meshes(None)
        };
        let wood = Color::new(0.7, 0.48, 0.25, 1.0);
        let crate_ = {
            let mut s = cuboid(Vec3::splat(-0.5), Vec3::splat(0.5), wood);
            for (a, b) in [
                (vec3(-0.52, -0.52, -0.52), vec3(0.52, -0.38, 0.52)),
                (vec3(-0.52, 0.38, -0.52), vec3(0.52, 0.52, 0.52)),
            ] {
                s.append(&cuboid(a, b, Color::new(0.5, 0.32, 0.15, 1.0)), Mat4::IDENTITY);
            }
            s.meshes(None)
        };
        let ball = sphere(Vec3::ZERO, Vec3::splat(0.5), 10, 14, Color::new(0.95, 0.3, 0.3, 1.0)).meshes(None);
        let barrel = {
            let mut s = cylinder(Vec3::ZERO, -0.5, 0.5, 0.4, 0.4, 14, Color::new(0.2, 0.55, 0.3, 1.0));
            for y in [-0.3f32, 0.3] {
                s.append(
                    &torus(0.41, 0.03, 14, 4, Color::new(0.3, 0.3, 0.32, 1.0)),
                    Mat4::from_translation(vec3(0.0, y, 0.0)),
                );
            }
            s.meshes(None)
        };
        Props {
            ring,
            spring: spring_of(Color::new(1.0, 0.85, 0.1, 1.0)),
            spring_red: spring_of(Color::new(0.95, 0.15, 0.12, 1.0)),
            panel,
            crawler,
            flyer,
            shadow,
            puff,
            flame,
            cloud,
            crate_,
            ball,
            barrel,
        }
    }
}
