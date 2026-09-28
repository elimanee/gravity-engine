//! Challenge editor: turn the current scene into a challenge. Place the
//! golden ball and the goal, set the ink, solve it once in a test, then
//! save it as a `.gchallenge` file under "My challenges".

use super::library::{ChallengeDef, Origin};
use super::*;
use crate::library::custom::{self, ChallengeFile};
use crate::library::BALL_RGB;
use crate::physics::zones::ZoneKind;
use crate::ui::editor_bar::EditorView;
use crate::ui::EditorCmd;
use std::path::PathBuf;
use std::rc::Rc;

const INK_STEP: f32 = 50.0;
const INK_RANGE: (f32, f32) = (100.0, 4000.0);
const BALL_SIZE: f32 = 36.0;

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Place {
    Ball,
    Goal,
}

pub(super) struct Editor {
    pub name: String,
    pub ink: f32,
    pub place: Option<Place>,
    /// Corner where a goal drag started (world px).
    pub goal_drag: Option<Vec2>,
    /// Solved in a test since the last change.
    pub verified: bool,
    /// The level while it is being tested.
    pub saved: Option<scene::SceneFile>,
    /// File being edited: saving overwrites it.
    pub path: Option<PathBuf>,
}

impl App {
    pub(super) fn open_editor(&mut self, existing: Option<usize>) {
        self.challenge = None;
        let mut ed = Editor {
            name: String::new(),
            ink: 600.0,
            place: None,
            goal_drag: None,
            verified: false,
            saved: None,
            path: None,
        };
        if let Some(i) = existing {
            self.refresh_custom();
            let Some((path, file)) = self.custom.get(i).cloned() else { return };
            self.record("Edit challenge");
            let (aw, ah) = self.arena();
            self.apply_scene(file.scene_for(aw, ah));
            ed.name = file.name.clone();
            ed.ink = file.ink;
            ed.path = Some(path);
        }
        let hint = if self.ball().is_some() { "" } else { "  ·  start by placing the ball" };
        self.toasts.info(format!("Challenge editor: build a level with any tool{hint}"));
        self.editor = Some(ed);
        self.library.fader.open = false;
    }

    pub(super) fn editor_view(&self) -> Option<EditorView> {
        let ed = self.editor.as_ref().filter(|_| self.challenge.is_none())?;
        Some(EditorView {
            ink: ed.ink,
            placing_ball: ed.place == Some(Place::Ball),
            placing_goal: ed.place == Some(Place::Goal),
            has_ball: self.ball().is_some(),
            has_goal: self.zones.iter().any(|z| z.kind == ZoneKind::Goal),
            verified: ed.verified,
        })
    }

    /// An edit happened: the level must be tested again.
    pub(super) fn editor_touched(&mut self) {
        if self.challenge.is_none() {
            if let Some(ed) = self.editor.as_mut() {
                ed.verified = false;
            }
        }
    }

    pub(super) fn editor_cmd(&mut self, cmd: EditorCmd) {
        let Some(ed) = self.editor.as_mut() else { return };
        match cmd {
            EditorCmd::PlaceBall => ed.place = if ed.place == Some(Place::Ball) { None } else { Some(Place::Ball) },
            EditorCmd::PlaceGoal => ed.place = if ed.place == Some(Place::Goal) { None } else { Some(Place::Goal) },
            EditorCmd::Ink(d) => {
                ed.ink = (ed.ink + d as f32 * INK_STEP).clamp(INK_RANGE.0, INK_RANGE.1);
                ed.verified = false;
            }
            EditorCmd::Test => self.editor_test(),
            EditorCmd::Save => self.editor_save(),
            EditorCmd::Exit => {
                self.editor = None;
                self.toasts.info("Left the challenge editor");
            }
        }
    }

    /// Left press in the world while placing the ball or the goal. Returns
    /// whether the press was used.
    pub(super) fn editor_press(&mut self, m: Vec2) -> bool {
        let Some(place) = self.editor.as_ref().and_then(|e| e.place) else { return false };
        match place {
            Place::Ball => {
                self.record("Place ball");
                if let Some(i) = self.ball() {
                    self.remove_object(i);
                }
                let m_ball = Material { bounce: 0.3, friction: 0.6, ..Material::DEFAULT };
                let rgb = (BALL_RGB[0], BALL_RGB[1], BALL_RGB[2]);
                self.spawn_shape(Shape::Circle, rgb, BALL_SIZE, m, true, m_ball);
                if let Some(ed) = self.editor.as_mut() {
                    ed.place = None;
                }
            }
            Place::Goal => {
                if let Some(ed) = self.editor.as_mut() {
                    ed.goal_drag = Some(m);
                }
            }
        }
        true
    }

    /// Pointer released: finish a goal drag.
    pub(super) fn editor_release(&mut self, m: Vec2) {
        let Some(start) = self.editor.as_mut().and_then(|e| e.goal_drag.take()) else { return };
        if (m.x - start.x).abs() < 24.0 || (m.y - start.y).abs() < 24.0 {
            self.toasts.status("editor", "Drag a larger rectangle for the goal");
            return;
        }
        self.record("Place goal");
        while let Some(i) = self.zones.iter().position(|z| z.kind == ZoneKind::Goal) {
            zones::remove(&mut self.zones, i);
        }
        zones::add(&mut self.zones, Zone::from_screen(ZoneKind::Goal, start, m, 0.0, 0.0));
        if let Some(ed) = self.editor.as_mut() {
            ed.place = None;
        }
    }

    /// The goal being dragged, for drawing.
    pub(super) fn editor_goal_drag(&self) -> Option<Vec2> {
        self.editor.as_ref().and_then(|e| e.goal_drag)
    }

    /// The level as it stands, with the ball pinned at its start.
    fn editor_scene(&mut self) -> Option<scene::SceneFile> {
        let Some(i) = self.ball() else {
            self.toasts.status("editor", "Place the ball first");
            return None;
        };
        if !self.zones.iter().any(|z| z.kind == ZoneKind::Goal) {
            self.toasts.status("editor", "Place the goal first");
            return None;
        }
        if !self.objects[i].pinned {
            self.objects[i].set_pinned(&mut self.world, true);
        }
        let water =
            self.s.water.then_some(scene::SceneWater { level: self.s.water_level, density: self.s.water_density });
        Some(scene::capture(&self.world, &self.objects, &self.links, &self.zones, water))
    }

    fn editor_test(&mut self) {
        let Some(sc) = self.editor_scene() else { return };
        let Some(ed) = self.editor.as_mut() else { return };
        ed.place = None;
        ed.saved = Some(sc.clone());
        let def = ChallengeDef {
            origin: Origin::Test,
            id: "test".into(),
            name: if ed.name.trim().is_empty() { "Untitled challenge".into() } else { ed.name.trim().into() },
            goal: "Get the ball into the goal".into(),
            ink: ed.ink,
            scene: sc,
        };
        self.play(Rc::new(def));
        self.toasts.info("Test: draw, then Go  ·  solve it to unlock Save");
    }

    /// Leave a test and bring the level back.
    pub(super) fn editor_back(&mut self) {
        self.challenge = None;
        let Some(sc) = self.editor.as_mut().and_then(|e| e.saved.take()) else { return };
        self.apply_scene(sc);
        self.history.clear();
        let solved = self.editor.as_ref().is_some_and(|e| e.verified);
        self.toasts.info(if solved { "Back in the editor  ·  Save when ready" } else { "Back in the editor" });
    }

    fn editor_save(&mut self) {
        if !self.editor.as_ref().is_some_and(|e| e.verified) {
            self.toasts.status("editor", "Test the level and solve it before saving (Test)");
            return;
        }
        let Some(sc) = self.editor_scene() else { return };
        let Some(dir) = crate::config::challenge_dir() else {
            self.toasts.error("No config folder to save challenges in");
            return;
        };
        let arena = self.arena();
        let Some(ed) = self.editor.as_mut() else { return };
        let file = ChallengeFile::new(&ed.name, ed.ink, arena, sc);
        let path = ed.path.clone().unwrap_or_else(|| custom::free_path(&dir, &file.name));
        match file.write(&path) {
            Ok(()) => {
                ed.path = Some(path.clone());
                self.refresh_custom();
                self.toasts.success(format!(
                    "Saved “{}”  ·  play it from Library → My challenges  ·  {}",
                    file.name,
                    crate::util::ellipsize(&path.to_string_lossy(), 48)
                ));
            }
            Err(e) => self.toasts.error(format!("Could not save the challenge: {e}")),
        }
    }

    /// Open a `.gchallenge` file (dropped, passed on the command line, or
    /// picked in the load dialog) and play it.
    pub(super) fn open_challenge_file(&mut self, path: &std::path::Path) {
        match ChallengeFile::read(path) {
            Ok(file) => {
                self.refresh_custom();
                let known = self.custom.iter().position(|(p, _)| p == path);
                let origin = known.map_or(Origin::File, Origin::Custom);
                self.editor = None;
                self.start_custom(origin, &file, path);
            }
            Err(e) => self.toasts.error(format!("Challenge: {e}")),
        }
    }
}
