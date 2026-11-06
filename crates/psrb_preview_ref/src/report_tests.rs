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
    MonthRow {
        month: Month::of(2018, 9),
        capital_start: 200_000_000,
        net,
        trades: 1,
        bp,
        pass,
    }
}

/// T-F57 and X-F57: the row statistics at their edges.
#[test]
fn x_f57_row_stats_extremes() {
    let rows = [
        month(Some(-5), -100_000, false),
        month(Some(300), 6_000_000, true),
        month(Some(10), 200_000, false),
    ];
    let s = row_stats(&rows, &[6_000_000, -100_000, 200_000, 0], 1_234);
    assert_eq!((s.months, s.months_pass, s.months_positive), (3, 1, 2));
    assert_eq!(
        (s.worst_bp, s.median_bp, s.best_bp),
        (Some(-5), Some(10), Some(300))
    );
    assert_eq!(
        (s.total_net, s.total_bp, s.trades, s.wins, s.losses),
        (6_100_000, 305, 4, 2, 1)
    );
    assert_eq!(
        (s.win_rate, s.avg_win, s.avg_loss),
        (Some(500), Some(3_100_000), Some(-100_000))
    );
    assert_eq!((s.profit_factor, s.charges), (Some(62_000), 1_234));
    // no trades: words, no division
    let none = row_stats(&rows, &[], 0);
    assert_eq!(
        (
            none.win_rate,
            none.avg_win,
            none.avg_loss,
            none.profit_factor
        ),
        (None, None, None, None)
    );
    // no wins, no losses
    assert_eq!(row_stats(&rows, &[-3], 0).avg_win, None);
    assert_eq!(row_stats(&rows, &[3], 0).profit_factor, None);
    // a net-zero trade is neither
    let zero = row_stats(&rows, &[0], 0);
    assert_eq!((zero.wins, zero.losses, zero.win_rate), (0, 0, Some(0)));
    // the lower median of two months is the first
    let two = row_stats(
        &[month(Some(7), 1, false), month(Some(3), 1, false)],
        &[],
        0,
    );
    assert_eq!(two.median_bp, Some(3));
    // combo: every month equal
    let same = row_stats(&[month(Some(4), 1, false); 5], &[], 0);
    assert_eq!(
        (same.worst_bp, same.median_bp, same.best_bp),
        (Some(4), Some(4), Some(4))
    );
    // combo: a huge gross loss stays exact
    let huge = row_stats(&rows, &[i128::from(i64::MAX), -i128::from(i64::MAX)], 0);
    assert_eq!(huge.profit_factor, Some(1_000));
    // oob: months at or below zero capital are left out of the bp statistics
    let broke = row_stats(&[month(None, 5, false), month(Some(2), 1, false)], &[], 0);
    assert_eq!(
        (
            broke.worst_bp,
            broke.median_bp,
            broke.best_bp,
            broke.months_positive
        ),
        (Some(2), Some(2), Some(2), 2)
    );
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
    assert_eq!(
        permille(i128::from(i64::MAX), 1),
        Some(1_000 * i128::from(i64::MAX))
    );
    assert_eq!(permille(0, 7), Some(0));
}

fn head_row(set: usize, k: u32, view: View, m: [i128; 4], floor: Option<i128>) -> HeadRow {
    HeadRow {
        set,
        k,
        view,
        measures: Measures {
            trades: m[0],
            net: m[1],
            dd_bp: m[2],
            dd: m[3],
        },
        floor,
    }
}

/// T-F60-1: AUTHOR-V2 and VIDEO-SCREEN over VIDEO-ORIGINAL (and VIDEO-SCREEN over AUTHOR-V2)
/// for trades, net and both drawdown readings.
#[test]
fn t_f60_1_the_pairs_in_permille() {
    let short = |i: usize| crate::ruleset::RULE_SETS[i].short;
    let names: Vec<(&str, &str)> = PAIRS.iter().map(|(o, b)| (short(*o), short(*b))).collect();
    assert_eq!(names, [("AV2", "VO"), ("VS", "VO"), ("VS", "AV2")]);
    let rows = [
        head_row(0, 5, View::Long, [100, 1_000_000, 500, 1_000_000], Some(10)),
        head_row(1, 5, View::Long, [150, 2_000_000, 250, 500_000], Some(20)),
        head_row(2, 5, View::Long, [50, -500_000, 1_000, 2_000_000], None),
    ];
    let head = head_compare(&rows, &PAIRS);
    let got: Vec<(usize, usize, [Per; 4])> = head
        .lines
        .iter()
        .map(|l| (l.over, l.base, [l.trades, l.net, l.dd_bp, l.dd]))
        .collect();
    let p = Per::Permille;
    assert_eq!(
        got,
        [
            (1, 0, [p(1_500), p(2_000), p(500), p(500)]),
            (2, 0, [p(500), p(-500), p(2_000), p(2_000)]),
            (2, 1, [p(333), p(-250), p(4_000), p(4_000)]),
        ]
    );
    assert!(head.lines.iter().all(|l| l.k == 5 && l.view == View::Long));
}

/// T-F60-2: the nearest met floor is the highest floor over every row; the first row
/// holding it is named; a row without a floor is passed over.
#[test]
fn t_f60_2_nearest_met_floor_over_every_row() {
    let floors = [Some(-30), Some(120), None, Some(120), Some(5)];
    let rows: Vec<HeadRow> = floors
        .iter()
        .enumerate()
        .map(|(i, f)| head_row(i % 3, 1, View::Both, [0; 4], *f))
        .collect();
    assert_eq!(head_compare(&rows, &PAIRS).nearest, Some((1, 120)));
    assert_eq!(head_compare(&rows[2..3], &PAIRS).nearest, None);
    assert_eq!(
        head_compare(&[], &PAIRS),
        Head {
            lines: vec![],
            nearest: None
        }
    );
}

/// T-F60-3: a base above, at and below zero for every measure (nine cases).
#[test]
fn t_f60_3_base_sign_for_every_measure() {
    for (k, base) in [(1, 40), (3, 0), (5, -40)] {
        let rows = [
            head_row(0, k, View::Short, [base; 4], None),
            head_row(1, k, View::Short, [10; 4], None),
        ];
        let head = head_compare(&rows, &PAIRS);
        assert_eq!(head.lines.len(), 1);
        let l = head.lines[0];
        let want = if base > 0 {
            Per::Permille(250)
        } else {
            Per::Base(base)
        };
        assert_eq!([l.trades, l.net, l.dd_bp, l.dd], [want; 4]);
    }
}

/// X-F60: the head at its edges.
#[test]
fn x_f60_head_extremes() {
    // X-F60-1 zero base, X-F60-2 a negative base: printed, never divided
    assert_eq!(per(5, 0), Per::Base(0));
    assert_eq!(per(5, -20), Per::Base(-20));
    // X-F60-3 equal: 1,000 permille
    assert_eq!(per(77, 77), Per::Permille(1_000));
    // X-F60-4 a huge ratio stays exact in i128
    assert_eq!(
        per(1_000 * i128::from(i64::MAX), 1),
        Per::Permille(1_000_000 * i128::from(i64::MAX))
    );
    // X-F60-5 no trades either side: every measure is a word
    let none = [
        head_row(0, 15, View::Long, [0; 4], None),
        head_row(1, 15, View::Long, [0; 4], None),
    ];
    let l = head_compare(&none, &PAIRS).lines[0];
    assert_eq!([l.trades, l.net, l.dd_bp, l.dd], [Per::Base(0); 4]);
    // X-F60-6 (combo) a zero base with a negative other, and a negative base with a zero other
    assert_eq!(per(-500, 0), Per::Base(0));
    assert_eq!(per(0, -100), Per::Base(-100));
    // X-F60-7 (combo) only two rule sets: the head still prints their pair, and only it
    let two = [
        head_row(0, 30, View::Both, [4, 8, 2, 6], Some(-1)),
        head_row(1, 30, View::Both, [2, 4, 4, 3], Some(-2)),
    ];
    let head = head_compare(&two, &PAIRS);
    assert_eq!(head.lines.len(), 1);
    assert_eq!((head.lines[0].over, head.lines[0].base), (1, 0));
    assert_eq!(head.lines[0].dd_bp, Per::Permille(2_000));
    // X-F60-8 (oob) every floor negative: the nearest met floor is the highest of them
    let neg: Vec<HeadRow> = [-500, -20, -300]
        .iter()
        .map(|f| head_row(0, 60, View::Short, [0; 4], Some(*f)))
        .collect();
    assert_eq!(head_compare(&neg, &PAIRS).nearest, Some((1, -20)));
    // a key named twice keeps its first row; keys keep the rows' order
    let order = [
        head_row(0, 60, View::Short, [1; 4], None),
        head_row(0, 1, View::Long, [1; 4], None),
        head_row(1, 1, View::Long, [3; 4], None),
        head_row(1, 60, View::Short, [2; 4], None),
        head_row(1, 60, View::Short, [9; 4], None),
    ];
    let lines = head_compare(&order, &PAIRS).lines;
    assert_eq!(
        lines.iter().map(|l| (l.k, l.trades)).collect::<Vec<_>>(),
        [(60, Per::Permille(2_000)), (1, Per::Permille(3_000))]
    );
}

/// P-F60-1: 10,000 generated pairs against the definition: a base at or below zero is
/// printed; else the permille `p` is the one with `p x base <= 1,000 x over < (p + 1) x base`.
#[test]
fn p_f60_1_head_vs_model() {
    for seed in crate::generator::SEEDS {
        let mut r = crate::rng::SplitMix64::new(seed);
        for _ in 0..2_500 {
            let over = i128::from(r.between(-5_000_000_000, 5_000_000_000));
            let base = i128::from(r.between(-1_000, 1_000_000_000));
            match per(over, base) {
                Per::Base(b) => assert!(base <= 0 && b == base),
                Per::Permille(p) => {
                    assert!(base > 0);
                    assert!(p * base <= 1_000 * over && 1_000 * over < (p + 1) * base);
                }
            }
        }
    }
}
