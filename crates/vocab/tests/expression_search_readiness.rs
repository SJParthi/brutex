//! Public wire boundaries and identity preservation for resumable grammar traversal.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "bounded independent wire fixtures and assertions are test-only"
)]

use vocab::ConditionMask;
use vocab::expression::{Expression, MAX_INSTRUCTIONS, Truth};
use vocab::expression_search::{CURSOR_BYTES, Cursor, Refusal, Step};

fn word(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn exact_wire_width_alphabet_and_initial_identity_survive_every_observed_state() {
    assert_eq!(CURSOR_BYTES, 3086, "V1 stores 16 + 384*2 + 1151*2 bytes");
    let live = [0, 63, 64, 127, 128, 192, 320, 369];
    let expected = live
        .iter()
        .fold(ConditionMask::ZERO, |mask, &bit| mask.with_bit(bit));
    let mut cursor = Cursor::new(&live).unwrap();
    let initial = cursor.encode();
    assert_eq!(&initial[..8], b"BTXEGN01");
    assert_eq!(initial.len(), 3086);
    let mut work = 0;
    let mut candidates = 0;
    let mut nested = false;
    for _ in 0..1500 {
        if matches!(cursor.advance(1, &mut work).unwrap(), Step::Candidate(_)) {
            candidates += 1;
        }
        let saved = cursor.encode();
        nested |= u16::from_le_bytes([saved[12], saved[13]]) > 0;
        assert_eq!(cursor.alphabet(), expected);
        assert_eq!(cursor.initial_descriptor(), initial);
        let reopened = Cursor::decode(&saved).unwrap();
        assert_eq!(reopened.encode(), saved);
        assert_eq!(reopened.alphabet(), expected);
        assert_eq!(reopened.initial_descriptor(), initial);
        cursor = reopened;
    }
    assert_eq!(work, 1500);
    assert!(candidates > live.len());
    assert!(
        nested,
        "identity was checked inside a partially built program"
    );
    assert_ne!(cursor.encode(), initial);
    assert_ne!(
        Cursor::new(&[0, 369]).unwrap().initial_descriptor(),
        initial
    );
}

#[test]
fn malformed_multibyte_bounds_and_exhaustion_payloads_refuse_explicitly() {
    assert_eq!(Cursor::new(&[0; 385]), Err(Refusal::Alphabet));
    let base = Cursor::new(&[0]).unwrap().encode();
    for (offset, value) in [
        (8, 0),
        (8, 385),
        (8, u16::MAX),
        (10, 0),
        (10, 1152),
        (10, u16::MAX),
        (12, 1),
        (12, 1151),
        (12, u16::MAX),
    ] {
        let mut changed = base;
        word(&mut changed, offset, value);
        assert_eq!(
            Cursor::decode(&changed),
            Err(Refusal::Cursor),
            "{offset}={value}"
        );
    }
    let mut finished = base;
    word(&mut finished, 10, 1151);
    finished[14] = 1;
    for (offset, value) in [(784, 1), (784, 4), (786, 1), (3084, 1)] {
        let mut changed = finished;
        word(&mut changed, offset, value);
        assert_eq!(Cursor::decode(&changed), Err(Refusal::Cursor));
    }
    let mut changed = finished;
    word(&mut changed, 12, 1);
    word(&mut changed, 784, 1);
    assert_eq!(Cursor::decode(&changed), Err(Refusal::Cursor));

    // Digits can be in range while their selected prefix is impossible.
    for selected in [0, 2, 3, 4] {
        let mut changed = base;
        word(&mut changed, 10, 3);
        word(&mut changed, 12, 1);
        word(&mut changed, 784, selected);
        assert_eq!(Cursor::decode(&changed), Err(Refusal::Cursor));
    }
    let mut reversed = Cursor::new(&[0, 369]).unwrap().encode();
    word(&mut reversed, 10, 4);
    word(&mut reversed, 12, 3);
    for (index, selected) in [2, 1, 4].into_iter().enumerate() {
        word(&mut reversed, 784 + index * 2, selected);
    }
    assert_eq!(Cursor::decode(&reversed), Err(Refusal::Cursor));
}

#[test]
fn maximum_program_and_final_backtrack_keep_the_fixed_capacity_contract() {
    // A valid late traversal state has one high-word operand followed by NOTs.
    // Build the public cursor wire directly; no private search state is exposed.
    let mut saved = Cursor::new(&[369]).unwrap().encode();
    word(&mut saved, 10, 1151);
    word(&mut saved, 12, 1150);
    word(&mut saved, 784, 1);
    for index in 1..1150 {
        word(&mut saved, 784 + index * 2, 2);
    }
    word(&mut saved, 3084, 1);
    let mut cursor = Cursor::decode(&saved).unwrap();
    let mut work = 41;
    let Step::Candidate(expression) = cursor.advance(1, &mut work).unwrap() else {
        panic!("the last NOT must finish the maximum-capacity program");
    };
    let mut expected = Expression::parse("369").unwrap().encode();
    word(&mut expected, 2, 1151);
    for index in 1..MAX_INSTRUCTIONS {
        expected[4 + index * 3] = 2;
    }
    assert_eq!(expression.encode(), expected);
    assert_eq!(Expression::decode(&expected).unwrap(), expression);
    assert_eq!(work, 42);
    let high = ConditionMask::ZERO.with_bit(369);
    assert_eq!(expression.evaluate(high, high), Truth::True);
    assert_eq!(expression.evaluate(ConditionMask::ZERO, high), Truth::False);
    assert_eq!(
        expression.evaluate(ConditionMask::ZERO, ConditionMask::ZERO),
        Truth::Unknown
    );
    assert_eq!(
        Cursor::decode(&cursor.encode()).unwrap().encode(),
        cursor.encode()
    );

    let mut tail = Cursor::new(&[369]).unwrap().encode();
    word(&mut tail, 10, 1151);
    word(&mut tail, 784, 4);
    let mut last = Cursor::decode(&tail).unwrap();
    assert_eq!(last.advance(1, &mut work), Ok(Step::Exhausted));
    assert_eq!(work, 43);
    let sealed = last.encode();
    assert_eq!(
        Cursor::decode(&sealed).unwrap().advance(99, &mut work),
        Ok(Step::Exhausted)
    );
    assert_eq!(work, 43);
    assert_eq!(
        last.initial_descriptor(),
        Cursor::new(&[369]).unwrap().encode()
    );
}
