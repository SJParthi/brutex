//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! Versioned retained Apriori checkpoints at level boundaries.
//!
//! This codec does not authenticate bytes: the caller must seal its durable
//! envelope and bind the exact input column, offered positions, configuration,
//! vocabulary and implementation to the supplied identity. Decoding checks the
//! format and state invariants; it cannot prove that omitted survivors existed.
//! Checkpointing and retained history cost O(total survivors), not O(1).

use std::fmt;
use std::io::{self, Read, Write};

use crate::{
    Breach, Column, Excluded, Frontier, Halt, Itemset, Ladder, Progress, Sink, Sweep, Why, keep,
};
use vocab::ConditionMask;

/// Immutable checkpoint wire version. A layout change requires another version.
pub const VERSION: u64 = 1;
const MAGIC: [u8; 8] = *b"BRTXCP01";
const HEADER_BYTES: u64 = 192;
const LEVEL_BYTES: u64 = 56;
const ITEM_BYTES: u64 = 56;

/// An explicit checkpoint refusal; none is converted into successful extinction.
#[derive(Debug)]
pub enum Error {
    /// The file, state or caller's identity/configuration does not agree.
    Invalid(&'static str),
    /// A fallible allocation was refused.
    Memory,
    /// Reading or encoding failed.
    Io(io::Error),
    /// The checkpoint sink refused; no next level was started.
    Callback(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(reason) => write!(formatter, "checkpoint refused: {reason}"),
            Self::Memory => formatter.write_str("checkpoint allocation refused"),
            Self::Io(error) => write!(formatter, "checkpoint I/O refused: {error}"),
            Self::Callback(error) => write!(formatter, "checkpoint sink refused: {error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<std::collections::TryReserveError> for Error {
    fn from(_: std::collections::TryReserveError) -> Self {
        Self::Memory
    }
}

/// Owned state decoded from a caller-verified durable envelope.
/// Fields are private so mutation cannot bypass validation after decoding.
#[derive(Debug)]
pub struct Checkpoint {
    identity: [u8; 32],
    ladder: Ladder,
    live: Vec<u32>,
    levels: Vec<Frontier>,
    progress: Progress,
}

impl Checkpoint {
    /// Decode one exact envelope payload within `max_bytes`.
    ///
    /// # Errors
    /// Refuses truncation, trailing bytes, foreign identity, unsupported
    /// versions, malformed counts/masks/state, or failed allocation/I/O.
    /// The caller must verify the payload's cryptographic seal BEFORE calling.
    pub fn read_from(
        reader: &mut impl Read,
        max_bytes: u64,
        expected_identity: [u8; 32],
    ) -> Result<Self, Error> {
        Self::restore_from(reader, max_bytes, expected_identity)?.into_checkpoint()
    }

    /// Decode the header, offers and exclusions of one exact envelope payload
    /// within `max_bytes`, check everything that needs no level, and leave
    /// the levels in `reader` for [`Restoring::next_level`] to decode one at
    /// a time. AC-whp-o1-1, D-4520.
    ///
    /// [`Self::read_from`] is this followed by every level, so both refuse
    /// exactly the same payloads; only the order in which a payload with
    /// several faults names its first one differs.
    ///
    /// # Errors
    /// As [`Self::read_from`], for every refusal that needs no level.
    /// The caller must verify the payload's cryptographic seal BEFORE calling.
    pub fn restore_from<R: Read>(
        reader: &mut R,
        max_bytes: u64,
        expected_identity: [u8; 32],
    ) -> Result<Restoring<'_, R>, Error> {
        let mut decoder = Decoder {
            reader,
            remaining: max_bytes,
        };
        let (mut head, counts) = decoder.header(expected_identity)?;
        head.live = decoder.live(counts.0)?;
        head.progress.excluded = decoder.excluded(counts.1)?;
        decoder.count(counts.2, LEVEL_BYTES)?;
        let (allowed, distinct) = head.validate_head(counts.2)?;
        Ok(Restoring {
            decoder,
            head,
            count: counts.2,
            decoded: 0,
            admitted: 0,
            allowed,
            distinct,
        })
    }

    /// All levels already recorded, including the current complete or halted level.
    #[must_use]
    pub fn levels(&self) -> &[Frontier] {
        &self.levels
    }

    /// Caller-supplied full run identity.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }

    /// Exclusions measured before the combination ladder.
    #[must_use]
    pub fn excluded(&self) -> &[Excluded] {
        &self.progress.excluded
    }

    /// Exact cumulative admitted joined candidates (singletons are separate).
    #[must_use]
    pub const fn admitted(&self) -> usize {
        self.progress.admitted
    }

    /// Exact cumulative pairs walked.
    #[must_use]
    pub const fn pairs(&self) -> u64 {
        self.progress.pairs
    }

    /// Resource refusal, if this is a terminal partial level.
    #[must_use]
    pub const fn halted(&self) -> Option<Halt> {
        self.progress.halted
    }

    /// Exact column row count.
    #[must_use]
    pub const fn bars(&self) -> u64 {
        self.progress.bars
    }

    /// Validate the caller before replaying any checkpoint-derived evidence.
    ///
    /// # Errors
    /// Refuses identity, row-count, offered-sequence or exact policy mismatch.
    /// The caller remains responsible for the complete column-byte identity.
    ///
    /// # Support lanes are deliberately NOT compared
    ///
    /// The lane count is a scheduling sentinel (`Ladder::support_lanes`), never
    /// a depth or answer parameter, and the run identity leaves it out:
    /// `a_support_lane_bound_changes_only_scheduling` proves the answer is the
    /// same at every lane count. Comparing it refused an interrupted
    /// `sweep-stored` resume on any machine or container with a different core
    /// count, because `cli` derives lanes from `available_parallelism`. The
    /// resumed walk schedules with the CALLER's lanes. D-1439.
    pub fn validate_for(
        &self,
        ladder: Ladder,
        column: &Column,
        live: &[u32],
        expected_identity: [u8; 32],
    ) -> Result<(), Error> {
        if self.identity != expected_identity
            || self.progress.bars != column.bars()
            || self.live != live
            || self.ladder.min_hits != ladder.min_hits
            || self.ladder.ceiling != ladder.ceiling
            || self.ladder.pair_budget != ladder.pair_budget
        {
            return Err(Error::Invalid(
                "run identity, column, offers or configuration mismatch",
            ));
        }
        Ok(())
    }

    /// Encode without cloning the retained frontier history.
    ///
    /// # Errors
    /// Propagates a writer failure. The caller publishes only after sealing and
    /// durably acknowledging the complete payload.
    pub fn write_to(&self, writer: &mut impl Write) -> Result<(), Error> {
        let (current, retired) = self.levels.split_last().ok_or(Error::Invalid("no level"))?;
        CheckpointView {
            identity: self.identity,
            ladder: self.ladder,
            live: &self.live,
            retired,
            retired_count: retired.len(),
            current,
            progress: &self.progress,
        }
        .write_to(writer)
    }

    /// Every check of [`Self::read_from`] that needs no level: the ladder,
    /// the singleton pair count, the halt and the exclusions. Returns the
    /// mask a survivor may draw from and how many distinct positions were
    /// offered, which the levels are checked against as they are decoded.
    fn validate_head(&self, count: usize) -> Result<(ConditionMask, usize), Error> {
        let mut offered = std::collections::HashSet::new();
        offered.try_reserve(self.live.len())?;
        offered.extend(self.live.iter().copied());
        if self.ladder.min_hits == 0 || self.ladder.ceiling == 0 || self.ladder.pair_budget == 0 {
            return Err(Error::Invalid("singleton accounting"));
        }
        let allowed = self
            .live
            .iter()
            .copied()
            .filter(|&position| u16::try_from(position).is_ok_and(vocab::table::is_live))
            .fold(ConditionMask::ZERO, ConditionMask::with_bit);
        // Exclusion and survival are mutually exclusive, even if the same
        // position was offered more than once. A sealed payload still has to
        // reconcile its own explicit claims before it can seed another level.
        let allowed = self
            .progress
            .excluded
            .iter()
            .fold(allowed, |mask, excluded| {
                mask.without_bit(excluded.position)
            });
        if count == 1 && self.progress.pairs != 0 {
            return Err(Error::Invalid("singleton pair accounting"));
        }
        self.validate_halt(count)?;
        self.validate_excluded(&offered)?;
        Ok((allowed, offered.len()))
    }

    fn validate_halt(&self, count: usize) -> Result<(), Error> {
        if let Some(halt) = self.progress.halted
            && (u64::from(halt.k) != crate::len_u64(count)
                || halt.k < 2
                || halt.ceiling != self.ladder.ceiling
                || halt.pair_budget != self.ladder.pair_budget
                || halt.candidates != self.progress.admitted
                || halt.pairs != self.progress.pairs
                || (halt.breach == Breach::Candidates && halt.candidates != halt.ceiling)
                || (halt.breach == Breach::Pairs && halt.pairs < halt.pair_budget))
        {
            return Err(Error::Invalid("halt does not match accumulated state"));
        }
        Ok(())
    }

    fn validate_excluded(&self, offered: &std::collections::HashSet<u32>) -> Result<(), Error> {
        let mut seen = std::collections::HashSet::new();
        seen.try_reserve(self.progress.excluded.len())?;
        for excluded in &self.progress.excluded {
            let live = u16::try_from(excluded.position).is_ok_and(vocab::table::is_live);
            let correct = match excluded.reason {
                Why::AlwaysFalse => live && excluded.support == Some(0),
                Why::AlwaysTrue => {
                    live && self.progress.bars > 0 && excluded.support == Some(self.progress.bars)
                }
                Why::NotLive => !live && excluded.support.is_none(),
            };
            if !correct || !seen.insert(excluded.position) || !offered.contains(&excluded.position)
            {
                return Err(Error::Invalid("invalid exclusion"));
            }
        }
        Ok(())
    }
}

/// A checkpoint payload decoded one level at a time. AC-whp-o1-1, D-4520.
///
/// [`Checkpoint::read_from`] decodes every level before it returns, so a
/// resume that went through it held the whole restored history at once:
/// O(total survivors), however soon each level was handed on. This holds the
/// header, the offers and the exclusions, and decodes and checks one level per
/// [`Self::next_level`]. Every check [`Checkpoint::read_from`] makes is made:
/// the ones that need no level by [`Checkpoint::restore_from`], each level's
/// own as it is decoded, and the trailing bytes and the cumulative counters
/// before the LAST level is returned. A caller that hands each level on and
/// drops it holds at most two levels at once; a level handed on before a later
/// one refuses is handed to a caller whose result that refusal then discards.
pub struct Restoring<'r, R> {
    decoder: Decoder<'r, R>,
    /// Everything but the levels, which stay in the reader.
    head: Checkpoint,
    count: usize,
    decoded: usize,
    admitted: u64,
    allowed: ConditionMask,
    distinct: usize,
}

impl<R: Read> Restoring<'_, R> {
    /// How many levels the payload holds, the current one included.
    #[must_use]
    pub const fn level_count(&self) -> usize {
        self.count
    }

    /// Exact cumulative admitted joined candidates the header records.
    #[must_use]
    pub const fn admitted(&self) -> usize {
        self.head.progress.admitted
    }

    /// Exact cumulative pairs walked, as the header records them.
    #[must_use]
    pub const fn pairs(&self) -> u64 {
        self.head.progress.pairs
    }

    /// [`Checkpoint::validate_for`], answered from the header alone.
    ///
    /// # Errors
    /// As [`Checkpoint::validate_for`].
    pub fn validate_for(
        &self,
        ladder: Ladder,
        column: &Column,
        live: &[u32],
        expected_identity: [u8; 32],
    ) -> Result<(), Error> {
        self.head
            .validate_for(ladder, column, live, expected_identity)
    }

    /// Decode and check the next level; `None` once every level has been
    /// returned. The last level is returned only after the payload's
    /// trailing bytes and cumulative counters are checked.
    ///
    /// # Errors
    /// Every level refusal of [`Checkpoint::read_from`], when the level that
    /// carries it is reached.
    pub fn next_level(&mut self) -> Result<Option<Frontier>, Error> {
        if self.decoded == self.count {
            return Ok(None);
        }
        let level = self.decoder.level()?;
        let offset = self.decoded;
        let head = &self.head;
        if offset == 0
            && (level.generated != crate::len_u64(head.live.len())
                || level.excluded != crate::len_u64(head.progress.excluded.len())
                || level.duplicates != crate::len_u64(head.live.len() - self.distinct))
        {
            return Err(Error::Invalid("singleton accounting"));
        }
        validate_level(
            &level,
            offset,
            self.count,
            head.progress.bars,
            head.ladder.min_hits,
            self.allowed,
        )?;
        if offset > 0 {
            self.admitted = self
                .admitted
                .checked_add(level.generated)
                .ok_or(Error::Invalid("admitted overflow"))?;
        }
        self.decoded += 1;
        if self.decoded == self.count {
            self.finish()?;
        }
        Ok(Some(level))
    }

    /// Every level, collected: what [`Checkpoint::read_from`] returns.
    ///
    /// # Errors
    /// As [`Self::next_level`], or a refused allocation.
    pub fn into_checkpoint(mut self) -> Result<Checkpoint, Error> {
        let mut levels = crate::reserved(self.count)?;
        while let Some(level) = self.next_level()? {
            levels.push(level);
        }
        let mut checkpoint = self.head;
        checkpoint.levels = levels;
        Ok(checkpoint)
    }

    fn finish(&mut self) -> Result<(), Error> {
        let mut extra = [0_u8; 1];
        if self.decoder.reader.read(&mut extra)? != 0 {
            return Err(Error::Invalid("trailing payload bytes"));
        }
        let progress = &self.head.progress;
        if self.admitted != crate::len_u64(progress.admitted)
            || progress.admitted > self.head.ladder.ceiling
            || self.admitted > progress.pairs
            || (progress.halted.is_none() && progress.pairs > self.head.ladder.pair_budget)
        {
            return Err(Error::Invalid("cumulative candidate or pair accounting"));
        }
        Ok(())
    }
}

/// Borrowed safe boundary. Writing it does not duplicate the survivor vectors.
pub struct CheckpointView<'a> {
    identity: [u8; 32],
    ladder: Ladder,
    live: &'a [u32],
    retired: &'a [Frontier],
    /// How many levels this walk has retired. Equal to `retired.len()` on a
    /// retaining walk; on a streamed one the levels were handed on and
    /// dropped, so `retired` is empty and only the count remains. AC-whp-o1-1,
    /// D-1844.
    retired_count: usize,
    current: &'a Frontier,
    progress: &'a Progress,
}

impl CheckpointView<'_> {
    /// Current level, complete unless `halted()` reports a resource refusal.
    #[must_use]
    pub const fn current(&self) -> &Frontier {
        self.current
    }
    /// Earlier retained levels, in canonical depth order.
    #[must_use]
    pub const fn retired(&self) -> &[Frontier] {
        self.retired
    }
    /// Caller-supplied full run identity.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }
    /// Exact cumulative admitted candidates.
    #[must_use]
    pub const fn admitted(&self) -> usize {
        self.progress.admitted
    }
    /// Exact cumulative pairs walked.
    #[must_use]
    pub const fn pairs(&self) -> u64 {
        self.progress.pairs
    }
    /// Resource refusal; this state must never advance to another level.
    #[must_use]
    pub const fn halted(&self) -> Option<Halt> {
        self.progress.halted
    }
    /// No remaining search: extinction or a named resource refusal.
    #[must_use]
    pub fn terminal(&self) -> bool {
        self.current.frequent.is_empty() || self.halted().is_some()
    }

    /// Encode the fixed version using constant scratch space.
    ///
    /// Exactly [`Self::write_prefix_to`] followed by every retained level and
    /// then the current one, each as [`Self::write_current_to`] writes it.
    ///
    /// # Errors
    /// Propagates any write failure. This writes no cryptographic envelope;
    /// sealing, flushing, durable publication and identity binding belong to the caller.
    pub fn write_to(&self, writer: &mut impl Write) -> Result<(), Error> {
        // A STREAMED BOUNDARY HOLDS NO EARLIER LEVEL, so it cannot write the
        // whole history; its caller saved each level as it was reached and
        // writes the prefix and the current level only. D-1844.
        if self.retired.len() != self.retired_count {
            return Err(Error::Invalid(
                "a streamed boundary holds no earlier level to write",
            ));
        }
        self.write_prefix_to(writer)?;
        for level in self.retired.iter().chain(std::iter::once(self.current)) {
            write_level(writer, level)?;
        }
        Ok(())
    }

    /// Encode everything [`Self::write_to`] writes before its first level: the
    /// header with this boundary's counters and level count, the offered
    /// positions and the exclusions.
    ///
    /// It writes `192 + 8 × offered + 24 × excluded` bytes, and no level. A
    /// durable caller that has already saved every earlier level writes this
    /// and [`Self::write_current_to`] at a boundary, not the whole history;
    /// the prefix of its newest boundary followed by every level in depth
    /// order is byte for byte what [`Self::write_to`] writes there.
    ///
    /// # Errors
    /// Propagates any write failure.
    pub fn write_prefix_to(&self, writer: &mut impl Write) -> Result<(), Error> {
        writer.write_all(&MAGIC)?;
        words(writer, [VERSION, u64::from(vocab::VOCAB_VERSION)])?;
        writer.write_all(&self.identity)?;
        words(
            writer,
            [
                self.ladder.min_hits,
                crate::len_u64(self.ladder.ceiling),
                self.ladder.pair_budget,
                crate::len_u64(self.ladder.support_lanes),
                self.progress.bars,
                crate::len_u64(self.progress.admitted),
                self.progress.pairs,
            ],
        )?;
        write_halt(writer, self.progress.halted)?;
        words(
            writer,
            [
                crate::len_u64(self.live.len()),
                crate::len_u64(self.progress.excluded.len()),
                crate::len_u64(self.retired_count) + 1,
            ],
        )?;
        for &position in self.live {
            words(writer, [u64::from(position)])?;
        }
        for excluded in &self.progress.excluded {
            let reason = match excluded.reason {
                Why::NotLive => 0,
                Why::AlwaysFalse => 1,
                Why::AlwaysTrue => 2,
            };
            words(
                writer,
                [
                    u64::from(excluded.position),
                    excluded.support.unwrap_or(0),
                    reason,
                ],
            )?;
        }
        Ok(())
    }

    /// Encode the current level alone, as [`Self::write_to`] writes it among
    /// the others: `56 + 56 × survivors` bytes, whatever the retained levels
    /// hold.
    ///
    /// # Errors
    /// Propagates any write failure.
    pub fn write_current_to(&self, writer: &mut impl Write) -> Result<(), Error> {
        write_level(writer, self.current)
    }
}

fn write_level(writer: &mut impl Write, level: &Frontier) -> Result<(), Error> {
    words(
        writer,
        [
            u64::from(level.k),
            crate::len_u64(level.frequent.len()),
            level.generated,
            level.duplicates,
            level.excluded,
            level.pruned,
            level.infrequent,
        ],
    )?;
    for item in &level.frequent {
        words(writer, item.mask.words())?;
        words(writer, [item.hits])?;
    }
    Ok(())
}

type Reporter<'a> = &'a mut dyn FnMut(&CheckpointView<'_>) -> Result<(), String>;

struct Retaining<'a> {
    levels: Vec<Frontier>,
    identity: [u8; 32],
    ladder: Ladder,
    live: &'a [u32],
    reporter: Reporter<'a>,
}

impl Sink for Retaining<'_> {
    type Error = Error;
    fn report(&mut self, _: &Frontier, _: usize, _: u64) {}
    fn retire(&mut self, level: Frontier, _: Option<&Frontier>) {
        self.levels.push(level);
    }
    fn checkpoint(&mut self, current: &Frontier, progress: &Progress) -> Result<(), Error> {
        (self.reporter)(&CheckpointView {
            identity: self.identity,
            ladder: self.ladder,
            live: self.live,
            retired: &self.levels,
            retired_count: self.levels.len(),
            current,
            progress,
        })
        .map_err(Error::Callback)
    }
}

/// Hands each retired level of a checkpointed walk to `on_retire` with its
/// adjacent successor, keeps its tally, and drops it. AC-whp-o1-1, D-1844.
///
/// The checkpointed walk kept every survivor of every level until the end,
/// only so its caller could rank them afterwards; a ranker that takes each
/// level as it retires needs at most two levels at once, which is what the
/// streamed uncheckpointed walk already holds.
pub type Retirement<'a> = &'a mut dyn FnMut(&Frontier, Option<&Frontier>);

struct Handing<'a> {
    tallies: Vec<keep::Tally>,
    streamed: u64,
    identity: [u8; 32],
    ladder: Ladder,
    live: &'a [u32],
    reporter: Reporter<'a>,
    on_retire: Retirement<'a>,
}

impl Handing<'_> {
    fn hand(&mut self, level: &Frontier, next: Option<&Frontier>) {
        (self.on_retire)(level, next);
        self.streamed = self
            .streamed
            .saturating_add(crate::len_u64(level.frequent.len()));
        self.tallies.push(keep::Tally::of(level));
    }
}

impl Sink for Handing<'_> {
    type Error = Error;
    fn report(&mut self, _: &Frontier, _: usize, _: u64) {}
    fn retire(&mut self, level: Frontier, next: Option<&Frontier>) {
        self.hand(&level, next);
    }
    fn checkpoint(&mut self, current: &Frontier, progress: &Progress) -> Result<(), Error> {
        (self.reporter)(&CheckpointView {
            identity: self.identity,
            ladder: self.ladder,
            live: self.live,
            retired: &[],
            retired_count: self.tallies.len(),
            current,
            progress,
        })
        .map_err(Error::Callback)
    }
}

impl Ladder {
    /// Start the shared retained walk, checkpointing before every next level.
    ///
    /// # Errors
    /// Refuses allocation or a failed checkpoint callback. No later level runs
    /// after callback failure; the caller may resume its last durable payload.
    pub fn walk_checkpointed(
        self,
        column: &Column,
        live: &[u32],
        identity: [u8; 32],
        reporter: Reporter<'_>,
    ) -> Result<Sweep, Error> {
        let levels = crate::reserved(crate::level_slots())?;
        let (current, progress) = self.first_level(column, live)?;
        self.finish_checkpointed(
            column,
            current,
            progress,
            Retaining {
                levels,
                identity,
                ladder: self,
                live,
                reporter,
            },
        )
    }

    /// Resume the same shared walk from a verified checkpoint.
    ///
    /// The existing boundary is reported again, allowing idempotent durable
    /// acknowledgement. Earlier levels are retained but never recomputed.
    /// Extinct and resource-halted checkpoints return their original outcome;
    /// a partial frontier never seeds another level. Every answer-bearing term
    /// of the configuration is exact; the support-lane count is scheduling only
    /// and is taken from the caller (see [`Checkpoint::validate_for`]).
    ///
    /// # Errors
    /// Refuses a foreign identity, column length, offered sequence or config,
    /// failed allocation or callback. Full column-byte identity is a caller duty.
    pub fn resume_checkpointed(
        self,
        column: &Column,
        live: &[u32],
        expected_identity: [u8; 32],
        mut checkpoint: Checkpoint,
        reporter: Reporter<'_>,
    ) -> Result<Sweep, Error> {
        checkpoint.validate_for(self, column, live, expected_identity)?;
        let current = checkpoint
            .levels
            .pop()
            .ok_or(Error::Invalid("no resume frontier"))?;
        checkpoint
            .levels
            .try_reserve_exact(crate::level_slots().saturating_sub(checkpoint.levels.len()))?;
        self.finish_checkpointed(
            column,
            current,
            checkpoint.progress,
            Retaining {
                levels: checkpoint.levels,
                identity: expected_identity,
                ladder: self,
                live,
                reporter,
            },
        )
    }

    /// [`Self::walk_checkpointed`] that hands each level to `on_retire` as it
    /// retires and keeps only its tally. The walk, every checkpoint the
    /// reporter sees (prefix and current level, byte for byte) and the levels
    /// handed on are exactly those of the retaining walk; only the survivors
    /// are no longer held. AC-whp-o1-1, D-1844.
    ///
    /// # Errors
    /// As [`Self::walk_checkpointed`].
    pub fn walk_checkpointed_streamed(
        self,
        column: &Column,
        live: &[u32],
        identity: [u8; 32],
        reporter: Reporter<'_>,
        on_retire: Retirement<'_>,
    ) -> Result<keep::Streamed, Error> {
        let tallies = crate::reserved(crate::level_slots())?;
        let (current, progress) = self.first_level(column, live)?;
        let mut sink = Handing {
            tallies,
            streamed: 0,
            identity,
            ladder: self,
            live,
            reporter,
            on_retire,
        };
        let tail = self.continue_walk(column, current, progress, &mut sink)?;
        Ok(streamed_of(sink, tail))
    }

    /// [`Self::resume_checkpointed`] that hands every level on as
    /// [`Self::walk_checkpointed_streamed`] does: first each restored earlier
    /// level, in depth order with its restored successor (none when that
    /// successor is the partial level a halt left), dropping each once it is
    /// handed on, then every level the walk retires. D-1844.
    ///
    /// # Errors
    /// As [`Self::resume_checkpointed`].
    pub fn resume_checkpointed_streamed(
        self,
        column: &Column,
        live: &[u32],
        expected_identity: [u8; 32],
        checkpoint: Checkpoint,
        reporter: Reporter<'_>,
        on_retire: Retirement<'_>,
    ) -> Result<keep::Streamed, Error> {
        checkpoint.validate_for(self, column, live, expected_identity)?;
        let mut sink = Handing {
            tallies: crate::reserved(crate::level_slots())?,
            streamed: 0,
            identity: expected_identity,
            ladder: self,
            live,
            reporter,
            on_retire,
        };
        let halted_at = checkpoint.progress.halted.map(|halt| halt.k);
        let mut restored = checkpoint.levels.into_iter();
        let current = hand_restored(&mut sink, halted_at, || Ok(restored.next()))?;
        let tail = self.continue_walk(column, current, checkpoint.progress, &mut sink)?;
        Ok(streamed_of(sink, tail))
    }

    /// [`Self::resume_checkpointed_streamed`] from a payload still being
    /// decoded: each restored level is decoded, checked, offered to `check`,
    /// handed on with its successor and dropped before the level after its
    /// successor is decoded, so the restore holds at most two levels, not the
    /// whole restored history. AC-whp-o1-1, D-4520.
    ///
    /// The levels handed on, the boundaries reported and the result are those
    /// of [`Self::resume_checkpointed_streamed`] over
    /// [`Checkpoint::read_from`] of the same payload.
    ///
    /// # Errors
    /// As [`Self::resume_checkpointed_streamed`] and [`Restoring::next_level`];
    /// a refusal by `check` is [`Error::Callback`]. A level refused after
    /// earlier ones were handed on refuses the whole resume, and no boundary
    /// is reported before every restored level has been checked.
    #[expect(
        clippy::too_many_arguments,
        reason = "the streamed resume's six inputs plus the payload and its caller's per-level check"
    )]
    pub fn resume_restoring_streamed<R: Read>(
        self,
        column: &Column,
        live: &[u32],
        expected_identity: [u8; 32],
        mut restoring: Restoring<'_, R>,
        check: &mut dyn FnMut(&Frontier) -> Result<(), String>,
        reporter: Reporter<'_>,
        on_retire: Retirement<'_>,
    ) -> Result<keep::Streamed, Error> {
        restoring.validate_for(self, column, live, expected_identity)?;
        let mut sink = Handing {
            tallies: crate::reserved(crate::level_slots())?,
            streamed: 0,
            identity: expected_identity,
            ladder: self,
            live,
            reporter,
            on_retire,
        };
        let halted_at = restoring.head.progress.halted.map(|halt| halt.k);
        let current = hand_restored(&mut sink, halted_at, || {
            let level = restoring.next_level()?;
            if let Some(level) = &level {
                check(level).map_err(Error::Callback)?;
            }
            Ok(level)
        })?;
        let tail = self.continue_walk(column, current, restoring.head.progress, &mut sink)?;
        Ok(streamed_of(sink, tail))
    }

    fn finish_checkpointed(
        self,
        column: &Column,
        current: Frontier,
        progress: Progress,
        mut sink: Retaining<'_>,
    ) -> Result<Sweep, Error> {
        let tail = self.continue_walk(column, current, progress, &mut sink)?;
        Ok(Sweep {
            levels: sink.levels,
            excluded: tail.excluded,
            bars: tail.bars,
            min_hits: tail.min_hits,
            halted: tail.halted,
        })
    }
}

/// Hands every restored level but the last to `sink` in depth order, each
/// with the level after it as its successor (none when that level is the
/// partial one a halt left), and returns the last: the resume frontier. Each
/// level is dropped once it has been handed on, so at most two are held.
fn hand_restored(
    sink: &mut Handing<'_>,
    halted_at: Option<u32>,
    mut next: impl FnMut() -> Result<Option<Frontier>, Error>,
) -> Result<Frontier, Error> {
    let mut held = next()?.ok_or(Error::Invalid("no resume frontier"))?;
    while let Some(following) = next()? {
        sink.hand(
            &held,
            Some(&following).filter(|level| halted_at != Some(level.k)),
        );
        held = following;
    }
    Ok(held)
}

fn streamed_of(sink: Handing<'_>, tail: crate::Tail) -> keep::Streamed {
    keep::Streamed {
        levels: sink.tallies,
        excluded: tail.excluded,
        bars: tail.bars,
        min_hits: tail.min_hits,
        halted: tail.halted,
        streamed: sink.streamed,
    }
}

fn words<const N: usize>(writer: &mut impl Write, values: [u64; N]) -> Result<(), Error> {
    for value in values {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn write_halt(writer: &mut impl Write, halt: Option<Halt>) -> Result<(), Error> {
    let values = halt.map_or([0; 7], |halt| {
        [
            1,
            u64::from(halt.k),
            crate::len_u64(halt.candidates),
            crate::len_u64(halt.ceiling),
            halt.pairs,
            halt.pair_budget,
            match halt.breach {
                Breach::Candidates => 1,
                Breach::Pairs => 2,
                Breach::Memory => 3,
                Breach::Workers => 4,
            },
        ]
    });
    words(writer, values)
}

struct Decoder<'a, R> {
    reader: &'a mut R,
    remaining: u64,
}

impl<R: Read> Decoder<'_, R> {
    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.remaining = self
            .remaining
            .checked_sub(crate::len_u64(N))
            .ok_or(Error::Invalid("payload exceeds byte bound"))?;
        let mut bytes = [0; N];
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    fn word(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
    fn size(&mut self) -> Result<usize, Error> {
        usize::try_from(self.word()?).map_err(|_| Error::Invalid("size leaves platform"))
    }
    fn position(&mut self) -> Result<u32, Error> {
        u32::try_from(self.word()?).map_err(|_| Error::Invalid("position leaves type"))
    }
    fn count(&self, count: usize, stride: u64) -> Result<(), Error> {
        if crate::len_u64(count) > self.remaining / stride {
            return Err(Error::Invalid("count exceeds remaining bytes"));
        }
        Ok(())
    }
    fn header(&mut self, expected: [u8; 32]) -> Result<(Checkpoint, (usize, usize, usize)), Error> {
        if self.remaining < HEADER_BYTES
            || self.bytes::<8>()? != MAGIC
            || self.word()? != VERSION
            || self.word()? != u64::from(vocab::VOCAB_VERSION)
        {
            return Err(Error::Invalid("unsupported header/version"));
        }
        let identity = self.bytes()?;
        if identity != expected {
            return Err(Error::Invalid("foreign identity"));
        }
        let ladder = Ladder {
            min_hits: self.word()?,
            ceiling: self.size()?,
            pair_budget: self.word()?,
            support_lanes: self.size()?,
        };
        let progress = Progress {
            excluded: Vec::new(),
            bars: self.word()?,
            admitted: self.size()?,
            pairs: self.word()?,
            halted: self.halt()?,
        };
        let counts = (self.size()?, self.size()?, self.size()?);
        if counts.2 == 0 || counts.2 > ConditionMask::BITS as usize + 1 {
            return Err(Error::Invalid("invalid level count"));
        }
        Ok((
            Checkpoint {
                identity,
                ladder,
                live: Vec::new(),
                levels: Vec::new(),
                progress,
            },
            counts,
        ))
    }
    fn halt(&mut self) -> Result<Option<Halt>, Error> {
        let present = self.word()?;
        let k = self.position()?;
        let candidates = self.size()?;
        let ceiling = self.size()?;
        let pairs = self.word()?;
        let pair_budget = self.word()?;
        let tag = self.word()?;
        if present == 0
            && [
                u64::from(k),
                crate::len_u64(candidates),
                crate::len_u64(ceiling),
                pairs,
                pair_budget,
                tag,
            ] == [0; 6]
        {
            return Ok(None);
        }
        if present != 1 {
            return Err(Error::Invalid("noncanonical halt flag"));
        }
        let breach = match tag {
            1 => Breach::Candidates,
            2 => Breach::Pairs,
            3 => Breach::Memory,
            4 => Breach::Workers,
            _ => return Err(Error::Invalid("unknown halt reason")),
        };
        Ok(Some(Halt {
            k,
            candidates,
            ceiling,
            pairs,
            pair_budget,
            breach,
        }))
    }
    fn live(&mut self, count: usize) -> Result<Vec<u32>, Error> {
        self.count(count, 8)?;
        let mut live = crate::reserved(count)?;
        for _ in 0..count {
            live.push(self.position()?);
        }
        Ok(live)
    }
    fn excluded(&mut self, count: usize) -> Result<Vec<Excluded>, Error> {
        self.count(count, 24)?;
        let mut excluded = crate::reserved(count)?;
        for _ in 0..count {
            let position = self.position()?;
            let hits = self.word()?;
            let (reason, support) = match self.word()? {
                0 if hits == 0 => (Why::NotLive, None),
                1 => (Why::AlwaysFalse, Some(hits)),
                2 => (Why::AlwaysTrue, Some(hits)),
                _ => return Err(Error::Invalid("unknown exclusion")),
            };
            excluded.push(Excluded {
                position,
                support,
                reason,
            });
        }
        Ok(excluded)
    }
    fn level(&mut self) -> Result<Frontier, Error> {
        let k = self.position()?;
        let length = self.size()?;
        let mut level = Frontier {
            k,
            generated: self.word()?,
            duplicates: self.word()?,
            excluded: self.word()?,
            pruned: self.word()?,
            infrequent: self.word()?,
            frequent: Vec::new(),
        };
        self.count(length, ITEM_BYTES)?;
        level.frequent = crate::reserved(length)?;
        for _ in 0..length {
            let mut mask = [0_u64; 6];
            for word in &mut mask {
                *word = self.word()?;
            }
            level.frequent.push(Itemset {
                mask: ConditionMask::from_words(mask),
                hits: self.word()?,
            });
        }
        Ok(level)
    }
}

fn validate_level(
    level: &Frontier,
    offset: usize,
    count: usize,
    bars: u64,
    min_hits: u64,
    allowed: ConditionMask,
) -> Result<(), Error> {
    let accounted = [
        level.duplicates,
        level.excluded,
        level.pruned,
        level.infrequent,
        crate::len_u64(level.frequent.len()),
    ]
    .into_iter()
    .try_fold(0_u64, u64::checked_add);
    if u64::from(level.k) != crate::len_u64(offset) + 1
        || accounted != Some(level.generated)
        || (offset + 1 < count && level.frequent.is_empty())
        || (offset > 0 && (level.duplicates != 0 || level.excluded != 0))
        || (offset == 0 && level.pruned != 0)
    {
        return Err(Error::Invalid("level sequence or reconciliation"));
    }
    let mut previous = None;
    for item in &level.frequent {
        if item.mask.popcount() != level.k
            || !allowed.hits(&item.mask)
            || item.hits < min_hits
            || item.hits >= bars
            || previous.is_some_and(|words| words >= item.mask.words())
        {
            return Err(Error::Invalid("invalid or noncanonical survivor"));
        }
        previous = Some(item.mask.words());
    }
    Ok(())
}
