//! BASE-COV-LIBS: every admission field has its own wire name, in the
//! table's order, and no two fields share one.

use std::collections::BTreeSet;

use runner::admission::AdmissionFieldV1;

const NAMES: [&str; 39] = [
    "min_support_hits",
    "min_independent_sessions",
    "min_trades",
    "max_mae_paisa",
    "min_worst_reward_risk_ppm",
    "min_win_rate_ppm",
    "min_wilson_win_rate_ppm",
    "min_return_drawdown_ppm",
    "min_weakest_period_return_paisa",
    "max_pbo_ppm",
    "max_fwer_p_value_ppm",
    "max_spa_p_value_ppm",
    "min_decided_folds",
    "max_ambiguous_fill_rate_ppm",
    "max_gap_affected_rate_ppm",
    "max_session_concentration_ppm",
    "max_largest_trade_profit_share_ppm",
    "max_drawdown_paisa",
    "max_worst_trade_loss_paisa",
    "max_losing_trade_rate_ppm",
    "max_losing_trades",
    "min_pessimistic_profit_paisa",
    "min_winning_trades",
    "min_average_win_paisa",
    "max_average_loss_paisa",
    "min_profit_factor_ppm",
    "max_consecutive_losing_streak",
    "min_consecutive_winning_streak",
    "min_bootstrap_draws",
    "min_bootstrap_strategies",
    "min_bootstrap_periods",
    "min_pbo_contributing_folds",
    "max_pbo_unrankable_folds",
    "min_profitable_oos_folds",
    "min_oos_pessimistic_return_paisa",
    "max_white_reality_p_value_ppm",
    "require_white_reality_rejection",
    "max_romano_wolf_p_value_ppm",
    "require_romano_wolf_rejection",
];

#[test]
fn every_admission_field_has_its_own_wire_name_in_table_order() {
    let said: Vec<&str> = AdmissionFieldV1::ALL
        .iter()
        .map(|field| field.name())
        .collect();
    assert_eq!(said, NAMES);
    let distinct: BTreeSet<&str> = said.iter().copied().collect();
    assert_eq!(distinct.len(), AdmissionFieldV1::ALL.len());
    for name in said {
        assert!(
            name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'),
            "{name}"
        );
        assert!(
            name.starts_with("min_") || name.starts_with("max_") || name.starts_with("require_")
        );
    }
}

#[test]
fn an_execution_series_without_its_feed_commit_or_calendar_is_refused_by_name() {
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use runner::exit_grid_policy::{ExecutionSeriesV1, ExitGridErrorV1};

    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("a key");
    let bars = runner::synthetic::sessions(1);
    let refused = |feed: &str, commit: &str, calendar: [u8; 32]| {
        ExecutionSeriesV1::new(&key, feed, commit, calendar, &bars).err()
    };
    assert_eq!(
        refused("", "c", [7; 32]),
        Some(ExitGridErrorV1::MissingSeriesIdentity("feed"))
    );
    assert_eq!(
        refused("zerodha", "", [7; 32]),
        Some(ExitGridErrorV1::MissingSeriesIdentity("commit"))
    );
    assert_eq!(
        refused("zerodha", "c", [0; 32]),
        Some(ExitGridErrorV1::MissingSeriesIdentity("calendar"))
    );
    // Every term missing at once: the feed is named first.
    assert_eq!(
        refused("", "", [0; 32]),
        Some(ExitGridErrorV1::MissingSeriesIdentity("feed"))
    );
    // One non-zero byte is a calendar identity.
    let mut one = [0; 32];
    one[31] = 1;
    assert_eq!(refused("zerodha", "c", one), None);
}

#[test]
fn a_streamed_sweep_tallies_the_same_levels_as_the_retained_one() {
    use engine::Ladder;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use runner::Sweeper;

    let evaluator = || {
        Evaluator::new(
            Widths::pinned().expect("pinned widths"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    };
    let ladder = || {
        Ladder::with_min_hits(600)
            .with_ceiling(50_000)
            .with_support_lanes(1)
    };
    let bars = runner::synthetic::sessions(8);
    let column = Column::build(&bars, &mut evaluator());
    let retained = Sweeper::new(ladder()).run_prepared(&column);
    let streamed = Sweeper::new(ladder()).run_prepared_streamed(&column);
    assert_eq!(streamed.census, retained.census);
    assert_eq!(streamed.first_swept, retained.first_swept);
    assert_eq!(streamed.sweep.bars, retained.sweep.bars);
    assert_eq!(streamed.sweep.levels.len(), retained.sweep.levels.len());
    assert!(streamed.is_complete());
    assert!(retained.is_complete());
    // A column with no bar warmed is never a complete walk.
    let empty = Column::build(&[], &mut evaluator());
    let nothing = Sweeper::new(ladder()).run_prepared_streamed(&empty);
    assert!(!nothing.is_complete());
    assert_eq!(nothing.first_swept, None);
}

#[test]
fn a_placement_of_too_few_candidates_is_unrankable_and_counted_so() {
    use runner::pbo::{
        AnchoredWalkForwardBottomHalfRateV1, PlacementV1, anchored_walk_forward_bottom_half_rate_v1,
        place_v1,
    };

    assert_eq!(PlacementV1::unrankable(2), None);
    assert_eq!(PlacementV1::unrankable(usize::MAX), None);
    let one = PlacementV1::unrankable(1).expect("one candidate");
    let none = PlacementV1::unrankable(0).expect("no candidate");
    for lone in [one, none] {
        assert_eq!(lone.relative_twice(), None);
        assert_eq!(lone.relative_ppm(), None);
        assert!(!lone.bottom_half());
    }
    // The in-sample winner (index 1) comes last out of sample: bottom half.
    let worst = place_v1(&[1, 9, 3], &[5, 0, 4]).expect("ranked");
    assert!(worst.bottom_half());
    // Its best out-of-sample: top.
    let best = place_v1(&[1, 9, 3], &[0, 7, 4]).expect("ranked");
    assert!(!best.bottom_half());
    let rate = anchored_walk_forward_bottom_half_rate_v1(&[worst, one, best, none]);
    assert_eq!(rate.folds, 2);
    assert_eq!(rate.bottom_half_folds, 1);
    assert_eq!(rate.unrankable, 2);
    assert_eq!(rate.rate_ppm(), Some(500_000));
    assert!(rate.at_or_above_half());

    let of = |folds: usize, bottom_half_folds: usize| AnchoredWalkForwardBottomHalfRateV1 {
        folds,
        bottom_half_folds,
        ..AnchoredWalkForwardBottomHalfRateV1::default()
    };
    assert_eq!(of(0, 0).rate_ppm(), None);
    assert_eq!(of(4, 5).rate_ppm(), None);
    assert_eq!(of(4, 4).rate_ppm(), Some(1_000_000));
    for (folds, bottom, above) in [
        (0, 0, false),
        (4, 5, false),
        (4, 2, true),
        (4, 1, false),
        (5, 3, true),
        (5, 2, false),
        (1, 1, true),
        (usize::MAX, usize::MAX, true),
    ] {
        assert_eq!(of(folds, bottom).at_or_above_half(), above, "{folds} {bottom}");
    }
}

#[test]
fn a_placement_is_relative_only_among_two_or_more_countable_candidates() {
    use runner::pbo::Placement;

    let at = |candidates: usize, winner_rank: usize| {
        Placement {
            candidates,
            winner_rank,
        }
        .relative()
    };
    assert_eq!(at(0, 0), None);
    assert_eq!(at(1, 0), None);
    assert_eq!(at(2, 0), Some(0));
    assert_eq!(at(2, 1), Some(1_000_000));
    assert_eq!(at(3, 1), Some(500_000));
    // Counts no i64 can hold are no placement, never a wrapped figure.
    assert_eq!(at(usize::MAX, 0), None);
    assert_eq!(at(3, usize::MAX), None);
    // A rank past the last is answered as computed, saturating, never a panic.
    assert_eq!(at(2, 5), Some(5_000_000));
}
