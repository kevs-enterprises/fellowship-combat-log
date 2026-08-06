//! v8 combat-log line encoding: typed `Event` in, log text out — the inverse of
//! `parse`. Canonicalizing, not byte-exact (DR-0002): `parse(encode_line(e)?)? ==
//! e` holds, but `encode_line(parse_line(text)?)?` is not guaranteed to equal
//! `text`, since parsing already discards data (unit names, most unit-state
//! sub-fields, the UTC offset, the entire `Invalid` payload) that encoding
//! therefore has nothing to reconstruct. `EventBody::Unknown` keeps its raw
//! fields (DR-0003), but `encode_body` still refuses to re-assemble them
//! generically — the type token past `Unknown` is opaque to this crate, so
//! only a consumer that knows what it means can encode it back to text (see
//! the `parse`/`encode` primitives it can reuse to do so).
//!
//! **Placeholder-value convention** for a field position the type doesn't
//! retain: `0` for an int, `0.0` for a float, `[]` for a list, `"-"` for a
//! discarded quoted display/ability name (matching the corpus convention seen
//! in e.g. `resource_changed.log`/`effect_aura.log`), and the Unix epoch
//! (`1970-01-01T00:00:00.000+00:00`) for a discarded nested timestamp
//! (`DUNGEON_START`'s f8). Every family task (#18-#21) applies this
//! convention to its own unretained fields, with a few documented exceptions
//! where the generic placeholder would misrepresent the corpus: cast/channel's
//! discarded target name (f9) uses a quoted `"0"`; `COMBATANT_INFO`'s gear
//! array uses `()` for a `None` slot, not `0`/`[]`/`"-"` (see `gear_list`);
//! and its `neck_traits`' unused middle element uses `1`, not `0` (see
//! `neck_trait_list`).
//!
//! Sits parallel to `parse`, not on top of it: both depend on `event` and
//! `timestamp`, and neither depends on the other.

use crate::event::{
    Cast, CastPhase, CombatantInfo, DamageAbsorbed, DamageHeal, DamageHealKind, Death, DeathKind,
    Dispel, DungeonEnd, DungeonStart, Effect, EffectPhase, Encounter, EncounterPhase, Event,
    EventBody, GearPiece, Guid, Interrupt, LoggingStarted, MapChange, Marker, NeckTraitChoice,
    ResourceChange, Resurrect, WorldMarker, ZoneChange,
};
use crate::timestamp::render_instant;

/// Why an `Event` could not be encoded. Total: `encode_line` hands callers an
/// `Err`, never a panic, mirroring `ParseError` on the decode side.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EncodeError {
    /// This crate doesn't know how to encode the event: `EventBody::Invalid`'s
    /// payload was discarded during parsing, and `EventBody::Unknown`'s raw
    /// type is opaque here even though its fields were kept — a consumer that
    /// knows what the type means can encode it itself from `raw_fields` using
    /// the `pub` primitives in this module. Carries the wire-format type token
    /// for diagnostics.
    Unrepresentable { event_type: String },
    /// A string field contains a character that would corrupt the wire format:
    /// a literal `|` (breaks the field framing everywhere — no field is safe
    /// from it), or, inside a bracketed-list element, an embedded `"` (would
    /// desynchronize `split_top_level`'s quote-toggle tracking on re-parse).
    InvalidText {
        field: &'static str,
        reason: &'static str,
    },
    /// A float field is NaN or infinite. `NaN != NaN`, so a non-finite value can
    /// never satisfy the round-trip contract `parse(encode(e)) == e`.
    NonFiniteFloat { field: &'static str },
    /// The event's timestamp falls outside the representable year range
    /// (`0000..=9999`) the wire format's fixed-width layout requires.
    TimestampOutOfRange,
    /// A struct's own fields contradict each other in a way `parse_line` could
    /// never produce from valid text — e.g. `Effect.refresher` is `Some` while
    /// `phase != Refreshed`, or `None` while `phase == Refreshed`. Encoding
    /// would otherwise have to silently drop data or desync the two fields on
    /// re-parse.
    InconsistentState { reason: &'static str },
}

/// Encode one `Event` back into v8 log-line text. `event.instant.seq` is not
/// part of the wire format (it never was — `parse_line` takes it as a
/// caller-supplied parameter, not something the text carries), so it plays no
/// role here either.
pub fn encode_line(event: &Event) -> Result<String, EncodeError> {
    let timestamp =
        render_instant(event.instant.utc_ms).map_err(|_| EncodeError::TimestampOutOfRange)?;
    let body = encode_body(&event.body)?;
    Ok(format!("{timestamp}|{body}"))
}

fn encode_body(body: &EventBody) -> Result<String, EncodeError> {
    match body {
        EventBody::DamageHeal(damage_heal) => encode_damage_heal(damage_heal),
        EventBody::DamageAbsorbed(damage_absorbed) => encode_damage_absorbed(damage_absorbed),
        EventBody::Cast(cast) => encode_cast(cast),
        EventBody::Effect(effect) => encode_effect(effect),
        EventBody::ResourceChange(resource_change) => encode_resource_change(resource_change),
        EventBody::Interrupt(interrupt) => encode_interrupt(interrupt),
        EventBody::Dispel(dispel) => encode_dispel(dispel),
        EventBody::Death(death) => encode_death(death),
        EventBody::UnitDestroyed { unit } => encode_unit_destroyed(unit),
        EventBody::Resurrect(resurrect) => encode_resurrect(resurrect),
        EventBody::Encounter(encounter) => encode_encounter(encounter),
        EventBody::LoggingStarted(logging_started) => encode_logging_started(logging_started),
        EventBody::ZoneChange(zone_change) => encode_zone_change(zone_change),
        EventBody::MapChange(map_change) => encode_map_change(map_change),
        EventBody::DungeonStart(dungeon_start) => encode_dungeon_start(dungeon_start),
        EventBody::DungeonEnd(dungeon_end) => encode_dungeon_end(dungeon_end),
        EventBody::Marker(marker) => encode_marker(marker),
        EventBody::WorldMarker(world_marker) => encode_world_marker(world_marker),
        EventBody::CombatantInfo(combatant_info) => encode_combatant_info(combatant_info),
        EventBody::Invalid => Err(EncodeError::Unrepresentable {
            event_type: "EVENT_INVALID".to_string(),
        }),
        EventBody::Unknown { raw_type, .. } => Err(EncodeError::Unrepresentable {
            event_type: raw_type.clone(),
        }),
    }
}

// --- WORLD_MARKER_PLACED / WORLD_MARKER_REMOVED (the tracer family) ---

/// `WorldMarker` (5 fields, all retained): the simplest possible proof the
/// dispatch, primitives, and timestamp rendering all fit together correctly.
fn encode_world_marker(world_marker: &WorldMarker) -> Result<String, EncodeError> {
    let token = if world_marker.removed {
        "WORLD_MARKER_REMOVED"
    } else {
        "WORLD_MARKER_PLACED"
    };
    Ok(format!(
        "{token}|{}|{}|{}",
        render_float(world_marker.x, "x")?,
        render_float(world_marker.y, "y")?,
        world_marker.slot
    ))
}

// --- simple events: UNIT_DESTROYED, MARKER_*, UNIT_DEATH/ALLY_DEATH,
// RESURRECT, ABILITY_INTERRUPT, ENCOUNTER_START/END ---

/// The discarded-quoted-name placeholder, per the module doc's convention —
/// matches the corpus's own `"-"` convention for a position the type never
/// retained. Shared by every family below that discards a unit/ability name.
const DISCARDED_NAME: &str = "\"-\"";

/// `UNIT_DESTROYED` (5 wire fields; only `unit` at f3 is retained — f4 the
/// discarded unit name, f5 an unmodeled float).
fn encode_unit_destroyed(unit: &Guid) -> Result<String, EncodeError> {
    Ok(format!(
        "UNIT_DESTROYED|{}|{DISCARDED_NAME}|0.0",
        unit.render()
    ))
}

/// `MARKER_PLACED`/`MARKER_REMOVED` (5 fields; f4 the discarded unit name).
fn encode_marker(marker: &Marker) -> Result<String, EncodeError> {
    let token = if marker.removed {
        "MARKER_REMOVED"
    } else {
        "MARKER_PLACED"
    };
    Ok(format!(
        "{token}|{}|{DISCARDED_NAME}|{}",
        marker.unit.render(),
        marker.index
    ))
}

/// `UNIT_DEATH`/`ALLY_DEATH` (10 fields; f4/f6 discarded names, f8 discarded
/// ability name, f9/f10 unmodeled int/float).
fn encode_death(death: &Death) -> Result<String, EncodeError> {
    let token = match death.kind {
        DeathKind::Unit => "UNIT_DEATH",
        DeathKind::Ally => "ALLY_DEATH",
    };
    Ok(format!(
        "{token}|{}|{DISCARDED_NAME}|{}|{DISCARDED_NAME}|{}|{DISCARDED_NAME}|0|0.0",
        death.dead.render(),
        death.killer.render(),
        death.killing_ability_id
    ))
}

/// `RESURRECT` (9 fields, trailing empty f9; f4/f6 discarded names, f8
/// discarded ability name).
fn encode_resurrect(resurrect: &Resurrect) -> Result<String, EncodeError> {
    Ok(format!(
        "RESURRECT|{}|{DISCARDED_NAME}|{}|{DISCARDED_NAME}|{}|{DISCARDED_NAME}|",
        resurrect.resurrecter.render(),
        resurrect.target.render(),
        resurrect.ability_id
    ))
}

/// `ABILITY_INTERRUPT` (10 fields; f4/f6/f8/f10 discarded names).
fn encode_interrupt(interrupt: &Interrupt) -> Result<String, EncodeError> {
    Ok(format!(
        "ABILITY_INTERRUPT|{}|{DISCARDED_NAME}|{}|{DISCARDED_NAME}|{}|{DISCARDED_NAME}|{}|{DISCARDED_NAME}",
        interrupt.interrupter.render(),
        interrupt.victim.render(),
        interrupt.interrupting_ability_id,
        interrupt.interrupted_ability_id
    ))
}

/// `ENCOUNTER_START` (4 fields) / `ENCOUNTER_END` (5 fields, trailing `1`/`0`
/// success). `bosses` is a strict array on decode — `parse_name_array` errors
/// without outer brackets — so an empty list must render exactly `[]`.
fn encode_encounter(encounter: &Encounter) -> Result<String, EncodeError> {
    let bosses = render_name_array(&encounter.bosses, "bosses")?;
    match encounter.phase {
        EncounterPhase::Start => Ok(format!(
            "ENCOUNTER_START|{}|{bosses}",
            encounter.encounter_id
        )),
        EncounterPhase::End { success } => Ok(format!(
            "ENCOUNTER_END|{}|{bosses}|{}",
            encounter.encounter_id,
            if success { 1 } else { 0 }
        )),
    }
}

// --- damage/heal (6 tokens) + DAMAGE_ABSORBED ---

/// Wrap a current-HP value back to its wire-format u32 bit pattern — the exact
/// inverse of `normalize_hp`'s decode side, which reinterprets a u32-wrapped
/// value as a negative `i32` (a post-death line logs curHP as e.g.
/// `4294962654` = −4642). Not named `render_*`: unlike the shared primitives
/// below, this is a one-line cast specific to `DamageHeal`'s two HP fields,
/// not a primitive other families import.
fn wrapped_hp(hp: i32) -> u32 {
    hp as u32
}

/// The 30-field damage/heal anatomy shared by all six tokens. `kind` selects
/// the token; f4/f6 discarded names, f8 discarded ability name. f18-23 and
/// f25-30 (the source/target unit-state blocks past the retained current-HP)
/// are entirely unmodeled: `UNIT_STATE_FILLER` for the middle five, `[]` for
/// the trailing resource list (`DamageHeal` has no resources field at all).
fn encode_damage_heal(damage_heal: &DamageHeal) -> Result<String, EncodeError> {
    let token = match damage_heal.kind {
        DamageHealKind::AbilityDamage => "ABILITY_DAMAGE",
        DamageHealKind::SwingDamage => "SWING_DAMAGE",
        DamageHealKind::PeriodicDamage => "ABILITY_PERIODIC_DAMAGE",
        DamageHealKind::Heal => "ABILITY_HEAL",
        DamageHealKind::PeriodicHeal => "ABILITY_PERIODIC_HEAL",
        DamageHealKind::LifestealHeal => "ABILITY_LIFESTEAL_HEAL",
    };
    let source = damage_heal.source.render();
    let target = damage_heal.target.render();
    let ability_id = damage_heal.ability_id;
    let parent_ability_id = damage_heal.parent_ability_id;
    let applied = damage_heal.applied;
    let absorbed = damage_heal.absorbed;
    let overkill = damage_heal.overkill;
    let blocked = damage_heal.blocked;
    let raw = damage_heal.raw;
    let school = damage_heal.school.render();
    let result = damage_heal.result.render();
    let source_hp = wrapped_hp(damage_heal.source_cur_hp);
    let target_hp = wrapped_hp(damage_heal.target_cur_hp);
    Ok(format!(
        "{token}|{source}|{DISCARDED_NAME}|{target}|{DISCARDED_NAME}|{ability_id}|{DISCARDED_NAME}|\
         {parent_ability_id}|{applied}|{absorbed}|{overkill}|{blocked}|{raw}|{school}|{result}|\
         {source_hp}|{UNIT_STATE_FILLER}|[]|{target_hp}|{UNIT_STATE_FILLER}|[]"
    ))
}

/// `DAMAGE_ABSORBED` (14 fields): caster-first ordering (f3/f4 shield caster,
/// f5/f6 shielded unit), f4/f6/f8/f11/f13 discarded names, f14 unmodeled int.
fn encode_damage_absorbed(damage_absorbed: &DamageAbsorbed) -> Result<String, EncodeError> {
    let shield_caster = damage_absorbed.shield_caster.render();
    let shielded = damage_absorbed.shielded.render();
    let shield_effect_id = damage_absorbed.shield_effect_id;
    let absorbed = damage_absorbed.absorbed;
    let attacker = damage_absorbed.attacker.render();
    let attacking_ability_id = damage_absorbed.attacking_ability_id;
    Ok(format!(
        "DAMAGE_ABSORBED|{shield_caster}|{DISCARDED_NAME}|{shielded}|{DISCARDED_NAME}|\
         {shield_effect_id}|{DISCARDED_NAME}|{absorbed}|{attacker}|{DISCARDED_NAME}|\
         {attacking_ability_id}|{DISCARDED_NAME}|0"
    ))
}

// --- session/dungeon: LOGGING_STARTED, ZONE_CHANGE, MAP_CHANGE,
// DUNGEON_START, DUNGEON_END ---

/// Placeholder for `DUNGEON_START`'s f8, a second nested timestamp the type
/// model doesn't retain. Renders the Unix epoch — a fixed, always-valid
/// placeholder, since `DungeonStart` carries no instant of its own to render.
const UNRETAINED_TIMESTAMP: &str = "1970-01-01T00:00:00.000+00:00";

/// `LOGGING_STARTED` (5 fields; f5 unmodeled). `game_build` (f4) is
/// deliberately unquoted, unlike every other string field — the corpus writes
/// it as raw text containing a space (`0.4.2.0 cl:112206`).
fn encode_logging_started(logging_started: &LoggingStarted) -> Result<String, EncodeError> {
    let game_build = render_unquoted(&logging_started.game_build, "game_build")?;
    Ok(format!(
        "LOGGING_STARTED|{}|{game_build}|0",
        logging_started.log_format_version
    ))
}

/// `ZONE_CHANGE` (6 fields, trailing empty f6).
fn encode_zone_change(zone_change: &ZoneChange) -> Result<String, EncodeError> {
    let zone_name = render_quoted(&zone_change.zone_name, "zone_name")?;
    Ok(format!(
        "ZONE_CHANGE|{zone_name}|{}|{}|",
        zone_change.zone_id, zone_change.difficulty
    ))
}

/// `MAP_CHANGE` (8 fields; f5-f8 unmodeled bounding-box floats).
fn encode_map_change(map_change: &MapChange) -> Result<String, EncodeError> {
    let floor_name = render_quoted(&map_change.floor_name, "floor_name")?;
    Ok(format!(
        "MAP_CHANGE|{}|{floor_name}|0.0|0.0|0.0|0.0",
        map_change.map_id
    ))
}

/// `DUNGEON_START` (9 fields, trailing empty f9). `modifiers` (f6) is a
/// **strict** array on decode — `parse_int_array` errors without outer
/// brackets. f7 unmodeled int; f8 the unretained nested timestamp.
fn encode_dungeon_start(dungeon_start: &DungeonStart) -> Result<String, EncodeError> {
    let name = render_quoted(&dungeon_start.name, "name")?;
    let modifiers = render_int_array(&dungeon_start.modifiers);
    Ok(format!(
        "DUNGEON_START|{name}|{}|{}|{modifiers}|0|{UNRETAINED_TIMESTAMP}|",
        dungeon_start.zone_id, dungeon_start.key_level
    ))
}

/// `DUNGEON_END` (12 fields). f6 is a modifier array the struct does **not**
/// retain (unlike `DUNGEON_START`, which does) — always `[]`. f10-f12 three
/// more unmodeled ints.
fn encode_dungeon_end(dungeon_end: &DungeonEnd) -> Result<String, EncodeError> {
    let name = render_quoted(&dungeon_end.name, "name")?;
    let score = render_float(dungeon_end.score, "score")?;
    let success = if dungeon_end.success { 1 } else { 0 };
    Ok(format!(
        "DUNGEON_END|{name}|{}|{}|[]|{success}|{}|{score}|0|0|0",
        dungeon_end.zone_id, dungeon_end.key_level, dungeon_end.duration_ms
    ))
}

// --- cast/channel (7 tokens), EFFECT_* (3 tokens), ABILITY_DISPEL,
// RESOURCE_CHANGED ---

/// The target-name placeholder cast/channel lines use at f9 — the corpus's own
/// convention when untargeted (`has_target=0`/`target=UnrecognizedType-0`) is a
/// quoted literal `"0"`, distinct from `DISCARDED_NAME`'s `"-"`. Since the
/// struct never retains a target name either way, every cast/channel line uses
/// this placeholder regardless of `has_target`.
const CAST_DISCARDED_TARGET_NAME: &str = "\"0\"";

/// The cast/channel pipeline (16-field base + phase-specific trailing field).
/// `phase` selects both the token and the trailing field (none for
/// `Activated`/`*Success`, a shortest-form float for `*Start`, a quoted reason
/// for `*Fail`). f4/f6 discarded names, f9 the target-name placeholder above,
/// f10-15 the caster's unmodeled unit-state (a `0` current-HP placeholder plus
/// `UNIT_STATE_FILLER`), f16 the real `resources` list.
fn encode_cast(cast: &Cast) -> Result<String, EncodeError> {
    let (token, trailing) = match &cast.phase {
        CastPhase::Activated => ("ABILITY_ACTIVATED", None),
        CastPhase::CastSuccess => ("ABILITY_CAST_SUCCESS", None),
        CastPhase::ChannelSuccess => ("ABILITY_CHANNEL_SUCCESS", None),
        CastPhase::CastStart { cast_seconds } => (
            "ABILITY_CAST_START",
            Some(render_float(*cast_seconds, "cast_seconds")?),
        ),
        CastPhase::ChannelStart { cast_seconds } => (
            "ABILITY_CHANNEL_START",
            Some(render_float(*cast_seconds, "cast_seconds")?),
        ),
        CastPhase::CastFail { reason } => {
            ("ABILITY_CAST_FAIL", Some(render_quoted(reason, "reason")?))
        }
        CastPhase::ChannelFail { reason } => (
            "ABILITY_CHANNEL_FAIL",
            Some(render_quoted(reason, "reason")?),
        ),
    };
    let caster = cast.caster.render();
    let ability_id = cast.ability_id;
    let has_target = if cast.has_target { 1 } else { 0 };
    let target = cast.target.render();
    let resources = render_resource_tuples(&cast.resources, "resources")?;
    let base = format!(
        "{token}|{caster}|{DISCARDED_NAME}|{ability_id}|{DISCARDED_NAME}|{has_target}|{target}|\
         {CAST_DISCARDED_TARGET_NAME}|0|{UNIT_STATE_FILLER}|{resources}"
    );
    Ok(match trailing {
        Some(trailing) => format!("{base}|{trailing}"),
        None => base,
    })
}

/// An effect (aura) applied/removed/refreshed event (21 fields; 23 for
/// `Refreshed`, which appends f22 the refresher guid and f23 its discarded
/// name). f4/f6/f8 discarded names, f12-18 the target's unmodeled unit-state
/// (this family retains neither end: a `0` current-HP placeholder and a
/// trailing `[]`), f20 the discarded granting-ability name, f21 an unmodeled
/// int. `refresher` must be `Some` exactly when `phase == Refreshed` — anything
/// else is a struct-level inconsistency `parse_line` could never produce.
fn encode_effect(effect: &Effect) -> Result<String, EncodeError> {
    let (token, refresher) = match (effect.phase, &effect.refresher) {
        (EffectPhase::Applied | EffectPhase::Removed, Some(_)) => {
            return Err(EncodeError::InconsistentState {
                reason: "refresher is Some but phase is not Refreshed",
            });
        }
        (EffectPhase::Refreshed, None) => {
            return Err(EncodeError::InconsistentState {
                reason: "phase is Refreshed but refresher is None",
            });
        }
        (EffectPhase::Applied, None) => ("EFFECT_APPLIED", None),
        (EffectPhase::Removed, None) => ("EFFECT_REMOVED", None),
        (EffectPhase::Refreshed, Some(refresher)) => ("EFFECT_REFRESHED", Some(refresher.render())),
    };
    let caster = effect.caster.render();
    let target = effect.target.render();
    let effect_id = effect.effect_id;
    let duration = render_float(effect.duration_seconds, "duration_seconds")?;
    let stacks = effect.stacks;
    let polarity = effect.polarity.render();
    let granting_ability_id = effect.granting_ability_id;
    let base = format!(
        "{token}|{caster}|{DISCARDED_NAME}|{target}|{DISCARDED_NAME}|{effect_id}|{DISCARDED_NAME}|\
         {duration}|{stacks}|{polarity}|0|{UNIT_STATE_FILLER}|[]|{granting_ability_id}|\
         {DISCARDED_NAME}|0"
    );
    Ok(match refresher {
        Some(refresher) => format!("{base}|{refresher}|{DISCARDED_NAME}"),
        None => base,
    })
}

/// `ABILITY_DISPEL` (12 fields; f4/f6/f8/f10 discarded names). `polarity` is
/// field 12 — #15 fixed the mis-reported decode-error field number for this
/// call site, so this encoder's field count keeps that fix honest.
fn encode_dispel(dispel: &Dispel) -> Result<String, EncodeError> {
    let dispeller = dispel.dispeller.render();
    let target = dispel.target.render();
    let dispel_ability_id = dispel.dispel_ability_id;
    let removed_effect_id = dispel.removed_effect_id;
    let remaining = render_float(dispel.remaining_seconds, "remaining_seconds")?;
    let polarity = dispel.polarity.render();
    Ok(format!(
        "ABILITY_DISPEL|{dispeller}|{DISCARDED_NAME}|{target}|{DISCARDED_NAME}|\
         {dispel_ability_id}|{DISCARDED_NAME}|{removed_effect_id}|{DISCARDED_NAME}|\
         {remaining}|{polarity}"
    ))
}

/// `RESOURCE_CHANGED` (13 fields; f4/f6 discarded names, f13 discarded name).
/// f11 is a separate unmodeled float the struct does not retain — distinct
/// from f10's `max`, which it does; the two must not be conflated.
fn encode_resource_change(resource_change: &ResourceChange) -> Result<String, EncodeError> {
    let source = resource_change.source.render();
    let owner = resource_change.owner.render();
    let resource_type = resource_change.resource_type;
    let delta = render_float(resource_change.delta, "delta")?;
    let current = render_float(resource_change.current, "current")?;
    let max = render_float(resource_change.max, "max")?;
    let causing_ability_id = resource_change.causing_ability_id;
    Ok(format!(
        "RESOURCE_CHANGED|{source}|{DISCARDED_NAME}|{owner}|{DISCARDED_NAME}|{resource_type}|\
         {delta}|{current}|{max}|0.0|{causing_ability_id}|{DISCARDED_NAME}"
    ))
}

// --- COMBATANT_INFO (the last, and most complex, family) ---

/// `COMBATANT_INFO` (20 fields). **The GUID is at f4, not f3** — f3 is the raw
/// ULID (unquoted, the same convention as `LOGGING_STARTED`'s `game_build`).
/// f5 is the character's quoted display name: discarded by the parser but
/// read out-of-band by `combatants::extract_name`, which trims the quotes and
/// keeps whatever's left — so an encoded line doesn't yield an *unnamed*
/// combatant through `list_combatants`, it yields one literally named `-`
/// (`DISCARDED_NAME` stripped of its quotes), including overwriting a real
/// name on a later snapshot (`combatants.rs`'s "never let a well-formed name
/// be replaced by an empty one" guard doesn't catch a non-empty `-`). This is
/// still consistent with spec #13's own scope boundary ("not an encoder for
/// `Combatant`/aggregated data from `combatants.rs`") — not a bug to fix
/// here, just not literally "unnamed".
/// f9 (`stat_sheet`) and f10 (`talents`) are strict arrays on decode; f11
/// (`gem_power`), f12 (`gear`), f17 (`trait_ranks`), and f19 (`neck_traits`)
/// are read leniently but always emitted here. f13/f14/f15/f16/f18/f20 are
/// entirely unmodeled.
fn encode_combatant_info(combatant_info: &CombatantInfo) -> Result<String, EncodeError> {
    let ulid = render_unquoted(&combatant_info.ulid, "ulid")?;
    let player = combatant_info.player.render();
    let is_recording_player = if combatant_info.is_recording_player {
        1
    } else {
        0
    };
    let hero_id = combatant_info.hero_id;
    let item_level = render_float(combatant_info.item_level, "item_level")?;
    let stat_sheet = render_float_array(&combatant_info.stat_sheet, "stat_sheet")?;
    let talents = render_int_array(&combatant_info.talents);
    let gem_power = render_float_array(&combatant_info.gem_power, "gem_power")?;
    let gear = gear_list(&combatant_info.gear)?;
    let trait_ranks = render_pair_list(&combatant_info.trait_ranks);
    let neck_traits = neck_trait_list(&combatant_info.neck_traits);
    Ok(format!(
        "COMBATANT_INFO|{ulid}|{player}|{DISCARDED_NAME}|{is_recording_player}|{hero_id}|\
         {item_level}|{stat_sheet}|{talents}|{gem_power}|{gear}|0|[]|[]|[]|{trait_ranks}|0|\
         {neck_traits}|0.0"
    ))
}

/// The equipped-gear array (f12): one element per slot. `None` renders as the
/// empty tuple `()` so its position — and every later slot's — survives
/// re-parsing: `parse_gear_piece` reads a shorter-than-13-part tuple as an
/// unreadable slot, not an absent one, so the slot count itself is what keeps
/// positions aligned. Not `render_*`-prefixed: like `wrapped_hp`, this is
/// specific to `CombatantInfo.gear`, not a primitive another family imports.
fn gear_list(gear: &[Option<GearPiece>]) -> Result<String, EncodeError> {
    let mut elements = Vec::with_capacity(gear.len());
    for piece in gear {
        elements.push(match piece {
            None => "()".to_string(),
            Some(piece) => gear_piece(piece)?,
        });
    }
    Ok(bracketed(elements))
}

/// One equipped piece as its 13-element wire tuple. Positions 5-6 (0-indexed)
/// are two scalars `GearPiece` never retained; both emit `0`.
fn gear_piece(piece: &GearPiece) -> Result<String, EncodeError> {
    let item_id = piece.item_id;
    let item_level = piece.item_level;
    let rarity = piece.rarity;
    let (temper0, temper1) = piece.temper;
    let stats = render_pair_list(&piece.stats);
    let set_bonus_id = match piece.set_bonus_id {
        None => "[]".to_string(),
        Some(id) => format!("[{id}]"),
    };
    let ability_grants = render_pair_list(&piece.ability_grants);
    let traits = render_pair_list(&piece.traits);
    let gems = render_pair_list(&piece.gems);
    let score = render_float(piece.score, "score")?;
    Ok(format!(
        "({item_id},{item_level},{rarity},{temper0},{temper1},0,0,{stats},\
         {set_bonus_id},{ability_grants},{traits},{gems},{score})"
    ))
}

/// `neck_traits` (f19): `(trait_id, 1, selected)` triples. The middle element
/// ("offered") is read but never stored by the parser (`parse_neck_traits`
/// discards it into `_offered`), so there's no field to source it from —
/// every element emits the corpus's own literal `1`. Not `render_*`-prefixed
/// for the same reason as `gear_list` above.
fn neck_trait_list(neck_traits: &[NeckTraitChoice]) -> String {
    bracketed(
        neck_traits
            .iter()
            .map(|choice| {
                let trait_id = choice.trait_id;
                let selected = if choice.selected { 1 } else { 0 };
                format!("({trait_id},1,{selected})")
            })
            .collect(),
    )
}

// --- shared primitives ---
//
// The family encoders above build their event bodies out of these. They're
// `pub`: a consumer encoding their own event type back to text — built from
// `EventBody::Unknown`'s `raw_fields`, or from a type that never went through
// `Unknown` at all — reuses the exact wire-format rendering rules here rather
// than re-deriving quoting, list bracketing, and float formatting from scratch.

/// A `|` anywhere corrupts the pipe-split that frames every line — the wire
/// format has no escaping (`parse.rs`'s module doc: "no `|` inside quoted
/// strings or bracketed lists"). Checked for every string field, quoted or not.
pub fn validate_text(s: &str, field: &'static str) -> Result<(), EncodeError> {
    if s.contains('|') {
        return Err(EncodeError::InvalidText {
            field,
            reason: "contains '|', which would corrupt the line's field framing",
        });
    }
    Ok(())
}

/// Join already-rendered elements into a bracketed list (`[e1,e2,...]`); `[]`
/// for empty, byte-for-byte — the only path `split_bracket_list`'s empty-inner
/// check accepts as zero elements. Shared by every list-shaped primitive below.
pub fn bracketed(elements: Vec<String>) -> String {
    format!("[{}]", elements.join(","))
}

/// Wrap `s` in double quotes for a quoted wire field. `field` names the struct
/// field being encoded, for diagnostics.
pub fn render_quoted(s: &str, field: &'static str) -> Result<String, EncodeError> {
    validate_text(s, field)?;
    Ok(format!("\"{s}\""))
}

/// Render `s` as a raw wire field with no surrounding quotes — `CombatantInfo`'s
/// `ulid` and `LoggingStarted`'s `game_build` are the two fields the wire format
/// carries this way (`game_build` contains a space but is still unquoted).
pub fn render_unquoted(s: &str, field: &'static str) -> Result<String, EncodeError> {
    validate_text(s, field)?;
    Ok(s.to_string())
}

/// A quoted-name array (`Encounter.bosses`): each element quoted, comma-joined,
/// wrapped in `[...]`. An embedded `"` inside an element is rejected — unlike a
/// top-level quoted field, an array element goes through `split_top_level`'s
/// quote-toggle tracking on re-parse, which a stray `"` would desynchronize.
pub fn render_name_array(names: &[String], field: &'static str) -> Result<String, EncodeError> {
    let mut elements = Vec::with_capacity(names.len());
    for name in names {
        validate_text(name, field)?;
        if name.contains('"') {
            return Err(EncodeError::InvalidText {
                field,
                reason: "contains '\"', which would desynchronize quote-tracking inside a bracketed list",
            });
        }
        elements.push(format!("\"{name}\""));
    }
    Ok(bracketed(elements))
}

/// A bracketed int array (`[4,6,8,19]`).
pub fn render_int_array(values: &[u32]) -> String {
    bracketed(values.iter().map(u32::to_string).collect())
}

/// A finite `f64`, formatted with Rust's default `Display` — the shortest
/// decimal string that re-parses to the identical value. Not fixed precision:
/// DR-0002's contract is `parse(encode(e)) == e`, which a fixed number of
/// decimal places cannot guarantee for an arbitrary float.
pub fn render_float(value: f64, field: &'static str) -> Result<String, EncodeError> {
    if !value.is_finite() {
        return Err(EncodeError::NonFiniteFloat { field });
    }
    Ok(value.to_string())
}

/// A bracketed float array; each element under the same finiteness contract as
/// `render_float`.
pub fn render_float_array(values: &[f64], field: &'static str) -> Result<String, EncodeError> {
    let mut elements = Vec::with_capacity(values.len());
    for &value in values {
        elements.push(render_float(value, field)?);
    }
    Ok(bracketed(elements))
}

/// A resource-tuple list (`[(2,100,100.5)]`) — `Cast.resources`. Elements
/// render via `render_float`'s shortest-form rule, not the corpus's original
/// fixed 2dp rendering (`(2,100.00,100.00)`) — DR-0002's contract only requires
/// `parse(encode(e)) == e`, which shortest-form already satisfies.
pub fn render_resource_tuples(
    tuples: &[(u32, f64, f64)],
    field: &'static str,
) -> Result<String, EncodeError> {
    let mut elements = Vec::with_capacity(tuples.len());
    for &(resource_type, current, max) in tuples {
        elements.push(format!(
            "({resource_type},{},{})",
            render_float(current, field)?,
            render_float(max, field)?
        ));
    }
    Ok(bracketed(elements))
}

/// An `(id, value)` pair list (`[(51,4),(42,4)]`) — gear stats/ability
/// grants/traits/gems, `CombatantInfo.trait_ranks`. `T` covers both `u32`
/// (grants/traits/gems/ranks) and `i64` (stats, which can be negative).
pub fn render_pair_list<T: std::fmt::Display>(pairs: &[(u32, T)]) -> String {
    bracketed(
        pairs
            .iter()
            .map(|(id, value)| format!("({id},{value})"))
            .collect(),
    )
}

/// The `maxHP|int|float|float|float` placeholder shape shared by the *middle*
/// five positions of every unit-state block in damage/heal, cast, and effect
/// lines — none of the three families retains any of these five. The block's
/// two ends still vary per caller: damage/heal prepends a real current-HP
/// value (the only family that retains one) and appends `[]` (it has no
/// resources field); cast prepends a `0` current-HP placeholder and appends
/// its real `resources` list; effect prepends `0` and appends `[]` (it retains
/// neither end).
pub(crate) const UNIT_STATE_FILLER: &str = "0|0|0.0|0.0|0.0";
