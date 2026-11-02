//! The sheet generator: a character's animations laid out as a sprite sheet
//! the way fans make theirs. A banner carries its name, where its pictures
//! come from and the credits, beside its palette; below, an animation a
//! row, each picture in its own box on a dark page. The regions that cut it
//! up again come with it, so the sheet works as it is. The pictures are the
//! game's own drawing of the character, or the ones its sheet uses.

use super::regions::{AnimRegions, Region, Regions};
use super::sheet::{upload, Anim, Pic};
use super::{art, Character, Sonic, SCALE};
use crate::ui::theme;
use macroquad::prelude::*;
use std::collections::{BTreeMap, HashMap};

/// The page and the boxes (both cut out), and the banner.
pub const PAGE: [u8; 3] = [18, 98, 120];
pub const BOX: [u8; 3] = [46, 190, 222];
const BANNER: [u8; 3] = [24, 44, 200];

/// Around a picture in its box, between boxes, around the page.
const PAD: f32 = 2.0;
const GAP: f32 = 4.0;
const MARGIN: f32 = 10.0;
/// A row's label, and between rows.
const LABEL_H: f32 = 15.0;
const ROW_GAP: f32 = 8.0;
const BANNER_H: f32 = 96.0;
/// The sheet's width: from its widest row, within these.
const MIN_W: f32 = 560.0;
const MAX_W: f32 = 1024.0;

/// The game's drawing: sheet pixels per guide pixel, and the room each
/// picture is drawn in (guide px).
pub const DRAWN_SCALE: f32 = 2.0;
const CELL: f32 = 96.0;

fn rgb(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

/// Where everything goes on a sheet, and the regions that cut it up.
#[derive(Debug)]
pub struct Layout {
    pub size: (u32, u32),
    /// Each row's label and where it is written.
    pub labels: Vec<(String, Vec2)>,
    /// Each picture placed: where it is on the source, and its box.
    pub boxes: Vec<(Rect, Rect)>,
    pub regions: Regions,
}

fn key(r: Rect) -> [i32; 4] {
    [r.x, r.y, r.w, r.h].map(|v| v.round() as i32)
}

/// Lay `rows` (animations, their pictures on one source) out: a row per
/// animation (and one for the pieces drawn behind), a picture repeated in
/// a row boxed once. `scale`: the source's pixels per guide pixel.
pub fn lay_out(rows: &[(String, Anim)], scale: f32) -> Layout {
    let unique = |pics: &[Pic]| {
        let mut seen = vec![];
        for p in pics {
            if !seen.iter().any(|q: &Pic| key(q.src) == key(p.src)) {
                seen.push(*p);
            }
        }
        seen
    };
    let lines: Vec<(String, Vec<Pic>)> = rows
        .iter()
        .flat_map(|(name, a)| {
            let body = (!a.pics.is_empty()).then(|| (name.clone(), unique(&a.pics)));
            let behind = (!a.behind.is_empty()).then(|| (format!("{name}  ·  behind"), unique(&a.behind)));
            body.into_iter().chain(behind)
        })
        .collect();
    let natural = |pics: &[Pic]| pics.iter().map(|p| p.src.w + 2.0 * PAD + GAP).sum::<f32>() - GAP;
    let widest = lines.iter().map(|(_, pics)| natural(pics)).fold(0.0, f32::max);
    let width = (widest + 2.0 * MARGIN).clamp(MIN_W, MAX_W).ceil();
    let (mut labels, mut boxes) = (vec![], vec![]);
    // Each line's boxes, by where the picture is on the source.
    let mut placed: Vec<HashMap<[i32; 4], Rect>> = vec![];
    let mut y = BANNER_H + MARGIN;
    for (label, pics) in &lines {
        labels.push((label.clone(), vec2(MARGIN, y + LABEL_H - 4.0)));
        y += LABEL_H;
        let (mut x, mut row_h) = (MARGIN, 0.0f32);
        let mut mine = HashMap::new();
        for p in pics {
            let (bw, bh) = (p.src.w + 2.0 * PAD, p.src.h + 2.0 * PAD);
            if x + bw > width - MARGIN && x > MARGIN {
                (x, y, row_h) = (MARGIN, y + row_h + GAP, 0.0);
            }
            let b = Rect::new(x, y, bw, bh);
            boxes.push((p.src, b));
            mine.insert(key(p.src), b);
            x += bw + GAP;
            row_h = row_h.max(bh);
        }
        placed.push(mine);
        y += row_h + ROW_GAP;
    }
    let height = (y - ROW_GAP + MARGIN).ceil();
    // The regions: each picture's box, its centre from the box's corner.
    let mut animations = BTreeMap::new();
    let mut line = 0;
    for (name, a) in rows {
        let mut to_regions = |pics: &[Pic]| -> Vec<Region> {
            let boxes = &placed[line];
            line += 1;
            pics.iter()
                .map(|p| {
                    let b = boxes[&key(p.src)];
                    Region {
                        rect: [b.x, b.y, b.w, b.h],
                        centre: [p.centre.x + PAD, p.centre.y + PAD],
                        flip: p.flip,
                        hold: p.hold,
                    }
                })
                .collect()
        };
        let frames = if a.pics.is_empty() { vec![] } else { to_regions(&a.pics) };
        let behind = if a.behind.is_empty() { vec![] } else { to_regions(&a.behind) };
        if frames.is_empty() && behind.is_empty() {
            continue;
        }
        let (fps, once, loop_from, repeats) = (a.fps, a.once, a.loop_from, a.repeats.clone());
        animations.insert(name.clone(), AnimRegions { frames, behind, fps, once, loop_from, repeats });
    }
    let size = (width as u32, height as u32);
    let regions = Regions {
        size: [size.0, size.1],
        background: vec![PAGE, BOX],
        soft: false,
        scale: (scale != 1.0).then_some(scale),
        animations,
    };
    Layout { size, labels, boxes, regions }
}

/// The colours a sheet's pictures use the most (opaque pixels), most used first.
pub fn palette(img: &image::RgbaImage, rects: impl Iterator<Item = Rect>, most: usize) -> Vec<[u8; 3]> {
    let mut count: HashMap<[u8; 3], u32> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for r in rects {
        if !seen.insert(key(r)) {
            continue;
        }
        let (x0, y0) = (r.x.max(0.0) as u32, r.y.max(0.0) as u32);
        let (x1, y1) = (((r.x + r.w) as u32).min(img.width()), ((r.y + r.h) as u32).min(img.height()));
        for y in y0..y1 {
            for x in x0..x1 {
                let p = img.get_pixel(x, y);
                if p[3] > 200 {
                    // Shades this close are one colour (lossy pictures have thousands).
                    *count.entry([p[0], p[1], p[2]].map(|v| (v & 0xF0) | 0x08)).or_default() += 1;
                }
            }
        }
    }
    let mut colours: Vec<([u8; 3], u32)> = count.into_iter().collect();
    colours.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    colours.into_iter().take(most).map(|(c, _)| c).collect()
}

/// Draw into a picture `w` × `h` px, `units` px a unit, and read it back
/// (`top_left`: the colour its top-left corner is drawn).
fn render(w: u32, h: u32, units: f32, top_left: [u8; 3], draw: impl FnOnce()) -> image::RgbaImage {
    let rt = render_target(w, h);
    rt.texture.set_filter(FilterMode::Nearest);
    let mut cam = Camera2D::from_display_rect(Rect::new(0.0, 0.0, w as f32 / units, h as f32 / units));
    cam.render_target = Some(rt.clone());
    set_camera(&cam);
    draw();
    unsafe { get_internal_gl() }.flush();
    set_default_camera();
    let data = rt.texture.get_texture_data();
    let mut img = image::RgbaImage::from_raw(w, h, data.bytes).unwrap_or_else(|| image::RgbaImage::new(w, h));
    // Render targets may come back upside down: the top-left corner says.
    let at = |x: u32, y: u32| {
        let p = img.get_pixel(x, y);
        [p[0], p[1], p[2]]
    };
    if at(0, 0) != top_left && at(0, h - 1) == top_left {
        image::imageops::flip_vertical_in_place(&mut img);
    }
    img
}

/// What the banner says.
pub struct Banner {
    pub title: String,
    pub lines: Vec<String>,
    pub credits: Vec<String>,
    pub palette: Vec<[u8; 3]>,
}

/// The sheet: banner, labels and boxes, the pictures taken from `texture`.
pub fn draw_sheet(layout: &Layout, texture: &Texture2D, banner: &Banner) -> image::RgbaImage {
    let (w, h) = layout.size;
    render(w, h, 1.0, BANNER, || {
        clear_background(rgb(PAGE));
        let wf = w as f32;
        draw_rectangle(0.0, 0.0, wf, BANNER_H, rgb(BANNER));
        // A strip of the page's colour under the banner, its edge.
        draw_rectangle(0.0, BANNER_H - 3.0, wf, 3.0, Color::from_rgba(12, 28, 140, 255));
        let white = Color::from_rgba(250, 250, 255, 255);
        theme::text_bold(&banner.title, 14.0, 34.0, 28.0, white);
        let mut y = 52.0;
        for l in &banner.lines {
            theme::text(l, 16.0, y, 12.0, white);
            y += 15.0;
        }
        // On the right: the palette, the credits under it.
        let right = wf - 14.0;
        let pal_w = (banner.palette.len().min(16) as f32) * 10.0;
        let credits_w = banner.credits.iter().map(|c| theme::measure(c, 11.0)).fold(0.0, f32::max);
        let x = right - pal_w.max(credits_w);
        for (i, c) in banner.palette.iter().enumerate() {
            let (col, row) = ((i % 16) as f32, (i / 16) as f32);
            draw_rectangle(x + col * 10.0, 12.0 + row * 10.0, 9.0, 9.0, rgb(*c));
        }
        let mut y = 50.0;
        for c in &banner.credits {
            theme::text(c, x, y, 11.0, white);
            y += 14.0;
        }
        let label = Color::from_rgba(214, 240, 248, 255);
        for (text, at) in &layout.labels {
            theme::text_bold(text, at.x, at.y, 11.0, label);
        }
        for (src, b) in &layout.boxes {
            draw_rectangle(b.x, b.y, b.w, b.h, rgb(BOX));
            draw_texture_ex(
                texture,
                b.x + PAD,
                b.y + PAD,
                WHITE,
                DrawTextureParams { source: Some(*src), dest_size: Some(src.size()), ..Default::default() },
            );
        }
    })
}

/// The animations the game draws for `c`, and how many pictures each.
pub fn drawn_animations(c: Character) -> Vec<(&'static str, usize)> {
    let mut out = vec![
        ("idle", 1),
        ("look_up", 1),
        ("crouch", 1),
        ("walk", 8),
        ("run", 4),
        ("skid", 1),
        ("push", 4),
        ("roll", 4),
        ("spindash", 4),
        ("spring", 1),
        ("hurt", 1),
        ("dead", 1),
    ];
    out.extend_from_slice(match c {
        Character::Sonic => &[],
        Character::Tails => &[("fly", 4), ("fly_tired", 4)],
        Character::Knuckles => &[("glide", 1), ("slide", 1), ("drop", 1), ("climb", 1), ("climb_up", 1)],
        Character::Amy => &[("hammer", 4), ("hammer_air", 4)],
        Character::Exe => &[("warp", 1), ("chase", 4), ("laugh", 1)],
    });
    out
}

/// `c` doing `name`, picture `k` of `n`, at `at`; and the time to draw it at.
pub fn posed(c: Character, name: &str, k: usize, n: usize, at: Vec2) -> (Sonic, f32) {
    use std::f32::consts::{PI, TAU};
    let mut s = Sonic::as_character(at, c);
    (s.grounded, s.facing) = (true, 1.0);
    let f = k as f32 / n.max(1) as f32;
    let mut t = 0.0;
    match name {
        "look_up" => s.looking_up = true,
        "crouch" => s.crouching = true,
        "walk" => (s.gsp, s.stride) = (3.0, f * TAU),
        // Running legs go round twice as fast.
        "run" | "chase" => (s.gsp, s.stride) = (7.0, f * PI),
        "skid" => (s.gsp, s.skidding) = (4.0, true),
        "push" => (s.gsp, s.pushing, s.stride) = (0.5, true, f * TAU),
        // Five quills: a fifth of a turn brings the ball round.
        "roll" => (s.rolling, s.spin) = (true, f * TAU / 5.0),
        "spindash" => (s.spindash, s.spin) = (Some(4.0), f * TAU / 5.0),
        "spring" => (s.grounded, s.ysp) = (false, -6.0),
        "hurt" => (s.grounded, s.hurt, s.ysp) = (false, true, -3.0),
        "dead" => (s.grounded, s.dead, s.ysp) = (false, true, -3.0),
        "fly" | "fly_tired" => {
            let tired = name == "fly_tired";
            (s.grounded, s.flying) = (false, Some(if tired { 100_000 } else { 0 }));
            // The two tails whirl half a turn apart.
            t = f * PI / if tired { 8.0 } else { 26.0 };
        }
        "glide" => (s.grounded, s.gliding) = (false, Some(4.0)),
        "slide" => s.sliding = true,
        "drop" => (s.grounded, s.dropping) = (false, true),
        "climb" => (s.grounded, s.climbing) = (false, Some(0.0)),
        "climb_up" => (s.grounded, s.climb_up) = (false, Some((9, at, at))),
        "hammer" => s.hammer = Some(((k + 1) as f32 / n.max(1) as f32 * 22.0) as u32),
        // In the air the hammer goes round with the spin.
        "hammer_air" => (s.grounded, s.hammer, s.hammer_air, s.spin) = (false, Some(5), true, f * PI),
        "warp" => (s.grounded, s.warp) = (false, Some(0)),
        _ => {}
    }
    (s, t)
}

/// A drawn picture made solid: the drawing's see-through parts (blurs,
/// dust) are kept, as opaque, when they mostly cover their pixel, else
/// dropped (a sheet's pictures have a background, not shades of it).
fn solidify(img: &mut image::RgbaImage) {
    for p in img.pixels_mut() {
        let a = p[3] as f32 / 255.0;
        if a >= 1.0 {
            continue;
        }
        // Drawn once over nothing: colour × a, alpha a².
        let cover = a.sqrt();
        if cover >= 0.5 {
            for k in 0..3 {
                p[k] = (p[k] as f32 / cover).min(255.0) as u8;
            }
            p[3] = 255;
        } else {
            *p = image::Rgba([0, 0, 0, 0]);
        }
    }
}

/// The game's drawing of `c`: every animation in [`drawn_animations`], as
/// a picture of all its poses and the animations cut from it.
pub fn draw_character(c: Character) -> (image::RgbaImage, Vec<(String, Anim)>) {
    let poses: Vec<(&str, usize, usize)> =
        drawn_animations(c).into_iter().flat_map(|(name, n)| (0..n).map(move |k| (name, k, n))).collect();
    let cell = (CELL * DRAWN_SCALE) as u32;
    let cols = 10u32;
    let rows = (poses.len() as u32).div_ceil(cols);
    let (w, h) = (cols * cell, rows * cell + 2);
    // World px → sheet px.
    let zoom = DRAWN_SCALE / SCALE;
    const MARK: [u8; 3] = [255, 0, 255];
    let mut img = render(w, h, zoom, MARK, || {
        clear_background(Color::new(0.0, 0.0, 0.0, 0.0));
        // A mark in the corner (to tell which way up the picture comes back).
        draw_rectangle(0.0, 0.0, 1.0 / zoom, 1.0 / zoom, rgb(MARK));
        for (i, &(name, k, n)) in poses.iter().enumerate() {
            let (col, row) = ((i as u32 % cols) as f32, (i as u32 / cols) as f32);
            let at = vec2((col + 0.5) * cell as f32, 2.0 + (row + 0.5) * cell as f32) / zoom;
            let (s, t) = posed(c, name, k, n, at);
            art::draw_at(&s, t);
        }
    });
    img.put_pixel(0, 0, image::Rgba([0, 0, 0, 0]));
    solidify(&mut img);
    // Each pose: tight around what was drawn, centred where he stood.
    let mut anims: Vec<(String, Anim)> = vec![];
    for (i, &(name, k, _)) in poses.iter().enumerate() {
        let (cx, cy) = ((i as u32 % cols) * cell, 2 + (i as u32 / cols) * cell);
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for y in cy..cy + cell {
            for x in cx..cx + cell {
                if img.get_pixel(x, y)[3] > 0 {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
                }
            }
        }
        if x0 == u32::MAX {
            continue;
        }
        let src = Rect::new(x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32);
        let centre = vec2(cx as f32 + cell as f32 / 2.0 - x0 as f32, cy as f32 + cell as f32 / 2.0 - y0 as f32);
        if k == 0 {
            anims.push((name.to_string(), Anim::default()));
        }
        if let Some((_, a)) = anims.last_mut().filter(|(n, _)| n == name) {
            a.pics.push(Pic::new(src, centre));
        }
    }
    (img, anims)
}

/// A sheet made: the picture and its regions.
pub struct Made {
    pub image: image::RgbaImage,
    pub regions: Regions,
}

/// A sheet of `c` as the game draws it.
pub fn from_drawing(c: Character) -> Made {
    let (atlas, anims) = draw_character(c);
    let texture = upload(&atlas);
    let layout = lay_out(&anims, DRAWN_SCALE);
    let rects = anims.iter().flat_map(|(_, a)| a.pics.iter().map(|p| p.src));
    let banner = Banner {
        title: c.label().to_uppercase(),
        lines: vec![
            "As Gravity Engine draws it".to_string(),
            "Made with Gravity Engine's sheet generator".to_string(),
            format!("{} pixels to one of the game's", DRAWN_SCALE),
        ],
        credits: vec![
            "Credits:".to_string(),
            "Drawn with shapes by Gravity Engine".to_string(),
            "Paint over the pictures, keep them in their boxes".to_string(),
        ],
        palette: palette(&atlas, rects, 32),
    };
    Made { image: draw_sheet(&layout, &texture, &banner), regions: layout.regions }
}

/// A sheet of the pictures `anims` use on `image` (a sheet with its
/// background cut out), tidied: each in its own box, an animation a row.
pub fn from_sheet(c: Character, name: &str, image: &image::RgbaImage, anims: &[(String, Anim)], scale: f32) -> Made {
    // Soft edges (lossy sheets) made solid or clear, not tinted by the boxes.
    let mut image = image.clone();
    for p in image.pixels_mut() {
        p[3] = if p[3] >= 128 { 255 } else { 0 };
    }
    let texture = upload(&image);
    let layout = lay_out(anims, scale);
    let rects = anims.iter().flat_map(|(_, a)| a.pics.iter().chain(&a.behind).map(|p| p.src));
    let banner = Banner {
        title: c.label().to_uppercase(),
        lines: vec![
            format!("The pictures {name} uses"),
            "Made with Gravity Engine's sheet generator".to_string(),
            format!("{} pixels to one of the game's", scale),
        ],
        credits: vec![
            "Credits:".to_string(),
            format!("Whoever drew {name}"),
            "(see where the sheet came from)".to_string(),
        ],
        palette: palette(&image, rects, 32),
    };
    Made { image: draw_sheet(&layout, &texture, &banner), regions: layout.regions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sonic::sheet::Repeat;

    fn pic(x: f32, w: f32, h: f32) -> Pic {
        Pic::new(Rect::new(x, 0.0, w, h), vec2(w / 2.0, h - 19.0))
    }

    #[test]
    fn rows_are_laid_out_and_cut_up_again() {
        // Walk: three pictures, the first used twice; Tails' tails behind.
        let walk = Anim {
            pics: vec![pic(0.0, 30.0, 40.0), pic(40.0, 30.0, 40.0), pic(0.0, 30.0, 40.0), pic(80.0, 34.0, 42.0)],
            fps: Some(12.0),
            loop_from: 1,
            repeats: vec![Repeat { from: 1, to: 2, times: 3 }],
            ..Default::default()
        };
        let idle =
            Anim { pics: vec![pic(120.0, 30.0, 40.0)], behind: vec![pic(160.0, 20.0, 20.0)], ..Default::default() };
        let rows = vec![("idle".to_string(), idle), ("walk".to_string(), walk), ("bored".to_string(), Anim::default())];
        let l = lay_out(&rows, 2.0);
        // Labels: idle, its tails, walk (an empty animation has no row).
        let labels: Vec<&str> = l.labels.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(labels, ["idle", "idle  ·  behind", "walk"]);
        // A picture repeated in its row is boxed once.
        assert_eq!(l.boxes.len(), 1 + 1 + 3);
        // Boxes lie on the page, below the banner, apart from each other.
        let (w, h) = (l.size.0 as f32, l.size.1 as f32);
        assert!((MIN_W..=MAX_W).contains(&w));
        for (i, (src, b)) in l.boxes.iter().enumerate() {
            assert_eq!((b.w, b.h), (src.w + 2.0 * PAD, src.h + 2.0 * PAD));
            assert!(b.x >= MARGIN && b.right() <= w - MARGIN + 0.01 && b.y >= BANNER_H && b.bottom() <= h);
            for (_, c) in &l.boxes[i + 1..] {
                assert!(!b.overlaps(c), "{b:?} and {c:?}");
            }
        }
        // The regions: the same pictures in the same order, on the new sheet.
        let r = &l.regions;
        assert_eq!(r.size, [l.size.0, l.size.1]);
        assert_eq!((r.background.as_slice(), r.scale), (&[PAGE, BOX][..], Some(2.0)));
        let walk = &r.animations["walk"];
        assert_eq!(walk.frames.len(), 4);
        assert_eq!(walk.frames[0].rect, walk.frames[2].rect, "the same picture, the same box");
        assert_eq!(walk.frames[0].centre, [15.0 + PAD, 21.0 + PAD]);
        assert_eq!((walk.fps, walk.loop_from, walk.repeats.len()), (Some(12.0), 1, 1));
        assert_eq!(r.animations["idle"].behind.len(), 1);
        assert!(!r.animations.contains_key("bored"));
    }

    #[test]
    fn each_drawn_pose_is_the_one_the_game_asks_for() {
        for c in Character::ALL {
            for (name, n) in drawn_animations(c) {
                for k in 0..n {
                    let (s, _) = posed(c, name, k, n, vec2(100.0, 100.0));
                    let first = s.poses()[0];
                    // The hunter's run and laugh are his own (the player's run and stand).
                    let wanted = match name {
                        "chase" => "run",
                        "laugh" => "idle",
                        n => n,
                    };
                    assert_eq!(first, wanted, "{c:?} {name} {k}");
                }
            }
        }
    }

    #[test]
    fn palettes_list_the_colours_used_most() {
        let mut img = image::RgbaImage::from_pixel(4, 2, image::Rgba([10, 20, 30, 255]));
        img.put_pixel(0, 0, image::Rgba([200, 0, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([0, 0, 0, 0]));
        let all = Rect::new(0.0, 0.0, 4.0, 2.0);
        // Near shades count as one colour.
        img.put_pixel(2, 1, image::Rgba([12, 22, 27, 255]));
        assert_eq!(palette(&img, [all, all].into_iter(), 8), [[8, 24, 24], [200, 8, 8]]);
        assert_eq!(palette(&img, [all].into_iter(), 1), [[8, 24, 24]]);
    }
}
