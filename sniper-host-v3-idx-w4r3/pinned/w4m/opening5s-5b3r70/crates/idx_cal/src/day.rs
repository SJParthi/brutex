//! Civil dates as day numbers (days since 1970-01-01), the proleptic Gregorian calendar (H. Hinnant's algorithm).

/// A civil date as days since 1970-01-01.
pub type Day = i32;

/// The day number of `y-m-d`.
#[must_use]
pub const fn from_civil(y: i32, m: u32, d: u32) -> Day {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    #[allow(clippy::cast_possible_wrap, reason = "mp <= 11 and d <= 31: small")]
    let doy = ((153 * mp + 2) / 5 + d - 1) as i32;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `(year, month, day)` of a day number.
#[must_use]
pub const fn to_civil(z: Day) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    #[allow(clippy::cast_sign_loss, reason = "1 <= d <= 31 and 1 <= m <= 12")]
    let (mu, du) = (m as u32, d as u32);
    (if m <= 2 { y + 1 } else { y }, mu, du)
}

/// Weekday, 0 = Sunday .. 6 = Saturday.
#[must_use]
pub const fn weekday(d: Day) -> u32 {
    #[allow(clippy::cast_sign_loss, reason = "rem_euclid(7) is 0..=6")]
    let w = (d + 4).rem_euclid(7) as u32;
    w
}

/// The weekday's three-letter name.
#[must_use]
pub fn weekday_name(d: Day) -> &'static str {
    ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].get(weekday(d) as usize).copied().unwrap_or("?")
}

/// `YYYY-MM-DD`.
#[must_use]
pub fn fmt(d: Day) -> String {
    let (y, m, dd) = to_civil(d);
    format!("{y:04}-{m:02}-{dd:02}")
}

/// Parses `YYYY-MM-DD` (strict: the date must exist).
#[must_use]
pub fn parse(s: &str) -> Option<Day> {
    let b = s.as_bytes();
    if b.len() != 10 || b.get(4) != Some(&b'-') || b.get(7) != Some(&b'-') {
        return None;
    }
    let y: i32 = s.get(0..4)?.parse().ok()?;
    let m: u32 = s.get(5..7)?.parse().ok()?;
    let d: u32 = s.get(8..10)?.parse().ok()?;
    let z = from_civil(y, m, d);
    (to_civil(z) == (y, m, d)).then_some(z)
}

/// Parses the old bhavcopy form `DD-Mon-YYYY` (`04-Apr-2019`, any case).
#[must_use]
pub fn parse_dmy(s: &str) -> Option<Day> {
    let mut it = s.split('-');
    let d: u32 = it.next()?.parse().ok()?;
    let mon = it.next()?.to_ascii_uppercase();
    let y: i32 = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    let m = u32::try_from(MONTHS.iter().position(|x| *x == mon)?).ok()? + 1;
    let z = from_civil(y, m, d);
    (to_civil(z) == (y, m, d)).then_some(z)
}

/// Month names as the old bhavcopy file names spell them.
pub const MONTHS: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

/// The last date of the month of `d`.
#[must_use]
pub const fn month_end(d: Day) -> Day {
    let (y, m, _) = to_civil(d);
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    from_civil(ny, nm, 1) - 1
}

/// `(year, month)` of `d`.
#[must_use]
pub const fn ym(d: Day) -> (i32, u32) {
    let (y, m, _) = to_civil(d);
    (y, m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_round_trips_and_weekdays() {
        assert_eq!(from_civil(1970, 1, 1), 0);
        assert_eq!(fmt(from_civil(2023, 6, 28)), "2023-06-28");
        assert_eq!(weekday_name(from_civil(2023, 6, 28)), "Wed");
        assert_eq!(weekday_name(from_civil(2025, 9, 30)), "Tue");
        assert_eq!(weekday_name(from_civil(2026, 3, 30)), "Mon");
        for z in from_civil(2017, 1, 1)..from_civil(2027, 12, 31) {
            let (y, m, d) = to_civil(z);
            assert_eq!(from_civil(y, m, d), z);
            assert_eq!(parse(&fmt(z)), Some(z));
        }
        assert_eq!(parse("2023-02-29"), None);
        assert_eq!(parse_dmy("04-Apr-2019"), parse("2019-04-04"));
        assert_eq!(parse_dmy("27-SEP-2018"), parse("2018-09-27"));
        assert_eq!(fmt(month_end(from_civil(2024, 2, 10))), "2024-02-29");
    }
}
