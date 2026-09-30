//! The physical player (`Shift+X`): the classic player windows become a
//! rigid body in the world. They fall, tumble, carry whatever lands on
//! them and get blown about by the tools, while every button still works
//! (the pointer is taken into the windows' own, turned frame). Grabbing a
//! title bar throws them. The windows are drawn into a texture that is
//! then drawn turned with the body.

use super::*;
use crate::ui::widgets;
use rapier2d::prelude::{nalgebra, vector, ColliderBuilder, ColliderHandle, Isometry, RigidBodyBuilder, SharedShape};

pub(super) struct PlayerBody {
    pub body: RigidBodyHandle,
    collider: ColliderHandle,
    /// The windows (relative to their top-left corner) the collider was built for.
    rects: Vec<Rect>,
    size: Vec2,
    target: Option<RenderTarget>,
}

/// Smallest rectangle around all of `rects`.
fn bounds(rects: &[Rect]) -> Rect {
    rects.iter().skip(1).fold(rects[0], |b, r| b.combine_with(*r))
}

/// `v` turned by `a` radians in screen space (y down).
fn turn(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

/// One box per window, around the body's centre.
fn collider(rects: &[Rect], size: Vec2) -> ColliderBuilder {
    let parts = rects
        .iter()
        .map(|r| {
            let c = vec2(r.x + r.w / 2.0, r.y + r.h / 2.0) - size / 2.0;
            let at = Isometry::translation(c.x / PPM, -c.y / PPM);
            (at, SharedShape::cuboid(r.w / 2.0 / PPM, r.h / 2.0 / PPM))
        })
        .collect();
    ColliderBuilder::compound(parts).density(0.5).friction(0.7).restitution(0.15)
}

impl App {
    /// The windows as they are laid out now.
    fn player_rects(&self) -> Vec<Rect> {
        let np = self.audio.now_playing();
        let view = player::view(&self.s, self.skin.as_ref(), &self.audio, np.as_ref());
        self.player.rects(&view)
    }

    /// Create, rebuild or remove the player's body to match the settings
    /// and the windows shown. Called every frame.
    pub(super) fn sync_player_body(&mut self) {
        let want = self.s.player && self.s.player_physics;
        if !want {
            if let Some(pb) = self.player_body.take() {
                // Back on screen, about where it was.
                let centre = self.world.bodies.get(pb.body).map(|b| to_screen_px(b.translation()));
                self.world.remove_body(pb.body);
                if self.grab.as_ref().is_some_and(|g| g.body == pb.body) {
                    self.grab = None;
                }
                self.player.free = false;
                if let Some(c) = centre {
                    self.player.place(self.view().to_screen(c) - pb.size / 2.0);
                }
            }
            return;
        }
        let rects = self.player_rects();
        if rects.is_empty() {
            return;
        }
        let b = bounds(&rects);
        let local: Vec<Rect> = rects.iter().map(|r| Rect::new(r.x - b.x, r.y - b.y, r.w, r.h)).collect();
        let size = b.size();
        match &mut self.player_body {
            None => {
                // Where the windows are on screen now.
                let centre = if self.player.free { b.center() } else { self.view().to_world(b.center()) };
                let (x, y) = to_phys(centre.x, centre.y);
                let body = RigidBodyBuilder::dynamic()
                    .translation(vector![x, y])
                    .angular_damping(0.8)
                    .ccd_enabled(true)
                    .build();
                let body = self.world.bodies.insert(body);
                let col = collider(&local, size).build();
                let collider = self.world.colliders.insert_with_parent(col, body, &mut self.world.bodies);
                self.player_body = Some(PlayerBody { body, collider, rects: local, size, target: None });
                self.player.free = true;
                self.player.place(Vec2::ZERO);
            }
            Some(pb) if pb.rects != local => {
                // A window was opened or closed: a new body, with the
                // top-left corner where it was, moving as before.
                let Some(old) = self.world.bodies.get(pb.body) else { return };
                let a = old.rotation().angle();
                let (v, w) = (*old.linvel(), old.angvel());
                let c = to_screen_px(old.translation());
                let top_left = c + turn(-pb.size / 2.0, -a);
                let centre = top_left + turn(size / 2.0, -a);
                let (x, y) = to_phys(centre.x, centre.y);
                if self.grab.as_ref().is_some_and(|g| g.body == pb.body) {
                    self.grab = None;
                }
                self.world.remove_body(pb.body);
                let body = RigidBodyBuilder::dynamic()
                    .position(Isometry::new(vector![x, y], a))
                    .linvel(v)
                    .angvel(w)
                    .angular_damping(0.8)
                    .ccd_enabled(true)
                    .build();
                pb.body = self.world.bodies.insert(body);
                let col = collider(&local, size).build();
                pb.collider = self.world.colliders.insert_with_parent(col, pb.body, &mut self.world.bodies);
                pb.rects = local;
                pb.size = size;
            }
            Some(_) => {}
        }
        // Lost far outside the world: back at the top of the view.
        let (aw, ah) = self.arena();
        let top = self.view().to_world(vec2(screen_width() / 2.0, self.hud.bottom() + size.y / 2.0 + 20.0));
        if let Some(body) = self.player_body.as_ref().and_then(|pb| self.world.bodies.get_mut(pb.body)) {
            let c = to_screen_px(body.translation());
            if c.x < -500.0 || c.x > aw + 500.0 || c.y > ah + 400.0 || c.y < -3000.0 {
                let (x, y) = to_phys(top.x, top.y);
                body.set_position(Isometry::new(vector![x, y], 0.0), true);
                body.set_linvel(vector![0.0, 0.0], true);
                body.set_angvel(0.0, true);
            }
        }
    }

    /// Centre (world px), angle and size of the physical player.
    fn player_frame(&self) -> Option<(Vec2, f32, Vec2)> {
        let pb = self.player_body.as_ref()?;
        let b = self.world.bodies.get(pb.body)?;
        Some((to_screen_px(b.translation()), b.rotation().angle(), pb.size))
    }

    /// World point `p` in the physical player's own frame (px from its
    /// top-left corner), or `None` when the player is not physical.
    pub(super) fn player_local(&self, p: Vec2) -> Option<Vec2> {
        let (c, a, size) = self.player_frame()?;
        Some(turn(p - c, a) + size / 2.0)
    }

    /// The title bar was grabbed: hold the windows by that point.
    pub(super) fn throw_player(&mut self, at: Vec2) {
        if let Some(pb) = &self.player_body {
            self.grab = Some(Grab::new(&self.world.bodies, pb.body, to_phys(at.x, at.y), Tool::Swing));
        }
    }

    /// Draw the windows into their texture (before the frame's other drawing).
    pub(super) fn render_player_body(&mut self, world_mouse: Vec2) {
        let local = self.player_local(world_mouse).unwrap_or(vec2(-1e4, -1e4));
        let Some(pb) = &mut self.player_body else { return };
        // Twice the pixels at normal size keeps the turned pixel art crisp.
        let k = if self.s.player_double { 1.0 } else { 2.0 };
        let (w, h) = ((pb.size.x * k).ceil() as u32, (pb.size.y * k).ceil() as u32);
        let fits = pb.target.as_ref().is_some_and(|t| (t.texture.width() as u32, t.texture.height() as u32) == (w, h));
        if !fits {
            let rt = render_target(w.max(1), h.max(1));
            rt.texture.set_filter(FilterMode::Linear);
            pb.target = Some(rt);
        }
        let Some(rt) = pb.target.clone() else { return };
        let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, pb.size.x, pb.size.y));
        cam.render_target = Some(rt);
        set_camera(&cam);
        clear_background(BLANK);
        widgets::set_clip_scale(Some(k));
        let np = self.audio.now_playing();
        let view = player::view(&self.s, self.skin.as_ref(), &self.audio, np.as_ref());
        self.player.draw(&view, local);
        widgets::set_clip_scale(None);
        set_default_camera();
    }

    /// Draw the physical player turned with its body (world pass).
    pub(super) fn draw_player_body(&self) {
        let (Some((c, a, size)), Some(rt)) =
            (self.player_frame(), self.player_body.as_ref().and_then(|pb| pb.target.as_ref()))
        else {
            return;
        };
        for (off, tint) in [(vec2(5.0, 7.0), Color::new(0.0, 0.0, 0.0, 0.3)), (Vec2::ZERO, WHITE)] {
            draw_texture_ex(
                &rt.texture,
                c.x - size.x / 2.0 + off.x,
                c.y - size.y / 2.0 + off.y,
                tint,
                DrawTextureParams { dest_size: Some(size), rotation: -a, flip_y: true, ..Default::default() },
            );
        }
    }
}

/// A body's translation in world pixels.
fn to_screen_px(t: &rapier2d::prelude::Vector<f32>) -> Vec2 {
    crate::physics::to_screen(t.x, t.y)
}
