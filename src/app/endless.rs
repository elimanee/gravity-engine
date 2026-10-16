//! Sonic's endless run, in the infinite world: stretches of terrain are
//! made ahead of him as he goes (hills, loops, ramps, springs, platforms,
//! crates, rings) and cleared away behind him, through themed zones
//! (meadow, twilight, snowfield, lagoon, storm) with more and more
//! badniks and spikes the further he gets. It is played as a level with
//! one life; the best run is kept as a ghost that races him on the same
//! course next time.

use super::*;
use crate::background::LevelSky;
use crate::game::{EnemyKind, LevelSpec, MonitorKind, MONITOR, SPIKES};
use crate::library::{Builder, CORAL, LEAF, SKY, SNOW, STONE, VIOLET, WOOD};
use crate::weather::Weather;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Terrain kept ready ahead of him, and kept behind him (px).
const AHEAD: f32 = 2600.0;
const BEHIND: f32 = 1800.0;
/// The floor in the builder's design coordinates.
const FLOOR: f32 = 690.0;
/// The ghost's track is sampled this often (per second).
const GHOST_RATE: f32 = 15.0;
/// Seconds between the end of a run and the next.
const RESTART: f32 = 3.0;
/// Seconds the sky takes to turn into the next zone's.
const SKY_TURN: f32 = 2.5;
/// Size of the ghost's texture (px).
const GHOST_PX: f32 = 200.0;

/// A zone of the run: its name, length (m), colours and sky.
struct Zone {
    name: &'static str,
    length: f32,
    ground: [u8; 3],
    edge: [u8; 3],
    sky: LevelSky,
    weather: Weather,
    night: bool,
    water: bool,
}

const ZONES: [Zone; 5] = [
    Zone {
        name: "Meadow",
        length: 250.0,
        ground: STONE,
        edge: LEAF,
        sky: LevelSky::DAY,
        weather: Weather::Clear,
        night: false,
        water: false,
    },
    Zone {
        name: "Twilight",
        length: 250.0,
        ground: [90, 80, 120],
        edge: VIOLET,
        sky: LevelSky::DUSK,
        weather: Weather::Clear,
        night: true,
        water: false,
    },
    Zone {
        name: "Snowfield",
        length: 250.0,
        ground: [150, 160, 180],
        edge: SNOW,
        sky: LevelSky::SNOW,
        weather: Weather::Snow,
        night: false,
        water: false,
    },
    Zone {
        name: "Lagoon",
        length: 150.0,
        ground: [200, 170, 110],
        edge: CORAL,
        sky: LevelSky::TROPIC,
        weather: Weather::Clear,
        night: false,
        water: true,
    },
    Zone {
        name: "Storm peak",
        length: 250.0,
        ground: [100, 100, 110],
        edge: SKY,
        sky: LevelSky::STORM,
        weather: Weather::Storm,
        night: false,
        water: false,
    },
];

/// The zone `d` metres into the run (they come round again).
fn zone_at(d: f32) -> usize {
    let lap: f32 = ZONES.iter().map(|z| z.length).sum();
    let mut d = d.max(0.0) % lap;
    for (i, z) in ZONES.iter().enumerate() {
        if d < z.length {
            return i;
        }
        d -= z.length;
    }
    0
}

/// A small seeded random generator, so a course can be made again.
#[derive(Clone)]
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (self.next() % 1_000_000) as f32 / 1_000_000.0 * (hi - lo)
    }

    fn below(&mut self, n: u32) -> u32 {
        (self.next() % n.max(1) as u64) as u32
    }

    fn chance(&mut self, p: f32) -> bool {
        self.range(0.0, 1.0) < p
    }
}

/// The best run: its course and where the character was, sampled.
#[derive(Serialize, Deserialize, Clone, Default)]
pub(super) struct Ghost {
    seed: u64,
    distance: f32,
    character: crate::sonic::Character,
    /// (x from the start, y from the floor, facing, curled up) per sample.
    track: Vec<[f32; 4]>,
}

fn ghost_path() -> Option<std::path::PathBuf> {
    crate::config::config_dir().map(|c| c.join("ghost.json"))
}

impl Ghost {
    fn load() -> Option<Ghost> {
        serde_json::from_str(&std::fs::read_to_string(ghost_path()?).ok()?).ok()
    }

    fn save(&self) -> Result<(), String> {
        let path = ghost_path().ok_or("no config folder")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, serde_json::to_string(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }
}

struct Stretch {
    x1: f32,
    bodies: Vec<RigidBodyHandle>,
}

pub(super) struct Endless {
    /// Where the next stretch starts and where the run started (px).
    next: f32,
    start: f32,
    floor: f32,
    stretches: VecDeque<Stretch>,
    made: u32,
    last_kind: u32,
    /// Furthest point reached (px).
    pub reached: f32,
    seed: u64,
    rng: Rng,
    /// The zone he is in, and the sky the player had before the run.
    zone: usize,
    before: (Weather, bool, bool, f32),
    /// The sky turning from the last zone's (0 → 1).
    sky_from: LevelSky,
    sky_t: f32,
    /// This run so far, and the best one racing him.
    clock: f32,
    track: Vec<[f32; 4]>,
    character: crate::sonic::Character,
    ghost: Option<Ghost>,
    /// The ghost is drawn here first, to show it see-through in one piece.
    ghost_target: Option<RenderTarget>,
    /// Seconds since the run ended.
    over: Option<f32>,
}

impl Endless {
    pub fn shift(&mut self, dx: f32) {
        self.next -= dx;
        self.start -= dx;
        self.reached -= dx;
        for s in &mut self.stretches {
            s.x1 -= dx;
        }
    }

    /// Distance run (m).
    pub fn distance(&self) -> f32 {
        ((self.reached - self.start) / PPM).max(0.0)
    }

    /// Keep this run as the ghost if it is the best one yet (only once):
    /// the distance, or why it could not be saved.
    fn keep_if_best(&mut self) -> Option<Result<f32, String>> {
        let d = self.distance();
        if self.track.is_empty() || d <= 20.0 || self.ghost.as_ref().is_some_and(|g| d <= g.distance) {
            return None;
        }
        let ghost =
            Ghost { seed: self.seed, distance: d, character: self.character, track: std::mem::take(&mut self.track) };
        Some(ghost.save().map(|()| d))
    }
}

impl App {
    /// Start an endless run: the infinite world, an empty floor and Sonic,
    /// on the best run's course (racing its ghost) or a new one.
    pub(super) fn start_endless(&mut self) {
        if !self.infinite() {
            self.set_world_size(4);
        }
        let (aw, ah) = self.arena();
        let floor = ah - WALL_T * PPM;
        let x = aw / 2.0;
        let start = vec2(x, floor - 40.0);
        let ghost = Ghost::load().filter(|g| !g.track.is_empty());
        let seed = ghost.as_ref().map_or_else(|| (get_time() * 1e6) as u64 ^ 0x9e37_79b9_7f4a_7c15, |g| g.seed);
        let before = self
            .endless
            .as_ref()
            .map_or((self.s.weather, self.s.night, self.s.water, self.s.water_level), |e| e.before);
        // One life, no goal: the run ends at the first fall.
        self.start_game(&LevelSpec { name: "Endless run".into(), ..LevelSpec::default() }, start);
        if let Some(g) = &mut self.game {
            g.endless = true;
        }
        self.camera.center = vec2(x, floor - screen_height() * 0.35);
        self.endless = Some(Endless {
            next: x - 500.0,
            start: x,
            floor,
            stretches: VecDeque::new(),
            made: 0,
            last_kind: 0,
            reached: x,
            seed,
            rng: Rng(seed | 1),
            zone: usize::MAX,
            before,
            sky_from: LevelSky::DAY,
            sky_t: 1.0,
            clock: 0.0,
            track: vec![],
            character: crate::sonic::Character::Sonic,
            ghost,
            ghost_target: self.endless.as_mut().and_then(|e| e.ghost_target.take()),
            over: None,
        });
    }

    /// The run is over: keep it if it is the best and give the player
    /// their sky back.
    pub(super) fn end_endless(&mut self) {
        if let Some(mut e) = self.endless.take() {
            (self.s.weather, self.s.night, self.s.water, self.s.water_level) = e.before;
            // The benchmark leaves the player's ghost alone.
            if self.bench.is_none() {
                self.ghost_toast(e.keep_if_best());
            }
        }
    }

    fn ghost_toast(&mut self, kept: Option<Result<f32, String>>) {
        match kept {
            Some(Ok(d)) => self.toasts.success(format!("New best: {d:.0} m  ·  your ghost races you next time")),
            Some(Err(err)) => self.toasts.warn(format!("Could not keep the ghost: {err}")),
            None => {}
        }
    }

    /// Once a step: terrain ahead, clean-up behind, zones, the record and the ghost.
    pub(super) fn update_endless(&mut self, step: f32) {
        let Some(sx) = self.sonic.as_ref().map(|r| r.sonic.pos.x) else {
            self.end_endless();
            return;
        };
        if self.endless.is_none() || !self.infinite() {
            self.end_endless();
            return;
        }
        while self.endless.as_ref().is_some_and(|e| e.next < sx + AHEAD) {
            self.make_stretch();
        }
        self.endless_zone(step);
        let over = self.game.as_ref().is_some_and(|g| g.phase != crate::game::Phase::Playing);
        let Some(run) = &self.sonic else { return };
        let (p, facing, ball) = (run.sonic.pos, run.sonic.facing, run.sonic.is_ball());
        let character = run.sonic.character;
        let Some(e) = &mut self.endless else { return };
        if !over {
            e.reached = e.reached.max(sx);
            e.character = character;
            // Sample the track for the ghost.
            let before = (e.clock * GHOST_RATE) as usize;
            e.clock += step;
            if (e.clock * GHOST_RATE) as usize != before {
                e.track.push([p.x - e.start, p.y - e.floor, facing, if ball { 1.0 } else { 0.0 }]);
            }
        }
        let d = e.distance();
        let mut old = vec![];
        while e.stretches.front().is_some_and(|s| s.x1 < sx - BEHIND) {
            old.extend(e.stretches.pop_front().map(|s| s.bodies).unwrap_or_default());
        }
        if d > self.s.sonic_best {
            self.s.sonic_best = d;
        }
        // The run is over: keep it if it is the best, then go again.
        if over {
            let t = e.over.map_or(0.0, |t| t + step);
            e.over = Some(t);
            let kept = if self.bench.is_none() { e.keep_if_best() } else { None };
            self.ghost_toast(kept);
            if t > RESTART {
                self.clear_objects();
                self.rings.clear();
                self.start_endless();
                return;
            }
        }
        for body in old {
            if let Some(i) = self.objects.iter().position(|o| o.body == body) {
                self.remove_object(i);
            }
        }
        if let Some(g) = &mut self.game {
            g.forget_before(&mut self.world, sx - BEHIND);
        }
        self.rings.retain(|r| r.x > sx - BEHIND);
    }

    /// The sky behind the level: the zone's in an endless run (turning
    /// from the last one's), a bright day otherwise.
    pub(super) fn level_sky(&self) -> LevelSky {
        match &self.endless {
            Some(e) => e.sky_from.mix(&ZONES.get(e.zone).map_or(LevelSky::DAY, |z| z.sky), e.sky_t),
            None => LevelSky::DAY,
        }
    }

    /// Entering a zone: its sky, weather and water.
    fn endless_zone(&mut self, step: f32) {
        let sky = self.level_sky();
        let Some(e) = &mut self.endless else { return };
        e.sky_t = (e.sky_t + step / SKY_TURN).min(1.0);
        let z = zone_at(e.distance());
        if z == e.zone {
            return;
        }
        let first = e.zone == usize::MAX;
        if !first {
            (e.sky_from, e.sky_t) = (sky, 0.0);
        }
        e.zone = z;
        let zone = &ZONES[z];
        self.s.weather = zone.weather;
        self.s.night = zone.night;
        self.s.water = zone.water;
        if zone.water {
            self.s.water_level = 0.17;
        }
        if !first {
            self.toasts.status("zone", format!("{} zone", zone.name));
        }
    }

    fn make_stretch(&mut self) {
        let (_, ah) = self.arena();
        let Some(e) = &mut self.endless else { return };
        let x0 = e.next;
        // A different kind each time (the first is flat).
        let kind = if e.made == 0 {
            0
        } else {
            let mut k = 1 + e.rng.below(5);
            if k == e.last_kind {
                k = k % 5 + 1;
            }
            k
        };
        e.made += 1;
        e.last_kind = kind;
        // Further on, the zone's look and more danger.
        let d = ((x0 - e.start) / PPM).max(0.0);
        let zone = &ZONES[zone_at(d)];
        let level = (d / 150.0) as u32;
        let mut rng = e.rng.clone();
        let mut b = Builder::at(x0, ah);
        let width = build(&mut b, kind, zone, &mut rng);
        let made = crate::scene::instantiate(&b.scene, &mut self.world);
        let bodies = made.objects.iter().map(|o| o.body).collect();
        self.objects.extend(made.objects);
        self.rings.extend(b.scene.rings.iter().map(|&[x, y]| crate::physics::to_screen(x, y)));
        let floor = ah - WALL_T * PPM;
        let first = self.endless.as_ref().is_none_or(|e| e.made <= 2);
        if !first {
            self.endless_dangers(kind, x0, floor, level, &mut rng);
        }
        if let Some(e) = &mut self.endless {
            e.rng = rng;
            e.next = x0 + width;
            e.stretches.push_back(Stretch { x1: x0 + width, bodies });
        }
    }

    /// Badniks, spikes and monitors for a stretch of kind `kind` from `x0`
    /// (px), more of them at higher `level`s.
    fn endless_dangers(&mut self, kind: u32, x0: f32, floor: f32, level: u32, rng: &mut Rng) {
        let Some(g) = &mut self.game else { return };
        let walker = |rng: &mut Rng| {
            if level >= 4 && rng.chance(0.4) {
                EnemyKind::Crab
            } else {
                EnemyKind::Crawler
            }
        };
        let on_floor = |x: f32| vec2(x0 + x, floor - 17.0);
        let count = (1 + level / 2).min(4);
        // Where each kind of stretch has room on the floor (design px).
        let room: &[(f32, f32)] = match kind {
            0 => &[(250.0, 820.0)],
            1 => &[(640.0, 760.0)],
            2 => &[(720.0, 960.0)],
            3 => &[(900.0, 1100.0)],
            4 => &[(320.0, 640.0), (720.0, 1000.0)],
            _ => &[(650.0, 950.0)],
        };
        for _ in 0..count {
            if !rng.chance(0.35 + level as f32 * 0.08) {
                continue;
            }
            let (a, b) = room[rng.below(room.len() as u32) as usize];
            let x = rng.range(a, b);
            if level >= 2 && rng.chance(0.3) {
                g.add_enemy(EnemyKind::Flyer, vec2(x0 + x, floor - rng.range(120.0, 200.0)));
            } else {
                g.add_enemy(walker(rng), on_floor(x));
            }
        }
        // Spikes on open floor.
        if level >= 1 && kind == 0 && rng.chance(0.4 + level as f32 * 0.05) {
            let x = rng.range(420.0, 640.0);
            for k in 0..(1 + level.min(3) / 2) {
                g.add_spikes(&mut self.world, vec2(x0 + x + k as f32 * SPIKES.x, floor - SPIKES.y / 2.0));
            }
        }
        // Now and then a monitor.
        if rng.chance(0.18) {
            let kinds = [
                MonitorKind::Rings,
                MonitorKind::Rings,
                MonitorKind::Shield,
                MonitorKind::Shoes,
                MonitorKind::Invincible,
            ];
            let k = if rng.chance(0.05) { MonitorKind::Life } else { kinds[rng.below(kinds.len() as u32) as usize] };
            let (a, _) = room[0];
            g.add_monitor(&mut self.world, k, vec2(x0 + a - 60.0, floor - MONITOR.y / 2.0));
        }
    }

    /// Where the best run's ghost is now, as a character to draw.
    pub(super) fn ghost_now(&self) -> Option<crate::sonic::Sonic> {
        let e = self.endless.as_ref()?;
        let g = e.ghost.as_ref()?;
        let last = g.track.len().checked_sub(1)?;
        let k = (e.clock * GHOST_RATE).min(last as f32);
        let (i, f) = (k as usize, k.fract());
        let (a, b) = (g.track[i], g.track[(i + 1).min(last)]);
        let pos = vec2(e.start + a[0] + (b[0] - a[0]) * f, e.floor + a[1] + (b[1] - a[1]) * f);
        let mut who = crate::sonic::Sonic::as_character(pos, g.character);
        who.facing = a[2];
        who.jumping = a[3] > 0.5;
        who.grounded = !who.jumping;
        who.gsp = (b[0] - a[0]) * GHOST_RATE / crate::sonic::FPS / crate::sonic::SCALE;
        who.spin = e.clock * 12.0;
        who.anim = e.clock * 8.0;
        Some(who)
    }

    /// Draw the ghost into its texture (before the frame's other drawing).
    pub(super) fn render_endless_ghost(&mut self) {
        let Some(who) = self.ghost_now() else { return };
        let Some(e) = &mut self.endless else { return };
        let rt = e
            .ghost_target
            .get_or_insert_with(|| {
                let rt = render_target(GHOST_PX as u32, GHOST_PX as u32);
                rt.texture.set_filter(FilterMode::Linear);
                rt
            })
            .clone();
        let half = GHOST_PX / 2.0;
        let mut cam = Camera2D::from_display_rect(Rect::new(who.pos.x - half, who.pos.y - half, GHOST_PX, GHOST_PX));
        cam.render_target = Some(rt);
        set_camera(&cam);
        clear_background(BLANK);
        match self.sonic_sheet.get(who.character) {
            Some(sheet) => sheet.draw(&who),
            None => crate::sonic::art::draw(&who),
        }
        set_default_camera();
    }

    /// The best run's ghost, see-through (world pass, before him).
    pub(super) fn draw_endless_ghost(&self) {
        let Some(who) = self.ghost_now() else { return };
        let Some(rt) = self.endless.as_ref().and_then(|e| e.ghost_target.as_ref()) else { return };
        let half = GHOST_PX / 2.0;
        draw_texture_ex(
            &rt.texture,
            who.pos.x - half,
            who.pos.y - half,
            Color::new(0.7, 0.85, 1.0, 0.5),
            DrawTextureParams { dest_size: Some(vec2(GHOST_PX, GHOST_PX)), flip_y: true, ..Default::default() },
        );
        theme::text_bold_centered("BEST", who.pos.x, who.pos.y - 46.0, 11.0, Color::new(1.0, 1.0, 1.0, 0.6));
    }

    /// Distance, record and zone (screen pass, under the counters).
    pub(super) fn draw_endless_hud(&self, top: f32) {
        let Some(e) = &self.endless else { return };
        let zone = ZONES.get(e.zone).map_or("", |z| z.name);
        let line = format!("{:.0} m   ·   best {:.0} m   ·   {zone}", e.distance(), self.s.sonic_best);
        let y = top + 100.0;
        theme::text_bold(&line, 18.0, y + 2.0, 15.0, Color::new(0.0, 0.0, 0.0, 0.6));
        theme::text_bold(&line, 16.0, y, 15.0, Color::new(0.85, 0.95, 1.0, 1.0));
        if let Some(t) = e.over {
            let (sw, sh) = (screen_width(), screen_height());
            let a = (t * 2.0).min(1.0);
            draw_rectangle(0.0, sh * 0.4, sw, 90.0, Color::new(0.0, 0.0, 0.0, 0.6 * a));
            let msg = format!("RUN OVER  ·  {:.0} m", e.distance());
            theme::text_bold_centered(&msg, sw / 2.0, sh * 0.4 + 52.0, 36.0, Color::new(1.0, 0.88, 0.2, a));
        }
    }
}

/// Build one stretch of kind `kind` from design x = 0; returns its width.
fn build(b: &mut Builder, kind: u32, zone: &Zone, rng: &mut Rng) -> f32 {
    let plank = Material { flammable: false, bounce: 0.1, friction: 0.6, ..Material::DEFAULT };
    let ring_row = |b: &mut Builder, x: f32, y: f32, n: usize| {
        for k in 0..n {
            b.ring(x + k as f32 * 36.0, y);
        }
    };
    match kind {
        // Flat ground and a row of rings.
        0 => {
            ring_row(b, 300.0, FLOOR - 40.0, 6);
            900.0
        }
        // Two rolling hills, rings along their tops.
        1 => {
            let h = rng.range(50.0, 95.0);
            for start in [0.0f32, 700.0] {
                let pts: Vec<(f32, f32)> = (0..=35)
                    .map(|i| {
                        let x = i as f32 / 35.0 * 700.0;
                        let y = FLOOR + 5.0 - (h + 5.0) * (std::f32::consts::PI * x / 700.0).sin().powi(2);
                        (start + x, y)
                    })
                    .collect();
                b.stroke(&pts, 16.0, zone.edge, true, plank);
                for k in 0..5 {
                    let x = 220.0 + k as f32 * 65.0;
                    let y = FLOOR - h * (std::f32::consts::PI * x / 700.0).sin().powi(2) - 45.0;
                    b.ring(start + x, y);
                }
            }
            1400.0
        }
        // A dash panel and a loop with rings round the inside.
        2 => {
            let dash = Material { conveyor: 20.0, flammable: false, ..Material::DEFAULT };
            b.stroke(&[(150.0, FLOOR - 3.0), (260.0, FLOOR - 3.0)], 8.0, CORAL, true, dash);
            let r = 110.0;
            b.sonic_loop(500.0, FLOOR - r, r, zone.ground);
            for k in 0..8 {
                let a = (k as f32 / 8.0 * 360.0).to_radians();
                b.ring(500.0 + a.cos() * (r - 42.0), FLOOR - r + a.sin() * (r - 42.0));
            }
            ring_row(b, 140.0, FLOOR - 40.0, 4);
            1000.0
        }
        // A ramp to fly off, rings in the air.
        3 => {
            let top = FLOOR - rng.range(60.0, 100.0);
            b.wall(&[(100.0, FLOOR + 4.0), (380.0, top)], zone.ground);
            for k in 0..7 {
                let t = k as f32 / 6.0;
                b.ring(470.0 + t * 420.0, top - 120.0 - 160.0 * t * (1.0 - t) * 2.0);
            }
            1150.0
        }
        // A spring up to a platform full of rings (or run on underneath).
        4 => {
            let spring = Material { bounce: 1.0, flammable: false, ..Material::DEFAULT };
            b.stroke(&[(150.0, FLOOR - 4.0), (205.0, FLOOR - 4.0)], 10.0, CORAL, true, spring);
            let y = FLOOR - 250.0;
            b.wall(&[(260.0, y), (680.0, y)], WOOD);
            ring_row(b, 300.0, y - 40.0, 10);
            for k in 0..3 {
                b.ring(178.0, FLOOR - 120.0 - k as f32 * 45.0);
            }
            1050.0
        }
        // Crates to bowl over (curl up!), rings behind them.
        _ => {
            let crate_ = Material { breakable: true, strength: 7.0, ..Material::DEFAULT };
            for (col, rows) in [(0, 3), (1, 2), (2, 1)] {
                for row in 0..rows {
                    let x = 420.0 + col as f32 * 50.0;
                    let rgb = if row % 2 == 0 { WOOD } else { SKY };
                    b.shape_with(Shape::Box, rgb, x, FLOOR - 24.0 - row as f32 * 48.0, 46.0, false, crate_);
                }
            }
            ring_row(b, 650.0, FLOOR - 40.0, 6);
            1000.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_follow_the_distance_and_come_round_again() {
        assert_eq!(zone_at(0.0), 0);
        assert_eq!(zone_at(249.0), 0);
        assert_eq!(zone_at(250.0), 1);
        let lap: f32 = ZONES.iter().map(|z| z.length).sum();
        assert_eq!(zone_at(lap + 10.0), 0);
        assert_eq!(zone_at(lap - 1.0), ZONES.len() - 1);
    }

    #[test]
    fn the_sky_turns_from_one_zone_to_the_next() {
        let (a, b) = (ZONES[0].sky, ZONES[1].sky);
        let close = |x: LevelSky, y: LevelSky| (x.top.r - y.top.r).abs() < 1e-5 && (x.sun - y.sun).abs() < 1e-5;
        assert!(close(a.mix(&b, 0.0), a));
        assert!(close(a.mix(&b, 1.0), b));
        assert!(close(a.mix(&b, 7.0), b), "held at the end");
        let half = a.mix(&b, 0.5);
        assert!((half.near.g - (a.near.g + b.near.g) / 2.0).abs() < 1e-5);
    }

    #[test]
    fn a_seed_makes_the_same_course_again() {
        let (mut a, mut b) = (Rng(42), Rng(42));
        let xs: Vec<u32> = (0..20).map(|_| a.below(5)).collect();
        let ys: Vec<u32> = (0..20).map(|_| b.below(5)).collect();
        assert_eq!(xs, ys);
        let mut c = Rng(43);
        assert!((0..20).map(|_| c.below(5)).collect::<Vec<_>>() != xs);
        let r = a.range(2.0, 3.0);
        assert!((2.0..3.0).contains(&r));
    }
}
