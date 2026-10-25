//! Gravity Engine — a 2D physics sandbox built on macroquad + rapier2d.
//!
//! Drop images (PNG / JPG / GIF / SVG / …), spawn shapes or fetch retro web
//! buttons; every one of them becomes a rigid body. Press F1 in the app for
//! the full list of controls.
//!
//! Usage: `gravity_engine [--no-title] [--benchmark] [--act=N] [--special-stage=N]
//! [--sprite-regions[=CHARACTER]] [FILES…]` — files may be images, audio
//! (tracker modules, mp3/flac/ogg/…, .pls playlists) or `.gscene` scenes.

// No console window behind the game on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod assets;
mod audio;
mod background;
mod camera;
mod config;
mod dialog;
mod drawing;
mod effects;
mod game;
mod gamepad;
mod history;
mod library;
mod lighting;
mod net;
mod physics;
mod profile;
mod recorder;
mod scene;
mod settings;
mod shapes;
mod skin;
mod sonic;
mod special;
mod storage;
mod ui;
mod util;
mod weather;
mod window_tracker;

use macroquad::prelude::*;

/// Lets the web page's `gravity_web.js` check that it matches this build.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn gravity_web_crate_version() -> u32 {
    1
}

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
    let mut verify = None;
    let mut benchmark = false;
    let mut act = None;
    let mut special = None;
    let mut regions = None;
    let mut files = vec![];
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--no-title" => skip_title = true,
            "--benchmark" => benchmark = true,
            // Development: play every challenge's reference solution.
            "--verify-challenges" => verify = Some(None),
            a if a.starts_with("--verify-challenge=") => verify = Some(a[19..].parse::<usize>().ok().map(|n| n - 1)),
            // Development: straight into Sonic act N, or special stage N.
            a if a.starts_with("--act=") => act = a[6..].parse::<usize>().ok().and_then(|n| n.checked_sub(1)),
            a if a.starts_with("--special-stage=") => {
                special = a[16..].parse::<usize>().ok().and_then(|n| n.checked_sub(1));
            }
            // Straight into the sprite region editor (on this character's sheet).
            "--sprite-regions" => regions = Some(None),
            a if a.starts_with("--sprite-regions=") => {
                let name = a[17..].to_lowercase();
                regions = Some(sonic::Character::ALL.into_iter().find(|c| c.short().to_lowercase() == name));
            }
            "-h" | "--help" => {
                println!(
                    "usage: gravity_engine [--no-title] [--benchmark] [--act=N] [--special-stage=N] \
                     [--sprite-regions[=CHARACTER]] [FILES…]\n\n  \
                     --benchmark        run stress scenes, print frame times and quit\n  \
                     --act=N            play Sonic act N (1-7)\n  \
                     --special-stage=N  play special stage N (1-7)\n  \
                     --sprite-regions   edit the characters' sprite sheet regions (sonic, tails,\n                     \
                     knuckles, amy or exe)\n\n\
                     Press F1 in the app for controls."
                );
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
    let mut app = app::App::new(files, skip_title || benchmark);
    if let Some(only) = verify {
        let ok = app.verify_challenges(only);
        // Exit without saving the settings the check changed.
        std::process::exit(if ok { 0 } else { 1 });
    }
    if benchmark {
        app.start_benchmark();
    }
    if let Some(i) = act {
        app.play_act(i);
    }
    if let Some(i) = special {
        app.play_special(i);
    }
    if let Some(c) = regions {
        app.edit_regions(c);
    }
    loop {
        app.frame();
        if app.benchmark_done() {
            // Exit without saving the settings the benchmark changed.
            std::process::exit(0);
        }
        // A web page cannot quit: the tab is closed instead.
        if (app.quit || is_quit_requested()) && !cfg!(target_arch = "wasm32") {
            break;
        }
        app.quit = false;
        next_frame().await;
    }
    // Settings are saved when `app` is dropped.
}
