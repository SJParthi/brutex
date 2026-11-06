#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::time::Month;

fn month(bp: Option<i128>, net: i128, pass: bool) -> MonthRow {
    MonthRow { month: Month::of(2018, 9), capital_start: 200_000_000, net, trades: 1, bp, pass }
}

/// T-F57 and X-F57: the row statistics at their edges.
#[test]
fn x_f57_row_stats_extremes() {
    let rows = [month(Some(-5), -100_000, false), month(Some(300), 6_000_000, true), month(Some(10), 200_000, false)];
    let s = row_stats(&rows, &[6_000_000, -100_000, 200_000, 0], 1_234);
    assert_eq!((s.months, s.months_pass, s.months_positive), (3, 1, 2));
    assert_eq!((s.worst_bp, s.median_bp, s.best_bp), (Some(-5), Some(10), Some(300)));
    assert_eq!((s.total_net, s.total_bp, s.trades, s.wins, s.losses), (6_100_000, 305, 4, 2, 1));
    assert_eq!((s.win_rate, s.avg_win, s.avg_loss), (Some(500), Some(3_100_000), Some(-100_000)));
    assert_eq!((s.profit_factor, s.charges), (Some(62_000), 1_234));
    // no trades: words, no division
    let none = row_stats(&rows, &[], 0);
    assert_eq!((none.win_rate, none.avg_win, none.avg_loss, none.profit_factor), (None, None, None, None));
    // no wins, no losses
    assert_eq!(row_stats(&rows, &[-3], 0).avg_win, None);
    assert_eq!(row_stats(&rows, &[3], 0).profit_factor, None);
    // a net-zero trade is neither
    let zero = row_stats(&rows, &[0], 0);
    assert_eq!((zero.wins, zero.losses, zero.win_rate), (0, 0, Some(0)));
    // the lower median of two months is the first
    let two = row_stats(&[month(Some(7), 1, false), month(Some(3), 1, false)], &[], 0);
    assert_eq!(two.median_bp, Some(3));
    // combo: every month equal
    let same = row_stats(&[month(Some(4), 1, false); 5], &[], 0);
    assert_eq!((same.worst_bp, same.median_bp, same.best_bp), (Some(4), Some(4), Some(4)));
    // combo: a huge gross loss stays exact
    let huge = row_stats(&rows, &[i128::from(i64::MAX), -i128::from(i64::MAX)], 0);
    assert_eq!(huge.profit_factor, Some(1_000));
    // oob: months at or below zero capital are left out of the bp statistics
    let broke = row_stats(&[month(None, 5, false), month(Some(2), 1, false)], &[], 0);
    assert_eq!((broke.worst_bp, broke.median_bp, broke.best_bp, broke.months_positive), (Some(2), Some(2), Some(2), 2));
    assert_eq!(row_stats(&[], &[], 0).median_bp, None);
}

/// X-F60: the head comparison's permille at its edges.
#[test]
fn x_f60_permille_extremes() {
    assert_eq!(permille(150, 100), Some(1_500));
    assert_eq!(permille(100, 100), Some(1_000));
    assert_eq!(permille(-50, 100), Some(-500));
    assert_eq!(permille(1, 3), Some(333));
    assert_eq!(permille(5, 0), None);
    assert_eq!(permille(5, -1), None);
    assert_eq!(permille(i128::from(i64::MAX), 1), Some(1_000 * i128::from(i64::MAX)));
    assert_eq!(permille(0, 7), Some(0));
}
