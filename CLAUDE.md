# Gravity Engine — notes for Claude

A 2D physics sandbox (Rust, macroquad 0.4, rapier2d) that grew a Sonic game
(acts, endless run, special stages, level and sprite editors) and a 3D mode
(rapier3d). Native on Linux / Windows, and a web version (wasm32).

Two programs on one library (`src/lib.rs`): the game (`gravity_engine`,
`src/main.rs`) and **the studio** (`gravity_studio`,
`src/bin/gravity_studio.rs`: the character editor and the sprite region
editor, opened from a page of their own). In the game, their buttons open
the studio found beside it (`src/app/studio.rs`); the web version only
builds the game and keeps the editors in it.

## Working with the owner

- **Talk to the user in French.** The repository itself (code, comments,
  README, CHANGELOG, commit messages) stays in English.
- Show progress with screenshots as you go (the owner likes to see the
  game, not only read about it).
- Don't open a pull request or make a release unless asked.
- Leave the README's *Usage of AI* section as the owner wrote it.

## Never commit other people's assets

The owner plays with Sega / fan sprite sheets (Sonic, Tails, Knuckles, Amy,
Sonic.exe, the Green Hill Zone objects sheet, NinjaCakes' Green Hill-style
tileset…) and 3D model packs (SRB2 MD3
packs such as Jeck Jims', which also hold Mario and others; 2011X's
Sonic.exe "RAGE" model). **None of them may go into the repository, its
releases, its docs or its tests**, not even resized or cropped.

- The game reads them at run time from `~/.config/gravity_engine/sonic/`
  (sheets, `zone.png`, and for 3D a pack's `models.dat` + `models/`).
- Everything they draw has a built-in stand-in (shapes, the game's own
  drawing, generated sheets); the game must work fully without them.
- Screenshots in `docs/screenshots/` are made **without** them (a scratch
  `HOME` with no `sonic/` folder).
- To try them, copy them into a scratch config folder only (see *Trying it*),
  never under the working tree. Before every commit, check
  `git diff --cached --name-only` for `.md3`, `_blend.png`, `models.dat`,
  sheets and archives.

## Checks (all must pass before a push)

CI (`.github/workflows/ci.yml`) uses the newest stable Rust. The machine's
default `stable` can be older, with fewer clippy lints, so run the checks
with the newest toolchain installed (e.g. `cargo +1.99.1 …`):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings   # the web version
cargo test
```

`rustfmt.toml`: 120 columns. The web build has no file system, threads or
native dialogs: anything that reads the player's files goes through
`storage.rs` / `config.rs` or is skipped under `cfg(target_arch = "wasm32")`.

## Trying it in the real app

Tests are not enough for anything drawn or played: run it. With no display,
use Xvfb + xdotool + ImageMagick's `import`, and a scratch `HOME` so the
owner's settings and files are never touched:

```bash
Xvfb :99 -screen 0 1100x720x24 &
DISPLAY=:99 HOME=/path/to/scratch-home target/debug/gravity_engine --no-title --3d=run &
DISPLAY=:99 xdotool key e            # …drive it
DISPLAY=:99 import -window root shot.png
```

Shortcuts: `--no-title`, `--act=N` (1–7), `--special-stage=N`,
`--3d=run` / `--3d=zone`, `--benchmark`, `--verify-challenges`; the studio:
`target/debug/gravity_studio --characters` or
`--sprite-regions[=sonic|tails|…|zone|custom:NAME]` (the game's same
options open the studio). The settings file is
`$HOME/.config/gravity_engine/settings.json` (`"character": "exe"`,
lowercase; `"show_title": false`).

## Test builds and releases

- **Test build** of a branch: run the *CI* workflow (`ci.yml`,
  workflow_dispatch) on it; its artifacts `gravity_engine-linux-x86_64` and
  `gravity_engine-windows-x86_64` (each with the game and the studio) are
  the builds to give the owner. Runs can
  sit "queued" for a while on GitHub's side; relaunch once if one never
  starts.
- **Release** `vX.Y.Z`: set `version` in `Cargo.toml`, move the CHANGELOG's
  `## [Unreleased]` entries to `## [X.Y.Z]` (with a *Downloads* list like
  the earlier ones), update the README's *What's new* and version badge,
  then run `release.yml` with the tag. It refuses a tag that does not match
  `Cargo.toml` and publishes the matching CHANGELOG section as the notes.
- Linux builds are made on Ubuntu 22.04 and checked to need glibc ≤ 2.35;
  keep it that way (3.5.0 built on the newest Ubuntu would not start on
  older systems).

## Docs

Every user-visible change goes in `CHANGELOG.md` under `## [Unreleased]`
(*Added* / *Changed* / *Fixed*), and in the README when it is a feature or
a control. Plain, concrete English in the existing voice; describe what the
player sees, not the code.

## Where things are

- `src/app/` — the app's frame loop and every mode's glue (`sonic.rs`,
  `game.rs`, `endless.rs`, `special.rs`, `three.rs`, `regions.rs`, the
  editors). Full-screen modes (`special`, `three`, `regions`) take over
  `App::frame` early.
- `src/sonic/` — the 2D characters, after the Sonic Physics Guide: speeds
  in guide px per 60 Hz frame (acc 0.046875, dec 0.5, frc 0.046875, top 6,
  air 0.09375, grv 0.21875, jmp 6.5…), drawn at `SCALE` 1.5. Momentum
  rules: holding the way they go, no air drag and no roll friction. Sheets,
  regions and the sheet generator (`maker.rs`) are here too.
- `src/game/` — the game mode (badniks, monitors, spikes, gimmicks in
  `things.rs`, bosses). `src/special/` — special stages.
- `src/library/` — examples, challenges, the built-in acts, and
  `procedural.rs`: the random acts and the 2D endless run's generator
  (every gap crossable, nothing dangerous where the character lands).
- `src/three/` — the 3D mode: `player.rs` (the same guide physics on any
  surface, `K` = 0.04 m per guide px), `phys.rs` (rapier3d), `world.rs`
  (the game: rings, springs, panels, badniks, camera, HUD), `course.rs`
  (the endless road), `zone.rs`, `gfx.rs` (lit shader, meshes cut to fit
  macroquad's 10 000-vertex draw calls), `md3.rs` + `models.rs` (SRB2 model
  packs, coloured through their `_blend` maps), `figure.rs` (the built-in
  shapes).
- `src/physics/`, `src/ui/`, `src/audio/` — the sandbox's physics, the
  interface, sound and music (all synthesised or the player's own files).

Tests that guard gameplay and must stay green: the 2D generator's
(`library::procedural`), the characters' (`sonic`), and the 3D ones,
including the automatic players that run the endless 3D road over many
courses (`three::course`) — a change that makes them fall is a real bug.
