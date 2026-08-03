//! Total-parsing regression: malformed / short lines and a bad timestamp return
//! an error, never a panic (the parse surface is total). Asserted by fixture.

use fellowship_combat_log::parse::parse_line;

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
