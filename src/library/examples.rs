//! Example scenes, opened from the library panel (E).

use super::*;

pub const ALL: &[Example] = &[
    Example { name: "Newton's cradle", about: "Five steel balls on ropes", build: cradle },
    Example { name: "Domino run", about: "A ball rolls down and knocks them over", build: dominoes },
    Example { name: "Hill climber", about: "A car with two motor-driven wheels", build: car },
    Example { name: "Wrecking ball", about: "A heavy ball swings into a glass tower", build: wrecking_ball },
    Example { name: "Pool party", about: "Floaters, sinkers and balloons", build: pool },
    Example { name: "Magnet field", about: "Magnets gather, repulsors keep away", build: magnets },
    Example { name: "Parcel factory", about: "Conveyor belts sort falling parcels", build: factory },
    Example { name: "Portal loop", about: "Endless fall through a pair of portals", build: portals },
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
