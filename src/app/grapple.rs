//! Grappling hooks. Held on an arrow key, the hook shoots at the pointer,
//! hooks onto what it hits with a rope and reels in; letting go of the key
//! lets go of the hook. One that always works (or on the beat) shoots
//! along its aim and hangs on at the length it caught.

use super::*;
use crate::audio::sfx::Sound;
use crate::physics::gadgets::GadgetKind;
use crate::physics::to_screen;
use rapier2d::prelude::{Collider, QueryFilter};

/// How far the hook flies (px).
const REACH_PX: f32 = 900.0;
/// How long a shot that missed shows (s).
const MISS_SHOW: f32 = 0.3;
/// Reeling speed (m/s) and the shortest rope (m).
const REEL_SPEED: f32 = 3.2;
const MIN_ROPE: f32 = 0.4;
/// Seconds before a hook that missed is shot again.
const RETRY: f32 = 0.5;

impl App {
    /// Shoot, reel in or let go of each grappling hook (before the step).
    pub(super) fn run_grapples(&mut self, dt: f32, mouse: Vec2) {
        for i in 0..self.gadgets.len() {
            let g = &mut self.gadgets[i];
            if g.spec.kind != GadgetKind::Grapple {
                continue;
            }
            if let Some((_, age)) = &mut g.miss {
                *age += dt;
                if *age > MISS_SHOW {
                    g.miss = None;
                }
            }
            if g.hook.as_ref().is_some_and(|l| !self.world.impulse_joints.contains(l.joint)) {
                g.hook = None;
            }
            if !g.on {
                if let Some(l) = g.hook.take() {
                    l.remove(&mut self.world);
                }
                g.cooldown = 0.0;
                continue;
            }
            let reels = g.spec.trigger.arrow().is_some() || g.spec.trigger == crate::physics::gadgets::Trigger::Beat;
            match &mut g.hook {
                Some(l) if reels && l.length > MIN_ROPE => {
                    let len = (l.length - REEL_SPEED * dt).max(MIN_ROPE);
                    l.set_rope_length(&mut self.world, len);
                }
                Some(_) => {}
                None => {
                    g.cooldown -= dt;
                    if g.cooldown <= 0.0 {
                        g.cooldown = RETRY;
                        self.shoot_hook(i, mouse);
                    }
                }
            }
        }
    }

    /// Shoot gadget `i`'s hook: at the pointer when it is held on a key,
    /// otherwise along its aim.
    fn shoot_hook(&mut self, i: usize, mouse: Vec2) {
        let g = &self.gadgets[i];
        let Some((p, d)) = g.pose(&self.world) else { return };
        let aim = if g.spec.trigger.arrow().is_some() {
            let (mx, my) = to_phys(mouse.x, mouse.y);
            (Point::new(mx, my) - p).try_normalize(1e-4).unwrap_or(d)
        } else {
            d
        };
        let host = g.host;
        let grains: HashSet<RigidBodyHandle> = self.grains.bodies().collect();
        let not_grain = |_, c: &Collider| c.parent().is_none_or(|b| !grains.contains(&b));
        let mut filter = QueryFilter::default().exclude_sensors().predicate(&not_grain);
        filter.exclude_rigid_body = host;
        let Some((c, toi, _)) = self.world.cast_ray(p, aim, REACH_PX / PPM, filter) else {
            let end = p + aim * (REACH_PX / PPM);
            self.gadgets[i].miss = Some((to_screen(end.x, end.y), 0.0));
            self.sound(Sound::Slice, to_screen(p.x, p.y), 0.25);
            return;
        };
        let q = p + aim * toi;
        let target = self.world.colliders.get(c).and_then(|c| c.parent());
        let dynamic = |h: RigidBodyHandle| self.world.bodies.get(h).is_some_and(|b| b.is_dynamic());
        let made = match host {
            Some(h) => Link::new(&mut self.world, LinkKind::Rope, h, target, p, q, 0.0),
            // Fixed in the world: a winch that pulls what it hooks.
            None => match target.filter(|&t| dynamic(t)) {
                Some(t) => Link::new(&mut self.world, LinkKind::Rope, t, None, q, p, 0.0),
                None => None,
            },
        };
        let Some(link) = made else { return };
        if let Some(b) = host.and_then(|h| self.world.bodies.get_mut(h)) {
            b.wake_up(true);
        }
        self.gadgets[i].hook = Some(link);
        let at = to_screen(q.x, q.y);
        self.sound(Sound::Snap, at, 0.6);
        if self.s.effects {
            self.effects.sparks(at, vec2(-aim.x, aim.y), 6.0);
        }
    }

    /// The knife cuts hook ropes it crosses. Returns whether it cut one.
    pub(super) fn knife_hooks(&mut self, from: Vec2, to: Vec2) -> bool {
        let mut cut = false;
        for g in &mut self.gadgets {
            let Some(l) = &g.hook else { continue };
            let Some((pa, pb)) = l.ends(&self.world) else { continue };
            if super::knife::segments_cross(from, to, to_screen(pa.x, pa.y), to_screen(pb.x, pb.y)) {
                if let Some(l) = g.hook.take() {
                    l.remove(&mut self.world);
                }
                // Not shot again until the key is pressed anew.
                g.cooldown = f32::INFINITY;
                cut = true;
            }
        }
        cut
    }

    /// Hook ropes and hooks (world pass).
    pub(super) fn draw_hooks(&self) {
        for g in &self.gadgets {
            if let (Some((end, age)), Some((p, _))) = (g.miss, g.pose(&self.world)) {
                // The hook flies out and drops back.
                let from = to_screen(p.x, p.y);
                let k = (age / MISS_SHOW).clamp(0.0, 1.0);
                let tip = from.lerp(end, 1.0 - (2.0 * k - 1.0).powi(2));
                let c = Color::new(0.85, 0.75, 0.55, 1.0 - k * 0.5);
                draw_line(from.x, from.y, tip.x, tip.y, 1.5, c);
                let dir = (end - from).try_normalize().unwrap_or(vec2(0.0, -1.0));
                crate::physics::gadgets::draw_hook(tip, dir, Color::from_rgba(200, 210, 220, 255));
            }
            let Some(l) = &g.hook else { continue };
            l.draw(&self.world);
            let Some((pa, pb)) = l.ends(&self.world) else { continue };
            // The hook sits at the far end from the gadget.
            let (from, hook) = if g.host.is_some() { (pa, pb) } else { (pb, pa) };
            let (a, b) = (to_screen(from.x, from.y), to_screen(hook.x, hook.y));
            let dir = (b - a).try_normalize().unwrap_or(vec2(0.0, -1.0));
            crate::physics::gadgets::draw_hook(b, dir, Color::from_rgba(200, 210, 220, 255));
        }
    }
}
