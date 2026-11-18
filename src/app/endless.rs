//! Sonic's endless run, in the infinite world: a level made as he goes
//! (see [`procedural`]: ground generated without end, with pits to get
//! over in all sorts of ways, cliffs, loops, walls and routes on two
//! levels), a new one each run, cleared away behind him, through themed
//! zones (meadow, twilight, snowfield, lagoon, storm) with more and more
//! badniks and spikes the further he gets. There is nothing under the
//! ground: a pit ends the run. It is played as a level with one life; a
//! flag on the course marks how far the best run went.

use super::*;
use crate::background::LevelSky;
use crate::game::{EnemyKind, LevelSpec, MONITOR, SPIKES};
use crate::library::procedural::{self, Course, Crossing, Danger, Look};
use crate::library::{Builder, CORAL, LEAF, SKY, SNOW, STONE, VIOLET};
use crate::physics::borders::BorderMode;
use crate::weather::Weather;
use std::collections::VecDeque;

/// Terrain kept ready ahead of him, and kept behind him (px).
const AHEAD: f32 = 2600.0;
const BEHIND: f32 = 1800.0;
/// Seconds between the end of a run and the next.
const RESTART: f32 = 3.0;
/// Seconds the sky takes to turn into the next zone's.
const SKY_TURN: f32 = 2.5;

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

struct Stretch {
    x1: f32,
    bodies: Vec<RigidBodyHandle>,
}

pub(super) struct Endless {
    /// Where the next stretch starts and where the run started (px).
    next: f32,
    start: f32,
    stretches: VecDeque<Stretch>,
    made: u32,
    /// What lays the ground ahead (a new one each run).
    course: Course,
    /// Furthest point reached (px).
    pub reached: f32,
    /// The zone he is in, and the sky the player had before the run.
    zone: usize,
    before: (Weather, bool, bool, f32, BorderMode),
    /// The sky turning from the last zone's (0 → 1).
    sky_from: LevelSky,
    sky_t: f32,
    /// Lanterns at the edges of the pits (px): at night they show where
    /// the ground stops.
    lanterns: VecDeque<Vec2>,
    /// The best run before this one (m), and whether he has got past it.
    record: f32,
    passed: bool,
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
        for l in &mut self.lanterns {
            l.x -= dx;
        }
    }

    /// Distance run (m).
    pub fn distance(&self) -> f32 {
        ((self.reached - self.start) / PPM).max(0.0)
    }

    /// Where the best run ended, on this course (px), once there is one
    /// worth showing.
    pub fn record_x(&self) -> Option<f32> {
        (self.record >= 20.0).then_some(self.start + self.record * PPM)
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
        // A new course every run (the benchmark's is always the same).
        let seed = match self.bench {
            Some(_) => 0x5eed,
            None => ((get_time() * 1e6) as u64 ^ 0x9e37_79b9_7f4a_7c15).wrapping_mul(0x2545_f491_4f6c_dd1d),
        };
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
            stretches: VecDeque::new(),
            made: 0,
            course: Course::new(seed),
            reached: x,
            zone: usize::MAX,
            before,
            sky_from: LevelSky::DAY,
            sky_t: 1.0,
            lanterns: VecDeque::new(),
            record: self.s.sonic_best,
            passed: false,
            over: None,
        });
        // Ground under him before he can fall.
        while self.endless.as_ref().is_some_and(|e| e.next < x + AHEAD) {
            self.make_stretch();
        }
    }

    /// The run is over: give the player their sky back.
    pub(super) fn end_endless(&mut self) {
        if let Some(e) = self.endless.take() {
            let border;
            (self.s.weather, self.s.night, self.s.water, self.s.water_level, border) = e.before;
            self.s.border = border;
            self.world.set_border(border);
        }
    }

    /// Once a step: terrain ahead, clean-up behind, zones and the record.
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
        let quiet = self.bench.is_some();
        let Some(e) = &mut self.endless else { return };
        if !over {
            e.reached = e.reached.max(sx);
        }
        let d = e.distance();
        // Past the flag: a new record on the way.
        let passed = !e.passed && !over && e.record >= 20.0 && d > e.record;
        e.passed |= passed;
        let ended = over && e.over.is_none();
        let mut old = vec![];
        while e.stretches.front().is_some_and(|s| s.x1 < sx - BEHIND) {
            old.extend(e.stretches.pop_front().map(|s| s.bodies).unwrap_or_default());
        }
        if d > self.s.sonic_best {
            self.s.sonic_best = d;
        }
        let record = e.record;
        // The run is over: then go again, on a new course.
        if over {
            let t = e.over.map_or(0.0, |t| t + step);
            e.over = Some(t);
            if ended && !quiet && d > record.max(20.0) {
                self.toasts.success(format!("New record: {d:.0} m"));
            }
            if t > RESTART {
                self.clear_objects();
                self.rings.clear();
                self.start_endless();
                return;
            }
        }
        if passed && !quiet {
            self.toasts.success(format!("Past your record of {record:.0} m!"));
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
        if let Some(e) = &mut self.endless {
            e.lanterns.retain(|l| l.x > sx - BEHIND);
        }
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

    /// How much the sky is a bright day (where the zone's sheet has its
    /// backdrop drawn): 1 in the meadows, 0 elsewhere, between while it turns.
    pub(super) fn day_weight(&self) -> f32 {
        let day = |s: &LevelSky| if *s == LevelSky::DAY { 1.0 } else { 0.0 };
        match &self.endless {
            Some(e) => {
                let to = ZONES.get(e.zone).map_or(1.0, |z| day(&z.sky));
                day(&e.sky_from) * (1.0 - e.sky_t) + to * e.sky_t
            }
            None => day(&self.level_sky()),
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
        let first = e.made == 0;
        let quiet = e.made <= 2;
        e.made += 1;
        // Further on, the zone's look and more danger.
        let d = ((x0 - e.start) / PPM).max(0.0);
        let zone = &ZONES[zone_at(d)];
        let look = Look { ground: zone.ground, edge: zone.edge };
        let level = (d / 150.0) as u32;
        let mut course = e.course.clone();
        let mut b = Builder::at(x0, ah);
        let section = match first {
            true => course.start(&mut b, 0.0, 1400.0, &look),
            false => course.section(&mut b, 0.0, &look, level),
        };
        // A lantern either side of each pit, on the ground.
        let lanterns: Vec<Vec2> = section
            .pits
            .iter()
            .filter(|p| p.how != Crossing::Ledge)
            .flat_map(|p| [b.to_world(p.x0 - 16.0, p.y0), b.to_world(p.x1 + 16.0, p.y1)])
            .collect();
        // Its own dangers (spikes in a dip), and badniks once under way.
        let mut dangers = section.dangers.clone();
        if !quiet {
            dangers.extend(course.dangers(&section.floors, level));
        }
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
            e.course = course;
            e.next = x0 + section.width;
            e.stretches.push_back(Stretch { x1: x0 + section.width, bodies });
            e.lanterns.extend(lanterns);
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

    /// The flag where the best run ended, in the world (on the ground
    /// line of the view's top third), if it is in view.
    pub(super) fn record_flag(&self) -> Option<Vec2> {
        let x = self.endless.as_ref()?.record_x()?;
        let seen = self.view().visible();
        (x > seen.x - 100.0 && x < seen.right() + 100.0).then(|| vec2(x, seen.y + seen.h * 0.22))
    }

    /// The lanterns at the pits' edges in view, at night (where they stand).
    pub(super) fn endless_lanterns(&self) -> Vec<Vec2> {
        let Some(e) = self.endless.as_ref().filter(|_| self.s.night) else { return vec![] };
        let seen = self.view().visible();
        e.lanterns.iter().copied().filter(|l| l.x > seen.x - 200.0 && l.x < seen.right() + 200.0).collect()
    }

    /// Lanterns at the pits' edges (world pass, at night).
    pub(super) fn draw_endless_lanterns(&self) {
        let t = get_time() as f32;
        for l in self.endless_lanterns() {
            let flicker = 0.85 + 0.15 * (t * 9.0 + l.x * 0.05).sin();
            draw_line(l.x, l.y, l.x, l.y - 34.0, 3.0, Color::new(0.25, 0.2, 0.18, 1.0));
            draw_rectangle(l.x - 6.0, l.y - 46.0, 12.0, 14.0, Color::new(0.2, 0.15, 0.12, 1.0));
            draw_rectangle(l.x - 4.0, l.y - 44.0, 8.0, 10.0, Color::new(1.0, 0.75 * flicker, 0.3, 1.0));
        }
    }

    /// The best run's mark: a line down the view and a flag with the
    /// distance (world pass, before him).
    pub(super) fn draw_endless_record(&self) {
        let (Some(flag), Some(e)) = (self.record_flag(), self.endless.as_ref()) else { return };
        let seen = self.view().visible();
        let gold = Color::new(1.0, 0.84, 0.25, 1.0);
        let faint = Color::new(1.0, 0.84, 0.25, 0.45);
        let mut y = flag.y;
        while y < seen.bottom() {
            draw_line(flag.x, y, flag.x, (y + 18.0).min(seen.bottom()), 3.0, faint);
            y += 30.0;
        }
        // The pole and the flag, with the distance on it.
        draw_line(flag.x, flag.y - 70.0, flag.x, flag.y, 4.0, gold);
        let label = format!("RECORD {:.0} m", e.record);
        let w = theme::measure_bold(&label, 14.0) + 20.0;
        let r = Rect::new(flag.x + 2.0, flag.y - 70.0, w, 28.0);
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.08, 0.06, 0.02, 0.75));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, gold);
        theme::text_bold(&label, r.x + 10.0, r.y + 19.0, 14.0, gold);
    }

    /// Distance, record and zone (screen pass, under the counters).
    pub(super) fn draw_endless_hud(&self, top: f32) {
        let Some(e) = &self.endless else { return };
        let zone = ZONES.get(e.zone).map_or("", |z| z.name);
        let line = format!("{:.0} m   ·   record {:.0} m   ·   {zone}", e.distance(), self.s.sonic_best);
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
        let mut course = Course::new(7);
        let mut seen = std::collections::HashSet::new();
        for i in 0..200 {
            let mut b = Builder::at(0.0, 1440.0);
            let s = course.section(&mut b, 0.0, &look, i / 10);
            seen.extend(s.feature);
            assert!(b.scene.objects.iter().all(|o| o.pinned), "{:?} has something loose", s.feature);
        }
        assert_eq!(seen.len(), procedural::Feature::ALL.len(), "every feature was tried");
    }

    #[test]
    fn the_record_is_marked_where_the_best_run_ended() {
        let e = Endless {
            next: 0.0,
            start: 1000.0,
            stretches: VecDeque::new(),
            made: 0,
            course: Course::new(1),
            reached: 1000.0,
            zone: 0,
            before: (Weather::Clear, false, false, 0.0, BorderMode::Open),
            sky_from: LevelSky::DAY,
            sky_t: 1.0,
            lanterns: VecDeque::from([vec2(1500.0, 600.0)]),
            record: 150.0,
            passed: false,
            over: None,
        };
        assert_eq!(e.record_x(), Some(1000.0 + 150.0 * PPM));
        // Everything moves back with the world.
        let mut moved = Endless { lanterns: e.lanterns.clone(), course: e.course.clone(), ..e };
        moved.shift(400.0);
        assert_eq!(moved.record_x(), Some(600.0 + 150.0 * PPM));
        assert_eq!(moved.lanterns[0], vec2(1100.0, 600.0));
        let e = moved;
        let short = Endless { record: 12.0, ..e };
        assert_eq!(short.record_x(), None, "not for a run that hardly began");
    }
}
