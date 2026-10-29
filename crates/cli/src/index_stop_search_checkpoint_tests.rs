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

// D-7410: recovery walks the acknowledged ancestry in a loop and every frame
// holds its cursor on the heap, so the walk fits a stack far below the 2 MiB
// harness default at any ancestry depth, in every build. Measured peak of
// these tests in the instrumented opt-level-0 build: over 176 KiB, at most
// 192 KiB (L-STACK1028-02); the bound is twice that.
const CHECKPOINT_STACK: usize = 384 << 10;
const DEEP: u64 = 60;
const FULL: u64 = 64;
const CHAIN_NODES: u64 = 100_000;

fn deep_budget() -> ReadBudget {
    ReadBudget {
        bytes: 64 * BYTES,
        records: 1000,
        nodes: 1000 * CHAIN_NODES,
    }
}
fn fake_links() -> [Option<Link>; 8] {
    let link = Link {
        identity: [1; 32],
        pin: [2; 32],
    };
    std::array::from_fn(|n| (SCOPE & (1 << n) != 0).then_some(link))
}
/// The declaration plus `batches` reserved-then-completed batches of
/// `programs` each, every completion carrying generated (unverifiable) links.
fn chain(journal: &mut Journal, batches: u64, programs: u64) -> Frame {
    let mut frame = first(journal);
    for _ in 0..batches {
        let batch = Batch::prepare(
            frame.cursor.clone(),
            frame.work,
            frame.programs,
            Budget {
                programs,
                nodes: CHAIN_NODES,
                bytes: BYTES,
            },
        )
        .unwrap();
        let mut pending = Frame::pending(&frame, batch).unwrap();
        pending
            .publish(journal, BYTES, deep_budget(), CHAIN_NODES)
            .unwrap();
        frame = Frame::done(pending, &fake_links()).unwrap();
        frame
            .publish(journal, BYTES, deep_budget(), CHAIN_NODES)
            .unwrap();
    }
    frame
}
fn restore_full(
    journal: &Journal,
    root: &Path,
    allowance: ReadBudget,
    programs: u64,
) -> Result<Option<Frame>, String> {
    recover(
        journal,
        DECLARATION,
        root,
        allowance.bytes,
        allowance.records,
        allowance.nodes,
        SCOPE,
        &Cursor::new(&[30, 31]).unwrap(),
        programs,
        CHAIN_NODES,
        &verification(),
    )
}
fn bounded<T: Send>(name: &str, body: impl FnOnce() -> T + Send) -> T {
    crate::boolean_grammar_batch::on_bounded_stack(name, CHECKPOINT_STACK, body).unwrap()
}

#[test]
fn a_checkpoint_frame_holds_its_cursor_off_the_stack() {
    let start = Cursor::new(&[30, 31]).unwrap();
    let cursor = std::mem::size_of_val(&start);
    let frame = Frame::declaration(DECLARATION.to_vec(), start, SCOPE);
    let inline = std::mem::size_of_val(&frame);
    assert!(inline <= 2048, "Frame is {inline} bytes inline");
    assert!(inline < cursor / 4, "cursor {cursor} bytes");
    // A boxed and an owned cursor declare the same frame.
    let boxed = Frame::declaration(
        DECLARATION.to_vec(),
        Box::new(Cursor::new(&[30, 31]).unwrap()),
        SCOPE,
    );
    assert_eq!(boxed.encode().unwrap(), frame.encode().unwrap());
}

#[test]
fn ancestries_of_one_and_two_frames_recover_exactly_on_a_bounded_stack() {
    bounded("checkpoint-shallow", || {
        let scratch = Scratch::new().unwrap();
        let id = hash(DECLARATION);
        let mut journal = Journal::open(&scratch.0, NAMESPACE, id).unwrap();
        let declaration = first(&mut journal);
        let one = restore(&journal, &scratch.0, budget()).unwrap().unwrap();
        assert_eq!(one.encode().unwrap(), declaration.encode().unwrap());
        assert_eq!(one.location, declaration.location);
        assert!(one.batch.is_none() && !one.pending);
        let mut pending = next(&declaration);
        pending
            .publish(&mut journal, BYTES, budget(), NODES)
            .unwrap();
        let two = restore(&journal, &scratch.0, budget()).unwrap().unwrap();
        assert_eq!(two.encode().unwrap(), pending.encode().unwrap());
        assert!(two.pending);
        assert_eq!(two.previous, declaration.location);
        assert_eq!(journal.acknowledged(), 2);
    });
}

#[test]
fn the_deepest_ancestry_is_walked_in_a_loop_and_refused_at_its_oldest_unverified_child() {
    let refusals = bounded("checkpoint-deepest", || {
        let mut refusals = Vec::new();
        for depth in [1, DEEP] {
            let scratch = Scratch::new().unwrap();
            let mut journal = Journal::open(&scratch.0, NAMESPACE, hash(DECLARATION)).unwrap();
            let latest = chain(&mut journal, depth, 2);
            assert_eq!(latest.completed_batches, depth);
            let why = restore_full(&journal, &scratch.0, deep_budget(), 2)
                .err()
                .unwrap();
            // Nothing is rewritten by a refused recovery.
            assert_eq!(journal.acknowledged(), 2 * depth + 1);
            assert_eq!(
                journal.latest(64 * BYTES).unwrap().unwrap().seal,
                latest.location.unwrap().1
            );
            refusals.push(why);
        }
        refusals
    });
    // The deepest chain reaches the same evidence refusal as a one-batch
    // chain: every ancestor decoded, the first completed child refused.
    assert_eq!(refusals.len(), 2);
    let (shallow, deepest) = (refusals.first().unwrap(), refusals.last().unwrap());
    assert_eq!(shallow, deepest);
    assert!(!shallow.contains("admission"), "{shallow}");
}

#[test]
fn a_deep_ancestry_of_full_batches_at_exact_record_and_byte_admission_fits_a_bounded_stack() {
    // Combination of extremes: the deepest chain, every batch full, and the
    // read admissions at exactly the chain's own size and one below it.
    bounded("checkpoint-deep-full", || {
        let scratch = Scratch::new().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, hash(DECLARATION)).unwrap();
        let latest = chain(&mut journal, DEEP, FULL);
        assert_eq!(latest.programs, DEEP * FULL);
        let frames = journal.acknowledged();
        assert_eq!(frames, 2 * DEEP + 1);
        let evidence = restore_full(&journal, &scratch.0, deep_budget(), FULL)
            .err()
            .unwrap();
        assert!(!evidence.contains("admission"), "{evidence}");
        let exact = ReadBudget {
            records: frames,
            ..deep_budget()
        };
        assert_eq!(
            restore_full(&journal, &scratch.0, exact, FULL)
                .err()
                .unwrap(),
            evidence
        );
        let short = ReadBudget {
            records: frames - 1,
            ..deep_budget()
        };
        assert!(
            restore_full(&journal, &scratch.0, short, FULL)
                .err()
                .unwrap()
                .contains("cold record admission")
        );
        let tight = ReadBudget {
            bytes: latest.retained_bytes - 1,
            ..deep_budget()
        };
        assert!(
            restore_full(&journal, &scratch.0, tight, FULL)
                .err()
                .unwrap()
                .contains("admission")
        );
        let starved = ReadBudget {
            nodes: 2 * DEEP * CHAIN_NODES - 1,
            ..deep_budget()
        };
        assert!(
            restore_full(&journal, &scratch.0, starved, FULL)
                .err()
                .unwrap()
                .contains("admission")
        );
    });
}

#[test]
fn a_pending_reservation_cut_and_reopened_twice_recovers_the_same_bytes_on_a_bounded_stack() {
    bounded("checkpoint-cut-twice", || {
        let scratch = Scratch::new().unwrap();
        let id = hash(DECLARATION);
        let mut journal = Journal::open(&scratch.0, NAMESPACE, id).unwrap();
        let declaration = first(&mut journal);
        let mut pending = next(&declaration);
        pending
            .publish(&mut journal, BYTES, budget(), NODES)
            .unwrap();
        drop(journal);
        let mut seen = Vec::new();
        for _ in 0..2 {
            let reopened = Journal::open(&scratch.0, NAMESPACE, id).unwrap();
            let recovered = restore(&reopened, &scratch.0, budget()).unwrap().unwrap();
            assert!(recovered.pending);
            assert_eq!(reopened.acknowledged(), 2);
            seen.push((recovered.encode().unwrap(), recovered.location));
        }
        assert_eq!(seen.len(), 2);
        assert_eq!(seen.first(), seen.last());
        assert_eq!(
            seen.first().unwrap(),
            &(pending.encode().unwrap(), pending.location)
        );
    });
}

#[test]
fn a_second_owner_is_refused_while_a_deep_recovery_holds_the_journal_and_then_agrees() {
    // Out of the box: two invocations reach one deep journal at once. The
    // second is refused by name while the first recovers on its small stack,
    // and once released it recovers to the same refusal; nothing is changed.
    let scratch = Scratch::new().unwrap();
    let id = hash(DECLARATION);
    let mut journal = Journal::open(&scratch.0, NAMESPACE, id).unwrap();
    chain(&mut journal, DEEP, 2);
    drop(journal);
    let root = &scratch.0;
    let held = Journal::open(root, NAMESPACE, id).unwrap();
    let (blocked, first) = std::thread::scope(|scope| {
        let second = scope.spawn(|| {
            bounded("checkpoint-second-owner", || {
                Journal::open(root, NAMESPACE, id).err().unwrap()
            })
        });
        let first = bounded("checkpoint-first-owner", || {
            restore_full(&held, root, deep_budget(), 2).err().unwrap()
        });
        (second.join().unwrap(), first)
    });
    assert!(blocked.contains("already owned"), "{blocked}");
    drop(held);
    let second = bounded("checkpoint-second-owner-after", || {
        let journal = Journal::open(root, NAMESPACE, id).unwrap();
        (
            restore_full(&journal, root, deep_budget(), 2)
                .err()
                .unwrap(),
            journal.acknowledged(),
        )
    });
    assert_eq!(second, (first, 2 * DEEP + 1));
}

/// One edit of a first frame, given the reservation and completion to borrow from.
type FirstChange = fn(&mut Frame, &Frame, &Frame);
/// One edit of a (prior, next) pair, given the completion to borrow from.
type Change = fn(&mut Frame, &mut Frame, &Frame);

/// An independent copy of `frame` saved at `at`, through the canonical codec.
fn copy(frame: &Frame, at: (u64, [u8; 32])) -> Frame {
    let saved = Saved {
        sequence: at.0,
        payload: frame.encode().unwrap(),
        seal: at.1,
    };
    Frame::decode(&saved, DECLARATION, BYTES, 1000, SCOPE).unwrap()
}
fn foreign_batch() -> Batch {
    Batch::prepare(
        Cursor::new(&[30]).unwrap(),
        0,
        0,
        Budget {
            programs: 2,
            nodes: NODES,
            bytes: BYTES,
        },
    )
    .unwrap()
}
/// A declaration, its reservation and its completion, each at its own place.
fn triple() -> (Frame, Frame, Frame) {
    let mut declaration =
        Frame::declaration(DECLARATION.to_vec(), Cursor::new(&[30, 31]).unwrap(), SCOPE);
    declaration.location = Some((1, [5; 32]));
    let mut pending = next(&declaration);
    pending.location = Some((2, [6; 32]));
    let reserved = copy(&pending, (2, [6; 32]));
    let mut done = Frame::done(reserved, &fake_links()).unwrap();
    done.location = Some((3, [7; 32]));
    (declaration, pending, done)
}

#[test]
fn every_first_frame_field_that_is_not_an_empty_declaration_is_refused() {
    let (declaration, pending, done) = triple();
    assert!(transition(None, &declaration).is_ok());
    let edits: [(&str, FirstChange); 9] = [
        ("predecessor", |f, _, _| f.previous = Some((0, [1; 32]))),
        ("pending", |f, _, _| f.pending = true),
        ("exhausted", |f, _, _| f.exhausted = true),
        ("batches", |f, _, _| f.completed_batches = 1),
        ("work", |f, _, _| f.work = 1),
        ("programs", |f, _, _| f.programs = 1),
        ("batch", |f, p, _| {
            f.batch = Some(
                Batch::decode(&p.batch.as_ref().unwrap().encode().unwrap(), BYTES, 1000).unwrap(),
            );
        }),
        ("links", |f, _, _| f.links = fake_links()),
        ("cursor", |f, _, d| f.cursor = d.cursor.clone()),
    ];
    for (name, change) in edits {
        let mut changed = copy(&declaration, (1, [5; 32]));
        change(&mut changed, &pending, &done);
        assert_eq!(
            transition(None, &changed).err().unwrap(),
            "single-stop first checkpoint is not an empty initial declaration",
            "{name}"
        );
    }
}

#[test]
fn every_reservation_and_completion_field_that_does_not_follow_is_refused() {
    let (declaration, pending, done) = triple();
    assert!(transition(Some(&declaration), &pending).is_ok());
    assert!(transition(Some(&pending), &done).is_ok());
    let ancestry = "single-stop checkpoint ancestry or terminal transition differs";
    let reserve = "single-stop reservation changed or skipped its acknowledged cursor";
    let complete = "single-stop completion changed the reserved programs or counters";
    // (refusal, which pair, change to the prior and the next)
    let cases: [(&str, bool, Change); 22] = [
        (ancestry, false, |_, n, _| n.previous = Some((9, [9; 32]))),
        (ancestry, false, |_, n, _| n.declaration = b"other".to_vec()),
        (ancestry, false, |_, n, _| n.scope = 1),
        (ancestry, false, |p, _, _| p.exhausted = true),
        (reserve, false, |p, _, _| p.pending = true),
        (reserve, false, |_, n, _| n.completed_batches = 1),
        (reserve, false, |_, n, _| n.work = 5),
        (reserve, false, |_, n, _| n.programs = 5),
        (reserve, false, |_, n, _| n.exhausted = true),
        (reserve, false, |_, n, d| n.cursor = d.cursor.clone()),
        (reserve, false, |_, n, _| n.links = fake_links()),
        (reserve, false, |_, n, _| n.batch = Some(foreign_batch())),
        (complete, true, |p, _, _| p.pending = false),
        (complete, true, |_, n, _| n.batch = Some(foreign_batch())),
        (complete, true, |_, n, _| n.completed_batches = 5),
        (complete, true, |_, n, _| n.work = 999),
        (complete, true, |_, n, _| n.programs = 999),
        (complete, true, |p, n, _| n.cursor = p.cursor.clone()),
        (complete, true, |_, n, _| n.exhausted = true),
        (
            "single-stop completion has missing or foreign timeframe evidence",
            true,
            |_, n, _| n.links[2] = None,
        ),
        (
            "single-stop continuation omitted its grammar body",
            true,
            |_, n, _| n.batch = None,
        ),
        (
            "single-stop completion has no previous batch",
            true,
            |p, _, _| p.batch = None,
        ),
    ];
    for (index, (refusal, completion, change)) in cases.into_iter().enumerate() {
        let (mut prior, mut next) = if completion {
            (copy(&pending, (2, [6; 32])), copy(&done, (3, [7; 32])))
        } else {
            (
                copy(&declaration, (1, [5; 32])),
                copy(&pending, (2, [6; 32])),
            )
        };
        change(&mut prior, &mut next, &done);
        assert_eq!(
            transition(Some(&prior), &next).err().unwrap(),
            refusal,
            "case {index}"
        );
    }
}

#[test]
fn an_empty_batch_completes_without_children_and_keeps_the_last_saved_table() {
    let (_, _, done) = triple();
    let one_node = Budget {
        programs: 2,
        nodes: 1,
        bytes: BYTES,
    };
    // After the one-instruction program the next needs more than one node,
    // so this reservation holds no program at all.
    let batch = Batch::prepare(done.cursor.clone(), done.work, done.programs, one_node).unwrap();
    let mut first = Frame::pending(&done, batch).unwrap();
    first.location = Some((4, [8; 32]));
    let mut empty = first;
    loop {
        if empty.batch.as_ref().unwrap().programs().is_empty() {
            break;
        }
        let mut finished = Frame::done(empty, &fake_links()).unwrap();
        finished.location = Some((5, [9; 32]));
        let batch = Batch::prepare(
            finished.cursor.clone(),
            finished.work,
            finished.programs,
            one_node,
        )
        .unwrap();
        empty = Frame::pending(&finished, batch).unwrap();
        empty.location = Some((6, [10; 32]));
    }
    let carried = empty.latest_saved;
    assert!(carried.is_some(), "the earlier completions saved a table");
    // The saved table is rebuilt from ancestors, never decoded, so the prior
    // for the transition is a codec copy and the completion takes the original.
    let prior = copy(&empty, empty.location.unwrap());
    assert!(Frame::done(copy(&empty, empty.location.unwrap()), &fake_links()).is_err());
    let mut completed = Frame::done(empty, &[None; 8]).unwrap();
    assert!(completed.links.iter().all(Option::is_none));
    assert_eq!(completed.latest_saved, carried);
    completed.location = Some((7, [11; 32]));
    assert!(transition(Some(&prior), &completed).is_ok());
}

#[test]
fn a_completion_needs_a_held_reservation_and_whole_nonzero_child_links() {
    let (declaration, pending, _) = triple();
    assert_eq!(
        Frame::done(copy(&declaration, (1, [5; 32])), &fake_links())
            .err()
            .unwrap(),
        "single-stop completion lacks its acknowledged reservation"
    );
    for (identity, pin) in [([0; 32], [2; 32]), ([1; 32], [0; 32])] {
        let links =
            std::array::from_fn(|n| (SCOPE & (1 << n) != 0).then_some(Link { identity, pin }));
        assert_eq!(
            Frame::done(copy(&pending, (2, [6; 32])), &links)
                .err()
                .unwrap(),
            "single-stop completion does not cover exactly its selected timeframes"
        );
    }
}

#[test]
fn the_decoder_refuses_oversize_unscoped_unpinned_later_and_half_linked_frames() {
    let (declaration, pending, _) = triple();
    let at = |frame: &Frame, sequence: u64| Saved {
        sequence,
        payload: frame.encode().unwrap(),
        seal: [3; 32],
    };
    let saved = at(&pending, 2);
    let exact = saved.payload.len() as u64;
    assert!(Frame::decode(&saved, DECLARATION, exact, 1000, SCOPE).is_ok());
    assert_eq!(
        Frame::decode(&saved, DECLARATION, exact - 1, 1000, SCOPE)
            .err()
            .unwrap(),
        "single-stop checkpoint byte admission refused"
    );
    let unscoped = Frame::declaration(DECLARATION.to_vec(), Cursor::new(&[30, 31]).unwrap(), 0);
    assert_eq!(
        Frame::decode(&at(&unscoped, 1), DECLARATION, BYTES, 1000, 0)
            .err()
            .unwrap(),
        "single-stop checkpoint scope or flags refused"
    );
    let earlier = "single-stop checkpoint predecessor is not earlier and pinned";
    // The predecessor is sequence 1: a frame saved at 1 or before is refused.
    for sequence in [0, 1] {
        assert_eq!(
            Frame::decode(&at(&pending, sequence), DECLARATION, BYTES, 1000, SCOPE)
                .err()
                .unwrap(),
            earlier
        );
    }
    let mut unpinned = at(&pending, 2);
    unpinned.payload.get_mut(24..56).unwrap().fill(0);
    assert_eq!(
        Frame::decode(&unpinned, DECLARATION, BYTES, 1000, SCOPE)
            .err()
            .unwrap(),
        earlier
    );
    let links = 96 + CURSOR_BYTES;
    for (from, to) in [(links, links + 32), (links + 32, links + 64)] {
        let mut half = at(&declaration, 1);
        half.payload.get_mut(from..to).unwrap().fill(1);
        assert_eq!(
            Frame::decode(&half, DECLARATION, BYTES, 1000, SCOPE)
                .err()
                .unwrap(),
            "single-stop child has only half an identity/pin pair"
        );
    }
}
