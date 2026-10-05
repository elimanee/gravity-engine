//! The Gadget tool (lasers, thrusters, cannons) and what gadgets do each
//! frame, plus motors driven with the arrow keys.

use super::fire::point_gap_px;
use super::*;
use crate::audio::sfx::Sound;
use crate::physics::gadgets::{self, Ammo, Beam, Gadget, GadgetKind, GadgetSpec, Surface, Trigger};
use crate::physics::to_screen;
use rapier2d::prelude::{nalgebra, vector, ColliderHandle, Vector};

/// Heat per second a laser puts into what it hits (1 = ignition).
const LASER_HEAT: f32 = 0.8;
/// How hard a laser pushes what it hits (N).
const LASER_PUSH: f32 = 0.5;
/// Most objects one cannon keeps in the world; older shots vanish.
const MAX_SHOTS: usize = 40;
/// A beat pulse lasts this long (s).
const PULSE: f32 = 0.18;
/// Reach (px) and half-angle (rad) of a thruster fixed in the world (a fan).
const FAN_REACH: f32 = 320.0;
const FAN_SPREAD: f32 = 0.35;
/// Standard gravity, for thrust given in weights.
const G: f32 = 9.81;
/// Size of cannonballs (px).
const BALL_PX: f32 = 22.0;

/// A Gadget-tool drag: where it started and the object under it (with
/// the point in the object's frame).
pub(super) struct GadgetDrag {
    host: Option<(RigidBodyHandle, Point<f32>)>,
    start: Vec2,
}

/// Physics direction → screen direction.
fn screen_dir(d: Vector<f32>) -> Vec2 {
    vec2(d.x, -d.y)
}

impl App {
    pub(super) fn gadget_press(&mut self, m: Vec2) {
        let (x, y) = to_phys(m.x, m.y);
        let host =
            object_at(&self.objects, &self.world, m.x, m.y).filter(|&i| !self.objects[i].is_visualizer()).map(|i| {
                let b = self.objects[i].body;
                (b, self.world.bodies[b].position().inverse_transform_point(&Point::new(x, y)))
            });
        self.gadget_drag = Some(GadgetDrag { host, start: m });
    }

    /// Default aim (screen direction) when the pointer did not move.
    fn default_aim(kind: GadgetKind) -> Vec2 {
        match kind {
            GadgetKind::Laser => vec2(1.0, 0.0),
            GadgetKind::Thruster => vec2(0.0, -1.0),
            GadgetKind::Cannon => vec2(0.7, -0.7).normalize(),
        }
    }

    pub(super) fn gadget_release(&mut self, m: Vec2) {
        let Some(drag) = self.gadget_drag.take() else { return };
        let kind = self.s.gadget_kind;
        let aim = m - drag.start;
        let aim = if aim.length() > 12.0 { aim.normalize() } else { Self::default_aim(kind) };
        let world_angle = (-aim.y).atan2(aim.x);
        let host = drag.host.filter(|(b, _)| self.world.bodies.contains(*b));
        let (at, angle) = match host {
            Some((b, local)) => ([local.x, local.y], world_angle - self.world.bodies[b].rotation().angle()),
            None => {
                let (x, y) = to_phys(drag.start.x, drag.start.y);
                ([x, y], world_angle)
            }
        };
        let spec = GadgetSpec {
            kind,
            trigger: self.s.gadget_trigger,
            ammo: self.s.cannon_ammo,
            power: if kind == GadgetKind::Thruster { self.s.thrust } else { self.s.cannon_speed },
            rate: self.s.cannon_rate,
            at,
            angle,
        };
        self.record(kind.label());
        self.gadgets.push(Gadget::new(spec, host.map(|(b, _)| b)));
        self.sound(Sound::Snap, drag.start, 0.5);
        let what = match (kind, host.is_some()) {
            (GadgetKind::Thruster, false) => "Fan".to_string(),
            _ => kind.label().to_string(),
        };
        self.toasts
            .status("gadget", format!("{what} added  ·  works {}  ·  right-click it to remove", spec.trigger.label()));
    }

    /// Index of the gadget under the pointer.
    pub(super) fn gadget_at(&self, m: Vec2) -> Option<usize> {
        (0..self.gadgets.len())
            .filter_map(|i| {
                let (p, _) = self.gadgets[i].pose(&self.world)?;
                Some((i, to_screen(p.x, p.y).distance(m)))
            })
            .filter(|&(_, d)| d < 16.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    pub(super) fn remove_gadget(&mut self, i: usize) {
        self.record("Remove gadget");
        let g = self.gadgets.remove(i);
        self.toasts.status("gadget", format!("{} removed", g.spec.kind.label()));
    }

    /// Whether an arrow key (or anything else) makes `t` work right now.
    fn trigger_held(&self, t: Trigger) -> bool {
        let typing = self.editor_bar.typing();
        let key = |k| !typing && is_key_down(k);
        match t {
            Trigger::Always => true,
            Trigger::Up => key(KeyCode::Up),
            Trigger::Down => key(KeyCode::Down),
            Trigger::Left => key(KeyCode::Left),
            Trigger::Right => key(KeyCode::Right),
            Trigger::Beat => false,
        }
    }

    /// Something is driven with ← (so holding it does not rewind).
    pub(super) fn listens_to_left(&self) -> bool {
        self.gadgets.iter().any(|g| g.spec.trigger == Trigger::Left) || self.links.iter().any(|l| l.drive)
    }

    /// Once per frame before the physics step: which gadgets work, thrust,
    /// cannon shots, laser beams and driven motors. `dt` is the simulated
    /// time (0 when paused).
    pub(super) fn update_gadgets(&mut self, dt: f32, running: bool) {
        let world = &self.world;
        self.gadgets.retain(|g| g.host.is_none_or(|h| world.bodies.contains(h)));
        let beat = running && self.audio.analyzer.beat_now;
        for i in 0..self.gadgets.len() {
            let t = self.gadgets[i].spec.trigger;
            let held = self.trigger_held(t);
            let g = &mut self.gadgets[i];
            if t == Trigger::Beat {
                if beat {
                    g.pulse = PULSE;
                }
                g.pulse = (g.pulse - dt).max(0.0);
                g.on = g.pulse > 0.0;
            } else {
                g.on = held;
            }
        }
        if running {
            let dir = if self.trigger_held(Trigger::Right) {
                Some(1.0)
            } else if self.trigger_held(Trigger::Left) {
                Some(-1.0)
            } else {
                None
            };
            for l in &self.links {
                l.set_drive(&mut self.world, dir);
            }
            self.run_thrusters(dt);
            self.run_cannons(dt);
        }
        self.trace_beams(dt, running);
    }

    fn run_thrusters(&mut self, dt: f32) {
        let mut roaring = 0;
        let mut jets = vec![];
        let mut warm = vec![];
        let fans: Vec<(Point<f32>, Vector<f32>, f32)> = self
            .gadgets
            .iter()
            .filter(|g| g.on && g.spec.kind == GadgetKind::Thruster && g.host.is_none())
            .filter_map(|g| g.pose(&self.world).map(|(p, d)| (p, d, g.spec.power)))
            .collect();
        for g in self.gadgets.iter().filter(|g| g.on && g.spec.kind == GadgetKind::Thruster) {
            let Some((p, d)) = g.pose(&self.world) else { continue };
            roaring += 1;
            let at = to_screen(p.x, p.y);
            jets.push((at - screen_dir(d) * 10.0, -screen_dir(d), (6.0 + g.spec.power).min(14.0)));
            let Some(h) = g.host else { continue };
            let Some(b) = self.world.bodies.get_mut(h).filter(|b| b.is_dynamic()) else { continue };
            let f = d * g.spec.power * b.mass() * G;
            b.add_force_at_point(f, p, true);
            // The exhaust warms what is right behind it.
            warm.push(at - screen_dir(d) * 30.0);
        }
        // Fans blow on what is in front of them.
        if !fans.is_empty() {
            for h in self.dynamic_bodies() {
                let Some(b) = self.world.bodies.get_mut(h).filter(|b| b.is_dynamic()) else { continue };
                let pos = Point::from(*b.translation());
                for &(p, d, power) in &fans {
                    let rel = pos - p;
                    let along = rel.dot(&d);
                    let reach = FAN_REACH / PPM;
                    if along <= 0.0 || along > reach || (rel - d * along).norm() > along * FAN_SPREAD + 0.3 {
                        continue;
                    }
                    let f = d * power * G * (1.0 - along / reach) * b.mass();
                    b.add_force(f, true);
                }
            }
        }
        for (at, dir, size) in jets {
            for _ in 0..2 {
                self.effects.jet(at, dir, size);
            }
        }
        for p in warm {
            let near: Vec<RigidBodyHandle> = self
                .objects
                .iter()
                .filter(|o| point_gap_px(&self.world, o, p).is_some_and(|g| g < 12.0))
                .map(|o| o.body)
                .collect();
            for b in near {
                self.fire.warm(b, 0.5 * dt);
            }
        }
        self.thrust_sound -= dt;
        if roaring > 0 && self.thrust_sound <= 0.0 {
            self.thrust_sound = 0.22;
            let at = self
                .gadgets
                .iter()
                .find(|g| g.on && g.spec.kind == GadgetKind::Thruster)
                .and_then(|g| g.pose(&self.world));
            if let Some((p, _)) = at {
                self.sound(Sound::Thrust, to_screen(p.x, p.y), (0.25 + roaring as f32 * 0.08).min(0.6));
            }
        }
    }

    fn run_cannons(&mut self, dt: f32) {
        for i in 0..self.gadgets.len() {
            let g = &mut self.gadgets[i];
            if g.spec.kind != GadgetKind::Cannon {
                continue;
            }
            g.cooldown = (g.cooldown - dt).max(0.0);
            if g.on && g.cooldown <= 0.0 {
                g.cooldown = 1.0 / g.spec.rate.max(0.1);
                self.fire_cannon(i);
            }
        }
    }

    fn fire_cannon(&mut self, i: usize) {
        let g = &self.gadgets[i];
        let Some((p, d)) = g.muzzle(&self.world) else { return };
        let (spec, host) = (g.spec, g.host);
        let carried =
            host.and_then(|h| self.world.bodies.get(h)).map_or(vector![0.0, 0.0], |b| b.velocity_at_point(&p));
        let vel = carried + d * spec.power;
        let at = to_screen(p.x, p.y) + screen_dir(d) * 14.0;
        let before = self.objects.len();
        let mut mass = 0.0;
        match spec.ammo {
            Ammo::Grains => {
                let kind = self.s.grain_kind;
                for _ in 0..6 {
                    let jitter = vector![rand::gen_range(-0.6, 0.6), rand::gen_range(-0.6, 0.6)];
                    let off = vec2(rand::gen_range(-4.0, 4.0), rand::gen_range(-4.0, 4.0));
                    self.grains.add(&mut self.world, kind, at + off, vel + jitter);
                }
                mass = 0.05;
            }
            Ammo::Image if self.last_image().is_some() => {
                let Some(j) = self.last_image() else { return };
                let o = &self.objects[j];
                let k = (56.0 / o.size.x.max(o.size.y)).min(1.0);
                let placement = Placement {
                    pos_px: (at.x, at.y),
                    size_px: Some((o.size.x * k, o.size.y * k)),
                    material: o.material,
                    ..Default::default()
                };
                let shot = Object::spawn(&mut self.world, o.source.clone(), o.visual(), placement);
                self.objects.push(shot);
            }
            Ammo::Shape => {
                let c = spawn_color(self.s.spawn_color, rand::gen_range(0.0, 400.0));
                let rgb = ((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
                let size = self.s.spawn_size.min(70.0);
                self.spawn_shape(self.s.spawn_shape, rgb, size, at, false, Material::DEFAULT);
            }
            _ => {
                let m = Material { bounce: 0.25, friction: 0.6, flammable: false, ..Material::DEFAULT };
                self.spawn_shape(Shape::Circle, (52, 54, 66), BALL_PX, at, false, m);
            }
        }
        for o in &self.objects[before..] {
            if let Some(b) = self.world.bodies.get_mut(o.body) {
                b.set_linvel(vel, true);
                mass += b.mass();
            }
        }
        let shots: Vec<RigidBodyHandle> = self.objects[before..].iter().map(|o| o.body).collect();
        let g = &mut self.gadgets[i];
        g.fired.extend(shots);
        let mut old = vec![];
        while g.fired.len() > MAX_SHOTS {
            old.extend(g.fired.pop_front());
        }
        for h in old {
            if let Some(j) = self.objects.iter().position(|o| o.body == h) {
                self.remove_object(j);
            }
        }
        // Recoil.
        if let Some(b) = host.and_then(|h| self.world.bodies.get_mut(h)).filter(|b| b.is_dynamic()) {
            b.apply_impulse_at_point(-(vel - carried) * mass, p, true);
        }
        let flash = screen_dir(d);
        if self.s.effects {
            self.effects.sparks(at, flash, 6.0 + spec.power * 0.3);
            self.effects.smoke(at, 10.0, 0.6);
        }
        self.sound(Sound::Shot, at, (0.3 + spec.power / 60.0).min(0.7));
    }

    /// The most recently added image (cannon ammunition).
    fn last_image(&self) -> Option<usize> {
        self.objects.iter().rposition(|o| matches!(o.source, Source::File(_) | Source::Memory { .. }))
    }

    /// Follow every working laser's beam; while running, what they hit is
    /// pushed and heated.
    fn trace_beams(&mut self, dt: f32, running: bool) {
        self.beams.clear();
        if !self.gadgets.iter().any(|g| g.on && g.spec.kind == GadgetKind::Laser) {
            return;
        }
        if !running {
            self.world.refresh_queries();
        }
        // What each collider does to a beam.
        let mut surfaces: HashMap<ColliderHandle, Surface> = HashMap::new();
        let mut owners: HashMap<ColliderHandle, usize> = HashMap::new();
        for (i, o) in self.objects.iter().enumerate() {
            owners.insert(o.collider, i);
            if o.material.mirror {
                surfaces.insert(o.collider, Surface::Mirror);
            } else if o.material.is_glass() {
                let tint = *self.glass_tints.entry(o.body).or_insert_with(|| {
                    o.source
                        .decode(24, false)
                        .and_then(|d| d.frames.first().map(super::impacts::average_colour))
                        .unwrap_or(Color::new(0.6, 0.9, 1.0, 1.0))
                });
                surfaces.insert(o.collider, Surface::Glass(tint));
            }
        }
        if self.glass_tints.len() > 256 {
            self.glass_tints.clear();
        }
        let surface = |c: ColliderHandle| surfaces.get(&c).copied().unwrap_or(Surface::Stop);
        let mut beams: Vec<Beam> = vec![];
        for g in self.gadgets.iter().filter(|g| g.on && g.spec.kind == GadgetKind::Laser) {
            let Some((p, d)) = g.muzzle(&self.world) else { continue };
            beams.push(gadgets::trace(&self.world, p, d, g.host, surface));
        }
        if running {
            let mut sparks = vec![];
            for beam in &beams {
                let Some((c, q, d)) = beam.hit else { continue };
                let Some(body) = self.world.colliders.get(c).and_then(|c| c.parent()) else { continue };
                if let Some(b) = self.world.bodies.get_mut(body).filter(|b| b.is_dynamic()) {
                    b.add_force_at_point(d * LASER_PUSH, q, true);
                }
                if owners.contains_key(&c) {
                    self.fire.warm(body, LASER_HEAT * dt);
                }
                if rand::gen_range(0.0, 1.0) < dt * 25.0 {
                    sparks.push((to_screen(q.x, q.y), -screen_dir(d)));
                }
            }
            if self.s.effects {
                for (at, n) in sparks {
                    self.effects.sparks(at, n, 4.0);
                }
            }
        }
        self.beams = beams;
    }

    /// Beams and gadgets (world pass).
    pub(super) fn draw_gadgets(&self) {
        let t = get_time() as f32;
        for beam in &self.beams {
            gadgets::draw_beam(beam, t);
        }
        for g in &self.gadgets {
            let Some((p, d)) = g.pose(&self.world) else { continue };
            gadgets::draw_gadget(g.spec.kind, to_screen(p.x, p.y), screen_dir(d), g.on, t);
        }
    }

    /// The Gadget tool: its aim while dragging, or what it would place.
    pub(super) fn draw_gadget_cursor(&self, m: Vec2) {
        let kind = self.s.gadget_kind;
        let c = kind.accent();
        match &self.gadget_drag {
            Some(drag) => {
                let aim = m - drag.start;
                let dir = if aim.length() > 12.0 { aim.normalize() } else { Self::default_aim(kind) };
                gadgets::draw_gadget(kind, drag.start, dir, false, get_time() as f32);
                let end = drag.start + dir * 70.0;
                draw_line(drag.start.x, drag.start.y, end.x, end.y, 1.5, theme::alpha(c, 0.8));
                let n = vec2(-dir.y, dir.x);
                draw_triangle(end + dir * 8.0, end + n * 5.0, end - n * 5.0, theme::alpha(c, 0.8));
            }
            None => {
                gadgets::draw_gadget(kind, m, Self::default_aim(kind), false, get_time() as f32);
                icons::trigger(self.s.gadget_trigger, m + vec2(20.0, -18.0), 14.0, theme::alpha(c, 0.9));
            }
        }
    }
}
