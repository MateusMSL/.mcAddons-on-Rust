//! A Bedrock resource pack as a Java resource pack folder.
//!
//! [`to_java`] writes what the Java client already knows how to load: the textures the pack's
//! entities, attachables and particles use as PNG under `assets/minecraft/textures/entity/bedrock/`
//! (so the entity atlas picks them up), the sounds as `assets/<namespace>/sounds/` with a
//! `sounds.json`, and the pack's own files, unchanged, under `assets/horizon_bedrock/` for the
//! renderer's Bedrock layer to read ([`crate::entity`], [`crate::animation`], and the rest).
//!
//! Nothing here touches game rules: the output is textures, sounds and the visual description.

use crate::pack::Pack;
use crate::{Error, Result, entity::ClientEntity, particle::Effect};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Where the pack's own files go, relative to the output folder.
pub const DATA_DIR: &str = "assets/horizon_bedrock";
/// Where its textures go, relative to `assets/minecraft/textures/`.
pub const TEXTURE_DIR: &str = "entity/bedrock";
/// The sound namespace of definitions that name none.
pub const DEFAULT_NAMESPACE: &str = "bedrock";

/// What a conversion wrote.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub textures: usize,
    pub sounds: usize,
    pub sound_events: usize,
    pub data_files: usize,
    /// Textures the pack names but does not hold (the game's own).
    pub missing_textures: usize,
    /// Sound files in a format Java cannot play.
    pub skipped_sounds: usize,
    /// Block, item, sky and colour-map textures the pack holds for the game's own, and the
    /// splash lines it adds.
    pub world_textures: usize,
}

/// The data folders copied unchanged.
const DATA_FOLDERS: [&str; 8] = [
    "entity/",
    "attachables/",
    "animations/",
    "animation_controllers/",
    "render_controllers/",
    "models/",
    "particles/",
    "materials/",
];

/// The path of a Bedrock texture (`textures/entity/x`) in the Java pack, as an atlas name
/// (`entity/bedrock/entity/x`).
pub fn atlas_name(texture: &str) -> String {
    let rest = texture.strip_prefix("textures/").unwrap_or(texture);
    format!("{TEXTURE_DIR}/{}", rest.to_ascii_lowercase())
}

/// Bedrock's name for a dye colour, Java's: `silver` is `light_gray`.
fn dye(color: &str) -> &str {
    if color == "silver" {
        "light_gray"
    } else {
        color
    }
}

/// The Java sprite a Bedrock texture the pack does not ship stands for: packs point at vanilla
/// textures (`textures/entity/skeleton/skeleton`, `textures/blocks/stone`, `textures/items/
/// iron_sword`) that the base game provides under names of its own. The answer is a sprite name
/// of the game's atlas (`entity/skeleton/skeleton`, `block/stone`, `item/iron_sword`).
pub fn java_texture(texture: &str) -> Option<String> {
    let rest = texture.strip_prefix("textures/")?.to_ascii_lowercase();
    let rest = rest
        .strip_suffix(".png")
        .or_else(|| rest.strip_suffix(".tga"))
        .unwrap_or(&rest)
        .to_owned();
    if let Some(name) = rest.strip_prefix("blocks/") {
        return Some(java_block(name));
    }
    if let Some(name) = rest.strip_prefix("items/") {
        return Some(format!("item/{}", java_item(name)));
    }
    if let Some(armor) = rest.strip_prefix("models/armor/") {
        let (material, layer) = armor.rsplit_once('_').unwrap_or((armor, "1"));
        let folder = if layer == "2" {
            "humanoid_leggings"
        } else {
            "humanoid"
        };
        return Some(match material {
            "elytra" => "entity/equipment/wings/elytra".into(),
            "chain" => format!("entity/equipment/{folder}/chainmail"),
            m => format!("entity/equipment/{folder}/{}", m.trim_end_matches('_')),
        });
    }
    if !rest.starts_with("entity/") {
        return None;
    }
    // Where the game names what Bedrock names otherwise.
    Some(match rest.as_str() {
        "entity/copper_golem/coper_golem_weathered" => {
            "entity/copper_golem/copper_golem_weathered".into()
        }
        "entity/iron_golem/cracked_low" => "entity/iron_golem/iron_golem_crackiness_low".into(),
        "entity/iron_golem/cracked_medium" => {
            "entity/iron_golem/iron_golem_crackiness_medium".into()
        }
        "entity/iron_golem/cracked_high" => "entity/iron_golem/iron_golem_crackiness_high".into(),
        "entity/endercrystal/endercrystal_beam" => "entity/end_crystal/end_crystal_beam".into(),
        "entity/fishhook" => "entity/fishing_hook".into(),
        "entity/shield" => "entity/shield/shield_base_nopattern".into(),
        "entity/skulls/creeper" => "entity/creeper/creeper".into(),
        "entity/skulls/skeleton" => "entity/skeleton/skeleton".into(),
        "entity/skulls/wither_skeleton" => "entity/skeleton/wither_skeleton".into(),
        "entity/skulls/zombie" => "entity/zombie/zombie".into(),
        r if r.starts_with("entity/villager2/levels/level_") => format!(
            "entity/villager/profession_level/{}",
            &r["entity/villager2/levels/level_".len()..]
        ),
        r if r.starts_with("entity/bed/") => {
            format!("entity/bed/{}", dye(&r["entity/bed/".len()..]))
        }
        _ => rest,
    })
}

/// A Bedrock block texture's sprite in Java.
fn java_block(name: &str) -> String {
    let named = |n: &str| format!("block/{n}");
    if let Some(c) = name.strip_prefix("wool_colored_") {
        return named(&format!("{}_wool", dye(c)));
    }
    if let Some(c) = name.strip_prefix("concrete_") {
        return named(&format!("{}_concrete", dye(c)));
    }
    if let Some(c) = name.strip_prefix("hardened_clay_stained_") {
        return named(&format!("{}_terracotta", dye(c)));
    }
    if name.ends_with("_pottery_pattern") {
        return format!("entity/decorated_pot/{name}");
    }
    named(match name {
        "hardened_clay" => "terracotta",
        "cobblestone_mossy" => "mossy_cobblestone",
        "stone_andesite" => "andesite",
        "stone_diorite" => "diorite",
        "stone_granite" => "granite",
        "nether_brick" => "nether_bricks",
        "brick" => "bricks",
        "prismarine_dark" => "dark_prismarine",
        "pumpkin_face_off" => "carved_pumpkin",
        "flower_rose" => "poppy",
        "pale_moss" => "pale_moss_block",
        "deepslate/cobbled_deepslate" => "cobbled_deepslate",
        other => other,
    })
}

/// A Bedrock item texture's name in Java: wood and gold tools are wooden and golden there.
fn java_item(name: &str) -> String {
    for (bedrock, java) in [("wood_", "wooden_"), ("gold_", "golden_")] {
        if let Some(rest) = name.strip_prefix(bedrock)
            && matches!(rest, "sword" | "axe" | "pickaxe" | "shovel" | "hoe")
        {
            return format!("{java}{rest}");
        }
    }
    match name {
        "crossbow_standby" => "crossbow".into(),
        other => other.into(),
    }
}

fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-./".contains(&b))
        && !s.contains("..")
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

/// Textures a pack's entities, attachables and particles name.
pub fn referenced_textures(pack: &Pack) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for p in pack.paths().map(str::to_owned).collect::<Vec<_>>() {
        if !p.ends_with(".json") {
            continue;
        }
        let in_entity = p.starts_with("entity/") || p.starts_with("attachables/");
        let in_particles = p.starts_with("particles/");
        if !in_entity && !in_particles {
            continue;
        }
        let Ok(v) = pack.read_json(&p) else { continue };
        if in_entity {
            if let Some(def) = ClientEntity::parse(&v) {
                out.extend(def.textures.into_values());
            }
        } else if let Some(fx) = Effect::parse(&v)
            && !fx.texture.is_empty()
        {
            out.insert(fx.texture);
        }
    }
    out
}

/// Finds a texture's file (`.png`, `.tga`) and returns it as PNG bytes, no bigger than the
/// game's atlas takes.
fn texture_png(pack: &Pack, texture: &str, lift: bool) -> Option<Vec<u8>> {
    let png = if let Ok(b) = pack.read(&format!("{texture}.png")) {
        b
    } else {
        let tga = pack.read(&format!("{texture}.tga")).ok()?;
        encode_png(&decode_tga(&tga)?)?
    };
    let png = fit_to_atlas(&png).unwrap_or(png);
    // A texture's faint texels (an eye painted at alpha 3) are lifted.
    Some(if lift {
        lift_faint_alpha(&png).unwrap_or(png)
    } else {
        png
    })
}

/// The widest and tallest texture the game's atlas packs (`HudAtlas::pack`): a bigger one is
/// left out of it, and a model wearing it is not drawn.
const ATLAS_MAX_WIDTH: u32 = 1024;
const ATLAS_MAX_HEIGHT: u32 = 512;

/// The side factor (a power of two) that brings a `width` x `height` texture within the atlas.
fn atlas_shrink(width: u32, height: u32) -> u32 {
    let mut factor = 1;
    while width.div_ceil(factor) > ATLAS_MAX_WIDTH || height.div_ceil(factor) > ATLAS_MAX_HEIGHT {
        factor *= 2;
    }
    factor
}

/// A texture past the atlas's size, averaged down by a whole factor (a model's UVs are
/// fractions of its texture, so it still wears the same picture). `None` when it fits.
fn fit_to_atlas(png_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let (w, h) = (reader.info().width, reader.info().height);
    let factor = atlas_shrink(w, h);
    if factor == 1 {
        return None;
    }
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let src = &buf[..info.buffer_size()];
    let rgba = |i: usize| -> [u32; 4] {
        match info.color_type {
            png::ColorType::Rgba => {
                [src[i * 4].into(), src[i * 4 + 1].into(), src[i * 4 + 2].into(), src[i * 4 + 3].into()]
            }
            png::ColorType::Rgb => [src[i * 3].into(), src[i * 3 + 1].into(), src[i * 3 + 2].into(), 255],
            png::ColorType::GrayscaleAlpha => {
                [src[i * 2].into(), src[i * 2].into(), src[i * 2].into(), src[i * 2 + 1].into()]
            }
            _ => [src[i].into(), src[i].into(), src[i].into(), 255],
        }
    };
    let (nw, nh) = (w.div_ceil(factor), h.div_ceil(factor));
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        for x in 0..nw {
            let (mut sum, mut alpha, mut count) = ([0u64; 3], 0u64, 0u64);
            for sy in (y * factor)..((y + 1) * factor).min(h) {
                for sx in (x * factor)..((x + 1) * factor).min(w) {
                    let p = rgba((sy * w + sx) as usize);
                    let a = u64::from(p[3]);
                    for c in 0..3 {
                        sum[c] += u64::from(p[c]) * a;
                    }
                    alpha += a;
                    count += 1;
                }
            }
            if alpha == 0 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                for c in sum {
                    out.push((c / alpha) as u8);
                }
                out.push((alpha / count) as u8);
            }
        }
    }
    encode_png(&Image {
        width: nw,
        height: nh,
        rgba: out,
    })
}

/// What an emissive material lights in a texture: the texels that are neither clear nor opaque,
/// the more the lower their alpha. Drawn at full brightness over the lit model. `None` when the
/// texture has none.
fn glow_png(png_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let pixels = &mut buf[..info.buffer_size()];
    let mut any = false;
    for px in pixels.chunks_exact_mut(4) {
        if (1..255).contains(&px[3]) {
            px[3] = 255 - px[3];
            any = true;
        } else {
            px.copy_from_slice(&[0, 0, 0, 0]);
        }
    }
    any.then(|| {
        encode_png(&Image {
            width: info.width,
            height: info.height,
            rgba: pixels.to_vec(),
        })
    })
    .flatten()
}

/// The texture as an emissive material draws it: its alpha is how much a texel glows, not how
/// see-through it is, so every texel that is not clear is opaque. `None` when the texture has no
/// texel between clear and opaque (it is drawn as it is).
fn solid_png(png_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let pixels = &mut buf[..info.buffer_size()];
    let mut any = false;
    for px in pixels.chunks_exact_mut(4) {
        if (1..255).contains(&px[3]) {
            px[3] = 255;
            any = true;
        }
    }
    any.then(|| {
        encode_png(&Image {
            width: info.width,
            height: info.height,
            rgba: pixels.to_vec(),
        })
    })
    .flatten()
}

/// The highest alpha a texel can have and still count as "faint".
const FAINT_ALPHA: u8 = 8;

/// A cut-out texture whose texels are drawn at an alpha of 1 to 8 (a villager's eye white and
/// pupil are 2 in a pack that wants them shown) has them made solid: Bedrock draws what the
/// pack painted there, while the game's alpha test throws them away. Textures with any real
/// translucency (a glow, a ghost) are left alone, since their faint texels are faint on purpose.
/// `None` when nothing changes.
fn lift_faint_alpha(png_bytes: &[u8]) -> Option<Vec<u8>> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let pixels = &mut buf[..info.buffer_size()];
    let (mut faint, mut solid, mut translucent) = (0usize, 0usize, false);
    for px in pixels.chunks_exact(4) {
        match px[3] {
            1..=FAINT_ALPHA => faint += 1,
            255 => solid += 1,
            a if a > FAINT_ALPHA => translucent = true,
            _ => {}
        }
    }
    // Faint texels that outnumber the solid ones are a filler (a profession's clothes sit on
    // white at alpha 1): the pack means them clear, not an eye to show.
    if faint == 0 || translucent || (solid > 0 && faint > solid) {
        return None;
    }
    for px in pixels.chunks_exact_mut(4) {
        if (1..=FAINT_ALPHA).contains(&px[3]) {
            px[3] = 255;
        }
    }
    encode_png(&Image {
        width: info.width,
        height: info.height,
        rgba: pixels.to_vec(),
    })
}

/// A decoded image: RGBA, top row first.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Targa: uncompressed or run-length, 8 (grey or indexed), 24 or 32 bit.
pub fn decode_tga(b: &[u8]) -> Option<Image> {
    if b.len() < 18 {
        return None;
    }
    let id_len = b[0] as usize;
    let cmap_type = b[1];
    let kind = b[2];
    let cmap_first = u16::from_le_bytes([b[3], b[4]]) as usize;
    let cmap_len = u16::from_le_bytes([b[5], b[6]]) as usize;
    let cmap_bits = b[7] as usize;
    let width = u16::from_le_bytes([b[12], b[13]]) as usize;
    let height = u16::from_le_bytes([b[14], b[15]]) as usize;
    let bits = b[16] as usize;
    let desc = b[17];
    if width == 0 || height == 0 || width * height > 1 << 26 {
        return None;
    }
    let rle = matches!(kind, 9..=11);
    let base = kind & 7;
    if !matches!(base, 1..=3) || bits % 8 != 0 || bits == 0 {
        return None;
    }
    let bpp = bits / 8;
    let mut at = 18 + id_len;
    let mut palette: Vec<[u8; 4]> = Vec::new();
    if cmap_type == 1 {
        let entry = cmap_bits.div_ceil(8);
        let mut cm = vec![[0, 0, 0, 255]; cmap_first + cmap_len];
        for i in 0..cmap_len {
            let e = b.get(at..at + entry)?;
            cm[cmap_first + i] = pixel(e, entry);
            at += entry;
        }
        palette = cm;
    }
    let px = |raw: &[u8]| -> [u8; 4] {
        match base {
            1 => palette
                .get(raw[0] as usize)
                .copied()
                .unwrap_or([0, 0, 0, 255]),
            3 => {
                if bpp == 1 {
                    [raw[0], raw[0], raw[0], 255]
                } else {
                    [raw[0], raw[0], raw[0], raw[1]]
                }
            }
            _ => pixel(raw, bpp),
        }
    };
    let total = width * height;
    let mut pixels: Vec<[u8; 4]> = Vec::with_capacity(total);
    if rle {
        while pixels.len() < total {
            let h = *b.get(at)?;
            at += 1;
            let n = (h & 0x7f) as usize + 1;
            if h & 0x80 != 0 {
                let p = px(b.get(at..at + bpp)?);
                at += bpp;
                pixels.extend(std::iter::repeat_n(p, n));
            } else {
                for _ in 0..n {
                    pixels.push(px(b.get(at..at + bpp)?));
                    at += bpp;
                }
            }
        }
        pixels.truncate(total);
    } else {
        for _ in 0..total {
            pixels.push(px(b.get(at..at + bpp)?));
            at += bpp;
        }
    }
    let top_first = desc & 0x20 != 0;
    let right_first = desc & 0x10 != 0;
    let mut rgba = vec![0u8; total * 4];
    for y in 0..height {
        let src_y = if top_first { y } else { height - 1 - y };
        for x in 0..width {
            let src_x = if right_first { width - 1 - x } else { x };
            rgba[(y * width + x) * 4..][..4].copy_from_slice(&pixels[src_y * width + src_x]);
        }
    }
    Some(Image {
        width: width as u32,
        height: height as u32,
        rgba,
    })
}

/// One pixel of 8 (grey), 16 (5-5-5-1), 24 or 32 bits, from BGR(A).
fn pixel(raw: &[u8], bpp: usize) -> [u8; 4] {
    match bpp {
        1 => [raw[0], raw[0], raw[0], 255],
        2 => {
            let v = u16::from_le_bytes([raw[0], raw[1]]);
            let c = |s: u16| (((v >> s) & 31) as u32 * 255 / 31) as u8;
            [c(10), c(5), c(0), if v & 0x8000 != 0 { 255 } else { 0 }]
        }
        3 => [raw[2], raw[1], raw[0], 255],
        _ => [raw[2], raw[1], raw[0], raw[3]],
    }
}

fn encode_png(img: &Image) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().ok()?;
        w.write_image_data(&img.rgba).ok()?;
    }
    Some(out)
}

fn average(v: Option<&Value>, d: f64) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(d),
        Some(Value::Array(a)) if a.len() >= 2 => {
            (a[0].as_f64().unwrap_or(d) + a[1].as_f64().unwrap_or(d)) / 2.0
        }
        _ => d,
    }
}

/// `sound_definitions` as Java sounds: files copied, one `sounds.json` per namespace.
fn convert_sounds(pack: &Pack, out: &Path, report: &mut Report) -> Result<()> {
    let Ok(root) = pack.read_json("sounds/sound_definitions.json") else {
        return Ok(());
    };
    let Some(defs) = root.get("sound_definitions").and_then(Value::as_object) else {
        return Ok(());
    };
    let mut by_ns: BTreeMap<String, serde_json::Map<String, Value>> = BTreeMap::new();
    let mut copied: BTreeSet<String> = BTreeSet::new();
    for (key, def) in defs {
        let key = key.to_ascii_lowercase();
        let (ns, event) = key
            .split_once(':')
            .unwrap_or((DEFAULT_NAMESPACE, key.as_str()));
        if !valid_name(ns) || !valid_name(event) {
            continue;
        }
        let Some(list) = def.get("sounds").and_then(Value::as_array) else {
            continue;
        };
        let mut sounds = Vec::new();
        for s in list {
            let (name, volume, pitch, weight, stream) = match s {
                Value::String(n) => (n.as_str(), 1.0, 1.0, 1, false),
                Value::Object(o) => (
                    o.get("name").and_then(Value::as_str).unwrap_or(""),
                    average(o.get("volume"), 1.0),
                    average(o.get("pitch"), 1.0),
                    o.get("weight").and_then(Value::as_u64).unwrap_or(1),
                    o.get("stream").and_then(Value::as_bool).unwrap_or(false),
                ),
                _ => continue,
            };
            let rel = name
                .strip_prefix("sounds/")
                .unwrap_or(name)
                .to_ascii_lowercase();
            if !valid_name(&rel) {
                continue;
            }
            let java_path = out.join(format!("assets/{ns}/sounds/{rel}.ogg"));
            if copied.contains(&java_path.to_string_lossy().into_owned()) || java_path.exists() {
                // already written
            } else if let Ok(bytes) = pack.read(&format!("{name}.ogg")) {
                write(&java_path, &bytes)?;
                copied.insert(java_path.to_string_lossy().into_owned());
                report.sounds += 1;
            } else {
                if pack.contains(&format!("{name}.wav")) || pack.contains(&format!("{name}.fsb")) {
                    report.skipped_sounds += 1;
                }
                continue;
            }
            let mut entry =
                json!({"name": format!("{ns}:{rel}"), "volume": volume, "pitch": pitch});
            if weight > 1 {
                entry["weight"] = json!(weight);
            }
            if stream {
                entry["stream"] = json!(true);
            }
            sounds.push(entry);
        }
        if sounds.is_empty() {
            continue;
        }
        let mut event_json = json!({"sounds": sounds});
        if let Some(c) = def.get("category").and_then(Value::as_str) {
            event_json["category"] = json!(c);
        }
        by_ns
            .entry(ns.to_owned())
            .or_default()
            .insert(event.to_owned(), event_json);
        report.sound_events += 1;
    }
    for (ns, events) in by_ns {
        write(
            &out.join(format!("assets/{ns}/sounds.json")),
            serde_json::to_string(&Value::Object(events))
                .map_err(|e| Error::Json(e.to_string()))?
                .as_bytes(),
        )?;
    }
    Ok(())
}

/// Writes the Java form of `pack` into `out` (created; existing files are replaced).
/// `format` is the Java resource pack format `pack.mcmeta` declares.
pub fn to_java(pack: &Pack, out: &Path, format: (i64, i64)) -> Result<Report> {
    let mut report = Report::default();
    fs::create_dir_all(out)?;
    let name = pack.manifest().name.clone();
    let meta = json!({"pack": {
        "description": format!("Bedrock: {name}"),
        "min_format": [format.0, format.1],
        "max_format": [format.0, format.1],
    }});
    write(&out.join("pack.mcmeta"), meta.to_string().as_bytes())?;
    if let Ok(icon) = pack.read("pack_icon.png") {
        write(&out.join("pack.png"), &icon)?;
    }
    // The pack's own files.
    for p in pack.paths().map(str::to_owned).collect::<Vec<_>>() {
        let wanted = DATA_FOLDERS
            .iter()
            .any(|d| p.starts_with(d) && (p.ends_with(".json") || p.ends_with(".material")))
            || p == "sounds.json"
            || p == "sounds/sound_definitions.json";
        if !wanted {
            continue;
        }
        write(
            &out.join(format!("{DATA_DIR}/{}/{p}", namespace(pack))),
            &pack.read(&p)?,
        )?;
        report.data_files += 1;
    }
    // Textures.
    for t in referenced_textures(pack) {
        match texture_png(pack, &t, true) {
            Some(bytes) => {
                let name = atlas_name(&t);
                if !valid_name(&name) {
                    continue;
                }
                write(
                    &out.join(format!("assets/minecraft/textures/{name}.png")),
                    &bytes,
                )?;
                report.textures += 1;
                // The texels an emissive material lights (alpha below opaque), apart.
                if let Some(glow) = texture_png(pack, &t, false).and_then(|b| glow_png(&b)) {
                    write(
                        &out.join(format!("assets/minecraft/textures/{name}_glow.png")),
                        &glow,
                    )?;
                }
                // The same texture with those texels opaque: what an emissive material shows
                // under the glow.
                if let Some(solid) = texture_png(pack, &t, false).and_then(|b| solid_png(&b)) {
                    write(
                        &out.join(format!("assets/minecraft/textures/{name}_solid.png")),
                        &solid,
                    )?;
                }
            }
            None => report.missing_textures += 1,
        }
    }
    convert_world(pack, out, &mut report)?;
    convert_sounds(pack, out, &mut report)?;
    convert_settings(pack, out)?;
    Ok(report)
}

/// What a pack retextures of the game itself: blocks and items by the names Java gives them, the
/// sky, the colour maps, and the splash texts.
fn convert_world(pack: &Pack, out: &Path, report: &mut Report) -> Result<()> {
    for p in pack.paths().map(str::to_owned).collect::<Vec<_>>() {
        let Some(stem) = p.strip_suffix(".png") else {
            continue;
        };
        let name = stem.to_ascii_lowercase();
        let target = if let Some(n) = name
            .strip_prefix("textures/blocks/")
            .or_else(|| name.strip_prefix("textures/items/"))
        {
            // Only the files straight in the folder: the folders under it are the pack's own.
            if n.contains('/') {
                continue;
            }
            let java = java_texture(&name).filter(|j| {
                j.starts_with("block/") || j.starts_with("item/") || j.starts_with("entity/")
            });
            match java {
                Some(j) => j,
                None => continue,
            }
        } else if matches!(
            name.as_str(),
            "textures/environment/sun"
                | "textures/environment/moon_phases"
                | "textures/environment/clouds"
                | "textures/environment/rain"
                | "textures/environment/snow"
                | "textures/colormap/grass"
                | "textures/colormap/foliage"
        ) {
            name.trim_start_matches("textures/").to_owned()
        } else {
            continue;
        };
        if !valid_name(&target) {
            continue;
        }
        write(
            &out.join(format!("assets/minecraft/textures/{target}.png")),
            &pack.read(&p)?,
        )?;
        report.world_textures += 1;
    }
    // `terrain_texture.json` and `item_texture.json` name the file a block or item uses, which a
    // pack may keep under any name: the file they name is the one the game's sprite gets.
    for (file, folder) in [
        ("textures/terrain_texture.json", "block"),
        ("textures/item_texture.json", "item"),
    ] {
        for (key, path) in texture_index(pack, file) {
            let Some(java) = java_texture(&format!(
                "textures/{}s/{}",
                folder,
                key.to_ascii_lowercase()
            )) else {
                continue;
            };
            if !valid_name(&java) || key == "missing" {
                continue;
            }
            let Ok(bytes) = pack.read(&format!("{path}.png")) else {
                continue;
            };
            write(&out.join(format!("assets/minecraft/textures/{java}.png")), &bytes)?;
            report.world_textures += 1;
        }
    }
    // `flipbook_textures.json`: a texture that is a strip of frames. Java's `.png.mcmeta`.
    for book in flipbooks(pack) {
        let Some(java) = java_texture(&book.texture.to_ascii_lowercase()) else {
            continue;
        };
        let target = out.join(format!("assets/minecraft/textures/{java}.png"));
        if !valid_name(&java) || pack.read(&format!("{}.png", book.texture)).is_err() {
            continue;
        }
        write(&target, &pack.read(&format!("{}.png", book.texture))?)?;
        let mut animation = serde_json::Map::new();
        if let Some(ticks) = book.ticks {
            animation.insert("frametime".into(), json!(ticks.round().max(1.0) as i64));
        }
        if !book.frames.is_empty() {
            animation.insert("frames".into(), json!(book.frames));
        }
        animation.insert("interpolate".into(), json!(book.blend));
        write(
            &out.join(format!("assets/minecraft/textures/{java}.png.mcmeta")),
            json!({"animation": animation}).to_string().as_bytes(),
        )?;
        report.world_textures += 1;
    }
    // `splashes.json`: `{"splashes": ["..."]}`, Java's `texts/splashes.txt` one to a line.
    if let Ok(v) = pack.read_json("splashes.json")
        && let Some(list) = v.get("splashes").and_then(Value::as_array)
    {
        let lines: Vec<&str> = list
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.contains('\n'))
            .collect();
        if !lines.is_empty() {
            write(
                &out.join("assets/minecraft/texts/splashes.txt"),
                lines.join("\n").as_bytes(),
            )?;
            report.world_textures += lines.len();
        }
    }
    Ok(())
}

/// A texture that is a strip of frames (`textures/flipbook_textures.json`).
struct Flipbook {
    /// `textures/blocks/water_still`.
    texture: String,
    ticks: Option<f64>,
    frames: Vec<i64>,
    blend: bool,
}

fn flipbooks(pack: &Pack) -> Vec<Flipbook> {
    let Ok(v) = pack.read_json("textures/flipbook_textures.json") else {
        return Vec::new();
    };
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|e| {
            Some(Flipbook {
                texture: e.get("flipbook_texture")?.as_str()?.to_owned(),
                ticks: e.get("ticks_per_frame").and_then(Value::as_f64),
                frames: e
                    .get("frames")
                    .and_then(Value::as_array)
                    .map(|a| a.iter().filter_map(Value::as_i64).collect())
                    .unwrap_or_default(),
                blend: e.get("blend_frames").and_then(Value::as_bool).unwrap_or(true),
            })
        })
        .collect()
}

/// What `terrain_texture.json` / `item_texture.json` map: a name to the one texture file it
/// uses. An entry listing different files is by data value, which Java has no way to say, and
/// is left to the file names.
fn texture_index(pack: &Pack, file: &str) -> Vec<(String, String)> {
    let Ok(v) = pack.read_json(file) else {
        return Vec::new();
    };
    let Some(data) = v.get("texture_data").and_then(Value::as_object) else {
        return Vec::new();
    };
    data.iter()
        .filter_map(|(key, entry)| {
            let paths: Vec<&str> = match entry.get("textures")? {
                Value::String(s) => vec![s.as_str()],
                Value::Array(a) => a
                    .iter()
                    .filter_map(|t| t.as_str().or_else(|| t.get("path")?.as_str()))
                    .collect(),
                _ => return None,
            };
            let first = *paths.first()?;
            paths
                .iter()
                .all(|p| *p == first)
                .then(|| (key.clone(), first.to_owned()))
        })
        .collect()
}

/// The folder under `assets/horizon_bedrock/` that holds one pack's data: its UUID, so that
/// packs stacked in the game (the overlay merges by path, last wins) never replace each other's
/// `materials/entity.material` or `sounds.json`.
pub fn namespace(pack: &Pack) -> String {
    let id: String = pack
        .manifest()
        .uuid
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect();
    if id.is_empty() { "pack".to_owned() } else { id }
}

/// The pack's settings, labels looked up in its language file (`en_US`, else the first one),
/// in the pack's own folder.
fn convert_settings(pack: &Pack, out: &Path) -> Result<()> {
    let defs = crate::settings::of_pack(pack);
    if defs.is_empty() {
        return Ok(());
    }
    write(
        &out.join(format!(
            "{DATA_DIR}/{}/{}",
            namespace(pack),
            crate::settings::FILE
        )),
        crate::settings::to_json(&defs).to_string().as_bytes(),
    )
}

/// Bumped whenever the conversion's output changes, so packs converted by an older engine
/// convert again instead of coming out of the cache.
pub const CONVERTER_REVISION: u64 = 11;

/// A folder name for a converted pack: stable for the same input, different for others.
pub fn cache_name(source: &Path, modified_secs: u64, size: u64) -> PathBuf {
    let stem = source
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let clean: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in source
        .to_string_lossy()
        .bytes()
        .chain(modified_secs.to_le_bytes())
        .chain(size.to_le_bytes())
        .chain(CONVERTER_REVISION.to_le_bytes())
    {
        h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
    }
    PathBuf::from(format!("{clean}-{h:016x}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn vanilla_textures_map_to_the_games_sprites() {
        let j = |t: &str| java_texture(t).unwrap();
        assert_eq!(j("textures/blocks/stone"), "block/stone");
        assert_eq!(
            j("textures/blocks/wool_colored_silver"),
            "block/light_gray_wool"
        );
        assert_eq!(
            j("textures/blocks/hardened_clay_stained_red"),
            "block/red_terracotta"
        );
        assert_eq!(j("textures/blocks/stone_andesite"), "block/andesite");
        assert_eq!(
            j("textures/blocks/angler_pottery_pattern"),
            "entity/decorated_pot/angler_pottery_pattern"
        );
        assert_eq!(j("textures/items/wood_sword"), "item/wooden_sword");
        assert_eq!(j("textures/items/gold_pickaxe"), "item/golden_pickaxe");
        assert_eq!(j("textures/items/diamond_axe"), "item/diamond_axe");
        assert_eq!(j("textures/entity/bed/silver"), "entity/bed/light_gray");
        assert_eq!(
            j("textures/entity/skulls/zombie.png"),
            "entity/zombie/zombie"
        );
        assert_eq!(
            j("textures/entity/villager2/levels/level_iron"),
            "entity/villager/profession_level/iron"
        );
        assert_eq!(
            j("textures/models/armor/chain_1"),
            "entity/equipment/humanoid/chainmail"
        );
        assert_eq!(
            j("textures/models/armor/iron_2"),
            "entity/equipment/humanoid_leggings/iron"
        );
        assert_eq!(
            j("textures/models/armor/elytra"),
            "entity/equipment/wings/elytra"
        );
        assert_eq!(java_texture("ui/x"), None);
    }

    #[test]
    fn a_texture_past_the_atlas_is_averaged_down_by_a_power_of_two() {
        assert_eq!(atlas_shrink(1024, 512), 1);
        assert_eq!(atlas_shrink(1024, 1024), 2);
        assert_eq!(atlas_shrink(3072, 128), 4);
        assert_eq!(atlas_shrink(64, 880), 2);
        let mut rgba = Vec::new();
        for y in 0..1024u32 {
            for x in 0..1024u32 {
                rgba.extend_from_slice(if (x + y) % 2 == 0 { &[200, 0, 0, 255] } else { &[0, 0, 0, 0] });
            }
        }
        let png = encode_png(&Image { width: 1024, height: 1024, rgba }).unwrap();
        let small = fit_to_atlas(&png).unwrap();
        let px = decode_png_for_test(&small);
        assert_eq!(px.len(), 512 * 512);
        // Half the texels are opaque red: the colour stays red, the alpha halves.
        assert_eq!(px[0], [200, 0, 0, 127]);
        assert!(fit_to_atlas(&small).is_none());
    }

    #[test]
    fn the_solid_texture_makes_every_texel_that_is_not_clear_opaque() {
        let eyes = encode_png(&Image {
            width: 2,
            height: 2,
            rgba: vec![
                1, 2, 3, 0, //
                4, 5, 6, 157, //
                7, 8, 9, 255, //
                10, 11, 12, 2,
            ],
        })
        .unwrap();
        let out = solid_png(&eyes).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(out))
            .read_info()
            .unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut buf).unwrap();
        let alpha: Vec<u8> = buf.chunks_exact(4).map(|p| p[3]).collect();
        assert_eq!(alpha, [0, 255, 255, 255]);
        assert!(
            solid_png(
                &encode_png(&Image {
                    width: 1,
                    height: 1,
                    rgba: vec![0, 0, 0, 255]
                })
                .unwrap()
            )
            .is_none()
        );
    }

    #[test]
    fn the_glow_holds_only_the_texels_less_than_opaque() {
        let png = |px: [[u8; 4]; 4]| {
            encode_png(&Image {
                width: 2,
                height: 2,
                rgba: px.concat(),
            })
            .unwrap()
        };
        let solid = png([[9, 9, 9, 255]; 4]);
        assert!(glow_png(&solid).is_none());
        let eyes = png([[1, 2, 3, 255], [4, 5, 6, 2], [7, 8, 9, 0], [1, 1, 1, 128]]);
        let out = glow_png(&eyes).unwrap();
        let img = decode_png_for_test(&out);
        assert_eq!(img[0], [0, 0, 0, 0]);
        assert_eq!(img[1], [4, 5, 6, 253]);
        assert_eq!(img[2], [0, 0, 0, 0]);
        assert_eq!(img[3], [1, 1, 1, 127]);
    }

    fn decode_png_for_test(b: &[u8]) -> Vec<[u8; 4]> {
        let mut r = png::Decoder::new(std::io::Cursor::new(b))
            .read_info()
            .unwrap();
        let mut buf = vec![0; r.output_buffer_size().unwrap()];
        let i = r.next_frame(&mut buf).unwrap();
        buf[..i.buffer_size()]
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect()
    }

    #[test]
    fn faint_texels_of_a_cutout_texture_turn_solid() {
        let png = |px: [[u8; 4]; 4]| {
            encode_png(&Image {
                width: 2,
                height: 2,
                rgba: px.concat(),
            })
            .unwrap()
        };
        let decode = |b: &[u8]| {
            let mut d = png::Decoder::new(std::io::Cursor::new(b));
            d.set_transformations(png::Transformations::EXPAND);
            let mut r = d.read_info().unwrap();
            let mut buf = vec![0; r.output_buffer_size().unwrap()];
            r.next_frame(&mut buf).unwrap();
            buf
        };
        // An eye white at alpha 2 beside solid and empty texels: it turns solid.
        let eye = png([
            [255, 255, 255, 2],
            [9, 9, 9, 255],
            [0, 0, 0, 0],
            [1, 2, 3, 255],
        ]);
        let lifted = lift_faint_alpha(&eye).unwrap();
        assert_eq!(decode(&lifted)[..4], [255, 255, 255, 255]);
        assert_eq!(decode(&lifted)[8..12], [0, 0, 0, 0]);
        // Real translucency anywhere keeps the faint texel faint.
        let glow = png([
            [255, 255, 255, 2],
            [9, 9, 9, 128],
            [0, 0, 0, 0],
            [1, 2, 3, 255],
        ]);
        assert!(lift_faint_alpha(&glow).is_none());
        // A filler of faint texels, more than the solid ones: left clear.
        let filler = png([
            [255, 255, 255, 1],
            [255, 255, 255, 1],
            [9, 9, 9, 255],
            [0, 0, 0, 0],
        ]);
        assert!(lift_faint_alpha(&filler).is_none());
        // Nothing faint, nothing to do.
        let plain = png([
            [255, 255, 255, 0],
            [9, 9, 9, 255],
            [0, 0, 0, 0],
            [1, 2, 3, 255],
        ]);
        assert!(lift_faint_alpha(&plain).is_none());
    }

    use super::*;
    use crate::pack::Pack;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "horizon-bedrock-conv-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn put(root: &Path, rel: &str, bytes: &[u8]) {
        write(&root.join(rel), bytes).unwrap();
    }

    fn tga(w: u16, h: u16, top_first: bool, rle: bool) -> Vec<u8> {
        let mut b = vec![0u8; 18];
        b[2] = if rle { 10 } else { 2 };
        b[12..14].copy_from_slice(&w.to_le_bytes());
        b[14..16].copy_from_slice(&h.to_le_bytes());
        b[16] = 32;
        b[17] = if top_first { 0x28 } else { 0x08 };
        // Two rows: red over green (BGRA).
        let row = |c: [u8; 4]| {
            std::iter::repeat_n(c, w as usize)
                .flatten()
                .collect::<Vec<u8>>()
        };
        let (first, second) = (row([0, 0, 255, 255]), row([0, 255, 0, 255]));
        if rle {
            for r in [&first, &second] {
                b.push(0x80 | (w as u8 - 1));
                b.extend_from_slice(&r[..4]);
            }
        } else {
            b.extend(first);
            b.extend(second);
        }
        b
    }

    #[test]
    fn tga_variants_decode_to_the_same_picture() {
        for (top, rle) in [(true, false), (true, true)] {
            let img = decode_tga(&tga(2, 2, top, rle)).unwrap();
            assert_eq!((img.width, img.height), (2, 2));
            assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255], "top={top} rle={rle}");
            assert_eq!(&img.rgba[8..12], &[0, 255, 0, 255]);
        }
        // Bottom-left origin: the first stored row is the bottom one.
        let img = decode_tga(&tga(2, 2, false, false)).unwrap();
        assert_eq!(&img.rgba[0..4], &[0, 255, 0, 255]);
        assert!(decode_tga(&[0; 5]).is_none());
        let png = encode_png(&img).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn a_tga_with_a_faint_texel_has_it_lifted_like_a_png() {
        let dir = tmp("tga-faint");
        fixture(&dir);
        // Two pixels (BGRA): an eye at alpha 3 and a solid one.
        let mut b = vec![0u8; 18];
        b[2] = 2;
        b[12] = 2;
        b[14] = 1;
        b[16] = 32;
        b[17] = 0x28;
        b.extend([0, 0, 255, 3, 9, 9, 9, 255]);
        put(&dir, "textures/entity/eye.tga", &b);
        let pack = Pack::open(&dir).unwrap();
        let alpha = |png: &[u8]| {
            let mut d = png::Decoder::new(std::io::Cursor::new(png));
            d.set_transformations(png::Transformations::EXPAND);
            let mut r = d.read_info().unwrap();
            let mut buf = vec![0; r.output_buffer_size().unwrap()];
            r.next_frame(&mut buf).unwrap();
            (buf[3], buf[7])
        };
        let lifted = texture_png(&pack, "textures/entity/eye", true).unwrap();
        assert_eq!(alpha(&lifted), (255, 255));
        let raw = texture_png(&pack, "textures/entity/eye", false).unwrap();
        assert_eq!(alpha(&raw), (3, 255));
    }

    fn fixture(dir: &Path) {
        put(
            dir,
            "manifest.json",
            br#"{"format_version":2,"header":{"name":"Demo","uuid":"00000000-0000-0000-0000-000000000001","version":[1,0,0]},
                "modules":[{"type":"resources","uuid":"00000000-0000-0000-0000-000000000002","version":[1,0,0]}]}"#,
        );
        put(dir, "pack_icon.png", b"\x89PNG icon");
        put(
            dir,
            "entity/zombie.entity.json",
            br#"{"minecraft:client_entity":{"description":{"identifier":"minecraft:zombie","textures":{"default":"textures/entity/Z","tga":"textures/entity/t","gone":"textures/entity/missing"},
                "geometry":{"default":"geometry.z"},"materials":{"default":"zombie"},"render_controllers":["controller.render.z"]}}}"#,
        );
        put(dir, "textures/entity/Z.png", b"\x89PNG zombie");
        put(dir, "textures/entity/t.tga", &tga(1, 1, true, false)[..]);
        put(dir, "textures/unused/x.png", b"no");
        put(
            dir,
            "particles/p.particle.json",
            br#"{"particle_effect":{"description":{"identifier":"d:p","basic_render_parameters":{"material":"particles_alpha","texture":"textures/particle/dust"}},"components":{}}}"#,
        );
        put(dir, "textures/particle/dust.png", b"\x89PNG dust");
        put(
            dir,
            "sounds/sound_definitions.json",
            br#"{"sound_definitions":{"Demo:Hit.Big":{"category":"neutral","sounds":["sounds/demo/a",{"name":"sounds/demo/b","volume":[0.5,1],"pitch":1.2,"weight":2},{"name":"sounds/demo/w"}]},
                "mob.zombie.step":{"sounds":["sounds/demo/a"]}}}"#,
        );
        put(dir, "sounds/demo/a.ogg", b"OggS a");
        put(dir, "sounds/demo/b.ogg", b"OggS b");
        put(dir, "sounds/demo/w.wav", b"RIFF");
        put(dir, "animations/a.animation.json", b"{}");
        put(dir, "models/entity/m.geo.json", b"{}");
        put(dir, "sounds.json", b"{}");
        put(dir, "texts/en_US.lang", b"a=b");
    }

    #[test]
    fn writes_a_java_pack() {
        let src = tmp("src");
        fixture(&src);
        let out = tmp("out");
        let pack = Pack::open(&src).unwrap();
        let r = to_java(&pack, &out, (64, 0)).unwrap();
        let ns = namespace(&pack);
        let meta: Value =
            serde_json::from_slice(&fs::read(out.join("pack.mcmeta")).unwrap()).unwrap();
        assert_eq!(meta["pack"]["min_format"], json!([64, 0]));
        assert!(out.join("pack.png").exists());
        // Textures: lower-cased atlas names, TGA converted, the unused one left out.
        assert!(
            out.join("assets/minecraft/textures/entity/bedrock/entity/z.png")
                .exists()
        );
        assert!(
            fs::read(out.join("assets/minecraft/textures/entity/bedrock/entity/t.png"))
                .unwrap()
                .starts_with(b"\x89PNG")
        );
        assert!(
            out.join("assets/minecraft/textures/entity/bedrock/particle/dust.png")
                .exists()
        );
        assert!(
            !out.join("assets/minecraft/textures/entity/bedrock/unused/x.png")
                .exists()
        );
        assert_eq!((r.textures, r.missing_textures), (3, 1));
        // Data copied as is.
        assert!(
            out.join(format!("assets/horizon_bedrock/{ns}/entity/zombie.entity.json"))
                .exists()
        );
        assert!(
            out.join(format!("assets/horizon_bedrock/{ns}/animations/a.animation.json"))
                .exists()
        );
        assert!(
            out.join(format!("assets/horizon_bedrock/{ns}/models/entity/m.geo.json"))
                .exists()
        );
        assert!(
            out.join(format!("assets/horizon_bedrock/{ns}/sounds/sound_definitions.json"))
                .exists()
        );
        assert!(!out.join(format!("assets/horizon_bedrock/{ns}/texts/en_US.lang")).exists());
        // Sounds: per namespace, Java names, averaged ranges, wav skipped.
        let sj: Value =
            serde_json::from_slice(&fs::read(out.join("assets/demo/sounds.json")).unwrap())
                .unwrap();
        let hit = &sj["hit.big"];
        assert_eq!(hit["category"], "neutral");
        assert_eq!(hit["sounds"].as_array().unwrap().len(), 2);
        assert_eq!(
            hit["sounds"][1],
            json!({"name":"demo:demo/b","volume":0.75,"pitch":1.2,"weight":2})
        );
        assert!(out.join("assets/demo/sounds/demo/a.ogg").exists());
        assert!(out.join("assets/bedrock/sounds.json").exists());
        assert_eq!(r.skipped_sounds, 1);
        assert_eq!(r.sound_events, 2);
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn cache_names_follow_the_input() {
        let a = cache_name(Path::new("/p/My Pack.mcpack"), 5, 10);
        assert!(a.to_string_lossy().starts_with("my_pack-"));
        assert_eq!(a, cache_name(Path::new("/p/My Pack.mcpack"), 5, 10));
        assert_ne!(a, cache_name(Path::new("/p/My Pack.mcpack"), 6, 10));
    }
}
