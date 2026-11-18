//! What a character can do: how they move (their physics, as multiples of
//! the Sonic Physics Guide's), whether they roll and spin dash, and their
//! own moves, each one a block on a button. The five built-in characters
//! are kits like any other (Tails' flight is a block, Knuckles' glide
//! another…), so made-up characters can mix them, and new moves with
//! them: a double jump, a dash through the air, a homing attack, a stomp,
//! something to throw, Sonic Mania's drop dash, Sonic CD's peel out,
//! Sonic 3's insta-shield and bubble bounce, a wall jump, a boost. On top
//! of these, two things they may just be: a ring magnet, and never out of
//! air underwater.

use super::Character;
use serde::{Deserialize, Serialize};

/// The button a move is on: jumping again in the air, or the action
/// button (on the ground or in the air, whichever the move allows).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Button {
    #[default]
    JumpAgain,
    Action,
}

/// A move, with its setting.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "move", rename_all = "snake_case")]
pub enum Ability {
    /// Hover down slowly, a flap up at each press, for so many seconds.
    Fly { seconds: f32 },
    /// Glide on, faster and faster, sinking slowly; into a wall, climb it.
    Glide { climb: bool },
    /// Swing a hammer (on the ground; in the air, spin it).
    Hammer,
    /// Vanish and turn up this far ahead (guide px), short of walls.
    Teleport { distance: f32 },
    /// Jump again from the air, this high (as a part of a jump).
    DoubleJump { power: f32 },
    /// Shoot forward through the air at this speed.
    AirDash { speed: f32 },
    /// At the nearest badnik or monitor ahead within this reach (guide
    /// px); with none, a dash forward.
    Homing { range: f32 },
    /// Straight down at this speed, smashing what is under.
    Stomp { speed: f32 },
    /// Throw something forward at this speed that breaks badniks.
    Shoot { speed: f32 },
    /// Held in the air, it charges; landing charged, off rolling at this
    /// speed (Sonic Mania's drop dash).
    DropDash { speed: f32 },
    /// Held standing, legs a blur; let go, off running at this speed
    /// (Sonic CD's peel out).
    PeelOut { speed: f32 },
    /// A flash all round for a moment: what it touches breaks, nothing
    /// hurts (Sonic 3's insta-shield).
    InstaShield,
    /// Straight down, and up again off the ground this high (as a part of a
    /// jump), again and again.
    Bounce { height: f32 },
    /// Against a wall in the air: off it, this strong (as often as there
    /// is a wall, and the jump's other moves are ready again).
    WallJump { power: f32 },
    /// Held: faster at once, breaking what is in the way, for so many
    /// seconds (it fills again slowly).
    Boost { seconds: f32 },
}

impl Ability {
    /// Every move, with its usual setting.
    pub const ALL: [Ability; 15] = [
        Ability::DoubleJump { power: 0.8 },
        Ability::AirDash { speed: 9.0 },
        Ability::Homing { range: 160.0 },
        Ability::DropDash { speed: 8.0 },
        Ability::Stomp { speed: 12.0 },
        Ability::Bounce { height: 1.0 },
        Ability::WallJump { power: 1.0 },
        Ability::InstaShield,
        Ability::PeelOut { speed: 12.0 },
        Ability::Boost { seconds: 3.0 },
        Ability::Shoot { speed: 9.0 },
        Ability::Fly { seconds: 8.0 },
        Ability::Glide { climb: true },
        Ability::Hammer,
        Ability::Teleport { distance: 56.0 },
    ];

    pub fn name(self) -> &'static str {
        match self {
            Ability::Fly { .. } => "Fly",
            Ability::Glide { .. } => "Glide",
            Ability::Hammer => "Hammer",
            Ability::Teleport { .. } => "Teleport",
            Ability::DoubleJump { .. } => "Double jump",
            Ability::AirDash { .. } => "Air dash",
            Ability::Homing { .. } => "Homing attack",
            Ability::Stomp { .. } => "Stomp",
            Ability::Shoot { .. } => "Throw",
            Ability::DropDash { .. } => "Drop dash",
            Ability::PeelOut { .. } => "Peel out",
            Ability::InstaShield => "Insta-shield",
            Ability::Bounce { .. } => "Bounce",
            Ability::WallJump { .. } => "Wall jump",
            Ability::Boost { .. } => "Boost",
        }
    }

    /// The same kind of move (whatever the setting).
    pub fn same_kind(self, other: Ability) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }

    /// The next kind of move (or the one before), with its usual setting.
    pub fn cycle(self, dir: i32) -> Ability {
        let i = Self::ALL.iter().position(|a| a.same_kind(self)).unwrap_or(0) as i32;
        Self::ALL[(i + dir).rem_euclid(Self::ALL.len() as i32) as usize]
    }

    /// Its setting as a name, a value, its range and how it reads.
    pub fn setting(self) -> Option<Setting> {
        let s = |name, value, lo, hi, unit| Some(Setting { name, value, lo, hi, unit });
        match self {
            Ability::Fly { seconds } => s("Time", seconds, 1.0, 20.0, "s"),
            Ability::Teleport { distance } => s("Distance", distance, 24.0, 160.0, "px"),
            Ability::DoubleJump { power } => s("Height", power, 0.3, 1.5, "×"),
            Ability::AirDash { speed } => s("Speed", speed, 4.0, 16.0, ""),
            Ability::Homing { range } => s("Reach", range, 60.0, 320.0, "px"),
            Ability::Stomp { speed } => s("Speed", speed, 6.0, 16.0, ""),
            Ability::Shoot { speed } => s("Speed", speed, 4.0, 16.0, ""),
            Ability::Glide { climb } => s("Climb walls", if climb { 1.0 } else { 0.0 }, 0.0, 1.0, "toggle"),
            Ability::DropDash { speed } => s("Speed", speed, 6.0, 12.0, ""),
            Ability::PeelOut { speed } => s("Speed", speed, 8.0, 16.0, ""),
            Ability::Bounce { height } => s("Height", height, 0.6, 1.6, "×"),
            Ability::WallJump { power } => s("Power", power, 0.6, 1.4, "×"),
            Ability::Boost { seconds } => s("Time", seconds, 1.0, 8.0, "s"),
            Ability::Hammer | Ability::InstaShield => None,
        }
    }

    /// With its setting changed to `v` (kept in its range).
    pub fn with(self, v: f32) -> Ability {
        let Some(st) = self.setting() else { return self };
        let v = v.clamp(st.lo, st.hi);
        match self {
            Ability::Fly { .. } => Ability::Fly { seconds: v },
            Ability::Teleport { .. } => Ability::Teleport { distance: v },
            Ability::DoubleJump { .. } => Ability::DoubleJump { power: v },
            Ability::AirDash { .. } => Ability::AirDash { speed: v },
            Ability::Homing { .. } => Ability::Homing { range: v },
            Ability::Stomp { .. } => Ability::Stomp { speed: v },
            Ability::Shoot { .. } => Ability::Shoot { speed: v },
            Ability::Glide { .. } => Ability::Glide { climb: v >= 0.5 },
            Ability::DropDash { .. } => Ability::DropDash { speed: v },
            Ability::PeelOut { .. } => Ability::PeelOut { speed: v },
            Ability::Bounce { .. } => Ability::Bounce { height: v },
            Ability::WallJump { .. } => Ability::WallJump { power: v },
            Ability::Boost { .. } => Ability::Boost { seconds: v },
            Ability::Hammer | Ability::InstaShield => self,
        }
    }

    /// The animations a sheet can have for it (name, what it is).
    pub fn animations(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Ability::Fly { .. } => &[("fly", "Flying (Space in the air)"), ("fly_tired", "Flying, tired out")],
            Ability::Glide { climb: false } => &[
                ("glide", "Gliding (Space in the air)"),
                ("slide", "Sliding along the ground after a glide"),
                ("drop", "Dropping out of a glide"),
            ],
            Ability::Glide { climb: true } => &[
                ("glide", "Gliding (Space in the air)"),
                ("slide", "Sliding along the ground after a glide"),
                ("drop", "Dropping out of a glide"),
                ("climb", "Climbing a wall"),
                ("climb_up", "Climbing over the top of a wall"),
            ],
            Ability::Hammer => &[("hammer", "Hammer swing on the ground"), ("hammer_air", "Hammer swing in the air")],
            Ability::Teleport { .. } => &[("warp", "Teleporting")],
            Ability::DoubleJump { .. } => &[("double_jump", "The second jump")],
            Ability::AirDash { .. } | Ability::Homing { .. } => &[("air_dash", "Dashing through the air")],
            Ability::Stomp { .. } => &[("stomp", "Stomping down")],
            Ability::Shoot { .. } => &[("throw", "Throwing")],
            Ability::DropDash { .. } => &[("drop_dash", "Charging a drop dash")],
            Ability::PeelOut { .. } => &[("peel_out", "Revving a peel out (legs a blur)")],
            Ability::InstaShield => &[("insta_shield", "The insta-shield's flash")],
            Ability::Bounce { .. } => &[("bounce", "Bouncing down")],
            Ability::WallJump { .. } => &[("wall_jump", "Kicking off a wall")],
            Ability::Boost { .. } => &[("boost", "Boosting")],
        }
    }

    /// Whether it can start on the ground (with the action button).
    pub fn on_ground(self) -> bool {
        matches!(
            self,
            Ability::Hammer
                | Ability::Shoot { .. }
                | Ability::Teleport { .. }
                | Ability::PeelOut { .. }
                | Ability::Boost { .. }
        )
    }

    /// The one button it works on, if it cannot be moved: what is held
    /// down (a peel out, a boost) is the action button, a drop dash is
    /// jump held.
    pub fn fixed_button(self) -> Option<Button> {
        match self {
            Ability::PeelOut { .. } | Ability::Boost { .. } => Some(Button::Action),
            Ability::DropDash { .. } => Some(Button::JumpAgain),
            _ => None,
        }
    }

    /// The button it is on unless chosen otherwise.
    pub fn usual_button(self) -> Button {
        match self {
            Ability::Hammer
            | Ability::Shoot { .. }
            | Ability::Stomp { .. }
            | Ability::Bounce { .. }
            | Ability::PeelOut { .. }
            | Ability::Boost { .. } => Button::Action,
            _ => Button::JumpAgain,
        }
    }
}

/// A move's setting, for the editor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setting {
    pub name: &'static str,
    pub value: f32,
    pub lo: f32,
    pub hi: f32,
    /// "toggle" for an on/off setting.
    pub unit: &'static str,
}

/// A move on a button.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    #[serde(flatten)]
    pub ability: Ability,
    #[serde(default)]
    pub on: Button,
}

impl Block {
    pub fn new(ability: Ability) -> Block {
        Block { ability, on: ability.usual_button() }
    }
}

/// How a character moves, as multiples of the guide's (1: as Sonic).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Physics {
    /// Top running speed.
    pub speed: f32,
    /// How quickly they get up to it.
    pub accel: f32,
    /// How high they jump.
    pub jump: f32,
    /// How heavily they fall.
    pub gravity: f32,
    /// How much they steer in the air.
    pub air: f32,
}

impl Default for Physics {
    fn default() -> Self {
        Physics { speed: 1.0, accel: 1.0, jump: 1.0, gravity: 1.0, air: 1.0 }
    }
}

impl Physics {
    /// Each multiple, its name and range (for the editor).
    pub const FIELDS: [(&'static str, f32, f32); 5] = [
        ("Top speed", 0.5, 2.0),
        ("Acceleration", 0.5, 3.0),
        ("Jump", 0.6, 1.6),
        ("Gravity", 0.4, 1.8),
        ("Air control", 0.3, 3.0),
    ];

    pub fn get(&self, k: usize) -> f32 {
        [self.speed, self.accel, self.jump, self.gravity, self.air][k]
    }

    pub fn set(&mut self, k: usize, v: f32) {
        let (_, lo, hi) = Self::FIELDS[k];
        let v = v.clamp(lo, hi);
        match k {
            0 => self.speed = v,
            1 => self.accel = v,
            2 => self.jump = v,
            3 => self.gravity = v,
            _ => self.air = v,
        }
    }
}

/// The most moves a character can have.
pub const MOST_BLOCKS: usize = 8;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kit {
    pub physics: Physics,
    /// Rolling (↓ on the run) and the spin dash (↓ + jump standing).
    pub roll: bool,
    pub spin_dash: bool,
    /// Their moves, tried in order on a press (jump again: the first one
    /// not used yet this jump).
    pub blocks: Vec<Block>,
    /// Rings close by fly to them.
    pub magnet: bool,
    /// They never run out of air underwater.
    pub breathe: bool,
}

impl Default for Kit {
    fn default() -> Self {
        Kit { physics: Physics::default(), roll: true, spin_dash: true, blocks: vec![], magnet: false, breathe: false }
    }
}

impl Kit {
    /// The built-in characters' kits.
    pub fn of(c: Character) -> Kit {
        let one = |ability| vec![Block::new(ability)];
        match c.base() {
            Character::Sonic => Kit::default(),
            Character::Tails => Kit { blocks: one(Ability::Fly { seconds: 8.0 }), ..Kit::default() },
            Character::Knuckles => Kit {
                // He jumps lower: 6 instead of 6.5.
                physics: Physics { jump: 6.0 / 6.5, ..Physics::default() },
                blocks: one(Ability::Glide { climb: true }),
                ..Kit::default()
            },
            Character::Amy => Kit {
                blocks: vec![
                    Block { ability: Ability::Hammer, on: Button::Action },
                    Block { ability: Ability::Hammer, on: Button::JumpAgain },
                ],
                ..Kit::default()
            },
            Character::Exe => Kit { blocks: one(Ability::Teleport { distance: 56.0 }), ..Kit::default() },
            Character::Custom(_) => Kit::default(),
        }
    }

    /// The first move of this kind it has.
    pub fn find(&self, kind: Ability) -> Option<Ability> {
        self.blocks.iter().map(|b| b.ability).find(|a| a.same_kind(kind))
    }

    pub fn has(&self, kind: Ability) -> bool {
        self.find(kind).is_some()
    }

    /// Kept within bounds: settings in range, at most [`MOST_BLOCKS`].
    pub fn sanitized(mut self) -> Kit {
        for k in 0..Physics::FIELDS.len() {
            let v = self.physics.get(k);
            self.physics.set(k, if v.is_finite() { v } else { 1.0 });
        }
        self.blocks.truncate(MOST_BLOCKS);
        for b in &mut self.blocks {
            b.on = b.ability.fixed_button().unwrap_or(b.on);
            if let Some(st) = b.ability.setting() {
                b.ability = b.ability.with(if st.value.is_finite() { st.value } else { st.lo });
            }
        }
        self
    }

    /// What it does, in a line (for the library and the HUD).
    pub fn about(&self) -> String {
        // The moves by the button they are on, in order.
        let mut parts: Vec<String> = [(Button::JumpAgain, "Space in the air"), (Button::Action, "X")]
            .into_iter()
            .filter_map(|(on, key)| {
                let names: Vec<String> =
                    self.blocks.iter().filter(|b| b.on == on).map(|b| b.ability.name().to_lowercase()).collect();
                (!names.is_empty()).then(|| format!("{key}: {}", names.join(", ")))
            })
            .collect();
        if self.spin_dash {
            parts.push("↓ + Space: spin dash".into());
        }
        if self.magnet {
            parts.push("rings fly to them".into());
        }
        if self.breathe {
            parts.push("breathes underwater".into());
        }
        if parts.is_empty() {
            "Runs and jumps".into()
        } else {
            parts.join("  ·  ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_characters_are_kits() {
        assert_eq!(Kit::of(Character::Sonic), Kit::default());
        assert!(Kit::of(Character::Tails).has(Ability::Fly { seconds: 0.0 }));
        let k = Kit::of(Character::Knuckles);
        assert!(k.find(Ability::Glide { climb: false }) == Some(Ability::Glide { climb: true }));
        assert!((k.physics.jump * 6.5 - 6.0).abs() < 1e-4, "Knuckles jumps lower");
        let a = Kit::of(Character::Amy);
        assert!(a.has(Ability::Hammer) && a.blocks.len() == 2);
        assert!(Kit::of(Character::Exe).has(Ability::Teleport { distance: 0.0 }));
    }

    #[test]
    fn kits_save_and_come_back() {
        let k = Kit {
            physics: Physics { speed: 1.4, ..Physics::default() },
            blocks: vec![
                Block::new(Ability::DoubleJump { power: 0.7 }),
                Block { ability: Ability::Shoot { speed: 8.0 }, on: Button::Action },
            ],
            ..Kit::default()
        };
        let json = serde_json::to_string(&k).unwrap();
        assert!(json.contains("\"move\":\"double_jump\"") && json.contains("\"on\":\"action\""), "{json}");
        assert_eq!(serde_json::from_str::<Kit>(&json).unwrap(), k);
        // Missing fields come back as usual.
        let k: Kit = serde_json::from_str(r#"{"blocks":[{"move":"hammer"}]}"#).unwrap();
        assert_eq!(k.blocks, vec![Block { ability: Ability::Hammer, on: Button::JumpAgain }]);
        assert_eq!(k.physics, Physics::default());
    }

    #[test]
    fn settings_stay_in_range() {
        let a = Ability::DoubleJump { power: 0.8 }.with(9.0);
        assert_eq!(a, Ability::DoubleJump { power: 1.5 });
        assert_eq!(Ability::Glide { climb: true }.with(0.2), Ability::Glide { climb: false });
        assert_eq!(Ability::Hammer.cycle(1), Ability::Teleport { distance: 56.0 });
        assert!(Ability::Teleport { distance: 1.0 }.cycle(1).same_kind(Ability::DoubleJump { power: 0.0 }));
        let mut k = Kit { blocks: vec![Block::new(Ability::Hammer); 9], ..Kit::default() };
        k.physics.speed = f32::NAN;
        k.physics.jump = 9.0;
        let k = k.sanitized();
        assert_eq!(k.blocks.len(), MOST_BLOCKS);
        assert!(k.physics.speed == 1.0 && k.physics.jump == 1.6);
        // What is held down stays on its button.
        let k = Kit {
            blocks: vec![
                Block { ability: Ability::Boost { seconds: 3.0 }, on: Button::JumpAgain },
                Block { ability: Ability::DropDash { speed: 8.0 }, on: Button::Action },
                Block { ability: Ability::Bounce { height: 1.0 }, on: Button::JumpAgain },
            ],
            ..Kit::default()
        }
        .sanitized();
        let on: Vec<Button> = k.blocks.iter().map(|b| b.on).collect();
        assert_eq!(on, [Button::Action, Button::JumpAgain, Button::JumpAgain]);
    }

    #[test]
    fn the_newer_moves_and_the_passives_are_kept() {
        let k = Kit {
            blocks: vec![Block::new(Ability::InstaShield), Block::new(Ability::WallJump { power: 1.2 })],
            magnet: true,
            breathe: true,
            ..Kit::default()
        };
        let json = serde_json::to_string(&k).unwrap();
        assert!(json.contains("\"move\":\"insta_shield\"") && json.contains("\"magnet\":true"), "{json}");
        assert_eq!(serde_json::from_str::<Kit>(&json).unwrap(), k);
        assert!(k.about().contains("rings fly to them") && k.about().contains("insta-shield"));
        // Every move has a name, its animations and, but for two, a setting.
        for a in Ability::ALL {
            assert!(!a.name().is_empty() && !a.animations().is_empty());
            assert_eq!(a.setting().is_none(), matches!(a, Ability::Hammer | Ability::InstaShield));
        }
    }
}
