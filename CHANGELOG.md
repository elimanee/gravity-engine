# Changelog

Notes for each release. The release workflow publishes the section matching
the tag as the GitHub release description. Versions up to 1.7.0 are
described on the [releases page](https://github.com/elimanee/gravity-engine/releases).

## [2.1.0]

### Added
- **Audio visualizer**, fed by whatever is playing (tracker modules, audio files, playlists and radio streams):
  - `V` cycles a layer drawn behind the objects: Bars, Wave or Radial (with a soft flash on each beat)
  - `Shift+V` (or the settings drawer) spawns a visualizer *screen* — a physics object you can throw, pin, resize, duplicate and save in scenes
  - Settings drawer section: layer style, screen style, sensitivity, and "objects jump on the beat"

### Changed
- Tracker module volume changes now apply instantly (they used to lag by up to a second)

## [2.0.1]

### Changed
- **Spring** pulls objects by their centre again, so they no longer start spinning when you let go (the 1.x behaviour)

### Added
- **Swing** tool (`9`): holds an object by the point you click, so it dangles, swings and spins when thrown (the spinning behaviour Spring had in 2.0.0)
- The tool picker is now a 3 × 3 grid

## [2.0.0] — Remaster

A complete remaster: the 4,200-line `main.rs` is now a modular codebase, with
a new interface and a batch of new features. Every 1.x feature is still here.

### New interface
- Floating HUD with gravity presets and status chips (adapts to narrow windows)
- Settings drawer that slides in when paused (`Space`) and gathers every setting
- Tool picker with animated icons (`Tab`, or keys `1`–`8`) and a tool card with radius / strength sliders
- Redesigned right-click menu, shape spawner, `F1` help and title screen
- Toast notifications, "now playing" pill, bundled Inter font
- Smooth gradient backgrounds and a new **Aurora** background

### New features
- **Scenes**: save / open with `Ctrl+S` / `Ctrl+O` (`.gscene`, images embedded so scenes can be shared)
- **Persistent settings** in `~/.config/gravity_engine/settings.json`
- **Drag & drop** images, audio, playlists or scenes onto the window
- **Screenshots** with `F12` (scene only, without UI)
- **Slow motion** (`[` / `]`) and frame-by-frame stepping (`.` while paused)
- **Pin** objects in place (right-click → Pin); pinned objects can still be dragged
- **Hexagon** and **Capsule** shapes, random colours, hold to keep spawning
- Mouse wheel changes the tool radius (`Shift`+wheel: strength), `Del` deletes the object under the cursor
- `--no-title` command-line flag; `.gscene` and audio files accepted as arguments

### Physics
- Frame-rate independent simulation (it used to run 2.4× faster on 144 Hz screens)
- Tools scale with mass, so small and large objects react the same way
- Spring grabs objects at the exact point you click
- Exact colliders for spawned shapes (balls, capsules, rounded boxes, concave stars)
- A ceiling keeps objects in the arena with reverse gravity; objects lost far off-screen are removed

### Fixes
- Duplicate and resize now work on fetched buttons and spawned shapes
- `G` no longer gets stuck on the Custom background
- Animated GIFs honour frame disposal
- Window tracking runs on a background thread and no longer stalls frames
- Web fetches download in parallel and report errors clearly

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
