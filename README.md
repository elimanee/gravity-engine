# Gravity Engine

A 2D physics sandbox built with Rust, [macroquad](https://github.com/not-fl3/macroquad) and [rapier2d](https://rapier.rs/). Drop images, spawn shapes or fetch retro web buttons — everything becomes a rigid body you can throw, pull, spin and blow up.

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![Static Badge](https://img.shields.io/badge/Claude-yes-green?logo=claude)
![Version](https://img.shields.io/badge/version-2.0_remaster-8b78ff)

![Title screen](docs/screenshots/title.png)

| Sandbox | Settings drawer (Space) | Tool picker (Tab) |
|---|---|---|
| ![Sandbox](docs/screenshots/sandbox.png) | ![Settings](docs/screenshots/settings.png) | ![Tools](docs/screenshots/tools.png) |

## What's new in 2.0 (remaster)

- **New UI** — floating HUD, a settings drawer that slides in when paused, a tool picker with animated icons, toasts, a “now playing” pill, redesigned menus, help and title screen, bundled [Inter](https://rsms.me/inter/) font.
- **Scenes** — save and reopen whole scenes (`Ctrl+S` / `Ctrl+O`, `.gscene`); images are embedded so scenes can be shared.
- **Persistent settings** — gravity, tool, trails, volume, background… are restored on the next launch.
- **Drag & drop** files onto the window (images, audio, playlists, scenes).
- **Screenshots** (`F12`), **slow motion** (`[` / `]`) and **frame-by-frame stepping** (`.` while paused).
- **Pin objects** in place (right-click → *Pin*); pinned objects can still be dragged around with *Spring*.
- **Swing tool** (`9`) — hold objects by the point you click; they dangle and spin when thrown.
- **Better physics** — frame-rate independent simulation (it used to run faster on high-refresh screens), mass-aware tools, exact colliders for spawned shapes (balls, capsules, rounded boxes, concave stars), a ceiling so *Reverse* gravity no longer loses your objects.
- **New content** — Hexagon and Capsule shapes, random-colour spawning, hold-to-spawn, *Aurora* background, smooth gradient backgrounds.
- **Fixes** — duplicate/resize now work for fetched buttons and spawned shapes, correct animated-GIF disposal, window tracking no longer stalls frames, web fetches run in parallel.

## Features

- **Drop any image** — PNG, JPG, GIF (animated), WebP, BMP, SVG, TIFF, ICO, TGA, QOI…; transparency is used to compute convex-hull colliders
- **Shape spawner** (`N`) — Circle, Box, Triangle, Pentagon, Hexagon, Star, Capsule; pick a colour (or random) and size, click or hold to spawn
- **88×31 button fetcher** (`F`) — 20 random classic web buttons scraped from 8 galleries (animated GIFs supported)
- **Game logo fetcher** (`L`) — 20 random logos from [SteamGridDB](https://www.steamgriddb.com/) (needs an API key, see below)
- **9 tools** — `Tab` or `1`–`9`:

  | # | Tool | Description |
  |---|------|-------------|
  | 1 | Spring | Grab, carry and throw (no spin) |
  | 2 | Slingshot | Pull back, release to launch |
  | 3 | Pull | Attract everything nearby |
  | 4 | Push | Repel everything nearby |
  | 5 | Vortex | Spin objects around the cursor |
  | 6 | Freeze | Slow objects to a crawl |
  | 7 | Orbit | Make objects circle the cursor |
  | 8 | Bomb | Click to detonate |
  | 9 | Swing | Hold by the clicked point: objects dangle and spin when thrown |

  Area tools show a card with radius / strength sliders; the mouse wheel changes the radius (`Shift`+wheel: strength).
- **7 border modes** — `B` to cycle (`Shift+B` backwards):

  | Mode | Description |
  |------|-------------|
  | Walls | Solid walls all around |
  | Loop | Objects wrap left ↔ right |
  | Kill | Objects leaving the sides are deleted |
  | Warp | Objects wrap on all four sides |
  | Bounce | Springy trampolines on the sides |
  | Repulse | Soft force field on every edge |
  | Portal | No floor — every edge is a portal, animated rainbow borders |
- **7 backgrounds** — `G` to cycle: Dark, Space, Grid, Sunset, Ocean, Aurora, Custom (`Shift+G` picks an image)
- **Gravity** — presets ZERO / MOON / MARS / EARTH / JUPITER / HEAVY / REVERSE in the HUD, or any value with the slider
- **Window shake** (`W`) — moving the window pushes every body; KDE Wayland via KWin DBus, X11 otherwise
- **Trails** (`T`) — motion-blur ghost trail with length and fade settings
- **Audio player** (`M`) — tracker modules (.mod/.xm/.it/.s3m/…), common formats (.mp3/.flac/.wav/.ogg/…), `.pls` playlists including HTTP radio streams
- **Debug overlay** (`D`) — FPS, collider outlines, velocities and details of the object under the cursor

## Controls

Press **F1** in the app for the full list.

| Key / Input | Action |
|-------------|--------|
| Left-drag | Use the current tool |
| Right-click | Object menu (Resize, Duplicate, Resize all, Pin, Delete) |
| `Tab` / `1`–`9` | Tool picker / select a tool |
| Wheel / `Shift`+Wheel | Tool radius / strength (spawn size when the spawner is open) |
| `A` | Add images |
| `N` | Shape spawner |
| `F` / `L` | Fetch 88×31 buttons / game logos |
| `Del` | Delete the object under the cursor |
| `R` | Clear all objects |
| `Space` | Pause and open the settings drawer |
| `.` | Step one frame (while paused) |
| `[` / `]` | Slower / faster time |
| `B` / `G` | Next border mode / background |
| `Shift+G` | Background from an image |
| `W` / `T` | Toggle window shake / trails |
| `M` / `P` | Load music / play-pause |
| `Ctrl+S` / `Ctrl+O` | Save / open a scene |
| `F12` | Screenshot (scene only, without UI) |
| `D` | Debug overlay |
| `F1` | Help |
| `Esc` | Close the top-most panel, or quit |
| `Q` | Quit |

## Build & Run

Requires Rust (stable) plus a few system libraries. On Debian / Ubuntu:

```bash
sudo apt install pkg-config libopenmpt-dev libasound2-dev libx11-dev libxi-dev \
                 libgl1-mesa-dev libwayland-dev libxkbcommon-dev libgtk-3-dev libdbus-1-dev
cargo run --release
```

Command line: `gravity_engine [--no-title] [FILES…]` — files can be images, audio, playlists or `.gscene` scenes.

For the fastest (but non-portable) binary, build for your own CPU:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Files & configuration

Everything lives in `~/.config/gravity_engine/` (or `$XDG_CONFIG_HOME/gravity_engine/`):

| File | Purpose |
|------|---------|
| `settings.json` | Your preferences, saved on exit. Set `"show_title": false` to skip the title screen. |
| `scenes/` | Default folder for saved scenes |
| `sgdb_key` | SteamGridDB API key for the logo fetcher (or set `SGDB_API_KEY`) |

Screenshots go to `~/Pictures/gravity_engine/` (or `~/.config/gravity_engine/screenshots/` if you have no Pictures folder).

## Project layout

```
src/
├── main.rs            entry point, window config, CLI
├── app.rs             app state and frame loop (input → UI → actions → simulation → render)
├── config.rs          constants and paths          settings.rs   persisted preferences
├── assets.rs          image / GIF / SVG decoding   shapes.rs     procedural shapes
├── scene.rs           .gscene save / load          background.rs animated backgrounds
├── net.rs             web fetchers                 window_tracker.rs  window shake
├── audio/             tracker modules, streams and playlists
├── physics/           world, objects, tools, border modes
└── ui/                theme, widgets, HUD, drawer, menus, overlays, title screen
```

`cargo test` runs the unit tests (decoding, shapes, scenes, settings, playlist parsing, scraping…).

## Usage of AI
this project is mainly developed by ai (totally not with claude )BUT the project will be rewritten at some point in future (if i don't get too lazy)
(might not be GDPR friendly)

## Stack

- [`macroquad`](https://github.com/not-fl3/macroquad) — windowing, rendering, input
- [`rapier2d`](https://rapier.rs/) — 2D rigid body physics
- [`rodio`](https://github.com/RustAudio/rodio) + [`symphonia`](https://github.com/pdeljanov/Symphonia) — audio decoding
- [`openmpt`](https://lib.openmpt.org/) + [`cpal`](https://github.com/RustAudio/cpal) — tracker module playback
- [`resvg`](https://github.com/RazrFalcon/resvg) — SVG rasterization
- [`rfd`](https://github.com/PolyMeilex/rfd) — native file dialogs
- [`image`](https://github.com/image-rs/image) / [`gif`](https://github.com/image-rs/image-gif) — image loading and processing
- [`ureq`](https://github.com/algesten/ureq) — HTTP for the web fetchers and audio streams
- [`serde`](https://serde.rs/) — settings and scene files
- [`x11`](https://github.com/erlepereira/x11-rs) / [`zbus`](https://github.com/dbus2/zbus) — window position tracking (X11 / KDE Wayland)

The bundled Inter font is © The Inter Project Authors, licensed under the SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).
