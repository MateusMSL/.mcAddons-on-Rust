//! A Bedrock pack as one file tree.
//!
//! [`Pack::open`] takes a folder, a `.mcpack` or a `.mcaddon` (a zip that may hold several
//! packs). The tree it shows is what the game would load:
//!
//! * `__brarchive/` folders are expanded: `a/__brarchive/b/c.brarchive` holds the files of
//!   `a/b/c/`, and `a/__brarchive/sounds.brarchive` those of `a/sounds/`.
//! * The chosen subpack (`subpacks/<folder>/`) is laid over the base, its files winning.
//! * Other subpacks are left out.
//!
//! Files are read on demand; archives are read once, when the pack opens.

use crate::brarchive::Archive;
use crate::manifest::{Manifest, Module, ModuleKind};
use crate::{Error, Result, jsonc};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const ARCHIVE_DIR: &str = "__brarchive";
const SUBPACKS: &str = "subpacks/";

/// Options for [`Pack::open_with`].
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The subpack folder to lay over the base (`"0_e"`); none for the base alone.
    pub subpack: Option<String>,
}

enum Source {
    Disk(PathBuf),
    Zip {
        zip: Arc<Mutex<zip::ZipArchive<File>>>,
        name: String,
    },
    Archive {
        bytes: Arc<Vec<u8>>,
        offset: usize,
        size: usize,
    },
}

/// A pack's files and manifest.
pub struct Pack {
    manifest: Manifest,
    files: BTreeMap<String, Source>,
    subpack: Option<String>,
}

/// The files under a pack root, before layering: path relative to the root, `/`-separated.
type Raw = BTreeMap<String, Source>;

impl Pack {
    /// The pack at `path` (a folder or one `.mcpack`). A `.mcaddon` holding several packs is an
    /// error here: use [`Pack::open_all`].
    pub fn open(path: &Path) -> Result<Pack> {
        Self::open_with(path, &Options::default())
    }

    pub fn open_with(path: &Path, options: &Options) -> Result<Pack> {
        let mut packs = Self::open_all(path, options)?;
        match packs.len() {
            1 => Ok(packs.remove(0)),
            0 => Err(Error::Manifest("no manifest.json".into())),
            n => Err(Error::Manifest(format!(
                "{n} packs in one file: use Pack::open_all"
            ))),
        }
    }

    /// Every pack in `path`: one for a folder or `.mcpack`, one or more for a `.mcaddon`.
    pub fn open_all(path: &Path, options: &Options) -> Result<Vec<Pack>> {
        if path.is_dir() {
            let mut raw = Raw::new();
            walk(path, path, &mut raw)?;
            return Ok(vec![Self::build(raw, options)?]);
        }
        let file = File::open(path)?;
        let zip = Arc::new(Mutex::new(zip::ZipArchive::new(file)?));
        let names: Vec<String> = {
            let z = zip.lock().expect("zip lock");
            z.file_names()
                .filter_map(|n| n.ok().map(|n| n.into_owned()))
                .collect()
        };
        // Pack roots: the folders holding a manifest.json (at most two levels down).
        let mut roots: Vec<String> = names
            .iter()
            .filter(|n| n.ends_with("manifest.json") && n.matches('/').count() <= 2)
            .map(|n| n[..n.len() - "manifest.json".len()].to_owned())
            .collect();
        roots.sort_by_key(|r| r.len());
        // A root inside another is a nested pack's own file, not a pack.
        let mut kept: Vec<String> = Vec::new();
        for r in roots {
            if !kept
                .iter()
                .any(|k| !k.is_empty() && r.starts_with(k.as_str()))
            {
                kept.push(r);
            }
        }
        let mut packs = Vec::new();
        for root in kept {
            let mut raw = Raw::new();
            for name in names
                .iter()
                .filter(|n| n.starts_with(&root) && !n.ends_with('/'))
            {
                raw.insert(
                    name[root.len()..].to_owned(),
                    Source::Zip {
                        zip: zip.clone(),
                        name: name.clone(),
                    },
                );
            }
            packs.push(Self::build(raw, options)?);
        }
        Ok(packs)
    }

    fn build(raw: Raw, options: &Options) -> Result<Pack> {
        let manifest = match raw.get("manifest.json") {
            Some(src) => Manifest::from_json(&jsonc::parse_bytes(&read_source(src)?)?)?,
            None => return Err(Error::Manifest("no manifest.json".into())),
        };
        if let Some(sub) = &options.subpack
            && manifest.subpack(sub).is_none()
        {
            return Err(Error::Manifest(format!("no subpack {sub}")));
        }
        // Layers: the base, then the chosen subpack.
        let subpack_prefix = options.subpack.as_ref().map(|s| format!("{SUBPACKS}{s}/"));
        let mut base = Raw::new();
        let mut over = Raw::new();
        for (path, src) in raw {
            if let Some(prefix) = &subpack_prefix
                && let Some(rest) = path.strip_prefix(prefix.as_str())
            {
                over.insert(rest.to_owned(), src);
            } else if !path.starts_with(SUBPACKS) {
                base.insert(path, src);
            }
        }
        let mut files = BTreeMap::new();
        for layer in [base, over] {
            for (path, src) in expand(layer)? {
                files.insert(path, src);
            }
        }
        Ok(Pack {
            manifest,
            files,
            subpack: options.subpack.clone(),
        })
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The subpack laid over the base, if one was chosen.
    pub fn subpack(&self) -> Option<&str> {
        self.subpack.as_deref()
    }

    /// Modules this crate never reads: behaviour packs and scripts.
    pub fn unsupported_modules(&self) -> Vec<&Module> {
        self.manifest
            .modules
            .iter()
            .filter(|m| matches!(m.kind, ModuleKind::Data | ModuleKind::Script))
            .collect()
    }

    /// Every file path, sorted.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    /// The file paths under `dir/` (`dir` without a trailing slash).
    pub fn paths_in<'a>(&'a self, dir: &str) -> impl Iterator<Item = &'a str> + 'a {
        let prefix = format!("{dir}/");
        self.files
            .range(prefix.clone()..)
            .map(|(k, _)| k.as_str())
            .take_while(move |k| k.starts_with(&prefix))
    }

    pub fn contains(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>> {
        read_source(
            self.files
                .get(path)
                .ok_or_else(|| Error::Missing(path.to_owned()))?,
        )
    }

    pub fn read_string(&self, path: &str) -> Result<String> {
        let bytes = self.read(path)?;
        String::from_utf8(bytes).map_err(|e| Error::Json(format!("{path}: {e}")))
    }

    /// A JSON file, comments and trailing commas allowed.
    pub fn read_json(&self, path: &str) -> Result<Value> {
        jsonc::parse_bytes(&self.read(path)?).map_err(|e| match e {
            Error::Json(m) => Error::Json(format!("{path}: {m}")),
            other => other,
        })
    }
}

fn read_source(src: &Source) -> Result<Vec<u8>> {
    match src {
        Source::Disk(p) => Ok(std::fs::read(p)?),
        Source::Archive {
            bytes,
            offset,
            size,
        } => Ok(bytes[*offset..*offset + *size].to_vec()),
        Source::Zip { zip, name } => {
            let mut z = zip.lock().expect("zip lock");
            let mut file = z.by_name(name)?;
            let mut out = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut out)?;
            Ok(out)
        }
    }
}

fn walk(root: &Path, dir: &Path, out: &mut Raw) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            walk(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            out.insert(rel, Source::Disk(path));
        }
    }
    Ok(())
}

/// Replaces the `.brarchive` files of one layer by the files they hold.
fn expand(layer: Raw) -> Result<Raw> {
    let mut out = Raw::new();
    let mut archives = Vec::new();
    for (path, src) in layer {
        match archive_dir(&path) {
            Some(dest) => archives.push((dest, src)),
            None => {
                out.insert(path, src);
            }
        }
    }
    for (dest, src) in archives {
        let bytes = Arc::new(read_source(&src)?);
        let archive = Archive::parse(&bytes).map_err(|e| match e {
            Error::Archive(m) => Error::Archive(format!("{dest}: {m}")),
            other => other,
        })?;
        // Entries are located by offset in the shared buffer: the archive's data area starts
        // after its index.
        let base = 16 + 256 * archive.entries().len();
        for entry in archive.entries() {
            let path = format!("{dest}{}", entry.name);
            // A pack that ships a loose file beside an empty archive entry (placeholders its
            // tooling leaves in) means the loose file.
            if entry.size == 0 && out.contains_key(&path) {
                continue;
            }
            out.insert(
                path,
                Source::Archive {
                    bytes: bytes.clone(),
                    offset: base + entry.offset as usize,
                    size: entry.size as usize,
                },
            );
        }
    }
    Ok(out)
}

/// Where the files of `a/__brarchive/b/c.brarchive` go: `a/b/c/` (with a trailing slash).
fn archive_dir(path: &str) -> Option<String> {
    let stem = path.strip_suffix(".brarchive")?;
    let (head, tail) = stem.split_once(&format!("{ARCHIVE_DIR}/"))?;
    Some(format!("{head}{tail}/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brarchive;

    const MANIFEST: &str = r#"{"format_version":3,"header":{"name":"n","uuid":"u","version":[1,0,0]},
        "modules":[{"type":"resources","uuid":"m","version":[1,0,0]},{"type":"script","uuid":"s","version":[1,0,0]}],
        "subpacks":[{"folder_name":"0_e","name":"Vanilla","memory_performance_tier":1}]}"#;

    fn fixture(dir: &Path) {
        let w = |rel: &str, bytes: &[u8]| {
            let p = dir.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, bytes).unwrap();
        };
        w("manifest.json", MANIFEST.as_bytes());
        w("textures/a.png", b"base");
        w("subpacks/0_e/textures/a.png", b"sub");
        w("subpacks/0_e/textures/only_sub.png", b"s");
        w(
            "__brarchive/entity.brarchive",
            &brarchive::write(&[("x.json", b"{\"e\":1}")]).unwrap(),
        );
        w(
            "__brarchive/models/entity.brarchive",
            &brarchive::write(&[("g.json", b"{}")]).unwrap(),
        );
        w(
            "__brarchive/sounds.brarchive",
            &brarchive::write(&[("sound_definitions.json", b"{}")]).unwrap(),
        );
        w(
            "subpacks/0_e/__brarchive/entity.brarchive",
            &brarchive::write(&[("x.json", b"{\"e\":2}")]).unwrap(),
        );
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("horizon-bedrock-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn archive_destinations() {
        assert_eq!(
            archive_dir("__brarchive/entity.brarchive").as_deref(),
            Some("entity/")
        );
        assert_eq!(
            archive_dir("__brarchive/models/entity.brarchive").as_deref(),
            Some("models/entity/")
        );
        assert_eq!(
            archive_dir("__brarchive/sounds/oreville/ans.brarchive").as_deref(),
            Some("sounds/oreville/ans/")
        );
        assert_eq!(
            archive_dir("a/__brarchive/b.brarchive").as_deref(),
            Some("a/b/")
        );
        assert_eq!(archive_dir("textures/a.png"), None);
        assert_eq!(archive_dir("loose.brarchive"), None);
    }

    #[test]
    fn folder_pack_without_subpack() {
        let dir = tmp("base");
        fixture(&dir);
        let pack = Pack::open(&dir).unwrap();
        assert_eq!(pack.read("textures/a.png").unwrap(), b"base");
        assert!(!pack.contains("textures/only_sub.png"));
        assert_eq!(pack.read_json("entity/x.json").unwrap()["e"], 1);
        assert!(pack.contains("models/entity/g.json"));
        assert!(pack.contains("sounds/sound_definitions.json"));
        assert!(
            pack.paths()
                .all(|p| !p.starts_with("subpacks/") && !p.contains("__brarchive"))
        );
        assert_eq!(pack.unsupported_modules().len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn subpack_wins_over_base() {
        let dir = tmp("sub");
        fixture(&dir);
        let pack = Pack::open_with(
            &dir,
            &Options {
                subpack: Some("0_e".into()),
            },
        )
        .unwrap();
        assert_eq!(pack.read("textures/a.png").unwrap(), b"sub");
        assert!(pack.contains("textures/only_sub.png"));
        assert_eq!(pack.read_json("entity/x.json").unwrap()["e"], 2);
        assert_eq!(pack.paths_in("textures").count(), 2);
        assert!(
            Pack::open_with(
                &dir,
                &Options {
                    subpack: Some("9_e".into())
                }
            )
            .is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn mcpack_zip_and_mcaddon_with_two_packs() {
        use std::io::Write;
        let dir = tmp("zip");
        let path = dir.join("x.mcaddon");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (name, body) in [
            ("RP/manifest.json", MANIFEST.as_bytes()),
            ("RP/textures/a.png", b"rp"),
            (
                "BP/manifest.json",
                br#"{"header":{"uuid":"b"},"modules":[{"type":"data"}]}"#,
            ),
            ("BP/entities/e.json", b"{}"),
        ] {
            zip.start_file(name, opts).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
        let packs = Pack::open_all(&path, &Options::default()).unwrap();
        assert_eq!(packs.len(), 2);
        let rp = packs
            .iter()
            .find(|p| p.manifest().is_resource_pack())
            .unwrap();
        assert_eq!(rp.read("textures/a.png").unwrap(), b"rp");
        let bp = packs
            .iter()
            .find(|p| !p.manifest().is_resource_pack())
            .unwrap();
        assert_eq!(bp.unsupported_modules().len(), 1);
        assert!(Pack::open(&path).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_manifest() {
        let dir = tmp("none");
        assert!(Pack::open(&dir).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
