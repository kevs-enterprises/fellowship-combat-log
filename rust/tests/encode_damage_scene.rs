//! Round-trip and byte-exact proof for the damage/heal/absorb family (6
//! damage/heal tokens + `DAMAGE_ABSORBED`) and the session/dungeon family
//! (`LOGGING_STARTED`, `ZONE_CHANGE`, `MAP_CHANGE`, `DUNGEON_START`,
//! `DUNGEON_END`) — 12 tokens, merged into one file because each half was too
//! small alone.
//!
//! The hand-built negative-HP test is load-bearing: none of the fixtures
//! exercise `normalize_hp`'s u32-wrap rule (every damage/heal line in
//! `all_events.log` has positive HP), so without it the wrap would ship
//! untested.

mod common;

use common::{assert_family_round_trips, body_of, event_with};
use fellowship_combat_log::encode::{EncodeError, encode_line};
use fellowship_combat_log::event::{
    DamageAbsorbed, DamageHeal, DamageHealKind, DungeonEnd, DungeonStart, EventBody, Guid,
    LoggingStarted, MapChange, ResultTier, School, ZoneChange,
};
use fellowship_combat_log::parse::parse_line;
use fellowship_combat_log::timestamp::render_instant;

fn is_damage_scene_family(body: &EventBody) -> bool {
    matches!(
        body,
        EventBody::DamageHeal(_)
            | EventBody::DamageAbsorbed(_)
            | EventBody::LoggingStarted(_)
            | EventBody::ZoneChange(_)
            | EventBody::MapChange(_)
            | EventBody::DungeonStart(_)
            | EventBody::DungeonEnd(_)
    )
}

#[test]
fn all_twelve_tokens_round_trip_from_the_fixture() {
    // 6 damage/heal + DamageAbsorbed + LoggingStarted + ZoneChange + MapChange
    // + DungeonStart + DungeonEnd = 12 fixture lines.
    assert_family_round_trips(is_damage_scene_family, 12);
}

#[test]
fn a_negative_current_hp_wraps_to_its_u32_bit_pattern_and_round_trips() {
    // -4642i32 as u32 == 4294962654, the exact value the corpus logs on a
    // post-death line (all_events.log's ABILITY_CHANNEL_FAIL example). Source
    // and target use different values so an off-by-one in the 7-field gap
    // between them can't hide.
    let event = event_with(EventBody::DamageHeal(DamageHeal {
        kind: DamageHealKind::AbilityDamage,
        source: Guid::Player(2000000001),
        target: Guid::Npc {
            spawn: 3049784064,
            template: 42,
        },
        ability_id: 2669,
        parent_ability_id: 1,
        applied: 112,
        absorbed: -5,
        overkill: -1,
        blocked: 0,
        raw: 118,
        school: School::Magical,
        result: ResultTier::CriticalStrike,
        source_cur_hp: -4642,
        target_cur_hp: 100,
    }));
    let encoded = encode_line(&event).unwrap();
    let body = body_of(&encoded);
    assert!(
        body.contains("|4294962654|"),
        "expected the u32-wrapped negative source HP in {body:?}"
    );
    assert!(
        body.contains("|100|"),
        "expected the positive target HP in {body:?}"
    );
    let reparsed = parse_line(1, &encoded).unwrap();
    assert_eq!(reparsed, event);
}

#[test]
fn damage_heal_wire_form_is_exact() {
    let event = event_with(EventBody::DamageHeal(DamageHeal {
        kind: DamageHealKind::SwingDamage,
        source: Guid::Player(1),
        target: Guid::Player(2),
        ability_id: 10,
        parent_ability_id: 0,
        applied: 100,
        absorbed: 0,
        overkill: -1,
        blocked: 0,
        raw: 100,
        school: School::Physical,
        result: ResultTier::Hit,
        source_cur_hp: 500,
        target_cur_hp: 400,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "SWING_DAMAGE|Player-1|\"-\"|Player-2|\"-\"|10|\"-\"|0|100|0|-1|0|100|Physical|Hit|500|0|0|0.0|0.0|0.0|[]|400|0|0|0.0|0.0|0.0|[]"
    );
}

#[test]
fn damage_absorbed_wire_form_is_exact() {
    let event = event_with(EventBody::DamageAbsorbed(DamageAbsorbed {
        shield_caster: Guid::Player(2000000001),
        shielded: Guid::Player(2000000001),
        shield_effect_id: 1968,
        absorbed: 1784,
        attacker: Guid::Npc {
            spawn: 2030044464,
            template: 136,
        },
        attacking_ability_id: 638,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "DAMAGE_ABSORBED|Player-2000000001|\"-\"|Player-2000000001|\"-\"|1968|\"-\"|1784|Npc-2030044464-136|\"-\"|638|\"-\"|0"
    );
}

#[test]
fn logging_started_game_build_is_unquoted() {
    let event = event_with(EventBody::LoggingStarted(LoggingStarted {
        log_format_version: 8,
        game_build: "0.4.2.0 cl:112206".to_string(),
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "LOGGING_STARTED|8|0.4.2.0 cl:112206|0"
    );
}

#[test]
fn zone_change_has_a_trailing_empty_field() {
    let event = event_with(EventBody::ZoneChange(ZoneChange {
        zone_name: "Everdawn Grove".to_string(),
        zone_id: 11,
        difficulty: 24,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "ZONE_CHANGE|\"Everdawn Grove\"|11|24|"
    );
}

#[test]
fn map_change_wire_form_is_exact() {
    let event = event_with(EventBody::MapChange(MapChange {
        map_id: 5,
        floor_name: "Global".to_string(),
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "MAP_CHANGE|5|\"Global\"|0.0|0.0|0.0|0.0"
    );
}

#[test]
fn dungeon_start_has_a_strict_modifiers_array_and_a_trailing_empty_field() {
    let with_modifiers = event_with(EventBody::DungeonStart(DungeonStart {
        name: "Everdawn Grove".to_string(),
        zone_id: 11,
        key_level: 24,
        modifiers: vec![4, 6, 8, 19],
    }));
    assert_eq!(
        body_of(&encode_line(&with_modifiers).unwrap()),
        "DUNGEON_START|\"Everdawn Grove\"|11|24|[4,6,8,19]|0|1970-01-01T00:00:00.000+00:00|"
    );

    let no_modifiers = event_with(EventBody::DungeonStart(DungeonStart {
        name: "Everdawn Grove".to_string(),
        zone_id: 11,
        key_level: 1,
        modifiers: vec![],
    }));
    let encoded = encode_line(&no_modifiers).unwrap();
    assert!(
        body_of(&encoded).contains("|[]|"),
        "empty modifiers must render exactly [] (strict array on decode)"
    );
    let reparsed = parse_line(1, &encoded).unwrap();
    assert_eq!(reparsed, no_modifiers);
}

#[test]
fn dungeon_starts_placeholder_nested_timestamp_matches_render_instant_at_the_epoch() {
    // f8's placeholder is a hardcoded literal in encode.rs (nothing can decode
    // it back — parse.rs's dungeon_start reads only f3-f6 — so no round-trip
    // test can ever catch it drifting from what render_instant(0) actually
    // produces). Assert the match directly instead.
    let expected = render_instant(0).expect("the epoch is always representable");
    let event = event_with(EventBody::DungeonStart(DungeonStart {
        name: "X".to_string(),
        zone_id: 0,
        key_level: 0,
        modifiers: vec![],
    }));
    let body = body_of(&encode_line(&event).unwrap()).to_string();
    // body starts at f2 (the token): 0=f2 token, 1=f3 name, 2=f4 zone_id,
    // 3=f5 key_level, 4=f6 modifiers, 5=f7 unmodeled int, 6=f8 the nested
    // timestamp under test.
    let nested_timestamp = body
        .split('|')
        .nth(6)
        .expect("DUNGEON_START body has an f8 position");
    assert_eq!(nested_timestamp, expected);
}

#[test]
fn dungeon_end_emits_an_empty_modifier_array_distinct_from_dungeon_starts_real_one() {
    let event = event_with(EventBody::DungeonEnd(DungeonEnd {
        name: "Everdawn Grove".to_string(),
        zone_id: 11,
        key_level: 24,
        success: true,
        duration_ms: 225464,
        score: 500.0,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "DUNGEON_END|\"Everdawn Grove\"|11|24|[]|1|225464|500|0|0|0"
    );

    let wipe = event_with(EventBody::DungeonEnd(DungeonEnd {
        name: "Everdawn Grove".to_string(),
        zone_id: 11,
        key_level: 24,
        success: false,
        duration_ms: 60000,
        score: 0.0,
    }));
    assert_eq!(
        body_of(&encode_line(&wipe).unwrap()),
        "DUNGEON_END|\"Everdawn Grove\"|11|24|[]|0|60000|0|0|0|0"
    );
}

#[test]
fn a_pipe_in_logging_starteds_game_build_is_rejected() {
    let event = event_with(EventBody::LoggingStarted(LoggingStarted {
        log_format_version: 8,
        game_build: "0.4.2.0|bad".to_string(),
    }));
    assert_eq!(
        encode_line(&event),
        Err(EncodeError::InvalidText {
            field: "game_build",
            reason: "contains '|', which would corrupt the line's field framing",
        })
    );
}
