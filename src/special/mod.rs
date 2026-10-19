//! Special stages, after the Blue Spheres of Sonic 3: Sonic runs on a
//! checkered globe and turns at the crossings. Blue spheres turn red once
//! he has passed over them; touching a red one ends the stage; catching
//! every blue sphere wins a chaos emerald. Rings count up, springs throw
//! him far ahead, bumpers send him running backwards. He speeds up as
//! time goes by.
//!
//! The seven stages (one per emerald) are laid out by code, the same
//! every time.

pub mod art;

use macroquad::prelude::{vec2, Vec2};

/// Chaos emeralds to find.
pub const EMERALDS: u32 = 7;
/// The globe's cells along each side (it wraps round).
pub const SIZE: i32 = 32;
/// Seconds before he starts running, and how long a jump and a spring last.
const INTRO: f32 = 1.6;
const JUMP: f32 = 0.75;
const SPRING: f32 = 1.5;
/// Speed (cells per second): at the start, its gain, how often, the top.
const SPEED: f32 = 4.0;
const SPEED_UP: f32 = 0.75;
const SPEED_EVERY: f32 = 25.0;
const TOP_SPEED: f32 = 8.5;
/// Seconds a turn takes on screen.
const TURN_TIME: f32 = 0.18;

/// How many emeralds a bit mask holds.
pub fn count(mask: u8) -> u32 {
    (mask & 0x7f).count_ones()
}

/// The first emerald not found yet.
pub fn next_missing(mask: u8) -> Option<usize> {
    (0..EMERALDS as usize).find(|i| mask & (1 << i) == 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    Empty,
    Blue,
    Red,
    Ring,
    Bumper,
    Spring,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outcome {
    /// Every blue sphere caught: the emerald (seconds since).
    Won(f32),
    /// A red sphere touched (seconds since).
    Lost(f32),
}

/// What happened during an update, for sounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Blue,
    Ring,
    Bumper,
    Spring,
    Jump,
    Won,
    Lost,
}

/// The four headings: +y, +x, -y, -x.
const HEADINGS: [(i32, i32); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];

pub fn heading(dir: i32) -> Vec2 {
    let (x, y) = HEADINGS[dir.rem_euclid(4) as usize];
    vec2(x as f32, y as f32)
}

fn wrap(i: i32) -> i32 {
    i.rem_euclid(SIZE)
}

/// Buttons for a frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub jump_pressed: bool,
}

pub struct Stage {
    /// Which emerald it holds (0‥6).
    pub index: usize,
    grid: Vec<Cell>,
    /// Where he is (cells; a cell's centre is at whole numbers).
    pub pos: Vec2,
    pub dir: i32,
    /// The way the view faces (in quarter turns), following `dir`.
    pub view: f32,
    pub speed: f32,
    /// Seconds since the stage began.
    pub time: f32,
    /// A turn asked for, made at the next crossing.
    turn: i32,
    /// In the air: seconds so far and the jump's length.
    pub air: Option<(f32, f32)>,
    /// Sent back by a bumper.
    pub backwards: bool,
    pub rings: u32,
    pub rings_total: u32,
    pub blue_left: u32,
    pub outcome: Option<Outcome>,
    /// The last crossing he took a sphere at (it is red behind him).
    last: (i32, i32),
}

impl Stage {
    pub fn new(index: usize) -> Self {
        let (grid, start) = layout(index);
        let count = |c: Cell| grid.iter().filter(|&&g| g == c).count() as u32;
        let (blue_left, rings_total) = (count(Cell::Blue), count(Cell::Ring));
        Stage {
            index,
            grid,
            pos: vec2(start.0 as f32, start.1 as f32),
            dir: 0,
            view: 0.0,
            speed: SPEED + index as f32 * 0.25,
            time: 0.0,
            turn: 0,
            air: None,
            backwards: false,
            rings: 0,
            rings_total,
            blue_left,
            outcome: None,
            last: (i32::MIN, i32::MIN),
        }
    }

    pub fn cell(&self, x: i32, y: i32) -> Cell {
        self.grid[(wrap(y) * SIZE + wrap(x)) as usize]
    }

    fn set(&mut self, x: i32, y: i32, c: Cell) {
        self.grid[(wrap(y) * SIZE + wrap(x)) as usize] = c;
    }

    /// Height of a jump (cells), 0 on the ground.
    pub fn height(&self) -> f32 {
        self.air.map_or(0.0, |(t, len)| {
            let s = (t / len).clamp(0.0, 1.0);
            4.0 * s * (1.0 - s) * len * 1.1
        })
    }

    /// Running (after the intro, until the end).
    pub fn running(&self) -> bool {
        self.time > INTRO && self.outcome.is_none()
    }

    pub fn intro(&self) -> bool {
        self.time <= INTRO
    }

    pub fn update(&mut self, dt: f32, input: Input) -> Vec<Event> {
        let mut out = vec![];
        self.time += dt;
        if let Some(o) = &mut self.outcome {
            match o {
                Outcome::Won(t) | Outcome::Lost(t) => *t += dt,
            }
            return out;
        }
        // The view turns smoothly to the heading.
        let target = self.dir as f32;
        let d = target - self.view;
        self.view += d.signum() * d.abs().min(dt / TURN_TIME);
        if !self.running() {
            return out;
        }
        let base = SPEED + self.index as f32 * 0.25;
        self.speed = (base + SPEED_UP * ((self.time - INTRO) / SPEED_EVERY).floor()).min(TOP_SPEED);
        if input.left != input.right {
            self.turn = if input.left { -1 } else { 1 };
        }
        if input.forward {
            self.backwards = false;
        }
        if input.jump_pressed && self.air.is_none() {
            self.air = Some((0.0, JUMP));
            out.push(Event::Jump);
        }
        // Small steps, so no crossing is missed.
        let mut left = self.speed * dt;
        while left > 0.0 && self.outcome.is_none() {
            let step = left.min(0.2);
            left -= step;
            self.advance(step, &mut out);
        }
        if let Some((t, len)) = &mut self.air {
            *t += dt;
            if *t >= *len {
                self.air = None;
            }
        }
        out
    }

    /// Move `step` cells on, acting at each crossing.
    fn advance(&mut self, step: f32, out: &mut Vec<Event>) {
        let sign = if self.backwards { -1.0 } else { 1.0 };
        let h = heading(self.dir) * sign;
        let before = self.pos;
        self.pos += h * step;
        // The crossing passed, if any, along the way he runs.
        let (a, b) = if h.x != 0.0 { (before.x, self.pos.x) } else { (before.y, self.pos.y) };
        let crossed = if b > a { a.floor() != b.floor() } else { a.ceil() != b.ceil() };
        if !crossed {
            return;
        }
        let at = if b > a { b.floor() } else { b.ceil() };
        let c =
            if h.x != 0.0 { (at as i32, self.pos.y.round() as i32) } else { (self.pos.x.round() as i32, at as i32) };
        // Wrap round the globe (keeping the numbers small).
        let size = SIZE as f32;
        if self.pos.x < 0.0 || self.pos.x >= size || self.pos.y < 0.0 || self.pos.y >= size {
            self.pos = vec2(self.pos.x.rem_euclid(size), self.pos.y.rem_euclid(size));
        }
        if self.air.is_none() {
            let was = self.backwards;
            self.touch(c, out);
            if self.outcome.is_some() {
                return;
            }
            // Bounced back off a bumper: from the crossing, the other way.
            if self.backwards != was {
                let back = if self.backwards { -1.0 } else { 1.0 };
                let p = vec2(c.0 as f32, c.1 as f32) + heading(self.dir) * back * (b - at).abs();
                self.pos = vec2(p.x.rem_euclid(size), p.y.rem_euclid(size));
                return;
            }
        }
        if self.turn != 0 && self.air.is_none() {
            // Turn on the spot at the crossing (running forward again),
            // keeping what is left of the step.
            let over = (b - at).abs();
            self.pos = vec2(c.0 as f32, c.1 as f32);
            self.dir = (self.dir + self.turn).rem_euclid(4);
            self.backwards = false;
            // The view takes the short way round.
            while self.dir as f32 - self.view > 2.0 {
                self.view += 4.0;
            }
            while self.view - self.dir as f32 > 2.0 {
                self.view -= 4.0;
            }
            self.turn = 0;
            self.pos += heading(self.dir) * over;
            self.pos = vec2(self.pos.x.rem_euclid(size), self.pos.y.rem_euclid(size));
        }
    }

    /// What he finds at crossing `c`.
    fn touch(&mut self, c: (i32, i32), out: &mut Vec<Event>) {
        match self.cell(c.0, c.1) {
            Cell::Blue => {
                self.set(c.0, c.1, Cell::Red);
                self.last = (wrap(c.0), wrap(c.1));
                self.blue_left -= 1;
                out.push(Event::Blue);
                if self.blue_left == 0 {
                    self.outcome = Some(Outcome::Won(0.0));
                    out.push(Event::Won);
                }
            }
            Cell::Red if self.last != (wrap(c.0), wrap(c.1)) => {
                self.outcome = Some(Outcome::Lost(0.0));
                out.push(Event::Lost);
            }
            Cell::Ring => {
                self.set(c.0, c.1, Cell::Empty);
                self.rings += 1;
                out.push(Event::Ring);
            }
            Cell::Bumper => {
                self.backwards = !self.backwards;
                self.turn = 0;
                out.push(Event::Bumper);
            }
            Cell::Spring => {
                self.air = Some((0.0, SPRING));
                out.push(Event::Spring);
            }
            _ => {}
        }
    }

    /// Every ring caught as well.
    pub fn perfect(&self) -> bool {
        self.rings_total > 0 && self.rings == self.rings_total
    }
}

/// A small deterministic random source for the layouts.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: i32) -> i32 {
        (self.next() % n as u64) as i32
    }
}

/// Stage `index`'s cells and where he starts (facing +y).
fn layout(index: usize) -> (Vec<Cell>, (i32, i32)) {
    let mut grid = vec![Cell::Empty; (SIZE * SIZE) as usize];
    let at = |x: i32, y: i32| (wrap(y) * SIZE + wrap(x)) as usize;
    let start = (16, 2);
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15 ^ (index as u64 + 1).wrapping_mul(0x2545_f491_4f6c_dd1d));
    // Nothing right round the start.
    let near_start = |x: i32, y: i32| {
        let dx = (wrap(x) - start.0).abs().min(SIZE - (wrap(x) - start.0).abs());
        let dy = (wrap(y) - start.1).abs().min(SIZE - (wrap(y) - start.1).abs());
        dx <= 2 && dy <= 3
    };
    // Pillars (every fourth crossing) are kept for red spheres, so the
    // free cells always join up.
    let pillar = |x: i32, y: i32| wrap(x) % 4 == 2 && wrap(y) % 4 == 2;
    let free = |grid: &Vec<Cell>, x: i32, y: i32| grid[at(x, y)] == Cell::Empty && !near_start(x, y) && !pillar(x, y);

    // Squares of blue spheres: walking round one catches them all.
    let squares = 3 + index as i32;
    let mut placed = 0;
    let mut tries = 0;
    while placed < squares && tries < 400 {
        tries += 1;
        let side = 3 + rng.below(3);
        let (x0, y0) = (rng.below(SIZE), rng.below(SIZE));
        let edge: Vec<(i32, i32)> = (0..side)
            .flat_map(|i| [(x0 + i, y0), (x0 + i, y0 + side - 1), (x0, y0 + i), (x0 + side - 1, y0 + i)])
            .collect();
        // Room round it, so squares do not touch.
        let clear = (-1..=side).all(|i| (-1..=side).all(|j| grid[at(x0 + i, y0 + j)] == Cell::Empty))
            && edge.iter().all(|&(x, y)| !near_start(x, y));
        if !clear {
            continue;
        }
        for (x, y) in edge {
            if !pillar(x, y) {
                grid[at(x, y)] = Cell::Blue;
            }
        }
        // A ring in the middle of the bigger ones.
        if side == 5 {
            grid[at(x0 + 2, y0 + 2)] = Cell::Ring;
        }
        placed += 1;
    }
    // Lines of blue spheres across the globe, and rows of rings.
    for k in 0..(2 + index as i32 / 2) {
        let (x0, y0) = (rng.below(SIZE), rng.below(SIZE));
        let (dx, dy) = if k % 2 == 0 { (1, 0) } else { (0, 1) };
        let len = 4 + rng.below(4);
        for i in 0..len {
            let (x, y) = (x0 + dx * i, y0 + dy * i);
            if free(&grid, x, y) {
                grid[at(x, y)] = Cell::Blue;
            }
        }
    }
    for _ in 0..4 {
        let (x0, y0) = (rng.below(SIZE), rng.below(SIZE));
        for i in 0..5 {
            if free(&grid, x0 + i, y0) {
                grid[at(x0 + i, y0)] = Cell::Ring;
            }
        }
    }
    // Red spheres on pillars with no blue sphere next to them; more in later stages.
    let reds = 0.18 + index as f32 * 0.07;
    for y in 0..SIZE {
        for x in 0..SIZE {
            if !pillar(x, y) || near_start(x, y) {
                continue;
            }
            let blue_near = (-1..=1).any(|i| (-1..=1).any(|j| grid[at(x + i, y + j)] == Cell::Blue));
            if !blue_near && (rng.below(1000) as f32) < reds * 1000.0 {
                grid[at(x, y)] = Cell::Red;
            }
        }
    }
    // A few bumpers and springs.
    for (cell, n) in [(Cell::Bumper, 3 + index as i32), (Cell::Spring, 2)] {
        let mut left = n;
        let mut tries = 0;
        while left > 0 && tries < 200 {
            tries += 1;
            let (x, y) = (rng.below(SIZE), rng.below(SIZE));
            let lonely = (-1..=1).all(|i| (-1..=1).all(|j| grid[at(x + i, y + j)] == Cell::Empty));
            if free(&grid, x, y) && lonely {
                grid[at(x, y)] = cell;
                left -= 1;
            }
        }
    }
    (grid, start)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(s: &mut Stage, secs: f32, input: Input) -> Vec<Event> {
        let mut out = vec![];
        for _ in 0..(secs * 60.0) as usize {
            out.extend(s.update(1.0 / 60.0, input));
            if s.outcome.is_some() {
                break;
            }
        }
        out
    }

    #[test]
    fn every_stage_has_blue_spheres_and_a_clear_start() {
        for i in 0..EMERALDS as usize {
            let s = Stage::new(i);
            assert!(s.blue_left >= 30, "stage {i}: {} blue spheres", s.blue_left);
            assert!(s.rings_total > 0);
            for dy in -2..=3 {
                assert_eq!(s.cell(16, 2 + dy), Cell::Empty, "stage {i} starts in the clear");
            }
            // The same every time.
            assert_eq!(Stage::new(i).grid, s.grid);
        }
        assert!(Stage::new(6).blue_left > Stage::new(0).blue_left, "later stages are bigger");
    }

    #[test]
    fn blue_spheres_turn_red_and_red_ones_end_the_stage() {
        let mut s = Stage::new(0);
        s.grid = vec![Cell::Empty; (SIZE * SIZE) as usize];
        s.set(16, 6, Cell::Blue);
        s.set(16, 8, Cell::Blue);
        s.set(16, 12, Cell::Red);
        s.blue_left = 2;
        let out = run(&mut s, INTRO - 0.05, Input::default());
        assert!(out.is_empty() && s.pos.y == 2.0, "still for the intro");
        let out = run(&mut s, 1.15, Input::default());
        assert!(out.contains(&Event::Blue));
        assert_eq!(s.cell(16, 6), Cell::Red, "red behind him");
        assert_eq!(s.blue_left, 1);
        // Catching the last one wins.
        let out = run(&mut s, 1.0, Input::default());
        assert!(out.contains(&Event::Won));
        assert_eq!(s.outcome, Some(Outcome::Won(0.0)));

        let mut s = Stage::new(0);
        s.grid = vec![Cell::Empty; (SIZE * SIZE) as usize];
        s.set(16, 6, Cell::Red);
        s.set(16, 20, Cell::Blue);
        s.blue_left = 1;
        let out = run(&mut s, INTRO + 2.0, Input::default());
        assert!(out.contains(&Event::Lost));
    }

    #[test]
    fn jumps_clear_red_spheres_and_turns_happen_at_crossings() {
        let mut s = Stage::new(0);
        s.grid = vec![Cell::Empty; (SIZE * SIZE) as usize];
        s.set(16, 4, Cell::Red);
        s.set(16, 30, Cell::Blue);
        s.blue_left = 1;
        run(&mut s, INTRO + 0.3, Input::default());
        let jump = Input { jump_pressed: true, ..Input::default() };
        run(&mut s, 1.0 / 60.0, jump);
        run(&mut s, 1.0, Input::default());
        assert!(s.outcome.is_none(), "jumped over it");
        assert!(s.pos.y > 4.5);
        // Turn right: at the next crossing he heads +x, on a whole y.
        let right = Input { right: true, ..Input::default() };
        run(&mut s, 1.0 / 60.0, right);
        run(&mut s, 0.6, Input::default());
        assert_eq!(s.dir, 1);
        assert_eq!(s.pos.y, s.pos.y.round());
        assert!(s.pos.x > 16.0);
    }

    #[test]
    fn bumpers_send_him_back_and_rings_count() {
        let mut s = Stage::new(0);
        s.grid = vec![Cell::Empty; (SIZE * SIZE) as usize];
        s.set(16, 4, Cell::Ring);
        s.set(16, 6, Cell::Bumper);
        s.set(16, 30, Cell::Blue);
        s.blue_left = 1;
        s.rings_total = 1;
        let out = run(&mut s, INTRO + 1.2, Input::default());
        assert!(out.contains(&Event::Ring) && out.contains(&Event::Bumper));
        assert!(s.backwards && s.pos.y < 6.0, "running back");
        assert!(s.perfect());
        run(&mut s, 1.0 / 60.0, Input { forward: true, ..Input::default() });
        assert!(!s.backwards);
    }

    #[test]
    fn emerald_masks() {
        assert_eq!(count(0b101), 2);
        assert_eq!(next_missing(0b0000_0111), Some(3));
        assert_eq!(next_missing(0x7f), None);
    }
}
