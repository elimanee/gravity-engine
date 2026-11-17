//! Characters made by the player (in the character editor): a name, the
//! built-in character whose drawing, size and sheet they start from, their
//! colours, a kit (how they move and their moves), and a 3D model. Kept in
//! `characters/` (one `.json` each) and known to the game as
//! [`Character::Custom`], by their place in the list loaded here.

use super::kit::Kit;
use super::Character;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Custom {
    pub name: String,
    /// Whose drawing, size and sprite sheet they start from (a built-in).
    pub base: Character,
    /// Their colours (main, belly and muzzle, shoes), or the base's.
    pub colours: Option<[[u8; 3]; 3]>,
    pub kit: Kit,
    /// Their 3D model: its name in a model pack's models.dat (empty: the
    /// base's).
    pub model: String,
}

impl Default for Custom {
    fn default() -> Self {
        Custom { name: String::new(), base: Character::Sonic, colours: None, kit: Kit::default(), model: String::new() }
    }
}

impl Custom {
    /// Kept within bounds (a built-in base, a sane kit, a short name).
    pub fn sanitized(mut self) -> Custom {
        self.base = self.base.base();
        self.kit = self.kit.sanitized();
        self.name = self.name.trim().chars().filter(|c| !c.is_control()).take(24).collect();
        if self.name.is_empty() {
            self.name = "Custom".into();
        }
        self.model = self.model.trim().chars().filter(|c| c.is_ascii_graphic()).take(32).collect();
        self
    }
}

/// The most characters the game knows at once.
pub const MOST: usize = 200;

struct Entry {
    custom: Custom,
    /// The name, kept for the life of the program (labels are `&'static`).
    label: &'static str,
}

static LIST: RwLock<Vec<Entry>> = RwLock::new(Vec::new());
static LOADED: AtomicBool = AtomicBool::new(false);

/// The character being made in the editor (tried before it is kept), as
/// [`PREVIEW`]; its name kept alive like the others'.
static DRAFT: RwLock<Option<Entry>> = RwLock::new(None);

/// The place that means the character in the editor.
pub const PREVIEW: u8 = u8::MAX;

/// Show `c` as the character being made (or none).
pub fn set_preview(c: Option<Custom>) {
    let mut d = DRAFT.write().unwrap_or_else(|e| e.into_inner());
    *d = c.map(|custom| {
        let custom = custom.sanitized();
        let label = match d.as_ref().filter(|e| e.label == custom.name) {
            Some(e) => e.label,
            None => Box::leak(custom.name.clone().into_boxed_str()),
        };
        Entry { custom, label }
    });
}

pub fn dir() -> Option<PathBuf> {
    crate::config::config_dir().map(|c| c.join("characters"))
}

/// A file name for a character's name.
pub fn slug(name: &str) -> String {
    let s: String = name.trim().to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() {
        "custom".into()
    } else {
        s
    }
}

fn set(list: Vec<Custom>) {
    let mut list: Vec<Custom> = list.into_iter().map(Custom::sanitized).collect();
    list.sort_by_key(|c| c.name.to_lowercase());
    list.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    list.truncate(MOST);
    let mut out = LIST.write().unwrap_or_else(|e| e.into_inner());
    // Labels stay alive: reuse the ones already made for the same names.
    let old: Vec<&'static str> = out.iter().map(|e| e.label).collect();
    *out = list
        .into_iter()
        .map(|custom| {
            let label = old
                .iter()
                .copied()
                .find(|l| *l == custom.name)
                .unwrap_or_else(|| Box::leak(custom.name.clone().into_boxed_str()));
            Entry { custom, label }
        })
        .collect();
    LOADED.store(true, Ordering::Relaxed);
}

/// Read the characters' files again.
pub fn reload() {
    let Some(dir) = dir() else {
        set(vec![]);
        return;
    };
    let list = crate::storage::list(&dir)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("json")))
        .filter_map(|p| crate::storage::read_to_string(&p).ok())
        .filter_map(|text| serde_json::from_str::<Custom>(&text).ok())
        .collect();
    set(list);
}

fn ensure() {
    if !LOADED.load(Ordering::Relaxed) {
        reload();
    }
}

/// How many there are.
pub fn count() -> usize {
    ensure();
    LIST.read().map_or(0, |l| l.len())
}

pub fn get(i: u8) -> Option<Custom> {
    if i == PREVIEW {
        return DRAFT.read().ok()?.as_ref().map(|e| e.custom.clone());
    }
    ensure();
    LIST.read().ok()?.get(i as usize).map(|e| e.custom.clone())
}

pub fn label(i: u8) -> &'static str {
    if i == PREVIEW {
        return DRAFT.read().ok().and_then(|d| d.as_ref().map(|e| e.label)).unwrap_or("New character");
    }
    ensure();
    LIST.read().ok().and_then(|l| l.get(i as usize).map(|e| e.label)).unwrap_or("Custom")
}

/// The one with this name.
pub fn find(name: &str) -> Option<Character> {
    ensure();
    let l = LIST.read().ok()?;
    l.iter().position(|e| e.custom.name.eq_ignore_ascii_case(name)).map(|i| Character::Custom(i as u8))
}

/// Every one, in order.
pub fn all() -> Vec<Character> {
    (0..count()).map(|i| Character::Custom(i as u8)).collect()
}

/// Keep `c` (in place of `was`, the name it had, when renamed); the list
/// is read again.
pub fn save(c: &Custom, was: Option<&str>) -> Result<Character, String> {
    let c = c.clone().sanitized();
    let dir = dir().ok_or("no folder for the characters")?;
    if let Some(old) = was.filter(|o| slug(o) != slug(&c.name)) {
        let _ = crate::storage::remove(&dir.join(format!("{}.json", slug(old))));
    }
    let json = serde_json::to_string_pretty(&c).map_err(|e| e.to_string())?;
    crate::storage::write(&dir.join(format!("{}.json", slug(&c.name))), json)?;
    reload();
    find(&c.name).ok_or_else(|| "saved, but could not be read back".to_string())
}

/// Forget the one named `name` (its file goes).
pub fn delete(name: &str) -> Result<(), String> {
    let dir = dir().ok_or("no folder for the characters")?;
    crate::storage::remove(&dir.join(format!("{}.json", slug(name))))?;
    reload();
    Ok(())
}

/// (Tests.) Make these the characters known, without files.
#[cfg(test)]
pub fn set_for_tests(list: Vec<Custom>) {
    set(list);
}

/// (Tests.) Held by the tests that change the list or the one edited.
#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sonic::kit::{Ability, Block};

    #[test]
    fn names_make_file_names() {
        assert_eq!(slug("Metal Sonic!"), "metal-sonic");
        assert_eq!(slug("  "), "custom");
        assert_eq!(slug("Été 2"), "été-2");
    }

    #[test]
    fn a_character_saves_and_comes_back_and_is_kept_sane() {
        let c = Custom {
            name: "Blaze".into(),
            base: Character::Amy,
            colours: Some([[150, 60, 200], [250, 220, 230], [230, 30, 60]]),
            kit: Kit { blocks: vec![Block::new(Ability::Shoot { speed: 10.0 })], ..Kit::default() },
            model: "BLAZE".into(),
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"base\":\"amy\""), "{json}");
        assert_eq!(serde_json::from_str::<Custom>(&json).unwrap(), c);
        // Odd files are made sane.
        let odd: Custom = serde_json::from_str(r#"{"name":"  ","model":"a b\nc"}"#).unwrap();
        let odd = odd.sanitized();
        assert_eq!((odd.name.as_str(), odd.model.as_str()), ("Custom", "abc"));
    }

    #[test]
    fn made_up_characters_are_known_by_name_after_the_built_in_ones() {
        let _held = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let made = |name: &str, base| Custom { name: name.into(), base, ..Custom::default() };
        // Kept in order of their names, one of each name.
        set_for_tests(vec![
            made("Nova", Character::Tails),
            made("blaze", Character::Amy),
            made("Blaze", Character::Sonic),
        ]);
        assert_eq!(count(), 2);
        let (blaze, nova) = (Character::Custom(0), Character::Custom(1));
        assert_eq!((blaze.label(), nova.label()), ("blaze", "Nova"));
        assert_eq!((blaze.base(), nova.base()), (Character::Amy, Character::Tails));
        // Kept in files by name, found again by it (whatever the case).
        assert_eq!(serde_json::to_string(&nova).unwrap(), "\"custom:Nova\"");
        assert_eq!(serde_json::from_str::<Character>("\"custom:NOVA\"").unwrap(), nova);
        assert_eq!(Character::from_key("custom:Gone"), Character::Sonic, "one deleted is Sonic again");
        // Shift+N goes through them after the built-in ones, and round.
        assert_eq!(Character::Exe.cycle(1), blaze);
        assert_eq!(nova.cycle(1), Character::Sonic);
        assert_eq!(Character::Sonic.cycle(-1), nova);
        // The one being edited is apart from the list.
        set_preview(Some(made("Draft", Character::Knuckles)));
        assert_eq!(
            (Character::Custom(PREVIEW).label(), Character::Custom(PREVIEW).base()),
            ("Draft", Character::Knuckles)
        );
        assert!(!Character::every().contains(&Character::Custom(PREVIEW)));
        set_preview(None);
        set_for_tests(vec![]);
    }
}
