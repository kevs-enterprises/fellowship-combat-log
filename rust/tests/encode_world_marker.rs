//! Round-trip and error-path proof for `encode_line`'s tracer family
//! (`WORLD_MARKER_PLACED`/`REMOVED`), plus `Unknown`/`Invalid` rejection,
//! non-finite-float rejection, and out-of-range timestamps. The shared
//! string-validation primitives (`render_quoted`, `render_name_array`) have no
//! test here since `WorldMarker` has no string fields — see
//! `encode_simple.rs`'s `Encounter` tests for `|`/`"` in a boss name instead.

mod common;

use common::{all_events, body_of};
use fellowship_combat_log::encode::{EncodeError, encode_line};
use fellowship_combat_log::event::{Event, EventBody, WorldMarker};
use fellowship_combat_log::parse::parse_line;
use fellowship_combat_log::timestamp::LogInstant;

#[test]
fn world_marker_placed_and_removed_round_trip_from_the_fixture() {
    let world_markers: Vec<_> = all_events()
        .into_iter()
        .filter(|e| matches!(e.body, EventBody::WorldMarker(_)))
        .collect();
    assert_eq!(
        world_markers.len(),
        2,
        "expected exactly WORLD_MARKER_PLACED and WORLD_MARKER_REMOVED in the fixture"
    );
    for event in world_markers {
        let encoded = encode_line(&event).expect("world marker should encode");
        let reparsed = parse_line(event.instant.seq, &encoded).expect("should re-parse");
        assert_eq!(reparsed, event);
    }
}

#[test]
fn world_marker_wire_form_is_exact() {
    let event = Event {
        instant: LogInstant { utc_ms: 0, seq: 1 },
        body: EventBody::WorldMarker(WorldMarker {
            x: 15599.095087,
            y: -6217.202963,
            slot: 4,
            removed: false,
        }),
    };
    assert_eq!(
        body_of(&encode_line(&event).unwrap()),
        "WORLD_MARKER_PLACED|15599.095087|-6217.202963|4"
    );

    let removed = Event {
        body: EventBody::WorldMarker(WorldMarker {
            x: 15599.095087,
            y: -6217.202963,
            slot: 4,
            removed: true,
        }),
        ..event
    };
    assert_eq!(
        body_of(&encode_line(&removed).unwrap()),
        "WORLD_MARKER_REMOVED|15599.095087|-6217.202963|4"
    );
}

#[test]
fn unknown_and_invalid_are_unrepresentable() {
    let unknown = Event {
        instant: LogInstant { utc_ms: 0, seq: 1 },
        body: EventBody::Unknown {
            raw_type: "SOME_FUTURE_EVENT".to_string(),
            raw_fields: vec![],
        },
    };
    assert_eq!(
        encode_line(&unknown),
        Err(EncodeError::Unrepresentable {
            event_type: "SOME_FUTURE_EVENT".to_string()
        })
    );

    let invalid = Event {
        instant: LogInstant { utc_ms: 0, seq: 1 },
        body: EventBody::Invalid,
    };
    assert_eq!(
        encode_line(&invalid),
        Err(EncodeError::Unrepresentable {
            event_type: "EVENT_INVALID".to_string()
        })
    );
}

#[test]
fn a_non_finite_float_errors_rather_than_emitting_nan_or_inf() {
    let event = Event {
        instant: LogInstant { utc_ms: 0, seq: 1 },
        body: EventBody::WorldMarker(WorldMarker {
            x: f64::NAN,
            y: 0.0,
            slot: 1,
            removed: false,
        }),
    };
    assert_eq!(
        encode_line(&event),
        Err(EncodeError::NonFiniteFloat { field: "x" })
    );

    let infinite = Event {
        body: EventBody::WorldMarker(WorldMarker {
            x: f64::INFINITY,
            y: 0.0,
            slot: 1,
            removed: false,
        }),
        ..event
    };
    assert_eq!(
        encode_line(&infinite),
        Err(EncodeError::NonFiniteFloat { field: "x" })
    );
}

#[test]
fn a_year_outside_the_representable_range_errors_rather_than_panicking() {
    // One millisecond past 9999-12-31T23:59:59.999+00:00.
    let top = parse_line(
        1,
        "9999-12-31T23:59:59.999+00:00|WORLD_MARKER_PLACED|0.0|0.0|1",
    )
    .unwrap()
    .instant
    .utc_ms
        + 1;
    let event = Event {
        instant: LogInstant {
            utc_ms: top,
            seq: 1,
        },
        body: EventBody::WorldMarker(WorldMarker {
            x: 0.0,
            y: 0.0,
            slot: 1,
            removed: false,
        }),
    };
    assert_eq!(encode_line(&event), Err(EncodeError::TimestampOutOfRange));
}
