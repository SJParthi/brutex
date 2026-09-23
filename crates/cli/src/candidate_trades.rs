//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! Exact selected-cell trades for every evaluated screen candidate and side.
//!
//! Each visited policy tier retains its actual cap and inputs. Immutable child
//! files are sealed before a catalog can publish the captured set. The catalog
//! is prepared evidence, not an institutional admission or parent-run success.
//! Pages are bounded; cold verification and all retained history are not O(1).

mod codec;

use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use costs::fill::Direction;
use indicators::column::Column;
use runner::grid;
use runner::outcome::Horizon;
use vocab::ConditionMask;
use vocab::expression::{ENCODED_LEN, Expression};

use codec::{Decoder, Encoder};

const START: [u8; 8] = *b"BRPCST01";
const TIER: [u8; 8] = *b"BRPCTR01";
const CANDIDATE: [u8; 8] = *b"BRPCCA01";
const TRADES: [u8; 8] = *b"BRPCTX01";
const CATALOG: [u8; 8] = *b"BRPCCT01";
const EXPRESSION_START: [u8; 8] = *b"BRPEST01";
const EXPRESSION_CATALOG: [u8; 8] = *b"BRPECT01";
const HEADER: usize = 16;
const SEAL: usize = 32;
const TRADE_BYTES: usize = 136;
/// Maximum rows returned by either page surface.
pub const MAX_PAGE: usize = 256;
/// Default cold-file admission. Larger evidence needs an explicitly larger bound.
pub const DEFAULT_MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Explicit predicate authority; readers never fall back between namespaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Model {
    /// Existing conjunction mask semantics and unchanged version-one bytes.
    And,
    /// Complete versioned expression; referenced bits alone are insufficient.
    Expression,
}
impl Model {
    /// Stable API label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::And => "and-mask",
            Self::Expression => "expression",
        }
    }
    const fn start(self) -> [u8; 8] {
        match self {
            Self::And => START,
            Self::Expression => EXPRESSION_START,
        }
    }
    const fn catalog(self) -> [u8; 8] {
        match self {
            Self::And => CATALOG,
            Self::Expression => EXPRESSION_CATALOG,
        }
    }
}

fn model_of(expression: Option<&Expression>) -> Model {
    if expression.is_some() {
        Model::Expression
    } else {
        Model::And
    }
}

/// Deterministic address within an attempt, independent of worker completion order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    /// Zero-based invocation of the policy screen, in cascade order.
    pub tier: u64,
    /// One-based rank in the retained signal evidence supplied to that screen.
    pub rank: u64,
    /// Exact grid direction; both directions are captured separately.
    pub direction: Direction,
}
impl Key {
    fn slot(self) -> Result<usize, String> {
        let offset = self
            .rank
            .checked_sub(1)
            .and_then(|rank| rank.checked_mul(2))
            .and_then(|rank| rank.checked_add(codec::direction(self.direction)))
            .ok_or("candidate key overflow or zero rank")?;
        usize::try_from(offset).map_err(|_| "candidate key exceeds address space".to_owned())
    }
}

/// Exact inputs to one screen pass. This is a pricing subset, not the sweep universe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tier {
    /// Zero-based invocation ordinal, assigned by the capture.
    pub index: u64,
    /// Retained signal candidates offered to this pass.
    pub eligible: u64,
    /// Candidates actually evaluated, before the two directions are expanded.
    pub evaluated: u64,
    /// Hold in execution bars.
    pub horizon: u64,
    /// Requested grid rung count.
    pub rungs: u64,
    /// Explicit grid step, if supplied.
    pub step_ppm: Option<i64>,
    /// Actual forced stop, if supplied.
    pub forced_ppm: Option<i64>,
    /// Whether the grid included ratio targets.
    pub ratios: bool,
    /// Complete policy used by this screen, including its visible page size.
    pub rules: crate::Rules,
    /// Explicit stop ladder supplied to the grid.
    pub stops_ppm: Vec<i64>,
}

/// One grid-side outcome, including a genuine no-cell result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// Full nine-term parent run identity.
    pub identity: [u8; 32],
    /// Durable sweep attempt, never a frontier rank.
    pub attempt: u64,
    /// Exact tier, evidence rank and direction.
    pub key: Key,
    /// Exact evaluated condition mask.
    pub mask_words: [u64; 6],
    /// Digest of the execution bars the materializer actually read.
    pub execution_digest: [u8; 32],
    /// Digest of the complete tier inputs.
    pub tier_digest: [u8; 32],
    /// Whether the selected cell passed the cell rules; no calendar inference.
    pub admitted: bool,
    /// The policy-selected cell. `None` explicitly means no cell was returned.
    pub cell: Option<grid::Cell>,
    /// Grid signal count before exclusivity.
    pub signals: u64,
    /// Grid paths refused by its existing input checks.
    pub refused_paths: u64,
    /// Actual stop ladder indexed by the selected cell.
    pub stops: Vec<i64>,
    /// Actual target ladder indexed by the selected cell.
    pub targets: Vec<i64>,
    /// Actual trailing ladder indexed by the selected cell.
    pub trails: Vec<i64>,
    /// Domain-separated identity repeated in each exact trade row.
    pub trade_identity: [u8; 32],
    /// Full digest of the exact trade file, including its header.
    pub trades_digest: [u8; 32],
}

/// The exact pricing authority while its grid is still in scope.
pub struct Evaluated<'a> {
    /// One-based retained signal evidence rank.
    pub rank: u64,
    /// Actual evaluated mask.
    pub mask: &'a ConditionMask,
    /// Actual evaluated side.
    pub direction: Direction,
    /// Grid whose cell and ladders will be replayed without a second selection.
    pub grid: &'a grid::Grid,
    /// Exact result of the existing policy selector, including its cell verdict.
    pub selected: Option<(grid::Cell, bool)>,
}

struct TierState {
    tier: Tier,
    digest: [u8; 32],
    acknowledged: Vec<Option<[u8; 32]>>,
}
#[derive(Default)]
struct State {
    tiers: Vec<TierState>,
    retained_bytes: usize,
    failed: Option<String>,
    finished: bool,
}

/// A capture bound to one already-durable attempt and one execution slice/column.
/// Candidate replay can run in parallel; the short publication section is serialized.
pub struct Capture<'a> {
    directory: PathBuf,
    identity: [u8; 32],
    attempt: u64,
    execution_digest: [u8; 32],
    bars: &'a [indicators::Candle],
    column: &'a Column,
    expression: Option<&'a Expression>,
    state: Mutex<State>,
}

impl<'a> Capture<'a> {
    /// Reserves the immutable capture start before pricing any candidate.
    ///
    /// # Errors
    /// Refuses an unreadable destination or a conflicting existing reservation.
    pub fn begin(
        root: &Path,
        attempt: &crate::sweep_evidence::Attempt,
        bars: &'a [indicators::Candle],
        column: &'a Column,
    ) -> Result<Self, String> {
        Self::begin_with(root, attempt, bars, column, None)
    }

    /// Records a complete expression before its grids run, in a separate namespace.
    ///
    /// # Errors
    /// Refuses conflicting descriptor bytes or an unavailable destination.
    pub fn begin_expression(
        root: &Path,
        attempt: &crate::sweep_evidence::Attempt,
        bars: &'a [indicators::Candle],
        column: &'a Column,
        expression: &'a Expression,
    ) -> Result<Self, String> {
        Self::begin_with(root, attempt, bars, column, Some(expression))
    }

    fn begin_with(
        root: &Path,
        attempt: &crate::sweep_evidence::Attempt,
        bars: &'a [indicators::Candle],
        column: &'a Column,
        expression: Option<&'a Expression>,
    ) -> Result<Self, String> {
        let model = model_of(expression);
        let directory = directory_for(root, &attempt.identity(), attempt.token(), model);
        fs::create_dir_all(&directory).map_err(io_error)?;
        let execution_digest = runner::identity::data_digest(bars);
        let mut start = Encoder::default();
        start.bytes(&attempt.identity());
        start.word(attempt.token());
        start.bytes(&execution_digest);
        if let Some(expression) = expression {
            start.bytes(&expression.encode());
        }
        write_exact(&directory.join("start.bin"), model.start(), &start.0)?;
        Ok(Self {
            directory,
            identity: attempt.identity(),
            attempt: attempt.token(),
            execution_digest,
            bars,
            column,
            expression,
            state: Mutex::new(State::default()),
        })
    }

    /// Records a tier's complete inputs and exact evaluated cap before its grids run.
    ///
    /// # Errors
    /// Refuses invalid bounds, allocation failure, or any previous persistence failure.
    pub fn tier(&self, mut tier: Tier) -> Result<Tier, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "candidate capture mutex poisoned")?;
        Self::healthy(&state)?;
        let result = (|| {
            if tier.evaluated > tier.eligible || tier.horizon == 0 {
                return Err("candidate tier has invalid bounds".to_owned());
            }
            tier.index = state.tiers.len() as u64;
            let count = usize::try_from(
                tier.evaluated
                    .checked_mul(2)
                    .ok_or("candidate count overflow")?,
            )
            .map_err(|_| "candidate count exceeds address space")?;
            let bytes = count
                .checked_mul(std::mem::size_of::<Option<[u8; 32]>>())
                .and_then(|bytes| bytes.checked_add(std::mem::size_of::<TierState>()))
                .and_then(|bytes| {
                    tier.stops_ppm
                        .len()
                        .checked_mul(8)
                        .and_then(|stops| bytes.checked_add(stops))
                })
                .and_then(|bytes| state.retained_bytes.checked_add(bytes))
                .ok_or("candidate acknowledgement allocation overflow")?;
            if bytes as u64 > DEFAULT_MAX_BYTES {
                return Err("candidate capture acknowledgement budget exceeds 64 MiB; this pass was not priced".to_owned());
            }
            let mut acknowledged = Vec::new();
            acknowledged.try_reserve_exact(count).map_err(io_error)?;
            acknowledged.resize(count, None);
            state.tiers.try_reserve(1).map_err(io_error)?;
            let digest = write_exact(
                &tier_path(&self.directory, tier.index),
                TIER,
                &codec::tier(&tier),
            )?;
            state.tiers.push(TierState {
                tier: tier.clone(),
                digest,
                acknowledged,
            });
            state.retained_bytes = bytes;
            Ok(tier)
        })();
        if let Err(why) = &result {
            state.failed = Some(why.clone());
        }
        result
    }

    fn healthy(state: &State) -> Result<(), String> {
        if let Some(why) = &state.failed {
            return Err(why.clone());
        }
        if state.finished {
            return Err("candidate capture is already sealed".to_owned());
        }
        Ok(())
    }

    /// Checks the capture's latched persistence state.
    ///
    /// # Errors
    /// Returns the first persistence failure, or refuses an already sealed capture.
    pub fn check(&self) -> Result<(), String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "candidate capture mutex poisoned")?;
        Self::healthy(&state)
    }

    /// Remembers a failed callback; a later `finish` cannot publish success.
    pub fn refuse(&self, why: String) {
        if let Ok(mut state) = self.state.lock() {
            state.failed.get_or_insert(why);
        }
    }

    /// Replays this exact policy-selected cell with the shared engine and saves it.
    /// A no-cell outcome gets an explicit manifest and an empty trade file.
    ///
    /// # Errors
    /// Any replay mismatch or durable write failure poisons the complete capture.
    pub fn record(&self, tier: &Tier, evaluated: &Evaluated<'_>) -> Result<(), String> {
        let result = self.record_inner(tier, evaluated);
        if let Err(why) = &result {
            self.refuse(why.clone());
        }
        result
    }

    fn record_inner(&self, tier: &Tier, evaluated: &Evaluated<'_>) -> Result<(), String> {
        if self
            .expression
            .is_some_and(|expression| expression.referenced() != *evaluated.mask)
        {
            return Err("expression candidate mask does not match its referenced bits".to_owned());
        }
        let key = Key {
            tier: tier.index,
            rank: evaluated.rank,
            direction: evaluated.direction,
        };
        let slot = key.slot()?;
        if evaluated.selected != crate::shown_cell(evaluated.grid, tier.rules) {
            return Err(
                "candidate selection differs from the recorded policy's actual grid selection"
                    .to_owned(),
            );
        }
        let tier_digest = {
            let state = self
                .state
                .lock()
                .map_err(|_| "candidate capture mutex poisoned")?;
            Self::healthy(&state)?;
            let held = state
                .tiers
                .get(usize::try_from(tier.index).map_err(io_error)?)
                .ok_or("candidate tier is absent")?;
            if held.tier != *tier || slot >= held.acknowledged.len() {
                return Err("candidate differs from its recorded tier bounds".to_owned());
            }
            held.digest
        };
        let rows = match evaluated.selected {
            Some((cell, _)) => {
                if !evaluated.grid.cells.contains(&cell) {
                    return Err("candidate cell is absent from its actual grid".to_owned());
                }
                self.materialize(tier, evaluated, &cell)?
            }
            None => Vec::new(),
        };
        let mut candidate = Candidate {
            identity: self.identity,
            attempt: self.attempt,
            key,
            mask_words: evaluated.mask.words(),
            execution_digest: self.execution_digest,
            tier_digest,
            admitted: evaluated.selected.is_some_and(|(_, admitted)| admitted),
            cell: evaluated.selected.map(|(cell, _)| cell),
            signals: evaluated.grid.signals,
            refused_paths: evaluated.grid.refused_paths,
            stops: evaluated.grid.stops.rungs().to_vec(),
            targets: evaluated.grid.targets.rungs().to_vec(),
            trails: evaluated.grid.trails.rungs().to_vec(),
            trade_identity: [0; 32],
            trades_digest: [0; 32],
        };
        candidate.trade_identity = candidate_identity(&candidate, self.expression);
        let trades = encoded_trades(&candidate, &rows)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "candidate capture mutex poisoned")?;
        Self::healthy(&state)?;
        candidate.trades_digest = write_exact(&trade_path(&self.directory, key), TRADES, &trades)?;
        let digest = write_exact(
            &candidate_path(&self.directory, key),
            CANDIDATE,
            &codec::candidate(&candidate),
        )?;
        let prior = state
            .tiers
            .get_mut(usize::try_from(tier.index).map_err(io_error)?)
            .and_then(|held| held.acknowledged.get_mut(slot))
            .ok_or("candidate acknowledgement slot disappeared")?;
        if prior.is_some_and(|old| old != digest) {
            return Err("candidate acknowledgement changed on repeated publication".to_owned());
        }
        *prior = Some(digest);
        Ok(())
    }

    fn materialize(
        &self,
        tier: &Tier,
        evaluated: &Evaluated<'_>,
        cell: &grid::Cell,
    ) -> Result<Vec<grid::TradeRow>, String> {
        let horizon = Horizon::bars(u32::try_from(tier.horizon).map_err(io_error)?)
            .ok_or("candidate horizon is zero")?;
        let side = crate::side_of_direction(evaluated.direction);
        match self.expression {
            Some(expression) => grid::materialize_expression_cell(
                self.bars,
                self.column,
                expression,
                horizon,
                side,
                evaluated.grid,
                cell,
            ),
            None => grid::materialize_cell(
                self.bars,
                self.column,
                evaluated.mask,
                horizon,
                side,
                evaluated.grid,
                cell,
            ),
        }
    }

    /// Verifies every acknowledged child, then seals the catalog before a parent commits.
    ///
    /// # Errors
    /// Missing, changed, unacknowledged or corrupt child evidence prevents publication.
    pub fn finish(&self) -> Result<Summary, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "candidate capture mutex poisoned")?;
        Self::healthy(&state)?;
        let result = self.finish_inner(&state, true);
        match &result {
            Ok(_) => state.finished = true,
            Err(why) => state.failed = Some(why.clone()),
        }
        result
    }

    /// Revalidates sealed evidence immediately before publishing the legacy parent.
    ///
    /// # Errors
    /// Refuses a missing seal or any child changed during later validation work.
    pub fn confirm(&self) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "candidate capture mutex poisoned")?;
        if let Some(why) = &state.failed {
            return Err(why.clone());
        }
        if !state.finished {
            return Err("candidate capture has not sealed its catalog".to_owned());
        }
        let result = self.finish_inner(&state, false).map(|_| ());
        if let Err(why) = &result {
            state.failed = Some(why.clone());
        }
        result
    }

    fn finish_inner(&self, state: &State, publish: bool) -> Result<Summary, String> {
        require_start(
            &self.directory,
            self.identity,
            self.attempt,
            self.execution_digest,
            self.expression,
        )?;
        let mut catalog = Encoder::default();
        catalog.bytes(&self.identity);
        catalog.word(self.attempt);
        catalog.bytes(&self.execution_digest);
        catalog.word(state.tiers.len() as u64);
        for held in &state.tiers {
            let (raw, digest) = read_sealed(
                &tier_path(&self.directory, held.tier.index),
                TIER,
                DEFAULT_MAX_BYTES,
            )?;
            if digest != held.digest || codec::decode_tier(&raw)? != held.tier {
                return Err("acknowledged candidate tier changed before completion".to_owned());
            }
            catalog.bytes(&digest);
            catalog.word(held.acknowledged.len() as u64);
            for (slot, acknowledged) in held.acknowledged.iter().enumerate() {
                let expected =
                    acknowledged.ok_or("an evaluated candidate has no acknowledged detail")?;
                let key = key_at(held.tier.index, slot);
                let (raw, actual) = read_sealed(
                    &candidate_path(&self.directory, key),
                    CANDIDATE,
                    DEFAULT_MAX_BYTES,
                )?;
                let candidate = codec::decode_candidate(&raw)?;
                if actual != expected {
                    return Err(
                        "acknowledged candidate manifest changed before completion".to_owned()
                    );
                }
                validate_candidate(
                    &candidate,
                    self.identity,
                    self.attempt,
                    self.execution_digest,
                    key,
                    digest,
                    self.expression,
                )?;
                let _verified =
                    TradeReader::open_candidate(&self.directory, &candidate, DEFAULT_MAX_BYTES)?;
                catalog.bytes(&expected);
            }
        }
        if let Some(expression) = self.expression {
            catalog.bytes(&expression.encode());
        }
        let model = model_of(self.expression);
        let path = self.directory.join("catalog.bin");
        let digest = if publish {
            write_exact(&path, model.catalog(), &catalog.0)?
        } else {
            let (saved, digest) = read_sealed(&path, model.catalog(), DEFAULT_MAX_BYTES)?;
            if saved != catalog.0 {
                return Err("sealed candidate catalog changed before parent publication".to_owned());
            }
            digest
        };
        decode_summary(&catalog.0, digest, model)
    }
}

#[derive(Clone, Debug)]
struct TierIndex {
    digest: [u8; 32],
    candidates: Vec<[u8; 32]>,
}
/// A verified immutable catalog snapshot. Carry this exact object across pages.
#[derive(Clone, Debug)]
pub struct Summary {
    /// Predicate namespace validated by this catalog, never guessed from a mask.
    pub model: Model,
    /// Exact parent identity.
    pub identity: [u8; 32],
    /// Durable attempt token.
    pub attempt: u64,
    /// Digest of the actual execution slice.
    pub execution_digest: [u8; 32],
    /// Number of visited screen invocations, not generated policy tiers.
    pub tiers: u64,
    /// Captured candidate-side outcomes, including no-cell outcomes.
    pub candidates: u64,
    /// Full immutable catalog digest, required to pin API pages.
    pub digest: [u8; 32],
    index: Vec<TierIndex>,
    expression: Option<Box<Expression>>,
}

impl Summary {
    /// Complete saved predicate for expression captures; absent for AND masks.
    #[must_use]
    pub fn expression(&self) -> Option<&Expression> {
        self.expression.as_deref()
    }
}

/// Reads a sealed capture; absent catalog means incomplete, never completed-empty.
///
/// # Errors
/// Refuses damaged, foreign or oversized catalogs. This does not assert parent success.
pub fn read(
    root: &Path,
    identity: [u8; 32],
    attempt: u64,
    max_bytes: u64,
) -> Result<Option<Summary>, String> {
    read_model(root, identity, attempt, Model::And, max_bytes)
}

/// Reads only the selected predicate namespace, with no automatic replacement.
///
/// # Errors
/// Refuses a damaged catalog, changed descriptor, wrong identity or oversized file.
pub fn read_model(
    root: &Path,
    identity: [u8; 32],
    attempt: u64,
    model: Model,
    max_bytes: u64,
) -> Result<Option<Summary>, String> {
    let dir = directory_for(root, &identity, attempt, model);
    let path = dir.join("catalog.bin");
    if !path.try_exists().map_err(io_error)? {
        return Ok(None);
    }
    let (payload, digest) = read_sealed(&path, model.catalog(), max_bytes)?;
    let summary = decode_summary(&payload, digest, model)?;
    if summary.identity != identity || summary.attempt != attempt {
        return Err("candidate catalog names another run or attempt".to_owned());
    }
    require_start(
        &dir,
        identity,
        attempt,
        summary.execution_digest,
        summary.expression(),
    )?;
    Ok(Some(summary))
}

fn require_start(
    dir: &Path,
    identity: [u8; 32],
    attempt: u64,
    execution_digest: [u8; 32],
    expression: Option<&Expression>,
) -> Result<(), String> {
    let (raw, _) = read_sealed(
        &dir.join("start.bin"),
        model_of(expression).start(),
        (120 + ENCODED_LEN) as u64,
    )?;
    let mut expected = Encoder::default();
    expected.bytes(&identity);
    expected.word(attempt);
    expected.bytes(&execution_digest);
    if let Some(expression) = expression {
        expected.bytes(&expression.encode());
    }
    if raw != expected.0 {
        return Err("candidate capture start differs from its catalog".to_owned());
    }
    Ok(())
}

fn decode_summary(raw: &[u8], digest: [u8; 32], model: Model) -> Result<Summary, String> {
    let mut input = Decoder::new(raw);
    let identity = input.bytes()?;
    let attempt = input.word()?;
    let execution_digest = input.bytes()?;
    let tiers = input.word()?;
    if tiers > raw.len() as u64 / 40 {
        return Err("candidate catalog tier count exceeds its bytes".to_owned());
    }
    let mut index = Vec::new();
    let mut candidates = 0_u64;
    for _ in 0..tiers {
        let digest = input.bytes()?;
        let count = input.size()?;
        if count % 2 != 0 || count > raw.len() / 32 {
            return Err("candidate catalog has invalid side count".to_owned());
        }
        let values: Result<Vec<_>, _> = (0..count).map(|_| input.bytes()).collect();
        candidates = candidates
            .checked_add(count as u64)
            .ok_or("candidate total overflow")?;
        index.push(TierIndex {
            digest,
            candidates: values?,
        });
    }
    let expression = match model {
        Model::And => None,
        Model::Expression => Some(Box::new(
            Expression::decode(&input.bytes()?)
                .map_err(|why| format!("candidate expression descriptor refused: {why:?}"))?,
        )),
    };
    input.done()?;
    Ok(Summary {
        model,
        identity,
        attempt,
        execution_digest,
        tiers,
        candidates,
        digest,
        index,
        expression,
    })
}

fn pinned(root: &Path, summary: &Summary, max_bytes: u64) -> Result<PathBuf, String> {
    let held = read_model(
        root,
        summary.identity,
        summary.attempt,
        summary.model,
        max_bytes,
    )?
    .ok_or("candidate catalog disappeared")?;
    if held.digest != summary.digest {
        return Err("candidate catalog changed between pages".to_owned());
    }
    Ok(directory_for(
        root,
        &summary.identity,
        summary.attempt,
        summary.model,
    ))
}

/// Reads the actual policy and pricing cap for a visited tier.
///
/// # Errors
/// Refuses an absent tier, changed catalog, or invalid tier bytes.
pub fn tier(root: &Path, summary: &Summary, index: u64, max_bytes: u64) -> Result<Tier, String> {
    let dir = pinned(root, summary, max_bytes)?;
    read_tier(&dir, summary, index, max_bytes)
}
fn read_tier(dir: &Path, summary: &Summary, index: u64, max_bytes: u64) -> Result<Tier, String> {
    let held = summary
        .index
        .get(usize::try_from(index).map_err(io_error)?)
        .ok_or("candidate tier index out of bounds")?;
    let (raw, digest) = read_sealed(&tier_path(dir, index), TIER, max_bytes)?;
    let tier = codec::decode_tier(&raw)?;
    if digest != held.digest
        || tier.index != index
        || tier.evaluated.checked_mul(2) != Some(held.candidates.len() as u64)
    {
        return Err("candidate tier differs from its sealed catalog".to_owned());
    }
    Ok(tier)
}

/// Reads at most 256 consecutive candidate-side manifests within one tier.
/// Start is a zero-based side ordinal: rank one long, rank one short, then rank two.
///
/// # Errors
/// Refuses damaged manifests, wrong bounds or a changed catalog. Trade bodies are
/// verified separately by `TradeReader`; this metadata page does not imply that check.
pub fn candidates_page(
    root: &Path,
    summary: &Summary,
    tier: u64,
    start: u64,
    limit: usize,
    max_bytes: u64,
) -> Result<Vec<Candidate>, String> {
    page_limit(limit)?;
    let dir = pinned(root, summary, max_bytes)?;
    let held = summary
        .index
        .get(usize::try_from(tier).map_err(io_error)?)
        .ok_or("candidate tier index out of bounds")?;
    let _tier = read_tier(&dir, summary, tier, max_bytes)?;
    let start = usize::try_from(start).map_err(io_error)?;
    if start > held.candidates.len() {
        return Err("candidate page starts beyond the recorded tier".to_owned());
    }
    let count = limit.min(held.candidates.len().saturating_sub(start));
    let mut remaining = max_bytes;
    let mut out = Vec::with_capacity(count);
    for slot in start..start.saturating_add(count) {
        let key = key_at(tier, slot);
        let path = candidate_path(&dir, key);
        let bytes = fs::metadata(&path).map_err(io_error)?.len();
        let candidate = read_candidate(&dir, summary, key, remaining)?;
        remaining = remaining
            .checked_sub(bytes)
            .ok_or("candidate page byte budget exhausted")?;
        out.push(candidate);
    }
    let _dir = pinned(root, summary, max_bytes)?;
    Ok(out)
}

fn read_candidate(
    dir: &Path,
    summary: &Summary,
    key: Key,
    max_bytes: u64,
) -> Result<Candidate, String> {
    let held = summary
        .index
        .get(usize::try_from(key.tier).map_err(io_error)?)
        .ok_or("candidate tier out of bounds")?;
    let expected = held
        .candidates
        .get(key.slot()?)
        .ok_or("candidate rank out of bounds")?;
    let (raw, digest) = read_sealed(&candidate_path(dir, key), CANDIDATE, max_bytes)?;
    if digest != *expected {
        return Err("candidate manifest differs from its sealed catalog".to_owned());
    }
    let candidate = codec::decode_candidate(&raw)?;
    validate_candidate(
        &candidate,
        summary.identity,
        summary.attempt,
        summary.execution_digest,
        key,
        held.digest,
        summary.expression(),
    )?;
    Ok(candidate)
}

fn validate_candidate(
    candidate: &Candidate,
    identity: [u8; 32],
    attempt: u64,
    execution: [u8; 32],
    key: Key,
    tier_digest: [u8; 32],
    expression: Option<&Expression>,
) -> Result<(), String> {
    if candidate.identity != identity
        || candidate.attempt != attempt
        || candidate.key != key
        || candidate.execution_digest != execution
        || candidate.tier_digest != tier_digest
        || expression
            .is_some_and(|expression| expression.referenced().words() != candidate.mask_words)
        || candidate.trade_identity != candidate_identity(candidate, expression)
    {
        return Err("candidate manifest has foreign identity, context or key".to_owned());
    }
    Ok(())
}

fn candidate_identity(candidate: &Candidate, expression: Option<&Expression>) -> [u8; 32] {
    let mut value = candidate.clone();
    value.trade_identity = [0; 32];
    value.trades_digest = [0; 32];
    let mut hash = brutex_core::blake3::Hasher::new();
    if let Some(expression) = expression {
        hash.update(b"brutex-priced-expression-candidate-v1\0");
        hash.update(&expression.encode());
    } else {
        hash.update(b"brutex-priced-candidate-v1\0");
    }
    hash.update(&codec::candidate(&value));
    hash.finalize()
}

/// One validated trade-file handle; cold open checks every row, warm pages read
/// only requested rows plus filesystem-generation checks. No history index is built.
pub struct TradeReader {
    file: File,
    path: PathBuf,
    generation: crate::result_set::FileGeneration,
    candidate: Candidate,
}
impl TradeReader {
    /// Opens this exact candidate's trades against a pinned catalog.
    ///
    /// # Errors
    /// Refuses corruption, replacement, foreign rows, count or financial mismatch.
    pub fn open(root: &Path, summary: &Summary, key: Key, max_bytes: u64) -> Result<Self, String> {
        let dir = pinned(root, summary, max_bytes)?;
        let _tier = read_tier(&dir, summary, key.tier, max_bytes)?;
        let candidate = read_candidate(&dir, summary, key, max_bytes)?;
        let reader = Self::open_candidate(&dir, &candidate, max_bytes)?;
        let _dir = pinned(root, summary, max_bytes)?;
        Ok(reader)
    }
    fn open_candidate(dir: &Path, candidate: &Candidate, max_bytes: u64) -> Result<Self, String> {
        let path = trade_path(dir, candidate.key);
        let mut file = crate::readonly_file::open(&path).map_err(io_error)?;
        lock_for_read(&file)?;
        let generation = crate::result_set::file_generation(&file, &path)?;
        let len = file.metadata().map_err(io_error)?.len();
        let count = candidate.cell.map_or(0, |cell| cell.trades);
        let expected = count
            .checked_mul(TRADE_BYTES as u64)
            .and_then(|n| n.checked_add((HEADER + SEAL) as u64))
            .ok_or("candidate trade extent overflow")?;
        if len != expected || len > max_bytes {
            return Err(
                "candidate trade extent differs from its cell or byte admission".to_owned(),
            );
        }
        let mut header = [0; HEADER];
        file.read_exact(&mut header).map_err(io_error)?;
        verify_header(&header, TRADES)?;
        let mut digest = brutex_core::blake3::Hasher::new();
        digest.update(&header);
        let mut financial = Financial::default();
        for seq in 0..count {
            let mut raw = [0; TRADE_BYTES];
            file.read_exact(&mut raw).map_err(io_error)?;
            digest.update(&raw);
            financial.add(checked_trade(&raw, candidate, seq)?);
        }
        let mut seal = [0; SEAL];
        file.read_exact(&mut seal).map_err(io_error)?;
        if digest.finalize() != seal || seal != candidate.trades_digest {
            return Err("candidate trade content digest differs from its manifest".to_owned());
        }
        financial.matches(candidate.cell.as_ref())?;
        let observed = crate::result_set::file_generation(&file, &path)?;
        crate::result_set::require_generation_unchanged(generation, observed, &path)?;
        file.unlock().map_err(io_error)?;
        Ok(Self {
            file,
            path,
            generation,
            candidate: candidate.clone(),
        })
    }
    /// Exact manifest validated by this reader.
    #[must_use]
    pub const fn candidate(&self) -> &Candidate {
        &self.candidate
    }
    /// Returns an exact bounded trade page, refusing any generation change.
    ///
    /// # Errors
    /// Refuses invalid pages, changed files and row corruption.
    pub fn page(&mut self, start: u64, limit: usize) -> Result<Vec<crate::trades::Row>, String> {
        page_limit(limit)?;
        lock_for_read(&self.file)?;
        let result = self.page_locked(start, limit);
        self.file.unlock().map_err(io_error)?;
        result
    }
    fn page_locked(&mut self, start: u64, limit: usize) -> Result<Vec<crate::trades::Row>, String> {
        let generation = crate::result_set::file_generation(&self.file, &self.path)?;
        crate::result_set::require_generation_unchanged(self.generation, generation, &self.path)?;
        let total = self.candidate.cell.map_or(0, |cell| cell.trades);
        if start > total {
            return Err("candidate trade page starts beyond its cell".to_owned());
        }
        let count = (total - start).min(limit as u64);
        let offset = start
            .checked_mul(TRADE_BYTES as u64)
            .and_then(|n| n.checked_add(HEADER as u64))
            .ok_or("candidate trade offset overflow")?;
        self.file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
        let mut out = Vec::with_capacity(usize::try_from(count).map_err(io_error)?);
        for seq in start..start.saturating_add(count) {
            let mut raw = [0; TRADE_BYTES];
            self.file.read_exact(&mut raw).map_err(io_error)?;
            out.push(checked_trade(&raw, &self.candidate, seq)?);
        }
        let observed = crate::result_set::file_generation(&self.file, &self.path)?;
        crate::result_set::require_generation_unchanged(generation, observed, &self.path)?;
        Ok(out)
    }
}

#[derive(Default)]
struct Financial {
    count: u64,
    pessimistic: i64,
    optimistic: i64,
    worst: i64,
    peak: i64,
    drawdown: i64,
}
impl Financial {
    fn add(&mut self, row: crate::trades::Row) {
        self.count += 1;
        self.pessimistic = self.pessimistic.saturating_add(row.worst);
        self.optimistic = self.optimistic.saturating_add(row.best);
        self.worst = self.worst.min(row.worst);
        self.peak = self.peak.max(self.pessimistic);
        self.drawdown = self
            .drawdown
            .max(self.peak.saturating_sub(self.pessimistic));
    }
    fn matches(&self, cell: Option<&grid::Cell>) -> Result<(), String> {
        let absent = grid::Cell::default();
        let cell = cell.unwrap_or(&absent);
        if (
            self.count,
            self.pessimistic,
            self.optimistic,
            self.worst,
            self.drawdown,
        ) != (
            cell.trades,
            cell.pessimistic,
            cell.optimistic,
            cell.worst_trade,
            cell.max_drawdown,
        ) {
            return Err("candidate trades do not reconcile with their selected cell".to_owned());
        }
        Ok(())
    }
}
fn checked_trade(
    raw: &[u8; TRADE_BYTES],
    candidate: &Candidate,
    seq: u64,
) -> Result<crate::trades::Row, String> {
    if !crate::trades::Row::seal_matches(raw) {
        return Err("candidate trade row seal mismatch".to_owned());
    }
    let row = crate::trades::Row::from_bytes(raw)?;
    if row.identity != candidate.trade_identity
        || u64::from(row.seq) != seq
        || row.direction != candidate.key.direction
        || row.entry_bar > row.exit_bar
        || row.entry_micros > row.exit_micros
        || row.worst > row.best
    {
        return Err("candidate trade row has foreign identity, sequence or bounds".to_owned());
    }
    Ok(row)
}
fn encoded_trades(candidate: &Candidate, rows: &[grid::TradeRow]) -> Result<Vec<u8>, String> {
    let extent = rows
        .len()
        .checked_mul(TRADE_BYTES)
        .and_then(|bytes| bytes.checked_add(HEADER + SEAL))
        .ok_or("candidate trade extent overflow")?;
    if extent as u64 > DEFAULT_MAX_BYTES {
        return Err("candidate trade file exceeds the 64 MiB capture admission".to_owned());
    }
    let mut out = Vec::new();
    out.try_reserve_exact(
        rows.len()
            .checked_mul(TRADE_BYTES)
            .ok_or("candidate trade allocation overflow")?,
    )
    .map_err(io_error)?;
    for (seq, row) in rows.iter().enumerate() {
        out.extend_from_slice(
            &crate::trades::Row {
                identity: candidate.trade_identity,
                seq: u32::try_from(seq).map_err(io_error)?,
                direction: candidate.key.direction,
                signal_bar: row.signal_bar as u64,
                entry_bar: row.entry_bar as u64,
                exit_bar: row.exit_bar as u64,
                best: row.best,
                worst: row.worst,
                entry_micros: row.entry_micros,
                exit_micros: row.exit_micros,
                adverse_ppm: row.adverse,
                adverse_paisa: row.adverse_paisa,
                favourable_ppm: row.favourable,
                favourable_paisa: row.favourable_paisa,
            }
            .to_bytes(),
        );
    }
    Ok(out)
}
fn key_at(tier: u64, slot: usize) -> Key {
    Key {
        tier,
        rank: (slot / 2) as u64 + 1,
        direction: if slot.is_multiple_of(2) {
            Direction::Long
        } else {
            Direction::Short
        },
    }
}
fn page_limit(limit: usize) -> Result<(), String> {
    if limit == 0 || limit > MAX_PAGE {
        Err("candidate page limit must be 1..=256".to_owned())
    } else {
        Ok(())
    }
}
fn directory_for(root: &Path, identity: &[u8; 32], attempt: u64, model: Model) -> PathBuf {
    root.join("results")
        .join(match model {
            Model::And => "candidate-trades-v1",
            Model::Expression => "expression-candidate-trades-v1",
        })
        .join(hex(identity))
        .join(attempt.to_string())
}
fn tier_path(dir: &Path, tier: u64) -> PathBuf {
    dir.join(format!("tier-{tier}.bin"))
}
fn candidate_path(dir: &Path, key: Key) -> PathBuf {
    dir.join(format!(
        "{}-{}-{}-candidate.bin",
        key.tier,
        key.rank,
        codec::direction(key.direction)
    ))
}
fn trade_path(dir: &Path, key: Key) -> PathBuf {
    dir.join(format!(
        "{}-{}-{}-trades.bin",
        key.tier,
        key.rank,
        codec::direction(key.direction)
    ))
}
fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for value in bytes {
        let _ = write!(out, "{value:02x}");
    }
    out
}
fn io_error(why: impl std::fmt::Display) -> String {
    format!("candidate detail refused: {why}")
}
fn header(magic: [u8; 8]) -> [u8; HEADER] {
    let mut raw = [0; HEADER];
    for (out, byte) in raw.iter_mut().zip(
        magic
            .into_iter()
            .chain(1_u32.to_le_bytes())
            .chain(0_u32.to_le_bytes()),
    ) {
        *out = byte;
    }
    raw
}
fn verify_header(raw: &[u8; HEADER], magic: [u8; 8]) -> Result<(), String> {
    if *raw == header(magic) {
        Ok(())
    } else {
        Err("unknown candidate detail file header".to_owned())
    }
}
fn write_exact(path: &Path, magic: [u8; 8], payload: &[u8]) -> Result<[u8; 32], String> {
    let header = header(magic);
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(&header);
    hash.update(payload);
    let digest = hash.finalize();
    match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => {
            file.lock().map_err(io_error)?;
            file.write_all(&header)
                .and_then(|()| file.write_all(payload))
                .and_then(|()| file.write_all(&digest))
                .and_then(|()| file.sync_all())
                .map_err(io_error)?;
            file.unlock().map_err(io_error)?;
        }
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(why) => return Err(io_error(why)),
    }
    let budget = (payload.len() as u64)
        .checked_add((HEADER + SEAL) as u64)
        .ok_or("candidate extent overflow")?;
    let (actual, observed) = read_sealed(path, magic, budget)?;
    if observed != digest || actual != payload {
        return Err("immutable candidate detail already contains different bytes".to_owned());
    }
    File::open(path.parent().ok_or("candidate detail path has no parent")?)
        .and_then(|file| file.sync_all())
        .map_err(io_error)?;
    Ok(digest)
}
fn read_sealed(path: &Path, magic: [u8; 8], max_bytes: u64) -> Result<(Vec<u8>, [u8; 32]), String> {
    let mut file = crate::readonly_file::open(path).map_err(io_error)?;
    lock_for_read(&file)?;
    let generation = crate::result_set::file_generation(&file, path)?;
    let metadata = file.metadata().map_err(io_error)?;
    let len = metadata.len();
    if !metadata.is_file() || len < (HEADER + SEAL) as u64 || len > max_bytes {
        return Err(
            "candidate detail file is nonregular, truncated or above its byte admission".to_owned(),
        );
    }
    let mut actual_header = [0; HEADER];
    file.read_exact(&mut actual_header).map_err(io_error)?;
    verify_header(&actual_header, magic)?;
    let payload_len = usize::try_from(len - (HEADER + SEAL) as u64).map_err(io_error)?;
    let mut payload = Vec::new();
    payload.try_reserve_exact(payload_len).map_err(io_error)?;
    payload.resize(payload_len, 0);
    file.read_exact(&mut payload).map_err(io_error)?;
    let mut seal = [0; SEAL];
    file.read_exact(&mut seal).map_err(io_error)?;
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(&actual_header);
    hash.update(&payload);
    if hash.finalize() != seal {
        return Err("candidate detail content seal mismatch".to_owned());
    }
    let observed = crate::result_set::file_generation(&file, path)?;
    crate::result_set::require_generation_unchanged(generation, observed, path)?;
    Ok((payload, seal))
}

fn lock_for_read(file: &File) -> Result<(), String> {
    file.try_lock_shared().map_err(|why| match why {
        std::fs::TryLockError::WouldBlock => {
            "candidate detail is busy; retry this exact saved page".to_owned()
        }
        std::fs::TryLockError::Error(why) => io_error(why),
    })
}

#[cfg(test)]
mod tests;
