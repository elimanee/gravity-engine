//! Feedback on top of the physics: sound effects, and a moment of slow
//! motion when something big happens.

use super::*;
use crate::audio::sfx::Sound;

/// Time runs this much slower at the heart of a slow-motion moment.
const SLOW_FACTOR: f32 = 0.25;
/// Real seconds spent at full slow motion (scaled by the trigger's weight)…
const SLOW_HOLD: f32 = 0.55;
/// …then easing back to normal speed.
const SLOW_EASE: f32 = 0.8;
/// No new slow motion for this long after one ends, so chain reactions do
/// not keep the game crawling.
const SLOW_COOLDOWN: f32 = 2.5;
/// Impact speed change (m/s) that triggers slow motion on its own.
pub(super) const SLOW_IMPACT: f32 = 22.0;
/// Quietest impact that makes a sound (m/s).
pub(super) const HIT_SOUND_SPEED: f32 = 1.2;
/// Most hit sounds started per frame.
pub(super) const HITS_PER_FRAME: usize = 3;

#[derive(Default)]
pub(super) struct SlowMo {
    /// Real seconds left.
    left: f32,
    cooldown: f32,
}

impl App {
    /// Play `sound` coming from `at` (world px).
    pub(super) fn sound(&mut self, sound: Sound, at: Vec2, volume: f32) {
        if !self.s.sfx {
            return;
        }
        let x = self.view().to_screen(at).x / screen_width().max(1.0);
        let pan = (x * 2.0 - 1.0).clamp(-1.0, 1.0) * 0.7;
        // Slow motion (and slowed time) plays lower, like a slowed tape.
        let pitch = (self.slow_factor() * self.s.time_scale).sqrt().clamp(0.45, 1.3);
        self.audio.play_sfx(get_time(), sound, volume * self.s.sfx_volume, pan, pitch);
    }

    /// Start a slow-motion moment; `weight` (0‥1) sets how long it lasts.
    pub(super) fn slow_motion(&mut self, weight: f32) {
        if !self.s.slow_motion || self.slow.cooldown > 0.0 || self.slow.left > 0.0 || self.recording.is_some() {
            return;
        }
        self.slow.left = SLOW_HOLD * weight.clamp(0.2, 1.0) + SLOW_EASE;
    }

    /// Current time multiplier from slow motion (1 = normal speed).
    pub(super) fn slow_factor(&self) -> f32 {
        slow_curve(self.slow.left)
    }

    /// Advance the slow-motion timer by real time.
    pub(super) fn tick_slow(&mut self, dt: f32) {
        if self.paused {
            return;
        }
        if self.slow.left > 0.0 {
            self.slow.left -= dt;
            if self.slow.left <= 0.0 {
                self.slow.left = 0.0;
                self.slow.cooldown = SLOW_COOLDOWN;
            }
        } else {
            self.slow.cooldown = (self.slow.cooldown - dt).max(0.0);
        }
    }

    /// A soft vignette while time is slowed.
    pub(super) fn draw_slow_vignette(&self) {
        let k = (1.0 - self.slow_factor()) / (1.0 - SLOW_FACTOR);
        if k <= 0.0 {
            return;
        }
        let (w, h) = (screen_width(), screen_height());
        let steps = 14;
        for i in 0..steps {
            let d = i as f32 * 6.0;
            let a = 0.14 * k * (1.0 - i as f32 / steps as f32).powi(2);
            let c = Color::new(0.35, 0.55, 1.0, a);
            draw_rectangle(0.0, d, w, 6.0, c);
            draw_rectangle(0.0, h - d - 6.0, w, 6.0, c);
            draw_rectangle(d, 0.0, 6.0, h, c);
            draw_rectangle(w - d - 6.0, 0.0, 6.0, h, c);
        }
    }
}

/// Time multiplier with `left` real seconds of slow motion remaining.
fn slow_curve(left: f32) -> f32 {
    if left <= 0.0 {
        1.0
    } else if left >= SLOW_EASE {
        SLOW_FACTOR
    } else {
        let t = left / SLOW_EASE;
        1.0 + (SLOW_FACTOR - 1.0) * t * t * (3.0 - 2.0 * t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_motion_eases_back_to_normal() {
        let mut prev = SLOW_FACTOR;
        let mut left = SLOW_HOLD + SLOW_EASE;
        while left > 0.0 {
            let f = slow_curve(left);
            assert!(f >= prev - 1e-6 && (SLOW_FACTOR..=1.0).contains(&f), "{left}: {f}");
            prev = f;
            left -= 0.01;
        }
        assert_eq!(slow_curve(0.0), 1.0);
        assert_eq!(slow_curve(SLOW_EASE + 0.1), SLOW_FACTOR);
    }
}
