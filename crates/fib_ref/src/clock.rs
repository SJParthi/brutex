//! IST wall clock in seconds of the day, the session's named instants, and
//! civil dates.
//!
//! A second `t` is seconds since 00:00:00 IST. Minute `m` of the session is
//! `09:15 + m`. A candle labelled `t` on timeframe `tf` covers `[t, t + tf)`
//! and closes at `min(t + tf, 15:30:00)` (PREVIEW-SPEC 4); the grid starts at
//! the session's first minute, 09:15 (the reading this crate takes; see the
//! report's ambiguity on the grid).

/// 09:15:00, the session's first second.
pub const OPEN: u32 = 9 * 3600 + 15 * 60;
/// 15:30:00, the close instant of the session's last minute.
pub const CANDLE_END: u32 = 15 * 3600 + 30 * 60;
/// 15:29:59, the last continuous-trading second before the auction era.
pub const SESSION_END: u32 = CANDLE_END - 1;
/// 15:14:59, the last continuous-trading second of an auction-eligible
/// stock-day from 2026-08-03 (PREVIEW-SPEC 3.4).
pub const SESSION_END_AUCTION: u32 = 15 * 3600 + 15 * 60 - 1;
/// 15:07:59, the last entry decision S42 may take (PREVIEW-SPEC 9.2).
pub const LAST_ENTRY: u32 = 15 * 3600 + 8 * 60 - 1;
/// 15:08:00: a strategy exit order starting at or after it is rejected, and
/// an entry's unfilled remainder is cancelled at it (PREVIEW-SPEC 9.2).
pub const CUT: u32 = 15 * 3600 + 8 * 60;
/// 15:10:00: the broker squares off whatever is open (PREVIEW-SPEC 9.2).
pub const SQUARE_OFF: u32 = 15 * 3600 + 10 * 60;
/// Minutes in a full session, 09:15 to 15:29.
pub const SESSION_MINUTES: u32 = 375;
/// Lane S's latency, 14 s on every stock-day (PREVIEW-SPEC 11.1).
pub const LAMBDA: u32 = 14;

/// The second at which session minute `m` starts.
pub fn minute_start(m: u32) -> u32 {
    OPEN + 60 * m
}

/// The session minute holding second `t`, or `None` before the open or after
/// the last minute.
pub fn minute_of(t: u32) -> Option<u32> {
    if !(OPEN..CANDLE_END).contains(&t) {
        return None;
    }
    Some((t - OPEN) / 60)
}

/// `HH:MM:SS` of a second of the day.
pub fn hms(t: u32) -> String {
    format!("{:02}:{:02}:{:02}", t / 3600, (t / 60) % 60, t % 60)
}

/// Parses `HH:MM:SS` or `HH:MM` into seconds of the day.
pub fn parse_hms(s: &str) -> Option<u32> {
    let (h, rest) = s.split_once(':')?;
    // a fourth part leaves a colon in the seconds, which then fail to parse
    let (m, sec) = rest.split_once(':').unwrap_or((rest, "0"));
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    let sec: u32 = sec.parse().ok()?;
    if h > 23 || m > 59 || sec > 59 {
        return None;
    }
    Some(h * 3600 + m * 60 + sec)
}

/// A civil date (proleptic Gregorian).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// Year.
    pub y: i32,
    /// Month, 1..=12.
    pub m: u32,
    /// Day of the month, 1..=31.
    pub d: u32,
}

impl Date {
    /// A date, checked for a real calendar day.
    pub fn new(y: i32, m: u32, d: u32) -> Option<Date> {
        if !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
            return None;
        }
        Some(Date { y, m, d })
    }

    /// Days since 1970-01-01 (negative before it).
    pub fn days(self) -> i64 {
        // Howard Hinnant's days_from_civil, in exact integers.
        let y = i64::from(self.y) - i64::from(self.m <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.m);
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + i64::from(self.d) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `n` days after 1970-01-01.
    pub fn from_days(n: i64) -> Date {
        let z = n + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        Date {
            y: i32::try_from(y).unwrap_or(i32::MAX),
            m: u32::try_from(m).unwrap_or(1),
            d: u32::try_from(d).unwrap_or(1),
        }
    }

    /// The calendar month as one integer, `y * 12 + m - 1`.
    pub fn month_index(self) -> i32 {
        self.y * 12 + i32::try_from(self.m).unwrap_or(1) - 1
    }

    /// Day of the week, 0 = Monday .. 6 = Sunday.
    pub fn weekday(self) -> u32 {
        // 1970-01-01 was a Thursday (3).
        u32::try_from((self.days() + 3).rem_euclid(7)).unwrap_or(0)
    }

    /// `YYYY-MM-DD`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.y, self.m, self.d)
    }

    /// Parses `YYYY-MM-DD`.
    pub fn parse(s: &str) -> Option<Date> {
        let (y, rest) = s.split_once('-')?;
        // a fourth part leaves a dash in the day, which then fails to parse
        let (m, d) = rest.split_once('-')?;
        Date::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?)
    }
}

/// `YYYY-MM` of a month index.
pub fn month_label(index: i32) -> String {
    format!(
        "{:04}-{:02}",
        index.div_euclid(12),
        index.rem_euclid(12) + 1
    )
}

/// Days in a month of a year.
pub fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
#[path = "clock_tests.rs"]
mod tests;
