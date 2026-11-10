//! Levels made from a seed: pieces of ground laid one after the other
//! (flats, hills, slopes, cliffs up with a spring and drops down, loops),
//! pits to get over (a jump, a bridge, a swinging platform, platforms, a
//! ledge that crumbles, a log with spikes going round it), walls to roll
//! through, and stretches with two paths: one up on platforms (rings and
//! monitors) and one below, where the danger is. The endless run is made
//! of them as it goes, and a random act is a run of them from a start to a
//! goal. The same seed makes the same level again.
//!
//! Everything is in design px (see [`Builder`]): the ground's top stays
//! between [`HIGH`] and [`LOW`], [`BASE`] where it starts.

use super::{Builder, LEAF, SKY, WOOD};
use crate::game::{EnemyKind, MonitorKind};

/// Where the ground starts, and how high and low its top goes (design px).
pub const BASE: f32 = 560.0;
pub const HIGH: f32 = 380.0;
pub const LOW: f32 = 620.0;
/// The widest pit the character has to jump without help (px).
pub const MOST_JUMP: f32 = 210.0;

/// A small seeded random generator (xorshift), so a level can be made again.
#[derive(Clone, Debug)]
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() % 1_000_000) as f32 / 1_000_000.0 * (hi - lo)
    }

    pub fn below(&mut self, n: u32) -> u32 {
        (self.next() % n.max(1) as u64) as u32
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.range(0.0, 1.0) < p
    }
}

/// The colours of a zone's ground and of the strip along its top (drawn
/// with the zone's tiles instead when there is a zone sheet).
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub ground: [u8; 3],
    pub edge: [u8; 3],
}

/// The kinds of pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Flat,
    Hills,
    Slope,
    Gap,
    Bridge,
    Swing,
    Platforms,
    Crumble,
    SpikeLog,
    Cliff,
    Drop,
    Loop,
    Walls,
    TwoPaths,
}

impl Kind {
    pub const ALL: [Kind; 14] = [
        Kind::Flat,
        Kind::Hills,
        Kind::Slope,
        Kind::Gap,
        Kind::Bridge,
        Kind::Swing,
        Kind::Platforms,
        Kind::Crumble,
        Kind::SpikeLog,
        Kind::Cliff,
        Kind::Drop,
        Kind::Loop,
        Kind::Walls,
        Kind::TwoPaths,
    ];

    /// How often it comes up.
    fn weight(self) -> u32 {
        match self {
            Kind::Flat | Kind::Hills => 3,
            Kind::Gap | Kind::Bridge | Kind::TwoPaths => 3,
            Kind::Slope | Kind::Swing | Kind::Platforms | Kind::Crumble => 2,
            Kind::SpikeLog | Kind::Cliff | Kind::Drop | Kind::Loop | Kind::Walls => 2,
        }
    }
}

/// How a pit is got over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossing {
    Jump,
    Bridge,
    Swing,
    Platforms,
    Crumble,
    SpikeLog,
}

/// Somewhere things can stand: from x0 to x1 on ground at y (design px),
/// up on the upper path or not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floor {
    pub x0: f32,
    pub x1: f32,
    pub y: f32,
    pub upper: bool,
}

/// A piece laid: how wide it is, where its ground ends, where things can
/// stand on it, and its pits (from, to, how they are crossed).
#[derive(Clone, Debug, Default)]
pub struct Piece {
    pub width: f32,
    pub end: f32,
    pub floors: Vec<Floor>,
    /// (What the tests check every pit by.)
    #[allow(dead_code)]
    pub pits: Vec<(f32, f32, Option<Crossing>)>,
}

fn floor(x0: f32, x1: f32, y: f32) -> Floor {
    Floor { x0, x1, y, upper: false }
}

fn ring_row(b: &mut Builder, x: f32, y: f32, n: usize) {
    for k in 0..n {
        b.ring(x + k as f32 * 36.0, y);
    }
}

/// Rings in an arc from (x0, y) to (x1, y), `h` px up in the middle.
fn ring_arc(b: &mut Builder, x0: f32, x1: f32, y: f32, h: f32, n: usize) {
    for k in 0..n {
        let t = k as f32 / (n - 1).max(1) as f32;
        b.ring(x0 + (x1 - x0) * t, y - h * 4.0 * t * (1.0 - t));
    }
}

/// The next kind of piece after `last`, with the ground's top at `y`: not
/// the same twice, and keeping the ground between [`HIGH`] and [`LOW`].
pub fn pick(rng: &mut Rng, last: Kind, y: f32) -> Kind {
    let can = |k: Kind| match k {
        Kind::Cliff => y - 120.0 >= HIGH,
        Kind::Drop => y + 80.0 <= LOW,
        _ => k != last,
    };
    let kinds: Vec<Kind> = Kind::ALL.into_iter().filter(|&k| k != last && can(k)).collect();
    let total: u32 = kinds.iter().map(|k| k.weight()).sum();
    let mut n = rng.below(total);
    for k in &kinds {
        if n < k.weight() {
            return *k;
        }
        n -= k.weight();
    }
    Kind::Flat
}

/// The start: flat ground, `w` px.
pub fn start(b: &mut Builder, x: f32, y: f32, w: f32, look: &Look) -> Piece {
    b.ground(x, x + w, y, look.ground, look.edge);
    Piece { width: w, end: y, floors: vec![floor(x + 80.0, x + w - 80.0, y)], pits: vec![] }
}

/// Lay a piece of `kind` from x with the ground's top at y.
pub fn piece(b: &mut Builder, kind: Kind, x: f32, y: f32, look: &Look, rng: &mut Rng) -> Piece {
    let (g, e) = (look.ground, look.edge);
    // Ground either side of a pit `gap` wide after a `pad` of ground.
    let pit = |b: &mut Builder, pad: f32, gap: f32| {
        b.ground(x, x + pad, y, g, e);
        b.ground(x + pad + gap, x + 2.0 * pad + gap, y, g, e);
        (x + pad, x + pad + gap)
    };
    let pads = |pad: f32, gap: f32| {
        vec![floor(x + 40.0, x + pad - 60.0, y), floor(x + pad + gap + 60.0, x + 2.0 * pad + gap - 40.0, y)]
    };
    match kind {
        Kind::Flat => {
            let w = rng.range(600.0, 900.0).round();
            b.ground(x, x + w, y, g, e);
            ring_row(b, x + 160.0, y - 40.0, 5);
            Piece { width: w, end: y, floors: vec![floor(x + 80.0, x + w - 80.0, y)], pits: vec![] }
        }
        Kind::Hills => {
            let (w, h) = (rng.range(800.0, 1200.0).round(), rng.range(60.0, 110.0));
            b.hill(x, x + w, y, h, g, e);
            for k in 0..5 {
                let t = 0.3 + 0.1 * k as f32;
                b.ring(x + w * t, y - h * (std::f32::consts::PI * t).sin().powi(2) - 45.0);
            }
            let floors = vec![floor(x + 30.0, x + 110.0, y), floor(x + w - 110.0, x + w - 30.0, y)];
            Piece { width: w, end: y, floors, pits: vec![] }
        }
        Kind::Slope => {
            let w = rng.range(500.0, 800.0).round();
            let up = if y - 100.0 < HIGH {
                false
            } else if y + 80.0 > LOW {
                true
            } else {
                rng.chance(0.5)
            };
            let d = rng.range(80.0, 150.0);
            let end = if up { (y - d).max(HIGH) } else { (y + d).min(LOW) };
            b.terrain(x, x + w, y, end, 0.0, g, e);
            ring_arc(b, x + 120.0, x + w - 120.0, (y + end) / 2.0 - 50.0, 20.0, 5);
            Piece { width: w, end, floors: vec![floor(x + w - 90.0, x + w - 20.0, end)], pits: vec![] }
        }
        Kind::Gap => {
            let (pad, gap) = (200.0, rng.range(140.0, MOST_JUMP).round());
            let (a, z) = pit(b, pad, gap);
            ring_arc(b, a - 40.0, z + 40.0, y - 60.0, 70.0, 5);
            Piece { width: 2.0 * pad + gap, end: y, floors: pads(pad, gap), pits: vec![(a, z, Some(Crossing::Jump))] }
        }
        Kind::Bridge => {
            let gap = (rng.range(220.0, 380.0) / 24.0).round() * 24.0;
            let pad = 160.0;
            let (a, z) = pit(b, pad, gap);
            b.bridge(a, z, y);
            ring_row(b, a + 30.0, y - 40.0, ((gap - 60.0) / 36.0) as usize + 1);
            Piece { width: 2.0 * pad + gap, end: y, floors: pads(pad, gap), pits: vec![(a, z, Some(Crossing::Bridge))] }
        }
        Kind::Swing => {
            let (pad, gap) = (200.0, rng.range(360.0, 440.0).round());
            let (a, z) = pit(b, pad, gap);
            b.swing((a + z) / 2.0, y - 230.0);
            ring_arc(b, a + 20.0, z - 20.0, y - 110.0, 60.0, 6);
            Piece { width: 2.0 * pad + gap, end: y, floors: pads(pad, gap), pits: vec![(a, z, Some(Crossing::Swing))] }
        }
        Kind::Platforms => {
            let (pad, gap) = (200.0, rng.range(480.0, 600.0).round());
            let (a, z) = pit(b, pad, gap);
            let (h1, h2) = (rng.range(50.0, 90.0), rng.range(40.0, 100.0));
            let p1 = (a + gap * 0.2, a + gap * 0.42, y - h1);
            let p2 = (a + gap * 0.58, a + gap * 0.8, y - h2);
            let mut floors = pads(pad, gap);
            for (x0, x1, top) in [p1, p2] {
                b.platform(x0, x1, top, WOOD);
                ring_row(b, x0 + 20.0, top - 40.0, 3);
                // The ledge's top is half its thickness above its line.
                floors.push(Floor { x0: x0 + 30.0, x1: x1 - 30.0, y: top - 8.0, upper: true });
            }
            Piece { width: 2.0 * pad + gap, end: y, floors, pits: vec![(a, z, Some(Crossing::Platforms))] }
        }
        Kind::Crumble => {
            let (pad, gap) = (160.0, rng.range(300.0, 420.0).round());
            let (a, z) = pit(b, pad, gap);
            b.crumbling(a, z, y);
            ring_row(b, a + 30.0, y - 40.0, ((gap - 60.0) / 36.0) as usize + 1);
            Piece {
                width: 2.0 * pad + gap,
                end: y,
                floors: pads(pad, gap),
                pits: vec![(a, z, Some(Crossing::Crumble))],
            }
        }
        Kind::SpikeLog => {
            let (pad, gap) = (180.0, rng.range(240.0, 360.0).round());
            let (a, z) = pit(b, pad, gap);
            b.spike_log(a, z, y);
            ring_arc(b, a, z, y - 70.0, 60.0, 5);
            Piece {
                width: 2.0 * pad + gap,
                end: y,
                floors: pads(pad, gap),
                pits: vec![(a, z, Some(Crossing::SpikeLog))],
            }
        }
        Kind::Cliff => {
            // Up a cliff on a spring.
            let (w1, w2) = (rng.range(400.0, 520.0).round(), rng.range(500.0, 700.0).round());
            let top = (y - rng.range(120.0, 170.0)).max(HIGH);
            b.ground(x, x + w1, y, g, e);
            b.spring(x + w1 - 80.0, y);
            for k in 0..3 {
                b.ring(x + w1 - 80.0, y - 110.0 - k as f32 * 45.0);
            }
            b.ground(x + w1, x + w1 + w2, top, g, e);
            ring_row(b, x + w1 + 80.0, top - 40.0, 4);
            let floors = vec![floor(x + 60.0, x + w1 - 160.0, y), floor(x + w1 + 60.0, x + w1 + w2 - 60.0, top)];
            Piece { width: w1 + w2, end: top, floors, pits: vec![] }
        }
        Kind::Drop => {
            let (w1, w2) = (rng.range(300.0, 450.0).round(), rng.range(500.0, 700.0).round());
            let low = (y + rng.range(80.0, 140.0)).min(LOW);
            b.ground(x, x + w1, y, g, e);
            b.ground(x + w1, x + w1 + w2, low, g, e);
            ring_arc(b, x + w1 - 40.0, x + w1 + 200.0, y - 40.0, 40.0, 5);
            let floors = vec![floor(x + 60.0, x + w1 - 60.0, y), floor(x + w1 + 200.0, x + w1 + w2 - 60.0, low)];
            Piece { width: w1 + w2, end: low, floors, pits: vec![] }
        }
        Kind::Loop => {
            let w = 1000.0;
            b.ground(x, x + w, y, g, e);
            b.dash_panel(x + 150.0, x + 270.0, y);
            let r = 110.0;
            b.sonic_loop(x + 520.0, y - r, r, g);
            for k in 0..8 {
                let a = (k as f32 / 8.0 * 360.0).to_radians();
                b.ring(x + 520.0 + a.cos() * (r - 42.0), y - r + a.sin() * (r - 42.0));
            }
            let floors = vec![floor(x + 30.0, x + 120.0, y), floor(x + 760.0, x + w - 60.0, y)];
            Piece { width: w, end: y, floors, pits: vec![] }
        }
        Kind::Walls => {
            let w = 1000.0;
            b.ground(x, x + w, y, g, e);
            b.breakable_wall(x + 360.0, y);
            ring_row(b, x + 430.0, y - 40.0, 4);
            b.rock(x + 760.0, y);
            Piece { width: w, end: y, floors: vec![floor(x + 440.0, x + 660.0, y)], pits: vec![] }
        }
        Kind::TwoPaths => {
            // Below, the ground (and the danger); a spring up to ledges
            // with rings and monitors.
            let w = rng.range(1400.0, 1700.0).round();
            b.ground(x, x + w, y, g, e);
            b.spring(x + 140.0, y);
            let up = (y - 230.0).max(HIGH - 120.0);
            let mut floors = vec![floor(x + 300.0, x + w - 120.0, y)];
            let mut at = x + 260.0;
            for k in 0..3 {
                let len = rng.range(260.0, 360.0).round();
                let end = (at + len).min(x + w - 120.0);
                b.platform(at, end, up - k as f32 * 20.0, if k % 2 == 0 { WOOD } else { SKY });
                ring_row(b, at + 30.0, up - k as f32 * 20.0 - 40.0, ((end - at - 60.0) / 36.0) as usize + 1);
                floors.push(Floor { x0: at + 40.0, x1: end - 40.0, y: up - k as f32 * 20.0 - 8.0, upper: true });
                at = end + rng.range(80.0, 120.0);
                if at > x + w - 300.0 {
                    break;
                }
            }
            Piece { width: w, end: y, floors, pits: vec![] }
        }
    }
}

/// Something dangerous (or a reward) for a level: a badnik walking on the
/// ground at (x, y), one flying there, spikes, or a monitor, standing on
/// the ground at y (design px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Danger {
    Walker(EnemyKind, f32, f32),
    Flyer(f32, f32),
    Spikes(f32, f32),
    Monitor(MonitorKind, f32, f32),
}

/// What goes on `floors`, `level` the further on (0 at first): badniks and
/// spikes below, monitors and the odd ring box up on the upper path.
/// The least ground (px) a badnik is put on.
const ROOMY: f32 = 300.0;

pub fn dangers(floors: &[Floor], level: u32, rng: &mut Rng) -> Vec<Danger> {
    let mut out = vec![];
    for f in floors {
        let (x0, x1) = (f.x0, f.x1);
        if x1 - x0 < 40.0 {
            continue;
        }
        let x = rng.range(x0, x1);
        if f.upper {
            if rng.chance(0.45) {
                let kinds = [
                    MonitorKind::Rings,
                    MonitorKind::Rings,
                    MonitorKind::Shield,
                    MonitorKind::Shoes,
                    MonitorKind::Invincible,
                ];
                out.push(Danger::Monitor(kinds[rng.below(kinds.len() as u32) as usize], x, f.y));
            }
            continue;
        }
        let busy = (0.35 + level as f32 * 0.08).min(0.85);
        if !rng.chance(busy) {
            continue;
        }
        // Badniks only where there is room: on a bit of ground between pits
        // (walkers go to its edges) they would be where the character lands,
        // and being hit there throws them back into the pit.
        let roomy = x1 - x0 >= ROOMY;
        match rng.below(10) {
            0..=4 if roomy => {
                let kind = if level >= 3 && rng.chance(0.4) { EnemyKind::Crab } else { EnemyKind::Crawler };
                out.push(Danger::Walker(kind, x, f.y));
            }
            5 | 6 if level >= 1 && roomy => out.push(Danger::Flyer(x, f.y - rng.range(120.0, 200.0))),
            7 | 8 if x1 - x0 > 160.0 => out.push(Danger::Spikes(x, f.y)),
            _ if rng.chance(0.3) => out.push(Danger::Monitor(MonitorKind::Rings, x, f.y)),
            _ => {}
        }
    }
    out
}

/// A whole act from `seed`: a start, `n` pieces with their dangers, a star
/// post every few, and the goal. Returns its name.
pub fn act(b: &mut Builder, seed: u64, n: usize) -> String {
    let mut rng = Rng::new(seed);
    let look = Look { ground: [150, 98, 62], edge: LEAF };
    let mut y = BASE;
    let first = start(b, -400.0, y, 1100.0, &look);
    b.sonic(140.0, y - 40.0);
    ring_row(b, 300.0, y - 40.0, 5);
    let mut x = -400.0 + first.width;
    let mut last = Kind::Flat;
    for i in 0..n {
        let kind = pick(&mut rng, last, y);
        let p = piece(b, kind, x, y, &look, &mut rng);
        for d in dangers(&p.floors, (i / 3) as u32, &mut rng) {
            match d {
                Danger::Walker(kind, x, y) => b.walker(kind, x, y),
                Danger::Flyer(x, y) => b.enemy(EnemyKind::Flyer, x, y),
                Danger::Spikes(x, y) => b.spikes(x, y, 2),
                Danger::Monitor(kind, x, y) => b.monitor(kind, x, y),
            }
        }
        // A star post after every few pieces, on the first ground below.
        if i % 4 == 3 {
            if let Some(f) = p.floors.iter().find(|f| !f.upper) {
                b.post((f.x0 + f.x1) / 2.0, f.y);
            }
        }
        x += p.width;
        y = p.end;
        last = kind;
    }
    b.ground(x, x + 1100.0, y, look.ground, look.edge);
    ring_row(b, x + 120.0, y - 40.0, 6);
    b.monitor(MonitorKind::Rings, x + 420.0, y);
    b.goal(x + 760.0, y);
    let name = format!("Random act {}", seed % 100_000);
    b.scene.game.get_or_insert_with(Default::default).name = name.clone();
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lay(seed: u64, n: usize) -> (Vec<(Kind, Piece)>, Builder) {
        let mut b = Builder::at(0.0, 1440.0);
        let mut rng = Rng::new(seed);
        let look = Look { ground: [1, 2, 3], edge: LEAF };
        let (mut x, mut y, mut last) = (0.0, BASE, Kind::Flat);
        let mut out = vec![];
        for _ in 0..n {
            let kind = pick(&mut rng, last, y);
            let p = piece(&mut b, kind, x, y, &look, &mut rng);
            x += p.width;
            y = p.end;
            last = kind;
            out.push((kind, p));
        }
        (out, b)
    }

    #[test]
    fn pieces_keep_the_ground_in_its_band_and_every_pit_can_be_crossed() {
        for seed in 1..40 {
            let (pieces, _) = lay(seed, 40);
            let mut last = None;
            for (kind, p) in &pieces {
                assert!((HIGH..=LOW).contains(&p.end), "{kind:?} ends at {}", p.end);
                assert!(p.width > 300.0, "{kind:?} is {} wide", p.width);
                assert_ne!(Some(*kind), last, "twice in a row");
                last = Some(*kind);
                for &(a, z, how) in &p.pits {
                    let gap = z - a;
                    match how {
                        Some(Crossing::Jump) => assert!(gap <= MOST_JUMP, "a jump over {gap}"),
                        Some(_) => assert!(gap <= 600.0, "{how:?} over {gap}"),
                        None => panic!("a pit with no way over"),
                    }
                }
                for f in &p.floors {
                    assert!(f.x0 < f.x1, "{kind:?}: {f:?}");
                }
            }
        }
    }

    #[test]
    fn every_kind_comes_up_and_two_paths_have_a_way_up() {
        let (pieces, _) = lay(7, 200);
        for k in Kind::ALL {
            assert!(pieces.iter().any(|(kind, _)| *kind == k), "{k:?} never comes up");
        }
        let (_, two) = pieces.iter().find(|(k, _)| *k == Kind::TwoPaths).unwrap();
        let below = two.floors.iter().find(|f| !f.upper).unwrap();
        assert!(two.floors.iter().any(|f| f.upper && below.y - f.y >= 190.0), "{:?}", two.floors);
    }

    #[test]
    fn a_seed_makes_the_same_act_again() {
        let made = |seed| {
            let mut b = Builder::at(0.0, 1440.0);
            let name = act(&mut b, seed, 12);
            (name, serde_json::to_string(&b.scene).unwrap())
        };
        let (a, b) = (made(12345), made(12345));
        assert_eq!(a, b);
        assert_ne!(a.1, made(54321).1);
        let mut b = Builder::at(0.0, 1440.0);
        act(&mut b, 99, 16);
        let g = b.scene.game.unwrap();
        assert!(b.scene.sonic.is_some() && g.goal.is_some() && g.posts.len() >= 3);
        assert!(!g.enemies.is_empty() && !g.monitors.is_empty());
        assert!(g.name.starts_with("Random act"));
    }

    #[test]
    fn dangers_stay_on_their_floors_with_monitors_up_top() {
        // Room for badniks, a ledge up top, and a short bit between pits.
        let floors = [
            floor(100.0, 400.0, 500.0),
            Floor { x0: 600.0, x1: 800.0, y: 300.0, upper: true },
            floor(1000.0, 1100.0, 500.0),
        ];
        let mut rng = Rng::new(3);
        for _ in 0..200 {
            for d in dangers(&floors, 5, &mut rng) {
                match d {
                    Danger::Walker(_, x, y) | Danger::Spikes(x, y) => {
                        assert!((100.0..=400.0).contains(&x) && y == 500.0)
                    }
                    Danger::Flyer(x, y) => assert!((100.0..=400.0).contains(&x) && y < 500.0),
                    Danger::Monitor(_, x, y) => assert!(
                        ((100.0..=800.0).contains(&x) || (1000.0..=1100.0).contains(&x)) && (y == 500.0 || y == 300.0)
                    ),
                }
            }
        }
    }
}
