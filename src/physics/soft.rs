//! Soft bodies. A **jelly** is a ring of small balls pulled back toward its
//! rest shape (shape matching: the rest shape is fitted to where the balls
//! are, rotation included, and each ball is sprung toward its place), so
//! it squashes, wobbles and rolls. A **cloth** is a grid of small balls tied
//! by ropes: it hangs from pins, drapes over things, blows in the wind and
//! tears when cut or pulled too hard. Both are drawn as a textured mesh
//! that follows the balls, so any image can become jelly or a flag.

use super::{to_screen, PhysWorld};
use crate::config::{DENSITY, PPM};
use macroquad::prelude::*;
use rapier2d::prelude::*;

/// Cloth balls do not collide with each other (folds would jitter).
const CLOTH_GROUP: Group = Group::GROUP_2;
/// Jelly stiffness and damping (as accelerations, 1/s² and 1/s).
const JELLY_K: f32 = 320.0;
const JELLY_C: f32 = 7.0;
/// Jelly pressure: outward push (as an acceleration) when squashed flat.
const JELLY_P: f32 = 260.0;
/// Springs along a jelly's skin keep its balls evenly spaced (1/s²).
const JELLY_SKIN_K: f32 = 600.0;
/// Target spacing of jelly balls and cloth knots (px).
const JELLY_SPACING: f32 = 12.0;
const CLOTH_SPACING: f32 = 18.0;
/// A cloth thread snaps when stretched this much.
const TEAR_STRETCH: f32 = 1.6;
/// Cloth is this much lighter than solid objects of the same area.
const CLOTH_DENSITY: f32 = 0.12;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SoftKind {
    Jelly,
    Cloth,
}

/// A cloth thread between two knots.
#[derive(Clone, Copy)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    /// Rest length (m).
    rest: f32,
    joint: Option<ImpulseJointHandle>,
}

/// A cloth triangle: its corners and the three threads it needs.
type Tri = ([usize; 3], [(usize, usize); 3]);

/// Everything needed to (re)build a soft body (undo, construction).
#[derive(Clone)]
pub struct SoftState {
    pub kind: SoftKind,
    pub name: String,
    texture: Option<Texture2D>,
    /// Per ball: position (x, y, m) and velocity (x, y, m/s).
    balls: Vec<[f32; 4]>,
    pinned: Vec<bool>,
    uv: Vec<Vec2>,
    /// Jelly: rest offsets from the centre (m, y up).
    rest: Vec<Vector<f32>>,
    /// Ball radius (m) and mass (kg).
    radius: f32,
    mass: f32,
    /// Cloth threads still whole: (a, b, rest length).
    edges: Vec<(usize, usize, f32)>,
    tris: Vec<Tri>,
    cols: usize,
}

pub struct Soft {
    pub kind: SoftKind,
    pub name: String,
    texture: Option<Texture2D>,
    pub balls: Vec<RigidBodyHandle>,
    pinned: Vec<bool>,
    uv: Vec<Vec2>,
    rest: Vec<Vector<f32>>,
    radius: f32,
    mass: f32,
    pub edges: Vec<Edge>,
    tris: Vec<Tri>,
    cols: usize,
    /// Cloth: seconds each knot has been burning (0: not alight, < 0: burnt).
    pub burn: Vec<f32>,
}

/// Resample a closed polygon (px) into `n` points evenly spaced along it.
fn resample(poly: &[Vec2], n: usize) -> Vec<Vec2> {
    let m = poly.len();
    let len = |i: usize| poly[i].distance(poly[(i + 1) % m]);
    let total: f32 = (0..m).map(len).sum();
    if m < 2 || total <= 0.0 {
        return poly.to_vec();
    }
    let step = total / n as f32;
    let mut out = Vec::with_capacity(n);
    let (mut seg, mut into) = (0, 0.0);
    for k in 0..n {
        let target = k as f32 * step;
        let mut walked: f32 = (0..seg).map(len).sum::<f32>() + into;
        while walked + (len(seg) - into) < target && seg < m - 1 {
            walked += len(seg) - into;
            seg += 1;
            into = 0.0;
        }
        into += target - walked;
        let t = (into / len(seg).max(1e-6)).clamp(0.0, 1.0);
        out.push(poly[seg].lerp(poly[(seg + 1) % m], t));
    }
    out
}

/// Area of a polygon (px²).
fn area(poly: &[Vec2]) -> f32 {
    let n = poly.len();
    (0..n).map(|i| poly[i].perp_dot(poly[(i + 1) % n])).sum::<f32>().abs() / 2.0
}

impl SoftState {
    /// A jelly with the outline `outline` (normalised to the sprite box,
    /// v up) of a `size` px sprite, centred on `centre` (m) and turned by
    /// `angle`, moving at `vel` (m/s).
    pub fn jelly(
        name: String,
        texture: Option<Texture2D>,
        outline: &[[f32; 2]],
        size: Vec2,
        centre: Vector<f32>,
        angle: f32,
        vel: Vector<f32>,
    ) -> Option<Self> {
        let poly: Vec<Vec2> = outline.iter().map(|&[u, v]| vec2(u * size.x, v * size.y)).collect();
        if poly.len() < 3 {
            return None;
        }
        let perimeter: f32 = (0..poly.len()).map(|i| poly[i].distance(poly[(i + 1) % poly.len()])).sum();
        let n = ((perimeter / JELLY_SPACING) as usize).clamp(12, 64);
        let pts = resample(&poly, n);
        let mid = pts.iter().copied().sum::<Vec2>() / n as f32;
        let spacing = perimeter / n as f32;
        let radius = (spacing * 0.5).max(3.0) / PPM;
        let mass = area(&poly).max(1.0) / (PPM * PPM) * DENSITY / n as f32;
        let rot = Rotation::new(angle);
        let mut balls = vec![];
        let mut rest = vec![];
        let mut uv = vec![];
        for p in &pts {
            let local = vector![(p.x - mid.x) / PPM, (p.y - mid.y) / PPM];
            let at = centre + rot * vector![p.x / PPM, p.y / PPM];
            balls.push([at.x, at.y, vel.x, vel.y]);
            rest.push(local);
            uv.push(vec2(p.x / size.x + 0.5, 0.5 - p.y / size.y));
        }
        Some(SoftState {
            kind: SoftKind::Jelly,
            name,
            texture,
            pinned: vec![false; n],
            balls,
            uv,
            rest,
            radius,
            mass,
            edges: vec![],
            tris: vec![],
            cols: 0,
        })
    }

    /// A cloth covering `size` px with its top-left corner at `top_left`
    /// (world px), hung from pins along its top edge.
    pub fn cloth(name: String, texture: Option<Texture2D>, top_left: Vec2, size: Vec2) -> Self {
        let cols = ((size.x / CLOTH_SPACING).round() as usize + 1).clamp(4, 26);
        let rows = ((size.y / CLOTH_SPACING).round() as usize + 1).clamp(3, 26);
        let (dx, dy) = (size.x / (cols - 1) as f32, size.y / (rows - 1) as f32);
        let mut balls = vec![];
        let mut uv = vec![];
        let mut pinned = vec![];
        let (x0, y0) = super::to_phys(top_left.x, top_left.y);
        for j in 0..rows {
            for i in 0..cols {
                let (x, y) = (x0 + i as f32 * dx / PPM, y0 - j as f32 * dy / PPM);
                balls.push([x, y, 0.0, 0.0]);
                uv.push(vec2(i as f32 / (cols - 1) as f32, j as f32 / (rows - 1) as f32));
                // Pins at the corners and every few knots between.
                pinned.push(j == 0 && (i % 4 == 0 || i == cols - 1));
            }
        }
        let id = |i: usize, j: usize| j * cols + i;
        let (hx, hy, hd) = (dx / PPM, dy / PPM, (dx * dx + dy * dy).sqrt() / PPM);
        let mut edges = vec![];
        let mut tris = vec![];
        for j in 0..rows {
            for i in 0..cols {
                if i + 1 < cols {
                    edges.push((id(i, j), id(i + 1, j), hx));
                }
                if j + 1 < rows {
                    edges.push((id(i, j), id(i, j + 1), hy));
                }
                if i + 1 < cols && j + 1 < rows {
                    let (a, b, c, d) = (id(i, j), id(i + 1, j), id(i + 1, j + 1), id(i, j + 1));
                    edges.push((a, c, hd));
                    edges.push((b, d, hd));
                    tris.push(([a, b, c], [(a, b), (b, c), (a, c)]));
                    tris.push(([a, c, d], [(a, c), (d, c), (a, d)]));
                }
            }
        }
        let n = balls.len();
        let mass = size.x * size.y / (PPM * PPM) * DENSITY * CLOTH_DENSITY / n as f32;
        let radius = (dx.min(dy) * 0.36).max(3.0) / PPM;
        SoftState {
            kind: SoftKind::Cloth,
            name,
            texture,
            balls,
            pinned,
            uv,
            rest: vec![],
            radius,
            mass,
            edges,
            tris,
            cols,
        }
    }

    /// Build the soft body in `world`, moved by `offset` (m).
    pub fn restore(&self, world: &mut PhysWorld, offset: Vector<f32>) -> Soft {
        let cloth = self.kind == SoftKind::Cloth;
        let density = self.mass / (std::f32::consts::PI * self.radius * self.radius);
        let group = super::events::new_soft_group();
        let balls: Vec<RigidBodyHandle> = self
            .balls
            .iter()
            .zip(&self.pinned)
            .map(|(&[x, y, vx, vy], &pinned)| {
                let ty = if pinned { RigidBodyType::KinematicPositionBased } else { RigidBodyType::Dynamic };
                let body = RigidBodyBuilder::new(ty)
                    .translation(vector![x, y] + offset)
                    .linvel(vector![vx, vy])
                    .linear_damping(if cloth { 0.8 } else { 0.1 })
                    .lock_rotations()
                    .ccd_enabled(!cloth)
                    .build();
                let h = world.bodies.insert(body);
                let mut col = ColliderBuilder::ball(self.radius)
                    .density(density)
                    .friction(if cloth { 0.5 } else { 0.9 })
                    .restitution(if cloth { 0.0 } else { 0.3 });
                if cloth {
                    col = col.collision_groups(InteractionGroups::new(CLOTH_GROUP, Group::ALL ^ CLOTH_GROUP));
                }
                let mut col = col.build();
                if !cloth {
                    // The balls of one jelly pass through each other, so a
                    // crumpled jelly can always spring back.
                    super::events::set_soft_group(&mut col, group);
                }
                world.colliders.insert_with_parent(col, h, &mut world.bodies);
                h
            })
            .collect();
        let edges = self
            .edges
            .iter()
            .map(|&(a, b, rest)| {
                let joint = RopeJointBuilder::new(rest).contacts_enabled(false).build();
                let joint = Some(world.impulse_joints.insert(balls[a], balls[b], joint, true));
                Edge { a, b, rest, joint }
            })
            .collect();
        Soft {
            kind: self.kind,
            name: self.name.clone(),
            texture: self.texture.clone(),
            balls,
            pinned: self.pinned.clone(),
            uv: self.uv.clone(),
            rest: self.rest.clone(),
            radius: self.radius,
            mass: self.mass,
            edges,
            tris: self.tris.clone(),
            cols: self.cols,
            burn: vec![0.0; self.balls.len()],
        }
    }
}

impl Soft {
    /// Current state, for undo.
    pub fn state(&self, world: &PhysWorld) -> SoftState {
        let balls = self
            .balls
            .iter()
            .map(|&h| match world.bodies.get(h) {
                Some(b) => [b.translation().x, b.translation().y, b.linvel().x, b.linvel().y],
                None => [0.0; 4],
            })
            .collect();
        SoftState {
            kind: self.kind,
            name: self.name.clone(),
            texture: self.texture.clone(),
            balls,
            pinned: self.pinned.clone(),
            uv: self.uv.clone(),
            rest: self.rest.clone(),
            radius: self.radius,
            mass: self.mass,
            edges: self.edges.iter().filter(|e| e.joint.is_some()).map(|e| (e.a, e.b, e.rest)).collect(),
            tris: self.tris.clone(),
            cols: self.cols,
        }
    }

    pub fn remove(self, world: &mut PhysWorld) {
        for h in self.balls {
            world.remove_body(h);
        }
    }

    /// Ball positions (world px).
    pub fn points(&self, world: &PhysWorld) -> Vec<Vec2> {
        self.balls
            .iter()
            .map(|&h| world.bodies.get(h).map_or(Vec2::ZERO, |b| to_screen(b.translation().x, b.translation().y)))
            .collect()
    }

    pub fn centre(&self, world: &PhysWorld) -> Vec2 {
        let pts = self.points(world);
        pts.iter().copied().sum::<Vec2>() / pts.len().max(1) as f32
    }

    pub fn radius_px(&self) -> f32 {
        self.radius * PPM
    }

    /// Whether the world point `p` (px) is on this soft body.
    pub fn contains(&self, world: &PhysWorld, p: Vec2) -> bool {
        let pts = self.points(world);
        match self.kind {
            SoftKind::Jelly => inside(&pts, p),
            SoftKind::Cloth => {
                self.tris.iter().any(|(t, need)| self.whole(need) && inside(&[pts[t[0]], pts[t[1]], pts[t[2]]], p))
            }
        }
    }

    /// Index of the ball nearest to `p` (world px), within `reach` px.
    pub fn nearest(&self, world: &PhysWorld, p: Vec2, reach: f32) -> Option<usize> {
        self.points(world)
            .iter()
            .enumerate()
            .map(|(i, q)| (i, q.distance(p)))
            .filter(|&(_, d)| d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// Whether the threads `need` are all still whole.
    fn whole(&self, need: &[(usize, usize); 3]) -> bool {
        need.iter().all(|&(a, b)| self.edges.iter().any(|e| e.a == a && e.b == b && e.joint.is_some()))
    }

    /// Both ends of thread `e` (world px).
    pub fn edge_ends(&self, world: &PhysWorld, e: usize) -> Option<(Vec2, Vec2)> {
        let edge = self.edges.get(e).filter(|e| e.joint.is_some())?;
        let p = |i: usize| world.bodies.get(self.balls[i]).map(|b| to_screen(b.translation().x, b.translation().y));
        Some((p(edge.a)?, p(edge.b)?))
    }

    /// Cut thread `e`.
    pub fn cut(&mut self, world: &mut PhysWorld, e: usize) {
        if let Some(j) = self.edges.get_mut(e).and_then(|e| e.joint.take()) {
            world.impulse_joints.remove(j, true);
        }
    }

    /// Cut every thread touching ball `i` (burnt through).
    pub fn cut_ball(&mut self, world: &mut PhysWorld, i: usize) {
        for e in 0..self.edges.len() {
            if self.edges[e].a == i || self.edges[e].b == i {
                self.cut(world, e);
            }
        }
    }

    /// Threads of ball `i` still whole, and the balls at their other end.
    pub fn neighbours(&self, i: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|e| e.joint.is_some())
            .filter_map(|e| {
                if e.a == i {
                    Some(e.b)
                } else if e.b == i {
                    Some(e.a)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Whether any cloth triangle is left.
    pub fn has_cloth_left(&self) -> bool {
        self.tris.iter().any(|(_, need)| self.whole(need))
    }

    /// Jelly: pull every ball toward its place in the best-fitting copy of
    /// the rest shape. Call once per frame before stepping.
    pub fn apply_forces(&self, world: &mut PhysWorld) {
        if self.kind != SoftKind::Jelly {
            return;
        }
        let n = self.balls.len();
        let mut pos = Vec::with_capacity(n);
        let mut vel = Vec::with_capacity(n);
        for &h in &self.balls {
            let Some(b) = world.bodies.get(h) else { return };
            pos.push(*b.translation());
            vel.push(*b.linvel());
        }
        let c = pos.iter().fold(vector![0.0, 0.0], |s, p| s + p) / n as f32;
        let vc = vel.iter().fold(vector![0.0, 0.0], |s, v| s + v) / n as f32;
        let (mut sin, mut cos) = (0.0, 0.0);
        for (p, r) in pos.iter().zip(&self.rest) {
            let q = p - c;
            sin += r.x * q.y - r.y * q.x;
            cos += r.x * q.x + r.y * q.y;
        }
        let rot = Rotation::new(sin.atan2(cos));
        // Pressure: squashed (or turned inside out), it pushes outward.
        let rest_area = signed_area(&self.rest);
        let deficit = ((rest_area - signed_area(&pos)) / rest_area).clamp(-0.5, 1.5);
        let side = rest_area.signum();
        let mut forces: Vec<Vector<f32>> = (0..n)
            .map(|k| {
                let goal = c + rot * self.rest[k];
                let t = pos[(k + 1) % n] - pos[(k + n - 1) % n];
                let out = vector![t.y, -t.x].try_normalize(1e-6).unwrap_or_default() * side;
                (goal - pos[k]) * JELLY_K - (vel[k] - vc) * JELLY_C + out * JELLY_P * deficit
            })
            .collect();
        // Skin springs between neighbours.
        for k in 0..n {
            let j = (k + 1) % n;
            let e = pos[j] - pos[k];
            let len = e.norm();
            if len < 1e-6 {
                continue;
            }
            let rest = (self.rest[j] - self.rest[k]).norm();
            let dir = e / len;
            let closing = (vel[j] - vel[k]).dot(&dir);
            let f = dir * ((len - rest) * JELLY_SKIN_K + closing * JELLY_C * 2.0);
            forces[k] += f;
            forces[j] -= f;
        }
        for (&h, f) in self.balls.iter().zip(forces) {
            if let Some(b) = world.bodies.get_mut(h) {
                b.add_force(f * self.mass, true);
            }
        }
    }

    /// Cloth: snap threads stretched too far. Returns where they snapped (px).
    pub fn tear(&mut self, world: &mut PhysWorld) -> Vec<Vec2> {
        let mut snapped = vec![];
        for e in 0..self.edges.len() {
            let Some((a, b)) = self.edge_ends_m(world, e) else { continue };
            if (a - b).norm() > self.edges[e].rest * TEAR_STRETCH {
                self.cut(world, e);
                let mid = (a + b) / 2.0;
                snapped.push(to_screen(mid.x, mid.y));
            }
        }
        snapped
    }

    /// Both ends of thread `e` (m).
    fn edge_ends_m(&self, world: &PhysWorld, e: usize) -> Option<(Vector<f32>, Vector<f32>)> {
        let edge = self.edges.get(e).filter(|e| e.joint.is_some())?;
        let p = |i: usize| world.bodies.get(self.balls[i]).map(|b| *b.translation());
        Some((p(edge.a)?, p(edge.b)?))
    }

    /// Draw the textured mesh (world pass).
    pub fn draw(&self, world: &PhysWorld) {
        let pts = self.points(world);
        match self.kind {
            SoftKind::Jelly => self.draw_jelly(&pts),
            SoftKind::Cloth => self.draw_cloth(&pts),
        }
    }

    fn draw_jelly(&self, pts: &[Vec2]) {
        let n = pts.len();
        let c = pts.iter().copied().sum::<Vec2>() / n as f32;
        // The skin sits on the outside of the balls (y is down on screen,
        // so the outward side flips with the outline's winding).
        let r = self.radius_px();
        let side = -signed_area(&self.rest).signum();
        let outer: Vec<Vec2> = (0..n)
            .map(|k| {
                let t = pts[(k + 1) % n] - pts[(k + n - 1) % n];
                pts[k] + vec2(t.y, -t.x).normalize_or_zero() * side * r
            })
            .collect();
        // One round of corner cutting smooths the outline.
        let mut skin = Vec::with_capacity(n * 2);
        let mut uvs = Vec::with_capacity(n * 2);
        for k in 0..n {
            let j = (k + 1) % n;
            for t in [0.25, 0.75] {
                skin.push(outer[k].lerp(outer[j], t));
                uvs.push(self.uv[k].lerp(self.uv[j], t));
            }
        }
        let n = skin.len();
        let uv_c = self.uv.iter().copied().sum::<Vec2>() / self.uv.len() as f32;
        let tint = Color::new(1.0, 1.0, 1.0, 0.93);
        let mut vertices = vec![Vertex::new(c.x, c.y, 0.0, uv_c.x, uv_c.y, tint)];
        for (p, uv) in skin.iter().zip(&uvs) {
            vertices.push(Vertex::new(p.x, p.y, 0.0, uv.x, uv.y, tint));
        }
        let mut indices = vec![];
        for i in 0..n {
            indices.extend([0, 1 + i as u16, 1 + ((i + 1) % n) as u16]);
        }
        // Shadow, body and a glossy highlight.
        for v in &mut vertices {
            v.position += vec3(3.0, 4.0, 0.0);
            v.color = [0, 0, 0, 50];
        }
        draw_mesh(&Mesh { vertices: vertices.clone(), indices: indices.clone(), texture: self.texture.clone() });
        for v in &mut vertices {
            v.position -= vec3(3.0, 4.0, 0.0);
            v.color = tint.into();
        }
        draw_mesh(&Mesh { vertices, indices, texture: self.texture.clone() });
        let size = skin.iter().map(|p| p.distance(c)).fold(0.0, f32::max);
        let top = skin.iter().copied().fold(c, |best, p| if p.y < best.y { p } else { best });
        let hl = c.lerp(top, 0.55) - vec2(size * 0.18, 0.0);
        draw_ellipse(hl.x, hl.y, size * 0.22, size * 0.1, -20.0, Color::new(1.0, 1.0, 1.0, 0.28));
        for i in 0..n {
            let (a, b) = (skin[i], skin[(i + 1) % n]);
            draw_line(a.x, a.y, b.x, b.y, 1.2, Color::new(0.0, 0.0, 0.0, 0.25));
        }
    }

    fn draw_cloth(&self, pts: &[Vec2]) {
        let cols = self.cols.max(1);
        // Folds (where the cloth bunches up sideways) are shaded darker.
        let rest = self.edges.first().map_or(1.0, |e| e.rest * PPM);
        let vertices: Vec<Vertex> = pts
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let (i, j) = (k % cols, k / cols);
                let left = if i > 0 { pts[k - 1] } else { *p };
                let right = if i + 1 < cols && k + 1 < pts.len() { pts[k + 1] } else { *p };
                let span = if i > 0 && i + 1 < cols { 2.0 } else { 1.0 };
                let squeeze = (left.distance(right) / (rest * span)).clamp(0.0, 1.0);
                let charred = match self.burn[k] {
                    b if b < 0.0 => 1.0,
                    b => (b / 1.2).min(1.0),
                };
                let shade = (0.55 + 0.45 * squeeze.powf(1.5) - (j as f32 * 0.004).min(0.1)) * (1.0 - 0.75 * charred);
                Vertex::new(p.x, p.y, 0.0, self.uv[k].x, self.uv[k].y, Color::new(shade, shade, shade, 1.0))
            })
            .collect();
        let indices: Vec<u16> =
            self.tris.iter().filter(|(_, need)| self.whole(need)).flat_map(|(t, _)| t.map(|i| i as u16)).collect();
        if !indices.is_empty() {
            draw_mesh(&Mesh { vertices, indices, texture: self.texture.clone() });
        }
        for (i, p) in pts.iter().enumerate().filter(|(i, _)| self.pinned[*i]) {
            if self.neighbours(i).is_empty() {
                continue;
            }
            draw_circle(p.x, p.y + 1.0, 4.0, Color::new(0.0, 0.0, 0.0, 0.35));
            draw_circle(p.x, p.y, 3.5, Color::from_rgba(200, 200, 215, 255));
            draw_circle(p.x - 1.0, p.y - 1.0, 1.2, WHITE);
        }
    }
}

/// Signed area of a polygon (m², positive counter-clockwise with y up).
fn signed_area(p: &[Vector<f32>]) -> f32 {
    let n = p.len();
    (0..n).map(|i| p[i].x * p[(i + 1) % n].y - p[(i + 1) % n].x * p[i].y).sum::<f32>() / 2.0
}

/// Point in polygon (even-odd rule).
fn inside(poly: &[Vec2], p: Vec2) -> bool {
    let n = poly.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
    }
    c
}

/// A woven fabric texture in `rgb`, `w` × `h` px, with a darker hem.
pub fn fabric(rgb: (u8, u8, u8), w: u32, h: u32) -> image::RgbaImage {
    image::RgbaImage::from_fn(w, h, |x, y| {
        let weave = if (x / 2 + y / 2) % 2 == 0 { 1.0 } else { 0.93 };
        let stripe = if (x / 24) % 2 == 0 { 1.0 } else { 0.9 };
        let hem = if x < 5 || y < 5 || x + 5 >= w || y + 5 >= h { 0.72 } else { 1.0 };
        let k = weave * stripe * hem;
        let c = |v: u8| (v as f32 * k).min(255.0) as u8;
        image::Rgba([c(rgb.0), c(rgb.1), c(rgb.2), 255])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;

    #[test]
    fn resampling_spaces_points_evenly() {
        let square = [vec2(0.0, 0.0), vec2(10.0, 0.0), vec2(10.0, 10.0), vec2(0.0, 10.0)];
        let pts = resample(&square, 8);
        assert_eq!(pts.len(), 8);
        for i in 0..8 {
            let d = pts[i].distance(pts[(i + 1) % 8]);
            assert!((d - 5.0).abs() < 0.01, "step {i}: {d}");
        }
        assert!((area(&square) - 100.0).abs() < 1e-3);
        assert!(inside(&square, vec2(5.0, 5.0)) && !inside(&square, vec2(15.0, 5.0)));
    }

    /// Textures need a window: the tests use none.
    fn texture() -> Option<Texture2D> {
        None
    }

    #[test]
    fn jelly_keeps_its_shape_when_dropped() {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (1200.0, 800.0));
        let circle: Vec<[f32; 2]> = (0..32)
            .map(|i| {
                let a = i as f32 / 32.0 * std::f32::consts::TAU;
                [a.cos() * 0.5, a.sin() * 0.5]
            })
            .collect();
        let st = SoftState::jelly(
            "jelly".into(),
            texture(),
            &circle,
            vec2(100.0, 100.0),
            vector![10.0, 8.0],
            0.0,
            vector![0.0, 0.0],
        )
        .unwrap();
        let soft = st.restore(&mut w, vector![0.0, 0.0]);
        for _ in 0..360 {
            w.reset_forces();
            soft.apply_forces(&mut w);
            w.step_fixed();
        }
        // Physics coordinates: other tests may resize the (global) world height.
        let pts: Vec<Vector<f32>> = soft.balls.iter().map(|&h| *w.bodies[h].translation()).collect();
        let c = pts.iter().fold(vector![0.0, 0.0], |s, p| s + p) / pts.len() as f32;
        let floor = crate::config::WALL_T;
        assert!(c.y > floor + 0.5 && c.y < floor + 1.0, "rests on the floor: {c:?}");
        let (lo, hi) =
            pts.iter().map(|p| (p - c).norm() * PPM).fold((f32::MAX, 0.0f32), |(lo, hi), r| (lo.min(r), hi.max(r)));
        assert!(lo > 38.0 && hi < 60.0, "stays roughly round: {lo}..{hi}");
    }

    #[test]
    fn jelly_springs_back_after_being_crushed() {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (1200.0, 800.0));
        let circle: Vec<[f32; 2]> = (0..32)
            .map(|i| {
                let a = i as f32 / 32.0 * std::f32::consts::TAU;
                [a.cos() * 0.5, a.sin() * 0.5]
            })
            .collect();
        let floor = crate::config::WALL_T;
        let st = SoftState::jelly(
            "jelly".into(),
            texture(),
            &circle,
            vec2(100.0, 100.0),
            vector![10.0, floor + 0.9],
            0.0,
            vector![0.0, 0.0],
        )
        .unwrap();
        let soft = st.restore(&mut w, vector![0.0, 0.0]);
        // A heavy slab lands on it…
        let slab = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![10.0, floor + 2.5]));
        w.colliders.insert_with_parent(ColliderBuilder::cuboid(1.5, 0.3).density(20.0), slab, &mut w.bodies);
        let step = |w: &mut PhysWorld, n: usize| {
            for _ in 0..n {
                w.reset_forces();
                soft.apply_forces(w);
                w.step_fixed();
            }
        };
        step(&mut w, 240);
        let squashed = w.bodies[slab].translation().y - floor;
        assert!(squashed > 0.5, "the jelly holds the slab up: {squashed}");
        // …and is lifted off again.
        w.remove_body(slab);
        step(&mut w, 360);
        let pts: Vec<Vector<f32>> = soft.balls.iter().map(|&h| *w.bodies[h].translation()).collect();
        let c = pts.iter().fold(vector![0.0, 0.0], |s, p| s + p) / pts.len() as f32;
        let (lo, hi) =
            pts.iter().map(|p| (p - c).norm() * PPM).fold((f32::MAX, 0.0f32), |(lo, hi), r| (lo.min(r), hi.max(r)));
        assert!(lo > 38.0 && hi < 60.0, "round again: {lo}..{hi}");
    }

    #[test]
    fn cloth_hangs_and_tears() {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (1200.0, 800.0));
        let st = SoftState::cloth("cloth".into(), texture(), vec2(400.0, 100.0), vec2(180.0, 120.0));
        let mut soft = st.restore(&mut w, vector![0.0, 0.0]);
        for _ in 0..240 {
            w.step_fixed();
            soft.tear(&mut w);
        }
        assert!(soft.has_cloth_left(), "its own weight does not tear it");
        let ys =
            |soft: &Soft, w: &PhysWorld| soft.balls.iter().map(|&h| w.bodies[h].translation().y).collect::<Vec<f32>>();
        let top = ys(&soft, &w).into_iter().fold(f32::MIN, f32::max);
        let drop = (top - ys(&soft, &w).into_iter().fold(f32::MAX, f32::min)) * PPM;
        assert!(drop > 100.0 && drop < 160.0, "hangs about its height below the pins: {drop}");
        // Cut every thread crossing the middle: the lower half falls away.
        let mid = top - 1.0;
        for e in 0..soft.edges.len() {
            if let Some((a, b)) = soft.edge_ends_m(&w, e) {
                if (a.y - mid) * (b.y - mid) < 0.0 {
                    soft.cut(&mut w, e);
                }
            }
        }
        for _ in 0..60 {
            w.step_fixed();
        }
        let lowest = ys(&soft, &w).into_iter().fold(f32::MAX, f32::min);
        let lowest = (top - lowest) * PPM;
        assert!(lowest > 200.0, "the cut part falls: {lowest}");
    }
}
