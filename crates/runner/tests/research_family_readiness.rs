//! Generated scope/price fixtures only; no institutional admission is implied.
use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind, Segment};
use brutex_core::universe::{FNO_UNDERLYINGS, NIFTY_TOTAL_MARKET};
use runner::excursion::Side;
use runner::exit_grid_policy::{
    ExecutionResolutionV1, ExecutionSeriesV1, ExitGridErrorV1, ExitGridPolicyV1,
    ExitGridSelectorV1, ForcedStopV1, InstrumentFamilyV1, RangeResolutionV1, RatioLimitsV1,
    RationalPercentileV1, RungPlanV1, printed_ohlcv_cost_model_id_v1,
};
use runner::research_family::{
    RESEARCH_FAMILY_BYTES_V1, RESEARCH_FAMILY_CAPACITY_V1, ResearchFamilyErrorV1, ResearchFamilyV1,
    ResearchScopeV1, membership_snapshot_digest_v1,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn runtime_scope_matches_the_independent_snapshot_intersection() -> TestResult {
    assert_eq!(RESEARCH_FAMILY_CAPACITY_V1, FNO_UNDERLYINGS.len() + 2);
    let mut keys = Vec::new();
    for (_, name) in InstrumentKey::SWEPT {
        keys.push(InstrumentKey::index(Exchange::Nse, name)?);
    }
    let mut cash_count = 0;
    for name in NIFTY_TOTAL_MARKET {
        let key = InstrumentKey::cash(Exchange::Nse, name)?;
        let expected = FNO_UNDERLYINGS.contains(&name);
        let family = ResearchFamilyV1::new(key);
        assert_eq!(family.is_ok(), expected, "{name}");
        if expected {
            let family = family?;
            assert!(family.is_cash());
            assert_eq!(family.instrument(), key);
            assert_eq!(family.legacy_index_family(), None);
            assert_eq!(family.membership_digest(), membership_snapshot_digest_v1());
            assert_ne!(family.membership_digest(), [0; 32]);
            keys.push(key);
            cash_count += 1;
        }
    }
    assert!(
        cash_count > 100,
        "the oracle must cover the actual cash snapshot"
    );
    let scope = ResearchScopeV1::new(&keys, keys.len())?;
    assert_eq!(scope.families().len(), cash_count + 2);
    for family in scope.families().iter().take(2) {
        assert!(!family.is_cash());
    }
    assert!(keys.iter().all(|key| scope.contains(key)));
    for (index, family) in scope.families().iter().enumerate() {
        assert_eq!(scope.position(&family.instrument()), Some(index));
    }
    keys.reverse();
    assert_eq!(scope, ResearchScopeV1::new(&keys, usize::MAX)?);
    assert_eq!(
        scope
            .families()
            .first()
            .ok_or("first")?
            .legacy_index_family(),
        Some(InstrumentFamilyV1::Nifty)
    );
    assert_eq!(
        scope
            .families()
            .get(1)
            .ok_or("second")?
            .legacy_index_family(),
        Some(InstrumentFamilyV1::BankNifty)
    );
    assert_eq!(
        scope.families().first().ok_or("first")?.membership_digest(),
        [0; 32]
    );
    Ok(())
}

#[test]
fn scope_refusals_report_the_named_problem_and_actual_physical_limit() {
    let messages = [
        (
            ResearchFamilyErrorV1::UnsupportedInstrument,
            "unsupported research instrument",
        ),
        (ResearchFamilyErrorV1::EmptyScope, "research scope is empty"),
        (
            ResearchFamilyErrorV1::DuplicateInstrument,
            "repeats an instrument",
        ),
        (
            ResearchFamilyErrorV1::AllocationRefused,
            "allocation refused",
        ),
        (
            ResearchFamilyErrorV1::InvalidEncoding,
            "encoding or membership snapshot is invalid",
        ),
    ];
    for (error, expected) in messages {
        assert!(error.to_string().contains(expected));
    }
    assert_eq!(
        ResearchFamilyErrorV1::ScopeLimitExceeded {
            requested: 9,
            maximum: 7
        }
        .to_string(),
        "research scope offers 9 families beyond its 7 family limit"
    );
}

#[test]
fn scope_refuses_impostors_duplicates_and_limits_without_silent_truncation() -> TestResult {
    let cash = InstrumentKey::cash(Exchange::Nse, "RELIANCE")?;
    let expiry = Expiry::new(2026, 9, 24)?;
    let impostors = [
        InstrumentKey::cash(Exchange::Nse, "NIFTY")?,
        InstrumentKey::cash(Exchange::Nse, "BANKNIFTY")?,
        InstrumentKey::index(Exchange::Nse, "INDIAVIX")?,
        InstrumentKey::cash(Exchange::Nse, "ZZQXNOTFNO")?,
        InstrumentKey::cash(Exchange::Bse, "RELIANCE")?,
        InstrumentKey {
            segment: Segment::Fno,
            kind: Kind::Future { expiry },
            ..cash
        },
        InstrumentKey {
            segment: Segment::Index,
            ..cash
        },
    ];
    let scope = ResearchScopeV1::new(&[cash], 1)?;
    for key in impostors {
        assert_eq!(
            ResearchFamilyV1::new(key),
            Err(ResearchFamilyErrorV1::UnsupportedInstrument)
        );
        assert!(!scope.contains(&key));
    }
    assert_eq!(
        ResearchScopeV1::new(&[], 1),
        Err(ResearchFamilyErrorV1::EmptyScope)
    );
    assert_eq!(
        ResearchScopeV1::new(&[cash, cash], 2),
        Err(ResearchFamilyErrorV1::DuplicateInstrument)
    );
    assert_eq!(
        ResearchScopeV1::new(&[cash], 0),
        Err(ResearchFamilyErrorV1::ScopeLimitExceeded {
            requested: 1,
            maximum: 0
        })
    );
    let excess = vec![cash; RESEARCH_FAMILY_CAPACITY_V1 + 1];
    assert_eq!(
        ResearchScopeV1::new(&excess, usize::MAX),
        Err(ResearchFamilyErrorV1::ScopeLimitExceeded {
            requested: excess.len(),
            maximum: RESEARCH_FAMILY_CAPACITY_V1
        })
    );
    let other = InstrumentKey::cash(Exchange::Nse, "ADANIENT")?;
    assert_ne!(scope.digest(), ResearchScopeV1::new(&[other], 1)?.digest());
    assert_ne!(
        scope.digest(),
        ResearchScopeV1::new(&[cash, other], 2)?.digest()
    );
    Ok(())
}

#[test]
fn fixed_family_codec_rejects_every_changed_byte_and_wrong_length() -> TestResult {
    for key in [
        InstrumentKey::index(Exchange::Nse, "NIFTY")?,
        InstrumentKey::index(Exchange::Nse, "BANKNIFTY")?,
        InstrumentKey::cash(Exchange::Nse, "RELIANCE")?,
        InstrumentKey::cash(Exchange::Nse, "ADANIENT")?,
    ] {
        let family = ResearchFamilyV1::new(key)?;
        let encoded = family.encode();
        assert_eq!(encoded.len(), RESEARCH_FAMILY_BYTES_V1);
        assert_eq!(encoded.get(72..104), Some(family.digest().as_slice()));
        assert_ne!(family.digest(), [0; 32]);
        assert_eq!(ResearchFamilyV1::decode(&encoded)?, family);
        for index in 0..encoded.len() {
            let mut corrupt = encoded;
            *corrupt.get_mut(index).ok_or("codec offset")? ^= 1;
            assert_eq!(
                ResearchFamilyV1::decode(&corrupt),
                Err(ResearchFamilyErrorV1::InvalidEncoding),
                "byte {index}"
            );
        }
        for length in 0..encoded.len() {
            assert!(ResearchFamilyV1::decode(encoded.get(..length).ok_or("prefix")?).is_err());
        }
        let mut excess = encoded.to_vec();
        excess.push(0);
        assert!(ResearchFamilyV1::decode(&excess).is_err());
    }
    Ok(())
}

#[test]
fn membership_identity_binds_the_complete_ordered_snapshot_tables() {
    // Independently assemble the documented canonical byte stream in one
    // buffer. A constant membership identifier would let stale cash evidence
    // pass even after the approved snapshot changed.
    let mut bytes = b"brutex.runner.cash-membership-snapshot.v1\0FNO\0".to_vec();
    bytes.extend_from_slice(FNO_UNDERLYINGS.join("\0").as_bytes());
    bytes.extend_from_slice(b"\0NTM\0");
    bytes.extend_from_slice(NIFTY_TOTAL_MARKET.join("\0").as_bytes());
    bytes.push(0);
    assert_eq!(
        membership_snapshot_digest_v1(),
        brutex_core::blake3::hash(&bytes)
    );
    assert_eq!(
        membership_snapshot_digest_v1(),
        membership_snapshot_digest_v1()
    );
}

fn policy(side: Side, cells: u64) -> Result<ExitGridPolicyV1, ExitGridErrorV1> {
    policy_with_forced(side, cells, ForcedStopV1::Disabled)
}

fn policy_with_forced(
    side: Side,
    cells: u64,
    forced: ForcedStopV1,
) -> Result<ExitGridPolicyV1, ExitGridErrorV1> {
    let half = RationalPercentileV1::new(1, 2)?;
    let full = RationalPercentileV1::new(1, 1)?;
    ExitGridPolicyV1::new(
        ExecutionResolutionV1::OneMinuteOhlcv,
        RangeResolutionV1::PpmCeiling,
        side,
        RungPlanV1::new(vec![half, full], vec![half, full], vec![half, full], 2)?,
        RatioLimitsV1::new(100, 1_000, 16)?,
        cells,
        ExitGridSelectorV1::GuaranteedFloor,
        printed_ohlcv_cost_model_id_v1(),
        forced,
        3,
        2,
    )
}

#[test]
fn exact_observed_required_stop_retains_its_public_coordinate() -> TestResult {
    let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE")?;
    let bars = training();
    let source = ExecutionSeriesV1::new(&key, "generated-test", "generated-build", [7; 32], &bars)?;
    for side in [Side::Long, Side::Short] {
        for (ppm, index) in [(800, 0), (1600, 1)] {
            let required =
                policy_with_forced(side, 10_000, ForcedStopV1::RequireExactObserved(ppm))?
                    .resolve_research_attested(source)?;
            assert_eq!(required.forced_stop_index(), Some(index));
            assert_eq!(required.stop_levels_ppm().get(index), Some(&ppm));
            assert!(required.digest_is_valid());
        }
        assert!(matches!(
            policy_with_forced(side, 10_000, ForcedStopV1::RequireExactObserved(801))?
                .resolve_research_attested(source),
            Err(ExitGridErrorV1::ForcedStopNotObserved(801))
        ));
    }
    Ok(())
}

fn training() -> Vec<indicators::Candle> {
    (0..16)
        .map(|minute| {
            indicators::Candle::new(
                (225 + minute) * 60_000_000,
                1_000_000,
                1_000_000 + (minute + 1) * 100,
                1_000_000 - (minute + 1) * 100,
                1_000_000,
                1,
                indicators::OI_NULL,
            )
        })
        .collect()
}

#[test]
fn cash_uses_identical_observed_levels_without_becoming_a_legacy_index() -> TestResult {
    let bars = training();
    let nifty = InstrumentKey::index(Exchange::Nse, "NIFTY")?;
    let cash = InstrumentKey::cash(Exchange::Nse, "RELIANCE")?;
    for side in [Side::Long, Side::Short] {
        let policy = policy(side, 10_000)?;
        let index_source =
            ExecutionSeriesV1::new(&nifty, "generated-test", "generated-build", [7; 32], &bars)?;
        let cash_source =
            ExecutionSeriesV1::new(&cash, "generated-test", "generated-build", [7; 32], &bars)?;
        let legacy = policy.resolve_attested(index_source)?;
        let research_index = policy.resolve_research_attested(index_source)?;
        let research_cash = policy.resolve_research_attested(cash_source)?;
        assert_eq!(legacy.family(), InstrumentFamilyV1::Nifty);
        assert!(matches!(
            policy.resolve_attested(cash_source),
            Err(ExitGridErrorV1::UnsupportedInstrument)
        ));
        for resolved in [&research_index, &research_cash] {
            assert!(resolved.digest_is_valid());
            assert_eq!(resolved.stop_levels_ppm(), &[800, 1600]);
            assert_eq!(resolved.target_levels_ppm(), &[800, 1600]);
            assert_eq!(resolved.trail_levels_ppm(), &[1600, 3200]);
            assert_eq!(resolved.cell_count(), 51);
            assert_eq!(resolved.policy_digest(), legacy.policy_digest());
            assert_eq!(resolved.training_digest(), legacy.training_digest());
            assert_eq!(resolved.training_bars(), 16);
            assert_eq!(resolved.feed_digest(), legacy.feed_digest());
            assert_eq!(resolved.commit_digest(), legacy.commit_digest());
            assert_eq!(resolved.calendar_digest(), [7; 32]);
            assert_eq!(resolved.training_first_ts_micros(), 225 * 60_000_000);
            assert_eq!(resolved.training_last_ts_micros(), 240 * 60_000_000);
            assert_eq!(resolved.side(), side);
            assert_eq!(resolved.policy(), &policy);
            assert_eq!(resolved.forced_stop_index(), None);
            assert_eq!(resolved.stop_levels_ppm(), legacy.stop_levels_ppm());
            assert_eq!(resolved.target_levels_ppm(), legacy.target_levels_ppm());
            assert_eq!(resolved.trail_levels_ppm(), legacy.trail_levels_ppm());
            assert_eq!(resolved.ratio_pairs(), legacy.ratio_pairs());
            assert_eq!(resolved.cell_count(), legacy.cell_count());
            assert_eq!(resolved.ladders()?, legacy.ladders()?);
            assert_ne!(resolved.digest(), legacy.digest());
        }
        assert_eq!(research_cash.family().legacy_index_family(), None);
        assert_ne!(research_index.digest(), research_cash.digest());
    }
    Ok(())
}

#[test]
fn legacy_resolution_identity_matches_the_recorded_pre_extraction_library() -> TestResult {
    // Captured from the prior compiled runner9d3d9971a1dd87bf, not calculated
    // through the new resolver. Ignored baseline helper/log records all inputs.
    let expected = [
        (
            "NIFTY",
            Side::Long,
            [
                154, 47, 234, 224, 209, 61, 29, 100, 203, 10, 249, 89, 244, 235, 127, 47, 64, 19,
                20, 144, 207, 19, 12, 136, 194, 114, 169, 52, 143, 119, 218, 212,
            ],
        ),
        (
            "NIFTY",
            Side::Short,
            [
                238, 132, 169, 244, 243, 106, 187, 13, 187, 80, 225, 227, 136, 168, 162, 138, 165,
                81, 71, 144, 130, 186, 9, 209, 175, 8, 104, 86, 110, 68, 29, 217,
            ],
        ),
        (
            "BANKNIFTY",
            Side::Long,
            [
                241, 6, 203, 248, 97, 180, 79, 156, 145, 50, 85, 238, 2, 18, 195, 172, 66, 188, 77,
                82, 221, 111, 50, 27, 158, 13, 16, 9, 230, 206, 80, 152,
            ],
        ),
        (
            "BANKNIFTY",
            Side::Short,
            [
                107, 130, 253, 153, 231, 142, 246, 168, 74, 184, 92, 11, 251, 36, 102, 177, 123,
                179, 251, 152, 128, 38, 235, 38, 142, 36, 17, 131, 197, 217, 228, 169,
            ],
        ),
    ];
    let bars = training();
    for (name, side, expected) in expected {
        let key = InstrumentKey::index(Exchange::Nse, name)?;
        let series =
            ExecutionSeriesV1::new(&key, "generated-test", "generated-build", [7; 32], &bars)?;
        let resolved = policy(side, 10_000)?.resolve_attested(series)?;
        assert_eq!(resolved.digest(), expected, "{name} {side:?}");
    }
    Ok(())
}

#[test]
fn new_resolution_retains_physical_refusals_and_exact_source_identity() -> TestResult {
    let bars = training();
    let cash = InstrumentKey::cash(Exchange::Nse, "RELIANCE")?;
    let base = ExecutionSeriesV1::new(&cash, "generated-test", "generated-build", [7; 32], &bars)?;
    assert!(matches!(
        policy(Side::Long, 1)?.resolve_research_attested(base),
        Err(ExitGridErrorV1::CellLimitExceeded { .. })
    ));
    let policy = policy(Side::Long, 10_000)?;
    let original = policy.resolve_research_attested(base)?;
    for (feed, commit, calendar) in [
        ("foreign-feed", "generated-build", [7; 32]),
        ("generated-test", "foreign-build", [7; 32]),
        ("generated-test", "generated-build", [8; 32]),
    ] {
        let source = ExecutionSeriesV1::new(&cash, feed, commit, calendar, &bars)?;
        assert_ne!(
            original.digest(),
            policy.resolve_research_attested(source)?.digest()
        );
    }
    let mut changed = bars.clone();
    changed.first_mut().ok_or("first")?.volume += 1;
    let source = ExecutionSeriesV1::new(
        &cash,
        "generated-test",
        "generated-build",
        [7; 32],
        &changed,
    )?;
    let revised = policy.resolve_research_attested(source)?;
    assert_eq!(original.stop_levels_ppm(), revised.stop_levels_ppm());
    assert_ne!(original.training_digest(), revised.training_digest());
    assert_ne!(original.digest(), revised.digest());
    changed.first_mut().ok_or("first")?.low = 0;
    let source = ExecutionSeriesV1::new(
        &cash,
        "generated-test",
        "generated-build",
        [7; 32],
        &changed,
    )?;
    assert!(policy.resolve_research_attested(source).is_err());
    Ok(())
}

#[test]
fn eligible_cash_prices_and_replays_every_boolean_coordinate_without_index_alias() -> TestResult {
    use indicators::column::Column;
    use indicators::evaluator::{Calendar, Evaluator, Widths};
    use runner::exit_grid_policy::expression_execution::ExpressionExecutionRunV1;
    use runner::expression::Expression;
    use runner::grid::Chosen;
    use runner::identity::{Direction, Params, Run, data_digest_with_execution};
    use runner::outcome::Horizon;

    let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE")?;
    let family = ResearchFamilyV1::new(key)?;
    assert!(family.is_cash());
    let availability = if family.is_cash() {
        indicators::vwap::Availability::Present
    } else {
        indicators::vwap::Availability::Absent
    };
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned()?,
        availability,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = Column::build(&bars, &mut evaluator);
    assert!(!column.is_empty());
    let program = Expression::parse("52 | !52").map_err(|why| format!("{why:?}"))?;
    let horizon = Horizon::bars(5).ok_or("horizon")?;
    let source = ExecutionSeriesV1::new(
        &key,
        "zerodha",
        "generated-cash-authority-fixture",
        [7; 32],
        &bars,
    )?;
    let foreign_key = InstrumentKey::cash(Exchange::Nse, "ADANIENT")?;
    let foreign_source = ExecutionSeriesV1::new(
        &foreign_key,
        "zerodha",
        "generated-cash-authority-fixture",
        [7; 32],
        &bars,
    )?;
    for (side, direction) in [
        (Side::Long, Direction::Long),
        (Side::Short, Direction::Short),
    ] {
        let policy = policy(side, 10_000)?;
        let resolved = policy.resolve_research_attested(source)?;
        let attested = resolved.attest_training(source, &column, horizon)?;
        let raw = Run {
            mask: program.referenced(),
            direction,
            instrument: &key,
            timeframe: "1min",
            params: Params::of(engine::Ladder::with_min_hits(1)),
            data_digest: data_digest_with_execution(&bars, None),
            commit: "generated-cash-authority-fixture",
            feed: "zerodha",
        };
        let run = ExpressionExecutionRunV1::new(&raw, &program, &bars, None)?;
        let evaluated = resolved.evaluate_expression_with_attested(&attested, &run)?;
        assert!(
            evaluated.summary().hits > 0,
            "cash's contributing volume makes the predicate knowable"
        );
        assert_eq!(evaluated.grid().signals, evaluated.summary().hits);
        let validated = resolved.validate_expression_evaluation(&evaluated)?;
        assert_eq!(
            u64::try_from(evaluated.grid().cells.len())?,
            resolved.cell_count()
        );
        let mut rows_seen = 0_usize;
        for (ordinal, cell) in evaluated.grid().cells.iter().enumerate() {
            let disposition =
                resolved.classify_expression_coordinate(&validated, Chosen::from_cell(cell))?;
            assert_eq!(disposition.ordinal(), ordinal);
            let rows =
                resolved.materialize_expression_coordinate(&attested, &validated, ordinal)?;
            assert_eq!(u64::try_from(rows.len())?, cell.trades);
            rows_seen = rows_seen.checked_add(rows.len()).ok_or("row sum")?;
        }
        assert!(
            rows_seen > 0,
            "the entire coordinate replay must measure actual fixture trades"
        );
        let foreign = policy.resolve_research_attested(foreign_source)?;
        let foreign_attested = foreign.attest_training(foreign_source, &column, horizon)?;
        assert!(
            foreign
                .evaluate_expression_with_attested(&foreign_attested, &run)
                .is_err(),
            "another cash family cannot reuse RELIANCE's source identity"
        );
        assert!(foreign.validate_expression_evaluation(&evaluated).is_err());
    }
    Ok(())
}
