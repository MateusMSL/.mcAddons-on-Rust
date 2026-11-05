//! Bedrock resource packs' look for mobs: a client entity's geometry, textures, animations and
//! render controllers ([`horizon_bedrock`]) over a mob the Java logic runs.
//!
//! Only what is drawn and heard comes from the pack. The mob's state is read, never written:
//! [`MobHost`] answers a pack's `query.*` from the `Mob` (its name tag, whether it is a baby,
//! how fast it walks...), and nothing in the pack reaches back into the game. Without a pack
//! the bundle is empty and every mob keeps its vanilla model.
//!
//! The pack's files reach this module through the resource root (`assets/horizon_bedrock/`,
//! written by `horizon_bedrock::convert`); its textures are in the entity atlas as
//! `entity/bedrock/...`.

use super::{Mob, Part, Polygon};
use horizon_bedrock::{
    animation::Pose,
    bundle::Bundle,
    convert::atlas_name,
    entity::{Assets, Emit, Instance, Layer},
    geometry::{self, Geometry},
    molang::{Host, Value},
    particle::{Emitter, Spawn, Sprite, World},
};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex, PoisonError},
    time::Instant,
};

/// A model baked for the game with the names of its parts.
pub struct Built {
    pub part: Arc<Part>,
    /// Every part's name (lower case), for hiding bones.
    pub names: Vec<String>,
    pub geometry: String,
}

/// What one mob draws this frame.
pub struct MobFrame {
    pub layers: Vec<DrawLayer>,
    pub pose: Pose,
    pub scale: f32,
    /// `scalex` / `scaley` / `scalez`, on top of `scale`.
    pub scale_axes: [f32; 3],
    /// How the pack has what the mob holds drawn.
    pub held: Held,
    /// What the mob holds or wears that the pack dresses (a bow in hand, a helmet).
    pub attached: Vec<Attached>,
}

/// `hide_held_items`, `held_item_scale` and `held_item_ignores_lighting` of a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub hidden: bool,
    pub scale: f32,
    pub full_bright: bool,
    /// The pack dresses the item in the main hand (an attachable draws it): the game's own
    /// model of it stays out.
    pub main_replaced: bool,
    /// The same for the off hand.
    pub off_replaced: bool,
}

impl Default for Held {
    fn default() -> Held {
        Held {
            hidden: false,
            scale: 1.0,
            full_bright: false,
            main_replaced: false,
            off_replaced: false,
        }
    }
}

/// A held or worn item's look: its layers, and the bone of the holder each of its bones follows.
pub struct Attached {
    pub slot: &'static str,
    pub layers: Vec<DrawLayer>,
    pub pose: Pose,
    /// The attachable's bone (lower case) to the holder's bone it follows.
    pub binds: HashMap<String, String>,
}

pub struct DrawLayer {
    pub built: Arc<Built>,
    /// The texture's atlas name (`entity/bedrock/...`).
    pub texture: String,
    /// The vanilla Java texture standing in when the pack does not ship `texture`.
    pub fallback: Option<String>,
    pub layer: Layer,
}

/// Something the frames set off, for the game to play or spawn.
#[derive(Clone, Debug, PartialEq)]
pub enum Emitted {
    Sound {
        event: String,
        at: [f64; 3],
    },
    Particle {
        effect: String,
        at: [f64; 3],
        yaw: f32,
        baby: bool,
        name: Option<String>,
        size: [f32; 2],
        /// The mob the effect is attached to, which it follows.
        follow: Option<Follow>,
    },
}

/// Where on a mob an effect is attached.
#[derive(Clone, Debug, PartialEq)]
pub struct Follow {
    pub mob: u32,
    /// The locator's name, which with the effect tells an effect already running from a new one.
    pub locator: Option<String>,
    /// From the mob's feet in the model's own axes (x, y up, z with the front at -z), in blocks.
    pub offset: [f64; 3],
}

/// A model-space offset of a mob standing at `pos` and turned `yaw` degrees, in the world.
fn attach_point(pos: [f64; 3], offset: [f64; 3], yaw: f32) -> [f64; 3] {
    let yaw = f64::from(yaw).to_radians();
    let [x, y, z] = offset;
    [
        pos[0] + x * yaw.cos() + z * yaw.sin(),
        pos[1] + y,
        pos[2] - x * yaw.sin() + z * yaw.cos(),
    ]
}

struct MobState {
    instance: Instance,
    /// The mob's age in ticks (with the partial tick) at the last update.
    time: f32,
    frame: Option<Arc<MobFrame>>,
    seen: Instant,
    /// When `HORIZON_BEDROCK_DUMP` last logged this mob (its age in ticks).
    dumped: f32,
}

/// A held item's running animations.
struct AttState {
    /// The attachable it runs.
    id: String,
    instance: Instance,
    time: f32,
    seen: Instant,
}

struct Active {
    /// When an animation last asked for this effect.
    requested: Instant,
    /// Set off by a controller state, which asks for it every frame it lasts.
    held: bool,
    emitter: Emitter,
    host: Snapshot,
    follow: Option<Follow>,
}

/// A mob's facts, kept for a particle effect that outlives the frame that asked for it.
#[derive(Clone, Default)]
struct Snapshot {
    baby: bool,
    name: Option<String>,
    age: f32,
    /// Width and height of the mob's box, for `emitter_shape_entity_aabb`.
    size: [f32; 2],
}

/// The level an effect rides in: it knows the size of the mob that carries it.
struct Carrier([f32; 2]);

impl World for Carrier {
    fn entity_size(&self) -> [f32; 2] {
        if self.0[0] > 0.0 && self.0[1] > 0.0 {
            self.0
        } else {
            [0.6, 1.8]
        }
    }
}

/// The size the game gives a mob's model before any `scale` script: the vanilla rabbit's script
/// reads `1 / 0.6` (`1 / 0.4` for a baby) to cancel it, and a pack that scales the rabbit writes
/// its own factor on top of the same.
fn builtin_scale(id: &str, baby: bool) -> f32 {
    match id {
        "minecraft:rabbit" if baby => 0.4,
        "minecraft:rabbit" => 0.6,
        _ => 1.0,
    }
}

/// A mob's box as `[width, height]`.
fn box_size(m: &Mob) -> [f32; 2] {
    let (w, h, _) = m.dims();
    [w as f32, h as f32]
}

impl Host for Snapshot {
    fn query(&mut self, name: &str, args: &[Value]) -> Value {
        match name {
            "is_baby" => Value::bool(self.baby),
            "is_name_any" => Value::bool(name_matches(self.name.as_deref(), args)),
            "life_time" => Value::Num(self.age / 20.0),
            _ => common_query(name, args),
        }
    }
}

fn name_matches(name: Option<&str>, args: &[Value]) -> bool {
    let Some(name) = name else { return false };
    args.iter()
        .any(|a| matches!(a, Value::Str(s) if &**s == name))
}

/// What a pack may ask about the world around a mob, sampled in the mob's tick (the render has
/// no world to ask): the blocks within two of it, its biome, the time and the weather.
#[derive(Debug, Default)]
pub struct Surroundings {
    /// Block names (`minecraft:stone`) of the 5x5x5 cube centred on the mob's block, x fastest,
    /// then z, then y.
    blocks: Vec<Box<str>>,
    biome: Box<str>,
    day_time: i64,
    raining: bool,
    thundering: bool,
}

const REACH: i32 = 2;

/// Set the first time a pack asks something the sample answers.
static WANTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// The game's block tags, kept from the first sample, for `relative_block_has_any_tag`.
static BLOCK_TAGS: std::sync::OnceLock<HashMap<String, std::collections::HashSet<String>>> =
    std::sync::OnceLock::new();

/// Whether any pack expression asked about the world: ticks sample only then.
pub fn wants_surroundings() -> bool {
    WANTED.load(std::sync::atomic::Ordering::Relaxed)
}

/// The sample of the world around `m`.
pub fn sample_surroundings(m: &Mob, w: &super::MobWorld) -> Surroundings {
    let _ = BLOCK_TAGS.set(w.block_tags.clone());
    let at = [
        m.position.x.floor() as i32,
        m.position.y.floor() as i32,
        m.position.z.floor() as i32,
    ];
    let mut blocks = Vec::with_capacity(125);
    for dy in -REACH..=REACH {
        for dz in -REACH..=REACH {
            for dx in -REACH..=REACH {
                let state = w.world.block(at[0] + dx, at[1] + dy, at[2] + dz);
                let name: &str = &w.facts.get(state).name;
                blocks.push(Box::from(name));
            }
        }
    }
    let biome = w
        .world
        .chunks
        .get(&(at[0] >> 4, at[2] >> 4))
        .map(|c| {
            w.biomes
                .get(c.biome(at[0], at[1], at[2]))
                .name
                .as_str()
                .into()
        })
        .unwrap_or_default();
    Surroundings {
        blocks,
        biome,
        day_time: w.day_time,
        raining: w.raining,
        thundering: w.thundering,
    }
}

/// Whether `name` has `tag` (a block tag of the game, with or without its namespace).
fn block_has_tag(name: &str, tag: &str) -> bool {
    let Some(tags) = BLOCK_TAGS.get() else {
        return false;
    };
    let tag = tag.to_ascii_lowercase();
    let full = if tag.contains(':') {
        tag.clone()
    } else {
        format!("minecraft:{tag}")
    };
    [&full, &tag]
        .iter()
        .any(|t| tags.get(t.as_str()).is_some_and(|set| set.contains(name)))
}

/// The answers that come from the sampled world.
fn world_query(m: &Mob, name: &str, args: &[Value]) -> Option<Value> {
    let asks = matches!(
        name,
        "relative_block_has_any_tag"
            | "entity_biome_has_any_identifier"
            | "time_of_day"
            | "moon_phase"
            | "is_raining"
            | "is_thundering"
    );
    if !asks {
        return None;
    }
    WANTED.store(true, std::sync::atomic::Ordering::Relaxed);
    let Some(s) = m.extra.surroundings.as_deref() else {
        return Some(Value::Num(0.0));
    };
    Some(match name {
        "relative_block_has_any_tag" => {
            let n = |i: usize| args.get(i).map_or(0.0, Value::num).floor() as i32;
            let (dx, dy, dz) = (n(0), n(1), n(2));
            if dx.abs() > REACH || dy.abs() > REACH || dz.abs() > REACH {
                return Some(Value::Num(0.0));
            }
            let i = ((dy + REACH) * 25 + (dz + REACH) * 5 + (dx + REACH)) as usize;
            let block = s.blocks.get(i).map(|b| &**b).unwrap_or("");
            Value::bool(args.iter().skip(3).any(|a| match a {
                Value::Str(t) => block_has_tag(block, t),
                _ => false,
            }))
        }
        "entity_biome_has_any_identifier" => Value::bool(args.iter().any(|a| match a {
            Value::Str(t) => {
                let t = t.trim_start_matches("minecraft:");
                let b = s.biome.trim_start_matches("minecraft:");
                t.eq_ignore_ascii_case(b)
            }
            _ => false,
        })),
        // Bedrock: midnight 0.0, sunrise 0.25, noon 0.5. Java's tick 6000 is noon, 0 sunrise.
        "time_of_day" => Value::Num(((s.day_time + 6000).rem_euclid(24000)) as f32 / 24000.0),
        "moon_phase" => Value::Num(s.day_time.div_euclid(24000).rem_euclid(8) as f32),
        "is_raining" => Value::bool(s.raining),
        _ => Value::bool(s.thundering),
    })
}

/// Whether the pack dresses players: its player animations run on the game's own humanoid
/// model and skin. On unless `HORIZON_BEDROCK_PLAYERS` is `0` or `off`.
fn players_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| {
        !std::env::var("HORIZON_BEDROCK_PLAYERS")
            .is_ok_and(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "0" | "off" | "false"))
    })
}

/// A player or mannequin: drawn on the game's skin.
pub fn is_avatar(id: &str) -> bool {
    matches!(id, "minecraft:player" | "minecraft:mannequin")
}

/// The frame of a held clock or compass, which a pack reads as `query.get_animation_frame`.
fn item_frame(m: &Mob, item: &str) -> f32 {
    let day = m.extra.surroundings.as_deref().map_or(0, |s| s.day_time);
    match item.trim_start_matches("minecraft:") {
        "clock" => (((day + 6000).rem_euclid(24000)) as f32 / 24000.0 * 64.0).floor(),
        _ => 0.0,
    }
}

/// Answers shared by every host: the parts of the world the game does not tell a pack.
fn common_query(name: &str, args: &[Value]) -> Value {
    match name {
        "is_in_ui" | "is_attached" | "is_spectator" | "is_local_player" | "is_emoting"
        | "vertical_speed" | "mark_variant" | "get_animation_frame" | "is_in_water" | "is_swimming" => {
            Value::Num(0.0)
        }
        "is_pack_setting_enabled" => Value::bool(
            matches!(args.first(), Some(Value::Str(n)) if horizon_bedrock::settings::is_enabled(n)),
        ),
        "is_pack_setting_selected" => Value::bool(matches!(
            (args.first(), args.get(1)),
            (Some(Value::Str(n)), Some(Value::Str(o))) if horizon_bedrock::settings::is_selected(n, o)
        )),
        "graphics_mode_is_any" => Value::bool(
            args.iter()
                .any(|a| matches!(a, Value::Str(s) if s.eq_ignore_ascii_case("fancy"))),
        ),
        "in_range" => {
            let n = |i: usize| args.get(i).map_or(0.0, Value::num);
            Value::bool(n(0) >= n(1) && n(0) <= n(2))
        }
        "is_owner_identifier_any" => Value::Num(0.0),
        // What the game keeps no equivalent of (spear timings, vibrations, timers, the skin
        // kind, the particle taken from a block): 0.
        "timer_flag_1"
        | "timer_flag_2"
        | "timer_flag_3"
        | "structural_integrity"
        | "key_frame_lerp_time"
        | "surface_particle_texture_size"
        | "surface_particle_texture_coordinate"
        | "surface_particle_color"
        | "kinetic_weapon_delay"
        | "kinetic_weapon_dismount_duration"
        | "kinetic_weapon_knockback_duration"
        | "kinetic_weapon_damage_duration"
        | "ticks_since_last_kinetic_weapon_hit"
        | "bone_aabb"
        | "tail_angle"
        | "show_bottom"
        | "heartbeat_phase"
        | "cape_flap_amount"
        | "life_span"
        | "spellcolor"
        | "get_root_locator_offset"
        | "texture_frame_index"
        | "is_persona_or_premium_skin"
        | "is_attached_to_entity"
        | "time_since_last_vibration_detection"
        // Logic the game keeps and a resource pack only reads for flavour: 0.
        | "actor_count"
        | "anger_level"
        | "can_climb"
        | "can_dash"
        | "can_fly"
        | "can_power_jump"
        | "combine_entities"
        | "cooldown_time"
        | "cooldown_time_remaining"
        | "is_cooldown_category"
        | "effect_emitter_count"
        | "effect_particle_count"
        | "had_component_group"
        | "has_any_family"
        | "heartbeat_interval"
        | "heightmap"
        | "is_avoiding_block"
        | "is_breathing"
        | "is_bribed"
        | "is_critical"
        | "is_enchanted"
        | "is_idling"
        | "is_ignited"
        | "is_illager_captain"
        | "is_laying_down"
        | "is_lingering"
        | "is_orphaned"
        | "is_ram_attacking"
        | "is_rising"
        | "is_rolling"
        | "is_scenting"
        | "is_selected_item"
        | "is_silent"
        | "is_sneezing"
        | "is_stackable"
        | "is_transforming"
        | "item_in_use_duration"
        | "item_max_use_duration"
        | "last_hit_by_player"
        | "log"
        | "max_durability"
        | "max_trade_tier"
        | "noise"
        | "out_of_control"
        | "player_level"
        | "remaining_durability"
        | "rider_body_x_rotation"
        | "rider_body_y_rotation"
        | "rider_head_x_rotation"
        | "rider_head_y_rotation"
        | "scoreboard"
        | "shield_blocking_bob"
        | "get_actor_info_id"
        | "get_locator_offset"
        | "bone_orientation_matrix"
        | "block_face"
        | "block_state"
        | "has_block_state"
        | "cardinal_facing"
        | "cardinal_facing_2d"
        | "cardinal_player_facing"
        | "block_has_all_tags"
        | "block_has_any_tag"
        | "block_neighbor_has_all_tags"
        | "block_neighbor_has_any_tag"
        | "relative_block_has_all_tags"
        | "entity_biome_has_all_tags"
        | "entity_biome_has_any_tags"
        | "has_all_biome_tags"
        | "has_any_biome_tags"
        | "has_biome_tag"
        | "equipped_item_all_tags"
        | "equipped_item_is_attachable"
        | "all_tags"
        | "any_tag"
        | "get_pack_setting" => Value::Num(0.0),
        "item_slot_to_bone_name" | "get_equipped_item_name" => Value::Str("".into()),
        other => {
            note_unanswered(other);
            Value::Null
        }
    }
}

/// The engine's build, logged once with `HORIZON_BEDROCK_DEBUG`: tells whether the game runs the
/// files the last overlay carried.
pub const ENGINE_BUILD: &str = "v50-defs-rabbit";

/// `HORIZON_BEDROCK_DUMP=skeleton,creeper`: the kinds (by part of the id) whose state is logged
/// once a second while within twelve blocks of the camera.
fn dump_wanted(kind: &str) -> bool {
    super::bedrock_debug::settings().dumps(kind)
}

/// One line of what a mob's pack is doing: what the game answers, what runs, the bones moved
/// and what is drawn.
fn dump_line(m: &Mob, host: &mut MobHost, frame: &horizon_bedrock::entity::Frame, running: &str) -> String {
    use std::fmt::Write;
    let mut q = String::new();
    for name in [
        "has_target",
        "facing_target_to_range_attack",
        "is_using_item",
        "target_x_rotation",
        "target_y_rotation",
        "modified_move_speed",
        "modified_distance_moved",
        "ground_speed",
        "vertical_speed",
        "is_swimming",
        "is_in_water",
        "is_sprinting",
        "is_on_ground",
        "swell_amount",
        "is_powered",
        "is_saddled",
        "is_baby",
        "is_tamed",
        "has_rider",
        "is_riding",
        "is_charged",
        "is_angry",
        "is_sheared",
        "is_sleeping",
        "is_sitting",
        "is_alive",
        "is_on_fire",
        "is_attached",
        "variant",
        "mark_variant",
        "skin_id",
        "body_x_rotation",
        "body_y_rotation",
        "yaw_speed",
        "life_time",
    ] {
        let v = host.query(name, &[]);
        let _ = write!(q, " {name}={}", v.num());
    }
    let _ = write!(
        q,
        " main_hand={:?} using_item={} hurt={} death={}",
        m.extra.main_hand,
        m.extra.using_item,
        m.hurt_time,
        m.death_time
    );
    let settings = super::bedrock_debug::settings();
    let wanted: Vec<String> = settings
        .dump_bones
        .iter()
        .map(|n| n.to_ascii_lowercase())
        .collect();
    let mut bones: Vec<_> = frame
        .pose
        .bones
        .iter()
        .filter(|(name, b)| {
            if !wanted.is_empty() {
                let name = name.to_ascii_lowercase();
                return wanted.iter().any(|w| name.contains(w.as_str()));
            }
            b.rotation
                .iter()
                .chain(&b.rotation_entity)
                .any(|v| v.abs() > 0.5)
                || b.position.iter().any(|v| v.abs() > 0.02)
                || b.scale.iter().any(|v| (v - 1.0).abs() > 0.01)
        })
        .collect();
    bones.sort_by(|a, b| a.0.cmp(b.0));
    let mut b = String::new();
    for (name, p) in bones.iter().take(if wanted.is_empty() { 30 } else { 80 }) {
        let _ = write!(
            b,
            " {name}(rot={:?} ent={:?} pos={:?} sc={:?})",
            p.rotation.map(|v| (v * 10.0).round() / 10.0),
            p.rotation_entity.map(|v| (v * 10.0).round() / 10.0),
            p.position.map(|v| (v * 10.0).round() / 10.0),
            p.scale.map(|v| (v * 100.0).round() / 100.0),
        );
    }
    // How many bones the animations touched at all, and the biggest turn among them: tells an
    // animation that never ran from one that ran with nothing to move.
    let touched = frame.pose.bones.len();
    let biggest = frame
        .pose
        .bones
        .values()
        .flat_map(|b| b.rotation.iter().chain(&b.rotation_entity))
        .fold(0.0f32, |a, v| a.max(v.abs()));
    let _ = write!(b, " [touched={touched} max_rot={biggest:.2}]");
    let layers: Vec<String> = frame
        .layers
        .iter()
        .map(|l| {
            format!(
                "{}|{}|mat={}|col={:?}|ov={:?}|uv={:?}|hide={:?}",
                l.geometry.trim_start_matches("geometry.oreville_ans."),
                l.texture.rsplit('/').next().unwrap_or(""),
                l.material,
                l.color,
                l.overlay,
                l.uv_anim,
                l.visibility
                    .iter()
                    .filter(|(_, v)| !*v)
                    .map(|(p, _)| p.as_str())
                    .collect::<Vec<_>>()
            )
        })
        .collect();
    format!(
        "q:{q} || running: {running} || bones:{b} || layers: {}",
        layers.join(" ; ")
    )
}

/// With `HORIZON_BEDROCK_DEBUG`, each query nothing answers is logged once: a pack's animation
/// that never starts is often waiting on one of them.
fn note_unanswered(name: &str) {
    if !super::bedrock_debug::debug() {
        return;
    }
    static SEEN: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);
    if lock(&SEEN)
        .get_or_insert_with(Default::default)
        .insert(name.to_owned())
    {
        log::info!("bedrock: query.{name} has no answer (reads as 0)");
    }
}

/// Answers a pack's queries from a mob.
pub struct MobHost<'a> {
    pub m: &'a Mob,
    pub alpha: f32,
    pub dt: f32,
    /// The mob's position minus the camera's, in blocks.
    pub from_camera: [f32; 3],
}

static CAMERA_ROTATION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The camera's `yaw` and `pitch` in degrees, for `query.camera_rotation`.
pub fn set_camera_rotation(yaw: f32, pitch: f32) {
    let bits = (u64::from(yaw.to_bits()) << 32) | u64::from(pitch.to_bits());
    CAMERA_ROTATION.store(bits, std::sync::atomic::Ordering::Relaxed);
}

fn camera_rotation() -> (f32, f32) {
    let bits = CAMERA_ROTATION.load(std::sync::atomic::Ordering::Relaxed);
    (f32::from_bits((bits >> 32) as u32), f32::from_bits(bits as u32))
}

/// `query.rotation_to_camera`: the (pitch, yaw) in degrees that would face the camera from a
/// mob `from_camera` blocks away from it.
fn rotation_to_camera(from_camera: [f32; 3]) -> (f32, f32) {
    let [x, y, z] = from_camera.map(|v| -v);
    let flat = x.hypot(z);
    if flat < 1e-4 && y.abs() < 1e-4 {
        return (0.0, 0.0);
    }
    ((-y.atan2(flat)).to_degrees(), (-x).atan2(z).to_degrees())
}

fn is_villager(id: &str) -> bool {
    matches!(id, "minecraft:villager" | "minecraft:zombie_villager")
}

/// Bedrock's profession number for the Java profession at `index` of
/// [`super::render::PROFESSIONS`].
fn bedrock_profession(index: u8) -> u32 {
    let java = super::render::PROFESSIONS[usize::from(index) % super::render::PROFESSIONS.len()];
    const BEDROCK: [&str; 15] = [
        "none",
        "farmer",
        "fisherman",
        "shepherd",
        "fletcher",
        "librarian",
        "cartographer",
        "cleric",
        "armorer",
        "weaponsmith",
        "toolsmith",
        "butcher",
        "leatherworker",
        "mason",
        "nitwit",
    ];
    BEDROCK.iter().position(|b| *b == java).unwrap_or(0) as u32
}

fn wrap_degrees(a: f32) -> f32 {
    let a = a % 360.0;
    if a > 180.0 {
        a - 360.0
    } else if a < -180.0 {
        a + 360.0
    } else {
        a
    }
}

impl Host for MobHost<'_> {
    fn variables(&mut self) -> Vec<(&'static str, Value)> {
        let m = self.m;
        // The swing's progress, 0 at rest to 1 at its end, as the Java mob keeps it.
        let a = self.alpha;
        let mut swing = m.anim.attack_anim - m.anim.attack_anim_o;
        if swing < 0.0 {
            swing += 1.0;
        }
        let mut swing = (m.anim.attack_anim_o + swing * a).clamp(0.0, 1.0);
        // A golem's swing is its own ten-tick clock.
        if m.kind.id == "minecraft:iron_golem" && m.extra.anim_ticks > 0 {
            swing = ((10.0 - m.extra.anim_ticks as f32 + a) / 10.0).clamp(0.0, 1.0);
        }
        // Ticks left of a golem's ten-tick swing (`attackAnimationTick`), 0 at rest.
        let ticks = if m.kind.id == "minecraft:iron_golem" {
            m.extra.anim_ticks.max(0) as f32
        } else {
            0.0
        };
        let holds = |slot: &str| held_items(m).iter().any(|(s, _)| *s == slot);
        let pitch = m.previous_head_pitch + (m.head_pitch - m.previous_head_pitch) * a;
        // What the game keeps for every entity a pack dresses (`variable.*` of vanilla's own
        // resources): the swing, the hands, the head's tilt, and the screens it is not drawn in.
        vec![
            ("attack_time", Value::Num(swing)),
            ("attack_animation_tick", Value::Num(ticks)),
            ("is_paperdoll", Value::bool(super::cem_anim::is_in_gui())),
            ("is_holding_right", Value::bool(holds("main_hand"))),
            ("is_holding_left", Value::bool(holds("off_hand"))),
            ("player_x_rotation", Value::Num(pitch)),
            ("map_face_icon", Value::Num(0.0)),
            ("short_arm_offset_left", Value::Num(0.0)),
            ("short_arm_offset_right", Value::Num(0.0)),
            ("bob_animation", Value::Num(0.0)),
            ("has_trim", Value::Num(0.0)),
            ("rail_rotation", Value::Num(0.0)),
            ("liedownamount", Value::Num(0.0)),
            ("is_enchanted", Value::Num(0.0)),
        ]
    }

    fn query(&mut self, name: &str, args: &[Value]) -> Value {
        if let Some(v) = super::bedrock_debug::query(name) {
            return Value::Num(v);
        }
        let m = self.m;
        if let Some(v) = world_query(m, name, args) {
            return v;
        }
        let a = self.alpha;
        let lerp = |p: f32, c: f32| p + (c - p) * a;
        match name {
            "life_time" => Value::Num((m.age as f32 + a) / 20.0),
            "is_in_ui" => Value::bool(super::cem_anim::is_in_gui()),
            "delta_time" => Value::Num(self.dt),
            "frame_alpha" => Value::Num(a),
            "is_name_any" => Value::bool(name_matches(m.custom_name.as_deref(), args)),
            "get_name" => Value::Str(m.custom_name.as_deref().unwrap_or("").into()),
            "is_baby" => Value::bool(m.is_baby()),
            "is_on_ground" => Value::bool(m.on_ground),
            "is_in_water" | "is_in_contact_with_water" => Value::bool(m.fluids.in_water),
            // A player is swimming in the swimming pose (sprinting through water) or anywhere in
            // water, which is what plays the pack's swim without the sprint; any other mob swims
            // in water.
            "is_swimming" => Value::bool(
                m.fluids.in_water || m.extra.avatar_pose == crate::pose::Pose::Swimming,
            ),
            "is_alive" => Value::bool(m.alive()),
            "health" => Value::Num(m.health),
            "max_health" => Value::Num(m.kind.max_health),
            "hurt_time" => Value::Num(m.hurt_time as f32),
            "is_on_fire" => Value::bool(m.fire_ticks > 0),
            "is_riding" => Value::bool(m.extra.passenger),
            // A mob the server runs has no target here: one drawing a bow or crossbow has one.
            "has_target" => Value::bool(m.ai.target.is_some() || m.extra.using_item > 0),
            "modified_move_speed" => Value::Num(lerp(m.previous_limb_speed, m.limb_speed).min(1.0)),
            // Blocks a second. A player's own velocity is not the game's to track (the client
            // drives it), so what it moved in the last tick counts too.
            "ground_speed" => {
                let v = m.velocity;
                let (dx, dz) = (m.position.x - m.previous.x, m.position.z - m.previous.z);
                let by_velocity = (v.x * v.x + v.z * v.z).sqrt();
                let by_moving = (dx * dx + dz * dz).sqrt();
                let flat = by_velocity.max(by_moving);
                // Swimming leans the body along the way it goes (`-asin(vertical_speed /
                // ground_speed)` is the pitch only if the speed counts the climb too).
                let speed = if m.extra.avatar_pose == crate::pose::Pose::Swimming {
                    let dy = m.position.y - m.previous.y;
                    let by_moving = (dx * dx + dy * dy + dz * dz).sqrt();
                    let by_velocity = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
                    by_velocity.max(by_moving)
                } else {
                    flat
                };
                Value::Num(speed as f32 * 20.0)
            }
            "vertical_speed" => {
                let (v, d) = (m.velocity.y, m.position.y - m.previous.y);
                // Gravity keeps pulling a mob on the ground (-0.0784 a tick, -1.57 a second) with
                // nothing moving it: that is not a fall to the packs.
                let speed = if m.on_ground && v < 0.0 {
                    d
                } else if v.abs() >= d.abs() {
                    v
                } else {
                    d
                };
                Value::Num(speed as f32 * 20.0)
            }
            "body_y_rotation" => Value::Num(wrap_degrees(lerp(m.previous_body_yaw, m.body_yaw))),
            // The head's yaw in the world, as `body_y_rotation` is the body's: the packs take
            // `head_y_rotation - target_y_rotation` for the body's.
            "head_y_rotation" => Value::Num(wrap_degrees(lerp(m.previous_head_yaw, m.head_yaw))),
            "head_x_rotation" => Value::Num(lerp(m.previous_head_pitch, m.head_pitch)),
            // The mount's pose a rider takes (`passengers.rs` gives it the vehicle's yaw): 0
            // off one.
            "ride_body_y_rotation" if m.extra.passenger => {
                Value::Num(wrap_degrees(lerp(m.previous_body_yaw, m.body_yaw)))
            }
            "ride_head_y_rotation" if m.extra.passenger => {
                Value::Num(wrap_degrees(lerp(m.previous_head_yaw, m.head_yaw)))
            }
            "ride_head_x_rotation" if m.extra.passenger => {
                Value::Num(lerp(m.previous_head_pitch, m.head_pitch))
            }
            "ride_body_x_rotation" | "ride_body_y_rotation" | "ride_head_x_rotation"
            | "ride_head_y_rotation" => Value::Num(0.0),
            "position" => {
                let axis = args.first().map_or(0.0, Value::num) as usize;
                Value::Num([m.position.x, m.position.y, m.position.z][axis.min(2)] as f32)
            }
            "distance_from_camera" => {
                let [x, y, z] = self.from_camera;
                Value::Num((x * x + y * y + z * z).sqrt())
            }
            "rotation_to_camera" => {
                let (pitch, yaw) = rotation_to_camera(self.from_camera);
                Value::Num(if args.first().map_or(0.0, Value::num) as i32 == 0 { pitch } else { yaw })
            }
            "camera_rotation" => {
                let (yaw, pitch) = camera_rotation();
                Value::Num(if args.first().map_or(0.0, Value::num) as i32 == 0 { pitch } else { yaw })
            }
            "swell_amount" => Value::Num(if m.max_swell > 0 {
                (lerp(m.previous_swell as f32, m.swell as f32) / m.max_swell as f32).clamp(0.0, 1.0)
            } else {
                0.0
            }),
            "is_item_name_any"
            | "get_equipped_item_name"
            | "equipped_item_any_tag"
            | "is_item_equipped" => {
                let item = item_at(m, args.first());
                let rest = || {
                    args.iter().skip(1).filter_map(|v| match v {
                        Value::Str(s) => {
                            Some(s.trim_start_matches("minecraft:").to_ascii_lowercase())
                        }
                        _ => None,
                    })
                };
                match name {
                    "get_equipped_item_name" => Value::Str(
                        item.as_deref()
                            .map_or("", |i| i.trim_start_matches("minecraft:"))
                            .into(),
                    ),
                    "is_item_equipped" => Value::bool(item.is_some()),
                    "equipped_item_any_tag" => Value::bool(item.as_deref().is_some_and(|i| {
                        args.iter()
                            .skip(1)
                            .any(|v| matches!(v, Value::Str(t) if item_has_tag(i, t)))
                    })),
                    _ => Value::bool(item.as_deref().is_some_and(|i| {
                        let short = i.trim_start_matches("minecraft:").to_ascii_lowercase();
                        rest().any(|n| n == short)
                    })),
                }
            }
            // The walk cycle: Java's limb position and speed are the numbers a Bedrock walk
            // animation reads as `modified_distance_moved` and `modified_move_speed`.
            "modified_distance_moved" | "distance_moved" => {
                Value::Num(m.limb_position - m.limb_speed * (1.0 - a))
            }
            // The head's turn and tilt from the body, in degrees.
            "target_y_rotation" => Value::Num(wrap_degrees(
                lerp(m.previous_head_yaw, m.head_yaw) - lerp(m.previous_body_yaw, m.body_yaw),
            )),
            "target_x_rotation" => Value::Num(lerp(m.previous_head_pitch, m.head_pitch)),
            "body_x_rotation" => Value::Num(0.0),
            "yaw_speed" => Value::Num(wrap_degrees(m.body_yaw - m.previous_body_yaw) * 20.0),
            // A swing lasts a third of a second in the game that made these packs.
            "base_swing_duration" | "modified_swing_duration" => Value::Num(0.3),
            "is_delayed_attacking" => {
                Value::bool(m.kind.id == "minecraft:iron_golem" && m.extra.anim_ticks > 0)
            }
            "death_ticks" => Value::Num(m.death_time as f32),
            "is_in_lava" => Value::bool(m.fluids.in_lava),
            "head_is_in_water" => Value::bool(m.fluids.eye_in_water),
            "is_gliding" => Value::bool(m.extra.fall_flying),
            "is_jumping" => Value::bool(!m.on_ground && m.velocity.y > 0.0),
            "model_scale" => Value::Num(1.0),
            // The world's clock is not the mob's to know: noon.
            "time_of_day" => Value::Num(0.5),
            "is_sheared" => Value::bool(m.extra.sheared),
            "is_powered" | "is_charged" => Value::bool(if m.species() == super::Species::Blaze {
                m.extra.flag
            } else {
                m.extra.charged
            }),
            "is_sitting" => Value::bool(m.extra.sitting),
            "sit_amount" => Value::Num(if m.extra.sitting { 1.0 } else { 0.0 }),
            "is_tamed" => Value::bool(m.extra.tame),
            "is_saddled" => Value::bool(m.extra.saddled || m.extra.saddle_stack.is_some()),
            "is_chested" => Value::bool(m.extra.chested),
            "is_sneaking" | "is_crouching" => Value::bool(m.extra.crouching),
            "is_sprinting" => Value::bool(m.extra.sprinting),
            "is_angry" => Value::bool(m.extra.anger > 0),
            "is_wall_climbing" => Value::bool(m.climbing),
            "is_carrying_block" => Value::bool(m.extra.carried.is_some()),
            "has_rider" => Value::bool(m.rider.is_some()),
            "is_using_item" => Value::bool(m.extra.using_item > 0),
            // Ticks left of the item in use, as Bedrock counts: 0 when none is in use.
            "main_hand_item_use_duration" => {
                Value::Num(if m.extra.using_item > 0 && !m.extra.using_off_hand {
                    (max_use_ticks(m.extra.main_hand.as_deref()) - m.extra.using_item as f32)
                        .max(1.0)
                } else {
                    0.0
                })
            }
            "main_hand_item_max_duration" => {
                Value::Num(max_use_ticks(m.extra.main_hand.as_deref()))
            }
            // Seconds left of the item in use.
            "item_remaining_use_duration" => {
                let off = matches!(args.first(), Some(Value::Str(s)) if s.contains("off"));
                let held = if off {
                    &m.extra.off_hand
                } else {
                    &m.extra.main_hand
                };
                let using = m.extra.using_item > 0 && m.extra.using_off_hand == off;
                Value::Num(if using {
                    ((max_use_ticks(held.as_deref()) - m.extra.using_item as f32) / 20.0).max(0.0)
                } else {
                    0.0
                })
            }
            "previous_squish_value" => Value::Num(m.anim.vo[0]),
            "current_squish_value" => Value::Num(m.anim.v[0]),
            "fuse_time" => Value::Num(m.max_swell as f32 / 20.0),
            "wing_flap_position" => Value::Num(lerp(m.anim.flap_o, m.anim.flap)),
            "wing_flap_speed" => Value::Num(lerp(m.anim.flap_speed_o, m.anim.flap_speed)),
            "movement_direction" => {
                let axis = args.first().map_or(0.0, Value::num) as usize;
                let v = [m.velocity.x, m.velocity.y, m.velocity.z];
                let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                Value::Num(if len > 1e-6 {
                    (v[axis.min(2)] / len) as f32
                } else {
                    0.0
                })
            }
            "eye_target_x_rotation" => Value::Num(lerp(m.previous_head_pitch, m.head_pitch)),
            "eye_target_y_rotation" => Value::Num(wrap_degrees(
                lerp(m.previous_head_yaw, m.head_yaw) - lerp(m.previous_body_yaw, m.body_yaw),
            )),
            "has_head_gear" => Value::bool(m.extra.head.is_some()),
            "equipment_count" => {
                Value::Num(m.equipment_stacks().iter().filter(|s| s.is_some()).count() as f32)
            }
            "has_any_leashed_entity_of_type" => Value::bool(m.extra.leash.is_some()),
            "armor_texture_slot" => {
                let slot = args.first().map_or(0.0, Value::num) as usize;
                Value::Num(armor_texture_index(m, slot))
            }
            "armor_color_slot" => {
                let slot = args.first().map_or(0.0, Value::num) as usize;
                let channel = args.get(1).map_or(0.0, Value::num) as usize;
                Value::Num(armor_color(m, slot, channel))
            }
            "overlay_alpha" => Value::Num(if m.hurt_time > 0 { 0.3 } else { 0.0 }),
            "owner_identifier" => Value::Str("".into()),
            "invulnerable_ticks" => Value::Num(m.invulnerable as f32),
            // A villager's look: Bedrock keeps the profession in `variant`, the biome in
            // `mark_variant` and the trade tier in `trade_tier`; a skin is picked per mob.
            "variant" if is_villager(&m.kind.id) => {
                Value::Num(bedrock_profession(m.extra.variant2) as f32)
            }
            "mark_variant" if is_villager(&m.kind.id) => Value::Num(f32::from(m.extra.variant)),
            "trade_tier" if is_villager(&m.kind.id) => {
                Value::Num((i32::from(m.extra.variant3) - 1).clamp(0, 4) as f32)
            }
            "skin_id" if is_villager(&m.kind.id) => Value::Num((m.id % 6) as f32),
            // A pufferfish keeps its puff state (0 small, 1 half, 2 full) apart from its skin.
            "variant" if m.species() == super::Species::Pufferfish => {
                Value::Num(m.extra.size as f32)
            }
            "variant" => Value::Num(f32::from(m.extra.variant)),
            "swelling_dir" => Value::Num(match m.swell.cmp(&m.previous_swell) {
                std::cmp::Ordering::Greater => 1.0,
                std::cmp::Ordering::Less => -1.0,
                std::cmp::Ordering::Equal => 0.0,
            }),
            "is_on_screen" => Value::Num(1.0),
            "is_invisible" => Value::bool(super::trader::is_invisible(m)),
            "is_leashed" => Value::bool(m.extra.leash.is_some()),
            "leashed_entity_count" => Value::Num(f32::from(u8::from(m.extra.leash.is_some()))),
            "is_in_love" => Value::bool(m.extra.in_love > 0),
            "is_moving" => Value::bool(m.velocity.x.abs() + m.velocity.z.abs() > 0.003),
            "position_delta" => {
                let axis = args.first().map_or(0.0, Value::num) as usize;
                Value::Num([m.velocity.x, m.velocity.y, m.velocity.z][axis.min(2)] as f32)
            }
            "walk_distance" => Value::Num(m.limb_position),
            "has_player_rider" => Value::bool(m.rider.is_some()),
            "is_eating" | "is_grazing" => Value::bool(
                (m.species() == super::Species::Sheep && m.extra.anim_ticks > 0)
                    || m.extra.using_item > 0,
            ),
            "is_roaring" => {
                Value::bool(m.species() == super::Species::Ravager && m.extra.anim_ticks > 0)
            }
            "is_stunned" => {
                Value::bool(m.species() == super::Species::Ravager && m.extra.counter > 0)
            }
            "hurt_direction" | "head_roll_angle" | "sleep_rotation" | "roll_counter"
            | "sneeze_counter" | "unhappy_counter" | "shake_angle" | "shake_time" => {
                Value::Num(0.0)
            }
            "swim_amount" => Value::Num(f32::from(u8::from(m.fluids.in_water && !m.on_ground))),
            "lie_amount" => Value::Num(0.0),
            "standing_scale" => Value::Num(if m.species() == super::Species::PolarBear {
                super::polar_bear::standing_scale(m, self.alpha)
            } else {
                0.0
            }),
            "is_celebrating"
            | "is_celebrating_special"
            | "is_admiring"
            | "is_dancing"
            | "is_playing_dead"
            | "is_casting"
            | "is_charging"
            | "is_digging"
            | "is_crawling"
            | "is_standing"
            | "is_interested"
            | "is_scared"
            | "is_stalking"
            | "is_searching"
            | "is_emerging"
            | "is_pregnant"
            | "is_laying_egg"
            | "is_levitating"
            | "is_croaking"
            | "is_sniffing"
            | "is_sonic_boom"
            | "is_ghost"
            | "is_elder"
            | "is_resting"
            | "is_jump_goal_jumping"
            | "has_dash_cooldown"
            | "is_shield_powered"
            | "is_shaking_wetness"
            | "is_eating_mob"
            | "can_damage_nearby_mobs" => Value::Num(0.0),
            "is_sleeping" | "is_avoiding_mobs" | "is_shaking" | "is_riding_any_entity_of_type" => {
                Value::Num(0.0)
            }
            "time_stamp" => Value::Num(m.age as f32),
            "facing_target_to_range_attack" => Value::bool(if m.species() == super::Species::Blaze {
                m.extra.flag || m.ai.target.is_some()
            } else {
                m.extra.using_item > 0
            }),
            "item_is_charged" => Value::bool(m.extra.flag2),
            "blocking" => Value::bool(m.extra.using_item > 0 && m.extra.off_hand.is_some()),
            // The properties vanilla's own entities carry, from the Java mob's state.
            "property" => match args.first() {
                Some(Value::Str(p)) if &**p == "minecraft:armadillo_state" => Value::Str(
                    match m.extra.armadillo.state {
                        super::armadillo::IDLE => "unrolled",
                        super::armadillo::UNROLLING => "rolled_up_unrolling",
                        _ => "rolled_up",
                    }
                    .into(),
                ),
                _ => Value::Null,
            },
            "has_property" => Value::bool(matches!(
                args.first(),
                Some(Value::Str(p)) if &**p == "minecraft:armadillo_state"
            )),
            // The stack size of what is held: one when there is something.
            "count" => Value::Num(f32::from(u8::from(item_at(m, args.first()).is_some()))),
            "is_onfire" => Value::bool(m.fire_ticks > 0),
            "on_fire_time" => Value::Num(m.fire_ticks.max(0) as f32 / 20.0),
            "is_in_water_or_rain" => Value::bool(m.fluids.in_water),
            "is_fire_immune" => Value::bool(m.kind.flags.fire_immune),
            "has_owner" => Value::bool(m.extra.tame),
            "has_gravity" | "has_collision" | "can_walk" | "can_swim" => Value::Num(1.0),
            "has_cape" => Value::bool(m.cape.is_some()),
            // The slot counts 0 head, 1 chest, 2 legs, 3 feet, 4 body.
            "has_armor_slot" => {
                let slot = args.first().map_or(0.0, Value::num) as usize;
                Value::bool(m.equipment_stacks().get(slot).is_some_and(|s| s.is_some()))
            }
            "armor_material_slot" => {
                let slot = args.first().map_or(0.0, Value::num) as usize;
                Value::Num(armor_texture_index(m, slot))
            }
            "armor_damage_slot" => Value::Num(0.0),
            // 0 up to the first distance from the camera, 1 from the second, linear between.
            "camera_distance_range_lerp" => {
                let [x, y, z] = self.from_camera;
                let d = (x * x + y * y + z * z).sqrt();
                let (lo, hi) = (
                    args.first().map_or(0.0, Value::num),
                    args.get(1).map_or(1.0, Value::num),
                );
                Value::Num(if hi > lo { ((d - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 0.0 })
            }
            "get_level_seed_based_fraction" => {
                Value::Num(((m.id.wrapping_mul(2654435761)) >> 8 & 0xFFFF) as f32 / 65536.0)
            }
            "average_frame_time" | "last_frame_time" | "minimum_frame_time"
            | "maximum_frame_time" => Value::Num(self.dt),
            "client_max_render_distance" => Value::Num(128.0),
            "client_memory_tier" | "server_memory_tier" => Value::Num(3.0),
            "last_input_mode_is_any" => Value::bool(
                args.iter()
                    .any(|a| matches!(a, Value::Str(s) if s.eq_ignore_ascii_case("keyboard_and_mouse"))),
            ),
            "day" => Value::Num(
                m.extra
                    .surroundings
                    .as_deref()
                    .map_or(0, |s| s.day_time.div_euclid(24000)) as f32,
            ),
            "moon_brightness" => {
                let phase = m
                    .extra
                    .surroundings
                    .as_deref()
                    .map_or(0, |s| s.day_time.div_euclid(24000).rem_euclid(8));
                Value::Num([1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75][phase as usize])
            }
            other => common_query(other, args),
        }
    }
}

/// What `m` holds or wears in the slot a query names: `slot.weapon.mainhand`, `main_hand`, 0 (the
/// main hand) or 1 (the off hand).
fn item_at(m: &Mob, slot: Option<&Value>) -> Option<Arc<str>> {
    let slot = match slot? {
        Value::Str(s) => {
            let s = s.to_ascii_lowercase();
            slot_of(&s).or(match s.as_str() {
                "main_hand" => Some("main_hand"),
                "off_hand" => Some("off_hand"),
                "head" => Some("head"),
                "chest" | "torso" => Some("torso"),
                "legs" => Some("legs"),
                "feet" => Some("feet"),
                "body" => Some("body"),
                _ => None,
            })?
        }
        Value::Num(n) if *n < 0.5 => "main_hand",
        Value::Num(_) => "off_hand",
        _ => return None,
    };
    held_items(m)
        .into_iter()
        .find_map(|(s, i)| (s == slot).then_some(i))
}

/// The ticks an item can be used for at most (`query.main_hand_item_max_duration`).
fn max_use_ticks(item: Option<&str>) -> f32 {
    let name = item.map_or("", |i| i.trim_start_matches("minecraft:"));
    match name {
        "crossbow" => 25.0,
        "potion" | "milk_bucket" | "honey_bottle" | "ominous_bottle" => 32.0,
        "dried_kelp" => 16.0,
        "" => 0.0,
        n if item_has_tag(n, "is_food") => 32.0,
        _ => 72000.0,
    }
}

/// What a worn piece in `slot` (0 head, 1 chest, 2 legs, 3 feet, 4 body armour) is made of, as
/// the index a pack's armour texture array uses: a horse's armour 1 leather, 2 iron, 3 gold,
/// 4 diamond; other armour 0 leather, 1 chain, 2 iron, 3 diamond, 4 gold, 5 netherite.
fn armor_texture_index(m: &Mob, slot: usize) -> f32 {
    let stacks = m.equipment_stacks();
    let index = match slot {
        0 => 5,
        1 => 4,
        2 => 3,
        3 => 2,
        _ => 6,
    };
    let Some(stack) = &stacks[index] else {
        return if slot >= 4 { 0.0 } else { -1.0 };
    };
    let name = stack.item.trim_start_matches("minecraft:");
    let kind = |k: &str| name.starts_with(k);
    if slot >= 4 {
        return if kind("leather") {
            1.0
        } else if kind("iron") {
            2.0
        } else if kind("golden") {
            3.0
        } else if kind("diamond") {
            4.0
        } else {
            0.0
        };
    }
    if kind("leather") {
        0.0
    } else if kind("chainmail") {
        1.0
    } else if kind("iron") {
        2.0
    } else if kind("diamond") {
        3.0
    } else if kind("golden") {
        4.0
    } else if kind("netherite") {
        5.0
    } else {
        -1.0
    }
}

/// A worn piece's dye (`query.armor_color_slot(slot, channel)`, channel 0..3 for r, g, b, a):
/// leather's own brown when undyed, white for any other material.
fn armor_color(m: &Mob, slot: usize, channel: usize) -> f32 {
    let stacks = m.equipment_stacks();
    let index = match slot {
        0 => 5,
        1 => 4,
        2 => 3,
        3 => 2,
        _ => 6,
    };
    let Some(stack) = &stacks[index] else {
        return 1.0;
    };
    if channel >= 3 {
        return 1.0;
    }
    let rgb = if stack.item.contains("leather") {
        stack
            .data()
            .and_then(|d| d.dyed_color)
            .unwrap_or(0xA0_65_40)
    } else {
        0x_FF_FF_FF
    };
    ((rgb >> (16 - 8 * channel as i32)) & 255) as f32 / 255.0
}

/// Whether the item carries the tag a pack asks about. A pack's own tags (`oreville:is_sword`)
/// come from its behaviour pack, which is not here: they read as the kind of item their name says.
fn item_has_tag(item: &str, tag: &str) -> bool {
    let path = item.rsplit(':').next().unwrap_or(item).to_ascii_lowercase();
    let tag = tag.rsplit(':').next().unwrap_or(tag).to_ascii_lowercase();
    let ends = |w: &str| path == w || path.ends_with(&format!("_{w}"));
    const GEAR: [&str; 10] = [
        "sword",
        "axe",
        "pickaxe",
        "shovel",
        "hoe",
        "spear",
        "helmet",
        "chestplate",
        "leggings",
        "boots",
    ];
    if let Some(tier) = tag.strip_suffix("_tier") {
        return path
            .strip_prefix(tier)
            .and_then(|r| r.strip_prefix('_'))
            .is_some_and(|r| GEAR.contains(&r));
    }
    let Some(word) = tag.strip_prefix("is_") else {
        return match tag.as_str() {
            "harness" => ends("harness"),
            "music_disc" => path.starts_with("music_disc"),
            "sign" => ends("sign") && !path.contains("hanging"),
            "hanging_sign" => ends("hanging_sign"),
            "nautilus_armor" => ends("nautilus_armor"),
            _ => false,
        };
    };
    match word {
        "armor" => ["helmet", "chestplate", "leggings", "boots"]
            .iter()
            .any(|w| ends(w)),
        "tool" => [
            "sword", "axe", "pickaxe", "shovel", "hoe", "spear", "shears",
        ]
        .iter()
        .any(|w| ends(w)),
        "soup" => ends("soup") || ends("stew"),
        "milk" => path == "milk_bucket",
        "drink" => {
            ends("potion")
                || path == "milk_bucket"
                || ends("bottle") && !path.contains("experience")
        }
        "horn" => path == "goat_horn",
        "readable" => ends("book"),
        "throwable" => matches!(
            path.as_str(),
            "snowball"
                | "egg"
                | "ender_pearl"
                | "wind_charge"
                | "splash_potion"
                | "lingering_potion"
                | "experience_bottle"
        ),
        "seeds" => path.ends_with("seeds"),
        "petals" => path.ends_with("petals"),
        "on_a_stick" => path.ends_with("_on_a_stick"),
        "rod" => path.ends_with("_rod") || path == "rod",
        w => ends(w),
    }
}

/// What a mob holds or wears, by slot (the names Bedrock's `context.item_slot` takes).
fn held_items(m: &Mob) -> Vec<(&'static str, Arc<str>)> {
    let e = &m.extra;
    [
        ("main_hand", &e.main_hand),
        ("off_hand", &e.off_hand),
        ("head", &e.head),
        ("torso", &e.chest),
        ("legs", &e.legs),
        ("feet", &e.feet),
        // What the body slot holds: a harness, a horse's armour, a llama's carpet.
        ("body", &e.body),
    ]
    .into_iter()
    .filter_map(|(slot, item)| Some((slot, item.clone()?)))
    .collect()
}

/// Items worn on the body that no hand holds as a model: armour, elytra, heads.
fn is_wearable(item: &str) -> bool {
    let name = item.trim_start_matches("minecraft:");
    ["_helmet", "_chestplate", "_leggings", "_boots", "_head", "_skull"]
        .iter()
        .any(|s| name.ends_with(s))
        || matches!(name, "elytra" | "carved_pumpkin" | "turtle_helmet")
}

/// The holder's bone an item in `slot` follows (`query.item_slot_to_bone_name`).
fn slot_bone(slot: &str) -> &'static str {
    match slot {
        "main_hand" => "rightitem",
        "off_hand" => "leftitem",
        "head" => "head",
        "torso" | "body" => "body",
        "legs" => "leftleg",
        _ => "leftleg",
    }
}

/// The slot a `slot.weapon.mainhand`-style name means.
fn slot_of(name: &str) -> Option<&'static str> {
    Some(match name {
        "slot.weapon.mainhand" => "main_hand",
        "slot.weapon.offhand" => "off_hand",
        "slot.armor.head" => "head",
        "slot.armor.chest" => "torso",
        "slot.armor.legs" => "legs",
        "slot.armor.body" => "body",
        "slot.armor.feet" => "feet",
        _ => return None,
    })
}

/// Answers an attachable's expressions: the holder's own answers, plus what concerns the item.
struct AttachHost<'a> {
    inner: MobHost<'a>,
    slot: &'static str,
    /// The holder's variables, for `context.owning_entity->variable.x`.
    owner: &'a horizon_bedrock::molang::Vars,
}

impl Host for AttachHost<'_> {
    fn variables(&mut self) -> Vec<(&'static str, Value)> {
        self.inner.variables()
    }
    fn query(&mut self, name: &str, args: &[Value]) -> Value {
        match name {
            "is_attached" => Value::Num(1.0),
            "get_animation_frame" => {
                let m = self.inner.m;
                let slot = self.slot;
                let item = held_items(m)
                    .into_iter()
                    .find_map(|(s, i)| (s == slot).then_some(i));
                Value::Num(item.map_or(0.0, |i| item_frame(m, &i)))
            }
            "item_slot_to_bone_name" => {
                let slot = match args.first() {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => self.slot.to_owned(),
                };
                Value::Str(slot_bone(&slot).into())
            }
            _ => self.inner.query(name, args),
        }
    }
    fn context(&mut self, name: &str) -> Value {
        match name {
            "item_slot" => Value::Str(self.slot.into()),
            "is_first_person" => Value::Num(0.0),
            "owning_entity" => Value::Entity(u64::from(self.inner.m.id)),
            _ => Value::Null,
        }
    }
    fn entity_var(&mut self, _entity: u64, var: horizon_bedrock::molang::Sym) -> Value {
        self.owner.get(&var).cloned().unwrap_or_default()
    }
    fn entity_query(&mut self, _entity: u64, name: &str, args: &[Value]) -> Value {
        self.inner.query(name, args)
    }
}

/// The packs' mob look: everything the bundle holds and what is running.
#[derive(Default)]
pub struct BedrockMobs {
    bundle: Bundle,
    parts: Mutex<HashMap<String, Option<Arc<Built>>>>,
    states: Mutex<HashMap<u32, MobState>>,
    queue: Mutex<Vec<Emitted>>,
    emitters: Mutex<Vec<Active>>,
    clock: Mutex<Option<Instant>>,
    /// The attachables' animations, by mob and slot.
    attached: Mutex<HashMap<(u32, &'static str), AttState>>,
    /// Item (lower case) to the attachables that dress it.
    item_index: HashMap<String, Vec<String>>,
    /// A geometry's bone bindings, parsed once.
    bindings: Mutex<HashMap<String, Arc<Vec<(String, horizon_bedrock::molang::Program)>>>>,
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How long a looping effect bound to a mob goes on after its animation last asked for it.
/// How long a state-held looping effect outlasts the state's last frame.
const HOLD_GRACE: std::time::Duration = std::time::Duration::from_millis(400);
const LOOP_GRACE: std::time::Duration = std::time::Duration::from_millis(2500);

/// How long a mob's running animations are kept without a draw.
const KEEP_SECONDS: u64 = 5;

/// Java entities Bedrock calls something else (the identifier the other game gives the same mob).
const BEDROCK_IDS: &[(&str, &str)] = &[
    ("minecraft:villager", "minecraft:villager_v2"),
    ("minecraft:zombie_villager", "minecraft:zombie_villager_v2"),
    ("minecraft:zombified_piglin", "minecraft:zombie_pigman"),
    ("minecraft:evoker", "minecraft:evocation_illager"),
    ("minecraft:evoker_fangs", "minecraft:evocation_fang"),
    ("minecraft:illusioner", "minecraft:illusioner"),
    ("minecraft:end_crystal", "minecraft:ender_crystal"),
    ("minecraft:experience_bottle", "minecraft:xp_bottle"),
    ("minecraft:experience_orb", "minecraft:xp_orb"),
    ("minecraft:eye_of_ender", "minecraft:eye_of_ender_signal"),
    ("minecraft:trident", "minecraft:thrown_trident"),
    ("minecraft:wind_charge", "minecraft:wind_charge_projectile"),
    ("minecraft:firework_rocket", "minecraft:fireworks_rocket"),
    ("minecraft:tropical_fish", "minecraft:tropicalfish"),
    ("minecraft:snow_golem", "minecraft:snow_golem"),
    ("minecraft:potion", "minecraft:splash_potion"),
    ("minecraft:ender_pearl", "minecraft:ender_pearl"),
    ("minecraft:chest_minecart", "minecraft:chest_minecart"),
    ("minecraft:furnace_minecart", "minecraft:minecart"),
    ("minecraft:spawner_minecart", "minecraft:minecart"),
];

fn bedrock_id(java: &str) -> Option<&'static str> {
    BEDROCK_IDS
        .iter()
        .find_map(|(j, b)| (*j == java && j != b).then_some(*b))
}

/// The invisible part above a geometry's bones.
const ROOT: &str = "<root>";

/// Draws a face's texels from inside its rectangle: a pack's face edges sit on texel borders (or
/// between texels, with fractional sizes), and a sample taken right on one picks the neighbour's
/// colour, a thin line along the edge. Each edge moves a twentieth of a texel inward (more than the half pixel a multisampled edge reaches past it up close).
fn inset_uv(vertices: &mut [[f32; 5]; 4], texture: [f32; 2]) {
    const INSET: f32 = 0.05;
    for axis in 0..2 {
        let size = texture[axis].max(1.0);
        let i = 3 + axis;
        let (lo, hi) = vertices
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v[i]), hi.max(v[i])));
        let span = (hi - lo) * size;
        let step = INSET.min(span * 0.25) / size;
        let mid = (lo + hi) / 2.0;
        for v in vertices.iter_mut() {
            if v[i] <= lo + f32::EPSILON {
                v[i] += step;
            } else if v[i] >= hi - f32::EPSILON {
                v[i] -= step;
            } else if v[i] < mid {
                v[i] += step;
            } else {
                v[i] -= step;
            }
        }
    }
}

fn bake(g: &Geometry) -> Built {
    fn convert(p: geometry::Part, names: &mut Vec<String>, texture: [f32; 2]) -> Part {
        let name = p.name.to_ascii_lowercase();
        names.push(name.clone());
        let polygons = if p.never_render {
            Vec::new()
        } else {
            p.quads
                .into_iter()
                .map(|mut q| {
                    inset_uv(&mut q.vertices, texture);
                    Polygon {
                        normal: q.normal,
                        vertices: q.vertices,
                        cull_back: false,
                    }
                })
                .collect()
        };
        Part {
            name,
            pose: p.pose,
            polygons,
            children: p
                .children
                .into_iter()
                .map(|c| convert(c, names, texture))
                .collect(),
        }
    }
    let mut names = Vec::new();
    // The tree's roots become children of one invisible root part.
    let children: Vec<Part> = g
        .to_tree()
        .into_iter()
        .map(|p| convert(p, &mut names, g.texture_size))
        .collect();
    // Not `root`: a pack's own `root` bone has its animation, and a shared name would apply it twice.
    let root = Part {
        name: ROOT.into(),
        pose: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        polygons: Vec::new(),
        children,
    };
    names.push(ROOT.into());
    Built {
        part: Arc::new(root),
        names,
        geometry: g.id.clone(),
    }
}

impl BedrockMobs {
    /// The bundle of the resource root `root` (empty without a Bedrock pack).
    pub fn load(root: &Path) -> BedrockMobs {
        let bundle = Bundle::load(root);
        if super::bedrock_debug::debug() && !bundle.ignored.is_empty() {
            let lines = horizon_bedrock::audit::lines(&bundle.ignored);
            log::info!(
                "bedrock: {} keys the pack carries and the engine ignores:\n{}",
                lines.len(),
                lines.join("\n")
            );
        }
        // The options the player chose for the packs, in the game folder.
        let game = std::env::current_dir().unwrap_or_default();
        horizon_bedrock::settings::install(
            &bundle.settings,
            &horizon_bedrock::settings::Config::read(&game).choices(),
        );
        let mut item_index: HashMap<String, Vec<String>> = HashMap::new();
        for (id, def) in &bundle.attachables {
            item_index.entry(id.clone()).or_default().push(id.clone());
            for (item, _) in &def.items {
                item_index
                    .entry(item.to_ascii_lowercase())
                    .or_default()
                    .push(id.clone());
            }
        }
        for ids in item_index.values_mut() {
            ids.sort();
            ids.dedup();
        }
        BedrockMobs {
            bundle,
            item_index,
            ..Default::default()
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bundle.is_empty()
    }

    pub fn bundle(&self) -> &Bundle {
        &self.bundle
    }

    /// The client entity that dresses the Java entity `id` (`minecraft:villager`): the one with
    /// the same identifier, or the one Bedrock calls it by.
    fn entity_def(&self, id: &str) -> Option<&horizon_bedrock::entity::ClientEntity> {
        let id = id.to_ascii_lowercase();
        self.bundle
            .entities
            .get(&id)
            .or_else(|| self.bundle.entities.get(bedrock_id(&id)?))
    }

    /// Whether the pack draws its own flames on `entity`: it names an effect for fire, so the
    /// game's flame quads stay out of the way.
    pub fn replaces_flames(&self, entity: &str) -> bool {
        !self.bundle.entities.is_empty()
            && self.entity_def(entity).is_some_and(|d| {
                d.particles.keys().any(|k| {
                    let k = k.to_ascii_lowercase();
                    k.contains("fire") || k.contains("flame") || k.contains("burn")
                })
            })
    }

    /// Whether the pack dresses `entity` (`minecraft:zombie`).
    /// How the pack has what `m` holds drawn, as of its last frame (`None` before the first or
    /// when no pack dresses it).
    pub fn held(&self, m: &Mob) -> Option<Held> {
        lock(&self.states)
            .get(&m.id)
            .and_then(|s| s.frame.as_ref())
            .map(|f| f.held)
    }

    pub fn has_entity(&self, entity: &str) -> bool {
        !self.bundle.entities.is_empty()
            && (players_enabled() || !is_avatar(entity))
            && self.entity_def(entity).is_some()
    }

    fn built(&self, geometry: &str) -> Option<Arc<Built>> {
        let mut parts = lock(&self.parts);
        parts
            .entry(geometry.to_owned())
            .or_insert_with(|| {
                self.bundle
                    .geometries
                    .get(geometry)
                    .map(|g| Arc::new(bake(g)))
            })
            .clone()
    }

    /// Every texture the entity may wear, as atlas names (so the game can ask for them).
    pub fn textures_of(&self, entity: &str) -> Vec<String> {
        self.entity_def(entity)
            .map(|d| d.textures.values().map(|t| atlas_name(t)).collect())
            .unwrap_or_default()
    }

    /// This frame of `m`, run once per `(mob, age + alpha)`; `None` when the pack does not dress
    /// the mob.
    pub fn frame(&self, m: &Mob, alpha: f32, from_camera: [f32; 3]) -> Option<Arc<MobFrame>> {
        let avatar = matches!(&*m.kind.id, "minecraft:player" | "minecraft:mannequin");
        if avatar && !players_enabled() {
            return None;
        }
        let def = self.entity_def(&m.kind.id)?;
        let time = m.age as f32 + alpha;
        // A screen's preview (the inventory's player box) keeps its own state, so it neither
        // reads nor replaces the world's frame, and it re-poses every draw: the cursor moves it.
        let gui = super::cem_anim::is_in_gui();
        let key = if gui { m.id | 0x8000_0000 } else { m.id };
        let mut states = lock(&self.states);
        let st = states.entry(key).or_insert_with(|| MobState {
            instance: Instance::new(u64::from(m.id) + 1),
            time,
            frame: None,
            seen: Instant::now(),
            dumped: f32::NEG_INFINITY,
        });
        let since = st.seen.elapsed().as_secs_f32();
        st.seen = Instant::now();
        if !gui
            && st.time == time
            && let Some(f) = &st.frame
        {
            return Some(f.clone());
        }
        let dt = if gui {
            since.clamp(0.0, 0.1)
        } else {
            ((time - st.time) / 20.0).clamp(0.0, 0.1)
        };
        let assets = Assets {
            animations: &self.bundle.library,
            render_controllers: &self.bundle.render_controllers,
            geometries: Some(&self.bundle.geometries),
        };
        let mut host = MobHost {
            m,
            alpha,
            dt,
            from_camera,
        };
        let frame = st.instance.update(def, &assets, &mut host, dt);
        st.time = time;
        if dump_wanted(&m.kind.id)
            && time - st.dumped >= 20.0
            && from_camera.iter().map(|v| v * v).sum::<f32>() < 144.0
        {
            st.dumped = time;
            log::info!(
                "bedrock-dump {}#{} age={} {}",
                m.kind.id,
                m.id,
                m.age,
                dump_line(m, &mut host, &frame, &st.instance.describe(def, &assets)),
            );
            let vars: Vec<String> = super::bedrock_debug::settings()
                .dump_vars
                .iter()
                .map(|v| {
                    let v = v.as_str();
                    format!(
                        "v.{v}={:?}",
                        st.instance.vars.get(&horizon_bedrock::molang::intern(v))
                    )
                })
                .collect();
            if !vars.is_empty() {
                log::info!("bedrock-dump-vars {}#{} {}", m.kind.id, m.id, vars.join(" "));
            }
        }
        let layers: Vec<DrawLayer> = frame
            .layers
            .into_iter()
            .filter_map(|layer| {
                // The player's body is the skin's own: its model and texture are the game's.
                let geometry = if avatar && m.slim && layer.geometry == "geometry.humanoid.custom" {
                    "geometry.humanoid.customslim"
                } else {
                    layer.geometry.as_str()
                };
                let built = self.built(geometry)?;
                let skinned = avatar && layer.texture == "textures/entity/steve";
                Some(DrawLayer {
                    built,
                    texture: if skinned {
                        m.skin.clone().unwrap_or_else(|| m.kind.texture.to_string())
                    } else {
                        atlas_name(&layer.texture)
                    },
                    fallback: horizon_bedrock::convert::java_texture(&layer.texture),
                    layer,
                })
            })
            .collect();
        let pos = [m.position.x, m.position.y, m.position.z];
        let mut queue = lock(&self.queue);
        for e in frame.emits {
            match e {
                Emit::Sound(event) => {
                    let event = if event.contains(':') {
                        event.to_ascii_lowercase()
                    } else {
                        format!("bedrock:{}", event.to_ascii_lowercase())
                    };
                    queue.push(Emitted::Sound { event, at: pos });
                }
                Emit::Hold { effect, locator } => {
                    if !m.alive() {
                        continue;
                    }
                    let offset = locator
                        .as_deref()
                        .and_then(|l| {
                            layers.iter().find_map(|d| {
                                self.bundle.geometries.get(&d.layer.geometry)?.locator(l)
                            })
                        })
                        .map_or([0.0; 3], |l| l.1.offset.map(|v| f64::from(v) / 16.0));
                    self.hold_particle(
                        &effect,
                        attach_point(pos, offset, m.body_yaw),
                        m.body_yaw,
                        m.is_baby(),
                        m.custom_name.clone(),
                        box_size(m),
                        Some(Follow {
                            mob: m.id,
                            locator,
                            offset,
                        }),
                    );
                }
                Emit::Particle { effect, locator } => {
                    let offset = locator
                        .as_deref()
                        .and_then(|l| {
                            layers.iter().find_map(|d| {
                                self.bundle.geometries.get(&d.layer.geometry)?.locator(l)
                            })
                        })
                        .map_or([0.0; 3], |l| l.1.offset.map(|v| f64::from(v) / 16.0));
                    queue.push(Emitted::Particle {
                        effect,
                        at: attach_point(pos, offset, m.body_yaw),
                        yaw: m.body_yaw,
                        baby: m.is_baby(),
                        name: m.custom_name.clone(),
                        size: box_size(m),
                        follow: Some(Follow {
                            mob: m.id,
                            locator,
                            offset,
                        }),
                    });
                }
            }
        }
        let scale = frame.scale * builtin_scale(&m.kind.id, m.is_baby());
        let mut held = Held {
            hidden: frame.hide_held_items,
            scale: frame.held_item_scale,
            full_bright: frame.held_item_ignores_lighting,
            main_replaced: false,
            off_replaced: false,
        };
        self.parent_setups(m, alpha, dt, from_camera, &mut st.instance.vars);
        let mut attached = self.attachments(m, alpha, dt, from_camera, &st.instance.vars);
        let dressed = |slot: &str| attached.iter().any(|a| a.slot == slot && !a.layers.is_empty());
        held.main_replaced = dressed("main_hand");
        held.off_replaced = dressed("off_hand");
        if held.hidden {
            attached.retain(|a| !matches!(a.slot, "main_hand" | "off_hand"));
        }
        let out = Arc::new(MobFrame {
            layers,
            pose: frame.pose,
            scale,
            scale_axes: frame.scale_axes,
            held,
            attached,
        });
        st.frame = Some(out.clone());
        drop(queue);
        // The effects attached to this mob go where it is.
        for a in lock(&self.emitters).iter_mut() {
            if let Some(f) = &a.follow
                && f.mob == m.id
            {
                // A dead mob burns no more.
                if !m.alive() && a.emitter.is_looping() {
                    a.emitter.stop();
                }
                let at = attach_point(pos, f.offset, m.body_yaw);
                a.emitter
                    .set_transform([at[0] as f32, at[1] as f32, at[2] as f32], m.body_yaw);
            }
        }
        Some(out)
    }

    /// The look of what `m` holds and wears that the pack dresses, one entry per slot.
    fn attachments(
        &self,
        m: &Mob,
        alpha: f32,
        dt: f32,
        from_camera: [f32; 3],
        owner: &horizon_bedrock::molang::Vars,
    ) -> Vec<Attached> {
        if self.bundle.attachables.is_empty() {
            return Vec::new();
        }
        let held = held_items(m);
        let mut out = Vec::new();
        let mut states = lock(&self.attached);
        states.retain(|(mob, slot), s| {
            (*mob != m.id || held.iter().any(|(h, _)| h == slot))
                && s.seen.elapsed().as_secs() < KEEP_SECONDS
        });
        let assets = Assets {
            animations: &self.bundle.library,
            render_controllers: &self.bundle.render_controllers,
            geometries: Some(&self.bundle.geometries),
        };
        let time = m.age as f32 + alpha;
        for (slot, item) in &held {
            let item_lc = item.to_ascii_lowercase();
            // Armour is dressed on its slot; in a hand it is an item like any other.
            if matches!(*slot, "main_hand" | "off_hand") && is_wearable(&item_lc) {
                continue;
            }
            let Some(candidates) = self.item_index.get(&item_lc) else {
                continue;
            };
            let mut host = AttachHost {
                inner: MobHost {
                    m,
                    alpha,
                    dt,
                    from_camera,
                },
                slot,
                owner,
            };
            let chosen = self.choose_attachable(candidates, &item_lc, &mut host);
            let Some(id) = chosen else { continue };
            let Some(def) = self.bundle.attachables.get(id) else {
                continue;
            };
            let st = states.entry((m.id, slot)).or_insert_with(|| AttState {
                id: id.clone(),
                instance: Instance::new(u64::from(m.id) * 8 + 3),
                time,
                seen: Instant::now(),
            });
            if st.id != *id {
                *st = AttState {
                    id: id.clone(),
                    instance: Instance::new(u64::from(m.id) * 8 + 3),
                    time,
                    seen: Instant::now(),
                };
            }
            st.seen = Instant::now();
            st.time = time;
            let frame = st.instance.update(def, &assets, &mut host, dt);
            let layers: Vec<DrawLayer> = frame
                .layers
                .into_iter()
                .filter_map(|layer| {
                    let Some(built) = self.built(&layer.geometry) else {
                        if super::bedrock_debug::debug() {
                            super::render::log_bedrock_once(format!("{}/{}/{}", m.kind.id, slot, layer.geometry), || {
                                format!(
                                    "{}: {slot} layer {} ({}) dropped: the geometry is not in the pack",
                                    m.kind.id, layer.geometry, layer.texture
                                )
                            });
                        }
                        return None;
                    };
                    Some(DrawLayer {
                        built,
                        texture: atlas_name(&layer.texture),
                        fallback: horizon_bedrock::convert::java_texture(&layer.texture),
                        layer,
                    })
                })
                .collect();
            let mut binds = HashMap::new();
            let mut rng = horizon_bedrock::molang::Rng::new(1);
            for d in &layers {
                for (bone, program) in self.bindings_of(&d.layer.geometry).iter() {
                    let v = program.eval(&mut horizon_bedrock::molang::EvalCtx {
                        host: &mut host,
                        vars: &mut st.instance.vars,
                        rng: &mut rng,
                        this: Value::Null,
                    });
                    if let Value::Str(to) = v {
                        binds.insert(bone.clone(), to.to_ascii_lowercase());
                    }
                }
            }
            // What is held, with no binding of its own, sits in the hand.
            if binds.is_empty() && matches!(*slot, "main_hand" | "off_hand") {
                for d in &layers {
                    for bone in self
                        .bundle
                        .geometries
                        .get(&d.layer.geometry)
                        .into_iter()
                        .flat_map(|g| g.bones.iter().filter(|b| b.parent.is_none()))
                    {
                        binds.insert(bone.name.to_ascii_lowercase(), slot_bone(slot).to_owned());
                    }
                }
            }
            out.push(Attached {
                slot,
                layers,
                pose: frame.pose,
                binds,
            });
        }
        out
    }

    /// The first attachable of `candidates` whose condition for `item` holds.
    fn choose_attachable<'s>(
        &'s self,
        candidates: &'s [String],
        item_lc: &str,
        host: &mut AttachHost,
    ) -> Option<&'s String> {
        candidates.iter().find(|id| {
            let Some(def) = self.bundle.attachables.get(*id) else {
                return false;
            };
            if id.as_str() == item_lc {
                return true;
            }
            def.items.iter().any(|(i, cond)| {
                i.eq_ignore_ascii_case(item_lc)
                    && cond.as_ref().is_none_or(|c| {
                        let mut vars = horizon_bedrock::molang::Vars::default();
                        let mut rng = horizon_bedrock::molang::Rng::new(1);
                        c.eval(&mut horizon_bedrock::molang::EvalCtx {
                            host: &mut *host,
                            vars: &mut vars,
                            rng: &mut rng,
                            this: Value::Null,
                        })
                        .truthy()
                    })
            })
        })
    }

    /// An attachable's `parent_setup` runs on the holder: it sets the holder's variables (that
    /// the item is held, which arm poses to use) the holder's animations and the item read.
    fn parent_setups(
        &self,
        m: &Mob,
        alpha: f32,
        dt: f32,
        from_camera: [f32; 3],
        holder: &mut horizon_bedrock::molang::Vars,
    ) {
        if self.bundle.attachables.is_empty() {
            return;
        }
        let none = horizon_bedrock::molang::Vars::default();
        let mut rng = horizon_bedrock::molang::Rng::new(1);
        for (slot, item) in &held_items(m) {
            let item_lc = item.to_ascii_lowercase();
            if matches!(*slot, "main_hand" | "off_hand") && is_wearable(&item_lc) {
                continue;
            }
            let Some(candidates) = self.item_index.get(&item_lc) else {
                continue;
            };
            let mut host = AttachHost {
                inner: MobHost {
                    m,
                    alpha,
                    dt,
                    from_camera,
                },
                slot,
                owner: &none,
            };
            let Some(id) = self.choose_attachable(candidates, &item_lc, &mut host) else {
                continue;
            };
            let Some(def) = self.bundle.attachables.get(id) else {
                continue;
            };
            for program in &def.parent_setup {
                program.eval(&mut horizon_bedrock::molang::EvalCtx {
                    host: &mut host,
                    vars: holder,
                    rng: &mut rng,
                    this: Value::Null,
                });
            }
        }
    }

    /// The bones of `geometry` that follow a bone of their holder, with the expression naming it.
    fn bindings_of(&self, geometry: &str) -> Arc<Vec<(String, horizon_bedrock::molang::Program)>> {
        lock(&self.bindings)
            .entry(geometry.to_owned())
            .or_insert_with(|| {
                Arc::new(
                    self.bundle
                        .geometries
                        .get(geometry)
                        .map(|g| {
                            g.bones
                                .iter()
                                .filter_map(|b| {
                                    let program =
                                        horizon_bedrock::molang::parse(b.binding.as_deref()?)
                                            .ok()?;
                                    Some((b.name.to_ascii_lowercase(), program))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                )
            })
            .clone()
    }

    /// Forgets mobs not drawn for a while and returns what the frames set off since the last call.
    pub fn take_emitted(&self) -> Vec<Emitted> {
        let mut states = lock(&self.states);
        states.retain(|_, s| s.seen.elapsed().as_secs() < KEEP_SECONDS);
        // A looping effect an animation no longer asks for stops (a flame once the fire is out).
        for a in lock(&self.emitters).iter_mut() {
            if a.follow.is_some()
                && a.emitter.is_looping()
                && a.requested.elapsed() > if a.held { HOLD_GRACE } else { LOOP_GRACE }
            {
                a.emitter.stop();
            }
        }
        // An effect attached to a mob that is gone goes with it.
        lock(&self.emitters).retain(|a| {
            a.follow
                .as_ref()
                .is_none_or(|f| states.contains_key(&f.mob))
        });
        drop(states);
        std::mem::take(&mut *lock(&self.queue))
    }

    /// Starts the effect `id` at `at`. Unknown effects are left out.
    pub fn spawn_particle(
        &self,
        id: &str,
        at: [f64; 3],
        yaw: f32,
        baby: bool,
        name: Option<String>,
        size: [f32; 2],
        follow: Option<Follow>,
    ) {
        self.start_particle(id, at, yaw, baby, name, size, follow, false);
    }

    /// Keeps `id` running for as long as the controller state that set it off lasts, starting
    /// it again if it ended meanwhile (a mob that left the view and came back).
    #[allow(clippy::too_many_arguments)]
    fn hold_particle(
        &self,
        id: &str,
        at: [f64; 3],
        yaw: f32,
        baby: bool,
        name: Option<String>,
        size: [f32; 2],
        follow: Option<Follow>,
    ) {
        self.start_particle(id, at, yaw, baby, name, size, follow, true);
    }

    #[allow(clippy::too_many_arguments)]
    fn start_particle(
        &self,
        id: &str,
        at: [f64; 3],
        yaw: f32,
        baby: bool,
        name: Option<String>,
        size: [f32; 2],
        follow: Option<Follow>,
        held: bool,
    ) {
        let Some(fx) = self.bundle.effects.get(&id.to_ascii_lowercase()) else {
            return;
        };
        // One effect at a time per mob and locator: an animation that sets it off every loop
        // does not pile them up.
        if let Some(f) = &follow
            && let Some(a) = lock(&self.emitters).iter_mut().find(|a| {
                !a.emitter.finished()
                    && (!held || a.emitter.emitting())
                    && a.follow.as_ref() == Some(f)
                    && std::ptr::eq(a.emitter.effect(), &**fx)
            })
        {
            // Asked for again: the animation that sets it off is still running.
            a.requested = Instant::now();
            a.held |= held;
            return;
        }
        let mut e = Emitter::new(
            fx.clone(),
            (at[0].to_bits() ^ at[2].to_bits().rotate_left(17)) | 1,
        );
        // A one-shot effect is not started again by a state that merely goes on.
        if held && !e.is_looping() {
            return;
        }
        e.set_transform([at[0] as f32, at[1] as f32, at[2] as f32], yaw);
        let mut emitters = lock(&self.emitters);
        // A runaway pack cannot fill memory.
        if emitters.len() < 512 {
            emitters.push(Active {
                emitter: e,
                host: Snapshot {
                    baby,
                    name,
                    age: 0.0,
                    size,
                },
                follow,
                requested: Instant::now(),
                held,
            });
        }
    }

    /// Advances the running effects to now, handing back the sprites to draw and the effects and
    /// sounds they set off in turn.
    pub fn step_particles(&self) -> (Vec<(Arc<str>, Vec<Sprite>, bool)>, Vec<Emitted>) {
        let now = Instant::now();
        let dt = {
            let mut clock = lock(&self.clock);
            let dt = clock
                .map_or(0.0, |t| now.duration_since(t).as_secs_f32())
                .min(0.1);
            *clock = Some(now);
            dt
        };
        let mut emitters = lock(&self.emitters);
        let mut drawn = Vec::new();
        let mut more = Vec::new();
        for a in emitters.iter_mut() {
            a.host.age += dt * 20.0;
            let mut out = Vec::new();
            let carrier = Carrier(a.host.size);
            a.emitter.update(dt, &mut a.host, &carrier, &mut out);
            for s in out {
                match s {
                    Spawn::Effect { effect, at } => more.push(Emitted::Particle {
                        effect,
                        at: [f64::from(at[0]), f64::from(at[1]), f64::from(at[2])],
                        yaw: 0.0,
                        baby: a.host.baby,
                        name: a.host.name.clone(),
                        size: a.host.size,
                        follow: None,
                    }),
                    Spawn::Sound { event, at } => {
                        let event = if event.contains(':') {
                            event.to_ascii_lowercase()
                        } else {
                            format!("bedrock:{}", event.to_ascii_lowercase())
                        };
                        more.push(Emitted::Sound {
                            event,
                            at: [f64::from(at[0]), f64::from(at[1]), f64::from(at[2])],
                        });
                    }
                }
            }
            let sprites = a.emitter.sprites(&mut a.host);
            if !sprites.is_empty() {
                let fx = a.emitter.effect();
                drawn.push((Arc::<str>::from(atlas_name(&fx.texture)), sprites, fx.lit));
            }
        }
        emitters.retain(|a| !a.emitter.finished());
        (drawn, more)
    }

    pub fn particle_count(&self) -> usize {
        lock(&self.emitters).len()
    }
}

/// Applies a Bedrock pose over a game pose (`Poses`).
pub fn apply_pose(pose: &Pose, names: &[String], poses: &mut super::anim::Poses) {
    for name in names {
        let Some(b) = pose.get(name) else { continue };
        let p = poses.get(name);
        let r = geometry::rotation_to_java([
            b.rotation[0] + b.rotation_entity[0],
            b.rotation[1] + b.rotation_entity[1],
            b.rotation[2] + b.rotation_entity[2],
        ]);
        p[0] += b.position[0];
        p[1] -= b.position[1];
        p[2] += b.position[2];
        p[3] += r[0];
        p[4] += r[1];
        p[5] += r[2];
        p[6] *= b.scale[0];
        p[7] *= b.scale[1];
        p[8] *= b.scale[2];
    }
}

/// Camera-facing quads for sprites, into `out`: the same vertices the game's own particles make.
#[allow(clippy::too_many_arguments)]
pub fn sprite_quads(
    sprites: &[Sprite],
    rect: [f32; 4],
    lit: bool,
    camera: glam::DVec3,
    yaw: f32,
    pitch: f32,
    light_at: &dyn Fn([i32; 3]) -> u8,
    out: &mut Vec<crate::mesher::Vertex>,
) {
    use crate::mesher::Vertex;
    let base = glam::Quat::from_rotation_y(std::f32::consts::PI - yaw.to_radians())
        * glam::Quat::from_rotation_x(-pitch.to_radians());
    let (du, dv) = (rect[2] - rect[0], rect[3] - rect[1]);
    for s in sprites {
        let p = glam::DVec3::new(
            f64::from(s.pos[0]),
            f64::from(s.pos[1]),
            f64::from(s.pos[2]),
        );
        let center = (p - camera).as_vec3();
        let rotation = if s.rotation != 0.0 {
            base * glam::Quat::from_rotation_z(s.rotation.to_radians())
        } else {
            base
        };
        let light = if lit {
            let packed = light_at([p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32]);
            (u32::from(packed >> 4) * 16) | (u32::from(packed & 15) * 16) << 8
        } else {
            super::render::FULL_BRIGHT
        };
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u32;
        let color = c(s.color[3]) << 24 | c(s.color[2]) << 16 | c(s.color[1]) << 8 | c(s.color[0]);
        let (u0, v0, u1, v1) = (
            rect[0] + du * s.uv[0],
            rect[1] + dv * s.uv[1],
            rect[0] + du * s.uv[2],
            rect[1] + dv * s.uv[3],
        );
        for (n, uv) in [
            ((1.0, -1.0), [u1, v1]),
            ((1.0, 1.0), [u1, v0]),
            ((-1.0, 1.0), [u0, v0]),
            ((-1.0, -1.0), [u0, v1]),
        ] {
            let v = center + rotation * glam::Vec3::new(n.0 * s.size[0], n.1 * s.size[1], 0.0);
            out.push(Vertex {
                normal: [0; 4],
                pos: v.to_array(),
                uv,
                layer: crate::gfx::TRANSLUCENT_PARTICLE_OVERLAY,
                color,
                light,
                glint_uv: [0.0, 0.0],
            });
        }
    }
}

#[cfg(test)]
mod tests {
    /// Every query the documentation lists has an answer somewhere: the host, the engine's
    /// bone tables, the animation clock, or the Molang builtins. One that reads as nothing is a
    /// gap in the engine, not in a pack.
    #[test]
    fn every_documented_query_has_an_answer() {
        let sources = [
            include_str!("bedrock.rs"),
            include_str!("../../../horizon-bedrock/src/entity.rs"),
            include_str!("../../../horizon-bedrock/src/animation.rs"),
            include_str!("../../../horizon-bedrock/src/molang/eval.rs"),
            include_str!("../../../horizon-bedrock/src/particle.rs"),
        ]
        .concat();
        let missing: Vec<&str> = horizon_bedrock::DOCUMENTED_QUERIES
            .lines()
            .filter(|q| !sources.contains(&format!("\"{q}\"")))
            .collect();
        assert!(missing.is_empty(), "no answer for: {missing:?}");
    }

    #[test]
    fn a_villager_profession_has_bedrocks_number() {
        use super::super::render::PROFESSIONS;
        let at = |name: &str| PROFESSIONS.iter().position(|p| *p == name).unwrap() as u8;
        assert_eq!(bedrock_profession(at("none")), 0);
        assert_eq!(bedrock_profession(at("farmer")), 1);
        assert_eq!(bedrock_profession(at("librarian")), 5);
        assert_eq!(bedrock_profession(at("armorer")), 8);
        assert_eq!(bedrock_profession(at("nitwit")), 14);
    }

    #[test]
    fn bedrock_calls_some_mobs_by_other_names() {
        assert_eq!(
            bedrock_id("minecraft:villager"),
            Some("minecraft:villager_v2")
        );
        assert_eq!(
            bedrock_id("minecraft:zombified_piglin"),
            Some("minecraft:zombie_pigman")
        );
        assert_eq!(bedrock_id("minecraft:zombie"), None);
        assert_eq!(bedrock_id("minecraft:snow_golem"), None);
    }

    #[test]
    fn an_attached_point_turns_with_the_mob() {
        let near = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-9);
        assert!(near(
            attach_point([10.0, 64.0, -5.0], [1.0, 2.0, 3.0], 0.0),
            [11.0, 66.0, -2.0]
        ));
        // A quarter turn: model x and z swap into the world's z and x.
        assert!(near(
            attach_point([0.0, 0.0, 0.0], [1.0, 0.0, 3.0], 90.0),
            [3.0, 0.0, -1.0]
        ));
    }

    use super::*;
    use horizon_bedrock::particle::Facing;

    #[test]
    fn names_are_exact() {
        assert!(name_matches(
            Some("Snowman"),
            &[Value::Str("snowman".into()), Value::Str("Snowman".into())]
        ));
        assert!(!name_matches(
            Some("snowman"),
            &[Value::Str("Snowman".into())]
        ));
        assert!(!name_matches(None, &[Value::Str("x".into())]));
    }

    #[test]
    fn shared_queries() {
        use horizon_bedrock::settings::{Choice, Def, Kind, install};
        let def = |name: &str, kind, default| Def {
            name: name.into(),
            label: String::new(),
            kind,
            options: vec![("a".into(), String::new()), ("b".into(), String::new())],
            default,
            range: None,
        };
        install(
            &[
                def("q:flag", Kind::Toggle, Choice::Flag(true)),
                def("q:mode", Kind::Dropdown, Choice::One("b".into())),
            ],
            &Default::default(),
        );
        let s = |t: &str| Value::Str(t.into());
        assert_eq!(
            common_query("is_pack_setting_enabled", &[s("q:flag")]),
            Value::Num(1.0)
        );
        assert_eq!(
            common_query("is_pack_setting_selected", &[s("q:mode"), s("a")]),
            Value::Num(0.0)
        );
        assert_eq!(
            common_query("is_pack_setting_selected", &[s("q:mode"), s("b")]),
            Value::Num(1.0)
        );
        assert_eq!(
            common_query(
                "in_range",
                &[Value::Num(5.0), Value::Num(1.0), Value::Num(9.0)]
            ),
            Value::Num(1.0)
        );
        assert_eq!(
            common_query(
                "in_range",
                &[Value::Num(10.0), Value::Num(1.0), Value::Num(9.0)]
            ),
            Value::Num(0.0)
        );
        assert_eq!(
            common_query(
                "graphics_mode_is_any",
                &[Value::Str("simple".into()), Value::Str("fancy".into())]
            ),
            Value::Num(1.0)
        );
        assert_eq!(common_query("nothing_like_it", &[]), Value::Null);
    }

    #[test]
    fn wraps() {
        assert_eq!(wrap_degrees(190.0), -170.0);
        assert_eq!(wrap_degrees(-190.0), 170.0);
        assert_eq!(wrap_degrees(720.0), 0.0);
    }

    #[test]
    fn a_held_item_modelled_at_the_origin_lands_on_the_hand() {
        use super::super::render::{bone_matrices, held_anchor, posed};
        let g = geometry::parse_file(
            &horizon_bedrock::jsonc::parse(
                r#"{"format_version":"1.12.0","minecraft:geometry":[{"description":{"identifier":"geometry.t","texture_width":16,"texture_height":16},
                "bones":[{"name":"item","pivot":[0,0,0],"cubes":[{"origin":[-1,0,-1],"size":[2,8,2],"uv":[0,0]}]},
                         {"name":"hand","pivot":[-6,15,1]}]}]}"#,
            )
            .unwrap(),
        )
        .unwrap()
        .remove(0);
        let b = bake(&g);
        let poses = super::super::anim::Poses::new(&b.part);
        let mut matrices = HashMap::new();
        bone_matrices(&b.part, &poses, &mut matrices);
        let (now, _) = matrices["hand"];
        let item = b.part.children.iter().find(|c| c.name == "item").unwrap().pose;
        // The item's origin, drawn the way `draw_attached` does: the holder's anchor, then the
        // item's own pose (in sixteenths, as every pose is).
        let at = (held_anchor(now, 1.0) * posed(glam::Mat4::IDENTITY, item)).transform_point3(glam::Vec3::ZERO);
        let hand = now.transform_point3(glam::Vec3::ZERO);
        assert!((at - hand).length() < 1e-4, "{at:?} vs {hand:?}");
        // And that is where the hand is: 15 sixteenths up, 6 to the side, not blocks away.
        assert!(hand.length() < 1.5, "{hand:?}");
    }

    /// `HORIZON_TEST_PACK=<pack dir> cargo test -p horizon-client-lib --lib -- sword_probe --ignored --nocapture`
    #[test]
    #[ignore = "needs a pack"]
    fn sword_probe() {
        use super::super::render::{bone_matrices, held_anchor, posed};
        let Ok(dir) = std::env::var("HORIZON_TEST_PACK") else { return };
        let pack = horizon_bedrock::pack::Pack::open(std::path::Path::new(&dir)).unwrap();
        let mut geos = HashMap::new();
        let mut builtin = horizon_bedrock::bundle::Bundle::default();
        builtin.add_builtins();
        geos.extend(builtin.geometries);
        for p in pack.paths().map(str::to_owned).collect::<Vec<_>>() {
            if p.starts_with("models/") && p.ends_with(".json") {
                if let Ok(list) = pack.read_json(&p).map_err(|_| ()).and_then(|v| geometry::parse_file(&v).map_err(|_| ())) {
                    for g in list {
                        geos.insert(g.id.to_ascii_lowercase(), g);
                    }
                }
            }
        }
        let player = bake(&geos["geometry.humanoid.custom"]);
        let sword = bake(&geos["geometry.oreville_ans.ccyhyr"]);
        let poses = super::super::anim::Poses::new(&player.part);
        let mut m = HashMap::new();
        bone_matrices(&player.part, &poses, &mut m);
        let (now, _) = m["rightitem"];
        let mut out = Vec::new();
        let item_poses = super::super::anim::Poses::new(&sword.part);
        let anchor = held_anchor(now, 1.0);
        let _ = posed;
        super::super::render::draw_attached(&sword.part, glam::Mat4::IDENTITY, glam::Mat4::IDENTITY, true, &|b| (b == "sword").then_some(anchor), &item_poses, [0.0, 0.0, 1.0, 1.0], 0, 0, 0, &mut out);
        let (mut lo, mut hi) = (glam::Vec3::splat(f32::MAX), glam::Vec3::splat(f32::MIN));
        for v in &out {
            let p = glam::Vec3::from_array(v.pos);
            lo = lo.min(p);
            hi = hi.max(p);
        }
        // The game's frame: y down from the 24 line; above the feet is 1.5 - y.
        eprintln!("sword x {:.2}..{:.2} height above feet {:.2}..{:.2} z {:.2}..{:.2}", lo.x, hi.x, 1.5 - hi.y, 1.5 - lo.y, lo.z, hi.z);
    }

    #[test]
    fn baking_lower_cases_parts_and_keeps_the_tree() {
        let g = geometry::parse_file(
            &horizon_bedrock::jsonc::parse(
                r#"{"format_version":"1.12.0","minecraft:geometry":[{"description":{"identifier":"geometry.t","texture_width":16,"texture_height":16},
                "bones":[{"name":"Body","pivot":[0,12,0],"cubes":[{"origin":[-4,6,-3],"size":[8,6,6],"uv":[0,0]}]},
                         {"name":"Head","parent":"Body","pivot":[0,12,0]}]}]}"#,
            )
            .unwrap(),
        )
        .unwrap()
        .remove(0);
        let b = bake(&g);
        assert_eq!(b.names, ["body", "head", ROOT]);
        assert_eq!(b.part.children[0].name, "body");
        assert_eq!(b.part.children[0].children[0].name, "head");
        assert_eq!(b.part.children[0].polygons.len(), 6);
    }

    #[test]
    fn a_bound_bone_follows_the_holders_bone() {
        use super::super::render::{bone_matrices, draw_attached};
        let quad = Polygon {
            normal: [0.0, 0.0, 1.0],
            vertices: [[0.0; 5]; 4],
            cull_back: false,
        };
        let part = |name: &str, pose: [f32; 9], polygons: Vec<Polygon>, children: Vec<Part>| Part {
            name: name.into(),
            pose,
            polygons,
            children,
        };
        let rest = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        // The holder: an arm at the origin, with a hand 16 units along.
        let hand = part(
            "hand",
            [16.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            vec![],
            vec![],
        );
        let holder = part("arm", rest, vec![], vec![hand]);
        let mut poses = super::super::anim::Poses::new(&holder);
        // The animation lifts the arm by one block.
        poses.get("arm")[1] += 16.0;
        let mut matrices = HashMap::new();
        bone_matrices(&holder, &poses, &mut matrices);
        let (now, was) = matrices["hand"];
        assert!(
            ((now * was.inverse()).transform_point3(glam::Vec3::ZERO) - glam::Vec3::Y).length()
                < 1e-5
        );
        // The attachable: one bone with a quad at its origin, bound to the hand.
        let item = part("item", rest, vec![quad], vec![]);
        let item_poses = super::super::anim::Poses::new(&item);
        let base_of = |bone: &str| (bone == "item").then(|| now * was.inverse());
        let mut out = Vec::new();
        draw_attached(
            &item,
            glam::Mat4::IDENTITY,
            glam::Mat4::IDENTITY,
            true,
            &base_of,
            &item_poses,
            [0.0, 0.0, 1.0, 1.0],
            0,
            0,
            0,
            &mut out,
        );
        assert_eq!(out.len(), 4);
        assert!((out[0].pos[1] - 1.0).abs() < 1e-5, "{:?}", out[0].pos);
        // Unbound bones of a model that binds some are not drawn.
        let mut none = Vec::new();
        draw_attached(
            &item,
            glam::Mat4::IDENTITY,
            glam::Mat4::IDENTITY,
            true,
            &|_| None,
            &item_poses,
            [0.0, 0.0, 1.0, 1.0],
            0,
            0,
            0,
            &mut none,
        );
        assert!(none.is_empty());
    }

    #[test]
    fn a_packs_item_tags_read_as_the_kind_of_item() {
        assert!(item_has_tag("minecraft:iron_sword", "minecraft:is_sword"));
        assert!(item_has_tag("minecraft:iron_sword", "oreville:is_sword"));
        assert!(!item_has_tag("minecraft:iron_axe", "oreville:is_sword"));
        assert!(item_has_tag(
            "minecraft:golden_pickaxe",
            "minecraft:golden_tier"
        ));
        assert!(!item_has_tag(
            "minecraft:iron_pickaxe",
            "minecraft:golden_tier"
        ));
        assert!(item_has_tag(
            "minecraft:carrot_on_a_stick",
            "oreville:is_on_a_stick"
        ));
        assert!(item_has_tag(
            "minecraft:fishing_rod",
            "oreville:is_fishing_rod"
        ));
        assert!(item_has_tag("minecraft:bow", "oreville:is_bow"));
        assert!(!item_has_tag("minecraft:crossbow", "oreville:is_bow"));
        assert!(item_has_tag("minecraft:white_harness", "minecraft:harness"));
    }

    #[test]
    fn slots_name_their_bones() {
        assert_eq!(slot_bone("main_hand"), "rightitem");
        assert_eq!(slot_bone("off_hand"), "leftitem");
        assert_eq!(slot_of("slot.armor.head"), Some("head"));
        assert_eq!(slot_of("slot.nothing"), None);
    }

    #[test]
    fn pose_adds_to_the_rest_pose() {
        let mut library = horizon_bedrock::animation::Library::default();
        library.add_file(
            &horizon_bedrock::jsonc::parse(r#"{"animations":{"animation.t":{"loop":true,"animation_length":1,"bones":{"body":{"rotation":[90,0,0],"position":[1,2,3],"scale":2}}}}}"#).unwrap(),
        );
        let mut vars = horizon_bedrock::molang::Vars::default();
        let mut rng = horizon_bedrock::molang::Rng::new(1);
        let mut host = horizon_bedrock::molang::NoHost;
        let mut env = horizon_bedrock::animation::Env {
            host: &mut host,
            vars: &mut vars,
            rng: &mut rng,
        };
        let mut pose = Pose::default();
        library.animations["animation.t"].sample(
            &horizon_bedrock::animation::AnimState::default(),
            1.0,
            &mut env,
            &mut pose,
        );
        let part = Part {
            name: "body".into(),
            pose: [0.0, 12.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            polygons: Vec::new(),
            children: Vec::new(),
        };
        let mut poses = super::super::anim::Poses::new(&part);
        apply_pose(&pose, &["body".to_owned()], &mut poses);
        let p = poses.pose(&part);
        assert_eq!(&p[..3], &[1.0, 10.0, 3.0]);
        assert!((p[3] - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(&p[6..], &[2.0, 2.0, 2.0]);
    }

    #[test]
    fn sprites_become_camera_facing_quads() {
        let sprite = Sprite {
            pos: [1.0, 2.0, 3.0],
            size: [0.5, 0.5],
            rotation: 0.0,
            uv: [0.0, 0.0, 1.0, 1.0],
            color: [1.0, 0.0, 0.0, 0.5],
            facing: Facing::LookAtXyz,
            velocity: [0.0; 3],
        };
        let mut out = Vec::new();
        sprite_quads(
            &[sprite],
            [0.0, 0.0, 0.5, 0.5],
            false,
            glam::DVec3::new(1.0, 2.0, 0.0),
            0.0,
            0.0,
            &|_| 0,
            &mut out,
        );
        assert_eq!(out.len(), 4);
        assert_eq!(out[0].uv, [0.5, 0.5]);
        assert_eq!(out[0].light, super::super::render::FULL_BRIGHT);
        assert_eq!(out[0].color >> 24, 127);
        // Facing the camera: all four corners at the sprite's depth.
        assert!(out.iter().all(|v| (v.pos[2] - 3.0).abs() < 1e-4));
    }
}
