//! Application state and the per-frame loop: input → UI → actions →
//! simulation → rendering.

use crate::audio::Audio;
use crate::background::{Background, BgMode};
use crate::camera::{self, Camera, View};
use crate::config::{self, ext_of, file_name_of, IMAGE_EXT, SCENE_EXT};
use crate::config::{DENSITY, PPM, WALL_T};
use crate::drawing;
use crate::effects::Effects;
use crate::history::{History, Snapshot};
use crate::net::{FetchEvent, FetchJob, FetchKind};
use crate::physics::grains::Grains;
use crate::physics::links::{self, Link, LinkKind};
use crate::physics::magnets;
use crate::physics::object::{object_at, objects_at, Material, Object, Placement, Source, Visual};
use crate::physics::planets;
use crate::physics::tools::{self, Grab, Tool};
use crate::physics::water::{self, Water};
use crate::physics::zones::{self, PortalState, Zone};
use crate::physics::{to_phys, PhysWorld};
use crate::recorder::{self, Finishing, Recording};
use crate::scene;
use crate::settings::{Settings, DRAW_THICKNESS_RANGE, RADIUS_RANGE, SPAWN_SIZE_RANGE, STRENGTH_RANGE};
use crate::shapes::{self, Shape};
use crate::ui::challenge_bar::ChallengeBar;
use crate::ui::context_menu::ContextMenu;
use crate::ui::cursor::{self, Blast};
use crate::ui::drawer::Drawer;
use crate::ui::editor_bar::EditorBar;
use crate::ui::help::Help;
use crate::ui::hud::{Hud, HudState};
use crate::ui::inspector::{Inspector, Props};
use crate::ui::library::{Library, LibraryData};
use crate::ui::now_playing::NowPlayingPill;
use crate::ui::skin_player::SkinPlayer;
use crate::ui::spawner::{spawn_color, Spawner};
use crate::ui::title::{self, PixelOut};
use crate::ui::toasts::Toasts;
use crate::ui::tool_card::ToolCard;
use crate::ui::tool_picker::ToolPicker;
use crate::ui::visualizer::{self, VisStyle};
use crate::ui::{debug, icons, theme, Action, Input, ObjectCmd, SelectionCmd};
use crate::weather::{Sky, Weather};
use crate::window_tracker::WindowTracker;
use macroquad::prelude::*;
use rapier2d::prelude::{Point, RigidBodyHandle};
use rfd::FileDialog;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

mod build;
mod editor;
mod fire;
mod gadgets;
mod grapple;
mod impacts;
mod juice;
mod knife;
mod library;
mod night;
mod player;
mod player_body;
mod ragdoll;
mod rewind;
mod select;
mod soft;
mod verify;
mod weather;

const TIME_STEPS: &[f32] = &[0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0];
const SPAWN_REPEAT: f32 = 0.11;
/// Grains poured per second.
const POUR_RATE: f32 = 170.0;

type SpawnKey = (Shape, u32, (u8, u8, u8));

/// A Draw-tool stroke in progress.
struct Stroke {
    points: Vec<(f32, f32)>,
    rgb: [u8; 3],
}

/// A Link-tool drag in progress.
struct LinkDrag {
    /// Object where the drag started (`None` for the background), with the
    /// grabbed point in its local frame so it follows the object.
    from: Option<(RigidBodyHandle, Point<f32>)>,
    /// World point where the drag started.
    start: Point<f32>,
    start_px: Vec2,
}

enum Title {
    Showing,
    Leaving(PixelOut),
}

pub struct App {
    s: Settings,
    world: PhysWorld,
    objects: Vec<Object>,
    links: Vec<Link>,
    zones: Vec<Zone>,
    portals: PortalState,
    history: History<Snapshot>,
    water: Water,
    effects: Effects,
    bg: Background,
    camera: Camera,
    /// Pointer position of the last frame of a middle-button pan.
    pan_last: Option<Vec2>,
    /// Bodies picked with the Select tool.
    selection: Vec<RigidBodyHandle>,
    select_drag: Option<select::SelectDrag>,
    /// Last copied selection (Ctrl+C).
    clipboard: Option<Snapshot>,
    audio: Audio,
    shaker: WindowTracker,
    jobs: Vec<FetchJob>,

    grab: Option<Grab>,
    field_active: bool,
    paused: bool,
    step_once: bool,
    blasts: Vec<Blast>,
    spawn_timer: f32,
    spawn_cache: HashMap<SpawnKey, Visual>,
    stroke: Option<Stroke>,
    link_drag: Option<LinkDrag>,
    /// Corner where a Zone-tool drag started.
    zone_drag: Option<Vec2>,
    /// Pieces of recently shattered objects, with the time they appeared.
    fresh: HashMap<RigidBodyHandle, f64>,
    recording: Option<Recording>,
    finishing: Option<Finishing>,
    slow: juice::SlowMo,
    rewind: rewind::Rewind,
    knife: knife::Knife,
    fire: fire::Fire,
    /// Jellies and cloths.
    softs: Vec<crate::physics::soft::Soft>,
    /// Gadgets (lasers, thrusters, cannons, lamps, hooks), a Gadget-tool drag, this frame's
    /// laser beams, and the colour of glass the beams cross.
    gadgets: Vec<crate::physics::gadgets::Gadget>,
    gadget_drag: Option<gadgets::GadgetDrag>,
    beams: Vec<crate::physics::gadgets::Beam>,
    glass_tints: HashMap<RigidBodyHandle, Color>,
    /// Seconds until the thrusters' roar is played again.
    thrust_sound: f32,
    grains: Grains,
    /// Rain, snow and lightning: their look, and what they do.
    sky: Sky,
    /// The night's light map.
    lights: crate::lighting::Lights,
    climate: weather::Climate,
    /// The Pour tool is pouring.
    pouring: bool,
    /// Seconds until the next pouring sound.
    pour_sound: f32,
    /// An undo step was already recorded for the open properties panel.
    inspector_recorded: bool,

    hud: Hud,
    drawer: Drawer,
    card: ToolCard,
    picker: ToolPicker,
    spawner: Spawner,
    menu: ContextMenu,
    inspector: Inspector,
    library: Library,
    challenge_bar: ChallengeBar,
    challenge: Option<library::ChallengeRun>,
    editor: Option<editor::Editor>,
    editor_bar: EditorBar,
    /// Classic player window and its skin (loaded when first shown).
    player: SkinPlayer,
    /// The player windows as a physical object (Shift+X).
    player_body: Option<player_body::PlayerBody>,
    skin: Option<crate::skin::Skin>,
    skin_tried: bool,
    /// "My challenges", read from the challenge folder.
    custom: Vec<(std::path::PathBuf, crate::library::custom::ChallengeFile)>,
    help: Help,
    toasts: Toasts,
    now_playing: NowPlayingPill,
    debug: bool,

    title: Option<Title>,
    last_size: (f32, f32),
    screenshot: bool,
    pub quit: bool,
}

impl App {
    pub fn new(preload: Vec<String>, skip_title: bool) -> Self {
        let s = Settings::load();
        let k = s.world_size as f32;
        let size = (screen_width() * k, screen_height() * k);
        let world = PhysWorld::new(s.gravity, s.border, size);
        let camera = Camera::new(vec2(size.0, size.1), vec2(screen_width(), screen_height()));
        let mut bg = Background::new(s.background);
        let mut toasts = Toasts::default();
        if let Some(path) = s.custom_background.clone() {
            if let Err(e) = bg.load_custom(&path) {
                toasts.warn(format!("Background image unavailable: {e}"));
            }
            bg.mode = s.background;
        }
        if bg.mode == BgMode::Custom && bg.texture.is_none() {
            bg.mode = BgMode::Dark;
        }
        let mut shaker = WindowTracker::start();
        shaker.set_enabled(s.window_shake);

        let mut app = App {
            audio: Audio::new(s.volume),
            title: (!skip_title && s.show_title).then_some(Title::Showing),
            s,
            world,
            objects: vec![],
            links: vec![],
            zones: vec![],
            portals: PortalState::default(),
            history: History::default(),
            water: Water::default(),
            effects: Effects::default(),
            bg,
            camera,
            pan_last: None,
            selection: Vec::new(),
            slow: Default::default(),
            rewind: Default::default(),
            knife: Default::default(),
            fire: Default::default(),
            softs: Vec::new(),
            gadgets: Vec::new(),
            gadget_drag: None,
            beams: Vec::new(),
            glass_tints: HashMap::new(),
            thrust_sound: 0.0,
            grains: Grains::default(),
            sky: Sky::default(),
            lights: Default::default(),
            climate: Default::default(),
            pouring: false,
            pour_sound: 0.0,
            select_drag: None,
            clipboard: None,
            shaker,
            jobs: vec![],
            grab: None,
            field_active: false,
            paused: false,
            step_once: false,
            blasts: vec![],
            spawn_timer: 0.0,
            spawn_cache: HashMap::new(),
            stroke: None,
            link_drag: None,
            zone_drag: None,
            fresh: HashMap::new(),
            recording: None,
            finishing: None,
            inspector_recorded: false,
            hud: Hud::default(),
            drawer: Drawer::default(),
            card: ToolCard::default(),
            picker: ToolPicker::default(),
            spawner: Spawner::default(),
            menu: ContextMenu::default(),
            inspector: Inspector::default(),
            library: Library::default(),
            challenge_bar: ChallengeBar,
            challenge: None,
            editor: None,
            editor_bar: EditorBar::default(),
            player: SkinPlayer::default(),
            player_body: None,
            skin: None,
            skin_tried: false,
            custom: Vec::new(),
            help: Help::default(),
            toasts,
            now_playing: NowPlayingPill::default(),
            debug: false,
            last_size: size,
            screenshot: false,
            quit: false,
        };
        let n = preload.len();
        for (i, path) in preload.into_iter().enumerate() {
            let x = screen_width() / 2.0 + (i as f32 - n as f32 / 2.0) * 40.0;
            let at = app.view().to_world(vec2(x, 120.0));
            app.open_path(&path, at);
        }
        app.history.clear();
        if app.audio.playlist.is_empty() {
            let saved = app.s.playlist.clone();
            app.audio.restore_playlist(&saved);
        }
        app
    }

    fn save_settings(&mut self) {
        self.s.volume = self.s.volume.clamp(0.0, 1.0);
        self.s.playlist = self.audio.playlist.iter().map(|e| e.path.clone()).collect();
        if let Err(e) = self.s.save() {
            eprintln!("could not save settings: {e}");
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Frame
    // ═══════════════════════════════════════════════════════════
    pub fn frame(&mut self) {
        let dt = get_frame_time().min(0.1);
        let (sw, sh) = (screen_width(), screen_height());

        if let Some(Title::Showing) = self.title {
            title::draw_title(sw, sh);
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Q) {
                self.quit = true;
            } else if get_char_pressed().is_some()
                || !get_keys_pressed().is_empty()
                || is_mouse_button_pressed(MouseButton::Left)
                || is_mouse_button_pressed(MouseButton::Right)
            {
                self.title = Some(Title::Leaving(PixelOut::new(sw, sh)));
            }
            return;
        }

        let arena = self.arena();
        if (arena.0 - self.last_size.0).abs() > 1.0 || (arena.1 - self.last_size.1).abs() > 1.0 {
            self.world.resize(arena);
            self.camera.set_arena(vec2(arena.0, arena.1), vec2(sw, sh));
            self.last_size = arena;
        }

        let mut input = Input::gather();
        input.world = self.view().to_world(input.mouse);
        let mut actions = Vec::new();
        let in_transition = self.title.is_some();
        if !in_transition {
            self.keyboard(&input, &mut actions);
        } else {
            input.consumed = true;
        }
        self.update_ui(dt, &mut input, &mut actions);
        self.handle_drops(input.world);
        if !in_transition {
            self.world_input(dt, &mut input);
        }
        for a in actions {
            self.apply(a, input.world);
        }
        self.sync_settings();
        self.poll_jobs();
        self.poll_recording();
        self.audio.tick();
        self.audio.analyze(dt, self.s.vis_gain);
        self.tick_slow(dt);
        // ← rewinds, unless it drives something (then Shift+← does).
        let rewind_key = is_key_down(KeyCode::Left) && (input.shift || !self.listens_to_left());
        let holding = rewind_key && !in_transition && !self.editor_bar.typing();
        if !self.rewind_tick(holding) {
            self.simulate(dt, input.world);
        }
        self.update_challenge(dt);
        if !self.selection.is_empty() {
            let objects = &self.objects;
            self.selection.retain(|h| objects.iter().any(|o| o.body == *h));
        }

        self.draw(dt, &input);

        if let Some(Title::Leaving(po)) = &mut self.title {
            po.draw();
            if po.update(dt) {
                self.title = None;
            }
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Input
    // ═══════════════════════════════════════════════════════════
    fn keyboard(&mut self, input: &Input, actions: &mut Vec<Action>) {
        let pressed = |k| is_key_pressed(k);
        let (shift, ctrl) = (input.shift, input.ctrl);
        if self.editor_bar.typing() {
            return;
        }
        // Over the player windows, Del / Enter act on the playlist.
        let over_player = self.s.player && self.player.hovered();
        if over_player && [KeyCode::Delete, KeyCode::Backspace, KeyCode::Enter].into_iter().any(pressed) {
            return;
        }

        if pressed(KeyCode::F1) {
            actions.push(Action::ToggleHelp);
        }
        if pressed(KeyCode::Escape) {
            if self.stroke.is_some() || self.link_drag.is_some() {
                self.stroke = None;
                self.link_drag = None;
            } else if self.help.fader.open {
                self.help.fader.open = false;
            } else if !self.selection.is_empty() {
                self.selection.clear();
            } else if self.library.fader.open {
                self.library.fader.open = false;
            } else if self.picker.fader.open {
                self.picker.fader.open = false;
            } else if self.menu.fader.open {
                self.menu.close();
            } else if self.inspector.fader.open {
                self.inspector.close();
            } else if self.spawner.fader.open {
                self.spawner.fader.open = false;
            } else if self.paused {
                self.paused = false;
            } else if self.editor.as_ref().is_some_and(|e| e.place.is_some()) {
                if let Some(e) = self.editor.as_mut() {
                    e.place = None;
                    e.goal_drag = None;
                }
            } else if self.challenge.is_some() {
                actions.push(Action::ChallengeExit);
            } else {
                self.quit = true;
            }
            return;
        }
        if ctrl {
            if pressed(KeyCode::S) {
                actions.push(Action::SaveScene);
            }
            if pressed(KeyCode::O) {
                actions.push(Action::LoadScene);
            }
            if pressed(KeyCode::Q) {
                self.quit = true;
            }
            if pressed(KeyCode::Z) {
                actions.push(if shift { Action::Redo } else { Action::Undo });
            }
            if pressed(KeyCode::Y) {
                actions.push(Action::Redo);
            }
            let sel = |c| Action::Selection(c);
            if pressed(KeyCode::C) && !self.selection.is_empty() {
                actions.push(sel(SelectionCmd::Copy));
            }
            if pressed(KeyCode::V) {
                actions.push(sel(SelectionCmd::Paste));
            }
            if pressed(KeyCode::D) && !self.selection.is_empty() {
                actions.push(sel(SelectionCmd::Duplicate));
            }
            if pressed(KeyCode::A) {
                actions.push(Action::SetTool(Tool::Select));
                actions.push(sel(SelectionCmd::All));
            }
            if pressed(KeyCode::G) && !self.selection.is_empty() {
                actions.push(sel(SelectionCmd::Glue));
            }
            return;
        }
        if pressed(KeyCode::Q) {
            self.quit = true;
        }
        let challenge = self.challenge.as_ref().map(|r| (r.started, r.won));
        if pressed(KeyCode::Space) {
            actions.push(match challenge {
                Some((false, _)) => Action::ChallengeGo,
                _ => Action::TogglePause,
            });
        }
        if pressed(KeyCode::Enter) && matches!(challenge, Some((_, true))) {
            actions.push(Action::ChallengeNext);
        }
        if pressed(KeyCode::E) {
            actions.push(if shift { Action::OpenEditor(None) } else { Action::OpenLibrary });
        }
        if pressed(KeyCode::Tab) {
            actions.push(Action::OpenToolPicker);
        }
        let screen = vec2(screen_width(), screen_height());
        let zoom_key = if pressed(KeyCode::Equal) || pressed(KeyCode::KpAdd) {
            1.25
        } else if pressed(KeyCode::Minus) || pressed(KeyCode::KpSubtract) {
            0.8
        } else {
            1.0
        };
        if zoom_key != 1.0 {
            self.camera.zoom_at(zoom_key, screen / 2.0, screen);
            self.zoom_toast();
        }
        if pressed(KeyCode::Home) {
            let (aw, ah) = self.arena();
            self.camera.fit(vec2(aw, ah), screen);
            self.zoom_toast();
        }
        let digits = [
            KeyCode::Key1,
            KeyCode::Key2,
            KeyCode::Key3,
            KeyCode::Key4,
            KeyCode::Key5,
            KeyCode::Key6,
            KeyCode::Key7,
            KeyCode::Key8,
            KeyCode::Key9,
            KeyCode::Key0,
            KeyCode::J,
            KeyCode::Z,
            KeyCode::S,
            KeyCode::K,
            KeyCode::C,
            KeyCode::Y,
            KeyCode::L,
        ];
        for (i, k) in digits.iter().enumerate() {
            if pressed(*k) {
                actions.push(Action::SetTool(Tool::ALL[i]));
                self.picker.fader.open = false;
            }
        }
        if pressed(KeyCode::H) {
            actions.push(if shift { Action::CycleWeather(1) } else { Action::ToggleWater });
        }
        if pressed(KeyCode::X) {
            actions.push(if shift { Action::TogglePlayerPhysics } else { Action::TogglePlayer });
        }
        if pressed(KeyCode::O) {
            actions.push(Action::SpawnRagdoll);
        }
        if pressed(KeyCode::U) {
            actions.push(if shift { Action::SpawnCloth } else { Action::SpawnJelly });
        }
        if pressed(KeyCode::F11) {
            actions.push(Action::ToggleRecording);
        }
        if pressed(KeyCode::I) {
            if let Some(i) = object_at(&self.objects, &self.world, input.world.x, input.world.y) {
                let h = self.objects[i].body;
                actions.push(Action::Object(ObjectCmd::Properties, h));
            }
        }
        let dir = if shift { -1 } else { 1 };
        if pressed(KeyCode::B) {
            actions.push(Action::CycleBorder(dir));
        }
        if pressed(KeyCode::G) {
            actions.push(if shift { Action::PickBackground } else { Action::CycleBackground(1) });
        }
        if pressed(KeyCode::V) {
            actions.push(if shift { Action::SpawnVisualizer } else { Action::CycleVisualizer(1) });
        }
        if pressed(KeyCode::W) {
            self.s.window_shake = !self.s.window_shake;
            self.toasts.status("shake", format!("Window shake {}", if self.s.window_shake { "on" } else { "off" }));
        }
        if pressed(KeyCode::T) && shift {
            actions.push(Action::ToggleNight);
        } else if pressed(KeyCode::T) {
            self.s.trails = !self.s.trails;
            self.toasts.status("trails", format!("Trails {}", if self.s.trails { "on" } else { "off" }));
        }
        if pressed(KeyCode::N) {
            actions.push(Action::ToggleSpawner);
        }
        if pressed(KeyCode::D) {
            self.debug = !self.debug;
        }
        if pressed(KeyCode::P) {
            actions.push(Action::ToggleAudio);
        }
        if pressed(KeyCode::M) {
            actions.push(Action::LoadAudio);
        }
        if pressed(KeyCode::A) {
            actions.push(Action::AddImages);
        }
        if pressed(KeyCode::F) {
            actions.push(if shift { Action::FetchLogos } else { Action::FetchButtons });
        }
        if pressed(KeyCode::R) {
            actions.push(if challenge.is_some() { Action::ChallengeRetry } else { Action::ClearAll });
        }
        if pressed(KeyCode::F12) {
            actions.push(Action::Screenshot);
        }
        if (pressed(KeyCode::Delete) || pressed(KeyCode::Backspace)) && !self.selection.is_empty() {
            actions.push(Action::Selection(SelectionCmd::Delete));
        } else if pressed(KeyCode::Delete) || pressed(KeyCode::Backspace) {
            if let Some(i) = object_at(&self.objects, &self.world, input.world.x, input.world.y) {
                let h = self.objects[i].body;
                actions.push(Action::Object(ObjectCmd::Delete, h));
            } else if let Some(i) = self.soft_at(input.world) {
                self.record("Delete");
                self.remove_soft(i);
            } else {
                self.remove_zone_at(input.world);
            }
        }
        if pressed(KeyCode::LeftBracket) || pressed(KeyCode::RightBracket) {
            let i =
                TIME_STEPS.iter().position(|&v| v >= self.s.time_scale - 0.01).unwrap_or(TIME_STEPS.len() - 1) as i32;
            let d = if pressed(KeyCode::LeftBracket) { -1 } else { 1 };
            self.s.time_scale = TIME_STEPS[(i + d).clamp(0, TIME_STEPS.len() as i32 - 1) as usize];
            self.toasts.status("time", format!("Time scale ×{:.2}", self.s.time_scale));
        }
        if pressed(KeyCode::Period) {
            if self.paused {
                self.step_once = true;
            } else {
                self.toasts.info("Pause first (Space) to step frame by frame");
            }
        }
    }

    fn update_ui(&mut self, dt: f32, input: &mut Input, actions: &mut Vec<Action>) {
        // Top-most first, so overlays swallow clicks meant for them.
        self.help.update(dt, input);
        self.library.update(dt, self.custom.len(), input, actions);
        self.picker.update(dt, input, actions);
        self.menu.update(dt, input, actions);
        self.update_inspector(dt, input);
        self.toasts.update(dt);
        self.drawer.fader.open = self.paused;
        let top = self.hud.bottom();
        self.drawer.update(dt, &mut self.s, top, input, actions);
        self.spawner.update(dt, &mut self.s, input);
        // The tool is fixed during challenges, and the card would hide the level.
        if self.challenge.is_none() {
            self.card.selected = self.selection.len();
            self.card.grains = self.grains.len();
            self.card.update(dt, &mut self.s, input, actions);
        }
        if let Some(v) = self.challenge_view() {
            self.challenge_bar.update(&v, top, input, actions);
        }
        if self.editor_view().is_some() {
            if let Some(ed) = self.editor.as_mut() {
                self.editor_bar.update(&mut ed.name, top, input, actions);
            }
        }
        let np = self.audio.now_playing();
        self.now_playing.update(dt, np.as_ref(), self.paused, input, actions);
        if self.s.player {
            self.ensure_skin();
        }
        self.sync_player_body();
        if self.s.player {
            self.player.animate(dt, &self.audio.analyzer.bands);
            // A physical player sees the pointer in its own, turned frame.
            let local = self.player_local(input.world);
            let screen_mouse = input.mouse;
            if let Some(l) = local {
                input.mouse = l;
            }
            let view = player::view(&self.s, self.skin.as_ref(), &self.audio, np.as_ref());
            self.player.update(&view, input, actions);
            input.mouse = screen_mouse;
            if self.player.throw.take().is_some() {
                self.throw_player(input.world);
            }
        }
        let hud_state = self.hud_state();
        let grabbing = self.grab.is_some()
            || self.field_active
            || self.card.dragging()
            || self.inspector.dragging()
            || self.stroke.is_some()
            || self.link_drag.is_some()
            || self.player.busy();
        let mut hud_actions = vec![];
        self.hud.update(dt, &hud_state, input, grabbing, &mut hud_actions);
        actions.extend(hud_actions);
    }

    /// Values shown in the properties panel for its target object.
    fn inspected(&self) -> Option<Props> {
        let o = self.objects.iter().find(|o| Some(o.body) == self.inspector.target)?;
        Some(Props { material: o.material, mass: o.mass, default_mass: o.area(&self.world) * DENSITY })
    }

    fn update_inspector(&mut self, dt: f32, input: &mut Input) {
        let mut props = self.inspected();
        if !self.inspector.update(dt, input, props.as_mut()) {
            return;
        }
        let Some(p) = props else { return };
        if !self.inspector_recorded {
            self.record("Edit properties");
            self.inspector_recorded = true;
        }
        if let Some(o) = self.objects.iter_mut().find(|o| Some(o.body) == self.inspector.target) {
            if o.material != p.material {
                o.set_material(&mut self.world, p.material);
            }
            if (o.mass - p.mass).abs() > 1e-6 {
                o.set_mass(&mut self.world, p.mass);
            }
        }
    }

    fn hud_state(&self) -> HudState<'static> {
        HudState {
            gravity: self.s.gravity,
            tool: self.s.tool,
            border: self.s.border,
            background: self.bg.mode,
            objects: self.objects.len(),
            paused: self.paused,
            time_scale: self.s.time_scale,
            busy: !self.jobs.is_empty(),
            title: config::APP_NAME,
        }
    }

    fn world_input(&mut self, dt: f32, input: &mut Input) {
        let m = input.world;
        let pos = (m.x, m.y);

        // Ctrl + wheel zooms around the pointer, the middle button pans.
        let screen = vec2(screen_width(), screen_height());
        if input.wheel != 0.0 && input.ctrl && !input.over_ui {
            self.camera.zoom_at(1.2f32.powf(input.wheel), input.mouse, screen);
            self.zoom_toast();
            input.wheel = 0.0;
        }
        if input.middle_down {
            if let Some(last) = self.pan_last {
                self.camera.pan(input.mouse - last, screen);
            }
            self.pan_last = Some(input.mouse);
        } else {
            self.pan_last = None;
        }

        // Mouse wheel adjusts the active tool / spawn size.
        if input.wheel != 0.0 && !input.over_ui {
            if self.spawner.is_open() {
                self.s.spawn_size =
                    (self.s.spawn_size + input.wheel * 8.0).clamp(SPAWN_SIZE_RANGE.0, SPAWN_SIZE_RANGE.1);
            } else if self.s.tool == Tool::Draw {
                self.s.draw_thickness =
                    (self.s.draw_thickness + input.wheel * 2.0).clamp(DRAW_THICKNESS_RANGE.0, DRAW_THICKNESS_RANGE.1);
            } else if self.s.tool.has_settings() {
                if input.shift {
                    self.s.tool_strength =
                        (self.s.tool_strength * 1.15f32.powf(input.wheel)).clamp(STRENGTH_RANGE.0, STRENGTH_RANGE.1);
                } else {
                    self.s.tool_radius =
                        (self.s.tool_radius + input.wheel * 15.0).clamp(RADIUS_RANGE.0, RADIUS_RANGE.1);
                }
            }
        }

        if input.left_pressed && !input.consumed && self.challenge.is_none() && self.editor_press(m) {
            // Placing the challenge ball or goal.
        } else if input.left_pressed && !input.consumed {
            if self.spawner.is_open() {
                self.record("Spawn shapes");
                self.spawn_shape_at(m);
                self.spawn_timer = SPAWN_REPEAT * 2.0;
            } else {
                let tool = self.s.tool;
                if tool == Tool::Draw && self.challenge.as_ref().is_some_and(|r| r.ink_left < 4.0 || r.started) {
                    let why = if self.challenge.as_ref().is_some_and(|r| r.started) {
                        "Retry (R) to draw again"
                    } else {
                        "Out of ink  ·  Retry (R) to start over"
                    };
                    self.toasts.status("ink", why);
                } else if tool == Tool::Draw {
                    let c = spawn_color(self.s.spawn_color, rand::gen_range(0.0, 400.0));
                    let rgb = [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8];
                    self.stroke = Some(Stroke { points: vec![(m.x, m.y)], rgb });
                } else if tool == Tool::Link {
                    self.start_link(m);
                } else if tool == Tool::Zone {
                    self.zone_drag = Some(m);
                } else if tool == Tool::Select {
                    self.select_press(m, input.shift);
                } else if tool == Tool::Pour {
                    self.pouring = true;
                } else if tool == Tool::Knife {
                    self.knife_press(m);
                } else if tool == Tool::Fire {
                    self.fire.lit = true;
                } else if tool == Tool::Gadget {
                    self.gadget_press(m);
                } else if tool.is_field() {
                    self.field_active = true;
                } else if tool == Tool::Bomb {
                    let handles = self.dynamic_bodies();
                    let hits = tools::detonate(
                        &mut self.world.bodies,
                        &handles,
                        pos,
                        self.s.tool_radius,
                        self.s.tool_strength,
                    );
                    self.bomb_hits(m, hits);
                    self.blasts.push(Blast::new(m, self.s.tool_radius));
                } else if tool.grabs() {
                    if let Some(i) = object_at(&self.objects, &self.world, m.x, m.y) {
                        self.grab = Some(Grab::new(&self.world.bodies, self.objects[i].body, to_phys(m.x, m.y), tool));
                    } else {
                        self.grab_soft(m, tool);
                    }
                }
            }
        }

        // Hold to keep spawning.
        if self.spawner.is_open() && input.left_down && !input.over_ui && self.spawn_timer > -1.0 {
            self.spawn_timer -= dt;
            if self.spawn_timer <= 0.0 && !input.consumed {
                self.spawn_shape_at(m + vec2(rand::gen_range(-6.0, 6.0), 0.0));
                self.spawn_timer = SPAWN_REPEAT;
            }
        }
        if input.left_pressed && input.consumed {
            self.spawn_timer = -2.0; // press started on UI: no hold-spawn
        }

        if let Some(last) = self.stroke.as_ref().and_then(|s| s.points.last().copied()) {
            let step = (m.x - last.0).hypot(m.y - last.1);
            if step >= drawing::SAMPLE_SPACING && self.spend_ink(step) {
                if let Some(stroke) = &mut self.stroke {
                    stroke.points.push((m.x, m.y));
                }
            }
        }

        if input.left_down && self.select_drag.is_some() {
            self.select_drag_to(m);
        }
        if input.left_down {
            self.knife_drag(m);
        }

        if input.left_released || !input.left_down {
            if self.select_drag.is_some() {
                self.select_release(m);
            }
            self.pouring = false;
            self.fire.lit = false;
            self.gadget_release(m);
            self.editor_release(m);
            self.knife_release(m);
            if let Some(g) = self.grab.take() {
                g.release(&mut self.world.bodies, to_phys(m.x, m.y));
            }
            self.field_active = false;
            if let Some(stroke) = self.stroke.take() {
                self.finish_stroke(stroke, input.shift);
            }
            if self.link_drag.is_some() {
                self.finish_link(m);
            }
            if self.zone_drag.is_some() {
                self.finish_zone(m);
            }
        }

        if self.s.tool == Tool::Pour && input.right_down && !input.consumed && !input.over_ui {
            self.grains.erase(&mut self.world, m, 26.0);
        } else if input.right_pressed && !input.consumed && !input.over_ui {
            let near_link = (self.s.tool == Tool::Link)
                .then(|| {
                    (0..self.links.len())
                        .map(|i| (i, self.links[i].distance_px(&self.world, m)))
                        .filter(|&(_, d)| d < 8.0)
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                })
                .flatten();
            let on_zone = self.s.tool == Tool::Zone && object_at(&self.objects, &self.world, m.x, m.y).is_none();
            let on_gadget = (self.s.tool == Tool::Gadget).then(|| self.gadget_at(m)).flatten();
            if on_zone && self.remove_zone_at(m) {
            } else if let Some(i) = on_gadget {
                self.remove_gadget(i);
            } else if let Some((i, _)) = near_link {
                self.record("Remove link");
                let l = self.links.remove(i);
                l.remove(&mut self.world);
                self.toasts.status("link", format!("{} removed", l.kind.label()));
            } else if let Some(i) = object_at(&self.objects, &self.world, m.x, m.y) {
                let o = &self.objects[i];
                let linked = self.links.iter().any(|l| l.involves(o.body));
                let flags = (o.pinned, linked, o.material.planet > 0.0);
                self.menu.open(input.mouse, o.body, o.name(), flags);
            }
        }
    }

    fn handle_drops(&mut self, mouse: Vec2) {
        let files = get_dropped_files();
        let n = files.len();
        if n > 0 {
            self.record("Drop files");
        }
        // Music dropped on the playlist window is added to it.
        let on_playlist = self.s.player && {
            let at = self.player_local(mouse).unwrap_or(mouse_position().into());
            let np = self.audio.now_playing();
            let view = player::view(&self.s, self.skin.as_ref(), &self.audio, np.as_ref());
            self.player.playlist_rect(&view).is_some_and(|r| r.contains(at))
        };
        for (i, f) in files.into_iter().enumerate() {
            let at = mouse + vec2((i as f32 - n as f32 / 2.0) * 30.0, -(i as f32) * 10.0);
            if let (true, Some(p)) = (on_playlist, f.path.as_ref()) {
                let p = p.to_string_lossy().into_owned();
                if Audio::is_audio_file(&p) {
                    self.audio.add(&[p]);
                    continue;
                }
            }
            match (f.path, f.bytes) {
                (Some(p), _) => self.open_path(&p.to_string_lossy(), at),
                (None, Some(bytes)) => {
                    let src = Source::Memory { name: "dropped".into(), data: Arc::new(bytes) };
                    self.add_object(src, at, None);
                }
                _ => {}
            }
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Actions
    // ═══════════════════════════════════════════════════════════
    fn apply(&mut self, action: Action, mouse: Vec2) {
        match action {
            Action::SetGravity(g) => self.s.gravity = g,
            Action::SetTool(t) if self.challenge.is_some() && t != Tool::Draw => {
                self.toasts.status("tool", "Challenges are solved by drawing  ·  Esc to leave the challenge");
            }
            Action::SetTool(t) => {
                if self.s.tool != t {
                    self.s.tool = t;
                    self.grab = None;
                    self.field_active = false;
                    self.stroke = None;
                    self.link_drag = None;
                    self.select_drag = None;
                    self.gadget_drag = None;
                    self.spawner.fader.open = false;
                }
            }
            Action::CycleBorder(d) => {
                self.s.border = self.s.border.cycle(d);
                self.world.set_border(self.s.border);
                self.toasts
                    .status("border", format!("Borders: {} — {}", self.s.border.label(), self.s.border.description()));
            }
            Action::CycleBackground(d) => {
                let mut next = self.bg.mode.cycle(d);
                if next == BgMode::Custom && self.bg.texture.is_none() {
                    if !self.pick_background() {
                        next = next.cycle(d);
                    } else {
                        next = BgMode::Custom;
                    }
                }
                self.bg.mode = next;
                self.s.background = next;
                self.toasts.status("bg", format!("Background: {}", next.label()));
            }
            Action::PickBackground => {
                if self.pick_background() {
                    self.toasts.success("Background image loaded");
                }
            }
            Action::AddImages => {
                if let Some(paths) =
                    FileDialog::new().add_filter("Images", IMAGE_EXT).add_filter("All files", &["*"]).pick_files()
                {
                    self.record("Add images");
                    let n = paths.len();
                    for (i, p) in paths.into_iter().enumerate() {
                        let x =
                            screen_width() / 2.0 + (i as f32 - n as f32 / 2.0) * 50.0 + rand::gen_range(-40.0, 40.0);
                        let at = self.spawn_point(x);
                        self.add_object(Source::File(p.to_string_lossy().into_owned()), at, None);
                    }
                }
            }
            Action::ToggleSpawner => {
                self.spawner.fader.toggle();
                self.menu.close();
            }
            Action::OpenToolPicker => self.picker.fader.toggle(),
            Action::LoadAudio => {
                let all: Vec<&str> =
                    config::TRACKER_EXT.iter().chain(config::AUDIO_EXT).chain(config::PLAYLIST_EXT).copied().collect();
                if let Some(p) = FileDialog::new()
                    .add_filter("All audio", &all)
                    .add_filter("Audio files", config::AUDIO_EXT)
                    .add_filter("Playlists", config::PLAYLIST_EXT)
                    .add_filter("Tracker modules", config::TRACKER_EXT)
                    .add_filter("All files", &["*"])
                    .pick_file()
                {
                    self.load_audio(&p.to_string_lossy());
                }
            }
            Action::ToggleAudio => match self.audio.toggle_pause() {
                Some(playing) => self.toasts.status("audio", if playing { "Music resumed" } else { "Music paused" }),
                None => self.toasts.info("No music loaded — press M"),
            },
            Action::FetchButtons => self.start_job(FetchKind::Buttons),
            Action::FetchLogos => self.start_job(FetchKind::Logos),
            Action::ClearAll => {
                let n = self.objects.len();
                if n > 0 {
                    self.record("Clear all");
                }
                self.clear_objects();
                self.grains.clear(&mut self.world);
                if n > 0 {
                    self.toasts.info(format!("Cleared {n} objects"));
                }
            }
            Action::SaveScene => self.save_scene(),
            Action::LoadScene => {
                let mut dlg = FileDialog::new()
                    .add_filter("Scene or challenge", &[SCENE_EXT, config::CHALLENGE_EXT])
                    .add_filter("Gravity scene", &[SCENE_EXT])
                    .add_filter("Gravity challenge", &[config::CHALLENGE_EXT]);
                if let Some(dir) = config::scene_dir().filter(|d| d.is_dir()) {
                    dlg = dlg.set_directory(dir);
                }
                if let Some(p) = dlg.pick_file() {
                    self.open_path(&p.to_string_lossy(), vec2(0.0, 0.0));
                }
            }
            Action::Screenshot => self.screenshot = true,
            Action::TogglePause => {
                self.paused = !self.paused;
                self.menu.close();
            }
            Action::ToggleHelp => self.help.fader.toggle(),
            Action::ResetSettings => {
                let keep_bg = self.s.custom_background.clone();
                self.s = Settings { custom_background: keep_bg, show_title: self.s.show_title, ..Settings::default() };
                self.world.set_border(self.s.border);
                self.bg.mode = self.s.background;
                self.toasts.info("Settings reset to defaults");
            }
            Action::CycleVisualizer(d) => {
                self.s.vis_background = self.s.vis_background.cycle(d, true);
                let hint = if self.audio.now_playing().is_none() && self.s.vis_background != VisStyle::Off {
                    "  ·  load music with M"
                } else {
                    ""
                };
                self.toasts.status("vis", format!("Visualizer: {}{hint}", self.s.vis_background.label()));
            }
            Action::CycleVisualizerObject(d) => {
                self.s.vis_object = self.s.vis_object.cycle(d, false);
                self.toasts.status("vis", format!("Visualizer objects: {}", self.s.vis_object.label()));
            }
            Action::SpawnVisualizer => {
                // From the keyboard it appears under the cursor, from the drawer near the top.
                let at = if self.paused { self.spawn_point(screen_width() / 2.0) } else { mouse };
                self.record("Add visualizer");
                self.spawn_visualizer(at);
            }
            Action::ToggleWater => {
                self.s.water = !self.s.water;
                self.toasts.status(
                    "water",
                    if self.s.water { "Water on  ·  level and density in settings" } else { "Water off" },
                );
            }
            Action::CycleWeather(d) => {
                self.s.weather = self.s.weather.cycle(d);
                let hint = match self.s.weather {
                    Weather::Clear => "Clear sky",
                    Weather::Rain => "Rain  ·  it puts out fires and fills containers",
                    Weather::Snow => "Snow  ·  it settles, and fire melts it",
                    Weather::Storm => "Storm  ·  gusts of wind and lightning that starts fires",
                };
                self.toasts.status("weather", hint);
            }
            Action::ToggleNight => {
                self.s.night = !self.s.night;
                self.toasts.status(
                    "night",
                    if self.s.night { "Night  ·  lamps, fire and lasers light the scene" } else { "Day" },
                );
            }
            Action::Undo | Action::Redo if self.challenge.is_some() => {
                self.toasts.status("undo", "No undo in challenges  ·  press R to start over");
            }
            Action::Undo => self.undo(false),
            Action::Redo => self.undo(true),
            Action::ToggleRecording => self.toggle_recording(),
            Action::SpawnRagdoll if self.challenge.is_some() => {}
            Action::SpawnRagdoll => {
                // From the drawer the pointer is over the panel: drop it mid-screen instead.
                let at = if self.paused { self.view().to_world(vec2(screen_width() * 0.4, 160.0)) } else { mouse };
                self.spawn_ragdoll(at);
            }
            Action::SpawnJelly | Action::SpawnCloth if self.challenge.is_some() => {}
            Action::SpawnJelly | Action::SpawnCloth => {
                let at = if self.paused { self.view().to_world(vec2(screen_width() * 0.4, 160.0)) } else { mouse };
                if action == Action::SpawnJelly {
                    self.jelly_at(at);
                } else {
                    self.cloth_at(at);
                }
            }
            Action::TogglePlayer => {
                self.s.player = !self.s.player;
                if self.s.player {
                    self.ensure_skin();
                    let skin = self.skin.as_ref().map_or("built-in look".to_string(), |s| format!("skin “{}”", s.name));
                    self.toasts.status(
                        "player",
                        format!("Classic player ({skin})  ·  X hides it, drop a .wsz to change the skin"),
                    );
                }
            }
            Action::TogglePlayerPhysics => {
                self.s.player_physics = !self.s.player_physics;
                if self.s.player_physics {
                    self.s.player = true;
                    self.toasts
                        .status("player", "Physical player  ·  drag a title bar to throw it  ·  Shift+X puts it back");
                } else {
                    self.toasts.status("player", "Player back on the screen");
                }
            }
            Action::Player(cmd) => self.player_cmd(cmd),
            Action::CycleSkin(d) => self.cycle_skin(d),
            Action::LoadSkin => self.pick_skin(),
            Action::ClearGrains if self.grains.is_empty() => {
                self.toasts.status("grains", "No grains to remove  ·  hold the mouse with the Pour tool (K)");
            }
            Action::ClearGrains => {
                let n = self.grains.len();
                self.grains.clear(&mut self.world);
                self.toasts.status("grains", format!("Removed {n} grains"));
            }
            Action::CycleWorldSize(d) => {
                self.s.world_size = (self.s.world_size as i32 - 1 + d).rem_euclid(3) as u8 + 1;
                let (aw, ah) = self.arena();
                self.world.resize((aw, ah));
                self.last_size = (aw, ah);
                let screen = vec2(screen_width(), screen_height());
                self.camera.fit(vec2(aw, ah), screen);
                self.toasts.status("world", format!("World size ×{}  ·  zoom with Ctrl+wheel", self.s.world_size));
            }
            Action::ToggleEffects => {
                self.s.effects = !self.s.effects;
                if !self.s.effects {
                    self.effects.clear();
                }
            }
            Action::OpenLibrary => {
                let open = !self.library.fader.open;
                if open {
                    self.refresh_custom();
                    self.library.open(self.challenge.is_some());
                } else {
                    self.library.fader.open = false;
                }
            }
            Action::LoadExample(i) => self.open_example(i),
            Action::StartChallenge(i) => self.start_challenge(i),
            Action::StartCustom(i) => self.start_custom_index(i),
            Action::OpenEditor(i) => self.open_editor(i),
            Action::Editor(cmd) => self.editor_cmd(cmd),
            Action::ChallengeGo | Action::ChallengeRetry | Action::ChallengeNext | Action::ChallengeExit => {
                self.challenge_action(action)
            }
            Action::Object(cmd, handle) => self.object_cmd(cmd, handle, mouse),
            Action::Selection(_) if self.challenge.is_some() => {}
            Action::Selection(cmd) => self.selection_cmd(cmd, mouse),
        }
    }

    /// A soft glow around planets, and a slow ripple of their pull.
    fn draw_planet_halos(&self) {
        let t = get_time() as f32;
        for o in self.objects.iter().filter(|o| o.material.planet > 0.0) {
            let (p, _) = o.screen_pos(&self.world);
            let r = o.size.x.min(o.size.y) / 2.0;
            let k = (o.material.planet / 30.0).clamp(0.25, 1.0);
            for i in 1..=4 {
                draw_circle(p.x, p.y, r * (1.0 + i as f32 * 0.3), Color::new(0.45, 0.6, 1.0, 0.035 * k));
            }
            let u = (t * 0.4 + o.size.x * 0.01) % 1.0;
            draw_circle_lines(p.x, p.y, r * (1.2 + u * 1.8), 1.0, Color::new(0.6, 0.75, 1.0, 0.3 * (1.0 - u) * k));
        }
    }

    fn spawn_visualizer(&mut self, at: Vec2) {
        let placement = Placement { pos_px: (at.x, at.y), size_px: Some((240.0, 130.0)), ..Default::default() };
        if let Some(o) = Object::load(&mut self.world, Source::Visualizer, placement) {
            self.objects.push(o);
            if self.audio.now_playing().is_none() {
                self.toasts.status("vis", "Visualizer added  ·  load music with M to see it move");
            }
        }
    }

    fn object_cmd(&mut self, cmd: ObjectCmd, handle: RigidBodyHandle, _mouse: Vec2) {
        let Some(i) = self.objects.iter().position(|o| o.body == handle) else { return };
        let label = match cmd {
            ObjectCmd::Delete => Some("Delete"),
            ObjectCmd::Duplicate => Some("Duplicate"),
            ObjectCmd::Resize(_) => Some("Resize"),
            ObjectCmd::SizeAll(_) => Some("Resize all"),
            ObjectCmd::TogglePin => Some("Pin"),
            ObjectCmd::Unlink => Some("Detach links"),
            ObjectCmd::Jelly => Some("Make jelly"),
            ObjectCmd::Flag => Some("Make flag"),
            ObjectCmd::Planet => Some("Planet"),
            ObjectCmd::Properties => None,
        };
        if let Some(label) = label {
            self.record(label);
        }
        match cmd {
            ObjectCmd::Delete => self.remove_object(i),
            ObjectCmd::Jelly => self.make_jelly(i),
            ObjectCmd::Flag => self.make_flag(i),
            ObjectCmd::Planet => {
                let o = &mut self.objects[i];
                if o.material.planet > 0.0 {
                    let m = Material { planet: 0.0, gravity: 1.0, ..o.material };
                    o.set_material(&mut self.world, m);
                    self.toasts.info("No longer a planet");
                } else {
                    // Pinned, it stays put for things to orbit (unpin it to let it move).
                    let m = Material { planet: Material::PLANET.planet, gravity: 0.0, ..o.material };
                    o.set_material(&mut self.world, m);
                    if !o.pinned {
                        o.set_pinned(&mut self.world, true);
                    }
                    self.toasts.info("Planet  ·  things fall toward it  ·  its pull is in Properties (I)");
                }
            }
            ObjectCmd::Properties => {
                let o = &self.objects[i];
                let (p, _) = o.screen_pos(&self.world);
                self.inspector.open(o.body, o.name(), self.view().to_screen(p));
                self.inspector_recorded = false;
                self.menu.close();
            }
            ObjectCmd::Unlink => {
                let (gone, keep): (Vec<Link>, Vec<Link>) =
                    std::mem::take(&mut self.links).into_iter().partition(|l| l.involves(handle));
                for l in &gone {
                    l.remove(&mut self.world);
                }
                self.links = keep;
                self.toasts.info(format!("Detached {} link{}", gone.len(), if gone.len() == 1 { "" } else { "s" }));
            }
            ObjectCmd::Duplicate => {
                let mut at = self.objects[i].placement(&self.world);
                let off = rand::gen_range(24.0, 48.0);
                at.pos_px = (at.pos_px.0 + off, at.pos_px.1 - off);
                at.linvel = (0.0, 0.0);
                at.angvel = 0.0;
                let dup = self.objects[i].duplicate(&mut self.world, at);
                self.objects.push(dup);
            }
            ObjectCmd::Resize(k) => self.objects[i].resize(&mut self.world, k),
            ObjectCmd::SizeAll(k) => {
                for o in &mut self.objects {
                    o.resize(&mut self.world, k);
                }
            }
            ObjectCmd::TogglePin => {
                let pinned = !self.objects[i].pinned;
                self.objects[i].set_pinned(&mut self.world, pinned);
                self.toasts.info(if pinned { "Pinned — drag it with Spring to move it" } else { "Unpinned" });
            }
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Objects
    // ═══════════════════════════════════════════════════════════
    /// World point near the top of the view, below the HUD, at screen x `sx`.
    fn spawn_point(&self, sx: f32) -> Vec2 {
        let sy = self.hud.bottom().max(20.0) + 60.0 + rand::gen_range(0.0, 50.0);
        self.view().to_world(vec2(sx, sy))
    }

    /// World size in pixels.
    fn arena(&self) -> (f32, f32) {
        let k = self.s.world_size as f32;
        (screen_width() * k, screen_height() * k)
    }

    fn view(&self) -> View {
        self.camera.view(vec2(screen_width(), screen_height()))
    }

    fn zoom_toast(&mut self) {
        let msg = format!("Zoom {:.0}%  ·  Ctrl+wheel, middle-drag to move, Home to fit", self.camera.zoom * 100.0);
        self.toasts.status("zoom", msg);
    }

    fn add_object(&mut self, source: Source, at: Vec2, vel: Option<(f32, f32)>) -> bool {
        let name = source.display_name();
        let placement = Placement {
            pos_px: (at.x, at.y),
            linvel: vel.unwrap_or((rand::gen_range(-2.0, 2.0), 0.0)),
            ..Default::default()
        };
        match Object::load(&mut self.world, source, placement) {
            Some(o) => {
                self.objects.push(o);
                true
            }
            None => {
                self.toasts.error(format!("Could not load {}", crate::util::ellipsize(&name, 40)));
                false
            }
        }
    }

    fn spawn_shape_at(&mut self, at: Vec2) {
        let shape = self.s.spawn_shape;
        let c = spawn_color(
            self.s.spawn_color,
            if self.s.spawn_color >= shapes::PALETTE.len() { rand::gen_range(0.0, 400.0) } else { 0.0 },
        );
        let rgb = ((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
        self.spawn_shape(shape, rgb, self.s.spawn_size, at, false, Material::DEFAULT);
        self.sound(crate::audio::sfx::Sound::Pop, at, 0.3);
    }

    /// Add a spawner shape of `size` px centred on `at` (world px).
    fn spawn_shape(&mut self, shape: Shape, rgb: (u8, u8, u8), size: f32, at: Vec2, pinned: bool, material: Material) {
        let size = size.round() as u32;
        let source = Source::Shape { shape, rgb };
        let visual = self
            .spawn_cache
            .entry((shape, size, rgb))
            .or_insert_with(|| {
                let img = shapes::rasterize(shape, size, rgb);
                let dec = crate::assets::Decoded { frames: vec![img], delays_ms: vec![], hull: None };
                Visual::upload(&source, &dec)
            })
            .clone();
        if self.spawn_cache.len() > 256 {
            self.spawn_cache.clear();
        }
        let placement = Placement { pos_px: (at.x, at.y), pinned, material, ..Default::default() };
        let o = Object::spawn(&mut self.world, source, visual, placement);
        self.objects.push(o);
    }

    fn remove_object(&mut self, i: usize) {
        let o = self.objects.remove(i);
        if self.grab.as_ref().is_some_and(|g| g.body == o.body) {
            self.grab = None;
        }
        if self.menu.target == Some(o.body) {
            self.menu.close();
        }
        if self.inspector.target == Some(o.body) {
            self.inspector.close();
        }
        if self.link_drag.as_ref().is_some_and(|d| d.from.is_some_and(|(b, _)| b == o.body)) {
            self.link_drag = None;
        }
        o.destroy(&mut self.world);
        links::prune(&mut self.links, &self.world);
    }

    /// Every body the tools act on: objects, grains, jelly and cloth.
    fn dynamic_bodies(&self) -> Vec<RigidBodyHandle> {
        let player = self.player_body.as_ref().map(|p| p.body);
        self.rigid_bodies().into_iter().chain(self.soft_bodies()).chain(player).collect()
    }

    /// Objects and grains: the bodies that wrap around the borders and go
    /// through portals (a jelly or cloth ball on its own cannot).
    fn rigid_bodies(&self) -> Vec<RigidBodyHandle> {
        self.objects.iter().map(|o| o.body).chain(self.grains.bodies()).collect()
    }

    fn clear_objects(&mut self) {
        self.grab = None;
        self.link_drag = None;
        self.menu.close();
        self.inspector.close();
        self.zone_drag = None;
        self.rewind.clear();
        self.fire.clear();
        self.selection.clear();
        self.select_drag = None;
        self.zones.clear();
        self.effects.clear();
        for l in self.links.drain(..) {
            l.remove(&mut self.world);
        }
        for o in self.objects.drain(..) {
            o.destroy(&mut self.world);
        }
        for s in self.softs.drain(..) {
            s.remove(&mut self.world);
        }
        self.gadgets.clear();
        self.gadget_drag = None;
        self.beams.clear();
    }

    // ═══════════════════════════════════════════════════════════
    // Undo / redo
    // ═══════════════════════════════════════════════════════════
    /// Remember the scene before an edit, so it can be undone.
    fn record(&mut self, label: &str) {
        self.editor_touched();
        let snap = self.snapshot();
        self.history.record(label, snap);
    }

    /// The whole scene, jelly and cloth included.
    fn snapshot(&self) -> Snapshot {
        let mut snap = Snapshot::capture(&self.world, &self.objects, &self.links, &self.zones);
        snap.softs = self.softs.iter().map(|s| s.state(&self.world)).collect();
        snap.gadgets = scene::capture_gadgets(&self.objects, &self.gadgets);
        snap
    }

    fn undo(&mut self, redo: bool) {
        let current = self.snapshot();
        let step = if redo { self.history.redo(current) } else { self.history.undo(current) };
        let Some((label, snap)) = step else {
            self.toasts.status("undo", if redo { "Nothing to redo" } else { "Nothing to undo" });
            return;
        };
        self.clear_objects();
        self.stroke = None;
        let restored = snap.restore(&mut self.world);
        self.objects = restored.objects;
        self.links = restored.links;
        self.zones = restored.zones;
        self.softs =
            snap.softs.iter().map(|s| s.restore(&mut self.world, rapier2d::prelude::Vector::zeros())).collect();
        let bodies: Vec<Option<RigidBodyHandle>> = self.objects.iter().map(|o| Some(o.body)).collect();
        self.gadgets = scene::restore_gadgets(&snap.gadgets, &bodies);
        self.toasts.status("undo", format!("{}: {label}", if redo { "Redo" } else { "Undo" }));
    }

    // ═══════════════════════════════════════════════════════════
    // GIF recording
    // ═══════════════════════════════════════════════════════════
    fn toggle_recording(&mut self) {
        if let Some(rec) = self.recording.take() {
            self.finishing = Some(rec.stop());
            self.toasts.status("gif", "Saving GIF…");
            return;
        }
        if self.finishing.is_some() {
            self.toasts.info("Still saving the previous GIF");
            return;
        }
        let Some(dir) = config::screenshot_dir() else {
            self.toasts.error("GIF: no home directory");
            return;
        };
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.toasts.error(format!("GIF: {e}"));
            return;
        }
        let path = dir.join(format!("gravity-{}.gif", stamp()));
        self.recording = Some(Recording::start(path, get_time()));
        self.toasts.status("gif", format!("Recording GIF  ·  F11 to stop (max {} s)", recorder::MAX_SECS as u32));
    }

    fn poll_recording(&mut self) {
        let Some(result) = self.finishing.as_mut().and_then(Finishing::poll) else { return };
        self.finishing = None;
        match result {
            Ok((path, frames)) => self.toasts.success(format!("GIF saved: {} ({frames} frames)", tilde(&path))),
            Err(e) => self.toasts.error(format!("GIF failed: {e}")),
        }
    }

    /// Grab a frame for the GIF (called after the scene is drawn, before the UI).
    fn capture_frame(&mut self) {
        let now = get_time();
        let Some(rec) = &mut self.recording else { return };
        if rec.due(now) {
            let img = get_screen_data();
            rec.push(img.bytes, img.width as u32, img.height as u32, now);
        }
        if rec.elapsed(now) >= recorder::MAX_SECS {
            self.toggle_recording();
        }
    }

    /// Open a path of any supported kind (image, audio, scene).
    fn open_path(&mut self, path: &str, at: Vec2) {
        let ext = ext_of(path);
        if ext == SCENE_EXT {
            self.load_scene(std::path::Path::new(path));
        } else if ext == "wsz" {
            self.use_skin(std::path::Path::new(path));
        } else if ext == config::CHALLENGE_EXT {
            self.open_challenge_file(std::path::Path::new(path));
        } else if Audio::is_audio_file(path) {
            self.load_audio(path);
        } else {
            self.add_object(Source::File(path.to_string()), at, None);
        }
    }

    fn load_audio(&mut self, path: &str) {
        match self.audio.load(path) {
            Ok(name) => self.toasts.success(format!("Playing {}", crate::util::ellipsize(&name, 40))),
            Err(e) => self.toasts.error(format!("Audio: {e}")),
        }
    }

    fn pick_background(&mut self) -> bool {
        let Some(p) = FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp", "gif", "tga", "qoi"])
            .pick_file()
        else {
            return false;
        };
        let p = p.to_string_lossy().into_owned();
        match self.bg.load_custom(&p) {
            Ok(()) => {
                self.s.custom_background = Some(p);
                self.s.background = BgMode::Custom;
                true
            }
            Err(e) => {
                self.toasts.error(format!("Background: {e}"));
                false
            }
        }
    }

    fn start_job(&mut self, kind: FetchKind) {
        if self.jobs.iter().any(|j| j.kind == kind) {
            self.toasts.info(format!("Already fetching {}", kind.label()));
            return;
        }
        self.record(match kind {
            FetchKind::Buttons => "Fetch buttons",
            FetchKind::Logos => "Fetch logos",
        });
        self.jobs.push(match kind {
            FetchKind::Buttons => FetchJob::buttons(20),
            FetchKind::Logos => FetchJob::logos(20),
        });
    }

    fn poll_jobs(&mut self) {
        let mut jobs = std::mem::take(&mut self.jobs);
        for job in &mut jobs {
            for ev in job.poll() {
                match ev {
                    FetchEvent::Item { name, data } => {
                        let x = rand::gen_range(60.0, (screen_width() - 60.0).max(61.0));
                        let src = Source::Memory { name, data: Arc::new(data) };
                        let at = self.spawn_point(x) - vec2(0.0, 40.0);
                        self.add_object(src, at, Some((rand::gen_range(-1.0, 1.0), 0.0)));
                    }
                    FetchEvent::Failed(msg) => self.toasts.error(msg),
                }
            }
            if job.done {
                if job.received > 0 {
                    self.toasts.success(format!("Fetched {} {}", job.received, job.kind.label()));
                } else if !job.failed {
                    self.toasts.warn(format!("No {} could be downloaded", job.kind.label()));
                }
            }
        }
        jobs.retain(|j| !j.done);
        self.jobs = jobs;
    }

    fn save_scene(&mut self) {
        let mut dlg =
            FileDialog::new().add_filter("Gravity scene", &[SCENE_EXT]).set_file_name(format!("scene.{SCENE_EXT}"));
        if let Some(dir) = config::scene_dir() {
            if std::fs::create_dir_all(&dir).is_ok() {
                dlg = dlg.set_directory(dir);
            }
        }
        let Some(mut path) = dlg.save_file() else { return };
        if path.extension().is_none() {
            path.set_extension(SCENE_EXT);
        }
        let water =
            self.s.water.then_some(scene::SceneWater { level: self.s.water_level, density: self.s.water_density });
        let mut scene = scene::capture(&self.world, &self.objects, &self.links, &self.zones, water);
        scene.gadgets = scene::capture_gadgets(&self.objects, &self.gadgets);
        scene.night = Some(self.s.night);
        scene.weather = Some(self.s.weather);
        match scene::write(&path, &scene) {
            Ok(()) => self.toasts.success(format!(
                "Saved {} objects to {}",
                scene.objects.len(),
                file_name_of(&path.to_string_lossy())
            )),
            Err(e) => self.toasts.error(format!("Save failed: {e}")),
        }
    }

    fn load_scene(&mut self, path: &std::path::Path) {
        match scene::read(path) {
            Ok(sc) => {
                self.record("Open scene");
                self.challenge = None;
                let (n, failed) = self.apply_scene(sc);
                if failed > 0 {
                    self.toasts.warn(format!("Loaded {n} objects ({failed} missing)"));
                } else {
                    self.toasts.success(format!("Loaded {n} objects"));
                }
            }
            Err(e) => self.toasts.error(format!("Open failed: {e}")),
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Simulation
    // ═══════════════════════════════════════════════════════════
    fn sync_settings(&mut self) {
        if (self.world.gravity.y - self.s.gravity).abs() > 1e-4 {
            self.world.set_gravity(self.s.gravity);
        }
        if self.world.border != self.s.border {
            self.world.set_border(self.s.border);
        }
        self.shaker.set_enabled(self.s.window_shake);
        self.audio.set_volume(self.s.volume);
        self.audio.set_eq(self.s.eq_on, self.s.eq_preamp, &self.s.eq_bands);
        self.audio.shuffle = self.s.shuffle;
        self.audio.repeat = self.s.repeat;
    }

    fn simulate(&mut self, dt: f32, mouse: Vec2) {
        let running = !self.paused || self.step_once;
        let time_scale = self.s.time_scale * self.slow_factor();
        let step = if self.step_once { crate::config::PHYSICS_DT } else { dt * time_scale };
        if running {
            let pos = (mouse.x, mouse.y);
            self.world.reset_forces();
            if self.pouring && self.s.tool == Tool::Pour {
                let n = self.grains.pour(&mut self.world, self.s.grain_kind, mouse, POUR_RATE, dt);
                self.pour_sound -= dt;
                if n > 0 && self.pour_sound <= 0.0 {
                    self.pour_sound = 0.15;
                    self.sound(crate::audio::sfx::Sound::Pour, mouse, 0.5);
                }
            }
            if let Some(g) = &self.grab {
                g.apply(&mut self.world.bodies, to_phys(mouse.x, mouse.y));
            }
            let handles = self.dynamic_bodies();
            if self.field_active {
                tools::apply_field(
                    &mut self.world.bodies,
                    &handles,
                    self.s.tool,
                    pos,
                    self.s.tool_radius,
                    self.s.tool_strength,
                );
            }
            self.world.border.apply_forces(&mut self.world, &handles);
            links::apply_springs(&self.links, &mut self.world);
            self.soft_forces();
            self.update_gadgets(step, true, mouse);
            magnets::apply(&mut self.world, &self.objects);
            let wells = planets::wells(&self.world, &self.objects);
            if !wells.is_empty() {
                let bodies = self.dynamic_bodies();
                planets::apply(&mut self.world, &wells, &bodies);
            }
            let rigid = self.rigid_bodies();
            let mut zoned = rigid.clone();
            zoned.extend(self.player_body.as_ref().map(|p| p.body));
            let teleports = zones::apply(&self.zones, &mut self.portals, &mut self.world, &zoned);
            if self.s.effects {
                for t in teleports.into_iter().take(6) {
                    self.effects.splash(crate::physics::to_screen(t.from.x, t.from.y), 0.6);
                    self.effects.splash(crate::physics::to_screen(t.to.x, t.to.y), 0.6);
                }
            }
            if self.s.water {
                let rest = water::rest_level(&self.world, self.s.water_level);
                let splashes = self.water.apply(&mut self.world, &self.objects, rest, self.s.water_density, step);
                self.water.step(step, self.audio.analyzer.bass, self.audio.analyzer.beat_now);
                if self.s.effects {
                    for (x, y, strength) in splashes.into_iter().filter(|s| s.2 > 0.25) {
                        self.effects.splash(crate::physics::to_screen(x, y), strength);
                    }
                }
            }
            if self.s.vis_dance && self.audio.analyzer.beat_now {
                self.dance();
            }
            if self.s.window_shake {
                self.shaker.apply(&mut self.world.bodies, self.s.shake_force);
            }
            if self.step_once {
                self.world.step_fixed();
                self.step_once = false;
            } else {
                self.world.step_frame(dt, time_scale);
            }
            self.rewind_record(step);

            let dead = self.world.border.apply_positions(&mut self.world, &rigid);
            let mut dead_grains = HashSet::new();
            for h in dead {
                if let Some(i) = self.objects.iter().position(|o| o.body == h) {
                    self.remove_object(i);
                } else {
                    dead_grains.insert(h);
                }
            }
            if !dead_grains.is_empty() {
                self.grains.remove(&mut self.world, &dead_grains);
            }
            self.process_impacts();
            self.update_fire(step, mouse);
            self.soft_after_step(step, mouse);
            links::prune(&mut self.links, &self.world);

            let (len, fade) = (self.s.trail_length as usize, self.s.trail_fade);
            for o in &mut self.objects {
                if self.s.trails {
                    o.update_trail(&self.world, len, fade);
                } else {
                    o.clear_trail();
                }
            }
        } else {
            // Paused: beams still follow what is moved around.
            self.update_gadgets(0.0, false, mouse);
        }
        let dt_ms = dt * 1000.0 * if self.paused { 0.0 } else { 1.0 };
        for o in &mut self.objects {
            o.update_anim(dt_ms);
        }
        self.update_weather(dt, running, step, mouse);
        self.blasts.retain(Blast::alive);
        let floor = if self.world.border.walls().floor { self.arena().1 - WALL_T * PPM } else { f32::MAX };
        let gravity = -self.world.gravity.y * PPM;
        self.effects.update(if running { step } else { 0.0 }, gravity, floor);
    }

    /// Make resting objects hop on a beat, harder with more bass.
    fn dance(&mut self) {
        let kick = 2.0 + self.audio.analyzer.bass * 3.5;
        let player = self.player_body.as_ref().map(|p| p.body);
        for h in self.objects.iter().map(|o| o.body).chain(player) {
            let Some(b) = self.world.bodies.get_mut(h) else { continue };
            if !b.is_dynamic() || b.linvel().y.abs() > 1.5 {
                continue;
            }
            let m = b.mass();
            b.apply_impulse(rapier2d::na::Vector2::new(rand::gen_range(-0.4, 0.4) * m, kick * m), true);
            b.apply_torque_impulse(rand::gen_range(-0.3, 0.3) * m, true);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Rendering
    // ═══════════════════════════════════════════════════════════
    fn draw(&mut self, dt: f32, input: &Input) {
        if self.player_body.is_some() {
            self.render_player_body(input.world);
        }
        let (sw, sh) = (screen_width(), screen_height());
        let screen = vec2(sw, sh);
        let (aw, ah) = self.arena();
        let m = input.world;
        self.bg.draw(sw, sh);
        let view = self.view();
        let floor = if self.world.border.walls().floor {
            (sh - view.to_screen(vec2(0.0, ah - WALL_T * PPM)).y).max(0.0)
        } else {
            0.0
        };
        visualizer::draw_background(self.s.vis_background, &self.audio.analyzer, sw, sh, floor);

        // The scene, in world pixels.
        self.camera.begin_world(screen);
        self.world.border.draw(aw, ah);
        zones::draw(&self.zones);

        if self.s.trails {
            for o in self.objects.iter().filter(|o| !o.is_visualizer()) {
                o.draw_trail(self.s.trail_fade);
            }
        }
        for s in &self.softs {
            s.draw(&self.world);
        }
        self.draw_planet_halos();
        for o in &self.objects {
            if o.is_visualizer() {
                let (p, angle) = o.screen_pos(&self.world);
                visualizer::draw_object(self.s.vis_object, &self.audio.analyzer, p, angle, o.size, 1.0);
            } else {
                let tint = self.fire.tint(o.body);
                // Mirrors get a cool sheen.
                let tint = if o.material.mirror { Color::new(tint.r * 0.8, tint.g * 0.95, tint.b, 1.0) } else { tint };
                o.draw_tinted(&self.world, tint);
            }
        }
        self.draw_player_body();
        self.draw_gadgets();
        self.grains.draw(&self.world);
        for l in &self.links {
            l.draw(&self.world);
        }
        let t = get_time() as f32;
        for o in &self.objects {
            let (p, _) = o.screen_pos(&self.world);
            let mut badges = vec![];
            if o.pinned {
                badges.push(0);
            }
            if o.material.magnet != 0.0 {
                badges.push(1);
            }
            if o.material.conveyor != 0.0 {
                badges.push(2);
            }
            if o.material.planet > 0.0 {
                badges.push(3);
            }
            let n = badges.len() as f32;
            for (k, badge) in badges.into_iter().enumerate() {
                let c = p + vec2((k as f32 - (n - 1.0) / 2.0) * 20.0, 0.0);
                draw_circle(c.x, c.y, 9.0, theme::alpha(BLACK, 0.35));
                match badge {
                    0 => icons::pin(c, 14.0, theme::WARNING),
                    1 => icons::magnet(c + vec2(0.0, 2.0), 14.0, 1.0, o.material.magnet < 0.0),
                    3 => icons::planet(c, 14.0, theme::alpha(Color::new(0.55, 0.7, 1.0, 1.0), 0.9)),
                    _ => icons::conveyor(c, 14.0, o.material.conveyor, t, 1.0),
                }
            }
        }
        if self.s.water {
            let rest = water::rest_level(&self.world, self.s.water_level);
            self.water.draw(&self.world, rest, aw, ah);
        }
        self.effects.draw();
        self.sky.draw();
        for target in [self.menu.target, self.inspector.target].into_iter().flatten() {
            if let Some(o) = self.objects.iter().find(|o| o.body == target) {
                let c = o.corners(&self.world);
                for i in 0..4 {
                    let (a, b) = (c[i], c[(i + 1) % 4]);
                    draw_line(a.x, a.y, b.x, b.y, 1.5, theme::alpha(theme::ACCENT_HI, 0.9));
                }
            }
        }

        camera::end_world();
        self.draw_night(m);

        if self.screenshot {
            self.screenshot = false;
            self.take_screenshot();
        }
        self.capture_frame();

        // Tool feedback, still in world pixels.
        self.camera.begin_world(screen);
        if let Some(g) = &self.grab {
            cursor::draw_grab(g, &self.world, m);
        }
        for b in &self.blasts {
            b.draw();
        }
        let zone_drag = self.zone_drag.map(|p| (p, self.s.zone_kind));
        let goal_drag = self.editor_goal_drag().map(|p| (p, zones::ZoneKind::Goal));
        if let Some((start, kind)) = zone_drag.or(goal_drag) {
            let zone = Zone::from_screen(kind, start, m, 0.0, 0.0);
            let r = zone.rect();
            let c = kind.accent();
            draw_rectangle(r.x, r.y, r.w, r.h, theme::alpha(c, 0.12));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, theme::alpha(c, 0.9));
        }
        if let Some(st) = &self.stroke {
            let c = Color::from_rgba(st.rgb[0], st.rgb[1], st.rgb[2], 255);
            cursor::draw_stroke(&st.points, self.s.draw_thickness, c);
        }
        if let Some(d) = &self.link_drag {
            let from = d.from.and_then(|(b, local)| self.world.bodies.get(b).map(|body| body.position() * local));
            let start = from.map_or(d.start_px, |p| crate::physics::to_screen(p.x, p.y));
            let target = objects_at(&self.objects, &self.world, m.x, m.y)
                .into_iter()
                .find(|&i| d.from.is_none_or(|(b, _)| b != self.objects[i].body))
                .map(|i| self.objects[i].corners(&self.world));
            cursor::draw_link_drag(start, m, self.s.link_kind, target);
        }
        if !input.over_ui && self.title.is_none() {
            if self.spawner.is_open() {
                self.spawner.draw_ghost(&self.s, m);
            } else if self.grab.is_none() && self.stroke.is_none() {
                match self.s.tool {
                    Tool::Draw => {
                        let c = spawn_color(self.s.spawn_color, get_time() as f32);
                        cursor::draw_pen_cursor(m, self.s.draw_thickness, c);
                    }
                    Tool::Link if self.link_drag.is_none() => cursor::draw_link_cursor(m, self.s.link_kind),
                    Tool::Fire => self.draw_fire_cursor(m),
                    Tool::Gadget => self.draw_gadget_cursor(m),
                    Tool::Zone if self.zone_drag.is_none() => {
                        draw_line(m.x - 8.0, m.y, m.x + 8.0, m.y, 1.2, self.s.zone_kind.accent());
                        draw_line(m.x, m.y - 8.0, m.x, m.y + 8.0, 1.2, self.s.zone_kind.accent());
                        icons::zone_kind(self.s.zone_kind, m + vec2(20.0, -16.0), 22.0, self.s.zone_kind.accent(), t);
                    }
                    _ => cursor::draw_tool_cursor(self.s.tool, m, self.s.tool_radius, self.field_active),
                }
            }
        }

        if self.debug {
            debug::draw_world(&self.world, &self.objects);
        }
        self.draw_selection(input.world);
        self.draw_knife();
        camera::end_world();
        self.draw_slow_vignette();
        self.sky.draw_flash();

        // Interface, in screen pixels.
        let (m, world_mouse) = (input.mouse, m);
        let top = self.hud.bottom();
        if self.debug {
            let stats =
                debug::DebugStats { paused: self.paused, time_scale: self.s.time_scale, zoom: self.camera.zoom };
            debug::draw_panel(&self.world, &self.objects, &stats, world_mouse, top);
        }

        if self.challenge.is_none() {
            self.card.draw(&self.s, m);
        }
        let np = self.audio.now_playing();
        self.now_playing.draw(np.as_ref(), m);
        if self.s.player && self.player_body.is_none() {
            let view = player::view(&self.s, self.skin.as_ref(), &self.audio, np.as_ref());
            self.player.draw(&view, m);
        }
        self.spawner.draw(&self.s, m);
        self.drawer.draw(&self.s, top, m, np.is_some());
        self.hud.draw(&self.hud_state(), m);
        self.toasts.draw(top, &self.jobs, dt);
        self.inspector.draw(self.inspected().as_ref(), m);
        self.menu.draw(m);
        self.picker.draw(self.s.tool, m);
        if let Some(v) = self.challenge_view() {
            self.challenge_bar.draw(&v, top, m);
        }
        if let (Some(v), Some(ed)) = (self.editor_view(), self.editor.as_ref()) {
            self.editor_bar.draw(&v, &ed.name, top, m);
        }
        if self.library.fader.visible() {
            let mine: Vec<(String, String, f32)> = self
                .custom
                .iter()
                .map(|(p, c)| {
                    let stem = p.file_stem().map_or_else(|| c.name.clone(), |s| s.to_string_lossy().into_owned());
                    (c.name.clone(), format!("custom:{stem}"), c.ink)
                })
                .collect();
            let data = LibraryData { done: &self.s.challenges_done, stars: &self.s.challenge_stars, mine: &mine };
            self.library.draw(&data, m);
        }
        self.help.draw();
        self.draw_rec_indicator(top);
        self.draw_rewind(top);
    }

    fn draw_rec_indicator(&self, top: f32) {
        let now = get_time();
        let label = match (&self.recording, &self.finishing) {
            (Some(r), _) => {
                let secs = r.elapsed(now) as u32;
                format!("REC  {}:{:02}", secs / 60, secs % 60)
            }
            (None, Some(_)) => "Saving GIF…".to_string(),
            _ => return,
        };
        let w = theme::measure_bold(&label, 13.0) + 40.0;
        let r = Rect::new((screen_width() - w) / 2.0, top.max(10.0) + 6.0, w, 28.0);
        theme::rrect(r, 14.0, theme::alpha(Color::new(0.12, 0.03, 0.05, 1.0), 0.88));
        theme::rrect_lines(r, 14.0, 1.0, theme::alpha(theme::DANGER, 0.6));
        let blink = if self.recording.is_some() { 0.55 + 0.45 * (now * 4.0).sin().abs() as f32 } else { 0.4 };
        draw_circle(r.x + 16.0, r.y + 14.0, 5.0, theme::alpha(theme::DANGER, blink));
        theme::text_bold(&label, r.x + 28.0, theme::baseline(r.y + 14.0, 13.0), 13.0, theme::TEXT);
    }

    fn take_screenshot(&mut self) {
        let img = get_screen_data();
        let (w, h) = (img.width as usize, img.height as usize);
        // The framebuffer is bottom-up.
        let mut flipped = Vec::with_capacity(img.bytes.len());
        for row in (0..h).rev() {
            flipped.extend_from_slice(&img.bytes[row * w * 4..(row + 1) * w * 4]);
        }
        for px in flipped.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
        let result = (|| -> Result<std::path::PathBuf, String> {
            let dir = config::screenshot_dir().ok_or("no home directory")?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join(format!("gravity-{}.png", stamp()));
            image::save_buffer(&path, &flipped, w as u32, h as u32, image::ExtendedColorType::Rgba8)
                .map_err(|e| e.to_string())?;
            Ok(path)
        })();
        match result {
            Ok(p) => self.toasts.success(format!("Screenshot saved: {}", tilde(&p))),
            Err(e) => self.toasts.error(format!("Screenshot failed: {e}")),
        }
    }
}

/// Drop broken portal pairs from zones read from a file.
fn sanitize_zones(mut zones: Vec<Zone>) -> Vec<Zone> {
    let n = zones.len();
    let pairs: Vec<Option<usize>> = zones.iter().map(|z| z.pair).collect();
    for (i, z) in zones.iter_mut().enumerate() {
        z.pair = z.pair.filter(|&j| j < n && j != i && pairs[j] == Some(i));
        z.strength = if z.strength.is_finite() { z.strength.clamp(0.0, 100.0) } else { 0.0 };
    }
    zones.retain(|z| z.min.iter().chain(&z.max).all(|v| v.is_finite()));
    zones
}

/// Milliseconds since the epoch, for unique file names.
fn stamp() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or_default()
}

/// Display a path with the home directory abbreviated to `~`.
fn tilde(p: &std::path::Path) -> String {
    match std::env::var_os("HOME").map(std::path::PathBuf::from) {
        Some(h) if p.starts_with(&h) => format!("~/{}", p.strip_prefix(&h).unwrap_or(p).display()),
        _ => p.display().to_string(),
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.save_settings();
        // Finish a GIF that is still being recorded or written.
        let mut finishing = self.recording.take().map(Recording::stop).or(self.finishing.take());
        if let Some(f) = &mut finishing {
            while f.poll().is_none() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}
