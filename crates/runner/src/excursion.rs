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
//!
//! # The trailing stop had the same problem and did not say so
//!
//! The trail took its peak AND its give-back from the same bar: the bar's own
//! favourable extreme raised the peak, and the retreat was then measured from
//! the raised peak against the *same* bar's adverse extreme. That is the
//! favourable-first ordering assumed silently, on every bar, and it is the
//! look-ahead class of error — it fires a trail on bars where the other
//! ordering would not have fired one at all, and prices it off a high the
//! order had not yet seen. Stop-versus-target was resolved both ways and
//! reported; the trail was resolved one way and not reported.
//!
//! Two things follow, and this module now separates them.
//!
//! **Whether the rung fired is settled by measuring the retreat against the
//! peak AS IT STOOD BEFORE THE BAR.** That test is order-independent: if the
//! pre-bar peak `P` satisfies `P - low >= d` then the level `P - d` sits at or
//! above the bar's low, so the low reaches it whichever extreme came first —
//! under favourable-first the level is only higher and is reached the sooner.
//! So a crossing recorded here happened under **both** orderings, and the
//! crossings the old rule invented out of the favourable-first assumption are
//! simply not recorded. They are not deferred to a later bar's *price* either;
//! the give-back is a running maximum and picks them up on whatever later bar
//! genuinely retreats that far, or never.
//!
//! **What the fill is priced off is still unknown**, and only on a bar that
//! raised the peak: favourable-first anchors the order at the raised peak,
//! retreat-first at the pre-bar peak, and both prices lie inside the bar's own
//! range. Both are recorded — [`Crossings::trail_peak_at`] and
//! [`Crossings::trail_peak_raised_at`] — and [`Crossings::trail_ambiguous`]
//! names the offsets where they differ, so [`crate::grid`] can price the trail
//! twice exactly as it already prices stop-versus-target twice.
//!
//! **This changes backtest results**, in both directions: a trail that used to
//! fire on the bar that made the peak now fires later or not at all, which is
//! sometimes worse and sometimes better for the position. It is not a strict
//! haircut and is not offered as one.

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
    /// The peak each trailing rung's order hung from, as it stood BEFORE the
    /// crossing bar.
    ///
    /// A trailing stop fills at `peak - distance`, and the peak MOVES -- so the
    /// rung alone cannot price the exit the way a fixed stop's can. Without
    /// this the fill fell back to the bar's close, which on a bar that ran far
    /// past the level is not the price the order got.
    ///
    /// **Pre-bar and not in-bar.** This is the anchor under the retreat-first
    /// ordering, which is the one reading a minute bar can be held to: the
    /// crossing test itself is made against this peak, so a rung recorded here
    /// was reached whichever extreme came first. See the module header.
    trail_peak: Vec<i64>,
    /// The same crossing's peak INCLUDING that bar's own favourable extreme.
    ///
    /// Equal to `trail_peak` on every bar that did not raise the peak. Where it
    /// differs, the two are the two intra-bar orderings: favourable-first
    /// anchors the order here, retreat-first at `trail_peak`, and one-minute
    /// data cannot say which. Both prices sit inside the bar -- the pre-bar
    /// anchor at or above the low by the crossing test, the raised anchor at or
    /// below the extreme that raised it.
    trail_peak_raised: Vec<i64>,
    /// Offsets on which BOTH a stop and a target rung were newly reached, so
    /// the bar's own order decides and one-minute data does not carry it.
    ambiguous: Vec<usize>,
    /// Offsets on which a trailing rung was newly crossed by a bar that ALSO
    /// raised the peak, so the give-back is priced off an anchor the bar's own
    /// ordering decides.
    ///
    /// Separate from `ambiguous` rather than folded into it because the two
    /// resolve differently and only one of them applies to a variant: a cell
    /// with no trailing rung cannot be moved by anything in here, and counting
    /// it there would report an uncertainty that variant does not carry.
    trail_ambiguous: Vec<usize>,
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

    /// The peak trailing rung `i`'s order hung from, or `None`.
    ///
    /// What a trailing exit fills at: `peak - distance` for a long,
    /// `peak + distance` for a short. The rung alone cannot say, because the
    /// level follows the best price seen rather than sitting a fixed distance
    /// from entry.
    ///
    /// **The PESSIMISTIC anchor**, being the peak as it stood before the
    /// crossing bar. On a bar that raised the peak the favourable-first
    /// ordering would have anchored higher; that reading is
    /// [`Self::trail_peak_raised_at`], and the offsets where the two differ are
    /// [`Self::trail_ambiguous`].
    #[must_use]
    pub fn trail_peak_at(&self, rung: usize) -> Option<i64> {
        if self.trail_at(rung) == NEVER {
            return None;
        }
        self.trail_peak.get(rung).copied()
    }

    /// The same crossing's anchor under the favourable-first ordering, or
    /// `None`.
    ///
    /// **The OPTIMISTIC anchor.** Equal to [`Self::trail_peak_at`] except where
    /// the crossing bar raised the peak itself, which is exactly the set
    /// [`Self::trail_ambiguous`] names. `None` for a rung the path never
    /// crossed, for the same reason the other accessor refuses one: an anchor
    /// for a crossing that never happened would price an exit that never
    /// happened.
    #[must_use]
    pub fn trail_peak_raised_at(&self, rung: usize) -> Option<i64> {
        if self.trail_at(rung) == NEVER {
            return None;
        }
        self.trail_peak_raised.get(rung).copied()
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

    /// Offsets where a trailing rung was crossed by the bar that raised the
    /// peak.
    ///
    /// One entry per BAR, not per rung, so several rungs crossing together
    /// count once — the same convention [`Self::ambiguous`] uses, and the one
    /// [`crate::grid::Cell::ambiguous_bars`] sums. Empty means every trailing
    /// crossing hung from a peak set on an earlier bar, so its fill price is
    /// not in doubt and the two readings agree on it.
    #[must_use]
    pub fn trail_ambiguous(&self) -> &[usize] {
        &self.trail_ambiguous
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
        trail_peak: vec![0; trails.len()],
        trail_peak_raised: vec![0; trails.len()],
        ambiguous: Vec::new(),
        trail_ambiguous: Vec::new(),
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

        // THE RETREAT IS MEASURED AGAINST THE PRE-BAR PEAK, AND THE PEAK IS
        // RAISED AFTERWARDS. The peak used to improve first, which measured
        // this bar's retreat from a high this bar itself had just made -- the
        // favourable-first ordering, assumed silently on every bar. See the
        // module header: that fires rungs the retreat-first ordering never
        // fires, which is look-ahead and flatters the result.
        //
        // `anchor` is where the resting order sat when the bar opened, so
        // `anchor - low >= d` means the low reached the level under EITHER
        // ordering -- favourable-first only raises the level further above the
        // low. A crossing recorded below therefore happened either way, and
        // that is the whole reason this is the conservative reading rather than
        // merely the other guess.
        let anchor = peak;
        let raised = match side {
            Side::Long => peak.max(favourable_price),
            Side::Short => peak.min(favourable_price),
        };
        let retreat = match side {
            Side::Long => anchor.saturating_sub(adverse_price),
            Side::Short => adverse_price.saturating_sub(anchor),
        };
        give_back = give_back.max(ppm_of(retreat, entry));
        let trail_crossed_here = cross_trail_rungs(
            &mut out,
            trails.rungs(),
            &mut trail_cursor,
            give_back,
            (offset, anchor, raised),
        );
        // The bar both raised the peak and fired the trail, so which anchor
        // priced the fill is the bar's own order to decide and minute data does
        // not carry it. Recorded once per bar, like `ambiguous` below it.
        if trail_crossed_here && raised != anchor {
            out.trail_ambiguous.push(offset);
        }
        peak = raised;

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

/// Advance the trailing cursor over every rung this bar's give-back reached.
///
/// One step of [`crossings`]'s single pass, holding no state of its own: the
/// cursor belongs to the caller and only advances, because the give-back is a
/// running maximum and a crossed rung stays crossed. It is a function rather
/// than an inline block only so that walk stays under `clippy::too_many_lines`
/// -- splitting it hides nothing, which is why it was worth splitting.
///
/// `at` is `(offset, anchor, raised)`: the bar, the peak the order hung from
/// before it, and the peak its own favourable extreme left behind. Both anchors
/// are recorded for every crossing; they differ exactly on the bars
/// [`Crossings::trail_ambiguous`] names.
///
/// Returns whether any rung was newly crossed here, which is what that
/// ambiguity record turns on.
fn cross_trail_rungs(
    out: &mut Crossings,
    rungs: &[Ppm],
    cursor: &mut usize,
    give_back: Ppm,
    at: (usize, i64, i64),
) -> bool {
    let (offset, anchor, raised) = at;
    let mut crossed = false;
    while rungs.get(*cursor).is_some_and(|&rung| give_back >= rung) {
        if let Some(slot) = out.trailing.get_mut(*cursor) {
            *slot = offset;
        }
        // The two anchors THIS crossing could have hung from, recorded at the
        // crossing rather than derived later: the peak only improves, so
        // reading it at the end of the walk would price the exit against a high
        // the position never saw.
        if let Some(slot) = out.trail_peak.get_mut(*cursor) {
            *slot = anchor;
        }
        if let Some(slot) = out.trail_peak_raised.get_mut(*cursor) {
            *slot = raised;
        }
        crossed = true;
        *cursor = cursor.saturating_add(1);
    }
    crossed
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
    fn a_trailing_rung_records_the_peak_it_was_measured_against() {
        // A trailing stop fills at `peak - distance`, and the peak MOVES. The
        // rung alone cannot price it, so the crossing has to carry the peak AS
        // IT STOOD when the rung was crossed -- reading it at the end of the
        // walk would price the exit against a high the position never saw,
        // because the peak only ever improves.
        let entry = 100_000_i64;
        // FLAT bars where each peak is set, so the retreat is measured ACROSS
        // bars rather than inside one. That is deliberate and is now a property
        // of the walk rather than an accident of the fixture: the retreat is
        // measured against the peak as it stood BEFORE the bar, so a bar that
        // sets a peak can never also price the give-back off it. Bar 1's own
        // high (101,500) is below the standing peak, so nothing here is
        // ambiguous and this test measures the peak recording alone.
        let bars = vec![
            bar(0, entry + 2_000, entry + 2_000), // peak set at 102,000
            bar(1, entry + 1_000, entry + 1_500), // retreat of 1,000 from it
            bar(2, entry + 5_000, entry + 5_000), // a LATER, higher peak
        ];
        // 1,000 paisa off a 100,000 entry is 10,000 ppm.
        let trails = ladder(&[10_000]);
        let other = ladder(&[500_000]);
        let c = crossings(
            &bars,
            0,
            2,
            entry,
            Side::Long,
            Ladders {
                stops: &other,
                targets: &other,
                trails: &trails,
            },
        );

        assert_eq!(c.trail_at(0), 1, "the give-back happened on bar 1");
        assert_eq!(
            c.trail_peak_at(0),
            Some(entry + 2_000),
            "the peak recorded must be the one at the CROSSING (102,000), not \
             the higher peak the path reached afterwards (105,000)"
        );
        assert_eq!(
            c.trail_peak_raised_at(0),
            Some(entry + 2_000),
            "bar 1 did not raise the peak, so both orderings anchor the order in \
             the same place and there is nothing for the two readings to differ \
             about"
        );
        assert!(
            c.trail_ambiguous().is_empty(),
            "no crossing bar raised the peak, so no fill price is in doubt"
        );
    }

    #[test]
    fn the_bar_that_makes_a_peak_can_never_also_price_the_give_back_off_it() {
        // THE LOOK-AHEAD THIS FILE SHIPPED. The peak used to improve first and
        // the retreat was then measured from it, so ONE bar supplied both the
        // high and the give-back from that high -- the favourable-first
        // ordering, assumed silently on every bar. A backtester that does that
        // reports exits it could not have had.
        //
        // Entry 100,000, and a single bar that runs to 110,000 and dips to
        // 99,500. Against the in-bar peak the retreat is 10,500 paisa, which
        // clears a 5% rung four times over. Against the peak the resting order
        // actually hung from -- 100,000, the entry -- it is 500 paisa and clears
        // nothing.
        let entry = 100_000_i64;
        let trails = ladder(&[50_000]);
        let other = ladder(&[900_000]);
        let one_bar = vec![bar(0, entry - 500, entry + 10_000)];
        let c = crossings(
            &one_bar,
            0,
            0,
            entry,
            Side::Long,
            Ladders {
                stops: &other,
                targets: &other,
                trails: &trails,
            },
        );
        assert_eq!(
            c.trail_at(0),
            NEVER,
            "the trail fired on the bar that made the peak it was measured \
             from, which is the ordering the data does not carry"
        );
        assert_eq!(
            c.trail_peak_at(0),
            None,
            "a crossing that did not happen must carry no anchor to price it"
        );
        assert!(c.trail_ambiguous().is_empty(), "nothing crossed at all");

        // NOT SUPPRESSED, DEFERRED TO A BAR THAT ACTUALLY RETREATS. The
        // give-back is a running maximum, so the same rung is picked up by the
        // next bar whose low falls that far below the settled peak -- priced
        // off 110,000, which by then is a high the position genuinely saw.
        let two_bars = vec![
            bar(0, entry - 500, entry + 10_000),
            bar(1, entry + 4_000, entry + 5_000),
        ];
        let d = crossings(
            &two_bars,
            0,
            1,
            entry,
            Side::Long,
            Ladders {
                stops: &other,
                targets: &other,
                trails: &trails,
            },
        );
        assert_eq!(d.trail_at(0), 1, "the retreat from 110,000 is on bar 1");
        assert_eq!(d.trail_peak_at(0), Some(entry + 10_000));
    }

    #[test]
    fn a_trail_fired_by_the_bar_that_raised_the_peak_records_both_anchors() {
        // THE SIBLING OF THE AMBIGUOUS-BAR RULE. When the bar that fires the
        // trail is also the bar that raised the peak, WHETHER it fired is not
        // in doubt -- the pre-bar level sits at or above the low, so the low
        // reaches it under either ordering -- but WHAT IT FILLED AT is. The
        // retreat-first ordering anchors the order at the peak the bar opened
        // with; the favourable-first ordering at the peak the bar itself made.
        // Both anchors are recorded and the offset is named, so `crate::grid`
        // can price the exit twice the way it already prices stop-versus-target
        // twice.
        let entry = 100_000_i64;
        let trails = ladder(&[40_000]);
        let other = ladder(&[900_000]);
        // Bar 1 opens with the peak at 105,000, dips 4,000 below it -- exactly
        // the 40,000-ppm rung -- and also makes a new high at 107,000.
        let bars = vec![
            bar(0, entry, entry + 5_000),
            bar(1, entry + 1_000, entry + 7_000),
        ];
        let c = crossings(
            &bars,
            0,
            1,
            entry,
            Side::Long,
            Ladders {
                stops: &other,
                targets: &other,
                trails: &trails,
            },
        );
        assert_eq!(c.trail_at(0), 1, "the give-back cleared the rung on bar 1");
        assert_eq!(
            c.trail_peak_at(0),
            Some(entry + 5_000),
            "the pessimistic anchor is the peak the bar OPENED with"
        );
        assert_eq!(
            c.trail_peak_raised_at(0),
            Some(entry + 7_000),
            "the optimistic anchor is the peak the bar itself made"
        );
        assert_eq!(
            c.trail_ambiguous(),
            &[1],
            "a trail fired by the bar that raised its own level must be named, \
             or the better of the two fills is taken silently"
        );
        assert!(
            c.ambiguous().is_empty(),
            "no stop and no target were reachable, so the OTHER ambiguity set \
             must stay empty -- the two are counted separately because a \
             variant with no trailing rung cannot be moved by this one"
        );

        // THE MIRROR. A sign error in the short arm would swap which anchor is
        // pessimistic, and every short trail would report the flattering fill.
        let short_bars = vec![
            bar(0, entry - 5_000, entry),
            bar(1, entry - 7_000, entry - 1_000),
        ];
        let s = crossings(
            &short_bars,
            0,
            1,
            entry,
            Side::Short,
            Ladders {
                stops: &other,
                targets: &other,
                trails: &trails,
            },
        );
        assert_eq!(s.trail_at(0), 1);
        assert_eq!(
            s.trail_peak_at(0),
            Some(entry - 5_000),
            "a short's peak is its LOW, and the pessimistic anchor is still the \
             one the bar opened with"
        );
        assert_eq!(s.trail_peak_raised_at(0), Some(entry - 7_000));
        assert_eq!(s.trail_ambiguous(), &[1]);
    }

    #[test]
    fn a_trailing_rung_never_reached_carries_no_peak() {
        let entry = 100_000_i64;
        let bars = vec![bar(0, entry, entry + 100)];
        let trails = ladder(&[500_000]);
        let c = crossings(
            &bars,
            0,
            0,
            entry,
            Side::Long,
            Ladders {
                stops: &trails,
                targets: &trails,
                trails: &trails,
            },
        );
        assert_eq!(c.trail_at(0), NEVER);
        assert_eq!(
            c.trail_peak_at(0),
            None,
            "a peak for a crossing that never happened would price an exit that \
             never happened"
        );
        assert_eq!(
            c.trail_peak_raised_at(0),
            None,
            "and the optimistic anchor refuses on the same grounds -- both are \
             gated on the crossing, not on the vector's default zero"
        );
        assert_eq!(
            c.last(),
            0,
            "one bar walked, so the last offset is zero -- the accessor a caller \
             uses to say `held to the end` rather than `never crossed`"
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
