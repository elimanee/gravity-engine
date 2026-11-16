//! Levels made from a seed as they are needed. The ground is generated as
//! it goes: flats, slopes, hills and dips of any length and height, the
//! ground drifting up and down towards heights picked at random. Between
//! runs of it comes something chosen at random, never the same as the
//! last few: a pit of any width (landing higher, lower or level) or a run
//! of them, a bridge, a swinging platform, stepping stones in the air, a
//! ledge that crumbles, a log with spikes, a cliff up on a spring, a drop,
//! a loop, walls to roll through, a dip with spikes at the bottom, a dash
//! panel and a long jump, or a stretch on two levels: a spring up to a
//! route of ledges (rings and monitors) over the ground below, where the
//! danger is. The endless run lays them as Sonic goes, a random act is a
//! run of them from a start to a goal. The same seed makes the same level
//! again; every pit can be got over.
//!
//! Everything is in design px (see [`Builder`]): the ground's top stays
//! between [`HIGH`] and [`LOW`], [`BASE`] where it starts.

use super::{Builder, LEAF, SKY, WOOD};
use crate::game::{EnemyKind, MonitorKind};
use std::f32::consts::PI;

/// Where the ground starts, and how high and low its top goes (design px).
pub const BASE: f32 = 560.0;
pub const HIGH: f32 = 380.0;
pub const LOW: f32 = 620.0;
/// The widest pit the character has to jump without help, level (px).
pub const MOST_JUMP: f32 = 210.0;
/// The most a jump has to climb (px).
pub const MOST_RISE: f32 = 90.0;
/// The widest pit jumped after a dash panel (px).
pub const MOST_LONG_JUMP: f32 = 320.0;
/// The steepest the ground gets (rise over run).
pub const STEEPEST: f32 = 0.7;
/// The least ground (px) a badnik is put on.
const ROOMY: f32 = 300.0;
/// A ledge's top is half its thickness above its line.
const LEDGE_HALF: f32 = 8.0;

/// The widest gap a jump gets over going up `rise` px (down when negative).
pub fn reach(rise: f32) -> f32 {
    if rise > 0.0 {
        MOST_JUMP - rise
    } else {
        MOST_JUMP + (-rise).min(120.0) * 0.5
    }
}

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

/// What comes after a run of ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    Gap,
    Gaps,
    Bridge,
    Swing,
    Stepping,
    Crumble,
    SpikeLog,
    Cliff,
    Drop,
    Loop,
    Walls,
    SpikeDip,
    LongJump,
    Upper,
}

impl Feature {
    pub const ALL: [Feature; 14] = [
        Feature::Gap,
        Feature::Gaps,
        Feature::Bridge,
        Feature::Swing,
        Feature::Stepping,
        Feature::Crumble,
        Feature::SpikeLog,
        Feature::Cliff,
        Feature::Drop,
        Feature::Loop,
        Feature::Walls,
        Feature::SpikeDip,
        Feature::LongJump,
        Feature::Upper,
    ];

    /// How often it comes up, `level` the further on (pits more and more).
    fn weight(self, level: u32) -> u32 {
        let hard = level.min(6);
        match self {
            Feature::Gap | Feature::Upper => 4,
            Feature::Bridge | Feature::Stepping => 3,
            Feature::Gaps => 2 + hard / 2,
            Feature::Crumble => 1 + hard / 2,
            Feature::SpikeLog | Feature::SpikeDip => 1 + hard / 3,
            Feature::Swing | Feature::Cliff | Feature::Drop | Feature::Loop | Feature::Walls | Feature::LongJump => 2,
        }
    }
}

/// How a gap is got over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crossing {
    Jump,
    /// Along a ledge in the air (between jumps).
    Ledge,
    LongJump,
    Bridge,
    Swing,
    Crumble,
    SpikeLog,
}

/// A way over a gap: from (x0, y0) to (x1, y1), the tops either side (for
/// a ledge, the ledge itself).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pit {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub how: Crossing,
}

/// Somewhere things can stand: from x0 to x1 on level ground at y (design
/// px), up on a ledge or not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floor {
    pub x0: f32,
    pub x1: f32,
    pub y: f32,
    pub upper: bool,
}

/// A stretch laid: how wide it is and what came after its ground, where
/// things can stand, its own dangers (spikes in a dip), the ground's top
/// strip by strip with the ways over the gaps between them, and the jumps
/// along a route of ledges up above.
#[derive(Clone, Debug, Default)]
pub struct Section {
    pub width: f32,
    pub feature: Option<Feature>,
    pub floors: Vec<Floor>,
    pub dangers: Vec<Danger>,
    pub ground: Vec<Vec<(f32, f32)>>,
    pub pits: Vec<Pit>,
    pub climbs: Vec<Pit>,
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

/// Ground being laid: its top since the last gap, and what it makes.
struct Lay<'a> {
    b: &'a mut Builder,
    look: Look,
    pts: Vec<(f32, f32)>,
    /// Where it has got to.
    end: (f32, f32),
    s: Section,
}

impl<'a> Lay<'a> {
    fn new(b: &'a mut Builder, look: Look, x: f32, y: f32) -> Lay<'a> {
        Lay { b, look, pts: vec![(x, y)], end: (x, y), s: Section::default() }
    }

    fn push(&mut self, p: (f32, f32)) {
        if self.pts.is_empty() {
            self.pts.push(self.end);
        }
        self.pts.push(p);
        self.end = p;
    }

    /// Straight on to (x, y) (a point every 200 px at most).
    fn line(&mut self, x: f32, y: f32) {
        let (x0, y0) = self.end;
        let n = ((x - x0) / 200.0).ceil().max(1.0) as usize;
        for k in 1..=n {
            let t = k as f32 / n as f32;
            self.push((x0 + (x - x0) * t, y0 + (y - y0) * t));
        }
    }

    /// On to x, the top at `f(t)` as t goes from 0 to 1 (a point every 40 px).
    fn curve(&mut self, x: f32, f: impl Fn(f32) -> f32) {
        let x0 = self.end.0;
        let n = ((x - x0) / 40.0).ceil().max(2.0) as usize;
        for k in 1..=n {
            let t = k as f32 / n as f32;
            self.push((x0 + (x - x0) * t, f(t)));
        }
    }

    /// The ground stops here (at a gap or a step): lay it.
    fn cut(&mut self) {
        let pts = std::mem::take(&mut self.pts);
        if pts.len() >= 2 {
            self.b.surface(&pts, self.look.ground, self.look.edge);
            self.s.ground.push(pts);
        }
    }

    /// Ground again from (x, y).
    fn from(&mut self, x: f32, y: f32) {
        self.cut();
        self.end = (x, y);
        self.pts.push(self.end);
    }

    /// The top at x, on the ground laid since the last gap.
    fn height(&self, x: f32) -> f32 {
        let pts = &self.pts;
        let Some(i) = pts.iter().position(|p| p.0 >= x) else { return self.end.1 };
        if i == 0 {
            return pts[0].1;
        }
        let (a, b) = (pts[i - 1], pts[i]);
        a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0).max(1e-3)
    }

    fn floor(&mut self, x0: f32, x1: f32, y: f32, upper: bool) {
        if x1 - x0 >= 20.0 {
            self.s.floors.push(Floor { x0, x1, y, upper });
        }
    }

    fn way(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, how: Crossing) {
        self.s.pits.push(Pit { x0, y0, x1, y1, how });
    }

    fn ring_row(&mut self, x: f32, y: f32, n: usize) {
        for k in 0..n {
            self.b.ring(x + k as f32 * 36.0, y);
        }
    }

    /// Rings in an arc from (x0, y) to (x1, y), `h` px up in the middle.
    fn ring_arc(&mut self, x0: f32, x1: f32, y: f32, h: f32, n: usize) {
        for k in 0..n {
            let t = k as f32 / (n - 1).max(1) as f32;
            self.b.ring(x0 + (x1 - x0) * t, y - h * 4.0 * t * (1.0 - t));
        }
    }

    /// Rings along the ground from x0 to x1, 45 px above it.
    fn rings_along(&mut self, x0: f32, x1: f32, n: usize) {
        for k in 0..n {
            let x = x0 + (x1 - x0) * (k as f32 + 0.5) / n as f32;
            let y = self.height(x) - 45.0;
            self.b.ring(x, y);
        }
    }

    /// A ledge in the air from x0 to x1, its top at `top`.
    fn ledge(&mut self, x0: f32, x1: f32, top: f32, rgb: [u8; 3]) {
        self.b.platform(x0, x1, top + LEDGE_HALF, rgb);
    }

    fn finish(mut self, x: f32) -> Section {
        self.cut();
        self.s.width = self.end.0 - x;
        self.s
    }
}

/// A course: lays one section after another, the ground going on from
/// where the last one ended.
#[derive(Clone, Debug)]
pub struct Course {
    rng: Rng,
    /// The ground's top where the next section starts (design px).
    pub y: f32,
    /// The height the ground is heading for.
    target: f32,
    /// What came lately (not to come again too soon).
    recent: Vec<Feature>,
}

impl Course {
    pub fn new(seed: u64) -> Course {
        Course { rng: Rng::new(seed), y: BASE, target: BASE, recent: vec![] }
    }

    /// The start: flat ground, `w` px from x.
    pub fn start(&mut self, b: &mut Builder, x: f32, w: f32, look: &Look) -> Section {
        let mut lay = Lay::new(b, *look, x, self.y);
        lay.line(x + w, self.y);
        lay.floor(x + 80.0, x + w - 80.0, self.y, false);
        lay.finish(x)
    }

    /// The next section from x: a run of ground, then a feature; `level`
    /// is how far on it is (0 at first).
    pub fn section(&mut self, b: &mut Builder, x: f32, look: &Look, level: u32) -> Section {
        let mut lay = Lay::new(b, *look, x, self.y);
        let run = x + self.rng.range(250.0, 900.0);
        while lay.end.0 < run {
            self.segment(&mut lay);
        }
        let f = self.pick(level, lay.end.1);
        self.feature(&mut lay, f, level);
        self.recent.push(f);
        if self.recent.len() > 3 {
            self.recent.remove(0);
        }
        self.y = lay.end.1;
        let mut s = lay.finish(x);
        s.feature = Some(f);
        s
    }

    /// Where the ground goes next (px down, or up when negative).
    fn drift(&mut self, y: f32) -> f32 {
        if (self.target - y).abs() < 30.0 || self.rng.chance(0.25) {
            self.target = self.rng.range(HIGH + 20.0, LOW - 10.0);
        }
        let mut dy = (self.target - y).clamp(-150.0, 150.0);
        if dy.abs() < 30.0 {
            dy = if y - 60.0 >= HIGH { -60.0 } else { 60.0 };
        }
        (y + dy).clamp(HIGH, LOW) - y
    }

    /// A bit of the run of ground: flat, a slope, a hill or a dip.
    fn segment(&mut self, lay: &mut Lay) {
        let (x, y) = lay.end;
        match self.rng.below(10) {
            0..=3 => {
                let len = self.rng.range(160.0, 560.0).round();
                lay.line(x + len, y);
                lay.floor(x + 40.0, x + len - 40.0, y, false);
                if self.rng.chance(0.5) {
                    lay.ring_row(x + 40.0, y - 40.0, ((len - 80.0) / 36.0).clamp(1.0, 6.0) as usize);
                }
            }
            4..=6 => {
                let dy = self.drift(y);
                let len = (dy.abs() * 1.5 / STEEPEST).max(200.0) + self.rng.range(0.0, 260.0).round();
                lay.curve(x + len, |t| y + dy * t * t * (3.0 - 2.0 * t));
                if self.rng.chance(0.4) {
                    lay.rings_along(x, x + len, 4);
                }
            }
            7 | 8 if y - HIGH >= 30.0 => {
                let h = self.rng.range(30.0, 100.0).min(y - HIGH);
                let len = (h * PI / STEEPEST).max(260.0) + self.rng.range(0.0, 300.0).round();
                lay.curve(x + len, |t| y - h * (PI * t).sin().powi(2));
                if self.rng.chance(0.5) {
                    lay.rings_along(x + len * 0.2, x + len * 0.8, 5);
                }
            }
            _ if LOW - y >= 25.0 => {
                let d = self.rng.range(25.0, 70.0).min(LOW - y);
                let len = (d * PI / STEEPEST).max(240.0) + self.rng.range(0.0, 200.0).round();
                lay.curve(x + len, |t| y + d * (PI * t).sin().powi(2));
                if self.rng.chance(0.3) {
                    lay.rings_along(x + len * 0.3, x + len * 0.7, 3);
                }
            }
            _ => lay.line(x + 200.0, y),
        }
    }

    /// The next feature, with the ground's top at y: not one of the last
    /// few, and only what fits between [`HIGH`] and [`LOW`].
    fn pick(&mut self, level: u32, y: f32) -> Feature {
        let fits = |f: Feature| match f {
            Feature::Cliff => y - 130.0 >= HIGH,
            Feature::Drop => y + 80.0 <= LOW,
            Feature::SpikeDip => y + 50.0 <= LOW,
            _ => true,
        };
        let kinds: Vec<Feature> = Feature::ALL.into_iter().filter(|f| !self.recent.contains(f) && fits(*f)).collect();
        let total: u32 = kinds.iter().map(|f| f.weight(level)).sum();
        let mut n = self.rng.below(total);
        for f in &kinds {
            if n < f.weight(level) {
                return *f;
            }
            n -= f.weight(level);
        }
        Feature::Gap
    }

    /// A height to land on, `rise` px up at most and `fall` down.
    fn landing(&mut self, y: f32, rise: f32, fall: f32) -> f32 {
        match self.rng.below(4) {
            0 => y - self.rng.range(0.0, rise.min(MOST_RISE)).min(y - HIGH),
            1 => y + self.rng.range(0.0, fall).min(LOW - y),
            _ => y,
        }
    }

    /// Jump a gap from where the ground has got to, landing at `y1`.
    fn jump(&mut self, lay: &mut Lay, gap: f32, y1: f32) {
        let (x, y) = lay.end;
        if self.rng.chance(0.6) {
            lay.ring_arc(x - 30.0, x + gap + 30.0, y.min(y1) - 60.0, 60.0, 5);
        }
        lay.from(x + gap, y1);
        lay.way(x, y, x + gap, y1, Crossing::Jump);
    }

    /// Level ground on from where it has got to, `lo` to `hi` px long.
    fn flat(&mut self, lay: &mut Lay, lo: f32, hi: f32) {
        let (x, y) = lay.end;
        let len = self.rng.range(lo, hi).round();
        lay.line(x + len, y);
        lay.floor(x + 60.0, x + len - 40.0, y, false);
    }

    /// A gap made of something (a bridge…) `lo` to `hi` px wide, level.
    fn spanned(&mut self, lay: &mut Lay, lo: f32, hi: f32, how: Crossing) -> (f32, f32, f32) {
        let (x, y) = lay.end;
        let gap = self.rng.range(lo, hi).round();
        lay.from(x + gap, y);
        lay.way(x, y, x + gap, y, how);
        (x, x + gap, y)
    }

    fn feature(&mut self, lay: &mut Lay, f: Feature, level: u32) {
        let (x, y) = lay.end;
        match f {
            Feature::Gap => {
                let y1 = self.landing(y, 70.0, 90.0);
                let gap = self.rng.range(110.0, reach(y - y1) - 10.0).round();
                self.jump(lay, gap, y1);
                self.flat(lay, 220.0, 420.0);
            }
            Feature::Gaps => {
                for _ in 0..2 + self.rng.below(2 + level.min(2)) {
                    let y0 = lay.end.1;
                    let y1 = self.landing(y0, 50.0, 50.0);
                    let gap = self.rng.range(100.0, reach(y0 - y1) - 20.0).round();
                    self.jump(lay, gap, y1);
                    self.flat(lay, 110.0, 200.0);
                }
                self.flat(lay, 200.0, 360.0);
            }
            Feature::Bridge => {
                let gap = (self.rng.range(220.0, 420.0) / 24.0).round() * 24.0;
                lay.b.bridge(x, x + gap, y);
                lay.ring_row(x + 30.0, y - 40.0, ((gap - 60.0) / 36.0) as usize + 1);
                lay.from(x + gap, y);
                lay.way(x, y, x + gap, y, Crossing::Bridge);
                self.flat(lay, 180.0, 360.0);
            }
            Feature::Swing => {
                let (a, z, y) = self.spanned(lay, 340.0, 440.0, Crossing::Swing);
                lay.b.swing((a + z) / 2.0, y - 230.0);
                lay.ring_arc(a + 20.0, z - 20.0, y - 110.0, 60.0, 6);
                self.flat(lay, 200.0, 360.0);
            }
            Feature::Crumble => {
                let (a, z, y) = self.spanned(lay, 260.0, 420.0, Crossing::Crumble);
                lay.b.crumbling(a, z, y);
                lay.ring_row(a + 30.0, y - 40.0, ((z - a - 60.0) / 36.0) as usize + 1);
                self.flat(lay, 200.0, 360.0);
            }
            Feature::SpikeLog => {
                let (a, z, y) = self.spanned(lay, 220.0, 360.0, Crossing::SpikeLog);
                lay.b.spike_log(a, z, y);
                lay.ring_arc(a, z, y - 70.0, 60.0, 5);
                self.flat(lay, 200.0, 360.0);
            }
            Feature::Stepping => self.stepping(lay),
            Feature::Cliff => {
                // Up a cliff on a spring.
                let rise = self.rng.range(130.0, 200.0).min(y - HIGH);
                let run = self.rng.range(300.0, 420.0).round();
                lay.line(x + run, y);
                lay.floor(x + 40.0, x + run - 220.0, y, false);
                let spring = x + run - 150.0;
                lay.b.spring(spring, y);
                for k in 0..3 {
                    lay.b.ring(spring, y - 110.0 - k as f32 * 45.0);
                }
                lay.from(x + run, y - rise);
                self.flat(lay, 300.0, 500.0);
            }
            Feature::Drop => {
                let d = self.rng.range(80.0, 150.0).min(LOW - y);
                let run = self.rng.range(120.0, 260.0).round();
                lay.line(x + run, y);
                lay.ring_arc(x + run - 40.0, x + run + 200.0, y - 40.0, 40.0, 5);
                lay.from(x + run, y + d);
                self.flat(lay, 300.0, 500.0);
            }
            Feature::Loop => {
                let w = 1000.0;
                lay.line(x + w, y);
                lay.b.dash_panel(x + 150.0, x + 270.0, y);
                let r = 110.0;
                let g = lay.look.ground;
                lay.b.sonic_loop(x + 520.0, y - r, r, g);
                for k in 0..8 {
                    let a = (k as f32 / 8.0 * 360.0).to_radians();
                    lay.b.ring(x + 520.0 + a.cos() * (r - 42.0), y - r + a.sin() * (r - 42.0));
                }
                lay.floor(x + 760.0, x + w - 60.0, y, false);
            }
            Feature::Walls => {
                let w = self.rng.range(600.0, 900.0).round();
                lay.line(x + w, y);
                let wall = x + self.rng.range(180.0, w * 0.45).round();
                lay.b.breakable_wall(wall, y);
                lay.ring_row(wall + 70.0, y - 40.0, 3);
                if self.rng.chance(0.7) {
                    let rock = x + self.rng.range(w * 0.65, w - 120.0).round();
                    lay.b.rock(rock, y);
                    lay.floor(wall + 80.0, rock - 80.0, y, false);
                } else {
                    lay.floor(wall + 80.0, x + w - 60.0, y, false);
                }
            }
            Feature::SpikeDip => {
                // Down into a dip with spikes at the bottom, and up again.
                let d = self.rng.range(45.0, 70.0).min(LOW - y);
                let side = (d * 1.5 / STEEPEST).max(140.0).round();
                let bottom = self.rng.range(150.0, 220.0).round();
                lay.curve(x + side, |t| y + d * t * t * (3.0 - 2.0 * t));
                lay.line(x + side + bottom, y + d);
                lay.curve(x + 2.0 * side + bottom, |t| y + d - d * t * t * (3.0 - 2.0 * t));
                lay.s.dangers.push(Danger::Spikes(x + side + bottom / 2.0, y + d));
                lay.ring_arc(x + side, x + side + bottom, y + d - 60.0, 50.0, 4);
                self.flat(lay, 200.0, 360.0);
            }
            Feature::LongJump => {
                // A dash panel, then a pit too wide for a jump without it.
                let run = self.rng.range(380.0, 520.0).round();
                lay.line(x + run, y);
                lay.b.dash_panel(x + run - 260.0, x + run - 140.0, y);
                let gap = self.rng.range(240.0, MOST_LONG_JUMP).round();
                lay.ring_arc(x + run, x + run + gap, y - 80.0, 80.0, 6);
                lay.from(x + run + gap, y);
                lay.way(x + run, y, x + run + gap, y, Crossing::LongJump);
                self.flat(lay, 300.0, 500.0);
            }
            Feature::Upper => self.upper(lay),
        }
    }

    /// Stepping stones in the air over a pit, then ground again.
    fn stepping(&mut self, lay: &mut Lay) {
        let (x0, y0) = lay.end;
        lay.cut();
        let (mut cx, mut cy) = (x0, y0);
        for k in 0..2 + self.rng.below(3) {
            let top = (cy - self.rng.range(-60.0, 60.0)).clamp(y0 - 170.0, y0 + 30.0);
            let hop = self.rng.range(80.0, reach(cy - top) - 20.0).round();
            let len = self.rng.range(90.0, 170.0).round();
            let sx = cx + hop;
            lay.ledge(sx, sx + len, top, if k % 2 == 0 { WOOD } else { SKY });
            lay.way(cx, cy, sx, top, Crossing::Jump);
            lay.way(sx, top, sx + len, top, Crossing::Ledge);
            lay.floor(sx + 25.0, sx + len - 25.0, top, true);
            lay.ring_row(sx + len / 2.0 - 18.0, top - 40.0, 2);
            (cx, cy) = (sx + len, top);
        }
        let y1 = (cy + self.rng.range(-40.0, 90.0)).clamp(HIGH, LOW);
        let hop = self.rng.range(80.0, reach(cy - y1) - 20.0).round();
        lay.from(cx + hop, y1);
        lay.way(cx, cy, cx + hop, y1, Crossing::Jump);
        self.flat(lay, 220.0, 400.0);
    }

    /// Two levels: a spring up to a route of ledges (rings, and monitors
    /// on them) over ground below, sometimes broken by pits to jump.
    fn upper(&mut self, lay: &mut Lay) {
        let (x, y) = lay.end;
        let len = self.rng.range(1300.0, 1900.0).round();
        lay.line(x + 260.0, y);
        lay.b.spring(x + 140.0, y);
        for k in 0..3 {
            lay.b.ring(x + 140.0, y - 110.0 - k as f32 * 45.0);
        }
        // Below.
        if self.rng.chance(0.5) {
            for _ in 0..2 {
                self.flat(lay, 160.0, 280.0);
                let gap = self.rng.range(110.0, 170.0).round();
                let y0 = lay.end.1;
                self.jump(lay, gap, y0);
            }
        }
        let (bx, by) = lay.end;
        if bx < x + len {
            lay.line(x + len, by);
            lay.floor(bx + 60.0, x + len - 60.0, by, false);
        }
        // Up above.
        let mut top = y - self.rng.range(220.0, 260.0);
        let mut at = x + 230.0;
        let mut last: Option<(f32, f32)> = None;
        for k in 0.. {
            let l = self.rng.range(220.0, 380.0).round().min(x + len - 120.0 - at);
            if l < 120.0 {
                break;
            }
            lay.ledge(at, at + l, top, if k % 2 == 0 { WOOD } else { SKY });
            lay.ring_row(at + 30.0, top - 40.0, ((l - 60.0) / 36.0) as usize + 1);
            lay.floor(at + 40.0, at + l - 40.0, top, true);
            if let Some((px, py)) = last {
                lay.s.climbs.push(Pit { x0: px, y0: py, x1: at, y1: top, how: Crossing::Jump });
            }
            last = Some((at + l, top));
            let next = (top + self.rng.range(-40.0, 40.0)).clamp(y - 290.0, y - 210.0);
            at += l + self.rng.range(70.0, reach(top - next) - 30.0).round();
            top = next;
        }
    }

    /// What goes on `floors`, `level` the further on (0 at first): badniks
    /// and spikes below, monitors and the odd ring box up on the ledges.
    pub fn dangers(&mut self, floors: &[Floor], level: u32) -> Vec<Danger> {
        let rng = &mut self.rng;
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
            // Badniks only where there is room: on a bit of ground between
            // pits (walkers go to its edges) they would be where the
            // character lands, and being hit there throws them back in.
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
}

/// A whole act from `seed`: a start, `n` sections with their dangers, a
/// star post every few, and the goal. Returns its name.
pub fn act(b: &mut Builder, seed: u64, n: usize) -> String {
    let mut course = Course::new(seed);
    let look = Look { ground: [150, 98, 62], edge: LEAF };
    let first = course.start(b, -400.0, 1100.0, &look);
    b.sonic(140.0, BASE - 40.0);
    for k in 0..5 {
        b.ring(300.0 + k as f32 * 36.0, BASE - 40.0);
    }
    let mut x = -400.0 + first.width;
    for i in 0..n {
        let level = (i / 3) as u32;
        let s = course.section(b, x, &look, level);
        let mut dangers = s.dangers.clone();
        dangers.extend(course.dangers(&s.floors, level));
        for d in dangers {
            match d {
                Danger::Walker(kind, x, y) => b.walker(kind, x, y),
                Danger::Flyer(x, y) => b.enemy(EnemyKind::Flyer, x, y),
                Danger::Spikes(x, y) => b.spikes(x, y, 2),
                Danger::Monitor(kind, x, y) => b.monitor(kind, x, y),
            }
        }
        // A star post after every few sections, on the ground.
        if i % 4 == 3 {
            if let Some(f) = s.floors.iter().find(|f| !f.upper) {
                b.post((f.x0 + f.x1) / 2.0, f.y);
            }
        }
        x += s.width;
    }
    let y = course.y;
    b.ground(x, x + 1100.0, y, look.ground, look.edge);
    for k in 0..6 {
        b.ring(x + 120.0 + k as f32 * 36.0, y - 40.0);
    }
    b.monitor(MonitorKind::Rings, x + 420.0, y);
    b.goal(x + 760.0, y);
    let name = format!("Random act {}", seed % 100_000);
    b.scene.game.get_or_insert_with(Default::default).name = name.clone();
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor(x0: f32, x1: f32, y: f32) -> Floor {
        Floor { x0, x1, y, upper: false }
    }

    fn lay(seed: u64, n: usize) -> Vec<Section> {
        let mut b = Builder::at(0.0, 1440.0);
        let mut course = Course::new(seed);
        let look = Look { ground: [1, 2, 3], edge: LEAF };
        let first = course.start(&mut b, 0.0, 800.0, &look);
        let mut x = first.width;
        let mut out = vec![first];
        for i in 0..n {
            let s = course.section(&mut b, x, &look, (i / 3) as u32);
            x += s.width;
            out.push(s);
        }
        out
    }

    fn can_jump(p: &Pit) {
        let rise = p.y0 - p.y1;
        assert!(p.x1 > p.x0, "{p:?}");
        assert!(rise <= MOST_RISE + 0.5, "a jump up {rise}: {p:?}");
        assert!(p.x1 - p.x0 <= reach(rise) + 0.5, "too wide: {p:?}");
    }

    #[test]
    fn the_ground_stays_in_its_band_is_never_too_steep_and_every_gap_has_a_way_over() {
        for seed in 1..40 {
            let sections = lay(seed, 40);
            for s in &sections {
                for strip in &s.ground {
                    for w in strip.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        assert!(b.0 > a.0, "it goes on: {a:?} {b:?}");
                        assert!(((b.1 - a.1) / (b.0 - a.0)).abs() <= STEEPEST + 0.02, "too steep: {a:?} {b:?}");
                    }
                    assert!(strip.iter().all(|p| (HIGH - 0.5..=LOW + 0.5).contains(&p.1)), "{:?}", s.feature);
                }
                for p in &s.pits {
                    match p.how {
                        Crossing::Jump => can_jump(p),
                        Crossing::Ledge => assert_eq!(p.y0, p.y1),
                        Crossing::LongJump => assert!(p.y0 == p.y1 && p.x1 - p.x0 <= MOST_LONG_JUMP + 0.5),
                        _ => assert!(p.y0 == p.y1 && p.x1 - p.x0 <= 440.5, "{p:?}"),
                    }
                }
                s.climbs.iter().for_each(can_jump);
                for f in &s.floors {
                    assert!(f.x0 < f.x1, "{:?}: {f:?}", s.feature);
                }
            }
            // From one strip of ground to the next: a step (a cliff, a
            // drop) or ways over, one after the other, end to end.
            let strips: Vec<&Vec<(f32, f32)>> = sections.iter().flat_map(|s| s.ground.iter()).collect();
            let pits: Vec<Pit> = sections.iter().flat_map(|s| s.pits.iter().copied()).collect();
            for w in strips.windows(2) {
                let (end, start) = (*w[0].last().unwrap(), w[1][0]);
                if (start.0 - end.0).abs() < 0.5 {
                    continue;
                }
                let mut at = end;
                while (at.0 - start.0).abs() > 0.5 {
                    let p = pits
                        .iter()
                        .find(|p| (p.x0 - at.0).abs() < 0.5 && (p.y0 - at.1).abs() < 0.5)
                        .unwrap_or_else(|| panic!("seed {seed}: no way on from {at:?} to {start:?}"));
                    at = (p.x1, p.y1);
                }
                assert!((at.1 - start.1).abs() < 0.5, "seed {seed}: lands at {at:?}, the ground is at {start:?}");
            }
        }
    }

    #[test]
    fn every_feature_comes_up_and_none_again_too_soon() {
        let sections = lay(7, 400);
        let features: Vec<Feature> = sections.iter().filter_map(|s| s.feature).collect();
        for f in Feature::ALL {
            assert!(features.contains(&f), "{f:?} never comes up");
        }
        for w in features.windows(4) {
            assert!(!w[1..].contains(&w[0]), "{:?} again too soon", w[0]);
        }
        // Two levels: a route of ledges well above the ground below.
        let two = sections.iter().find(|s| s.feature == Some(Feature::Upper)).unwrap();
        let below = two.floors.iter().rfind(|f| !f.upper).unwrap();
        assert!(two.floors.iter().any(|f| f.upper && below.y - f.y >= 200.0), "{:?}", two.floors);
    }

    #[test]
    fn the_ground_is_never_the_same_twice() {
        let tops = |seed| -> Vec<(i32, i32)> {
            lay(seed, 30).iter().flat_map(|s| s.ground.iter().flatten()).map(|p| (p.0 as i32, p.1 as i32)).collect()
        };
        assert_eq!(tops(5), tops(5), "the same seed, the same ground");
        assert_ne!(tops(5), tops(6));
        // It goes up and down, and the sections are all sorts of lengths.
        let sections = lay(9, 60);
        let ys: Vec<f32> = sections.iter().flat_map(|s| s.ground.iter().flatten()).map(|p| p.1).collect();
        let (lo, hi) = ys.iter().fold((f32::MAX, f32::MIN), |(a, b), &y| (a.min(y), b.max(y)));
        assert!(hi - lo > 180.0, "from {lo} to {hi}");
        let widths: Vec<f32> = sections.iter().map(|s| s.width).collect();
        let (lo, hi) = widths.iter().fold((f32::MAX, f32::MIN), |(a, b), &w| (a.min(w), b.max(w)));
        assert!(hi > 2.0 * lo, "widths from {lo} to {hi}");
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
        let mut course = Course::new(3);
        for _ in 0..200 {
            for d in course.dangers(&floors, 5) {
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
