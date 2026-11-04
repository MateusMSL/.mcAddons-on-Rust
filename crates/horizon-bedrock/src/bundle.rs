//! Everything a converted pack holds, loaded for the renderer: the folder
//! [`crate::convert::to_java`] wrote (or any folder with `assets/horizon_bedrock/` in it, such as the
//! game's merged resource root).

use crate::animation::Library;
use crate::entity::{ClientEntity, RenderController};
use crate::geometry::{self, Geometry};
use crate::material::Materials;
use crate::particle::Effect;
use crate::sounds::{Definitions, EventSounds};
use crate::{convert::DATA_DIR, jsonc};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// A pack's visual description.
#[derive(Default)]
pub struct Bundle {
    /// Client entities by identifier (`minecraft:zombie`), lower case.
    pub entities: HashMap<String, ClientEntity>,
    /// Attachables by the item they dress, lower case.
    pub attachables: HashMap<String, ClientEntity>,
    pub library: Library,
    pub render_controllers: HashMap<String, RenderController>,
    pub geometries: HashMap<String, Geometry>,
    pub materials: Materials,
    pub effects: HashMap<String, Arc<Effect>>,
    pub sound_definitions: Definitions,
    pub sound_events: EventSounds,
    /// The packs' options.
    pub settings: Vec<crate::settings::Def>,
    /// Keys the packs carry that nothing reads (see [`crate::audit`]).
    pub ignored: crate::audit::Report,
}

/// A PNG's size and which pixels are not clear.
fn read_sprite(path: &Path) -> Option<geometry::Sprite> {
    let mut decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(path).ok()?));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let channels = info.color_type.samples();
    let alpha_at = |i: usize| -> bool {
        match info.color_type {
            png::ColorType::Rgba => buf[i * 4 + 3] > 8,
            png::ColorType::GrayscaleAlpha => buf[i * 2 + 1] > 8,
            _ => true,
        }
    };
    let _ = channels;
    Some(geometry::Sprite {
        size: [w, h],
        opaque: (0..(w * h) as usize).map(alpha_at).collect(),
    })
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push(p);
        }
    }
}

impl Bundle {
    /// What the game itself gives every pack: the vanilla animations, render controllers and
    /// geometries a pack refers to without defining them. A pack's own files, read after, win.
    pub fn add_builtins(&mut self) {
        let b = self;
        if let Ok(v) = jsonc::parse_bytes(include_bytes!("builtin_animations.json")) {
            b.library.add_file(&v);
        }
        if let Ok(v) = jsonc::parse_bytes(include_bytes!("builtin_geometries.json"))
            && let Ok(list) = geometry::parse_file(&v)
        {
            for g in list {
                b.geometries.insert(g.id.to_ascii_lowercase(), g);
            }
        }
        if let Ok(v) = jsonc::parse_bytes(include_bytes!("builtin_render_controllers.json")) {
            RenderController::add_file(&v, &mut b.render_controllers);
        }
    }

    /// Reads `<root>/assets/horizon_bedrock/`. A root without it gives an empty bundle.
    pub fn load(root: &Path) -> Bundle {
        let base = root.join(DATA_DIR);
        let mut all = Vec::new();
        files(&base, &mut all);
        let mut b = Bundle::default();
        b.add_builtins();
        for path in all {
            let Ok(rel) = path.strip_prefix(&base) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            let Ok(bytes) = fs::read(&path) else { continue };
            if rel.ends_with(".material") {
                if let Ok(v) = jsonc::parse_bytes(&bytes) {
                    b.materials.add_file(&v);
                }
                continue;
            }
            if !rel.ends_with(".json") {
                continue;
            }
            let v = match jsonc::parse_bytes(&bytes) {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("Horizon: Bedrock file {rel} does not parse: {e}");
                    continue;
                }
            };
            crate::audit::scan(rel.split('/').next().unwrap_or(""), &v, &mut b.ignored);
            match rel.split('/').next().unwrap_or("") {
                "entity" => {
                    if let Some(d) = ClientEntity::parse(&v) {
                        b.entities.insert(d.id.to_ascii_lowercase(), d);
                    }
                }
                "attachables" => {
                    if let Some(d) = ClientEntity::parse(&v) {
                        b.attachables.insert(d.id.to_ascii_lowercase(), d);
                    }
                }
                "animations" | "animation_controllers" => b.library.add_file(&v),
                "render_controllers" => RenderController::add_file(&v, &mut b.render_controllers),
                "models" => {
                    if let Ok(list) = geometry::parse_file(&v) {
                        for g in list {
                            for variant in g.mesh_variants() {
                                b.geometries.insert(variant.id.clone(), variant);
                            }
                            b.geometries.insert(g.id.to_ascii_lowercase(), g);
                        }
                    }
                }
                "particles" => {
                    if let Some(fx) = Effect::parse(&v) {
                        b.effects.insert(fx.id.to_ascii_lowercase(), Arc::new(fx));
                    }
                }
                "sounds" if rel == "sounds/sound_definitions.json" => {
                    b.sound_definitions.add_file(&v)
                }
                "sounds.json" => b.sound_events.add_file(&v),
                _ if rel == crate::settings::FILE => {
                    b.settings.extend(crate::settings::from_json(&v));
                }
                _ => {}
            }
        }
        b.fit_sprites(root);
        b
    }

    /// Gives each `texture_meshes` sprite its texture's real size and, for `use_pixel_depth`
    /// ones, its silhouette, read from the converted PNGs.
    fn fit_sprites(&mut self, root: &Path) {
        let mut fitted: Vec<Geometry> = Vec::new();
        for def in self.entities.values().chain(self.attachables.values()) {
            for (key, path) in &def.textures {
                for geometry in def.geometry.values() {
                    let Some(base) = self.geometries.get(geometry) else {
                        continue;
                    };
                    if !self
                        .geometries
                        .contains_key(&geometry::mesh_id(geometry, key))
                    {
                        continue;
                    }
                    let png = root.join(format!(
                        "assets/minecraft/textures/{}.png",
                        crate::convert::atlas_name(path)
                    ));
                    if let Some(sprite) = read_sprite(&png) {
                        fitted.push(base.mesh_variant(key, Some(&sprite)));
                    }
                }
            }
        }
        for g in fitted {
            self.geometries.insert(g.id.clone(), g);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty() && self.attachables.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_what_to_java_wrote() {
        let src =
            std::env::temp_dir().join(format!("horizon-bedrock-bundle-{}", std::process::id()));
        let _ = fs::remove_dir_all(&src);
        let put = |rel: &str, text: &str| {
            let p = src.join(DATA_DIR).join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, text).unwrap();
        };
        put(
            "entity/z.json",
            r#"{"minecraft:client_entity":{"description":{"identifier":"minecraft:Zombie","geometry":{"default":"geometry.z"}}}}"#,
        );
        put(
            "attachables/a.json",
            r#"{"minecraft:attachable":{"description":{"identifier":"minecraft:stick"}}}"#,
        );
        put("animations/a.json", r#"{"animations":{"animation.x":{}}}"#);
        put(
            "render_controllers/r.json",
            r#"{"render_controllers":{"controller.render.x":{}}}"#,
        );
        put(
            "models/entity/z.geo.json",
            r#"{"format_version":"1.12.0","minecraft:geometry":[{"description":{"identifier":"geometry.Z","texture_width":16,"texture_height":16},"bones":[]}]}"#,
        );
        put(
            "particles/p.json",
            r#"{"particle_effect":{"description":{"identifier":"t:p"},"components":{}}}"#,
        );
        put(
            "materials/m.material",
            r#"{"materials":{"version":"1.0.0","x:entity":{}}}"#,
        );
        put(
            "sounds/sound_definitions.json",
            r#"{"sound_definitions":{"t:s":{"sounds":["sounds/a"]}}}"#,
        );
        put(
            "sounds.json",
            r#"{"entity_sounds":{"entities":{"zombie":{"events":{"step":""}}}}}"#,
        );
        let b = Bundle::load(&src);
        assert!(b.entities.contains_key("minecraft:zombie"));
        assert!(b.attachables.contains_key("minecraft:stick"));
        assert!(b.library.animations.contains_key("animation.x"));
        assert!(b.render_controllers.contains_key("controller.render.x"));
        assert!(b.geometries.contains_key("geometry.z"));
        assert!(b.effects.contains_key("t:p"));
        assert!(b.sound_definitions.get("t:s").is_some());
        assert!(!b.sound_events.is_empty());
        assert!(!b.is_empty());
        assert!(Bundle::load(&src.join("nothing")).is_empty());
        let _ = fs::remove_dir_all(&src);
    }
}
