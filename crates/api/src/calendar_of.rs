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
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

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

/// The per-instrument calendar cache `Site` holds.
///
/// A named alias because the bare type is four levels deep and appears in both
/// the field and the accessor, where clippy is right that a reader has to parse
/// it twice to learn it is a map.
pub type Cache = std::sync::Mutex<
    std::collections::HashMap<(Vendor, String, String, String), (std::time::SystemTime, Calendar)>,
>;

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

/// The manifest file whose modification time keys the cache.
///
/// One file per vendor, rewritten on every pull that stores anything, so its
/// mtime is the cheapest honest "has the store changed" signal available — one
/// `stat`, no directory walk, no counter to maintain.
fn manifest_stamp(store_root: &Path, vendor: Vendor) -> Option<std::time::SystemTime> {
    std::fs::metadata(
        store_root
            .join("manifest")
            .join(format!("{}.man", vendor.as_str())),
    )
    .ok()?
    .modified()
    .ok()
}

/// The calendar for one instrument, derived once per store change.
///
/// # Why this is not just [`derive`]
///
/// `derive` measured **0.28 s** for one instrument across 81 months — fine once
/// after a pull, and not fine on a page that polls every two seconds. This
/// returns the cached answer when the vendor's manifest has not been rewritten
/// since it was built, which is one `stat` and one map probe.
///
/// **A store that has never been written has no manifest and therefore no
/// stamp.** That case re-derives every call, which is correct rather than
/// unfortunate: an empty store derives an empty calendar in microseconds, and
/// caching "I found nothing" against a key that cannot change would answer
/// `Unmeasured` for ever once the first pull landed.
pub fn cached(
    site_calendars: &Cache,
    store_root: &Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    months: &[YearMonth],
) -> Calendar {
    let stamp = manifest_stamp(store_root, vendor);
    // THE KEY IS THE WHOLE PATH THIS DERIVATION READS, NOT JUST THE NAME.
    //
    // It was `(vendor, symbol)`, which was safe only for as long as every
    // caller passed the same `exchange`/`segment` — and they did, because both
    // passed the literals `"NSE"` and `"INDEX"`. Fixing the caller to send the
    // census's real segment makes one symbol addressable under two of them, and
    // a two-field key would then serve `NSE/CASH/X`'s calendar to a request for
    // `NSE/INDEX/X` under a cache hit. That is the same class of defect the
    // literals caused, arriving through the cache instead of the path: an
    // answer for a different series wearing the shape of the right one.
    let key = (
        vendor,
        exchange.to_owned(),
        segment.to_owned(),
        symbol.to_owned(),
    );
    if let Some(now) = stamp {
        // READ THROUGH A POISONED LOCK rather than around it. A panic while
        // holding it means some other request died; the map is still readable,
        // and refusing to look would make one panicked request cost every later
        // one a 0.28 s re-derivation for the life of the process.
        let held = site_calendars
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((at, calendar)) = held.get(&key)
            && *at == now
        {
            return calendar.clone();
        }
    }

    let (calendar, _report) = derive(store_root, vendor, exchange, segment, symbol, months);
    if let Some(now) = stamp {
        let mut held = site_calendars
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.insert(key, (now, calendar.clone()));
    }
    calendar
}

/// The calendar as JSON: which days traded, and how many minute bars each owes.
///
/// # The shape, and why it is days rather than tables
///
/// The browser held four tables of this — 92 holidays, 8 weekend sessions, 4
/// short sessions, 5 with no minute series — derived from the same bars on the
/// same afternoon as `crates/pull/src/calendar.rs` and checked against it by
/// nothing. Serving **days** instead of **rules** removes the duplication
/// entirely: there is no rule to keep in step, only an answer to read.
///
/// `owed` is `null` for a day the exchange traded and this build cannot size —
/// the five pre-2025 Muhurats. `null` is not `0`: one says nobody has measured
/// the session, the other says the exchange was shut, and a page that renders
/// them the same reports a Muhurat as a holiday.
///
/// Closed days are **omitted** rather than listed as `owed: 0`. A reader wants
/// the sessions; the gaps between them are the closed days by construction, and
/// listing 784 zeroes would be most of the payload.
#[must_use]
pub fn json(calendar: &Calendar) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64 * 1024);
    out.push_str("{\"firstDay\":");
    let _ = write!(out, "{}", calendar.first_day());
    out.push_str(",\"lastDay\":");
    let _ = write!(out, "{}", calendar.last_day());
    out.push_str(",\"sessions\":");
    let _ = write!(out, "{}", calendar.sessions());
    out.push_str(",\"days\":[");
    let mut first = true;
    for day in calendar.first_day()..=calendar.last_day() {
        let kind = calendar.kind_of(day);
        if matches!(
            kind,
            pull::calendar::DayKind::Closed | pull::calendar::DayKind::Unmeasured
        ) {
            continue;
        }
        if !first {
            out.push(',');
        }
        first = false;
        let _ = write!(out, "{{\"day\":{day},\"owed\":");
        match calendar.expected_bars(day) {
            Some(bars) => {
                let _ = write!(out, "{bars}");
            }
            None => out.push_str("null"),
        }
        out.push('}');
    }
    out.push_str("]}");
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod wire {
    use super::*;

    /// **CLOSED DAYS ARE OMITTED AND UNSIZED DAYS ARE `null`, NOT `0`.**
    ///
    /// Both halves matter. Listing 784 closed days as `owed: 0` would be most
    /// of the payload for no information — the gaps between sessions ARE the
    /// closed days. And rendering an unsized Muhurat as `0` would tell the page
    /// the exchange was shut on a day a daily bar proves it traded, which is the
    /// confident wrong answer the whole calendar exists to remove.
    #[test]
    fn the_wire_distinguishes_a_closed_day_from_one_it_cannot_size() {
        let cal = Calendar::from_observed(&[
            // A full session.
            pull::calendar::Observed::from_runs(100, &[(555, 929)]),
            // Day 101 is absent entirely -> Closed -> omitted.
            // A day that traded with no minute series -> owed null.
            pull::calendar::Observed::from_runs(102, &[]),
        ]);
        let wire = json(&cal);

        assert!(wire.contains("\"firstDay\":100"), "{wire}");
        assert!(wire.contains("\"lastDay\":102"), "{wire}");
        assert!(wire.contains("\"sessions\":2"), "{wire}");
        assert!(
            wire.contains("{\"day\":100,\"owed\":375}"),
            "a full session is sized: {wire}"
        );
        assert!(
            wire.contains("{\"day\":102,\"owed\":null}"),
            "a day that traded and cannot be sized is null, NOT 0: {wire}"
        );
        assert!(
            !wire.contains("\"day\":101"),
            "a closed day is omitted rather than listed as zero: {wire}"
        );
    }

    /// **AN EMPTY CALENDAR IS VALID JSON THAT CLAIMS NOTHING.**
    ///
    /// The state the operator was in after clearing the store. A page fed a
    /// truncated or malformed document here would fail in the renderer, which
    /// is a worse place to discover an empty store than the payload.
    #[test]
    fn an_empty_calendar_is_still_well_formed() {
        let wire = json(&Calendar::from_observed(&[]));
        assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
        assert!(
            wire.contains("\"days\":[]"),
            "no days, and the array is there: {wire}"
        );
        assert!(wire.contains("\"sessions\":0"), "{wire}");
    }
}

/// One day the instruments did not read the same way.
///
/// **This is a finding, not an error.** Two instruments on one exchange trade
/// the same days; where their stores disagree, one of them has a hole. Naming
/// the day and who was silent is what turns "the calendar is fuzzy here" into
/// "this instrument is missing this day".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disagreement {
    /// The day, as an epoch day.
    pub day: i64,
    /// Instruments whose store proves a session — sorted.
    pub seen_by: Vec<String>,
    /// Instruments in range for that day whose store holds nothing — sorted.
    pub silent: Vec<String>,
    /// Every distinct session length measured, ascending. One entry means the
    /// instruments agreed on the length and disagreed only about existence.
    pub owed: Vec<u16>,
}

/// Agree several instruments' readings into one exchange calendar.
///
/// # The rule, and why it is a union rather than an intersection
///
/// **A bar is proof that the exchange traded; silence is not proof that it did
/// not.** So a day is a session if ANY instrument observed one, and the session
/// taken is the LONGEST any instrument measured. A shorter reading is that
/// instrument's hole, not a shorter exchange day — the opposite rule would let
/// one vendor's missing morning shorten the calendar for everything else, which
/// is how an expected-bar count becomes quietly too small and a real loss stops
/// being reported.
///
/// **A day outside a reading's own span is not a vote.** An instrument whose
/// history begins in 2019 has no opinion about 2015, and counting its absence
/// as a closure would manufacture four years of holidays. Only readings whose
/// `first_day..=last_day` contains the day are consulted.
///
/// `OpenLengthUnmeasured` survives agreement only when nothing measured a
/// length — one instrument that saw the Muhurat session's 60 minutes settles it
/// for all of them, and that is the same "a bar is proof" rule.
///
/// # Cost
///
/// One pass over the union span, and for each day one lookup per reading —
/// bounded by the instrument count, which is the two the engine sweeps plus
/// whatever else the store holds. Not O(1) and not on a bar path.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn agree(readings: &[(String, Calendar)]) -> (Calendar, Vec<Disagreement>) {
    use pull::calendar::DayKind;

    let live: Vec<&(String, Calendar)> = readings
        .iter()
        .filter(|(_, calendar)| calendar.sessions() > 0)
        .collect();
    let (Some(first), Some(last)) = (
        live.iter().map(|(_, c)| c.first_day()).min(),
        live.iter().map(|(_, c)| c.last_day()).max(),
    ) else {
        return (Calendar::from_observed(&[]), Vec::new());
    };

    let mut observed: Vec<Observed> = Vec::new();
    let mut clashes: Vec<Disagreement> = Vec::new();
    for day in first..=last {
        let mut longest: Option<pull::calendar::Session> = None;
        let mut seen_by: Vec<String> = Vec::new();
        let mut silent: Vec<String> = Vec::new();
        let mut owed: Vec<u16> = Vec::new();
        for (name, calendar) in &live {
            if day < calendar.first_day() || day > calendar.last_day() {
                continue;
            }
            match calendar.kind_of(day) {
                DayKind::Open(session) => {
                    seen_by.push(name.clone());
                    let bars = session.bars();
                    if !owed.contains(&bars) {
                        owed.push(bars);
                    }
                    if longest.is_none_or(|held| held.bars() < bars) {
                        longest = Some(session);
                    }
                }
                // SEEN, BUT NOT SIZED. It votes for the day existing and not for its
                // length, which is what leaving `longest` alone means.
                DayKind::OpenLengthUnmeasured => seen_by.push(name.clone()),
                DayKind::Closed | DayKind::Unmeasured => silent.push(name.clone()),
            }
        }
        if seen_by.is_empty() {
            continue;
        }
        observed.push(Observed {
            day,
            // `from_observed` reads `None` as "traded, length unknown", which is
            // exactly the Muhurat case and exactly what is meant when every
            // instrument that saw the day saw it without a minute series.
            session: longest,
        });
        if !silent.is_empty() || owed.len() > 1 {
            owed.sort_unstable();
            seen_by.sort();
            silent.sort();
            clashes.push(Disagreement {
                day,
                seen_by,
                silent,
                owed,
            });
        }
    }
    (Calendar::from_observed(&observed), clashes)
}

/// The exchange calendar on the wire, with the evidence behind it.
///
/// A superset of [`json`]: the same `days` array, plus who it was derived from
/// and every day they disagreed about. **The provenance is not decoration** —
/// a calendar agreed from one instrument and a calendar agreed from six are
/// different claims, and a caller that cannot tell them apart will trust the
/// first as much as the second.
#[must_use]
pub fn exchange_json(
    calendar: &Calendar,
    derived_from: &[String],
    clashes: &[Disagreement],
) -> String {
    use std::fmt::Write as _;
    let inner = json(calendar);
    // `json` closes with `]}`; the extra members are spliced before that brace
    // rather than the whole payload rebuilt, so the two can never describe the
    // same days differently.
    let body = inner.strip_suffix('}').unwrap_or(&inner);
    let mut out = String::with_capacity(body.len() + 1024);
    out.push_str(body);
    out.push_str(",\"derivedFrom\":[");
    for (position, name) in derived_from.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        out.push_str(&crate::pullrun::quote_for_json(name));
    }
    out.push_str("],\"disagreements\":[");
    for (position, clash) in clashes.iter().enumerate() {
        if position > 0 {
            out.push(',');
        }
        let _ = write!(out, "{{\"day\":{},\"seenBy\":[", clash.day);
        for (n, name) in clash.seen_by.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            out.push_str(&crate::pullrun::quote_for_json(name));
        }
        out.push_str("],\"silent\":[");
        for (n, name) in clash.silent.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            out.push_str(&crate::pullrun::quote_for_json(name));
        }
        out.push_str("],\"owed\":[");
        for (n, bars) in clash.owed.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(out, "{bars}");
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod agreement {
    use super::*;

    /// 09:15–15:29 inclusive — the 375-minute session.
    const FULL_RUN: [(u16, u16); 1] = [(555, 929)];
    /// A 60-minute Muhurat session.
    const SHORT_RUN: [(u16, u16); 1] = [(555, 614)];

    /// **A BAR IS PROOF; SILENCE IS NOT.**
    ///
    /// Two halves in one test because they are one rule. A day only one
    /// instrument saw is still a session — the other has a hole, and the hole is
    /// reported rather than allowed to close the exchange. A day that falls
    /// outside an instrument's own history is not that instrument voting
    /// "closed": it has no opinion, and counting its absence would manufacture
    /// holidays for every year before it started.
    #[test]
    fn a_day_one_instrument_missed_is_a_session_and_a_day_it_predates_is_not_its_vote() {
        let nifty = Calendar::from_observed(&[
            Observed::from_runs(100, &FULL_RUN),
            Observed::from_runs(101, &FULL_RUN),
        ]);
        let banknifty = Calendar::from_observed(&[
            Observed::from_runs(100, &FULL_RUN),
            Observed::from_runs(102, &FULL_RUN),
        ]);
        let (exchange, clashes) = agree(&[
            ("NIFTY".to_owned(), nifty),
            ("BANKNIFTY".to_owned(), banknifty),
        ]);

        assert_eq!(exchange.sessions(), 3, "the union, not the intersection");
        assert_eq!(exchange.expected_bars(101), Some(375));
        assert_eq!(exchange.expected_bars(102), Some(375));

        // Day 101 is in BANKNIFTY's span and missing from its store: a hole,
        // and named. Day 102 is past NIFTY's last day, so NIFTY is not silent
        // about it -- it was never asked.
        assert_eq!(clashes.len(), 1, "{clashes:?}");
        let only = clashes.first().expect("one clash");
        assert_eq!(only.day, 101);
        assert_eq!(only.seen_by, ["NIFTY"]);
        assert_eq!(only.silent, ["BANKNIFTY"]);
    }

    /// **THE LONGEST READING WINS, because a short one is that store's hole.**
    ///
    /// The opposite rule is the dangerous one: letting a vendor's missing
    /// morning shorten the exchange day makes every expected-bar count for that
    /// day too small, so a real loss stops being reported as a loss.
    #[test]
    fn the_longest_session_measured_is_the_one_the_exchange_gets() {
        let whole = Calendar::from_observed(&[Observed::from_runs(200, &FULL_RUN)]);
        let holed = Calendar::from_observed(&[Observed::from_runs(200, &SHORT_RUN)]);
        let (exchange, clashes) =
            agree(&[("HOLED".to_owned(), holed), ("WHOLE".to_owned(), whole)]);

        assert_eq!(exchange.expected_bars(200), Some(375));
        let only = clashes.first().expect("a length disagreement is a clash");
        assert_eq!(only.owed, [60, 375], "ascending, and both are reported");
        assert!(only.silent.is_empty(), "neither was silent: {only:?}");
        assert_eq!(only.seen_by, ["HOLED", "WHOLE"]);
    }

    /// **ONE MEASUREMENT SETTLES A MUHURAT FOR ALL OF THEM.**
    ///
    /// `OpenLengthUnmeasured` means "a daily bar proves it traded and no minute
    /// bar sizes it". That is a property of a STORE, not of the exchange, so it
    /// survives agreement only while nothing measured a length — and the moment
    /// one instrument did, withholding the number would be refusing evidence
    /// rather than declining to invent it.
    #[test]
    fn an_unsized_day_stays_unsized_only_until_something_sizes_it() {
        let blind = Calendar::from_observed(&[Observed::from_runs(300, &[])]);
        let alsoblind = Calendar::from_observed(&[Observed::from_runs(300, &[])]);
        let (dark, _) = agree(&[("A".to_owned(), blind.clone()), ("B".to_owned(), alsoblind)]);
        assert_eq!(dark.expected_bars(300), None, "nothing measured it");

        let sighted = Calendar::from_observed(&[Observed::from_runs(300, &SHORT_RUN)]);
        let (lit, _) = agree(&[("A".to_owned(), blind), ("B".to_owned(), sighted)]);
        assert_eq!(
            lit.expected_bars(300),
            Some(60),
            "one measurement is enough"
        );
    }

    /// **A FEED THAT HOLDS NOTHING CANNOT CLOSE THE EXCHANGE.**
    ///
    /// `site.entries` spans every vendor, so asking one feed's store for an
    /// instrument only another carries reads no files and yields an empty
    /// calendar. Counted as agreement it would vote "closed" on every day there
    /// is, which is the confident wrong answer this module exists to remove.
    #[test]
    fn an_empty_reading_is_excluded_rather_than_counted_as_closed() {
        let real = Calendar::from_observed(&[Observed::from_runs(400, &FULL_RUN)]);
        let (exchange, clashes) = agree(&[
            ("ABSENT".to_owned(), Calendar::from_observed(&[])),
            ("REAL".to_owned(), real),
        ]);
        assert_eq!(exchange.sessions(), 1);
        assert!(clashes.is_empty(), "{clashes:?}");

        let (nothing, none) = agree(&[]);
        assert_eq!(nothing.sessions(), 0);
        assert!(none.is_empty());
    }

    /// **THE PROVENANCE TRAVELS WITH THE ANSWER.**
    ///
    /// A calendar agreed from one instrument and one agreed from six are
    /// different claims. A caller that cannot tell them apart trusts the first
    /// as much as the second, so `derivedFrom` is on the wire beside the days
    /// rather than left for a reader to assume.
    #[test]
    fn the_exchange_wire_carries_who_it_was_derived_from_and_where_they_differed() {
        let nifty = Calendar::from_observed(&[
            Observed::from_runs(500, &FULL_RUN),
            Observed::from_runs(501, &FULL_RUN),
        ]);
        let vix = Calendar::from_observed(&[
            Observed::from_runs(500, &FULL_RUN),
            Observed::from_runs(501, &SHORT_RUN),
        ]);
        let readings = [("NIFTY".to_owned(), nifty), ("INDIAVIX".to_owned(), vix)];
        let (exchange, clashes) = agree(&readings);
        let from: Vec<String> = readings.iter().map(|(name, _)| name.clone()).collect();
        let wire = exchange_json(&exchange, &from, &clashes);

        assert!(wire.starts_with('{') && wire.ends_with('}'), "{wire}");
        assert!(wire.contains("\"days\":["), "{wire}");
        assert!(wire.contains("{\"day\":500,\"owed\":375}"), "{wire}");
        assert!(
            wire.contains("\"derivedFrom\":[\"NIFTY\",\"INDIAVIX\"]"),
            "{wire}"
        );
        assert!(
            wire.contains(
                "{\"day\":501,\"seenBy\":[\"INDIAVIX\",\"NIFTY\"],\"silent\":[],\"owed\":[60,375]}"
            ),
            "{wire}"
        );

        // The per-symbol payload is untouched: no caller of `json` gains fields
        // it did not ask for.
        assert!(!json(&exchange).contains("derivedFrom"));
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod segments {
    use super::*;

    /// One day's worth of bars for `symbol` under `segment`, on the daily rung.
    ///
    /// The daily rung alone is enough: `derive` builds its set of traded days
    /// from `DAY_1` and only measures session LENGTH from `MINUTE_1`, so a
    /// store with days and no minutes is a calendar with sessions of unknown
    /// size — which is exactly the shape a segment probe either finds or does
    /// not.
    fn write_day(root: &std::path::Path, segment: &str, symbol: &str, month: YearMonth) {
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Dhan,
            exchange: "NSE",
            segment,
            symbol,
            contract: None,
            timeframe: Timeframe::DAY_1,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path");
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the id is the cross-check `open` folds; any 32 bits serve"
        )]
        let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
        let mut file =
            store::file::BarFile::open_or_create(root, path, symbol_id).expect("a bar file");
        // MIDDAY ON THE FIRST, so the bar is unambiguously inside the month the
        // path names — the store resolves a record's slot from its stamp, and a
        // bar stamped outside its own month is a different test, not a smaller
        // one. Days-since-epoch by the civil-from-days algorithm the store uses.
        let (y, m) = (i64::from(month.year()), i64::from(month.month()));
        let (y2, m2) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
        let era = y2.div_euclid(400);
        let yoe = y2 - era * 400;
        let doy = (153 * m2 + 2) / 5;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        let rows = [store::format::Bar {
            ts_micros: days * 86_400 * 1_000_000 + 6 * 3_600 * 1_000_000,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: i64::MIN,
        }];
        file.append(&rows).expect("one legal daily bar");
    }

    /// **A CALENDAR IS READ FROM THE SEGMENT IT IS GIVEN, AND THERE IS NO
    /// DEFAULT.**
    ///
    /// This is the regression test for the defect that cost `ADANIENT` its
    /// whole history on the page. `calendar_json` keyed its month map on the
    /// bare symbol and passed the literals `"NSE"` and `"INDEX"` for every
    /// instrument, so an equity — stored under `CASH`, exactly where the
    /// master's `instrument_type` column puts an `EQ` row — was probed at
    /// `NSE/INDEX/ADANIENT/`. **366 measured refusals**, 61 held months × 2
    /// rungs × 3 derivations, against 1,240 daily bars that were sitting on
    /// disk the whole time.
    ///
    /// Both halves are asserted because only the pair proves the property. That
    /// the right segment reads is necessary; that the WRONG one comes back
    /// empty is what makes the first half evidence rather than coincidence — a
    /// `derive` that ignored its `segment` argument and always found the file
    /// would pass the first assertion alone.
    #[test]
    fn a_cash_instrument_is_read_from_cash_and_is_absent_from_index() {
        let root = std::env::temp_dir().join(format!(
            "brutex-calendar-segment-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        let month = YearMonth::new(2026, 1).expect("a legal month");
        write_day(&root, "CASH", "ADANIENT", month);

        let (found, found_report) = derive(
            &root,
            Vendor::Dhan,
            "NSE",
            "CASH",
            "ADANIENT",
            std::slice::from_ref(&month),
        );
        assert_eq!(
            found.sessions(),
            1,
            "the segment the bars were written under reads them back"
        );
        assert!(
            found_report.unreadable.is_empty(),
            "nothing was unreadable: {:?}",
            found_report.unreadable
        );

        let (missed, missed_report) = derive(
            &root,
            Vendor::Dhan,
            "NSE",
            "INDEX",
            "ADANIENT",
            std::slice::from_ref(&month),
        );
        assert_eq!(
            missed.sessions(),
            0,
            "the wrong segment finds nothing — which is why the literal was a \
             silent wrong answer rather than a loud one"
        );
        assert!(
            !missed_report.unreadable.is_empty(),
            "and it is REPORTED unreadable rather than passed off as a month \
             of holidays"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
