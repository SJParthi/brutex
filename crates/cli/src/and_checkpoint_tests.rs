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
    chunk: 1024,
};

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
            bytes.div_ceil(SMALL.chunk as u64),
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
        chunk: 64,
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

/// Every count, length, order and seal in a boundary record is exact: each
/// altered alone refuses, and nothing is truncated or padded.
#[test]
fn boundary_counts_lengths_order_and_seals_refuse_exactly() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
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
    let journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
    let saved = journal.latest(MAX_BYTES)?.ok_or("checkpoint")?;
    let boundary = decode_boundary(&saved.payload, saved.sequence)?;
    assert_eq!(boundary.rows.len(), 2);
    assert_eq!(
        boundary.levels.iter().map(Vec::len).collect::<Vec<_>>(),
        [1, 2]
    );
    replay(&journal, &boundary, ID, SMALL)?;
    let prefix = boundary.prefix.len();
    let pieces_at = BOUNDARY_HEADER + 2 * DEPTH_BYTES + prefix;
    for (offset, word) in [
        (0, 0),
        (8, 0),
        (8, 386),
        (16, 0),
        (24, 1),
        (24, 4),
        (BOUNDARY_HEADER + 8 * 8, 2),
        (pieces_at, 0),
        (pieces_at, 2),
        (pieces_at + 8, 0),
        (pieces_at + 8, 3),
        (pieces_at + 8 + 8, 0),
    ] {
        let mut altered = saved.payload.clone();
        altered
            .get_mut(offset..offset + 8)
            .ok_or("fixture field")?
            .copy_from_slice(&u64::to_le_bytes(word));
        assert!(
            decode_boundary(&altered, saved.sequence)
                .and_then(|decoded| replay(&journal, &decoded, ID, SMALL))
                .is_err(),
            "offset {offset} word {word}"
        );
    }
    assert!(
        decode_boundary(&saved.payload, 1).is_err(),
        "chunks must precede their boundary"
    );
    let mut resealed = boundary.levels.clone();
    let seal = resealed
        .get_mut(1)
        .and_then(|level| level.first_mut())
        .ok_or("level 2 chunk")?;
    seal.seal = [0; 32];
    let forged = Boundary {
        rows: boundary.rows.clone(),
        prefix: boundary.prefix.clone(),
        levels: resealed,
    };
    assert!(replay(&journal, &forged, ID, SMALL).is_err(), "seal");
    let mut stretched = boundary.levels.clone();
    stretched
        .get_mut(1)
        .and_then(|level| level.get_mut(1))
        .ok_or("level 2 second chunk")?
        .length += 1;
    let forged = Boundary {
        rows: boundary.rows.clone(),
        prefix: boundary.prefix.clone(),
        levels: stretched,
    };
    assert!(replay(&journal, &forged, ID, SMALL).is_err(), "length");
    let swapped = Boundary {
        rows: boundary.rows.clone(),
        prefix: boundary.prefix.clone(),
        levels: boundary.levels.iter().rev().cloned().collect(),
    };
    assert!(
        replay(&journal, &swapped, ID, SMALL).is_err(),
        "depth order"
    );
    assert!(replay(&journal, &boundary, [0; 32], SMALL).is_err());
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
    for (case, payload) in [
        ("unknown magic", b"not a checkpoint entry".to_vec()),
        ("depth-2 chunk after no boundary", chunk(2, 0)),
        ("chunk naming a later boundary", chunk(1, 9)),
    ] {
        let scratch = Scratch::new().map_err(error)?;
        let mut journal = Journal::open(&scratch.0, NAMESPACE, ID)?;
        journal.publish(&payload, MAX_BYTES)?;
        drop(journal);
        let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
        assert!(
            walk(
                &scratch.0,
                &attempt,
                Ladder::with_min_hits(1),
                &column()?,
                &POSITIONS,
                &mut |_| Ok(()),
            )
            .is_err(),
            "{case}"
        );
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
    for limits in [
        Limits {
            entry: MAX_BYTES,
            chunk: 0,
        },
        Limits {
            entry: 32 + 64 + ENVELOPE - 1,
            chunk: 64,
        },
    ] {
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
    }
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
