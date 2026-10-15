//! The infinite world (World size ∞): a floor that never ends, no side
//! walls or ceiling, and everything moved back towards the middle of the
//! arena whenever the view wanders off, so positions stay precise.

use super::*;

/// Width of the stretch kept around the view (px).
pub(super) const INFINITE_W: f32 = 60_000.0;
/// The world shifts once the view is this far from the middle (px).
const SHIFT_AT: f32 = 12_000.0;

impl App {
    pub(super) fn infinite(&self) -> bool {
        self.s.world_size >= 4
    }

    /// Change the world size, keeping what is in view where it is when
    /// going to or from the infinite world.
    pub(super) fn set_world_size(&mut self, size: u8) {
        let was = self.infinite();
        self.s.world_size = size;
        let now = self.infinite();
        let (aw, ah) = self.arena();
        if was != now {
            let cx = self.camera.center.x;
            self.shift_world((cx - aw / 2.0).round());
            self.world.set_infinite(now);
            self.camera.infinite = now;
            if now && self.s.border != crate::physics::borders::BorderMode::Walls {
                self.s.border = crate::physics::borders::BorderMode::Walls;
                self.world.set_border(self.s.border);
            }
        }
        self.world.resize((aw, ah));
        self.last_size = (aw, ah);
        let screen = vec2(screen_width(), screen_height());
        self.camera.fit(vec2(aw, ah), screen);
        if !now {
            self.endless = None;
        }
        if now {
            if let Some(run) = &self.sonic {
                self.camera.center.x = run.sonic.pos.x;
            }
        }
    }

    /// Once a frame: bring the world back under the view if it wandered off.
    pub(super) fn recentre(&mut self) {
        if !self.infinite() {
            return;
        }
        let dx = self.camera.center.x - INFINITE_W / 2.0;
        if dx.abs() > SHIFT_AT {
            self.shift_world(dx.round());
        }
    }

    /// Move the whole world `dx` px to the left: bodies, joints to the
    /// background, zones, gadgets, Sonic, rings, particles and the view.
    pub(super) fn shift_world(&mut self, dx: f32) {
        if dx == 0.0 {
            return;
        }
        let dm = dx / PPM;
        self.world.shift(dm);
        for l in self.links.iter_mut().filter(|l| l.b.is_none()) {
            l.lb.x -= dm;
        }
        for g in &mut self.gadgets {
            if g.host.is_none() {
                g.spec.at[0] -= dm;
            }
            if let Some(l) = g.hook.as_mut().filter(|l| l.b.is_none()) {
                l.lb.x -= dm;
            }
            if let Some((end, _)) = &mut g.miss {
                end.x -= dx;
            }
        }
        for z in &mut self.zones {
            z.min[0] -= dm;
            z.max[0] -= dm;
        }
        for r in &mut self.rings {
            r.x -= dx;
        }
        if let Some(run) = &mut self.sonic {
            run.sonic.pos.x -= dx;
            run.sonic.spawn.x -= dx;
        }
        if let Some(e) = &mut self.endless {
            e.shift(dx);
        }
        for o in &mut self.objects {
            o.clear_trail();
        }
        self.camera.shift(dx);
        self.effects.shift(dx);
        self.sky.shift(dx);
        self.blasts.clear();
        self.rewind.clear();
        // Undo steps hold the old positions.
        self.history.clear();
        self.stroke = None;
        self.link_drag = None;
        self.zone_drag = None;
        self.select_drag = None;
        self.gadget_drag = None;
    }
}
