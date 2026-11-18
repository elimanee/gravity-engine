//! The zone's sheet (`zone.png` in the Sonic folder, none ships with the
//! program): the pictures its ground, gimmicks, backdrop and scenery are
//! drawn with, each a "tile" the sprite region editor can lay out like a
//! character's animation. Two sheets are known as they are: the Green Hill
//! Zone objects sheet of The Spriters Resource (864 × 707) and NinjaCakes'
//! tileset in its style (725 × 617), with a backdrop and scenery.

use super::sheet::{Anim, Pic};
use macroquad::prelude::{vec2, Rect};
use std::collections::HashMap;

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
        assert!(backdrop.iter().chain(&SCENERY).all(|t| n.contains_key(*t)), "the tileset's backdrop and scenery");
        assert_eq!(n["sea"].pics.len(), 6);
        // The backdrop's layers repeat across: as wide as each other.
        assert!(["sky", "clouds", "hills", "islands"].iter().all(|l| n[*l].pics[0].src.w == 256.0));
        assert!(known((10, 10)).is_none());
    }
}
