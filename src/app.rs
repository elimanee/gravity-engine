//! Application state and the per-frame loop: input → UI → actions →
//! simulation → rendering.

use crate::audio::Audio;
use crate::background::{Background, BgMode};
use crate::config::{self, ext_of, file_name_of, IMAGE_EXT, SCENE_EXT};
use crate::net::{FetchEvent, FetchJob, FetchKind};
use crate::physics::object::{object_at, Object, Placement, Source, Visual};
use crate::physics::tools::{self, Grab, Tool};
use crate::physics::PhysWorld;
use crate::scene;
use crate::settings::{Settings, RADIUS_RANGE, SPAWN_SIZE_RANGE, STRENGTH_RANGE};
use crate::shapes::{self, Shape};
use crate::ui::context_menu::ContextMenu;
use crate::ui::cursor::{self, Blast};
use crate::ui::drawer::Drawer;
use crate::ui::help::Help;
use crate::ui::hud::{Hud, HudState};
use crate::ui::now_playing::NowPlayingPill;
use crate::ui::spawner::{spawn_color, Spawner};
use crate::ui::title::{self, PixelOut};
use crate::ui::toasts::Toasts;
use crate::ui::tool_card::ToolCard;
use crate::ui::tool_picker::ToolPicker;
use crate::ui::{debug, icons, theme, Action, Input, ObjectCmd};
use crate::window_tracker::WindowTracker;
use macroquad::prelude::*;
use rapier2d::prelude::RigidBodyHandle;
use rfd::FileDialog;
use std::collections::HashMap;
use std::sync::Arc;

const TIME_STEPS: &[f32] = &[0.1, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0];
const SPAWN_REPEAT: f32 = 0.11;

type SpawnKey = (Shape, u32, (u8, u8, u8));

enum Title {
    Showing,
    Leaving(PixelOut),
}

pub struct App {
    s: Settings,
    world: PhysWorld,
    objects: Vec<Object>,
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

    hud: Hud,
    drawer: Drawer,
    card: ToolCard,
    picker: ToolPicker,
    spawner: Spawner,
    menu: ContextMenu,
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
            hud: Hud::default(),
            drawer: Drawer::default(),
            card: ToolCard::default(),
            picker: ToolPicker::default(),
            spawner: Spawner::default(),
            menu: ContextMenu::default(),
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
        self.audio.tick();
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
            if self.help.fader.open {
                self.help.fader.open = false;
            } else if self.picker.fader.open {
                self.picker.fader.open = false;
            } else if self.menu.fader.open {
                self.menu.close();
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
        ];
        for (i, k) in digits.iter().enumerate() {
            if pressed(*k) {
                actions.push(Action::SetTool(Tool::ALL[i]));
                self.picker.fader.open = false;
            }
        }
        let dir = if shift { -1 } else { 1 };
        if pressed(KeyCode::B) {
            actions.push(Action::CycleBorder(dir));
        }
        if pressed(KeyCode::G) {
            actions.push(if shift { Action::PickBackground } else { Action::CycleBackground(1) });
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
        self.toasts.update(dt);
        self.drawer.fader.open = self.paused;
        let top = self.hud.bottom();
        self.drawer.update(dt, &mut self.s, top, input, actions);
        self.spawner.update(dt, &mut self.s, input);
        self.card.update(dt, &mut self.s, input, actions);
        let np = self.audio.now_playing();
        self.now_playing.update(dt, np.as_ref(), self.paused, input, actions);
        let hud_state = self.hud_state();
        let grabbing = self.grab.is_some() || self.field_active || self.card.dragging();
        let mut hud_actions = vec![];
        self.hud.update(dt, &hud_state, input, grabbing, &mut hud_actions);
        actions.extend(hud_actions);
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
                self.spawn_shape_at(m);
                self.spawn_timer = SPAWN_REPEAT * 2.0;
            } else {
                let tool = self.s.tool;
                if tool.is_field() {
                    self.field_active = true;
                } else if tool == Tool::Bomb {
                    let handles: Vec<RigidBodyHandle> = self.objects.iter().map(|o| o.body).collect();
                    tools::detonate(&mut self.world.bodies, &handles, pos, self.s.tool_radius, self.s.tool_strength);
                    self.blasts.push(Blast::new(m, self.s.tool_radius));
                } else if let Some(i) = object_at(&self.objects, &self.world, m.x, m.y) {
                    self.grab = Some(Grab::new(&self.world.bodies, self.objects[i].body, pos, tool));
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

        if input.left_released || !input.left_down {
            if let Some(g) = self.grab.take() {
                g.release(&mut self.world.bodies, pos);
            }
            self.field_active = false;
        }

        if input.right_pressed && !input.consumed && !input.over_ui {
            if let Some(i) = object_at(&self.objects, &self.world, m.x, m.y) {
                let o = &self.objects[i];
                self.menu.open(m, o.body, o.name(), o.pinned);
            }
        }
    }

    fn handle_drops(&mut self, mouse: Vec2) {
        let files = get_dropped_files();
        let n = files.len();
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
            Action::Object(cmd, handle) => self.object_cmd(cmd, handle, mouse),
        }
    }

    fn object_cmd(&mut self, cmd: ObjectCmd, handle: RigidBodyHandle, _mouse: Vec2) {
        let Some(i) = self.objects.iter().position(|o| o.body == handle) else { return };
        match cmd {
            ObjectCmd::Delete => self.remove_object(i),
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
        o.destroy(&mut self.world);
    }

    fn clear_objects(&mut self) {
        self.grab = None;
        self.menu.close();
        for o in self.objects.drain(..) {
            o.destroy(&mut self.world);
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
        let scene = scene::capture(&self.world, &self.objects);
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
                self.clear_objects();
                self.s.gravity = sc.gravity.clamp(crate::settings::GRAVITY_RANGE.0, crate::settings::GRAVITY_RANGE.1);
                self.s.border = sc.border;
                self.world.set_border(sc.border);
                self.world.set_gravity(self.s.gravity);
                let (objs, failed) = scene::instantiate(&sc, &mut self.world);
                let n = objs.len();
                self.objects = objs;
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
                g.apply(&mut self.world.bodies, pos);
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

    // ═══════════════════════════════════════════════════════════
    // Rendering
    // ═══════════════════════════════════════════════════════════
    fn draw(&mut self, dt: f32, input: &Input) {
        let (sw, sh) = (screen_width(), screen_height());
        let m = input.mouse;
        self.bg.draw(sw, sh);
        self.world.border.draw(sw, sh);

        if self.s.trails {
            for o in &self.objects {
                o.draw_trail(self.s.trail_fade);
            }
        }
        for o in &self.objects {
            o.draw(&self.world);
        }
        for o in self.objects.iter().filter(|o| o.pinned) {
            let (p, _) = o.screen_pos(&self.world);
            draw_circle(p.x, p.y, 9.0, theme::alpha(BLACK, 0.35));
            icons::pin(p, 14.0, theme::WARNING);
        }
        if let Some(target) = self.menu.target {
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

        if let Some(g) = &self.grab {
            cursor::draw_grab(g, &self.world, m);
        }
        for b in &self.blasts {
            b.draw();
        }
        if !input.over_ui && self.title.is_none() {
            if self.spawner.is_open() {
                self.spawner.draw_ghost(&self.s, m);
            } else if self.grab.is_none() {
                cursor::draw_tool_cursor(self.s.tool, m, self.s.tool_radius, self.field_active);
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
        self.menu.draw(m);
        self.picker.draw(self.s.tool, m);
        self.help.draw();
    }

    fn take_screenshot(&mut self) {
        let img = get_screen_data();
        let (w, h) = (img.width as usize, img.height as usize);
        // The framebuffer is bottom-up.
        let mut flipped = Vec::with_capacity(img.bytes.len());
        for row in (0..h).rev() {
            flipped.extend_from_slice(&img.bytes[row * w * 4..(row + 1) * w * 4]);
        }
        for px in flipped.chunks_exact_mut(4) {
            px[3] = 255;
        }
        let result = (|| -> Result<std::path::PathBuf, String> {
            let dir = config::screenshot_dir().ok_or("no home directory")?;
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or_default();
            let path = dir.join(format!("gravity-{stamp}.png"));
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
    }
}
