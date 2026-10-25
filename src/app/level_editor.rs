//! The Sonic level editor: lay out a level in the infinite world (ground,
//! ledges, loops, springs, and anything the sandbox tools make), place
//! the start, rings, badniks, monitors, spikes, star posts, air vents, the
//! goal and a boss, play it, and save it to "My levels" (Library → Sonic).

use super::*;
use crate::game::{
    art as game_art, BossKind, EnemyKind, Game, LevelSpec, MonitorKind, Placed, ARENA_HALF, MONITOR, SPIKES,
};
use crate::library::{levels, Builder, LEAF, STONE, WOOD};
use crate::sonic::SCALE;
use crate::ui::level_bar::{Item, LevelView};
use crate::ui::LevelCmd;
use rapier2d::prelude::{nalgebra, vector, QueryFilter};
use std::path::PathBuf;

/// Ground colour of the blocks it makes.
const DIRT: [u8; 3] = [150, 98, 62];
/// Rings in a dragged row are this far apart (px).
const RING_GAP: f32 = 36.0;
/// How near a click must be to erase something (px).
const ERASE_REACH: f32 = 30.0;

pub(super) struct LevelEditor {
    pub name: String,
    pub path: Option<PathBuf>,
    pub item: Option<Item>,
    pub monitor: MonitorKind,
    pub badnik: EnemyKind,
    /// Where a drag started (ground, ledge, row of rings), and the last ring laid.
    drag: Option<Vec2>,
    last_ring: Option<Vec2>,
    spec: LevelSpec,
    /// The character's start (world px).
    start: Option<Vec2>,
    /// The level while it is played from the editor.
    testing: Option<scene::SceneFile>,
    /// The level's things, shown (not moving) while editing.
    preview: Option<Game>,
    /// Saved since the last change.
    saved: bool,
}

impl App {
    /// Open the editor on a new level, or on "My levels" entry `i`.
    pub(super) fn open_level_editor(&mut self, existing: Option<usize>) {
        self.challenge = None;
        self.editor = None;
        self.stop_game();
        self.library.fader.open = false;
        let mut ed = LevelEditor {
            name: String::new(),
            path: None,
            item: None,
            monitor: MonitorKind::Rings,
            badnik: EnemyKind::Crawler,
            drag: None,
            last_ring: None,
            spec: LevelSpec::default(),
            start: None,
            testing: None,
            preview: None,
            saved: true,
        };
        let sc = match existing {
            Some(i) => {
                self.refresh_levels();
                let Some((path, _)) = self.levels.get(i).cloned() else { return };
                match scene::read(&path) {
                    Ok(sc) => {
                        ed.spec = sc.game.clone().unwrap_or_default();
                        ed.name = ed.spec.name.clone();
                        ed.path = Some(path);
                        sc
                    }
                    Err(e) => {
                        self.toasts.error(format!("Could not open the level: {e}"));
                        return;
                    }
                }
            }
            None => self.blank_level(),
        };
        // The world it was made in first: points in metres turn into pixels in it.
        self.level_world(&sc);
        ed.start = sc.sonic.map(|[x, y]| crate::physics::to_screen(x, y));
        self.level_editor = Some(ed);
        self.edit_scene(sc);
        if let Some(start) = self.level_editor.as_ref().and_then(|e| e.start) {
            self.camera.center = start + vec2(screen_width() * 0.3, -60.0);
        }
        self.camera.zoom = 1.0;
        self.history.clear();
        self.toasts.info("Sonic level editor  ·  pick something to place, build with any tool, Play to try it");
    }

    /// A new level: the infinite world with nothing under it, and a first
    /// stretch of ground with the start on it.
    fn blank_level(&mut self) -> scene::SceneFile {
        if !self.infinite() {
            self.set_world_size(4);
        }
        let (aw, ah) = self.arena();
        let x = aw / 2.0;
        let top = ah - 160.0;
        let mut b = Builder::world(ah);
        b.scene.border = crate::physics::borders::BorderMode::Open;
        b.ground(x - 600.0, x + 900.0, top, DIRT, LEAF);
        b.scene.sonic = Some(to_phys(x - 400.0, top - 19.0 * SCALE).into());
        b.scene.world_size = Some(4);
        b.scene
    }

    /// Show `sc` for editing: its terrain and rings, the level's things
    /// standing still, and no one playing.
    fn edit_scene(&mut self, mut sc: scene::SceneFile) {
        sc.game = None;
        sc.sonic = None;
        self.level_world(&sc);
        self.apply_scene(sc);
        self.sonic = None;
        self.level_preview();
    }

    /// Switch to the world size `sc` was made in.
    fn level_world(&mut self, sc: &scene::SceneFile) {
        if let Some(size) = sc.world_size.filter(|&s| s != self.s.world_size && (1..=4).contains(&s)) {
            self.set_world_size(size);
        }
    }

    /// Rebuild the still picture of the level's things.
    fn level_preview(&mut self) {
        let Some(ed) = self.level_editor.as_mut() else { return };
        if let Some(old) = ed.preview.take() {
            old.remove(&mut self.world);
        }
        if ed.testing.is_none() {
            let start = ed.start.unwrap_or_default();
            ed.preview = Some(Game::new(&ed.spec, &mut self.world, start));
        }
    }

    /// Take the preview's bodies out of the world (the scene is going).
    pub(super) fn drop_level_preview(&mut self) {
        if let Some(p) = self.level_editor.as_mut().and_then(|e| e.preview.take()) {
            p.remove(&mut self.world);
        }
    }

    pub(super) fn level_view(&self) -> Option<LevelView> {
        let ed = self.level_editor.as_ref()?;
        Some(LevelView {
            item: ed.item,
            monitor: ed.monitor,
            badnik: ed.badnik,
            boss: ed.spec.boss_kind,
            hunter: ed.spec.hunter,
            testing: ed.testing.is_some(),
            has_start: ed.start.is_some(),
            has_end: ed.spec.goal.is_some() || ed.spec.boss.is_some(),
            saved: ed.saved,
        })
    }

    /// Playing the level from the editor.
    pub(super) fn level_testing(&self) -> bool {
        self.level_editor.as_ref().is_some_and(|e| e.testing.is_some())
    }

    pub(super) fn level_cmd(&mut self, cmd: LevelCmd) {
        let Some(ed) = self.level_editor.as_mut() else { return };
        match cmd {
            LevelCmd::Pick(item) => {
                if item == Item::Hunter {
                    ed.spec.hunter = !ed.spec.hunter;
                    ed.saved = false;
                    let msg = if ed.spec.hunter { "Sonic.exe will hunt the player" } else { "No more Sonic.exe" };
                    self.toasts.status("level", msg);
                    return;
                }
                // Again on Monitor, Badnik or Boss: the next kind.
                let again = ed.item == Some(item);
                if again && item == Item::Monitor {
                    let i = MonitorKind::ALL.iter().position(|&k| k == ed.monitor).unwrap_or(0);
                    ed.monitor = MonitorKind::ALL[(i + 1) % MonitorKind::ALL.len()];
                } else if again && item == Item::Badnik {
                    let i = EnemyKind::ALL.iter().position(|&k| k == ed.badnik).unwrap_or(0);
                    ed.badnik = EnemyKind::ALL[(i + 1) % EnemyKind::ALL.len()];
                } else if again && item == Item::Boss {
                    let i = BossKind::ALL.iter().position(|&k| k == ed.spec.boss_kind).unwrap_or(0);
                    ed.spec.boss_kind = BossKind::ALL[(i + 1) % BossKind::ALL.len()];
                    ed.saved = false;
                    ed.drag = None;
                    self.level_preview();
                    return;
                } else {
                    ed.item = if ed.item == Some(item) { None } else { Some(item) };
                }
                ed.drag = None;
            }
            LevelCmd::Play => self.level_play(),
            LevelCmd::Stop => self.level_stop(),
            LevelCmd::Save => self.level_save(),
            LevelCmd::Exit => {
                self.drop_level_preview();
                self.level_editor = None;
                self.toasts.info("Left the level editor");
            }
        }
    }

    /// The level as it stands, ready to play or save.
    fn level_scene(&self) -> Option<scene::SceneFile> {
        let ed = self.level_editor.as_ref()?;
        let water =
            self.s.water.then_some(scene::SceneWater { level: self.s.water_level, density: self.s.water_density });
        let mut sc = scene::capture(&self.world, &self.objects, &self.links, &self.zones, water);
        sc.gadgets = scene::capture_gadgets(&self.objects, &self.gadgets);
        sc.night = Some(self.s.night);
        sc.weather = Some(self.s.weather);
        sc.sonic = ed.start.map(|p| to_phys(p.x, p.y).into());
        sc.rings = self.rings.iter().map(|r| to_phys(r.x, r.y).into()).collect();
        let mut spec = ed.spec.clone();
        spec.name = ed.name.trim().to_string();
        sc.game = Some(spec);
        sc.world_size = Some(self.s.world_size);
        Some(sc)
    }

    fn level_play(&mut self) {
        let Some(ed) = self.level_editor.as_ref() else { return };
        if ed.start.is_none() {
            self.toasts.status("level", "Place the start first");
            return;
        }
        if ed.spec.goal.is_none() && ed.spec.boss.is_none() {
            self.toasts.status("level", "Place the goal (or a boss) first");
            return;
        }
        let Some(sc) = self.level_scene() else { return };
        self.drop_level_preview();
        if let Some(ed) = self.level_editor.as_mut() {
            ed.testing = Some(sc.clone());
            ed.item = None;
        }
        self.apply_scene(sc);
        self.toasts.info("Playing your level  ·  Stop to go back to editing");
    }

    fn level_stop(&mut self) {
        let Some(sc) = self.level_editor.as_mut().and_then(|e| e.testing.take()) else { return };
        self.stop_game();
        self.edit_scene(sc);
        self.history.clear();
    }

    fn level_save(&mut self) {
        let Some(sc) = self.level_scene() else { return };
        let Some(dir) = crate::config::level_dir() else {
            self.toasts.error("No config folder to save levels in");
            return;
        };
        let Some(ed) = self.level_editor.as_mut() else { return };
        if ed.name.trim().is_empty() {
            ed.name = "My level".into();
        }
        let path = ed.path.clone().unwrap_or_else(|| levels::free_path(&dir, &ed.name));
        let mut sc = sc;
        if let Some(g) = &mut sc.game {
            g.name = ed.name.trim().to_string();
        }
        match levels::write(&path, &sc) {
            Ok(()) => {
                ed.path = Some(path);
                ed.saved = true;
                let msg = format!("Saved “{}”  ·  play it from Library → Sonic", ed.name.trim());
                self.refresh_levels();
                self.toasts.success(msg);
            }
            Err(e) => self.toasts.error(format!("Could not save the level: {e}")),
        }
    }

    /// The ground under `m` (world px): where things placed there stand.
    fn ground_under(&self, m: Vec2) -> f32 {
        let (x, y) = to_phys(m.x, m.y);
        let filter = QueryFilter::default().exclude_sensors();
        match self.world.cast_ray(Point::new(x, y), vector![0.0, -1.0], 30.0, filter) {
            Some((_, toi, _)) => m.y + toi * PPM,
            None => m.y,
        }
    }

    /// Left press in the world with something picked: true when used.
    pub(super) fn level_press(&mut self, m: Vec2) -> bool {
        let Some(item) = self.level_editor.as_ref().filter(|e| e.testing.is_none()).and_then(|e| e.item) else {
            return false;
        };
        let g = self.ground_under(m);
        let at = |p: Vec2| -> [f32; 2] { to_phys(p.x, p.y).into() };
        let (_, ah) = self.arena();
        let mut objects: Option<Builder> = None;
        let Some(ed) = self.level_editor.as_mut() else { return false };
        match item {
            Item::Start => ed.start = Some(vec2(m.x, g - 19.0 * SCALE)),
            Item::Ring => {
                self.rings.push(m);
                ed.drag = Some(m);
                ed.last_ring = Some(m);
            }
            Item::Badnik => {
                let kind = ed.badnik;
                let p = if walks(kind) { vec2(m.x, g - 17.0) } else { m };
                ed.spec.enemies.push(Placed { kind, at: at(p) });
            }
            Item::Vent => ed.spec.vents.push(at(vec2(m.x, g))),
            Item::Pumpkin => ed.spec.pumpkins.push(at(vec2(m.x, g - 14.0))),
            Item::Hunter => return false,
            Item::Monitor => ed.spec.monitors.push(Placed { kind: ed.monitor, at: at(vec2(m.x, g - MONITOR.y / 2.0)) }),
            Item::Spikes => ed.spec.spikes.push(at(vec2(m.x, g - SPIKES.y / 2.0))),
            Item::Post => ed.spec.posts.push(at(vec2(m.x, g - 40.0))),
            Item::Goal => {
                ed.spec.goal = Some(at(vec2(m.x, g - 40.0)));
                if ed.spec.boss.is_some() {
                    self.toasts.status("level", "With a boss, the goal appears where it is beaten instead");
                }
            }
            Item::Boss => ed.spec.boss = Some(at(m)),
            Item::Spring => {
                let mut b = Builder::world(ah);
                b.spring(m.x, g);
                objects = Some(b);
            }
            Item::Loop => {
                let r = 110.0;
                let mut b = Builder::world(ah);
                b.sonic_loop(m.x, g - r, r, STONE);
                objects = Some(b);
            }
            Item::Ground | Item::Platform => ed.drag = Some(m),
            Item::Erase => {
                self.level_erase(m);
                return true;
            }
        }
        if let Some(ed) = self.level_editor.as_mut() {
            ed.saved = false;
        }
        if let Some(b) = objects {
            self.level_add(b, item.label());
        }
        self.level_preview();
        true
    }

    /// Dragging with the pointer down: rings along the way.
    pub(super) fn level_drag(&mut self, m: Vec2) {
        let Some(ed) = self.level_editor.as_mut().filter(|e| e.item == Some(Item::Ring)) else { return };
        let Some(last) = ed.last_ring.filter(|_| ed.drag.is_some()) else { return };
        if (m - last).length() >= RING_GAP {
            let p = last + (m - last).normalize() * RING_GAP;
            ed.last_ring = Some(p);
            self.rings.push(p);
        }
    }

    /// Pointer released: finish a ground or ledge drag.
    pub(super) fn level_release(&mut self, m: Vec2) {
        let (_, ah) = self.arena();
        let Some(ed) = self.level_editor.as_mut() else { return };
        let Some(start) = ed.drag.take() else { return };
        ed.last_ring = None;
        let (x0, x1) = (start.x.min(m.x), start.x.max(m.x));
        if x1 - x0 < 40.0 {
            return;
        }
        let mut b = Builder::world(ah);
        match ed.item {
            Some(Item::Ground) => b.ground(x0, x1, start.y.min(m.y), DIRT, LEAF),
            Some(Item::Platform) => b.platform(x0, x1, start.y, WOOD),
            _ => return,
        }
        ed.saved = false;
        let label = ed.item.map_or("", |i| i.label());
        self.level_add(b, label);
    }

    /// Add what `b` made to the scene (undoable).
    fn level_add(&mut self, b: Builder, what: &str) {
        self.record(&format!("Place {}", what.to_lowercase()));
        let made = scene::instantiate(&b.scene, &mut self.world);
        self.objects.extend(made.objects);
        self.links.extend(made.links);
    }

    /// Take away what is under `m`: a level thing, a ring, the start, or an object.
    fn level_erase(&mut self, m: Vec2) {
        let near = |p: [f32; 2]| (crate::physics::to_screen(p[0], p[1]) - m).length() < ERASE_REACH;
        let Some(ed) = self.level_editor.as_mut() else { return };
        let s = &mut ed.spec;
        let tally = |s: &LevelSpec| {
            let small = s.posts.len() + s.vents.len() + s.pumpkins.len();
            (s.enemies.len(), s.monitors.len(), s.spikes.len(), small, s.goal, s.boss)
        };
        let before = tally(s);
        if let Some(i) = s.enemies.iter().position(|e| near(e.at)) {
            s.enemies.remove(i);
        } else if let Some(i) = s.monitors.iter().position(|e| near(e.at)) {
            s.monitors.remove(i);
        } else if let Some(i) = s.spikes.iter().position(|&p| near(p)) {
            s.spikes.remove(i);
        } else if let Some(i) = s.posts.iter().position(|&p| near(p)) {
            s.posts.remove(i);
        } else if let Some(i) = s.vents.iter().position(|&p| near(p)) {
            s.vents.remove(i);
        } else if let Some(i) = s.pumpkins.iter().position(|&p| near(p)) {
            s.pumpkins.remove(i);
        } else if s.goal.is_some_and(near) {
            s.goal = None;
        } else if s.boss.is_some_and(near) {
            s.boss = None;
        }
        let changed = before != tally(s);
        if changed {
            ed.saved = false;
            self.level_preview();
            return;
        }
        if ed.start.is_some_and(|p| (p - m).length() < ERASE_REACH * 1.5) {
            ed.start = None;
            ed.saved = false;
            return;
        }
        if let Some(i) = self.rings.iter().position(|r| (*r - m).length() < ERASE_REACH * 0.7) {
            self.rings.remove(i);
            ed.saved = false;
            return;
        }
        if let Some(i) = object_at(&self.objects, &self.world, m.x, m.y) {
            ed.saved = false;
            self.record("Erase");
            self.remove_object(i);
        }
    }

    /// The level's things, the start and what is being dragged (world pass).
    pub(super) fn draw_level_editor(&self, m: Vec2) {
        let Some(ed) = self.level_editor.as_ref().filter(|e| e.testing.is_none()) else { return };
        let t = get_time() as f32;
        if let Some(g) = &ed.preview {
            game_art::draw(g, t, self.game_sprites.as_ref());
        }
        // A boss guards its arena.
        if let Some(b) = ed.spec.boss {
            let p = crate::physics::to_screen(b[0], b[1]);
            let c = Color::new(1.0, 0.4, 0.3, 0.6);
            for x in [p.x - ARENA_HALF, p.x + ARENA_HALF] {
                let mut y = p.y - 300.0;
                while y < p.y + 300.0 {
                    draw_line(x, y, x, y + 12.0, 2.0, c);
                    y += 22.0;
                }
            }
        }
        if let Some(start) = ed.start {
            let who = crate::sonic::Sonic::as_character(start, self.s.character);
            match self.sonic_sheet.get(who.character) {
                Some(sheet) => sheet.draw(&who),
                None => crate::sonic::art::draw(&who),
            }
            theme::text_bold_centered("START", start.x, start.y - 44.0, 12.0, Color::new(1.0, 1.0, 1.0, 0.85));
        }
        // Drag previews.
        if let (Some(start), Some(item)) = (ed.drag, ed.item) {
            let (x0, x1) = (start.x.min(m.x), start.x.max(m.x));
            let c = Color::new(0.4, 0.9, 0.5, 0.7);
            match item {
                Item::Ground => {
                    let top = start.y.min(m.y);
                    let (_, ah) = self.arena();
                    draw_rectangle(x0, top, x1 - x0, ah - top, Color { a: 0.18, ..c });
                    draw_rectangle_lines(x0, top, x1 - x0, ah - top, 2.0, c);
                }
                Item::Platform => draw_line(x0, start.y, x1, start.y, 16.0, Color { a: 0.4, ..c }),
                _ => {}
            }
        }
        // Where a click would put it.
        let dropped = ed.item.is_some_and(|i| match i {
            Item::Ring | Item::Boss | Item::Erase | Item::Hunter => false,
            Item::Badnik => walks(ed.badnik),
            _ => true,
        });
        if dropped && ed.drag.is_none() {
            let g = self.ground_under(m);
            draw_line(m.x, m.y, m.x, g, 1.0, Color::new(1.0, 1.0, 1.0, 0.35));
            draw_circle_lines(m.x, g, 6.0, 1.5, Color::new(1.0, 1.0, 1.0, 0.7));
        }
        if ed.item == Some(Item::Erase) {
            draw_circle_lines(m.x, m.y, ERASE_REACH, 1.5, Color::new(1.0, 0.4, 0.4, 0.8));
        }
    }

    pub(super) fn refresh_levels(&mut self) {
        self.levels = levels::list();
    }

    /// Play "My levels" entry `i`.
    pub(super) fn open_level(&mut self, i: usize) {
        self.refresh_levels();
        let Some((path, _)) = self.levels.get(i).cloned() else { return };
        match scene::read(&path) {
            Ok(sc) => {
                self.challenge = None;
                self.editor = None;
                self.drop_level_preview();
                self.level_editor = None;
                self.level_world(&sc);
                self.apply_scene(sc);
                self.camera.zoom = 1.0;
                self.library.fader.open = false;
            }
            Err(e) => self.toasts.error(format!("Could not open the level: {e}")),
        }
    }
}

/// Badniks that walk on the ground (the others float where placed).
fn walks(kind: EnemyKind) -> bool {
    matches!(kind, EnemyKind::Crawler | EnemyKind::Crab)
}
