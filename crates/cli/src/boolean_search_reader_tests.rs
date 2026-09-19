#![cfg(test)]
//! Real journal operations over generated payloads, without market/source authority.
#![expect(clippy::unwrap_used, reason = "bounded generated journal assertions")]
use super::*;
use crate::boolean_search_record::tests::{
    complete_generated_record, completed_record, decode_bytes, generated_plan,
    generated_plan_with_limits, generated_record,
};
use crate::search_checkpoint::{Journal, tests::Scratch};
use brutex_core::blake3::hash;

const OBSERVE: u64 = 4 * 1024 * 1024;

fn publish(journal: &mut Journal, record: &mut Record) -> (u64, [u8; 32]) {
    record.previous = journal
        .latest(record.spec.bytes)
        .unwrap()
        .map(|s| (s.sequence, s.seal));
    journal
        .publish(&record.encode().unwrap(), record.spec.bytes)
        .unwrap()
}

#[test]
fn journal_reopens_pending_refused_and_complete_observations_without_inventing_child_authority() {
    let scratch = Scratch::new().unwrap();
    let mut record = generated_record();
    let identity = record.spec.identity();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    let first = publish(&mut journal, &mut record);
    let pending = Reader::open(&scratch.0, identity, OBSERVE, 100).unwrap();
    let planned = Some(completed_record().campaign.identity);
    assert_eq!(pending.planned_campaign().unwrap(), planned);
    assert_eq!(
        (pending.identity(), pending.pin(), pending.sequence()),
        (identity, first.1, first.0)
    );
    assert_eq!(
        (
            pending.batch(),
            pending.phase(),
            pending.completed_batches()
        ),
        (0, 0, 0)
    );
    assert_eq!(pending.programs().unwrap(), 2);
    assert_eq!(pending.work(), record.batch.work());
    assert_eq!(pending.alpha_ppm(), 50_000);
    assert_eq!(pending.replay_nodes(), 100);
    assert_eq!(pending.summaries(), &[Summary::EMPTY; 8]);
    assert!(!pending.exhausted());
    assert!(pending.owner_observed());
    assert!(pending.verify_batch(0).is_err());

    record.phase = 2;
    record.reason = "generated child refused before completion".into();
    publish(&mut journal, &mut record);
    pending.require_current().unwrap();
    assert_eq!(
        pending.phase(),
        0,
        "appends do not silently replace a pinned observation"
    );
    let refused = Reader::open(&scratch.0, identity, OBSERVE, 200).unwrap();
    assert_eq!((refused.phase(), refused.completed_batches()), (2, 0));
    assert_eq!(refused.reason(), record.reason);
    assert_eq!(refused.planned_campaign().unwrap(), planned);
    assert_eq!(refused.replay_nodes(), 200);

    let mut complete = completed_record();
    let final_pin = publish(&mut journal, &mut complete);
    let final_view = Reader::open(&scratch.0, identity, OBSERVE, 400).unwrap();
    assert_eq!(
        (
            final_view.phase(),
            final_view.completed_batches(),
            final_view.sequence()
        ),
        (1, 1, 3)
    );
    assert_eq!(final_view.pin(), final_pin.1);
    assert_eq!(final_view.planned_campaign().unwrap(), planned);
    assert_eq!(final_view.summaries(), &complete.summaries);
    assert!(!final_view.exhausted());
    assert_eq!(
        final_view.verify_batch(0).unwrap_err(),
        "qualified campaign not recorded"
    );
    assert_eq!(
        final_view.rung(final_view.pin(), 0, 0).err().unwrap(),
        "qualified campaign not recorded"
    );
    assert_eq!(
        final_view.rung(first.1, 0, 0).err().unwrap(),
        "search checkpoint pin differs; no replacement page returned"
    );
    assert!(final_view.verify_batch(1).is_err());
    drop(journal);
    let reopened = Reader::open(&scratch.0, identity, OBSERVE, 300).unwrap();
    assert!(!reopened.owner_observed());
    assert_eq!(reopened.pin(), final_pin.1);
}

#[test]
fn node_only_continuation_exposes_no_invented_campaign_address() {
    use crate::boolean_grammar_batch::Batch;
    use vocab::expression_search::Cursor;

    let scratch = Scratch::new().unwrap();
    let mut record = generated_record();
    record.spec.nodes = 1;
    record.spec.initial = Cursor::new(&[30]).unwrap().initial_descriptor();
    record.batch =
        Batch::prepare(record.spec.cursor().unwrap(), 0, 0, record.spec.budget()).unwrap();
    record.plan = generated_plan(record.batch.programs());
    let identity = record.spec.identity();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    publish(&mut journal, &mut record);
    record = complete_generated_record(record);
    publish(&mut journal, &mut record);
    record.batch = Batch::prepare(
        record.batch.next_cursor(),
        record.batch.work(),
        record.batch.cumulative_programs().unwrap(),
        record.spec.budget(),
    )
    .unwrap();
    assert!(record.batch.programs().is_empty());
    record.ordinal = 1;
    record.plan.clear();
    record.campaign = Summary::EMPTY.child;
    record.summaries = [Summary::EMPTY; 8];
    for (phase, nodes) in [(0, 3), (1, 4)] {
        record.phase = phase;
        publish(&mut journal, &mut record);
        let reader = Reader::open(&scratch.0, identity, OBSERVE, nodes).unwrap();
        assert_eq!((reader.batch(), reader.phase()), (1, phase));
        assert_eq!(reader.planned_campaign().unwrap(), None);
        assert_eq!(reader.summaries(), &[Summary::EMPTY; 8]);
        if phase == 1 {
            assert_eq!(
                open_rung(&scratch.0, &record, 0, OBSERVE).err().unwrap(),
                "search batch has no completed qualification rows"
            );
            assert_eq!(
                reader.verify_batch(1).unwrap_err(),
                "search grammar replay exceeds independent node admission"
            );
            let detail = Reader::open(&scratch.0, identity, OBSERVE, nodes + 1).unwrap();
            detail.verify_batch(1).unwrap();
            assert_eq!(reader.completed_batches(), 2);
        }
    }
}

#[test]
fn foreign_requested_identity_is_rejected_before_a_malformed_grammar_is_replayed() {
    let scratch = Scratch::new().unwrap();
    let record = generated_record();
    let foreign = hash(b"generated foreign search declaration");
    let mut raw = record.encode().unwrap();
    let batch_at = raw.len() - record.plan.len() - record.batch.encode().unwrap().len();
    *raw.get_mut(batch_at).unwrap() ^= 1;
    assert!(
        Record::decode(&raw, record.spec.bytes).is_err(),
        "inner malformed grammar is a real fault"
    );
    let mut journal = Journal::open(&scratch.0, NAMESPACE, foreign).unwrap();
    journal.publish(&raw, record.spec.bytes).unwrap();
    assert_eq!(
        Reader::open(&scratch.0, foreign, OBSERVE, 1).err().unwrap(),
        "search declaration identity or history extent differs"
    );
}

#[test]
fn exact_complete_history_bytes_and_replay_allowances_are_not_per_record_fallbacks() {
    let scratch = Scratch::new().unwrap();
    let mut record = generated_record();
    let identity = record.spec.identity();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    publish(&mut journal, &mut record);
    let one = Reader::open(&scratch.0, identity, OBSERVE, 100).unwrap();
    let decoding = decode_bytes(&record, &record.encode().unwrap());
    let bytes = (one.admitted_bytes() * 2).max(decoding);
    assert_eq!(
        Reader::open(&scratch.0, identity, bytes, 100)
            .unwrap()
            .admitted_bytes(),
        one.admitted_bytes()
    );
    assert!(Reader::open(&scratch.0, identity, OBSERVE, 99).is_err());
    record.phase = 2;
    record.reason = "generated retry remains in the same slot".into();
    publish(&mut journal, &mut record);
    assert!(Reader::open(&scratch.0, identity, OBSERVE, 199).is_err());
    let two = Reader::open(&scratch.0, identity, OBSERVE, 200).unwrap();
    assert_eq!(two.replay_nodes(), 200);
    let history_bytes = two.admitted_bytes() * 2;
    assert!(
        history_bytes >= decoding,
        "this two-record fixture isolates history admission from decode buffers"
    );
    assert_eq!(
        Reader::open(&scratch.0, identity, history_bytes, 200)
            .unwrap()
            .admitted_bytes(),
        two.admitted_bytes()
    );
    assert_eq!(
        Reader::open(&scratch.0, identity, history_bytes - 1, 200)
            .err()
            .unwrap(),
        "qualified search complete history exceeds byte admission"
    );
    assert_eq!(
        Reader::open(&scratch.0, identity, bytes, 200)
            .err()
            .unwrap(),
        "qualified search complete history exceeds byte admission"
    );
}

#[test]
fn skipping_an_acknowledged_record_or_replacing_its_pin_refuses_the_entire_history() {
    for wrong_pin in [false, true] {
        let scratch = Scratch::new().unwrap();
        let mut record = generated_record();
        let identity = record.spec.identity();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
        let first = publish(&mut journal, &mut record);
        record.phase = 2;
        record.reason = "generated first refusal".into();
        publish(&mut journal, &mut record);
        record.reason = "generated second refusal".into();
        record.previous = Some(if wrong_pin {
            (2, hash(b"generated foreign predecessor pin"))
        } else {
            first
        });
        journal
            .publish(&record.encode().unwrap(), record.spec.bytes)
            .unwrap();
        assert_eq!(
            Reader::open(&scratch.0, identity, OBSERVE, 300)
                .err()
                .unwrap(),
            if wrong_pin {
                "search predecessor pin differs"
            } else {
                "search chain omits acknowledged history"
            }
        );
    }
}

#[test]
fn lost_or_corrupt_latest_acknowledgement_never_falls_back_to_an_older_snapshot() {
    for remove in [false, true] {
        let scratch = Scratch::new().unwrap();
        let mut record = generated_record();
        let identity = record.spec.identity();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
        publish(&mut journal, &mut record);
        record.phase = 2;
        record.reason = "generated refused latest attempt".into();
        let second = publish(&mut journal, &mut record);
        let reader = Reader::open(&scratch.0, identity, OBSERVE, 200).unwrap();
        let path = scratch
            .0
            .join(NAMESPACE)
            .join(crate::identity_hex(&identity))
            .join(format!("{:016x}", second.0))
            .join("payload");
        if remove {
            std::fs::remove_file(path).unwrap();
        } else {
            let mut bytes = std::fs::read(&path).unwrap();
            *bytes.last_mut().unwrap() ^= 1;
            std::fs::write(path, bytes).unwrap();
        }
        assert!(reader.require_current().is_err());
        assert!(Reader::open(&scratch.0, identity, OBSERVE, 200).is_err());
    }
}

#[test]
fn unknown_empty_and_zero_replay_requests_have_distinct_explicit_refusals() {
    let scratch = Scratch::new().unwrap();
    let identity = generated_record().spec.identity();
    assert_eq!(
        Reader::open(&scratch.0, identity, OBSERVE, 0)
            .err()
            .unwrap(),
        "search replay node admission must be positive"
    );
    assert_eq!(
        Reader::open(&scratch.0, identity, OBSERVE, 100)
            .err()
            .unwrap(),
        "qualified search is not recorded"
    );
    let _journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    assert_eq!(
        Reader::open(&scratch.0, identity, OBSERVE, 100)
            .err()
            .unwrap(),
        "qualified search has no acknowledged reservation"
    );
}

#[test]
fn self_or_forward_predecessor_refuses_before_opening_a_foreign_checkpoint() {
    for sequence in [1, 2, u64::MAX] {
        let scratch = Scratch::new().unwrap();
        let mut record = generated_record();
        record.previous = Some((sequence, hash(b"generated predecessor")));
        let identity = record.spec.identity();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
        journal
            .publish(&record.encode().unwrap(), record.spec.bytes)
            .unwrap();
        assert_eq!(
            Reader::open(&scratch.0, identity, OBSERVE, 100)
                .err()
                .unwrap(),
            "search predecessor is not strictly earlier"
        );
    }
}

#[test]
fn declared_record_allowance_counts_every_acknowledged_retry() {
    use crate::boolean_grammar_batch::Batch;
    let scratch = Scratch::new().unwrap();
    let mut record = generated_record();
    record.spec.nodes = 2;
    record.spec.records = 2;
    record.batch =
        Batch::prepare(record.spec.cursor().unwrap(), 0, 0, record.spec.budget()).unwrap();
    record.plan = generated_plan_with_limits(
        record.batch.programs(),
        [record.spec.bytes, record.spec.records],
    );
    let identity = record.spec.identity();
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    publish(&mut journal, &mut record);
    record.phase = 2;
    record.reason = "first acknowledged generated refusal".into();
    publish(&mut journal, &mut record);
    assert_eq!(
        Reader::open(&scratch.0, identity, OBSERVE, 100)
            .unwrap()
            .sequence(),
        2
    );
    record.reason = "extra acknowledged generated refusal".into();
    publish(&mut journal, &mut record);
    assert_eq!(
        Reader::open(&scratch.0, identity, OBSERVE, 100)
            .err()
            .unwrap(),
        "search declaration identity or history extent differs"
    );
}

#[test]
fn replay_cache_corruption_cannot_replace_authenticated_history_or_completed_positions() {
    // These indexes are derived by production replay. Inject cache faults only
    // after authenticating the actual journal: a cached answer is not authority.
    for fault in 0..6 {
        let scratch = Scratch::new().unwrap();
        let mut record = generated_record();
        let identity = record.spec.identity();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
        publish(&mut journal, &mut record);
        let mut complete = completed_record();
        publish(&mut journal, &mut complete);
        let mut reader = Reader::open(&scratch.0, identity, OBSERVE, 400).unwrap();
        match fault {
            0 => {
                reader.history.first_mut().unwrap().seal = hash(b"wrong cached seal");
                assert_eq!(
                    reader.require_current().err().unwrap(),
                    "qualified search history changed"
                );
            }
            1 => {
                reader.history.first_mut().unwrap().payload.push(0);
                assert_eq!(
                    reader.require_current().err().unwrap(),
                    "qualified search history changed"
                );
            }
            2 => {
                *reader.completed.first_mut().unwrap() = usize::MAX;
                assert_eq!(
                    reader.completed_record(0).err().unwrap(),
                    "search completed batch index absent"
                );
            }
            3 => {
                *reader.completed.first_mut().unwrap() = 0;
                assert_eq!(
                    reader.completed_record(0).err().unwrap(),
                    "search completed batch mapping differs"
                );
            }
            4 => {
                reader.max_replay_nodes = reader.replay_nodes - 1;
                assert_eq!(
                    reader.completed_record(0).err().unwrap(),
                    "search detail replay admission exhausted"
                );
            }
            _ => {
                reader.max_bytes = reader.bytes - 1;
                assert_eq!(
                    reader.remaining().err().unwrap(),
                    "search observation admission exhausted"
                );
            }
        }
    }
}

#[test]
fn a_completed_result_cannot_skip_the_next_prework_reservation() {
    let before = completed_record();
    let next = completed_record();
    assert_eq!(
        transition(Some(&before), &next).err().unwrap(),
        "search skipped, repeated or continued exhausted grammar work"
    );
    let mut wrong_initial = generated_record();
    wrong_initial.ordinal = 1;
    assert_eq!(
        transition(None, &wrong_initial).err().unwrap(),
        "search first record is not its initial pre-work reservation"
    );
    let scratch = Scratch::new().unwrap();
    assert_eq!(
        open_rung(&scratch.0, &wrong_initial, 0, OBSERVE)
            .err()
            .unwrap(),
        "search batch has no completed qualification rows"
    );
}

fn waiting_campaign(root: &Path, record: &Record) -> crate::boolean_qualified_journal::Writer {
    let plan = record.observed_plan().unwrap();
    crate::boolean_qualified_journal::Writer::open(
        root,
        plan.descriptor(),
        *plan.units(),
        OBSERVE,
        100,
        |_, _, _| Err("unexpected completed child in generated waiting fixture".into()),
    )
    .unwrap()
}

#[test]
fn a_new_campaign_checkpoint_cannot_replace_the_exact_pin_in_a_completed_search_claim() {
    let scratch = Scratch::new().unwrap();
    let mut record = generated_record();
    let identity = record.spec.identity();
    let mut campaign = waiting_campaign(&scratch.0, &record);
    let old_pin = campaign.pin().unwrap();
    campaign.begin(0).unwrap();
    let new_pin = campaign.pin().unwrap();
    assert_ne!(old_pin, new_pin);

    // The parent declaration is untrusted. The real child journal has only a
    // waiting checkpoint followed by a start, never qualification authority.
    let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
    publish(&mut journal, &mut record);
    let mut claim = complete_generated_record(record);
    claim.campaign.pin = old_pin;
    let parent = publish(&mut journal, &mut claim);
    drop(campaign);
    drop(journal);

    let child = QualifiedCampaign::open(&scratch.0, claim.campaign.identity, OBSERVE).unwrap();
    assert_eq!(child.pin(), new_pin);
    assert!(child.slots().first().unwrap().started);
    assert!(child.slots().iter().all(|slot| slot.complete.is_none()));
    let reader = Reader::open(&scratch.0, identity, OBSERVE, 400).unwrap();
    assert_eq!(reader.pin(), parent.1);
    assert_eq!(
        reader.verify_batch(0).err().unwrap(),
        "search batch campaign pin or plan differs"
    );
    assert_eq!(
        reader.rung(parent.1, 0, 0).err().unwrap(),
        "search detail campaign or declared child mapping differs"
    );
    reader.require_current().unwrap();
    child.require_current().unwrap();
}

#[test]
fn matching_campaign_pin_cannot_promote_waiting_started_or_refused_slots_to_complete() {
    for state in 0..3 {
        let scratch = Scratch::new().unwrap();
        let mut record = generated_record();
        let identity = record.spec.identity();
        let mut campaign = waiting_campaign(&scratch.0, &record);
        if state > 0 {
            campaign.begin(0).unwrap();
        }
        if state == 2 {
            campaign.refuse(0, "generated child refusal").unwrap();
        }
        let actual_pin = campaign.pin().unwrap();
        let mut journal = Journal::open(&scratch.0, NAMESPACE, identity).unwrap();
        publish(&mut journal, &mut record);
        let mut claim = complete_generated_record(record);
        claim.campaign.pin = actual_pin;
        let parent = publish(&mut journal, &mut claim);
        drop(campaign);
        drop(journal);

        let child = QualifiedCampaign::open(&scratch.0, claim.campaign.identity, OBSERVE).unwrap();
        assert_eq!(child.pin(), actual_pin);
        let slot = child.slots().first().unwrap();
        assert_eq!(slot.started, state > 0);
        assert_eq!(slot.reason.is_empty(), state != 2);
        assert!(child.slots().iter().all(|slot| slot.complete.is_none()));
        let reader = Reader::open(&scratch.0, identity, OBSERVE, 400).unwrap();
        assert_eq!(reader.pin(), parent.1);
        assert_eq!(
            reader.verify_batch(0).err().unwrap(),
            "search batch campaign child mapping differs"
        );
        assert_eq!(
            reader.rung(parent.1, 0, 0).err().unwrap(),
            "search detail campaign or declared child mapping differs"
        );
        reader.require_current().unwrap();
        child.require_current().unwrap();
    }
}
