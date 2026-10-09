//! Durable retained AND checkpoints on the actual stored-sweep path.
//! The envelope preserves the exact historical depth counters; none are
//! reconstructed from later totals or fabricated during attempt recovery.
//!
//! # Version 2: each boundary saves its own level, never the whole history
//!
//! Version 1 (`and-checkpoint-v1`, `BRTXAN01`) re-encoded EVERY retained level
//! into one in-memory buffer at every level boundary and published it as one
//! journal entry under a 64 MiB admission. So each boundary wrote the whole
//! history again, and a history past that admission refused after the level
//! that crossed it had been built. Reproduced on 21 conditions true together:
//! the 1,048,575 survivors through depth 10 were saved, depth 11 was built and
//! refused with "AND checkpoint exceeds 64 MiB byte admission", and a rerun
//! resumed at depth 10, rebuilt depth 11 and refused again.
//!
//! Version 2 (`and-checkpoint-v2`) publishes, at the boundary for depth `k`,
//! only level `k`'s engine bytes, split into chunk entries of at most
//! [`CHUNK_BYTES`] each, and then one boundary record: every depth row, this
//! boundary's engine prefix (header, offers, exclusions) and, for every level,
//! the sequence, length and seal of each chunk that holds it. Resuming reads
//! the newest boundary record and streams its prefix and every named chunk,
//! each checked against its recorded seal, through the engine's own decoder.
//! No entry holds more than one chunk of one level. D-0712.
//!
//! # A resume restores one level at a time
//!
//! The decoder hands the walk each restored level as it reaches it, checked
//! against its depth row, and the ranker takes it and drops it before the
//! level after the next is read: a resume holds at most two restored levels
//! and one chunk, where it held the whole restored history until D-4520
//! (AC-whp-o1-1).

use std::io::{self, Read, Write};
use std::num::NonZeroUsize;
use std::path::Path;

use engine::Frontier;
use engine::Ladder;
#[cfg(test)]
use engine::Sweep;
use engine::resume::{Checkpoint, CheckpointView, Restoring};
use indicators::column::Column;

use crate::search_checkpoint::{Entries, Journal};
use crate::sweep_evidence::{Attempt, DepthRow};

const NAMESPACE: &str = "and-checkpoint-v2";
/// The byte admission of one journal entry, chunk or boundary record.
const MAX_BYTES: u64 = 64 * 1024 * 1024;
/// Level bytes one chunk entry carries in production: half an entry's
/// admission, so a chunk and its header always fit. Written as one plus the
/// rest, which is nonzero by type, so that gate 11's rule 5 finds no `unwrap`.
const CHUNK_BYTES: NonZeroUsize = NonZeroUsize::MIN.saturating_add(32 * 1024 * 1024 - 1);
const CHUNK_MAGIC: [u8; 8] = *b"BRTXAC02";
const BOUNDARY_MAGIC: [u8; 8] = *b"BRTXAB02";
/// Magic, depth, chunk index within the level, previous boundary sequence.
const CHUNK_HEADER: usize = 32;
/// Magic, depth-row count, engine-prefix length, total chunk count.
const BOUNDARY_HEADER: usize = 32;
const DEPTH_BYTES: usize = 80;
/// Sequence, length and seal of one chunk.
const PIECE_BYTES: usize = 48;
/// The journal's own envelope around a payload.
const ENVELOPE: u64 = 96;
// Wire masks contain six words; at most 384 nonempty depths and their witness.
const MAX_ROWS: usize = 385;

/// The two sizes a walk publishes under. Production uses [`PRODUCTION`];
/// tests lower both so a small fixture crosses them.
#[derive(Clone, Copy)]
struct Limits {
    /// Byte admission of one journal entry.
    entry: u64,
    /// Level bytes per chunk entry. Never zero, so every pass of the chunk
    /// writer takes at least one byte.
    chunk: NonZeroUsize,
}

const PRODUCTION: Limits = Limits {
    entry: MAX_BYTES,
    chunk: CHUNK_BYTES,
};

/// One acknowledged chunk of one level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Piece {
    sequence: u64,
    length: u64,
    seal: [u8; 32],
}

/// A decoded boundary record.
struct Boundary {
    rows: Vec<DepthRow>,
    prefix: Vec<u8>,
    levels: Vec<Vec<Piece>>,
}

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
    // RANKED AS EACH LEVEL RETIRES, NOT AFTER THE WALK (AC-whp-o1-1, D-1844).
    // Every level is already durable in the journal when it retires, and the
    // ranker scores it then, so no survivor is held for a ranking pass at the
    // end; the walk holds what a streamed walk holds.
    runner::rank_checkpointed_streamed(
        column,
        Some(scoring_column),
        forward,
        crate::STORED_KEEP,
        runner::rank::Lens::Detectability,
        |on_retire| {
            walk_core(
                root,
                attempt,
                ladder,
                &bits,
                &runner::live_positions(),
                PRODUCTION,
                &mut |view| {
                    crate::emit_ladder_level(view.current(), view.admitted(), view.pairs());
                    Ok(())
                },
                |restored, reporter| {
                    drive_streamed(
                        ladder,
                        &bits,
                        &runner::live_positions(),
                        attempt.identity(),
                        restored,
                        reporter,
                        on_retire,
                    )
                },
            )
        },
    )
}

/// The production engine door: a resume restores level by level, each
/// level checked against its depth row and handed to `on_retire` before the
/// level after the next is read (D-4520); a fresh walk hands each level on as
/// it retires (D-1844).
fn drive_streamed(
    ladder: Ladder,
    bits: &engine::column::Column,
    live: &[u32],
    identity: [u8; 32],
    restored: Option<Restored<'_, '_>>,
    reporter: &mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>,
    on_retire: engine::resume::Retirement<'_>,
) -> Result<engine::keep::Streamed, engine::resume::Error> {
    match restored {
        Some(Restored {
            restoring,
            mut rows,
        }) => ladder.resume_restoring_streamed(
            bits,
            live,
            identity,
            restoring,
            &mut |level| rows.level(level).map(drop),
            reporter,
            on_retire,
        ),
        None => ladder.walk_checkpointed_streamed(bits, live, identity, reporter, on_retire),
    }
}

#[cfg(test)]
fn walk(
    root: &Path,
    attempt: &Attempt,
    ladder: Ladder,
    column: &engine::column::Column,
    live: &[u32],
    after_saved: &mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>,
) -> Result<Sweep, String> {
    walk_within(root, attempt, ladder, column, live, PRODUCTION, after_saved)
}

/// The journal walk with the retaining engine door: the whole frequent set
/// comes back. Tests compare it with an unjournalled walk; production ranks
/// through [`walk_core`]'s streamed door instead (D-1844).
#[cfg(test)]
fn walk_within(
    root: &Path,
    attempt: &Attempt,
    ladder: Ladder,
    column: &engine::column::Column,
    live: &[u32],
    limits: Limits,
    after_saved: &mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>,
) -> Result<Sweep, String> {
    walk_core(
        root,
        attempt,
        ladder,
        column,
        live,
        limits,
        after_saved,
        |restored, reporter| match restored {
            Some(Restored {
                restoring,
                mut rows,
            }) => {
                let checkpoint = restoring.into_checkpoint()?;
                for level in checkpoint.levels() {
                    rows.level(level).map_err(engine::resume::Error::Callback)?;
                }
                ladder.resume_checkpointed(column, live, attempt.identity(), checkpoint, reporter)
            }
            None => ladder.walk_checkpointed(column, live, attempt.identity(), reporter),
        },
    )
}

/// Recover, journal every level as the walk reaches it, and verify the final
/// boundary; `drive` runs the engine door (retaining or streamed) from the
/// recovered checkpoint, if any, with the journaling reporter. D-1844.
#[expect(
    clippy::too_many_arguments,
    reason = "the six journal inputs plus the reporter and the engine door; splitting them would separate recovery from the door it feeds"
)]
fn walk_core<R>(
    root: &Path,
    attempt: &Attempt,
    ladder: Ladder,
    column: &engine::column::Column,
    live: &[u32],
    limits: Limits,
    after_saved: &mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>,
    drive: impl FnOnce(
        Option<Restored<'_, '_>>,
        &mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>,
    ) -> Result<R, engine::resume::Error>,
) -> Result<R, String> {
    attempt.check()?;
    let mut journal = Journal::open(root, NAMESPACE, attempt.identity())?;
    let recovered = recover(&journal, limits)?;
    let mut acknowledged = recovered
        .as_ref()
        .map(|(sequence, seal, _)| (*sequence, *seal));
    let mut previous = acknowledged.map_or(0, |(sequence, _)| sequence);
    // The recovered boundary is read, not copied, by the restore; the walk
    // extends these copies of its rows and chunk lists, at most `MAX_ROWS`
    // of each.
    let (mut rows, mut levels) = recovered.as_ref().map_or_else(
        || (Vec::new(), Vec::new()),
        |(_, _, boundary)| (boundary.rows.clone(), boundary.levels.clone()),
    );
    let mut replays = match &recovered {
        Some((_, _, boundary)) => Some(Replay::of(journal.entries(), boundary, limits)?),
        None => None,
    };
    let restored = match (&recovered, replays.as_mut()) {
        (Some((sequence, _, boundary)), Some(replays)) => {
            let restored =
                Restored::open(replays, &boundary.rows, attempt.identity(), Some(attempt))?;
            restored
                .restoring
                .validate_for(ladder, column, live, attempt.identity())
                .map_err(error)?;
            crate::note(
                &telemetry::Event::info("cli.sweep", "AND checkpoint recovered")
                    .with("sequence", *sequence)
                    .with("depth", u64::try_from(boundary.rows.len()).map_err(error)?)
                    .with("interrupted_reservations", journal.interrupted()),
            );
            Some(restored)
        }
        _ => None,
    };
    rows.try_reserve_exact(MAX_ROWS.saturating_sub(rows.len()))
        .map_err(error)?;
    levels
        .try_reserve_exact(MAX_ROWS.saturating_sub(levels.len()))
        .map_err(error)?;
    // The current boundary is emitted once by the engine's resume callback.
    // Each earlier row is rehydrated, with its original counters, by the
    // restore's row check once its level has been read back and checked
    // against it (`RowCheck::level`), never before.
    let mut checkpointed = |view: &CheckpointView<'_>| {
        let row = DepthRow::of(view.current(), view.admitted(), view.pairs());
        let depth = usize::try_from(row.k).map_err(error)?;
        if depth == rows.len() {
            if rows.last() != Some(&row) {
                return Err("resumed AND boundary changed exact depth counters".into());
            }
        } else if depth == rows.len().saturating_add(1) {
            // THE NEW LEVEL ONLY. Every earlier level is already durable in
            // chunks the boundary record below names by seal.
            let pieces = publish_level(&mut journal, view, row.k, previous, limits)?;
            rows.push(row);
            levels.push(pieces);
            let payload = encode_boundary(view, &rows, &levels, limits)?;
            let published = journal.publish(&payload, limits.entry)?;
            previous = published.0;
            acknowledged = Some(published);
        } else {
            return Err("AND checkpoint depth sequence is discontinuous".into());
        }
        // If this fails, the just-published checkpoint remains a valid recovery
        // point but this attempt is refused. The next depth is never started.
        attempt.level(row)?;
        after_saved(view)
    };
    let sweep = drive(restored, &mut checkpointed).map_err(error)?;
    // Reopen the final boundary and every chunk it names before returning a
    // rankable run. Checking the exact acknowledged seals also rejects a newly
    // resealed replacement, not only accidental byte corruption.
    let entries = journal.entries();
    let final_saved = journal
        .latest(limits.entry)?
        .ok_or("AND final checkpoint is absent")?;
    if Some((final_saved.sequence, final_saved.seal)) != acknowledged {
        return Err("AND final checkpoint differs from its acknowledged publication".into());
    }
    let named = decode_boundary(&final_saved.payload, final_saved.sequence)?;
    for (level, pieces) in named.levels.iter().enumerate() {
        for (index, piece) in pieces.iter().enumerate() {
            if read_named(&entries, piece, level, index, limits)?.seal != piece.seal {
                return Err("AND final checkpoint chunk differs from its acknowledgment".into());
            }
        }
    }
    Ok(sweep)
}

/// The newest acknowledged boundary, with its sequence and seal.
///
/// The newest entry is either that boundary, or a chunk of the level after it
/// that an interrupted boundary never acknowledged. Such a chunk names the
/// boundary it followed. Past the journal's own open-time discovery, in which
/// `Journal::open` lists the identity's whole entry directory, recovery reads
/// the newest entry and at most one more, the boundary that chunk names. A
/// depth-1 chunk that follows no boundary means none was ever acknowledged.
fn recover(journal: &Journal, limits: Limits) -> Result<Option<(u64, [u8; 32], Boundary)>, String> {
    let Some(latest) = journal.latest(limits.entry)? else {
        return Ok(None);
    };
    let saved = match latest.payload.get(..8) {
        Some(magic) if magic == BOUNDARY_MAGIC => latest,
        Some(magic) if magic == CHUNK_MAGIC => {
            let [_, depth, _, previous] = chunk_header(&latest.payload)?;
            if previous == 0 {
                if depth != 1 {
                    return Err("AND orphan chunk follows no boundary but is not depth 1".into());
                }
                return Ok(None);
            }
            if previous >= latest.sequence {
                return Err("AND orphan chunk names a later boundary".into());
            }
            let boundary = journal.read(previous, limits.entry)?;
            let decoded = decode_boundary(&boundary.payload, boundary.sequence)?;
            if u64::try_from(decoded.rows.len())
                .map_err(error)?
                .checked_add(1)
                != Some(depth)
            {
                return Err("AND orphan chunk is not the level after its boundary".into());
            }
            boundary
        }
        _ => return Err("unsupported AND checkpoint entry".into()),
    };
    let boundary = decode_boundary(&saved.payload, saved.sequence)?;
    Ok(Some((saved.sequence, saved.seal, boundary)))
}

/// Publishes the current level as chunk entries and returns their pieces.
fn publish_level(
    journal: &mut Journal,
    view: &CheckpointView<'_>,
    depth: u64,
    previous: u64,
    limits: Limits,
) -> Result<Vec<Piece>, String> {
    if u64::try_from(CHUNK_HEADER + limits.chunk.get())
        .map_err(error)?
        .checked_add(ENVELOPE)
        .is_none_or(|bytes| bytes > limits.entry)
    {
        return Err("AND checkpoint chunk does not fit one journal entry".into());
    }
    let mut writer = Chunks {
        journal,
        depth,
        previous,
        limits,
        buffer: Vec::new(),
        pieces: Vec::new(),
    };
    view.write_current_to(&mut writer).map_err(error)?;
    writer.finish()
}

/// A writer that publishes every `limits.chunk` level bytes as one chunk
/// entry.
///
/// A full chunk is published when the next byte for its level arrives, or by
/// [`Chunks::finish`], and the caller takes the buffer to publish it and puts
/// it back emptied. So every pass of the write loop starts below a full chunk
/// and takes at least one byte, whatever `publish` does.
///
/// The buffer is reserved whole, a chunk header and a full chunk of level
/// bytes, when the level's first byte arrives, and every chunk of the level
/// reuses it, so no later byte grows or moves it. A level smaller than a chunk
/// still reserves the whole chunk.
struct Chunks<'a> {
    journal: &'a mut Journal,
    depth: u64,
    previous: u64,
    limits: Limits,
    /// The chunk being filled, header first; empty between chunks, its
    /// capacity kept.
    buffer: Vec<u8>,
    pieces: Vec<Piece>,
}

impl Chunks<'_> {
    /// Publishes one chunk, header first, and records its piece.
    fn publish(&mut self, chunk: &[u8]) -> Result<(), String> {
        let length = chunk
            .len()
            .checked_sub(CHUNK_HEADER)
            .ok_or("AND checkpoint chunk header missing")?;
        let (sequence, seal) = self.journal.publish(chunk, self.limits.entry)?;
        self.pieces.push(Piece {
            sequence,
            length: u64::try_from(length).map_err(error)?,
            seal,
        });
        Ok(())
    }

    fn finish(mut self) -> Result<Vec<Piece>, String> {
        // A buffer is empty between chunks, and otherwise holds its header
        // and at least one level byte: `write` adds the header only with bytes
        // to follow it.
        let last = std::mem::take(&mut self.buffer);
        if !last.is_empty() {
            self.publish(&last)?;
        }
        if self.pieces.is_empty() {
            return Err("AND checkpoint level wrote no bytes".into());
        }
        Ok(self.pieces)
    }
}

impl Write for Chunks<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let full = CHUNK_HEADER + self.limits.chunk.get();
        let mut rest = bytes;
        while !rest.is_empty() {
            if self.buffer.len() == full {
                let mut chunk = std::mem::take(&mut self.buffer);
                self.publish(&chunk).map_err(io::Error::other)?;
                chunk.clear();
                self.buffer = chunk;
            }
            if self.buffer.is_empty() {
                let index = u64::try_from(self.pieces.len()).map_err(io::Error::other)?;
                // Whole, once: every later chunk of the level finds the
                // buffer the last one was published from, emptied with its
                // capacity kept.
                if self.buffer.capacity() < full {
                    #[cfg(test)]
                    tests::RESERVATIONS.with(|count| count.set(count.get() + 1));
                    self.buffer
                        .try_reserve_exact(full)
                        .map_err(io::Error::other)?;
                }
                for word in [
                    u64::from_le_bytes(CHUNK_MAGIC),
                    self.depth,
                    index,
                    self.previous,
                ] {
                    self.buffer.extend_from_slice(&word.to_le_bytes());
                }
            }
            let room = full.saturating_sub(self.buffer.len());
            let (now, later) = rest.split_at(room.min(rest.len()));
            // Within the reservation: `now` never passes a full chunk.
            self.buffer.extend_from_slice(now);
            rest = later;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn chunk_header(payload: &[u8]) -> Result<[u64; 4], String> {
    let mut words = [0; 4];
    let mut input = payload;
    for word in &mut words {
        *word = read_word(&mut input)?;
    }
    if words[0] != u64::from_le_bytes(CHUNK_MAGIC) {
        return Err("AND checkpoint chunk has no chunk header".into());
    }
    Ok(words)
}

fn encode_boundary(
    view: &CheckpointView<'_>,
    rows: &[DepthRow],
    levels: &[Vec<Piece>],
    limits: Limits,
) -> Result<Vec<u8>, String> {
    let mut prefix = Vec::new();
    view.write_prefix_to(&mut prefix).map_err(error)?;
    let pieces = levels.iter().map(Vec::len).sum::<usize>();
    let bytes = boundary_bytes(rows.len(), prefix.len(), pieces)
        .ok_or("AND checkpoint boundary size overflow")?;
    if u64::try_from(bytes)
        .map_err(error)?
        .checked_add(ENVELOPE)
        .is_none_or(|total| total > limits.entry)
    {
        return Err(format!(
            "AND checkpoint boundary record at depth {} needs {bytes} bytes for its depth rows and {pieces} chunks, over the {} byte journal entry admission",
            rows.len(),
            limits.entry
        ));
    }
    let mut out = Vec::new();
    out.try_reserve_exact(bytes).map_err(error)?;
    out.extend_from_slice(&BOUNDARY_MAGIC);
    for count in [rows.len(), prefix.len(), pieces] {
        out.extend_from_slice(&u64::try_from(count).map_err(error)?.to_le_bytes());
    }
    for row in rows {
        for word in row.words() {
            out.extend_from_slice(&word.to_le_bytes());
        }
    }
    out.extend_from_slice(&prefix);
    for level in levels {
        out.extend_from_slice(&u64::try_from(level.len()).map_err(error)?.to_le_bytes());
        for piece in level {
            out.extend_from_slice(&piece.sequence.to_le_bytes());
            out.extend_from_slice(&piece.length.to_le_bytes());
            out.extend_from_slice(&piece.seal);
        }
    }
    Ok(out)
}

/// A boundary record's exact size: its header, each depth row with its
/// level's chunk count, the engine prefix and every chunk's piece.
fn boundary_bytes(rows: usize, prefix: usize, pieces: usize) -> Option<usize> {
    BOUNDARY_HEADER
        .checked_add(rows.checked_mul(DEPTH_BYTES + 8)?)?
        .checked_add(prefix)?
        .checked_add(pieces.checked_mul(PIECE_BYTES)?)
}

/// Decodes the boundary record published at `sequence`.
fn decode_boundary(bytes: &[u8], sequence: u64) -> Result<Boundary, String> {
    let mut input = bytes;
    let mut magic = [0; 8];
    input.read_exact(&mut magic).map_err(error)?;
    if magic != BOUNDARY_MAGIC {
        return Err("unsupported AND checkpoint boundary version".into());
    }
    let count = usize::try_from(read_word(&mut input)?).map_err(error)?;
    let prefix_bytes = usize::try_from(read_word(&mut input)?).map_err(error)?;
    let pieces = usize::try_from(read_word(&mut input)?).map_err(error)?;
    if count == 0
        || count > MAX_ROWS
        || pieces < count
        || boundary_bytes(count, prefix_bytes, pieces) != Some(bytes.len())
    {
        return Err("AND checkpoint boundary has invalid exact lengths".into());
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
    let (prefix, rest) = input
        .split_at_checked(prefix_bytes)
        .ok_or("AND checkpoint boundary prefix is truncated")?;
    input = rest;
    let mut levels = Vec::new();
    levels.try_reserve_exact(count).map_err(error)?;
    let mut last = 0_u64;
    let mut named = 0_usize;
    for _ in 0..count {
        let length = usize::try_from(read_word(&mut input)?).map_err(error)?;
        if length == 0 || length > pieces - named {
            return Err("AND checkpoint boundary names an empty or excess level".into());
        }
        named += length;
        let mut level = Vec::new();
        level.try_reserve_exact(length).map_err(error)?;
        for _ in 0..length {
            let at = read_word(&mut input)?;
            let bytes = read_word(&mut input)?;
            let mut seal = [0; 32];
            input.read_exact(&mut seal).map_err(error)?;
            // Chunks are published in depth order before the boundary that
            // names them, so their sequences strictly increase below it.
            if at <= last || bytes == 0 {
                return Err("AND checkpoint boundary chunk order or length is invalid".into());
            }
            last = at;
            level.push(Piece {
                sequence: at,
                length: bytes,
                seal,
            });
        }
        levels.push(level);
    }
    // The record's exact length leaves `(pieces - named) × PIECE_BYTES` bytes
    // unread here, so every piece is named exactly when the input is spent.
    if last >= sequence || named != pieces {
        return Err("AND checkpoint boundary chunks do not precede it exactly".into());
    }
    let mut owned = Vec::new();
    owned.try_reserve_exact(prefix.len()).map_err(error)?;
    owned.extend_from_slice(prefix);
    Ok(Boundary {
        rows,
        prefix: owned,
        levels,
    })
}

/// Rebuilds the engine checkpoint a boundary record names, whole: every
/// level decoded and checked against its depth row. The walk does not call
/// this; it restores level by level through [`Restored`] (D-4520).
#[cfg(test)]
fn replay(
    journal: &Journal,
    boundary: &Boundary,
    identity: [u8; 32],
    limits: Limits,
) -> Result<Checkpoint, String> {
    let mut replays = Replay::of(journal.entries(), boundary, limits)?;
    let Restored {
        restoring,
        mut rows,
    } = Restored::open(&mut replays, &boundary.rows, identity, None)?;
    let checkpoint = restoring.into_checkpoint().map_err(error)?;
    for level in checkpoint.levels() {
        rows.level(level)?;
    }
    Ok(checkpoint)
}

/// A recovered boundary's engine payload, decoded from its chunks one level
/// at a time, and the check each restored level passes before the walk hands
/// it on (AC-whp-o1-1, D-4520).
struct Restored<'r, 'b> {
    restoring: Restoring<'r, Replay<'b>>,
    rows: RowCheck<'b>,
}

impl<'r, 'b> Restored<'r, 'b> {
    /// Opens the payload `replays` reads, refusing a foreign identity and a
    /// depth history whose length is not the payload's level count before
    /// any level is read.
    fn open(
        replays: &'r mut Replay<'b>,
        rows: &'b [DepthRow],
        identity: [u8; 32],
        attempt: Option<&'b Attempt>,
    ) -> Result<Self, String> {
        let total = replays.total;
        let restoring = Checkpoint::restore_from(replays, total, identity).map_err(error)?;
        if rows.len() != restoring.level_count() {
            return Err("AND checkpoint depth history length mismatch".into());
        }
        let rows = RowCheck {
            rows,
            next: 0,
            admitted: 0,
            pairs: 0,
            final_admitted: u64::try_from(restoring.admitted()).map_err(error)?,
            final_pairs: restoring.pairs(),
            attempt,
        };
        Ok(Self { restoring, rows })
    }
}

/// Checks each restored level, in depth order, against the depth row its
/// boundary record holds for it: what `validate_rows` checked over the whole
/// decoded history before D-4520, one level at a time. With an attempt, each
/// row before the resume frontier's is recorded in it once its level has
/// passed; the frontier's own row is recorded when the engine reports its
/// boundary again.
struct RowCheck<'b> {
    rows: &'b [DepthRow],
    next: usize,
    admitted: u64,
    pairs: u64,
    /// The payload header's cumulative counters, which the last row reaches.
    final_admitted: u64,
    final_pairs: u64,
    attempt: Option<&'b Attempt>,
}

impl RowCheck<'_> {
    /// Checks the next restored level against its row and returns the row.
    fn level(&mut self, level: &Frontier) -> Result<DepthRow, String> {
        let offset = self.next;
        let row = *self
            .rows
            .get(offset)
            .ok_or("AND checkpoint depth history length mismatch")?;
        if offset > 0 {
            self.admitted = self
                .admitted
                .checked_add(level.generated)
                .ok_or("AND admitted counter overflow")?;
        }
        let expected = DepthRow::of(
            level,
            usize::try_from(self.admitted).map_err(error)?,
            row.pairs,
        );
        if expected != row
            || row.pairs < self.pairs
            || row.pairs < self.admitted
            || (offset == 0 && row.pairs != 0)
        {
            return Err("AND checkpoint depth history differs from the retained search".into());
        }
        self.pairs = row.pairs;
        self.next = offset + 1;
        if self.next == self.rows.len() {
            if self.admitted != self.final_admitted || self.pairs != self.final_pairs {
                return Err("AND checkpoint final counters disagree".into());
            }
        } else if let Some(attempt) = self.attempt {
            attempt.level(row)?;
        }
        Ok(row)
    }
}

/// The prefix, then every named chunk's level bytes, in depth order. Each
/// chunk is read only when the decoder reaches it, and the one before it is
/// released first (`self.chunk = Vec::new();` ahead of the read), so at most
/// one chunk payload is held beside the level being decoded.
struct Replay<'a> {
    entries: Entries,
    limits: Limits,
    /// Every byte the decoder may read: the prefix and every named chunk's
    /// level bytes.
    total: u64,
    prefix: &'a [u8],
    levels: &'a [Vec<Piece>],
    level: usize,
    piece: usize,
    /// The chunk payload being read, and the read position within it.
    chunk: Vec<u8>,
    at: usize,
}

impl<'a> Replay<'a> {
    /// The reader of the payload `boundary` names.
    fn of(entries: Entries, boundary: &'a Boundary, limits: Limits) -> Result<Self, String> {
        let total = boundary
            .levels
            .iter()
            .flatten()
            .try_fold(
                u64::try_from(boundary.prefix.len()).map_err(error)?,
                |sum, piece| sum.checked_add(piece.length),
            )
            .ok_or("AND checkpoint history length overflow")?;
        Ok(Self {
            entries,
            limits,
            total,
            prefix: &boundary.prefix,
            levels: &boundary.levels,
            level: 0,
            piece: 0,
            chunk: Vec::new(),
            at: 0,
        })
    }
}

impl Replay<'_> {
    /// Loads the next named chunk; `false` once every chunk has been read.
    fn load(&mut self) -> Result<bool, String> {
        while let Some(level) = self.levels.get(self.level) {
            let Some(piece) = level.get(self.piece) else {
                self.level += 1;
                self.piece = 0;
                continue;
            };
            // Release the chunk just read before reading the next, so two
            // chunk payloads are never held at once.
            self.chunk = Vec::new();
            let saved = read_named(&self.entries, piece, self.level, self.piece, self.limits)?;
            #[cfg(test)]
            tests::CHUNK_LOADS.with(|count| count.set(count.get() + 1));
            let [_, depth, index, _] = chunk_header(&saved.payload)?;
            if saved.seal != piece.seal
                || u64::try_from(saved.payload.len()).map_err(error)?
                    != piece
                        .length
                        .saturating_add(u64::try_from(CHUNK_HEADER).map_err(error)?)
                || usize::try_from(depth).ok() != self.level.checked_add(1)
                || usize::try_from(index).ok() != Some(self.piece)
            {
                return Err("AND checkpoint chunk differs from its boundary record".into());
            }
            self.chunk = saved.payload;
            self.at = CHUNK_HEADER;
            self.piece += 1;
            return Ok(true);
        }
        Ok(false)
    }
}

impl Read for Replay<'_> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if !self.prefix.is_empty() {
            return self.prefix.read(out);
        }
        if self.at >= self.chunk.len() && !self.load().map_err(io::Error::other)? {
            return Ok(0);
        }
        let available = self.chunk.get(self.at..).unwrap_or_default();
        let taken = available.len().min(out.len());
        let (into, _) = out.split_at_mut(taken);
        let (from, _) = available.split_at(taken);
        into.copy_from_slice(from);
        self.at += taken;
        Ok(taken)
    }
}

/// Reads the chunk a boundary record names as `level`'s chunk `index`
/// (both from zero). A read that fails, an entry vanished or left without
/// its completion marker among them, refuses naming the chunk's sequence and
/// its one-based depth and zero-based index beside the journal's own reason.
fn read_named(
    entries: &Entries,
    piece: &Piece,
    level: usize,
    index: usize,
    limits: Limits,
) -> Result<crate::search_checkpoint::Saved, String> {
    entries.read(piece.sequence, limits.entry).map_err(|why| {
        format!(
            "AND checkpoint chunk {} (depth {}, index {index}) named by its boundary record cannot be read: {why}",
            piece.sequence,
            level.saturating_add(1)
        )
    })
}

fn read_word(input: &mut &[u8]) -> Result<u64, String> {
    let mut bytes = [0; 8];
    input.read_exact(&mut bytes).map_err(error)?;
    Ok(u64::from_le_bytes(bytes))
}

fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "and_checkpoint_tests.rs"]
mod tests;
