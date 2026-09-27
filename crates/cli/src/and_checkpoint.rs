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

use std::io::{self, Read, Write};
use std::num::NonZeroUsize;
use std::path::Path;

use engine::resume::{Checkpoint, CheckpointView};
use engine::{Ladder, Sweep};
use indicators::column::Column;

use crate::search_checkpoint::Journal;
use crate::sweep_evidence::{Attempt, DepthRow};

const NAMESPACE: &str = "and-checkpoint-v2";
/// The byte admission of one journal entry, chunk or boundary record.
const MAX_BYTES: u64 = 64 * 1024 * 1024;
/// Level bytes one chunk entry carries in production: half an entry's
/// admission, so a chunk and its header always fit.
/// A zero literal here fails const evaluation, and so the build.
const CHUNK_BYTES: NonZeroUsize = NonZeroUsize::new(32 * 1024 * 1024).unwrap();
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
    walk_within(root, attempt, ladder, column, live, PRODUCTION, after_saved)
}

fn walk_within(
    root: &Path,
    attempt: &Attempt,
    ladder: Ladder,
    column: &engine::column::Column,
    live: &[u32],
    limits: Limits,
    after_saved: &mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>,
) -> Result<Sweep, String> {
    attempt.check()?;
    let mut journal = Journal::open(root, NAMESPACE, attempt.identity())?;
    let recovered = recover(&journal, limits)?;
    let mut acknowledged = recovered
        .as_ref()
        .map(|(sequence, seal, _)| (*sequence, *seal));
    let mut previous = acknowledged.map_or(0, |(sequence, _)| sequence);
    let (checkpoint, mut rows, mut levels) = match recovered {
        Some((sequence, _, boundary)) => {
            let checkpoint = replay(&journal, &boundary, attempt.identity(), limits)?;
            checkpoint
                .validate_for(ladder, column, live, attempt.identity())
                .map_err(error)?;
            crate::note(
                &telemetry::Event::info("cli.sweep", "AND checkpoint recovered")
                    .with("sequence", sequence)
                    .with("depth", u64::try_from(boundary.rows.len()).map_err(error)?)
                    .with("interrupted_reservations", journal.interrupted()),
            );
            (Some(checkpoint), boundary.rows, boundary.levels)
        }
        None => (None, Vec::new(), Vec::new()),
    };
    rows.try_reserve_exact(MAX_ROWS.saturating_sub(rows.len()))
        .map_err(error)?;
    levels
        .try_reserve_exact(MAX_ROWS.saturating_sub(levels.len()))
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
    // Reopen the final boundary and every chunk it names before returning a
    // rankable run. Checking the exact acknowledged seals also rejects a newly
    // resealed replacement, not only accidental byte corruption.
    let final_saved = journal
        .latest(limits.entry)?
        .ok_or("AND final checkpoint is absent")?;
    if Some((final_saved.sequence, final_saved.seal)) != acknowledged {
        return Err("AND final checkpoint differs from its acknowledged publication".into());
    }
    let named = decode_boundary(&final_saved.payload, final_saved.sequence)?;
    for piece in named.levels.iter().flatten() {
        if journal.read(piece.sequence, limits.entry)?.seal != piece.seal {
            return Err("AND final checkpoint chunk differs from its acknowledgment".into());
        }
    }
    Ok(sweep)
}

/// The newest acknowledged boundary, with its sequence and seal.
///
/// The newest entry is either that boundary, or a chunk of the level after it
/// that an interrupted boundary never acknowledged. Such a chunk names the
/// boundary it followed, so recovery reads exactly one extra entry and scans
/// nothing. A depth-1 chunk that follows no boundary means none was ever
/// acknowledged.
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
/// [`Chunks::finish`], and the caller takes the buffer before publishing it.
/// So every pass of the write loop starts below a full chunk and takes at
/// least one byte, whatever `publish` does.
struct Chunks<'a> {
    journal: &'a mut Journal,
    depth: u64,
    previous: u64,
    limits: Limits,
    /// The chunk being filled, header first; empty between chunks.
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
                let chunk = std::mem::take(&mut self.buffer);
                self.publish(&chunk).map_err(io::Error::other)?;
            }
            if self.buffer.is_empty() {
                let index = u64::try_from(self.pieces.len()).map_err(io::Error::other)?;
                self.buffer
                    .try_reserve(CHUNK_HEADER)
                    .map_err(io::Error::other)?;
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
            self.buffer
                .try_reserve(now.len())
                .map_err(io::Error::other)?;
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
    if last >= sequence || named != pieces || !input.is_empty() {
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

/// Rebuilds the engine checkpoint a boundary record names.
fn replay(
    journal: &Journal,
    boundary: &Boundary,
    identity: [u8; 32],
    limits: Limits,
) -> Result<Checkpoint, String> {
    let total = boundary
        .levels
        .iter()
        .flatten()
        .try_fold(
            u64::try_from(boundary.prefix.len()).map_err(error)?,
            |sum, piece| sum.checked_add(piece.length),
        )
        .ok_or("AND checkpoint history length overflow")?;
    let mut stream = Replay {
        journal,
        limits,
        prefix: &boundary.prefix,
        levels: &boundary.levels,
        level: 0,
        piece: 0,
        chunk: Vec::new(),
        at: 0,
    };
    let checkpoint = Checkpoint::read_from(&mut stream, total, identity).map_err(error)?;
    validate_rows(&checkpoint, &boundary.rows)?;
    Ok(checkpoint)
}

/// The prefix, then every named chunk's level bytes, in depth order. Each
/// chunk is read only when the decoder reaches it, so at most one chunk is
/// held beside the checkpoint being rebuilt.
struct Replay<'a> {
    journal: &'a Journal,
    limits: Limits,
    prefix: &'a [u8],
    levels: &'a [Vec<Piece>],
    level: usize,
    piece: usize,
    /// The chunk payload being read, and the read position within it.
    chunk: Vec<u8>,
    at: usize,
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
            let saved = self.journal.read(piece.sequence, self.limits.entry)?;
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

fn error(why: impl std::fmt::Display) -> String {
    why.to_string()
}

#[cfg(test)]
#[path = "and_checkpoint_tests.rs"]
mod tests;
