//! The game's objects drawn from sprite sheets the user provides (none
//! ship with the program), in the Sonic folder beside Sonic's sheet:
//!
//! - `objects.png`: Sonic 1's "Common Objects" sheet (The Spriters
//!   Resource, 712 × 1014): rings, monitors, spikes, star posts, the goal
//!   signpost, springs, explosions, the shield, invincibility stars, score
//!   numbers and "GAME OVER";
//! - `monitors.png`: Sonic 3's "Monitors" sheet (822 × 813): its monitors,
//!   and the 1-up faces of Tails and Knuckles.
//!
//! Without them everything is drawn with shapes ([`super::art`]).

use super::{MonitorKind, MONITOR, SPIKES};
use crate::sonic::SCALE;
use macroquad::prelude::*;
use std::path::Path;

pub const OBJECTS: &str = "objects.png";
pub const MONITORS: &str = "monitors.png";

/// Which of the two sheets a PNG of this size is.
pub fn sheet_name(size: (u32, u32)) -> Option<&'static str> {
    match size {
        (712, 1014) => Some(OBJECTS),
        (822, 813) => Some(MONITORS),
        _ => None,
    }
}

const fn r(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}

// Sonic 1 "Common Objects".
const S1_BACKGROUND: [[u8; 3]; 2] = [[37, 102, 26], [13, 72, 7]];
const RING: [Rect; 4] =
    [r(24., 198., 16., 16.), r(48., 198., 16., 16.), r(68., 198., 16., 16.), r(88., 198., 16., 16.)];
const RING_SPARKLE: [Rect; 4] =
    [r(112., 198., 16., 16.), r(136., 198., 16., 16.), r(160., 198., 16., 16.), r(184., 198., 16., 16.)];
const SPIKE: Rect = r(324., 198., 8., 32.);
const POST: Rect = r(132., 534., 16., 48.);
const POST_BLUE: Rect = r(132., 486., 16., 16.);
const POST_RED: Rect = r(132., 510., 16., 16.);
const SIGN_ROBOTNIK: Rect = r(244., 678., 48., 48.);
const SIGN_SONIC: Rect = r(244., 734., 48., 48.);
const SIGN_TURN: [Rect; 2] = [r(300., 706., 32., 48.), r(356., 706., 32., 48.)];
const SIGN_EDGE: Rect = r(340., 706., 8., 48.);
const SPRING: Rect = r(530., 486., 28., 16.);
const SPRING_OUT: Rect = r(530., 510., 28., 32.);
const POP: [Rect; 4] =
    [r(58., 678., 30., 32.), r(96., 678., 32., 32.), r(137., 695., 38., 38.), r(186., 695., 36., 39.)];
const BLAST: [Rect; 5] = [
    r(26., 706., 19., 16.),
    r(58., 718., 30., 32.),
    r(96., 718., 32., 32.),
    r(137., 695., 38., 38.),
    r(186., 695., 36., 39.),
];
const SHIELD: [Rect; 3] = [r(472., 285., 48., 44.), r(528., 285., 48., 44.), r(584., 285., 48., 44.)];
const STARS: [Rect; 4] =
    [r(472., 198., 48., 48.), r(528., 198., 48., 48.), r(584., 198., 48., 48.), r(640., 198., 48., 48.)];
const GAME_OVER: Rect = r(472., 407., 143., 16.);
const SCORES: [(u32, Rect); 5] = [
    (100, r(326., 315., 14., 8.)),
    (200, r(349., 315., 15., 8.)),
    (500, r(373., 315., 15., 8.)),
    (1000, r(396., 315., 19., 8.)),
    (10000, r(428., 315., 24., 8.)),
];

// Sonic 3 "Monitors".
const S3_BACKGROUND: [[u8; 3]; 2] = [[65, 169, 184], [16, 112, 132]];
const S3_TAILS: Rect = r(28., 280., 16., 16.);
const S3_KNUCKLES: Rect = r(220., 280., 16., 16.);

/// Where a sheet's monitor parts are.
struct MonitorParts {
    body: Rect,
    /// The screen's top-left inside the body (sheet px).
    screen: Vec2,
    broken: Rect,
    statics: &'static [Rect],
    /// Icons in [`MonitorKind::ALL`] order.
    icons: [Rect; 5],
}

const S1_MONITOR: MonitorParts = MonitorParts {
    body: r(25., 487., 30., 30.),
    screen: Vec2::new(7.0, 4.0),
    broken: r(64., 501., 32., 16.),
    statics: &[r(24., 540., 16., 16.), r(48., 540., 16., 16.), r(72., 540., 16., 16.)],
    icons: [
        r(96., 540., 16., 16.),
        r(48., 564., 16., 16.),
        r(72., 564., 16., 16.),
        r(24., 564., 16., 16.),
        r(96., 564., 16., 16.),
    ],
};

const S3_MONITOR: MonitorParts = MonitorParts {
    body: r(246., 24., 28., 32.),
    screen: Vec2::new(6.0, 3.0),
    broken: r(286., 42., 28., 14.),
    statics: &[r(324., 32., 16., 16.), r(348., 32., 16., 16.)],
    icons: [
        r(24., 436., 16., 16.),
        r(468., 32., 16., 16.),
        r(516., 32., 16., 16.),
        r(372., 32., 16., 16.),
        r(28., 200., 16., 16.),
    ],
};

/// The loaded sheets.
pub struct Sprites {
    s1: Option<Texture2D>,
    s3: Option<Texture2D>,
}

fn load_sheet(path: &Path, name: &str, background: &[[u8; 3]]) -> Result<Option<Texture2D>, String> {
    if !path.is_file() {
        return Ok(None);
    }
    let mut img = image::open(path).map_err(|e| format!("{name}: {e}"))?.to_rgba8();
    if sheet_name(img.dimensions()) != Some(name) {
        return Err(format!("{name} is not the expected sprite sheet ({}×{})", img.width(), img.height()));
    }
    crate::sonic::sheet::clear_background(&mut img, background);
    let tex = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, &img);
    tex.set_filter(FilterMode::Nearest);
    Ok(Some(tex))
}

/// Draw `src` of `tex` centred on `at`, `k` world px per sheet px.
fn blit(tex: &Texture2D, src: Rect, at: Vec2, k: f32, tint: Color) {
    blit_xy(tex, src, at, vec2(k, k), tint);
}

/// Like [`blit`], scaled differently across and down.
fn blit_xy(tex: &Texture2D, src: Rect, at: Vec2, k: Vec2, tint: Color) {
    let size = vec2(src.w, src.h) * k;
    let params = DrawTextureParams { dest_size: Some(size), source: Some(src), ..Default::default() };
    draw_texture_ex(tex, at.x - size.x / 2.0, at.y - size.y / 2.0, tint, params);
}

/// Like [`blit`], standing on `foot` (its bottom centre).
fn stand(tex: &Texture2D, src: Rect, foot: Vec2, k: f32) {
    blit(tex, src, foot - vec2(0.0, src.h * k / 2.0), k, WHITE);
}

impl Sprites {
    /// The sheets in the Sonic folder (`None` when neither is there).
    pub fn load() -> Result<Option<Sprites>, String> {
        let Some(dir) = crate::sonic::sheet::dir() else { return Ok(None) };
        let s1 = load_sheet(&dir.join(OBJECTS), OBJECTS, &S1_BACKGROUND)?;
        let s3 = load_sheet(&dir.join(MONITORS), MONITORS, &S3_BACKGROUND)?;
        Ok((s1.is_some() || s3.is_some()).then_some(Sprites { s1, s3 }))
    }

    /// What was loaded, for a message.
    pub fn describe(&self) -> String {
        match (&self.s1, &self.s3) {
            (Some(_), Some(_)) => "Sonic 1 objects and Sonic 3 monitors".into(),
            (Some(_), None) => "Sonic 1 objects".into(),
            _ => "Sonic 3 monitors".into(),
        }
    }

    fn monitor_sheet(&self) -> Option<(&Texture2D, &'static MonitorParts)> {
        match (&self.s1, &self.s3) {
            (Some(t), _) => Some((t, &S1_MONITOR)),
            (None, Some(t)) => Some((t, &S3_MONITOR)),
            _ => None,
        }
    }

    /// A spinning ring (false: not on the sheets).
    pub fn ring(&self, at: Vec2, t: f32) -> bool {
        let Some(tex) = &self.s1 else { return false };
        blit(tex, RING[(t * 8.0) as usize % 4], at, SCALE, WHITE);
        true
    }

    /// A caught ring's sparkle, `age` s old (false: over, or not on the sheets).
    pub fn sparkle(&self, at: Vec2, age: f32) -> bool {
        let Some(tex) = &self.s1 else { return false };
        let k = (age / 0.07) as usize;
        if k >= RING_SPARKLE.len() {
            return false;
        }
        blit(tex, RING_SPARKLE[k], at, SCALE, WHITE);
        true
    }

    /// A monitor whose box is centred on `at`; `broken`: seconds since.
    pub fn monitor(&self, kind: MonitorKind, at: Vec2, broken: Option<f32>, t: f32) -> bool {
        let Some((tex, m)) = self.monitor_sheet() else { return false };
        let foot = at + vec2(0.0, MONITOR.y / 2.0);
        let icon = m.icons[MonitorKind::ALL.iter().position(|&k| k == kind).unwrap_or(0)];
        match broken {
            Some(age) => {
                stand(tex, m.broken, foot, SCALE);
                // The item pops up out of it and goes.
                if age < 1.2 {
                    let rise = 26.0 * (age / 0.3).min(1.0);
                    blit(tex, icon, at - vec2(0.0, 12.0 + rise), SCALE, WHITE);
                }
            }
            None => {
                stand(tex, m.body, foot, SCALE);
                let top_left = foot - vec2(m.body.w / 2.0, m.body.h) * SCALE;
                let centre = top_left + (m.screen + vec2(8.0, 8.0)) * SCALE;
                // The item, with a burst of static now and then.
                let phase = (t * 12.0 + at.x * 0.013) as usize;
                let screen = if phase % 10 < 2 { m.statics[phase % m.statics.len()] } else { icon };
                blit(tex, screen, centre, SCALE, WHITE);
            }
        }
        true
    }

    /// The 1-up face of a character ("sonic", "tails", "knuckles"), for the
    /// lives counter (screen pass), `size` px high.
    pub fn face(&self, character: &str, at: Vec2, size: f32) -> bool {
        let pick = match character {
            "tails" => self.s3.as_ref().map(|t| (t, S3_TAILS)),
            "knuckles" => self.s3.as_ref().map(|t| (t, S3_KNUCKLES)),
            _ => None,
        };
        let pick = pick.or_else(|| self.monitor_sheet().map(|(t, m)| (t, m.icons[4])));
        let Some((tex, src)) = pick else { return false };
        blit(tex, src, at, size / src.h, WHITE);
        true
    }

    /// A row of spikes on a block centred on `at`.
    pub fn spikes(&self, at: Vec2) -> bool {
        let Some(tex) = &self.s1 else { return false };
        // Side by side across the block, as tall as it.
        let n = (SPIKES.x / (SPIKE.w * SCALE)).round().max(1.0) as usize;
        let step = SPIKES.x / n as f32;
        let k = vec2(step / SPIKE.w, SPIKES.y / SPIKE.h);
        for i in 0..n {
            let x = at.x - SPIKES.x / 2.0 + step * (i as f32 + 0.5);
            blit_xy(tex, SPIKE, vec2(x, at.y), k, WHITE);
        }
        true
    }

    /// A star post whose foot is at `foot`; `lit`: seconds since passed.
    pub fn post(&self, foot: Vec2, lit: Option<f32>) -> bool {
        let Some(tex) = &self.s1 else { return false };
        stand(tex, POST, foot, SCALE);
        let top = foot - vec2(0.0, POST.h * SCALE);
        let rest = top - vec2(0.0, 8.0 * SCALE);
        let (ball, at) = match lit {
            // Spins twice round the top, then stays red.
            Some(age) if age < 0.8 => {
                let a = age / 0.8 * std::f32::consts::TAU * 2.0;
                (POST_RED, top + vec2(a.sin(), -a.cos()) * 8.0 * SCALE)
            }
            Some(_) => (POST_RED, rest),
            None => (POST_BLUE, rest),
        };
        blit(tex, ball, at, SCALE, WHITE);
        true
    }

    /// The goal signpost standing on `foot`; `spin`: how far it has turned (rad).
    pub fn signpost(&self, foot: Vec2, spin: Option<f32>) -> bool {
        let Some(tex) = &self.s1 else { return false };
        let frame = match spin {
            None => SIGN_ROBOTNIK,
            Some(a) => {
                let c = a.cos().abs();
                if c > 0.8 {
                    // Robotnik while it spins, Sonic once it slows down.
                    if a > super::art::SIGN_FLIP {
                        SIGN_SONIC
                    } else {
                        SIGN_ROBOTNIK
                    }
                } else if c > 0.3 {
                    SIGN_TURN[(a / std::f32::consts::PI) as usize % 2]
                } else {
                    SIGN_EDGE
                }
            }
        };
        stand(tex, frame, foot, SCALE);
        true
    }

    /// A spring pad standing on `foot`, `w` px wide; `out`: just fired.
    pub fn spring(&self, foot: Vec2, w: f32, out: bool) -> bool {
        let Some(tex) = &self.s1 else { return false };
        let src = if out { SPRING_OUT } else { SPRING };
        let k = w / src.w;
        stand(tex, src, foot, k);
        true
    }

    /// An explosion `age` s old: a puff for badniks and monitors, fire for
    /// bombs and the boss (false: over, or not on the sheets).
    pub fn explosion(&self, at: Vec2, age: f32, fiery: bool) -> bool {
        let Some(tex) = &self.s1 else { return false };
        let frames: &[Rect] = if fiery { &BLAST } else { &POP };
        let Some(&src) = frames.get((age / 0.07) as usize) else { return false };
        blit(tex, src, at, SCALE, WHITE);
        true
    }

    /// The shield around Sonic (it flickers, as in the game).
    pub fn shield(&self, at: Vec2, t: f32) -> bool {
        let Some(tex) = &self.s1 else { return false };
        let k = (t * 30.0) as usize;
        if k.is_multiple_of(2) {
            blit(tex, SHIELD[(k / 2) % 3], at, SCALE, WHITE);
        }
        true
    }

    /// Invincibility stars around Sonic.
    pub fn stars(&self, at: Vec2, t: f32) -> bool {
        let Some(tex) = &self.s1 else { return false };
        blit(tex, STARS[(t * 15.0) as usize % 4], at, SCALE, WHITE);
        true
    }

    /// A score popup, if the sheet has that number.
    pub fn score(&self, score: &str, at: Vec2, alpha: f32) -> bool {
        let Some(tex) = &self.s1 else { return false };
        let Some(&(_, src)) = SCORES.iter().find(|(n, _)| n.to_string() == score) else { return false };
        blit(tex, src, at, SCALE, Color::new(1.0, 1.0, 1.0, alpha));
        true
    }

    /// "GAME OVER" centred on `at` (screen pass), `k` px per sheet px.
    pub fn game_over(&self, at: Vec2, k: f32) -> bool {
        let Some(tex) = &self.s1 else { return false };
        blit(tex, GAME_OVER, at, k, WHITE);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_lie_inside_their_sheets() {
        let inside = |r: &Rect, (w, h): (f32, f32)| r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= w && r.y + r.h <= h;
        let s1 = (712.0, 1014.0);
        let mut all: Vec<Rect> = vec![SPIKE, POST, POST_BLUE, POST_RED, SIGN_ROBOTNIK, SIGN_SONIC, SIGN_EDGE];
        all.extend([SPRING, SPRING_OUT, GAME_OVER]);
        all.extend(RING.iter().chain(&RING_SPARKLE).chain(&SIGN_TURN).chain(&POP).chain(&BLAST));
        all.extend(SHIELD.iter().chain(&STARS));
        all.extend(SCORES.iter().map(|s| s.1));
        all.extend(S1_MONITOR.icons.iter().chain(S1_MONITOR.statics));
        all.extend([S1_MONITOR.body, S1_MONITOR.broken]);
        for r in &all {
            assert!(inside(r, s1), "{r:?}");
        }
        let s3 = (822.0, 813.0);
        for r in S3_MONITOR.icons.iter().chain(S3_MONITOR.statics).chain(&[S3_MONITOR.body, S3_MONITOR.broken]) {
            assert!(inside(r, s3), "{r:?}");
        }
        assert!(inside(&S3_TAILS, s3) && inside(&S3_KNUCKLES, s3));
        // Both monitors' screens hold a 16 px icon.
        for m in [&S1_MONITOR, &S3_MONITOR] {
            assert!(m.screen.x + 16.0 <= m.body.w && m.screen.y + 16.0 <= m.body.h);
        }
        assert_eq!(sheet_name((712, 1014)), Some(OBJECTS));
        assert_eq!(sheet_name((690, 1558)), None);
    }
}
