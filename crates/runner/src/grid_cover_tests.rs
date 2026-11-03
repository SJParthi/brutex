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

/// BASE-COV-LIBS-LS2: A TARGET LADDER TOO WIDE FOR THE CELL BUDGET IS THINNED
/// BY EVEN STRIDE, keeping its tightest rung and wide ones; a ladder of two
/// rungs, a grid inside the budget, and a grid just past it whose integer
/// walk keeps every rung are each returned whole.
#[test]
fn a_target_ladder_past_the_cell_budget_is_thinned_by_even_stride() {
    let ladder = |rungs: i64| Ladder::new((1..=rungs).map(|r| r * 1_000).collect()).unwrap();
    // (10+1) x (201 x 11 + 20100 x 55) cells: far past 24,000. The quadratic
    // estimate keeps 8 rungs, so the stride is ceil(200 / 8) = 25.
    let thinned = thin_to_budget(ladder(200), 10, 10);
    let want: Vec<Ppm> = (0..8).map(|i| (1 + 25 * i) * 1_000).collect();
    assert_eq!(thinned.rungs(), want.as_slice());
    assert!(variants(10, thinned.rungs().len(), 10) < variants(10, 200, 10));
    // 126 x 3 + 7875 x 3 = 24,003 cells, one stop: past the budget, yet
    // T(T+1)/2 <= 8000 holds for every one of the 125 rungs.
    assert!(variants(0, 125, 2) > 24_000);
    for (targets, stops, trails) in [
        (ladder(2), 1000, 1000),
        (ladder(10), 1, 1),
        (ladder(125), 0, 2),
    ] {
        assert_eq!(thin_to_budget(targets.clone(), stops, trails), targets);
    }
}

/// A plain swept column over `sessions` generated sessions.
fn swept(sessions: i64) -> (Vec<Candle>, Column) {
    let bars = crate::synthetic::sessions(sessions);
    let mut evaluator = indicators::evaluator::Evaluator::new(
        indicators::evaluator::Widths::pinned().expect("pinned widths are valid"),
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
    );
    let column = Column::build(&bars, &mut evaluator);
    (bars, column)
}

/// THE CONVENIENCE DOORS ANSWER WHAT THE HOISTED DOORS DO: `evaluate_with`
/// derives the slice facts and gives `evaluate_over`'s grid whether or not a
/// square-off table is passed; `with_levels_over` gives `with_levels`'s cell.
#[test]
fn the_convenience_doors_give_the_hoisted_doors_answers() {
    let (bars, column) = swept(8);
    let facts = crate::trade::SliceFacts::of(&bars, &column);
    let horizon = Horizon::bars(15).expect("a nonzero horizon");
    let mask = ConditionMask::default();
    for side in [Side::Long, Side::Short] {
        let over = evaluate_over(
            &bars,
            &column,
            &mask,
            horizon,
            side,
            Levels::derived(3),
            &facts,
        );
        assert!(!over.cells.is_empty(), "{side:?}");
        for exits in [None, Some(facts.exits())] {
            assert_eq!(
                evaluate_with(
                    &bars,
                    &column,
                    &mask,
                    horizon,
                    side,
                    Levels::derived(3),
                    exits
                ),
                over,
                "{side:?}"
            );
        }
        let ladders = Ladders {
            stops: &over.stops,
            targets: &over.targets,
            trails: &over.trails,
        };
        for cell in over.cells.iter().take(6) {
            let chosen = Chosen::from_cell(cell);
            assert_eq!(
                with_levels_over(
                    &bars, &column, &mask, horizon, side, ladders, chosen, &facts
                ),
                with_levels(&bars, &column, &mask, horizon, side, ladders, chosen),
                "{side:?} {chosen:?}"
            );
        }
    }
}

/// A REPLAYED PRICE AND CANDIDATE READ BACK THE FIELDS THEY WERE SEALED WITH.
#[test]
fn a_replayed_price_and_candidate_read_back_their_fields() {
    let row = TradeRow {
        signal_bar: 3,
        entry_bar: 4,
        exit_bar: 9,
        best: 10,
        worst: -20,
        entry_micros: 240,
        exit_micros: 540,
        adverse: 7,
        adverse_paisa: 70,
        favourable: 8,
        favourable_paisa: 80,
    };
    let price = ReplayPriceV1 {
        row,
        ambiguous_bars: 1,
        gap_fills: 0,
    };
    assert_eq!(
        (price.row(), price.ambiguous_bars(), price.gap_fills()),
        (row, 1, 0)
    );
    let candidate = ReplayCandidateV1 {
        signal_bar: 3,
        entry_bar: 4,
        occupied_through_bar: 9,
        signal_micros: 180,
        entry_micros: 240,
        occupied_through_micros: 540,
        path: ReplayPathV1::Priceable(price),
    };
    assert_eq!(
        (candidate.signal_bar(), candidate.signal_micros()),
        (3, 180)
    );
}

/// One-minute bars at minutes 0..4, the second dipping 2% (a stop at 1% is
/// crossed there).
fn four_bars() -> Vec<Candle> {
    [
        (2_500_000, 2_500_100, 2_499_900, 2_500_000),
        (2_500_000, 2_500_100, 2_450_000, 2_460_000),
        (2_460_000, 2_460_100, 2_459_900, 2_460_000),
        (2_460_000, 2_460_100, 2_459_900, 2_460_000),
    ]
    .iter()
    .zip(0_i64..)
    .map(|(&(open, high, low, close), minute)| {
        Candle::new(
            minute * 60_000_000,
            open,
            high,
            low,
            close,
            100,
            indicators::OI_NULL,
        )
    })
    .collect()
}

/// A long replay candidate over `bars`, every bar accepted.
fn long_candidate(
    bars: &[Candle],
    (signal, entry, time_exit): (usize, usize, usize),
    ladders: Ladders<'_>,
) -> Candidate {
    let (entry_pess, entry_opt) = entry_fills(bars, entry, Side::Long);
    Candidate {
        signal,
        entry,
        time_exit,
        block_only: false,
        hole: None,
        entry_pess,
        entry_opt,
        cross: crossings_checked(
            bars,
            entry,
            time_exit,
            entry_opt,
            Side::Long,
            ladders,
            &vec![true; bars.len()],
        ),
    }
}

/// EVERY REPLAY RECONCILIATION A CALLER'S CANDIDATES CAN BREAK IS REFUSED:
/// entries that repeat, a signal or entry past the slice, a refused path whose
/// occupancy is past the slice or ends before its entry or that carries no
/// refusal fact at all, a priced path that does not price to one row, and an
/// entry instant that is not its bar's.
#[test]
fn every_replay_reconciliation_a_candidate_can_break_is_refused() {
    let bars = four_bars();
    let (stops, targets, trails) = (
        Ladder::new(vec![10_000]).unwrap(),
        Ladder::new(vec![900_000]).unwrap(),
        Ladder::new(vec![900_000]).unwrap(),
    );
    let ladders = Ladders {
        stops: &stops,
        targets: &targets,
        trails: &trails,
    };
    let rungs = (stops.rungs(), targets.rungs(), trails.rungs());
    let baseline = Variant {
        stop: None,
        target: None,
        tsl: None,
        ttp: None,
    };
    let clean = || long_candidate(&bars, (0, 1, 3), ladders);
    let refused = |why| crate::exit_grid_policy::ExitGridErrorV1::ReplayEvidenceRefused(why);
    let universe =
        replay_universe_from_candidates_v1(&bars, &[clean()], rungs, baseline, Side::Long)
            .expect("one clean candidate replays");
    assert_eq!(universe.candidates.len(), 1);
    assert_eq!(
        replay_universe_from_candidates_v1(&bars, &[clean(), clean()], rungs, baseline, Side::Long)
            .map(|_| ()),
        Err(refused("candidate entries are duplicated or out of order"))
    );
    assert_eq!(
        replay_candidate_stamps_v1(
            &bars,
            &Candidate {
                signal: 9,
                ..clean()
            }
        ),
        Err(refused(
            "candidate signal coordinate is outside the execution slice"
        ))
    );
    assert_eq!(
        replay_candidate_stamps_v1(
            &bars,
            &Candidate {
                entry: 9,
                ..clean()
            }
        ),
        Err(refused(
            "candidate entry coordinate is outside the execution slice"
        ))
    );
    let entry_micros = bars[1].ts_micros;
    let block_only = || Candidate {
        block_only: true,
        ..clean()
    };
    assert_eq!(
        refused_candidate_path_v1(
            &bars,
            &Candidate {
                time_exit: 9,
                ..block_only()
            },
            entry_micros
        ),
        Err(refused(
            "block-only occupied-through coordinate is outside the execution slice"
        ))
    );
    assert_eq!(
        refused_candidate_path_v1(
            &bars,
            &Candidate {
                time_exit: 0,
                ..block_only()
            },
            entry_micros
        ),
        Err(refused("block-only occupancy ends before its entry"))
    );
    // COMBINED: the right occupancy, and an entry instant after it.
    assert_eq!(
        refused_candidate_path_v1(&bars, &block_only(), bars[3].ts_micros + 1),
        Err(refused("block-only occupancy ends before its entry"))
    );
    assert_eq!(
        refused_candidate_path_v1(&bars, &clean(), entry_micros),
        Err(refused("pricing-refusal branch carried no refusal fact"))
    );
    let mut row = Vec::new();
    assert_eq!(
        priceable_candidate_path_v1(
            &bars,
            &block_only(),
            rungs,
            baseline,
            Side::Long,
            entry_micros,
            &mut row
        ),
        Err(refused(
            "a priceable candidate did not replay to exactly one row"
        ))
    );
    assert_eq!(
        priceable_candidate_path_v1(
            &bars,
            &clean(),
            rungs,
            baseline,
            Side::Long,
            entry_micros + 1,
            &mut row
        ),
        Err(refused(
            "chosen candidate row does not reconcile with its occupancy path"
        ))
    );
    assert!(
        priceable_candidate_path_v1(
            &bars,
            &clean(),
            rungs,
            baseline,
            Side::Long,
            entry_micros,
            &mut row
        )
        .is_ok()
    );
    // Each refusal also leaves the doors that call the two path readers.
    assert_eq!(
        replay_candidate_path_v1(
            &bars,
            &clean(),
            rungs,
            baseline,
            Side::Long,
            entry_micros + 1,
            &mut row
        )
        .map(|_| ()),
        Err(refused(
            "chosen candidate row does not reconcile with its occupancy path"
        ))
    );
    assert_eq!(
        replay_universe_from_candidates_v1(
            &bars,
            &[Candidate {
                time_exit: 0,
                ..block_only()
            }],
            rungs,
            baseline,
            Side::Long
        )
        .map(|_| ()),
        Err(refused("block-only occupancy ends before its entry"))
    );
}
