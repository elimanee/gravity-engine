//! Rewind: the last seconds of the simulation are recorded (every body's
//! position and velocity, 60 times a simulated second); holding ← plays
//! them back in reverse at double speed, and letting go carries on from
//! there.

use super::*;
use rapier2d::prelude::{nalgebra, vector, Isometry};
use std::collections::VecDeque;

/// Seconds kept.
const KEEP_SECS: f32 = 8.0;
/// Simulated seconds between two recorded frames.
const SAMPLE: f32 = 1.0 / 60.0;
/// Recorded frames played back per rendered frame: twice real time at 60 fps.
const SPEED: usize = 2;

/// Position (x, y, angle) and velocity (x, y, angular) of a body.
type BodyState = (RigidBodyHandle, [f32; 6]);

#[derive(Default)]
pub(super) struct Rewind {
    frames: VecDeque<Vec<BodyState>>,
    /// Simulated time since the last recorded frame.
    since: f32,
    /// Playing back right now.
    pub active: bool,
}

impl Rewind {
    pub fn clear(&mut self) {
        self.frames.clear();
        self.since = 0.0;
    }

    /// Seconds that can be rewound.
    pub fn available(&self) -> f32 {
        self.frames.len() as f32 * SAMPLE
    }
}

impl App {
    /// After a simulation step of `step` seconds: record a frame when due.
    pub(super) fn rewind_record(&mut self, step: f32) {
        self.rewind.since += step;
        // (A little slack, so 60 fps steps record every frame.)
        if self.rewind.since + 1e-4 < SAMPLE {
            return;
        }
        self.rewind.since = (self.rewind.since - SAMPLE).clamp(0.0, SAMPLE);
        let bodies = &self.world.bodies;
        let frame: Vec<BodyState> = self
            .dynamic_bodies()
            .into_iter()
            .filter_map(|h| {
                let b = bodies.get(h)?;
                let (p, v) = (b.translation(), b.linvel());
                Some((h, [p.x, p.y, b.rotation().angle(), v.x, v.y, b.angvel()]))
            })
            .collect();
        self.rewind.frames.push_back(frame);
        let keep = (KEEP_SECS / SAMPLE) as usize;
        while self.rewind.frames.len() > keep {
            self.rewind.frames.pop_front();
        }
    }

    /// Once per frame before the simulation. Returns whether time is being
    /// rewound (the simulation then does not run).
    pub(super) fn rewind_tick(&mut self, holding: bool) -> bool {
        if !holding || self.challenge.as_ref().is_some_and(|r| r.won) {
            if self.rewind.active {
                self.rewind.active = false;
                self.rewind.since = 0.0;
            }
            return false;
        }
        if !self.rewind.active {
            if self.rewind.frames.is_empty() {
                self.toasts.status("rewind", "Nothing to rewind yet");
                return false;
            }
            self.rewind.active = true;
            self.grab = None;
            self.field_active = false;
            self.effects.clear();
            for o in &mut self.objects {
                o.clear_trail();
            }
        }
        // Keep the oldest frame: it is where we stop.
        let mut frame = None;
        for _ in 0..SPEED {
            if self.rewind.frames.len() > 1 {
                frame = self.rewind.frames.pop_back();
            }
        }
        let frame = frame.or_else(|| self.rewind.frames.back().cloned());
        if let Some(frame) = frame {
            for (h, [x, y, a, vx, vy, w]) in frame {
                if let Some(b) = self.world.bodies.get_mut(h) {
                    b.set_position(Isometry::new(vector![x, y], a), true);
                    if b.is_dynamic() {
                        b.set_linvel(vector![vx, vy], true);
                        b.set_angvel(w, true);
                    }
                }
            }
        }
        true
    }

    /// Tint and label while rewinding.
    pub(super) fn draw_rewind(&self, top: f32) {
        if !self.rewind.active {
            return;
        }
        let (w, h) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, w, h, Color::new(0.35, 0.5, 1.0, 0.07));
        // Scan lines.
        let t = get_time() as f32;
        let mut y = (t * 90.0) % 6.0;
        while y < h {
            draw_rectangle(0.0, y, w, 1.0, Color::new(1.0, 1.0, 1.0, 0.025));
            y += 6.0;
        }
        let secs = self.rewind.available();
        let label = format!("REWIND  {secs:.1} s");
        let lw = theme::measure_bold(&label, 14.0);
        let r = Rect::new((w - lw) / 2.0 - 40.0, top.max(10.0) + 6.0, lw + 60.0, 30.0);
        theme::panel(r, 1.0);
        let (cx, cy) = (r.x + 22.0, r.y + r.h / 2.0);
        for k in 0..2 {
            let x = cx - k as f32 * 9.0;
            draw_triangle(vec2(x - 8.0, cy), vec2(x, cy - 6.0), vec2(x, cy + 6.0), theme::ACCENT_HI);
        }
        theme::text_bold(&label, r.x + 36.0, r.y + 20.0, 14.0, theme::TEXT);
    }
}
