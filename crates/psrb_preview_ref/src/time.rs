//! Civil days, calendar months and IST seconds of the day.
//!
//! IST has no daylight saving, so an IST instant is a civil day plus a second of
//! that day, and the UTC instant is fixed 19,800 seconds earlier (X-F08-8).

/// Seconds in a day.
pub const DAY_SECONDS: u32 = 86_400;
/// 09:15:00 IST: the session opens.
pub const OPEN: u32 = 33_300;
/// 15:30:00 IST: the continuous close of a full session.
pub const CLOSE_FULL: u32 = 55_800;
/// 15:15:00 IST: the continuous close of a closing-auction-eligible stock-day (from 2026-08-03).
pub const CLOSE_AUCTION: u32 = 54_900;
/// 15:14:00 IST: the last minute an auction-eligible stock-day may hold.
pub const LAST_AUCTION_MINUTE: u32 = 54_840;
/// 15:08:00 IST: no entry order from this instant.
pub const ENTRY_CUT: u32 = 54_480;
/// 15:07:59 IST: the time exit order.
pub const TIME_EXIT: u32 = 54_479;
/// 15:10:00 IST: the broker's square-off.
pub const SQUARE_OFF: u32 = 54_600;
/// IST is UTC + 5:30.
pub const IST_OFFSET: u32 = 19_800;

/// A civil date, held as days since 1970-01-01.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Day(i32);

/// A calendar month, held as `12 x year + (month - 1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Month(i32);

/// Days from 1970-01-01 to a civil date (Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9).rem_euclid(12);
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The civil date of a day count (Hinnant's `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = (mp + 2).rem_euclid(12) + 1;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// Days in a month of a year (Gregorian).
const fn month_length(y: i32, m: u32) -> u32 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

impl Day {
    /// The day `y-m-d`, or `None` when it is not a date in 1970..=2199.
    #[must_use]
    pub fn from_ymd(y: i32, m: u32, d: u32) -> Option<Self> {
        let in_range = (1970..=2199).contains(&y) && (1..=12).contains(&m);
        if !in_range || d == 0 || d > month_length(y, m) {
            return None;
        }
        let n = days_from_civil(i64::from(y), i64::from(m), i64::from(d));
        i32::try_from(n).ok().map(Self)
    }

    /// The day `n` days after 1970-01-01.
    #[must_use]
    pub const fn from_index(n: i32) -> Self {
        Self(n)
    }

    /// Days since 1970-01-01.
    #[must_use]
    pub const fn index(self) -> i32 {
        self.0
    }

    /// Year, month and day of the month.
    #[must_use]
    pub fn ymd(self) -> (i32, u32, u32) {
        let (y, m, d) = civil_from_days(i64::from(self.0));
        (
            i32::try_from(y).unwrap_or(0),
            u32::try_from(m).unwrap_or(0),
            u32::try_from(d).unwrap_or(0),
        )
    }

    /// 0 = Monday .. 6 = Sunday (1970-01-01 was a Thursday).
    #[must_use]
    pub fn weekday(self) -> u32 {
        u32::try_from((i64::from(self.0) + 3).rem_euclid(7)).unwrap_or(0)
    }

    /// The calendar month holding the day.
    #[must_use]
    pub fn month(self) -> Month {
        let (y, m, _) = self.ymd();
        Month::of(y, m)
    }

    /// The next day.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// `YYYY-MM-DD`.
    #[must_use]
    pub fn label(self) -> String {
        let (y, m, d) = self.ymd();
        format!("{y:04}-{m:02}-{d:02}")
    }
}

impl Month {
    /// The month `m` of year `y` (`m` in 1..=12; any other value wraps into the year).
    #[must_use]
    pub fn of(y: i32, m: u32) -> Self {
        let m0 = i32::try_from(m).unwrap_or(1) - 1;
        Self(y * 12 + m0)
    }

    /// The raw ordinal `12 x year + month - 1`.
    #[must_use]
    pub const fn ordinal(self) -> i32 {
        self.0
    }

    /// Year and month (1..=12).
    #[must_use]
    pub fn ym(self) -> (i32, u32) {
        let y = self.0.div_euclid(12);
        let m = u32::try_from(self.0.rem_euclid(12) + 1).unwrap_or(1);
        (y, m)
    }

    /// The next month.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// The previous month.
    #[must_use]
    pub const fn prev(self) -> Self {
        Self(self.0 - 1)
    }

    /// The first day of the month, `None` outside the supported years.
    #[must_use]
    pub fn first_day(self) -> Option<Day> {
        let (y, m) = self.ym();
        Day::from_ymd(y, m, 1)
    }

    /// The first day of the month, for any year (the proleptic Gregorian calendar).
    #[must_use]
    pub fn start(self) -> Day {
        let (y, m) = self.ym();
        let n = days_from_civil(i64::from(y), i64::from(m), 1);
        Day(i32::try_from(n).unwrap_or(i32::MAX))
    }

    /// The last day of the month, for any year.
    #[must_use]
    pub fn end(self) -> Day {
        Day(self.next().start().0 - 1)
    }

    /// Every day of the month, in order (empty outside the supported years).
    #[must_use]
    pub fn days(self) -> Vec<Day> {
        let (y, m) = self.ym();
        (1..=month_length(y, m)).filter_map(|d| Day::from_ymd(y, m, d)).collect()
    }

    /// `YYYY-MM`.
    #[must_use]
    pub fn label(self) -> String {
        let (y, m) = self.ym();
        format!("{y:04}-{m:02}")
    }
}

/// `hh:mm:ss` of a second of the day.
#[must_use]
pub fn hms(sec: u32) -> String {
    format!("{:02}:{:02}:{:02}", sec / 3_600, (sec / 60) % 60, sec % 60)
}

/// `hh:mm` of a second of the day.
#[must_use]
pub fn hm(sec: u32) -> String {
    format!("{:02}:{:02}", sec / 3_600, (sec / 60) % 60)
}

/// The IST second of the day `hh:mm:ss`, `None` when a field is out of range.
#[must_use]
pub fn ist(hh: u32, mm: u32, ss: u32) -> Option<u32> {
    (hh < 24 && mm < 60 && ss < 60).then_some(hh * 3_600 + mm * 60 + ss)
}

/// The UTC instant, in microseconds since 1970-01-01, of an IST second of a day.
#[must_use]
pub fn utc_micros(day: Day, sec: u32) -> i64 {
    let local = i64::from(day.index()) * i64::from(DAY_SECONDS) + i64::from(sec);
    (local - i64::from(IST_OFFSET)) * 1_000_000
}

/// The IST day and second of a UTC instant in microseconds; `None` off a whole second.
#[must_use]
pub fn from_utc_micros(micros: i64) -> Option<(Day, u32)> {
    if micros.rem_euclid(1_000_000) != 0 {
        return None;
    }
    let local = micros.div_euclid(1_000_000) + i64::from(IST_OFFSET);
    let day = i32::try_from(local.div_euclid(i64::from(DAY_SECONDS))).ok()?;
    let sec = u32::try_from(local.rem_euclid(i64::from(DAY_SECONDS))).ok()?;
    Some((Day(day), sec))
}

#[cfg(test)]
#[path = "time_tests.rs"]
mod tests;
