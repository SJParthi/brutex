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
        "exit", "trades", "won", "stopped", "pess", "opt", "winner MAE", "ratio"
    );

    // The baseline first, always, then the rest by pessimistic total. A reader
    // comparing "with levels" against "without" must not have to hunt for the
    // row that is the comparison.
    let mut ordered: Vec<&Cell> = g.cells.iter().collect();
    ordered.sort_by_key(|c| {
        let base = c.stop.is_none() && c.target.is_none() && c.trail.is_none();
        (!base, -c.pessimistic)
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
            c.optimistic,
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
    if v.not_considered > 0 {
        row(
            out,
            "  candidates NOT ranked",
            &v.not_considered.to_string(),
            "the budget was reached; the search was not exhaustive",
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {:<6}{:>10}{:>10}{:>12}{:>14}{:>14}",
        "fold", "train", "test", "candidates", "in-sample", "out-of-sample"
    );
    for f in &v.folds {
        let _ = writeln!(
            out,
            "  {:<6}{:>10}{:>10}{:>12}{:>14}{:>14}",
            f.index,
            f.train_bars,
            f.test_bars,
            f.considered,
            f.in_sample.worst,
            f.out_of_sample.worst
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
    let _ = writeln!(out);
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
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
