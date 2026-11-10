//! The levels' ground and ledges drawn with the zone's tiles, when there is
//! a zone sheet: each piece of ground filled with its "ground" tile (the
//! same all over the world, so the pieces meet), its top with the "grass"
//! along it (blades above it too), and ledges in the air with "platform".
//! Made once for each piece, again when the zone's sheet changes.

use super::*;
use crate::physics::object::Skin;
use crate::sonic::sheet::{Pic, Sheet};
use image::{Rgba, RgbaImage};

/// A tile of the zone's picture, `k` world px a pixel of it.
#[derive(Clone, Copy)]
pub struct Tile<'a> {
    pub img: &'a RgbaImage,
    pub pic: Pic,
    pub k: f32,
}

impl Tile<'_> {
    /// Its pixel at (x, y) world px from its top-left, repeated across
    /// (and down too when `down`; else none past its bottom or above it).
    fn at(&self, x: f32, y: f32, down: bool) -> Option<Rgba<u8>> {
        let (w, h) = (self.pic.src.w * self.k, self.pic.src.h * self.k);
        let y = if down { y.rem_euclid(h) } else { y };
        if y < 0.0 || y >= h || w <= 0.0 {
            return None;
        }
        let sx = self.pic.src.x + (x.rem_euclid(w) / self.k).floor().min(self.pic.src.w - 1.0);
        let sy = self.pic.src.y + (y / self.k).floor().min(self.pic.src.h - 1.0);
        let (sx, sy) = (sx.max(0.0) as u32, sy.max(0.0) as u32);
        (sx < self.img.width() && sy < self.img.height()).then(|| *self.img.get_pixel(sx, sy))
    }
}

/// Where a polygon's top is at x (the highest of its edges there), if it
/// is over x at all.
fn top_at(poly: &[Vec2], x: f32) -> Option<f32> {
    let n = poly.len();
    let mut top: Option<f32> = None;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (lo, hi) = (a.x.min(b.x), a.x.max(b.x));
        if (b.x - a.x).abs() < 1e-6 || x < lo || x >= hi {
            continue;
        }
        let y = a.y + (x - a.x) / (b.x - a.x) * (b.y - a.y);
        top = Some(top.map_or(y, |t| t.min(y)));
    }
    top
}

/// Where a row at y is inside a polygon: pairs of x (left, right).
fn spans_at(poly: &[Vec2], y: f32) -> Vec<(f32, f32)> {
    let n = poly.len();
    let mut xs: Vec<f32> = (0..n)
        .filter_map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let crosses = (a.y <= y && y < b.y) || (b.y <= y && y < a.y);
            crosses.then(|| a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x))
        })
        .collect();
    xs.sort_by(f32::total_cmp);
    xs.as_chunks::<2>().0.iter().map(|p| (p[0], p[1])).collect()
}

/// A piece of ground (its outline, world px) filled with `ground`, `grass`
/// along its top (the tile's centre on the top line). Returns the picture
/// and where it goes (world px).
pub fn bake_ground(poly: &[Vec2], ground: Tile, grass: Option<Tile>) -> Option<(RgbaImage, Rect)> {
    if poly.len() < 3 {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in poly {
        (x0, y0, x1, y1) = (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y));
    }
    let above = grass.map_or(0.0, |g| (g.pic.centre.y * g.k).ceil());
    let below = grass.map_or(0.0, |g| (g.pic.src.h - g.pic.centre.y) * g.k);
    let (x0, y0) = (x0.floor(), (y0 - above).floor());
    let (w, h) = ((x1.ceil() - x0) as u32, (y1.ceil() - y0) as u32);
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return None;
    }
    let tops: Vec<Option<f32>> = (0..w).map(|i| top_at(poly, x0 + i as f32 + 0.5)).collect();
    let mut img = RgbaImage::new(w, h);
    for j in 0..h {
        let cy = y0 + j as f32 + 0.5;
        let spans = spans_at(poly, cy);
        for (i, top) in tops.iter().enumerate() {
            let Some(top) = *top else { continue };
            let cx = x0 + i as f32 + 0.5;
            let inside = spans.iter().any(|&(a, b)| cx >= a && cx < b);
            let depth = cy - top;
            let px = match grass {
                // The grass along the top, over the ground (blades above it).
                Some(g) if depth < below && depth >= -above => {
                    let blade = g.at(cx, g.pic.centre.y * g.k + depth, false).filter(|p| p[3] > 0);
                    match (blade, inside) {
                        (Some(p), _) => Some(p),
                        (None, true) => ground.at(cx, cy, true),
                        (None, false) => None,
                    }
                }
                _ if inside => ground.at(cx, cy, true),
                _ => None,
            };
            if let Some(p) = px {
                img.put_pixel(i as u32, j, p);
            }
        }
    }
    Some((img, Rect::new(x0, y0, w as f32, h as f32)))
}

/// A ledge in the air along `line` (its centre line, world px), `thick`
/// px thick: `tile` along it, its centre on the ledge's top.
pub fn bake_platform(line: &[Vec2], thick: f32, tile: Tile) -> Option<(RgbaImage, Rect)> {
    let (lo, hi) = line.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    let top = line.iter().map(|p| p.y).fold(f32::MAX, f32::min) - thick / 2.0;
    let (x0, x1) = ((lo - thick / 2.0).floor(), (hi + thick / 2.0).ceil());
    let y0 = (top - tile.pic.centre.y * tile.k).floor();
    let (w, h) = ((x1 - x0) as u32, (tile.pic.src.h * tile.k).ceil() as u32);
    if w == 0 || h == 0 || w > 8192 {
        return None;
    }
    let mut img = RgbaImage::new(w, h);
    for j in 0..h {
        for i in 0..w {
            if let Some(p) = tile.at(i as f32 + 0.5, j as f32 + 0.5, false) {
                img.put_pixel(i, j, p);
            }
        }
    }
    Some((img, Rect::new(x0, y0, w as f32, h as f32)))
}

/// What `o` looks like with the zone's tiles (none: its own sprite).
fn skin_for(o: &Object, world: &PhysWorld, zone: &Sheet) -> Option<(Texture2D, Rect)> {
    let img = zone.image.as_deref()?;
    let tile = |name: &str| zone.pics(name).and_then(|p| p.first().copied()).map(|pic| Tile { img, pic, k: zone.k() });
    let Source::Drawing(d) = &o.source else { return None };
    let (pos, angle) = o.screen_pos(world);
    if angle.abs() > 0.01 {
        return None;
    }
    let pts: Vec<Vec2> = d.points.iter().map(|&[u, v]| pos + vec2(u * o.size.x, -v * o.size.y)).collect();
    let made = match o.material.skin {
        Skin::Ground if d.closed => bake_ground(&pts, tile("ground")?, tile("grass")),
        // The ground draws the grass along it.
        Skin::Grass => {
            tile("grass")?;
            return Some((Texture2D::empty(), Rect::default()));
        }
        Skin::Platform if !d.closed => bake_platform(&pts, d.thickness * o.size.x.max(o.size.y), tile("platform")?),
        _ => None,
    };
    let (img, r) = made?;
    let texture = crate::sonic::sheet::upload(&img);
    Some((texture, Rect::new(r.x - pos.x, r.y - pos.y, r.w, r.h)))
}

impl App {
    /// The ground and ledges made with the zone's tiles (again, after its
    /// sheet changed).
    pub(super) fn update_skins(&mut self) {
        let v = self.zone_version;
        let (world, zone) = (&self.world, self.sonic_sheet.zone.as_ref());
        for o in self.objects.iter_mut().filter(|o| o.material.skin != Skin::None && o.skinned != v) {
            o.skinned = v;
            o.skin = zone.and_then(|z| skin_for(o, world, z));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_is_tiled_with_grass_on_top_and_ledges_along() {
        // A 4 × 4 picture: rows 0-1 grass (blades in row 0 only at x 0),
        // rows 2-3 a ground of two colours.
        let mut img = RgbaImage::new(4, 4);
        let (grass, blade, a, b) = ([0, 200, 0, 255], [0, 255, 0, 255], [200, 100, 0, 255], [100, 50, 0, 255]);
        img.put_pixel(0, 0, Rgba(blade));
        for x in 0..4 {
            img.put_pixel(x, 1, Rgba(grass));
            img.put_pixel(x, 2, Rgba(if x % 2 == 0 { a } else { b }));
            img.put_pixel(x, 3, Rgba(if x % 2 == 0 { b } else { a }));
        }
        let ground = Tile { img: &img, pic: Pic::new(Rect::new(0.0, 2.0, 2.0, 2.0), vec2(1.0, 1.0)), k: 1.0 };
        let grass_tile = Tile { img: &img, pic: Pic::new(Rect::new(0.0, 0.0, 4.0, 2.0), vec2(2.0, 1.0)), k: 1.0 };
        // A block from (10, 20) to (18, 30).
        let poly = [vec2(10.0, 20.0), vec2(18.0, 20.0), vec2(18.0, 30.0), vec2(10.0, 30.0)];
        let (pic, r) = bake_ground(&poly, ground, Some(grass_tile)).unwrap();
        // One row above the top for the blades.
        assert_eq!(r, Rect::new(10.0, 19.0, 8.0, 11.0));
        let at = |x: u32, y: u32| pic.get_pixel(x, y).0;
        // The grass is aligned on the world: its blade (x 0 of 4) at x 12.
        assert_eq!(at(2, 0), blade, "a blade above the top");
        assert_eq!((at(0, 0)[3], at(1, 0)[3]), (0, 0), "nothing between the blades");
        assert_eq!((at(0, 1), at(5, 1)), (grass, grass), "grass along the top");
        // Below it, the ground as it is all over the world: y 21 is its
        // second row (b, a), y 22 its first (a, b), from x 10 (even).
        assert_eq!((at(0, 2), at(1, 2), at(0, 3)), (b, a, a));
        assert_eq!(top_at(&poly, 12.0), Some(20.0));
        assert_eq!(top_at(&poly, 30.0), None);
        // A ledge: the tile along it, its centre row on the ledge's top.
        let line = [vec2(100.0, 50.0), vec2(108.0, 50.0)];
        let (pic, r) = bake_platform(&line, 4.0, grass_tile).unwrap();
        assert_eq!(r, Rect::new(98.0, 47.0, 12.0, 2.0));
        assert_eq!(pic.get_pixel(1, 1).0, grass);
    }
}
