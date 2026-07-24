//! Timezone-glitch regression: the sporadic `+01:59` formatter glitch renders the
//! same UTC instant with a one-minute-earlier wall clock. Trusting the offset
//! yields the identical instant, so ordering by `(utc_ms, seq)` keeps the stream
//! non-decreasing. Asserted at the public parse surface.

use fellowship_combat_log::parse::parse_line;

#[test]
fn plus_one_fifty_nine_glitch_yields_the_same_instant() {
    let instants: Vec<_> = include_str!("fixtures/timezone_glitch.log")
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            parse_line(i as u32 + 1, line)
                .expect("glitch line parses")
                .instant
        })
        .collect();

    // Same UTC instant despite the +02:00 vs +01:59 / one-minute wall-clock skew.
    assert_eq!(instants[0].utc_ms, instants[1].utc_ms);
    // The line-sequence tiebreaker keeps them ordered; the stream is non-decreasing.
    assert!(instants[0] < instants[1]);
    assert!(instants.windows(2).all(|pair| pair[0] <= pair[1]));
}
