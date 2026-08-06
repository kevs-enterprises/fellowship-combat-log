//! Mirror types bridging `fellowship_combat_log::event`/`combatants` into the JSON shape
//! handed to JavaScript.
//!
//! `wasm-bindgen` can only describe a `JsValue`-returning export as `any` in the TypeScript it
//! generates, so the precise discriminated-union types consumers see come from hand-written `.ts`
//! declarations (see `ts/types.ts`) instead. This module's job is to make sure the JSON this
//! crate actually serializes matches those declarations byte for byte:
//!
//! - every mirror struct derives `Serialize` with `#[serde(rename_all = "camelCase")]`, so field
//!   names match idiomatic JS instead of the Rust source's `snake_case`;
//! - pure unit-only enums (`ResultTierJson`, `SchoolJson`, ...) serialize under serde's default
//!   (externally tagged) representation, which for a variant with no payload is just the bare
//!   variant name string (`"Hit"`, `"CriticalStrike"`, ...) — exactly a TS string-literal union;
//! - enums that mix unit and data-carrying variants (`CastPhaseJson`, `EncounterPhaseJson`,
//!   `EventBodyJson`) are internally tagged (`#[serde(tag = "type")]`) so JS gets a discriminant
//!   field, with `rename_all_fields = "camelCase"` renaming the *fields* of any struct-like
//!   variant declared directly on the enum without touching the tag values themselves. Newtype
//!   variants that wrap a whole mirror struct (e.g. `EventBodyJson::DamageHeal(DamageHealJson)`)
//!   flatten automatically under internal tagging because the wrapped struct serializes as a map.
//! - `GuidJson` is the one enum wrapping a bare scalar in the real type (`Guid::Player(u32)`).
//!   Internally tagged enums can't flatten a bare scalar, so the mirror gives it a named field
//!   instead (`Player { id: u32 }`), tagged the same way as the other mixed enums.
//!
//! Every mirror type has a `From<&real type>` (or `From<real type>`) conversion below, kept in
//! the same order as the real types in `fellowship_combat_log::event` for easy side-by-side
//! comparison.

use fellowship_combat_log::combatants::Combatant;
use fellowship_combat_log::event::{
    Cast, CastPhase, CombatantInfo, DamageAbsorbed, DamageHeal, DamageHealKind, Death, DeathKind,
    Dispel, DungeonEnd, DungeonStart, Effect, EffectPhase, Encounter, EncounterPhase, Event,
    EventBody, GearPiece, Guid, Interrupt, LoggingStarted, MapChange, Marker, NeckTraitChoice,
    Polarity, ResourceChange, ResultTier, Resurrect, School, WorldMarker, ZoneChange,
};
use fellowship_combat_log::parse::ParseError;
use fellowship_combat_log::timestamp::LogInstant;
use serde::Serialize;
use wasm_bindgen::JsValue;

/// Serialize a mirror value the same way for every export: camelCase-shaped JSON, with an
/// absent `Option` rendered as `null` (rather than `serde_wasm_bindgen`'s default `undefined`) so
/// the JS shape matches ordinary JSON and doesn't depend on a field's absence being distinguished
/// from an explicit null.
pub fn to_js<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_missing_as_null(true);
    value
        .serialize(&serializer)
        .map_err(|error| JsValue::from(js_sys::Error::new(&error.to_string())))
}

/// Render a `ParseError` as the message a thrown `Error` carries. `seq` (the same file line
/// number the caller passed to `parseLine`) is folded in since the error itself doesn't carry it.
pub fn describe_parse_error(seq: u32, error: &ParseError) -> String {
    let detail = match error {
        ParseError::Empty => "the line is empty".to_string(),
        ParseError::TooFewFields { expected, got } => {
            format!("too few fields: expected at least {expected}, got {got}")
        }
        ParseError::BadField { field, reason } => {
            format!("field {field} is not a valid {reason}")
        }
    };
    format!("fellowship-combat-log: failed to parse line {seq}: {detail}")
}

// --- Guid ---

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum GuidJson {
    Player { id: u32 },
    Npc { spawn: u32, template: u32 },
    Environment,
    Unrecognized,
}

impl From<&Guid> for GuidJson {
    fn from(guid: &Guid) -> Self {
        match *guid {
            Guid::Player(id) => GuidJson::Player { id },
            Guid::Npc { spawn, template } => GuidJson::Npc { spawn, template },
            Guid::Environment => GuidJson::Environment,
            Guid::Unrecognized => GuidJson::Unrecognized,
        }
    }
}

// --- unit-only enums: default (externally tagged) representation is a bare string ---

#[derive(Serialize)]
pub enum ResultTierJson {
    Hit,
    CriticalStrike,
    GrievousCriticalStrike,
    Block,
    Parry,
    Dodge,
    Miss,
    None,
}

impl From<ResultTier> for ResultTierJson {
    fn from(value: ResultTier) -> Self {
        match value {
            ResultTier::Hit => ResultTierJson::Hit,
            ResultTier::CriticalStrike => ResultTierJson::CriticalStrike,
            ResultTier::GrievousCriticalStrike => ResultTierJson::GrievousCriticalStrike,
            ResultTier::Block => ResultTierJson::Block,
            ResultTier::Parry => ResultTierJson::Parry,
            ResultTier::Dodge => ResultTierJson::Dodge,
            ResultTier::Miss => ResultTierJson::Miss,
            ResultTier::None => ResultTierJson::None,
        }
    }
}

#[derive(Serialize)]
pub enum SchoolJson {
    Physical,
    Magical,
    None,
}

impl From<School> for SchoolJson {
    fn from(value: School) -> Self {
        match value {
            School::Physical => SchoolJson::Physical,
            School::Magical => SchoolJson::Magical,
            School::None => SchoolJson::None,
        }
    }
}

#[derive(Serialize)]
pub enum DamageHealKindJson {
    AbilityDamage,
    SwingDamage,
    PeriodicDamage,
    Heal,
    PeriodicHeal,
    LifestealHeal,
}

impl From<DamageHealKind> for DamageHealKindJson {
    fn from(value: DamageHealKind) -> Self {
        match value {
            DamageHealKind::AbilityDamage => DamageHealKindJson::AbilityDamage,
            DamageHealKind::SwingDamage => DamageHealKindJson::SwingDamage,
            DamageHealKind::PeriodicDamage => DamageHealKindJson::PeriodicDamage,
            DamageHealKind::Heal => DamageHealKindJson::Heal,
            DamageHealKind::PeriodicHeal => DamageHealKindJson::PeriodicHeal,
            DamageHealKind::LifestealHeal => DamageHealKindJson::LifestealHeal,
        }
    }
}

#[derive(Serialize)]
pub enum EffectPhaseJson {
    Applied,
    Removed,
    Refreshed,
}

impl From<EffectPhase> for EffectPhaseJson {
    fn from(value: EffectPhase) -> Self {
        match value {
            EffectPhase::Applied => EffectPhaseJson::Applied,
            EffectPhase::Removed => EffectPhaseJson::Removed,
            EffectPhase::Refreshed => EffectPhaseJson::Refreshed,
        }
    }
}

#[derive(Serialize)]
pub enum PolarityJson {
    Buff,
    Debuff,
}

impl From<Polarity> for PolarityJson {
    fn from(value: Polarity) -> Self {
        match value {
            Polarity::Buff => PolarityJson::Buff,
            Polarity::Debuff => PolarityJson::Debuff,
        }
    }
}

#[derive(Serialize)]
pub enum DeathKindJson {
    Unit,
    Ally,
}

impl From<DeathKind> for DeathKindJson {
    fn from(value: DeathKind) -> Self {
        match value {
            DeathKind::Unit => DeathKindJson::Unit,
            DeathKind::Ally => DeathKindJson::Ally,
        }
    }
}

// --- DamageHeal ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageHealJson {
    pub kind: DamageHealKindJson,
    pub source: GuidJson,
    pub target: GuidJson,
    pub ability_id: u32,
    pub parent_ability_id: u32,
    pub applied: i64,
    pub absorbed: i64,
    pub overkill: i64,
    pub blocked: i64,
    pub raw: i64,
    pub school: SchoolJson,
    pub result: ResultTierJson,
    pub source_cur_hp: i32,
    pub target_cur_hp: i32,
}

impl From<&DamageHeal> for DamageHealJson {
    fn from(value: &DamageHeal) -> Self {
        DamageHealJson {
            kind: value.kind.into(),
            source: (&value.source).into(),
            target: (&value.target).into(),
            ability_id: value.ability_id,
            parent_ability_id: value.parent_ability_id,
            applied: value.applied,
            absorbed: value.absorbed,
            overkill: value.overkill,
            blocked: value.blocked,
            raw: value.raw,
            school: value.school.into(),
            result: value.result.into(),
            source_cur_hp: value.source_cur_hp,
            target_cur_hp: value.target_cur_hp,
        }
    }
}

// --- Cast / CastPhase ---

#[derive(Serialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum CastPhaseJson {
    Activated,
    CastStart { cast_seconds: f64 },
    CastSuccess,
    CastFail { reason: String },
    ChannelStart { cast_seconds: f64 },
    ChannelSuccess,
    ChannelFail { reason: String },
}

impl From<&CastPhase> for CastPhaseJson {
    fn from(value: &CastPhase) -> Self {
        match value {
            CastPhase::Activated => CastPhaseJson::Activated,
            CastPhase::CastStart { cast_seconds } => CastPhaseJson::CastStart {
                cast_seconds: *cast_seconds,
            },
            CastPhase::CastSuccess => CastPhaseJson::CastSuccess,
            CastPhase::CastFail { reason } => CastPhaseJson::CastFail {
                reason: reason.clone(),
            },
            CastPhase::ChannelStart { cast_seconds } => CastPhaseJson::ChannelStart {
                cast_seconds: *cast_seconds,
            },
            CastPhase::ChannelSuccess => CastPhaseJson::ChannelSuccess,
            CastPhase::ChannelFail { reason } => CastPhaseJson::ChannelFail {
                reason: reason.clone(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CastJson {
    pub phase: CastPhaseJson,
    pub caster: GuidJson,
    pub ability_id: u32,
    pub has_target: bool,
    pub target: GuidJson,
    pub resources: Vec<(u32, f64, f64)>,
}

impl From<&Cast> for CastJson {
    fn from(value: &Cast) -> Self {
        CastJson {
            phase: (&value.phase).into(),
            caster: (&value.caster).into(),
            ability_id: value.ability_id,
            has_target: value.has_target,
            target: (&value.target).into(),
            resources: value.resources.clone(),
        }
    }
}

// --- Effect ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectJson {
    pub phase: EffectPhaseJson,
    pub caster: GuidJson,
    pub target: GuidJson,
    pub effect_id: u32,
    pub duration_seconds: f64,
    pub stacks: i64,
    pub polarity: PolarityJson,
    pub granting_ability_id: u32,
    pub refresher: Option<GuidJson>,
}

impl From<&Effect> for EffectJson {
    fn from(value: &Effect) -> Self {
        EffectJson {
            phase: value.phase.into(),
            caster: (&value.caster).into(),
            target: (&value.target).into(),
            effect_id: value.effect_id,
            duration_seconds: value.duration_seconds,
            stacks: value.stacks,
            polarity: value.polarity.into(),
            granting_ability_id: value.granting_ability_id,
            refresher: value.refresher.as_ref().map(GuidJson::from),
        }
    }
}

// --- ResourceChange ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChangeJson {
    pub source: GuidJson,
    pub owner: GuidJson,
    pub resource_type: u32,
    pub delta: f64,
    pub current: f64,
    pub max: f64,
    pub causing_ability_id: u32,
}

impl From<&ResourceChange> for ResourceChangeJson {
    fn from(value: &ResourceChange) -> Self {
        ResourceChangeJson {
            source: (&value.source).into(),
            owner: (&value.owner).into(),
            resource_type: value.resource_type,
            delta: value.delta,
            current: value.current,
            max: value.max,
            causing_ability_id: value.causing_ability_id,
        }
    }
}

// --- Encounter / EncounterPhase ---

#[derive(Serialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum EncounterPhaseJson {
    Start,
    End { success: bool },
}

impl From<&EncounterPhase> for EncounterPhaseJson {
    fn from(value: &EncounterPhase) -> Self {
        match *value {
            EncounterPhase::Start => EncounterPhaseJson::Start,
            EncounterPhase::End { success } => EncounterPhaseJson::End { success },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EncounterJson {
    pub phase: EncounterPhaseJson,
    pub encounter_id: u32,
    pub bosses: Vec<String>,
}

impl From<&Encounter> for EncounterJson {
    fn from(value: &Encounter) -> Self {
        EncounterJson {
            phase: (&value.phase).into(),
            encounter_id: value.encounter_id,
            bosses: value.bosses.clone(),
        }
    }
}

// --- Gear / CombatantInfo ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GearPieceJson {
    pub item_id: u32,
    pub item_level: u32,
    pub rarity: u32,
    pub temper: (u32, u32),
    pub stats: Vec<(u32, i64)>,
    pub set_bonus_id: Option<u32>,
    pub ability_grants: Vec<(u32, u32)>,
    pub traits: Vec<(u32, u32)>,
    pub gems: Vec<(u32, u32)>,
    pub score: f64,
}

impl From<&GearPiece> for GearPieceJson {
    fn from(value: &GearPiece) -> Self {
        GearPieceJson {
            item_id: value.item_id,
            item_level: value.item_level,
            rarity: value.rarity,
            temper: value.temper,
            stats: value.stats.clone(),
            set_bonus_id: value.set_bonus_id,
            ability_grants: value.ability_grants.clone(),
            traits: value.traits.clone(),
            gems: value.gems.clone(),
            score: value.score,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeckTraitChoiceJson {
    pub trait_id: u32,
    pub selected: bool,
}

impl From<&NeckTraitChoice> for NeckTraitChoiceJson {
    fn from(value: &NeckTraitChoice) -> Self {
        NeckTraitChoiceJson {
            trait_id: value.trait_id,
            selected: value.selected,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatantInfoJson {
    pub ulid: String,
    pub player: GuidJson,
    pub is_recording_player: bool,
    pub hero_id: u32,
    pub item_level: f64,
    pub stat_sheet: Vec<f64>,
    pub talents: Vec<u32>,
    pub gem_power: Vec<f64>,
    pub gear: Vec<Option<GearPieceJson>>,
    pub trait_ranks: Vec<(u32, u32)>,
    pub neck_traits: Vec<NeckTraitChoiceJson>,
}

impl From<&CombatantInfo> for CombatantInfoJson {
    fn from(value: &CombatantInfo) -> Self {
        CombatantInfoJson {
            ulid: value.ulid.clone(),
            player: (&value.player).into(),
            is_recording_player: value.is_recording_player,
            hero_id: value.hero_id,
            item_level: value.item_level,
            stat_sheet: value.stat_sheet.clone(),
            talents: value.talents.clone(),
            gem_power: value.gem_power.clone(),
            gear: value
                .gear
                .iter()
                .map(|piece| piece.as_ref().map(GearPieceJson::from))
                .collect(),
            trait_ranks: value.trait_ranks.clone(),
            neck_traits: value
                .neck_traits
                .iter()
                .map(NeckTraitChoiceJson::from)
                .collect(),
        }
    }
}

// --- DamageAbsorbed / Interrupt / Dispel ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DamageAbsorbedJson {
    pub shield_caster: GuidJson,
    pub shielded: GuidJson,
    pub shield_effect_id: u32,
    pub absorbed: i64,
    pub attacker: GuidJson,
    pub attacking_ability_id: u32,
}

impl From<&DamageAbsorbed> for DamageAbsorbedJson {
    fn from(value: &DamageAbsorbed) -> Self {
        DamageAbsorbedJson {
            shield_caster: (&value.shield_caster).into(),
            shielded: (&value.shielded).into(),
            shield_effect_id: value.shield_effect_id,
            absorbed: value.absorbed,
            attacker: (&value.attacker).into(),
            attacking_ability_id: value.attacking_ability_id,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterruptJson {
    pub interrupter: GuidJson,
    pub victim: GuidJson,
    pub interrupting_ability_id: u32,
    pub interrupted_ability_id: u32,
}

impl From<&Interrupt> for InterruptJson {
    fn from(value: &Interrupt) -> Self {
        InterruptJson {
            interrupter: (&value.interrupter).into(),
            victim: (&value.victim).into(),
            interrupting_ability_id: value.interrupting_ability_id,
            interrupted_ability_id: value.interrupted_ability_id,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DispelJson {
    pub dispeller: GuidJson,
    pub target: GuidJson,
    pub dispel_ability_id: u32,
    pub removed_effect_id: u32,
    pub remaining_seconds: f64,
    pub polarity: PolarityJson,
}

impl From<&Dispel> for DispelJson {
    fn from(value: &Dispel) -> Self {
        DispelJson {
            dispeller: (&value.dispeller).into(),
            target: (&value.target).into(),
            dispel_ability_id: value.dispel_ability_id,
            removed_effect_id: value.removed_effect_id,
            remaining_seconds: value.remaining_seconds,
            polarity: value.polarity.into(),
        }
    }
}

// --- Death / Resurrect ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeathJson {
    pub kind: DeathKindJson,
    pub dead: GuidJson,
    pub killer: GuidJson,
    pub killing_ability_id: u32,
}

impl From<&Death> for DeathJson {
    fn from(value: &Death) -> Self {
        DeathJson {
            kind: value.kind.into(),
            dead: (&value.dead).into(),
            killer: (&value.killer).into(),
            killing_ability_id: value.killing_ability_id,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResurrectJson {
    pub resurrecter: GuidJson,
    pub target: GuidJson,
    pub ability_id: u32,
}

impl From<&Resurrect> for ResurrectJson {
    fn from(value: &Resurrect) -> Self {
        ResurrectJson {
            resurrecter: (&value.resurrecter).into(),
            target: (&value.target).into(),
            ability_id: value.ability_id,
        }
    }
}

// --- LoggingStarted / ZoneChange / MapChange / DungeonStart / DungeonEnd ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingStartedJson {
    pub log_format_version: u32,
    pub game_build: String,
}

impl From<&LoggingStarted> for LoggingStartedJson {
    fn from(value: &LoggingStarted) -> Self {
        LoggingStartedJson {
            log_format_version: value.log_format_version,
            game_build: value.game_build.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneChangeJson {
    pub zone_name: String,
    pub zone_id: u32,
    pub difficulty: u32,
}

impl From<&ZoneChange> for ZoneChangeJson {
    fn from(value: &ZoneChange) -> Self {
        ZoneChangeJson {
            zone_name: value.zone_name.clone(),
            zone_id: value.zone_id,
            difficulty: value.difficulty,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapChangeJson {
    pub map_id: u32,
    pub floor_name: String,
}

impl From<&MapChange> for MapChangeJson {
    fn from(value: &MapChange) -> Self {
        MapChangeJson {
            map_id: value.map_id,
            floor_name: value.floor_name.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DungeonStartJson {
    pub name: String,
    pub zone_id: u32,
    pub key_level: u32,
    pub modifiers: Vec<u32>,
}

impl From<&DungeonStart> for DungeonStartJson {
    fn from(value: &DungeonStart) -> Self {
        DungeonStartJson {
            name: value.name.clone(),
            zone_id: value.zone_id,
            key_level: value.key_level,
            modifiers: value.modifiers.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DungeonEndJson {
    pub name: String,
    pub zone_id: u32,
    pub key_level: u32,
    pub success: bool,
    pub duration_ms: u64,
    pub score: f64,
}

impl From<&DungeonEnd> for DungeonEndJson {
    fn from(value: &DungeonEnd) -> Self {
        DungeonEndJson {
            name: value.name.clone(),
            zone_id: value.zone_id,
            key_level: value.key_level,
            success: value.success,
            duration_ms: value.duration_ms,
            score: value.score,
        }
    }
}

// --- Marker / WorldMarker ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkerJson {
    pub unit: GuidJson,
    pub index: u32,
    pub removed: bool,
}

impl From<&Marker> for MarkerJson {
    fn from(value: &Marker) -> Self {
        MarkerJson {
            unit: (&value.unit).into(),
            index: value.index,
            removed: value.removed,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldMarkerJson {
    pub x: f64,
    pub y: f64,
    pub slot: u32,
    pub removed: bool,
}

impl From<&WorldMarker> for WorldMarkerJson {
    fn from(value: &WorldMarker) -> Self {
        WorldMarkerJson {
            x: value.x,
            y: value.y,
            slot: value.slot,
            removed: value.removed,
        }
    }
}

// --- Event / EventBody ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogInstantJson {
    pub utc_ms: i64,
    pub seq: u32,
}

impl From<&LogInstant> for LogInstantJson {
    fn from(value: &LogInstant) -> Self {
        LogInstantJson {
            utc_ms: value.utc_ms,
            seq: value.seq,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum EventBodyJson {
    DamageHeal(DamageHealJson),
    DamageAbsorbed(DamageAbsorbedJson),
    Cast(CastJson),
    Effect(EffectJson),
    ResourceChange(ResourceChangeJson),
    Interrupt(InterruptJson),
    Dispel(DispelJson),
    Death(DeathJson),
    UnitDestroyed {
        unit: GuidJson,
    },
    Resurrect(ResurrectJson),
    Encounter(EncounterJson),
    LoggingStarted(LoggingStartedJson),
    ZoneChange(ZoneChangeJson),
    MapChange(MapChangeJson),
    DungeonStart(DungeonStartJson),
    DungeonEnd(DungeonEndJson),
    Marker(MarkerJson),
    WorldMarker(WorldMarkerJson),
    CombatantInfo(CombatantInfoJson),
    /// The vestigial `EVENT_INVALID` line — damage-shaped but safe to drop.
    Invalid,
    Unknown {
        raw_type: String,
    },
}

impl From<&EventBody> for EventBodyJson {
    fn from(value: &EventBody) -> Self {
        match value {
            EventBody::DamageHeal(inner) => EventBodyJson::DamageHeal(inner.into()),
            EventBody::DamageAbsorbed(inner) => EventBodyJson::DamageAbsorbed(inner.into()),
            EventBody::Cast(inner) => EventBodyJson::Cast(inner.into()),
            EventBody::Effect(inner) => EventBodyJson::Effect(inner.into()),
            EventBody::ResourceChange(inner) => EventBodyJson::ResourceChange(inner.into()),
            EventBody::Interrupt(inner) => EventBodyJson::Interrupt(inner.into()),
            EventBody::Dispel(inner) => EventBodyJson::Dispel(inner.into()),
            EventBody::Death(inner) => EventBodyJson::Death(inner.into()),
            EventBody::UnitDestroyed { unit } => EventBodyJson::UnitDestroyed { unit: unit.into() },
            EventBody::Resurrect(inner) => EventBodyJson::Resurrect(inner.into()),
            EventBody::Encounter(inner) => EventBodyJson::Encounter(inner.into()),
            EventBody::LoggingStarted(inner) => EventBodyJson::LoggingStarted(inner.into()),
            EventBody::ZoneChange(inner) => EventBodyJson::ZoneChange(inner.into()),
            EventBody::MapChange(inner) => EventBodyJson::MapChange(inner.into()),
            EventBody::DungeonStart(inner) => EventBodyJson::DungeonStart(inner.into()),
            EventBody::DungeonEnd(inner) => EventBodyJson::DungeonEnd(inner.into()),
            EventBody::Marker(inner) => EventBodyJson::Marker(inner.into()),
            EventBody::WorldMarker(inner) => EventBodyJson::WorldMarker(inner.into()),
            EventBody::CombatantInfo(inner) => EventBodyJson::CombatantInfo(inner.into()),
            EventBody::Invalid => EventBodyJson::Invalid,
            // `raw_fields` (DR-0003) stays Rust-only for now — not yet propagated across
            // the JSON boundary; see the crate's `extend_custom_event` test/README section
            // for the Rust-side extension path this bridge doesn't yet expose.
            EventBody::Unknown { raw_type, .. } => EventBodyJson::Unknown {
                raw_type: raw_type.clone(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EventJson {
    pub instant: LogInstantJson,
    pub body: EventBodyJson,
}

impl From<&Event> for EventJson {
    fn from(value: &Event) -> Self {
        EventJson {
            instant: (&value.instant).into(),
            body: (&value.body).into(),
        }
    }
}

// --- combatants::Combatant ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatantJson {
    pub ulid: String,
    pub name: String,
    pub is_recording_player: bool,
    pub info: CombatantInfoJson,
    pub snapshot_count: u32,
}

impl From<&Combatant> for CombatantJson {
    fn from(value: &Combatant) -> Self {
        CombatantJson {
            ulid: value.ulid.clone(),
            name: value.name.clone(),
            is_recording_player: value.is_recording_player,
            info: (&value.info).into(),
            snapshot_count: value.snapshot_count,
        }
    }
}
