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

use costs::fill::{Bar as FillBar, Direction, worst_case_fills};
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
    /// Paisa per unit if both legs filled at the bar's open, one tick adverse.
    pub best: i64,
    /// Paisa per unit if both legs filled at the bar's adverse extreme.
    ///
    /// Never better than [`Self::best`], and the gap between them is the whole
    /// range of outcomes a real fill can land in.
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
    /// Bars the mask fired on, whether or not they became a trade.
    ///
    /// The number [`crate::outcome::edge`] would have used as its `n`. Reported
    /// beside [`Self::trades`] because the ratio between them is how much of the
    /// old figure was overlap.
    pub signals: u64,
    /// Signals ignored because a position was already open.
    pub while_open: u64,
    /// Signals that could not be entered: at or past the square-off, at the end
    /// of a session, or at the end of the slice.
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

        // RULE 4. A position is open, so this signal buys nothing. Not a long,
        // not a short, not a scale-in.
        if open_until.is_some_and(|until| signal < until) {
            out.while_open = out.while_open.saturating_add(1);
            continue;
        }

        // RULE 2. The fill is on the NEXT bar; the signal bar's close has
        // already printed and cannot be traded at.
        let Some(entry) = signal.checked_add(1) else {
            out.too_late = out.too_late.saturating_add(1);
            continue;
        };
        // RULE 1. The entry bar must itself be inside the tradeable window of
        // its own session, and the exit must exist.
        let Some(forced) = exits.get(entry).copied().flatten() else {
            out.too_late = out.too_late.saturating_add(1);
            continue;
        };
        let wanted = entry.saturating_add(h);
        let exit = wanted.min(forced);
        if exit <= entry {
            out.too_late = out.too_late.saturating_add(1);
            continue;
        }
        let Some(trade) = round_trip(bars, signal, entry, exit, exit < wanted, direction) else {
            out.too_late = out.too_late.saturating_add(1);
            continue;
        };
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
    let e = bars.get(entry_bar)?;
    let x = bars.get(exit_bar)?;

    // BEST: both legs at the bar's open, through the same fill rule as the
    // worst case so the two differ only in the bar and never in the model.
    let best_fills = worst_case_fills(
        FillBar::flat(paisa(e.open)).ok()?,
        FillBar::flat(paisa(x.open)).ok()?,
        direction,
    )
    .ok()?;

    // WORST: the adverse extreme of each bar. `Bar::new` takes (high, low) and
    // refuses an inverted or sub-tick bar, which is a corrupt candle rather than
    // a tradeable one.
    let worst_fills = worst_case_fills(
        FillBar::new(paisa(e.high), paisa(e.low)).ok()?,
        FillBar::new(paisa(x.high), paisa(x.low)).ok()?,
        direction,
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

/// For each bar, the last bar of its own session that a fill can still land in.
///
/// One backward pass, so every bar gets its answer in O(1) amortised rather than
/// searching its session. `None` where the bar is at or past the square-off, or
/// where the session's bars simply run out before it — see
/// [`crate::outcome`]'s note on why the end of the data is not a square-off.
///
/// UNVERIFIED as a measured figure, and no bench row covers it. What is claimed
/// is the SHAPE: the loop body is a fixed number of integer operations and one
/// `Option` copy, and the answer for bar `i` is inherited from bar `i + 1`
/// whenever they share a day, so nothing re-walks a session. A forward search
/// per bar would be O(H) with `H` caller-supplied — constant only by accident,
/// which is the kind of bound `CLAUDE.md` §3 rule 6 asks to be labelled rather
/// than asserted.
fn forced_exits(bars: &[Candle]) -> Vec<Option<usize>> {
    let stamps: Vec<(i64, i64)> = bars
        .iter()
        .map(|b| (indicators::ist_day(b.ts_micros), minute_of_day(b.ts_micros)))
        .collect();

    let mut out: Vec<Option<usize>> = vec![None; bars.len()];
    for i in (0..bars.len()).rev() {
        let Some(&(day, minute)) = stamps.get(i) else {
            continue;
        };
        let inherited = i
            .checked_add(1)
            .filter(|&j| stamps.get(j).is_some_and(|&(d, _)| d == day))
            .and_then(|j| out.get(j).copied().flatten());
        let mine = if minute <= LAST_FILL_MINUTE {
            Some(i)
        } else {
            None
        };
        if let Some(slot) = out.get_mut(i) {
            *slot = inherited.or(mine);
        }
    }
    out
}

/// The last bar whose interval ends at or before the 15:10 square-off.
///
/// A bar is stamped at its OPEN, so the bar stamped 15:09 covers 15:09–15:10 and
/// its close IS the 15:10 price. The bar stamped 15:10 closes at 15:11, a minute
/// after the position is already gone. The same constant, and the same reason,
/// as [`crate::outcome`] — measured at a 36% error on fourteen observations per
/// session when it was wrong.
const LAST_FILL_MINUTE: i64 = 15 * 60 + 10 - 1;

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
    use super::{Trades, walk};
    use crate::outcome::Horizon;
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
        // RULE 1. Every exit is inside the entry's own session and at or before
        // the bar whose close is 15:10.
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
            let entry_ts = bars.get(trade.entry_bar).map_or(0, |b| b.ts_micros);
            let exit_ts = bars.get(trade.exit_bar).map_or(0, |b| b.ts_micros);
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
}
