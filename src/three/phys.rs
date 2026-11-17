//! The 3D mode's physics: rapier3d with the ground (meshes and height
//! fields, fixed), things that fall, roll and get thrown (dynamic), and
//! rays for the character's sensors. Metres, y up, stepped at 60 Hz.

use macroquad::prelude::{vec3, Vec3};
#[cfg(test)]
use rapier3d::na::DMatrix;
use rapier3d::prelude::*;

pub struct Phys {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad: DefaultBroadPhase,
    narrow: NarrowPhase,
    joints: ImpulseJointSet,
    multibody: MultibodyJointSet,
    ccd: CCDSolver,
    query: QueryPipeline,
    params: IntegrationParameters,
    gravity: Vector<f32>,
}

/// What a ray found: where, the surface's normal, how far, and the body
/// if it is one that moves.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub point: Vec3,
    pub normal: Vec3,
    pub dist: f32,
    pub moving: Option<RigidBodyHandle>,
}

fn v(p: Vec3) -> Vector<f32> {
    vector![p.x, p.y, p.z]
}

fn to_vec3(p: &Vector<f32>) -> Vec3 {
    vec3(p.x, p.y, p.z)
}

/// The shapes things can be.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Solid {
    Box(Vec3),
    Ball(f32),
    Cylinder(f32, f32),
}

impl Default for Phys {
    fn default() -> Self {
        Self::new()
    }
}

impl Phys {
    pub fn new() -> Phys {
        let params = IntegrationParameters { dt: 1.0 / 60.0, ..Default::default() };
        Phys {
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad: DefaultBroadPhase::new(),
            narrow: NarrowPhase::new(),
            joints: ImpulseJointSet::new(),
            multibody: MultibodyJointSet::new(),
            ccd: CCDSolver::new(),
            query: QueryPipeline::new(),
            params,
            gravity: vector![0.0, -9.81 * 2.2, 0.0],
        }
    }

    /// A fixed mesh of triangles (the ground of a stretch of course).
    pub fn add_mesh(&mut self, points: &[Vec3], tris: &[[u32; 3]]) -> Option<ColliderHandle> {
        if tris.is_empty() {
            return None;
        }
        let pts: Vec<Point<f32>> = points.iter().map(|p| point![p.x, p.y, p.z]).collect();
        let shape = SharedShape::trimesh_with_flags(pts, tris.to_vec(), TriMeshFlags::FIX_INTERNAL_EDGES);
        let c = ColliderBuilder::new(shape).friction(0.8).build();
        Some(self.colliders.insert(c))
    }

    /// Ground over a grid of heights: `heights[z][x]`, `cell` metres apart,
    /// its corner at `origin`.
    #[cfg(test)]
    pub fn add_heights(&mut self, heights: &[Vec<f32>], cell: f32, origin: Vec3) -> Option<ColliderHandle> {
        let rows = heights.len();
        let cols = heights.first()?.len();
        if rows < 2 || cols < 2 {
            return None;
        }
        let m = DMatrix::from_fn(rows, cols, |r, c| heights[r][c]);
        let size = vector![(cols - 1) as f32 * cell, 1.0, (rows - 1) as f32 * cell];
        let centre = origin + vec3(size.x / 2.0, 0.0, size.z / 2.0);
        let c = ColliderBuilder::heightfield(m, size).translation(v(centre)).friction(0.8).build();
        Some(self.colliders.insert(c))
    }

    pub fn remove_collider(&mut self, c: ColliderHandle) {
        self.colliders.remove(c, &mut self.islands, &mut self.bodies, false);
    }

    /// Something that moves: a solid at `at`, going at `vel`.
    pub fn add_body(&mut self, solid: Solid, at: Vec3, vel: Vec3, density: f32) -> RigidBodyHandle {
        let body = RigidBodyBuilder::dynamic()
            .translation(v(at))
            .linvel(v(vel))
            .angvel(vector![vel.z * 0.3, 0.0, -vel.x * 0.3])
            .ccd_enabled(true)
            .build();
        let h = self.bodies.insert(body);
        let shape = match solid {
            Solid::Box(half) => ColliderBuilder::cuboid(half.x, half.y, half.z),
            Solid::Ball(r) => ColliderBuilder::ball(r),
            Solid::Cylinder(half, r) => ColliderBuilder::cylinder(half, r),
        };
        let c = shape.density(density).friction(0.7).restitution(0.25).build();
        self.colliders.insert_with_parent(c, h, &mut self.bodies);
        h
    }

    pub fn remove_body(&mut self, b: RigidBodyHandle) {
        self.bodies.remove(b, &mut self.islands, &mut self.colliders, &mut self.joints, &mut self.multibody, true);
    }

    /// Where a body is and how it is turned.
    pub fn pose(&self, b: RigidBodyHandle) -> Option<(Vec3, rapier3d::na::UnitQuaternion<f32>)> {
        let body = self.bodies.get(b)?;
        Some((to_vec3(body.translation()), *body.rotation()))
    }

    #[cfg(test)]
    pub fn velocity(&self, b: RigidBodyHandle) -> Vec3 {
        self.bodies.get(b).map_or(Vec3::ZERO, |b| to_vec3(b.linvel()))
    }

    /// A push (impulse, kg·m/s) on a body at a point.
    pub fn push(&mut self, b: RigidBodyHandle, impulse: Vec3, at: Vec3) {
        if let Some(body) = self.bodies.get_mut(b) {
            body.apply_impulse_at_point(v(impulse), point![at.x, at.y, at.z], true);
        }
    }

    /// A blast at `at`: everything within `radius` thrown away from it.
    pub fn blast(&mut self, at: Vec3, radius: f32, strength: f32) {
        let hits: Vec<RigidBodyHandle> = self
            .bodies
            .iter()
            .filter(|(_, b)| b.is_dynamic() && (to_vec3(b.translation()) - at).length() < radius)
            .map(|(h, _)| h)
            .collect();
        for h in hits {
            let Some(body) = self.bodies.get_mut(h) else { continue };
            let d = to_vec3(body.translation()) - at;
            let k = (1.0 - d.length() / radius).max(0.0);
            let dir = (d + Vec3::Y * 0.6).normalize_or_zero();
            let m = body.mass();
            body.apply_impulse(v(dir * strength * k * m), true);
            body.apply_torque_impulse(vector![k * m * 0.5, k * m, -k * m * 0.4], true);
        }
    }

    /// How many bodies are moving (not asleep).
    pub fn awake(&self) -> usize {
        self.islands.active_dynamic_bodies().len()
    }

    pub fn step(&mut self) {
        self.pipeline.step(
            &self.gravity,
            &self.params,
            &mut self.islands,
            &mut self.broad,
            &mut self.narrow,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.joints,
            &mut self.multibody,
            &mut self.ccd,
            Some(&mut self.query),
            &(),
            &(),
        );
    }

    /// Make rays see colliders added since the last step.
    pub fn refresh(&mut self) {
        self.query.update(&self.colliders);
    }

    /// The nearest thing along a ray from `from` in `dir` (normalised),
    /// up to `max` metres; rays starting inside something see its far side.
    pub fn ray(&self, from: Vec3, dir: Vec3, max: f32) -> Option<Hit> {
        let ray = Ray::new(point![from.x, from.y, from.z], v(dir));
        let (c, hit) = self.query.cast_ray_and_get_normal(
            &self.bodies,
            &self.colliders,
            &ray,
            max,
            false,
            QueryFilter::default().exclude_sensors(),
        )?;
        let moving = self
            .colliders
            .get(c)
            .and_then(|c| c.parent())
            .filter(|b| self.bodies.get(*b).is_some_and(|b| b.is_dynamic()));
        Some(Hit {
            point: from + dir * hit.time_of_impact,
            normal: to_vec3(&hit.normal).normalize_or_zero(),
            dist: hit.time_of_impact,
            moving,
        })
    }

    /// How many bodies move about.
    #[cfg(test)]
    pub fn moving(&self) -> usize {
        self.bodies.iter().filter(|(_, b)| b.is_dynamic()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rays_find_meshes_and_height_fields_and_things_fall_onto_them() {
        let mut p = Phys::new();
        // A floor at y = 0 from (-10, -10) to (10, 10).
        let pts = [vec3(-10.0, 0.0, -10.0), vec3(10.0, 0.0, -10.0), vec3(10.0, 0.0, 10.0), vec3(-10.0, 0.0, 10.0)];
        p.add_mesh(&pts, &[[0, 2, 1], [0, 3, 2]]).unwrap();
        // A slope of heights beside it: y = x / 10 for x from 20 to 40.
        let heights: Vec<Vec<f32>> = (0..5).map(|_| (0..5).map(|c| c as f32 * 0.5).collect()).collect();
        p.add_heights(&heights, 5.0, vec3(20.0, 0.0, -10.0)).unwrap();
        p.refresh();
        let hit = p.ray(vec3(1.0, 5.0, 2.0), -Vec3::Y, 10.0).unwrap();
        assert!((hit.point.y).abs() < 1e-4 && hit.normal.y > 0.99 && (hit.dist - 5.0).abs() < 1e-4);
        assert!(hit.moving.is_none());
        // On the height field: x = 30 is half way (0.5 × 2 cells = 1.0 high).
        let hit = p.ray(vec3(30.0, 10.0, 0.0), -Vec3::Y, 20.0).unwrap();
        assert!((hit.point.y - 1.0).abs() < 1e-3, "height {}", hit.point.y);
        assert!(hit.normal.x < -0.05 && hit.normal.y > 0.9, "slopes up to +x: {:?}", hit.normal);
        assert!(p.ray(vec3(15.0, 5.0, 0.0), -Vec3::Y, 10.0).is_none(), "nothing between them");
        // A box dropped falls and rests on the floor.
        let b = p.add_body(Solid::Box(Vec3::splat(0.5)), vec3(0.0, 4.0, 0.0), Vec3::ZERO, 1.0);
        for _ in 0..240 {
            p.step();
        }
        let (at, _) = p.pose(b).unwrap();
        assert!((at.y - 0.5).abs() < 0.05, "resting at {at:?}");
        let hit = p.ray(vec3(0.0, 5.0, 0.0), -Vec3::Y, 10.0).unwrap();
        assert_eq!(hit.moving, Some(b), "rays see it");
        // A blast throws it.
        p.blast(vec3(0.0, 0.2, -1.0), 5.0, 8.0);
        p.step();
        assert!(p.velocity(b).z > 1.0 && p.velocity(b).y > 0.5);
        assert_eq!(p.moving(), 1);
        p.remove_body(b);
        assert_eq!(p.moving(), 0);
    }
}
