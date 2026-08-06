// Hand-written mirror of the JSON shape `../src/bridge.rs` serializes, field for field and
// variant for variant. `wasm-bindgen` can only describe a `JsValue`-returning export as `any` in
// the `.d.ts` it generates, so these are the types that actually give consumers a typed API — see
// `index.ts`, which wraps the raw generated bindings with them.
//
// Conventions (matching bridge.rs's doc comment on the Rust side):
//   - every field name is camelCase (idiomatic JS), mirroring the Rust source's snake_case;
//   - a pure unit-only Rust enum (no variant carries data) is a plain string-literal union
//     (`ResultTier`, `School`, ...) — serde's default (externally tagged) representation of a
//     fieldless variant is just its bare variant name string;
//   - an enum that mixes unit and data-carrying variants (`CastPhase`, `EncounterPhase`,
//     `EventBody`) is a discriminated union tagged by a `type` field, matching the Rust side's
//     internally tagged representation. A variant that wraps a whole mirror struct (e.g.
//     `EventBody`'s `DamageHeal`) has that struct's fields hoisted to the top level alongside
///     `type`, rather than nested under a payload key — that's how serde's internal tagging
//     flattens a newtype variant wrapping a struct;
//   - `Guid` is the one enum wrapping a bare scalar on the Rust side (`Guid.Player(u32)`); since
//     an internally tagged enum can't flatten a bare scalar, `Player` carries a named `id` field
//     instead;
//   - an absent Rust `Option` serializes as JSON `null` (never an absent/`undefined` key), so
//     every optional field here is typed `T | null`, not `T | undefined`.

// --- Guid ---

export type Guid =
  | { type: "Player"; id: number }
  | { type: "Npc"; spawn: number; template: number }
  | { type: "Environment" }
  | { type: "Unrecognized" };

// --- unit-only enums: plain string-literal unions ---

export type ResultTier =
  | "Hit"
  | "CriticalStrike"
  | "GrievousCriticalStrike"
  | "Block"
  | "Parry"
  | "Dodge"
  | "Miss"
  | "None";

export type School = "Physical" | "Magical" | "None";

export type DamageHealKind =
  | "AbilityDamage"
  | "SwingDamage"
  | "PeriodicDamage"
  | "Heal"
  | "PeriodicHeal"
  | "LifestealHeal";

export type EffectPhase = "Applied" | "Removed" | "Refreshed";

export type Polarity = "Buff" | "Debuff";

export type DeathKind = "Unit" | "Ally";

// --- DamageHeal ---

export interface DamageHeal {
  kind: DamageHealKind;
  source: Guid;
  target: Guid;
  abilityId: number;
  parentAbilityId: number;
  /** Amount actually applied to health (damage dealt / effective healing). */
  applied: number;
  /** Absorbed amount — can be negative on damage (amplification component). */
  absorbed: number;
  /** Overkill (damage) / overheal (heal); -1 = none. */
  overkill: number;
  /** Blocked amount (nonzero only with result `"Block"`). */
  blocked: number;
  /** Raw/base amount before mitigation; always 0 for heals. */
  raw: number;
  school: School;
  result: ResultTier;
  /** Source current HP, u32-wrap normalized (see the Rust doc comment on the field). */
  sourceCurHp: number;
  /** Target current HP, u32-wrap normalized. */
  targetCurHp: number;
}

// --- Cast / CastPhase ---

export type CastPhase =
  | { type: "Activated" }
  | { type: "CastStart"; castSeconds: number }
  | { type: "CastSuccess" }
  | { type: "CastFail"; reason: string }
  | { type: "ChannelStart"; castSeconds: number }
  | { type: "ChannelSuccess" }
  | { type: "ChannelFail"; reason: string };

export interface Cast {
  phase: CastPhase;
  caster: Guid;
  abilityId: number;
  hasTarget: boolean;
  target: Guid;
  /** The caster's resource snapshot at cast time — `[type, current, max]` triples. */
  resources: Array<[number, number, number]>;
}

// --- Effect ---

export interface Effect {
  phase: EffectPhase;
  caster: Guid;
  target: Guid;
  effectId: number;
  /** Duration seconds (-1 = infinite/permanent; 0 on removal). */
  durationSeconds: number;
  stacks: number;
  polarity: Polarity;
  /** Granting ability id — the tick-attribution anchor for DoTs/HoTs. */
  grantingAbilityId: number;
  /** The unit that refreshed the aura (refreshed events only), often an NPC. */
  refresher: Guid | null;
}

// --- ResourceChange ---

export interface ResourceChange {
  source: Guid;
  owner: Guid;
  /** Resource type id — a per-class slot, not a global identity. */
  resourceType: number;
  delta: number;
  current: number;
  /** Max (-1.0 = no max). */
  max: number;
  /** Causing ability id (0 = passive). */
  causingAbilityId: number;
}

// --- Encounter / EncounterPhase ---

export type EncounterPhase = { type: "Start" } | { type: "End"; success: boolean };

export interface Encounter {
  phase: EncounterPhase;
  encounterId: number;
  bosses: string[];
}

// --- Gear / CombatantInfo ---

export interface GearPiece {
  /** The equipped item (`ItemID.*`). Legendaries are marked in the tag itself. */
  itemId: number;
  itemLevel: number;
  /** Index into the game's rarity list (`Common`...`Legendary`). */
  rarity: number;
  /** `[current, max]` temper. */
  temper: [number, number];
  /** `[attribute id, value]` pairs — the piece's fixed slots first, then its rolls. */
  stats: Array<[number, number]>;
  /** The set this piece carries (`ItemID.SetBonus.*`); a set activates across two carriers. */
  setBonusId: number | null;
  /** `[rank id, rank]` pairs — gear-granted hero abilities, not traits. */
  abilityGrants: Array<[number, number]>;
  /** `[trait id, level]` pairs in the `ItemTrait.ID` namespace. */
  traits: Array<[number, number]>;
  /** `[gem colour id, power]` pairs — the piece's attunements (`ItemID.GemType.*`). */
  gems: Array<[number, number]>;
  score: number;
}

export interface NeckTraitChoice {
  traitId: number;
  selected: boolean;
}

export interface CombatantInfo {
  /** The persistent character ULID (1:1 with character name). */
  ulid: string;
  /** The run-scoped player id. */
  player: Guid;
  /** The recording-player flag (true exactly once per dungeon = the owner). */
  isRecordingPlayer: boolean;
  heroId: number;
  /** Average item level (the mean of the 14 gear ilvls). */
  itemLevel: number;
  /** The final computed stat sheet — post-diminishing-returns, post-set-bonus. */
  statSheet: number[];
  /** Hero talent picks. */
  talents: number[];
  /** Total gem power per colour, in the game's alphabetical gem order. */
  gemPower: number[];
  /**
   * The equipped pieces, one entry per gear slot in the game's own slot order. Slot identity is
   * positional, so a piece that fails to decode is `null` in place rather than omitted.
   */
  gear: Array<GearPiece | null>;
  /** `[trait id, rank]` pairs for every trait realized on the build. */
  traitRanks: Array<[number, number]>;
  /** The neck-trait candidates and which are chosen. */
  neckTraits: NeckTraitChoice[];
}

// --- DamageAbsorbed / Interrupt / Dispel ---

export interface DamageAbsorbed {
  shieldCaster: Guid;
  shielded: Guid;
  shieldEffectId: number;
  /** Amount absorbed (matches the same-ms damage line's `absorbed` field). */
  absorbed: number;
  attacker: Guid;
  attackingAbilityId: number;
}

export interface Interrupt {
  interrupter: Guid;
  victim: Guid;
  interruptingAbilityId: number;
  interruptedAbilityId: number;
}

export interface Dispel {
  dispeller: Guid;
  target: Guid;
  dispelAbilityId: number;
  removedEffectId: number;
  /** The removed effect's remaining duration — can be negative. */
  remainingSeconds: number;
  polarity: Polarity;
}

// --- Death / Resurrect ---

export interface Death {
  kind: DeathKind;
  dead: Guid;
  killer: Guid;
  killingAbilityId: number;
}

export interface Resurrect {
  resurrecter: Guid;
  target: Guid;
  abilityId: number;
}

// --- LoggingStarted / ZoneChange / MapChange / DungeonStart / DungeonEnd ---

export interface LoggingStarted {
  logFormatVersion: number;
  gameBuild: string;
}

export interface ZoneChange {
  zoneName: string;
  zoneId: number;
  difficulty: number;
}

export interface MapChange {
  mapId: number;
  floorName: string;
}

export interface DungeonStart {
  name: string;
  zoneId: number;
  keyLevel: number;
  /** Modifier/affix ids (empty on a treeless run). */
  modifiers: number[];
}

export interface DungeonEnd {
  name: string;
  zoneId: number;
  keyLevel: number;
  /** An abandoned/failed run has `success: false` with a zeroed-out `score`. */
  success: boolean;
  durationMs: number;
  score: number;
}

// --- Marker / WorldMarker ---

export interface Marker {
  unit: Guid;
  index: number;
  removed: boolean;
}

export interface WorldMarker {
  x: number;
  y: number;
  slot: number;
  removed: boolean;
}

// --- Event / EventBody ---

export interface LogInstant {
  utcMs: number;
  seq: number;
}

/**
 * The typed body of a decoded line. Covers the full v8 event catalog; a token outside it becomes
 * `Unknown` (surfaced, never dropped).
 */
export type EventBody =
  | ({ type: "DamageHeal" } & DamageHeal)
  | ({ type: "DamageAbsorbed" } & DamageAbsorbed)
  | ({ type: "Cast" } & Cast)
  | ({ type: "Effect" } & Effect)
  | ({ type: "ResourceChange" } & ResourceChange)
  | ({ type: "Interrupt" } & Interrupt)
  | ({ type: "Dispel" } & Dispel)
  | ({ type: "Death" } & Death)
  | { type: "UnitDestroyed"; unit: Guid }
  | ({ type: "Resurrect" } & Resurrect)
  | ({ type: "Encounter" } & Encounter)
  | ({ type: "LoggingStarted" } & LoggingStarted)
  | ({ type: "ZoneChange" } & ZoneChange)
  | ({ type: "MapChange" } & MapChange)
  | ({ type: "DungeonStart" } & DungeonStart)
  | ({ type: "DungeonEnd" } & DungeonEnd)
  | ({ type: "Marker" } & Marker)
  | ({ type: "WorldMarker" } & WorldMarker)
  | ({ type: "CombatantInfo" } & CombatantInfo)
  /** The vestigial `EVENT_INVALID` line — damage-shaped but safe to drop. */
  | { type: "Invalid" }
  | { type: "Unknown"; rawType: string; rawFields: string[] };

/** One decoded v8 log line: its instant plus a typed body. */
export interface Event {
  instant: LogInstant;
  body: EventBody;
}

// --- combatants::Combatant ---

/** One combatant found in a log, with the gear snapshot a caller reconstructs from. */
export interface Combatant {
  /** Stable across a session — the same character re-appears under this ulid every encounter. */
  ulid: string;
  /** The in-game character name as the log records it. */
  name: string;
  /** True for the player whose client wrote the log. */
  isRecordingPlayer: boolean;
  /** The latest snapshot seen for this combatant. */
  info: CombatantInfo;
  /** Snapshots seen for this combatant across the session. */
  snapshotCount: number;
}
