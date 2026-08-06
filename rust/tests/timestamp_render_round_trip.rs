//! Round-trip proof for `render_instant`, the wire-text inverse of
//! `parse_instant`: rendering a `utc_ms` and re-parsing it must reproduce the
//! same instant, for a representative range including pre-1970 (negative)
//! instants, and for the real instants in the timezone-glitch fixture.
//!
//! Boundary `utc_ms` values are derived from `parse_instant` on known date
//! strings rather than hand-computed, so a slipped digit here can't silently
//! mistest the boundary it's meant to pin.

use fellowship_combat_log::parse::parse_line;
use fellowship_combat_log::timestamp::{RenderInstantError, parse_instant, render_instant};

fn assert_round_trips(utc_ms: i64) {
    let rendered = render_instant(utc_ms).unwrap_or_else(|_| panic!("{utc_ms} should render"));

    // The fixed 29-byte layout parse_instant itself validates — checked for
    // every case, not just one, since a layout slip could easily be specific to
    // a particular year/millisecond combination (e.g. single-digit padding).
    assert_eq!(
        rendered.len(),
        29,
        "{rendered:?} (from {utc_ms}) should be 29 bytes"
    );
    let bytes = rendered.as_bytes();
    assert_eq!(bytes[10], b'T', "{rendered:?} (from {utc_ms})");
    assert_eq!(bytes[19], b'.', "{rendered:?} (from {utc_ms})");
    assert_eq!(bytes[23], b'+', "{rendered:?} (from {utc_ms})");
    assert_eq!(bytes[26], b':', "{rendered:?} (from {utc_ms})");
    assert!(
        rendered.ends_with("+00:00"),
        "{rendered:?} (from {utc_ms}) should end in +00:00"
    );

    let reparsed = parse_instant(1, &rendered)
        .unwrap_or_else(|| panic!("{rendered:?} (from {utc_ms}) should re-parse"));
    assert_eq!(
        reparsed.utc_ms, utc_ms,
        "utc_ms {utc_ms} rendered to {rendered:?} but re-parsed to {}",
        reparsed.utc_ms
    );
}

fn utc_ms_of(iso: &str) -> i64 {
    parse_instant(1, iso)
        .unwrap_or_else(|| panic!("{iso:?} should parse"))
        .utc_ms
}

#[test]
fn representative_instants_round_trip() {
    for utc_ms in [
        0,                                          // the epoch itself
        1,                                          // first ms after the epoch
        -1,                                         // one ms before the epoch
        utc_ms_of("2026-07-22T08:45:49.764+00:00"), // a realistic recent instant
        utc_ms_of("1969-12-31T00:00:00.000+00:00"), // a full day before the epoch
        utc_ms_of("1900-01-01T00:00:00.000+00:00"), // well before the epoch
        utc_ms_of("0000-01-01T00:00:00.000+00:00"), // the bottom of the representable range
        utc_ms_of("9999-12-31T23:59:59.999+00:00"), // the top of the representable range
    ] {
        assert_round_trips(utc_ms);
    }
}

#[test]
fn timezone_glitch_fixture_instants_round_trip() {
    for (i, line) in include_str!("fixtures/timezone_glitch.log")
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        let instant = parse_line(i as u32 + 1, line)
            .unwrap_or_else(|_| panic!("fixture line {} should parse", i + 1))
            .instant;
        assert_round_trips(instant.utc_ms);
    }
}

#[test]
fn source_offset_is_not_retained_only_the_utc_instant() {
    // assert_round_trips already checks the byte layout for every case; this
    // test is specifically about +02:00 not surviving into the render.
    let rendered =
        render_instant(utc_ms_of("2026-07-22T08:45:49.764+02:00")).expect("should render");
    assert!(rendered.starts_with("2026-07-22T06:45:49.764"));
    assert!(rendered.ends_with("+00:00"));
}

#[test]
fn a_year_outside_the_four_digit_range_errors_rather_than_panics() {
    // One millisecond past the top of the representable range crosses into
    // year 10000, which can't fit the wire format's fixed 4-digit year slot.
    let one_past_top = utc_ms_of("9999-12-31T23:59:59.999+00:00") + 1;
    assert_eq!(render_instant(one_past_top), Err(RenderInstantError));

    // One millisecond before the bottom of the representable range crosses into
    // a year before 0000, which the fixed-width layout can't represent either.
    let one_before_bottom = utc_ms_of("0000-01-01T00:00:00.000+00:00") - 1;
    assert_eq!(render_instant(one_before_bottom), Err(RenderInstantError));
}
