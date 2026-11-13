//! Timestamp and duration helpers. All internal times are microseconds since the Unix epoch, UTC.

use anyhow::{bail, Context, Result};

pub const US_PER_SEC: i64 = 1_000_000;
const US_PER_DAY: i64 = 86_400 * US_PER_SEC;

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverse of [`days_from_civil`].
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn digits(b: &[u8], from: usize, n: usize, s: &str) -> Result<i64> {
    let part = b
        .get(from..from + n)
        .with_context(|| format!("timestamp too short: {s:?}"))?;
    let mut v = 0i64;
    for &c in part {
        if !c.is_ascii_digit() {
            bail!("bad digit in timestamp {s:?}");
        }
        v = v * 10 + (c - b'0') as i64;
    }
    Ok(v)
}

fn expect(b: &[u8], at: usize, chars: &[u8], s: &str) -> Result<()> {
    match b.get(at) {
        Some(c) if chars.contains(c) => Ok(()),
        _ => bail!("unexpected character at position {at} in timestamp {s:?}"),
    }
}

/// Parse `YYYY-MM-DD[T ]HH:MM:SS[.fraction][Z|±HH:MM]` into microseconds since the epoch.
/// A missing offset means UTC. Fraction digits past microseconds are truncated.
pub fn parse_ts_us(s: &str) -> Result<i64> {
    let b = s.as_bytes();
    let y = digits(b, 0, 4, s)?;
    expect(b, 4, b"-", s)?;
    let mo = digits(b, 5, 2, s)? as u32;
    expect(b, 7, b"-", s)?;
    let d = digits(b, 8, 2, s)? as u32;
    expect(b, 10, b"T ", s)?;
    let h = digits(b, 11, 2, s)?;
    expect(b, 13, b":", s)?;
    let mi = digits(b, 14, 2, s)?;
    expect(b, 16, b":", s)?;
    let se = digits(b, 17, 2, s)?;
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        bail!("timestamp out of range: {s:?}");
    }
    let mut i = 19;
    let mut frac_us = 0i64;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let mut scale = 100_000i64;
        while let Some(c) = b.get(i).filter(|c| c.is_ascii_digit()) {
            frac_us += (c - b'0') as i64 * scale;
            scale /= 10;
            i += 1;
        }
    }
    let offset_s = match b.get(i) {
        None => 0,
        Some(b'Z') | Some(b'z') => {
            i += 1;
            0
        }
        Some(&sign @ (b'+' | b'-')) => {
            let oh = digits(b, i + 1, 2, s)?;
            let om = if b.get(i + 3) == Some(&b':') {
                digits(b, i + 4, 2, s)?
            } else {
                digits(b, i + 3, 2, s)?
            };
            i = b.len();
            let off = oh * 3600 + om * 60;
            if sign == b'+' {
                off
            } else {
                -off
            }
        }
        _ => bail!("unexpected trailing characters in timestamp {s:?}"),
    };
    if i != b.len() {
        bail!("unexpected trailing characters in timestamp {s:?}");
    }
    let days = days_from_civil(y, mo, d);
    Ok((days * 86_400 + h * 3600 + mi * 60 + se - offset_s) * US_PER_SEC + frac_us)
}

fn push2(out: &mut String, v: u32) {
    out.push((b'0' + (v / 10) as u8) as char);
    out.push((b'0' + (v % 10) as u8) as char);
}

/// Append `YYYY-MM-DDTHH:MM:SS[.ffffff]Z` to `out`. The fraction is written only when non-zero.
pub fn format_ts_us(us: i64, out: &mut String) {
    let days = us.div_euclid(US_PER_DAY);
    let rem = us.rem_euclid(US_PER_DAY);
    let (y, m, d) = civil_from_days(days);
    let secs = (rem / US_PER_SEC) as u32;
    let frac = (rem % US_PER_SEC) as u32;
    if (0..=9999).contains(&y) {
        let y = y as u32;
        push2(out, y / 100);
        push2(out, y % 100);
    } else {
        out.push_str(&format!("{y:04}"));
    }
    out.push('-');
    push2(out, m);
    out.push('-');
    push2(out, d);
    out.push('T');
    push2(out, secs / 3600);
    out.push(':');
    push2(out, (secs / 60) % 60);
    out.push(':');
    push2(out, secs % 60);
    if frac != 0 {
        out.push_str(&format!(".{frac:06}"));
    }
    out.push('Z');
}

pub fn ts_string(us: i64) -> String {
    let mut s = String::with_capacity(27);
    format_ts_us(us, &mut s);
    s
}

/// Parse durations such as `10s`, `12h`, `1d`, `30m`, `500ms`, `1h30m` into microseconds.
pub fn parse_duration_us(s: &str) -> Result<i64> {
    let t = s.trim();
    if t.is_empty() {
        bail!("empty duration");
    }
    let b = t.as_bytes();
    let mut i = 0;
    let mut total: i64 = 0;
    while i < b.len() {
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if start == i {
            bail!("bad duration {s:?}: expected a number");
        }
        let n: i64 = t[start..i].parse()?;
        let ustart = i;
        while i < b.len() && b[i].is_ascii_alphabetic() {
            i += 1;
        }
        let mult = match &t[ustart..i] {
            "us" | "µs" => 1,
            "ms" => 1_000,
            "s" => US_PER_SEC,
            "m" => 60 * US_PER_SEC,
            "h" => 3_600 * US_PER_SEC,
            "d" => US_PER_DAY,
            u => bail!("bad duration {s:?}: unknown unit {u:?} (use us, ms, s, m, h, d)"),
        };
        total += n * mult;
    }
    Ok(total)
}

/// PostgreSQL interval text for a duration, e.g. `43200000000` -> `12 hours`.
pub fn pg_interval(us: i64) -> String {
    for (unit, size) in [
        ("days", US_PER_DAY),
        ("hours", 3_600 * US_PER_SEC),
        ("minutes", 60 * US_PER_SEC),
        ("seconds", US_PER_SEC),
    ] {
        if us % size == 0 {
            return format!("{} {}", us / size, unit);
        }
    }
    format!("{} microseconds", us)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for s in [
            "2016-01-01T00:00:00Z",
            "1999-12-31T23:59:59.123456Z",
            "2024-02-29T12:00:00Z",
            "1969-07-20T20:17:40Z",
        ] {
            let us = parse_ts_us(s).unwrap();
            assert_eq!(ts_string(us), s);
        }
        assert_eq!(
            parse_ts_us("2016-01-01T00:00:00Z").unwrap(),
            1_451_606_400 * US_PER_SEC
        );
        assert_eq!(
            parse_ts_us("2016-01-01 02:00:00+02:00").unwrap(),
            1_451_606_400 * US_PER_SEC
        );
        assert_eq!(
            parse_ts_us("2016-01-01T00:00:00.123456789Z").unwrap(),
            1_451_606_400 * US_PER_SEC + 123_456
        );
        assert!(parse_ts_us("2016-01-01T00:00:00Zx").is_err());
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration_us("10s").unwrap(), 10 * US_PER_SEC);
        assert_eq!(parse_duration_us("1h30m").unwrap(), 5_400 * US_PER_SEC);
        assert_eq!(parse_duration_us("1d").unwrap(), 86_400 * US_PER_SEC);
        assert_eq!(pg_interval(parse_duration_us("12h").unwrap()), "12 hours");
        assert!(parse_duration_us("12x").is_err());
    }
}
