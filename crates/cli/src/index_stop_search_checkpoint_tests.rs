#![cfg(test)]
//! Generated grammar and durable-fault fixtures, never historical sweep runs.
#![expect(
    clippy::unwrap_used,
    reason = "finite named checkpoint regression assertions"
)]
use super::*;
use crate::search_checkpoint::tests::Scratch;

const BYTES: u64 = 1024 * 1024;
const NODES: u64 = 100;
const SCOPE: u8 = 0b1000_0100;
const DECLARATION: &[u8] = b"generated-native-stop-declaration";

fn budget() -> ReadBudget {
    ReadBudget {
        bytes: BYTES,
        records: 100,
        nodes: 1000,
    }
}
fn verification() -> Verification {
    let training = qualification::ExpectedSource {
        identity: [11; 32],
        first_day: 1,
        last_day: 10,
    };
    let later = qualification::ExpectedSource {
        identity: [12; 32],
        first_day: 11,
        last_day: 20,
    };
    Verification {
        sources: std::array::from_fn(|rung| {
            (SCOPE & (1 << rung) != 0).then_some((training, later))
        }),
        policy: crate::boolean_search_record::tests::generated_policy_with_ceilings([1_000_000; 4]),
        procedure: PopulationStatisticsProcedureV2::new(255, 49, 2).unwrap(),
        bounds: qualification::Bounds {
            candidates: 100,
            bootstrap_work: 1000,
            split_work: 1000,
            memory_bytes: BYTES,
            bytes: BYTES,
        },
        replay: qualification::ReplayBounds {
            bootstrap_work: 1000,
            split_work: 1000,
            memory_bytes: BYTES,
        },
    }
}
fn first(journal: &mut Journal) -> Frame {
    let mut value =
        Frame::declaration(DECLARATION.to_vec(), Cursor::new(&[30, 31]).unwrap(), SCOPE);
    value.publish(journal, BYTES, budget(), NODES).unwrap();
    value
}
fn next(previous: &Frame) -> Frame {
    let batch = Batch::prepare(
        previous.cursor.clone(),
        previous.work,
        previous.programs,
        Budget {
            programs: 2,
            nodes: NODES,
            bytes: BYTES,
        },
    )
    .unwrap();
    Frame::pending(previous, batch).unwrap()
}
fn restore(journal: &Journal, root: &Path, allowance: ReadBudget) -> Result<Option<Frame>, String> {
    recover(
        journal,
        DECLARATION,
        root,
        allowance.bytes,
        allowance.records,
        allowance.nodes,
        SCOPE,
        &Cursor::new(&[30, 31]).unwrap(),
        2,
        NODES,
        &verification(),
    )
}

#[test]
fn pending_restart_preserves_exact_programs_and_never_marks_partial_work_complete() {
    let scratch = Scratch::new().unwrap();
    let id = hash(DECLARATION);
    let mut journal = Journal::open(&scratch.0, NAMESPACE, id).unwrap();
    assert!(restore(&journal, &scratch.0, budget()).unwrap().is_none());
    let declaration = first(&mut journal);
    let mut pending = next(&declaration);
    pending
        .publish(&mut journal, BYTES, budget(), NODES)
        .unwrap();
    let pin = pending.location;
    let bytes = pending.encode().unwrap();
    drop(journal);
    let reopened = Journal::open(&scratch.0, NAMESPACE, id).unwrap();
    let recovered = restore(&reopened, &scratch.0, budget()).unwrap().unwrap();
    assert_eq!(recovered.encode().unwrap(), bytes);
    assert_eq!(recovered.location, pin);
    assert!(recovered.pending);
    assert_eq!(recovered.completed_batches, 0);
    assert_eq!(recovered.programs, 0);
    assert!(!recovered.exhausted);
    assert!(recovered.links.iter().all(Option::is_none));
    assert_eq!(reopened.acknowledged(), 2);
}

#[test]
fn byte_node_and_record_admission_happen_before_acknowledgment_and_can_be_raised_for_retry() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, hash(DECLARATION)).unwrap();
    let declaration = first(&mut journal);
    let mut pending = next(&declaration);
    let exact_bytes =
        declaration.retained_bytes + u64::try_from(pending.encode().unwrap().len()).unwrap() + 96;
    for refused in [
        ReadBudget {
            bytes: exact_bytes - 1,
            ..budget()
        },
        ReadBudget {
            nodes: NODES - 1,
            ..budget()
        },
        ReadBudget {
            records: 1,
            ..budget()
        },
    ] {
        assert!(
            pending
                .publish(&mut journal, BYTES, refused, NODES)
                .is_err()
        );
        assert_eq!(journal.acknowledged(), 1);
        assert!(pending.location.is_none());
        let old = restore(&journal, &scratch.0, budget()).unwrap().unwrap();
        assert_eq!(old.location, declaration.location);
    }
    let exact = ReadBudget {
        bytes: exact_bytes,
        records: 2,
        nodes: NODES,
    };
    pending.publish(&mut journal, BYTES, exact, NODES).unwrap();
    let reopened = restore(&journal, &scratch.0, exact).unwrap().unwrap();
    assert_eq!(reopened.retained_bytes, exact_bytes);
    assert_eq!(reopened.replayed_nodes, NODES);
    assert_eq!(journal.acknowledged(), 2);
    assert!(
        restore(
            &journal,
            &scratch.0,
            ReadBudget {
                bytes: exact_bytes - 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(
        restore(
            &journal,
            &scratch.0,
            ReadBudget {
                records: 1,
                ..exact
            }
        )
        .is_err()
    );
    assert!(
        restore(
            &journal,
            &scratch.0,
            ReadBudget {
                nodes: NODES - 1,
                ..exact
            }
        )
        .is_err()
    );
}

#[test]
fn resealed_foreign_alphabet_and_changed_declared_batch_limits_refuse() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, hash(DECLARATION)).unwrap();
    let declaration = first(&mut journal);
    assert!(
        recover(
            &journal,
            DECLARATION,
            &scratch.0,
            BYTES,
            100,
            1000,
            SCOPE,
            &Cursor::new(&[30]).unwrap(),
            2,
            NODES,
            &verification()
        )
        .is_err()
    );
    let mut pending = next(&declaration);
    pending
        .publish(&mut journal, BYTES, budget(), NODES)
        .unwrap();
    for (programs, nodes) in [(1, NODES), (2, NODES + 1)] {
        assert!(
            recover(
                &journal,
                DECLARATION,
                &scratch.0,
                BYTES,
                100,
                1000,
                SCOPE,
                &Cursor::new(&[30, 31]).unwrap(),
                programs,
                nodes,
                &verification()
            )
            .is_err()
        );
    }
    assert!(
        recover(
            &journal,
            b"other-declaration",
            &scratch.0,
            BYTES,
            100,
            1000,
            SCOPE,
            &Cursor::new(&[30, 31]).unwrap(),
            2,
            NODES,
            &verification()
        )
        .is_err()
    );
}

#[test]
fn every_selected_timeframe_requires_one_exact_child_and_no_unselected_child() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, hash(DECLARATION)).unwrap();
    let declaration = first(&mut journal);
    let link = Link {
        identity: [1; 32],
        pin: [2; 32],
    };
    let links = std::array::from_fn(|n| (SCOPE & (1 << n) != 0).then_some(link));
    for rung in 0..8 {
        let mut pending = next(&declaration);
        pending.location = Some((2, [7; 32]));
        let mut changed = links;
        let slot = changed.get_mut(rung).unwrap();
        *slot = if slot.is_some() { None } else { Some(link) };
        assert!(
            Frame::done(pending, &changed).is_err(),
            "physical rung {rung}"
        );
    }
    let mut pending = next(&declaration);
    pending.location = Some((2, [7; 32]));
    let complete = Frame::done(pending, &links).unwrap();
    assert_eq!(complete.completed_batches, 1);
    assert_eq!(complete.programs, 2);
    assert_eq!(complete.links, links);
    assert!(!complete.pending);
    assert!(!complete.exhausted);
    let pending = next(&declaration);
    assert!(
        Frame::done(pending, &links).is_err(),
        "unacknowledged reservation"
    );
}

#[test]
fn canonical_decoder_rejects_bad_flags_padding_truncation_and_hidden_ancestry() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, hash(DECLARATION)).unwrap();
    let declaration = first(&mut journal);
    let saved = journal.latest(BYTES).unwrap().unwrap();
    let decoded = Frame::decode(&saved, DECLARATION, BYTES, 1000, SCOPE).unwrap();
    assert_eq!(decoded.encode().unwrap(), declaration.encode().unwrap());
    for offset in [0, 8, 9, 10, 11, 12, 16, 24] {
        let mut changed = Saved {
            sequence: saved.sequence,
            payload: saved.payload.clone(),
            seal: saved.seal,
        };
        *changed.payload.get_mut(offset).unwrap() = 255;
        assert!(
            Frame::decode(&changed, DECLARATION, BYTES, 1000, SCOPE).is_err(),
            "offset {offset}"
        );
    }
    let mut changed = Saved {
        sequence: saved.sequence,
        payload: saved.payload.clone(),
        seal: saved.seal,
    };
    changed.payload.push(0);
    assert!(Frame::decode(&changed, DECLARATION, BYTES, 1000, SCOPE).is_err());
    for extent in [0, 8, 96, saved.payload.len() - 1] {
        changed.payload = saved.payload.get(..extent).unwrap().to_vec();
        assert!(Frame::decode(&changed, DECLARATION, BYTES, 1000, SCOPE).is_err());
    }
}

#[test]
fn resealed_history_forks_and_missing_selected_evidence_are_not_completed_research() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, hash(DECLARATION)).unwrap();
    let declaration = first(&mut journal);
    let mut pending = next(&declaration);
    pending
        .publish(&mut journal, BYTES, budget(), NODES)
        .unwrap();
    let link = Link {
        identity: [1; 32],
        pin: [2; 32],
    };
    let mut complete = Frame::done(
        pending,
        &std::array::from_fn(|n| (SCOPE & (1 << n) != 0).then_some(link)),
    )
    .unwrap();
    complete
        .publish(&mut journal, BYTES, budget(), NODES)
        .unwrap();
    assert!(
        restore(&journal, &scratch.0, budget()).is_err(),
        "missing child evidence must refuse"
    );
    let mut fork = Frame::declaration(DECLARATION.to_vec(), Cursor::new(&[30, 31]).unwrap(), SCOPE);
    fork.publish(&mut journal, BYTES, budget(), NODES).unwrap();
    assert!(
        restore(&journal, &scratch.0, budget()).is_err(),
        "resealed reset omits acknowledged ancestry"
    );
}
