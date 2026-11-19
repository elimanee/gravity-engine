//! The Gadget tool (lasers, thrusters, cannons, lamps, hooks, and the
//! mechanisms: buttons, sensors, targets, timers, pistons) and what gadgets
//! do each frame, plus motors driven with the arrow keys or a signal.

use super::fire::point_gap_px;
use super::*;
use crate::audio::sfx::Sound;
use crate::physics::gadgets::{self, Ammo, Beam, Gadget, GadgetKind, GadgetSpec, Surface, Trigger, CHANNELS};
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

/// A dashed line from `a` to `b` (screen).
fn dashed(a: Vec2, b: Vec2, dash: f32, gap: f32, width: f32, c: Color) {
    let len = a.distance(b);
    if len < 1.0 {
        return;
    }
    let d = (b - a) / len;
    let mut s = 0.0;
    while s < len {
        let e = (s + dash).min(len);
        let (p, q) = (a + d * s, a + d * e);
        draw_line(p.x, p.y, q.x, q.y, width, c);
        s += dash + gap;
    }
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
            GadgetKind::Lamp => vec2(0.0, 1.0),
            GadgetKind::Grapple => vec2(0.0, -1.0),
            GadgetKind::Sensor => vec2(1.0, 0.0),
            GadgetKind::Button | GadgetKind::Target | GadgetKind::Timer | GadgetKind::Piston => vec2(0.0, -1.0),
        }
    }

    pub(super) fn gadget_release(&mut self, m: Vec2) {
        let Some(drag) = self.gadget_drag.take() else { return };
        let kind = self.s.gadget_kind;
        let aim = m - drag.start;
        let aim = if aim.length() > 12.0 { aim.normalize() } else { Self::default_aim(kind) };
        let world_angle = (-aim.y).atan2(aim.x);
        let host = drag.host.filter(|(b, _)| self.world.bodies.contains(*b));
        if kind == GadgetKind::Piston && host.is_none() {
            self.toasts.status("gadget", "A piston goes on an object (a plank becomes a door or a lift)");
            return;
        }
        let (at, angle) = match host {
            Some((b, local)) => ([local.x, local.y], world_angle - self.world.bodies[b].rotation().angle()),
            None => {
                let (x, y) = to_phys(drag.start.x, drag.start.y);
                ([x, y], world_angle)
            }
        };
        let power = match kind {
            GadgetKind::Thruster => self.s.thrust,
            GadgetKind::Lamp => self.s.lamp_reach / PPM,
            GadgetKind::Sensor => self.s.sensor_reach,
            GadgetKind::Timer => self.s.timer_period,
            GadgetKind::Piston => self.s.piston_travel,
            _ => self.s.cannon_speed,
        };
        let rate = if kind == GadgetKind::Piston { self.s.piston_speed } else { self.s.cannon_rate };
        // What sends a signal works by itself.
        let trigger = if kind.sends() { Trigger::Always } else { self.s.gadget_trigger };
        let spread =
            if kind == GadgetKind::Lamp && self.s.lamp_spot { gadgets::SPOT_SPREAD } else { std::f32::consts::PI };
        let colour = gadgets::LAMP_COLOURS[self.s.lamp_colour.min(gadgets::LAMP_COLOURS.len() - 1)];
        let spec = GadgetSpec {
            kind,
            trigger,
            ammo: self.s.cannon_ammo,
            power,
            rate,
            at,
            angle,
            spread,
            colour,
            channel: self.s.signal_channel,
            mode: self.s.signal_mode,
            ext: 0.0,
        };
        self.record(kind.label());
        // A piston holds its object still (it is pinned) and slides it.
        if let Some((b, _)) = host.filter(|_| kind == GadgetKind::Piston) {
            if let Some(o) = self.objects.iter_mut().find(|o| o.body == b) {
                o.set_pinned(&mut self.world, true);
            }
        }
        self.gadgets.push(Gadget::new(spec, host.map(|(b, _)| b)));
        self.sound(Sound::Snap, drag.start, 0.5);
        let what = match (kind, host.is_some()) {
            (GadgetKind::Thruster, false) => "Fan".to_string(),
            (GadgetKind::Grapple, false) => "Winch".to_string(),
            (GadgetKind::Grapple, true) => "Grappling hook".to_string(),
            _ => kind.label().to_string(),
        };
        let msg = if kind == GadgetKind::Lamp && !self.s.night {
            format!("{what} added  ·  its light shows at night (Shift+T)")
        } else if kind.sends() {
            let how = if kind == GadgetKind::Timer { "while on" } else { spec.mode.about() };
            format!("{what} added  ·  sends the {} signal {how}", gadgets::channel_name(spec.channel))
        } else if kind == GadgetKind::Piston {
            format!("Piston added  ·  slides its object {}", spec.trigger.label())
        } else {
            format!("{what} added  ·  works {}  ·  right-click it to remove", spec.trigger.label())
        };
        self.toasts.status("gadget", msg);
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
        if let Some(l) = &g.hook {
            l.remove(&mut self.world);
        }
        self.toasts.status("gadget", format!("{} removed", g.spec.kind.label()));
    }

    /// Whether an arrow key (or anything else) makes `t` work right now.
    fn trigger_held(&self, t: Trigger) -> bool {
        let typing = self.editor_bar.typing() || self.level_bar.typing();
        let key = |k| !typing && is_key_down(k);
        let gp = self.pad.state();
        match t {
            Trigger::Always => true,
            Trigger::Up => key(KeyCode::Up) || gp.up(),
            Trigger::Down => key(KeyCode::Down) || gp.down(),
            Trigger::Left => key(KeyCode::Left) || gp.left(),
            Trigger::Right => key(KeyCode::Right) || gp.right(),
            Trigger::Beat => false,
            Trigger::Signal(c) => self.signals.get(c as usize).copied().unwrap_or(false),
        }
    }

    /// Something is driven with ← (so holding it does not rewind).
    pub(super) fn listens_to_left(&self) -> bool {
        self.gadgets.iter().any(|g| g.spec.trigger == Trigger::Left)
            || self.links.iter().any(|l| l.drive && l.channel.is_none())
            || self.sonic.is_some()
    }

    /// Where the character is (centre and half size, m), for buttons and
    /// sensors.
    fn sonic_box(&self) -> Option<(Point<f32>, Vector<f32>)> {
        let s = &self.sonic.as_ref()?.sonic;
        if s.dead {
            return None;
        }
        let (wr, hr) = s.radii();
        let (x, y) = to_phys(s.pos.x, s.pos.y);
        let k = crate::sonic::SCALE / PPM;
        Some((Point::new(x, y), vector![wr * k, hr * k]))
    }

    /// Whether what is on button `i` presses it: anything that is not
    /// fixed in place (pinned and driven things too), or the character.
    fn button_pressed(&self, i: usize, sonic: Option<(Point<f32>, Vector<f32>)>) -> bool {
        let g = &self.gadgets[i];
        g.pad_touched(&self.world)
            || sonic.zip(g.pad(&self.world)).is_some_and(|((c, half), (centre, _))| {
                let r = gadgets::PAD_HALF.1;
                (c.x - centre.x).abs() < half.x + r && (c.y - centre.y).abs() < half.y + r
            })
    }

    /// Sensor `i`'s beam: how far it reaches (m) and whether something
    /// that is not fixed in place (or the character) is in it.
    fn sensor_beam(&self, i: usize, sonic: Option<(Point<f32>, Vector<f32>)>) -> (f32, bool) {
        let g = &self.gadgets[i];
        let (seen, moving) = g.sense(&self.world);
        let crossed = g.muzzle(&self.world).zip(sonic).is_some_and(|((p, d), (c, half))| {
            let (min, max) = ([c.x - half.x, c.y - half.y], [c.x + half.x, c.y + half.y]);
            gadgets::segment_hits_rect(p, p + d * seen, min, max)
        });
        (seen, moving || crossed)
    }

    /// The mechanisms: what each one senses, and so which channels are on.
    fn update_signals(&mut self, dt: f32, running: bool) {
        if !self.gadgets.iter().any(|g| g.spec.kind.sends()) {
            self.signals = [false; CHANNELS];
            return;
        }
        if !running {
            self.world.refresh_queries();
        }
        let sonic = self.sonic_box();
        let mut signals = [false; CHANNELS];
        let mut clicks = vec![];
        for i in 0..self.gadgets.len() {
            let kind = self.gadgets[i].spec.kind;
            if !kind.sends() {
                continue;
            }
            let raw = match kind {
                GadgetKind::Button => self.button_pressed(i, sonic),
                GadgetKind::Sensor => {
                    let (seen, hit) = self.sensor_beam(i, sonic);
                    self.gadgets[i].seen = seen;
                    hit
                }
                GadgetKind::Target => self.gadgets[i].lit,
                _ => {
                    let g = &mut self.gadgets[i];
                    let period = g.spec.power.max(0.1);
                    if running {
                        g.clock = (g.clock + dt) % period;
                    }
                    g.clock < period / 2.0
                }
            };
            let g = &mut self.gadgets[i];
            let was = g.on;
            if g.signal(raw) {
                signals[(g.spec.channel as usize).min(CHANNELS - 1)] = true;
            }
            if g.on != was && kind != GadgetKind::Timer {
                if let Some((p, _)) = g.pose(&self.world) {
                    clicks.push((to_screen(p.x, p.y), g.on));
                }
            }
        }
        self.signals = signals;
        for (at, on) in clicks {
            self.sound(Sound::Snap, at, if on { 0.4 } else { 0.2 });
        }
    }

    /// Pistons slide their object out while they work, and back.
    fn run_pistons(&mut self, dt: f32) {
        for i in 0..self.gadgets.len() {
            let g = &self.gadgets[i];
            if g.spec.kind != GadgetKind::Piston {
                continue;
            }
            let (Some(h), Some((_, d))) = (g.host, g.pose(&self.world)) else { continue };
            // It holds its object (pinned again if it was let go).
            if let Some(o) = self.objects.iter_mut().find(|o| o.body == h && !o.pinned) {
                o.set_pinned(&mut self.world, true);
            }
            let g = &mut self.gadgets[i];
            let (ext, goal, step) = (g.spec.ext, if g.on { g.spec.power } else { 0.0 }, g.spec.rate * dt);
            let next = if ext < goal { (ext + step).min(goal) } else { (ext - step).max(goal) };
            if next == ext {
                continue;
            }
            g.spec.ext = next;
            if let Some(b) = self.world.bodies.get_mut(h) {
                let to = b.translation() + d * (next - ext);
                b.set_next_kinematic_translation(to);
            }
            self.world.wake_around(h, 0.3);
        }
    }

    /// Once per frame before the physics step: which gadgets work, thrust,
    /// cannon shots, laser beams and driven motors. `dt` is the simulated
    /// time (0 when paused).
    pub(super) fn update_gadgets(&mut self, dt: f32, running: bool, mouse: Vec2) {
        let world = &self.world;
        self.gadgets.retain(|g| g.host.is_none_or(|h| world.bodies.contains(h)));
        self.update_signals(dt, running);
        let beat = running && self.audio.analyzer.beat_now;
        for i in 0..self.gadgets.len() {
            if self.gadgets[i].spec.kind.sends() {
                continue;
            }
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
            let signals = self.signals;
            for l in &self.links {
                let d = match l.channel {
                    Some(c) => signals[(c as usize).min(CHANNELS - 1)].then_some(1.0),
                    None => dir,
                };
                l.set_drive(&mut self.world, d);
            }
            self.run_thrusters(dt);
            self.run_cannons(dt);
            self.run_grapples(dt, mouse);
            self.run_pistons(dt);
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
        for g in self.gadgets.iter_mut().filter(|g| g.spec.kind == GadgetKind::Target) {
            g.lit = false;
        }
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
        // Targets stop the beams that reach them, and light up.
        let targets: Vec<(usize, Point<f32>)> = (0..self.gadgets.len())
            .filter(|&i| self.gadgets[i].spec.kind == GadgetKind::Target)
            .filter_map(|i| self.gadgets[i].pose(&self.world).map(|(p, _)| (i, p)))
            .collect();
        if !targets.is_empty() {
            let centres: Vec<Point<f32>> = targets.iter().map(|t| t.1).collect();
            for beam in &mut beams {
                if let Some(k) = gadgets::clip_at_target(beam, &centres) {
                    self.gadgets[targets[k].0].lit = true;
                }
            }
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
        self.draw_sensor_beams(t);
        self.draw_wires(t);
        self.draw_hooks();
        for g in self.gadgets.iter().filter(|g| g.spec.kind != GadgetKind::Lamp) {
            let Some((p, d)) = g.pose(&self.world) else { continue };
            let on = if g.spec.kind == GadgetKind::Grapple { g.hook.is_some() } else { g.on };
            gadgets::draw_gadget(g.spec.kind, to_screen(p.x, p.y), screen_dir(d), on, t);
        }
        self.draw_lamps();
        // Each mechanism's channel, and what works on one.
        for g in &self.gadgets {
            let channel = if g.spec.kind.sends() { Some(g.spec.channel) } else { g.spec.trigger.channel() };
            let (Some(c), Some((p, d))) = (channel, g.pose(&self.world)) else { continue };
            let at = to_screen(p.x, p.y) - screen_dir(d) * 9.0 + vec2(9.0, -9.0);
            let on = self.signals.get(c as usize).copied().unwrap_or(false);
            gadgets::draw_channel_dot(at, c, on);
        }
    }

    /// Sensors' beams: faint, bright while something crosses them.
    fn draw_sensor_beams(&self, t: f32) {
        for g in self.gadgets.iter().filter(|g| g.spec.kind == GadgetKind::Sensor) {
            let Some((p, d)) = g.muzzle(&self.world) else { continue };
            let (a, b) = (to_screen(p.x, p.y), to_screen(p.x + d.x * g.seen, p.y + d.y * g.seen));
            let c = GadgetKind::Sensor.accent();
            let alpha = if g.on { 0.85 } else { 0.3 + 0.08 * (t * 3.0).sin() };
            dashed(a, b, 6.0, 4.0, 1.4, Color { a: alpha, ..c });
            if g.on {
                draw_circle(b.x, b.y, 4.0, Color { a: 0.5, ..c });
            }
        }
    }

    /// The wires from what sends each signal to what works on it, while
    /// the Gadget or Link tool is out (brighter while it is on).
    fn draw_wires(&self, t: f32) {
        if !matches!(self.s.tool, Tool::Gadget | Tool::Link) {
            return;
        }
        let at = |g: &Gadget| g.pose(&self.world).map(|(p, _)| to_screen(p.x, p.y));
        for c in 0..CHANNELS as u8 {
            let senders: Vec<Vec2> =
                self.gadgets.iter().filter(|g| g.spec.kind.sends() && g.spec.channel == c).filter_map(at).collect();
            if senders.is_empty() {
                continue;
            }
            let mut ends: Vec<Vec2> =
                self.gadgets.iter().filter(|g| g.spec.trigger == Trigger::Signal(c)).filter_map(at).collect();
            for l in self.links.iter().filter(|l| l.channel == Some(c)) {
                if let Some(b) = self.world.bodies.get(l.a) {
                    let p = b.position() * l.la;
                    ends.push(to_screen(p.x, p.y));
                }
            }
            let on = self.signals[c as usize];
            let colour = gadgets::channel_colour(c);
            let colour = Color { a: if on { 0.75 } else { 0.35 }, ..colour };
            for &a in &senders {
                for &b in &ends {
                    dashed(a, b, 7.0, 5.0, if on { 2.0 } else { 1.2 }, colour);
                    if on {
                        // A spark running along it.
                        let u = (t * 0.8 + a.x * 0.001) % 1.0;
                        let p = a + (b - a) * u;
                        draw_circle(p.x, p.y, 2.5, Color { a: 0.9, ..colour });
                    }
                }
            }
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
                if kind == GadgetKind::Lamp {
                    super::night::cursor_lamp(&self.s, drag.start, dir);
                } else {
                    gadgets::draw_gadget(kind, drag.start, dir, false, get_time() as f32);
                }
                let end = drag.start + dir * 70.0;
                draw_line(drag.start.x, drag.start.y, end.x, end.y, 1.5, theme::alpha(c, 0.8));
                let n = vec2(-dir.y, dir.x);
                draw_triangle(end + dir * 8.0, end + n * 5.0, end - n * 5.0, theme::alpha(c, 0.8));
            }
            None => {
                if kind == GadgetKind::Lamp {
                    super::night::cursor_lamp(&self.s, m, Self::default_aim(kind));
                } else {
                    gadgets::draw_gadget(kind, m, Self::default_aim(kind), false, get_time() as f32);
                }
                if kind.sends() {
                    gadgets::draw_channel_dot(m + vec2(20.0, -18.0), self.s.signal_channel, false);
                } else {
                    icons::trigger(self.s.gadget_trigger, m + vec2(20.0, -18.0), 14.0, theme::alpha(c, 0.9));
                }
            }
        }
    }
}
