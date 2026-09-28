//! Challenges: press Go to release the golden ball and get it into the goal.
//! Only drawing is allowed, with a limited amount of ink.

use super::*;

pub const ALL: &[Challenge] = &[
    Challenge {
        id: "ramp",
        name: "Into the cup",
        goal: "Draw a ramp so the ball lands in the cup",
        ink: 900.0,
        build: ramp,
        solution: &[&[(250.0, 330.0), (820.0, 540.0)]],
    },
    Challenge {
        id: "gap",
        name: "Mind the gap",
        goal: "Bridge the gap between the platforms",
        ink: 350.0,
        build: gap,
        solution: &[&[(430.0, 445.0), (650.0, 440.0)]],
    },
    Challenge {
        id: "catch",
        name: "Slide",
        goal: "Catch the falling ball and send it left",
        ink: 700.0,
        build: catch,
        solution: &[&[(600.0, 200.0), (250.0, 480.0)]],
    },
    Challenge {
        id: "portal",
        name: "Through the wall",
        goal: "The portal gets it past the wall — then guide it",
        ink: 450.0,
        build: portal,
        solution: &[&[(650.0, 300.0), (870.0, 560.0)]],
    },
    Challenge {
        id: "wind",
        name: "Headwind",
        goal: "Keep the ball out of the wind",
        ink: 400.0,
        build: headwind,
        solution: &[&[(840.0, 240.0), (840.0, 560.0)]],
    },
    Challenge {
        id: "funnel",
        name: "Funnel",
        goal: "Send the falling ball into the cup on the left",
        ink: 800.0,
        build: funnel,
        solution: &[&[(640.0, 180.0), (240.0, 470.0)]],
    },
    Challenge {
        id: "bouncer",
        name: "Bouncer",
        goal: "Bounce off the red pad and over the wall",
        ink: 400.0,
        build: bouncer,
        solution: &[&[(250.0, 318.0), (400.0, 400.0)]],
    },
    Challenge {
        id: "tailwind",
        name: "Tailwind",
        goal: "Let the wind carry the ball across",
        ink: 700.0,
        build: tailwind,
        solution: &[&[(240.0, 215.0), (560.0, 240.0)]],
    },
    Challenge {
        id: "lift",
        name: "Lift",
        goal: "Get the ball into the rising air",
        ink: 600.0,
        build: lift,
        solution: &[&[(250.0, 535.0), (560.0, 600.0)]],
    },
    Challenge {
        id: "smash",
        name: "Smash",
        goal: "Drop the ball hard enough to break the glass",
        ink: 600.0,
        build: smash,
        solution: &[&[(850.0, 175.0), (560.0, 300.0)]],
    },
    Challenge {
        id: "keyhole",
        name: "Keyhole",
        goal: "Get over the box: the only way in is the low door on its right",
        ink: 600.0,
        build: keyhole,
        solution: &[&[(260.0, 315.0), (500.0, 450.0)]],
    },
    Challenge {
        id: "wrongway",
        name: "Wrong way",
        goal: "The roof sends the ball left — make it go right",
        ink: 500.0,
        build: wrong_way,
        solution: &[&[(640.0, 165.0), (890.0, 220.0)]],
    },
    Challenge {
        id: "splash",
        name: "Splash",
        goal: "Land beyond the wall and let the wind do the rest",
        ink: 800.0,
        build: splash,
        solution: &[&[(250.0, 320.0), (700.0, 420.0)]],
    },
    Challenge {
        id: "wormhole",
        name: "Wormhole",
        goal: "The portal on the left leads to the cup",
        ink: 700.0,
        build: wormhole,
        solution: &[&[(870.0, 260.0), (460.0, 520.0)]],
    },
    Challenge {
        id: "stopsign",
        name: "Stop sign",
        goal: "The ball jumps the gap — stop it above the cup",
        ink: 150.0,
        build: stop_sign,
        solution: &[&[(648.0, 400.0), (648.0, 478.0)]],
    },
];

const FLOOR: f32 = 690.0;

/// A tilted ledge on the left with the ball on it.
fn ledge(b: &mut Builder) {
    b.wall(&[(70.0, 260.0), (260.0, 300.0)], STONE);
    ball(b, 100.0, 238.0);
}

fn ramp(b: &mut Builder) {
    ledge(b);
    goal_cup(b, 840.0, 1000.0, FLOOR, 110.0);
}

fn gap(b: &mut Builder) {
    b.wall(&[(60.0, 290.0), (440.0, 430.0)], STONE);
    ball(b, 95.0, 278.0);
    b.wall(&[(640.0, 430.0), (860.0, 430.0)], STONE);
    b.wall(&[(860.0, 320.0), (860.0, 430.0)], STONE);
    b.zone(ZoneKind::Goal, (720.0, 340.0, 852.0, 423.0), 0.0, 0.0);
}

fn catch(b: &mut Builder) {
    ball(b, 560.0, 130.0);
    goal_cup(b, 70.0, 230.0, FLOOR, 110.0);
    b.wall(&[(400.0, FLOOR), (400.0, 590.0)], STONE);
}

fn portal(b: &mut Builder) {
    ledge(b);
    b.wall(&[(560.0, 150.0), (560.0, FLOOR)], STONE);
    b.zone(ZoneKind::Portal, (270.0, 590.0, 470.0, FLOOR), 0.0, 0.0);
    b.zone(ZoneKind::Portal, (640.0, 140.0, 780.0, 240.0), 0.0, 0.0);
    goal_cup(b, 880.0, 1040.0, FLOOR, 110.0);
}

fn headwind(b: &mut Builder) {
    ball(b, 880.0, 80.0);
    b.zone(ZoneKind::Wind, (420.0, 250.0, 1000.0, 470.0), 180.0, 30.0);
    goal_cup(b, 780.0, 960.0, FLOOR, 110.0);
}

fn funnel(b: &mut Builder) {
    ball(b, 550.0, 130.0);
    // Doing nothing, the ball just lands on this shelf.
    b.wall(&[(490.0, 400.0), (610.0, 400.0)], STONE);
    goal_cup(b, 60.0, 200.0, FLOOR, 110.0);
}

fn bouncer(b: &mut Builder) {
    ledge(b);
    let rubber = Material { bounce: 1.0, friction: 0.8, ..Material::DEFAULT };
    b.stroke(&dense(&[(520.0, 672.0), (680.0, 672.0)]), 14.0, CORAL, true, rubber);
    b.wall(&[(760.0, 460.0), (760.0, FLOOR)], STONE);
    goal_cup(b, 800.0, 980.0, FLOOR, 110.0);
}

fn tailwind(b: &mut Builder) {
    b.wall(&[(60.0, 160.0), (240.0, 200.0)], STONE);
    ball(b, 90.0, 138.0);
    b.zone(ZoneKind::Wind, (260.0, 60.0, 1060.0, 300.0), 0.0, 12.0);
    b.wall(&[(500.0, 480.0), (500.0, FLOOR)], STONE);
    goal_cup(b, 880.0, 1040.0, FLOOR, 110.0);
}

fn lift(b: &mut Builder) {
    b.wall(&[(60.0, 480.0), (240.0, 520.0)], STONE);
    ball(b, 90.0, 458.0);
    // A bump stops the ball rolling into the lift by itself.
    b.wall(&[(420.0, 640.0), (420.0, FLOOR)], STONE);
    b.zone(ZoneKind::Float, (560.0, 180.0, 730.0, FLOOR), 0.0, 14.0);
    // Stops the ball inside the rising air.
    b.wall(&[(740.0, 380.0), (740.0, FLOOR)], STONE);
    b.zone(ZoneKind::Wind, (560.0, 60.0, 1060.0, 180.0), 0.0, 20.0);
    b.wall(&[(780.0, 300.0), (1000.0, 300.0)], STONE);
    goal_cup(b, 800.0, 980.0, 300.0, 90.0);
}

fn smash(b: &mut Builder) {
    b.wall(&[(1040.0, 120.0), (860.0, 160.0)], STONE);
    ball(b, 1010.0, 102.0);
    // A cup whose goal is under the glass: the glass has to go.
    b.wall(&[(250.0, 510.0), (250.0, FLOOR)], STONE);
    b.wall(&[(410.0, 510.0), (410.0, FLOOR)], STONE);
    b.zone(ZoneKind::Goal, (262.0, 592.0, 398.0, FLOOR), 0.0, 0.0);
    let glass = Material { breakable: true, strength: 7.0, bounce: 0.2, ..Material::DEFAULT };
    b.stroke(&dense(&[(269.0, 600.0), (391.0, 600.0)]), 22.0, GLASS, true, glass);
}

fn keyhole(b: &mut Builder) {
    ledge(b);
    // A box around the goal, open only at the bottom of its right side,
    // and a slope that brings anything landing on the right back to it.
    b.wall(&[(520.0, 560.0), (520.0, FLOOR)], STONE);
    b.wall(&[(520.0, 560.0), (680.0, 560.0)], STONE);
    b.wall(&[(680.0, 560.0), (680.0, 620.0)], STONE);
    b.zone(ZoneKind::Goal, (532.0, 580.0, 668.0, FLOOR), 0.0, 0.0);
    b.wall(&[(1080.0, 520.0), (760.0, 684.0)], STONE);
}

fn wrong_way(b: &mut Builder) {
    ball(b, 700.0, 130.0);
    b.wall(&[(300.0, 300.0), (800.0, 200.0)], STONE);
    goal_cup(b, 880.0, 1062.0, FLOOR, 110.0);
}

fn splash(b: &mut Builder) {
    ledge(b);
    b.water(0.3, 1.6);
    b.zone(ZoneKind::Wind, (260.0, 380.0, 1060.0, 560.0), 0.0, 12.0);
    b.wall(&[(600.0, 420.0), (600.0, FLOOR)], STONE);
    b.zone(ZoneKind::Goal, (880.0, 380.0, 1090.0, 560.0), 0.0, 0.0);
}

fn wormhole(b: &mut Builder) {
    b.wall(&[(1040.0, 200.0), (880.0, 240.0)], STONE);
    ball(b, 1010.0, 182.0);
    // The entrance sits in a walled pit; the exit is above the cup.
    b.wall(&[(290.0, 560.0), (290.0, FLOOR)], STONE);
    b.wall(&[(450.0, 560.0), (450.0, FLOOR)], STONE);
    b.zone(ZoneKind::Portal, (300.0, 590.0, 440.0, FLOOR), 0.0, 0.0);
    b.zone(ZoneKind::Portal, (70.0, 330.0, 190.0, 430.0), 0.0, 0.0);
    goal_cup(b, 60.0, 200.0, FLOOR, 110.0);
}

fn stop_sign(b: &mut Builder) {
    ledge(b);
    b.wall(&[(300.0, 360.0), (590.0, 470.0)], STONE);
    b.wall(&[(646.0, 484.0), (950.0, 560.0)], STONE);
    goal_cup(b, 540.0, 690.0, FLOOR, 110.0);
}
