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
//!    squared off compulsorily at **15:10 IST**, long or short, whether or not
//!    the horizon has run out. Nothing is ever held overnight.
//! 2. **Fills happen on the NEXT bar.** A signal is a fact about a bar's close;
//!    you cannot trade at a price that has already printed. A signal on bar `N`
//!    fills on bar `N + 1`.
//! 3. **Two cases, both reported, neither chosen.** BEST is the next bar's open.
//!    WORST is the adverse extreme of the next bar — the high if you are buying,
//!    the low if you are selling. A single number would hide which of the two a
//!    result depended on.
//! 4. **One position at a time.** While a trade is open, no further signal opens
//!    anything — not another long, and not a short. The next eligible signal is
//!    the first one at or after the exit bar.
//!
//! # Both fills go through `crates/costs`
//!
//! [`costs::fill::worst_case_fills`] applies one tick of adverse movement to
//! each leg, floors the sell, and refuses rather than saturates at the `i64`
//! edge. It is the function `COSTS_VERIFIED` checks its worked examples
//! against. Reimplementing "a tick against you" here would be a second copy of
//! a rule that is already settled, and the two would drift.
//!
//! The BEST case is the same function over [`costs::fill::Bar::flat`] bars built
//! from the open, so best and worst differ only in the bar handed in — not in
//! the fill rule. A best case computed by a different code path would be
//! comparing two models rather than two scenarios.
//!
//! # What is NOT here, and why it is not invented
//!
//! Brokerage, STT, stamp duty, exchange charges and GST. `costs::trip::price`
//! computes all of them and needs a [`costs::trip::Contract`], a broker and a
//! quantity. The sweep runs on **spot indices**, and a spot index is not
//! tradeable: the contract that would actually be bought is a future or an
//! option, and choosing which is a decision `CLAUDE.md` §3 rule 1 forbids this
//! module from making up. The per-unit slippage IS applied, because that is a
//! property of the bar and not of a contract.
//!
//! So every figure here is **gross of charges and net of slippage**, and says so
//! in its own name rather than in a footnote.
//!
//! # Measured: what the two rules cost the old numbers
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
//! **Slippage alone flips the sign.** At H=5 and H=15 the best case is
//! profitable and the worst case is a heavy loss; only at H=60, where there are
//! 18 trades instead of 177, does the worst case stay positive. The mechanism is
//! not subtle: each round trip pays two ticks, so ten times the trades pays ten
//! times the spread. A model that reports one number cannot show this, and the
//! one number it would have reported is the optimistic one.
//!
//! These are figures from a SYNTHETIC fixture and prove nothing about the
//! market. They are here because they size the DEFECT, and that is a fact about
//! this code rather than about NIFTY.

use costs::fill::{Anchor, Bar as FillBar, Direction, fills_at, worst_case_fills};
use indicators::Candle;
use indicators::column::Column;
use vocab::ConditionMask;

use crate::outcome::Horizon;

/// One completed round trip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trade {
    /// The bar whose close carried the signal. Nothing is traded on it.
    pub signal_bar: usize,
    /// The bar the entry filled in — always `signal_bar + 1`.
    pub entry_bar: usize,
    /// The bar the exit filled in: the horizon, or the 15:10 square-off,
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
    /// Paisa per unit if both legs filled at the bar's adverse extreme, plus
    /// one tick on each leg.
    ///
    /// Never better than [`Self::best`], and the gap between them is the whole
    /// range of outcomes a real fill can land in. **Selection ranks on THIS
    /// one**, unchanged: a search ranked on the flattering reading picks
    /// whatever the flattering assumption helped most.
    pub worst: i64,
    /// True when the exit was the 15:10 square-off rather than the horizon.
    ///
    /// Counted because a strategy whose trades are mostly force-closed is not
    /// the strategy its horizon describes — it is a different one wearing that
    /// horizon's name.
    pub forced: bool,
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
    /// separates them from the 15:10 refusals.
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
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn walk(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    direction: Direction,
) -> Trades {
    let exits = forced_exits(bars);
    let h = horizon.as_bars() as usize;
    let mut out = Trades::default();
    // The bar the open position exits on. Until the first entry there is none,
    // and a signal is eligible only at or after it.
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
        let blocked = open_until.is_some_and(|until| signal < until);

        // RULE 2. The fill is on the NEXT bar; the signal bar's close has
        // already printed and cannot be traded at.
        // Every refusal below is a LATENESS, and a blocked signal is charged to
        // `while_open` rather than to it -- so each branch asks `blocked` first.
        // The macro-free way of writing that is one `late` closure, but the
        // counter is on `out` and a closure would borrow it, so it is spelled
        // out at each site.
        let Some(entry) = signal.checked_add(1) else {
            if blocked {
                out.while_open = out.while_open.saturating_add(1);
            } else {
                out.too_late = out.too_late.saturating_add(1);
            }
            continue;
        };
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
        // intraday rule permits and that this engine squares off at 15:10
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
        let wanted = entry.saturating_add(h);

        // RULE 1c. THE HOLD ENDS AT THE HORIZON OR AT THE 15:10 SQUARE-OFF,
        // WHICHEVER COMES FIRST — AND "THE SLICE RAN OUT" IS NEITHER.
        //
        // This was `wanted.min(forced)`, which is right for the first two and
        // fabricates the third. `forced_exits` answers "the last bar of this
        // bar's session a fill can land in", and on the final session of a
        // slice that was cut mid-day that bar is THE CUT. A slice ending at
        // 10:54 therefore exited at 10:54 and stamped `forced: true` on it: a
        // 15:10 square-off on a bar where no square-off happened, at a price
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
        let exit = if wanted <= forced.bar {
            wanted
        } else if forced.real {
            forced.bar
        } else {
            out.too_late = out.too_late.saturating_add(1);
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
        let Some(trade) = round_trip(bars, signal, entry, exit, exit < wanted, direction) else {
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
    // `Candle::check` is the same predicate the column applies, so this refuses
    // exactly what the census counted and never more. A refused leg returns
    // `None`, which `walk` already accounts as a signal that took no trade
    // rather than as a trade worth zero.
    //
    // `FillBar::new` below refuses an inverted or sub-tick bar and would have
    // caught SOME of these -- but only some. The bar that produced the sign flip
    // has `high >= low` and both legs above a tick; what makes it corrupt is
    // that its close sits outside `[low, high]`, which is a question
    // `costs::fill` never asks because a fill does not use the close.
    let e = bars.get(entry_bar)?;
    let x = bars.get(exit_bar)?;
    e.check().ok()?;
    x.check().ok()?;

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
    // most favourable price that PRINTED, the worst is the adverse extreme plus
    // a tick, and every achievable fill lies between them.

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
    let entry_fill_bar = FillBar::new(paisa(e.open), paisa(e.high), paisa(e.low)).ok()?;
    let exit_fill_bar = FillBar::new(paisa(x.open), paisa(x.high), paisa(x.low)).ok()?;
    let best_fills = fills_at(entry_fill_bar, exit_fill_bar, direction, Anchor::Open).ok()?;
    let worst_fills = worst_case_fills(entry_fill_bar, exit_fill_bar, direction).ok()?;

    Some(Trade {
        signal_bar,
        entry_bar,
        exit_bar,
        best: pnl(&best_fills),
        worst: pnl(&worst_fills),
        forced,
    })
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
struct SquareOff {
    /// The last bar of this bar's own session a fill can still land in.
    bar: usize,
    /// True when [`is_window_end`] holds at [`Self::bar`]: a bar after it exists
    /// and lies in another session or past the square-off, so the tradeable
    /// window genuinely ENDED there.
    ///
    /// False when `bar` is only where the slice stopped. There is then no 15:10
    /// price at all, and a hold that overruns its horizon has no exit rather
    /// than an exit here — [`walk`]'s RULE 1c.
    real: bool,
}

/// For each bar, where a position opened on it is squared off — and whether
/// that square-off is a fact about the session or about the file.
///
/// One backward pass, so every bar gets its answer in O(1) amortised rather than
/// searching its session. `None` where the bar is at or past the square-off:
/// nothing entered there can be exited inside its own session at all.
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
/// is the SHAPE: the loop body is a fixed number of integer operations, one
/// `Option` copy and one [`is_window_end`], which reads at most one neighbouring
/// stamp; and the answer for bar `i` is inherited from bar `i + 1` whenever they
/// share a day, so nothing re-walks a session. A forward search per bar would be
/// O(H) with `H` caller-supplied — constant only by accident, which is the kind
/// of bound `CLAUDE.md` §3 rule 6 asks to be labelled rather than asserted.
fn forced_exits(bars: &[Candle]) -> Vec<Option<SquareOff>> {
    let stamps: Vec<(i64, i64)> = bars
        .iter()
        .map(|b| (indicators::ist_day(b.ts_micros), minute_of_day(b.ts_micros)))
        .collect();

    let mut out: Vec<Option<SquareOff>> = vec![None; bars.len()];
    for i in (0..bars.len()).rev() {
        let Some(&(day, minute)) = stamps.get(i) else {
            continue;
        };
        let inherited = i
            .checked_add(1)
            .filter(|&j| stamps.get(j).is_some_and(|&(d, _)| d == day))
            .and_then(|j| out.get(j).copied().flatten());
        let mine = if minute <= LAST_FILL_MINUTE {
            Some(SquareOff {
                bar: i,
                // Asked HERE, while the table is built, and not once per signal:
                // the answer depends only on the bar, so paying for it per
                // signal would put a lookup that a hot mask performs millions of
                // times behind a table that already exists. `inherited` then
                // carries the flag of the bar it NAMES rather than of the bar
                // inheriting it, which is the only pairing that stays true as
                // the chain walks backwards.
                real: is_window_end(&stamps, i),
            })
        } else {
            None
        };
        if let Some(slot) = out.get_mut(i) {
            *slot = inherited.or(mine);
        }
    }
    out
}

/// Is bar `j` the genuine last tradeable bar of its window, or just the last bar
/// in the slice?
///
/// The difference is a fill that happened against one that did not. A position
/// squared off at 15:10 has a real exit price and a real return; a slice that
/// simply stops at 10:54 has neither, and treating its final bar as a forced
/// close would report an exit the market never gave.
///
/// `j` ends the window when the bar after it belongs to another day, or is at or
/// past the forced close. When there is no bar after it, the data ran out and
/// the answer is no.
///
/// A SECOND COPY of [`crate::outcome`]'s function of the same name -- its BODY
/// character for character, and only the examples in this comment differ -- and
/// that is stated rather than tidied away. The rule belongs to both modules —
/// `outcome` measures the tail and `trade` fills in it — and unifying them means
/// a shared home for a rule that neither module owns.
/// Deferred rather than done here; until then the two must be changed together,
/// and the tests below pin this copy independently so a drift shows up as a
/// failure rather than as a quiet disagreement about the same slice.
fn is_window_end(stamps: &[(i64, i64)], j: usize) -> bool {
    let Some(&(day, _)) = stamps.get(j) else {
        return false;
    };
    j.checked_add(1)
        .and_then(|k| stamps.get(k))
        .is_some_and(|&(next_day, next_minute)| next_day != day || next_minute > LAST_FILL_MINUTE)
}

/// The last bar whose interval ends at or before the 15:10 square-off.
///
/// A bar is stamped at its OPEN, so the bar stamped 15:09 covers 15:09–15:10 and
/// its close IS the 15:10 price. The bar stamped 15:10 closes at 15:11, a minute
/// after the position is already gone — measured at a 36% error on fourteen
/// observations per session when it was wrong.
///
/// # Derived, because "the same constant" was a copy
///
/// This read `15 * 60 + 10 - 1` and its own doc said *"the same constant, and
/// the same reason, as `crate::outcome`"*. It was not the same constant. It was
/// a second, independent spelling of it, and the sentence claiming otherwise is
/// what would have made the divergence hard to see.
///
/// **This repository trades intraday only and squares off at 15:10.** That is
/// one rule, so it gets one definition: change
/// [`crate::outcome::AUTO_CLOSE_MINUTE`] and both the entry cut-off and the fill
/// boundary move together. Two copies of a policy constant is how one of them
/// gets updated.
const LAST_FILL_MINUTE: i64 = crate::outcome::AUTO_CLOSE_MINUTE - 1;

/// Minute of the IST day, `0..1440`.
const fn minute_of_day(ts_micros: i64) -> i64 {
    ts_micros
        .saturating_add(indicators::IST_OFFSET_MICROS)
        .div_euclid(60_000_000)
        .rem_euclid(1_440)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{LAST_FILL_MINUTE, Trades, walk};
    use crate::outcome::{AUTO_CLOSE_MINUTE, Horizon};

    /// THE SQUARE-OFF HAS ONE DEFINITION, AND THIS IS WHAT KEEPS IT THAT WAY.
    ///
    /// # The shape of the defect this replaces
    ///
    /// `trade` and `outcome` each held a `LAST_FILL_MINUTE`. `outcome` derived
    /// its own from `AUTO_CLOSE_MINUTE`; `trade` re-spelled `15 * 60 + 10 - 1`
    /// and carried a doc comment asserting *"the same constant … as
    /// `crate::outcome`"*. They agreed, so nothing failed — and the sentence
    /// claiming they were one constant is exactly what would have stopped anyone
    /// noticing when they stopped agreeing.
    ///
    /// This repository trades **intraday only** and squares off at 15:10. That
    /// is a policy, and a policy with two spellings is a policy that gets half
    /// updated.
    ///
    /// # Why the assertion is a relation, not a value
    ///
    /// Asserting `LAST_FILL_MINUTE == 909` would pin today's number and pass
    /// happily if someone moved the square-off and left the fill boundary
    /// behind — the very drift this exists to catch. Asserting the RELATION
    /// holds at any square-off time.
    #[test]
    fn the_fill_boundary_is_derived_from_the_square_off_and_not_re_spelled() {
        assert_eq!(
            LAST_FILL_MINUTE,
            AUTO_CLOSE_MINUTE - 1,
            "the last fillable bar is the one whose interval ENDS at the \
             square-off. A bar is stamped at its open, so the bar stamped 15:09 \
             covers 15:09-15:10 and its close IS the 15:10 price; the bar \
             stamped 15:10 closes at 15:11, a minute after the position is gone."
        );

        // AND THE POLICY ITSELF, stated once so a reader of this test learns the
        // rule rather than only the arithmetic. 15:10 IST = minute 910.
        assert_eq!(
            AUTO_CLOSE_MINUTE,
            15 * 60 + 10,
            "every position is force-closed at 15:10 IST — this engine is \
             intraday only and holds nothing overnight"
        );
    }

    use costs::fill::Direction;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use vocab::ConditionMask;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
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
                trade.entry_bar >= previous_exit || n == 0,
                "trade {n} entered at {} while the previous was open until {previous_exit}",
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
    fn the_best_case_is_the_open_exactly_and_the_worst_carries_two_ticks() {
        // Five paisa, the tick grid `CLAUDE.md` section 7 fixes. Declared at the
        // top of the function so `clippy::items_after_statements` is satisfied.
        const TICK: i64 = 5;

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

            // WORST, long: buy the entry HIGH plus a tick, sell the exit LOW
            // minus a tick. TICK is five paisa, so the round trip gives up ten.
            assert_eq!(
                trade.worst,
                x.low
                    .saturating_sub(TICK)
                    .saturating_sub(e.high.saturating_add(TICK)),
                "the worst case must be the adverse extremes plus a tick each \
                 way -- trade at {}",
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
        // one session, and the exit is at or before the bar whose close is
        // 15:10.
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
                 across the overnight gap the 15:10 square-off exists to prevent"
            );
            assert_eq!(
                indicators::ist_day(entry_ts),
                indicators::ist_day(exit_ts),
                "a trade entered on one day exited on another"
            );
            assert!(
                super::minute_of_day(exit_ts) <= super::LAST_FILL_MINUTE,
                "a trade exited at IST minute {}, past the 15:10 square-off",
                super::minute_of_day(exit_ts)
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
            "at H=400 every exit is the 15:10 square-off and must be flagged"
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

    /// Sessions that STOP MID-DAY: 300 bars each, so the last bar of every one
    /// of them is stamped 14:14 IST -- inside the tradeable window, and nowhere
    /// near the 15:10 square-off.
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
    /// # Only the LAST cut is detectable, and that asymmetry is the point
    ///
    /// Sessions 0 through 6 each end at 14:14 with another day's bar after them,
    /// so as far as anything here can tell their window ended -- which is also
    /// the right answer for a session that really was short (the 2025 Muhurat
    /// session was sixty bars), and the two cannot be told apart without an
    /// exchange calendar this repository does not have. Session 7's last bar is
    /// followed by nothing, and THAT is what a slice cut mid-session produces.
    fn sessions_cut_mid_day() -> (Vec<indicators::Candle>, Column) {
        let bars = crate::synthetic::session_of(8, 300);
        let column = Column::build(&bars, &mut evaluator());
        (bars, column)
    }

    /// The IST day of bar `at`, or zero where the slice has no such bar.
    fn day_of(bars: &[indicators::Candle], at: usize) -> i64 {
        bars.get(at).map_or(0, |b| indicators::ist_day(b.ts_micros))
    }

    /// Did the tradeable window genuinely end on bar `at`?
    ///
    /// Read off the BARS, deliberately, rather than by calling
    /// `super::is_window_end`. A test that asks the walk's own predicate whether
    /// the walk was right compares a function to itself; this one goes back to
    /// the timestamps that predicate was supposed to be derived from. Only the
    /// square-off constant is shared, because a second spelling of 15:10 would
    /// be a different rule rather than an independent check of this one.
    fn window_ends_at(bars: &[indicators::Candle], at: usize) -> bool {
        let here = day_of(bars, at);
        bars.get(at.saturating_add(1)).is_some_and(|next| {
            indicators::ist_day(next.ts_micros) != here
                || super::minute_of_day(next.ts_micros) > super::LAST_FILL_MINUTE
        })
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
        // respect: sessions 0..=6 end because the day did, session 7 because the
        // file did.
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
            // The general form of the assertion above, and the one that keeps
            // holding if the fixture ever grows a gap in its middle: an exit the
            // walk CALLS a square-off must be a bar the window really ended on.
            assert!(
                !trade.forced || window_ends_at(&bars, trade.exit_bar),
                "trade exited at bar {} and flagged it a 15:10 square-off, but \
                 the tradeable window did not end there",
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
                 session that has no 15:10 bar at all",
                trade.entry_bar
            );
        }
        assert!(
            in_cut_session > 0,
            "a session the slice cut short is still tradeable up to the cut, \
             and refusing all of it would be a second wrong answer: {t:?}"
        );
    }

    #[test]
    fn the_window_ends_where_the_session_does_and_not_where_the_file_does() {
        // `is_window_end` is a SECOND COPY of a rule `crate::outcome` also
        // holds, so it is pinned here independently. If the two ever drift, one
        // of the two tests fails -- rather than the two modules quietly
        // disagreeing about the same slice, which is the failure the
        // duplication was accepted to keep visible.
        //
        // One case per branch of the function, including the two that answer
        // "no" for entirely different reasons.
        let close = super::LAST_FILL_MINUTE;

        let then_past_the_close = [(0_i64, close), (0_i64, close.saturating_add(1))];
        assert!(
            super::is_window_end(&then_past_the_close, 0),
            "a bar followed, same day, by one past 15:10 ends the window"
        );

        let then_tomorrow = [(0_i64, close), (1_i64, 555_i64)];
        assert!(
            super::is_window_end(&then_tomorrow, 0),
            "a bar followed by the next session's 09:15 ends the window"
        );

        let mid_session = [(0_i64, 600_i64), (0_i64, 601_i64)];
        assert!(
            !super::is_window_end(&mid_session, 0),
            "a bar with a tradeable same-day bar after it is mid-window"
        );
        assert!(
            !super::is_window_end(&mid_session, 1),
            "the last stamp in the slice is where the FILE ended, and the end \
             of a file is not a square-off"
        );
        assert!(
            !super::is_window_end(&mid_session, 9),
            "an index past the end names no bar, so it ends no window"
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
        let bars = crate::synthetic::session_of(8, 300);
        let table = super::forced_exits(&bars);

        // Bar 0 inherits session 0's square-off, and that session ended because
        // the next day started.
        let first = table
            .first()
            .copied()
            .flatten()
            .expect("bar 0 is inside the tradeable window, so it has an exit");
        assert_eq!(first.bar, 299, "session 0's last bar is index 299");
        assert!(first.real, "another day follows 299, so that window ended");

        // The same question in the final session, where the file stops instead.
        let cut_session = table
            .get(2_100)
            .copied()
            .flatten()
            .expect("the final session's first bar is tradeable too");
        assert_eq!(
            cut_session.bar,
            bars.len().saturating_sub(1),
            "the final session's exit is the last bar the slice has"
        );
        assert!(
            !cut_session.real,
            "nothing follows the last bar of the slice, so the window did not \
             end there -- the file did"
        );
    }
}
