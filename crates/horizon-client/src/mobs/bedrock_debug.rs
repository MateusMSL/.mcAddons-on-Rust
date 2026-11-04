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
//!   "materials": { "oreville_auulis": { "additive": false }, "*": {} },
//!   "queries": { "is_powered": 1 }
//! }
//! ```
//!
//! * `hide`: layers whose texture ends with one of these names are not drawn.
//! * `dump` / `dump_vars`: kinds (part of the id) logged once a second, and the `v.` variables
//!   logged with them.
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
        for k in &mut self.dump {
            k.make_ascii_lowercase();
        }
        self
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
  "_help": "Salvar recarrega no jogo (1 s). hide: sufixos de textura que nao desenham. dump: ids (parte) logados 1x/s; dump_vars: variaveis v.* logadas junto. materials: flags forcadas por nome do material ('*' = todos); true liga, false desliga. queries: query.x fixa em um numero para todo mob.",
  "debug": true,
  "hide": [],
  "dump": [],
  "dump_vars": [],
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
        if !st.path.exists() {
            if std::fs::write(&st.path, TEMPLATE).is_ok() {
                log::info!("bedrock-debug: template written to {}", st.path.display());
            }
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
                    "bedrock-debug: loaded {}: debug={} hide={:?} dump={:?} dump_vars={:?} materials={:?} queries={:?}",
                    st.path.display(),
                    s.debug,
                    s.hide,
                    s.dump,
                    s.dump_vars,
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
