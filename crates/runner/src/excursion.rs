//! One pass over a trade's path, and every SL/TP answer afterwards is a lookup.
//!
//! # The problem this exists to make affordable
//!
//! A time-based exit is decided once: `min(horizon, 15:10)`. A stop-loss or a
//! target is **path-dependent** — you must ask, at every bar between entry and
//! exit, whether a level was touched. Doing that once per candidate level turns
//! the exit grid into a multiplier on the whole search.
//!
//! Measured on this repository's own numbers: 207 distinct entry combinations
//! survive closure, and a grid of 20 stop rungs by 20 target rungs is 400
//! variants each. Re-walking the path per variant is 82,800 passes over the
//! bars. This module makes it **one** pass, after which every (stop, target)
//! pair is three integer compares.
//!
//! # The two quantities, and why they are enough
//!
//! From an entry, only two things about the path matter to a stop or a target:
//!
//! * **MAE**, the maximum adverse excursion — how far it went against you.
//! * **MFE**, the maximum favourable excursion — how far it went for you.
//!
//! Both are running maxima, so both are **monotone non-decreasing in time**. A
//! stop of `x` is hit at the first bar where MAE reaches `x`; a target of `y` at
//! the first bar where MFE reaches `y`. Because the sequences are monotone, the
//! first-crossing bar for every rung of a sorted ladder can be found in a single
//! merge — no search per rung, and no repeated pass.
//!
//! `docs/07-o1-architecture.md` layer 4 bans `binary_search` outright and CI
//! gate 11 rule 1 enforces it. That ban is not worked around here: two monotone
//! sequences are merged with two cursors, which is why the ladder must be sorted
//! and is asserted to be.
//!
//! # The rungs are DERIVED, never typed
//!
//! A human writing "stop at 20 points" is the manual intervention this engine
//! exists to remove, and it is also wrong across instruments: 20 points is a
//! scratch on BANKNIFTY and a real move on NIFTY. [`Ladder::from_excursions`]
//! places the rungs at **quantiles of the excursions the instrument actually
//! produced**, so the grid is a property of the data rather than of an opinion.
//!
//! Held in **parts per million of the entry price**, not in paisa, for the same
//! reason: a basis point means the same thing on both indices, and a paisa does
//! not.
//!
//! # Which came first, inside one bar
//!
//! A one-minute bar carries a high and a low and **no order between them**. When
//! a bar's low would trigger the stop and its high would trigger the target,
//! the data cannot say which happened first, and every backtest that guesses
//! flatters itself systematically.
//!
//! So this module does not guess. [`Outcome::ambiguous`] is set on exactly those
//! bars, and [`crate::trade`]'s pessimistic case resolves them as **the stop
//! first**. The optimistic case resolves them as the target first, and the gap
//! between the two is reported rather than averaged away.

use indicators::Candle;

/// Parts per million of the entry price, as an integer.
///
/// Integer because `CLAUDE.md` §7 keeps prices in integers and a threshold
/// compared against a price is a price. Basis points rather than paisa because
/// a rung must mean the same thing on NIFTY and BANKNIFTY.
pub type Ppm = i64;

/// One million: the parts-per-million denominator.
const PPM_ONE: i64 = 1_000_000;

/// A sorted ladder of thresholds, in parts per million.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ladder {
    rungs: Vec<Ppm>,
}

impl Ladder {
    /// A ladder from an explicit, ascending list.
    ///
    /// `None` when the list is empty, unsorted, or carries a non-positive rung.
    /// Refused rather than sorted for the caller: a ladder arriving out of order
    /// means the caller computed it wrongly, and quietly fixing it would hide
    /// that.
    #[must_use]
    pub fn new(rungs: Vec<Ppm>) -> Option<Self> {
        if rungs.is_empty() || rungs.first().is_some_and(|&r| r <= 0) {
            return None;
        }
        if rungs.windows(2).any(|w| w.first() >= w.last()) {
            return None;
        }
        Some(Self { rungs })
    }

    /// A ladder placed at quantiles of the excursions actually observed.
    ///
    /// # No number is chosen by anybody
    ///
    /// `observed` is every excursion the instrument produced on the training
    /// bars. The rungs land at evenly spaced quantiles of it, so a rung is
    /// "the level a fifth of moves reached" rather than "twenty points". On an
    /// instrument that has become quieter the whole ladder moves down with it,
    /// and nothing has to be re-typed.
    ///
    /// `count` rungs are requested; fewer are returned when the sample has
    /// fewer distinct values, because duplicate rungs would test the same
    /// threshold twice and report it as two.
    ///
    /// `None` when nothing was observed, or when every observation was zero —
    /// an instrument that never moved has no ladder, and inventing one would be
    /// inventing a distribution.
    #[must_use]
    pub fn from_excursions(observed: &mut [Ppm], count: usize) -> Option<Self> {
        if observed.is_empty() || count == 0 {
            return None;
        }
        observed.sort_unstable();
        let mut rungs: Vec<Ppm> = Vec::with_capacity(count);
        for i in 1..=count {
            // The i-th of `count+1` quantiles, so the ladder spans the body of
            // the distribution and never places a rung at its maximum — a rung
            // no move can exceed is a rung nothing is ever measured against.
            let at = observed.len().saturating_mul(i) / count.saturating_add(1);
            let Some(&value) = observed.get(at.min(observed.len().saturating_sub(1))) else {
                continue;
            };
            if value > 0 && rungs.last() != Some(&value) {
                rungs.push(value);
            }
        }
        Self::new(rungs)
    }

    /// The rungs, ascending.
    #[must_use]
    pub fn rungs(&self) -> &[Ppm] {
        &self.rungs
    }

    /// How many rungs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rungs.len()
    }

    /// Is the ladder empty?
    ///
    /// Never true for a `Ladder` built through [`Self::new`], which refuses an
    /// empty list. Present because clippy asks for it beside `len`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rungs.is_empty()
    }
}

/// The three ladders a path is measured against.
///
/// Bundled because they always travel together and a function taking six loose
/// parameters invites a caller to swap two of them silently -- the stop and the
/// target ladders have the same type and opposite meanings.
#[derive(Clone, Copy, Debug)]
pub struct Ladders<'a> {
    /// Distance from entry, against the position.
    pub stops: &'a Ladder,
    /// Distance from entry, for the position.
    pub targets: &'a Ladder,
    /// Give-back from the running peak.
    pub trails: &'a Ladder,
}

/// Where a trade's path first crossed each rung.
///
/// Offsets are **bars after the entry bar**, so `0` means the entry bar itself
/// and `NEVER` means the path did not reach that rung before the trade ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Crossings {
    /// First offset at which the adverse excursion reached rung `i`.
    adverse: Vec<usize>,
    /// First offset at which the favourable excursion reached rung `i`.
    favourable: Vec<usize>,
    /// First offset at which the retreat from the running peak reached rung `i`.
    ///
    /// A TRAILING stop: the level follows the best price seen and fires on the
    /// give-back, not on the distance from entry. Measured in the same pass and
    /// for the same reason -- the largest retreat so far is monotone even though
    /// the retreat itself is not.
    trailing: Vec<usize>,
    /// Offsets on which BOTH a stop and a target rung were newly reached, so
    /// the bar's own order decides and one-minute data does not carry it.
    ambiguous: Vec<usize>,
    /// The last offset walked, so a lookup can say "held to the end".
    last: usize,
}

/// A rung that the path never reached.
pub const NEVER: usize = usize::MAX;

impl Crossings {
    /// The offset a stop at rung `i` would have exited on, or [`NEVER`].
    #[must_use]
    pub fn stop_at(&self, rung: usize) -> usize {
        self.adverse.get(rung).copied().unwrap_or(NEVER)
    }

    /// The offset a target at rung `i` would have exited on, or [`NEVER`].
    #[must_use]
    pub fn target_at(&self, rung: usize) -> usize {
        self.favourable.get(rung).copied().unwrap_or(NEVER)
    }

    /// The offset a trailing stop at rung `i` would have exited on, or [`NEVER`].
    ///
    /// # This is the exit that makes a sniper setup pay
    ///
    /// A fixed stop asks "how far from where I got in". A trailing stop asks
    /// "how much of what I made am I willing to give back", which is the
    /// question that lets a winner run while still cutting it. TRAILING TAKE
    /// PROFIT is this same lookup ARMED by a target rung: reach the target,
    /// then trail -- so it composes from the two ladders rather than needing a
    /// third mechanism.
    #[must_use]
    pub fn trail_at(&self, rung: usize) -> usize {
        self.trailing.get(rung).copied().unwrap_or(NEVER)
    }

    /// Offsets where a stop and a target were both newly reached on one bar.
    ///
    /// The set the pessimistic and optimistic cases resolve differently — see
    /// the module header. Empty means no bar was ambiguous and the two cases
    /// agree on every exit.
    #[must_use]
    pub fn ambiguous(&self) -> &[usize] {
        &self.ambiguous
    }

    /// The last offset the path was walked to.
    #[must_use]
    pub const fn last(&self) -> usize {
        self.last
    }
}

/// Which way the position is facing.
///
/// A local enum rather than `costs::fill::Direction` because this module is
/// about a price path and not about a fill, and because it must be usable
/// before a fill exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// Profits when price rises.
    Long,
    /// Profits when price falls.
    Short,
}

/// Walk one trade's path once and record where it crossed every rung.
///
/// `bars[from..=to]` is the held window, `entry` the fill price in paisa.
///
/// # Cost
///
/// One pass over the held bars, and within it two cursors that only ever
/// advance — each rung is crossed at most once, so the total work is
/// `O(bars + rungs)` for the whole ladder rather than `O(bars)` per rung.
/// Neither cursor rewinds and nothing is searched, which is what keeps
/// `docs/07-o1-architecture.md` layer 4's ban on `binary_search` from being
/// something to work around.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn crossings(
    bars: &[Candle],
    from: usize,
    to: usize,
    entry: i64,
    side: Side,
    ladders: Ladders<'_>,
) -> Crossings {
    let Ladders {
        stops,
        targets,
        trails,
    } = ladders;
    let mut out = Crossings {
        adverse: vec![NEVER; stops.len()],
        favourable: vec![NEVER; targets.len()],
        trailing: vec![NEVER; trails.len()],
        ambiguous: Vec::new(),
        last: 0,
    };
    if entry <= 0 || from > to {
        return out;
    }

    // Cursors into the three ladders. All only advance: once a rung is crossed
    // it stays crossed, because MAE, MFE and the trailing give-back are all
    // running maxima.
    let mut stop_cursor = 0_usize;
    let mut target_cursor = 0_usize;
    let mut trail_cursor = 0_usize;
    let mut mae: Ppm = 0;
    let mut mfe: Ppm = 0;
    // THE TRAILING PAIR. `peak` is the best price seen so far and only ever
    // improves; `give_back` is the largest retreat FROM that peak and, being a
    // running maximum, is monotone in exactly the way the merge needs.
    //
    // That monotonicity is the whole reason a trailing stop costs no more than
    // a fixed one. The retreat itself wobbles -- it shrinks whenever a new peak
    // is made -- but "has a trail of `d` fired by bar j" asks about the LARGEST
    // retreat so far, and that never decreases.
    let mut peak: i64 = entry;
    let mut give_back: Ppm = 0;

    for offset in 0..=to.saturating_sub(from) {
        let Some(bar) = bars.get(from.saturating_add(offset)) else {
            break;
        };
        out.last = offset;

        // Adverse is the extreme that hurts, favourable the one that helps, and
        // which is which is the whole of what `side` decides.
        let (adverse_price, favourable_price) = match side {
            Side::Long => (bar.low, bar.high),
            Side::Short => (bar.high, bar.low),
        };
        let adverse_move = match side {
            Side::Long => entry.saturating_sub(adverse_price),
            Side::Short => adverse_price.saturating_sub(entry),
        };
        let favourable_move = match side {
            Side::Long => favourable_price.saturating_sub(entry),
            Side::Short => entry.saturating_sub(favourable_price),
        };
        mae = mae.max(ppm_of(adverse_move, entry));
        mfe = mfe.max(ppm_of(favourable_move, entry));

        // The peak improves first, then the retreat is measured FROM it. Order
        // matters: measuring the retreat before updating the peak would compare
        // this bar's low against the previous bar's high and report a give-back
        // one bar late on every new high.
        peak = match side {
            Side::Long => peak.max(favourable_price),
            Side::Short => peak.min(favourable_price),
        };
        let retreat = match side {
            Side::Long => peak.saturating_sub(adverse_price),
            Side::Short => adverse_price.saturating_sub(peak),
        };
        give_back = give_back.max(ppm_of(retreat, entry));
        while trails
            .rungs()
            .get(trail_cursor)
            .is_some_and(|&rung| give_back >= rung)
        {
            if let Some(slot) = out.trailing.get_mut(trail_cursor) {
                *slot = offset;
            }
            trail_cursor = trail_cursor.saturating_add(1);
        }

        let stop_before = stop_cursor;
        while stops
            .rungs()
            .get(stop_cursor)
            .is_some_and(|&rung| mae >= rung)
        {
            if let Some(slot) = out.adverse.get_mut(stop_cursor) {
                *slot = offset;
            }
            stop_cursor = stop_cursor.saturating_add(1);
        }
        let target_before = target_cursor;
        while targets
            .rungs()
            .get(target_cursor)
            .is_some_and(|&rung| mfe >= rung)
        {
            if let Some(slot) = out.favourable.get_mut(target_cursor) {
                *slot = offset;
            }
            target_cursor = target_cursor.saturating_add(1);
        }

        // BOTH SIDES MOVED ON ONE BAR, and the bar does not say which first.
        // Recorded rather than resolved: `crate::trade` resolves it, and the
        // pessimistic and optimistic answers differ exactly here.
        if stop_cursor > stop_before && target_cursor > target_before {
            out.ambiguous.push(offset);
        }
    }
    out
}

/// A price move as parts per million of the entry price.
///
/// Integer arithmetic at `i128` width and narrowed once: a paisa move times
/// 10,000 leaves `i64` for a large enough move, and a wrapped threshold would
/// compare a huge adverse excursion as a tiny one.
fn ppm_of(move_paisa: i64, entry: i64) -> Ppm {
    if move_paisa <= 0 || entry <= 0 {
        return 0;
    }
    let scaled = i128::from(move_paisa).saturating_mul(i128::from(PPM_ONE)) / i128::from(entry);
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Ladder, Ladders, NEVER, Ppm, Side, crossings};
    use indicators::{Candle, OI_NULL};

    fn bar(minute: i64, low: i64, high: i64) -> Candle {
        Candle::new(
            minute.saturating_mul(60_000_000),
            low,
            high,
            low,
            high,
            100,
            OI_NULL,
        )
    }

    fn ladder(rungs: &[Ppm]) -> Ladder {
        Ladder::new(rungs.to_vec()).expect("an ascending ladder")
    }

    #[test]
    fn a_ladder_refuses_what_it_cannot_merge_against() {
        // The single pass depends on the rungs ascending. An unsorted ladder is
        // refused rather than sorted, because arriving unsorted means the
        // caller computed it wrongly and fixing it quietly hides that.
        assert!(Ladder::new(vec![]).is_none(), "empty");
        assert!(Ladder::new(vec![0]).is_none(), "a zero rung");
        assert!(Ladder::new(vec![-5, 10]).is_none(), "a negative rung");
        assert!(Ladder::new(vec![30, 10]).is_none(), "descending");
        assert!(Ladder::new(vec![10, 10]).is_none(), "a repeated rung");
        assert!(Ladder::new(vec![10, 20, 30]).is_some());
    }

    #[test]
    fn every_rung_records_the_first_bar_that_reached_it_and_not_a_later_one() {
        // Entry at 100_000 paisa. The path goes 50 bps against, then 150 bps
        // against, so a 50-bps stop exits on bar 0 and a 150-bps stop on bar 1.
        // A stop of 300 bps is never reached.
        let entry = 100_000_i64;
        let bars = vec![
            bar(0, entry - 500, entry),   // 50 bps adverse
            bar(1, entry - 1_500, entry), // 150 bps adverse
            bar(2, entry - 2_000, entry), // 200 bps adverse
        ];
        let stops = ladder(&[5_000, 15_000, 30_000]);
        let targets = ladder(&[5_000]);
        let c = crossings(
            &bars,
            0,
            2,
            entry,
            Side::Long,
            Ladders {
                stops: &stops,
                targets: &targets,
                trails: &targets,
            },
        );

        assert_eq!(c.stop_at(0), 0, "5,000 ppm was reached on the first bar");
        assert_eq!(c.stop_at(1), 1, "15,000 ppm on the second");
        assert_eq!(c.stop_at(2), NEVER, "30,000 ppm was never reached");
        assert_eq!(c.target_at(0), NEVER, "the path never went favourable");
    }

    #[test]
    fn a_short_reads_the_high_as_adverse_and_the_low_as_favourable() {
        // The mirror of the case above. Getting this backwards would make every
        // short's stop and target swap, which no aggregate would reveal.
        let entry = 100_000_i64;
        let bars = vec![bar(0, entry - 1_000, entry + 1_000)];
        let rungs = ladder(&[5_000]);
        let long = crossings(
            &bars,
            0,
            0,
            entry,
            Side::Long,
            Ladders {
                stops: &rungs,
                targets: &rungs,
                trails: &rungs,
            },
        );
        let short = crossings(
            &bars,
            0,
            0,
            entry,
            Side::Short,
            Ladders {
                stops: &rungs,
                targets: &rungs,
                trails: &rungs,
            },
        );

        assert_eq!(long.stop_at(0), 0, "a long is stopped by the LOW");
        assert_eq!(long.target_at(0), 0, "and targeted by the HIGH");
        assert_eq!(short.stop_at(0), 0, "a short is stopped by the HIGH");
        assert_eq!(short.target_at(0), 0, "and targeted by the LOW");
    }

    #[test]
    fn a_bar_that_would_trigger_both_is_recorded_as_ambiguous_rather_than_guessed() {
        // THE HONESTY CASE. A one-minute bar carries a high and a low and no
        // order between them. A backtest that silently assumed the target came
        // first would flatter every strategy, systematically and invisibly.
        let entry = 100_000_i64;
        let bars = vec![bar(0, entry - 1_000, entry + 1_000)];
        let rungs = ladder(&[5_000]);
        let c = crossings(
            &bars,
            0,
            0,
            entry,
            Side::Long,
            Ladders {
                stops: &rungs,
                targets: &rungs,
                trails: &rungs,
            },
        );

        assert_eq!(
            c.ambiguous(),
            &[0],
            "a bar reaching both a stop and a target must be flagged"
        );
    }

    #[test]
    fn a_path_that_only_goes_one_way_leaves_nothing_ambiguous() {
        let entry = 100_000_i64;
        let bars = vec![bar(0, entry, entry + 1_000), bar(1, entry, entry + 2_000)];
        let rungs = ladder(&[5_000]);
        let c = crossings(
            &bars,
            0,
            1,
            entry,
            Side::Long,
            Ladders {
                stops: &rungs,
                targets: &rungs,
                trails: &rungs,
            },
        );
        assert!(c.ambiguous().is_empty(), "nothing went adverse at all");
        assert_eq!(c.stop_at(0), NEVER);
        assert_eq!(c.target_at(0), 0);
    }

    #[test]
    fn the_rungs_are_derived_from_what_the_instrument_did_and_never_typed() {
        // Ten observed excursions, four rungs requested. Every rung must be one
        // of the observed values -- a rung nobody's move ever reached is a
        // threshold measured against nothing.
        let mut observed: Vec<Ppm> = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let l = Ladder::from_excursions(&mut observed, 4).expect("a ladder from ten moves");
        assert!(l.len() <= 4);
        for rung in l.rungs() {
            assert!(
                observed.contains(rung),
                "rung {rung} is not a value the instrument produced"
            );
        }
        assert!(
            l.rungs().windows(2).all(|w| w.first() < w.last()),
            "the derived ladder must ascend"
        );
    }

    #[test]
    fn an_instrument_that_never_moved_has_no_ladder_rather_than_an_invented_one() {
        assert!(Ladder::from_excursions(&mut [], 4).is_none(), "no sample");
        assert!(
            Ladder::from_excursions(&mut [0, 0, 0], 4).is_none(),
            "every observation was zero, so there is no distribution to place \
             rungs on and inventing one would be inventing the data"
        );
    }

    #[test]
    fn an_empty_window_or_a_zero_entry_records_nothing_rather_than_panicking() {
        let rungs = ladder(&[5_000]);
        let bars = vec![bar(0, 90_000, 110_000)];
        let c = crossings(
            &bars,
            0,
            0,
            0,
            Side::Long,
            Ladders {
                stops: &rungs,
                targets: &rungs,
                trails: &rungs,
            },
        );
        assert_eq!(c.stop_at(0), NEVER, "a zero entry price divides nothing");
        let d = crossings(
            &bars,
            5,
            1,
            100_000,
            Side::Long,
            Ladders {
                stops: &rungs,
                targets: &rungs,
                trails: &rungs,
            },
        );
        assert_eq!(d.stop_at(0), NEVER, "an inverted window walks no bar");
    }
}
