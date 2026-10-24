//! The measurements R4-27's decisions were taken on (INT-15, AUT-10), as
//! verbs a second agent repeats. Both READ ONLY.
//!
//! * `cli padded-measure FEED LIST`: the vendor-padded minute measured over
//!   stored months, both fold treatments side by side. It opens bar files
//!   through the store's own verified read and writes nothing; the store root
//!   is the one every verb resolves (`$BRUTEX_STORE`).
//! * `cli rate-replay TABLE FROM TO`: an RBI 91-day T-bill table replayed
//!   over the calendar's sessions under the staleness bounds the decision
//!   compared (14, 21 and 28 days). It reads the one table file it is named
//!   and no store.
//!
//! # `padded-measure`: what one row says
//!
//! `LIST` names one `SYMBOL YYYY-MM` a line. For each, the one-minute month is
//! read once, its padded minutes counted by `pull::padded::per_day` (the full
//! read), and each of the seven derived rungs folded three times through
//! `pull::fold::derive_minutes_with`: as before INT-15 (`as_trade`), under
//! treatment (a) (`keep_flat`) and under treatment (b) (`absent`). A row is
//!
//! `SYMBOL YM RUNG padded flat0 buckets open_moved range_moved only_padded
//! stored diff_as_trade diff_keep_flat diff_absent`
//!
//! * `padded`: the month's vendor-padded minutes, and `flat0` its flat
//!   zero-volume minutes of any price (the offline script's `only_z_flat`
//!   further asks that GDFL has no trade in them);
//! * `buckets`: bars the fold before INT-15 writes;
//! * `open_moved`, `range_moved`: of those, the bars whose open, or whose
//!   high or low, the new reading changes (the two treatments agree on every
//!   bucket that traded at all);
//! * `only_padded`: buckets of padded minutes and nothing else, which (a)
//!   keeps flat and (b) leaves without a bar;
//! * `stored`: bars the store holds for the rung-month, or `-` when it holds
//!   no readable month;
//! * `diff_*`: buckets on which the stored month and that rule's fold differ
//!   (present in one only, or any field unequal).
//!
//! # Cost
//!
//! One read of the minute month and of each stored rung-month, and three
//! folds a rung: linear in the month, once per listed month. A measurement,
//! never on a request path.

use std::fmt::Write as _;
use std::path::Path;

use brutex_core::vendor::Vendor;
use pull::fold::{Bucket, PaddedFold, derive_minutes_with};
use store::format::Bar;
use store::path::{Timeframe, YearMonth};

use crate::fold_audit::{DERIVED_RUNGS, read_month};

/// The three readings, in the order every row prints them.
const RULES: [PaddedFold; 3] = [
    PaddedFold::AsTrade,
    PaddedFold::KeepFlat,
    PaddedFold::Absent,
];

/// The `padded` cell of a series the padded rule does not judge (an index,
/// a future): never `0`, which would read as "none found".
const NOT_CASH: &str = "not_cash";

/// The rows' header.
const HEADER: &str = "symbol\tmonth\trung\tpadded\tflat0\tbuckets\topen_moved\trange_moved\tonly_padded\tstored\tdiff_as_trade\tdiff_keep_flat\tdiff_absent";

/// One rung-month, measured.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RungMeasure {
    /// Bars the fold before INT-15 writes.
    pub(crate) buckets: u64,
    /// Of them, bars whose open the new reading moves.
    pub(crate) open_moved: u64,
    /// Of them, bars whose high or low the new reading moves.
    pub(crate) range_moved: u64,
    /// Buckets of padded minutes only.
    pub(crate) only_padded: u64,
    /// Bars stored, when the store holds a readable month.
    pub(crate) stored: Option<u64>,
    /// Buckets the stored month and each rule's fold differ on ([`RULES`]).
    pub(crate) differs: [u64; 3],
}

/// Buckets present in one of `a` and `b` only, or present in both with any
/// field unequal. Both are in strictly increasing stamp order, as a fold and
/// a committed month are.
#[must_use]
pub(crate) fn differing(left: &[Bar], right: &[Bar]) -> u64 {
    let (mut at_left, mut at_right, mut count) = (0, 0, 0_u64);
    loop {
        match (left.get(at_left), right.get(at_right)) {
            (Some(held), Some(other)) if held.ts_micros == other.ts_micros => {
                count += u64::from(held != other);
                at_left += 1;
                at_right += 1;
            }
            (Some(held), Some(other)) if held.ts_micros < other.ts_micros => {
                count += 1;
                at_left += 1;
            }
            (Some(_) | None, Some(_)) => {
                count += 1;
                at_right += 1;
            }
            (Some(_), None) => {
                count += 1;
                at_left += 1;
            }
            (None, None) => return count,
        }
    }
}

/// Of the stamps `before` and `after` share, how many bars `moved` says
/// changed.
fn moved(before: &[Bar], after: &[Bar], moved: impl Fn(&Bar, &Bar) -> bool) -> u64 {
    let mut at = 0;
    let mut count = 0_u64;
    for old in before {
        while after
            .get(at)
            .is_some_and(|new| new.ts_micros < old.ts_micros)
        {
            at += 1;
        }
        if let Some(new) = after.get(at).filter(|new| new.ts_micros == old.ts_micros) {
            count += u64::from(moved(old, new));
        }
    }
    count
}

/// One rung of a month under the three readings, against its stored month.
///
/// # Errors
///
/// The fold's refusal, in its own words.
pub(crate) fn measure_rung(
    minutes: &[Bar],
    stored: Option<&[Bar]>,
    rung: Timeframe,
    venue: pull::vendor::Venue,
    schedule: Option<&pull::cash_auction::Schedule>,
) -> Result<RungMeasure, String> {
    let bucket =
        Bucket::of_secs(rung.secs()).ok_or_else(|| format!("{} has no bucket", rung.as_str()))?;
    let mut folds: [Vec<Bar>; 3] = Default::default();
    for (slot, rule) in folds.iter_mut().zip(RULES) {
        *slot = derive_minutes_with(
            minutes,
            bucket,
            venue,
            schedule,
            pull::calendar::Runtime::default(),
            rule,
        )
        .map_err(|why| why.to_string())?
        .bars;
    }
    let [as_trade, keep_flat, absent] = &folds;
    let mut differs = [0_u64; 3];
    if let Some(stored) = stored {
        for (slot, fold) in differs.iter_mut().zip(&folds) {
            *slot = differing(stored, fold);
        }
    }
    Ok(RungMeasure {
        buckets: u64::try_from(as_trade.len()).unwrap_or(u64::MAX),
        open_moved: moved(as_trade, keep_flat, |old, new| old.open != new.open),
        range_moved: moved(as_trade, keep_flat, |old, new| {
            old.high != new.high || old.low != new.low
        }),
        only_padded: u64::try_from(keep_flat.len().saturating_sub(absent.len()))
            .unwrap_or(u64::MAX),
        stored: stored.map(|bars| u64::try_from(bars.len()).unwrap_or(u64::MAX)),
        differs,
    })
}

/// The totals the last lines print.
#[derive(Debug, Default)]
struct Totals {
    months: u64,
    refused: u64,
    padded: u64,
    flat0: u64,
    rungs: u64,
    rungs_stored: u64,
    agree: [u64; 3],
    measure: RungMeasure,
}

impl Totals {
    fn add(&mut self, m: &RungMeasure) {
        let t = &mut self.measure;
        t.buckets += m.buckets;
        t.open_moved += m.open_moved;
        t.range_moved += m.range_moved;
        t.only_padded += m.only_padded;
        self.rungs += 1;
        if let Some(stored) = m.stored {
            self.rungs_stored += 1;
            t.stored = Some(t.stored.unwrap_or(0) + stored);
            for ((sum, agree), diff) in t.differs.iter_mut().zip(&mut self.agree).zip(m.differs) {
                *sum += diff;
                *agree += u64::from(diff == 0);
            }
        }
    }
}

/// `SYMBOL YYYY-MM` lines, numbered refusals for any other.
fn parse_list(text: &str) -> Result<Vec<(String, YearMonth)>, String> {
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let refuse = || format!("LIST line {}: expected `SYMBOL YYYY-MM`", index + 1);
        let mut words = line.split_whitespace();
        let (Some(symbol), Some(month), None) = (words.next(), words.next(), words.next()) else {
            return Err(refuse());
        };
        let (year, number) = month.split_once('-').ok_or_else(refuse)?;
        let ym = year
            .parse::<u16>()
            .ok()
            .zip(number.parse::<u8>().ok())
            .and_then(|(y, m)| YearMonth::new(y, m).ok())
            .ok_or_else(refuse)?;
        out.push((symbol.to_owned(), ym));
    }
    Ok(out)
}

/// Every rung of one listed month, appended to `out` as rows.
fn measure_month(
    root: &Path,
    vendor: Vendor,
    symbol: &str,
    ym: YearMonth,
    out: &mut String,
    totals: &mut Totals,
) -> Result<(), String> {
    let key = crate::stored::swept_index(symbol)?;
    let venue = pull::vendor::Venue::for_segment(key.exchange, key.segment)
        .ok_or_else(|| format!("{symbol}: no verified minute-completeness venue"))?;
    let minutes = read_month(root, vendor, &key, Timeframe::MINUTE_1, ym)?;
    // THE SAME DATED CASH SESSION THE DERIVE FOLDED UNDER, from the local
    // receipted masters: without it a cash day from 2026-08-03 is withheld.
    let days = minutes.iter().map(|bar| {
        (bar.ts_micros.div_euclid(1_000_000) + pull::session::IST_OFFSET_SECS).div_euclid(86_400)
    });
    let closes = crate::stored::load_cash_closes(root, symbol, days)?;
    let schedule = (venue == pull::vendor::Venue::NseCash).then(|| closes.schedule());
    // A SERIES THE RULE DOES NOT JUDGE IS NAMED, NEVER COUNTED AS ZERO
    // (verify-R4-27-r1 follow-up): an index or a future has no padded minute
    // by definition, which is not "none found", so its cell reads `not_cash`
    // and the total counts nothing for it.
    let padded = match pull::padded::per_day(
        &minutes,
        venue,
        schedule,
        pull::calendar::Runtime::default(),
    ) {
        Ok(days) => Some(days.iter().map(|day| u64::from(day.padded)).sum::<u64>()),
        Err(pull::padded::Uncounted::NotCash(_)) => None,
        Err(why) => return Err(why.to_string()),
    };
    let flat0 = minutes
        .iter()
        .filter(|b| b.volume == 0 && b.open == b.high && b.high == b.low && b.low == b.close)
        .count();
    let flat0 = u64::try_from(flat0).unwrap_or(u64::MAX);
    totals.months += 1;
    totals.padded += padded.unwrap_or(0);
    let padded = padded.map_or_else(|| NOT_CASH.to_owned(), |count| count.to_string());
    totals.flat0 += flat0;
    for rung in DERIVED_RUNGS {
        let stored = read_month(root, vendor, &key, rung, ym).ok();
        let m = measure_rung(&minutes, stored.as_deref(), rung, venue, schedule)?;
        totals.add(&m);
        let stored = m.stored.map_or_else(|| "-".to_owned(), |n| n.to_string());
        let _ = writeln!(
            out,
            "{symbol}\t{ym}\t{}\t{padded}\t{flat0}\t{}\t{}\t{}\t{}\t{stored}\t{}\t{}\t{}",
            rung.as_str(),
            m.buckets,
            m.open_moved,
            m.range_moved,
            m.only_padded,
            m.differs[0],
            m.differs[1],
            m.differs[2],
        );
    }
    Ok(())
}

/// `cli padded-measure FEED LIST`.
pub(crate) fn padded(feed: &str, list: &str, out: &mut String) -> u8 {
    let vendor = match crate::parse_vendor(feed) {
        Ok(vendor) => vendor,
        Err(why) => return crate::refuse(out, &why),
    };
    let text = match std::fs::read_to_string(list) {
        Ok(text) => text,
        Err(why) => return crate::fail(out, &format!("LIST {list}: {why}")),
    };
    let months = match parse_list(&text) {
        Ok(months) => months,
        Err(why) => return crate::refuse(out, &why),
    };
    let root = match crate::store_root() {
        Ok(root) => root,
        Err(why) => return crate::fail(out, &why),
    };
    let _ = writeln!(
        out,
        "PADDED MINUTES -- both fold treatments measured over {} listed month(s) of feed {feed} \
         (INT-15, R4-27). Reads only.\n{HEADER}",
        months.len()
    );
    let mut totals = Totals::default();
    for (symbol, ym) in &months {
        if let Err(why) = measure_month(&root, vendor, symbol, *ym, out, &mut totals) {
            totals.refused += 1;
            let _ = writeln!(out, "{symbol}\t{ym}\trefused: {why}");
        }
    }
    let t = &totals.measure;
    let _ = writeln!(
        out,
        "TOTAL\tmonths {} (refused {})\tpadded {}\tflat0 {}\trung-months {} (stored {})\tbuckets {}\topen_moved {}\trange_moved {}\tonly_padded {}\tdiff as_trade/keep_flat/absent {}/{}/{}\tagree as_trade/keep_flat/absent {}/{}/{}",
        totals.months,
        totals.refused,
        totals.padded,
        totals.flat0,
        totals.rungs,
        totals.rungs_stored,
        t.buckets,
        t.open_moved,
        t.range_moved,
        t.only_padded,
        t.differs[0],
        t.differs[1],
        t.differs[2],
        totals.agree[0],
        totals.agree[1],
        totals.agree[2],
    );
    if totals.refused > 0 {
        crate::FAILED
    } else {
        crate::OK
    }
}

/// The staleness bounds the rate decision compared, in calendar days.
pub(crate) const RATE_BOUNDS: [i32; 3] = [14, 21, 28];

/// `YYYY-MM-DD` as a trade day, or why it is not one: not written that way,
/// not a real calendar day, or a year outside the trade-day window, each in
/// its own words (verify-R4-27-r1 follow-up: all three read "must be days
/// written YYYY-MM-DD").
fn civil_day(text: &str) -> Result<pull::pricing::TradeDay, String> {
    let mut parts = text.split('-');
    let written = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d), None) => y
            .parse::<u16>()
            .ok()
            .zip(m.parse::<u8>().ok())
            .zip(d.parse::<u8>().ok()),
        _ => None,
    };
    let Some(((year, month), day)) = written else {
        return Err(format!("{text:?} is not a day written YYYY-MM-DD"));
    };
    pull::pricing::TradeDay::new(year, month, day).map_err(|why| format!("{text}: {why}"))
}

/// What the calendar says of every day from `from` to `to`: its open
/// sessions (a special session included), in order, and the days it cannot
/// classify, outside its measured window or open with no stated length.
/// Those are never replayed and never read as days with no session: the
/// replay names them (verify-R4-27-r1 follow-up).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Days {
    /// The open sessions, in order.
    pub(crate) sessions: Vec<pull::pricing::TradeDay>,
    /// How many days the calendar cannot classify.
    pub(crate) unclassified: u32,
    /// The first and the last of them.
    pub(crate) unclassified_span: Option<(pull::pricing::TradeDay, pull::pricing::TradeDay)>,
}

/// [`Days`] from `from` to `to`, one calendar lookup a day.
#[must_use]
pub(crate) fn sessions_between(from: pull::pricing::TradeDay, to: pull::pricing::TradeDay) -> Days {
    let runtime = pull::calendar::Runtime::default();
    let mut days = Days {
        sessions: Vec::new(),
        unclassified: 0,
        unclassified_span: None,
    };
    for ordinal in from.ordinal()..=to.ordinal() {
        let Ok(day) = pull::pricing::TradeDay::from_ordinal(ordinal) else {
            continue;
        };
        match runtime.kind_of(i64::from(ordinal)) {
            pull::calendar::DayKind::Open(_) => days.sessions.push(day),
            pull::calendar::DayKind::Unmeasured | pull::calendar::DayKind::OpenLengthUnmeasured => {
                days.unclassified = days.unclassified.saturating_add(1);
                let first = days.unclassified_span.map_or(day, |(first, _)| first);
                days.unclassified_span = Some((first, day));
            }
            pull::calendar::DayKind::Closed => {}
        }
    }
    days
}

/// The line a replay prints for the days its calendar cannot classify, or
/// `None` when it classifies every day it was asked.
fn unclassified_line(days: &Days) -> Option<String> {
    let (first, last) = days.unclassified_span?;
    Some(format!(
        "unclassified {} day(s) {first}..{last}: outside the calendar's measured window or open with no stated length; not replayed, and not counted as days with no session",
        days.unclassified
    ))
}

/// `cli rate-replay TABLE FROM TO`.
pub(crate) fn rate_replay(table: &str, from: &str, to: &str, out: &mut String) -> u8 {
    let (first, last) = match (civil_day(from), civil_day(to)) {
        (Ok(first), Ok(last)) => (first, last),
        (Err(why), _) => return crate::refuse(out, &format!("FROM {why}")),
        (_, Err(why)) => return crate::refuse(out, &format!("TO {why}")),
    };
    if last.ordinal() < first.ordinal() {
        return crate::refuse(out, "TO is before FROM");
    }
    let text = match std::fs::metadata(table).and_then(|meta| {
        if meta.len() > pull::riskfree::MAX_FILE_BYTES {
            Err(std::io::Error::other("larger than a rate table can be"))
        } else {
            std::fs::read_to_string(table)
        }
    }) {
        Ok(text) => text,
        Err(why) => return crate::fail(out, &format!("TABLE {table}: {why}")),
    };
    let series = match pull::riskfree::Series::parse(&text) {
        Ok(series) => series,
        Err(why) => return crate::fail(out, &format!("TABLE {table}: {why}")),
    };
    let days = sessions_between(first, last);
    let sessions = &days.sessions;
    let mut digest = String::with_capacity(64);
    for byte in series.sha256() {
        let _ = write!(digest, "{byte:02x}");
    }
    let _ = writeln!(
        out,
        "RATE REPLAY -- {table} (sha256 {digest}; {} results, {} documented gaps) over {} \
         sessions {from}..{to} (AUT-10 decision 2, R4-27). Reads only.",
        series.auction_count(),
        series.gap_count(),
        sessions.len()
    );
    if let Some(line) = unclassified_line(&days) {
        let _ = writeln!(out, "{line}");
    }
    for bound in RATE_BOUNDS {
        match pull::riskfree::replay(&text, bound, sessions.iter().copied()) {
            Ok(replayed) => {
                let _ = writeln!(
                    out,
                    "bound {bound} days\tfresh {}\tcarried {}\trefused {}",
                    replayed.fresh,
                    replayed.carried,
                    replayed.refused.len()
                );
                for (day, code) in &replayed.refused {
                    let _ = writeln!(out, "  refused {day} {code}");
                }
            }
            Err(why) => return crate::fail(out, &why.to_string()),
        }
    }
    crate::OK
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;
    use store::format::OI_NULL;

    fn bar(ts: i64, price: i64) -> Bar {
        Bar {
            ts_micros: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 1,
            open_interest: OI_NULL,
        }
    }

    #[test]
    fn differing_counts_every_bucket_held_by_one_side_or_unequal() {
        let left = [bar(1, 10), bar(2, 20), bar(4, 40)];
        let right = [bar(2, 20), bar(3, 30), bar(4, 41), bar(5, 50)];
        // 1 only on the left, 3 and 5 only on the right, 4 unequal; 2 equal.
        assert_eq!(differing(&left, &right), 4);
        assert_eq!(differing(&right, &left), 4);
        assert_eq!(differing(&left, &left), 0);
        assert_eq!(differing(&[], &left), 3);
        assert_eq!(differing(&left, &[]), 3);
        assert_eq!(moved(&left, &right, |old, new| old.open != new.open), 1);
    }

    #[test]
    fn the_list_is_symbol_and_month_a_line_and_refused_by_line() {
        let parsed = parse_list("# comment\n\nPGEL 2019-03\n RELIANCE 2026-08 \n").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "PGEL");
        assert_eq!(parsed[1].1, YearMonth::new(2026, 8).unwrap());
        for bad in [
            "PGEL",
            "PGEL 2019-3x",
            "PGEL 2019-13",
            "PGEL 2019-03 extra",
            "PGEL 201903",
        ] {
            let why = parse_list(bad).unwrap_err();
            assert!(why.starts_with("LIST line 1: "), "{bad}: {why}");
        }
    }

    #[test]
    fn a_replay_reads_the_calendar_sessions_and_refuses_garbage_days() {
        assert!(civil_day("2024-9-26").is_ok());
        for bad in ["", "2024-09", "2024-09-31", "2024-09-26-1", "x-1-1"] {
            assert!(civil_day(bad).is_err(), "{bad}");
        }
        // 2024-03-01 (Fri), 03-02 (the DR Saturday, a special session) and
        // 03-04 (Mon) are sessions; 03-03 is a Sunday.
        let days = sessions_between(
            civil_day("2024-03-01").unwrap(),
            civil_day("2024-03-04").unwrap(),
        );
        assert_eq!(
            days.sessions,
            vec![
                civil_day("2024-03-01").unwrap(),
                civil_day("2024-03-02").unwrap(),
                civil_day("2024-03-04").unwrap()
            ]
        );
        assert_eq!(days.unclassified, 0);
        assert_eq!(unclassified_line(&days), None);
        let mut out = String::new();
        assert_eq!(
            rate_replay("t", "2024-03-04", "2024-03-01", &mut out),
            crate::MISUSED
        );
        assert!(out.starts_with("refused: TO is before FROM"), "{out}");
        let mut out = String::new();
        assert_eq!(
            rate_replay("t", "2024-03-01", "nope", &mut out),
            crate::MISUSED
        );
        assert!(
            out.starts_with("refused: TO \"nope\" is not a day written YYYY-MM-DD"),
            "{out}"
        );
        let mut out = String::new();
        let missing = std::env::temp_dir().join("brutex-no-such-rate-table");
        assert_eq!(
            rate_replay(
                missing.to_str().unwrap(),
                "2024-03-01",
                "2024-03-04",
                &mut out
            ),
            crate::FAILED
        );
        assert!(out.starts_with("refused: TABLE "), "{out}");
    }

    /// **A DAY THAT IS NOT ONE IS REFUSED IN ITS OWN WORDS** (verify-R4-27-r1
    /// follow-up): an impossible date, a year outside the trade-day window and
    /// a day not written `YYYY-MM-DD` each say which they are; all three read
    /// "must be days written YYYY-MM-DD" before.
    #[test]
    fn a_day_that_is_not_one_is_refused_in_its_own_words() {
        let impossible = civil_day("2018-02-30").unwrap_err();
        assert!(impossible.starts_with("2018-02-30: "), "{impossible}");
        assert!(
            impossible.contains("not a real calendar date"),
            "{impossible}"
        );
        for outside in ["1900-01-01", "9999-12-31"] {
            let why = civil_day(outside).unwrap_err();
            assert!(why.contains("is outside 1990..=2100"), "{outside}: {why}");
        }
        assert_eq!(
            civil_day("2018/02/03").unwrap_err(),
            "\"2018/02/03\" is not a day written YYYY-MM-DD"
        );
        let mut out = String::new();
        assert_eq!(
            rate_replay("t", "1900-01-01", "2024-03-04", &mut out),
            crate::MISUSED
        );
        assert!(
            out.starts_with("refused: FROM 1900-01-01: year 1900 is outside"),
            "{out}"
        );
    }

    /// **THE DAYS THE CALENDAR CANNOT CLASSIFY ARE NAMED, NEVER READ AS NO
    /// SESSION** (verify-R4-27-r1 follow-up): 2000-01-03..10 lies before the
    /// calendar's measured window; the replay answered "over 0 sessions" and
    /// said nothing else.
    #[test]
    fn the_days_the_calendar_cannot_classify_are_named() {
        let days = sessions_between(
            civil_day("2000-01-03").unwrap(),
            civil_day("2000-01-10").unwrap(),
        );
        assert!(days.sessions.is_empty());
        assert_eq!(days.unclassified, 8);
        assert_eq!(
            days.unclassified_span,
            Some((
                civil_day("2000-01-03").unwrap(),
                civil_day("2000-01-10").unwrap()
            ))
        );
        let line = unclassified_line(&days).unwrap();
        assert!(
            line.starts_with("unclassified 8 day(s) 2000-01-03..2000-01-10: "),
            "{line}"
        );
        assert!(line.contains("not replayed"), "{line}");
    }

    /// **AN INDEX MONTH IS `not_cash`, NEVER `0` PADDED MINUTES**
    /// (verify-R4-27-r1 follow-up): the rule does not judge an index, which
    /// is not the same answer as "none found", and the total counts nothing
    /// for it.
    #[test]
    fn an_index_month_reads_not_cash_and_counts_nothing() {
        let root = std::env::temp_dir().join(format!(
            "brutex-padded-measure-index-{}",
            std::process::id()
        ));
        let _fresh = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let key = crate::stored::swept_index("NIFTY").unwrap();
        let ym = YearMonth::new(2024, 6).unwrap();
        let path = store::path::StorePath::for_key(
            Vendor::Zerodha,
            &key,
            Timeframe::MINUTE_1,
            ym,
            store::path::FileKind::Bars,
        )
        .unwrap();
        let hash = brutex_core::universe::fnv1a("NIFTY").to_le_bytes();
        let symbol = u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]);
        // 2024-06-03 09:15 IST onward, flat and silent: padded-looking, and
        // still not judged, because an index does not trade.
        let minutes: Vec<Bar> = (0..30)
            .map(|minute| Bar {
                ts_micros: 1_717_386_300_000_000 + minute * 60_000_000,
                volume: 0,
                ..bar(0, 2_200_000)
            })
            .collect();
        store::file::BarFile::open_or_create(&root, path, symbol)
            .unwrap()
            .append(&minutes)
            .unwrap();
        let mut out = String::new();
        let mut totals = Totals::default();
        measure_month(&root, Vendor::Zerodha, "NIFTY", ym, &mut out, &mut totals).unwrap();
        let _cleanup = std::fs::remove_dir_all(&root);
        let rows: Vec<&str> = out.lines().collect();
        assert_eq!(rows.len(), DERIVED_RUNGS.len(), "{out}");
        for row in rows {
            let cells: Vec<&str> = row.split('\t').collect();
            assert_eq!(cells[3], NOT_CASH, "{row}");
        }
        assert_eq!(totals.padded, 0);
        assert_eq!(totals.months, 1);
    }
}
