//! Example scenes, opened from the library panel (E).

use super::*;
use crate::physics::gadgets::{LAMP_COLOURS, SPOT_SPREAD};

pub const ALL: &[Example] = &[
    Example { name: "Newton's cradle", about: "Five steel balls on ropes", build: cradle },
    Example { name: "Domino run", about: "A ball rolls down and knocks them over", build: dominoes },
    Example { name: "Hill climber", about: "A car with two motor-driven wheels", build: car },
    Example { name: "Wrecking ball", about: "A heavy ball swings into a glass tower", build: wrecking_ball },
    Example { name: "Pool party", about: "Floaters, sinkers and balloons", build: pool },
    Example { name: "Magnet field", about: "Magnets gather, repulsors keep away", build: magnets },
    Example { name: "Parcel factory", about: "Conveyor belts sort falling parcels", build: factory },
    Example { name: "Portal loop", about: "Endless fall through a pair of portals", build: portals },
    Example { name: "Laser lab", about: "Mirrors bend the beam, glass tints it, wood burns", build: laser_lab },
    Example { name: "Rocket car", about: "← → drive  ·  ↑ thruster  ·  ↓ cannon", build: rocket_car },
    Example { name: "Solar system", about: "Planets orbit a sun with its own gravity", build: solar_system },
    Example { name: "Night lights", about: "Lamps cast shadows in the rain  ·  Shift+T for day", build: night_lights },
    Example {
        name: "Grapple swing", about: "↑ hooks at the pointer and reels in  ·  ← → push", build: grapple_swing
    },
    Example {
        name: "Sonic loop",
        about: "← → run  ·  ↓ roll  ·  Space jump  ·  ↓ + Space spin dash",
        build: sonic_loop,
    },
    Example {
        name: "Endless run",
        about: "Sonic in an infinite world made as he runs  ·  how far can you go?",
        build: endless_run,
    },
];

const FLOOR: f32 = 690.0;

fn cradle(b: &mut Builder) {
    b.wall(&[(360.0, 120.0), (740.0, 120.0)], WOOD);
    let steel = Material { bounce: 0.98, friction: 0.0, ..Material::DEFAULT };
    let (len, size) = (300.0, 56.0);
    for i in 0..5 {
        let top = (438.0 + i as f32 * size, 126.0);
        // The first ball starts pulled up to the left.
        let (x, y) = if i == 0 {
            let a = 60f32.to_radians();
            (top.0 - len * a.sin(), top.1 + len * a.cos())
        } else {
            (top.0, top.1 + len)
        };
        let ball = b.shape_with(Shape::Circle, SNOW, x, y, size, false, steel);
        b.link(LinkKind::Rope, ball, (x, y), None, top, 0.0);
    }
}

fn dominoes(b: &mut Builder) {
    b.wall(&[(60.0, 260.0), (330.0, 440.0)], WOOD);
    b.shape(Shape::Circle, CORAL, 90.0, 225.0, 46.0);
    for i in 0..13 {
        let x = 390.0 + i as f32 * 46.0;
        let rgb = if i % 2 == 0 { SKY } else { SNOW };
        b.stroke(&dense(&[(x, FLOOR - 2.0), (x, FLOOR - 84.0)]), 12.0, rgb, false, Material::DEFAULT);
    }
    for (row, n) in [(0, 3), (1, 2), (2, 1)] {
        for k in 0..n {
            let x = 1000.0 - n as f32 * 25.0 + k as f32 * 50.0 + 25.0;
            b.shape(Shape::Box, LEAF, x, FLOOR - 26.0 - row as f32 * 50.0, 48.0);
        }
    }
}

fn car(b: &mut Builder) {
    let hills: Vec<(f32, f32)> = (0..=34)
        .map(|i| {
            let x = 40.0 + i as f32 * 30.0;
            (x, 610.0 - 40.0 * (x / 150.0).sin() - x * 0.04)
        })
        .collect();
    b.wall(&hills, LEAF);
    let body = b.stroke(&dense(&[(120.0, 470.0), (290.0, 470.0)]), 20.0, CORAL, false, Material::DEFAULT);
    let grip = Material { friction: 1.5, bounce: 0.1, ..Material::DEFAULT };
    for x in [140.0, 270.0] {
        let wheel = b.shape_with(Shape::Circle, STONE, x, 500.0, 58.0, false, grip);
        b.link(LinkKind::Motor, wheel, (x, 500.0), Some(body), (x, 500.0), 6.0);
    }
}

fn wrecking_ball(b: &mut Builder) {
    let anchor = (430.0, 50.0);
    let len = 390.0;
    let a = 60f32.to_radians();
    let (x, y) = (anchor.0 - len * a.sin(), anchor.1 + len * a.cos());
    let heavy = Material { bounce: 0.1, friction: 0.6, ..Material::DEFAULT };
    let ball = b.shape_with(Shape::Circle, STONE, x, y, 84.0, false, heavy);
    b.scene.objects[ball].mass = Some(40.0);
    b.link(LinkKind::Rope, ball, (x, y), None, anchor, 0.0);
    for col in 0..3 {
        for row in 0..5 {
            let x = 470.0 + col as f32 * 62.0;
            let y = FLOOR - 31.0 - row as f32 * 62.0;
            b.shape_with(Shape::Box, GLASS, x, y, 60.0, false, Material::GLASS);
        }
    }
}

fn pool(b: &mut Builder) {
    b.water(0.42, 1.6);
    // A raft with passengers.
    b.stroke(&dense(&[(380.0, 380.0), (700.0, 380.0)]), 26.0, WOOD, false, Material::DEFAULT);
    for (i, s) in [Shape::Box, Shape::Triangle, Shape::Hexagon].iter().enumerate() {
        b.shape(*s, [CORAL, LEAF, VIOLET][i], 450.0 + i as f32 * 90.0, 330.0, 50.0);
    }
    // Heavy stones sink, balloons rise.
    for i in 0..4 {
        let stone =
            b.shape_with(Shape::Pentagon, STONE, 150.0 + i as f32 * 70.0, 200.0, 50.0, false, Material::DEFAULT);
        b.scene.objects[stone].mass = Some(8.0);
    }
    for i in 0..3 {
        b.shape_with(Shape::Circle, CORAL, 820.0 + i as f32 * 70.0, 560.0, 44.0, false, Material::BALLOON);
    }
    for i in 0..6 {
        b.shape(
            Shape::Circle,
            [SKY, SNOW, LEAF][i % 3],
            780.0 + (i % 3) as f32 * 60.0,
            150.0 + (i / 3) as f32 * 60.0,
            40.0,
        );
    }
}

fn magnets(b: &mut Builder) {
    b.scene.gravity = -1.6;
    let core = Material { magnet: 2.0, ..Material::MAGNET };
    b.shape_with(Shape::Circle, CORAL, 550.0, 330.0, 90.0, true, core);
    for i in 0..10 {
        let a = i as f32 / 10.0 * std::f32::consts::TAU;
        let (x, y) = (550.0 + a.cos() * 300.0, 330.0 + a.sin() * 230.0);
        if i % 3 == 2 {
            let repel = Material { magnet: -1.0, ..Material::DEFAULT };
            b.shape_with(Shape::Hexagon, SKY, x, y, 44.0, false, repel);
        } else {
            b.shape_with(Shape::Box, CORAL, x, y, 36.0, false, Material::MAGNET);
        }
    }
}

fn factory(b: &mut Builder) {
    let belt = |speed: f32| Material { conveyor: speed, friction: 1.2, bounce: 0.05, ..Material::DEFAULT };
    b.stroke(&dense(&[(90.0, 200.0), (640.0, 240.0)]), 16.0, STONE, true, belt(3.0));
    b.stroke(&dense(&[(720.0, 340.0), (230.0, 380.0)]), 16.0, STONE, true, belt(-3.0));
    b.stroke(&dense(&[(160.0, 480.0), (720.0, 520.0)]), 16.0, STONE, true, belt(3.0));
    goal_bin(b, 780.0, 1000.0);
    for i in 0..8 {
        let shape = if i % 2 == 0 { Shape::Box } else { Shape::Circle };
        b.shape(shape, [WOOD, CORAL, SKY, LEAF][i % 4], 120.0 + i as f32 * 55.0, 160.0, 40.0);
    }
}

/// An open bin on the floor (no goal zone: examples have no goal).
fn goal_bin(b: &mut Builder, x0: f32, x1: f32) {
    b.wall(&[(x0, FLOOR - 130.0), (x0 + 10.0, FLOOR), (x1 - 10.0, FLOOR), (x1, FLOOR - 130.0)], WOOD);
}

fn portals(b: &mut Builder) {
    b.scene.border = BorderMode::Walls;
    b.zone(ZoneKind::Portal, (420.0, 600.0, 680.0, FLOOR), 0.0, 0.0);
    b.zone(ZoneKind::Portal, (420.0, 40.0, 680.0, 120.0), 0.0, 0.0);
    b.zone(ZoneKind::Wind, (140.0, 300.0, 400.0, 480.0), 0.0, 16.0);
    b.zone(ZoneKind::Float, (760.0, 280.0, 1000.0, FLOOR - 10.0), 0.0, 14.0);
    for i in 0..9 {
        let shape = [Shape::Circle, Shape::Star, Shape::Box][i % 3];
        b.shape(shape, [CORAL, SKY, LEAF][i % 3], 470.0 + (i % 3) as f32 * 70.0, 200.0 + (i / 3) as f32 * 70.0, 40.0);
    }
}

fn laser_lab(b: &mut Builder) {
    b.laser((80.0, 120.0), 0.0);
    let mirror = |b: &mut Builder, pts: &[(f32, f32)]| {
        b.stroke(&dense(pts), 14.0, SNOW, true, Material::MIRROR);
    };
    // Right, down, then back to the left through a glass pane.
    mirror(b, &[(670.0, 90.0), (730.0, 150.0)]);
    mirror(b, &[(670.0, 590.0), (730.0, 530.0)]);
    b.stroke(&dense(&[(480.0, 470.0), (480.0, 650.0)]), 18.0, GLASS, true, Material::GLASS);
    for y in [666.0, 618.0, 570.0] {
        b.shape(Shape::Box, WOOD, 200.0, y, 48.0);
    }
    b.shape(Shape::Box, WOOD, 250.0, 666.0, 48.0);
}

fn rocket_car(b: &mut Builder) {
    // A ramp to jump and a tower to shoot at.
    b.wall(&[(560.0, FLOOR), (720.0, 610.0)], STONE);
    for row in 0..4 {
        for k in 0..2 {
            let x = 930.0 + k as f32 * 50.0;
            b.shape(Shape::Box, SKY, x, FLOOR - 26.0 - row as f32 * 50.0, 48.0);
        }
    }
    let body = b.stroke(&dense(&[(110.0, 600.0), (280.0, 600.0)]), 20.0, CORAL, false, Material::DEFAULT);
    let grip = Material { friction: 1.5, bounce: 0.1, ..Material::DEFAULT };
    for x in [130.0, 260.0] {
        let wheel = b.shape_with(Shape::Circle, STONE, x, 632.0, 58.0, false, grip);
        b.link(LinkKind::Motor, wheel, (x, 632.0), Some(body), (x, 632.0), 8.0);
        b.drive_last();
    }
    let thruster = GadgetSpec { kind: GadgetKind::Thruster, trigger: Trigger::Up, power: 4.0, ..GadgetSpec::default() };
    b.gadget(Some(body), (112.0, 600.0), 0.0, thruster);
    let cannon = GadgetSpec {
        kind: GadgetKind::Cannon,
        trigger: Trigger::Down,
        power: 16.0,
        rate: 3.0,
        ..GadgetSpec::default()
    };
    b.gadget(Some(body), (240.0, 588.0), 25.0, cannon);
}

fn solar_system(b: &mut Builder) {
    b.scene.gravity = 0.0;
    let (cx, cy) = (550.0, 360.0);
    let sun = Material { planet: 14.0, ..Material::PLANET };
    b.shape_with(Shape::Circle, [255, 190, 60], cx, cy, 110.0, true, sun);
    let radius = 55.0 / crate::config::PPM;
    // Planets set off on circular orbits (anticlockwise).
    for (dist, angle, size, rgb) in [(150.0, 0.0f32, 30.0, SKY), (230.0, 2.2, 40.0, CORAL), (310.0, 4.0, 24.0, LEAF)] {
        let (x, y) = (cx + dist * angle.cos(), cy - dist * angle.sin());
        let i = b.shape_with(Shape::Circle, rgb, x, y, size, false, Material { bounce: 0.6, ..Material::DEFAULT });
        let v = crate::physics::planets::orbit_speed(sun.planet, radius, dist / crate::config::PPM);
        b.scene.objects[i].vx = -angle.sin() * v;
        b.scene.objects[i].vy = angle.cos() * v;
    }
}

fn night_lights(b: &mut Builder) {
    b.scene.night = Some(true);
    b.scene.weather = Some(crate::weather::Weather::Rain);
    let lamp = |colour: usize, spread: f32, reach: f32| GadgetSpec {
        kind: GadgetKind::Lamp,
        power: reach / crate::config::PPM,
        spread,
        colour: LAMP_COLOURS[colour],
        ..GadgetSpec::default()
    };
    // A street lamp: a post with an arm, the bulb under its end.
    b.stroke(&dense(&[(900.0, FLOOR), (900.0, 270.0), (820.0, 270.0)]), 10.0, STONE, true, Material::DEFAULT);
    b.gadget(None, (820.0, 284.0), -90.0, lamp(0, std::f32::consts::PI, 520.0));
    // A lantern swinging on a rope.
    let lantern = b.shape_with(Shape::Circle, WOOD, 420.0, 250.0, 26.0, false, Material::DEFAULT);
    b.link(LinkKind::Rope, lantern, (420.0, 250.0), None, (420.0, 70.0), 0.0);
    b.scene.objects[lantern].vx = 3.5;
    b.gadget(Some(lantern), (420.0, 250.0), 0.0, lamp(0, std::f32::consts::PI, 420.0));
    // A blue spotlight from the top-left corner.
    b.gadget(None, (70.0, 110.0), -40.0, lamp(4, SPOT_SPREAD, 760.0));
    // Crates, a ball and a glass pane to throw shadows.
    for (x, row) in [(300.0, 0), (352.0, 0), (404.0, 0), (326.0, 1), (378.0, 1), (352.0, 2)] {
        b.shape(Shape::Box, WOOD, x, FLOOR - 26.0 - row as f32 * 50.0, 48.0);
    }
    b.shape(Shape::Circle, CORAL, 620.0, FLOOR - 40.0, 80.0);
    b.shape_with(Shape::Box, GLASS, 720.0, FLOOR - 60.0, 40.0, false, Material::GLASS);
}

fn grapple_swing(b: &mut Builder) {
    // Rocks in the air to hook onto.
    for (x, y) in [(220.0, 150.0), (470.0, 110.0), (720.0, 160.0), (960.0, 120.0)] {
        b.shape_with(Shape::Circle, STONE, x, y, 44.0, true, Material::DEFAULT);
    }
    // A tower of boxes to swing into.
    for row in 0..5 {
        b.shape(Shape::Box, SKY, 1000.0, FLOOR - 26.0 - row as f32 * 50.0, 48.0);
    }
    let hero = b.shape(Shape::Box, CORAL, 110.0, FLOOR - 30.0, 54.0);
    let hook = GadgetSpec { kind: GadgetKind::Grapple, trigger: Trigger::Up, ..GadgetSpec::default() };
    b.gadget(Some(hero), (110.0, FLOOR - 30.0), 90.0, hook);
    for (trigger, deg, at) in [(Trigger::Right, 0.0, 84.0), (Trigger::Left, 180.0, 136.0)] {
        let push = GadgetSpec { kind: GadgetKind::Thruster, trigger, power: 0.8, ..GadgetSpec::default() };
        b.gadget(Some(hero), (at, FLOOR - 30.0), deg, push);
    }
}

fn sonic_loop(b: &mut Builder) {
    b.sonic(130.0, FLOOR - 40.0);
    // A loop on the floor.
    let (cx, r) = (560.0, 120.0);
    b.sonic_loop(cx, FLOOR - r, r, STONE);
    // A spring before it, and rings along the way and round the loop.
    let spring = Material { bounce: 1.0, flammable: false, ..Material::DEFAULT };
    // Behind him: a spring to bounce up to a column of rings.
    b.stroke(&dense(&[(45.0, FLOOR - 4.0), (90.0, FLOOR - 4.0)]), 10.0, CORAL, true, spring);
    for k in 0..5 {
        b.ring(190.0 + k as f32 * 40.0, FLOOR - 40.0);
        b.ring(68.0, FLOOR - 150.0 - k as f32 * 45.0);
    }
    for k in 0..10 {
        let a = (k as f32 / 10.0 * 360.0).to_radians();
        b.ring(cx + a.cos() * (r - 40.0), FLOOR - r + a.sin() * (r - 40.0));
    }
    // A ramp to jump off, and crates to bowl over.
    b.wall(&[(760.0, FLOOR), (900.0, FLOOR - 70.0)], STONE);
    for k in 0..3 {
        b.ring(840.0 + k as f32 * 40.0, FLOOR - 160.0 - k as f32 * 20.0);
    }
    for (x, row) in [(980.0, 0), (1030.0, 0), (1005.0, 1)] {
        b.shape(Shape::Box, WOOD, x, FLOOR - 24.0 - row as f32 * 48.0, 46.0);
    }
}

fn endless_run(b: &mut Builder) {
    // The terrain is made while he runs (see `app::endless`).
    b.scene.endless = true;
}
