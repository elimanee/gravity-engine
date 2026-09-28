//! Scene files (`.gscene`): every object with its source, pose, velocity and
//! properties, the links between them and the water. Images are embedded
//! (base64) so a scene can be shared on its own; very large files are
//! referenced by path instead.

use crate::drawing::Drawing;
use crate::physics::borders::BorderMode;
use crate::physics::links::{Link, LinkKind, LinkSpec};
use crate::physics::object::{Material, Object, Placement, Source};
use crate::physics::zones::Zone;
use crate::physics::{to_phys, to_screen, PhysWorld};
use crate::shapes::Shape;
use base64::Engine;
use rapier2d::prelude::Point;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// 2: drawings, links, object properties and water. 3: motors and zones.
const VERSION: u32 = 3;
const EMBED_LIMIT: u64 = 4 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
pub struct SceneFile {
    pub version: u32,
    pub gravity: f32,
    pub border: BorderMode,
    pub objects: Vec<SceneObject>,
    #[serde(default)]
    pub links: Vec<SceneLink>,
    /// `None`: no water.
    #[serde(default)]
    pub water: Option<SceneWater>,
    #[serde(default)]
    pub zones: Vec<Zone>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Debug)]
pub struct SceneWater {
    pub level: f32,
    pub density: f32,
}

#[derive(Serialize, Deserialize)]
pub struct SceneLink {
    pub kind: LinkKind,
    /// Index into `objects`.
    pub a: usize,
    /// Index into `objects`; `None` for the background.
    pub b: Option<usize>,
    /// Local anchors (the world point for the background).
    pub la: [f32; 2],
    pub lb: [f32; 2],
    pub length: f32,
    /// Motor speed (rad/s).
    #[serde(default)]
    pub speed: f32,
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
    #[serde(default)]
    pub material: Material,
    /// kg; missing in version 1 files (derived from the size).
    #[serde(default)]
    pub mass: Option<f32>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SceneSource {
    File { path: String },
    Embedded { name: String, data: String },
    Shape { shape: Shape, rgb: [u8; 3] },
    Visualizer,
    Drawing(Drawing),
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
            Source::Drawing(d) => SceneSource::Drawing((**d).clone()),
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
            SceneSource::Drawing(d) => Source::Drawing(Arc::new(d.clone())),
        })
    }
}

pub fn capture(
    world: &PhysWorld,
    objects: &[Object],
    links: &[Link],
    zones: &[Zone],
    water: Option<SceneWater>,
) -> SceneFile {
    let index: HashMap<_, _> = objects.iter().enumerate().map(|(i, o)| (o.body, i)).collect();
    let links = links
        .iter()
        .filter_map(|l| {
            Some(SceneLink {
                kind: l.kind,
                a: *index.get(&l.a)?,
                b: match l.b {
                    Some(b) => Some(*index.get(&b)?),
                    None => None,
                },
                la: [l.la.x, l.la.y],
                lb: [l.lb.x, l.lb.y],
                length: l.length,
                speed: l.speed,
            })
        })
        .collect();
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
                material: o.material,
                mass: Some(o.mass),
            }
        })
        .collect();
    SceneFile {
        version: VERSION,
        gravity: world.gravity.y,
        border: world.border,
        objects,
        links,
        water,
        zones: zones.to_vec(),
    }
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

/// Spawn every object and link of `scene`. Returns them and how many objects failed.
pub fn instantiate(scene: &SceneFile, world: &mut PhysWorld) -> (Vec<Object>, Vec<Link>, usize) {
    let mut out = Vec::with_capacity(scene.objects.len());
    let mut handles = Vec::with_capacity(scene.objects.len());
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
            material: so.material,
            mass: so.mass,
        };
        match so.source.to_source().and_then(|src| Object::load(world, src, placement)) {
            Some(o) => {
                handles.push(Some(o.body));
                out.push(o);
            }
            None => {
                handles.push(None);
                failed += 1;
            }
        }
    }
    let body = |i: usize| handles.get(i).copied().flatten();
    let links = scene
        .links
        .iter()
        .filter_map(|l| {
            let a = body(l.a)?;
            let b = match l.b {
                Some(i) => Some(body(i)?),
                None => None,
            };
            let length = if l.length.is_finite() { l.length.max(0.05) } else { 1.0 };
            let speed = if l.speed.is_finite() { l.speed.clamp(-50.0, 50.0) } else { 0.0 };
            let (la, lb) = (Point::new(l.la[0], l.la[1]), Point::new(l.lb[0], l.lb[1]));
            Some(Link::restore(world, LinkSpec { kind: l.kind, a, b, la, lb, length, speed }))
        })
        .collect();
    (out, links, failed)
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
                material: Material::ICE,
                mass: Some(3.0),
            }],
            links: vec![SceneLink {
                kind: LinkKind::Motor,
                a: 0,
                b: None,
                la: [0.0, 0.0],
                lb: [1.0, 5.0],
                length: 2.0,
                speed: 3.0,
            }],
            water: Some(SceneWater { level: 0.4, density: 2.0 }),
            zones: vec![Zone {
                kind: crate::physics::zones::ZoneKind::Wind,
                min: [0.0, 0.0],
                max: [2.0, 1.0],
                angle: 1.0,
                strength: 12.0,
                pair: None,
            }],
        };
        let json = serde_json::to_string(&scene).unwrap();
        assert!(json.contains("\"type\":\"embedded\""));
        let back: SceneFile = serde_json::from_str(&json).unwrap();
        assert_eq!(back.border, BorderMode::Portal);
        assert_eq!(back.objects[0].material, Material::ICE);
        assert_eq!((back.links[0].kind, back.links[0].speed), (LinkKind::Motor, 3.0));
        assert_eq!(back.zones[0].strength, 12.0);
        assert_eq!(back.water, Some(SceneWater { level: 0.4, density: 2.0 }));
        match back.objects[0].source.to_source() {
            Some(Source::Memory { data, .. }) => assert_eq!(*data, vec![1, 2, 3]),
            _ => panic!("expected memory source"),
        }
    }

    #[test]
    fn version_1_files_still_load() {
        let json = r#"{"version":1,"gravity":-9.8,"border":"Walls","objects":[{"source":{"type":"shape","shape":"Box","rgb":[1,2,3]},"x":1,"y":2,"angle":0,"vx":0,"vy":0,"angvel":0,"w":40,"h":40}]}"#;
        let scene: SceneFile = serde_json::from_str(json).unwrap();
        assert!(scene.links.is_empty() && scene.water.is_none());
        assert_eq!(scene.objects[0].material, Material::default());
        assert_eq!(scene.objects[0].mass, None);
    }

    #[test]
    fn drawings_round_trip() {
        let d = Drawing {
            points: vec![[-0.5, 0.0], [0.5, 0.1]],
            closed: false,
            thickness: 0.1,
            aspect: 0.2,
            rgb: [9, 8, 7],
        };
        let json = serde_json::to_string(&SceneSource::Drawing(d.clone())).unwrap();
        assert!(json.starts_with(r#"{"type":"drawing""#));
        let back: SceneSource = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.to_source(), Some(Source::Drawing(x)) if *x == d));
    }

    #[test]
    fn visualizer_objects_round_trip() {
        let json = serde_json::to_string(&SceneSource::Visualizer).unwrap();
        assert_eq!(json, r#"{"type":"visualizer"}"#);
        let back: SceneSource = serde_json::from_str(&json).unwrap();
        assert!(matches!(back.to_source(), Some(Source::Visualizer)));
    }
}
