//! Gravity Engine — a 2D physics sandbox built on macroquad + rapier2d.
//!
//! Drop images (PNG / JPG / GIF / SVG / …), spawn shapes or fetch retro web
//! buttons; every one of them becomes a rigid body. Press F1 in the app for
//! the full list of controls.
//!
//! Usage: `gravity_engine [--no-title] [FILES…]` — files may be images,
//! audio (tracker modules, mp3/flac/ogg/…, .pls playlists) or `.gscene` scenes.

// No console window behind the game on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod assets;
mod audio;
mod background;
mod config;
mod drawing;
mod effects;
mod history;
mod library;
mod net;
mod physics;
mod recorder;
mod scene;
mod settings;
mod shapes;
mod ui;
mod util;
mod window_tracker;

use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: format!("{} {}", config::APP_NAME, config::APP_VERSION),
        window_width: 1100,
        window_height: 720,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut skip_title = false;
    let mut files = vec![];
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--no-title" => skip_title = true,
            "-h" | "--help" => {
                println!("usage: gravity_engine [--no-title] [FILES…]\n\nPress F1 in the app for controls.");
                return;
            }
            "-V" | "--version" => {
                println!("{} {}", config::APP_NAME, config::APP_VERSION);
                return;
            }
            _ => files.push(arg),
        }
    }

    ui::theme::load_fonts();
    prevent_quit();
    let mut app = app::App::new(files, skip_title);
    loop {
        app.frame();
        if app.quit || is_quit_requested() {
            break;
        }
        next_frame().await;
    }
    // Settings are saved when `app` is dropped.
}
