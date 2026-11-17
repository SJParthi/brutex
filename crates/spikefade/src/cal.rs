//! Time helpers (integer only). Instants for the fill crate are UTC microseconds (`at_sod`, the sell-diag's rule); everything else is
//! the IST day (days since 1970-01-01) and the IST second of the day (sod).

/// One second, microseconds.
pub const US_S: i64 = 1_000_000;
/// IST = UTC + 05:30, seconds.
pub const IST_S: i64 = 19_800;
/// 09:15:00 IST: the minute grid's anchor (Tickvault anchors every timeframe here, contract 4.1).
pub const OPEN_S: i64 = 9 * 3_600 + 15 * 60;
/// Minute slots of the grid: 09:15 .. 15:39 (Tickvault folds trades in [09:15, 15:40)).
pub const NM: usize = 385;
/// 15:08:00: no own order at or after it (REQ 70 S14, the B1 cut).
pub const CUT_S: i64 = 15 * 3_600 + 8 * 60;
/// 15:10:00: the broker square-off.
pub const SQ_S: i64 = 15 * 3_600 + 10 * 60;
/// 15:30:00: the market close; a decision candle must end at or before it to count as an event.
pub const CLOSE_S: i64 = 15 * 3_600 + 30 * 60;
/// 15:40:00: the option session end used by the sell-diag from 2026-08-03 (the square-off walks to it; UNVERIFIED, inherited).
pub const END_S: i64 = 15 * 3_600 + 40 * 60;

/// The UTC instant (microseconds) of IST day `day` at `sod` seconds after IST midnight.
#[must_use]
pub const fn at_sod(day: i64, sod: i64) -> i64 {
    (day * 86_400 + sod - IST_S) * US_S
}

/// The IST second of the day of an instant.
#[must_use]
pub const fn sod_of(us: i64) -> i64 {
    (us.div_euclid(US_S) + IST_S).rem_euclid(86_400)
}

/// Days since 1970-01-01 -> (year, month, day) (Hinnant).
#[must_use]
pub const fn civil(day: i64) -> (i64, i64, i64) {
    let z = day + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// (year, month, day) -> days since 1970-01-01 (Hinnant's inverse).
#[must_use]
pub const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// "YYYY-MM-DD" -> day.
#[must_use]
pub fn parse_day(s: &str) -> Option<i64> {
    let mut it = s.trim().split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// yyyymmdd -> day.
#[must_use]
pub const fn day_of_ymd(ymd: u32) -> i64 {
    let y = (ymd / 10_000) as i64;
    let m = ((ymd / 100) % 100) as i64;
    let d = (ymd % 100) as i64;
    days_from_civil(y, m, d)
}

/// day -> yyyymmdd.
#[must_use]
pub fn ymd_of(day: i64) -> u32 {
    let (y, m, d) = civil(day);
    u32::try_from(y * 10_000 + m * 100 + d).unwrap_or(0)
}

/// "YYYY-MM-DD".
#[must_use]
pub fn fmt_day(day: i64) -> String {
    let (y, m, d) = civil(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// "HH:MM:SS" of a second of the day.
#[must_use]
pub fn fmt_sod(s: i64) -> String {
    if s < 0 {
        return "-".to_owned();
    }
    format!("{:02}:{:02}:{:02}", s / 3_600, (s / 60) % 60, s % 60)
}

/// "HH:MM:SS" of an instant (IST).
#[must_use]
pub fn fmt_us(us: i64) -> String {
    if us <= 0 { "-".to_owned() } else { fmt_sod(sod_of(us)) }
}

/// Paisa as "1,234.56" (Indian grouping not attempted; plain thousands with commas).
#[must_use]
pub fn fmt_rs(p: i64) -> String {
    let neg = p < 0;
    let a = p.unsigned_abs();
    let r = a / 100;
    let f = a % 100;
    let s = r.to_string();
    let mut out = String::new();
    let b = s.as_bytes();
    for (i, c) in b.iter().enumerate() {
        if i > 0 && (b.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(char::from(*c));
    }
    format!("{}{out}.{f:02}", if neg { "-" } else { "" })
}
