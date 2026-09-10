//! Generated OHLCV fixtures only; no market or profitability evidence.
#![expect(
    clippy::unwrap_used,
    reason = "finite fixture invariants must fail loudly"
)]

use super::*;
use brutex_core::instrument::{Exchange, InstrumentKey};
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;

const LAST_DAY: i64 = 39;
const BASE: i64 = 100_000;

struct Fixture {
    bars: Vec<Candle>,
    signals: Vec<Candle>,
    column: Column,
    key: InstrumentKey,
    label: &'static str,
    signal_minute: usize,
}
impl Fixture {
    fn new(minutes: u32, signal_minute: usize) -> Self {
        let mut bars = crate::synthetic::sessions(LAST_DAY + 1);
        for bar in &mut bars {
            if indicators::ist_day(bar.ts_micros) == LAST_DAY {
                *bar = Candle::new(
                    bar.ts_micros,
                    BASE,
                    BASE + 10,
                    BASE - 10,
                    BASE,
                    1000,
                    indicators::OI_NULL,
                );
            }
        }
        let signal_time = stamp(signal_minute);
        let selected = bars
            .iter_mut()
            .find(|bar| bar.ts_micros == signal_time)
            .unwrap();
        selected.low = BASE - 1000;
        selected.high = BASE + 1000;
        let mut signals = if minutes == 1 {
            bars.clone()
        } else {
            crate::resample::resample(&bars, crate::resample::Period::minutes(minutes).unwrap())
        };
        let position = signals
            .iter()
            .position(|bar| bar.ts_micros == signal_time)
            .unwrap();
        signals.truncate(position + 1);
        let mut column = column(&signals);
        column.clear_before(position);
        assert!(
            column.sources().contains(&position),
            "fixture signal must have warmed: {minutes}min"
        );
        let label = match minutes {
            1 => "1min",
            2 => "2min",
            3 => "3min",
            5 => "5min",
            10 => "10min",
            15 => "15min",
            30 => "30min",
            60 => "60min",
            _ => unreachable!(),
        };
        Self {
            bars,
            signals,
            column,
            key: InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap(),
            label,
            signal_minute,
        }
    }
    fn source(&self) -> Source<'_> {
        Source {
            series: ExecutionSeriesV1::new(
                &self.key,
                "generated-test-feed",
                "generated-test-commit",
                [7; 32],
                &self.bars,
            )
            .unwrap(),
            signal_bars: &self.signals,
            signal_column: &self.column,
            minute_context: &self.bars,
            daily_reference: reference(),
            timeframe: self.label,
            params: Params {
                min_hits: 0,
                ceiling: 100_000,
                pair_budget: 100_000,
                policy: 1,
            },
            first_day: 0,
            last_day: LAST_DAY,
        }
    }
    fn prepared(&self) -> Prepared<'_> {
        Prepared::new(Policy::V1, self.source()).unwrap()
    }
    fn set(&mut self, minute: usize, open: i64, high: i64, low: i64, close: i64) {
        let bar = self
            .bars
            .iter_mut()
            .find(|bar| bar.ts_micros == stamp(minute))
            .unwrap();
        *bar = Candle::new(
            bar.ts_micros,
            open,
            high,
            low,
            close,
            1000,
            indicators::OI_NULL,
        );
    }
    fn evaluate(&self, side: Direction) -> Evaluation {
        self.prepared().evaluate(&always(), side, 100_000).unwrap()
    }
    fn all_later_signals(&mut self) {
        self.signals = self.bars.clone();
        self.column = column(&self.signals);
        let first = self
            .signals
            .iter()
            .position(|bar| bar.ts_micros == stamp(self.signal_minute))
            .unwrap();
        self.column.clear_before(first);
    }
}
fn column(bars: &[Candle]) -> Column {
    Column::build(
        bars,
        &mut Evaluator::new(
            Widths::pinned().unwrap(),
            Availability::Absent,
            Thresholds::CLASSICAL,
        ),
    )
}
fn reference() -> DailyReferenceBinding<'static> {
    DailyReferenceBinding {
        daily_bars: &[],
        eligibility: &[],
        schema: 1,
        eligibility_policy: 1,
        gap_overlay_policy: 1,
        excluded_ist_days: &[],
        daily_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
        minute_integrity: crate::identity::ReferenceIntegrity::UnverifiedNoReceipt,
        swept_series_calendar_policy: 1,
    }
}
fn stamp(minute: usize) -> i64 {
    LAST_DAY * DAY + crate::synthetic::IST_OPEN_UTC_MICROS + i64::try_from(minute).unwrap() * MINUTE
}
fn always() -> Expression {
    Expression::parse("0 | !0").unwrap()
}
fn exact_trade(result: &Evaluation) -> Trade {
    assert_eq!(result.trades().len(), 1);
    *result.trades().first().unwrap()
}
fn roundtrip(result: &Evaluation) {
    assert_eq!(
        Policy::decode(&result.policy().canonical_bytes()).unwrap(),
        Policy::V1
    );
    assert_eq!(
        Metrics::decode(&result.metrics().canonical_bytes()).unwrap(),
        result.metrics()
    );
    for row in result.trades() {
        assert_eq!(Trade::decode(&row.canonical_bytes()).unwrap(), *row);
    }
    for row in result.events() {
        assert_eq!(Event::decode(&row.canonical_bytes()).unwrap(), *row);
    }
    for row in result.periods() {
        assert_eq!(Period::decode(&row.canonical_bytes()).unwrap(), *row);
    }
    assert_eq!(
        result.digest(),
        observation_digest(
            result.source_id(),
            result.run_id(),
            result.truth(),
            result.metrics(),
            result.events(),
            result.trades(),
            result.periods()
        )
    );
    assert!(result.truth().reconciles());
    assert_eq!(result.events().len() as u64, result.truth().hits);
}

#[test]
fn all_eight_rungs_keep_original_signal_low_high_and_exact_next_minute_entry() {
    for minutes in [1, 2, 3, 5, 10, 15, 30, 60] {
        // 10:00 IST is a complete edge on each canonical resampling grid.
        let fixture = Fixture::new(minutes, 45);
        for side in [Direction::Long, Direction::Short] {
            let prepared = fixture.prepared();
            let result = prepared.evaluate(&always(), side, 100_000).unwrap();
            let row = exact_trade(&result);
            assert_eq!(
                row.entry_micros,
                stamp(45 + usize::try_from(minutes).unwrap())
            );
            assert_eq!(row.signal_micros, stamp(45));
            assert_eq!(row.signal_close_micros, row.entry_micros);
            assert_eq!(row.signal_bar, fixture.signals.len() as u64 - 1);
            assert_eq!(
                row.stop_paisa,
                if side == Direction::Long {
                    BASE - 1000
                } else {
                    BASE + 1000
                }
            );
            assert_eq!(row.entry_paisa, BASE);
            assert_eq!(row.exit_reason, ExitReason::Forced1510);
            assert_eq!(row.exit_bar_micros, stamp(354));
            assert_eq!(row.exit_from_micros, stamp(355));
            assert_eq!(row.exit_until_micros, stamp(355));
            assert_eq!(prepared.run_id(&always(), side).unwrap(), result.run_id());
            assert_eq!(result.timeframe(), fixture.label);
            roundtrip(&result);
        }
    }
}

#[test]
fn entry_at_1509_can_stop_or_close_in_its_own_minute_on_both_sides() {
    for side in [Direction::Long, Direction::Short] {
        let mut fixture = Fixture::new(1, 353);
        fixture.set(354, BASE, BASE + 500, BASE - 500, BASE + 200);
        let close = exact_trade(&fixture.evaluate(side));
        assert_eq!(close.entry_bar, close.exit_bar);
        assert_eq!(close.holding_minutes, 1);
        assert_eq!(close.exit_reason, ExitReason::Forced1510);
        assert_eq!(close.optimistic_exit_paisa, BASE + 200);
        assert_eq!(close.pessimistic_exit_paisa, BASE + 200);
        assert_eq!(
            close.optimistic_paisa,
            if side == Direction::Long { 200 } else { -200 }
        );
        assert_eq!(close.optimistic_paisa, close.pessimistic_paisa);
        if side == Direction::Long {
            fixture.set(354, BASE, BASE + 2000, BASE - 1100, BASE + 100);
        } else {
            fixture.set(354, BASE, BASE + 1100, BASE - 2000, BASE - 100);
        }
        let result = fixture.evaluate(side);
        let stop = exact_trade(&result);
        assert_eq!(stop.exit_reason, ExitReason::Stop);
        assert_eq!(stop.holding_minutes, 1);
        assert_eq!(stop.optimistic_paisa, -1000);
        assert_eq!(stop.pessimistic_paisa, -1100);
        assert_eq!(
            stop.favourable_paisa, 0,
            "unknown stop-minute order cannot credit a later favourable extreme"
        );
        assert_eq!(stop.exit_from_micros, stamp(354));
        assert_eq!(stop.exit_until_micros, stamp(355));
        roundtrip(&result);
    }
}

#[test]
fn exact_open_at_or_beyond_stop_skips_the_entry_without_a_free_gap_trade() {
    for side in [Direction::Long, Direction::Short] {
        for distance in [0, 1, 1000] {
            let mut fixture = Fixture::new(1, 30);
            let open = if side == Direction::Long {
                BASE - 1000 - distance
            } else {
                BASE + 1000 + distance
            };
            fixture.set(31, open, open + 20, open - 20, open);
            let result = fixture.evaluate(side);
            assert!(result.trades().is_empty());
            assert_eq!(result.metrics().gap_invalid, 1);
            let event = result.events().first().unwrap();
            assert_eq!(event.reason, Reason::GapInvalid);
            assert!(event.occupied_through_micros.is_none());
            roundtrip(&result);
        }
    }
}

#[test]
fn shared_stop_fill_uses_open_gap_before_retrace_and_printed_extreme_for_both_sides() {
    for side in [Side::Long, Side::Short] {
        let stop = BASE;
        for (open, expected_gap) in if side == Side::Long {
            [(BASE - 1, true), (BASE, false), (BASE + 1, false)]
        } else {
            [(BASE + 1, true), (BASE, false), (BASE - 1, false)]
        } {
            let bar = Candle::new(
                stamp(30),
                open,
                BASE + 100,
                BASE - 100,
                BASE,
                1,
                indicators::OI_NULL,
            );
            let (best, worst, gap) = crate::grid::printed_stop_fills_v1(&bar, stop, side, true);
            assert_eq!(gap, expected_gap);
            assert_eq!(best, if expected_gap { open } else { stop });
            assert_eq!(
                worst,
                if side == Side::Long {
                    BASE - 100
                } else {
                    BASE + 100
                }
            );
            let (new_best, _, new_gap) =
                crate::grid::printed_stop_fills_v1(&bar, stop, side, false);
            assert_eq!(new_best, stop);
            assert!(!new_gap, "a new order was not resting before the open");
        }
    }
}

#[test]
fn later_gap_stop_has_two_honest_gross_readings_and_ignores_prices_after_exit() {
    for side in [Direction::Long, Direction::Short] {
        let mut fixture = Fixture::new(1, 30);
        if side == Direction::Long {
            fixture.set(32, BASE - 1100, BASE + 10, BASE - 1200, BASE);
        } else {
            fixture.set(32, BASE + 1100, BASE + 1200, BASE - 10, BASE);
        }
        let before = fixture.evaluate(side);
        let row = exact_trade(&before);
        assert!(row.gapped);
        assert_eq!(row.optimistic_paisa, -1100);
        assert_eq!(row.pessimistic_paisa, -1200);
        assert_eq!(row.exit_bar_micros, stamp(32));
        fixture.set(33, BASE, BASE + 5000, BASE - 5000, BASE);
        let after = fixture.evaluate(side);
        assert_eq!(after.trades(), before.trades());
        assert_ne!(after.source_id(), before.source_id());
        assert_ne!(after.run_id(), before.run_id());
        roundtrip(&before);
        roundtrip(&after);
    }
}

#[test]
fn missing_or_refused_path_before_a_stop_preserves_occupancy_and_never_prices_later_touch() {
    for side in [Direction::Long, Direction::Short] {
        for mode in 0..3 {
            let mut fixture = Fixture::new(1, 30);
            if side == Direction::Long {
                fixture.set(33, BASE, BASE + 10, BASE - 2000, BASE);
            } else {
                fixture.set(33, BASE, BASE + 2000, BASE - 10, BASE);
            }
            let position = fixture
                .bars
                .iter()
                .position(|bar| bar.ts_micros == stamp(32))
                .unwrap();
            match mode {
                0 => {
                    fixture.bars.remove(position);
                }
                1 => {
                    let row = *fixture.bars.get(position).unwrap();
                    fixture.bars.insert(position, row);
                }
                _ => fixture.bars.get_mut(position).unwrap().close = BASE + 50_000,
            }
            let result = fixture.evaluate(side);
            assert!(result.trades().is_empty());
            assert_eq!(result.metrics().path_refused, 1);
            let event = result.events().first().unwrap();
            assert_eq!(event.reason, Reason::PathRefused);
            assert_eq!(event.occupied_through_micros, Some(stamp(354)));
            roundtrip(&result);
        }
    }
}

#[test]
fn missing_duplicate_or_corrupt_closing_record_is_not_a_nearby_close() {
    for side in [Direction::Long, Direction::Short] {
        for mode in 0..3 {
            let mut fixture = Fixture::new(1, 30);
            let position = fixture
                .bars
                .iter()
                .position(|bar| bar.ts_micros == stamp(354))
                .unwrap();
            match mode {
                0 => {
                    fixture.bars.remove(position);
                }
                1 => {
                    let row = *fixture.bars.get(position).unwrap();
                    fixture.bars.insert(position, row);
                }
                _ => fixture.bars.get_mut(position).unwrap().close = BASE + 999_999,
            }
            let result = fixture.evaluate(side);
            assert!(result.trades().is_empty());
            let event = result.events().first().unwrap();
            assert!(matches!(
                event.reason,
                Reason::ClosingMinute | Reason::PathRefused
            ));
            assert_eq!(event.occupied_through_micros, Some(stamp(354)));
            assert!(!result.periods().last().unwrap().close_verified);
            roundtrip(&result);
        }
    }
}

#[test]
fn missing_immediate_entry_never_moves_to_a_later_open_and_duplicate_entry_blocks_day() {
    for side in [Direction::Long, Direction::Short] {
        let mut absent = Fixture::new(1, 30);
        absent.bars.retain(|bar| bar.ts_micros != stamp(31));
        let result = absent.evaluate(side);
        assert!(result.trades().is_empty());
        assert_eq!(result.metrics().unreachable, 1);
        assert_eq!(result.events().first().unwrap().entry_bar, None);
        roundtrip(&result);
        let mut duplicate = Fixture::new(1, 30);
        let position = duplicate
            .bars
            .iter()
            .position(|bar| bar.ts_micros == stamp(31))
            .unwrap();
        let row = *duplicate.bars.get(position).unwrap();
        duplicate.bars.insert(position, row);
        let result = duplicate.evaluate(side);
        assert_eq!(result.metrics().entry_refused, 1);
        assert!(result.trades().is_empty());
        assert_eq!(
            result.events().first().unwrap().occupied_through_micros,
            Some(stamp(354))
        );
        roundtrip(&result);
    }
}

#[test]
fn one_position_allows_sequential_reentry_but_never_the_same_exit_minute() {
    for side in [Direction::Long, Direction::Short] {
        let mut fixture = Fixture::new(1, 30);
        if side == Direction::Long {
            fixture.set(31, BASE, BASE + 10, BASE - 1100, BASE);
        } else {
            fixture.set(31, BASE, BASE + 1100, BASE - 10, BASE);
        }
        fixture.all_later_signals();
        let result = fixture.evaluate(side);
        assert!(result.trades().len() >= 2);
        assert!(result.metrics().while_open > 0);
        assert!(
            result
                .trades()
                .windows(2)
                .all(|pair| pair.first().unwrap().exit_bar_micros
                    < pair.get(1).unwrap().entry_micros)
        );
        assert_eq!(
            result.metrics().stopped + result.metrics().forced,
            result.metrics().trades
        );
        assert_eq!(
            result.periods().iter().map(|day| day.trades).sum::<u64>(),
            result.metrics().trades
        );
        roundtrip(&result);
    }
}

#[test]
fn fixed_later_window_preserves_causal_column_and_rekeys_without_counting_earlier_days() {
    let mut fixture = Fixture::new(1, 30);
    fixture.signals = fixture.bars.clone();
    fixture.column = column(&fixture.signals);
    let prepared = fixture.prepared();
    for side in [Direction::Long, Direction::Short] {
        let full = prepared.evaluate(&always(), side, 100_000).unwrap();
        let later = prepared
            .evaluate_days(&always(), side, 100_000, LAST_DAY, LAST_DAY)
            .unwrap();
        let rows: Vec<_> = full
            .trades()
            .iter()
            .filter(|row| indicators::ist_day(row.entry_micros) == LAST_DAY)
            .copied()
            .collect();
        assert_eq!(later.trades(), rows);
        assert_eq!(later.periods().len(), 1);
        assert_eq!(later.first_day(), LAST_DAY);
        assert_eq!(later.last_day(), LAST_DAY);
        assert_eq!(later.source_id(), full.source_id());
        assert_ne!(later.run_id(), full.run_id());
        assert_eq!(
            prepared
                .run_id_days(&always(), side, LAST_DAY, LAST_DAY)
                .unwrap(),
            later.run_id()
        );
        roundtrip(&later);
    }
    assert!(matches!(
        prepared.evaluate_days(&always(), Direction::Long, 100, 4, 3),
        Err(Error::Scope)
    ));
    assert!(matches!(
        prepared.evaluate_days(&always(), Direction::Long, 100, -1, LAST_DAY),
        Err(Error::Scope)
    ));
}

#[test]
fn scope_identity_unavailable_truth_and_evidence_admission_are_explicit() {
    let fixture = Fixture::new(1, 30);
    let prepared = fixture.prepared();
    let long = prepared
        .evaluate(&always(), Direction::Long, 100_000)
        .unwrap();
    assert_ne!(
        prepared.run_id(&always(), Direction::Long).unwrap(),
        prepared.run_id(&always(), Direction::Short).unwrap()
    );
    assert_ne!(
        prepared.run_id(&always(), Direction::Long).unwrap(),
        prepared
            .run_id(&Expression::parse("0 & !0").unwrap(), Direction::Long)
            .unwrap()
    );
    assert!(matches!(
        prepared.evaluate(&always(), Direction::Long, 0),
        Err(Error::Bound)
    ));
    assert!(matches!(
        prepared.evaluate(&always(), Direction::Undirected, 1),
        Err(Error::Scope)
    ));
    let zero = prepared
        .evaluate(&Expression::parse("0 & !0").unwrap(), Direction::Long, 0)
        .unwrap();
    assert_eq!(zero.truth().hits, 0);
    assert!(zero.trades().is_empty());
    assert!(!zero.periods().is_empty());
    assert!(zero.periods().iter().all(|period| period.trades == 0));
    roundtrip(&zero);
    let mut unavailable = Fixture::new(1, 30);
    unavailable.column.clear_before(unavailable.signals.len());
    let unknown = unavailable
        .prepared()
        .evaluate(&Expression::parse("!0").unwrap(), Direction::Long, 0)
        .unwrap();
    assert!(unknown.truth().evaluated > 0);
    assert_eq!(unknown.truth().unknown, unknown.truth().evaluated);
    assert_eq!(unknown.truth().hits, 0);
    assert!(unknown.events().is_empty());
    assert!(unknown.trades().is_empty());
    let cash = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
    let mut source = fixture.source();
    source.series = ExecutionSeriesV1::new(
        &cash,
        "generated-test-feed",
        "generated-test-commit",
        [7; 32],
        &fixture.bars,
    )
    .unwrap();
    assert!(matches!(
        Prepared::new(Policy::V1, source),
        Err(Error::Scope)
    ));
    for label in ["1day", "daily", "", "4min"] {
        let mut source = fixture.source();
        source.timeframe = label;
        assert!(matches!(
            Prepared::new(Policy::V1, source),
            Err(Error::Scope)
        ));
    }
    let bank = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap();
    let mut source = fixture.source();
    source.series = ExecutionSeriesV1::new(
        &bank,
        "generated-test-feed",
        "generated-test-commit",
        [7; 32],
        &fixture.bars,
    )
    .unwrap();
    let bank = Prepared::new(Policy::V1, source)
        .unwrap()
        .evaluate(&always(), Direction::Long, 100_000)
        .unwrap();
    assert_ne!(bank.run_id(), long.run_id());
    assert_eq!(bank.trades(), long.trades());
    roundtrip(&bank);
}

#[test]
fn attested_closed_boundary_days_are_bound_but_never_fabricated_as_observations() {
    let fixture = Fixture::new(1, 30);
    let original = fixture.prepared();
    let mut source = fixture.source();
    source.first_day = -1;
    source.last_day = LAST_DAY + 1;
    let extended = Prepared::new(Policy::V1, source).unwrap();
    assert_ne!(original.source_id(), extended.source_id());
    let result = extended
        .evaluate(&always(), Direction::Long, 100_000)
        .unwrap();
    assert_eq!(result.first_day(), -1);
    assert_eq!(result.last_day(), LAST_DAY + 1);
    assert_eq!(result.periods().first().unwrap().day, 0);
    assert_eq!(result.periods().last().unwrap().day, LAST_DAY);
    let absent = extended
        .evaluate_days(&always(), Direction::Long, 0, LAST_DAY + 1, LAST_DAY + 1)
        .unwrap();
    assert!(absent.periods().is_empty());
    assert_eq!(absent.truth().evaluated, 0);
    assert!(absent.events().is_empty());
    assert!(absent.trades().is_empty());
    assert!(matches!(
        extended.run_id_days(&always(), Direction::Long, LAST_DAY + 2, LAST_DAY + 2),
        Err(Error::Scope)
    ));
    for (first, last) in [
        (1, LAST_DAY),
        (0, LAST_DAY - 1),
        (2, 1),
        (i64::MIN, LAST_DAY),
        (0, i64::MAX),
    ] {
        let mut source = fixture.source();
        source.first_day = first;
        source.last_day = last;
        assert!(matches!(
            Prepared::new(Policy::V1, source),
            Err(Error::Source)
        ));
    }
    let empty = Column::default();
    let mut source = fixture.source();
    source.signal_column = &empty;
    assert!(matches!(
        Prepared::new(Policy::V1, source),
        Err(Error::Source)
    ));
    roundtrip(&result);
    roundtrip(&absent);
}

#[test]
fn codecs_and_checked_aggregate_overflow_cannot_change_trade_readings_or_clocks() {
    let fixture = Fixture::new(1, 30);
    let result = fixture.evaluate(Direction::Long);
    let row = exact_trade(&result);
    roundtrip(&result);
    for len in 0..Trade::BYTE_LEN {
        assert!(Trade::decode(row.canonical_bytes().get(..len).unwrap()).is_err());
    }
    let mut changed = row;
    changed.pessimistic_paisa = changed.optimistic_paisa + 1;
    assert!(Trade::decode(&changed.canonical_bytes()).is_err());
    let mut changed = row;
    changed.entry_paisa += 1;
    assert!(Trade::decode(&changed.canonical_bytes()).is_err());
    let mut changed = row;
    changed.exit_until_micros += MINUTE;
    assert!(Trade::decode(&changed.canonical_bytes()).is_err());
    let mut changed = row;
    changed.exit_bar += 1;
    assert!(Trade::decode(&changed.canonical_bytes()).is_err());
    for seed in [i64::MIN, i64::MAX] {
        let mut metrics = Metrics {
            optimistic_paisa: seed,
            pessimistic_paisa: seed,
            ..Metrics::default()
        };
        let mut trade = row;
        trade.optimistic_paisa = if seed < 0 { -1 } else { 1 };
        trade.pessimistic_paisa = trade.optimistic_paisa;
        assert_eq!(
            record_trade(&mut metrics, &mut 0, &trade),
            Err(Error::Arithmetic)
        );
    }
    let mut metrics = Metrics {
        trades: u64::MAX,
        ..Metrics::default()
    };
    assert_eq!(
        record_trade(&mut metrics, &mut 0, &row),
        Err(Error::Arithmetic)
    );
    let mut metrics = Metrics {
        pessimistic_paisa: i64::MIN + 1,
        optimistic_paisa: i64::MIN + 1,
        ..Metrics::default()
    };
    let mut peak = i64::MAX;
    assert_eq!(
        record_trade(&mut metrics, &mut peak, &row),
        Err(Error::Arithmetic)
    );
    for byte in 0..Policy::BYTE_LEN {
        let mut bytes = Policy::V1.canonical_bytes();
        *bytes.get_mut(byte).unwrap() ^= 1;
        assert!(Policy::decode(&bytes).is_err());
    }
}
