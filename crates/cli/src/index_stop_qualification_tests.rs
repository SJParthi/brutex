#![cfg(test)]
//! Generated OHLCV fixtures only; never real market or profit evidence.
#![expect(
    clippy::unwrap_used,
    reason = "named finite regression fixture assertions"
)]
use super::*;
use brutex_core::instrument::{Exchange, InstrumentKey};
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use runner::expression::Expression;
use runner::identity::{DailyReferenceBinding, Params, ReferenceIntegrity};
use runner::signal_candle_stop::{Prepared, Source};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const BYTES: u64 = 32_000_000;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-native-qualification-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn days() -> Vec<i64> {
    eligible_days(16)
}
fn eligible_days(count: usize) -> Vec<i64> {
    let first = i64::from(
        pull::session::Day::new(2025, 1, 6)
            .unwrap()
            .days_from_epoch(),
    );
    (first..first + 100)
        .filter(|day| {
            crate::index_consistency::eligibility_of(*day)
                == crate::index_consistency::Eligibility::Eligible
        })
        .take(count)
        .collect()
}
fn evaluate(days: &[i64], window: &[i64], program: &Expression) -> Vec<Evaluation> {
    let mut bars = Vec::new();
    for (ordinal, &day) in days.iter().enumerate() {
        for minute in 0..runner::synthetic::BARS_PER_SESSION {
            let mut row = runner::synthetic::bar(i64::try_from(ordinal).unwrap(), minute);
            row.ts_micros = day * runner::synthetic::DAY_MICROS
                + runner::synthetic::IST_OPEN_UTC_MICROS
                + i64::try_from(minute).unwrap() * runner::synthetic::MINUTE_MICROS;
            let factor = i64::try_from(ordinal % 5 + 1).unwrap();
            for price in [&mut row.open, &mut row.high, &mut row.low, &mut row.close] {
                *price = runner::synthetic::BASE + (*price - runner::synthetic::BASE) * factor;
            }
            bars.push(row);
        }
    }
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().unwrap(),
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = Column::build(&bars, &mut evaluator);
    let key = InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap();
    let prepared = Prepared::new(
        Policy::V1,
        Source {
            series: runner::exit_grid_policy::ExecutionSeriesV1::new(
                &key,
                "zerodha",
                "generated-test-build",
                [7; 32],
                &bars,
            )
            .unwrap(),
            signal_bars: &bars,
            signal_column: &column,
            minute_context: &bars,
            daily_reference: DailyReferenceBinding {
                daily_bars: &[],
                eligibility: &[],
                schema: 1,
                eligibility_policy: 1,
                gap_overlay_policy: 1,
                excluded_ist_days: &[],
                daily_integrity: ReferenceIntegrity::ChecksumReceiptV1([8; 32]),
                minute_integrity: ReferenceIntegrity::ChecksumReceiptV1([9; 32]),
                swept_series_calendar_policy: 1,
            },
            timeframe: "1min",
            first_day: *days.first().unwrap(),
            last_day: *days.last().unwrap(),
            params: Params {
                min_hits: 0,
                ceiling: 100_000,
                pair_budget: 1,
                policy: 1,
            },
        },
    )
    .unwrap();
    [
        runner::identity::Direction::Long,
        runner::identity::Direction::Short,
    ]
    .into_iter()
    .map(|side| {
        prepared
            .evaluate_days(
                program,
                side,
                100_000,
                *window.first().unwrap(),
                *window.last().unwrap(),
            )
            .unwrap()
    })
    .collect()
}
fn save(root: &Path, namespace: &str, id: [u8; 32], body: &[u8]) -> [u8; 32] {
    let pending = persistence::prepare_in_namespace(root, namespace, id, body).unwrap();
    pending
        .verify_body(hash(body), u64::try_from(body.len()).unwrap())
        .unwrap();
    pending
        .finish(id, hash(body), u64::try_from(body.len()).unwrap())
        .unwrap()
}
struct Fixture {
    root: Temp,
    training: Vec<Evaluation>,
    later: Vec<Evaluation>,
    facts: Facts,
    program: Expression,
}
impl Fixture {
    fn new(no_trades: bool) -> Self {
        Self::with_days(no_trades, &days())
    }
    fn with_days(no_trades: bool, days: &[i64]) -> Self {
        Self::with_training_periods(no_trades, days, 8)
    }
    fn with_training_periods(no_trades: bool, days: &[i64], training_periods: usize) -> Self {
        let root = Temp::new();
        let split = days.get(..training_periods).unwrap();
        let suffix = days.get(training_periods..).unwrap();
        let program = Expression::parse(if no_trades { "0 & !0" } else { "0 | !0" }).unwrap();
        let training = evaluate(split, split, &program);
        let later = evaluate(days, suffix, &program);
        let old = [31; 32];
        let new = [32; 32];
        let training_body = candidates::encode(old, &training, BYTES).unwrap();
        let later_body = candidates::encode(new, &later, BYTES).unwrap();
        let old_pin = save(&root.0, candidates::NAMESPACE, old, &training_body);
        let new_pin = save(&root.0, candidates::NAMESPACE, new, &later_body);
        let facts = Facts {
            search: [33; 32],
            training: Link {
                identity: old,
                completion: old_pin,
            },
            later: Link {
                identity: new,
                completion: new_pin,
            },
            policy: crate::boolean_search_record::tests::generated_policy_with_ceilings(
                [1_000_000; 4],
            ),
            procedure: PopulationStatisticsProcedureV2::new(31, 49, 2).unwrap(),
            allocation: runner::search_allocation_v1::allocate(0, 0, 1_000_000).unwrap(),
            bounds: Bounds {
                candidates: 8,
                bootstrap_work: 10_000_000,
                split_work: 10_000_000,
                memory_bytes: BYTES,
                bytes: BYTES,
            },
            count: 2,
        };
        Self {
            root,
            training,
            later,
            facts,
            program,
        }
    }
    fn publish(&self) -> Reader {
        let (statistics, rows) =
            numeric::measure(&self.training, &self.later, &self.facts).unwrap();
        let id = identity(&self.facts);
        let bytes = codec::encode(
            id,
            &Body {
                facts: self.copy_facts(),
                statistics,
                rows,
            },
        )
        .unwrap();
        let pin = save(&self.root.0, NAMESPACE, id, &bytes);
        let daily = numeric::daily(&self.training, &self.later, &self.facts).unwrap();
        daily::produce(&self.root.0, id, pin, daily, BYTES, || Ok(())).unwrap();
        Reader::open(&self.root.0, id, BYTES, 100_000).unwrap()
    }
    fn copy_facts(&self) -> Facts {
        Facts {
            search: self.facts.search,
            training: self.facts.training,
            later: self.facts.later,
            policy: self.facts.policy,
            procedure: self.facts.procedure,
            allocation: self.facts.allocation,
            bounds: self.facts.bounds,
            count: self.facts.count,
        }
    }
    fn expected(&self) -> ExpectedSearchSlot<'_> {
        let source = |row: &Evaluation| ExpectedSource {
            identity: row.source_id(),
            first_day: row.first_day(),
            last_day: row.last_day(),
        };
        ExpectedSearchSlot {
            search: self.facts.search,
            training: source(self.training.first().unwrap()),
            later: source(self.later.first().unwrap()),
            policy: self.facts.policy,
            procedure: self.facts.procedure,
            allocation: self.facts.allocation,
            bounds: self.facts.bounds,
            programs: std::slice::from_ref(&self.program),
        }
    }
    fn replay(&self) -> ReplayBounds {
        ReplayBounds {
            bootstrap_work: self.facts.bounds.bootstrap_work,
            split_work: self.facts.bounds.split_work,
            memory_bytes: self.facts.bounds.memory_bytes,
        }
    }
}

#[test]
fn checkpoint_expected_facts_reject_recomputed_foreign_children_before_replay() {
    let original = Fixture::new(false);
    let expected = original.expected();
    let no_replay = ReplayBounds {
        bootstrap_work: 1,
        split_work: 1,
        memory_bytes: 1,
    };
    for variant in 0..5 {
        let mut foreign = Fixture::new(false);
        match variant {
            0 => {
                foreign.facts.policy =
                    crate::boolean_search_record::tests::generated_policy_with_ceilings(
                        [500_000; 4],
                    );
            }
            1 => foreign.facts.procedure = PopulationStatisticsProcedureV2::new(31, 50, 2).unwrap(),
            2 => {
                foreign.facts.allocation =
                    runner::search_allocation_v1::allocate(0, 0, 500_000).unwrap();
            }
            3 => foreign.facts.bounds.bootstrap_work += 1,
            4 => foreign.facts.bounds.memory_bytes += 1,
            _ => unreachable!(),
        }
        // Publish genuine native-parent measurements with the same supplied
        // search hash. Their self-consistent child hashes are not the declaration.
        let reader = foreign.publish();
        assert_eq!(reader.search_identity(), expected.search);
        let why = verify_search_slot_bounded(
            &foreign.root.0,
            reader.identity(),
            reader.completion_digest(),
            BYTES,
            100_000,
            &expected,
            no_replay,
        )
        .unwrap_err();
        assert!(
            why.contains("declared policy, procedure, allocation, bounds"),
            "variant {variant}: {why}"
        );
    }
}

#[test]
fn checkpoint_expected_sources_and_windows_reject_valid_foreign_ancestry_before_replay() {
    let original = Fixture::new(false);
    let start = *days().first().unwrap() + 28;
    let shifted: Vec<_> = (start..start + 40)
        .filter(|day| {
            crate::index_consistency::eligibility_of(*day)
                == crate::index_consistency::Eligibility::Eligible
        })
        .take(16)
        .collect();
    let foreign = Fixture::with_days(false, &shifted);
    let reader = foreign.publish();
    let expected = original.expected();
    assert_eq!(reader.search_identity(), expected.search);
    let no_replay = ReplayBounds {
        bootstrap_work: 1,
        split_work: 1,
        memory_bytes: 1,
    };
    let why = verify_search_slot_bounded(
        &foreign.root.0,
        reader.identity(),
        reader.completion_digest(),
        BYTES,
        100_000,
        &expected,
        no_replay,
    )
    .unwrap_err();
    assert!(why.contains("declared native source"), "{why}");
    for training in [true, false] {
        let mut wrong_window = foreign.expected();
        if training {
            wrong_window.training.first_day += 1;
        } else {
            wrong_window.later.last_day -= 1;
        }
        let why = verify_search_slot_bounded(
            &foreign.root.0,
            reader.identity(),
            reader.completion_digest(),
            BYTES,
            100_000,
            &wrong_window,
            no_replay,
        )
        .unwrap_err();
        assert!(why.contains("measurement window"), "{why}");
    }
}

#[test]
fn checkpoint_replay_caps_cannot_be_enlarged_by_declared_producer_bounds() {
    let fixture = Fixture::new(false);
    let reader = fixture.publish();
    let expected = fixture.expected();
    verify_search_slot_bounded(
        &fixture.root.0,
        reader.identity(),
        reader.completion_digest(),
        BYTES,
        100_000,
        &expected,
        fixture.replay(),
    )
    .unwrap();
    for (limited, message) in [
        (
            ReplayBounds {
                bootstrap_work: 1,
                ..fixture.replay()
            },
            "bootstrap work",
        ),
        (
            ReplayBounds {
                split_work: 1,
                ..fixture.replay()
            },
            "CSCV",
        ),
        (
            ReplayBounds {
                memory_bytes: 1,
                ..fixture.replay()
            },
            "memory",
        ),
    ] {
        let why = verify_search_slot_bounded(
            &fixture.root.0,
            reader.identity(),
            reader.completion_digest(),
            BYTES,
            100_000,
            &expected,
            limited,
        )
        .unwrap_err();
        assert!(why.contains(message), "{why}");
    }
    assert_eq!(
        reader.completion_digest(),
        fixture.publish().completion_digest()
    );
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one regression follows the same native family through exact measurements, immutable reopening, lease lifetime and checkpoint-coordinate refusal"
)]
fn native_parents_reopen_with_exact_statistics_folds_and_all_three_daily_periods() {
    let fixture = Fixture::new(false);
    let reader = fixture.publish();
    let pin = reader.completion_digest();
    assert_eq!(reader.rows().len(), 2);
    assert_eq!(reader.search_identity(), fixture.facts.search);
    for (index, row) in reader.rows().iter().enumerate() {
        assert_eq!(row.folds.len(), 2);
        assert_eq!(
            row.folds.iter().map(|f| f.pessimistic_paisa).sum::<i64>(),
            fixture
                .later
                .get(index)
                .unwrap()
                .metrics()
                .pessimistic_paisa
        );
        let consistency = reader.consistency(pin, index).unwrap();
        assert_eq!(consistency.training_session_count, 8);
        assert_eq!(consistency.sessions.len(), 16);
        assert_eq!(
            consistency.training.first_day,
            fixture.training.first().unwrap().first_day()
        );
        assert_eq!(
            consistency.later.first_day,
            fixture.later.first().unwrap().first_day()
        );
        assert_eq!(consistency.evaluation.summary.missing_days, 0);
        assert_eq!(
            reader.combined_qualifies(pin, index).unwrap(),
            consistency.combined_qualifies(row.projection.verdict().status())
        );
    }
    let again = fixture.publish();
    assert_eq!(again.identity(), reader.identity());
    assert_eq!(again.completion_digest(), pin);
    let daily = reader.consistency_receipt();
    let owners: Vec<_> = [
        (NAMESPACE, reader.identity()),
        (candidates::NAMESPACE, fixture.facts.training.identity),
        (candidates::NAMESPACE, fixture.facts.later.identity),
        ("index-consistency-v1", daily.identity),
    ]
    .into_iter()
    .map(|(namespace, id)| {
        std::fs::File::open(
            fixture
                .root
                .0
                .join(namespace)
                .join(crate::identity_hex(&id))
                .join("owner.lock"),
        )
        .unwrap()
    })
    .collect();
    reader
        .with_current(|| {
            assert_eq!(reader.rows().len(), reader.consistencies().len());
            assert_eq!(
                reader.training().records().len(),
                reader.later().records().len()
            );
            for owner in &owners {
                assert!(owner.try_lock().is_err());
            }
            Ok(())
        })
        .unwrap();
    for owner in &owners {
        owner.try_lock().unwrap();
        owner.unlock().unwrap();
    }
    verify_search_slot(
        &fixture.root.0,
        reader.identity(),
        pin,
        BYTES,
        100_000,
        fixture.facts.search,
        0,
        0,
        std::slice::from_ref(&fixture.program),
    )
    .unwrap();
    for (search, batch, rung) in [
        ([88; 32], 0, 0),
        (fixture.facts.search, 1, 0),
        (fixture.facts.search, 0, 1),
    ] {
        assert!(
            verify_search_slot(
                &fixture.root.0,
                reader.identity(),
                pin,
                BYTES,
                100_000,
                search,
                batch,
                rung,
                std::slice::from_ref(&fixture.program)
            )
            .is_err()
        );
    }
    assert!(
        verify_search_slot(
            &fixture.root.0,
            reader.identity(),
            pin,
            BYTES,
            100_000,
            fixture.facts.search,
            0,
            0,
            &[Expression::parse("0 & !0").unwrap()]
        )
        .is_err()
    );
}

#[test]
fn missing_daily_native_parent_foreign_pin_or_changed_body_never_becomes_admission() {
    let fixture = Fixture::new(false);
    let reader = fixture.publish();
    let id = reader.identity();
    let pin = reader.completion_digest();
    assert!(reader.row([99; 32], 0).is_err());
    assert!(reader.row(pin, 2).is_err());
    assert!(Reader::open(&fixture.root.0, id, 128, 100_000).is_err());
    assert!(Reader::open(&fixture.root.0, id, BYTES, 1).is_err());
    let daily = reader.consistency_receipt();
    let path = fixture
        .root
        .0
        .join("index-consistency-v1")
        .join(crate::identity_hex(&daily.identity))
        .join("body.bin");
    let saved = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(Reader::open(&fixture.root.0, id, BYTES, 100_000).is_err());
    assert!(reader.combined_qualifies(pin, 0).is_err());
    std::fs::write(path, saved).unwrap();
    let path = fixture
        .root
        .0
        .join(candidates::NAMESPACE)
        .join(crate::identity_hex(&fixture.facts.training.identity))
        .join("body.bin");
    let mut bytes = std::fs::read(&path).unwrap();
    *bytes.last_mut().unwrap() ^= 1;
    std::fs::write(path, bytes).unwrap();
    assert!(Reader::open(&fixture.root.0, id, BYTES, 100_000).is_err());
    assert!(reader.row(pin, 0).is_err());
}

#[test]
fn zero_trade_family_remains_in_statistics_and_does_not_pass_vacuously() {
    let fixture = Fixture::new(true);
    let reader = fixture.publish();
    assert_eq!(reader.rows().len(), 2);
    for (index, row) in reader.rows().iter().enumerate() {
        assert_eq!(row.romano.first(), Some(&2));
        assert_eq!(
            row.projection.values().trades,
            runner::admission::ObservedU64V1::Measured(0)
        );
        let days = reader
            .consistency(reader.completion_digest(), index)
            .unwrap();
        assert_eq!(days.evaluation.summary.no_trade_days, 16);
        assert!(
            !reader
                .combined_qualifies(reader.completion_digest(), index)
                .unwrap()
        );
    }
}

#[test]
fn whole_family_scope_work_and_codec_bounds_refuse_without_omitting_rows() {
    let fixture = Fixture::new(false);
    let mut facts = fixture.copy_facts();
    facts.allocation = runner::search_allocation_v1::allocate(0, 1, 1_000_000).unwrap();
    assert!(numeric::measure(&fixture.training, &fixture.later, &facts).is_err());
    let mut facts = fixture.copy_facts();
    facts.bounds.memory_bytes = 1;
    assert!(numeric::measure(&fixture.training, &fixture.later, &facts).is_err());
    let mut facts = fixture.copy_facts();
    facts.bounds.split_work = 1;
    assert!(numeric::measure(&fixture.training, &fixture.later, &facts).is_err());
    let mut facts = fixture.copy_facts();
    facts.bounds.bootstrap_work = 1;
    assert!(numeric::measure(&fixture.training, &fixture.later, &facts).is_err());
    assert!(
        numeric::measure(
            fixture.training.get(..1).unwrap(),
            &fixture.later,
            &fixture.facts
        )
        .is_err()
    );
    let (statistics, rows) =
        numeric::measure(&fixture.training, &fixture.later, &fixture.facts).unwrap();
    let id = identity(&fixture.facts);
    let bytes = codec::encode(
        id,
        &Body {
            facts: fixture.copy_facts(),
            statistics,
            rows,
        },
    )
    .unwrap();
    for cut in [0, 7, 39, bytes.len() - 1] {
        assert!(codec::decode(bytes.get(..cut).unwrap(), id, 100_000).is_err());
    }
    assert!(codec::decode(&bytes, [55; 32], 100_000).is_err());
    let mut under_budget = fixture.copy_facts();
    under_budget.bounds.bytes = u64::try_from(bytes.len()).unwrap() - 1;
    let changed_identity = identity(&under_budget);
    let mut bad_budget = bytes.clone();
    bad_budget
        .get_mut(8..40)
        .unwrap()
        .copy_from_slice(&changed_identity);
    let at = 8
        + 32
        + Policy::BYTE_LEN
        + crate::index_consistency::Policy::BYTE_LEN
        + 160
        + runner::admission::ADMISSION_POLICY_CANONICAL_LEN_V1
        + 4 * 8;
    bad_budget
        .get_mut(at..at + 8)
        .unwrap()
        .copy_from_slice(&under_budget.bounds.bytes.to_le_bytes());
    assert!(codec::decode(&bad_budget, changed_identity, 100_000).is_err());
    let mut bad = bytes;
    bad.push(0);
    assert!(codec::decode(&bad, id, 100_000).is_err());
}

#[test]
fn a_resealed_foreign_statistical_claim_cannot_pass_the_native_parent_reconciliation() {
    let fixture = Fixture::new(false);
    let (mut statistics, rows) =
        numeric::measure(&fixture.training, &fixture.later, &fixture.facts).unwrap();
    *statistics.white.first_mut().unwrap() ^= 1;
    let id = identity(&fixture.facts);
    let body = codec::encode(
        id,
        &Body {
            facts: fixture.copy_facts(),
            statistics,
            rows,
        },
    )
    .unwrap();
    let pin = save(&fixture.root.0, NAMESPACE, id, &body);
    let daily = numeric::daily(&fixture.training, &fixture.later, &fixture.facts).unwrap();
    daily::produce(&fixture.root.0, id, pin, daily, BYTES, || Ok(())).unwrap();
    let why = Reader::open(&fixture.root.0, id, BYTES, 100_000)
        .err()
        .unwrap();
    assert!(why.contains("does not reproduce"), "{why}");
}

#[test]
fn tighter_sufficient_replay_limits_preserve_original_statistics_and_completion() {
    let fixture = Fixture::new(false);
    let saved = fixture.publish();
    let work = saved.replay_work().unwrap();
    let limits = ReplayBounds {
        bootstrap_work: work.bootstrap_work,
        split_work: work.split_work,
        memory_bytes: BYTES / 2,
    };
    assert!(limits.bootstrap_work < fixture.facts.bounds.bootstrap_work);
    assert!(limits.split_work < fixture.facts.bounds.split_work);
    assert!(limits.memory_bytes < fixture.facts.bounds.memory_bytes);
    let reopened =
        Reader::open_with_replay_bounds(&fixture.root.0, saved.identity(), BYTES, 100_000, limits)
            .unwrap();
    assert_eq!(reopened.completion_digest(), saved.completion_digest());
    assert_eq!(reopened.body.facts.bounds, fixture.facts.bounds);
    assert_eq!(reopened.body.statistics, saved.body.statistics);
    assert_eq!(reopened.rows(), saved.rows());
    reopened.require_current().unwrap();
    saved.require_current().unwrap();
}

#[test]
fn sixteen_training_periods_refuse_unadmitted_cscv_payload_before_replay() {
    let fixture = Fixture::with_training_periods(true, &eligible_days(32), 16);
    for row in fixture.training.iter().chain(&fixture.later) {
        assert_eq!(row.periods().len(), 16);
        assert!(row.trades().is_empty());
    }
    let layout = crate::population_observations_v1::derive_layout(16).unwrap();
    assert_eq!(layout.segment_count(), 16);
    assert_eq!(layout.split_count(), 6_435);
    let saved = fixture.publish();
    assert_eq!(saved.body.statistics.splits.len(), 6_435);
    let payload = layout.split_count()
        * u64::try_from(std::mem::size_of::<(u64, u64)>() + 2 * std::mem::size_of::<Split>())
            .unwrap()
        + u64::try_from(2 * fixture.facts.count * std::mem::size_of::<i64>()).unwrap();
    let cap = 100_000;
    assert!(cap < payload);
    eprintln!(
        "CSCV admission regression: splits={}, native_split_bytes={}, retained/reproduced/mask/score_payload={payload}, current_cap={cap}",
        layout.split_count(),
        std::mem::size_of::<Split>()
    );
    let result = Reader::open_with_replay_bounds(
        &fixture.root.0,
        saved.identity(),
        BYTES,
        100_000,
        ReplayBounds {
            memory_bytes: cap,
            ..fixture.replay()
        },
    );
    assert!(
        result.is_err(),
        "cold replay accepted a cap below its known overlapping CSCV payload"
    );
    let why = result.err().unwrap();
    assert!(why.contains("memory"), "{why}");
    saved.require_current().unwrap();
}

#[test]
fn complete_cscv_payload_exact_and_minus_one_admission_preserves_saved_evidence() {
    let fixture = Fixture::with_training_periods(true, &eligible_days(32), 16);
    let saved = fixture.publish();
    let estimate = u64::try_from(
        numeric::required_memory(&fixture.training, &fixture.later, &fixture.facts).unwrap(),
    )
    .unwrap();
    assert!(estimate > u64::try_from(6_435 * std::mem::size_of::<Split>()).unwrap());
    assert!(estimate < fixture.facts.bounds.memory_bytes);
    eprintln!("complete single-stop payload estimate={estimate} bytes; not process RSS");

    let mut production = fixture.copy_facts();
    production.bounds.memory_bytes = estimate;
    let measured = numeric::measure(&fixture.training, &fixture.later, &production).unwrap();
    assert_eq!(measured.0.splits.len(), 6_435);
    assert_eq!(measured.1.len(), fixture.facts.count);
    production.bounds.memory_bytes -= 1;
    let why = numeric::measure(&fixture.training, &fixture.later, &production)
        .err()
        .unwrap();
    assert!(why.contains("memory"), "{why}");

    let limits = ReplayBounds {
        memory_bytes: estimate,
        ..fixture.replay()
    };
    let reopened =
        Reader::open_with_replay_bounds(&fixture.root.0, saved.identity(), BYTES, 100_000, limits)
            .unwrap();
    assert_eq!(reopened.identity(), saved.identity());
    assert_eq!(reopened.completion_digest(), saved.completion_digest());
    assert_eq!(reopened.body.facts.bounds, fixture.facts.bounds);
    assert_eq!(reopened.body.statistics, saved.body.statistics);
    assert_eq!(reopened.rows(), saved.rows());
    let why = Reader::open_with_replay_bounds(
        &fixture.root.0,
        saved.identity(),
        BYTES,
        100_000,
        ReplayBounds {
            memory_bytes: estimate - 1,
            ..limits
        },
    )
    .err()
    .unwrap();
    assert!(why.contains("memory"), "{why}");
    reopened.require_current().unwrap();
    saved.require_current().unwrap();
}

#[test]
fn resealed_cscv_layout_and_extent_are_rejected_before_numeric_replay() {
    let no_layout = Fixture::with_training_periods(true, &eligible_days(32), 15);
    let absence = no_layout.publish();
    assert_eq!(absence.body.statistics.layout, None);
    assert_eq!(absence.body.statistics.layout_digest, [0; 32]);
    assert!(absence.body.statistics.splits.is_empty());
    for variant in 0..6 {
        let fixture = if variant == 5 {
            Fixture::with_training_periods(true, &eligible_days(32), 15)
        } else {
            Fixture::new(true)
        };
        let (mut statistics, rows) =
            numeric::measure(&fixture.training, &fixture.later, &fixture.facts).unwrap();
        match variant {
            0 => {
                statistics.splits.pop();
            }
            1 => statistics
                .splits
                .push(statistics.splits.first().unwrap().clone()),
            2 => {
                statistics.layout = None;
                statistics.layout_digest = [0; 32];
                statistics.splits.clear();
            }
            3 => statistics.layout_digest = [57; 32],
            _ => statistics.layout = Some([16, 16, 1, 6_435]),
        }
        let id = identity(&fixture.facts);
        let bytes = codec::encode(
            id,
            &Body {
                facts: fixture.copy_facts(),
                statistics,
                rows,
            },
        )
        .unwrap();
        let pin = save(&fixture.root.0, NAMESPACE, id, &bytes);
        let days = numeric::daily(&fixture.training, &fixture.later, &fixture.facts).unwrap();
        daily::produce(&fixture.root.0, id, pin, days, BYTES, || Ok(())).unwrap();
        let why = Reader::open_with_replay_bounds(
            &fixture.root.0,
            id,
            BYTES,
            100_000,
            ReplayBounds {
                bootstrap_work: 1,
                split_work: 1,
                memory_bytes: 1,
            },
        )
        .err()
        .unwrap();
        assert!(why.contains("saved CSCV layout or split extent"), "{why}");
    }
}

#[test]
fn saved_limits_cannot_enlarge_current_cold_replay_work_or_memory_admission() {
    let fixture = Fixture::new(false);
    let reader = fixture.publish();
    let limits = ReplayBounds {
        bootstrap_work: 10_000_000,
        split_work: 10_000_000,
        memory_bytes: BYTES,
    };
    let reopened =
        Reader::open_with_replay_bounds(&fixture.root.0, reader.identity(), BYTES, 100_000, limits)
            .unwrap();
    assert_eq!(reopened.completion_digest(), reader.completion_digest());
    assert_eq!(reopened.rows(), reader.rows());
    for (limited, expected) in [
        (
            ReplayBounds {
                bootstrap_work: 1,
                ..limits
            },
            "bootstrap work",
        ),
        (
            ReplayBounds {
                split_work: 1,
                ..limits
            },
            "CSCV",
        ),
        (
            ReplayBounds {
                memory_bytes: 1,
                ..limits
            },
            "memory",
        ),
        (
            ReplayBounds {
                memory_bytes: 0,
                ..limits
            },
            "positive",
        ),
    ] {
        let why = Reader::open_with_replay_bounds(
            &fixture.root.0,
            reader.identity(),
            BYTES,
            100_000,
            limited,
        )
        .err()
        .unwrap();
        assert!(why.contains(expected), "{why}");
    }

    // A newly resealed, syntactically valid body offers enormous historical
    // work/memory limits. It cannot authorize that work in a current API read.
    let mut facts = fixture.copy_facts();
    facts.bounds.bootstrap_work = u64::MAX;
    facts.bounds.split_work = u64::MAX;
    facts.bounds.memory_bytes = u64::MAX;
    facts.procedure = PopulationStatisticsProcedureV2::new(u64::MAX / 16, 49, 2).unwrap();
    let id = identity(&facts);
    let body = codec::encode(
        id,
        &Body {
            facts,
            statistics: reader.body.statistics.clone(),
            rows: reader.body.rows.clone(),
        },
    )
    .unwrap();
    let pin = save(&fixture.root.0, NAMESPACE, id, &body);
    let records = numeric::daily(&fixture.training, &fixture.later, &facts).unwrap();
    daily::produce(&fixture.root.0, id, pin, records, BYTES, || Ok(())).unwrap();
    let why = Reader::open_with_replay_bounds(&fixture.root.0, id, BYTES, 100_000, limits)
        .err()
        .unwrap();
    assert!(why.contains("bootstrap work"), "{why}");
    reader.require_current().unwrap();
}

#[test]
fn cscv_segment_totals_reproduce_every_per_period_split_exactly() {
    // D-0604. Summing each segment once must change no split, digest or
    // placement, and a row whose absolute total could overflow keeps the
    // per-period loop.
    use crate::population_observations_v1::{canonical_masks, derive_layout};
    let layout = derive_layout(24).unwrap();
    let width = usize::try_from(layout.periods_per_segment()).unwrap();
    assert_eq!(width, 2);
    let rows: Vec<Vec<i64>> = (0..5_i64)
        .map(|row| (0..24_i64).map(|p| (row * 37 + p * 11) % 23 - 11).collect())
        .collect();
    let totals = numeric::segment_sums(&rows, width).unwrap().unwrap();
    assert!(totals.iter().all(|row| row.len() == 12));
    let masks = canonical_masks(layout).unwrap();
    assert_eq!(masks.len(), 462);
    for (train, test) in masks {
        assert_eq!(
            numeric::split(&rows, width, layout.digest(), train, test).unwrap(),
            numeric::split(&totals, 1, layout.digest(), train, test).unwrap()
        );
    }
    // Both sides of the exactness bound.
    let at_bound = [vec![i64::MAX - 1, 1, 0, 0]];
    assert_eq!(
        numeric::segment_sums(&at_bound, 2).unwrap(),
        Some(vec![vec![i64::MAX, 0]])
    );
    for past in [vec![i64::MAX - 1, 2, 0, 0], vec![i64::MIN, 0, 0, 0]] {
        assert_eq!(numeric::segment_sums(&[past], 2).unwrap(), None);
    }
}

#[test]
fn a_shared_walk_refusal_keeps_the_message_its_separate_procedure_produced() {
    use runner::bootstrap_zero_v2::{FamilyRefusal, Refusal};
    assert_eq!(
        numeric::refusal(FamilyRefusal::RomanoWolf(Refusal::NonzeroConstant {
            strategy: 3
        })),
        "single-stop complete later Romano-Wolf procedure refused: NonzeroConstant { strategy: 3 }"
    );
    assert_eq!(
        numeric::refusal(FamilyRefusal::White),
        "single-stop later White procedure refused"
    );
    assert_eq!(
        numeric::refusal(FamilyRefusal::Spa),
        "single-stop later SPA procedure refused"
    );
}

#[test]
fn saved_statistics_are_the_three_separate_procedures_bit_for_bit() {
    // A qualification written by the three-pass build must still re-verify:
    // `check()` compares a fresh `measure` with the saved bytes. So every
    // bootstrap-derived word `measure` saves is compared here with the same
    // word taken from the three separate public procedures on the same
    // returns, for a trading family and for an all-zero one.
    use runner::bootstrap_zero_v2 as zero;
    for fixture in [Fixture::new(false), Fixture::new(true)] {
        let (statistics, rows) =
            numeric::measure(&fixture.training, &fixture.later, &fixture.facts).unwrap();
        let returns: Vec<Vec<i64>> = fixture
            .later
            .iter()
            .map(|row| {
                row.periods()
                    .iter()
                    .map(|day| day.pessimistic_paisa)
                    .collect()
            })
            .collect();
        let draws = usize::try_from(fixture.facts.procedure.draws()).unwrap();
        let block = usize::try_from(fixture.facts.procedure.block_length()).unwrap();
        let seed = fixture.facts.procedure.seed();
        let romano = zero::evaluate(
            &returns,
            draws,
            seed,
            block,
            zero::Bounds {
                max_work: fixture.facts.bounds.bootstrap_work,
                max_bytes: fixture.facts.bounds.memory_bytes,
            },
        )
        .unwrap();
        let white = runner::bootstrap::white_reality_check_receipt_v1(&returns, draws, seed, block)
            .unwrap();
        let spa = runner::bootstrap::spa_receipt_v1(&returns, draws, seed, block).unwrap();
        let words = |bits: u64, p: u64, numerator: usize, denominator: usize, matched: usize| {
            [
                bits,
                p,
                u64::try_from(numerator).unwrap(),
                u64::try_from(denominator).unwrap(),
                u64::try_from(matched).unwrap(),
            ]
        };
        assert_eq!(
            statistics.white,
            words(
                white.statistic_bits(),
                white.p_value_bits(),
                white.exact_p_value().numerator(),
                white.exact_p_value().denominator(),
                white.matched_or_exceeded()
            )
        );
        assert_eq!(
            statistics.spa,
            words(
                spa.statistic_bits(),
                spa.p_value_bits(),
                spa.exact_p_value().numerator(),
                spa.exact_p_value().denominator(),
                spa.matched_or_exceeded()
            )
        );
        assert_eq!(statistics.family, romano.family_digest());
        assert_eq!(statistics.shared, romano.shared_digest());
        assert_eq!(rows.len(), romano.strategies());
        for (index, row) in rows.iter().enumerate() {
            let candidate = romano.candidate(index).unwrap();
            assert_eq!(row.romano[1], candidate.p_value().numerator());
            assert_eq!(row.romano[2], candidate.p_value().denominator());
            match candidate.shared() {
                Some(shared) => {
                    assert_eq!(row.romano[3], 1);
                    assert_eq!(row.romano[6], shared.observed_statistic().to_bits());
                    assert_eq!(
                        row.romano[7],
                        u64::try_from(shared.strict_exceedances()).unwrap()
                    );
                    assert_eq!(
                        row.romano[10],
                        u64::try_from(shared.adjusted_p_value().numerator()).unwrap()
                    );
                }
                None => assert_eq!(row.romano[3], 0),
            }
        }
    }
}
