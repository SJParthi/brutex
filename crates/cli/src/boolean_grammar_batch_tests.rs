#![expect(
    clippy::unwrap_used,
    reason = "exact finite grammar and corruption assertions"
)]
use super::*;

fn budget(programs: u64, nodes: u64) -> Budget {
    Budget {
        programs,
        nodes,
        bytes: 1024 * 1024,
    }
}

#[test]
fn bounded_batches_resume_without_skipping_reordering_or_false_exhaustion() {
    let start = Cursor::new(&[30, 31]).unwrap();
    let whole = Batch::prepare(start.clone(), 0, 0, budget(100, 100_000)).unwrap();
    let mut cursor = start;
    let mut work = 0;
    let mut count = 0;
    let mut observed = Vec::new();
    for _ in 0..20 {
        let batch = Batch::prepare(cursor, work, count, budget(5, 100_000)).unwrap();
        assert!(!batch.exhausted());
        observed.extend(batch.programs().iter().map(Expression::encode));
        cursor = batch.next_cursor();
        work = batch.work();
        count = batch.cumulative_programs().unwrap();
    }
    assert_eq!(
        observed,
        whole
            .programs()
            .iter()
            .map(Expression::encode)
            .collect::<Vec<_>>()
    );
    assert_eq!(cursor.encode(), whole.next_cursor().encode());
    assert_eq!(work, whole.work());
    assert_eq!(count, 100);
}

#[test]
fn binary_batch_replay_detects_changed_counters_programs_and_padding() {
    let batch = Batch::prepare(Cursor::new(&[30, 31]).unwrap(), 0, 0, budget(12, 1000)).unwrap();
    let raw = batch.encode().unwrap();
    assert_eq!(
        Batch::decode(&raw, 1024 * 1024, 1000)
            .unwrap()
            .encode()
            .unwrap(),
        raw
    );
    for at in [
        0,
        8,
        16,
        32,
        40,
        48,
        56,
        57,
        64,
        64 + CURSOR_BYTES,
        HEADER,
        raw.len() - 1,
    ] {
        let mut changed = raw.clone();
        *changed.get_mut(at).unwrap() ^= 1;
        assert!(
            Batch::decode(&changed, 1024 * 1024, 1000).is_err(),
            "offset {at}"
        );
    }
    assert!(Batch::decode(&raw, raw.len() as u64 - 1, 1000).is_err());
    assert!(Batch::decode(&raw, 1024 * 1024, 999).is_err());
    assert!(Batch::decode(raw.get(..raw.len() - 1).unwrap(), 1024 * 1024, 1000).is_err());
}

#[test]
fn node_only_progress_remains_paused_and_all_arithmetic_is_checked() {
    let start = Cursor::new(&[30]).unwrap();
    let first = Batch::prepare(start, 0, 0, budget(5, 1)).unwrap();
    assert_eq!(first.programs().len(), 1);
    assert!(!first.exhausted());
    let second = Batch::prepare(first.next_cursor(), first.work(), 1, budget(5, 1)).unwrap();
    assert!(second.programs().is_empty());
    assert_eq!(second.work(), 2);
    assert!(!second.exhausted());
    assert!(Batch::prepare(Cursor::new(&[30]).unwrap(), u64::MAX, 0, budget(1, 1)).is_err());
    assert!(Batch::prepare(Cursor::new(&[30]).unwrap(), 0, u64::MAX, budget(1, 1)).is_err());
    for invalid in [
        budget(0, 1),
        budget(1, 0),
        budget(u64::MAX, 1),
        Budget {
            bytes: 1,
            ..budget(1, 1)
        },
    ] {
        assert!(Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, invalid).is_err());
    }
}

#[test]
fn only_the_cursor_terminal_state_can_report_fixed_grammar_exhaustion() {
    let mut raw = Cursor::new(&[30]).unwrap().encode();
    raw.get_mut(10..12).unwrap().copy_from_slice(
        &u16::try_from(runner::expression::MAX_INSTRUCTIONS)
            .unwrap()
            .to_le_bytes(),
    );
    *raw.get_mut(14).unwrap() = 1;
    let terminal = Cursor::decode(&raw).unwrap();
    let batch = Batch::prepare(terminal, 42, 7, budget(2, 100)).unwrap();
    assert!(batch.exhausted());
    assert!(batch.programs().is_empty());
    assert_eq!(batch.work(), 42);
    assert_eq!(batch.cumulative_programs().unwrap(), 7);
    assert!(
        Batch::decode(&batch.encode().unwrap(), 1024 * 1024, 100)
            .unwrap()
            .exhausted()
    );
}

#[test]
fn exact_binary_admission_accepts_equality_and_retains_every_program() {
    let bytes = u64::try_from(HEADER + 3 * ENCODED_LEN).unwrap();
    let allowed = Budget {
        programs: 3,
        nodes: 100,
        bytes,
    };
    let initial = Cursor::new(&[30, 31]).unwrap();
    let batch = Batch::prepare(initial.clone(), 11, 9, allowed).unwrap();
    assert_eq!(batch.programs().len(), 3);
    assert_eq!(batch.cumulative_programs().unwrap(), 12);
    let raw = batch.encode().unwrap();
    assert_eq!(u64::try_from(raw.len()).unwrap(), bytes);
    assert_eq!(
        Batch::decode(&raw, bytes, 100).unwrap().encode().unwrap(),
        raw
    );
    assert!(
        Batch::prepare(
            initial,
            11,
            9,
            Budget {
                bytes: bytes - 1,
                ..allowed
            }
        )
        .is_err()
    );
    assert!(Batch::decode(&raw, bytes - 1, 100).is_err());
}

#[test]
fn progress_and_budget_links_require_each_exact_component() {
    let initial = Cursor::new(&[30, 31]).unwrap();
    let first = Batch::prepare(initial.clone(), 0, 0, budget(17, 1000)).unwrap();
    let before = first.next_cursor();
    let canonical = Cursor::decode(&before.encode()).unwrap();
    let next = Batch::prepare(before, first.work(), 17, budget(3, 1000)).unwrap();
    assert!(next.follows(&canonical, first.work(), 17));
    assert!(!next.follows(&initial, first.work(), 17));
    assert!(!next.follows(&canonical, first.work() + 1, 17));
    assert!(!next.follows(&canonical, first.work(), 18));
    for (programs, nodes, expected) in [
        (3, 1000, true),
        (2, 1000, false),
        (3, 999, false),
        (2, 999, false),
    ] {
        assert_eq!(next.uses_budget(programs, nodes), expected);
    }
    assert_eq!(next.cumulative_programs().unwrap(), 20);
    assert!(next.work() > first.work());
}

#[test]
fn malformed_header_fields_and_impossible_wire_counts_refuse_without_partial_decode() {
    let batch = Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, budget(2, 100)).unwrap();
    let raw = batch.encode().unwrap();
    for length in 0..64 {
        assert!(
            Batch::decode(raw.get(..length).unwrap(), BYTES, 100).is_err(),
            "length {length}"
        );
    }
    for count in [u64::MAX, 3] {
        let mut changed = raw.clone();
        changed
            .get_mut(48..56)
            .unwrap()
            .copy_from_slice(&count.to_le_bytes());
        assert!(Batch::decode(&changed, BYTES, 100).is_err());
    }
    let mut no_work = raw.clone();
    no_work
        .get_mut(40..48)
        .unwrap()
        .copy_from_slice(&0_u64.to_le_bytes());
    assert_eq!(
        Batch::decode(&no_work, BYTES, 100).err().unwrap(),
        "grammar program and node work allowances must be positive"
    );
    assert_eq!(field::<0>(&raw, raw.len()).unwrap(), [0_u8; 0]);
    assert_eq!(field::<8>(&raw, 0).unwrap(), *MAGIC);
    assert_eq!(
        field::<8>(&raw, usize::MAX).unwrap_err(),
        "grammar batch field is missing"
    );
    assert!(field::<CURSOR_BYTES>(&raw, raw.len() - 1).is_err());
}

const BYTES: u64 = 1024 * 1024;

#[test]
fn replay_preserves_claimed_origin_but_cannot_authenticate_its_predecessor() {
    let cursor = Cursor::new(&[30]).unwrap();
    let batch = Batch::prepare(cursor.clone(), 0, 7, budget(2, 100)).unwrap();
    let raw = batch.encode().unwrap();
    let replayed = Batch::decode(&raw, BYTES, 100).unwrap();
    assert!(replayed.follows(&cursor, 0, 7));
    assert!(!replayed.follows(&cursor, 0, 0));
    assert_eq!(replayed.cumulative_programs().unwrap(), 9);
    let mut broken = replayed;
    broken.programs_before = u64::MAX;
    assert_eq!(
        broken.cumulative_programs().unwrap_err(),
        "grammar cumulative program count overflow"
    );
}

#[test]
fn allocation_size_refusal_and_cursor_errors_retain_their_reasons() {
    let programs =
        u64::try_from(isize::MAX as usize / std::mem::size_of::<Expression>() + 1).unwrap();
    let refused = Batch::prepare(
        Cursor::new(&[30]).unwrap(),
        0,
        0,
        Budget {
            programs,
            nodes: 1,
            bytes: u64::MAX,
        },
    );
    let reason = refused.err().unwrap();
    assert!(reason.contains("capacity"), "{reason}");
    let overflow = Batch::prepare(Cursor::new(&[30]).unwrap(), u64::MAX, 0, budget(1, 1));
    assert_eq!(overflow.err().unwrap(), "WorkOverflow");
}

fn malformed_before_cursor() -> Vec<u8> {
    let batch = Batch::prepare(Cursor::new(&[30]).unwrap(), 0, 0, budget(2, 100)).unwrap();
    assert_eq!(batch.programs().len(), 2);
    let mut raw = batch.encode().unwrap();
    assert_eq!(
        Batch::decode(&raw, BYTES, 100).unwrap().encode().unwrap(),
        raw
    );
    // The before-cursor's version byte is deliberately wrong. Valid outer
    // framing reaches this error; malformed framing must refuse earlier.
    *raw.get_mut(64).unwrap() ^= 1;
    assert_eq!(Batch::decode(&raw, BYTES, 100).err().unwrap(), "Cursor");
    raw
}

#[test]
fn batch_format_and_byte_admission_precede_cursor_parsing() {
    let raw = malformed_before_cursor();
    let mut wrong_version = raw.clone();
    *wrong_version.first_mut().unwrap() ^= 1;
    let named = "grammar batch format or byte admission refused";
    assert_eq!(
        Batch::decode(&wrong_version, wrong_version.len() as u64, 100)
            .err()
            .unwrap(),
        named,
        "wrong batch magic must refuse even when every byte fits"
    );
    assert_eq!(
        Batch::decode(&raw, raw.len() as u64 - 1, 100)
            .err()
            .unwrap(),
        named,
        "correct batch magic cannot bypass the physical byte ceiling"
    );
    assert_eq!(
        Batch::decode(raw.get(..7).unwrap(), 6, 100).err().unwrap(),
        named,
        "oversized short input is refused before reading even its version field"
    );
}

#[test]
fn batch_count_and_exact_length_admission_precede_cursor_parsing() {
    let raw = malformed_before_cursor();
    let named = "grammar batch count or replay work admission refused";
    let mut excessive_count = raw.clone();
    excessive_count
        .get_mut(32..40)
        .unwrap()
        .copy_from_slice(&1_u64.to_le_bytes());
    assert_eq!(number(&excessive_count, 48).unwrap(), 2);
    assert_eq!(excessive_count.len(), HEADER + 2 * ENCODED_LEN);
    assert_eq!(
        Batch::decode(&excessive_count, BYTES, 100).err().unwrap(),
        named,
        "count over program allowance refuses even when encoded length is exact"
    );
    let mut appended = raw.clone();
    appended.push(0);
    assert_eq!(
        Batch::decode(&appended, BYTES, 100).err().unwrap(),
        named,
        "extra bytes refuse even when count is within its program allowance"
    );
    let mut impossible_length = raw.clone();
    for offset in [32, 48] {
        impossible_length
            .get_mut(offset..offset + 8)
            .unwrap()
            .copy_from_slice(&u64::MAX.to_le_bytes());
    }
    assert_eq!(
        Batch::decode(&impossible_length, BYTES, 100).err().unwrap(),
        named,
        "unrepresentable exact wire length refuses before cursor replay or allocation"
    );
    assert_eq!(
        Batch::decode(&raw, BYTES, 99).err().unwrap(),
        named,
        "node admission is unconditional, independently of count and length"
    );
}
