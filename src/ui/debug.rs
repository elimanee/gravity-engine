//! Debug overlay (D): stats, collider outlines and details of the hovered object.

use super::theme::*;
use crate::physics::object::{object_at, Object};
use crate::physics::{to_screen, PhysWorld};
use macroquad::prelude::*;
use rapier2d::prelude::*;

pub struct DebugStats {
    pub paused: bool,
    pub time_scale: f32,
    pub zoom: f32,
}

fn draw_shape(shape: &dyn Shape, iso: &Isometry<f32>, color: Color) {
    let screen = |p: Point<f32>| {
        let w = iso * p;
        to_screen(w.x, w.y)
    };
    let polyline = |pts: &[Point<f32>]| {
        for i in 0..pts.len() {
            let (a, b) = (screen(pts[i]), screen(pts[(i + 1) % pts.len()]));
            draw_line(a.x, a.y, b.x, b.y, 1.2, color);
        }
    };
    if let Some(ball) = shape.as_ball() {
        let c = screen(point![0.0, 0.0]);
        draw_circle_lines(c.x, c.y, ball.radius * crate::config::PPM, 1.2, color);
    } else if let Some(poly) = shape.as_convex_polygon() {
        polyline(poly.points());
    } else if let Some(c) = shape.as_cuboid() {
        let (x, y) = (c.half_extents.x, c.half_extents.y);
        polyline(&[point![-x, -y], point![x, -y], point![x, y], point![-x, y]]);
    } else if let Some(rc) = shape.as_round_cuboid() {
        let (x, y) =
            (rc.inner_shape.half_extents.x + rc.border_radius, rc.inner_shape.half_extents.y + rc.border_radius);
        polyline(&[point![-x, -y], point![x, -y], point![x, y], point![-x, y]]);
    } else if let Some(cap) = shape.as_capsule() {
        let (a, b) = (screen(cap.segment.a), screen(cap.segment.b));
        let r = cap.radius * crate::config::PPM;
        draw_circle_lines(a.x, a.y, r, 1.2, color);
        draw_circle_lines(b.x, b.y, r, 1.2, color);
        draw_line(a.x, a.y, b.x, b.y, 1.2, color);
    } else if let Some(comp) = shape.as_compound() {
        for (sub_iso, sub) in comp.shapes() {
            draw_shape(sub.as_ref(), &(iso * sub_iso), color);
        }
    }
}

/// Collider outlines, centres of mass and velocities (in world pixels).
pub fn draw_world(world: &PhysWorld, objects: &[Object]) {
    for o in objects {
        let (Some(col), Some(body)) = (world.colliders.get(o.collider), world.bodies.get(o.body)) else { continue };
        let color = if !body.is_dynamic() {
            alpha(WARNING, 0.8)
        } else if body.is_sleeping() {
            alpha(TEXT_MUTED, 0.6)
        } else {
            alpha(SUCCESS, 0.8)
        };
        draw_shape(col.shape(), col.position(), color);
        let com = body.center_of_mass();
        let c = to_screen(com.x, com.y);
        draw_circle(c.x, c.y, 2.5, color);
        let v = body.linvel();
        let tip = to_screen(com.x + v.x * 0.15, com.y + v.y * 0.15);
        draw_line(c.x, c.y, tip.x, tip.y, 1.0, alpha(ACCENT_HI, 0.7));
    }
}

/// Stats panel; `mouse` is in world pixels.
pub fn draw_panel(world: &PhysWorld, objects: &[Object], stats: &DebugStats, mouse: Vec2, top: f32) {
    let sleeping = objects.iter().filter(|o| world.bodies.get(o.body).is_some_and(|b| b.is_sleeping())).count();
    let mut lines = vec![
        format!("FPS {}   frame {:.1} ms", get_fps(), get_frame_time() * 1000.0),
        format!("objects {}   sleeping {}", objects.len(), sleeping),
        format!("bodies {}   colliders {}", world.bodies.len(), world.colliders.len()),
        format!(
            "gravity {:+.2} m/s²   time ×{:.2}   zoom {:.0}%",
            world.gravity.y,
            stats.time_scale,
            stats.zoom * 100.0
        ),
        format!("border {}   {}", world.border.label(), if stats.paused { "PAUSED" } else { "running" }),
    ];
    if let Some(i) = object_at(objects, world, mouse.x, mouse.y) {
        let o = &objects[i];
        let b = &world.bodies[o.body];
        lines.push(String::new());
        lines.push(format!("#{i} {}", crate::util::ellipsize(&o.name(), 26)));
        lines.push(format!(
            "pos ({:.2}, {:.2}) m   rot {:.0}°",
            b.translation().x,
            b.translation().y,
            b.rotation().angle().to_degrees()
        ));
        lines.push(format!("vel ({:.2}, {:.2})   ω {:.2}", b.linvel().x, b.linvel().y, b.angvel()));
        lines.push(format!(
            "mass {:.2} kg   {}×{} px{}",
            o.mass,
            o.size.x as i32,
            o.size.y as i32,
            if o.is_animated() { "   animated" } else { "" }
        ));
    }

    let font = 12.0;
    let line_h = 17.0;
    let w = lines.iter().map(|l| measure(l, font)).fold(200.0f32, f32::max) + 24.0;
    let r = Rect::new(10.0, top + 10.0, w, lines.len() as f32 * line_h + 34.0);
    panel(r, 1.0);
    text_bold("DEBUG", r.x + 12.0, r.y + 20.0, 11.0, WARNING);
    for (i, l) in lines.iter().enumerate() {
        text(l, r.x + 12.0, r.y + 38.0 + i as f32 * line_h, font, if i < 5 { TEXT_DIM } else { TEXT });
    }
}
