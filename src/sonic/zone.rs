//! The zone's sheet (`zone.png` in the Sonic folder, none ships with the
//! program): the pictures its ground, gimmicks, backdrop and scenery are
//! drawn with, each a "tile" the sprite region editor can lay out like a
//! character's animation. Two sheets are known as they are: the Green Hill
//! Zone objects sheet of The Spriters Resource (864 × 707) and NinjaCakes'
//! tileset in its style (725 × 617), with a backdrop and scenery.

use super::sheet::{Anim, Pic};
use macroquad::prelude::{vec2, Rect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A piece of a tileset: its top-left on the sheet (sheet px).
pub type Piece = (u32, u32);

/// A column of a tileset's terrain: its pieces from the top down. The
/// ground's top line is the top of `top`; above it the grass sticks out
/// (tall grass, a tuft), below it the earth goes on down: `earth` (or a
/// plain colour where there is none) and `under`, again and again.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Column {
    pub tall: [Option<Piece>; 2],
    pub tuft: Option<Piece>,
    pub top: Option<Piece>,
    pub earth: Option<Piece>,
    pub under: Option<Piece>,
    /// Where there is no `earth` piece: the colour it is (worked out from
    /// the sheet, the main colour of the top row of `under`).
    #[serde(skip)]
    pub plain: [u8; 4],
}

/// Where a tileset's pieces are on its sheet: their side, the top-left of
/// one, and the step from one to the next (side and gap).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Grid {
    pub piece: u32,
    pub origin: Piece,
    pub step: u32,
}

impl Default for Grid {
    fn default() -> Self {
        Grid { piece: 8, origin: (0, 0), step: 8 }
    }
}

impl Grid {
    /// The piece at sheet point (x, y), if (x, y) is on one (not between).
    pub fn piece_at(&self, x: f32, y: f32) -> Option<Piece> {
        let (ox, oy) = (self.origin.0 as f32, self.origin.1 as f32);
        if x < ox || y < oy || self.step == 0 {
            return None;
        }
        let (step, piece) = (self.step as f32, self.piece as f32);
        let (i, j) = (((x - ox) / step).floor(), ((y - oy) / step).floor());
        let (px, py) = (ox + i * step, oy + j * step);
        (x < px + piece && y < py + piece).then_some((px as u32, py as u32))
    }
}

/// A tileset's terrain: its columns, which go side by side in order (and
/// round again); a piece all grass, where tall grass has no tuft under; in
/// how many stretches of columns one has tall grass (0: none); and where
/// the pieces are on the sheet. Kept beside the zone's sheet as
/// `zone.terrain.json` (the sheet's size with it).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Terrain {
    pub size: [u32; 2],
    pub grid: Grid,
    pub columns: Vec<Column>,
    pub grass: Option<Piece>,
    pub tall_every: u32,
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain { size: [0, 0], grid: Grid::default(), columns: vec![Column::default()], grass: None, tall_every: 2 }
    }
}

impl Terrain {
    /// Whether it can build ground: a column with a top at least.
    pub fn usable(&self) -> bool {
        self.columns.iter().any(|c| c.top.is_some())
    }

    /// Its pieces inside a sheet of `img`'s size, the plain colours worked
    /// out from it.
    pub fn fitted(mut self, img: &image::RgbaImage) -> Terrain {
        let (w, h) = img.dimensions();
        let n = self.grid.piece.clamp(1, 64);
        self.grid.piece = n;
        self.grid.step = self.grid.step.max(n);
        let inside = |p: Option<Piece>| p.filter(|&(x, y)| x + n <= w && y + n <= h);
        self.grass = inside(self.grass);
        for c in &mut self.columns {
            c.tall = [inside(c.tall[0]), inside(c.tall[1])];
            (c.tuft, c.top, c.earth, c.under) = (inside(c.tuft), inside(c.top), inside(c.earth), inside(c.under));
            // The colour most of the top row of `under` is.
            let mut counts: Vec<([u8; 4], usize)> = vec![];
            if let Some((x, y)) = c.under {
                for px in x..x + n {
                    let col = img.get_pixel(px, y).0;
                    match counts.iter_mut().find(|(k, _)| *k == col) {
                        Some((_, m)) => *m += 1,
                        None => counts.push((col, 1)),
                    }
                }
            }
            c.plain = counts.iter().max_by_key(|(_, m)| *m).map_or([0, 0, 0, 0], |(col, _)| *col);
        }
        self.columns.truncate(256);
        self
    }
}

/// The file the zone's terrain is kept in.
pub fn terrain_path() -> Option<std::path::PathBuf> {
    super::sheet::dir().map(|d| d.join("zone.terrain.json"))
}

/// The terrain kept for a zone sheet of this size, if any.
pub fn read_terrain(size: (u32, u32)) -> Option<Terrain> {
    let text = std::fs::read_to_string(terrain_path()?).ok()?;
    let t: Terrain = serde_json::from_str(&text).ok()?;
    (t.size == [size.0, size.1]).then_some(t)
}

/// Keep `t` as the zone's terrain (none: the file goes, back to the sheet's
/// own, if the game knows one).
pub fn save_terrain(t: Option<&Terrain>) -> Result<(), String> {
    let p = terrain_path().ok_or("no config directory")?;
    match t {
        Some(t) => {
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            let json = serde_json::to_string_pretty(t).map_err(|e| e.to_string())?;
            std::fs::write(&p, json).map_err(|e| e.to_string())
        }
        None => match std::fs::remove_file(&p) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        },
    }
}

/// The terrain a zone sheet is built with: the one kept for it, else the
/// one the game knows for it (none: its ground is drawn from its tiles).
pub fn terrain_for(img: &image::RgbaImage) -> Option<Terrain> {
    read_terrain(img.dimensions()).or_else(|| ninja_terrain(img)).map(|t| t.fitted(img)).filter(Terrain::usable)
}

/// NinjaCakes' terrain: 26 columns of 8 × 8 pieces a pixel apart (its
/// "main tiles", which its "grouped" strip shows put together), from the
/// sheet with its background cleared (a piece is there when it is not all
/// clear).
pub fn ninja_terrain(img: &image::RgbaImage) -> Option<Terrain> {
    if img.dimensions() != NINJA_SIZE {
        return None;
    }
    let n = 8;
    let there =
        |x: u32, y: u32| (y..y + n).any(|py| (x..x + n).any(|px| img.get_pixel(px, py)[3] > 0)).then_some((x, y));
    let full = |x: u32, y: u32| (y..y + n).all(|py| (x..x + n).all(|px| img.get_pixel(px, py)[3] > 0));
    // Rows: tall grass (two), tufts, grass into earth, earth, earth into
    // the next colour.
    let [t2, t1, tuft, top, earth, under] = [40, 49, 58, 67, 76, 85];
    let columns = (0..26)
        .map(|i| {
            let x = 10 + 9 * i;
            Column {
                tall: [there(x, t2), there(x, t1)],
                tuft: there(x, tuft),
                top: there(x, top),
                earth: there(x, earth),
                under: there(x, under),
                plain: [0; 4],
            }
        })
        .collect();
    let grass = (0..26).map(|i| 10 + 9 * i).find(|&x| full(x, tuft)).map(|x| (x, tuft));
    let grid = Grid { piece: n, origin: (1, 4), step: 9 };
    let size = [NINJA_SIZE.0, NINJA_SIZE.1];
    Some(Terrain { size, grid, columns, grass, tall_every: 2 }.fitted(img))
}

/// The tiles a zone's sheet can have, and what each is for.
pub const TILES: &[(&str, &str)] = &[
    ("grass", "The ground's top, along it (the cross: where the ground is)"),
    ("ground", "Inside the ground, repeated all over"),
    ("platform", "A ledge in the air, along it (the cross: its top)"),
    ("log", "A log of a bridge (they dip underfoot)"),
    ("post_left", "The post at a bridge's left end"),
    ("post_right", "The post at a bridge's right end"),
    ("swing", "A platform swinging on a chain"),
    ("chain", "A link of its chain"),
    ("anchor", "What the chain hangs from"),
    ("ledge", "A crumbling ledge, a column a picture (in turn)"),
    ("wall", "A wall broken by rolling into it"),
    ("spike_log", "A log with spikes going round: a picture each eighth of a turn"),
    ("rock", "A rock"),
    ("sky", "The backdrop's sky, repeated across (above it, its top colour)"),
    ("clouds", "Clouds over the sky, drifting (parallax)"),
    ("hills", "Far hills or mountains (parallax)"),
    ("islands", "The nearest of the backdrop, its bottom on the horizon"),
    ("sea", "The sea below the horizon: a picture each moment, in turn"),
    ("tree", "A tree on the ground: its pictures together, each by its cross (on the ground)"),
    ("sunflower", "A flower on the ground: its pictures together, each by its cross"),
    ("totem", "A totem pole on the ground: its pictures together, each by its cross"),
    ("plant", "A small plant on the ground: its pictures together, each by its cross"),
];

/// The tiles of scenery put along the ground.
pub const SCENERY: [&str; 4] = ["tree", "sunflower", "totem", "plant"];

/// The Green Hill Zone objects sheet: its size and background colours.
pub const GHZ_SIZE: (u32, u32) = (864, 707);
pub const GHZ_BACKGROUND: [[u8; 3]; 2] = [[6, 44, 128], [4, 20, 83]];

/// NinjaCakes' tileset: its size and background colours (the page, and
/// behind the pieces).
pub const NINJA_SIZE: (u32, u32) = (725, 617);
pub const NINJA_BACKGROUND: [[u8; 3]; 2] = [[238, 255, 237], [193, 242, 191]];

/// A known sheet's background colours and tiles.
pub type Layout = (Vec<[u8; 3]>, HashMap<String, Anim>);

/// A zone sheet the game knows, by its size: its background colours and
/// its tiles.
pub fn known(size: (u32, u32)) -> Option<Layout> {
    match size {
        GHZ_SIZE => Some((GHZ_BACKGROUND.to_vec(), ghz())),
        NINJA_SIZE => Some((NINJA_BACKGROUND.to_vec(), ninja())),
        _ => None,
    }
}

fn pic(x: f32, y: f32, w: f32, h: f32, cx: f32, cy: f32) -> Pic {
    Pic::new(Rect::new(x, y, w, h), vec2(cx, cy))
}

/// Its tiles: where each picture is and its centre.
pub fn ghz() -> HashMap<String, Anim> {
    let spikes = [(576.0, 436.0), (712.0, 436.0), (576.0, 476.0), (712.0, 476.0)]
        .into_iter()
        .chain([(576.0, 516.0), (712.0, 516.0), (576.0, 556.0), (712.0, 556.0)])
        .map(|(x, y)| pic(x, y, 128.0, 32.0, 64.0, 16.0))
        .collect();
    let tiles: [(&str, Vec<Pic>); 13] = [
        ("grass", vec![pic(492.0, 212.0, 64.0, 22.0, 32.0, 6.0)]),
        ("ground", vec![pic(300.0, 551.0, 32.0, 32.0, 16.0, 16.0)]),
        ("platform", vec![pic(300.0, 214.0, 64.0, 30.0, 32.0, 6.0)]),
        ("log", vec![pic(64.0, 223.0, 16.0, 16.0, 8.0, 8.0)]),
        ("post_left", vec![pic(24.0, 223.0, 32.0, 16.0, 8.0, 8.0)]),
        ("post_right", vec![pic(88.0, 223.0, 32.0, 16.0, 24.0, 8.0)]),
        ("swing", vec![pic(576.0, 212.0, 48.0, 16.0, 24.0, 8.0)]),
        ("chain", vec![pic(636.0, 215.0, 16.0, 16.0, 8.0, 8.0)]),
        ("anchor", vec![pic(664.0, 215.0, 16.0, 16.0, 8.0, 8.0)]),
        ("ledge", vec![pic(88.0, 557.0, 16.0, 48.0, 8.0, 24.0), pic(104.0, 557.0, 16.0, 48.0, 8.0, 24.0)]),
        ("wall", vec![pic(284.0, 551.0, 32.0, 64.0, 16.0, 32.0)]),
        ("spike_log", spikes),
        ("rock", vec![pic(300.0, 349.0, 48.0, 32.0, 24.0, 16.0)]),
    ];
    tiles.into_iter().map(|(n, pics)| (n.to_string(), Anim { pics, ..Default::default() })).collect()
}

/// NinjaCakes' tileset: 8 × 8 pieces, some laid out together. The ground's
/// top is its grouped strip (bumps and tall grass above, grass into earth
/// below), the earth a block of it; scenery pictures share one cross, where
/// they stand on the ground.
pub fn ninja() -> HashMap<String, Anim> {
    let sea = (0..6).map(|i| pic(459.0 + 20.0 * i as f32, 500.0, 16.0, 107.0, 8.0, 0.0)).collect();
    let trunk = |y| pic(107.0, 288.0, 8.0, 8.0, 4.0, y);
    let stem = |y| pic(121.0, 206.0, 8.0, 8.0, 4.0, y);
    let tiles: [(&str, Vec<Pic>); 14] = [
        ("grass", vec![pic(247.0, 40.0, 208.0, 32.0, 104.0, 24.0)]),
        ("ground", vec![pic(343.0, 72.0, 32.0, 16.0, 16.0, 8.0)]),
        ("platform", vec![pic(247.0, 56.0, 208.0, 24.0, 104.0, 8.0)]),
        // A column of earth each (the grass on the ledge's top).
        ("ledge", vec![pic(343.0, 56.0, 16.0, 40.0, 8.0, 32.0), pic(359.0, 56.0, 16.0, 40.0, 8.0, 32.0)]),
        ("wall", vec![pic(10.0, 144.0, 16.0, 56.0, 8.0, 28.0)]),
        ("rock", vec![pic(155.0, 146.0, 32.0, 24.0, 16.0, 12.0)]),
        ("sky", vec![pic(459.0, 36.0, 256.0, 112.0, 128.0, 112.0)]),
        ("clouds", vec![pic(459.0, 152.0, 256.0, 112.0, 128.0, 112.0)]),
        ("hills", vec![pic(459.0, 268.0, 256.0, 112.0, 128.0, 112.0)]),
        ("islands", vec![pic(459.0, 384.0, 256.0, 112.0, 128.0, 112.0)]),
        ("sea", sea),
        // A palm on two more pieces of its trunk.
        ("tree", vec![pic(116.0, 233.0, 71.0, 63.0, 36.0, 79.0), trunk(16.0), trunk(8.0)]),
        // Its head on its stalk, leaves either side at the bottom.
        (
            "sunflower",
            vec![
                pic(139.0, 199.0, 24.0, 24.0, 12.0, 48.0),
                stem(24.0),
                stem(16.0),
                pic(112.0, 215.0, 8.0, 8.0, 12.0, 8.0),
                pic(121.0, 215.0, 8.0, 8.0, 4.0, 8.0),
                pic(130.0, 215.0, 8.0, 8.0, -4.0, 8.0),
            ],
        ),
        (
            "totem",
            vec![
                pic(10.0, 207.0, 16.0, 16.0, 8.0, 48.0),
                pic(27.0, 207.0, 16.0, 16.0, 8.0, 32.0),
                pic(44.0, 207.0, 16.0, 16.0, 8.0, 16.0),
            ],
        ),
    ];
    let mut out: HashMap<String, Anim> =
        tiles.into_iter().map(|(n, pics)| (n.to_string(), Anim { pics, ..Default::default() })).collect();
    out.insert("plant".into(), Anim { pics: vec![pic(62.0, 214.0, 14.0, 9.0, 7.0, 9.0)], ..Default::default() });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_sheets_have_their_tiles_inside_them() {
        for size in [GHZ_SIZE, NINJA_SIZE] {
            let (background, t) = known(size).unwrap();
            assert_eq!(background.len(), 2);
            let (w, h) = (size.0 as f32, size.1 as f32);
            for (name, a) in &t {
                assert!(TILES.iter().any(|(n, _)| n == name), "{name} is a tile the game knows");
                assert!(!a.pics.is_empty(), "{name}");
                for p in &a.pics {
                    assert!(p.src.x >= 0.0 && p.src.y >= 0.0 && p.src.right() <= w && p.src.bottom() <= h, "{name}");
                    // Scenery pictures share a cross on the ground, which can
                    // be outside some of them.
                    if !SCENERY.contains(&name.as_str()) {
                        assert!(p.centre.x >= 0.0 && p.centre.x <= p.src.w && p.centre.y <= p.src.h, "{name}");
                    }
                }
            }
        }
        let g = ghz();
        assert_eq!(g.len(), 13, "the Green Hill sheet's ground and gimmicks");
        assert_eq!(g["spike_log"].pics.len(), 8, "an eighth of a turn each");
        let n = ninja();
        let backdrop = ["sky", "clouds", "hills", "islands", "sea"];
        // Its terrain: 26 columns found on the sheet, a pixel apart.
        let mut img = image::RgbaImage::new(NINJA_SIZE.0, NINJA_SIZE.1);
        let mut paint = |x: u32, y: u32, c: [u8; 4]| {
            for py in y..y + 8 {
                for px in x..x + 8 {
                    img.put_pixel(px, py, image::Rgba(c));
                }
            }
        };
        for i in 0..26 {
            let x = 10 + 9 * i;
            paint(x, 67, [0, 200, 0, 255]);
            paint(x, 85, [240, 120, 0, 255]);
            if i % 2 == 0 {
                paint(x, 76, [90, 40, 0, 255]);
            }
        }
        paint(10 + 9 * 13, 58, [0, 150, 0, 255]);
        let t = ninja_terrain(&img).unwrap();
        assert_eq!(t.columns.len(), 26);
        assert_eq!((t.columns[0].earth, t.columns[1].earth), (Some((10, 76)), None));
        assert_eq!((t.columns[1].plain, t.columns[13].tuft), ([240, 120, 0, 255], Some((127, 58))));
        assert_eq!(t.grass, Some((127, 58)), "a tuft all grass, for under the tall grass");
        assert!(t.usable() && ninja_terrain(&image::RgbaImage::new(10, 10)).is_none());
        // Its pieces are on its grid: a pixel apart.
        assert_eq!(t.grid.piece_at(13.0, 70.0), Some((10, 67)));
        assert_eq!(t.grid.piece_at(18.5, 70.0), None, "between two pieces");
        // Kept as a file and read back the same (the plain colours worked
        // out again from the sheet).
        let json = serde_json::to_string(&t).unwrap();
        let back: Terrain = serde_json::from_str(&json).unwrap();
        assert_eq!(back.fitted(&img), t);
        // Pieces outside the sheet are dropped.
        let odd = Terrain { columns: vec![Column { top: Some((9000, 0)), ..Column::default() }], ..Terrain::default() };
        assert!(!odd.fitted(&img).usable());
        assert!(backdrop.iter().chain(&SCENERY).all(|t| n.contains_key(*t)), "the tileset's backdrop and scenery");
        assert_eq!(n["sea"].pics.len(), 6);
        // The backdrop's layers repeat across: as wide as each other.
        assert!(["sky", "clouds", "hills", "islands"].iter().all(|l| n[*l].pics[0].src.w == 256.0));
        assert!(known((10, 10)).is_none());
    }
}
