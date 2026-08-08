//! The wall clock, in UTC, written twice on purpose.
//!
//! # Why both `ms` and `ts` are on the line
//!
//! `"ms":1786312991427` is what a filter compares — one integer, no calendar,
//! no parsing, and `since` is a `<` against it. `"ts":"2026-08-08T14:03:11.427Z"`
//! is what a person reads when they `grep` the file at three in the morning.
//! Carrying only the integer makes the file unreadable by hand; carrying only
//! the string makes every time filter a date parse. Twenty-four extra bytes a
//! line buys both, and the two can never disagree because the string is
//! derived from the integer at the moment it is written, in [`push_rfc3339`].
//!
//! # Why UTC and not IST
//!
//! The rest of this repository thinks in IST because the exchange does. A log
//! does not: it is read next to other machines' logs, it is compared across a
//! restart, and IST has no daylight rule today but the file will outlive that
//! claim. UTC with a `Z` is unambiguous and sorts lexicographically in the
//! same order as it sorts numerically.
//!
//! # No dependency
//!
//! The civil-date conversion is Howard Hinnant's `civil_from_days`, which is
//! exact integer arithmetic over a 400-year cycle with no table and no
//! leap-second concept — the same thing every date library does inside. It is
//! twelve lines. A dependency for twelve lines would be a transitive graph
//! forced on every crate that takes this one; see `Cargo.toml`.

use std::time::{Duration, SystemTime};

/// Milliseconds in one day. No leap seconds: `SystemTime` has none either.
pub(crate) const MILLIS_PER_DAY: i64 = 86_400_000;

/// Now, as milliseconds since the Unix epoch, negative before it.
#[must_use]
pub fn now_millis() -> i64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(since) => millis_of(&since, false),
        // A clock set before 1970. Not a failure and not a refusal: the event
        // happened, and the honest record of when is a negative number.
        Err(before) => millis_of(&before.duration(), true),
    }
}

/// A duration either side of the epoch, as signed milliseconds.
///
/// Saturating rather than wrapping. The saturating arm needs a clock roughly
/// 292 million years from the epoch, which no host reaches — but it is
/// reachable from a test, and [`tests::a_duration_past_the_width_of_the_answer_saturates`]
/// drives it, because a wrap here would print a date on the wrong side of the
/// epoch and that is worse than a clamp.
pub(crate) fn millis_of(span: &Duration, before_epoch: bool) -> i64 {
    let magnitude = i64::try_from(span.as_millis()).unwrap_or(i64::MAX);
    if before_epoch { -magnitude } else { magnitude }
}

/// The civil year, month and day a count of days since 1970-01-01 names.
///
/// Howard Hinnant's `civil_from_days`, shifted to an era starting 0000-03-01
/// so that the leap day is the last day of the era and needs no special case.
/// Exact for every `i64` day this crate can produce.
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    // 719_468 = days from 0000-03-01 to 1970-01-01.
    let shifted = days.saturating_add(719_468);
    // Floor division towards the era start, which is what makes the negative
    // side exact rather than merely close.
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097; // [0, 146_096]
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365; // [0, 399]
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153; // [0, 11], March is 0
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1; // [1, 31]
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// `YYYY-MM-DDTHH:MM:SS.mmmZ`, appended.
///
/// Fixed width for every year in `1000..=9999`, which is every year a running
/// process will produce. A year outside that widens or gains a leading `-`
/// rather than being clamped: a lie about the date would be worse than an
/// unusual column width, and nothing parses this field by offset.
pub(crate) fn push_rfc3339(out: &mut Vec<u8>, millis: i64) {
    let days = millis.div_euclid(MILLIS_PER_DAY);
    let in_day = millis.rem_euclid(MILLIS_PER_DAY);
    let (year, month, day) = civil_from_days(days);
    if year < 0 {
        out.push(b'-');
    }
    push_padded(out, year.unsigned_abs(), 4);
    out.push(b'-');
    push_padded(out, month.unsigned_abs(), 2);
    out.push(b'-');
    push_padded(out, day.unsigned_abs(), 2);
    out.push(b'T');
    push_padded(out, in_day.unsigned_abs() / 3_600_000, 2);
    out.push(b':');
    push_padded(out, (in_day.unsigned_abs() / 60_000) % 60, 2);
    out.push(b':');
    push_padded(out, (in_day.unsigned_abs() / 1_000) % 60, 2);
    out.push(b'.');
    push_padded(out, in_day.unsigned_abs() % 1_000, 3);
    out.push(b'Z');
}

/// A number, zero-padded to at least `width` digits.
pub(crate) fn push_padded(out: &mut Vec<u8>, value: u64, width: usize) {
    let mut digits = [0u8; 20];
    let mut count = 0usize;
    let mut rest = value;
    loop {
        // A `match` and not `b'0' + (rest % 10) as u8`: the cast is denied by
        // the workspace lints, and every arm here is driven by formatting a
        // number that uses every digit.
        let digit = match rest % 10 {
            0 => b'0',
            1 => b'1',
            2 => b'2',
            3 => b'3',
            4 => b'4',
            5 => b'5',
            6 => b'6',
            7 => b'7',
            8 => b'8',
            _ => b'9',
        };
        if let Some(slot) = digits.get_mut(count) {
            *slot = digit;
        }
        count = count.saturating_add(1);
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    for _ in count..width {
        out.push(b'0');
    }
    for i in (0..count).rev() {
        if let Some(byte) = digits.get(i) {
            out.push(*byte);
        }
    }
}

/// A signed number, with a `-` when it needs one.
pub(crate) fn push_i64(out: &mut Vec<u8>, value: i64) {
    if value < 0 {
        out.push(b'-');
    }
    push_padded(out, value.unsigned_abs(), 1);
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]
mod tests {
    use super::{
        Duration, MILLIS_PER_DAY, civil_from_days, millis_of, now_millis, push_i64, push_padded,
        push_rfc3339,
    };

    fn stamp(millis: i64) -> String {
        let mut out = Vec::new();
        push_rfc3339(&mut out, millis);
        String::from_utf8(out).expect("ASCII")
    }

    /// THE DATE ARITHMETIC IS THE ONE THING HERE NOBODY CAN EYEBALL.
    ///
    /// Every value below is a date whose epoch-millisecond value is fixed and
    /// checkable, and they are chosen to hit each way the conversion can be
    /// wrong: the epoch itself, a leap day in a year divisible by 4, the
    /// century that is NOT a leap year, the century that is, the last
    /// millisecond of a day, and both sides of the epoch.
    #[test]
    fn a_known_instant_formats_to_its_known_utc_string() {
        assert_eq!(stamp(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(stamp(1), "1970-01-01T00:00:00.001Z");
        assert_eq!(stamp(MILLIS_PER_DAY - 1), "1970-01-01T23:59:59.999Z");
        assert_eq!(stamp(MILLIS_PER_DAY), "1970-01-02T00:00:00.000Z");
        // 2000-02-29 — a leap day in a century year that IS divisible by 400.
        assert_eq!(stamp(951_782_400_000), "2000-02-29T00:00:00.000Z");
        // 1900-03-01 — the day after 1900-02-28, because 1900 is NOT a leap
        // year. A conversion using the naive four-year rule prints 1900-02-29.
        assert_eq!(stamp(-2_203_891_200_000), "1900-03-01T00:00:00.000Z");
        // 2024-02-29 — an ordinary leap day.
        assert_eq!(stamp(1_709_164_800_000), "2024-02-29T00:00:00.000Z");
        // A real instant with every field non-zero.
        assert_eq!(stamp(1_786_197_791_427), "2026-08-08T14:03:11.427Z");
        // Before the epoch, which is where `div_euclid`/`rem_euclid` earn their
        // place: a truncating divide would put this on the wrong day.
        assert_eq!(stamp(-1), "1969-12-31T23:59:59.999Z");
        assert_eq!(stamp(-MILLIS_PER_DAY), "1969-12-31T00:00:00.000Z");
    }

    #[test]
    fn a_year_outside_four_digits_widens_or_signs_rather_than_lying() {
        // Year 1 and a negative year both exist in the i64 range and neither is
        // clamped into the four-digit column.
        let (year, month, day) = civil_from_days(-719_162);
        assert_eq!((year, month, day), (1, 1, 1), "0001-01-01");
        assert!(stamp(-719_162 * MILLIS_PER_DAY).starts_with("0001-01-01"));
        let ancient = stamp(-800_000 * MILLIS_PER_DAY);
        assert!(
            ancient.starts_with('-'),
            "a negative year is signed: {ancient}"
        );
    }

    #[test]
    fn the_civil_conversion_round_trips_across_a_whole_leap_cycle() {
        // Every day of a 400-year era, walked forward: the month must stay in
        // 1..=12, the day in 1..=31, and the sequence must be strictly
        // increasing when read as (y, m, d). That catches an off-by-one in any
        // of the four divisions without pinning 146,097 literals.
        let mut previous = (i64::MIN, 0, 0);
        for days in -20_000..126_097 {
            let now = civil_from_days(days);
            assert!((1..=12).contains(&now.1), "{days} gave month {}", now.1);
            assert!((1..=31).contains(&now.2), "{days} gave day {}", now.2);
            assert!(
                now > previous,
                "{days}: {now:?} did not follow {previous:?}"
            );
            previous = now;
        }
    }

    #[test]
    fn a_duration_past_the_width_of_the_answer_saturates_instead_of_wrapping() {
        assert_eq!(millis_of(&Duration::from_millis(5), false), 5);
        assert_eq!(millis_of(&Duration::from_millis(5), true), -5);
        assert_eq!(millis_of(&Duration::ZERO, true), 0);
        // 2^64 seconds is far past what an i64 of milliseconds can hold. A
        // wrap here would print a date on the wrong side of the epoch.
        let absurd = Duration::new(u64::MAX, 999_999_999);
        assert_eq!(millis_of(&absurd, false), i64::MAX);
        assert_eq!(millis_of(&absurd, true), i64::MIN + 1);
    }

    #[test]
    fn the_clock_is_after_this_file_was_written_and_is_not_a_constant() {
        // 2026-01-01T00:00:00Z. Not a tautology: it fails if `now_millis`
        // returns zero, returns seconds instead of milliseconds, or returns
        // microseconds.
        let now = now_millis();
        assert!(now > 1_767_225_600_000, "{now} is not a plausible now");
        assert!(now < 4_102_444_800_000, "{now} is past the year 2100");
    }

    #[test]
    fn padding_widens_but_never_truncates_and_every_digit_has_a_byte() {
        let mut out = Vec::new();
        push_padded(&mut out, 7, 3);
        push_padded(&mut out, 1_234_567_890, 1);
        push_padded(&mut out, 0, 2);
        push_padded(&mut out, 98_765, 2);
        assert_eq!(String::from_utf8(out).unwrap(), "00712345678900098765");

        let mut wide = Vec::new();
        push_padded(&mut wide, u64::MAX, 1);
        assert_eq!(String::from_utf8(wide).unwrap(), "18446744073709551615");
    }

    #[test]
    fn a_signed_number_carries_its_sign_and_the_extremes_do_not_wrap() {
        let mut out = Vec::new();
        push_i64(&mut out, 0);
        out.push(b' ');
        push_i64(&mut out, -1);
        out.push(b' ');
        push_i64(&mut out, i64::MIN);
        out.push(b' ');
        push_i64(&mut out, i64::MAX);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "0 -1 -9223372036854775808 9223372036854775807"
        );
    }
}
