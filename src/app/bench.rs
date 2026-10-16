//! `--benchmark`: a few stress scenes, each run for a fixed number of
//! frames at a fixed 60 Hz step, then a report of the frame times and
//! where they went (printed to the terminal), and the app quits without
//! touching the player's settings or files.

use super::*;
use crate::physics::grains::GrainKind;
use rapier2d::prelude::{nalgebra, vector};
use std::time::{Duration, Instant};

/// Frames per scene, and how many of them warm up first (not counted).
const FRAMES: u32 = 420;
const WARMUP: u32 = 60;

struct Scene {
    name: &'static str,
    setup: fn(&mut App),
}

const SCENES: [Scene; 5] = [
    Scene { name: "400 shapes", setup: shapes },
    Scene { name: "Sand and water", setup: sand },
    Scene { name: "Night storm", setup: night_storm },
    Scene { name: "Ragdolls", setup: ragdolls },
    Scene { name: "Sonic endless run", setup: sonic_run },
];

/// One scene's numbers.
struct Report {
    name: &'static str,
    bodies: usize,
    /// Moving bodies (the rest sleep), at the end.
    awake: usize,
    /// Update and draw per frame (ms), and waiting for the screen.
    cpu: Vec<f32>,
    present: Vec<f32>,
    spans: Vec<(&'static str, Duration)>,
}

pub(crate) struct Bench {
    scene: usize,
    frame: u32,
    began: Option<Instant>,
    ended: Option<Instant>,
    cpu: Vec<f32>,
    present: Vec<f32>,
    reports: Vec<Report>,
    pub(crate) done: bool,
}

impl App {
    pub fn start_benchmark(&mut self) {
        self.s.sfx = false;
        self.s.slow_motion = false;
        self.s.time_scale = 1.0;
        self.s.effects = true;
        self.title = None;
        self.bench = Some(Bench {
            scene: 0,
            frame: 0,
            began: None,
            ended: None,
            cpu: vec![],
            present: vec![],
            reports: vec![],
            done: false,
        });
    }

    pub fn benchmark_done(&self) -> bool {
        self.bench.as_ref().is_some_and(|b| b.done)
    }

    /// The frame number in the current benchmark scene.
    pub(super) fn bench_frame(&self) -> Option<u32> {
        self.bench.as_ref().map(|b| b.frame)
    }

    /// Start of a frame: set the scene up on its first frame, note how
    /// long the last one waited for the screen.
    pub(super) fn bench_begin(&mut self) {
        let Some(b) = &mut self.bench else { return };
        let now = Instant::now();
        if let Some(end) = b.ended.take() {
            if b.frame > WARMUP {
                b.present.push(ms(now - end));
            }
        }
        b.began = Some(now);
        if b.frame == 0 {
            let setup = SCENES[b.scene].setup;
            crate::profile::enable(false);
            setup(self);
            println!("benchmark: {}…", SCENES[self.bench.as_ref().map_or(0, |b| b.scene)].name);
        }
    }

    /// End of a frame: count it, and move on to the next scene (or finish).
    pub(super) fn bench_end(&mut self) {
        let bodies = self.world.bodies.len();
        let awake = self.world.islands.active_dynamic_bodies().len();
        let Some(b) = &mut self.bench else { return };
        let now = Instant::now();
        if let Some(start) = b.began.take() {
            if b.frame >= WARMUP {
                b.cpu.push(ms(now - start));
            }
        }
        b.frame += 1;
        if b.frame == WARMUP {
            crate::profile::enable(true);
        }
        b.ended = Some(now);
        if b.frame < FRAMES {
            return;
        }
        b.reports.push(Report {
            name: SCENES[b.scene].name,
            bodies,
            awake,
            cpu: std::mem::take(&mut b.cpu),
            present: std::mem::take(&mut b.present),
            spans: crate::profile::take(),
        });
        crate::profile::enable(false);
        b.scene += 1;
        b.frame = 0;
        b.ended = None;
        if b.scene == SCENES.len() {
            b.done = true;
            print_report(&b.reports, (screen_width(), screen_height()));
        }
    }
}

fn ms(d: Duration) -> f32 {
    d.as_secs_f32() * 1000.0
}

fn print_report(reports: &[Report], (w, h): (f32, f32)) {
    println!();
    println!(
        "{} {} benchmark  ·  {w:.0}×{h:.0}  ·  {} frames per scene at 60 Hz",
        crate::config::APP_NAME,
        crate::config::APP_VERSION,
        FRAMES - WARMUP
    );
    println!();
    println!(
        "{:<20} {:>6} {:>6}   {:>24}   {:>6}   {:>10}",
        "scene", "bodies", "awake", "frame ms  avg / p95 / max", "fps*", "present ms"
    );
    for r in reports {
        let (avg, p95, max) = stats(&r.cpu);
        let (present, _, _) = stats(&r.present);
        println!(
            "{:<20} {:>6} {:>6}   {:>8.2} / {:>5.2} / {:>5.1}   {:>6.0}   {:>10.2}",
            r.name,
            r.bodies,
            r.awake,
            avg,
            p95,
            max,
            1000.0 / avg.max(1e-3),
            present
        );
    }
    println!();
    println!("* frames per second the update and drawing alone would allow");
    println!("  (present: time waiting for the screen and the GPU)");
    println!();
    println!("Where the time goes (ms per frame):");
    for r in reports {
        let n = r.cpu.len().max(1) as f32;
        let parts: Vec<String> = r
            .spans
            .iter()
            .filter(|(_, d)| ms(*d) / n >= 0.01)
            .take(9)
            .map(|(name, d)| format!("{name} {:.2}", ms(*d) / n))
            .collect();
        println!("  {:<20} {}", r.name, parts.join("  ·  "));
    }
}

/// Average, 95th percentile and worst.
fn stats(v: &[f32]) -> (f32, f32, f32) {
    if v.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let mut s = v.to_vec();
    s.sort_by(f32::total_cmp);
    let avg = s.iter().sum::<f32>() / s.len() as f32;
    let p95 = s[((s.len() - 1) as f32 * 0.95) as usize];
    (avg, p95, s[s.len() - 1])
}

/// An empty arena of the usual size, by day, without weather or water.
fn fresh(app: &mut App) {
    app.end_endless();
    app.clear_objects();
    app.rings.clear();
    if app.s.world_size != 1 {
        app.set_world_size(1);
    }
    app.s.weather = crate::weather::Weather::Clear;
    app.s.night = false;
    app.s.water = false;
    app.paused = false;
    let (aw, ah) = app.arena();
    app.camera.fit(vec2(aw, ah), vec2(screen_width(), screen_height()));
}

fn shapes(app: &mut App) {
    fresh(app);
    let (aw, ah) = app.arena();
    // A grid inside the arena, nothing touching yet.
    let mut k = 0usize;
    for row in 0..16 {
        for col in 0..25 {
            let x = aw * (0.08 + 0.84 * (col as f32 + 0.5 * (row % 2) as f32) / 25.0);
            let y = ah * 0.06 + row as f32 * (ah * 0.8 / 16.0);
            let shape = Shape::ALL[k % Shape::ALL.len()];
            let rgb = shapes::PALETTE[k % shapes::PALETTE.len()];
            app.spawn_shape(shape, rgb, 26.0, vec2(x, y), false, Material::DEFAULT);
            k += 1;
        }
    }
}

fn sand(app: &mut App) {
    fresh(app);
    let (aw, ah) = app.arena();
    app.s.water = true;
    app.s.water_level = 0.25;
    for k in 0..3000 {
        let kind = if k % 3 == 0 { GrainKind::Liquid } else { GrainKind::Sand };
        let at = vec2(aw * (0.1 + 0.8 * ((k * 37) % 1000) as f32 / 1000.0), ah * 0.5 - (k / 60) as f32 * 9.0);
        app.grains.add(&mut app.world, kind, at, vector![0.0, 0.0]);
    }
}

fn night_storm(app: &mut App) {
    fresh(app);
    if let Some(i) = crate::library::examples::ALL.iter().position(|e| e.name == "Night lights") {
        app.open_example(i);
    }
    app.s.night = true;
    app.s.weather = crate::weather::Weather::Storm;
    let (aw, ah) = app.arena();
    for k in 0..150 {
        let at = vec2(aw * (0.1 + 0.8 * (k % 30) as f32 / 30.0), ah * 0.1 - (k / 30) as f32 * 50.0);
        app.spawn_shape(Shape::ALL[k % Shape::ALL.len()], (220, 200, 160), 34.0, at, false, Material::DEFAULT);
    }
}

fn ragdolls(app: &mut App) {
    fresh(app);
    let (aw, ah) = app.arena();
    for k in 0..24 {
        app.spawn_ragdoll(vec2(aw * (0.1 + 0.8 * (k % 8) as f32 / 8.0), ah * 0.15 + (k / 8) as f32 * 120.0));
    }
}

fn sonic_run(app: &mut App) {
    fresh(app);
    if let Some(i) = crate::library::examples::ALL.iter().position(|e| e.name == "Endless run") {
        app.open_example(i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_time_stats() {
        let v: Vec<f32> = (1..=100).map(|k| k as f32).collect();
        let (avg, p95, max) = stats(&v);
        assert!((avg - 50.5).abs() < 1e-3);
        assert_eq!(p95, 95.0);
        assert_eq!(max, 100.0);
        assert_eq!(stats(&[]), (0.0, 0.0, 0.0));
    }
}
