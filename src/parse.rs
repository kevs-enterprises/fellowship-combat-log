//! v8 combat-log line parsing (see the v8 combat-log format reference). Total
//! over its input — a malformed, short, or unrecognized line yields an
//! `Unknown` body or a `ParseError`, never a panic — because the corpus contains
//! routine quirks (u32-wrapped HP, negative amplification) that must decode
//! without choking even though this layer doesn't yet interpret them. Naive
//! pipe-splitting the whole line is always safe (no `|` inside quoted strings or
//! bracketed lists); splitting a *bracketed* field on commas is never safe, so
//! bracket contents go through a nesting-aware tokenizer.

use crate::event::{
    Cast, CastPhase, CombatantInfo, DamageAbsorbed, DamageHeal, DamageHealKind, Death, DeathKind,
    Dispel, DungeonEnd, DungeonStart, Effect, EffectPhase, Encounter, EncounterPhase, Event,
    EventBody, Guid, Interrupt, LoggingStarted, MapChange, Marker, Polarity, ResourceChange,
    ResultTier, Resurrect, School, WorldMarker, ZoneChange,
};
use crate::timestamp::parse_instant;

/// Why a line could not be parsed. Total parsing hands callers an `Err`, never a
/// panic.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseError {
    Empty,
    TooFewFields { expected: usize, got: usize },
    BadField { field: usize, reason: &'static str },
}

/// Parse one v8 log line. `seq` is the file line number — the ordering tiebreaker
/// for sub-millisecond ties. Field 1 is the timestamp; field 2 selects the family.
pub fn parse_line(seq: u32, text: &str) -> Result<Event, ParseError> {
    let line = text.trim_end_matches(['\n', '\r']);
    if line.is_empty() {
        return Err(ParseError::Empty);
    }
    // A trailing `|` yields an empty last field — `split` keeps it, so per-family
    // field counts stay exact (some families legitimately end in `|`).
    let fields: Vec<&str> = line.split('|').collect();
    if fields.len() < 2 {
        return Err(ParseError::TooFewFields {
            expected: 2,
            got: fields.len(),
        });
    }
    let instant = parse_instant(seq, fields[0]).ok_or(ParseError::BadField {
        field: 1,
        reason: "timestamp",
    })?;
    Ok(Event {
        instant,
        body: parse_body(&fields)?,
    })
}

fn parse_body(fields: &[&str]) -> Result<EventBody, ParseError> {
    use DamageHealKind::*;
    match fields[1] {
        "ABILITY_DAMAGE" => damage_heal(fields, AbilityDamage),
        "SWING_DAMAGE" => damage_heal(fields, SwingDamage),
        "ABILITY_PERIODIC_DAMAGE" => damage_heal(fields, PeriodicDamage),
        "ABILITY_HEAL" => damage_heal(fields, Heal),
        "ABILITY_PERIODIC_HEAL" => damage_heal(fields, PeriodicHeal),
        "ABILITY_LIFESTEAL_HEAL" => damage_heal(fields, LifestealHeal),
        "ABILITY_ACTIVATED" => cast(fields, CastPhase::Activated),
        "ABILITY_CAST_SUCCESS" => cast(fields, CastPhase::CastSuccess),
        "ABILITY_CHANNEL_SUCCESS" => cast(fields, CastPhase::ChannelSuccess),
        "ABILITY_CAST_START" => {
            cast_timed(fields, |cast_seconds| CastPhase::CastStart { cast_seconds })
        }
        "ABILITY_CHANNEL_START" => cast_timed(fields, |cast_seconds| CastPhase::ChannelStart {
            cast_seconds,
        }),
        "ABILITY_CAST_FAIL" => cast_failed(fields, |reason| CastPhase::CastFail { reason }),
        "ABILITY_CHANNEL_FAIL" => cast_failed(fields, |reason| CastPhase::ChannelFail { reason }),
        "EFFECT_APPLIED" => effect(fields, EffectPhase::Applied),
        "EFFECT_REMOVED" => effect(fields, EffectPhase::Removed),
        "EFFECT_REFRESHED" => effect(fields, EffectPhase::Refreshed),
        "RESOURCE_CHANGED" => resource_changed(fields),
        "DAMAGE_ABSORBED" => damage_absorbed(fields),
        "ABILITY_INTERRUPT" => interrupt(fields),
        "ABILITY_DISPEL" => dispel(fields),
        "UNIT_DEATH" => death(fields, DeathKind::Unit),
        "ALLY_DEATH" => death(fields, DeathKind::Ally),
        "UNIT_DESTROYED" => Ok(EventBody::UnitDestroyed {
            unit: guid(fields, 3, "unit")?,
        }),
        "RESURRECT" => resurrect(fields),
        "ENCOUNTER_START" => encounter(fields, false),
        "ENCOUNTER_END" => encounter(fields, true),
        "LOGGING_STARTED" => logging_started(fields),
        "ZONE_CHANGE" => zone_change(fields),
        "MAP_CHANGE" => map_change(fields),
        "DUNGEON_START" => dungeon_start(fields),
        "DUNGEON_END" => dungeon_end(fields),
        "MARKER_PLACED" => marker(fields, false),
        "MARKER_REMOVED" => marker(fields, true),
        "WORLD_MARKER_PLACED" => world_marker(fields, false),
        "WORLD_MARKER_REMOVED" => world_marker(fields, true),
        "COMBATANT_INFO" => combatant_info(fields),
        // Vestigial: damage-shaped but safe to drop, so it is never folded.
        "EVENT_INVALID" => Ok(EventBody::Invalid),
        other => Ok(EventBody::Unknown {
            raw_type: other.to_string(),
        }),
    }
}

// --- field accessors (1-based, matching the reference's f-numbers) ---

fn field<'a>(fields: &[&'a str], n: usize) -> Result<&'a str, ParseError> {
    fields.get(n - 1).copied().ok_or(ParseError::TooFewFields {
        expected: n,
        got: fields.len(),
    })
}

fn number<T: std::str::FromStr>(
    fields: &[&str],
    n: usize,
    reason: &'static str,
) -> Result<T, ParseError> {
    field(fields, n)?
        .parse()
        .map_err(|_| ParseError::BadField { field: n, reason })
}

fn guid(fields: &[&str], n: usize, reason: &'static str) -> Result<Guid, ParseError> {
    parse_guid(field(fields, n)?).ok_or(ParseError::BadField { field: n, reason })
}

// --- family decoders (only the fields downstream needs; unit-state deferred) ---

/// Shared 30-field damage/heal anatomy. The source/target unit-state blocks
/// (f17–30) are decoded once a consumer reads them.
fn damage_heal(fields: &[&str], kind: DamageHealKind) -> Result<EventBody, ParseError> {
    Ok(EventBody::DamageHeal(DamageHeal {
        kind,
        source: guid(fields, 3, "source")?,
        target: guid(fields, 5, "target")?,
        ability_id: number(fields, 7, "ability id")?,
        parent_ability_id: number(fields, 9, "parent ability")?,
        applied: number(fields, 10, "applied")?,
        absorbed: number(fields, 11, "absorbed")?,
        overkill: number(fields, 12, "overkill")?,
        blocked: number(fields, 13, "blocked")?,
        raw: number(fields, 14, "raw")?,
        school: parse_school(field(fields, 15)?)?,
        result: parse_result(field(fields, 16)?)?,
    }))
}

/// Cast/channel base (16 fields): f3 caster, f5 ability id, f7 has-target,
/// f8 target. The caster unit-state block (f10–16) is decoded once needed.
fn cast_common(fields: &[&str], phase: CastPhase) -> Result<Cast, ParseError> {
    Ok(Cast {
        phase,
        caster: guid(fields, 3, "caster")?,
        ability_id: number(fields, 5, "ability id")?,
        has_target: field(fields, 7)? == "1",
        target: guid(fields, 8, "target")?,
    })
}

fn cast(fields: &[&str], phase: CastPhase) -> Result<EventBody, ParseError> {
    Ok(EventBody::Cast(cast_common(fields, phase)?))
}

/// A `*_START` cast/channel: the base plus the trailing f17 cast-time float. The
/// caller supplies the phase constructor, so the start/channel split can't be
/// mismatched.
fn cast_timed(
    fields: &[&str],
    make_phase: impl Fn(f64) -> CastPhase,
) -> Result<EventBody, ParseError> {
    let cast_seconds = number(fields, 17, "cast time")?;
    Ok(EventBody::Cast(cast_common(
        fields,
        make_phase(cast_seconds),
    )?))
}

/// A `*_FAIL` cast/channel: the base plus the trailing f17 quoted reason.
fn cast_failed(
    fields: &[&str],
    make_phase: impl Fn(String) -> CastPhase,
) -> Result<EventBody, ParseError> {
    let reason = unquote(field(fields, 17)?).to_string();
    Ok(EventBody::Cast(cast_common(fields, make_phase(reason))?))
}

/// Effect (aura) events (21 fields; refreshed = 23 with the trailing refresher).
fn effect(fields: &[&str], phase: EffectPhase) -> Result<EventBody, ParseError> {
    Ok(EventBody::Effect(Effect {
        phase,
        caster: guid(fields, 3, "caster")?,
        target: guid(fields, 5, "target")?,
        effect_id: number(fields, 7, "effect id")?,
        duration_seconds: number(fields, 9, "duration")?,
        stacks: number(fields, 10, "stacks")?,
        polarity: parse_polarity(field(fields, 11)?)?,
        granting_ability_id: number(fields, 19, "granting ability")?,
        refresher: match phase {
            EffectPhase::Refreshed => Some(guid(fields, 22, "refresher")?),
            _ => None,
        },
    }))
}

/// `RESOURCE_CHANGED` (13 fields) — scalar fields only.
fn resource_changed(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::ResourceChange(ResourceChange {
        source: guid(fields, 3, "source")?,
        owner: guid(fields, 5, "owner")?,
        resource_type: number(fields, 7, "resource type")?,
        delta: number(fields, 8, "delta")?,
        current: number(fields, 9, "current")?,
        max: number(fields, 10, "max")?,
        causing_ability_id: number(fields, 12, "causing ability")?,
    }))
}

/// `ENCOUNTER_START` (4 fields) / `ENCOUNTER_END` (5 fields, trailing success).
fn encounter(fields: &[&str], has_success: bool) -> Result<EventBody, ParseError> {
    let phase = if has_success {
        EncounterPhase::End {
            success: field(fields, 5)? == "1",
        }
    } else {
        EncounterPhase::Start
    };
    Ok(EventBody::Encounter(Encounter {
        phase,
        encounter_id: number(fields, 3, "encounter id")?,
        bosses: parse_name_array(field(fields, 4)?)?,
    }))
}

/// `COMBATANT_INFO` (20 fields) framed at the top level: the deeply-nested stat/
/// gear/talent payload and the catalog name-join are decoded by a later slice.
fn combatant_info(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::CombatantInfo(CombatantInfo {
        ulid: field(fields, 3)?.to_string(),
        player: guid(fields, 4, "player")?,
        is_recording_player: field(fields, 6)? == "1",
        hero_id: number(fields, 7, "hero id")?,
    }))
}

/// `DAMAGE_ABSORBED` (14 fields): caster-first (f3/f4 shield caster, f5/f6 shielded).
fn damage_absorbed(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::DamageAbsorbed(DamageAbsorbed {
        shield_caster: guid(fields, 3, "shield caster")?,
        shielded: guid(fields, 5, "shielded")?,
        shield_effect_id: number(fields, 7, "shield effect")?,
        absorbed: number(fields, 9, "absorbed")?,
        attacker: guid(fields, 10, "attacker")?,
        attacking_ability_id: number(fields, 12, "attacking ability")?,
    }))
}

/// `ABILITY_INTERRUPT` (10 fields).
fn interrupt(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::Interrupt(Interrupt {
        interrupter: guid(fields, 3, "interrupter")?,
        victim: guid(fields, 5, "victim")?,
        interrupting_ability_id: number(fields, 7, "interrupting ability")?,
        interrupted_ability_id: number(fields, 9, "interrupted ability")?,
    }))
}

/// `ABILITY_DISPEL` (12 fields).
fn dispel(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::Dispel(Dispel {
        dispeller: guid(fields, 3, "dispeller")?,
        target: guid(fields, 5, "target")?,
        dispel_ability_id: number(fields, 7, "dispel ability")?,
        removed_effect_id: number(fields, 9, "removed effect")?,
        remaining_seconds: number(fields, 11, "remaining")?,
        polarity: parse_polarity(field(fields, 12)?)?,
    }))
}

/// `UNIT_DEATH` / `ALLY_DEATH` (10 fields).
fn death(fields: &[&str], kind: DeathKind) -> Result<EventBody, ParseError> {
    Ok(EventBody::Death(Death {
        kind,
        dead: guid(fields, 3, "dead unit")?,
        killer: guid(fields, 5, "killer")?,
        killing_ability_id: number(fields, 7, "killing ability")?,
    }))
}

/// `RESURRECT` (9 fields, trailing empty).
fn resurrect(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::Resurrect(Resurrect {
        resurrecter: guid(fields, 3, "resurrecter")?,
        target: guid(fields, 5, "target")?,
        ability_id: number(fields, 7, "ability id")?,
    }))
}

/// `LOGGING_STARTED` (5 fields): the game build (f4) is unquoted and contains a space.
fn logging_started(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::LoggingStarted(LoggingStarted {
        log_format_version: number(fields, 3, "log format version")?,
        game_build: field(fields, 4)?.to_string(),
    }))
}

/// `ZONE_CHANGE` (6 fields).
fn zone_change(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::ZoneChange(ZoneChange {
        zone_name: unquote(field(fields, 3)?).to_string(),
        zone_id: number(fields, 4, "zone id")?,
        difficulty: number(fields, 5, "difficulty")?,
    }))
}

/// `MAP_CHANGE` (8 fields); the bounding-box floats are decoded once a consumer needs them.
fn map_change(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::MapChange(MapChange {
        map_id: number(fields, 3, "map id")?,
        floor_name: unquote(field(fields, 4)?).to_string(),
    }))
}

/// `DUNGEON_START` (9 fields).
fn dungeon_start(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::DungeonStart(DungeonStart {
        name: unquote(field(fields, 3)?).to_string(),
        zone_id: number(fields, 4, "zone id")?,
        key_level: number(fields, 5, "key level")?,
        modifiers: parse_int_array(field(fields, 6)?, 6)?,
    }))
}

/// `DUNGEON_END` (12 fields). A failed/abandoned run logs success=0.
fn dungeon_end(fields: &[&str]) -> Result<EventBody, ParseError> {
    Ok(EventBody::DungeonEnd(DungeonEnd {
        name: unquote(field(fields, 3)?).to_string(),
        zone_id: number(fields, 4, "zone id")?,
        key_level: number(fields, 5, "key level")?,
        success: field(fields, 7)? == "1",
        duration_ms: number(fields, 8, "duration")?,
        score: number(fields, 9, "score")?,
    }))
}

/// `MARKER_PLACED` / `MARKER_REMOVED` (5 fields).
fn marker(fields: &[&str], removed: bool) -> Result<EventBody, ParseError> {
    Ok(EventBody::Marker(Marker {
        unit: guid(fields, 3, "unit")?,
        index: number(fields, 5, "marker index")?,
        removed,
    }))
}

/// `WORLD_MARKER_PLACED` / `WORLD_MARKER_REMOVED` (5 fields).
fn world_marker(fields: &[&str], removed: bool) -> Result<EventBody, ParseError> {
    Ok(EventBody::WorldMarker(WorldMarker {
        x: number(fields, 3, "world x")?,
        y: number(fields, 4, "world y")?,
        slot: number(fields, 5, "marker slot")?,
        removed,
    }))
}

// --- primitive decoders ---

/// Decode a unit id across the four v8 namespaces. `None` when the namespace or
/// its numeric parts don't parse, so the caller can report which field failed.
fn parse_guid(s: &str) -> Option<Guid> {
    if s == "Environment-0" {
        return Some(Guid::Environment);
    }
    if s == "UnrecognizedType-0" {
        return Some(Guid::Unrecognized);
    }
    if let Some(rest) = s.strip_prefix("Player-") {
        return rest.parse::<u32>().ok().map(Guid::Player);
    }
    if let Some(rest) = s.strip_prefix("Npc-") {
        let (spawn, template) = rest.split_once('-')?;
        return Some(Guid::Npc {
            spawn: spawn.parse().ok()?,
            template: template.parse().ok()?,
        });
    }
    None
}

fn parse_school(s: &str) -> Result<School, ParseError> {
    match s {
        "Physical" => Ok(School::Physical),
        "Magical" => Ok(School::Magical),
        "None" => Ok(School::None),
        _ => Err(ParseError::BadField {
            field: 15,
            reason: "school",
        }),
    }
}

fn parse_result(s: &str) -> Result<ResultTier, ParseError> {
    match s {
        "Hit" => Ok(ResultTier::Hit),
        "CriticalStrike" => Ok(ResultTier::CriticalStrike),
        "GrievousCriticalStrike" => Ok(ResultTier::GrievousCriticalStrike),
        "Block" => Ok(ResultTier::Block),
        "Parry" => Ok(ResultTier::Parry),
        "Dodge" => Ok(ResultTier::Dodge),
        "Miss" => Ok(ResultTier::Miss),
        "None" => Ok(ResultTier::None),
        _ => Err(ParseError::BadField {
            field: 16,
            reason: "result tier",
        }),
    }
}

fn parse_polarity(s: &str) -> Result<Polarity, ParseError> {
    match s {
        "BUFF" => Ok(Polarity::Buff),
        "DEBUFF" => Ok(Polarity::Debuff),
        _ => Err(ParseError::BadField {
            field: 11,
            reason: "polarity",
        }),
    }
}

/// Strip one layer of surrounding double quotes; no backslash escaping exists in
/// the corpus, so a quoted string is simply `"…"`.
fn unquote(s: &str) -> &str {
    s.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .unwrap_or(s)
}

/// A quoted-name array (`["Boss A","Xul, The Blood Monolith"]`) → the names. Uses
/// the nesting-aware tokenizer so a name's internal comma never splits an element.
fn parse_name_array(s: &str) -> Result<Vec<String>, ParseError> {
    let elements = split_bracket_list(s).ok_or(ParseError::BadField {
        field: 4,
        reason: "name array",
    })?;
    Ok(elements
        .iter()
        .map(|element| unquote(element.trim()).to_string())
        .collect())
}

/// A bracketed integer array (`[4,6,8,19]`) → the ids; `[]` yields no ids.
fn parse_int_array(s: &str, field: usize) -> Result<Vec<u32>, ParseError> {
    let elements = split_bracket_list(s).ok_or(ParseError::BadField {
        field,
        reason: "int array",
    })?;
    elements
        .iter()
        .map(|element| {
            element
                .trim()
                .parse::<u32>()
                .map_err(|_| ParseError::BadField {
                    field,
                    reason: "int array element",
                })
        })
        .collect()
}

/// Split a bracketed list (`[a,(b,c),["d","e"]]`) into its top-level element
/// slices, respecting quoted strings and nested brackets/parens so a comma inside
/// a quote or a nested group never splits. Input includes the outer `[` `]`; the
/// nesting-awareness is what a deeper nested-payload decode will reuse. `None`
/// when the outer brackets are absent.
fn split_bracket_list(s: &str) -> Option<Vec<&str>> {
    let inner = s.strip_prefix('[')?.strip_suffix(']')?;
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut elements = Vec::new();
    let mut depth = 0i32;
    let mut in_quote = false;
    let mut start = 0;
    for (i, byte) in inner.bytes().enumerate() {
        match byte {
            b'"' => in_quote = !in_quote,
            b'[' | b'(' if !in_quote => depth += 1,
            b']' | b')' if !in_quote => depth -= 1,
            b',' if !in_quote && depth == 0 => {
                elements.push(&inner[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    elements.push(&inner[start..]);
    Some(elements)
}
