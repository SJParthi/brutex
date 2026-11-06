#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::generator::day;
use crate::rng::SplitMix64;

fn w(first: (i32, u32), last: (i32, u32)) -> Window {
    Window { first: Month::of(first.0, first.1), last: Month::of(last.0, last.1) }
}

fn c(d: Day, net: i128) -> Closed {
    Closed { day: d, second: 40_000, net }
}

/// X-F55: the month bar at its edges.
#[test]
fn x_f55_month_extremes() {
    let window = w((2018, 9), (2018, 11));
    let rows = months(&window, &[c(day(2018, 9, 4), 5_000_000), c(day(2018, 10, 31), -2_000), c(day(2018, 11, 1), 4_999_800)]);
    assert_eq!(rows.len(), 3);
    assert_eq!((rows[0].bp, rows[0].pass), (Some(250), true));
    assert_eq!((rows[1].capital_start, rows[1].bp, rows[1].pass), (205_000_000, Some(-1), false));
    assert_eq!(rows[2].capital_start, 204_998_000);
    assert_eq!((rows[2].bp, rows[2].pass), (Some(243), false));
    // no trade: 0 bp and a fail
    let empty = months(&w((2018, 9), (2018, 9)), &[]);
    assert_eq!((empty[0].bp, empty[0].pass, empty[0].trades), (Some(0), false, 0));
    // exactly 2.4999%: 249 bp fails; exactly 2.5000% passes
    assert!(!months(&w((2018, 9), (2018, 9)), &[c(day(2018, 9, 4), 4_999_999)])[0].pass);
    // combo: the account at or below zero: n/a and a fail, even on a positive month
    let broke = months(&w((2018, 9), (2018, 10)), &[c(day(2018, 9, 4), -300_000_000), c(day(2018, 10, 4), 900_000_000)]);
    assert_eq!((broke[1].bp, broke[1].pass), (None, false));
    let zero = months(&w((2018, 9), (2018, 10)), &[c(day(2018, 9, 4), -200_000_000), c(day(2018, 10, 4), 1)]);
    assert_eq!((zero[1].capital_start, zero[1].bp), (0, None));
    // combo: a million trades sum exactly in i128
    let many: Vec<Closed> = (0..1_000_000).map(|_| c(day(2018, 9, 4), 1)).collect();
    assert_eq!(months(&w((2018, 9), (2018, 9)), &many)[0].net, 1_000_000);
    // oob: the month count comes from the window
    assert_eq!(months(&w((2018, 1), (2026, 2)), &[]).len(), 98);
}

/// X-F56: the drawdown at its edges.
#[test]
fn x_f56_drawdown_extremes() {
    let d = |n: u32| day(2018, 9, n);
    assert_eq!(drawdown(&[]), Drawdown { bp: 0, paisa: 0, peak_day: None, low_day: None });
    assert_eq!(drawdown(&[c(d(3), 10), c(d(4), 10)]).low_day, None);
    let first_loss = drawdown(&[c(d(3), -2_000_000)]);
    assert_eq!((first_loss.bp, first_loss.paisa, first_loss.peak_day, first_loss.low_day), (100, 2_000_000, None, Some(d(3))));
    let recovered = drawdown(&[c(d(3), 1_000_000), c(d(4), -3_000_000), c(d(5), 5_000_000), c(d(6), -1_000_000)]);
    assert_eq!((recovered.paisa, recovered.peak_day, recovered.low_day), (3_000_000, Some(d(3)), Some(d(4))));
    // equal to the previous peak: no new drawdown, the peak day unchanged
    let equal = drawdown(&[c(d(3), 100), c(d(4), -100), c(d(5), 100)]);
    assert_eq!((equal.peak_day, equal.low_day), (Some(d(3)), Some(d(4))));
    // two equal falls: the first one's dates (Q9)
    let twice = drawdown(&[c(d(3), -1_000_000), c(d(4), 1_000_000), c(d(5), -1_000_000)]);
    assert_eq!((twice.peak_day, twice.low_day), (None, Some(d(3))));
    // combo: below zero, more than 100%
    let deep = drawdown(&[c(d(3), -300_000_000)]);
    assert_eq!(deep.bp, 15_000);
    // combo: a tiny fall with 0 bp still names its days
    let tiny = drawdown(&[c(d(3), -1)]);
    assert_eq!((tiny.bp, tiny.paisa, tiny.low_day), (0, 1, Some(d(3))));
    // oob: a million marks in fixed state
    let marks: Vec<Closed> = (0..1_000_000).map(|i| c(d(3), if i % 2 == 0 { 3 } else { -2 })).collect();
    assert_eq!(drawdown(&marks).paisa, 2);
}

/// P-F55-1 and P-F56-1: generated trade lists against a naive per-month sum and an
/// O(n^2) drawdown over every (peak, later low) pair.
#[test]
fn p_f55_f56_months_and_drawdown_vs_naive() {
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..250 {
            let mut list: Vec<Closed> = (0..40)
                .map(|_| {
                    let dd = day(2018, 9, 1).index() + i32::try_from(r.below(80)).unwrap();
                    c(Day::from_index(dd), i128::from(r.between(-3_000_000, 3_000_000)))
                })
                .collect();
            list.sort_by_key(|x| x.day);
            let window = w((2018, 9), (2018, 11));
            let rows = months(&window, &list);
            let mut capital = 200_000_000i128;
            for (row, m) in rows.iter().zip(window.months()) {
                let net: i128 = list.iter().filter(|x| x.day.month() == m).map(|x| x.net).sum();
                assert_eq!((row.capital_start, row.net), (capital, net));
                assert_eq!(row.pass, capital > 0 && net * 10_000 >= capital * 250);
                capital += net;
            }
            let mut equity = vec![200_000_000i128];
            for x in &list {
                equity.push(equity.last().unwrap() + x.net);
            }
            let mut best = 0i128;
            for i in 0..equity.len() {
                let peak = equity[..=i].iter().copied().max().unwrap();
                best = best.max((10_000 * (peak - equity[i])).div_euclid(peak));
            }
            assert_eq!(drawdown(&list).bp, best);
        }
    }
}
