//! Sonic's acts: levels with badniks, hazards, item monitors, star posts,
//! a goal and, at the end of a zone, a boss. They are played in the
//! infinite world with nothing under the ground: a pit is a lost life.

use super::{Builder, LEAF, SKY, SNOW, STONE, WOOD};
use crate::game::{EnemyKind, MonitorKind};
use crate::physics::borders::BorderMode;
use crate::scene::SceneFile;

/// One act of the campaign.
pub struct Act {
    pub name: &'static str,
    pub about: &'static str,
    pub build: fn(&mut Builder),
}

impl Act {
    /// The act's scene, starting at world x `x0` (px) in a world `ah` px high.
    pub fn scene(&self, x0: f32, ah: f32) -> SceneFile {
        let mut b = Builder::at(x0, ah);
        b.scene.border = BorderMode::Open;
        (self.build)(&mut b);
        b.scene.game.get_or_insert_with(Default::default).name = self.name.to_string();
        b.scene
    }
}

pub const ALL: &[Act] = &[
    Act { name: "Meadow Act 1", about: "Run, roll through badniks, mind the spikes", build: meadow_1 },
    Act { name: "Meadow Act 2", about: "Platforms over pits, flyers and crabs", build: meadow_2 },
    Act { name: "Meadow Act 3", about: "The boss waits at the end", build: meadow_3 },
];

/// The ground's top in design px (the world's bottom is at 720).
const G: f32 = 560.0;
const DIRT: [u8; 3] = [150, 98, 62];
const ROCK: [u8; 3] = [112, 104, 128];

fn rings(b: &mut Builder, x: f32, y: f32, n: usize) {
    for k in 0..n {
        b.ring(x + k as f32 * 36.0, y);
    }
}

fn arc(b: &mut Builder, x: f32, y: f32, n: usize, w: f32, h: f32) {
    for k in 0..n {
        let t = k as f32 / (n - 1).max(1) as f32;
        b.ring(x + t * w, y - h * 4.0 * t * (1.0 - t));
    }
}

fn meadow_1(b: &mut Builder) {
    b.sonic(140.0, G - 40.0);
    b.ground(-400.0, 1500.0, G, DIRT, LEAF);
    rings(b, 320.0, G - 40.0, 5);
    b.monitor(MonitorKind::Rings, 620.0, G);
    b.walker(EnemyKind::Crawler, 900.0, G);
    b.walker(EnemyKind::Crawler, 1250.0, G);
    // Rolling hills.
    b.hill(1500.0, 2300.0, G, 90.0, DIRT, LEAF);
    arc(b, 1700.0, G - 150.0, 7, 400.0, 30.0);
    b.ground(2300.0, 2700.0, G, DIRT, LEAF);
    b.post(2340.0, G);
    b.monitor(MonitorKind::Shield, 2420.0, G);
    b.spikes(2520.0, G, 2);
    // The first pit.
    arc(b, 2730.0, G - 80.0, 5, 180.0, 90.0);
    b.ground(2940.0, 4700.0, G, DIRT, LEAF);
    b.dash_panel(3200.0, 3320.0, G);
    b.post(4050.0, G);
    let r = 110.0;
    b.sonic_loop(3600.0, G - r, r, STONE);
    for k in 0..8 {
        let a = (k as f32 / 8.0 * 360.0).to_radians();
        b.ring(3600.0 + a.cos() * (r - 42.0), G - r + a.sin() * (r - 42.0));
    }
    b.enemy(EnemyKind::Flyer, 4000.0, G - 150.0);
    b.walker(EnemyKind::Crab, 4250.0, G);
    // A spring up to a ledge with an invincibility monitor.
    b.spring(4420.0, G);
    b.platform(4480.0, 4680.0, G - 230.0, WOOD);
    b.monitor(MonitorKind::Invincible, 4600.0, G - 238.0);
    rings(b, 4500.0, G - 280.0, 3);
    // The second pit, then the goal.
    b.ground(4900.0, 6200.0, G, DIRT, LEAF);
    b.walker(EnemyKind::Crawler, 5150.0, G);
    b.walker(EnemyKind::Crawler, 5350.0, G);
    rings(b, 5450.0, G - 40.0, 5);
    b.goal(5800.0, G);
}

fn meadow_2(b: &mut Builder) {
    b.sonic(140.0, G - 40.0);
    b.ground(-400.0, 1000.0, G, DIRT, LEAF);
    b.monitor(MonitorKind::Shoes, 420.0, G);
    rings(b, 560.0, G - 40.0, 6);
    // Steps over a long pit.
    for (k, (x0, x1, y)) in
        [(1150.0, 1390.0, G - 60.0), (1500.0, 1700.0, G - 140.0), (1820.0, 2050.0, G - 60.0)].into_iter().enumerate()
    {
        b.platform(x0, x1, y, if k % 2 == 0 { WOOD } else { SKY });
        rings(b, x0 + 40.0, y - 40.0, 4);
    }
    b.enemy(EnemyKind::Flyer, 1600.0, G - 300.0);
    b.ground(2200.0, 3600.0, G, DIRT, LEAF);
    b.post(2300.0, G);
    b.walker(EnemyKind::Crab, 2600.0, G);
    b.spikes(2850.0, G, 3);
    b.monitor(MonitorKind::Life, 3150.0, G);
    b.walker(EnemyKind::Crawler, 3400.0, G);
    // Up a cliff with a spring.
    b.spring(3520.0, G);
    b.ground(3600.0, 4600.0, G - 200.0, ROCK, LEAF);
    b.enemy(EnemyKind::Flyer, 3950.0, G - 360.0);
    b.walker(EnemyKind::Crab, 4250.0, G - 200.0);
    rings(b, 3700.0, G - 240.0, 6);
    // A dash down a slope and over a last pit.
    b.ground(4600.0, 5400.0, G, DIRT, LEAF);
    b.wall(&[(4600.0, G - 200.0), (5000.0, G - 6.0)], ROCK);
    b.dash_panel(5100.0, 5220.0, G);
    arc(b, 5450.0, G - 60.0, 6, 260.0, 120.0);
    b.ground(5780.0, 7000.0, G, DIRT, LEAF);
    b.post(5900.0, G);
    b.walker(EnemyKind::Crawler, 6150.0, G);
    b.spikes(6350.0, G, 2);
    b.monitor(MonitorKind::Rings, 6560.0, G);
    b.goal(6800.0, G);
}

fn meadow_3(b: &mut Builder) {
    b.sonic(140.0, G - 40.0);
    b.ground(-400.0, 1800.0, G, DIRT, LEAF);
    rings(b, 300.0, G - 40.0, 8);
    b.monitor(MonitorKind::Shield, 800.0, G);
    b.monitor(MonitorKind::Rings, 900.0, G);
    b.walker(EnemyKind::Crawler, 1200.0, G);
    b.enemy(EnemyKind::Flyer, 1500.0, G - 160.0);
    b.post(1700.0, G);
    // The boss's arena (it closes once Sonic is in).
    b.ground(1800.0, 3300.0, G, ROCK, SNOW);
    rings(b, 1950.0, G - 40.0, 4);
    b.boss(2500.0, G - 160.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_act_has_a_start_and_an_end() {
        for a in ALL {
            let sc = a.scene(30_000.0, 1440.0);
            let g = sc.game.as_ref().expect("an act is a level");
            assert!(sc.sonic.is_some(), "{} has a start", a.name);
            assert!(g.goal.is_some() || g.boss.is_some(), "{} has an end", a.name);
            assert_eq!(g.name, a.name);
            assert_eq!(sc.border, BorderMode::Open);
            let json = serde_json::to_string(&sc).unwrap();
            assert_eq!(serde_json::from_str::<SceneFile>(&json).unwrap().game, sc.game, "{} round-trips", a.name);
        }
    }
}
