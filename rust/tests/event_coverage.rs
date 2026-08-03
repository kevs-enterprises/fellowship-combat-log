//! Full v8 event-catalog coverage: a miniature multi-family log with one line per
//! event type parses end-to-end into the ordered event stream — every line to its
//! typed body (never `Unknown`), and an out-of-catalog token surfaces as
//! `Unknown`. Every fixture here is a committed synthetic snippet.

use fellowship_combat_log::event::{DeathKind, Event, EventBody, Guid};
use fellowship_combat_log::parse::parse_line;

fn events(fixture: &str) -> Vec<Event> {
    fixture
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            parse_line(i as u32 + 1, line)
                .unwrap_or_else(|error| panic!("catalog line {} failed: {error:?}", i + 1))
        })
        .collect()
}

#[test]
fn every_catalog_line_parses_to_a_typed_body() {
    let events = events(include_str!("fixtures/all_events.log"));
    assert_eq!(events.len(), 37, "expected one line per v8 event type");
    for event in &events {
        assert!(
            !matches!(event.body, EventBody::Unknown { .. }),
            "a known catalog line decoded as Unknown: {:?}",
            event.body
        );
    }
}

#[test]
fn the_multi_family_stream_is_ordered() {
    let events = events(include_str!("fixtures/all_events.log"));
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].instant <= pair[1].instant)
    );
}

#[test]
fn an_out_of_catalog_token_surfaces_as_unknown() {
    let body = parse_line(1, "2026-07-22T10:28:38.000+02:00|SOME_FUTURE_EVENT|a|b")
        .expect("parses (timestamp valid)")
        .body;
    assert_eq!(
        body,
        EventBody::Unknown {
            raw_type: "SOME_FUTURE_EVENT".to_string()
        }
    );
}

#[test]
fn the_new_families_decode_their_key_fields() {
    let bodies: Vec<EventBody> = events(include_str!("fixtures/all_events.log"))
        .into_iter()
        .map(|event| event.body)
        .collect();
    let find =
        |predicate: fn(&EventBody) -> bool| bodies.iter().find(|body| predicate(body)).unwrap();

    let EventBody::DamageAbsorbed(absorbed) = find(|b| matches!(b, EventBody::DamageAbsorbed(_)))
    else {
        unreachable!()
    };
    assert_eq!(absorbed.absorbed, 1784);
    assert_eq!(
        absorbed.attacker,
        Guid::Npc {
            spawn: 2_030_044_464,
            template: 136
        }
    );

    let EventBody::Death(unit_death) =
        find(|b| matches!(b, EventBody::Death(d) if d.kind == DeathKind::Unit))
    else {
        unreachable!()
    };
    assert_eq!(unit_death.killing_ability_id, 101);

    let EventBody::Death(ally_death) =
        find(|b| matches!(b, EventBody::Death(d) if d.kind == DeathKind::Ally))
    else {
        unreachable!()
    };
    assert_eq!(ally_death.dead, Guid::Player(4_000_000_001));

    let EventBody::DungeonStart(start) = find(|b| matches!(b, EventBody::DungeonStart(_))) else {
        unreachable!()
    };
    assert_eq!(start.modifiers, vec![4, 6, 8, 19]);

    let EventBody::DungeonEnd(end) = find(|b| matches!(b, EventBody::DungeonEnd(_))) else {
        unreachable!()
    };
    assert!(end.success);
    assert_eq!(end.duration_ms, 225_464);
    assert_eq!(end.key_level, 24);

    let EventBody::LoggingStarted(logging) = find(|b| matches!(b, EventBody::LoggingStarted(_)))
    else {
        unreachable!()
    };
    assert_eq!(logging.log_format_version, 8);
    assert_eq!(logging.game_build, "0.4.2.0 cl:112206");

    let EventBody::Marker(marker) = find(|b| matches!(b, EventBody::Marker(_))) else {
        unreachable!()
    };
    assert_eq!(marker.index, 1);
    assert!(!marker.removed);

    let EventBody::WorldMarker(world_marker) = find(|b| matches!(b, EventBody::WorldMarker(_)))
    else {
        unreachable!()
    };
    assert_eq!(world_marker.slot, 4);

    let EventBody::Dispel(dispel) = find(|b| matches!(b, EventBody::Dispel(_))) else {
        unreachable!()
    };
    assert_eq!(dispel.remaining_seconds, 14.894555);

    assert!(bodies.iter().any(|b| matches!(b, EventBody::Invalid)));
}
