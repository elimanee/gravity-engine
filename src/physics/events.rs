//! Hooks into the rapier pipeline: collision impacts (for particles and
//! fracture) and conveyor surfaces.

use rapier2d::prelude::*;
use std::sync::Mutex;

/// Contact forces below `mass × IMPACT_ACCEL` are not reported, so objects
/// resting on each other stay quiet.
pub const IMPACT_ACCEL: f32 = 45.0;

/// A strong contact during one physics step.
#[derive(Debug, Clone, Copy)]
pub struct Impact {
    pub collider1: ColliderHandle,
    pub collider2: ColliderHandle,
    /// World-space contact point (m).
    pub point: Point<f32>,
    /// Contact normal (from collider 1 toward collider 2).
    pub normal: Vector<f32>,
    /// Impulse exchanged during the step (N·s).
    pub impulse: f32,
}

/// Collects impacts reported by the pipeline.
#[derive(Default)]
pub struct Events {
    impacts: Mutex<Vec<Impact>>,
}

impl Events {
    pub fn drain(&self) -> Vec<Impact> {
        self.impacts.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }
}

impl EventHandler for Events {
    fn handle_collision_event(&self, _: &RigidBodySet, _: &ColliderSet, _: CollisionEvent, _: Option<&ContactPair>) {}

    fn handle_contact_force_event(
        &self,
        dt: f32,
        _: &RigidBodySet,
        _: &ColliderSet,
        pair: &ContactPair,
        total_force_magnitude: f32,
    ) {
        let Some((manifold, contact)) =
            pair.manifolds.iter().find_map(|m| m.data.solver_contacts.first().map(|c| (m, c)))
        else {
            return;
        };
        let impact = Impact {
            collider1: pair.collider1,
            collider2: pair.collider2,
            point: contact.point,
            normal: manifold.data.normal,
            impulse: total_force_magnitude * dt,
        };
        if let Ok(mut v) = self.impacts.lock() {
            // Bounded: a pile-up must not grow this without limit between frames.
            if v.len() < 512 {
                v.push(impact);
            }
        }
    }
}

/// Conveyor speed (m/s) stored in a collider's user data.
pub fn conveyor_speed(c: &Collider) -> f32 {
    f32::from_bits(c.user_data as u32)
}

pub fn set_conveyor_speed(c: &mut Collider, speed: f32) {
    c.user_data = speed.to_bits() as u128;
    let hooks = if speed != 0.0 { ActiveHooks::MODIFY_SOLVER_CONTACTS } else { ActiveHooks::empty() };
    c.set_active_hooks(hooks);
}

/// Makes conveyor surfaces drag what touches them along.
pub struct Hooks;

impl PhysicsHooks for Hooks {
    fn modify_solver_contacts(&self, ctx: &mut ContactModificationContext) {
        let speed = |h: ColliderHandle| ctx.colliders.get(h).map_or(0.0, conveyor_speed);
        // The belt moves along the surface tangent; the sign depends on which
        // side of the pair the conveyor is.
        let (s1, s2) = (speed(ctx.collider1), speed(ctx.collider2));
        let n = *ctx.normal;
        let tangent = vector![n.y, -n.x];
        let v = tangent * (s1 - s2);
        for c in ctx.solver_contacts.iter_mut() {
            c.tangent_velocity = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;
    use crate::physics::PhysWorld;

    /// Horizontal speed of a box resting on a pinned conveyor plank of `speed`.
    fn carried(speed: f32) -> f32 {
        let mut w = PhysWorld::new(-9.8, BorderMode::Portal, (1200.0, 1200.0));
        let belt = w.bodies.insert(RigidBodyBuilder::kinematic_position_based().translation(vector![10.0, 5.0]));
        let mut col = ColliderBuilder::cuboid(4.0, 0.2).friction(1.0).build();
        set_conveyor_speed(&mut col, speed);
        w.colliders.insert_with_parent(col, belt, &mut w.bodies);
        let b = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![10.0, 5.7]));
        w.colliders.insert_with_parent(ColliderBuilder::cuboid(0.4, 0.4).friction(1.0), b, &mut w.bodies);
        for _ in 0..90 {
            w.step_fixed();
        }
        w.bodies[b].linvel().x
    }

    #[test]
    fn conveyors_carry_objects_clockwise() {
        // Clockwise on screen: the top of the belt moves right.
        let v = carried(2.0);
        assert!((v - 2.0).abs() < 0.4, "expected ≈2 m/s to the right, got {v}");
        assert!(carried(-2.0) < -1.5);
        assert!(carried(0.0).abs() < 0.05);
    }
}
