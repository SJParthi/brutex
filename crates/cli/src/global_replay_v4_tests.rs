//! Finite scheduler/storage attacks plus a real stored-origin Runner witness.
#![allow(clippy::expect_used, clippy::indexing_slicing)]
use super::*;
use runner::portfolio::StrategyDigest;
use std::sync::atomic::{AtomicU64, Ordering};

const MINUTE: i64 = 60_000_000;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Scratch(std::path::PathBuf);
impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "brutex-replay-v4-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).expect("unique scratch");
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Stamps {
    calls: Vec<(Vendor, i64)>,
    exact: bool,
}
impl VixLookup for Stamps {
    fn stamp(&mut self, feed: Vendor, ts: i64) -> Result<VixStamp, String> {
        self.calls.push((feed, ts));
        Ok(if self.exact {
            VixStamp::Exact(indicators::Candle {
                ts_micros: ts,
                open: 10,
                high: 12,
                low: 9,
                close: 11,
                volume: 0,
                open_interest: i64::MIN,
            })
        } else {
            VixStamp::Absent
        })
    }
}
fn bounds() -> GlobalReplayV4Bounds {
    GlobalReplayV4Bounds::new(1024, 1024 * codec::STRIDE as u64).expect("finite bounds")
}
fn request() -> StoredPostTrainingOosRequestV1 {
    let bound = crate::stored::StoredSpanLoadBoundV1::new(40_000).expect("fixture load bound");
    StoredPostTrainingOosRequestV1::new((2025, 10), (2025, 10), bound, bound, bound)
        .expect("explicit OOS month")
}

#[test]
fn replay_plan_binds_every_source_slot_dates_and_all_load_and_output_ceilings() {
    let snapshots: [SelectionV6Snapshot; 8] = std::array::from_fn(|index| SelectionV6Snapshot {
        rung_seconds: [60, 120, 180, 300, 600, 900, 1800, 3600][index],
        identity: [u8::try_from(index + 1).expect("eight fixed slots"); 32],
        envelope: [u8::try_from(index + 2).expect("eight fixed envelopes"); 960],
        winners: Vec::new(),
    });
    let original =
        lifecycle::plan_identity(&snapshots, request(), bounds()).expect("plan identity");
    assert_ne!(original, [0; 32]);
    assert_eq!(
        original,
        lifecycle::plan_identity(&snapshots, request(), bounds()).expect("repeat")
    );
    for index in 0..8 {
        let mut changed = snapshots.clone();
        changed[index].identity[0] ^= 1;
        assert_ne!(
            original,
            lifecycle::plan_identity(&changed, request(), bounds()).expect("changed source")
        );
        changed = snapshots.clone();
        changed[index].envelope[959] ^= 1;
        assert_ne!(
            original,
            lifecycle::plan_identity(&changed, request(), bounds()).expect("changed envelope")
        );
    }
    let ceiling = crate::stored::StoredSpanLoadBoundV1::new(40_000).expect("base ceiling");
    let changed = crate::stored::StoredSpanLoadBoundV1::new(40_001).expect("changed ceiling");
    for revised in [
        StoredPostTrainingOosRequestV1::new((2025, 11), (2025, 11), ceiling, ceiling, ceiling),
        StoredPostTrainingOosRequestV1::new((2025, 10), (2025, 11), ceiling, ceiling, ceiling),
        StoredPostTrainingOosRequestV1::new((2025, 10), (2025, 10), changed, ceiling, ceiling),
        StoredPostTrainingOosRequestV1::new((2025, 10), (2025, 10), ceiling, changed, ceiling),
        StoredPostTrainingOosRequestV1::new((2025, 10), (2025, 10), ceiling, ceiling, changed),
    ] {
        assert_ne!(
            original,
            lifecycle::plan_identity(
                &snapshots,
                revised.expect("valid explicit request"),
                bounds()
            )
            .expect("revised request")
        );
    }
    for revised in [
        GlobalReplayV4Bounds::new(1023, 1024 * codec::STRIDE as u64),
        GlobalReplayV4Bounds::new(1024, 1024 * codec::STRIDE as u64 + 1),
    ] {
        assert_ne!(
            original,
            lifecycle::plan_identity(
                &snapshots,
                request(),
                revised.expect("valid physical ceiling")
            )
            .expect("revised bounds")
        );
    }
}

#[test]
fn durable_parent_and_exact_stage_starts_precede_work_and_refusal_is_retained() {
    use crate::stored_post_training_oos::StoredOosComputationV1 as Event;
    use crate::sweep_evidence::{self, Completion, Operation};
    let root = Scratch::new();
    let parent = [31; 32];
    let fold = [32; 32];
    let replay = [33; 32];
    let read = |id| {
        sweep_evidence::read(&root.0, id, 1_048_576)
            .expect("valid durable evidence")
            .expect("saved identity")
    };
    let failure: Result<(), String> = lifecycle::recorded(&root.0, parent, |observe| {
        assert_eq!(read(parent).completion, Completion::Running);
        observe(Event::FoldStarted(fold))?;
        assert_eq!(read(fold).operation, Operation::Preparation);
        assert_eq!(read(fold).completion, Completion::Running);
        observe(Event::FoldCompleted)?;
        assert_eq!(read(fold).completion, Completion::Completed);
        observe(Event::ReplayStarted(replay))?;
        assert_eq!(read(replay).operation, Operation::GlobalReplayStream);
        assert_eq!(read(replay).completion, Completion::Running);
        Err("injected trade computation refusal".to_owned())
    });
    assert_eq!(
        failure,
        Err("injected trade computation refusal".to_owned())
    );
    assert_eq!(read(parent).completion, Completion::Refused);
    assert_eq!(read(replay).completion, Completion::Refused);
    assert_eq!(read(fold).completion, Completion::Completed);
    let prior = read(parent).attempt;
    lifecycle::recorded(&root.0, parent, |observe| {
        observe(Event::FoldStarted(fold))?;
        observe(Event::FoldCompleted)?;
        observe(Event::ReplayStarted(replay))?;
        observe(Event::ReplayCompleted)
    })
    .expect("complete separately identified retry");
    assert!(read(parent).attempt > prior);
    assert_eq!(read(parent).completion, Completion::Completed);
    assert_eq!(read(replay).completion, Completion::Completed);
    assert_eq!(
        sweep_evidence::read_attempt(&root.0, parent, prior, 1_048_576)
            .expect("retained refusal")
            .expect("old attempt")
            .completion,
        Completion::Refused
    );
}

#[test]
fn missing_durable_start_and_invalid_stage_order_never_return_success() {
    use crate::stored_post_training_oos::StoredOosComputationV1 as Event;
    let root = Scratch::new();
    std::fs::write(root.0.join("blocked"), b"regular file").expect("blocked output path");
    let mut called = false;
    let denied = lifecycle::recorded(&root.0.join("blocked"), [41; 32], |_| {
        called = true;
        Ok(())
    });
    assert!(denied.is_err());
    assert!(!called, "missing durable parent start refuses before work");
    for events in [
        vec![Event::ReplayCompleted],
        vec![Event::FoldStarted([42; 32]), Event::ReplayCompleted],
        vec![Event::ReplayStarted([43; 32]), Event::FoldStarted([44; 32])],
        vec![Event::FoldStarted([45; 32])],
    ] {
        let result = lifecycle::recorded(&root.0, [46; 32], |observe| {
            for event in events {
                observe(event)?;
            }
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(
            crate::sweep_evidence::read(&root.0, [46; 32], 1_048_576)
                .expect("valid parent")
                .expect("saved parent")
                .completion,
            crate::sweep_evidence::Completion::Refused
        );
    }
}

#[test]
fn real_stored_witness_observer_refuses_before_each_computation_and_binds_actual_run() {
    use crate::stored_post_training_oos::StoredOosComputationV1 as Event;
    for fail_at in [0, 2, usize::MAX] {
        let mut events = Vec::new();
        let mut witness_returned = false;
        let result = crate::step3_orchestrator::with_stored_oos_replay_observer_fixture(
            &mut |event| {
                events.push(event);
                if events.len() - 1 == fail_at {
                    Err("injected durable callback refusal".to_owned())
                } else {
                    Ok(())
                }
            },
            |witness| {
                witness_returned = true;
                let (_, _, witness) = witness.into_parts();
                Ok(witness.run_id().bytes())
            },
        );
        if fail_at == usize::MAX {
            let identity = result.expect("real stored witness completed");
            assert!(witness_returned);
            assert_eq!(events.len(), 4);
            assert!(matches!(events[0], Event::FoldStarted(id) if id != [0;32]));
            assert_eq!(events[1], Event::FoldCompleted);
            assert_eq!(events[2], Event::ReplayStarted(identity));
            assert_eq!(events[3], Event::ReplayCompleted);
        } else {
            assert_eq!(result, Err("injected durable callback refusal".to_owned()));
            assert!(!witness_returned);
            assert_eq!(
                events.len(),
                fail_at + 1,
                "no later computation boundary reached after refusal"
            );
        }
    }
}

// Private scheduling facts only; the production constructor never accepts these.
fn attempt(
    id: u8,
    minute: i64,
    through: i64,
    rank: u16,
    family: &str,
    rung: u16,
    priced: bool,
) -> Attempt {
    let entry = usize::try_from(minute).expect("entry bar");
    let exit = usize::try_from(through).expect("exit bar");
    let row = runner::grid::TradeRow {
        signal_bar: entry - 1,
        entry_bar: entry,
        exit_bar: exit,
        best: 11,
        worst: 7,
        entry_micros: minute * MINUTE,
        exit_micros: through * MINUTE,
        adverse: 0,
        adverse_paisa: 0,
        favourable: 10,
        favourable_paisa: 1,
    };
    Attempt {
        stream: u64::from(id),
        ordinal: 0,
        constituent: Constituent {
            priority: rank,
            strategy_digest: StrategyDigest::new([id; 32]),
            instrument: brutex_core::instrument::InstrumentKey::index(
                brutex_core::instrument::Exchange::Nse,
                family,
            )
            .expect("fixture instrument"),
            direction: if id.is_multiple_of(2) {
                costs::fill::Direction::Short
            } else {
                costs::fill::Direction::Long
            },
            rung_minutes: rung,
        },
        feed: Vendor::Zerodha,
        candidate: CandidateProjection {
            signal_bar: entry - 1,
            entry_bar: entry,
            occupied_through_bar: exit,
            signal_micros: (minute - 1) * MINUTE,
            entry_micros: minute * MINUTE,
            occupied_through_micros: through * MINUTE,
            path: if priced {
                PathProjection::Priceable(PriceProjection {
                    row,
                    ambiguous_bars: 1,
                    gap_fills: 0,
                })
            } else {
                PathProjection::BlockOnly
            },
        },
    }
}

#[test]
fn global_occupancy_crosses_family_direction_and_rung_and_retains_unpriced_hold() {
    let attempts = [
        attempt(1, 1, 3, 1, "NIFTY", 1, false),
        attempt(2, 1, 1, 2, "BANKNIFTY", 60, true),
        attempt(3, 3, 3, 1, "BANKNIFTY", 5, true),
        attempt(4, 4, 4, 3, "NIFTY", 15, true),
    ];
    let mut rows = Vec::new();
    let mut vix = Stamps {
        calls: Vec::new(),
        exact: false,
    };
    let audit = schedule(&attempts, &mut rows, bounds(), &mut vix).expect("chronological schedule");
    assert_eq!(audit.counters.offered, 4);
    assert_eq!(audit.counters.admitted, 2);
    assert_eq!(audit.counters.blocked_simultaneous, 1);
    assert_eq!(audit.counters.blocked_occupied, 1);
    assert_eq!(audit.admitted_pricing_refused, 1);
    assert_eq!(audit.money_rows, 1);
    assert_eq!(audit.pessimistic_paisa, 7);
    assert_eq!(audit.optimistic_paisa, 11);
    assert_eq!(vix.calls, vec![(Vendor::Zerodha, 4 * MINUTE); 2]);
    assert_eq!(rows.iter().filter(|row| codec::kind(row) == 4).count(), 4);
    assert_eq!(rows.iter().filter(|row| codec::kind(row) == 5).count(), 1);
}

#[test]
fn scheduler_ties_are_order_invariant_duplicate_keys_and_overwide_minutes_refuse() {
    let first = attempt(1, 1, 1, 1, "NIFTY", 1, false);
    let second = attempt(2, 1, 1, 1, "BANKNIFTY", 60, false);
    let schedule_rows = |rows: &[Attempt]| {
        let mut out = Vec::new();
        let mut vix = Stamps {
            calls: Vec::new(),
            exact: false,
        };
        let audit = schedule(rows, &mut out, bounds(), &mut vix)?;
        Ok::<_, String>((audit, out))
    };
    assert_eq!(
        schedule_rows(&[first, second]).expect("first order"),
        schedule_rows(&[second, first]).expect("reverse order")
    );
    assert!(schedule_rows(&[first, first]).is_err());
    let oversized: Vec<_> = (0..201).map(|_| first).collect();
    assert!(schedule_rows(&oversized).is_err());
    let (empty, rows) = schedule_rows(&[]).expect("valid zero streams");
    assert_eq!(empty.counters.offered, 0);
    assert!(rows.is_empty());
}

#[test]
fn reference_changes_publication_bytes_but_never_economic_decisions() {
    let input = [attempt(1, 1, 1, 1, "NIFTY", 1, true)];
    let mut absent = Vec::new();
    let mut exact = Vec::new();
    let a = schedule(
        &input,
        &mut absent,
        bounds(),
        &mut Stamps {
            calls: Vec::new(),
            exact: false,
        },
    )
    .expect("absent reference");
    let b = schedule(
        &input,
        &mut exact,
        bounds(),
        &mut Stamps {
            calls: Vec::new(),
            exact: true,
        },
    )
    .expect("exact reference");
    assert_eq!(a, b);
    assert_ne!(absent, exact);
    let economics = |rows: &[Record]| {
        codec::digest_records(
            b"test-economic",
            rows.iter().filter(|row| codec::kind(row) != 5),
        )
    };
    assert_eq!(economics(&absent), economics(&exact));
}

fn empty_records() -> Vec<Record> {
    let mut rows = vec![codec::header(request()).expect("header")];
    for index in 0..8 {
        rows.push(Writer::new(1, index).finish().expect("structural roster"));
    }
    let audit = schedule(
        &[],
        &mut rows,
        bounds(),
        &mut Stamps {
            calls: Vec::new(),
            exact: false,
        },
    )
    .expect("empty schedule");
    rows.push(codec::completion(&audit, 9).expect("completion"));
    rows
}
fn flatten(rows: &[Record]) -> Vec<u8> {
    rows.iter().flat_map(|row| row.iter().copied()).collect()
}

#[test]
fn fixed_v4_records_authenticate_every_byte_and_reject_old_formats() {
    let original = codec::header(request()).expect("header");
    for index in 0..codec::STRIDE {
        let mut attacked = original;
        attacked[index] ^= 1;
        assert!(codec::verify_record(&attacked).is_err(), "byte {index}");
    }
    let mut out = Writer::new(0, 0);
    assert!(out.bytes(&[1; 1024]).is_err());
    assert!(GlobalReplayV4Bounds::new(9, 1_000_000).is_err());
    assert!(GlobalReplayV4Bounds::new(u64::MAX, u64::MAX).is_err());
}

#[test]
fn publication_reopens_reuses_and_refuses_validly_sealed_foreign_bytes() {
    let root = Scratch::new();
    let rows = empty_records();
    let (path, written, digest) =
        storage::persist(&root.0, bounds(), &rows, [1; 32]).expect("publish");
    assert!(written);
    assert!(
        !storage::persist(&root.0, bounds(), &rows, [1; 32])
            .expect("reuse")
            .1
    );
    storage::verify(&path, bounds(), 10, digest).expect("fresh reopen");
    let bytes = flatten(&rows);
    let mut foreign = rows;
    foreign[1] = Writer::new(1, 99).finish().expect("sealed foreign roster");
    std::fs::write(&path, flatten(&foreign)).expect("replace fixture");
    assert!(storage::verify(&path, bounds(), 10, digest).is_err());
    assert!(storage::persist(&root.0, bounds(), &empty_records(), [1; 32]).is_err());
    assert_eq!(
        std::fs::read(&path).expect("refused bytes remain"),
        flatten(&foreign)
    );
    std::fs::write(&path, bytes).expect("restore fixture");
    std::fs::remove_file(&path).expect("remove fixture");
    assert!(storage::verify(&path, bounds(), 10, digest).is_err());
}

#[test]
fn interrupted_publication_resumes_only_exact_prefix_and_never_accepts_missing_footer() {
    let rows = empty_records();
    let bytes = flatten(&rows);
    for length in [1, 24, 1023, 1024, 4096, bytes.len() - 33, bytes.len() - 1] {
        let root = Scratch::new();
        let (path, _, digest) =
            storage::persist(&root.0, bounds(), &rows, [2; 32]).expect("control");
        std::fs::write(&path, &bytes[..length]).expect("torn fixture");
        assert!(storage::verify(&path, bounds(), 10, digest).is_err());
        assert!(
            storage::persist(&root.0, bounds(), &rows, [2; 32])
                .expect("exact append-only resume")
                .1
        );
        assert_eq!(std::fs::read(path).expect("complete"), bytes);
    }
}

#[test]
fn genuine_stored_oos_witness_reaches_private_projection_and_chronological_scheduler() {
    crate::step3_orchestrator::with_stored_oos_replay_fixture(|held| {
        let (cohort, stored_witness, replay) = held.into_parts();
        assert_ne!(cohort, [0; 32]);
        assert_ne!(stored_witness, [0; 32]);
        replay.require_integrity().map_err(|why| why.to_string())?;
        assert!(
            !replay.candidates().is_empty(),
            "fixture must exercise actual replay entries"
        );
        let mut attempts = Vec::new();
        let mut previous = None;
        for (ordinal, candidate) in replay.candidates().iter().copied().enumerate() {
            check_candidate(&candidate, replay.first_oos(), previous)?;
            previous = Some((candidate.entry_bar(), candidate.entry_micros()));
            attempts.push(Attempt {
                stream: 0,
                ordinal: codec::count(ordinal)?,
                constituent: Constituent {
                    priority: 1,
                    strategy_digest: StrategyDigest::new([3; 32]),
                    instrument: replay.instrument(),
                    direction: replay.direction(),
                    rung_minutes: 1,
                },
                feed: replay.feed(),
                candidate: CandidateProjection::from_runner(candidate),
            });
        }
        let mut rows = Vec::new();
        let capacity = GlobalReplayV4Bounds::new(1_000_000, 1_000_000 * codec::STRIDE as u64)?;
        let audit = schedule(
            &attempts,
            &mut rows,
            capacity,
            &mut Stamps {
                calls: Vec::new(),
                exact: false,
            },
        )?;
        assert_eq!(
            audit.counters.offered,
            codec::count(replay.candidates().len())?
        );
        assert!(audit.counters.admitted > 0);
        assert!(audit.counters.reconciles());
        assert_eq!(
            audit.counters.admitted,
            audit.money_rows + audit.admitted_pricing_refused
        );
        Ok(())
    })
    .expect("real stored-origin OOS capability");
}
