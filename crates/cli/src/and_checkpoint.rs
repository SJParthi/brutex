//! Durable retained AND checkpoints on the actual stored-sweep path.
//! The envelope preserves the exact historical depth counters; none are
//! reconstructed from later totals or fabricated during attempt recovery.

use std::io::{self, Read as _, Write};
use std::path::Path;

use engine::resume::{Checkpoint, CheckpointView};
use engine::{Ladder, Sweep};
use indicators::column::Column;

use crate::search_checkpoint::Journal;
use crate::sweep_evidence::{Attempt, DepthRow};

const MAX_BYTES: u64 = 64 * 1024 * 1024;
const MAGIC: [u8; 8] = *b"BRTXAN01";
const HEADER: usize = 24;
const DEPTH_BYTES: usize = 80;
// Wire masks contain six words; at most 384 nonempty depths and their witness.
const MAX_ROWS: usize = 385;

pub(crate) fn run(
    root: &Path,
    attempt: &Attempt,
    ladder: Ladder,
    column: Column,
    scoring_column: &Column,
    forward: &runner::outcome::Forward,
) -> Result<runner::RankedRun, String> {
    let census = column.census();
    if census.swept == 0
        || column.first_swept().is_none()
        || !census.reconciles()
        || census.swept != u64::try_from(column.bits().len()).map_err(error)?
    {
        return Err("AND checkpoint sweep requires a measured, reconciled signal column".into());
    }
    let bits = engine::column::Column::try_from_rows(column.bits()).map_err(error)?;
    let sweep = walk(
        root,
        attempt,
        ladder,
        &bits,
        &runner::live_positions(),
        &mut |view| {
            crate::emit_ladder_level(view.current(), view.admitted(), view.pairs());
            Ok(())
        },
    )?;
    runner::rank_checkpointed_sweep(
        column,
        Some(scoring_column),
        forward,
        sweep,
        crate::STORED_KEEP,
        runner::rank::Lens::Detectability,
    )
}

fn walk(
    root: &Path,
    attempt: &Attempt,
    ladder: Ladder,
    column: &engine::column::Column,
    live: &[u32],
    after_saved: &mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>,
) -> Result<Sweep, String> {
    attempt.check()?;
    let mut journal = Journal::open(root, "and-checkpoint-v1", attempt.identity())?;
    let saved = journal.latest(MAX_BYTES)?;
    let mut acknowledged = saved.as_ref().map(|saved| (saved.sequence, saved.seal));
    let (checkpoint, mut rows) = match saved {
        Some(saved) => {
            let (checkpoint, rows) = decode(&saved.payload, attempt.identity())?;
            checkpoint
                .validate_for(ladder, column, live, attempt.identity())
                .map_err(error)?;
            crate::note(
                &telemetry::Event::info("cli.sweep", "AND checkpoint recovered")
                    .with("sequence", saved.sequence)
                    .with("depth", u64::try_from(rows.len()).map_err(error)?)
                    .with("interrupted_reservations", journal.interrupted()),
            );
            (Some(checkpoint), rows)
        }
        None => (None, Vec::new()),
    };
    rows.try_reserve_exact(MAX_ROWS.saturating_sub(rows.len()))
        .map_err(error)?;
    // The current boundary is emitted once by the engine's resume callback.
    // Only earlier rows are rehydrated here, with their original counters.
    if checkpoint.is_some() {
        for row in rows.iter().take(rows.len().saturating_sub(1)) {
            attempt.level(*row)?;
        }
    }
    let mut checkpointed = |view: &CheckpointView<'_>| {
        let row = DepthRow::of(view.current(), view.admitted(), view.pairs());
        let depth = usize::try_from(row.k).map_err(error)?;
        if depth == rows.len() {
            if rows.last() != Some(&row) {
                return Err("resumed AND boundary changed exact depth counters".into());
            }
        } else if depth == rows.len().saturating_add(1) {
            rows.push(row);
            let payload = encode(view, &rows)?;
            acknowledged = Some(journal.publish(&payload, MAX_BYTES)?);
        } else {
            return Err("AND checkpoint depth sequence is discontinuous".into());
        }
        // If this fails, the just-published checkpoint remains a valid recovery
        // point but this attempt is refused. The next depth is never started.
        attempt.level(row)?;
        after_saved(view)
    };
    let sweep = match checkpoint {
        Some(checkpoint) => ladder.resume_checkpointed(
            column,
            live,
            attempt.identity(),
            checkpoint,
            &mut checkpointed,
        ),
        None => ladder.walk_checkpointed(column, live, attempt.identity(), &mut checkpointed),
    }
    .map_err(error)?;
    // Reopen the final self-contained history before returning a rankable run.
    // Checking the exact acknowledged seal also rejects a newly resealed
    // replacement, not only accidental byte corruption.
    let final_saved = journal
        .latest(MAX_BYTES)?
        .ok_or("AND final checkpoint is absent")?;
    if Some((final_saved.sequence, final_saved.seal)) != acknowledged {
        return Err("AND final checkpoint differs from its acknowledged publication".into());
    }
    Ok(sweep)
}

fn encode(view: &CheckpointView<'_>, rows: &[DepthRow]) -> Result<Vec<u8>, String> {
    let mut writer = BoundedBytes::new(MAX_BYTES - 96)?;
    writer.write_all(&MAGIC).map_err(error)?;
    writer
        .write_all(&u64::try_from(rows.len()).map_err(error)?.to_le_bytes())
        .map_err(error)?;
    writer.write_all(&[0; 8]).map_err(error)?;
    for row in rows {
        for word in row.words() {
            writer.write_all(&word.to_le_bytes()).map_err(error)?;
        }
    }
    let start = writer.bytes.len();
    view.write_to(&mut writer).map_err(error)?;
    let length = u64::try_from(writer.bytes.len() - start).map_err(error)?;
    writer
        .bytes
        .get_mut(16..24)
        .ok_or("AND checkpoint header range")?
        .copy_from_slice(&length.to_le_bytes());
    Ok(writer.bytes)
}

fn decode(bytes: &[u8], identity: [u8; 32]) -> Result<(Checkpoint, Vec<DepthRow>), String> {
    let mut input = bytes;
    let mut magic = [0; 8];
    input.read_exact(&mut magic).map_err(error)?;
    if magic != MAGIC {
        return Err("unsupported AND checkpoint wrapper version".into());
    }
    let count = usize::try_from(read_word(&mut input)?).map_err(error)?;
    let engine_bytes = usize::try_from(read_word(&mut input)?).map_err(error)?;
    if count == 0
        || count > MAX_ROWS
        || count
            .checked_mul(DEPTH_BYTES)
            .and_then(|n| n.checked_add(HEADER))
            .and_then(|n| n.checked_add(engine_bytes))
            != Some(bytes.len())
    {
        return Err("AND checkpoint wrapper has invalid exact lengths".into());
    }
    let mut rows = Vec::new();
    rows.try_reserve_exact(count).map_err(error)?;
    for _ in 0..count {
        let mut words = [0; 10];
        for word in &mut words {
            *word = read_word(&mut input)?;
        }
        rows.push(DepthRow::from_words(words)?);
    }
    let checkpoint = Checkpoint::read_from(
        &mut input,
        u64::try_from(engine_bytes).map_err(error)?,
        identity,
    )
    .map_err(error)?;
    validate_rows(&checkpoint, &rows)?;
    Ok((checkpoint, rows))
}

fn validate_rows(checkpoint: &Checkpoint, rows: &[DepthRow]) -> Result<(), String> {
    if rows.len() != checkpoint.levels().len() {
        return Err("AND checkpoint depth history length mismatch".into());
    }
    let mut admitted = 0_u64;
    let mut pairs = 0_u64;
    for (offset, (level, row)) in checkpoint.levels().iter().zip(rows).enumerate() {
        if offset > 0 {
            admitted = admitted
                .checked_add(level.generated)
                .ok_or("AND admitted counter overflow")?;
        }
        let expected = DepthRow::of(level, usize::try_from(admitted).map_err(error)?, row.pairs);
        if expected != *row
            || row.pairs < pairs
            || row.pairs < admitted
            || (offset == 0 && row.pairs != 0)
        {
            return Err("AND checkpoint depth history differs from the retained search".into());
        }
        pairs = row.pairs;
    }
    if admitted != u64::try_from(checkpoint.admitted()).map_err(error)?
        || pairs != checkpoint.pairs()
    {
        return Err("AND checkpoint final counters disagree".into());
    }
    Ok(())
}

fn read_word(input: &mut &[u8]) -> Result<u64, String> {
    let mut bytes = [0; 8];
    input.read_exact(&mut bytes).map_err(error)?;
    Ok(u64::from_le_bytes(bytes))
}

struct BoundedBytes {
    bytes: Vec<u8>,
    limit: usize,
}
impl BoundedBytes {
    fn new(limit: u64) -> Result<Self, String> {
        Ok(Self {
            bytes: Vec::new(),
            limit: usize::try_from(limit).map_err(error)?,
        })
    }
}
impl Write for BoundedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let needed = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .filter(|&n| n <= self.limit)
            .ok_or_else(|| io::Error::other("AND checkpoint exceeds 64 MiB byte admission"))?;
        if needed > self.bytes.capacity() {
            let reserved = needed
                .max(self.bytes.capacity().saturating_mul(2))
                .min(self.limit);
            self.bytes
                .try_reserve_exact(reserved - self.bytes.len())
                .map_err(io::Error::other)?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "and_checkpoint_tests.rs"]
mod tests;
