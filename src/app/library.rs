//! Example scenes and challenge mode.

use super::*;
use crate::library::custom::{self, ChallengeFile};
use crate::library::{challenges, examples, BALL_RGB};
use crate::physics::zones::ZoneKind;
use crate::ui::challenge_bar::ChallengeView;
use std::rc::Rc;

/// The ball must stay in the goal this long (s) to count.
const WIN_HOLD: f32 = 0.6;
/// After Go, the attempt fails when nothing happened for this long (s).
const TIME_LIMIT: f64 = 15.0;

/// Where a challenge comes from.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Origin {
    BuiltIn(usize),
    /// Index into the "My challenges" list.
    Custom(usize),
    /// A `.gchallenge` opened from elsewhere.
    File,
    /// Testing the level in the editor.
    Test,
}

/// Everything needed to (re)start a challenge.
pub(super) struct ChallengeDef {
    pub origin: Origin,
    pub id: String,
    pub name: String,
    pub goal: String,
    pub ink: f32,
    pub scene: scene::SceneFile,
}

/// A challenge being played.
pub(super) struct ChallengeRun {
    pub def: Rc<ChallengeDef>,
    pub ink_left: f32,
    pub started: bool,
    pub go_time: f64,
    pub in_goal: f32,
    pub won: bool,
    pub failed: bool,
    /// Stars earned (set on a win).
    pub stars: u8,
}

impl App {
    /// Replace the scene with `sc` (from a file or the library).
    pub(super) fn apply_scene(&mut self, sc: scene::SceneFile) -> (usize, usize) {
        self.clear_objects();
        self.grains.clear(&mut self.world);
        self.s.gravity = sc.gravity.clamp(crate::settings::GRAVITY_RANGE.0, crate::settings::GRAVITY_RANGE.1);
        self.s.border = sc.border;
        self.world.set_border(sc.border);
        self.world.set_gravity(self.s.gravity);
        if sc.version >= 2 {
            self.s.water = sc.water.is_some();
        }
        if let Some(w) = sc.water {
            self.s.water_level = w.level;
            self.s.water_density = w.density;
            self.s = self.s.clone().sanitized();
        }
        self.zones = sanitize_zones(sc.zones.clone());
        let made = scene::instantiate(&sc, &mut self.world);
        let n = made.objects.len();
        self.objects = made.objects;
        self.links = made.links;
        self.gadgets = made.gadgets;
        (n, made.failed)
    }

    pub(super) fn open_example(&mut self, i: usize) {
        let Some(e) = examples::ALL.get(i) else { return };
        self.challenge = None;
        self.record(&format!("Open {}", e.name));
        let (aw, ah) = self.arena();
        self.apply_scene(e.scene(aw, ah));
        self.frame_design_area();
        self.paused = false;
        self.toasts.success(format!("{}  ·  {}", e.name, e.about));
    }

    pub(super) fn start_challenge(&mut self, i: usize) {
        let Some(c) = challenges::ALL.get(i) else { return };
        let (aw, ah) = self.arena();
        self.play(Rc::new(ChallengeDef {
            origin: Origin::BuiltIn(i),
            id: c.id.to_string(),
            name: c.name.to_string(),
            goal: c.goal.to_string(),
            ink: c.ink,
            scene: c.scene(aw, ah),
        }));
    }

    /// Play a challenge from "My challenges" (or a file opened directly).
    pub(super) fn start_custom(&mut self, origin: Origin, file: &ChallengeFile, path: &std::path::Path) {
        let (aw, ah) = self.arena();
        let stem = path.file_stem().map_or_else(|| file.name.clone(), |s| s.to_string_lossy().into_owned());
        self.play(Rc::new(ChallengeDef {
            origin,
            id: format!("custom:{stem}"),
            name: file.name.clone(),
            goal: file.goal.clone(),
            ink: file.ink,
            scene: file.scene_for(aw, ah),
        }));
    }

    pub(super) fn start_custom_index(&mut self, i: usize) {
        self.refresh_custom();
        let Some((path, file)) = self.custom.get(i).cloned() else { return };
        self.start_custom(Origin::Custom(i), &file, &path);
    }

    /// Re-read the "My challenges" folder.
    pub(super) fn refresh_custom(&mut self) {
        self.custom = crate::config::challenge_dir().map(|d| custom::list(&d)).unwrap_or_default();
    }

    /// (Re)start the challenge `def`.
    pub(super) fn play(&mut self, def: Rc<ChallengeDef>) {
        self.apply_scene(def.scene.clone());
        if def.origin != Origin::Test {
            self.frame_design_area();
        }
        self.history.clear();
        self.challenge = Some(ChallengeRun {
            ink_left: def.ink,
            def,
            started: false,
            go_time: 0.0,
            in_goal: 0.0,
            won: false,
            failed: false,
            stars: 0,
        });
        self.s.tool = Tool::Draw;
        self.paused = false;
        self.stroke = None;
        self.selection.clear();
        self.picker.fader.open = false;
        self.spawner.fader.open = false;
    }

    /// Point the camera at the area built-in scenes are laid out in.
    fn frame_design_area(&mut self) {
        let (aw, ah) = self.arena();
        let r = crate::library::design_rect(aw, ah);
        self.camera.frame(r, vec2(screen_width(), screen_height()));
    }

    /// Index of the challenge ball.
    pub(super) fn ball(&self) -> Option<usize> {
        let gold = (BALL_RGB[0], BALL_RGB[1], BALL_RGB[2]);
        self.objects.iter().position(|o| matches!(o.source, Source::Shape { shape: Shape::Circle, rgb } if rgb == gold))
    }

    /// The challenge being played is a laser level: drawings are mirrors,
    /// and the beam has to reach the goal.
    pub(super) fn light_challenge(&self) -> bool {
        self.challenge.as_ref().is_some_and(|r| {
            r.def.scene.gadgets.iter().any(|g| g.spec.kind == crate::physics::gadgets::GadgetKind::Laser)
        })
    }

    pub(super) fn challenge_go(&mut self) {
        if self.light_challenge() {
            self.toasts.status("challenge", "The laser is always on  ·  draw mirrors to bend it into the target");
            return;
        }
        let Some(i) = self.ball() else { return };
        let Some(run) = self.challenge.as_mut().filter(|r| !r.started) else { return };
        run.started = true;
        run.go_time = get_time();
        self.paused = false;
        self.stroke = None;
        self.objects[i].set_pinned(&mut self.world, false);
    }

    /// Spend ink on a stroke segment; false when there is not enough left.
    pub(super) fn spend_ink(&mut self, length: f32) -> bool {
        match self.challenge.as_mut() {
            Some(run) if run.ink_left >= length => {
                run.ink_left -= length;
                true
            }
            Some(_) => false,
            None => true,
        }
    }

    /// Win / fail detection, once per frame.
    pub(super) fn update_challenge(&mut self, dt: f32) {
        let ball = self.ball();
        let goal = self.zones.iter().position(|z| z.kind == ZoneKind::Goal);
        if self.light_challenge() {
            // Laser level: the beam must stay on the target for a moment.
            let lit = goal.is_some_and(|g| {
                let z = &self.zones[g];
                self.beams.iter().any(|b| b.crosses(z.min, z.max))
            });
            let Some(run) = self.challenge.as_mut().filter(|r| !r.won) else { return };
            run.in_goal = if lit { run.in_goal + dt } else { 0.0 };
            if run.in_goal >= WIN_HOLD {
                let at = goal.map_or(Vec2::ZERO, |g| {
                    let r = self.zones[g].rect();
                    vec2(r.x + r.w / 2.0, r.y + r.h / 2.0)
                });
                self.challenge_won(at);
            }
            return;
        }
        let Some(run) = self.challenge.as_mut() else { return };
        if !run.started || run.won || run.failed {
            return;
        }
        let Some(i) = ball else {
            run.failed = true;
            return;
        };
        let p = *self.world.bodies[self.objects[i].body].translation();
        let inside = goal.is_some_and(|g| self.zones[g].contains(p));
        run.in_goal = if inside { run.in_goal + dt } else { 0.0 };
        if run.in_goal >= WIN_HOLD {
            self.challenge_won(crate::physics::to_screen(p.x, p.y));
        } else if get_time() - run.go_time > TIME_LIMIT && run.in_goal == 0.0 {
            run.failed = true;
        }
    }

    /// The challenge is solved (`at`: where to celebrate).
    fn challenge_won(&mut self, at: Vec2) {
        let Some(run) = self.challenge.as_mut() else { return };
        {
            run.won = true;
            run.stars = custom::stars(1.0 - run.ink_left / run.def.ink.max(1.0));
            let (stars, def) = (run.stars, run.def.clone());
            self.effects.confetti(at);
            self.effects.confetti(at + vec2(-120.0, 0.0));
            self.effects.confetti(at + vec2(120.0, 0.0));
            let shown = format!("{stars}/3 stars");
            if def.origin == Origin::Test {
                if let Some(ed) = self.editor.as_mut() {
                    ed.verified = true;
                }
                self.toasts.success(format!("Solved in test ({shown})  ·  × goes back to the editor, then Save"));
            } else {
                let best = self.s.challenge_stars.entry(def.id.clone()).or_insert(0);
                let improved = stars > *best;
                *best = (*best).max(stars);
                if !self.s.challenges_done.iter().any(|d| *d == def.id) {
                    self.s.challenges_done.push(def.id.clone());
                }
                self.save_settings();
                let more = if stars < 3 { "  ·  use less ink for more stars" } else { "" };
                let new = if improved { "  ·  new best" } else { "" };
                self.toasts.success(format!("Solved “{}”  ·  {shown}{new}{more}", def.name));
            }
            self.sound(crate::audio::sfx::Sound::Win, at, 0.8);
        }
    }

    pub(super) fn challenge_view(&self) -> Option<ChallengeView<'_>> {
        let run = self.challenge.as_ref()?;
        let def = &run.def;
        let (head, has_next) = match def.origin {
            Origin::BuiltIn(i) => {
                (format!("CHALLENGE {} / {}", i + 1, challenges::ALL.len()), i + 1 < challenges::ALL.len())
            }
            Origin::Custom(i) => ("MY CHALLENGE".to_string(), i + 1 < self.custom.len()),
            Origin::File => ("CHALLENGE".to_string(), false),
            Origin::Test => ("TESTING".to_string(), false),
        };
        Some(ChallengeView {
            head,
            name: &def.name,
            goal: &def.goal,
            ink: (run.ink_left / def.ink).clamp(0.0, 1.0),
            started: run.started,
            won: run.won,
            failed: run.failed,
            has_next,
            stars: run.stars,
            testing: def.origin == Origin::Test,
            light: self.light_challenge(),
        })
    }

    pub(super) fn challenge_action(&mut self, action: Action) {
        let Some(def) = self.challenge.as_ref().map(|r| r.def.clone()) else { return };
        match action {
            Action::ChallengeGo => self.challenge_go(),
            Action::ChallengeRetry => self.play(def),
            Action::ChallengeNext => match def.origin {
                Origin::BuiltIn(i) => self.start_challenge(i + 1),
                Origin::Custom(i) => self.start_custom_index(i + 1),
                _ => {}
            },
            Action::ChallengeExit if def.origin == Origin::Test => self.editor_back(),
            Action::ChallengeExit => {
                self.challenge = None;
                self.toasts.info("Left the challenge  ·  the scene stays as a sandbox");
            }
            _ => {}
        }
    }
}
