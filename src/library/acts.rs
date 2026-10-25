//! Sonic's acts: levels with badniks, hazards, item monitors, star posts,
//! a goal and, at the end of a zone, a boss. They are played in the
//! infinite world with nothing under the ground: a pit is a lost life.
//!
//! Three zones: Meadow (green hills), Harbour (docks over the sea, a
//! flooded tunnel, the anchor crane) and, for Halloween, a graveyard at
//! night where Sonic.exe hunts the player.

use super::{Builder, LEAF, SKY, SNOW, STONE, WOOD};
use crate::game::{BossKind, EnemyKind, MonitorKind};
use crate::physics::borders::BorderMode;
use crate::scene::SceneFile;

/// The campaign's zones (each has its own sky and song).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Meadow,
    Harbour,
    Halloween,
}

/// One act of the campaign.
pub struct Act {
    pub name: &'static str,
    pub about: &'static str,
    pub zone: Zone,
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
    Act {
        name: "Meadow Act 1",
        about: "Run, roll through badniks, mind the spikes",
        zone: Zone::Meadow,
        build: meadow_1,
    },
    Act { name: "Meadow Act 2", about: "Platforms over pits, flyers and crabs", zone: Zone::Meadow, build: meadow_2 },
    Act { name: "Meadow Act 3", about: "The boss waits at the end", zone: Zone::Meadow, build: meadow_3 },
    Act {
        name: "Harbour Act 1",
        about: "Along the docks: fish leap from the sea, gulls dive",
        zone: Zone::Harbour,
        build: harbour_1,
    },
    Act {
        name: "Harbour Act 2",
        about: "A flooded tunnel: mind your air, breathe the big bubbles",
        zone: Zone::Harbour,
        build: harbour_2,
    },
    Act { name: "Harbour Act 3", about: "The anchor crane guards the pier", zone: Zone::Harbour, build: harbour_3 },
    Act {
        name: "Halloween Night",
        about: "Sonic.exe is after you: keep running, don't let him catch you",
        zone: Zone::Halloween,
        build: halloween,
    },
];

/// The ground's top in design px (the world's bottom is at 720).
const G: f32 = 560.0;
const DIRT: [u8; 3] = [150, 98, 62];
const ROCK: [u8; 3] = [112, 104, 128];
/// Harbour: the sea's surface and its floor (design y), the docks' planks,
/// the stone quays and the sand.
const SEA: f32 = 600.0;
const SEABED: f32 = 700.0;
const PLANK: [u8; 3] = [150, 104, 70];
const PLANK_EDGE: [u8; 3] = [200, 152, 100];
const QUAY: [u8; 3] = [118, 116, 136];
const QUAY_EDGE: [u8; 3] = [196, 194, 210];
const SAND: [u8; 3] = [210, 186, 128];
/// Halloween: graveyard soil, dead grass, gravestones.
const SOIL: [u8; 3] = [70, 54, 86];
const DEAD_GRASS: [u8; 3] = [96, 112, 72];
const TOMB: [u8; 3] = [136, 136, 152];

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
    rings(b, 1000.0, G - 40.0, 5);
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
    rings(b, 3000.0, G - 40.0, 5);
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
    arc(b, 4730.0, G - 80.0, 5, 150.0, 90.0);
    b.ground(4900.0, 6200.0, G, DIRT, LEAF);
    b.monitor(MonitorKind::Rings, 5000.0, G);
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
    rings(b, 2380.0, G - 40.0, 5);
    b.monitor(MonitorKind::Rings, 2560.0, G);
    b.walker(EnemyKind::Crab, 2700.0, G);
    b.spikes(2850.0, G, 3);
    b.monitor(MonitorKind::Life, 3150.0, G);
    b.walker(EnemyKind::Crawler, 3400.0, G);
    // Up a cliff with a spring.
    b.spring(3520.0, G);
    b.ground(3600.0, 4600.0, G - 200.0, ROCK, LEAF);
    b.enemy(EnemyKind::Flyer, 3950.0, G - 360.0);
    b.walker(EnemyKind::Crab, 4250.0, G - 200.0);
    rings(b, 3700.0, G - 240.0, 6);
    rings(b, 4350.0, G - 240.0, 5);
    // A dash down a slope and over a last pit.
    b.ground(4600.0, 5400.0, G, DIRT, LEAF);
    b.wall(&[(4600.0, G - 200.0), (5000.0, G - 6.0)], ROCK);
    b.dash_panel(5100.0, 5220.0, G);
    arc(b, 5450.0, G - 60.0, 6, 260.0, 120.0);
    b.ground(5780.0, 7000.0, G, DIRT, LEAF);
    b.post(5900.0, G);
    rings(b, 5960.0, G - 40.0, 4);
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
    rings(b, 560.0, G - 40.0, 6);
    rings(b, 1000.0, G - 40.0, 5);
    b.walker(EnemyKind::Crawler, 1200.0, G);
    arc(b, 1300.0, G - 100.0, 6, 220.0, 60.0);
    b.enemy(EnemyKind::Flyer, 1500.0, G - 160.0);
    b.monitor(MonitorKind::Rings, 1600.0, G);
    b.post(1700.0, G);
    // The boss's arena (it closes once Sonic is in).
    b.ground(1800.0, 3300.0, G, ROCK, SNOW);
    rings(b, 1950.0, G - 40.0, 4);
    rings(b, 2650.0, G - 40.0, 5);
    rings(b, 2250.0, G - 40.0, 6);
    b.boss(2500.0, G - 160.0);
}

fn harbour_1(b: &mut Builder) {
    b.water_to(SEA, 1.0);
    b.sonic(140.0, G - 40.0);
    b.ground(-400.0, 1100.0, G, QUAY, QUAY_EDGE);
    rings(b, 300.0, G - 40.0, 6);
    b.monitor(MonitorKind::Rings, 620.0, G);
    b.walker(EnemyKind::Crab, 850.0, G);
    // The sea between the piers: fish leap from it; fall in, jump out.
    b.ground(1100.0, 1450.0, SEABED, SAND, SAND);
    b.enemy(EnemyKind::Fish, 1280.0, SEA + 50.0);
    arc(b, 1120.0, G - 70.0, 7, 320.0, 120.0);
    b.ground(1450.0, 2700.0, G, PLANK, PLANK_EDGE);
    rings(b, 1550.0, G - 40.0, 5);
    b.enemy(EnemyKind::Gull, 1700.0, G - 300.0);
    b.monitor(MonitorKind::Shield, 2050.0, G);
    b.post(2200.0, G);
    rings(b, 2280.0, G - 40.0, 5);
    b.walker(EnemyKind::Crawler, 2550.0, G);
    b.ground(2700.0, 3150.0, SEABED, SAND, SAND);
    b.enemy(EnemyKind::Fish, 2800.0, SEA + 50.0);
    b.enemy(EnemyKind::Fish, 3020.0, SEA + 50.0);
    b.platform(2860.0, 2980.0, G - 30.0, PLANK);
    rings(b, 2875.0, G - 70.0, 3);
    b.ground(3150.0, 4800.0, G, QUAY, QUAY_EDGE);
    b.dash_panel(3250.0, 3370.0, G);
    let r = 110.0;
    b.sonic_loop(3700.0, G - r, r, STONE);
    for k in 0..8 {
        let a = (k as f32 / 8.0 * 360.0).to_radians();
        b.ring(3700.0 + a.cos() * (r - 42.0), G - r + a.sin() * (r - 42.0));
    }
    b.enemy(EnemyKind::Gull, 4150.0, G - 320.0);
    b.enemy(EnemyKind::Puffer, 4380.0, G - 80.0);
    b.spring(4560.0, G);
    b.platform(4600.0, 4790.0, G - 230.0, PLANK);
    b.monitor(MonitorKind::Invincible, 4700.0, G - 238.0);
    rings(b, 4620.0, G - 280.0, 4);
    b.ground(4800.0, 5100.0, SEABED, SAND, SAND);
    b.enemy(EnemyKind::Fish, 4950.0, SEA + 50.0);
    arc(b, 4820.0, G - 70.0, 6, 260.0, 110.0);
    b.ground(5100.0, 6600.0, G, PLANK, PLANK_EDGE);
    b.walker(EnemyKind::Crab, 5350.0, G);
    rings(b, 5450.0, G - 40.0, 6);
    b.enemy(EnemyKind::Gull, 5700.0, G - 300.0);
    b.monitor(MonitorKind::Rings, 5900.0, G);
    b.goal(6250.0, G);
}

fn harbour_2(b: &mut Builder) {
    b.water_to(SEA, 1.0);
    b.sonic(140.0, G - 40.0);
    b.ground(-400.0, 950.0, G, QUAY, QUAY_EDGE);
    rings(b, 280.0, G - 40.0, 6);
    b.monitor(MonitorKind::Rings, 600.0, G);
    b.post(850.0, G);
    b.enemy(EnemyKind::Gull, 2600.0, G - 320.0);
    // Down into the sea and through the flooded tunnel under the docks.
    b.ground(950.0, 4350.0, SEABED, QUAY, QUAY_EDGE);
    b.block(1150.0, 4150.0, G, 625.0, PLANK, PLANK_EDGE);
    // A tall pillar keeps those who cannot fly or climb off the deck.
    b.block(1150.0, 1210.0, G - 420.0, G, QUAY, QUAY_EDGE);
    rings(b, 1300.0, G - 40.0, 8);
    rings(b, 2600.0, G - 40.0, 8);
    b.monitor(MonitorKind::Life, 3900.0, G);
    rings(b, 1250.0, SEABED - 30.0, 6);
    b.walker(EnemyKind::Crawler, 1700.0, SEABED);
    b.enemy(EnemyKind::Puffer, 2050.0, SEABED - 38.0);
    b.vent(2300.0, SEABED);
    rings(b, 2400.0, SEABED - 30.0, 5);
    b.walker(EnemyKind::Crawler, 2750.0, SEABED);
    b.enemy(EnemyKind::Puffer, 3050.0, SEABED - 38.0);
    b.vent(3350.0, SEABED);
    rings(b, 3450.0, SEABED - 30.0, 5);
    b.enemy(EnemyKind::Puffer, 3800.0, SEABED - 38.0);
    // Up and out.
    b.enemy(EnemyKind::Fish, 4250.0, SEA + 50.0);
    b.ground(4350.0, 6800.0, G, PLANK, PLANK_EDGE);
    b.post(4450.0, G);
    rings(b, 4550.0, G - 40.0, 6);
    b.walker(EnemyKind::Crab, 4900.0, G);
    b.enemy(EnemyKind::Gull, 5200.0, G - 320.0);
    b.spikes(5450.0, G, 2);
    b.enemy(EnemyKind::Puffer, 5750.0, G - 90.0);
    b.monitor(MonitorKind::Rings, 6000.0, G);
    rings(b, 6080.0, G - 40.0, 4);
    b.goal(6450.0, G);
}

fn harbour_3(b: &mut Builder) {
    b.water_to(SEA, 1.0);
    b.sonic(140.0, G - 40.0);
    b.ground(-400.0, 1300.0, G, PLANK, PLANK_EDGE);
    rings(b, 300.0, G - 40.0, 8);
    b.monitor(MonitorKind::Shield, 700.0, G);
    b.monitor(MonitorKind::Rings, 800.0, G);
    b.enemy(EnemyKind::Gull, 1050.0, G - 300.0);
    rings(b, 950.0, G - 40.0, 5);
    b.ground(1300.0, 1550.0, SEABED, SAND, SAND);
    b.enemy(EnemyKind::Fish, 1420.0, SEA + 50.0);
    arc(b, 1310.0, G - 70.0, 6, 230.0, 110.0);
    b.ground(1550.0, 3400.0, G, QUAY, QUAY_EDGE);
    b.post(1650.0, G);
    b.monitor(MonitorKind::Rings, 1750.0, G);
    rings(b, 1820.0, G - 40.0, 6);
    rings(b, 550.0, G - 40.0, 5);
    // The anchor crane's arena.
    rings(b, 2100.0, G - 40.0, 5);
    rings(b, 2750.0, G - 40.0, 5);
    b.boss_kind(BossKind::Anchor);
    b.boss(2500.0, G - 170.0);
}

fn halloween(b: &mut Builder) {
    b.scene.night = Some(true);
    b.hunter();
    b.sonic(140.0, G - 40.0);
    b.ground(-400.0, 1600.0, G, SOIL, DEAD_GRASS);
    rings(b, 300.0, G - 40.0, 6);
    b.pumpkin(520.0, G);
    b.tomb(760.0, G, TOMB);
    rings(b, 820.0, G - 40.0, 4);
    b.pumpkin(1000.0, G);
    b.monitor(MonitorKind::Rings, 1150.0, G);
    b.walker(EnemyKind::Crawler, 1380.0, G);
    // Over the hill.
    b.hill(1600.0, 2500.0, G, 100.0, SOIL, DEAD_GRASS);
    arc(b, 1800.0, G - 170.0, 7, 450.0, 30.0);
    b.ground(2500.0, 2900.0, G, SOIL, DEAD_GRASS);
    b.post(2560.0, G);
    b.pumpkin(2700.0, G);
    rings(b, 2740.0, G - 40.0, 4);
    // An open grave.
    arc(b, 2920.0, G - 80.0, 5, 180.0, 90.0);
    b.ground(3120.0, 4900.0, G, SOIL, DEAD_GRASS);
    b.dash_panel(3200.0, 3320.0, G);
    let r = 110.0;
    b.sonic_loop(3700.0, G - r, r, STONE);
    for k in 0..8 {
        let a = (k as f32 / 8.0 * 360.0).to_radians();
        b.ring(3700.0 + a.cos() * (r - 42.0), G - r + a.sin() * (r - 42.0));
    }
    b.tomb(4150.0, G, TOMB);
    b.pumpkin(4300.0, G);
    rings(b, 4200.0, G - 90.0, 5);
    b.enemy(EnemyKind::Flyer, 4450.0, G - 170.0);
    // Stars to keep him away, up on a ledge.
    b.spring(4650.0, G);
    b.platform(4700.0, 4890.0, G - 230.0, WOOD);
    b.monitor(MonitorKind::Invincible, 4800.0, G - 238.0);
    arc(b, 4920.0, G - 80.0, 5, 160.0, 90.0);
    b.ground(5100.0, 7700.0, G, SOIL, DEAD_GRASS);
    b.post(5200.0, G);
    rings(b, 5280.0, G - 40.0, 6);
    b.pumpkin(5450.0, G);
    b.tomb(5650.0, G, TOMB);
    b.monitor(MonitorKind::Shield, 5850.0, G);
    b.pumpkin(5950.0, G);
    b.tomb(6150.0, G, TOMB);
    rings(b, 6250.0, G - 40.0, 6);
    b.walker(EnemyKind::Crab, 6450.0, G);
    b.pumpkin(6650.0, G);
    b.monitor(MonitorKind::Rings, 6800.0, G);
    b.pumpkin(7150.0, G);
    b.goal(7350.0, G);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harbour_has_water_new_badniks_and_the_anchor_crane() {
        let harbour: Vec<&Act> = ALL.iter().filter(|a| a.zone == Zone::Harbour).collect();
        assert_eq!(harbour.len(), 3);
        let specs: Vec<_> = harbour.iter().map(|a| a.scene(0.0, 1440.0)).collect();
        assert!(specs.iter().all(|sc| sc.water.is_some_and(|w| w.level > 0.05 && w.level < 0.2)));
        let kinds: Vec<EnemyKind> =
            specs.iter().flat_map(|sc| sc.game.as_ref().unwrap().enemies.iter().map(|e| e.kind)).collect();
        for k in [EnemyKind::Fish, EnemyKind::Gull, EnemyKind::Puffer] {
            assert!(kinds.contains(&k), "{k:?} shows up");
        }
        assert!(specs[1].game.as_ref().unwrap().vents.len() >= 2, "the tunnel has air");
        assert_eq!(specs[2].game.as_ref().unwrap().boss_kind, BossKind::Anchor);
    }

    #[test]
    fn halloween_is_a_night_hunt() {
        let a = ALL.iter().find(|a| a.zone == Zone::Halloween).expect("a Halloween act");
        let sc = a.scene(0.0, 1440.0);
        let g = sc.game.as_ref().unwrap();
        assert!(g.hunter && g.goal.is_some());
        assert_eq!(sc.night, Some(true));
        assert!(g.pumpkins.len() >= 6);
    }

    #[test]
    fn acts_with_a_goal_hold_enough_rings_for_the_giant_ring() {
        for a in ALL {
            let sc = a.scene(0.0, 1440.0);
            let g = sc.game.as_ref().unwrap();
            let boxes = g.monitors.iter().filter(|m| m.kind == MonitorKind::Rings).count();
            let total = sc.rings.len() + boxes * 10;
            assert!(total >= 60, "{}: {total} rings", a.name);
        }
    }

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
