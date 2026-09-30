//! Classic Winamp 2 skins, as used by Audacious's "Winamp Classic" interface:
//! `.wsz` / `.zip` archives or plain folders of BMP or PNG sprite sheets
//! (`main`, `cbuttons`, `titlebar`, `text`, `numbers` / `nums_ex`, …), plus
//! `viscolor.txt` (visualizer colours) and Audacious's `skin.hints` (moved
//! widgets and other window sizes).

use image::RgbaImage;
use macroquad::prelude::*;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const SKIN_EXT: [&str; 2] = ["wsz", "zip"];

/// Sprite sheets a skin may have; only `main` is required.
const SHEETS: [&str; 14] = [
    "main", "titlebar", "cbuttons", "text", "numbers", "playpaus", "monoster", "posbar", "volume", "balance",
    "shufrep", "nums_ex", "pledit", "eqmain",
];

/// Colours of the playlist window (`pledit.txt`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaylistColors {
    pub normal: [u8; 3],
    pub current: [u8; 3],
    pub normal_bg: [u8; 3],
    pub selected_bg: [u8; 3],
}

impl Default for PlaylistColors {
    fn default() -> Self {
        PlaylistColors { normal: [0, 255, 0], current: [255, 255, 255], normal_bg: [0, 0, 0], selected_bg: [0, 0, 198] }
    }
}

/// Parse `pledit.txt` (`Normal=#00FF00`, …); missing keys keep Winamp's colours.
pub fn parse_pledit(s: &str) -> PlaylistColors {
    let mut c = PlaylistColors::default();
    for line in s.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let hex = v.trim().trim_start_matches('#');
        let Some(rgb) = (hex.len() >= 6)
            .then(|| (0..3).map(|i| u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()).collect::<Option<Vec<u8>>>())
            .flatten()
        else {
            continue;
        };
        let rgb = [rgb[0], rgb[1], rgb[2]];
        match k.trim().to_ascii_lowercase().as_str() {
            "normal" => c.normal = rgb,
            "current" => c.current = rgb,
            "normalbg" => c.normal_bg = rgb,
            "selectedbg" => c.selected_bg = rgb,
            _ => {}
        }
    }
    c
}

/// Where the widgets of the main window are, in skin pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub size: Vec2,
    /// Minus sign, minute tens, minute units, second tens, second units.
    pub numbers: [Vec2; 5],
    pub play_status: Vec2,
    pub vis: Vec2,
    pub text: Vec2,
    pub text_width: f32,
    pub volume: Vec2,
    pub balance: Vec2,
    pub position: Vec2,
    pub mono: Vec2,
    pub stereo: Vec2,
    pub previous: Vec2,
    pub play: Vec2,
    pub pause: Vec2,
    pub stop: Vec2,
    pub next: Vec2,
    pub eject: Vec2,
    pub shuffle: Vec2,
    pub repeat: Vec2,
    pub eq_button: Vec2,
    pub pl_button: Vec2,
    pub close: Vec2,
    pub text_visible: bool,
    pub vis_visible: bool,
    /// Info text and the mono / stereo lights.
    pub othertext_visible: bool,
}

impl Default for Layout {
    /// The Winamp 2 positions.
    fn default() -> Self {
        Layout {
            size: vec2(275.0, 116.0),
            numbers: [vec2(36.0, 26.0), vec2(48.0, 26.0), vec2(60.0, 26.0), vec2(78.0, 26.0), vec2(90.0, 26.0)],
            play_status: vec2(24.0, 28.0),
            vis: vec2(24.0, 43.0),
            text: vec2(111.0, 27.0),
            text_width: 154.0,
            volume: vec2(107.0, 57.0),
            balance: vec2(177.0, 57.0),
            position: vec2(16.0, 72.0),
            mono: vec2(212.0, 41.0),
            stereo: vec2(239.0, 41.0),
            previous: vec2(16.0, 88.0),
            play: vec2(39.0, 88.0),
            pause: vec2(62.0, 88.0),
            stop: vec2(85.0, 88.0),
            next: vec2(108.0, 88.0),
            eject: vec2(136.0, 89.0),
            shuffle: vec2(164.0, 89.0),
            repeat: vec2(210.0, 89.0),
            eq_button: vec2(219.0, 58.0),
            pl_button: vec2(242.0, 58.0),
            close: vec2(264.0, 3.0),
            text_visible: true,
            vis_visible: true,
            othertext_visible: true,
        }
    }
}

impl Layout {
    /// Apply Audacious `skin.hints` (`mainwinPlayX=33`, …).
    pub fn with_hints(mut self, hints: &str) -> Self {
        let mut v: HashMap<String, f32> = HashMap::new();
        for line in hints.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((k, val)) = line.split_once('=') else { continue };
            if let Ok(n) = val.trim().parse::<f32>() {
                v.insert(k.trim().to_ascii_lowercase(), n);
            }
        }
        let get = |k: &str| v.get(&format!("mainwin{k}")).copied();
        let pos = |p: &mut Vec2, name: &str| {
            if let Some(x) = get(&format!("{name}x")) {
                p.x = x;
            }
            if let Some(y) = get(&format!("{name}y")) {
                p.y = y;
            }
        };
        for (i, n) in self.numbers.iter_mut().enumerate() {
            pos(n, &format!("number{i}"));
        }
        let named: [(&mut Vec2, &str); 19] = [
            (&mut self.play_status, "playstatus"),
            (&mut self.vis, "vis"),
            (&mut self.text, "text"),
            (&mut self.volume, "volume"),
            (&mut self.balance, "balance"),
            (&mut self.position, "position"),
            (&mut self.previous, "previous"),
            (&mut self.play, "play"),
            (&mut self.pause, "pause"),
            (&mut self.stop, "stop"),
            (&mut self.next, "next"),
            (&mut self.eject, "eject"),
            (&mut self.shuffle, "shuffle"),
            (&mut self.repeat, "repeat"),
            (&mut self.eq_button, "eqbutton"),
            (&mut self.pl_button, "plbutton"),
            (&mut self.close, "close"),
            (&mut self.mono, "mono"),
            (&mut self.stereo, "stereo"),
        ];
        for (p, name) in named {
            pos(p, name);
        }
        if let Some(w) = get("textwidth") {
            self.text_width = w;
        }
        if let Some(w) = get("width") {
            self.size.x = w;
        }
        if let Some(h) = get("height") {
            self.size.y = h;
        }
        if let Some(t) = get("textvisible") {
            self.text_visible = t != 0.0;
        }
        if let Some(t) = get("visvisible") {
            self.vis_visible = t != 0.0;
        }
        if let Some(t) = get("othertextvisible") {
            self.othertext_visible = t != 0.0;
        }
        self
    }
}

/// Winamp's default visualizer palette (used when `viscolor.txt` is missing).
const DEFAULT_VIS: [[u8; 3]; 24] = [
    [0, 0, 0],
    [24, 33, 41],
    [239, 49, 16],
    [206, 41, 16],
    [214, 90, 0],
    [214, 102, 0],
    [214, 115, 0],
    [198, 123, 8],
    [222, 165, 24],
    [214, 181, 33],
    [189, 222, 41],
    [148, 222, 33],
    [41, 206, 16],
    [50, 190, 16],
    [57, 181, 16],
    [49, 156, 8],
    [41, 148, 0],
    [24, 132, 8],
    [255, 255, 255],
    [214, 214, 222],
    [181, 189, 189],
    [160, 170, 175],
    [148, 156, 165],
    [150, 150, 150],
];

/// Parse `viscolor.txt`: 24 lines of `r,g,b` (comments allowed).
pub fn parse_viscolor(s: &str) -> [[u8; 3]; 24] {
    let mut out = DEFAULT_VIS;
    let lines = s.lines().filter_map(|line| {
        let nums: Vec<u8> = line
            .split(|c: char| !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
            .take(3)
            .filter_map(|t| t.parse::<u32>().ok().map(|n| n.min(255) as u8))
            .collect();
        (nums.len() == 3).then(|| [nums[0], nums[1], nums[2]])
    });
    for (slot, rgb) in out.iter_mut().zip(lines) {
        *slot = rgb;
    }
    out
}

/// Cell (column, row) of `c` in `text.bmp` (5 × 6 pixel glyphs).
pub fn glyph(c: char) -> (u32, u32) {
    const ROW0: &str = "abcdefghijklmnopqrstuvwxyz\"@";
    const ROW1: &str = "0123456789….:()-'!_+\\/[]^&%,=$#";
    const ROW2: &str = "åöä?*";
    let c = c.to_lowercase().next().unwrap_or(' ');
    for (row, chars) in [ROW0, ROW1, ROW2].iter().enumerate() {
        if let Some(col) = chars.chars().position(|x| x == c) {
            return (col as u32, row as u32);
        }
    }
    match c {
        '<' => (22, 1),
        '>' => (23, 1),
        '{' => (13, 1),
        '}' => (14, 1),
        ';' => (12, 1),
        '|' => (21, 1),
        '`' => (16, 1),
        '~' => (15, 1),
        'é' | 'è' | 'ê' | 'ë' => (4, 0),
        'à' | 'â' => (0, 0),
        'ù' | 'û' | 'ü' => (20, 0),
        'î' | 'ï' => (8, 0),
        'ô' => (14, 0),
        'ç' => (2, 0),
        _ => (30, 0), // space
    }
}

/// Minute and second digits for the time display (minutes capped at 99).
pub fn time_digits(secs: f64) -> [u32; 4] {
    let s = secs.max(0.0) as u64;
    let (m, s) = ((s / 60).min(99), s % 60);
    [(m / 10) as u32, (m % 10) as u32, (s / 10) as u32, (s % 10) as u32]
}

/// A skin decoded in memory (no textures yet).
pub struct SkinData {
    pub name: String,
    pub sheets: HashMap<&'static str, RgbaImage>,
    pub layout: Layout,
    pub vis: [[u8; 3]; 24],
    pub playlist: PlaylistColors,
}

impl SkinData {
    pub fn load(path: &Path) -> Result<Self, String> {
        let files = if path.is_dir() { read_dir_files(path)? } else { read_zip_files(path)? };
        Self::from_files(skin_name(path), &files)
    }

    /// Build from `file name (lower case) → bytes`.
    pub fn from_files(name: String, files: &HashMap<String, Vec<u8>>) -> Result<Self, String> {
        let find = |stem: &str| ["bmp", "png"].iter().find_map(|ext| files.get(&format!("{stem}.{ext}")));
        let mut sheets = HashMap::new();
        for stem in SHEETS {
            if let Some(bytes) = find(stem) {
                match image::load_from_memory(bytes) {
                    Ok(img) => {
                        sheets.insert(stem, img.to_rgba8());
                    }
                    Err(e) => eprintln!("skin {name}: {stem}: {e}"),
                }
            }
        }
        if !sheets.contains_key("main") {
            return Err("not a Winamp skin (no main.bmp)".into());
        }
        // Audacious ships `nums_ex` (with a minus sign) instead of `numbers`.
        if let Some(n) = sheets.remove("nums_ex") {
            sheets.entry("numbers").or_insert(n);
        }
        let text = |f: &str| files.get(f).map(|b| String::from_utf8_lossy(b).into_owned());
        // The window is the size of main.bmp unless the hints say otherwise.
        let main = &sheets["main"];
        let mut layout = Layout { size: vec2(main.width() as f32, main.height() as f32), ..Layout::default() };
        if let Some(h) = text("skin.hints") {
            layout = layout.with_hints(&h);
        }
        let vis = text("viscolor.txt").map_or(DEFAULT_VIS, |s| parse_viscolor(&s));
        let playlist = text("pledit.txt").map_or_else(PlaylistColors::default, |s| parse_pledit(&s));
        Ok(SkinData { name, sheets, layout, vis, playlist })
    }
}

fn skin_name(path: &Path) -> String {
    path.file_stem().map_or_else(|| "Skin".into(), |s| s.to_string_lossy().into_owned())
}

/// Files directly in a folder (skins are flat).
fn read_dir_files(dir: &Path) -> Result<HashMap<String, Vec<u8>>, String> {
    let mut out = HashMap::new();
    for e in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        if p.is_file() {
            if let (Some(n), Ok(b)) = (p.file_name(), std::fs::read(&p)) {
                out.insert(n.to_string_lossy().to_ascii_lowercase(), b);
            }
        }
    }
    Ok(out)
}

/// Every file of an archive, by lower-case name (skins are often zipped
/// with a folder inside).
fn read_zip_files(path: &Path) -> Result<HashMap<String, Vec<u8>>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("not a zip archive: {e}"))?;
    let mut out = HashMap::new();
    for i in 0..zip.len() {
        let Ok(mut f) = zip.by_index(i) else { continue };
        if f.is_dir() || f.size() > 8 * 1024 * 1024 {
            continue;
        }
        let name = f.name().replace('\\', "/");
        let base = name.rsplit('/').next().unwrap_or(&name).to_ascii_lowercase();
        let mut bytes = Vec::with_capacity(f.size() as usize);
        if f.read_to_end(&mut bytes).is_ok() {
            out.insert(base, bytes);
        }
    }
    Ok(out)
}

/// A skin ready to draw.
pub struct Skin {
    pub name: String,
    pub path: Option<PathBuf>,
    sheets: HashMap<&'static str, Texture2D>,
    pub layout: Layout,
    pub vis: [Color; 24],
    pub playlist: PlaylistColors,
}

impl Skin {
    pub fn upload(data: SkinData, path: Option<PathBuf>) -> Self {
        let sheets = data
            .sheets
            .into_iter()
            .map(|(k, img)| {
                let t = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, img.as_raw());
                t.set_filter(FilterMode::Nearest);
                (k, t)
            })
            .collect();
        let vis = data.vis.map(|[r, g, b]| Color::from_rgba(r, g, b, 255));
        Skin { name: data.name, path, sheets, layout: data.layout, vis, playlist: data.playlist }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        SkinData::load(path).map(|d| Skin::upload(d, Some(path.to_path_buf())))
    }

    pub fn has(&self, sheet: &str) -> bool {
        self.sheets.contains_key(sheet)
    }

    /// Size of a sheet (0 when missing).
    pub fn sheet_size(&self, sheet: &str) -> Vec2 {
        self.sheets.get(sheet).map_or(Vec2::ZERO, |t| t.size())
    }

    /// Draw `src` of `sheet` with its top-left at `at` (skin px) of a window
    /// whose origin is `origin`, scaled by `scale`.
    pub fn sprite(&self, sheet: &str, src: Rect, at: Vec2, origin: Vec2, scale: f32) {
        let Some(t) = self.sheets.get(sheet) else { return };
        // Clip sources that run past small or odd-sized sheets.
        let (tw, th) = (t.width(), t.height());
        if src.x >= tw || src.y >= th {
            return;
        }
        let src = Rect::new(src.x, src.y, src.w.min(tw - src.x), src.h.min(th - src.y));
        draw_texture_ex(
            t,
            origin.x + at.x * scale,
            origin.y + at.y * scale,
            WHITE,
            DrawTextureParams { source: Some(src), dest_size: Some(src.size() * scale), ..Default::default() },
        );
    }
}

/// Folders where skins are looked for: Audacious's (system and user),
/// Winamp's on Windows, and our own `skins/` folder.
pub fn skin_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![];
    if let Some(c) = crate::config::config_dir() {
        dirs.push(c.join("skins"));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = std::env::var_os(var) {
                dirs.push(PathBuf::from(p).join("Winamp").join("Skins"));
            }
        }
    } else {
        let data_home = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|h| h.join(".local/share")));
        if let Some(d) = data_home {
            dirs.push(d.join("audacious/Skins"));
        }
        if let Some(h) = &home {
            dirs.push(h.join(".audacious/Skins"));
        }
        dirs.push(PathBuf::from("/usr/local/share/audacious/Skins"));
        dirs.push(PathBuf::from("/usr/share/audacious/Skins"));
    }
    dirs
}

/// Every skin found in [`skin_dirs`], sorted by name.
pub fn discover() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = vec![];
    for dir in skin_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for p in entries.flatten().map(|e| e.path()) {
            let skin_file = p.is_file() && SKIN_EXT.contains(&crate::config::ext_of(&p.to_string_lossy()).as_str());
            let skin_dir =
                p.is_dir() && ["main.bmp", "main.png", "MAIN.BMP", "Main.bmp"].iter().any(|m| p.join(m).is_file());
            if skin_file || skin_dir {
                found.push(p);
            }
        }
    }
    found.sort_by_key(|p| skin_name(p).to_lowercase());
    found.dedup_by_key(|p| skin_name(p).to_lowercase());
    found
}

pub fn display_name(path: &Path) -> String {
    skin_name(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = std::io::Cursor::new(vec![]);
        image::RgbaImage::new(w, h).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn hints_move_widgets_and_resize_the_window() {
        let hints = "[skin]\nmainwinWidth=425\nmainwinHeight=100\nmainwinPlayX=33 # comment\nmainwinPlayY=48\n\
                     mainwinNumber0X=28\nmainwinTextWidth=319\nmainwinVisVisible=0\n";
        let l = Layout::default().with_hints(hints);
        assert_eq!(l.size, vec2(425.0, 100.0));
        assert_eq!(l.play, vec2(33.0, 48.0));
        assert_eq!(l.numbers[0], vec2(28.0, 26.0));
        assert_eq!(l.text_width, 319.0);
        assert!(!l.vis_visible && l.text_visible);
        // Untouched widgets keep the Winamp positions.
        assert_eq!(l.stop, Layout::default().stop);
    }

    #[test]
    fn viscolor_accepts_comments_and_short_files() {
        let v = parse_viscolor("0,0,0\t// background\n 1, 2 ,3\nnot a colour\n255,128,64 // high\n");
        assert_eq!(&v[..3], &[[0, 0, 0], [1, 2, 3], [255, 128, 64]]);
        assert_eq!(v[23], DEFAULT_VIS[23]);
    }

    #[test]
    fn pledit_colours() {
        let c =
            parse_pledit("[Text]\nNormal=#00FF00\nCurrent=#ffffff\nNormalBG=#000000\nSelectedBG=#0000C6\nFont=Arial\n");
        assert_eq!(c.selected_bg, [0, 0, 198]);
        assert_eq!(c.current, [255, 255, 255]);
        assert_eq!(parse_pledit("NormalBG=zz\n").normal_bg, PlaylistColors::default().normal_bg);
    }

    #[test]
    fn glyphs_and_time_digits() {
        assert_eq!(glyph('a'), (0, 0));
        assert_eq!(glyph('Z'), (25, 0));
        assert_eq!(glyph('7'), (7, 1));
        assert_eq!(glyph(':'), (12, 1));
        assert_eq!(glyph('é'), glyph('e'));
        assert_eq!(glyph('€'), (30, 0));
        assert_eq!(time_digits(754.9), [1, 2, 3, 4]);
        assert_eq!(time_digits(99.0 * 60.0 + 3600.0), [9, 9, 0, 0]);
    }

    #[test]
    fn skins_load_from_folders_and_zip_archives() {
        let dir = std::env::temp_dir().join(format!("ge-skin-{}", std::process::id()));
        let folder = dir.join("Folder Skin");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("MAIN.PNG"), png(275, 116)).unwrap();
        std::fs::write(folder.join("nums_ex.png"), png(108, 13)).unwrap();
        let s = SkinData::load(&folder).unwrap();
        assert_eq!(s.name, "Folder Skin");
        assert!(s.sheets.contains_key("numbers"), "nums_ex stands in for numbers");
        assert_eq!(s.layout.size, vec2(275.0, 116.0));

        // A .wsz with the sheets inside a sub-folder, and hints.
        let wsz = dir.join("zipped.wsz");
        let mut zw = zip::ZipWriter::new(std::fs::File::create(&wsz).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        zw.start_file("zipped/Main.bmp", opts).unwrap();
        let mut bmp = std::io::Cursor::new(vec![]);
        image::RgbImage::new(275, 116).write_to(&mut bmp, image::ImageFormat::Bmp).unwrap();
        std::io::Write::write_all(&mut zw, bmp.get_ref()).unwrap();
        zw.start_file("zipped/skin.hints", opts).unwrap();
        std::io::Write::write_all(&mut zw, b"mainwinPlayX=1\n").unwrap();
        zw.finish().unwrap();
        let z = SkinData::load(&wsz).unwrap();
        assert_eq!((z.name.as_str(), z.layout.play.x), ("zipped", 1.0));

        assert!(SkinData::load(&dir.join("missing.wsz")).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
