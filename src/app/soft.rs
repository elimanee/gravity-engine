//! Jelly and cloth in the app: turning objects into them (or making new
//! ones), wind, tearing, burning, grabbing and drawing.

use super::fire::{point_gap_px, LIGHTER_REACH};
use super::*;
use crate::audio::sfx::Sound;
use crate::physics::soft::{self, SoftKind, SoftState};
use crate::physics::zones::ZoneKind;
use rapier2d::prelude::{nalgebra, vector};

/// A cloth knot burns this long before it gives way (s)…
const KNOT_BURN_SECS: f32 = 1.4;
/// …and sets its neighbours alight after this long.
const KNOT_SPREAD_SECS: f32 = 0.5;
/// Default cloth size (px).
const CLOTH_SIZE: Vec2 = vec2(220.0, 150.0);

impl App {
    /// `U`: the object under the pointer turns to jelly, or a new blob of
    /// the spawner's shape (in a random colour) drops there.
    pub(super) fn jelly_at(&mut self, at: Vec2) {
        if let Some(i) = self.object_for_soft(at) {
            self.record("Make jelly");
            self.make_jelly(i);
            return;
        }
        self.record("Add jelly");
        let c = spawn_color(shapes::PALETTE.len(), rand::gen_range(0.0, 400.0));
        let rgb = ((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
        let size = (self.s.spawn_size * 1.4).max(80.0);
        self.spawn_shape(self.s.spawn_shape, rgb, size, at, false, Material::DEFAULT);
        self.make_jelly(self.objects.len() - 1);
    }

    /// `Shift+U`: the object under the pointer is hung as a flag, or a new
    /// piece of cloth (in a random colour) is hung with the middle of its
    /// top edge at the pointer.
    pub(super) fn cloth_at(&mut self, at: Vec2) {
        if let Some(i) = self.object_for_soft(at) {
            self.record("Make flag");
            self.make_flag(i);
            return;
        }
        self.record("Add cloth");
        let c = spawn_color(shapes::PALETTE.len(), rand::gen_range(0.0, 400.0));
        let rgb = ((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
        let img = soft::fabric(rgb, 128, 96);
        let st = SoftState::cloth("Cloth".into(), Some(upload(&img)), at - vec2(CLOTH_SIZE.x / 2.0, 0.0), CLOTH_SIZE);
        let s = st.restore(&mut self.world, vector![0.0, 0.0]);
        self.softs.push(s);
        self.sound(Sound::Pop, at, 0.3);
    }

    fn object_for_soft(&self, at: Vec2) -> Option<usize> {
        object_at(&self.objects, &self.world, at.x, at.y).filter(|&i| !self.objects[i].is_visualizer())
    }

    /// Replace object `i` by a jelly of the same look.
    pub(super) fn make_jelly(&mut self, i: usize) {
        let o = &self.objects[i];
        if o.is_visualizer() {
            self.toasts.info("The visualizer screen cannot turn to jelly");
            return;
        }
        let Some(b) = self.world.bodies.get(o.body) else { return };
        let (centre, angle, vel) = (*b.translation(), b.rotation().angle(), *b.linvel());
        let texture = Some(o.texture().clone());
        let Some(st) = SoftState::jelly(o.name(), texture, &o.outline_polygon(), o.size, centre, angle, vel) else {
            return;
        };
        let (at, _) = o.screen_pos(&self.world);
        self.remove_object(i);
        let s = st.restore(&mut self.world, vector![0.0, 0.0]);
        self.softs.push(s);
        self.sound(Sound::Pop, at, 0.4);
    }

    /// Replace object `i` by a cloth of its picture, pinned along its top.
    pub(super) fn make_flag(&mut self, i: usize) {
        let o = &self.objects[i];
        if o.is_visualizer() {
            self.toasts.info("The visualizer screen cannot be hung as a flag");
            return;
        }
        let (pos, _) = o.screen_pos(&self.world);
        let size = o.size.max(vec2(60.0, 40.0)).min(vec2(460.0, 460.0));
        let st = SoftState::cloth(o.name(), Some(o.texture().clone()), pos - size / 2.0, size);
        self.remove_object(i);
        let s = st.restore(&mut self.world, vector![0.0, 0.0]);
        self.softs.push(s);
        self.sound(Sound::Snap, pos, 0.4);
    }

    /// Every jelly and cloth ball.
    pub(super) fn soft_bodies(&self) -> impl Iterator<Item = RigidBodyHandle> + '_ {
        self.softs.iter().flat_map(|s| s.balls.iter().copied())
    }

    /// Forces for this frame: jelly springiness, and wind (cloth ripples in it).
    pub(super) fn soft_forces(&mut self) {
        for s in &self.softs {
            s.apply_forces(&mut self.world);
        }
        // Wind and float zones (not portals: a single ball cannot go through).
        let calm: Vec<Zone> = self.zones.iter().filter(|z| z.kind != ZoneKind::Portal).cloned().collect();
        if calm.is_empty() {
            return;
        }
        let balls: Vec<RigidBodyHandle> = self.soft_bodies().collect();
        zones::apply(&calm, &mut PortalState::default(), &mut self.world, &balls);
        let t = get_time() as f32;
        for s in self.softs.iter().filter(|s| s.kind == SoftKind::Cloth) {
            for (k, &h) in s.balls.iter().enumerate() {
                let Some(b) = self.world.bodies.get_mut(h).filter(|b| b.is_dynamic()) else { continue };
                let p = *b.translation();
                for z in calm.iter().filter(|z| z.kind == ZoneKind::Wind && z.contains(p)) {
                    // Gusts rolling along the cloth.
                    let wave = (t * 7.0 - k as f32 * 0.45).sin() + 0.5 * (t * 3.1 + k as f32 * 0.9).sin();
                    let (along, across) =
                        (vector![z.angle.cos(), z.angle.sin()], vector![-z.angle.sin(), z.angle.cos()]);
                    let f = (along * wave * 0.5 + across * wave * 0.6) * z.strength * b.mass();
                    b.add_force(f, true);
                }
            }
        }
    }

    /// After the physics step: threads pulled too far snap, lost soft
    /// bodies are removed, cloth burns.
    pub(super) fn soft_after_step(&mut self, dt: f32, mouse: Vec2) {
        let mut snapped = vec![];
        for s in &mut self.softs {
            snapped.extend(s.tear(&mut self.world));
        }
        if let Some(&at) = snapped.first() {
            self.sound(Sound::Snap, at, 0.4);
            if self.s.effects {
                self.effects.dust(at, 4.0);
            }
        }
        self.burn_cloth(dt, mouse);
        // Far outside the world, or burnt away: gone.
        let (aw, ah) = self.arena();
        let world = &self.world;
        let (keep, gone): (Vec<_>, Vec<_>) = std::mem::take(&mut self.softs).into_iter().partition(|s| {
            let c = s.centre(world);
            let inside = c.x > -400.0 && c.x < aw + 400.0 && c.y > -2000.0 && c.y < ah + 400.0;
            inside && (s.kind == SoftKind::Jelly || s.has_cloth_left())
        });
        self.softs = keep;
        for s in gone {
            s.remove(&mut self.world);
        }
    }

    /// Cloth catches fire from the Fire tool and burning objects, the fire
    /// creeps along it, and burnt knots leave holes.
    fn burn_cloth(&mut self, dt: f32, mouse: Vec2) {
        let lighter = (self.fire.lit && self.s.tool == Tool::Fire).then_some(mouse);
        let burning: Vec<usize> =
            (0..self.objects.len()).filter(|&i| self.fire.burning(self.objects[i].body)).collect();
        let mut flames = vec![];
        let mut ignited = None;
        for s in self.softs.iter_mut().filter(|s| s.kind == SoftKind::Cloth) {
            let pts = s.points(&self.world);
            let mut light = vec![];
            for (k, p) in pts.iter().enumerate() {
                if s.burn[k] != 0.0 {
                    continue;
                }
                let by_tool = lighter.is_some_and(|m| m.distance(*p) < LIGHTER_REACH + 4.0);
                let by_fire =
                    burning.iter().any(|&i| point_gap_px(&self.world, &self.objects[i], *p).is_some_and(|d| d < 10.0));
                if by_tool || by_fire {
                    light.push(k);
                }
            }
            for (k, &p) in pts.iter().enumerate() {
                if s.burn[k] <= 0.0 {
                    continue;
                }
                s.burn[k] += dt;
                if rand::gen_range(0.0, 1.0) < dt * 12.0 {
                    flames.push(p + vec2(rand::gen_range(-7.0, 7.0), rand::gen_range(-7.0, 7.0)));
                }
                if s.burn[k] > KNOT_SPREAD_SECS && rand::gen_range(0.0, 1.0) < dt * 2.5 {
                    light.extend(s.neighbours(k).into_iter().filter(|&n| s.burn[n] == 0.0));
                }
                if s.burn[k] > KNOT_BURN_SECS {
                    s.burn[k] = -1.0;
                    s.cut_ball(&mut self.world, k);
                }
            }
            for k in light {
                if s.burn[k] == 0.0 {
                    s.burn[k] = 1e-3;
                    ignited = Some(pts[k]);
                }
            }
        }
        for p in flames {
            self.effects.flame(p, 9.0);
            if self.s.effects && rand::gen_range(0, 6) == 0 {
                self.effects.smoke(p, 10.0, 0.0);
            }
        }
        if let Some(at) = ignited {
            if rand::gen_range(0, 4) == 0 {
                self.sound(Sound::Crackle, at, 0.3);
            }
        }
    }

    /// The Spring / Slingshot / Swing tools hold the ball under the pointer.
    pub(super) fn grab_soft(&mut self, m: Vec2, tool: Tool) -> bool {
        for s in self.softs.iter().rev() {
            let reach = if s.contains(&self.world, m) { f32::MAX } else { s.radius_px() + 8.0 };
            if let Some(k) = s.nearest(&self.world, m, reach) {
                self.grab = Some(Grab::new(&self.world.bodies, s.balls[k], to_phys(m.x, m.y), tool));
                return true;
            }
        }
        false
    }

    /// Index of the soft body under the pointer.
    pub(super) fn soft_at(&self, m: Vec2) -> Option<usize> {
        self.softs.iter().rposition(|s| s.contains(&self.world, m))
    }

    pub(super) fn remove_soft(&mut self, i: usize) {
        let s = self.softs.remove(i);
        if self.grab.as_ref().is_some_and(|g| s.balls.contains(&g.body)) {
            self.grab = None;
        }
        s.remove(&mut self.world);
    }

    /// The knife cuts the cloth threads it crosses.
    pub(super) fn knife_cloth(&mut self, from: Vec2, to: Vec2) -> bool {
        let mut cut = false;
        for s in self.softs.iter_mut().filter(|s| s.kind == SoftKind::Cloth) {
            let crossed: Vec<usize> = (0..s.edges.len())
                .filter(|&e| {
                    s.edge_ends(&self.world, e).is_some_and(|(a, b)| super::knife::segments_cross(from, to, a, b))
                })
                .collect();
            for e in crossed {
                s.cut(&mut self.world, e);
                cut = true;
            }
        }
        cut
    }
}

fn upload(img: &image::RgbaImage) -> Texture2D {
    let t = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, img);
    t.set_filter(FilterMode::Linear);
    t
}
