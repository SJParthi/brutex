//! Independent finite grammar enumeration and every-node restart checks.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "assertions and finite independent oracle indexing are test-only"
)]

use std::collections::BTreeSet;
use vocab::expression::{Expression, MAX_INSTRUCTIONS};
use vocab::expression_search::{CURSOR_BYTES, Cursor, Refusal, Step};

fn size(expression: &Expression) -> usize {
    let bytes = expression.encode();
    usize::from(u16::from_le_bytes([bytes[2], bytes[3]]))
}

// An independent recursive INFIX grammar, not the cursor's postfix traversal.
fn reference(live: &[u32], limit: usize) -> BTreeSet<Vec<u8>> {
    let mut levels: Vec<Vec<String>> = vec![Vec::new(); limit + 1];
    levels[1] = live.iter().map(u32::to_string).collect();
    for length in 2..=limit {
        let mut expressions: BTreeSet<String> = levels[length - 1]
            .iter()
            .map(|s| format!("!({s})"))
            .collect();
        for left in 1..length - 1 {
            for a in &levels[left] {
                for b in &levels[length - left - 1] {
                    for op in ["&", "|"] {
                        expressions.insert(format!("({a}){op}({b})"));
                    }
                }
            }
        }
        levels[length] = expressions.into_iter().collect();
    }
    levels
        .into_iter()
        .flatten()
        .map(|text| {
            Expression::parse(&text)
                .expect("valid oracle grammar")
                .encode()
                .to_vec()
        })
        .collect()
}

#[test]
fn grammar_prefix_equals_independent_nested_infix_enumeration_without_duplicates() {
    for (live, limit) in [
        (vec![0, 369], 5),
        (vec![0, 63, 64, 127, 128, 192, 274, 320, 369], 3),
    ] {
        let expected = reference(&live, limit);
        let mut cursor = Cursor::new(&live).unwrap();
        let mut observed = BTreeSet::new();
        let mut work = 0;
        loop {
            match cursor.advance(50_000, &mut work).unwrap() {
                Step::Candidate(expression) => {
                    if size(&expression) > limit {
                        break;
                    }
                    let encoded = expression.encode();
                    assert_eq!(Expression::decode(&encoded).unwrap(), expression);
                    assert!(
                        observed.insert(encoded.to_vec()),
                        "duplicate syntax program"
                    );
                }
                Step::Paused => assert!(work < 5_000_000, "finite fixture exceeded work ceiling"),
                Step::Exhausted => panic!("wire language cannot end inside this tiny prefix"),
            }
        }
        assert_eq!(observed, expected);
        assert!(work > u64::try_from(observed.len()).unwrap());
    }
}

#[test]
fn every_node_can_resume_with_the_identical_remaining_program_order_and_work() {
    let mut original = Cursor::new(&[0, 369]).unwrap();
    let mut restored = original.clone();
    let mut left_work = 0;
    let mut right_work = 0;
    let mut candidates = 0;
    for _ in 0..20_000 {
        let before = restored.encode();
        restored = Cursor::decode(&before).unwrap();
        assert_eq!(restored.encode(), before);
        let left = original.advance(1, &mut left_work).unwrap();
        let right = restored.advance(1, &mut right_work).unwrap();
        assert_eq!(left, right);
        assert_eq!(original.encode(), restored.encode());
        assert_eq!(left_work, right_work);
        if matches!(left, Step::Candidate(_)) {
            candidates += 1;
        }
    }
    assert!(candidates > 100);
}

#[test]
fn no_work_and_counter_overflow_do_not_advance_or_claim_exhaustion() {
    let mut cursor = Cursor::new(&[0]).unwrap();
    let before = cursor.encode();
    let mut work = 7;
    assert_eq!(cursor.advance(0, &mut work), Ok(Step::Paused));
    assert_eq!(work, 7);
    assert_eq!(cursor.encode(), before);
    work = u64::MAX;
    assert_eq!(cursor.advance(1, &mut work), Err(Refusal::WorkOverflow));
    assert_eq!(cursor.encode(), before);
}

#[test]
fn malformed_alphabets_and_checkpoint_fields_are_refused() {
    for bits in [vec![], vec![369, 0], vec![0, 0], vec![u32::MAX], vec![384]] {
        assert_eq!(Cursor::new(&bits).unwrap_err(), Refusal::Alphabet);
    }
    let base = Cursor::new(&[0, 369]).unwrap().encode();
    assert_eq!(base.len(), CURSOR_BYTES);
    for (offset, value) in [
        (0, 0),
        (8, 0),
        (10, 0),
        (12, 1),
        (14, 2),
        (15, 1),
        (20, 1),
        (784, 6),
        (786, 1),
    ] {
        let mut changed = base;
        changed[offset] = value;
        assert!(Cursor::decode(&changed).is_err(), "offset {offset}");
    }
    let mut finished = base;
    finished[14] = 1;
    assert!(
        Cursor::decode(&finished).is_err(),
        "early exhaustion is invalid"
    );
    finished[10..12].copy_from_slice(&u16::try_from(MAX_INSTRUCTIONS).unwrap().to_le_bytes());
    let mut cursor = Cursor::decode(&finished).unwrap();
    let mut work = 0;
    assert_eq!(cursor.advance(100, &mut work), Ok(Step::Exhausted));
    assert_eq!(work, 0);
    // A valid exact checkpoint must still be bound by the caller's identity and
    // seal. Structural validation alone cannot authenticate an arbitrary file.
}

/// ET-expressions-3 (D-0750): a cursor's equality is exactly its encoded
/// state. The unencoded instruction scratch that `advance` leaves behind at
/// `at` after a candidate, and above `at` after a backtrack, is not state:
/// every slot is rewritten before it is read. A decoded checkpoint therefore
/// equals the cursor it was saved from, and a real change of state does not.
#[test]
fn cursor_equality_is_the_encoded_state_and_ignores_unencoded_scratch() {
    let mut cursor = Cursor::new(&[0, 369]).unwrap();
    let mut work = 0;
    let mut saw_candidate = false;
    let mut saw_backtrack = false;
    for _ in 0..400 {
        let before = cursor.clone();
        let step = cursor.advance(1, &mut work).unwrap();
        let bytes = cursor.encode();
        let reopened = Cursor::decode(&bytes).unwrap();
        assert_eq!(reopened.encode(), bytes);
        // `assert!` rather than `assert_eq!`: a failure would otherwise print
        // two 1,151-slot scratch arrays.
        assert!(
            reopened == cursor,
            "a reopened checkpoint equals its source"
        );
        assert!(cursor == reopened, "equality is symmetric");
        if matches!(step, Step::Candidate(_)) {
            saw_candidate = true;
        }
        // Consecutive positions of one traversal are different states, and
        // equality must say so.
        assert!(before != cursor, "every node changes the encoded state");
        assert_ne!(before.encode(), bytes);
        let at = |b: &[u8; CURSOR_BYTES]| u16::from_le_bytes([b[12], b[13]]);
        if at(&bytes) < at(&before.encode()) {
            saw_backtrack = true;
        }
    }
    assert!(saw_candidate && saw_backtrack);
    // Two different alphabets at the same progress are different states.
    assert_ne!(
        Cursor::new(&[0, 369]).unwrap(),
        Cursor::new(&[0, 63]).unwrap()
    );
}

fn first_live(n: usize) -> Vec<u32> {
    vocab::table::TABLE
        .iter()
        .filter(|row| row.status == vocab::table::BitStatus::Live)
        .take(n)
        .map(|row| u32::from(row.index))
        .collect()
}

fn cursor_length(cursor: &Cursor) -> u16 {
    let bytes = cursor.encode();
    u16::from_le_bytes([bytes[10], bytes[11]])
}

/// ET-expressions-5 (D-0752): a node budget bounds grammar choices, not
/// progress between candidates. Every rank is tried at every position,
/// including leaves that cannot fit and reversed siblings rejected only at
/// their operator, so nodes per emitted candidate grow with the alphabet and a
/// run of more than 4,096 nodes can pass with no candidate at all. Counted, not
/// timed.
#[test]
fn grammar_nodes_per_candidate_grow_with_the_alphabet_and_gaps_exceed_one_replay() {
    let mut per_alphabet = Vec::new();
    for n in [2, 8, 32] {
        let mut cursor = Cursor::new(&first_live(n)).unwrap();
        let (mut work, mut candidates) = (0_u64, 0_u64);
        while cursor_length(&cursor) < 4 {
            if matches!(cursor.advance(1, &mut work).unwrap(), Step::Candidate(_)) {
                candidates += 1;
            }
        }
        // Programs of one to three instructions: c leaves, c single NOTs,
        // c double NOTs and c(c+1) ordered AND/OR pairs.
        let c = u64::try_from(n).unwrap();
        assert_eq!(candidates, 3 * c + c * (c + 1));
        per_alphabet.push((n, work, candidates));
    }
    assert_eq!(
        per_alphabet,
        [(2, 78, 12), (8, 1_092, 96), (32, 40_428, 1_152)]
    );

    let mut cursor = Cursor::new(&first_live(2)).unwrap();
    let (mut work, mut candidates, mut last) = (0_u64, 0_u64, 0_u64);
    let gap = loop {
        if matches!(cursor.advance(1, &mut work).unwrap(), Step::Candidate(_)) {
            candidates += 1;
            if work - last > 4_096 {
                break work - last;
            }
            last = work;
        }
        assert!(work < 100_000, "no gap above 4,096 nodes was reached");
    };
    assert_eq!((gap, candidates, work), (6_577, 7_116, 73_130));
    assert_eq!(cursor_length(&cursor), 9);
}
