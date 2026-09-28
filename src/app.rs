//! Application state and the per-frame loop: input → UI → actions →
//! simulation → rendering.

use crate::audio::Audio;
use crate::background::{Background, BgMode};
use crate::config::{self, ext_of, file_name_of, IMAGE_EXT, SCENE_EXT};
use crate::config::{DENSITY, PPM, WALL_T};
use crate::drawing;
use crate::history::{History, Snapshot};
use crate::net::{FetchEvent, FetchJob, FetchKind};
use crate::physics::links::{self, Link, LinkKind};
use crate::physics::object::{object_at, objects_at, Object, Placement, Source, Visual};
use crate::physics::tools::{self, Grab, Tool};
use crate::physics::water::{self, Water};
use crate::physics::{to_phys, PhysWorld};
use crate::recorder::{self, Finishing, Recording};
use crate::scene;
use crate::settings::{Settings, DRAW_THICKNESS_RANGE, RADIUS_RANGE, SPAWN_SIZE_RANGE, STRENGTH_RANGE};
use crate::shapes::{self, Shape};
use crate::ui::context_menu::ContextMenu;
use crate::ui::cursor::{self, Blast};
use crate::ui::drawer::Drawer;
use crate::ui::help::Help;
use crate::ui::hud::{Hud, HudState};
use crate::ui::inspector::{Inspector, Props};
use crate::ui::now_playing::NowPlayingPill;
use crate::ui::spawner::{spawn_color, Spawner};
use crate::ui::title::{self, PixelOut};
use crate::ui::toasts::Toasts;
use crate::ui::tool_card::ToolCard;
use crate::ui::tool_picker::ToolPicker;
use crate::ui::visualizer::{self, VisStyle};
use crate::ui::{debug, icons, theme, Action, Input, ObjectCmd};
use crate::window_tracker::WindowTracker;
use macroquad::prelude::*;
use rapier2d::prelude::{Point, RigidBodyHandle};
use rfd::FileDialog;
use std::collections::HashMap;
use std::sync::Arc;

const TIME_STEPS: &[f32] = &[0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0];
const SPAWN_REPEAT: f32 = 0.11;

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
    history: History<Snapshot>,
    water: Water,
    bg: Background,
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
    recording: Option<Recording>,
    finishing: Option<Finishing>,
    /// An undo step was already recorded for the open properties panel.
    inspector_recorded: bool,

    hud: Hud,
    drawer: Drawer,
    card: ToolCard,
    picker: ToolPicker,
    spawner: Spawner,
    menu: ContextMenu,
    inspector: Inspector,
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
        let size = (screen_width(), screen_height());
        let world = PhysWorld::new(s.gravity, s.border, size);
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
            history: History::default(),
            water: Water::default(),
            bg,
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
            app.open_path(&path, vec2(x, 120.0));
        }
        app.history.clear();
        app
    }

    fn save_settings(&mut self) {
        self.s.volume = self.s.volume.clamp(0.0, 1.0);
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

        if (sw - self.last_size.0).abs() > 1.0 || (sh - self.last_size.1).abs() > 1.0 {
            self.world.resize((sw, sh));
            self.last_size = (sw, sh);
        }

        let mut input = Input::gather();
        let mut actions = Vec::new();
        let in_transition = self.title.is_some();
        if !in_transition {
            self.keyboard(&input, &mut actions);
        } else {
            input.consumed = true;
        }
        self.update_ui(dt, &mut input, &mut actions);
        self.handle_drops(input.mouse);
        if !in_transition {
            self.world_input(dt, &mut input);
        }
        for a in actions {
            self.apply(a, input.mouse);
        }
        self.sync_settings();
        self.poll_jobs();
        self.poll_recording();
        self.audio.tick();
        self.audio.analyze(dt, self.s.vis_gain);
        self.simulate(dt, input.mouse);

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

        if pressed(KeyCode::F1) {
            actions.push(Action::ToggleHelp);
        }
        if pressed(KeyCode::Escape) {
            if self.stroke.is_some() || self.link_drag.is_some() {
                self.stroke = None;
                self.link_drag = None;
            } else if self.help.fader.open {
                self.help.fader.open = false;
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
            return;
        }
        if pressed(KeyCode::Q) {
            self.quit = true;
        }
        if pressed(KeyCode::Space) {
            actions.push(Action::TogglePause);
        }
        if pressed(KeyCode::Tab) {
            actions.push(Action::OpenToolPicker);
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
        ];
        for (i, k) in digits.iter().enumerate() {
            if pressed(*k) {
                actions.push(Action::SetTool(Tool::ALL[i]));
                self.picker.fader.open = false;
            }
        }
        if pressed(KeyCode::H) {
            actions.push(Action::ToggleWater);
        }
        if pressed(KeyCode::F11) {
            actions.push(Action::ToggleRecording);
        }
        if pressed(KeyCode::I) {
            if let Some(i) = object_at(&self.objects, &self.world, input.mouse.x, input.mouse.y) {
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
        if pressed(KeyCode::T) {
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
            actions.push(Action::FetchButtons);
        }
        if pressed(KeyCode::L) {
            actions.push(Action::FetchLogos);
        }
        if pressed(KeyCode::R) {
            actions.push(Action::ClearAll);
        }
        if pressed(KeyCode::F12) {
            actions.push(Action::Screenshot);
        }
        if pressed(KeyCode::Delete) || pressed(KeyCode::Backspace) {
            if let Some(i) = object_at(&self.objects, &self.world, input.mouse.x, input.mouse.y) {
                let h = self.objects[i].body;
                actions.push(Action::Object(ObjectCmd::Delete, h));
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
        self.picker.update(dt, input, actions);
        self.menu.update(dt, input, actions);
        self.update_inspector(dt, input);
        self.toasts.update(dt);
        self.drawer.fader.open = self.paused;
        let top = self.hud.bottom();
        self.drawer.update(dt, &mut self.s, top, input, actions);
        self.spawner.update(dt, &mut self.s, input);
        self.card.update(dt, &mut self.s, input, actions);
        let np = self.audio.now_playing();
        self.now_playing.update(dt, np.as_ref(), self.paused, input, actions);
        let hud_state = self.hud_state();
        let grabbing = self.grab.is_some()
            || self.field_active
            || self.card.dragging()
            || self.inspector.dragging()
            || self.stroke.is_some()
            || self.link_drag.is_some();
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
        let m = input.mouse;
        let pos = (m.x, m.y);

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

        if input.left_pressed && !input.consumed {
            if self.spawner.is_open() {
                self.record("Spawn shapes");
                self.spawn_shape_at(m);
                self.spawn_timer = SPAWN_REPEAT * 2.0;
            } else {
                let tool = self.s.tool;
                if tool == Tool::Draw {
                    let c = spawn_color(self.s.spawn_color, rand::gen_range(0.0, 400.0));
                    let rgb = [(c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8];
                    self.stroke = Some(Stroke { points: vec![(m.x, m.y)], rgb });
                } else if tool == Tool::Link {
                    self.start_link(m);
                } else if tool.is_field() {
                    self.field_active = true;
                } else if tool == Tool::Bomb {
                    let handles: Vec<RigidBodyHandle> = self.objects.iter().map(|o| o.body).collect();
                    tools::detonate(&mut self.world.bodies, &handles, pos, self.s.tool_radius, self.s.tool_strength);
                    self.blasts.push(Blast::new(m, self.s.tool_radius));
                } else if tool.grabs() {
                    if let Some(i) = object_at(&self.objects, &self.world, m.x, m.y) {
                        self.grab = Some(Grab::new(&self.world.bodies, self.objects[i].body, to_phys(m.x, m.y), tool));
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

        if let Some(stroke) = &mut self.stroke {
            let last = *stroke.points.last().expect("strokes start with a point");
            if (m.x - last.0).hypot(m.y - last.1) >= drawing::SAMPLE_SPACING {
                stroke.points.push((m.x, m.y));
            }
        }

        if input.left_released || !input.left_down {
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
        }

        if input.right_pressed && !input.consumed && !input.over_ui {
            let near_link = (self.s.tool == Tool::Link)
                .then(|| {
                    (0..self.links.len())
                        .map(|i| (i, self.links[i].distance_px(&self.world, m)))
                        .filter(|&(_, d)| d < 8.0)
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                })
                .flatten();
            if let Some((i, _)) = near_link {
                self.record("Remove link");
                let l = self.links.remove(i);
                l.remove(&mut self.world);
                self.toasts.status("link", format!("{} removed", l.kind.label()));
            } else if let Some(i) = object_at(&self.objects, &self.world, m.x, m.y) {
                let o = &self.objects[i];
                let linked = self.links.iter().any(|l| l.involves(o.body));
                self.menu.open(m, o.body, o.name(), o.pinned, linked);
            }
        }
    }

    fn handle_drops(&mut self, mouse: Vec2) {
        let files = get_dropped_files();
        let n = files.len();
        if n > 0 {
            self.record("Drop files");
        }
        for (i, f) in files.into_iter().enumerate() {
            let at = mouse + vec2((i as f32 - n as f32 / 2.0) * 30.0, -(i as f32) * 10.0);
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
            Action::SetTool(t) => {
                if self.s.tool != t {
                    self.s.tool = t;
                    self.grab = None;
                    self.field_active = false;
                    self.stroke = None;
                    self.link_drag = None;
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
                        self.add_object(Source::File(p.to_string_lossy().into_owned()), vec2(x, self.spawn_y()), None);
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
                if n > 0 {
                    self.toasts.info(format!("Cleared {n} objects"));
                }
            }
            Action::SaveScene => self.save_scene(),
            Action::LoadScene => {
                let mut dlg = FileDialog::new().add_filter("Gravity scene", &[SCENE_EXT]);
                if let Some(dir) = config::scene_dir().filter(|d| d.is_dir()) {
                    dlg = dlg.set_directory(dir);
                }
                if let Some(p) = dlg.pick_file() {
                    self.load_scene(&p);
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
                let at = if self.paused { vec2(screen_width() / 2.0, self.spawn_y()) } else { mouse };
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
            Action::Undo => self.undo(false),
            Action::Redo => self.undo(true),
            Action::ToggleRecording => self.toggle_recording(),
            Action::Object(cmd, handle) => self.object_cmd(cmd, handle, mouse),
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
            ObjectCmd::Properties => None,
        };
        if let Some(label) = label {
            self.record(label);
        }
        match cmd {
            ObjectCmd::Delete => self.remove_object(i),
            ObjectCmd::Properties => {
                let o = &self.objects[i];
                let (p, _) = o.screen_pos(&self.world);
                self.inspector.open(o.body, o.name(), p);
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
    fn spawn_y(&self) -> f32 {
        self.hud.bottom().max(20.0) + 60.0 + rand::gen_range(0.0, 50.0)
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
        let size = self.s.spawn_size.round() as u32;
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
        let o =
            Object::spawn(&mut self.world, source, visual, Placement { pos_px: (at.x, at.y), ..Default::default() });
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

    fn clear_objects(&mut self) {
        self.grab = None;
        self.link_drag = None;
        self.menu.close();
        self.inspector.close();
        for l in self.links.drain(..) {
            l.remove(&mut self.world);
        }
        for o in self.objects.drain(..) {
            o.destroy(&mut self.world);
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Undo / redo
    // ═══════════════════════════════════════════════════════════
    /// Remember the scene before an edit, so it can be undone.
    fn record(&mut self, label: &str) {
        let snap = Snapshot::capture(&self.world, &self.objects, &self.links);
        self.history.record(label, snap);
    }

    fn undo(&mut self, redo: bool) {
        let current = Snapshot::capture(&self.world, &self.objects, &self.links);
        let step = if redo { self.history.redo(current) } else { self.history.undo(current) };
        let Some((label, snap)) = step else {
            self.toasts.status("undo", if redo { "Nothing to redo" } else { "Nothing to undo" });
            return;
        };
        self.clear_objects();
        self.stroke = None;
        let (objects, links) = snap.restore(&mut self.world);
        self.objects = objects;
        self.links = links;
        self.toasts.status("undo", format!("{}: {label}", if redo { "Redo" } else { "Undo" }));
    }

    // ═══════════════════════════════════════════════════════════
    // Draw and Link tools
    // ═══════════════════════════════════════════════════════════
    fn finish_stroke(&mut self, stroke: Stroke, shift: bool) {
        let Some(placed) = drawing::from_stroke(&stroke.points, self.s.draw_thickness, stroke.rgb) else { return };
        let pinned = self.s.draw_pinned != shift;
        let placement = Placement { pos_px: placed.center, size_px: Some(placed.size), pinned, ..Default::default() };
        self.record("Draw");
        match Object::load(&mut self.world, Source::Drawing(Arc::new(placed.drawing)), placement) {
            Some(o) => self.objects.push(o),
            None => self.toasts.error("Could not create the drawing"),
        }
    }

    fn start_link(&mut self, m: Vec2) {
        let p = to_phys(m.x, m.y);
        let at = Point::new(p.0, p.1);
        let under = objects_at(&self.objects, &self.world, m.x, m.y);
        if self.s.link_kind == LinkKind::Hinge {
            let (a, b) = match under.as_slice() {
                [] => {
                    self.toasts.status("link", "Click on an object to nail it, or where two objects overlap");
                    return;
                }
                [i] => (self.objects[*i].body, None),
                [i, j, ..] => (self.objects[*i].body, Some(self.objects[*j].body)),
            };
            self.add_link(LinkKind::Hinge, a, b, at, at);
        } else {
            let from = under.first().map(|&i| {
                let body = self.objects[i].body;
                (body, self.world.bodies[body].position().inverse_transform_point(&at))
            });
            self.link_drag = Some(LinkDrag { from, start: at, start_px: m });
        }
    }

    fn finish_link(&mut self, m: Vec2) {
        let Some(drag) = self.link_drag.take() else { return };
        if m.distance(drag.start_px) < 10.0 {
            self.toasts.status("link", "Drag from one object to another (or to the background)");
            return;
        }
        let p = to_phys(m.x, m.y);
        let end = Point::new(p.0, p.1);
        let from_body = drag.from.map(|(b, _)| b);
        let target = objects_at(&self.objects, &self.world, m.x, m.y)
            .into_iter()
            .map(|i| self.objects[i].body)
            .find(|&b| Some(b) != from_body);
        let kind = self.s.link_kind;
        match (drag.from, target) {
            (Some((a, local)), b) => {
                let pa = self.world.bodies.get(a).map_or(drag.start, |body| body.position() * local);
                self.add_link(kind, a, b, pa, end)
            }
            (None, Some(b)) => self.add_link(kind, b, None, end, drag.start),
            (None, None) => self.toasts.status("link", "Start or end the link on an object"),
        }
    }

    fn add_link(
        &mut self,
        kind: LinkKind,
        a: RigidBodyHandle,
        b: Option<RigidBodyHandle>,
        pa: Point<f32>,
        pb: Point<f32>,
    ) {
        self.record(kind.label());
        if let Some(l) = Link::new(&mut self.world, kind, a, b, pa, pb) {
            self.links.push(l);
            let hint = if b.is_none() { " to the background" } else { "" };
            self.toasts.status("link", format!("{} added{hint}  ·  right-click it to remove", kind.label()));
        }
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
                        let at = vec2(x, self.spawn_y() - 40.0);
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
        let scene = scene::capture(&self.world, &self.objects, &self.links, water);
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
                self.clear_objects();
                self.s.gravity = sc.gravity.clamp(crate::settings::GRAVITY_RANGE.0, crate::settings::GRAVITY_RANGE.1);
                self.s.border = sc.border;
                self.world.set_border(sc.border);
                self.world.set_gravity(self.s.gravity);
                if sc.version >= 2 {
                    self.s.water = sc.water.is_some();
                }
                if let Some(w) = sc.water {
                    self.s.water_level = w.level;
                    self.s.water_density = w.density;
                    self.s = self.s.clone().sanitized();
                }
                let (objs, links, failed) = scene::instantiate(&sc, &mut self.world);
                let n = objs.len();
                self.objects = objs;
                self.links = links;
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
    }

    fn simulate(&mut self, dt: f32, mouse: Vec2) {
        let running = !self.paused || self.step_once;
        if running {
            let pos = (mouse.x, mouse.y);
            self.world.reset_forces();
            if let Some(g) = &self.grab {
                g.apply(&mut self.world.bodies, to_phys(mouse.x, mouse.y));
            }
            if self.field_active {
                let handles: Vec<RigidBodyHandle> = self.objects.iter().map(|o| o.body).collect();
                tools::apply_field(
                    &mut self.world.bodies,
                    &handles,
                    self.s.tool,
                    pos,
                    self.s.tool_radius,
                    self.s.tool_strength,
                );
            }
            self.world.border.apply_forces(&mut self.world, &self.objects);
            links::apply_springs(&self.links, &mut self.world);
            if self.s.water {
                let rest = water::rest_level(&self.world, self.s.water_level);
                let step = if self.step_once { crate::config::PHYSICS_DT } else { dt * self.s.time_scale };
                self.water.apply(&mut self.world, &self.objects, rest, self.s.water_density, step);
                self.water.step(step, self.audio.analyzer.bass, self.audio.analyzer.beat_now);
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
                self.world.step_frame(dt, self.s.time_scale);
            }

            let dead = self.world.border.apply_positions(&mut self.world, &self.objects);
            for h in dead {
                if let Some(i) = self.objects.iter().position(|o| o.body == h) {
                    self.remove_object(i);
                }
            }
            links::prune(&mut self.links, &self.world);

            let (len, fade) = (self.s.trail_length as usize, self.s.trail_fade);
            for o in &mut self.objects {
                if self.s.trails {
                    o.update_trail(&self.world, len, fade);
                } else {
                    o.clear_trail();
                }
            }
        }
        let dt_ms = dt * 1000.0 * if self.paused { 0.0 } else { 1.0 };
        for o in &mut self.objects {
            o.update_anim(dt_ms);
        }
        self.blasts.retain(Blast::alive);
    }

    /// Make resting objects hop on a beat, harder with more bass.
    fn dance(&mut self) {
        let kick = 2.0 + self.audio.analyzer.bass * 3.5;
        for o in &self.objects {
            let Some(b) = self.world.bodies.get_mut(o.body) else { continue };
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
        let (sw, sh) = (screen_width(), screen_height());
        let m = input.mouse;
        self.bg.draw(sw, sh);
        let floor = if self.world.border.walls().floor { WALL_T * PPM } else { 0.0 };
        visualizer::draw_background(self.s.vis_background, &self.audio.analyzer, sw, sh, floor);
        self.world.border.draw(sw, sh);

        if self.s.trails {
            for o in self.objects.iter().filter(|o| !o.is_visualizer()) {
                o.draw_trail(self.s.trail_fade);
            }
        }
        for o in &self.objects {
            if o.is_visualizer() {
                let (p, angle) = o.screen_pos(&self.world);
                visualizer::draw_object(self.s.vis_object, &self.audio.analyzer, p, angle, o.size, 1.0);
            } else {
                o.draw(&self.world);
            }
        }
        for l in &self.links {
            l.draw(&self.world);
        }
        for o in self.objects.iter().filter(|o| o.pinned) {
            let (p, _) = o.screen_pos(&self.world);
            draw_circle(p.x, p.y, 9.0, theme::alpha(BLACK, 0.35));
            icons::pin(p, 14.0, theme::WARNING);
        }
        if self.s.water {
            let rest = water::rest_level(&self.world, self.s.water_level);
            self.water.draw(&self.world, rest, sw, sh);
        }
        for target in [self.menu.target, self.inspector.target].into_iter().flatten() {
            if let Some(o) = self.objects.iter().find(|o| o.body == target) {
                let c = o.corners(&self.world);
                for i in 0..4 {
                    let (a, b) = (c[i], c[(i + 1) % 4]);
                    draw_line(a.x, a.y, b.x, b.y, 1.5, theme::alpha(theme::ACCENT_HI, 0.9));
                }
            }
        }

        if self.screenshot {
            self.screenshot = false;
            self.take_screenshot();
        }
        self.capture_frame();

        if let Some(g) = &self.grab {
            cursor::draw_grab(g, &self.world, m);
        }
        for b in &self.blasts {
            b.draw();
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
                    _ => cursor::draw_tool_cursor(self.s.tool, m, self.s.tool_radius, self.field_active),
                }
            }
        }

        let top = self.hud.bottom();
        if self.debug {
            debug::draw(
                &self.world,
                &self.objects,
                &debug::DebugStats { paused: self.paused, time_scale: self.s.time_scale },
                m,
                top,
            );
        }

        self.card.draw(&self.s, m);
        let np = self.audio.now_playing();
        self.now_playing.draw(np.as_ref(), m);
        self.spawner.draw(&self.s, m);
        self.drawer.draw(&self.s, top, m, np.is_some());
        self.hud.draw(&self.hud_state(), m);
        self.toasts.draw(top, &self.jobs, dt);
        self.inspector.draw(self.inspected().as_ref(), m);
        self.menu.draw(m);
        self.picker.draw(self.s.tool, m);
        self.help.draw();
        self.draw_rec_indicator(top);
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
