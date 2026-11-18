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
use crate::sonic::zone::Terrain;
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

/// A pixel of a tileset's terrain, `d` sheet px below the ground's top in
/// sheet column `u` (counted over the whole world, so pieces line up): the
/// columns side by side, each its pieces from the top down; tall grass in
/// some stretches only. None: nothing there.
pub fn terrain_px(t: &Terrain, img: &RgbaImage, u: i64, d: i64) -> Option<Rgba<u8>> {
    let piece = t.grid.piece.max(1) as i64;
    let n = t.columns.len() as i64;
    if n == 0 {
        return None;
    }
    let c = &t.columns[u.div_euclid(piece).rem_euclid(n) as usize];
    // Tall grass in one stretch of the columns in so many, the same every
    // time.
    let tall = t.tall_every > 0 && hash(u.div_euclid(piece * n)).is_multiple_of(t.tall_every as u64);
    let (x, y) = (u.rem_euclid(piece) as u32, d.rem_euclid(piece) as u32);
    let at = |p: (u32, u32)| {
        let (px, py) = (p.0 + x, p.1 + y);
        if px >= img.width() || py >= img.height() {
            return None;
        }
        let px = *img.get_pixel(px, py);
        (px[3] > 0).then_some(px)
    };
    match d.div_euclid(piece) {
        -3 if tall => c.tall[0].and_then(at),
        -2 if tall => c.tall[1].and_then(at),
        -1 => match (c.tuft, tall && c.tall[1].is_some()) {
            (Some(p), _) => at(p),
            // Under tall grass, grass all the same.
            (None, true) => t.grass.and_then(at),
            (None, false) => None,
        },
        0 => c.top.and_then(at),
        r if r > 0 && r % 2 == 1 => c.earth.map_or((c.plain[3] > 0).then_some(Rgba(c.plain)), at),
        r if r > 0 => c.under.and_then(at),
        _ => None,
    }
}

/// Sheet px above the ground's top a tileset's terrain reaches (tall grass).
fn terrain_above(t: &Terrain) -> i64 {
    3 * t.grid.piece.max(1) as i64
}

/// The scenery, at `spots` (world px), put into a picture of sheet pixels
/// whose top-left is sheet pixel (`u0`, `v0`), behind what is there.
fn put_scenery(out: &mut RgbaImage, u0: i64, v0: i64, k: f32, scenery: &[Item], spots: &[(usize, Vec2)]) {
    for &(i, at) in spots {
        let (au, av) = ((at.x / k).round() as i64, (at.y / k).round() as i64);
        for t in &scenery[i].tiles {
            let (sx, sy) = (t.pic.src.x as i64, t.pic.src.y as i64);
            let (left, top) = (au - t.pic.centre.x.round() as i64, av - t.pic.centre.y.round() as i64);
            for j in 0..t.pic.src.h as i64 {
                for i in 0..t.pic.src.w as i64 {
                    let (x, y) = (left + i - u0, top + j - v0);
                    if x < 0 || y < 0 || x >= out.width() as i64 || y >= out.height() as i64 {
                        continue;
                    }
                    let (px, py) = ((sx + i) as u32, (sy + j) as u32);
                    if px >= t.img.width() || py >= t.img.height() || out.get_pixel(x as u32, y as u32)[3] > 0 {
                        continue;
                    }
                    let p = *t.img.get_pixel(px, py);
                    if p[3] > 0 {
                        out.put_pixel(x as u32, y as u32, p);
                    }
                }
            }
        }
    }
}

/// A piece of ground (its outline, world px) built from a tileset's
/// terrain, piece by piece, `k` world px a sheet pixel: in sheet pixels
/// (drawn `k` times bigger), each column of them down from the ground's top
/// there, its scenery on it. Returns the picture and where it goes.
pub fn bake_terrain(
    poly: &[Vec2],
    k: f32,
    t: &Terrain,
    img: &RgbaImage,
    scenery: &[Item],
) -> Option<(RgbaImage, Rect)> {
    if poly.len() < 3 || k <= 0.0 {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in poly {
        (x0, y0, x1, y1) = (x0.min(p.x), y0.min(p.y), x1.max(p.x), y1.max(p.y));
    }
    let spots = scenery_spots(poly, x0, x1, scenery);
    let tall = spots.iter().map(|&(i, _)| scenery[i].reach().2 / k).fold(0.0f32, f32::max).ceil() as i64;
    let (u0, u1) = ((x0 / k).floor() as i64, (x1 / k).ceil() as i64);
    let above = terrain_above(t);
    let (v0, v1) = ((y0 / k).floor() as i64 - above.max(tall), (y1 / k).ceil() as i64);
    let (w, h) = ((u1 - u0) as u32, (v1 - v0) as u32);
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return None;
    }
    // The ground's top in each sheet column (whole sheet px).
    let tops: Vec<Option<i64>> =
        (u0..u1).map(|u| top_at(poly, (u as f32 + 0.5) * k).map(|y| (y / k).round() as i64)).collect();
    let mut out = RgbaImage::new(w, h);
    for v in v0..v1 {
        let spans = spans_at(poly, (v as f32 + 0.5) * k);
        for (i, top) in tops.iter().enumerate() {
            let Some(top) = *top else { continue };
            let u = u0 + i as i64;
            let d = v - top;
            let cx = (u as f32 + 0.5) * k;
            // Below the top, only inside the ground; above, its grass.
            let inside = spans.iter().any(|&(a, b)| cx >= a && cx < b);
            if (d >= 0 && !inside) || d < -above {
                continue;
            }
            if let Some(p) = terrain_px(t, img, u, d) {
                out.put_pixel(i as u32, (v - v0) as u32, p);
            }
        }
    }
    put_scenery(&mut out, u0, v0, k, scenery, &spots);
    Some((out, Rect::new(u0 as f32 * k, v0 as f32 * k, w as f32 * k, h as f32 * k)))
}

/// A ledge in the air along `line` (its centre line, world px), `thick`
/// px thick, built from a tileset's terrain: its tufts, grass into earth
/// and the piece under that, its top on the ledge's.
pub fn bake_terrain_ledge(
    line: &[Vec2],
    thick: f32,
    k: f32,
    t: &Terrain,
    img: &RgbaImage,
) -> Option<(RgbaImage, Rect)> {
    let (lo, hi) = line.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
    let top = line.iter().map(|p| p.y).fold(f32::MAX, f32::min) - thick / 2.0;
    let (u0, u1) = (((lo - thick / 2.0) / k).floor() as i64, ((hi + thick / 2.0) / k).ceil() as i64);
    let piece = t.grid.piece.max(1) as i64;
    let vt = (top / k).round() as i64;
    let (v0, v1) = (vt - piece, vt + 2 * piece);
    let (w, h) = ((u1 - u0) as u32, (v1 - v0) as u32);
    if w == 0 || w > 8192 || k <= 0.0 {
        return None;
    }
    let mut out = RgbaImage::new(w, h);
    for v in v0..v1 {
        for u in u0..u1 {
            // Its own tufts only (no tall grass on a ledge).
            let d = v - vt;
            let p = if d < 0 {
                t.columns
                    .get(u.div_euclid(piece).rem_euclid(t.columns.len().max(1) as i64) as usize)
                    .and_then(|c| c.tuft)
                    .and_then(|p| {
                        let px = *img.get_pixel(p.0 + u.rem_euclid(piece) as u32, p.1 + d.rem_euclid(piece) as u32);
                        (px[3] > 0).then_some(px)
                    })
            } else {
                terrain_px(t, img, u, if d < piece { d } else { d + piece })
            };
            if let Some(p) = p {
                out.put_pixel((u - u0) as u32, (v - v0) as u32, p);
            }
        }
    }
    Some((out, Rect::new(u0 as f32 * k, v0 as f32 * k, w as f32 * k, h as f32 * k)))
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
    // A tileset's terrain, piece by piece, when the game knows it.
    if let Some(t) = &zone.terrain {
        let made = match o.material.skin {
            Skin::Ground if d.closed => bake_terrain(&pts, zone.k(), t, img, &scenery),
            Skin::Grass => return Some((Texture2D::empty(), Rect::default())),
            Skin::Platform if !d.closed => {
                bake_terrain_ledge(&pts, d.thickness * o.size.x.max(o.size.y), zone.k(), t, img)
            }
            _ => None,
        };
        let (img, r) = made?;
        let texture = crate::sonic::sheet::upload(&img);
        return Some((texture, Rect::new(r.x - pos.x, r.y - pos.y, r.w, r.h)));
    }
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

    /// A tileset of 8 × 8 pieces, each one colour: two columns.
    fn little_terrain() -> (RgbaImage, Terrain) {
        use crate::sonic::zone::Column;
        let mut img = RgbaImage::new(48, 8);
        let colours = [[0, 200, 0, 255], [90, 40, 0, 255], [240, 120, 0, 255], [0, 150, 0, 255]];
        for (k, c) in colours.iter().enumerate() {
            for y in 0..8 {
                for x in 0..8 {
                    img.put_pixel(k as u32 * 8 + x, y, Rgba(*c));
                }
            }
        }
        let (top, earth, under, tuft) = (Some((0, 0)), Some((8, 0)), Some((16, 0)), Some((24, 0)));
        let columns = vec![
            Column { tall: [None, None], tuft, top, earth, under, plain: [1, 2, 3, 255] },
            Column { tall: [None, None], tuft: None, top, earth: None, under, plain: [1, 2, 3, 255] },
        ];
        (img, Terrain { columns, grass: None, tall_every: 0, ..Terrain::default() })
    }

    #[test]
    fn a_tileset_terrain_is_built_piece_by_piece_down_from_the_top() {
        let (img, t) = little_terrain();
        let k = 1.5;
        // A flat block whose top is at 150 world px (100 sheet px).
        let poly = [vec2(0.0, 150.0), vec2(48.0, 150.0), vec2(48.0, 300.0), vec2(0.0, 300.0)];
        let (pic, r) = bake_terrain(&poly, k, &t, &img, &[]).unwrap();
        // In sheet pixels, drawn k times bigger, on the sheet's grid.
        assert_eq!(pic.width(), 32);
        let above = terrain_above(&t);
        assert_eq!((r.x, r.y, r.w), (0.0, (100 - above) as f32 * k, 48.0));
        let at = |u: u32, d: i64| pic.get_pixel(u, (above + d) as u32).0;
        // Column 0: its top, earth, under, earth…; a tuft above.
        assert_eq!(
            [at(3, 0), at(3, 8), at(3, 16), at(3, 24)],
            [img.get_pixel(0, 0).0, [90, 40, 0, 255], [240, 120, 0, 255], [90, 40, 0, 255]]
        );
        assert_eq!(at(3, -1), [0, 150, 0, 255], "its tuft above the ground's top");
        // Column 1 has no earth piece: its plain colour; nothing above.
        assert_eq!((at(12, 8), at(12, -1)[3]), ([1, 2, 3, 255], 0));
        // Then the columns go round.
        assert_eq!(at(19, -1), [0, 150, 0, 255]);
        // A ledge: tufts, the top, the piece under.
        let line = [vec2(0.0, 60.0), vec2(48.0, 60.0)];
        let (pic, r) = bake_terrain_ledge(&line, 6.0, k, &t, &img).unwrap();
        assert_eq!((pic.height(), r.y), (24, (38 - 8) as f32 * k));
        assert_eq!(
            [pic.get_pixel(2, 7).0, pic.get_pixel(2, 8).0, pic.get_pixel(2, 16).0],
            [[0, 150, 0, 255], [0, 200, 0, 255], [240, 120, 0, 255]]
        );
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
