//! Version-separated, append-only sweep attempts and retained signal evidence.
//!
//! A durable start precedes engine work. Each attempt owns fixed-stride depth
//! and ranking files; a terminal record follows their durability barriers.
//! Process death leaves `Running`, never an inferred successful completion.
//! Initial file admission, one event/level append and bounded-page addressing
//! use fixed work; writing or reading N candidate rows still costs O(N).

use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

const HEADER: u64 = 16;
const EVENTS: [u8; 8] = *b"BRSWAT01";
const DEPTHS: [u8; 8] = *b"BRSWDP01";
const RANKS: [u8; 8] = *b"BRSWRK01";
const EVENT_BYTES: usize = 96;
const DEPTH_BYTES: usize = 128;
const RANK_BYTES: usize = 200;

/// The computation this attempt actually performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Fixed-threshold AND search and signal ranking.
    Sweep,
    /// AND search, exit grids and requested institutional validation.
    Audit,
    /// Threshold-search orchestration.
    AutoSearch,
    /// One exact threshold-search probe.
    AutoProbe,
    /// One explicitly versioned Boolean expression, never a legacy AND mask.
    Expression,
    /// Input-bound condition folding before a search threshold is known.
    Preparation,
    /// Exhaustive fixed-language Boolean grammar traversal, separate from AND.
    ExpressionSearch,
    /// Stored chronological replay of actual institutionally selected sources.
    GlobalReplay,
    /// One exact strategy's OOS stream, not whole-plan publication or admission.
    GlobalReplayStream,
    /// Independent historical checksum audit, never a market sweep or admission policy.
    ChecksumAudit,
    /// Complete explicit Boolean catalog; not exhaustive grammar or statistical admission.
    BooleanCandidates,
    /// Complete Boolean coordinate statistics; not strategy admission.
    BooleanStatistics,
    /// Explicit program-aware research-policy admission.
    BooleanAdmission,
    /// Later strict OHLCV replay of frozen Boolean training coordinates.
    BooleanOos,
    /// Finite-scope later-window research qualification, never trading approval.
    BooleanQualification,
    /// Source-bound daily and weekly index consistency assessment.
    IndexConsistency,
    /// Single completed-signal-candle stop, with both directions and fill readings.
    IndexStop,
    /// Native signal-stop institutional and daily/weekly qualification.
    IndexStopQualification,
}

impl Operation {
    /// Stable API label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sweep => "sweep",
            Self::Audit => "audit",
            Self::AutoSearch => "auto-search",
            Self::AutoProbe => "auto-probe",
            Self::Expression => "expression",
            Self::Preparation => "preparation",
            Self::ExpressionSearch => "expression-search",
            Self::GlobalReplay => "global-replay",
            Self::GlobalReplayStream => "global-replay-stream",
            Self::ChecksumAudit => "checksum-audit",
            Self::BooleanCandidates => "boolean-candidates",
            Self::BooleanStatistics => "boolean-statistics",
            Self::BooleanAdmission => "boolean-admission",
            Self::BooleanOos => "boolean-oos",
            Self::BooleanQualification => "boolean-qualification",
            Self::IndexConsistency => "index-consistency",
            Self::IndexStop => "index-stop",
            Self::IndexStopQualification => "index-stop-qualification",
        }
    }
    const fn byte(self) -> u8 {
        match self {
            Self::Sweep => 1,
            Self::Audit => 2,
            Self::AutoSearch => 3,
            Self::AutoProbe => 4,
            Self::Expression => 5,
            Self::Preparation => 6,
            Self::ExpressionSearch => 7,
            Self::GlobalReplay => 8,
            Self::GlobalReplayStream => 9,
            Self::ChecksumAudit => 10,
            Self::BooleanCandidates => 11,
            Self::BooleanStatistics => 12,
            Self::BooleanAdmission => 13,
            Self::BooleanOos => 14,
            Self::BooleanQualification => 15,
            Self::IndexConsistency => 16,
            Self::IndexStop => 17,
            Self::IndexStopQualification => 18,
        }
    }
    fn decode(value: u8) -> Result<Self, String> {
        match value {
            1 => Ok(Self::Sweep),
            2 => Ok(Self::Audit),
            3 => Ok(Self::AutoSearch),
            4 => Ok(Self::AutoProbe),
            5 => Ok(Self::Expression),
            6 => Ok(Self::Preparation),
            7 => Ok(Self::ExpressionSearch),
            8 => Ok(Self::GlobalReplay),
            9 => Ok(Self::GlobalReplayStream),
            10 => Ok(Self::ChecksumAudit),
            11 => Ok(Self::BooleanCandidates),
            12 => Ok(Self::BooleanStatistics),
            13 => Ok(Self::BooleanAdmission),
            14 => Ok(Self::BooleanOos),
            15 => Ok(Self::BooleanQualification),
            16 => Ok(Self::IndexConsistency),
            17 => Ok(Self::IndexStop),
            18 => Ok(Self::IndexStopQualification),
            _ => Err(format!("unknown sweep operation {value}")),
        }
    }
}

/// Durable attempt state; age alone never changes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Completion {
    /// Start was recorded; no terminal record is present.
    Running,
    /// The requested computation and evidence publication completed.
    Completed,
    /// The engine explicitly stopped at a resource bound.
    Halted,
    /// An input, execution or persistence refusal prevented completion.
    Refused,
}

impl Completion {
    /// Stable API label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Halted => "halted",
            Self::Refused => "refused",
        }
    }
    const fn byte(self) -> u8 {
        match self {
            Self::Running => 0,
            Self::Completed => 1,
            Self::Halted => 2,
            Self::Refused => 3,
        }
    }
    fn decode(value: u8) -> Result<Self, String> {
        match value {
            0 => Ok(Self::Running),
            1 => Ok(Self::Completed),
            2 => Ok(Self::Halted),
            3 => Ok(Self::Refused),
            _ => Err(format!("unknown sweep completion {value}")),
        }
    }
}

/// One completed ladder level, including its exact accounting buckets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DepthRow {
    /// Result depth, never a caller-supplied search limit.
    pub k: u64,
    /// Generated candidates.
    pub generated: u64,
    /// Repeated input positions.
    pub duplicates: u64,
    /// Subset-pruned candidates.
    pub pruned: u64,
    /// Candidates below support.
    pub infrequent: u64,
    /// Surviving candidates.
    pub frequent: u64,
    /// Cumulative candidate admissions.
    pub admitted: u64,
    /// Cumulative pair iterations.
    pub pairs: u64,
    /// Whether the level's exact buckets reconcile.
    pub reconciles: bool,
    /// Always-true or always-false condition exclusions.
    pub excluded: u64,
}

impl DepthRow {
    /// Capture a frontier at its reporting boundary.
    #[must_use]
    pub fn of(level: &engine::Frontier, admitted: usize, pairs: u64) -> Self {
        Self {
            k: u64::from(level.k),
            generated: level.generated,
            duplicates: level.duplicates,
            pruned: level.pruned,
            infrequent: level.infrequent,
            frequent: u64::try_from(level.frequent.len()).unwrap_or(u64::MAX),
            admitted: u64::try_from(admitted).unwrap_or(u64::MAX),
            pairs,
            reconciles: level.reconciles(),
            excluded: level.excluded,
        }
    }
    pub(crate) fn words(self) -> [u64; 10] {
        [
            self.k,
            self.generated,
            self.duplicates,
            self.pruned,
            self.infrequent,
            self.frequent,
            self.admitted,
            self.pairs,
            u64::from(self.reconciles),
            self.excluded,
        ]
    }
    pub(crate) fn from_words(w: [u64; 10]) -> Result<Self, String> {
        let [
            k,
            generated,
            duplicates,
            pruned,
            infrequent,
            frequent,
            admitted,
            pairs,
            reconciles,
            excluded,
        ] = w;
        if reconciles > 1 {
            return Err("invalid depth reconciliation byte".to_owned());
        }
        Ok(Self {
            k,
            generated,
            duplicates,
            pruned,
            infrequent,
            frequent,
            admitted,
            pairs,
            reconciles: reconciles == 1,
            excluded,
        })
    }
}

/// Full-precision retained signal evidence; this is not a priced trade row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RankedRow {
    /// One-based rank in the displayed retained result.
    pub rank: u64,
    /// Exact legacy AND combination; expression matches use their own format.
    pub mask_words: [u64; 6],
    /// Historical support count.
    pub hits: u64,
    /// Signals with measurable forward outcomes.
    pub observations: u64,
    /// IEEE bits of the unrounded statistical mean in paisa.
    pub mean_bits: u64,
    /// IEEE bits of the unrounded test statistic.
    pub t_bits: u64,
    /// Unpriceable/refused outcome count.
    pub refused: u64,
    /// Misaligned outcome count.
    pub mismatched: u64,
    /// Positive outcomes.
    pub wins: u64,
    /// Negative outcomes; flat outcomes are neither wins nor losses.
    pub losses: u64,
    /// IEEE bits of summed positive moves.
    pub win_sum_bits: u64,
    /// IEEE bits of summed negative moves.
    pub loss_sum_bits: u64,
    /// IEEE bits of summed adverse excursions.
    pub adverse_sum_bits: u64,
    /// IEEE bits of summed favourable excursions.
    pub favourable_sum_bits: u64,
}

impl RankedRow {
    /// Preserve a ranked candidate without rounding any stored statistic.
    #[must_use]
    pub fn from_scored(rank: u64, row: &runner::rank::Scored) -> Self {
        Self {
            rank,
            mask_words: row.mask.words(),
            hits: row.hits,
            observations: row.edge.n,
            mean_bits: row.edge.mean_paisa.to_bits(),
            t_bits: row.edge.t.to_bits(),
            refused: row.edge.refused,
            mismatched: row.edge.mismatched,
            wins: row.edge.wins,
            losses: row.edge.losses,
            win_sum_bits: row.edge.win_sum.to_bits(),
            loss_sum_bits: row.edge.loss_sum.to_bits(),
            adverse_sum_bits: row.edge.adverse_sum.to_bits(),
            favourable_sum_bits: row.edge.favourable_sum.to_bits(),
        }
    }
    fn words(self) -> [u64; 19] {
        let [word0, word1, word2, word3, word4, word5] = self.mask_words;
        [
            self.rank,
            word0,
            word1,
            word2,
            word3,
            word4,
            word5,
            self.hits,
            self.observations,
            self.mean_bits,
            self.t_bits,
            self.refused,
            self.mismatched,
            self.wins,
            self.win_sum_bits,
            self.loss_sum_bits,
            self.adverse_sum_bits,
            self.favourable_sum_bits,
            self.losses,
        ]
    }
    fn from_words(payload: [u64; 19]) -> Self {
        let [
            rank,
            word0,
            word1,
            word2,
            word3,
            word4,
            word5,
            hits,
            observations,
            mean_bits,
            t_bits,
            refused,
            mismatched,
            wins,
            win_sum_bits,
            loss_sum_bits,
            adverse_sum_bits,
            favourable_sum_bits,
            losses,
        ] = payload;
        Self {
            rank,
            mask_words: [word0, word1, word2, word3, word4, word5],
            hits,
            observations,
            mean_bits,
            t_bits,
            refused,
            mismatched,
            wins,
            win_sum_bits,
            loss_sum_bits,
            adverse_sum_bits,
            favourable_sum_bits,
            losses,
        }
    }
}

/// Validated latest lifecycle state and detail cardinalities for one attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Evidence {
    /// Exact run identity.
    pub identity: [u8; 32],
    /// Durable global journal sequence of the start record.
    pub attempt: u64,
    /// Computation family.
    pub operation: Operation,
    /// Explicit lifecycle state.
    pub completion: Completion,
    /// Start wall clock, microseconds since epoch.
    pub started_micros: i64,
    /// Last lifecycle event wall clock.
    pub updated_micros: i64,
    /// Number of durable fixed-stride depth rows.
    pub depth_rows: u64,
    /// Number of durable fixed-stride retained signal rows.
    pub ranked_rows: u64,
    /// Whether a retained-ranking file was explicitly published, even if empty.
    pub ranked_available: bool,
    /// Actual audit option. Requested validation is not proof it passed.
    pub validation_requested: Option<bool>,
}

/// Writer authority whose start has already reached stable storage.
pub struct Attempt {
    root: PathBuf,
    evidence: Evidence,
    failure: Mutex<Option<String>>,
    ranked_published: AtomicBool,
    acknowledged_depths: AtomicU64,
    acknowledged_ranks: AtomicU64,
    acknowledged_ranking: AtomicBool,
    depth_digest: Mutex<brutex_core::blake3::Hasher>,
    ranked_digest: Mutex<brutex_core::blake3::Hasher>,
    terminal: bool,
}

/// Record a new attempt durably before any sweep/probe is performed.
///
/// # Errors
/// Refuses invalid/torn evidence, missing durability barriers and I/O failures.
pub fn begin(root: &Path, identity: [u8; 32], operation: Operation) -> Result<Attempt, String> {
    begin_with_validation(root, identity, operation, None)
}

/// Record an audit's actual validation request alongside its durable identity.
///
/// # Errors
/// Refuses validation metadata on non-audit operations and every I/O refusal
/// from [`begin`]. It never labels requested validation as passed admission.
pub fn begin_with_validation(
    root: &Path,
    identity: [u8; 32],
    operation: Operation,
    validation_requested: Option<bool>,
) -> Result<Attempt, String> {
    if validation_requested.is_some() && operation != Operation::Audit {
        return Err("validation request metadata belongs only to an audit attempt".to_owned());
    }
    let timestamp = now()?;
    let mut evidence = Evidence {
        identity,
        attempt: 0,
        operation,
        completion: Completion::Running,
        started_micros: timestamp,
        updated_micros: timestamp,
        depth_rows: 0,
        ranked_rows: 0,
        ranked_available: false,
        validation_requested,
    };
    let base = base(root);
    durable_directory(&base)?;
    evidence.attempt = append_event(&base.join("attempts.bin"), evidence, true)?;
    reserve_start(root, evidence)?;
    // Once reservation is durable, every ordinary admission refusal gets the
    // same best-effort Refused terminal as an interrupted admitted attempt.
    // An I/O failure may still prevent that terminal; no completion is invented.
    let attempt = Attempt {
        root: root.to_path_buf(),
        evidence,
        failure: Mutex::new(None),
        ranked_published: AtomicBool::new(false),
        acknowledged_depths: AtomicU64::new(0),
        acknowledged_ranks: AtomicU64::new(0),
        acknowledged_ranking: AtomicBool::new(false),
        depth_digest: Mutex::new(brutex_core::blake3::Hasher::new()),
        ranked_digest: Mutex::new(brutex_core::blake3::Hasher::new()),
        terminal: false,
    };
    let own = directory(root, &identity);
    durable_directory(&own)?;
    append_event(&lifecycle_path(root, &evidence), evidence, false)?;
    append_identity_start(&own.join("starts.bin"), evidence)?;
    emit(evidence);
    Ok(attempt)
}

impl Attempt {
    /// Globally unique persisted attempt sequence.
    #[must_use]
    pub const fn token(&self) -> u64 {
        self.evidence.attempt
    }
    /// Exact identity admitted before computation.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.evidence.identity
    }
    /// Save one completed depth with a durability barrier.
    ///
    /// # Errors
    /// Refuses a prior write failure, malformed file or current I/O failure.
    pub fn level(&self, row: DepthRow) -> Result<(), String> {
        self.check()?;
        let raw = encode_words::<10, DEPTH_BYTES>(self.evidence, row.words())?;
        let mut digest = self
            .depth_digest
            .lock()
            .map_err(|_| "depth evidence digest lock poisoned".to_owned())?;
        self.remember(
            append_row(&self.detail("levels"), DEPTHS, &raw).and_then(|_| {
                self.acknowledged_depths
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                        count.checked_add(1)
                    })
                    .map(|_| ())
                    .map_err(|_| "acknowledged depth cardinality overflowed".to_owned())
            }),
        )?;
        digest.update(&raw);
        Ok(())
    }
    /// Save the exact displayed signal ranking, including a legitimate zero rows.
    ///
    /// # Errors
    /// Refuses duplicate publication, invalid ranks, malformed files or I/O errors.
    pub fn ranked(&self, rows: &[RankedRow]) -> Result<(), String> {
        self.check()?;
        if self.ranked_published.swap(true, Ordering::AcqRel) {
            return self.remember(Err(
                "this attempt already published ranked evidence; no rows were replaced".to_owned(),
            ));
        }
        let mut digest = brutex_core::blake3::Hasher::new();
        let result = (|| {
            for (index, row) in rows.iter().enumerate() {
                if row.rank != u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1) {
                    return Err("ranked evidence must have contiguous one-based ranks".to_owned());
                }
            }
            let path = self.detail("ranked");
            let mut file = open_append(&path)?;
            file.lock().map_err(io_error)?;
            let result = (|| {
                let count = shape::<RANK_BYTES>(&mut file, &path, RANKS, true)?;
                if count != 0 {
                    return Err(
                        "this attempt already published ranked evidence; no rows were replaced"
                            .to_owned(),
                    );
                }
                file.seek(SeekFrom::End(0)).map_err(io_error)?;
                for row in rows {
                    let raw = encode_words::<19, RANK_BYTES>(self.evidence, row.words())?;
                    file.write_all(&raw).map_err(io_error)?;
                    digest.update(&raw);
                }
                file.sync_all().map_err(io_error)
            })();
            let released = file.unlock().map_err(io_error);
            result.and(released)
        })();
        self.remember(result)?;
        *self
            .ranked_digest
            .lock()
            .map_err(|_| "ranked evidence digest lock poisoned".to_owned())? = digest;
        self.acknowledged_ranks.store(
            u64::try_from(rows.len()).map_err(|_| "rank cardinality exceeds u64".to_owned())?,
            Ordering::Release,
        );
        self.acknowledged_ranking.store(true, Ordering::Release);
        Ok(())
    }
    /// Seal one explicit terminal status after successful evidence writes.
    ///
    /// # Errors
    /// Refuses `Running` as terminal, any earlier evidence failure or I/O failure.
    pub fn finish(mut self, status: Completion) -> Result<(), String> {
        if status == Completion::Running {
            return Err("Running is not a terminal completion".to_owned());
        }
        self.check()?;
        self.write_terminal(status)?;
        self.terminal = true;
        Ok(())
    }
    fn detail(&self, kind: &str) -> PathBuf {
        directory(&self.root, &self.evidence.identity)
            .join(format!("{}-{kind}.bin", self.evidence.attempt))
    }
    pub(crate) fn check(&self) -> Result<(), String> {
        self.failure
            .lock()
            .map_err(|_| "sweep evidence failure lock poisoned".to_owned())?
            .clone()
            .map_or(Ok(()), Err)
    }
    fn remember(&self, result: Result<(), String>) -> Result<(), String> {
        if let Err(why) = &result {
            *self
                .failure
                .lock()
                .map_err(|_| "sweep evidence failure lock poisoned".to_owned())? =
                Some(why.clone());
        }
        result
    }
    fn write_terminal(&self, status: Completion) -> Result<(), String> {
        let mut evidence = self.evidence;
        evidence = measured_details(&self.root, evidence, u64::MAX)?;
        if evidence.depth_rows != self.acknowledged_depths.load(Ordering::Acquire)
            || evidence.ranked_rows != self.acknowledged_ranks.load(Ordering::Acquire)
            || evidence.ranked_available != self.acknowledged_ranking.load(Ordering::Acquire)
        {
            return Err(
                "acknowledged sweep evidence was lost or changed before its terminal seal"
                    .to_owned(),
            );
        }
        if evidence.depth_rows != 0 {
            verify_terminal_detail::<DEPTH_BYTES>(
                &self.detail("levels"),
                DEPTHS,
                evidence,
                evidence.depth_rows,
                self.depth_digest
                    .lock()
                    .map_err(|_| "depth evidence digest lock poisoned".to_owned())?
                    .finalize(),
            )?;
        }
        if evidence.ranked_available {
            verify_terminal_detail::<RANK_BYTES>(
                &self.detail("ranked"),
                RANKS,
                evidence,
                evidence.ranked_rows,
                self.ranked_digest
                    .lock()
                    .map_err(|_| "ranked evidence digest lock poisoned".to_owned())?
                    .finalize(),
            )?;
        }
        evidence.completion = status;
        evidence.updated_micros = now()?;
        append_event(&lifecycle_path(&self.root, &evidence), evidence, false)?;
        append_event(&base(&self.root).join("attempts.bin"), evidence, false)?;
        emit(evidence);
        Ok(())
    }
}

/// One final sequential verification, using one fixed-size row buffer.
/// This is O(saved rows), outside the candidate and per-append operations.
fn verify_terminal_detail<const N: usize>(
    path: &Path,
    magic: [u8; 8],
    evidence: Evidence,
    expected: u64,
    expected_digest: [u8; 32],
) -> Result<(), String> {
    let mut file = File::open(path).map_err(io_error)?;
    file.lock_shared().map_err(io_error)?;
    let result = (|| {
        let before = crate::result_set::file_generation(&file, path)?;
        if shape::<N>(&mut file, path, magic, false)? != expected {
            return Err(
                "acknowledged child cardinality changed during terminal verification".to_owned(),
            );
        }
        file.seek(SeekFrom::Start(HEADER)).map_err(io_error)?;
        let mut digest = brutex_core::blake3::Hasher::new();
        for _ in 0..expected {
            let mut row = [0_u8; N];
            file.read_exact(&mut row).map_err(io_error)?;
            verify(&row)?;
            digest.update(&row);
            if row.get(..32) != Some(evidence.identity.as_slice())
                || row.get(32..40) != Some(evidence.attempt.to_le_bytes().as_slice())
            {
                return Err("terminal child belongs to a different identity or attempt".to_owned());
            }
        }
        if digest.finalize() != expected_digest {
            return Err("acknowledged child bytes changed before their terminal seal".to_owned());
        }
        let after = crate::result_set::file_generation(&file, path)?;
        crate::result_set::require_generation_unchanged(before, after, path)
    })();
    let released = file.unlock().map_err(io_error);
    result.and(released)
}

impl Drop for Attempt {
    fn drop(&mut self) {
        if !self.terminal
            && let Err(why) = self.write_terminal(Completion::Refused)
        {
            crate::note(
                &telemetry::Event::error(
                    "cli.lifecycle",
                    "sweep evidence terminal could not be recorded",
                )
                .with("identity", hex(&self.evidence.identity).as_str())
                .with("attempt", self.evidence.attempt)
                .with("reason", why.as_str()),
            );
        }
    }
}

/// Read the latest attempt for one identity without scanning its history.
///
/// # Errors
/// Refuses corrupt/torn/over-limit lifecycle or detail evidence.
pub fn read(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Option<Evidence>, String> {
    let path = directory(root, &identity).join("starts.bin");
    let Some(start) = last_event(&path, max_bytes)? else {
        return Ok(None);
    };
    let evidence = last_event(&lifecycle_path(root, &start), max_bytes)?
        .ok_or_else(|| "latest sweep attempt has no durable lifecycle".to_owned())?;
    if evidence.attempt != start.attempt {
        return Err("sweep lifecycle attempt differs from its latest start".to_owned());
    }
    if evidence.identity != identity {
        return Err("lifecycle identity differs from its path".to_owned());
    }
    require_start(root, evidence, max_bytes)?;
    checked_details(root, evidence, max_bytes).map(Some)
}

/// Read one exact historical attempt without substituting a newer same-ID run.
///
/// # Errors
/// Invalid identity/token, missing durable start, corrupt details or I/O failure.
pub fn read_attempt(
    root: &Path,
    identity: [u8; 32],
    attempt: u64,
    max_bytes: u64,
) -> Result<Option<Evidence>, String> {
    if attempt == 0 {
        return Err("sweep attempt must be nonzero".to_owned());
    }
    let path = directory(root, &identity).join(format!("{attempt}-lifecycle.bin"));
    let Some(evidence) = last_event(&path, max_bytes)? else {
        return Ok(None);
    };
    if evidence.identity != identity || evidence.attempt != attempt {
        return Err("exact attempt differs from its identity/path".to_owned());
    }
    require_start(root, evidence, max_bytes)?;
    checked_details(root, evidence, max_bytes).map(Some)
}

/// Read the latest global start/terminal event at a fixed byte offset.
/// This describes one run or probe, not an entire multi-rung command.
///
/// # Errors
/// Refuses corrupt/torn/over-limit evidence or I/O failures.
pub fn latest(root: &Path, max_bytes: u64) -> Result<Option<Evidence>, String> {
    let Some(event) = last_event(&base(root).join("attempts.bin"), max_bytes)? else {
        return Ok(None);
    };
    require_start(root, event, max_bytes)?;
    checked_details(root, event, max_bytes).map(Some)
}

/// Read a bounded page of persisted depth evidence.
///
/// # Errors
/// Refuses invalid bounds, corrupt rows, changed attempts or I/O failures.
pub fn depth_page(
    root: &Path,
    evidence: &Evidence,
    offset: u64,
    limit: usize,
    max_bytes: u64,
) -> Result<Vec<DepthRow>, String> {
    page::<DEPTH_BYTES>(root, evidence, "levels", DEPTHS, offset, limit, max_bytes)?
        .into_iter()
        .map(|raw| DepthRow::from_words(decode_words::<10, DEPTH_BYTES>(&raw)?))
        .collect()
}

/// Read a bounded page of full-precision retained signal evidence.
///
/// # Errors
/// Refuses invalid bounds, corrupt rows, changed attempts or I/O failures.
pub fn ranked_page(
    root: &Path,
    evidence: &Evidence,
    offset: u64,
    limit: usize,
    max_bytes: u64,
) -> Result<Vec<RankedRow>, String> {
    page::<RANK_BYTES>(root, evidence, "ranked", RANKS, offset, limit, max_bytes)?
        .iter()
        .map(|raw| decode_words::<19, RANK_BYTES>(raw).map(RankedRow::from_words))
        .collect()
}

fn measured_details(
    root: &Path,
    mut evidence: Evidence,
    max_bytes: u64,
) -> Result<Evidence, String> {
    evidence.depth_rows =
        count_optional::<DEPTH_BYTES>(&detail_path(root, &evidence, "levels"), DEPTHS, max_bytes)?;
    let ranked = detail_path(root, &evidence, "ranked");
    evidence.ranked_available = ranked.try_exists().map_err(io_error)?;
    evidence.ranked_rows = count_optional::<RANK_BYTES>(&ranked, RANKS, max_bytes)?;
    Ok(evidence)
}

fn checked_details(root: &Path, evidence: Evidence, max_bytes: u64) -> Result<Evidence, String> {
    let measured = measured_details(root, evidence, max_bytes)?;
    if evidence.completion != Completion::Running
        && (evidence.depth_rows != measured.depth_rows
            || evidence.ranked_rows != measured.ranked_rows
            || evidence.ranked_available != measured.ranked_available)
    {
        return Err("terminal sweep evidence disagrees with its sealed child cardinalities or availability; missing or changed detail is not an empty result".to_owned());
    }
    Ok(measured)
}

fn start_path(root: &Path, attempt: u64) -> PathBuf {
    base(root).join(format!("{attempt}-start.bin"))
}

fn lifecycle_path(root: &Path, evidence: &Evidence) -> PathBuf {
    directory(root, &evidence.identity).join(format!("{}-lifecycle.bin", evidence.attempt))
}

fn append_identity_start(path: &Path, evidence: Evidence) -> Result<(), String> {
    let mut file = open_append(path)?;
    file.lock().map_err(io_error)?;
    let result = (|| {
        let count = shape::<EVENT_BYTES>(&mut file, path, EVENTS, true)?;
        if count > 0 {
            let mut previous = [0_u8; EVENT_BYTES];
            file.seek(SeekFrom::Start(HEADER + (count - 1) * EVENT_BYTES as u64))
                .and_then(|_| file.read_exact(&mut previous))
                .map_err(io_error)?;
            let previous = decode_event(&previous)?;
            if previous.identity != evidence.identity || previous.attempt >= evidence.attempt {
                return Err("a newer same-identity attempt already started; this superseded attempt will not compute or replace it".to_owned());
            }
        }
        let raw = event_bytes(evidence)?;
        file.seek(SeekFrom::End(0))
            .and_then(|_| file.write_all(&raw))
            .and_then(|()| file.sync_all())
            .map_err(io_error)
    })();
    let released = file.unlock().map_err(io_error);
    result.and(released)
}

fn reserve_start(root: &Path, evidence: Evidence) -> Result<(), String> {
    let path = start_path(root, evidence.attempt);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|why| {
            format!(
                "sweep attempt reservation refused at {}: {why}; existing history was not replaced",
                path.display()
            )
        })?;
    shape::<EVENT_BYTES>(&mut file, &path, EVENTS, true)?;
    file.seek(SeekFrom::End(0))
        .and_then(|_| file.write_all(&event_bytes(evidence).map_err(std::io::Error::other)?))
        .and_then(|()| file.sync_all())
        .map_err(io_error)?;
    Ok(())
}

fn require_start(root: &Path, evidence: Evidence, max_bytes: u64) -> Result<(), String> {
    let Some(start) = last_event(&start_path(root, evidence.attempt), max_bytes)? else {
        return Err("sweep attempt has no durable identity reservation".to_owned());
    };
    if start.identity != evidence.identity
        || start.attempt != evidence.attempt
        || start.operation != evidence.operation
        || start.validation_requested != evidence.validation_requested
        || start.started_micros != evidence.started_micros
        || start.completion != Completion::Running
    {
        return Err("sweep lifecycle differs from its immutable identity reservation".to_owned());
    }
    Ok(())
}

fn base(root: &Path) -> PathBuf {
    root.join("results").join("sweep-evidence-v1")
}
fn directory(root: &Path, id: &[u8; 32]) -> PathBuf {
    base(root).join(hex(id))
}
fn detail_path(root: &Path, e: &Evidence, kind: &str) -> PathBuf {
    directory(root, &e.identity).join(format!("{}-{kind}.bin", e.attempt))
}
fn hex(id: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut text = String::with_capacity(64);
    for byte in id {
        let _ = write!(text, "{byte:02x}");
    }
    text
}
fn io_error(why: impl std::fmt::Display) -> String {
    format!("sweep evidence I/O refused: {why}")
}
fn now() -> Result<i64, String> {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|why| format!("sweep evidence clock refused: {why}"))?
            .as_micros(),
    )
    .map_err(|why| format!("sweep evidence timestamp overflow: {why}"))
}
fn durable_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(io_error)?;
    for directory in path.ancestors() {
        File::open(directory)
            .and_then(|file| file.sync_all())
            .map_err(io_error)?;
    }
    Ok(())
}
fn open_append(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(io_error)
}
fn emit(e: Evidence) {
    crate::note(
        &telemetry::Event::info(
            "cli.lifecycle",
            if e.completion == Completion::Running {
                "sweep evidence attempt started"
            } else {
                "sweep evidence attempt finished"
            },
        )
        .with("identity", hex(&e.identity).as_str())
        .with("attempt", e.attempt)
        .with("operation", e.operation.as_str())
        .with("completion", e.completion.as_str()),
    );
}

fn shape<const N: usize>(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    create: bool,
) -> Result<u64, String> {
    let mut len = file.metadata().map_err(io_error)?.len();
    if len == 0 && create {
        let mut header = [0_u8; 16];
        header[..8].copy_from_slice(&magic);
        header[8..12].copy_from_slice(&1_u32.to_le_bytes());
        header[12..].copy_from_slice(
            &u32::try_from(N)
                .map_err(|why| why.to_string())?
                .to_le_bytes(),
        );
        file.write_all(&header)
            .and_then(|()| file.sync_all())
            .map_err(io_error)?;
        if let Some(parent) = path.parent() {
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(io_error)?;
        }
        len = HEADER;
    }
    let stride = u64::try_from(N).map_err(|why| why.to_string())?;
    if len < HEADER || !(len - HEADER).is_multiple_of(stride) {
        return Err(format!(
            "{} has a torn or short fixed-stride evidence file; no byte was changed",
            path.display()
        ));
    }
    let mut header = [0_u8; 16];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut header))
        .map_err(io_error)?;
    if header[..8] != magic
        || header[8..12] != 1_u32.to_le_bytes()
        || header[12..]
            != u32::try_from(N)
                .map_err(|why| why.to_string())?
                .to_le_bytes()
    {
        return Err(format!(
            "{} has an unknown evidence format/version/stride",
            path.display()
        ));
    }
    Ok((len - HEADER) / stride)
}
fn seal<const N: usize>(raw: &mut [u8; N]) -> Result<(), String> {
    let end = N
        .checked_sub(8)
        .ok_or_else(|| "evidence row is shorter than its seal".to_owned())?;
    let mut h = brutex_core::blake3::Hasher::new();
    h.update(
        raw.get(..end)
            .ok_or_else(|| "evidence payload bounds differ from its stride".to_owned())?,
    );
    raw.get_mut(end..)
        .ok_or_else(|| "evidence seal bounds differ from its stride".to_owned())?
        .copy_from_slice(&h.finalize()[..8]);
    Ok(())
}
fn verify<const N: usize>(raw: &[u8; N]) -> Result<(), String> {
    let mut copy = *raw;
    seal(&mut copy)?;
    if copy == *raw {
        Ok(())
    } else {
        Err("sweep evidence row fails its seal; no partial or corrupted row is exposed".to_owned())
    }
}
fn encode_words<const W: usize, const N: usize>(
    evidence: Evidence,
    words: [u64; W],
) -> Result<[u8; N], String> {
    if W.checked_mul(8).and_then(|bytes| bytes.checked_add(48)) != Some(N) {
        return Err("evidence word count differs from its fixed stride".to_owned());
    }
    let mut raw = [0_u8; N];
    raw.get_mut(..32)
        .ok_or_else(|| "evidence identity is outside its stride".to_owned())?
        .copy_from_slice(&evidence.identity);
    raw.get_mut(32..40)
        .ok_or_else(|| "evidence attempt is outside its stride".to_owned())?
        .copy_from_slice(&evidence.attempt.to_le_bytes());
    for (chunk, word) in raw
        .get_mut(40..)
        .ok_or_else(|| "evidence payload is outside its stride".to_owned())?
        .chunks_exact_mut(8)
        .zip(words)
    {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    seal(&mut raw)?;
    Ok(raw)
}
fn decode_words<const W: usize, const N: usize>(raw: &[u8; N]) -> Result<[u64; W], String> {
    let mut words = [0_u64; W];
    for (word, chunk) in words.iter_mut().zip(
        raw.get(40..)
            .ok_or_else(|| "evidence payload is outside its stride".to_owned())?
            .chunks_exact(8),
    ) {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(chunk);
        *word = u64::from_le_bytes(bytes);
    }
    Ok(words)
}
fn append_row<const N: usize>(path: &Path, magic: [u8; 8], raw: &[u8; N]) -> Result<u64, String> {
    let mut file = open_append(path)?;
    file.lock().map_err(io_error)?;
    let result = (|| {
        let at = shape::<N>(&mut file, path, magic, true)?;
        file.seek(SeekFrom::End(0))
            .and_then(|_| file.write_all(raw))
            .and_then(|()| file.sync_all())
            .map_err(io_error)?;
        Ok(at)
    })();
    let released = file.unlock().map_err(io_error);
    match (result, released) {
        (Ok(at), Ok(())) => Ok(at),
        (Err(why), _) | (_, Err(why)) => Err(why),
    }
}
fn event_bytes(e: Evidence) -> Result<[u8; EVENT_BYTES], String> {
    let mut raw = [0_u8; EVENT_BYTES];
    raw[..8].copy_from_slice(&e.attempt.to_le_bytes());
    raw[8..40].copy_from_slice(&e.identity);
    raw[40..48].copy_from_slice(&e.started_micros.to_le_bytes());
    raw[48..56].copy_from_slice(&e.updated_micros.to_le_bytes());
    raw[56] = e.operation.byte();
    raw[57] = e.completion.byte();
    raw[58..66].copy_from_slice(&e.depth_rows.to_le_bytes());
    raw[66..74].copy_from_slice(&e.ranked_rows.to_le_bytes());
    raw[74] = u8::from(e.ranked_available);
    raw[75] = match e.validation_requested {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    };
    seal(&mut raw)?;
    Ok(raw)
}
fn decode_event(raw: &[u8; EVENT_BYTES]) -> Result<Evidence, String> {
    verify(raw)?;
    if raw[76..EVENT_BYTES - 8].iter().any(|byte| *byte != 0) {
        return Err("sweep lifecycle reserved bytes are nonzero".to_owned());
    }
    let mut identity = [0_u8; 32];
    identity.copy_from_slice(&raw[8..40]);
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&raw[..8]);
    let attempt = u64::from_le_bytes(bytes);
    bytes.copy_from_slice(&raw[40..48]);
    let started_micros = i64::from_le_bytes(bytes);
    bytes.copy_from_slice(&raw[48..56]);
    let updated_micros = i64::from_le_bytes(bytes);
    bytes.copy_from_slice(&raw[58..66]);
    let depth_rows = u64::from_le_bytes(bytes);
    bytes.copy_from_slice(&raw[66..74]);
    let ranked_rows = u64::from_le_bytes(bytes);
    let ranked_available = match raw[74] {
        0 => false,
        1 => true,
        _ => return Err("invalid ranked availability in lifecycle evidence".to_owned()),
    };
    let validation_requested = match raw[75] {
        0 => None,
        1 => Some(false),
        2 => Some(true),
        _ => return Err("invalid validation request in lifecycle evidence".to_owned()),
    };
    Ok(Evidence {
        identity,
        attempt,
        operation: Operation::decode(raw[56])?,
        completion: Completion::decode(raw[57])?,
        started_micros,
        updated_micros,
        depth_rows,
        ranked_rows,
        ranked_available,
        validation_requested,
    })
}
fn append_event(path: &Path, mut e: Evidence, allocate: bool) -> Result<u64, String> {
    let mut file = open_append(path)?;
    file.lock().map_err(io_error)?;
    let result = (|| {
        let index = shape::<EVENT_BYTES>(&mut file, path, EVENTS, true)?;
        if allocate {
            e.attempt = index
                .checked_add(1)
                .ok_or_else(|| "sweep attempt sequence exhausted".to_owned())?;
        }
        let bytes = event_bytes(e)?;
        file.seek(SeekFrom::End(0))
            .and_then(|_| file.write_all(&bytes))
            .and_then(|()| file.sync_all())
            .map_err(io_error)?;
        Ok(e.attempt)
    })();
    let released = file.unlock().map_err(io_error);
    match (result, released) {
        (Ok(token), Ok(())) => Ok(token),
        (Err(why), _) | (_, Err(why)) => Err(why),
    }
}
fn last_event(path: &Path, max_bytes: u64) -> Result<Option<Evidence>, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(io_error(why)),
    };
    file.lock_shared().map_err(io_error)?;
    let result = (|| {
        bound(&file, max_bytes)?;
        let count = shape::<EVENT_BYTES>(&mut file, path, EVENTS, false)?;
        if count == 0 {
            return Ok(None);
        }
        let mut raw = [0_u8; EVENT_BYTES];
        file.seek(SeekFrom::Start(HEADER + (count - 1) * EVENT_BYTES as u64))
            .and_then(|_| file.read_exact(&mut raw))
            .map_err(io_error)?;
        decode_event(&raw).map(Some)
    })();
    let released = file.unlock().map_err(io_error);
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (_, Err(why)) => Err(why),
    }
}
fn bound(file: &File, max_bytes: u64) -> Result<(), String> {
    let len = file.metadata().map_err(io_error)?.len();
    if len > max_bytes {
        return Err(format!(
            "sweep evidence file is {len} bytes, beyond its {max_bytes}-byte read bound"
        ));
    }
    Ok(())
}
fn count_optional<const N: usize>(
    path: &Path,
    magic: [u8; 8],
    max_bytes: u64,
) -> Result<u64, String> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(why) => return Err(io_error(why)),
    };
    file.lock_shared().map_err(io_error)?;
    let result = (|| {
        bound(&file, max_bytes)?;
        shape::<N>(&mut file, path, magic, false)
    })();
    let released = file.unlock().map_err(io_error);
    match (result, released) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(why), _) | (_, Err(why)) => Err(why),
    }
}
fn page<const N: usize>(
    root: &Path,
    e: &Evidence,
    kind: &str,
    magic: [u8; 8],
    offset: u64,
    limit: usize,
    max_bytes: u64,
) -> Result<Vec<[u8; N]>, String> {
    if limit > 4096 {
        return Err("a sweep evidence page is limited to 4096 rows".to_owned());
    }
    require_start(root, *e, max_bytes)?;
    let expected = if kind == "levels" {
        e.depth_rows
    } else {
        e.ranked_rows
    };
    let path = detail_path(root, e, kind);
    let mut file = match File::open(&path) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return if expected > 0 || (kind == "ranked" && e.ranked_available) {
                Err("sweep evidence page is missing a declared child file".to_owned())
            } else {
                Ok(Vec::new())
            };
        }
        Err(why) => return Err(io_error(why)),
    };
    file.lock_shared().map_err(io_error)?;
    let generation = crate::result_set::file_generation(&file, &path)?;
    let result = (|| {
        bound(&file, max_bytes)?;
        let count = shape::<N>(&mut file, &path, magic, false)?;
        if count != expected {
            return Err(
                "sweep evidence page cardinality changed after its lifecycle snapshot".to_owned(),
            );
        }
        let take = count
            .saturating_sub(offset)
            .min(u64::try_from(limit).unwrap_or(u64::MAX));
        let mut rows = Vec::new();
        rows.try_reserve_exact(usize::try_from(take).map_err(|why| why.to_string())?)
            .map_err(|why| why.to_string())?;
        if take == 0 {
            return Ok(rows);
        }
        file.seek(SeekFrom::Start(HEADER + offset * N as u64))
            .map_err(io_error)?;
        for _ in 0..take {
            let mut row = [0_u8; N];
            file.read_exact(&mut row).map_err(io_error)?;
            verify(&row)?;
            if row.get(..32) != Some(e.identity.as_slice())
                || row.get(32..40) != Some(e.attempt.to_le_bytes().as_slice())
            {
                return Err(
                    "sweep detail row belongs to a different identity or attempt".to_owned(),
                );
            }
            rows.push(row);
        }
        Ok(rows)
    })();
    let result = result.and_then(|rows| {
        let observed = crate::result_set::file_generation(&file, &path)?;
        crate::result_set::require_generation_unchanged(generation, observed, &path)?;
        Ok(rows)
    });
    let released = file.unlock().map_err(io_error);
    match (result, released) {
        (Ok(rows), Ok(())) => Ok(rows),
        (Err(why), _) | (_, Err(why)) => Err(why),
    }
}
