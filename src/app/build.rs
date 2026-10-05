//! Building tools: Draw, Link and Zone.

use super::*;
use crate::physics::zones::{self, Zone, ZoneKind};

impl App {
    pub(super) fn finish_stroke(&mut self, stroke: Stroke, shift: bool) {
        // In laser levels the lines are mirrors, and look silvery.
        let light = self.light_challenge();
        let rgb = if light { [214, 228, 246] } else { stroke.rgb };
        let Some(placed) = drawing::from_stroke(&stroke.points, self.s.draw_thickness, rgb) else { return };
        // In challenges drawings stay put unless Shift is held.
        let pinned = if self.challenge.is_some() { !shift } else { self.s.draw_pinned != shift };
        let material = if light { Material::MIRROR } else { Material::DEFAULT };
        let placement =
            Placement { pos_px: placed.center, size_px: Some(placed.size), pinned, material, ..Default::default() };
        self.record("Draw");
        match Object::load(&mut self.world, Source::Drawing(Arc::new(placed.drawing)), placement) {
            Some(o) => self.objects.push(o),
            None => self.toasts.error("Could not create the drawing"),
        }
    }

    /// Where a hinge or motor clicked at `at` goes: the centre of a round
    /// object when the click is near it (so wheels turn around their axle).
    fn pivot_point(&self, i: usize, at: Point<f32>) -> Point<f32> {
        let o = &self.objects[i];
        let round = matches!(o.source, Source::Shape { shape: Shape::Circle, .. });
        let centre = *self.world.bodies[o.body].translation();
        let reach = o.size.x.min(o.size.y) / 2.0 / PPM * if round { 0.7 } else { 0.25 };
        if (at.coords - centre).norm() <= reach {
            Point::from(centre)
        } else {
            at
        }
    }

    pub(super) fn start_link(&mut self, m: Vec2) {
        let p = to_phys(m.x, m.y);
        let at = Point::new(p.0, p.1);
        let under = objects_at(&self.objects, &self.world, m.x, m.y);
        let kind = self.s.link_kind;
        if kind.is_pivot() {
            let (a, b, at) = match under.as_slice() {
                [] => {
                    let what = if kind == LinkKind::Motor { "a wheel" } else { "an object" };
                    self.toasts.status("link", format!("Click on {what}, or where two objects overlap"));
                    return;
                }
                [i] => (self.objects[*i].body, None, self.pivot_point(*i, at)),
                [i, j, ..] => (self.objects[*i].body, Some(self.objects[*j].body), self.pivot_point(*i, at)),
            };
            self.add_link(kind, a, b, at, at);
        } else {
            let from = under.first().map(|&i| {
                let body = self.objects[i].body;
                (body, self.world.bodies[body].position().inverse_transform_point(&at))
            });
            self.link_drag = Some(LinkDrag { from, start: at, start_px: m });
        }
    }

    pub(super) fn finish_link(&mut self, m: Vec2) {
        let Some(drag) = self.link_drag.take() else { return };
        if m.distance(drag.start_px) < 10.0 {
            self.toasts.status("link", "Drag from one object to another (or to the background)");
            return;
        }
        let p = to_phys(m.x, m.y);
        let end = Point::new(p.0, p.1);
        let from_body = drag.from.map(|(b, _)| b);
        let target = objects_at(&self.objects, &self.world, m.x, m.y)
            .into_iter()
            .map(|i| self.objects[i].body)
            .find(|&b| Some(b) != from_body);
        let kind = self.s.link_kind;
        match (drag.from, target) {
            (Some((a, local)), b) => {
                let pa = self.world.bodies.get(a).map_or(drag.start, |body| body.position() * local);
                self.add_link(kind, a, b, pa, end)
            }
            (None, Some(b)) => self.add_link(kind, b, None, end, drag.start),
            (None, None) => self.toasts.status("link", "Start or end the link on an object"),
        }
    }

    pub(super) fn add_link(
        &mut self,
        kind: LinkKind,
        a: RigidBodyHandle,
        b: Option<RigidBodyHandle>,
        pa: Point<f32>,
        pb: Point<f32>,
    ) {
        self.record(kind.label());
        let speed = if kind == LinkKind::Motor { self.s.motor_speed } else { 0.0 };
        if let Some(mut l) = Link::new(&mut self.world, kind, a, b, pa, pb, speed) {
            if kind == LinkKind::Motor && self.s.motor_drive {
                // Rebuilt as a driven motor (it coasts until ← or → is held).
                l.remove(&mut self.world);
                l = Link::restore(&mut self.world, links::LinkSpec { drive: true, ..l.spec() });
            }
            self.links.push(l);
            let p = crate::physics::to_screen(pa.x, pa.y);
            self.sound(crate::audio::sfx::Sound::Snap, p, 0.6);
            let hint = if b.is_none() { " to the background" } else { "" };
            self.toasts.status("link", format!("{} added{hint}  ·  right-click it to remove", kind.label()));
        }
    }

    pub(super) fn finish_zone(&mut self, m: Vec2) {
        let Some(start) = self.zone_drag.take() else { return };
        if (m.x - start.x).abs() < 16.0 || (m.y - start.y).abs() < 16.0 {
            self.toasts.status("zone", "Drag a rectangle to make a zone");
            return;
        }
        let kind = self.s.zone_kind;
        self.record(&format!("{} zone", kind.label()));
        let zone = Zone::from_screen(kind, start, m, self.s.zone_angle.to_radians(), self.s.zone_strength);
        let paired = zones::add(&mut self.zones, zone);
        let msg = match kind {
            ZoneKind::Portal if paired => "Portal pair ready: objects entering one come out of the other".to_string(),
            ZoneKind::Portal => "Portal entrance placed  ·  now drag its exit".to_string(),
            k => format!("{} zone added  ·  right-click it to remove", k.label()),
        };
        self.toasts.status("zone", msg);
    }

    /// Remove the zone under `m`, if any.
    pub(super) fn remove_zone_at(&mut self, m: Vec2) -> bool {
        let Some(i) = zones::zone_at(&self.zones, m) else { return false };
        let kind = self.zones[i].kind;
        self.record(&format!("Remove {} zone", kind.label().to_lowercase()));
        zones::remove(&mut self.zones, i);
        self.toasts.status("zone", format!("{} zone removed", kind.label()));
        true
    }
}
