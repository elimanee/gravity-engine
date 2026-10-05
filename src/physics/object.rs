//! A physics object: one sprite (still or animated) attached to a rigid body.

use super::{to_phys, to_screen, PhysWorld};
use crate::assets::{self, Decoded};
use crate::config::{file_name_of, BOUNCE, DENSITY, FRICTION, MAX_IMG_PX, PPM, TRAIL_MAX};
use crate::drawing::Drawing;
use crate::shapes::{self, Shape, BOX_ROUNDING};
use macroquad::prelude::*;
use rapier2d::na;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Where an object's pixels come from. Kept around so the object can be
/// re-rasterised on resize, duplicated, or saved into a scene.
#[derive(Clone)]
pub enum Source {
    File(String),
    Memory {
        name: String,
        data: Arc<Vec<u8>>,
    },
    Shape {
        shape: Shape,
        rgb: (u8, u8, u8),
    },
    /// A live audio visualizer screen (drawn every frame, no sprite).
    Visualizer,
    /// A shape drawn with the Draw tool.
    Drawing(Arc<Drawing>),
}

impl Source {
    pub fn display_name(&self) -> String {
        match self {
            Source::File(p) => file_name_of(p),
            Source::Memory { name, .. } => name.clone(),
            Source::Shape { shape, .. } => shape.label().to_string(),
            Source::Visualizer => "Audio visualizer".to_string(),
            Source::Drawing(d) => if d.closed { "Drawn shape" } else { "Drawn plank" }.to_string(),
        }
    }

    /// Decode at `max_px` on the longest side.
    pub fn decode(&self, max_px: u32, upscale: bool) -> Option<Decoded> {
        match self {
            Source::File(p) => assets::decode_file(p, max_px, upscale),
            Source::Memory { name, data } => assets::decode(name, data, max_px, upscale),
            Source::Shape { shape, rgb } => {
                Some(Decoded { frames: vec![shapes::rasterize(*shape, max_px, *rgb)], delays_ms: vec![], hull: None })
            }
            // Drawn procedurally; a transparent placeholder keeps the sprite code happy.
            Source::Visualizer => {
                Some(Decoded { frames: vec![image::RgbaImage::new(2, 1)], delays_ms: vec![], hull: None })
            }
            Source::Drawing(d) => Some(Decoded { frames: vec![d.rasterize(max_px)], delays_ms: vec![], hull: None }),
        }
    }

    fn outline(&self, decoded: &Decoded) -> Outline {
        match self {
            Source::Shape { shape, .. } => match shape {
                Shape::Circle => Outline::Ball,
                Shape::Box => Outline::RoundBox,
                Shape::Capsule => Outline::Capsule,
                s => {
                    let poly = s.polygon().unwrap_or_default();
                    if *s == Shape::Star {
                        Outline::Concave(poly)
                    } else {
                        Outline::Hull(poly)
                    }
                }
            },
            Source::Visualizer => Outline::RoundBox,
            Source::Drawing(d) if d.closed => Outline::Concave(d.points.clone()),
            Source::Drawing(d) => Outline::Stroke { pts: d.points.clone(), thickness: d.thickness },
            _ => match &decoded.hull {
                Some(h) => Outline::Hull(h.clone()),
                None => Outline::Box,
            },
        }
    }
}

/// Collider geometry, normalised to the sprite's box.
#[derive(Clone)]
enum Outline {
    Box,
    RoundBox,
    Ball,
    Capsule,
    Hull(Vec<[f32; 2]>),
    Concave(Vec<[f32; 2]>),
    /// A thick polyline; `thickness` is relative to the longest side.
    Stroke {
        pts: Vec<[f32; 2]>,
        thickness: f32,
    },
}

impl Outline {
    fn collider(&self, hw: f32, hh: f32) -> ColliderBuilder {
        let hw = hw.max(0.02);
        let hh = hh.max(0.02);
        let scale = |pts: &[[f32; 2]]| -> Vec<na::Point2<f32>> {
            pts.iter().map(|&[u, v]| point![u * 2.0 * hw, v * 2.0 * hh]).collect()
        };
        match self {
            Outline::Box => ColliderBuilder::cuboid(hw, hh),
            Outline::RoundBox => {
                let r = (hw * 2.0 * BOX_ROUNDING).min(hw * 0.9).min(hh * 0.9);
                ColliderBuilder::round_cuboid(hw - r, hh - r, r)
            }
            Outline::Ball => ColliderBuilder::ball(hw.min(hh)),
            Outline::Capsule => ColliderBuilder::capsule_x((hw - hh).max(0.0), hh),
            Outline::Hull(pts) => {
                ColliderBuilder::convex_hull(&scale(pts)).unwrap_or_else(|| ColliderBuilder::cuboid(hw, hh))
            }
            Outline::Concave(pts) => {
                let verts = scale(pts);
                let n = verts.len() as u32;
                let idx: Vec<[u32; 2]> = (0..n).map(|i| [i, (i + 1) % n]).collect();
                let shape = SharedShape::convex_decomposition(&verts, &idx);
                // Degenerate hand-drawn outlines can decompose into nothing.
                match shape.as_compound().is_some_and(|c| !c.shapes().is_empty()) {
                    true => ColliderBuilder::new(shape),
                    false => ColliderBuilder::convex_hull(&verts).unwrap_or_else(|| ColliderBuilder::cuboid(hw, hh)),
                }
            }
            Outline::Stroke { pts, thickness } => {
                let verts = scale(pts);
                let r = (thickness * hw.max(hh)).max(0.02);
                let mut parts: Vec<(Isometry<f32>, SharedShape)> =
                    verts.windows(2).map(|w| (Isometry::identity(), SharedShape::capsule(w[0], w[1], r))).collect();
                match parts.len() {
                    0 => ColliderBuilder::ball(r),
                    1 => ColliderBuilder::new(parts.pop().expect("one part").1),
                    _ => ColliderBuilder::compound(parts),
                }
            }
        }
    }
}

/// Surface and motion properties an object can be given from its menu.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Material {
    /// Restitution.
    pub bounce: f32,
    pub friction: f32,
    /// Multiplier of the world gravity (negative floats up).
    pub gravity: f32,
    /// Shatters on a hard enough impact.
    pub breakable: bool,
    /// Speed change (m/s) of an impact that breaks the object.
    pub strength: f32,
    /// Magnet strength: positive attracts other magnets, negative repels them.
    pub magnet: f32,
    /// Conveyor surface speed (m/s); positive moves things clockwise around it.
    pub conveyor: f32,
    /// Catches fire when heated (otherwise it only glows).
    pub flammable: bool,
    /// Reflects laser beams.
    pub mirror: bool,
    /// Planet: gravity at its surface (m/s²) pulling everything else in; 0: none.
    pub planet: f32,
    /// For Sonic's loops: 0 always solid, 1 only on the way in, 2 only on
    /// the way out, 3 the top that switches between them.
    pub layer: u8,
}

impl Default for Material {
    fn default() -> Self {
        Material::DEFAULT
    }
}

impl Material {
    pub const DEFAULT: Material = Material {
        bounce: BOUNCE,
        friction: FRICTION,
        gravity: 1.0,
        breakable: false,
        strength: 9.0,
        magnet: 0.0,
        conveyor: 0.0,
        flammable: true,
        mirror: false,
        planet: 0.0,
        layer: 0,
    };
    pub const RUBBER: Material = Material { bounce: 0.92, friction: 0.9, ..Material::DEFAULT };
    pub const ICE: Material = Material { bounce: 0.05, friction: 0.0, flammable: false, ..Material::DEFAULT };
    pub const BALLOON: Material = Material { bounce: 0.6, friction: 0.4, gravity: -0.25, ..Material::DEFAULT };
    pub const GLASS: Material =
        Material { bounce: 0.2, friction: 0.4, breakable: true, strength: 6.0, flammable: false, ..Material::DEFAULT };
    pub const MAGNET: Material = Material { magnet: 1.0, friction: 0.8, flammable: false, ..Material::DEFAULT };

    /// A planet: its own gravity, unaffected by the world's.
    pub const PLANET: Material =
        Material { planet: 12.0, gravity: 0.0, bounce: 0.2, friction: 0.9, flammable: false, ..Material::DEFAULT };
    pub const MIRROR: Material =
        Material { bounce: 0.3, friction: 0.3, flammable: false, mirror: true, ..Material::DEFAULT };

    /// Glass (breakable, not flammable): laser beams go through it.
    pub fn is_glass(&self) -> bool {
        self.breakable && !self.flammable && !self.mirror
    }

    /// The Ice preset (or close to it): melts instead of burning.
    pub fn is_ice(&self) -> bool {
        self.friction < 0.05 && self.bounce < 0.2 && !self.flammable
    }
}

/// Impact reporting, conveyor behaviour and physical coefficients of a collider.
fn configure(c: &mut Collider, m: &Material, mass: f32) {
    c.set_restitution(m.bounce);
    // Rubber and bouncier: the bouncy side wins, so a trampoline bounces
    // whatever lands on it (the default averages both sides).
    let rule = if m.bounce >= 0.9 { CoefficientCombineRule::Max } else { CoefficientCombineRule::Average };
    c.set_restitution_combine_rule(rule);
    c.set_friction(m.friction);
    c.set_active_events(ActiveEvents::CONTACT_FORCE_EVENTS);
    c.set_contact_force_event_threshold(mass.max(1e-3) * super::events::IMPACT_ACCEL);
    super::events::set_conveyor_speed(c, m.conveyor);
}

/// Initial placement of a new object.
#[derive(Clone, Copy, Default)]
pub struct Placement {
    /// Centre, in screen pixels.
    pub pos_px: (f32, f32),
    pub angle: f32,
    pub linvel: (f32, f32),
    pub angvel: f32,
    /// Display size in pixels; `None` keeps the decoded size.
    pub size_px: Option<(f32, f32)>,
    pub pinned: bool,
    pub material: Material,
    /// Mass in kg; `None` derives it from the size.
    pub mass: Option<f32>,
}

struct Ghost {
    pos: Vec2,
    angle: f32,
    frame: usize,
    time: f64,
}

pub struct Object {
    pub source: Source,
    frames: Vec<Texture2D>,
    delays_ms: Vec<u16>,
    frame_idx: usize,
    frame_timer: f32,
    outline: Outline,
    pub body: RigidBodyHandle,
    pub collider: ColliderHandle,
    /// Display size in pixels.
    pub size: Vec2,
    /// Mass is preserved across resizes.
    pub mass: f32,
    pub pinned: bool,
    pub material: Material,
    trail: VecDeque<Ghost>,
}

fn upload(img: &image::RgbaImage) -> Texture2D {
    let tex = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, img);
    tex.set_filter(FilterMode::Linear);
    tex
}

/// GPU-side sprite data. Cheap to clone (textures are reference counted), so
/// duplicates and rapid-fire spawns share it.
#[derive(Clone)]
pub struct Visual {
    frames: Vec<Texture2D>,
    delays_ms: Vec<u16>,
    outline: Outline,
}

impl Visual {
    pub fn upload(source: &Source, decoded: &Decoded) -> Self {
        Visual {
            frames: decoded.frames.iter().map(upload).collect(),
            delays_ms: decoded.delays_ms.clone(),
            outline: source.outline(decoded),
        }
    }

    /// Natural size of the first frame in pixels.
    pub fn size(&self) -> (f32, f32) {
        (self.frames[0].width(), self.frames[0].height())
    }
}

impl Object {
    /// Build an object from an uploaded visual.
    pub fn spawn(world: &mut PhysWorld, source: Source, visual: Visual, at: Placement) -> Self {
        let (w, h) = at.size_px.unwrap_or_else(|| visual.size());
        let (bx, by) = to_phys(at.pos_px.0, at.pos_px.1);
        let body_type = if at.pinned { RigidBodyType::KinematicPositionBased } else { RigidBodyType::Dynamic };
        let body = RigidBodyBuilder::new(body_type)
            .translation(vector![bx, by])
            .rotation(at.angle)
            .linvel(vector![at.linvel.0, at.linvel.1])
            .angvel(at.angvel)
            .gravity_scale(at.material.gravity)
            .angular_damping(0.6)
            .ccd_enabled(true)
            .build();
        let body = world.bodies.insert(body);
        let mut col = visual.outline.collider(w / 2.0 / PPM, h / 2.0 / PPM).density(DENSITY).build();
        if let Some(m) = at.mass.filter(|m| m.is_finite() && *m > 0.0) {
            col.set_density(DENSITY * m / col.mass().max(1e-6));
        }
        let mass = col.mass();
        configure(&mut col, &at.material, mass);
        let collider = world.colliders.insert_with_parent(col, body, &mut world.bodies);

        Object {
            source,
            frames: visual.frames,
            delays_ms: visual.delays_ms,
            frame_idx: 0,
            frame_timer: 0.0,
            outline: visual.outline,
            body,
            collider,
            size: vec2(w, h),
            mass,
            pinned: at.pinned,
            material: at.material,
            trail: VecDeque::new(),
        }
    }

    /// Decode `source` and spawn it. Returns `None` when decoding fails.
    pub fn load(world: &mut PhysWorld, source: Source, at: Placement) -> Option<Self> {
        let max_px = match at.size_px {
            Some((w, h)) => w.max(h).ceil() as u32,
            None => MAX_IMG_PX,
        };
        let upscale = at.size_px.is_some();
        let decoded = source.decode(max_px.max(8), upscale)?;
        let visual = Visual::upload(&source, &decoded);
        Some(Self::spawn(world, source, visual, at))
    }

    /// This object's sprite and collider geometry (textures are shared).
    pub fn visual(&self) -> Visual {
        Visual { frames: self.frames.clone(), delays_ms: self.delays_ms.clone(), outline: self.outline.clone() }
    }

    /// A copy of this object (sharing its textures) at `at`.
    pub fn duplicate(&self, world: &mut PhysWorld, at: Placement) -> Self {
        let mut o = Self::spawn(
            world,
            self.source.clone(),
            self.visual(),
            Placement {
                size_px: Some((self.size.x, self.size.y)),
                material: self.material,
                mass: Some(self.mass),
                ..at
            },
        );
        o.frame_idx = self.frame_idx;
        o
    }

    pub fn name(&self) -> String {
        self.source.display_name()
    }

    pub fn is_visualizer(&self) -> bool {
        matches!(self.source, Source::Visualizer)
    }

    pub fn is_animated(&self) -> bool {
        self.frames.len() > 1
    }

    /// Index of the animation frame currently shown.
    pub fn frame(&self) -> usize {
        self.frame_idx
    }

    pub fn texture(&self) -> &Texture2D {
        &self.frames[self.frame_idx]
    }

    /// Current state as a placement (used for duplication and scene saving).
    pub fn placement(&self, world: &PhysWorld) -> Placement {
        let b = &world.bodies[self.body];
        let p = to_screen(b.translation().x, b.translation().y);
        Placement {
            pos_px: (p.x, p.y),
            angle: b.rotation().angle(),
            linvel: (b.linvel().x, b.linvel().y),
            angvel: b.angvel(),
            size_px: Some((self.size.x, self.size.y)),
            pinned: self.pinned,
            material: self.material,
            mass: Some(self.mass),
        }
    }

    pub fn screen_pos(&self, world: &PhysWorld) -> (Vec2, f32) {
        let b = &world.bodies[self.body];
        (to_screen(b.translation().x, b.translation().y), b.rotation().angle())
    }

    pub fn update_anim(&mut self, dt_ms: f32) {
        if self.frames.len() < 2 {
            return;
        }
        self.frame_timer += dt_ms;
        // Bounded loop: a huge dt (e.g. after a file dialog) must not spin.
        for _ in 0..self.frames.len() {
            let delay = self.delays_ms[self.frame_idx].max(10) as f32;
            if self.frame_timer < delay {
                return;
            }
            self.frame_timer -= delay;
            self.frame_idx = (self.frame_idx + 1) % self.frames.len();
        }
        self.frame_timer = 0.0;
    }

    pub fn update_trail(&mut self, world: &PhysWorld, max_len: usize, fade_secs: f32) {
        let now = get_time();
        while self.trail.front().is_some_and(|g| (now - g.time) as f32 > fade_secs) {
            self.trail.pop_front();
        }
        let max_len = max_len.min(TRAIL_MAX);
        while self.trail.len() > max_len {
            self.trail.pop_front();
        }
        let (pos, angle) = self.screen_pos(world);
        let far_enough = self.trail.back().is_none_or(|g| g.pos.distance_squared(pos) >= 9.0);
        if far_enough && max_len > 0 {
            self.trail.push_back(Ghost { pos, angle, frame: self.frame_idx, time: now });
            if self.trail.len() > max_len {
                self.trail.pop_front();
            }
        }
    }

    pub fn clear_trail(&mut self) {
        self.trail.clear();
    }

    pub fn draw_trail(&self, fade_secs: f32) {
        let now = get_time();
        for g in &self.trail {
            let youth = 1.0 - ((now - g.time) as f32 / fade_secs).clamp(0.0, 1.0);
            let alpha = youth * youth * 0.40;
            if alpha < 0.005 {
                continue;
            }
            self.draw_sprite(&self.frames[g.frame], g.pos, g.angle, Color::new(1.0, 1.0, 1.0, alpha));
        }
    }

    fn draw_sprite(&self, tex: &Texture2D, pos: Vec2, angle: f32, tint: Color) {
        draw_texture_ex(
            tex,
            pos.x - self.size.x / 2.0,
            pos.y - self.size.y / 2.0,
            tint,
            DrawTextureParams { dest_size: Some(self.size), rotation: -angle, ..Default::default() },
        );
    }

    /// Draw with the sprite multiplied by `tint` (charring, heat glow).
    pub fn draw_tinted(&self, world: &PhysWorld, tint: Color) {
        let (pos, angle) = self.screen_pos(world);
        // Soft contact shadow.
        self.draw_sprite(self.texture(), pos + vec2(3.0, 4.0), angle, Color::new(0.0, 0.0, 0.0, 0.22));
        self.draw_sprite(self.texture(), pos, angle, tint);
    }

    /// The collider's outline as a polygon, normalised to the sprite box
    /// ([-0.5, 0.5]², v up), in order around it.
    pub fn outline_polygon(&self) -> Vec<[f32; 2]> {
        let (w, h) = (self.size.x.max(1.0), self.size.y.max(1.0));
        let rect = vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
        // Rounded corners: arcs of radius (ru, rv) around the inner corners.
        let rounded = |ru: f32, rv: f32| -> Vec<[f32; 2]> {
            let mut pts = vec![];
            for (k, (cx, cy)) in
                [(0.5 - ru, 0.5 - rv), (-0.5 + ru, 0.5 - rv), (-0.5 + ru, -0.5 + rv), (0.5 - ru, -0.5 + rv)]
                    .into_iter()
                    .enumerate()
            {
                for i in 0..=6 {
                    let a = (k as f32 + i as f32 / 6.0) * std::f32::consts::FRAC_PI_2;
                    pts.push([cx + a.cos() * ru, cy + a.sin() * rv]);
                }
            }
            pts
        };
        match &self.outline {
            Outline::Box | Outline::Stroke { .. } => rect,
            Outline::RoundBox => {
                let r = (w * BOX_ROUNDING).min(w * 0.45).min(h * 0.45);
                rounded(r / w, r / h)
            }
            Outline::Ball => {
                let r = w.min(h) / 2.0;
                (0..32)
                    .map(|i| {
                        let a = i as f32 / 32.0 * std::f32::consts::TAU;
                        [a.cos() * r / w, a.sin() * r / h]
                    })
                    .collect()
            }
            Outline::Capsule => rounded((h / 2.0).min(w / 2.0) / w, 0.5),
            Outline::Hull(pts) => convex_hull(pts),
            Outline::Concave(pts) => pts.clone(),
        }
    }

    /// Four corners of the sprite box in screen space.
    pub fn corners(&self, world: &PhysWorld) -> [Vec2; 4] {
        let (pos, angle) = self.screen_pos(world);
        let (s, c) = (-angle).sin_cos();
        let (hw, hh) = (self.size.x / 2.0, self.size.y / 2.0);
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| pos + vec2(x * c - y * s, x * s + y * c))
    }

    /// Scale the object by `factor`, re-rasterising for sharpness and keeping its mass.
    pub fn resize(&mut self, world: &mut PhysWorld, factor: f32) {
        let new = (self.size * factor).max(vec2(8.0, 8.0)).min(vec2(2400.0, 2400.0));
        let longest = new.x.max(new.y).ceil() as u32;
        if let Some(dec) = self.source.decode(longest, true) {
            self.frames = dec.frames.iter().map(upload).collect();
            self.delays_ms = dec.delays_ms;
            self.frame_idx = self.frame_idx.min(self.frames.len() - 1);
        }
        self.size = new;
        self.trail.clear();

        world.remove_collider(self.collider);
        let mut col = self.outline.collider(new.x / 2.0 / PPM, new.y / 2.0 / PPM).density(1.0).build();
        let unit_mass = col.mass().max(1e-6);
        col.set_density(self.mass / unit_mass);
        configure(&mut col, &self.material, self.mass);
        self.collider = world.colliders.insert_with_parent(col, self.body, &mut world.bodies);
        if let Some(b) = world.bodies.get_mut(self.body) {
            b.wake_up(true);
        }
    }

    pub fn set_material(&mut self, world: &mut PhysWorld, m: Material) {
        self.material = m;
        if let Some(c) = world.colliders.get_mut(self.collider) {
            configure(c, &m, self.mass);
        }
        if let Some(b) = world.bodies.get_mut(self.body) {
            b.set_gravity_scale(m.gravity, true);
        }
    }

    pub fn set_mass(&mut self, world: &mut PhysWorld, mass: f32) {
        let Some(c) = world.colliders.get_mut(self.collider) else { return };
        let unit = c.shape().mass_properties(1.0).mass().max(1e-6);
        c.set_density(mass / unit);
        c.set_contact_force_event_threshold(mass.max(1e-3) * super::events::IMPACT_ACCEL);
        self.mass = mass;
        if let Some(b) = world.bodies.get_mut(self.body) {
            b.wake_up(true);
        }
    }

    /// Area of the collider in m².
    pub fn area(&self, world: &PhysWorld) -> f32 {
        world.colliders.get(self.collider).map_or(0.0, |c| c.shape().mass_properties(1.0).mass())
    }

    pub fn set_pinned(&mut self, world: &mut PhysWorld, pinned: bool) {
        self.pinned = pinned;
        if let Some(b) = world.bodies.get_mut(self.body) {
            let ty = if pinned { RigidBodyType::KinematicPositionBased } else { RigidBodyType::Dynamic };
            b.set_body_type(ty, true);
            b.set_linvel(vector![0.0, 0.0], true);
            b.set_angvel(0.0, true);
        }
    }

    pub fn contains_px(&self, world: &PhysWorld, px: f32, py: f32) -> bool {
        let (bx, by) = to_phys(px, py);
        world.colliders.get(self.collider).is_some_and(|c| c.shape().contains_point(c.position(), &point![bx, by]))
    }

    pub fn destroy(self, world: &mut PhysWorld) {
        world.remove_body(self.body);
    }
}

#[cfg(test)]
impl Object {
    /// An object without sprite, for physics tests.
    pub fn test_stub(body: RigidBodyHandle, collider: ColliderHandle) -> Self {
        Object {
            source: Source::Visualizer,
            frames: vec![],
            delays_ms: vec![],
            frame_idx: 0,
            frame_timer: 0.0,
            outline: Outline::Box,
            body,
            collider,
            size: vec2(60.0, 60.0),
            mass: 1.0,
            pinned: false,
            material: Material::default(),
            trail: VecDeque::new(),
        }
    }
}

/// Convex hull of a point cloud, counter-clockwise (monotone chain).
pub fn convex_hull(pts: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut p = pts.to_vec();
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: [f32; 2], a: [f32; 2], b: [f32; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let chain = |points: &mut dyn Iterator<Item = [f32; 2]>| {
        let mut h: Vec<[f32; 2]> = vec![];
        for q in points {
            while h.len() >= 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
        h
    };
    let mut hull = chain(&mut p.iter().copied());
    hull.extend(chain(&mut p.iter().rev().copied()));
    hull
}

/// Index of the top-most object under the given screen point.
pub fn object_at(objects: &[Object], world: &PhysWorld, px: f32, py: f32) -> Option<usize> {
    objects.iter().rposition(|o| o.contains_px(world, px, py))
}

/// Indices of every object under the given screen point, top-most first.
pub fn objects_at(objects: &[Object], world: &PhysWorld, px: f32, py: f32) -> Vec<usize> {
    (0..objects.len()).rev().filter(|&i| objects[i].contains_px(world, px, py)).collect()
}

#[cfg(test)]
mod hull_tests {
    #[test]
    fn hull_of_a_square_with_inner_points() {
        let pts = [[0.0, 0.0], [1.0, 0.0], [0.5, 0.5], [1.0, 1.0], [0.0, 1.0], [0.2, 0.7]];
        let h = super::convex_hull(&pts);
        assert_eq!(h, vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
    }
}
