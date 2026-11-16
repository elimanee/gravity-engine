//! The 3D mode: Sonic and his friends in a 3D world, with the 2D game's
//! physics carried over (see [`player`]), an endless course generated as
//! they run and an open zone to explore, things to drop, throw and blow up
//! (rapier3d), and the characters drawn from a model pack (MD3, loaded at
//! run time from the player's own files) or built from simple shapes.

pub mod course;
pub mod figure;
pub mod gfx;
pub mod md3;
pub mod models;
pub mod phys;
pub mod player;
pub mod world;
pub mod zone;
