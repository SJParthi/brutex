//! Days whose one-minute series has a hole, and what withholding them costs.
//!
//! # The refusal this removes, and it is not the charter's
//!
//! `SWEPT_SERIES_CALENDAR_POLICY` withholds the nine days `docs/00-charter.md`
//! names as non-regular sessions. This is a different fault with the same shape:
//! an ORDINARY trading day whose one-minute series is missing a minute the
//! vendor never served.
//!
//! MEASURED on the operator's freshly pulled zerodha NIFTY store, by decoding
//! every one-minute timestamp of all 82 months and looking for a gap inside a
//! session — **28 minutes across 12 days**:
//!
//! | IST day | date | minutes lost |
//! |---|---|---|
//! | 18 305 | 2020-02-13 | 1 |
//! | 18 325 | 2020-03-04 | 1 |
//! | 18 501 | 2020-08-27 | 2 |
//! | 18 568 | 2020-11-02 | 1 |
//! | 18 981 | 2021-12-20 | 1 |
//! | 19 058 | 2022-03-07 | 3 |
//! | 19 513 | 2023-06-05 | 1 |
//! | 19 522 | 2023-06-14 | 7 |
//! | 19 550 | 2023-07-12 | 1 |
//! | 19 562 | 2023-07-24 | 3 |
//! | 19 576 | 2023-08-07 | 1 |
//! | 19 836 | 2024-04-23 | 6 |
//!
//! A coarse rung's bucket closes on a minute that must exist:
//! `overlay_exact_minute_gapfib` demands the minute opening at the signal bar's
//! close and refuses `MissingClosingMinute` when it is absent. One such minute
//! anywhere in the span refuses the WHOLE span, so six of the eight rungs could
//! not run over 2020-01..2026-07 at all — for 28 minutes in 618,000.
//!
//! # Re-pulling does not fix it, which is why this exists
//!
//! MEASURED: the operator wiped the store and pulled zerodha from scratch. The
//! same 28 minutes came back missing. `crates/pull/src/gaps.rs` names the reason
//! for a `VendorHole` — *"It cannot be repaired from the same vendor: a re-run
//! returns the same nothing"* — and 2022-03-07 is absent from dhan too, so that
//! one is exchange-side and unrecoverable from any feed.
//!
//! # Why withholding the DAY and not substituting the minute
//!
//! Four tests exist to stop exactly that substitution —
//! `exact_minute_overlay_never_substitutes_a_later_minute` and its three
//! siblings — and their reasoning is right: a minute other sessions hold,
//! missing from this one, is a hole, and papering over it is what `CLAUDE.md`
//! §4 forbids. **None of them is weakened here.** A hole still refuses when its
//! day is swept. What changes is that the day is not swept: it is removed from
//! the sample, counted, named by date, and bound into the run identity, exactly
//! as a charter session is.
//!
//! The distinction that makes this honest rather than a fallback: substituting
//! would answer a question with the wrong bar. Withholding declines to answer
//! it, out loud.
//!
//! # Measured, never written down
//!
//! The twelve days above are evidence, not a table. [`days_with_interior_gaps`]
//! derives them from the minute stream it is handed, so a re-pull that recovers
//! a minute silently stops withholding that day, and a new hole is caught
//! without an edit. A hardcoded list would be the invention `CLAUDE.md` §3
//! rule 1 forbids and would rot the first time the store changed.
//!
//! # Cost
//!
//! One pass over the minute slice comparing each bar to its predecessor —
//! O(minutes), the same order as the load that produced it, with no allocation
//! beyond the day list. Withholding is one pass per slice. Neither is one of the
//! five operations §3 rule 4 bounds. **UNVERIFIED as a measured bound**; read
//! off the source per §3 rule 6.

use indicators::Candle;

/// Version of the rule that keeps a holed session out of a swept series.
///
/// # Why a term of its own beside `SWEPT_SERIES_CALENDAR_POLICY`
///
/// The two withhold for different reasons and can change independently. The
/// charter policy answers *"is this session non-regular?"* from a fixed list of
/// exchange facts; this answers *"can this day's minutes price a coarse bucket's
/// close?"* from the bytes actually on disk. A build could change either without
/// the other, and under one version number a later change to this would be
/// indistinguishable from a change to that.
///
/// TWO since D-1662: the census also withholds a day whose demanded closing
/// minute is absent ([`days_with_minute_holes`]), so a holed span withholds
/// more days than version 1 did and must not share its identity.
pub const MINUTE_GAP_POLICY: u32 = 2;

/// One minute, in microseconds.
const MINUTE_MICROS: i64 = 60_000_000;

/// Which days a run withheld for a minute gap, and what they cost it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GapExclusion {
    /// IST days withheld, ascending. Empty on a gap-free span.
    days: Vec<i64>,
    /// Signal bars removed.
    signal_bars: u64,
    /// Exact one-minute bars removed.
    minute_bars: u64,
}

impl GapExclusion {
    /// Nothing was withheld.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            days: Vec::new(),
            signal_bars: 0,
            minute_bars: 0,
        }
    }

    /// What a door that withholds from its SIGNAL bars alone removed, on a
    /// coarse rung. D-0694.
    ///
    /// `sweep-stored`, like `auto-stored`, `audit-range`, `screen` and `pool`,
    /// removes the holed `days` from the signal series with [`withhold`]. On a
    /// coarse rung it leaves its separately loaded execution minutes whole:
    /// with no signal bar on a withheld day, none of that day's minutes is
    /// ever looked up. So no minute bar was removed, and this says so rather
    /// than counting one. On `1min` there is no separate series, and
    /// [`Self::one_series`] is the answer.
    #[must_use]
    pub const fn signal_only(days: Vec<i64>, signal_bars: u64) -> Self {
        Self {
            days,
            signal_bars,
            minute_bars: 0,
        }
    }

    /// What a door removed from a series that is both its signal and its
    /// execution: the `1min` rung. D-0694.
    ///
    /// There the signal bars ARE the one-minute execution bars, so every bar
    /// [`withhold`] removed is a minute bar too, and the day leaves the
    /// execution as well. [`Self::signal_only`] recorded zero minute bars
    /// here, which said the execution minutes stayed whole when they did not.
    #[must_use]
    pub const fn one_series(days: Vec<i64>, bars: u64) -> Self {
        Self {
            days,
            signal_bars: bars,
            minute_bars: bars,
        }
    }

    /// How many days were withheld.
    #[must_use]
    pub fn days(&self) -> usize {
        self.days.len()
    }

    /// The withheld IST days, ascending.
    #[must_use]
    pub fn day_numbers(&self) -> &[i64] {
        &self.days
    }

    /// Signal bars removed.
    #[must_use]
    pub const fn signal_bars(&self) -> u64 {
        self.signal_bars
    }

    /// Exact one-minute bars removed.
    #[must_use]
    pub const fn minute_bars(&self) -> u64 {
        self.minute_bars
    }

    /// Whether this run withheld nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.days.is_empty()
    }

    /// The withheld days as `YYYY-MM-DD`, ascending.
    ///
    /// Named and not merely counted, for the reason `CLAUDE.md` §3 rule 2
    /// gives: an operator reading "twelve days were withheld" cannot check that
    /// against anything, and a day that will not convert falls back to its raw
    /// number rather than being dropped.
    #[must_use]
    pub fn day_names(&self) -> Vec<String> {
        self.days
            .iter()
            .map(|day| {
                u32::try_from(*day)
                    .ok()
                    .and_then(|days| pull::session::Day::from_days(days).ok())
                    .map_or_else(|| day.to_string(), |date| date.to_string())
            })
            .collect()
    }
}

/// Days whose one-minute series skips a minute inside a session.
///
/// A gap is a pair of ADJACENT bars on the SAME IST day more than one minute
/// apart. The same-day test is what makes this a hole rather than a night: the
/// step from 15:29 to the next morning's 09:15 is enormous and is not a fault.
///
/// The result is ascending and holds each day once.
#[must_use]
pub fn days_with_interior_gaps(minutes: &[Candle]) -> Vec<i64> {
    let mut days: Vec<i64> = Vec::new();
    let mut previous: Option<&Candle> = None;
    for bar in minutes {
        if let Some(before) = previous {
            let day = indicators::ist_day(bar.ts_micros);
            let same_day = indicators::ist_day(before.ts_micros) == day;
            let step = bar.ts_micros.saturating_sub(before.ts_micros);
            if same_day && step > MINUTE_MICROS && days.last() != Some(&day) {
                days.push(day);
            }
        }
        previous = Some(bar);
    }
    // The scan visits bars in time order, so `days` is already ascending and
    // `last() != day` suppressed the repeats. Sorting would be a second claim
    // about an order the input already guarantees.
    days
}

/// Days a stored span must withhold before its column is built: every day
/// [`days_with_interior_gaps`] finds, and every day holding a signal bar whose
/// closing minute the exact-minute overlay will demand and the minute stream
/// does not hold. W2-cli9-3, D-1662.
///
/// # The holes the interior walk could not see
///
/// The interior walk compares adjacent minutes on the SAME day, so a session
/// that stops early (15:25 to 15:29 missing) steps from its last minute to the
/// next morning's 09:15, fails the same-day test, and is never flagged. A day
/// holding signal bars and no minutes at all is invisible to it for the same
/// reason. The overlay then refused `MissingClosingMinute` for that day's last
/// bucket, `pool` and `screen` refused the whole span, and `audit-range` found
/// the days one refusal at a time, reloading both contexts and rebuilding the
/// column per day (W2-cli8-6).
///
/// # Asking the overlay's own question
///
/// For each signal bar this computes the minute
/// [`indicators::anchored::exact_closing_minute`] says the overlay will demand,
/// with `session_close`, which every caller takes from
/// [`crate::stored::session_close_for`] -- the same close the stored column
/// passes the overlay, dated per day for a cash share (D-2102) -- and checks
/// that exact stamp is held. So the census
/// and the join cannot disagree about which day is unsourceable, the retry
/// loops behind it become a defence, and a day the overlay would not refuse is
/// not withheld: a `1min` rung whose session ends early has no signal bar
/// demanding the missing minutes, and nothing here invents one.
///
/// # Cost
///
/// O(signal + minutes) for d flagged days: one interior pass, one cursor walk
/// that only moves forward because the demanded minute is non-decreasing in
/// signal order, one `kind_of` lookup per signal bar (bounded by
/// `pull::calendar::MAX_WINDOWS`), and one O(d) merge of the two ascending
/// flagged-day lists (no sort).
/// **UNVERIFIED as a measured bound**; read off the source per `CLAUDE.md` §3
/// rule 6.
#[must_use]
pub fn days_with_minute_holes(
    signal: &[Candle],
    minutes: &[Candle],
    signal_length_micros: i64,
    session_close: impl Fn(i64) -> Option<u16>,
) -> Vec<i64> {
    let interior = days_with_interior_gaps(minutes);
    let mut edges: Vec<i64> = Vec::new();
    let mut cursor = 0_usize;
    for bar in signal {
        let day = indicators::ist_day(bar.ts_micros);
        let expected = indicators::anchored::exact_closing_minute(
            bar.ts_micros,
            signal_length_micros,
            session_close(day),
        );
        while minutes
            .get(cursor)
            .is_some_and(|minute| minute.ts_micros < expected)
        {
            cursor = cursor.saturating_add(1);
        }
        if minutes
            .get(cursor)
            .is_none_or(|minute| minute.ts_micros != expected)
            && edges.last() != Some(&day)
        {
            edges.push(day);
        }
    }
    merge_ascending(&interior, &edges)
}

/// The ascending union of two ascending day lists, each day once.
///
/// Both inputs are built by a forward walk over time-ordered bars, so each is
/// already ascending; a two-cursor merge keeps that order in O(a + b) without
/// the sort gate 11 rule 4 refuses on this path.
fn merge_ascending(left: &[i64], right: &[i64]) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::with_capacity(left.len().saturating_add(right.len()));
    let (mut l, mut r) = (left.iter().peekable(), right.iter().peekable());
    loop {
        let next = match (l.peek(), r.peek()) {
            (Some(&&a), Some(&&b)) if a <= b => l.next().copied(),
            (Some(_) | None, Some(_)) => r.next().copied(),
            (Some(_), None) => l.next().copied(),
            (None, None) => break,
        };
        if let Some(day) = next
            && out.last() != Some(&day)
        {
            out.push(day);
        }
    }
    out
}

/// UNVERIFIED performance: no named cost test or measured latency bound is established here.
/// Remove every bar falling on one of `days`, counting what went.
///
/// Returns the kept bars and the number removed.
///
/// # The membership test is a SET, and it was a slice scan
///
/// This read `days.contains(&ist_day(bar.ts_micros))` inside `for bar in bars`,
/// justified by a measurement: *"twelve on the operator's whole 6.5-year span,
/// so membership is a linear scan over a handful of `i64`, not a set."*
///
/// Twelve is what the operator's store happens to hold, not a bound.
/// [`days_with_interior_gaps`] caps nothing — it appends a day for every
/// interior hole it finds — so a feed with a hole in most sessions yields a
/// `days` proportional to the span and the loop becomes `O(bars × days)`. On a
/// 609,722-bar minute series a three-hundred-day list is 1.8 × 10⁸ compares,
/// and nothing anywhere refuses that list.
///
/// A measurement is not a bound, and a comment saying "expected short" is not
/// an argument that it must be. The set costs one allocation of the same
/// handful of `i64` and removes the multiplication entirely, so the O(1)
/// membership holds however many days are handed in.
///
/// `days` is still expected ascending; nothing here needs that, and nothing
/// here breaks if it is not.
#[must_use]
pub fn withhold(bars: &[Candle], days: &[i64]) -> (Vec<Candle>, u64) {
    if days.is_empty() {
        return (bars.to_vec(), 0);
    }
    let withheld: std::collections::HashSet<i64> = days.iter().copied().collect();
    let mut kept: Vec<Candle> = Vec::with_capacity(bars.len());
    let mut removed = 0_u64;
    for bar in bars {
        if withheld.contains(&indicators::ist_day(bar.ts_micros)) {
            removed = removed.saturating_add(1);
        } else {
            kept.push(*bar);
        }
    }
    (kept, removed)
}

/// Withhold the holed days from a signal series and its exact-minute stream.
///
/// **Both slices are filtered by ONE measured day set, and that is the whole
/// correctness argument.** The set is derived from the minute stream, because
/// the minute stream is what the overlay indexes; filtering the signal by a
/// different set would leave a signal bar whose closing minute had been removed,
/// which is the refusal this exists to prevent, arriving by a new route.
#[must_use]
pub fn withhold_holed_days(
    signal: &[Candle],
    minutes: &[Candle],
) -> (Vec<Candle>, Vec<Candle>, GapExclusion) {
    let days = days_with_interior_gaps(minutes);
    if days.is_empty() {
        return (signal.to_vec(), minutes.to_vec(), GapExclusion::none());
    }
    let (kept_signal, signal_bars) = withhold(signal, &days);
    let (kept_minutes, minute_bars) = withhold(minutes, &days);
    (
        kept_signal,
        kept_minutes,
        GapExclusion {
            days,
            signal_bars,
            minute_bars,
        },
    )
}

#[cfg(test)]
mod tests {

    /// `days_with_minute_holes` unions its two ascending lists by a merge, not
    /// a sort (gate 11 rule 4, D-1662): ascending, each day once, nothing
    /// dropped, on every interleaving including empty sides and full overlap.
    #[test]
    fn the_flagged_day_union_is_ascending_and_each_day_once() {
        let merge = super::merge_ascending;
        assert_eq!(merge(&[], &[]), Vec::<i64>::new());
        assert_eq!(merge(&[3], &[]), vec![3]);
        assert_eq!(merge(&[], &[3]), vec![3]);
        assert_eq!(merge(&[1, 4, 9], &[2, 4, 10]), vec![1, 2, 4, 9, 10]);
        assert_eq!(merge(&[5, 6], &[1, 2]), vec![1, 2, 5, 6]);
        assert_eq!(merge(&[1, 2], &[5, 6]), vec![1, 2, 5, 6]);
        assert_eq!(merge(&[7, 7], &[7]), vec![7]);
        assert_eq!(
            merge(&[i64::MIN, 0], &[0, i64::MAX]),
            vec![i64::MIN, 0, i64::MAX]
        );
        // Against a sort-and-dedup reference over a deterministic sweep.
        for seed in 0_i64..64 {
            let left: Vec<i64> = (0..seed % 7).map(|k| k * 3 + seed % 2).collect();
            let right: Vec<i64> = (0..seed % 5).map(|k| k * 2 + seed % 3).collect();
            let mut expected = [left.clone(), right.clone()].concat();
            expected.sort_unstable();
            expected.dedup();
            assert_eq!(merge(&left, &right), expected, "seed {seed}");
        }
    }
    use super::*;

    /// A one-minute bar `minute` minutes past 09:15 IST on `day`.
    fn bar_on(day: i64, minute: i64) -> Candle {
        const IST_OFFSET_MICROS: i64 = 19_800 * 1_000_000;
        const OPEN_MINUTE: i64 = 555;
        let ts = day
            .saturating_mul(86_400)
            .saturating_add(OPEN_MINUTE.saturating_mul(60))
            .saturating_mul(1_000_000)
            .saturating_sub(IST_OFFSET_MICROS)
            .saturating_add(minute.saturating_mul(MINUTE_MICROS));
        Candle {
            ts_micros: ts,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: i64::MIN,
        }
    }

    /// A contiguous session has no gap, and an overnight step is not one.
    #[test]
    fn a_contiguous_series_and_an_overnight_step_are_both_gap_free() {
        let mut bars: Vec<Candle> = (0..30).map(|m| bar_on(19_000, m)).collect();
        bars.extend((0..30).map(|m| bar_on(19_001, m)));
        assert!(
            days_with_interior_gaps(&bars).is_empty(),
            "the night between two sessions is not a hole"
        );
    }

    /// One missing minute names its day, once, however wide the hole.
    #[test]
    fn an_interior_hole_names_its_day_exactly_once() {
        let mut bars: Vec<Candle> = Vec::new();
        for m in 0..10 {
            bars.push(bar_on(19_522, m));
        }
        // 19_522 is 2023-06-14, where the operator's store loses seven minutes.
        for m in 18..30 {
            bars.push(bar_on(19_522, m));
        }
        assert_eq!(days_with_interior_gaps(&bars), vec![19_522]);

        // A second, separate hole on the same day still names it once.
        let mut twice: Vec<Candle> = Vec::new();
        for m in [0, 1, 5, 6, 20, 21] {
            twice.push(bar_on(19_522, m));
        }
        assert_eq!(days_with_interior_gaps(&twice), vec![19_522]);
    }

    /// Several holed days come back ascending, and clean days are untouched.
    #[test]
    fn every_holed_day_is_named_ascending_and_clean_days_are_not() {
        let mut bars: Vec<Candle> = Vec::new();
        // clean
        bars.extend((0..20).map(|m| bar_on(18_305, m)));
        // holed
        bars.extend([0, 1, 2, 9, 10].map(|m| bar_on(18_501, m)));
        // clean
        bars.extend((0..20).map(|m| bar_on(18_568, m)));
        // holed
        bars.extend([0, 7, 8].map(|m| bar_on(19_058, m)));
        assert_eq!(days_with_interior_gaps(&bars), vec![18_501, 19_058]);
    }

    /// Both slices lose the SAME days, which is the property the overlay needs.
    #[test]
    fn the_signal_and_the_minute_stream_lose_the_same_days() {
        let mut minutes: Vec<Candle> = Vec::new();
        minutes.extend((0..20).map(|m| bar_on(19_000, m)));
        minutes.extend([0, 1, 12, 13].map(|m| bar_on(19_001, m))); // holed
        minutes.extend((0..20).map(|m| bar_on(19_002, m)));

        // A five-minute signal series over the same three days.
        let mut signal: Vec<Candle> = Vec::new();
        for day in [19_000, 19_001, 19_002] {
            signal.extend((0..4).map(|b| bar_on(day, b * 5)));
        }

        let (kept_signal, kept_minutes, excluded) = withhold_holed_days(&signal, &minutes);
        assert_eq!(excluded.day_numbers(), &[19_001]);
        assert_eq!(excluded.days(), 1);
        assert_eq!(excluded.signal_bars(), 4, "the holed day's signal bars go");
        assert_eq!(
            excluded.minute_bars(),
            4,
            "and its minute bars go with them"
        );
        assert!(!excluded.is_empty());

        for bar in kept_signal.iter().chain(kept_minutes.iter()) {
            assert_ne!(
                indicators::ist_day(bar.ts_micros),
                19_001,
                "no bar of a withheld day may survive in either slice"
            );
        }
    }

    /// A gap-free span withholds nothing and copies both slices unchanged.
    #[test]
    fn a_gap_free_span_withholds_nothing() {
        let minutes: Vec<Candle> = (0..40).map(|m| bar_on(19_000, m)).collect();
        let signal: Vec<Candle> = (0..8).map(|b| bar_on(19_000, b * 5)).collect();
        let (kept_signal, kept_minutes, excluded) = withhold_holed_days(&signal, &minutes);
        assert!(excluded.is_empty());
        assert_eq!(excluded.days(), 0);
        assert_eq!(excluded.signal_bars(), 0);
        assert_eq!(excluded.minute_bars(), 0);
        assert_eq!(kept_signal, signal);
        assert_eq!(kept_minutes, minutes);
        assert!(excluded.day_names().is_empty());
    }

    /// The days are named as dates, which is what an operator can check.
    #[test]
    fn withheld_days_are_named_as_dates() {
        let minutes: Vec<Candle> = [0_i64, 1, 9, 10]
            .iter()
            .map(|m| bar_on(19_522, *m))
            .collect();
        let (_, _, excluded) = withhold_holed_days(&[], &minutes);
        assert_eq!(excluded.day_names(), vec!["2023-06-14".to_owned()]);
    }
}
