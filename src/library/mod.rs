//! Built-in scenes: examples that show what the sandbox can do, and
//! challenges (get the ball into the goal by drawing).
//!
//! Scenes are described in code, in "design" pixels of a 1100 × 720 window,
//! and turned into a regular [`SceneFile`] for the current window size: the
//! layout is centred horizontally and anchored to the floor.

pub mod challenges;
pub mod examples;

use crate::config::PPM;
use crate::drawing;
use crate::physics::borders::BorderMode;
use crate::physics::links::LinkKind;
use crate::physics::object::Material;
use crate::physics::zones::{self, Zone, ZoneKind};
use crate::scene::{SceneFile, SceneLink, SceneObject, SceneSource, SceneWater};
use crate::shapes::Shape;

const DESIGN_W: f32 = 1100.0;
const DESIGN_H: f32 = 720.0;

/// Colour reserved for the challenge ball (it identifies the ball).
pub const BALL_RGB: [u8; 3] = [255, 196, 36];

/// Some pleasant colours for scenery.
pub const STONE: [u8; 3] = [120, 116, 150];
pub const WOOD: [u8; 3] = [196, 140, 90];
pub const LEAF: [u8; 3] = [98, 204, 120];
pub const SKY: [u8; 3] = [84, 140, 245];
pub const CORAL: [u8; 3] = [239, 86, 102];
pub const SNOW: [u8; 3] = [228, 228, 236];
pub const VIOLET: [u8; 3] = [150, 110, 250];
pub const GLASS: [u8; 3] = [150, 220, 245];

pub struct Builder {
    /// Offset from design to screen pixels.
    dx: f32,
    dy: f32,
    sh: f32,
    pub scene: SceneFile,
}

impl Builder {
    pub fn new(sw: f32, sh: f32) -> Self {
        Builder {
            dx: (sw - DESIGN_W) / 2.0,
            dy: sh - DESIGN_H,
            sh,
            scene: SceneFile {
                version: 3,
                gravity: -9.8,
                border: BorderMode::Walls,
                objects: vec![],
                links: vec![],
                water: None,
                zones: vec![],
            },
        }
    }

    /// Design pixels → physics metres.
    fn phys(&self, x: f32, y: f32) -> [f32; 2] {
        [(x + self.dx) / PPM, (self.sh - (y + self.dy)) / PPM]
    }

    fn push(&mut self, source: SceneSource, centre: (f32, f32), size: (f32, f32), pinned: bool, m: Material) -> usize {
        let [x, y] = self.phys(centre.0, centre.1);
        self.scene.objects.push(SceneObject {
            source,
            x,
            y,
            angle: 0.0,
            vx: 0.0,
            vy: 0.0,
            angvel: 0.0,
            w: size.0,
            h: size.1,
            pinned,
            material: m,
            mass: None,
        });
        self.scene.objects.len() - 1
    }

    /// A spawner shape of `size` px centred on (x, y).
    pub fn shape(&mut self, shape: Shape, rgb: [u8; 3], x: f32, y: f32, size: f32) -> usize {
        self.shape_with(shape, rgb, x, y, size, false, Material::DEFAULT)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_with(
        &mut self,
        shape: Shape,
        rgb: [u8; 3],
        x: f32,
        y: f32,
        size: f32,
        pinned: bool,
        m: Material,
    ) -> usize {
        self.push(SceneSource::Shape { shape, rgb }, (x, y), (size, size * shape.aspect()), pinned, m)
    }

    /// A drawn stroke through `points` (design px): a plank, or a filled
    /// shape when the stroke closes.
    pub fn stroke(&mut self, points: &[(f32, f32)], thickness: f32, rgb: [u8; 3], pinned: bool, m: Material) -> usize {
        let placed = drawing::from_stroke(points, thickness, rgb).expect("strokes have points");
        self.push(SceneSource::Drawing(placed.drawing), placed.center, placed.size, pinned, m)
    }

    /// A pinned plank along `points`.
    pub fn wall(&mut self, points: &[(f32, f32)], rgb: [u8; 3]) -> usize {
        self.stroke(&dense(points), 14.0, rgb, true, Material::DEFAULT)
    }

    /// Link object `a` (at design point `pa`) to object `b` or the background (at `pb`).
    pub fn link(&mut self, kind: LinkKind, a: usize, pa: (f32, f32), b: Option<usize>, pb: (f32, f32), speed: f32) {
        let local = |i: Option<usize>, p: (f32, f32)| {
            let [x, y] = self.phys(p.0, p.1);
            match i {
                Some(i) => [x - self.scene.objects[i].x, y - self.scene.objects[i].y],
                None => [x, y],
            }
        };
        let (la, lb) = (local(Some(a), pa), local(b, pb));
        let [ax, ay] = self.phys(pa.0, pa.1);
        let [bx, by] = self.phys(pb.0, pb.1);
        let length = ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt().max(0.05);
        self.scene.links.push(SceneLink { kind, a, b, la, lb, length, speed });
    }

    /// A zone over the design rectangle `(x0, y0, x1, y1)`.
    pub fn zone(&mut self, kind: ZoneKind, rect: (f32, f32, f32, f32), angle_deg: f32, strength: f32) -> usize {
        let (x0, y0, x1, y1) = rect;
        let (a, b) = (self.phys(x0, y1), self.phys(x1, y0));
        let zone = Zone { kind, min: a, max: b, angle: angle_deg.to_radians(), strength, pair: None };
        zones::add(&mut self.scene.zones, zone);
        self.scene.zones.len() - 1
    }

    pub fn water(&mut self, level: f32, density: f32) {
        self.scene.water = Some(SceneWater { level, density });
    }
}

/// Resample a polyline so the drawing code gets evenly spaced points.
fn dense(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut out = vec![points[0]];
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        let n = (len / 6.0).ceil().max(1.0) as usize;
        for k in 1..=n {
            let t = k as f32 / n as f32;
            out.push((a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t));
        }
    }
    out
}

/// An example scene.
pub struct Example {
    pub name: &'static str,
    pub about: &'static str,
    pub build: fn(&mut Builder),
}

/// A challenge: bring the ball into the goal zone by drawing.
pub struct Challenge {
    pub id: &'static str,
    pub name: &'static str,
    pub goal: &'static str,
    /// Length of line (px) the player may draw.
    pub ink: f32,
    pub build: fn(&mut Builder),
}

impl Challenge {
    /// Build the level; the ball must be a pinned circle of [`BALL_RGB`].
    pub fn scene(&self, sw: f32, sh: f32) -> SceneFile {
        let mut b = Builder::new(sw, sh);
        (self.build)(&mut b);
        b.scene
    }
}

impl Example {
    pub fn scene(&self, sw: f32, sh: f32) -> SceneFile {
        let mut b = Builder::new(sw, sh);
        (self.build)(&mut b);
        b.scene
    }
}

/// The challenge ball: a pinned golden circle released by "Go".
pub fn ball(b: &mut Builder, x: f32, y: f32) -> usize {
    let m = Material { bounce: 0.3, friction: 0.6, ..Material::DEFAULT };
    b.shape_with(Shape::Circle, BALL_RGB, x, y, 36.0, true, m)
}

/// A pinned cup around a goal zone spanning x0..x1 on a surface at `floor`.
pub fn goal_cup(b: &mut Builder, x0: f32, x1: f32, floor: f32, depth: f32) {
    b.wall(&[(x0, floor - depth), (x0, floor)], STONE);
    b.wall(&[(x1, floor - depth), (x1, floor)], STONE);
    b.zone(ZoneKind::Goal, (x0 + 8.0, floor - depth + 10.0, x1 - 8.0, floor), 0.0, 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_built_in_scene_builds_and_round_trips() {
        for e in examples::ALL {
            let sc = e.scene(1100.0, 720.0);
            assert!(!sc.objects.is_empty(), "{} is empty", e.name);
            let json = serde_json::to_string(&sc).unwrap();
            assert!(serde_json::from_str::<SceneFile>(&json).is_ok(), "{} round-trips", e.name);
        }
        for c in challenges::ALL {
            let sc = c.scene(1100.0, 720.0);
            let balls = sc
                .objects
                .iter()
                .filter(|o| matches!(o.source, SceneSource::Shape { rgb, .. } if rgb == BALL_RGB) && o.pinned)
                .count();
            assert_eq!(balls, 1, "{} has one pinned ball", c.name);
            assert_eq!(sc.zones.iter().filter(|z| z.kind == ZoneKind::Goal).count(), 1, "{} has one goal", c.name);
        }
    }

    #[test]
    fn design_coordinates_follow_the_window() {
        let b = Builder::new(1300.0, 820.0);
        // Centred horizontally, anchored to the bottom.
        let [x, y] = b.phys(550.0, 720.0);
        assert!((x * PPM - 650.0).abs() < 1e-3 && y.abs() < 1e-3);
    }
}
