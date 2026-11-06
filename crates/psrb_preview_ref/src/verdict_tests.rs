#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::rng::SplitMix64;
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

/// X-F58: the bar at its edges.
#[test]
fn x_f58_bar_extremes() {
    let pass = bar(&[
        month(Some(250), 5_000_000, true),
        month(Some(300), 6_000_000, true),
    ]);
    assert_eq!(
        (pass.pass, pass.gaps.len(), pass.floor),
        (true, 0, Some(250))
    );
    let miss = bar(&[
        month(Some(249), 4_999_800, false),
        month(Some(-10), -200_000, false),
    ]);
    assert!(!miss.pass);
    assert_eq!(
        miss.gaps[0],
        Gap {
            index: 0,
            bp: Some(1),
            exact: 2_000_000
        }
    );
    assert_eq!(miss.gaps[1].bp, Some(260));
    assert_eq!(miss.floor, Some(-10));
    // all months without trades: every gap 250 bp
    let quiet = bar(&[month(Some(0), 0, false); 3]);
    assert!(quiet.gaps.iter().all(|g| g.bp == Some(250)));
    // combo: a month at or below zero capital
    let broke = bar(&[month(None, 5, false)]);
    assert_eq!((broke.gaps[0].bp, broke.floor), (None, None));
    // oob: no month at all is not a pass
    assert!(!bar(&[]).pass);
}

fn row(
    pass: bool,
    dd: i128,
    net: i128,
    wr: i128,
    below: usize,
    worst: Option<i128>,
    tie: usize,
) -> Rankable {
    Rankable {
        pass,
        max_dd_bp: dd,
        total_net: net,
        win_rate: wr,
        months_below: below,
        worst,
        tie: (tie, 1, 0),
    }
}

/// T-F59-4 and X-F59: every criterion decides alone, and the edges.
#[test]
fn x_f59_rank_extremes() {
    let pairs = [
        (
            row(true, 1, 0, 0, 0, Some(1), 1),
            row(true, 2, 0, 0, 0, Some(1), 0),
        ),
        (
            row(true, 1, 9, 0, 0, Some(1), 1),
            row(true, 1, 8, 0, 0, Some(1), 0),
        ),
        (
            row(true, 1, 9, 7, 0, Some(1), 1),
            row(true, 1, 9, 6, 0, Some(1), 0),
        ),
        (
            row(false, 9, 0, 0, 1, Some(1), 1),
            row(false, 1, 0, 0, 2, Some(1), 0),
        ),
        (
            row(false, 9, 0, 0, 1, Some(5), 1),
            row(false, 1, 0, 0, 1, Some(4), 0),
        ),
        (
            row(false, 1, 0, 0, 1, Some(5), 1),
            row(false, 2, 0, 0, 1, Some(5), 0),
        ),
        (
            row(false, 1, 9, 0, 1, Some(5), 1),
            row(false, 1, 8, 0, 1, Some(5), 0),
        ),
        (
            row(true, 99, -9, 0, 0, Some(1), 1),
            row(false, 0, 999, 900, 0, Some(9), 0),
        ),
        (
            row(false, 0, 0, 0, 1, Some(1), 0),
            row(false, 0, 0, 0, 1, Some(1), 1),
        ),
        (
            row(false, 0, 0, 0, 1, Some(-1), 1),
            row(false, 0, 0, 0, 1, None, 0),
        ),
    ];
    for (i, (a, b)) in pairs.iter().enumerate() {
        assert_eq!(compare(a, b), Ordering::Less, "pair {i}");
        assert_eq!(compare(b, a), Ordering::Greater, "pair {i}");
        assert_eq!(rank(&[*b, *a]), vec![1, 0], "pair {i}");
    }
    assert_eq!(rank(&[]), Vec::<usize>::new());
    assert_eq!(rank(&[pairs[0].0]), vec![0]);
}

/// P-F59-1: generated row sets against an explicit comparator sort, input order free.
#[test]
fn p_f59_rank_vs_comparator_model() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..2_500 {
            let rows: Vec<Rankable> = (0..6)
                .map(|i| {
                    let pass = r.chance(1, 3);
                    row(
                        pass,
                        r.between(0, 3).into(),
                        r.between(-2, 2).into(),
                        r.between(0, 2).into(),
                        usize::try_from(r.below(3)).unwrap(),
                        Some(r.between(-2, 2).into()),
                        i,
                    )
                })
                .collect();
            let order = rank(&rows);
            for pair in order.windows(2) {
                let (a, b) = (&rows[pair[0]], &rows[pair[1]]);
                let key = |x: &Rankable| {
                    (
                        !x.pass,
                        if x.pass {
                            (x.max_dd_bp, -x.total_net, -x.win_rate, 0, 0)
                        } else {
                            (
                                i128::try_from(x.months_below).unwrap(),
                                -x.worst.unwrap(),
                                x.max_dd_bp,
                                -x.total_net,
                                0,
                            )
                        },
                        x.tie,
                    )
                };
                assert!(key(a) <= key(b));
            }
            let mut rev = rows.clone();
            rev.reverse();
            let back: Vec<usize> = rank(&rev).into_iter().map(|i| rows.len() - 1 - i).collect();
            assert_eq!(back, order);
        }
    }
}
