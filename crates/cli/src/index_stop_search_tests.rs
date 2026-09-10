//! Complete generated stored-data runs, never operator historical sweeps.
use super::*;
use crate::audited_range_command::StrictConfig;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;

fn config(fixture: &Fixture) -> Result<Configuration, String> {
    let capture = crate::index_stop::tests::limits();
    Ok(Configuration {
        strict: StrictConfig::from_values(
            Some(fixture.root.as_os_str().to_owned()),
            Some(capture.bytes.to_string().into()),
            Some(capture.records.to_string().into()),
        )
        .map_err(display)?,
        policy: crate::boolean_search_record::tests::generated_policy_with_ceilings([1_000_000; 4]),
        // Two successive allocations must both be resolvable by this fixed
        // procedure. A small draw budget must not relax the second batch cap.
        procedure: PopulationStatisticsProcedureV2::new(255, 49, 2)?,
        capture,
        qualification: qualification::Bounds {
            candidates: 8,
            bootstrap_work: 10_000_000,
            split_work: 10_000_000,
            memory_bytes: capture.bytes,
            bytes: capture.bytes * 4,
        },
        workers: 2,
        history_bytes: capture.bytes * 4,
        source_observation_bytes: capture.bytes * 4,
        replay_nodes: 10_000_000,
    })
}
fn request<'a>(
    fixture: &'a Fixture,
    configuration: &'a Configuration,
) -> Result<Request<'a>, String> {
    Ok(Request {
        root: &fixture.root,
        feed: "zerodha",
        index: "NSE-NIFTY",
        training: ((2025, 5), (2025, 5)),
        later: ((2025, 6), (2025, 6)),
        rungs: RungScope::new(&["1min"])?,
        alphabet: &[0, 30],
        batch_programs: 2,
        node_allowance: 100,
        batch_allowance: 1,
        configuration,
    })
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one generated end-to-end sequence proves frozen ancestry, rejected replay, exact resume and pending restoration together"
)]
fn complete_native_search_reopens_all_ancestors_and_resumes_without_duplicating_the_batch()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    let configuration = config(&fixture)?;
    let request = request(&fixture, &configuration)?;
    let mut observed = Vec::new();
    let report = execute_with(
        &request,
        &mut |progress| {
            observed.push(progress);
            Ok(())
        },
        crate::index_stop::tests::load_generated,
    )?;
    assert!(report.contains("Search exhausted: false"));
    let first = observed.last().ok_or("complete progress missing")?;
    assert_eq!(first.completed_batches, 1);
    assert!(!first.exhausted);
    assert_eq!(first.qualifications.iter().flatten().count(), 1);
    let (id, pin) = first.qualifications[0].ok_or("selected timeframe receipt missing")?;
    let reader = qualification::Reader::open(
        &fixture.root,
        id,
        configuration.qualification.bytes,
        configuration.capture.records,
    )?;
    assert_eq!(reader.completion_digest(), pin);
    assert_eq!(reader.search_identity(), first.identity);
    assert_eq!(reader.rows().len(), 4);
    assert_eq!(reader.training().records().len(), 4);
    assert_eq!(reader.later().records().len(), 4);
    for pair in reader.training().records().chunks_exact(2) {
        let [long, short] = pair else {
            return Err("complete program direction pair missing".into());
        };
        assert_eq!(long.program().encode(), short.program().encode());
        assert_eq!(long.direction(), runner::identity::Direction::Long);
        assert_eq!(short.direction(), runner::identity::Direction::Short);
    }
    for (index, _) in reader.rows().iter().enumerate() {
        let daily = reader.consistency(pin, index)?;
        assert!(daily.training.first_day < daily.later.first_day);
        assert!(daily.training.last_day < daily.later.first_day);
        assert_eq!(daily.evaluation.first_day, daily.training.first_day);
        assert_eq!(daily.evaluation.last_day, daily.later.last_day);
    }
    let first_identity = first.identity;
    // An independently smaller observation budget keeps the same immutable
    // declaration but must refuse numerical replay before announcing a resume.
    let mut limited = config(&fixture)?;
    limited.replay_nodes = 1_000;
    let limited_request = Request {
        configuration: &limited,
        ..request
    };
    let before = {
        let journal = Journal::open(&fixture.root, NAMESPACE, first_identity)?;
        (
            journal.acknowledged(),
            journal
                .latest(configuration.history_bytes)?
                .ok_or("saved checkpoint missing")?
                .seal,
        )
    };
    let mut announced = false;
    let refusal = execute_with(
        &limited_request,
        &mut |_| {
            announced = true;
            Ok(())
        },
        crate::index_stop::tests::load_generated,
    )
    .err()
    .ok_or("current numerical replay admission must refuse")?;
    assert!(
        refusal.contains("CSCV") || refusal.contains("bootstrap work"),
        "{refusal}"
    );
    assert!(!announced);
    {
        let journal = Journal::open(&fixture.root, NAMESPACE, first_identity)?;
        assert_eq!(journal.acknowledged(), before.0);
        assert_eq!(
            journal
                .latest(configuration.history_bytes)?
                .ok_or("saved checkpoint removed")?
                .seal,
            before.1
        );
    }
    let mut resumed = Vec::new();
    execute_with(
        &request,
        &mut |progress| {
            resumed.push(progress);
            Ok(())
        },
        crate::index_stop::tests::load_generated,
    )?;
    let next = resumed.last().ok_or("resumed progress missing")?;
    assert_eq!(next.identity, first_identity);
    assert_eq!(next.completed_batches, 2);
    assert_ne!(next.qualifications[0], Some((id, pin)));
    let pending = resumed
        .iter()
        .find(|state| state.current_batch == Some(1))
        .ok_or("second batch reservation was not observed")?;
    let retained = pending
        .latest_saved
        .as_ref()
        .ok_or("prior saved table disappeared while running")?;
    assert_eq!(retained.batch, 0);
    assert_eq!(retained.qualifications[0], Some((id, pin)));
    reader.require_current()?;
    let (next_id, next_pin) = next.qualifications[0].ok_or("second timeframe receipt missing")?;
    let later = qualification::Reader::open(
        &fixture.root,
        next_id,
        configuration.qualification.bytes,
        configuration.capture.records,
    )?;
    assert_eq!(later.completion_digest(), next_pin);
    assert_ne!(later.allocation(), reader.allocation());
    // Simulate loss of the UI observer immediately after the next durable
    // reservation, then open it in a fresh invocation without pricing again.
    let pause = execute_with(
        &request,
        &mut |state| {
            if state.current_batch == Some(2) {
                Err("generated observer pause".into())
            } else {
                Ok(())
            }
        },
        crate::index_stop::tests::load_generated,
    );
    assert!(pause.is_err());
    let mut restored = None;
    assert!(
        execute_with(
            &request,
            &mut |state| {
                restored = Some(state);
                Err("generated inspection stops before pricing".into())
            },
            crate::index_stop::tests::load_generated
        )
        .is_err()
    );
    let restored = restored.ok_or("pending state not restored")?;
    assert_eq!(restored.completed_batches, 2);
    assert_eq!(restored.current_batch, Some(2));
    let saved = restored
        .latest_saved
        .ok_or("fresh invocation lost acknowledged tables")?;
    assert_eq!(saved.batch, 1);
    assert_eq!(saved.qualifications[0], Some((next_id, next_pin)));
    Ok(())
}

#[test]
fn source_and_observer_refusals_never_create_a_completed_search() -> Result<(), String> {
    let fixture = Fixture::new()?;
    let configuration = config(&fixture)?;
    let request = request(&fixture, &configuration)?;
    let mut calls = 0;
    assert!(
        execute_with(
            &request,
            &mut |_| {
                calls += 1;
                Ok(())
            },
            crate::index_stop::tests::load_generated
        )
        .is_err()
    );
    assert_eq!(calls, 0);
    assert!(!fixture.root.join(NAMESPACE).exists());
    fixture.prepare("NIFTY")?;
    let mut captured = None;
    let why = execute_with(
        &request,
        &mut |state| {
            captured = Some(state);
            Err("generated observer loss".into())
        },
        crate::index_stop::tests::load_generated,
    )
    .err()
    .ok_or("observer refusal hidden")?;
    assert!(why.contains("generated observer loss"));
    let state = captured.ok_or("initial declaration not observed")?;
    assert_eq!(state.completed_batches, 0);
    assert!(!state.exhausted);
    assert!(state.qualifications.iter().all(Option::is_none));
    let journal = Journal::open(&fixture.root, NAMESPACE, state.identity)?;
    assert_eq!(journal.acknowledged(), 1);
    Ok(())
}

#[test]
fn launch_scope_refuses_overlap_future_boundaries_and_non_index_instruments_before_loading()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let configuration = config(&fixture)?;
    let mut input = request(&fixture, &configuration)?;
    for index in [
        "NIFTY",
        "NSE-RELIANCE",
        "BSE-SENSEX",
        "NSE-INDIAVIX",
        "NSE-NIFTY,NSE-BANKNIFTY",
    ] {
        input.index = index;
        assert!(validate(&input).is_err());
    }
    input.index = "NSE-NIFTY";
    input.later = ((2025, 5), (2025, 6));
    assert!(validate(&input).is_err());
    input.later = ((2025, 6), (2025, 6));
    input.training.0 = (2025, 13);
    assert!(validate(&input).is_err());
    input.training.0 = (2025, 5);
    for (programs, nodes, batches) in [(0, 100, 1), (5, 100, 1), (2, 0, 1), (2, 100, 0)] {
        input.batch_programs = programs;
        input.node_allowance = nodes;
        input.batch_allowance = batches;
        assert!(validate(&input).is_err());
    }
    assert!(!fixture.root.join(NAMESPACE).exists());
    Ok(())
}
