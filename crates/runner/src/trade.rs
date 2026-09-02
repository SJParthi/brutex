//! Signals turned into TRADES — one position at a time, filled on the next bar.
//!
//! # What this replaces, and why the difference is not small
//!
//! [`crate::outcome`] answers "on every bar this combination fired, what did the
//! price do next". That is a property of the SIGNAL. It is not a property of a
//! strategy, because it counts a mask that fires on a hundred consecutive bars
//! as a hundred independent observations, when a trader holding one position
//! took **one** trade.
//!
//! Overlapping observations are not independent, and every statistic downstream
//! assumes they are: the mean, the Welford variance, the t-statistic, the
//! Bonferroni bar computed from a trial count. Inflating `n` by a factor of
//! however long a signal persists inflates `t` by roughly its square root, and
//! nothing in the report would look wrong.
//!
//! # The execution model, stated by the operator and enforced here
//!
//! 1. **Everything is intraday.** Entry from 09:15 IST, and the position is
//!    squared off at the fixed 15:10 IST policy deadline, long or short,
//!    whether or not the horizon has run out. With left-labelled bars, only the
//!    accepted unique 15:09 row can price it. Nothing is ever held overnight.
//! 2. **Fills happen on the NEXT bar.** A signal is a fact about a bar's close;
//!    you cannot trade at a price that has already printed. A signal on bar `N`
//!    fills on bar `N + 1`.
//! 3. **Two cases, both reported, neither chosen.** BEST is the next bar's open.
//!    WORST is the PRINTED extreme of the next bar — the high if you are buying,
//!    the low if you are selling. A single number would hide which of the two a
//!    result depended on.
//! 4. **One position at a time.** While a trade is open, no further signal opens
//!    anything — not another long, and not a short. The next entry must be
//!    strictly after the previous exit bar; an exit minute cannot also be a new
//!    entry minute.
//!
//! # Both fills go through `crates/costs`, and NEITHER adds a tick
//!
//! Both legs of both cases are [`costs::fill::fills_at`], differing only in the
//! [`costs::fill::Anchor`]: `Open` for the best, `PrintedExtreme` for the worst.
//! One code path, two scenarios — a best case computed by a different function
//! would be comparing two models rather than two readings.
//!
//! **The worst case used to be `AdverseExtreme`: the high PLUS one tick and the
//! low MINUS one tick.** That tick is a claim about microstructure rather than a
//! reading of the data, and `costs::fill::Anchor`'s own doc makes the objection
//! in those terms — *"a fill invented one tick outside them is exactly the
//! invention `CLAUDE.md` §3 rule 1 forbids"*. On a series whose finest
//! resolution IS one minute, the four numbers of the bar are the whole of what
//! is known, and a price one tick outside them is a price nobody traded at.
//!
//! It also ended a disagreement between the two halves of the pipeline.
//! [`crate::grid`] has always priced its exit ladder at `PrintedExtreme`, so the
//! same trades on the same bars cost two ticks per round trip MORE here than
//! there — and the grid's number is the one selection ranks on. The two now read
//! the same bars the same way.
//!
//! **What is lost is the slippage allowance, and it is not smuggled back as a
//! smaller number: it is gone.** Every figure here is a fill at a price that
//! printed, with no execution penalty modelled at all. That is a stated limit,
//! not an invisible optimism, and `docs/06-limits.md` is where it belongs.
//!
//! # What is NOT here, and why it is not invented
//!
//! Brokerage, STT, stamp duty, exchange charges and GST. `costs::trip::price`
//! computes all of them and needs a [`costs::trip::Contract`], a broker and a
//! quantity. The sweep runs on **spot indices**, and a spot index is not
//! tradeable: the contract that would actually be bought is a future or an
//! option, and choosing which is a decision `CLAUDE.md` §3 rule 1 forbids this
//! module from making up.
//!
//! Slippage is no longer applied either -- see above. So every figure here is
//! **gross of charges AND gross of slippage**: a fill at a price the bar
//! actually printed, and nothing modelled beyond that.
//!
//! # Measured: what the two rules cost the old numbers
//!
//! **These figures were taken while the worst case still added a tick per leg,
//! and they are kept for what they proved rather than as a description of this
//! code.** The `worst` column below is two ticks per round trip harsher than
//! what this module now computes; the `signals`, `trades` and `blocked` columns
//! are untouched by the fill model and still hold.
//!
//! `synthetic::sessions(8)`, the empty mask (which fires on every swept bar, so
//! this is the extreme case), long, per horizon:
//!
//! | H | signals | trades | blocked while open | best | worst |
//! |---|---|---|---|---|---|
//! | 5 | 1,124 | 177 | 884 | +882 p | **−27,883 p** |
//! | 15 | 1,124 | **68** | 994 | +2,302 p | **−8,658 p** |
//! | 60 | 1,124 | 18 | 1,045 | +2,955 p | +70 p |
//!
//! Two things fall out of that table and neither is small.
//!
//! **`n` was inflated 16.5x at the default horizon.** [`crate::outcome::edge`]
//! would have counted 1,124 observations where a trader took 68 trades. A
//! t-statistic scales with the square root of `n`, so that alone overstates it
//! by roughly **4x** — before any question of whether the edge is real.
//!
//! **Slippage alone flipped the sign, WHEN SLIPPAGE WAS MODELLED.** At H=5 and
//! H=15 the best case was profitable and the worst case a heavy loss; only at
//! H=60, with 18 trades instead of 177, did the worst case stay positive. The
//! mechanism was not subtle: each round trip paid two ticks, so ten times the
//! trades paid ten times the spread.
//!
//! **That sensitivity has not gone away -- the model of it has.** The tick is no
//! longer charged, so these worst-case figures would now read far better on the
//! same bars. What the table still proves is that a high-frequency variant is
//! the one execution cost punishes hardest, and NOTHING in this module now
//! charges it. `docs/06-limits.md` carries that as a stated limit.
//!
//! These are figures from a SYNTHETIC fixture and prove nothing about the
//! market. They are here because they size the DEFECT, and that is a fact about
//! this code rather than about NIFTY.

use costs::fill::{Anchor, Bar as FillBar, Direction, fills_at};
use indicators::Candle;
use indicators::column::Column;
use vocab::ConditionMask;

use crate::outcome::{Horizon, SessionBounds, median_step_micros, median_step_micros_over};

/// The authoritative execution cadence promised by this engine surface.
///
/// This is deliberately not inferred from the stored slice. A damaged or
/// sparse one-minute file can have a two- or five-minute median gap; using that
/// observation as the cadence would turn a missing immediate bar into a delayed
/// fill and silently change the strategy.
const EXECUTION_MINUTE_MICROS: i64 = 60_000_000;

/// One completed round trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trade {
    /// The bar whose close carried the signal. Nothing is traded on it.
    pub signal_bar: usize,
    /// The bar the entry filled in — always `signal_bar + 1`.
    pub entry_bar: usize,
    /// The bar the exit filled in: the horizon, or that session's square-off,
    /// whichever came first.
    pub exit_bar: usize,
    /// Paisa per unit if both legs filled at the bar's OPEN, with no slippage.
    ///
    /// The most favourable fill that could have happened: the open is a price
    /// that PRINTED, so this is reachable rather than hypothetical. It assumes
    /// no queue, no spread crossed and no movement between the decision and the
    /// fill, which is why it is a bound and not an expectation.
    ///
    /// **This used to carry one adverse tick** and its doc said so. That was a
    /// consequence of `costs::fill::Bar` not carrying an open: the only way to
    /// anchor there was a flat bar run through the adverse rule, which still
    /// charged the tick. `costs::fill::Anchor::Open` reads the open directly, so
    /// the best case is now the best case.
    pub best: i64,
    /// Paisa per unit if both legs filled at the bar's PRINTED extreme — the
    /// high buying, the low selling, **with no tick added**.
    ///
    /// Never better than [`Self::best`], and the gap between them is the whole
    /// range of outcomes a real fill can land in. **Selection ranks on THIS
    /// one**, unchanged: a search ranked on the flattering reading picks
    /// whatever the flattering assumption helped most.
    ///
    /// **This used to carry one adverse tick on each leg** and its doc said so.
    /// A tick outside the bar is a price nobody traded at, which on a one-minute
    /// series is the invention §3 rule 1 forbids — `costs::fill::Anchor` makes
    /// that objection in its own words. It also disagreed with
    /// [`crate::grid`], which has always priced at the printed extreme, by two
    /// ticks per round trip on the same bars.
    ///
    /// So this is now the worst price the bar ACTUALLY PRINTED, and no execution
    /// penalty is modelled anywhere. That is a real loss of conservatism and it
    /// is stated rather than hidden.
    pub worst: i64,
    /// True when the exit was that session's square-off rather than the horizon.
    ///
    /// Counted because a strategy whose trades are mostly force-closed is not
    /// the strategy its horizon describes — it is a different one wearing that
    /// horizon's name.
    pub forced: bool,
}

/// One signal's occupancy interval, whether or not both fills were priceable.
///
/// The entry can be known to have happened while the exit record is corrupt.
/// Such a path contributes no money and no [`Trade`], but it still owns the
/// position through [`Self::exit_bar`]. Keeping that fact in the same ordered
/// stream as priceable paths prevents both this walk and every exit-grid cell
/// from promoting a later overlapping signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Occupancy {
    /// The bar whose completed condition set caused this entry.
    ///
    /// On a same-rung walk this is the bar immediately before [`Self::entry_bar`].
    /// On a reprojected column it is the execution-series source stamped at the
    /// signal close, which is also the immediate fill bar. Kept rather than
    /// reconstructed so a chosen-grid replay can persist the same provenance as
    /// the level-less walk without guessing which column convention produced it.
    pub signal_bar: usize,
    /// The execution bar on which the position opened.
    pub entry_bar: usize,
    /// The latest bar through which it must conservatively remain occupied.
    pub exit_bar: usize,
    /// Whether both legs were priceable and therefore have a [`Trade`].
    pub priceable: bool,
}

/// Every trade one combination produced, and what was skipped to get there.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trades {
    /// The round trips, in bar order and never overlapping.
    pub trades: Vec<Trade>,
    /// Every signal that COULD have opened a position, exclusivity not applied.
    ///
    /// # Why a second list, and what its absence cost
    ///
    /// [`Self::trades`] is the answer under rule 4 with LEVEL-LESS exits — the
    /// longest possible holds, so the most exclusion. `crate::grid::evaluate`
    /// built its candidates from it, and its own per-variant exclusivity then
    /// had nothing to do: a tighter stop frees the next signal only if that
    /// signal is in the list, and rule 4 had already removed it.
    ///
    /// So **all 625 exit cells measured one trade set** — the time-exit
    /// baseline's — while `crate::grid`'s module header claims in writing that
    /// the sequence is re-walked per variant because "a stop that fires early
    /// ends the position early, and that frees the NEXT signal to be taken
    /// sooner". An adversarial fleet measured the guard firing **zero times
    /// across 168,892 cells**.
    ///
    /// This is that universe: every signal that passed rules 1, 1b, 1c and 2,
    /// whether or not a position happened to be open at the time. Exclusivity is
    /// then whoever applies it — rule 4 here for [`Self::trades`], and
    /// `one_variant`'s own `open_until` for each grid cell, which is now the
    /// only place it can differ.
    ///
    /// **A superset of `trades`, always**, and equal to it exactly when no
    /// signal was ever blocked. `an_eligible_signal_blocked_here_can_open_a_cell`
    /// holds the containment.
    pub eligible: Vec<Trade>,
    /// Every path with a priceable entry, in signal order.
    ///
    /// [`Self::eligible`] contains only paths whose entry AND exit can be
    /// priced. This stream also retains an exit-unpriceable path as a
    /// block-only interval. It is therefore the sequence an exclusivity walk
    /// consumes; removing a block-only member would invent room for the next
    /// signal.
    pub occupancy: Vec<Occupancy>,
    /// Bars the mask fired on, whether or not they became a trade.
    ///
    /// The number [`crate::outcome::edge`] would have used as its `n`. Reported
    /// beside [`Self::trades`] because the ratio between them is how much of the
    /// old figure was overlap.
    pub signals: u64,
    /// Signals ignored because a position was already open.
    pub while_open: u64,
    /// Signals that produced no trade: at or past the square-off, at the end
    /// of a session, or at the end of the slice — and signals whose ENTRY the
    /// slice allowed but whose EXIT it did not, which is RULE 1c below.
    ///
    /// That last case is not a lateness of the signal at all; it is the data
    /// stopping mid-session. It is counted here rather than in a bucket of its
    /// own because [`Self::reconciles`] admits exactly three outcomes and a
    /// fourth would be a format change to every caller of this struct. What it
    /// costs is stated rather than hidden: a slice cut mid-day inflates
    /// `too_late` by however many signals its tail carried, and nothing here
    /// separates them from the square-off refusals.
    pub too_late: u64,
}

impl Trades {
    /// How many round trips were taken.
    #[must_use]
    pub fn count(&self) -> u64 {
        u64::try_from(self.trades.len()).unwrap_or(u64::MAX)
    }

    /// Every signal is either a trade, blocked by an open position, or too
    /// late. Nothing else can happen to one.
    ///
    /// A reconciliation rather than a nicety: a walk that silently dropped a
    /// signal would show up here and nowhere else, and `crates/engine`'s
    /// `Frontier::reconciles` exists because exactly that went unnoticed for a
    /// long time on the level counts.
    #[must_use]
    pub fn reconciles(&self) -> bool {
        self.count()
            .saturating_add(self.while_open)
            .saturating_add(self.too_late)
            == self.signals
    }
}

/// Walk one combination's signals into non-overlapping intraday round trips.
///
/// `bars` must be the same slice `column` was built from — [`Column::sources`]
/// indexes into it, and pairing a column with another slice's bars is the defect
/// [`crate::outcome::Forward::built_from_same_slice_as`] exists to refuse.
///
/// # Cost
///
/// One pass over the column. Per signal: one mask test, one same-day comparison,
/// two `costs::fill::Bar` constructions and two `worst_case_fills`, each a fixed
/// count of integer operations. The forced-close index comes from a table built
/// once in [`forced_exits`], so no per-signal search walks the session.
/// `CLAUDE.md` §3 rule 4.
///
/// **This entry point derives [`SliceFacts`] on the way in, so it is O(bars) and
/// allocates the median-step sample per CALL.** That is the right cost for a
/// one-off; a loop over candidates must hoist the facts and use [`walk_over`].
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn walk(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    direction: Direction,
) -> Trades {
    walk_over(
        bars,
        column,
        mask,
        horizon,
        direction,
        &SliceFacts::of(bars, column),
    )
}

/// Everything about a SLICE that [`walk`] needs and that no candidate can
/// change.
///
/// # One struct, because there were two of these and only one was hoisted
///
/// [`walk_with`] exists to lift [`forced_exits`] out of the per-candidate loop,
/// and its own doc explains why at length — `crate::rank` cites a real run at
/// **17.8 million survivors against a 91,874-bar column**, and rebuilding an
/// O(bars) table for each of them is work that scales uniformly and that no
/// ratio gate can therefore see.
///
/// **A second per-slice quantity was left in the loop, and it is the more
/// expensive of the two.** `walk_with` called [`median_step_micros`] on every
/// entry under a comment reading *"MEASURED ONCE: the bar spacing is a property
/// of the slice, not of a signal"* — true of the quantity and false of the call,
/// which happened once per CANDIDATE. That function allocates a `Vec<i64>` of
/// `bars.len() − 1`; the version the audit found also sorted it. The selection
/// is linear now, but the roughly 5 MB allocation per candidate at 623,546 bars
/// remains the defect hoisting removes.
///
/// The measured shape of the exposure, from the audit that found it: `crates/cli`
/// calls `grid::evaluate` from a `par_iter` over up to 10,000 screened
/// candidates, and `crate::validate` calls `evaluate_with` AND `walk_with`
/// inside one `par_iter` over the closed frequent set — two median allocations
/// per candidate there. `CLAUDE.md` §3 rule 4 is what that violates, and
/// `walk_with`'s own doc had already written down why nothing would catch it.
///
/// So both facts live here, derived once, and [`walk_over`] takes them together.
/// A caller that hoists one and forgets the other is the defect this type exists
/// to make unspellable.
///
/// # Cost
///
/// One reverse session-boundary derivation and one linear projection into the
/// square-off table. A checked [`indicators::column::Sourced::Fill`] column has
/// the engine's authoritative one-minute execution cadence; a native
/// [`indicators::column::Sourced::Signal`] compatibility walk retains the
/// measured rung cadence. Every per-candidate lookup afterwards is a
/// bounds-checked read.
pub struct SliceFacts {
    /// [`forced_exits`] over the slice.
    exits: Vec<Option<SquareOff>>,
    /// The authoritative one-minute cadence for fill-sourced execution, or the
    /// measured native rung cadence for a signal-sourced compatibility walk.
    step_micros: i64,
    /// The exact evaluator verdict captured on the column for this execution
    /// slice. An [`Arc`] clone shares the one bitmap; it does not allocate one
    /// per candidate.
    accepted: Option<std::sync::Arc<[bool]>>,
    /// Prefix count of refused bars. This is derived from (not a second copy
    /// of) `accepted` and makes "does this whole path contain a hole?" one
    /// subtraction instead of a scan per candidate/grid cell.
    refused_prefix: Vec<u64>,
    /// Prefix count of timestamp gaps that do not equal `step_micros`.
    /// This turns “is every execution minute present on the held path?” into
    /// the same two-read O(1) check as evaluator acceptance.
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    broken_prefix: Vec<u64>,
    /// Accepted bars by exact timestamp. Built once per slice so an exit at a
    /// wall-clock deadline is a lookup rather than an H-bar scan.
    at_timestamp: std::collections::HashMap<i64, usize>,
}

impl SliceFacts {
    /// Derive every execution-slice fact from `bars` and its column, once.
    #[must_use]
    pub fn of(bars: &[Candle], column: &Column) -> Self {
        let accepted = column
            .acceptance()
            .filter(|verdict| verdict.len() == bars.len());
        let step_micros = match column.sourced() {
            indicators::column::Sourced::Fill => EXECUTION_MINUTE_MICROS,
            indicators::column::Sourced::Signal => {
                median_step_micros_over(bars, accepted.as_deref())
            }
        };
        let mut at_timestamp = std::collections::HashMap::with_capacity(bars.len());
        let mut refused_prefix: Vec<u64> = Vec::with_capacity(bars.len().saturating_add(1));
        let mut broken_prefix: Vec<u64> = Vec::with_capacity(bars.len().saturating_add(1));
        refused_prefix.push(0);
        broken_prefix.push(0);
        let mut prior_timestamp: Option<i64> = None;
        if let Some(verdict) = accepted.as_deref() {
            for (index, bar) in bars.iter().enumerate() {
                let is_accepted = verdict.get(index).copied().unwrap_or(false);
                let refused = refused_prefix.last().copied().unwrap_or(0);
                refused_prefix.push(refused.saturating_add(u64::from(!is_accepted)));
                let broken = prior_timestamp.is_some_and(|prior| {
                    step_micros <= 0 || bar.ts_micros.saturating_sub(prior) != step_micros
                });
                let gaps = broken_prefix.last().copied().unwrap_or(0);
                broken_prefix.push(gaps.saturating_add(u64::from(broken)));
                prior_timestamp = Some(bar.ts_micros);
                if is_accepted {
                    at_timestamp.entry(bar.ts_micros).or_insert(index);
                }
            }
        }
        Self {
            exits: forced_exits_with_step(bars, step_micros, accepted.as_deref()),
            step_micros,
            accepted,
            refused_prefix,
            broken_prefix,
            at_timestamp,
        }
    }

    /// The square-off table, for a caller that already holds one and wants to
    /// pass it on rather than rebuild it.
    #[must_use]
    pub fn exits(&self) -> &[Option<SquareOff>] {
        &self.exits
    }

    /// The execution/native cadence selected by [`Self::of`], in microseconds.
    #[must_use]
    pub const fn step_micros(&self) -> i64 {
        self.step_micros
    }

    /// Did the evaluator accept bar `index` on this exact execution slice?
    #[must_use]
    pub fn accepts(&self, index: usize) -> bool {
        self.accepted
            .as_deref()
            .and_then(|verdict| verdict.get(index))
            .copied()
            .unwrap_or(false)
    }

    /// Borrow the evaluator verdict for path walkers.
    ///
    /// An unknown compatibility projection yields the empty slice, under which
    /// every lookup refuses. Shipping callers construct facts from a native or
    /// checked-reprojected column, so they receive one entry per bar.
    #[must_use]
    pub fn acceptance(&self) -> &[bool] {
        self.accepted.as_deref().unwrap_or(&[])
    }

    /// Does this fact set carry one evaluator verdict for every supplied bar?
    #[must_use]
    pub fn covers(&self, bars: &[Candle]) -> bool {
        self.accepted
            .as_deref()
            .is_some_and(|verdict| verdict.len() == bars.len())
    }

    /// The accepted bar stamped at exactly `timestamp`, if one exists.
    #[must_use]
    pub fn at_timestamp(&self, timestamp: i64) -> Option<usize> {
        self.at_timestamp.get(&timestamp).copied()
    }

    /// Did every bar in the inclusive path `from..=to` pass the evaluator, and
    /// did every adjacent timestamp advance by exactly this slice's cadence?
    ///
    /// Two prefix reads and one subtraction. `false` for a reversed or
    /// out-of-range interval and when no checked acceptance map exists.
    #[must_use]
    pub fn path_accepts(&self, from: usize, to: usize) -> bool {
        if from > to {
            return false;
        }
        let Some(before) = self.refused_prefix.get(from).copied() else {
            return false;
        };
        let Some(after) = self.refused_prefix.get(to.saturating_add(1)).copied() else {
            return false;
        };
        let Some(gaps_before) = self.broken_prefix.get(from.saturating_add(1)).copied() else {
            return false;
        };
        let Some(gaps_after) = self.broken_prefix.get(to.saturating_add(1)).copied() else {
            return false;
        };
        after == before && gaps_after == gaps_before
    }
}

/// [`walk`], over the per-slice facts the caller already derived.
///
/// **This is the entry point a per-candidate loop must use.** It performs no
/// allocation and no sort of its own: everything that is a property of the slice
/// arrives in [`SliceFacts`], and what is left is one pass over the column.
///
/// `crates/cli` and `crate::validate` reach this through
/// [`crate::grid::evaluate_with`] as well as directly, and both of those loops
/// are the ones the type's doc measures.
#[must_use]
pub fn walk_over(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    direction: Direction,
    facts: &SliceFacts,
) -> Trades {
    walk_core(bars, column, mask, horizon, direction, facts.exits(), facts)
}

/// [`walk`], over a square-off table the caller already built.
///
/// # Why this exists, and it is the same table every time
///
/// [`forced_exits`] is a **pure function of `bars`** — the mask, the direction
/// and the horizon never reach it — and `walk` rebuilt it on entry. `walk` is
/// the per-CANDIDATE entry point, so a sweep paid, for every survivor:
///
/// * one `Vec<(i64, i64)>` of `bars.len()`,
/// * one `vec![None; bars.len()]`,
/// * and a reverse pass calling `ist_day` and `minute_of_day` on every bar.
///
/// `crates/runner/src/validate.rs` calls it **twice** per candidate — once
/// directly and once inside `grid::evaluate` — from a `par_iter` over the
/// closed frequent set. `crate::rank` cites a real run at **17.8 million
/// survivors** against a 91,874-bar column.
///
/// Constant per candidate, so no ratio gate could see it: work that scales
/// uniformly does not change a ratio. The same shape as `read_row`'s
/// per-record allocation and `ingest`'s per-row counting loop.
///
/// # THIS ENTRY POINT IS SUPERSEDED AND STILL REALLOCATES
///
/// It hoists the square-off table and NOT the bar spacing, so even the `Some`
/// path pays one [`median_step_micros`] — a `Vec<i64>` of `bars.len() − 1` — on
/// every call. That is another full-slice allocation beside the O(bars) table it
/// was written to avoid rebuilding, and it is exactly the shape its own
/// paragraph above warns cannot be seen by a ratio gate.
///
/// It is kept so every existing caller compiles unchanged. **A per-candidate
/// loop must move to [`walk_over`]**, which takes both facts and allocates
/// nothing; [`SliceFacts`] carries the measurement and the caller list.
#[must_use]
pub fn walk_with(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    direction: Direction,
    exits: Option<&[Option<SquareOff>]>,
) -> Trades {
    // BORROWED WHERE THE CALLER HOISTED, BUILT WHERE IT DID NOT. `owned` holds
    // the fallback alive for the borrow below and is untouched otherwise.
    let owned;
    let exits: &[Option<SquareOff>] = if let Some(table) = exits {
        table
    } else {
        owned = forced_exits(bars);
        &owned
    };
    walk_core(
        bars,
        column,
        mask,
        horizon,
        direction,
        exits,
        &SliceFacts::of(bars, column),
    )
}

/// The walk itself, over facts that have already been derived.
///
/// Private, and takes both per-slice quantities as plain values rather than as
/// options: a body that can still decide to build something is a body a caller
/// cannot read a cost off. [`walk_over`] and [`walk_with`] differ only in where
/// those two values came from.
#[allow(
    clippy::too_many_lines,
    reason = "one ordered signal-state machine; splitting its refusal branches \
              would obscure the one-position-at-a-time transition"
)]
fn walk_core(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    direction: Direction,
    exits: &[Option<SquareOff>],
    facts: &SliceFacts,
) -> Trades {
    let h = horizon.as_bars() as usize;
    let step_micros = facts.step_micros();
    let mut out = Trades::default();
    // The bar the open position exits on. Until the first entry there is none,
    // and a new fill is eligible only strictly after it.
    let mut open_until: Option<usize> = None;

    for (bits, &signal) in column.bits().iter().zip(column.sources()) {
        if !bits.hits(mask) {
            continue;
        }
        out.signals = out.signals.saturating_add(1);

        // RULE 4, DECIDED HERE AND APPLIED BELOW.
        //
        // A position is open, so this signal buys nothing. Not a long, not a
        // short, not a scale-in.
        //
        // IT USED TO `continue` HERE, AND THAT MADE THE EXIT GRID MEASURE ONE
        // TRADE SET FOR ALL 625 CELLS.
        //
        // `crate::grid::evaluate` builds its candidates from `out.trades`, which
        // is this walk's answer with LEVEL-LESS exits -- the longest possible
        // holds, so the most exclusion. Its own per-variant exclusivity then had
        // nothing left to do: a tighter stop frees the next signal only if that
        // signal is in the list, and a `continue` here is exactly what kept it
        // out. An adversarial fleet measured the guard firing **zero times
        // across 168,892 cells**, and the module header of `crate::grid` claims
        // in writing that the sequence differs per variant.
        //
        // So eligibility is now decided for EVERY signal and recorded in
        // [`Trades::eligible`], and rule 4 selects `trades` out of it. The
        // counters are unmoved: `blocked` is still tested before lateness, so a
        // signal that is both still counts as `while_open` exactly as before.
        // RULE 2. The fill is on the NEXT bar; the signal bar's close has
        // already printed and cannot be traded at.
        // Every refusal below is a LATENESS, and a blocked signal is charged to
        // `while_open` rather than to it -- so each branch asks `blocked` first.
        // The macro-free way of writing that is one `late` closure, but the
        // counter is on `out` and a closure would borrow it, so it is spelled
        // out at each site.
        //
        // AND `step` IS 1 ONLY WHEN THE SOURCE IS A SIGNAL BAR.
        //
        // This added one unconditionally. On a REPROJECTED column that is wrong:
        // `align::onto_execution` already returns the execution bar stamped
        // exactly at the signal's close -- the immediate one-minute fill bar --
        // and `Column::reproject` stores THAT in `sources`. Adding one to it
        // skips a bar.
        //
        // MEASURED on 60 synthetic sessions: 100.00% of projected rows on all
        // SEVEN aligned rungs entered exactly one execution bar late, and with
        // 4.8% of bars missing the lateness ran out to 1,071 minutes -- an
        // overnight crossing. The 1-minute rung was correct only because it
        // skips alignment entirely, so the eight rungs `range-all` compares were
        // never measured under one execution rule.
        //
        // `Column::sourced` is what makes the two conventions distinguishable;
        // before it, one field carried both meanings and this line could not
        // tell which one it had.
        let step = match column.sourced() {
            indicators::column::Sourced::Signal => 1,
            indicators::column::Sourced::Fill => 0,
        };
        let Some(entry) = signal.checked_add(step) else {
            let blocked = open_until.is_some_and(|until| signal <= until);
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        };
        // Compare FILLS, not signals. A signal on the previous exit bar is
        // valid when it fills on the following minute; a fill on the exit bar
        // itself is not. This also handles reprojected columns, whose source is
        // already the execution fill index and therefore adds no step.
        let blocked = open_until.is_some_and(|until| entry <= until);
        // THE ENTRY RECORD MUST HAVE PASSED THE SAME STATEFUL EVALUATOR PASS
        // THAT BUILT THIS EXECUTION SLICE. `Candle::check` cannot see a
        // duplicate timestamp or a VWAP accumulator overflow.
        if !facts.accepts(entry) {
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        }
        // A native column names a signal bar and uses that native series'
        // cadence. Shipping one-minute execution paths are checked-reprojected
        // to `Sourced::Fill` first, so a sparse minute file cannot redefine the
        // cadence here; legacy same-series callers retain their explicit rung
        // cadence, while operator-facing coarse paths are refused before this
        // point unless they supplied one-minute execution. A
        // fill-sourced column already came through `onto_execution`, whose
        // exact timestamp equality selected the immediate one-minute bar.
        let immediate = match column.sourced() {
            indicators::column::Sourced::Signal => bars
                .get(signal)
                .zip(bars.get(entry))
                .is_some_and(|(signal_bar, entry_bar)| {
                    step_micros > 0
                        && signal_bar.ts_micros.checked_add(step_micros)
                            == Some(entry_bar.ts_micros)
                }),
            indicators::column::Sourced::Fill => true,
        };
        if !immediate {
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        }
        // RULE 1. The entry bar must itself be inside the tradeable window of
        // its own session, and the exit must exist. `forced` names the last bar
        // of that window a fill can land in; whether the MARKET gave that bar,
        // or the slice merely stopped there, is RULE 1c's question and not this
        // one's.
        let Some(forced) = exits.get(entry).copied().flatten() else {
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        };

        // RULE 1b. AND THAT SESSION MUST BE THE SIGNAL'S OWN.
        //
        // `exits` answers "the last bar of THIS bar's session a fill can land
        // in", which is exactly right for the entry bar and says nothing about
        // where the signal was. A signal on the final bar of a session takes
        // `entry = signal + 1`, which is the FIRST bar of the next session, and
        // that bar has a perfectly good forced exit of its own — so the guard
        // above passed it and the position opened across the overnight gap.
        //
        // MEASURED before this check existed, `synthetic::sessions(12)`, 41
        // single-condition masks: 132 trades entered on a later session's bar.
        // The first is a signal on bar 2,249 of day 5 filling on bar 2,250 of
        // day 6. Every one of them priced an entry against a gap that no
        // intraday rule permits and that this engine squares off before close
        // precisely to avoid.
        //
        // This is the same defect class as the forward-return gap fixed
        // earlier: the square-off bounds the EXIT, and nothing bounded the
        // entry to the same day.
        let same_session = bars.get(signal).zip(bars.get(entry)).is_some_and(|(s, e)| {
            indicators::ist_day(s.ts_micros) == indicators::ist_day(e.ts_micros)
        });
        if !same_session {
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        }
        // THE HORIZON IN TIME, NOT IN BAR INDICES. `entry + h` is a duration
        // only while the bars are contiguous, and they are not -- see
        // `horizon_bar` for the 104-minute hold billed as fifteen minutes, and
        // for why a position carried across a market closure has every exit
        // order inert. Identical to `entry + h` on a contiguous slice.
        // RULE 1c. THE HOLD ENDS AT THE HORIZON OR AT ITS SESSION'S SQUARE-OFF,
        // WHICHEVER COMES FIRST — AND "THE SLICE RAN OUT" IS NEITHER.
        //
        // This was `wanted.min(forced)`, which is right for the first two and
        // fabricates the third. `forced_exits` answers "the last bar of this
        // bar's session a fill can land in", and on the final session of a
        // slice that was cut mid-day that bar is THE CUT. A slice ending at
        // 10:54 therefore exited at 10:54 and stamped `forced: true` on it: a
        // square-off on a bar where no square-off happened, at a price
        // the position was never closed at.
        //
        // That is worse than a missing trade. A missing trade is a smaller `n`;
        // this one arrives in the table with a best/worst pair, a return and a
        // `forced` flag, indistinguishable from a real round trip — a
        // manufactured fill in a backtester, which `CLAUDE.md` §4 calls a
        // fallback that hides a failure.
        //
        // Three cases, and the third is a refusal rather than a guess:
        //
        //   * the horizon fits inside the tradeable window — an ordinary exit;
        //   * it does not, and `forced.bar` genuinely ENDS that window — the
        //     square-off, which is a real measured outcome;
        //   * it does not, and `forced.bar` is only where the data stopped —
        //     no exit price exists, so the signal is counted `too_late` and
        //     nothing is traded.
        //
        // The same three-way branch, and the same reason, as
        // [`crate::outcome::forward`]. That module refused the truncated tail
        // and this one invented it, so the two disagreed about the same slice:
        // `edge` reported no observation where `walk` reported a trade.
        //
        // WHAT THIS DOES NOT FIX: a slice cut at 15:05 still loses the five
        // minutes to the square-off, and every signal in them is now
        // `too_late`. The trades that DO survive are unchanged — the tail is
        // dropped, not re-priced — so this narrows the sample and never moves a
        // number that was already measured.
        let span = i64::try_from(h).unwrap_or(i64::MAX);
        let deadline = bars.get(entry).map_or(i64::MAX, |bar| {
            bar.ts_micros
                .saturating_add(step_micros.saturating_mul(span))
        });
        let forced_stamp = bars.get(forced.bar).map_or(i64::MIN, |bar| bar.ts_micros);
        let (exit, forced_exit) = if deadline > forced_stamp && forced.real {
            (forced.bar, true)
        } else if deadline <= forced_stamp {
            if let Some(wanted) = horizon_bar(bars, entry, h, step_micros, facts) {
                (wanted, false)
            } else {
                out.occupancy.push(Occupancy {
                    signal_bar: signal,
                    entry_bar: entry,
                    exit_bar: forced.bar,
                    priceable: false,
                });
                if !blocked {
                    open_until = Some(forced.bar);
                }
                if blocked {
                    out.while_open = out.while_open.saturating_add(1);
                } else {
                    out.too_late = out.too_late.saturating_add(1);
                }
                continue;
            }
        } else {
            out.occupancy.push(Occupancy {
                signal_bar: signal,
                entry_bar: entry,
                exit_bar: forced.bar,
                priceable: false,
            });
            if !blocked {
                open_until = Some(forced.bar);
            }
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        };
        if exit <= entry {
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        }
        // ONE REFUSED INTERIOR BAR MAKES EVERY LEVEL ORDER AND EVERY PATH
        // EXTREMUM UNKNOWABLE. Keep the occupied interval, but publish no
        // trade and let the grid retain it as block-only.
        if !facts.path_accepts(entry, exit) {
            out.occupancy.push(Occupancy {
                signal_bar: signal,
                entry_bar: entry,
                exit_bar: exit,
                priceable: false,
            });
            if !blocked {
                open_until = Some(exit);
            }
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        }
        let Some(trade) = round_trip(bars, signal, entry, exit, forced_exit, direction, facts)
        else {
            // ENTRY AND EXIT ARE DIFFERENT FACTS. If the entry bar was
            // priceable, the position opened even though the exit record could
            // not be priced. Forgetting that interval promoted the next signal
            // into an overlapping trade in both this walk and the exit grid.
            //
            // A refused ENTRY does not block: no position can be claimed to
            // have opened. A refused EXIT does: the conservative extent is the
            // time exit, exactly as `grid::blocks_without_pricing` treats a
            // corrupt bar inside a path.
            if entry_is_priceable(bars, entry, direction, facts) {
                out.occupancy.push(Occupancy {
                    signal_bar: signal,
                    entry_bar: entry,
                    exit_bar: exit,
                    priceable: false,
                });
                if !blocked {
                    open_until = Some(exit);
                }
            }
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        };
        // THE UNIVERSE FIRST, THE SELECTION SECOND.
        //
        // This signal is tradeable: it cleared every rule except rule 4. It goes
        // into `eligible` whether or not a position is open, because the exit
        // grid needs the signals a DIFFERENT exit would have freed -- see that
        // field for what its absence cost.
        out.eligible.push(trade);
        out.occupancy.push(Occupancy {
            signal_bar: signal,
            entry_bar: entry,
            exit_bar: exit,
            priceable: true,
        });
        if blocked {
            out.while_open = out.while_open.saturating_add(1);
            continue;
        }
        open_until = Some(exit);
        out.trades.push(trade);
    }
    out
}

/// Price one round trip both ways.
///
/// `None` when a bar is missing or a fill leaves `i64` — refused rather than
/// saturated, because a saturated fill is a price nobody traded at, which is
/// `costs::fill`'s own rule and not a new one.
fn round_trip(
    bars: &[Candle],
    signal_bar: usize,
    entry_bar: usize,
    exit_bar: usize,
    forced: bool,
    direction: Direction,
    facts: &SliceFacts,
) -> Option<Trade> {
    // A BAR THIS RUN ALREADY REFUSED MAY NOT FILL A LEG.
    //
    // `bars.get(..)` alone was the whole guard, so a record
    // `indicators::column::Column::build` had charged to
    // `Census::price_outside_range` still supplied an open, a high and a low to
    // `costs::fill`. `crate::outcome::priced` carries the measurement: one such
    // record among 7,500 flipped the sign of `walk`'s best total, from
    // **−14,700 to +79,930**, with `Trades::reconciles()` still true.
    //
    // `facts.accepts` reads the exact `Evaluator::step` verdict the column
    // retained, including the two stateful refusals no local candle predicate
    // can reconstruct. A refused leg returns `None`, which `walk` already
    // accounts as a signal that took no trade rather than as a trade worth zero.
    //
    // `FillBar::new` below refuses an inverted or sub-tick bar and would have
    // caught SOME of these -- but only some. The bar that produced the sign flip
    // has `high >= low` and both legs above a tick; what makes it corrupt is
    // that its close sits outside `[low, high]`, which is a question
    // `costs::fill` never asks because a fill does not use the close.
    let e = bars.get(entry_bar)?;
    let x = bars.get(exit_bar)?;
    if !facts.accepts(entry_bar) || !facts.accepts(exit_bar) {
        return None;
    }

    // BEST: both legs at the bar's open, through `Anchor::Open`.
    //
    // # This was `worst_case_fills` on a FLAT bar, and the tick is the change
    //
    // `costs::fill::Bar` carried only the extremes, so the only way to anchor on
    // an open was to build a bar where the open WAS both extremes and run the
    // adverse rule over it. That works -- on a flat bar the adverse anchor is
    // the open -- but the adverse TICK still applied, so the "best" case bought
    // one tick above the open and sold one tick below it. It was the open plus a
    // penalty, which is not the best case; it is a slightly-worse-than-best one
    // wearing that name.
    //
    // `Bar` carries the open again and `Anchor::Open` reads it, so this is now
    // the open exactly, with zero slippage and none of the flat-bar staging.
    // The two readings therefore bracket a real fill properly: the best is the
    // most favourable price that PRINTED, the worst is the least favourable
    // price that PRINTED, and every achievable fill lies between them.

    // WORST: the adverse extreme of each bar. `Bar::new` takes (open, high, low)
    // and refuses an inverted bar, a sub-tick high, or an open its own extremes
    // do not bracket -- each a corrupt candle rather than a tradeable one.
    //
    // THE OPEN IS PASSED NOW AND IT WAS NOT BEFORE. `costs::fill::Bar` carried
    // only the extremes while one fill model existed; it carries the open again
    // because `Anchor::Open` reads it. Passing the candle's real open rather
    // than a stand-in matters: the bracket check is now a genuine invariant, so
    // a candle whose open sits outside its own high-low is refused HERE, at the
    // fill, instead of silently pricing off extremes that never contained it.
    // NO TICK. THE FOUR NUMBERS OF THE BAR ARE THE WHOLE OF WHAT IS KNOWN.
    //
    // This was `worst_case_fills`, which is `Anchor::AdverseExtreme`: a buy at
    // the high PLUS one tick and a sell at the low MINUS one tick. That tick is
    // a claim about microstructure, not a reading of the data — it names a price
    // at which nothing traded in that minute. `costs::fill::Anchor`'s own doc
    // makes the objection: *"a fill invented one tick outside them is exactly
    // the invention `CLAUDE.md` §3 rule 1 forbids"*.
    //
    // On a series whose finest resolution IS one minute, the honest worst case
    // is the worst price the bar actually PRINTED. That is
    // `Anchor::PrintedExtreme`, and it is what `crate::grid` has always used to
    // price the exit ladder.
    //
    // # This also ENDS a disagreement between the two halves of the pipeline
    //
    // The grid priced at `PrintedExtreme` and this walk priced at
    // `AdverseExtreme`, so the same trades on the same bars cost two ticks per
    // round trip more here than in the grid — and the GRID's number is the one
    // selection ranks on. The two now read the same bars the same way, so a
    // combination's totals mean the same thing wherever they are printed.
    //
    // What is LOST is the slippage allowance, and it is not smuggled back in as
    // a smaller number: it is gone, and every figure downstream is now
    // explicitly a fill at a price that printed, with no execution penalty
    // modelled at all. `docs/06-limits.md` is where that belongs as a stated
    // limit rather than as an invisible optimism.
    let entry_fill_bar = FillBar::new(paisa(e.open), paisa(e.high), paisa(e.low)).ok()?;
    let exit_fill_bar = FillBar::new(paisa(x.open), paisa(x.high), paisa(x.low)).ok()?;
    let best_fills = fills_at(entry_fill_bar, exit_fill_bar, direction, Anchor::Open).ok()?;
    let worst_fills = fills_at(
        entry_fill_bar,
        exit_fill_bar,
        direction,
        Anchor::PrintedExtreme,
    )
    .ok()?;

    Some(Trade {
        signal_bar,
        entry_bar,
        exit_bar,
        best: pnl(&best_fills),
        worst: pnl(&worst_fills),
        forced,
    })
}

/// Whether the entry leg itself can be priced under both reported readings.
///
/// This deliberately asks only about the entry. The caller reaches it after a
/// whole round trip refused and needs to distinguish "the position never
/// opened" from "the position opened and its exit cannot be measured".
fn entry_is_priceable(
    bars: &[Candle],
    entry: usize,
    direction: Direction,
    facts: &SliceFacts,
) -> bool {
    let Some(bar) = bars.get(entry) else {
        return false;
    };
    if !facts.accepts(entry) {
        return false;
    }
    let Ok(fill_bar) = FillBar::new(paisa(bar.open), paisa(bar.high), paisa(bar.low)) else {
        return false;
    };
    fills_at(fill_bar, fill_bar, direction, Anchor::Open).is_ok()
        && fills_at(fill_bar, fill_bar, direction, Anchor::PrintedExtreme).is_ok()
}

/// Sell minus buy, per unit, in paisa.
///
/// The same expression for both directions, and that is not a shortcut:
/// `costs::fill::Fills` already assigned the legs — for a long the buy is the
/// entry and the sell is the exit, for a short they are the other way round —
/// so the profit is `sell - buy` in both cases and reversing it for shorts would
/// double-count the direction.
fn pnl(fills: &costs::fill::Fills) -> i64 {
    fills.sell().raw().saturating_sub(fills.buy().raw())
}

/// A paisa integer, as `costs` spells it.
fn paisa(raw: i64) -> brutex_core::price::Paisa {
    brutex_core::price::Paisa::from_raw(raw)
}

/// Where one bar's position is squared off, and whether the market gave that
/// bar or the file merely ended on it.
///
/// The two are the same INDEX and a different fact, which is exactly why they
/// were confused: a bar at 15:09 and the last bar of a truncated slice are both
/// "the last bar of this session a fill can land in", and only one of them has
/// a closing price the position was actually exited at.
// `Clone` and `Copy` are load-bearing -- `vec![None; n]` needs the one and
// `Option::copied` the other. `Debug`, `PartialEq` and `Eq` were derived here
// too and are not: nothing compares two of these and nothing prints one. A
// derived impl that no call site reaches is an instrumented region no test can
// cover, and `CLAUDE.md` §9 asks for 100% on every touched crate, so they are
// gone rather than carried. The tests below read `bar` and `real` as fields for
// the same reason. Add either back WITH the code that uses it.
#[derive(Clone, Copy)]
pub struct SquareOff {
    /// The last bar of this bar's own session a fill can still land in.
    pub(crate) bar: usize,
    /// True when [`SessionBounds::day_ended`] holds at [`Self::bar`]: the
    /// day contains exactly one accepted 15:09 one-minute record, so the
    /// square-off price is a fact in the modeled path.
    ///
    /// False when that record is missing, refused, duplicated, or the path is
    /// coarse-only. There is then no square-off price at all, and a hold that
    /// overruns its horizon has no exit rather than an exit here — [`walk`]'s
    /// RULE 1c.
    ///
    /// A later day proves nothing about the missing required minute.
    pub(crate) real: bool,
}

/// The bar a hold of `h` bars may not run past, given where it started.
///
/// # A hold of fifteen bars became a hold of a hundred and four minutes
///
/// `wanted = entry + h` is bar-index arithmetic, and a bar index is only a
/// duration while the bars are contiguous. They are not: the store is missing
/// 0.50% of its one-minute bars, and four sessions in eighty-one months are
/// short or split.
///
/// **2024-03-02**, an NSE disaster-recovery Saturday, holds 105 one-minute bars:
/// 09:15–09:59, then nothing until 11:30, then 11:30–12:29. A long entered at
/// 09:57 with `Horizon::DEFAULT` exits fifteen BARS later, at 11:41 — a
/// **104-minute hold billed as fifteen minutes**, held across a 91-minute
/// closure. 2024-05-18 is the same shape.
///
/// The duration is the smaller half of it. No bar exists inside the gap, so
/// `excursion::crossings` examines none, so **no stop, target or trail can fire
/// there** — the position is carried through a market closure with every exit
/// order inert. That biases every one of those trades toward the favourable
/// side, which is the direction §4 cares about.
///
/// # The deadline is derived, not typed
///
/// `h` bars of a series whose bars are `spacing` apart is `h * spacing` of
/// wall clock, and `spacing` is measured from the slice itself — the same
/// quantity `cli::signal_spacing_minutes` reads, taken here from the median
/// step so a handful of missing bars cannot move it. On a contiguous slice the
/// answer is bit-identical to `entry + h`; it differs only where the bars
/// themselves do, which is the defect.
///
/// Returns only a bar stamped at the EXACT deadline.
///
/// Returning the last bar before a gap made the exit retroactive: 09:15,
/// 09:16, 09:18 at a two-minute horizon exited at 09:16, before the 09:17
/// deadline had even arrived. Absence is now absence, and the caller retains a
/// conservative block-only occupancy instead of manufacturing a close.
fn horizon_bar(
    bars: &[Candle],
    entry: usize,
    h: usize,
    step_micros: i64,
    facts: &SliceFacts,
) -> Option<usize> {
    if step_micros <= 0 {
        return None;
    }
    let start = bars.get(entry)?.ts_micros;
    let span = i64::try_from(h).unwrap_or(i64::MAX);
    let deadline = start.saturating_add(step_micros.saturating_mul(span));
    facts.at_timestamp(deadline)
}

/// For each bar, where a position opened on it is squared off — and whether
/// that square-off is a fact about the session or about the file.
///
/// [`SessionBounds`] derives every day's boundary in one backward traversal,
/// then this projects its O(1) answers into the table the hot walk consumes.
/// `None` where the bar is at or past the square-off: nothing entered there can
/// be exited inside its own session at all.
///
/// Where the session's bars simply run out first the answer is
/// `Some(SquareOff { real: false, .. })` and deliberately NOT `None`. That bar
/// is still a legitimate exit for a hold whose horizon lands on or before it —
/// that price printed and that trade completed — and it is only the OVERRUN
/// that has nowhere to go. Refusing the whole truncated session here would
/// discard round trips that finished inside the slice, which is a second wrong
/// answer rather than a fix for the first. [`walk`] makes the distinction, and
/// [`crate::outcome`] carries the same note on why the end of the data is not a
/// square-off.
///
/// UNVERIFIED as a measured figure, and no bench row covers it. What is claimed
/// is the SHAPE: each projected entry is one bounds-checked read of
/// [`SessionBounds`] plus one read of its proven-end flag. The boundary table
/// already carries the last fill for every bar, so nothing re-walks a session.
/// A forward search per bar would be O(H) with `H` caller-supplied — constant
/// only by accident, which is the kind of bound `CLAUDE.md` §3 rule 6 asks to
/// be labelled rather than asserted.
///
/// # The boundary is fixed; the price must be proved
///
/// [`SessionBounds`] carries the one product clock shared with outcome
/// measurement. A unique accepted one-minute row stamped 15:09 is proof; a
/// truncated endpoint, nearby row, duplicate, coarse bucket, or later day is
/// not. Both modules read this table rather than keeping parallel spellings.
#[must_use]
pub fn forced_exits(bars: &[Candle]) -> Vec<Option<SquareOff>> {
    forced_exits_with_step(bars, median_step_micros(bars), None)
}

/// [`forced_exits`] with the slice's median timeframe already measured.
///
/// `SliceFacts` calls this form so its horizon clock and its square-off table
/// cannot allocate or infer the same slice twice.
fn forced_exits_with_step(
    bars: &[Candle],
    step_micros: i64,
    accepted: Option<&[bool]>,
) -> Vec<Option<SquareOff>> {
    let session = SessionBounds::with_step(bars, step_micros, accepted);
    (0..bars.len())
        .map(|index| {
            session.last_fill_bar(index).map(|bar| SquareOff {
                bar,
                // The observed boundary is usable as a forced fill only when
                // this day proved one unique accepted 15:09 execution row.
                real: session.day_ended(bar),
            })
        })
        .collect()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Trades, walk};
    use crate::outcome::{FORCED_EXIT_MINUTE, Horizon, SessionBounds};

    /// THE POLICY IS FIXED 15:10 AND ONLY THE 15:09 INTERVAL CAN PRICE IT.
    /// Regular and extended fixtures contain that record; a short Muhurat
    /// fixture does not. This asserts the explicit policy consequence without
    /// pretending it is an exchange closing-time fact.
    #[test]
    fn the_forced_exit_is_fixed_at_1510_and_requires_the_exact_1509_bar() {
        assert_eq!(FORCED_EXIT_MINUTE, 15 * 60 + 10);

        for (last_open, what) in [(15 * 60 + 29, "regular"), (16 * 60 + 59, "extended")] {
            let bars: Vec<indicators::Candle> = (9 * 60 + 15..=last_open)
                .map(|m| flat(ist_minute_stamp(0, m)))
                .collect();
            let bounds = SessionBounds::of(&bars);
            let last_fill = usize::try_from(15 * 60 + 9 - (9 * 60 + 15))
                .expect("15:09 is inside both sessions");
            assert_eq!(bounds.last_fill_minute(0), Some(15 * 60 + 9), "{what}");
            assert!(bounds.fillable(last_fill), "{what}: 15:09 closes at 15:10");
            assert!(
                !bounds.fillable(last_fill + 1),
                "{what}: 15:10 is post-deadline"
            );
            assert!(bounds.day_ended(0), "{what}: the exact bar proves the fill");
        }

        let muhurat: Vec<indicators::Candle> = (13 * 60 + 45..=14 * 60 + 44)
            .map(|m| flat(ist_minute_stamp(0, m)))
            .collect();
        let bounds = SessionBounds::of(&muhurat);
        assert!(
            !bounds.day_ended(0),
            "a non-regular session without 15:09 has no fabricated forced fill"
        );
    }

    /// The UTC microsecond stamp of IST minute `minute` on IST day `day`.
    ///
    /// `day` counts whole days from the epoch, which is what
    /// [`indicators::ist_day`] returns, so `day + 1` is genuinely the next
    /// session and not merely a later timestamp.
    fn ist_minute_stamp(day: i64, minute: i64) -> i64 {
        (day * 1_440 + minute) * 60_000_000 - indicators::IST_OFFSET_MICROS
    }

    /// Minute of the IST day, computed HERE rather than borrowed from the
    /// module under test.
    ///
    /// `super::minute_of_day` is gone: [`SessionBounds`] owns the stamping now,
    /// and a test that asks the code whether the code is right compares a
    /// function to itself. This is the same arithmetic written independently,
    /// which is what the removed helper's callers were relying on it to be.
    fn ist_minute_of(ts_micros: i64) -> i64 {
        (ts_micros + indicators::IST_OFFSET_MICROS).div_euclid(60_000_000) % 1_440
    }

    use costs::fill::Direction;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use vocab::ConditionMask;

    fn evaluator() -> Evaluator {
        evaluator_with(Availability::Absent)
    }

    fn evaluator_with(availability: Availability) -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            availability,
            Thresholds::CLASSICAL,
        )
    }

    fn h(n: u32) -> Horizon {
        Horizon::bars(n).expect("a non-zero horizon")
    }

    fn swept() -> (Vec<indicators::Candle>, Column) {
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        (bars, column)
    }

    /// A REPROJECTED COLUMN ENTERS ON THE FILL BAR, NOT ONE AFTER IT.
    ///
    /// # The operator's rule, and where it was broken
    ///
    /// Whatever timeframe generates the signal, entry fills on the next ONE-MINUTE
    /// bar. `align::onto_execution` returns exactly that bar — the execution bar
    /// stamped at the signal's close — and `Column::reproject` stores it in
    /// `sources`. `walk` then added `+1` to it, because one field carried
    /// two conventions and nothing recorded which.
    ///
    /// MEASURED before [`indicators::column::Sourced`] existed: 100.00% of
    /// projected rows on all seven aligned rungs entered one execution bar late.
    /// Only the 1-minute rung was right, and only because it skips alignment —
    /// so the eight rungs `range-all` compares were never measured under one
    /// execution rule.
    ///
    /// This asserts the two conventions produce entries exactly one bar apart on
    /// the SAME sources, which is the whole of the defect and cannot be satisfied
    /// by an off-by-one in either direction.
    #[test]
    fn a_reprojected_column_enters_on_the_bar_it_was_told_to() {
        let (bars, signal_column) = swept();
        // Reproject onto ITSELF: `onto[j] = source[j]`, so the fill bar and the
        // signal bar are the same index and the ONLY difference between the two
        // walks is the convention. A fixture that also moved the indices could
        // not tell a convention error from an alignment one.
        let onto: Vec<Option<usize>> = signal_column.sources().iter().map(|&s| Some(s)).collect();
        let (fill_column, dropped) = signal_column
            .reproject_checked(&onto, &bars)
            .expect("the identity projection is the same length");
        assert_eq!(
            dropped, 0,
            "nothing is unreachable in an identity projection"
        );
        assert_eq!(
            fill_column.sourced(),
            indicators::column::Sourced::Fill,
            "reprojection records that its sources are fill bars"
        );
        assert_eq!(
            signal_column.sourced(),
            indicators::column::Sourced::Signal,
            "and a built column records that its sources are signal bars"
        );
        assert_eq!(
            fill_column.sources(),
            signal_column.sources(),
            "the identity projection moved no index, so any difference below is \
             the convention and nothing else"
        );

        let mask = ConditionMask::default();
        let from_signal = walk(&bars, &signal_column, &mask, h(15), Direction::Long);
        let from_fill = walk(&bars, &fill_column, &mask, h(15), Direction::Long);

        // THE FIRST TRADE, AND ONLY THE FIRST.
        //
        // The two sequences DIVERGE after it, and that is rule 4 working rather
        // than a defect: entering a bar earlier exits a bar earlier, which frees
        // the next eligible signal sooner, so the second trade is a different
        // signal in the two walks. Measured on this fixture the second pair is
        // already two bars apart. Comparing them all would assert that
        // exclusivity does NOT depend on the entry bar, which is the opposite of
        // what `grid` relies on.
        let fill = from_fill.trades.first().expect("the fixture must trade");
        let signal = from_signal
            .trades
            .first()
            .expect("and so must the signal-sourced walk");
        assert_eq!(
            fill.signal_bar, signal.signal_bar,
            "the same signal, or the two walks are not comparable at all"
        );
        assert_eq!(
            fill.entry_bar.saturating_add(1),
            signal.entry_bar,
            "a fill-sourced column must enter ONE BAR EARLIER than a \
             signal-sourced one on the same index — that bar is the operator's \
             next-1-minute fill, and adding one skips it"
        );
        assert_eq!(
            fill.entry_bar, fill.signal_bar,
            "and on a fill-sourced column the entry IS the source: nothing is \
             added to a bar that is already the fill"
        );

        assert!(
            from_fill.trades.len() > 1,
            "the fixture needs adjacent projected trades to test re-entry"
        );
        for pair in from_fill.trades.windows(2) {
            let [previous, next] = pair else {
                unreachable!("windows(2) always yields exactly two trades")
            };
            assert!(
                next.entry_bar > previous.exit_bar,
                "a projected fill at {} must be strictly after the previous \
                 exit bar {}; one minute cannot close and reopen the position",
                next.entry_bar,
                previous.exit_bar
            );
        }
    }

    /// “Next row” is not “next minute” when the native one-minute file has a
    /// hole. The later row is observable evidence, not a substitute fill.
    #[test]
    fn a_native_one_minute_gap_never_delays_entry_to_the_next_stored_row() {
        let (mut bars, _) = swept();
        let signal_stamp = bars.first().expect("the fixture has a first bar").ts_micros;
        let omitted = bars.remove(1);
        assert_eq!(
            omitted.ts_micros,
            signal_stamp + 60_000_000,
            "the removed record is exactly the immediate next minute"
        );
        assert_eq!(
            bars.get(1).expect("a later stored row remains").ts_micros,
            signal_stamp + 120_000_000,
            "the row now adjacent in storage is two minutes late"
        );

        let column = Column::build(&bars, &mut evaluator());
        assert_eq!(column.sourced(), indicators::column::Sourced::Signal);
        let walked = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Direction::Long,
        );

        assert!(walked.signals > 0, "the empty mask must exercise the gap");
        assert!(
            walked.reconciles(),
            "the refused signal remains accounted for"
        );
        assert!(
            walked.too_late > 0,
            "the 09:15 signal has no immediate fill and must be refused"
        );
        assert!(
            walked.trades.iter().all(|trade| trade.signal_bar != 0),
            "the signal before the hole must not become a delayed trade"
        );
        assert!(
            walked.eligible.iter().all(|trade| trade.signal_bar != 0),
            "the delayed row must not reach any exit-grid candidate"
        );
        assert!(
            walked.occupancy.iter().all(|held| held.signal_bar != 0),
            "a trade that never entered must not occupy the position"
        );
    }

    /// The cadence cannot be learned from damaged evidence. When most stored
    /// neighbours are two minutes apart, the median is two minutes; the engine
    /// still requires the missing one-minute bar and refuses every delayed row.
    #[test]
    fn a_sparse_native_slice_cannot_redefine_one_minute_as_its_median_gap() {
        let (dense, _) = swept();
        let bars: Vec<_> = dense
            .into_iter()
            .enumerate()
            .filter_map(|(index, bar)| (index <= 1 || !index.is_multiple_of(2)).then_some(bar))
            .collect();
        let column = Column::build(&bars, &mut evaluator());
        assert_eq!(
            super::median_step_micros(&bars),
            120_000_000,
            "the adversarial slice must actually have a two-minute inferred cadence"
        );
        let alignment = crate::align::onto_execution(
            &bars,
            column.sources(),
            &bars,
            super::EXECUTION_MINUTE_MICROS,
        )
        .expect("the native minute series aligns onto itself");
        assert!(
            alignment.unreachable > 0,
            "signals without their exact next minute must be counted as dropped"
        );
        for (&source, target) in column.sources().iter().zip(alignment.onto.iter()) {
            if let Some(fill) = *target {
                assert_eq!(
                    bars.get(fill).map(|bar| bar.ts_micros),
                    bars.get(source)
                        .and_then(|bar| bar.ts_micros.checked_add(60_000_000)),
                    "every admitted mapping is the exact next minute"
                );
            }
        }
        let (projected, _) = column
            .reproject_checked(&alignment.onto, &bars)
            .expect("checked self-projection");
        assert_eq!(projected.sourced(), indicators::column::Sourced::Fill);
        let facts = super::SliceFacts::of(&bars, &projected);
        assert_eq!(
            facts.step_micros(),
            super::EXECUTION_MINUTE_MICROS,
            "the execution contract stays one minute despite the two-minute median"
        );
        let bounds =
            super::SessionBounds::with_step(&bars, facts.step_micros(), Some(facts.acceptance()));
        assert!(
            !bounds.day_ended(0),
            "this every-other-minute fixture omits 15:09 and cannot price the deadline"
        );
        assert!(
            !facts.path_accepts(0, bars.len().saturating_sub(1)),
            "missing interior minutes make the held path unpriceable"
        );

        let walked = super::walk_over(
            &bars,
            &projected,
            &ConditionMask::default(),
            h(15),
            Direction::Long,
            &facts,
        );
        assert!(
            walked.reconciles(),
            "every refused signal remains accounted for"
        );
        assert_eq!(walked.trades.len(), 0, "no delayed fill can become a trade");
        assert!(
            walked.eligible.is_empty(),
            "no sparse path can reach an exit-grid candidate"
        );
    }

    #[test]
    fn no_two_trades_overlap_and_every_signal_is_accounted_for() {
        // RULE 4, asserted directly: while a position is open nothing else is
        // entered. Checked over every pair rather than sampled, because
        // "usually one at a time" is not the rule.
        let (bars, column) = swept();
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Direction::Long,
        );

        assert!(t.count() > 0, "the fixture must produce trades");
        assert!(
            t.reconciles(),
            "trades + while_open + too_late must equal signals: {t:?}"
        );

        let mut previous_exit = 0_usize;
        for (n, trade) in t.trades.iter().enumerate() {
            assert!(
                trade.entry_bar > previous_exit || n == 0,
                "trade {n} entered at {} on or before the previous exit bar {previous_exit}",
                trade.entry_bar
            );
            assert_eq!(
                trade.entry_bar,
                trade.signal_bar.saturating_add(1),
                "the fill is on the bar AFTER the signal, never on the signal"
            );
            assert!(trade.exit_bar > trade.entry_bar, "a trade must be held");
            previous_exit = trade.exit_bar;
        }
    }

    #[test]
    fn exclusivity_collapses_an_always_true_mask_to_far_fewer_trades_than_signals() {
        // The empty mask fires on EVERY swept bar, so this is the extreme case
        // of the defect: `outcome::edge` would count every one of those as an
        // independent observation. With one position at a time and a 15-bar
        // hold, the count must fall by roughly the hold length.
        let (bars, column) = swept();
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Direction::Long,
        );

        assert!(
            t.count() * 5 < t.signals,
            "an always-true mask at H=15 must yield far fewer trades ({}) than \
             signals ({}) -- if these are close, exclusivity is not being applied",
            t.count(),
            t.signals
        );
        assert!(
            t.while_open > 0,
            "signals must have been blocked by an open position"
        );
    }

    #[test]
    fn the_best_case_is_the_open_and_the_worst_is_the_printed_extreme() {
        // WHY THIS EXISTS. `Trade::best` was pinned by ONE assertion --
        // `worst <= best` -- and by nothing else in the workspace. So when the
        // best case changed from "the open, one tick adverse on each leg" to
        // "the open exactly", every test still passed: an ordering check cannot
        // see a two-tick shift that moves both readings the same way.
        //
        // A value that only an inequality constrains is a value any arithmetic
        // can produce. This pins BOTH readings against the bars they came from,
        // recomputed here from the raw candles rather than from the fill module,
        // so a change in either place has to be deliberate.
        let (bars, column) = swept();
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Direction::Long,
        );
        assert!(t.count() > 0, "the fixture must produce trades");

        for trade in &t.trades {
            let e = bars.get(trade.entry_bar).expect("the entry bar exists");
            let x = bars.get(trade.exit_bar).expect("the exit bar exists");

            // BEST, long: buy the entry open, sell the exit open. No tick.
            assert_eq!(
                trade.best,
                x.open.saturating_sub(e.open),
                "the best case must be the two opens and nothing else -- trade \
                 at {}",
                trade.entry_bar
            );

            // WORST, long: buy the entry HIGH, sell the exit LOW. NO TICK.
            //
            // This asserted `high + TICK` and `low - TICK` until the operator
            // ruled that no figure may come from a price the bar did not print.
            // A tick outside the bar is exactly that, and
            // `costs::fill::Anchor` had already written the objection down:
            // "a fill invented one tick outside them is exactly the invention
            // CLAUDE.md section 3 rule 1 forbids".
            assert_eq!(
                trade.worst,
                x.low.saturating_sub(e.high),
                "the worst case must be the PRINTED extremes, with no tick added \
                 either way -- trade at {}",
                trade.entry_bar
            );

            // AND THE GAP IS THE FILL ASSUMPTION, priced. It is what an operator
            // is really asking when they ask how much of an edge is real.
            assert!(
                trade.best.saturating_sub(trade.worst) > 0,
                "on a bar with any range at all the two readings must differ"
            );
        }
    }
    #[test]
    fn the_worst_case_is_never_better_than_the_best_case() {
        // The two scenarios bracket every real fill. A worst case that came out
        // ahead would mean the adverse extreme was applied to the wrong leg,
        // which is the mistake `costs::fill` reconciled for shorts.
        for direction in [Direction::Long, Direction::Short] {
            let (bars, column) = swept();
            let t = walk(&bars, &column, &ConditionMask::default(), h(15), direction);
            assert!(t.count() > 0);
            for trade in &t.trades {
                assert!(
                    trade.worst <= trade.best,
                    "{direction:?} trade at {} priced worst {} above best {}",
                    trade.entry_bar,
                    trade.worst,
                    trade.best
                );
            }
        }
    }

    #[test]
    fn no_trade_is_held_past_the_square_off_or_into_another_day() {
        // RULE 1. The WHOLE round trip -- signal, entry and exit -- is inside
        // one session, and the exit is at or before that session's derived
        // square-off.
        //
        // # This test used to check two of those three, and the missing one was
        // # where the bug was
        //
        // It compared ENTRY against EXIT and stopped. A signal on the last bar
        // of day 5 takes `entry = signal + 1`, which is day 6's first bar, and
        // then exits on day 6 — so entry and exit agree perfectly and this test
        // passed while the position had been opened across the overnight gap.
        //
        // MEASURED at the time the signal leg was added: 132 such trades across
        // 41 single-condition masks on `synthetic::sessions(12)`, the first
        // being bar 2,249 of day 5 filling on bar 2,250 of day 6. The guard is
        // now in `walk` as RULE 1b and this asserts all three legs.
        //
        // A test whose NAME says "into another day" and which checks one of the
        // two boundaries a day has is the failure mode this comment exists to
        // keep visible.
        let (bars, column) = swept();
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(60),
            Direction::Long,
        );
        assert!(t.count() > 0);

        for trade in &t.trades {
            let signal_ts = bars.get(trade.signal_bar).map_or(0, |b| b.ts_micros);
            let entry_ts = bars.get(trade.entry_bar).map_or(0, |b| b.ts_micros);
            let exit_ts = bars.get(trade.exit_bar).map_or(0, |b| b.ts_micros);
            assert_eq!(
                indicators::ist_day(signal_ts),
                indicators::ist_day(entry_ts),
                "a signal on one day filled on another -- the position opened \
                 across the overnight gap the intraday square-off exists to \
                 prevent"
            );
            assert_eq!(
                indicators::ist_day(entry_ts),
                indicators::ist_day(exit_ts),
                "a trade entered on one day exited on another"
            );
            // The fixed 15:09 last-fill minute is computed independently by the
            // helper below, not read out of the production table.
            let deadline = last_fill_minute_of_day(&bars, indicators::ist_day(exit_ts));
            assert!(
                ist_minute_of(exit_ts) <= deadline,
                "a trade exited at IST minute {}, past its own session's \
                 square-off at {deadline}",
                ist_minute_of(exit_ts)
            );
        }
    }

    #[test]
    fn a_long_horizon_forces_every_trade_and_says_so() {
        // At H=400 no trade can reach its horizon inside a 375-bar session, so
        // every exit must be the square-off and every trade must be marked.
        let (bars, column) = swept();
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(400),
            Direction::Long,
        );
        assert!(t.count() > 0);
        assert!(
            t.trades.iter().all(|x| x.forced),
            "at H=400 every exit is its session's square-off and must be flagged"
        );
    }

    #[test]
    fn a_bar_the_fill_model_refuses_is_counted_as_too_late_and_never_traded() {
        // THE ENTRY BAR IS ONE THE COLUMN MAY HAVE SKIPPED.
        //
        // `Column::build` refuses corrupt bars, so they never become SIGNALS.
        // But the entry is `signal + 1`, read from the raw slice, so a bar the
        // column passed over still reaches `round_trip` -- and
        // `costs::fill::Bar::new` refuses a price below one tick, because a fill
        // there is a price nobody traded at.
        //
        // Nothing had ever exercised that path. If it were wrong the walk would
        // either panic or, worse, trade at a fabricated price and report it as
        // an ordinary result. It must count the signal as `too_late` and take
        // no position, and the reconciliation must still hold.
        let mut bars = crate::synthetic::sessions(8);
        // Price every bar at one paisa -- below the five-paisa tick, so every
        // fill is refused. Timestamps are untouched, so the bars stay valid
        // candles and the column still sweeps them.
        for bar in &mut bars {
            *bar = indicators::Candle::new(bar.ts_micros, 1, 1, 1, 1, 100, indicators::OI_NULL);
        }
        let column = Column::build(&bars, &mut evaluator());
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Direction::Long,
        );

        assert!(t.signals > 0, "the fixture must still produce signals");
        assert_eq!(
            t.count(),
            0,
            "not one trade may be taken at a price the fill model refuses"
        );
        assert!(
            t.too_late > 0,
            "a refused fill must be counted, not dropped"
        );
        assert!(
            t.reconciles(),
            "every signal must still land in exactly one bucket: {t:?}"
        );
    }

    /// A valid entry followed by an invalid exit is still an open position.
    ///
    /// The old walk called `round_trip`, received `None`, counted the signal,
    /// and left `open_until` empty. The very next signal then opened inside the
    /// interval whose entry had already printed. Entry refusal and exit refusal
    /// are opposite occupancy facts and must never share that answer.
    #[test]
    fn an_unpriceable_exit_still_occupies_both_long_and_short_walks() {
        let mut bars = crate::synthetic::sessions(8);
        let probe = Column::build(&bars, &mut evaluator());
        let first = *probe.sources().first().expect("the fixture must sweep");
        let second = *probe
            .sources()
            .get(1)
            .expect("the fixture needs an overlapping successor");
        assert_eq!(
            second,
            first.saturating_add(1),
            "the first two signals must be adjacent"
        );

        let failed_entry = first.saturating_add(1);
        let failed_exit = failed_entry.saturating_add(2);
        let corrupt = bars
            .get_mut(failed_exit)
            .expect("the two-bar horizon fits the fixture");
        corrupt.close = corrupt.high.saturating_add(1);
        assert!(
            corrupt.check().is_err(),
            "the exit record must actually be unpriceable"
        );

        let column = Column::build(&bars, &mut evaluator());
        let facts = super::SliceFacts::of(&bars, &column);
        let conservative = facts
            .exits()
            .get(failed_entry)
            .copied()
            .flatten()
            .expect("the entry has a session boundary")
            .bar;
        for direction in [Direction::Long, Direction::Short] {
            let walked = walk(&bars, &column, &ConditionMask::default(), h(2), direction);
            let held = walked
                .occupancy
                .first()
                .expect("the valid entry must leave an occupancy interval");
            assert_eq!(held.entry_bar, failed_entry, "the known entry is kept");
            assert_eq!(
                held.exit_bar, conservative,
                "a refused timed exit keeps the position occupied through the \
                 proved square-off"
            );
            assert!(
                !held.priceable,
                "a corrupt exit can hold the position but can never become money"
            );
            assert!(
                walked
                    .trades
                    .iter()
                    .all(|trade| trade.entry_bar > conservative),
                "{direction:?}: no later trade may enter on or before the \
                 unpriceable path's conservative exit bar"
            );
            assert!(walked.reconciles(), "{direction:?}: {walked:?}");
        }
    }

    /// Both stateful evaluator refusals gate every role a bar can play in a
    /// long or short trade. A refused interior/exit also leaves a conservative
    /// occupancy interval so the next signal cannot overlap an unresolved
    /// position.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one adversarial cross-product over two refusals, three path \
                  roles, and both directions; keeping it together proves every cell"
    )]
    fn stateful_refusals_gate_long_and_short_entries_exits_and_interiors() {
        #[derive(Clone, Copy, Debug)]
        enum Poison {
            DuplicateTime,
            Accumulator,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Role {
            Entry,
            Interior,
            Exit,
        }

        let clean = crate::synthetic::sessions(8);
        let horizon = h(4);
        for poison in [Poison::DuplicateTime, Poison::Accumulator] {
            let availability = match poison {
                Poison::DuplicateTime => Availability::Absent,
                Poison::Accumulator => Availability::Present,
            };
            let clean_column = Column::build(&clean, &mut evaluator_with(availability));
            let baseline = walk(
                &clean,
                &clean_column,
                &ConditionMask::default(),
                horizon,
                Direction::Long,
            );
            let first = baseline
                .trades
                .first()
                .copied()
                .expect("the warm regular fixture takes a baseline trade");
            assert!(
                !first.forced,
                "the first trade must reach its exact horizon"
            );

            for role in [Role::Entry, Role::Interior, Role::Exit] {
                let victim = match role {
                    Role::Entry => first.entry_bar,
                    Role::Interior => first.entry_bar.saturating_add(2),
                    Role::Exit => first.exit_bar,
                };
                let mut dirty = clean.clone();
                match poison {
                    Poison::DuplicateTime => {
                        let prior = dirty
                            .get(victim.saturating_sub(1))
                            .map_or(0, |bar| bar.ts_micros);
                        if let Some(bar) = dirty.get_mut(victim) {
                            bar.ts_micros = prior;
                        }
                    }
                    Poison::Accumulator => {
                        let stamp = dirty.get(victim).map_or(0, |bar| bar.ts_micros);
                        if let Some(bar) = dirty.get_mut(victim) {
                            *bar = indicators::Candle::new(
                                stamp,
                                5_000_000_000_000_000_000,
                                5_000_000_000_000_000_000,
                                5_000_000_000_000_000_000,
                                5_000_000_000_000_000_000,
                                1,
                                indicators::OI_NULL,
                            );
                        }
                    }
                }
                assert!(
                    dirty.get(victim).is_some_and(|bar| bar.check().is_ok()),
                    "{poison:?}/{role:?} must be stateful, not candle-local"
                );
                let column = Column::build(&dirty, &mut evaluator_with(availability));
                assert!(!column.accepts(victim));

                for direction in [Direction::Long, Direction::Short] {
                    let walked = walk(
                        &dirty,
                        &column,
                        &ConditionMask::default(),
                        horizon,
                        direction,
                    );
                    assert!(
                        walked
                            .trades
                            .iter()
                            .all(|trade| victim < trade.entry_bar || victim > trade.exit_bar),
                        "{poison:?}/{role:?}/{direction:?}: a refused bar entered a priced trade"
                    );
                    if role != Role::Entry {
                        assert!(
                            walked.occupancy.iter().any(|held| {
                                !held.priceable
                                    && held.entry_bar <= victim
                                    && held.exit_bar >= victim
                            }),
                            "{poison:?}/{role:?}/{direction:?}: the unresolved \
                             position vanished instead of blocking overlap"
                        );
                    }
                    assert!(walked.reconciles(), "{poison:?}/{role:?}/{direction:?}");
                }
            }
        }
    }

    #[test]
    fn an_empty_column_produces_no_trades_and_still_reconciles() {
        let bars: Vec<indicators::Candle> = Vec::new();
        let column = Column::build(&bars, &mut evaluator());
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Direction::Long,
        );
        assert_eq!(t, Trades::default());
        assert!(t.reconciles());
    }

    /// Seven complete regular sessions followed by one that STOPS MID-DAY after
    /// 300 bars, at 14:14 IST. It contains no 15:09 row and therefore cannot
    /// certify the fixed 15:10 forced exit.
    ///
    /// # Eight sessions, and not the two the defect needs
    ///
    /// `Evaluator::warmed_up` requires **five completed sessions** before
    /// `Column::build` sweeps a single bar. A two-session fixture -- the obvious
    /// way to write "one whole session and one cut short" -- sweeps NOTHING, and
    /// every assertion below would then hold over an empty trade list while
    /// proving nothing. The count is eight to match `swept`, so the two fixtures
    /// differ only in session length.
    ///
    /// # The exact required minute, not position in the slice, is the distinction
    ///
    /// Sessions 0 through 6 carry 15:09. Session 7 stops before it. Adding a
    /// ninth day's first bar cannot provide the missing prior-day record.
    fn sessions_cut_mid_day() -> (Vec<indicators::Candle>, Column) {
        let mut bars = crate::synthetic::sessions(7);
        bars.extend((0..300).map(|minute| crate::synthetic::bar(7, minute)));
        let column = Column::build(&bars, &mut evaluator());
        (bars, column)
    }

    /// The IST day of bar `at`, or zero where the slice has no such bar.
    fn day_of(bars: &[indicators::Candle], at: usize) -> i64 {
        bars.get(at).map_or(0, |b| indicators::ist_day(b.ts_micros))
    }

    /// Does bar `at`'s raw IST day contain exactly one 15:09 row?
    ///
    /// Read off the BARS, deliberately, rather than by calling
    /// `SessionBounds::day_ended`. A test that asks the walk's own predicate
    /// whether the walk was right compares a function to itself; this one goes
    /// back to the timestamps that predicate is supposed to be derived from.
    ///
    /// This deliberately does not call the production predicate. It counts the
    /// raw required timestamp independently, so a neighbour or later day cannot
    /// make it true.
    fn forced_price_exists(bars: &[indicators::Candle], at: usize) -> bool {
        let here = day_of(bars, at);
        bars.iter()
            .filter(|bar| indicators::ist_day(bar.ts_micros) == here)
            .filter(|bar| ist_minute_of(bar.ts_micros) == FORCED_EXIT_MINUTE - 1)
            .count()
            == 1
    }

    /// The last observed IST minute of `day` at which a fill may land under the
    /// fixed policy. A truncated day returns its last observed pre-deadline row;
    /// the separate `real` flag still refuses using that row as liquidation.
    fn last_fill_minute_of_day(bars: &[indicators::Candle], day: i64) -> i64 {
        bars.iter()
            .filter(|b| indicators::ist_day(b.ts_micros) == day)
            .map(|b| ist_minute_of(b.ts_micros))
            .max()
            .unwrap_or(0)
            .min(FORCED_EXIT_MINUTE - 1)
    }

    /// One flat bar at `ts`. The price never moves, because this fixture is
    /// about WHEN a hold ends, not about what it earned.
    fn flat(ts: i64) -> indicators::Candle {
        indicators::Candle {
            ts_micros: ts,
            open: 2_500_000,
            high: 2_500_000,
            low: 2_500_000,
            close: 2_500_000,
            volume: 1,
            open_interest: indicators::OI_NULL,
        }
    }

    /// A HOLD OF FIFTEEN BARS BECAME A HOLD OF A HUNDRED AND FOUR MINUTES.
    ///
    /// # The day this is built from is real
    ///
    /// **2024-03-02** is an NSE disaster-recovery Saturday. The operator's store
    /// holds 105 one-minute bars for it: 09:15–09:59, then nothing until 11:30,
    /// then 11:30–12:29. A long entered at 09:57 with `Horizon::DEFAULT` exited
    /// fifteen BARS later — at 11:41, a **104-minute hold billed as fifteen
    /// minutes**, carried across a 91-minute closure. 2024-05-18 is the same
    /// shape.
    ///
    /// The duration is the smaller half. No bar exists inside the gap, so
    /// `excursion::crossings` examines none, so no stop, target or trail can
    /// fire there — every exit order is inert for ninety-one minutes while the
    /// position is open. That biases the trade toward the favourable side, which
    /// is the direction §4 is about.
    ///
    /// # What this asserts
    ///
    /// The hold has no exit when its exact deadline falls inside the halt. The
    /// prior bar is not an exit that happened at the future deadline.
    #[test]
    fn a_hold_does_not_run_across_an_intraday_halt() {
        // 09:15..09:59 then 11:30..12:29, one minute apart within each block.
        let mut bars: Vec<indicators::Candle> = Vec::new();
        let day = 1_709_337_600_000_000_i64; // 2024-03-02 00:00 UTC
        let ist = 19_800_000_000_i64;
        for m in 0..45_i64 {
            let ts = day - ist + (9 * 60 + 15 + m) * 60_000_000;
            bars.push(flat(ts));
        }
        for m in 0..60_i64 {
            let ts = day - ist + (11 * 60 + 30 + m) * 60_000_000;
            bars.push(flat(ts));
        }
        assert_eq!(bars.len(), 105, "the fixture is the real day's bar count");

        let step = super::median_step_micros(&bars);
        assert_eq!(
            step, 60_000_000,
            "one minute, taken as the MEDIAN so the \
                                      single 91-minute gap cannot move it"
        );

        // Entry at 09:57 is index 42; fifteen bars later is index 57, which is
        // 11:42 -- on the far side of the closure.
        let column = Column::build(&bars, &mut evaluator());
        let facts = super::SliceFacts::of(&bars, &column);
        let entry = 42_usize;
        assert_eq!(
            super::horizon_bar(&bars, entry, 15, step, &facts),
            None,
            "09:59 is before the 10:12 deadline, not a retroactive exit at it"
        );

        // AND A CONTIGUOUS SLICE IS UNCHANGED, or this would be a behaviour
        // change dressed as a bug fix. Fifteen bars from index 0 is index 15.
        assert_eq!(
            super::horizon_bar(&bars, 0, 15, step, &facts),
            Some(15),
            "with no gap in the way the answer is exactly `entry + h`"
        );
    }
    #[test]
    fn a_slice_that_stops_mid_session_fabricates_no_square_off() {
        // THE DEFECT, as the fixture that produced it. `exit =
        // wanted.min(forced)` treated the last bar of a truncated slice as the
        // 15:10 close, so a hold that overran its horizon in the final session
        // exited at 14:14 and reported `forced: true` -- a square-off on a bar
        // where none happened, arriving in the table with a best/worst pair and
        // a return, indistinguishable from a real round trip.
        //
        // H=400 against 300-bar sessions so that EVERY hold overruns. No trade
        // in this fixture can reach its horizon, so the only thing a trade can
        // be here is a square-off, and the sessions then differ in exactly one
        // respect: sessions 0..=6 contain a complete accepted regular path;
        // session 7 is a truncated prefix.
        let (bars, column) = sessions_cut_mid_day();
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(400),
            Direction::Long,
        );

        assert!(
            t.signals > 0,
            "the fixture swept nothing, so every assertion below would hold \
             over an empty list and prove nothing: {t:?}"
        );
        assert!(t.reconciles(), "every signal lands in one bucket: {t:?}");

        let cut = bars.len().saturating_sub(1);
        for trade in &t.trades {
            assert_ne!(
                trade.exit_bar, cut,
                "a trade exited on the last bar of the slice, 14:14 IST: the \
                 data ran out there, the session did not"
            );
            // The general form: an exit called forced must belong to a day with
            // exactly one raw 15:09 row.
            assert!(
                !trade.forced || forced_price_exists(&bars, trade.exit_bar),
                "trade exited at bar {} and flagged it as a square-off, but \
                 that day has no unique 15:09 price",
                trade.exit_bar
            );
        }

        assert!(
            t.too_late > 0,
            "the signals in the truncated tail have no exit, so they must be \
             counted as refusals rather than traded: {t:?}"
        );
        // WITHOUT THIS the loop above is vacuous. An empty trade list satisfies
        // "no trade is wrongly forced" perfectly, so a fix that refused the
        // whole fixture would pass it. The real square-offs, on the sessions
        // that genuinely ended, must survive.
        assert!(
            t.trades.iter().any(|x| x.forced),
            "the square-offs on the sessions that genuinely ended must still be \
             taken and still be flagged: {t:?}"
        );
    }

    #[test]
    fn a_horizon_that_fits_inside_the_truncated_session_is_still_traded() {
        // THE SECOND WRONG ANSWER, refused. Dropping the final session whole
        // would have been the easier fix and a worse one: a five-bar hold
        // entered at 11:00 on a slice that stops at 14:14 COMPLETED -- both its
        // prices printed and both fills are real -- and discarding it would
        // shrink `n` for a reason that has nothing to do with the data.
        //
        // Only the OVERRUN has nowhere to go. This pins that difference, and it
        // is why `forced_exits` answers `real: false` rather than `None` for a
        // session the file cut short.
        let (bars, column) = sessions_cut_mid_day();
        let t = walk(
            &bars,
            &column,
            &ConditionMask::default(),
            h(5),
            Direction::Long,
        );
        assert!(t.reconciles(), "every signal lands in one bucket: {t:?}");

        let cut_day = day_of(&bars, bars.len().saturating_sub(1));
        let mut in_cut_session = 0_usize;
        for trade in &t.trades {
            if day_of(&bars, trade.entry_bar) != cut_day {
                continue;
            }
            in_cut_session = in_cut_session.saturating_add(1);
            assert!(
                !trade.forced,
                "the trade entered at bar {} was flagged a square-off in a \
                 session that has no proven square-off at all",
                trade.entry_bar
            );
        }
        assert!(
            in_cut_session > 0,
            "a session the slice cut short is still tradeable up to the cut, \
             and refusing all of it would be a second wrong answer: {t:?}"
        );
    }

    /// NOTHING A CANDIDATE LOOP CALLS MAY SORT, AND THIS READS THE SOURCE TO
    /// SAY SO.
    ///
    /// # Why a behavioural test cannot reach this
    ///
    /// `median_step_micros` is a pure function of `bars`, so calling it once per
    /// candidate and once per slice produce **identical answers**. Every
    /// assertion about trades, counts, returns and reconciliation passed while
    /// the median allocation sat inside the per-candidate entry point — that is
    /// precisely how it survived, and `walk_with`'s own doc had already written
    /// down why no ratio gate could see it either: work that scales uniformly
    /// does not change a ratio.
    ///
    /// What is left is the SHAPE of the code, so that is what this asserts. The
    /// hot path is [`walk_core`], which both public entry points delegate to,
    /// and it may not derive a per-slice fact at all: no `median_step_micros`,
    /// no `forced_exits`, no median selection or sort of any kind.
    ///
    /// # The scan, and why it does not count braces
    ///
    /// A brace count over Rust source is unsound — braces live inside string
    /// literals, and this file is mostly prose. The body is taken from the
    /// signature to the next `\n}\n`, which is a closing brace at column zero:
    /// the one place `rustfmt` puts an item's end and nowhere a string literal
    /// in this file reaches.
    #[test]
    fn the_per_candidate_walk_derives_nothing_and_sorts_nothing() {
        let source = include_str!("trade.rs");
        let start = source
            .find("fn walk_core(")
            .expect("the hot path must still be called walk_core");
        let rest = source.get(start..).unwrap_or_default();
        let end = rest
            .find("\n}\n")
            .expect("walk_core must end at a closing brace in column zero");
        let body = rest.get(..end).unwrap_or_default();

        assert!(
            body.len() > 1_000,
            "the scan found a {}-byte body, which is not the walk -- the \
             delimiters moved and this test would pass over nothing",
            body.len()
        );
        for banned in [
            "median_step_micros(",
            "forced_exits(",
            "SliceFacts::of(",
            ".sort",
            "to_vec()",
            "Vec::with_capacity(",
        ] {
            assert!(
                !body.contains(banned),
                "`{banned}` is inside `walk_core`, which runs once per \
                 CANDIDATE. Every per-slice fact belongs in `SliceFacts`, \
                 hoisted out of the loop -- see its doc for what this cost when \
                 the bar spacing was left behind."
            );
        }
    }

    /// THE THREE ENTRY POINTS ARE ONE WALK, or the hoist changed an answer.
    ///
    /// `walk`, `walk_with(Some(..))` and `walk_over` differ only in where the
    /// per-slice facts came from. If hoisting them moved a single trade, the
    /// facts were not per-slice after all and the whole argument collapses.
    #[test]
    fn hoisting_the_slice_facts_changes_no_trade() {
        let (bars, column) = swept();
        let facts = super::SliceFacts::of(&bars, &column);
        let exits = super::forced_exits(&bars);
        let mask = ConditionMask::default();

        let plain = walk(&bars, &column, &mask, h(15), Direction::Long);
        let hoisted = super::walk_over(&bars, &column, &mask, h(15), Direction::Long, &facts);
        let half = super::walk_with(&bars, &column, &mask, h(15), Direction::Long, Some(&exits));

        assert!(
            plain.count() > 0,
            "the fixture must take trades, or the three answers agree over \
             nothing: {plain:?}"
        );
        assert_eq!(plain, hoisted, "walk_over must be walk");
        assert_eq!(plain, half, "walk_with must be walk");

        // AND THE FACTS THEMSELVES ARE THE SAME FACTS, read through the
        // accessors the callers use.
        assert_eq!(facts.exits().len(), exits.len());
        assert_eq!(
            facts.step_micros(),
            super::median_step_micros(&bars),
            "the spacing a caller hoists is the spacing the walk would have \
             measured"
        );
        assert_eq!(
            facts.step_micros(),
            60_000_000,
            "one minute, on a one-minute fixture"
        );
    }

    /// A LATER DAY CANNOT SUPPLY AN EARLIER DAY'S MISSING 15:09 RECORD.
    #[test]
    fn completeness_not_a_later_day_proves_a_session_ended() {
        // One session contains the exact required row even though nothing
        // follows it.
        let whole: Vec<indicators::Candle> = (9 * 60 + 15..=15 * 60 + 29)
            .map(|m| flat(ist_minute_stamp(0, m)))
            .collect();
        let alone = SessionBounds::of(&whole);
        assert!(
            alone.day_ended(0),
            "the accepted unique 15:09 row proves the fixed fill"
        );
        assert!(
            alone.fillable(0) && !alone.fillable(whole.len() - 1),
            "the boundary still applies on that day -- admitting its last ten \
             minutes would open positions inside the square-off window"
        );

        // A truncated day remains unproved even when tomorrow follows it.
        let mut cut_then_tomorrow: Vec<indicators::Candle> = (9 * 60 + 15..=14 * 60 + 14)
            .map(|m| flat(ist_minute_stamp(0, m)))
            .collect();
        cut_then_tomorrow.push(flat(ist_minute_stamp(1, 9 * 60 + 15)));
        let after = SessionBounds::of(&cut_then_tomorrow);
        assert!(
            !after.day_ended(0),
            "tomorrow does not prove yesterday reached the regular close"
        );
        assert!(
            !after.day_ended(cut_then_tomorrow.len() - 1),
            "and the new final day is unproven in its turn -- the property is \
             about the day, not about the index"
        );

        // A BAR THE SLICE DOES NOT HOLD BELONGS TO NO SESSION, so it is neither
        // fillable nor proven. Both answers are `false` and they are reached by
        // the same bounds check.
        assert!(!after.day_ended(9_999) && !after.fillable(9_999));
        assert!(
            after.last_fill_minute(9_999).is_none(),
            "and there is no boundary to report for a bar that is not there"
        );
        assert!(
            !after.same_day(0, 9_999) && !after.same_day(9_999, 0),
            "two bars that do not both exist share no day, whichever is missing"
        );
    }

    #[test]
    fn the_forced_exit_table_flags_a_real_window_end_and_refuses_the_cut() {
        // The table `walk` reads, asserted directly rather than through the
        // trades it produces. Both fields matter and a trade only ever shows
        // one of them: `bar` is what the existing square-off tests check, `real`
        // is what RULE 1c branches on, and a table naming the right bar with the
        // wrong flag would refuse every genuine square-off or fabricate every
        // truncated one -- while every assertion about `exit_bar` still passed.
        //
        // The complete sessions contain the exact 15:09 row. The cut session
        // ends at 14:14 and cannot prove any forced exit; its last observed bar
        // may still settle an ordinary horizon that ends before the deadline.
        let (bars, _) = sessions_cut_mid_day();
        let table = super::forced_exits(&bars);

        // Bar 0 inherits session 0's square-off because that day contains the
        // exact accepted 15:09 row.
        let first = table
            .first()
            .copied()
            .flatten()
            .expect("bar 0 is inside the tradeable window, so it has an exit");
        assert_eq!(
            first.bar, 354,
            "the exact 15:09 bar closes at the fixed 15:10 deadline"
        );
        assert!(
            first.real,
            "the exact accepted minute proves the forced price"
        );

        // The same question in the final session, where the file stops instead.
        let cut_session = table
            .get(7 * crate::synthetic::BARS_PER_SESSION)
            .copied()
            .flatten()
            .expect("the final session's first bar is tradeable too");
        assert_eq!(
            cut_session.bar,
            bars.len().saturating_sub(1),
            "the cut session's last observed pre-deadline bar bounds ordinary \
             horizons, but cannot become a forced fill"
        );
        assert!(
            !cut_session.real,
            "the final session is a truncated prefix, so nothing proves a \
             square-off -- regardless of what follows it"
        );

        // A position may enter at the 15:09 OPEN and must leave at that bar's
        // 15:10 CLOSE. A 15:10-stamped bar is already post-deadline, so every
        // row from the next offset onward has no square-off to reach.
        for offset in 355..375 {
            assert!(
                table.get(offset).copied().flatten().is_none(),
                "bar {offset} of session 0 is past its forced exit, so nothing \
                 may be opened on it"
            );
        }
    }

    /// The deadline is priced from the exact stored interval that ENDS there.
    /// A spectacular 15:10 row is post-deadline and cannot move either fill;
    /// changing the accepted 15:09 row must move the result, or the required
    /// one-minute execution series is decoration rather than an input.
    #[test]
    fn the_1509_row_prices_the_forced_fill_and_the_1510_row_is_unreachable() {
        let mut bars = crate::synthetic::sessions(8);
        let day = 6_usize;
        let start = day.saturating_mul(crate::synthetic::BARS_PER_SESSION);
        let entry = start.saturating_add(200);
        let forced = start.saturating_add(354);
        let post_deadline = forced.saturating_add(1);

        let column = Column::build(&bars, &mut evaluator());
        let facts = super::SliceFacts::of(&bars, &column);
        let square_off = facts
            .exits()
            .get(entry)
            .copied()
            .flatten()
            .expect("the complete day has a square-off");
        assert_eq!(square_off.bar, forced, "15:09 is the forced fill row");
        assert!(square_off.real, "the exact row is accepted and unique");
        let baseline = super::round_trip(
            &bars,
            entry.saturating_sub(1),
            entry,
            square_off.bar,
            true,
            Direction::Long,
            &facts,
        )
        .expect("both printed legs are priceable");

        // Change ONLY 15:10. Both open and printed extremes move violently,
        // but this row starts after the fixed liquidation instant.
        let post = bars.get(post_deadline).copied().expect("15:10 exists");
        *bars
            .get_mut(post_deadline)
            .expect("the copied 15:10 row remains present") = indicators::Candle::new(
            post.ts_micros,
            9_000_000,
            9_100_000,
            8_900_000,
            9_000_000,
            post.volume,
            post.open_interest,
        );
        let post_column = Column::build(&bars, &mut evaluator());
        let post_facts = super::SliceFacts::of(&bars, &post_column);
        let post_square_off = post_facts
            .exits()
            .get(entry)
            .copied()
            .flatten()
            .expect("15:10 cannot erase the earlier proof");
        let after_post = super::round_trip(
            &bars,
            entry.saturating_sub(1),
            entry,
            post_square_off.bar,
            true,
            Direction::Long,
            &post_facts,
        )
        .expect("the forced legs remain priceable");
        assert_eq!(post_square_off.bar, forced);
        assert_eq!(after_post, baseline, "post-deadline OHLCV changed a fill");

        // Change 15:09 itself. This is the hidden one-minute path the signal
        // series cannot reveal, so both result and run identity must bind it.
        let exit = bars.get(forced).copied().expect("15:09 exists");
        *bars
            .get_mut(forced)
            .expect("the copied 15:09 row remains present") = indicators::Candle::new(
            exit.ts_micros,
            exit.open.saturating_add(10_000),
            exit.high.saturating_add(10_000),
            exit.low.saturating_add(10_000),
            exit.close.saturating_add(10_000),
            exit.volume,
            exit.open_interest,
        );
        let exit_column = Column::build(&bars, &mut evaluator());
        let exit_facts = super::SliceFacts::of(&bars, &exit_column);
        let changed = super::round_trip(
            &bars,
            entry.saturating_sub(1),
            entry,
            forced,
            true,
            Direction::Long,
            &exit_facts,
        )
        .expect("the changed printed row remains valid");
        assert_ne!(changed, baseline, "the required 15:09 OHLCV was ignored");
    }

    /// The shipping sweep path brackets fills only by prices that actually
    /// printed. The legacy adverse-tick helper remains in `costs`, but it is not
    /// reachable from this production round-trip function.
    #[test]
    fn the_active_round_trip_cannot_reach_the_adverse_tick_fill_policy() {
        let source = include_str!("trade.rs");
        let body = source
            .split_once("fn round_trip(")
            .and_then(|(_, rest)| rest.split_once("fn entry_is_priceable("))
            .map(|(body, _)| body)
            .expect("round_trip remains bounded by entry_is_priceable");
        let code: String = body
            .lines()
            .map(str::trim_start)
            .filter(|line| !line.starts_with("//"))
            .collect();
        assert!(code.contains("Anchor::Open"));
        assert!(code.contains("Anchor::PrintedExtreme"));
        assert!(!code.contains("Anchor::AdverseExtreme"));
        assert!(!code.contains("worst_case_fills("));
    }
}
