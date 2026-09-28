#![cfg(test)]
use super::*;
use crate::search_checkpoint::tests::Scratch;
use crate::sweep_evidence::{self, Completion, Operation};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use vocab::ConditionMask;

const POSITIONS: [u32; 8] = [0, 63, 64, 127, 128, 192, 320, 369];
const ID: [u8; 32] = [0x37; 32];
/// Small enough that the eight-position lattice's 14,784-byte history
/// crosses one entry's admission several times over and its widest level
/// needs four chunks.
const SMALL: Limits = Limits {
    entry: 4096,
    chunk: chunk(1024),
};

/// A test chunk size, from a nonzero literal at every call.
#[expect(clippy::unwrap_used, reason = "nonzero test literals")]
const fn chunk(bytes: usize) -> NonZeroUsize {
    NonZeroUsize::new(bytes).unwrap()
}
const LENGTHS: &str = "AND checkpoint boundary has invalid exact lengths";
const LEVEL: &str = "AND checkpoint boundary names an empty or excess level";
const ORDER: &str = "AND checkpoint boundary chunk order or length is invalid";
const UNNAMED: &str = "AND checkpoint boundary chunks do not precede it exactly";

fn column() -> Result<engine::column::Column, String> {
    let full = POSITIONS
        .into_iter()
        .fold(ConditionMask::ZERO, ConditionMask::with_bit);
    engine::column::Column::try_from_rows(&[full, full, ConditionMask::ZERO]).map_err(error)
}

fn identity_hex() -> Result<String, String> {
    use std::fmt::Write as _;
    ID.iter().try_fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").map_err(error)?;
        Ok(text)
    })
}

fn namespace(root: &Path) -> Result<PathBuf, String> {
    Ok(root.join(NAMESPACE).join(identity_hex()?))
}

/// The newest boundary record, decoded, with its sequence.
fn newest_boundary(root: &Path) -> Result<(u64, Boundary), String> {
    let journal = Journal::open(root, NAMESPACE, ID)?;
    let saved = journal.latest(MAX_BYTES)?.ok_or("no checkpoint")?;
    Ok((
        saved.sequence,
        decode_boundary(&saved.payload, saved.sequence)?,
    ))
}

#[test]
fn persisted_interrupt_restart_and_terminal_replay_preserve_every_depth_row() -> Result<(), String>
{
    for limits in [PRODUCTION, SMALL] {
        for ladder in [
            Ladder::with_min_hits(1),
            Ladder::with_min_hits(2),
            Ladder::with_min_hits(20),
            Ladder::with_min_hits(1).with_ceiling(29),
            Ladder::with_min_hits(1).with_pair_budget(29),
        ] {
            let scratch = Scratch::new().map_err(error)?;
            let ladder = ladder.with_support_lanes(1);
            let column = column()?;
            let expected_rows = RefCell::new(Vec::new());
            let expected = ladder.walk_column(&column, &POSITIONS, &|level, admitted, pairs| {
                expected_rows
                    .borrow_mut()
                    .push(DepthRow::of(level, admitted, pairs));
            });
            let stop = u32::try_from(expected.levels.len().min(3)).map_err(error)?;
            let first = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
            let stopped = walk_within(
                &scratch.0,
                &first,
                ladder,
                &column,
                &POSITIONS,
                limits,
                &mut |view| {
                    if view.current().k == stop {
                        Err("intentional interruption after durable depth".into())
                    } else {
                        Ok(())
                    }
                },
            );
            assert!(stopped.is_err());
            drop(first);
            let first_evidence =
                sweep_evidence::read(&scratch.0, ID, MAX_BYTES)?.ok_or("first attempt missing")?;
            assert_eq!(first_evidence.completion, Completion::Refused);
            assert_eq!(first_evidence.depth_rows, u64::from(stop));

            let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
            let mut visited = Vec::new();
            let resumed = walk_within(
                &scratch.0,
                &attempt,
                ladder,
                &column,
                &POSITIONS,
                limits,
                &mut |view| {
                    visited.push(view.current().k);
                    Ok(())
                },
            )?;
            assert_eq!(resumed, expected);
            assert_eq!(visited.first().copied(), Some(stop));
            let completion = if resumed.halted.is_some() {
                Completion::Halted
            } else {
                Completion::Completed
            };
            attempt.finish(completion)?;
            let evidence = sweep_evidence::read(&scratch.0, ID, MAX_BYTES)?
                .ok_or("resumed attempt missing")?;
            assert_eq!(evidence.completion, completion);
            assert_eq!(evidence.depth_rows, expected.levels.len() as u64);
            assert_eq!(
                sweep_evidence::depth_page(&scratch.0, &evidence, 0, MAX_ROWS, MAX_BYTES)?,
                *expected_rows.borrow()
            );
            terminal_replay(&scratch.0, ladder, &column, &expected, completion, limits)?;
        }
    }
    Ok(())
}

fn terminal_replay(
    root: &Path,
    ladder: Ladder,
    column: &engine::column::Column,
    expected: &Sweep,
    completion: Completion,
    limits: Limits,
) -> Result<(), String> {
    let journal = Journal::open(root, NAMESPACE, ID)?;
    let saved = journal
        .latest(MAX_BYTES)?
        .ok_or("missing completed checkpoint")?;
    let original = saved.payload.clone();
    let sequence = saved.sequence;
    let next = journal.next_sequence();
    drop(journal);
    let replay = sweep_evidence::begin(root, ID, Operation::Sweep)?;
    let mut callbacks = 0;
    assert_eq!(
        walk_within(
            root,
            &replay,
            ladder,
            column,
            &POSITIONS,
            limits,
            &mut |_| {
                callbacks += 1;
                Ok(())
            }
        )?,
        *expected
    );
    assert_eq!(callbacks, 1, "completed/partial terminal never advances");
    replay.finish(completion)?;
    let journal = Journal::open(root, NAMESPACE, ID)?;
    let latest = journal
        .latest(MAX_BYTES)?
        .ok_or("missing replay checkpoint")?;
    assert_eq!(
        latest.sequence, sequence,
        "replay is an acknowledgment, not a duplicate checkpoint"
    );
    assert_eq!(latest.payload, original);
    assert_eq!(journal.next_sequence(), next, "replay publishes nothing");
    Ok(())
}

/// **The defect this format exists to close.** Version 1 re-encoded the whole
/// retained history into ONE journal entry at every boundary, so a history
/// larger than one entry's admission refused after building the level that
/// crossed it, and did so again on every rerun. Here one entry admits 4,096
/// bytes and the history is 14,784: the walk completes, no entry exceeds the
/// admission, and each boundary published exactly its own level, once, in
/// chunks immediately followed by the boundary record that names them.
/// AC-whp-o1-0, D-0712.
#[test]
fn a_history_past_one_entrys_admission_completes_one_level_per_boundary() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let ladder = Ladder::with_min_hits(1).with_support_lanes(1);
    let column = column()?;
    let expected = ladder.walk_column(&column, &POSITIONS, &|_, _, _| {});
    let history = expected
        .levels
        .iter()
        .map(|level| 56 + 56 * level.frequent.len() as u64)
        .sum::<u64>();
    assert_eq!(history, 14_784);
    assert!(history > SMALL.entry, "the fixture must cross one entry");

    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let walked = walk_within(
        &scratch.0,
        &attempt,
        ladder,
        &column,
        &POSITIONS,
        SMALL,
        &mut |_| Ok(()),
    )?;
    assert_eq!(walked, expected);
    attempt.finish(Completion::Completed)?;

    let (last, boundary) = newest_boundary(&scratch.0)?;
    assert_eq!(boundary.rows.len(), expected.levels.len());
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    for sequence in 1..journal.next_sequence() {
        let saved = journal.read(sequence, SMALL.entry)?;
        assert!(
            saved.payload.len() as u64 + ENVELOPE <= SMALL.entry,
            "entry {sequence} exceeds one entry's admission"
        );
    }
    let mut next = 1;
    let mut chunks = 0;
    for (depth, (level, pieces)) in expected.levels.iter().zip(&boundary.levels).enumerate() {
        let bytes = pieces.iter().map(|piece| piece.length).sum::<u64>();
        assert_eq!(
            bytes,
            56 + 56 * level.frequent.len() as u64,
            "depth {} saved exactly its own level",
            depth + 1
        );
        assert_eq!(
            pieces.len() as u64,
            bytes.div_ceil(SMALL.chunk.get() as u64),
            "depth {} split into the fewest chunks",
            depth + 1
        );
        for piece in pieces {
            assert_eq!(piece.sequence, next, "chunks are published in order");
            next += 1;
        }
        chunks += pieces.len();
        // The entry after a level's last chunk is the boundary that names it.
        let saved = journal.read(next, SMALL.entry)?;
        let named = decode_boundary(&saved.payload, next)?;
        assert_eq!(named.rows.len(), depth + 1);
        assert_eq!(named.levels.get(depth), Some(pieces));
        next += 1;
    }
    assert_eq!(last + 1, next);
    assert_eq!(chunks, 20);
    Ok(())
}

/// A chunk size that is not a multiple of eight ends chunks inside the
/// engine's eight-byte words. Every chunk but a level's last carries exactly
/// that many bytes, a level's chunks carry exactly its bytes, and a walk
/// paused at depth 2 resumes through those split words to the uninterrupted
/// answer.
#[test]
fn a_chunk_boundary_inside_a_word_splits_and_replays_exactly() -> Result<(), String> {
    let odd = Limits {
        entry: 4096,
        chunk: chunk(1001),
    };
    let scratch = Scratch::new().map_err(error)?;
    let ladder = Ladder::with_min_hits(1).with_support_lanes(1);
    let column = column()?;
    let expected = ladder.walk_column(&column, &POSITIONS, &|_, _, _| {});
    let first = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let paused = walk_within(
        &scratch.0,
        &first,
        ladder,
        &column,
        &POSITIONS,
        odd,
        &mut |view| {
            if view.current().k == 2 {
                Err("pause".into())
            } else {
                Ok(())
            }
        },
    );
    assert!(paused.is_err());
    drop(first);
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let mut visited = Vec::new();
    let resumed = walk_within(
        &scratch.0,
        &attempt,
        ladder,
        &column,
        &POSITIONS,
        odd,
        &mut |view| {
            visited.push(view.current().k);
            Ok(())
        },
    )?;
    assert_eq!(resumed, expected);
    assert_eq!(visited.first().copied(), Some(2), "resumed, not restarted");
    attempt.finish(Completion::Completed)?;
    let (_, boundary) = newest_boundary(&scratch.0)?;
    assert_eq!(boundary.levels.len(), expected.levels.len());
    let mut whole_chunks = 0;
    for (level, pieces) in expected.levels.iter().zip(&boundary.levels) {
        let bytes = 56 + 56 * level.frequent.len() as u64;
        assert_eq!(pieces.iter().map(|piece| piece.length).sum::<u64>(), bytes);
        assert_eq!(pieces.len() as u64, bytes.div_ceil(1001));
        let (last, whole) = pieces.split_last().ok_or("a level has a chunk")?;
        assert!(whole.iter().all(|piece| piece.length == 1001));
        assert!(last.length <= 1001);
        whole_chunks += whole.len();
    }
    assert!(whole_chunks > 0, "some chunk ended inside a word");
    Ok(())
}

/// A boundary interrupted after its chunks and before its record leaves an
/// orphan chunk as the newest entry. Recovery follows that chunk's back
/// pointer to the boundary before it, resumes there, and rebuilds only the
/// level that was not acknowledged.
#[test]
fn an_unacknowledged_boundary_resumes_from_the_one_before() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let ladder = Ladder::with_min_hits(1).with_support_lanes(1);
    let column = column()?;
    let expected = ladder.walk_column(&column, &POSITIONS, &|_, _, _| {});
    let first = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    walk_within(
        &scratch.0,
        &first,
        ladder,
        &column,
        &POSITIONS,
        SMALL,
        &mut |_| Ok(()),
    )?;
    first.finish(Completion::Completed)?;
    let (last, _) = newest_boundary(&scratch.0)?;
    // The crash: the newest boundary's completion marker never landed.
    fs::remove_file(
        namespace(&scratch.0)?
            .join(format!("{last:016x}"))
            .join("complete"),
    )
    .map_err(error)?;

    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let mut visited = Vec::new();
    let resumed = walk_within(
        &scratch.0,
        &attempt,
        ladder,
        &column,
        &POSITIONS,
        SMALL,
        &mut |view| {
            visited.push(view.current().k);
            Ok(())
        },
    )?;
    assert_eq!(resumed, expected);
    let deepest = u32::try_from(expected.levels.len()).map_err(error)?;
    assert_eq!(
        visited,
        [deepest - 1, deepest],
        "only the unacknowledged level is rebuilt"
    );
    attempt.finish(Completion::Completed)?;
    let (_, boundary) = newest_boundary(&scratch.0)?;
    assert_eq!(boundary.rows.len(), expected.levels.len());
    Ok(())
}

/// A first boundary whose record could not be admitted leaves only depth-1
/// chunks, which follow no boundary: the next walk starts fresh, and the
/// refusal named the record's size and the admission it exceeded.
#[test]
fn a_first_boundary_that_never_landed_restarts_and_its_refusal_names_the_sizes()
-> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let ladder = Ladder::with_min_hits(1).with_support_lanes(1);
    let column = column()?;
    let tight = Limits {
        entry: 32 + 64 + ENVELOPE,
        chunk: chunk(64),
    };
    let first = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let refused = walk_within(
        &scratch.0,
        &first,
        ladder,
        &column,
        &POSITIONS,
        tight,
        &mut |_| Ok(()),
    )
    .err()
    .ok_or("a boundary record over the admission must refuse")?;
    assert!(
        refused.contains("boundary record at depth 1 needs")
            && refused.contains("over the 192 byte journal entry admission"),
        "{refused}"
    );
    drop(first);
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    let orphan = journal.latest(MAX_BYTES)?.ok_or("depth-1 chunks landed")?;
    let [_, depth, index, previous] = chunk_header(&orphan.payload)?;
    assert_eq!(
        [depth, index, previous],
        [1, 7, 0],
        "the last of level 1's eight chunks"
    );
    drop(journal);

    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let mut visited = Vec::new();
    let walked = walk_within(
        &scratch.0,
        &attempt,
        ladder,
        &column,
        &POSITIONS,
        SMALL,
        &mut |view| {
            visited.push(view.current().k);
            Ok(())
        },
    )?;
    assert_eq!(
        walked,
        ladder.walk_column(&column, &POSITIONS, &|_, _, _| {})
    );
    assert_eq!(visited.first().copied(), Some(1), "a fresh walk");
    Ok(())
}

/// A boundary record exactly one entry's admission long lands; the admission
/// is inclusive. Level 1 is eight chunks of at most 64 bytes, published as
/// sequences 1 to 8, and its record at sequence 9 is 760 bytes, which with the
/// journal's 96-byte envelope is the whole admission. The depth-2 record is
/// longer and refuses, naming its depth.
#[test]
fn a_boundary_record_exactly_one_entry_long_lands() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let exact = Limits {
        entry: 760 + ENVELOPE,
        chunk: chunk(64),
    };
    let refused = walk_within(
        &scratch.0,
        &attempt,
        Ladder::with_min_hits(1),
        &column()?,
        &POSITIONS,
        exact,
        &mut |_| Ok(()),
    )
    .err()
    .ok_or("the depth-2 boundary record must refuse")?;
    assert!(
        refused.contains("boundary record at depth 2 needs"),
        "{refused}"
    );
    drop(attempt);
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    let saved = journal.read(9, MAX_BYTES)?;
    assert_eq!(saved.payload.len(), 760);
    let boundary = decode_boundary(&saved.payload, 9)?;
    assert_eq!(boundary.rows.len(), 1);
    assert_eq!(boundary.levels.iter().map(Vec::len).sum::<usize>(), 8);
    Ok(())
}

#[test]
fn depth_evidence_failure_stops_after_the_just_published_recovery_point() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let identity = identity_hex()?;
    let path = scratch
        .0
        .join("results/sweep-evidence-v1")
        .join(identity)
        .join(format!("{}-levels.bin", attempt.token()));
    fs::create_dir(&path).map_err(error)?;
    let mut calls = 0;
    let result = walk(
        &scratch.0,
        &attempt,
        Ladder::with_min_hits(1),
        &column()?,
        &POSITIONS,
        &mut |_| {
            calls += 1;
            Ok(())
        },
    );
    assert!(result.is_err());
    assert_eq!(
        calls, 0,
        "failed durable depth append prevents the observer and next level"
    );
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    let saved = journal
        .latest(MAX_BYTES)?
        .ok_or("first boundary should already be recoverable")?;
    assert_eq!(saved.sequence, 2, "one chunk, then its boundary");
    let boundary = decode_boundary(&saved.payload, saved.sequence)?;
    assert_eq!(boundary.rows.len(), 1);
    let checkpoint = replay(&journal, &boundary, ID, PRODUCTION)?;
    assert_eq!(checkpoint.levels().len(), 1);
    assert!(attempt.finish(Completion::Completed).is_err());
    Ok(())
}

/// A SMALL walk paused after its depth-2 boundary: level 1 is one chunk
/// (sequence 1) and level 2 two (sequences 3 and 4), named by the boundary
/// record at sequence 5.
fn paused_at_depth_two(
    scratch: &Scratch,
) -> Result<(Journal, crate::search_checkpoint::Saved, Boundary), String> {
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    assert!(
        walk_within(
            &scratch.0,
            &attempt,
            Ladder::with_min_hits(1),
            &column()?,
            &POSITIONS,
            SMALL,
            &mut |view| if view.current().k == 2 {
                Err("pause".into())
            } else {
                Ok(())
            }
        )
        .is_err()
    );
    drop(attempt);
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    let saved = journal.latest(MAX_BYTES)?.ok_or("checkpoint")?;
    assert_eq!(saved.sequence, 5);
    let boundary = decode_boundary(&saved.payload, saved.sequence)?;
    assert_eq!(boundary.rows.len(), 2);
    assert_eq!(
        boundary.levels.iter().map(Vec::len).collect::<Vec<_>>(),
        [1, 2]
    );
    replay(&journal, &boundary, ID, SMALL)?;
    Ok((journal, saved, boundary))
}

/// Every count, length and order in a boundary record is exact: each altered
/// alone refuses in its own words, and nothing is truncated or padded.
#[test]
fn boundary_counts_lengths_and_order_refuse_exactly() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let (journal, saved, boundary) = paused_at_depth_two(&scratch)?;
    let pieces_at = BOUNDARY_HEADER + 2 * DEPTH_BYTES + boundary.prefix.len();
    // The word after level 1's one chunk is level 2's chunk count.
    for (offset, word, refusal) in [
        (0, 0, "unsupported AND checkpoint boundary version"),
        (8, 0, LENGTHS),
        (8, 386, LENGTHS),
        (16, 0, LENGTHS),
        (24, 1, LENGTHS),
        (24, 4, LENGTHS),
        (
            BOUNDARY_HEADER + 8 * 8,
            2,
            "invalid depth reconciliation byte",
        ),
        (pieces_at, 0, LEVEL),
        (pieces_at, 4, LEVEL),
        (pieces_at + 8 + PIECE_BYTES, 3, LEVEL),
        (pieces_at + 8 + PIECE_BYTES, 1, UNNAMED),
        (pieces_at + 8, 0, ORDER),
        (pieces_at + 8, 3, ORDER),
        (pieces_at + 8 + 8, 0, ORDER),
    ] {
        let mut altered = saved.payload.clone();
        altered
            .get_mut(offset..offset + 8)
            .ok_or("fixture field")?
            .copy_from_slice(&u64::to_le_bytes(word));
        assert_eq!(
            decode_boundary(&altered, saved.sequence).err().as_deref(),
            Some(refusal),
            "offset {offset} word {word}"
        );
    }
    // Level 1 naming two chunks misreads what follows it and still refuses.
    let mut altered = saved.payload.clone();
    altered
        .get_mut(pieces_at..pieces_at + 8)
        .ok_or("fixture field")?
        .copy_from_slice(&u64::to_le_bytes(2));
    assert!(
        decode_boundary(&altered, saved.sequence)
            .and_then(|decoded| replay(&journal, &decoded, ID, SMALL))
            .is_err()
    );
    assert_eq!(
        decode_boundary(&saved.payload, 1).err().as_deref(),
        Some(UNNAMED),
        "chunks must precede their boundary"
    );
    Ok(())
}

/// Replay reads each named chunk only as its boundary record describes it: a
/// changed seal, a changed length or levels in the wrong depth order refuse
/// as a chunk that differs from its record, and a foreign identity refuses.
#[test]
fn replayed_chunks_must_match_their_seals_lengths_and_depths() -> Result<(), String> {
    const DIFFERS: &str = "AND checkpoint chunk differs from its boundary record";
    let scratch = Scratch::new().map_err(error)?;
    let (journal, _, boundary) = paused_at_depth_two(&scratch)?;
    let with = |levels: Vec<Vec<Piece>>| Boundary {
        rows: boundary.rows.clone(),
        prefix: boundary.prefix.clone(),
        levels,
    };
    let mut resealed = boundary.levels.clone();
    resealed
        .get_mut(1)
        .and_then(|level| level.first_mut())
        .ok_or("level 2 chunk")?
        .seal = [0; 32];
    let mut stretched = boundary.levels.clone();
    stretched
        .get_mut(1)
        .and_then(|level| level.get_mut(1))
        .ok_or("level 2 second chunk")?
        .length += 1;
    let swapped = boundary.levels.iter().rev().cloned().collect();
    for (case, levels) in [
        ("seal", resealed),
        ("length", stretched),
        ("depth order", swapped),
    ] {
        let refused = replay(&journal, &with(levels), ID, SMALL)
            .err()
            .ok_or(case)?;
        assert!(refused.contains(DIFFERS), "{case}: {refused}");
    }
    assert!(replay(&journal, &boundary, [0; 32], SMALL).is_err());
    Ok(())
}

/// A boundary record of zeros under a header naming these counts, sized
/// exactly as those counts require.
fn forged(count: usize, prefix: usize, pieces: usize) -> Result<Vec<u8>, String> {
    let bytes = boundary_bytes(count, prefix, pieces).ok_or("forged size")?;
    let mut out = vec![0; bytes];
    for (index, word) in [
        u64::from_le_bytes(BOUNDARY_MAGIC),
        count as u64,
        prefix as u64,
        pieces as u64,
    ]
    .into_iter()
    .enumerate()
    {
        out.get_mut(index * 8..index * 8 + 8)
            .ok_or("forged header")?
            .copy_from_slice(&word.to_le_bytes());
    }
    Ok(out)
}

/// Counts whose record length agrees with them are still held to their
/// bounds: no rows, more than `MAX_ROWS`, and fewer chunks than rows each
/// refuse as invalid lengths. At exactly `MAX_ROWS` the counts pass and the
/// zeroed first level is what refuses. A byte past the counted length refuses
/// as invalid lengths too.
#[test]
fn boundary_counts_are_held_to_their_bounds_where_the_length_agrees() -> Result<(), String> {
    for (count, prefix, pieces, refusal) in [
        (0, 8, 0, LENGTHS),
        (MAX_ROWS + 1, 0, MAX_ROWS + 1, LENGTHS),
        (2, 0, 1, LENGTHS),
        (MAX_ROWS, 0, MAX_ROWS, LEVEL),
    ] {
        assert_eq!(
            decode_boundary(&forged(count, prefix, pieces)?, u64::MAX)
                .err()
                .as_deref(),
            Some(refusal),
            "{count} rows, {prefix} prefix bytes, {pieces} chunks"
        );
    }
    let mut longer = forged(1, 0, 1)?;
    longer.push(0);
    assert_eq!(
        decode_boundary(&longer, u64::MAX).err().as_deref(),
        Some(LENGTHS)
    );
    Ok(())
}

/// An entry recovery cannot place refuses the walk rather than being skipped
/// or treated as an empty journal.
#[test]
fn an_entry_recovery_cannot_place_refuses() -> Result<(), String> {
    let chunk = |depth: u64, previous: u64| {
        [u64::from_le_bytes(CHUNK_MAGIC), depth, 0, previous]
            .iter()
            .flat_map(|word| word.to_le_bytes())
            .chain([0; 56])
            .collect::<Vec<u8>>()
    };
    for (payload, refusal) in [
        (
            b"not a checkpoint entry".to_vec(),
            "unsupported AND checkpoint entry",
        ),
        (
            chunk(2, 0),
            "AND orphan chunk follows no boundary but is not depth 1",
        ),
        (chunk(1, 9), "AND orphan chunk names a later boundary"),
        (chunk(2, 1), "AND orphan chunk names a later boundary"),
    ] {
        let scratch = Scratch::new().map_err(error)?;
        let mut journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
        assert_eq!(journal.publish(&payload, MAX_BYTES)?.0, 1);
        drop(journal);
        let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
        let refused = walk(
            &scratch.0,
            &attempt,
            Ladder::with_min_hits(1),
            &column()?,
            &POSITIONS,
            &mut |_| Ok(()),
        )
        .err()
        .ok_or(refusal)?;
        assert_eq!(refused, refusal);
    }
    // A chunk after a real boundary must be the level after it.
    let scratch = Scratch::new().map_err(error)?;
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    walk(
        &scratch.0,
        &attempt,
        Ladder::with_min_hits(1),
        &column()?,
        &POSITIONS,
        &mut |view| {
            if view.current().k == 2 {
                Err("pause".into())
            } else {
                Ok(())
            }
        },
    )
    .err()
    .ok_or("paused")?;
    drop(attempt);
    let (last, _) = newest_boundary(&scratch.0)?;
    let mut journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    journal.publish(&chunk(5, last), MAX_BYTES)?;
    drop(journal);
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let refused = walk(
        &scratch.0,
        &attempt,
        Ladder::with_min_hits(1),
        &column()?,
        &POSITIONS,
        &mut |_| Ok(()),
    )
    .err()
    .ok_or("a stray level must refuse")?;
    assert!(
        refused.contains("not the level after its boundary"),
        "{refused}"
    );
    Ok(())
}

#[test]
fn a_chunk_that_cannot_fit_one_entry_refuses_before_publishing() -> Result<(), String> {
    // One byte short of a 64-byte chunk, its 32-byte header and the envelope.
    let limits = Limits {
        entry: 32 + 64 + ENVELOPE - 1,
        chunk: chunk(64),
    };
    let scratch = Scratch::new().map_err(error)?;
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let refused = walk_within(
        &scratch.0,
        &attempt,
        Ladder::with_min_hits(1),
        &column()?,
        &POSITIONS,
        limits,
        &mut |_| Ok(()),
    )
    .err()
    .ok_or("an unfit chunk must refuse")?;
    assert!(
        refused.contains("does not fit one journal entry"),
        "{refused}"
    );
    drop(attempt);
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    assert_eq!(journal.next_sequence(), 1, "nothing was published");
    Ok(())
}

#[test]
fn actual_ranked_helper_replays_durable_history_and_refuses_unwarmed_data() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let bars = runner::synthetic::sessions(8);
    let column = Column::build(&bars, &mut crate::evaluator().map_err(error)?);
    let forward = runner::outcome::forward(&bars, &column, crate::Horizon::DEFAULT);
    let ladder = Ladder::with_min_hits(600)
        .with_ceiling(50_000)
        .with_support_lanes(1);
    let first = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let expected = run(
        &scratch.0,
        &first,
        ladder,
        column.clone(),
        &column,
        &forward,
    )?;
    assert!(expected.outcome.is_complete());
    first.finish(Completion::Completed)?;
    let second = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let resumed = run(
        &scratch.0,
        &second,
        ladder,
        column.clone(),
        &column,
        &forward,
    )?;
    assert_eq!(resumed.ranked.top, expected.ranked.top);
    assert_eq!(resumed.ranked.closed_top, expected.ranked.closed_top);
    assert_eq!(resumed.outcome.sweep.levels, expected.outcome.sweep.levels);
    assert_eq!(resumed.outcome.trials, expected.outcome.trials);
    second.finish(Completion::Completed)?;
    let cold = Column::build(&[], &mut crate::evaluator().map_err(error)?);
    let refused = sweep_evidence::begin(&scratch.0, [8; 32], Operation::Sweep)?;
    assert!(run(&scratch.0, &refused, ladder, cold, &column, &forward).is_err());
    Ok(())
}

/// The final boundary and every chunk it names are reopened before a
/// rankable run is returned: changing either after the terminal callback
/// refuses the attempt.
#[test]
fn final_checkpoint_corruption_after_callback_cannot_return_a_completed_sweep() -> Result<(), String>
{
    for damage_boundary in [true, false] {
        let scratch = Scratch::new().map_err(error)?;
        let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
        let directory = namespace(&scratch.0)?;
        let result = walk_within(
            &scratch.0,
            &attempt,
            Ladder::with_min_hits(1),
            &column()?,
            &POSITIONS,
            SMALL,
            &mut |view| {
                if view.terminal() {
                    let sequence = if damage_boundary {
                        fs::read_dir(&directory)
                            .map_err(error)?
                            .filter_map(|entry| {
                                u64::from_str_radix(
                                    &entry.ok()?.file_name().into_string().ok()?,
                                    16,
                                )
                                .ok()
                            })
                            .max()
                            .ok_or("no entry")?
                    } else {
                        1
                    };
                    fs::write(
                        directory.join(format!("{sequence:016x}")).join("payload"),
                        b"changed after acknowledged callback",
                    )
                    .map_err(error)?;
                }
                Ok(())
            },
        );
        assert!(result.is_err(), "boundary damaged {damage_boundary}");
        drop(attempt);
        let evidence = sweep_evidence::read(&scratch.0, ID, MAX_BYTES)?.ok_or("attempt missing")?;
        assert_eq!(evidence.completion, Completion::Refused);
    }
    Ok(())
}

/// Replaces entry `sequence` of this identity's journal under `root` with
/// `payload`, VALIDLY sealed: published at the same sequence through a second
/// journal under another root and copied over, header, seal and completion
/// marker alike. The journal's own read accepts it; only a comparison with the
/// seal recorded when the original was published can tell them apart.
fn reseal(root: &Path, sequence: u64, payload: &[u8]) -> Result<(), String> {
    let other = Scratch::new().map_err(error)?;
    let mut journal = Journal::open(&other.0, NAMESPACE, ID)?;
    while journal.next_sequence() < sequence {
        journal.publish(b"filler", MAX_BYTES)?;
    }
    assert_eq!(journal.publish(payload, MAX_BYTES)?.0, sequence);
    drop(journal);
    let from = namespace(&other.0)?.join(format!("{sequence:016x}"));
    let to = namespace(root)?.join(format!("{sequence:016x}"));
    for file in ["payload", "complete"] {
        fs::copy(from.join(file), to.join(file)).map_err(error)?;
    }
    Ok(())
}

/// The payload of entry `sequence`, read from its file around the journal's
/// 64-byte header and 32-byte seal, while the walk owns the journal.
fn payload_of(root: &Path, sequence: u64) -> Result<Vec<u8>, String> {
    let bytes = fs::read(
        namespace(root)?
            .join(format!("{sequence:016x}"))
            .join("payload"),
    )
    .map_err(error)?;
    Ok(bytes
        .get(64..bytes.len().saturating_sub(32))
        .ok_or("entry payload")?
        .to_vec())
}

/// **A replacement that is itself validly sealed is refused by the recorded
/// seals, not by the journal.** After the terminal callback, the final
/// boundary record is replaced by one whose engine prefix differs in one byte,
/// or level 1's chunk by one whose level bytes do, each resealed at its own
/// sequence so that the journal's read accepts it. The walk refuses in the
/// comparison's own words, the attempt is refused, and the journal still
/// reads the replacement back: nothing but the comparison with the seal
/// acknowledged at publication refused it.
#[test]
fn a_resealed_final_boundary_or_chunk_is_refused_by_its_recorded_seal() -> Result<(), String> {
    for (replace_boundary, refusal) in [
        (
            true,
            "AND final checkpoint differs from its acknowledged publication",
        ),
        (
            false,
            "AND final checkpoint chunk differs from its acknowledgment",
        ),
    ] {
        let scratch = Scratch::new().map_err(error)?;
        let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
        let mut replaced = None;
        let result = walk_within(
            &scratch.0,
            &attempt,
            Ladder::with_min_hits(1),
            &column()?,
            &POSITIONS,
            SMALL,
            &mut |view| {
                if !view.terminal() {
                    return Ok(());
                }
                let sequence = if replace_boundary {
                    fs::read_dir(namespace(&scratch.0)?)
                        .map_err(error)?
                        .filter_map(|entry| {
                            u64::from_str_radix(&entry.ok()?.file_name().into_string().ok()?, 16)
                                .ok()
                        })
                        .max()
                        .ok_or("no entry")?
                } else {
                    1
                };
                let mut payload = payload_of(&scratch.0, sequence)?;
                let at = if replace_boundary {
                    // The engine prefix's first byte, after the header and
                    // every depth row; the record still decodes.
                    let rows = decode_boundary(&payload, sequence)?.rows.len();
                    BOUNDARY_HEADER + rows * DEPTH_BYTES
                } else {
                    // Level 1's first byte, after the chunk header.
                    CHUNK_HEADER
                };
                *payload.get_mut(at).ok_or("replaced byte")? ^= 1;
                if replace_boundary {
                    decode_boundary(&payload, sequence)?;
                }
                reseal(&scratch.0, sequence, &payload)?;
                replaced = Some((sequence, payload));
                Ok(())
            },
        );
        assert_eq!(result.err().as_deref(), Some(refusal));
        drop(attempt);
        let evidence = sweep_evidence::read(&scratch.0, ID, MAX_BYTES)?.ok_or("attempt missing")?;
        assert_eq!(evidence.completion, Completion::Refused);
        let (sequence, payload) = replaced.ok_or("nothing was replaced")?;
        let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
        assert_eq!(journal.read(sequence, MAX_BYTES)?.payload, payload);
    }
    Ok(())
}

/// **The reproduction at production size, through the production door.**
/// Twenty-one live conditions true together on two of three bars make every
/// one of their 2,097,151 combinations frequent, so the retained history
/// passes 64 MiB during depth 11. Version 1 refused there with "AND checkpoint
/// exceeds 64 MiB byte admission", after building the level, and a rerun
/// rebuilt it and refused again. Here the walk completes, and replaying the
/// completed identity makes one callback and publishes nothing. AC-whp-o1-0.
#[test]
fn a_production_history_past_64_mib_completes_and_replays_without_recomputing() -> Result<(), String>
{
    // The production door's sizes, checked before the walk they govern.
    assert_eq!(
        (PRODUCTION.entry, PRODUCTION.chunk.get()),
        (64 << 20, 32 << 20)
    );
    let positions = runner::live_positions()
        .into_iter()
        .take(21)
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 21);
    let full = positions
        .iter()
        .copied()
        .fold(ConditionMask::ZERO, ConditionMask::with_bit);
    let column =
        engine::column::Column::try_from_rows(&[full, full, ConditionMask::ZERO]).map_err(error)?;
    let ladder = Ladder::with_min_hits(1).with_support_lanes(1);
    let scratch = Scratch::new().map_err(error)?;
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let walked = walk(
        &scratch.0,
        &attempt,
        ladder,
        &column,
        &positions,
        &mut |_| Ok(()),
    )?;
    attempt.finish(Completion::Completed)?;
    assert!(walked.completed());
    let survivors = walked
        .levels
        .iter()
        .map(|level| level.frequent.len())
        .sum::<usize>();
    assert_eq!(survivors, (1 << 21) - 1);
    let history = walked
        .levels
        .iter()
        .map(|level| 56 + 56 * level.frequent.len() as u64)
        .sum::<u64>();
    assert!(history > MAX_BYTES, "{history} bytes of history");
    // Each level is saved as one chunk entry per 32 MiB of its own bytes.
    let (_, boundary) = newest_boundary(&scratch.0)?;
    assert_eq!(boundary.levels.len(), walked.levels.len());
    for (level, pieces) in walked.levels.iter().zip(&boundary.levels) {
        let bytes = 56 + 56 * level.frequent.len() as u64;
        assert_eq!(pieces.len() as u64, bytes.div_ceil(32 << 20));
        assert_eq!(pieces.iter().map(|piece| piece.length).sum::<u64>(), bytes);
    }

    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    let next = journal.next_sequence();
    drop(journal);
    let replay = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let mut callbacks = 0;
    let replayed = walk(
        &scratch.0,
        &replay,
        ladder,
        &column,
        &positions,
        &mut |_| {
            callbacks += 1;
            Ok(())
        },
    )?;
    replay.finish(Completion::Completed)?;
    assert_eq!(replayed, walked);
    assert_eq!(
        callbacks, 1,
        "the completed boundary is acknowledged, not rebuilt"
    );
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    assert_eq!(journal.next_sequence(), next, "replay publishes nothing");
    Ok(())
}
