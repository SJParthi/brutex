#![cfg(test)]
//! Finite generated journal faults, not market or qualification-performance proof.
#![expect(clippy::unwrap_used, reason = "exact bounded durable-state assertions")]
use super::*;
use crate::search_checkpoint::tests::Scratch;
use std::fs;

const LIMIT: u64 = 64 * 1024;
const DESCRIPTOR: [u8; 32] = [91; 32];

fn units() -> [[u8; 32]; 8] {
    std::array::from_fn(|index| [u8::try_from(index + 1).unwrap(); 32])
}
fn link(index: usize) -> Link {
    Link {
        identity: [u8::try_from(index + 21).unwrap(); 32],
        pin: [u8::try_from(index + 41).unwrap(); 32],
    }
}
fn open(root: &Path, records: u64) -> Result<Writer, String> {
    Writer::open(
        root,
        DESCRIPTOR,
        units(),
        LIMIT,
        records,
        |index, unit, received| {
            assert_eq!(Some(&unit), units().get(index));
            assert_eq!(received, link(index));
            Ok(())
        },
    )
}
fn directory(root: &Path, id: [u8; 32]) -> std::path::PathBuf {
    root.join(NAMESPACE).join(crate::identity_hex(&id))
}

#[test]
fn all_eight_slots_restart_without_reassignment_and_reuse_exact_pending_reservations() {
    let scratch = Scratch::new().unwrap();
    let mut writer = open(&scratch.0, 17).unwrap();
    assert_eq!(writer.identity(), identity(DESCRIPTOR, units()));
    assert_eq!(writer.journal.acknowledged(), 1);
    for index in 0..8 {
        assert!(!writer.completed());
        writer.begin(index).unwrap();
        let start_pin = writer.pin().unwrap();
        let sequence = writer.journal.next_sequence();
        writer.begin(index).unwrap();
        assert_eq!(writer.pin().unwrap(), start_pin);
        assert_eq!(writer.journal.next_sequence(), sequence);
        drop(writer);
        writer = open(&scratch.0, 17).unwrap();
        writer.begin(index).unwrap();
        assert_eq!(writer.pin().unwrap(), start_pin);
        writer.finish(index, link(index)).unwrap();
        assert_eq!(
            writer.slots().get(index).unwrap().complete,
            Some(link(index))
        );
        assert!(writer.begin(index).is_err());
    }
    assert!(writer.completed());
    let pin = writer.pin().unwrap();
    assert_eq!(writer.journal.acknowledged(), 17);
    drop(writer);
    let writer = open(&scratch.0, 17).unwrap();
    assert!(writer.completed());
    assert_eq!(writer.pin().unwrap(), pin);
    drop(writer);
    assert!(open(&scratch.0, 16).is_err());
}

#[test]
fn selected_nonadjacent_slots_resume_without_starting_or_completing_excluded_units() {
    let scratch = Scratch::new().unwrap();
    let rungs = RungScope::new(&["3min", "60min"]).unwrap();
    let open_selected = || {
        Writer::open_for_rungs(
            &scratch.0,
            DESCRIPTOR,
            units(),
            rungs,
            LIMIT,
            17,
            |index, _, received| {
                assert!(rungs.contains(index));
                assert_eq!(received, link(index));
                Ok(())
            },
        )
    };
    let mut writer = open_selected().unwrap();
    let id = writer.identity();
    assert_ne!(id, identity(DESCRIPTOR, units()));
    assert!(!writer.completed());
    assert!(writer.begin(0).is_err());
    assert!(
        writer.begin(7).is_err(),
        "selected predecessor is unfinished"
    );
    writer.begin(2).unwrap();
    writer.finish(2, link(2)).unwrap();
    let completed_pin = writer.pin().unwrap();
    drop(writer);
    writer = open_selected().unwrap();
    assert_eq!(writer.pin().unwrap(), completed_pin);
    assert_eq!(writer.slots()[2].complete, Some(link(2)));
    writer.begin(7).unwrap();
    writer.refuse(7, "generated retryable failure").unwrap();
    drop(writer);
    writer = open_selected().unwrap();
    assert!(!writer.completed());
    writer.begin(7).unwrap();
    writer.finish(7, link(7)).unwrap();
    assert!(writer.completed());
    let pin = writer.pin().unwrap();
    drop(writer);
    let reader = Reader::open(&scratch.0, id, LIMIT * 4).unwrap();
    assert_eq!(reader.rungs(), rungs);
    assert_eq!(reader.pin(), pin);
    for (index, slot) in reader.slots().iter().enumerate() {
        if rungs.contains(index) {
            assert_eq!(slot.complete, Some(link(index)));
        } else {
            assert!(!slot.started);
            assert_eq!(slot.complete, None);
            assert!(slot.reason.is_empty());
        }
    }
    let saved = reader.slots().clone();
    let mut forged = saved.clone();
    forged[0].started = true;
    assert!(transition_for_rungs(&saved, &forged, rungs).is_err());
    let final_writer = open_selected().unwrap();
    assert!(final_writer.completed());
    assert_eq!(final_writer.pin().unwrap(), pin);
}

#[test]
fn failed_begin_finish_and_refusal_keep_old_slots_and_poison_the_uncertain_writer() {
    for operation in 0..3 {
        let scratch = Scratch::new().unwrap();
        let mut writer = open(&scratch.0, 100).unwrap();
        if operation != 0 {
            writer.begin(0).unwrap();
        }
        let old = writer.slots().clone();
        let acknowledged = writer.journal.acknowledged();
        let reserved = directory(&scratch.0, writer.identity())
            .join(format!("{:016x}", writer.journal.next_sequence()));
        fs::create_dir(reserved).unwrap();
        let result = match operation {
            0 => writer.begin(0),
            1 => writer.finish(0, link(0)),
            _ => writer.refuse(0, "exact injected publication collision"),
        };
        assert!(result.is_err());
        assert_eq!(writer.slots(), &old);
        assert_eq!(writer.journal.acknowledged(), acknowledged);
        assert!(!writer.completed());
        assert!(writer.pin().is_err());
        assert!(writer.begin(0).is_err());
        assert!(writer.finish(0, link(0)).is_err());
        assert!(writer.refuse(0, "still refused").is_err());
        drop(writer);
        let writer = open(&scratch.0, 100).unwrap();
        assert_eq!(writer.slots(), &old);
        assert_eq!(writer.journal.interrupted(), 1);
        assert_eq!(writer.journal.acknowledged(), acknowledged);
    }
}

#[test]
fn uncertain_readback_never_exposes_unacknowledged_completion_and_reopen_checks_the_child() {
    for fault in 0..3 {
        let scratch = Scratch::new().unwrap();
        let mut writer = open(&scratch.0, 100).unwrap();
        for index in 0..7 {
            writer.begin(index).unwrap();
            writer.finish(index, link(index)).unwrap();
        }
        writer.begin(7).unwrap();
        let old = writer.slots().clone();
        let mut next = old.clone();
        next.last_mut().unwrap().complete = Some(link(7));
        let result = writer.publish_observed(&next, |journal, sequence, bytes| {
            let mut saved = journal.read(sequence, bytes)?;
            match fault {
                0 => return Err("injected readback failure after durable publication".into()),
                1 => saved.seal = [99; 32],
                _ => *saved.payload.last_mut().unwrap() ^= 1,
            }
            Ok(saved)
        });
        assert!(result.is_err());
        assert_eq!(writer.slots(), &old);
        assert!(!writer.completed());
        assert!(writer.pin().is_err());
        drop(writer);
        assert!(
            Writer::open(&scratch.0, DESCRIPTOR, units(), LIMIT, 100, |_, _, _| Err(
                "child authority missing".into()
            ))
            .is_err()
        );
        let restored = open(&scratch.0, 100).unwrap();
        assert!(restored.completed());
        assert_eq!(restored.slots(), &next);
    }
}

#[test]
fn exact_checkpoint_cap_counts_crash_holes_but_not_as_acknowledged_records() {
    let scratch = Scratch::new().unwrap();
    let mut writer = open(&scratch.0, 4).unwrap();
    writer.begin(0).unwrap();
    fs::create_dir(directory(&scratch.0, writer.identity()).join("0000000000000003")).unwrap();
    assert!(writer.finish(0, link(0)).is_err());
    drop(writer);
    let mut writer = open(&scratch.0, 4).unwrap();
    assert_eq!(writer.journal.next_sequence(), 4);
    assert_eq!(writer.journal.interrupted(), 1);
    assert_eq!(writer.journal.acknowledged(), 2);
    writer.begin(0).unwrap();
    assert_eq!(writer.journal.next_sequence(), 4);
    writer.finish(0, link(0)).unwrap();
    assert_eq!(writer.journal.acknowledged(), 3);
    let state = writer.slots().clone();
    let pin = writer.pin().unwrap();
    assert!(writer.begin(1).is_err());
    assert_eq!(writer.slots(), &state);
    assert_eq!(writer.pin().unwrap(), pin);
    drop(writer);
    assert_eq!(open(&scratch.0, 4).unwrap().slots(), &state);
}

#[test]
fn validly_resealed_latest_cannot_omit_acknowledged_intermediate_history() {
    let scratch = Scratch::new().unwrap();
    let mut writer = open(&scratch.0, 100).unwrap();
    let initial = writer.previous;
    let empty = writer.slots().clone();
    writer.begin(0).unwrap();
    writer.finish(0, link(0)).unwrap();
    let body = encode(writer.identity(), DESCRIPTOR, initial, &empty).unwrap();
    writer.journal.publish(&body, LIMIT).unwrap();
    assert!(Reader::open(&scratch.0, writer.identity(), LIMIT).is_err());
    drop(writer);
    assert_eq!(
        open(&scratch.0, 100).err().unwrap(),
        "qualified campaign chain omits acknowledged checkpoints"
    );
}

#[test]
fn read_only_overview_observes_owner_and_all_eight_slots_without_opening_child_bodies() {
    let scratch = Scratch::new().unwrap();
    let missing = scratch.0.join("does-not-exist");
    assert!(Reader::open(&missing, identity(DESCRIPTOR, units()), LIMIT).is_err());
    assert!(!missing.exists());
    let mut writer = open(&scratch.0, 100).unwrap();
    for index in 0..8 {
        writer.begin(index).unwrap();
        writer.finish(index, link(index)).unwrap();
    }
    let reader = Reader::open(&scratch.0, writer.identity(), 256 * 1024).unwrap();
    assert!(reader.owner_observed());
    assert_eq!(reader.identity(), writer.identity());
    assert_eq!(reader.descriptor(), DESCRIPTOR);
    assert_eq!(reader.pin(), writer.pin().unwrap());
    assert_eq!(reader.sequence(), 17);
    assert_eq!(reader.history_records(), 17);
    assert_eq!(reader.slots(), writer.slots());
    let exact = reader.admitted_bytes();
    assert!(Reader::open(&scratch.0, writer.identity(), exact).is_ok());
    assert!(Reader::open(&scratch.0, writer.identity(), exact - 1).is_err());
    drop(writer);
    assert!(
        !Reader::open(&scratch.0, reader.identity(), exact)
            .unwrap()
            .owner_observed()
    );
    reader.require_current().unwrap();
}

#[test]
fn overview_keeps_recorded_refusal_and_refuses_changed_pinned_history() {
    let scratch = Scratch::new().unwrap();
    let mut writer = open(&scratch.0, 100).unwrap();
    writer.begin(0).unwrap();
    writer.refuse(0, "actual saved refusal").unwrap();
    let reader = Reader::open(&scratch.0, writer.identity(), LIMIT).unwrap();
    assert_eq!(
        reader.slots().first().unwrap().reason,
        "actual saved refusal"
    );
    assert!(
        reader
            .slots()
            .iter()
            .skip(1)
            .all(|s| !s.started && s.complete.is_none())
    );
    writer.begin(0).unwrap();
    reader.require_current().unwrap();
    assert_ne!(writer.pin().unwrap(), reader.pin());
    assert_eq!(
        reader.slots().first().unwrap().reason,
        "actual saved refusal"
    );
    fs::write(
        directory(&scratch.0, writer.identity()).join("0000000000000001/payload"),
        b"changed",
    )
    .unwrap();
    assert!(reader.require_current().is_err());
    assert!(Reader::open(&scratch.0, writer.identity(), LIMIT).is_err());
}

#[test]
fn resealed_state_cannot_skip_begin_retract_completion_or_crosswire_units() {
    for fault in 0..7 {
        let scratch = Scratch::new().unwrap();
        let mut writer = open(&scratch.0, 100).unwrap();
        if (3..6).contains(&fault) {
            writer.begin(0).unwrap();
            writer.finish(0, link(0)).unwrap();
        }
        let mut slots = writer.slots().clone();
        match fault {
            0 => slots.get_mut(1).unwrap().started = true,
            1 => {
                let first = slots.first_mut().unwrap();
                first.started = true;
                first.complete = Some(link(0));
            }
            2 => {
                slots.first_mut().unwrap().started = true;
                slots.get_mut(1).unwrap().started = true;
            }
            3 => slots.first_mut().unwrap().complete = None,
            4 => slots.first_mut().unwrap().complete = Some(link(1)),
            5 => slots.first_mut().unwrap().unit = [123; 32],
            _ => {
                let first = slots.first_mut().unwrap();
                first.started = true;
                first.reason = "no separately acknowledged start".into();
            }
        }
        let raw = encode(writer.identity(), DESCRIPTOR, writer.previous, &slots).unwrap();
        writer.journal.publish(&raw, LIMIT).unwrap();
        drop(writer);
        assert!(open(&scratch.0, 100).is_err(), "fault {fault}");
    }
}

#[test]
fn scope_initial_record_foreign_identity_and_partial_completion_refuse() {
    let scratch = Scratch::new().unwrap();
    let mut duplicate = units();
    *duplicate.last_mut().unwrap() = *duplicate.first().unwrap();
    assert!(
        Writer::open(&scratch.0, DESCRIPTOR, duplicate, LIMIT, 100, |_, _, _| Ok(
            ()
        ))
        .is_err()
    );
    assert!(Writer::open(&scratch.0, [0; 32], units(), LIMIT, 100, |_, _, _| Ok(())).is_err());
    assert!(
        Writer::open(
            &scratch.0,
            DESCRIPTOR,
            units(),
            u64::try_from(BYTES + 95).unwrap(),
            100,
            |_, _, _| Ok(())
        )
        .is_err()
    );
    assert!(open(&scratch.0, 2).is_err());
    let mut writer = open(&scratch.0, 100).unwrap();
    assert!(writer.finish(0, link(0)).is_err());
    assert!(writer.begin(8).is_err());
    writer.begin(0).unwrap();
    assert!(
        writer
            .finish(
                0,
                Link {
                    identity: [0; 32],
                    pin: [1; 32]
                }
            )
            .is_err()
    );
    assert!(
        writer
            .finish(
                0,
                Link {
                    identity: [1; 32],
                    pin: [0; 32]
                }
            )
            .is_err()
    );
    let raw = encode([37; 32], DESCRIPTOR, writer.previous, writer.slots()).unwrap();
    writer.journal.publish(&raw, LIMIT).unwrap();
    drop(writer);
    assert_eq!(
        open(&scratch.0, 100).err().unwrap(),
        "qualified campaign manifest differs"
    );
    let scratch = Scratch::new().unwrap();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity(DESCRIPTOR, units())).unwrap();
    let mut slots = units().map(|unit| Slot {
        unit,
        started: false,
        complete: None,
        reason: String::new(),
    });
    slots.first_mut().unwrap().started = true;
    journal
        .publish(
            &encode(identity(DESCRIPTOR, units()), DESCRIPTOR, None, &slots).unwrap(),
            LIMIT,
        )
        .unwrap();
    drop(journal);
    assert_eq!(
        open(&scratch.0, 100).err().unwrap(),
        "qualified campaign initial scope was not acknowledged before work"
    );
}

#[test]
fn utf8_refusal_is_bounded_retained_and_retry_clears_only_the_pending_reason() {
    assert_eq!(bounded_reason(&"é".repeat(512)), "é".repeat(512));
    let long = "界".repeat(800);
    let expected = bounded_reason(&long);
    assert!(expected.len() <= REASON);
    assert!(expected.ends_with(" [truncated]"));
    assert!(long.starts_with(expected.strip_suffix(" [truncated]").unwrap()));
    let scratch = Scratch::new().unwrap();
    let mut writer = open(&scratch.0, 100).unwrap();
    writer.begin(0).unwrap();
    let pin = writer.pin().unwrap();
    assert!(writer.refuse(0, " \n\t").is_err());
    assert_eq!(writer.pin().unwrap(), pin);
    writer.refuse(0, &long).unwrap();
    assert_eq!(writer.slots().first().unwrap().reason, expected);
    drop(writer);
    let mut writer = open(&scratch.0, 100).unwrap();
    assert_eq!(writer.slots().first().unwrap().reason, expected);
    writer.begin(0).unwrap();
    assert!(writer.slots().first().unwrap().reason.is_empty());
    writer.finish(0, link(0)).unwrap();
    assert_eq!(writer.slots().first().unwrap().complete, Some(link(0)));
}
