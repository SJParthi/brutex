#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

/// T-F33-4: side x across x band x hole (the time half is the walk's, F39).
#[test]
fn t_f33_every_trailing_permutation() {
    for side in Side::BOTH {
        let long = side == Side::Long;
        for across_now in [true, false] {
            for (band_vs_stop, mapped) in [(1, 101), (0, 100), (-1, 99)] {
                for hole in [true, false] {
                    let close = if across_now == long { 90 } else { 110 };
                    let band = 100;
                    let level = if long { mapped } else { 200 - mapped };
                    let got = step(side, close, band, hole, Some(level), 100);
                    let want = if hole {
                        Step::Hold
                    } else if across_now {
                        Step::Flip
                    } else if band_vs_stop == 1 {
                        Step::Move(level)
                    } else {
                        Step::Hold
                    };
                    assert_eq!(got, want, "{side:?} across {across_now} {band_vs_stop} hole {hole}");
                }
            }
        }
    }
}

/// X-F33: the trailing rule at its edges.
#[test]
fn x_f33_trailing_extremes() {
    // equal band: not tighter, no move
    assert_eq!(step(Side::Long, 105, 100, false, Some(100), 100), Step::Hold);
    // the close exactly on the band is not across
    assert!(!across(100, 100, Side::Long) && !across(100, 100, Side::Short));
    // combo: across and tighter at once: the flip wins
    assert_eq!(step(Side::Long, 90, 100, false, Some(150), 100), Step::Flip);
    // a mapping failure: the stop stays, counted
    assert_eq!(step(Side::Short, 90, 100, false, None, 100), Step::NoRatio);
    // combo: a hole that is also across: nothing
    assert_eq!(step(Side::Short, 120, 100, true, Some(50), 100), Step::Hold);
    // oob: a long's new stop above the market and above its entry is still a move
    assert_eq!(step(Side::Long, 101, 100, false, Some(1_000), 100), Step::Move(1_000));
    assert!(tighter(101, 100, Side::Long) && !tighter(100, 100, Side::Long));
    assert!(tighter(99, 100, Side::Short) && !tighter(101, 100, Side::Short));
    // an i128 band beyond every close
    assert!(across(i64::MAX, i128::MAX, Side::Long) && across(i64::MIN, -i128::MAX, Side::Short));
}
