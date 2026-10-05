//! Sonic's endless run, in the infinite world: stretches of terrain are
//! made ahead of him as he goes (hills, loops, ramps, springs, platforms,
//! crates, rings) and cleared away behind him. The distance run and the
//! best one are shown under the ring counter.

use super::*;
use crate::library::{Builder, CORAL, LEAF, SKY, STONE, WOOD};
use std::collections::VecDeque;

/// Terrain kept ready ahead of him, and kept behind him (px).
const AHEAD: f32 = 2600.0;
const BEHIND: f32 = 1800.0;
/// The floor in the builder's design coordinates.
const FLOOR: f32 = 690.0;

struct Stretch {
    x1: f32,
    bodies: Vec<RigidBodyHandle>,
}

pub(super) struct Endless {
    /// Where the next stretch starts and where the run started (px).
    next: f32,
    start: f32,
    stretches: VecDeque<Stretch>,
    made: u32,
    last_kind: u32,
    /// Furthest point reached (px).
    pub reached: f32,
}

impl Endless {
    pub fn shift(&mut self, dx: f32) {
        self.next -= dx;
        self.start -= dx;
        self.reached -= dx;
        for s in &mut self.stretches {
            s.x1 -= dx;
        }
    }

    /// Distance run (m).
    pub fn distance(&self) -> f32 {
        ((self.reached - self.start) / PPM).max(0.0)
    }
}

impl App {
    /// Start an endless run: the infinite world, an empty floor and Sonic.
    pub(super) fn start_endless(&mut self) {
        if !self.infinite() {
            self.set_world_size(4);
        }
        let (aw, ah) = self.arena();
        let floor = ah - WALL_T * PPM;
        let x = aw / 2.0;
        self.spawn_sonic(vec2(x, floor - 40.0));
        self.camera.center = vec2(x, floor - screen_height() * 0.35);
        self.endless =
            Some(Endless { next: x - 500.0, start: x, stretches: VecDeque::new(), made: 0, last_kind: 0, reached: x });
    }

    /// Once a frame: terrain ahead, clean-up behind, the record.
    pub(super) fn update_endless(&mut self) {
        let Some(sx) = self.sonic.as_ref().map(|r| r.sonic.pos.x) else {
            self.endless = None;
            return;
        };
        if self.endless.is_none() || !self.infinite() {
            self.endless = None;
            return;
        }
        while self.endless.as_ref().is_some_and(|e| e.next < sx + AHEAD) {
            self.make_stretch();
        }
        let Some(e) = &mut self.endless else { return };
        e.reached = e.reached.max(sx);
        let d = e.distance();
        let mut old = vec![];
        while e.stretches.front().is_some_and(|s| s.x1 < sx - BEHIND) {
            old.extend(e.stretches.pop_front().map(|s| s.bodies).unwrap_or_default());
        }
        if d > self.s.sonic_best {
            self.s.sonic_best = d;
        }
        for body in old {
            if let Some(i) = self.objects.iter().position(|o| o.body == body) {
                self.remove_object(i);
            }
        }
        self.rings.retain(|r| r.x > sx - BEHIND);
    }

    fn make_stretch(&mut self) {
        let (_, ah) = self.arena();
        let Some(e) = &mut self.endless else { return };
        let x0 = e.next;
        // A different kind each time (the first is flat).
        let kind = if e.made == 0 {
            0
        } else {
            let mut k = rand::gen_range(1u32, 6);
            if k == e.last_kind {
                k = k % 5 + 1;
            }
            k
        };
        e.made += 1;
        e.last_kind = kind;
        let mut b = Builder::at(x0, ah);
        let width = build(&mut b, kind);
        let made = crate::scene::instantiate(&b.scene, &mut self.world);
        let bodies = made.objects.iter().map(|o| o.body).collect();
        self.objects.extend(made.objects);
        self.rings.extend(b.scene.rings.iter().map(|&[x, y]| crate::physics::to_screen(x, y)));
        if let Some(e) = &mut self.endless {
            e.next = x0 + width;
            e.stretches.push_back(Stretch { x1: x0 + width, bodies });
        }
    }

    /// Distance and record (screen pass, under the ring counter).
    pub(super) fn draw_endless_hud(&self, top: f32) {
        let Some(e) = &self.endless else { return };
        let line = format!("{:.0} m   ·   best {:.0} m", e.distance(), self.s.sonic_best);
        theme::text_bold(&line, 18.0, top + 50.0, 15.0, Color::new(0.0, 0.0, 0.0, 0.6));
        theme::text_bold(&line, 16.0, top + 48.0, 15.0, Color::new(0.85, 0.95, 1.0, 1.0));
    }
}

/// Build one stretch of kind `kind` from design x = 0; returns its width.
fn build(b: &mut Builder, kind: u32) -> f32 {
    let plank = Material { flammable: false, bounce: 0.1, friction: 0.6, ..Material::DEFAULT };
    let ring_row = |b: &mut Builder, x: f32, y: f32, n: usize| {
        for k in 0..n {
            b.ring(x + k as f32 * 36.0, y);
        }
    };
    match kind {
        // Flat ground and a row of rings.
        0 => {
            ring_row(b, 300.0, FLOOR - 40.0, 6);
            900.0
        }
        // Two rolling hills, rings along their tops.
        1 => {
            let h = rand::gen_range(50.0, 95.0);
            for start in [0.0f32, 700.0] {
                let pts: Vec<(f32, f32)> = (0..=35)
                    .map(|i| {
                        let x = i as f32 / 35.0 * 700.0;
                        let y = FLOOR + 5.0 - (h + 5.0) * (std::f32::consts::PI * x / 700.0).sin().powi(2);
                        (start + x, y)
                    })
                    .collect();
                b.stroke(&pts, 16.0, LEAF, true, plank);
                for k in 0..5 {
                    let x = 220.0 + k as f32 * 65.0;
                    let y = FLOOR - h * (std::f32::consts::PI * x / 700.0).sin().powi(2) - 45.0;
                    b.ring(start + x, y);
                }
            }
            1400.0
        }
        // A dash panel and a loop with rings round the inside.
        2 => {
            let dash = Material { conveyor: 20.0, flammable: false, ..Material::DEFAULT };
            b.stroke(&[(150.0, FLOOR - 3.0), (260.0, FLOOR - 3.0)], 8.0, CORAL, true, dash);
            let r = 110.0;
            b.sonic_loop(500.0, FLOOR - r, r, STONE);
            for k in 0..8 {
                let a = (k as f32 / 8.0 * 360.0).to_radians();
                b.ring(500.0 + a.cos() * (r - 42.0), FLOOR - r + a.sin() * (r - 42.0));
            }
            ring_row(b, 140.0, FLOOR - 40.0, 4);
            1000.0
        }
        // A ramp to fly off, rings in the air.
        3 => {
            let top = FLOOR - rand::gen_range(60.0, 100.0);
            b.wall(&[(100.0, FLOOR + 4.0), (380.0, top)], STONE);
            for k in 0..7 {
                let t = k as f32 / 6.0;
                b.ring(470.0 + t * 420.0, top - 120.0 - 160.0 * t * (1.0 - t) * 2.0);
            }
            1150.0
        }
        // A spring up to a platform full of rings (or run on underneath).
        4 => {
            let spring = Material { bounce: 1.0, flammable: false, ..Material::DEFAULT };
            b.stroke(&[(150.0, FLOOR - 4.0), (205.0, FLOOR - 4.0)], 10.0, CORAL, true, spring);
            let y = FLOOR - 250.0;
            b.wall(&[(260.0, y), (680.0, y)], WOOD);
            ring_row(b, 300.0, y - 40.0, 10);
            for k in 0..3 {
                b.ring(178.0, FLOOR - 120.0 - k as f32 * 45.0);
            }
            1050.0
        }
        // Crates to bowl over (curl up!), rings behind them.
        _ => {
            let crate_ = Material { breakable: true, strength: 7.0, ..Material::DEFAULT };
            for (col, rows) in [(0, 3), (1, 2), (2, 1)] {
                for row in 0..rows {
                    let x = 420.0 + col as f32 * 50.0;
                    let rgb = if row % 2 == 0 { WOOD } else { SKY };
                    b.shape_with(Shape::Box, rgb, x, FLOOR - 24.0 - row as f32 * 48.0, 46.0, false, crate_);
                }
            }
            ring_row(b, 650.0, FLOOR - 40.0, 6);
            1000.0
        }
    }
}
