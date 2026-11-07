//! Live switches for debugging a Bedrock pack in the game, read from a file that is reloaded
//! while the game runs: edit, save, and the next frame draws with it. Nothing to compile.
//!
//! The file is `bedrock-debug.json` in the game folder (`HORIZON_BEDROCK_DEBUG_FILE` names
//! another). With `HORIZON_BEDROCK_DEBUG` set and no file, a template is written there.
//! The `HORIZON_BEDROCK_DEBUG`, `_DUMP`, `_DUMP_VARS` and `_HIDE` variables still work and add
//! to the file's lists.
//!
//! ```json
//! {
//!   "debug": true,
//!   "hide": ["crv"],
//!   "dump": ["spider", "enderman"],
//!   "dump_vars": ["habcyx", "acloko"],
//!   "dump_bones": ["rightarm", "rightitem"],
//!   "held_anchor": "auto",
//!   "trace_rider": false,
//!   "materials": { "oreville_auulis": { "additive": false }, "*": {} },
//!   "queries": { "is_powered": 1 }
//! }
//! ```
//!
//! * `hide`: layers whose texture ends with one of these names are not drawn.
//! * `dump` / `dump_vars`: kinds (part of the id) logged once a second, and the `v.` variables
//!   logged with them. `dump_bones`: bones (part of the name) whose pose the dump lists.
//! * `first_person_off`, `fp_offset`, `fp_rotate`, `fp_scale`: the pack's attachable in the first-person hand (shift in blocks, turn in degrees x,y,z, scale).
//! * `held_anchor`: `"auto"`, `"pivot"` or `"frame"`, how held items are put on the hand.
//! * `trace_rider`: while riding, writes `rider-trace.log` (the logs folder): `rider-trace-frame` (every
//!   frame), `rider-trace-tick` and `rider-trace-pose` (the mount's and the rider's bones, as
//!   `dump_bones` names them, else root/waist/body/head) lines: the mount's and the rider's old/new/drawn heights and yaws, to find a tremor.
//! * `materials`: flags forced on a material by its name (`*` for every one): `true` sets it,
//!   `false` clears it, absent leaves what the pack says.
//! * `queries`: `query.x` answers this number for every mob, whatever the game says.

use horizon_bedrock::material::MaterialFlags;
use serde::Deserialize;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant, SystemTime},
};

/// Flags forced on a material; absent ones keep what the pack's material says.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Patch {
    pub alpha_test: Option<bool>,
    pub blend: Option<bool>,
    pub additive: Option<bool>,
    pub emissive: Option<bool>,
    pub emissive_alpha: Option<bool>,
    pub double_sided: Option<bool>,
    pub uv_anim: Option<bool>,
    pub energy_swirl: Option<bool>,
    pub no_color: Option<bool>,
    pub glint: Option<bool>,
    pub invert_culling: Option<bool>,
    pub no_depth_write: Option<bool>,
    pub no_depth_test: Option<bool>,
    pub multitexture: Option<bool>,
    /// `0` clears the stencil write.
    pub stencil_write: Option<u8>,
    /// `0` clears the stencil test.
    pub stencil_not_equal: Option<u8>,
}

impl Patch {
    pub fn apply(&self, f: &mut MaterialFlags) {
        let set = |dst: &mut bool, v: Option<bool>| {
            if let Some(v) = v {
                *dst = v;
            }
        };
        set(&mut f.alpha_test, self.alpha_test);
        set(&mut f.blend, self.blend);
        set(&mut f.additive, self.additive);
        set(&mut f.emissive, self.emissive);
        set(&mut f.emissive_alpha, self.emissive_alpha);
        set(&mut f.double_sided, self.double_sided);
        set(&mut f.uv_anim, self.uv_anim);
        set(&mut f.energy_swirl, self.energy_swirl);
        set(&mut f.no_color, self.no_color);
        set(&mut f.glint, self.glint);
        set(&mut f.invert_culling, self.invert_culling);
        set(&mut f.no_depth_write, self.no_depth_write);
        set(&mut f.no_depth_test, self.no_depth_test);
        set(&mut f.multitexture, self.multitexture);
        if let Some(v) = self.stencil_write {
            f.stencil_write = v;
        }
        if let Some(v) = self.stencil_not_equal {
            f.stencil_not_equal = v;
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// Free text: ignored.
    #[serde(rename = "_help")]
    pub help: Option<serde_json::Value>,
    pub debug: bool,
    pub hide: Vec<String>,
    pub dump: Vec<String>,
    pub dump_vars: Vec<String>,
    /// Bones (part of the name) whose pose the dump lists, moved or not, instead of the first
    /// thirty moved ones.
    pub dump_bones: Vec<String>,
    /// Where what is held sits on the hand: `"auto"` (default) puts the frame origin on it for a
    /// bound bone with no parent (the vanilla-made models, whose animations add the 24 sixteenths
    /// themselves) and the bone's pivot for one under a parent bone; `"pivot"` and `"frame"`
    /// force one of the two.
    pub held_anchor: String,
    /// `true` leaves the first-person hand on the game's own item models (the pack's attachable
    /// draws it by default).
    pub first_person_off: bool,
    /// Where a pack's attachable sits on the first-person hand, from the game's hand pose: the
    /// shift in blocks, the turn in degrees (x, y, z) and the scale (1 when empty).
    pub fp_offset: Vec<f32>,
    pub fp_rotate: Vec<f32>,
    pub fp_scale: Option<f32>,
    /// Writes `rider-trace.log` (logs folder, every frame and tick) while the player rides: heights, yaws and
    /// the seat of the mount and of the rider's model.
    pub trace_rider: bool,
    pub materials: HashMap<String, Patch>,
    pub queries: HashMap<String, f32>,
}

fn list(var: &str) -> Vec<String> {
    std::env::var(var)
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

impl Settings {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut s: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        s.materials = std::mem::take(&mut s.materials)
            .into_iter()
            .map(|(k, v)| (k.to_ascii_lowercase(), v))
            .collect();
        s.queries = std::mem::take(&mut s.queries)
            .into_iter()
            .map(|(k, v)| (k.trim_start_matches("query.").to_ascii_lowercase(), v))
            .collect();
        Ok(s)
    }

    /// The file's settings with the environment variables' lists added.
    fn with_env(mut self) -> Self {
        self.debug |= std::env::var_os("HORIZON_BEDROCK_DEBUG").is_some();
        self.hide.extend(list("HORIZON_BEDROCK_HIDE"));
        self.dump
            .extend(list("HORIZON_BEDROCK_DUMP").into_iter().map(|k| k.to_ascii_lowercase()));
        self.dump_vars.extend(list("HORIZON_BEDROCK_DUMP_VARS"));
        self.dump_bones.extend(list("HORIZON_BEDROCK_DUMP_BONES"));
        for k in &mut self.dump {
            k.make_ascii_lowercase();
        }
        self
    }

    /// Whether a held item's bound bone puts the frame origin on the hand (`true`) or its own
    /// pivot (`false`); `has_parent` is whether the bone sits under another bone of its model.
    pub fn held_by_frame(&self, has_parent: bool) -> bool {
        match self.held_anchor.to_ascii_lowercase().as_str() {
            "frame" => true,
            "pivot" => false,
            _ => !has_parent,
        }
    }

    /// The first-person item's own matrix on the hand: the shift and turn from the file, then
    /// `orient` (the item stood upright, `mobs::render::stand_upright`), at the file's scale.
    pub fn first_person_matrix(&self, orient: glam::Mat4) -> glam::Mat4 {
        let at = |v: &[f32], i: usize| v.get(i).copied().unwrap_or(0.0);
        // Where the first-person hand holds a tool, from the game's hand pose: a little to the
        // left and lower than the game's own items, with the tool turned 14 degrees clockwise
        // (as seen from the eye) so it stands straight.
        let offset = if self.fp_offset.is_empty() { &[-0.12, -0.12, 0.0][..] } else { &self.fp_offset[..] };
        let rotate = if self.fp_rotate.is_empty() { &[0.0, 0.0, -6.0][..] } else { &self.fp_rotate[..] };
        glam::Mat4::from_translation(glam::Vec3::new(at(offset, 0), at(offset, 1), at(offset, 2)))
            * glam::Mat4::from_euler(
                glam::EulerRot::YXZ,
                at(rotate, 1).to_radians(),
                at(rotate, 0).to_radians(),
                at(rotate, 2).to_radians(),
            ) * glam::Mat4::from_scale(glam::Vec3::splat(self.fp_scale.unwrap_or(0.8)))
            * orient
    }

    pub fn hides(&self, texture: &str) -> bool {
        self.hide.iter().any(|h| texture.ends_with(h.as_str()))
    }

    pub fn dumps(&self, kind: &str) -> bool {
        let kind = kind.to_ascii_lowercase();
        self.dump
            .iter()
            .any(|k| k == "all" || kind.contains(k.as_str()))
    }

    pub fn flags(&self, mut f: MaterialFlags, material: &str) -> MaterialFlags {
        if self.materials.is_empty() {
            return f;
        }
        if let Some(p) = self.materials.get("*") {
            p.apply(&mut f);
        }
        if let Some(p) = self.materials.get(&material.to_ascii_lowercase()) {
            p.apply(&mut f);
        }
        f
    }
}

const TEMPLATE: &str = r#"{
  "_help": "Salvar recarrega no jogo (1 s). hide: sufixos de textura que nao desenham. dump: ids (parte) logados 1x/s; dump_vars: variaveis v.* logadas junto; dump_bones: ossos (parte do nome) cuja pose entra no dump. held_anchor: 'auto' (osso sem pai: origem do modelo na mao; osso com pai: pivo), 'pivot' ou 'frame'. materials: flags forcadas por nome do material ('*' = todos); true liga, false desliga. queries: query.x fixa em um numero para todo mob.",
  "debug": true,
  "hide": [],
  "dump": [],
  "dump_vars": [],
  "dump_bones": [],
  "held_anchor": "auto",
  "trace_rider": false,
  "materials": {},
  "queries": {}
}
"#;

struct State {
    path: PathBuf,
    checked: Option<Instant>,
    stamp: Option<(SystemTime, u64)>,
    cur: Arc<Settings>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn file_path() -> PathBuf {
    match std::env::var_os("HORIZON_BEDROCK_DEBUG_FILE").filter(|p| !p.is_empty()) {
        Some(p) => PathBuf::from(p),
        None => horizon_client_render::logging::game_home().join("bedrock-debug.json"),
    }
}

fn stamp(path: &Path) -> Option<(SystemTime, u64)> {
    let m = std::fs::metadata(path).ok()?;
    Some((m.modified().ok()?, m.len()))
}

/// The settings now in force; the file is looked at once a second.
pub fn settings() -> Arc<Settings> {
    let mut guard = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    let first = guard.is_none();
    let st = guard.get_or_insert_with(|| State {
        path: file_path(),
        checked: None,
        stamp: None,
        cur: Arc::new(Settings::default().with_env()),
    });
    if st.checked.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
        return st.cur.clone();
    }
    st.checked = Some(Instant::now());
    if first && st.cur.debug {
        log::info!("bedrock: engine build {}", super::bedrock::ENGINE_BUILD);
        if !st.path.exists() && std::fs::write(&st.path, TEMPLATE).is_ok() {
            log::info!("bedrock-debug: template written to {}", st.path.display());
        }
    }
    let now = stamp(&st.path);
    if now == st.stamp && !first {
        return st.cur.clone();
    }
    let had_file = st.stamp.is_some();
    st.stamp = now;
    match now {
        None => {
            if had_file {
                log::info!("bedrock-debug: {} gone, back to the environment", st.path.display());
                st.cur = Arc::new(Settings::default().with_env());
            }
        }
        Some(_) => match std::fs::read_to_string(&st.path)
            .map_err(|e| e.to_string())
            .and_then(|t| Settings::parse(&t))
        {
            Ok(s) => {
                let s = s.with_env();
                log::info!(
                    "bedrock-debug: loaded {}: debug={} hide={:?} dump={:?} dump_vars={:?} held_anchor={:?} materials={:?} queries={:?}",
                    st.path.display(),
                    s.debug,
                    s.hide,
                    s.dump,
                    s.dump_vars,
                    s.held_anchor,
                    s.materials.keys().collect::<Vec<_>>(),
                    s.queries,
                );
                st.cur = Arc::new(s);
            }
            Err(e) => log::warn!(
                "bedrock-debug: {} not applied, keeping the last one: {e}",
                st.path.display()
            ),
        },
    }
    st.cur.clone()
}

/// One line of the `trace_rider` trace, in `rider-trace.log` beside the game's saves (emptied at
/// the first line of a run), stamped with the milliseconds since the first one. Not the game's
/// log: a line every frame would flood it.
pub fn trace(line: &str) {
    use std::io::Write;
    static FILE: Mutex<Option<(std::fs::File, Instant)>> = Mutex::new(None);
    let mut guard = FILE.lock().unwrap_or_else(PoisonError::into_inner);
    if guard.is_none() {
        let logs = horizon_client_render::logging::game_home().join("logs");
        let path = if logs.is_dir() { logs } else { horizon_client_render::logging::game_home() }
            .join("rider-trace.log");
        match std::fs::File::create(&path) {
            Ok(f) => {
                log::info!("bedrock-debug: trace_rider writes {}", path.display());
                *guard = Some((f, Instant::now()));
            }
            Err(_) => return,
        }
    }
    if let Some((f, start)) = guard.as_mut() {
        let _ = writeln!(f, "t={} {line}", start.elapsed().as_millis());
    }
}

static TRACED_MOUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

/// The mob the player rides now (for `trace_rider`'s bone lines).
pub fn set_traced_mount(id: Option<u32>) {
    TRACED_MOUNT.store(id.unwrap_or(u32::MAX), std::sync::atomic::Ordering::Relaxed);
}

pub fn is_traced_mount(id: u32) -> bool {
    TRACED_MOUNT.load(std::sync::atomic::Ordering::Relaxed) == id
}

pub fn debug() -> bool {
    settings().debug
}

/// A material's flags with the file's patches on them.
pub fn flags(materials: &horizon_bedrock::material::Materials, name: &str) -> MaterialFlags {
    settings().flags(materials.flags(name), name)
}

/// What a query is fixed to, if the file says.
pub fn query(name: &str) -> Option<f32> {
    let s = settings();
    if s.queries.is_empty() {
        return None;
    }
    s.queries.get(name).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_material_patch_sets_and_clears_flags() {
        let s = Settings::parse(
            r#"{"materials":{"Oreville_X":{"additive":false,"blend":true,"stencil_write":0},"*":{"emissive":true}}}"#,
        )
        .unwrap();
        let base = MaterialFlags {
            additive: true,
            stencil_write: 2,
            ..Default::default()
        };
        let f = s.flags(base, "oreville_x");
        assert!(!f.additive && f.blend && f.emissive && f.stencil_write == 0);
        let g = s.flags(base, "other");
        assert!(g.additive && g.emissive && g.stencil_write == 2);
    }

    #[test]
    fn a_misspelt_key_is_an_error_not_a_silent_no_op() {
        assert!(Settings::parse(r#"{"materials":{"a":{"additve":true}}}"#).is_err());
        assert!(Settings::parse(r#"{"hid":[]}"#).is_err());
    }

    #[test]
    fn hide_dump_and_queries_read_as_written() {
        let s = Settings::parse(
            r#"{"hide":["crv"],"dump":["Spider"],"queries":{"query.is_powered":1}}"#,
        )
        .unwrap()
        .with_env();
        assert!(s.hides("entity/bedrock/x/crv") && !s.hides("entity/bedrock/x/bkj"));
        assert!(s.dumps("minecraft:cave_spider"));
        assert_eq!(s.queries.get("is_powered"), Some(&1.0));
    }

    #[test]
    fn the_template_parses() {
        assert!(Settings::parse(TEMPLATE).is_ok());
    }
}
