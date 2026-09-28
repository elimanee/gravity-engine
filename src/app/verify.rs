//! `--verify-challenges`: play every built-in challenge with its reference
//! solution (and once without drawing anything) to check that each level
//! can be solved within its ink, and is not solved by doing nothing.

use super::library::Origin;
use super::*;
use crate::library::challenges;

/// Simulated seconds allowed per attempt.
const SIM_SECS: f32 = 20.0;

impl App {
    /// Returns whether every level passed. Needs a window (textures).
    pub fn verify_challenges(&mut self, only: Option<usize>) -> bool {
        self.s.sfx = false;
        self.s.slow_motion = false;
        self.s.effects = false;
        self.s.window_shake = false;
        self.s.time_scale = 1.0;
        self.s.draw_thickness = 14.0;
        let mut all_ok = true;
        for (i, c) in challenges::ALL.iter().enumerate() {
            if only.is_some_and(|o| o != i) {
                continue;
            }
            let idle = self.attempt(i, false);
            let solved = self.attempt(i, true);
            let used = self.challenge.as_ref().map_or(0.0, |r| 1.0 - r.ink_left / c.ink);
            let ok = solved.won && !idle.won;
            all_ok &= ok;
            println!(
                "{:>2} {:<18} {}  solution: {} in {:>4.1}s, ink {:>3.0}% ({} stars), ball ends at ({:.0}, {:.0})  ·  no drawing: {} ({:.0}, {:.0})",
                i + 1,
                c.name,
                if ok { "ok  " } else { "FAIL" },
                if solved.won { "won " } else { "lost" },
                solved.time,
                used * 100.0,
                crate::library::custom::stars(used),
                solved.end.x,
                solved.end.y,
                if idle.won { "WON" } else { "lost" },
                idle.end.x,
                idle.end.y,
            );
        }
        all_ok
    }

    fn attempt(&mut self, i: usize, draw: bool) -> Attempt {
        self.start_challenge(i);
        let origin = self.design_origin();
        if draw {
            for stroke in challenges::ALL[i].solution {
                self.draw_design_stroke(stroke, origin);
            }
        }
        self.challenge_go();
        let dt = 1.0 / 60.0;
        let mut t = 0.0;
        let trace = std::env::var_os("GE_TRACE").is_some();
        let mut steps = 0;
        while t < SIM_SECS {
            self.simulate(dt, Vec2::ZERO);
            self.update_challenge(dt);
            t += dt;
            steps += 1;
            if trace && steps % 15 == 0 {
                if let Some(b) = self.ball() {
                    let body = &self.world.bodies[self.objects[b].body];
                    let p = crate::physics::to_screen(body.translation().x, body.translation().y) - origin;
                    let v = body.linvel();
                    println!(
                        "   {} t={t:>4.2} ball ({:>4.0}, {:>4.0}) v ({:>5.1}, {:>5.1})",
                        if draw { "draw" } else { "idle" },
                        p.x,
                        p.y,
                        v.x,
                        v.y
                    );
                }
            }
            if self.challenge.as_ref().is_some_and(|r| r.won) {
                break;
            }
        }
        let won = self.challenge.as_ref().is_some_and(|r| r.won && matches!(r.def.origin, Origin::BuiltIn(_)));
        let end = self
            .ball()
            .map(|b| {
                let p = self.world.bodies[self.objects[b].body].translation();
                crate::physics::to_screen(p.x, p.y) - origin
            })
            .unwrap_or(vec2(f32::NAN, f32::NAN));
        Attempt { won, time: t, end }
    }

    /// World position of the design-space origin.
    fn design_origin(&self) -> Vec2 {
        let (aw, ah) = self.arena();
        let r = crate::library::design_rect(aw, ah);
        vec2(r.x, r.y)
    }

    /// Draw a pinned stroke through design points, spending ink like the
    /// Draw tool does.
    fn draw_design_stroke(&mut self, points: &[(f32, f32)], origin: Vec2) {
        let mut out = vec![(points[0].0 + origin.x, points[0].1 + origin.y)];
        for w in points.windows(2) {
            let (a, b) = (vec2(w[0].0, w[0].1) + origin, vec2(w[1].0, w[1].1) + origin);
            let n = (a.distance(b) / drawing::SAMPLE_SPACING).ceil().max(1.0) as usize;
            for k in 1..=n {
                let p = a.lerp(b, k as f32 / n as f32);
                let last = *out.last().unwrap_or(&(p.x, p.y));
                if self.spend_ink((p.x - last.0).hypot(p.y - last.1)) {
                    out.push((p.x, p.y));
                }
            }
        }
        self.finish_stroke(Stroke { points: out, rgb: [230, 230, 240] }, false);
    }
}

struct Attempt {
    won: bool,
    time: f32,
    /// Where the ball ended, in design px.
    end: Vec2,
}
