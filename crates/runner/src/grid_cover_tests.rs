#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "tests state their expectations"
)]
//! BASE-COV-LIBS: the grid's small answers no sweep in the other tests asks
//! for: a cell with no trades or no losers, the trade count a rate needs, the
//! two cell pickers on an empty, a tradeless and a tied grid, and the entry
//! fill's two refusals.
//!
//! Extreme cases: zero trades; every trade a winner; a ceiling of zero and of
//! one; an assurance of zero, of `i64::MIN` and above the rate itself; a rate
//! above 100% and below zero; combinations: a tradeless grid whose only cell
//! would otherwise win both pickers, and a tie on the first key broken by the
//! second; and, out of the box, an entry index past the last bar.

use super::*;

fn cell(trades: u64, wins: u64) -> Cell {
    Cell {
        trades,
        wins,
        ..Cell::default()
    }
}

#[test]
fn a_cell_without_trades_or_losers_answers_zero_never_a_division() {
    let empty = cell(0, 0);
    assert_eq!(empty.win_rate_bp(), 0);
    assert_eq!(empty.avg_bars_held(), 0);
    assert_eq!(empty.avg_loss_magnitude_ceil(), 0);
    let all_won = Cell {
        gross_loss: -500,
        bars_held: 9,
        ..cell(3, 3)
    };
    assert_eq!(all_won.avg_loss_magnitude_ceil(), 0);
    assert_eq!(all_won.avg_bars_held(), 3);
    assert_eq!(all_won.win_rate_bp(), 10_000);
    let one_loser = Cell {
        gross_loss: -500,
        ..cell(3, 2)
    };
    assert_eq!(one_loser.avg_loss_magnitude_ceil(), 500);
}

#[test]
fn the_trades_a_rate_needs_is_the_first_count_that_clears_its_assurance() {
    for assurance in [0, -1, i64::MIN] {
        assert_eq!(trades_needed_for(8_000, assurance, 1_000), 1);
    }
    // The smallest count that clears, and the one before it does not.
    let needed = trades_needed_for(8_000, 5_000, 10_000);
    assert!(needed > 1 && needed < 10_000, "{needed}");
    assert!(Cell::at_rate(needed, 8_000).assurance_bp() >= 5_000);
    assert!(Cell::at_rate(needed - 1, 8_000).assurance_bp() < 5_000);
    // A floor above the rate itself is never reached: the ceiling comes back.
    assert_eq!(trades_needed_for(5_000, 6_000, 300), 300);
    assert_eq!(trades_needed_for(5_000, 6_000, 0), 0);
    assert_eq!(trades_needed_for(-50, 1, 1), 1);
    // A rate past 100% is held to every trade a winner, never more wins.
    let over = Cell::at_rate(10, 20_000);
    assert_eq!((over.trades, over.wins), (10, 10));
    let under = Cell::at_rate(10, -3);
    assert_eq!((under.trades, under.wins), (10, 0));
    assert_eq!(Cell::at_rate(3, 5_000).wins, 2, "wins round up");
}

#[test]
fn the_two_pickers_skip_tradeless_cells_and_break_ties_by_their_second_key() {
    let empty = Grid::default();
    assert_eq!(empty.tightest_containment(), None);
    assert_eq!(empty.best_within(|_| true), None);
    // The only cell has no trades: neither picker may name it.
    let tradeless = Grid {
        cells: vec![Cell {
            pessimistic: i64::MAX,
            worst_mae: 0,
            ..cell(0, 0)
        }],
        ..Grid::default()
    };
    assert_eq!(tradeless.tightest_containment(), None);
    assert_eq!(tradeless.best_within(|_| true), None);

    let tight = Cell {
        worst_mae: 10,
        pessimistic: 5,
        ..cell(2, 1)
    };
    let tight_richer = Cell {
        worst_mae: 10,
        pessimistic: 9,
        stop: Some(0),
        ..cell(2, 1)
    };
    let loose = Cell {
        worst_mae: 900,
        pessimistic: 9,
        ..cell(4, 4)
    };
    let grid = Grid {
        cells: vec![loose, tight, tight_richer],
        ..Grid::default()
    };
    // Equal containment: the larger pessimistic total is the tighter pick.
    assert_eq!(grid.tightest_containment(), Some(&tight_richer));
    // Equal pessimistic totals: the simpler cell (fewer rungs) wins.
    assert_eq!(grid.best_within(|_| true), Some(&loose));
    assert_eq!(grid.best_within(|c| c.worst_mae < 100), Some(&tight_richer));
    assert_eq!(grid.best_within(|_| false), None);
}

#[test]
fn an_entry_fill_without_a_bar_or_inside_a_broken_bar_is_no_entry() {
    let good = Candle {
        ts_micros: 1,
        open: 10_000,
        high: 10_100,
        low: 9_900,
        close: 10_050,
        volume: 1,
        open_interest: indicators::OI_NULL,
    };
    let bars = [good];
    assert_eq!(entry_fills(&bars, 1, Side::Long), (0, 0));
    assert_eq!(entry_fills(&bars, usize::MAX, Side::Short), (0, 0));
    assert_eq!(entry_fills(&[], 0, Side::Long), (0, 0));
    // An open above its own high is refused at the bracket: no fill, the open
    // reported beside it.
    let broken = Candle {
        open: 10_200,
        ..good
    };
    assert_eq!(entry_fills(&[broken], 0, Side::Long), (0, 10_200));
    assert_eq!(entry_fills(&[broken], 0, Side::Short), (0, 10_200));
    let (fill, open) = entry_fills(&bars, 0, Side::Long);
    assert_eq!(open, 10_000);
    assert!(fill > 0, "{fill}");
}
