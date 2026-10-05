//! Rapier world wrapper, coordinate conversion and the arena walls.

pub mod borders;
pub mod events;
pub mod fracture;
pub mod gadgets;
pub mod grains;
pub mod links;
pub mod magnets;
pub mod object;
pub mod soft;
pub mod tools;
pub mod water;
pub mod zones;

use crate::config::{BOUNCE, FRICTION, PHYSICS_DT, PPM, WALL_T};
use borders::BorderMode;
use macroquad::prelude::{vec2, Vec2};
use rapier2d::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// Height of the world in pixels (720 until a world is created).
static WORLD_H: AtomicU32 = AtomicU32::new(0x4434_0000);

fn world_height() -> f32 {
    f32::from_bits(WORLD_H.load(Ordering::Relaxed))
}

/// World pixels (y down, see `camera`) → physics metres (y up).
pub fn to_phys(px: f32, py: f32) -> (f32, f32) {
    (px / PPM, (world_height() - py) / PPM)
}

/// Physics metres → world pixels (y down).
pub fn to_screen(bx: f32, by: f32) -> Vec2 {
    vec2(bx * PPM, world_height() - by * PPM)
}

pub struct PhysWorld {
    pub bodies: RigidBodySet,
    pub colliders: ColliderSet,
    pub gravity: Vector<f32>,
    params: IntegrationParameters,
    pipeline: PhysicsPipeline,
    pub islands: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    pub impulse_joints: ImpulseJointSet,
    pub multibody_joints: MultibodyJointSet,
    ccd: CCDSolver,
    query: QueryPipeline,
    events: events::Events,
    walls: Vec<RigidBodyHandle>,
    /// Fixed body at the origin that links attach to when they are pinned to
    /// the background.
    pub ground: RigidBodyHandle,
    pub border: BorderMode,
    /// Arena size in metres.
    pub arena: (f32, f32),
}

impl PhysWorld {
    pub fn new(gravity_y: f32, border: BorderMode, arena_px: (f32, f32)) -> Self {
        let mut bodies = RigidBodySet::new();
        let ground = bodies.insert(RigidBodyBuilder::fixed());
        let mut w = PhysWorld {
            bodies,
            colliders: ColliderSet::new(),
            gravity: vector![0.0, gravity_y],
            params: IntegrationParameters::default(),
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: DefaultBroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd: CCDSolver::new(),
            query: QueryPipeline::new(),
            events: events::Events::default(),
            walls: vec![],
            ground,
            border,
            arena: (arena_px.0 / PPM, arena_px.1 / PPM),
        };
        WORLD_H.store(arena_px.1.to_bits(), Ordering::Relaxed);
        w.rebuild_walls();
        w
    }

    pub fn resize(&mut self, arena_px: (f32, f32)) {
        self.arena = (arena_px.0 / PPM, arena_px.1 / PPM);
        WORLD_H.store(arena_px.1.to_bits(), Ordering::Relaxed);
        self.rebuild_walls();
    }

    pub fn set_border(&mut self, mode: BorderMode) {
        self.border = mode;
        self.rebuild_walls();
        self.wake_all();
    }

    fn rebuild_walls(&mut self) {
        for h in std::mem::take(&mut self.walls) {
            self.remove_body(h);
        }
        let (sw, sh) = self.arena;
        let (hw, hh) = (sw / 2.0, sh / 2.0);
        let spec = self.border.walls();
        let t = WALL_T / 2.0;

        let mut walls = vec![];
        if spec.floor {
            walls.push((vector![hw, t], vector![hw + WALL_T, t]));
        }
        if spec.ceiling {
            walls.push((vector![hw, sh + t], vector![hw + WALL_T, t]));
        }
        if spec.sides {
            walls.push((vector![t, hh], vector![t, hh + WALL_T]));
            walls.push((vector![sw - t, hh], vector![t, hh + WALL_T]));
        }
        for (pos, half) in walls {
            let bh = self.bodies.insert(RigidBodyBuilder::fixed().translation(pos));
            let col = ColliderBuilder::cuboid(half.x, half.y).restitution(BOUNCE * 0.6).friction(FRICTION);
            self.colliders.insert_with_parent(col, bh, &mut self.bodies);
            self.walls.push(bh);
        }
    }

    pub fn set_gravity(&mut self, gy: f32) {
        self.gravity = vector![0.0, gy];
        self.wake_all();
    }

    pub fn wake_all(&mut self) {
        for (_, b) in self.bodies.iter_mut() {
            if b.is_dynamic() {
                b.wake_up(true);
            }
        }
    }

    /// Advance the simulation by one rendered frame. The frame is split into
    /// equal sub-steps no longer than `PHYSICS_DT`, so the simulation speed no
    /// longer depends on the monitor refresh rate.
    pub fn step_frame(&mut self, frame_dt: f32, time_scale: f32) {
        let total = frame_dt.min(1.0 / 20.0) * time_scale;
        if total <= 0.0 {
            return;
        }
        let n = ((total / PHYSICS_DT).ceil() as usize).clamp(1, 8);
        self.params.dt = total / n as f32;
        for _ in 0..n {
            self.step_once();
        }
    }

    /// One fixed step (used for frame-by-frame stepping while paused).
    pub fn step_fixed(&mut self) {
        self.params.dt = PHYSICS_DT;
        self.step_once();
    }

    fn step_once(&mut self) {
        self.pipeline.step(
            &self.gravity,
            &self.params,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd,
            Some(&mut self.query),
            &events::Hooks,
            &self.events,
        );
    }

    /// The first collider along a ray from `from` in direction `dir` (unit),
    /// within `max` m: its handle, the distance and the surface normal.
    pub fn cast_ray(
        &self,
        from: Point<f32>,
        dir: Vector<f32>,
        max: f32,
        filter: QueryFilter,
    ) -> Option<(ColliderHandle, f32, Vector<f32>)> {
        let ray = Ray::new(from, dir);
        self.query
            .cast_ray_and_get_normal(&self.bodies, &self.colliders, &ray, max, true, filter)
            .map(|(c, hit)| (c, hit.time_of_impact, hit.normal))
    }

    /// Bring ray casts up to date without stepping (while paused).
    pub fn refresh_queries(&mut self) {
        self.query.update(&self.colliders);
    }

    /// Impacts reported since the last call.
    pub fn take_impacts(&self) -> Vec<events::Impact> {
        self.events.drain()
    }

    pub fn remove_body(&mut self, h: RigidBodyHandle) {
        self.bodies.remove(
            h,
            &mut self.islands,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            true,
        );
    }

    pub fn remove_collider(&mut self, h: ColliderHandle) {
        self.colliders.remove(h, &mut self.islands, &mut self.bodies, true);
    }

    /// Clear user forces and torques on every body (called once per frame
    /// before tools add theirs). Rapier keeps both until they are reset, and
    /// forces applied off-centre add a torque.
    pub fn reset_forces(&mut self) {
        for (_, b) in self.bodies.iter_mut() {
            if b.user_force() != vector![0.0, 0.0] || b.user_torque() != 0.0 {
                b.reset_forces(false);
                b.reset_torques(false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_centre_forces_do_not_keep_turning_things() {
        let mut w = PhysWorld::new(0.0, BorderMode::Portal, (1200.0, 1200.0));
        let h = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![10.0, 10.0]));
        w.colliders.insert_with_parent(ColliderBuilder::cuboid(0.5, 0.5), h, &mut w.bodies);
        w.bodies[h].add_force_at_point(vector![1.0, 0.0], point![10.0, 10.4], true);
        w.reset_forces();
        assert_eq!(w.bodies[h].user_force(), vector![0.0, 0.0]);
        assert_eq!(w.bodies[h].user_torque(), 0.0, "the torque goes too");
    }
}
