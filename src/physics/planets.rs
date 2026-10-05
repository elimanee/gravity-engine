//! Planets: objects with a gravity of their own. Everything else falls
//! toward them, more weakly further away (inverse square), so things can
//! orbit a pinned sun or be thrown around a moon. Planets pull on each
//! other too.

use super::object::Object;
use super::PhysWorld;
use rapier2d::prelude::*;

/// Pull of one planet: where it is, its radius (m) and its surface gravity (m/s²).
#[derive(Clone, Copy, Debug)]
pub struct Well {
    pub body: RigidBodyHandle,
    pub centre: Vector<f32>,
    pub radius: f32,
    pub surface: f32,
}

/// Planets further than this many radii away are ignored.
const REACH_RADII: f32 = 60.0;

/// The planets among `objects`.
pub fn wells(world: &PhysWorld, objects: &[Object]) -> Vec<Well> {
    objects
        .iter()
        .filter(|o| o.material.planet > 0.0)
        .filter_map(|o| {
            let b = world.bodies.get(o.body)?;
            let radius = (o.size.x.min(o.size.y) / 2.0 / crate::config::PPM).max(0.1);
            Some(Well { body: o.body, centre: b.center_of_mass().coords, radius, surface: o.material.planet })
        })
        .collect()
}

/// Acceleration (m/s²) the planets give a body at `p` (not counting `skip`).
pub fn accel_at(wells: &[Well], p: Vector<f32>, skip: RigidBodyHandle) -> Vector<f32> {
    let mut a = vector![0.0, 0.0];
    for w in wells.iter().filter(|w| w.body != skip) {
        let d = w.centre - p;
        let r = d.norm();
        if r < 1e-4 || r > w.radius * REACH_RADII {
            continue;
        }
        let r_eff = r.max(w.radius);
        a += d / r * w.surface * (w.radius / r_eff).powi(2);
    }
    a
}

/// Pull every dynamic body in `bodies` toward the planets.
pub fn apply(world: &mut PhysWorld, wells: &[Well], bodies: &[RigidBodyHandle]) {
    if wells.is_empty() {
        return;
    }
    for &h in bodies {
        let Some(b) = world.bodies.get_mut(h).filter(|b| b.is_dynamic()) else { continue };
        let a = accel_at(wells, b.center_of_mass().coords, h);
        if a != vector![0.0, 0.0] {
            let m = b.mass();
            b.add_force(a * m, true);
        }
    }
}

/// Speed (m/s) of a circular orbit at `r` m from the centre of a planet.
pub fn orbit_speed(surface: f32, radius: f32, r: f32) -> f32 {
    (surface * radius * radius / r.max(radius)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;

    #[test]
    fn a_moon_stays_in_orbit() {
        let mut w = PhysWorld::new(0.0, BorderMode::Portal, (2400.0, 2400.0));
        let sun = w.bodies.insert(RigidBodyBuilder::fixed().translation(vector![20.0, 20.0]));
        let (surface, radius, r) = (10.0, 1.0, 5.0);
        let v = orbit_speed(surface, radius, r);
        let moon =
            w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![20.0 + r, 20.0]).linvel(vector![0.0, v]));
        w.colliders.insert_with_parent(ColliderBuilder::ball(0.2), moon, &mut w.bodies);
        let wells = [Well { body: sun, centre: vector![20.0, 20.0], radius, surface }];
        // About one turn.
        let period = std::f32::consts::TAU * r / v;
        let steps = (period / crate::config::PHYSICS_DT) as usize;
        let (mut lo, mut hi) = (f32::MAX, 0.0f32);
        for _ in 0..steps {
            w.reset_forces();
            apply(&mut w, &wells, &[moon]);
            w.step_fixed();
            let d = (w.bodies[moon].translation() - vector![20.0, 20.0]).norm();
            lo = lo.min(d);
            hi = hi.max(d);
        }
        assert!(lo > 4.6 && hi < 5.4, "stays about 5 m away: {lo}..{hi}");
        let p = w.bodies[moon].translation();
        assert!((p - vector![25.0, 20.0]).norm() < 1.5, "back near where it started: {p:?}");
    }

    #[test]
    fn pull_falls_off_with_distance() {
        let h = RigidBodyHandle::from_raw_parts(1, 0);
        let wells = [Well { body: h, centre: vector![0.0, 0.0], radius: 1.0, surface: 8.0 }];
        let other = RigidBodyHandle::from_raw_parts(2, 0);
        let near = accel_at(&wells, vector![1.0, 0.0], other);
        let far = accel_at(&wells, vector![2.0, 0.0], other);
        assert!((near.x + 8.0).abs() < 1e-4 && (far.x + 2.0).abs() < 1e-4);
        assert_eq!(accel_at(&wells, vector![1.0, 0.0], h), vector![0.0, 0.0], "not on itself");
    }
}
