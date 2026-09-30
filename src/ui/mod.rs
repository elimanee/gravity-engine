//! User interface. Components compute their layout from the current state,
//! consume pointer input first (top-most first) and emit `Action`s that the
//! app applies.

pub mod challenge_bar;
pub mod context_menu;
pub mod cursor;
pub mod debug;
pub mod drawer;
pub mod editor_bar;
pub mod help;
pub mod hud;
pub mod icons;
pub mod inspector;
pub mod library;
pub mod now_playing;
pub mod skin_player;
pub mod spawner;
pub mod theme;
pub mod title;
pub mod toasts;
pub mod tool_card;
pub mod tool_picker;
pub mod visualizer;
pub mod widgets;

use crate::physics::tools::Tool;
use macroquad::prelude::*;
use rapier2d::prelude::RigidBodyHandle;

/// Pointer / modifier snapshot for one frame.
pub struct Input {
    /// Pointer in screen pixels (for the UI).
    pub mouse: Vec2,
    /// Pointer in world pixels (for the scene; see `camera`).
    pub world: Vec2,
    pub middle_down: bool,
    pub left_pressed: bool,
    pub left_down: bool,
    pub left_released: bool,
    pub right_pressed: bool,
    pub right_down: bool,
    pub wheel: f32,
    pub shift: bool,
    pub ctrl: bool,
    /// A UI element claimed this frame's press.
    pub consumed: bool,
    /// The pointer is over a UI surface (world hover effects are suppressed).
    pub over_ui: bool,
}

impl Input {
    pub fn gather() -> Self {
        let (mx, my) = mouse_position();
        let wheel = mouse_wheel().1;
        Input {
            mouse: vec2(mx, my),
            world: vec2(mx, my),
            middle_down: is_mouse_button_down(MouseButton::Middle),
            left_pressed: is_mouse_button_pressed(MouseButton::Left),
            left_down: is_mouse_button_down(MouseButton::Left),
            left_released: is_mouse_button_released(MouseButton::Left),
            right_pressed: is_mouse_button_pressed(MouseButton::Right),
            right_down: is_mouse_button_down(MouseButton::Right),
            wheel: if wheel.abs() > 0.0 { wheel.signum() } else { 0.0 },
            shift: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
            ctrl: is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl),
            consumed: false,
            over_ui: false,
        }
    }

    /// Mark the pointer as over `r`; returns whether it is.
    pub fn hover(&mut self, r: Rect) -> bool {
        let inside = r.contains(self.mouse);
        if inside {
            self.over_ui = true;
        }
        inside
    }

    /// Claim a left press inside `r` (so the world never sees it).
    pub fn block(&mut self, r: Rect) {
        if self.hover(r) && self.left_pressed {
            self.consumed = true;
        }
        if r.contains(self.mouse) {
            self.wheel = 0.0;
        }
    }
}

/// Commands emitted by UI components.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    SetGravity(f32),
    SetTool(Tool),
    CycleBorder(i32),
    CycleBackground(i32),
    PickBackground,
    AddImages,
    ToggleSpawner,
    OpenToolPicker,
    LoadAudio,
    ToggleAudio,
    FetchButtons,
    FetchLogos,
    ClearAll,
    SaveScene,
    LoadScene,
    Screenshot,
    TogglePause,
    ToggleHelp,
    ResetSettings,
    /// Cycle the background visualizer (Off / Bars / Wave / Radial).
    CycleVisualizer(i32),
    /// Cycle the style of visualizer objects.
    CycleVisualizerObject(i32),
    /// Drop a visualizer object into the scene.
    SpawnVisualizer,
    ToggleWater,
    Undo,
    Redo,
    ToggleRecording,
    ToggleEffects,
    /// Remove every poured grain.
    ClearGrains,
    /// Open the challenge editor on the current scene (or on "My
    /// challenges" entry `Some(i)`).
    OpenEditor(Option<usize>),
    Editor(EditorCmd),
    /// Play "My challenges" entry `i`.
    StartCustom(usize),
    CycleWorldSize(i32),
    Selection(SelectionCmd),
    /// Drop a ragdoll at the pointer.
    SpawnRagdoll,
    /// Show / hide the classic player window.
    TogglePlayer,
    Player(PlayerCmd),
    /// Next / previous installed skin (the built-in look included).
    CycleSkin(i32),
    /// Pick a .wsz skin file.
    LoadSkin,
    OpenLibrary,
    LoadExample(usize),
    StartChallenge(usize),
    ChallengeGo,
    ChallengeRetry,
    ChallengeNext,
    ChallengeExit,
    Object(ObjectCmd, RigidBodyHandle),
}

/// Classic player window commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlayerCmd {
    Previous,
    Play,
    Pause,
    Stop,
    Next,
    /// Load music.
    Eject,
    Close,
    DoubleSize,
    /// Jump to this fraction of the song.
    Seek(f32),
    Volume(f32),
    ToggleEq,
    TogglePlaylist,
    Shuffle,
    Repeat,
    /// Equalizer on / off.
    EqOn,
    /// Slider `i` (0 = preamp, 1‥10 = bands) to this many dB.
    EqGain(usize, f32),
    /// Next equalizer preset.
    EqPreset,
    PlayEntry(usize),
    RemoveEntry(usize),
    MoveEntry(usize, usize),
    AddFiles,
    SortPlaylist,
    ClearPlaylist,
}

/// Challenge editor commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditorCmd {
    PlaceBall,
    PlaceGoal,
    /// Change the ink budget by this many steps.
    Ink(i32),
    Test,
    Save,
    Exit,
}

/// Commands for the Select tool's selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionCmd {
    Duplicate,
    Delete,
    TogglePin,
    Glue,
    Copy,
    Paste,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectCmd {
    Resize(f32),
    SizeAll(f32),
    Duplicate,
    TogglePin,
    Properties,
    Unlink,
    Delete,
}

/// Open/close animation helper.
#[derive(Default)]
pub struct Fader {
    pub open: bool,
    t: f32,
}

impl Fader {
    pub fn update(&mut self, dt: f32, speed: f32) {
        let target = if self.open { 1.0 } else { 0.0 };
        let step = dt * speed;
        self.t = if self.t < target { (self.t + step).min(target) } else { (self.t - step).max(target) };
    }

    /// Eased visibility, 0‥1.
    pub fn value(&self) -> f32 {
        theme::ease_out_cubic(self.t)
    }

    pub fn visible(&self) -> bool {
        self.t > 0.001
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }
}
