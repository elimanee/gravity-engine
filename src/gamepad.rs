//! Game controllers, without extra libraries: the kernel's joystick devices
//! (`/dev/input/js*`) on Linux and XInput on Windows. One controller is
//! read (the first found), with Xbox-style names for its buttons; rumble
//! works where the system offers it (XInput).

/// A controller button, by its place on an Xbox-style pad.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub enum Button {
    /// A / Cross.
    South,
    /// B / Circle.
    East,
    /// X / Square.
    West,
    /// Y / Triangle.
    North,
    LeftShoulder,
    RightShoulder,
    /// Back / Select / Share.
    Select,
    Start,
}

/// Stick and d-pad positions (−1‥1) and the buttons held.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct State {
    pub lx: f32,
    pub ly: f32,
    pub dx: f32,
    pub dy: f32,
    buttons: u16,
}

/// How far a stick must lean to count as a direction.
const DEADZONE: f32 = 0.45;

impl State {
    pub fn held(&self, b: Button) -> bool {
        self.buttons & (1 << b as u16) != 0
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn set(&mut self, b: Button, down: bool) {
        if down {
            self.buttons |= 1 << b as u16;
        } else {
            self.buttons &= !(1 << b as u16);
        }
    }

    pub fn left(&self) -> bool {
        self.dx < -0.5 || self.lx < -DEADZONE
    }
    pub fn right(&self) -> bool {
        self.dx > 0.5 || self.lx > DEADZONE
    }
    pub fn up(&self) -> bool {
        self.dy < -0.5 || self.ly < -DEADZONE
    }
    pub fn down(&self) -> bool {
        self.dy > 0.5 || self.ly > DEADZONE
    }

    /// A, B or Y jump (X is the action button: Amy's hammer).
    pub fn jump(&self) -> bool {
        [Button::South, Button::East, Button::North].into_iter().any(|b| self.held(b))
    }
}

#[derive(Default)]
pub struct Gamepad {
    now: State,
    before: State,
    /// Name of the controller in use.
    pub name: Option<String>,
    backend: backend::Backend,
}

impl Gamepad {
    /// Read the controller (once a frame).
    pub fn poll(&mut self) {
        self.before = self.now;
        match self.backend.poll(&mut self.now) {
            Some(name) => self.name = Some(name),
            None => {
                self.name = None;
                self.now = State::default();
            }
        }
    }

    pub fn connected(&self) -> bool {
        self.name.is_some()
    }

    pub fn state(&self) -> State {
        self.now
    }

    /// Pressed since the last frame.
    pub fn pressed(&self, b: Button) -> bool {
        self.now.held(b) && !self.before.held(b)
    }

    /// Any face button pressed since the last frame.
    pub fn jump_pressed(&self) -> bool {
        self.now.jump() && !self.before.jump()
    }

    /// Shake the controller (`strength` 0‥1) for `secs` seconds, where possible.
    pub fn rumble(&mut self, strength: f32, secs: f32) {
        self.backend.rumble(strength.clamp(0.0, 1.0), secs.max(0.0));
    }
}

#[cfg(target_os = "linux")]
mod backend {
    //! The kernel joystick interface: 8-byte events (time, value, type, number).

    use super::{Button, State};
    use std::fs::File;
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    use std::time::Instant;

    const O_NONBLOCK: i32 = 0o4000;
    const JS_EVENT_BUTTON: u8 = 0x01;
    const JS_EVENT_AXIS: u8 = 0x02;
    const JS_EVENT_INIT: u8 = 0x80;

    #[derive(Default)]
    pub struct Backend {
        dev: Option<Device>,
        /// When devices were last looked for.
        scanned: Option<Instant>,
    }

    struct Device {
        file: File,
        name: String,
        /// PlayStation pads number their buttons differently.
        sony: bool,
    }

    fn open(i: usize) -> Option<Device> {
        let file =
            std::fs::OpenOptions::new().read(true).custom_flags(O_NONBLOCK).open(format!("/dev/input/js{i}")).ok()?;
        let name = std::fs::read_to_string(format!("/sys/class/input/js{i}/device/name"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| format!("Controller {i}"));
        let lower = name.to_lowercase();
        let sony =
            ["sony", "playstation", "wireless controller", "dualsense", "dualshock"].iter().any(|k| lower.contains(k));
        Some(Device { file, name, sony })
    }

    fn button(sony: bool, n: u8) -> Option<Button> {
        Some(match (sony, n) {
            (_, 0) => Button::South,
            (_, 1) => Button::East,
            (false, 2) | (true, 3) => Button::West,
            (false, 3) | (true, 2) => Button::North,
            (_, 4) => Button::LeftShoulder,
            (_, 5) => Button::RightShoulder,
            (false, 6) | (true, 8) => Button::Select,
            (false, 7) | (true, 9) => Button::Start,
            _ => return None,
        })
    }

    /// One joystick event: time (u32), value (i16), type, number.
    pub fn apply(state: &mut State, sony: bool, e: &[u8; 8]) {
        let value = i16::from_le_bytes([e[4], e[5]]);
        let (kind, number) = (e[6] & !JS_EVENT_INIT, e[7]);
        if kind == JS_EVENT_BUTTON {
            if let Some(b) = button(sony, number) {
                state.set(b, value != 0);
            }
        } else if kind == JS_EVENT_AXIS {
            let v = value as f32 / 32767.0;
            match number {
                0 => state.lx = v,
                1 => state.ly = v,
                6 => state.dx = v,
                7 => state.dy = v,
                _ => {}
            }
        }
    }

    impl Backend {
        pub fn poll(&mut self, state: &mut State) -> Option<String> {
            if self.dev.is_none() {
                // Look for a controller every two seconds.
                if self.scanned.is_some_and(|t| t.elapsed().as_secs_f32() < 2.0) {
                    return None;
                }
                self.scanned = Some(Instant::now());
                self.dev = (0..4).find_map(open);
                *state = State::default();
            }
            let dev = self.dev.as_mut()?;
            let mut buf = [0u8; 8 * 32];
            loop {
                match dev.file.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        for e in buf[..n].as_chunks::<8>().0 {
                            apply(state, dev.sony, e);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => {
                        // Unplugged.
                        self.dev = None;
                        return None;
                    }
                }
            }
            Some(dev.name.clone())
        }

        pub fn rumble(&mut self, _strength: f32, _secs: f32) {}
    }
}

#[cfg(windows)]
mod backend {
    //! XInput (Xbox controllers and anything that pretends to be one).

    use super::{Button, State};
    use std::time::Instant;
    use windows_sys::Win32::UI::Input::XboxController::*;

    #[derive(Default)]
    pub struct Backend {
        connected: bool,
        scanned: Option<Instant>,
        rumble_until: Option<Instant>,
    }

    impl Backend {
        pub fn poll(&mut self, state: &mut State) -> Option<String> {
            // Asking an empty slot is slow: only every two seconds.
            if !self.connected && self.scanned.is_some_and(|t| t.elapsed().as_secs_f32() < 2.0) {
                return None;
            }
            self.scanned = Some(Instant::now());
            // SAFETY: XInputGetState fills a plain struct we own.
            let mut s: XINPUT_STATE = unsafe { std::mem::zeroed() };
            let ok = unsafe { XInputGetState(0, &mut s) } == 0;
            self.connected = ok;
            if !ok {
                return None;
            }
            let g = s.Gamepad;
            let w = g.wButtons;
            let flag = |f: u16| w & f != 0;
            let axis = |v: i16| (v as f32 / 32767.0).clamp(-1.0, 1.0);
            state.lx = axis(g.sThumbLX);
            state.ly = -axis(g.sThumbLY);
            state.dx = flag(XINPUT_GAMEPAD_DPAD_RIGHT) as i32 as f32 - flag(XINPUT_GAMEPAD_DPAD_LEFT) as i32 as f32;
            state.dy = flag(XINPUT_GAMEPAD_DPAD_DOWN) as i32 as f32 - flag(XINPUT_GAMEPAD_DPAD_UP) as i32 as f32;
            for (b, f) in [
                (Button::South, XINPUT_GAMEPAD_A),
                (Button::East, XINPUT_GAMEPAD_B),
                (Button::West, XINPUT_GAMEPAD_X),
                (Button::North, XINPUT_GAMEPAD_Y),
                (Button::LeftShoulder, XINPUT_GAMEPAD_LEFT_SHOULDER),
                (Button::RightShoulder, XINPUT_GAMEPAD_RIGHT_SHOULDER),
                (Button::Select, XINPUT_GAMEPAD_BACK),
                (Button::Start, XINPUT_GAMEPAD_START),
            ] {
                state.set(b, flag(f));
            }
            if self.rumble_until.is_some_and(|t| Instant::now() >= t) {
                self.rumble_until = None;
                self.set_motors(0);
            }
            Some("Xbox controller".to_string())
        }

        fn set_motors(&self, speed: u16) {
            let v = XINPUT_VIBRATION { wLeftMotorSpeed: speed, wRightMotorSpeed: speed };
            // SAFETY: a plain struct passed by pointer.
            unsafe { XInputSetState(0, &v) };
        }

        pub fn rumble(&mut self, strength: f32, secs: f32) {
            if !self.connected {
                return;
            }
            self.set_motors((strength * 65535.0) as u16);
            self.rumble_until = Some(Instant::now() + std::time::Duration::from_secs_f32(secs));
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod backend {
    use super::State;

    #[derive(Default)]
    pub struct Backend;

    impl Backend {
        pub fn poll(&mut self, _state: &mut State) -> Option<String> {
            None
        }
        pub fn rumble(&mut self, _strength: f32, _secs: f32) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directions_from_sticks_and_dpad() {
        let mut s = State { lx: 0.8, ..State::default() };
        assert!(s.right() && !s.left());
        s.lx = 0.2;
        assert!(!s.right(), "inside the dead zone");
        s.dy = 1.0;
        assert!(s.down());
        s.set(Button::East, true);
        assert!(s.jump() && s.held(Button::East) && !s.held(Button::South));
        s.set(Button::East, false);
        s.set(Button::West, true);
        assert!(!s.jump(), "X is the action button");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_joystick_events() {
        let ev = |value: i16, kind: u8, number: u8| {
            let v = value.to_le_bytes();
            [0, 0, 0, 0, v[0], v[1], kind, number]
        };
        let mut s = State::default();
        backend::apply(&mut s, false, &ev(1, 0x81, 7));
        assert!(s.held(Button::Start), "initial state counts too");
        backend::apply(&mut s, true, &ev(1, 0x01, 2));
        assert!(s.held(Button::North), "PlayStation triangle");
        backend::apply(&mut s, false, &ev(-32767, 0x02, 6));
        assert!(s.left());
        backend::apply(&mut s, false, &ev(0, 0x02, 6));
        assert!(!s.left());
    }
}
