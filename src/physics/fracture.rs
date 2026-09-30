//! Shattering: an object's sprite is cut into Voronoi cells around seeds
//! clustered near the impact, and each non-empty cell becomes a new object
//! (its collider is the convex hull of its pixels).

use image::RgbaImage;

/// Pixels with less alpha than this are empty space.
const SOLID: u8 = 24;
/// Cells with fewer solid pixels than this turn into dust instead of a piece.
const MIN_PIXELS: usize = 40;

/// One piece of a shattered sprite.
pub struct Piece {
    /// The piece's pixels, cropped to their bounding box.
    pub image: RgbaImage,
    /// Centre of the piece relative to the sprite centre, as a fraction of the
    /// sprite size (y down).
    pub offset: (f32, f32),
    /// Size of the piece as a fraction of the sprite size.
    pub scale: (f32, f32),
}

/// Small deterministic generator (the seed makes tests reproducible).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// Cut `img` into up to `count` pieces. `impact` is where it was hit, as a
/// fraction of the sprite size (0‥1, y down); pieces are smaller there.
pub fn shatter(img: &RgbaImage, impact: (f32, f32), count: usize, seed: u64) -> Vec<Piece> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    if w < 2 || h < 2 || count < 2 {
        return vec![];
    }
    let mut rng = Rng(seed | 1);
    // Half the seeds cluster around the impact, the rest spread evenly.
    let seeds: Vec<(f32, f32)> = (0..count)
        .map(|i| {
            if i % 2 == 0 {
                let a = rng.next() * std::f32::consts::TAU;
                let r = rng.next().sqrt() * 0.28;
                ((impact.0 + a.cos() * r).clamp(0.0, 1.0), (impact.1 + a.sin() * r).clamp(0.0, 1.0))
            } else {
                (rng.next(), rng.next())
            }
        })
        .map(|(u, v)| (u * w as f32, v * h as f32))
        .collect();

    // Nearest seed of every solid pixel.
    let mut owner = vec![u8::MAX; w * h];
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x as u32, y as u32)[3] < SOLID {
                continue;
            }
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let best = seeds
                .iter()
                .enumerate()
                .map(|(i, &(sx, sy))| (i, (sx - px).powi(2) + (sy - py).powi(2)))
                .fold((0, f32::MAX), |m, c| if c.1 < m.1 { c } else { m })
                .0;
            owner[y * w + x] = best as u8;
        }
    }
    cells_to_pieces(img, &owner, count)
}

/// Cut a sprite in two along the line through `a` and `b` (pixels). Returns
/// both halves, or nothing when the line misses the solid pixels.
pub fn slice(img: &RgbaImage, a: (f32, f32), b: (f32, f32)) -> Vec<Piece> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if w < 2 || h < 2 || dx.hypot(dy) < 1e-3 {
        return vec![];
    }
    let mut owner = vec![u8::MAX; w * h];
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x as u32, y as u32)[3] >= SOLID {
                let (px, py) = (x as f32 + 0.5 - a.0, y as f32 + 0.5 - a.1);
                owner[y * w + x] = (dx * py - dy * px > 0.0) as u8;
            }
        }
    }
    let pieces = cells_to_pieces(img, &owner, 2);
    if pieces.len() == 2 {
        pieces
    } else {
        vec![]
    }
}

/// One piece per cell (`owner` gives each pixel's cell, `u8::MAX` for
/// empty space); cells that are too small are dropped.
fn cells_to_pieces(img: &RgbaImage, owner: &[u8], count: usize) -> Vec<Piece> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let mut bbox = vec![(usize::MAX, usize::MAX, 0usize, 0usize, 0usize); count];
    for y in 0..h {
        for x in 0..w {
            let Some(b) = bbox.get_mut(owner[y * w + x] as usize) else { continue };
            b.0 = b.0.min(x);
            b.1 = b.1.min(y);
            b.2 = b.2.max(x);
            b.3 = b.3.max(y);
            b.4 += 1;
        }
    }
    let mut pieces = vec![];
    for (i, &(x0, y0, x1, y1, n)) in bbox.iter().enumerate() {
        if n < MIN_PIXELS {
            continue;
        }
        let (pw, ph) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut piece = RgbaImage::new(pw as u32, ph as u32);
        for y in y0..=y1 {
            for x in x0..=x1 {
                if owner[y * w + x] == i as u8 {
                    piece.put_pixel((x - x0) as u32, (y - y0) as u32, *img.get_pixel(x as u32, y as u32));
                }
            }
        }
        pieces.push(Piece {
            image: piece,
            offset: ((x0 + x1 + 1) as f32 / 2.0 / w as f32 - 0.5, (y0 + y1 + 1) as f32 / 2.0 / h as f32 - 0.5),
            scale: (pw as f32 / w as f32, ph as f32 / h as f32),
        });
    }
    pieces
}

/// Number of pieces for a sprite of the given size (px).
pub fn piece_count(w: f32, h: f32) -> usize {
    ((w * h).sqrt() / 22.0).clamp(3.0, 9.0) as usize
}

/// Encode a piece as PNG, so it can be a regular in-memory image source.
pub fn to_png(img: &RgbaImage) -> Option<Vec<u8>> {
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_cuts_in_two_halves() {
        let img = crate::shapes::rasterize(crate::shapes::Shape::Box, 100, (200, 80, 80));
        let solid = img.pixels().filter(|p| p[3] >= SOLID).count();
        // A vertical cut through the middle.
        let halves = slice(&img, (50.0, -10.0), (50.0, 110.0));
        assert_eq!(halves.len(), 2);
        let total: usize = halves.iter().map(|p| p.image.pixels().filter(|q| q[3] >= SOLID).count()).sum();
        assert_eq!(total, solid, "every solid pixel lands in one half");
        assert!(halves[0].offset.0 * halves[1].offset.0 < 0.0, "one half on each side");
        assert!(halves.iter().all(|p| (p.scale.0 - 0.5).abs() < 0.08));
        // A line that misses the shape does not cut it.
        assert!(slice(&img, (-5.0, 0.0), (-5.0, 100.0)).is_empty());
    }

    #[test]
    fn pieces_cover_the_sprite_without_overlap() {
        let img = crate::shapes::rasterize(crate::shapes::Shape::Box, 120, (200, 80, 80));
        let solid = img.pixels().filter(|p| p[3] >= SOLID).count();
        let pieces = shatter(&img, (0.2, 0.3), 7, 42);
        assert!(pieces.len() >= 4, "got {} pieces", pieces.len());
        let covered: usize = pieces.iter().map(|p| p.image.pixels().filter(|q| q[3] >= SOLID).count()).sum();
        // Tiny cells are dropped, so a sliver may be missing.
        assert!(covered <= solid && covered as f32 > solid as f32 * 0.9);
        for p in &pieces {
            assert!(p.offset.0.abs() <= 0.5 && p.offset.1.abs() <= 0.5);
            assert!(p.scale.0 > 0.0 && p.scale.0 <= 1.0 && p.scale.1 > 0.0 && p.scale.1 <= 1.0);
            let png = to_png(&p.image).unwrap();
            let decoded = crate::assets::decode("piece.png", &png, 256, false).unwrap();
            assert!(decoded.hull.is_some(), "pieces get a hull collider");
        }
    }

    #[test]
    fn empty_images_give_no_pieces() {
        assert!(shatter(&RgbaImage::new(50, 50), (0.5, 0.5), 6, 1).is_empty());
    }
}
