//! Round-trip and byte-exact proof for the simple event families:
//! `UNIT_DESTROYED`, `MARKER_PLACED`/`REMOVED`, `UNIT_DEATH`/`ALLY_DEATH`,
//! `RESURRECT`, `ABILITY_INTERRUPT`, `ENCOUNTER_START`/`END` — 9 tokens.
//!
//! `Encounter` is the first family with a string field (`bosses`), so this
//! file is also where the shared `render_name_array` validation
//! (`|`/embedded `"`) gets its first real exercise.

mod common;

use common::{assert_family_round_trips, body_of, event_with};
use fellowship_combat_log::encode::{EncodeError, encode_line};
use fellowship_combat_log::event::{
    Death, DeathKind, Encounter, EncounterPhase, EventBody, Guid, Interrupt, Marker, Resurrect,
};
use fellowship_combat_log::parse::parse_line;

fn is_simple_family(body: &EventBody) -> bool {
    matches!(
        body,
        EventBody::UnitDestroyed { .. }
            | EventBody::Marker(_)
            | EventBody::Death(_)
            | EventBody::Resurrect(_)
            | EventBody::Interrupt(_)
            | EventBody::Encounter(_)
    )
}

#[test]
fn all_nine_simple_tokens_round_trip_from_the_fixture() {
    // UnitDestroyed, Marker x2 (placed/removed), Death x2 (unit/ally),
    // Resurrect, Interrupt, Encounter x2 (start/end) = 9 fixture lines.
    assert_family_round_trips(is_simple_family, 9);
}

#[test]
fn unit_destroyed_wire_form_is_exact() {
    let event = event_with(EventBody::UnitDestroyed {
        unit: Guid::Npc {
            spawn: 1991247744,
            template: 279,
        },
    });
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "UNIT_DESTROYED|Npc-1991247744-279|\"-\"|0.0"
    );
}

#[test]
fn marker_placed_and_removed_wire_forms_are_exact() {
    let placed = event_with(EventBody::Marker(Marker {
        unit: Guid::Player(2000000001),
        index: 1,
        removed: false,
    }));
    assert_eq!(
        body_of(&encode_line(&placed).unwrap()),
        "MARKER_PLACED|Player-2000000001|\"-\"|1"
    );

    let removed = event_with(EventBody::Marker(Marker {
        unit: Guid::Player(1000000004),
        index: 8,
        removed: true,
    }));
    assert_eq!(
        body_of(&encode_line(&removed).unwrap()),
        "MARKER_REMOVED|Player-1000000004|\"-\"|8"
    );
}

#[test]
fn death_wire_forms_are_exact_for_both_kinds() {
    let unit_death = event_with(EventBody::Death(Death {
        kind: DeathKind::Unit,
        dead: Guid::Npc {
            spawn: 2214068688,
            template: 136,
        },
        killer: Guid::Player(1000000001),
        killing_ability_id: 101,
    }));
    assert_eq!(
        body_of(&encode_line(&unit_death).unwrap()),
        "UNIT_DEATH|Npc-2214068688-136|\"-\"|Player-1000000001|\"-\"|101|\"-\"|0|0.0"
    );

    let ally_death = event_with(EventBody::Death(Death {
        kind: DeathKind::Ally,
        dead: Guid::Player(4000000001),
        killer: Guid::Npc {
            spawn: 3206022608,
            template: 41,
        },
        killing_ability_id: 1519,
    }));
    assert_eq!(
        body_of(&encode_line(&ally_death).unwrap()),
        "ALLY_DEATH|Player-4000000001|\"-\"|Npc-3206022608-41|\"-\"|1519|\"-\"|0|0.0"
    );
}

#[test]
fn resurrect_wire_form_has_a_trailing_empty_field() {
    let event = event_with(EventBody::Resurrect(Resurrect {
        resurrecter: Guid::Player(500000005),
        target: Guid::Player(3000000003),
        ability_id: 22,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "RESURRECT|Player-500000005|\"-\"|Player-3000000003|\"-\"|22|\"-\"|"
    );
}

#[test]
fn interrupt_wire_form_is_exact() {
    let event = event_with(EventBody::Interrupt(Interrupt {
        interrupter: Guid::Player(2000000001),
        victim: Guid::Npc {
            spawn: 3320317520,
            template: 40,
        },
        interrupting_ability_id: 1844,
        interrupted_ability_id: 1545,
    }));
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "ABILITY_INTERRUPT|Player-2000000001|\"-\"|Npc-3320317520-40|\"-\"|1844|\"-\"|1545|\"-\""
    );
}

#[test]
fn encounter_start_and_end_wire_forms_are_exact_with_empty_bosses() {
    let start = event_with(EventBody::Encounter(Encounter {
        phase: EncounterPhase::Start,
        encounter_id: 30,
        bosses: vec![],
    }));
    assert_eq!(
        body_of(&encode_line(&start).unwrap()),
        "ENCOUNTER_START|30|[]"
    );

    let end = event_with(EventBody::Encounter(Encounter {
        phase: EncounterPhase::End { success: true },
        encounter_id: 30,
        bosses: vec!["Malgut the Fetid".to_string()],
    }));
    assert_eq!(
        body_of(&encode_line(&end).unwrap()),
        "ENCOUNTER_END|30|[\"Malgut the Fetid\"]|1"
    );

    let wipe = event_with(EventBody::Encounter(Encounter {
        phase: EncounterPhase::End { success: false },
        encounter_id: 30,
        bosses: vec![],
    }));
    assert_eq!(
        body_of(&encode_line(&wipe).unwrap()),
        "ENCOUNTER_END|30|[]|0"
    );
}

#[test]
fn a_boss_name_with_an_internal_comma_round_trips() {
    let event = event_with(EventBody::Encounter(Encounter {
        phase: EncounterPhase::Start,
        encounter_id: 30,
        bosses: vec![
            "Malgut the Fetid".to_string(),
            "Xul, The Blood Monolith".to_string(),
        ],
    }));
    let encoded = encode_line(&event).unwrap();
    assert_eq!(
        body_of(&encoded),
        "ENCOUNTER_START|30|[\"Malgut the Fetid\",\"Xul, The Blood Monolith\"]"
    );
    let reparsed = parse_line(1, &encoded).unwrap();
    assert_eq!(reparsed, event);
}

#[test]
fn a_pipe_in_a_boss_name_is_rejected_rather_than_corrupting_the_line() {
    let event = event_with(EventBody::Encounter(Encounter {
        phase: EncounterPhase::Start,
        encounter_id: 30,
        bosses: vec!["Bad|Boss".to_string()],
    }));
    assert_eq!(
        encode_line(&event),
        Err(EncodeError::InvalidText {
            field: "bosses",
            reason: "contains '|', which would corrupt the line's field framing",
        })
    );
}

#[test]
fn an_embedded_quote_in_a_boss_name_is_rejected() {
    let event = event_with(EventBody::Encounter(Encounter {
        phase: EncounterPhase::Start,
        encounter_id: 30,
        bosses: vec!["Bad\"Boss".to_string()],
    }));
    assert_eq!(
        encode_line(&event),
        Err(EncodeError::InvalidText {
            field: "bosses",
            reason: "contains '\"', which would desynchronize quote-tracking inside a bracketed list",
        })
    );
}
