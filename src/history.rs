//! Undo / redo. Before each edit (adding, deleting, resizing, linking…) the
//! app records a snapshot of every object and link; undoing swaps the scene
//! back to it. Snapshots share textures with the live objects, so they are
//! cheap to take and restore without decoding anything again.

use crate::physics::links::{Link, LinkSpec};
use crate::physics::object::{Object, Placement, Source, Visual};
use crate::physics::zones::Zone;
use crate::physics::PhysWorld;
use rapier2d::prelude::*;
use std::collections::HashMap;

/// Snapshots kept on each stack.
const LIMIT: usize = 40;

struct ObjectState {
    source: Source,
    visual: Visual,
    placement: Placement,
}

struct LinkState {
    /// Body handles in `spec` are stale; `a` and `b` index the objects.
    spec: LinkSpec,
    a: usize,
    b: Option<usize>,
}

pub struct Snapshot {
    objects: Vec<ObjectState>,
    links: Vec<LinkState>,
    zones: Vec<Zone>,
}

/// What a snapshot brings back.
pub struct Restored {
    pub objects: Vec<Object>,
    pub links: Vec<Link>,
    pub zones: Vec<Zone>,
}

impl Snapshot {
    pub fn capture(world: &PhysWorld, objects: &[Object], links: &[Link], zones: &[Zone]) -> Self {
        let index: HashMap<RigidBodyHandle, usize> = objects.iter().enumerate().map(|(i, o)| (o.body, i)).collect();
        Snapshot {
            objects: objects
                .iter()
                .map(|o| ObjectState { source: o.source.clone(), visual: o.visual(), placement: o.placement(world) })
                .collect(),
            links: links
                .iter()
                .filter_map(|l| {
                    Some(LinkState {
                        spec: l.spec(),
                        a: *index.get(&l.a)?,
                        b: match l.b {
                            Some(b) => Some(*index.get(&b)?),
                            None => None,
                        },
                    })
                })
                .collect(),
            zones: zones.to_vec(),
        }
    }

    /// Spawn the snapshot's objects and links into `world` (which should be empty of objects).
    pub fn restore(&self, world: &mut PhysWorld) -> Restored {
        let objects: Vec<Object> = self
            .objects
            .iter()
            .map(|s| Object::spawn(world, s.source.clone(), s.visual.clone(), s.placement))
            .collect();
        let links = self
            .links
            .iter()
            .map(|l| {
                let spec = LinkSpec { a: objects[l.a].body, b: l.b.map(|i| objects[i].body), ..l.spec };
                Link::restore(world, spec)
            })
            .collect();
        Restored { objects, links, zones: self.zones.clone() }
    }
}

/// Two stacks of labelled states.
pub struct History<T> {
    undo: Vec<(String, T)>,
    redo: Vec<(String, T)>,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        History { undo: vec![], redo: vec![] }
    }
}

impl<T> History<T> {
    /// Remember the state before an edit called `label`.
    pub fn record(&mut self, label: impl Into<String>, before: T) {
        self.undo.push((label.into(), before));
        if self.undo.len() > LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Step back: returns the edit's label and the state to restore. `current`
    /// is kept so the edit can be redone.
    pub fn undo(&mut self, current: T) -> Option<(String, T)> {
        let (label, state) = self.undo.pop()?;
        self.redo.push((label.clone(), current));
        Some((label, state))
    }

    pub fn redo(&mut self, current: T) -> Option<(String, T)> {
        let (label, state) = self.redo.pop()?;
        self.undo.push((label.clone(), current));
        Some((label, state))
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_round_trip() {
        let mut h = History::default();
        let mut state = 0;
        h.record("add", state);
        state = 1;
        h.record("add", state);
        state = 2;

        let (label, s) = h.undo(state).unwrap();
        assert_eq!((label.as_str(), s), ("add", 1));
        state = s;
        let (_, s) = h.undo(state).unwrap();
        state = s;
        assert_eq!(state, 0);
        assert!(h.undo(state).is_none());

        let (_, s) = h.redo(state).unwrap();
        state = s;
        assert_eq!(state, 1);
        // A new edit drops the redo stack.
        h.record("delete", state);
        assert!(h.redo.is_empty());
    }

    #[test]
    fn history_is_bounded() {
        let mut h = History::default();
        for i in 0..100 {
            h.record("x", i);
        }
        assert_eq!(h.undo.len(), LIMIT);
        assert_eq!(h.undo[0].1, 100 - LIMIT);
    }
}
