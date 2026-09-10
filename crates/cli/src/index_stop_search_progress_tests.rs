//! Bounded generated-store boundary tests; no operator market history is read.
use super::super::{Configuration, execute_observed_with};
use super::*;
use crate::candidate_universe::boolean_candidate_v1::tests::Fixture;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn configuration(fixture: &Fixture) -> Result<Configuration, String> {
    let capture = crate::index_stop::tests::limits();
    Ok(Configuration {
        strict: crate::audited_range_command::StrictConfig::from_values(
            Some(fixture.root.as_os_str().to_owned()),
            Some(capture.bytes.to_string().into()),
            Some(capture.records.to_string().into()),
        )
        .map_err(display)?,
        policy: crate::boolean_search_record::tests::generated_policy_with_ceilings([1_000_000; 4]),
        procedure: crate::population_statistics_v2::PopulationStatisticsProcedureV2::new(
            31, 49, 2,
        )?,
        capture,
        qualification: crate::index_stop_qualification::Bounds {
            candidates: 8,
            bootstrap_work: 10_000_000,
            split_work: 10_000_000,
            memory_bytes: capture.bytes,
            bytes: capture.bytes * 4,
        },
        workers: 2,
        history_bytes: capture.bytes * 4,
        source_observation_bytes: capture.bytes * 4,
        replay_nodes: 10_000,
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
        rungs: crate::boolean_campaign::RungScope::new(&["1min"])?,
        alphabet: &[0, 30],
        batch_programs: 2,
        node_allowance: 100,
        batch_allowance: 1,
        configuration,
    })
}
fn forbidden_load(_: crate::index_stop::Request<'_>) -> Result<crate::index_stop::Loaded, String> {
    Err("loader was reached after its observer refused".into())
}

#[test]
fn preparing_has_no_invented_identity_and_observer_refusal_prevents_source_reads()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    let configuration = configuration(&fixture)?;
    let request = request(&fixture, &configuration)?;
    let called = Rc::new(Cell::new(false));
    let observed = Rc::clone(&called);
    let why = execute_observed_with(
        &request,
        &mut |event| {
            assert_eq!(
                event,
                Observation::Preparing {
                    rung: 0,
                    stage: RungStage::Preparing
                }
            );
            observed.set(true);
            Err("private observer refusal before source".into())
        },
        forbidden_load,
    )
    .err()
    .ok_or("source boundary refusal hidden")?;
    assert!(called.get());
    assert_eq!(why, "private observer refusal before source");
    assert!(!fixture.root.join(super::super::NAMESPACE).exists());
    Ok(())
}

#[test]
fn non_send_observer_failure_joins_workers_and_never_acknowledges_the_pending_batch()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    let configuration = configuration(&fixture)?;
    let request = request(&fixture, &configuration)?;
    let seen = Rc::new(RefCell::new(None));
    let observed = Rc::clone(&seen);
    let why = execute_observed_with(
        &request,
        &mut |event| {
            if let Observation::Search(progress) = event
                && progress.rung_stages.contains(&Some(RungStage::Training))
            {
                *observed.borrow_mut() = Some(progress);
                return Err("private observer lost while training".into());
            }
            Ok(())
        },
        crate::index_stop::tests::load_generated,
    )
    .err()
    .ok_or("lost observer reported completion")?;
    assert_eq!(why, "private observer lost while training");
    let borrowed = seen.borrow();
    let progress = borrowed
        .as_ref()
        .ok_or("training boundary never observed")?;
    assert_eq!(progress.completed_batches, 0);
    assert_eq!(progress.completed_programs, 0);
    assert_eq!(progress.completed_work, 0);
    assert_eq!(progress.current_batch, Some(0));
    assert!(!progress.exhausted);
    assert!(progress.qualifications.iter().all(Option::is_none));
    let journal = crate::search_checkpoint::Journal::open(
        &fixture.root,
        super::super::NAMESPACE,
        progress.identity,
    )?;
    assert_eq!(journal.acknowledged(), 2);
    assert!(
        !fixture
            .root
            .join(crate::index_stop_qualification::NAMESPACE)
            .exists()
    );
    Ok(())
}

#[test]
fn cancelled_boundary_cannot_enqueue_or_begin_another_stage() -> Result<(), String> {
    let (sender, receiver) = sync_channel(1);
    let cancelled = AtomicBool::new(true);
    let boundary = Boundary {
        sender: &sender,
        cancelled: &cancelled,
        rung: 0,
    };
    assert!(boundary.send(RungStage::Training).is_err());
    assert!(matches!(
        receiver.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    cancelled.store(false, Ordering::Release);
    boundary.send(RungStage::Training)?;
    assert_eq!(receiver.recv().map_err(display)?, (0, RungStage::Training));
    drop(receiver);
    assert!(boundary.send(RungStage::Later).is_err());
    Ok(())
}

#[test]
fn insufficient_fixed_draw_resolution_refuses_before_training_or_candidate_publication()
-> Result<(), String> {
    let fixture = Fixture::new()?;
    fixture.prepare("NIFTY")?;
    let mut configuration = configuration(&fixture)?;
    configuration.procedure =
        crate::population_statistics_v2::PopulationStatisticsProcedureV2::new(1, 49, 2)?;
    let request = request(&fixture, &configuration)?;
    let mut observed = Vec::new();
    let why = execute_observed_with(
        &request,
        &mut |event| {
            observed.push(event);
            Ok(())
        },
        crate::index_stop::tests::load_generated,
    )
    .err()
    .ok_or("unresolvable draws were accepted")?;
    assert!(why.contains("batch 0, timeframe 1min"));
    assert!(why.contains("configured bootstrap draws 1"));
    assert!(why.contains("required minimum 15"));
    assert!(why.contains("no candidate pricing started"));
    let stages: Vec<_> = observed
        .iter()
        .filter_map(|event| match event {
            Observation::Search(progress) => Some(progress),
            Observation::Preparing { .. } => None,
        })
        .collect();
    assert!(
        stages
            .iter()
            .any(|progress| progress.rung_stages.contains(&Some(RungStage::Refused)))
    );
    assert!(
        stages
            .iter()
            .all(|progress| !progress.rung_stages.contains(&Some(RungStage::Training)))
    );
    assert!(
        stages
            .iter()
            .all(|progress| progress.completed_batches == 0 && !progress.exhausted)
    );
    assert!(
        !fixture
            .root
            .join(crate::index_stop_store::NAMESPACE)
            .exists()
    );
    assert!(
        !fixture
            .root
            .join(crate::index_stop_qualification::NAMESPACE)
            .exists()
    );
    configuration.procedure =
        crate::population_statistics_v2::PopulationStatisticsProcedureV2::new(15, 49, 2)?;
    assert!(super::super::pricing_allocation(&configuration, 0, 0).is_ok());
    assert!(super::super::pricing_allocation(&configuration, 1, 0).is_err());
    Ok(())
}
