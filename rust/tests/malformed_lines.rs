//! Total-parsing regression: malformed / short lines and a bad timestamp return
//! an error, never a panic (the parse surface is total). Asserted by fixture.

use fellowship_combat_log::parse::{ParseError, parse_line};

#[test]
fn malformed_lines_error_without_panic() {
    for (i, line) in include_str!("fixtures/malformed.log")
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        assert!(
            parse_line(i as u32 + 1, line).is_err(),
            "line {} should error, not parse: {line}",
            i + 1
        );
    }
}

#[test]
fn empty_input_is_an_error_not_a_panic() {
    assert!(parse_line(1, "").is_err());
    assert!(parse_line(1, "\n").is_err());
}

#[test]
fn adversarial_timestamps_error_not_panic() {
    // A 29-byte timestamp whose bytes are non-ASCII: a naive positional slice
    // would panic on the multibyte char (`é` straddles byte index 4). Parse must
    // return Err instead.
    assert!(
        parse_line(
            1,
            "a\u{e9}\u{e9}xxxxxT12345678.123456:12|ENCOUNTER_START|30|[\"B\"]"
        )
        .is_err()
    );
    // An impossible calendar date is rejected, not silently rolled over.
    assert!(
        parse_line(
            1,
            "2026-02-31T08:45:49.764+02:00|ENCOUNTER_START|30|[\"B\"]"
        )
        .is_err()
    );
}

#[test]
fn a_bad_polarity_token_attributes_the_calling_familys_own_field_number() {
    // ABILITY_DISPEL reads polarity at f12 — a bad token there must attribute
    // field 12, not the field 11 the pre-fix code always hardcoded regardless of
    // which family called it.
    let dispel = "2026-07-22T10:00:00.000+00:00|ABILITY_DISPEL|Player-1000000001|\"P1\"|Npc-1000000000-1|\"Test\"|100|\"Ability\"|200|\"Effect\"|5.0|INVALID";
    assert_eq!(
        parse_line(1, dispel),
        Err(ParseError::BadField {
            field: 12,
            reason: "polarity"
        })
    );

    // EFFECT_APPLIED reads polarity at f11 — this was already correct before the
    // fix, and must stay correct now that the field number travels per call site.
    let effect = "2026-07-22T10:00:00.000+00:00|EFFECT_APPLIED|Player-1000000001|\"P1\"|Npc-1000000000-1|\"Test\"|100|\"Effect\"|5.0|1|INVALID";
    assert_eq!(
        parse_line(1, effect),
        Err(ParseError::BadField {
            field: 11,
            reason: "polarity"
        })
    );
}
