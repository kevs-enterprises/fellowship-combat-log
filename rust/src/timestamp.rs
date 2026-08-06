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

/// Why `render_instant` could not render a `utc_ms`: the year fell outside the
/// four-ASCII-digit range the wire format's fixed 29-byte layout requires.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RenderInstantError;

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

/// Render a UTC-millisecond instant back to the wire's fixed 29-byte layout,
/// always with a `+00:00` offset — the offset a `LogInstant` was originally
/// logged under is never retained, only the UTC instant (see the module doc's
/// `+01:59` glitch handling, which already treats offset as non-canonical).
/// Errors rather than producing a string `parse_instant` can't read back.
pub fn render_instant(utc_ms: i64) -> Result<String, RenderInstantError> {
    // Plain `/`/`%` truncate toward zero in Rust, which would misplace the
    // millisecond-of-day for any instant before 1970 — div_euclid/rem_euclid
    // give the correct always-non-negative decomposition.
    let days = utc_ms.div_euclid(86_400_000);
    let ms_of_day = utc_ms.rem_euclid(86_400_000);
    let hour = ms_of_day / 3_600_000;
    let minute = (ms_of_day / 60_000) % 60;
    let second = (ms_of_day / 1_000) % 60;
    let millis = ms_of_day % 1_000;
    let (year, month, day) = civil_from_days(days);
    if !(0..=9999).contains(&year) {
        // digits() requires exactly 4 ASCII digits for the year — a negative or
        // 5+-digit year can't fit the fixed 29-byte layout.
        return Err(RenderInstantError);
    }
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}+00:00"
    ))
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

/// Civil date for a day count since the Unix epoch — the exact inverse of
/// `days_from_civil`, via Howard Hinnant's companion algorithm. Pure integer
/// math, wasm-safe, no time crate. Total: every `i64` `days` maps to some date,
/// there is no rejection path (unlike the forward direction, which rejects an
/// impossible calendar date).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let march_based_days = days + 719_468;
    let era = if march_based_days >= 0 {
        march_based_days
    } else {
        march_based_days - 146_096
    } / 146_097;
    let day_of_era = march_based_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let y = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let march_based_month = (5 * day_of_year + 2) / 153;
    let d = day_of_year - (153 * march_based_month + 2) / 5 + 1;
    let m = march_based_month + if march_based_month < 10 { 3 } else { -9 };
    let y = y + i64::from(m <= 2);
    (y, m, d)
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
