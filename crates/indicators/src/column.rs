//! The bar column the sweep consumes: one mask per bar, and a census of every
//! bar that did not reach it.
//!
//! # The seam that did not exist
//!
//! `crates/engine` takes `&[ConditionMask]`. This crate produces one
//! [`ConditionMask`] per [`Candle`]. Until this module, **nothing in the
//! workspace joined them** — the vocabulary, the indicators and the ladder were
//! each finished, tested and unreachable from the other two. This is the join,
//! and it is the whole of it: a slice of bars in, the column the ladder walks
//! out, and a count of everything that fell out on the way.
//!
//! # It reads no bar from anywhere
//!
//! [`Column::build`] takes a slice the caller already holds. This module opens
//! nothing, and CI gate 22 is what makes that structural rather than a promise:
//! `crates/indicators` may declare exactly one dependency, `vocab`, and no
//! filesystem call site may appear in this crate's `src/` or `benches/`. A bar
//! cannot reach the sweep through this file because there is no expressible way
//! for it to arrive.
//!
//! # The warm-up boundary is the point, not a detail
//!
//! [`Evaluator::step`] keeps emitting before the run has warmed up, and those
//! bits are correct — a position that cannot be evaluated evaluates false, which
//! is `docs/03-vocabulary.md` §4. But a bar on which whole families are
//! structurally unable to answer still **counts in support's denominator**, and
//! finding `F-0C28E4` in `docs/11-findings.md` records what that costs: a
//! genuinely frequent combination is pushed under `min_hits` and dies at k=1,
//! before a reader ever sees it.
//!
//! [`Evaluator::warmed_up`] published that boundary and **no caller consumed
//! it**. This one does. A mask enters the column only if the run had already
//! warmed up *before* the bar was folded.
//!
//! ## The reason this doc first gave was wrong, and the correction is the point
//!
//! It said `step` emits before it folds, so the bar that makes a run warm emits
//! its mask while still cold. **Three quarters of that is false.**
//! [`Evaluator::warmed_up`] reads four things, and `prev5`, `yesterday` and
//! `previous` are all written by `close_the_books` inside the session-rollover
//! block, which runs *six lines before* the emit. Only the trend conjunct is
//! updated by the fold that follows.
//!
//! So on the shape this engine actually sweeps — NSE one-minute spot, 375-bar
//! sessions — the binding conjunct is the five-session ladder and not the
//! 200-candle trend seed, and it fills before the emit. Reading the verdict
//! after `step` would therefore sweep a bar that is genuinely **warm**. The cost
//! of reading it before is one lost warm bar per run: 1 of 1,125 on an
//! eight-session fixture.
//!
//! **Before is still what this module does**, and now for the true reason: it is
//! correct for *every* shape, not just this one. The trend conjunct is the last
//! to fill whenever a session is shorter than 200 bars — a daily timeframe, a
//! Muhurat session, any instrument whose sessions are short — and on those the
//! after-reading sweeps a bar whose EMA200 could not answer. Losing one warm bar
//! is the price of a boundary that does not depend on the bar count per session,
//! and paying it knowingly is different from paying it by accident.
//!
//! `warmed_up` is monotone, so the emitted column is a contiguous suffix of the
//! offered bars less any refusals inside it —
//! [`crate::column::tests::the_column_is_a_suffix_and_never_reorders`].
//!
//! # Nothing is dropped silently
//!
//! `CLAUDE.md` §4 bans a fallback that hides a failure. Every bar handed in
//! lands in exactly one bucket of [`Census`], and the buckets are counted in the
//! loop rather than derived afterwards — a residual would absorb a bar nobody
//! measured and report it as one that was. [`Census::reconciles`] is therefore a
//! loop invariant, proved by
//! [`crate::column::tests::every_bar_lands_in_exactly_one_bucket`].
//!
//! The refusal buckets are one per [`Corrupt`] variant with **no catch-all**, so
//! a new variant is a compile error here rather than a bar quietly counted as
//! something it is not.
//!
//! # Nothing is logged
//!
//! CI gate 17 forbids an event emit in this crate outright: the sweep evaluates
//! a mask per (bar, combination) and a billion O(1) calls is still a billion
//! calls. [`Census`] is plain integers, returned to the caller, who emits it
//! **once** at a structural boundary. That is the shape `pull::session::
//! DropCensus` already uses.
//!
//! **The rule is spelled around, not quoted, and that is deliberate.** Gate 17
//! greps for the emit path as a literal token and does **not** strip comments,
//! so a doc block quoting the banned spelling in order to explain it fails the
//! gate it is describing. This paragraph exists because that is exactly what
//! happened here — the same "text in a comment is text" defect gate 22's own
//! clause A records having been caught by.
//!
//! # Cost
//!
//! Per bar: one `bool` read, one [`Evaluator::step`], and one push into a vector
//! reserved once at entry. No allocation inside the loop, no scan of what came
//! before, and no branch whose cost depends on the data. The per-bar cost is
//! therefore whatever `step` costs plus a constant, and it is measured rather
//! than asserted: row `C-I-06` of `crates/indicators/benches/ratio.rs` builds a
//! 20,000-bar column and a 200,000-bar one and compares the cost PER BAR.
//!
//! That row exists because naming a bench that does not measure the claim is the
//! one thing CI gate 12 cannot see -- it checks that a proof is named and that
//! the named thing exists, never that it proves anything. This doc block named
//! the file before the row was written, which is precisely the defect the gate
//! converts from unfalsifiable to falsifiable and no further.

use vocab::ConditionMask;

use crate::evaluator::Evaluator;
use crate::{Candle, Corrupt};

/// Where every offered bar went.
///
/// One bucket per outcome, counted in the loop. `offered` is incremented beside
/// the others rather than read from the slice length, so
/// [`Self::reconciles`] is an invariant of the loop and not an identity that a
/// residual makes true by construction — the defect `engine::Frontier` records
/// having had.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Census {
    /// Bars handed to [`Column::build`].
    pub offered: u64,
    /// Bars folded successfully while the run was still cold. Their bits were
    /// correct and are deliberately not swept — see the module doc.
    pub warming: u64,
    /// Bars whose mask entered the column. Equal to [`Column::len`].
    pub swept: u64,
    /// [`Corrupt::HighBelowLow`].
    pub high_below_low: u64,
    /// [`Corrupt::RangeOverflows`].
    pub range_overflows: u64,
    /// [`Corrupt::PriceOutsideRange`].
    pub price_outside_range: u64,
    /// [`Corrupt::TimestampNotIncreasing`].
    pub timestamp_not_increasing: u64,
    /// [`Corrupt::NegativeVolume`].
    pub negative_volume: u64,
    /// [`Corrupt::AccumulatorTooLarge`].
    pub accumulator_too_large: u64,
}

impl Census {
    /// Every bar refused as not-a-bar, across all six reasons.
    #[must_use]
    pub const fn refused(&self) -> u64 {
        self.high_below_low
            .saturating_add(self.range_overflows)
            .saturating_add(self.price_outside_range)
            .saturating_add(self.timestamp_not_increasing)
            .saturating_add(self.negative_volume)
            .saturating_add(self.accumulator_too_large)
    }

    /// Did every offered bar land in exactly one bucket?
    ///
    /// False is a defect in this module, never in the data. Proved by
    /// [`crate::column::tests::every_bar_lands_in_exactly_one_bucket`].
    #[must_use]
    pub const fn reconciles(&self) -> bool {
        self.offered
            == self
                .warming
                .saturating_add(self.swept)
                .saturating_add(self.refused())
    }

    /// Charges `corrupt` to its own bucket.
    ///
    /// Exhaustive by construction: there is no catch-all arm, so a new
    /// [`Corrupt`] variant fails to compile here rather than being counted as a
    /// neighbour.
    const fn charge(&mut self, corrupt: Corrupt) {
        match corrupt {
            Corrupt::HighBelowLow => {
                self.high_below_low = self.high_below_low.saturating_add(1);
            }
            Corrupt::RangeOverflows => {
                self.range_overflows = self.range_overflows.saturating_add(1);
            }
            Corrupt::PriceOutsideRange => {
                self.price_outside_range = self.price_outside_range.saturating_add(1);
            }
            Corrupt::TimestampNotIncreasing => {
                self.timestamp_not_increasing = self.timestamp_not_increasing.saturating_add(1);
            }
            Corrupt::NegativeVolume => {
                self.negative_volume = self.negative_volume.saturating_add(1);
            }
            Corrupt::AccumulatorTooLarge => {
                self.accumulator_too_large = self.accumulator_too_large.saturating_add(1);
            }
        }
    }
}

/// One [`ConditionMask`] per swept bar, in the order the bars arrived.
///
/// This is the exact input `engine::Ladder::walk` takes as `bar_bits`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Column {
    bits: Vec<ConditionMask>,
    /// The caller-slice index each entry in `bits` came from.
    ///
    /// Parallel to `bits`, and the ONLY honest way to map a column position
    /// back to a bar. `first_swept + j` is not that map: a refusal can occur
    /// anywhere mid-stream, so the two run out of step from the first corrupt
    /// bar onward. Anything that pairs a signal with what happened AFTER it --
    /// a forward return, a trade outcome -- would then be reading the wrong
    /// bar's future, silently and only on data that contains a refusal.
    ///
    /// Eight bytes per swept bar, which is the honest cost of that map.
    source: Vec<usize>,
    census: Census,
    first_swept: Option<usize>,
}

impl Column {
    /// Folds `bars` through `evaluator`, keeping the masks of warmed-up bars.
    ///
    /// The evaluator is borrowed rather than constructed here on purpose:
    /// [`Evaluator::new`] needs a `Widths`, an `Availability` and a
    /// `Thresholds`, and every one of those is a decision about the run that
    /// `CLAUDE.md` §3 rule 1 will not let this module invent on the caller's
    /// behalf. It is left borrowed on return so a caller can read the warm-up
    /// diagnostics — [`Evaluator::sessions_until_every_family_can_answer`] — off
    /// a column that came back empty.
    ///
    /// Bars are read strictly forward and each is folded before the next is
    /// looked at, so §3 rule 7 (no look-ahead) holds by construction:
    /// [`crate::column::tests::a_prefix_of_the_bars_gives_a_prefix_of_the_column`].
    ///
    /// # Cost
    ///
    /// One `bool` read, one [`Evaluator::step`] and one amortised push per bar,
    /// into a vector reserved once before the loop. Constant per bar, measured
    /// by row `C-I-06` of `crates/indicators/benches/ratio.rs`.
    #[must_use]
    pub fn build(bars: &[Candle], evaluator: &mut Evaluator) -> Self {
        let mut bits = Vec::with_capacity(bars.len());
        let mut census = Census::default();
        let mut first_swept = None;
        let mut source: Vec<usize> = Vec::with_capacity(bars.len());

        for (index, bar) in bars.iter().enumerate() {
            census.offered = census.offered.saturating_add(1);

            // The verdict for the state BEFORE this bar folds. On 375-bar
            // sessions this drops one genuinely warm bar per run; on any session
            // shorter than the 200-candle trend seed it is the only reading that
            // does not sweep a bar whose EMA200 could not answer. The module doc
            // carries the measurement and the correction it replaced.
            let warm = evaluator.warmed_up();

            match evaluator.step(bar) {
                Ok(mask) => {
                    if warm {
                        if first_swept.is_none() {
                            first_swept = Some(index);
                        }
                        bits.push(mask);
                        source.push(index);
                        census.swept = census.swept.saturating_add(1);
                    } else {
                        census.warming = census.warming.saturating_add(1);
                    }
                }
                // `step` commits on success only, so a refused bar leaves the
                // evaluator exactly as the previous bar left it.
                Err(corrupt) => census.charge(corrupt),
            }
        }

        Self {
            bits,
            source,
            census,
            first_swept,
        }
    }

    /// The caller-slice index of each swept bar, parallel to [`Self::bits`].
    ///
    /// `sources()[j]` is the index, in the slice handed to [`Self::build`], of
    /// the bar whose mask is `bits()[j]`. A caller pairing a signal with what
    /// followed it MUST go through this rather than assuming
    /// `first_swept() + j`: refusals are counted, not removed from the middle,
    /// so the two diverge from the first corrupt bar and every outcome after it
    /// would be read off the wrong bar.
    #[must_use]
    pub fn sources(&self) -> &[usize] {
        &self.source
    }

    /// The column, as `engine::Ladder::walk` wants it.
    #[must_use]
    pub fn bits(&self) -> &[ConditionMask] {
        &self.bits
    }

    /// Where every offered bar went.
    #[must_use]
    pub const fn census(&self) -> Census {
        self.census
    }

    /// The index, in the offered slice, of the first bar that was swept.
    ///
    /// `None` when the run never warmed up. This is the honest answer to "how
    /// much history did this column cost", and it is an index into the caller's
    /// own slice rather than a bar count this module would have to guess.
    #[must_use]
    pub const fn first_swept(&self) -> Option<usize> {
        self.first_swept
    }

    /// How many masks the column holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bits.len()
    }

    /// Is the column empty? True whenever the run never warmed up.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail. `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind the way an \
              `unreachable!` expanding in this crate would."
)]
mod tests {
    use super::*;
    use crate::OI_NULL;
    use crate::evaluator::Widths;
    use crate::pattern::Thresholds;
    use crate::vwap::Availability;

    /// 09:15 IST as microseconds past midnight UTC. 555 minutes IST, less the
    /// 330-minute offset.
    const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
    const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;
    const MINUTE_MICROS: i64 = 60 * 1_000_000;
    /// 25,000.00 index points, in paisa.
    const BASE: i64 = 2_500_000;
    /// A regular NSE session.
    const BARS_PER_SESSION: usize = 375;
    /// Enough completed sessions to clear the five-session ladder, the previous
    /// day and the 200-candle trend seed with room to spare.
    const WARM_SESSIONS: i64 = 8;

    fn widths() -> Widths {
        Widths::pinned().expect("both pinned widths are valid")
    }

    fn evaluator(availability: Availability) -> Evaluator {
        Evaluator::new(widths(), availability, Thresholds::CLASSICAL)
    }

    /// A deterministic bar. Every field is a function of `(day, minute)` alone,
    /// so the same arguments give the same bar forever — which is what makes the
    /// idempotence test below mean anything.
    fn bar(day: i64, minute: usize) -> Candle {
        let m = i64::try_from(minute).unwrap_or(0);
        // A drift plus a bounded wobble. Both integer; §7 forbids a float here
        // as firmly as it does in shipping code.
        let drift = day.saturating_mul(400).saturating_add(m.saturating_mul(3));
        let wobble = ((m % 7) - 3).saturating_mul(25);
        let close = BASE.saturating_add(drift).saturating_add(wobble);
        let open = close.saturating_sub(wobble);
        let high = open.max(close).saturating_add(60);
        let low = open.min(close).saturating_sub(60);
        Candle::new(
            day.saturating_mul(DAY_MICROS)
                .saturating_add(IST_OPEN_UTC_MICROS)
                .saturating_add(m.saturating_mul(MINUTE_MICROS)),
            open,
            high,
            low,
            close,
            1_000 + (m % 11),
            OI_NULL,
        )
    }

    /// `sessions` consecutive trading days of `BARS_PER_SESSION` bars each.
    fn run(sessions: i64) -> Vec<Candle> {
        let mut out = Vec::with_capacity(usize::try_from(sessions).unwrap_or(0) * BARS_PER_SESSION);
        for day in 0..sessions {
            for minute in 0..BARS_PER_SESSION {
                out.push(bar(day, minute));
            }
        }
        out
    }

    /// A run long enough that the column is non-empty.
    fn warm_run() -> Vec<Candle> {
        run(WARM_SESSIONS)
    }

    /// `sources()` is the only honest map from a column position to a bar.
    ///
    /// # Why `first_swept() + j` is not that map
    ///
    /// Refusals are COUNTED, not removed from the middle of the stream. So the
    /// moment one bar is refused, the naive offset runs one behind and stays
    /// there. Nothing in a sweep notices — the masks are all correct — but a
    /// caller pairing a signal with what happened AFTER it would read every
    /// outcome from the wrong bar, silently, and only on data that contains a
    /// refusal. That is the shape of every defect an audit found today.
    #[test]
    fn sources_maps_a_column_position_to_its_bar_even_across_a_refusal() {
        let mut bars = run(8);
        // Corrupt one bar deep inside the swept region: high below low.
        let victim = bars.len() - 20;
        if let Some(b) = bars.get_mut(victim) {
            *b = Candle::new(
                b.ts_micros,
                b.open,
                b.low - 100,
                b.high,
                b.close,
                1,
                OI_NULL,
            );
        }
        let column = Column::build(&bars, &mut evaluator(Availability::Absent));

        assert_eq!(
            column.census().high_below_low,
            1,
            "the fixture must actually contain a refusal, or this proves nothing"
        );
        assert_eq!(
            column.sources().len(),
            column.bits().len(),
            "the map must be parallel to the column"
        );

        // Every source index is strictly increasing and points at a real bar.
        let mut previous: Option<usize> = None;
        for &s in column.sources() {
            assert!(s < bars.len(), "source {s} is past the end of the input");
            if let Some(p) = previous {
                assert!(s > p, "sources must ascend: {p} then {s}");
            }
            previous = Some(s);
        }

        // AND THE NAIVE OFFSET IS WRONG, which is the whole reason this exists.
        let first = column.first_swept().expect("the run warms up");
        let naive_agrees = column
            .sources()
            .iter()
            .enumerate()
            .all(|(j, &s)| s == first.saturating_add(j));
        assert!(
            !naive_agrees,
            "with a refusal inside the swept region, `first_swept + j` MUST \
             diverge from the real bar index -- if it agrees here the fixture \
             is not exercising the case this map was added for"
        );
    }

    #[test]
    fn an_empty_slice_gives_an_empty_column() {
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&[], &mut ev);

        assert!(column.is_empty());
        assert_eq!(column.len(), 0);
        assert_eq!(column.bits(), &[]);
        assert_eq!(column.first_swept(), None);
        assert_eq!(column.census(), Census::default());
        assert!(column.census().reconciles());
    }

    #[test]
    fn a_cold_run_sweeps_nothing_and_says_so() {
        // One session cannot fill a five-session ladder, so every bar folds and
        // none is swept. The column is empty and the census explains it.
        let bars = run(1);
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);

        assert!(column.is_empty());
        assert_eq!(column.first_swept(), None);
        let census = column.census();
        assert_eq!(census.offered, BARS_PER_SESSION as u64);
        assert_eq!(census.warming, BARS_PER_SESSION as u64);
        assert_eq!(census.swept, 0);
        assert_eq!(census.refused(), 0);
        assert!(census.reconciles());
        // The caller can still ask why, off the borrowed evaluator.
        assert!(!ev.warmed_up());
        assert!(ev.sessions_until_every_family_can_answer() > 0);
    }

    #[test]
    fn a_warm_run_sweeps_and_the_counts_agree() {
        let bars = warm_run();
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);

        assert!(!column.is_empty(), "eight sessions must warm the run");
        let census = column.census();
        assert_eq!(census.offered, bars.len() as u64);
        assert_eq!(census.swept, column.len() as u64);
        assert_eq!(census.refused(), 0);
        assert!(census.reconciles());
        assert_eq!(
            census.warming.saturating_add(census.swept),
            bars.len() as u64
        );
    }

    #[test]
    fn the_column_is_a_suffix_and_never_reorders() {
        // `warmed_up` is monotone, so the swept bars are a contiguous tail. Prove
        // it by rebuilding the same masks by hand and comparing position for
        // position.
        let bars = warm_run();
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);

        let first = column.first_swept().expect("a warm run swept something");
        let mut by_hand = evaluator(Availability::Absent);
        let mut expected = Vec::new();
        for (index, candle) in bars.iter().enumerate() {
            let warm = by_hand.warmed_up();
            let mask = by_hand.step(candle).expect("synthetic bars are bars");
            if warm {
                assert!(index >= first, "a swept bar before the first swept index");
                expected.push(mask);
            }
        }
        assert_eq!(column.bits(), expected.as_slice());
        // Contiguous: every index from `first` onward was swept.
        assert_eq!(column.len(), bars.len() - first);
    }

    #[test]
    fn a_prefix_of_the_bars_gives_a_prefix_of_the_column() {
        // §3 rule 7. Bar N's mask cannot depend on bar N+1, so truncating the
        // input can only truncate the output — never change it.
        let bars = warm_run();
        let mut full_ev = evaluator(Availability::Absent);
        let full = Column::build(&bars, &mut full_ev);

        let cut = bars.len().saturating_sub(BARS_PER_SESSION);
        let head = bars.get(..cut).expect("cut is inside the run");
        let mut short_ev = evaluator(Availability::Absent);
        let short = Column::build(head, &mut short_ev);

        assert!(!short.is_empty());
        assert_eq!(short.first_swept(), full.first_swept());
        assert_eq!(
            short.bits(),
            full.bits()
                .get(..short.len())
                .expect("the short column cannot outgrow the full one")
        );
    }

    #[test]
    fn the_same_bars_give_the_same_column_twice() {
        // §3 rule 5, idempotence, byte for byte.
        let bars = warm_run();
        let mut a = evaluator(Availability::Absent);
        let mut b = evaluator(Availability::Absent);
        assert_eq!(Column::build(&bars, &mut a), Column::build(&bars, &mut b));
    }

    #[test]
    fn every_bar_lands_in_exactly_one_bucket() {
        // One of every refusal, mixed into a warm run, plus the warm-up tail.
        let mut bars = warm_run();
        // The last bar the evaluator will ACCEPT. The three refusals below commit
        // nothing, so this stays the high-water mark for the fourth.
        let last_accepted = bars.last().map_or(0, |candle| candle.ts_micros);
        let ts = |n: i64| {
            WARM_SESSIONS
                .saturating_mul(DAY_MICROS)
                .saturating_add(IST_OPEN_UTC_MICROS)
                .saturating_add(n.saturating_mul(MINUTE_MICROS))
        };
        // high < low.
        bars.push(Candle::new(
            ts(1),
            BASE,
            BASE - 10,
            BASE + 10,
            BASE,
            1,
            OI_NULL,
        ));
        // high - low leaves i64.
        bars.push(Candle::new(ts(2), 0, i64::MAX, i64::MIN, 0, 1, OI_NULL));
        // open above high.
        bars.push(Candle::new(
            ts(3),
            BASE + 500,
            BASE + 100,
            BASE - 100,
            BASE,
            1,
            OI_NULL,
        ));
        // A timestamp that does not strictly increase. Equal is enough: the
        // rollover keys on the IST day CHANGING, so a repeat closes the books on
        // a session that has not happened yet.
        bars.push(Candle::new(
            last_accepted,
            BASE,
            BASE + 10,
            BASE - 10,
            BASE,
            1,
            OI_NULL,
        ));

        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        let census = column.census();

        assert_eq!(census.offered, bars.len() as u64);
        assert_eq!(census.high_below_low, 1);
        assert_eq!(census.range_overflows, 1);
        assert_eq!(census.price_outside_range, 1);
        assert_eq!(census.timestamp_not_increasing, 1);
        assert_eq!(census.refused(), 4);
        assert!(census.reconciles());
        // A refused bar changes nothing: the column is the warm run's column.
        let mut clean_ev = evaluator(Availability::Absent);
        let clean = Column::build(&warm_run(), &mut clean_ev);
        assert_eq!(column.bits(), clean.bits());
    }

    #[test]
    fn a_negative_volume_is_charged_to_its_own_bucket() {
        // Only the VWAP family reads volume, so this refusal needs an evaluator
        // that runs VWAP at all.
        let bars = vec![Candle::new(
            IST_OPEN_UTC_MICROS,
            BASE,
            BASE + 10,
            BASE - 10,
            BASE,
            -1,
            OI_NULL,
        )];
        let mut ev = evaluator(Availability::Present);
        let census = Column::build(&bars, &mut ev).census();

        assert_eq!(census.negative_volume, 1);
        assert_eq!(census.refused(), 1);
        assert_eq!(census.swept, 0);
        assert_eq!(census.warming, 0);
        assert!(census.reconciles());
    }

    #[test]
    fn an_oversized_accumulator_is_charged_to_its_own_bucket() {
        // Unreachable on any price a market prints, and constructible here in one
        // line -- which is the argument for synthetic bars rather than stored
        // ones. With real data this branch could never be covered.
        let bars = vec![Candle::new(
            IST_OPEN_UTC_MICROS,
            i64::MAX - 1,
            i64::MAX,
            i64::MAX - 2,
            i64::MAX,
            i64::MAX,
            OI_NULL,
        )];
        let mut ev = evaluator(Availability::Present);
        let census = Column::build(&bars, &mut ev).census();

        assert_eq!(census.accumulator_too_large, 1);
        assert_eq!(census.refused(), 1);
        assert!(census.reconciles());
    }

    #[test]
    fn the_census_charges_each_variant_exactly_once() {
        // `charge` is exhaustive and injective. Walk every variant and assert the
        // bucket it moved -- a mutant that redirects one arm to a neighbour dies
        // here.
        let variants = [
            Corrupt::HighBelowLow,
            Corrupt::RangeOverflows,
            Corrupt::PriceOutsideRange,
            Corrupt::TimestampNotIncreasing,
            Corrupt::NegativeVolume,
            Corrupt::AccumulatorTooLarge,
        ];
        for variant in variants {
            let mut census = Census::default();
            census.charge(variant);
            assert_eq!(census.refused(), 1, "{variant:?} charged no bucket");
            let moved = [
                census.high_below_low,
                census.range_overflows,
                census.price_outside_range,
                census.timestamp_not_increasing,
                census.negative_volume,
                census.accumulator_too_large,
            ];
            assert_eq!(
                moved.iter().filter(|n| **n == 1).count(),
                1,
                "{variant:?} charged more than one bucket"
            );
        }
    }

    #[test]
    fn reconciles_is_false_when_a_bar_is_lost() {
        // The invariant has to be able to FAIL, or asserting it proves nothing.
        let census = Census {
            offered: 2,
            warming: 0,
            swept: 1,
            ..Census::default()
        };
        assert!(!census.reconciles());
    }

    #[test]
    fn first_swept_is_the_index_in_the_callers_own_slice() {
        let bars = warm_run();
        let mut ev = evaluator(Availability::Absent);
        let column = Column::build(&bars, &mut ev);
        let first = column.first_swept().expect("a warm run swept something");

        // Everything before it was warming; nothing before it was refused.
        assert_eq!(column.census().warming, first as u64);
        assert_eq!(column.census().refused(), 0);
        assert!(first > 0, "a run cannot be warm on its first bar");
    }
}
