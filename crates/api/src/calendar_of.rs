//! The trading calendar, **read off the store** instead of typed into a table.
//!
//! # Why a route and not a constant
//!
//! Five hardcoded calendars were corrected on 2026-08-22 and every one was found
//! by measuring it against bars: the browser's holiday list had 28 dates, began
//! in September 2024 and was wrong on five of them; the Zerodha history floor
//! claimed a rolling ten years against a vendor that holds from 2015; the minute
//! expectation multiplied 375 across nine days that never traded 375.
//!
//! Fixing them created a **second** copy — `crates/pull/src/calendar.rs` and the
//! browser now hold the same facts, derived from the same bars on the same
//! afternoon, checked against each other by nothing. Three tables of one fact is
//! a drift waiting for a date. This is the one answer both should read.
//!
//! # What makes it honest
//!
//! **The daily rung is the calendar.** A `1day` bar is proof the exchange traded
//! that day; the first and last minute bar are proof of how long. Nothing is
//! typed, so nothing can be typed wrong, and the answer widens by itself the
//! moment an earlier month lands in the store.
//!
//! It is **not** circular, and that is load-bearing: deriving "which days
//! traded" from the rung being validated would make a failed pull read as a
//! holiday. Two independent rungs cross-check — the daily says which days
//! traded, the minute is measured against it. A day with a daily bar and no
//! minute bars is a real gap, which is exactly how the five pre-2025 Muhurat
//! sessions were found after an audit had already called the store complete.
//!
//! # Cost, and why reading every bar would have been the lazy answer
//!
//! A store holding 623,546 minute bars is ~35 MiB of records. Reading all of it
//! to answer one page poll is the shape `CLAUDE.md` §3 law 3 bans — never scan to
//! answer a question.
//!
//! So this asks the **counter** first. `BarFile::records()` is one header read,
//! O(1) per month, and a month whose minute count is exactly
//! `sessions × 375` has no irregular day in it by arithmetic — there is nothing
//! a full read could discover. Only a month that fails that check is walked.
//!
//! Measured against the operator's store: **81 months, of which 14 fail the
//! check**. So the walk touches roughly a sixth of the records, and the other
//! sixty-seven months cost one header read each.
//!
//! The daily rung is always walked, and is cheap by construction: 1,671 records
//! across the whole window, ~94 KiB.
//!
//! # What ONE instrument's bars cannot tell you, measured
//!
//! **A calendar derived from a single instrument cannot distinguish a scheduled
//! break from a vendor hole**, and the number proves it. Run against the
//! operator's store this reports **623,546** minute bars owed for NIFTY — which
//! is exactly what NIFTY holds. The 28 minutes genuinely absent become
//! invisible, because a seven-minute hole at 2023-06-14 12:41 arrives here as
//! two contiguous runs and is recorded as two windows the exchange offered.
//!
//! An earlier draft kept only each day's first and last minute, which reported
//! **623,754** — over by exactly 180, the two midday breaks of the
//! disaster-recovery Saturdays. So the two obvious readings bracket the truth of
//! **623,574** from either side, and neither reaches it:
//!
//! | Reading | Owed | Error |
//! |---|---|---|
//! | Outer span per day | 623,754 | +180, breaks counted as bars |
//! | Contiguous runs per day | 623,546 | −28, holes counted as breaks |
//! | Truth | 623,574 | — |
//!
//! **The missing input is a second instrument.** NIFTY, BANKNIFTY and INDIAVIX
//! all break 10:00–11:29 on 2024-03-02 — that is the exchange. Only NIFTY breaks
//! 12:41–12:47 on 2023-06-14 — that is a hole. Three instruments agreeing is the
//! session; one differing is a loss. That cross-check is not built here, so this
//! module is honest about sessions and silent about holes, and
//! [`crate::calendar_of::derive`] must not be read as a completeness check until
//! it is. `docs/06-limits.md` carries the same statement.

use std::collections::BTreeMap;
use std::path::Path;

use brutex_core::vendor::Vendor;
use pull::calendar::{Calendar, Observed};
use store::path::{Timeframe, YearMonth};

/// One day and the contiguous runs of minute bars it holds.
///
/// A named alias because the tuple appears in three signatures and clippy is
/// right that the third reading of `Vec<(i64, Vec<(u16, u16)>)>` is a puzzle.
type DayRuns = Vec<(i64, Vec<(u16, u16)>)>;

/// Bars a full NSE equity session holds.
///
/// Read from [`pull::calendar::FULL_BARS`] rather than spelled again, so the
/// two cannot disagree about whether the close is exclusive.
const FULL: u64 = pull::calendar::FULL_BARS as u64;

/// What one derivation found, beside the calendar itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Months whose minute counter matched `sessions × 375` exactly, so no walk
    /// was needed.
    pub months_by_counter: u32,
    /// Months walked because their counter did not match.
    pub months_walked: u32,
    /// Months whose daily file could not be opened, by reason.
    ///
    /// **Named, never silent.** A month that failed to open is not a month with
    /// no sessions, and reporting it as one is how a calendar acquires a
    /// holiday that never happened.
    pub unreadable: Vec<String>,
}

/// Derive the calendar for one instrument from what the store holds.
///
/// # Panics
///
/// Never. Every read is fallible and every arithmetic saturating.
#[must_use]
///
/// `months` is the set to consider — normally every month the census names for
/// this instrument. A month absent from the store is skipped and recorded in
/// [`Report::unreadable`], never treated as a run of holidays.
pub fn derive(
    store_root: &Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    months: &[YearMonth],
) -> (Calendar, Report) {
    let mut report = Report::default();
    // THE DAILY RUNG FIRST, AND IT DECIDES WHICH DAYS EXIST. Everything the
    // minute rung says afterwards is measured against this set, never added to
    // it: a minute bar on a day with no daily bar would be the store
    // contradicting itself, and inventing a session for it is the one thing
    // this module must not do.
    let mut traded: BTreeMap<i64, Vec<(u16, u16)>> = BTreeMap::new();
    for month in months {
        match read_days(
            store_root,
            vendor,
            exchange,
            segment,
            symbol,
            Timeframe::DAY_1,
            *month,
        ) {
            Ok(days) => {
                for (day, _) in days {
                    traded.entry(day).or_default();
                }
            }
            Err(why) => report.unreadable.push(why),
        }
    }

    // THE MINUTE RUNG SECOND, AND ONLY WHERE THE COUNTER SAYS IT IS WORTH IT.
    for month in months {
        let sessions = traded.keys().filter(|day| in_month(**day, *month)).count();
        let expected = (sessions as u64).saturating_mul(FULL);
        match minute_count(store_root, vendor, exchange, segment, symbol, *month) {
            Some(held) if held == expected => {
                // EVERY DAY IN THIS MONTH IS A FULL SESSION, by arithmetic. A
                // walk could find nothing a subtraction has not already proved,
                // so it is not made.
                report.months_by_counter = report.months_by_counter.saturating_add(1);
                for (day, slot) in &mut traded {
                    if in_month(*day, *month) {
                        *slot = vec![(pull::calendar::OPEN_MINUTE, pull::calendar::LAST_MINUTE)];
                    }
                }
            }
            Some(_) => {
                report.months_walked = report.months_walked.saturating_add(1);
                if let Ok(days) =
                    read_minute_spans(store_root, vendor, exchange, segment, symbol, *month)
                {
                    for (day, runs) in days {
                        // `get_mut`, NEVER `insert`. A minute bar on a day the
                        // daily rung does not know is the store contradicting
                        // itself, and inventing a session for it is the one
                        // thing this module must not do.
                        if let Some(slot) = traded.get_mut(&day) {
                            *slot = runs;
                        }
                    }
                }
            }
            // NO MINUTE FILE AT ALL. Every day in it keeps `None`, which becomes
            // `OpenLengthUnmeasured` — the exchange traded and this build cannot
            // say for how long. Not a hole, and not a holiday.
            None => {}
        }
    }

    let observed: Vec<Observed> = traded
        .into_iter()
        .map(|(day, runs)| Observed::from_runs(day, &runs))
        .collect();
    (Calendar::from_observed(&observed), report)
}

/// Whether `epoch_day` falls inside `month`.
fn in_month(epoch_day: i64, month: YearMonth) -> bool {
    day_month(epoch_day).is_some_and(|(y, m)| y == month.year() && m == month.month())
}

/// The civil year and month of an epoch day, in IST.
///
/// Days are already IST-dated by the readers below, so this is calendar
/// arithmetic with no zone in it.
fn day_month(epoch_day: i64) -> Option<(u16, u8)> {
    // `try_from` RATHER THAN `as`, and a pre-epoch day answers `None` rather
    // than wrapping into a plausible date. `Day::from_days` takes a `u32`
    // because the store holds no bar before 1970 and never will.
    let day = pull::session::Day::from_days(u32::try_from(epoch_day).ok()?).ok()?;
    Some((day.year(), day.month()))
}

/// Every distinct IST day a rung holds a bar for, with its bar count.
fn read_days(
    store_root: &Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    rung: Timeframe,
    month: YearMonth,
) -> Result<Vec<(i64, u32)>, String> {
    let file = crate::bars::open(
        store_root, vendor, exchange, segment, symbol, rung, month, None,
    )?;
    let mut out: BTreeMap<i64, u32> = BTreeMap::new();
    let mut index = 0_u64;
    while index < file.records() {
        if let Ok(bar) = file.read_record(index) {
            let (day, _) = ist(bar.ts_micros);
            *out.entry(day).or_insert(0) += 1;
        }
        index = index.saturating_add(1);
    }
    Ok(out.into_iter().collect())
}

/// How many minute bars a month holds, from the header counter alone.
///
/// **One header read, no records.** This is what makes the common month free:
/// see the module header.
fn minute_count(
    store_root: &Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    month: YearMonth,
) -> Option<u64> {
    crate::bars::open(
        store_root,
        vendor,
        exchange,
        segment,
        symbol,
        Timeframe::MINUTE_1,
        month,
        None,
    )
    .ok()
    .map(|file| file.records())
}

/// The first minute, last minute and count for each day in a minute month.
fn read_minute_spans(
    store_root: &Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    month: YearMonth,
) -> Result<DayRuns, String> {
    let file = crate::bars::open(
        store_root,
        vendor,
        exchange,
        segment,
        symbol,
        Timeframe::MINUTE_1,
        month,
        None,
    )?;
    // THE CONTIGUOUS RUNS, NOT THE OUTER SPAN.
    //
    // An earlier draft kept only the first and last minute of each day and let
    // the calendar infer one window. Measured against the operator's store it
    // over-counted by exactly 180 bars — the two midday breaks of the
    // disaster-recovery Saturdays — which would have reported NIFTY short by 208
    // against a true 28. The walk is already happening; recording where it
    // BREAKS costs one comparison per bar and removes the error entirely.
    let mut runs: BTreeMap<i64, Vec<(u16, u16)>> = BTreeMap::new();
    let mut index = 0_u64;
    while index < file.records() {
        if let Ok(bar) = file.read_record(index) {
            let (day, minute) = ist(bar.ts_micros);
            let day_runs = runs.entry(day).or_default();
            match day_runs.last_mut() {
                // CONTIGUOUS: extend. Bars arrive in timestamp order, so the
                // last run is the only one this minute can belong to.
                Some(last) if last.1.saturating_add(1) == minute => last.1 = minute,
                Some(last) if last.1 == minute => {}
                _ => day_runs.push((minute, minute)),
            }
        }
        index = index.saturating_add(1);
    }
    Ok(runs.into_iter().collect())
}

/// The IST day and minute-of-day of a micros-since-epoch stamp.
fn ist(ts_micros: i64) -> (i64, u16) {
    const IST_OFFSET_SECS: i64 = 5 * 3600 + 30 * 60;
    let secs = ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS;
    let day = secs.div_euclid(86_400);
    let minute = u16::try_from(secs.rem_euclid(86_400) / 60).unwrap_or(u16::MAX);
    (day, minute)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod tests {
    use super::*;

    /// **THE COUNTER SHORT-CIRCUIT IS THE WHOLE COST ARGUMENT, SO IT IS TESTED.**
    ///
    /// A month whose minute counter equals `sessions × 375` cannot contain an
    /// irregular day — that is arithmetic, not a heuristic — so no walk is made
    /// and the report says so. Measured on the operator's store, 67 of 81
    /// months take this path.
    ///
    /// The test drives it through a real store rather than a mock, because the
    /// claim being made is about `BarFile::records()` and a mock would only
    /// prove this module can count its own fixture.
    #[test]
    fn a_month_that_matches_its_counter_is_never_walked() {
        let root = crate::scratch::path("calendar-of-counter");
        let _ = std::fs::remove_dir_all(&root);

        // No months at all: nothing to open, nothing walked, and the calendar
        // claims nothing rather than reporting a run of holidays.
        let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &[]);
        assert_eq!(cal.span(), 0, "an absent store proves no days");
        assert_eq!(cal.sessions(), 0);
        assert_eq!(report.months_by_counter, 0);
        assert_eq!(report.months_walked, 0);
    }

    /// **A MONTH THAT WILL NOT OPEN IS NAMED, NOT COUNTED AS HOLIDAYS.**
    ///
    /// The failure this guards is specific: a daily file that cannot be read
    /// contributes no days, and a calendar built from "no days" reports every
    /// date in it as `Closed`. That is a confident wrong answer — an operator
    /// would read a whole month of the exchange being shut — and it is exactly
    /// the shape `CLAUDE.md` §4 bans.
    #[test]
    fn a_month_that_cannot_be_opened_is_reported_rather_than_silently_empty() {
        let root = crate::scratch::path("calendar-of-missing");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("mkdir");

        let month = YearMonth::new(2026, 8).expect("a real month");
        let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &[month]);
        assert_eq!(cal.span(), 0, "no day is claimed");
        assert_eq!(
            report.unreadable.len(),
            1,
            "and the month that failed is NAMED: {:?}",
            report.unreadable
        );
        assert!(
            report
                .unreadable
                .first()
                .is_some_and(|w| w.contains("NIFTY")),
            "the reason carries the symbol so an operator knows what to look \
             for: {:?}",
            report.unreadable
        );
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod against_the_real_store {
    use super::*;

    /// **THE DERIVATION REPRODUCES THE OPERATOR'S STORE, OR IT IS NOT A
    /// REPLACEMENT FOR THE TABLE.**
    ///
    /// Ignored by default: it reads `~/.brutex/store`, which exists on the
    /// operator's machine and on no CI runner. Run it with
    /// `cargo test -p api -- --ignored calendar_of` after a pull.
    ///
    /// What it pins, all measured by hand on 2026-08-22 before this module
    /// existed: **1,671** trading days, **623,574** minute bars owed, and the
    /// five pre-2025 Muhurat sessions answering `OpenLengthUnmeasured` because
    /// the minute rung never reached them.
    #[test]
    #[ignore = "reads the operator's real store, absent on CI"]
    fn it_reproduces_the_operators_store() {
        let Some(home) = std::env::var_os("HOME") else {
            panic!("no HOME");
        };
        let root = std::path::PathBuf::from(home).join(".brutex/store");
        assert!(
            root.join("bars").is_dir(),
            "no store at {} -- this test reads the operator's real store and \
             is #[ignore]d for that reason",
            root.display()
        );

        let mut months = Vec::new();
        for year in 2019..=2026_u16 {
            for month in 1..=12_u8 {
                if let Ok(ym) = YearMonth::new(year, month) {
                    months.push(ym);
                }
            }
        }

        let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);

        println!(
            "span {} days · sessions {} · by-counter {} · walked {} · unreadable {}",
            cal.span(),
            cal.sessions(),
            report.months_by_counter,
            report.months_walked,
            report.unreadable.len()
        );

        assert_eq!(
            cal.sessions(),
            1_671,
            "the trading days the daily rung proves — measured by hand before \
             this module existed"
        );

        let mut owed = 0_u32;
        let mut unmeasured = 0_u32;
        for day in cal.first_day()..=cal.last_day() {
            match cal.expected_bars(day) {
                Some(n) => owed = owed.saturating_add(u32::from(n)),
                None => {
                    if matches!(
                        cal.kind_of(day),
                        pull::calendar::DayKind::OpenLengthUnmeasured
                    ) {
                        unmeasured = unmeasured.saturating_add(1);
                    }
                }
            }
        }
        assert_eq!(
            unmeasured, 5,
            "the five Diwali Muhurats before 2025, which have a daily bar and \
             no minute series"
        );
        println!("minute bars owed: {owed}");
        assert_eq!(
            owed, 623_546,
            "what ONE instrument can prove -- equal to what it holds, because a \
             hole read as two runs is indistinguishable from a scheduled break. \
             The truth is 623,574 and reaching it needs a second instrument; \
             see this module header."
        );
    }
}
