#![expect(
    clippy::unwrap_used,
    reason = "finite durable checkpoint fault assertions"
)]
use super::*;
use crate::search_checkpoint::tests::Scratch;

const BYTES: u64 = 1024 * 1024;

fn budget() -> Budget {
    Budget {
        programs: 2,
        nodes: 100,
        bytes: BYTES - (ENVELOPE + 96) as u64,
    }
}

#[test]
fn restart_keeps_unfinished_batch_and_requires_exact_campaign_before_advancing() {
    let scratch = Scratch::new().unwrap();
    let initial = Cursor::new(&[30, 31]).unwrap();
    let first = Batch::prepare(initial.clone(), 0, 0, budget()).unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [3; 32]).unwrap();
    let plan = journal
        .publish(&plan_record((0, [0; 32]), &first).unwrap(), BYTES)
        .unwrap();
    drop(journal);
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [3; 32]).unwrap();
    let latest = journal.latest(BYTES).unwrap();
    let pending = restore(
        &journal,
        latest.as_ref(),
        &initial,
        budget(),
        100,
        |_, _, _| Err("unexpected validation".into()),
    )
    .unwrap();
    assert_eq!(pending.work, 0);
    assert_eq!(pending.programs, 0);
    assert_eq!(
        pending.pending.unwrap().1.encode().unwrap(),
        first.encode().unwrap()
    );
    journal
        .publish(&done_record(plan, ([7; 32], [8; 32])), BYTES)
        .unwrap();
    let latest = journal.latest(BYTES).unwrap();
    assert!(
        restore(
            &journal,
            latest.as_ref(),
            &initial,
            budget(),
            100,
            |_, _, _| Err("saved campaign corrupted".into())
        )
        .is_err()
    );
    let mut observed = 0;
    let resumed = restore(
        &journal,
        latest.as_ref(),
        &initial,
        budget(),
        100,
        |id, pin, programs| {
            assert_eq!(id, [7; 32]);
            assert_eq!(pin, [8; 32]);
            assert_eq!(programs, first.programs());
            observed += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(observed, 1);
    assert!(resumed.pending.is_none());
    assert_eq!(resumed.programs, 2);
    assert_eq!(resumed.work, first.work());
    assert_eq!(resumed.cursor.encode(), first.next_cursor().encode());
    assert!(!resumed.exhausted);
    assert!(
        restore(
            &journal,
            latest.as_ref(),
            &initial,
            budget(),
            1,
            |_, _, _| Ok(())
        )
        .is_err()
    );
}

#[test]
fn sealed_but_skipped_reordered_foreign_and_incomplete_history_refuses() {
    let initial = Cursor::new(&[30, 31]).unwrap();
    let first = Batch::prepare(initial.clone(), 0, 0, budget()).unwrap();
    for mode in 0..6 {
        let scratch = Scratch::new().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, [4; 32]).unwrap();
        let first_pin = journal
            .publish(&plan_record((0, [0; 32]), &first).unwrap(), BYTES)
            .unwrap();
        let payload = match mode {
            0 => plan_record(first_pin, &first).unwrap(),
            1 => done_record((first_pin.0, [99; 32]), ([7; 32], [8; 32])),
            2 => done_record(first_pin, ([0; 32], [0; 32])),
            3 => done_record((first_pin.0 + 1, first_pin.1), ([7; 32], [8; 32])),
            4 => {
                let mut raw = done_record(first_pin, ([7; 32], [8; 32]));
                raw.push(0);
                raw
            }
            _ => {
                let mut raw = done_record(first_pin, ([7; 32], [8; 32]));
                *raw.first_mut().unwrap() = b'?';
                raw
            }
        };
        journal.publish(&payload, BYTES).unwrap();
        let latest = journal.latest(BYTES).unwrap();
        assert!(
            restore(
                &journal,
                latest.as_ref(),
                &initial,
                budget(),
                100,
                |_, _, _| Ok(())
            )
            .is_err(),
            "case {mode}"
        );
    }
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [4; 32]).unwrap();
    let skipped = Batch::prepare(first.next_cursor(), first.work(), 2, budget()).unwrap();
    journal
        .publish(&plan_record((0, [0; 32]), &skipped).unwrap(), BYTES)
        .unwrap();
    assert!(
        restore(
            &journal,
            journal.latest(BYTES).unwrap().as_ref(),
            &initial,
            budget(),
            100,
            |_, _, _| Ok(())
        )
        .is_err()
    );
}

#[test]
fn empty_node_work_checkpoint_can_resume_without_inventing_a_campaign() {
    let initial = Cursor::new(&[30]).unwrap();
    let budget = Budget {
        programs: 2,
        nodes: 1,
        ..budget()
    };
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [5; 32]).unwrap();
    let first = Batch::prepare(initial.clone(), 0, 0, budget).unwrap();
    let plan = journal
        .publish(&plan_record((0, [0; 32]), &first).unwrap(), BYTES)
        .unwrap();
    let done = journal
        .publish(&done_record(plan, ([7; 32], [8; 32])), BYTES)
        .unwrap();
    let empty = Batch::prepare(first.next_cursor(), first.work(), 1, budget).unwrap();
    assert!(empty.programs().is_empty());
    let plan = journal
        .publish(&plan_record(done, &empty).unwrap(), BYTES)
        .unwrap();
    journal
        .publish(&done_record(plan, ([0; 32], [0; 32])), BYTES)
        .unwrap();
    let mut calls = 0;
    let resumed = restore(
        &journal,
        journal.latest(BYTES).unwrap().as_ref(),
        &initial,
        budget,
        100,
        |_, _, _| {
            calls += 1;
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(resumed.programs, 1);
    assert_eq!(resumed.work, 2);
    assert!(!resumed.exhausted);
}

#[test]
fn command_shape_refuses_invalid_alphabet_spans_and_work_allowances() {
    let args = [
        "zerodha", "NIFTY", "2025", "4", "2025", "5", "30,31", "5", "50", "2", "100", "output",
    ];
    assert!(parse(&args).is_ok());
    for (at, value) in [
        (3, "13"),
        (2, "2026"),
        (6, "31,30"),
        (6, "30,30"),
        (6, "999"),
        (7, "0"),
        (8, "0"),
        (9, "0"),
        (10, "0"),
    ] {
        let mut changed = args;
        *changed.get_mut(at).unwrap() = value;
        assert!(parse(&changed).is_err(), "{at} {value}");
    }
    assert!(parse(&[]).is_err());
}

#[test]
fn checkpoint_counter_reseeding_and_changed_batch_boundaries_refuse() {
    let initial = Cursor::new(&[30, 31]).unwrap();
    for mode in 0..3 {
        let scratch = Scratch::new().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, [6; 32]).unwrap();
        let chosen = if mode == 2 {
            Budget {
                programs: 1,
                ..budget()
            }
        } else {
            budget()
        };
        let batch = Batch::prepare(
            initial.clone(),
            u64::from(mode == 0),
            u64::from(mode == 1),
            chosen,
        )
        .unwrap();
        journal
            .publish(&plan_record((0, [0; 32]), &batch).unwrap(), BYTES)
            .unwrap();
        assert!(
            restore(
                &journal,
                journal.latest(BYTES).unwrap().as_ref(),
                &initial,
                budget(),
                100,
                |_, _, _| Ok(())
            )
            .is_err(),
            "case {mode}"
        );
    }
}

#[test]
fn checkpoint_admission_reserves_plan_and_done_before_work_and_restarts_at_exact_cap() {
    let scratch = Scratch::new().unwrap();
    let identity = [21; 32];
    let initial = Cursor::new(&[30, 31]).unwrap();
    let first = Batch::prepare(initial.clone(), 0, 0, budget()).unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    assert!(admit_checkpoints(journal.next_sequence(), false, 1, BYTES).is_err());
    assert!(journal.latest(BYTES).unwrap().is_none());
    assert_eq!(journal.next_sequence(), 1);
    admit_checkpoints(journal.next_sequence(), false, 2, BYTES).unwrap();
    let plan = journal
        .publish(&plan_record((0, [0; 32]), &first).unwrap(), BYTES)
        .unwrap();
    drop(journal);
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    let pending = restore(
        &journal,
        journal.latest(BYTES).unwrap().as_ref(),
        &initial,
        budget(),
        2,
        |_, _, _| Err("unfinished plan must not be priced on read".into()),
    )
    .unwrap();
    assert!(pending.pending.is_some());
    assert!(admit_checkpoints(journal.next_sequence(), false, 2, BYTES).is_err());
    admit_checkpoints(journal.next_sequence(), true, 2, BYTES).unwrap();
    let done = publish_done(
        &mut journal,
        &done_record(plan, ([7; 32], [8; 32])),
        BYTES,
        || Ok(()),
    )
    .unwrap();
    drop(journal);
    let journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    let resumed = restore(
        &journal,
        journal.latest(BYTES).unwrap().as_ref(),
        &initial,
        budget(),
        2,
        |id, pin, programs| {
            assert_eq!((id, pin), ([7; 32], [8; 32]));
            assert_eq!(programs, first.programs());
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(resumed.programs, 2);
    assert!(resumed.pending.is_none());
    assert!(!resumed.exhausted);
    assert!(admit_checkpoints(journal.next_sequence(), false, 2, BYTES).is_err());
    assert_eq!(journal.latest(BYTES).unwrap().unwrap().seal, done.1);
}

#[test]
fn checkpoint_admission_charges_crash_holes_memory_discovery_and_overflow() {
    let scratch = Scratch::new().unwrap();
    let identity = [22; 32];
    let journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    drop(journal);
    let directory = scratch
        .0
        .join(NAMESPACE)
        .join(crate::identity_hex(&identity));
    // A reserved directory with no marker represents a crashed publication.
    std::fs::create_dir(directory.join("0000000000000003")).unwrap();
    let journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    assert_eq!(journal.interrupted(), 1);
    assert_eq!(journal.next_sequence(), 4);
    assert!(journal.latest(BYTES).unwrap().is_none());
    assert!(admit_checkpoints(journal.next_sequence(), false, 4, BYTES).is_err());
    admit_checkpoints(journal.next_sequence(), false, 5, BYTES).unwrap();
    assert!(admit_checkpoints(1, false, 100, 79).is_err());
    admit_checkpoints(1, false, 100, 80).unwrap();
    let physical = crate::search_checkpoint::DIRECTORY_LIMIT as u64;
    admit_checkpoints(physical - 2, false, u64::MAX, u64::MAX).unwrap();
    assert!(admit_checkpoints(physical - 1, false, u64::MAX, u64::MAX).is_err());
    admit_checkpoints(physical - 1, true, u64::MAX, u64::MAX).unwrap();
    assert!(admit_checkpoints(physical, true, u64::MAX, u64::MAX).is_err());
    assert!(admit_checkpoints(u64::MAX, false, u64::MAX, u64::MAX).is_err());
    assert!(admit_checkpoints(0, true, u64::MAX, u64::MAX).is_err());
    drop(journal);
    let journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    assert_eq!(journal.next_sequence(), 4);
    assert!(admit_checkpoints(journal.next_sequence(), false, 4, BYTES).is_err());
}

#[test]
fn grammar_done_holds_and_rechecks_source_generation_before_and_after_acknowledgment() {
    use std::fs::{self, File};

    for mutate_at in 0..=2 {
        let scratch = Scratch::new().unwrap();
        let identity = [23; 32];
        let path = scratch.0.join("retained-source-receipt");
        fs::write(&path, b"exact original receipt").unwrap();
        let source = crate::readonly_file::open(&path).unwrap();
        source.try_lock_shared().unwrap();
        let before = crate::result_set::file_generation(&source, &path).unwrap();
        let batch = Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, budget()).unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
        let plan = journal
            .publish(&plan_record((0, [0; 32]), &batch).unwrap(), BYTES)
            .unwrap();
        let completed_path = scratch
            .0
            .join(NAMESPACE)
            .join(crate::identity_hex(&identity))
            .join("0000000000000002/complete");
        let mut checks = 0;
        let result = publish_done(
            &mut journal,
            &done_record(plan, ([7; 32], [8; 32])),
            BYTES,
            || {
                checks += 1;
                assert_eq!(completed_path.exists(), checks == 2);
                assert!(matches!(
                    File::open(&path).unwrap().try_lock(),
                    Err(std::fs::TryLockError::WouldBlock)
                ));
                if checks == mutate_at {
                    let replacement = scratch.0.join("foreign-receipt");
                    fs::write(&replacement, b"exact original receipt").unwrap();
                    fs::rename(&replacement, &path).unwrap();
                }
                let after = crate::result_set::file_generation(&source, &path)?;
                crate::result_set::require_generation_unchanged(before, after, &path)
            },
        );
        assert_eq!(checks, if mutate_at == 1 { 1 } else { 2 });
        assert_eq!(result.is_ok(), mutate_at == 0);
        // A post-acknowledgment refusal is visible without deleting history.
        assert_eq!(
            journal.latest(BYTES).unwrap().unwrap().sequence,
            if mutate_at == 1 { 1 } else { 2 }
        );
        drop(source);
        File::open(&path).unwrap().try_lock().unwrap();
    }
}

#[test]
fn parsed_request_and_campaign_projection_preserve_all_declared_inputs() {
    let args = [
        "zerodha",
        "NIFTY,BANKNIFTY",
        "2025",
        "5",
        "2025",
        "5",
        "all",
        "7",
        "123",
        "11",
        "999",
        "archive",
    ];
    let request = parse(&args).unwrap();
    assert_eq!(
        request.initial.encode(),
        Cursor::new(&runner::live_positions()).unwrap().encode()
    );
    let programs = vec![runner::expression::Expression::parse("30 | !31").unwrap()];
    let campaign = request.campaign(&programs);
    assert_eq!(campaign.vendor, "zerodha");
    assert_eq!(campaign.symbols, "NIFTY,BANKNIFTY");
    assert_eq!(campaign.from, (2025, 5));
    assert_eq!(campaign.to, (2025, 5));
    assert_eq!(campaign.programs, programs);
    assert_eq!(campaign.horizon.as_bars(), 7);
    assert_eq!(campaign.max_points, 123);
    assert_eq!(campaign.output, Path::new("archive"));
    assert_eq!(campaign.max_rung_jobs, 8);
    assert_eq!(request.programs, 11);
    assert_eq!(request.nodes, 999);
    assert_eq!(positive("18446744073709551615").unwrap(), u64::MAX);
    for invalid in ["", " ", "-1", "0", "18446744073709551616", "1.5", "1x"] {
        assert_eq!(positive(invalid).unwrap_err(), "positive integer required");
    }
    for (at, invalid) in [
        (2, "65536"),
        (3, "256"),
        (6, "not-a-position"),
        (7, "4294967296"),
    ] {
        let mut changed = args;
        *changed.get_mut(at).unwrap() = invalid;
        assert!(parse(&changed).is_err());
    }
    let exact_limit = format!("{}30", "0".repeat(4094));
    let mut exact = args;
    *exact.get_mut(6).unwrap() = &exact_limit;
    assert_eq!(
        parse(&exact).unwrap().initial.encode(),
        Cursor::new(&[30]).unwrap().encode()
    );
    let oversized = format!("0{exact_limit}");
    let mut changed = args;
    *changed.get_mut(6).unwrap() = &oversized;
    assert_eq!(
        parse(&changed).err().unwrap(),
        "grammar alphabet exceeds its input byte admission"
    );
}

#[test]
fn empty_history_and_overflowing_restore_admission_never_invent_progress() {
    let scratch = Scratch::new().unwrap();
    let initial = Cursor::new(&[30, 31]).unwrap();
    let journal = Journal::open(&scratch.0, NAMESPACE, [24; 32]).unwrap();
    let state = restore(&journal, None, &initial, budget(), 0, |_, _, _| {
        Err("unexpected child".into())
    })
    .unwrap();
    assert_eq!(state.cursor.encode(), initial.encode());
    assert_eq!((state.work, state.programs, state.exhausted), (0, 0, false));
    assert!(state.pending.is_none());
    let invalid = Budget {
        bytes: u64::MAX,
        ..budget()
    };
    assert_eq!(
        restore(&journal, None, &initial, invalid, 100, |_, _, _| Ok(()))
            .err()
            .unwrap(),
        "grammar journal ceiling overflow"
    );
    assert!(journal.latest(BYTES).unwrap().is_none());
}

#[test]
fn empty_batch_completion_requires_both_campaign_words_absent() {
    let initial = Cursor::new(&[30]).unwrap();
    let allowed = Budget {
        nodes: 1,
        ..budget()
    };
    let first = Batch::prepare(initial.clone(), 0, 0, allowed).unwrap();
    let empty = Batch::prepare(first.next_cursor(), first.work(), 1, allowed).unwrap();
    assert!(empty.programs().is_empty());
    let request = parse(&[
        "zerodha", "NIFTY", "2025", "5", "2025", "5", "30", "5", "50", "2", "1", "unused",
    ])
    .unwrap();
    let mut output = String::new();
    assert_eq!(
        complete_batch(
            &request,
            &empty,
            [7; 32],
            &mut output,
            crate::boolean_campaign::prepare
        )
        .unwrap(),
        ([0; 32], [0; 32])
    );
    assert!(output.is_empty());
    for campaign in [([1; 32], [0; 32]), ([0; 32], [2; 32]), ([1; 32], [2; 32])] {
        let scratch = Scratch::new().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, [25; 32]).unwrap();
        let plan = journal
            .publish(&plan_record((0, [0; 32]), &first).unwrap(), BYTES)
            .unwrap();
        let done = journal
            .publish(&done_record(plan, ([7; 32], [8; 32])), BYTES)
            .unwrap();
        let plan = journal
            .publish(&plan_record(done, &empty).unwrap(), BYTES)
            .unwrap();
        journal
            .publish(&done_record(plan, campaign), BYTES)
            .unwrap();
        assert_eq!(
            restore(
                &journal,
                journal.latest(BYTES).unwrap().as_ref(),
                &initial,
                allowed,
                100,
                |_, _, _| Ok(())
            )
            .err()
            .unwrap(),
            "empty grammar work unexpectedly names a campaign"
        );
    }
}

#[test]
fn nonempty_done_needs_both_child_words_and_cannot_start_the_history() {
    let initial = Cursor::new(&[30]).unwrap();
    let batch = Batch::prepare(initial.clone(), 0, 0, budget()).unwrap();
    for campaign in [([7; 32], [0; 32]), ([0; 32], [8; 32])] {
        let scratch = Scratch::new().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, [26; 32]).unwrap();
        let plan = journal
            .publish(&plan_record((0, [0; 32]), &batch).unwrap(), BYTES)
            .unwrap();
        journal
            .publish(&done_record(plan, campaign), BYTES)
            .unwrap();
        assert_eq!(
            restore(
                &journal,
                journal.latest(BYTES).unwrap().as_ref(),
                &initial,
                budget(),
                100,
                |_, _, _| Ok(())
            )
            .err()
            .unwrap(),
            "nonempty grammar batch has no campaign completion"
        );
    }
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [27; 32]).unwrap();
    journal
        .publish(&done_record((0, [0; 32]), ([7; 32], [8; 32])), BYTES)
        .unwrap();
    assert_eq!(
        restore(
            &journal,
            journal.latest(BYTES).unwrap().as_ref(),
            &initial,
            budget(),
            100,
            |_, _, _| Ok(())
        )
        .err()
        .unwrap(),
        "grammar completion has no preceding batch"
    );
}

#[test]
fn history_memory_and_predecessor_boundaries_refuse_before_replaying() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [28; 32]).unwrap();
    let batch = Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, budget()).unwrap();
    let mut previous = (0, [0; 32]);
    for _ in 0..9 {
        previous = journal
            .publish(&plan_record(previous, &batch).unwrap(), BYTES)
            .unwrap();
    }
    let latest = journal.latest(BYTES).unwrap();
    let chain = discover_chain(&journal, latest.as_ref(), BYTES, 360, 9).unwrap();
    assert_eq!(chain.len(), 9);
    assert!(
        chain.capacity() <= 9,
        "retained chain capacity exceeds its admitted records"
    );
    assert_eq!(chain.first(), Some(&previous));
    assert_eq!(chain.last().unwrap().0, 1);
    assert!(discover_chain(&journal, latest.as_ref(), BYTES, 359, 9).is_err());
    assert!(discover_chain(&journal, latest.as_ref(), BYTES, 360, 8).is_err());
    for predecessor in [(0, [7; 32]), (1, [7; 32]), (2, [7; 32]), (1, [0; 32])] {
        let scratch = Scratch::new().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, [29; 32]).unwrap();
        journal
            .publish(&plan_record(predecessor, &batch).unwrap(), BYTES)
            .unwrap();
        assert_eq!(
            discover_chain(
                &journal,
                journal.latest(BYTES).unwrap().as_ref(),
                BYTES,
                BYTES,
                100
            )
            .unwrap_err(),
            "grammar checkpoint predecessor is invalid"
        );
    }
}

#[test]
fn earlier_nonzero_predecessor_requires_its_nonzero_seal_before_replay() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [49; 32]).unwrap();
    let batch = Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, budget()).unwrap();
    let first = journal
        .publish(&plan_record((0, [0; 32]), &batch).unwrap(), BYTES)
        .unwrap();
    assert_eq!(first.0, 1);
    journal
        .publish(&plan_record((first.0, [0; 32]), &batch).unwrap(), BYTES)
        .unwrap();
    let latest = journal.latest(BYTES).unwrap();
    assert_eq!(latest.as_ref().unwrap().sequence, 2);
    assert_eq!(
        discover_chain(&journal, latest.as_ref(), BYTES, BYTES, 100).unwrap_err(),
        "grammar checkpoint predecessor is invalid",
        "an otherwise ordered link must refuse its missing seal before reading the predecessor"
    );
}

#[test]
fn changed_node_budget_and_checkpoint_after_terminal_exhaustion_refuse() {
    let initial = Cursor::new(&[30]).unwrap();
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [30; 32]).unwrap();
    let changed = Batch::prepare(
        initial.clone(),
        0,
        0,
        Budget {
            nodes: 99,
            ..budget()
        },
    )
    .unwrap();
    journal
        .publish(&plan_record((0, [0; 32]), &changed).unwrap(), BYTES)
        .unwrap();
    assert_eq!(
        restore(
            &journal,
            journal.latest(BYTES).unwrap().as_ref(),
            &initial,
            budget(),
            100,
            |_, _, _| Ok(())
        )
        .err()
        .unwrap(),
        "grammar history skips or repeats cursor work"
    );
    let mut raw = initial.encode();
    raw.get_mut(10..12).unwrap().copy_from_slice(
        &u16::try_from(runner::expression::MAX_INSTRUCTIONS)
            .unwrap()
            .to_le_bytes(),
    );
    *raw.get_mut(14).unwrap() = 1;
    let terminal = Cursor::decode(&raw).unwrap();
    let batch = Batch::prepare(terminal.clone(), 0, 0, budget()).unwrap();
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [31; 32]).unwrap();
    let plan = journal
        .publish(&plan_record((0, [0; 32]), &batch).unwrap(), BYTES)
        .unwrap();
    let done = journal
        .publish(&done_record(plan, ([0; 32], [0; 32])), BYTES)
        .unwrap();
    let state = restore(
        &journal,
        journal.latest(BYTES).unwrap().as_ref(),
        &terminal,
        budget(),
        100,
        |_, _, _| Err("terminal must have no campaign".into()),
    )
    .unwrap();
    assert!(state.exhausted);
    assert_eq!((state.programs, state.work), (0, 0));
    journal
        .publish(&plan_record(done, &batch).unwrap(), BYTES)
        .unwrap();
    assert_eq!(
        restore(
            &journal,
            journal.latest(BYTES).unwrap().as_ref(),
            &terminal,
            budget(),
            100,
            |_, _, _| Ok(())
        )
        .err()
        .unwrap(),
        "grammar history changed or continued after exhaustion"
    );
}

#[test]
fn truncated_checkpoint_fields_and_command_misuse_keep_visible_refusal() {
    assert_eq!(
        field::<8>(&[1, 2, 3], 0).unwrap_err(),
        "grammar checkpoint field missing"
    );
    assert!(field::<32>(&[], usize::MAX).is_err());
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [32; 32]).unwrap();
    journal.publish(PLAN, BYTES).unwrap();
    assert_eq!(
        discover_chain(
            &journal,
            journal.latest(BYTES).unwrap().as_ref(),
            BYTES,
            BYTES,
            100
        )
        .unwrap_err(),
        "grammar checkpoint field missing"
    );
    let mut out = String::new();
    assert_eq!(command(&[], &mut out), crate::MISUSED);
    assert!(out.contains("requires its 12 explicit arguments"));
    assert!(out.starts_with("refused: "));
}

#[test]
fn absent_or_insufficient_physical_admission_refuses_before_journal_creation() {
    const CHILD: &str = "BRUTEX_GRAMMAR_ADMISSION_CHILD";
    if let Ok(mode) = std::env::var(CHILD) {
        let root = std::env::var("BRUTEX_STORE").unwrap();
        let args = [
            "zerodha", "NIFTY", "2025", "5", "2025", "5", "30", "5", "50", "2", "100", &root,
        ];
        let mut out = String::new();
        assert_eq!(command(&args, &mut out), crate::MISUSED);
        let expected = match mode.as_str() {
            "missing" => {
                "strict input configuration is unavailable; missing [BRUTEX_CHECKSUM_RECEIPTS, BRUTEX_CHECKSUM_MAX_BYTES, BRUTEX_CHECKSUM_MAX_RECORDS]"
            }
            "143" => "grammar journal byte ceiling is too small",
            "144" => "grammar batch exceeds its complete binary byte admission",
            _ => unreachable!("parent supplies a finite explicit mode"),
        };
        assert!(out.contains(expected), "{out}");
        assert!(!Path::new(&root).join(NAMESPACE).exists());
        return;
    }
    for mode in ["missing", "143", "144"] {
        let scratch = Scratch::new().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child.args(["--exact", "boolean_grammar_campaign::tests::absent_or_insufficient_physical_admission_refuses_before_journal_creation", "--test-threads=1"]);
        for (key, _) in std::env::vars_os() {
            if key.to_string_lossy().starts_with("BRUTEX_") {
                child.env_remove(key);
            }
        }
        child.env(CHILD, mode).env("BRUTEX_STORE", &scratch.0);
        if mode != "missing" {
            child
                .env("BRUTEX_CHECKSUM_RECEIPTS", &scratch.0)
                .env("BRUTEX_CHECKSUM_MAX_BYTES", mode)
                .env("BRUTEX_CHECKSUM_MAX_RECORDS", "100");
        }
        let result = child.output().unwrap();
        assert!(
            result.status.success(),
            "mode={mode}\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stdout).contains("1 passed"));
        assert!(!scratch.0.join(NAMESPACE).exists());
    }
}

#[test]
fn reauthenticated_checkpoint_replacement_after_discovery_cannot_change_history() {
    use std::fs;
    let scratch = Scratch::new().unwrap();
    let alternate = Scratch::new().unwrap();
    let identity = [33; 32];
    let initial = Cursor::new(&[30, 31]).unwrap();
    let first = Batch::prepare(initial.clone(), 0, 0, budget()).unwrap();
    let second = Batch::prepare(first.next_cursor(), first.work(), 2, budget()).unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    let plan = journal
        .publish(&plan_record((0, [0; 32]), &first).unwrap(), BYTES)
        .unwrap();
    let done = journal
        .publish(&done_record(plan, ([7; 32], [8; 32])), BYTES)
        .unwrap();
    let plan = journal
        .publish(&plan_record(done, &second).unwrap(), BYTES)
        .unwrap();
    journal
        .publish(&done_record(plan, ([9; 32], [10; 32])), BYTES)
        .unwrap();
    let mut replacement = Journal::open(&alternate.0, NAMESPACE, identity).unwrap();
    for _ in 0..3 {
        replacement
            .publish(&plan_record((0, [0; 32]), &first).unwrap(), BYTES)
            .unwrap();
    }
    let name = Path::new(NAMESPACE)
        .join(crate::identity_hex(&identity))
        .join("0000000000000003");
    let latest = journal.latest(BYTES).unwrap();
    let mut calls = 0;
    let result = restore(
        &journal,
        latest.as_ref(),
        &initial,
        budget(),
        100,
        |id, pin, programs| {
            calls += 1;
            assert_eq!((id, pin), ([7; 32], [8; 32]));
            assert_eq!(programs, first.programs());
            let directory = scratch.0.join(&name);
            fs::rename(&directory, scratch.0.join("superseded-checkpoint")).unwrap();
            fs::rename(alternate.0.join(&name), directory).unwrap();
            Ok(())
        },
    );
    assert_eq!(calls, 1);
    assert_eq!(
        result.err().unwrap(),
        "grammar history changed or continued after exhaustion"
    );
}

#[test]
fn one_slot_history_admission_bounds_actual_retained_capacity() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [34; 32]).unwrap();
    let batch = Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, budget()).unwrap();
    let expected = journal
        .publish(&plan_record((0, [0; 32]), &batch).unwrap(), BYTES)
        .unwrap();
    let latest = journal.latest(BYTES).unwrap();
    let chain = discover_chain(&journal, latest.as_ref(), BYTES, 40, 100).unwrap();
    assert_eq!(chain.as_slice(), &[expected]);
    assert!(
        chain.capacity() * std::mem::size_of::<(u64, [u8; 32])>() <= 40,
        "a single admitted slot must not use implicit four-slot growth or over-reserve"
    );
    assert_eq!(
        discover_chain(&journal, latest.as_ref(), BYTES, 39, 100).unwrap_err(),
        "grammar checkpoint history exceeds its record admission"
    );
}

#[test]
fn plan_buffer_capacity_stays_within_exact_serialized_admission() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [35; 32]).unwrap();
    let batch = Batch::prepare(Cursor::new(&[30, 31]).unwrap(), 0, 0, budget()).unwrap();
    let payload = batch.encode().unwrap();
    let raw = plan_record((0, [0; 32]), &batch).unwrap();
    let admitted = ENVELOPE + payload.len();
    assert_eq!(raw.len(), admitted);
    assert!(
        raw.capacity() <= admitted,
        "the Plan buffer must not reserve more bytes than its exact admitted encoding"
    );
    assert_eq!(raw.get(ENVELOPE..).unwrap(), payload);
    let (sequence, pin) = journal.publish(&raw, admitted as u64 + 96).unwrap();
    let saved = journal.read(sequence, admitted as u64 + 96).unwrap();
    assert_eq!((saved.seal, saved.payload), (pin, raw));
}

#[test]
fn restore_rejects_sealed_oversize_before_body_decode_or_child_replay() {
    let initial = Cursor::new(&[30, 31]).unwrap();
    let batch = Batch::prepare(initial.clone(), 0, 0, budget()).unwrap();
    let allowed = Budget {
        bytes: batch.encode().unwrap().len() as u64,
        ..budget()
    };
    for oversized in [false, true] {
        let scratch = Scratch::new().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, [36; 32]).unwrap();
        let mut raw = plan_record((0, [0; 32]), &batch).unwrap();
        if oversized {
            raw.push(0);
        }
        journal.publish(&raw, BYTES).unwrap();
        // The oversized record is independently authenticated under a larger
        // writer bound; the smaller recovery bound must refuse before decoding.
        let latest = journal.latest(BYTES).unwrap();
        assert_eq!(latest.as_ref().unwrap().payload, raw);
        let mut calls = 0;
        let restored = restore(
            &journal,
            latest.as_ref(),
            &initial,
            allowed,
            100,
            |_, _, _| {
                calls += 1;
                Ok(())
            },
        );
        assert_eq!(calls, 0);
        if oversized {
            assert_eq!(
                restored.err().unwrap(),
                "checkpoint payload exceeds its type or byte admission"
            );
        } else {
            assert_eq!(
                restored.unwrap().pending.unwrap().1.encode().unwrap(),
                batch.encode().unwrap()
            );
        }
    }
}

#[test]
fn campaign_projection_requires_each_exact_descriptor_and_program_field() {
    let programs = vec![runner::expression::Expression::parse("30 | !31").unwrap()];
    let digest = crate::boolean_campaign::program_digest(&programs);
    require_campaign_binding(([1; 32], digest), [1; 32], &programs).unwrap();
    for actual in [([2; 32], digest), ([1; 32], [3; 32]), ([2; 32], [3; 32])] {
        assert_eq!(
            require_campaign_binding(actual, [1; 32], &programs).unwrap_err(),
            "grammar completion belongs to different programs or source-policy descriptor"
        );
    }
}

#[test]
fn acknowledged_record_requires_both_expected_seal_and_exact_payload() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [37; 32]).unwrap();
    let raw = done_record((1, [1; 32]), ([2; 32], [3; 32]));
    let (sequence, pin) = journal.publish(&raw, BYTES).unwrap();
    let saved = journal.read(sequence, BYTES).unwrap();
    require_acknowledged(&saved, pin, &raw).unwrap();
    let mut different = raw.clone();
    *different.last_mut().unwrap() ^= 1;
    for (expected_pin, expected_raw) in [([9; 32], &raw), (pin, &different), ([9; 32], &different)]
    {
        assert_eq!(
            require_acknowledged(&saved, expected_pin, expected_raw).unwrap_err(),
            "grammar completion changed before acknowledgment"
        );
    }
}

#[test]
fn done_link_requires_exact_plan_sequence_seal_and_record_width() {
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, [38; 32]).unwrap();
    let batch = Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, budget()).unwrap();
    let pin = journal
        .publish(&plan_record((0, [0; 32]), &batch).unwrap(), BYTES)
        .unwrap();
    let plan = journal.read(pin.0, BYTES).unwrap();
    for (link, padding) in [
        (pin, false),
        ((pin.0 + 1, pin.1), false),
        ((pin.0, [7; 32]), false),
        ((pin.0 + 1, [7; 32]), false),
        (pin, true),
    ] {
        let mut raw = done_record(link, ([2; 32], [3; 32]));
        if padding {
            raw.push(0);
        }
        let (sequence, _) = journal.publish(&raw, BYTES).unwrap();
        let done = journal.read(sequence, BYTES).unwrap();
        let result = require_plan_link(&done, &plan);
        if link == pin && !padding {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err(),
                "grammar completion refers to a different batch"
            );
        }
    }
}
