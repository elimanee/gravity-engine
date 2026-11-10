//! Sonic's endless run, in the infinite world: a level made from a seed as
//! he goes (see [`procedural`]: hills, slopes, cliffs, pits to get over on
//! bridges, swings, platforms or crumbling ledges, loops, walls, two
//! paths) and cleared away behind him, through themed zones (meadow,
//! twilight, snowfield, lagoon, storm) with more and more badniks and
//! spikes the further he gets. There is nothing under the ground: a pit
//! ends the run. It is played as a level with one life; the best run is
//! kept as a ghost that races him on the same course next time.

use super::*;
use crate::background::LevelSky;
use crate::game::{EnemyKind, LevelSpec, MONITOR, SPIKES};
use crate::library::procedural::{self, Danger, Kind, Look, Rng};
use crate::library::{Builder, CORAL, LEAF, SKY, SNOW, STONE, VIOLET};
use crate::physics::borders::BorderMode;
use crate::weather::Weather;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Terrain kept ready ahead of him, and kept behind him (px).
const AHEAD: f32 = 2600.0;
const BEHIND: f32 = 1800.0;
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

/// Ghosts of courses made the way they are made now (older ones raced
/// on other courses).
const GHOST_VERSION: u32 = 2;

/// The best run: its course and where the character was, sampled.
#[derive(Serialize, Deserialize, Clone, Default)]
pub(super) struct Ghost {
    #[serde(default)]
    version: u32,
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
        let g: Ghost = serde_json::from_str(&crate::storage::read_to_string(&ghost_path()?).ok()?).ok()?;
        (g.version == GHOST_VERSION).then_some(g)
    }

    fn save(&self) -> Result<(), String> {
        let path = ghost_path().ok_or("no config folder")?;
        crate::storage::write(&path, serde_json::to_string(self).map_err(|e| e.to_string())?)
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
    last_kind: Kind,
    /// The ground's top where the next stretch starts (design px).
    ground: f32,
    /// Furthest point reached (px).
    pub reached: f32,
    seed: u64,
    rng: Rng,
    /// The zone he is in, and the sky the player had before the run.
    zone: usize,
    before: (Weather, bool, bool, f32, BorderMode),
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
        let track = std::mem::take(&mut self.track);
        let ghost = Ghost { version: GHOST_VERSION, seed: self.seed, distance: d, character: self.character, track };
        Some(ghost.save().map(|()| d))
    }
}

impl App {
    pub(super) fn start_endless(&mut self) {
        if !self.infinite() {
            self.set_world_size(4);
        }
        let (aw, ah) = self.arena();
        // The ground starts at the generator's base line.
        let floor = Builder::at(0.0, ah).to_world(0.0, procedural::BASE).y;
        let x = aw / 2.0;
        let start = vec2(x, floor - 40.0);
        let ghost = Ghost::load().filter(|g| !g.track.is_empty());
        let seed = ghost.as_ref().map_or_else(|| (get_time() * 1e6) as u64 ^ 0x9e37_79b9_7f4a_7c15, |g| g.seed);
        let before = self
            .endless
            .as_ref()
            .map_or((self.s.weather, self.s.night, self.s.water, self.s.water_level, self.s.border), |e| e.before);
        // Nothing under the ground: a pit ends the run.
        self.s.border = BorderMode::Open;
        self.world.set_border(BorderMode::Open);
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
            last_kind: Kind::Flat,
            ground: procedural::BASE,
            reached: x,
            seed,
            rng: Rng::new(seed),
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
        // Ground under him before he can fall.
        while self.endless.as_ref().is_some_and(|e| e.next < x + AHEAD) {
            self.make_stretch();
        }
    }

    /// The run is over: keep it if it is the best and give the player
    /// their sky back.
    pub(super) fn end_endless(&mut self) {
        if let Some(mut e) = self.endless.take() {
            let border;
            (self.s.weather, self.s.night, self.s.water, self.s.water_level, border) = e.before;
            self.s.border = border;
            self.world.set_border(border);
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

    /// The song of the endless run's zone.
    pub(super) fn endless_song(&self) -> Option<crate::audio::music::Song> {
        use crate::audio::music::Song;
        let e = self.endless.as_ref()?;
        Some(match e.zone {
            1 => Song::Twilight,
            2 => Song::Snowfield,
            3 => Song::Lagoon,
            4 => Song::Storm,
            _ => Song::Meadow,
        })
    }

    /// The sky behind the level: the zone's in an endless run (turning
    /// from the last one's) or in an act, a bright day otherwise.
    pub(super) fn level_sky(&self) -> LevelSky {
        use crate::library::acts::{Zone, ALL};
        match &self.endless {
            Some(e) => e.sky_from.mix(&ZONES.get(e.zone).map_or(LevelSky::DAY, |z| z.sky), e.sky_t),
            None => match self.act.and_then(|i| ALL.get(i)).map(|a| a.zone) {
                Some(Zone::Harbour) => LevelSky::HARBOUR,
                Some(Zone::Halloween) => LevelSky::HALLOWEEN,
                _ => LevelSky::DAY,
            },
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
        let (x0, ground, last) = (e.next, e.ground, e.last_kind);
        let first = e.made == 0;
        let quiet = e.made <= 2;
        e.made += 1;
        // Further on, the zone's look and more danger.
        let d = ((x0 - e.start) / PPM).max(0.0);
        let zone = &ZONES[zone_at(d)];
        let look = Look { ground: zone.ground, edge: zone.edge };
        let level = (d / 150.0) as u32;
        let mut rng = e.rng.clone();
        let mut b = Builder::at(x0, ah);
        let (kind, piece) = if first {
            (Kind::Flat, procedural::start(&mut b, 0.0, ground, 1400.0, &look))
        } else {
            let kind = procedural::pick(&mut rng, last, ground);
            (kind, procedural::piece(&mut b, kind, 0.0, ground, &look, &mut rng))
        };
        let dangers = if quiet { vec![] } else { procedural::dangers(&piece.floors, level, &mut rng) };
        // Its gimmicks are the game's (bridges, swings, walls…).
        if let (Some(g), Some(spec)) = (self.game.as_mut(), b.scene.game.as_ref()) {
            g.add_things(&mut self.world, spec);
        }
        let made = crate::scene::instantiate(&b.scene, &mut self.world);
        let bodies = made.objects.iter().map(|o| o.body).collect();
        self.objects.extend(made.objects);
        self.rings.extend(b.scene.rings.iter().map(|&[x, y]| crate::physics::to_screen(x, y)));
        for d in dangers {
            self.endless_danger(&b, d);
        }
        if let Some(e) = &mut self.endless {
            e.rng = rng;
            e.last_kind = kind;
            e.ground = piece.end;
            e.next = x0 + piece.width;
            e.stretches.push_back(Stretch { x1: x0 + piece.width, bodies });
        }
    }

    /// A badnik, spikes or a monitor on a stretch made with `b`.
    fn endless_danger(&mut self, b: &Builder, d: Danger) {
        let Some(g) = &mut self.game else { return };
        match d {
            Danger::Walker(kind, x, y) => g.add_enemy(kind, b.to_world(x, y - 17.0)),
            Danger::Flyer(x, y) => g.add_enemy(EnemyKind::Flyer, b.to_world(x, y)),
            Danger::Spikes(x, y) => g.add_spikes(&mut self.world, b.to_world(x, y - SPIKES.y / 2.0)),
            Danger::Monitor(kind, x, y) => g.add_monitor(&mut self.world, kind, b.to_world(x, y - MONITOR.y / 2.0)),
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
    fn nothing_on_the_course_falls_or_rolls_about() {
        let zone = &ZONES[0];
        let look = Look { ground: zone.ground, edge: zone.edge };
        let mut rng = Rng::new(7);
        for kind in Kind::ALL {
            let mut b = Builder::at(0.0, 1440.0);
            procedural::piece(&mut b, kind, 0.0, procedural::BASE, &look, &mut rng);
            assert!(b.scene.objects.iter().all(|o| o.pinned), "{kind:?} has something loose");
        }
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
