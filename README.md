# Gravity Engine

A 2D physics sandbox built with Rust, [macroquad](https://github.com/not-fl3/macroquad) and [rapier2d](https://rapier.rs/). Drop images, spawn shapes or fetch retro web buttons — everything becomes a rigid body you can throw, pull, spin and blow up.

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)
![Static Badge](https://img.shields.io/badge/Claude-yes-green?logo=claude)
![Version](https://img.shields.io/badge/version-2.3-8b78ff)

![Title screen](docs/screenshots/title.png)

| Sandbox | Settings drawer (Space) | Tool picker (Tab) |
|---|---|---|
| ![Sandbox](docs/screenshots/sandbox.png) | ![Settings](docs/screenshots/settings.png) | ![Tools](docs/screenshots/tools.png) |

## What's new in 2.3

| Challenges (E) | Breakable objects |
|---|---|
| ![Challenge](docs/screenshots/challenge.png) | ![Shattered glass tower](docs/screenshots/shatter.png) |

- **Challenges** — five puzzles: draw lines (with limited ink) so the golden ball reaches the goal. `Space` to release it, `R` to retry.
- **Example scenes** — Newton's cradle, domino run, a car, a wrecking ball, a pool party, magnets, a conveyor factory and a portal loop, all in the library (`E`).
- **Breakable objects** — objects marked *Breakable* (or *Glass*) shatter into real pieces on hard impacts.
- **Particle effects** — sparks, dust, splashes, debris and confetti.
- **Motors** — spinning hinges for wheels, mills and cars (Link tool → Motor).
- **Zone tool** (`Z`) — wind, float and portal areas.
- **Magnets and conveyor surfaces** in the Properties panel.

![Library](docs/screenshots/library.png)

## What's new in 2.2

![Drawings, links and water](docs/screenshots/playground.png)

- **Draw tool** (`0`) — draw with the mouse: a loop becomes a filled shape, a line becomes a plank. Pin drawings to build ramps and courses.
- **Link tool** (`J`) — ropes, springs and hinges between objects, or to the background.
- **Undo / redo** (`Ctrl+Z` / `Ctrl+Y`) for every edit.
- **Object properties** (right-click → *Properties…*, or `I`) — bounce, friction, mass and gravity, with presets (Rubber, Ice, Heavy, Balloon).
- **Water** (`H`) — buoyancy and drag; light objects float, heavy ones sink, and the waves follow the music.
- **GIF recording** (`F11`) — record the scene without the interface.
- **Windows build** on the releases page.

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
- **12 tools** — `Tab`, `1`–`9`, `0`, `J` or `Z`:

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
  | 0 | Draw | Draw shapes and planks (`Shift` pins / unpins them) |
  | J | Link | Ropes, springs, hinges and motors between objects, or to the background |
  | Z | Zone | Wind, float and portal areas |

  The tool card (bottom left) holds each tool's settings: radius / strength for area tools (mouse wheel: radius, `Shift`+wheel: strength), thickness, colour and pinning for Draw, the kind of link (and motor speed) for Link, and the kind of zone for Zone.
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
- **Water** (`H`) — a pool with buoyancy and drag; level and density in the settings drawer, waves react to splashes and to the music
- **Object properties** — bounce, friction, mass, gravity, breakable, magnet and conveyor per object (right-click → *Properties…*, or `I`)
- **Library** (`E`) — example scenes and challenges
- **Undo / redo** — `Ctrl+Z` / `Ctrl+Y` for every edit
- **Gravity** — presets ZERO / MOON / MARS / EARTH / JUPITER / HEAVY / REVERSE in the HUD, or any value with the slider
- **Window shake** (`W`) — moving the window pushes every body; KDE Wayland via KWin DBus, X11 otherwise
- **Trails** (`T`) — motion-blur ghost trail with length and fade settings
- **Audio visualizer** — `V` cycles a spectrum layer behind the objects (Bars / Wave / Radial); `Shift+V` drops a visualizer *screen* that is a real physics object (throw it, pin it, resize it). Sensitivity and an optional "objects jump on the beat" mode live in the settings drawer
- **Audio player** (`M`) — tracker modules (.mod/.xm/.it/.s3m/…), common formats (.mp3/.flac/.wav/.ogg/…), `.pls` playlists including HTTP radio streams
- **Screenshots** (`F12`) and **GIF recording** (`F11`, up to 30 s) of the scene, without the interface
- **Debug overlay** (`D`) — FPS, collider outlines, velocities and details of the object under the cursor

## Controls

Press **F1** in the app for the full list.

| Key / Input | Action |
|-------------|--------|
| Left-drag | Use the current tool |
| Right-click | Object menu (Resize, Duplicate, Resize all, Properties, Pin, Detach links, Delete) |
| `Tab` / `1`–`9`, `0`, `J`, `Z` | Tool picker / select a tool |
| `E` | Examples & challenges |
| `Space` / `R` / `Enter` (in a challenge) | Release the ball / retry / next challenge |
| Wheel / `Shift`+Wheel | Tool radius / strength (Draw: thickness; spawn size when the spawner is open) |
| `I` | Properties of the object under the cursor |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
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
| `W` / `T` / `H` | Toggle window shake / trails / water |
| `M` / `P` | Load music / play-pause |
| `V` / `Shift+V` | Visualizer behind objects / spawn a visualizer screen |
| `Ctrl+S` / `Ctrl+O` | Save / open a scene |
| `F12` / `F11` | Screenshot / start or stop a GIF recording (scene only, without UI) |
| `D` | Debug overlay |
| `F1` | Help |
| `Esc` | Close the top-most panel, or quit |
| `Q` | Quit |

## Download

Linux and Windows builds are attached to each [release](https://github.com/elimanee/gravity-engine/releases). On Windows, unzip `gravity_engine-windows-x86_64.zip` and run `gravity_engine.exe` (keep the DLLs next to it).

## Build & Run

Requires Rust (stable) plus a few system libraries. On Debian / Ubuntu:

```bash
sudo apt install pkg-config libopenmpt-dev libasound2-dev libx11-dev libxi-dev \
                 libgl1-mesa-dev libwayland-dev libxkbcommon-dev libgtk-3-dev libdbus-1-dev
cargo run --release
```

Command line: `gravity_engine [--no-title] [FILES…]` — files can be images, audio, playlists or `.gscene` scenes.

On Windows, install libopenmpt with [vcpkg](https://vcpkg.io) (`vcpkg install libopenmpt:x64-windows`), copy its `libopenmpt.lib` to a folder as `openmpt.lib`, set `OPENMPT_LIB_DIR` to that folder and run `cargo build --release`; the DLLs from vcpkg's `installed\x64-windows\bin` must sit next to the executable (or be on `PATH`). `.github/scripts/openmpt-windows.ps1` does all of this in CI.

For the fastest (but non-portable) binary, build for your own CPU:

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
```

## Files & configuration

Everything lives in `~/.config/gravity_engine/` (or `$XDG_CONFIG_HOME/gravity_engine/`, or `%APPDATA%\gravity_engine\` on Windows):

| File | Purpose |
|------|---------|
| `settings.json` | Your preferences, saved on exit. Set `"show_title": false` to skip the title screen. |
| `scenes/` | Default folder for saved scenes |
| `sgdb_key` | SteamGridDB API key for the logo fetcher (or set `SGDB_API_KEY`) |

Screenshots and GIFs go to `~/Pictures/gravity_engine/` (or `~/.config/gravity_engine/screenshots/` if you have no Pictures folder).

## Project layout

```
src/
├── main.rs            entry point, window config, CLI
├── app/               app state and frame loop (input → UI → actions → simulation → render),
│                      building tools, impacts and shattering, challenge mode
├── config.rs          constants and paths          settings.rs   persisted preferences
├── assets.rs          image / GIF / SVG decoding   shapes.rs     procedural shapes
├── scene.rs           .gscene save / load          background.rs animated backgrounds
├── drawing.rs         Draw-tool strokes → shapes   history.rs    undo / redo
├── recorder.rs        GIF recording                net.rs        web fetchers
├── effects.rs         particles                    library/      example scenes and challenges
├── window_tracker.rs  window shake
├── audio/             tracker modules, streams, playlists and the visualizer's analyzer
├── physics/           world, objects, tools, links, zones, water, magnets, fracture, border modes
└── ui/                theme, widgets, HUD, drawer, tool card, menus, properties panel, overlays, title screen
```

`cargo test` runs the unit tests (decoding, shapes, drawings, links, motors, conveyors, magnets, zones, water, fracture, particles, scenes, the built-in library, undo, GIF encoding, settings, playlist parsing, scraping…).

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
- [`x11`](https://github.com/erlepereira/x11-rs) / [`zbus`](https://github.com/dbus2/zbus) / [`windows-sys`](https://github.com/microsoft/windows-rs) — window position tracking (X11 / KDE Wayland / Windows)

The bundled Inter font is © The Inter Project Authors, licensed under the SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).
