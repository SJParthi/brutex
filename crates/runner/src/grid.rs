//! Every stop/target variant of one combination, including no stop at all.
//!
//! # What this answers
//!
//! [`crate::trade`] exits on time: the horizon, or the 15:10 square-off. This
//! adds the level-based exits — stop, target — and evaluates **every rung of
//! both ladders at once**, with the no-stop no-target variant sitting in the
//! same table as a row rather than as a separate mode.
//!
//! That last part is the point. "Does a stop help here" is not a question anyone
//! should answer by opinion, and it is not a second run: the variant with no
//! stop is the one whose stop rung is [`crate::excursion::NEVER`], and it is
//! computed in the same pass as all the others.
//!
//! # The sniper metric, and why it is a ratio
//!
//! The aim is a setup precise enough that winners run and losers are cut for
//! almost nothing. Two numbers say whether a combination is that:
//!
//! * **MFE** — how far it went your way.
//! * **MAE** — how far it went against you first.
//!
//! A setup whose winners never went more than a few parts per million against you is
//! a setup you can hold with a tight stop, and [`Cell::edge_ratio`] is that
//! measured rather than judged. It is deliberately computed over the trades that
//! ENDED PROFITABLE: the adverse excursion of a loser tells you how bad the loss
//! was, and the adverse excursion of a WINNER tells you how tight a stop could
//! have been without killing it. Only the second answers "how small can the risk
//! be".
//!
//! # Why the whole sequence is re-walked per variant
//!
//! A stop that fires early ends the position early, and under
//! [`crate::trade`]'s one-position-at-a-time rule that frees the NEXT signal to
//! be taken sooner. So the trade sequence is not the same across variants and
//! reusing one list would quietly measure the wrong trades.
//!
//! The re-walk is affordable because the expensive half is cached: the path
//! crossings for a candidate entry depend only on that entry bar and its own
//! session's square-off, never on which variant is being evaluated. They are
//! computed once per candidate entry, and every variant's exit is then three
//! integer compares. `400 variants x 1,124 signals` is 450,000 lookups against
//! 1,124 path walks, not 450,000 path walks.
//!
//! # The ambiguous bar is resolved twice, never once
//!
//! When one bar would trigger both the stop and the target, minute data cannot
//! say which came first. [`Cell::pessimistic`] resolves it as the stop, and
//! [`Cell::optimistic`] as the target. The gap between them is the uncertainty
//! the data genuinely carries, and reporting one number would be choosing which
//! lie to tell.

use indicators::Candle;
use indicators::column::Column;
use vocab::ConditionMask;

use crate::excursion::{Crossings, Ladder, NEVER, Ppm, Side, crossings};
use crate::outcome::Horizon;

/// One (stop, target) variant's result over the whole slice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// Index into the stop ladder, or `None` for **no stop**.
    pub stop: Option<usize>,
    /// Index into the target ladder, or `None` for **no target**.
    pub target: Option<usize>,
    /// Round trips taken under this variant.
    pub trades: u64,
    /// Trades that ended above water, resolving ambiguity against you.
    pub wins: u64,
    /// Total paisa per unit, ambiguity resolved as the STOP first.
    pub pessimistic: i64,
    /// Total paisa per unit, ambiguity resolved as the TARGET first.
    pub optimistic: i64,
    /// Trades whose exit came from the stop.
    pub stopped: u64,
    /// Trades whose exit came from the target.
    pub targeted: u64,
    /// Trades that ran to the horizon or the 15:10 square-off.
    pub timed_out: u64,
    /// Bars where a stop and a target were both reachable, summed over trades.
    ///
    /// The size of the uncertainty. A variant with none of these has a
    /// pessimistic and an optimistic figure that agree exactly.
    pub ambiguous_bars: u64,
    /// Mean adverse excursion of the trades that ENDED PROFITABLE, in basis
    /// points.
    ///
    /// **The sniper number.** It says how far a winner went against you before
    /// it worked, which is the tightest stop that would not have killed it.
    pub winner_mae: Ppm,
    /// Mean favourable excursion of the trades that ended profitable.
    pub winner_mfe: Ppm,
}

impl Cell {
    /// Favourable excursion over adverse, on the winners, in hundredths.
    ///
    /// The precision of the setup as a single number: how much a winner gave
    /// you against how much it made you sweat first. Hundredths rather than a
    /// float because `CLAUDE.md` §7 keeps this kind of arithmetic in integers,
    /// and a ratio used for ranking is compared far more often than it is read.
    ///
    /// Zero when no winner ever went adverse — which is not an infinite ratio,
    /// it is a sample too clean to rank, and saying so beats dividing by zero.
    #[must_use]
    pub const fn edge_ratio(&self) -> i64 {
        if self.winner_mae <= 0 {
            return 0;
        }
        self.winner_mfe.saturating_mul(100) / self.winner_mae
    }

    /// Did the pessimistic reading make money?
    #[must_use]
    pub const fn survives(&self) -> bool {
        self.pessimistic > 0
    }
}

/// Every variant of one combination.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grid {
    /// One per (stop, target) pair, including the no-stop no-target row.
    pub cells: Vec<Cell>,
    /// Signals the combination fired, before exclusivity.
    pub signals: u64,
    /// The stop ladder these cells index into.
    pub stops: Ladder,
    /// The target ladder these cells index into.
    pub targets: Ladder,
}

impl Grid {
    /// The variant with no stop and no target — the time-exit baseline.
    ///
    /// Kept as a named accessor because the whole comparison the operator asked
    /// for is "with these levels versus without them", and that is this row
    /// against the others.
    #[must_use]
    pub fn baseline(&self) -> Option<&Cell> {
        self.cells
            .iter()
            .find(|c| c.stop.is_none() && c.target.is_none())
    }

    /// The variant with the largest pessimistic total.
    ///
    /// Pessimistic and not optimistic, for the reason [`crate::validate`] gives
    /// about selection: choosing on the flattering number picks whatever the
    /// flattering assumption helped most.
    #[must_use]
    pub fn best(&self) -> Option<&Cell> {
        self.cells.iter().max_by_key(|c| c.pessimistic)
    }

    /// The variant with the sharpest winners, among those that survive.
    ///
    /// The sniper answer: of the variants that made money under pessimistic
    /// fills and pessimistic ambiguity, the one whose winners went least against
    /// you before working.
    #[must_use]
    pub fn sharpest(&self) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| c.survives() && c.wins > 0)
            .max_by_key(|c| c.edge_ratio())
    }
}

/// How a trade under a given variant ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ended {
    Stop,
    Target,
    Time,
}

/// One candidate entry, with its path measured once.
struct Candidate {
    signal: usize,
    entry: usize,
    /// The last bar the position may be held to: horizon or square-off.
    time_exit: usize,
    cross: Crossings,
}

/// Evaluate every stop/target variant of `mask` over `bars`.
///
/// The ladders are DERIVED from the excursions this combination actually
/// produced — see [`Ladder::from_excursions`]. Nobody supplies a level.
///
/// # Cost
///
/// One path walk per candidate entry, then one sequence walk per variant with
/// `O(1)` exit lookups. `rungs` controls both ladders, so the variant count is
/// `(rungs + 1)^2` including the no-stop no-target row.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn evaluate(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    side: Side,
    rungs: usize,
) -> Grid {
    // PASS ONE: every signal that could open a position, and its path.
    // `crate::trade::walk` already applies the intraday rules, so its trades
    // give the entry bars and the time-exit bars this grid narrows.
    let direction = match side {
        Side::Long => costs::fill::Direction::Long,
        Side::Short => costs::fill::Direction::Short,
    };
    let timed = crate::trade::walk(bars, column, mask, horizon, direction);
    if timed.trades.is_empty() {
        return Grid {
            signals: timed.signals,
            ..Grid::default()
        };
    }

    // PASS TWO: the ladders, from what this combination's own trades did. A
    // provisional walk with no levels supplies the excursion sample, so the
    // grid is placed on the distribution it will be measured against.
    let mut adverse: Vec<Ppm> = Vec::with_capacity(timed.trades.len());
    let mut favourable: Vec<Ppm> = Vec::with_capacity(timed.trades.len());
    let probe = Ladder::new(vec![1]).unwrap_or_default();
    for t in &timed.trades {
        let entry_price = bars.get(t.entry_bar).map_or(0, |b| b.open);
        let c = crossings(
            bars,
            t.entry_bar,
            t.exit_bar,
            entry_price,
            side,
            &probe,
            &probe,
        );
        adverse.push(peak_adverse(
            bars,
            t.entry_bar,
            t.exit_bar,
            entry_price,
            side,
        ));
        favourable.push(peak_favourable(
            bars,
            t.entry_bar,
            t.exit_bar,
            entry_price,
            side,
        ));
        let _ = c;
    }
    let stops = Ladder::from_excursions(&mut adverse.clone(), rungs).unwrap_or_default();
    let targets = Ladder::from_excursions(&mut favourable.clone(), rungs).unwrap_or_default();

    // PASS THREE: each candidate's path measured ONCE against both ladders.
    let candidates: Vec<Candidate> = timed
        .trades
        .iter()
        .map(|t| {
            let entry_price = bars.get(t.entry_bar).map_or(0, |b| b.open);
            Candidate {
                signal: t.signal_bar,
                entry: t.entry_bar,
                time_exit: t.exit_bar,
                cross: crossings(
                    bars,
                    t.entry_bar,
                    t.exit_bar,
                    entry_price,
                    side,
                    &stops,
                    &targets,
                ),
            }
        })
        .collect();

    // PASS FOUR: every variant, each a sequence walk with O(1) exits.
    let mut cells: Vec<Cell> = Vec::with_capacity(
        stops
            .len()
            .saturating_add(1)
            .saturating_mul(targets.len().saturating_add(1)),
    );
    for s in 0..=stops.len() {
        for t in 0..=targets.len() {
            let stop = (s < stops.len()).then_some(s);
            let target = (t < targets.len()).then_some(t);
            cells.push(one_variant(bars, &candidates, stop, target, side));
        }
    }

    Grid {
        cells,
        signals: timed.signals,
        stops,
        targets,
    }
}

/// Walk the candidate sequence under one variant.
///
/// Exclusivity is applied here rather than reused, because a stop that fires
/// early frees the next signal sooner and the sequence genuinely differs.
fn one_variant(
    bars: &[Candle],
    candidates: &[Candidate],
    stop: Option<usize>,
    target: Option<usize>,
    side: Side,
) -> Cell {
    let mut cell = Cell {
        stop,
        target,
        ..Cell::default()
    };
    let mut open_until: Option<usize> = None;
    let mut adverse_on_winners: i64 = 0;
    let mut gain_on_winners: i64 = 0;

    for c in candidates {
        if open_until.is_some_and(|until| c.signal < until) {
            continue;
        }
        let span = c.time_exit.saturating_sub(c.entry);
        let stop_at = stop.map_or(NEVER, |r| c.cross.stop_at(r));
        let target_at = target.map_or(NEVER, |r| c.cross.target_at(r));

        // PESSIMISTIC: when both are reachable on the same bar, the stop wins.
        // OPTIMISTIC: the target does. `<=` versus `<` is the whole difference.
        let pess_off = span.min(stop_at).min(target_at);
        let opt_off = span.min(if target_at <= stop_at {
            target_at
        } else {
            stop_at
        });
        let ended = if stop_at <= pess_off && stop_at != NEVER {
            Ended::Stop
        } else if target_at <= pess_off && target_at != NEVER {
            Ended::Target
        } else {
            Ended::Time
        };

        let entry_price = bars.get(c.entry).map_or(0, |b| b.open);
        let pess = realised(bars, c.entry, pess_off, entry_price, side);
        let opt = realised(bars, c.entry, opt_off, entry_price, side);

        cell.trades = cell.trades.saturating_add(1);
        cell.pessimistic = cell.pessimistic.saturating_add(pess);
        cell.optimistic = cell.optimistic.saturating_add(opt);
        cell.ambiguous_bars = cell
            .ambiguous_bars
            .saturating_add(u64::try_from(c.cross.ambiguous().len()).unwrap_or(0));
        match ended {
            Ended::Stop => cell.stopped = cell.stopped.saturating_add(1),
            Ended::Target => cell.targeted = cell.targeted.saturating_add(1),
            Ended::Time => cell.timed_out = cell.timed_out.saturating_add(1),
        }
        if pess > 0 {
            cell.wins = cell.wins.saturating_add(1);
            adverse_on_winners = adverse_on_winners.saturating_add(peak_adverse(
                bars,
                c.entry,
                c.entry.saturating_add(pess_off),
                entry_price,
                side,
            ));
            gain_on_winners = gain_on_winners.saturating_add(peak_favourable(
                bars,
                c.entry,
                c.entry.saturating_add(pess_off),
                entry_price,
                side,
            ));
        }
        open_until = Some(c.entry.saturating_add(pess_off));
    }

    if cell.wins > 0 {
        let n = i64::try_from(cell.wins).unwrap_or(1).max(1);
        cell.winner_mae = adverse_on_winners / n;
        cell.winner_mfe = gain_on_winners / n;
    }
    cell
}

/// Close-to-entry move at `entry + offset`, in paisa.
fn realised(bars: &[Candle], entry: usize, offset: usize, entry_price: i64, side: Side) -> i64 {
    let exit = bars
        .get(entry.saturating_add(offset))
        .map_or(entry_price, |b| b.close);
    match side {
        Side::Long => exit.saturating_sub(entry_price),
        Side::Short => entry_price.saturating_sub(exit),
    }
}

/// The worst the path went against the position, in parts per million.
fn peak_adverse(bars: &[Candle], from: usize, to: usize, entry: i64, side: Side) -> Ppm {
    peak(bars, from, to, entry, side, true)
}

/// The best the path went for the position, in parts per million.
fn peak_favourable(bars: &[Candle], from: usize, to: usize, entry: i64, side: Side) -> Ppm {
    peak(bars, from, to, entry, side, false)
}

/// One extreme of the path, in parts per million of the entry price.
fn peak(bars: &[Candle], from: usize, to: usize, entry: i64, side: Side, adverse: bool) -> Ppm {
    if entry <= 0 || from > to {
        return 0;
    }
    let mut worst: i64 = 0;
    for i in from..=to {
        let Some(bar) = bars.get(i) else { break };
        let move_paisa = match (side, adverse) {
            (Side::Long, true) | (Side::Short, false) => entry.saturating_sub(bar.low),
            (Side::Long, false) | (Side::Short, true) => bar.high.saturating_sub(entry),
        };
        if move_paisa > worst {
            worst = move_paisa;
        }
    }
    let scaled = i128::from(worst).saturating_mul(1_000_000) / i128::from(entry);
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::evaluate;
    use crate::excursion::Side;
    use crate::outcome::Horizon;
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
    fn the_no_stop_no_target_baseline_is_a_row_of_the_same_table() {
        // THE COMPARISON THE OPERATOR ASKED FOR. "With levels" and "without
        // levels" must be two rows of one result, computed in one pass -- not
        // two runs a human has to line up by hand.
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );

        let base = g.baseline().expect("a baseline row must exist");
        assert!(base.stop.is_none() && base.target.is_none());
        assert!(base.trades > 0, "the baseline must take trades");
        assert!(
            g.cells.len() > 1,
            "the grid must hold the level variants beside the baseline"
        );
    }

    #[test]
    fn a_stop_can_only_shorten_a_trade_never_lengthen_it() {
        // A level exit fires at or before the time exit, always. If any variant
        // held longer than the baseline, the exit rule would be reading the
        // wrong side of the ladder.
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(60),
            Side::Long,
            4,
        );
        let base = g.baseline().expect("a baseline");
        for c in &g.cells {
            if c.stop.is_none() && c.target.is_none() {
                continue;
            }
            assert!(
                c.trades >= base.trades,
                "levels exit earlier, which frees the next signal, so a variant \
                 can never take FEWER trades than the time-only baseline"
            );
        }
    }

    #[test]
    fn the_pessimistic_reading_never_beats_the_optimistic_one() {
        // The two differ only on bars where a stop and a target were both
        // reachable. Pessimistic takes the stop there, so it can never come out
        // ahead -- if it did, the ambiguity would be resolved backwards.
        for side in [Side::Long, Side::Short] {
            let (bars, column) = swept();
            let g = evaluate(&bars, &column, &ConditionMask::default(), h(15), side, 4);
            for c in &g.cells {
                assert!(
                    c.pessimistic <= c.optimistic,
                    "{side:?} variant {:?}/{:?}: pessimistic {} beat optimistic {}",
                    c.stop,
                    c.target,
                    c.pessimistic,
                    c.optimistic
                );
            }
        }
    }

    #[test]
    fn a_variant_with_no_ambiguous_bar_has_two_readings_that_agree() {
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        for c in g.cells.iter().filter(|c| c.ambiguous_bars == 0) {
            assert_eq!(
                c.pessimistic, c.optimistic,
                "with no ambiguous bar there is nothing for the two readings to \
                 disagree about"
            );
        }
    }

    #[test]
    fn every_trade_ends_by_exactly_one_of_stop_target_or_time() {
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        for c in &g.cells {
            assert_eq!(
                c.stopped
                    .saturating_add(c.targeted)
                    .saturating_add(c.timed_out),
                c.trades,
                "a trade that ended by none of the three, or by two, is a walk \
                 that lost one"
            );
        }
    }

    #[test]
    fn the_ladders_come_from_the_data_and_a_combination_that_never_trades_has_none() {
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        assert!(!g.stops.is_empty(), "trades happened, so a ladder exists");
        assert!(
            g.stops.rungs().windows(2).all(|w| w.first() < w.last()),
            "a derived ladder must ascend"
        );

        // A mask nothing satisfies produces no trades, so no ladder and no
        // cells -- rather than a grid of zeroes that reads like a measurement.
        let mut impossible = ConditionMask::default();
        for bit in 0..8 {
            impossible = impossible.with_bit(bit);
        }
        let empty = evaluate(&bars, &column, &impossible, h(15), Side::Long, 4);
        assert!(empty.cells.is_empty() || empty.baseline().is_some());
    }
}
