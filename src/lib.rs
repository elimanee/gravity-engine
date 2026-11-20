//! Gravity Engine — a 2D physics sandbox built on macroquad + rapier2d,
//! with a Sonic game, a 3D mode and the editors to make things for them.
//!
//! Two programs are built on this library: the game (`gravity_engine`,
//! [`run_game`]) and the studio (`gravity_studio`, [`run_studio`]), where
//! characters are made up and sprite sheets laid out.

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
mod three;
mod ui;
mod util;
mod weather;
mod window_tracker;

use macroquad::prelude::*;

/// The game's window.
pub fn game_conf() -> Conf {
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

/// The game, from the command line's arguments: `gravity_engine
/// [--no-title] [--benchmark] [--act=N] [--special-stage=N] [--3d[=run|zone]]
/// [FILES…]`; files may be images, audio (tracker modules, mp3/flac/ogg/…,
/// .pls playlists) or `.gscene` scenes.
pub async fn run_game() {
    let mut skip_title = false;
    let mut verify = None;
    let mut benchmark = false;
    let mut act = None;
    let mut special = None;
    let mut three = None;
    let mut studio = None;
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
            // Straight into the 3D mode: the endless run or the open zone.
            "--3d" | "--3d=run" => three = Some(false),
            "--3d=zone" => three = Some(true),
            // The studio's editors: the studio is opened on them.
            a if app::studio_arg(a).is_some() => studio = app::studio_arg(a),
            "-h" | "--help" => {
                println!(
                    "usage: gravity_engine [--no-title] [--benchmark] [--act=N] [--special-stage=N] \
                     [--3d[=run|zone]] [FILES…]\n\n  \
                     --benchmark        run stress scenes, print frame times and quit\n  \
                     --act=N            play Sonic act N (1-7)\n  \
                     --special-stage=N  play special stage N (1-7)\n  \
                     --3d[=run|zone]    play the 3D mode: the endless run or the open zone\n\n\
                     The character editor and the sprite region editor are in the studio\n\
                     (gravity_studio --help). Press F1 in the app for controls."
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
    if let Some(zone) = three {
        app.play_3d(zone);
    }
    if let Some(start) = studio {
        app.open_studio(start);
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

/// The studio's window.
pub fn studio_conf() -> Conf {
    Conf { window_title: format!("{} Studio {}", config::APP_NAME, config::APP_VERSION), ..game_conf() }
}

/// The studio, from the command line's arguments: `gravity_studio
/// [--characters | --sprite-regions[=WHO]]`.
pub async fn run_studio() {
    let mut start = None;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "usage: gravity_studio [--characters | --sprite-regions[=WHO]]\n\n  \
                     --characters          open on the character editor\n  \
                     --sprite-regions      open on the sprite region editor: a character's sheet\n                        \
                     (sonic, tails, knuckles, amy, exe, custom:NAME) or the\n                        \
                     zone's tiles (zone)\n\n\
                     What it makes is kept where the game finds it."
                );
                return;
            }
            "-V" | "--version" => {
                println!("{} Studio {}", config::APP_NAME, config::APP_VERSION);
                return;
            }
            a => match app::studio_arg(a) {
                Some(s) => start = Some(s),
                None => eprintln!("gravity_studio: unknown argument {a} (see --help)"),
            },
        }
    }
    ui::theme::load_fonts();
    prevent_quit();
    let mut app = app::App::studio();
    if let Some(s) = start {
        app.studio_start(s);
    }
    loop {
        app.frame();
        if app.quit || is_quit_requested() {
            break;
        }
        next_frame().await;
    }
}
