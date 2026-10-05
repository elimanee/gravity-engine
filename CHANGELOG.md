# Changelog

Notes for each release. The release workflow publishes the section matching
the tag as the GitHub release description. Versions up to 1.7.0 are
described on the [releases page](https://github.com/elimanee/gravity-engine/releases).

## [2.10.0]

### Added
- **Playable Sonic** (`Shift+O`, or *Sonic* in the settings): `←` `→` run, `↓` crouch / roll, `Space` jump (hold for a higher jump), `↓` + `Space` spin dash, `↑` look up; `Shift+Space` pauses while he is in the scene. He moves after the Sonic Physics Guide: ground speed with the games' acceleration, braking and friction, slope factor, floor sensors that follow walls and ceilings so he runs round loops, slipping off walls when too slow, rolling, the spin dash, variable jumps, air drag and underwater physics
- Sonic pushes objects (curled up he bowls them over), stands and rides on them, bounces off springs (any bouncy surface), slides on ice, is blown away by bombs and can be thrown with the Spring tool. The camera follows him
- Rings to collect, with a counter, and the example scene *Sonic loop*. Sonic and the rings are saved in scenes
- Sounds for jumping, the spin dash, braking, springs and rings
- **Your own Sonic sprite sheet** (*Sonic sprites…* in the settings, or a PNG in `~/.config/gravity_engine/sonic/`): Triangly's Sonic 1 sheet is recognised as it is, other sheets are cut up by a `sheet.json` (see the README). None ships with the program

### Changed
- While Sonic is in the scene, `Space` is his jump button and `←` does not rewind (`Shift+←` still does)

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

## [2.9.0]

### Added
- **Planets**: any object can have its own gravity. Right-click → *Make it a planet*, or the *Planet gravity* slider and *Planet* preset in the properties panel; everything nearby falls towards it and can orbit it. Example scene *Solar system*
- **Weather** (`Shift+H`, or *Weather* in the settings): **rain** falls as real drops that fill containers and put out fires under the open sky; **snow** settles in drifts and melts near fire (also a new grain for the Pour tool); a **storm** brings gusts of wind, heavy rain and **lightning** that strikes the highest thing around and sets it on fire, with thunder. No weather in challenges
- **Night** (`Shift+T`, or *Night* and *Darkness* in the settings): the scene goes dark and only light shows it. Fire, embers, sparks, laser beams, thrusters, hot objects and lightning glow
- **Lamps** (Gadget tool): a bulb that lights all round or a spotlight you aim, in six colours and with an adjustable reach. Objects cast real shadows. Like the other gadgets, a lamp can be on all the time, on an arrow key or on the beat
- **Grappling hook** (Gadget tool): put it on an object and hold its arrow key — the hook shoots at the pointer, catches what it hits and reels in; let go of the key to let go. Set to *always*, it hangs on where it catches; fixed in the world, it is a winch that pulls things in. The knife cuts its rope
- Example scenes *Night lights* and *Grapple swing*; night and weather are saved in scene files

### Changed
- The Gadget tool has five gadgets: laser, thruster, cannon, lamp and hook

### Fixed
- Unit tests could fail now and then when run side by side

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

## [2.8.0]

### Added
- **Gadget tool** (`L`): attach a **laser**, a **thruster** or a **cannon** to an object, or fix it in the world, and drag to aim it. Each gadget works all the time, while an arrow key is held (`↑` `↓` `←` `→`), or on the beats of the music. Right-click a gadget to remove it
- **Lasers**: the beam bounces off mirrors, goes through glass and takes its colour, and pushes and heats what it hits (wood catches fire, ice melts)
- **Mirrors**: a new *Mirror* switch and preset in the properties panel
- **Thrusters** push their object with a jet of flame (and warm what is behind them); fixed in the world, a thruster is a fan that blows on what is in front of it. **Cannons** fire balls, shapes from the spawner, grains or copies of the last image added, at the chosen speed and rate, and recoil
- **Driven motors**: Link tool → Motor → *Drive it with ← →*, then drive with the arrow keys (the wheels coast when no key is held)
- **Three laser challenges** (*Bank shot*, *Periscope*, *Stained glass*): the lines you draw are mirrors; bend the beam into the target
- Example scenes *Laser lab* and *Rocket car*
- Gadgets and driven motors are saved in scenes and undo; the library scrolls with the mouse wheel

### Changed
- 17 tools. Game logos are fetched with `Shift+F` (`L` is the Gadget tool)
- `←` still rewinds, unless something is driven with it; `Shift+←` always rewinds

### Fixed
- Forces applied off-centre (springs, the Swing tool…) left a turning force behind that never went away

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

## [2.7.0]

### Added
- **Fire tool** (`Y`): hold it on something to heat it up. Flammable things catch fire, burn for a while (bigger ones longer), set fire to what they touch — above all what is above them — and crumble to ash; ropes and springs tied to them burn through. Things that do not burn glow and cool down again, **glass** cracks, **ice** melts into water, and the water pool or poured liquid puts fires out. Explosions set things alight. A new *Flammable* switch in the properties panel (the Ice, Glass and Magnet presets turn it off)
- **Jelly** (`U`, or *Jelly* in the settings): squashy blobs that wobble, squash under weight and spring back. Press `U` over any image, shape or drawing (or right-click → *Make it jelly*) to turn it into jelly
- **Cloth** (`Shift+U`, or *Cloth* in the settings): a piece of fabric pinned along its top edge. It drapes over things, flaps in wind zones, tears when pulled too hard, can be cut with the knife and burns away. Right-click an image → *Hang it as a flag* to turn it into cloth. The Spring tool can grab jelly and cloth, and moves cloth pins
- **Physical player** (`Shift+X`, or *Physical player* in the settings): the classic player windows become an object in the world. They fall, tumble, carry what lands on them and react to the tools and explosions, while every button, slider and the playlist keep working. Drag a title bar to throw them; `Shift+X` puts them back on the screen
- Sounds for catching fire, crackling and hissing

### Changed
- 16 tools
- Jelly and cloth are part of undo / redo (they are not saved in scene files)

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

## [2.6.0]

### Added
- **Playlist window** (the *PL* button of the classic player): every song with its length, the current one highlighted. Double-click or Enter plays a song, drag to reorder, Del removes, and the bottom buttons add files, remove, select all, sort and clear. Drop music on it to add it. The playlist is kept between sessions, `.m3u` playlists open too, and it plays through, with **shuffle** and **repeat** buttons
- **Equalizer** (the *EQ* button): ten bands from 60 Hz to 16 kHz and a preamp, ±12 dB, applied to all music (files, radios and tracker modules), with presets (Rock, Pop, Dance, Classical, Full bass, Full treble, Headphones…). Both windows use the skin's `pledit` / `eqmain` sheets and colours, stack under the player like Winamp's, and have a built-in look without a skin
- **Rewind** (hold `←`): the last 8 seconds of the simulation play backwards at double speed; let go to carry on from there
- **Knife tool** (`C`): drag across the scene to cut ropes and springs, pull hinges, motors and glue apart, and slice objects in two (images, shapes and drawings)
- **Ragdolls** (`O`, or *Ragdoll* in the settings): floppy characters with limited joints. Press `O` over an image and it becomes the ragdoll's head. The knife cuts their joints

### Changed
- 15 tools
- Songs play once and the playlist moves on (tracker modules too); with *Repeat* off, playback stops after the last song

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

## [2.5.0]

### Added
- **Classic player** (`X`): a Winamp-style player window that drives the built-in music player — previous / play / pause / stop / next / eject (load), elapsed time, scrolling title, spectrum analyzer, volume slider and a seek bar. Drag it by its title bar, double-click the title bar for normal or double size, and click × to hide it
- **Winamp 2 / Audacious skins**: the player uses classic `.wsz` skins (BMP or PNG sprite sheets, `viscolor.txt`, and Audacious `skin.hints` layouts, including wider windows). Installed Audacious skins (`/usr/share/audacious/Skins`, `~/.local/share/audacious/Skins`), Winamp's `Skins` folder on Windows and `~/.config/gravity_engine/skins/` are found automatically; cycle them in the settings (*Audio → Skin*), pick a file with *Load a skin…*, or drop a `.wsz` on the window. Without any skin the player has a built-in look
- Seeking in songs, next / previous track in playlists (next / previous pattern in tracker modules) and a stop button

### Fixed
- Song lengths are read from the file itself (the audio library reported them several seconds too long)

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

## [2.4.0]

### Added
- **Challenge editor** (`Shift+E`, or Library → *My challenges* → *New challenge*): turn any scene into a level. Place the golden ball and the goal, set the ink, build the course with every tool, then *Test* it — a level can only be saved once you have solved it. Saved levels are `.gchallenge` files in `~/.config/gravity_engine/challenges/`; share them, drop them on the window or open them with `Ctrl+O`
- **Ten new challenges** (fifteen in all): Funnel, Bouncer, Tailwind, Lift, Smash, Keyhole, Wrong way, Splash, Wormhole and Stop sign
- **Stars**: ★★★ for solving a challenge with half the ink or less, ★★ up to 80 %, ★ otherwise. Your best score is shown in the library
- **Sound effects**, synthesised on the fly: hits that depend on size and bounciness, breaking glass, explosions, pops, snaps and a fanfare when you win. Volume and on/off in the settings (*Audio*)
- **Slow motion** for a moment when something shatters, a bomb goes off or two objects hit very hard (can be turned off in *Effects*)
- **Camera**: zoom with `Ctrl`+wheel (at the pointer) or `+` / `-`, `Home` to fit, pan with the middle mouse button. *World size* in the settings makes the arena up to three times the window
- **Select tool** (`S`): click, `Shift`+click or drag a box to select objects, then move them together, duplicate (`Ctrl+D`), copy and paste (`Ctrl+C` / `Ctrl+V`, at the pointer), pin, delete, or **glue** them into one rigid group (`Ctrl+G`). `Ctrl+A` selects everything
- **Glue** links (Link tool → Glue) weld two objects together
- **Pour tool** (`K`): hold to pour **sand** that piles up, a **liquid** that flows and levels out, or bouncy **beads** — up to 1600 grains, pushed by the tools, wind, float zones and portals. Right-drag erases, the tool card clears them all

### Changed
- Bouncy materials (*Rubber*, bounce ≥ 0.9) now make whatever hits them bounce, like a trampoline
- 14 tools; the tool picker is a 4 × 4 grid
- `Ctrl+O` opens scenes and challenges

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)

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
