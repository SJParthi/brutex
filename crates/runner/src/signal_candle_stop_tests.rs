#![cfg(test)]
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
                .unwrap()
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
        // 10:15 IST is a complete edge on every open-anchored resampling grid
        // (D-1430): sixty minutes past the open is a multiple of each rung.
        let fixture = Fixture::new(minutes, 60);
        for side in [Direction::Long, Direction::Short] {
            let prepared = fixture.prepared();
            let result = prepared.evaluate(&always(), side, 100_000).unwrap();
            let row = exact_trade(&result);
            assert_eq!(
                row.entry_micros,
                stamp(60 + usize::try_from(minutes).unwrap())
            );
            assert_eq!(row.signal_micros, stamp(60));
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
fn a_signal_closing_after_the_last_entry_minute_is_too_late_even_with_no_minute_there() {
    // D-0603. Each rung's last bucket closes at or after 15:30, where no minute
    // exists, so alignment gives it none. That is "too late to trade today" --
    // the verdict a present 15:10 entry gets -- never a missing observation,
    // which refuses the whole setting's execution completeness. Buckets sit on
    // the open-anchored grid (D-1430), so the last one starts at 15:29, 15:29,
    // 15:27, 15:25, 15:25, 15:15, 15:15 and 15:15 respectively.
    let last_buckets = [
        (1, 374),
        (2, 374),
        (3, 372),
        (5, 370),
        (10, 370),
        (15, 360),
        (30, 360),
        (60, 360),
    ];
    for side in [Direction::Long, Direction::Short] {
        for (minutes, signal_minute) in last_buckets {
            let result = Fixture::new(minutes, signal_minute).evaluate(side);
            assert!(result.trades().is_empty(), "{minutes}min");
            assert_eq!(result.metrics().too_late, 1, "{minutes}min");
            assert_eq!(result.metrics().unreachable, 0, "{minutes}min");
            let event = result.events().first().unwrap();
            assert_eq!(event.reason, Reason::TooLate, "{minutes}min");
            assert_eq!(event.entry_bar, None, "{minutes}min");
            roundtrip(&result);
        }
        // The boundary. A 15:09 entry is allowed, so a missing 15:09 minute hid
        // a tradeable entry: a real hole. A missing 15:10 minute hid nothing.
        for (signal_minute, missing, unreachable, too_late) in [(353, 354, 1, 0), (354, 355, 0, 1)]
        {
            let mut fixture = Fixture::new(1, signal_minute);
            fixture.bars.retain(|bar| bar.ts_micros != stamp(missing));
            let result = fixture.evaluate(side);
            assert!(result.trades().is_empty(), "signal minute {signal_minute}");
            assert_eq!(
                (result.metrics().unreachable, result.metrics().too_late),
                (unreachable, too_late),
                "signal minute {signal_minute}"
            );
            roundtrip(&result);
        }
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

#[test]
fn a_tabled_day_window_seals_exactly_what_the_full_row_walk_did() {
    // D-2307: the window is read from per-day tables of rows and periods
    // rather than found by walking every row and period. The pinned fold below was
    // measured on the build before that change, over every start day and three
    // end days per start, both sides, so any drift in which rows or periods a
    // window takes changes it.
    let mut fixture = Fixture::new(1, 30);
    fixture.signals = fixture.bars.clone();
    fixture.column = column(&fixture.signals);
    let prepared = fixture.prepared();
    let mut fold = Hasher::new();
    let mut windows = 0_u32;
    for first in 0..=LAST_DAY {
        for last in [first, (first + 1).min(LAST_DAY), LAST_DAY] {
            for side in [Direction::Long, Direction::Short] {
                let result = prepared
                    .evaluate_days(&always(), side, 100_000, first, last)
                    .unwrap();
                assert!(
                    result
                        .periods()
                        .iter()
                        .all(|period| (first..=last).contains(&period.day))
                );
                fold.update(&result.digest());
                windows += 1;
            }
        }
    }
    assert_eq!(windows, 240);
    let pin: [u8; 32] = [
        115, 168, 60, 123, 173, 208, 37, 133, 254, 72, 232, 166, 232, 107, 241, 176, 137, 227, 42,
        200, 185, 248, 182, 169, 204, 8, 90, 7, 172, 20, 223, 148,
    ];
    assert_eq!(fold.finalize(), pin);
}

/// BASE-COV-LIBS: each word a run or a page can show is its own, so two
/// failures are never one sentence.
#[test]
fn every_error_reason_and_exit_has_its_own_words() {
    let errors = [
        Error::Scope,
        Error::Source,
        Error::Arithmetic,
        Error::Bound,
        Error::Allocation,
        Error::Encoding,
    ];
    let said: std::collections::HashSet<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(said.len(), errors.len());
    assert!(
        said.iter()
            .all(|word| word.starts_with("signal-candle stop"))
    );
    let reasons = [
        Reason::None,
        Reason::Unreachable,
        Reason::TooLate,
        Reason::GapInvalid,
        Reason::WhileOpen,
        Reason::EntryRefused,
        Reason::PathRefused,
        Reason::ClosingMinute,
    ];
    let words: std::collections::HashSet<&str> =
        reasons.iter().map(|reason| reason.as_str()).collect();
    assert_eq!(words.len(), reasons.len());
    assert_eq!(Reason::None.as_str(), "priced");
    assert_ne!(ExitReason::Stop.as_str(), ExitReason::Forced1510.as_str());
    for (code, reason) in (0_u64..).zip(reasons) {
        assert_eq!(Reason::decode(code), Ok(reason));
    }
    for code in [8, 9, u64::MAX] {
        assert_eq!(Reason::decode(code), Err(Error::Encoding));
    }
    assert_eq!(Policy::V1.digest(), hash(&Policy::V1.canonical_bytes()));
    assert_ne!(Policy::V1.digest(), hash(&[]));
}

/// The ppm rule at its edges: no entry, a negative one, a negative distance,
/// nothing at all, and a distance whose ppm cannot fit.
#[test]
fn ppm_refuses_what_has_no_ratio_and_what_cannot_fit() {
    assert_eq!(ppm(1, 0), Err(Error::Arithmetic));
    assert_eq!(ppm(1, -1), Err(Error::Arithmetic));
    assert_eq!(ppm(-1, 1), Err(Error::Arithmetic));
    assert_eq!(ppm(0, 1), Ok(0));
    assert_eq!(ppm(1, 2), Ok(500_000));
    assert_eq!(ppm(i64::MAX, 1), Err(Error::Arithmetic));
    assert_eq!(ppm(i64::MAX, i64::MAX), Ok(1_000_000));
}

/// A TRADE'S STATISTICAL ROW IS THE TRADE'S OWN FIGURES, and the evaluation
/// says what it was asked: family, side, program and the bars it aligned.
#[test]
fn a_priced_trade_reads_back_as_its_statistical_row() {
    let fixture = Fixture::new(1, 30);
    for side in [Direction::Long, Direction::Short] {
        let result = fixture.evaluate(side);
        assert_eq!(result.direction(), side);
        assert_eq!(*result.program(), always());
        assert!(!fixture.prepared().alignment().is_empty());
        let family = result.family();
        assert_eq!(family, ResearchFamilyV1::new(fixture.key).unwrap());
        assert!(family.legacy_index_family().is_some());
        let trade = exact_trade(&result);
        let row = trade.statistical_row().unwrap();
        assert_eq!(row.best, trade.optimistic_paisa);
        assert_eq!(row.worst, trade.pessimistic_paisa);
        assert_eq!(row.entry_micros, trade.entry_micros);
        assert_eq!(row.exit_micros, trade.exit_bar_micros);
        assert_eq!(row.adverse_paisa, trade.adverse_paisa);
        assert_eq!(
            row.adverse,
            ppm(trade.adverse_paisa, trade.entry_paisa).unwrap()
        );
        // COMBINED: a row whose entry is gone refuses, whatever else it holds.
        let mut broken = trade;
        broken.entry_paisa = 0;
        assert_eq!(broken.statistical_row().err(), Some(Error::Arithmetic));
    }
}

/// Every word of a canonical record that only one value may hold is refused
/// at any other: an exit reason, the gap flag, an option flag.
#[test]
fn a_canonical_word_outside_its_values_is_refused() {
    let fixture = Fixture::new(1, 30);
    let result = fixture.evaluate(Direction::Long);
    let row = exact_trade(&result);
    assert_eq!(option(0, 0), Ok(None));
    assert_eq!(option(1, 0), Ok(Some(0)));
    assert_eq!(option(0, 1), Err(Error::Encoding));
    assert_eq!(option(2, 0), Err(Error::Encoding));
    let good = row.canonical_bytes();
    let mut refused = 0_usize;
    // Eight-byte words after the eight-byte magic: each one, to a value none
    // of them holds. Only the record's own checks can answer.
    for word in 0..(Trade::BYTE_LEN / 8).saturating_sub(1) {
        let mut bytes = good;
        let at = 8 + word * 8;
        bytes
            .get_mut(at..at + 8)
            .unwrap()
            .copy_from_slice(&u64::MAX.to_le_bytes());
        if Trade::decode(&bytes).is_err() {
            refused += 1;
        }
    }
    assert!(refused > 10, "only {refused} words were checked");
    let mut bytes = good;
    bytes.get_mut(..8).unwrap().copy_from_slice(b"BRSCXX01");
    assert_eq!(Trade::decode(&bytes), Err(Error::Encoding));
}

// BASE-COV-LIBS-LS2: every refusal arm of the four record codecs and of the
// source check, each reached by a record or source that breaks exactly that
// one rule and keeps every rule checked before it.

/// A long trade stopped at minute 40 (09:55 IST): the stop is the signal
/// candle's low, BASE - 1000, and minute 40 prints a low of BASE - 1100.
fn stopped_long() -> Trade {
    let mut fixture = Fixture::new(1, 30);
    fixture.set(40, BASE, BASE + 10, BASE - 1100, BASE);
    let trade = exact_trade(&fixture.evaluate(Direction::Long));
    assert_eq!(trade.exit_reason, ExitReason::Stop);
    assert_eq!(
        (trade.optimistic_paisa, trade.pessimistic_paisa),
        (-1000, -1100)
    );
    assert_eq!(trade.holding_minutes, 10);
    trade
}

/// A long trade closed at the exact 15:10 print.
fn forced_long() -> Trade {
    let trade = exact_trade(&Fixture::new(1, 30).evaluate(Direction::Long));
    assert_eq!(trade.exit_reason, ExitReason::Forced1510);
    trade
}

/// A canonical record with one eight-byte word, counted after the
/// eight-byte magic, replaced.
fn with_word<const B: usize>(mut bytes: [u8; B], word: usize, value: u64) -> [u8; B] {
    bytes
        .get_mut(8 + word * 8..16 + word * 8)
        .unwrap()
        .copy_from_slice(&value.to_le_bytes());
    bytes
}

/// Every case is refused as an encoding fault, never read back as a trade.
fn assert_each_trade_refused(cases: &[(&str, Trade)]) {
    for (what, row) in cases {
        assert_eq!(
            Trade::decode(&row.canonical_bytes()),
            Err(Error::Encoding),
            "{what}"
        );
    }
}

/// EACH RULE OF A STOP-EXIT RECORD REFUSES: a record that keeps every other
/// rule the decoder checks and breaks one is never read back as a trade. The
/// one exception is a later-day exit, which is also past its entry day's 15:09
/// close and so breaks two rules at once: a later IST day starts after it.
#[test]
fn a_stop_exit_record_that_breaks_one_rule_is_refused() {
    let stop = stopped_long();
    assert_eq!(Trade::decode(&stop.canonical_bytes()), Ok(stop));
    let late = stamp(355);
    let late_hold = u64::try_from((late + MINUTE - stop.entry_micros) / MINUTE).unwrap();
    assert_each_trade_refused(&[
        (
            "an exit on a later day than its entry",
            Trade {
                exit_bar_micros: stop.exit_bar_micros + DAY,
                ..stop
            },
        ),
        (
            "a stop exit observed after its bar opened",
            Trade {
                exit_from_micros: stop.exit_from_micros + 1,
                ..stop
            },
        ),
        (
            "a stop exit that does not end with its minute",
            Trade {
                exit_until_micros: stop.exit_until_micros + 1,
                ..stop
            },
        ),
        (
            "a stop exit that did not lose",
            Trade {
                optimistic_exit_paisa: stop.entry_paisa,
                optimistic_paisa: 0,
                ..stop
            },
        ),
        (
            "an entry off the minute grid",
            Trade {
                entry_bar: stop.entry_bar + 1,
                entry_micros: stop.entry_micros + 1,
                signal_close_micros: stop.signal_close_micros + 1,
                holding_minutes: stop.holding_minutes - 1,
                ..stop
            },
        ),
        (
            "an exit off the minute grid",
            Trade {
                exit_bar_micros: stop.exit_bar_micros + 1,
                exit_from_micros: stop.exit_from_micros + 1,
                exit_until_micros: stop.exit_until_micros + 1,
                ..stop
            },
        ),
        (
            "a stop exit after the 15:09 closing minute",
            Trade {
                exit_bar: stop.entry_bar + late_hold - 1,
                exit_bar_micros: late,
                exit_from_micros: late,
                exit_until_micros: late + MINUTE,
                holding_minutes: late_hold,
                ..stop
            },
        ),
    ]);
}

/// EACH RULE OF A FORCED-CLOSE RECORD REFUSES, and an entry exactly at its
/// stop has no side to price whichever exit it took.
#[test]
fn a_forced_close_record_that_breaks_one_rule_is_refused() {
    let forced = forced_long();
    assert_eq!(Trade::decode(&forced.canonical_bytes()), Ok(forced));
    assert_each_trade_refused(&[
        (
            "a forced close one minute before 15:10",
            Trade {
                exit_bar: forced.exit_bar - 1,
                exit_bar_micros: forced.exit_bar_micros - MINUTE,
                exit_from_micros: forced.exit_from_micros - MINUTE,
                exit_until_micros: forced.exit_until_micros - MINUTE,
                holding_minutes: forced.holding_minutes - 1,
                ..forced
            },
        ),
        (
            "a forced close read at two prices",
            Trade {
                pessimistic_exit_paisa: forced.optimistic_exit_paisa - 1,
                pessimistic_paisa: forced.pessimistic_paisa - 1,
                ..forced
            },
        ),
        (
            "a forced close marked as a gap",
            Trade {
                gapped: true,
                ..forced
            },
        ),
        (
            "an entry exactly at its stop, which has no side",
            Trade {
                stop_paisa: forced.entry_paisa,
                ..forced
            },
        ),
    ]);
}

/// EACH DISPOSITION RULE OF AN EVENT RECORD REFUSES ON ITS OWN, including a
/// version word no writer emits and a priced event with a missing coordinate.
#[test]
fn an_event_record_that_breaks_one_disposition_rule_is_refused() {
    let result = Fixture::new(1, 30).evaluate(Direction::Long);
    let event = *result.events().first().unwrap();
    assert_eq!(event.reason, Reason::None);
    assert!(event.entry_bar.is_some() && event.occupied_through_micros.is_some());
    assert_eq!(Event::decode(&event.canonical_bytes()), Ok(event));
    // Word 5 is the version; only 1 exists.
    for version in [0, 2, u64::MAX] {
        let bytes = with_word(event.canonical_bytes(), 5, version);
        assert_eq!(Event::decode(&bytes), Err(Error::Encoding), "{version}");
    }
    let close = event.signal_close_micros;
    let cases = [
        (
            "a signal that opens at its own close",
            Event {
                signal_micros: close,
                ..event
            },
        ),
        (
            "a stop of zero",
            Event {
                stop_paisa: 0,
                ..event
            },
        ),
        (
            "an entry at another instant than the signal's close",
            Event {
                entry_micros: Some(close + MINUTE),
                ..event
            },
        ),
        (
            "an occupation that ends before the entry",
            Event {
                occupied_through_micros: Some(close - 1),
                ..event
            },
        ),
        (
            "a priced event with no trade",
            Event {
                trade_index: None,
                ..event
            },
        ),
        (
            "a trade with no entry coordinate",
            Event {
                entry_bar: None,
                entry_micros: None,
                ..event
            },
        ),
        (
            "a trade with no occupied minute",
            Event {
                occupied_through_micros: None,
                ..event
            },
        ),
    ];
    for (what, row) in cases {
        assert_eq!(
            Event::decode(&row.canonical_bytes()),
            Err(Error::Encoding),
            "{what}"
        );
    }
}

/// EACH COUNT RULE OF A PERIOD RECORD REFUSES ON ITS OWN, including counts
/// that would overflow and money on a day that traded nothing.
#[test]
fn a_period_record_that_breaks_one_count_rule_is_refused() {
    let day = Period {
        day: LAST_DAY,
        offered_bars: 375,
        accepted_bars: 375,
        unavailable_minutes: 0,
        close_verified: true,
        trades: 1,
        wins: 0,
        optimistic_paisa: -1000,
        pessimistic_paisa: -1100,
        refused: 0,
        signals: 1,
    };
    assert_eq!(Period::decode(&day.canonical_bytes()), Ok(day));
    let quiet = Period {
        trades: 0,
        optimistic_paisa: 0,
        pessimistic_paisa: 0,
        ..day
    };
    assert_eq!(Period::decode(&quiet.canonical_bytes()), Ok(quiet));
    // Word 4 is the close flag; only 0 and 1 exist.
    let bytes = with_word(day.canonical_bytes(), 4, 2);
    assert_eq!(Period::decode(&bytes), Err(Error::Encoding));
    let cases = [
        (
            "a day that offered no bar",
            Period {
                offered_bars: 0,
                accepted_bars: 0,
                ..day
            },
        ),
        (
            "more bars accepted than offered",
            Period {
                accepted_bars: 376,
                ..day
            },
        ),
        ("more wins than trades", Period { wins: 2, ..day }),
        (
            "more trades and refusals than signals",
            Period { refused: 1, ..day },
        ),
        (
            "a trade count that overflows with its refusals",
            Period {
                trades: u64::MAX,
                refused: 1,
                ..day
            },
        ),
        (
            "a pessimistic total above the optimistic one",
            Period {
                pessimistic_paisa: -900,
                ..day
            },
        ),
        (
            "an optimistic total on a day with no trade",
            Period {
                optimistic_paisa: 5,
                ..quiet
            },
        ),
        (
            "a pessimistic total on a day with no trade",
            Period {
                pessimistic_paisa: -5,
                ..quiet
            },
        ),
    ];
    for (what, row) in cases {
        assert_eq!(
            Period::decode(&row.canonical_bytes()),
            Err(Error::Encoding),
            "{what}"
        );
    }
}

/// EACH ACCOUNTING RULE OF A METRICS RECORD REFUSES ON ITS OWN, including
/// money with no trade behind it. A sign rule is broken on a one-trade record,
/// so the no-trade money rule cannot be what refuses it.
#[test]
fn a_metrics_record_that_breaks_one_accounting_rule_is_refused() {
    let none = Metrics::default();
    assert_eq!(Metrics::decode(&none.canonical_bytes()), Ok(none));
    let one = Metrics {
        trades: 1,
        stopped: 1,
        holding_minutes: 1,
        ..none
    };
    assert_eq!(Metrics::decode(&one.canonical_bytes()), Ok(one));
    let cases = [
        ("a win with no trade", Metrics { wins: 1, ..none }),
        ("a stop with no trade", Metrics { stopped: 1, ..none }),
        (
            "a stop gap with no stop",
            Metrics {
                stop_gaps: 1,
                ..none
            },
        ),
        (
            "an optimistic total below the pessimistic one",
            Metrics {
                optimistic_paisa: -1,
                ..one
            },
        ),
        (
            "a negative drawdown",
            Metrics {
                drawdown_paisa: -1,
                ..one
            },
        ),
        (
            "a negative adverse excursion",
            Metrics {
                adverse_paisa: -1,
                ..one
            },
        ),
        (
            "a negative favourable excursion",
            Metrics {
                favourable_paisa: -1,
                ..one
            },
        ),
        (
            "a trade that held no minute",
            Metrics {
                holding_minutes: 0,
                ..one
            },
        ),
        (
            "a drawdown with no trade",
            Metrics {
                drawdown_paisa: 5,
                ..none
            },
        ),
    ];
    for (what, row) in cases {
        assert_eq!(
            Metrics::decode(&row.canonical_bytes()),
            Err(Error::Encoding),
            "{what}"
        );
    }
}

/// A FREQUENCY RUN HAS NO TRADING IDENTITY, and only the eight intraday
/// labels have a canonical name.
#[test]
fn an_undirected_run_id_and_an_unknown_timeframe_are_refused_by_scope() {
    let fixture = Fixture::new(1, 30);
    let prepared = fixture.prepared();
    assert_eq!(
        prepared.run_id(&always(), Direction::Undirected),
        Err(Error::Scope)
    );
    assert!(prepared.run_id(&always(), Direction::Long).is_ok());
    for label in [
        "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min",
    ] {
        assert_eq!(canonical_timeframe(label), Ok(label));
        assert!(timeframe_micros(label).is_ok());
    }
    for label in ["", "4min", "1day", "60MIN", "1min "] {
        assert_eq!(canonical_timeframe(label), Err(Error::Scope), "{label}");
        assert_eq!(timeframe_micros(label), Err(Error::Scope), "{label}");
    }
}

/// One edit of a signal bar the column's last row reads, with what it breaks.
type SignalEdit = (&'static str, fn(&mut Candle));

/// The source error `Prepared::new` answers, if any.
fn refused_source(source: Source<'_>) -> Option<Error> {
    Prepared::new(Policy::V1, source).err()
}

/// THE SIGNAL SIDE IS CHECKED BEFORE ANY ALIGNMENT: nothing to read, a
/// column that is not a signal column, bars out of order, and a row whose
/// signal bar is unlike the bar it was folded from (the caller hands both, so
/// the evaluator's own refusals do not cover them).
#[test]
fn a_signal_side_that_is_empty_unsorted_or_unlike_its_rows_is_refused() {
    let fixture = Fixture::new(1, 30);
    assert!(Prepared::new(Policy::V1, fixture.source()).is_ok());
    // The empty signal side carries a column folded from no bar, so its
    // acceptance covers it exactly and only the emptiness can refuse it.
    let nothing = column(&[]);
    assert!(nothing.acceptance_covers(0));
    assert_eq!(
        refused_source(Source {
            signal_bars: &[],
            signal_column: &nothing,
            ..fixture.source()
        }),
        Some(Error::Source)
    );
    // A column already projected onto fill bars is not a signal column.
    let width = fixture.column.len();
    let (fill, _) = fixture.column.reproject(&vec![None; width], width).unwrap();
    assert_eq!(fill.sourced(), Sourced::Fill);
    assert_eq!(
        refused_source(Source {
            signal_column: &fill,
            ..fixture.source()
        }),
        Some(Error::Source)
    );
    // Signal bars whose last row repeats the stamp before it.
    let mut repeated = fixture.signals.clone();
    let before = repeated.get(repeated.len() - 2).unwrap().ts_micros;
    repeated.last_mut().unwrap().ts_micros = before;
    assert_eq!(
        refused_source(Source {
            signal_bars: &repeated,
            ..fixture.source()
        }),
        Some(Error::Source)
    );
    let row = *fixture.column.sources().last().unwrap();
    let edits: [SignalEdit; 4] = [
        ("off the minute grid", |bar| bar.ts_micros += 1),
        ("no close instant", |bar| {
            bar.ts_micros = i64::MAX / MINUTE * MINUTE;
        }),
        ("a low of zero", |bar| bar.low = 0),
        ("a high below its low", |bar| bar.high = bar.low - 1),
    ];
    for (what, edit) in edits {
        let mut signals = fixture.signals.clone();
        edit(signals.get_mut(row).unwrap());
        assert_eq!(
            refused_source(Source {
                signal_bars: &signals,
                ..fixture.source()
            }),
            Some(Error::Source),
            "{what}"
        );
    }
}

/// THE EXECUTION SIDE IS CHECKED BEFORE ANY ALIGNMENT: no bar, bars or
/// context out of order, a context bar past the last placeable instant, and
/// a context that does not hold the execution bars it claims to.
#[test]
fn an_execution_side_that_is_empty_unsorted_or_not_its_context_is_refused() {
    let fixture = Fixture::new(1, 30);
    let series = |bars| {
        ExecutionSeriesV1::new(
            &fixture.key,
            "generated-test-feed",
            "generated-test-commit",
            [7; 32],
            bars,
        )
        .unwrap()
    };
    assert_eq!(
        refused_source(Source {
            series: series(&[]),
            ..fixture.source()
        }),
        Some(Error::Source)
    );
    let mut swapped = fixture.bars.clone();
    swapped.swap(10, 11);
    let mut beyond = fixture.bars.clone();
    let mut last = *beyond.last().unwrap();
    last.ts_micros = i64::MAX;
    beyond.push(last);
    let mut edited = fixture.bars.clone();
    edited.get_mut(100).unwrap().close += 1;
    for (what, context) in [
        ("out of order", &swapped),
        ("past the last placeable instant", &beyond),
        ("not the execution bars", &edited),
    ] {
        assert_eq!(
            refused_source(Source {
                minute_context: context,
                ..fixture.source()
            }),
            Some(Error::Source),
            "{what}"
        );
    }
    // COMBINED: the execution bars and their context agree with each other,
    // and both are out of order.
    assert_eq!(
        refused_source(Source {
            series: series(&swapped),
            minute_context: &swapped,
            ..fixture.source()
        }),
        Some(Error::Source)
    );
}
