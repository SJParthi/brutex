#![cfg(test)]
use super::*;
use crate::search_checkpoint::tests::Scratch;

#[test]
fn exact_candidate_evidence_survives_restart_and_missing_history_refuses() -> Result<(), String> {
    use indicators::evaluator::{Calendar, Evaluator, Widths};
    let scratch = Scratch::new().map_err(debug)?;
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(debug)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = indicators::column::Column::build(&bars, &mut evaluator);
    assert!(!column.bits().is_empty());
    let cursor = Cursor::new(&[0, 369]).map_err(debug)?;
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .map_err(debug)?;
    let run = Run {
        mask: cursor.alphabet(),
        direction: Direction::Undirected,
        instrument: &key,
        timeframe: "synthetic",
        params: Params::of(engine::Ladder::with_min_hits(1)),
        data_digest: [8; 32],
        commit: "generated-fixture",
        feed: "synthetic",
    };
    let identity = runner::expression::search_identity(&run, &cursor);
    let mut journal = Journal::open(&scratch.0, "expression-search-v1", identity)?;
    let state = execute(
        &scratch.0,
        &run,
        &column,
        &bars,
        &mut journal,
        State::new(cursor.clone()),
        (0, [0; 32]),
        (3, 10000),
        None,
    )?;
    assert_eq!(state.candidates, 3);
    assert!(!state.exhausted);
    drop(journal);
    let mut journal = Journal::open(&scratch.0, "expression-search-v1", identity)?;
    let saved = journal
        .latest(CHECKPOINT_MAX)?
        .ok_or("missing checkpoint")?;
    verify_history(&scratch.0, &journal, Some(&saved), &run, None)?;
    let state = execute(
        &scratch.0,
        &run,
        &column,
        &bars,
        &mut journal,
        State::decode(&saved.payload)?,
        (saved.sequence, saved.seal),
        (4, 10000),
        None,
    )?;
    assert_eq!(state.candidates, 7);
    assert_eq!(
        state.rows,
        7 * u64::try_from(column.bits().len()).map_err(debug)?
    );
    let latest = journal.latest(CHECKPOINT_MAX)?.ok_or("missing final")?;
    verify_history(&scratch.0, &journal, Some(&latest), &run, None)?;
    let previous = journal.read(State::decode(&latest.payload)?.previous.0, CHECKPOINT_MAX)?;
    let candidate = State::decode(&previous.payload)?
        .last
        .ok_or("missing seventh candidate")?;
    let mut oracle = cursor;
    let mut work = 0;
    let mut expected = None;
    for _ in 0..7 {
        let Step::Candidate(expression) = oracle.advance(10000, &mut work).map_err(debug)? else {
            return Err("oracle did not yield".to_owned());
        };
        expected = Some(expression);
    }
    assert_eq!(
        candidate.identity,
        candidate_identity(&run, &expected.ok_or("no oracle")?)
    );
    std::fs::remove_file(candidate_path(&scratch.0, &candidate)).map_err(debug)?;
    assert!(verify_history(&scratch.0, &journal, Some(&latest), &run, None).is_err());
    Ok(())
}

#[test]
fn malformed_controls_state_truncation_and_overflow_cannot_claim_completion() -> Result<(), String>
{
    for bits in ["0,0", "369,0", "999", "", "-1"] {
        assert!(
            Args::parse(&["test", "NSE-NIFTY", "1m", "2025", "1", bits, "1", "1", "1"]).is_err()
        );
    }
    for index in [6, 7, 8] {
        let mut args = [
            "test",
            "NSE-NIFTY",
            "1m",
            "2025",
            "1",
            "0,369",
            "1",
            "1",
            "1",
        ];
        *args.get_mut(index).ok_or("argument index")? = "0";
        assert!(Args::parse(&args).is_err());
    }
    let state = State::new(Cursor::new(&[0, 369]).map_err(debug)?);
    let mut bytes = state.encode();
    for length in 0..bytes.len() {
        assert!(State::decode(bytes.get(..length).ok_or("prefix")?).is_err());
    }
    assert!(!State::decode(&bytes)?.exhausted);
    *bytes.last_mut().ok_or("padding")? = 1;
    assert!(State::decode(&bytes).is_err());
    assert_eq!(add(7, 8)?, 15);
    assert!(add(u64::MAX, 1).is_err());
    Ok(())
}

#[test]
fn structurally_valid_sealed_early_exhaustion_and_wrong_successor_are_refused() -> Result<(), String>
{
    let scratch = Scratch::new().map_err(debug)?;
    let cursor = Cursor::new(&[0, 369]).map_err(debug)?;
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .map_err(debug)?;
    let run = Run {
        mask: cursor.alphabet(),
        direction: Direction::Undirected,
        instrument: &key,
        timeframe: "synthetic",
        params: Params::of(engine::Ladder::with_min_hits(1)),
        data_digest: [0; 32],
        commit: "generated-fixture",
        feed: "synthetic",
    };
    let mut finished = cursor.encode();
    finished
        .get_mut(10..12)
        .ok_or("length field")?
        .copy_from_slice(
            &u16::try_from(vocab::expression::MAX_INSTRUCTIONS)
                .map_err(debug)?
                .to_le_bytes(),
        );
    *finished.get_mut(14).ok_or("finished flag")? = 1;
    let mut forged = State::new(Cursor::decode(&finished).map_err(debug)?);
    forged.exhausted = true;
    assert!(
        State::decode(&forged.encode()).is_ok(),
        "shape alone cannot attest traversal"
    );
    let mut journal = Journal::open(
        &scratch.0,
        "expression-search-v1",
        runner::expression::search_identity(&run, &cursor),
    )?;
    journal.publish(&forged.encode(), CHECKPOINT_MAX)?;
    assert!(
        verify_history(
            &scratch.0,
            &journal,
            journal.latest(CHECKPOINT_MAX)?.as_ref(),
            &run,
            None
        )
        .is_err(),
        "even a valid public seal cannot certify an impossible transition"
    );
    let before = State::new(cursor.clone());
    let mut next = State::new(cursor);
    let Step::Candidate(first) = next.cursor.advance(4096, &mut next.work).map_err(debug)? else {
        return Err("expected first candidate".to_owned());
    };
    next.last = Some(Candidate {
        identity: candidate_identity(&run, &first),
        attempt: 1,
        summary: Summary {
            evaluated: 1,
            hits: 1,
            misses: 0,
            unknown: 0,
        },
    });
    assert!(verify_transition(&before, &next, &run).is_ok());
    next.last.as_mut().ok_or("candidate")?.identity =
        candidate_identity(&run, &Expression::parse("369").map_err(debug)?);
    assert!(verify_transition(&before, &next, &run).is_err());
    next.last = None;
    assert!(verify_transition(&before, &next, &run).is_err());
    Ok(())
}

#[test]
fn priced_search_restarts_with_both_sides_and_refuses_missing_trade_children() -> Result<(), String>
{
    use crate::expression_pricing::{Plan, Prepared, Verify};
    use indicators::evaluator::{Calendar, Evaluator, Widths};
    let scratch = Scratch::new().map_err(debug)?;
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(debug)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = indicators::column::Column::build(&bars, &mut evaluator);
    let plan = Plan {
        horizon: runner::outcome::Horizon::bars(3).ok_or("horizon")?,
        rungs: 1,
        step: 100,
        rules: crate::Rules::BASELINE,
    };
    let (execution, projected, execution_note) =
        crate::project_onto_execution(&bars, &column, None, true, plan.horizon)?;
    let proof = Verify {
        plan,
        execution_digest: runner::identity::data_digest(&execution),
    };
    let pricing = Prepared::new(execution, projected, plan, execution_note);
    let cursor = Cursor::new(&[0, 369]).map_err(debug)?;
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .map_err(debug)?;
    let run = Run {
        mask: cursor.alphabet(),
        direction: Direction::Undirected,
        instrument: &key,
        timeframe: "generated-1min",
        params: Params::of(engine::Ladder::with_min_hits(1)).with_policy(&plan.words()),
        data_digest: [11; 32],
        commit: "generated-fixture",
        feed: "synthetic",
    };
    let identity = runner::expression::search_identity(&run, &cursor);
    let mut journal = Journal::open(&scratch.0, "expression-search-v1", identity)?;
    let first = execute(
        &scratch.0,
        &run,
        &column,
        &bars,
        &mut journal,
        State::new(cursor),
        (0, [0; 32]),
        (3, 10000),
        Some(&pricing),
    )?;
    assert_eq!(first.candidates, 3);
    drop(journal);
    let mut journal = Journal::open(&scratch.0, "expression-search-v1", identity)?;
    let saved = journal.latest(CHECKPOINT_MAX)?.ok_or("first checkpoint")?;
    verify_history(&scratch.0, &journal, Some(&saved), &run, Some(proof))?;
    let resumed = execute(
        &scratch.0,
        &run,
        &column,
        &bars,
        &mut journal,
        State::decode(&saved.payload)?,
        (saved.sequence, saved.seal),
        (7, 10000),
        Some(&pricing),
    )?;
    assert_eq!(resumed.candidates, 10);
    let latest = journal.latest(CHECKPOINT_MAX)?.ok_or("latest checkpoint")?;
    verify_history(&scratch.0, &journal, Some(&latest), &run, Some(proof))?;
    let last = journal.read(State::decode(&latest.payload)?.previous.0, CHECKPOINT_MAX)?;
    let candidate = State::decode(&last.payload)?
        .last
        .ok_or("last priced candidate")?;
    let summary = crate::candidate_trades::read_model(
        &scratch.0,
        candidate.identity,
        candidate.attempt,
        crate::candidate_trades::Model::Expression,
        SIGNAL_FILE_MAX,
    )?
    .ok_or("priced catalog")?;
    assert_eq!((summary.tiers, summary.candidates), (1, 2));
    std::fs::remove_file(
        scratch
            .0
            .join("results/expression-candidate-trades-v1")
            .join(hex(&candidate.identity))
            .join(candidate.attempt.to_string())
            .join("0-1-0-trades.bin"),
    )
    .map_err(debug)?;
    assert!(verify_history(&scratch.0, &journal, Some(&latest), &run, Some(proof)).is_err());
    assert!(
        verify_history(&scratch.0, &journal, Some(&latest), &run, None).is_ok(),
        "signal evidence alone must not certify priced evidence"
    );
    Ok(())
}

const PRICING_KNOBS: [(&str, &str); 10] = [
    ("BRUTEX_MAX_MAE_PPM", "0"),
    ("BRUTEX_MIN_RR_BP", "125"),
    ("BRUTEX_MIN_WIN_RATE_BP", "5000"),
    ("BRUTEX_MIN_TRADES", "0"),
    ("BRUTEX_MIN_WEAKEST_BP", "0"),
    ("BRUTEX_MIN_RET_OVER_DD_BP", "500"),
    ("BRUTEX_MIN_FILL_HEADROOM_BP", "200"),
    ("BRUTEX_MIN_AVG_RR_BP", "150"),
    ("BRUTEX_TOP", "25"),
    ("BRUTEX_PROTECTED_EXITS", "1"),
];

struct ClearPricingKnobs;
impl Drop for ClearPricingKnobs {
    fn drop(&mut self) {
        crate::knobs::clear_all();
    }
}

fn pricing_knobs() -> ClearPricingKnobs {
    crate::knobs::clear_all();
    for (name, value) in PRICING_KNOBS {
        crate::knobs::set(name, value);
    }
    ClearPricingKnobs
}

#[test]
fn pricing_plan_matches_independent_cell_and_integer_admission() -> Result<(), String> {
    use crate::expression_pricing::Plan;
    let _serial = crate::knobs::serially();
    let _clear = pricing_knobs();
    let mut accepted = 0;
    let mut refused = 0;
    for rungs in (1_u64..=40).chain([u64::MAX]) {
        let n = u128::from(rungs);
        let triangular = n * (n + 1) / 2;
        let cells = (n + 2).saturating_mul(
            (n + 1)
                .saturating_mul(n + 1)
                .saturating_add(triangular.saturating_mul(triangular)),
        );
        for step in [
            1,
            100,
            i64::MAX.cast_unsigned(),
            i64::MAX.cast_unsigned() / rungs,
        ] {
            let words = ["1".to_owned(), rungs.to_string(), step.to_string()];
            let parsed = Plan::parse(&words.each_ref().map(String::as_str));
            let expected = step > 0 && n * u128::from(step) <= i64::MAX as u128 && cells <= 100_000;
            assert_eq!(parsed.is_ok(), expected, "rungs={rungs}, step={step}");
            if let Ok(plan) = parsed {
                assert_eq!(plan.horizon.as_bars(), 1);
                assert_eq!(plan.rungs as u64, rungs);
                assert_eq!(plan.step.cast_unsigned(), step);
                accepted += 1;
            } else {
                refused += 1;
            }
        }
    }
    assert!(accepted > 20 && refused > 20);
    for fields in [
        ["0", "1", "1"],
        ["-1", "1", "1"],
        ["4294967296", "1", "1"],
        ["1", "0", "1"],
        ["1", "1", "0"],
        ["1", "1", "-1"],
        ["1", "1", "9223372036854775808"],
    ] {
        assert!(Plan::parse(&fields).is_err(), "{fields:?}");
    }
    assert_eq!(
        Plan::parse(&["4294967295", "1", "1"])?.horizon.as_bars(),
        u32::MAX
    );
    assert!(Plan::parse(&[]).is_err());
    assert!(Plan::parse(&["1", "1", "1", "1"]).is_err());
    Ok(())
}

#[test]
fn pricing_refuses_each_malformed_override_and_identity_binds_every_policy_term()
-> Result<(), String> {
    use crate::expression_pricing::Plan;
    let _serial = crate::knobs::serially();
    let _clear = pricing_knobs();
    let args = ["3", "1", "100"];
    let baseline = Plan::parse(&args)?;
    for (name, original) in PRICING_KNOBS {
        for bad in ["-1", "malformed", "9223372036854775808"] {
            crate::knobs::set(name, bad);
            assert!(
                Plan::parse(&args).is_err(),
                "{name}={bad} silently fell back"
            );
        }
        crate::knobs::set(name, original);
    }
    crate::knobs::set("BRUTEX_MIN_WIN_RATE_BP", "10001");
    assert!(Plan::parse(&args).is_err());
    crate::knobs::set("BRUTEX_MIN_WIN_RATE_BP", "5000");
    crate::knobs::set("BRUTEX_PROTECTED_EXITS", "2");
    assert!(Plan::parse(&args).is_err());
    crate::knobs::set("BRUTEX_PROTECTED_EXITS", "1");
    assert_eq!(Plan::parse(&args)?.words(), baseline.words());
    let changes: [fn(&mut Plan); 14] = [
        |p| p.horizon = runner::outcome::Horizon::bars(4).unwrap_or(p.horizon),
        |p| p.rungs += 1,
        |p| p.step += 1,
        |p| p.rules.max_mae_ppm += 1,
        |p| p.rules.min_rr_bp += 1,
        |p| p.rules.min_win_rate_bp += 1,
        |p| p.rules.min_trades += 1,
        |p| p.rules.min_assurance_bp += 1,
        |p| p.rules.min_weakest_bp += 1,
        |p| p.rules.min_ret_over_dd_bp += 1,
        |p| p.rules.require_protective_exits = !p.rules.require_protective_exits,
        |p| p.rules.min_fill_headroom_bp += 1,
        |p| p.rules.min_avg_rr_bp += 1,
        |p| p.rules.top += 1,
    ];
    let cursor = Cursor::new(&[0, 369]).map_err(debug)?;
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .map_err(debug)?;
    let identity = |plan: Plan| {
        runner::expression::search_identity(
            &Run {
                mask: cursor.alphabet(),
                direction: Direction::Undirected,
                instrument: &key,
                timeframe: "generated-1min",
                params: Params::of(engine::Ladder::with_min_hits(1)).with_policy(&plan.words()),
                data_digest: [71; 32],
                commit: "generated-pricing-plan",
                feed: "synthetic",
            },
            &cursor,
        )
    };
    let mut identities = std::collections::BTreeSet::from([identity(baseline)]);
    for change in changes {
        let mut plan = baseline;
        change(&mut plan);
        assert!(
            identities.insert(identity(plan)),
            "a policy change reused the search identity"
        );
    }
    assert_eq!(identities.len(), 15);
    Ok(())
}

#[test]
fn priced_report_retains_exact_execution_mapping_and_dropped_signal_note() -> Result<(), String> {
    use crate::expression_pricing::{Plan, Prepared};
    use indicators::evaluator::{Calendar, Evaluator, Widths};
    let bars = runner::synthetic::sessions(8);
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().map_err(debug)?,
        indicators::vwap::Availability::Absent,
        indicators::pattern::Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let column = indicators::column::Column::build(&bars, &mut evaluator);
    let plan = Plan {
        horizon: runner::outcome::Horizon::bars(3).ok_or("horizon")?,
        rungs: 1,
        step: 100,
        rules: crate::Rules::BASELINE,
    };
    let (execution, projected, note) =
        crate::project_onto_execution(&bars, &column, None, true, plan.horizon)?;
    assert!(projected.sources().len() < column.sources().len());
    assert!(note.contains("DROPPED"));
    let pricing = Prepared::new(execution, projected, plan, note.clone());
    let mut args = Args::parse(&[
        "synthetic",
        "NIFTY",
        "1min",
        "2026",
        "1",
        "0",
        "1",
        "1",
        "1",
    ])?;
    args.pricing = Some(plan);
    let state = State::new(Cursor::new(&args.live).map_err(debug)?);
    let report = render(&args, &state, [72; 32], 0, Some(&pricing));
    assert!(report.contains(&note));
    assert!(report.contains("PRICED EXPRESSION RESEARCH"));
    assert!(report.contains("MIN_HITS reports signal support"));
    assert!(report.contains("Cost-excluded, unvalidated research"));
    assert!(report.contains("printed-fill amounts are not net trading profits"));
    Ok(())
}
