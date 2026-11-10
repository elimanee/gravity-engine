//! The zone's sheet (`zone.png` in the Sonic folder, none ships with the
//! program): the pictures its ground and gimmicks are drawn with, each a
//! "tile" the sprite region editor can lay out like a character's
//! animation. The Green Hill Zone objects sheet of The Spriters Resource
//! (864 × 707) is known as it is.

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
];

/// The Green Hill Zone objects sheet: its size and background colours.
pub const GHZ_SIZE: (u32, u32) = (864, 707);
pub const GHZ_BACKGROUND: [[u8; 3]; 2] = [[6, 44, 128], [4, 20, 83]];

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_sheet_has_every_tile_inside_it() {
        let t = ghz();
        assert_eq!(t.len(), TILES.len());
        let (w, h) = (GHZ_SIZE.0 as f32, GHZ_SIZE.1 as f32);
        for (name, _) in TILES {
            let a = &t[*name];
            assert!(!a.pics.is_empty(), "{name}");
            for p in &a.pics {
                assert!(p.src.x >= 0.0 && p.src.y >= 0.0 && p.src.right() <= w && p.src.bottom() <= h, "{name}");
                assert!(p.centre.x >= 0.0 && p.centre.x <= p.src.w && p.centre.y <= p.src.h, "{name}");
            }
        }
        assert_eq!(t["spike_log"].pics.len(), 8, "an eighth of a turn each");
    }
}
