//! One pass over a trade's path, and every SL/TP answer afterwards is a lookup.
//!
//! # The problem this exists to make affordable
//!
//! A time-based exit is decided once: the exact horizon, or a proved earlier
//! session square-off. A stop-loss or a
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

    /// An ABSOLUTE ladder: `count` rungs at `step`, `2·step`, `3·step`, …
    ///
    /// # Why a quantile ladder cannot answer the question this one answers
    ///
    /// [`Self::from_excursions`] reads rungs out of the excursions a combination
    /// itself produced, so every level is relative to that signal's own
    /// behaviour. On a loose signal even the tightest rung is far: measured on a
    /// real 15-minute NIFTY run, the tightest stop the engine would try was 312
    /// index points, and no rung count reaches below a distribution's own floor.
    ///
    /// An operator asking *"which combinations survive a two point stop"* is
    /// asking about an ABSOLUTE level. That question has no expression in a
    /// relative ladder at any depth, and it is the question that matters when
    /// the rule is "no trade may run more than N points against me" — a rule
    /// about points, not about percentiles.
    ///
    /// # Stepped, so depth is the only knob
    ///
    /// One index point is the natural step, and everything coarser is a choice
    /// nobody made. `count` alone decides how far the ladder reaches: 25 rungs
    /// at a one-point step covers 1pt…25pt, which is the whole region a tight
    /// rule lives in. Raising it extends the reach; it never changes the
    /// spacing, so a level that was tried at one depth is tried at every deeper
    /// one and results stay comparable across runs.
    ///
    /// # What it does not do
    ///
    /// It invents no price. A rung is a DISTANCE an order rests at, and whether
    /// the market reached it is decided by the bars — exactly as with a derived
    /// rung. What changes is which distances get asked about.
    ///
    /// Returns `None` when `step` or `count` is zero, or when the arithmetic
    /// would leave the ladder empty; a caller must decide what to do without one
    /// rather than receive a silent default.
    #[must_use]
    pub fn stepped(step: Ppm, count: usize) -> Option<Self> {
        if step <= 0 || count == 0 {
            return None;
        }
        let rungs: Vec<Ppm> = (1..=count)
            .filter_map(|i| i64::try_from(i).ok()?.checked_mul(step))
            .collect();
        Self::new(rungs)
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
            // GEOMETRIC, NOT UNIFORM, AND THE TIGHT END IS THE WHOLE REASON.
            //
            // # What uniform spacing could never reach
            //
            // This placed rung `i` at quantile `i / (count + 1)` — with four
            // rungs, the 20th, 40th, 60th and 80th percentiles. The tightest
            // stop the engine would ever try was therefore "the 20th percentile
            // of whatever this signal happens to do", and on a loose signal that
            // is 312 index points. Measured on a real 15-minute NIFTY run: not
            // one of 1,024,058 combinations was ever tested against a stop
            // tighter than that, so a combination that works ONLY with a tight
            // stop could not be found, however many of them the sweep produced.
            //
            // Raising `count` does not help. Uniform quantiles subdivide the
            // whole distribution evenly, so the tightest rung moves from the
            // 20th percentile to the 10th to the 5th — linearly, while the
            // interesting region is the first fraction of a percent.
            //
            // # The spacing, and why it is derived rather than chosen
            //
            // Rung `i` sits at quantile `1 / 2^(count - i)`, so with four rungs
            // the ladder reads 12.5%, 25%, 50%, 100% of the way through the
            // sorted excursions — halving toward the tight end each step down.
            // A fifth rung adds 6.25% rather than shifting everything; a tenth
            // reaches 0.2%. Depth buys TIGHTNESS instead of buying resolution in
            // the middle, which is where nothing was ever in doubt.
            //
            // Nothing here is a level: every rung is still a value the data
            // actually produced, read out of the sorted array at a different
            // place. §3 rule 1 is untouched — no price is invented, and an
            // operator supplies nothing.
            // `count - i + 1`, not `count - i`: at `count - i` the last rung
            // divides by one and lands on the MAXIMUM observation, and a rung no
            // move can exceed is a rung nothing is ever measured against -- the
            // invariant this function has always stated and which the first
            // version of this geometric spacing silently broke. No test caught
            // it; one is added below.
            let from_top = count.saturating_sub(i).saturating_add(1);
            let divisor = 1_usize.checked_shl(u32::try_from(from_top).unwrap_or(u32::MAX));
            let at = match divisor {
                // `1 << from_top` overflows only past 64 rungs, and a ladder
                // that deep is asking for the single tightest observation.
                None | Some(0) => 0,
                Some(d) => observed.len().saturating_sub(1) / d,
            };
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
    /// TRAILING TAKE PROFIT: the same three tables again, once per ARMING rung.
    ///
    /// Row-major, `arm * trail_count + trail`. A trailing take profit is a
    /// trail that does not exist until the position has already made a
    /// favourable move, so its give-back is measured from the arming bar
    /// FORWARD and never from entry — which is a different running maximum, not
    /// a filtered view of `trailing`.
    ///
    /// # Why the plain trail's table cannot answer this
    ///
    /// `trailing` is the largest retreat since ENTRY. A path that wandered
    /// 30 ppm against, recovered, armed, and then gave back 10 ppm has a
    /// since-entry retreat of 30 and a since-arming retreat of 10. Reading a
    /// 20 ppm rung off `trailing` would fire the armed trail on the arming bar
    /// itself, for a give-back the position never made after arming.
    ///
    /// The arming rung indexes the TARGET ladder, so no fourth ladder exists:
    /// "start protecting once it has run this far" is a favourable-excursion
    /// question and the target ladder is already scaled on that distribution.
    armed: Vec<usize>,
    /// The pre-bar peak each armed crossing hung from, shaped like `armed`.
    armed_peak: Vec<i64>,
    /// The same crossing under the favourable-first ordering, shaped like
    /// `armed`.
    armed_peak_raised: Vec<i64>,
    /// Offsets where an ARMED rung was crossed by the bar that raised the peak.
    ///
    /// Separate from `trail_ambiguous` for the reason that list is separate
    /// from `ambiguous`: a plain-trail cell cannot be moved by an armed
    /// crossing, and charging it one would report an uncertainty it does not
    /// carry.
    armed_ambiguous: Vec<usize>,
    /// Row stride of `armed`, kept so a lookup does not have to be told it.
    trail_count: usize,
    /// Bars in this path that `Candle::check` refused.
    ///
    /// # A path with one of these cannot be priced, and used to be anyway
    ///
    /// `crate::outcome::priced` and `crate::trade::round_trip` refuse a
    /// mis-assembled record at the ENTRY bar and the EXIT bar. Every bar
    /// STRICTLY BETWEEN them still reached the walk below with no check at all,
    /// and those are the bars that decide every level exit: the MAE, the MFE and
    /// every rung crossing are read off them.
    ///
    /// Measured by an adversarial fleet, one refused record among 2,250 bars:
    ///
    /// | | clean | one refused bar mid-hold |
    /// |---|---|---|
    /// | chosen cell | `target(0)` | `trail(0) + arm(1)` |
    /// | its total | 3,210 paisa | **499,089 paisa** |
    ///
    /// A factor of 155, and a **different exit instrument recommended**. The
    /// entry/exit guard could not see it, because the corrupt bar was neither.
    ///
    /// The bar is skipped in the walk and counted here. `crate::grid` then drops
    /// the whole candidate rather than pricing a path with a hole in it — a
    /// skipped bar does not merely lose one observation, it makes the running
    /// maxima wrong for every bar after it, so the honest unit of refusal is the
    /// PATH and not the bar.
    refused: usize,
    /// The last offset walked, so a lookup can say "held to the end".
    last: usize,
}

/// A rung that the path never reached.
pub const NEVER: usize = usize::MAX;

impl Crossings {
    /// Every table sized to its ladder, every rung uncrossed.
    ///
    /// The one place the shapes are decided, so a row count and a stride cannot
    /// be spelled differently in the constructor and in the lookup. The armed
    /// tables are `targets.len()` rows of `trails.len()` because arming indexes
    /// the TARGET ladder — see [`Self::armed`].
    ///
    /// A path that is never walked — a non-positive entry, or an empty range —
    /// returns exactly this, so every rung reads [`NEVER`] and every anchor
    /// reads `None`. That is the honest answer for a trade with no path, and it
    /// is why the guard in [`crossings`] can return early without a second
    /// construction.
    fn empty(ladders: Ladders<'_>) -> Self {
        let Ladders {
            stops,
            targets,
            trails,
        } = ladders;
        let armed_cells = targets.len().saturating_mul(trails.len());
        Self {
            adverse: vec![NEVER; stops.len()],
            favourable: vec![NEVER; targets.len()],
            trailing: vec![NEVER; trails.len()],
            trail_peak: vec![0; trails.len()],
            trail_peak_raised: vec![0; trails.len()],
            ambiguous: Vec::new(),
            trail_ambiguous: Vec::new(),
            armed: vec![NEVER; armed_cells],
            armed_peak: vec![0; armed_cells],
            armed_peak_raised: vec![0; armed_cells],
            armed_ambiguous: Vec::new(),
            trail_count: trails.len(),
            refused: 0,
            last: 0,
        }
    }

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

    /// Row-major index into the three `armed` tables, if both rungs exist.
    ///
    /// `None` rather than a clamped index, so an out-of-range rung reads as
    /// "never fired" at every accessor below instead of silently answering for
    /// a different cell.
    fn armed_slot(&self, arm: usize, trail: usize) -> Option<usize> {
        if trail >= self.trail_count {
            return None;
        }
        let slot = arm.checked_mul(self.trail_count)?.checked_add(trail)?;
        (slot < self.armed.len()).then_some(slot)
    }

    /// The offset a TRAILING TAKE PROFIT would have exited on, or [`NEVER`].
    ///
    /// Armed by target rung `arm`, then trailing by trail rung `trail`. The
    /// give-back is measured from the arming bar forward — see the `armed`
    /// field for why the plain trail's table cannot be filtered into this one.
    ///
    /// **Never equal to the arming bar itself.** Arming is recorded at the END
    /// of the bar that reached the target rung, so the first bar that can fire
    /// the armed trail is the next one. Claiming a single minute both reached
    /// the target and gave back the trail distance is exactly the intra-bar
    /// ordering this module refuses to assume elsewhere.
    #[must_use]
    pub fn armed_at(&self, arm: usize, trail: usize) -> usize {
        self.armed_slot(arm, trail)
            .and_then(|slot| self.armed.get(slot).copied())
            .unwrap_or(NEVER)
    }

    /// The peak an armed crossing hung from before its bar — the PESSIMISTIC
    /// anchor.
    #[must_use]
    pub fn armed_peak_at(&self, arm: usize, trail: usize) -> Option<i64> {
        if self.armed_at(arm, trail) == NEVER {
            return None;
        }
        self.armed_slot(arm, trail)
            .and_then(|slot| self.armed_peak.get(slot).copied())
    }

    /// The same crossing under the favourable-first ordering — the OPTIMISTIC
    /// anchor.
    #[must_use]
    pub fn armed_peak_raised_at(&self, arm: usize, trail: usize) -> Option<i64> {
        if self.armed_at(arm, trail) == NEVER {
            return None;
        }
        self.armed_slot(arm, trail)
            .and_then(|slot| self.armed_peak_raised.get(slot).copied())
    }

    /// Offsets where an armed rung was crossed by the bar that raised the peak.
    ///
    /// The [`Self::trail_ambiguous`] of the trailing take profit, kept apart
    /// for the same reason: a variant that does not arm cannot be moved by
    /// anything in here.
    #[must_use]
    pub fn armed_ambiguous(&self) -> &[usize] {
        &self.armed_ambiguous
    }

    /// The last offset the path was walked to.
    #[must_use]
    pub const fn last(&self) -> usize {
        self.last
    }

    /// How many bars of this path the engine refused as not-a-bar.
    ///
    /// **Non-zero means the path is unpriceable, not merely shorter.** A skipped
    /// bar leaves every running maximum after it computed on incomplete data, so
    /// `crate::grid` drops the candidate rather than reporting a total. See the
    /// `refused` field for the measurement that forced this.
    #[must_use]
    pub const fn refused(&self) -> usize {
        self.refused
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
/// # Cost — TIME
///
/// One pass over the held bars. Every cursor only ever advances — each rung is
/// crossed at most once, and nothing rewinds and nothing is searched, which is
/// what keeps `docs/07-o1-architecture.md` layer 4's ban on `binary_search`
/// from being something to work around.
///
/// **This block said `O(bars + rungs)` and "two cursors", and the arming axis
/// made both false.** There are `3 + targets.len()` cursors: one for stops, one
/// for targets, one for the plain trail, and one per ARMING row. The armed loop
/// runs over every arming rung on every bar, so the honest figure is
///
/// ```text
/// O(bars * arm_rungs  +  rungs  +  arm_rungs * trail_rungs)
/// ```
///
/// which at the shipped four rungs is four times the per-bar trailing work. The
/// admission was written into `docs/04-invariants.md` and NOT here, so the
/// function's own doc — which is what a caller reads — stayed wrong. Nothing
/// gate-checks either copy, so `CLAUDE.md` §10's "believe the gate" does not
/// apply and both have to be right.
///
/// # Cost — SPACE, which no version of this block mentioned
///
/// [`Crossings`] grew from `Θ(S + T + R)` to `Θ(S + T + R + T·R)`, and it is
/// **retained per candidate** for the whole grid evaluation —
/// `crate::grid::Candidate` holds one. At `DEFAULT_RUNGS = 4` that is 3 × 16
/// extra slots per candidate, which is small. At the twenty rungs this module's
/// own header uses as its worked example it is 3 × 400 slots ≈ 9.6 KB per
/// candidate, and the 11,013-candidate fold `crate::validate` records would then
/// hold on the order of 100 MB.
///
/// That is a reason to keep the rung count small, not a defect at the shipped
/// four. It is stated because a space cost that appears only at a rung count
/// nobody has run is exactly the kind that is discovered by running out.
///
/// UNVERIFIED as measured figures: no bench row covers this crate, in time or
/// in space. Both are read off the loop and the type, not off an instrument.
#[must_use]
pub fn crossings(
    bars: &[Candle],
    from: usize,
    to: usize,
    entry: i64,
    side: Side,
    ladders: Ladders<'_>,
) -> Crossings {
    crossings_with(bars, from, to, entry, side, ladders, None)
}

/// [`crossings`], gated by the execution evaluator's exact per-index verdict.
///
/// This is the engine path. The public compatibility function remains useful
/// for standalone excursion arithmetic, but can see only `Candle::check`'s
/// record-local refusals. A run/grid must use this form so duplicate timestamps
/// and accumulator overflows cannot move a peak or fire an order.
#[must_use]
pub(crate) fn crossings_checked(
    bars: &[Candle],
    from: usize,
    to: usize,
    entry: i64,
    side: Side,
    ladders: Ladders<'_>,
    accepted: &[bool],
) -> Crossings {
    crossings_with(bars, from, to, entry, side, ladders, Some(accepted))
}

fn crossings_with(
    bars: &[Candle],
    from: usize,
    to: usize,
    entry: i64,
    side: Side,
    ladders: Ladders<'_>,
    accepted: Option<&[bool]>,
) -> Crossings {
    let Ladders {
        stops,
        targets,
        trails,
    } = ladders;
    let mut out = Crossings::empty(ladders);
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
    // THE ARMED PAIR, ONE PER ARMING RUNG. `armed_give_back[a]` is the largest
    // retreat seen on the bars STRICTLY AFTER target rung `a` was reached, and
    // is a running maximum for the same reason `give_back` is -- which is what
    // keeps each row's cursor advance-only and the whole thing a single pass.
    //
    // `armed_give_back[a]` stays 0 until rung `a` arms, so its cursor cannot
    // advance before then: a rung of 0 ppm would be the only exception and
    // `Ladder::from_excursions` never emits one.
    let mut armed_from: Vec<bool> = vec![false; targets.len()];
    let mut armed_give_back: Vec<Ppm> = vec![0; targets.len()];
    let mut armed_cursor: Vec<usize> = vec![0; targets.len()];

    for offset in 0..=to.saturating_sub(from) {
        let index = from.saturating_add(offset);
        let Some(bar) = bars.get(index) else {
            break;
        };
        // A BAR THE ENGINE REFUSED MAY NOT MOVE A RUNNING MAXIMUM.
        //
        // The checked engine path supplies the exact `Evaluator::step` bitmap,
        // including timestamp and accumulator state that `Candle::check` cannot
        // see. The local check remains only for the public geometry helper that
        // has no column. Counted rather than silently skipped: `crate::grid`
        // reads the count and drops the whole candidate, because a hole in the
        // middle of a path leaves every maximum after it wrong. See the
        // `refused` field for what one such bar did to a real audit -- 3,210
        // paisa to 499,089, and a different exit recommended.
        if accepted.is_some_and(|verdict| !verdict.get(index).copied().unwrap_or(false))
            || bar.check().is_err()
        {
            out.refused = out.refused.saturating_add(1);
            continue;
        }
        out.last = offset;

        let m = BarMoves::of(bar, entry, peak, side);
        let (anchor, raised, retreat_ppm) = (m.anchor, m.raised, m.retreat);
        mae = mae.max(m.adverse);
        mfe = mfe.max(m.favourable);
        give_back = give_back.max(retreat_ppm);
        let trail_crossed_here = cross_rungs(
            RungCross {
                at: &mut out.trailing,
                peak: &mut out.trail_peak,
                peak_raised: &mut out.trail_peak_raised,
            },
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

        // EVERY ARMED ROW, SAME BAR, SAME ANCHORS.
        let armed_crossed_here = cross_armed_rows(
            &mut out,
            trails.rungs(),
            ArmedState {
                armed_from: &armed_from,
                give_back: &mut armed_give_back,
                cursor: &mut armed_cursor,
            },
            retreat_ppm,
            (offset, anchor, raised),
        );
        if armed_crossed_here && raised != anchor {
            out.armed_ambiguous.push(offset);
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

        // ARMING IS RECORDED AT THE END OF THE BAR THAT REACHED THE RUNG.
        //
        // After the target cursor, so a rung reached on THIS bar arms for the
        // NEXT one. The alternative -- arming and measuring the same bar's
        // retreat -- would let one minute both reach the target and give back
        // the trail distance, which is the intra-bar ordering the header
        // refuses to assume for the plain trail and would be no better here.
        for arm in target_before..target_cursor {
            if let Some(slot) = armed_from.get_mut(arm) {
                *slot = true;
            }
        }
    }
    out
}

/// What one bar did, with the side already resolved.
///
/// Every `match side` in the walk collapsed into one place. They were seven
/// separate matches over two arms each, which is fourteen chances for a long's
/// arm to be pasted into a short's, and `a_short_arms_on_a_fall_and_trails_on_
/// the_bounce` exists because exactly that class of error is invisible in a
/// plausible-looking number.
struct BarMoves {
    /// How far this bar went AGAINST the position, in ppm of entry.
    adverse: Ppm,
    /// How far it went FOR the position, in ppm of entry.
    favourable: Ppm,
    /// The peak as the bar OPENED — the pessimistic trailing anchor.
    anchor: i64,
    /// The peak this bar's own favourable extreme leaves behind.
    raised: i64,
    /// The retreat from `anchor`, in ppm of entry.
    retreat: Ppm,
}

impl BarMoves {
    /// Resolve one bar against the entry price and the running peak.
    ///
    /// # The retreat is measured against the PRE-BAR peak
    ///
    /// The peak used to improve first, which measured this bar's retreat from a
    /// high this bar itself had just made — the favourable-first ordering,
    /// assumed silently on every bar. See the module header: that fires rungs
    /// the retreat-first ordering never fires, which is look-ahead and flatters
    /// the result.
    ///
    /// `anchor` is where the resting order sat when the bar opened, so
    /// `anchor - low >= d` means the low reached the level under EITHER
    /// ordering — favourable-first only raises the level further above the low.
    /// A crossing recorded against this therefore happened either way, and that
    /// is the whole reason it is the conservative reading rather than merely the
    /// other guess.
    fn of(bar: &Candle, entry: i64, peak: i64, side: Side) -> Self {
        // Adverse is the extreme that hurts, favourable the one that helps, and
        // which is which is the whole of what `side` decides.
        let (adverse_price, favourable_price) = match side {
            Side::Long => (bar.low, bar.high),
            Side::Short => (bar.high, bar.low),
        };
        let (adverse_move, favourable_move, raised, retreat) = match side {
            Side::Long => (
                entry.saturating_sub(adverse_price),
                favourable_price.saturating_sub(entry),
                peak.max(favourable_price),
                peak.saturating_sub(adverse_price),
            ),
            Side::Short => (
                adverse_price.saturating_sub(entry),
                entry.saturating_sub(favourable_price),
                peak.min(favourable_price),
                adverse_price.saturating_sub(peak),
            ),
        };
        Self {
            adverse: ppm_of(adverse_move, entry),
            favourable: ppm_of(favourable_move, entry),
            anchor: peak,
            raised,
            retreat: ppm_of(retreat, entry),
        }
    }
}

/// The per-arming-rung state [`crossings`] carries across bars.
///
/// Three parallel vectors, one entry per ARMING rung, bundled because they are
/// indexed together and a caller passing two of one row and one of another
/// would produce a give-back attributed to the wrong arm — silently, and with a
/// plausible number.
struct ArmedState<'a> {
    /// Whether rung `a` armed on a STRICTLY EARLIER bar.
    armed_from: &'a [bool],
    /// The largest retreat seen since rung `a` armed.
    give_back: &'a mut [Ppm],
    /// How far up the trailing ladder rung `a`'s row has been crossed.
    cursor: &'a mut [usize],
}

/// Advance every ARMED row over this bar's retreat.
///
/// One step of [`crossings`]'s pass, split out for the reason
/// [`cross_rungs`] is: the walk would otherwise exceed
/// `clippy::too_many_lines`, and nothing is hidden by the split because the
/// caller still owns every piece of state.
///
/// **Only rows armed on a strictly earlier bar advance.** `armed_from` is set at
/// the BOTTOM of the caller's loop, after the target cursor, so a rung that arms
/// on this bar first sees a retreat on the next one. See [`Crossings::armed_at`]
/// for why that is the honest reading rather than a conservative one.
///
/// `at` is `(offset, anchor, raised)`, the same triple the plain trail is given.
/// Returns whether any armed rung was newly crossed here.
fn cross_armed_rows(
    out: &mut Crossings,
    rungs: &[Ppm],
    state: ArmedState<'_>,
    retreat: Ppm,
    at: (usize, i64, i64),
) -> bool {
    let ArmedState {
        armed_from,
        give_back,
        cursor,
    } = state;
    let width = rungs.len();
    let mut crossed = false;
    // ZIPPED, NOT INDEXED, AND THAT IS A COVERAGE DECISION AS MUCH AS A STYLE
    // ONE.
    //
    // This was `for (arm, armed) in give_back.iter_mut().enumerate()` with three
    // `else { continue }` arms below it -- one for `armed_from.get(arm)`, one
    // for the three row slices, one for `cursor.get_mut(arm)`. All three are
    // structurally unreachable: the four vectors are built together from
    // `targets.len()`, and `armed.len()` is exactly `targets.len() * width`.
    //
    // Unreachable arms cost twice. `cargo llvm-cov` counts each as a region that
    // never runs, which blocks the `CLAUDE.md` §9 100% floor with branches no
    // test can take. And one of them carried a comment ARGUING for the skip --
    // "a mismatch is a bug to survive rather than a reason to abort a sweep" --
    // which is the §4 fallback that hides a failure, written down and defended,
    // for a failure that cannot happen.
    //
    // `zip` over `chunks_exact_mut` removes all three. The iterator yields only
    // rows that exist in every vector, so there is nothing left to guard and
    // nothing left uncovered.
    let rows = out
        .armed
        .chunks_exact_mut(width.max(1))
        .zip(out.armed_peak.chunks_exact_mut(width.max(1)))
        .zip(out.armed_peak_raised.chunks_exact_mut(width.max(1)));
    for (arm, ((row_at, row_peak), row_raised)) in rows.enumerate() {
        let (Some(&armed_yet), Some(held), Some(row_cursor)) = (
            armed_from.get(arm),
            give_back.get_mut(arm),
            cursor.get_mut(arm),
        ) else {
            // Unreachable for the reason above; `break` rather than `continue`
            // because a shorter parallel vector cannot lengthen later, so
            // carrying on would visit rows this one already proved absent.
            break;
        };
        if !armed_yet {
            continue;
        }
        *held = (*held).max(retreat);
        crossed |= cross_rungs(
            RungCross {
                at: row_at,
                peak: row_peak,
                peak_raised: row_raised,
            },
            rungs,
            row_cursor,
            *held,
            at,
        );
    }
    crossed
}

/// The three parallel tables one give-back series writes into.
///
/// Borrowed as slices rather than as `&mut Crossings` so the SAME step serves
/// both the plain trail -- which owns whole vectors -- and one armed row, which
/// is a window into the middle of them. Without that the armed rows would need
/// a second copy of this loop, and the two copies would be free to disagree
/// about what a crossing means.
struct RungCross<'a> {
    /// First offset each rung was crossed at.
    at: &'a mut [usize],
    /// The pre-bar anchor of that crossing.
    peak: &'a mut [i64],
    /// The same crossing's anchor under the favourable-first ordering.
    peak_raised: &'a mut [i64],
}

/// Advance one cursor over every rung this bar's give-back reached.
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
/// [`Crossings::trail_ambiguous`] and [`Crossings::armed_ambiguous`] name.
///
/// Returns whether any rung was newly crossed here, which is what that
/// ambiguity record turns on.
fn cross_rungs(
    into: RungCross<'_>,
    rungs: &[Ppm],
    cursor: &mut usize,
    give_back: Ppm,
    at: (usize, i64, i64),
) -> bool {
    let (offset, anchor, raised) = at;
    let RungCross {
        at: crossed_at,
        peak,
        peak_raised,
    } = into;
    let mut crossed = false;
    while rungs.get(*cursor).is_some_and(|&rung| give_back >= rung) {
        if let Some(slot) = crossed_at.get_mut(*cursor) {
            *slot = offset;
        }
        // The two anchors THIS crossing could have hung from, recorded at the
        // crossing rather than derived later: the peak only improves, so
        // reading it at the end of the walk would price the exit against a high
        // the position never saw.
        if let Some(slot) = peak.get_mut(*cursor) {
            *slot = anchor;
        }
        if let Some(slot) = peak_raised.get_mut(*cursor) {
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
    use super::{Ladder, Ladders, NEVER, Ppm, Side, crossings, crossings_checked};
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

    /// A statefully refused interior bar cannot move MAE/MFE, raise a trailing
    /// peak, or fire any order in either direction.
    #[test]
    fn rejected_membership_blocks_every_interior_crossing_and_peak() {
        let bars = [bar(0, 997, 1_003), bar(1, 100, 5_000), bar(2, 997, 1_003)];
        let stops = ladder(&[10_000]);
        let targets = ladder(&[10_000]);
        let trails = ladder(&[10_000]);
        let ladders = Ladders {
            stops: &stops,
            targets: &targets,
            trails: &trails,
        };
        let accepted = [true, false, true];

        for side in [Side::Long, Side::Short] {
            let unchecked = crossings(&bars, 0, 2, 1_000, side, ladders);
            assert_eq!(unchecked.stop_at(0), 1, "the poison reaches the stop");
            assert_eq!(unchecked.target_at(0), 1, "the poison reaches the target");

            let checked = crossings_checked(&bars, 0, 2, 1_000, side, ladders, &accepted);
            assert_eq!(checked.refused(), 1);
            assert_eq!(checked.stop_at(0), NEVER);
            assert_eq!(checked.target_at(0), NEVER);
            assert_eq!(checked.trail_at(0), NEVER);
            assert_eq!(checked.trail_peak_at(0), None);
            assert!(checked.ambiguous().is_empty());
            assert!(checked.trail_ambiguous().is_empty());
            assert_eq!(checked.last(), 2, "the accepted suffix is still walked");
        }
    }

    /// NO RUNG SITS ON THE MAXIMUM, AND DEPTH BUYS TIGHTNESS.
    ///
    /// # Two properties, and the first was stated for years without a test
    ///
    /// `from_excursions` has always documented that it "never places a rung at
    /// its maximum — a rung no move can exceed is a rung nothing is ever
    /// measured against". Nothing asserted it. When the spacing changed from
    /// uniform to geometric the top rung landed exactly on the maximum, the
    /// whole suite stayed green, and the defect was caught by reading the
    /// arithmetic rather than by running it. A documented invariant with no test
    /// is a comment.
    ///
    /// # The second property is what the geometric spacing exists for
    ///
    /// Uniform quantiles put the tightest rung at `1/(count+1)` of the
    /// distribution — the 20th percentile at four rungs — and raising `count`
    /// moves it linearly. So on a loose signal the engine could not test a tight
    /// stop at any depth: measured on a real 15-minute NIFTY run, the tightest
    /// stop ever tried was the 20th percentile, 312 index points, and no
    /// combination of 1,024,058 was ever asked how it behaves under anything
    /// tighter.
    ///
    /// Geometric halving makes each added rung reach half as far into the tight
    /// tail, so depth buys the region where the answer actually lives.
    #[test]
    fn the_ladder_reaches_the_tight_tail_and_never_lands_on_the_maximum() {
        // A hundred distinct excursions, so every quantile is a different value
        // and a rung landing on the maximum is unambiguous.
        let observed: Vec<Ppm> = (1..=100).map(|x| x * 10).collect();
        let max = *observed.last().expect("non-empty");

        for count in 1..=8_usize {
            let ladder = Ladder::from_excursions(&mut observed.clone(), count)
                .expect("a hundred distinct values yield a ladder");
            let rungs = ladder.rungs();
            assert!(
                rungs.iter().all(|&r| r < max),
                "count {count}: a rung at the maximum {max} can never be \
                 exceeded, so nothing is ever measured against it -- got {rungs:?}"
            );
            assert!(
                rungs.windows(2).all(|w| match w {
                    [a, b] => a < b,
                    _ => true,
                }),
                "count {count}: rungs must ascend strictly -- got {rungs:?}"
            );
        }

        // DEPTH BUYS TIGHTNESS: the tightest rung must FALL as rungs are added,
        // which is the whole difference from uniform spacing. Under uniform
        // quantiles this holds too but linearly; here each step halves, and the
        // assertion that matters is that it moves at all in the tight direction.
        let tightest = |count: usize| -> Ppm {
            Ladder::from_excursions(&mut observed.clone(), count)
                .expect("a ladder")
                .rungs()
                .first()
                .copied()
                .expect("at least one rung")
        };
        assert!(
            tightest(8) < tightest(4),
            "eight rungs must reach tighter than four: got {} vs {}",
            tightest(8),
            tightest(4)
        );
        assert!(
            tightest(4) < tightest(2),
            "and four tighter than two: got {} vs {}",
            tightest(4),
            tightest(2)
        );
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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod armed_tests {
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

    /// THE WHOLE REASON THE ARMED TABLE EXISTS, AS A PATH THAT SEPARATES THEM.
    ///
    /// A trailing take profit does not exist until the move has paid. So a
    /// give-back made BEFORE arming must not fire it — and reading the plain
    /// trail's table with a filter would do exactly that, because that table
    /// holds the largest retreat since ENTRY.
    ///
    /// The path: run up 300 ppm, fall back 200 ppm (a large PRE-ARM give-back),
    /// run to 800 ppm, then retreat 100 ppm. A 150 ppm trail:
    ///
    /// * un-armed, it fired on the 200 ppm pre-arm give-back, at bar 1;
    /// * armed at a 500 ppm target rung, arming happens on bar 2 and the only
    ///   retreat after that is 100 ppm — under the rung, so it NEVER fires.
    ///
    /// If both answered the same, the arming would be decorative.
    #[test]
    fn a_give_back_before_arming_cannot_fire_the_armed_trail() {
        let entry = 1_000_000_i64;
        let bars = vec![
            // bar 0: up 300 ppm. peak 1_000_300.
            bar(0, entry, entry + 300),
            // bar 1: back to 1_000_100 -- 200 ppm off the peak. The un-armed
            // 150 ppm trail fires HERE.
            bar(1, entry + 100, entry + 100),
            // bar 2: up 800 ppm. Crosses the 500 ppm target rung, so the trail
            // ARMS at the end of this bar.
            bar(2, entry + 100, entry + 800),
            // bar 3: back 100 ppm from the 800 peak. Under the 150 rung.
            bar(3, entry + 700, entry + 700),
        ];
        let stops = ladder(&[900_000]);
        let targets = ladder(&[500]);
        let trails = ladder(&[150]);
        let c = crossings(
            &bars,
            0,
            3,
            entry,
            Side::Long,
            Ladders {
                stops: &stops,
                targets: &targets,
                trails: &trails,
            },
        );

        assert_eq!(
            c.trail_at(0),
            1,
            "the UN-ARMED trail fires on the pre-arm give-back, which is what a \
             trailing stop loss is for"
        );
        assert_eq!(c.target_at(0), 2, "the 500 ppm rung is reached on bar 2");
        assert_eq!(
            c.armed_at(0, 0),
            NEVER,
            "ARMED at that rung, the only give-back that counts is the 100 ppm \
             after bar 2 -- under the 150 ppm trail. Reading the plain table \
             with a filter would have answered bar 1 for a retreat the position \
             never made after arming."
        );
        assert_eq!(
            c.armed_peak_at(0, 0),
            None,
            "no crossing, so no anchor: an anchor for an exit that never \
             happened would price a fill that never happened"
        );
        assert_eq!(c.armed_peak_raised_at(0, 0), None);
    }

    /// ARMING IS RECORDED AT THE END OF ITS BAR, SO THE ARM BAR CANNOT FIRE.
    ///
    /// One minute both reaching the target and giving the trail distance back is
    /// an intra-bar ordering this module refuses to assume for the plain trail;
    /// assuming it here would be no better. The bar below reaches the target AND
    /// falls far enough to clear the trail rung, and the armed exit is still the
    /// NEXT bar's.
    #[test]
    fn the_arming_bar_itself_never_fires_the_trail_it_armed() {
        // THE FIRST FIXTURE HERE PROVED NOTHING, AND AN ADVERSARIAL READ FOUND
        // IT.
        //
        // It was `bar(0, entry + 100, entry + 600)` -- arming bar low ABOVE the
        // entry. The pre-bar anchor on bar 0 is the entry itself, so
        // `retreat = entry - (entry + 100)` is negative and `retreat_ppm` is 0.
        // The armed row could not cross on bar 0 whatever the arming order was,
        // so hoisting the `armed_from` block above `cross_armed_rows` -- which
        // is exactly the rule under test -- left every assertion passing.
        //
        // The docstring compounded it: it said the bar "falls far enough to
        // clear the trail rung", which measures against the RAISED peak. The
        // code measures against the pre-bar one.
        //
        // This fixture discriminates. The arming bar's low is BELOW the running
        // peak by more than the rung, so start-of-bar arming would fire it and
        // end-of-bar arming cannot.
        let entry = 1_000_000_i64;
        let bars = vec![
            // bar 0: flat at +400. Peak becomes entry + 400. No target yet.
            bar(0, entry + 400, entry + 400),
            // bar 1: low +100 -- a 300 ppm retreat from the +400 anchor, three
            // times the 100 ppm rung -- AND a high of +600, which crosses the
            // 500 ppm target rung and arms. Under start-of-bar arming the armed
            // row would cross HERE. Under end-of-bar arming it cannot.
            bar(1, entry + 100, entry + 600),
            // bar 2: flat at +600, no retreat from the +600 peak.
            bar(2, entry + 600, entry + 600),
        ];
        let stops = ladder(&[900_000]);
        let targets = ladder(&[500]);
        let trails = ladder(&[100]);
        let c = crossings(
            &bars,
            0,
            2,
            entry,
            Side::Long,
            Ladders {
                stops: &stops,
                targets: &targets,
                trails: &trails,
            },
        );

        assert_eq!(c.target_at(0), 1, "the 500 ppm rung is reached on bar 1");
        assert_eq!(
            c.trail_at(0),
            1,
            "the UN-ARMED trail fires on bar 1's 300 ppm retreat -- so the bar \
             genuinely clears the rung, and the armed row's silence is the \
             arming rule and not an absent give-back"
        );
        assert_eq!(
            c.armed_at(0, 0),
            NEVER,
            "the arming bar's own retreat is not the armed trail's. Arming is \
             recorded at the END of bar 1, and bar 2 retreats not at all"
        );
        assert_eq!(c.armed_peak_at(0, 0), None);
    }

    /// A RUNG PAIR THAT DOES NOT EXIST READS AS NEVER, NOT AS SOMEONE ELSE'S.
    ///
    /// The table is row-major and flat, so an out-of-range trailing rung would
    /// otherwise wrap into the next arming row and answer for a different cell
    /// entirely — silently, and with a plausible number.
    #[test]
    fn an_out_of_range_rung_pair_answers_never_and_not_a_neighbour() {
        let entry = 1_000_000_i64;
        let bars = vec![bar(0, entry, entry + 900), bar(1, entry, entry)];
        let stops = ladder(&[900_000]);
        let targets = ladder(&[100, 200]);
        let trails = ladder(&[50, 60]);
        let c = crossings(
            &bars,
            0,
            1,
            entry,
            Side::Long,
            Ladders {
                stops: &stops,
                targets: &targets,
                trails: &trails,
            },
        );

        assert_eq!(c.armed_at(0, 0), 1, "row 0 column 0 is real");
        assert_eq!(
            c.armed_at(0, 2),
            NEVER,
            "column 2 does not exist; without the stride guard this is row 1 \
             column 0"
        );
        assert_eq!(c.armed_at(9, 0), NEVER, "row 9 does not exist");
        assert_eq!(c.armed_peak_at(0, 2), None);
        assert_eq!(c.armed_peak_raised_at(9, 0), None);
    }

    /// AN ARMED CROSSING ON A PEAK-RAISING BAR IS ITS OWN AMBIGUITY SET.
    ///
    /// It cannot be charged to `trail_ambiguous`, because a plain-trail variant
    /// is not exposed to it: the two crossings happen on different bars, and a
    /// bar that made one unknowable need not have made the other unknowable at
    /// all.
    #[test]
    fn the_armed_ambiguity_set_is_separate_from_the_plain_trail_s() {
        // THE FIRST FIXTURE DEMONSTRATED THE OPPOSITE OF THE TEST'S NAME.
        //
        // In it the plain trail crossed on the same bar, with the same two
        // anchors, so `trail_ambiguous() == armed_ambiguous() == [1]` and every
        // field of the two tables agreed. A mutant
        // `fn armed_ambiguous(&self) -> &[usize] { &self.trail_ambiguous }`
        // survived it and every other armed test -- the rest leave both sets
        // empty. A test whose fixture cannot separate the two things it is
        // named for is `CLAUDE.md` §4's test that asserts nothing.
        //
        // This one separates them. The plain trail crosses EARLY, on a bar that
        // raises no peak, so its ambiguity set stays empty; the armed row
        // crosses LATER, on a bar that does raise the peak.
        let entry = 1_000_000_i64;
        let bars = vec![
            // bar 0: flat at +200. Peak = entry + 200.
            bar(0, entry + 200, entry + 200),
            // bar 1: flat at +140 -- a 60 ppm retreat from the +200 anchor,
            // over the 50 ppm rung, so the UN-ARMED trail crosses here. The
            // bar raises no peak, so it is NOT ambiguous.
            bar(1, entry + 140, entry + 140),
            // bar 2: up to +600, crossing the 500 ppm target rung. Arms at the
            // end of this bar.
            bar(2, entry + 140, entry + 600),
            // bar 3: low +550 -- a 50 ppm retreat from the +600 anchor, which
            // crosses the armed rung -- AND a high of +900, which raises the
            // peak. So THIS crossing is ambiguous and the plain trail's was not.
            bar(3, entry + 550, entry + 900),
        ];
        let stops = ladder(&[900_000]);
        let targets = ladder(&[500]);
        let trails = ladder(&[50]);
        let c = crossings(
            &bars,
            0,
            3,
            entry,
            Side::Long,
            Ladders {
                stops: &stops,
                targets: &targets,
                trails: &trails,
            },
        );

        assert_eq!(c.trail_at(0), 1, "the un-armed trail crosses on bar 1");
        assert_eq!(
            c.trail_ambiguous(),
            [] as [usize; 0],
            "and bar 1 raised no peak, so nothing about that fill is in doubt"
        );
        assert_eq!(c.target_at(0), 2, "armed at the end of bar 2");
        assert_eq!(c.armed_at(0, 0), 3, "the armed row crosses on bar 3");
        assert_eq!(
            c.armed_ambiguous(),
            [3],
            "which DID raise the peak. The two sets are different sets, and a \
             plain-trail variant cannot be charged for this one"
        );
        assert_eq!(
            c.armed_peak_at(0, 0),
            Some(entry + 600),
            "pessimistic: the peak bar 3 opened with"
        );
        assert_eq!(
            c.armed_peak_raised_at(0, 0),
            Some(entry + 900),
            "optimistic: the peak bar 3's own high left behind"
        );
    }

    /// A SHORT ARMS AND TRAILS THE SAME WAY, WITH EVERY EXTREME MIRRORED.
    ///
    /// The side decides which extreme helps and which hurts, and it decides it
    /// in one place. A short whose armed trail behaved like a long's would be a
    /// sign-error hidden behind a plausible number, so it is asserted rather
    /// than assumed from the long case.
    #[test]
    fn a_short_arms_on_a_fall_and_trails_on_the_bounce() {
        let entry = 1_000_000_i64;
        let bars = vec![
            // A short profits as price FALLS. Down 600 arms the 500 rung.
            bar(0, entry - 600, entry),
            // Bounce to -400: a 200 ppm retreat from the -600 peak.
            bar(1, entry - 400, entry - 400),
        ];
        let stops = ladder(&[900_000]);
        let targets = ladder(&[500]);
        let trails = ladder(&[150]);
        let c = crossings(
            &bars,
            0,
            1,
            entry,
            Side::Short,
            Ladders {
                stops: &stops,
                targets: &targets,
                trails: &trails,
            },
        );

        assert_eq!(c.target_at(0), 0, "a 600 ppm FALL is the short's favour");
        assert_eq!(c.armed_at(0, 0), 1, "and the bounce is its give-back");
        assert_eq!(
            c.armed_peak_at(0, 0),
            Some(entry - 600),
            "a short's peak is its LOW, and the anchor mirrors with it"
        );
    }

    /// NO TARGET LADDER MEANS NO ARMING, AND AN EMPTY TABLE RATHER THAN A PANIC.
    ///
    /// Arming indexes the target ladder, so a combination whose favourable
    /// excursions produced no ladder has nothing to arm on. The row count is
    /// then zero and every lookup must still answer.
    #[test]
    fn a_path_with_no_target_ladder_has_no_armed_rows_at_all() {
        let entry = 1_000_000_i64;
        let bars = vec![bar(0, entry, entry + 900), bar(1, entry, entry)];
        let stops = ladder(&[900_000]);
        let targets = Ladder::default();
        let trails = ladder(&[50]);
        let c = crossings(
            &bars,
            0,
            1,
            entry,
            Side::Long,
            Ladders {
                stops: &stops,
                targets: &targets,
                trails: &trails,
            },
        );

        assert_eq!(c.trail_at(0), 1, "the plain trail still works");
        assert_eq!(c.armed_at(0, 0), NEVER, "and nothing can arm it");
        assert!(c.armed_ambiguous().is_empty());
    }
}
