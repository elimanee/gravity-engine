//! The zone's gimmicks: bridges of logs that dip under the character,
//! platforms swinging on chains, ledges that crumble away once stood on,
//! walls broken by rolling into them, logs with spikes going round them,
//! and rocks. Each is made of bodies of its own in the world, moved by the
//! game (the logs and the swinging platforms carry the character along).

use super::*;

/// Sizes (world px): a bridge log, the dip of a bridge per log from its
/// nearer end (and the most), a swinging platform and its chain, a
/// crumbling ledge's columns and depth, a breakable wall, a spiked log's
/// height and its spikes, and a rock.
pub const LOG: f32 = 16.0 * SCALE;
const DIP: f32 = 2.0 * SCALE;
const MOST_DIP: f32 = 16.0 * SCALE;
pub const SWING_PLATFORM: Vec2 = Vec2::new(48.0 * SCALE, 16.0 * SCALE);
pub const CHAIN: f32 = 96.0 * SCALE;
pub const LEDGE_COLUMN: f32 = 16.0 * SCALE;
pub const LEDGE_DEPTH: f32 = 48.0 * SCALE;
pub const WALL: Vec2 = Vec2::new(32.0 * SCALE, 64.0 * SCALE);
pub const SPIKE_LOG: f32 = 16.0 * SCALE;
pub const ROCK: Vec2 = Vec2::new(48.0 * SCALE, 32.0 * SCALE);
/// A swing's reach either way (rad) and how long a swing there and back
/// takes (s).
const SWING_AMPLITUDE: f32 = 1.05;
const SWING_PERIOD: f32 = 3.4;
/// A ledge crumbles this long after it is stood on, a column after the
/// next as they go away from where; it comes back this long after, once
/// the character is this far away.
const CRUMBLE_DELAY: f32 = 0.5;
const CRUMBLE_EACH: f32 = 0.1;
const LEDGE_BACK: f32 = 5.0;
const LEDGE_AWAY: f32 = 500.0;
/// Pieces falling (world px/s²) and how long they are seen.
const FALL: f32 = 1500.0;
const FALL_TIME: f32 = 1.6;
/// The spikes go round a log this often (s), each log a little after the
/// one before.
pub const SPIN_PERIOD: f32 = 1.8;
pub const SPIN_STAGGER: f32 = 0.125;

/// The gimmicks the level editor places.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gimmick {
    Bridge,
    Swing,
    Crumble,
    Wall,
    SpikeLog,
    Rock,
}

impl Gimmick {
    pub const ALL: [Gimmick; 6] =
        [Gimmick::Bridge, Gimmick::Swing, Gimmick::Crumble, Gimmick::Wall, Gimmick::SpikeLog, Gimmick::Rock];

    pub fn label(self) -> &'static str {
        match self {
            Gimmick::Bridge => "Bridge",
            Gimmick::Swing => "Swing",
            Gimmick::Crumble => "Crumble",
            Gimmick::Wall => "Wall",
            Gimmick::SpikeLog => "Spin log",
            Gimmick::Rock => "Rock",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Gimmick::Bridge => {
                "Drag across a gap for a bridge of logs (it dips underfoot)  ·  click again: another gimmick"
            }
            Gimmick::Swing => {
                "Click where a platform hangs from: it swings on its chain  ·  click again: another gimmick"
            }
            Gimmick::Crumble => "Drag across for a ledge that crumbles once stood on  ·  click again: another gimmick",
            Gimmick::Wall => "Click above the ground for a wall to roll through  ·  click again: another gimmick",
            Gimmick::SpikeLog => "Drag across for a log with spikes going round it  ·  click again: another gimmick",
            Gimmick::Rock => "Click above the ground for a rock  ·  click again: another gimmick",
        }
    }

    /// Made by dragging across (else by a click).
    pub fn dragged(self) -> bool {
        matches!(self, Gimmick::Bridge | Gimmick::Crumble | Gimmick::SpikeLog)
    }
}

/// A span along the level: from x0 to x1 (metres), with its top at y.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub x0: f32,
    pub x1: f32,
    pub y: f32,
}

impl Span {
    /// In world px: (x0, x1, top).
    pub fn px(&self) -> (f32, f32, f32) {
        let a = to_screen(self.x0, self.y);
        let b = to_screen(self.x1, self.y);
        (a.x.min(b.x), a.x.max(b.x), a.y)
    }

    pub fn from_px(x0: f32, x1: f32, top: f32) -> Span {
        let (a, y) = to_phys(x0.min(x1), top);
        let (b, _) = to_phys(x0.max(x1), top);
        Span { x0: a, x1: b, y }
    }
}

pub struct Bridge {
    /// Each log's body and collider, where it rests (centre, world px) and
    /// how far it dips now.
    pub logs: Vec<(RigidBodyHandle, ColliderHandle, Vec2, f32)>,
}

pub struct Swing {
    /// What it hangs from (world px).
    pub anchor: Vec2,
    body: RigidBodyHandle,
    pub collider: ColliderHandle,
    /// Where the platform is now (centre) and how far round (rad).
    pub pos: Vec2,
    pub angle: f32,
}

/// One column of a crumbling ledge: where it is (centre), its body while
/// it stands, and how long it has been falling.
pub struct Column {
    pub home: Vec2,
    body: Option<(RigidBodyHandle, ColliderHandle)>,
    pub falling: Option<f32>,
    /// Seconds before it falls, once the ledge goes.
    wait: f32,
}

pub struct Ledge {
    pub columns: Vec<Column>,
    /// Seconds since it was stood on.
    trigger: Option<f32>,
}

pub struct Wall {
    pub pos: Vec2,
    body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

pub struct SpikeLog {
    pub x0: f32,
    pub x1: f32,
    pub top: f32,
    body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

pub struct Rock {
    pub pos: Vec2,
    body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

/// A zone's gimmicks in play.
#[derive(Default)]
pub struct Things {
    pub bridges: Vec<Bridge>,
    pub swings: Vec<Swing>,
    pub ledges: Vec<Ledge>,
    pub walls: Vec<Wall>,
    pub spike_logs: Vec<SpikeLog>,
    pub rocks: Vec<Rock>,
    /// The clock the swings and spikes keep.
    pub t: f32,
}

fn moving_box(world: &mut PhysWorld, centre: Vec2, size: Vec2) -> (RigidBodyHandle, ColliderHandle) {
    let (x, y) = to_phys(centre.x, centre.y);
    let body = world.bodies.insert(RigidBodyBuilder::kinematic_position_based().translation(vector![x, y]));
    let col = ColliderBuilder::cuboid(size.x / 2.0 / PPM, size.y / 2.0 / PPM).friction(0.6);
    let collider = world.colliders.insert_with_parent(col, body, &mut world.bodies);
    (body, collider)
}

fn move_to(world: &mut PhysWorld, body: RigidBodyHandle, centre: Vec2) {
    if let Some(b) = world.bodies.get_mut(body) {
        let (x, y) = to_phys(centre.x, centre.y);
        b.set_next_kinematic_translation(vector![x, y]);
    }
}

/// Where a swinging platform is (centre) `angle` round from straight down.
pub fn swing_at(anchor: Vec2, angle: f32) -> Vec2 {
    anchor + vec2(angle.sin(), angle.cos()) * CHAIN + vec2(0.0, SWING_PLATFORM.y / 2.0)
}

/// How far round a swing is at `t` (s).
fn swing_angle(t: f32) -> f32 {
    SWING_AMPLITUDE * (t * std::f32::consts::TAU / SWING_PERIOD).sin()
}

/// How far round (0 ‥ 1, 0 on top, a quarter towards the front) the spikes
/// of the `k`-th piece of a spiked log are at `t`: they come up and over
/// towards the front, the pattern moving along the log (as the zone
/// sheet's eight pictures show it: the second piece up in the first).
pub fn spin(t: f32, k: usize) -> f32 {
    (k as f32 * SPIN_STAGGER - SPIN_STAGGER - t / SPIN_PERIOD).rem_euclid(1.0)
}

/// Which of a spiked log's eight pictures shows at `t`.
pub fn spin_picture(t: f32) -> usize {
    ((t / SPIN_PERIOD * 8.0).floor() as i64).rem_euclid(8) as usize
}

/// Whether spikes `turn` round (0 ‥ 1) point up out of the log.
pub fn spikes_up(turn: f32) -> bool {
    (turn * std::f32::consts::TAU).cos() > 0.6
}

/// How far each of a bridge's `n` logs dips with the character on log `on`.
pub fn dips(n: usize, on: Option<usize>) -> Vec<f32> {
    let Some(i) = on.filter(|&i| i < n) else { return vec![0.0; n] };
    let most = (DIP * (i + 1).min(n - i) as f32).min(MOST_DIP);
    (0..n)
        .map(|k| {
            let t = if k <= i { (k + 1) as f32 / (i + 1) as f32 } else { (n - k) as f32 / (n - i) as f32 };
            most * (t * std::f32::consts::FRAC_PI_2).sin()
        })
        .collect()
}

impl Things {
    pub fn new(spec: &LevelSpec, world: &mut PhysWorld) -> Things {
        let mut th = Things::default();
        th.add(spec, world);
        th
    }

    /// Those of `spec` too (an endless run's, as it is made).
    pub fn add(&mut self, spec: &LevelSpec, world: &mut PhysWorld) {
        let th = self;
        for s in &spec.bridges {
            let (x0, x1, top) = s.px();
            let n = ((x1 - x0) / LOG).round().clamp(2.0, 60.0) as usize;
            let w = (x1 - x0) / n as f32;
            let logs = (0..n)
                .map(|k| {
                    let c = vec2(x0 + (k as f32 + 0.5) * w, top + LOG / 4.0);
                    let (b, col) = moving_box(world, c, vec2(w, LOG / 2.0));
                    (b, col, c, 0.0)
                })
                .collect();
            th.bridges.push(Bridge { logs });
        }
        for &a in &spec.swings {
            let anchor = px(a);
            let pos = swing_at(anchor, 0.0);
            let (body, collider) = moving_box(world, pos, SWING_PLATFORM);
            th.swings.push(Swing { anchor, body, collider, pos, angle: 0.0 });
        }
        for s in &spec.ledges {
            let (x0, x1, top) = s.px();
            let n = ((x1 - x0) / LEDGE_COLUMN).round().clamp(1.0, 60.0) as usize;
            let w = (x1 - x0) / n as f32;
            let columns = (0..n)
                .map(|k| {
                    let home = vec2(x0 + (k as f32 + 0.5) * w, top + LEDGE_DEPTH / 2.0);
                    Column { home, body: Some(fixed_box(world, home, vec2(w, LEDGE_DEPTH))), falling: None, wait: 0.0 }
                })
                .collect();
            th.ledges.push(Ledge { columns, trigger: None });
        }
        for &w in &spec.walls {
            let pos = px(w) - vec2(0.0, WALL.y / 2.0);
            let (body, collider) = fixed_box(world, pos, WALL);
            th.walls.push(Wall { pos, body, collider });
        }
        for s in &spec.spike_logs {
            let (x0, x1, top) = s.px();
            let (body, collider) =
                fixed_box(world, vec2((x0 + x1) / 2.0, top + SPIKE_LOG / 2.0), vec2(x1 - x0, SPIKE_LOG));
            th.spike_logs.push(SpikeLog { x0, x1, top, body, collider });
        }
        for &r in &spec.rocks {
            let pos = px(r) - vec2(0.0, ROCK.y / 2.0);
            let (body, collider) = fixed_box(world, pos, ROCK);
            th.rocks.push(Rock { pos, body, collider });
        }
    }

    /// Forget those wholly behind `x` (endless runs), their bodies too.
    pub fn forget_before(&mut self, world: &mut PhysWorld, x: f32) {
        let mut gone = vec![];
        self.bridges.retain(|b| {
            let keep = b.logs.last().is_some_and(|l| l.2.x > x);
            if !keep {
                gone.extend(b.logs.iter().map(|l| l.0));
            }
            keep
        });
        self.swings.retain(|s| {
            let keep = s.anchor.x + CHAIN > x;
            if !keep {
                gone.push(s.body);
            }
            keep
        });
        self.ledges.retain(|l| {
            let keep = l.columns.last().is_some_and(|c| c.home.x > x);
            if !keep {
                gone.extend(l.columns.iter().filter_map(|c| c.body.map(|b| b.0)));
            }
            keep
        });
        self.walls.retain(|w| {
            let keep = w.pos.x + WALL.x > x;
            if !keep {
                gone.push(w.body);
            }
            keep
        });
        self.spike_logs.retain(|s| {
            let keep = s.x1 > x;
            if !keep {
                gone.push(s.body);
            }
            keep
        });
        self.rocks.retain(|r| {
            let keep = r.pos.x + ROCK.x > x;
            if !keep {
                gone.push(r.body);
            }
            keep
        });
        for b in gone {
            world.remove_body(b);
        }
    }

    fn bodies(&self) -> impl Iterator<Item = RigidBodyHandle> + '_ {
        let logs = self.bridges.iter().flat_map(|b| b.logs.iter().map(|l| l.0));
        let swings = self.swings.iter().map(|s| s.body);
        let ledges = self.ledges.iter().flat_map(|l| l.columns.iter().filter_map(|c| c.body.map(|b| b.0)));
        let walls = self.walls.iter().map(|w| w.body);
        let logs2 = self.spike_logs.iter().map(|s| s.body);
        let rocks = self.rocks.iter().map(|r| r.body);
        logs.chain(swings).chain(ledges).chain(walls).chain(logs2).chain(rocks)
    }

    /// Take their bodies out of the world.
    pub fn remove(&self, world: &mut PhysWorld) {
        for b in self.bodies() {
            world.remove_body(b);
        }
    }

    /// Everything moved `dx` px to the left (infinite world).
    pub fn shift(&mut self, dx: f32) {
        let d = vec2(dx, 0.0);
        for b in &mut self.bridges {
            for l in &mut b.logs {
                l.2 -= d;
            }
        }
        for s in &mut self.swings {
            s.anchor -= d;
            s.pos -= d;
        }
        for l in &mut self.ledges {
            for c in &mut l.columns {
                c.home -= d;
            }
        }
        for w in &mut self.walls {
            w.pos -= d;
        }
        for s in &mut self.spike_logs {
            s.x0 -= dx;
            s.x1 -= dx;
        }
        for r in &mut self.rocks {
            r.pos -= d;
        }
    }

    /// What they are to the character underfoot.
    pub fn grounds(&self) -> Vec<(ColliderHandle, Ground)> {
        let solid = Ground { friction: 0.6, ..Ground::default() };
        let mut out = vec![];
        out.extend(self.bridges.iter().flat_map(|b| b.logs.iter().map(move |l| (l.1, solid))));
        out.extend(self.swings.iter().map(|s| (s.collider, solid)));
        out.extend(self.ledges.iter().flat_map(|l| l.columns.iter().filter_map(move |c| Some((c.body?.1, solid)))));
        out.extend(self.walls.iter().map(|w| (w.collider, Ground { breakable: true, ..solid })));
        out.extend(self.spike_logs.iter().map(|s| (s.collider, solid)));
        out.extend(self.rocks.iter().map(|r| (r.collider, solid)));
        out
    }

    /// A frame: the bridges dip under the character, the platforms swing,
    /// the ledges stood on crumble (and come back later). Returns where
    /// a ledge began to fall.
    pub fn step(&mut self, world: &mut PhysWorld, sonic: &Sonic, dt: f32) -> Vec<Vec2> {
        self.t += dt;
        let on = sonic.on.filter(|_| sonic.grounded);
        // Bridges: the logs ease towards their dip.
        let ease = 1.0 - 0.7f32.powf(dt * 60.0);
        for b in &mut self.bridges {
            let under = b.logs.iter().position(|l| Some(l.1) == on);
            let want = dips(b.logs.len(), under);
            for (l, d) in b.logs.iter_mut().zip(want) {
                l.3 += (d - l.3) * ease;
                move_to(world, l.0, l.2 + vec2(0.0, l.3));
            }
        }
        for s in &mut self.swings {
            s.angle = swing_angle(self.t);
            s.pos = swing_at(s.anchor, s.angle);
            move_to(world, s.body, s.pos);
        }
        let mut fell = vec![];
        for l in &mut self.ledges {
            let stood = l.columns.iter().position(|c| c.body.is_some_and(|b| Some(b.1) == on));
            if let (None, Some(k)) = (l.trigger, stood) {
                l.trigger = Some(0.0);
                // The column underfoot goes first, then those beside it.
                for (j, c) in l.columns.iter_mut().enumerate() {
                    c.wait = CRUMBLE_DELAY + j.abs_diff(k) as f32 * CRUMBLE_EACH;
                }
                fell.push(l.columns[k].home);
            }
            let Some(t) = l.trigger.as_mut() else { continue };
            *t += dt;
            let t = *t;
            for c in &mut l.columns {
                if c.falling.is_none() && t >= c.wait {
                    if let Some((b, _)) = c.body.take() {
                        world.remove_body(b);
                    }
                    c.falling = Some(0.0);
                }
                if let Some(f) = c.falling.as_mut() {
                    *f += dt;
                }
            }
            // Back once it is all gone and out of the way.
            let gone = l.columns.iter().all(|c| c.falling.is_some_and(|f| f > FALL_TIME));
            let far = l.columns.iter().all(|c| (c.home - sonic.pos).length() > LEDGE_AWAY);
            if gone && far && t > LEDGE_BACK {
                let w = l
                    .columns
                    .first()
                    .map_or(LEDGE_COLUMN, |c| l.columns.get(1).map_or(LEDGE_COLUMN, |d| (d.home.x - c.home.x).abs()));
                for c in &mut l.columns {
                    c.body = Some(fixed_box(world, c.home, vec2(w, LEDGE_DEPTH)));
                    c.falling = None;
                }
                l.trigger = None;
            }
        }
        fell
    }

    /// Where a falling column of a ledge is now (centre), if still seen.
    pub fn falling_at(c: &Column) -> Option<Vec2> {
        let f = c.falling.filter(|&f| f <= FALL_TIME)?;
        Some(c.home + vec2(0.0, FALL * f * f / 2.0))
    }

    /// Spikes up under the character standing on a spiked log: where they
    /// hurt from.
    pub fn spiked(&self, sonic: &Sonic) -> Option<Vec2> {
        let on = sonic.on.filter(|_| sonic.grounded)?;
        let s = self.spike_logs.iter().find(|s| s.collider == on)?;
        let k = ((sonic.pos.x - s.x0) / SPIKE_LOG).floor().max(0.0) as usize;
        spikes_up(spin(self.t, k)).then(|| vec2(sonic.pos.x - sonic.facing, s.top))
    }

    /// The wall `collider` broken: where it was.
    pub fn break_wall(&mut self, world: &mut PhysWorld, collider: ColliderHandle) -> Option<Vec2> {
        let i = self.walls.iter().position(|w| w.collider == collider)?;
        let w = self.walls.remove(i);
        world.remove_body(w.body);
        Some(w.pos)
    }

    pub fn is_wall(&self, collider: ColliderHandle) -> bool {
        self.walls.iter().any(|w| w.collider == collider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridges_dip_most_under_the_character_and_least_at_their_ends() {
        assert_eq!(dips(5, None), vec![0.0; 5]);
        let d = dips(12, Some(5));
        let deepest = d.iter().cloned().fold(0.0, f32::max);
        assert_eq!(d[5], deepest, "{d:?}");
        assert!((deepest - 6.0 * DIP).abs() < 1e-3);
        assert!(d[0] < d[3] && d[11] < d[8], "{d:?}");
        // Near an end it hardly dips; a long bridge stops at the most.
        assert!((dips(12, Some(0))[0] - DIP).abs() < 1e-3);
        assert!(dips(40, Some(20)).iter().all(|&v| v <= MOST_DIP + 1e-3));
    }

    #[test]
    fn bridges_dip_and_swings_swing_in_the_world_and_ledges_crumble() {
        use crate::physics::borders::BorderMode;
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (3000.0, 900.0));
        let spec = LevelSpec {
            bridges: vec![Span::from_px(400.0, 400.0 + 12.0 * LOG, 600.0)],
            swings: vec![to_phys(1200.0, 200.0).into()],
            ledges: vec![Span::from_px(1600.0, 1600.0 + 4.0 * LEDGE_COLUMN, 600.0)],
            walls: vec![to_phys(2000.0, 600.0).into()],
            ..Default::default()
        };
        let mut th = Things::new(&spec, &mut w);
        assert_eq!(th.bridges[0].logs.len(), 12);
        let log = th.bridges[0].logs[6];
        // The character on the middle log: it goes down, the ends hardly.
        let mut s = Sonic::new(log.2 - vec2(0.0, 30.0));
        (s.grounded, s.on) = (true, Some(log.1));
        for _ in 0..40 {
            th.step(&mut w, &s, 1.0 / 60.0);
            w.step_fixed();
        }
        let y = |b: RigidBodyHandle| to_screen(0.0, w.bodies[b].translation().y).y;
        let (middle, end) = (y(th.bridges[0].logs[6].0), y(th.bridges[0].logs[0].0));
        assert!(middle > log.2.y + 5.0 * DIP, "the middle dips: {middle} from {}", log.2.y);
        assert!(end - th.bridges[0].logs[0].2.y < 2.0 * DIP, "the end hardly: {end}");
        // A quarter of a swing later the platform is well off to the side.
        let x = w.bodies[th.swings[0].body].translation().x * PPM;
        assert!((x - 1200.0).abs() > CHAIN * 0.5, "swung to {x}");
        // Standing on the ledge: it is gone a moment later; the wall breaks.
        let col = th.ledges[0].columns[1].body.unwrap().1;
        s.on = Some(col);
        th.step(&mut w, &s, 1.0 / 60.0);
        // Not at once: running on, the character keeps ahead of it.
        for _ in 0..20 {
            th.step(&mut w, &s, 1.0 / 60.0);
        }
        assert!(th.ledges[0].columns.iter().all(|c| c.body.is_some()));
        for _ in 0..30 {
            th.step(&mut w, &s, 1.0 / 60.0);
        }
        assert!(th.ledges[0].columns.iter().all(|c| c.body.is_none() && c.falling.is_some()));
        let wall = th.walls[0].collider;
        assert!(th.is_wall(wall) && th.break_wall(&mut w, wall).is_some() && th.walls.is_empty());
        // The bridge's logs and the swing; the ledge has fallen, the wall is gone.
        assert_eq!(th.grounds().len(), 13);
        th.remove(&mut w);
    }

    #[test]
    fn swings_go_both_ways_and_spikes_come_round() {
        let anchor = vec2(500.0, 100.0);
        let down = swing_at(anchor, 0.0);
        assert_eq!(down, anchor + vec2(0.0, CHAIN + SWING_PLATFORM.y / 2.0));
        let quarter = SWING_PERIOD / 4.0;
        assert!((swing_angle(quarter) - SWING_AMPLITUDE).abs() < 1e-4);
        assert!((swing_angle(3.0 * quarter) + SWING_AMPLITUDE).abs() < 1e-4);
        assert!(swing_at(anchor, swing_angle(quarter)).x > anchor.x + CHAIN * 0.8);
        // Up, then round under the log, then up again.
        assert!(spikes_up(spin(0.0, 1)));
        assert!(!spikes_up(spin(SPIN_PERIOD / 2.0, 1)));
        assert!(spikes_up(spin(SPIN_PERIOD, 1)));
        // Each piece an eighth of a turn on from the one before; a picture
        // later, the pattern has moved one piece along.
        assert!(((spin(0.0, 2) - spin(0.0, 1)).rem_euclid(1.0) - SPIN_STAGGER).abs() < 1e-5);
        let next = SPIN_PERIOD / 8.0;
        assert!((spin(next, 2) - spin(0.0, 1)).abs() < 1e-4);
        assert_eq!((spin_picture(0.0), spin_picture(next * 1.01), spin_picture(SPIN_PERIOD * 1.001)), (0, 1, 0));
    }
}
