//! Round-trip and byte-exact proof for `COMBATANT_INFO` — the last, and most
//! complex, family in the catalog: 20 fields, a strict pair of arrays
//! (`stat_sheet`/`talents`), a positional `gear` array where a `None` slot
//! must survive re-parsing at the same index, and a `set_bonus_id` whose
//! `None`/`Some` distinction is carried by list length (`[]` vs `[id]`), not
//! a sentinel value.
//!
//! `tests/fixtures/combatant_info.log` and `combatant_info_v8.log` aren't
//! part of `all_events.log`, so they're loaded directly here rather than
//! through `common::all_events`.

mod common;

use common::{assert_family_round_trips, body_of, event_with, events_from};
use fellowship_combat_log::encode::encode_line;
use fellowship_combat_log::event::{CombatantInfo, EventBody, GearPiece, Guid};
use fellowship_combat_log::parse::parse_line;

#[test]
fn combatant_info_round_trips_from_the_all_events_fixture() {
    assert_family_round_trips(|body| matches!(body, EventBody::CombatantInfo(_)), 1);
}

#[test]
fn combatant_info_round_trips_from_both_dedicated_fixtures() {
    let fixtures = [
        include_str!("fixtures/combatant_info.log"),
        include_str!("fixtures/combatant_info_v8.log"),
    ];
    let events: Vec<_> = fixtures.iter().flat_map(|f| events_from(f)).collect();
    // 1 combatant in combatant_info.log + 2 in combatant_info_v8.log; a count
    // assertion here (rather than only round-tripping whatever's found) is
    // what keeps this test from passing vacuously if a fixture went empty.
    assert_eq!(
        events.len(),
        3,
        "expected 1 + 2 combatants across both fixtures"
    );
    for event in events {
        let encoded = encode_line(&event).expect("event should encode");
        let reparsed = parse_line(event.instant.seq, &encoded).expect("should re-parse");
        assert_eq!(reparsed, event);
    }
}

#[test]
fn the_short_gear_tuple_fixture_decodes_to_a_none_slot_and_the_none_slot_round_trips() {
    let events = events_from(include_str!("fixtures/combatant_info.log"));
    let EventBody::CombatantInfo(combatant_info) = &events[0].body else {
        panic!("fixture line should decode to CombatantInfo");
    };
    assert_eq!(
        combatant_info.gear,
        vec![None],
        "the 12-element tuple (one short of the required 13) should decode to a single unreadable slot"
    );
    let encoded = encode_line(&events[0]).unwrap();
    assert!(
        body_of(&encoded).contains("|[()]|"),
        "a None slot must round-trip to the empty tuple (), not be omitted, in {:?}",
        body_of(&encoded)
    );
}

#[test]
fn a_mixed_gear_array_preserves_slot_position_across_none_and_some() {
    let piece = GearPiece {
        item_id: 5204,
        item_level: 315,
        rarity: 5,
        temper: (8, 8),
        stats: vec![(1, 28), (2, 15)],
        set_bonus_id: Some(102),
        ability_grants: vec![(3, 4)],
        traits: vec![(1, 2)],
        gems: vec![],
        score: 120.0,
    };
    let event = event_with(EventBody::CombatantInfo(CombatantInfo {
        ulid: "01AAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
        player: Guid::Player(1),
        is_recording_player: true,
        hero_id: 10,
        item_level: 318.2,
        stat_sheet: vec![],
        talents: vec![],
        gem_power: vec![],
        gear: vec![None, Some(piece), None],
        trait_ranks: vec![],
        neck_traits: vec![],
    }));
    let encoded = encode_line(&event).unwrap();
    let reparsed = parse_line(1, &encoded).unwrap();
    assert_eq!(reparsed, event);
    let EventBody::CombatantInfo(reparsed_info) = reparsed.body else {
        unreachable!()
    };
    assert!(reparsed_info.gear[0].is_none());
    assert!(reparsed_info.gear[1].is_some());
    assert!(reparsed_info.gear[2].is_none());
}

#[test]
fn gear_wire_form_pins_the_thirteen_element_shape_and_the_set_bonus_id_distinction() {
    let with_set = GearPiece {
        item_id: 5204,
        item_level: 315,
        rarity: 5,
        temper: (8, 8),
        // stats is (u32, i64) — a negative value (an amplification-style
        // penalty roll) must survive, not just the positive rolls every
        // fixture happens to carry.
        stats: vec![(1, 28), (2, -15)],
        set_bonus_id: Some(102),
        ability_grants: vec![(3, 4)],
        traits: vec![(1, 2)],
        gems: vec![],
        score: 120.0,
    };
    let without_set = GearPiece {
        item_id: 1,
        item_level: 1,
        rarity: 1,
        temper: (0, 0),
        stats: vec![],
        set_bonus_id: None,
        ability_grants: vec![],
        traits: vec![],
        gems: vec![],
        score: 0.0,
    };
    let event = event_with(EventBody::CombatantInfo(CombatantInfo {
        ulid: "X".to_string(),
        player: Guid::Player(1),
        is_recording_player: false,
        hero_id: 0,
        item_level: 0.0,
        stat_sheet: vec![],
        talents: vec![],
        gem_power: vec![],
        gear: vec![Some(with_set), Some(without_set)],
        trait_ranks: vec![],
        neck_traits: vec![],
    }));
    let encoded = encode_line(&event).unwrap();
    let body = body_of(&encoded).to_string();
    assert!(
        body.contains("(5204,315,5,8,8,0,0,[(1,28),(2,-15)],[102],[(3,4)],[(1,2)],[],120)"),
        "expected the 13-element tuple shape with a real set_bonus_id and a negative stat in {body:?}"
    );
    assert!(
        body.contains("(1,1,1,0,0,0,0,[],[],[],[],[],0)"),
        "expected set_bonus_id: None to render as [], not omitted or a bare value, in {body:?}"
    );
    let reparsed = parse_line(1, &encoded).unwrap();
    assert_eq!(reparsed, event);
}

#[test]
fn combatant_info_wire_form_is_exact_with_empty_collections() {
    let event = event_with(EventBody::CombatantInfo(CombatantInfo {
        ulid: "01AAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
        player: Guid::Player(1000000001),
        is_recording_player: true,
        hero_id: 10,
        item_level: 318.2,
        stat_sheet: vec![],
        talents: vec![],
        gem_power: vec![],
        gear: vec![],
        trait_ranks: vec![],
        neck_traits: vec![],
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "COMBATANT_INFO|01AAAAAAAAAAAAAAAAAAAAAAAA|Player-1000000001|\"-\"|1|10|318.2|[]|[]|[]|[]|0|[]|[]|[]|[]|0|[]|0.0"
    );
}

#[test]
fn neck_traits_wire_form_always_emits_a_literal_one_as_the_middle_element() {
    use fellowship_combat_log::event::NeckTraitChoice;
    let event = event_with(EventBody::CombatantInfo(CombatantInfo {
        ulid: "X".to_string(),
        player: Guid::Player(1),
        is_recording_player: false,
        hero_id: 0,
        item_level: 0.0,
        stat_sheet: vec![],
        talents: vec![],
        gem_power: vec![],
        gear: vec![],
        trait_ranks: vec![],
        neck_traits: vec![
            NeckTraitChoice {
                trait_id: 60,
                selected: true,
            },
            NeckTraitChoice {
                trait_id: 61,
                selected: false,
            },
        ],
    }));
    let encoded = encode_line(&event).unwrap();
    assert!(
        body_of(&encoded).contains("[(60,1,1),(61,1,0)]"),
        "expected the middle element to always be the literal 1 in {:?}",
        body_of(&encoded)
    );
    let reparsed = parse_line(1, &encoded).unwrap();
    assert_eq!(reparsed, event);
}
