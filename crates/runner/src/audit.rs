//! The audit surface: everything the newer stages measured, rendered so a human
//! can read it without opening the code.
//!
//! # The gap this closes
//!
//! [`crate::report`] renders the SWEEP, the BARS, the LADDER, the SIGNIFICANCE
//! bar and the FINDINGS. It was written before any of the execution stages
//! existed and it shows **none** of them: not the trades, not the exit grid, not
//! the sniper measure, not the walk-forward, not the overfitting probability,
//! not the bootstrap.
//!
//! So the engine could measure all of it and an operator could see none of it.
//! `CLAUDE.md` §4 bans a result that hides a failure behind a success, and a
//! number nobody can read is not far off — a walk-forward that came out negative
//! and a walk-forward that never ran looked identical from outside.
//!
//! # Every section states what it CANNOT tell you
//!
//! That is not modesty, it is the difference between an audit and a brochure.
//! A reader who takes "PBO 0.08" as "this is profitable" has been misled by a
//! true number, and the only defence is to put the limit beside the figure
//! rather than in a document they will not open.
//!
//! # Plain text, and no dependency
//!
//! The same choice [`crate::report`] makes: a sweep is audited from a terminal
//! or a log file, and both of those are text. A browser view belongs under
//! `web/` and this crate cannot reach it.
//!
//! # Cost
//!
//! One `String` per render. It touches no bar and re-measures nothing — every
//! figure was already counted by the stage that produced it. `O(cells + folds)`,
//! at a once-per-run boundary.
//!
//! UNVERIFIED as a measured figure: no bench row covers this yet.

use core::fmt::Write as _;

use crate::bootstrap::Verdict;
use crate::grid::{Cell, Grid};
use crate::pbo::Pbo;
use crate::trade::Trades;
use crate::validate::Validated;

/// Column width for the label side of every row.
const LABEL: usize = 36;

/// One line: a label, a value, and a note.
fn row(out: &mut String, label: &str, value: &str, note: &str) {
    let _ = writeln!(out, "  {label:<LABEL$}{value:>16}  {note}");
}

/// A ratio in tenths of a percent, computed in integers.
///
/// `CLAUDE.md` §7 forbids a float here for the same reason [`crate::report`]
/// gives: an audit that disagreed with the counters it renders would be worse
/// than no audit.
fn permille(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "-".to_owned();
    }
    let tenths = part.saturating_mul(1_000) / whole;
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// Everything, in one call.
///
/// # Why a single entry point exists
///
/// The sections below are separately callable because a caller may hold only
/// some of the inputs. But an operator asking "how did this run go" wants the
/// whole thing, and making them assemble six calls in the right order is how a
/// section quietly stops being printed — the failure this module was written to
/// remove in the first place.
///
/// Every argument is optional and an absent one is REPORTED rather than
/// skipped. A run with no walk-forward and a run whose walk-forward was omitted
/// from the render must not look the same.
///
/// # What is NOT in these figures, for a spot run
///
/// Nothing. On a spot index there is no brokerage, no STT, no stamp duty and no
/// GST, because a spot index is not tradeable and no order is placed —
/// `costs::scope::is_cost_free` returns true for `IndexSpot` and says so. The
/// only cost that exists is slippage, and it is applied: two ticks per round
/// trip through `costs::fill::worst_case_fills`.
///
/// **That changes when options land.** An option leg carries the whole charge
/// stack, and the figures here would then be gross of it until
/// `costs::trip::price` is wired. This paragraph is the record of which of
/// those two worlds a reader is in.
#[must_use]
pub fn render(
    taken: Option<&Trades>,
    exits: Option<&Grid>,
    folds: Option<&Validated>,
    overfit: Option<&Pbo>,
    boot: Option<(Option<&Verdict>, Option<&Verdict>, usize)>,
    rows: usize,
) -> String {
    let mut out = String::with_capacity(4_096);
    let _ = writeln!(out, "AUDIT");
    let _ = writeln!(
        out,
        "  INDEX SPOT run. Slippage IS in these figures -- two ticks a round\n  \
         trip. There is no brokerage, STT, stamp or GST, because an INDEX is\n  \
         not tradeable: no order is placed, so nothing charges for one.\n\n  \
         THIS IS SCOPED TO AN INDEX AND TO NOTHING ELSE. A STOCK spot IS\n  \
         tradeable -- you buy real shares -- so brokerage, STT, stamp, the\n  \
         exchange charge and GST all apply there, and so do they on options.\n  \
         `costs::scope::Segment` has no equity-spot variant today, so that\n  \
         charge path does not exist yet and this header would be WRONG the\n  \
         day a stock is swept."
    );
    let _ = writeln!(out);

    match taken {
        Some(x) => trades(&mut out, x),
        None => absent(&mut out, "TRADES"),
    }
    match exits {
        Some(x) => grid(&mut out, x, rows),
        None => absent(&mut out, "EXIT GRID"),
    }
    match folds {
        Some(x) => walk_forward(&mut out, x),
        None => absent(&mut out, "WALK-FORWARD"),
    }
    match overfit {
        Some(x) => overfitting(&mut out, x),
        None => absent(&mut out, "OVERFITTING"),
    }
    match boot {
        Some((rc, spa, named)) => bootstrap(&mut out, rc, spa, named),
        None => absent(&mut out, "BOOTSTRAP"),
    }
    out
}

/// A section whose input the caller did not hold.
///
/// Printed rather than skipped. A run that HAD no walk-forward and a render
/// that was not GIVEN one are different facts, and a silently missing section
/// reads as the first.
fn absent(out: &mut String, section: &str) {
    let _ = writeln!(out, "{section}");
    let _ = writeln!(
        out,
        "  NOT SUPPLIED to this render. Absent from the report is not the same \
         as absent from the run."
    );
    let _ = writeln!(out);
}

/// How the signals of one combination became trades.
///
/// # What this section is for
///
/// `signals` is the number [`crate::outcome::edge`] would have used as its `n`.
/// `trades` is what a trader holding one position actually took. The ratio
/// between them is how much of the old figure was the same position counted
/// again, and on the measured fixture it was **16.5x** at the default horizon.
pub fn trades(out: &mut String, t: &Trades) {
    let _ = writeln!(out, "TRADES");
    row(
        out,
        "signals fired",
        &t.signals.to_string(),
        "bars the mask hit",
    );
    row(
        out,
        "round trips taken",
        &t.count().to_string(),
        &permille(t.count(), t.signals),
    );
    row(
        out,
        "  blocked, position already open",
        &t.while_open.to_string(),
        "one position at a time",
    );
    row(
        out,
        "  too late to enter",
        &t.too_late.to_string(),
        "at or past the 15:10 square-off",
    );
    if !t.reconciles() {
        let _ = writeln!(
            out,
            "  RECONCILIATION FAILED: trades + blocked + too-late does not equal \
             signals. A signal was dropped without being counted."
        );
    }
    let _ = writeln!(out);
}

/// The exit grid: with levels against without them.
///
/// # The comparison, as a table rather than two runs
///
/// The first row is the baseline — no stop, no target, no trail — and every
/// other row is a level variant. They were computed in one pass over the same
/// trades, so the comparison is exact rather than two runs a reader lines up by
/// hand.
///
/// `keep` bounds the rows printed. Whatever it drops is stated, because a table
/// that quietly showed the best twelve of a hundred would read as the whole
/// grid.
pub fn grid(out: &mut String, g: &Grid, keep: usize) {
    let _ = writeln!(out, "EXIT GRID");
    if g.cells.is_empty() {
        row(out, "variants", "-", "the combination took no trades");
        let _ = writeln!(out);
        return;
    }
    row(out, "variants evaluated", &g.cells.len().to_string(), "");
    row(
        out,
        "stop rungs / target rungs",
        &format!("{} / {}", g.stops.len(), g.targets.len()),
        "derived from this combination's own excursions",
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {:<10}{:>8}{:>8}{:>8}{:>10}{:>10}{:>12}{:>10}",
        "exit", "trades", "won", "stopped", "worst", "unknown", "winner MAE", "ratio"
    );

    // The baseline first, always, then the rest by pessimistic total. A reader
    // comparing "with levels" against "without" must not have to hunt for the
    // row that is the comparison.
    let mut ordered: Vec<&Cell> = g.cells.iter().collect();
    ordered.sort_by_key(|c| {
        let base = c.stop.is_none() && c.target.is_none() && c.trail.is_none();
        // `Reverse`, NOT `-c.pessimistic`, AND THE DIFFERENCE IS AN ABORT.
        //
        // `crate::grid` accumulates this field with `saturating_add`, whose floor
        // is exactly `i64::MIN` -- so the one value the accumulator is DESIGNED to
        // produce when a variant loses without bound is the one value that has no
        // positive counterpart. Under `overflow-checks = true` the negation panics,
        // and the release profile sets `panic = "abort"`, so the process dies with
        // no message, no partial report and no name for what happened. Sorting a
        // results table is not a thing that should be able to kill the process.
        //
        // `Reverse` orders descending with no arithmetic at all, so there is
        // nothing left to overflow. `costs::moneyness` reaches for `checked_neg`
        // at the same hazard; here the negation can be deleted outright rather
        // than guarded, which is the better of the two.
        (!base, core::cmp::Reverse(c.pessimistic))
    });

    let shown = ordered.len().min(keep);
    for c in ordered.iter().take(keep) {
        let name = match (c.stop, c.target, c.trail) {
            (None, None, None) => "NONE".to_owned(),
            (s, t, r) => format!(
                "{}/{}/{}",
                s.map_or_else(|| "-".to_owned(), |i| i.to_string()),
                t.map_or_else(|| "-".to_owned(), |i| i.to_string()),
                r.map_or_else(|| "-".to_owned(), |i| i.to_string())
            ),
        };
        let _ = writeln!(
            out,
            "  {:<10}{:>8}{:>8}{:>8}{:>10}{:>10}{:>12}{:>10}",
            name,
            c.trades,
            c.wins,
            c.stopped,
            c.pessimistic,
            c.uncertainty(),
            c.winner_mae,
            c.edge_ratio()
        );
    }
    if ordered.len() > shown {
        let _ = writeln!(
            out,
            "  ... {} further variant(s) NOT SHOWN. The grid is complete; this \
             table is not.",
            ordered.len().saturating_sub(shown)
        );
    }
    let _ = writeln!(out);

    // THE UNCERTAINTY COLUMN IS NOT AN OPTIMISTIC ESTIMATE. It is the width
    // of what minute bars cannot settle -- the gap between resolving an
    // ambiguous bar as the stop and as the target. Two variants with the same
    // worst-case total and different spreads are not equally trustworthy.
    let unresolved = g
        .cells
        .iter()
        .filter(|c| c.depends_on_unknowable_ordering())
        .count();
    if unresolved > 0 {
        let _ = writeln!(
            out,
            "  {unresolved} variant(s) depend on intra-bar ordering this data \
             cannot settle. The `unknown` column is how much -- it is a
             MEASUREMENT ERROR, not an upside. Only second-level data closes it."
        );
    } else {
        let _ = writeln!(
            out,
            "  No variant depends on intra-bar ordering: every exit was
           unambiguous, so second-level data would change nothing here."
        );
    }
    let _ = writeln!(out);

    // THE SNIPER LINE. Of the variants that survived pessimistic fills AND
    // pessimistic ambiguity, the one whose winners went least against you.
    if let Some(sharp) = g.sharpest() {
        let _ = writeln!(
            out,
            "  SHARPEST: winners went {} ppm against before working, and {} ppm \
             for. Ratio {}. That adverse figure is the tightest stop that would \
             not have killed a winner.",
            sharp.winner_mae,
            sharp.winner_mfe,
            sharp.edge_ratio()
        );
    } else {
        let _ = writeln!(
            out,
            "  NO SHARPEST VARIANT: nothing survived pessimistic fills, so there \
             is no setup to call precise. That is a finding, not a gap."
        );
    }
    let _ = writeln!(out);
}

/// Walk-forward: what was chosen on the past, and what it did on the future.
pub fn walk_forward(out: &mut String, v: &Validated) {
    let _ = writeln!(out, "WALK-FORWARD");
    if v.folds.is_empty() {
        row(
            out,
            "folds",
            "-",
            "the slice was too short to split, so nothing was validated",
        );
        let _ = writeln!(out);
        return;
    }
    row(out, "folds", &v.folds.len().to_string(), "");
    row(
        out,
        "  that chose a combination",
        &v.decided().to_string(),
        "",
    );
    row(
        out,
        "  still positive out of sample",
        &v.held_up().to_string(),
        "under pessimistic fills",
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {:<6}{:>10}{:>10}{:>12}{:>14}{:>14}{:>15}",
        "fold", "train", "test", "candidates", "in-sample", "oos no exit", "oos with exit"
    );
    for f in &v.folds {
        let _ = writeln!(
            out,
            "  {:<6}{:>10}{:>10}{:>12}{:>14}{:>14}{:>15}",
            f.index,
            f.train_bars,
            f.test_bars,
            f.considered,
            f.in_sample.worst,
            f.out_of_sample.worst,
            // THE COLUMN `held_up` ACTUALLY JUDGES, and it was not on this table.
            //
            // `held_up` counts a fold as holding up when `out_of_sample_exit` is
            // above zero, falling back to the no-exit walk only when the fold
            // chose no exit. The table printed the NO-EXIT total under a heading
            // reading "out-of-sample", so a run could report "still positive out
            // of sample: 4" above four NEGATIVE figures and look like it was
            // lying. Neither number was wrong; they are different quantities, and
            // nothing said so.
            //
            // Both are on the table now. A dash means the fold chose no exit
            // levels, which is when the two columns are the same measurement.
            f.out_of_sample_exit
                .map_or_else(|| "--".to_owned(), |v| v.to_string())
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  A fold is ONE DRAW. Holding up out of sample on one window is one \
         comparison survived, not a proof -- see OVERFITTING below."
    );
    let _ = writeln!(out);
}

/// The probability that the selection procedure is fooling itself.
pub fn overfitting(out: &mut String, p: &Pbo) {
    let _ = writeln!(out, "OVERFITTING");
    match p.probability() {
        None => {
            row(
                out,
                "PBO",
                "NOT MEASURED",
                "no fold could be ranked -- not the same as zero",
            );
            if p.unrankable > 0 {
                row(
                    out,
                    "  folds with too few candidates",
                    &p.unrankable.to_string(),
                    "one candidate is not a choice",
                );
            }
        }
        Some(ppm) => {
            row(
                out,
                "PBO",
                &format!("{}.{}%", ppm / 10_000, (ppm / 1_000) % 10),
                if p.is_noise() {
                    "AT OR ABOVE 50% -- the selection is a coin flip"
                } else {
                    "below 50% -- selection carries information"
                },
            );
            row(out, "  folds ranked", &p.folds.to_string(), "");
            row(
                out,
                "  winner below median out of sample",
                &p.overfit_folds.to_string(),
                "",
            );
            row(
                out,
                "  median placement",
                &format!(
                    "{}.{}%",
                    p.median_placement / 10_000,
                    (p.median_placement / 1_000) % 10
                ),
                "0% = still best, 100% = worst",
            );
            if p.unrankable > 0 {
                row(
                    out,
                    "  folds NOT ranked",
                    &p.unrankable.to_string(),
                    "excluded from the denominator",
                );
            }
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  A LOW PBO DOES NOT MEAN PROFITABLE. It means the SELECTION is not \
         noise-fitting. A procedure can reliably pick the best of a hundred \
         worthless combinations and score perfectly while losing on every one."
    );
    let _ = writeln!(out);
}

/// The three bootstrap tests, side by side.
pub fn bootstrap(out: &mut String, rc: Option<&Verdict>, spa: Option<&Verdict>, named: usize) {
    let _ = writeln!(out, "BOOTSTRAP");
    let _ = writeln!(
        out,
        "  {:<28}{:>12}{:>12}{:>10}",
        "test", "statistic", "p-value", "clears 5%"
    );
    for (name, v) in [("White's Reality Check", rc), ("Hansen's SPA", spa)] {
        match v {
            Some(x) => {
                let _ = writeln!(
                    out,
                    "  {:<28}{:>12.3}{:>12.4}{:>10}",
                    name,
                    x.statistic,
                    x.p_value,
                    if x.clears() { "yes" } else { "NO" }
                );
            }
            None => {
                let _ = writeln!(out, "  {name:<28}{:>12}{:>12}{:>10}", "-", "REFUSED", "-");
            }
        }
    }
    let _ = writeln!(
        out,
        "  {:<28}{:>12}{:>12}{:>10}",
        "Romano-Wolf (named)", "-", "-", named
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  The first two ask whether ANYTHING here is real. Only Romano-Wolf \
         says WHICH, and it names {named}."
    );

    // WHAT THE SAMPLE SIZE WAS WORTH, BESIDE THE P-VALUE IT PRODUCED.
    //
    // A p-value keeps no record of how much evidence produced it, so the two
    // rows above render identically at three periods and at three hundred while
    // meaning entirely different things. `docs/06-limits.md` §77 measures the
    // gap -- 37.1% false positives at three periods against a nominal 5%.
    //
    // This crate picks no minimum-period floor: which floor is a number
    // `CLAUDE.md` §3 rule 1 forbids it inventing, with no charter source to take
    // it from. §3 rule 6 asks instead that the limit be stated where it will be
    // read, and a limit recorded only in a document is not stated to the person
    // looking at the verdict. So it prints here. D-0172.
    if let Some(v) = rc.or(spa) {
        let _ = writeln!(out);
        let _ = writeln!(out, "  sample: {}", v.calibration());
        if v.periods < 100 {
            let _ = writeln!(
                out,
                "  READ THE TWO ROWS ABOVE WITH THAT IN MIND -- at this sample \
                 size these tests fire falsely far more often than 5%."
            );
        }
    }
    let _ = writeln!(out);
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {

    /// THE VERDICT'S SAMPLE SIZE IS PRINTED WHERE IT WILL BE READ.
    ///
    /// A limit recorded only in `docs/06-limits.md` is not stated to the person
    /// looking at the p-value. At a short sample the render must say so beside
    /// the number, not somewhere else.
    #[test]
    fn a_short_sample_is_named_beside_the_p_value_it_produced() {
        let short = crate::bootstrap::Verdict {
            statistic: 3.0,
            p_value: 0.01,
            draws: 1_000,
            strategies: 2,
            periods: 3,
        };
        let mut out = String::new();
        bootstrap(&mut out, Some(&short), None, 1);
        assert!(out.contains("37.1%"), "the measured rate is shown:\n{out}");
        assert!(
            out.contains("READ THE TWO ROWS ABOVE"),
            "and a short sample is called out rather than left to the reader:\n{out}"
        );

        let long = crate::bootstrap::Verdict {
            periods: 400,
            ..short
        };
        let mut out = String::new();
        bootstrap(&mut out, Some(&long), None, 1);
        assert!(out.contains("nominal"), "a long sample says so:\n{out}");
        assert!(
            !out.contains("READ THE TWO ROWS ABOVE"),
            "and is NOT warned about -- a warning on every verdict is a warning \
             nobody reads:\n{out}"
        );
    }

    /// A LOSING VARIANT MUST NOT BE ABLE TO KILL THE PROCESS.
    ///
    /// `crate::grid` accumulates `Cell::pessimistic` with `saturating_add`, whose
    /// floor is exactly `i64::MIN`. This function used to sort on
    /// `-c.pessimistic`, and `i64::MIN` has no positive counterpart: under
    /// `overflow-checks = true` that negation panics, and the release profile sets
    /// `panic = "abort"`, so the process would die with no message and no partial
    /// report. The one value the accumulator is DESIGNED to produce on unbounded
    /// loss was the one value the sort key could not take.
    ///
    /// Sorting a results table is not a thing that should be able to abort a
    /// process, so the negation is gone rather than guarded.
    #[test]
    fn the_grid_table_orders_a_saturated_loss_without_negating_it() {
        let cell = |stop: Option<usize>, pess: i64| crate::grid::Cell {
            stop,
            trades: 1,
            pessimistic: pess,
            optimistic: pess,
            ..crate::grid::Cell::default()
        };
        let g = crate::grid::Grid {
            // The base row (no stop, no target, no trail) must still lead, and the
            // two rungs must still order best-first beneath it.
            cells: vec![cell(None, -5), cell(Some(0), i64::MIN), cell(Some(1), 100)],
            signals: 3,
            ..crate::grid::Grid::default()
        };

        let mut out = String::new();
        grid(&mut out, &g, 10);

        assert!(!out.is_empty(), "the table rendered rather than aborting");
        let best = out.find("100").expect("the profitable rung is shown");
        let worst = out
            .find(&i64::MIN.to_string())
            .expect("the saturated rung is shown rather than dropped");
        assert!(
            best < worst,
            "the better rung must sort above the saturated one:\n{out}"
        );
    }
    use super::{bootstrap, grid, overfitting, trades, walk_forward};
    use crate::pbo::{Placement, probability_of_overfitting};
    use crate::trade::{Trade, Trades};
    use crate::validate::Validated;

    /// The value column of a labelled row, so an assertion reads the number
    /// rather than any text that happens to contain it.
    ///
    /// Written because two assertions in `crate::report` were once satisfied by
    /// the wrong thing: `contains("NO")` is true of the string "NOT RECORDED".
    /// A test that passes for the wrong reason is worse than no test.
    fn cell(text: &str, label: &str) -> String {
        text.lines()
            .find(|l| l.trim_start().starts_with(label))
            .map(|l| {
                l.get(2 + super::LABEL..)
                    .unwrap_or("")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .unwrap_or_default()
    }

    #[test]
    fn the_trades_section_shows_how_much_of_the_signal_count_was_overlap() {
        let t = Trades {
            trades: vec![
                Trade {
                    signal_bar: 0,
                    entry_bar: 1,
                    exit_bar: 5,
                    best: 10,
                    worst: -5,
                    forced: false,
                };
                3
            ],
            signals: 50,
            while_open: 45,
            too_late: 2,
        };
        let mut out = String::new();
        trades(&mut out, &t);
        assert_eq!(cell(&out, "signals fired"), "50");
        assert_eq!(cell(&out, "round trips taken"), "3");
        assert!(
            out.contains("blocked, position already open"),
            "the overlap must be visible, not inferred"
        );
    }

    #[test]
    fn a_walk_that_lost_a_signal_says_so_rather_than_reconciling_silently() {
        // The counts deliberately do not add up. An audit that printed them
        // without noticing would report a walk that dropped a signal as a
        // healthy one.
        let t = Trades {
            trades: Vec::new(),
            signals: 100,
            while_open: 1,
            too_late: 1,
        };
        let mut out = String::new();
        trades(&mut out, &t);
        assert!(
            out.contains("RECONCILIATION FAILED"),
            "a walk whose counts do not sum must say so"
        );
    }

    #[test]
    fn an_unmeasured_pbo_is_not_rendered_as_zero() {
        // "Never overfit" and "never measured" are different facts, and zero
        // reads as the first. This is the assertion that stops an audit from
        // reporting a perfect score for a test that never ran.
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&[]));
        assert_eq!(cell(&out, "PBO"), "NOT");
        assert!(out.contains("not the same as zero"));
        assert!(
            !out.contains("0.0%"),
            "an unmeasured PBO must not render as a number at all"
        );
    }

    #[test]
    fn a_coin_flip_pbo_is_named_as_one() {
        let folds = [
            Placement {
                candidates: 3,
                winner_rank: 0,
            },
            Placement {
                candidates: 3,
                winner_rank: 2,
            },
        ];
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&folds));
        assert!(
            out.contains("coin flip"),
            "a PBO at or above half must be called what it is"
        );
    }

    #[test]
    fn every_section_states_what_it_cannot_tell_you() {
        // The difference between an audit and a brochure. A reader who takes a
        // true number for a claim it does not support has been misled by it,
        // and the limit has to sit beside the figure.
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&[]));
        assert!(out.contains("DOES NOT MEAN PROFITABLE"));

        let mut w = String::new();
        walk_forward(&mut w, &Validated::default());
        assert!(w.contains("nothing was validated"));

        let mut g = String::new();
        grid(&mut g, &crate::grid::Grid::default(), 10);
        assert!(g.contains("took no trades"));
    }

    #[test]
    fn one_call_renders_every_section_and_names_the_ones_it_was_not_given() {
        // A section that is silently missing reads as "the run did not do this".
        // A run that HAD no walk-forward and a render that was not GIVEN one are
        // different facts, and only one of them is a finding.
        let out = super::render(None, None, None, None, None, 10);
        for section in [
            "TRADES",
            "EXIT GRID",
            "WALK-FORWARD",
            "OVERFITTING",
            "BOOTSTRAP",
        ] {
            assert!(
                out.contains(section),
                "{section} is missing from the render"
            );
        }
        assert_eq!(
            out.matches("NOT SUPPLIED").count(),
            5,
            "every absent section must say it was absent from the RENDER rather \
             than from the run"
        );
    }

    #[test]
    fn an_index_run_says_charges_do_not_exist_and_names_where_they_do() {
        // The caveat I carried for most of a session was WRONG for spot: a spot
        // index is not tradeable, so no order is placed and there is no
        // brokerage, STT, stamp or GST to be gross of. `costs::scope` says so.
        // The header records which of the two worlds a reader is in, because
        // the answer changes the moment options land.
        let out = super::render(None, None, None, None, None, 10);
        assert!(out.contains("Slippage IS in these figures"));
        assert!(out.contains("no brokerage"));

        // THE SCOPE IS THE POINT. "No brokerage" is true of an INDEX and false
        // of a STOCK -- a stock spot is tradeable, you buy real shares, and
        // every charge applies. A header saying "spot" would be silently wrong
        // the day a stock is swept, which is the stale-doc failure this
        // repository keeps finding.
        assert!(
            out.contains("INDEX SPOT run"),
            "the header must name INDEX specifically, never just spot"
        );
        assert!(
            out.contains("STOCK spot IS"),
            "the header must state that a stock spot DOES carry charges"
        );
        assert!(
            out.contains("no equity-spot variant"),
            "the gap must be named: `costs::scope::Segment` cannot represent a \
             stock at all, so the charge path does not exist yet"
        );
    }

    /// A grid with real cells, a baseline and a survivor.
    ///
    /// Built by hand rather than by sweeping, so the numbers are known and the
    /// assertions can be exact. Every earlier test in this module exercised the
    /// EMPTY path -- an empty grid, a default `Validated`, `None` verdicts --
    /// which left the populated renderers at 79% coverage. A renderer that has
    /// never rendered anything is not a tested renderer.
    fn populated_grid() -> crate::grid::Grid {
        crate::grid::Grid {
            cells: vec![
                // The baseline: no levels at all.
                crate::grid::Cell {
                    trades: 40,
                    wins: 18,
                    pessimistic: -900,
                    optimistic: -900,
                    timed_out: 40,
                    winner_mae: 240,
                    winner_mfe: 300,
                    ..crate::grid::Cell::default()
                },
                // A stop-only variant: exercises the "-" rendering for a rung
                // that is absent while another is present.
                crate::grid::Cell {
                    stop: Some(0),
                    trades: 51,
                    wins: 24,
                    pessimistic: 1_021,
                    optimistic: 1_240,
                    stopped: 22,
                    timed_out: 29,
                    winner_mae: 96,
                    winner_mfe: 210,
                    ..crate::grid::Cell::default()
                },
                // A target-and-trail variant with NO stop, so the other two "-"
                // renderings are exercised too.
                crate::grid::Cell {
                    target: Some(0),
                    trail: Some(1),
                    trades: 44,
                    wins: 20,
                    pessimistic: 300,
                    optimistic: 300,
                    targeted: 25,
                    timed_out: 19,
                    winner_mae: 150,
                    winner_mfe: 200,
                    ..crate::grid::Cell::default()
                },
                // A survivor with sharp winners.
                crate::grid::Cell {
                    stop: Some(0),
                    target: Some(1),
                    trades: 62,
                    wins: 37,
                    pessimistic: 4_880,
                    optimistic: 5_102,
                    stopped: 15,
                    targeted: 30,
                    timed_out: 17,
                    ambiguous_bars: 4,
                    winner_mae: 41,
                    winner_mfe: 380,
                    ..crate::grid::Cell::default()
                },
            ],
            signals: 1_124,
            ..crate::grid::Grid::default()
        }
    }

    #[test]
    fn a_populated_grid_puts_the_baseline_first_and_names_the_sharpest() {
        // The baseline row IS the comparison the operator asked for, so it must
        // never be buried among variants sorted by profit -- a reader must not
        // have to hunt for the row that answers "with levels or without".
        let mut out = String::new();
        grid(&mut out, &populated_grid(), 10);

        let base_at = out.find("NONE").expect("the baseline row must be present");
        let variant_at = out.find("0/1/-").expect("the level row must be present");
        assert!(
            base_at < variant_at,
            "the baseline must be printed before the variants, not sorted among \
             them by profit"
        );
        assert!(
            out.contains("SHARPEST"),
            "a grid with a surviving variant must name it"
        );
        assert!(
            out.contains("41 ppm against"),
            "the sniper figure is the tightest stop that would not have killed a \
             winner, and it must appear as a number"
        );
        assert!(
            out.contains("depend on intra-bar ordering"),
            "four ambiguous bars must be reported as uncertainty, not hidden"
        );
    }

    #[test]
    fn a_grid_where_nothing_survives_says_so_rather_than_naming_a_loser() {
        // `sharpest` returns None when no variant made money under pessimistic
        // fills. Printing the "best of a bad set" would read as a recommendation.
        let losing = crate::grid::Grid {
            cells: vec![crate::grid::Cell {
                trades: 10,
                wins: 2,
                pessimistic: -5_000,
                optimistic: -5_000,
                ..crate::grid::Cell::default()
            }],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &losing, 10);
        assert!(out.contains("NO SHARPEST VARIANT"));
        assert!(
            out.contains("a finding, not a gap"),
            "nothing surviving is a result, and must not read as missing data"
        );
    }

    #[test]
    fn a_populated_walk_forward_prints_a_row_per_fold() {
        let v = Validated {
            folds: vec![
                crate::validate::FoldResult {
                    index: 0,
                    train_bars: 1_000,
                    purged: 15,
                    test_bars: 500,
                    considered: 207,
                    priced: 207,
                    halted: None,
                    chosen_exit: Some((Some(0), None, None)),
                    chosen_exit_total: Some(1_000),
                    out_of_sample_exit: Some(500),
                    chosen: Some(vocab::ConditionMask::default()),
                    in_sample: crate::validate::Summary {
                        trades: 30,
                        best: 900,
                        worst: 400,
                        forced: 2,
                    },
                    out_of_sample: crate::validate::Summary {
                        trades: 12,
                        best: 300,
                        worst: -150,
                        forced: 1,
                    },
                },
                crate::validate::FoldResult {
                    index: 1,
                    train_bars: 2_000,
                    purged: 15,
                    test_bars: 500,
                    considered: 311,
                    priced: 311,
                    halted: None,
                    chosen_exit: Some((Some(1), Some(2), None)),
                    chosen_exit_total: Some(2_000),
                    out_of_sample_exit: Some(-500),
                    chosen: Some(vocab::ConditionMask::default()),
                    in_sample: crate::validate::Summary {
                        trades: 55,
                        best: 1_400,
                        worst: 800,
                        forced: 3,
                    },
                    out_of_sample: crate::validate::Summary {
                        trades: 20,
                        best: 600,
                        worst: 220,
                        forced: 2,
                    },
                },
            ],
        };
        let mut out = String::new();
        walk_forward(&mut out, &v);

        assert_eq!(cell(&out, "folds"), "2");
        assert_eq!(cell(&out, "still positive out of sample"), "1");
        // THE ROW THIS ASSERTED IS GONE, and its absence is the fix rather than
        // a regression. It read "candidates NOT ranked 44" and its message here
        // said a search that looks at the first N and calls it exhaustive is the
        // defect. That was right about the defect and wrong about the remedy:
        // the budget was reporting the count it discarded while discarding the
        // best candidate along with it, so the honest-looking line was what made
        // the wrong answer survivable. The cap is deleted, every candidate is
        // priced, and `runner::validate::the_chosen_combination_is_the_best_of_every_candidate_and_not_of_a_prefix`
        // holds the property this row used to gesture at.
        assert!(
            !out.contains("candidates NOT ranked"),
            "the candidate budget no longer exists, so nothing may report one"
        );
        assert!(
            out.contains("ONE DRAW"),
            "the walk must state that one fold is not a proof"
        );
    }

    #[test]
    fn a_measured_pbo_prints_its_denominator_and_its_median() {
        // A PBO over three usable folds out of ten is a different number from
        // one over ten, and a reader who cannot see the denominator cannot tell
        // them apart.
        let folds = [
            Placement {
                candidates: 5,
                winner_rank: 0,
            },
            Placement {
                candidates: 5,
                winner_rank: 1,
            },
            Placement {
                candidates: 5,
                winner_rank: 4,
            },
            Placement {
                candidates: 1,
                winner_rank: 0,
            },
        ];
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&folds));

        assert_eq!(cell(&out, "folds ranked"), "3");
        assert!(
            out.contains("folds NOT ranked"),
            "the unrankable fold must be named and excluded from the denominator"
        );
        assert!(out.contains("median placement"));
        assert!(
            out.contains("selection carries information"),
            "one of three below median is under half, so the procedure is not a \
             coin flip and must be described as such"
        );
    }

    #[test]
    fn a_completed_bootstrap_prints_each_test_and_whether_it_cleared() {
        let clears = crate::bootstrap::Verdict {
            statistic: 3.42,
            p_value: 0.004,
            draws: 1_000,
            periods: 250,
            strategies: 20,
        };
        let fails = crate::bootstrap::Verdict {
            statistic: 0.81,
            p_value: 0.612,
            draws: 1_000,
            periods: 250,
            strategies: 20,
        };
        let mut out = String::new();
        bootstrap(&mut out, Some(&clears), Some(&fails), 2);

        assert!(out.contains("White's Reality Check"));
        assert!(out.contains("Hansen's SPA"));
        assert!(
            out.contains("0.0040"),
            "the p-value must be printed to enough places to be read"
        );
        assert!(
            out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
            "the report must say which test answers which question"
        );
    }

    #[test]
    fn one_call_with_everything_supplied_renders_every_section_populated() {
        // The single entry point, exercised with real inputs rather than None.
        // `render(None, ...)` proved the absent path; this proves the present
        // one, and between them every branch of the dispatcher is taken.
        let taken = Trades {
            trades: vec![
                Trade {
                    signal_bar: 0,
                    entry_bar: 1,
                    exit_bar: 5,
                    best: 10,
                    worst: -5,
                    forced: false,
                };
                3
            ],
            signals: 50,
            while_open: 45,
            too_late: 2,
        };
        let grid_in = populated_grid();
        let folds = Validated::default();
        let over = probability_of_overfitting(&[]);
        let out = super::render(
            Some(&taken),
            Some(&grid_in),
            Some(&folds),
            Some(&over),
            Some((None, None, 0)),
            10,
        );

        assert!(
            !out.contains("NOT SUPPLIED"),
            "every section was supplied, so none may claim otherwise"
        );
        for section in [
            "TRADES",
            "EXIT GRID",
            "WALK-FORWARD",
            "OVERFITTING",
            "BOOTSTRAP",
        ] {
            assert!(out.contains(section), "{section} missing");
        }
    }

    #[test]
    fn a_refused_bootstrap_is_shown_as_refused_and_never_as_a_pass() {
        let mut out = String::new();
        bootstrap(&mut out, None, None, 0);
        assert!(out.contains("REFUSED"));
        assert!(
            !out.contains("yes"),
            "a test that was refused must never render as clearing"
        );
    }

    #[test]
    fn a_grid_table_that_hides_rows_says_how_many() {
        // A table showing the best twelve of a hundred reads as the whole grid
        // unless it says otherwise. `CLAUDE.md` section 4: never silently.
        let g = crate::grid::Grid {
            cells: vec![crate::grid::Cell::default(); 30],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 5);
        assert!(
            out.contains("NOT SHOWN"),
            "a truncated table must name what it dropped"
        );
        assert!(out.contains("25 further"));
    }
}
