# Changelog

Notes for each release. The release workflow publishes the section matching
the tag as the GitHub release description. Versions up to 1.7.0 are
described on the [releases page](https://github.com/elimanee/gravity-engine/releases).

## [2.3.0]

### Added
- **Challenges** (`E` → Challenges): five puzzles where you draw lines to get the golden ball into the goal, with a limited amount of ink. `Space` releases the ball, `R` retries, `Enter` goes to the next one. Solved challenges are remembered
- **Example scenes** (`E` → Examples): Newton's cradle, domino run, hill-climbing car, wrecking ball, pool party, magnet field, parcel factory and portal loop
- **Breakable objects**: tick *Breakable* in the Properties panel (or use the *Glass* preset) and the object shatters into real pieces when hit hard enough — by a fall, a collision or a bomb
- **Particle effects**: sparks and dust on hard impacts, splashes in the water, debris when something shatters, confetti when you solve a challenge (can be turned off in the settings)
- **Motors** (Link tool → Motor): a hinge that turns by itself, with an adjustable speed — click a wheel on a body to build cars, mills and conveyors. Hinges and motors snap to the centre of round objects
- **Zone tool** (`Z`): drag rectangles of **wind** (four directions, adjustable strength), **float** (objects lose their weight) or **portals** (draw an entrance then an exit; objects jump between the two)
- **Magnets**: a Magnet strength in the Properties panel; magnets attract each other, negative ones repel
- **Conveyor surfaces**: a Conveyor speed in the Properties panel makes an object's surface carry what touches it

### Changed
- The Properties panel has *Breakable*, *Magnet* and *Conveyor* settings and two more presets (*Glass*, *Magnet*)
- Scenes (format version 3) save motors and zones; older scenes still open

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

## [2.2.0]

### Added
- **Draw tool** (`0`): draw with the mouse and your stroke becomes a physics object — a loop becomes a filled shape, a line becomes a plank. Drawings can be pinned in place (toggle in the tool card, `Shift` inverts) to build ramps, funnels and courses
- **Link tool** (`J`): tie objects together with a **rope**, a **spring** or a **hinge**, or hang them from the background. Right-click a link with the tool to remove it, or use *Detach links* in the object menu
- **Undo / redo** (`Ctrl+Z`, `Ctrl+Y` or `Ctrl+Shift+Z`) for every edit: adding, drawing, linking, deleting, resizing, pinning, clearing, opening a scene…
- **Object properties** (right-click → *Properties…*, or `I`): bounce, friction, mass and gravity per object, with *Rubber*, *Ice*, *Heavy* and *Balloon* presets
- **Water** (`H`): a pool with buoyancy and drag — light objects float, heavy ones sink — and waves that react to splashes and to the music's bass. Level and density in the settings drawer
- **GIF recording** (`F11`): records the scene (without the interface) to `~/Pictures/gravity_engine/`, up to 30 seconds
- **Windows build** attached to releases, and window shake now works on Windows too

### Changed
- The tool picker is now a 4 × 3 grid
- Scenes (format version 2) also save drawings, links, object properties and water; older scenes still open

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

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
