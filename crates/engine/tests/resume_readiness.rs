//! Independent finite resume oracle. These fixtures do not certify all inputs.

#![allow(
    clippy::expect_used,
    reason = "bounded test fixtures must encode and decode"
)]

use std::collections::BTreeMap;

use engine::column::Column;
use engine::resume::{Checkpoint, CheckpointView, Error};
use engine::{Breach, Ladder, Sweep};
use vocab::ConditionMask;

const POSITIONS: [u32; 8] = [0, 63, 64, 127, 128, 192, 320, 369];
const IDENTITY: [u8; 32] = [0x5a; 32];

fn masks(count: usize, mixed: bool) -> Vec<ConditionMask> {
    (0..count)
        .map(|row| {
            POSITIONS
                .iter()
                .enumerate()
                .fold(ConditionMask::ZERO, |mask, (bit, &position)| {
                    if row + 1 < count && (!mixed || (row.wrapping_mul(37) + bit * 11) % 7 < 5) {
                        mask.with_bit(position)
                    } else {
                        mask
                    }
                })
        })
        .collect()
}

fn oracle(rows: &[ConditionMask], min_hits: u64) -> BTreeMap<[u64; 6], u64> {
    let bars = u64::try_from(rows.len()).expect("finite rows");
    let eligible: Vec<u32> = POSITIONS
        .into_iter()
        .filter(|position| {
            let hits = rows.iter().filter(|row| row.get(*position)).count() as u64;
            hits > 0 && hits < bars && hits >= min_hits.max(1)
        })
        .collect();
    let mut expected = BTreeMap::new();
    for subset in 1_usize..(1 << eligible.len()) {
        let positions: Vec<u32> = eligible
            .iter()
            .enumerate()
            .filter_map(|(bit, &position)| (subset & (1 << bit) != 0).then_some(position))
            .collect();
        let hits = rows
            .iter()
            .filter(|row| positions.iter().all(|&position| row.get(position)))
            .count() as u64;
        if hits >= min_hits.max(1) {
            let mask = positions
                .into_iter()
                .fold(ConditionMask::ZERO, ConditionMask::with_bit);
            assert!(expected.insert(mask.words(), hits).is_none());
        }
    }
    expected
}

fn encode(view: &CheckpointView<'_>) -> Vec<u8> {
    let mut bytes = Vec::new();
    view.write_to(&mut bytes).expect("fixture encodes");
    bytes
}

fn decode(bytes: &[u8]) -> Checkpoint {
    let decoded = Checkpoint::read_from(&mut &*bytes, bytes.len() as u64, IDENTITY);
    assert!(
        decoded.is_ok(),
        "fixture decode: {decoded:?}, header {:?}",
        bytes.get(56..192).expect("header")
    );
    decoded.expect("fixture decodes")
}

fn check_interruption(column: &Column, live: &[u32], ladder: Ladder, expected: &Sweep, stop: u32) {
    let mut saved = Vec::new();
    let mut before = Vec::new();
    let final_depth = expected.levels.last().expect("terminal level").k;
    let interrupted = ladder.walk_checkpointed(column, live, IDENTITY, &mut |view| {
        before.push(view.current().k);
        assert_eq!(view.terminal(), view.current().k == final_depth);
        if view.current().k == stop {
            saved = encode(view);
            Err("simulated durable pause".into())
        } else {
            Ok(())
        }
    });
    assert!(
        matches!(interrupted, Err(Error::Callback(ref reason)) if reason == "simulated durable pause")
    );
    assert_eq!(before, (1..=stop).collect::<Vec<_>>());
    let checkpoint = decode(&saved);
    assert_eq!(checkpoint.identity(), IDENTITY);
    assert_eq!(checkpoint.levels().last().expect("current").k, stop);
    assert_eq!(
        checkpoint.levels(),
        expected.levels.get(..stop as usize).expect("known prefix")
    );
    let mut canonical = Vec::new();
    checkpoint.write_to(&mut canonical).expect("roundtrip");
    assert_eq!(canonical, saved);
    let mut after = Vec::new();
    let resumed = ladder
        .resume_checkpointed(column, live, IDENTITY, checkpoint, &mut |view| {
            after.push(view.current().k);
            assert_eq!(view.retired().len() + 1, view.current().k as usize);
            assert_eq!(view.identity(), IDENTITY);
            assert_eq!(view.terminal(), view.current().k == final_depth);
            Ok(())
        })
        .expect("resume");
    assert_eq!(resumed, *expected);
    assert_eq!(
        after,
        (stop..=expected.levels.last().expect("terminal").k).collect::<Vec<_>>()
    );
}

#[test]
fn every_safe_depth_resumes_to_the_exhaustive_subset_oracle() {
    let mut cases = 0;
    let mut boundaries = 0;
    let mut largest_depth = 0;
    for count in [0, 1, 2, 9, 65] {
        for mixed in [false, true] {
            let rows = masks(count, mixed);
            let column = Column::try_from_rows(&rows).expect("column");
            for threshold in [0, 1, 2, count as u64, count as u64 + 1, u64::MAX] {
                for repeated in [false, true] {
                    let mut live = POSITIONS.to_vec();
                    if repeated {
                        live.reverse();
                        live.extend(POSITIONS);
                        live.extend([6, 384, u32::MAX, 6]);
                    }
                    let ladder = Ladder::with_min_hits(threshold).with_support_lanes(1);
                    let expected = ladder.walk_column(&column, &live, &|_, _, _| {});
                    let actual: BTreeMap<_, _> = expected
                        .all_frequent()
                        .map(|item| (item.mask.words(), item.hits))
                        .collect();
                    assert_eq!(actual, oracle(&rows, threshold));
                    assert!(expected.completed());
                    assert!(expected.levels.iter().all(engine::Frontier::reconciles));
                    for level in &expected.levels {
                        check_interruption(&column, &live, ladder, &expected, level.k);
                        boundaries += 1;
                        largest_depth = largest_depth.max(level.k);
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 120);
    assert!(boundaries > cases);
    assert_eq!(
        largest_depth, 9,
        "the full eight-bit powerset includes an empty ninth level"
    );
}

fn fixture_at_second_level() -> (Ladder, Column, Vec<u8>) {
    let column = Column::try_from_rows(&masks(3, false)).expect("column");
    let ladder = Ladder::with_min_hits(1).with_support_lanes(1);
    let mut saved = Vec::new();
    let result = ladder.walk_checkpointed(&column, &POSITIONS, IDENTITY, &mut |view| {
        if view.current().k == 2 {
            saved = encode(view);
            Err("pause".into())
        } else {
            Ok(())
        }
    });
    assert!(matches!(result, Err(Error::Callback(_))));
    (ladder, column, saved)
}

fn replace_word(bytes: &mut [u8], offset: usize, value: u64) {
    bytes
        .get_mut(offset..offset + 8)
        .expect("field exists")
        .copy_from_slice(&value.to_le_bytes());
}

#[test]
fn truncated_trailing_foreign_and_malformed_payloads_are_refused() {
    let (_, _, bytes) = fixture_at_second_level();
    for length in 0..bytes.len() {
        assert!(
            Checkpoint::read_from(
                &mut bytes.get(..length).expect("prefix"),
                bytes.len() as u64,
                IDENTITY
            )
            .is_err(),
            "truncation at {length}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(
        Checkpoint::read_from(&mut trailing.as_slice(), trailing.len() as u64, IDENTITY).is_err()
    );
    assert!(
        Checkpoint::read_from(&mut bytes.as_slice(), bytes.len() as u64 - 1, IDENTITY).is_err()
    );
    assert!(Checkpoint::read_from(&mut bytes.as_slice(), bytes.len() as u64, [0; 32]).is_err());
    let first = 192 + POSITIONS.len() * 8;
    for (offset, value) in [
        (0, 0),
        (8, 2),
        (16, 999),
        (24, 0),
        (56, 0),
        (64, 0),
        (64, 27),
        (72, 0),
        (72, 28), // the final empty outer row refuses exact budget exhaustion
        (96, u64::MAX),
        (104, 0),
        (104, u64::MAX),
        (120, 1),
        (168, u64::MAX),
        (176, u64::MAX),
        (184, 0),
        (184, 386),
        (first, 2),
        (first + 8, u64::MAX),
        (first + 16, 9),
        (first + 24, 1),
        (first + 40, 1),
        (first + 56 + 40, 0),
        (first + 56, 1 << 6),
        (first + 56 + 48, 0),
        (first + 56 + 48, 3),
    ] {
        let mut bad = bytes.clone();
        replace_word(&mut bad, offset, value);
        assert!(
            Checkpoint::read_from(&mut bad.as_slice(), bad.len() as u64, IDENTITY).is_err(),
            "field {offset}, value {value}"
        );
    }
}

#[test]
fn resumed_identity_column_offers_and_every_configuration_term_must_match() {
    let (ladder, column, bytes) = fixture_at_second_level();
    for other in [
        ladder.with_ceiling(ladder.ceiling() - 1),
        Ladder::with_min_hits(2).with_support_lanes(1),
        ladder.with_pair_budget(ladder.pair_budget() - 1),
        ladder.with_support_lanes(usize::MAX),
    ] {
        assert!(matches!(
            other.resume_checkpointed(&column, &POSITIONS, IDENTITY, decode(&bytes), &mut |_| Ok(
                ()
            )),
            Err(Error::Invalid(_))
        ));
    }
    assert!(
        ladder
            .resume_checkpointed(
                &column,
                &POSITIONS,
                [0; 32],
                decode(&bytes),
                &mut |_| Ok(())
            )
            .is_err()
    );
    let other_column = Column::try_from_rows(&masks(4, false)).expect("other column");
    assert!(
        ladder
            .resume_checkpointed(
                &other_column,
                &POSITIONS,
                IDENTITY,
                decode(&bytes),
                &mut |_| Ok(())
            )
            .is_err()
    );
    let mut other = POSITIONS;
    other.reverse();
    assert!(
        ladder
            .resume_checkpointed(&column, &other, IDENTITY, decode(&bytes), &mut |_| Ok(()))
            .is_err()
    );
}

#[test]
fn real_resource_halts_keep_exact_cumulative_budgets_and_never_advance() {
    let column = Column::try_from_rows(&masks(3, false)).expect("column");
    for (ladder, reason, depth) in [
        (
            Ladder::with_min_hits(1).with_ceiling(29),
            Breach::Candidates,
            3,
        ),
        (
            Ladder::with_min_hits(1).with_pair_budget(29),
            Breach::Pairs,
            3,
        ),
        (
            Ladder::with_min_hits(1).with_ceiling(1),
            Breach::Candidates,
            2,
        ),
        (
            Ladder::with_min_hits(1).with_pair_budget(1),
            Breach::Pairs,
            2,
        ),
        (
            Ladder::with_min_hits(1).with_ceiling(2).with_pair_budget(1),
            Breach::Candidates,
            2,
        ),
    ] {
        let ladder = ladder.with_support_lanes(1);
        let expected = ladder.walk_column(&column, &POSITIONS, &|_, _, _| {});
        let halt = expected.halted.expect("resource halt");
        assert_eq!(halt.breach, reason);
        assert_eq!(
            halt.k, depth,
            "first joined level and accumulated later budget"
        );
        if ladder.pair_budget() == 1 {
            assert!(
                halt.pairs > 1,
                "the shared outer-row check admits a whole row"
            );
            if reason == Breach::Candidates {
                assert_eq!(halt.pairs, 3);
                assert_eq!(halt.candidates, 2);
            }
        }
        for level in &expected.levels {
            check_interruption(&column, &POSITIONS, ladder, &expected, level.k);
        }
        let mut saved = Vec::new();
        let result = ladder
            .walk_checkpointed(&column, &POSITIONS, IDENTITY, &mut |view| {
                if view.halted().is_some() {
                    assert!(view.terminal());
                    assert_eq!(view.admitted(), halt.candidates);
                    assert_eq!(view.pairs(), halt.pairs);
                    saved = encode(view);
                }
                Ok(())
            })
            .expect("checkpointed halted outcome");
        assert_eq!(result, expected);
        let checkpoint = decode(&saved);
        assert_eq!(checkpoint.halted(), Some(halt));
        assert_eq!(checkpoint.bars(), 3);
        assert_eq!(checkpoint.admitted(), halt.candidates);
        assert_eq!(checkpoint.pairs(), halt.pairs);
        assert!(checkpoint.excluded().is_empty());
        let mut calls = 0;
        let resumed = ladder
            .resume_checkpointed(&column, &POSITIONS, IDENTITY, checkpoint, &mut |_| {
                calls += 1;
                Ok(())
            })
            .expect("terminal replay");
        assert_eq!(calls, 1);
        assert_eq!(resumed, expected);
        assert!(!resumed.completed());
    }
}

struct FailingIo<T> {
    inner: T,
    remaining: usize,
}

impl<T: std::io::Read> std::io::Read for FailingIo<T> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::other("injected read failure"));
        }
        let length = buffer.len().min(self.remaining);
        let read = self
            .inner
            .read(buffer.get_mut(..length).expect("bounded buffer"))?;
        self.remaining -= read;
        Ok(read)
    }
}

impl<T: std::io::Write> std::io::Write for FailingIo<T> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(std::io::Error::other("injected write failure"));
        }
        let length = buffer.len().min(self.remaining);
        let written = self
            .inner
            .write(buffer.get(..length).expect("bounded buffer"))?;
        self.remaining -= written;
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

#[test]
fn every_payload_byte_and_final_read_probe_propagate_io_failure() {
    let (_, _, bytes) = fixture_at_second_level();
    check_io_boundaries(&bytes);
    let exclusions = capture_singletons(&[0, 64, 6], &[ConditionMask::ZERO.with_bit(0)]);
    check_io_boundaries(&exclusions);
}

fn check_io_boundaries(bytes: &[u8]) {
    use std::error::Error as _;

    let checkpoint = decode(bytes);
    for remaining in 0..=bytes.len() {
        let mut reader = FailingIo {
            inner: bytes,
            remaining,
        };
        let error = Checkpoint::read_from(&mut reader, bytes.len() as u64, IDENTITY)
            .expect_err("failure, including the exact-EOF probe, must not be hidden");
        assert!(matches!(error, Error::Io(_)), "byte {remaining}: {error}");
        assert_eq!(
            error.source().expect("I/O source").to_string(),
            "injected read failure"
        );
        assert_eq!(
            error.to_string(),
            "checkpoint I/O refused: injected read failure"
        );
        if remaining < bytes.len() {
            let mut writer = FailingIo {
                inner: Vec::new(),
                remaining,
            };
            let error = checkpoint
                .write_to(&mut writer)
                .expect_err("partial write refuses");
            assert_eq!(
                writer.inner,
                bytes.get(..remaining).expect("written prefix")
            );
            assert_eq!(
                error.source().expect("I/O source").to_string(),
                "injected write failure"
            );
        }
    }
    let mut writer = FailingIo {
        inner: Vec::new(),
        remaining: bytes.len(),
    };
    checkpoint
        .write_to(&mut writer)
        .expect("exact byte capacity");
    assert_eq!(writer.inner, bytes);
}

#[test]
fn public_refusal_diagnostics_preserve_the_reason_and_error_source() {
    use std::error::Error as _;

    let mut impossible = Vec::<u8>::new();
    let reserve = impossible
        .try_reserve(usize::MAX)
        .expect_err("capacity exceeds isize");
    for (error, expected) in [
        (
            Error::Invalid("wrong identity"),
            "checkpoint refused: wrong identity",
        ),
        (
            Error::Callback("disk refused".into()),
            "checkpoint sink refused: disk refused",
        ),
        (Error::from(reserve), "checkpoint allocation refused"),
    ] {
        assert_eq!(error.to_string(), expected);
        assert!(error.source().is_none());
    }
}

#[test]
fn an_excluded_bit_cannot_reappear_as_a_surviving_singleton() {
    let live = [0, 63, 64];
    let column = Column::try_from_rows(&[
        ConditionMask::ZERO.with_bit(0).with_bit(63),
        ConditionMask::ZERO.with_bit(63),
        ConditionMask::ZERO,
    ])
    .expect("column");
    let ladder = Ladder::with_min_hits(1).with_support_lanes(1);
    let mut saved = Vec::new();
    let result = ladder.walk_checkpointed(&column, &live, IDENTITY, &mut |view| {
        saved = encode(view);
        Err("pause at singletons".into())
    });
    assert!(matches!(result, Err(Error::Callback(_))));
    let checkpoint = decode(&saved);
    assert_eq!(checkpoint.excluded().len(), 1);
    assert_eq!(
        checkpoint.excluded().first().expect("exclusion").position,
        64
    );
    assert_eq!(
        checkpoint
            .levels()
            .first()
            .expect("singleton level")
            .frequent
            .len(),
        2
    );
    // The payload remains internally sized, ordered and reconciled: replace
    // singleton 0 by offered bit 64, which the same payload says never hits.
    let first_item = 192 + live.len() * 8 + 24 + 56;
    replace_word(&mut saved, first_item, 0);
    replace_word(&mut saved, first_item + 8, 1);
    assert!(matches!(
        Checkpoint::read_from(&mut saved.as_slice(), saved.len() as u64, IDENTITY),
        Err(Error::Invalid(_))
    ));
}

fn capture_singletons(live: &[u32], rows: &[ConditionMask]) -> Vec<u8> {
    let column = Column::try_from_rows(rows).expect("column");
    let mut saved = Vec::new();
    let result = Ladder::with_min_hits(1)
        .with_support_lanes(1)
        .walk_checkpointed(&column, live, IDENTITY, &mut |view| {
            saved = encode(view);
            Err("pause at singletons".into())
        });
    assert!(matches!(result, Err(Error::Callback(_))));
    saved
}

#[test]
fn exclusions_and_singleton_counts_are_independently_reconciled() {
    let live = [0, 63, 64, 6, 384, u32::MAX, 6];
    let saved = capture_singletons(
        &live,
        &[
            ConditionMask::ZERO.with_bit(0).with_bit(63),
            ConditionMask::ZERO.with_bit(0),
            ConditionMask::ZERO.with_bit(0),
        ],
    );
    let checkpoint = decode(&saved);
    let excluded = checkpoint.excluded();
    assert_eq!(excluded.len(), 5);
    assert_eq!(
        excluded.first(),
        Some(&engine::Excluded {
            position: 0,
            support: Some(3),
            reason: engine::Why::AlwaysTrue
        })
    );
    assert_eq!(
        excluded.get(1),
        Some(&engine::Excluded {
            position: 64,
            support: Some(0),
            reason: engine::Why::AlwaysFalse
        })
    );
    assert!(
        excluded
            .get(2..)
            .expect("nonlive exclusions")
            .iter()
            .all(|row| row.reason == engine::Why::NotLive && row.support.is_none())
    );
    assert_eq!(
        checkpoint
            .levels()
            .first()
            .expect("singleton level")
            .duplicates,
        1
    );
    let mut roundtrip = Vec::new();
    checkpoint
        .write_to(&mut roundtrip)
        .expect("re-encode exclusions");
    assert_eq!(roundtrip, saved);
    let start = 192 + live.len() * 8;
    let level = start + excluded.len() * 24;
    for (offset, value) in [
        (104, 1),        // a singleton checkpoint has walked no pairs
        (level + 24, 0), // duplicate accounting independently disagrees
        (level + 32, 4), // exclusion accounting independently disagrees
        (start, 1),      // no such offered position
        (start, 6),      // a nonlive position cannot be always true
        (start + 8, 2),  // always true must equal the bar count
        (start + 16, 1), // always false must have zero support
        (start + 24, 0), // two exclusions cannot name the same position
        (start + 32, 1),
        (start + 40, 2),
        (start + 48, 63), // live bits cannot be marked not live
        (start + 56, 1),  // not-live wire support must be zero
        (start + 64, 3),  // unknown reason tags refuse
        (start + 64, u64::MAX),
    ] {
        let mut bad = saved.clone();
        replace_word(&mut bad, offset, value);
        assert!(
            Checkpoint::read_from(&mut bad.as_slice(), bad.len() as u64, IDENTITY).is_err(),
            "field {offset}, value {value}"
        );
    }
    // Keep total generated == all accounting buckets, so each inconsistent
    // bucket must be checked against its own independent source evidence.
    for offset in [level + 24, level + 32] {
        let mut bad = saved.clone();
        replace_word(&mut bad, offset, if offset == level + 24 { 0 } else { 4 });
        replace_word(&mut bad, level + 48, 1);
        assert!(Checkpoint::read_from(&mut bad.as_slice(), bad.len() as u64, IDENTITY).is_err());
    }
}

#[test]
fn reconciled_levels_still_require_continuity_order_and_depth_specific_buckets() {
    let (_, _, saved) = fixture_at_second_level();
    let first = 192 + POSITIONS.len() * 8;
    let second = first + 56 + POSITIONS.len() * 56;
    let mut wrong_offered_count = saved.clone();
    replace_word(
        &mut wrong_offered_count,
        first + 16,
        POSITIONS.len() as u64 + 1,
    );
    replace_word(&mut wrong_offered_count, first + 48, 1);
    assert!(matches!(
        Checkpoint::read_from(
            &mut wrong_offered_count.as_slice(),
            wrong_offered_count.len() as u64,
            IDENTITY
        ),
        Err(Error::Invalid("singleton accounting"))
    ));
    let mut empty_parent = saved.clone();
    empty_parent.drain(first + 56..second);
    replace_word(&mut empty_parent, first + 8, 0);
    replace_word(&mut empty_parent, first + 48, POSITIONS.len() as u64);
    assert!(
        Checkpoint::read_from(
            &mut empty_parent.as_slice(),
            empty_parent.len() as u64,
            IDENTITY
        )
        .is_err()
    );
    for offset in [first + 40, second + 24, second + 32] {
        let level = if offset < second { first } else { second };
        let mut bad = saved.clone();
        let length = if level == first { 8 } else { 28 };
        bad.drain(level + 56..level + 112);
        replace_word(&mut bad, level + 8, length - 1);
        replace_word(&mut bad, offset, 1);
        assert!(
            Checkpoint::read_from(&mut bad.as_slice(), bad.len() as u64, IDENTITY).is_err(),
            "forbidden bucket at {offset}"
        );
    }
    for duplicate in [true, false] {
        let mut bad = saved.clone();
        let first_item = first + 56;
        let mut items = bad
            .get(first_item..first_item + 112)
            .expect("two full items")
            .to_vec();
        if duplicate {
            items.copy_within(..56, 56);
        } else {
            items.rotate_left(56);
        }
        bad.get_mut(first_item..first_item + 112)
            .expect("two item slots")
            .copy_from_slice(&items);
        assert!(
            Checkpoint::read_from(&mut bad.as_slice(), bad.len() as u64, IDENTITY).is_err(),
            "duplicate={duplicate}"
        );
    }
}

#[test]
fn empty_frontiers_do_not_hide_invalid_exclusions_or_singleton_halts() {
    let live = [0, 64, 6];
    let saved = capture_singletons(&live, &[ConditionMask::ZERO.with_bit(0)]);
    let checkpoint = decode(&saved);
    assert!(
        checkpoint
            .levels()
            .first()
            .expect("singleton level")
            .frequent
            .is_empty()
    );
    assert_eq!(checkpoint.excluded().len(), 3);
    let start = 192 + live.len() * 8;
    for changes in [
        vec![(start + 3 * 24, 2)],      // an empty singleton still has depth one
        vec![(start + 3 * 24 + 48, 1)], // empty output must still reconcile every bucket
        vec![(72, 0)],                  // zero work still requires a positive policy budget
        vec![(88, 0), (start + 8, 0)],  // no AlwaysTrue exclusion in a zero-row column
        vec![(start + 24, 6)],          // an always-false position must be live
        vec![(start + 48, 64)],         // a not-live position cannot be live
        vec![(start + 16, 0), (start + 8, 0)],
        vec![(192, u64::from(u32::MAX) + 1)],
    ] {
        let mut bad = saved.clone();
        for (offset, value) in changes {
            replace_word(&mut bad, offset, value);
        }
        assert!(Checkpoint::read_from(&mut bad.as_slice(), bad.len() as u64, IDENTITY).is_err());
    }
    for max_bytes in [0, 1, 191, 192, saved.len() as u64 - 1] {
        assert!(Checkpoint::read_from(&mut saved.as_slice(), max_bytes, IDENTITY).is_err());
    }
    let mut halted = saved.clone();
    let word = |offset| {
        u64::from_le_bytes(
            saved
                .get(offset..offset + 8)
                .expect("field")
                .try_into()
                .expect("word"),
        )
    };
    for (offset, value) in [
        (112, 1),
        (120, 1),
        (136, word(64)),
        (152, word(72)),
        (160, 3),
    ] {
        replace_word(&mut halted, offset, value);
    }
    assert!(matches!(
        Checkpoint::read_from(&mut halted.as_slice(), halted.len() as u64, IDENTITY),
        Err(Error::Invalid("halt does not match accumulated state"))
    ));
}

#[test]
fn declared_counts_refuse_at_the_byte_boundary_before_reading_their_payload() {
    let (_, _, saved) = fixture_at_second_level();
    let first = 192 + POSITIONS.len() * 8;
    for (offset, count, consumed) in [
        (168, saved.len() as u64, 192),
        (176, saved.len() as u64, first),
        (184, 385, first),
        (first + 8, saved.len() as u64, first + 56),
    ] {
        let mut bad = saved.clone();
        replace_word(&mut bad, offset, count);
        let mut reader = std::io::Cursor::new(bad.as_slice());
        let result = Checkpoint::read_from(&mut reader, bad.len() as u64, IDENTITY);
        assert!(
            matches!(result, Err(Error::Invalid("count exceeds remaining bytes"))),
            "field {offset}: {result:?}"
        );
        assert_eq!(
            reader.position(),
            consumed as u64,
            "no count-governed payload may be read after refusal"
        );
    }
    for count in [0, 386, u64::from(u32::MAX)] {
        let mut bad = saved.clone();
        replace_word(&mut bad, 184, count);
        let mut reader = std::io::Cursor::new(bad.as_slice());
        let result = Checkpoint::read_from(&mut reader, bad.len() as u64, IDENTITY);
        assert!(
            matches!(result, Err(Error::Invalid("invalid level count"))),
            "count {count}: {result:?}"
        );
        assert_eq!(
            reader.position(),
            192,
            "invalid level count refuses before any vectors"
        );
    }
    for budget in [0, 1, 191, 192] {
        let mut reader = std::io::Cursor::new(saved.as_slice());
        let result = Checkpoint::read_from(&mut reader, budget, IDENTITY);
        let expected = if budget < 192 {
            "unsupported header/version"
        } else {
            "count exceeds remaining bytes"
        };
        assert!(matches!(result, Err(Error::Invalid(reason)) if reason == expected));
        assert_eq!(reader.position(), if budget < 192 { 0 } else { 192 });
    }
}

#[test]
fn an_infrequent_live_bit_cannot_be_relabelled_as_not_live() {
    let live = [0, 63, 64, 6];
    let column = Column::try_from_rows(&[
        ConditionMask::ZERO.with_bit(0).with_bit(63),
        ConditionMask::ZERO.with_bit(0),
        ConditionMask::ZERO.with_bit(0),
    ])
    .expect("column");
    let mut saved = Vec::new();
    let result = Ladder::with_min_hits(2)
        .with_support_lanes(1)
        .walk_checkpointed(&column, &live, IDENTITY, &mut |view| {
            saved = encode(view);
            Err("pause".into())
        });
    assert!(matches!(result, Err(Error::Callback(_))));
    let checkpoint = decode(&saved);
    assert_eq!(checkpoint.levels().first().expect("level").infrequent, 1);
    assert!(
        checkpoint
            .levels()
            .first()
            .expect("level")
            .frequent
            .is_empty()
    );
    assert_eq!(checkpoint.excluded().last().expect("nonlive").position, 6);
    // No duplicate, survivor overlap, accounting discrepancy or absent offer
    // masks this contradiction: bit 63 is live but below the support threshold.
    replace_word(&mut saved, 192 + live.len() * 8 + 2 * 24, 63);
    assert!(matches!(
        Checkpoint::read_from(&mut saved.as_slice(), saved.len() as u64, IDENTITY),
        Err(Error::Invalid("invalid exclusion"))
    ));
}

#[test]
fn serialized_resource_tags_preserve_terminal_refusal_and_reject_contradictions() {
    let column = Column::try_from_rows(&masks(3, false)).expect("column");
    let ladder = Ladder::with_min_hits(1)
        .with_support_lanes(1)
        .with_pair_budget(29);
    let mut saved = Vec::new();
    let sweep = ladder
        .walk_checkpointed(&column, &POSITIONS, IDENTITY, &mut |view| {
            if view.halted().is_some() {
                saved = encode(view);
            }
            Ok(())
        })
        .expect("real pair halt");
    assert_eq!(sweep.halted.expect("halt").breach, Breach::Pairs);
    // The codec supports persisted Memory/Workers outcomes without requiring
    // this test to exhaust process memory or interfere with operating-system threads.
    for (tag, expected) in [
        (2, Breach::Pairs),
        (3, Breach::Memory),
        (4, Breach::Workers),
    ] {
        let mut tagged = saved.clone();
        replace_word(&mut tagged, 160, tag);
        let checkpoint = decode(&tagged);
        assert_eq!(checkpoint.halted().expect("named halt").breach, expected);
        let mut encoded = Vec::new();
        checkpoint
            .write_to(&mut encoded)
            .expect("roundtrip named halt");
        assert_eq!(encoded, tagged);
        let mut calls = 0;
        let resumed = ladder
            .resume_checkpointed(&column, &POSITIONS, IDENTITY, checkpoint, &mut |view| {
                calls += 1;
                assert!(view.terminal());
                assert_eq!(view.halted().expect("named halt").breach, expected);
                Ok(())
            })
            .expect("terminal checkpoint");
        assert_eq!(calls, 1);
        assert!(!resumed.completed());
    }
    for (offset, value) in [
        (112, 2),
        (120, 1),
        (120, 4),
        (128, 0),
        (136, 0),
        (144, 0),
        (152, 0),
        (160, 0),
        (160, 1),
        (160, 5),
    ] {
        let mut bad = saved.clone();
        replace_word(&mut bad, offset, value);
        assert!(
            Checkpoint::read_from(&mut bad.as_slice(), bad.len() as u64, IDENTITY).is_err(),
            "field {offset}, value {value}"
        );
    }
    let mut premature_halt = saved.clone();
    replace_word(&mut premature_halt, 72, 30);
    replace_word(&mut premature_halt, 152, 30);
    assert!(matches!(
        Checkpoint::read_from(
            &mut premature_halt.as_slice(),
            premature_halt.len() as u64,
            IDENTITY
        ),
        Err(Error::Invalid("halt does not match accumulated state"))
    ));
    let mut zero_policy = saved.clone();
    for (offset, value) in [(72, 0), (152, 0), (160, 3)] {
        replace_word(&mut zero_policy, offset, value);
    }
    assert!(matches!(
        Checkpoint::read_from(
            &mut zero_policy.as_slice(),
            zero_policy.len() as u64,
            IDENTITY
        ),
        Err(Error::Invalid("singleton accounting"))
    ));
}
