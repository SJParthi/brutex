use super::*;
use crate::search_checkpoint::tests::Scratch;
use crate::sweep_evidence::{self, Completion, Operation};
use std::cell::RefCell;
use std::fs;
use vocab::ConditionMask;

const POSITIONS: [u32; 8] = [0, 63, 64, 127, 128, 192, 320, 369];
const ID: [u8; 32] = [0x37; 32];

fn column() -> Result<engine::column::Column, String> {
    let full = POSITIONS
        .into_iter()
        .fold(ConditionMask::ZERO, ConditionMask::with_bit);
    engine::column::Column::try_from_rows(&[full, full, ConditionMask::ZERO]).map_err(error)
}

#[test]
fn persisted_interrupt_restart_and_terminal_replay_preserve_every_depth_row() -> Result<(), String>
{
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
        let stopped = walk(
            &scratch.0,
            &first,
            ladder,
            &column,
            &POSITIONS,
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
        let resumed = walk(
            &scratch.0,
            &attempt,
            ladder,
            &column,
            &POSITIONS,
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
        let evidence =
            sweep_evidence::read(&scratch.0, ID, MAX_BYTES)?.ok_or("resumed attempt missing")?;
        assert_eq!(evidence.completion, completion);
        assert_eq!(evidence.depth_rows, expected.levels.len() as u64);
        assert_eq!(
            sweep_evidence::depth_page(&scratch.0, &evidence, 0, MAX_ROWS, MAX_BYTES)?,
            *expected_rows.borrow()
        );
        terminal_replay(&scratch.0, ladder, &column, &expected, completion)?;
    }
    Ok(())
}

fn terminal_replay(
    root: &Path,
    ladder: Ladder,
    column: &engine::column::Column,
    expected: &Sweep,
    completion: Completion,
) -> Result<(), String> {
    let journal = Journal::open(root, "and-checkpoint-v1", ID)?;
    let saved = journal
        .latest(MAX_BYTES)?
        .ok_or("missing completed checkpoint")?;
    let original = saved.payload.clone();
    let sequence = saved.sequence;
    drop(journal);
    let replay = sweep_evidence::begin(root, ID, Operation::Sweep)?;
    let mut callbacks = 0;
    assert_eq!(
        walk(root, &replay, ladder, column, &POSITIONS, &mut |_| {
            callbacks += 1;
            Ok(())
        })?,
        *expected
    );
    assert_eq!(callbacks, 1, "completed/partial terminal never advances");
    replay.finish(completion)?;
    let journal = Journal::open(root, "and-checkpoint-v1", ID)?;
    let latest = journal
        .latest(MAX_BYTES)?
        .ok_or("missing replay checkpoint")?;
    assert_eq!(
        latest.sequence, sequence,
        "replay is an acknowledgment, not a duplicate checkpoint"
    );
    assert_eq!(latest.payload, original);
    Ok(())
}

fn identity_hex() -> Result<String, String> {
    use std::fmt::Write as _;
    ID.iter().try_fold(String::new(), |mut text, byte| {
        write!(text, "{byte:02x}").map_err(error)?;
        Ok(text)
    })
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
    let journal = Journal::open(&scratch.0, "and-checkpoint-v1", ID)?;
    let saved = journal
        .latest(MAX_BYTES)?
        .ok_or("first boundary should already be recoverable")?;
    assert_eq!(saved.sequence, 1);
    let (checkpoint, rows) = decode(&saved.payload, ID)?;
    assert_eq!(checkpoint.levels().len(), 1);
    assert_eq!(rows.len(), 1);
    assert!(attempt.finish(Completion::Completed).is_err());
    Ok(())
}

#[test]
fn wrapper_counters_lengths_and_bounded_writer_refuse_without_truncation() -> Result<(), String> {
    let scratch = Scratch::new().map_err(error)?;
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    assert!(
        walk(
            &scratch.0,
            &attempt,
            Ladder::with_min_hits(1),
            &column()?,
            &POSITIONS,
            &mut |view| if view.current().k == 2 {
                Err("pause".into())
            } else {
                Ok(())
            }
        )
        .is_err()
    );
    let journal = Journal::open(&scratch.0, "and-checkpoint-v1", ID)?;
    let saved = journal.latest(MAX_BYTES)?.ok_or("checkpoint")?;
    assert_eq!(decode(&saved.payload, ID)?.1.len(), 2);
    for (offset, word) in [
        (0, 0),
        (8, 0),
        (8, 386),
        (16, 0),
        (HEADER + 6 * 8, 1),
        (HEADER + 7 * 8, 1),
        (HEADER + DEPTH_BYTES + 7 * 8, u64::MAX),
        (HEADER + 8 * 8, 2),
    ] {
        let mut altered = saved.payload.clone();
        altered
            .get_mut(offset..offset + 8)
            .ok_or("fixture field")?
            .copy_from_slice(&word.to_le_bytes());
        assert!(decode(&altered, ID).is_err(), "offset {offset}");
    }
    assert!(decode(&saved.payload, [0; 32]).is_err());
    let mut writer = BoundedBytes::new(7)?;
    writer.write_all(b"1234567").map_err(error)?;
    assert!(writer.write_all(b"8").is_err());
    assert_eq!(writer.bytes, b"1234567");
    writer.flush().map_err(error)?;
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

#[test]
fn final_checkpoint_corruption_after_callback_cannot_return_a_completed_sweep() -> Result<(), String>
{
    let scratch = Scratch::new().map_err(error)?;
    let attempt = sweep_evidence::begin(&scratch.0, ID, Operation::Sweep)?;
    let identity = identity_hex()?;
    let result = walk(
        &scratch.0,
        &attempt,
        Ladder::with_min_hits(1),
        &column()?,
        &POSITIONS,
        &mut |view| {
            if view.terminal() {
                let path = scratch
                    .0
                    .join("and-checkpoint-v1")
                    .join(&identity)
                    .join(format!("{:016x}", view.current().k))
                    .join("payload");
                fs::write(path, b"changed after acknowledged callback").map_err(error)?;
            }
            Ok(())
        },
    );
    assert!(result.is_err());
    drop(attempt);
    let evidence = sweep_evidence::read(&scratch.0, ID, MAX_BYTES)?.ok_or("attempt missing")?;
    assert_eq!(evidence.completion, Completion::Refused);
    Ok(())
}
