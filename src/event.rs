//! The typed event model a parsed v8 line decodes into. Data types only — the
//! decoding lives in `parse`. Field maps follow the v8 combat-log format
//! reference. Only the fields downstream consumers need so far are decoded; the
//! source/target unit-state blocks and the deeply-nested COMBATANT_INFO payload
//! are decoded by later slices.

use crate::timestamp::LogInstant;

/// A combat-log unit id. `Player` ids are ephemeral u32s (reassigned per dungeon
/// instance); `Npc` ids carry a stable creature-template suffix; the
/// `Environment`/`Unrecognized` namespaces are game content, never anonymized.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Guid {
    Player(u32),
    Npc { spawn: u32, template: u32 },
    Environment,
    Unrecognized,
}

/// Attack/heal result tier (damage/heal f16). Crits are indicated *only* here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ResultTier {
    Hit,
    CriticalStrike,
    GrievousCriticalStrike,
    Block,
    Parry,
    Dodge,
    Miss,
    None,
}

/// Damage school (damage/heal f15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum School {
    Physical,
    Magical,
    None,
}

/// Which of the six families sharing the 30-field damage/heal anatomy a row is.
/// `is_damage` splits the damage side (aggregated for DPS) from the heal side.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DamageHealKind {
    AbilityDamage,
    SwingDamage,
    PeriodicDamage,
    Heal,
    PeriodicHeal,
    LifestealHeal,
}

impl DamageHealKind {
    pub fn is_damage(self) -> bool {
        matches!(
            self,
            DamageHealKind::AbilityDamage
                | DamageHealKind::SwingDamage
                | DamageHealKind::PeriodicDamage
        )
    }
}

/// A damage or heal event (shared 30-field anatomy). The source/target unit-state
/// blocks (f17–30) are decoded once a consumer reads them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DamageHeal {
    pub kind: DamageHealKind,
    pub source: Guid,
    pub target: Guid,
    pub ability_id: u32,
    /// f9: parent/root ability (0 = cast directly, 1 = generic passive/proc).
    pub parent_ability_id: u32,
    /// f10: amount actually applied to health (damage dealt / effective healing).
    pub applied: i64,
    /// f11: absorbed amount — can be negative on damage (amplification component).
    pub absorbed: i64,
    /// f12: overkill (damage) / overheal (heal); −1 = none.
    pub overkill: i64,
    /// f13: blocked amount (nonzero only with result Block).
    pub blocked: i64,
    /// f14: raw/base amount before mitigation; always 0 for heals.
    pub raw: i64,
    pub school: School,
    pub result: ResultTier,
    /// f17: source current HP, u32-wrap normalized — a post-death line logs a
    /// wrapped (negative) value, but its damage still counts. The rest of the
    /// source/target unit-state blocks are decoded once a consumer reads them.
    pub source_cur_hp: i32,
    /// f24: target current HP, u32-wrap normalized.
    pub target_cur_hp: i32,
}

/// Which phase of the cast/channel pipeline a line records. `*Start` carries the
/// haste-adjusted cast time; `*Fail` carries the quoted failure reason.
#[derive(Clone, PartialEq, Debug)]
pub enum CastPhase {
    Activated,
    CastStart { cast_seconds: f64 },
    CastSuccess,
    CastFail { reason: String },
    ChannelStart { cast_seconds: f64 },
    ChannelSuccess,
    ChannelFail { reason: String },
}

/// A cast/channel pipeline event (16-field base + phase-specific trailing field).
#[derive(Clone, PartialEq, Debug)]
pub struct Cast {
    pub phase: CastPhase,
    pub caster: Guid,
    pub ability_id: u32,
    pub has_target: bool,
    pub target: Guid,
    /// f16: the caster's resource snapshot at cast time — `(type, current, max)`
    /// tuples (empty when the caster's unit-state carries none).
    pub resources: Vec<(u32, f64, f64)>,
}

/// Which effect (aura) lifecycle transition a line records.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectPhase {
    Applied,
    Removed,
    Refreshed,
}

/// Effect polarity (f11). "Mounted" is logged `Debuff`, per the reference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Polarity {
    Buff,
    Debuff,
}

/// An effect (aura) applied/removed/refreshed event. The target unit-state block
/// (f12–f18) is decoded once a consumer reads it.
#[derive(Clone, PartialEq, Debug)]
pub struct Effect {
    pub phase: EffectPhase,
    pub caster: Guid,
    pub target: Guid,
    pub effect_id: u32,
    /// f9: duration seconds (−1 = infinite/permanent; 0 on removal).
    pub duration_seconds: f64,
    /// f10: stacks.
    pub stacks: i64,
    pub polarity: Polarity,
    /// f19: granting ability id — the tick-attribution anchor for DoTs/HoTs.
    pub granting_ability_id: u32,
    /// f22 (refreshed only): the unit that refreshed the aura, often an NPC.
    pub refresher: Option<Guid>,
}

/// A `RESOURCE_CHANGED` event (13 fields). Per-event deltas can't reconstruct a
/// gapless timeline (changes occur between logged events), per the reference.
#[derive(Clone, PartialEq, Debug)]
pub struct ResourceChange {
    pub source: Guid,
    pub owner: Guid,
    /// f7: resource type id — a per-class slot, not a global identity.
    pub resource_type: u32,
    /// f8: signed delta.
    pub delta: f64,
    /// f9: current-after.
    pub current: f64,
    /// f10: max (−1.0 = no max).
    pub max: f64,
    /// f12: causing ability id (0 = passive).
    pub causing_ability_id: u32,
}

/// A boss-encounter boundary. Encounters nest strictly inside dungeon spans;
/// wipes emit `ENCOUNTER_END|…|0`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EncounterPhase {
    Start,
    End { success: bool },
}

/// An `ENCOUNTER_START`/`ENCOUNTER_END` event.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Encounter {
    pub phase: EncounterPhase,
    pub encounter_id: u32,
    /// f4: the boss name array (game content, not player identity).
    pub bosses: Vec<String>,
}

/// One equipped piece, as a `COMBATANT_INFO` gear entry records it.
///
/// Every id here is an engine gameplay-tag index, and each field indexes a *different*
/// namespace — resolving one against another's catalog silently yields the wrong entity, so
/// consumers must keep them apart (the id spaces overlap numerically).
#[derive(Clone, PartialEq, Debug)]
pub struct GearPiece {
    /// The equipped item (`ItemID.*`). Legendaries are marked in the tag itself.
    pub item_id: u32,
    pub item_level: u32,
    /// `(attribute id, value)` — the piece's fixed slots first, then its rolls. Attribute ids
    /// are engine `AttributeSet` property indices, not gameplay tags.
    pub stats: Vec<(u32, i64)>,
    /// The set this piece carries (`ItemID.SetBonus.*`); a set activates across two carriers.
    pub set_bonus_id: Option<u32>,
    /// `(rank id, rank)` in the per-hero `DynamicItemAbilityRank` namespace — gear-granted
    /// hero abilities, **not** traits.
    pub ability_grants: Vec<(u32, u32)>,
    /// `(trait id, level)` in the `ItemTrait.ID` namespace.
    pub traits: Vec<(u32, u32)>,
    /// `(gem colour id, power)` — the piece's attunements (`ItemID.GemType.*`).
    pub gems: Vec<(u32, u32)>,
    pub score: f64,
}

/// One of the neck-trait candidates a combatant was offered, and whether it is chosen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NeckTraitChoice {
    pub trait_id: u32,
    pub selected: bool,
}

/// A `COMBATANT_INFO` line: the identity anchors, the build context a sim reproduces (hero,
/// average item level, the final computed stat sheet, talent picks), and the equipped gear a
/// build can be reconstructed from.
#[derive(Clone, PartialEq, Debug)]
pub struct CombatantInfo {
    /// f3: the persistent character ULID (1:1 with character name).
    pub ulid: String,
    /// f4: the run-scoped player id.
    pub player: Guid,
    /// f6: the recording-player flag (true exactly once per dungeon = the owner).
    pub is_recording_player: bool,
    /// f7: hero id.
    pub hero_id: u32,
    /// f8: average item level (the mean of the 14 gear ilvls).
    pub item_level: f64,
    /// f9: the final computed stat sheet — post-DR, post-set-bonus, sim-consumable.
    pub stat_sheet: Vec<f64>,
    /// f10: hero talent picks.
    pub talents: Vec<u32>,
    /// f12: the equipped pieces, one entry per gear slot in the game's own slot order.
    ///
    /// Slot identity is *positional*, so a piece that fails to decode is `None` in place rather
    /// than omitted — dropping it would silently shift every later slot by one and hand back a
    /// build wearing the wrong gear. `None` is the caller's signal to report an unreadable slot.
    pub gear: Vec<Option<GearPiece>>,
    /// f17: `(trait id, rank)` for every trait realized on the build, and the authoritative
    /// list to read trait levels from.
    ///
    /// It is not merely the sum of the per-piece grants: ranks here also reflect contributions
    /// the gear array doesn't carry, so a trait can appear at a higher rank than its pieces
    /// alone explain, or with no per-piece source at all.
    pub trait_ranks: Vec<(u32, u32)>,
    /// f19: the neck-trait candidates and which are chosen.
    pub neck_traits: Vec<NeckTraitChoice>,
}

/// A `DAMAGE_ABSORBED` event (14 fields): an absorb credited to a shield. Caster-
/// first ordering (f3/f4 shield caster, f5/f6 the shielded unit).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DamageAbsorbed {
    pub shield_caster: Guid,
    pub shielded: Guid,
    pub shield_effect_id: u32,
    /// f9: amount absorbed (matches the same-ms damage line's absorbed field).
    pub absorbed: i64,
    pub attacker: Guid,
    pub attacking_ability_id: u32,
}

/// An `ABILITY_INTERRUPT` event (10 fields).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Interrupt {
    pub interrupter: Guid,
    pub victim: Guid,
    pub interrupting_ability_id: u32,
    pub interrupted_ability_id: u32,
}

/// An `ABILITY_DISPEL` event (12 fields).
#[derive(Clone, PartialEq, Debug)]
pub struct Dispel {
    pub dispeller: Guid,
    pub target: Guid,
    pub dispel_ability_id: u32,
    pub removed_effect_id: u32,
    /// f11: the removed effect's remaining duration — can be negative.
    pub remaining_seconds: f64,
    pub polarity: Polarity,
}

/// Which death family: an enemy NPC (`UNIT_DEATH`) or a player (`ALLY_DEATH`).
/// Same 10-field anatomy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeathKind {
    Unit,
    Ally,
}

/// A `UNIT_DEATH` / `ALLY_DEATH` event.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Death {
    pub kind: DeathKind,
    pub dead: Guid,
    pub killer: Guid,
    pub killing_ability_id: u32,
}

/// A `RESURRECT` event (9 fields).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Resurrect {
    pub resurrecter: Guid,
    pub target: Guid,
    pub ability_id: u32,
}

/// A `LOGGING_STARTED` header (5 fields). May be entirely absent from a file, so
/// nothing may require it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LoggingStarted {
    pub log_format_version: u32,
    pub game_build: String,
}

/// A `ZONE_CHANGE` event (6 fields).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ZoneChange {
    pub zone_name: String,
    pub zone_id: u32,
    pub difficulty: u32,
}

/// A `MAP_CHANGE` event (8 fields); the four bounding-box floats are decoded once
/// a consumer needs them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MapChange {
    pub map_id: u32,
    pub floor_name: String,
}

/// A `DUNGEON_START` event (9 fields).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DungeonStart {
    pub name: String,
    pub zone_id: u32,
    pub key_level: u32,
    /// f6: modifier/affix ids (empty on a treeless run).
    pub modifiers: Vec<u32>,
}

/// A `DUNGEON_END` event (12 fields). An abandoned/failed run logs `success=false`
/// with an emptied modifier bracket and zero score.
#[derive(Clone, PartialEq, Debug)]
pub struct DungeonEnd {
    pub name: String,
    pub zone_id: u32,
    pub key_level: u32,
    pub success: bool,
    pub duration_ms: u64,
    pub score: f64,
}

/// A `MARKER_PLACED` / `MARKER_REMOVED` event (5 fields): a raid marker on a unit.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Marker {
    pub unit: Guid,
    pub index: u32,
    pub removed: bool,
}

/// A `WORLD_MARKER_PLACED` / `WORLD_MARKER_REMOVED` event (5 fields): a marker at
/// a world position.
#[derive(Clone, PartialEq, Debug)]
pub struct WorldMarker {
    pub x: f64,
    pub y: f64,
    pub slot: u32,
    pub removed: bool,
}

/// One decoded v8 log line: its instant plus a typed body. Unrecognized event
/// types are surfaced as `Unknown`, never dropped, so the long tail is visible.
#[derive(Clone, PartialEq, Debug)]
pub struct Event {
    pub instant: LogInstant,
    pub body: EventBody,
}

/// The typed body of a decoded line. Covers the full v8 event catalog; a token
/// outside it becomes `Unknown` (surfaced, never dropped).
#[derive(Clone, PartialEq, Debug)]
pub enum EventBody {
    DamageHeal(DamageHeal),
    DamageAbsorbed(DamageAbsorbed),
    Cast(Cast),
    Effect(Effect),
    ResourceChange(ResourceChange),
    Interrupt(Interrupt),
    Dispel(Dispel),
    Death(Death),
    UnitDestroyed {
        unit: Guid,
    },
    Resurrect(Resurrect),
    Encounter(Encounter),
    LoggingStarted(LoggingStarted),
    ZoneChange(ZoneChange),
    MapChange(MapChange),
    DungeonStart(DungeonStart),
    DungeonEnd(DungeonEnd),
    Marker(Marker),
    WorldMarker(WorldMarker),
    CombatantInfo(CombatantInfo),
    /// The vestigial `EVENT_INVALID` line — damage-shaped but safe to drop.
    Invalid,
    Unknown {
        raw_type: String,
    },
}
