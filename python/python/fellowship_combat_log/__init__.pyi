"""Type stubs for `fellowship_combat_log`.

These mirror, field for field, the JSON shape the Rust extension (`src/bridge.rs`) actually
serializes: every discriminated union below uses a `"type"` key (`Literal[...]`) matching
`#[serde(tag = "type")]` on the corresponding Rust mirror enum, and every unit-only Rust enum
(`ResultTier`, `School`, ...) becomes a bare `Literal[...]` string union. Field names are the
decoder's own snake_case names verbatim — there is no renaming on the Python side.
"""

from typing import List, Literal, Optional, Tuple, TypedDict, Union

# --- Guid ---

class GuidPlayer(TypedDict):
    type: Literal["Player"]
    id: int

class GuidNpc(TypedDict):
    type: Literal["Npc"]
    spawn: int
    template: int

class GuidEnvironment(TypedDict):
    type: Literal["Environment"]

class GuidUnrecognized(TypedDict):
    type: Literal["Unrecognized"]

Guid = Union[GuidPlayer, GuidNpc, GuidEnvironment, GuidUnrecognized]

# --- unit-only enums (bare string literals) ---

ResultTier = Literal[
    "Hit",
    "CriticalStrike",
    "GrievousCriticalStrike",
    "Block",
    "Parry",
    "Dodge",
    "Miss",
    "None",
]

School = Literal["Physical", "Magical", "None"]

DamageHealKind = Literal[
    "AbilityDamage",
    "SwingDamage",
    "PeriodicDamage",
    "Heal",
    "PeriodicHeal",
    "LifestealHeal",
]

EffectPhase = Literal["Applied", "Removed", "Refreshed"]

Polarity = Literal["Buff", "Debuff"]

DeathKind = Literal["Unit", "Ally"]

# --- DamageHeal ---

class DamageHeal(TypedDict):
    kind: DamageHealKind
    source: Guid
    target: Guid
    ability_id: int
    parent_ability_id: int
    applied: int
    absorbed: int
    overkill: int
    blocked: int
    raw: int
    school: School
    result: ResultTier
    source_cur_hp: int
    target_cur_hp: int

# --- Cast / CastPhase ---

class CastPhaseActivated(TypedDict):
    type: Literal["Activated"]

class CastPhaseCastStart(TypedDict):
    type: Literal["CastStart"]
    cast_seconds: float

class CastPhaseCastSuccess(TypedDict):
    type: Literal["CastSuccess"]

class CastPhaseCastFail(TypedDict):
    type: Literal["CastFail"]
    reason: str

class CastPhaseChannelStart(TypedDict):
    type: Literal["ChannelStart"]
    cast_seconds: float

class CastPhaseChannelSuccess(TypedDict):
    type: Literal["ChannelSuccess"]

class CastPhaseChannelFail(TypedDict):
    type: Literal["ChannelFail"]
    reason: str

CastPhase = Union[
    CastPhaseActivated,
    CastPhaseCastStart,
    CastPhaseCastSuccess,
    CastPhaseCastFail,
    CastPhaseChannelStart,
    CastPhaseChannelSuccess,
    CastPhaseChannelFail,
]

class Cast(TypedDict):
    phase: CastPhase
    caster: Guid
    ability_id: int
    has_target: bool
    target: Guid
    resources: List[Tuple[int, float, float]]

# --- Effect ---

class Effect(TypedDict):
    phase: EffectPhase
    caster: Guid
    target: Guid
    effect_id: int
    duration_seconds: float
    stacks: int
    polarity: Polarity
    granting_ability_id: int
    refresher: Optional[Guid]

# --- ResourceChange ---

class ResourceChange(TypedDict):
    source: Guid
    owner: Guid
    resource_type: int
    delta: float
    current: float
    max: float
    causing_ability_id: int

# --- Encounter / EncounterPhase ---

class EncounterPhaseStart(TypedDict):
    type: Literal["Start"]

class EncounterPhaseEnd(TypedDict):
    type: Literal["End"]
    success: bool

EncounterPhase = Union[EncounterPhaseStart, EncounterPhaseEnd]

class Encounter(TypedDict):
    phase: EncounterPhase
    encounter_id: int
    bosses: List[str]

# --- Gear / CombatantInfo ---

class GearPiece(TypedDict):
    item_id: int
    item_level: int
    rarity: int
    temper: Tuple[int, int]
    stats: List[Tuple[int, int]]
    set_bonus_id: Optional[int]
    ability_grants: List[Tuple[int, int]]
    traits: List[Tuple[int, int]]
    gems: List[Tuple[int, int]]
    score: float

class NeckTraitChoice(TypedDict):
    trait_id: int
    selected: bool

class CombatantInfo(TypedDict):
    ulid: str
    player: Guid
    is_recording_player: bool
    hero_id: int
    item_level: float
    stat_sheet: List[float]
    talents: List[int]
    gem_power: List[float]
    gear: List[Optional[GearPiece]]
    trait_ranks: List[Tuple[int, int]]
    neck_traits: List[NeckTraitChoice]

# --- DamageAbsorbed / Interrupt / Dispel ---

class DamageAbsorbed(TypedDict):
    shield_caster: Guid
    shielded: Guid
    shield_effect_id: int
    absorbed: int
    attacker: Guid
    attacking_ability_id: int

class Interrupt(TypedDict):
    interrupter: Guid
    victim: Guid
    interrupting_ability_id: int
    interrupted_ability_id: int

class Dispel(TypedDict):
    dispeller: Guid
    target: Guid
    dispel_ability_id: int
    removed_effect_id: int
    remaining_seconds: float
    polarity: Polarity

# --- Death / Resurrect ---

class Death(TypedDict):
    kind: DeathKind
    dead: Guid
    killer: Guid
    killing_ability_id: int

class Resurrect(TypedDict):
    resurrecter: Guid
    target: Guid
    ability_id: int

# --- LoggingStarted / ZoneChange / MapChange / DungeonStart / DungeonEnd ---

class LoggingStarted(TypedDict):
    log_format_version: int
    game_build: str

class ZoneChange(TypedDict):
    zone_name: str
    zone_id: int
    difficulty: int

class MapChange(TypedDict):
    map_id: int
    floor_name: str

class DungeonStart(TypedDict):
    name: str
    zone_id: int
    key_level: int
    modifiers: List[int]

class DungeonEnd(TypedDict):
    name: str
    zone_id: int
    key_level: int
    success: bool
    duration_ms: int
    score: float

# --- Marker / WorldMarker ---

class Marker(TypedDict):
    unit: Guid
    index: int
    removed: bool

class WorldMarker(TypedDict):
    x: float
    y: float
    slot: int
    removed: bool

# --- Event / EventBody ---

class LogInstant(TypedDict):
    utc_ms: int
    seq: int

class EventBodyDamageHeal(DamageHeal):
    type: Literal["DamageHeal"]

class EventBodyDamageAbsorbed(DamageAbsorbed):
    type: Literal["DamageAbsorbed"]

class EventBodyCast(Cast):
    type: Literal["Cast"]

class EventBodyEffect(Effect):
    type: Literal["Effect"]

class EventBodyResourceChange(ResourceChange):
    type: Literal["ResourceChange"]

class EventBodyInterrupt(Interrupt):
    type: Literal["Interrupt"]

class EventBodyDispel(Dispel):
    type: Literal["Dispel"]

class EventBodyDeath(Death):
    type: Literal["Death"]

class EventBodyUnitDestroyed(TypedDict):
    type: Literal["UnitDestroyed"]
    unit: Guid

class EventBodyResurrect(Resurrect):
    type: Literal["Resurrect"]

class EventBodyEncounter(Encounter):
    type: Literal["Encounter"]

class EventBodyLoggingStarted(LoggingStarted):
    type: Literal["LoggingStarted"]

class EventBodyZoneChange(ZoneChange):
    type: Literal["ZoneChange"]

class EventBodyMapChange(MapChange):
    type: Literal["MapChange"]

class EventBodyDungeonStart(DungeonStart):
    type: Literal["DungeonStart"]

class EventBodyDungeonEnd(DungeonEnd):
    type: Literal["DungeonEnd"]

class EventBodyMarker(Marker):
    type: Literal["Marker"]

class EventBodyWorldMarker(WorldMarker):
    type: Literal["WorldMarker"]

class EventBodyCombatantInfo(CombatantInfo):
    type: Literal["CombatantInfo"]

class EventBodyInvalid(TypedDict):
    """The vestigial `EVENT_INVALID` line — damage-shaped but safe to drop."""

    type: Literal["Invalid"]

class EventBodyUnknown(TypedDict):
    type: Literal["Unknown"]
    raw_type: str
    raw_fields: List[str]

EventBody = Union[
    EventBodyDamageHeal,
    EventBodyDamageAbsorbed,
    EventBodyCast,
    EventBodyEffect,
    EventBodyResourceChange,
    EventBodyInterrupt,
    EventBodyDispel,
    EventBodyDeath,
    EventBodyUnitDestroyed,
    EventBodyResurrect,
    EventBodyEncounter,
    EventBodyLoggingStarted,
    EventBodyZoneChange,
    EventBodyMapChange,
    EventBodyDungeonStart,
    EventBodyDungeonEnd,
    EventBodyMarker,
    EventBodyWorldMarker,
    EventBodyCombatantInfo,
    EventBodyInvalid,
    EventBodyUnknown,
]

class Event(TypedDict):
    instant: LogInstant
    body: EventBody

# --- combatants.Combatant ---

class Combatant(TypedDict):
    ulid: str
    name: str
    is_recording_player: bool
    info: CombatantInfo
    snapshot_count: int

# --- module functions ---

def parse_line(seq: int, line: str) -> Event:
    """Decode one v8 combat-log line into a typed `Event` dict.

    Raises `ValueError` with a clear message if `line` is malformed.
    """

def list_combatants(log: str) -> List[Combatant]:
    """Every combatant `log` mentions, in first-seen order, each with its latest gear snapshot.

    Never raises: a malformed or unparseable line is skipped rather than aborting the scan.
    """

def version() -> str:
    """The decoder's version (`fellowship_combat_log`'s `CARGO_PKG_VERSION`)."""
