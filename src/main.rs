//! Gravity Engine — a 2D physics sandbox built on macroquad + rapier2d.
//!
//! Drop images (PNG / JPG / GIF / SVG / …), spawn shapes or fetch retro web
//! buttons; every one of them becomes a rigid body. Press F1 in the app for
//! the full list of controls. The game itself is in the library
//! ([`gravity_engine::run_game`]).

// No console window behind the game on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

/// Lets the web page's `gravity_web.js` check that it matches this build.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn gravity_web_crate_version() -> u32 {
    1
}

fn window_conf() -> macroquad::window::Conf {
    gravity_engine::game_conf()
}

#[macroquad::main(window_conf)]
async fn main() {
    gravity_engine::run_game().await;
}
