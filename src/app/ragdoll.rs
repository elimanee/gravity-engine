//! Ragdolls: a head, a torso and two-part arms and legs joined by limited
//! hinges ([`LinkKind::Limb`]), so they flop around like a stuntman. The
//! head can be any image; the joints can be cut with the knife.

use super::*;
use crate::physics::fracture;
use crate::physics::links::LinkSpec;

const SKIN: (u8, u8, u8) = (240, 200, 165);
const PANTS: (u8, u8, u8) = (62, 74, 128);
const HEAD: f32 = 32.0;

/// A body part: centre offset from the torso's centre, size (px), colour.
struct Part {
    at: Vec2,
    size: Vec2,
    rgb: (u8, u8, u8),
}

/// A ragdoll joint: offset of the pivot, the two parts (indices in the part
/// list, 0 = torso, 9 = head) and how far it bends (rad, from the pose).
type Joint = ((f32, f32), usize, usize, f32, f32);

const JOINTS: [Joint; 9] = [
    ((-20.0, -20.0), 0, 1, -2.8, 2.8), // left shoulder
    ((20.0, -20.0), 0, 2, -2.8, 2.8),  // right shoulder
    ((-20.0, 6.0), 1, 3, -2.4, 0.1),   // left elbow
    ((20.0, 6.0), 2, 4, -0.1, 2.4),    // right elbow
    ((-8.0, 24.0), 0, 5, -1.6, 1.6),   // left hip
    ((8.0, 24.0), 0, 6, -1.6, 1.6),    // right hip
    ((-8.0, 56.0), 5, 7, -0.1, 2.4),   // left knee
    ((8.0, 56.0), 6, 8, -0.1, 2.4),    // right knee
    ((0.0, -27.0), 0, 9, -0.7, 0.7),   // neck
];

fn parts(shirt: (u8, u8, u8)) -> [Part; 9] {
    let p = |x: f32, y: f32, w: f32, h: f32, rgb| Part { at: vec2(x, y), size: vec2(w, h), rgb };
    [
        p(0.0, 0.0, 28.0, 50.0, shirt),
        p(-20.0, -8.0, 11.0, 28.0, shirt),
        p(20.0, -8.0, 11.0, 28.0, shirt),
        p(-20.0, 19.0, 10.0, 26.0, SKIN),
        p(20.0, 19.0, 10.0, 26.0, SKIN),
        p(-8.0, 40.0, 13.0, 32.0, PANTS),
        p(8.0, 40.0, 13.0, 32.0, PANTS),
        p(-8.0, 70.0, 12.0, 30.0, PANTS),
        p(8.0, 70.0, 12.0, 30.0, PANTS),
    ]
}

/// A round face: skin-coloured disc with eyes and a smile.
fn face(size: u32) -> image::RgbaImage {
    let mut img = shapes::rasterize(Shape::Circle, size, SKIN);
    let s = size as f32;
    let dark = image::Rgba([40, 36, 48, 255]);
    let mut dot = |cx: f32, cy: f32, r: f32| {
        for y in 0..size {
            for x in 0..size {
                if (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy) <= r {
                    img.put_pixel(x, y, dark);
                }
            }
        }
    };
    dot(s * 0.36, s * 0.42, s * 0.07);
    dot(s * 0.64, s * 0.42, s * 0.07);
    for k in 0..24 {
        let a = std::f32::consts::PI * (0.2 + 0.6 * k as f32 / 23.0);
        dot(s * 0.5 + a.cos() * s * 0.2, s * 0.56 + a.sin() * s * 0.12, s * 0.035);
    }
    img
}

impl App {
    /// Drop a ragdoll with its torso at `at` (world px). Pressed over an
    /// image, the image becomes its head.
    pub(super) fn spawn_ragdoll(&mut self, at: Vec2) {
        let head_image = object_at(&self.objects, &self.world, at.x, at.y)
            .filter(|&i| matches!(self.objects[i].source, Source::File(_) | Source::Memory { .. }));
        self.record("Ragdoll");
        let mut head = None;
        if let Some(i) = head_image {
            let o = &self.objects[i];
            let k = (HEAD * 1.4) / o.size.x.max(o.size.y);
            head = Some((o.source.clone(), o.size * k));
            self.remove_object(i);
        }
        let c = spawn_color(shapes::PALETTE.len(), rand::gen_range(0.0, 400.0));
        let shirt = ((c.r * 255.0) as u8, (c.g * 255.0) as u8, (c.b * 255.0) as u8);
        let material = Material { friction: 0.8, bounce: 0.1, ..Material::DEFAULT };

        let mut bodies = vec![];
        for part in parts(shirt) {
            bodies.push(self.spawn_part(part.rgb, part.size, at + part.at, material));
        }
        let head_at = at + vec2(0.0, -27.0 - HEAD / 2.0 + 2.0);
        let (source, size) = match head {
            Some((src, size)) => (src, size),
            None => {
                let png = fracture::to_png(&face(64)).unwrap_or_default();
                (Source::Memory { name: "Ragdoll".into(), data: Arc::new(png) }, vec2(HEAD, HEAD))
            }
        };
        let placement = Placement {
            pos_px: (head_at.x, head_at.y),
            size_px: Some((size.x, size.y)),
            material,
            ..Default::default()
        };
        match Object::load(&mut self.world, source, placement) {
            Some(o) => {
                bodies.push(o.body);
                self.objects.push(o);
            }
            None => return,
        }
        for ((dx, dy), a, b, min, max) in JOINTS {
            self.limb(bodies[a], bodies[b], at + vec2(dx, dy), min, max);
        }
        self.sound(crate::audio::sfx::Sound::Pop, at, 0.4);
    }

    /// A rounded box of `size` px (a torso, arm or leg).
    fn spawn_part(&mut self, rgb: (u8, u8, u8), size: Vec2, at: Vec2, material: Material) -> RigidBodyHandle {
        let source = Source::Shape { shape: Shape::Box, rgb };
        let visual = self
            .spawn_cache
            .entry((Shape::Box, 48, rgb))
            .or_insert_with(|| {
                let img = shapes::rasterize(Shape::Box, 48, rgb);
                let dec = crate::assets::Decoded { frames: vec![img], delays_ms: vec![], hull: None };
                Visual::upload(&source, &dec)
            })
            .clone();
        let placement =
            Placement { pos_px: (at.x, at.y), size_px: Some((size.x, size.y)), material, ..Default::default() };
        let o = Object::spawn(&mut self.world, source, visual, placement);
        let body = o.body;
        self.objects.push(o);
        body
    }

    /// Join `a` and `b` at `at` (world px) with a joint bending from `min`
    /// to `max` radians away from their current pose.
    fn limb(&mut self, a: RigidBodyHandle, b: RigidBodyHandle, at: Vec2, min: f32, max: f32) {
        let (x, y) = to_phys(at.x, at.y);
        let p = Point::new(x, y);
        let (Some(ba), Some(bb)) = (self.world.bodies.get(a), self.world.bodies.get(b)) else { return };
        let (la, lb) = (ba.position().inverse_transform_point(&p), bb.position().inverse_transform_point(&p));
        let rest = bb.rotation().angle() - ba.rotation().angle();
        let spec = LinkSpec { kind: LinkKind::Limb, a, b: Some(b), la, lb, length: rest + min, speed: rest + max };
        let link = Link::restore(&mut self.world, spec);
        self.links.push(link);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joints_connect_neighbouring_parts() {
        let parts = parts((200, 60, 60));
        for ((dx, dy), a, b, min, max) in JOINTS {
            assert!(min < max && max - min < std::f32::consts::TAU);
            // Each pivot lies near both parts it joins (the head is index 9).
            let near = |i: usize| {
                if i == 9 {
                    return true;
                }
                let p = &parts[i];
                (dx - p.at.x).abs() <= p.size.x / 2.0 + 6.0 && (dy - p.at.y).abs() <= p.size.y / 2.0 + 6.0
            };
            assert!(near(a) && near(b), "joint at ({dx}, {dy})");
        }
    }
}
