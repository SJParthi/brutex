#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "tests state their expectations: a fixture bar is read by its fixed \
              position, and one refusal table reads as one list"
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

/// BASE-COV-LIBS-LS2: A CELL THAT NAMES NO TRADES NEVER CLEARS, whatever its
/// other counters say: a torn cell claiming a win over zero trades fails even
/// the lowest thresholds, and the same win over one trade clears zero ones.
#[test]
fn a_cell_naming_no_trades_never_clears_even_the_lowest_thresholds() {
    let torn = cell(0, 1);
    assert!(!torn.clears(0, 0));
    assert!(!torn.clears(i64::MIN, i64::MIN));
    assert!(cell(1, 1).clears(0, 0));
}

/// BASE-COV-LIBS-LS2: AN EMPTY GRID HAS NO BEST CELL TO MATERIALIZE, and
/// answers no rows rather than a refusal or a fabricated cell, on either side.
#[test]
fn an_empty_grid_materializes_no_rows() {
    let (bars, column) = swept(2);
    let horizon = Horizon::bars(15).expect("a nonzero horizon");
    for side in [Side::Long, Side::Short] {
        assert_eq!(
            materialize_best(
                &bars,
                &column,
                &ConditionMask::default(),
                horizon,
                side,
                &Grid::default()
            ),
            Ok(Vec::new()),
            "{side:?}"
        );
    }
}

/// BASE-COV-LIBS-LS2: A STEPPED GRID WITH NO CALLER STOP LADDER STEPS ITS
/// STOPS AND ITS TRAILS, with the operator's forced level merged into the
/// stepped stops (the target stays the one far target); a step that can build
/// no rung (zero rungs) falls back to the quantile derivation, which at zero
/// rungs places none.
#[test]
fn a_stepped_grid_steps_its_own_stops_and_merges_the_forced_level() {
    let (bars, column) = swept(8);
    let horizon = Horizon::bars(15).expect("a nonzero horizon");
    let mask = ConditionMask::default();
    for side in [Side::Long, Side::Short] {
        let stepped = evaluate(
            &bars,
            &column,
            &mask,
            horizon,
            side,
            Levels {
                rungs: 3,
                step_ppm: Some(25),
                stops_ppm: &[],
                forced: Some(30),
                ratios: false,
            },
        );
        assert_eq!(stepped.stops.rungs(), &[25, 30, 50, 75], "{side:?}");
        assert_eq!(stepped.trails.rungs(), &[25, 50, 75], "{side:?}");
        assert!(!stepped.cells.is_empty(), "{side:?}");

        let unbuildable = evaluate(
            &bars,
            &column,
            &mask,
            horizon,
            side,
            Levels {
                rungs: 0,
                step_ppm: Some(25),
                stops_ppm: &[],
                forced: None,
                ratios: false,
            },
        );
        let derived = evaluate(&bars, &column, &mask, horizon, side, Levels::derived(0));
        assert_eq!(unbuildable.targets, derived.targets, "{side:?}");
        assert_eq!(unbuildable.trails, derived.trails, "{side:?}");
        assert!(unbuildable.trails.rungs().is_empty(), "{side:?}");
    }
}

/// BASE-COV-LIBS-LS2 (5 Nov): THE RESOLVED-POLICY DOOR REFUSES PRICES ITS
/// MONEY ENVELOPE CANNOT HOLD, and says so as the population it could not
/// enumerate. Unscaled, the door prices one cell per resolved coordinate;
/// with every price scaled so four times the largest price on any path passes
/// `i64::MAX`, the exact-ladder pass refuses before pricing (no cell), and the
/// door answers the resolution's cell count against the zero it got. (The
/// derived door and the replay doors refuse the same prices in
/// `a_grid_that_could_saturate_is_refused_not_clamped`; this door checks the
/// envelope itself.)
#[test]
fn the_resolved_policy_door_refuses_prices_past_the_money_envelope() {
    use crate::exit_grid_policy::{
        ExecutionResolutionV1, ExecutionSeriesV1, ExitGridErrorV1, ExitGridPolicyV1,
        ExitGridSelectorV1, ForcedStopV1, RangeResolutionV1, RatioLimitsV1, RationalPercentileV1,
        RungPlanV1, printed_ohlcv_cost_model_id_v2,
    };
    let (bars, column) = swept(8);
    let horizon = Horizon::bars(15).expect("a nonzero horizon");
    let mask = ConditionMask::default();
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .expect("an allowed index");
    let middle = RationalPercentileV1::new(1, 2).expect("one half");
    let resolved = ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        Side::Long,
        RungPlanV1::new(vec![middle], vec![middle], vec![middle], 1).expect("one rung each"),
        RatioLimitsV1::new(1, 100_000, 1).expect("a broad single pair"),
        32,
        ExitGridSelectorV1::PessimisticTotal,
        printed_ohlcv_cost_model_id_v2(),
        ForcedStopV1::Disabled,
        u64::MAX,
        u64::MAX,
    )
    .expect("a valid policy")
    .resolve_attested(
        ExecutionSeriesV1::new(&key, "test-feed", "test-commit", [0xA5; 32], &bars)
            .expect("complete identity"),
    )
    .expect("the grid resolves");
    let facts = crate::trade::SliceFacts::of(&bars, &column);
    let priced = evaluate_resolved_policy_v1(
        &bars,
        &column,
        &mask,
        horizon,
        Side::Long,
        &resolved,
        &facts,
    )
    .expect("the whole population prices");
    assert_eq!(priced.cells.len() as u64, resolved.cell_count());
    assert!(priced.cells.iter().any(|cell| cell.trades > 0));

    let smallest = bars
        .iter()
        .map(|bar| bar.low.min(bar.open).min(bar.close))
        .min()
        .expect("bars");
    let scale = i64::MAX / 4 / smallest + 1;
    let scaled: Vec<Candle> = bars
        .iter()
        .map(|bar| Candle {
            open: bar.open * scale,
            high: bar.high * scale,
            low: bar.low * scale,
            close: bar.close * scale,
            ..*bar
        })
        .collect();
    let scaled_facts = crate::trade::SliceFacts::of(&scaled, &column);
    assert_eq!(
        evaluate_resolved_policy_v1(
            &scaled,
            &column,
            &mask,
            horizon,
            Side::Long,
            &resolved,
            &scaled_facts
        ),
        Err(ExitGridErrorV1::ReplayCellCountMismatch {
            expected: resolved.cell_count(),
            actual: 0,
        })
    );
}

/// One plain bar at the epoch: open and close 100,000, high 120,000, low
/// 80,000.
fn wide_bar() -> Candle {
    Candle {
        open: 100_000,
        high: 120_000,
        low: 80_000,
        close: 100_000,
        ..Candle::default()
    }
}

/// BASE-COV-LIBS-LS2 (5 Nov): A LEVEL OR TRAIL EXIT WHOSE BAR IS NOT IN THE
/// SLICE IS BOOKED FLAT, never invented (an empty slice, and an offset one past
/// the only bar), on both sides; and a trail with no recorded peak, which
/// `Trailing::ended` answers with an anchor of zero, is priced as the market
/// order a time exit is, on the bar it fired on, under both readings.
#[test]
fn an_exit_off_the_slice_is_flat_and_a_peakless_trail_is_a_market_order() {
    let fills = EntryPrices {
        charged: 100_000,
        anchor: 100_000,
    };
    let bar = wide_bar();
    for side in [Side::Long, Side::Short] {
        for (by, kind) in [
            (Ended::Stop, Resting::Stop),
            (Ended::Target, Resting::Target),
        ] {
            let level = Some(Level { kind, ppm: 100_000 });
            for (slice, offset) in [(&[][..], 0), (core::slice::from_ref(&bar), 1)] {
                let flat = realised(slice, 0, offset, fills, side, by, level, true);
                assert_eq!((flat.paisa, flat.gapped), (0, false), "{side:?} {by:?}");
            }
        }
        let trail = Ended::Trail {
            anchor: 105_000,
            ppm: 100_000,
            kind: TrailKind::Live,
            rested_at_open: true,
        };
        for (slice, offset) in [(&[][..], 0), (core::slice::from_ref(&bar), 1)] {
            let flat = realised(slice, 0, offset, fills, side, trail, None, true);
            assert_eq!((flat.paisa, flat.gapped), (0, false), "{side:?}");
        }
        for kind in [TrailKind::Live, TrailKind::Armed] {
            let fired = Trailing {
                kind,
                at: 0,
                before: None,
                raised: None,
                ppm: 100_000,
            };
            for pessimistic in [true, false] {
                let peakless = fired.ended(pessimistic);
                assert_eq!(
                    peakless,
                    Ended::Trail {
                        anchor: 0,
                        ppm: 100_000,
                        kind,
                        rested_at_open: pessimistic,
                    }
                );
                let as_trail = realised(
                    core::slice::from_ref(&bar),
                    0,
                    0,
                    fills,
                    side,
                    peakless,
                    None,
                    pessimistic,
                );
                let as_time = realised(
                    core::slice::from_ref(&bar),
                    0,
                    0,
                    fills,
                    side,
                    Ended::Time,
                    None,
                    pessimistic,
                );
                assert_eq!(
                    (as_trail.paisa, as_trail.gapped),
                    (as_time.paisa, false),
                    "{side:?} {kind:?} {pessimistic}"
                );
            }
        }
        // The market-order bracket itself: the worst reading is the bar's
        // adverse extreme, the best its favourable one.
        let worst = realised(
            core::slice::from_ref(&bar),
            0,
            0,
            fills,
            side,
            Ended::Time,
            None,
            true,
        );
        let best = realised(
            core::slice::from_ref(&bar),
            0,
            0,
            fills,
            side,
            Ended::Time,
            None,
            false,
        );
        assert!(worst.paisa < best.paisa, "{side:?}");
    }
}

/// BASE-COV-LIBS-LS2 (5 Nov): WHEN BOTH TRAILS HAVE FIRED BY THE CHOSEN EXIT,
/// THE EARLIER ONE IS THE EXIT: the live trail when it fired first, the armed
/// one when it fired first, and the armed one on a tie (the order the arms
/// state). The chosen trail's own peaks are what both readings carry.
#[test]
fn of_two_fired_trails_the_earlier_one_exits() {
    let trailing = |kind, at, before, raised| Trailing {
        kind,
        at,
        before: Some(before),
        raised: Some(raised),
        ppm: 1_000,
    };
    let none_fixed = |live, armed| {
        ExitChoices::of(
            Firing {
                stop_at: NEVER,
                target_at: NEVER,
                live,
                armed,
            },
            9,
            false,
        )
    };
    for (live_at, armed_at, live_wins) in [(2, 5, true), (5, 2, false), (4, 4, false)] {
        let live = trailing(TrailKind::Live, live_at, 104_000, 104_500);
        let armed = trailing(TrailKind::Armed, armed_at, 106_000, 107_000);
        let chosen = if live_wins { live } else { armed };
        let choices = none_fixed(live, armed);
        assert_eq!(
            choices.trail_before,
            Some(chosen.ended(true)),
            "{live_at} {armed_at}"
        );
        assert_eq!(choices.trail_raised, Some(chosen.ended(false)));
        assert_eq!(
            (choices.stop, choices.target, choices.timed),
            (None, None, None)
        );
    }
    // A trail that fired after the chosen exit is not one of them.
    let late = none_fixed(
        trailing(TrailKind::Live, 2, 104_000, 104_000),
        trailing(TrailKind::Armed, 11, 106_000, 107_000),
    );
    assert_eq!(
        late.trail_before,
        Some(trailing(TrailKind::Live, 2, 104_000, 104_000).ended(true))
    );
    assert_eq!(late.trail_raised, None, "an unraised peak reads once");
}

/// BASE-COV-LIBS-LS2 (5 Nov): AN EXIT NAMES A LEVEL ONLY WHEN ITS VARIANT HAS
/// THAT RUNG: a stop exit with no stop rung and a target exit with no target
/// rung name none, whatever the other rung is, and a time or trail exit never
/// names one.
#[test]
fn an_exit_names_a_level_only_when_its_variant_has_that_rung() {
    assert_eq!(level_for(Ended::Stop, None, Some(5)), None);
    assert_eq!(level_for(Ended::Target, Some(5), None), None);
    assert_eq!(level_for(Ended::Stop, None, None), None);
    assert_eq!(level_for(Ended::Target, None, None), None);
    assert_eq!(
        level_for(Ended::Stop, Some(7), None),
        Some(Level {
            kind: Resting::Stop,
            ppm: 7
        })
    );
    assert_eq!(
        level_for(Ended::Target, None, Some(9)),
        Some(Level {
            kind: Resting::Target,
            ppm: 9
        })
    );
    assert_eq!(level_for(Ended::Time, Some(1), Some(2)), None);
    let trail = Ended::Trail {
        anchor: 1,
        ppm: 1,
        kind: TrailKind::Live,
        rested_at_open: false,
    };
    assert_eq!(level_for(trail, Some(1), Some(2)), None);
}

/// BASE-COV-LIBS-LS2 (5 Nov): A RESTING LEVEL OUTSIDE ITS BAR'S PRINTED RANGE
/// FILLS AT THE BAR'S OPEN AND IS COUNTED A GAP, below the low and above the
/// high alike, for every order kind and side; a level inside the range fills
/// at the level, at either edge of it, and is not a gap.
#[test]
fn a_resting_level_outside_its_bar_fills_at_the_open_as_a_gap() {
    let bar = wide_bar();
    for kind in [Resting::Stop, Resting::Target] {
        for side in [Side::Long, Side::Short] {
            for outside in [bar.low - 1, 60_000, bar.high + 1, 140_000] {
                assert_eq!(
                    level_fill(&bar, outside, kind, side, false),
                    (bar.open, true),
                    "{kind:?} {side:?} {outside}"
                );
            }
            for inside in [bar.low, 95_000, bar.high] {
                assert_eq!(
                    level_fill(&bar, inside, kind, side, false),
                    (inside, false),
                    "{kind:?} {side:?} {inside}"
                );
            }
        }
    }
}
