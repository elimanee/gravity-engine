//! The Select tool: pick objects (click, Shift+click, or drag a box), move
//! them together, and duplicate, delete, pin, glue, copy or paste them.

use super::*;
use rapier2d::prelude::{nalgebra, vector};

/// What a Select-tool drag is doing.
pub(super) enum SelectDrag {
    /// Rubber-band box from this corner (world px).
    Box(Vec2),
    /// Moving the selection; last pointer position (world px) and whether an
    /// undo step was recorded yet.
    Move { last: Vec2, recorded: bool },
}

impl App {
    pub(super) fn selected_indices(&self) -> Vec<usize> {
        (0..self.objects.len()).filter(|&i| self.selection.contains(&self.objects[i].body)).collect()
    }

    /// Left press with the Select tool.
    pub(super) fn select_press(&mut self, m: Vec2, shift: bool) {
        match object_at(&self.objects, &self.world, m.x, m.y).map(|i| self.objects[i].body) {
            Some(body) if shift && self.selection.contains(&body) => self.selection.retain(|b| *b != body),
            Some(body) => {
                if !self.selection.contains(&body) {
                    if !shift {
                        self.selection.clear();
                    }
                    self.selection.push(body);
                }
                self.select_drag = Some(SelectDrag::Move { last: m, recorded: false });
            }
            None => {
                if !shift {
                    self.selection.clear();
                }
                self.select_drag = Some(SelectDrag::Box(m));
            }
        }
    }

    /// Pointer held with the Select tool.
    pub(super) fn select_drag_to(&mut self, m: Vec2) {
        let Some(SelectDrag::Move { last, recorded }) = self.select_drag else { return };
        let delta = m - last;
        if delta.length_squared() < 0.01 {
            return;
        }
        if !recorded {
            self.record("Move");
        }
        let d = vector![delta.x / PPM, -delta.y / PPM];
        for &h in &self.selection {
            if let Some(b) = self.world.bodies.get_mut(h) {
                let p = b.translation() + d;
                b.set_translation(p, true);
                b.set_linvel(vector![0.0, 0.0], true);
                b.set_angvel(0.0, true);
            }
        }
        self.select_drag = Some(SelectDrag::Move { last: m, recorded: true });
    }

    pub(super) fn select_release(&mut self, m: Vec2) {
        if let Some(SelectDrag::Box(start)) = self.select_drag.take() {
            let r = Rect::new(start.x.min(m.x), start.y.min(m.y), (m.x - start.x).abs(), (m.y - start.y).abs());
            for o in &self.objects {
                let (p, _) = o.screen_pos(&self.world);
                if r.contains(p) && !self.selection.contains(&o.body) {
                    self.selection.push(o.body);
                }
            }
        }
    }

    pub(super) fn selection_cmd(&mut self, cmd: SelectionCmd, mouse: Vec2) {
        let picked = self.selected_indices();
        let count = picked.len();
        let needs_selection = !matches!(cmd, SelectionCmd::Paste | SelectionCmd::All);
        if needs_selection && count == 0 {
            self.toasts.status("select", "Nothing selected  ·  click or drag a box with the Select tool (S)");
            return;
        }
        match cmd {
            SelectionCmd::All => {
                self.selection = self.objects.iter().map(|o| o.body).collect();
                self.toasts.status("select", format!("Selected {} objects", self.selection.len()));
            }
            SelectionCmd::Delete => {
                self.record("Delete selection");
                for i in picked.into_iter().rev() {
                    self.remove_object(i);
                }
                self.selection.clear();
                self.toasts.status("select", format!("Deleted {count} objects"));
            }
            SelectionCmd::Copy => {
                self.clipboard = Some(Snapshot::capture_some(&self.world, &self.objects, &picked, &self.links));
                self.toasts.status("select", format!("Copied {count} objects  ·  Ctrl+V to paste at the pointer"));
            }
            SelectionCmd::Duplicate => {
                let copy = Snapshot::capture_some(&self.world, &self.objects, &picked, &self.links);
                self.paste(&copy, copy.centre() + vec2(40.0, -40.0), "Duplicate");
            }
            SelectionCmd::Paste => match self.clipboard.take() {
                Some(clip) => {
                    self.paste(&clip, mouse, "Paste");
                    self.clipboard = Some(clip);
                }
                None => self.toasts.status("select", "Nothing to paste  ·  copy a selection with Ctrl+C"),
            },
            SelectionCmd::TogglePin => {
                self.record("Pin selection");
                let pin = picked.iter().any(|&i| !self.objects[i].pinned);
                for i in picked {
                    self.objects[i].set_pinned(&mut self.world, pin);
                }
                self.toasts.status("select", if pin { "Selection pinned" } else { "Selection unpinned" });
            }
            SelectionCmd::Glue => {
                if count < 2 {
                    self.toasts.status("select", "Select at least two objects to glue them together");
                    return;
                }
                self.record("Glue selection");
                let first = self.objects[picked[0]].body;
                for &i in &picked[1..] {
                    let o = &self.objects[i];
                    let at = Point::from(*self.world.bodies[o.body].translation());
                    if let Some(l) = Link::new(&mut self.world, LinkKind::Glue, first, Some(o.body), at, at, 0.0) {
                        self.links.push(l);
                    }
                }
                self.toasts.status("select", format!("Glued {count} objects together  ·  they now move as one"));
                self.sound(crate::audio::sfx::Sound::Snap, mouse, 0.6);
            }
        }
    }

    /// Spawn `clip` centred on `at` (world px) and select the new objects.
    fn paste(&mut self, clip: &Snapshot, at: Vec2, label: &str) {
        self.record(label);
        let restored = clip.restore_offset(&mut self.world, at - clip.centre());
        self.selection = restored.objects.iter().map(|o| o.body).collect();
        let n = restored.objects.len();
        self.objects.extend(restored.objects);
        self.links.extend(restored.links);
        self.toasts.status("select", format!("{label}: {n} objects"));
    }

    /// Selection outlines and the rubber band (world pass).
    pub(super) fn draw_selection(&self, m: Vec2) {
        let t = get_time() as f32;
        for o in self.objects.iter().filter(|o| self.selection.contains(&o.body)) {
            let c = o.corners(&self.world);
            for i in 0..4 {
                let (a, b) = (c[i], c[(i + 1) % 4]);
                dashed(a, b, theme::alpha(theme::TEXT, 0.9), t);
            }
        }
        if let Some(SelectDrag::Box(start)) = self.select_drag {
            let r = Rect::new(start.x.min(m.x), start.y.min(m.y), (m.x - start.x).abs(), (m.y - start.y).abs());
            draw_rectangle(r.x, r.y, r.w, r.h, theme::alpha(theme::ACCENT, 0.08));
            let corners = [vec2(r.x, r.y), vec2(r.x + r.w, r.y), vec2(r.x + r.w, r.y + r.h), vec2(r.x, r.y + r.h)];
            for i in 0..4 {
                dashed(corners[i], corners[(i + 1) % 4], theme::alpha(theme::ACCENT_HI, 0.9), t);
            }
        }
    }
}

/// Marching-ants line.
fn dashed(a: Vec2, b: Vec2, c: Color, t: f32) {
    let len = a.distance(b);
    if len < 1.0 {
        return;
    }
    let dir = (b - a) / len;
    let mut d = -((t * 24.0) % 10.0);
    while d < len {
        let (s, e) = (d.max(0.0), (d + 5.0).min(len));
        if e > s {
            let (p, q) = (a + dir * s, a + dir * e);
            draw_line(p.x, p.y, q.x, q.y, 1.5, c);
        }
        d += 10.0;
    }
}
