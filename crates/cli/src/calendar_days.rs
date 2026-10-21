//! `cli calendar-days YEAR` and `cli calendar-days check` (CAL-1,
//! D-5150..D-5158).
//!
//! **`calendar-days YEAR`** prints every day of YEAR the tradable-day rule
//! excludes, one line each, then the year's totals. The rule is
//! `pull::calendar::tradable_day`: a day takes a trade and enters an own-past
//! statistic only when it is a regular weekday 09:15-15:29 session with no
//! special session, no notice of one and no market-wide halt. Ordinary closed
//! weekends are counted, not listed: about 104 lines a year that say nothing
//! a calendar does not. Every other excluded day is listed, holidays included.
//! It reads compile-time tables and no store, and the year's counts are one
//! `pull::calendar::excluded_between`: two lookups in the running sums.
//!
//! **`calendar-days check`** holds the calendar against the store
//! (`BRUTEX_STORE`), read-only:
//!
//! 1. **The years to complete** (operator, 16 Oct 2026, "NOTHING TYPED" item
//!    1): every calendar year holding a stored NSE month of a swept or
//!    reference instrument (NIFTY, BANKNIFTY, INDIA VIX, the 213 F&O cash
//!    equities) in any feed, read once per run from each feed's census
//!    (`pull::manifest`; BSE and MCX never count), each printed with whether
//!    its holiday and special-session circulars are recorded.
//! 2. **The cross-check** (item 2): every day NIFTY's or BANKNIFTY's bar
//!    months hold, in every feed, read through each month's own time index —
//!    two `BarFile::first_at_or_after` lookups bound a day (one index entry
//!    and one bar each through the month's `.tix`; a month without one is
//!    answered by the store's loud bisection), and the day is one indexed
//!    write into a dense day map ([`DayMasks`]), so a day costs O(1) per feed
//!    and no bar month is scanned. It prints (a) the days a
//!    feed holds that `tradable_day` refuses, (b) the weekend or holiday days
//!    a feed holds that no row names, as candidates UNVERIFIED until their
//!    circular is found, and (c) the weekdays called open that no feed holds.
//!
//! An unknown case is refused loudly, never counted quietly: a year holding a
//! day no table explains, a month that cannot be read, a candidate inside the
//! record or a missing session exits `FAILED` with the count named.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::path::Path;
use std::time::Instant;

use brutex_core::instrument::{Exchange, InstrumentKey, Segment};
use brutex_core::vendor::Vendor;
use pull::calendar::{self, DayKind, NotTradable, YearExclusions, YearHolidays};
use pull::ingest::census_store;
use pull::manifest::{Entry, Manifest};
use store::file::BarFile;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

/// `cli calendar-days YEAR`.
pub(crate) fn command(year: &str, out: &mut String) -> u8 {
    // FOUR ASCII DIGITS, NOTHING ELSE. `u16::from_str` alone also takes a
    // sign (`+2024` printed 2024's page) and any width (`7`, `02024`), so a
    // mistyped word could answer as a year it does not spell; it is refused
    // by name instead (CAL-1 self-check, D-5153).
    let four_digits = year.len() == 4 && year.bytes().all(|byte| byte.is_ascii_digit());
    let parsed = if four_digits {
        year.parse::<u16>().ok()
    } else {
        None
    };
    let Some(year) = parsed else {
        return crate::refuse(
            out,
            &format!("calendar-days takes a four-digit YEAR or `check`, and `{year}` is neither"),
        );
    };
    let counted = match calendar::year_exclusions(year) {
        Ok(counted) => counted,
        Err(why) => return crate::fail(out, &why.to_string()),
    };
    let counts = counted.counts;
    out.push_str(&page(&counted));
    crate::note(
        &telemetry::Event::info("cli.calendar_days", "excluded days counted")
            .with("year", u64::from(year))
            .with("tradable", u64::from(counts.tradable))
            .with("excluded_sessions", u64::from(counts.excluded_sessions()))
            .with(
                "closed_days",
                u64::from(
                    counts
                        .count(NotTradable::Weekend)
                        .saturating_add(counts.count(NotTradable::Holiday)),
                ),
            )
            .with("unexplained", u64::from(counts.unexplained())),
    );
    verdict(&counted, out)
}

/// `OK`, or a refusal naming the days no row explains.
fn verdict(counted: &YearExclusions, out: &mut String) -> u8 {
    let (year, unexplained) = (counted.year, counted.counts.unexplained());
    if unexplained == 0 {
        crate::OK
    } else {
        crate::fail(
            out,
            &format!(
                "{unexplained} day(s) of {year} have no row that explains them (marked \
                 UNEXPLAINED above); each must become a sourced row and a test before {year} \
                 is counted"
            ),
        )
    }
}

/// The whole page for one counted year.
fn page(counted: &YearExclusions) -> String {
    let mut out = String::new();
    let (year, counts) = (counted.year, counted.counts);
    let _ = writeln!(
        out,
        "CALENDAR DAYS {year} -- the tradable-day rule (pull::calendar::tradable_day, D-5150)"
    );
    out.push_str(
        "A day takes a trade and enters an own-past statistic only when it is a regular \
         weekday 09:15-15:29 session with no special session, no notice of one and no \
         market-wide halt. Every other day is excluded; its data stays stored and labelled. \
         A measured short day stays tradable: its missing minutes are the readers' question. \
         Ordinary closed weekends are counted below, not listed.\n",
    );
    let record = calendar::year_record(year);
    let _ = writeln!(
        out,
        "Holidays of {year}: {}. Special-session notices recorded: {} ({} untimed).\n",
        holidays_text(record.holidays),
        record.notices,
        record.untimed_notices
    );
    let _ = writeln!(
        out,
        "{:<10}  {:<3}  {:<22}  what it was",
        "day", "wd", "reason"
    );
    let mut day = counts.first_day;
    while day <= counts.last_day {
        if let Err(reason) = calendar::tradable_day(day)
            && reason != NotTradable::Weekend
        {
            out.push_str(&line(day, reason));
        }
        day += 1;
    }
    out.push('\n');
    let _ = writeln!(out, "TOTALS {year}");
    let _ = writeln!(out, "  days in the year        {:>4}", counts.days());
    let _ = writeln!(out, "  tradable sessions       {:>4}", counts.tradable);
    let _ = writeln!(
        out,
        "  excluded sessions       {:>4}   (traded by the exchange, kept stored and labelled)",
        counts.excluded_sessions()
    );
    for reason in NotTradable::ALL {
        if reason.is_session() {
            let _ = writeln!(
                out,
                "    {:<21} {:>4}   {}",
                reason.name(),
                counts.count(reason),
                reason.label()
            );
        }
    }
    let _ = writeln!(
        out,
        "  closed: weekends        {:>4}",
        counts.count(NotTradable::Weekend)
    );
    let _ = writeln!(
        out,
        "  closed: holidays        {:>4}",
        counts.count(NotTradable::Holiday)
    );
    let _ = writeln!(out, "  unexplained             {:>4}", counts.unexplained());
    out
}

/// How a year's holidays are on record, in words.
fn holidays_text(holidays: YearHolidays) -> String {
    match holidays {
        YearHolidays::Recorded(number) => format!("NSE circular {number}"),
        YearHolidays::Measured => "measured from the store (the whole year lies in the \
                                   measured window)"
            .to_owned(),
        YearHolidays::Owed => "OWED: no NSE holiday circular recorded; days outside the \
                               measured window are Unmeasured"
            .to_owned(),
    }
}

/// One excluded day: its date, weekday, reason and what the tables say.
fn line(day: i64, reason: NotTradable) -> String {
    let record = calendar::record_of(day);
    let mut what = String::from(reason.label());
    if reason.is_unexplained() {
        what.push_str("; UNEXPLAINED");
    }
    if let Some(holiday) = record.holiday {
        let _ = write!(what, "; {}", holiday.name);
    } else if reason == NotTradable::Holiday {
        // Inside the measured window a holiday is a weekday the store's
        // daily bars prove closed; no circular row names it (D-2805), so the
        // line says so rather than ending as if a name were missing by error.
        what.push_str("; measured closed from the store's daily bars, no name recorded");
    }
    if let Some(notice) = calendar::session_circular(day) {
        let _ = write!(what, "; source {}", notice.source);
    } else if let Some(special) = record.special {
        let _ = write!(what, "; evidence {}", special.evidence);
    }
    if let Some(halt) = record.halt {
        let lane = if halt.evidence.is_suspected() {
            "suspected, no NSE circular recorded: UNVERIFIED"
        } else {
            "sourced"
        };
        // The reason's own label already says "market-wide halt" when the
        // halt is the reason; a day excluded for something else first (the
        // 2021-02-24 outage) names the halt as a second fact.
        if reason == NotTradable::MarketHalt {
            let _ = write!(what, " ({lane})");
        } else {
            let _ = write!(what, "; also a market-wide halt ({lane})");
        }
    }
    format!(
        "{:<10}  {:<3}  {:<22}  {what}\n",
        civil(day),
        record.weekday.short(),
        reason.name()
    )
}

/// `YYYY-MM-DD` for a day the civil calendar spells; the raw number otherwise.
fn civil(day: i64) -> String {
    u32::try_from(day)
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
        .map_or_else(|| day.to_string(), |civil| civil.to_string())
}

// ══ `calendar-days check`: the calendar against the store (D-5158) ══

/// The two indices whose bar months are the per-day source of every feed.
const DAY_SOURCES: [&str; 2] = ["NIFTY", "BANKNIFTY"];

/// Whether `symbol` is one of the [`DAY_SOURCES`]: two string comparisons,
/// the table destructured whole, so a third source is a compile error here
/// rather than a symbol the year count silently never matches (gate 11
/// rule 7: no slice membership test whose length the reader must look up).
fn is_day_source(symbol: &str) -> bool {
    let [first, second] = DAY_SOURCES;
    symbol == first || symbol == second
}

/// The reference index that counts toward a year (CLAUDE.md §1: stored and
/// stamped, never swept).
const REFERENCE_INDEX: &str = "INDIAVIX";

/// Feeds one day's `u8` mask can name.
const MAX_FEEDS: usize = 8;

/// Microseconds in a day.
const DAY_MICROS: i64 = 86_400_000_000;

/// IST is UTC+05:30: an IST midnight is this many microseconds before the
/// UTC midnight of the same date.
const IST_OFFSET_MICROS: i64 = 19_800_000_000;

/// The UTC microsecond an IST day starts at.
const fn ist_day_start(day: i64) -> i64 {
    day.saturating_mul(DAY_MICROS)
        .saturating_sub(IST_OFFSET_MICROS)
}

/// One feed's per-day source: a vendor and the bar length its index months
/// are read at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Feed {
    /// The vendor.
    pub(crate) vendor: Vendor,
    /// The bar length read: `1day` where the feed holds it, else the finest
    /// it holds.
    pub(crate) timeframe: Timeframe,
}

impl Feed {
    /// `vendor timeframe`, as the page prints it.
    fn name(self) -> String {
        format!("{} {}", self.vendor.as_str(), self.timeframe.as_str())
    }
}

/// Day (since the epoch) to the mask of feeds holding it, **dense**: slot `i`
/// is day `first + i`, so marking a day and reading it are one indexed access
/// each, O(1) at any store size, and the days come out in order with no sort
/// (D-5158). It was an ordered tree map, O(log d) a day; the span is now
/// fixed before a single month is read, from the months the censuses say
/// will be read, so it never grows during a read: one byte per calendar day
/// of that span (4,291 bytes for 2015-01..2026-09). Measured at a 31-day and
/// a 36,525-day span by
/// `cli::calendar_days::tests::a_day_costs_the_same_at_any_span_measured`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DayMasks {
    /// The day slot 0 stands for.
    first: i64,
    /// One mask per day of the span; 0 is "no feed holds it".
    masks: Vec<u8>,
    /// Days with a non-zero mask.
    held: usize,
    /// The first and the last day held, kept as days are marked.
    bounds: Option<(i64, i64)>,
}

impl DayMasks {
    /// An empty map over `first..=last`; an inverted span holds no slot.
    pub(crate) fn over(first: i64, last: i64) -> Self {
        let width = last
            .checked_sub(first)
            .and_then(|width| width.checked_add(1))
            .and_then(|width| usize::try_from(width).ok())
            .unwrap_or(0);
        Self {
            first,
            masks: vec![0; width],
            held: 0,
            bounds: None,
        }
    }

    /// `day`'s slot, or `None` outside the span.
    fn slot(&self, day: i64) -> Option<usize> {
        usize::try_from(day.checked_sub(self.first)?)
            .ok()
            .filter(|slot| *slot < self.masks.len())
    }

    /// OR `mask` into `day`'s mask. `false`, and nothing marked, for a day
    /// outside the span: the caller refuses it by name.
    pub(crate) fn mark(&mut self, day: i64, mask: u8) -> bool {
        let Some(slot) = self.slot(day) else {
            return false;
        };
        let Some(held) = self.masks.get_mut(slot) else {
            return false;
        };
        if *held == 0 && mask != 0 {
            self.held = self.held.saturating_add(1);
            self.bounds = Some(match self.bounds {
                Some((low, high)) => (low.min(day), high.max(day)),
                None => (day, day),
            });
        }
        *held |= mask;
        true
    }

    /// The mask of a held day; `None` for a day no feed holds.
    pub(crate) fn get(&self, day: i64) -> Option<u8> {
        self.slot(day)
            .and_then(|slot| self.masks.get(slot).copied())
            .filter(|mask| *mask != 0)
    }

    /// Whether any feed holds `day`.
    pub(crate) fn contains(&self, day: i64) -> bool {
        self.get(day).is_some()
    }

    /// Days held by any feed.
    pub(crate) const fn len(&self) -> usize {
        self.held
    }

    /// Whether no feed holds any day.
    pub(crate) const fn is_empty(&self) -> bool {
        self.held == 0
    }

    /// The first day held.
    pub(crate) fn first_day(&self) -> Option<i64> {
        self.bounds.map(|(first, _)| first)
    }

    /// The last day held.
    pub(crate) fn last_day(&self) -> Option<i64> {
        self.bounds.map(|(_, last)| last)
    }

    /// Every held day with its mask, in day order: one walk of the span.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (i64, u8)> + '_ {
        let first = self.first;
        (0_i64..)
            .zip(self.masks.iter())
            .filter(|(_, mask)| **mask != 0)
            .map(move |(offset, mask)| (first.saturating_add(offset), *mask))
    }

    /// Every held day's mask, in day order.
    pub(crate) fn values(&self) -> impl Iterator<Item = &u8> + '_ {
        self.masks.iter().filter(|mask| **mask != 0)
    }
}

impl FromIterator<(i64, u8)> for DayMasks {
    /// A map spanning exactly the days given, each marked with its mask.
    fn from_iter<I: IntoIterator<Item = (i64, u8)>>(days: I) -> Self {
        let days: Vec<(i64, u8)> = days.into_iter().collect();
        let first = days.iter().map(|(day, _)| *day).min();
        let last = days.iter().map(|(day, _)| *day).max();
        let mut map = match (first, last) {
            (Some(first), Some(last)) => Self::over(first, last),
            _ => Self::default(),
        };
        for (day, mask) in days {
            map.mark(day, mask);
        }
        map
    }
}

/// What the store holds, as the check reads it: the input of [`findings`].
#[derive(Debug, Clone, Default)]
pub(crate) struct Holdings {
    /// Every feed read, in order; a day's mask bit `i` is feed `i`.
    pub(crate) feeds: Vec<Feed>,
    /// Day (since the epoch) to the mask of feeds holding it.
    pub(crate) days: DayMasks,
    /// Year to the vendors holding a swept or reference NSE month in it.
    pub(crate) years: BTreeMap<u16, BTreeSet<&'static str>>,
    /// Instrument-months read, per feed.
    pub(crate) months_read: Vec<u32>,
    /// Every census or month that could not be read, with its reason.
    pub(crate) refusals: Vec<String>,
}

/// What the check found: the three lists, and the owed years' held days.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Findings {
    /// (a) held days `tradable_day` refuses as a session, with the reason.
    pub(crate) refused: Vec<(i64, NotTradable, u8)>,
    /// (a) held days outside the record, per year: (count, first, last).
    pub(crate) unmeasured: BTreeMap<i64, (u32, i64, i64)>,
    /// (b) held weekend or holiday days no row names, with why, inside the
    /// record.
    pub(crate) candidates: Vec<(i64, &'static str, u8)>,
    /// (b) held weekend days of an owed year: no row can name them yet.
    pub(crate) owed_candidates: Vec<(i64, u8)>,
    /// (c) weekdays the record calls open that no feed holds, between the
    /// first and the last held day.
    pub(crate) missing: Vec<i64>,
    /// Held days past the measured window, inside the record: what a next
    /// measured version would add.
    pub(crate) past_window: u32,
}

impl Findings {
    /// Findings that make the check fail: an unexplained refusal, a
    /// candidate inside the record or a missing session.
    fn failing(&self) -> usize {
        self.refused
            .iter()
            .filter(|(_, reason, _)| reason.is_unexplained())
            .count()
            + self.candidates.len()
            + self.missing.len()
    }
}

/// The three lists, a pure function of what the store holds and the tables:
/// one `tradable_day` per held day and one per weekday between the first and
/// the last held day, each O(1), and each day's mask one indexed read of the
/// dense [`DayMasks`]. Measured per held day over a whole record's days by
/// `cli::calendar_days::tests::the_check_costs_constant_time_per_day_measured`.
pub(crate) fn findings(held: &Holdings) -> Findings {
    let mut found = Findings::default();
    for (day, mask) in held.days.iter() {
        match calendar::tradable_day(day) {
            Ok(()) => {}
            Err(NotTradable::Unmeasured) => {
                let slot = found
                    .unmeasured
                    .entry(calendar::civil_year(day))
                    .or_insert((0, day, day));
                slot.0 = slot.0.saturating_add(1);
                slot.2 = day;
                if calendar::weekday_of(day).is_weekend() {
                    found.owed_candidates.push((day, mask));
                }
            }
            Err(NotTradable::Weekend) => {
                found
                    .candidates
                    .push((day, "a weekend the record calls closed", mask));
            }
            Err(NotTradable::Holiday) => {
                found
                    .candidates
                    .push((day, "a trading holiday the record calls closed", mask));
            }
            Err(reason) => found.refused.push((day, reason, mask)),
        }
        if day > calendar::LAST_DAY && day <= calendar::SOURCED_LAST_DAY {
            found.past_window = found.past_window.saturating_add(1);
        }
    }
    if let (Some(first), Some(last)) = (held.days.first_day(), held.days.last_day()) {
        let mut day = first;
        while day <= last {
            let open = matches!(
                calendar::sourced_kind_of(day),
                DayKind::Open(_) | DayKind::OpenLengthUnmeasured
            );
            if open && !held.days.contains(day) {
                found.missing.push(day);
            }
            day += 1;
        }
    }
    found
}

/// `cli calendar-days check`: read the store named by `BRUTEX_STORE`.
pub(crate) fn check_command(out: &mut String) -> u8 {
    let root = match crate::preflight_store_root() {
        Ok(root) => root,
        Err(why) => return crate::fail(out, &why),
    };
    check_at(&root, out)
}

/// The check over the store at `root`, read-only.
pub(crate) fn check_at(root: &Path, out: &mut String) -> u8 {
    let started = Instant::now();
    let held = read_holdings(root);
    let read_ns = started.elapsed().as_nanos();
    let found = findings(&held);
    let days_read = total_feed_days(&held);
    out.push_str(&check_page(root, &held, &found, read_ns));
    crate::note(
        &telemetry::Event::info("cli.calendar_check", "calendar held against the store")
            .with("feeds", wide(held.feeds.len()))
            .with(
                "months_read",
                held.months_read.iter().map(|n| u64::from(*n)).sum::<u64>(),
            )
            .with("days_held", wide(held.days.len()))
            .with("feed_days", wide(days_read))
            .with("refused_sessions", wide(found.refused.len()))
            .with("candidates", wide(found.candidates.len()))
            .with("owed_candidates", wide(found.owed_candidates.len()))
            .with("missing", wide(found.missing.len()))
            .with("refusals", wide(held.refusals.len()))
            .with(
                "read_ms",
                u64::try_from(read_ns / 1_000_000).unwrap_or(u64::MAX),
            ),
    );
    // AN EMPTY CHECK IS NOT A PASS (CAL-1 self-check): a store whose censuses
    // name no NIFTY or BANKNIFTY bar month held nothing against the calendar,
    // and "0 findings" over 0 days would read as agreement.
    if held.days.is_empty() && held.refusals.is_empty() {
        return crate::fail(
            out,
            "no feed's census names a NIFTY or BANKNIFTY bar month holding a day, so no stored \
             day was held against the calendar; a check of nothing is not a pass",
        );
    }
    let failing = found.failing();
    if held.refusals.is_empty() && failing == 0 {
        crate::OK
    } else {
        crate::fail(
            out,
            &format!(
                "{} source(s) could not be read and {failing} finding(s) inside the record \
                 need a row (listed above); each becomes a sourced row and a test, or a \
                 repaired month, before the calendar and the store agree",
                held.refusals.len()
            ),
        )
    }
}

/// The page of the check: its head, the years, the sources, the three
/// lists and the totals, each written by its own section.
fn check_page(root: &Path, held: &Holdings, found: &Findings, read_ns: u128) -> String {
    let mut out = String::new();
    head_section(root, &mut out);
    years_section(held, &mut out);
    sources_section(held, read_ns, &mut out);
    lists_section(held, found, &mut out);
    totals_section(held, found, &mut out);
    out
}

/// The store and the calendar version the check holds it against.
fn head_section(root: &Path, out: &mut String) {
    let version = calendar::CalendarVersion::newest();
    let (record_first, record_last) = version.record();
    let (measured_first, measured_last) = version.measured();
    let digest =
        calendar::calendar_digest()
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            });
    let _ = writeln!(
        out,
        "CALENDAR CHECK -- the calendar of record against the stored days (D-5158), read-only"
    );
    let _ = writeln!(out, "store: {}", root.display());
    let _ = writeln!(
        out,
        "calendar version {}, digest {digest}; record {}..{}; measured {}..{}\n",
        version.number(),
        civil(record_first),
        civil(record_last),
        civil(measured_first),
        civil(measured_last)
    );
}

/// Item 1: every year holding a stored month, and its record.
fn years_section(held: &Holdings, out: &mut String) {
    let _ = writeln!(
        out,
        "YEARS HOLDING A STORED NSE MONTH (NIFTY, BANKNIFTY, INDIAVIX, the F&O cash equities; \
         any feed)"
    );
    // Two spaces between columns whatever a cell holds: the widest holidays
    // text (an owed year's) ran into its notices before (D-5158).
    let _ = writeln!(
        out,
        "{:<6}{:<88}  {:<15}  feeds",
        "year", "holidays", "notices"
    );
    for (year, vendors) in &held.years {
        let record = calendar::year_record(*year);
        let feeds: Vec<&str> = vendors.iter().copied().collect();
        let notices = format!("{} ({} untimed)", record.notices, record.untimed_notices);
        let _ = writeln!(
            out,
            "{:<6}{:<88}  {:<15}  {}",
            year,
            holidays_text(record.holidays),
            notices,
            feeds.join(" ")
        );
    }
    let owed: Vec<String> = held
        .years
        .keys()
        .filter(|year| !calendar::year_record(**year).answered())
        .map(u16::to_string)
        .collect();
    if !owed.is_empty() {
        let _ = writeln!(
            out,
            "OWED: the holiday and special-session circulars of {} are not recorded; their \
             days are Unmeasured until they are (docs/06-limits.md L-CAL-8).",
            owed.join(", ")
        );
    }
    let horizon = calendar::record_horizon(held.days.last_day().unwrap_or(i64::MIN));
    if horizon.ends_soon {
        let _ = writeln!(
            out,
            "OWED SOON: the newest stored day is in {}, the latest sourced year; NSE's \
             holiday circular for {} is owed (D-5156).",
            calendar::SOURCED_LAST_YEAR,
            horizon.owed_year
        );
    }
}

/// What each feed's per-day source read, what it cost, and what it could not.
fn sources_section(held: &Holdings, read_ns: u128, out: &mut String) {
    let _ = writeln!(
        out,
        "\nPER-DAY SOURCES: NIFTY and BANKNIFTY bar months, two time-index lookups a day"
    );
    for (bit, feed) in held.feeds.iter().enumerate() {
        let _ = writeln!(
            out,
            "  {:<16} {:>5} months read, {:>5} days held",
            feed.name(),
            held.months_read.get(bit).copied().unwrap_or(0),
            feed_days(held, bit)
        );
    }
    let per_day = read_ns
        .checked_div(u128::try_from(total_feed_days(held)).unwrap_or(u128::MAX))
        .unwrap_or(0);
    let _ = writeln!(
        out,
        "  read in {} ms, {per_day} ns per held feed-day (censuses, opens and lookups together)",
        read_ns / 1_000_000
    );
    for refusal in &held.refusals {
        let _ = writeln!(out, "  NOT READ: {refusal}");
    }
}

/// The three lists, (a), (b) and (c).
fn lists_section(held: &Holdings, found: &Findings, out: &mut String) {
    let _ = writeln!(
        out,
        "\n(a) DAYS A FEED HOLDS THAT tradable_day REFUSES (excluded, kept stored and labelled)"
    );
    for (day, reason, mask) in &found.refused {
        let mut what = reason.label().to_owned();
        if reason.is_unexplained() {
            what.push_str("; UNEXPLAINED");
        }
        let _ = writeln!(
            out,
            "{:<10}  {:<3}  {:<22}  {what}; held by {}",
            civil(*day),
            calendar::weekday_of(*day).short(),
            reason.name(),
            holders(held, *mask)
        );
    }
    for (year, (count, first, last)) in &found.unmeasured {
        let _ = writeln!(
            out,
            "{year}: {count} held days are Unmeasured ({}..{}): the year's circulars are owed",
            civil(*first),
            civil(*last)
        );
    }
    let _ = writeln!(
        out,
        "\n(b) WEEKEND OR HOLIDAY DAYS A FEED HOLDS THAT NO ROW NAMES -- candidates, UNVERIFIED \
         until their circular is found"
    );
    for (day, why, mask) in &found.candidates {
        let _ = writeln!(
            out,
            "{:<10}  {:<3}  {why}; held by {}; UNVERIFIED",
            civil(*day),
            calendar::weekday_of(*day).short(),
            holders(held, *mask)
        );
    }
    for (day, mask) in &found.owed_candidates {
        let _ = writeln!(
            out,
            "{:<10}  {:<3}  a weekend in an owed year; held by {}; UNVERIFIED",
            civil(*day),
            calendar::weekday_of(*day).short(),
            holders(held, *mask)
        );
    }
    let _ = writeln!(
        out,
        "\n(c) WEEKDAYS CALLED OPEN THAT NO FEED HOLDS (between the first and the last held day)"
    );
    for day in &found.missing {
        let _ = writeln!(
            out,
            "{:<10}  {:<3}  called open; no feed holds a NIFTY or BANKNIFTY bar",
            civil(*day),
            calendar::weekday_of(*day).short()
        );
    }
}

/// The totals of the check.
fn totals_section(held: &Holdings, found: &Findings, out: &mut String) {
    let _ = writeln!(out, "\nTOTALS");
    let _ = writeln!(out, "  days held by any feed        {:>6}", held.days.len());
    let _ = writeln!(
        out,
        "  (a) refused sessions held    {:>6}",
        found.refused.len()
    );
    let _ = writeln!(
        out,
        "  (a) unmeasured days held     {:>6}",
        found.unmeasured.values().map(|(n, _, _)| *n).sum::<u32>()
    );
    let _ = writeln!(
        out,
        "  (b) candidates in the record {:>6}",
        found.candidates.len()
    );
    let _ = writeln!(
        out,
        "  (b) candidates in owed years {:>6}",
        found.owed_candidates.len()
    );
    let _ = writeln!(
        out,
        "  (c) missing sessions         {:>6}",
        found.missing.len()
    );
    let _ = writeln!(
        out,
        "  held days past the measured window, inside the record: {} (a next measured \
         version would add them, D-5154)",
        found.past_window
    );
}

/// A count, widened for an event field.
fn wide(count: usize) -> u64 {
    u64::try_from(count).unwrap_or(u64::MAX)
}

/// The days feed `bit` holds.
fn feed_days(held: &Holdings, bit: usize) -> usize {
    held.days
        .values()
        .filter(|mask| (u32::from(**mask) >> bit) & 1 == 1)
        .count()
}

/// The days every feed holds, summed over the feeds.
fn total_feed_days(held: &Holdings) -> usize {
    (0..held.feeds.len()).map(|bit| feed_days(held, bit)).sum()
}

/// The feeds a mask names, by name.
fn holders(held: &Holdings, mask: u8) -> String {
    let names: Vec<String> = held
        .feeds
        .iter()
        .enumerate()
        .filter(|(bit, _)| (u32::from(mask) >> bit) & 1 == 1)
        .map(|(_, feed)| feed.name())
        .collect();
    names.join(", ")
}

/// Every feed's spot censuses, read once, then every NIFTY and BANKNIFTY bar
/// month through its time index. A census or month that cannot be read is a
/// named refusal, never a silent gap.
///
/// Two passes so the day map is dense from the start: the censuses first,
/// which name every month that will be read and so the span of days
/// ([`DayMasks::over`]); then the months, each day one indexed write.
pub(crate) fn read_holdings(root: &Path) -> Holdings {
    let mut held = Holdings::default();
    let mut plans: Vec<MonthPlan> = Vec::new();
    let mut span: Option<(i64, i64)> = None;
    let fno: HashSet<&str> = brutex_core::universe::FNO_UNDERLYINGS
        .iter()
        .copied()
        .collect();
    for vendor in Vendor::ALL {
        let censuses = match spot_censuses(root, vendor) {
            Ok(censuses) => censuses,
            Err(why) => {
                held.refusals
                    .push(format!("the {} census: {why}", vendor.as_str()));
                continue;
            }
        };
        let mut index_months: BTreeMap<(&'static str, Timeframe), BTreeSet<YearMonth>> =
            BTreeMap::new();
        for census in &censuses {
            for entry in census.newest() {
                if !counts_toward_a_year(&entry, &fno) {
                    continue;
                }
                held.years
                    .entry(entry.key.month.year())
                    .or_default()
                    .insert(vendor.as_str());
                if entry.key.segment == Segment::Index {
                    for symbol in DAY_SOURCES {
                        if entry.key.symbol.as_str() == symbol {
                            index_months
                                .entry((symbol, entry.key.timeframe))
                                .or_default()
                                .insert(entry.key.month);
                        }
                    }
                }
            }
        }
        let Some(timeframe) = day_source_timeframe(&index_months) else {
            continue;
        };
        let bit = held.feeds.len();
        if bit >= MAX_FEEDS {
            held.refusals.push(format!(
                "{}: more than {MAX_FEEDS} feeds, more than one day's mask can name",
                vendor.as_str()
            ));
            continue;
        }
        held.feeds.push(Feed { vendor, timeframe });
        held.months_read.push(0);
        let mut months = Vec::new();
        for symbol in DAY_SOURCES {
            let Some(held_months) = index_months.get(&(symbol, timeframe)) else {
                continue;
            };
            for month in held_months {
                let (first, last) = month_day_bounds(*month);
                span = Some(match span {
                    Some((low, high)) => (low.min(first), high.max(last)),
                    None => (first, last),
                });
                months.push((symbol, *month));
            }
        }
        plans.push(MonthPlan {
            bit,
            vendor,
            timeframe,
            months,
        });
    }
    if let Some((first, last)) = span {
        held.days = DayMasks::over(first, last);
    }
    read_planned(root, plans, &mut held);
    held
}

/// The second pass of [`read_holdings`]: every planned month through its
/// time index, each held day one indexed write into `held.days`.
fn read_planned(root: &Path, plans: Vec<MonthPlan>, held: &mut Holdings) {
    for MonthPlan {
        bit,
        vendor,
        timeframe,
        months,
    } in plans
    {
        for (symbol, month) in months {
            match month_days(root, vendor, symbol, timeframe, month) {
                Ok(days) => {
                    note_month(vendor, symbol, timeframe, month, days.len());
                    for day in days {
                        if !held.days.mark(day, 1_u8 << bit) {
                            held.refusals.push(format!(
                                "{} {symbol} {} {month}: day {day} lies outside the span the \
                                 censuses named",
                                vendor.as_str(),
                                timeframe.as_str()
                            ));
                        }
                    }
                    if let Some(read) = held.months_read.get_mut(bit) {
                        *read = read.saturating_add(1);
                    }
                }
                Err(why) => held.refusals.push(format!(
                    "{} {symbol} {} {month}: {why}",
                    vendor.as_str(),
                    timeframe.as_str()
                )),
            }
        }
    }
}

/// One feed's months to read, planned from its censuses before any is read.
struct MonthPlan {
    /// The feed's bit in a day's mask.
    bit: usize,
    /// The vendor.
    vendor: Vendor,
    /// The bar length read.
    timeframe: Timeframe,
    /// Every index month to read, with its symbol.
    months: Vec<(&'static str, YearMonth)>,
}

/// The first and the last IST day of `month`, as days since the epoch.
fn month_day_bounds(month: YearMonth) -> (i64, i64) {
    let (start, end) = month.ist_bounds_micros();
    (
        start.saturating_add(IST_OFFSET_MICROS) / DAY_MICROS,
        end.saturating_add(IST_OFFSET_MICROS) / DAY_MICROS - 1,
    )
}

/// One event per instrument-month read (operator rule 4).
fn note_month(vendor: Vendor, symbol: &str, timeframe: Timeframe, month: YearMonth, days: usize) {
    let month = month.to_string();
    crate::note(
        &telemetry::Event::info("cli.calendar_check", "instrument-month read")
            .with("vendor", telemetry::Value::Str(vendor.as_str()))
            .with("symbol", telemetry::Value::Str(symbol))
            .with("timeframe", telemetry::Value::Str(timeframe.as_str()))
            .with("month", telemetry::Value::Str(&month))
            .with("days_held", wide(days)),
    );
}

/// Whether a census entry is a stored NSE month of a swept or reference
/// instrument: NIFTY, BANKNIFTY, INDIA VIX, or an F&O underlying's cash
/// equity. A contract, BSE, MCX and every other index or stock never count.
fn counts_toward_a_year(entry: &Entry, fno: &HashSet<&str>) -> bool {
    let key = &entry.key;
    if key.exchange != Exchange::Nse || key.contract.is_some() || entry.rows == 0 {
        return false;
    }
    let symbol = key.symbol.as_str();
    match key.segment {
        Segment::Index => is_day_source(symbol) || symbol == REFERENCE_INDEX,
        Segment::Cash => fno.contains(symbol),
        Segment::Fno => false,
    }
}

/// The bar length a feed's days are read at: `1day` when it holds daily
/// months of either index, else the finest length it holds of them.
fn day_source_timeframe(
    months: &BTreeMap<(&'static str, Timeframe), BTreeSet<YearMonth>>,
) -> Option<Timeframe> {
    if months.keys().any(|(_, tf)| *tf == Timeframe::DAY_1) {
        return Some(Timeframe::DAY_1);
    }
    months.keys().map(|(_, tf)| *tf).min()
}

/// A vendor's censuses of the spot segments: the single file, or the INDEX
/// and CASH partitions' current epochs (the FNO partition is never loaded:
/// no contract counts toward a year).
fn spot_censuses(root: &Path, vendor: Vendor) -> Result<Vec<Manifest>, String> {
    match census_store::locate(root, vendor)? {
        census_store::Location::Single(_) => Ok(census_store::read_vendor(root, vendor)?
            .into_iter()
            .map(|read| read.census)
            .collect()),
        census_store::Location::Partitioned(_) => {
            let mut out = Vec::new();
            for segment in [Segment::Index, Segment::Cash] {
                if let Some(epoch) = census_store::current_epoch(root, vendor, segment)? {
                    let path = pull::manifest::epoch_path(root, vendor, segment, epoch);
                    out.push(census_store::read_epoch(&path, vendor, segment)?);
                }
            }
            Ok(out)
        }
    }
}

/// The IST days one index month holds a bar on: for each day of the month,
/// two `first_at_or_after` lookups (the day's start and the next day's,
/// shared with the next day), each at most one index entry and one bar read
/// through the month's `.tix`, the bisection otherwise. O(1) per day,
/// measured over n = 240 calls of 31 days by
/// `cli::calendar_days::tests::the_check_costs_constant_time_per_day_measured`.
fn month_days(
    root: &Path,
    vendor: Vendor,
    symbol: &str,
    timeframe: Timeframe,
    month: YearMonth,
) -> Result<Vec<i64>, String> {
    let key = InstrumentKey::index(Exchange::Nse, symbol).map_err(|why| why.to_string())?;
    let path = StorePath::for_key(vendor, &key, timeframe, month, FileKind::Bars)
        .map_err(|why| why.to_string())?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id IS the low 32 bits of the FNV-1a hash, exactly as the store's writer \
                  and `fold_audit::read_month` compute it"
    )]
    let symbol_id = brutex_core::universe::fnv1a(key.underlying.as_str()) as u32;
    let file = BarFile::open_existing(root, path, symbol_id).map_err(|why| why.to_string())?;
    let (first_day, last_day) = month_day_bounds(month);
    let mut days = Vec::new();
    let mut row = file
        .first_at_or_after(ist_day_start(first_day))
        .map_err(|why| why.to_string())?;
    let mut day = first_day;
    while day <= last_day {
        let next = file
            .first_at_or_after(ist_day_start(day + 1))
            .map_err(|why| why.to_string())?;
        if next > row {
            days.push(day);
        }
        row = next;
        day += 1;
    }
    Ok(days)
}

#[cfg(test)]
#[path = "calendar_days_tests.rs"]
mod tests;
