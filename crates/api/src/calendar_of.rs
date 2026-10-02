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
//! **The daily rung normally supplies the calendar.** A `1day` bar is proof the
//! exchange traded that day; its minute runs describe the windows. The one
//! exception is 2021-02-24: SEBI's primary record proves normal trading ran
//! 09:15–11:40 and 15:45–17:00, while every stored series shares the same
//! truncated 54-bar prefix. `pull::calendar::Calendar::from_observed` applies
//! that fixed exchange-session override so shared index truncation cannot
//! rewrite the market timetable. `pull::gaps::classify_spot_index_against`
//! separately refuses an exact index-bar denominator for that day.
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
//! across the whole window, ~94 KiB. Every walk is one positional read per
//! record, counted in [`Report::records_read`]; `docs/06-limits.md` states the
//! O(records) bound under D-0950.
//!
//! # A month whose daily rung was not read is withheld, not closed
//!
//! Only a daily rung that was read can say a day was shut. A month in the span
//! the census holds at another rung only, or whose daily records failed their
//! checks, is `Unmeasured` day by day and named in `withheld` on the wire,
//! rather than filled with `Closed` between the days on either side of it.
//! D-0950.
//!
//! # What ONE instrument's bars cannot tell you, measured
//!
//! **A calendar derived from a single instrument cannot distinguish a scheduled
//! break from a vendor hole.** Before D-0420, the operator's store made that
//! visible numerically: contiguous runs reported **623,546**, exactly what
//! NIFTY held; outer spans reported **623,754**; peer union reported **623,574**.
//! The 28-minute difference was a self-derived hole boundary.
//!
//! Primary evidence then proved that all three readings also shared the same
//! bad premise on 2021-02-24: the 54-bar index prefix was not the exchange
//! session. Replacing it with SEBI's 220 normal-market slots adds 166 to each
//! exchange-timetable reading: 623,712 / 623,920 / 623,740 respectively. None
//! is promoted to an exact spot-index total, because the same order records
//! unavailable NIFTY computation and no common index-publication interval.
//! `classify_spot_index_against` therefore withholds that date.
//!
//! **The usual missing input is a second instrument.** NIFTY, BANKNIFTY and INDIAVIX
//! all break 10:00–11:29 on 2024-03-02 — that is the exchange. Only NIFTY breaks
//! 12:41–12:47 on 2023-06-14 — that is a hole. Three instruments agreeing is the
//! session; one differing is a loss. That cross-check is not built here, so this
//! module is honest about sessions and silent about holes, except for the one
//! independently sourced 2021-02-24 session, and
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
/// # The cached value is an `Arc`, and it was a `Calendar`
///
/// [`cached`] returned `calendar.clone()` **while still holding the mutex**, so
/// every hit copied the whole calendar — a `Vec` of every day in the span — and
/// serialised every other request behind that copy.
///
/// That is the defect `d55937d6` fixed for the census cache one file over, in
/// its words: *"the census cache handed out a whole-store memcpy on every hit,
/// under the mutex."* The fix there was `Arc::clone`; the same cache one door
/// along kept the memcpy. A hit is now a refcount bump, and the lock is held
/// for a pointer copy rather than an allocation.
///
/// # Concurrent misses derive once, and wait for that one
///
/// [`cached`] released the map's lock before deriving, so every request that
/// missed the same key at the same stamp derived it again: a manifest rewrite
/// under a polling page, or a flood of `/calendar.json`, multiplied a
/// derivation documented at 0.28 s by the number of requests in flight
/// (W1-api2-11). A miss now registers a flight for its key and stamp; a
/// request that finds one waits for it and is answered what it derived. A
/// leader that panics abandons its flight, and each waiter then tries again,
/// one of them as the new leader. D-0950.
#[derive(Default)]
pub struct Cache {
    /// The kept calendars, keyed by series, each under the stamp it was
    /// derived from.
    held: std::sync::Mutex<
        std::collections::HashMap<Key, (std::time::SystemTime, std::sync::Arc<Calendar>)>,
    >,
    /// Derivations in progress, by series and stamp.
    flights: std::sync::Mutex<
        std::collections::HashMap<(Key, std::time::SystemTime), std::sync::Arc<Flight>>,
    >,
    /// Requests waiting on another's derivation right now. A gauge, for the
    /// single-flight test to know every follower has joined before the
    /// leader finishes; nothing in production reads it.
    waiting: std::sync::atomic::AtomicUsize,
}

/// The kept calendars' map, as [`Cache::lock`] hands it out.
pub type Held = std::collections::HashMap<Key, (std::time::SystemTime, std::sync::Arc<Calendar>)>;

/// One series' address: vendor, exchange, segment and symbol.
pub type Key = (Vendor, String, String, String);

impl std::fmt::Debug for Cache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache")
            .field("waiting", &self.waiting())
            .finish_non_exhaustive()
    }
}

impl Cache {
    /// The kept calendars, locked.
    ///
    /// # Errors
    ///
    /// A poisoned lock, exactly as [`std::sync::Mutex::lock`] reports it.
    pub fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, Held>> {
        self.held.lock()
    }

    /// Requests waiting on another request's derivation at this moment.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.waiting.load(std::sync::atomic::Ordering::Acquire)
    }
}

/// One derivation in progress, and what it answered once it has.
#[derive(Default)]
struct Flight {
    /// Where its leader has got to.
    landed: std::sync::Mutex<Landed>,
    /// Signalled once, when `landed` is set.
    ready: std::sync::Condvar,
}

/// Where one [`Flight`]'s leader has got to.
#[derive(Default, Clone)]
enum Landed {
    /// Still deriving.
    #[default]
    Deriving,
    /// Answered, and this is the answer.
    Answered(Derived),
    /// Unwound without answering.
    Abandoned,
}

/// The leader's side of a [`Flight`]: removes the flight and wakes every
/// follower when dropped, answered or not, so a panic mid-derivation cannot
/// strand a waiter.
struct Landing<'a> {
    cache: &'a Cache,
    key: Option<(Key, std::time::SystemTime)>,
    flight: std::sync::Arc<Flight>,
    answer: Option<Derived>,
}

impl Drop for Landing<'_> {
    fn drop(&mut self) {
        if let Some(key) = self.key.take() {
            self.cache
                .flights
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&key);
        }
        *self
            .flight
            .landed
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = self
            .answer
            .take()
            .map_or(Landed::Abandoned, Landed::Answered);
        self.flight.ready.notify_all();
    }
}

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
    /// Every file this derivation asked for and could not open, daily or
    /// minute, in the order it asked.
    ///
    /// Beside [`Self::unreadable`] and not folded into it, because the two
    /// answer different questions. `unreadable` names what the calendar lacks,
    /// and an absent minute file is not in it: that month's days are open and
    /// unsized, not missing. This names what the disk did not give, which is
    /// what [`cached`] holds against what the census says the disk holds. A
    /// file the census holds and a derivation could not open is a read that
    /// did not reach the store, and its calendar is not kept. D-0695.
    pub unopened: Vec<Unopened>,
    /// Every month inside the calendar's span whose daily rung was not read
    /// whole, in order, and whose days are therefore `Unmeasured` rather than
    /// `Closed`.
    ///
    /// A month the census holds only at the minute rung, one it does not hold
    /// at all, and one whose daily records failed their checks prove nothing
    /// about which of their days the exchange was shut. Before D-0950 every
    /// such day inside the span was `Closed`, and `/calendar.json` served the
    /// month as a run of holidays. R9-api-law-0, W1-api2-9.
    pub withheld: Vec<YearMonth>,
    /// Bar records this derivation read, one positional read each: every
    /// record of every daily month that opened, plus every record of every
    /// minute month whose counter did not match. Not O(1); the bound is in
    /// `docs/06-limits.md` under D-0950, and the count is pinned by
    /// `api::calendar_of::a_derivation_reads_each_daily_record_once_and_minutes_only_when_walked`.
    /// W1-api2-1.
    pub records_read: u64,
}

/// One file [`derive`] asked for and could not open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unopened {
    /// The rung it asked for: `1day` or `1min`, the two a calendar reads.
    pub rung: Timeframe,
    /// The month it asked for.
    pub month: YearMonth,
    /// The refusal, in `bars::open`'s own words.
    pub reason: String,
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
/// [`Report::unreadable`], never treated as a run of holidays. Every file that
/// did not open, at either rung, is also recorded in [`Report::unopened`].
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
    // EACH TRADED DAY IS FILED UNDER ITS CIVIL MONTH ONCE, AS IT IS FIRST SEEN.
    //
    // The minute pass below asked `in_month` of every traded day in the whole
    // span for every month: once to count the month's sessions and once more
    // to mark them full when the counter matched. That is `O(M × D)` calls to
    // `Day::from_days`, quadratic in the history's length, on every cache miss
    // and once per spot series on the exchange branch (W1-api2-0). Filing the
    // day here costs one classification per distinct day, and each month then
    // reads only its own days. D-0950.
    let mut by_month: BTreeMap<(u16, u8), Vec<i64>> = BTreeMap::new();
    // THE MONTHS WHOSE DAILY RUNG WAS READ WHOLE, and only those may call a day
    // `Closed`. A month the census holds at another rung only, or whose daily
    // records failed their checks, proves nothing about which of its days the
    // exchange was shut, and `Calendar::from_observed` would otherwise fill it
    // with `Closed` because it sits between two observed days. Those months are
    // withheld below. R9-api-law-0, W1-api2-9, D-0950.
    let mut proved: std::collections::BTreeSet<(u16, u8)> = std::collections::BTreeSet::new();
    for month in months {
        // OPENED HERE, AND READ BELOW, so a file that did not open is told
        // apart from one that opened and would not read. Both are named in
        // `unreadable`. Only the first is `unopened`: it is what `cached` holds
        // against the census. D-0695.
        let days = open_rung(
            store_root,
            vendor,
            exchange,
            segment,
            symbol,
            Timeframe::DAY_1,
            *month,
        )
        .inspect_err(|reason| {
            report.unopened.push(Unopened {
                rung: Timeframe::DAY_1,
                month: *month,
                reason: reason.clone(),
            });
        })
        .and_then(|file| {
            report.records_read = report.records_read.saturating_add(file.records());
            read_days(&file, *month)
        });
        match days {
            Ok(days) => {
                proved.insert((month.year(), month.month()));
                for (day, _) in days {
                    if let std::collections::btree_map::Entry::Vacant(slot) = traded.entry(day) {
                        slot.insert(Vec::new());
                        if let Some(civil) = day_month(day) {
                            by_month.entry(civil).or_default().push(day);
                        }
                    }
                }
            }
            Err(why) => report.unreadable.push(why),
        }
    }

    // THE MINUTE RUNG SECOND, AND ONLY WHERE THE COUNTER SAYS IT IS WORTH IT.
    for month in months {
        let own: &[i64] = by_month
            .get(&(month.year(), month.month()))
            .map_or(&[], Vec::as_slice);
        let expected = (own.len() as u64).saturating_mul(FULL);
        // ONE OPEN FOR THE COUNTER AND THE WALK. The walk opened the month a
        // second time, so a file could count and then fail to open for the walk.
        // Its refusal was recorded, but not as a file that did not open.
        match open_rung(
            store_root,
            vendor,
            exchange,
            segment,
            symbol,
            Timeframe::MINUTE_1,
            *month,
        ) {
            Ok(file) if file.records() == expected => {
                // EVERY DAY IN THIS MONTH IS A FULL SESSION, by arithmetic. A
                // walk could find nothing a subtraction has not already proved,
                // so it is not made.
                report.months_by_counter = report.months_by_counter.saturating_add(1);
                for day in own {
                    if let Some(slot) = traded.get_mut(day) {
                        *slot = vec![(pull::calendar::OPEN_MINUTE, pull::calendar::LAST_MINUTE)];
                    }
                }
            }
            Ok(file) => {
                report.months_walked = report.months_walked.saturating_add(1);
                // THE MINUTE SIDE'S REFUSAL IS RECORDED, AND IT WAS THE ONE
                // `Err` IN THIS FUNCTION THAT WAS NOT.
                //
                // Above, the daily read does
                // `Err(why) => report.unreadable.push(why)`, and
                // `Report::unreadable`'s own doc says *"Named, never silent. A
                // month that failed to open is not a month with no sessions."*
                // This arm dropped its error — and `months_walked` had already
                // been incremented, so the report claimed a walk that never
                // happened and every day in the month fell to
                // `OpenLengthUnmeasured` with nothing saying why.
                report.records_read = report.records_read.saturating_add(file.records());
                match read_minute_spans(&file, *month) {
                    Ok(days) => {
                        for (day, runs) in days {
                            // `get_mut`, NEVER `insert`. A minute bar on a day
                            // the daily rung does not know is the store
                            // contradicting itself, and inventing a session for
                            // it is the one thing this module must not do.
                            if let Some(slot) = traded.get_mut(&day) {
                                *slot = runs;
                            }
                        }
                    }
                    Err(why) => report.unreadable.push(why),
                }
            }
            // NO MINUTE FILE AT ALL. Every day in it keeps `None`, which becomes
            // `OpenLengthUnmeasured` — the exchange traded and this build cannot
            // say for how long. Not a hole, and not a holiday. So it is not
            // `unreadable`. It is `unopened`, because whether it should have
            // opened is the census's to say, not this function's.
            Err(reason) => report.unopened.push(Unopened {
                rung: Timeframe::MINUTE_1,
                month: *month,
                reason,
            }),
        }
    }

    let observed: Vec<Observed> = traded
        .into_iter()
        .map(|(day, runs)| Observed::from_runs(day, &runs))
        .collect();
    let mut calendar = Calendar::from_observed(&observed);
    withhold_unproved(&mut calendar, &proved, &mut report);
    (calendar, report)
}

/// Withhold every month inside `calendar`'s span whose daily rung was not read
/// whole, so none of its days reads as `Closed`. R9-api-law-0, W1-api2-9,
/// D-0950.
///
/// `from_observed` fills every unobserved day between the first and last
/// observed day with `Closed`. That is a claim the daily rung makes, and only
/// for a month whose daily file was read: a month held at the minute rung
/// alone, one the census does not hold at all, and one whose daily records
/// failed their checks have no daily reading, and the days in them were served
/// as a run of exchange holidays. They are now `Unmeasured`, the answer the
/// calendar already gives outside its span, and each month withheld is named
/// in [`Report::withheld`].
///
/// Cost: one step per civil month in the span and one slot per day withheld.
/// `proved` is walked in lockstep with the months, both ascending, so each
/// proved month is passed exactly once and no month pays a membership probe.
fn withhold_unproved(
    calendar: &mut Calendar,
    proved: &std::collections::BTreeSet<(u16, u8)>,
    report: &mut Report,
) {
    if calendar.span() == 0 {
        return;
    }
    let last = calendar.last_day();
    let Some(mut month) = u32::try_from(calendar.first_day())
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
        .and_then(|day| pull::session::Day::new(day.year(), day.month(), 1).ok())
    else {
        return;
    };
    let mut ahead = proved.iter().peekable();
    while i64::from(month.days_from_epoch()) <= last {
        let end = month.end_of_month();
        let key = (month.year(), month.month());
        while ahead.next_if(|held| **held < key).is_some() {}
        if ahead.next_if_eq(&&key).is_none() {
            let changed = calendar.withhold_closed(
                i64::from(month.days_from_epoch()),
                i64::from(end.days_from_epoch()),
            );
            if changed > 0
                && let Ok(named) = YearMonth::new(month.year(), month.month())
            {
                report.withheld.push(named);
            }
        }
        let Ok(next) = end.succ() else {
            return;
        };
        month = next;
    }
}

/// The civil year and month of an epoch day, in IST.
///
/// Days are already IST-dated by the readers below, so this is calendar
/// arithmetic with no zone in it.
fn day_month(epoch_day: i64) -> Option<(u16, u8)> {
    #[cfg(test)]
    DAY_MONTH_CALLS.with(|calls| calls.set(calls.get() + 1));
    // `try_from` RATHER THAN `as`, and a pre-epoch day answers `None` rather
    // than wrapping into a plausible date. `Day::from_days` takes a `u32`
    // because the store holds no bar before 1970 and never will.
    let day = pull::session::Day::from_days(u32::try_from(epoch_day).ok()?).ok()?;
    Some((day.year(), day.month()))
}

#[cfg(test)]
thread_local! {
    static DAY_MONTH_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// One rung's file for one month, opened for reading and not yet read.
///
/// **One header read, no records.** `records()` on what this returns is the
/// minute counter that makes the common month free: see the module header.
/// It was two helpers, one of which folded every refusal into "no minute file"
/// with `.ok()`, so a minute file that did not open could not be told from one
/// the store never held.
fn open_rung(
    store_root: &Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    rung: Timeframe,
    month: YearMonth,
) -> Result<store::file::BarFile, String> {
    crate::bars::open(
        store_root, vendor, exchange, segment, symbol, rung, month, None,
    )
}

/// Every distinct IST day a daily file holds a bar for, with its bar count.
fn read_days(file: &store::file::BarFile, month: YearMonth) -> Result<Vec<(i64, u32)>, String> {
    // AN UNREADABLE DAILY BAR IS NOT A HOLIDAY, AND SKIPPING IT MADE ONE.
    //
    // This loop was `if let Ok(bar) = file.read_record(index)`, so a record
    // whose checksum failed never entered `out`, the day never entered
    // `traded`, and this module — whose header declares *"the daily rung IS the
    // calendar. A `1day` bar is proof the exchange traded that day"* — reported
    // an exchange holiday that never happened.
    //
    // Its own header names the hazard one paragraph later: *"deriving 'which
    // days traded' from the rung being validated would make a failed pull read
    // as a holiday."* The circularity it guards against is between RUNGS; this
    // was the same outcome reached by dropping a byte.
    //
    // **The blast radius is every consumer of the derived calendar**, which
    // since D-0365 includes `/gaps.json`'s peer denominator: a corrupt daily
    // record in ANY peer removes that day from what the exchange owed, and a
    // real hole on that day then reads as no loss.
    //
    // `crates/api/src/bars.rs` had the answer two files away — it collects
    // `faults` beside the rows and the caller reports the gap. `Report` already
    // carries `unreadable` for exactly this, with the doc *"Named, never
    // silent. A month that failed to open is not a month with no sessions."*
    // A record that failed to READ is not a day with no session, by the same
    // sentence.
    let mut unreadable = 0_u32;
    let mut out: BTreeMap<i64, u32> = BTreeMap::new();
    let mut index = 0_u64;
    while index < file.records() {
        match file.read_record(index) {
            Ok(bar) => {
                let (day, _) = ist(bar.ts_micros);
                *out.entry(day).or_insert(0) += 1;
            }
            Err(_) => unreadable = unreadable.saturating_add(1),
        }
        index = index.saturating_add(1);
    }
    // REFUSE THE WHOLE MONTH RATHER THAN ANSWER FROM PART OF IT.
    //
    // The caller records this in `Report::unreadable` and treats the month as
    // one it could not read — which is the honest state. Answering with the
    // days that DID decode would hand back a calendar missing exactly the days
    // whose bars are damaged, and that is the holiday this refusal exists to
    // stop being invented.
    if unreadable > 0 {
        return Err(format!(
            "{unreadable} daily record(s) in {month} could not be read, so this \
             month cannot say which days traded. A day whose bar did not decode \
             is not a day the exchange was shut, and answering without it would \
             put a holiday in the calendar that never happened."
        ));
    }
    Ok(out.into_iter().collect())
}

/// The first minute, last minute and count for each day in a minute file.
fn read_minute_spans(file: &store::file::BarFile, month: YearMonth) -> Result<DayRuns, String> {
    // THE CONTIGUOUS RUNS, NOT THE OUTER SPAN.
    //
    // An earlier draft kept only the first and last minute of each day and let
    // the calendar infer one window. Measured against the operator's store it
    // over-counted by exactly 180 bars — the two midday breaks of the
    // disaster-recovery Saturdays — which would have reported NIFTY short by 208
    // against a true 28. The walk is already happening; recording where it
    // BREAKS costs one comparison per bar and removes the error entirely.
    let mut runs: BTreeMap<i64, Vec<(u16, u16)>> = BTreeMap::new();
    // AND HERE A DROPPED RECORD SPLITS A RUN, WHICH IS WORSE THAN LOSING IT.
    //
    // The runs above are what become the day's trading WINDOWS. Skip one
    // unreadable minute in the middle of a session and the run breaks in two,
    // so the calendar reports a session with a hole in it — a scheduled midday
    // break the exchange never took. Every consumer then treats the minutes
    // inside that invented break as `outside-window`, which is not a loss, so a
    // genuine gap on those minutes becomes invisible.
    //
    // That is the same failure the module's 180-bar note above describes,
    // arriving from the opposite direction: there the outer span over-counted a
    // real break, here a byte invents one.
    let mut unreadable = 0_u32;
    let mut index = 0_u64;
    while index < file.records() {
        match file.read_record(index) {
            Ok(bar) => {
                let (day, minute) = ist(bar.ts_micros);
                let day_runs = runs.entry(day).or_default();
                match day_runs.last_mut() {
                    // CONTIGUOUS: extend. Bars arrive in timestamp order, so
                    // the last run is the only one this minute can belong to.
                    Some(last) if last.1.saturating_add(1) == minute => last.1 = minute,
                    Some(last) if last.1 == minute => {}
                    _ => day_runs.push((minute, minute)),
                }
            }
            Err(_) => unreadable = unreadable.saturating_add(1),
        }
        index = index.saturating_add(1);
    }
    if unreadable > 0 {
        return Err(format!(
            "{unreadable} minute record(s) in {month} could not be read, so the \
             session windows this month would report are not the ones it traded. \
             A minute that did not decode splits a run, and a split run is a \
             midday break the exchange never took."
        ));
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
pub(crate) mod tests {
    use super::*;
    use pull::calendar::{DayKind, Session};

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
    ///
    /// **Until 2026-09-23 it did not.** It passed `&[]` as the months, so no
    /// file was opened, no counter was read, and `months_by_counter == 0` was
    /// asserted of a derivation that had nothing to count. The empty case is
    /// kept below — an absent store still has to claim nothing — but the claim
    /// the name makes is now made against a month on disk: two full sessions,
    /// 750 minute bars, a counter that matches, and a report that says the
    /// counter decided and no walk was made.
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

        // THE CLAIM ITSELF. Two sessions the daily rung proves, and exactly
        // 2 x 375 minute bars beside them.
        let month = YearMonth::new(2026, 1).expect("a real month");
        let (monday, tuesday) = (epoch_day(2026, 1, 5), epoch_day(2026, 1, 6));
        write_bars(
            &root,
            Timeframe::DAY_1,
            month,
            &[stamp(monday, OPEN), stamp(tuesday, OPEN)],
        );
        let mut minutes = minutes_of(monday, &[(OPEN, LAST)]);
        minutes.extend(minutes_of(tuesday, &[(OPEN, LAST)]));
        assert_eq!(minutes.len(), 750, "the premise: 2 x 375");
        write_bars(&root, Timeframe::MINUTE_1, month, &minutes);

        let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &[month]);
        assert_eq!(
            (report.months_by_counter, report.months_walked),
            (1, 0),
            "750 held against 2 x 375 owed: the counter decides and no record is walked"
        );
        assert!(
            report.unreadable.is_empty(),
            "both rungs opened: {:?}",
            report.unreadable
        );
        assert_eq!(cal.sessions(), 2);
        for day in [monday, tuesday] {
            assert_eq!(
                cal.kind_of(day),
                DayKind::Open(Session::full()),
                "day {day} is the full 09:15-15:29 session the arithmetic proves"
            );
            assert_eq!(cal.expected_bars(day), Some(375));
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A MONTH WHOSE MINUTE COUNT IS NOT `sessions × 375` IS WALKED, AND THE
    /// WALK RECORDS EACH DAY AS IT TRADED.**
    ///
    /// The other half of the counter test above, and until it existed no test
    /// in this module reached the walk at all: every derivation here either
    /// opened nothing or read a daily rung with no minute file beside it. It is
    /// the half the operator's store takes for 14 of its 81 months, and it is
    /// the only place a short session or an interior hole is measured rather
    /// than assumed.
    ///
    /// January holds four days the daily rung proves — a full session, a
    /// 60-minute one, one with a five-minute hole from 12:00 to 12:04 IST
    /// (minutes 720 to 724, so 370 of 375 held), and one with no minute bars
    /// at all — and 810 minute bars against the 1,500 four full sessions would
    /// owe, so it is walked. Five of those minutes sit on a day with NO daily
    /// bar, which is the store contradicting itself; the walk must not turn
    /// them into a session. February holds one full session, matches its
    /// counter, and is not walked, so one derivation shows the decision being
    /// made per month rather than once per call.
    #[test]
    fn a_month_whose_minutes_are_not_sessions_times_375_is_walked_into_its_runs() {
        let root = crate::scratch::path("calendar-of-walked");
        let _ = std::fs::remove_dir_all(&root);

        let january = YearMonth::new(2026, 1).expect("a real month");
        let february = YearMonth::new(2026, 2).expect("a real month");
        let full = epoch_day(2026, 1, 5);
        let short = epoch_day(2026, 1, 6);
        let stray = epoch_day(2026, 1, 7);
        let holed = epoch_day(2026, 1, 8);
        let no_minutes = epoch_day(2026, 1, 9);
        let whole = epoch_day(2026, 2, 2);

        write_bars(
            &root,
            Timeframe::DAY_1,
            january,
            &[
                stamp(full, OPEN),
                stamp(short, OPEN),
                stamp(holed, OPEN),
                stamp(no_minutes, OPEN),
            ],
        );
        let mut minutes = minutes_of(full, &[(OPEN, LAST)]);
        minutes.extend(minutes_of(short, &[(OPEN, 614)]));
        minutes.extend(minutes_of(stray, &[(OPEN, 559)]));
        minutes.extend(minutes_of(holed, &[(OPEN, 719), (725, LAST)]));
        assert_eq!(
            minutes.len(),
            375 + 60 + 5 + 370,
            "the premise: 810 held, which is not 4 x 375"
        );
        write_bars(&root, Timeframe::MINUTE_1, january, &minutes);

        write_bars(&root, Timeframe::DAY_1, february, &[stamp(whole, OPEN)]);
        write_bars(
            &root,
            Timeframe::MINUTE_1,
            february,
            &minutes_of(whole, &[(OPEN, LAST)]),
        );

        let (cal, report) = derive(
            &root,
            Vendor::Zerodha,
            "NSE",
            "INDEX",
            "NIFTY",
            &[january, february],
        );
        assert_eq!(
            report.months_walked, 1,
            "January is walked: 810 held against 4 x 375 = 1,500 owed"
        );
        assert_eq!(
            report.months_by_counter, 1,
            "February is not: 375 held against 1 x 375"
        );
        assert!(
            report.unreadable.is_empty(),
            "every file opened and every record read: {:?}",
            report.unreadable
        );

        assert_eq!(
            cal.sessions(),
            5,
            "the four January days the daily rung proves and February's one; \
             the stray minutes add none"
        );
        assert_eq!(windows_of(&cal, full), [(OPEN, LAST)]);
        assert_eq!(cal.expected_bars(full), Some(375));
        assert_eq!(
            windows_of(&cal, short),
            [(OPEN, 614)],
            "a short session is measured at its own length, not rounded up to 375"
        );
        assert_eq!(cal.expected_bars(short), Some(60));
        assert_eq!(
            windows_of(&cal, holed),
            [(OPEN, 719), (725, LAST)],
            "an interior hole splits the day into two runs, not one outer span"
        );
        assert_eq!(cal.expected_bars(holed), Some(370));
        assert_eq!(
            cal.kind_of(no_minutes),
            DayKind::OpenLengthUnmeasured,
            "a day the daily rung proves and the minute rung never reached is \
             open and unsized — not closed, and not 375"
        );
        assert_eq!(cal.expected_bars(no_minutes), None);
        assert_eq!(
            cal.kind_of(stray),
            DayKind::Closed,
            "minute bars on a day with no daily bar invent no session"
        );
        assert_eq!(cal.expected_bars(stray), Some(0));
        assert_eq!(
            cal.kind_of(whole),
            DayKind::Open(Session::full()),
            "February's counter decided it"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **ONLY A DERIVATION THAT OPENED EVERY FILE THE CENSUS HOLDS IS KEPT.**
    /// D-0695.
    ///
    /// `cached` is keyed on the manifest's modified time, and a bar file that
    /// does not open moves no manifest, so a calendar derived through one was
    /// served until the next pull. The key is the one the caller hands it,
    /// from the stamps its census was read under, and the first case also
    /// checks that a call under another stamp, or none, derives again rather
    /// than meeting the kept calendar. January's daily file is on disk and
    /// February's is not; neither month has a minute file. Whether each absence
    /// is a fault is the census's to say, so `holds` says it three ways, each
    /// over a cold cache:
    ///
    /// * **January at the daily rung only.** February is held at no rung this
    ///   reads, and January's minutes were never held, so nothing that failed to
    ///   open is held: the calendar is kept, and the second call is the same
    ///   `Arc`. A rule that declined every derivation naming a month
    ///   `unreadable` fails here, and would re-derive such a store on every
    ///   request.
    /// * **Both months at the daily rung.** February's daily file is held and
    ///   did not open: its refusal is named, the calendar still carries
    ///   January's session, and nothing is kept, so the second call derives
    ///   again.
    /// * **January at the minute rung too.** Its minute file is held and did
    ///   not open, which the derivation used to fold into "no minute file" and
    ///   keep.
    #[test]
    fn a_derivation_is_kept_only_when_every_file_it_could_not_open_is_unheld() {
        use std::sync::Arc;
        /// What `cached` asks of each file that did not open.
        type Holds<'a> = &'a dyn Fn(Timeframe, YearMonth) -> bool;
        let root = crate::scratch::path("calendar-of-kept");
        let _ = std::fs::remove_dir_all(&root);
        let january = YearMonth::new(2026, 1).expect("a real month");
        let february = YearMonth::new(2026, 2).expect("a real month");
        let monday = epoch_day(2026, 1, 5);
        write_bars(&root, Timeframe::DAY_1, january, &[stamp(monday, OPEN)]);
        // THE STAMP `cached` KEYS ON is the caller's, handed in: no manifest is
        // on disk for it to find, and it must not look for one.
        let stamped = Some(std::time::SystemTime::UNIX_EPOCH);
        let months = [january, february];
        let call = |cache: &Cache, holds: Holds<'_>| {
            cached(
                cache,
                &root,
                Vendor::Zerodha,
                "NSE",
                "INDEX",
                "NIFTY",
                stamped,
                &months,
                holds,
            )
        };

        let cache = Cache::default();
        let january_daily = |rung, month| rung == Timeframe::DAY_1 && month == january;
        let first = call(&cache, &january_daily);
        assert!(first.unopened.is_empty(), "{:?}", first.unopened);
        assert_eq!(first.calendar.sessions(), 1);
        assert_eq!(cache.lock().map_or(0, |held| held.len()), 1, "kept");
        let again = call(&cache, &january_daily);
        assert!(
            Arc::ptr_eq(&first.calendar, &again.calendar),
            "the second call is the kept calendar"
        );
        assert!(again.unopened.is_empty(), "a hit names nothing");
        // KEPT UNDER THE STAMP IT WAS HANDED, AND ONLY THAT ONE: a caller whose
        // census was read under another stamp derives again, and so does every
        // caller with no stamp, whose derivation is not kept.
        let under = |other| {
            cached(
                &cache,
                &root,
                Vendor::Zerodha,
                "NSE",
                "INDEX",
                "NIFTY",
                other,
                &months,
                january_daily,
            )
            .calendar
        };
        let later = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1);
        assert!(
            !Arc::ptr_eq(&first.calendar, &under(Some(later))),
            "another stamp derives again"
        );
        let unstamped = under(None);
        assert!(
            !Arc::ptr_eq(&unstamped, &under(None)),
            "no stamp derives again every call"
        );

        let cases: [(&str, Holds<'_>, &str); 2] = [
            (
                "both months at the daily rung",
                &|rung, _| rung == Timeframe::DAY_1,
                "2026-02",
            ),
            (
                "January at the minute rung too",
                &|_, month| month == january,
                "2026-01",
            ),
        ];
        for (what, holds, named) in cases {
            let cache = Cache::default();
            let first = call(&cache, holds);
            assert_eq!(first.unopened.len(), 1, "{what}: {:?}", first.unopened);
            assert!(
                first.unopened.iter().all(|why| why.contains(named)),
                "{what}: the held file is named: {:?}",
                first.unopened
            );
            assert_eq!(
                first.calendar.sessions(),
                1,
                "{what}: January's session is still derived"
            );
            assert_eq!(
                cache.lock().map_or(1, |held| held.len()),
                0,
                "{what}: not kept"
            );
            let again = call(&cache, holds);
            assert!(
                !Arc::ptr_eq(&first.calendar, &again.calendar),
                "{what}: the second call derives again"
            );
            assert_eq!(again.unopened, first.unopened, "{what}: and meets it again");
        }

        let _ = std::fs::remove_dir_all(&root);
    }

    /// **`holds` IS ASKED ONCE PER FILE THAT DID NOT OPEN, AND NOT AT ALL ON A
    /// HIT OR FOR A DERIVATION THAT OPENED EVERY FILE.** D-0695.
    ///
    /// AF-28e states what `cached` costs its caller's census: one question per
    /// file the derivation could not open. The test above counts nothing, so
    /// asking for every file asked for, or twice for each, passed it. Here
    /// January's daily file is the only one on disk, over January and February:
    /// four files are asked for and three do not open, February's daily file
    /// and both minute files. `holds` says none of them is held, so the
    /// calendar is kept, and it must have been asked about exactly those three,
    /// once each, in the order `derive` asked for them. The second call is a
    /// hit and asks nothing. Then, with January's minute file written too, a
    /// cold call over January alone opens every file it asks for, and asks
    /// nothing either.
    #[test]
    fn holds_is_asked_once_per_file_that_did_not_open_and_never_on_a_hit() {
        use std::sync::Arc;
        let root = crate::scratch::path("calendar-of-holds-asked");
        let _ = std::fs::remove_dir_all(&root);
        let january = YearMonth::new(2026, 1).expect("a real month");
        let february = YearMonth::new(2026, 2).expect("a real month");
        let monday = epoch_day(2026, 1, 5);
        write_bars(&root, Timeframe::DAY_1, january, &[stamp(monday, OPEN)]);
        let asked = std::cell::RefCell::new(Vec::new());
        let holds = |rung, month| {
            asked.borrow_mut().push((rung, month));
            false
        };
        let call = |cache: &Cache, months: &[YearMonth]| {
            cached(
                cache,
                &root,
                Vendor::Zerodha,
                "NSE",
                "INDEX",
                "NIFTY",
                Some(std::time::SystemTime::UNIX_EPOCH),
                months,
                holds,
            )
        };

        let cache = Cache::default();
        let first = call(&cache, &[january, february]);
        assert!(first.unopened.is_empty(), "{:?}", first.unopened);
        assert_eq!(
            asked.take(),
            vec![
                (Timeframe::DAY_1, february),
                (Timeframe::MINUTE_1, january),
                (Timeframe::MINUTE_1, february),
            ],
            "once per file that did not open, and never for January's daily file"
        );
        let again = call(&cache, &[january, february]);
        assert!(
            Arc::ptr_eq(&first.calendar, &again.calendar),
            "the second call is the kept calendar"
        );
        assert_eq!(asked.take(), Vec::new(), "a hit asks nothing");

        write_bars(&root, Timeframe::MINUTE_1, january, &[stamp(monday, OPEN)]);
        let opened = call(&Cache::default(), &[january]);
        assert!(opened.unopened.is_empty(), "{:?}", opened.unopened);
        assert_eq!(opened.calendar.sessions(), 1, "January's session");
        assert_eq!(
            asked.take(),
            Vec::new(),
            "a derivation that opened every file asks nothing"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// 09:15 IST, the first minute of a session.
    pub(crate) const OPEN: u16 = pull::calendar::OPEN_MINUTE;
    /// 15:29 IST, the last.
    const LAST: u16 = pull::calendar::LAST_MINUTE;

    /// Days since the epoch of a civil date, by the `Day` the store's months
    /// are named from.
    pub(crate) fn epoch_day(year: u16, month: u8, day: u8) -> i64 {
        i64::from(
            pull::session::Day::new(year, month, day)
                .expect("a real date")
                .days_from_epoch(),
        )
    }

    /// The UTC micros stamp of minute-of-day `minute` on IST day `day` — the
    /// inverse of [`ist`], which is the reading `derive` applies.
    pub(crate) fn stamp(day: i64, minute: u16) -> i64 {
        const IST_OFFSET_SECS: i64 = 5 * 3600 + 30 * 60;
        (day * 86_400 + i64::from(minute) * 60 - IST_OFFSET_SECS) * 1_000_000
    }

    /// One stamp per minute of every inclusive run, in order.
    fn minutes_of(day: i64, runs: &[(u16, u16)]) -> Vec<i64> {
        runs.iter()
            .flat_map(|&(from, to)| (from..=to).map(move |minute| stamp(day, minute)))
            .collect()
    }

    /// The windows an open day was given, as `(from, to)` pairs.
    fn windows_of(cal: &Calendar, day: i64) -> Vec<(u16, u16)> {
        match cal.kind_of(day) {
            DayKind::Open(session) => session
                .windows
                .iter()
                .take(usize::from(session.count))
                .map(|window| (window.from, window.to))
                .collect(),
            other => panic!("day {day} is {other:?}, not an open session"),
        }
    }

    /// Appends one legal bar per stamp to `NSE/INDEX/NIFTY`'s `rung` file for
    /// `month`, the way `segments::write_day` writes its one daily bar.
    fn write_bars(root: &Path, rung: Timeframe, month: YearMonth, stamps: &[i64]) {
        write_bars_for(root, "NIFTY", rung, month, stamps);
    }

    /// [`write_bars`], for `NSE/INDEX/<symbol>`.
    pub(crate) fn write_bars_for(
        root: &Path,
        symbol: &str,
        rung: Timeframe,
        month: YearMonth,
        stamps: &[i64],
    ) {
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol,
            contract: None,
            timeframe: rung,
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
        let rows: Vec<store::format::Bar> = stamps
            .iter()
            .map(|&ts_micros| store::format::Bar {
                ts_micros,
                open: 100,
                high: 110,
                low: 90,
                close: 105,
                volume: 1,
                open_interest: i64::MIN,
            })
            .collect();
        file.append(&rows).expect("legal bars in timestamp order");
    }

    /// Flips one byte inside record `index` of `NSE/INDEX/NIFTY`'s `rung` file
    /// for `month`, so its checksum block no longer verifies.
    fn corrupt_record(root: &Path, rung: Timeframe, month: YearMonth, index: u64) {
        use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
        let path = store::path::StorePath::new(store::path::PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: rung,
            month,
            file: store::path::FileKind::Bars,
        })
        .expect("a legal path")
        .to_path_buf(root);
        let at = store::format::HEADER_LEN + index * store::format::RECORD_STRIDE + 9;
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("the bar file exists");
        let mut byte = [0_u8; 1];
        file.seek(SeekFrom::Start(at)).expect("seek");
        file.read_exact(&mut byte).expect("read");
        byte[0] ^= 0x5A;
        file.seek(SeekFrom::Start(at)).expect("seek");
        file.write_all(&byte).expect("write");
    }

    /// Every day of `month` as epoch days, first to last.
    fn days_of(month: YearMonth) -> std::ops::RangeInclusive<i64> {
        let first = pull::session::Day::new(month.year(), month.month(), 1).expect("a real month");
        i64::from(first.days_from_epoch())..=i64::from(first.end_of_month().days_from_epoch())
    }

    /// **A MONTH THE DAILY RUNG DID NOT PROVE IS WITHHELD, NOT A RUN OF
    /// HOLIDAYS.** R9-api-law-0, D-0950.
    ///
    /// January and March hold daily bars; February is held only at the minute
    /// rung, with a full session on 2 February. Before D-0950 every February
    /// day came out `Closed` and `/calendar.json` answered 200 with the month
    /// simply missing, while January's genuine closure (7 January, no daily bar
    /// in a month whose daily file was read) is still `Closed`.
    #[test]
    fn a_month_held_only_at_the_minute_rung_is_withheld_not_closed() {
        let root = crate::scratch::path("calendar-of-minute-only-month");
        let _ = std::fs::remove_dir_all(&root);
        let (january, february, march) = (
            YearMonth::new(2026, 1).expect("a real month"),
            YearMonth::new(2026, 2).expect("a real month"),
            YearMonth::new(2026, 3).expect("a real month"),
        );
        let (jan6, jan8, feb2, mar2) = (
            epoch_day(2026, 1, 6),
            epoch_day(2026, 1, 8),
            epoch_day(2026, 2, 2),
            epoch_day(2026, 3, 2),
        );
        write_bars(
            &root,
            Timeframe::DAY_1,
            january,
            &[stamp(jan6, OPEN), stamp(jan8, OPEN)],
        );
        write_bars(
            &root,
            Timeframe::MINUTE_1,
            february,
            &minutes_of(feb2, &[(OPEN, LAST)]),
        );
        write_bars(&root, Timeframe::DAY_1, march, &[stamp(mar2, OPEN)]);

        let (cal, report) = derive(
            &root,
            Vendor::Zerodha,
            "NSE",
            "INDEX",
            "NIFTY",
            &[january, february, march],
        );
        assert_eq!((cal.first_day(), cal.last_day()), (jan6, mar2));
        assert_eq!(
            cal.kind_of(epoch_day(2026, 1, 7)),
            DayKind::Closed,
            "January was read"
        );
        for day in days_of(february) {
            assert_eq!(
                cal.kind_of(day),
                DayKind::Unmeasured,
                "Feb day {day} is not proved shut"
            );
            assert_eq!(
                cal.expected_bars(day),
                None,
                "Feb day {day} owes an unknown count"
            );
        }
        assert_eq!(report.withheld, vec![february]);
        assert_eq!(cal.sessions(), 3);
        let wire = json(&cal);
        let (from, to) = (
            days_of(february).start().to_owned(),
            days_of(february).end().to_owned(),
        );
        assert!(
            wire.contains(&format!("\"withheld\":[{{\"from\":{from},\"to\":{to}}}]")),
            "the wire names the withheld stretch: {wire}"
        );

        // THROUGH THE CACHE TOO: kept (nothing held failed to open), and kept
        // honest.
        let cache = Cache::default();
        let derived = cached(
            &cache,
            &root,
            Vendor::Zerodha,
            "NSE",
            "INDEX",
            "NIFTY",
            Some(std::time::SystemTime::UNIX_EPOCH),
            &[january, february, march],
            // The census as written above: daily January and March, minute
            // February, and nothing else.
            |rung, month| (rung == Timeframe::DAY_1) == (month != february),
        );
        assert!(derived.unopened.is_empty(), "{:?}", derived.unopened);
        assert!(
            cache.lock().expect("unpoisoned").contains_key(&(
                Vendor::Zerodha,
                "NSE".to_owned(),
                "INDEX".to_owned(),
                "NIFTY".to_owned()
            )),
            "kept under the stamp"
        );
        assert_eq!(derived.calendar.kind_of(feb2), DayKind::Unmeasured);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **A DAILY MONTH WHOSE RECORDS FAIL THEIR CHECKS IS WITHHELD, NOT SERVED
    /// AS HOLIDAYS.** W1-api2-9, D-0950.
    ///
    /// Three cases over calendar edges: a corrupt February in a leap year
    /// (2024, so 29 days withheld), a corrupt December across a year boundary,
    /// and a corrupt month at the very start of the span, which lies outside
    /// the span and claims nothing either way.
    #[test]
    fn a_corrupt_daily_month_is_withheld_not_served_as_holidays() {
        type Month = (u16, u8);
        let cases: [(Month, Month, Month, u8); 3] = [
            ((2024, 1), (2024, 2), (2024, 3), 29),
            ((2025, 11), (2025, 12), (2026, 1), 31),
            ((2026, 1), (2026, 2), (2026, 3), 0),
        ];
        for (n, (before, damaged, after, withheld_days)) in cases.into_iter().enumerate() {
            let root = crate::scratch::path(&format!("calendar-of-corrupt-daily-{n}"));
            let _ = std::fs::remove_dir_all(&root);
            let month = |(y, m): (u16, u8)| YearMonth::new(y, m).expect("a real month");
            let (before, damaged, after) = (month(before), month(damaged), month(after));
            let day_in = |month: YearMonth, day: u8| epoch_day(month.year(), month.month(), day);
            let edge = withheld_days == 0;
            if !edge {
                write_bars(
                    &root,
                    Timeframe::DAY_1,
                    before,
                    &[stamp(day_in(before, 10), OPEN)],
                );
            }
            write_bars(
                &root,
                Timeframe::DAY_1,
                damaged,
                &[
                    stamp(day_in(damaged, 2), OPEN),
                    stamp(day_in(damaged, 3), OPEN),
                ],
            );
            corrupt_record(&root, Timeframe::DAY_1, damaged, 0);
            write_bars(
                &root,
                Timeframe::DAY_1,
                after,
                &[stamp(day_in(after, 5), OPEN)],
            );
            let months: Vec<YearMonth> = if edge {
                vec![damaged, after]
            } else {
                vec![before, damaged, after]
            };

            let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);
            assert_eq!(
                report.unreadable.len(),
                1,
                "case {n}: named: {:?}",
                report.unreadable
            );
            let named = report.unreadable.first().cloned().unwrap_or_default();
            assert!(named.contains(&damaged.to_string()), "case {n}: {named}");
            let unmeasured = days_of(damaged)
                .filter(|day| cal.kind_of(*day) == DayKind::Unmeasured)
                .count();
            assert_eq!(
                unmeasured,
                days_of(damaged).count(),
                "case {n}: not one day of the damaged month is claimed shut"
            );
            let closed = days_of(damaged)
                .filter(|day| cal.kind_of(*day) == DayKind::Closed)
                .count();
            assert_eq!(closed, 0, "case {n}");
            if edge {
                assert_eq!(report.withheld, Vec::<YearMonth>::new(), "outside the span");
                assert!(json(&cal).contains("\"withheld\":[]"), "case {n}");
            } else {
                assert_eq!(report.withheld, vec![damaged], "case {n}");
                let wire = json(&cal);
                let (from, to) = (*days_of(damaged).start(), *days_of(damaged).end());
                assert_eq!(
                    usize::try_from(to - from + 1).ok(),
                    Some(usize::from(withheld_days)),
                    "case {n}: the calendar edge under test"
                );
                assert!(
                    wire.contains(&format!("{{\"from\":{from},\"to\":{to}}}")),
                    "case {n}: {wire}"
                );
                // The neighbouring months were read, so their gaps stay closed.
                assert_eq!(
                    cal.kind_of(day_in(before, 10) + 1),
                    DayKind::Closed,
                    "case {n}"
                );
            }
            // RERUN: same inputs, same answer, byte for byte.
            let (again, _) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);
            assert_eq!(json(&again), json(&cal), "case {n}: idempotent");
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    /// **ONE CIVIL-MONTH CLASSIFICATION PER TRADED DAY, NOT ONE PER MONTH PER
    /// DAY.** W1-api2-0, D-0950.
    ///
    /// `derive` asked `in_month` of every traded day for every month, twice on
    /// a month whose counter matched, so a span of M months and D days cost
    /// `O(M × D)` calls to `Day::from_days`. Counted here, deterministically:
    /// 24 months, two daily bars each, no minute files (the counter cannot
    /// match, so only the classification is at stake), then the same with
    /// counter-matching minute files on the first two months.
    #[test]
    fn derive_classifies_each_traded_day_once_not_once_per_month() {
        let root = crate::scratch::path("calendar-of-classify-once");
        let _ = std::fs::remove_dir_all(&root);
        let mut months = Vec::new();
        let mut days = Vec::new();
        for offset in 0_u8..24 {
            let year = 2024 + u16::from(offset / 12);
            let month = YearMonth::new(year, offset % 12 + 1).expect("a real month");
            let pair = [
                epoch_day(year, offset % 12 + 1, 10),
                epoch_day(year, offset % 12 + 1, 11),
            ];
            write_bars(
                &root,
                Timeframe::DAY_1,
                month,
                &[stamp(pair[0], OPEN), stamp(pair[1], OPEN)],
            );
            months.push(month);
            days.extend(pair);
        }
        DAY_MONTH_CALLS.with(|calls| calls.set(0));
        let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);
        let calls = DAY_MONTH_CALLS.with(std::cell::Cell::get);
        assert_eq!(cal.sessions(), 48);
        assert_eq!(report.months_by_counter, 0);
        assert!(
            calls <= 48,
            "{calls} civil-month classifications for 48 traded days over 24 months; \
             one per day is the bound, {} is the quadratic shape",
            24 * 48
        );

        for month in months.iter().take(2) {
            let mut minutes = Vec::new();
            for day in days_of(*month) {
                if cal.kind_of(day) == DayKind::OpenLengthUnmeasured {
                    minutes.extend(minutes_of(day, &[(OPEN, LAST)]));
                }
            }
            write_bars(&root, Timeframe::MINUTE_1, *month, &minutes);
        }
        DAY_MONTH_CALLS.with(|calls| calls.set(0));
        let (cal, report) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &months);
        let calls = DAY_MONTH_CALLS.with(std::cell::Cell::get);
        assert_eq!(report.months_by_counter, 2);
        assert_eq!(
            cal.expected_bars(*days.first().expect("five days")),
            Some(375)
        );
        assert_eq!(
            cal.expected_bars(*days.get(3).expect("five days")),
            Some(375)
        );
        assert_eq!(
            cal.expected_bars(*days.get(4).expect("five days")),
            None,
            "March has no minute file"
        );
        assert!(
            calls <= 48,
            "{calls} classifications with two counter-matched months"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **WHAT A DERIVATION READS IS COUNTED, RECORD BY RECORD.** W1-api2-1,
    /// D-0950.
    ///
    /// The reads are one positional read per record and are not O(1): the
    /// bound is the records of every daily month plus the records of every
    /// minute month whose counter did not match, stated in
    /// `docs/06-limits.md`. Pinned here exactly, so a change that walks a
    /// counter-matched month, or reads a record twice, fails: this test is
    /// `api::calendar_of::a_derivation_reads_each_daily_record_once_and_minutes_only_when_walked`.
    #[test]
    fn a_derivation_reads_each_daily_record_once_and_minutes_only_when_walked() {
        let root = crate::scratch::path("calendar-of-records-read");
        let _ = std::fs::remove_dir_all(&root);
        let (january, february) = (
            YearMonth::new(2026, 1).expect("a real month"),
            YearMonth::new(2026, 2).expect("a real month"),
        );
        let (jan5, feb2, feb3) = (
            epoch_day(2026, 1, 5),
            epoch_day(2026, 2, 2),
            epoch_day(2026, 2, 3),
        );
        write_bars(&root, Timeframe::DAY_1, january, &[stamp(jan5, OPEN)]);
        write_bars(
            &root,
            Timeframe::MINUTE_1,
            january,
            &minutes_of(jan5, &[(OPEN, LAST)]),
        );
        write_bars(
            &root,
            Timeframe::DAY_1,
            february,
            &[stamp(feb2, OPEN), stamp(feb3, OPEN)],
        );
        // February is short by one minute, so it is walked: 749 records.
        write_bars(&root, Timeframe::MINUTE_1, february, &{
            let mut m = minutes_of(feb2, &[(OPEN, LAST)]);
            m.extend(minutes_of(feb3, &[(OPEN, LAST - 1)]));
            m
        });

        let (_, report) = derive(
            &root,
            Vendor::Zerodha,
            "NSE",
            "INDEX",
            "NIFTY",
            &[january, february],
        );
        assert_eq!((report.months_by_counter, report.months_walked), (1, 1));
        assert_eq!(
            report.records_read,
            1 + 2 + 749,
            "daily 1 + 2, walked minutes 749"
        );

        // Empty: nothing asked, nothing read.
        let (_, empty) = derive(&root, Vendor::Zerodha, "NSE", "INDEX", "NIFTY", &[]);
        assert_eq!(empty.records_read, 0);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE LIMIT IS WRITTEN WHERE `CLAUDE.md` §10 SAYS LIMITS LIVE.**
    /// W1-api2-1, D-0950: the per-record walk was documented only in this
    /// module's header.
    #[test]
    fn the_derivation_walk_is_named_in_the_limits_document() {
        let limits = include_str!("../../../docs/06-limits.md");
        let section = limits
            .split("## The calendar derivation reads records, one positional read each — D-0950")
            .nth(1)
            .expect("docs/06-limits.md carries the D-0950 section");
        for named in [
            "read_days",
            "read_minute_spans",
            "records_read",
            "O(records)",
        ] {
            assert!(section.contains(named), "the D-0950 limit names `{named}`");
        }
    }

    /// The series every single-flight test asks for.
    fn nifty_key() -> Key {
        (
            Vendor::Zerodha,
            "NSE".to_owned(),
            "INDEX".to_owned(),
            "NIFTY".to_owned(),
        )
    }

    /// A one-session calendar, as a stand-in derivation's answer.
    fn one_session() -> (Calendar, Report) {
        (
            Calendar::from_observed(&[Observed::from_runs(100, &[(OPEN, LAST)])]),
            Report::default(),
        )
    }

    /// Holds a derivation open until `followers` requests wait on it, bounded
    /// so a broken single-flight fails its count rather than hanging.
    fn until_joined(cache: &Cache, followers: usize) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while cache.waiting() < followers && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    /// **CONCURRENT MISSES ON ONE KEY DERIVE ONCE AND SHARE THE ANSWER.**
    /// W1-api2-11, D-0950.
    ///
    /// Eight requests miss the same series at the same stamp. The derivation
    /// is held open until the other seven are waiting on it, so the count is
    /// decided by the code and not by a clock: one derivation, eight answers
    /// sharing one `Arc`, nobody left waiting, no flight left behind, and a
    /// ninth request afterwards is a hit that derives nothing.
    #[test]
    fn concurrent_misses_on_one_key_derive_once_and_share_the_answer() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        const FOLLOWERS: usize = 7;
        let cache = Cache::default();
        let runs = AtomicUsize::new(0);
        let now = Some(std::time::SystemTime::UNIX_EPOCH);
        let derive = || {
            runs.fetch_add(1, Ordering::SeqCst);
            until_joined(&cache, FOLLOWERS);
            one_session()
        };
        let answers: Vec<Derived> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..=FOLLOWERS)
                .map(|_| scope.spawn(|| cached_by(&cache, &nifty_key(), now, derive, |_, _| false)))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("no request panicked"))
                .collect()
        });
        assert_eq!(
            runs.load(Ordering::SeqCst),
            1,
            "eight concurrent misses on one key and stamp are one derivation"
        );
        let first = answers.first().expect("eight answers");
        assert!(
            answers
                .iter()
                .all(|answer| std::sync::Arc::ptr_eq(&answer.calendar, &first.calendar)),
            "every request is answered the one derivation"
        );
        assert_eq!(cache.waiting(), 0, "nobody left waiting");
        assert!(
            cache.flights.lock().is_ok_and(|flights| flights.is_empty()),
            "no flight left behind"
        );
        let hit = cached_by(&cache, &nifty_key(), now, derive, |_, _| false);
        assert!(std::sync::Arc::ptr_eq(&hit.calendar, &first.calendar));
        assert_eq!(runs.load(Ordering::SeqCst), 1, "a hit derives nothing");

        // ANOTHER STAMP IS ANOTHER FLIGHT: it derives, once.
        let later = Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1));
        let moved = cached_by(
            &cache,
            &nifty_key(),
            later,
            one_session_counted(&runs),
            |_, _| false,
        );
        assert!(!std::sync::Arc::ptr_eq(&moved.calendar, &first.calendar));
        assert_eq!(runs.load(Ordering::SeqCst), 2);
    }

    /// [`one_session`], counting each call on `runs`.
    fn one_session_counted(
        runs: &std::sync::atomic::AtomicUsize,
    ) -> impl Fn() -> (Calendar, Report) + '_ {
        move || {
            runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            one_session()
        }
    }

    /// **A LEADER THAT UNWINDS STRANDS NO FOLLOWER AND KEEPS NOTHING.**
    /// W1-api2-11, D-0950.
    ///
    /// Alone: the panic reaches its own caller, no flight and no calendar are
    /// left, and the next request derives normally. With three followers
    /// waiting on it: each is woken, one leads a fresh derivation, and all
    /// three are answered by it, so the count is the failed derivation plus
    /// exactly one.
    #[test]
    fn a_leader_that_unwinds_strands_no_follower_and_keeps_nothing() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        const FOLLOWERS: usize = 3;
        let now = Some(std::time::SystemTime::UNIX_EPOCH);

        let cache = Cache::default();
        let died = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cached_by(
                &cache,
                &nifty_key(),
                now,
                || -> (Calendar, Report) { panic!("the derivation died") },
                |_, _| false,
            )
        }));
        assert!(died.is_err(), "the panic reaches the request that led");
        assert!(cache.flights.lock().is_ok_and(|flights| flights.is_empty()));
        assert_eq!(cache.lock().map_or(1, |held| held.len()), 0, "nothing kept");
        let runs = AtomicUsize::new(0);
        let after = cached_by(
            &cache,
            &nifty_key(),
            now,
            one_session_counted(&runs),
            |_, _| false,
        );
        assert_eq!(after.calendar.sessions(), 1);
        assert_eq!(runs.load(Ordering::SeqCst), 1);

        let cache = Cache::default();
        let runs = AtomicUsize::new(0);
        let derive = || {
            if runs.fetch_add(1, Ordering::SeqCst) == 0 {
                until_joined(&cache, FOLLOWERS);
                panic!("the first derivation died with three requests waiting on it");
            }
            one_session()
        };
        let outcomes: Vec<Option<Derived>> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..=FOLLOWERS)
                .map(|_| scope.spawn(|| cached_by(&cache, &nifty_key(), now, derive, |_, _| false)))
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().ok())
                .collect()
        });
        let answered: Vec<&Derived> = outcomes.iter().flatten().collect();
        assert_eq!(
            outcomes.len() - answered.len(),
            1,
            "only the leader's request failed"
        );
        assert_eq!(answered.len(), FOLLOWERS);
        assert!(
            answered
                .iter()
                .all(|answer| answer.calendar.sessions() == 1)
        );
        assert_eq!(
            runs.load(Ordering::SeqCst),
            2,
            "the derivation that died, and exactly one that answered the followers"
        );
        assert_eq!(cache.waiting(), 0);
        assert!(cache.flights.lock().is_ok_and(|flights| flights.is_empty()));
    }

    /// **NO STAMP, NO FLIGHT.** A derivation that is never kept is never
    /// shared either: each unstamped request derives for itself, and leaves
    /// no flight behind.
    #[test]
    fn an_unstamped_request_derives_for_itself_and_registers_no_flight() {
        let cache = Cache::default();
        let runs = std::sync::atomic::AtomicUsize::new(0);
        for _ in 0..3 {
            let _ = cached_by(
                &cache,
                &nifty_key(),
                None,
                one_session_counted(&runs),
                |_, _| false,
            );
        }
        assert_eq!(runs.load(std::sync::atomic::Ordering::SeqCst), 3);
        assert!(cache.flights.lock().is_ok_and(|flights| flights.is_empty()));
        assert_eq!(cache.lock().map_or(1, |held| held.len()), 0);
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
    /// What it pins: **1,671** trading days, **623,712** exchange-timetable
    /// minute slots derivable from one instrument after the SEBI outage
    /// override, and the five pre-2025 Muhurat sessions answering
    /// `OpenLengthUnmeasured` because the minute rung never reached them. This
    /// is not an index completeness verdict; `gaps` withholds the outage date.
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
            owed, 623_712,
            "one instrument's contiguous runs plus the fixed 166-slot SEBI \
             exchange-session correction. This is not a common index-bar \
             denominator; see this module header and D-0420."
        );
    }
}

/// A calendar [`cached`] answered, and every file the caller's census holds that
/// the derivation behind it could not open.
#[derive(Debug, Clone)]
pub struct Derived {
    /// The calendar, shared with the cache when it was kept there.
    pub calendar: std::sync::Arc<Calendar>,
    /// The refusal of each file `holds` said the store holds and this request's
    /// derivation could not open, in the order [`derive`] asked for them.
    ///
    /// Empty on a cache hit and on every derivation that was kept, which are the
    /// same set: a derivation is kept only when this is empty. When it is not,
    /// [`Self::calendar`] lacks what those files would have proved, and a caller
    /// must not answer it as the store's calendar.
    pub unopened: Vec<String>,
}

/// The calendar for one instrument, derived once per store change.
///
/// # Why this is not just [`derive`]
///
/// `derive` measured **0.28 s** for one instrument across 81 months — fine once
/// after a pull, and not fine on a page that polls every two seconds. This
/// returns the cached answer when the vendor's manifest has not been rewritten
/// since it was built, which is one map probe.
///
/// # The key is the caller's, taken before its census was read
///
/// `stamp` is the modified time of the vendor's manifest from the stamps the
/// caller's census was read under: `server::census_now_stamped`, whose stamps
/// are taken BEFORE its read on a miss. `months` and `holds` come from that
/// census, so the three share one moment, and a calendar is never kept under a
/// time newer than the months it was derived from.
///
/// This function took the key itself, one `stat` of the manifest after the
/// census had been read. A pull that installed a manifest between the two had
/// the calendar derived from the OLDER census's months kept under the NEWER
/// modified time, and every later request, reading the newer census, hit it
/// until the manifest was next written. A review measured it: a June bar and a
/// manifest naming May and June installed between the census and this call,
/// then three `/calendar.json` requests that each answered one session where a
/// cold derivation answered two. Taken before the census, the key is older
/// than what it keys: the next request's stamp differs, misses, and derives
/// again, which costs one derivation. It is the order D-0695 gave the census
/// cache for the same reason. The `stat` this took per call is gone too, and
/// on the exchange branch of `/calendar.json` that was one per series.
///
/// That alone did not make every kept calendar current. The stamps come with
/// every census, including one whose read contradicts them, which the census
/// cache refuses to keep: a store root gone after `read_all`'s own check reads
/// the feed "absent" under a stamp of its manifest. The symbol branch of
/// `/calendar.json` derived over no months from such a read and this kept the
/// empty calendar under that unmoved time, so every caller was served it after
/// the root came back, until the manifest was next written. The caller's
/// stamp is therefore `None` for a census row the stamp could not have read
/// (`server::CensusStamps::modified`), and a derivation from it is answered to
/// its request and not kept. D-0695.
///
/// **A store that has never been written has no manifest and therefore no
/// stamp**, and neither does one whose `stat` failed: `stamp` is `None`. That
/// case re-derives every call, which is correct rather than unfortunate: an
/// empty store derives an empty calendar in microseconds, and caching "I found
/// nothing" against a key that cannot change would answer `Unmeasured` for
/// ever once the first pull landed.
///
/// # Only a derivation that opened every file the census holds is kept
///
/// The key is the manifest's modified time, and the manifest is what says which
/// bar files the store holds. A derivation that could not open one of them did
/// not read the store its key names: a bar directory moved aside for one
/// request, a store root gone after the census was read, an I/O error on open.
/// None of these moves the manifest, so the calendar derived through one --
/// empty, or short of the months that failed -- was cached under the same key
/// and served until the next pull wrote the manifest, long after the fault
/// ended. A review measured it: with `bars/dhan` moved aside for one request
/// and put back, `/calendar.json` still answered `{"sessions":0}` while `/bars`
/// served the month. The census cache in front of this one had been given the
/// same rule by D-0695; this cache had not.
///
/// So `holds` answers, for a rung and a month, whether the caller's census
/// holds that file of this series, and a derivation is kept only when every
/// file it could not open is one `holds` says the store does not hold. A month
/// held at the minute rung and not the daily one opens no daily file and is
/// still kept, because the census says so, and so is a minute file the store
/// never held. Kept, and since D-0950 no longer kept as holidays: such a
/// month's days are withheld (`Unmeasured`, named in `withheld` on the wire),
/// because a daily rung that was not read proves no day shut. R9-api-law-0. A held file whose records fail their checks did open, and is
/// kept too, as the census cache keeps a manifest whose bytes do not decode:
/// damaged bytes do not heal between two requests, and a pull that rewrites
/// them moves the manifest. Anything else answers the request that derived it,
/// with [`Derived::unopened`] naming each held file that did not open, and is
/// derived again on the next request. `holds` is asked only about files that
/// did not open, so a derivation that opened everything costs no probe. D-0695.
#[expect(
    clippy::too_many_arguments,
    reason = "the cache, the root and the four terms of the series' address that \
              `bars::open` also takes one by one (it declares its own count, for \
              the reason it gives), then what one census says: the stamp it was \
              read under, the months, and which of those files exist; a struct for \
              any of them would be a second spelling of one address in the crate"
)]
pub fn cached(
    site_calendars: &Cache,
    store_root: &Path,
    vendor: Vendor,
    exchange: &str,
    segment: &str,
    symbol: &str,
    stamp: Option<std::time::SystemTime>,
    months: &[YearMonth],
    holds: impl Fn(Timeframe, YearMonth) -> bool,
) -> Derived {
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
    cached_by(
        site_calendars,
        &key,
        stamp,
        || derive(store_root, vendor, exchange, segment, symbol, months),
        holds,
    )
}

/// [`cached`], with the derivation passed in.
///
/// Production passes [`derive`] over the request's series and months and
/// nothing else. The parameter is here so a test can hold a derivation open
/// until every concurrent request has joined it, and count how many times it
/// ran: the single-flight claim is a count, and a count needs no clock.
fn cached_by(
    site_calendars: &Cache,
    key: &Key,
    stamp: Option<std::time::SystemTime>,
    derive: impl Fn() -> (Calendar, Report),
    holds: impl Fn(Timeframe, YearMonth) -> bool,
) -> Derived {
    // NO STAMP, NO KEEPING, AND SO NOTHING TO SHARE: such a derivation answers
    // its own request only. See the doc above.
    let Some(now) = stamp else {
        return derived_and_kept(site_calendars, key, None, &derive, &holds);
    };
    loop {
        if let Some(hit) = kept(site_calendars, key, now) {
            return hit;
        }
        let flight_key = (key.clone(), now);
        let (flight, leads) = {
            let mut flights = site_calendars
                .flights
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(flight) = flights.get(&flight_key) {
                (std::sync::Arc::clone(flight), false)
            } else {
                let flight = std::sync::Arc::new(Flight::default());
                flights.insert(flight_key.clone(), std::sync::Arc::clone(&flight));
                (flight, true)
            }
        };
        if leads {
            let mut landing = Landing {
                cache: site_calendars,
                key: Some(flight_key),
                flight,
                answer: None,
            };
            // A FLIGHT THAT LANDED between the probe above and this one's
            // registration kept its calendar first: answer that, not a second
            // derivation.
            let answer = kept(site_calendars, key, now).unwrap_or_else(|| {
                derived_and_kept(site_calendars, key, Some(now), &derive, &holds)
            });
            landing.answer = Some(answer.clone());
            return answer;
        }
        site_calendars
            .waiting
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let landed = {
            let mut landed = flight
                .landed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            while matches!(*landed, Landed::Deriving) {
                landed = flight
                    .ready
                    .wait(landed)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            landed.clone()
        };
        site_calendars
            .waiting
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
        if let Landed::Answered(answer) = landed {
            return answer;
        }
        // THE LEADER UNWOUND WITHOUT ANSWERING. Go round again: the flight is
        // gone, so one waiter becomes the new leader and the rest join it.
    }
}

/// The calendar kept for `key` under exactly `now`, if there is one.
fn kept(site_calendars: &Cache, key: &Key, now: std::time::SystemTime) -> Option<Derived> {
    // READ THROUGH A POISONED LOCK rather than around it. A panic while
    // holding it means some other request died; the map is still readable,
    // and refusing to look would make one panicked request cost every later
    // one a 0.28 s re-derivation for the life of the process.
    let held = site_calendars
        .held
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.get(key)
        .filter(|(at, _)| *at == now)
        // A REFCOUNT BUMP, NOT A COPY. See [`Cache`].
        .map(|(_, calendar)| Derived {
            calendar: std::sync::Arc::clone(calendar),
            unopened: Vec::new(),
        })
}

/// Derive once, keep it when it may be kept, and say what was withheld.
fn derived_and_kept(
    site_calendars: &Cache,
    key: &Key,
    stamp: Option<std::time::SystemTime>,
    derive: &impl Fn() -> (Calendar, Report),
    holds: &impl Fn(Timeframe, YearMonth) -> bool,
) -> Derived {
    let (calendar, report) = derive();
    // A MONTH WITHHELD OR UNREADABLE IS SAID WHERE THE OPERATOR READS, once
    // per derivation. `read_days` refused such a month and nothing was logged,
    // so the only trace of it was a month missing from a 200. W1-api2-9,
    // D-0950.
    if !report.withheld.is_empty() || !report.unreadable.is_empty() {
        let first = report.unreadable.first().map_or("", String::as_str);
        let _dropped_when_filtered = telemetry::emit_if!(
            telemetry::Level::Warn,
            "api.calendar",
            "withheld",
            "vendor" => telemetry::Value::Str(key.0.as_str()),
            "symbol" => telemetry::Value::Str(&key.3),
            "months_withheld" => telemetry::Value::Uint(report.withheld.len() as u64),
            "months_unreadable" => telemetry::Value::Uint(report.unreadable.len() as u64),
            "first_unreadable" => telemetry::Value::Str(first),
        );
    }
    // WHAT THE DISK DID NOT GIVE, HELD AGAINST WHAT THE CENSUS SAYS IT HOLDS.
    let unopened: Vec<String> = report
        .unopened
        .into_iter()
        .filter(|file| holds(file.rung, file.month))
        .map(|file| file.reason)
        .collect();
    let calendar = std::sync::Arc::new(calendar);
    if let Some(now) = stamp
        && unopened.is_empty()
    {
        let mut held = site_calendars
            .held
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        held.insert(key.clone(), (now, std::sync::Arc::clone(&calendar)));
    }
    Derived { calendar, unopened }
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
/// the five pre-2025 Muhurats. `indexOwed` is the same number except on the
/// 2021-02-24 systems-outage day, where it is `null`: SEBI proves the 220-minute
/// market session and separately records unavailable NIFTY computation, without
/// one common NIFTY/BANKNIFTY/VIX publication window. `null` is not `0`: one
/// says the denominator is unproved, the other says nothing was owed.
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
        let expected = calendar.expected_bars(day);
        let _ = write!(out, "{{\"day\":{day},\"owed\":");
        match expected {
            Some(bars) => {
                let _ = write!(out, "{bars}");
            }
            None => out.push_str("null"),
        }
        out.push_str(",\"indexOwed\":");
        if day == pull::calendar::SYSTEMS_OUTAGE_DAY {
            out.push_str("null");
        } else {
            match expected {
                Some(bars) => {
                    let _ = write!(out, "{bars}");
                }
                None => out.push_str("null"),
            }
        }
        out.push('}');
    }
    // THE DAYS INSIDE THE SPAN THAT ARE NEITHER SESSIONS NOR CLOSED, AS RUNS.
    //
    // `days` omits them as it omits closed days, so without this a month whose
    // daily rung was not read was indistinguishable on the wire from a month
    // of holidays: 200, the month simply missing, and nothing saying why.
    // `withheld` names each stretch the store cannot speak for, so a reader
    // can tell "the exchange was shut" from "this was not measured".
    // R9-api-law-0, W1-api2-9, D-0950.
    out.push_str("],\"withheld\":[");
    let mut run: Option<(i64, i64)> = None;
    let mut first = true;
    let mut flush = |out: &mut String, run: (i64, i64)| {
        if !first {
            out.push(',');
        }
        first = false;
        let _ = write!(out, "{{\"from\":{},\"to\":{}}}", run.0, run.1);
    };
    for day in calendar.first_day()..=calendar.last_day() {
        if calendar.kind_of(day) == pull::calendar::DayKind::Unmeasured {
            run = Some(run.map_or((day, day), |(from, _)| (from, day)));
        } else if let Some(done) = run.take() {
            flush(&mut out, done);
        }
    }
    if let Some(done) = run {
        flush(&mut out, done);
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
            wire.contains("{\"day\":100,\"owed\":375,\"indexOwed\":375}"),
            "a full session is sized: {wire}"
        );
        assert!(
            wire.contains("{\"day\":102,\"owed\":null,\"indexOwed\":null}"),
            "a day that traded and cannot be sized is null, NOT 0: {wire}"
        );
        assert!(
            !wire.contains("\"day\":101"),
            "a closed day is omitted rather than listed as zero: {wire}"
        );
    }

    /// **THE EXCHANGE AND SPOT-INDEX DENOMINATORS DIVERGE LOUDLY ON THE
    /// SYSTEMS-OUTAGE DAY.**
    #[test]
    fn the_outage_day_publishes_market_minutes_and_refuses_index_minutes() {
        let cal = Calendar::from_observed(&[pull::calendar::Observed::from_runs(
            pull::calendar::SYSTEMS_OUTAGE_DAY,
            &[(555, 608)],
        )]);
        let wire = json(&cal);
        assert!(
            wire.contains(&format!(
                "{{\"day\":{},\"owed\":220,\"indexOwed\":null}}",
                pull::calendar::SYSTEMS_OUTAGE_DAY
            )),
            "220 market minutes are known; a common index count is not: {wire}"
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
    let mut withheld: Vec<i64> = Vec::new();
    for day in first..=last {
        let mut shut = false;
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
                DayKind::Closed => {
                    shut = true;
                    silent.push(name.clone());
                }
                // NOT MEASURED IS NOT A VOTE EITHER WAY. A reading whose daily
                // rung was not read for this day's month withheld it (D-0950):
                // it is no more silent about the day than a reading whose span
                // does not reach it.
                DayKind::Unmeasured => {}
            }
        }
        if seen_by.is_empty() {
            // NOBODY SAW A SESSION, AND NOBODY PROVED THE DAY SHUT EITHER: every
            // reading in range withheld it, or none reaches it (a gap between
            // two readings' spans). `from_observed` would fill it with `Closed`,
            // so it is withheld below instead. D-0950.
            if !shut {
                withheld.push(day);
            }
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
    let mut agreed = Calendar::from_observed(&observed);
    for day in withheld {
        agreed.withhold_closed(day, day);
    }
    (agreed, clashes)
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

    /// **A DAY NO READING PROVED SHUT IS WITHHELD FROM THE AGREEMENT, AND A
    /// WITHHELD READING IS NOT SILENT.** D-0950.
    ///
    /// NIFTY covers days 100-110 and withheld 104-106 (its daily rung was not
    /// read there); BANKNIFTY covers 100-102 and 108-110 only, as two readings
    /// whose spans leave a gap. Day 101 is shut by both: still `Closed`. Day 105
    /// is withheld by NIFTY and outside both BANKNIFTY readings: nobody proved
    /// it shut, so it is withheld, where `from_observed` alone made it a
    /// holiday. Day 103 is shut by NIFTY: `Closed`. No reading that withheld a
    /// day is named silent about it.
    #[test]
    fn a_day_no_reading_proved_shut_is_withheld_and_withholding_is_not_silence() {
        use pull::calendar::DayKind;
        let mut nifty = Calendar::from_observed(&[
            Observed::from_runs(100, &FULL_RUN),
            Observed::from_runs(102, &FULL_RUN),
            Observed::from_runs(107, &FULL_RUN),
            Observed::from_runs(110, &FULL_RUN),
        ]);
        assert_eq!(nifty.withhold_closed(104, 106), 3);
        let early = Calendar::from_observed(&[
            Observed::from_runs(100, &FULL_RUN),
            Observed::from_runs(102, &FULL_RUN),
        ]);
        let late = Calendar::from_observed(&[
            Observed::from_runs(108, &FULL_RUN),
            Observed::from_runs(110, &FULL_RUN),
        ]);
        let (exchange, clashes) = agree(&[
            ("NIFTY".to_owned(), nifty),
            ("BANKNIFTY-A".to_owned(), early),
            ("BANKNIFTY-B".to_owned(), late),
        ]);
        let kind = |day| exchange.kind_of(day);
        assert_eq!(kind(101), DayKind::Closed, "shut by every reading in range");
        assert_eq!(
            kind(103),
            DayKind::Closed,
            "shut by NIFTY's read daily rung"
        );
        for day in 104..=106 {
            assert_eq!(
                kind(day),
                DayKind::Unmeasured,
                "day {day}: nobody proved it shut"
            );
        }
        assert_eq!(kind(107), DayKind::Open(pull::calendar::Session::full()));
        assert_eq!(kind(108), DayKind::Open(pull::calendar::Session::full()));
        assert!(
            clashes
                .iter()
                .all(|clash| !(104..=106).contains(&clash.day)),
            "a withheld day is no reading's silence: {clashes:?}"
        );
        assert!(
            json(&exchange).contains("\"withheld\":[{\"from\":104,\"to\":106}]"),
            "{}",
            json(&exchange)
        );

        // ALL READINGS WITHHOLD: nothing is claimed shut, nothing invented.
        let mut only = Calendar::from_observed(&[
            Observed::from_runs(200, &FULL_RUN),
            Observed::from_runs(203, &FULL_RUN),
        ]);
        only.withhold_closed(i64::MIN, i64::MAX);
        let (exchange, clashes) = agree(&[("NIFTY".to_owned(), only)]);
        assert_eq!(exchange.sessions(), 2);
        assert_eq!(exchange.kind_of(201), DayKind::Unmeasured);
        assert!(clashes.is_empty(), "{clashes:?}");
    }

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
        assert!(
            wire.contains("{\"day\":500,\"owed\":375,\"indexOwed\":375}"),
            "{wire}"
        );
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
