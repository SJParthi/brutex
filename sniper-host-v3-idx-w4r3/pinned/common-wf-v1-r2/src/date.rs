//! IST session dates (days since 1970-01-01) and month keys (year * 12 + month - 1), P1. Integer only.

use crate::{Day, Mk, WfErr};

/// Days from a civil date (H. Hinnant's algorithm), or None out of range.
#[must_use]
pub fn days_from_civil(y: i64, m: i64, d: i64) -> Option<Day> {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Day::try_from(era * 146_097 + doe - 719_468).ok()
}

/// The civil date (year, month, day) of a day.
#[must_use]
pub fn civil_from_days(z: Day) -> (i64, i64, i64) {
    let z = i64::from(z) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn digits(s: &str) -> Option<i64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// Parses `YYYY-MM-DD`.
///
/// # Errors
/// `BadDay` for anything else or an impossible date.
pub fn parse_day(s: &str) -> Result<Day, WfErr> {
    let mut it = s.split('-');
    let (Some(a), Some(b), Some(c), None) = (it.next(), it.next(), it.next(), it.next()) else { return Err(WfErr::BadDay) };
    if a.len() != 4 || b.len() != 2 || c.len() != 2 {
        return Err(WfErr::BadDay);
    }
    let (Some(y), Some(m), Some(d)) = (digits(a), digits(b), digits(c)) else { return Err(WfErr::BadDay) };
    let z = days_from_civil(y, m, d).ok_or(WfErr::BadDay)?;
    if civil_from_days(z) != (y, m, d) {
        return Err(WfErr::BadDay);
    }
    Ok(z)
}

/// Parses `YYYY-MM` into a month key.
#[must_use]
pub fn parse_mk(s: &str) -> Option<Mk> {
    let (a, b) = s.split_once('-')?;
    if a.len() != 4 || b.len() != 2 {
        return None;
    }
    let (y, m) = (digits(a)?, digits(b)?);
    if !(1..=12).contains(&m) {
        return None;
    }
    Mk::try_from(y * 12 + m - 1).ok()
}

/// The month key of a day.
#[must_use]
pub fn mk_of_day(z: Day) -> Mk {
    let (y, m, _) = civil_from_days(z);
    Mk::try_from(y * 12 + m - 1).unwrap_or(Mk::MAX)
}
/// The year of a month key.
#[must_use]
pub const fn mk_y(mk: Mk) -> i32 {
    mk.div_euclid(12)
}
/// The month (1..=12) of a month key.
#[must_use]
pub const fn mk_m(mk: Mk) -> i32 {
    mk.rem_euclid(12) + 1
}
/// `YYYY-MM`.
#[must_use]
pub fn mk_str(mk: Mk) -> String {
    format!("{}-{:02}", mk_y(mk), mk_m(mk))
}
/// `YYYY-MM-DD`.
#[must_use]
pub fn day_str(z: Day) -> String {
    let (y, m, d) = civil_from_days(z);
    format!("{y}-{m:02}-{d:02}")
}
/// The first day of a month.
#[must_use]
pub fn first_day(mk: Mk) -> Day {
    days_from_civil(i64::from(mk_y(mk)), i64::from(mk_m(mk)), 1).unwrap_or(Day::MAX)
}
/// The last day of a month.
#[must_use]
pub fn last_day(mk: Mk) -> Day {
    first_day(mk.saturating_add(1)).saturating_sub(1)
}
/// The three-letter month name.
#[must_use]
pub const fn mon(mk: Mk) -> &'static str {
    match mk_m(mk) {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        _ => "Dec",
    }
}
/// `YYYY-MM..YYYY-MM`.
#[must_use]
pub fn range_str(a: Mk, b: Mk) -> String {
    format!("{}..{}", mk_str(a), mk_str(b))
}
/// `YYYY..YYYY` (or `YYYY` for one year).
#[must_use]
pub fn years_str(a: Mk, b: Mk) -> String {
    if mk_y(a) == mk_y(b) { format!("{}", mk_y(a)) } else { format!("{}..{}", mk_y(a), mk_y(b)) }
}
/// An overlap as whole years `YYYY-YYYY` (or `YYYY`) when it runs January..December, else `YYYY-MM..YYYY-MM`.
#[must_use]
pub fn overlap_str(a: Mk, b: Mk) -> String {
    if mk_m(a) == 1 && mk_m(b) == 12 {
        if mk_y(a) == mk_y(b) { format!("{}", mk_y(a)) } else { format!("{}-{}", mk_y(a), mk_y(b)) }
    } else {
        range_str(a, b)
    }
}
