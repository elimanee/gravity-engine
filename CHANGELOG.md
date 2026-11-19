# Changelog

Notes for each release. The release workflow publishes the section matching
the tag as the GitHub release description. Versions up to 1.7.0 are
described on the [releases page](https://github.com/elimanee/gravity-engine/releases).

## [Unreleased]

### Added
- **Mechanisms** (Gadget tool, its second row): a **button** pressed by whatever rests on it (an object, grains, a ragdoll, the character), a **sensor** whose invisible beam something crosses, a **target** that lights up where a laser's beam reaches it (and stops the beam), and a **timer** on half of each period, each sending a **signal** on one of four coloured channels: while it is on, from the first time on for good, or switched at each press. Lasers, thrusters, cannons, lamps, hooks, the new **piston** (it slides the object it is on out and back: a door, a lift) and **motors** can work on a signal. With the Gadget or Link tool out, dashed wires in each channel's colour join what sends to what works on it, lit while the signal is on. Saved in scenes and undone like the rest; example scene *Chain reaction*
- **The 3D mode** (Library → Sonic → *3D run* / *3D zone*, or `--3d=run` / `--3d=zone`): the characters in a 3D world with the 2D game's physics carried over (acceleration, braking, friction, the jump, rolling, the spin dash, slopes, and the momentum rules), running along whatever is under their feet, round loops and up walls when fast enough, turning smoothly so bends keep their speed; each character's move (Tails' flight, Knuckles' glide, Amy's hammer, Sonic.exe's teleport). A camera that follows (turned with `Q` / `E`, right-drag or the shoulder buttons), the stick going the way it looks; round a loop it stands off to the side so the whole way round shows, and the view widens with speed. `P` pauses (with the controls), which also show at the start
- **The 3D run**: an endless road of grass on earth over the sea, a new one every run, made as you go: bends that lean, hills, ramps over gaps, loops that come out beside the way in, springs up to a higher road, dash panels, lines of rings, badniks; falling off ends the run, and the longest is kept as your record. Holding forward alone follows the road (it eases you back towards the middle until you steer yourself); the dash panels and the springs reach right across it, and every 500 m is marked. Off the edge, the view stays up by the road and watches the character fall into the sea, with a splash
- **The 3D zone**: hills to roam, ringed by mountains, with a loop, ramps, platforms on pillars reached by springs, rings, badniks, trees, and things that fall, roll and get knocked over (rapier3d): a stack of crates, barrels and a big ball; `N` drops another, `B` blows up everything near
- Rings, springs, dash panels and badniks in 3D: rings lost in a hit bounce about and can be caught again, a hit with none is the end (the run) or back to the start (the zone), badniks burst when rolled or jumped into
- **3D models**: the characters, badniks, rings and springs drawn from a Sonic Robo Blast 2 model pack (MD3, with its animations, and the character's colour put on through the `_blend` maps as SRB2 does) put in `sonic/` (`models.dat` and `models/`); Sonic.exe from a pack's `EXE_` model (a model of poses, like 2011X's) or Sonic's in darker colours. Everything missing is built from simple shapes. Nothing of the models is shipped
- **Characters of your own** (Library → Sonic → *Characters*, or `--characters`): a character editor over the whole window. A character has a name, the look of a built-in character (its drawing, size and sheet layout) in colours of your choosing (body, belly and muzzle, shoes), its physics as multiples of Sonic's (top speed, acceleration, jump, gravity, air control), rolling and the spin dash or not, and up to eight moves, each on the jump button in the air or on `X`, with its setting. A press tries them in order, each once a jump, so moves on the same button chain. A preview plays each move; *Try in 2D* (on the test track) and *Try in 3D* (in the zone) play it as edited, and `Esc` comes back. Kept in `characters/` (one `.json` each); chosen with *Yours* under *Play as* in Library → Sonic, or `Shift+N`, and played everywhere the others are, in 2D and 3D
- **New moves** for them: a double jump, an air dash, a homing attack (at the nearest badnik or monitor ahead; hitting it bounces up and makes the moves ready again), a stomp (straight down; landing breaks the badniks on the ground close by), and something to throw that breaks the first badnik or monitor it meets (in 3D it knocks things over too); and the built-in characters' moves, which are now blocks like these: flying (for so long), gliding (climbing walls or not), the hammer, the teleport (so far)
- **More moves** for them, in 2D and 3D: Sonic Mania's **drop dash** (jump again in the air and hold it: landing charged, off rolling), Sonic CD's **peel out** (hold `X` standing, let go to be off running), Sonic 3's **insta-shield** (a flash that reaches further, breaks what it touches, spiky badniks included, and lets nothing hurt them) and **bounce** (straight down and back up higher than a jump, again and again), a **wall jump** (a kick off a wall in the air; away from walls the press goes to the next move), and a **boost** (hold `X`: twice the top speed, breaking badniks and walls, with a gauge that fills again slowly). Each has its setting, its animation in made sheets and its look when the game draws them. And two switches: a **ring magnet** (rings close by fly to them) and **breathes underwater**
- A made-up character's **sprite sheet**: *Make from drawing* makes one from the game's drawing of them in their colours, with every animation their moves need (`custom-<name>.png` and its regions in the Sonic folder), to paint over in the sprite region editor, which lists them beside the built-in characters; *Choose a picture…* makes any picture their sheet. In 3D, any model of a model pack, by its name in `models.dat`, coloured like them
- **NinjaCakes' Green Hill-style tileset** (725 × 617) is recognised as a zone sheet (`zone.png`, put in place by *Sonic sprites…*) and used as a tileset: the ground and the ledges in the air are built from its 8 × 8 pieces on a grid, column after column as its pieces go side by side (tufts and stretches of tall grass sticking up, the grass into the earth along the top, its earth down to the bottom), in the sheet's own pixels so they stay square, following the slopes; crumbling ledges, walls to roll through and rocks are drawn from it too, and so is a **backdrop** behind the levels by day: its sky, clouds, mountains and islands each scrolling at their own pace, and the sea shimmering below the horizon (and down the pits). Its **palms, sunflowers, totem poles and plants** stand here and there on level ground, behind the grass, at the same places every time
- **The terrain editor** (sprite region editor → Zone → *Terrain…*): how a tileset's pieces make the ground, for that tileset or any other. Its columns, side by side in order, and each one's pieces from the top down (tall grass, a tuft, the top, the earth, the piece under it) are chosen by clicking pieces on the sheet, on a grid set to its pieces (size, step, start); columns are copied, removed and moved, tall grass shows in one stretch in so many, and the ground they make is shown as they change. Kept as `zone.terrain.json`; the levels are built with it at once
- Zone sheets can have that backdrop (`sky`, `clouds`, `hills`, `islands`, `sea`) and scenery (`tree`, `sunflower`, `totem`, `plant`): new tiles in the sprite region editor's **Zone** tab. A wall's picture is repeated over the wall, and a rock's is made as big as the rock
- **Random acts** (Library → Sonic → *Random act*): a new act made each time from a seed, from the start through a dozen sections and star posts to the goal. Its ground is generated as it goes, not put together from ready-made blocks: flats, slopes, hills and dips of any length and height, drifting up and down. Between them, something chosen at random and never the same as the last few: pits of any width (landing higher, lower or level) or runs of them, bridges, swinging platforms, stepping stones in the air, crumbling ledges, spike logs, cliffs with a spring, drops, loops, walls and rocks, dips with spikes, a dash panel before a long jump, and routes on two levels (a spring up to ledges with the monitors, over ground below with the badniks). Every pit can be crossed and every slope run up; more badniks towards the goal. Drawn with the zone's tiles when there is a zone sheet. *Edit* opens the last one played in the level editor, to change it and keep it in *My levels*

### Changed
- How a **motor** turns (Link tool → Motor) is a row of choices: by itself, while `←` or `→` is held, or on one of the four signals, in place of the *Drive it with ← →* switch
- Tails' flight, Knuckles' glide, Amy's hammer and Sonic.exe's teleport are **blocks of a kit**, as a made-up character's moves are; they play as before. In 3D, a made-up character with a ground move on the action button (the hammer, a throw, the teleport) does it instead of rolling and the spin dash, as Amy swings her hammer
- The sprite test track (the region editor's *Try*) has two more badniks, a crawler near the start and a flyer over the hill, to try moves on
- The lives counter's face is the character's colour when there is no sprite of them
- Opening an example or a challenge starts it **by day and clear** unless it is meant otherwise: after *Night lights*, the night and the rain stayed on in the next ones
- Messages no longer slide **under the level editor's and the challenge editor's bars** (they were half hidden): they show below them
- The characters drawn by the game have a **hurt pose**, thrown back with their arms and legs flung out, and a crossed-out eye when they are done for (they used to fall through the floor in their jumping pose); sheets made by *Generate…* get it too
- In 3D, Amy's swing shows her **hammer**; out of view, the ground and things are not drawn (the open zone draws about a third faster)
- `--benchmark` has two 3D scenes: the run and the zone with things dropped and blown up
- **Stuck at a wall that breaks or at a monitor** (pushing at it and getting nowhere), the game now says how to get through, once: roll into the wall, jump on the monitor
- In the random acts and the endless run, **spikes are never where the character comes down** any more: they keep well clear of the start of a stretch of ground (the foot of a drop, the far side of a pit), where they could not be seen in time
- **Edit sprite regions** for a character with no sprite sheet yet asks what to do instead of only opening a file chooser: *Make one* from the game's own drawing of them (the editor then opens on it, ready to lay out again or paint over), or *Choose a picture…*. The sheet generator could not be reached before without a sheet to start from
- The **endless run** is made by the same generator as the random acts, as Sonic goes, **a new course every run**: real ground generated without end, pits to get over (falling in one ends the run), routes on two levels and the zone gimmicks, instead of stretches laid on a flat floor
- The best run's ghost (which raced you on the same course every time) is now a **flag** on the course where your record is, with *Past your record!* when you get beyond it; `ghost.json` is not used any more
- Crumbling ledges wait half a second before they start to go, and go a piece at a time more slowly, so running on keeps ahead of them
- The level editor shows the sprite sheets and the zone's tiles from the start, before anything is played
- **Momentum**: holding the way they go, the characters keep their speed over a jump (the air drag near the top of a jump only slows them when you let go: a jump at speed used to land a third slower) and while rolling (it used to slow to a crawl in a few seconds); Amy swings her hammer on the run instead of stopping, and can only keep going or brake meanwhile
- At night in the endless run (the Twilight zone), **lanterns** stand either side of every pit and the character's light reaches ahead, so the pits show before you are on them
- In the random acts and the endless run, badniks are only put where there is room: no more walker or flyer on a short bit of ground between two pits, where the character lands (a hit there threw them back into the pit)

### Fixed
- The ground, its grass and the ledges laid out over a tileset's pieces drawn apart (8 × 8 pieces a pixel apart) are joined without the gaps: the ground had holes and lines through it
- The ground's solid shape now follows its drawn top exactly: it was cut into convex parts only approximately, and as the ground goes all the way down that filled its dips (up to 20 px), so the character ran on air at the bottom of a slope and could not get on
- A character coming down on the corner of the ground with their middle just above its top could slip into the ground and fall through it; they now land on the edge (and, lower down, the side stops them)
- In the infinite world, pinned things moved twice as far as everything else whenever the world was brought back under the view: in the endless run the ground vanished from under the character after 200 m, and in the sandbox pinned objects jumped

## [3.5.1]

### Fixed
- The Linux build runs on older Linux again (glibc 2.35 and up: Ubuntu 22.04, Debian 12, Fedora 36…). 3.5.0's was built on the newest Ubuntu and would not start elsewhere (`GLIBC_2.43 not found`). The builds are now made on Ubuntu 22.04, and checked so that this cannot come back unnoticed

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (glibc 2.35 or newer; needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/ (the sprite region editor needs the desktop version, like your own sprite and zone sheets)

## [3.5.0]

### Added
- **Zone gimmicks** in the acts and the level editor (*Gimmick*, click again for the next): bridges of logs that dip under the character, platforms swinging on chains (they carry the character along), ledges that crumble once stood on (back a while later), walls broken by rolling into them, logs with spikes going round them, and rocks. In Meadow's acts, and every one of them behind the start of the sprite test track
- **The zone's sheet** (`zone.png`): the acts' and your levels' ground drawn with its tiles (the grass along the top, blades over the sky, the ground repeated all over the world so the pieces meet), the ledges in the air, and every gimmick. The Green Hill Zone objects sheet from The Spriters Resource (864 × 707) is known as it is; any other is laid out in the sprite region editor's new **Zone** tab (`--sprite-regions=zone`), its tiles listed instead of animations, and *Try* plays the test track with them. Without one, everything is drawn as before
- **New animations** a sheet can have, played by the game when it has them (else the character does what it did before): `full_speed` (flat out on the ground, faster than running), `fall` (falling without being curled up), `land` (a moment on touching down from a fall), `balance` (teetering at the edge of a drop, facing it), `breathe` (catching an air bubble), `drown`, `transform` (turning super) and `victory` (past the goal post). In the editor's list with what each is, and in the game's drawing for the generator
- **Sheet generator** in the sprite region editor (*Generate…*, `G`): makes a sheet laid out the way fans lay theirs out (a banner with the character's name, the pictures' origin, credits and palette; an animation a row, each picture in its own box on a dark page), with its regions beside it so it works as it is. From **the game's drawing** of the character (every animation the game asks of them, two pixels to one of the game's: to play, or to paint over) or from **the pictures its sheet uses**, tidied. Kept with the screenshots; *Use it for … now* makes it the character's sheet at once. Clicking a character with no sheet offers it too
- **Paint** (`P`) in the editor: a pencil and an eraser (1 to 16 pixels), a fill and an eyedropper (or Alt-click) on the sheet itself, with a colour picker and the sheet's own colours; strokes undo like any change, and *Save* writes the sheet again in place (as a PNG)
- **Colours** in the editor: change a character's colours without touching the sheet (pick them from a list of its colours or on the sheet; lossy sheets take the shades close to them), and give them **Super colours** of their own, used by the game when they turn Super instead of the golden glow. Kept with the regions (`recolour`, `super`); undone like any change
- **Grid** (`R`) in the editor: drag a box over pictures laid out in rows and columns; their columns and rows are counted and the box fits itself around them (Alt: as drawn), then *Add* (`Enter`) adds every cell with a sprite in it, with the character at the same place in each, so the animation does not wobble
- **Import…** in the editor: pictures from files (PNG, WebP, BMP, JPEG, or every frame of an animated GIF, its timing kept as holds and pace) added to the animation, below the sheet; *Save* writes the sheet again with them. Dropping pictures on the editor does the same
- **GIF** (`E`) in the editor: the animation shown as an animated GIF, three times as big on a transparent background, at its pace, kept with the screenshots
- **Turns** for each animation: with the ground (as before), by 45° as the Mega Drive games drew it, or never (kept as `turn` in the regions)
- **A layer in front**: besides the pieces drawn behind (Tails' tails), any animation can have pieces drawn in front of the character (dust, sparks…), playing on their own; *Body* / *Behind* / *Front* (`B`) for every character. In the generator's sheets, GIFs and the regions (`front`)

### Changed
- *Open…* (formerly *Open a sheet…*) also brings in the regions kept beside a picture (`<name>.regions.json`), so a generated or shared sheet comes in laid out
- The **endless run** has nothing that falls or rolls about any more: its stretch of crates (loose boxes that tumbled and got in the way) is now two walls to roll through and a rock to jump

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/ (the sprite region editor needs the desktop version, like your own sprite and zone sheets)

## [3.4.2]

### Added
- **Repeated runs** in the sprite region editor: choose pictures that follow one another and *Repeat* them 2 to 16 times before the animation goes on, as many runs per animation as needed (like the "Loop ×8" marks on sheets), besides its loop point. A bar along their tops and `↻×8` show them; saved as `repeats` in `.regions.json`

### Changed
- In the sprite region editor, Shift-click now chooses a run of pictures (from the selected one); Ctrl-click still adds one

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/ (the sprite region editor needs the desktop version, like your own sprite sheets)

## [3.4.1]

### Added
- **Loop points** in the sprite region editor: *Loop here* (`L`) on a picture makes the ones before it an opening, played once, and the animation loops from there (`↻` marks it; bars under the pictures show the opening and the loop). In the game the opening plays again whenever the character starts the move; saved as `loop_from` in `.regions.json`
- Sonic.exe's chase is timed from when he appears

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/ (the sprite region editor needs the desktop version, like your own sprite sheets)

## [3.4.0]

### Added
- **Any picture as a character's sheet**, in the sprite region editor: *Open a sheet…* makes any PNG, WebP, GIF, BMP or JPEG a character's sheet (kept as a PNG under its name), and a sheet the game does not know opens empty, to be laid out there. An eyedropper (*Pick*, `I`) cuts out its background colours, *Soft edges* cuts them by closeness (lossy pictures), *Scale* sets its pixels per game pixel, and a checkerboard shows through what is cut out
- **Sprites found by themselves** once the background is gone (Shift shows them): **Shift-drag a box** to add every sprite in it, row by row
- **Animation settings**: a *Pace* per animation (the games' own, following the character's speed, or 4 to 30 pictures a second), *Play once* (stops on the last picture; it starts again with the move), and per picture *Hold* (2 to 8 times as long, `-` / `=`) and *Mirror* (`H`)
- **Several pictures at once**: Shift- / Ctrl-click or `Ctrl+A` to choose them, then drag, nudge, fit, hold, mirror, duplicate or delete them together; *Same centre* (`C`) lines them up on one picture; *Copy* / *Paste* (`Ctrl+C` / `Ctrl+V`) take pictures to another animation
- **Try** (`T`): play the character as edited, saved or not, on a test track (stand, crouch, spin dash, push, a hill, a loop, a badnik, a spring, a wall to climb); `Esc` comes back to the editor and then to your scene

### Changed
- The `.regions.json` files can hold the new settings (`flip`, `hold`, `fps`, `once`, `background`, `soft`, `scale`); those written by 3.3.0 still read

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/ (the sprite region editor needs the desktop version, like your own sprite sheets)

## [3.3.0]

### Added
- **Sprite region editor** (*Edit sprite regions…* in the settings, or `--sprite-regions[=CHARACTER]`): on a character's sprite sheet, change which pictures each animation uses (idle, walk, spin dash…) and where the character stands in them. Draw, move and resize the pictures on the sheet, double-click a sprite to add it, drag the character's centre (its body and the ground are shown), fit a picture around its sprite, reorder, duplicate and delete them, with undo, an animated preview and onion skin. Saved beside the sheet as `<character>.regions.json` (only what changed, for that sheet only), and the game draws from it at once

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/ (the sprite region editor needs the desktop version, like your own sprite sheets)

## [3.2.0]

### Added
- **Halloween Night**: a new act after Harbour, a graveyard at night with gravestones, jack-o'-lanterns that light the dark and its own eerie song, where **Sonic.exe hunts you**: he appears behind in a burst of static, flies after you through everything a little slower than a run (and faster as time goes on), and when he catches you he laughs and you lose your rings or a life; then he vanishes and comes back from behind. Invincible or super, he cannot touch you
- **Sonic.exe as a fifth character** (`Shift+N`, *Exe* under *Play as*): runs like Sonic; jump again in the air to teleport forward, once a jump, never through a wall. Drawn with shapes, or from a Sonic.exe sheet in the style of Sonic 1 (691 × 677) kept as `exe.png`, which also gives the hunter his run and laugh
- DeltaConduit's Amy sheet in the style of Sonic 1 (583 × 570) is recognised as Amy's (`amy.png`), alongside CartoonsAnimate22's
- Level editor: *Pumpkin* (jack-o'-lanterns) and *Exe* (Sonic.exe hunts the player)
- Sounds: broken static and a laugh

### Fixed
- In the web version, text needing new letters could be drawn for a frame from a texture already deleted (errors in the browser's console): the usual letters are now ready from the start

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/

## [3.1.0]

### Added
- **Web version**: [play in the browser](https://elimanee.github.io/gravity-engine/). The same program compiled to WebAssembly, published on GitHub Pages by the new *Web version* workflow (`web/build.sh` builds it locally). Settings, levels, challenges and the endless run's ghost are kept in the browser's storage; what needs files or the network (opening images, music, scenes and skins, the fetchers, screenshots, GIFs, sprite sheets) says it needs the desktop version
- **Harbour zone**: three acts after Meadow, over the sea. Act 1 along the docks, Act 2 through a flooded tunnel, Act 3 against the **anchor crane**, a flying crane whose anchor swings under it and slams down, sending shock waves along the ground
- New badniks: **fish** leaping out of the water, **gulls** that hover, cry out and swoop down, and **puffers** whose spikes hurt even when you are curled up
- **Air under water**: chimes as it runs low, a countdown over your head with its own music, drowning; **vents** let out big bubbles to breathe
- **Special stages**: reach a goal with 50 rings and a giant ring takes you, after the tally, to a Blue Spheres stage on a checkered globe: turn at the crossings, catch every blue sphere, avoid the red ones (or jump over them), with rings, bumpers and springs. Seven stages, one **chaos emerald** each, shown in Library → Sonic
- **Super Sonic**: with all seven emeralds and 50 rings, jump again in the air to turn super (any character): golden, faster, higher jumps, unhurt by badniks and bosses, a ring a second
- **Music for Sonic's mode**, made by the program (no sound from the games): a chiptune song per zone, the boss's, invincibility's, Super Sonic's and the special stages', and jingles for an act cleared, a 1-up, Game Over, an emerald and the drowning countdown. *Sonic's music* and *Music volume* in the settings; it keeps quiet while your own music plays
- New sound effects: air chimes, a breath, turning super, the giant ring's warp, blue spheres and bumpers
- Level editor: one *Badnik* button for all six badniks, both bosses, and air vents
- `--act=N` and `--special-stage=N` open an act or a special stage straight away
- CI lints the web version

### Changed
- The Meadow acts have more rings (enough for the giant ring)
- After losing a life you are safe for a moment, so nothing waiting at the star post takes the next one at once

### Downloads
- `gravity_engine-linux-x86_64` — Linux build (needs `libopenmpt` and the usual X11 / ALSA libraries)
- `gravity_engine-windows-x86_64.zip` — Windows build: unzip and run `gravity_engine.exe` (keep the DLLs next to it)
- Or play it in the browser: https://elimanee.github.io/gravity-engine/

## [3.0.0]

### Added
- **Playable Sonic** (`Shift+O`, or *Sonic* in the settings): `←` `→` run, `↓` crouch / roll, `Space` jump (hold for a higher jump), `↓` + `Space` spin dash, `↑` look up; `Shift+Space` pauses while he is in the scene. He moves after the Sonic Physics Guide: ground speed with the games' acceleration, braking and friction, slope factor, floor sensors that follow walls and ceilings so he runs round loops, slipping off walls when too slow, rolling, the spin dash, variable jumps, air drag and underwater physics
- Sonic pushes objects (curled up he bowls them over and smashes breakable ones), stands and rides on them, bounces off springs (any bouncy surface), slides on ice, is launched by conveyors like dash panels, is blown away by bombs and can be thrown with the Spring tool. The camera follows him
- Rings to collect, with a counter, and the example scene *Sonic loop*. Sonic and the rings are saved in scenes
- **Tails, Knuckles and Amy** (`Shift+N`, the controller's Select, or *Play as* in Library → Sonic): Tails flies (tap `Space` to flap, tired after 8 s), Knuckles glides faster and faster, climbs walls, pulls himself over ledges and slides on landing, Amy swings her hammer (`X`) and spins with it in the air
- **Sonic's game mode**: three *Meadow* acts in Library → Sonic, with badniks (a crawler, a flyer that shoots, a crab that lobs shots; chained hits score 100, 200, 500, 1000…), item monitors (10 rings, shield, invincibility, speed shoes, extra life), spikes, bottomless pits, star posts, the goal signpost and a boss who takes eight hits in an arena the view locks on. Hit with rings, Sonic loses them in a circle and can catch some back; 100 rings give an extra life; three lives, then *Game Over*. Score, time, rings and lives on screen and the end-of-act tally with time and ring bonuses
- **Level editor** (Library → Sonic → *New level*): place the start, rings (drag for a row), badniks, monitors, spikes, star posts, the goal, the boss, springs, ground, ledges and loops in the infinite world, *Play* to try the level and *Stop* to go back, *Save* to keep it in `~/.config/gravity_engine/levels/`; saved levels are listed in Library → Sonic to play or edit
- **Infinite world** (*World size* → `∞` in the settings): a floor that goes on forever with no walls or ceiling; the world is moved back under the view as it travels so positions stay precise. Weather and water follow the view
- **Endless run** (example scene): Sonic in the infinite world, with hills, loops with dash panels, ramps, springs, platforms, crates and rings made ahead of him and cleared behind him, through five zones (Meadow, Twilight at night, Snowfield, Lagoon with water, Storm peak) with their own skies, and more badniks and spikes the further he gets. One life; the best run is kept as a ghost that races you on the same course
- **Open** border mode: no walls and no floor, what falls off is gone
- **Game controllers** (Linux joystick devices, XInput on Windows): stick or d-pad, A / B / Y jump, X action, Select changes character, Start pauses; rumble on Windows
- **Your own sprite sheets** (*Sonic sprites…* in the settings, or PNGs in `~/.config/gravity_engine/sonic/`): Triangly's Sonic 1 sheet is recognised as it is, other sheets are cut up by a `sheet.json`; the Sonic 3 Tails, Sonic 3 & Knuckles Knuckles and CartoonsAnimate22's Amy sheets, the Sonic 1 *Common Objects* sheet and the Sonic 3 *Monitors* sheet are recognised by their size (see the README). None ships with the program: everything is drawn with shapes otherwise
- At night Sonic carries a soft light, rings gleam and the endless run's ghost glows
- `--benchmark`: five stress scenes run at a fixed 60 Hz, then a report of the frame times and where they go (physics, each part of the drawing, waiting for the screen)
- CI uploads Linux and Windows test builds for every run

### Changed
- While someone plays, `Space` is the jump button, `X` the action button and `←` does not rewind (`Shift+←` still does)
- Objects and grains outside the view are not drawn (faster in big and infinite worlds)

### Fixed
- Sonic could be pushed through the floor by bits of smashed crates (or a crate fallen on him)

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
