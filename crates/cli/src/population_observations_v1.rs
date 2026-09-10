//! Sealed per-session observations derived from exact Candidate V1 replay.
//!
//! Population Statistics V2 cannot accept caller-authored period returns or
//! split scores as production evidence. This module supplies the missing typed
//! seam without changing Statistics V2 bytes: Candidate production replays
//! each exact evaluated exit cell into [`runner::grid::TradeRow`] values, then
//! this module attributes those rows to their exit IST session and derives the
//! complete complementary-half score family internally.
//!
//! A period exists for every ordered IST session containing at least one bar
//! accepted by the exact Candidate execution column. A session with no exits
//! is retained with zero return, trades and wins. Return is the checked sum of
//! `TradeRow::worst`; a win is exactly `worst > 0`; an entry and exit on
//! different IST days refuses the complete candidate. NIFTY and BANKNIFTY may
//! be paired only when their exact session sequences and shared source-policy
//! terms agree.
//!
//! CSCV layout has no caller parameter and no default. Version one selects the
//! largest even divisor of the period count in `2..=16`. The bound is a
//! versioned scientific-layout term, not a runtime resource fallback. If no
//! such divisor exists, pairing refuses; it never drops periods, pads a block,
//! or silently selects a different segment count. Every contiguous block is
//! therefore exactly equal-sized.
//!
//! Construction, replay, aggregation, split enumeration and hashing are all
//! input-dependent. This module makes no whole-operation O(1) time, space,
//! latency, capacity or profitability claim.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};

use brutex_core::blake3::Hasher;
use indicators::{Candle, column::Column};
use pull::calendar::{DayKind, kind_of};
use runner::grid::{Cell, TradeRow};

use crate::candidate_universe::CandidateUniverseReceiptV1;
use crate::population::InstrumentFamilyV1;
use crate::pre_admission_data::{
    PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2, PreAdmissionDataV2, PreAdmissionProductionCommitV2,
    ProducedPreAdmissionDataV2, decode_observation_source_v2,
};
use crate::stored::CompleteCalendarReceiptV2;

/// Version of the sealed Candidate-observation capability.
pub const OBSERVATION_SCHEMA_VERSION_V1: u32 = 1;
/// Version of accepted-session enumeration and exit-session attribution.
pub const SESSION_OBSERVATION_POLICY_VERSION_V1: u32 = 1;
/// Version of the pessimistic-return and strict-win score policy.
pub const OBSERVATION_SCORE_POLICY_VERSION_V1: u32 = 1;
/// Version of the deterministic equal-block CSCV layout policy.
pub const CSCV_LAYOUT_POLICY_VERSION_V1: u32 = 1;
/// Largest segment count considered by deterministic layout version one.
pub const MAX_CSCV_SEGMENTS_V1: u32 = 16;
/// Structural maximum split count implied by `MAX_CSCV_SEGMENTS_V1`.
pub const MAX_CANONICAL_CSCV_SPLITS_V1: u64 = 6_435;

const OBSERVATION_POLICY_DOMAIN: &[u8] = b"brutex-candidate-session-observation-policy-v1\0";
const PERIOD_ID_DOMAIN: &[u8] = b"brutex-candidate-session-period-v1\0";
const FAMILY_ID_DOMAIN: &[u8] = b"brutex-candidate-session-family-v1\0";
const LAYOUT_POLICY_DOMAIN: &[u8] = b"brutex-candidate-session-cscv-layout-v1\0";
const PAIR_SOURCE_ID_DOMAIN: &[u8] = b"brutex-candidate-session-pair-source-v1\0";
const SPLIT_ROW_ID_DOMAIN: &[u8] = b"brutex-candidate-session-split-row-v1\0";
const PAIR_ID_DOMAIN: &[u8] = b"brutex-candidate-session-pair-v1\0";
const AUTHORITY_ID_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-id-v1\0";
const AUTHORITY_RECORD_SEAL_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-record-v1\0";
const AUTHORITY_COMPLETION_SEAL_DOMAIN: &[u8] =
    b"brutex-candidate-observation-authority-completion-v1\0";
const AUTHORITY_FILE_HEADER_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-header-v1\0";
const AUTHORITY_FILE_DIGEST_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-file-v1\0";
const AUTHORITY_FILE: &str = "candidate-observation-authorities-v1.bin";
const AUTHORITY_LOCK_FILE: &str = "candidate-observation-authorities-v1.lock";
const AUTHORITY_HEADER_MAGIC: [u8; 16] = *b"BTX-OBS-AUTH-H1\0";
const AUTHORITY_DATA_MAGIC: [u8; 16] = *b"BTX-OBS-AUTH-D1\0";
const AUTHORITY_COMPLETION_MAGIC: [u8; 16] = *b"BTX-OBS-AUTH-C1\0";
const AUTHORITY_FILE_VERSION: u32 = 1;
const AUTHORITY_DATA_KIND: u32 = 1;
const AUTHORITY_COMPLETION_KIND: u32 = 2;
const AUTHORITY_HEADER_BYTES: usize = 64;
const AUTHORITY_RECORD_BYTES: usize = 512;
const AUTHORITY_RECORD_BYTES_U32: u32 = 512;
const AUTHORITY_RECORD_PAYLOAD_BYTES: usize = 480;

/// Fixed header width of the companion observation-authority file.
pub const OBSERVATION_AUTHORITY_HEADER_BYTES_V1: u64 = 64;
/// Fixed width of both Data and Completion records.
pub const OBSERVATION_AUTHORITY_RECORD_STRIDE_V1: u64 = 512;

const _: () = assert!(AUTHORITY_HEADER_BYTES as u64 == OBSERVATION_AUTHORITY_HEADER_BYTES_V1);
const _: () = assert!(AUTHORITY_RECORD_BYTES as u64 == OBSERVATION_AUTHORITY_RECORD_STRIDE_V1);
const _: () = assert!(AUTHORITY_RECORD_BYTES_U32 as usize == AUTHORITY_RECORD_BYTES);
const _: () = assert!(AUTHORITY_RECORD_PAYLOAD_BYTES + 32 == AUTHORITY_RECORD_BYTES);

/// Version of the zero-family Observation successor format.
pub const OBSERVATION_AUTHORITY_SCHEMA_VERSION_V2: u32 = 2;
/// Fixed header width of the Observation V2 authority file.
pub const OBSERVATION_AUTHORITY_HEADER_BYTES_V2: u64 = 64;
/// Fixed width of both Observation V2 Data and Completion records.
pub const OBSERVATION_AUTHORITY_RECORD_STRIDE_V2: u64 = 1_024;

const AUTHORITY_V2_HEADER_BYTES: usize = 64;
const AUTHORITY_V2_RECORD_BYTES: usize = 1_024;
const AUTHORITY_V2_PAYLOAD_BYTES: usize = 992;
const AUTHORITY_V2_SOURCE_OFFSET: usize = 120;
const AUTHORITY_V2_SOURCE_END: usize =
    AUTHORITY_V2_SOURCE_OFFSET + PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2;
const AUTHORITY_V2_FILE: &str = "candidate-observation-authorities-v2.bin";
const AUTHORITY_V2_LOCK_FILE: &str = "candidate-observation-authorities-v2.lock";
const AUTHORITY_V2_HEADER_MAGIC: [u8; 16] = *b"BTX-OBS-AUTH-H2\0";
const AUTHORITY_V2_RECORD_MAGIC: [u8; 16] = *b"BTX-OBS-AUTH-R2\0";
const AUTHORITY_V2_DATA_KIND: u32 = 1;
const AUTHORITY_V2_COMPLETION_KIND: u32 = 2;
const AUTHORITY_V2_DISPOSITION_NATURAL_EXTINCTION: u32 = 1;
const AUTHORITY_V2_HEADER_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-header-v2\0";
const AUTHORITY_V2_POLICY_DOMAIN: &[u8] = b"brutex-candidate-observation-policy-v2\0";
const AUTHORITY_V2_ID_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-id-v2\0";
const AUTHORITY_V2_RECORD_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-record-v2\0";
const AUTHORITY_V2_FILE_DOMAIN: &[u8] = b"brutex-candidate-observation-authority-file-v2\0";

const _: () = assert!(AUTHORITY_V2_HEADER_BYTES as u64 == OBSERVATION_AUTHORITY_HEADER_BYTES_V2);
const _: () = assert!(AUTHORITY_V2_RECORD_BYTES as u64 == OBSERVATION_AUTHORITY_RECORD_STRIDE_V2);
const _: () = assert!(AUTHORITY_V2_PAYLOAD_BYTES + 32 == AUTHORITY_V2_RECORD_BYTES);
const _: () = assert!(AUTHORITY_V2_SOURCE_END <= AUTHORITY_V2_PAYLOAD_BYTES);

/// One exact accepted-session period for one Candidate V1 row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateSessionPeriodV1 {
    candidate_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    period_sequence: u64,
    exit_ist_day: i64,
    return_paisa: i64,
    trades: u64,
    wins: u64,
    identity: [u8; 32],
}

impl CandidateSessionPeriodV1 {
    /// Candidate sequence in the complete canonical family.
    #[must_use]
    pub const fn candidate_sequence(self) -> u64 {
        self.candidate_sequence
    }

    /// Candidate semantic identity produced by Candidate Universe V1.
    #[must_use]
    pub const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate_semantic_digest
    }

    /// Zero-based sequence in the exact accepted-session list.
    #[must_use]
    pub const fn period_sequence(self) -> u64 {
        self.period_sequence
    }

    /// IST civil day owning every exit attributed to this period.
    #[must_use]
    pub const fn exit_ist_day(self) -> i64 {
        self.exit_ist_day
    }

    /// Checked sum of exact `TradeRow::worst` values exiting in this session.
    #[must_use]
    pub const fn return_paisa(self) -> i64 {
        self.return_paisa
    }

    /// Exact number of trades exiting in this session.
    #[must_use]
    pub const fn trades(self) -> u64 {
        self.trades
    }

    /// Exact number of exiting trades whose `worst` value is strictly positive.
    #[must_use]
    pub const fn wins(self) -> u64 {
        self.wins
    }

    /// Domain-separated identity binding source, policy, candidate and values.
    #[must_use]
    pub const fn identity(self) -> [u8; 32] {
        self.identity
    }
}

/// Complete ordered accepted-session observations for one Candidate V1 row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateSessionObservationsV1 {
    candidate_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    periods: Vec<CandidateSessionPeriodV1>,
    total_return_paisa: i64,
    total_trades: u64,
    total_wins: u64,
}

impl CandidateSessionObservationsV1 {
    /// Candidate sequence in canonical Candidate V1 row order.
    #[must_use]
    pub const fn candidate_sequence(&self) -> u64 {
        self.candidate_sequence
    }

    /// Candidate semantic identity.
    #[must_use]
    pub const fn candidate_semantic_digest(&self) -> [u8; 32] {
        self.candidate_semantic_digest
    }

    /// One row per exact accepted IST session, including explicit zeroes.
    #[must_use]
    pub fn periods(&self) -> &[CandidateSessionPeriodV1] {
        &self.periods
    }

    /// Exact checked pessimistic return across all periods.
    #[must_use]
    pub const fn total_return_paisa(&self) -> i64 {
        self.total_return_paisa
    }

    /// Exact trade count across all periods.
    #[must_use]
    pub const fn total_trades(&self) -> u64 {
        self.total_trades
    }

    /// Exact strict-positive worst-case win count across all periods.
    #[must_use]
    pub const fn total_wins(&self) -> u64 {
        self.total_wins
    }
}

/// Identity-bound version receipt for the observation and score policies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationPolicyReceiptV1 {
    schema_version: u32,
    session_policy_version: u32,
    score_policy_version: u32,
    digest: [u8; 32],
}

impl ObservationPolicyReceiptV1 {
    /// Observation capability schema version.
    #[must_use]
    pub const fn schema_version(self) -> u32 {
        self.schema_version
    }

    /// Accepted-session and exit-attribution policy version.
    #[must_use]
    pub const fn session_policy_version(self) -> u32 {
        self.session_policy_version
    }

    /// Pessimistic-return and strict-win policy version.
    #[must_use]
    pub const fn score_policy_version(self) -> u32 {
        self.score_policy_version
    }

    /// Domain-separated policy identity.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

/// Complete sealed observation family for one swept index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateFamilyObservationsV1 {
    source: CandidateUniverseReceiptV1,
    policy: ObservationPolicyReceiptV1,
    accepted_ist_sessions: Vec<i64>,
    candidates: Vec<CandidateSessionObservationsV1>,
    identity: [u8; 32],
}

impl CandidateFamilyObservationsV1 {
    /// Exact Candidate V1 completion that owns this family.
    #[must_use]
    pub const fn source(&self) -> CandidateUniverseReceiptV1 {
        self.source
    }

    /// Swept index family.
    #[must_use]
    pub const fn family(&self) -> InstrumentFamilyV1 {
        self.source.family()
    }

    /// Identity-bound observation/score policy.
    #[must_use]
    pub const fn policy(&self) -> ObservationPolicyReceiptV1 {
        self.policy
    }

    /// Exact ordered IST sessions accepted by this Candidate execution column.
    #[must_use]
    pub fn accepted_ist_sessions(&self) -> &[i64] {
        &self.accepted_ist_sessions
    }

    /// Complete candidates in exact Candidate V1 row order.
    #[must_use]
    pub fn candidates(&self) -> &[CandidateSessionObservationsV1] {
        &self.candidates
    }

    /// Number of candidate rows in this family.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Number of aligned accepted-session periods per candidate.
    #[must_use]
    pub fn period_count(&self) -> usize {
        self.accepted_ist_sessions.len()
    }

    /// Domain-separated identity of source, policies, sessions and observations.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }

    /// Revalidates this opaque family and derives the exact single-family CSCV
    /// layout and score rows consumed by the Statistics V3 successor.
    ///
    /// The caller supplies neither masks nor scores.  Keeping this projection
    /// beside Observation V1 preserves its canonical largest-even-divisor and
    /// ascending-Gosper policies instead of creating a second policy authority
    /// in the statistics layer.
    pub(crate) fn statistics_v3_projection(
        &self,
    ) -> Result<(CscvLayoutReceiptV1, Vec<CandidateSplitScoreV1>), String> {
        self.validate()?;
        let layout = derive_layout(self.period_count())?;
        let masks = canonical_masks(layout)?;
        let row_count = self
            .candidate_count()
            .checked_mul(masks.len())
            .ok_or_else(|| "Statistics V3 split row count overflowed usize".to_owned())?;
        let mut scores = Vec::new();
        scores.try_reserve_exact(row_count).map_err(|why| {
            format!("Statistics V3 could not reserve {row_count} split rows: {why}")
        })?;
        append_family_split_scores(&mut scores, self, 0, layout, &masks, self.identity)?;
        Ok((layout, scores))
    }

    fn validate(&self) -> Result<(), String> {
        if self.policy != observation_policy_receipt() {
            return Err(
                "candidate observation family carries a foreign observation policy".to_owned(),
            );
        }
        let expected_candidates = usize::try_from(self.source.row_count())
            .map_err(|_| "candidate observation source row count does not fit usize".to_owned())?;
        if self.candidates.len() != expected_candidates {
            return Err(format!(
                "candidate observation family has {} candidates, not source row count {expected_candidates}",
                self.candidates.len()
            ));
        }
        validate_session_sequence(&self.accepted_ist_sessions)?;
        for (candidate_index, candidate) in self.candidates.iter().enumerate() {
            let candidate_sequence = u64::try_from(candidate_index)
                .map_err(|_| "candidate observation sequence does not fit u64".to_owned())?;
            if candidate.candidate_sequence != candidate_sequence
                || is_zero_digest(candidate.candidate_semantic_digest)
                || candidate.periods.len() != self.accepted_ist_sessions.len()
            {
                return Err(format!(
                    "candidate observation {candidate_index} has foreign sequence, semantic identity or period width"
                ));
            }
            let mut total_return = 0_i64;
            let mut total_trades = 0_u64;
            let mut total_wins = 0_u64;
            for (period_index, (period, day)) in candidate
                .periods
                .iter()
                .zip(&self.accepted_ist_sessions)
                .enumerate()
            {
                let period_sequence = u64::try_from(period_index)
                    .map_err(|_| "candidate period sequence does not fit u64".to_owned())?;
                let expected_identity = period_identity(
                    self.source.universe_id(),
                    self.policy.digest,
                    candidate_sequence,
                    candidate.candidate_semantic_digest,
                    period_sequence,
                    *day,
                    period.return_paisa,
                    period.trades,
                    period.wins,
                );
                if period.candidate_sequence != candidate_sequence
                    || period.candidate_semantic_digest != candidate.candidate_semantic_digest
                    || period.period_sequence != period_sequence
                    || period.exit_ist_day != *day
                    || period.wins > period.trades
                    || period.identity != expected_identity
                {
                    return Err(format!(
                        "candidate observation {candidate_index} period {period_index} is not canonical"
                    ));
                }
                total_return = total_return
                    .checked_add(period.return_paisa)
                    .ok_or_else(|| {
                        format!(
                            "candidate observation {candidate_index} period return overflowed i64"
                        )
                    })?;
                total_trades = total_trades.checked_add(period.trades).ok_or_else(|| {
                    format!("candidate observation {candidate_index} period trades overflowed u64")
                })?;
                total_wins = total_wins.checked_add(period.wins).ok_or_else(|| {
                    format!("candidate observation {candidate_index} period wins overflowed u64")
                })?;
            }
            if total_return != candidate.total_return_paisa
                || total_trades != candidate.total_trades
                || total_wins != candidate.total_wins
            {
                return Err(format!(
                    "candidate observation {candidate_index} totals do not reconcile"
                ));
            }
        }
        let expected = family_identity(
            &self.source,
            self.policy,
            &self.accepted_ist_sessions,
            &self.candidates,
        );
        if self.identity != expected {
            return Err("candidate observation family identity does not recompute".to_owned());
        }
        Ok(())
    }
}

/// Versioned deterministic equal-block layout derived from aligned periods.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CscvLayoutReceiptV1 {
    policy_version: u32,
    period_count: u64,
    segment_count: u32,
    periods_per_segment: u64,
    split_count: u64,
    maximum_segments: u32,
    maximum_split_count: u64,
    digest: [u8; 32],
}

impl CscvLayoutReceiptV1 {
    /// Layout policy version.
    #[must_use]
    pub const fn policy_version(self) -> u32 {
        self.policy_version
    }

    /// Complete aligned period count; no period is dropped or padded.
    #[must_use]
    pub const fn period_count(self) -> u64 {
        self.period_count
    }

    /// Deterministically selected even segment count.
    #[must_use]
    pub const fn segment_count(self) -> u32 {
        self.segment_count
    }

    /// Exact equal width of every contiguous segment.
    #[must_use]
    pub const fn periods_per_segment(self) -> u64 {
        self.periods_per_segment
    }

    /// Canonical complementary-half split count.
    #[must_use]
    pub const fn split_count(self) -> u64 {
        self.split_count
    }

    /// Versioned maximum segment count used by the scientific layout rule.
    #[must_use]
    pub const fn maximum_segments(self) -> u32 {
        self.maximum_segments
    }

    /// Structural maximum split count implied by the segment bound.
    #[must_use]
    pub const fn maximum_split_count(self) -> u64 {
        self.maximum_split_count
    }

    /// Domain-separated layout-policy identity.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

/// One internally derived complementary-half score row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CandidateSplitScoreV1 {
    global_candidate_sequence: u64,
    family: InstrumentFamilyV1,
    family_candidate_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    split_sequence: u64,
    train_mask: u64,
    test_mask: u64,
    train_score_paisa: i64,
    test_score_paisa: i64,
    identity: [u8; 32],
}

impl CandidateSplitScoreV1 {
    /// Candidate sequence in NIFTY-then-BANKNIFTY order.
    #[must_use]
    pub const fn global_candidate_sequence(self) -> u64 {
        self.global_candidate_sequence
    }

    /// Candidate index family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Candidate sequence within its exact family.
    #[must_use]
    pub const fn family_candidate_sequence(self) -> u64 {
        self.family_candidate_sequence
    }

    /// Candidate semantic identity.
    #[must_use]
    pub const fn candidate_semantic_digest(self) -> [u8; 32] {
        self.candidate_semantic_digest
    }

    /// Split sequence in canonical ascending-Gosper order.
    #[must_use]
    pub const fn split_sequence(self) -> u64 {
        self.split_sequence
    }

    /// Canonical in-sample segment mask; bit zero is deliberately absent.
    #[must_use]
    pub const fn train_mask(self) -> u64 {
        self.train_mask
    }

    /// Exact complementary out-of-sample segment mask.
    #[must_use]
    pub const fn test_mask(self) -> u64 {
        self.test_mask
    }

    /// Checked sum of pessimistic period returns in train segments.
    #[must_use]
    pub const fn train_score_paisa(self) -> i64 {
        self.train_score_paisa
    }

    /// Checked sum of pessimistic period returns in test segments.
    #[must_use]
    pub const fn test_score_paisa(self) -> i64 {
        self.test_score_paisa
    }

    /// Domain-separated identity binding source, layout, masks and scores.
    #[must_use]
    pub const fn identity(self) -> [u8; 32] {
        self.identity
    }
}

/// Sealed aligned NIFTY-then-BANKNIFTY observation and split-score capability.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairedCandidateObservationsV1 {
    nifty: CandidateFamilyObservationsV1,
    banknifty: CandidateFamilyObservationsV1,
    layout: CscvLayoutReceiptV1,
    split_scores: Vec<CandidateSplitScoreV1>,
    source_identity: [u8; 32],
    identity: [u8; 32],
}

impl PairedCandidateObservationsV1 {
    /// Exact NIFTY family, always first.
    #[must_use]
    pub const fn nifty(&self) -> &CandidateFamilyObservationsV1 {
        &self.nifty
    }

    /// Exact BANKNIFTY family, always second.
    #[must_use]
    pub const fn banknifty(&self) -> &CandidateFamilyObservationsV1 {
        &self.banknifty
    }

    /// Deterministic, equal-sized, identity-bound CSCV layout.
    #[must_use]
    pub const fn layout(&self) -> CscvLayoutReceiptV1 {
        self.layout
    }

    /// Complete score rows in global-candidate then split order.
    #[must_use]
    pub fn split_scores(&self) -> &[CandidateSplitScoreV1] {
        &self.split_scores
    }

    /// NIFTY-then-BANKNIFTY candidate count.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.nifty
            .candidate_count()
            .saturating_add(self.banknifty.candidate_count())
    }

    /// Source-only identity used as the root for every derived split row.
    #[must_use]
    pub const fn source_identity(&self) -> [u8; 32] {
        self.source_identity
    }

    /// Complete paired observation and split-score identity.
    #[must_use]
    pub const fn identity(&self) -> [u8; 32] {
        self.identity
    }

    /// Pairs exact family observations and derives every score internally.
    ///
    /// The public inputs are opaque capabilities: no external caller can
    /// construct either family, author a period return, supply a split score,
    /// or choose the layout. The minimal Statistics V2 integration seam is a
    /// private constructor accepting `&Self`, projecting its period and split
    /// rows, and rechecking this identity before preparing existing V2 bytes.
    ///
    /// # Errors
    ///
    /// Refuses foreign instrument order, unequal accepted-session sequences,
    /// mismatched source/policy terms, a non-canonical family, a period count
    /// that cannot form the versioned equal-block layout, or arithmetic and
    /// allocation failure while deriving every canonical split score.
    pub fn from_families(
        nifty: CandidateFamilyObservationsV1,
        banknifty: CandidateFamilyObservationsV1,
    ) -> Result<Self, String> {
        nifty.validate()?;
        banknifty.validate()?;
        require_pair_sources(&nifty, &banknifty)?;
        let layout = derive_layout(nifty.period_count())?;
        let source_identity = pair_source_identity(&nifty, &banknifty, layout);
        let masks = canonical_masks(layout)?;
        let candidate_count = nifty
            .candidate_count()
            .checked_add(banknifty.candidate_count())
            .ok_or_else(|| "paired candidate count overflowed usize".to_owned())?;
        let split_count = usize::try_from(layout.split_count)
            .map_err(|_| "paired CSCV split count does not fit usize".to_owned())?;
        let row_count = candidate_count
            .checked_mul(split_count)
            .ok_or_else(|| "paired CSCV score-row count overflowed usize".to_owned())?;
        let mut split_scores = Vec::new();
        split_scores.try_reserve_exact(row_count).map_err(|why| {
            format!("paired CSCV score rows could not reserve {row_count} records: {why}")
        })?;
        append_family_split_scores(
            &mut split_scores,
            &nifty,
            0,
            layout,
            &masks,
            source_identity,
        )?;
        let bank_offset = u64::try_from(nifty.candidate_count())
            .map_err(|_| "NIFTY candidate count does not fit u64".to_owned())?;
        append_family_split_scores(
            &mut split_scores,
            &banknifty,
            bank_offset,
            layout,
            &masks,
            source_identity,
        )?;
        if split_scores.len() != row_count {
            return Err(format!(
                "paired CSCV derived {} score rows, not expected {row_count}",
                split_scores.len()
            ));
        }
        let identity = pair_identity(source_identity, &split_scores);
        Ok(Self {
            nifty,
            banknifty,
            layout,
            split_scores,
            source_identity,
            identity,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PeriodAccumulatorV1 {
    return_paisa: i64,
    trades: u64,
    wins: u64,
}

#[derive(Debug)]
struct CandidateObservationDraftV1 {
    candidate_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    periods: Vec<PeriodAccumulatorV1>,
    total_return_paisa: i64,
    total_trades: u64,
    total_wins: u64,
}

/// Crate-private builder fed only by exact Candidate cell replay.
#[derive(Debug)]
pub(crate) struct CandidateObservationBuilderV1 {
    execution_calendar_first_day: i64,
    execution_calendar_last_day: i64,
    execution_calendar_digest: [u8; 32],
    first_day: i64,
    accepted_ist_sessions: Vec<i64>,
    day_to_period: Vec<Option<usize>>,
    semantics: HashSet<[u8; 32]>,
    candidates: Vec<CandidateObservationDraftV1>,
}

impl CandidateObservationBuilderV1 {
    #[cfg(test)]
    pub(crate) fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    pub(crate) fn from_exact_execution(
        calendar: CompleteCalendarReceiptV2,
        bars: &[Candle],
        column: &Column,
    ) -> Result<Self, String> {
        if calendar.rung_seconds() != 60 {
            return Err(format!(
                "candidate observations require a complete 60-second execution calendar, got {}",
                calendar.rung_seconds()
            ));
        }
        if !column.acceptance_covers(bars.len()) {
            return Err(format!(
                "candidate observations require one exact execution acceptance verdict per bar; bars={}, known={}",
                bars.len(),
                column.acceptance().map_or(0, |accepted| accepted.len())
            ));
        }
        let span = calendar
            .last_day()
            .checked_sub(calendar.first_day())
            .and_then(|width| width.checked_add(1))
            .ok_or_else(|| "candidate observation calendar day span overflowed i64".to_owned())?;
        let span = usize::try_from(span)
            .map_err(|_| "candidate observation calendar day span does not fit usize".to_owned())?;
        let mut day_to_period = Vec::new();
        day_to_period.try_reserve_exact(span).map_err(|why| {
            format!("candidate observation day index could not reserve {span} slots: {why}")
        })?;
        day_to_period.resize(span, None);
        let mut accepted_ist_sessions = Vec::new();
        let mut previous_ts = None;
        for (index, bar) in bars.iter().enumerate() {
            if previous_ts.is_some_and(|previous| bar.ts_micros <= previous) {
                return Err(format!(
                    "candidate observation execution timestamp at bar {index} is not strictly increasing"
                ));
            }
            previous_ts = Some(bar.ts_micros);
            let day = indicators::ist_day(bar.ts_micros);
            if day < calendar.first_day() || day > calendar.last_day() {
                return Err(format!(
                    "candidate observation execution bar {index} belongs to IST day {day}, outside complete calendar {}..={} ",
                    calendar.first_day(),
                    calendar.last_day()
                ));
            }
            if !column.accepts(index) {
                continue;
            }
            if !matches!(kind_of(day), DayKind::Open(_)) {
                return Err(format!(
                    "candidate observation execution column accepted bar {index} on non-open or unmeasured IST day {day}"
                ));
            }
            let offset = day_offset(calendar.first_day(), day, span)?;
            let indexed_period = day_to_period.get_mut(offset).ok_or_else(|| {
                "candidate observation accepted-session index disappeared after bounds check"
                    .to_owned()
            })?;
            if indexed_period.is_none() {
                let sequence = accepted_ist_sessions.len();
                accepted_ist_sessions.try_reserve(1).map_err(|why| {
                    format!("candidate observation session list could not grow: {why}")
                })?;
                accepted_ist_sessions.push(day);
                *indexed_period = Some(sequence);
            }
        }
        validate_session_sequence(&accepted_ist_sessions)?;
        Ok(Self {
            execution_calendar_first_day: calendar.first_day(),
            execution_calendar_last_day: calendar.last_day(),
            execution_calendar_digest: calendar.digest(),
            first_day: calendar.first_day(),
            accepted_ist_sessions,
            day_to_period,
            semantics: HashSet::new(),
            candidates: Vec::new(),
        })
    }

    #[expect(
        clippy::too_many_lines,
        reason = "exact trade validation, exit-session attribution and total reconciliation remain one fail-closed transaction"
    )]
    pub(crate) fn observe_candidate(
        &mut self,
        candidate_semantic_digest: [u8; 32],
        expected: &Cell,
        execution_bars: &[Candle],
        trades: &[TradeRow],
    ) -> Result<(), String> {
        if is_zero_digest(candidate_semantic_digest) {
            return Err("candidate observation semantic digest is zero".to_owned());
        }
        self.semantics
            .try_reserve(1)
            .map_err(|why| format!("candidate observation semantic index could not grow: {why}"))?;
        if !self.semantics.insert(candidate_semantic_digest) {
            return Err("candidate observation semantic digest was duplicated".to_owned());
        }
        let candidate_sequence = u64::try_from(self.candidates.len())
            .map_err(|_| "candidate observation sequence does not fit u64".to_owned())?;
        let mut periods = Vec::new();
        periods
            .try_reserve_exact(self.accepted_ist_sessions.len())
            .map_err(|why| {
                format!(
                    "candidate observation {candidate_sequence} could not reserve {} explicit periods: {why}",
                    self.accepted_ist_sessions.len()
                )
            })?;
        periods.resize(
            self.accepted_ist_sessions.len(),
            PeriodAccumulatorV1::default(),
        );
        let mut total_return = 0_i64;
        let mut total_trades = 0_u64;
        let mut total_wins = 0_u64;
        for (trade_index, trade) in trades.iter().enumerate() {
            let entry = execution_bars.get(trade.entry_bar).ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} trade {trade_index} entry bar {} is outside execution length {}",
                    trade.entry_bar,
                    execution_bars.len()
                )
            })?;
            let exit = execution_bars.get(trade.exit_bar).ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} trade {trade_index} exit bar {} is outside execution length {}",
                    trade.exit_bar,
                    execution_bars.len()
                )
            })?;
            if trade.entry_bar > trade.exit_bar
                || entry.ts_micros != trade.entry_micros
                || exit.ts_micros != trade.exit_micros
            {
                return Err(format!(
                    "candidate observation {candidate_sequence} trade {trade_index} carries foreign indices or timestamps"
                ));
            }
            let entry_day = indicators::ist_day(trade.entry_micros);
            let exit_day = indicators::ist_day(trade.exit_micros);
            if entry_day != exit_day {
                return Err(format!(
                    "candidate observation {candidate_sequence} trade {trade_index} crosses IST days {entry_day}->{exit_day}; version one requires intraday entry and exit"
                ));
            }
            let offset = day_offset(self.first_day, exit_day, self.day_to_period.len())?;
            let period_index = self
                .day_to_period
                .get(offset)
                .copied()
                .flatten()
                .ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} trade {trade_index} exits on IST day {exit_day}, which the exact execution column did not accept"
                )
            })?;
            let period = periods.get_mut(period_index).ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} lost accepted period {period_index}"
                )
            })?;
            period.return_paisa = period.return_paisa.checked_add(trade.worst).ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} period {period_index} pessimistic return overflowed i64"
                )
            })?;
            period.trades = period.trades.checked_add(1).ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} period {period_index} trade count overflowed u64"
                )
            })?;
            if trade.worst > 0 {
                period.wins = period.wins.checked_add(1).ok_or_else(|| {
                    format!(
                        "candidate observation {candidate_sequence} period {period_index} win count overflowed u64"
                    )
                })?;
                total_wins = total_wins.checked_add(1).ok_or_else(|| {
                    format!(
                        "candidate observation {candidate_sequence} total win count overflowed u64"
                    )
                })?;
            }
            total_return = total_return.checked_add(trade.worst).ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} total pessimistic return overflowed i64"
                )
            })?;
            total_trades = total_trades.checked_add(1).ok_or_else(|| {
                format!(
                    "candidate observation {candidate_sequence} total trade count overflowed u64"
                )
            })?;
        }
        let materialized_count = u64::try_from(trades.len())
            .map_err(|_| "materialized trade count does not fit u64".to_owned())?;
        if materialized_count != expected.trades
            || total_trades != expected.trades
            || total_wins != expected.wins
            || total_return != expected.pessimistic
        {
            return Err(format!(
                "candidate observation {candidate_sequence} exact trade rows reconcile as trades/wins/worst={total_trades}/{total_wins}/{total_return}, not evaluated cell {}/{}/{}",
                expected.trades, expected.wins, expected.pessimistic
            ));
        }
        self.candidates
            .try_reserve(1)
            .map_err(|why| format!("candidate observation family could not grow: {why}"))?;
        self.candidates.push(CandidateObservationDraftV1 {
            candidate_sequence,
            candidate_semantic_digest,
            periods,
            total_return_paisa: total_return,
            total_trades,
            total_wins,
        });
        Ok(())
    }

    pub(crate) fn seal(
        self,
        source: &CandidateUniverseReceiptV1,
    ) -> Result<CandidateFamilyObservationsV1, String> {
        let coverage = source.calendar_coverage();
        if coverage.execution_rung_seconds() != 60
            || coverage.first_day() != self.execution_calendar_first_day
            || coverage.last_day() != self.execution_calendar_last_day
            || coverage.execution_receipt_digest() != self.execution_calendar_digest
        {
            return Err(
                "candidate observation execution calendar does not equal the sealed Candidate source"
                    .to_owned(),
            );
        }
        let expected_candidates = usize::try_from(source.row_count())
            .map_err(|_| "candidate source row count does not fit usize".to_owned())?;
        if self.candidates.len() != expected_candidates {
            return Err(format!(
                "candidate observations contain {} candidates, not sealed Candidate row count {expected_candidates}",
                self.candidates.len()
            ));
        }
        let policy = observation_policy_receipt();
        let mut candidates = Vec::new();
        candidates
            .try_reserve_exact(self.candidates.len())
            .map_err(|why| {
                format!(
                    "candidate observations could not reserve {} sealed candidates: {why}",
                    self.candidates.len()
                )
            })?;
        for draft in self.candidates {
            let mut periods = Vec::new();
            periods
                .try_reserve_exact(draft.periods.len())
                .map_err(|why| {
                    format!(
                        "candidate observation {} could not reserve {} sealed periods: {why}",
                        draft.candidate_sequence,
                        draft.periods.len()
                    )
                })?;
            for (period_index, (accumulator, day)) in draft
                .periods
                .into_iter()
                .zip(&self.accepted_ist_sessions)
                .enumerate()
            {
                let period_sequence = u64::try_from(period_index)
                    .map_err(|_| "candidate period sequence does not fit u64".to_owned())?;
                periods.push(CandidateSessionPeriodV1 {
                    candidate_sequence: draft.candidate_sequence,
                    candidate_semantic_digest: draft.candidate_semantic_digest,
                    period_sequence,
                    exit_ist_day: *day,
                    return_paisa: accumulator.return_paisa,
                    trades: accumulator.trades,
                    wins: accumulator.wins,
                    identity: period_identity(
                        source.universe_id(),
                        policy.digest,
                        draft.candidate_sequence,
                        draft.candidate_semantic_digest,
                        period_sequence,
                        *day,
                        accumulator.return_paisa,
                        accumulator.trades,
                        accumulator.wins,
                    ),
                });
            }
            candidates.push(CandidateSessionObservationsV1 {
                candidate_sequence: draft.candidate_sequence,
                candidate_semantic_digest: draft.candidate_semantic_digest,
                periods,
                total_return_paisa: draft.total_return_paisa,
                total_trades: draft.total_trades,
                total_wins: draft.total_wins,
            });
        }
        let identity = family_identity(source, policy, &self.accepted_ist_sessions, &candidates);
        let family = CandidateFamilyObservationsV1 {
            source: *source,
            policy,
            accepted_ist_sessions: self.accepted_ist_sessions,
            candidates,
            identity,
        };
        family.validate()?;
        Ok(family)
    }

    #[cfg(test)]
    fn from_sessions_for_test(sessions: Vec<i64>) -> Result<Self, String> {
        validate_session_sequence(&sessions)?;
        let first_day = *sessions
            .first()
            .ok_or_else(|| "test observation sessions are empty".to_owned())?;
        let last_day = *sessions
            .last()
            .ok_or_else(|| "test observation sessions are empty".to_owned())?;
        let span = usize::try_from(
            last_day
                .checked_sub(first_day)
                .and_then(|width| width.checked_add(1))
                .ok_or_else(|| "test observation day span overflowed".to_owned())?,
        )
        .map_err(|_| "test observation day span does not fit usize".to_owned())?;
        let mut day_to_period = vec![None; span];
        for (index, day) in sessions.iter().copied().enumerate() {
            let offset = day_offset(first_day, day, span)?;
            *day_to_period
                .get_mut(offset)
                .ok_or_else(|| "test observation session index disappeared".to_owned())? =
                Some(index);
        }
        Ok(Self {
            execution_calendar_first_day: first_day,
            execution_calendar_last_day: last_day,
            execution_calendar_digest: [1; 32],
            first_day,
            accepted_ist_sessions: sessions,
            day_to_period,
            semantics: HashSet::new(),
            candidates: Vec::new(),
        })
    }
}

fn observation_policy_receipt() -> ObservationPolicyReceiptV1 {
    let mut hasher = Hasher::new();
    hasher.update(OBSERVATION_POLICY_DOMAIN);
    hasher.update(&OBSERVATION_SCHEMA_VERSION_V1.to_le_bytes());
    hasher.update(&SESSION_OBSERVATION_POLICY_VERSION_V1.to_le_bytes());
    hasher.update(&OBSERVATION_SCORE_POLICY_VERSION_V1.to_le_bytes());
    hasher.update(
        b"exact-execution-column-accepted-ist-session;exit-session-attribution;explicit-zero-session;cross-day-refusal;checked-sum-trade-row-worst;win-iff-worst-positive",
    );
    ObservationPolicyReceiptV1 {
        schema_version: OBSERVATION_SCHEMA_VERSION_V1,
        session_policy_version: SESSION_OBSERVATION_POLICY_VERSION_V1,
        score_policy_version: OBSERVATION_SCORE_POLICY_VERSION_V1,
        digest: hasher.finalize(),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the period identity binds every exact observation value without an authorable summary"
)]
fn period_identity(
    universe_id: [u8; 32],
    policy_digest: [u8; 32],
    candidate_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    period_sequence: u64,
    exit_ist_day: i64,
    return_paisa: i64,
    trades: u64,
    wins: u64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(PERIOD_ID_DOMAIN);
    hasher.update(&universe_id);
    hasher.update(&policy_digest);
    hasher.update(&candidate_sequence.to_le_bytes());
    hasher.update(&candidate_semantic_digest);
    hasher.update(&period_sequence.to_le_bytes());
    hasher.update(&exit_ist_day.to_le_bytes());
    hasher.update(&return_paisa.to_le_bytes());
    hasher.update(&trades.to_le_bytes());
    hasher.update(&wins.to_le_bytes());
    hasher.finalize()
}

fn family_identity(
    source: &CandidateUniverseReceiptV1,
    policy: ObservationPolicyReceiptV1,
    sessions: &[i64],
    candidates: &[CandidateSessionObservationsV1],
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(FAMILY_ID_DOMAIN);
    hasher.update(&source.universe_id());
    hasher.update(&source.content_digest());
    hasher.update(&source.ordered_row_digest());
    hasher.update(&policy.digest);
    hasher.update(
        &u64::try_from(sessions.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for day in sessions {
        hasher.update(&day.to_le_bytes());
    }
    hasher.update(
        &u64::try_from(candidates.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for candidate in candidates {
        hasher.update(&candidate.candidate_sequence.to_le_bytes());
        hasher.update(&candidate.candidate_semantic_digest);
        hasher.update(&candidate.total_return_paisa.to_le_bytes());
        hasher.update(&candidate.total_trades.to_le_bytes());
        hasher.update(&candidate.total_wins.to_le_bytes());
        for period in &candidate.periods {
            hasher.update(&period.identity);
        }
    }
    hasher.finalize()
}

fn require_pair_sources(
    nifty: &CandidateFamilyObservationsV1,
    banknifty: &CandidateFamilyObservationsV1,
) -> Result<(), String> {
    if nifty.family() != InstrumentFamilyV1::Nifty
        || banknifty.family() != InstrumentFamilyV1::BankNifty
    {
        return Err(
            "paired observations require exact NIFTY then BANKNIFTY family order".to_owned(),
        );
    }
    if nifty.accepted_ist_sessions != banknifty.accepted_ist_sessions {
        return Err(
            "paired observations require an exactly equal ordered accepted-IST-session sequence"
                .to_owned(),
        );
    }
    let left = nifty.source;
    let right = banknifty.source;
    let left_ids = left.identities();
    let right_ids = right.identities();
    if left.rung_seconds() != right.rung_seconds()
        || left.horizon_bars() != right.horizon_bars()
        || left.requested_span() != right.requested_span()
        || left_ids.feed_digest() != right_ids.feed_digest()
        || left_ids.source_commit_digest() != right_ids.source_commit_digest()
        || left_ids.vocabulary_digest() != right_ids.vocabulary_digest()
        || left_ids.evaluation_policy_digest() != right_ids.evaluation_policy_digest()
        || left_ids.calendar_policy_digest() != right_ids.calendar_policy_digest()
        || left_ids.daily_reference_policy_digest() != right_ids.daily_reference_policy_digest()
        || nifty.policy != banknifty.policy
    {
        return Err(
            "paired observations carry mismatched rung, horizon, span, feed, commit, vocabulary, evaluation, calendar, daily-reference or observation policy"
                .to_owned(),
        );
    }
    Ok(())
}

pub(crate) fn derive_layout(period_count: usize) -> Result<CscvLayoutReceiptV1, String> {
    let period_count = u64::try_from(period_count)
        .map_err(|_| "aligned observation period count does not fit u64".to_owned())?;
    let upper = u32::try_from(period_count.min(u64::from(MAX_CSCV_SEGMENTS_V1)))
        .map_err(|_| "CSCV segment search bound does not fit u32".to_owned())?;
    let mut selected = None;
    for segment_count in (2..=upper).rev() {
        if !segment_count.is_multiple_of(2)
            || !period_count.is_multiple_of(u64::from(segment_count))
        {
            continue;
        }
        selected = Some(segment_count);
        break;
    }
    let segment_count = selected.ok_or_else(|| {
        format!(
            "aligned period count {period_count} has no even divisor in 2..={MAX_CSCV_SEGMENTS_V1}; no period may be dropped, padded or defaulted"
        )
    })?;
    let split_count = canonical_split_count(segment_count)?;
    if split_count > MAX_CANONICAL_CSCV_SPLITS_V1 {
        return Err(format!(
            "CSCV segment policy selected {segment_count} segments whose {split_count} splits exceed the structural maximum {MAX_CANONICAL_CSCV_SPLITS_V1}"
        ));
    }
    let periods_per_segment = period_count
        .checked_div(u64::from(segment_count))
        .filter(|width| *width > 0)
        .ok_or_else(|| "CSCV equal segment width is zero".to_owned())?;
    let mut receipt = CscvLayoutReceiptV1 {
        policy_version: CSCV_LAYOUT_POLICY_VERSION_V1,
        period_count,
        segment_count,
        periods_per_segment,
        split_count,
        maximum_segments: MAX_CSCV_SEGMENTS_V1,
        maximum_split_count: MAX_CANONICAL_CSCV_SPLITS_V1,
        digest: [0; 32],
    };
    receipt.digest = layout_digest(receipt);
    Ok(receipt)
}

fn layout_digest(layout: CscvLayoutReceiptV1) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(LAYOUT_POLICY_DOMAIN);
    hasher.update(&layout.policy_version.to_le_bytes());
    hasher.update(&layout.period_count.to_le_bytes());
    hasher.update(&layout.segment_count.to_le_bytes());
    hasher.update(&layout.periods_per_segment.to_le_bytes());
    hasher.update(&layout.split_count.to_le_bytes());
    hasher.update(&layout.maximum_segments.to_le_bytes());
    hasher.update(&layout.maximum_split_count.to_le_bytes());
    hasher.update(
        b"largest-even-divisor;2-through-16;complete-periods;contiguous-equal-blocks;canonical-half-complements;bit-zero-test;ascending-gosper;checked-sum-worst;first-strict-max-tie-policy-in-statistics-v2;no-default-no-truncation-no-padding",
    );
    hasher.finalize()
}

pub(crate) fn canonical_masks(layout: CscvLayoutReceiptV1) -> Result<Vec<(u64, u64)>, String> {
    if layout.digest != layout_digest(layout)
        || layout.split_count != canonical_split_count(layout.segment_count)?
        || layout.period_count
            != layout
                .periods_per_segment
                .checked_mul(u64::from(layout.segment_count))
                .ok_or_else(|| "CSCV layout period multiplication overflowed u64".to_owned())?
    {
        return Err("CSCV layout receipt does not recompute".to_owned());
    }
    let split_count = usize::try_from(layout.split_count)
        .map_err(|_| "CSCV split count does not fit usize".to_owned())?;
    let mut masks = Vec::new();
    masks.try_reserve_exact(split_count).map_err(|why| {
        format!("CSCV canonical masks could not reserve {split_count} rows: {why}")
    })?;
    let full = segment_mask(layout.segment_count)?;
    let mut train = first_train_mask(layout.segment_count)?;
    for sequence in 0..layout.split_count {
        masks.push((train, full ^ train));
        if sequence + 1 < layout.split_count {
            train = next_train_mask(train, layout.segment_count)?;
        }
    }
    Ok(masks)
}

fn append_family_split_scores(
    output: &mut Vec<CandidateSplitScoreV1>,
    family: &CandidateFamilyObservationsV1,
    global_offset: u64,
    layout: CscvLayoutReceiptV1,
    masks: &[(u64, u64)],
    source_identity: [u8; 32],
) -> Result<(), String> {
    append_candidate_split_scores(
        output,
        family.family(),
        &family.candidates,
        global_offset,
        layout,
        masks,
        source_identity,
    )
}

fn append_candidate_split_scores(
    output: &mut Vec<CandidateSplitScoreV1>,
    family: InstrumentFamilyV1,
    candidates: &[CandidateSessionObservationsV1],
    global_offset: u64,
    layout: CscvLayoutReceiptV1,
    masks: &[(u64, u64)],
    source_identity: [u8; 32],
) -> Result<(), String> {
    let block_width = usize::try_from(layout.periods_per_segment)
        .map_err(|_| "CSCV periods per segment does not fit usize".to_owned())?;
    for candidate in candidates {
        let global_candidate_sequence = global_offset
            .checked_add(candidate.candidate_sequence)
            .ok_or_else(|| "global paired candidate sequence overflowed u64".to_owned())?;
        for (split_index, (train_mask, test_mask)) in masks.iter().copied().enumerate() {
            let split_sequence = u64::try_from(split_index)
                .map_err(|_| "CSCV split sequence does not fit u64".to_owned())?;
            let mut train_score = 0_i64;
            let mut test_score = 0_i64;
            for (period_index, period) in candidate.periods.iter().enumerate() {
                let segment = period_index
                    .checked_div(block_width)
                    .ok_or_else(|| "CSCV block width is zero".to_owned())?;
                let segment = u32::try_from(segment)
                    .map_err(|_| "CSCV segment index does not fit u32".to_owned())?;
                let bit = 1_u64
                    .checked_shl(segment)
                    .ok_or_else(|| "CSCV segment bit overflowed u64".to_owned())?;
                if train_mask & bit != 0 {
                    train_score = train_score.checked_add(period.return_paisa).ok_or_else(|| {
                        format!(
                            "CSCV train score overflowed i64 for global candidate {global_candidate_sequence} split {split_sequence}"
                        )
                    })?;
                } else if test_mask & bit != 0 {
                    test_score = test_score.checked_add(period.return_paisa).ok_or_else(|| {
                        format!(
                            "CSCV test score overflowed i64 for global candidate {global_candidate_sequence} split {split_sequence}"
                        )
                    })?;
                } else {
                    return Err(format!(
                        "CSCV segment {segment} belongs to neither train nor test mask"
                    ));
                }
            }
            let identity = split_row_identity(
                source_identity,
                global_candidate_sequence,
                family,
                candidate.candidate_sequence,
                candidate.candidate_semantic_digest,
                split_sequence,
                train_mask,
                test_mask,
                train_score,
                test_score,
            );
            output.push(CandidateSplitScoreV1 {
                global_candidate_sequence,
                family,
                family_candidate_sequence: candidate.candidate_sequence,
                candidate_semantic_digest: candidate.candidate_semantic_digest,
                split_sequence,
                train_mask,
                test_mask,
                train_score_paisa: train_score,
                test_score_paisa: test_score,
                identity,
            });
        }
    }
    Ok(())
}

fn pair_source_identity(
    nifty: &CandidateFamilyObservationsV1,
    banknifty: &CandidateFamilyObservationsV1,
    layout: CscvLayoutReceiptV1,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(PAIR_SOURCE_ID_DOMAIN);
    hasher.update(&nifty.identity);
    hasher.update(&banknifty.identity);
    hasher.update(&nifty.policy.digest);
    hasher.update(&layout.digest);
    hasher.update(b"NIFTY-then-BANKNIFTY");
    hasher.finalize()
}

#[expect(
    clippy::too_many_arguments,
    reason = "the score row identity binds source, candidate, masks and both internally derived scores"
)]
fn split_row_identity(
    source_identity: [u8; 32],
    global_candidate_sequence: u64,
    family: InstrumentFamilyV1,
    family_candidate_sequence: u64,
    candidate_semantic_digest: [u8; 32],
    split_sequence: u64,
    train_mask: u64,
    test_mask: u64,
    train_score: i64,
    test_score: i64,
) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(SPLIT_ROW_ID_DOMAIN);
    hasher.update(&source_identity);
    hasher.update(&global_candidate_sequence.to_le_bytes());
    hasher.update(&[family_byte(family)]);
    hasher.update(&family_candidate_sequence.to_le_bytes());
    hasher.update(&candidate_semantic_digest);
    hasher.update(&split_sequence.to_le_bytes());
    hasher.update(&train_mask.to_le_bytes());
    hasher.update(&test_mask.to_le_bytes());
    hasher.update(&train_score.to_le_bytes());
    hasher.update(&test_score.to_le_bytes());
    hasher.finalize()
}

fn pair_identity(source_identity: [u8; 32], rows: &[CandidateSplitScoreV1]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(PAIR_ID_DOMAIN);
    hasher.update(&source_identity);
    hasher.update(&u64::try_from(rows.len()).unwrap_or(u64::MAX).to_le_bytes());
    for row in rows {
        hasher.update(&row.identity);
    }
    hasher.finalize()
}

fn validate_session_sequence(sessions: &[i64]) -> Result<(), String> {
    if sessions.is_empty() {
        return Err("candidate observation accepted-IST-session sequence is empty".to_owned());
    }
    let mut previous = None;
    for (index, day) in sessions.iter().copied().enumerate() {
        if !matches!(kind_of(day), DayKind::Open(_)) {
            return Err(format!(
                "candidate observation session {index} day {day} is not a measured open IST session"
            ));
        }
        if previous.is_some_and(|earlier| earlier >= day) {
            return Err(format!(
                "candidate observation session {index} day {day} is duplicated or out of order"
            ));
        }
        previous = Some(day);
    }
    Ok(())
}

fn day_offset(first_day: i64, day: i64, span: usize) -> Result<usize, String> {
    let offset = day
        .checked_sub(first_day)
        .ok_or_else(|| "candidate observation day offset overflowed i64".to_owned())?;
    let offset = usize::try_from(offset).map_err(|_| {
        format!("candidate observation IST day {day} precedes first accepted day {first_day}")
    })?;
    if offset >= span {
        return Err(format!(
            "candidate observation IST day {day} is outside indexed span beginning {first_day}"
        ));
    }
    Ok(offset)
}

fn canonical_split_count(segment_count: u32) -> Result<u64, String> {
    validate_segment_count(segment_count)?;
    choose_u64(segment_count - 1, segment_count / 2)
}

fn validate_segment_count(segment_count: u32) -> Result<(), String> {
    if !(2..=64).contains(&segment_count) || !segment_count.is_multiple_of(2) {
        return Err(format!(
            "CSCV segment count {segment_count} is not even in 2..=64"
        ));
    }
    Ok(())
}

fn choose_u64(n: u32, k: u32) -> Result<u64, String> {
    let k = k.min(
        n.checked_sub(k)
            .ok_or_else(|| "CSCV combination k exceeds n".to_owned())?,
    );
    let mut value = 1_u128;
    for index in 0..k {
        value = value
            .checked_mul(u128::from(n - index))
            .ok_or_else(|| "CSCV combination count overflowed u128".to_owned())?
            / u128::from(index + 1);
    }
    u64::try_from(value).map_err(|_| "CSCV combination count exceeds u64".to_owned())
}

fn segment_mask(segment_count: u32) -> Result<u64, String> {
    validate_segment_count(segment_count)?;
    Ok(if segment_count == 64 {
        u64::MAX
    } else {
        1_u64
            .checked_shl(segment_count)
            .and_then(|value| value.checked_sub(1))
            .ok_or_else(|| "CSCV segment mask overflowed".to_owned())?
    })
}

fn first_train_mask(segment_count: u32) -> Result<u64, String> {
    validate_segment_count(segment_count)?;
    let compressed = 1_u64
        .checked_shl(segment_count / 2)
        .and_then(|value| value.checked_sub(1))
        .ok_or_else(|| "CSCV first train mask overflowed".to_owned())?;
    compressed
        .checked_shl(1)
        .ok_or_else(|| "CSCV first train mask overflowed".to_owned())
}

fn next_train_mask(current: u64, segment_count: u32) -> Result<u64, String> {
    validate_segment_count(segment_count)?;
    let compressed = current >> 1;
    let smallest = compressed & compressed.wrapping_neg();
    if smallest == 0 {
        return Err("CSCV train mask has no set bit".to_owned());
    }
    let ripple = compressed
        .checked_add(smallest)
        .ok_or_else(|| "CSCV next-mask ripple overflowed".to_owned())?;
    let ones = ((compressed ^ ripple) >> 2) / smallest;
    let next = ripple | ones;
    let compressed_bits = segment_count - 1;
    if compressed_bits < 64 && next >= (1_u64 << compressed_bits) {
        return Err("CSCV canonical train-mask sequence is exhausted".to_owned());
    }
    next.checked_shl(1)
        .ok_or_else(|| "CSCV next train mask overflowed".to_owned())
}

const fn family_byte(family: InstrumentFamilyV1) -> u8 {
    match family {
        InstrumentFamilyV1::Nifty => 1,
        InstrumentFamilyV1::BankNifty => 2,
    }
}

fn is_zero_digest(digest: [u8; 32]) -> bool {
    digest == [0; 32]
}

/// Explicit resource limits for the fixed-stride observation-authority file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationAuthorityBoundsV1 {
    max_authorities: u64,
    max_file_bytes: u64,
}

impl ObservationAuthorityBoundsV1 {
    /// Creates non-zero authority-count and file-byte ceilings.
    ///
    /// # Errors
    ///
    /// Refuses zero or a file ceiling smaller than one header plus one complete
    /// Data/Completion pair. No implicit default exists.
    pub fn new(max_authorities: u64, max_file_bytes: u64) -> Result<Self, String> {
        let minimum = OBSERVATION_AUTHORITY_HEADER_BYTES_V1
            .checked_add(
                OBSERVATION_AUTHORITY_RECORD_STRIDE_V1
                    .checked_mul(2)
                    .ok_or_else(|| {
                        "observation authority minimum size overflowed u64".to_owned()
                    })?,
            )
            .ok_or_else(|| "observation authority minimum size overflowed u64".to_owned())?;
        if max_authorities == 0 || max_file_bytes < minimum {
            return Err(format!(
                "observation authority bounds require nonzero authorities and at least {minimum} bytes; got {max_authorities}/{max_file_bytes}"
            ));
        }
        Ok(Self {
            max_authorities,
            max_file_bytes,
        })
    }

    /// Maximum completed authorities admitted from disk or by append.
    #[must_use]
    pub const fn max_authorities(self) -> u64 {
        self.max_authorities
    }

    /// Maximum total fixed-stride file bytes admitted.
    #[must_use]
    pub const fn max_file_bytes(self) -> u64 {
        self.max_file_bytes
    }
}

/// Freshly reopened durable observation authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationAuthorityAuditV1 {
    record_sequence: u64,
    authority_id: [u8; 32],
    pair_identity: [u8; 32],
    source_identity: [u8; 32],
    observation_policy_digest: [u8; 32],
    layout_policy_digest: [u8; 32],
    period_count: u64,
    candidate_count: u64,
    split_count: u64,
    split_score_row_count: u64,
    data_record_digest: [u8; 32],
    completion_digest: [u8; 32],
}

impl ObservationAuthorityAuditV1 {
    /// Zero-based Data/Completion pair sequence.
    #[must_use]
    pub const fn record_sequence(self) -> u64 {
        self.record_sequence
    }

    /// Durable authority identity Finalization V2 must bind.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.authority_id
    }

    /// Complete in-memory paired observation identity.
    #[must_use]
    pub const fn pair_identity(self) -> [u8; 32] {
        self.pair_identity
    }

    /// Source-only paired identity.
    #[must_use]
    pub const fn source_identity(self) -> [u8; 32] {
        self.source_identity
    }

    /// Identity of the exact observation and score policy.
    #[must_use]
    pub const fn observation_policy_digest(self) -> [u8; 32] {
        self.observation_policy_digest
    }

    /// Identity of the deterministic equal-block layout policy.
    #[must_use]
    pub const fn layout_policy_digest(self) -> [u8; 32] {
        self.layout_policy_digest
    }

    /// Shared aligned period count.
    #[must_use]
    pub const fn period_count(self) -> u64 {
        self.period_count
    }

    /// Complete NIFTY plus BANKNIFTY candidate count.
    #[must_use]
    pub const fn candidate_count(self) -> u64 {
        self.candidate_count
    }

    /// Canonical complementary-half split count.
    #[must_use]
    pub const fn split_count(self) -> u64 {
        self.split_count
    }

    /// Complete candidate-by-split score-row count.
    #[must_use]
    pub const fn split_score_row_count(self) -> u64 {
        self.split_score_row_count
    }

    /// Digest/seal of the source+policy+layout Data record.
    #[must_use]
    pub const fn data_record_digest(self) -> [u8; 32] {
        self.data_record_digest
    }

    /// Receipt-last completion digest.
    #[must_use]
    pub const fn completion_digest(self) -> [u8; 32] {
        self.completion_digest
    }
}

/// Whether a receipt-last observation authority was appended or exactly reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationAuthorityCommitV1 {
    /// New Data and Completion records were synced, then freshly reopened.
    Written(ObservationAuthorityAuditV1),
    /// Exact existing semantic bytes were freshly reopened without duplication.
    Reused(ObservationAuthorityAuditV1),
}

impl ObservationAuthorityCommitV1 {
    /// Freshly reopened exact audit.
    #[must_use]
    pub const fn audit(self) -> ObservationAuthorityAuditV1 {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationAuthorityDataV1 {
    authority_id: [u8; 32],
    pair_identity: [u8; 32],
    source_identity: [u8; 32],
    observation_policy_digest: [u8; 32],
    layout_policy_digest: [u8; 32],
    nifty_universe_id: [u8; 32],
    banknifty_universe_id: [u8; 32],
    nifty_family_identity: [u8; 32],
    banknifty_family_identity: [u8; 32],
    accepted_session_digest: [u8; 32],
    ordered_period_digest: [u8; 32],
    ordered_split_score_digest: [u8; 32],
    rung_seconds: u32,
    horizon_bars: u32,
    period_count: u64,
    nifty_candidate_count: u64,
    banknifty_candidate_count: u64,
    segment_count: u32,
    observation_schema_version: u32,
    periods_per_segment: u64,
    split_count: u64,
    split_score_row_count: u64,
    maximum_segments: u32,
    record_digest: [u8; 32],
}

impl ObservationAuthorityDataV1 {
    fn from_pair(pair: &PairedCandidateObservationsV1) -> Result<Self, String> {
        pair.nifty.validate()?;
        pair.banknifty.validate()?;
        require_pair_sources(&pair.nifty, &pair.banknifty)?;
        if pair.layout != derive_layout(pair.nifty.period_count())?
            || pair.source_identity
                != pair_source_identity(&pair.nifty, &pair.banknifty, pair.layout)
            || pair.identity != pair_identity(pair.source_identity, &pair.split_scores)
        {
            return Err(
                "paired observation capability does not recompute before durability".to_owned(),
            );
        }
        validate_paired_split_rows(pair)?;
        let nifty_candidate_count = u64::try_from(pair.nifty.candidate_count())
            .map_err(|_| "NIFTY candidate count does not fit u64".to_owned())?;
        let banknifty_candidate_count = u64::try_from(pair.banknifty.candidate_count())
            .map_err(|_| "BANKNIFTY candidate count does not fit u64".to_owned())?;
        let candidate_count = nifty_candidate_count
            .checked_add(banknifty_candidate_count)
            .ok_or_else(|| "paired candidate count overflowed u64".to_owned())?;
        let split_score_row_count = u64::try_from(pair.split_scores.len())
            .map_err(|_| "paired split-score row count does not fit u64".to_owned())?;
        if split_score_row_count
            != candidate_count
                .checked_mul(pair.layout.split_count)
                .ok_or_else(|| "paired split-score count overflowed u64".to_owned())?
        {
            return Err(
                "paired split-score row count does not equal candidates times splits".to_owned(),
            );
        }
        let mut value = Self {
            authority_id: [0; 32],
            pair_identity: pair.identity,
            source_identity: pair.source_identity,
            observation_policy_digest: pair.nifty.policy.digest,
            layout_policy_digest: pair.layout.digest,
            nifty_universe_id: pair.nifty.source.universe_id(),
            banknifty_universe_id: pair.banknifty.source.universe_id(),
            nifty_family_identity: pair.nifty.identity,
            banknifty_family_identity: pair.banknifty.identity,
            accepted_session_digest: digest_sessions(&pair.nifty.accepted_ist_sessions),
            ordered_period_digest: digest_paired_periods(pair),
            ordered_split_score_digest: digest_split_scores(&pair.split_scores),
            rung_seconds: pair.nifty.source.rung_seconds(),
            horizon_bars: pair.nifty.source.horizon_bars(),
            period_count: pair.layout.period_count,
            nifty_candidate_count,
            banknifty_candidate_count,
            segment_count: pair.layout.segment_count,
            observation_schema_version: pair.nifty.policy.schema_version,
            periods_per_segment: pair.layout.periods_per_segment,
            split_count: pair.layout.split_count,
            split_score_row_count,
            maximum_segments: pair.layout.maximum_segments,
            record_digest: [0; 32],
        };
        value.authority_id = authority_id(&value)?;
        value.record_digest = authority_record_digest(&value)?;
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), String> {
        for (name, digest) in [
            ("authority", self.authority_id),
            ("pair", self.pair_identity),
            ("source", self.source_identity),
            ("observation policy", self.observation_policy_digest),
            ("layout policy", self.layout_policy_digest),
            ("NIFTY universe", self.nifty_universe_id),
            ("BANKNIFTY universe", self.banknifty_universe_id),
            ("NIFTY family", self.nifty_family_identity),
            ("BANKNIFTY family", self.banknifty_family_identity),
            ("accepted sessions", self.accepted_session_digest),
            ("ordered periods", self.ordered_period_digest),
            ("ordered split scores", self.ordered_split_score_digest),
            ("record", self.record_digest),
        ] {
            if is_zero_digest(digest) {
                return Err(format!("observation authority {name} digest is zero"));
            }
        }
        if self.rung_seconds == 0
            || self.horizon_bars == 0
            || self.period_count < 2
            || self.nifty_candidate_count == 0
            || self.banknifty_candidate_count == 0
            || self.observation_schema_version != OBSERVATION_SCHEMA_VERSION_V1
            || self.maximum_segments != MAX_CSCV_SEGMENTS_V1
        {
            return Err(
                "observation authority carries zero or foreign source/policy counts".to_owned(),
            );
        }
        if self.observation_policy_digest != observation_policy_receipt().digest {
            return Err(
                "observation authority carries a foreign observation/score policy".to_owned(),
            );
        }
        validate_segment_count(self.segment_count)?;
        if self.segment_count > self.maximum_segments
            || self
                .periods_per_segment
                .checked_mul(u64::from(self.segment_count))
                != Some(self.period_count)
            || canonical_split_count(self.segment_count)? != self.split_count
        {
            return Err("observation authority equal-block layout does not recompute".to_owned());
        }
        let period_count = usize::try_from(self.period_count)
            .map_err(|_| "observation authority period count does not fit usize".to_owned())?;
        let layout = derive_layout(period_count)?;
        if layout.segment_count != self.segment_count
            || layout.periods_per_segment != self.periods_per_segment
            || layout.split_count != self.split_count
            || layout.maximum_segments != self.maximum_segments
            || layout.digest != self.layout_policy_digest
        {
            return Err(
                "observation authority carries a foreign deterministic layout policy".to_owned(),
            );
        }
        let candidate_count = self
            .nifty_candidate_count
            .checked_add(self.banknifty_candidate_count)
            .ok_or_else(|| "observation authority candidate count overflowed u64".to_owned())?;
        if candidate_count.checked_mul(self.split_count) != Some(self.split_score_row_count) {
            return Err("observation authority split-score count does not recompute".to_owned());
        }
        if authority_id(&self)? != self.authority_id
            || authority_record_digest(&self)? != self.record_digest
        {
            return Err(
                "observation authority identity or Data digest does not recompute".to_owned(),
            );
        }
        Ok(())
    }

    fn record(self) -> Result<[u8; AUTHORITY_RECORD_BYTES], String> {
        self.validate()?;
        let mut record = [0_u8; AUTHORITY_RECORD_BYTES];
        let payload = authority_data_payload(&self, false)?;
        record[..AUTHORITY_RECORD_PAYLOAD_BYTES].copy_from_slice(&payload);
        record[AUTHORITY_RECORD_PAYLOAD_BYTES..].copy_from_slice(&self.record_digest);
        Ok(record)
    }

    fn decode(record: &[u8]) -> Result<Self, String> {
        if record.len() != AUTHORITY_RECORD_BYTES {
            return Err("observation authority Data record has foreign stride".to_owned());
        }
        let payload = record
            .get(..AUTHORITY_RECORD_PAYLOAD_BYTES)
            .ok_or_else(|| "observation authority Data payload is absent".to_owned())?;
        if get_fixed::<16>(payload, 0)? != AUTHORITY_DATA_MAGIC
            || get_u32_obs(payload, 16)? != AUTHORITY_FILE_VERSION
            || get_u32_obs(payload, 20)? != AUTHORITY_DATA_KIND
        {
            return Err("observation authority Data magic/version/kind is foreign".to_owned());
        }
        let value = Self {
            authority_id: get_fixed(payload, 24)?,
            pair_identity: get_fixed(payload, 56)?,
            source_identity: get_fixed(payload, 88)?,
            observation_policy_digest: get_fixed(payload, 120)?,
            layout_policy_digest: get_fixed(payload, 152)?,
            nifty_universe_id: get_fixed(payload, 184)?,
            banknifty_universe_id: get_fixed(payload, 216)?,
            nifty_family_identity: get_fixed(payload, 248)?,
            banknifty_family_identity: get_fixed(payload, 280)?,
            accepted_session_digest: get_fixed(payload, 312)?,
            ordered_period_digest: get_fixed(payload, 344)?,
            ordered_split_score_digest: get_fixed(payload, 376)?,
            rung_seconds: get_u32_obs(payload, 408)?,
            horizon_bars: get_u32_obs(payload, 412)?,
            period_count: get_u64_obs(payload, 416)?,
            nifty_candidate_count: get_u64_obs(payload, 424)?,
            banknifty_candidate_count: get_u64_obs(payload, 432)?,
            segment_count: get_u32_obs(payload, 440)?,
            observation_schema_version: get_u32_obs(payload, 444)?,
            periods_per_segment: get_u64_obs(payload, 448)?,
            split_count: get_u64_obs(payload, 456)?,
            split_score_row_count: get_u64_obs(payload, 464)?,
            maximum_segments: get_u32_obs(payload, 472)?,
            record_digest: get_fixed(record, AUTHORITY_RECORD_PAYLOAD_BYTES)?,
        };
        if record
            .get(476..AUTHORITY_RECORD_PAYLOAD_BYTES)
            .is_none_or(|reserved| reserved.iter().any(|byte| *byte != 0))
        {
            return Err("observation authority Data reserved bytes are nonzero".to_owned());
        }
        value.validate()?;
        Ok(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationAuthorityCompletionV1 {
    authority_id: [u8; 32],
    data_record_digest: [u8; 32],
    pair_identity: [u8; 32],
    record_sequence: u64,
    completion_digest: [u8; 32],
}

impl ObservationAuthorityCompletionV1 {
    fn for_data(data: &ObservationAuthorityDataV1, record_sequence: u64) -> Result<Self, String> {
        data.validate()?;
        let mut value = Self {
            authority_id: data.authority_id,
            data_record_digest: data.record_digest,
            pair_identity: data.pair_identity,
            record_sequence,
            completion_digest: [0; 32],
        };
        value.completion_digest = completion_digest(value);
        Ok(value)
    }

    fn validate(self, data: &ObservationAuthorityDataV1) -> Result<(), String> {
        if self.authority_id != data.authority_id
            || self.data_record_digest != data.record_digest
            || self.pair_identity != data.pair_identity
            || self.completion_digest != completion_digest(self)
        {
            return Err("observation authority Completion does not match adjacent Data".to_owned());
        }
        Ok(())
    }

    fn record(self) -> Result<[u8; AUTHORITY_RECORD_BYTES], String> {
        let mut record = [0_u8; AUTHORITY_RECORD_BYTES];
        record[..16].copy_from_slice(&AUTHORITY_COMPLETION_MAGIC);
        put_u32_obs(&mut record, 16, AUTHORITY_FILE_VERSION)?;
        put_u32_obs(&mut record, 20, AUTHORITY_COMPLETION_KIND)?;
        put_fixed(&mut record, 24, &self.authority_id)?;
        put_fixed(&mut record, 56, &self.data_record_digest)?;
        put_fixed(&mut record, 88, &self.pair_identity)?;
        put_u64_obs(&mut record, 120, self.record_sequence)?;
        put_fixed(
            &mut record,
            AUTHORITY_RECORD_PAYLOAD_BYTES,
            &self.completion_digest,
        )?;
        Ok(record)
    }

    fn decode(record: &[u8]) -> Result<Self, String> {
        if record.len() != AUTHORITY_RECORD_BYTES
            || get_fixed::<16>(record, 0)? != AUTHORITY_COMPLETION_MAGIC
            || get_u32_obs(record, 16)? != AUTHORITY_FILE_VERSION
            || get_u32_obs(record, 20)? != AUTHORITY_COMPLETION_KIND
        {
            return Err(
                "observation authority Completion magic/version/kind is foreign".to_owned(),
            );
        }
        if record
            .get(128..AUTHORITY_RECORD_PAYLOAD_BYTES)
            .is_none_or(|reserved| reserved.iter().any(|byte| *byte != 0))
        {
            return Err("observation authority Completion reserved bytes are nonzero".to_owned());
        }
        Ok(Self {
            authority_id: get_fixed(record, 24)?,
            data_record_digest: get_fixed(record, 56)?,
            pair_identity: get_fixed(record, 88)?,
            record_sequence: get_u64_obs(record, 120)?,
            completion_digest: get_fixed(record, AUTHORITY_RECORD_PAYLOAD_BYTES)?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationFileGenerationV1 {
    len: u64,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    modified_seconds: i64,
    #[cfg(unix)]
    modified_nanoseconds: i64,
    #[cfg(unix)]
    changed_seconds: i64,
    #[cfg(unix)]
    changed_nanoseconds: i64,
    #[cfg(not(unix))]
    modified: Option<std::time::SystemTime>,
}

type ObservationAuditIndexV1 = std::collections::HashMap<[u8; 32], ObservationAuthorityAuditV1>;
type ObservationDataIndexV1 = std::collections::HashMap<[u8; 32], ObservationAuthorityDataV1>;
type ObservationScanV1 = (
    ObservationAuditIndexV1,
    ObservationDataIndexV1,
    Option<(u64, ObservationAuthorityDataV1)>,
);

/// Open bounded companion observation-authority ledger.
#[derive(Debug)]
pub struct ObservationAuthorityLedgerV1 {
    lock_path: PathBuf,
    file_path: PathBuf,
    file: File,
    lock_file: File,
    bounds: ObservationAuthorityBoundsV1,
    audits: std::collections::HashMap<[u8; 32], ObservationAuthorityAuditV1>,
    data_by_id: std::collections::HashMap<[u8; 32], ObservationAuthorityDataV1>,
    orphan: Option<(u64, ObservationAuthorityDataV1)>,
    snapshot_digest: [u8; 32],
    lock_generation: ObservationFileGenerationV1,
    file_generation: ObservationFileGenerationV1,
    writable: bool,
}

impl ObservationAuthorityLedgerV1 {
    fn open(root: impl AsRef<Path>, bounds: ObservationAuthorityBoundsV1) -> Result<Self, String> {
        Self::open_inner(root.as_ref(), bounds, true)
    }

    /// Opens an existing companion ledger read-only with explicit bounds.
    ///
    /// # Errors
    ///
    /// Refuses an absent/non-directory root, absent/ragged/corrupt/foreign
    /// file, resource-bound breach, duplicate authority or mismatched receipt.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: ObservationAuthorityBoundsV1,
    ) -> Result<Self, String> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    fn open_inner(
        root: &Path,
        bounds: ObservationAuthorityBoundsV1,
        writable: bool,
    ) -> Result<Self, String> {
        let admitted_root = admit_existing_observation_root(root)?;
        let file_path = admitted_root.join(AUTHORITY_FILE);
        let lock_path = admitted_root.join(AUTHORITY_LOCK_FILE);
        let lock = OpenOptions::new()
            .read(true)
            .write(writable)
            .create(writable)
            .open(&lock_path)
            .map_err(|why| {
                format!(
                    "cannot open observation authority lock {}: {why}",
                    lock_path.display()
                )
            })?;
        if writable {
            lock.lock().map_err(|why| {
                format!(
                    "cannot lock observation authority writer {}: {why}",
                    lock_path.display()
                )
            })?;
        } else {
            lock.lock_shared().map_err(|why| {
                format!(
                    "cannot take shared observation authority lock {}: {why}",
                    lock_path.display()
                )
            })?;
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(writable)
            .create(writable)
            .open(&file_path)
            .map_err(|why| {
                format!(
                    "cannot open observation authority file {}: {why}",
                    file_path.display()
                )
            })?;
        if writable && file.metadata().map_err(|why| why.to_string())?.len() == 0 {
            file.write_all(&authority_header())
                .and_then(|()| file.sync_data())
                .map_err(|why| format!("cannot initialize observation authority file: {why}"))?;
        }
        let bytes = read_bounded_authority_file(&mut file, bounds)?;
        let (audits, data_by_id, orphan) = scan_authority_file(&bytes, bounds)?;
        let snapshot_digest = digest_authority_file(&bytes);
        let lock_generation = observation_file_generation(&lock, &lock_path)?;
        let file_generation = observation_file_generation(&file, &file_path)?;
        Ok(Self {
            lock_path,
            file_path,
            file,
            lock_file: lock,
            bounds,
            audits,
            data_by_id,
            orphan,
            snapshot_digest,
            lock_generation,
            file_generation,
            writable,
        })
    }

    /// Returns one cached audit only after detecting any stale same-length edit.
    ///
    /// # Errors
    ///
    /// Refuses any file mutation or bounded read failure since open.
    pub fn reopen_audit(
        &mut self,
        authority_id: &[u8; 32],
    ) -> Result<Option<ObservationAuthorityAuditV1>, String> {
        self.require_unchanged()?;
        Ok(self.audits.get(authority_id).copied())
    }

    fn append_data(
        &mut self,
        data: &ObservationAuthorityDataV1,
    ) -> Result<ObservationAuthorityCommitV1, String> {
        if !self.writable {
            return Err("read-only observation authority ledger cannot append".to_owned());
        }
        data.validate()?;
        self.require_unchanged()?;
        if let Some(existing) = self.audits.get(&data.authority_id).copied() {
            let existing_data = self
                .data_by_id
                .get(&data.authority_id)
                .copied()
                .ok_or_else(|| {
                    "observation authority audit lost its adjacent Data record".to_owned()
                })?;
            if existing_data != *data {
                return Err(
                    "observation authority identity aliases foreign semantic bytes".to_owned(),
                );
            }
            return Ok(ObservationAuthorityCommitV1::Reused(existing));
        }
        let completed = u64::try_from(self.audits.len())
            .map_err(|_| "observation authority count does not fit u64".to_owned())?;
        if completed >= self.bounds.max_authorities {
            return Err(format!(
                "observation authority count would exceed configured maximum {}",
                self.bounds.max_authorities
            ));
        }
        let record_sequence = if let Some((sequence, orphan)) = self.orphan {
            if orphan != *data {
                return Err(
                    "observation authority trailing Data belongs to a foreign pair".to_owned(),
                );
            }
            sequence
        } else {
            let next_bytes = self
                .file
                .metadata()
                .map_err(|why| format!("cannot stat observation authority file: {why}"))?
                .len()
                .checked_add(
                    OBSERVATION_AUTHORITY_RECORD_STRIDE_V1
                        .checked_mul(2)
                        .ok_or_else(|| {
                            "observation authority append size overflowed u64".to_owned()
                        })?,
                )
                .ok_or_else(|| "observation authority append size overflowed u64".to_owned())?;
            if next_bytes > self.bounds.max_file_bytes {
                return Err(format!(
                    "observation authority append would require {next_bytes} bytes above configured maximum {}",
                    self.bounds.max_file_bytes
                ));
            }
            let sequence = completed;
            let record = data.record()?;
            self.file
                .seek(SeekFrom::End(0))
                .and_then(|_| self.file.write_all(&record))
                .and_then(|()| self.file.sync_data())
                .map_err(|why| format!("cannot sync observation authority Data: {why}"))?;
            self.orphan = Some((sequence, *data));
            sequence
        };
        let completion_next_bytes = self
            .file
            .metadata()
            .map_err(|why| format!("cannot stat observation authority file: {why}"))?
            .len()
            .checked_add(OBSERVATION_AUTHORITY_RECORD_STRIDE_V1)
            .ok_or_else(|| "observation authority Completion size overflowed u64".to_owned())?;
        if completion_next_bytes > self.bounds.max_file_bytes {
            return Err(format!(
                "observation authority Completion would require {completion_next_bytes} bytes above configured maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        let completion = ObservationAuthorityCompletionV1::for_data(data, record_sequence)?;
        let completion_record = completion.record()?;
        self.file
            .seek(SeekFrom::End(0))
            .and_then(|_| self.file.write_all(&completion_record))
            .and_then(|()| self.file.sync_data())
            .map_err(|why| format!("cannot sync observation authority Completion: {why}"))?;
        let audit = audit_of(data, completion);
        self.audits.insert(data.authority_id, audit);
        self.data_by_id.insert(data.authority_id, *data);
        self.orphan = None;
        self.refresh_snapshot()?;
        Ok(ObservationAuthorityCommitV1::Written(audit))
    }

    fn require_unchanged(&mut self) -> Result<(), String> {
        require_observation_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_observation_generation(self.file_generation, &self.file, &self.file_path)?;
        let bytes = read_bounded_authority_file(&mut self.file, self.bounds)?;
        if digest_authority_file(&bytes) != self.snapshot_digest {
            return Err(format!(
                "observation authority file {} changed after open",
                self.file_path.display()
            ));
        }
        Ok(())
    }

    fn refresh_snapshot(&mut self) -> Result<(), String> {
        let bytes = read_bounded_authority_file(&mut self.file, self.bounds)?;
        self.snapshot_digest = digest_authority_file(&bytes);
        self.lock_generation = observation_file_generation(&self.lock_file, &self.lock_path)?;
        self.file_generation = observation_file_generation(&self.file, &self.file_path)?;
        Ok(())
    }
}

impl PairedCandidateObservationsV1 {
    /// Requires a freshly reopened durable audit to bind this complete pair.
    ///
    /// This recomputes the companion Data record from the opaque observations,
    /// derives the receipt-last Completion at the audit's exact record
    /// sequence, and compares every public audit term byte-for-byte.
    ///
    /// # Errors
    ///
    /// Refuses a non-canonical pair or any foreign, incomplete or altered
    /// observation-authority audit.
    pub fn require_reopened_authority(
        &self,
        audit: &ObservationAuthorityAuditV1,
    ) -> Result<(), String> {
        let data = ObservationAuthorityDataV1::from_pair(self)?;
        require_authority_audit_for_data(&data, audit)
    }

    /// Persists this opaque capability Data-first/Completion-last and requires
    /// a fresh read-only reopen before returning success.
    ///
    /// # Errors
    ///
    /// Refuses a non-canonical capability, absent/replaced storage root or
    /// file, bounded-capacity breach, foreign trailing Data, changed bytes,
    /// write/sync/lock failure, or any fresh-reopen audit mismatch.
    pub fn append_and_reopen(
        &self,
        root: impl AsRef<Path>,
        bounds: ObservationAuthorityBoundsV1,
    ) -> Result<ObservationAuthorityCommitV1, String> {
        let data = ObservationAuthorityDataV1::from_pair(self)?;
        let committed = append_authority_data_and_reopen(root.as_ref(), bounds, &data)?;
        self.require_reopened_authority(&committed.audit())?;
        Ok(committed)
    }
}

fn append_authority_data_and_reopen(
    root: &Path,
    bounds: ObservationAuthorityBoundsV1,
    data: &ObservationAuthorityDataV1,
) -> Result<ObservationAuthorityCommitV1, String> {
    let mut ledger = ObservationAuthorityLedgerV1::open(root, bounds)?;
    let committed = ledger.append_data(data)?;
    let expected = committed.audit();
    drop(ledger);
    let mut reopened = ObservationAuthorityLedgerV1::open_read(root, bounds)?;
    let audit = reopened
        .reopen_audit(&expected.authority_id)?
        .ok_or_else(|| "observation authority disappeared after receipt-last append".to_owned())?;
    if audit != expected {
        return Err("observation authority fresh reopen differs from written bytes".to_owned());
    }
    require_authority_audit_for_data(data, &audit)?;
    Ok(match committed {
        ObservationAuthorityCommitV1::Written(_) => ObservationAuthorityCommitV1::Written(audit),
        ObservationAuthorityCommitV1::Reused(_) => ObservationAuthorityCommitV1::Reused(audit),
    })
}

fn require_authority_audit_for_data(
    data: &ObservationAuthorityDataV1,
    audit: &ObservationAuthorityAuditV1,
) -> Result<(), String> {
    let completion = ObservationAuthorityCompletionV1::for_data(data, audit.record_sequence)?;
    let expected = audit_of(data, completion);
    if *audit != expected {
        return Err(
            "observation authority audit does not bind the exact paired observations".to_owned(),
        );
    }
    Ok(())
}

fn validate_paired_split_rows(pair: &PairedCandidateObservationsV1) -> Result<(), String> {
    let masks = canonical_masks(pair.layout)?;
    let mut rebuilt = Vec::new();
    rebuilt
        .try_reserve_exact(pair.split_scores.len())
        .map_err(|why| format!("cannot reserve paired split verification rows: {why}"))?;
    append_family_split_scores(
        &mut rebuilt,
        &pair.nifty,
        0,
        pair.layout,
        &masks,
        pair.source_identity,
    )?;
    let offset = u64::try_from(pair.nifty.candidate_count())
        .map_err(|_| "NIFTY candidate count does not fit u64".to_owned())?;
    append_family_split_scores(
        &mut rebuilt,
        &pair.banknifty,
        offset,
        pair.layout,
        &masks,
        pair.source_identity,
    )?;
    if rebuilt != pair.split_scores {
        return Err("paired observation split rows do not recompute from exact periods".to_owned());
    }
    Ok(())
}

fn authority_data_payload(
    value: &ObservationAuthorityDataV1,
    zero_authority: bool,
) -> Result<[u8; AUTHORITY_RECORD_PAYLOAD_BYTES], String> {
    let mut payload = [0_u8; AUTHORITY_RECORD_PAYLOAD_BYTES];
    payload[..16].copy_from_slice(&AUTHORITY_DATA_MAGIC);
    put_u32_obs(&mut payload, 16, AUTHORITY_FILE_VERSION)?;
    put_u32_obs(&mut payload, 20, AUTHORITY_DATA_KIND)?;
    put_fixed(
        &mut payload,
        24,
        if zero_authority {
            &[0; 32]
        } else {
            &value.authority_id
        },
    )?;
    put_fixed(&mut payload, 56, &value.pair_identity)?;
    put_fixed(&mut payload, 88, &value.source_identity)?;
    put_fixed(&mut payload, 120, &value.observation_policy_digest)?;
    put_fixed(&mut payload, 152, &value.layout_policy_digest)?;
    put_fixed(&mut payload, 184, &value.nifty_universe_id)?;
    put_fixed(&mut payload, 216, &value.banknifty_universe_id)?;
    put_fixed(&mut payload, 248, &value.nifty_family_identity)?;
    put_fixed(&mut payload, 280, &value.banknifty_family_identity)?;
    put_fixed(&mut payload, 312, &value.accepted_session_digest)?;
    put_fixed(&mut payload, 344, &value.ordered_period_digest)?;
    put_fixed(&mut payload, 376, &value.ordered_split_score_digest)?;
    put_u32_obs(&mut payload, 408, value.rung_seconds)?;
    put_u32_obs(&mut payload, 412, value.horizon_bars)?;
    put_u64_obs(&mut payload, 416, value.period_count)?;
    put_u64_obs(&mut payload, 424, value.nifty_candidate_count)?;
    put_u64_obs(&mut payload, 432, value.banknifty_candidate_count)?;
    put_u32_obs(&mut payload, 440, value.segment_count)?;
    put_u32_obs(&mut payload, 444, value.observation_schema_version)?;
    put_u64_obs(&mut payload, 448, value.periods_per_segment)?;
    put_u64_obs(&mut payload, 456, value.split_count)?;
    put_u64_obs(&mut payload, 464, value.split_score_row_count)?;
    put_u32_obs(&mut payload, 472, value.maximum_segments)?;
    Ok(payload)
}

fn authority_id(value: &ObservationAuthorityDataV1) -> Result<[u8; 32], String> {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_ID_DOMAIN);
    hasher.update(&authority_data_payload(value, true)?);
    Ok(hasher.finalize())
}

fn authority_record_digest(value: &ObservationAuthorityDataV1) -> Result<[u8; 32], String> {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_RECORD_SEAL_DOMAIN);
    hasher.update(&authority_data_payload(value, false)?);
    Ok(hasher.finalize())
}

fn completion_digest(value: ObservationAuthorityCompletionV1) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_COMPLETION_SEAL_DOMAIN);
    hasher.update(&value.authority_id);
    hasher.update(&value.data_record_digest);
    hasher.update(&value.pair_identity);
    hasher.update(&value.record_sequence.to_le_bytes());
    hasher.finalize()
}

fn audit_of(
    data: &ObservationAuthorityDataV1,
    completion: ObservationAuthorityCompletionV1,
) -> ObservationAuthorityAuditV1 {
    ObservationAuthorityAuditV1 {
        record_sequence: completion.record_sequence,
        authority_id: data.authority_id,
        pair_identity: data.pair_identity,
        source_identity: data.source_identity,
        observation_policy_digest: data.observation_policy_digest,
        layout_policy_digest: data.layout_policy_digest,
        period_count: data.period_count,
        candidate_count: data
            .nifty_candidate_count
            .saturating_add(data.banknifty_candidate_count),
        split_count: data.split_count,
        split_score_row_count: data.split_score_row_count,
        data_record_digest: data.record_digest,
        completion_digest: completion.completion_digest,
    }
}

fn authority_header() -> [u8; AUTHORITY_HEADER_BYTES] {
    let mut header = [0_u8; AUTHORITY_HEADER_BYTES];
    header[..16].copy_from_slice(&AUTHORITY_HEADER_MAGIC);
    header[16..20].copy_from_slice(&AUTHORITY_FILE_VERSION.to_le_bytes());
    header[20..24].copy_from_slice(&AUTHORITY_RECORD_BYTES_U32.to_le_bytes());
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_FILE_HEADER_DOMAIN);
    hasher.update(&header[..32]);
    header[32..].copy_from_slice(&hasher.finalize());
    header
}

fn scan_authority_file(
    bytes: &[u8],
    bounds: ObservationAuthorityBoundsV1,
) -> Result<ObservationScanV1, String> {
    if bytes.get(..AUTHORITY_HEADER_BYTES) != Some(authority_header().as_slice()) {
        return Err("observation authority file header is absent or corrupt".to_owned());
    }
    let body = bytes
        .get(AUTHORITY_HEADER_BYTES..)
        .ok_or_else(|| "observation authority file body is absent".to_owned())?;
    if !body.len().is_multiple_of(AUTHORITY_RECORD_BYTES) {
        return Err("observation authority fixed-stride file is ragged".to_owned());
    }
    let record_count = body.len() / AUTHORITY_RECORD_BYTES;
    let maximum_records = bounds
        .max_authorities
        .checked_mul(2)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| "observation authority record bound overflowed u64".to_owned())?;
    if u64::try_from(record_count).map_err(|_| "record count does not fit u64".to_owned())?
        > maximum_records
    {
        return Err("observation authority record count exceeds configured maximum".to_owned());
    }
    let mut audits = std::collections::HashMap::new();
    let mut data_by_id = std::collections::HashMap::new();
    audits
        .try_reserve(
            usize::try_from(bounds.max_authorities)
                .unwrap_or(usize::MAX)
                .min(record_count / 2),
        )
        .map_err(|why| format!("cannot reserve observation authority index: {why}"))?;
    data_by_id
        .try_reserve(audits.capacity())
        .map_err(|why| format!("cannot reserve observation authority data index: {why}"))?;
    let mut pending = None;
    for (physical, record) in body.chunks_exact(AUTHORITY_RECORD_BYTES).enumerate() {
        let kind = get_u32_obs(record, 20)?;
        match kind {
            AUTHORITY_DATA_KIND => {
                if pending.is_some() {
                    return Err(
                        "observation authority file contains consecutive Data records".to_owned(),
                    );
                }
                let sequence = u64::try_from(physical / 2)
                    .map_err(|_| "observation authority sequence does not fit u64".to_owned())?;
                pending = Some((sequence, ObservationAuthorityDataV1::decode(record)?));
            }
            AUTHORITY_COMPLETION_KIND => {
                let (sequence, data) = pending.take().ok_or_else(|| {
                    "observation authority Completion has no adjacent Data".to_owned()
                })?;
                let completion = ObservationAuthorityCompletionV1::decode(record)?;
                if completion.record_sequence != sequence {
                    return Err("observation authority Completion sequence is foreign".to_owned());
                }
                completion.validate(&data)?;
                if audits
                    .insert(data.authority_id, audit_of(&data, completion))
                    .is_some()
                    || data_by_id.insert(data.authority_id, data).is_some()
                {
                    return Err("observation authority identity is duplicated".to_owned());
                }
            }
            other => {
                return Err(format!(
                    "observation authority record kind {other} is unknown"
                ));
            }
        }
    }
    Ok((audits, data_by_id, pending))
}

fn admit_existing_observation_root(root: &Path) -> Result<PathBuf, String> {
    let metadata = std::fs::metadata(root).map_err(|why| {
        format!(
            "observation authority root {} must already exist and be a directory: {why}",
            root.display()
        )
    })?;
    if !metadata.is_dir() {
        return Err(format!(
            "observation authority root {} is not a directory",
            root.display()
        ));
    }
    root.canonicalize().map_err(|why| {
        format!(
            "observation authority root {} cannot be admitted canonically: {why}",
            root.display()
        )
    })
}

fn observation_file_generation(
    file: &File,
    path: &Path,
) -> Result<ObservationFileGenerationV1, String> {
    let held = file.metadata().map_err(|why| {
        format!(
            "cannot stat opened observation authority file {}: {why}",
            path.display()
        )
    })?;
    let named = std::fs::metadata(path).map_err(|why| {
        format!(
            "cannot stat named observation authority file {}: {why}",
            path.display()
        )
    })?;
    let held_generation = observation_generation_of(&held);
    let named_generation = observation_generation_of(&named);
    #[cfg(unix)]
    if (held_generation.device, held_generation.inode)
        != (named_generation.device, named_generation.inode)
    {
        return Err(format!(
            "{} no longer names the opened observation authority file",
            path.display()
        ));
    }
    if held_generation != named_generation {
        return Err(format!(
            "{} changed while its observation authority generation was measured",
            path.display()
        ));
    }
    Ok(held_generation)
}

#[cfg(unix)]
fn observation_generation_of(metadata: &std::fs::Metadata) -> ObservationFileGenerationV1 {
    ObservationFileGenerationV1 {
        len: metadata.len(),
        device: metadata.dev(),
        inode: metadata.ino(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    }
}

#[cfg(not(unix))]
fn observation_generation_of(metadata: &std::fs::Metadata) -> ObservationFileGenerationV1 {
    ObservationFileGenerationV1 {
        len: metadata.len(),
        modified: metadata.modified().ok(),
    }
}

fn require_observation_generation(
    expected: ObservationFileGenerationV1,
    file: &File,
    path: &Path,
) -> Result<(), String> {
    let observed = observation_file_generation(file, path)?;
    if observed == expected {
        Ok(())
    } else {
        Err(format!(
            "observation authority file {} changed since open; cached audit is stale",
            path.display()
        ))
    }
}

fn read_bounded_authority_file(
    file: &mut File,
    bounds: ObservationAuthorityBoundsV1,
) -> Result<Vec<u8>, String> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat observation authority file: {why}"))?
        .len();
    if len > bounds.max_file_bytes {
        return Err(format!(
            "observation authority file is {len} bytes above configured maximum {}",
            bounds.max_file_bytes
        ));
    }
    let len = usize::try_from(len)
        .map_err(|_| "observation authority file length does not fit usize".to_owned())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|why| format!("cannot reserve {len} observation authority bytes: {why}"))?;
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_to_end(&mut bytes))
        .map_err(|why| format!("cannot read observation authority file: {why}"))?;
    if bytes.len() != len {
        return Err("observation authority file changed length during bounded read".to_owned());
    }
    Ok(bytes)
}

fn digest_authority_file(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_FILE_DIGEST_DOMAIN);
    hasher.update(bytes);
    hasher.finalize()
}

fn digest_sessions(sessions: &[i64]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"brutex-candidate-observation-sessions-v1\0");
    hasher.update(
        &u64::try_from(sessions.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for day in sessions {
        hasher.update(&day.to_le_bytes());
    }
    hasher.finalize()
}

fn digest_paired_periods(pair: &PairedCandidateObservationsV1) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"brutex-candidate-observation-periods-v1\0");
    for family in [&pair.nifty, &pair.banknifty] {
        hasher.update(&[family_byte(family.family())]);
        for candidate in &family.candidates {
            for period in &candidate.periods {
                hasher.update(&period.identity);
            }
        }
    }
    hasher.finalize()
}

fn digest_split_scores(rows: &[CandidateSplitScoreV1]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(b"brutex-candidate-observation-split-scores-v1\0");
    hasher.update(&u64::try_from(rows.len()).unwrap_or(u64::MAX).to_le_bytes());
    for row in rows {
        hasher.update(&row.identity);
    }
    hasher.finalize()
}

fn put_fixed<const N: usize>(
    bytes: &mut [u8],
    offset: usize,
    value: &[u8; N],
) -> Result<(), String> {
    bytes
        .get_mut(offset..offset.saturating_add(N))
        .ok_or_else(|| {
            format!(
                "observation authority write {offset}..{} is out of bounds",
                offset.saturating_add(N)
            )
        })?
        .copy_from_slice(value);
    Ok(())
}

fn get_fixed<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], String> {
    bytes
        .get(offset..offset.saturating_add(N))
        .ok_or_else(|| {
            format!(
                "observation authority read {offset}..{} is out of bounds",
                offset.saturating_add(N)
            )
        })?
        .try_into()
        .map_err(|_| "observation authority fixed-width decode failed".to_owned())
}

fn put_u32_obs(bytes: &mut [u8], offset: usize, value: u32) -> Result<(), String> {
    put_fixed(bytes, offset, &value.to_le_bytes())
}

fn put_u64_obs(bytes: &mut [u8], offset: usize, value: u64) -> Result<(), String> {
    put_fixed(bytes, offset, &value.to_le_bytes())
}

fn get_u32_obs(bytes: &[u8], offset: usize) -> Result<u32, String> {
    Ok(u32::from_le_bytes(get_fixed(bytes, offset)?))
}

fn get_u64_obs(bytes: &[u8], offset: usize) -> Result<u64, String> {
    Ok(u64::from_le_bytes(get_fixed(bytes, offset)?))
}

/// Terminal meaning of one Observation V2 family authority.
///
/// V2 deliberately has no fabricated `Observed` or statistics state. Its sole
/// disposition records that the exact Candidate frontier naturally became
/// empty before any candidate observation row could exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationAuthorityDispositionV2 {
    /// Candidate V1 proved natural extinction, complete closure and zero rows.
    NaturallyExtinct,
}

impl ObservationAuthorityDispositionV2 {
    const fn byte(self) -> u32 {
        match self {
            Self::NaturallyExtinct => AUTHORITY_V2_DISPOSITION_NATURAL_EXTINCTION,
        }
    }

    fn from_byte(value: u32) -> Result<Self, String> {
        match value {
            AUTHORITY_V2_DISPOSITION_NATURAL_EXTINCTION => Ok(Self::NaturallyExtinct),
            _ => Err(format!(
                "Observation V2 disposition {value} is unknown; no statistics fallback exists"
            )),
        }
    }
}

/// Explicit authority-count and file-byte ceilings for Observation V2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationAuthorityBoundsV2 {
    max_authorities: u64,
    max_file_bytes: u64,
}

impl ObservationAuthorityBoundsV2 {
    /// Creates nonzero bounds large enough for one Data/Completion pair.
    ///
    /// # Errors
    ///
    /// Refuses zero authorities or a byte ceiling smaller than one complete
    /// receipt-last authority. The ceiling is a refusal bound, not truncation.
    pub fn new(max_authorities: u64, max_file_bytes: u64) -> Result<Self, String> {
        let minimum = OBSERVATION_AUTHORITY_HEADER_BYTES_V2
            .checked_add(
                OBSERVATION_AUTHORITY_RECORD_STRIDE_V2
                    .checked_mul(2)
                    .ok_or_else(|| "Observation V2 minimum size overflowed u64".to_owned())?,
            )
            .ok_or_else(|| "Observation V2 minimum size overflowed u64".to_owned())?;
        if max_authorities == 0 || max_file_bytes < minimum {
            return Err(format!(
                "Observation V2 bounds require nonzero authorities and at least {minimum} bytes; got {max_authorities}/{max_file_bytes}"
            ));
        }
        Ok(Self {
            max_authorities,
            max_file_bytes,
        })
    }

    /// Maximum completed authorities admitted from disk or by append.
    #[must_use]
    pub const fn max_authorities(self) -> u64 {
        self.max_authorities
    }

    /// Maximum total fixed-stride file bytes admitted.
    #[must_use]
    pub const fn max_file_bytes(self) -> u64 {
        self.max_file_bytes
    }
}

/// Freshly reopened zero-family Observation V2 authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationAuthorityAuditV2 {
    record_sequence: u64,
    authority_id: [u8; 32],
    pre_admission_authority_id: [u8; 32],
    pre_admission_record_index: u64,
    candidate_universe_id: [u8; 32],
    candidate_completion_digest: [u8; 32],
    family: InstrumentFamilyV1,
    rung_seconds: u32,
    observation_row_count: u64,
    disposition: ObservationAuthorityDispositionV2,
    policy_digest: [u8; 32],
    data_record_digest: [u8; 32],
    completion_digest: [u8; 32],
}

impl ObservationAuthorityAuditV2 {
    /// Zero-based Data/Completion pair sequence in the V2 file.
    #[must_use]
    pub const fn record_sequence(self) -> u64 {
        self.record_sequence
    }

    /// Domain-separated identity of the complete V2 authority.
    #[must_use]
    pub const fn authority_id(self) -> [u8; 32] {
        self.authority_id
    }

    /// Exact Pre-Admission Data V2 authority embedded by this record.
    #[must_use]
    pub const fn pre_admission_authority_id(self) -> [u8; 32] {
        self.pre_admission_authority_id
    }

    /// Physical Pre-Admission V2 Data-record index authenticated at production.
    #[must_use]
    pub const fn pre_admission_record_index(self) -> u64 {
        self.pre_admission_record_index
    }

    /// Candidate Universe identity sealed inside the embedded source.
    #[must_use]
    pub const fn candidate_universe_id(self) -> [u8; 32] {
        self.candidate_universe_id
    }

    /// Candidate receipt digest sealed inside the embedded source.
    #[must_use]
    pub const fn candidate_completion_digest(self) -> [u8; 32] {
        self.candidate_completion_digest
    }

    /// Exact naturally extinct index family.
    #[must_use]
    pub const fn family(self) -> InstrumentFamilyV1 {
        self.family
    }

    /// Exact signal timeframe in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }

    /// Exact number of candidate observation rows; V2 requires zero.
    #[must_use]
    pub const fn observation_row_count(self) -> u64 {
        self.observation_row_count
    }

    /// Terminal zero-family disposition.
    #[must_use]
    pub const fn disposition(self) -> ObservationAuthorityDispositionV2 {
        self.disposition
    }

    /// Versioned Observation V2 policy identity.
    #[must_use]
    pub const fn policy_digest(self) -> [u8; 32] {
        self.policy_digest
    }

    /// Digest sealing the Data record.
    #[must_use]
    pub const fn data_record_digest(self) -> [u8; 32] {
        self.data_record_digest
    }

    /// Digest sealing the adjacent receipt-last Completion.
    #[must_use]
    pub const fn completion_digest(self) -> [u8; 32] {
        self.completion_digest
    }
}

/// Whether an Observation V2 authority was written or exactly reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationAuthorityCommitV2 {
    /// Data and then Completion were synchronized and freshly reopened.
    Written(ObservationAuthorityAuditV2),
    /// Exact semantic bytes already existed and were freshly reopened.
    Reused(ObservationAuthorityAuditV2),
}

impl ObservationAuthorityCommitV2 {
    /// Freshly reopened exact audit.
    #[must_use]
    pub const fn audit(self) -> ObservationAuthorityAuditV2 {
        match self {
            Self::Written(audit) | Self::Reused(audit) => audit,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ObservationAuthorityDataV2 {
    record_sequence: u64,
    authority_id: [u8; 32],
    pre_admission_record_index: u64,
    observation_row_count: u64,
    disposition: ObservationAuthorityDispositionV2,
    policy_digest: [u8; 32],
    source: PreAdmissionDataV2,
    source_record: [u8; PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2],
}

impl ObservationAuthorityDataV2 {
    fn from_authenticated_source(
        pre_admission_record_index: u64,
        source: &PreAdmissionDataV2,
        source_record: &[u8; PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2],
    ) -> Result<Self, String> {
        let mut value = Self {
            record_sequence: 0,
            authority_id: [0; 32],
            pre_admission_record_index,
            observation_row_count: 0,
            disposition: ObservationAuthorityDispositionV2::NaturallyExtinct,
            policy_digest: observation_v2_policy_digest(),
            source: *source,
            source_record: *source_record,
        };
        value.authority_id = observation_v2_authority_id(&value);
        value.validate()?;
        Ok(value)
    }

    const fn with_sequence(mut self, record_sequence: u64) -> Self {
        self.record_sequence = record_sequence;
        self
    }

    fn same_semantics(self, other: Self) -> bool {
        self.with_sequence(0) == other.with_sequence(0)
    }

    fn validate(self) -> Result<(), String> {
        let decoded = decode_observation_source_v2(&self.source_record).map_err(|why| {
            format!("Observation V2 embedded Pre-Admission source refused: {why}")
        })?;
        if decoded != self.source {
            return Err(
                "Observation V2 embedded bytes differ from decoded Pre-Admission source".to_owned(),
            );
        }
        let proof = self.source.candidate_reconciliation();
        if self.source.candidate_row_count() != 0
            || self.observation_row_count != 0
            || self.disposition != ObservationAuthorityDispositionV2::NaturallyExtinct
            || !proof.extinction_complete
            || !proof.closure_complete
            || (proof.extinction_depth == 0 && proof.frequent_itemsets != 0)
            || proof.unknown_closure_itemsets != 0
            || proof.closed_itemsets != 0
        {
            return Err(
                "Observation V2 requires exact zero rows with natural extinction, complete closure and no unknown or closed masks"
                    .to_owned(),
            );
        }
        if self.policy_digest != observation_v2_policy_digest() {
            return Err("Observation V2 carries a foreign policy identity".to_owned());
        }
        if self.source.authority_id() == [0; 32]
            || self.source.candidate_universe_id() == [0; 32]
            || self.source.candidate_completion_digest() == [0; 32]
            || self.authority_id != observation_v2_authority_id(&self)
        {
            return Err(
                "Observation V2 source or authority identity does not recompute".to_owned(),
            );
        }
        Ok(())
    }

    fn record(self, kind: u32) -> Result<[u8; AUTHORITY_V2_RECORD_BYTES], String> {
        self.validate()?;
        if !matches!(kind, AUTHORITY_V2_DATA_KIND | AUTHORITY_V2_COMPLETION_KIND) {
            return Err("Observation V2 record kind is foreign".to_owned());
        }
        let mut raw = [0_u8; AUTHORITY_V2_RECORD_BYTES];
        raw[..16].copy_from_slice(&AUTHORITY_V2_RECORD_MAGIC);
        put_u32_obs(&mut raw, 16, OBSERVATION_AUTHORITY_SCHEMA_VERSION_V2)?;
        put_u32_obs(&mut raw, 20, kind)?;
        put_u64_obs(&mut raw, 24, self.record_sequence)?;
        put_fixed(&mut raw, 32, &self.authority_id)?;
        put_u64_obs(&mut raw, 64, self.pre_admission_record_index)?;
        put_u64_obs(&mut raw, 72, self.observation_row_count)?;
        put_u32_obs(&mut raw, 80, self.disposition.byte())?;
        put_u32_obs(&mut raw, 84, OBSERVATION_AUTHORITY_SCHEMA_VERSION_V2)?;
        put_fixed(&mut raw, 88, &self.policy_digest)?;
        raw.get_mut(AUTHORITY_V2_SOURCE_OFFSET..AUTHORITY_V2_SOURCE_END)
            .ok_or_else(|| "Observation V2 embedded source range is absent".to_owned())?
            .copy_from_slice(&self.source_record);
        let seal = observation_v2_record_digest(&raw[..AUTHORITY_V2_PAYLOAD_BYTES]);
        raw[AUTHORITY_V2_PAYLOAD_BYTES..].copy_from_slice(&seal);
        Ok(raw)
    }

    fn decode(record: &[u8]) -> Result<(u32, Self), String> {
        if record.len() != AUTHORITY_V2_RECORD_BYTES
            || get_fixed::<16>(record, 0)? != AUTHORITY_V2_RECORD_MAGIC
            || get_u32_obs(record, 16)? != OBSERVATION_AUTHORITY_SCHEMA_VERSION_V2
        {
            return Err("Observation V2 record magic/version/stride is foreign".to_owned());
        }
        let kind = get_u32_obs(record, 20)?;
        if !matches!(kind, AUTHORITY_V2_DATA_KIND | AUTHORITY_V2_COMPLETION_KIND) {
            return Err("Observation V2 record kind is unknown".to_owned());
        }
        if record
            .get(AUTHORITY_V2_SOURCE_END..AUTHORITY_V2_PAYLOAD_BYTES)
            .is_none_or(|reserved| reserved.iter().any(|byte| *byte != 0))
        {
            return Err("Observation V2 reserved bytes are nonzero".to_owned());
        }
        let observed_seal = get_fixed::<32>(record, AUTHORITY_V2_PAYLOAD_BYTES)?;
        let expected_seal = observation_v2_record_digest(
            record
                .get(..AUTHORITY_V2_PAYLOAD_BYTES)
                .ok_or_else(|| "Observation V2 payload is absent".to_owned())?,
        );
        if observed_seal != expected_seal {
            return Err("Observation V2 record seal does not match its payload".to_owned());
        }
        let source_record = get_fixed::<PRE_ADMISSION_OBSERVATION_SOURCE_BYTES_V2>(
            record,
            AUTHORITY_V2_SOURCE_OFFSET,
        )?;
        let source = decode_observation_source_v2(&source_record).map_err(|why| {
            format!("Observation V2 embedded Pre-Admission source refused: {why}")
        })?;
        let value = Self {
            record_sequence: get_u64_obs(record, 24)?,
            authority_id: get_fixed(record, 32)?,
            pre_admission_record_index: get_u64_obs(record, 64)?,
            observation_row_count: get_u64_obs(record, 72)?,
            disposition: ObservationAuthorityDispositionV2::from_byte(get_u32_obs(record, 80)?)?,
            policy_digest: get_fixed(record, 88)?,
            source,
            source_record,
        };
        if get_u32_obs(record, 84)? != OBSERVATION_AUTHORITY_SCHEMA_VERSION_V2 {
            return Err("Observation V2 policy schema is foreign".to_owned());
        }
        value.validate()?;
        Ok((kind, value))
    }
}

/// Opaque production preparation for one naturally extinct family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProducedObservationAuthorityV2 {
    value: ObservationAuthorityDataV2,
}

/// Opaque, revalidated natural-extinction source for Statistics V3.
///
/// This value is crate-private so no public caller can pair a detached audit
/// with caller-authored source fields.  It is minted only after the supplied
/// receipt-last commit is proven to be the exact commit of the produced V2
/// authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ObservationNaturalExtinctionSourceV2 {
    audit: ObservationAuthorityAuditV2,
    source: PreAdmissionDataV2,
}

impl ObservationNaturalExtinctionSourceV2 {
    pub(crate) const fn audit(self) -> ObservationAuthorityAuditV2 {
        self.audit
    }

    pub(crate) const fn source(self) -> PreAdmissionDataV2 {
        self.source
    }
}

impl ProducedObservationAuthorityV2 {
    /// Authenticates the exact receipt-last V2 commit and returns an opaque
    /// natural-extinction projection for Statistics V3.
    pub(crate) fn authenticated_statistics_v3_source(
        &self,
        commit: &ObservationAuthorityCommitV2,
    ) -> Result<ObservationNaturalExtinctionSourceV2, String> {
        self.value.validate()?;
        let audit = commit.audit();
        let expected = observation_v2_audit(&self.value.with_sequence(audit.record_sequence()))?;
        if audit != expected {
            return Err(
                "Statistics V3 received an Observation V2 commit from a foreign production"
                    .to_owned(),
            );
        }
        Ok(ObservationNaturalExtinctionSourceV2 {
            audit,
            source: self.value.source,
        })
    }

    /// Persists Data first and Completion last, then requires a fresh read-only
    /// reopen before returning success.
    pub(crate) fn append_and_reopen(
        &self,
        root: impl AsRef<Path>,
        bounds: ObservationAuthorityBoundsV2,
    ) -> Result<ObservationAuthorityCommitV2, String> {
        let root = root.as_ref();
        let mut ledger = ObservationAuthorityLedgerV2::open(root, bounds)?;
        let committed = ledger.append_data(&self.value)?;
        let expected = committed.audit();
        drop(ledger);
        let mut reopened = ObservationAuthorityLedgerV2::open_read(root, bounds)?;
        let audit = reopened
            .reopen_audit(&self.value.authority_id)?
            .ok_or_else(|| "Observation V2 authority disappeared after append".to_owned())?;
        if audit != expected {
            return Err("Observation V2 fresh reopen differs from written bytes".to_owned());
        }
        Ok(match committed {
            ObservationAuthorityCommitV2::Written(_) => {
                ObservationAuthorityCommitV2::Written(audit)
            }
            ObservationAuthorityCommitV2::Reused(_) => ObservationAuthorityCommitV2::Reused(audit),
        })
    }
}

/// Derives the zero-family Observation V2 authority from one exact opaque
/// Pre-Admission V2 production and its own receipt-last commit.
///
/// The function accepts no row count, extinction flag, digest, family,
/// statistic or fallback from its caller. Nonzero families refuse and must use
/// the exact-observation path rather than being mislabeled extinct.
pub(crate) fn produce_natural_extinction_observation_v2(
    pre_admission: &ProducedPreAdmissionDataV2,
    commit: &PreAdmissionProductionCommitV2,
) -> Result<ProducedObservationAuthorityV2, String> {
    let (audit, source_record) = pre_admission.authenticated_observation_source(commit)?;
    let source = audit.value();
    if source.candidate_row_count() != 0 {
        return Err(
            "Observation V2 natural-extinction producer received a nonzero Candidate family"
                .to_owned(),
        );
    }
    Ok(ProducedObservationAuthorityV2 {
        value: ObservationAuthorityDataV2::from_authenticated_source(
            audit.data_record_index(),
            &source,
            &source_record,
        )?,
    })
}

/// Open, bounded Observation V2 audit ledger.
#[derive(Debug)]
pub struct ObservationAuthorityLedgerV2 {
    lock_path: PathBuf,
    file_path: PathBuf,
    file: File,
    lock_file: File,
    bounds: ObservationAuthorityBoundsV2,
    audits: HashMap<[u8; 32], ObservationAuthorityAuditV2>,
    data_by_id: HashMap<[u8; 32], ObservationAuthorityDataV2>,
    orphan: Option<ObservationAuthorityDataV2>,
    snapshot_digest: [u8; 32],
    lock_generation: ObservationFileGenerationV1,
    file_generation: ObservationFileGenerationV1,
    writable: bool,
}

impl ObservationAuthorityLedgerV2 {
    fn open(root: impl AsRef<Path>, bounds: ObservationAuthorityBoundsV2) -> Result<Self, String> {
        Self::open_inner(root.as_ref(), bounds, true)
    }

    /// Opens existing V2 bytes read-only under explicit refusal ceilings.
    ///
    /// # Errors
    ///
    /// Refuses absent/replaced paths, a foreign header, ragged/corrupt or
    /// resealed semantic bytes, duplicate identities, middle orphans and any
    /// configured count/byte-bound breach.
    pub fn open_read(
        root: impl AsRef<Path>,
        bounds: ObservationAuthorityBoundsV2,
    ) -> Result<Self, String> {
        Self::open_inner(root.as_ref(), bounds, false)
    }

    fn open_inner(
        root: &Path,
        bounds: ObservationAuthorityBoundsV2,
        writable: bool,
    ) -> Result<Self, String> {
        let admitted_root = admit_existing_observation_root(root)?;
        let file_path = admitted_root.join(AUTHORITY_V2_FILE);
        let lock_path = admitted_root.join(AUTHORITY_V2_LOCK_FILE);
        let lock_file = OpenOptions::new()
            .read(true)
            .write(writable)
            .create(writable)
            .truncate(false)
            .open(&lock_path)
            .map_err(|why| format!("cannot open Observation V2 lock: {why}"))?;
        if writable {
            lock_file
                .lock()
                .map_err(|why| format!("cannot lock Observation V2 writer: {why}"))?;
        } else {
            lock_file
                .lock_shared()
                .map_err(|why| format!("cannot take shared Observation V2 lock: {why}"))?;
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(writable)
            .create(writable)
            .truncate(false)
            .open(&file_path)
            .map_err(|why| format!("cannot open Observation V2 file: {why}"))?;
        if writable
            && file
                .metadata()
                .map_err(|why| format!("cannot stat Observation V2 file: {why}"))?
                .len()
                == 0
        {
            file.write_all(&observation_v2_header())
                .and_then(|()| file.sync_data())
                .map_err(|why| format!("cannot initialize Observation V2 file: {why}"))?;
        }
        let bytes = read_bounded_observation_v2_file(&mut file, bounds)?;
        let (audits, data_by_id, orphan) = scan_observation_v2_file(&bytes, bounds)?;
        let snapshot_digest = digest_observation_v2_file(&bytes);
        let lock_generation = observation_file_generation(&lock_file, &lock_path)?;
        let file_generation = observation_file_generation(&file, &file_path)?;
        Ok(Self {
            lock_path,
            file_path,
            file,
            lock_file,
            bounds,
            audits,
            data_by_id,
            orphan,
            snapshot_digest,
            lock_generation,
            file_generation,
            writable,
        })
    }

    /// Returns one cached audit only after detecting stale or replaced bytes.
    ///
    /// # Errors
    ///
    /// Refuses any lock/data generation change or bounded content mismatch.
    pub fn reopen_audit(
        &mut self,
        authority_id: &[u8; 32],
    ) -> Result<Option<ObservationAuthorityAuditV2>, String> {
        self.require_unchanged()?;
        Ok(self.audits.get(authority_id).copied())
    }

    fn append_data(
        &mut self,
        prepared: &ObservationAuthorityDataV2,
    ) -> Result<ObservationAuthorityCommitV2, String> {
        if !self.writable {
            return Err("read-only Observation V2 ledger cannot append".to_owned());
        }
        prepared.validate()?;
        self.require_unchanged()?;
        if let Some(existing) = self.audits.get(&prepared.authority_id).copied() {
            let existing_data = self
                .data_by_id
                .get(&prepared.authority_id)
                .copied()
                .ok_or_else(|| "Observation V2 audit lost its Data record".to_owned())?;
            if !existing_data.same_semantics(*prepared) {
                return Err("Observation V2 identity aliases foreign semantic bytes".to_owned());
            }
            return Ok(ObservationAuthorityCommitV2::Reused(existing));
        }
        let completed = u64::try_from(self.audits.len())
            .map_err(|_| "Observation V2 authority count does not fit u64".to_owned())?;
        if completed >= self.bounds.max_authorities {
            return Err("Observation V2 authority count exceeds configured maximum".to_owned());
        }
        let data = if let Some(orphan) = self.orphan {
            if !orphan.same_semantics(*prepared) {
                return Err("Observation V2 trailing Data belongs to a foreign source".to_owned());
            }
            orphan
        } else {
            self.require_append_capacity(2)?;
            let data = prepared.with_sequence(completed);
            let record = data.record(AUTHORITY_V2_DATA_KIND)?;
            self.file
                .seek(SeekFrom::End(0))
                .and_then(|_| self.file.write_all(&record))
                .and_then(|()| self.file.sync_data())
                .map_err(|why| format!("cannot sync Observation V2 Data: {why}"))?;
            self.orphan = Some(data);
            data
        };
        self.require_append_capacity(1)?;
        let completion = data.record(AUTHORITY_V2_COMPLETION_KIND)?;
        self.file
            .seek(SeekFrom::End(0))
            .and_then(|_| self.file.write_all(&completion))
            .and_then(|()| self.file.sync_data())
            .map_err(|why| format!("cannot sync Observation V2 Completion: {why}"))?;
        let audit = observation_v2_audit(&data)?;
        self.audits.insert(data.authority_id, audit);
        self.data_by_id.insert(data.authority_id, data);
        self.orphan = None;
        self.refresh_snapshot()?;
        Ok(ObservationAuthorityCommitV2::Written(audit))
    }

    fn require_append_capacity(&self, records: u64) -> Result<(), String> {
        let current = self
            .file
            .metadata()
            .map_err(|why| format!("cannot stat Observation V2 file: {why}"))?
            .len();
        let added = OBSERVATION_AUTHORITY_RECORD_STRIDE_V2
            .checked_mul(records)
            .ok_or_else(|| "Observation V2 append size overflowed u64".to_owned())?;
        let next = current
            .checked_add(added)
            .ok_or_else(|| "Observation V2 file size overflowed u64".to_owned())?;
        if next > self.bounds.max_file_bytes {
            return Err(format!(
                "Observation V2 append would require {next} bytes above configured maximum {}",
                self.bounds.max_file_bytes
            ));
        }
        Ok(())
    }

    fn require_unchanged(&mut self) -> Result<(), String> {
        require_observation_generation(self.lock_generation, &self.lock_file, &self.lock_path)?;
        require_observation_generation(self.file_generation, &self.file, &self.file_path)?;
        let bytes = read_bounded_observation_v2_file(&mut self.file, self.bounds)?;
        if digest_observation_v2_file(&bytes) != self.snapshot_digest {
            return Err("Observation V2 file changed after open".to_owned());
        }
        Ok(())
    }

    fn refresh_snapshot(&mut self) -> Result<(), String> {
        let bytes = read_bounded_observation_v2_file(&mut self.file, self.bounds)?;
        self.snapshot_digest = digest_observation_v2_file(&bytes);
        self.lock_generation = observation_file_generation(&self.lock_file, &self.lock_path)?;
        self.file_generation = observation_file_generation(&self.file, &self.file_path)?;
        Ok(())
    }
}

fn observation_v2_policy_digest() -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_V2_POLICY_DOMAIN);
    hasher.update(&OBSERVATION_AUTHORITY_SCHEMA_VERSION_V2.to_le_bytes());
    hasher.update(
        b"exact-pre-admission-v2-data;natural-extinction-only;zero-candidate-observation-rows;no-statistics",
    );
    hasher.finalize()
}

fn observation_v2_authority_id(value: &ObservationAuthorityDataV2) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_V2_ID_DOMAIN);
    hasher.update(&value.pre_admission_record_index.to_le_bytes());
    hasher.update(&value.observation_row_count.to_le_bytes());
    hasher.update(&value.disposition.byte().to_le_bytes());
    hasher.update(&value.policy_digest);
    hasher.update(&value.source_record);
    hasher.finalize()
}

fn observation_v2_record_digest(payload: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_V2_RECORD_DOMAIN);
    hasher.update(payload);
    hasher.finalize()
}

fn observation_v2_header() -> [u8; AUTHORITY_V2_HEADER_BYTES] {
    let mut raw = [0_u8; AUTHORITY_V2_HEADER_BYTES];
    raw[..16].copy_from_slice(&AUTHORITY_V2_HEADER_MAGIC);
    raw[16..20].copy_from_slice(&OBSERVATION_AUTHORITY_SCHEMA_VERSION_V2.to_le_bytes());
    raw[20..24].copy_from_slice(
        &u32::try_from(AUTHORITY_V2_RECORD_BYTES)
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_V2_HEADER_DOMAIN);
    hasher.update(&raw[..32]);
    raw[32..].copy_from_slice(&hasher.finalize());
    raw
}

fn observation_v2_audit(
    data: &ObservationAuthorityDataV2,
) -> Result<ObservationAuthorityAuditV2, String> {
    let data_record = data.record(AUTHORITY_V2_DATA_KIND)?;
    let completion_record = data.record(AUTHORITY_V2_COMPLETION_KIND)?;
    Ok(ObservationAuthorityAuditV2 {
        record_sequence: data.record_sequence,
        authority_id: data.authority_id,
        pre_admission_authority_id: data.source.authority_id(),
        pre_admission_record_index: data.pre_admission_record_index,
        candidate_universe_id: data.source.candidate_universe_id(),
        candidate_completion_digest: data.source.candidate_completion_digest(),
        family: data.source.family(),
        rung_seconds: data.source.rung_seconds(),
        observation_row_count: data.observation_row_count,
        disposition: data.disposition,
        policy_digest: data.policy_digest,
        data_record_digest: get_fixed(&data_record, AUTHORITY_V2_PAYLOAD_BYTES)?,
        completion_digest: get_fixed(&completion_record, AUTHORITY_V2_PAYLOAD_BYTES)?,
    })
}

type ObservationV2Scan = (
    HashMap<[u8; 32], ObservationAuthorityAuditV2>,
    HashMap<[u8; 32], ObservationAuthorityDataV2>,
    Option<ObservationAuthorityDataV2>,
);

fn scan_observation_v2_file(
    bytes: &[u8],
    bounds: ObservationAuthorityBoundsV2,
) -> Result<ObservationV2Scan, String> {
    if bytes.get(..AUTHORITY_V2_HEADER_BYTES) != Some(observation_v2_header().as_slice()) {
        return Err("Observation V2 file header is absent or corrupt".to_owned());
    }
    let body = bytes
        .get(AUTHORITY_V2_HEADER_BYTES..)
        .ok_or_else(|| "Observation V2 body is absent".to_owned())?;
    if !body.len().is_multiple_of(AUTHORITY_V2_RECORD_BYTES) {
        return Err("Observation V2 fixed-stride file is ragged".to_owned());
    }
    let record_count = body.len() / AUTHORITY_V2_RECORD_BYTES;
    let maximum_records = bounds
        .max_authorities
        .checked_mul(2)
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| "Observation V2 record bound overflowed u64".to_owned())?;
    if u64::try_from(record_count)
        .map_err(|_| "Observation V2 count does not fit u64".to_owned())?
        > maximum_records
    {
        return Err("Observation V2 record count exceeds configured maximum".to_owned());
    }
    let mut audits = HashMap::new();
    let mut data_by_id = HashMap::new();
    audits
        .try_reserve(record_count / 2)
        .map_err(|why| format!("cannot reserve Observation V2 audit index: {why}"))?;
    data_by_id
        .try_reserve(record_count / 2)
        .map_err(|why| format!("cannot reserve Observation V2 data index: {why}"))?;
    let mut pending: Option<ObservationAuthorityDataV2> = None;
    for (physical, record) in body.chunks_exact(AUTHORITY_V2_RECORD_BYTES).enumerate() {
        let (kind, data) = ObservationAuthorityDataV2::decode(record)?;
        let expected_sequence = u64::try_from(physical / 2)
            .map_err(|_| "Observation V2 sequence does not fit u64".to_owned())?;
        if data.record_sequence != expected_sequence {
            return Err("Observation V2 record sequence is foreign".to_owned());
        }
        match kind {
            AUTHORITY_V2_DATA_KIND => {
                if pending.replace(data).is_some() {
                    return Err("Observation V2 contains consecutive Data records".to_owned());
                }
            }
            AUTHORITY_V2_COMPLETION_KIND => {
                let preceding = pending
                    .take()
                    .ok_or_else(|| "Observation V2 Completion has no adjacent Data".to_owned())?;
                if preceding != data {
                    return Err(
                        "Observation V2 Completion differs from adjacent Data semantics".to_owned(),
                    );
                }
                let audit = observation_v2_audit(&data)?;
                if audits.insert(data.authority_id, audit).is_some()
                    || data_by_id.insert(data.authority_id, data).is_some()
                {
                    return Err("Observation V2 authority identity is duplicated".to_owned());
                }
            }
            _ => return Err("Observation V2 record kind is foreign".to_owned()),
        }
    }
    Ok((audits, data_by_id, pending))
}

fn read_bounded_observation_v2_file(
    file: &mut File,
    bounds: ObservationAuthorityBoundsV2,
) -> Result<Vec<u8>, String> {
    let len = file
        .metadata()
        .map_err(|why| format!("cannot stat Observation V2 file: {why}"))?
        .len();
    if len > bounds.max_file_bytes {
        return Err(format!(
            "Observation V2 file is {len} bytes above configured maximum {}",
            bounds.max_file_bytes
        ));
    }
    let len = usize::try_from(len)
        .map_err(|_| "Observation V2 file length does not fit usize".to_owned())?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len)
        .map_err(|why| format!("cannot reserve {len} Observation V2 bytes: {why}"))?;
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_to_end(&mut bytes))
        .map_err(|why| format!("cannot read Observation V2 file: {why}"))?;
    if bytes.len() != len {
        return Err("Observation V2 file changed length during read".to_owned());
    }
    Ok(bytes)
}

fn digest_observation_v2_file(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(AUTHORITY_V2_FILE_DOMAIN);
    hasher.update(bytes);
    hasher.finalize()
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "focused fixtures use explicit panic assertions and small, locally constructed exact arrays"
)]
mod tests {
    use super::*;

    const DAY_MICROS: i64 = 86_400_000_000;

    fn stamp(day: i64, minute: i64) -> i64 {
        day.checked_mul(DAY_MICROS)
            .and_then(|value| value.checked_add(minute * 60_000_000))
            .and_then(|value| value.checked_sub(indicators::IST_OFFSET_MICROS))
            .expect("fixture timestamp fits i64")
    }

    fn candle(day: i64, minute: i64) -> Candle {
        Candle {
            ts_micros: stamp(day, minute),
            open: 2_000_000,
            high: 2_000_100,
            low: 1_999_900,
            close: 2_000_050,
            volume: 1_000,
            open_interest: i64::MIN,
        }
    }

    fn measured_open_sessions(count: usize) -> Vec<i64> {
        (20_080_i64..20_200)
            .filter(|day| matches!(kind_of(*day), DayKind::Open(_)))
            .take(count)
            .collect()
    }

    struct TestDir(PathBuf);

    impl TestDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn test_dir() -> TestDir {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "brutex-observation-authority-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("test authority directory is writable");
        TestDir(path)
    }

    fn authority_bounds() -> ObservationAuthorityBoundsV1 {
        ObservationAuthorityBoundsV1::new(
            8,
            OBSERVATION_AUTHORITY_HEADER_BYTES_V1 + OBSERVATION_AUTHORITY_RECORD_STRIDE_V1 * 18,
        )
        .expect("test authority bounds are explicit")
    }

    fn authority_bounds_v2() -> ObservationAuthorityBoundsV2 {
        ObservationAuthorityBoundsV2::new(
            8,
            OBSERVATION_AUTHORITY_HEADER_BYTES_V2 + OBSERVATION_AUTHORITY_RECORD_STRIDE_V2 * 18,
        )
        .expect("test Observation V2 bounds are explicit")
    }

    fn authority_data_fixture(tag: u8) -> ObservationAuthorityDataV1 {
        let layout = derive_layout(4).expect("four fixture periods derive layout");
        let policy = observation_policy_receipt();
        let mut value = ObservationAuthorityDataV1 {
            authority_id: [0; 32],
            pair_identity: [tag; 32],
            source_identity: [tag.wrapping_add(1); 32],
            observation_policy_digest: policy.digest,
            layout_policy_digest: layout.digest,
            nifty_universe_id: [tag.wrapping_add(2); 32],
            banknifty_universe_id: [tag.wrapping_add(3); 32],
            nifty_family_identity: [tag.wrapping_add(4); 32],
            banknifty_family_identity: [tag.wrapping_add(5); 32],
            accepted_session_digest: [tag.wrapping_add(6); 32],
            ordered_period_digest: [tag.wrapping_add(7); 32],
            ordered_split_score_digest: [tag.wrapping_add(8); 32],
            rung_seconds: 60,
            horizon_bars: 30,
            period_count: layout.period_count,
            nifty_candidate_count: 2,
            banknifty_candidate_count: 3,
            segment_count: layout.segment_count,
            observation_schema_version: OBSERVATION_SCHEMA_VERSION_V1,
            periods_per_segment: layout.periods_per_segment,
            split_count: layout.split_count,
            split_score_row_count: 5 * layout.split_count,
            maximum_segments: layout.maximum_segments,
            record_digest: [0; 32],
        };
        value.authority_id = authority_id(&value).expect("fixture authority ID derives");
        value.record_digest = authority_record_digest(&value).expect("fixture Data digest derives");
        value
            .validate()
            .expect("fixture authority Data is canonical");
        value
    }

    fn trade(
        entry_bar: usize,
        exit_bar: usize,
        entry_day: i64,
        entry_minute: i64,
        exit_day: i64,
        exit_minute: i64,
        worst: i64,
    ) -> TradeRow {
        TradeRow {
            signal_bar: entry_bar,
            entry_bar,
            exit_bar,
            best: worst,
            worst,
            entry_micros: stamp(entry_day, entry_minute),
            exit_micros: stamp(exit_day, exit_minute),
            adverse: 0,
            adverse_paisa: 0,
            favourable: 0,
            favourable_paisa: 0,
        }
    }

    #[test]
    fn exact_worst_rows_attribute_to_exit_sessions_and_preserve_zero_sessions() {
        let sessions = measured_open_sessions(4);
        assert_eq!(sessions.len(), 4);
        let mut builder = CandidateObservationBuilderV1::from_sessions_for_test(sessions.clone())
            .expect("measured sessions form a builder");
        let bars = vec![
            candle(sessions[0], 555),
            candle(sessions[0], 556),
            candle(sessions[2], 555),
            candle(sessions[2], 556),
        ];
        let rows = vec![
            trade(0, 1, sessions[0], 555, sessions[0], 556, 125),
            trade(2, 3, sessions[2], 555, sessions[2], 556, -40),
        ];
        let expected = Cell {
            trades: 2,
            wins: 1,
            pessimistic: 85,
            ..Cell::default()
        };
        builder
            .observe_candidate([7; 32], &expected, &bars, &rows)
            .expect("exact rows reconcile");
        let candidate = &builder.candidates[0];
        assert_eq!(candidate.periods.len(), sessions.len());
        assert_eq!(candidate.periods[0].return_paisa, 125);
        assert_eq!(candidate.periods[0].trades, 1);
        assert_eq!(candidate.periods[0].wins, 1);
        assert_eq!(candidate.periods[1], PeriodAccumulatorV1::default());
        assert_eq!(candidate.periods[2].return_paisa, -40);
        assert_eq!(candidate.periods[2].wins, 0);
        assert_eq!(candidate.periods[3], PeriodAccumulatorV1::default());
    }

    #[test]
    fn cross_day_foreign_timestamp_duplicate_semantic_and_overflow_refuse() {
        let sessions = measured_open_sessions(2);
        assert_eq!(sessions.len(), 2);
        let bars = vec![candle(sessions[0], 555), candle(sessions[1], 555)];
        let cross = trade(0, 1, sessions[0], 555, sessions[1], 555, 10);
        let expected = Cell {
            trades: 1,
            wins: 1,
            pessimistic: 10,
            ..Cell::default()
        };
        let mut builder = CandidateObservationBuilderV1::from_sessions_for_test(sessions.clone())
            .expect("measured sessions form a builder");
        assert!(
            builder
                .observe_candidate([1; 32], &expected, &bars, &[cross])
                .expect_err("cross-day trade must refuse")
                .contains("crosses IST days")
        );

        let same_day_bars = vec![candle(sessions[0], 555), candle(sessions[0], 556)];
        let rows = [trade(0, 1, sessions[0], 555, sessions[0], 556, 10)];
        let mut duplicate = CandidateObservationBuilderV1::from_sessions_for_test(sessions.clone())
            .expect("measured sessions form a builder");
        duplicate
            .observe_candidate([2; 32], &expected, &same_day_bars, &rows)
            .expect("first semantic is exact");
        assert!(
            duplicate
                .observe_candidate([2; 32], &expected, &same_day_bars, &rows)
                .expect_err("duplicate semantic must refuse")
                .contains("duplicated")
        );

        let overflow_rows = [
            trade(0, 0, sessions[0], 555, sessions[0], 555, i64::MAX),
            trade(1, 1, sessions[0], 556, sessions[0], 556, 1),
        ];
        let overflow_expected = Cell {
            trades: 2,
            wins: 2,
            pessimistic: i64::MAX,
            ..Cell::default()
        };
        let mut overflow = CandidateObservationBuilderV1::from_sessions_for_test(vec![sessions[0]])
            .expect("one session can aggregate even though it cannot form CSCV");
        assert!(
            overflow
                .observe_candidate([3; 32], &overflow_expected, &same_day_bars, &overflow_rows,)
                .expect_err("checked worst sum must refuse overflow")
                .contains("overflowed i64")
        );
    }

    #[test]
    fn deterministic_layout_uses_all_periods_equal_blocks_and_never_defaults() {
        let eight = derive_layout(8).expect("eight periods admit eight equal segments");
        assert_eq!(eight.segment_count, 8);
        assert_eq!(eight.periods_per_segment, 1);
        assert_eq!(eight.split_count, 35);
        assert_eq!(canonical_masks(eight).expect("masks derive").len(), 35);

        let twelve = derive_layout(12).expect("twelve periods admit twelve equal segments");
        assert_eq!(twelve.segment_count, 12);
        assert_eq!(twelve.periods_per_segment, 1);
        assert_eq!(twelve.split_count, 462);

        let eighteen = derive_layout(18)
            .expect("eighteen periods choose the largest even divisor through sixteen");
        assert_eq!(eighteen.segment_count, 6);
        assert_eq!(eighteen.periods_per_segment, 3);
        assert_eq!(eighteen.split_count, 10);
        assert!(
            derive_layout(7)
                .expect_err("odd prime periods cannot be equal even blocks")
                .contains("no period may be dropped")
        );
    }

    #[test]
    fn split_scores_are_checked_complete_and_identity_bound() {
        let layout = derive_layout(4).expect("four periods derive a layout");
        let masks = canonical_masks(layout).expect("canonical masks derive");
        assert_eq!(masks, vec![(6, 9), (10, 5), (12, 3)]);
        let candidate = CandidateSessionObservationsV1 {
            candidate_sequence: 0,
            candidate_semantic_digest: [9; 32],
            periods: [10, 20, 30, 40]
                .into_iter()
                .enumerate()
                .map(|(sequence, value)| CandidateSessionPeriodV1 {
                    candidate_sequence: 0,
                    candidate_semantic_digest: [9; 32],
                    period_sequence: u64::try_from(sequence).expect("sequence fits"),
                    exit_ist_day: [20_090, 20_091, 20_092, 20_095][sequence],
                    return_paisa: value,
                    trades: 1,
                    wins: 1,
                    identity: [1; 32],
                })
                .collect(),
            total_return_paisa: 100,
            total_trades: 4,
            total_wins: 4,
        };
        let mut scores = Vec::new();
        append_candidate_split_scores(
            &mut scores,
            InstrumentFamilyV1::Nifty,
            &[candidate],
            0,
            layout,
            &masks,
            [3; 32],
        )
        .expect("scores derive without caller values");
        assert_eq!(scores.len(), 3);
        assert_eq!(scores[0].train_score_paisa, 50);
        assert_eq!(scores[0].test_score_paisa, 50);
        assert_eq!(scores[1].train_score_paisa, 60);
        assert_eq!(scores[1].test_score_paisa, 40);
        assert_eq!(scores[2].train_score_paisa, 70);
        assert_eq!(scores[2].test_score_paisa, 30);
        assert!(
            scores
                .windows(2)
                .all(|pair| pair[0].identity != pair[1].identity)
        );
    }

    #[test]
    fn authority_is_receipt_last_freshly_reopened_and_exactly_reused() {
        let root = test_dir();
        let bounds = authority_bounds();
        let data = authority_data_fixture(20);
        let written = append_authority_data_and_reopen(root.path(), bounds, &data)
            .expect("Data then Completion sync and freshly reopen");
        assert!(matches!(written, ObservationAuthorityCommitV1::Written(_)));
        assert_eq!(written.audit().authority_id(), data.authority_id);
        assert_eq!(written.audit().candidate_count(), 5);
        assert_eq!(written.audit().split_score_row_count(), 15);

        let reused = append_authority_data_and_reopen(root.path(), bounds, &data)
            .expect("exact retry freshly reopens without duplicate records");
        assert!(matches!(reused, ObservationAuthorityCommitV1::Reused(_)));
        assert_eq!(reused.audit(), written.audit());

        let mut read = ObservationAuthorityLedgerV1::open_read(root.path(), bounds)
            .expect("completed authority opens read-only");
        assert_eq!(
            read.reopen_audit(&data.authority_id)
                .expect("unchanged bytes validate"),
            Some(written.audit())
        );
    }

    #[test]
    fn authority_refuses_absent_root_foreign_orphan_corruption_and_stale_bytes() {
        let parent = test_dir();
        let missing = parent.path().join("detached-observation-volume");
        let bounds = authority_bounds();
        assert!(
            ObservationAuthorityLedgerV1::open(&missing, bounds)
                .expect_err("missing configured root must refuse")
                .contains("must already exist")
        );
        assert!(!missing.exists(), "writer must not manufacture the root");

        let file_root = parent.path().join("not-a-directory");
        File::create(&file_root).expect("regular-file root fixture exists");
        assert!(
            ObservationAuthorityLedgerV1::open(&file_root, bounds)
                .expect_err("regular file cannot be a root")
                .contains("not a directory")
        );

        let orphan_root = test_dir();
        drop(
            ObservationAuthorityLedgerV1::open(orphan_root.path(), bounds)
                .expect("empty authority file initializes"),
        );
        let orphan = authority_data_fixture(30);
        let path = orphan_root.path().join(AUTHORITY_FILE);
        let mut raw = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("authority file reopens for crash fixture");
        raw.write_all(&orphan.record().expect("orphan Data encodes"))
            .and_then(|()| raw.sync_data())
            .expect("orphan Data is durable without Completion");
        drop(raw);
        let mut ledger = ObservationAuthorityLedgerV1::open(orphan_root.path(), bounds)
            .expect("one trailing Data is recoverable");
        assert!(
            ledger
                .append_data(&authority_data_fixture(31))
                .expect_err("foreign pair cannot overwrite orphan")
                .contains("foreign pair")
        );
        drop(ledger);
        assert!(matches!(
            append_authority_data_and_reopen(orphan_root.path(), bounds, &orphan)
                .expect("exact orphan retry appends only Completion"),
            ObservationAuthorityCommitV1::Written(_)
        ));

        let stale_root = test_dir();
        let stale_data = authority_data_fixture(40);
        append_authority_data_and_reopen(stale_root.path(), bounds, &stale_data)
            .expect("stale fixture first commits cleanly");
        let mut stale = ObservationAuthorityLedgerV1::open_read(stale_root.path(), bounds)
            .expect("stale fixture opens read-only");
        let stale_path = stale_root.path().join(AUTHORITY_FILE);
        let mut external = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&stale_path)
            .expect("external stale writer opens");
        external
            .seek(SeekFrom::Start(OBSERVATION_AUTHORITY_HEADER_BYTES_V1 + 100))
            .expect("stale byte seeks");
        let mut byte = [0_u8; 1];
        external.read_exact(&mut byte).expect("stale byte reads");
        byte[0] ^= 1;
        external
            .seek(SeekFrom::Start(OBSERVATION_AUTHORITY_HEADER_BYTES_V1 + 100))
            .and_then(|_| external.write_all(&byte))
            .and_then(|()| external.sync_data())
            .expect("same-length stale mutation persists");
        drop(external);
        assert!(
            stale
                .reopen_audit(&stale_data.authority_id)
                .expect_err("cached audit must refuse same-length mutation")
                .contains("changed")
        );
        drop(stale);
        assert!(
            ObservationAuthorityLedgerV1::open_read(stale_root.path(), bounds)
                .expect_err("fresh open must refuse corrupt Data seal")
                .contains("does not recompute")
        );
    }

    #[cfg(unix)]
    #[test]
    fn replaced_lock_and_data_paths_refuse_cached_authority() {
        let bounds = authority_bounds();
        for (tag, name) in [AUTHORITY_LOCK_FILE, AUTHORITY_FILE]
            .into_iter()
            .enumerate()
        {
            let root = test_dir();
            let data = authority_data_fixture(
                u8::try_from(50 + tag).expect("two replacement fixture tags fit u8"),
            );
            append_authority_data_and_reopen(root.path(), bounds, &data)
                .expect("replacement fixture commits before reopen");
            let mut ledger = ObservationAuthorityLedgerV1::open_read(root.path(), bounds)
                .expect("replacement fixture opens cached audit");
            let named = root.path().join(name);
            let displaced = root.path().join(format!("{name}.displaced"));
            std::fs::rename(&named, &displaced)
                .expect("held observation authority inode can be displaced on Unix");
            File::create(&named).expect("replacement observation authority path is created");

            assert!(
                ledger
                    .reopen_audit(&data.authority_id)
                    .expect_err("path replacement must invalidate the cached authority")
                    .contains("no longer names"),
                "replacement of {name} did not fail closed"
            );
        }
    }

    #[test]
    fn v2_zero_family_consumes_only_authenticated_extinction_and_keeps_family_identity() {
        let (nifty_source, nifty_commit) =
            crate::pre_admission_data::observation_v2_zero_production_fixture(80)
                .expect("even tag produces zero NIFTY Pre-Admission V2");
        let (bank_source, bank_commit) =
            crate::pre_admission_data::observation_v2_zero_production_fixture(81)
                .expect("odd tag produces zero BANKNIFTY Pre-Admission V2");
        let nifty = produce_natural_extinction_observation_v2(&nifty_source, &nifty_commit)
            .expect("authenticated zero NIFTY produces Observation V2");
        let bank = produce_natural_extinction_observation_v2(&bank_source, &bank_commit)
            .expect("authenticated zero BANKNIFTY produces Observation V2");
        assert_eq!(nifty.value.source.family(), InstrumentFamilyV1::Nifty);
        assert_eq!(bank.value.source.family(), InstrumentFamilyV1::BankNifty);
        assert_ne!(nifty.value.authority_id, bank.value.authority_id);
        assert_eq!(nifty.value.observation_row_count, 0);
        assert_eq!(
            nifty.value.disposition,
            ObservationAuthorityDispositionV2::NaturallyExtinct
        );

        assert!(
            produce_natural_extinction_observation_v2(&nifty_source, &bank_commit)
                .expect_err("foreign Pre-Admission commit must refuse")
                .contains("foreign production")
        );
        let (nonzero_source, nonzero_commit) =
            crate::pre_admission_data::observation_v2_nonzero_production_fixture(82)
                .expect("nonzero Pre-Admission fixture is authenticated");
        assert!(
            produce_natural_extinction_observation_v2(&nonzero_source, &nonzero_commit)
                .expect_err("nonzero family cannot be mislabeled naturally extinct")
                .contains("nonzero Candidate family")
        );

        let data = nifty
            .value
            .record(AUTHORITY_V2_DATA_KIND)
            .expect("Observation V2 Data encodes");
        let completion = nifty
            .value
            .record(AUTHORITY_V2_COMPLETION_KIND)
            .expect("Observation V2 Completion encodes");
        assert_ne!(data, completion, "receipt kind is inside the outer seal");
        assert_eq!(
            ObservationAuthorityDataV2::decode(&data),
            Ok((AUTHORITY_V2_DATA_KIND, nifty.value))
        );
        assert_eq!(
            ObservationAuthorityDataV2::decode(&completion),
            Ok((AUTHORITY_V2_COMPLETION_KIND, nifty.value))
        );
        for index in 0..AUTHORITY_V2_RECORD_BYTES {
            let mut changed = data;
            changed[index] ^= 1;
            assert!(
                ObservationAuthorityDataV2::decode(&changed).is_err(),
                "unresealed Observation V2 mutation at byte {index} must refuse"
            );
        }

        let mut invented = nifty.value;
        invented.observation_row_count = 1;
        invented.authority_id = observation_v2_authority_id(&invented);
        assert!(
            invented
                .validate()
                .expect_err("invented zero-family observation row refuses")
                .contains("exact zero rows")
        );
    }

    #[test]
    fn v2_authority_is_receipt_last_freshly_reopened_idempotent_and_orphan_safe() {
        let bounds = authority_bounds_v2();
        let root = test_dir();
        let (source, commit) =
            crate::pre_admission_data::observation_v2_zero_production_fixture(84)
                .expect("zero source fixture derives");
        let produced = produce_natural_extinction_observation_v2(&source, &commit)
            .expect("zero source prepares Observation V2");
        let written = produced
            .append_and_reopen(root.path(), bounds)
            .expect("Observation V2 writes Data then Completion and reopens");
        assert!(matches!(written, ObservationAuthorityCommitV2::Written(_)));
        let audit = written.audit();
        assert_eq!(audit.record_sequence(), 0);
        assert_eq!(audit.observation_row_count(), 0);
        assert_eq!(audit.family(), InstrumentFamilyV1::Nifty);
        assert_eq!(
            audit.disposition(),
            ObservationAuthorityDispositionV2::NaturallyExtinct
        );
        let reused = produced
            .append_and_reopen(root.path(), bounds)
            .expect("exact retry reuses byte-identical authority");
        assert!(matches!(reused, ObservationAuthorityCommitV2::Reused(_)));
        assert_eq!(reused.audit(), audit);
        assert_eq!(
            std::fs::metadata(root.path().join(AUTHORITY_V2_FILE))
                .expect("Observation V2 file exists")
                .len(),
            OBSERVATION_AUTHORITY_HEADER_BYTES_V2 + OBSERVATION_AUTHORITY_RECORD_STRIDE_V2 * 2
        );

        let orphan_root = test_dir();
        drop(
            ObservationAuthorityLedgerV2::open(orphan_root.path(), bounds)
                .expect("orphan file header initializes"),
        );
        let orphan = produced.value.with_sequence(0);
        let mut file = OpenOptions::new()
            .append(true)
            .open(orphan_root.path().join(AUTHORITY_V2_FILE))
            .expect("orphan file opens");
        file.write_all(
            &orphan
                .record(AUTHORITY_V2_DATA_KIND)
                .expect("orphan Data encodes"),
        )
        .and_then(|()| file.sync_data())
        .expect("orphan Data is durable without Completion");
        drop(file);
        let mut ledger = ObservationAuthorityLedgerV2::open(orphan_root.path(), bounds)
            .expect("one trailing Data is recoverable");
        let (foreign_source, foreign_commit) =
            crate::pre_admission_data::observation_v2_zero_production_fixture(85)
                .expect("foreign zero source derives");
        let foreign = produce_natural_extinction_observation_v2(&foreign_source, &foreign_commit)
            .expect("foreign zero source prepares");
        assert!(
            ledger
                .append_data(&foreign.value)
                .expect_err("foreign source cannot bless trailing Data")
                .contains("foreign source")
        );
        assert!(matches!(
            ledger
                .append_data(&orphan)
                .expect("exact orphan retry appends only Completion"),
            ObservationAuthorityCommitV2::Written(_)
        ));
    }

    #[test]
    fn v2_ragged_corrupt_resealed_and_stale_authorities_fail_closed() {
        let bounds = authority_bounds_v2();
        let ragged_root = test_dir();
        drop(
            ObservationAuthorityLedgerV2::open(ragged_root.path(), bounds)
                .expect("ragged header initializes"),
        );
        let mut ragged = OpenOptions::new()
            .append(true)
            .open(ragged_root.path().join(AUTHORITY_V2_FILE))
            .expect("ragged file opens");
        ragged.write_all(&[1]).expect("ragged byte writes");
        ragged.sync_data().expect("ragged byte syncs");
        drop(ragged);
        assert!(
            ObservationAuthorityLedgerV2::open_read(ragged_root.path(), bounds)
                .expect_err("ragged V2 file refuses")
                .contains("ragged")
        );

        let (source, commit) =
            crate::pre_admission_data::observation_v2_zero_production_fixture(86)
                .expect("zero source fixture derives");
        let produced = produce_natural_extinction_observation_v2(&source, &commit)
            .expect("zero source prepares");
        let resealed_root = test_dir();
        drop(
            ObservationAuthorityLedgerV2::open(resealed_root.path(), bounds)
                .expect("resealed header initializes"),
        );
        let mut data = produced
            .value
            .record(AUTHORITY_V2_DATA_KIND)
            .expect("Data encodes");
        let mut completion = produced
            .value
            .record(AUTHORITY_V2_COMPLETION_KIND)
            .expect("Completion encodes");
        for raw in [&mut data, &mut completion] {
            raw[AUTHORITY_V2_SOURCE_OFFSET + 200] ^= 1;
            let seal = observation_v2_record_digest(&raw[..AUTHORITY_V2_PAYLOAD_BYTES]);
            raw[AUTHORITY_V2_PAYLOAD_BYTES..].copy_from_slice(&seal);
        }
        let mut file = OpenOptions::new()
            .append(true)
            .open(resealed_root.path().join(AUTHORITY_V2_FILE))
            .expect("resealed file opens");
        file.write_all(&data)
            .and_then(|()| file.write_all(&completion))
            .and_then(|()| file.sync_data())
            .expect("outer-resealed pair writes");
        drop(file);
        assert!(
            ObservationAuthorityLedgerV2::open_read(resealed_root.path(), bounds)
                .expect_err("outer reseal cannot bless corrupt embedded source")
                .contains("Pre-Admission")
        );

        let stale_root = test_dir();
        produced
            .append_and_reopen(stale_root.path(), bounds)
            .expect("stale fixture commits");
        let mut cached = ObservationAuthorityLedgerV2::open_read(stale_root.path(), bounds)
            .expect("stale fixture opens read-only");
        let path = stale_root.path().join(AUTHORITY_V2_FILE);
        let mut external = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("stale external writer opens");
        let offset = OBSERVATION_AUTHORITY_HEADER_BYTES_V2 + 100;
        external.seek(SeekFrom::Start(offset)).expect("byte seeks");
        let mut byte = [0_u8; 1];
        external.read_exact(&mut byte).expect("byte reads");
        byte[0] ^= 1;
        external
            .seek(SeekFrom::Start(offset))
            .and_then(|_| external.write_all(&byte))
            .and_then(|()| external.sync_data())
            .expect("same-length mutation persists");
        drop(external);
        assert!(
            cached
                .reopen_audit(&produced.value.authority_id)
                .expect_err("cached audit refuses stale same-length mutation")
                .contains("changed")
        );
    }

    #[cfg(unix)]
    #[test]
    fn v2_replaced_lock_and_data_paths_refuse_cached_zero_family_authority() {
        let bounds = authority_bounds_v2();
        for (tag, name) in [AUTHORITY_V2_LOCK_FILE, AUTHORITY_V2_FILE]
            .into_iter()
            .enumerate()
        {
            let root = test_dir();
            let tag = u8::try_from(90 + tag).expect("two tags fit u8");
            let (source, commit) =
                crate::pre_admission_data::observation_v2_zero_production_fixture(tag)
                    .expect("replacement source derives");
            let produced = produce_natural_extinction_observation_v2(&source, &commit)
                .expect("replacement source prepares");
            produced
                .append_and_reopen(root.path(), bounds)
                .expect("replacement fixture commits");
            let mut cached = ObservationAuthorityLedgerV2::open_read(root.path(), bounds)
                .expect("replacement fixture opens cached audit");
            let named = root.path().join(name);
            let displaced = root.path().join(format!("{name}.displaced"));
            std::fs::rename(&named, &displaced)
                .expect("held Observation V2 inode can be displaced");
            File::create(&named).expect("replacement path is created");
            assert!(
                cached
                    .reopen_audit(&produced.value.authority_id)
                    .expect_err("path replacement must invalidate cached audit")
                    .contains("no longer names"),
                "replacement of {name} did not fail closed"
            );
        }
    }
}
