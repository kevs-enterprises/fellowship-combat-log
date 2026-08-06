//! Spec #13's acceptance-criteria proof: every event `parse_line` can decode,
//! `encode_line` can now encode, and the two round-trip — across the whole
//! 37-line catalog fixture in one place, rather than split family-by-family
//! as the other `encode_*.rs` files do. `tests/event_coverage.rs` already
//! owns asserting the fixture holds exactly 37 lines (one per v8 event
//! type), so that count isn't re-asserted here.
//!
//! `EVENT_INVALID` is the fixture's one deliberate exception: it decodes to
//! `EventBody::Invalid`, which has no reconstructable payload (DR-0002), so
//! it must error rather than round-trip.

mod common;

use common::{all_events, assert_family_round_trips};
use fellowship_combat_log::encode::{EncodeError, encode_line};
use fellowship_combat_log::event::EventBody;

#[test]
fn every_non_invalid_catalog_line_round_trips() {
    assert_family_round_trips(|body| !matches!(body, EventBody::Invalid), 36);
}

#[test]
fn the_fixtures_event_invalid_line_is_unrepresentable_not_a_round_trip() {
    let event = all_events()
        .into_iter()
        .find(|e| e.body == EventBody::Invalid)
        .expect("all_events.log should contain exactly one EVENT_INVALID line");
    assert_eq!(
        encode_line(&event),
        Err(EncodeError::Unrepresentable {
            event_type: "EVENT_INVALID".to_string()
        })
    );
}
