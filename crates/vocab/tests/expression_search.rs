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
