//! Scene files (`.gscene`): every object with its source, pose and velocity.
//! Images are embedded (base64) so a scene can be shared on its own; very
//! large files are referenced by path instead.

use crate::physics::borders::BorderMode;
use crate::physics::object::{Object, Placement, Source};
use crate::physics::{to_phys, to_screen, PhysWorld};
use crate::shapes::Shape;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const VERSION: u32 = 1;
const EMBED_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub struct SceneFile {
    pub version: u32,
    pub gravity: f32,
    pub border: BorderMode,
    pub objects: Vec<SceneObject>,
}

#[derive(Serialize, Deserialize)]
pub struct SceneObject {
    pub source: SceneSource,
    /// Centre in physics metres (y up), so scenes survive window resizes.
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub vx: f32,
    pub vy: f32,
    pub angvel: f32,
    pub w: f32,
    pub h: f32,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneSource {
    File { path: String },
    Embedded { name: String, data: String },
    Shape { shape: Shape, rgb: [u8; 3] },
    Visualizer,
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

impl SceneSource {
    fn from_source(s: &Source) -> Self {
        match s {
            Source::File(path) => {
                let small = std::fs::metadata(path).map(|m| m.len() <= EMBED_LIMIT).unwrap_or(false);
                match std::fs::read(path).ok().filter(|_| small) {
                    Some(bytes) => {
                        SceneSource::Embedded { name: crate::config::file_name_of(path), data: b64().encode(bytes) }
                    }
                    None => SceneSource::File { path: path.clone() },
                }
            }
            Source::Memory { name, data } => {
                SceneSource::Embedded { name: name.clone(), data: b64().encode(data.as_slice()) }
            }
            Source::Shape { shape, rgb } => SceneSource::Shape { shape: *shape, rgb: [rgb.0, rgb.1, rgb.2] },
            Source::Visualizer => SceneSource::Visualizer,
        }
    }

    fn to_source(&self) -> Option<Source> {
        Some(match self {
            SceneSource::File { path } => Source::File(path.clone()),
            SceneSource::Embedded { name, data } => {
                Source::Memory { name: name.clone(), data: Arc::new(b64().decode(data).ok()?) }
            }
            SceneSource::Shape { shape, rgb } => Source::Shape { shape: *shape, rgb: (rgb[0], rgb[1], rgb[2]) },
            SceneSource::Visualizer => Source::Visualizer,
        })
    }
}

pub fn capture(world: &PhysWorld, objects: &[Object]) -> SceneFile {
    let objects = objects
        .iter()
        .map(|o| {
            let p = o.placement(world);
            let (x, y) = to_phys(p.pos_px.0, p.pos_px.1);
            SceneObject {
                source: SceneSource::from_source(&o.source),
                x,
                y,
                angle: p.angle,
                vx: p.linvel.0,
                vy: p.linvel.1,
                angvel: p.angvel,
                w: o.size.x,
                h: o.size.y,
                pinned: o.pinned,
            }
        })
        .collect();
    SceneFile { version: VERSION, gravity: world.gravity.y, border: world.border, objects }
}

pub fn write(path: &std::path::Path, scene: &SceneFile) -> Result<(), String> {
    let json = serde_json::to_vec(scene).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn read(path: &std::path::Path) -> Result<SceneFile, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let scene: SceneFile = serde_json::from_slice(&bytes).map_err(|e| format!("invalid scene: {e}"))?;
    if scene.version > VERSION {
        return Err(format!("scene version {} is newer than supported ({VERSION})", scene.version));
    }
    Ok(scene)
}

/// Spawn every object of `scene`. Returns the objects and how many failed.
pub fn instantiate(scene: &SceneFile, world: &mut PhysWorld) -> (Vec<Object>, usize) {
    let mut out = Vec::with_capacity(scene.objects.len());
    let mut failed = 0;
    for so in &scene.objects {
        let pos = to_screen(so.x, so.y);
        let placement = Placement {
            pos_px: (pos.x, pos.y),
            angle: so.angle,
            linvel: (so.vx, so.vy),
            angvel: so.angvel,
            size_px: Some((so.w.max(4.0), so.h.max(4.0))),
            pinned: so.pinned,
        };
        match so.source.to_source().and_then(|src| Object::load(world, src, placement)) {
            Some(o) => out.push(o),
            None => failed += 1,
        }
    }
    (out, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_json_round_trip() {
        let scene = SceneFile {
            version: VERSION,
            gravity: -1.6,
            border: BorderMode::Portal,
            objects: vec![SceneObject {
                source: SceneSource::Embedded { name: "a.png".into(), data: b64().encode([1u8, 2, 3]) },
                x: 1.0,
                y: 2.0,
                angle: 0.5,
                vx: 0.0,
                vy: 0.0,
                angvel: 0.0,
                w: 10.0,
                h: 20.0,
                pinned: true,
            }],
        };
        let json = serde_json::to_string(&scene).unwrap();
        assert!(json.contains("\"type\":\"embedded\""));
        let back: SceneFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.border, BorderMode::Portal);
        match back.objects[0].source.to_source() {
            Some(Source::Memory { data, .. }) => assert_eq!(*data, vec![1, 2, 3]),
            _ => panic!("expected memory source"),
        }
    }

    #[test]
    fn visualizer_objects_round_trip() {
        let json = serde_json::to_string(&SceneSource::Visualizer).unwrap();
        assert_eq!(json, r#"{"type":"visualizer"}"#);
        let back: SceneSource = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.to_source(), Some(Source::Visualizer)));
    }
}
