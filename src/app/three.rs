//! The 3D mode in the app: the endless run or the open zone over the whole
//! screen, with the keyboard or a controller; Esc goes back.

use super::*;
use crate::audio::sfx::Sound;
use crate::three::world::{Input as Hands, Looks, Maker, Noise, World3};
use crate::three::zone::Zone;

pub(super) struct Three {
    pub world: World3,
    looks: Looks,
    /// The open zone (else the endless run).
    pub zone: bool,
}

/// A new world: the zone, or a new course for the run (from `seed`, or a
/// new one every time).
fn maker(zone: bool, seed: Option<u32>) -> Box<dyn Maker> {
    let fresh = (get_time() * 1000.0) as u32 ^ (miniquad::date::now() * 1000.0) as u32;
    if zone {
        Box::new(Zone::new(7))
    } else {
        Box::new(crate::three::course::Course::new(seed.unwrap_or(fresh).max(1)))
    }
}

impl App {
    /// Into the 3D mode: the open zone or the endless run.
    pub fn play_3d(&mut self, zone: bool) {
        self.play_3d_seeded(zone, None);
    }

    /// The same, the run's course from `seed` (the benchmark's is always the same).
    pub(super) fn play_3d_seeded(&mut self, zone: bool, seed: impl Into<Option<u32>>) {
        let seed = seed.into();
        let Some(looks) = Looks::new(self.s.character) else {
            self.toasts.error("3D drawing is not available here");
            return;
        };
        self.stop_game();
        self.library.fader.open = false;
        let n = looks.pack.count();
        if n > 0 {
            self.toasts.status("models", format!("3D models: {n} from your pack"));
        }
        let world = World3::new(maker(zone, seed), self.s.character, self.s.run_3d_best);
        self.three = Some(Three { world, looks, zone });
    }

    pub(super) fn three_frame(&mut self, dt: f32) {
        let gp = self.pad.state();
        let key = |k: &[KeyCode]| k.iter().any(|k| is_key_down(*k));
        let pressed = |k: &[KeyCode]| k.iter().any(|k| is_key_pressed(*k));
        let mut stick = vec2(
            key(&[KeyCode::Right, KeyCode::D]) as i32 as f32 - key(&[KeyCode::Left, KeyCode::A]) as i32 as f32,
            key(&[KeyCode::Up, KeyCode::W]) as i32 as f32 - key(&[KeyCode::Down, KeyCode::S]) as i32 as f32,
        );
        let pad = vec2(gp.lx + gp.dx, -(gp.ly + gp.dy));
        if pad.length() > 0.2 {
            stick = pad;
        }
        use crate::gamepad::Button;
        let action_keys = [KeyCode::LeftShift, KeyCode::RightShift, KeyCode::X, KeyCode::C];
        let mut turn = key(&[KeyCode::E]) as i32 as f32 - key(&[KeyCode::Q]) as i32 as f32;
        turn += gp.held(Button::RightShoulder) as i32 as f32 - gp.held(Button::LeftShoulder) as i32 as f32;
        if is_mouse_button_down(MouseButton::Right) {
            turn += mouse_delta_position().x * -40.0;
        }
        let hands = Hands {
            stick,
            jump: key(&[KeyCode::Space]) || gp.jump(),
            jump_pressed: pressed(&[KeyCode::Space]) || self.pad.jump_pressed(),
            action: key(&action_keys) || gp.held(Button::West),
            action_pressed: pressed(&action_keys) || self.pad.pressed(Button::West),
            turn,
            spawn: pressed(&[KeyCode::N]),
            blast: pressed(&[KeyCode::B]),
            again: pressed(&[KeyCode::Space, KeyCode::Enter]) || self.pad.pressed(Button::South),
            pause: pressed(&[KeyCode::P]) || self.pad.pressed(Button::Start),
        };
        let bench = self.bench_frame();
        let Some(t) = &mut self.three else { return };
        let hands = match bench {
            Some(frame) => super::bench::hands_3d(frame, t.zone),
            None => hands,
        };
        if t.world.over && hands.again && bench.is_none() {
            let zone = t.zone;
            self.s.run_3d_best = self.s.run_3d_best.max(t.world.distance);
            let _ = self.s.save();
            t.world.restart(maker(zone, None));
        } else {
            let _t = crate::profile::span("3d/update");
            t.world.update(dt, hands);
        }
        {
            let _t = crate::profile::span("3d/draw");
            t.world.draw(&t.looks);
        }
        let noises = std::mem::take(&mut t.world.noises);
        let distance = t.world.distance;
        let over = t.world.over;
        let centre = vec2(screen_width() / 2.0, screen_height() / 2.0);
        for n in noises {
            let (sound, volume) = match n {
                Noise::Jump => (Sound::Jump, 0.4),
                Noise::Rev => (Sound::Rev, 0.4),
                Noise::Dash => (Sound::Dash, 0.5),
                Noise::Skid => (Sound::Skid, 0.4),
                Noise::Spring => (Sound::Spring, 0.6),
                Noise::Ring => (Sound::Ring, 0.4),
                Noise::Hurt => (Sound::Hurt, 0.6),
                Noise::Scatter => (Sound::Scatter, 0.6),
                Noise::Pop => (Sound::Pop, 0.7),
                Noise::Boom => (Sound::Boom, 0.6),
                Noise::Thud => (Sound::Pop, 0.3),
                Noise::Shot => (Sound::Shot, 0.5),
                Noise::Mark => (Sound::Post, 0.6),
            };
            self.sound(sound, centre, volume);
        }
        if over && distance > self.s.run_3d_best && bench.is_none() {
            self.s.run_3d_best = distance;
            let _ = self.s.save();
        }
        if is_key_pressed(KeyCode::Escape) || self.pad.pressed(Button::Select) {
            self.three = None;
            if self.char_test.is_some() {
                // Trying a character: back to the editor.
                self.end_char_test();
            } else {
                self.library.fader.open = true;
            }
        }
    }
}
