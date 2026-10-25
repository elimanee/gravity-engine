//! The characters drawn from sprite sheets the user provides (none ships
//! with the program), in `~/.config/gravity_engine/sonic/`:
//!
//! - Sonic: a PNG cut up by a `sheet.json` beside it, or by the built-in
//!   layout of Triangly's Sonic 1 sheet (The Spriters Resource);
//! - `tails.png`, `knuckles.png`, `amy.png` / `amy.webp`: the Sonic 3
//!   Tails and Sonic 3 & Knuckles Knuckles sheets (The Spriters Resource),
//!   and CartoonsAnimate22's Amy sheet (its Sonic Mania style) or
//!   DeltaConduit's (in the style of Sonic 1), each recognised by its size;
//! - `exe.png`: a Sonic.exe sheet in the style of Sonic 1 (691 × 677), with
//!   his teleport, and the run and laugh of the one that hunts you in the
//!   Halloween act.
//!
//! A picture is drawn with the character's centre at the point the layout
//! gives (for the built-in sheets, worked out from the pixels: the feet,
//! or the middle of a ball), facing right, one sheet pixel per guide
//! pixel (or `scale`).
//!
//! The game's objects can have sheets in the same folder too (see
//! [`crate::game::sprites`]); those names are not Sonic's.

use super::regions::Regions;
use super::{Character, Sonic};
use macroquad::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Where the sheets live.
pub fn dir() -> Option<PathBuf> {
    crate::config::config_dir().map(|c| c.join("sonic"))
}

/// How to cut Sonic's sheet up (`sheet.json`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(default)]
pub struct Layout {
    /// Width and height of a cell (px).
    pub cell: [u32; 2],
    /// Background colours made transparent.
    pub transparent: Vec<[u8; 3]>,
    /// Sheet pixels per guide pixel.
    pub scale: f32,
    /// Top-left corners of each animation's cells, in order.
    pub animations: HashMap<String, Vec<[u32; 2]>>,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { cell: [64, 64], transparent: vec![], scale: 1.0, animations: HashMap::new() }
    }
}

/// Triangly's Sonic 1 sheet (690 × 1558): 66 px cells on dark green, the
/// in-game tile space in a lighter green.
fn sonic1() -> Layout {
    let row = |y: u32, xs: &[u32]| xs.iter().map(|&x| [x, y]).collect::<Vec<_>>();
    let mut a = HashMap::new();
    a.insert("idle".into(), row(243, &[24]));
    a.insert("bored".into(), row(243, &[110, 180, 250, 320]));
    a.insert("look_up".into(), row(243, &[406]));
    a.insert("crouch".into(), row(243, &[492]));
    a.insert("spring".into(), row(243, &[578]));
    a.insert("walk".into(), row(334, &[24, 94, 164, 234, 304, 374]));
    a.insert("skid".into(), row(334, &[460, 530]));
    a.insert("balance".into(), row(425, &[460, 530]));
    a.insert("run".into(), row(516, &[24, 94, 164, 234]));
    a.insert("roll".into(), row(607, &[24, 94, 164, 234, 304]));
    a.insert("push".into(), row(607, &[390, 460, 530, 600]));
    a.insert("hurt".into(), row(789, &[24, 94]));
    Layout { cell: [66, 66], transparent: vec![[13, 72, 7], [37, 102, 26], [67, 153, 49]], scale: 1.0, animations: a }
}

/// Where a character's centre sits in a built-in picture.
#[derive(Clone, Copy, PartialEq)]
enum Fit {
    /// Standing: its feet are the bottom of the picture.
    Feet,
    /// Curled up, flying, gliding: the middle of the picture.
    Middle,
}

/// An animation of a built-in sheet: its pictures (x, y, w, h), and
/// pictures drawn behind them (Tails' tails).
struct Spec {
    name: &'static str,
    fit: Fit,
    pics: &'static [[u16; 4]],
    behind: &'static [[u16; 4]],
    /// Where the centre is in each picture (x, y from its top-left), when
    /// the pixels would mislead (a hammer swung beside her).
    at: &'static [[u16; 2]],
}

const fn spec(name: &'static str, fit: Fit, pics: &'static [[u16; 4]]) -> Spec {
    Spec { name, fit, pics, behind: &[], at: &[] }
}

/// Pictures with their centres given.
const fn placed(name: &'static str, pics: &'static [[u16; 4]], at: &'static [[u16; 2]]) -> Spec {
    Spec { name, fit: Fit::Middle, pics, behind: &[], at }
}

/// A built-in sheet: its size, background colours, whether its pictures
/// sit in boxes laid out as in the game (else each is cut tight around
/// the character), and its animations.
struct Builtin {
    character: Character,
    size: (u32, u32),
    background: &'static [[u8; 3]],
    /// Lossy (WebP): backgrounds are cut out by closeness, not exactly.
    lossy: bool,
    boxed: bool,
    specs: &'static [Spec],
}

const fn b48(x: u16, y: u16) -> [u16; 4] {
    [x, y, 48, 48]
}

// Sonic 3 Tails (1174 × 1820): the body and the tails are separate pieces
// for standing, rolling and the spin dash.
const TAILS_WAG: &[[u16; 4]] =
    &[[24, 264, 50, 48], [78, 264, 50, 48], [132, 264, 50, 48], [186, 264, 50, 48], [240, 264, 50, 48]];
const TAILS_ROLL: &[[u16; 4]] = &[[314, 264, 60, 48], [378, 264, 60, 48], [442, 264, 60, 48], [506, 264, 60, 48]];
const TAILS_DASH: &[[u16; 4]] = &[[590, 264, 52, 48], [646, 264, 52, 48], [702, 264, 52, 48], [758, 264, 52, 48]];
const TAILS_ROTOR: &[[u16; 4]] = &[[1044, 264, 51, 48], [1099, 264, 51, 48]];
const TAILS: Builtin = Builtin {
    character: Character::Tails,
    size: (1174, 1820),
    background: &[[37, 102, 26], [13, 72, 7], [34, 177, 76], [168, 230, 29]],
    lossy: false,
    boxed: true,
    specs: &[
        Spec { name: "idle", fit: Fit::Feet, pics: &[b48(24, 335)], behind: TAILS_WAG, at: &[] },
        Spec {
            name: "bored",
            fit: Fit::Feet,
            pics: &[b48(24, 408), b48(76, 408), b48(128, 408), b48(180, 408), b48(232, 408)],
            behind: TAILS_WAG,
            at: &[],
        },
        Spec { name: "look_up", fit: Fit::Feet, pics: &[b48(24, 481)], behind: TAILS_WAG, at: &[] },
        Spec { name: "crouch", fit: Fit::Feet, pics: &[b48(76, 481)], behind: TAILS_WAG, at: &[] },
        spec(
            "walk",
            Fit::Feet,
            &[
                [24, 647, 54, 48],
                [82, 647, 54, 48],
                [140, 647, 54, 48],
                [198, 647, 54, 48],
                [256, 647, 54, 48],
                [314, 647, 54, 48],
                [372, 647, 54, 48],
                [430, 647, 54, 48],
            ],
        ),
        spec("run", Fit::Feet, &[b48(504, 647), b48(556, 647), b48(608, 647), b48(660, 647)]),
        Spec {
            name: "roll",
            fit: Fit::Middle,
            pics: &[b48(314, 401), b48(366, 401), b48(418, 401)],
            behind: TAILS_ROLL,
            at: &[],
        },
        Spec {
            name: "spindash",
            fit: Fit::Feet,
            pics: &[b48(590, 337), b48(642, 337), b48(694, 337)],
            behind: TAILS_DASH,
            at: &[],
        },
        Spec { name: "skid", fit: Fit::Feet, pics: &[b48(590, 466), b48(642, 466)], behind: TAILS_DASH, at: &[] },
        Spec {
            name: "push",
            fit: Fit::Feet,
            pics: &[b48(590, 539), b48(642, 539), b48(694, 539), b48(746, 539)],
            behind: TAILS_DASH,
            at: &[],
        },
        spec("spring", Fit::Middle, &[[808, 720, 48, 57], [860, 720, 48, 57]]),
        spec("hurt", Fit::Middle, &[b48(848, 647)]),
        spec("dead", Fit::Middle, &[b48(968, 647)]),
        spec("balance", Fit::Feet, &[b48(1036, 647), b48(1088, 647)]),
        Spec { name: "fly", fit: Fit::Middle, pics: &[b48(1044, 337)], behind: TAILS_ROTOR, at: &[] },
        spec("fly_tired", Fit::Middle, &[b48(1044, 410), b48(1096, 410)]),
    ],
};

// Sonic 3 & Knuckles Knuckles (1131 × 1862), 48 px boxes.
const KNUCKLES: Builtin = Builtin {
    character: Character::Knuckles,
    size: (1131, 1862),
    background: &[[65, 169, 184], [16, 112, 132], [153, 217, 234]],
    lossy: false,
    boxed: true,
    specs: &[
        spec("idle", Fit::Feet, &[b48(24, 277)]),
        spec("bored", Fit::Feet, &[b48(76, 277), b48(128, 277)]),
        spec("look_up", Fit::Feet, &[b48(725, 277)]),
        spec("crouch", Fit::Feet, &[b48(845, 277)]),
        spec(
            "walk",
            Fit::Feet,
            &[
                b48(664, 350),
                b48(716, 350),
                b48(768, 350),
                b48(820, 350),
                b48(872, 350),
                b48(924, 350),
                b48(976, 350),
                b48(1028, 350),
            ],
        ),
        spec("run", Fit::Feet, &[b48(24, 423), b48(76, 423), b48(128, 423), b48(180, 423)]),
        spec(
            "roll",
            Fit::Middle,
            &[
                b48(248, 423),
                b48(456, 423),
                b48(300, 423),
                b48(456, 423),
                b48(352, 423),
                b48(456, 423),
                b48(404, 423),
                b48(456, 423),
            ],
        ),
        spec(
            "spindash",
            Fit::Feet,
            &[
                b48(24, 508),
                b48(284, 508),
                b48(76, 508),
                b48(284, 508),
                b48(128, 508),
                b48(284, 508),
                b48(180, 508),
                b48(284, 508),
            ],
        ),
        spec("skid", Fit::Feet, &[b48(412, 633), b48(464, 633)]),
        spec("push", Fit::Feet, &[b48(736, 779), b48(788, 779), b48(840, 779), b48(892, 779)]),
        spec("spring", Fit::Middle, &[b48(636, 633)]),
        spec("hurt", Fit::Middle, &[b48(960, 556)]),
        spec("dead", Fit::Middle, &[b48(728, 884)]),
        spec("balance", Fit::Feet, &[b48(740, 524), b48(792, 524), b48(845, 524)]),
        spec("glide", Fit::Middle, &[[24, 1009, 48, 34]]),
        spec("drop", Fit::Middle, &[b48(128, 1009)]),
        spec("slide", Fit::Feet, &[[184, 1009, 48, 34]]),
        spec(
            "climb",
            Fit::Middle,
            &[b48(24, 1085), b48(76, 1085), b48(128, 1085), b48(180, 1085), b48(232, 1085), b48(284, 1085)],
        ),
        spec("climb_up", Fit::Middle, &[b48(24, 1137), b48(76, 1137), b48(128, 1137)]),
    ],
};

// CartoonsAnimate22's Amy (1274 × 2000, WebP), its Sonic Mania style:
// pictures cut tight around her.
const AMY: Builtin = Builtin {
    character: Character::Amy,
    size: (1274, 2000),
    // The boxes, the page, and the blur between them.
    background: &[[0, 190, 220], [0, 97, 128], [0, 145, 176]],
    lossy: true,
    boxed: false,
    specs: &[
        spec("idle", Fit::Feet, &[[545, 670, 19, 32]]),
        spec("bored", Fit::Feet, &[[568, 671, 20, 31], [591, 671, 20, 31], [614, 671, 21, 31], [637, 671, 22, 31]]),
        spec("look_up", Fit::Feet, &[[548, 1221, 19, 36]]),
        spec("crouch", Fit::Feet, &[[1016, 678, 22, 24]]),
        spec(
            "walk",
            Fit::Feet,
            &[
                [547, 708, 21, 31],
                [576, 707, 24, 32],
                [606, 706, 26, 32],
                [636, 706, 26, 30],
                [668, 708, 24, 30],
                [700, 708, 21, 31],
                [731, 708, 21, 31],
                [757, 707, 25, 32],
                [787, 706, 28, 31],
                [818, 706, 28, 31],
                [851, 707, 23, 31],
                [883, 708, 20, 31],
            ],
        ),
        spec(
            "run",
            Fit::Feet,
            &[
                [547, 789, 25, 31],
                [578, 789, 24, 32],
                [610, 791, 22, 30],
                [639, 790, 24, 31],
                [668, 789, 25, 32],
                [698, 789, 25, 31],
                [730, 789, 24, 32],
                [762, 791, 22, 30],
                [790, 790, 25, 31],
                [821, 789, 24, 32],
            ],
        ),
        spec(
            "roll",
            Fit::Middle,
            &[
                [608, 908, 24, 24],
                [639, 908, 23, 24],
                [669, 909, 24, 24],
                [700, 909, 23, 23],
                [730, 908, 22, 25],
                [759, 908, 23, 25],
                [789, 908, 25, 22],
                [820, 908, 24, 23],
            ],
        ),
        spec(
            "spindash",
            Fit::Middle,
            &[
                [909, 871, 25, 22],
                [939, 871, 24, 22],
                [970, 871, 24, 22],
                [1000, 871, 24, 22],
                [1030, 871, 24, 22],
                [1060, 871, 25, 23],
                [1091, 871, 24, 22],
                [1121, 871, 24, 22],
                [1152, 871, 24, 23],
            ],
        ),
        spec("push", Fit::Feet, &[[548, 1153, 23, 29], [579, 1153, 24, 29], [671, 1153, 22, 29], [702, 1153, 21, 29]]),
        spec(
            "spring",
            Fit::Middle,
            &[
                [548, 1186, 28, 33],
                [584, 1186, 29, 33],
                [624, 1187, 26, 31],
                [662, 1186, 26, 33],
                [700, 1187, 26, 32],
                [737, 1186, 26, 32],
            ],
        ),
        spec("hurt", Fit::Middle, &[[942, 1154, 30, 26]]),
        spec("dead", Fit::Middle, &[[642, 1263, 21, 38]]),
        spec(
            "hammer",
            Fit::Feet,
            &[[550, 1031, 28, 33], [582, 1026, 32, 36], [619, 1030, 37, 30], [658, 1029, 34, 30], [700, 1026, 28, 36]],
        ),
        spec(
            "hammer_air",
            Fit::Middle,
            &[
                [851, 908, 34, 24],
                [940, 901, 24, 34],
                [968, 898, 43, 45],
                [1019, 908, 33, 24],
                [1056, 899, 44, 43],
                [1104, 905, 24, 33],
                [1131, 898, 44, 44],
            ],
        ),
    ],
};

// DeltaConduit's Amy in the style of Sonic 1 (583 × 570): each picture in
// its own box cut tight around her.
const AMY_S1: Builtin = Builtin {
    character: Character::Amy,
    size: (583, 570),
    // The page, the boxes and the lines between the spinning pictures.
    background: &[[108, 180, 252], [36, 108, 180], [72, 144, 216]],
    lossy: false,
    boxed: false,
    specs: &[
        spec("idle", Fit::Feet, &[[35, 63, 23, 36]]),
        spec("bored", Fit::Feet, &[[59, 63, 23, 36], [83, 63, 23, 36], [107, 63, 22, 36], [83, 63, 23, 36]]),
        spec("look_up", Fit::Feet, &[[139, 63, 24, 36]]),
        spec("crouch", Fit::Feet, &[[173, 75, 28, 24]]),
        spec(
            "walk",
            Fit::Feet,
            &[
                [211, 64, 22, 35],
                [234, 62, 30, 35],
                [265, 63, 27, 35],
                [293, 64, 21, 35],
                [315, 62, 33, 34],
                [349, 63, 28, 34],
            ],
        ),
        spec("run", Fit::Feet, &[[2, 130, 31, 33], [34, 130, 27, 33], [62, 130, 30, 33], [93, 130, 27, 33]]),
        spec("skid", Fit::Feet, &[[261, 129, 35, 34], [297, 129, 34, 34]]),
        spec(
            "spindash",
            Fit::Middle,
            &[[341, 136, 32, 27], [374, 136, 31, 27], [406, 136, 31, 27], [438, 136, 31, 27], [470, 136, 31, 27]],
        ),
        // The ball, between the turning pictures.
        spec(
            "roll",
            Fit::Middle,
            &[
                [32, 202, 29, 29],
                [2, 202, 29, 29],
                [62, 202, 29, 29],
                [2, 202, 29, 29],
                [92, 202, 29, 29],
                [2, 202, 29, 29],
                [122, 202, 29, 29],
                [2, 202, 29, 29],
            ],
        ),
        spec("push", Fit::Feet, &[[2, 256, 24, 32], [27, 255, 18, 33], [46, 256, 22, 32], [69, 255, 18, 33]]),
        spec("spring", Fit::Middle, &[[242, 247, 20, 41]]),
        spec("hurt", Fit::Middle, &[[2, 321, 35, 30]]),
        spec("dead", Fit::Middle, &[[86, 312, 32, 39]]),
        placed(
            "hammer",
            &[[161, 198, 43, 33], [205, 190, 63, 41], [269, 198, 43, 33]],
            &[[21, 17], [33, 23], [15, 17]],
        ),
        placed(
            "hammer_air",
            &[[322, 198, 42, 33], [365, 196, 22, 35], [388, 199, 42, 31], [365, 196, 22, 35]],
            &[[13, 16], [11, 17], [17, 15], [11, 17]],
        ),
    ],
};

// Sonic.exe in the style of Sonic 1 (691 × 677): pictures loose on a
// dark red page, cut tight around him.
const EXE: Builtin = Builtin {
    character: Character::Exe,
    size: (691, 677),
    background: &[[147, 70, 70]],
    lossy: false,
    boxed: false,
    specs: &[
        spec("idle", Fit::Feet, &[[113, 12, 33, 41]]),
        spec(
            "bored",
            Fit::Feet,
            &[[72, 423, 29, 39], [103, 423, 29, 39], [134, 423, 29, 39], [165, 423, 29, 39], [196, 423, 29, 39]],
        ),
        spec("look_up", Fit::Feet, &[[40, 418, 27, 44]]),
        spec("crouch", Fit::Feet, &[[307, 632, 32, 25]]),
        spec(
            "walk",
            Fit::Feet,
            &[
                [12, 368, 27, 41],
                [44, 369, 27, 40],
                [82, 369, 32, 40],
                [122, 369, 27, 40],
                [154, 368, 27, 41],
                [187, 369, 28, 40],
                [224, 370, 32, 40],
                [260, 370, 28, 40],
            ],
        ),
        spec(
            "run",
            Fit::Feet,
            &[[377, 313, 31, 39], [428, 313, 31, 39], [479, 313, 31, 39], [530, 313, 31, 39], [581, 313, 31, 39]],
        ),
        spec("skid", Fit::Feet, &[[294, 375, 37, 35]]),
        spec("push", Fit::Feet, &[[257, 476, 31, 39], [308, 476, 31, 39]]),
        // The spinning pictures between the plain ball.
        spec(
            "roll",
            Fit::Middle,
            &[
                [287, 220, 30, 30],
                [217, 220, 30, 30],
                [322, 220, 30, 30],
                [217, 220, 30, 30],
                [357, 220, 30, 30],
                [217, 220, 30, 30],
            ],
        ),
        spec(
            "spindash",
            Fit::Middle,
            &[
                [7, 223, 30, 27],
                [42, 223, 30, 27],
                [77, 223, 30, 27],
                [112, 223, 30, 27],
                [147, 223, 30, 27],
                [182, 223, 30, 27],
            ],
        ),
        spec("spring", Fit::Middle, &[[181, 255, 36, 47]]),
        spec("hurt", Fit::Middle, &[[10, 480, 38, 32], [53, 481, 39, 31]]),
        spec("dead", Fit::Middle, &[[97, 472, 33, 40]]),
        spec("warp", Fit::Middle, &[[317, 431, 44, 26], [365, 433, 43, 26]]),
        // The one who hunts you: arms out as he runs, and his laugh.
        spec("chase", Fit::Feet, &[[296, 266, 39, 43], [347, 265, 39, 43], [400, 265, 36, 43], [443, 265, 38, 42]]),
        spec("laugh", Fit::Feet, &[[489, 263, 44, 44], [539, 264, 44, 43]]),
    ],
};

const BUILTINS: [&Builtin; 5] = [&TAILS, &KNUCKLES, &AMY, &AMY_S1, &EXE];

/// The file a character's sheet is kept in (Sonic: any other PNG).
fn file_names(c: Character) -> &'static [&'static str] {
    match c {
        Character::Sonic => &["sheet.png"],
        Character::Tails => &["tails.png"],
        Character::Knuckles => &["knuckles.png"],
        Character::Amy => &["amy.png", "amy.webp"],
        Character::Exe => &["exe.png"],
    }
}

/// Whether `c` has a sheet in the Sonic folder (usable or not).
pub fn has_sheet(c: Character) -> bool {
    let Some(dir) = dir().filter(|d| d.is_dir()) else { return false };
    match c {
        Character::Sonic => find(&dir).is_ok(),
        c => file_names(c).iter().any(|n| dir.join(n).is_file()),
    }
}

/// Which character's built-in sheet an image of this size is.
pub fn character_of(size: (u32, u32)) -> Option<Character> {
    BUILTINS.iter().find(|b| b.size == size).map(|b| b.character)
}

/// A picture: where it is on the sheet, where the character's centre is
/// in it (sheet px from its top-left), whether it is drawn mirrored, and
/// how long it shows (in pictures: 2 is twice as long as the others).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pic {
    pub src: Rect,
    pub centre: Vec2,
    pub flip: bool,
    pub hold: f32,
}

impl Pic {
    pub fn new(src: Rect, centre: Vec2) -> Pic {
        Pic { src, centre, flip: false, hold: 1.0 }
    }
}

/// An animation's pictures, the pieces drawn behind them (Tails' tails, on
/// their own clock), its pace in pictures a second (none: the games' own,
/// which follows the character's speed) and whether it plays once,
/// stopping on its last picture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Anim {
    pub pics: Vec<Pic>,
    pub behind: Vec<Pic>,
    pub fps: Option<f32>,
    pub once: bool,
}

impl Anim {
    /// How long its pictures show together (in pictures).
    pub fn length(&self) -> f32 {
        self.pics.iter().map(|p| p.hold.max(0.01)).sum()
    }

    /// The picture showing `t` pictures in: looping, or stopping on the
    /// last when played once.
    pub fn at(&self, t: f32, once: bool) -> usize {
        let length = self.length();
        let t = if once { t.clamp(0.0, length) } else { t.rem_euclid(length.max(0.01)) };
        let mut end = 0.0;
        for (k, p) in self.pics.iter().enumerate() {
            end += p.hold.max(0.01);
            if t < end {
                return k;
            }
        }
        self.pics.len().saturating_sub(1)
    }
}

/// A loaded sheet.
pub struct Sheet {
    texture: Texture2D,
    anims: HashMap<String, Anim>,
    scale: f32,
    pub name: String,
}

/// The sheets of all the characters (each one optional).
#[derive(Default)]
pub struct Sheets {
    sheets: Vec<(Character, Sheet)>,
}

impl Sheets {
    pub fn get(&self, c: Character) -> Option<&Sheet> {
        self.sheets.iter().find(|(k, _)| *k == c).map(|(_, s)| s)
    }

    /// Draw `c` from `sheet` (one being edited, to try it).
    pub fn set(&mut self, c: Character, sheet: Sheet) {
        self.sheets.retain(|(k, _)| *k != c);
        self.sheets.push((c, sheet));
    }

    /// Load every character's sheet in the Sonic folder; problems are
    /// returned beside what did load.
    pub fn load() -> (Sheets, Vec<String>) {
        let mut out = Sheets::default();
        let mut errors = vec![];
        for c in Character::ALL {
            match Sheet::load(c) {
                Ok(Some(s)) => out.sheets.push((c, s)),
                Ok(None) => {}
                Err(e) => errors.push(format!("{}: {e}", c.label())),
            }
        }
        (out, errors)
    }
}

/// Is this file one of the other sheets rather than Sonic's (or the
/// regions edited on a sheet)?
fn is_not_sonics(p: &Path) -> bool {
    use crate::game::sprites::{MONITORS, OBJECTS};
    let others = Character::ALL.iter().skip(1).flat_map(|&c| file_names(c).iter().copied());
    p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
        n == OBJECTS || n == MONITORS || n.ends_with(".regions.json") || others.clone().any(|o| o == n)
    })
}

/// Sonic's PNG in `dir` (`sheet.png` first), and its layout.
fn find(dir: &Path) -> Result<(PathBuf, Option<Layout>), String> {
    let mut pngs: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) && !is_not_sonics(p))
        .collect();
    pngs.sort_by_key(|p| (p.file_name().is_none_or(|n| n != "sheet.png"), p.clone()));
    let png = pngs.into_iter().next().ok_or("no PNG sprite sheet")?;
    let json = dir.join("sheet.json");
    let layout = match std::fs::read_to_string(&json) {
        Ok(text) => Some(serde_json::from_str::<Layout>(&text).map_err(|e| format!("sheet.json: {e}"))?),
        Err(_) => None,
    };
    Ok((png, layout))
}

/// Cut out the background colours.
pub fn clear_background(img: &mut image::RgbaImage, colours: &[[u8; 3]]) {
    for p in img.pixels_mut() {
        if colours.iter().any(|c| c[0] == p[0] && c[1] == p[1] && c[2] == p[2]) {
            p[3] = 0;
        }
    }
}

/// Cut out colours near the background ones, softly (for lossy images):
/// edge pixels keep their own colour without the background's tint.
pub fn clear_background_near(img: &mut image::RgbaImage, colours: &[[u8; 3]]) {
    const CLEAR: f32 = 40.0;
    const SOLID: f32 = 80.0;
    for p in img.pixels_mut() {
        let dist = |c: &[u8; 3]| (0..3).map(|k| (p[k] as f32 - c[k] as f32).powi(2)).sum::<f32>().sqrt();
        let Some(near) = colours.iter().min_by(|a, b| dist(a).total_cmp(&dist(b))) else { return };
        let a = ((dist(near) - CLEAR) / (SOLID - CLEAR)).clamp(0.0, 1.0);
        if a > 0.0 && a < 1.0 {
            for k in 0..3 {
                p[k] = ((p[k] as f32 - (1.0 - a) * near[k] as f32) / a).clamp(0.0, 255.0) as u8;
            }
        }
        p[3] = (p[3] as f32 * a) as u8;
    }
}

/// The opaque part of `r` in `img`: its bounding box (x0, y0, x1, y1) and
/// the middle of its upper part (the head and body, steadier than the
/// limbs).
fn measure(img: &image::RgbaImage, r: [u16; 4]) -> Option<([f32; 4], f32)> {
    let [x, y, w, h] = r.map(u32::from);
    let opaque = |px: u32, py: u32| px < img.width() && py < img.height() && img.get_pixel(px, py)[3] > 96;
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for py in y..y + h {
        for px in x..x + w {
            if opaque(px, py) {
                (x0, y0, x1, y1) = (x0.min(px), y0.min(py), x1.max(px + 1), y1.max(py + 1));
            }
        }
    }
    if x0 == u32::MAX {
        return None;
    }
    let top = y0 + (y1 - y0) * 45 / 100;
    let (mut sum, mut n) = (0.0, 0.0);
    for py in y0..top.max(y0 + 1) {
        for px in x0..x1 {
            if opaque(px, py) {
                sum += px as f32 + 0.5;
                n += 1.0;
            }
        }
    }
    let head = if n > 0.0 { sum / n } else { (x0 + x1) as f32 / 2.0 };
    Some(([x0 as f32, y0 as f32, x1 as f32, y1 as f32], head))
}

fn median(mut v: Vec<f32>) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    v[v.len() / 2]
}

/// The pictures of a built-in animation, with the character's centre in
/// each. Boxed sheets keep the game's placement (one offset for the whole
/// animation); tight pictures each get their own.
fn cut(img: &image::RgbaImage, b: &Builtin, s: &Spec) -> Anim {
    let hr = b.character.stand().1;
    let measured: Vec<_> = s.pics.iter().map(|&r| (r, measure(img, r))).collect();
    let centre_of = |(bx, head): ([f32; 4], f32)| -> Vec2 {
        match s.fit {
            Fit::Feet => vec2(head, bx[3] - hr),
            Fit::Middle => vec2((bx[0] + bx[2]) / 2.0, (bx[1] + bx[3]) / 2.0),
        }
    };
    let rect = |r: [u16; 4]| Rect::new(r[0] as f32, r[1] as f32, r[2] as f32, r[3] as f32);
    let (dx, dy) = {
        let offs: Vec<Vec2> =
            measured.iter().filter_map(|(r, m)| m.map(|m| centre_of(m) - vec2(r[0] as f32, r[1] as f32))).collect();
        (median(offs.iter().map(|o| o.x).collect()), median(offs.iter().map(|o| o.y).collect()))
    };
    let pics = measured
        .iter()
        .enumerate()
        .map(|(k, &(r, m))| {
            let centre = match (s.at.get(k), m) {
                (Some(&[x, y]), _) => vec2(x as f32, y as f32),
                (None, Some(m)) if !b.boxed => centre_of(m) - vec2(r[0] as f32, r[1] as f32),
                _ => vec2(dx, dy),
            };
            Pic::new(rect(r), centre)
        })
        .collect();
    // Pieces behind are boxed like the body: same place in their box.
    let behind = s.behind.iter().map(|&r| Pic::new(rect(r), vec2(dx, dy))).collect();
    Anim { pics, behind, ..Default::default() }
}

/// Neighbouring pixels, corners included.
const AROUND: [(i64, i64); 8] = [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)];

fn opaque_at(img: &image::RgbaImage, x: u32, y: u32) -> bool {
    img.as_raw()[(y as usize * img.width() as usize + x as usize) * 4 + 3] > 96
}

/// Every sprite on a sheet with its background cleared: the boxes around
/// its groups of joined opaque pixels (pieces a pixel apart are one), in
/// reading order, leaving out specks.
pub fn detect_sprites(img: &image::RgbaImage) -> Vec<Rect> {
    let (w, h) = img.dimensions();
    let mut seen = vec![false; w as usize * h as usize];
    let mut boxes: Vec<[u32; 4]> = vec![];
    let mut todo = vec![];
    for y in 0..h {
        for x in 0..w {
            let i = y as usize * w as usize + x as usize;
            if seen[i] || !opaque_at(img, x, y) {
                continue;
            }
            seen[i] = true;
            todo.push((x, y));
            let mut b = [x, y, x + 1, y + 1];
            while let Some((x, y)) = todo.pop() {
                b = [b[0].min(x), b[1].min(y), b[2].max(x + 1), b[3].max(y + 1)];
                for (dx, dy) in AROUND {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    let (nx, ny) = (nx as u32, ny as u32);
                    let j = ny as usize * w as usize + nx as usize;
                    if !seen[j] && opaque_at(img, nx, ny) {
                        seen[j] = true;
                        todo.push((nx, ny));
                    }
                }
            }
            boxes.push(b);
        }
    }
    // Boxes touching (or a pixel apart) are one sprite.
    loop {
        let mut joined = false;
        let mut k = 0;
        while k < boxes.len() {
            let mut j = k + 1;
            while j < boxes.len() {
                let (a, b) = (boxes[k], boxes[j]);
                if a[0] <= b[2] + 1 && b[0] <= a[2] + 1 && a[1] <= b[3] + 1 && b[1] <= a[3] + 1 {
                    boxes[k] = [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])];
                    boxes.swap_remove(j);
                    joined = true;
                } else {
                    j += 1;
                }
            }
            k += 1;
        }
        if !joined {
            break;
        }
    }
    boxes.retain(|b| b[2] - b[0] >= 4 && b[3] - b[1] >= 4 && (b[2] - b[0]) * (b[3] - b[1]) >= 48);
    boxes.sort_by_key(|b| (b[1], b[0]));
    boxes.into_iter().map(|b| Rect::new(b[0] as f32, b[1] as f32, (b[2] - b[0]) as f32, (b[3] - b[1]) as f32)).collect()
}

/// The sprite at `r` on a sheet with its background cleared: the opaque
/// pixels inside `r` and those joined to them up to `grow` px around it.
/// Returns the picture tight around it, centred as a standing character
/// (`feet`: its half height above the bottom, under its head) or a curled
/// up one (the middle).
pub fn find_sprite(img: &image::RgbaImage, r: Rect, grow: f32, feet: Option<f32>) -> Option<Pic> {
    let (w, h) = img.dimensions();
    let span = |a: f32, b: f32, max: u32| ((a.max(0.0) as u32).min(max), (b.max(0.0).ceil() as u32).min(max));
    let (ax0, ax1) = span(r.x - grow, r.x + r.w + grow, w);
    let (ay0, ay1) = span(r.y - grow, r.y + r.h + grow, h);
    let (sx0, sx1) = span(r.x, r.x + r.w, w);
    let (sy0, sy1) = span(r.y, r.y + r.h, h);
    let opaque = |x: u32, y: u32| opaque_at(img, x, y);
    let aw = (ax1 - ax0) as usize;
    let mut seen = vec![false; aw * (ay1 - ay0) as usize];
    let at = |x: u32, y: u32| (y - ay0) as usize * aw + (x - ax0) as usize;
    let mut todo = vec![];
    for y in sy0..sy1 {
        for x in (sx0..sx1).filter(|&x| opaque(x, y)) {
            seen[at(x, y)] = true;
            todo.push((x, y));
        }
    }
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    while let Some((x, y)) = todo.pop() {
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
        for (dx, dy) in AROUND {
            let (nx, ny) = (x as i64 + dx, y as i64 + dy);
            if nx < ax0 as i64 || ny < ay0 as i64 || nx >= ax1 as i64 || ny >= ay1 as i64 {
                continue;
            }
            let (nx, ny) = (nx as u32, ny as u32);
            if !seen[at(nx, ny)] && opaque(nx, ny) {
                seen[at(nx, ny)] = true;
                todo.push((nx, ny));
            }
        }
    }
    if x0 == u32::MAX {
        return None;
    }
    let (bw, bh) = (x1 - x0, y1 - y0);
    let (_, head) = measure(img, [x0, y0, bw, bh].map(|v| v.min(u16::MAX as u32) as u16))?;
    let centre = match feet {
        Some(hr) => vec2(head - x0 as f32, bh as f32 - hr),
        None => vec2(bw as f32, bh as f32) / 2.0,
    };
    Some(Pic::new(Rect::new(x0 as f32, y0 as f32, bw as f32, bh as f32), centre))
}

/// Cut out `colours`: exactly, or by closeness (`soft`, for lossy images).
fn clear(img: &mut image::RgbaImage, colours: &[[u8; 3]], soft: bool) {
    if colours.is_empty() {
    } else if soft {
        clear_background_near(img, colours);
    } else {
        clear_background(img, colours);
    }
}

/// A character's sheet ready to draw from, as its layout cuts it.
struct Prepared {
    /// The sheet as it is, and with the layout's background cleared.
    original: image::RgbaImage,
    image: image::RgbaImage,
    /// The layout's background colours, and whether they go by closeness.
    background: Vec<[u8; 3]>,
    soft: bool,
    scale: f32,
    name: String,
    anims: HashMap<String, Anim>,
    /// The game knows how to cut it up (else only regions edited on it can).
    known: bool,
}

/// `c`'s sheet in the Sonic folder, if there is one, cut by its layout.
fn prepare(c: Character) -> Result<Option<Prepared>, String> {
    let Some(dir) = dir().filter(|d| d.is_dir()) else { return Ok(None) };
    if c == Character::Sonic {
        return prepare_sonic(&dir);
    }
    let Some(path) = file_names(c).iter().map(|n| dir.join(n)).find(|p| p.is_file()) else { return Ok(None) };
    let original = image::open(&path).map_err(|e| e.to_string())?.to_rgba8();
    let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut img = original.clone();
    // A character may have more than one known sheet: its size says which.
    let Some(b) = BUILTINS.iter().find(|b| b.character == c && b.size == img.dimensions()) else {
        let anims = HashMap::new();
        return Ok(Some(Prepared {
            original,
            image: img,
            background: vec![],
            soft: false,
            scale: 1.0,
            name,
            anims,
            known: false,
        }));
    };
    clear(&mut img, b.background, b.lossy);
    let anims = b.specs.iter().map(|s| (s.name.to_string(), cut(&img, b, s))).collect();
    let background = b.background.to_vec();
    Ok(Some(Prepared { original, image: img, background, soft: b.lossy, scale: 1.0, name, anims, known: true }))
}

fn prepare_sonic(dir: &Path) -> Result<Option<Prepared>, String> {
    let (png, layout) = match find(dir) {
        Ok(found) => found,
        Err(e) if e == "no PNG sprite sheet" => return Ok(None),
        Err(e) => return Err(e),
    };
    let original = image::open(&png).map_err(|e| e.to_string())?.to_rgba8();
    let name = png.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    let mut img = original.clone();
    let layout = match layout {
        Some(l) => l,
        None if img.dimensions() == (690, 1558) => sonic1(),
        None => {
            let anims = HashMap::new();
            return Ok(Some(Prepared {
                original,
                image: img,
                background: vec![],
                soft: false,
                scale: 1.0,
                name,
                anims,
                known: false,
            }));
        }
    };
    clear_background(&mut img, &layout.transparent);
    let [w, h] = layout.cell.map(|v| v as f32);
    let anims = layout
        .animations
        .iter()
        .map(|(name, cells)| {
            let pics =
                cells.iter().map(|&[x, y]| Pic::new(Rect::new(x as f32, y as f32, w, h), vec2(w, h) / 2.0)).collect();
            (name.clone(), Anim { pics, ..Default::default() })
        })
        .collect();
    let (background, scale) = (layout.transparent, layout.scale);
    Ok(Some(Prepared { original, image: img, background, soft: false, scale, name, anims, known: true }))
}

/// A character's sheet as the sprite region editor works on it.
pub struct Editable {
    /// The sheet as it is, and as drawn (its backgrounds cleared).
    pub original: image::RgbaImage,
    pub image: image::RgbaImage,
    pub texture: Texture2D,
    /// The layout's own background colours, their closeness and its scale.
    pub layout_background: Vec<[u8; 3]>,
    pub layout_soft: bool,
    pub layout_scale: f32,
    /// Background colours picked besides, cut out by closeness or not, and
    /// sheet pixels per guide pixel.
    pub background: Vec<[u8; 3]>,
    pub soft: bool,
    pub scale: f32,
    pub name: String,
    /// The game knows how to cut this sheet up (else it starts empty).
    pub known: bool,
    /// The layout's own animations.
    pub defaults: HashMap<String, Anim>,
    /// With the regions kept so far.
    pub anims: HashMap<String, Anim>,
    /// Why the regions kept could not be read (the editor starts over).
    pub warning: Option<String>,
}

impl Editable {
    /// `c`'s sheet, if there is one.
    pub fn open(c: Character) -> Result<Option<Editable>, String> {
        let Some(p) = prepare(c)? else { return Ok(None) };
        let mut ed = Editable {
            texture: upload(&p.image),
            image: p.image,
            original: p.original,
            layout_background: p.background,
            layout_soft: p.soft,
            layout_scale: p.scale,
            background: vec![],
            soft: false,
            scale: p.scale,
            name: p.name,
            known: p.known,
            anims: p.anims.clone(),
            defaults: p.anims,
            warning: None,
        };
        match Regions::read(c, ed.image.dimensions()) {
            Ok(Some(r)) => {
                r.apply(&mut ed.anims);
                ed.background = r.background;
                ed.soft = r.soft;
                ed.scale = r.scale.unwrap_or(ed.scale);
                if !ed.background.is_empty() || ed.soft {
                    ed.recolour();
                }
            }
            Ok(None) => {}
            Err(e) => ed.warning = Some(e),
        }
        Ok(Some(ed))
    }

    /// Clear the backgrounds again, after the colours changed.
    pub fn recolour(&mut self) {
        let mut img = self.original.clone();
        clear(&mut img, &self.layout_background, self.layout_soft);
        clear(&mut img, &self.background, self.soft || self.layout_soft);
        self.texture = upload(&img);
        self.image = img;
    }

    /// The regions to keep: what changed from the layout.
    pub fn regions(&self) -> Regions {
        let mut r = Regions::changes(self.image.dimensions(), &self.defaults, &self.anims);
        r.background = self.background.clone();
        r.soft = self.soft;
        r.scale = (self.scale != self.layout_scale).then_some(self.scale);
        r
    }

    /// The sheet as edited, to play with before it is saved.
    pub fn sheet(&self) -> Sheet {
        Sheet { texture: self.texture.clone(), anims: self.anims.clone(), scale: self.scale, name: self.name.clone() }
    }
}

impl Sheet {
    /// `c`'s sheet in the Sonic folder, if there is one, with the regions
    /// edited on it.
    pub fn load(c: Character) -> Result<Option<Sheet>, String> {
        let Some(mut p) = prepare(c)? else { return Ok(None) };
        let (w, h) = p.image.dimensions();
        let regions = Regions::read(c, (w, h))?;
        if !p.known && regions.is_none() {
            return Err(format!("the game does not know this sheet ({w} × {h}): lay it out with Edit sprite regions…"));
        }
        let mut scale = p.scale;
        if let Some(r) = &regions {
            clear(&mut p.image, &r.background, r.soft || p.soft);
            r.apply(&mut p.anims);
            scale = r.scale.unwrap_or(scale);
        }
        if p.anims.get("idle").is_none_or(|a| a.pics.is_empty()) {
            return Err("no \"idle\" picture: give it one with Edit sprite regions…".into());
        }
        Ok(Some(Sheet { texture: upload(&p.image), anims: p.anims, scale, name: p.name }))
    }

    /// Copy a chosen sheet into the Sonic folder: as the character or the
    /// objects' sheet its size says it is, else as Sonic's (with its
    /// sheet.json, if any). Returns the file it became.
    pub fn install(png: &Path) -> Result<String, String> {
        let dir = dir().ok_or("no config directory")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let size = image::image_dimensions(png).map_err(|e| e.to_string())?;
        if let Some(name) = crate::game::sprites::sheet_name(size) {
            std::fs::copy(png, dir.join(name)).map_err(|e| e.to_string())?;
            return Ok(name.to_string());
        }
        if let Some(c) = character_of(size) {
            for n in file_names(c) {
                let _ = std::fs::remove_file(dir.join(n));
            }
            let ext = png.extension().map_or("png".into(), |e| e.to_string_lossy().to_lowercase());
            let name = file_names(c).iter().find(|n| n.ends_with(&ext)).unwrap_or(&file_names(c)[0]).to_string();
            std::fs::copy(png, dir.join(&name)).map_err(|e| e.to_string())?;
            return Ok(name);
        }
        for old in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
            if is_not_sonics(&old.path()) {
                continue;
            }
            if old.path().extension().is_some_and(|e| e.eq_ignore_ascii_case("png") || e == "json") {
                let _ = std::fs::remove_file(old.path());
            }
        }
        std::fs::copy(png, dir.join("sheet.png")).map_err(|e| e.to_string())?;
        if let Some(json) = png.parent().map(|p| p.join("sheet.json")).filter(|j| j.is_file()) {
            std::fs::copy(json, dir.join("sheet.json")).map_err(|e| e.to_string())?;
        }
        Ok("sheet.png".into())
    }

    /// Make `src` (any picture file) `c`'s sheet, as a PNG under the
    /// character's name; the sheet it had goes (Sonic's layout too).
    pub fn install_for(src: &Path, c: Character) -> Result<String, String> {
        let dir = dir().ok_or("no config directory")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let img = image::open(src).map_err(|e| e.to_string())?;
        if c == Character::Sonic {
            for old in std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten() {
                let p = old.path();
                let theirs = p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png") || e == "json");
                if theirs && !is_not_sonics(&p) {
                    let _ = std::fs::remove_file(p);
                }
            }
        } else {
            for n in file_names(c) {
                let _ = std::fs::remove_file(dir.join(n));
            }
        }
        let name = file_names(c)[0];
        img.save(dir.join(name)).map_err(|e| e.to_string())?;
        Ok(name.to_string())
    }

    fn anim(&self, name: &str) -> Option<&Anim> {
        self.anims.get(name).filter(|a| !a.pics.is_empty())
    }

    /// The animation for his state, and which of its pictures.
    fn pick(&self, s: &Sonic) -> Option<(&Anim, usize)> {
        let name = s.poses().into_iter().chain(["walk", "idle"]).find(|n| self.anim(n).is_some())?;
        let anim = self.anim(name)?;
        let length = anim.length();
        let (t, once) = match (anim.fps, name) {
            (Some(fps), _) => (s.pose_frames as f32 / super::FPS * fps, anim.once),
            (None, "bored") => (s.idle.saturating_sub(180) as f32 / 30.0, anim.once),
            (None, "push") => (s.anim / 8.0, anim.once),
            // Played once through, with the move.
            (None, "hammer") => (s.hammer.map_or(0.0, |h| h as f32 / 22.0) * length, true),
            (None, "climb_up") => (s.climb_up.map_or(0.0, |(t, _, _)| t as f32 / 18.0) * length, true),
            (None, _) if anim.once => (s.anim - s.pose_anim, true),
            (None, _) => (s.anim, false),
        };
        Some((anim, anim.at(t, once)))
    }

    fn blit(&self, pic: Pic, s: &Sonic, angle: f32) {
        self.blit_at(pic, s.pos, s.facing, angle, WHITE);
    }

    fn blit_at(&self, pic: Pic, pos: Vec2, facing: f32, angle: f32, tint: Color) {
        let k = super::SCALE / self.scale.max(0.01);
        let size = vec2(pic.src.w, pic.src.h) * k;
        // A mirrored picture faces the other way.
        let flip = (facing < 0.0) != pic.flip;
        let cx = if flip { pic.src.w - pic.centre.x } else { pic.centre.x };
        let top_left = pos - vec2(cx, pic.centre.y) * k;
        draw_texture_ex(
            &self.texture,
            top_left.x,
            top_left.y,
            tint,
            DrawTextureParams {
                dest_size: Some(size),
                source: Some(pic.src),
                rotation: -angle,
                flip_x: flip,
                pivot: Some(pos),
                ..Default::default()
            },
        );
    }

    /// Animation `name` `seconds` in (at its own pace, else `fps`),
    /// centred on `pos` and facing `facing` (the one who hunts you in the
    /// Halloween act); false when the sheet has no such animation.
    pub fn draw_anim(&self, name: &str, seconds: f32, fps: f32, pos: Vec2, facing: f32, tint: Color) -> bool {
        let Some(anim) = self.anim(name) else { return false };
        let k = anim.at(seconds * anim.fps.unwrap_or(fps), anim.once);
        self.blit_at(anim.pics[k], pos, facing, 0.0, tint);
        true
    }

    /// Draw him from the sheet (world pass); golden when super.
    pub fn draw(&self, s: &Sonic) {
        let Some((anim, k)) = self.pick(s) else { return };
        let upright = !s.grounded || (s.gsp == 0.0 && super::mode_down(s.angle).y > 0.5 && s.angle.sin().abs() < 0.4);
        let angle = if upright && s.grounded { 0.0 } else { s.angle };
        let golden = if s.super_form { golden_material() } else { None };
        if let Some(m) = &golden {
            gl_use_material(m);
            m.set_uniform("shine", (get_time() as f32 * 10.0).sin());
            // Sonic's blue turns gold; the others brighten all over.
            m.set_uniform("whole", if s.character == Character::Sonic { 0.0f32 } else { 1.0 });
        }
        if !anim.behind.is_empty() {
            // Tails' tails swish on their own clock.
            let t = (get_time() * if s.flying.is_some() { 20.0 } else { 8.0 }) as usize;
            self.blit(anim.behind[t % anim.behind.len()], s, angle);
        }
        self.blit(anim.pics[k], s, angle);
        if golden.is_some() {
            gl_use_default_material();
        }
    }
}

const GOLDEN_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

const GOLDEN_FRAGMENT: &str = r#"#version 100
precision mediump float;
varying lowp vec4 color;
varying lowp vec2 uv;
uniform sampler2D Texture;
uniform float shine;
uniform float whole;
void main() {
    vec4 c = texture2D(Texture, uv) * color;
    float blue = c.b - max(c.r, c.g);
    if (whole > 0.5) {
        c.rgb = mix(c.rgb, vec3(1.0, 0.95, 0.7), 0.35 + shine * 0.1);
    } else if (blue > 0.1) {
        float l = (c.r + c.g + c.b) / 3.0 + c.b * 0.35;
        c.rgb = min(vec3(1.0, 0.85, 0.3) * (0.55 + l * 0.9) + shine * 0.08, vec3(1.0));
    }
    gl_FragColor = c;
}
"#;

thread_local! {
    static GOLDEN: std::cell::OnceCell<Option<Material>> = const { std::cell::OnceCell::new() };
}

/// The shader that turns a sprite golden (made once; None if it cannot be).
fn golden_material() -> Option<Material> {
    GOLDEN.with(|cell| {
        cell.get_or_init(|| {
            use miniquad::{BlendFactor, BlendState, BlendValue, Equation, PipelineParams};
            let blend = BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            );
            let params = MaterialParams {
                uniforms: vec![
                    UniformDesc::new("shine", UniformType::Float1),
                    UniformDesc::new("whole", UniformType::Float1),
                ],
                pipeline_params: PipelineParams { color_blend: Some(blend), ..Default::default() },
                ..Default::default()
            };
            let source = ShaderSource::Glsl { vertex: GOLDEN_VERTEX, fragment: GOLDEN_FRAGMENT };
            load_material(source, params).ok()
        })
        .clone()
    })
}

fn upload(img: &image::RgbaImage) -> Texture2D {
    let texture = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, img);
    texture.set_filter(FilterMode::Nearest);
    texture
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_parse_and_backgrounds_clear() {
        let json = r#"{"cell": [48, 48], "transparent": [[0, 128, 0]], "animations": {"idle": [[0, 0]], "walk": [[48, 0], [96, 0]]}}"#;
        let l: Layout = serde_json::from_str(json).unwrap();
        assert_eq!(l.cell, [48, 48]);
        assert_eq!(l.scale, 1.0, "scale defaults to 1");
        assert_eq!(l.animations["walk"].len(), 2);
        let mut img = image::RgbaImage::from_pixel(2, 1, image::Rgba([0, 128, 0, 255]));
        img.put_pixel(1, 0, image::Rgba([10, 20, 200, 255]));
        clear_background(&mut img, &l.transparent);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(1, 0)[3], 255);
        let s1 = sonic1();
        for (name, frames) in &s1.animations {
            for f in frames {
                assert!(f[0] + 66 <= 690 && f[1] + 66 <= 1558, "{name} cell inside the sheet");
            }
        }
    }

    #[test]
    fn built_in_pictures_lie_inside_their_sheets() {
        for b in BUILTINS {
            assert!(b.specs.iter().any(|s| s.name == "idle"), "{:?} has idle", b.character);
            for s in b.specs {
                for r in s.pics.iter().chain(s.behind) {
                    let (w, h) = b.size;
                    assert!((r[0] + r[2]) as u32 <= w && (r[1] + r[3]) as u32 <= h, "{} {r:?}", s.name);
                }
                // Centres given for every picture (or none), inside them.
                assert!(s.at.is_empty() || s.at.len() == s.pics.len(), "{} centres", s.name);
                for (r, c) in s.pics.iter().zip(s.at) {
                    assert!(c[0] < r[2] && c[1] < r[3], "{} centre {c:?} in {r:?}", s.name);
                }
            }
            assert_eq!(character_of(b.size), Some(b.character));
        }
    }

    #[test]
    fn centres_come_from_the_pixels() {
        // A 10 × 20 figure standing at the bottom of a 48 px box.
        let mut img = image::RgbaImage::from_pixel(48, 48, image::Rgba([0, 0, 0, 0]));
        for y in 28..48 {
            for x in 19..29 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        let ([x0, y0, x1, y1], head) = measure(&img, [0, 0, 48, 48]).unwrap();
        assert_eq!((x0, y0, x1, y1), (19.0, 28.0, 29.0, 48.0));
        assert_eq!(head, 24.0);
        let anim = cut(&img, &TAILS, &spec("idle", Fit::Feet, &[[0, 0, 48, 48]]));
        // Tails stands 15 guide px tall above his feet.
        assert_eq!(anim.pics[0].centre, vec2(24.0, 48.0 - 15.0));
        // Soft cut-out of near colours.
        let mut img = image::RgbaImage::from_pixel(2, 1, image::Rgba([2, 188, 222, 255]));
        img.put_pixel(1, 0, image::Rgba([240, 80, 140, 255]));
        clear_background_near(&mut img, AMY.background);
        assert_eq!(img.get_pixel(0, 0)[3], 0);
        assert_eq!(img.get_pixel(1, 0)[3], 255);
    }
    #[test]
    fn sprites_are_found_around_a_point_or_in_a_box() {
        // Two figures: 10 × 20 at (5, 10), with a hole, and 6 × 6 at (30, 4).
        let mut img = image::RgbaImage::from_pixel(48, 40, image::Rgba([0, 0, 0, 0]));
        for y in 10..30 {
            for x in 5..15 {
                img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }
        img.put_pixel(9, 20, image::Rgba([0, 0, 0, 0]));
        for y in 4..10 {
            for x in 30..36 {
                img.put_pixel(x, y, image::Rgba([0, 0, 255, 255]));
            }
        }
        // From one of its pixels, it grows to the whole figure (not the other).
        let p = find_sprite(&img, Rect::new(6.0, 11.0, 1.0, 1.0), 64.0, Some(15.0)).unwrap();
        assert_eq!(p.src, Rect::new(5.0, 10.0, 10.0, 20.0));
        assert_eq!(p.centre, vec2(5.0, 20.0 - 15.0), "standing: under the head, its half height above the feet");
        // Nothing to find in the hole alone.
        assert!(find_sprite(&img, Rect::new(9.0, 20.0, 1.0, 1.0), 64.0, None).is_none());
        // A loose box tightens around what is in it, centred in the middle.
        let p = find_sprite(&img, Rect::new(26.0, 0.0, 14.0, 14.0), 0.0, None).unwrap();
        assert_eq!(p.src, Rect::new(30.0, 4.0, 6.0, 6.0));
        assert_eq!(p.centre, vec2(3.0, 3.0));
        // A box cutting a figure grows to it, within reach (8 px past it).
        let p = find_sprite(&img, Rect::new(5.0, 10.0, 10.0, 8.0), 8.0, None).unwrap();
        assert_eq!(p.src, Rect::new(5.0, 10.0, 10.0, 16.0));
        let p = find_sprite(&img, Rect::new(5.0, 10.0, 10.0, 8.0), 16.0, None).unwrap();
        assert_eq!(p.src, Rect::new(5.0, 10.0, 10.0, 20.0));
    }

    #[test]
    fn pictures_hold_and_animations_loop_or_play_once() {
        let pic = |hold: f32| Pic { hold, ..Pic::new(Rect::new(0.0, 0.0, 8.0, 8.0), vec2(4.0, 4.0)) };
        // The second picture shows twice as long as the others.
        let a = Anim { pics: vec![pic(1.0), pic(2.0), pic(1.0)], ..Default::default() };
        assert_eq!(a.length(), 4.0);
        let looped: Vec<usize> = (0..9).map(|t| a.at(t as f32, false)).collect();
        assert_eq!(looped, [0, 1, 1, 2, 0, 1, 1, 2, 0]);
        // Once through, it stays on its last picture.
        let once: Vec<usize> = (0..7).map(|t| a.at(t as f32, true)).collect();
        assert_eq!(once, [0, 1, 1, 2, 2, 2, 2]);
        assert_eq!(Anim::default().at(3.0, false), 0, "no pictures: nothing to pick from");
    }

    #[test]
    fn sprites_are_found_on_the_whole_sheet_in_reading_order() {
        let mut img = image::RgbaImage::from_pixel(64, 40, image::Rgba([0, 0, 0, 0]));
        let mut fill = |x0: u32, y0: u32, x1: u32, y1: u32| {
            for y in y0..y1 {
                for x in x0..x1 {
                    img.put_pixel(x, y, image::Rgba([200, 60, 60, 255]));
                }
            }
        };
        // A figure whose detached piece is a pixel away, another to its
        // left lower down, one on the next row, and a speck.
        fill(30, 4, 40, 16);
        fill(41, 6, 44, 10);
        fill(4, 6, 14, 18);
        fill(10, 24, 20, 36);
        fill(56, 30, 58, 32);
        let found = detect_sprites(&img);
        assert_eq!(
            found,
            [Rect::new(30.0, 4.0, 14.0, 12.0), Rect::new(4.0, 6.0, 10.0, 12.0), Rect::new(10.0, 24.0, 10.0, 12.0)]
        );
    }
}
