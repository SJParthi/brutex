//! Version-separated eight-rung global replay successor.
//!
//! This module owns a new V3 byte format and scheduler boundary. It never
//! opens, copies, or reinterprets V1/V2 replay records. Exactly eight canonical
//! Selection V5 authorities must ultimately supply twenty-five authenticated
//! winners each. Every winner then needs one opaque OOS replay universe whose
//! selected-exit digest equals the durable Execution V3 disposition retained
//! by Selection V5. Those two authorities are deliberately not replaceable by
//! caller-written digests or rows.
//!
//! The in-memory scheduler is complete now: it validates the fixed 8 x 25
//! witness topology, retains every reachable price-refused attempt as real
//! occupancy, offers same-minute intents to [`GlobalSinglePositionV1`], and
//! permits a following entry only strictly after the inclusive occupied-through
//! timestamp. Exact money rows are attached only to globally admitted,
//! priceable decisions. India VIX is looked up only after that admission and is
//! exact-or-absent; it never changes selection, execution, P&L, or replay
//! identity.
//!
//! The production join consumes two opaque capabilities: Selection V5 visits
//! its complete authenticated Top-25s in canonical rung/rank order, while
//! [`GlobalReplayWitnessUniverseV1`] carries the exact Runner-authenticated OOS
//! instrument, feed, direction, selected-exit identity and candidate universe.
//! The join exact-matches those capabilities before deriving a private V3
//! witness. It accepts no caller-authored vendor, family, direction, mask,
//! ranking fact, exit identity or replay digest.
//!
//! # Cost
//!
//! Validation, scheduling, canonical encoding, persistence, and reopen are
//! linear in witness/candidate/decision/money records. Each scheduler minute
//! is bounded by the fixed 200-stream surface. Fixed-record address arithmetic
//! is O(1); filesystem latency, hashing, sorting, scanning, and a complete
//! replay are not O(1).
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

#![expect(
    dead_code,
    reason = "Global Replay V3 remains crate-private until the Step-3 orchestrator moves the all-rung Selection V5 and exact OOS replay capabilities into this terminal join"
)]

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use brutex_core::blake3::Hasher;
use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use costs::fill::Direction;
use indicators::Candle;
use pull::session::IstMoment;
use runner::exit_grid_policy::GlobalReplayWitnessUniverseV1;
use runner::grid::{ReplayCandidateV1, ReplayPathV1, TradeRow};
use runner::portfolio::{
    Constituent, Counters, Disposition, Evidence, GlobalSinglePositionV1, Intent,
    MAX_INTENTS_PER_MINUTE, StrategyDigest,
};
use store::path::YearMonth;

use crate::all_rung_selection_v5::AllRungSelectionV5SuccessorSetV1;
use crate::population::InstrumentFamilyV1;
use crate::selection_v5::SelectionV5SuccessorWinnerV1;
use crate::vix_reference::{VixReferenceMonth, VixStamp};

const VERSION: u32 = 3;
const RUNG_COUNT: usize = 8;
const TOP_PER_RUNG: usize = 25;
const WITNESS_COUNT: usize = RUNG_COUNT * TOP_PER_RUNG;
const CANONICAL_RUNGS_SECONDS: [u32; RUNG_COUNT] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];
const MICROS_PER_MINUTE: i64 = 60_000_000;
const SEAL_BYTES: usize = 32;

pub(crate) const GLOBAL_REPLAY_V3_WITNESS_BYTES: usize = 512;
pub(crate) const GLOBAL_REPLAY_V3_CANDIDATE_BYTES: usize = 512;
pub(crate) const GLOBAL_REPLAY_V3_DECISION_BYTES: usize = 384;
pub(crate) const GLOBAL_REPLAY_V3_MONEY_BYTES: usize = 512;
pub(crate) const GLOBAL_REPLAY_V3_COMPLETION_BYTES: usize = 1_024;

const WITNESS_MAGIC: [u8; 16] = *b"BTX-GRV3-WIT\0\0\0\0";
const CANDIDATE_MAGIC: [u8; 16] = *b"BTX-GRV3-CAN\0\0\0\0";
const DECISION_MAGIC: [u8; 16] = *b"BTX-GRV3-DEC\0\0\0\0";
const MONEY_MAGIC: [u8; 16] = *b"BTX-GRV3-MNY\0\0\0\0";
const COMPLETION_MAGIC: [u8; 16] = *b"BTX-GRV3-CMP\0\0\0\0";

const WITNESS_SEAL_DOMAIN: &[u8] = b"brutex-global-replay-v3-witness-seal\0";
const CANDIDATE_SEAL_DOMAIN: &[u8] = b"brutex-global-replay-v3-candidate-seal\0";
const DECISION_SEAL_DOMAIN: &[u8] = b"brutex-global-replay-v3-decision-seal\0";
const MONEY_SEAL_DOMAIN: &[u8] = b"brutex-global-replay-v3-money-seal\0";
const COMPLETION_SEAL_DOMAIN: &[u8] = b"brutex-global-replay-v3-completion-seal\0";
const WITNESS_ID_DOMAIN: &[u8] = b"brutex-global-replay-v3-witness-id\0";
const CANDIDATE_ID_DOMAIN: &[u8] = b"brutex-global-replay-v3-candidate-id\0";
const DECISION_ID_DOMAIN: &[u8] = b"brutex-global-replay-v3-decision-id\0";
const MONEY_ID_DOMAIN: &[u8] = b"brutex-global-replay-v3-money-id\0";
const ORDERED_WITNESS_DOMAIN: &[u8] = b"brutex-global-replay-v3-witness-order\0";
const ORDERED_CANDIDATE_DOMAIN: &[u8] = b"brutex-global-replay-v3-candidate-order\0";
const ORDERED_DECISION_DOMAIN: &[u8] = b"brutex-global-replay-v3-decision-order\0";
const ORDERED_MONEY_DOMAIN: &[u8] = b"brutex-global-replay-v3-money-order\0";
const REPLAY_ID_DOMAIN: &[u8] = b"brutex-global-replay-v3-replay-id\0";
const PUBLICATION_ID_DOMAIN: &[u8] = b"brutex-global-replay-v3-publication-id\0";
const COMPLETION_ID_DOMAIN: &[u8] = b"brutex-global-replay-v3-completion-id\0";
const VIX_POLICY_DOMAIN: &[u8] = b"brutex-vix-exact-or-absent-no-interpolation-v3\0";

const WITNESS_FILE: &str = "global-replay-witnesses-v3.bin";
const CANDIDATE_FILE: &str = "global-replay-candidates-v3.bin";
const DECISION_FILE: &str = "global-replay-decisions-v3.bin";
const MONEY_FILE: &str = "global-replay-money-v3.bin";
const COMPLETION_FILE: &str = "global-replay-completions-v3.bin";
const LOCK_FILE: &str = "global-replay-v3.lock";

const _: () = assert!(WITNESS_COUNT == MAX_INTENTS_PER_MINUTE);
const _: () = assert!(GLOBAL_REPLAY_V3_WITNESS_BYTES > SEAL_BYTES);
const _: () = assert!(GLOBAL_REPLAY_V3_CANDIDATE_BYTES > SEAL_BYTES);
const _: () = assert!(GLOBAL_REPLAY_V3_DECISION_BYTES > SEAL_BYTES);
const _: () = assert!(GLOBAL_REPLAY_V3_MONEY_BYTES > SEAL_BYTES);
const _: () = assert!(GLOBAL_REPLAY_V3_COMPLETION_BYTES > SEAL_BYTES);

type GlobalReplayV3Refusal = String;

/// Explicit physical ceilings for every V3 file. There is no `Default`.
#[expect(
    clippy::struct_field_names,
    reason = "each independently bounded V3 ledger names its physical record ceiling explicitly"
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GlobalReplayV3Bounds {
    witness_records: u64,
    candidate_records: u64,
    decision_records: u64,
    money_records: u64,
    completion_records: u64,
}

impl GlobalReplayV3Bounds {
    pub(crate) fn new(
        witness_records: u64,
        candidate_records: u64,
        decision_records: u64,
        money_records: u64,
        completion_records: u64,
    ) -> Result<Self, GlobalReplayV3Refusal> {
        for (name, value) in [
            ("witness", witness_records),
            ("candidate", candidate_records),
            ("decision", decision_records),
            ("money", money_records),
            ("Completion", completion_records),
        ] {
            if value == 0 {
                return Err(format!(
                    "Global Replay V3 {name} record bound must be nonzero"
                ));
            }
        }
        if witness_records < WITNESS_COUNT as u64 {
            return Err(format!(
                "Global Replay V3 witness bound {witness_records} cannot hold the fixed {WITNESS_COUNT}-witness block"
            ));
        }
        Ok(Self {
            witness_records,
            candidate_records,
            decision_records,
            money_records,
            completion_records,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum CandidatePathV3 {
    Priceable = 1,
    BlockOnly = 2,
    CrossingRefused = 3,
    BlockOnlyAndCrossingRefused = 4,
}

impl CandidatePathV3 {
    const fn pricing_refused(self) -> bool {
        !matches!(self, Self::Priceable)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DecisionDispositionV3 {
    Admitted { occupied_through_micros: i64 },
    BlockedOccupied { occupied_through_micros: i64 },
    BlockedSimultaneous { admitted: [u8; 32] },
    Unreachable,
    Refused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VixPairV3 {
    entry: VixStamp,
    exit: VixStamp,
}

/// Private typed target for the runner successor. No caller-authored
/// constructor exists in production.
#[derive(Clone, Debug, PartialEq, Eq)]
struct AuthenticatedReplayWitnessV3 {
    selection_id: [u8; 32],
    row_id: [u8; 32],
    selected_exit_digest: [u8; 32],
    universe_digest: [u8; 32],
    run_id: [u8; 32],
    feed: Vendor,
    family: InstrumentFamilyV1,
    direction: Direction,
    strategy_digest: [u8; 32],
    mask_words: [u64; 6],
    rung_seconds: u32,
    rank: u16,
    candidates: Vec<AuthenticatedReplayCandidateV3>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AuthenticatedReplayCandidateV3 {
    signal_bar: u64,
    entry_bar: u64,
    occupied_through_bar: u64,
    signal_micros: i64,
    entry_micros: i64,
    occupied_through_micros: i64,
    path: CandidatePathV3,
    price: Option<TradeRow>,
    ambiguous_bars: u64,
    gap_fills: u64,
}

impl AuthenticatedReplayCandidateV3 {
    fn validate(self) -> Result<(), GlobalReplayV3Refusal> {
        if self.signal_bar > self.entry_bar
            || self.entry_bar > self.occupied_through_bar
            || self.signal_micros > self.entry_micros
            || self.entry_micros > self.occupied_through_micros
            || self.signal_micros.rem_euclid(MICROS_PER_MINUTE) != 0
            || self.entry_micros.rem_euclid(MICROS_PER_MINUTE) != 0
            || self.occupied_through_micros.rem_euclid(MICROS_PER_MINUTE) != 0
        {
            return Err("Global Replay V3 candidate has impossible causal coordinates".to_owned());
        }
        match (self.path, self.price) {
            (CandidatePathV3::Priceable, Some(row)) => {
                if usize_to_u64(row.signal_bar, "money signal bar")? != self.signal_bar
                    || usize_to_u64(row.entry_bar, "money entry bar")? != self.entry_bar
                    || usize_to_u64(row.exit_bar, "money exit bar")? != self.occupied_through_bar
                    || row.entry_micros != self.entry_micros
                    || row.exit_micros != self.occupied_through_micros
                    || self.ambiguous_bars > 1
                    || self.gap_fills > 1
                    || row.adverse < 0
                    || row.adverse_paisa < 0
                    || row.favourable < 0
                    || row.favourable_paisa < 0
                {
                    return Err(
                        "Global Replay V3 priceable candidate has contradictory exact money evidence"
                            .to_owned(),
                    );
                }
            }
            (CandidatePathV3::Priceable, None) => {
                return Err("Global Replay V3 priceable candidate has no money row".to_owned());
            }
            (_, Some(_)) => {
                return Err(
                    "Global Replay V3 pricing-refused candidate acquired a money row".to_owned(),
                );
            }
            (_, None) => {
                if self.ambiguous_bars != 0 || self.gap_fills != 0 {
                    return Err(
                        "Global Replay V3 pricing-refused candidate invented quality counts"
                            .to_owned(),
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WitnessRecordV3 {
    witness_id: [u8; 32],
    selection_id: [u8; 32],
    row_id: [u8; 32],
    selected_exit_digest: [u8; 32],
    universe_digest: [u8; 32],
    run_id: [u8; 32],
    feed_digest: [u8; 32],
    strategy_digest: [u8; 32],
    mask_words: [u64; 6],
    ordered_candidate_digest: [u8; 32],
    candidate_first: u64,
    candidate_count: u64,
    pricing_refused_count: u64,
    rung_seconds: u32,
    rank: u16,
    family: InstrumentFamilyV1,
    direction: Direction,
    feed: Vendor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CandidateRecordV3 {
    candidate_id: [u8; 32],
    witness_id: [u8; 32],
    selection_id: [u8; 32],
    row_id: [u8; 32],
    strategy_digest: [u8; 32],
    stream_ordinal: u16,
    candidate_ordinal: u64,
    signal_bar: u64,
    entry_bar: u64,
    occupied_through_bar: u64,
    signal_micros: i64,
    entry_micros: i64,
    occupied_through_micros: i64,
    path: CandidatePathV3,
    price: Option<TradeRow>,
    ambiguous_bars: u64,
    gap_fills: u64,
    rung_seconds: u32,
    rank: u16,
    family: InstrumentFamilyV1,
    direction: Direction,
    feed: Vendor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecisionRecordV3 {
    decision_id: [u8; 32],
    candidate_id: [u8; 32],
    witness_id: [u8; 32],
    strategy_digest: [u8; 32],
    sequence: u64,
    stream_ordinal: u16,
    candidate_ordinal: u64,
    entry_micros: i64,
    occupied_through_micros: i64,
    rung_seconds: u32,
    rank: u16,
    family: InstrumentFamilyV1,
    direction: Direction,
    path: CandidatePathV3,
    disposition: DecisionDispositionV3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MoneyRecordV3 {
    money_id: [u8; 32],
    decision_id: [u8; 32],
    candidate_id: [u8; 32],
    witness_id: [u8; 32],
    strategy_digest: [u8; 32],
    decision_sequence: u64,
    stream_ordinal: u16,
    rung_seconds: u32,
    rank: u16,
    family: InstrumentFamilyV1,
    direction: Direction,
    feed: Vendor,
    row: TradeRow,
    vix: VixPairV3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompletionRecordV3 {
    completion_id: [u8; 32],
    replay_id: [u8; 32],
    publication_id: [u8; 32],
    selection_ids: [[u8; 32]; RUNG_COUNT],
    ordered_witness_digest: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    ordered_money_digest: [u8; 32],
    witness_first: u64,
    witness_count: u64,
    candidate_first: u64,
    candidate_count: u64,
    decision_first: u64,
    decision_count: u64,
    money_first: u64,
    money_count: u64,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
    block_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedGlobalReplayV3 {
    witnesses: Vec<WitnessRecordV3>,
    candidates: Vec<CandidateRecordV3>,
    decisions: Vec<DecisionRecordV3>,
    money: Vec<MoneyRecordV3>,
    selection_ids: [[u8; 32]; RUNG_COUNT],
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
    ordered_witness_digest: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    ordered_money_digest: [u8; 32],
    replay_id: [u8; 32],
    publication_id: [u8; 32],
}

/// Freshly reopened projection of one committed V3 publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GlobalReplayV3ReopenAudit {
    completion_id: [u8; 32],
    replay_id: [u8; 32],
    publication_id: [u8; 32],
    witness_count: u64,
    candidate_count: u64,
    decision_count: u64,
    money_count: u64,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
}

impl GlobalReplayV3ReopenAudit {
    #[must_use]
    pub(crate) const fn completion_id(self) -> [u8; 32] {
        self.completion_id
    }

    #[must_use]
    pub(crate) const fn replay_id(self) -> [u8; 32] {
        self.replay_id
    }

    #[must_use]
    pub(crate) const fn publication_id(self) -> [u8; 32] {
        self.publication_id
    }

    #[must_use]
    pub(crate) const fn witness_count(self) -> u64 {
        self.witness_count
    }

    #[must_use]
    pub(crate) const fn candidate_count(self) -> u64 {
        self.candidate_count
    }

    #[must_use]
    pub(crate) const fn decision_count(self) -> u64 {
        self.decision_count
    }

    #[must_use]
    pub(crate) const fn money_count(self) -> u64 {
        self.money_count
    }

    #[must_use]
    pub(crate) const fn counters(self) -> Counters {
        self.counters
    }

    #[must_use]
    pub(crate) const fn pricing_refused_candidates(self) -> u64 {
        self.pricing_refused_candidates
    }

    #[must_use]
    pub(crate) const fn admitted_pricing_refused(self) -> u64 {
        self.admitted_pricing_refused
    }
}

trait ExactVixAuthorityV3 {
    fn stamp(&self, feed: Vendor, ts_micros: i64) -> Result<VixStamp, GlobalReplayV3Refusal>;
}

struct StoredVixCatalogV3<'a> {
    by_month: HashMap<(Vendor, YearMonth), &'a VixReferenceMonth>,
}

impl<'a> StoredVixCatalogV3<'a> {
    fn new(months: &'a [VixReferenceMonth]) -> Result<Self, GlobalReplayV3Refusal> {
        let mut by_month = HashMap::new();
        by_month
            .try_reserve(months.len())
            .map_err(|why| format!("Global Replay V3 VIX month index allocation refused: {why}"))?;
        for month in months {
            let key = (month.vendor(), month.month());
            if by_month.insert(key, month).is_some() {
                return Err(format!(
                    "Global Replay V3 VIX authority {} {} was supplied more than once",
                    month.vendor().as_str(),
                    month.month()
                ));
            }
        }
        Ok(Self { by_month })
    }
}

impl ExactVixAuthorityV3 for StoredVixCatalogV3<'_> {
    fn stamp(&self, feed: Vendor, ts_micros: i64) -> Result<VixStamp, GlobalReplayV3Refusal> {
        let seconds = ts_micros.div_euclid(1_000_000);
        let moment = IstMoment::from_epoch_secs(seconds).map_err(|why| {
            format!("Global Replay V3 VIX timestamp {ts_micros} is not an IST minute: {why}")
        })?;
        let month = moment.day().year_month().map_err(|why| {
            format!("Global Replay V3 VIX timestamp {ts_micros} has no storable month: {why}")
        })?;
        let authority = self.by_month.get(&(feed, month)).ok_or_else(|| {
            format!(
                "Global Replay V3 globally admitted trade at {ts_micros} has no {} India VIX authority for {month}",
                feed.as_str()
            )
        })?;
        authority.stamp(ts_micros)
    }
}

impl PreparedGlobalReplayV3 {
    /// Joins the sole canonical all-rung Selection V5 capability to exactly
    /// 200 opaque Runner OOS replay capabilities, then prepares the V3 block.
    ///
    /// Selection performs its complete eight-rung preflight before the first
    /// callback. Each callback exact-matches the durable selected-exit identity
    /// and Runner-authenticated instrument/direction before any private V3 row
    /// is derived. A shuffled, duplicated, missing or foreign replay capability
    /// therefore refuses; caller order never becomes authority by assertion.
    fn prepare_from_authorities(
        selection: AllRungSelectionV5SuccessorSetV1,
        replay: Vec<GlobalReplayWitnessUniverseV1>,
        vix: &impl ExactVixAuthorityV3,
    ) -> Result<Self, GlobalReplayV3Refusal> {
        if replay.len() != WITNESS_COUNT {
            return Err(format!(
                "Global Replay V3 requires exactly {WITNESS_COUNT} opaque Runner OOS replay capabilities, not {}",
                replay.len()
            ));
        }
        let mut replay = replay.into_iter();
        let mut authenticated = Vec::new();
        authenticated
            .try_reserve_exact(WITNESS_COUNT)
            .map_err(|why| format!("Global Replay V3 authority-join allocation refused: {why}"))?;
        selection.visit_canonical(|winner| {
            let universe = replay.next().ok_or_else(|| {
                "Global Replay V3 Runner OOS capability stream ended before Selection V5".to_owned()
            })?;
            authenticated.push(join_successor_winner(&winner, universe)?);
            Ok(())
        })?;
        if replay.next().is_some() || authenticated.len() != WITNESS_COUNT {
            return Err(
                "Global Replay V3 Runner OOS capability count diverged from canonical Selection V5"
                    .to_owned(),
            );
        }
        Self::prepare(&authenticated, vix)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one fail-before-scheduling pass binds the fixed 8x25 topology and every candidate range into one atomic preparation"
    )]
    fn prepare(
        authenticated: &[AuthenticatedReplayWitnessV3],
        vix: &impl ExactVixAuthorityV3,
    ) -> Result<Self, GlobalReplayV3Refusal> {
        if authenticated.len() != WITNESS_COUNT {
            return Err(format!(
                "Global Replay V3 requires exactly {WITNESS_COUNT} authenticated Selection V5 winners, not {}",
                authenticated.len()
            ));
        }

        let mut witnesses = Vec::new();
        witnesses.try_reserve_exact(WITNESS_COUNT).map_err(|why| {
            format!("Global Replay V3 witness allocation refused before scheduling: {why}")
        })?;
        let candidate_total = authenticated.iter().try_fold(0_usize, |total, witness| {
            total
                .checked_add(witness.candidates.len())
                .ok_or_else(|| "Global Replay V3 candidate population length overflowed".to_owned())
        })?;
        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(candidate_total)
            .map_err(|why| {
                format!("Global Replay V3 candidate allocation refused before scheduling: {why}")
            })?;
        let mut row_ids = HashSet::new();
        row_ids.try_reserve(WITNESS_COUNT).map_err(|why| {
            format!("Global Replay V3 witness-identity index allocation refused: {why}")
        })?;
        let mut selection_ids = [[0_u8; 32]; RUNG_COUNT];

        for (stream_ordinal, authority) in authenticated.iter().enumerate() {
            let rung_index = stream_ordinal / TOP_PER_RUNG;
            let expected_rung = CANONICAL_RUNGS_SECONDS
                .get(rung_index)
                .copied()
                .ok_or_else(|| {
                    format!(
                        "Global Replay V3 stream {stream_ordinal} escaped the fixed rung topology"
                    )
                })?;
            let expected_rank = u16::try_from((stream_ordinal % TOP_PER_RUNG) + 1)
                .map_err(|_| "Global Replay V3 fixed rank does not fit u16".to_owned())?;
            if authority.rung_seconds != expected_rung || authority.rank != expected_rank {
                return Err(format!(
                    "Global Replay V3 witness {stream_ordinal} has rung/rank {}s/{}, expected {expected_rung}s/{expected_rank}; caller order is not authority",
                    authority.rung_seconds, authority.rank
                ));
            }
            for (name, digest) in [
                ("selection", &authority.selection_id),
                ("selection row", &authority.row_id),
                ("selected exit", &authority.selected_exit_digest),
                ("OOS universe", &authority.universe_digest),
                ("OOS run", &authority.run_id),
                ("strategy", &authority.strategy_digest),
            ] {
                require_digest(name, digest)?;
            }
            if authority.mask_words.iter().all(|word| *word == 0) {
                return Err(format!(
                    "Global Replay V3 witness {stream_ordinal} has an empty condition mask"
                ));
            }
            if !row_ids.insert(authority.row_id) {
                return Err(format!(
                    "Global Replay V3 selection row {} appears in more than one canonical witness slot",
                    hex(&authority.row_id)
                ));
            }
            let held_selection = selection_ids.get_mut(rung_index).ok_or_else(|| {
                format!("Global Replay V3 stream {stream_ordinal} has no canonical Selection slot")
            })?;
            if *held_selection == [0; 32] {
                *held_selection = authority.selection_id;
            } else if *held_selection != authority.selection_id {
                return Err(format!(
                    "Global Replay V3 rung {expected_rung}s mixes Selection V5 authorities {} and {}",
                    hex(held_selection),
                    hex(&authority.selection_id)
                ));
            }

            let candidate_first = usize_to_u64(candidates.len(), "candidate first")?;
            let mut previous: Option<(u64, i64)> = None;
            let mut pricing_refused_count = 0_u64;
            let witness_id = witness_id(authority);
            for (candidate_ordinal, candidate) in authority.candidates.iter().copied().enumerate() {
                candidate.validate()?;
                if previous.is_some_and(|(entry_bar, entry_micros)| {
                    candidate.entry_bar <= entry_bar || candidate.entry_micros <= entry_micros
                }) {
                    return Err(format!(
                        "Global Replay V3 witness {stream_ordinal} candidates are not strictly increasing in entry bar and exact minute"
                    ));
                }
                previous = Some((candidate.entry_bar, candidate.entry_micros));
                pricing_refused_count = pricing_refused_count
                    .checked_add(u64::from(candidate.path.pricing_refused()))
                    .ok_or_else(|| {
                        "Global Replay V3 pricing-refused candidate count overflowed".to_owned()
                    })?;
                let candidate_ordinal = usize_to_u64(candidate_ordinal, "candidate ordinal")?;
                let candidate_id = candidate_id(witness_id, candidate_ordinal, candidate)?;
                candidates.push(CandidateRecordV3 {
                    candidate_id,
                    witness_id,
                    selection_id: authority.selection_id,
                    row_id: authority.row_id,
                    strategy_digest: authority.strategy_digest,
                    stream_ordinal: u16::try_from(stream_ordinal).map_err(|_| {
                        "Global Replay V3 stream ordinal does not fit u16".to_owned()
                    })?,
                    candidate_ordinal,
                    signal_bar: candidate.signal_bar,
                    entry_bar: candidate.entry_bar,
                    occupied_through_bar: candidate.occupied_through_bar,
                    signal_micros: candidate.signal_micros,
                    entry_micros: candidate.entry_micros,
                    occupied_through_micros: candidate.occupied_through_micros,
                    path: candidate.path,
                    price: candidate.price,
                    ambiguous_bars: candidate.ambiguous_bars,
                    gap_fills: candidate.gap_fills,
                    rung_seconds: authority.rung_seconds,
                    rank: authority.rank,
                    family: authority.family,
                    direction: authority.direction,
                    feed: authority.feed,
                });
            }
            witnesses.push(WitnessRecordV3 {
                witness_id,
                selection_id: authority.selection_id,
                row_id: authority.row_id,
                selected_exit_digest: authority.selected_exit_digest,
                universe_digest: authority.universe_digest,
                run_id: authority.run_id,
                feed_digest: feed_digest(authority.feed),
                strategy_digest: authority.strategy_digest,
                mask_words: authority.mask_words,
                ordered_candidate_digest: ordered_ids(
                    ORDERED_CANDIDATE_DOMAIN,
                    candidates
                        .get(u64_to_usize(candidate_first, "candidate first")?..)
                        .ok_or_else(|| {
                            "Global Replay V3 witness candidate range escaped preparation"
                                .to_owned()
                        })?
                        .iter()
                        .map(|record| record.candidate_id),
                ),
                candidate_first,
                candidate_count: usize_to_u64(
                    authority.candidates.len(),
                    "witness candidate count",
                )?,
                pricing_refused_count,
                rung_seconds: authority.rung_seconds,
                rank: authority.rank,
                family: authority.family,
                direction: authority.direction,
                feed: authority.feed,
            });
        }
        let mut unique_selections = HashSet::new();
        for selection_id in selection_ids {
            if !unique_selections.insert(selection_id) {
                return Err(format!(
                    "Global Replay V3 reuses Selection V5 authority {} for more than one rung",
                    hex(&selection_id)
                ));
            }
        }

        let scheduled = schedule_candidates(&witnesses, &candidates, Some(vix))?;
        let ordered_witness_digest = ordered_ids(
            ORDERED_WITNESS_DOMAIN,
            witnesses.iter().map(|record| record.witness_id),
        );
        let ordered_candidate_digest = ordered_ids(
            ORDERED_CANDIDATE_DOMAIN,
            candidates.iter().map(|record| record.candidate_id),
        );
        let ordered_decision_digest = ordered_ids(
            ORDERED_DECISION_DOMAIN,
            scheduled.decisions.iter().map(|record| record.decision_id),
        );
        let ordered_money_digest = ordered_ids(
            ORDERED_MONEY_DOMAIN,
            scheduled.money.iter().map(|record| record.money_id),
        );
        let replay_id = replay_id(
            &selection_ids,
            ordered_witness_digest,
            ordered_candidate_digest,
            ordered_decision_digest,
            scheduled.counters,
            scheduled.pricing_refused_candidates,
            scheduled.admitted_pricing_refused,
        );
        let publication_id = publication_id(replay_id, ordered_money_digest);
        Ok(Self {
            witnesses,
            candidates,
            decisions: scheduled.decisions,
            money: scheduled.money,
            selection_ids,
            counters: scheduled.counters,
            pricing_refused_candidates: scheduled.pricing_refused_candidates,
            admitted_pricing_refused: scheduled.admitted_pricing_refused,
            ordered_witness_digest,
            ordered_candidate_digest,
            ordered_decision_digest,
            ordered_money_digest,
            replay_id,
            publication_id,
        })
    }
}

fn join_successor_winner(
    winner: &SelectionV5SuccessorWinnerV1,
    replay: GlobalReplayWitnessUniverseV1,
) -> Result<AuthenticatedReplayWitnessV3, GlobalReplayV3Refusal> {
    replay
        .require_integrity()
        .map_err(|why| format!("Global Replay V3 Runner OOS capability refused: {why}"))?;
    let row = winner.row();
    let selected_exit_digest = winner.selected_exit_digest().ok_or_else(|| {
        "Global Replay V3 Selection V5 winner has no authorized selected-exit identity".to_owned()
    })?;
    if replay.selected_exit_digest() != selected_exit_digest {
        return Err(format!(
            "Global Replay V3 Selection V5 row {} and Runner OOS replay carry different selected exits",
            hex(&row.row_id())
        ));
    }
    let family = row.instrument_family();
    if replay.instrument() != instrument_of(family)? {
        return Err(format!(
            "Global Replay V3 Selection V5 row {} and Runner OOS replay carry different swept instruments",
            hex(&row.row_id())
        ));
    }
    if replay.direction() != row.direction() {
        return Err(format!(
            "Global Replay V3 Selection V5 row {} and Runner OOS replay carry different directions",
            hex(&row.row_id())
        ));
    }

    let first_oos = replay.first_oos();
    let universe_digest = replay.universe_digest();
    let run_id = replay.run_id().bytes();
    let feed = replay.feed();
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(replay.candidates().len())
        .map_err(|why| format!("Global Replay V3 candidate join allocation refused: {why}"))?;
    for candidate in replay.candidates() {
        if candidate.signal_bar() < first_oos {
            return Err(format!(
                "Global Replay V3 Runner OOS candidate signal {} precedes authenticated OOS boundary {}",
                candidate.signal_bar(),
                first_oos
            ));
        }
        candidates.push(authenticated_candidate(candidate)?);
    }

    // The capability is intentionally one-use. All authenticated projections
    // have been copied into the private V3 witness, so consume the opaque
    // Runner owner before returning that replacement authority.
    drop(replay);

    Ok(AuthenticatedReplayWitnessV3 {
        selection_id: row.selection_id(),
        row_id: row.row_id(),
        selected_exit_digest,
        universe_digest,
        run_id,
        feed,
        family,
        direction: row.direction(),
        strategy_digest: row.strategy_digest(),
        mask_words: row.mask_words(),
        rung_seconds: row.rung_seconds(),
        rank: u16::try_from(row.rank())
            .map_err(|_| "Global Replay V3 Selection V5 rank does not fit u16".to_owned())?,
        candidates,
    })
}

fn authenticated_candidate(
    candidate: &ReplayCandidateV1,
) -> Result<AuthenticatedReplayCandidateV3, GlobalReplayV3Refusal> {
    let (path, price, ambiguous_bars, gap_fills) = match candidate.path() {
        ReplayPathV1::Priceable(price) => (
            CandidatePathV3::Priceable,
            Some(price.row()),
            price.ambiguous_bars(),
            price.gap_fills(),
        ),
        ReplayPathV1::BlockOnly => (CandidatePathV3::BlockOnly, None, 0, 0),
        ReplayPathV1::CrossingRefused => (CandidatePathV3::CrossingRefused, None, 0, 0),
        ReplayPathV1::BlockOnlyAndCrossingRefused => {
            (CandidatePathV3::BlockOnlyAndCrossingRefused, None, 0, 0)
        }
    };
    Ok(AuthenticatedReplayCandidateV3 {
        signal_bar: usize_to_u64(candidate.signal_bar(), "candidate signal bar")?,
        entry_bar: usize_to_u64(candidate.entry_bar(), "candidate entry bar")?,
        occupied_through_bar: usize_to_u64(
            candidate.occupied_through_bar(),
            "candidate occupied-through bar",
        )?,
        signal_micros: candidate.signal_micros(),
        entry_micros: candidate.entry_micros(),
        occupied_through_micros: candidate.occupied_through_micros(),
        path,
        price,
        ambiguous_bars,
        gap_fills,
    })
}

struct ScheduledGlobalReplayV3 {
    decisions: Vec<DecisionRecordV3>,
    money: Vec<MoneyRecordV3>,
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
}

#[expect(
    clippy::too_many_lines,
    reason = "the single global scheduler fold keeps minute grouping, occupancy decisions, exact money, and VIX publication in one auditable state transition"
)]
fn schedule_candidates(
    witnesses: &[WitnessRecordV3],
    candidates: &[CandidateRecordV3],
    vix: Option<&dyn ExactVixAuthorityV3>,
) -> Result<ScheduledGlobalReplayV3, GlobalReplayV3Refusal> {
    let mut order: Vec<(usize, CandidateRecordV3)> =
        candidates.iter().copied().enumerate().collect();
    order.sort_unstable_by_key(|(_, record)| {
        (
            record.entry_micros,
            record.rank,
            record.strategy_digest,
            record.witness_id,
            record.candidate_ordinal,
        )
    });
    let mut decisions = Vec::new();
    decisions
        .try_reserve_exact(candidates.len())
        .map_err(|why| {
            format!("Global Replay V3 decision allocation refused before scheduling: {why}")
        })?;
    let mut money = Vec::new();
    money.try_reserve(candidates.len()).map_err(|why| {
        format!("Global Replay V3 money allocation refused before scheduling: {why}")
    })?;
    let pricing_refused_candidates = candidates.iter().try_fold(0_u64, |count, record| {
        count
            .checked_add(u64::from(record.path.pricing_refused()))
            .ok_or_else(|| "Global Replay V3 pricing-refused count overflowed".to_owned())
    })?;
    let mut admitted_pricing_refused = 0_u64;
    let mut scheduler = GlobalSinglePositionV1::new();
    let mut cursor = 0_usize;
    while cursor < order.len() {
        let first = order
            .get(cursor)
            .map(|(_, candidate)| candidate.entry_micros)
            .ok_or_else(|| "Global Replay V3 minute cursor escaped candidate order".to_owned())?;
        let mut end = cursor + 1;
        while order
            .get(end)
            .is_some_and(|(_, candidate)| candidate.entry_micros == first)
        {
            end += 1;
        }
        let group = order
            .get(cursor..end)
            .ok_or_else(|| "Global Replay V3 minute group escaped candidate order".to_owned())?;
        if group.len() > MAX_INTENTS_PER_MINUTE {
            return Err(format!(
                "Global Replay V3 minute {first} carries {} candidate intents, above the fixed {MAX_INTENTS_PER_MINUTE} bound",
                group.len()
            ));
        }
        let mut intents = Vec::new();
        intents
            .try_reserve_exact(group.len())
            .map_err(|why| format!("Global Replay V3 minute intent allocation refused: {why}"))?;
        let mut candidate_by_key = HashMap::new();
        candidate_by_key.try_reserve(group.len()).map_err(|why| {
            format!("Global Replay V3 minute correlation allocation refused: {why}")
        })?;
        for (_, candidate) in group.iter().copied() {
            let key = (candidate.rank, candidate.strategy_digest);
            if candidate_by_key.insert(key, candidate).is_some() {
                return Err(format!(
                    "Global Replay V3 minute {first} repeats rank/strategy scheduling key {}/{}",
                    candidate.rank,
                    hex(&candidate.strategy_digest)
                ));
            }
            intents.push(Intent {
                constituent: Constituent {
                    priority: candidate.rank,
                    strategy_digest: StrategyDigest::new(candidate.strategy_digest),
                    instrument: instrument_of(candidate.family)?,
                    direction: candidate.direction,
                    rung_minutes: u16::try_from(candidate.rung_seconds / 60)
                        .map_err(|_| "Global Replay V3 rung minutes do not fit u16".to_owned())?,
                },
                evidence: Evidence::Reachable {
                    occupied_through_micros: candidate.occupied_through_micros,
                },
            });
        }
        let minute = scheduler
            .schedule_minute(first, &intents)
            .map_err(|refusal| {
                format!("Global Replay V3 global scheduler refused minute {first}: {refusal:?}")
            })?;
        for decision in minute.decisions() {
            let key = (
                decision.constituent.priority,
                decision.constituent.strategy_digest.bytes(),
            );
            let candidate = candidate_by_key.remove(&key).ok_or_else(|| {
                "Global Replay V3 scheduler returned a constituent absent from its offered minute"
                    .to_owned()
            })?;
            let sequence = usize_to_u64(decisions.len(), "decision sequence")?;
            let disposition = disposition_of(decision.disposition);
            let decision_id = decision_id(sequence, &candidate, disposition);
            let durable = DecisionRecordV3 {
                decision_id,
                candidate_id: candidate.candidate_id,
                witness_id: candidate.witness_id,
                strategy_digest: candidate.strategy_digest,
                sequence,
                stream_ordinal: candidate.stream_ordinal,
                candidate_ordinal: candidate.candidate_ordinal,
                entry_micros: candidate.entry_micros,
                occupied_through_micros: candidate.occupied_through_micros,
                rung_seconds: candidate.rung_seconds,
                rank: candidate.rank,
                family: candidate.family,
                direction: candidate.direction,
                path: candidate.path,
                disposition,
            };
            if matches!(decision.disposition, Disposition::Admitted { .. }) {
                if let Some(row) = candidate.price {
                    let Some(vix) = vix else {
                        return Err(
                            "Global Replay V3 reopen cannot reconstruct a missing admitted money row"
                                .to_owned(),
                        );
                    };
                    let stamps = VixPairV3 {
                        entry: vix.stamp(candidate.feed, row.entry_micros)?,
                        exit: vix.stamp(candidate.feed, row.exit_micros)?,
                    };
                    validate_vix_stamp(stamps.entry, row.entry_micros, "entry")?;
                    validate_vix_stamp(stamps.exit, row.exit_micros, "exit")?;
                    let money_id = money_id(decision_id, row, stamps)?;
                    money.push(MoneyRecordV3 {
                        money_id,
                        decision_id,
                        candidate_id: candidate.candidate_id,
                        witness_id: candidate.witness_id,
                        strategy_digest: candidate.strategy_digest,
                        decision_sequence: sequence,
                        stream_ordinal: candidate.stream_ordinal,
                        rung_seconds: candidate.rung_seconds,
                        rank: candidate.rank,
                        family: candidate.family,
                        direction: candidate.direction,
                        feed: candidate.feed,
                        row,
                        vix: stamps,
                    });
                } else {
                    admitted_pricing_refused =
                        admitted_pricing_refused.checked_add(1).ok_or_else(|| {
                            "Global Replay V3 admitted pricing-refused count overflowed".to_owned()
                        })?;
                }
            }
            decisions.push(durable);
        }
        if !candidate_by_key.is_empty() {
            return Err(
                "Global Replay V3 scheduler omitted one or more offered constituents".to_owned(),
            );
        }
        cursor = end;
    }
    let counters = scheduler.counters();
    if !counters.reconciles()
        || counters.offered != usize_to_u64(candidates.len(), "scheduled candidate count")?
        || counters.admitted
            != usize_to_u64(money.len(), "admitted money count")?
                .checked_add(admitted_pricing_refused)
                .ok_or_else(|| "Global Replay V3 admitted reconciliation overflowed".to_owned())?
    {
        return Err("Global Replay V3 scheduler counters do not reconcile".to_owned());
    }
    for (ordinal, witness) in witnesses.iter().enumerate() {
        if usize::from(witness.rank) != (ordinal % TOP_PER_RUNG) + 1 {
            return Err("Global Replay V3 witness order changed during scheduling".to_owned());
        }
    }
    Ok(ScheduledGlobalReplayV3 {
        decisions,
        money,
        counters,
        pricing_refused_candidates,
        admitted_pricing_refused,
    })
}

const fn disposition_of(disposition: Disposition) -> DecisionDispositionV3 {
    match disposition {
        Disposition::Admitted {
            occupied_through_micros,
        } => DecisionDispositionV3::Admitted {
            occupied_through_micros,
        },
        Disposition::BlockedOccupied {
            occupied_through_micros,
        } => DecisionDispositionV3::BlockedOccupied {
            occupied_through_micros,
        },
        Disposition::BlockedSimultaneous { admitted } => {
            DecisionDispositionV3::BlockedSimultaneous {
                admitted: admitted.bytes(),
            }
        }
        Disposition::Unreachable => DecisionDispositionV3::Unreachable,
        Disposition::Refused(_) => DecisionDispositionV3::Refused,
    }
}

fn witness_id(authority: &AuthenticatedReplayWitnessV3) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(WITNESS_ID_DOMAIN);
    for digest in [
        authority.selection_id,
        authority.row_id,
        authority.selected_exit_digest,
        authority.universe_digest,
        authority.run_id,
        feed_digest(authority.feed),
        authority.strategy_digest,
    ] {
        hasher.update(&digest);
    }
    for word in authority.mask_words {
        hasher.update(&word.to_le_bytes());
    }
    hasher.update(&authority.rung_seconds.to_le_bytes());
    hasher.update(&authority.rank.to_le_bytes());
    hasher.update(&[
        family_byte(authority.family),
        direction_byte(authority.direction),
    ]);
    hasher.finalize()
}

fn candidate_id(
    witness_id: [u8; 32],
    candidate_ordinal: u64,
    candidate: AuthenticatedReplayCandidateV3,
) -> Result<[u8; 32], String> {
    candidate.validate()?;
    let mut hasher = Hasher::new();
    hasher.update(CANDIDATE_ID_DOMAIN);
    hasher.update(&witness_id);
    hasher.update(&candidate_ordinal.to_le_bytes());
    put_candidate_fields(&mut hasher, candidate)?;
    Ok(hasher.finalize())
}

fn decision_id(
    sequence: u64,
    candidate: &CandidateRecordV3,
    disposition: DecisionDispositionV3,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(DECISION_ID_DOMAIN);
    hasher.update(&sequence.to_le_bytes());
    hasher.update(&candidate.candidate_id);
    put_disposition(&mut hasher, disposition);
    hasher.finalize()
}

fn money_id(decision_id: [u8; 32], row: TradeRow, vix: VixPairV3) -> Result<[u8; 32], String> {
    let mut hasher = Hasher::new();
    hasher.update(MONEY_ID_DOMAIN);
    hasher.update(&decision_id);
    put_trade_row(&mut hasher, row)?;
    put_vix_stamp(&mut hasher, vix.entry);
    put_vix_stamp(&mut hasher, vix.exit);
    Ok(hasher.finalize())
}

fn ordered_ids(domain: &[u8], ids: impl Iterator<Item = [u8; 32]>) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(domain);
    let mut count = 0_u64;
    for id in ids {
        hasher.update(&id);
        count = count.saturating_add(1);
    }
    hasher.update(&count.to_le_bytes());
    hasher.finalize()
}

fn replay_id(
    selection_ids: &[[u8; 32]; RUNG_COUNT],
    witness_digest: [u8; 32],
    candidate_digest: [u8; 32],
    decision_digest: [u8; 32],
    counters: Counters,
    pricing_refused: u64,
    admitted_pricing_refused: u64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(REPLAY_ID_DOMAIN);
    for selection_id in selection_ids {
        hasher.update(selection_id);
    }
    for digest in [witness_digest, candidate_digest, decision_digest] {
        hasher.update(&digest);
    }
    put_counters(&mut hasher, counters);
    hasher.update(&pricing_refused.to_le_bytes());
    hasher.update(&admitted_pricing_refused.to_le_bytes());
    hasher.finalize()
}

fn publication_id(replay_id: [u8; 32], money_digest: [u8; 32]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(PUBLICATION_ID_DOMAIN);
    hasher.update(&replay_id);
    hasher.update(&money_digest);
    hasher.update(VIX_POLICY_DOMAIN);
    hasher.finalize()
}

fn completion_id(record: &CompletionRecordV3) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(COMPLETION_ID_DOMAIN);
    for digest in [
        record.replay_id,
        record.publication_id,
        record.ordered_witness_digest,
        record.ordered_candidate_digest,
        record.ordered_decision_digest,
        record.ordered_money_digest,
    ] {
        hasher.update(&digest);
    }
    for selection_id in record.selection_ids {
        hasher.update(&selection_id);
    }
    for value in [
        record.witness_first,
        record.witness_count,
        record.candidate_first,
        record.candidate_count,
        record.decision_first,
        record.decision_count,
        record.money_first,
        record.money_count,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    put_counters(&mut hasher, record.counters);
    hasher.update(&record.pricing_refused_candidates.to_le_bytes());
    hasher.update(&record.admitted_pricing_refused.to_le_bytes());
    hasher.update(&record.block_sequence.to_le_bytes());
    hasher.finalize()
}

fn put_candidate_fields(
    hasher: &mut Hasher,
    candidate: AuthenticatedReplayCandidateV3,
) -> Result<(), String> {
    for value in [
        candidate.signal_bar,
        candidate.entry_bar,
        candidate.occupied_through_bar,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    for value in [
        candidate.signal_micros,
        candidate.entry_micros,
        candidate.occupied_through_micros,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    hasher.update(&[path_byte(candidate.path)]);
    match candidate.price {
        Some(row) => {
            hasher.update(&[1]);
            put_trade_row(hasher, row)?;
        }
        None => hasher.update(&[0]),
    }
    hasher.update(&candidate.ambiguous_bars.to_le_bytes());
    hasher.update(&candidate.gap_fills.to_le_bytes());
    Ok(())
}

fn put_trade_row(hasher: &mut Hasher, row: TradeRow) -> Result<(), String> {
    validate_trade_row(row)?;
    for value in [row.signal_bar, row.entry_bar, row.exit_bar] {
        hasher.update(&usize_to_u64(value, "trade-row bar")?.to_le_bytes());
    }
    for value in [
        row.best,
        row.worst,
        row.entry_micros,
        row.exit_micros,
        row.adverse,
        row.adverse_paisa,
        row.favourable,
        row.favourable_paisa,
    ] {
        hasher.update(&value.to_le_bytes());
    }
    Ok(())
}

fn put_vix_stamp(hasher: &mut Hasher, stamp: VixStamp) {
    match stamp {
        VixStamp::Absent => hasher.update(&[0]),
        VixStamp::Exact(candle) => {
            hasher.update(&[1]);
            for value in [
                candle.ts_micros,
                candle.open,
                candle.high,
                candle.low,
                candle.close,
                candle.volume,
                candle.open_interest,
            ] {
                hasher.update(&value.to_le_bytes());
            }
        }
    }
}

fn put_disposition(hasher: &mut Hasher, disposition: DecisionDispositionV3) {
    match disposition {
        DecisionDispositionV3::Admitted {
            occupied_through_micros,
        } => {
            hasher.update(&[1]);
            hasher.update(&occupied_through_micros.to_le_bytes());
        }
        DecisionDispositionV3::BlockedOccupied {
            occupied_through_micros,
        } => {
            hasher.update(&[2]);
            hasher.update(&occupied_through_micros.to_le_bytes());
        }
        DecisionDispositionV3::BlockedSimultaneous { admitted } => {
            hasher.update(&[3]);
            hasher.update(&admitted);
        }
        DecisionDispositionV3::Unreachable => hasher.update(&[4]),
        DecisionDispositionV3::Refused => hasher.update(&[5]),
    }
}

fn put_counters(hasher: &mut Hasher, counters: Counters) {
    for value in [
        counters.offered,
        counters.admitted,
        counters.blocked_occupied,
        counters.blocked_simultaneous,
        counters.unreachable,
        counters.refused,
    ] {
        hasher.update(&value.to_le_bytes());
    }
}

fn feed_digest(feed: Vendor) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"brutex-global-replay-v3-feed\0");
    hasher.update(feed.as_str().as_bytes());
    hasher.finalize()
}

fn validate_trade_row(row: TradeRow) -> Result<(), String> {
    if row.signal_bar >= row.entry_bar
        || row.entry_bar > row.exit_bar
        || row.entry_micros > row.exit_micros
        || row.adverse < 0
        || row.adverse_paisa < 0
        || row.favourable < 0
        || row.favourable_paisa < 0
    {
        return Err(
            "Global Replay V3 money row has impossible coordinates or excursions".to_owned(),
        );
    }
    Ok(())
}

fn validate_vix_stamp(stamp: VixStamp, expected: i64, leg: &str) -> Result<(), String> {
    let VixStamp::Exact(candle) = stamp else {
        return Ok(());
    };
    if candle.ts_micros != expected
        || candle.open <= 0
        || candle.high < candle.open.max(candle.close)
        || candle.low > candle.open.min(candle.close)
        || candle.low <= 0
        || candle.volume < 0
        || (candle.open_interest < 0 && candle.open_interest != i64::MIN)
    {
        return Err(format!(
            "Global Replay V3 {leg} India VIX stamp is not exact sane stored OHLCV/OI evidence"
        ));
    }
    Ok(())
}

fn instrument_of(family: InstrumentFamilyV1) -> Result<InstrumentKey, String> {
    InstrumentKey::index(
        Exchange::Nse,
        match family {
            InstrumentFamilyV1::Nifty => "NIFTY",
            InstrumentFamilyV1::BankNifty => "BANKNIFTY",
        },
    )
    .map_err(|why| format!("Global Replay V3 canonical swept instrument is invalid: {why}"))
}

fn require_digest(name: &str, digest: &[u8; 32]) -> Result<(), String> {
    if digest == &[0; 32] {
        Err(format!("Global Replay V3 {name} digest is all zero"))
    } else {
        Ok(())
    }
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

const fn direction_byte(direction: Direction) -> u8 {
    match direction {
        Direction::Long => 1,
        Direction::Short => 2,
    }
}

const fn path_byte(path: CandidatePathV3) -> u8 {
    path as u8
}

fn usize_to_u64(value: usize, subject: &str) -> Result<u64, String> {
    u64::try_from(value).map_err(|_| format!("Global Replay V3 {subject} does not fit u64"))
}

fn u64_to_usize(value: u64, subject: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("Global Replay V3 {subject} does not fit usize"))
}

fn hex(digest: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for byte in digest {
        use core::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

impl WitnessRecordV3 {
    fn encode(self) -> Result<[u8; GLOBAL_REPLAY_V3_WITNESS_BYTES], String> {
        self.validate()?;
        let mut raw = [0_u8; GLOBAL_REPLAY_V3_WITNESS_BYTES];
        let payload = raw
            .get_mut(..GLOBAL_REPLAY_V3_WITNESS_BYTES - SEAL_BYTES)
            .ok_or_else(|| "Global Replay V3 witness payload bound is invalid".to_owned())?;
        let mut encoder = Encoder::new(payload);
        encoder.bytes(&WITNESS_MAGIC)?;
        encoder.u32(VERSION)?;
        for digest in [
            self.witness_id,
            self.selection_id,
            self.row_id,
            self.selected_exit_digest,
            self.universe_digest,
            self.run_id,
            self.feed_digest,
            self.strategy_digest,
            self.ordered_candidate_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        for word in self.mask_words {
            encoder.u64(word)?;
        }
        encoder.u64(self.candidate_first)?;
        encoder.u64(self.candidate_count)?;
        encoder.u64(self.pricing_refused_count)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u16(self.rank)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(direction_byte(self.direction))?;
        encoder.u8(vendor_byte(self.feed)?)?;
        encoder.finish_padded()?;
        seal_record(WITNESS_SEAL_DOMAIN, &mut raw)?;
        Ok(raw)
    }

    fn decode(raw: &[u8; GLOBAL_REPLAY_V3_WITNESS_BYTES]) -> Result<Self, String> {
        let payload = verified_payload(WITNESS_SEAL_DOMAIN, raw)?;
        let mut decoder = Decoder::new(payload);
        decoder.expect_bytes(&WITNESS_MAGIC, "witness magic")?;
        decoder.expect_u32(VERSION, "witness version")?;
        let value = Self {
            witness_id: decoder.array_32()?,
            selection_id: decoder.array_32()?,
            row_id: decoder.array_32()?,
            selected_exit_digest: decoder.array_32()?,
            universe_digest: decoder.array_32()?,
            run_id: decoder.array_32()?,
            feed_digest: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            ordered_candidate_digest: decoder.array_32()?,
            mask_words: [
                decoder.u64()?,
                decoder.u64()?,
                decoder.u64()?,
                decoder.u64()?,
                decoder.u64()?,
                decoder.u64()?,
            ],
            candidate_first: decoder.u64()?,
            candidate_count: decoder.u64()?,
            pricing_refused_count: decoder.u64()?,
            rung_seconds: decoder.u32()?,
            rank: decoder.u16()?,
            family: decode_family(decoder.u8()?)?,
            direction: decode_direction(decoder.u8()?)?,
            feed: decode_vendor(decoder.u8()?)?,
        };
        decoder.finish_zeros()?;
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), String> {
        for (name, digest) in [
            ("witness", &self.witness_id),
            ("selection", &self.selection_id),
            ("row", &self.row_id),
            ("selected exit", &self.selected_exit_digest),
            ("universe", &self.universe_digest),
            ("run", &self.run_id),
            ("feed", &self.feed_digest),
            ("strategy", &self.strategy_digest),
            ("candidate order", &self.ordered_candidate_digest),
        ] {
            require_digest(name, digest)?;
        }
        validate_rung_rank(self.rung_seconds, self.rank)?;
        if self.mask_words.iter().all(|word| *word == 0)
            || self.pricing_refused_count > self.candidate_count
            || self.feed_digest != feed_digest(self.feed)
            || self.witness_id != witness_id_from_record(&self)
        {
            return Err("Global Replay V3 witness record fails canonical validation".to_owned());
        }
        Ok(())
    }
}

impl CandidateRecordV3 {
    fn encode(self) -> Result<[u8; GLOBAL_REPLAY_V3_CANDIDATE_BYTES], String> {
        self.validate()?;
        let mut raw = [0_u8; GLOBAL_REPLAY_V3_CANDIDATE_BYTES];
        let payload = raw
            .get_mut(..GLOBAL_REPLAY_V3_CANDIDATE_BYTES - SEAL_BYTES)
            .ok_or_else(|| "Global Replay V3 candidate payload bound is invalid".to_owned())?;
        let mut encoder = Encoder::new(payload);
        encoder.bytes(&CANDIDATE_MAGIC)?;
        encoder.u32(VERSION)?;
        for digest in [
            self.candidate_id,
            self.witness_id,
            self.selection_id,
            self.row_id,
            self.strategy_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u16(self.stream_ordinal)?;
        encoder.u64(self.candidate_ordinal)?;
        encoder.u64(self.signal_bar)?;
        encoder.u64(self.entry_bar)?;
        encoder.u64(self.occupied_through_bar)?;
        encoder.i64(self.signal_micros)?;
        encoder.i64(self.entry_micros)?;
        encoder.i64(self.occupied_through_micros)?;
        encoder.u8(path_byte(self.path))?;
        encode_optional_trade_row(&mut encoder, self.price)?;
        encoder.u64(self.ambiguous_bars)?;
        encoder.u64(self.gap_fills)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u16(self.rank)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(direction_byte(self.direction))?;
        encoder.u8(vendor_byte(self.feed)?)?;
        encoder.finish_padded()?;
        seal_record(CANDIDATE_SEAL_DOMAIN, &mut raw)?;
        Ok(raw)
    }

    fn decode(raw: &[u8; GLOBAL_REPLAY_V3_CANDIDATE_BYTES]) -> Result<Self, String> {
        let payload = verified_payload(CANDIDATE_SEAL_DOMAIN, raw)?;
        let mut decoder = Decoder::new(payload);
        decoder.expect_bytes(&CANDIDATE_MAGIC, "candidate magic")?;
        decoder.expect_u32(VERSION, "candidate version")?;
        let value = Self {
            candidate_id: decoder.array_32()?,
            witness_id: decoder.array_32()?,
            selection_id: decoder.array_32()?,
            row_id: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            stream_ordinal: decoder.u16()?,
            candidate_ordinal: decoder.u64()?,
            signal_bar: decoder.u64()?,
            entry_bar: decoder.u64()?,
            occupied_through_bar: decoder.u64()?,
            signal_micros: decoder.i64()?,
            entry_micros: decoder.i64()?,
            occupied_through_micros: decoder.i64()?,
            path: decode_path(decoder.u8()?)?,
            price: decode_optional_trade_row(&mut decoder)?,
            ambiguous_bars: decoder.u64()?,
            gap_fills: decoder.u64()?,
            rung_seconds: decoder.u32()?,
            rank: decoder.u16()?,
            family: decode_family(decoder.u8()?)?,
            direction: decode_direction(decoder.u8()?)?,
            feed: decode_vendor(decoder.u8()?)?,
        };
        decoder.finish_zeros()?;
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), String> {
        for (name, digest) in [
            ("candidate", &self.candidate_id),
            ("witness", &self.witness_id),
            ("selection", &self.selection_id),
            ("row", &self.row_id),
            ("strategy", &self.strategy_digest),
        ] {
            require_digest(name, digest)?;
        }
        validate_rung_rank(self.rung_seconds, self.rank)?;
        if usize::from(self.stream_ordinal) >= WITNESS_COUNT {
            return Err("Global Replay V3 candidate stream ordinal is outside 200".to_owned());
        }
        let candidate = self.authenticated_candidate();
        candidate.validate()?;
        if self.candidate_id != candidate_id(self.witness_id, self.candidate_ordinal, candidate)? {
            return Err(
                "Global Replay V3 candidate identity differs from its exact fields".to_owned(),
            );
        }
        Ok(())
    }

    const fn authenticated_candidate(self) -> AuthenticatedReplayCandidateV3 {
        AuthenticatedReplayCandidateV3 {
            signal_bar: self.signal_bar,
            entry_bar: self.entry_bar,
            occupied_through_bar: self.occupied_through_bar,
            signal_micros: self.signal_micros,
            entry_micros: self.entry_micros,
            occupied_through_micros: self.occupied_through_micros,
            path: self.path,
            price: self.price,
            ambiguous_bars: self.ambiguous_bars,
            gap_fills: self.gap_fills,
        }
    }
}

impl DecisionRecordV3 {
    fn encode(self) -> Result<[u8; GLOBAL_REPLAY_V3_DECISION_BYTES], String> {
        self.validate()?;
        let mut raw = [0_u8; GLOBAL_REPLAY_V3_DECISION_BYTES];
        let payload = raw
            .get_mut(..GLOBAL_REPLAY_V3_DECISION_BYTES - SEAL_BYTES)
            .ok_or_else(|| "Global Replay V3 decision payload bound is invalid".to_owned())?;
        let mut encoder = Encoder::new(payload);
        encoder.bytes(&DECISION_MAGIC)?;
        encoder.u32(VERSION)?;
        for digest in [
            self.decision_id,
            self.candidate_id,
            self.witness_id,
            self.strategy_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u64(self.sequence)?;
        encoder.u16(self.stream_ordinal)?;
        encoder.u64(self.candidate_ordinal)?;
        encoder.i64(self.entry_micros)?;
        encoder.i64(self.occupied_through_micros)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u16(self.rank)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(direction_byte(self.direction))?;
        encoder.u8(path_byte(self.path))?;
        encode_disposition(&mut encoder, self.disposition)?;
        encoder.finish_padded()?;
        seal_record(DECISION_SEAL_DOMAIN, &mut raw)?;
        Ok(raw)
    }

    fn decode(raw: &[u8; GLOBAL_REPLAY_V3_DECISION_BYTES]) -> Result<Self, String> {
        let payload = verified_payload(DECISION_SEAL_DOMAIN, raw)?;
        let mut decoder = Decoder::new(payload);
        decoder.expect_bytes(&DECISION_MAGIC, "decision magic")?;
        decoder.expect_u32(VERSION, "decision version")?;
        let value = Self {
            decision_id: decoder.array_32()?,
            candidate_id: decoder.array_32()?,
            witness_id: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            sequence: decoder.u64()?,
            stream_ordinal: decoder.u16()?,
            candidate_ordinal: decoder.u64()?,
            entry_micros: decoder.i64()?,
            occupied_through_micros: decoder.i64()?,
            rung_seconds: decoder.u32()?,
            rank: decoder.u16()?,
            family: decode_family(decoder.u8()?)?,
            direction: decode_direction(decoder.u8()?)?,
            path: decode_path(decoder.u8()?)?,
            disposition: decode_disposition(&mut decoder)?,
        };
        decoder.finish_zeros()?;
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), String> {
        for (name, digest) in [
            ("decision", &self.decision_id),
            ("candidate", &self.candidate_id),
            ("witness", &self.witness_id),
            ("strategy", &self.strategy_digest),
        ] {
            require_digest(name, digest)?;
        }
        validate_rung_rank(self.rung_seconds, self.rank)?;
        if usize::from(self.stream_ordinal) >= WITNESS_COUNT
            || self.entry_micros.rem_euclid(MICROS_PER_MINUTE) != 0
            || self.occupied_through_micros < self.entry_micros
        {
            return Err("Global Replay V3 decision has impossible coordinates".to_owned());
        }
        match self.disposition {
            DecisionDispositionV3::Admitted {
                occupied_through_micros,
            } if occupied_through_micros == self.occupied_through_micros => {}
            DecisionDispositionV3::BlockedOccupied { .. } => {}
            DecisionDispositionV3::BlockedSimultaneous { admitted } if admitted != [0; 32] => {}
            DecisionDispositionV3::Unreachable | DecisionDispositionV3::Refused => {
                return Err(
                    "Global Replay V3 persisted a non-reachable terminal bucket for an authenticated reachable candidate"
                        .to_owned(),
                );
            }
            _ => {
                return Err(
                    "Global Replay V3 decision disposition contradicts its candidate".to_owned(),
                );
            }
        }
        Ok(())
    }
}

impl MoneyRecordV3 {
    fn encode(self) -> Result<[u8; GLOBAL_REPLAY_V3_MONEY_BYTES], String> {
        self.validate()?;
        let mut raw = [0_u8; GLOBAL_REPLAY_V3_MONEY_BYTES];
        let payload = raw
            .get_mut(..GLOBAL_REPLAY_V3_MONEY_BYTES - SEAL_BYTES)
            .ok_or_else(|| "Global Replay V3 money payload bound is invalid".to_owned())?;
        let mut encoder = Encoder::new(payload);
        encoder.bytes(&MONEY_MAGIC)?;
        encoder.u32(VERSION)?;
        for digest in [
            self.money_id,
            self.decision_id,
            self.candidate_id,
            self.witness_id,
            self.strategy_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        encoder.u64(self.decision_sequence)?;
        encoder.u16(self.stream_ordinal)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u16(self.rank)?;
        encoder.u8(family_byte(self.family))?;
        encoder.u8(direction_byte(self.direction))?;
        encoder.u8(vendor_byte(self.feed)?)?;
        encode_trade_row(&mut encoder, self.row)?;
        encode_vix_stamp(&mut encoder, self.vix.entry)?;
        encode_vix_stamp(&mut encoder, self.vix.exit)?;
        encoder.finish_padded()?;
        seal_record(MONEY_SEAL_DOMAIN, &mut raw)?;
        Ok(raw)
    }

    fn decode(raw: &[u8; GLOBAL_REPLAY_V3_MONEY_BYTES]) -> Result<Self, String> {
        let payload = verified_payload(MONEY_SEAL_DOMAIN, raw)?;
        let mut decoder = Decoder::new(payload);
        decoder.expect_bytes(&MONEY_MAGIC, "money magic")?;
        decoder.expect_u32(VERSION, "money version")?;
        let value = Self {
            money_id: decoder.array_32()?,
            decision_id: decoder.array_32()?,
            candidate_id: decoder.array_32()?,
            witness_id: decoder.array_32()?,
            strategy_digest: decoder.array_32()?,
            decision_sequence: decoder.u64()?,
            stream_ordinal: decoder.u16()?,
            rung_seconds: decoder.u32()?,
            rank: decoder.u16()?,
            family: decode_family(decoder.u8()?)?,
            direction: decode_direction(decoder.u8()?)?,
            feed: decode_vendor(decoder.u8()?)?,
            row: decode_trade_row(&mut decoder)?,
            vix: VixPairV3 {
                entry: decode_vix_stamp(&mut decoder)?,
                exit: decode_vix_stamp(&mut decoder)?,
            },
        };
        decoder.finish_zeros()?;
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), String> {
        for (name, digest) in [
            ("money", &self.money_id),
            ("decision", &self.decision_id),
            ("candidate", &self.candidate_id),
            ("witness", &self.witness_id),
            ("strategy", &self.strategy_digest),
        ] {
            require_digest(name, digest)?;
        }
        validate_rung_rank(self.rung_seconds, self.rank)?;
        validate_trade_row(self.row)?;
        validate_vix_stamp(self.vix.entry, self.row.entry_micros, "entry")?;
        validate_vix_stamp(self.vix.exit, self.row.exit_micros, "exit")?;
        if usize::from(self.stream_ordinal) >= WITNESS_COUNT
            || self.money_id != money_id(self.decision_id, self.row, self.vix)?
        {
            return Err("Global Replay V3 money identity differs from exact money/VIX".to_owned());
        }
        Ok(())
    }
}

impl CompletionRecordV3 {
    fn encode(self) -> Result<[u8; GLOBAL_REPLAY_V3_COMPLETION_BYTES], String> {
        self.validate()?;
        let mut raw = [0_u8; GLOBAL_REPLAY_V3_COMPLETION_BYTES];
        let payload = raw
            .get_mut(..GLOBAL_REPLAY_V3_COMPLETION_BYTES - SEAL_BYTES)
            .ok_or_else(|| "Global Replay V3 completion payload bound is invalid".to_owned())?;
        let mut encoder = Encoder::new(payload);
        encoder.bytes(&COMPLETION_MAGIC)?;
        encoder.u32(VERSION)?;
        for digest in [self.completion_id, self.replay_id, self.publication_id] {
            encoder.bytes(&digest)?;
        }
        for selection_id in self.selection_ids {
            encoder.bytes(&selection_id)?;
        }
        for digest in [
            self.ordered_witness_digest,
            self.ordered_candidate_digest,
            self.ordered_decision_digest,
            self.ordered_money_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        for value in [
            self.witness_first,
            self.witness_count,
            self.candidate_first,
            self.candidate_count,
            self.decision_first,
            self.decision_count,
            self.money_first,
            self.money_count,
        ] {
            encoder.u64(value)?;
        }
        encode_counters(&mut encoder, self.counters)?;
        encoder.u64(self.pricing_refused_candidates)?;
        encoder.u64(self.admitted_pricing_refused)?;
        encoder.u64(self.block_sequence)?;
        encoder.finish_padded()?;
        seal_record(COMPLETION_SEAL_DOMAIN, &mut raw)?;
        Ok(raw)
    }

    fn decode(raw: &[u8; GLOBAL_REPLAY_V3_COMPLETION_BYTES]) -> Result<Self, String> {
        let payload = verified_payload(COMPLETION_SEAL_DOMAIN, raw)?;
        let mut decoder = Decoder::new(payload);
        decoder.expect_bytes(&COMPLETION_MAGIC, "completion magic")?;
        decoder.expect_u32(VERSION, "completion version")?;
        let value = Self {
            completion_id: decoder.array_32()?,
            replay_id: decoder.array_32()?,
            publication_id: decoder.array_32()?,
            selection_ids: [
                decoder.array_32()?,
                decoder.array_32()?,
                decoder.array_32()?,
                decoder.array_32()?,
                decoder.array_32()?,
                decoder.array_32()?,
                decoder.array_32()?,
                decoder.array_32()?,
            ],
            ordered_witness_digest: decoder.array_32()?,
            ordered_candidate_digest: decoder.array_32()?,
            ordered_decision_digest: decoder.array_32()?,
            ordered_money_digest: decoder.array_32()?,
            witness_first: decoder.u64()?,
            witness_count: decoder.u64()?,
            candidate_first: decoder.u64()?,
            candidate_count: decoder.u64()?,
            decision_first: decoder.u64()?,
            decision_count: decoder.u64()?,
            money_first: decoder.u64()?,
            money_count: decoder.u64()?,
            counters: decode_counters(&mut decoder)?,
            pricing_refused_candidates: decoder.u64()?,
            admitted_pricing_refused: decoder.u64()?,
            block_sequence: decoder.u64()?,
        };
        decoder.finish_zeros()?;
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), String> {
        for (name, digest) in [
            ("completion", &self.completion_id),
            ("replay", &self.replay_id),
            ("publication", &self.publication_id),
            ("witness order", &self.ordered_witness_digest),
            ("candidate order", &self.ordered_candidate_digest),
            ("decision order", &self.ordered_decision_digest),
            ("money order", &self.ordered_money_digest),
        ] {
            require_digest(name, digest)?;
        }
        let mut unique = HashSet::new();
        for selection_id in self.selection_ids {
            require_digest("rung selection", &selection_id)?;
            if !unique.insert(selection_id) {
                return Err(
                    "Global Replay V3 completion repeats one selection across rungs".to_owned(),
                );
            }
        }
        if self.witness_count != WITNESS_COUNT as u64
            || self.decision_count != self.candidate_count
            || self.counters.offered != self.candidate_count
            || !self.counters.reconciles()
            || self.pricing_refused_candidates > self.candidate_count
            || self.admitted_pricing_refused > self.counters.admitted
            || self.money_count
                != self
                    .counters
                    .admitted
                    .checked_sub(self.admitted_pricing_refused)
                    .ok_or_else(|| {
                        "Global Replay V3 completion admitted counts underflow".to_owned()
                    })?
            || self.publication_id != publication_id(self.replay_id, self.ordered_money_digest)
            || self.completion_id != completion_id(&self)
        {
            return Err("Global Replay V3 completion fails receipt reconciliation".to_owned());
        }
        Ok(())
    }
}

fn witness_id_from_record(record: &WitnessRecordV3) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(WITNESS_ID_DOMAIN);
    for digest in [
        record.selection_id,
        record.row_id,
        record.selected_exit_digest,
        record.universe_digest,
        record.run_id,
        record.feed_digest,
        record.strategy_digest,
    ] {
        hasher.update(&digest);
    }
    for word in record.mask_words {
        hasher.update(&word.to_le_bytes());
    }
    hasher.update(&record.rung_seconds.to_le_bytes());
    hasher.update(&record.rank.to_le_bytes());
    hasher.update(&[family_byte(record.family), direction_byte(record.direction)]);
    hasher.finalize()
}

fn validate_rung_rank(rung_seconds: u32, rank: u16) -> Result<(), String> {
    if !CANONICAL_RUNGS_SECONDS.contains(&rung_seconds) {
        return Err(format!(
            "Global Replay V3 rung {rung_seconds}s is not one of the eight canonical rungs"
        ));
    }
    if rank == 0 || usize::from(rank) > TOP_PER_RUNG {
        return Err(format!(
            "Global Replay V3 rank {rank} is outside canonical Top-{TOP_PER_RUNG}"
        ));
    }
    Ok(())
}

fn vendor_byte(vendor: Vendor) -> Result<u8, String> {
    match vendor.as_str() {
        "groww" => Ok(1),
        "dhan" => Ok(2),
        "truedata" => Ok(3),
        "gdfl" => Ok(4),
        "zerodha" => Ok(5),
        other => Err(format!(
            "Global Replay V3 vendor {other} has no version-three codec tag"
        )),
    }
}

fn decode_vendor(byte: u8) -> Result<Vendor, String> {
    match byte {
        1 => Ok(Vendor::Groww),
        2 => Ok(Vendor::Dhan),
        3 => Ok(Vendor::TrueData),
        4 => Ok(Vendor::Gdfl),
        5 => Ok(Vendor::Zerodha),
        _ => Err(format!(
            "Global Replay V3 vendor codec tag {byte} is unknown"
        )),
    }
}

fn decode_family(byte: u8) -> Result<InstrumentFamilyV1, String> {
    match byte {
        1 => Ok(InstrumentFamilyV1::Nifty),
        2 => Ok(InstrumentFamilyV1::BankNifty),
        _ => Err(format!(
            "Global Replay V3 instrument-family tag {byte} is unknown"
        )),
    }
}

fn decode_direction(byte: u8) -> Result<Direction, String> {
    match byte {
        1 => Ok(Direction::Long),
        2 => Ok(Direction::Short),
        _ => Err(format!("Global Replay V3 direction tag {byte} is unknown")),
    }
}

fn decode_path(byte: u8) -> Result<CandidatePathV3, String> {
    match byte {
        1 => Ok(CandidatePathV3::Priceable),
        2 => Ok(CandidatePathV3::BlockOnly),
        3 => Ok(CandidatePathV3::CrossingRefused),
        4 => Ok(CandidatePathV3::BlockOnlyAndCrossingRefused),
        _ => Err(format!(
            "Global Replay V3 candidate-path tag {byte} is unknown"
        )),
    }
}

fn encode_optional_trade_row(
    encoder: &mut Encoder<'_>,
    row: Option<TradeRow>,
) -> Result<(), String> {
    if let Some(row) = row {
        encoder.u8(1)?;
        encode_trade_row(encoder, row)
    } else {
        encoder.u8(0)?;
        encoder.zeros(88)
    }
}

fn decode_optional_trade_row(decoder: &mut Decoder<'_>) -> Result<Option<TradeRow>, String> {
    match decoder.u8()? {
        0 => {
            decoder.zeros(88, "absent trade-row payload")?;
            Ok(None)
        }
        1 => decode_trade_row(decoder).map(Some),
        tag => Err(format!(
            "Global Replay V3 optional trade-row tag {tag} is unknown"
        )),
    }
}

fn encode_trade_row(encoder: &mut Encoder<'_>, row: TradeRow) -> Result<(), String> {
    validate_trade_row(row)?;
    encoder.u64(usize_to_u64(row.signal_bar, "trade signal bar")?)?;
    encoder.u64(usize_to_u64(row.entry_bar, "trade entry bar")?)?;
    encoder.u64(usize_to_u64(row.exit_bar, "trade exit bar")?)?;
    for value in [
        row.best,
        row.worst,
        row.entry_micros,
        row.exit_micros,
        row.adverse,
        row.adverse_paisa,
        row.favourable,
        row.favourable_paisa,
    ] {
        encoder.i64(value)?;
    }
    Ok(())
}

fn decode_trade_row(decoder: &mut Decoder<'_>) -> Result<TradeRow, String> {
    let row = TradeRow {
        signal_bar: u64_to_usize(decoder.u64()?, "trade signal bar")?,
        entry_bar: u64_to_usize(decoder.u64()?, "trade entry bar")?,
        exit_bar: u64_to_usize(decoder.u64()?, "trade exit bar")?,
        best: decoder.i64()?,
        worst: decoder.i64()?,
        entry_micros: decoder.i64()?,
        exit_micros: decoder.i64()?,
        adverse: decoder.i64()?,
        adverse_paisa: decoder.i64()?,
        favourable: decoder.i64()?,
        favourable_paisa: decoder.i64()?,
    };
    validate_trade_row(row)?;
    Ok(row)
}

fn encode_vix_stamp(encoder: &mut Encoder<'_>, stamp: VixStamp) -> Result<(), String> {
    match stamp {
        VixStamp::Absent => {
            encoder.u8(0)?;
            encoder.zeros(63)
        }
        VixStamp::Exact(candle) => {
            encoder.u8(1)?;
            encoder.zeros(7)?;
            for value in [
                candle.ts_micros,
                candle.open,
                candle.high,
                candle.low,
                candle.close,
                candle.volume,
                candle.open_interest,
            ] {
                encoder.i64(value)?;
            }
            Ok(())
        }
    }
}

fn decode_vix_stamp(decoder: &mut Decoder<'_>) -> Result<VixStamp, String> {
    match decoder.u8()? {
        0 => {
            decoder.zeros(63, "absent India VIX payload")?;
            Ok(VixStamp::Absent)
        }
        1 => {
            decoder.zeros(7, "exact India VIX reserve")?;
            Ok(VixStamp::Exact(Candle {
                ts_micros: decoder.i64()?,
                open: decoder.i64()?,
                high: decoder.i64()?,
                low: decoder.i64()?,
                close: decoder.i64()?,
                volume: decoder.i64()?,
                open_interest: decoder.i64()?,
            }))
        }
        tag => Err(format!(
            "Global Replay V3 India VIX stamp tag {tag} is unknown"
        )),
    }
}

fn encode_disposition(
    encoder: &mut Encoder<'_>,
    disposition: DecisionDispositionV3,
) -> Result<(), String> {
    match disposition {
        DecisionDispositionV3::Admitted {
            occupied_through_micros,
        } => {
            encoder.u8(1)?;
            encoder.i64(occupied_through_micros)?;
            encoder.zeros(32)
        }
        DecisionDispositionV3::BlockedOccupied {
            occupied_through_micros,
        } => {
            encoder.u8(2)?;
            encoder.i64(occupied_through_micros)?;
            encoder.zeros(32)
        }
        DecisionDispositionV3::BlockedSimultaneous { admitted } => {
            encoder.u8(3)?;
            encoder.i64(0)?;
            encoder.bytes(&admitted)
        }
        DecisionDispositionV3::Unreachable => {
            encoder.u8(4)?;
            encoder.i64(0)?;
            encoder.zeros(32)
        }
        DecisionDispositionV3::Refused => {
            encoder.u8(5)?;
            encoder.i64(0)?;
            encoder.zeros(32)
        }
    }
}

fn decode_disposition(decoder: &mut Decoder<'_>) -> Result<DecisionDispositionV3, String> {
    let tag = decoder.u8()?;
    let through = decoder.i64()?;
    let admitted = decoder.array_32()?;
    match tag {
        1 if admitted == [0; 32] => Ok(DecisionDispositionV3::Admitted {
            occupied_through_micros: through,
        }),
        2 if admitted == [0; 32] => Ok(DecisionDispositionV3::BlockedOccupied {
            occupied_through_micros: through,
        }),
        3 if through == 0 && admitted != [0; 32] => {
            Ok(DecisionDispositionV3::BlockedSimultaneous { admitted })
        }
        4 if through == 0 && admitted == [0; 32] => Ok(DecisionDispositionV3::Unreachable),
        5 if through == 0 && admitted == [0; 32] => Ok(DecisionDispositionV3::Refused),
        _ => Err("Global Replay V3 disposition payload contradicts its tag".to_owned()),
    }
}

fn encode_counters(encoder: &mut Encoder<'_>, counters: Counters) -> Result<(), String> {
    for value in [
        counters.offered,
        counters.admitted,
        counters.blocked_occupied,
        counters.blocked_simultaneous,
        counters.unreachable,
        counters.refused,
    ] {
        encoder.u64(value)?;
    }
    Ok(())
}

fn decode_counters(decoder: &mut Decoder<'_>) -> Result<Counters, String> {
    Ok(Counters {
        offered: decoder.u64()?,
        admitted: decoder.u64()?,
        blocked_occupied: decoder.u64()?,
        blocked_simultaneous: decoder.u64()?,
        unreachable: decoder.u64()?,
        refused: decoder.u64()?,
    })
}

fn seal_record<const N: usize>(domain: &[u8], raw: &mut [u8; N]) -> Result<(), String> {
    let payload_len = N
        .checked_sub(SEAL_BYTES)
        .ok_or_else(|| "Global Replay V3 record is shorter than its seal".to_owned())?;
    let (payload, seal) = raw.split_at_mut(payload_len);
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(payload);
    seal.copy_from_slice(&hasher.finalize());
    Ok(())
}

fn verified_payload<'a, const N: usize>(
    domain: &[u8],
    raw: &'a [u8; N],
) -> Result<&'a [u8], String> {
    let payload_len = N
        .checked_sub(SEAL_BYTES)
        .ok_or_else(|| "Global Replay V3 record is shorter than its seal".to_owned())?;
    let (payload, seal) = raw.split_at(payload_len);
    let mut hasher = Hasher::new();
    hasher.update(domain);
    hasher.update(payload);
    if seal != hasher.finalize() {
        return Err("Global Replay V3 record seal is corrupt".to_owned());
    }
    Ok(payload)
}

struct Encoder<'a> {
    bytes: &'a mut [u8],
    cursor: usize,
}

impl<'a> Encoder<'a> {
    const fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn bytes(&mut self, value: &[u8]) -> Result<(), String> {
        let end = self
            .cursor
            .checked_add(value.len())
            .ok_or_else(|| "Global Replay V3 encoder cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Global Replay V3 encoder exceeded fixed record".to_owned())?
            .copy_from_slice(value);
        self.cursor = end;
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), String> {
        self.bytes(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), String> {
        self.bytes(&value.to_le_bytes())
    }

    fn zeros(&mut self, count: usize) -> Result<(), String> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Global Replay V3 encoder zero cursor overflowed".to_owned())?;
        self.bytes
            .get_mut(self.cursor..end)
            .ok_or_else(|| "Global Replay V3 encoder zeros exceeded fixed record".to_owned())?
            .fill(0);
        self.cursor = end;
        Ok(())
    }

    fn finish_padded(self) -> Result<(), String> {
        self.bytes
            .get_mut(self.cursor..)
            .ok_or_else(|| "Global Replay V3 encoder padding exceeded fixed record".to_owned())?
            .fill(0);
        Ok(())
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .cursor
            .checked_add(count)
            .ok_or_else(|| "Global Replay V3 decoder cursor overflowed".to_owned())?;
        let value = self
            .bytes
            .get(self.cursor..end)
            .ok_or_else(|| "Global Replay V3 decoder exceeded fixed record".to_owned())?;
        self.cursor = end;
        Ok(value)
    }

    fn expect_bytes(&mut self, expected: &[u8], subject: &str) -> Result<(), String> {
        if self.take(expected.len())? == expected {
            Ok(())
        } else {
            Err(format!("Global Replay V3 {subject} differs"))
        }
    }

    fn expect_u32(&mut self, expected: u32, subject: &str) -> Result<(), String> {
        let actual = self.u32()?;
        if actual == expected {
            Ok(())
        } else {
            Err(format!(
                "Global Replay V3 {subject} {actual} differs from {expected}"
            ))
        }
    }

    fn array_32(&mut self) -> Result<[u8; 32], String> {
        self.take(32)?
            .try_into()
            .map_err(|_| "Global Replay V3 digest field differs".to_owned())
    }

    fn u8(&mut self) -> Result<u8, String> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| "Global Replay V3 u8 field differs".to_owned())
    }

    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().map_err(
            |_| "Global Replay V3 u16 field differs".to_owned(),
        )?))
    }

    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().map_err(
            |_| "Global Replay V3 u32 field differs".to_owned(),
        )?))
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| "Global Replay V3 u64 field differs".to_owned(),
        )?))
    }

    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().map_err(
            |_| "Global Replay V3 i64 field differs".to_owned(),
        )?))
    }

    fn zeros(&mut self, count: usize, subject: &str) -> Result<(), String> {
        if self.take(count)?.iter().any(|byte| *byte != 0) {
            Err(format!("Global Replay V3 {subject} is nonzero"))
        } else {
            Ok(())
        }
    }

    fn finish_zeros(mut self) -> Result<(), String> {
        let remaining = self.bytes.len().saturating_sub(self.cursor);
        self.zeros(remaining, "reserved tail")?;
        if self.cursor == self.bytes.len() {
            Ok(())
        } else {
            Err("Global Replay V3 decoder left trailing bytes".to_owned())
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlobalReplayCommitV3 {
    Written,
    Reused,
}

#[derive(Debug)]
struct GlobalReplayPathsV3 {
    witness: PathBuf,
    candidate: PathBuf,
    decision: PathBuf,
    money: PathBuf,
    completion: PathBuf,
    lock: PathBuf,
}

impl GlobalReplayPathsV3 {
    fn new(root: &Path) -> Self {
        Self {
            witness: root.join(WITNESS_FILE),
            candidate: root.join(CANDIDATE_FILE),
            decision: root.join(DECISION_FILE),
            money: root.join(MONEY_FILE),
            completion: root.join(COMPLETION_FILE),
            lock: root.join(LOCK_FILE),
        }
    }
}

/// Append-only V3 authority whose completion record is synchronized last.
#[derive(Debug)]
struct GlobalReplayLedgerV3 {
    paths: GlobalReplayPathsV3,
    bounds: GlobalReplayV3Bounds,
    writer_lock: File,
}

impl GlobalReplayLedgerV3 {
    fn open(root: &Path, bounds: GlobalReplayV3Bounds) -> Result<Self, String> {
        std::fs::create_dir_all(root).map_err(|why| {
            format!(
                "Global Replay V3 authority root {} could not be created: {why}",
                root.display()
            )
        })?;
        let paths = GlobalReplayPathsV3::new(root);
        for path in [
            &paths.witness,
            &paths.candidate,
            &paths.decision,
            &paths.money,
            &paths.completion,
        ] {
            OpenOptions::new()
                .create(true)
                .append(true)
                .read(true)
                .open(path)
                .map_err(|why| {
                    format!(
                        "Global Replay V3 authority file {} could not be opened: {why}",
                        path.display()
                    )
                })?;
        }
        let writer_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&paths.lock)
            .map_err(|why| {
                format!(
                    "Global Replay V3 writer lock {} could not be opened: {why}",
                    paths.lock.display()
                )
            })?;
        let ledger = Self {
            paths,
            bounds,
            writer_lock,
        };
        ledger.load_snapshot()?;
        Ok(ledger)
    }

    fn commit(
        &mut self,
        prepared: &PreparedGlobalReplayV3,
    ) -> Result<GlobalReplayCommitV3, String> {
        self.writer_lock
            .lock()
            .map_err(|why| format!("Global Replay V3 writer lock could not be acquired: {why}"))?;
        let result = self.commit_locked(prepared);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("Global Replay V3 writer lock could not be released: {why}"));
        match (result, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), Ok(())) | (Ok(_), Err(why)) => Err(why),
            (Err(first), Err(second)) => Err(format!("{first}; additionally, {second}")),
        }
    }

    fn commit_locked(
        &self,
        prepared: &PreparedGlobalReplayV3,
    ) -> Result<GlobalReplayCommitV3, String> {
        validate_prepared(prepared)?;
        let snapshot = self.load_snapshot()?;
        if let Some(existing) = snapshot.by_publication.get(&prepared.publication_id) {
            validate_completion_matches_prepared(existing, prepared, &snapshot)?;
            return Ok(GlobalReplayCommitV3::Reused);
        }
        let witness_first = usize_to_u64(snapshot.witnesses.len(), "witness file length")?;
        let candidate_first = usize_to_u64(snapshot.candidates.len(), "candidate file length")?;
        let decision_first = usize_to_u64(snapshot.decisions.len(), "decision file length")?;
        let money_first = usize_to_u64(snapshot.money.len(), "money file length")?;
        let block_sequence = usize_to_u64(snapshot.completions.len(), "completion sequence")?;
        require_capacity(
            "witness",
            witness_first,
            usize_to_u64(prepared.witnesses.len(), "witness append")?,
            self.bounds.witness_records,
        )?;
        require_capacity(
            "candidate",
            candidate_first,
            usize_to_u64(prepared.candidates.len(), "candidate append")?,
            self.bounds.candidate_records,
        )?;
        require_capacity(
            "decision",
            decision_first,
            usize_to_u64(prepared.decisions.len(), "decision append")?,
            self.bounds.decision_records,
        )?;
        require_capacity(
            "money",
            money_first,
            usize_to_u64(prepared.money.len(), "money append")?,
            self.bounds.money_records,
        )?;
        require_capacity(
            "completion",
            block_sequence,
            1,
            self.bounds.completion_records,
        )?;

        let mut witnesses = prepared.witnesses.clone();
        for witness in &mut witnesses {
            witness.candidate_first = witness
                .candidate_first
                .checked_add(candidate_first)
                .ok_or_else(|| {
                    "Global Replay V3 durable witness candidate offset overflowed".to_owned()
                })?;
        }
        let mut completion = CompletionRecordV3 {
            completion_id: [0; 32],
            replay_id: prepared.replay_id,
            publication_id: prepared.publication_id,
            selection_ids: prepared.selection_ids,
            ordered_witness_digest: prepared.ordered_witness_digest,
            ordered_candidate_digest: prepared.ordered_candidate_digest,
            ordered_decision_digest: prepared.ordered_decision_digest,
            ordered_money_digest: prepared.ordered_money_digest,
            witness_first,
            witness_count: usize_to_u64(witnesses.len(), "witness block")?,
            candidate_first,
            candidate_count: usize_to_u64(prepared.candidates.len(), "candidate block")?,
            decision_first,
            decision_count: usize_to_u64(prepared.decisions.len(), "decision block")?,
            money_first,
            money_count: usize_to_u64(prepared.money.len(), "money block")?,
            counters: prepared.counters,
            pricing_refused_candidates: prepared.pricing_refused_candidates,
            admitted_pricing_refused: prepared.admitted_pricing_refused,
            block_sequence,
        };
        completion.completion_id = completion_id(&completion);
        completion.validate()?;

        append_records(&self.paths.witness, &witnesses, WitnessRecordV3::encode)?;
        append_records(
            &self.paths.candidate,
            &prepared.candidates,
            CandidateRecordV3::encode,
        )?;
        append_records(
            &self.paths.decision,
            &prepared.decisions,
            DecisionRecordV3::encode,
        )?;
        append_records(&self.paths.money, &prepared.money, MoneyRecordV3::encode)?;
        append_records(
            &self.paths.completion,
            core::slice::from_ref(&completion),
            CompletionRecordV3::encode,
        )?;
        Ok(GlobalReplayCommitV3::Written)
    }

    fn load_snapshot(&self) -> Result<GlobalReplaySnapshotV3, String> {
        let witnesses = read_records(
            &self.paths.witness,
            GLOBAL_REPLAY_V3_WITNESS_BYTES,
            self.bounds.witness_records,
            WitnessRecordV3::decode,
        )?;
        let candidates = read_records(
            &self.paths.candidate,
            GLOBAL_REPLAY_V3_CANDIDATE_BYTES,
            self.bounds.candidate_records,
            CandidateRecordV3::decode,
        )?;
        let decisions = read_records(
            &self.paths.decision,
            GLOBAL_REPLAY_V3_DECISION_BYTES,
            self.bounds.decision_records,
            DecisionRecordV3::decode,
        )?;
        let money = read_records(
            &self.paths.money,
            GLOBAL_REPLAY_V3_MONEY_BYTES,
            self.bounds.money_records,
            MoneyRecordV3::decode,
        )?;
        let completions = read_records(
            &self.paths.completion,
            GLOBAL_REPLAY_V3_COMPLETION_BYTES,
            self.bounds.completion_records,
            CompletionRecordV3::decode,
        )?;
        let mut by_publication = HashMap::new();
        by_publication
            .try_reserve(completions.len())
            .map_err(|why| {
                format!("Global Replay V3 completion index allocation refused: {why}")
            })?;
        let snapshot = GlobalReplaySnapshotV3 {
            witnesses,
            candidates,
            decisions,
            money,
            completions,
            by_publication: HashMap::new(),
        };
        let mut prior_ends = [0_u64; 4];
        for (ordinal, completion) in snapshot.completions.iter().enumerate() {
            if completion.block_sequence != usize_to_u64(ordinal, "completion ordinal")? {
                return Err(format!(
                    "Global Replay V3 completion {ordinal} carries sequence {}",
                    completion.block_sequence
                ));
            }
            let starts = [
                completion.witness_first,
                completion.candidate_first,
                completion.decision_first,
                completion.money_first,
            ];
            let counts = [
                completion.witness_count,
                completion.candidate_count,
                completion.decision_count,
                completion.money_count,
            ];
            for (index, ((prior_end, start), count)) in
                prior_ends.iter_mut().zip(starts).zip(counts).enumerate()
            {
                if start < *prior_end {
                    return Err(format!(
                        "Global Replay V3 completion {ordinal} overlaps an earlier durable block in file class {index}"
                    ));
                }
                *prior_end = start.checked_add(count).ok_or_else(|| {
                    format!(
                        "Global Replay V3 completion {ordinal} range overflows in file class {index}"
                    )
                })?;
            }
            validate_committed_block(completion, &snapshot)?;
            if by_publication
                .insert(completion.publication_id, *completion)
                .is_some()
            {
                return Err(format!(
                    "Global Replay V3 publication {} has more than one completion",
                    hex(&completion.publication_id)
                ));
            }
        }
        Ok(GlobalReplaySnapshotV3 {
            by_publication,
            ..snapshot
        })
    }
}

/// Opaque live authority returned only after V3 data and Completion have been
/// committed and then semantically reopened.
pub(crate) struct CommittedStoredGlobalReplayV3 {
    ledger: GlobalReplayLedgerV3,
    publication_id: [u8; 32],
    was_written: bool,
}

impl CommittedStoredGlobalReplayV3 {
    #[must_use]
    pub(crate) const fn was_written(&self) -> bool {
        self.was_written
    }

    /// Reopens and reauthenticates the complete persisted block before
    /// releasing a fixed audit projection.
    pub(crate) fn audit(&self) -> Result<GlobalReplayV3ReopenAudit, String> {
        let snapshot = self.ledger.load_snapshot()?;
        let completion = snapshot
            .by_publication
            .get(&self.publication_id)
            .copied()
            .ok_or_else(|| {
                format!(
                    "Global Replay V3 committed publication {} disappeared on fresh reopen",
                    hex(&self.publication_id)
                )
            })?;
        validate_committed_block(&completion, &snapshot)?;
        Ok(GlobalReplayV3ReopenAudit {
            completion_id: completion.completion_id,
            replay_id: completion.replay_id,
            publication_id: completion.publication_id,
            witness_count: completion.witness_count,
            candidate_count: completion.candidate_count,
            decision_count: completion.decision_count,
            money_count: completion.money_count,
            counters: completion.counters,
            pricing_refused_candidates: completion.pricing_refused_candidates,
            admitted_pricing_refused: completion.admitted_pricing_refused,
        })
    }
}

/// Sole Step-3 production door for a paired-nonempty Global Replay V3 block.
///
/// The Selection successor and all 200 Runner capabilities are consumed by
/// value. The returned capability exists only after the five V3 ledgers have
/// committed receipt-last and a fresh semantic reopen has reproduced the same
/// publication.
pub(crate) fn commit_stored_global_replay_v3(
    root: &Path,
    bounds: GlobalReplayV3Bounds,
    selection: AllRungSelectionV5SuccessorSetV1,
    replay: Vec<GlobalReplayWitnessUniverseV1>,
    vix_months: &[VixReferenceMonth],
) -> Result<CommittedStoredGlobalReplayV3, String> {
    let vix = StoredVixCatalogV3::new(vix_months)?;
    let prepared = PreparedGlobalReplayV3::prepare_from_authorities(selection, replay, &vix)?;
    commit_prepared_global_replay_v3(root, bounds, &prepared)
}

fn commit_prepared_global_replay_v3(
    root: &Path,
    bounds: GlobalReplayV3Bounds,
    prepared: &PreparedGlobalReplayV3,
) -> Result<CommittedStoredGlobalReplayV3, String> {
    let mut ledger = GlobalReplayLedgerV3::open(root, bounds)?;
    let result = ledger.commit(prepared)?;
    let committed = CommittedStoredGlobalReplayV3 {
        ledger,
        publication_id: prepared.publication_id,
        was_written: matches!(result, GlobalReplayCommitV3::Written),
    };
    let audit = committed.audit()?;
    if audit.replay_id != prepared.replay_id
        || audit.publication_id != prepared.publication_id
        || audit.witness_count != usize_to_u64(prepared.witnesses.len(), "witness commit")?
        || audit.candidate_count != usize_to_u64(prepared.candidates.len(), "candidate commit")?
        || audit.decision_count != usize_to_u64(prepared.decisions.len(), "decision commit")?
        || audit.money_count != usize_to_u64(prepared.money.len(), "money commit")?
        || audit.counters != prepared.counters
        || audit.pricing_refused_candidates != prepared.pricing_refused_candidates
        || audit.admitted_pricing_refused != prepared.admitted_pricing_refused
    {
        return Err(
            "Global Replay V3 fresh committed capability differs from prepared evidence".to_owned(),
        );
    }
    Ok(committed)
}

struct GlobalReplaySnapshotV3 {
    witnesses: Vec<WitnessRecordV3>,
    candidates: Vec<CandidateRecordV3>,
    decisions: Vec<DecisionRecordV3>,
    money: Vec<MoneyRecordV3>,
    completions: Vec<CompletionRecordV3>,
    by_publication: HashMap<[u8; 32], CompletionRecordV3>,
}

fn require_capacity(name: &str, held: u64, adding: u64, bound: u64) -> Result<(), String> {
    let needed = held
        .checked_add(adding)
        .ok_or_else(|| format!("Global Replay V3 {name} capacity arithmetic overflowed"))?;
    if needed > bound {
        Err(format!(
            "Global Replay V3 {name} file needs {needed} records but explicit bound is {bound}"
        ))
    } else {
        Ok(())
    }
}

fn append_records<T, const N: usize>(
    path: &Path,
    records: &[T],
    encode: fn(T) -> Result<[u8; N], String>,
) -> Result<(), String>
where
    T: Copy,
{
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|why| {
            format!(
                "Global Replay V3 append file {} could not be opened: {why}",
                path.display()
            )
        })?;
    for record in records.iter().copied() {
        file.write_all(&encode(record)?).map_err(|why| {
            format!(
                "Global Replay V3 append file {} could not write a complete fixed record: {why}",
                path.display()
            )
        })?;
    }
    file.sync_all().map_err(|why| {
        format!(
            "Global Replay V3 append file {} could not synchronize: {why}",
            path.display()
        )
    })
}

fn read_records<T, const N: usize>(
    path: &Path,
    stride: usize,
    bound: u64,
    decode: fn(&[u8; N]) -> Result<T, String>,
) -> Result<Vec<T>, String> {
    if stride != N || N == 0 {
        return Err("Global Replay V3 fixed-record stride contract is invalid".to_owned());
    }
    let mut file = OpenOptions::new().read(true).open(path).map_err(|why| {
        format!(
            "Global Replay V3 authority file {} could not be read: {why}",
            path.display()
        )
    })?;
    let bytes = file
        .metadata()
        .map_err(|why| {
            format!(
                "Global Replay V3 authority file {} has no metadata: {why}",
                path.display()
            )
        })?
        .len();
    let stride_u64 = usize_to_u64(N, "record stride")?;
    if bytes % stride_u64 != 0 {
        return Err(format!(
            "Global Replay V3 authority file {} ends with a torn fixed record: {bytes} bytes is not divisible by {N}",
            path.display()
        ));
    }
    let count = bytes / stride_u64;
    if count > bound {
        return Err(format!(
            "Global Replay V3 authority file {} contains {count} records above explicit bound {bound}",
            path.display()
        ));
    }
    let count_usize = u64_to_usize(count, "record count")?;
    let mut values = Vec::new();
    values.try_reserve_exact(count_usize).map_err(|why| {
        format!(
            "Global Replay V3 authority file {} allocation refused: {why}",
            path.display()
        )
    })?;
    file.seek(SeekFrom::Start(0)).map_err(|why| {
        format!(
            "Global Replay V3 authority file {} could not seek: {why}",
            path.display()
        )
    })?;
    for ordinal in 0..count_usize {
        let mut raw = [0_u8; N];
        file.read_exact(&mut raw).map_err(|why| {
            format!(
                "Global Replay V3 authority file {} record {ordinal} could not be read: {why}",
                path.display()
            )
        })?;
        values.push(decode(&raw).map_err(|why| {
            format!(
                "Global Replay V3 authority file {} record {ordinal} refused: {why}",
                path.display()
            )
        })?);
    }
    Ok(values)
}

#[derive(Clone, Copy)]
struct SemanticSummaryV3 {
    ordered_witness_digest: [u8; 32],
    ordered_candidate_digest: [u8; 32],
    ordered_decision_digest: [u8; 32],
    ordered_money_digest: [u8; 32],
    counters: Counters,
    pricing_refused_candidates: u64,
    admitted_pricing_refused: u64,
    replay_id: [u8; 32],
    publication_id: [u8; 32],
}

fn validate_prepared(prepared: &PreparedGlobalReplayV3) -> Result<(), String> {
    let summary = validate_semantics(
        &prepared.witnesses,
        &prepared.candidates,
        &prepared.decisions,
        &prepared.money,
        prepared.selection_ids,
        0,
    )?;
    if summary.ordered_witness_digest != prepared.ordered_witness_digest
        || summary.ordered_candidate_digest != prepared.ordered_candidate_digest
        || summary.ordered_decision_digest != prepared.ordered_decision_digest
        || summary.ordered_money_digest != prepared.ordered_money_digest
        || summary.counters != prepared.counters
        || summary.pricing_refused_candidates != prepared.pricing_refused_candidates
        || summary.admitted_pricing_refused != prepared.admitted_pricing_refused
        || summary.replay_id != prepared.replay_id
        || summary.publication_id != prepared.publication_id
    {
        return Err("Global Replay V3 prepared authority is internally inconsistent".to_owned());
    }
    Ok(())
}

fn validate_committed_block(
    completion: &CompletionRecordV3,
    snapshot: &GlobalReplaySnapshotV3,
) -> Result<(), String> {
    let witnesses = block_slice(
        &snapshot.witnesses,
        completion.witness_first,
        completion.witness_count,
        "witness",
    )?;
    let candidates = block_slice(
        &snapshot.candidates,
        completion.candidate_first,
        completion.candidate_count,
        "candidate",
    )?;
    let decisions = block_slice(
        &snapshot.decisions,
        completion.decision_first,
        completion.decision_count,
        "decision",
    )?;
    let money = block_slice(
        &snapshot.money,
        completion.money_first,
        completion.money_count,
        "money",
    )?;
    let summary = validate_semantics(
        witnesses,
        candidates,
        decisions,
        money,
        completion.selection_ids,
        completion.candidate_first,
    )?;
    if summary.ordered_witness_digest != completion.ordered_witness_digest
        || summary.ordered_candidate_digest != completion.ordered_candidate_digest
        || summary.ordered_decision_digest != completion.ordered_decision_digest
        || summary.ordered_money_digest != completion.ordered_money_digest
        || summary.counters != completion.counters
        || summary.pricing_refused_candidates != completion.pricing_refused_candidates
        || summary.admitted_pricing_refused != completion.admitted_pricing_refused
        || summary.replay_id != completion.replay_id
        || summary.publication_id != completion.publication_id
    {
        return Err(format!(
            "Global Replay V3 completion {} differs from a fresh replay of its durable blocks",
            hex(&completion.completion_id)
        ));
    }
    Ok(())
}

fn validate_completion_matches_prepared(
    existing: &CompletionRecordV3,
    prepared: &PreparedGlobalReplayV3,
    snapshot: &GlobalReplaySnapshotV3,
) -> Result<(), String> {
    validate_committed_block(existing, snapshot)?;
    if existing.replay_id != prepared.replay_id
        || existing.selection_ids != prepared.selection_ids
        || existing.ordered_witness_digest != prepared.ordered_witness_digest
        || existing.ordered_candidate_digest != prepared.ordered_candidate_digest
        || existing.ordered_decision_digest != prepared.ordered_decision_digest
        || existing.ordered_money_digest != prepared.ordered_money_digest
        || existing.counters != prepared.counters
        || existing.pricing_refused_candidates != prepared.pricing_refused_candidates
        || existing.admitted_pricing_refused != prepared.admitted_pricing_refused
    {
        return Err(format!(
            "Global Replay V3 publication {} aliases different prepared evidence",
            hex(&prepared.publication_id)
        ));
    }
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "fresh reopen replays the complete fixed witness/candidate/decision/money block in one fail-closed semantic validator"
)]
fn validate_semantics(
    witnesses: &[WitnessRecordV3],
    candidates: &[CandidateRecordV3],
    decisions: &[DecisionRecordV3],
    money: &[MoneyRecordV3],
    selection_ids: [[u8; 32]; RUNG_COUNT],
    candidate_base: u64,
) -> Result<SemanticSummaryV3, String> {
    if witnesses.len() != WITNESS_COUNT || decisions.len() != candidates.len() {
        return Err(format!(
            "Global Replay V3 semantic block needs {WITNESS_COUNT} witnesses and one decision per candidate"
        ));
    }
    let mut expected_candidate = candidate_base;
    let mut row_ids = HashSet::new();
    let mut selection_set = HashSet::new();
    for selection_id in selection_ids {
        require_digest("rung selection", &selection_id)?;
        if !selection_set.insert(selection_id) {
            return Err("Global Replay V3 semantic block repeats a rung selection".to_owned());
        }
    }
    for (ordinal, witness) in witnesses.iter().copied().enumerate() {
        witness.validate()?;
        let rung_index = ordinal / TOP_PER_RUNG;
        let expected_rung = CANONICAL_RUNGS_SECONDS
            .get(rung_index)
            .copied()
            .ok_or_else(|| {
                format!("Global Replay V3 witness {ordinal} escaped canonical rung topology")
            })?;
        let expected_selection = selection_ids.get(rung_index).copied().ok_or_else(|| {
            format!("Global Replay V3 witness {ordinal} has no canonical Selection slot")
        })?;
        let expected_rank = u16::try_from((ordinal % TOP_PER_RUNG) + 1)
            .map_err(|_| "Global Replay V3 canonical rank does not fit u16".to_owned())?;
        if witness.rung_seconds != expected_rung
            || witness.rank != expected_rank
            || witness.selection_id != expected_selection
            || witness.candidate_first != expected_candidate
            || !row_ids.insert(witness.row_id)
        {
            return Err(format!(
                "Global Replay V3 witness {ordinal} violates canonical topology or contiguous candidate ownership"
            ));
        }
        let local_first = witness
            .candidate_first
            .checked_sub(candidate_base)
            .ok_or_else(|| {
                "Global Replay V3 witness candidate range precedes its block".to_owned()
            })?;
        let owned = block_slice(
            candidates,
            local_first,
            witness.candidate_count,
            "witness-owned candidate",
        )?;
        if witness.ordered_candidate_digest
            != ordered_ids(
                ORDERED_CANDIDATE_DOMAIN,
                owned.iter().map(|record| record.candidate_id),
            )
        {
            return Err(format!(
                "Global Replay V3 witness {ordinal} candidate-order digest differs"
            ));
        }
        let mut previous: Option<(u64, i64)> = None;
        let mut refused = 0_u64;
        for (candidate_ordinal, candidate) in owned.iter().copied().enumerate() {
            candidate.validate()?;
            if usize::from(candidate.stream_ordinal) != ordinal
                || candidate.candidate_ordinal
                    != usize_to_u64(candidate_ordinal, "candidate ordinal")?
                || candidate.witness_id != witness.witness_id
                || candidate.selection_id != witness.selection_id
                || candidate.row_id != witness.row_id
                || candidate.strategy_digest != witness.strategy_digest
                || candidate.rung_seconds != witness.rung_seconds
                || candidate.rank != witness.rank
                || candidate.family != witness.family
                || candidate.direction != witness.direction
                || candidate.feed != witness.feed
                || previous.is_some_and(|before| {
                    candidate.entry_bar <= before.0 || candidate.entry_micros <= before.1
                })
            {
                return Err(format!(
                    "Global Replay V3 witness {ordinal} candidate {candidate_ordinal} is foreign or reordered"
                ));
            }
            previous = Some((candidate.entry_bar, candidate.entry_micros));
            refused = refused
                .checked_add(u64::from(candidate.path.pricing_refused()))
                .ok_or_else(|| "Global Replay V3 witness refusal count overflowed".to_owned())?;
        }
        if refused != witness.pricing_refused_count {
            return Err(format!(
                "Global Replay V3 witness {ordinal} pricing-refused count differs"
            ));
        }
        expected_candidate = expected_candidate
            .checked_add(witness.candidate_count)
            .ok_or_else(|| "Global Replay V3 candidate ownership overflowed".to_owned())?;
    }
    if expected_candidate
        != candidate_base
            .checked_add(usize_to_u64(candidates.len(), "semantic candidate length")?)
            .ok_or_else(|| "Global Replay V3 semantic candidate end overflowed".to_owned())?
    {
        return Err(
            "Global Replay V3 witnesses do not own every candidate exactly once".to_owned(),
        );
    }

    let persisted_vix = PersistedVixAuthorityV3::new(money)?;
    let scheduled = schedule_candidates(witnesses, candidates, Some(&persisted_vix))?;
    if scheduled.decisions != decisions || scheduled.money != money {
        return Err(
            "Global Replay V3 durable decisions/money differ from a fresh global replay".to_owned(),
        );
    }
    let ordered_witness_digest = ordered_ids(
        ORDERED_WITNESS_DOMAIN,
        witnesses.iter().map(|record| record.witness_id),
    );
    let ordered_candidate_digest = ordered_ids(
        ORDERED_CANDIDATE_DOMAIN,
        candidates.iter().map(|record| record.candidate_id),
    );
    let ordered_decision_digest = ordered_ids(
        ORDERED_DECISION_DOMAIN,
        decisions.iter().map(|record| record.decision_id),
    );
    let ordered_money_digest = ordered_ids(
        ORDERED_MONEY_DOMAIN,
        money.iter().map(|record| record.money_id),
    );
    let replay_id = replay_id(
        &selection_ids,
        ordered_witness_digest,
        ordered_candidate_digest,
        ordered_decision_digest,
        scheduled.counters,
        scheduled.pricing_refused_candidates,
        scheduled.admitted_pricing_refused,
    );
    let publication_id = publication_id(replay_id, ordered_money_digest);
    Ok(SemanticSummaryV3 {
        ordered_witness_digest,
        ordered_candidate_digest,
        ordered_decision_digest,
        ordered_money_digest,
        counters: scheduled.counters,
        pricing_refused_candidates: scheduled.pricing_refused_candidates,
        admitted_pricing_refused: scheduled.admitted_pricing_refused,
        replay_id,
        publication_id,
    })
}

struct PersistedVixAuthorityV3 {
    by_stamp: HashMap<(Vendor, i64), VixStamp>,
}

impl PersistedVixAuthorityV3 {
    fn new(money: &[MoneyRecordV3]) -> Result<Self, String> {
        let mut by_stamp = HashMap::new();
        by_stamp
            .try_reserve(money.len().saturating_mul(2))
            .map_err(|why| {
                format!("Global Replay V3 durable VIX replay index allocation refused: {why}")
            })?;
        for record in money {
            record.validate()?;
            for (timestamp, stamp) in [
                (record.row.entry_micros, record.vix.entry),
                (record.row.exit_micros, record.vix.exit),
            ] {
                if let Some(previous) = by_stamp.insert((record.feed, timestamp), stamp)
                    && previous != stamp
                {
                    return Err(format!(
                        "Global Replay V3 durable India VIX evidence conflicts for {} at {timestamp}",
                        record.feed.as_str()
                    ));
                }
            }
        }
        Ok(Self { by_stamp })
    }
}

impl ExactVixAuthorityV3 for PersistedVixAuthorityV3 {
    fn stamp(&self, feed: Vendor, ts_micros: i64) -> Result<VixStamp, String> {
        self.by_stamp
            .get(&(feed, ts_micros))
            .copied()
            .ok_or_else(|| {
                format!(
                    "Global Replay V3 durable money block has no {} India VIX stamp at admitted timestamp {ts_micros}",
                    feed.as_str()
                )
            })
    }
}

fn block_slice<'a, T>(
    values: &'a [T],
    first: u64,
    count: u64,
    subject: &str,
) -> Result<&'a [T], String> {
    let first = u64_to_usize(first, subject)?;
    let count = u64_to_usize(count, subject)?;
    let end = first
        .checked_add(count)
        .ok_or_else(|| format!("Global Replay V3 {subject} range overflowed"))?;
    values.get(first..end).ok_or_else(|| {
        format!(
            "Global Replay V3 {subject} range {first}..{end} exceeds {} records",
            values.len()
        )
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell as CounterCell;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    struct AbsentVix {
        calls: CounterCell<u64>,
    }

    impl AbsentVix {
        const fn new() -> Self {
            Self {
                calls: CounterCell::new(0),
            }
        }
    }

    impl ExactVixAuthorityV3 for AbsentVix {
        fn stamp(&self, _feed: Vendor, _ts_micros: i64) -> Result<VixStamp, String> {
            self.calls.set(self.calls.get().saturating_add(1));
            Ok(VixStamp::Absent)
        }
    }

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(label: &str) -> Result<Self, String> {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "brutex-global-replay-v3-{label}-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir(&path).map_err(|why| {
                format!(
                    "cannot create unique Global Replay V3 test root {}: {why}",
                    path.display()
                )
            })?;
            Ok(Self(path))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn digest(label: &[u8], value: usize) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(b"brutex-global-replay-v3-test-only\0");
        hasher.update(label);
        hasher.update(&value.to_le_bytes());
        hasher.finalize()
    }

    fn priceable(signal_bar: usize, entry_bar: usize, exit_bar: usize) -> TradeRow {
        TradeRow {
            signal_bar,
            entry_bar,
            exit_bar,
            best: 25,
            worst: 10,
            entry_micros: i64::try_from(entry_bar)
                .unwrap_or(i64::MAX)
                .saturating_mul(MICROS_PER_MINUTE),
            exit_micros: i64::try_from(exit_bar)
                .unwrap_or(i64::MAX)
                .saturating_mul(MICROS_PER_MINUTE),
            adverse: 1,
            adverse_paisa: 1,
            favourable: 2,
            favourable_paisa: 2,
        }
    }

    fn candidate(
        signal_bar: usize,
        entry_bar: usize,
        exit_bar: usize,
        path: CandidatePathV3,
    ) -> AuthenticatedReplayCandidateV3 {
        let price = matches!(path, CandidatePathV3::Priceable)
            .then(|| priceable(signal_bar, entry_bar, exit_bar));
        AuthenticatedReplayCandidateV3 {
            signal_bar: u64::try_from(signal_bar).unwrap_or(u64::MAX),
            entry_bar: u64::try_from(entry_bar).unwrap_or(u64::MAX),
            occupied_through_bar: u64::try_from(exit_bar).unwrap_or(u64::MAX),
            signal_micros: i64::try_from(signal_bar)
                .unwrap_or(i64::MAX)
                .saturating_mul(MICROS_PER_MINUTE),
            entry_micros: i64::try_from(entry_bar)
                .unwrap_or(i64::MAX)
                .saturating_mul(MICROS_PER_MINUTE),
            occupied_through_micros: i64::try_from(exit_bar)
                .unwrap_or(i64::MAX)
                .saturating_mul(MICROS_PER_MINUTE),
            path,
            price,
            ambiguous_bars: 0,
            gap_fills: 0,
        }
    }

    fn authorities() -> Result<Vec<AuthenticatedReplayWitnessV3>, String> {
        let mut values = Vec::with_capacity(WITNESS_COUNT);
        for index in 0..WITNESS_COUNT {
            let rung_index = index / TOP_PER_RUNG;
            let rung_seconds = CANONICAL_RUNGS_SECONDS
                .get(rung_index)
                .copied()
                .ok_or_else(|| format!("fixture rung index {rung_index} escaped topology"))?;
            values.push(AuthenticatedReplayWitnessV3 {
                selection_id: digest(b"selection", rung_index),
                row_id: digest(b"row", index),
                selected_exit_digest: digest(b"selected-exit", index),
                universe_digest: digest(b"universe", index),
                run_id: digest(b"run", index),
                feed: Vendor::Zerodha,
                family: if index.is_multiple_of(2) {
                    InstrumentFamilyV1::Nifty
                } else {
                    InstrumentFamilyV1::BankNifty
                },
                direction: if index.is_multiple_of(2) {
                    Direction::Long
                } else {
                    Direction::Short
                },
                strategy_digest: digest(b"strategy", index),
                mask_words: [
                    u64::try_from(index).unwrap_or(0).saturating_add(1),
                    0,
                    0,
                    0,
                    0,
                    0,
                ],
                rung_seconds,
                rank: u16::try_from((index % TOP_PER_RUNG) + 1).unwrap_or(u16::MAX),
                candidates: Vec::new(),
            });
        }
        Ok(values)
    }

    fn add_candidate(
        values: &mut [AuthenticatedReplayWitnessV3],
        index: usize,
        value: AuthenticatedReplayCandidateV3,
    ) -> Result<(), String> {
        values
            .get_mut(index)
            .ok_or_else(|| format!("fixture witness index {index} is absent"))?
            .candidates
            .push(value);
        Ok(())
    }

    fn corrupt_byte<const N: usize>(bytes: &mut [u8; N]) -> Result<(), String> {
        let byte = bytes
            .get_mut(17)
            .ok_or_else(|| "fixed fixture codec has no corruption byte 17".to_owned())?;
        *byte ^= 1;
        Ok(())
    }

    fn prepared_fixture() -> Result<(PreparedGlobalReplayV3, u64), String> {
        let mut values = authorities()?;
        add_candidate(
            &mut values,
            0,
            candidate(0, 1, 2, CandidatePathV3::Priceable),
        )?;
        add_candidate(
            &mut values,
            1,
            candidate(0, 1, 1, CandidatePathV3::Priceable),
        )?;
        add_candidate(
            &mut values,
            25,
            candidate(1, 2, 2, CandidatePathV3::Priceable),
        )?;
        add_candidate(
            &mut values,
            26,
            candidate(2, 3, 4, CandidatePathV3::BlockOnly),
        )?;
        add_candidate(
            &mut values,
            27,
            candidate(3, 4, 4, CandidatePathV3::Priceable),
        )?;
        add_candidate(
            &mut values,
            28,
            candidate(4, 5, 6, CandidatePathV3::Priceable),
        )?;
        let vix = AbsentVix::new();
        let prepared = PreparedGlobalReplayV3::prepare(&values, &vix)?;
        Ok((prepared, vix.calls.get()))
    }

    #[test]
    fn fixed_topology_and_one_inclusive_global_lock_are_enforced() -> Result<(), String> {
        let (prepared, vix_calls) = prepared_fixture()?;
        assert_eq!(prepared.witnesses.len(), WITNESS_COUNT);
        assert_eq!(prepared.candidates.len(), 6);
        assert_eq!(prepared.decisions.len(), 6);
        assert_eq!(prepared.money.len(), 2);
        assert_eq!(prepared.counters.offered, 6);
        assert_eq!(prepared.counters.admitted, 3);
        assert_eq!(prepared.counters.blocked_simultaneous, 1);
        assert_eq!(prepared.counters.blocked_occupied, 2);
        assert_eq!(prepared.pricing_refused_candidates, 1);
        assert_eq!(prepared.admitted_pricing_refused, 1);
        assert_eq!(
            vix_calls, 4,
            "VIX is read twice per admitted priceable trade only"
        );
        assert_eq!(validate_prepared(&prepared), Ok(()));
        assert!(matches!(
            prepared
                .decisions
                .get(1)
                .ok_or_else(|| "fixture decision 1 is absent".to_owned())?
                .disposition,
            DecisionDispositionV3::BlockedSimultaneous { .. }
        ));
        assert!(matches!(
            prepared
                .decisions
                .get(2)
                .ok_or_else(|| "fixture decision 2 is absent".to_owned())?
                .disposition,
            DecisionDispositionV3::BlockedOccupied { .. }
        ));
        assert!(matches!(
            prepared
                .decisions
                .get(3)
                .ok_or_else(|| "fixture decision 3 is absent".to_owned())?
                .disposition,
            DecisionDispositionV3::Admitted { .. }
        ));
        assert!(
            prepared
                .money
                .iter()
                .all(|row| row.vix.entry == VixStamp::Absent && row.vix.exit == VixStamp::Absent)
        );
        Ok(())
    }

    #[test]
    fn malformed_or_noncanonical_authority_sets_refuse_before_publication() -> Result<(), String> {
        let vix = AbsentVix::new();
        let mut too_short = authorities()?;
        let _ = too_short.pop();
        assert!(PreparedGlobalReplayV3::prepare(&too_short, &vix).is_err());

        let mut wrong_rank = authorities()?;
        wrong_rank
            .get_mut(10)
            .ok_or_else(|| "fixture witness 10 is absent".to_owned())?
            .rank = 25;
        assert!(PreparedGlobalReplayV3::prepare(&wrong_rank, &vix).is_err());

        let mut mixed_selection = authorities()?;
        mixed_selection
            .get_mut(1)
            .ok_or_else(|| "fixture witness 1 is absent".to_owned())?
            .selection_id = digest(b"foreign-selection", 1);
        assert!(PreparedGlobalReplayV3::prepare(&mixed_selection, &vix).is_err());

        let mut duplicate_row = authorities()?;
        let first_row = duplicate_row
            .first()
            .ok_or_else(|| "fixture first witness is absent".to_owned())?
            .row_id;
        duplicate_row
            .get_mut(1)
            .ok_or_else(|| "fixture witness 1 is absent".to_owned())?
            .row_id = first_row;
        assert!(PreparedGlobalReplayV3::prepare(&duplicate_row, &vix).is_err());

        let mut empty_mask = authorities()?;
        empty_mask
            .first_mut()
            .ok_or_else(|| "fixture first witness is absent".to_owned())?
            .mask_words = [0; 6];
        assert!(PreparedGlobalReplayV3::prepare(&empty_mask, &vix).is_err());

        let mut impossible_money = authorities()?;
        impossible_money
            .first_mut()
            .ok_or_else(|| "fixture first witness is absent".to_owned())?
            .candidates
            .push(AuthenticatedReplayCandidateV3 {
                path: CandidatePathV3::BlockOnly,
                price: Some(priceable(0, 1, 2)),
                ..candidate(0, 1, 2, CandidatePathV3::BlockOnly)
            });
        assert!(PreparedGlobalReplayV3::prepare(&impossible_money, &vix).is_err());
        Ok(())
    }

    #[test]
    fn every_v3_codec_round_trips_and_refuses_one_corrupt_byte() -> Result<(), String> {
        let (prepared, _) = prepared_fixture()?;
        let witness = prepared
            .witnesses
            .first()
            .copied()
            .ok_or_else(|| "fixture first witness is absent".to_owned())?;
        let candidate = prepared
            .candidates
            .first()
            .copied()
            .ok_or_else(|| "fixture first candidate is absent".to_owned())?;
        let decision = prepared
            .decisions
            .first()
            .copied()
            .ok_or_else(|| "fixture first decision is absent".to_owned())?;
        let money = prepared
            .money
            .first()
            .copied()
            .ok_or_else(|| "fixture first money row is absent".to_owned())?;

        let mut witness_bytes = witness.encode()?;
        assert_eq!(WitnessRecordV3::decode(&witness_bytes), Ok(witness));
        corrupt_byte(&mut witness_bytes)?;
        assert!(WitnessRecordV3::decode(&witness_bytes).is_err());

        let mut candidate_bytes = candidate.encode()?;
        assert_eq!(CandidateRecordV3::decode(&candidate_bytes), Ok(candidate));
        corrupt_byte(&mut candidate_bytes)?;
        assert!(CandidateRecordV3::decode(&candidate_bytes).is_err());

        let mut decision_bytes = decision.encode()?;
        assert_eq!(DecisionRecordV3::decode(&decision_bytes), Ok(decision));
        corrupt_byte(&mut decision_bytes)?;
        assert!(DecisionRecordV3::decode(&decision_bytes).is_err());

        let mut money_bytes = money.encode()?;
        assert_eq!(MoneyRecordV3::decode(&money_bytes), Ok(money));
        corrupt_byte(&mut money_bytes)?;
        assert!(MoneyRecordV3::decode(&money_bytes).is_err());
        Ok(())
    }

    #[test]
    fn receipt_last_commit_reuses_exact_bytes_and_reopen_replays_semantics() -> Result<(), String> {
        let (prepared, _) = prepared_fixture()?;
        let root = TempRoot::new("commit")?;
        let bounds = GlobalReplayV3Bounds::new(400, 20, 20, 20, 2)?;
        let committed = commit_prepared_global_replay_v3(root.path(), bounds, &prepared)?;
        assert!(committed.was_written());
        let audit = committed.audit()?;
        assert_eq!(audit.replay_id(), prepared.replay_id);
        assert_eq!(audit.publication_id(), prepared.publication_id);
        assert_eq!(
            audit.witness_count(),
            usize_to_u64(WITNESS_COUNT, "witness audit count")?
        );
        assert_eq!(
            audit.candidate_count(),
            usize_to_u64(prepared.candidates.len(), "candidate audit count")?
        );
        assert_eq!(
            audit.decision_count(),
            usize_to_u64(prepared.decisions.len(), "decision audit count")?
        );
        assert_eq!(
            audit.money_count(),
            usize_to_u64(prepared.money.len(), "money audit count")?
        );
        assert_eq!(audit.counters(), prepared.counters);
        assert_eq!(
            audit.pricing_refused_candidates(),
            prepared.pricing_refused_candidates
        );
        assert_eq!(
            audit.admitted_pricing_refused(),
            prepared.admitted_pricing_refused
        );

        let reused = commit_prepared_global_replay_v3(root.path(), bounds, &prepared)?;
        assert!(!reused.was_written());
        assert_eq!(reused.audit()?, audit);

        let snapshot = reused.ledger.load_snapshot()?;
        assert_eq!(snapshot.witnesses.len(), WITNESS_COUNT);
        assert_eq!(snapshot.candidates.len(), prepared.candidates.len());
        assert_eq!(snapshot.decisions.len(), prepared.decisions.len());
        assert_eq!(snapshot.money.len(), prepared.money.len());
        assert_eq!(snapshot.completions.len(), 1);

        let completion = snapshot
            .completions
            .first()
            .copied()
            .ok_or_else(|| "fixture completion is absent".to_owned())?;
        assert_eq!(audit.completion_id(), completion.completion_id);
        let mut completion_bytes = completion.encode()?;
        assert_eq!(
            CompletionRecordV3::decode(&completion_bytes),
            Ok(completion)
        );
        corrupt_byte(&mut completion_bytes)?;
        assert!(CompletionRecordV3::decode(&completion_bytes).is_err());
        Ok(())
    }

    #[test]
    fn torn_tail_and_foreign_money_are_never_hidden_by_a_valid_receipt() -> Result<(), String> {
        let (prepared, _) = prepared_fixture()?;
        let root = TempRoot::new("torn")?;
        let bounds = GlobalReplayV3Bounds::new(400, 20, 20, 20, 2)?;
        let mut ledger = GlobalReplayLedgerV3::open(root.path(), bounds)?;
        assert_eq!(ledger.commit(&prepared), Ok(GlobalReplayCommitV3::Written));

        let mut file = OpenOptions::new()
            .append(true)
            .open(&ledger.paths.candidate)
            .map_err(|why| format!("cannot open candidate tail: {why}"))?;
        file.write_all(&[0xA5])
            .map_err(|why| format!("cannot write one torn tail byte: {why}"))?;
        file.sync_data()
            .map_err(|why| format!("cannot synchronize torn byte: {why}"))?;
        assert!(GlobalReplayLedgerV3::open(root.path(), bounds).is_err());

        let mut foreign_money = prepared.clone();
        foreign_money
            .money
            .first_mut()
            .ok_or_else(|| "fixture first money row is absent".to_owned())?
            .candidate_id = digest(b"foreign-candidate", 0);
        assert!(validate_prepared(&foreign_money).is_err());
        Ok(())
    }

    fn assert_resealed_record_bytes_refuse<const N: usize>(
        committed: &CommittedStoredGlobalReplayV3,
        path: &Path,
        domain: &[u8],
        records: &[usize],
    ) -> Result<usize, String> {
        let original = std::fs::read(path).map_err(|why| why.to_string())?;
        let authority = committed.audit()?;
        let mut checked = 0;
        for record in records {
            let start = record
                .checked_mul(N)
                .ok_or_else(|| "fixture record offset overflowed".to_owned())?;
            let end = start
                .checked_add(N)
                .ok_or_else(|| "fixture record end overflowed".to_owned())?;
            for byte in 0..N {
                let mut changed = original.clone();
                let raw: &mut [u8; N] = changed
                    .get_mut(start..end)
                    .ok_or_else(|| "fixture record is absent".to_owned())?
                    .try_into()
                    .map_err(|why| format!("fixture record stride differs: {why}"))?;
                *raw.get_mut(byte)
                    .ok_or_else(|| "fixture byte is absent".to_owned())? ^= 1;
                if byte < N - SEAL_BYTES {
                    seal_record(domain, raw)?;
                }
                std::fs::write(path, &changed).map_err(|why| why.to_string())?;
                assert!(
                    committed.audit().is_err(),
                    "{} record {record} byte {byte} retained its original authority",
                    path.display()
                );
                assert_eq!(
                    std::fs::read(path).map_err(|why| why.to_string())?,
                    changed,
                    "refusal cannot rewrite the corrupted record"
                );
                checked += 1;
            }
        }
        std::fs::write(path, &original).map_err(|why| why.to_string())?;
        assert_eq!(committed.audit()?, authority);
        assert_eq!(
            std::fs::read(path).map_err(|why| why.to_string())?,
            original,
            "reopening restored bytes is read-only"
        );
        Ok(checked)
    }

    #[test]
    fn replay_v3_record_bytes_remain_bound_after_outer_resealing() -> Result<(), String> {
        let (prepared, _) = prepared_fixture()?;
        let root = TempRoot::new("resealed-record-bytes")?;
        let bounds = GlobalReplayV3Bounds::new(400, 20, 20, 20, 2)?;
        let committed = commit_prepared_global_replay_v3(root.path(), bounds, &prepared)?;
        assert!(committed.was_written());
        let authority = committed.audit()?;
        let paths = GlobalReplayPathsV3::new(root.path());
        // The complete 200-witness topology stays present. Exercise both
        // boundary witness records and every record of the other four files.
        let mut checked = assert_resealed_record_bytes_refuse::<GLOBAL_REPLAY_V3_WITNESS_BYTES>(
            &committed,
            &paths.witness,
            WITNESS_SEAL_DOMAIN,
            &[0, WITNESS_COUNT - 1],
        )?;
        checked += assert_resealed_record_bytes_refuse::<GLOBAL_REPLAY_V3_CANDIDATE_BYTES>(
            &committed,
            &paths.candidate,
            CANDIDATE_SEAL_DOMAIN,
            &(0..prepared.candidates.len()).collect::<Vec<_>>(),
        )?;
        checked += assert_resealed_record_bytes_refuse::<GLOBAL_REPLAY_V3_DECISION_BYTES>(
            &committed,
            &paths.decision,
            DECISION_SEAL_DOMAIN,
            &(0..prepared.decisions.len()).collect::<Vec<_>>(),
        )?;
        checked += assert_resealed_record_bytes_refuse::<GLOBAL_REPLAY_V3_MONEY_BYTES>(
            &committed,
            &paths.money,
            MONEY_SEAL_DOMAIN,
            &(0..prepared.money.len()).collect::<Vec<_>>(),
        )?;
        checked += assert_resealed_record_bytes_refuse::<GLOBAL_REPLAY_V3_COMPLETION_BYTES>(
            &committed,
            &paths.completion,
            COMPLETION_SEAL_DOMAIN,
            &[0],
        )?;
        assert_eq!(checked, 8_448);
        let reopened = commit_prepared_global_replay_v3(root.path(), bounds, &prepared)?;
        assert!(!reopened.was_written());
        assert_eq!(reopened.audit()?, authority);
        Ok(())
    }
}
