//! Magnets: objects with a magnet strength pull each other together (or push
//! each other away when either one is negative).

use super::object::Object;
use super::PhysWorld;
use rapier2d::prelude::*;

/// Pull between two unit magnets 1 m apart, as an acceleration (m/s²).
const K: f32 = 60.0;
/// Magnets further apart than this (m) ignore each other.
const RANGE: f32 = 12.0;
/// Closer than this (m) the force stops growing.
const SOFTENING: f32 = 0.6;
/// Upper bound of the acceleration, so touching magnets do not explode.
const MAX_ACCEL: f32 = 90.0;

pub fn apply(world: &mut PhysWorld, objects: &[Object]) {
    let magnets: Vec<(RigidBodyHandle, f32)> =
        objects.iter().filter(|o| o.material.magnet != 0.0).map(|o| (o.body, o.material.magnet)).collect();
    for i in 0..magnets.len() {
        for j in i + 1..magnets.len() {
            let ((hi, qi), (hj, qj)) = (magnets[i], magnets[j]);
            let (Some(bi), Some(bj)) = (world.bodies.get(hi), world.bodies.get(hj)) else { continue };
            let d = bj.translation() - bi.translation();
            let r = d.norm();
            if r > RANGE || r < 1e-4 {
                continue;
            }
            let dir = d / r;
            let attract = qi > 0.0 && qj > 0.0;
            let accel = (K * (qi * qj).abs() / r.max(SOFTENING).powi(2)).min(MAX_ACCEL);
            let (di, dj) = (bi.is_dynamic(), bj.is_dynamic());
            let m = match (di, dj) {
                (true, true) => bi.mass() * bj.mass() / (bi.mass() + bj.mass()).max(1e-6),
                (true, false) => bi.mass(),
                (false, true) => bj.mass(),
                (false, false) => continue,
            };
            let f = dir * accel * m * if attract { 1.0 } else { -1.0 };
            if di {
                world.bodies[hi].add_force(f, true);
            }
            if dj {
                world.bodies[hj].add_force(-f, true);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;
    use crate::physics::object::Material;

    fn gap_after(qa: f32, qb: f32) -> f32 {
        let mut w = PhysWorld::new(0.0, BorderMode::Portal, (1200.0, 1200.0));
        let mut objs = vec![];
        for (x, q) in [(8.0, qa), (11.0, qb)] {
            let h = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![x, 10.0]));
            let c = w.colliders.insert_with_parent(ColliderBuilder::ball(0.3), h, &mut w.bodies);
            let mut o = Object::test_stub(h, c);
            o.material = Material { magnet: q, ..Material::DEFAULT };
            objs.push(o);
        }
        for _ in 0..30 {
            w.reset_forces();
            apply(&mut w, &objs);
            w.step_fixed();
        }
        w.bodies[objs[1].body].translation().x - w.bodies[objs[0].body].translation().x
    }

    #[test]
    fn magnets_attract_and_repel() {
        assert!(gap_after(1.0, 1.0) < 2.9, "two magnets pull together");
        assert!(gap_after(1.0, -1.0) > 3.1, "a negative magnet pushes away");
        assert!((gap_after(1.0, 0.0) - 3.0).abs() < 1e-3, "plain objects are not affected");
    }
}
