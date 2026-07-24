//! v8 combat-log line parsing (see the v8 combat-log format reference). Total
//! over its input — a malformed, short, or unrecognized line yields an
//! `Event::Unknown` or a `ParseError`, never a panic — because the corpus
//! contains routine quirks (u32-wrapped HP, negative amplification) that must
//! decode without choking even though this layer doesn't yet interpret them.
//! Naive pipe-splitting is safe corpus-wide: no `|` occurs inside quoted strings
//! or bracketed lists.

/// A combat-log unit id (§2). `Player` ids are ephemeral u32s (reassigned per
/// dungeon instance); `Npc` ids carry a stable creature-template suffix; the
/// `Environment`/`Unrecognized` namespaces are game content, never anonymized.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Guid {
    Player(u32),
    Npc { spawn: u32, template: u32 },
    Environment,
    Unrecognized,
}

/// Attack/heal result tier (f16). Crits are indicated *only* here — there is no
/// separate crit-amount field (§3.1).
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

/// Damage school (f15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum School {
    Physical,
    Magical,
    None,
}

/// A damage/heal event decoded from the shared 30-field anatomy. Only the fields
/// the extract folds on are decoded here; the full source/target unit-state
/// blocks (f17–30) and heal-specific semantics are decoded once a consumer needs
/// them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DamageEvent {
    pub source: Guid,
    pub target: Guid,
    pub ability_id: u32,
    /// f10: amount actually applied to health (damage dealt / effective healing).
    pub applied: i64,
    /// f14: raw/base amount before target-side mitigation (0 for heals).
    pub raw: i64,
    pub school: School,
    pub result: ResultTier,
}

/// One parsed log line. Unknown event types are surfaced, never dropped, so a
/// later event-family sweep can find the long tail.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Event {
    Damage(DamageEvent),
    Unknown { raw_type: String },
}

/// Why a line could not be parsed. Total parsing hands callers an `Err`, never a
/// panic.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ParseError {
    Empty,
    TooFewFields { expected: usize, got: usize },
    BadField { field: usize, reason: &'static str },
}

/// The six damage/heal families that share the 30-field anatomy. The
/// damage-shaped but vestigial `EVENT_INVALID` line is deliberately excluded — it
/// surfaces as an `Unknown` so it can never be folded as real damage; giving it
/// proper drop semantics is a later concern.
const DAMAGE_HEAL_TYPES: &[&str] = &[
    "ABILITY_DAMAGE",
    "SWING_DAMAGE",
    "ABILITY_PERIODIC_DAMAGE",
    "ABILITY_HEAL",
    "ABILITY_PERIODIC_HEAL",
    "ABILITY_LIFESTEAL_HEAL",
];

/// Parse one v8 log line. `seq` is the file line number — the ordering tiebreaker
/// for the sub-millisecond ties the ordered event stream will need; it is part of
/// the surface now so that stream doesn't force a later signature change. Field 2
/// selects the event family.
pub fn parse_line(seq: u32, text: &str) -> Result<Event, ParseError> {
    let _ = seq;
    let line = text.trim_end_matches(['\n', '\r']);
    if line.is_empty() {
        return Err(ParseError::Empty);
    }
    let fields: Vec<&str> = line.split('|').collect();
    if fields.len() < 2 {
        return Err(ParseError::TooFewFields {
            expected: 2,
            got: fields.len(),
        });
    }
    if DAMAGE_HEAL_TYPES.contains(&fields[1]) {
        parse_damage(&fields).map(Event::Damage)
    } else {
        Ok(Event::Unknown {
            raw_type: fields[1].to_string(),
        })
    }
}

/// Decode the shared 30-field damage/heal anatomy. Only the fields through f16
/// are needed so far; the source/target unit-state blocks (f17–30) are decoded
/// once a consumer reads them.
fn parse_damage(fields: &[&str]) -> Result<DamageEvent, ParseError> {
    if fields.len() < 16 {
        return Err(ParseError::TooFewFields {
            expected: 16,
            got: fields.len(),
        });
    }
    let source = parse_guid(fields[2]).ok_or(ParseError::BadField {
        field: 3,
        reason: "source guid",
    })?;
    let target = parse_guid(fields[4]).ok_or(ParseError::BadField {
        field: 5,
        reason: "target guid",
    })?;
    let ability_id = parse_num(fields[6], 7, "ability id")?;
    let applied = parse_num(fields[9], 10, "applied amount")?;
    let raw = parse_num(fields[13], 14, "raw amount")?;
    let school = parse_school(fields[14])?;
    let result = parse_result(fields[15])?;
    Ok(DamageEvent {
        source,
        target,
        ability_id,
        applied,
        raw,
        school,
        result,
    })
}

/// Decode a unit id across the four v8 namespaces (§2). Returns `None` when the
/// namespace or its numeric parts don't parse, so the caller can report which
/// field failed.
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

fn parse_num<T: std::str::FromStr>(
    s: &str,
    field: usize,
    reason: &'static str,
) -> Result<T, ParseError> {
    s.parse::<T>()
        .map_err(|_| ParseError::BadField { field, reason })
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
