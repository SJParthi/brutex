//! `CLAUDE.md` §3 rule 7 on the execution clock: appending FUTURE bars must not
//! change any trade, outcome or square-off already decided by bars `0..=N`.
//!
//! Generated OHLCV fixtures; no market execution or profit assurance.

#![allow(
    clippy::expect_used,
    reason = "fixture failures name the broken premise"
)]

use costs::fill::Direction;
use indicators::Candle;
use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::outcome::{Horizon, SessionBounds, forward};
use runner::trade::{SliceFacts, walk};
use vocab::ConditionMask;

const PREFIX_SESSIONS: i64 = 8;
const BARS_PER_SESSION: usize = 375;

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

fn horizon(bars: u32) -> Horizon {
    Horizon::bars(bars).expect("a non-zero horizon")
}

/// Eight one-minute sessions, then forty LATER sessions that keep only every
/// other minute. The future holds more same-day steps than the past, and every
/// one of them is two minutes long, so a cadence measured over the WHOLE slice
/// is decided by bars that have not happened yet at any prefix bar.
fn prefix_and_extended() -> (Vec<Candle>, Vec<Candle>) {
    let all = runner::synthetic::sessions(PREFIX_SESSIONS + 40);
    let cut = usize::try_from(PREFIX_SESSIONS).expect("small") * BARS_PER_SESSION;
    let prefix = all.get(..cut).expect("cut is inside").to_vec();
    let mut extended = prefix.clone();
    extended.extend(
        all.iter()
            .skip(cut)
            .enumerate()
            .filter(|(index, _)| index % 2 == 0)
            .map(|(_, bar)| *bar),
    );
    (prefix, extended)
}

fn columns(prefix: &[Candle], extended: &[Candle]) -> (Column, Column) {
    let past = Column::build(prefix, &mut evaluator());
    let whole = Column::build(extended, &mut evaluator());
    assert!(!past.sources().is_empty(), "the prefix must pass warm-up");
    assert_eq!(
        whole.sources().get(..past.sources().len()),
        Some(past.sources()),
        "premise: the column itself is prefix-stable (indicators' own rule 7 test)"
    );
    (past, whole)
}

#[test]
fn appending_future_bars_changes_no_completed_trade() {
    let (prefix, extended) = prefix_and_extended();
    let (past, whole) = columns(&prefix, &extended);
    for direction in [Direction::Long, Direction::Short] {
        for h in [1, 15, 60] {
            let before = walk(&prefix, &past, &ConditionMask::ZERO, horizon(h), direction);
            let after = walk(
                &extended,
                &whole,
                &ConditionMask::ZERO,
                horizon(h),
                direction,
            );
            assert!(
                !before.trades.is_empty(),
                "the prefix must trade, or the comparison below is vacuous"
            );
            let kept: Vec<_> = after
                .trades
                .iter()
                .filter(|trade| trade.exit_bar < prefix.len())
                .copied()
                .collect();
            assert_eq!(
                kept, before.trades,
                "h={h} {direction:?}: a trade finished inside bars 0..N changed when later bars were appended"
            );
        }
    }
}

#[test]
fn appending_future_bars_changes_no_measured_outcome() {
    let (prefix, extended) = prefix_and_extended();
    let (past, whole) = columns(&prefix, &extended);
    for h in [1, 15, 60] {
        let before = forward(&prefix, &past, horizon(h));
        let after = forward(&extended, &whole, horizon(h));
        let mut measured = 0_usize;
        for i in 0..prefix.len() {
            let Some(move_paisa) = before.at(i) else {
                continue;
            };
            measured += 1;
            assert_eq!(after.at(i), Some(move_paisa), "h={h} bar {i}: return");
            assert_eq!(after.adverse_at(i), before.adverse_at(i), "h={h} bar {i}");
            assert_eq!(
                after.favourable_at(i),
                before.favourable_at(i),
                "h={h} bar {i}"
            );
        }
        assert!(
            measured > 1_000,
            "h={h}: the prefix must measure outcomes, got {measured}"
        );
    }
}

#[test]
fn appending_future_bars_changes_no_square_off_or_session_bound() {
    let (prefix, extended) = prefix_and_extended();
    let (past, whole) = columns(&prefix, &extended);

    let early = SessionBounds::of(&prefix);
    let late = SessionBounds::of(&extended);
    assert!(
        (0..prefix.len()).any(|i| early.day_ended(i)),
        "the prefix must prove at least one real square-off"
    );
    for i in 0..prefix.len() {
        assert_eq!(late.fillable(i), early.fillable(i), "bar {i}: fillable");
        assert_eq!(late.day_ended(i), early.day_ended(i), "bar {i}: day_ended");
        assert_eq!(late.last_fill_bar(i), early.last_fill_bar(i), "bar {i}");
    }

    let small = SliceFacts::of(&prefix, &past);
    let large = SliceFacts::of(&extended, &whole);
    for i in 0..prefix.len() {
        assert_eq!(large.step_at(i), small.step_at(i), "bar {i}: cadence");
        for to in [i, i.saturating_add(15).min(prefix.len() - 1)] {
            assert_eq!(
                large.path_accepts(i, to),
                small.path_accepts(i, to),
                "path {i}..={to}"
            );
        }
    }
}

/// The same law when the prefix ends MID-SESSION: the rest of that day, and
/// every later day, arrive afterwards. A trade or outcome the truncated slice
/// already completed may not move; only the tail it refused may be filled in.
#[test]
fn a_prefix_cut_mid_session_keeps_every_completed_trade_and_outcome() {
    let (whole_days, extended) = prefix_and_extended();
    for cut in [whole_days.len() - 200, whole_days.len() - 374, 7 * 375 + 1] {
        let prefix = whole_days.get(..cut).expect("cut is inside").to_vec();
        let (past, whole) = columns(&prefix, &extended);
        for h in [1, 15, 60] {
            let before = walk(
                &prefix,
                &past,
                &ConditionMask::ZERO,
                horizon(h),
                Direction::Long,
            );
            let after = walk(
                &extended,
                &whole,
                &ConditionMask::ZERO,
                horizon(h),
                Direction::Long,
            );
            assert!(!before.trades.is_empty(), "cut {cut} h={h}: vacuous");
            let kept: Vec<_> = after
                .trades
                .iter()
                .filter(|trade| trade.exit_bar < prefix.len())
                .copied()
                .collect();
            assert_eq!(kept, before.trades, "cut {cut} h={h}");

            let early = forward(&prefix, &past, horizon(h));
            let late = forward(&extended, &whole, horizon(h));
            for i in 0..prefix.len() {
                if let Some(move_paisa) = early.at(i) {
                    assert_eq!(late.at(i), Some(move_paisa), "cut {cut} h={h} bar {i}");
                }
            }
        }
    }
}
