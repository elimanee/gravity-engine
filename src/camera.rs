//! The view onto the world: zoom and pan.
//!
//! The scene is drawn in *world pixels* (y down, one physics metre is `PPM`
//! pixels). At zoom 1 with the default centre, world pixels are exactly the
//! window's pixels, so a 1× world looks as it always did. The UI is drawn
//! on top in screen pixels.

use macroquad::prelude::*;
use std::cell::Cell;

pub const MAX_ZOOM: f32 = 4.0;

/// A snapshot of the camera for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub zoom: f32,
    /// World point shown at the centre of the window.
    pub center: Vec2,
    /// Window size (logical pixels).
    pub screen: Vec2,
}

impl View {
    pub fn to_screen(self, p: Vec2) -> Vec2 {
        (p - self.center) * self.zoom + self.screen / 2.0
    }

    pub fn to_world(self, p: Vec2) -> Vec2 {
        (p - self.screen / 2.0) / self.zoom + self.center
    }

    pub fn rect_to_screen(&self, r: Rect) -> Rect {
        let a = self.to_screen(vec2(r.x, r.y));
        Rect::new(a.x, a.y, r.w * self.zoom, r.h * self.zoom)
    }

    /// The part of the world that is visible.
    pub fn visible(&self) -> Rect {
        let size = self.screen / self.zoom;
        Rect::new(self.center.x - size.x / 2.0, self.center.y - size.y / 2.0, size.x, size.y)
    }
}

thread_local! {
    static CURRENT: Cell<View> = const {
        Cell::new(View { zoom: 1.0, center: Vec2::ZERO, screen: Vec2::ZERO })
    };
}

/// The view used for the world pass being drawn (for code that needs screen
/// coordinates, like scissor clipping).
pub fn current() -> View {
    CURRENT.with(|c| c.get())
}

pub struct Camera {
    pub zoom: f32,
    pub center: Vec2,
    /// World size in pixels.
    arena: Vec2,
}

impl Camera {
    pub fn new(arena: Vec2, screen: Vec2) -> Self {
        let mut c = Camera { zoom: 1.0, center: arena / 2.0, arena };
        c.fit(arena, screen);
        c
    }

    /// Smallest zoom: the whole world fits with a margin.
    fn min_zoom(&self, screen: Vec2) -> f32 {
        (screen.x / self.arena.x).min(screen.y / self.arena.y).min(1.0) * 0.9
    }

    /// Show the whole world (at most 100 %).
    pub fn fit(&mut self, arena: Vec2, screen: Vec2) {
        self.arena = arena;
        self.zoom = (screen.x / arena.x).min(screen.y / arena.y).min(1.0);
        self.center = arena / 2.0;
    }

    /// Frame a world rectangle (at most 100 %).
    pub fn frame(&mut self, r: Rect, screen: Vec2) {
        self.zoom = (screen.x / r.w).min(screen.y / r.h).min(1.0);
        self.center = vec2(r.x + r.w / 2.0, r.y + r.h / 2.0);
        self.clamp(screen);
    }

    pub fn set_arena(&mut self, arena: Vec2, screen: Vec2) {
        self.arena = arena;
        self.clamp(screen);
    }

    pub fn view(&self, screen: Vec2) -> View {
        View { zoom: self.zoom, center: self.center, screen }
    }

    /// Zoom by `factor` keeping the world point under `anchor` (screen px) still.
    pub fn zoom_at(&mut self, factor: f32, anchor: Vec2, screen: Vec2) {
        let before = self.view(screen).to_world(anchor);
        self.zoom = (self.zoom * factor).clamp(self.min_zoom(screen), MAX_ZOOM);
        let after = self.view(screen).to_world(anchor);
        self.center += before - after;
        self.clamp(screen);
    }

    /// Move the view by a screen-space drag.
    pub fn pan(&mut self, delta: Vec2, screen: Vec2) {
        self.center -= delta / self.zoom;
        self.clamp(screen);
    }

    /// Keep the view over the world: centred when the world is smaller than
    /// the window, otherwise no further out than its edges.
    fn clamp(&mut self, screen: Vec2) {
        self.zoom = self.zoom.clamp(self.min_zoom(screen), MAX_ZOOM);
        let half = screen / self.zoom / 2.0;
        for (c, (h, a)) in [(&mut self.center.x, (half.x, self.arena.x)), (&mut self.center.y, (half.y, self.arena.y))]
        {
            *c = if h * 2.0 >= a { a / 2.0 } else { c.clamp(h, a - h) };
        }
    }

    /// Draw in world pixels until [`end_world`].
    pub fn begin_world(&self, screen: Vec2) {
        let v = self.view(screen);
        CURRENT.with(|c| c.set(v));
        // `Camera2D::from_display_rect` flips y (it is meant for render
        // targets); drawing to the window needs y down as it is.
        let r = v.visible();
        set_camera(&Camera2D {
            target: vec2(r.x + r.w / 2.0, r.y + r.h / 2.0),
            zoom: vec2(2.0 / r.w, 2.0 / r.h),
            ..Default::default()
        });
    }
}

/// Back to screen pixels (for the UI).
pub fn end_world() {
    set_default_camera();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_1x_world_is_the_identity() {
        let screen = vec2(1100.0, 720.0);
        let c = Camera::new(screen, screen);
        let v = c.view(screen);
        assert_eq!(v.to_world(vec2(10.0, 20.0)), vec2(10.0, 20.0));
        assert_eq!(v.to_screen(vec2(500.0, 300.0)), vec2(500.0, 300.0));
    }

    #[test]
    fn zooming_keeps_the_anchor_and_stays_inside() {
        let screen = vec2(1000.0, 600.0);
        let mut c = Camera::new(vec2(2000.0, 1200.0), screen);
        assert!((c.zoom - 0.5).abs() < 1e-4, "a 2× world starts zoomed out to fit");
        let anchor = vec2(700.0, 200.0);
        let before = c.view(screen).to_world(anchor);
        c.zoom_at(2.0, anchor, screen);
        let after = c.view(screen).to_world(anchor);
        assert!((before - after).length() < 1e-3);
        c.pan(vec2(-100_000.0, 0.0), screen);
        let visible = c.view(screen).visible();
        assert!(visible.x + visible.w <= 2000.0 + 1e-2, "cannot pan past the world's edge");
    }
}
