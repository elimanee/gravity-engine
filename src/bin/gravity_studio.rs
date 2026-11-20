//! Gravity Engine Studio — the character editor and the sprite region editor
//! as a program of their own: characters made up (looks, colours, physics,
//! moves) and sprite sheets laid out (the characters' and the zone's), kept
//! where the game finds them. The studio itself is in the library
//! ([`gravity_engine::run_studio`]).

// No console window behind it on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn window_conf() -> macroquad::window::Conf {
    gravity_engine::studio_conf()
}

#[macroquad::main(window_conf)]
async fn main() {
    gravity_engine::run_studio().await;
}
