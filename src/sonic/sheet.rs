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

/// Which character's built-in sheet an image of this size is.
pub fn character_of(size: (u32, u32)) -> Option<Character> {
    BUILTINS.iter().find(|b| b.size == size).map(|b| b.character)
}

/// A picture: where it is on the sheet, and where the character's centre
/// is in it (sheet px from its top-left).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Pic {
    src: Rect,
    centre: Vec2,
}

#[derive(Default)]
struct Anim {
    pics: Vec<Pic>,
    behind: Vec<Pic>,
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

/// Is this file one of the other sheets rather than Sonic's?
fn is_not_sonics(p: &Path) -> bool {
    use crate::game::sprites::{MONITORS, OBJECTS};
    let others = Character::ALL.iter().skip(1).flat_map(|&c| file_names(c).iter().copied());
    p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == OBJECTS || n == MONITORS || others.clone().any(|o| o == n))
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
            Pic { src: rect(r), centre }
        })
        .collect();
    // Pieces behind are boxed like the body: same place in their box.
    let behind = s.behind.iter().map(|&r| Pic { src: rect(r), centre: vec2(dx, dy) }).collect();
    Anim { pics, behind }
}

impl Sheet {
    /// `c`'s sheet in the Sonic folder, if there is one.
    pub fn load(c: Character) -> Result<Option<Sheet>, String> {
        let Some(dir) = dir().filter(|d| d.is_dir()) else { return Ok(None) };
        if c == Character::Sonic {
            return Self::load_sonic(&dir);
        }
        let Some(path) = file_names(c).iter().map(|n| dir.join(n)).find(|p| p.is_file()) else { return Ok(None) };
        let mut img = image::open(&path).map_err(|e| e.to_string())?.to_rgba8();
        // A character may have more than one known sheet: its size says which.
        let Some(b) = BUILTINS.iter().find(|b| b.character == c && b.size == img.dimensions()) else {
            return Err(format!("not a known sheet ({}×{})", img.width(), img.height()));
        };
        if b.lossy {
            clear_background_near(&mut img, b.background);
        } else {
            clear_background(&mut img, b.background);
        }
        let anims = b.specs.iter().map(|s| (s.name.to_string(), cut(&img, b, s))).collect();
        let name = path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        Ok(Some(Sheet { texture: upload(&img), anims, scale: 1.0, name }))
    }

    fn load_sonic(dir: &Path) -> Result<Option<Sheet>, String> {
        let (png, layout) = match find(dir) {
            Ok(found) => found,
            Err(e) if e == "no PNG sprite sheet" => return Ok(None),
            Err(e) => return Err(e),
        };
        let mut img = image::open(&png).map_err(|e| e.to_string())?.to_rgba8();
        let layout = match layout {
            Some(l) => l,
            None if img.dimensions() == (690, 1558) => sonic1(),
            None => return Err("unknown sheet layout: add a sheet.json beside it (see the README)".into()),
        };
        if !layout.animations.contains_key("idle") {
            return Err("the sheet has no \"idle\" animation".into());
        }
        clear_background(&mut img, &layout.transparent);
        let [w, h] = layout.cell.map(|v| v as f32);
        let anims = layout
            .animations
            .iter()
            .map(|(name, cells)| {
                let pics = cells
                    .iter()
                    .map(|&[x, y]| Pic { src: Rect::new(x as f32, y as f32, w, h), centre: vec2(w, h) / 2.0 })
                    .collect();
                (name.clone(), Anim { pics, behind: vec![] })
            })
            .collect();
        let name = png.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        Ok(Some(Sheet { texture: upload(&img), anims, scale: layout.scale, name }))
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

    fn anim(&self, name: &str) -> Option<&Anim> {
        self.anims.get(name).filter(|a| !a.pics.is_empty())
    }

    /// The animation for his state, and which of its pictures.
    fn pick(&self, s: &Sonic) -> Option<(&Anim, usize)> {
        let speed = if s.grounded { s.gsp.abs() } else { s.xsp.abs() };
        let has = |n: &str| self.anim(n).is_some();
        let name = if s.dead && has("dead") {
            "dead"
        } else if (s.hurt || s.dead) && has("hurt") {
            "hurt"
        } else if s.climb_up.is_some() && has("climb_up") {
            "climb_up"
        } else if s.climbing.is_some() && has("climb") {
            "climb"
        } else if s.gliding.is_some() && has("glide") {
            "glide"
        } else if s.sliding && has("slide") {
            "slide"
        } else if s.dropping && has("drop") {
            "drop"
        } else if s.flying.is_some() && s.tired() && has("fly_tired") {
            "fly_tired"
        } else if s.flying.is_some() && has("fly") {
            "fly"
        } else if s.hammer.is_some() && s.hammer_air && has("hammer_air") {
            "hammer_air"
        } else if s.hammer.is_some() && !s.hammer_air && has("hammer") {
            "hammer"
        } else if s.warp.is_some() && has("warp") {
            "warp"
        } else if s.spindash.is_some() && has("spindash") {
            "spindash"
        } else if s.is_ball() {
            "roll"
        } else if !s.grounded && s.ysp < 0.0 {
            "spring"
        } else if s.skidding {
            "skid"
        } else if s.pushing {
            "push"
        } else if s.crouching {
            "crouch"
        } else if s.looking_up {
            "look_up"
        } else if s.grounded && speed == 0.0 {
            if s.idle > 180 && has("bored") {
                "bored"
            } else {
                "idle"
            }
        } else if speed >= 6.0 && has("run") {
            "run"
        } else {
            "walk"
        };
        let anim = self.anim(name).or_else(|| self.anim("walk")).or_else(|| self.anim("idle"))?;
        let n = anim.pics.len();
        let k = match name {
            "bored" => ((s.idle - 180) / 30) as usize,
            "push" => (s.anim / 8.0) as usize,
            // Played once through.
            "hammer" => s.hammer.map_or(0, |h| (h as usize * n / 22).min(n - 1)),
            "climb_up" => s.climb_up.map_or(0, |(t, _, _)| (t as usize * n / 18).min(n - 1)),
            _ => s.anim as usize,
        };
        Some((anim, k % n))
    }

    fn blit(&self, pic: Pic, s: &Sonic, angle: f32) {
        self.blit_at(pic, s.pos, s.facing, angle, WHITE);
    }

    fn blit_at(&self, pic: Pic, pos: Vec2, facing: f32, angle: f32, tint: Color) {
        let k = super::SCALE / self.scale.max(0.01);
        let size = vec2(pic.src.w, pic.src.h) * k;
        let flip = facing < 0.0;
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

    /// Picture `k` of animation `name`, centred on `pos` and facing
    /// `facing` (the one who hunts you in the Halloween act); false when the
    /// sheet has no such animation.
    pub fn draw_anim(&self, name: &str, k: usize, pos: Vec2, facing: f32, tint: Color) -> bool {
        let Some(anim) = self.anim(name) else { return false };
        self.blit_at(anim.pics[k % anim.pics.len()], pos, facing, 0.0, tint);
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
}
