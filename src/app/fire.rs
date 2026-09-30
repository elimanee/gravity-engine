//! Fire and heat. The Fire tool heats whatever it touches; flammable
//! objects catch fire, burn for a while (longer when bigger), set their
//! neighbours alight — above all the ones over them — and crumble to ash.
//! Ropes and springs tied to a burning object burn through. Objects that do
//! not burn glow and cool down again, glass cracks, ice melts into water,
//! and water or poured liquid puts fires out.

use super::*;
use crate::audio::sfx::Sound;
use crate::physics::grains::GrainKind;
use crate::physics::to_screen;
use rapier2d::parry::query;
use rapier2d::prelude::{nalgebra, vector};

/// Reach of the Fire tool's flame (px).
pub(super) const LIGHTER_REACH: f32 = 16.0;
/// Heat per second from the Fire tool (1 = ignition).
const LIGHTER_HEAT: f32 = 1.6;
/// A burning object heats neighbours this close (px)…
const SPREAD_REACH: f32 = 14.0;
/// …this fast when touching (per second).
const SPREAD_HEAT: f32 = 0.9;
/// Things right above a fire heat up faster.
const ABOVE_BONUS: f32 = 1.8;
/// Cooling per second of objects that are not burning.
const COOLING: f32 = 0.22;
/// Ropes and springs tied to something burning snap after this long (s).
const ROPE_BURN_SECS: f32 = 1.2;
/// Ice starts melting at this temperature.
const MELT_TEMP: f32 = 0.35;
/// Most objects turned to ash or cracked by heat per frame.
const CHANGES_PER_FRAME: usize = 3;

#[derive(Clone, Copy, Default)]
struct Heat {
    /// 0 cold ‥ 1 ignition point.
    temp: f32,
    /// Fuel left while burning (1 → 0).
    fuel: Option<f32>,
    /// Seconds spent burning.
    burnt: f32,
    /// Seconds until ice melts a little more.
    melt: f32,
}

#[derive(Default)]
pub(super) struct Fire {
    heat: HashMap<RigidBodyHandle, Heat>,
    /// The Fire tool is held down.
    pub lit: bool,
    /// Seconds until the next crackle.
    crackle: f32,
}

impl Fire {
    pub fn clear(&mut self) {
        self.heat.clear();
        self.lit = false;
    }

    pub fn burning(&self, body: RigidBodyHandle) -> bool {
        self.heat.get(&body).is_some_and(|h| h.fuel.is_some())
    }

    /// Warm `body` by `amount` (1 = enough to catch fire).
    pub fn warm(&mut self, body: RigidBodyHandle, amount: f32) {
        let h = self.heat.entry(body).or_default();
        h.temp = (h.temp + amount).min(1.5);
    }

    /// Sprite tint: charred while burning, reddish while hot.
    pub fn tint(&self, body: RigidBodyHandle) -> Color {
        let Some(h) = self.heat.get(&body) else { return WHITE };
        let char = h.fuel.map_or(0.0, |f| 1.0 - f);
        let glow = h.temp.clamp(0.0, 1.0);
        let k = 1.0 - char * 0.82;
        Color::new(k, k * (1.0 - glow * 0.3), k * (1.0 - glow * 0.5), 1.0)
    }
}

/// Seconds a flammable object of `size` px burns.
fn burn_secs(size: Vec2) -> f32 {
    3.0 + (size.x * size.y).sqrt() / 22.0
}

/// Distance (px) between the outlines of two objects.
fn gap_px(world: &PhysWorld, a: &Object, b: &Object) -> Option<f32> {
    let (ca, cb) = (world.colliders.get(a.collider)?, world.colliders.get(b.collider)?);
    query::distance(ca.position(), ca.shape(), cb.position(), cb.shape()).ok().map(|d| d * PPM)
}

/// Distance (px) from `p` (world px) to an object's outline (0 inside).
pub(super) fn point_gap_px(world: &PhysWorld, o: &Object, p: Vec2) -> Option<f32> {
    let c = world.colliders.get(o.collider)?;
    let (x, y) = to_phys(p.x, p.y);
    Some(c.shape().distance_to_point(c.position(), &Point::new(x, y), true) * PPM)
}

/// A random point inside the object (world px).
fn random_point(world: &PhysWorld, o: &Object) -> Vec2 {
    let (pos, angle) = o.screen_pos(world);
    let (s, c) = (-angle).sin_cos();
    for _ in 0..4 {
        let (x, y) = (rand::gen_range(-0.45, 0.45) * o.size.x, rand::gen_range(-0.45, 0.45) * o.size.y);
        let p = pos + vec2(x * c - y * s, x * s + y * c);
        if o.contains_px(world, p.x, p.y) {
            return p;
        }
    }
    pos
}

impl App {
    /// One frame of fire (while the simulation runs), `dt` in simulated seconds.
    pub(super) fn update_fire(&mut self, dt: f32, mouse: Vec2) {
        let alive: HashSet<RigidBodyHandle> = self.objects.iter().map(|o| o.body).collect();
        self.fire.heat.retain(|h, _| alive.contains(h));
        if dt <= 0.0 {
            return;
        }

        // The Fire tool.
        if self.fire.lit && self.s.tool == Tool::Fire {
            for _ in 0..2 {
                self.effects.flame(mouse + vec2(rand::gen_range(-5.0, 5.0), 0.0), 9.0);
            }
            let touched: Vec<RigidBodyHandle> = self
                .objects
                .iter()
                .filter(|o| !o.is_visualizer())
                .filter(|o| point_gap_px(&self.world, o, mouse).is_some_and(|d| d < LIGHTER_REACH))
                .map(|o| o.body)
                .collect();
            for b in touched {
                self.fire.warm(b, LIGHTER_HEAT * dt);
            }
        }

        // Burning objects heat their neighbours.
        let burning: Vec<usize> =
            (0..self.objects.len()).filter(|&i| self.fire.burning(self.objects[i].body)).collect();
        let mut gains: Vec<(RigidBodyHandle, f32)> = vec![];
        for &i in &burning {
            let a = &self.objects[i];
            let (pa, _) = a.screen_pos(&self.world);
            let ra = a.size.length() / 2.0;
            for b in &self.objects {
                if b.body == a.body || b.is_visualizer() || self.fire.burning(b.body) {
                    continue;
                }
                let (pb, _) = b.screen_pos(&self.world);
                if pa.distance(pb) > ra + b.size.length() / 2.0 + SPREAD_REACH {
                    continue;
                }
                let Some(d) = gap_px(&self.world, a, b).filter(|d| *d < SPREAD_REACH) else { continue };
                let above = if pb.y < pa.y { ABOVE_BONUS } else { 1.0 };
                gains.push((b.body, SPREAD_HEAT * (1.0 - d / SPREAD_REACH) * above * dt));
            }
        }
        for (b, g) in gains {
            self.fire.warm(b, g);
        }

        // Water puts fires out.
        let water_top = self.s.water.then(|| {
            let rest = water::rest_level(&self.world, self.s.water_level);
            crate::physics::to_screen(0.0, rest).y
        });
        let drops = if burning.is_empty() { vec![] } else { self.grains.liquid_px(&self.world) };

        let mut ash = vec![];
        let mut crack = vec![];
        let mut melted = vec![];
        let mut ignited = None;
        let mut hissed = None;
        for i in 0..self.objects.len() {
            let o = &self.objects[i];
            let Some(mut h) = self.fire.heat.get(&o.body).copied() else { continue };
            let (pos, _) = o.screen_pos(&self.world);
            let wet = water_top.is_some_and(|top| pos.y > top)
                || (h.fuel.is_some()
                    && drops.iter().any(|&d| point_gap_px(&self.world, o, d).is_some_and(|g| g < 6.0)));
            if wet && (h.fuel.is_some() || h.temp > 0.3) {
                if h.fuel.is_some() || h.temp > 0.6 {
                    hissed = Some(pos);
                    for _ in 0..6 {
                        self.effects.smoke(random_point(&self.world, o), 12.0, 1.0);
                    }
                }
                h = Heat { fuel: None, temp: 0.0, ..h };
            }
            if let Some(fuel) = h.fuel {
                let left = fuel - dt / burn_secs(o.size);
                h.burnt += dt;
                h.temp = h.temp.max(1.0);
                // Flames (and smoke) over the whole object, dying down at the end.
                let area = (o.size.x * o.size.y).sqrt();
                let rate = area * 0.8 * (0.35 + fuel);
                let n = (rate * dt + rand::gen_range(0.0, 1.0)) as usize;
                let size = (area / 3.5).clamp(9.0, 30.0) * (0.6 + 0.4 * fuel);
                for _ in 0..n.min(8) {
                    let p = random_point(&self.world, o);
                    self.effects.flame(p, size);
                }
                if self.s.effects && rand::gen_range(0.0, 1.0) < dt * 7.0 {
                    let p = random_point(&self.world, o) - vec2(0.0, o.size.y * 0.3);
                    self.effects.smoke(p, size * 1.3, 0.0);
                }
                if left <= 0.0 {
                    ash.push(i);
                }
                h.fuel = Some(left.max(0.0));
            } else if h.temp >= 1.0 && o.material.flammable {
                h.fuel = Some(1.0);
                ignited = Some(pos);
            } else if h.temp >= 1.0 && o.material.breakable {
                crack.push(i);
            } else if o.material.is_ice() && h.temp > MELT_TEMP {
                h.melt -= dt;
                if h.melt <= 0.0 {
                    h.melt = 0.25 / h.temp.clamp(0.5, 1.5);
                    melted.push(i);
                }
            }
            if h.fuel.is_none() {
                h.temp -= COOLING * dt;
            }
            if h.fuel.is_none() && h.temp <= 0.0 {
                self.fire.heat.remove(&o.body);
            } else {
                self.fire.heat.insert(o.body, h);
            }
        }
        if let Some(at) = ignited {
            self.sound(Sound::Ignite, at, 0.5);
        }
        if let Some(at) = hissed {
            self.sound(Sound::Hiss, at, 0.6);
        }

        // Crackling while anything burns.
        if !burning.is_empty() {
            self.fire.crackle -= dt;
            if self.fire.crackle <= 0.0 {
                self.fire.crackle = rand::gen_range(0.15, 0.5);
                let i = burning[rand::gen_range(0, burning.len())];
                let (at, _) = self.objects[i].screen_pos(&self.world);
                let loud = (burning.len() as f32 / 6.0).min(1.0);
                self.sound(Sound::Crackle, at, 0.2 + loud * 0.25);
            }
        }

        self.burn_links();
        self.melt_ice(melted);
        for i in crack.into_iter().take(CHANGES_PER_FRAME) {
            let o = &self.objects[i];
            let (body, c) = (o.body, *self.world.bodies[o.body].translation());
            self.fire.heat.remove(&body);
            self.shatter(body, Point::new(c.x, c.y));
        }
        // Burnt out: crumble to ash (highest index first, so the others stay valid).
        ash.sort_unstable();
        for i in ash.into_iter().rev().take(CHANGES_PER_FRAME) {
            let o = &self.objects[i];
            let (at, _) = o.screen_pos(&self.world);
            let r = o.size.x.max(o.size.y) / 2.0;
            self.fire.heat.remove(&o.body);
            self.remove_object(i);
            self.effects.debris(at, r, Color::new(0.16, 0.15, 0.15, 1.0));
            for _ in 0..(r / 6.0).clamp(3.0, 12.0) as usize {
                let p = at + vec2(rand::gen_range(-r, r), rand::gen_range(-r, r) * 0.5);
                self.effects.smoke(p, 14.0, 0.1);
            }
            self.sound(Sound::Crackle, at, 0.5);
        }
    }

    /// Ropes and springs tied to something that has burnt a while snap.
    fn burn_links(&mut self) {
        let fire = &self.fire;
        let burnt = |b: RigidBodyHandle| fire.heat.get(&b).is_some_and(|h| h.burnt > ROPE_BURN_SECS);
        let gone: Vec<usize> = (0..self.links.len())
            .filter(|&i| {
                let l = &self.links[i];
                matches!(l.kind, LinkKind::Rope | LinkKind::Spring) && (burnt(l.a) || l.b.is_some_and(burnt))
            })
            .collect();
        for &i in gone.iter().rev() {
            let l = self.links.remove(i);
            if let Some((pa, pb)) = l.ends(&self.world) {
                let mid = (to_screen(pa.x, pa.y) + to_screen(pb.x, pb.y)) / 2.0;
                for _ in 0..6 {
                    self.effects.flame(mid, 8.0);
                }
                self.sound(Sound::Snap, mid, 0.5);
            }
            l.remove(&mut self.world);
        }
    }

    /// Hot ice shrinks and drips water; small enough, it is gone.
    fn melt_ice(&mut self, mut melted: Vec<usize>) {
        melted.sort_unstable();
        for i in melted.into_iter().rev() {
            let o = &self.objects[i];
            let (pos, _) = o.screen_pos(&self.world);
            let bottom = pos + vec2(rand::gen_range(-o.size.x, o.size.x) * 0.3, o.size.y * 0.45);
            let small = o.size.x.min(o.size.y) < 16.0;
            let drips = if small { 6 } else { 2 };
            for k in 0..drips {
                let at = bottom + vec2(k as f32 * 3.0 - drips as f32 * 1.5, 4.0);
                self.grains.add(&mut self.world, GrainKind::Liquid, at, vector![0.0, -0.5]);
            }
            if small {
                self.fire.heat.remove(&o.body);
                self.remove_object(i);
                self.sound(Sound::Hiss, pos, 0.3);
            } else {
                self.objects[i].resize(&mut self.world, 0.94);
            }
        }
    }

    /// The Fire tool's pointer: a small flame and its reach.
    pub(super) fn draw_fire_cursor(&self, m: Vec2) {
        let c = Tool::Fire.accent();
        draw_circle_lines(m.x, m.y, LIGHTER_REACH, 1.2, theme::alpha(c, if self.fire.lit { 0.7 } else { 0.35 }));
        icons::tool(Tool::Fire, m + vec2(16.0, -18.0), 24.0, theme::alpha(c, 0.9), get_time() as f32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heat_tints_and_burn_times() {
        let mut f = Fire::default();
        let h = RigidBodyHandle::from_raw_parts(3, 0);
        assert_eq!(f.tint(h), WHITE);
        f.warm(h, 0.8);
        let t = f.tint(h);
        assert!(t.r > t.b, "hot things look reddish");
        assert!(!f.burning(h));
        assert!(burn_secs(vec2(200.0, 200.0)) > burn_secs(vec2(20.0, 20.0)), "big things burn longer");
    }
}
