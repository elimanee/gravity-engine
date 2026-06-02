# Gravity Engine

A 2D physics sandbox built with Rust, [macroquad](https://github.com/not-fl3/macroquad) and [rapier2d](https://rapier.rs/). Drop images into the window and watch them fall, collide, and fly.

![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust)

## Features

- **Drop any image** — PNG, JPG, GIF (animated), WebP, BMP, SVG and more; each becomes a rigid body
- **Object spawner** (`N`) — spawn Circle, Box, Triangle, Pentagon or Star shapes with custom size and color
- **88×31 button fetcher** (`F`) — fetches 20 random classic web buttons from [cyber.dabamos.de](https://cyber.dabamos.de/88x31/) and [hellnet.work](https://hellnet.work/8831/) and drops them as live physics objects (animated GIFs supported)
- **8 drag modes** — press `Tab` to open the grid picker:
  | Mode | Description |
  |------|-------------|
  | Spring | Pull and throw objects with a spring force |
  | Slingshot | Pull back and launch in the opposite direction |
  | Pull | Gravitational attraction field |
  | Push | Repulsion field |
  | Vortex | Spin objects counterclockwise around the cursor |
  | Freeze | Slow objects down within a radius |
  | Orbit | Make objects circle the cursor |
  | Bomb | Click to detonate an instant radial explosion |
- **7 border modes** — press `B` to cycle:
  | Mode | Description |
  |------|-------------|
  | Walls | Solid side walls |
  | Loop | Objects wrap left ↔ right |
  | Kill | Objects leaving the sides are deleted |
  | Warp | Objects wrap on all 4 sides |
  | Bounce | Elastic impulse repulsion near edges |
  | Repulse | Soft force field pushing objects away from all edges |
  | Portal | All 4 edges are portals, no floor — animated rainbow borders |
- **6 backgrounds** — press `G` to cycle:
  | Mode | Description |
  |------|-------------|
  | Dark | Default dark purple gradient |
  | Space | Deep space with twinkling stars and nebula |
  | Grid | Dark grid (useful for observing physics) |
  | Sunset | Purple sky fading to an orange horizon with a glowing sun |
  | Ocean | Deep blue with animated caustic ripple rings |
  | Custom | Load any image as a background |
- **Window shake physics** (`W` to toggle) — moving or shaking the window pushes all rigid bodies via inertia; works natively on KDE Wayland via KWin DBus, falls back to X11
- **Gravity presets** — ZERO / MOON / MARS / EARTH / JUPITER / HEAVY / REVERSE
- **Animated trails** — motion-blur-style ghost trail with length and fade sliders
- **Audio player** — load tracker modules (.mod/.xm/.it/.s3m) or common formats (.mp3/.flac/.wav/.ogg); supports `.pls` playlists and HTTP radio streams
- **SVG support** — SVG files are rasterized and used as physics bodies
- **Convex-hull colliders** — transparency is used to compute accurate collision shapes
- **Pause-gated UI** — press `Space` to pause and reveal all sliders and controls

## Controls

| Key / Input | Action |
|-------------|--------|
| `A` | Open file picker (add image) |
| `N` | Open object spawner |
| `F` | Fetch 20 random 88×31 buttons from the web |
| Left-drag | Throw / interact with objects (depends on drag mode) |
| Right-click | Context menu (Resize, Copy & Paste, Size All, Delete) |
| `Tab` | Open drag mode picker |
| `B` | Cycle border mode |
| `G` | Cycle background (or load custom image) |
| `W` | Toggle window shake on/off |
| `Space` | Pause / unpause physics (reveals UI sliders) |
| `T` | Toggle object trails |
| `M` | Load audio file |
| `P` | Pause / resume audio |
| `D` | Toggle debug overlay |
| `R` | Clear all objects |
| `Q` / `Esc` | Quit |

### Pause menu sliders

| Slider | Description |
|--------|-------------|
| Volume | Audio playback volume |
| Trail Length | Number of ghost frames in the trail |
| Trail Fade | How long (seconds) before trail ghosts vanish |
| Window Force | How strongly window movement shakes the rigid bodies (0–40) |

## Build & Run

Requires Rust (stable) and the `libopenmpt` system library.

```bash
cargo run --release
```

## Stack

- [`macroquad`](https://github.com/not-fl3/macroquad) — windowing, rendering, input
- [`rapier2d`](https://rapier.rs/) — 2D rigid body physics
- [`rodio`](https://github.com/RustAudio/rodio) + [`symphonia`](https://github.com/pdeljanov/Symphonia) — audio decoding
- [`resvg`](https://github.com/RazrFalcon/resvg) — SVG rasterization
- [`rfd`](https://github.com/PolyMeilex/rfd) — native file dialog
- [`image`](https://github.com/image-rs/image) — image loading and processing
- [`ureq`](https://github.com/algesten/ureq) — HTTP for 88×31 button fetching and audio streams
- [`x11`](https://github.com/erlepereira/x11-rs) — window position tracking for shake physics (X11)
- [`zbus`](https://github.com/dbus2/zbus) — KDE KWin DBus for window position on Wayland
