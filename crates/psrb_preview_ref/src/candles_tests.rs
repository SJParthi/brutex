#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::many_single_char_names,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::generator::day;
use crate::rng::SplitMix64;
use crate::time::{CLOSE_FULL, ENTRY_CUT};

fn minute(i: u32, price: i64, volume: i64) -> Minute {
    Minute {
        start: OPEN + 60 * i,
        bar: Bar::flat(price, volume),
    }
}

fn full_day(count: u32) -> Vec<Minute> {
    (0..count)
        .map(|i| minute(i, 100 + i64::from(i), 10))
        .collect()
}

#[test]
fn t_f07_bucket_counts_on_both_session_shapes() {
    let d = day(2018, 9, 4);
    let eligible = day(2026, 8, 4);
    for (k, full, auction) in [
        (1, 375, 360),
        (3, 125, 120),
        (5, 75, 72),
        (15, 25, 24),
        (30, 13, 12),
        (60, 7, 6),
    ] {
        let mut c = FoldCounts::default();
        assert_eq!(
            fold_day(d, &full_day(375), k, false, &mut c).len(),
            full,
            "k {k}"
        );
        assert_eq!(
            fold_day(eligible, &full_day(360), k, false, &mut c).len(),
            auction,
            "k {k}"
        );
        assert_eq!(c.holes, 0);
    }
    let three = fold_day(d, &full_day(375), 3, false, &mut FoldCounts::default());
    let b = three[0].bar.unwrap();
    assert_eq!(
        (b.open, b.high, b.low, b.close, b.volume),
        (100, 102, 100, 102, 30)
    );
    assert_eq!((three[0].start, three[0].t), (OPEN, OPEN + 180));
    assert!(TIMEFRAMES.contains(&60));
}

/// X-F07 and X-F10: the fold and the hole rule at their edges.
#[test]
fn x_f07_f10_fold_and_hole_extremes() {
    let d = day(2018, 9, 4);
    let mut c = FoldCounts::default();
    // a bucket of only padded minutes has no bar and is not a hole
    let mut padded = full_day(375);
    padded[0].bar.volume = 0;
    padded[1].bar.volume = 0;
    padded[2].bar.volume = 0;
    let out = fold_day(d, &padded, 3, false, &mut c);
    assert_eq!(out.len(), 124);
    assert_eq!(c.holes, 0);
    // one missing minute: a hole that still holds a bar
    let mut gap = full_day(375);
    gap.remove(1);
    let out = fold_day(d, &gap, 3, false, &mut c);
    assert!(out[0].hole && out[0].bar.is_some());
    // a one-minute day: k=1 one clean candle and 374 holes; k=60 one hole holding a bar
    let one = vec![minute(0, 100, 10)];
    let k1 = fold_day(d, &one, 1, false, &mut FoldCounts::default());
    assert_eq!((k1.len(), k1.iter().filter(|b| b.hole).count()), (375, 374));
    let k60 = fold_day(d, &one, 60, false, &mut FoldCounts::default());
    assert!(k60[0].hole && k60[0].bar.is_some() && k60[1].bar.is_none());
    // combo: a withheld day: every bucket a hole and no bar
    let blocked = fold_day(d, &full_day(375), 15, true, &mut FoldCounts::default());
    assert!(blocked.iter().all(|b| b.hole && b.bar.is_none()) && blocked.len() == 25);
    // the 15:15-15:30 stub folds for k=30 and k=60 and closes after 15:08
    let k30 = fold_day(d, &full_day(375), 30, false, &mut FoldCounts::default());
    let stub = k30.last().unwrap();
    assert_eq!((stub.start, stub.t), (OPEN + 60 * 360, CLOSE_FULL));
    assert!(stub.t >= ENTRY_CUT);
    // oob: minutes outside the session are counted and never folded
    let mut outside = full_day(375);
    outside.insert(
        0,
        Minute {
            start: OPEN - 900,
            bar: Bar::flat(5, 5),
        },
    );
    outside.push(Minute {
        start: CLOSE_FULL + 60,
        bar: Bar::flat(5, 5),
    });
    let mut oc = FoldCounts::default();
    let out = fold_day(d, &outside, 1, false, &mut oc);
    assert_eq!((out.len(), oc.outside_session), (375, 2));
    assert_eq!(out[0].bar.unwrap().open, 100);
    // combo: an auction-eligible day with a padded 15:14 minute and k=60
    let mut auction = full_day(360);
    auction[359].bar.volume = 0;
    let out = fold_day(
        day(2026, 8, 3),
        &auction,
        60,
        false,
        &mut FoldCounts::default(),
    );
    assert_eq!(out.len(), 6);
    assert_eq!(out[5].t, CLOSE_AUCTION);
    assert!(fold_day(d, &full_day(375), 0, false, &mut FoldCounts::default()).is_empty());
    assert_eq!(fold_bar(&[]), None);
}

/// X-F08: close instants at their edges.
#[test]
fn x_f08_close_instant_extremes() {
    let at = |hh: u32, mm: u32| hh * 3_600 + mm * 60;
    assert_eq!(close_instant(OPEN, 3, CLOSE_FULL), Some(at(9, 18)));
    assert_eq!(close_instant(at(15, 0), 30, CLOSE_FULL), Some(at(15, 30)));
    assert_eq!(close_instant(at(15, 15), 30, CLOSE_FULL), Some(at(15, 30)));
    assert_eq!(
        close_instant(at(15, 0), 30, CLOSE_AUCTION),
        Some(at(15, 15))
    );
    assert_eq!(close_instant(at(15, 15), 60, CLOSE_FULL), Some(at(15, 30)));
    assert_eq!(close_instant(at(15, 30), 1, CLOSE_FULL), None);
    assert_eq!(close_instant(at(15, 6), 1, CLOSE_FULL), Some(at(15, 7)));
    assert_eq!(close_instant(at(15, 7), 1, CLOSE_FULL), Some(ENTRY_CUT));
    // combo: a 3-minute bucket at 15:06 closes at 15:09, cut
    assert_eq!(close_instant(at(15, 6), 3, CLOSE_FULL), Some(at(15, 9)));
    // combo: an auction-eligible 15-minute bucket at 15:00
    assert_eq!(
        close_instant(at(15, 0), 15, CLOSE_AUCTION),
        Some(at(15, 15))
    );
    // oob: off-session and k = 0
    assert_eq!(close_instant(at(9, 0), 1, CLOSE_FULL), None);
    assert_eq!(close_instant(OPEN, 0, CLOSE_FULL), None);
    assert_eq!(close_instant(at(15, 29), 1, CLOSE_FULL), Some(CLOSE_FULL));
}

/// X-F09: auction eligibility at its edges.
#[test]
fn x_f09_auction_eligible_extremes() {
    let last = |hh: u32, mm: u32, v: i64| {
        vec![
            minute(0, 100, 10),
            Minute {
                start: hh * 3_600 + mm * 60,
                bar: Bar::flat(100, v),
            },
        ]
    };
    assert_eq!(auction_eligible(day(2026, 7, 31), &last(15, 14, 10)), None);
    assert_eq!(
        auction_eligible(day(2026, 8, 3), &last(15, 14, 10)),
        Some(true)
    );
    assert_eq!(
        auction_eligible(day(2026, 8, 3), &last(15, 15, 10)),
        Some(false)
    );
    assert_eq!(
        auction_eligible(day(2026, 8, 3), &last(15, 29, 10)),
        Some(false)
    );
    assert_eq!(auction_eligible(day(2026, 8, 3), &[]), None);
    // combo: a padded last minute is still stored: 15:29 padded is not eligible
    assert_eq!(
        auction_eligible(day(2026, 8, 3), &last(15, 29, 0)),
        Some(false)
    );
    // combo: only 09:15 on the switch day: eligible
    assert_eq!(
        auction_eligible(day(2026, 8, 3), &[minute(0, 100, 10)]),
        Some(true)
    );
    // oob: a minute stored after 15:30
    assert_eq!(
        auction_eligible(day(2026, 8, 3), &last(15, 45, 10)),
        Some(false)
    );
    assert_eq!(
        session_close(day(2026, 8, 3), &[minute(0, 100, 10)]),
        CLOSE_AUCTION
    );
    assert_eq!(
        session_close(day(2018, 9, 4), &[minute(0, 100, 10)]),
        CLOSE_FULL
    );
}

#[test]
fn x_f07_the_fold_refuses_off_grid_and_repeated_stamps() {
    let d = day(2018, 9, 4);
    assert_eq!(check_minutes(d, &full_day(375)), Ok(()));
    let mut off = full_day(3);
    off[1].start += 30;
    assert_eq!(
        check_minutes(d, &off),
        Err(FoldError::OffGrid(d, OPEN + 90))
    );
    let mut twice = full_day(3);
    twice[2].start = twice[1].start;
    assert_eq!(
        check_minutes(d, &twice),
        Err(FoldError::Duplicate(d, OPEN + 60))
    );
    let mut back = full_day(3);
    back[2].start = OPEN;
    assert_eq!(check_minutes(d, &back), Err(FoldError::Duplicate(d, OPEN)));
}

/// P-F07-1 and P-F10-1: generated days against a naive grouping model.
#[test]
fn p_f07_fold_vs_bucket_grouping_model() {
    let d = day(2018, 9, 4);
    for seed in crate::generator::SEEDS {
        let mut r = SplitMix64::new(seed);
        for _ in 0..250 {
            let k = TIMEFRAMES[usize::try_from(r.below(6)).unwrap()];
            let mut minutes = Vec::new();
            for i in 0..375u32 {
                if r.chance(1, 20) {
                    continue;
                }
                let p = 1_000 + i64::try_from(r.below(50)).unwrap();
                let v = if r.chance(1, 10) { 0 } else { 5 };
                minutes.push(Minute {
                    start: OPEN + 60 * i,
                    bar: Bar::new(p, p + 3, p - 3, p + 1, v),
                });
            }
            let got = fold_day(d, &minutes, k, false, &mut FoldCounts::default());
            let mut want = Vec::new();
            for b in 0..375u32.div_ceil(k) {
                let (s, e) = (OPEN + 60 * k * b, (OPEN + 60 * k * (b + 1)).min(CLOSE_FULL));
                let inside: Vec<&Minute> = minutes
                    .iter()
                    .filter(|m| m.start >= s && m.start < e)
                    .collect();
                let hole = u32::try_from(inside.len()).unwrap() * 60 < e - s;
                let traded: Vec<&&Minute> = inside.iter().filter(|m| m.bar.volume > 0).collect();
                let bar = traded.first().map(|f| Bar {
                    open: f.bar.open,
                    high: traded.iter().map(|m| m.bar.high).max().unwrap(),
                    low: traded.iter().map(|m| m.bar.low).min().unwrap(),
                    close: traded.last().unwrap().bar.close,
                    volume: traded.iter().map(|m| m.bar.volume).sum(),
                });
                if hole || bar.is_some() {
                    want.push(Bucket {
                        day: d,
                        start: s,
                        t: e,
                        bar,
                        hole,
                    });
                }
            }
            assert_eq!(got, want);
        }
    }
}
