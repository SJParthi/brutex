//! Calendar readings: the IST day, the civil date, the weekday, the week and month keys, the candle's number in the
//! session counted from the session table's OPEN (TM-CANDLE, K14) and its start time (TM-TOD, a named duplicate).

/// IST offset from UTC, seconds.
pub const IST: i64 = 19_800;

/// The IST day number (days since 1970-01-01 IST) of an epoch second (0 outside the i32 range).
#[must_use]
pub fn ist_day(t: i64) -> i32 {
    i32::try_from((t + IST).div_euclid(86_400)).unwrap_or(0)
}

/// Seconds since IST midnight of an epoch second.
#[must_use]
pub const fn ist_sod(t: i64) -> i64 {
    (t + IST).rem_euclid(86_400)
}

/// The epoch second of IST `hh:mm:ss` on IST day `day`.
#[must_use]
pub fn at(day: i32, hh: i64, mm: i64, ss: i64) -> i64 {
    i64::from(day) * 86_400 - IST + hh * 3600 + mm * 60 + ss
}

/// Weekday of an IST day: 0 = Sunday ... 6 = Saturday (1970-01-01 was a Thursday).
#[must_use]
pub fn weekday(day: i32) -> u8 {
    u8::try_from((i64::from(day) + 4).rem_euclid(7)).unwrap_or(0)
}

/// The civil (year, month, day) of a day number (H. Hinnant's `civil_from_days`, integer only).
#[must_use]
pub fn civil(day: i32) -> (i32, u32, u32) {
    let z = i64::from(day) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (i32::try_from(y).unwrap_or(0), u32::try_from(m).unwrap_or(1), u32::try_from(d).unwrap_or(1))
}

/// The day number of a civil date (H. Hinnant's `days_from_civil`).
#[must_use]
pub fn days_from_civil(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { i64::from(y) - 1 } else { i64::from(y) };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mi = i64::from(m);
    let doy = (153 * (if mi > 2 { mi - 3 } else { mi + 9 }) + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    i32::try_from(era * 146_097 + doe - 719_468).unwrap_or(0)
}

/// `YYYY-MM-DD` of a day number.
#[must_use]
pub fn ymd(day: i32) -> String {
    let (y, m, d) = civil(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Parses `YYYY-MM-DD`.
#[must_use]
pub fn parse_ymd(s: &str) -> Option<i32> {
    let mut it = s.trim().split('-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// The calendar week key (weeks start on Monday): equal for the days of one Monday..Sunday week.
#[must_use]
pub const fn week_key(day: i32) -> i32 {
    (day + 3).div_euclid(7)
}

/// The calendar month key `year x 12 + month - 1`.
#[must_use]
pub fn month_key(day: i32) -> i32 {
    let (y, m, _) = civil(day);
    y * 12 + i32::try_from(m).unwrap_or(1) - 1
}

/// The candle's number in its session counted from the session table's OPEN (K14): `k = ceil((start - open) / tf) + 1`,
/// so the candle that starts at the open, or that starts before it and overlaps it, is k = 1, and each later candle of
/// the grid adds 1. On a grid anchored at the open (every regular session, the 4-minute and seconds folds) this is
/// v1's `floor + 1`. REPAIR 1 (check-build-feat-v1 B2): a vendor candle that starts BEFORE the open but holds real
/// trades of the session (Zerodha's 09:15-anchored 60-minute 17:15-18:15 candle of a 17:30 Muhurat) is k = 1, never
/// 0, and the next one k = 2, never a second k = 1. Under the mutant `m_cidx_0915` it counts from a fixed 09:15:00 IST;
/// under `m_cidx_floor` with v1's floor (k = 0 for such a candle).
#[must_use]
pub fn candle_index(start: i64, session_open: i64, tf: i64) -> i64 {
    let anchor = if cfg!(feature = "m_cidx_0915") { at(ist_day(start), 9, 15, 0) } else { session_open };
    let tf = tf.max(1);
    let x = start - anchor;
    if cfg!(feature = "m_cidx_floor") {
        return x.div_euclid(tf) + 1;
    }
    (x + tf - 1).div_euclid(tf) + 1
}

/// K11 / SNIPER-ADANIENT-PLAN 2.1 HALTS: true when the instant `t` (a decision candle's END) lies inside a market-wide
/// halt window `[from, resume)` of the dated session table (epoch seconds; `from` inclusive, `resume` exclusive).
/// A decision candle whose END lies inside a halt never enters (`decision_in_halt`, REPAIR 1, check-build-feat-v1 B3).
/// Under the mutant `m_halt_ignored` it is never true.
#[must_use]
pub fn in_halt(t: i64, halts: &[(i64, i64)]) -> bool {
    if cfg!(feature = "m_halt_ignored") {
        return false;
    }
    halts.iter().any(|&(from, resume)| t >= from && t < resume)
}
