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
    },
    Challenge { id: "gap", name: "Mind the gap", goal: "Bridge the gap between the platforms", ink: 350.0, build: gap },
    Challenge { id: "catch", name: "Slide", goal: "Catch the falling ball and send it left", ink: 700.0, build: catch },
    Challenge {
        id: "portal",
        name: "Through the wall",
        goal: "The portal gets it past the wall — then guide it",
        ink: 450.0,
        build: portal,
    },
    Challenge { id: "wind", name: "Headwind", goal: "Keep the ball out of the wind", ink: 400.0, build: headwind },
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
    ball(b, 560.0, 80.0);
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
