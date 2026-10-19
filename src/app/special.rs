//! Special stages in the app: through the giant ring after an act's
//! tally, played over the whole screen; an emerald won is saved, then the
//! campaign goes on.

use super::*;
use crate::audio::music::Jingle;
use crate::audio::sfx::Sound;
use crate::special::{self, Event, Outcome, Stage};

pub(super) struct SpecialRun {
    pub stage: Stage,
    /// The act to play afterwards (none: back to the library).
    then: Option<usize>,
}

impl App {
    /// Into the special stage of the next emerald not found yet.
    pub(super) fn start_special(&mut self, then: Option<usize>) {
        let Some(i) = special::next_missing(self.s.emeralds) else { return };
        self.stop_game();
        self.special = Some(SpecialRun { stage: Stage::new(i), then });
        self.sound(Sound::Warp, vec2(screen_width() / 2.0, screen_height() / 2.0), 0.8);
    }

    /// Special stage `i` on its own (`--special-stage`).
    pub fn play_special(&mut self, i: usize) {
        if i < special::EMERALDS as usize {
            self.special = Some(SpecialRun { stage: Stage::new(i), then: None });
        }
    }

    /// A frame of the special stage: controls, sounds, the picture.
    pub(super) fn special_frame(&mut self, dt: f32) {
        let gp = self.pad.state();
        let input = special::Input {
            left: is_key_down(KeyCode::Left) || gp.left(),
            right: is_key_down(KeyCode::Right) || gp.right(),
            forward: is_key_down(KeyCode::Up) || gp.up(),
            jump_pressed: is_key_pressed(KeyCode::Space) || self.pad.jump_pressed(),
        };
        let go_on = is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Enter)
            || self.pad.pressed(crate::gamepad::Button::South);
        let Some(run) = &mut self.special else { return };
        let events = run.stage.update(dt, input);
        let ended = match run.stage.outcome {
            Some(Outcome::Won(t)) | Some(Outcome::Lost(t)) => t > 2.0,
            None => false,
        };
        let index = run.stage.index;
        special::art::draw(&run.stage, self.s.character, false, self.s.emeralds, get_time() as f32);
        let centre = vec2(screen_width() / 2.0, screen_height() * 0.7);
        for e in events {
            match e {
                Event::Blue => self.sound(Sound::Sphere, centre, 0.35),
                Event::Ring => self.sound(Sound::Ring, centre, 0.4),
                Event::Bumper => self.sound(Sound::Bumper, centre, 0.6),
                Event::Spring => self.sound(Sound::Spring, centre, 0.6),
                Event::Jump => self.sound(Sound::Jump, centre, 0.4),
                Event::Won => {
                    self.s.emeralds |= 1 << index;
                    let _ = self.s.save();
                    self.play_jingle(Jingle::Emerald);
                    self.pad.rumble(0.5, 0.4);
                }
                Event::Lost => {
                    self.sound(Sound::Hurt, centre, 0.6);
                    self.pad.rumble(0.7, 0.3);
                }
            }
        }
        if is_key_pressed(KeyCode::Escape) || (ended && go_on) {
            self.finish_special();
        }
    }

    fn finish_special(&mut self) {
        let Some(run) = self.special.take() else { return };
        match run.then {
            Some(next) => self.open_act(next),
            None => {
                self.library.fader.open = true;
                let n = special::count(self.s.emeralds);
                self.toasts.success(format!("Chaos emeralds: {n} of {}", special::EMERALDS));
            }
        }
    }
}
