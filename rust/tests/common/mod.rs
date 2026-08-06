//! Shared test helpers for the encode test suite. Not a test binary itself —
//! `tests/common/mod.rs` (a subdirectory, not `tests/common.rs`) is the
//! standard way to share code across integration test files without Cargo
//! compiling it as its own test target.

use fellowship_combat_log::encode::encode_line;
use fellowship_combat_log::event::{Event, EventBody};
use fellowship_combat_log::parse::parse_line;
use fellowship_combat_log::timestamp::LogInstant;

/// Every event in `tests/fixtures/all_events.log`, one per v8 catalog entry
/// (`tests/event_coverage.rs` asserts this fixture holds exactly 37 lines).
#[allow(dead_code)]
pub fn all_events() -> Vec<Event> {
    events_from(include_str!("../fixtures/all_events.log"))
}

/// Parse every non-empty line of already-loaded fixture text (pass it
/// `include_str!("fixtures/whatever.log")`) into a `Vec<Event>`, panicking
/// with the offending line number if any line fails to parse. Shared by
/// `all_events()` and any test file whose family has its own dedicated
/// fixture outside `all_events.log`.
#[allow(dead_code)]
pub fn events_from(fixture: &str) -> Vec<Event> {
    fixture
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            parse_line(i as u32 + 1, line)
                .unwrap_or_else(|_| panic!("fixture line {} should parse", i + 1))
        })
        .collect()
}

/// Everything in an encoded line after the timestamp field. Byte-exact
/// wire-form tests pin the *body*, not the timestamp — timestamp rendering has
/// its own dedicated test file (`timestamp_render_round_trip.rs`) — so tests
/// asserting a body's exact form use this rather than hand-computing which
/// calendar date a given `utc_ms` renders to.
#[allow(dead_code)]
pub fn body_of(encoded: &str) -> &str {
    encoded.split_once('|').expect("encoded line has a body").1
}

/// An `Event` at `utc_ms: 0`, for tests pinning a body's exact wire form where
/// the timestamp itself doesn't matter (use `body_of` to strip it from the
/// assertion rather than relying on `utc_ms: 0` rendering any particular way).
#[allow(dead_code)]
pub fn event_with(body: EventBody) -> Event {
    Event {
        instant: LogInstant { utc_ms: 0, seq: 1 },
        body,
    }
}

/// Filter `all_events()` by `is_member` (one family's variants), assert the
/// family holds exactly `expected_count` fixture lines — one per token, per
/// `all_events.log`'s one-line-per-catalog-entry convention — then round-trip
/// each through `encode_line`/`parse_line` and assert equality. Shared by
/// every encode test file's "all N tokens round-trip from the fixture" test,
/// which repeated this loop nearly verbatim across three files.
#[allow(dead_code)]
pub fn assert_family_round_trips(is_member: impl Fn(&EventBody) -> bool, expected_count: usize) {
    let events: Vec<_> = all_events()
        .into_iter()
        .filter(|e| is_member(&e.body))
        .collect();
    assert_eq!(
        events.len(),
        expected_count,
        "expected exactly one fixture line per token in this family"
    );
    for event in events {
        let encoded = encode_line(&event).expect("event should encode");
        let reparsed = parse_line(event.instant.seq, &encoded).expect("should re-parse");
        assert_eq!(reparsed, event);
    }
}
