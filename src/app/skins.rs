//! The levels' ground and ledges drawn with the zone's tiles, when there is
//! a zone sheet: each piece of ground filled with its "ground" tile (the
//! same all over the world, so the pieces meet), its top with the "grass"
//! along it (blades above it too) and the sheet's scenery here and there
//! on it (trees, flowers…, behind the grass), and ledges in the air with
//! "platform". Made once for each piece, again when the zone's sheet
//! changes.

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

/// A picture laid over pieces with gaps between them (a tileset's 8 × 8
/// pieces drawn a pixel apart): the pieces joined, without the gaps, so the
/// ground has no seams. A gap is a clear line with something either side;
/// none: the picture as it is.
fn joined(img: &RgbaImage, pic: Pic) -> Option<(RgbaImage, Pic)> {
    let (x0, y0) = (pic.src.x.max(0.0) as u32, pic.src.y.max(0.0) as u32);
    let (w, h) = (pic.src.w.max(0.0) as u32, pic.src.h.max(0.0) as u32);
    if w < 3 || h < 3 || x0 + w > img.width() || y0 + h > img.height() {
        return None;
    }
    let clear_col = |x: u32| (y0..y0 + h).all(|y| img.get_pixel(x, y)[3] == 0);
    let clear_row = |y: u32| (x0..x0 + w).all(|x| img.get_pixel(x, y)[3] == 0);
    let gap = |i: u32, lo: u32, n: u32, clear: &dyn Fn(u32) -> bool| {
        i > lo && i + 1 < lo + n && clear(i) && !clear(i - 1) && !clear(i + 1)
    };
    let cols: Vec<u32> = (x0..x0 + w).filter(|&x| !gap(x, x0, w, &clear_col)).collect();
    let rows: Vec<u32> = (y0..y0 + h).filter(|&y| !gap(y, y0, h, &clear_row)).collect();
    if cols.len() == w as usize && rows.len() == h as usize {
        return None;
    }
    let mut out = RgbaImage::new(cols.len() as u32, rows.len() as u32);
    for (j, &y) in rows.iter().enumerate() {
        for (i, &x) in cols.iter().enumerate() {
            out.put_pixel(i as u32, j as u32, *img.get_pixel(x, y));
        }
    }
    // The centre moves by the gaps before it.
    let before =
        |lines: &[u32], from: u32, at: f32| lines.iter().filter(|&&l| (l - from) as f32 + 0.5 < at).count() as f32;
    let centre = vec2(before(&cols, x0, pic.centre.x), before(&rows, y0, pic.centre.y));
    let size = Rect::new(0.0, 0.0, cols.len() as f32, rows.len() as f32);
    Some((out, Pic { src: size, centre, ..pic }))
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

/// Something put on the ground: its pictures drawn together, each by its
/// centre, at one point on the ground; and how often it is chosen.
pub struct Item<'a> {
    pub tiles: Vec<Tile<'a>>,
    pub weight: f32,
}

impl Item<'_> {
    /// How far it reaches left, right and up from where it stands (world px).
    fn reach(&self) -> (f32, f32, f32) {
        self.tiles.iter().fold((0.0f32, 0.0f32, 0.0f32), |(l, r, u), t| {
            (l.max(t.pic.centre.x * t.k), r.max((t.pic.src.w - t.pic.centre.x) * t.k), u.max(t.pic.centre.y * t.k))
        })
    }
}

/// Room along the ground for one thing of scenery (world px), and the
/// chance a room has none.
const SCENERY_ROOM: f32 = 150.0;
const SCENERY_NONE: f32 = 0.35;

/// A number from `n`, the same every time (splitmix64).
fn hash(n: i64) -> u64 {
    let mut z = (n as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Where scenery stands on a piece of ground from `x0` to `x1` (world px):
/// in rooms laid out over the whole world (so it is the same every time),
/// where the ground is level enough and it fits; which one, and the point.
fn scenery_spots(poly: &[Vec2], x0: f32, x1: f32, items: &[Item]) -> Vec<(usize, Vec2)> {
    let total: f32 = items.iter().map(|i| i.weight).sum();
    if total <= 0.0 {
        return vec![];
    }
    let mut out = vec![];
    for room in (x0 / SCENERY_ROOM).floor() as i64..=(x1 / SCENERY_ROOM).floor() as i64 {
        let h = hash(room);
        let pick = (h >> 16) as f32 / (1u64 << 48) as f32;
        if pick < SCENERY_NONE {
            continue;
        }
        let mut w = (pick - SCENERY_NONE) / (1.0 - SCENERY_NONE) * total;
        let Some(k) = items.iter().position(|i| {
            w -= i.weight;
            w < 0.0
        }) else {
            continue;
        };
        let x = (room as f32 + 0.2 + (h & 0xffff) as f32 / 65535.0 * 0.6) * SCENERY_ROOM;
        let (left, right, _) = items[k].reach();
        if x - left < x0 || x + right > x1 {
            continue;
        }
        // On level ground (a little either side of its foot).
        let tops: Vec<Option<f32>> = [-10.0, 0.0, 10.0].iter().map(|d| top_at(poly, x + d)).collect();
        let (Some(a), Some(y), Some(b)) = (tops[0], tops[1], tops[2]) else { continue };
        if (a - y).abs() > 3.0 || (b - y).abs() > 3.0 {
            continue;
        }
        out.push((k, vec2(x, y)));
    }
    out
}

/// A piece of ground (its outline, world px) filled with `ground`, `grass`
/// along its top (the tile's centre on the top line) and `scenery` here
/// and there on it, behind the grass. Returns the picture and where it goes
/// (world px).
pub fn bake_ground(poly: &[Vec2], ground: Tile, grass: Option<Tile>, scenery: &[Item]) -> Option<(RgbaImage, Rect)> {
    if poly.len() < 3 {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in poly {
        (x0, y0, x1, y1) = (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y));
    }
    let spots = scenery_spots(poly, x0, x1, scenery);
    let tall = spots.iter().map(|&(k, _)| scenery[k].reach().2).fold(0.0f32, f32::max);
    let above = grass.map_or(0.0, |g| (g.pic.centre.y * g.k).ceil()).max(tall.ceil());
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
    // The scenery, behind what is there already.
    for (k, at) in spots {
        for t in &scenery[k].tiles {
            let left = at.x - t.pic.centre.x * t.k - x0;
            let top = at.y - t.pic.centre.y * t.k - y0;
            let (pw, ph) = ((t.pic.src.w * t.k).ceil() as i32, (t.pic.src.h * t.k).ceil() as i32);
            for j in 0..ph {
                for i in 0..pw {
                    let (ix, iy) = ((left + i as f32).floor() as i64, (top + j as f32).floor() as i64);
                    if ix < 0 || iy < 0 || ix >= w as i64 || iy >= h as i64 {
                        continue;
                    }
                    let (ix, iy) = (ix as u32, iy as u32);
                    if img.get_pixel(ix, iy)[3] > 0 {
                        continue;
                    }
                    let sx = t.pic.src.x + ((i as f32 + 0.5) / t.k).floor().min(t.pic.src.w - 1.0);
                    let sy = t.pic.src.y + ((j as f32 + 0.5) / t.k).floor().min(t.pic.src.h - 1.0);
                    let (sx, sy) = (sx.max(0.0) as u32, sy.max(0.0) as u32);
                    if sx < t.img.width() && sy < t.img.height() {
                        let p = *t.img.get_pixel(sx, sy);
                        if p[3] > 0 {
                            img.put_pixel(ix, iy, p);
                        }
                    }
                }
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
    let first = |name: &str| zone.pics(name).and_then(|p| p.first().copied());
    // The ground's tiles, their pieces joined if they were drawn apart.
    let parts: Vec<(&str, Option<(RgbaImage, Pic)>)> =
        ["ground", "grass", "platform"].into_iter().map(|n| (n, first(n).and_then(|p| joined(img, p)))).collect();
    let tile = |name: &str| match parts.iter().find(|(n, _)| *n == name) {
        Some((_, Some((part, pic)))) => Some(Tile { img: part, pic: *pic, k: zone.k() }),
        _ => first(name).map(|pic| Tile { img, pic, k: zone.k() }),
    };
    let Source::Drawing(d) = &o.source else { return None };
    let (pos, angle) = o.screen_pos(world);
    if angle.abs() > 0.01 {
        return None;
    }
    let pts: Vec<Vec2> = d.points.iter().map(|&[u, v]| pos + vec2(u * o.size.x, -v * o.size.y)).collect();
    // The sheet's scenery, the bigger the rarer.
    let scenery: Vec<Item> = crate::sonic::zone::SCENERY
        .iter()
        .zip([2.0, 3.0, 1.5, 3.0])
        .filter_map(|(name, weight)| {
            let tiles = zone.pics(name)?.iter().map(|&pic| Tile { img, pic, k: zone.k() }).collect();
            Some(Item { tiles, weight })
        })
        .collect();
    let made = match o.material.skin {
        Skin::Ground if d.closed => bake_ground(&pts, tile("ground")?, tile("grass"), &scenery),
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
        let (pic, r) = bake_ground(&poly, ground, Some(grass_tile), &[]).unwrap();
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

    #[test]
    fn pieces_drawn_apart_are_joined_without_the_gaps() {
        // Four 2 × 2 pieces a pixel apart (a 5 × 5 picture), the gaps clear.
        let mut img = RgbaImage::new(5, 5);
        for (i, (x, y)) in [(0, 0), (3, 0), (0, 3), (3, 3)].into_iter().enumerate() {
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                img.put_pixel(x + dx, y + dy, Rgba([i as u8 * 50, 100, 0, 255]));
            }
        }
        let pic = Pic::new(Rect::new(0.0, 0.0, 5.0, 5.0), vec2(4.0, 1.0));
        let (out, p) = joined(&img, pic).unwrap();
        assert_eq!(out.dimensions(), (4, 4));
        assert_eq!((out.get_pixel(1, 0)[0], out.get_pixel(2, 0)[0], out.get_pixel(2, 3)[0]), (0, 50, 150));
        assert!(out.pixels().all(|p| p[3] == 255), "no holes left");
        assert_eq!((p.src, p.centre), (Rect::new(0.0, 0.0, 4.0, 4.0), vec2(3.0, 1.0)), "the cross moves with them");
        // A picture with no gaps is left as it is (clear edges are not gaps).
        let mut solid = RgbaImage::new(6, 3);
        for x in 1..5 {
            for y in 0..3 {
                solid.put_pixel(x, y, Rgba([1, 2, 3, 255]));
            }
        }
        assert!(joined(&solid, Pic::new(Rect::new(0.0, 0.0, 6.0, 3.0), vec2(3.0, 1.0))).is_none());
    }

    #[test]
    fn scenery_stands_on_level_ground_the_same_every_time() {
        // A thing of scenery: a 2 × 3 post standing on its bottom middle.
        let mut img = RgbaImage::new(4, 4);
        let post = [9, 9, 200, 255];
        for y in 0..3 {
            for x in 0..2 {
                img.put_pixel(x, y, Rgba(post));
            }
        }
        let ground_img = {
            let mut g = RgbaImage::new(2, 2);
            g.pixels_mut().for_each(|p| *p = Rgba([120, 60, 0, 255]));
            g
        };
        let ground = Tile { img: &ground_img, pic: Pic::new(Rect::new(0.0, 0.0, 2.0, 2.0), vec2(1.0, 1.0)), k: 1.0 };
        let item = |weight| Item {
            tiles: vec![Tile { img: &img, pic: Pic::new(Rect::new(0.0, 0.0, 2.0, 3.0), vec2(1.0, 3.0)), k: 4.0 }],
            weight,
        };
        // A long flat block: posts here and there, on its top, behind
        // nothing (there is no grass).
        let long = [vec2(0.0, 500.0), vec2(6000.0, 500.0), vec2(6000.0, 600.0), vec2(0.0, 600.0)];
        let (pic, r) = bake_ground(&long, ground, None, &[item(1.0)]).unwrap();
        assert_eq!(r.y, 500.0 - 12.0, "room above the top for them");
        let spots = scenery_spots(&long, 0.0, 6000.0, &[item(1.0)]);
        assert!(spots.len() > 15 && spots.len() < 40, "{}", spots.len());
        for (_, at) in &spots {
            assert_eq!(at.y, 500.0);
            let (x, y) = ((at.x - r.x) as u32, (at.y - r.y) as u32);
            assert_eq!(pic.get_pixel(x, y - 1).0, post, "a post just above the top");
        }
        // The same every time, and on another piece over the same stretch.
        let half = [vec2(3000.0, 300.0), vec2(6000.0, 300.0), vec2(6000.0, 400.0), vec2(3000.0, 400.0)];
        let other: Vec<f32> = scenery_spots(&half, 3000.0, 6000.0, &[item(1.0)]).iter().map(|s| s.1.x).collect();
        let same: Vec<f32> = spots.iter().map(|s| s.1.x).filter(|x| *x > 3004.0).collect();
        assert_eq!(other, same);
        // None on a steep slope, nor on a piece too narrow for one.
        let slope = [vec2(0.0, 3600.0), vec2(6000.0, 0.0), vec2(6000.0, 3700.0), vec2(0.0, 3700.0)];
        assert!(scenery_spots(&slope, 0.0, 6000.0, &[item(1.0)]).is_empty());
        let narrow = [vec2(0.0, 500.0), vec2(6.0, 500.0), vec2(6.0, 600.0), vec2(0.0, 600.0)];
        assert!(scenery_spots(&narrow, 0.0, 6.0, &[item(1.0)]).is_empty());
    }
}
