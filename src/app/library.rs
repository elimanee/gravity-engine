//! Example scenes and challenge mode.

use super::*;
use crate::library::{challenges, examples, BALL_RGB};
use crate::physics::zones::ZoneKind;
use crate::ui::challenge_bar::ChallengeView;

/// The ball must stay in the goal this long (s) to count.
const WIN_HOLD: f32 = 0.6;
/// After Go, the attempt fails when nothing happened for this long (s).
const TIME_LIMIT: f64 = 15.0;

/// A challenge being played.
pub(super) struct ChallengeRun {
    pub index: usize,
    pub ink_left: f32,
    pub started: bool,
    pub go_time: f64,
    pub in_goal: f32,
    pub won: bool,
    pub failed: bool,
}

impl ChallengeRun {
    fn challenge(&self) -> &'static crate::library::Challenge {
        &challenges::ALL[self.index]
    }
}

impl App {
    /// Replace the scene with `sc` (from a file or the library).
    pub(super) fn apply_scene(&mut self, sc: scene::SceneFile) -> (usize, usize) {
        self.clear_objects();
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
        let (objs, links, failed) = scene::instantiate(&sc, &mut self.world);
        let n = objs.len();
        self.objects = objs;
        self.links = links;
        (n, failed)
    }

    pub(super) fn open_example(&mut self, i: usize) {
        let Some(e) = examples::ALL.get(i) else { return };
        self.challenge = None;
        self.record(&format!("Open {}", e.name));
        self.apply_scene(e.scene(screen_width(), screen_height()));
        self.paused = false;
        self.toasts.success(format!("{}  ·  {}", e.name, e.about));
    }

    pub(super) fn start_challenge(&mut self, i: usize) {
        let Some(c) = challenges::ALL.get(i) else { return };
        self.apply_scene(c.scene(screen_width(), screen_height()));
        self.history.clear();
        self.challenge = Some(ChallengeRun {
            index: i,
            ink_left: c.ink,
            started: false,
            go_time: 0.0,
            in_goal: 0.0,
            won: false,
            failed: false,
        });
        self.s.tool = Tool::Draw;
        self.paused = false;
        self.stroke = None;
        self.picker.fader.open = false;
        self.spawner.fader.open = false;
    }

    /// Index of the challenge ball.
    fn ball(&self) -> Option<usize> {
        let gold = (BALL_RGB[0], BALL_RGB[1], BALL_RGB[2]);
        self.objects.iter().position(|o| matches!(o.source, Source::Shape { shape: Shape::Circle, rgb } if rgb == gold))
    }

    pub(super) fn challenge_go(&mut self) {
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
            run.won = true;
            let (id, name) = (run.challenge().id, run.challenge().name);
            let at = crate::physics::to_screen(p.x, p.y);
            self.effects.confetti(at);
            self.effects.confetti(at + vec2(-120.0, 0.0));
            self.effects.confetti(at + vec2(120.0, 0.0));
            if !self.s.challenges_done.iter().any(|d| d == id) {
                self.s.challenges_done.push(id.to_string());
                self.save_settings();
            }
            self.toasts.success(format!("Solved “{name}”!"));
        } else if get_time() - run.go_time > TIME_LIMIT && run.in_goal == 0.0 {
            run.failed = true;
        }
    }

    pub(super) fn challenge_view(&self) -> Option<ChallengeView<'static>> {
        let run = self.challenge.as_ref()?;
        let c = run.challenge();
        Some(ChallengeView {
            number: run.index + 1,
            count: challenges::ALL.len(),
            name: c.name,
            goal: c.goal,
            ink: (run.ink_left / c.ink).clamp(0.0, 1.0),
            started: run.started,
            won: run.won,
            failed: run.failed,
            has_next: run.index + 1 < challenges::ALL.len(),
        })
    }

    pub(super) fn challenge_action(&mut self, action: Action) {
        let Some(index) = self.challenge.as_ref().map(|r| r.index) else { return };
        match action {
            Action::ChallengeGo => self.challenge_go(),
            Action::ChallengeRetry => self.start_challenge(index),
            Action::ChallengeNext => self.start_challenge(index + 1),
            Action::ChallengeExit => {
                self.challenge = None;
                self.toasts.info("Left the challenge  ·  the scene stays as a sandbox");
            }
            _ => {}
        }
    }
}
