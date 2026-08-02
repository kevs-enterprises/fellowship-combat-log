//! Wasm-safe timestamp parsing: a hand-rolled ISO-8601 + UTC-offset
//! decoder — no chrono, no `SystemTime` — producing a UTC-millisecond instant.
//! The sporadic `+01:59` formatter glitch (the same UTC instant rendered with a
//! one-minute-earlier wall clock) is handled for free by trusting the offset:
//! converting to UTC yields the same instant, so the stream stays non-decreasing.
//! Ordering is `(utc_ms, seq)` — the file line number breaks sub-millisecond ties,
//! never a raw-timestamp-string compare (which the glitch would misorder).

/// A parsed log timestamp: a UTC millisecond plus the file line sequence. `Ord`
/// compares `(utc_ms, seq)`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct LogInstant {
    pub utc_ms: i64,
    pub seq: u32,
}

/// Parse an ISO-8601 timestamp with milliseconds and a UTC offset
/// (`2026-07-22T08:45:49.764+02:00`) into a UTC-millisecond instant. Returns
/// `None` on any malformed component — the caller keeps parsing total.
pub fn parse_instant(seq: u32, s: &str) -> Option<LogInstant> {
    // Fixed layout: YYYY-MM-DDTHH:MM:SS.mmm±HH:MM (29 bytes, all ASCII). The
    // `is_ascii` guard is load-bearing: it makes every byte a char boundary, so
    // the positional string slices below can't panic on a multibyte char.
    let bytes = s.as_bytes();
    if bytes.len() != 29
        || !s.is_ascii()
        || bytes[10] != b'T'
        || bytes[19] != b'.'
        || bytes[26] != b':'
    {
        return None;
    }
    let year: i64 = digits(&s[0..4])?;
    let month: i64 = digits(&s[5..7])?;
    let day: i64 = digits(&s[8..10])?;
    let hour: i64 = digits(&s[11..13])?;
    let minute: i64 = digits(&s[14..16])?;
    let second: i64 = digits(&s[17..19])?;
    let millis: i64 = digits(&s[20..23])?;
    let sign: i64 = match bytes[23] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let offset_hours: i64 = digits(&s[24..26])?;
    let offset_minutes: i64 = digits(&s[27..29])?;

    let days = days_from_civil(year, month, day)?;
    let local_seconds = days * 86_400 + hour * 3600 + minute * 60 + second;
    let offset_seconds = sign * (offset_hours * 3600 + offset_minutes * 60);
    Some(LogInstant {
        utc_ms: (local_seconds - offset_seconds) * 1000 + millis,
        seq,
    })
}

/// Parse a fixed run of ASCII digits; `None` on any non-digit (rejects the `+`/`-`
/// and stray separators a positional slice could otherwise capture).
fn digits(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Days since the Unix epoch for a proleptic-Gregorian date, via Howard Hinnant's
/// civil algorithm — pure integer math, wasm-safe, no time crate. Rejects an
/// out-of-range day so an impossible date (e.g. Feb 31) is `None`, not a silent
/// roll-over.
fn days_from_civil(y: i64, m: i64, d: i64) -> Option<i64> {
    if !(1..=12).contains(&m) || d < 1 || d > days_in_month(y, m) {
        return None;
    }
    let y = y - i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let month_shift = if m > 2 { -3 } else { 9 };
    let day_of_year = (153 * (m + month_shift) + 2) / 5 + d - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}
