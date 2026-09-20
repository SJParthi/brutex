//! Durable, replayable storage for one complete strategy population.
//!
//! A final Top-N is only authoritative when both ranking passes can replay the
//! exact same *complete* population.  Keeping only the winners cannot prove
//! that an omitted row would not have won.  This module therefore persists
//! every expanded strategy candidate before it persists a completion receipt.
//!
//! The store is four append-only, fixed-stride files: one row file, two legacy
//! audit receipt files and one authoritative receipt file:
//!
//! * `population-v1.bin` holds one canonical [`PopulationRowV1`] per exact
//!   `(closed mask, direction, exit coordinate)` strategy;
//! * `population-completions-v4.bin` holds one authoritative
//!   [`CompletionReceiptV4`] per population, appended only after its contiguous
//!   row block is synced.
//!
//! The version-one completion codec remains readable as
//! [`CompletionReceiptV1`], but it is deliberately not an authoritative ledger
//! commit: version one carried only one exit-policy/resolution identity and one
//! caller-supplied total cell count for a population containing both long and
//! short strategies.  Version two has a different path, header and stride; no
//! version-one byte is silently reinterpreted or automatically blessed.  The
//! version-two and version-three completion paths are likewise retained as
//! audit sources: neither is reinterpreted as version four because V3 bound the
//! requested month span but not measured signal/execution calendar coverage.
//!
//! Every present file has its own magic, version, reserved-byte checks and
//! BLAKE3 record seals. One cross-process lock serialises row preparation and
//! the V2/V3-audit/V4-authoritative receipt commit sequence. A crash after the row
//! sync but before the receipt leaves a hidden
//! prepared block; an exact rerun byte-verifies and reuses it.  A different
//! rerun is refused and no existing byte is replaced.
//!
//! # Cost, stated honestly
//!
//! Opening walks the row file and every present receipt file once,
//! O(total rows + receipts), and builds a
//! population-id to contiguous-block hash index.  Duplicate reconciliation uses
//! temporary sets proportional to the largest block's distinct strategy keys;
//! they are dropped when that block is indexed.  After open, finding a committed
//! population is one in-memory hash probe, a row read is one seek and one fixed
//! read, and a page read is O(requested rows).  Appending is O(new rows) time and
//! buffer space.  The complete brute-force sweep, open-time scan and whole-block
//! append are not O(1), and this module makes no such claim. Serialization uses
//! one fixed 32-row write chunk; the duplicate-reconciliation sets, not the byte
//! buffer, are the population-sized append allocation.
//!
//! **UNVERIFIED as a measured bound.** No bench in this workspace
//! times this, so the shape above is read from the source rather
//! than measured. `CLAUDE.md` §3 rule 6.

use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::hash::Hash;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use crate::stored::CompleteCalendarReceiptV2;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt as _;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt as _;

/// An operator-facing persistence refusal.
pub type PopulationRefusal = String;

const ROW_MAGIC: [u8; 8] = *b"BRUTEXPP";
const RECEIPT_V2_MAGIC: [u8; 8] = *b"BRUTXPV2";
const RECEIPT_V3_MAGIC: [u8; 8] = *b"BRUTXPV3";
const RECEIPT_V4_MAGIC: [u8; 8] = *b"BRUTXPV4";
const ROW_VERSION: u32 = 1;
const RECEIPT_V2_VERSION: u32 = 2;
const RECEIPT_V3_VERSION: u32 = 3;
const RECEIPT_V4_VERSION: u32 = 4;
const HEADER: u64 = 16;
const HEADER_BYTES: usize = 16;

/// Bytes in one version-one population row.
pub const ROW_STRIDE: u64 = 312;
/// [`ROW_STRIDE`] as a machine-sized constant for fixed buffers.
pub const ROW_STRIDE_BYTES: usize = 312;
const ROW_PAYLOAD_BYTES: usize = 304;

/// Bytes in one legacy version-one completion receipt.
pub const RECEIPT_V1_STRIDE: u64 = 608;
/// [`RECEIPT_V1_STRIDE`] as a machine-sized constant for fixed buffers.
pub const RECEIPT_V1_STRIDE_BYTES: usize = 608;
const RECEIPT_V1_PAYLOAD_BYTES: usize = 600;

/// Bytes in one legacy/audit version-two completion receipt.
pub const RECEIPT_STRIDE: u64 = 704;
/// [`RECEIPT_STRIDE`] as a machine-sized constant for fixed buffers.
pub const RECEIPT_STRIDE_BYTES: usize = 704;
const RECEIPT_PAYLOAD_BYTES: usize = 696;

/// Bytes in one authoritative version-three completion receipt.
pub const RECEIPT_V3_STRIDE: u64 = 768;
/// [`RECEIPT_V3_STRIDE`] as a machine-sized constant for fixed buffers.
pub const RECEIPT_V3_STRIDE_BYTES: usize = 768;
const RECEIPT_V3_PAYLOAD_BYTES: usize = 760;
/// Bytes in one authoritative version-four completion receipt.
pub const RECEIPT_V4_STRIDE: u64 = 864;
/// [`RECEIPT_V4_STRIDE`] as a machine-sized constant for fixed buffers.
pub const RECEIPT_V4_STRIDE_BYTES: usize = 864;
const RECEIPT_V4_PAYLOAD_BYTES: usize = 856;
const POPULATION_CALENDAR_COVERAGE_BYTES: usize = 96;
const POPULATION_CALENDAR_COVERAGE_VERSION: u32 = 1;
/// Bytes in one canonical [`RequestedSpanIdentityV1`] representation.
pub const REQUESTED_SPAN_IDENTITY_BYTES: usize = 20;
const REQUESTED_SPAN_VERSION: u32 = 1;
const REQUESTED_SPAN_DIGEST_DOMAIN: &[u8] = b"brutex-requested-month-span-v1\0";
const RECEIPT_V3_CONTENT_DOMAIN: &[u8] = b"brutex-population-completion-v3\0";
const RECEIPT_V4_CONTENT_DOMAIN: &[u8] = b"brutex-population-completion-v4\0";
const ROW_PAYLOAD_DIGEST_DOMAIN: &[u8] = b"brutex-population-row-payload-v1\0";

const SEAL_BYTES: usize = 8;
/// Hard version-one ceiling for one decoded population page.
///
/// The API's other detail surfaces use the same human-sized bound.  Keeping it
/// in the persistence format's versioned contract prevents an untrusted caller
/// from turning a valid multi-billion-row receipt into one multi-billion-row
/// allocation request.  Zero remains a legitimate empty page request.
pub const MAX_PAGE_ROWS_V1: u64 = 256;
const ROW_WRITE_CHUNK_ROWS: usize = 32;
const ROW_WRITE_CHUNK_BYTES: usize = ROW_WRITE_CHUNK_ROWS * ROW_STRIDE_BYTES;
const EMPTY_ROW_DIGEST_DOMAIN: &[u8] = b"brutex-population-rows-v1\0";

const _: () = assert!(HEADER_BYTES as u64 == HEADER);
const _: () = assert!(ROW_STRIDE_BYTES as u64 == ROW_STRIDE);
const _: () = assert!(RECEIPT_V1_STRIDE_BYTES as u64 == RECEIPT_V1_STRIDE);
const _: () = assert!(RECEIPT_STRIDE_BYTES as u64 == RECEIPT_STRIDE);
const _: () = assert!(RECEIPT_V3_STRIDE_BYTES as u64 == RECEIPT_V3_STRIDE);
const _: () = assert!(RECEIPT_V4_STRIDE_BYTES as u64 == RECEIPT_V4_STRIDE);
const _: () = assert!(ROW_PAYLOAD_BYTES + SEAL_BYTES == ROW_STRIDE_BYTES);
const _: () = assert!(RECEIPT_V1_PAYLOAD_BYTES + SEAL_BYTES == RECEIPT_V1_STRIDE_BYTES);
const _: () = assert!(RECEIPT_PAYLOAD_BYTES + SEAL_BYTES == RECEIPT_STRIDE_BYTES);
const _: () = assert!(RECEIPT_V3_PAYLOAD_BYTES + SEAL_BYTES == RECEIPT_V3_STRIDE_BYTES);
const _: () = assert!(RECEIPT_V4_PAYLOAD_BYTES + SEAL_BYTES == RECEIPT_V4_STRIDE_BYTES);
const _: () = assert!(
    RECEIPT_PAYLOAD_BYTES + REQUESTED_SPAN_IDENTITY_BYTES + 32 + 12 == RECEIPT_V3_PAYLOAD_BYTES
);
const _: () = assert!(
    RECEIPT_V3_PAYLOAD_BYTES + POPULATION_CALENDAR_COVERAGE_BYTES == RECEIPT_V4_PAYLOAD_BYTES
);
const _: () = assert!(ROW_WRITE_CHUNK_ROWS > 0);

/// The only two instrument families the engine may sweep.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InstrumentFamilyV1 {
    /// `NSE-NIFTY` spot index.
    Nifty,
    /// `NSE-BANKNIFTY` spot index.
    BankNifty,
}

impl InstrumentFamilyV1 {
    const fn byte(self) -> u8 {
        match self {
            Self::Nifty => 1,
            Self::BankNifty => 2,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, PopulationRefusal> {
        match byte {
            1 => Ok(Self::Nifty),
            2 => Ok(Self::BankNifty),
            _ => Err(format!(
                "instrument-family byte {byte} is unknown; only 1=NIFTY and 2=BANKNIFTY are version-one values"
            )),
        }
    }
}

/// Canonical inclusive month span requested from the stored market-data lake.
///
/// This is the request identity, not a summary of whichever months happened to
/// be present.  Keeping it beside the content digest prevents two differently
/// requested runs that encountered the same available bytes from aliasing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequestedSpanIdentityV1 {
    from_year: u16,
    from_month: u8,
    to_year: u16,
    to_month: u8,
}

impl RequestedSpanIdentityV1 {
    /// Builds one inclusive `(year, month)..=(year, month)` request identity.
    ///
    /// # Errors
    ///
    /// Refuses a year outside the store's `1970..=9999` domain, a month outside
    /// `1..=12`, or an end month before the start.
    pub fn new(
        from_year: u16,
        from_month: u8,
        to_year: u16,
        to_month: u8,
    ) -> Result<Self, PopulationRefusal> {
        for (name, year) in [("from_year", from_year), ("to_year", to_year)] {
            if !(1970..=9999).contains(&year) {
                return Err(format!(
                    "requested-span {name} {year} is invalid; stored years are 1970..=9999"
                ));
            }
        }
        for (name, month) in [("from_month", from_month), ("to_month", to_month)] {
            if !(1..=12).contains(&month) {
                return Err(format!(
                    "requested-span {name} {month} is invalid; canonical months are 1..=12"
                ));
            }
        }
        if (to_year, to_month) < (from_year, from_month) {
            return Err(format!(
                "requested inclusive span {from_year}-{from_month:02}..={to_year}-{to_month:02} steps backwards"
            ));
        }
        Ok(Self {
            from_year,
            from_month,
            to_year,
            to_month,
        })
    }

    /// Inclusive first year.
    #[must_use]
    pub const fn from_year(self) -> u16 {
        self.from_year
    }

    /// Inclusive first month in `1..=12`.
    #[must_use]
    pub const fn from_month(self) -> u8 {
        self.from_month
    }

    /// Inclusive last year.
    #[must_use]
    pub const fn to_year(self) -> u16 {
        self.to_year
    }

    /// Inclusive last month in `1..=12`.
    #[must_use]
    pub const fn to_month(self) -> u8 {
        self.to_month
    }

    /// Versioned fixed canonical bytes used by receipts and shared-cohort IDs.
    #[must_use]
    pub fn canonical_bytes(self) -> [u8; REQUESTED_SPAN_IDENTITY_BYTES] {
        let mut raw = [0_u8; REQUESTED_SPAN_IDENTITY_BYTES];
        for (slot, value) in raw.chunks_exact_mut(4).zip([
            REQUESTED_SPAN_VERSION,
            u32::from(self.from_year),
            u32::from(self.from_month),
            u32::from(self.to_year),
            u32::from(self.to_month),
        ]) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        raw
    }

    /// Domain-separated identity of the exact inclusive request.
    #[must_use]
    pub fn digest(self) -> [u8; 32] {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(REQUESTED_SPAN_DIGEST_DOMAIN);
        hasher.update(&self.canonical_bytes());
        hasher.finalize()
    }

    /// Returns the one shared term only when two populations requested exactly
    /// the same inclusive months.
    ///
    /// # Errors
    ///
    /// Refuses every endpoint mismatch; callers must not combine differently
    /// requested NIFTY and BANKNIFTY samples under one Top-N cohort.
    pub fn require_same(self, other: Self) -> Result<Self, PopulationRefusal> {
        if self == other {
            Ok(self)
        } else {
            Err(format!(
                "requested population spans differ: {}-{:02}..={}-{:02} versus {}-{:02}..={}-{:02}",
                self.from_year,
                self.from_month,
                self.to_year,
                self.to_month,
                other.from_year,
                other.from_month,
                other.to_year,
                other.to_month
            ))
        }
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), PopulationRefusal> {
        encoder.bytes(&self.canonical_bytes())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, PopulationRefusal> {
        let version = decoder.u32()?;
        if version != REQUESTED_SPAN_VERSION {
            return Err(format!(
                "requested-span identity version {version} is unknown; this build requires version {REQUESTED_SPAN_VERSION}"
            ));
        }
        let from_year = u16::try_from(decoder.u32()?)
            .map_err(|_| "requested-span from_year does not fit u16".to_owned())?;
        let from_month = u8::try_from(decoder.u32()?)
            .map_err(|_| "requested-span from_month does not fit u8".to_owned())?;
        let to_year = u16::try_from(decoder.u32()?)
            .map_err(|_| "requested-span to_year does not fit u16".to_owned())?;
        let to_month = u8::try_from(decoder.u32()?)
            .map_err(|_| "requested-span to_month does not fit u8".to_owned())?;
        Self::new(from_year, from_month, to_year, to_month)
    }
}

/// Explicit side of one expanded strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TradeDirectionV1 {
    /// Buy first, sell to close.
    Long,
    /// Sell first, buy to close.
    Short,
}

impl TradeDirectionV1 {
    const fn byte(self) -> u8 {
        match self {
            Self::Long => 1,
            Self::Short => 2,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, PopulationRefusal> {
        match byte {
            1 => Ok(Self::Long),
            2 => Ok(Self::Short),
            _ => Err(format!(
                "direction byte {byte} is unknown; only 1=long and 2=short are version-one values"
            )),
        }
    }
}

/// Exact closure classification inherited from the uncapped Apriori walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClosureV1 {
    /// No immediate superset had equal support.
    Closed,
    /// An equal-support immediate superset represents this itemset.
    Redundant,
    /// The successor frontier did not decide closure.
    Unknown,
}

impl ClosureV1 {
    const fn byte(self) -> u8 {
        match self {
            Self::Closed => 1,
            Self::Redundant => 2,
            Self::Unknown => 3,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, PopulationRefusal> {
        match byte {
            1 => Ok(Self::Closed),
            2 => Ok(Self::Redundant),
            3 => Ok(Self::Unknown),
            _ => Err(format!(
                "closure byte {byte} is unknown; only 1=closed, 2=redundant and 3=unknown are version-one values"
            )),
        }
    }
}

/// Terminal result of the fixed institutional admission stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdmissionStatusV1 {
    /// Every required check passed with measured evidence.
    Admitted,
    /// Measured evidence violated at least one policy check.
    Rejected,
    /// At least one required input was unmeasured and none was refused upstream.
    Unmeasured,
    /// At least one required input was explicitly refused upstream.
    Refused,
}

impl AdmissionStatusV1 {
    const fn byte(self) -> u8 {
        match self {
            Self::Admitted => 0,
            Self::Rejected => 1,
            Self::Unmeasured => 2,
            Self::Refused => 3,
        }
    }

    fn from_byte(byte: u8) -> Result<Self, PopulationRefusal> {
        match byte {
            0 => Ok(Self::Admitted),
            1 => Ok(Self::Rejected),
            2 => Ok(Self::Unmeasured),
            3 => Ok(Self::Refused),
            _ => Err(format!(
                "admission-status byte {byte} is unknown; version one defines only 0..=3"
            )),
        }
    }
}

/// One exact stop/target/trailing coordinate.
///
/// `None` is encoded separately from rung zero.  The trailing-take-profit arm
/// and trail are one option so a half-present pair is unrepresentable in memory
/// and refused on disk. `u32::MAX` is the on-disk `None` sentinel and is refused
/// as a present index. When a fixed target exists, the TTP must arm strictly
/// below it; when a live TSL exists, the armed trail must be strictly tighter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExitCoordinateV1 {
    /// Fixed-stop rung, or no fixed stop.
    pub stop: Option<u32>,
    /// Fixed-target rung, or no fixed target.
    pub target: Option<u32>,
    /// Trailing-stop-loss rung, or no TSL.
    pub tsl: Option<u32>,
    /// `(target arm rung, trailing distance rung)`, or no TTP.
    pub ttp: Option<(u32, u32)>,
}

impl ExitCoordinateV1 {
    fn validate(self) -> Result<(), PopulationRefusal> {
        for (name, index) in [
            ("stop", self.stop),
            ("target", self.target),
            ("tsl", self.tsl),
        ] {
            if index == Some(u32::MAX) {
                return Err(format!(
                    "exit {name} index u32::MAX is reserved for None and is not a canonical rung"
                ));
            }
        }
        let Some((arm, trail)) = self.ttp else {
            return Ok(());
        };
        if arm == u32::MAX || trail == u32::MAX {
            return Err(
                "exit TTP arm/trail index u32::MAX is reserved for None and is not canonical"
                    .to_owned(),
            );
        }
        if self.target.is_some_and(|target| arm >= target) {
            return Err(format!(
                "exit TTP arm index {arm} must be below fixed-target index {}",
                self.target.unwrap_or(u32::MAX)
            ));
        }
        if self.tsl.is_some_and(|tsl| trail >= tsl) {
            return Err(format!(
                "exit TTP trail index {trail} must be below live-TSL index {}",
                self.tsl.unwrap_or(u32::MAX)
            ));
        }
        Ok(())
    }
}

/// The complete final-ranking measurement vector.
///
/// Optional ratios retain an explicit domain tag on disk.  `None` is an
/// undefined denominator, not numeric zero. Winning plus losing trades is the
/// classified-trade denominator; both stored rates are its exact floor PPM
/// projections, including zero for a zero denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TopMetricsV1 {
    /// Maximum peak-to-trough loss magnitude, in paisa.
    pub drawdown: u64,
    /// Largest single losing-trade magnitude, in paisa.
    pub worst_loss: u64,
    /// Losing trades per million total trades.
    pub losing_rate_ppm: u64,
    /// Number of losing trades.
    pub losing_trades: u64,
    /// Gross-loss magnitude divided by gross win, in ppm.
    pub loss_ratio_ppm: Option<u64>,
    /// Worst-case total profit, signed paisa.
    pub pessimistic_profit: i64,
    /// Number of winning trades.
    pub winning_trades: u64,
    /// Winning trades per million total trades.
    pub win_rate_ppm: u64,
    /// Reward divided by risk, in ppm.
    pub reward_to_risk_ppm: Option<u64>,
    /// Average winning-trade magnitude, in paisa.
    pub average_win: u64,
    /// Average losing-trade magnitude, in paisa.
    pub average_loss: u64,
    /// Statistical lower-bound assurance tie-break, in ppm.
    pub assurance_ppm: u64,
}

/// All four durable institutional-admission reason partitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionV1 {
    /// Terminal classification.
    pub status: AdmissionStatusV1,
    /// Union of the three reason partitions.
    pub reasons: u64,
    /// Measured policy failures.
    pub failed: u64,
    /// Required evidence that was not measurable.
    pub unmeasured: u64,
    /// Evidence explicitly refused by an upstream validator.
    pub refused: u64,
}

impl AdmissionV1 {
    fn validate(self) -> Result<(), PopulationRefusal> {
        for (name, bits) in [
            ("reasons", self.reasons),
            ("failed", self.failed),
            ("unmeasured", self.unmeasured),
            ("refused", self.refused),
        ] {
            runner::admission::ReasonBits::from_bits(bits).ok_or_else(|| {
                format!("admission {name} contains unknown version-one reason bits")
            })?;
        }
        if self.failed & self.unmeasured != 0
            || self.failed & self.refused != 0
            || self.unmeasured & self.refused != 0
        {
            return Err(
                "admission failed, unmeasured and refused reason partitions overlap".to_owned(),
            );
        }
        if self.reasons != self.failed | self.unmeasured | self.refused {
            return Err(
                "admission reason union does not equal failed | unmeasured | refused".to_owned(),
            );
        }
        let expected = if self.refused != 0 {
            AdmissionStatusV1::Refused
        } else if self.unmeasured != 0 {
            AdmissionStatusV1::Unmeasured
        } else if self.failed != 0 {
            AdmissionStatusV1::Rejected
        } else {
            AdmissionStatusV1::Admitted
        };
        if self.status != expected {
            return Err(format!(
                "admission status {:?} disagrees with its reason partitions; the canonical status is {expected:?}",
                self.status
            ));
        }
        Ok(())
    }

    /// Verifies the legacy row summary against a recomputed runner verdict.
    ///
    /// This comparison does not authorize the verdict by itself; callers must
    /// first reconstruct a sealed decision from canonical policy and evidence.
    /// It prevents the historical caller-authored summary from disagreeing
    /// with that independently recomputed authority.
    ///
    /// # Errors
    ///
    /// Refuses a malformed legacy summary or any status/reason-partition
    /// mismatch with `verdict`.
    pub fn require_matches_verdict(
        self,
        verdict: runner::admission::AdmissionVerdictV1,
    ) -> Result<(), PopulationRefusal> {
        self.validate()?;
        let status = match verdict.status() {
            runner::admission::AdmissionStatusV1::Admitted => AdmissionStatusV1::Admitted,
            runner::admission::AdmissionStatusV1::Rejected => AdmissionStatusV1::Rejected,
            runner::admission::AdmissionStatusV1::Unmeasured => AdmissionStatusV1::Unmeasured,
            runner::admission::AdmissionStatusV1::Refused => AdmissionStatusV1::Refused,
        };
        let expected = Self {
            status,
            reasons: verdict.reasons().bits(),
            failed: verdict.failed().bits(),
            unmeasured: verdict.unmeasured().bits(),
            refused: verdict.refused().bits(),
        };
        if self == expected {
            Ok(())
        } else {
            Err(
                "population row admission summary differs from the recomputed sealed verdict"
                    .to_owned(),
            )
        }
    }
}

/// One canonical member of a complete, expanded population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationRowV1 {
    /// Identity shared by every row and its completion receipt.
    pub population_id: [u8; 32],
    /// Zero-based canonical position inside this population's contiguous block.
    pub sequence: u64,
    /// Digest of the full semantic strategy, including policy identities.
    pub strategy_digest: [u8; 32],
    /// Six append-only vocabulary words.
    pub mask_words: [u64; 6],
    /// Explicit long or short expansion.
    pub direction: TradeDirectionV1,
    /// NIFTY or BANKNIFTY spot index.
    pub instrument_family: InstrumentFamilyV1,
    /// Closure inherited from the itemset population.
    pub closure: ClosureV1,
    /// Signal timeframe in seconds; version one accepts the eight intraday rungs.
    pub rung_seconds: u32,
    /// Exact itemset support measured by the engine.
    pub support_hits: u64,
    /// Exact resolved exit coordinate.
    pub exit: ExitCoordinateV1,
    /// Every metric used by final Top-N ordering.
    pub metrics: TopMetricsV1,
    /// Full institutional admission outcome.
    pub admission: AdmissionV1,
}

impl PopulationRowV1 {
    fn validate(self) -> Result<(), PopulationRefusal> {
        require_nonzero_digest("population_id", &self.population_id)?;
        require_nonzero_digest("strategy_digest", &self.strategy_digest)?;
        if self.mask_words.iter().all(|word| *word == 0) {
            return Err("a population row has an empty condition mask".to_owned());
        }
        runner::replay_mask::from_stored_words(self.mask_words).map_err(|why| {
            format!("population row condition mask is not a canonical live vocabulary mask: {why}")
        })?;
        validate_rung(self.rung_seconds)?;
        self.exit.validate()?;
        if self.metrics.losing_rate_ppm > 1_000_000 {
            return Err(format!(
                "losing_rate_ppm {} exceeds the complete ppm domain",
                self.metrics.losing_rate_ppm
            ));
        }
        if self.metrics.win_rate_ppm > 1_000_000 {
            return Err(format!(
                "win_rate_ppm {} exceeds the complete ppm domain",
                self.metrics.win_rate_ppm
            ));
        }
        if self.metrics.assurance_ppm > 1_000_000 {
            return Err(format!(
                "assurance_ppm {} exceeds the complete ppm domain",
                self.metrics.assurance_ppm
            ));
        }
        let classified_trades = self
            .metrics
            .winning_trades
            .checked_add(self.metrics.losing_trades)
            .ok_or_else(|| "winning plus losing trade counts overflow u64".to_owned())?;
        let expected_win_rate = canonical_rate_ppm(self.metrics.winning_trades, classified_trades);
        if self.metrics.win_rate_ppm != expected_win_rate {
            return Err(format!(
                "win_rate_ppm {} is inconsistent with {}/{} winning/classified trades; canonical floor projection is {expected_win_rate}",
                self.metrics.win_rate_ppm, self.metrics.winning_trades, classified_trades
            ));
        }
        let expected_losing_rate =
            canonical_rate_ppm(self.metrics.losing_trades, classified_trades);
        if self.metrics.losing_rate_ppm != expected_losing_rate {
            return Err(format!(
                "losing_rate_ppm {} is inconsistent with {}/{} losing/classified trades; canonical floor projection is {expected_losing_rate}",
                self.metrics.losing_rate_ppm, self.metrics.losing_trades, classified_trades
            ));
        }
        self.admission.validate()
    }

    fn payload_bytes(self) -> Result<[u8; ROW_PAYLOAD_BYTES], PopulationRefusal> {
        self.validate()?;
        let mut raw = [0_u8; ROW_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.sequence)?;
        encoder.bytes(&self.strategy_digest)?;
        for word in self.mask_words {
            encoder.u64(word)?;
        }
        encoder.u8(self.direction.byte())?;
        encoder.u8(self.instrument_family.byte())?;
        encoder.u8(self.closure.byte())?;
        encoder.u8(self.admission.status.byte())?;
        encoder.u32(self.rung_seconds)?;
        encoder.u64(self.support_hits)?;
        encode_index(&mut encoder, self.exit.stop)?;
        encode_index(&mut encoder, self.exit.target)?;
        encode_index(&mut encoder, self.exit.tsl)?;
        let (ttp_arm, ttp_trail) = self
            .exit
            .ttp
            .map_or((None, None), |(arm, trail)| (Some(arm), Some(trail)));
        encode_index(&mut encoder, ttp_arm)?;
        encode_index(&mut encoder, ttp_trail)?;
        encoder.zeros(4)?;
        encoder.u64(self.metrics.drawdown)?;
        encoder.u64(self.metrics.worst_loss)?;
        encoder.u64(self.metrics.losing_rate_ppm)?;
        encoder.u64(self.metrics.losing_trades)?;
        encode_optional_u64(&mut encoder, self.metrics.loss_ratio_ppm)?;
        encoder.i64(self.metrics.pessimistic_profit)?;
        encoder.u64(self.metrics.winning_trades)?;
        encoder.u64(self.metrics.win_rate_ppm)?;
        encode_optional_u64(&mut encoder, self.metrics.reward_to_risk_ppm)?;
        encoder.u64(self.metrics.average_win)?;
        encoder.u64(self.metrics.average_loss)?;
        encoder.u64(self.metrics.assurance_ppm)?;
        encoder.u64(self.admission.reasons)?;
        encoder.u64(self.admission.failed)?;
        encoder.u64(self.admission.unmeasured)?;
        encoder.u64(self.admission.refused)?;
        encoder.finish()?;
        Ok(raw)
    }

    /// Domain-separated digest of the exact canonical 304-byte row payload.
    ///
    /// This is the population-owned binding used by the admission sidecar; the
    /// sidecar never reimplements or guesses this codec.
    ///
    /// # Errors
    ///
    /// Refuses every malformed row condition that prevents canonical encoding.
    pub fn payload_digest(self) -> Result<[u8; 32], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(ROW_PAYLOAD_DIGEST_DOMAIN);
        hasher.update(&payload);
        Ok(hasher.finalize())
    }

    fn to_bytes(self) -> Result<[u8; ROW_STRIDE_BYTES], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let seal = seal(&payload);
        let mut raw = [0_u8; ROW_STRIDE_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&payload)?;
        encoder.bytes(&seal)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; ROW_STRIDE_BYTES]) -> Result<Self, PopulationRefusal> {
        let payload = raw
            .get(..ROW_PAYLOAD_BYTES)
            .ok_or_else(|| "population row payload is absent".to_owned())?;
        let stored_seal = raw
            .get(ROW_PAYLOAD_BYTES..)
            .ok_or_else(|| "population row seal is absent".to_owned())?;
        if stored_seal != seal(payload) {
            return Err("population row failed its BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let population_id = decoder.array_32()?;
        let sequence = decoder.u64()?;
        let strategy_digest = decoder.array_32()?;
        let mut mask_words = [0_u64; 6];
        for word in &mut mask_words {
            *word = decoder.u64()?;
        }
        let direction = TradeDirectionV1::from_byte(decoder.u8()?)?;
        let instrument_family = InstrumentFamilyV1::from_byte(decoder.u8()?)?;
        let closure = ClosureV1::from_byte(decoder.u8()?)?;
        let status = AdmissionStatusV1::from_byte(decoder.u8()?)?;
        let rung_seconds = decoder.u32()?;
        let support_hits = decoder.u64()?;
        let stop = decode_index(&mut decoder)?;
        let target = decode_index(&mut decoder)?;
        let tsl = decode_index(&mut decoder)?;
        let ttp_arm = decode_index(&mut decoder)?;
        let ttp_trail = decode_index(&mut decoder)?;
        let ttp = match (ttp_arm, ttp_trail) {
            (None, None) => None,
            (Some(arm), Some(trail)) => Some((arm, trail)),
            _ => {
                return Err(
                    "population row has a half-present trailing-take-profit coordinate".to_owned(),
                );
            }
        };
        decoder.zeros(4, "population row coordinate reserve")?;
        let metrics = TopMetricsV1 {
            drawdown: decoder.u64()?,
            worst_loss: decoder.u64()?,
            losing_rate_ppm: decoder.u64()?,
            losing_trades: decoder.u64()?,
            loss_ratio_ppm: decode_optional_u64(&mut decoder, "loss_ratio_ppm")?,
            pessimistic_profit: decoder.i64()?,
            winning_trades: decoder.u64()?,
            win_rate_ppm: decoder.u64()?,
            reward_to_risk_ppm: decode_optional_u64(&mut decoder, "reward_to_risk_ppm")?,
            average_win: decoder.u64()?,
            average_loss: decoder.u64()?,
            assurance_ppm: decoder.u64()?,
        };
        let admission = AdmissionV1 {
            status,
            reasons: decoder.u64()?,
            failed: decoder.u64()?,
            unmeasured: decoder.u64()?,
            refused: decoder.u64()?,
        };
        decoder.finish()?;
        let row = Self {
            population_id,
            sequence,
            strategy_digest,
            mask_words,
            direction,
            instrument_family,
            closure,
            rung_seconds,
            support_hits,
            exit: ExitCoordinateV1 {
                stop,
                target,
                tsl,
                ttp,
            },
            metrics,
            admission,
        };
        row.validate()?;
        Ok(row)
    }
}

/// Every identity needed to interpret and replay one population.
///
/// Each field is a full digest rather than a path or mutable label.  There is
/// deliberately no default; absence is refused as an all-zero digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationIdentitiesV1 {
    /// Nine-term signal-run identity.
    pub run_identity: [u8; 32],
    /// Exact stored market-data digest.
    pub data_digest: [u8; 32],
    /// Feed/vendor identity digest.
    pub feed_digest: [u8; 32],
    /// Source commit identity digest.
    pub source_commit_digest: [u8; 32],
    /// Append-only vocabulary identity.
    pub vocabulary_digest: [u8; 32],
    /// Entry/evaluation policy identity.
    pub evaluation_policy_digest: [u8; 32],
    /// Exit-grid policy identity.
    pub exit_grid_policy_digest: [u8; 32],
    /// Exact TRAINING-resolved exit-grid identity.
    pub resolved_exit_grid_digest: [u8; 32],
    /// Institutional admission policy identity.
    pub admission_policy_digest: [u8; 32],
    /// Final Top-N ranking policy identity.
    pub ranking_policy_digest: [u8; 32],
    /// IST trading-calendar policy identity.
    pub calendar_policy_digest: [u8; 32],
    /// Causal prior-day reference policy identity.
    pub daily_reference_policy_digest: [u8; 32],
}

impl PopulationIdentitiesV1 {
    fn validate(self) -> Result<(), PopulationRefusal> {
        for (name, digest) in [
            ("run_identity", self.run_identity),
            ("data_digest", self.data_digest),
            ("feed_digest", self.feed_digest),
            ("source_commit_digest", self.source_commit_digest),
            ("vocabulary_digest", self.vocabulary_digest),
            ("evaluation_policy_digest", self.evaluation_policy_digest),
            ("exit_grid_policy_digest", self.exit_grid_policy_digest),
            ("resolved_exit_grid_digest", self.resolved_exit_grid_digest),
            ("admission_policy_digest", self.admission_policy_digest),
            ("ranking_policy_digest", self.ranking_policy_digest),
            ("calendar_policy_digest", self.calendar_policy_digest),
            (
                "daily_reference_policy_digest",
                self.daily_reference_policy_digest,
            ),
        ] {
            require_nonzero_digest(name, &digest)?;
        }
        Ok(())
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), PopulationRefusal> {
        for digest in [
            self.run_identity,
            self.data_digest,
            self.feed_digest,
            self.source_commit_digest,
            self.vocabulary_digest,
            self.evaluation_policy_digest,
            self.exit_grid_policy_digest,
            self.resolved_exit_grid_digest,
            self.admission_policy_digest,
            self.ranking_policy_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, PopulationRefusal> {
        let out = Self {
            run_identity: decoder.array_32()?,
            data_digest: decoder.array_32()?,
            feed_digest: decoder.array_32()?,
            source_commit_digest: decoder.array_32()?,
            vocabulary_digest: decoder.array_32()?,
            evaluation_policy_digest: decoder.array_32()?,
            exit_grid_policy_digest: decoder.array_32()?,
            resolved_exit_grid_digest: decoder.array_32()?,
            admission_policy_digest: decoder.array_32()?,
            ranking_policy_digest: decoder.array_32()?,
            calendar_policy_digest: decoder.array_32()?,
            daily_reference_policy_digest: decoder.array_32()?,
        };
        out.validate()?;
        Ok(out)
    }
}

/// Upstream reconciliation facts that cannot be reconstructed from expanded rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionReconciliationV1 {
    /// Every candidate support test, frequent plus infrequent.
    pub sweep_trials: u64,
    /// Frequent itemsets streamed at frontier retirement.
    pub frequent_itemsets: u64,
    /// Infrequent candidates rejected by support.
    pub infrequent_itemsets: u64,
    /// Closed frequent itemsets expanded into strategies.
    pub closed_itemsets: u64,
    /// Equal-support redundant frequent itemsets.
    pub redundant_itemsets: u64,
    /// Frequent itemsets whose closure could not be decided.
    pub unknown_closure_itemsets: u64,
    /// Expected `(closed mask, direction)` pairs before exit expansion.
    pub direction_members_expected: u64,
    /// Expected exact exit-cell strategies after expansion.
    pub exit_cells_expected: u64,
    /// Last combination depth processed before frequent-frontier extinction.
    pub extinction_depth: u32,
    /// Whether the ladder reached natural frequent-frontier extinction.
    pub extinction_complete: bool,
    /// Whether every frequent itemset received a terminal closure verdict.
    pub closure_complete: bool,
}

/// Durable proof that one contiguous row block is a complete population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionReceiptV1 {
    /// Identity shared by the committed row block.
    pub population_id: [u8; 32],
    /// Exact row count, including a legitimate zero.
    pub row_count: u64,
    /// BLAKE3 over every canonical row payload in sequence order.
    pub ordered_row_digest: [u8; 32],
    /// Every support test performed by the extinct ladder.
    pub sweep_trials: u64,
    /// Frequent itemsets retired by the ladder.
    pub frequent_itemsets: u64,
    /// Infrequent support-test candidates.
    pub infrequent_itemsets: u64,
    /// Closed frequent itemsets.
    pub closed_itemsets: u64,
    /// Redundant equal-support itemsets.
    pub redundant_itemsets: u64,
    /// Closure-unknown itemsets; zero is required for completion.
    pub unknown_closure_itemsets: u64,
    /// Expected `(closed mask, direction)` count.
    pub direction_members_expected: u64,
    /// Distinct `(closed mask, direction)` pairs actually persisted.
    pub direction_members_evaluated: u64,
    /// Expected exact exit-cell strategies.
    pub exit_cells_expected: u64,
    /// Exact exit-cell rows actually persisted.
    pub exit_cells_evaluated: u64,
    /// Rows admitted by every institutional check.
    pub admitted_rows: u64,
    /// Rows rejected on measured policy evidence.
    pub rejected_rows: u64,
    /// Rows with at least one unmeasured admission input.
    pub unmeasured_rows: u64,
    /// Rows explicitly refused upstream.
    pub refused_rows: u64,
    /// Rows with at least one undefined optional Top-N ratio.
    pub topn_undefined_rows: u64,
    /// Last combination depth processed before extinction.
    pub extinction_depth: u32,
    /// Signal timeframe, retained even for a zero-row population.
    pub rung_seconds: u32,
    /// NIFTY or BANKNIFTY, retained even for a zero-row population.
    pub instrument_family: InstrumentFamilyV1,
    /// Whether natural frequent-frontier extinction completed.
    pub extinction_complete: bool,
    /// Whether every frequent itemset received a closure verdict.
    pub closure_complete: bool,
    /// Policy, data, feed, commit, calendar and reference identities.
    pub identities: PopulationIdentitiesV1,
}

impl CompletionReceiptV1 {
    /// Builds a receipt from explicit upstream reconciliation and the exact rows.
    ///
    /// Counts reconstructible from rows are computed here rather than trusted
    /// from a second caller-maintained tally.
    ///
    /// # Errors
    ///
    /// Refuses malformed rows, sequence/identity drift, duplicate strategies or
    /// coordinates, an incomplete reconciliation, or an absent identity.
    pub fn for_rows(
        population_id: [u8; 32],
        instrument_family: InstrumentFamilyV1,
        rung_seconds: u32,
        rows: &[PopulationRowV1],
        reconciliation: CompletionReconciliationV1,
        identities: PopulationIdentitiesV1,
    ) -> Result<Self, PopulationRefusal> {
        let facts = facts_of_rows(population_id, rows)?;
        let receipt = Self {
            population_id,
            row_count: facts.block.count,
            ordered_row_digest: facts.digest,
            sweep_trials: reconciliation.sweep_trials,
            frequent_itemsets: reconciliation.frequent_itemsets,
            infrequent_itemsets: reconciliation.infrequent_itemsets,
            closed_itemsets: reconciliation.closed_itemsets,
            redundant_itemsets: reconciliation.redundant_itemsets,
            unknown_closure_itemsets: reconciliation.unknown_closure_itemsets,
            direction_members_expected: reconciliation.direction_members_expected,
            direction_members_evaluated: facts.direction_members,
            exit_cells_expected: reconciliation.exit_cells_expected,
            exit_cells_evaluated: facts.block.count,
            admitted_rows: facts.admitted,
            rejected_rows: facts.rejected,
            unmeasured_rows: facts.unmeasured,
            refused_rows: facts.refused,
            topn_undefined_rows: facts.topn_undefined,
            extinction_depth: reconciliation.extinction_depth,
            rung_seconds,
            instrument_family,
            extinction_complete: reconciliation.extinction_complete,
            closure_complete: reconciliation.closure_complete,
            identities,
        };
        receipt.validate_semantics()?;
        validate_receipt_against_facts(&receipt, Some(&facts))?;
        Ok(receipt)
    }

    fn validate_semantics(self) -> Result<(), PopulationRefusal> {
        require_nonzero_digest("population_id", &self.population_id)?;
        require_nonzero_digest("ordered_row_digest", &self.ordered_row_digest)?;
        validate_rung(self.rung_seconds)?;
        self.identities.validate()?;
        let classified = checked_sum(
            &[
                self.admitted_rows,
                self.rejected_rows,
                self.unmeasured_rows,
                self.refused_rows,
            ],
            "admission row counts",
        )?;
        if classified != self.row_count {
            return Err(format!(
                "admission row counts total {classified}, not receipt row_count {}",
                self.row_count
            ));
        }
        if self.topn_undefined_rows > self.row_count {
            return Err(format!(
                "topn_undefined_rows {} exceeds row_count {}",
                self.topn_undefined_rows, self.row_count
            ));
        }
        let trials = self
            .frequent_itemsets
            .checked_add(self.infrequent_itemsets)
            .ok_or_else(|| "frequent plus infrequent itemsets overflow u64".to_owned())?;
        if trials != self.sweep_trials {
            return Err(format!(
                "frequent + infrequent itemsets is {trials}, not sweep_trials {}",
                self.sweep_trials
            ));
        }
        let classified_frequent = checked_sum(
            &[
                self.closed_itemsets,
                self.redundant_itemsets,
                self.unknown_closure_itemsets,
            ],
            "closure itemset counts",
        )?;
        if classified_frequent != self.frequent_itemsets {
            return Err(format!(
                "closed + redundant + unknown itemsets is {classified_frequent}, not frequent_itemsets {}",
                self.frequent_itemsets
            ));
        }
        if !self.extinction_complete {
            return Err(
                "a completion receipt cannot claim an incomplete extinction walk".to_owned(),
            );
        }
        if !self.closure_complete || self.unknown_closure_itemsets != 0 {
            return Err(
                "a completion receipt requires closure_complete=true and zero unknown closure itemsets"
                    .to_owned(),
            );
        }
        let expected_directions = self
            .closed_itemsets
            .checked_mul(2)
            .ok_or_else(|| "closed itemsets times two directions overflow u64".to_owned())?;
        if self.direction_members_expected != expected_directions {
            return Err(format!(
                "direction_members_expected {} is not closed_itemsets {} times long+short",
                self.direction_members_expected, self.closed_itemsets
            ));
        }
        if self.direction_members_evaluated != self.direction_members_expected {
            return Err(format!(
                "evaluated direction members {} do not reconcile to expected {}",
                self.direction_members_evaluated, self.direction_members_expected
            ));
        }
        if self.exit_cells_expected != self.exit_cells_evaluated
            || self.exit_cells_evaluated != self.row_count
        {
            return Err(format!(
                "exit-cell expected/evaluated/row counts are {}/{}/{}, not one exact population",
                self.exit_cells_expected, self.exit_cells_evaluated, self.row_count
            ));
        }
        Ok(())
    }

    fn payload_bytes(self) -> Result<[u8; RECEIPT_V1_PAYLOAD_BYTES], PopulationRefusal> {
        self.validate_semantics()?;
        let mut raw = [0_u8; RECEIPT_V1_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_count)?;
        encoder.bytes(&self.ordered_row_digest)?;
        for count in [
            self.sweep_trials,
            self.frequent_itemsets,
            self.infrequent_itemsets,
            self.closed_itemsets,
            self.redundant_itemsets,
            self.unknown_closure_itemsets,
            self.direction_members_expected,
            self.direction_members_evaluated,
            self.exit_cells_expected,
            self.exit_cells_evaluated,
            self.admitted_rows,
            self.rejected_rows,
            self.unmeasured_rows,
            self.refused_rows,
            self.topn_undefined_rows,
        ] {
            encoder.u64(count)?;
        }
        encoder.u32(self.extinction_depth)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u8(self.instrument_family.byte())?;
        encoder.u8(u8::from(self.extinction_complete))?;
        encoder.u8(u8::from(self.closure_complete))?;
        encoder.zeros(5)?;
        self.identities.encode(&mut encoder)?;
        encoder.zeros(8)?;
        encoder.finish()?;
        Ok(raw)
    }

    /// Encodes the legacy receipt without granting it V2 commit authority.
    ///
    /// # Errors
    ///
    /// Refuses any non-canonical V1 field or count relationship.
    pub fn to_bytes(self) -> Result<[u8; RECEIPT_V1_STRIDE_BYTES], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let seal = seal(&payload);
        let mut raw = [0_u8; RECEIPT_V1_STRIDE_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&payload)?;
        encoder.bytes(&seal)?;
        encoder.finish()?;
        Ok(raw)
    }

    /// Decodes legacy V1 bytes for explicit audit or migration tooling.
    ///
    /// Decoding does not make this receipt authoritative in
    /// [`PopulationLedger`]; only a freshly reconciled [`CompletionReceiptV2`]
    /// can commit the shared V1 row block.
    ///
    /// # Errors
    ///
    /// Refuses a bad seal, reserve, tag, identity or V1 count relationship.
    pub fn from_bytes(raw: &[u8; RECEIPT_V1_STRIDE_BYTES]) -> Result<Self, PopulationRefusal> {
        let payload = raw
            .get(..RECEIPT_V1_PAYLOAD_BYTES)
            .ok_or_else(|| "population receipt payload is absent".to_owned())?;
        let stored_seal = raw
            .get(RECEIPT_V1_PAYLOAD_BYTES..)
            .ok_or_else(|| "population receipt seal is absent".to_owned())?;
        if stored_seal != seal(payload) {
            return Err("population completion receipt failed its BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let population_id = decoder.array_32()?;
        let row_count = decoder.u64()?;
        let ordered_row_digest = decoder.array_32()?;
        let receipt = Self {
            population_id,
            row_count,
            ordered_row_digest,
            sweep_trials: decoder.u64()?,
            frequent_itemsets: decoder.u64()?,
            infrequent_itemsets: decoder.u64()?,
            closed_itemsets: decoder.u64()?,
            redundant_itemsets: decoder.u64()?,
            unknown_closure_itemsets: decoder.u64()?,
            direction_members_expected: decoder.u64()?,
            direction_members_evaluated: decoder.u64()?,
            exit_cells_expected: decoder.u64()?,
            exit_cells_evaluated: decoder.u64()?,
            admitted_rows: decoder.u64()?,
            rejected_rows: decoder.u64()?,
            unmeasured_rows: decoder.u64()?,
            refused_rows: decoder.u64()?,
            topn_undefined_rows: decoder.u64()?,
            extinction_depth: decoder.u32()?,
            rung_seconds: decoder.u32()?,
            instrument_family: InstrumentFamilyV1::from_byte(decoder.u8()?)?,
            extinction_complete: decode_bool(decoder.u8()?, "extinction_complete")?,
            closure_complete: decode_bool(decoder.u8()?, "closure_complete")?,
            identities: {
                decoder.zeros(5, "population receipt flag reserve")?;
                PopulationIdentitiesV1::decode(&mut decoder)?
            },
        };
        decoder.zeros(8, "population receipt trailing reserve")?;
        decoder.finish()?;
        receipt.validate_semantics()?;
        Ok(receipt)
    }
}

/// One side-specific exit-policy and exact TRAINING-resolution identity.
///
/// The direction is supplied by its canonical field in
/// [`LongShortExitGridIdentitiesV2`], not by a caller-provided tag that could be
/// duplicated or mislabeled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideExitGridIdentityV2 {
    /// Identity of the side-specific grid construction and fill policy.
    pub policy_digest: [u8; 32],
    /// Identity of the exact side-specific TRAINING-resolved grid.
    pub resolved_digest: [u8; 32],
}

impl SideExitGridIdentityV2 {
    fn validate(self, side: &str) -> Result<(), PopulationRefusal> {
        require_nonzero_digest(
            &format!("{side}_exit_grid_policy_digest"),
            &self.policy_digest,
        )?;
        require_nonzero_digest(
            &format!("{side}_resolved_exit_grid_digest"),
            &self.resolved_digest,
        )
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), PopulationRefusal> {
        encoder.bytes(&self.policy_digest)?;
        encoder.bytes(&self.resolved_digest)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, PopulationRefusal> {
        Ok(Self {
            policy_digest: decoder.array_32()?,
            resolved_digest: decoder.array_32()?,
        })
    }
}

/// Canonical long-plus-short exit-grid identity for one population.
///
/// Long is always encoded and hashed first with tag `1`; short is always second
/// with tag `2`.  Repeating one side in both fields is refused, and swapping the
/// two fields changes the composite digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LongShortExitGridIdentitiesV2 {
    /// Exact long-side policy and TRAINING resolution.
    pub long: SideExitGridIdentityV2,
    /// Exact short-side policy and TRAINING resolution.
    pub short: SideExitGridIdentityV2,
}

impl LongShortExitGridIdentitiesV2 {
    fn validate(self) -> Result<(), PopulationRefusal> {
        self.long.validate("long")?;
        self.short.validate("short")?;
        if self.long == self.short {
            return Err(
                "long and short exit-grid identities are identical; one side may have been omitted or copied"
                    .to_owned(),
            );
        }
        Ok(())
    }

    /// Direction-tagged digest of both side-specific identities.
    ///
    /// # Errors
    ///
    /// Refuses an absent digest or one side copied into both canonical fields.
    pub fn composite_digest(self) -> Result<[u8; 32], PopulationRefusal> {
        self.validate()?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"brutex-long-short-exit-grids-v2\0");
        hasher.update(&[TradeDirectionV1::Long.byte()]);
        hasher.update(&self.long.policy_digest);
        hasher.update(&self.long.resolved_digest);
        hasher.update(&[TradeDirectionV1::Short.byte()]);
        hasher.update(&self.short.policy_digest);
        hasher.update(&self.short.resolved_digest);
        Ok(hasher.finalize())
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), PopulationRefusal> {
        self.validate()?;
        self.long.encode(encoder)?;
        self.short.encode(encoder)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, PopulationRefusal> {
        let identities = Self {
            long: SideExitGridIdentityV2::decode(decoder)?,
            short: SideExitGridIdentityV2::decode(decoder)?,
        };
        identities.validate()?;
        Ok(identities)
    }
}

/// Every identity needed to interpret and replay one authoritative population.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationIdentitiesV2 {
    /// Nine-term signal-run identity.
    pub run_identity: [u8; 32],
    /// Exact stored market-data digest.
    pub data_digest: [u8; 32],
    /// Feed/vendor identity digest.
    pub feed_digest: [u8; 32],
    /// Source commit identity digest.
    pub source_commit_digest: [u8; 32],
    /// Append-only vocabulary identity.
    pub vocabulary_digest: [u8; 32],
    /// Entry/evaluation policy identity.
    pub evaluation_policy_digest: [u8; 32],
    /// Canonical side-specific exit identities.
    pub exit_grids: LongShortExitGridIdentitiesV2,
    /// Institutional admission policy identity.
    pub admission_policy_digest: [u8; 32],
    /// Final Top-N ranking policy identity.
    pub ranking_policy_digest: [u8; 32],
    /// IST trading-calendar policy identity.
    pub calendar_policy_digest: [u8; 32],
    /// Causal prior-day reference policy identity.
    pub daily_reference_policy_digest: [u8; 32],
}

impl PopulationIdentitiesV2 {
    fn validate(self) -> Result<(), PopulationRefusal> {
        for (name, digest) in [
            ("run_identity", self.run_identity),
            ("data_digest", self.data_digest),
            ("feed_digest", self.feed_digest),
            ("source_commit_digest", self.source_commit_digest),
            ("vocabulary_digest", self.vocabulary_digest),
            ("evaluation_policy_digest", self.evaluation_policy_digest),
            ("admission_policy_digest", self.admission_policy_digest),
            ("ranking_policy_digest", self.ranking_policy_digest),
            ("calendar_policy_digest", self.calendar_policy_digest),
            (
                "daily_reference_policy_digest",
                self.daily_reference_policy_digest,
            ),
        ] {
            require_nonzero_digest(name, &digest)?;
        }
        self.exit_grids.validate()
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), PopulationRefusal> {
        self.validate()?;
        for digest in [
            self.run_identity,
            self.data_digest,
            self.feed_digest,
            self.source_commit_digest,
            self.vocabulary_digest,
            self.evaluation_policy_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        self.exit_grids.encode(encoder)?;
        for digest in [
            self.admission_policy_digest,
            self.ranking_policy_digest,
            self.calendar_policy_digest,
            self.daily_reference_policy_digest,
        ] {
            encoder.bytes(&digest)?;
        }
        Ok(())
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, PopulationRefusal> {
        let identities = Self {
            run_identity: decoder.array_32()?,
            data_digest: decoder.array_32()?,
            feed_digest: decoder.array_32()?,
            source_commit_digest: decoder.array_32()?,
            vocabulary_digest: decoder.array_32()?,
            evaluation_policy_digest: decoder.array_32()?,
            exit_grids: LongShortExitGridIdentitiesV2::decode(decoder)?,
            admission_policy_digest: decoder.array_32()?,
            ranking_policy_digest: decoder.array_32()?,
            calendar_policy_digest: decoder.array_32()?,
            daily_reference_policy_digest: decoder.array_32()?,
        };
        identities.validate()?;
        Ok(identities)
    }
}

/// Exact side-specific number of resolved exit cells expanded for every mask.
///
/// Construction is fallible so a missing side cannot be represented by a
/// canonical completion request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitCellsPerMaskV2 {
    long: u64,
    short: u64,
}

impl ExitCellsPerMaskV2 {
    /// Creates the exact per-side grid cardinalities.
    ///
    /// # Errors
    ///
    /// Refuses zero for either side: a zero would erase that direction from the
    /// purported long-plus-short population.
    pub fn new(long: u64, short: u64) -> Result<Self, PopulationRefusal> {
        if long == 0 || short == 0 {
            return Err(format!(
                "both long and short exit grids must contain at least one exact cell; received long={long}, short={short}"
            ));
        }
        Ok(Self { long, short })
    }

    /// Exact long-grid cells expanded for each closed mask.
    #[must_use]
    pub const fn long(self) -> u64 {
        self.long
    }

    /// Exact short-grid cells expanded for each closed mask.
    #[must_use]
    pub const fn short(self) -> u64 {
        self.short
    }
}

/// Upstream reconciliation facts for an authoritative V2 completion.
///
/// There is no caller-supplied direction total or row total.  Both are derived
/// with checked arithmetic from `closed_itemsets` and the typed per-side grid
/// cardinalities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionReconciliationV2 {
    /// Every candidate support test, frequent plus infrequent.
    pub sweep_trials: u64,
    /// Frequent itemsets streamed at frontier retirement.
    pub frequent_itemsets: u64,
    /// Infrequent candidates rejected by support.
    pub infrequent_itemsets: u64,
    /// Closed frequent masks expanded into both directions.
    pub closed_itemsets: u64,
    /// Equal-support redundant frequent itemsets.
    pub redundant_itemsets: u64,
    /// Frequent itemsets whose closure could not be decided.
    pub unknown_closure_itemsets: u64,
    /// Exact long and short resolved-grid cardinalities per closed mask.
    pub exit_cells_per_mask: ExitCellsPerMaskV2,
    /// Last combination depth processed before frequent-frontier extinction.
    pub extinction_depth: u32,
    /// Whether the ladder reached natural frequent-frontier extinction.
    pub extinction_complete: bool,
    /// Whether every frequent itemset received a terminal closure verdict.
    pub closure_complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExpectedPopulationCountsV2 {
    directions: u64,
    long_cells: u64,
    short_cells: u64,
    rows: u64,
}

fn expected_population_counts_v2(
    closed_masks: u64,
    cells: ExitCellsPerMaskV2,
) -> Result<ExpectedPopulationCountsV2, PopulationRefusal> {
    let directions = closed_masks.checked_mul(2).ok_or_else(|| {
        format!("closed mask count {closed_masks} times the two canonical directions overflows u64")
    })?;
    let long_cells = closed_masks.checked_mul(cells.long()).ok_or_else(|| {
        format!(
            "closed mask count {closed_masks} times long grid cells {} overflows u64",
            cells.long()
        )
    })?;
    let short_cells = closed_masks.checked_mul(cells.short()).ok_or_else(|| {
        format!(
            "closed mask count {closed_masks} times short grid cells {} overflows u64",
            cells.short()
        )
    })?;
    let rows = long_cells.checked_add(short_cells).ok_or_else(|| {
        format!("derived long plus short population rows {long_cells}+{short_cells} overflow u64")
    })?;
    Ok(ExpectedPopulationCountsV2 {
        directions,
        long_cells,
        short_cells,
        rows,
    })
}

/// Version-two proof retained for exact audit and embedded intact in V3.
///
/// It is not authoritative for new combined selection because it does not bind
/// the inclusive requested month span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionReceiptV2 {
    /// Identity shared by the committed row block.
    pub population_id: [u8; 32],
    /// Exact row count, including a legitimate zero-mask population.
    pub row_count: u64,
    /// BLAKE3 over every canonical row payload in sequence order.
    pub ordered_row_digest: [u8; 32],
    /// Every support test performed by the extinct ladder.
    pub sweep_trials: u64,
    /// Frequent itemsets retired by the ladder.
    pub frequent_itemsets: u64,
    /// Infrequent support-test candidates.
    pub infrequent_itemsets: u64,
    /// Closed frequent masks.
    pub closed_itemsets: u64,
    /// Redundant equal-support itemsets.
    pub redundant_itemsets: u64,
    /// Closure-unknown itemsets; zero is required for completion.
    pub unknown_closure_itemsets: u64,
    /// Derived `(closed mask, long|short)` count.
    pub direction_members_expected: u64,
    /// Distinct `(closed mask, direction)` pairs actually persisted.
    pub direction_members_evaluated: u64,
    /// Exact long-grid cells required for each closed mask.
    pub long_exit_cells_per_mask: u64,
    /// Exact short-grid cells required for each closed mask.
    pub short_exit_cells_per_mask: u64,
    /// Long rows actually persisted.
    pub long_exit_cells_evaluated: u64,
    /// Short rows actually persisted.
    pub short_exit_cells_evaluated: u64,
    /// Derived total exact exit-cell strategies.
    pub exit_cells_expected: u64,
    /// Exact exit-cell rows actually persisted.
    pub exit_cells_evaluated: u64,
    /// Rows admitted by every institutional check.
    pub admitted_rows: u64,
    /// Rows rejected on measured policy evidence.
    pub rejected_rows: u64,
    /// Rows with at least one unmeasured admission input.
    pub unmeasured_rows: u64,
    /// Rows explicitly refused upstream.
    pub refused_rows: u64,
    /// Rows with at least one undefined optional Top-N ratio.
    pub topn_undefined_rows: u64,
    /// Last combination depth processed before extinction.
    pub extinction_depth: u32,
    /// Signal timeframe, retained even for a zero-row population.
    pub rung_seconds: u32,
    /// NIFTY or BANKNIFTY, retained even for a zero-row population.
    pub instrument_family: InstrumentFamilyV1,
    /// Whether natural frequent-frontier extinction completed.
    pub extinction_complete: bool,
    /// Whether every frequent itemset received a closure verdict.
    pub closure_complete: bool,
    /// Data, policy and canonical side-specific grid identities.
    pub identities: PopulationIdentitiesV2,
}

impl CompletionReceiptV2 {
    /// Builds an authoritative receipt from exact typed shape facts and rows.
    ///
    /// # Errors
    ///
    /// Refuses malformed or partial rows, a missing/swapped/copied side
    /// identity, checked-count overflow, incomplete closure/extinction, or any
    /// mask whose long or short coordinate population is not exact.
    pub fn for_rows(
        population_id: [u8; 32],
        instrument_family: InstrumentFamilyV1,
        rung_seconds: u32,
        rows: &[PopulationRowV1],
        reconciliation: CompletionReconciliationV2,
        identities: PopulationIdentitiesV2,
    ) -> Result<Self, PopulationRefusal> {
        let expected = expected_population_counts_v2(
            reconciliation.closed_itemsets,
            reconciliation.exit_cells_per_mask,
        )?;
        let facts = facts_of_rows(population_id, rows)?;
        let receipt = Self {
            population_id,
            row_count: facts.block.count,
            ordered_row_digest: facts.digest,
            sweep_trials: reconciliation.sweep_trials,
            frequent_itemsets: reconciliation.frequent_itemsets,
            infrequent_itemsets: reconciliation.infrequent_itemsets,
            closed_itemsets: reconciliation.closed_itemsets,
            redundant_itemsets: reconciliation.redundant_itemsets,
            unknown_closure_itemsets: reconciliation.unknown_closure_itemsets,
            direction_members_expected: expected.directions,
            direction_members_evaluated: facts.direction_members,
            long_exit_cells_per_mask: reconciliation.exit_cells_per_mask.long(),
            short_exit_cells_per_mask: reconciliation.exit_cells_per_mask.short(),
            long_exit_cells_evaluated: facts.long_rows,
            short_exit_cells_evaluated: facts.short_rows,
            exit_cells_expected: expected.rows,
            exit_cells_evaluated: facts.block.count,
            admitted_rows: facts.admitted,
            rejected_rows: facts.rejected,
            unmeasured_rows: facts.unmeasured,
            refused_rows: facts.refused,
            topn_undefined_rows: facts.topn_undefined,
            extinction_depth: reconciliation.extinction_depth,
            rung_seconds,
            instrument_family,
            extinction_complete: reconciliation.extinction_complete,
            closure_complete: reconciliation.closure_complete,
            identities,
        };
        receipt.validate_semantics()?;
        validate_receipt_v2_against_facts(&receipt, Some(&facts))?;
        Ok(receipt)
    }

    fn cells_per_mask(self) -> Result<ExitCellsPerMaskV2, PopulationRefusal> {
        ExitCellsPerMaskV2::new(
            self.long_exit_cells_per_mask,
            self.short_exit_cells_per_mask,
        )
    }

    fn validate_semantics(self) -> Result<(), PopulationRefusal> {
        require_nonzero_digest("population_id", &self.population_id)?;
        require_nonzero_digest("ordered_row_digest", &self.ordered_row_digest)?;
        validate_rung(self.rung_seconds)?;
        self.identities.validate()?;
        let classified = checked_sum(
            &[
                self.admitted_rows,
                self.rejected_rows,
                self.unmeasured_rows,
                self.refused_rows,
            ],
            "admission row counts",
        )?;
        if classified != self.row_count {
            return Err(format!(
                "admission row counts total {classified}, not receipt row_count {}",
                self.row_count
            ));
        }
        if self.topn_undefined_rows > self.row_count {
            return Err(format!(
                "topn_undefined_rows {} exceeds row_count {}",
                self.topn_undefined_rows, self.row_count
            ));
        }
        let trials = self
            .frequent_itemsets
            .checked_add(self.infrequent_itemsets)
            .ok_or_else(|| "frequent plus infrequent itemsets overflow u64".to_owned())?;
        if trials != self.sweep_trials {
            return Err(format!(
                "frequent + infrequent itemsets is {trials}, not sweep_trials {}",
                self.sweep_trials
            ));
        }
        let classified_frequent = checked_sum(
            &[
                self.closed_itemsets,
                self.redundant_itemsets,
                self.unknown_closure_itemsets,
            ],
            "closure itemset counts",
        )?;
        if classified_frequent != self.frequent_itemsets {
            return Err(format!(
                "closed + redundant + unknown itemsets is {classified_frequent}, not frequent_itemsets {}",
                self.frequent_itemsets
            ));
        }
        if !self.extinction_complete {
            return Err(
                "a completion receipt cannot claim an incomplete extinction walk".to_owned(),
            );
        }
        if !self.closure_complete || self.unknown_closure_itemsets != 0 {
            return Err(
                "a completion receipt requires closure_complete=true and zero unknown closure itemsets"
                    .to_owned(),
            );
        }
        let expected = expected_population_counts_v2(self.closed_itemsets, self.cells_per_mask()?)?;
        if self.direction_members_expected != expected.directions {
            return Err(format!(
                "direction_members_expected {} is not the derived closed-mask × two-direction count {}",
                self.direction_members_expected, expected.directions
            ));
        }
        if self.direction_members_evaluated != expected.directions {
            return Err(format!(
                "evaluated direction members {} do not reconcile to derived expected {}",
                self.direction_members_evaluated, expected.directions
            ));
        }
        if self.long_exit_cells_evaluated != expected.long_cells
            || self.short_exit_cells_evaluated != expected.short_cells
            || self.exit_cells_expected != expected.rows
            || self.exit_cells_evaluated != expected.rows
            || self.row_count != expected.rows
        {
            return Err(format!(
                "derived/evaluated long, short and total exit rows disagree: expected {}/{}/{}, evaluated {}/{}/{}, row_count {}",
                expected.long_cells,
                expected.short_cells,
                expected.rows,
                self.long_exit_cells_evaluated,
                self.short_exit_cells_evaluated,
                self.exit_cells_evaluated,
                self.row_count
            ));
        }
        Ok(())
    }

    fn payload_bytes(self) -> Result<[u8; RECEIPT_PAYLOAD_BYTES], PopulationRefusal> {
        self.validate_semantics()?;
        let mut raw = [0_u8; RECEIPT_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.population_id)?;
        encoder.u64(self.row_count)?;
        encoder.bytes(&self.ordered_row_digest)?;
        for count in [
            self.sweep_trials,
            self.frequent_itemsets,
            self.infrequent_itemsets,
            self.closed_itemsets,
            self.redundant_itemsets,
            self.unknown_closure_itemsets,
            self.direction_members_expected,
            self.direction_members_evaluated,
            self.long_exit_cells_per_mask,
            self.short_exit_cells_per_mask,
            self.long_exit_cells_evaluated,
            self.short_exit_cells_evaluated,
            self.exit_cells_expected,
            self.exit_cells_evaluated,
            self.admitted_rows,
            self.rejected_rows,
            self.unmeasured_rows,
            self.refused_rows,
            self.topn_undefined_rows,
        ] {
            encoder.u64(count)?;
        }
        encoder.u32(self.extinction_depth)?;
        encoder.u32(self.rung_seconds)?;
        encoder.u8(self.instrument_family.byte())?;
        encoder.u8(u8::from(self.extinction_complete))?;
        encoder.u8(u8::from(self.closure_complete))?;
        encoder.zeros(5)?;
        self.identities.encode(&mut encoder)?;
        encoder.zeros(8)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; RECEIPT_STRIDE_BYTES], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let seal = seal(&payload);
        let mut raw = [0_u8; RECEIPT_STRIDE_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&payload)?;
        encoder.bytes(&seal)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; RECEIPT_STRIDE_BYTES]) -> Result<Self, PopulationRefusal> {
        let payload = raw
            .get(..RECEIPT_PAYLOAD_BYTES)
            .ok_or_else(|| "population V2 receipt payload is absent".to_owned())?;
        let stored_seal = raw
            .get(RECEIPT_PAYLOAD_BYTES..)
            .ok_or_else(|| "population V2 receipt seal is absent".to_owned())?;
        if stored_seal != seal(payload) {
            return Err("population V2 completion receipt failed its BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let receipt = Self {
            population_id: decoder.array_32()?,
            row_count: decoder.u64()?,
            ordered_row_digest: decoder.array_32()?,
            sweep_trials: decoder.u64()?,
            frequent_itemsets: decoder.u64()?,
            infrequent_itemsets: decoder.u64()?,
            closed_itemsets: decoder.u64()?,
            redundant_itemsets: decoder.u64()?,
            unknown_closure_itemsets: decoder.u64()?,
            direction_members_expected: decoder.u64()?,
            direction_members_evaluated: decoder.u64()?,
            long_exit_cells_per_mask: decoder.u64()?,
            short_exit_cells_per_mask: decoder.u64()?,
            long_exit_cells_evaluated: decoder.u64()?,
            short_exit_cells_evaluated: decoder.u64()?,
            exit_cells_expected: decoder.u64()?,
            exit_cells_evaluated: decoder.u64()?,
            admitted_rows: decoder.u64()?,
            rejected_rows: decoder.u64()?,
            unmeasured_rows: decoder.u64()?,
            refused_rows: decoder.u64()?,
            topn_undefined_rows: decoder.u64()?,
            extinction_depth: decoder.u32()?,
            rung_seconds: decoder.u32()?,
            instrument_family: InstrumentFamilyV1::from_byte(decoder.u8()?)?,
            extinction_complete: decode_bool(decoder.u8()?, "extinction_complete")?,
            closure_complete: decode_bool(decoder.u8()?, "closure_complete")?,
            identities: {
                decoder.zeros(5, "population V2 receipt flag reserve")?;
                PopulationIdentitiesV2::decode(&mut decoder)?
            },
        };
        decoder.zeros(8, "population V2 receipt trailing reserve")?;
        decoder.finish()?;
        receipt.validate_semantics()?;
        Ok(receipt)
    }
}

/// Authoritative V3 completion proof binding every V2 fact to the exact
/// inclusive month span requested from storage.
///
/// Version two remains embedded as typed facts rather than being reinterpreted
/// on disk.  Version three has its own path, header, stride, seal and content
/// digest, so a V2 record can never become authoritative merely because it is
/// read by newer code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionReceiptV3 {
    v2: CompletionReceiptV2,
    requested_span: RequestedSpanIdentityV1,
}

impl CompletionReceiptV3 {
    /// Builds an authoritative receipt from exact typed shape facts and rows.
    ///
    /// # Errors
    ///
    /// Every refusal from [`CompletionReceiptV2::for_rows`].  The requested
    /// span is already valid by construction and is sealed into the V3 record.
    pub fn for_rows(
        population_id: [u8; 32],
        instrument_family: InstrumentFamilyV1,
        rung_seconds: u32,
        rows: &[PopulationRowV1],
        requested_span: RequestedSpanIdentityV1,
        reconciliation: CompletionReconciliationV2,
        identities: PopulationIdentitiesV2,
    ) -> Result<Self, PopulationRefusal> {
        Self::from_v2(
            CompletionReceiptV2::for_rows(
                population_id,
                instrument_family,
                rung_seconds,
                rows,
                reconciliation,
                identities,
            )?,
            requested_span,
        )
    }

    /// Attaches an explicit requested span to already reconciled V2 facts.
    ///
    /// This creates a new V3 value only; it never changes or rewrites a V2
    /// record.  A ledger additionally byte-compares any existing same-ID V2
    /// audit receipt before accepting this value.
    ///
    /// # Errors
    ///
    /// Refuses malformed V2 facts.
    pub fn from_v2(
        v2: CompletionReceiptV2,
        requested_span: RequestedSpanIdentityV1,
    ) -> Result<Self, PopulationRefusal> {
        v2.validate_semantics()?;
        Ok(Self { v2, requested_span })
    }

    /// Every pre-span V2 completion fact carried by this receipt.
    #[must_use]
    pub const fn v2(self) -> CompletionReceiptV2 {
        self.v2
    }

    /// Exact inclusive requested month-span identity.
    #[must_use]
    pub const fn requested_span(self) -> RequestedSpanIdentityV1 {
        self.requested_span
    }

    /// Identity shared by this receipt and its row block.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.v2.population_id
    }

    /// Full domain-separated digest of every V2 fact and requested-span field.
    ///
    /// # Errors
    ///
    /// Refuses malformed embedded V2 semantics.
    pub fn content_digest(self) -> Result<[u8; 32], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(RECEIPT_V3_CONTENT_DOMAIN);
        hasher.update(&payload);
        Ok(hasher.finalize())
    }

    fn validate_semantics(self) -> Result<(), PopulationRefusal> {
        self.v2.validate_semantics()?;
        let canonical = RequestedSpanIdentityV1::new(
            self.requested_span.from_year(),
            self.requested_span.from_month(),
            self.requested_span.to_year(),
            self.requested_span.to_month(),
        )?;
        if canonical != self.requested_span {
            return Err("requested-span identity is not canonical".to_owned());
        }
        Ok(())
    }

    fn payload_bytes(self) -> Result<[u8; RECEIPT_V3_PAYLOAD_BYTES], PopulationRefusal> {
        self.validate_semantics()?;
        let v2_payload = self.v2.payload_bytes()?;
        let mut raw = [0_u8; RECEIPT_V3_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&v2_payload)?;
        self.requested_span.encode(&mut encoder)?;
        encoder.bytes(&self.requested_span.digest())?;
        encoder.zeros(12)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; RECEIPT_V3_STRIDE_BYTES], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let seal = seal(&payload);
        let mut raw = [0_u8; RECEIPT_V3_STRIDE_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&payload)?;
        encoder.bytes(&seal)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; RECEIPT_V3_STRIDE_BYTES]) -> Result<Self, PopulationRefusal> {
        let payload = raw
            .get(..RECEIPT_V3_PAYLOAD_BYTES)
            .ok_or_else(|| "population V3 receipt payload is absent".to_owned())?;
        let stored_seal = raw
            .get(RECEIPT_V3_PAYLOAD_BYTES..)
            .ok_or_else(|| "population V3 receipt seal is absent".to_owned())?;
        if stored_seal != seal(payload) {
            return Err("population V3 completion receipt failed its BLAKE3 seal".to_owned());
        }

        let mut decoder = Decoder::new(payload);
        let v2_payload = decoder.bytes::<RECEIPT_PAYLOAD_BYTES>()?;
        let mut v2_raw = [0_u8; RECEIPT_STRIDE_BYTES];
        v2_raw
            .get_mut(..RECEIPT_PAYLOAD_BYTES)
            .ok_or_else(|| "fixed V2 receipt payload slot is absent".to_owned())?
            .copy_from_slice(&v2_payload);
        v2_raw
            .get_mut(RECEIPT_PAYLOAD_BYTES..)
            .ok_or_else(|| "fixed V2 receipt seal slot is absent".to_owned())?
            .copy_from_slice(&seal(&v2_payload));
        let v2 = CompletionReceiptV2::from_bytes(&v2_raw)?;
        let requested_span = RequestedSpanIdentityV1::decode(&mut decoder)?;
        let stored_span_digest = decoder.array_32()?;
        if stored_span_digest != requested_span.digest() {
            return Err(
                "population V3 requested-span fields disagree with their canonical digest"
                    .to_owned(),
            );
        }
        decoder.zeros(12, "population V3 receipt trailing reserve")?;
        decoder.finish()?;
        let receipt = Self { v2, requested_span };
        receipt.validate_semantics()?;
        Ok(receipt)
    }
}

/// Exact complete-calendar evidence attached to one population completion.
///
/// Signal and one-minute execution coverage are deliberately separate. A
/// complete coarse signal stream cannot authorize missing execution minutes,
/// and a complete execution stream cannot authorize a hole in the rung that
/// generated the signal. The day bounds name the full requested civil-month
/// span, never merely the first and last observed bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationCalendarCoverageV1 {
    version: u32,
    signal_rung_seconds: u32,
    execution_rung_seconds: u32,
    reserved: u32,
    first_day: i64,
    last_day: i64,
    signal_complete_receipt_digest: [u8; 32],
    execution_complete_receipt_digest: [u8; 32],
}

impl PopulationCalendarCoverageV1 {
    fn from_complete(
        v3: &CompletionReceiptV3,
        signal: CompleteCalendarReceiptV2,
        execution: CompleteCalendarReceiptV2,
    ) -> Result<Self, PopulationRefusal> {
        let coverage = Self {
            version: POPULATION_CALENDAR_COVERAGE_VERSION,
            signal_rung_seconds: signal.rung_seconds(),
            execution_rung_seconds: execution.rung_seconds(),
            reserved: 0,
            first_day: signal.first_day(),
            last_day: signal.last_day(),
            signal_complete_receipt_digest: signal.digest(),
            execution_complete_receipt_digest: execution.digest(),
        };
        if signal.first_day() != execution.first_day() || signal.last_day() != execution.last_day()
        {
            return Err(format!(
                "complete signal calendar covers IST days {}..={}, but complete one-minute execution calendar covers {}..={}",
                signal.first_day(),
                signal.last_day(),
                execution.first_day(),
                execution.last_day()
            ));
        }
        coverage.validate_against_v3(v3)?;
        Ok(coverage)
    }

    /// Coverage schema version, currently one.
    #[must_use]
    pub const fn version(self) -> u32 {
        self.version
    }

    /// Signal population rung proved complete by its typed receipt.
    #[must_use]
    pub const fn signal_rung_seconds(self) -> u32 {
        self.signal_rung_seconds
    }

    /// Execution rung proved complete, always exactly one minute.
    #[must_use]
    pub const fn execution_rung_seconds(self) -> u32 {
        self.execution_rung_seconds
    }

    /// First requested IST civil day, inclusive.
    #[must_use]
    pub const fn first_day(self) -> i64 {
        self.first_day
    }

    /// Last requested IST civil day, inclusive.
    #[must_use]
    pub const fn last_day(self) -> i64 {
        self.last_day
    }

    /// Digest of the complete signal-rung calendar receipt.
    #[must_use]
    pub const fn signal_complete_receipt_digest(self) -> [u8; 32] {
        self.signal_complete_receipt_digest
    }

    /// Digest of the complete one-minute execution calendar receipt.
    #[must_use]
    pub const fn execution_complete_receipt_digest(self) -> [u8; 32] {
        self.execution_complete_receipt_digest
    }

    fn validate_against_v3(self, v3: &CompletionReceiptV3) -> Result<(), PopulationRefusal> {
        if self.version != POPULATION_CALENDAR_COVERAGE_VERSION {
            return Err(format!(
                "population calendar coverage version {} is unknown; expected {}",
                self.version, POPULATION_CALENDAR_COVERAGE_VERSION
            ));
        }
        if self.reserved != 0 {
            return Err(format!(
                "population calendar coverage reserve is {}, not zero",
                self.reserved
            ));
        }
        if self.signal_rung_seconds != v3.v2().rung_seconds {
            return Err(format!(
                "complete signal-calendar rung {} does not equal population rung {}",
                self.signal_rung_seconds,
                v3.v2().rung_seconds
            ));
        }
        if self.execution_rung_seconds != 60 {
            return Err(format!(
                "complete execution calendar is {} seconds; population execution authority requires exactly 60",
                self.execution_rung_seconds
            ));
        }
        let (expected_first, expected_last) = requested_span_day_bounds(v3.requested_span())?;
        if (self.first_day, self.last_day) != (expected_first, expected_last) {
            return Err(format!(
                "complete population calendar covers IST days {}..={}, not the full requested civil-month span {expected_first}..={expected_last}",
                self.first_day, self.last_day
            ));
        }
        require_nonzero_digest(
            "signal_complete_receipt_digest",
            &self.signal_complete_receipt_digest,
        )?;
        require_nonzero_digest(
            "execution_complete_receipt_digest",
            &self.execution_complete_receipt_digest,
        )?;
        Ok(())
    }

    fn encode(self, encoder: &mut Encoder<'_>) -> Result<(), PopulationRefusal> {
        encoder.u32(self.version)?;
        encoder.u32(self.signal_rung_seconds)?;
        encoder.u32(self.execution_rung_seconds)?;
        encoder.u32(self.reserved)?;
        encoder.i64(self.first_day)?;
        encoder.i64(self.last_day)?;
        encoder.bytes(&self.signal_complete_receipt_digest)?;
        encoder.bytes(&self.execution_complete_receipt_digest)
    }

    fn decode(decoder: &mut Decoder<'_>) -> Result<Self, PopulationRefusal> {
        Ok(Self {
            version: decoder.u32()?,
            signal_rung_seconds: decoder.u32()?,
            execution_rung_seconds: decoder.u32()?,
            reserved: decoder.u32()?,
            first_day: decoder.i64()?,
            last_day: decoder.i64()?,
            signal_complete_receipt_digest: decoder.array_32()?,
            execution_complete_receipt_digest: decoder.array_32()?,
        })
    }
}

fn requested_span_day_bounds(
    span: RequestedSpanIdentityV1,
) -> Result<(i64, i64), PopulationRefusal> {
    let first = pull::session::Day::new(span.from_year(), span.from_month(), 1).map_err(|why| {
        format!(
            "requested-span first civil month {}-{:02} cannot be named: {why}",
            span.from_year(),
            span.from_month()
        )
    })?;
    let last = pull::session::Day::new(span.to_year(), span.to_month(), 1)
        .map_err(|why| {
            format!(
                "requested-span last civil month {}-{:02} cannot be named: {why}",
                span.to_year(),
                span.to_month()
            )
        })?
        .end_of_month();
    Ok((
        i64::from(first.days_from_epoch()),
        i64::from(last.days_from_epoch()),
    ))
}

/// Authoritative V4 population completion binding V3 facts to measured
/// signal-rung and exact one-minute execution calendar coverage.
///
/// V3 remains embedded byte-for-byte as its unchanged 760-byte payload. V4 is
/// a distinct append-only file/codec and never rewrites or reinterprets a V3
/// record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletionReceiptV4 {
    v3: CompletionReceiptV3,
    coverage: PopulationCalendarCoverageV1,
}

impl CompletionReceiptV4 {
    /// Builds V4 authority from rows plus two typed complete-calendar proofs.
    ///
    /// # Errors
    ///
    /// Every V3 row/reconciliation refusal, unequal calendar bounds, a signal
    /// rung different from the population rung, execution other than 60
    /// seconds, bounds different from the full requested civil months, or a
    /// zero calendar digest.
    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is an independently bound population authority term"
    )]
    pub fn for_rows(
        population_id: [u8; 32],
        instrument_family: InstrumentFamilyV1,
        rung_seconds: u32,
        rows: &[PopulationRowV1],
        requested_span: RequestedSpanIdentityV1,
        reconciliation: CompletionReconciliationV2,
        identities: PopulationIdentitiesV2,
        signal_calendar: CompleteCalendarReceiptV2,
        execution_calendar: CompleteCalendarReceiptV2,
    ) -> Result<Self, PopulationRefusal> {
        let v3 = CompletionReceiptV3::for_rows(
            population_id,
            instrument_family,
            rung_seconds,
            rows,
            requested_span,
            reconciliation,
            identities,
        )?;
        Self::from_complete_v3(&v3, signal_calendar, execution_calendar)
    }

    fn from_complete_v3(
        v3: &CompletionReceiptV3,
        signal_calendar: CompleteCalendarReceiptV2,
        execution_calendar: CompleteCalendarReceiptV2,
    ) -> Result<Self, PopulationRefusal> {
        v3.validate_semantics()?;
        let coverage =
            PopulationCalendarCoverageV1::from_complete(v3, signal_calendar, execution_calendar)?;
        Ok(Self { v3: *v3, coverage })
    }

    /// Every unchanged V3 fact embedded in this receipt.
    #[must_use]
    pub const fn v3(self) -> CompletionReceiptV3 {
        self.v3
    }

    /// Exact signal/execution calendar coverage authority.
    #[must_use]
    pub const fn coverage(self) -> PopulationCalendarCoverageV1 {
        self.coverage
    }

    /// Identity shared by this receipt and its contiguous row block.
    #[must_use]
    pub const fn population_id(self) -> [u8; 32] {
        self.v3.population_id()
    }

    /// Domain-separated digest of the exact V4 payload.
    ///
    /// # Errors
    ///
    /// Refuses malformed embedded V3 or calendar-coverage semantics.
    pub fn content_digest(self) -> Result<[u8; 32], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(RECEIPT_V4_CONTENT_DOMAIN);
        hasher.update(&payload);
        Ok(hasher.finalize())
    }

    fn validate_semantics(self) -> Result<(), PopulationRefusal> {
        self.v3.validate_semantics()?;
        self.coverage.validate_against_v3(&self.v3)
    }

    fn payload_bytes(self) -> Result<[u8; RECEIPT_V4_PAYLOAD_BYTES], PopulationRefusal> {
        self.validate_semantics()?;
        let mut raw = [0_u8; RECEIPT_V4_PAYLOAD_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&self.v3.payload_bytes()?)?;
        self.coverage.encode(&mut encoder)?;
        encoder.finish()?;
        Ok(raw)
    }

    fn to_bytes(self) -> Result<[u8; RECEIPT_V4_STRIDE_BYTES], PopulationRefusal> {
        let payload = self.payload_bytes()?;
        let mut raw = [0_u8; RECEIPT_V4_STRIDE_BYTES];
        let mut encoder = Encoder::new(&mut raw);
        encoder.bytes(&payload)?;
        encoder.bytes(&seal(&payload))?;
        encoder.finish()?;
        Ok(raw)
    }

    fn from_bytes(raw: &[u8; RECEIPT_V4_STRIDE_BYTES]) -> Result<Self, PopulationRefusal> {
        let payload = raw
            .get(..RECEIPT_V4_PAYLOAD_BYTES)
            .ok_or_else(|| "population V4 receipt payload is absent".to_owned())?;
        let stored_seal = raw
            .get(RECEIPT_V4_PAYLOAD_BYTES..)
            .ok_or_else(|| "population V4 receipt seal is absent".to_owned())?;
        if stored_seal != seal(payload) {
            return Err("population V4 completion receipt failed its BLAKE3 seal".to_owned());
        }
        let mut decoder = Decoder::new(payload);
        let v3_payload = decoder.bytes::<RECEIPT_V3_PAYLOAD_BYTES>()?;
        let mut v3_raw = [0_u8; RECEIPT_V3_STRIDE_BYTES];
        v3_raw
            .get_mut(..RECEIPT_V3_PAYLOAD_BYTES)
            .ok_or_else(|| "fixed V3 receipt payload slot is absent".to_owned())?
            .copy_from_slice(&v3_payload);
        v3_raw
            .get_mut(RECEIPT_V3_PAYLOAD_BYTES..)
            .ok_or_else(|| "fixed V3 receipt seal slot is absent".to_owned())?
            .copy_from_slice(&seal(&v3_payload));
        let v3 = CompletionReceiptV3::from_bytes(&v3_raw)?;
        let coverage = PopulationCalendarCoverageV1::decode(&mut decoder)?;
        decoder.finish()?;
        let receipt = Self { v3, coverage };
        receipt.validate_semantics()?;
        Ok(receipt)
    }
}

/// Where a committed population's contiguous rows sit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationBlock {
    /// Zero-based index of the first row.
    pub first: u64,
    /// Number of rows, including zero for a legitimate empty population.
    pub count: u64,
}

/// One bounded page from a committed population.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PopulationPageV1 {
    /// Exact total committed rows for this population.
    pub total: u64,
    /// Requested zero-based offset.
    pub offset: u64,
    /// Contiguous canonical rows, in sequence order.
    pub rows: Vec<PopulationRowV1>,
}

/// Whether a complete population was newly committed or exactly reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopulationCommit {
    /// Rows and completion receipt were durably appended by this call.
    Written,
    /// An existing block and receipt matched byte for byte.
    Reused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BlockFacts {
    block: PopulationBlock,
    digest: [u8; 32],
    masks: u64,
    direction_members: u64,
    long_rows: u64,
    short_rows: u64,
    long_cells_min: u64,
    long_cells_max: u64,
    short_cells_min: u64,
    short_cells_max: u64,
    admitted: u64,
    rejected: u64,
    unmeasured: u64,
    refused: u64,
    topn_undefined: u64,
    instrument_family: Option<InstrumentFamilyV1>,
    rung_seconds: Option<u32>,
}

#[derive(Debug)]
struct CommittedPopulation {
    block: PopulationBlock,
    receipt: CompletionReceiptV2,
}

#[derive(Debug)]
struct AuthoritativePopulationV3 {
    block: PopulationBlock,
    receipt: CompletionReceiptV3,
}

#[derive(Debug)]
struct AuthoritativePopulationV4 {
    block: PopulationBlock,
    receipt: CompletionReceiptV4,
}

/// Constant-size evidence for the exact file generation already indexed.
///
/// Unix and Windows expose both a stable file identity and a change clock.
/// Other targets may read and validate a population, but append fails closed:
/// portable metadata cannot distinguish a same-length path replacement. This
/// detects ordinary filesystem mutation, not an actor able to forge metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileGeneration {
    len: u64,
    platform: PlatformGeneration,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    device: u64,
    inode: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(windows)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration {
    volume_serial: u32,
    file_index: u64,
    creation_time: u64,
    last_write_time: u64,
}

#[cfg(not(any(unix, windows)))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlatformGeneration;

struct PopulationFiles {
    row: File,
    receipt_v2: File,
    receipt_v3: Option<File>,
    receipt_v4: Option<File>,
    writer_lock: File,
}

struct PopulationPaths {
    row: PathBuf,
    receipt_v2: PathBuf,
    receipt_v3: PathBuf,
    receipt_v4: PathBuf,
}

/// Open population row/receipt files and their in-memory exact indexes.
#[derive(Debug)]
pub struct PopulationLedger {
    row_file: File,
    receipt_file: File,
    receipt_v3_file: Option<File>,
    receipt_v4_file: Option<File>,
    writer_lock: File,
    row_path: PathBuf,
    receipt_path: PathBuf,
    receipt_v3_path: PathBuf,
    receipt_v4_path: PathBuf,
    raw_blocks: HashMap<[u8; 32], BlockFacts>,
    receipts: HashMap<[u8; 32], CompletionReceiptV2>,
    committed: HashMap<[u8; 32], CommittedPopulation>,
    receipts_v3: HashMap<[u8; 32], CompletionReceiptV3>,
    committed_v3: HashMap<[u8; 32], AuthoritativePopulationV3>,
    receipts_v4: HashMap<[u8; 32], CompletionReceiptV4>,
    committed_v4: HashMap<[u8; 32], AuthoritativePopulationV4>,
    row_scanned: u64,
    receipt_scanned: u64,
    receipt_v3_scanned: u64,
    receipt_v4_scanned: u64,
    row_generation: FileGeneration,
    receipt_generation: FileGeneration,
    receipt_v3_generation: Option<FileGeneration>,
    receipt_v4_generation: Option<FileGeneration>,
    writable: bool,
}

impl PopulationLedger {
    /// Fixed-stride population-row file beside the other result files.
    #[must_use]
    pub fn row_path(root: &Path) -> PathBuf {
        root.join("results").join("population-v1.bin")
    }

    /// Legacy V2 completion path retained as an explicit audit source.
    #[must_use]
    pub fn receipt_path(root: &Path) -> PathBuf {
        root.join("results").join("population-completions-v2.bin")
    }

    /// Authoritative V3 completion path binding the requested inclusive span.
    #[must_use]
    pub fn receipt_v3_path(root: &Path) -> PathBuf {
        root.join("results").join("population-completions-v3.bin")
    }

    /// Authoritative V4 completion path binding measured signal and execution
    /// calendar coverage without reinterpreting any legacy receipt bytes.
    #[must_use]
    pub fn receipt_v4_path(root: &Path) -> PathBuf {
        root.join("results").join("population-completions-v4.bin")
    }

    /// Legacy V1 completion path, retained for explicit discovery only.
    ///
    /// This ledger never reads it as an authoritative V2 commit and never
    /// rewrites or deletes it.
    #[must_use]
    pub fn legacy_receipt_path_v1(root: &Path) -> PathBuf {
        root.join("results").join("population-completions-v1.bin")
    }

    fn lock_path(root: &Path) -> PathBuf {
        root.join("results").join("population-write.lock")
    }

    /// Opens existing population files without creating any path.
    ///
    /// Opening holds the shared two-file lock while both complete indexes are
    /// built.  The scan is O(total rows + receipts), never hidden as O(1).
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    ///
    /// # Errors
    ///
    /// Refuses absent files, a bad header/version/reserve, ragged length, torn
    /// seal, invalid schema, duplicate/interleaved identity, bad sequence, or a
    /// receipt that does not exactly reconcile to its row block.
    pub fn open_read(root: &Path) -> Result<Self, PopulationRefusal> {
        Self::open_existing(root, None)
    }

    /// Opens existing population files only when each fits `max_bytes`.
    ///
    /// The length gate runs before either index scan, giving an HTTP caller a
    /// hard ceiling on the otherwise O(total rows + receipts) open cost.
    ///
    /// # Errors
    ///
    /// Every refusal from [`Self::open_read`], plus a file over `max_bytes`.
    pub fn open_read_bounded(root: &Path, max_bytes: u64) -> Result<Self, PopulationRefusal> {
        Self::open_existing(root, Some(max_bytes))
    }

    fn open_existing(root: &Path, max_bytes: Option<u64>) -> Result<Self, PopulationRefusal> {
        let row_path = Self::row_path(root);
        let receipt_path = Self::receipt_path(root);
        let receipt_v3_path = Self::receipt_v3_path(root);
        let receipt_v4_path = Self::receipt_v4_path(root);
        let lock_path = Self::lock_path(root);
        let writer_lock = File::open(&lock_path)
            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
        writer_lock
            .lock_shared()
            .map_err(|why| format!("{} could not be shared-locked: {why}", lock_path.display()))?;
        let opened = (|| {
            let row_file = File::open(&row_path)
                .map_err(|why| format!("{} could not be opened: {why}", row_path.display()))?;
            let receipt_file = File::open(&receipt_path)
                .map_err(|why| format!("{} could not be opened: {why}", receipt_path.display()))?;
            let receipt_v3_file = match File::open(&receipt_v3_path) {
                Ok(file) => Some(file),
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
                Err(why) => {
                    return Err(format!(
                        "{} could not be opened: {why}",
                        receipt_v3_path.display()
                    ));
                }
            };
            let receipt_v4_file = match File::open(&receipt_v4_path) {
                Ok(file) => Some(file),
                Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
                Err(why) => {
                    return Err(format!(
                        "{} could not be opened: {why}",
                        receipt_v4_path.display()
                    ));
                }
            };
            Self::from_files(
                PopulationFiles {
                    row: row_file,
                    receipt_v2: receipt_file,
                    receipt_v3: receipt_v3_file,
                    receipt_v4: receipt_v4_file,
                    writer_lock: writer_lock.try_clone().map_err(|why| {
                        format!(
                            "{} lock handle could not be cloned: {why}",
                            lock_path.display()
                        )
                    })?,
                },
                PopulationPaths {
                    row: row_path,
                    receipt_v2: receipt_path,
                    receipt_v3: receipt_v3_path,
                    receipt_v4: receipt_v4_path,
                },
                max_bytes,
                false,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", lock_path.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Opens for append, creating fresh durable headers when absent.
    ///
    /// # Errors
    ///
    /// Refuses directory/header creation, locking, schema, integrity or
    /// reconciliation failures.  Existing bytes are never replaced.
    pub fn open(root: &Path) -> Result<Self, PopulationRefusal> {
        let dir = root.join("results");
        std::fs::create_dir_all(&dir)
            .map_err(|why| format!("the results directory could not be made: {why}"))?;
        let lock_path = Self::lock_path(root);
        let writer_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|why| format!("{} could not be opened: {why}", lock_path.display()))?;
        writer_lock
            .lock()
            .map_err(|why| format!("{} could not be locked: {why}", lock_path.display()))?;
        let row_path = Self::row_path(root);
        let receipt_path = Self::receipt_path(root);
        let receipt_v3_path = Self::receipt_v3_path(root);
        let receipt_v4_path = Self::receipt_v4_path(root);
        let opened = (|| {
            let mut row_file = open_or_create(&row_path)?;
            let mut receipt_file = open_or_create(&receipt_path)?;
            let mut receipt_v3_file = open_or_create(&receipt_v3_path)?;
            let mut receipt_v4_file = open_or_create(&receipt_v4_path)?;
            ensure_header(&mut row_file, &row_path, ROW_MAGIC, ROW_VERSION)?;
            ensure_header(
                &mut receipt_file,
                &receipt_path,
                RECEIPT_V2_MAGIC,
                RECEIPT_V2_VERSION,
            )?;
            ensure_header(
                &mut receipt_v3_file,
                &receipt_v3_path,
                RECEIPT_V3_MAGIC,
                RECEIPT_V3_VERSION,
            )?;
            ensure_header(
                &mut receipt_v4_file,
                &receipt_v4_path,
                RECEIPT_V4_MAGIC,
                RECEIPT_V4_VERSION,
            )?;
            sync_directory(&dir)?;
            Self::from_files(
                PopulationFiles {
                    row: row_file,
                    receipt_v2: receipt_file,
                    receipt_v3: Some(receipt_v3_file),
                    receipt_v4: Some(receipt_v4_file),
                    writer_lock: writer_lock.try_clone().map_err(|why| {
                        format!(
                            "{} lock handle could not be cloned: {why}",
                            lock_path.display()
                        )
                    })?,
                },
                PopulationPaths {
                    row: row_path,
                    receipt_v2: receipt_path,
                    receipt_v3: receipt_v3_path,
                    receipt_v4: receipt_v4_path,
                },
                None,
                true,
            )
        })();
        let released = writer_lock
            .unlock()
            .map_err(|why| format!("{} could not be unlocked: {why}", lock_path.display()));
        match (opened, released) {
            (Ok(ledger), Ok(())) => Ok(ledger),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one ordered open-time transaction validates every file before constructing any reusable ledger state"
    )]
    fn from_files(
        files: PopulationFiles,
        paths: PopulationPaths,
        max_bytes: Option<u64>,
        writable: bool,
    ) -> Result<Self, PopulationRefusal> {
        let PopulationFiles {
            row: mut row_file,
            receipt_v2: mut receipt_file,
            receipt_v3: mut receipt_v3_file,
            receipt_v4: mut receipt_v4_file,
            writer_lock,
        } = files;
        let PopulationPaths {
            row: row_path,
            receipt_v2: receipt_path,
            receipt_v3: receipt_v3_path,
            receipt_v4: receipt_v4_path,
        } = paths;
        let row_len = measured_len(&row_file, &row_path)?;
        let receipt_len = measured_len(&receipt_file, &receipt_path)?;
        let receipt_v3_len = if let Some(file) = receipt_v3_file.as_ref() {
            measured_len(file, &receipt_v3_path)?
        } else {
            0
        };
        let receipt_v4_len = if let Some(file) = receipt_v4_file.as_ref() {
            measured_len(file, &receipt_v4_path)?
        } else {
            0
        };
        if let Some(max_bytes) = max_bytes {
            for (path, len) in [
                (&row_path, row_len),
                (&receipt_path, receipt_len),
                (&receipt_v3_path, receipt_v3_len),
                (&receipt_v4_path, receipt_v4_len),
            ] {
                if len > max_bytes {
                    return Err(format!(
                        "{} is {len} bytes; this bounded population reader accepts at most {max_bytes}. No partial index was built",
                        path.display()
                    ));
                }
            }
        }
        check_header(
            &mut row_file,
            &row_path,
            row_len,
            ROW_MAGIC,
            ROW_VERSION,
            ROW_STRIDE,
        )?;
        check_header(
            &mut receipt_file,
            &receipt_path,
            receipt_len,
            RECEIPT_V2_MAGIC,
            RECEIPT_V2_VERSION,
            RECEIPT_STRIDE,
        )?;
        if let Some(file) = &mut receipt_v3_file {
            check_header(
                file,
                &receipt_v3_path,
                receipt_v3_len,
                RECEIPT_V3_MAGIC,
                RECEIPT_V3_VERSION,
                RECEIPT_V3_STRIDE,
            )?;
        }
        if let Some(file) = &mut receipt_v4_file {
            check_header(
                file,
                &receipt_v4_path,
                receipt_v4_len,
                RECEIPT_V4_MAGIC,
                RECEIPT_V4_VERSION,
                RECEIPT_V4_STRIDE,
            )?;
        }
        let raw_blocks = index_rows(&mut row_file, &row_path, row_len)?;
        let receipts = index_receipts(&mut receipt_file, &receipt_path, receipt_len)?;
        let committed = reconcile_receipts(&raw_blocks, &receipts)?;
        let receipts_v3 = if let Some(file) = receipt_v3_file.as_mut() {
            index_receipts_v3(file, &receipt_v3_path, receipt_v3_len)?
        } else {
            HashMap::new()
        };
        let committed_v3 = reconcile_receipts_v3(&raw_blocks, &receipts, &receipts_v3)?;
        let receipts_v4 = if let Some(file) = receipt_v4_file.as_mut() {
            index_receipts_v4(file, &receipt_v4_path, receipt_v4_len)?
        } else {
            HashMap::new()
        };
        let committed_v4 =
            reconcile_receipts_v4(&raw_blocks, &receipts, &receipts_v3, &receipts_v4)?;
        let row_generation =
            validated_generation(&row_file, &row_path, row_len, "population row file")?;
        let receipt_generation = validated_generation(
            &receipt_file,
            &receipt_path,
            receipt_len,
            "population V2 receipt file",
        )?;
        let receipt_v3_generation = receipt_v3_file
            .as_ref()
            .map(|file| {
                validated_generation(
                    file,
                    &receipt_v3_path,
                    receipt_v3_len,
                    "population V3 receipt file",
                )
            })
            .transpose()?;
        let receipt_v4_generation = receipt_v4_file
            .as_ref()
            .map(|file| {
                validated_generation(
                    file,
                    &receipt_v4_path,
                    receipt_v4_len,
                    "population V4 receipt file",
                )
            })
            .transpose()?;
        Ok(Self {
            row_file,
            receipt_file,
            receipt_v3_file,
            receipt_v4_file,
            writer_lock,
            row_path,
            receipt_path,
            receipt_v3_path,
            receipt_v4_path,
            raw_blocks,
            receipts,
            committed,
            receipts_v3,
            committed_v3,
            receipts_v4,
            committed_v4,
            row_scanned: row_len,
            receipt_scanned: receipt_len,
            receipt_v3_scanned: receipt_v3_len,
            receipt_v4_scanned: receipt_v4_len,
            row_generation,
            receipt_generation,
            receipt_v3_generation,
            receipt_v4_generation,
            writable,
        })
    }

    /// One in-memory probe for a legacy V2 audit receipt.
    ///
    /// V2 did not bind the requested month span and is not authoritative for
    /// new selection. Use [`Self::receipt_v3`] for that decision boundary.
    #[must_use]
    pub fn receipt(&self, population_id: &[u8; 32]) -> Option<CompletionReceiptV2> {
        self.committed.get(population_id).map(|entry| entry.receipt)
    }

    /// One in-memory probe for a span-bound V3 audit receipt.
    #[must_use]
    pub fn receipt_v3(&self, population_id: &[u8; 32]) -> Option<CompletionReceiptV3> {
        self.committed_v3
            .get(population_id)
            .map(|entry| entry.receipt)
    }

    /// One in-memory probe for authoritative calendar-bound V4 completion.
    #[must_use]
    pub fn receipt_v4(&self, population_id: &[u8; 32]) -> Option<CompletionReceiptV4> {
        self.committed_v4
            .get(population_id)
            .map(|entry| entry.receipt)
    }

    /// One in-memory probe for a committed contiguous row block.
    #[must_use]
    pub fn block(&self, population_id: &[u8; 32]) -> Option<PopulationBlock> {
        self.committed.get(population_id).map(|entry| entry.block)
    }

    /// One in-memory probe for a V3-audited contiguous row block.
    #[must_use]
    pub fn block_v3(&self, population_id: &[u8; 32]) -> Option<PopulationBlock> {
        self.committed_v3
            .get(population_id)
            .map(|entry| entry.block)
    }

    /// One in-memory probe for a V4-authorized contiguous row block.
    #[must_use]
    pub fn block_v4(&self, population_id: &[u8; 32]) -> Option<PopulationBlock> {
        self.committed_v4
            .get(population_id)
            .map(|entry| entry.block)
    }

    /// Number of committed populations indexed by this open handle.
    #[must_use]
    pub fn populations(&self) -> usize {
        self.committed.len()
    }

    /// Number of span-bound V3 audit populations in this handle.
    ///
    /// The legacy method name is retained byte/API-compatibly. New authority
    /// checks must use [`Self::authoritative_v4_populations`].
    #[must_use]
    pub fn authoritative_populations(&self) -> usize {
        self.committed_v3.len()
    }

    /// Number of authoritative calendar-bound V4 populations in this handle.
    #[must_use]
    pub fn authoritative_v4_populations(&self) -> usize {
        self.committed_v4.len()
    }

    /// Appends one complete row block and then its receipt, or exactly reuses it.
    ///
    /// The receipt is the only commit marker and is synced after the row block.
    /// A same-id rerun with different rows or receipt bytes is nondeterminism and
    /// is refused.  A prepared orphan block is reused only after row-for-row
    /// equality, then receives the missing receipt.
    ///
    /// # Errors
    ///
    /// Refuses invalid rows/receipt, lock or I/O failures, stale-handle damage,
    /// duplicates, mismatch, or inability to durably sync either file.
    pub fn append_complete(
        &mut self,
        rows: &[PopulationRowV1],
        receipt: &CompletionReceiptV2,
    ) -> Result<PopulationCommit, PopulationRefusal> {
        if !self.writable {
            return Err(
                "a read-only population handle cannot prepare or commit rows; reopen with PopulationLedger::open"
                    .to_owned(),
            );
        }
        let expected = facts_of_rows(receipt.population_id, rows)?;
        receipt.validate_semantics()?;
        validate_receipt_v2_against_facts(receipt, Some(&expected))?;
        self.writer_lock
            .lock()
            .map_err(|why| format!("the population writer lock could not be acquired: {why}"))?;
        let attempted = self.append_complete_locked(rows, receipt, expected);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("the population writer lock could not be released: {why}"));
        match (attempted, released) {
            (Ok(state), Ok(())) => Ok(state),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Appends one span-bound V3 audit receipt after its exact rows.
    ///
    /// Existing V2 bytes remain audit records. If a same-ID V2 receipt exists,
    /// every embedded V2 fact must be byte-identical before V3 can commit.
    ///
    /// # Errors
    ///
    /// Refuses every row/V2 validation failure, a different same-ID span or
    /// receipt, stale-handle damage, locking/I/O failure, or a read-only handle.
    pub fn append_complete_v3(
        &mut self,
        rows: &[PopulationRowV1],
        receipt: &CompletionReceiptV3,
    ) -> Result<PopulationCommit, PopulationRefusal> {
        if !self.writable {
            return Err(
                "a read-only population handle cannot prepare or commit rows; reopen with PopulationLedger::open"
                    .to_owned(),
            );
        }
        let v2 = receipt.v2();
        let expected = facts_of_rows(v2.population_id, rows)?;
        receipt.validate_semantics()?;
        validate_receipt_v2_against_facts(&v2, Some(&expected))?;
        self.writer_lock
            .lock()
            .map_err(|why| format!("the population writer lock could not be acquired: {why}"))?;
        let attempted = self.append_complete_v3_locked(rows, receipt, expected);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("the population writer lock could not be released: {why}"));
        match (attempted, released) {
            (Ok(state), Ok(())) => Ok(state),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Appends one authoritative calendar-bound V4 completion.
    ///
    /// The durable order is rows, V2 audit, V3 audit, then V4 authority. A
    /// crash can therefore leave only a verified legacy prefix; it can never
    /// make unmeasured calendar coverage authoritative. Exact reruns reuse all
    /// bytes, while different coverage under the same population ID refuses.
    ///
    /// # Errors
    ///
    /// Every row, V2, V3, calendar-coverage, stale-generation, locking or I/O
    /// refusal, including any same-ID mismatch at any earlier receipt version.
    pub fn append_complete_v4(
        &mut self,
        rows: &[PopulationRowV1],
        receipt: &CompletionReceiptV4,
    ) -> Result<PopulationCommit, PopulationRefusal> {
        if !self.writable {
            return Err(
                "a read-only population handle cannot prepare or commit rows; reopen with PopulationLedger::open"
                    .to_owned(),
            );
        }
        let v2 = receipt.v3().v2();
        let expected = facts_of_rows(v2.population_id, rows)?;
        receipt.validate_semantics()?;
        validate_receipt_v2_against_facts(&v2, Some(&expected))?;
        self.writer_lock
            .lock()
            .map_err(|why| format!("the population writer lock could not be acquired: {why}"))?;
        let attempted = self.append_complete_v4_locked(rows, receipt, expected);
        let released = self
            .writer_lock
            .unlock()
            .map_err(|why| format!("the population writer lock could not be released: {why}"));
        match (attempted, released) {
            (Ok(state), Ok(())) => Ok(state),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    fn append_complete_locked(
        &mut self,
        rows: &[PopulationRowV1],
        receipt: &CompletionReceiptV2,
        expected: BlockFacts,
    ) -> Result<PopulationCommit, PopulationRefusal> {
        self.absorb_rows()?;
        self.absorb_receipts()?;
        self.absorb_receipts_v3()?;
        self.absorb_receipts_v4()?;
        if let Some(authoritative) = self.receipts_v3.get(&receipt.population_id)
            && authoritative.v2() != *receipt
        {
            return Err(format!(
                "population {} already has authoritative V3 facts different from the offered V2 audit receipt; no byte was replaced",
                hex(&receipt.population_id)
            ));
        }
        if let Some(authoritative) = self.receipts_v4.get(&receipt.population_id)
            && authoritative.v3().v2() != *receipt
        {
            return Err(format!(
                "population {} already has authoritative V4 facts different from the offered V2 audit receipt; no byte was replaced",
                hex(&receipt.population_id)
            ));
        }
        if let Some(existing) = self.receipts.get(&receipt.population_id).copied() {
            if existing != *receipt {
                return Err(format!(
                    "population {} already has a different completion receipt; no byte was replaced",
                    hex(&receipt.population_id)
                ));
            }
            self.require_exact_existing(rows, receipt, expected)?;
            self.row_file.sync_all().map_err(|why| {
                format!("the existing population rows could not be synced: {why}")
            })?;
            self.receipt_file.sync_all().map_err(|why| {
                format!("the existing population receipt could not be synced: {why}")
            })?;
            return Ok(PopulationCommit::Reused);
        }

        if self.raw_blocks.contains_key(&receipt.population_id) {
            self.require_exact_existing(rows, receipt, expected)?;
            self.row_file.sync_all().map_err(|why| {
                format!("the prepared population block could not be synced: {why}")
            })?;
        } else if !rows.is_empty() {
            self.append_rows(rows, expected)?;
        }

        reserve_map(&mut self.receipts, 1, "completion-receipt index")?;
        reserve_map(&mut self.committed, 1, "committed-population index")?;
        self.append_receipt(receipt)?;
        let block = self
            .raw_blocks
            .get(&receipt.population_id)
            .map_or(PopulationBlock { first: 0, count: 0 }, |facts| facts.block);
        self.receipts.insert(receipt.population_id, *receipt);
        self.committed.insert(
            receipt.population_id,
            CommittedPopulation {
                block,
                receipt: *receipt,
            },
        );
        Ok(PopulationCommit::Written)
    }

    fn append_complete_v3_locked(
        &mut self,
        rows: &[PopulationRowV1],
        receipt: &CompletionReceiptV3,
        expected: BlockFacts,
    ) -> Result<PopulationCommit, PopulationRefusal> {
        self.absorb_rows()?;
        self.absorb_receipts()?;
        self.absorb_receipts_v3()?;
        self.absorb_receipts_v4()?;
        let v2 = receipt.v2();
        if let Some(audit) = self.receipts.get(&v2.population_id)
            && *audit != v2
        {
            return Err(format!(
                "population {} has V2 audit facts different from the offered V3 receipt; no byte was replaced",
                hex(&v2.population_id)
            ));
        }
        if let Some(authoritative) = self.receipts_v4.get(&v2.population_id)
            && authoritative.v3() != *receipt
        {
            return Err(format!(
                "population {} already has authoritative V4 facts different from the offered V3 audit receipt; no byte was replaced",
                hex(&v2.population_id)
            ));
        }
        if let Some(existing) = self.receipts_v3.get(&v2.population_id).copied() {
            if existing != *receipt {
                return Err(format!(
                    "population {} already has a different authoritative V3 completion receipt; no byte was replaced",
                    hex(&v2.population_id)
                ));
            }
            self.require_exact_existing(rows, &v2, expected)?;
            self.row_file.sync_all().map_err(|why| {
                format!("the existing population rows could not be synced: {why}")
            })?;
            let file = self
                .receipt_v3_file
                .as_ref()
                .ok_or_else(|| "a writable population handle has no V3 receipt file".to_owned())?;
            file.sync_all().map_err(|why| {
                format!("the existing V3 population receipt could not be synced: {why}")
            })?;
            return Ok(PopulationCommit::Reused);
        }

        if self.raw_blocks.contains_key(&v2.population_id) {
            self.require_exact_existing(rows, &v2, expected)?;
            self.row_file.sync_all().map_err(|why| {
                format!("the prepared population block could not be synced: {why}")
            })?;
        } else if !rows.is_empty() {
            self.append_rows(rows, expected)?;
        }

        reserve_map(&mut self.receipts_v3, 1, "V3 completion-receipt index")?;
        reserve_map(
            &mut self.committed_v3,
            1,
            "authoritative V3 population index",
        )?;
        self.append_receipt_v3(receipt)?;
        let block = self
            .raw_blocks
            .get(&v2.population_id)
            .map_or(PopulationBlock { first: 0, count: 0 }, |facts| facts.block);
        self.receipts_v3.insert(v2.population_id, *receipt);
        self.committed_v3.insert(
            v2.population_id,
            AuthoritativePopulationV3 {
                block,
                receipt: *receipt,
            },
        );
        Ok(PopulationCommit::Written)
    }

    fn append_complete_v4_locked(
        &mut self,
        rows: &[PopulationRowV1],
        receipt: &CompletionReceiptV4,
        expected: BlockFacts,
    ) -> Result<PopulationCommit, PopulationRefusal> {
        let v3 = receipt.v3();
        let v2 = v3.v2();
        let population_id = v2.population_id;

        // Each lower version is an audit checkpoint in its own unchanged file.
        // These helpers re-absorb every generation before deciding reuse.
        self.append_complete_locked(rows, &v2, expected)?;
        self.append_complete_v3_locked(rows, &v3, expected)?;
        self.absorb_receipts_v4()?;

        if let Some(existing) = self.receipts_v4.get(&population_id).copied() {
            if existing != *receipt {
                return Err(format!(
                    "population {} already has a different authoritative V4 completion receipt; no byte was replaced",
                    hex(&population_id)
                ));
            }
            self.require_exact_existing(rows, &v2, expected)?;
            self.row_file.sync_all().map_err(|why| {
                format!("the existing population rows could not be synced: {why}")
            })?;
            let file = self
                .receipt_v4_file
                .as_ref()
                .ok_or_else(|| "a writable population handle has no V4 receipt file".to_owned())?;
            file.sync_all().map_err(|why| {
                format!("the existing V4 population receipt could not be synced: {why}")
            })?;
            return Ok(PopulationCommit::Reused);
        }

        reserve_map(&mut self.receipts_v4, 1, "V4 completion-receipt index")?;
        reserve_map(
            &mut self.committed_v4,
            1,
            "authoritative V4 population index",
        )?;
        self.append_receipt_v4(receipt)?;
        let block = self
            .raw_blocks
            .get(&population_id)
            .map_or(PopulationBlock { first: 0, count: 0 }, |facts| facts.block);
        self.receipts_v4.insert(population_id, *receipt);
        self.committed_v4.insert(
            population_id,
            AuthoritativePopulationV4 {
                block,
                receipt: *receipt,
            },
        );
        Ok(PopulationCommit::Written)
    }

    fn require_exact_existing(
        &mut self,
        rows: &[PopulationRowV1],
        receipt: &CompletionReceiptV2,
        expected: BlockFacts,
    ) -> Result<(), PopulationRefusal> {
        let actual = self.raw_blocks.get(&receipt.population_id).copied();
        if receipt.row_count == 0 {
            if actual.is_some() || !rows.is_empty() {
                return Err(format!(
                    "zero-row population {} has row bytes; no receipt was changed",
                    hex(&receipt.population_id)
                ));
            }
            return Ok(());
        }
        let actual = actual.ok_or_else(|| {
            format!(
                "population {} has a receipt but no row block",
                hex(&receipt.population_id)
            )
        })?;
        let mut expected_at_actual_location = expected;
        expected_at_actual_location.block.first = actual.block.first;
        if actual != expected_at_actual_location {
            return Err(format!(
                "population {} already has different row facts; no byte was replaced",
                hex(&receipt.population_id)
            ));
        }
        self.require_exact_block(actual.block, &receipt.population_id, rows)
    }

    fn append_rows(
        &mut self,
        rows: &[PopulationRowV1],
        expected: BlockFacts,
    ) -> Result<(), PopulationRefusal> {
        reserve_map(&mut self.raw_blocks, 1, "population-block index")?;
        let row_count = u64::try_from(rows.len())
            .map_err(|_| "population row count does not fit u64".to_owned())?;
        let written = row_count
            .checked_mul(ROW_STRIDE)
            .ok_or_else(|| "population row byte length overflowed u64".to_owned())?;
        let at = self
            .row_file
            .seek(SeekFrom::End(0))
            .map_err(|why| format!("{} could not be extended: {why}", self.row_path.display()))?;
        let mut buffer = [0_u8; ROW_WRITE_CHUNK_BYTES];
        for chunk in rows.chunks(ROW_WRITE_CHUNK_ROWS) {
            let mut used = 0_usize;
            for row in chunk {
                let raw = row.to_bytes().map_err(|why| {
                    rollback_population_message(&self.row_file, at, "population rows", &why)
                })?;
                let end = used.checked_add(ROW_STRIDE_BYTES).ok_or_else(|| {
                    rollback_population_message(
                        &self.row_file,
                        at,
                        "population rows",
                        "the fixed write-chunk offset overflowed usize",
                    )
                })?;
                let slot = buffer.get_mut(used..end).ok_or_else(|| {
                    rollback_population_message(
                        &self.row_file,
                        at,
                        "population rows",
                        "the fixed write chunk could not hold one canonical row",
                    )
                })?;
                slot.copy_from_slice(&raw);
                used = end;
            }
            let encoded = buffer.get(..used).ok_or_else(|| {
                rollback_population_message(
                    &self.row_file,
                    at,
                    "population rows",
                    "the encoded write-chunk width exceeded its fixed buffer",
                )
            })?;
            self.row_file
                .write_all(encoded)
                .map_err(|why| rollback_message(&self.row_file, at, "population rows", &why))?;
        }
        self.row_file
            .sync_all()
            .map_err(|why| format!("the new population rows could not be synced: {why}"))?;
        let first = at
            .checked_sub(HEADER)
            .ok_or_else(|| "population row file ended before its header".to_owned())?
            / ROW_STRIDE;
        let mut stored = expected;
        stored.block.first = first;
        let identity = rows
            .first()
            .map(|row| row.population_id)
            .ok_or_else(|| "an empty population row block cannot be appended".to_owned())?;
        self.raw_blocks.insert(identity, stored);
        self.row_scanned = at
            .checked_add(written)
            .ok_or_else(|| "population row scanned offset overflowed u64".to_owned())?;
        self.row_generation = validated_generation(
            &self.row_file,
            &self.row_path,
            self.row_scanned,
            "population row file",
        )?;
        Ok(())
    }

    fn append_receipt(&mut self, receipt: &CompletionReceiptV2) -> Result<(), PopulationRefusal> {
        let at = self.receipt_file.seek(SeekFrom::End(0)).map_err(|why| {
            format!(
                "{} could not be extended: {why}",
                self.receipt_path.display()
            )
        })?;
        let raw = receipt.to_bytes()?;
        self.receipt_file.write_all(&raw).map_err(|why| {
            rollback_message(
                &self.receipt_file,
                at,
                "population completion receipt",
                &why,
            )
        })?;
        self.receipt_file.sync_all().map_err(|why| {
            format!("the new population completion receipt could not be synced: {why}")
        })?;
        self.receipt_scanned = at.saturating_add(RECEIPT_STRIDE);
        self.receipt_generation = validated_generation(
            &self.receipt_file,
            &self.receipt_path,
            self.receipt_scanned,
            "population V2 receipt file",
        )?;
        Ok(())
    }

    fn append_receipt_v3(
        &mut self,
        receipt: &CompletionReceiptV3,
    ) -> Result<(), PopulationRefusal> {
        let file = self
            .receipt_v3_file
            .as_mut()
            .ok_or_else(|| "a writable population handle has no V3 receipt file".to_owned())?;
        let at = file.seek(SeekFrom::End(0)).map_err(|why| {
            format!(
                "{} could not be extended: {why}",
                self.receipt_v3_path.display()
            )
        })?;
        let raw = receipt.to_bytes()?;
        file.write_all(&raw)
            .map_err(|why| rollback_message(file, at, "population V3 completion receipt", &why))?;
        file.sync_all()
            .map_err(|why| format!("the new population V3 receipt could not be synced: {why}"))?;
        self.receipt_v3_scanned = at.saturating_add(RECEIPT_V3_STRIDE);
        self.receipt_v3_generation = Some(validated_generation(
            file,
            &self.receipt_v3_path,
            self.receipt_v3_scanned,
            "population V3 receipt file",
        )?);
        Ok(())
    }

    fn append_receipt_v4(
        &mut self,
        receipt: &CompletionReceiptV4,
    ) -> Result<(), PopulationRefusal> {
        let file = self
            .receipt_v4_file
            .as_mut()
            .ok_or_else(|| "a writable population handle has no V4 receipt file".to_owned())?;
        let at = file.seek(SeekFrom::End(0)).map_err(|why| {
            format!(
                "{} could not be extended: {why}",
                self.receipt_v4_path.display()
            )
        })?;
        let raw = receipt.to_bytes()?;
        file.write_all(&raw)
            .map_err(|why| rollback_message(file, at, "population V4 completion receipt", &why))?;
        file.sync_all()
            .map_err(|why| format!("the new population V4 receipt could not be synced: {why}"))?;
        self.receipt_v4_scanned = at.saturating_add(RECEIPT_V4_STRIDE);
        self.receipt_v4_generation = Some(validated_generation(
            file,
            &self.receipt_v4_path,
            self.receipt_v4_scanned,
            "population V4 receipt file",
        )?);
        Ok(())
    }

    fn absorb_rows(&mut self) -> Result<(), PopulationRefusal> {
        let observed = file_generation(&self.row_file, &self.row_path)?;
        let len = observed.len;
        if len < self.row_scanned {
            return Err(format!(
                "{} shrank from byte {} to {len}; append-only population history was replaced",
                self.row_path.display(),
                self.row_scanned
            ));
        }
        check_ragged(&self.row_path, len, ROW_STRIDE, "population rows")?;
        if len == self.row_scanned {
            require_generation_unchanged(
                self.row_generation,
                observed,
                &self.row_path,
                "population row file",
            )?;
            self.row_generation = observed;
            return Ok(());
        }
        let new_blocks =
            index_rows_range(&mut self.row_file, &self.row_path, self.row_scanned, len)?;
        reserve_map(
            &mut self.raw_blocks,
            new_blocks.len(),
            "population-block index",
        )?;
        for (identity, facts) in new_blocks {
            if self.raw_blocks.insert(identity, facts).is_some() {
                return Err(format!(
                    "{} gained a duplicate/non-contiguous block for population {}",
                    self.row_path.display(),
                    hex(&identity)
                ));
            }
        }
        self.row_scanned = len;
        self.row_generation =
            validated_generation(&self.row_file, &self.row_path, len, "population row file")?;
        Ok(())
    }

    fn absorb_receipts(&mut self) -> Result<(), PopulationRefusal> {
        let observed = file_generation(&self.receipt_file, &self.receipt_path)?;
        let len = observed.len;
        if len < self.receipt_scanned {
            return Err(format!(
                "{} shrank from byte {} to {len}; append-only completion history was replaced",
                self.receipt_path.display(),
                self.receipt_scanned
            ));
        }
        check_ragged(
            &self.receipt_path,
            len,
            RECEIPT_STRIDE,
            "population completion receipts",
        )?;
        if len == self.receipt_scanned {
            require_generation_unchanged(
                self.receipt_generation,
                observed,
                &self.receipt_path,
                "population V2 receipt file",
            )?;
            self.receipt_generation = observed;
            return Ok(());
        }
        let mut at = self.receipt_scanned;
        while at < len {
            let receipt = read_receipt_at(&mut self.receipt_file, &self.receipt_path, at)?;
            if self.receipts.contains_key(&receipt.population_id) {
                return Err(format!(
                    "{} gained duplicate receipt identity {}",
                    self.receipt_path.display(),
                    hex(&receipt.population_id)
                ));
            }
            let facts = self.raw_blocks.get(&receipt.population_id);
            validate_receipt_v2_against_facts(&receipt, facts)?;
            if let Some(authoritative) = self.receipts_v3.get(&receipt.population_id)
                && authoritative.v2() != receipt
            {
                return Err(format!(
                    "{} gained V2 facts that conflict with authoritative V3 population {}",
                    self.receipt_path.display(),
                    hex(&receipt.population_id)
                ));
            }
            let block = facts.map_or(PopulationBlock { first: 0, count: 0 }, |held| held.block);
            reserve_map(&mut self.receipts, 1, "completion-receipt index")?;
            reserve_map(&mut self.committed, 1, "committed-population index")?;
            self.receipts.insert(receipt.population_id, receipt);
            self.committed.insert(
                receipt.population_id,
                CommittedPopulation { block, receipt },
            );
            at = at.saturating_add(RECEIPT_STRIDE);
        }
        self.receipt_scanned = len;
        self.receipt_generation = validated_generation(
            &self.receipt_file,
            &self.receipt_path,
            len,
            "population V2 receipt file",
        )?;
        Ok(())
    }

    fn absorb_receipts_v3(&mut self) -> Result<(), PopulationRefusal> {
        let Some(file) = self.receipt_v3_file.as_mut() else {
            if self.writable {
                return Err("a writable population handle has no V3 receipt file".to_owned());
            }
            return Ok(());
        };
        let observed = file_generation(file, &self.receipt_v3_path)?;
        let len = observed.len;
        if len < self.receipt_v3_scanned {
            return Err(format!(
                "{} shrank from byte {} to {len}; append-only V3 completion history was replaced",
                self.receipt_v3_path.display(),
                self.receipt_v3_scanned
            ));
        }
        check_ragged(
            &self.receipt_v3_path,
            len,
            RECEIPT_V3_STRIDE,
            "population V3 completion receipts",
        )?;
        if len == self.receipt_v3_scanned {
            let expected = self.receipt_v3_generation.ok_or_else(|| {
                "an opened population V3 receipt file has no retained filesystem generation"
                    .to_owned()
            })?;
            require_generation_unchanged(
                expected,
                observed,
                &self.receipt_v3_path,
                "population V3 receipt file",
            )?;
            self.receipt_v3_generation = Some(observed);
            return Ok(());
        }
        let mut at = self.receipt_v3_scanned;
        while at < len {
            let receipt = read_receipt_v3_at(file, &self.receipt_v3_path, at)?;
            let population_id = receipt.population_id();
            if self.receipts_v3.contains_key(&population_id) {
                return Err(format!(
                    "{} gained duplicate V3 receipt identity {}",
                    self.receipt_v3_path.display(),
                    hex(&population_id)
                ));
            }
            let v2 = receipt.v2();
            if let Some(audit) = self.receipts.get(&population_id)
                && *audit != v2
            {
                return Err(format!(
                    "{} gained V3 facts that conflict with V2 audit population {}",
                    self.receipt_v3_path.display(),
                    hex(&population_id)
                ));
            }
            let facts = self.raw_blocks.get(&population_id);
            validate_receipt_v2_against_facts(&v2, facts)?;
            let block = facts.map_or(PopulationBlock { first: 0, count: 0 }, |held| held.block);
            reserve_map(&mut self.receipts_v3, 1, "V3 completion-receipt index")?;
            reserve_map(
                &mut self.committed_v3,
                1,
                "authoritative V3 population index",
            )?;
            self.receipts_v3.insert(population_id, receipt);
            self.committed_v3
                .insert(population_id, AuthoritativePopulationV3 { block, receipt });
            at = at.saturating_add(RECEIPT_V3_STRIDE);
        }
        self.receipt_v3_scanned = len;
        self.receipt_v3_generation = Some(validated_generation(
            file,
            &self.receipt_v3_path,
            len,
            "population V3 receipt file",
        )?);
        Ok(())
    }

    fn absorb_receipts_v4(&mut self) -> Result<(), PopulationRefusal> {
        let Some(file) = self.receipt_v4_file.as_mut() else {
            if self.writable {
                return Err("a writable population handle has no V4 receipt file".to_owned());
            }
            return Ok(());
        };
        let observed = file_generation(file, &self.receipt_v4_path)?;
        let len = observed.len;
        if len < self.receipt_v4_scanned {
            return Err(format!(
                "{} shrank from byte {} to {len}; append-only V4 completion history was replaced",
                self.receipt_v4_path.display(),
                self.receipt_v4_scanned
            ));
        }
        check_ragged(
            &self.receipt_v4_path,
            len,
            RECEIPT_V4_STRIDE,
            "population V4 completion receipts",
        )?;
        if len == self.receipt_v4_scanned {
            let expected = self.receipt_v4_generation.ok_or_else(|| {
                "an opened population V4 receipt file has no retained filesystem generation"
                    .to_owned()
            })?;
            require_generation_unchanged(
                expected,
                observed,
                &self.receipt_v4_path,
                "population V4 receipt file",
            )?;
            self.receipt_v4_generation = Some(observed);
            return Ok(());
        }
        let mut at = self.receipt_v4_scanned;
        while at < len {
            let receipt = read_receipt_v4_at(file, &self.receipt_v4_path, at)?;
            let population_id = receipt.population_id();
            if self.receipts_v4.contains_key(&population_id) {
                return Err(format!(
                    "{} gained duplicate V4 receipt identity {}",
                    self.receipt_v4_path.display(),
                    hex(&population_id)
                ));
            }
            let v3 = receipt.v3();
            let v2 = v3.v2();
            let audit_v2 = self.receipts.get(&population_id).ok_or_else(|| {
                format!(
                    "{} gained V4 population {} without its V2 audit receipt",
                    self.receipt_v4_path.display(),
                    hex(&population_id)
                )
            })?;
            if *audit_v2 != v2 {
                return Err(format!(
                    "{} gained V4 facts that conflict with V2 audit population {}",
                    self.receipt_v4_path.display(),
                    hex(&population_id)
                ));
            }
            let audit_v3 = self.receipts_v3.get(&population_id).ok_or_else(|| {
                format!(
                    "{} gained V4 population {} without its byte-identical V3 audit receipt",
                    self.receipt_v4_path.display(),
                    hex(&population_id)
                )
            })?;
            if *audit_v3 != v3 {
                return Err(format!(
                    "{} gained V4 facts that conflict with V3 audit population {}",
                    self.receipt_v4_path.display(),
                    hex(&population_id)
                ));
            }
            let facts = self.raw_blocks.get(&population_id);
            validate_receipt_v2_against_facts(&v2, facts)?;
            let block = facts.map_or(PopulationBlock { first: 0, count: 0 }, |held| held.block);
            reserve_map(&mut self.receipts_v4, 1, "V4 completion-receipt index")?;
            reserve_map(
                &mut self.committed_v4,
                1,
                "authoritative V4 population index",
            )?;
            self.receipts_v4.insert(population_id, receipt);
            self.committed_v4
                .insert(population_id, AuthoritativePopulationV4 { block, receipt });
            at = at.saturating_add(RECEIPT_V4_STRIDE);
        }
        self.receipt_v4_scanned = len;
        self.receipt_v4_generation = Some(validated_generation(
            file,
            &self.receipt_v4_path,
            len,
            "population V4 receipt file",
        )?);
        Ok(())
    }

    /// Reads one bounded contiguous page from a committed population.
    ///
    /// Finding the block is one hash probe.  Reading and decoding costs
    /// O(returned rows).  `limit == 0` is a legitimate empty page; a larger
    /// request than [`MAX_PAGE_ROWS_V1`] is refused rather than silently clamped.
    ///
    /// # Errors
    ///
    /// Refuses a limit above [`MAX_PAGE_ROWS_V1`], an offset past the committed
    /// count, arithmetic/address-space/allocation failure, locking/I/O failure,
    /// or any damaged/foreign/reordered row. No valid prefix is returned on
    /// failure.
    pub fn page(
        &mut self,
        population_id: &[u8; 32],
        offset: u64,
        limit: u64,
    ) -> Result<Option<PopulationPageV1>, PopulationRefusal> {
        if limit > MAX_PAGE_ROWS_V1 {
            return Err(format!(
                "population page limit {limit} exceeds the version-one maximum {MAX_PAGE_ROWS_V1}"
            ));
        }
        let block = if let Some(entry) = self.committed_v4.get(population_id) {
            entry.block
        } else if let Some(entry) = self.committed_v3.get(population_id) {
            entry.block
        } else if let Some(entry) = self.committed.get(population_id) {
            entry.block
        } else {
            return Ok(None);
        };
        if offset > block.count {
            return Err(format!(
                "population {} page offset {offset} is past its {} committed rows",
                hex(population_id),
                block.count
            ));
        }
        let count = limit.min(block.count.saturating_sub(offset));
        if count == 0 {
            return Ok(Some(PopulationPageV1 {
                total: block.count,
                offset,
                rows: Vec::new(),
            }));
        }
        let first = block
            .first
            .checked_add(offset)
            .ok_or_else(|| "population page row offset overflowed u64".to_owned())?;
        let at = HEADER
            .checked_add(
                first
                    .checked_mul(ROW_STRIDE)
                    .ok_or_else(|| "population page byte offset overflowed u64".to_owned())?,
            )
            .ok_or_else(|| "population page byte address overflowed u64".to_owned())?;
        let capacity = usize::try_from(count)
            .map_err(|_| format!("population page count {count} does not fit this machine"))?;
        let mut rows = Vec::new();
        rows.try_reserve_exact(capacity).map_err(|why| {
            format!("population page could not reserve {capacity} decoded row slots: {why}")
        })?;
        self.row_file
            .lock_shared()
            .map_err(|why| format!("the population row file could not be shared-locked: {why}"))?;
        let read = (|| {
            self.row_file
                .seek(SeekFrom::Start(at))
                .map_err(|why| format!("the population page could not be seeked: {why}"))?;
            for nth in 0..count {
                let mut bytes = [0_u8; ROW_STRIDE_BYTES];
                self.row_file
                    .read_exact(&mut bytes)
                    .map_err(|why| format!("the population page could not be read: {why}"))?;
                let row = PopulationRowV1::from_bytes(&bytes).map_err(|why| {
                    format!(
                        "population {} page row {} is invalid: {why}",
                        hex(population_id),
                        first.saturating_add(nth)
                    )
                })?;
                let expected_sequence = offset
                    .checked_add(nth)
                    .ok_or_else(|| "population page sequence overflowed u64".to_owned())?;
                if &row.population_id != population_id || row.sequence != expected_sequence {
                    return Err(format!(
                        "population page expected identity {} sequence {expected_sequence}, found identity {} sequence {}",
                        hex(population_id),
                        hex(&row.population_id),
                        row.sequence
                    ));
                }
                rows.push(row);
            }
            Ok(PopulationPageV1 {
                total: block.count,
                offset,
                rows,
            })
        })();
        let released = self
            .row_file
            .unlock()
            .map_err(|why| format!("the population row file could not be unlocked: {why}"));
        match (read, released) {
            (Ok(page), Ok(())) => Ok(Some(page)),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Reads one bounded page only when V4 calendar authority is present and
    /// every indexed file generation is still exact.
    ///
    /// This is the Step-3 ranking boundary. Legacy [`Self::page`] remains an
    /// explicit audit reader for V2/V3 callers, but cannot be mistaken for V4
    /// admission by new selection code.
    ///
    /// # Errors
    ///
    /// Every refusal from [`Self::page`], plus any post-open replacement,
    /// same-length mutation, or append to rows or V2/V3/V4 receipt files. A
    /// changed generation requires a full reopen/revalidation before serving.
    pub fn page_v4(
        &mut self,
        population_id: &[u8; 32],
        offset: u64,
        limit: u64,
    ) -> Result<Option<PopulationPageV1>, PopulationRefusal> {
        self.with_shared_writer_lock("a V4 page", |ledger| {
            require_generation_unchanged(
                ledger.row_generation,
                file_generation(&ledger.row_file, &ledger.row_path)?,
                &ledger.row_path,
                "population row file",
            )?;
            require_generation_unchanged(
                ledger.receipt_generation,
                file_generation(&ledger.receipt_file, &ledger.receipt_path)?,
                &ledger.receipt_path,
                "population V2 receipt file",
            )?;
            require_optional_generation_unchanged(
                ledger.receipt_v3_file.as_ref(),
                ledger.receipt_v3_generation,
                &ledger.receipt_v3_path,
                "population V3 receipt file",
            )?;
            require_optional_generation_unchanged(
                ledger.receipt_v4_file.as_ref(),
                ledger.receipt_v4_generation,
                &ledger.receipt_v4_path,
                "population V4 receipt file",
            )?;
            if !ledger.committed_v4.contains_key(population_id) {
                return Ok(None);
            }
            ledger.page(population_id, offset, limit)
        })
    }

    fn with_shared_writer_lock<T>(
        &mut self,
        purpose: &str,
        operation: impl FnOnce(&mut Self) -> Result<T, PopulationRefusal>,
    ) -> Result<T, PopulationRefusal> {
        self.writer_lock.lock_shared().map_err(|why| {
            format!("the population writer lock could not be shared-locked for {purpose}: {why}")
        })?;
        let attempted = operation(self);
        let released = self.writer_lock.unlock().map_err(|why| {
            format!("the population writer lock could not be released after {purpose}: {why}")
        });
        match (attempted, released) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(why), _) | (Ok(_), Err(why)) => Err(why),
        }
    }

    /// Reads one row by its canonical sequence number.
    ///
    /// # Errors
    ///
    /// Every refusal from [`Self::page`].  A sequence at or beyond the count is
    /// refused rather than returned as an empty page.
    pub fn row(
        &mut self,
        population_id: &[u8; 32],
        sequence: u64,
    ) -> Result<Option<PopulationRowV1>, PopulationRefusal> {
        let Some(block) = self
            .block_v4(population_id)
            .or_else(|| self.block_v3(population_id))
            .or_else(|| self.block(population_id))
        else {
            return Ok(None);
        };
        if sequence >= block.count {
            return Err(format!(
                "population {} row {sequence} is past its {} committed rows",
                hex(population_id),
                block.count
            ));
        }
        let page = self.page(population_id, sequence, 1)?.ok_or_else(|| {
            "a committed population disappeared from its in-memory index".to_owned()
        })?;
        page.rows
            .first()
            .copied()
            .map(Some)
            .ok_or_else(|| "a one-row population page decoded no row".to_owned())
    }

    fn require_exact_block(
        &mut self,
        block: PopulationBlock,
        population_id: &[u8; 32],
        expected_rows: &[PopulationRowV1],
    ) -> Result<(), PopulationRefusal> {
        let expected_count = u64::try_from(expected_rows.len())
            .map_err(|_| "population comparison row count does not fit u64".to_owned())?;
        if block.count != expected_count {
            return Err(format!(
                "population {} block has {} rows, not the {expected_count} rows offered for exact reuse",
                hex(population_id),
                block.count
            ));
        }
        let at = HEADER
            .checked_add(
                block
                    .first
                    .checked_mul(ROW_STRIDE)
                    .ok_or_else(|| "population block offset overflowed u64".to_owned())?,
            )
            .ok_or_else(|| "population block address overflowed u64".to_owned())?;
        self.row_file
            .seek(SeekFrom::Start(at))
            .map_err(|why| format!("the population block could not be seeked: {why}"))?;
        for (nth, expected) in expected_rows.iter().enumerate() {
            let mut bytes = [0_u8; ROW_STRIDE_BYTES];
            self.row_file
                .read_exact(&mut bytes)
                .map_err(|why| format!("the population block could not be read: {why}"))?;
            let row = PopulationRowV1::from_bytes(&bytes)?;
            let sequence = u64::try_from(nth).unwrap_or(u64::MAX);
            if &row.population_id != population_id || row.sequence != sequence {
                return Err(format!(
                    "population block {} expected sequence {sequence}, found identity {} sequence {}",
                    hex(population_id),
                    hex(&row.population_id),
                    row.sequence
                ));
            }
            if &row != expected {
                return Err(format!(
                    "population {} has the same count/digest facts but different canonical row bytes at sequence {sequence}; no byte was replaced",
                    hex(population_id)
                ));
            }
        }
        Ok(())
    }
}

fn facts_of_rows(
    population_id: [u8; 32],
    rows: &[PopulationRowV1],
) -> Result<BlockFacts, PopulationRefusal> {
    require_nonzero_digest("population_id", &population_id)?;
    let mut builder = FactsBuilder::new(population_id, 0);
    for (nth, row) in rows.iter().copied().enumerate() {
        let expected = u64::try_from(nth).unwrap_or(u64::MAX);
        if row.population_id != population_id {
            return Err(format!(
                "population row {nth} belongs to {}, not {}",
                hex(&row.population_id),
                hex(&population_id)
            ));
        }
        if row.sequence != expected {
            return Err(format!(
                "population {} row {nth} carries sequence {}, not canonical sequence {expected}",
                hex(&population_id),
                row.sequence
            ));
        }
        builder.observe(&row)?;
    }
    builder.finish()
}

struct FactsBuilder {
    population_id: [u8; 32],
    first: u64,
    count: u64,
    hasher: brutex_core::blake3::Hasher,
    masks: HashSet<[u64; 6]>,
    direction_cells: HashMap<([u64; 6], TradeDirectionV1), u64>,
    strategies: HashSet<[u8; 32]>,
    coordinates: HashSet<([u64; 6], TradeDirectionV1, ExitCoordinateV1)>,
    admitted: u64,
    rejected: u64,
    unmeasured: u64,
    refused: u64,
    topn_undefined: u64,
    instrument_family: Option<InstrumentFamilyV1>,
    rung_seconds: Option<u32>,
}

impl FactsBuilder {
    fn new(population_id: [u8; 32], first: u64) -> Self {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(EMPTY_ROW_DIGEST_DOMAIN);
        Self {
            population_id,
            first,
            count: 0,
            hasher,
            masks: HashSet::new(),
            direction_cells: HashMap::new(),
            strategies: HashSet::new(),
            coordinates: HashSet::new(),
            admitted: 0,
            rejected: 0,
            unmeasured: 0,
            refused: 0,
            topn_undefined: 0,
            instrument_family: None,
            rung_seconds: None,
        }
    }

    fn observe(&mut self, row: &PopulationRowV1) -> Result<(), PopulationRefusal> {
        row.validate()?;
        if row.population_id != self.population_id {
            return Err("one population facts block crossed identities".to_owned());
        }
        if row.sequence != self.count {
            return Err(format!(
                "population {} sequence jumped from expected {} to {}",
                hex(&self.population_id),
                self.count,
                row.sequence
            ));
        }
        if row.closure != ClosureV1::Closed {
            return Err(format!(
                "expanded strategy row {} is {:?}; only closed itemsets enter direction/cell expansion",
                row.sequence, row.closure
            ));
        }
        match self.instrument_family {
            Some(family) if family != row.instrument_family => {
                return Err("one population block crossed instrument families".to_owned());
            }
            None => self.instrument_family = Some(row.instrument_family),
            Some(_) => {}
        }
        match self.rung_seconds {
            Some(rung) if rung != row.rung_seconds => {
                return Err("one population block crossed signal timeframes".to_owned());
            }
            None => self.rung_seconds = Some(row.rung_seconds),
            Some(_) => {}
        }
        self.reserve_one()?;
        if !self.strategies.insert(row.strategy_digest) {
            return Err(format!(
                "population {} repeats strategy digest {}",
                hex(&self.population_id),
                hex(&row.strategy_digest)
            ));
        }
        if !self
            .coordinates
            .insert((row.mask_words, row.direction, row.exit))
        {
            return Err(format!(
                "population {} repeats mask/direction/exit coordinate at sequence {}",
                hex(&self.population_id),
                row.sequence
            ));
        }
        self.masks.insert(row.mask_words);
        let side_cells = self
            .direction_cells
            .entry((row.mask_words, row.direction))
            .or_insert(0);
        *side_cells = side_cells.checked_add(1).ok_or_else(|| {
            format!(
                "population {} coordinate count overflowed for one mask/direction",
                hex(&self.population_id)
            )
        })?;
        match row.admission.status {
            AdmissionStatusV1::Admitted => self.admitted = self.admitted.saturating_add(1),
            AdmissionStatusV1::Rejected => self.rejected = self.rejected.saturating_add(1),
            AdmissionStatusV1::Unmeasured => {
                self.unmeasured = self.unmeasured.saturating_add(1);
            }
            AdmissionStatusV1::Refused => self.refused = self.refused.saturating_add(1),
        }
        if row.metrics.loss_ratio_ppm.is_none() || row.metrics.reward_to_risk_ppm.is_none() {
            self.topn_undefined = self.topn_undefined.saturating_add(1);
        }
        self.hasher.update(&row.payload_bytes()?);
        self.count = self.count.saturating_add(1);
        Ok(())
    }

    fn reserve_one(&mut self) -> Result<(), PopulationRefusal> {
        for (subject, result) in [
            ("mask set", self.masks.try_reserve(1)),
            (
                "direction-member count map",
                self.direction_cells.try_reserve(1),
            ),
            ("strategy-digest set", self.strategies.try_reserve(1)),
            ("exit-coordinate set", self.coordinates.try_reserve(1)),
        ] {
            result.map_err(|why| {
                format!("population facts {subject} could not reserve one more slot: {why}")
            })?;
        }
        Ok(())
    }

    fn finish(self) -> Result<BlockFacts, PopulationRefusal> {
        let masks = u64::try_from(self.masks.len())
            .map_err(|_| "population mask count does not fit u64".to_owned())?;
        let direction_members = u64::try_from(self.direction_cells.len())
            .map_err(|_| "population direction-member count does not fit u64".to_owned())?;
        let mut long_rows = 0_u64;
        let mut short_rows = 0_u64;
        let mut long_cells_min = u64::MAX;
        let mut long_cells_max = 0_u64;
        let mut short_cells_min = u64::MAX;
        let mut short_cells_max = 0_u64;
        for mask in &self.masks {
            let long = self
                .direction_cells
                .get(&(*mask, TradeDirectionV1::Long))
                .copied()
                .unwrap_or(0);
            let short = self
                .direction_cells
                .get(&(*mask, TradeDirectionV1::Short))
                .copied()
                .unwrap_or(0);
            long_rows = long_rows
                .checked_add(long)
                .ok_or_else(|| "long population row count overflowed u64".to_owned())?;
            short_rows = short_rows
                .checked_add(short)
                .ok_or_else(|| "short population row count overflowed u64".to_owned())?;
            long_cells_min = long_cells_min.min(long);
            long_cells_max = long_cells_max.max(long);
            short_cells_min = short_cells_min.min(short);
            short_cells_max = short_cells_max.max(short);
        }
        if self.masks.is_empty() {
            long_cells_min = 0;
            short_cells_min = 0;
        }
        Ok(BlockFacts {
            block: PopulationBlock {
                first: self.first,
                count: self.count,
            },
            digest: self.hasher.finalize(),
            masks,
            direction_members,
            long_rows,
            short_rows,
            long_cells_min,
            long_cells_max,
            short_cells_min,
            short_cells_max,
            admitted: self.admitted,
            rejected: self.rejected,
            unmeasured: self.unmeasured,
            refused: self.refused,
            topn_undefined: self.topn_undefined,
            instrument_family: self.instrument_family,
            rung_seconds: self.rung_seconds,
        })
    }
}

fn validate_receipt_against_facts(
    receipt: &CompletionReceiptV1,
    facts: Option<&BlockFacts>,
) -> Result<(), PopulationRefusal> {
    receipt.validate_semantics()?;
    let empty_digest = empty_row_digest();
    let (count, digest, masks, directions, admitted, rejected, unmeasured, refused, undefined) =
        facts.map_or((0, empty_digest, 0, 0, 0, 0, 0, 0, 0), |facts| {
            (
                facts.block.count,
                facts.digest,
                facts.masks,
                facts.direction_members,
                facts.admitted,
                facts.rejected,
                facts.unmeasured,
                facts.refused,
                facts.topn_undefined,
            )
        });
    if receipt.row_count != count || receipt.ordered_row_digest != digest {
        return Err(format!(
            "population {} receipt commits {} row(s) digest {}, but the contiguous block has {count} row(s) digest {}",
            hex(&receipt.population_id),
            receipt.row_count,
            hex(&receipt.ordered_row_digest),
            hex(&digest)
        ));
    }
    if receipt.closed_itemsets != masks {
        return Err(format!(
            "population {} receipt claims {} closed itemsets, but rows contain {masks} distinct masks",
            hex(&receipt.population_id),
            receipt.closed_itemsets
        ));
    }
    if receipt.direction_members_evaluated != directions {
        return Err(format!(
            "population {} receipt claims {} evaluated direction members, but rows contain {directions}",
            hex(&receipt.population_id),
            receipt.direction_members_evaluated
        ));
    }
    for (name, claimed, actual) in [
        ("admitted", receipt.admitted_rows, admitted),
        ("rejected", receipt.rejected_rows, rejected),
        ("unmeasured", receipt.unmeasured_rows, unmeasured),
        ("refused", receipt.refused_rows, refused),
        ("topn undefined", receipt.topn_undefined_rows, undefined),
    ] {
        if claimed != actual {
            return Err(format!(
                "population {} receipt claims {claimed} {name} row(s), but the row block has {actual}",
                hex(&receipt.population_id)
            ));
        }
    }
    if let Some(facts) = facts.filter(|held| held.block.count > 0)
        && (facts.instrument_family != Some(receipt.instrument_family)
            || facts.rung_seconds != Some(receipt.rung_seconds))
    {
        return Err(format!(
            "population {} receipt instrument/timeframe disagrees with its rows",
            hex(&receipt.population_id)
        ));
    }
    Ok(())
}

fn validate_receipt_v2_against_facts(
    receipt: &CompletionReceiptV2,
    facts: Option<&BlockFacts>,
) -> Result<(), PopulationRefusal> {
    receipt.validate_semantics()?;
    let facts = facts.copied().unwrap_or_else(empty_block_facts);
    if receipt.row_count != facts.block.count || receipt.ordered_row_digest != facts.digest {
        return Err(format!(
            "population {} V2 receipt commits {} row(s) digest {}, but the contiguous block has {} row(s) digest {}",
            hex(&receipt.population_id),
            receipt.row_count,
            hex(&receipt.ordered_row_digest),
            facts.block.count,
            hex(&facts.digest)
        ));
    }
    if receipt.closed_itemsets != facts.masks {
        return Err(format!(
            "population {} V2 receipt claims {} closed masks, but rows contain {} distinct masks",
            hex(&receipt.population_id),
            receipt.closed_itemsets,
            facts.masks
        ));
    }
    if receipt.direction_members_evaluated != facts.direction_members {
        return Err(format!(
            "population {} V2 receipt claims {} evaluated direction members, but rows contain {}",
            hex(&receipt.population_id),
            receipt.direction_members_evaluated,
            facts.direction_members
        ));
    }
    if receipt.long_exit_cells_evaluated != facts.long_rows
        || receipt.short_exit_cells_evaluated != facts.short_rows
    {
        return Err(format!(
            "population {} V2 receipt claims long/short rows {}/{}, but the block has {}/{}",
            hex(&receipt.population_id),
            receipt.long_exit_cells_evaluated,
            receipt.short_exit_cells_evaluated,
            facts.long_rows,
            facts.short_rows
        ));
    }
    if facts.masks > 0
        && (facts.long_cells_min != receipt.long_exit_cells_per_mask
            || facts.long_cells_max != receipt.long_exit_cells_per_mask
            || facts.short_cells_min != receipt.short_exit_cells_per_mask
            || facts.short_cells_max != receipt.short_exit_cells_per_mask)
    {
        return Err(format!(
            "population {} has a partial/non-uniform coordinate population: long min/max {}/{}, expected {}; short min/max {}/{}, expected {}",
            hex(&receipt.population_id),
            facts.long_cells_min,
            facts.long_cells_max,
            receipt.long_exit_cells_per_mask,
            facts.short_cells_min,
            facts.short_cells_max,
            receipt.short_exit_cells_per_mask
        ));
    }
    for (name, claimed, actual) in [
        ("admitted", receipt.admitted_rows, facts.admitted),
        ("rejected", receipt.rejected_rows, facts.rejected),
        ("unmeasured", receipt.unmeasured_rows, facts.unmeasured),
        ("refused", receipt.refused_rows, facts.refused),
        (
            "topn undefined",
            receipt.topn_undefined_rows,
            facts.topn_undefined,
        ),
    ] {
        if claimed != actual {
            return Err(format!(
                "population {} V2 receipt claims {claimed} {name} row(s), but the row block has {actual}",
                hex(&receipt.population_id)
            ));
        }
    }
    if facts.block.count > 0
        && (facts.instrument_family != Some(receipt.instrument_family)
            || facts.rung_seconds != Some(receipt.rung_seconds))
    {
        return Err(format!(
            "population {} V2 receipt instrument/timeframe disagrees with its rows",
            hex(&receipt.population_id)
        ));
    }
    Ok(())
}

fn empty_block_facts() -> BlockFacts {
    BlockFacts {
        block: PopulationBlock { first: 0, count: 0 },
        digest: empty_row_digest(),
        masks: 0,
        direction_members: 0,
        long_rows: 0,
        short_rows: 0,
        long_cells_min: 0,
        long_cells_max: 0,
        short_cells_min: 0,
        short_cells_max: 0,
        admitted: 0,
        rejected: 0,
        unmeasured: 0,
        refused: 0,
        topn_undefined: 0,
        instrument_family: None,
        rung_seconds: None,
    }
}

fn index_rows(
    file: &mut File,
    path: &Path,
    len: u64,
) -> Result<HashMap<[u8; 32], BlockFacts>, PopulationRefusal> {
    index_rows_range(file, path, HEADER, len)
}

fn index_rows_range(
    file: &mut File,
    path: &Path,
    start: u64,
    len: u64,
) -> Result<HashMap<[u8; 32], BlockFacts>, PopulationRefusal> {
    let mut blocks = HashMap::new();
    let mut builder: Option<FactsBuilder> = None;
    let mut at = start;
    while at < len {
        let row = read_row_at(file, path, at)?;
        let row_index = at.saturating_sub(HEADER) / ROW_STRIDE;
        let changes_identity = builder
            .as_ref()
            .is_some_and(|open| open.population_id != row.population_id);
        if changes_identity {
            let identity = builder
                .as_ref()
                .map(|open| open.population_id)
                .ok_or_else(|| "population block builder disappeared".to_owned())?;
            let finished = builder
                .take()
                .ok_or_else(|| "population block builder disappeared".to_owned())?
                .finish()?;
            reserve_map(&mut blocks, 1, "scanned population-block index")?;
            if blocks.insert(identity, finished).is_some() {
                return Err(format!(
                    "{} contains a non-contiguous duplicate population block {}",
                    path.display(),
                    hex(&identity)
                ));
            }
        }
        if builder.is_none() {
            if row.sequence != 0 {
                return Err(format!(
                    "{} population block at row {row_index} begins with sequence {}, not zero",
                    path.display(),
                    row.sequence
                ));
            }
            if blocks.contains_key(&row.population_id) {
                return Err(format!(
                    "{} interleaves a second block for population {}",
                    path.display(),
                    hex(&row.population_id)
                ));
            }
            builder = Some(FactsBuilder::new(row.population_id, row_index));
        }
        if let Some(open) = &mut builder {
            open.observe(&row).map_err(|why| {
                format!(
                    "{} population row {row_index} is invalid: {why}",
                    path.display()
                )
            })?;
        }
        at = at.saturating_add(ROW_STRIDE);
    }
    if let Some(open) = builder {
        let identity = open.population_id;
        let facts = open.finish()?;
        reserve_map(&mut blocks, 1, "scanned population-block index")?;
        if blocks.insert(identity, facts).is_some() {
            return Err(format!(
                "{} contains a non-contiguous duplicate population block {}",
                path.display(),
                hex(&identity)
            ));
        }
    }
    Ok(blocks)
}

fn index_receipts(
    file: &mut File,
    path: &Path,
    len: u64,
) -> Result<HashMap<[u8; 32], CompletionReceiptV2>, PopulationRefusal> {
    let mut receipts = HashMap::new();
    let mut at = HEADER;
    while at < len {
        let receipt = read_receipt_at(file, path, at)?;
        reserve_map(&mut receipts, 1, "scanned completion-receipt index")?;
        if receipts.insert(receipt.population_id, receipt).is_some() {
            return Err(format!(
                "{} contains duplicate completion receipt identity {}",
                path.display(),
                hex(&receipt.population_id)
            ));
        }
        at = at.saturating_add(RECEIPT_STRIDE);
    }
    Ok(receipts)
}

fn index_receipts_v3(
    file: &mut File,
    path: &Path,
    len: u64,
) -> Result<HashMap<[u8; 32], CompletionReceiptV3>, PopulationRefusal> {
    let mut receipts = HashMap::new();
    let mut at = HEADER;
    while at < len {
        let receipt = read_receipt_v3_at(file, path, at)?;
        let population_id = receipt.population_id();
        reserve_map(&mut receipts, 1, "scanned V3 completion-receipt index")?;
        if receipts.insert(population_id, receipt).is_some() {
            return Err(format!(
                "{} contains duplicate V3 completion receipt identity {}",
                path.display(),
                hex(&population_id)
            ));
        }
        at = at.saturating_add(RECEIPT_V3_STRIDE);
    }
    Ok(receipts)
}

fn index_receipts_v4(
    file: &mut File,
    path: &Path,
    len: u64,
) -> Result<HashMap<[u8; 32], CompletionReceiptV4>, PopulationRefusal> {
    let mut receipts = HashMap::new();
    let mut at = HEADER;
    while at < len {
        let receipt = read_receipt_v4_at(file, path, at)?;
        let population_id = receipt.population_id();
        reserve_map(&mut receipts, 1, "scanned V4 completion-receipt index")?;
        if receipts.insert(population_id, receipt).is_some() {
            return Err(format!(
                "{} contains duplicate V4 completion receipt identity {}",
                path.display(),
                hex(&population_id)
            ));
        }
        at = at.saturating_add(RECEIPT_V4_STRIDE);
    }
    Ok(receipts)
}

fn reconcile_receipts(
    blocks: &HashMap<[u8; 32], BlockFacts>,
    receipts: &HashMap<[u8; 32], CompletionReceiptV2>,
) -> Result<HashMap<[u8; 32], CommittedPopulation>, PopulationRefusal> {
    let mut committed = HashMap::new();
    reserve_map(
        &mut committed,
        receipts.len(),
        "reconciled committed-population index",
    )?;
    for (identity, receipt) in receipts {
        let facts = blocks.get(identity);
        validate_receipt_v2_against_facts(receipt, facts)?;
        let block = facts.map_or(PopulationBlock { first: 0, count: 0 }, |held| held.block);
        committed.insert(
            *identity,
            CommittedPopulation {
                block,
                receipt: *receipt,
            },
        );
    }
    Ok(committed)
}

fn reconcile_receipts_v3(
    blocks: &HashMap<[u8; 32], BlockFacts>,
    audit_v2: &HashMap<[u8; 32], CompletionReceiptV2>,
    receipts: &HashMap<[u8; 32], CompletionReceiptV3>,
) -> Result<HashMap<[u8; 32], AuthoritativePopulationV3>, PopulationRefusal> {
    let mut committed = HashMap::new();
    reserve_map(
        &mut committed,
        receipts.len(),
        "reconciled authoritative V3 population index",
    )?;
    for (identity, receipt) in receipts {
        let v2 = receipt.v2();
        if let Some(audit) = audit_v2.get(identity)
            && *audit != v2
        {
            return Err(format!(
                "population {} has conflicting V2 audit and V3 completion facts",
                hex(identity)
            ));
        }
        let facts = blocks.get(identity);
        validate_receipt_v2_against_facts(&v2, facts)?;
        let block = facts.map_or(PopulationBlock { first: 0, count: 0 }, |held| held.block);
        committed.insert(
            *identity,
            AuthoritativePopulationV3 {
                block,
                receipt: *receipt,
            },
        );
    }
    Ok(committed)
}

fn reconcile_receipts_v4(
    blocks: &HashMap<[u8; 32], BlockFacts>,
    audit_v2: &HashMap<[u8; 32], CompletionReceiptV2>,
    audit_v3: &HashMap<[u8; 32], CompletionReceiptV3>,
    receipts: &HashMap<[u8; 32], CompletionReceiptV4>,
) -> Result<HashMap<[u8; 32], AuthoritativePopulationV4>, PopulationRefusal> {
    let mut committed = HashMap::new();
    reserve_map(
        &mut committed,
        receipts.len(),
        "reconciled authoritative V4 population index",
    )?;
    for (identity, receipt) in receipts {
        let v3 = receipt.v3();
        let v2 = v3.v2();
        let held_v2 = audit_v2.get(identity).ok_or_else(|| {
            format!(
                "population {} has a V4 completion without its V2 audit receipt",
                hex(identity)
            )
        })?;
        if *held_v2 != v2 {
            return Err(format!(
                "population {} has conflicting V2 audit and V4 completion facts",
                hex(identity)
            ));
        }
        let held_v3 = audit_v3.get(identity).ok_or_else(|| {
            format!(
                "population {} has a V4 completion without its V3 audit receipt",
                hex(identity)
            )
        })?;
        if *held_v3 != v3 {
            return Err(format!(
                "population {} has conflicting V3 audit and V4 completion facts",
                hex(identity)
            ));
        }
        let facts = blocks.get(identity);
        validate_receipt_v2_against_facts(&v2, facts)?;
        let block = facts.map_or(PopulationBlock { first: 0, count: 0 }, |held| held.block);
        committed.insert(
            *identity,
            AuthoritativePopulationV4 {
                block,
                receipt: *receipt,
            },
        );
    }
    Ok(committed)
}

fn read_row_at(
    file: &mut File,
    path: &Path,
    at: u64,
) -> Result<PopulationRowV1, PopulationRefusal> {
    let mut raw = [0_u8; ROW_STRIDE_BYTES];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            format!(
                "{} population row at byte {at} could not be read: {why}",
                path.display()
            )
        })?;
    PopulationRowV1::from_bytes(&raw).map_err(|why| {
        format!(
            "{} population row at byte {at} is invalid: {why}",
            path.display()
        )
    })
}

fn read_receipt_at(
    file: &mut File,
    path: &Path,
    at: u64,
) -> Result<CompletionReceiptV2, PopulationRefusal> {
    let mut raw = [0_u8; RECEIPT_STRIDE_BYTES];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            format!(
                "{} population receipt at byte {at} could not be read: {why}",
                path.display()
            )
        })?;
    CompletionReceiptV2::from_bytes(&raw).map_err(|why| {
        format!(
            "{} population receipt at byte {at} is invalid: {why}",
            path.display()
        )
    })
}

fn read_receipt_v3_at(
    file: &mut File,
    path: &Path,
    at: u64,
) -> Result<CompletionReceiptV3, PopulationRefusal> {
    let mut raw = [0_u8; RECEIPT_V3_STRIDE_BYTES];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            format!(
                "{} population V3 receipt at byte {at} could not be read: {why}",
                path.display()
            )
        })?;
    CompletionReceiptV3::from_bytes(&raw).map_err(|why| {
        format!(
            "{} population V3 receipt at byte {at} is invalid: {why}",
            path.display()
        )
    })
}

fn read_receipt_v4_at(
    file: &mut File,
    path: &Path,
    at: u64,
) -> Result<CompletionReceiptV4, PopulationRefusal> {
    let mut raw = [0_u8; RECEIPT_V4_STRIDE_BYTES];
    file.seek(SeekFrom::Start(at))
        .and_then(|_| file.read_exact(&mut raw))
        .map_err(|why| {
            format!(
                "{} population V4 receipt at byte {at} could not be read: {why}",
                path.display()
            )
        })?;
    CompletionReceiptV4::from_bytes(&raw).map_err(|why| {
        format!(
            "{} population V4 receipt at byte {at} is invalid: {why}",
            path.display()
        )
    })
}

fn open_or_create(path: &Path) -> Result<File, PopulationRefusal> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|why| format!("{} could not be opened: {why}", path.display()))
}

fn ensure_header(
    file: &mut File,
    path: &Path,
    magic: [u8; 8],
    version: u32,
) -> Result<(), PopulationRefusal> {
    let len = measured_len(file, path)?;
    if len != 0 {
        return Ok(());
    }
    let mut header = [0_u8; HEADER_BYTES];
    let mut encoder = Encoder::new(&mut header);
    encoder.bytes(&magic)?;
    encoder.u32(version)?;
    encoder.zeros(4)?;
    encoder.finish()?;
    file.write_all(&header)
        .and_then(|()| file.sync_all())
        .map_err(|why| {
            format!(
                "{} could not receive a durable header: {why}",
                path.display()
            )
        })
}

fn check_header(
    file: &mut File,
    path: &Path,
    len: u64,
    magic: [u8; 8],
    expected_version: u32,
    stride: u64,
) -> Result<(), PopulationRefusal> {
    if len < HEADER {
        return Err(format!(
            "{} has length {len}, shorter than its {HEADER}-byte header",
            path.display()
        ));
    }
    let mut header = [0_u8; HEADER_BYTES];
    file.seek(SeekFrom::Start(0))
        .and_then(|_| file.read_exact(&mut header))
        .map_err(|why| format!("{} header could not be read: {why}", path.display()))?;
    if header.get(..8) != Some(&magic) {
        return Err(format!(
            "{} has the wrong population-file magic; no record was trusted",
            path.display()
        ));
    }
    let version = header
        .get(8..12)
        .and_then(|bytes| <[u8; 4]>::try_from(bytes).ok())
        .map_or(0, u32::from_le_bytes);
    if version != expected_version {
        return Err(format!(
            "{} is population format version {version}; this file requires version {expected_version} and will not guess a stride",
            path.display()
        ));
    }
    if header.get(12..16) != Some(&[0_u8; 4]) {
        return Err(format!(
            "{} has non-zero reserved header bytes; their meaning is unknown",
            path.display()
        ));
    }
    check_ragged(path, len, stride, "records")
}

fn check_ragged(
    path: &Path,
    len: u64,
    stride: u64,
    subject: &str,
) -> Result<(), PopulationRefusal> {
    if len < HEADER || !len.saturating_sub(HEADER).is_multiple_of(stride) {
        return Err(format!(
            "{} has length {len}, which is not a {HEADER}-byte header plus whole {stride}-byte {subject}; a torn/ragged tail is never ignored or padded",
            path.display()
        ));
    }
    Ok(())
}

fn measured_len(file: &File, path: &Path) -> Result<u64, PopulationRefusal> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|why| format!("{} could not be measured: {why}", path.display()))
}

fn validated_generation(
    file: &File,
    path: &Path,
    expected_len: u64,
    kind: &str,
) -> Result<FileGeneration, PopulationRefusal> {
    let generation = file_generation(file, path)?;
    if generation.len != expected_len {
        return Err(format!(
            "{} {kind} changed length from the just-validated byte {expected_len} to {} before its filesystem generation could be retained; nothing further was appended",
            path.display(),
            generation.len
        ));
    }
    Ok(generation)
}

fn file_generation(file: &File, path: &Path) -> Result<FileGeneration, PopulationRefusal> {
    let held = file
        .metadata()
        .map_err(|why| format!("{} open file could not be measured: {why}", path.display()))?;
    let named = std::fs::metadata(path)
        .map_err(|why| format!("{} path could not be measured: {why}", path.display()))?;
    platform_generation(&held, &named, path)
}

#[cfg(unix)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, PopulationRefusal> {
    let held = FileGeneration {
        len: held.len(),
        platform: PlatformGeneration {
            device: held.dev(),
            inode: held.ino(),
            modified_seconds: held.mtime(),
            modified_nanoseconds: held.mtime_nsec(),
            changed_seconds: held.ctime(),
            changed_nanoseconds: held.ctime_nsec(),
        },
    };
    let named = FileGeneration {
        len: named.len(),
        platform: PlatformGeneration {
            device: named.dev(),
            inode: named.ino(),
            modified_seconds: named.mtime(),
            modified_nanoseconds: named.mtime_nsec(),
            changed_seconds: named.ctime(),
            changed_nanoseconds: named.ctime_nsec(),
        },
    };
    if (held.platform.device, held.platform.inode) != (named.platform.device, named.platform.inode)
    {
        return Err(format!(
            "{} no longer names the opened population file; a replacement or path swap was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was being measured; cached population history was not reused",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(windows)]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, PopulationRefusal> {
    fn of(metadata: &std::fs::Metadata, path: &Path) -> Result<FileGeneration, PopulationRefusal> {
        let volume_serial = metadata.volume_serial_number().ok_or_else(|| {
            format!(
                "{} has no Windows volume serial; same-length stale detection fails closed",
                path.display()
            )
        })?;
        let file_index = metadata.file_index().ok_or_else(|| {
            format!(
                "{} has no Windows file index; same-length stale detection fails closed",
                path.display()
            )
        })?;
        Ok(FileGeneration {
            len: metadata.len(),
            platform: PlatformGeneration {
                volume_serial,
                file_index,
                creation_time: metadata.creation_time(),
                last_write_time: metadata.last_write_time(),
            },
        })
    }

    let held = of(held, path)?;
    let named = of(named, path)?;
    if (held.platform.volume_serial, held.platform.file_index)
        != (named.platform.volume_serial, named.platform.file_index)
    {
        return Err(format!(
            "{} no longer names the opened population file; a replacement or path swap was refused",
            path.display()
        ));
    }
    if held != named {
        return Err(format!(
            "{} changed while its filesystem generation was being measured; cached population history was not reused",
            path.display()
        ));
    }
    Ok(held)
}

#[cfg(not(any(unix, windows)))]
fn platform_generation(
    held: &std::fs::Metadata,
    named: &std::fs::Metadata,
    path: &Path,
) -> Result<FileGeneration, PopulationRefusal> {
    if held.len() != named.len() {
        return Err(format!(
            "{} changed length while it was being measured; cached population history was not reused",
            path.display()
        ));
    }
    Ok(FileGeneration {
        len: held.len(),
        platform: PlatformGeneration,
    })
}

#[cfg(any(unix, windows))]
fn require_generation_unchanged(
    expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
    kind: &str,
) -> Result<(), PopulationRefusal> {
    if expected == observed {
        return Ok(());
    }
    Err(format!(
        "{} {kind} kept length {} but its validated filesystem generation changed; cached population history was not reused and nothing was appended. Reopen to validate the complete file",
        path.display(),
        observed.len
    ))
}

#[cfg(not(any(unix, windows)))]
fn require_generation_unchanged(
    _expected: FileGeneration,
    observed: FileGeneration,
    path: &Path,
    kind: &str,
) -> Result<(), PopulationRefusal> {
    Err(format!(
        "{} {kind} is unchanged at {} bytes, but this target exposes no stable file identity; append fails closed rather than trusting a possibly replaced same-length file",
        path.display(),
        observed.len
    ))
}

fn require_optional_generation_unchanged(
    file: Option<&File>,
    expected: Option<FileGeneration>,
    path: &Path,
    kind: &str,
) -> Result<(), PopulationRefusal> {
    match (file, expected) {
        (Some(file), Some(expected)) => {
            require_generation_unchanged(expected, file_generation(file, path)?, path, kind)
        }
        (None, None) => match std::fs::metadata(path) {
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Ok(_) => Err(format!(
                "{} {kind} appeared after this population handle was indexed; reopen before serving V4 authority",
                path.display()
            )),
            Err(why) => Err(format!(
                "{} absent {kind} path could not be rechecked: {why}",
                path.display()
            )),
        },
        (Some(_), None) => Err(format!(
            "{} opened {kind} has no retained filesystem generation",
            path.display()
        )),
        (None, Some(_)) => Err(format!(
            "{} absent {kind} unexpectedly retained a filesystem generation",
            path.display()
        )),
    }
}

fn sync_directory(path: &Path) -> Result<(), PopulationRefusal> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|why| {
            format!(
                "{} could not durably confirm the population file names: {why}",
                path.display()
            )
        })
}

fn rollback_message(file: &File, at: u64, subject: &str, why: &std::io::Error) -> String {
    match file.set_len(at) {
        Ok(()) => format!(
            "the {subject} could not be written: {why}. Partial bytes were rolled back to byte {at}"
        ),
        Err(and) => format!(
            "the {subject} could not be written: {why}. Rolling partial bytes back to byte {at} also failed: {and}; the tail is refused until forensic repair"
        ),
    }
}

fn rollback_population_message(file: &File, at: u64, subject: &str, why: &str) -> String {
    match file.set_len(at) {
        Ok(()) => {
            format!(
                "the {subject} could not be encoded: {why}. Partial bytes were rolled back to byte {at}"
            )
        }
        Err(and) => format!(
            "the {subject} could not be encoded: {why}. Rolling partial bytes back to byte {at} also failed: {and}; the tail is refused until forensic repair"
        ),
    }
}

fn validate_rung(seconds: u32) -> Result<(), PopulationRefusal> {
    if [60, 120, 180, 300, 600, 900, 1_800, 3_600].contains(&seconds) {
        Ok(())
    } else {
        Err(format!(
            "signal rung {seconds} seconds is not one of the eight version-one intraday sweep rungs"
        ))
    }
}

fn require_nonzero_digest(name: &str, digest: &[u8; 32]) -> Result<(), PopulationRefusal> {
    if digest.iter().all(|byte| *byte == 0) {
        Err(format!(
            "{name} is absent (all zero); no default identity exists"
        ))
    } else {
        Ok(())
    }
}

fn checked_sum(values: &[u64], subject: &str) -> Result<u64, PopulationRefusal> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or_else(|| format!("{subject} overflow u64"))
    })
}

fn reserve_map<K: Eq + Hash, V>(
    map: &mut HashMap<K, V>,
    additional: usize,
    subject: &str,
) -> Result<(), PopulationRefusal> {
    map.try_reserve(additional)
        .map_err(|why| format!("{subject} could not reserve {additional} more slot(s): {why}"))
}

fn canonical_rate_ppm(part: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    let projected = u128::from(part).saturating_mul(1_000_000) / u128::from(total);
    u64::try_from(projected).unwrap_or(u64::MAX)
}

fn empty_row_digest() -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(EMPTY_ROW_DIGEST_DOMAIN);
    hasher.finalize()
}

fn seal(payload: &[u8]) -> [u8; SEAL_BYTES] {
    let digest = brutex_core::blake3::hash(payload);
    let mut out = [0_u8; SEAL_BYTES];
    if let Some(prefix) = digest.get(..SEAL_BYTES) {
        out.copy_from_slice(prefix);
    }
    out
}

fn encode_index(encoder: &mut Encoder<'_>, value: Option<u32>) -> Result<(), PopulationRefusal> {
    encoder.u32(value.unwrap_or(u32::MAX))
}

fn decode_index(decoder: &mut Decoder<'_>) -> Result<Option<u32>, PopulationRefusal> {
    let value = decoder.u32()?;
    Ok((value != u32::MAX).then_some(value))
}

fn encode_optional_u64(
    encoder: &mut Encoder<'_>,
    value: Option<u64>,
) -> Result<(), PopulationRefusal> {
    if let Some(value) = value {
        encoder.u8(1)?;
        encoder.zeros(7)?;
        encoder.u64(value)
    } else {
        encoder.u8(0)?;
        encoder.zeros(7)?;
        encoder.u64(0)
    }
}

fn decode_optional_u64(
    decoder: &mut Decoder<'_>,
    name: &str,
) -> Result<Option<u64>, PopulationRefusal> {
    let tag = decoder.u8()?;
    decoder.zeros(7, &format!("{name} option reserve"))?;
    let value = decoder.u64()?;
    match (tag, value) {
        (0, 0) => Ok(None),
        (0, _) => Err(format!(
            "{name} has absent tag 0 but a non-zero payload {value}"
        )),
        (1, value) => Ok(Some(value)),
        _ => Err(format!(
            "{name} option tag {tag} is unknown; version one defines only 0=absent and 1=present"
        )),
    }
}

fn decode_bool(byte: u8, name: &str) -> Result<bool, PopulationRefusal> {
    match byte {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(format!(
            "{name} byte {byte} is unknown; version one defines only 0=false and 1=true"
        )),
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

struct Encoder<'a> {
    raw: &'a mut [u8],
    at: usize,
}

impl<'a> Encoder<'a> {
    const fn new(raw: &'a mut [u8]) -> Self {
        Self { raw, at: 0 }
    }

    fn bytes(&mut self, bytes: &[u8]) -> Result<(), PopulationRefusal> {
        let raw_len = self.raw.len();
        let end = self
            .at
            .checked_add(bytes.len())
            .ok_or_else(|| "fixed encoder offset overflowed usize".to_owned())?;
        let slot = self.raw.get_mut(self.at..end).ok_or_else(|| {
            format!(
                "fixed encoder needed bytes {}..{end} in a {}-byte record",
                self.at, raw_len
            )
        })?;
        slot.copy_from_slice(bytes);
        self.at = end;
        Ok(())
    }

    fn zeros(&mut self, count: usize) -> Result<(), PopulationRefusal> {
        let zeros = [0_u8; 8];
        let mut remaining = count;
        while remaining > 0 {
            let take = remaining.min(zeros.len());
            self.bytes(zeros.get(..take).unwrap_or(&[]))?;
            remaining = remaining.saturating_sub(take);
        }
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), PopulationRefusal> {
        self.bytes(&[value])
    }

    fn u32(&mut self, value: u32) -> Result<(), PopulationRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), PopulationRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), PopulationRefusal> {
        self.bytes(&value.to_le_bytes())
    }

    fn finish(self) -> Result<(), PopulationRefusal> {
        if self.at == self.raw.len() {
            Ok(())
        } else {
            Err(format!(
                "fixed encoder filled {} of {} bytes",
                self.at,
                self.raw.len()
            ))
        }
    }
}

struct Decoder<'a> {
    raw: &'a [u8],
    at: usize,
}

impl<'a> Decoder<'a> {
    const fn new(raw: &'a [u8]) -> Self {
        Self { raw, at: 0 }
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], PopulationRefusal> {
        let end = self
            .at
            .checked_add(N)
            .ok_or_else(|| "fixed decoder offset overflowed usize".to_owned())?;
        let source = self.raw.get(self.at..end).ok_or_else(|| {
            format!(
                "fixed decoder needed bytes {}..{end} in a {}-byte record",
                self.at,
                self.raw.len()
            )
        })?;
        let mut out = [0_u8; N];
        out.copy_from_slice(source);
        self.at = end;
        Ok(out)
    }

    fn zeros(&mut self, count: usize, subject: &str) -> Result<(), PopulationRefusal> {
        let end = self
            .at
            .checked_add(count)
            .ok_or_else(|| "fixed reserve decoder offset overflowed usize".to_owned())?;
        let bytes = self.raw.get(self.at..end).ok_or_else(|| {
            format!(
                "fixed decoder needed reserve bytes {}..{end} in a {}-byte record",
                self.at,
                self.raw.len()
            )
        })?;
        if bytes.iter().any(|byte| *byte != 0) {
            return Err(format!(
                "{subject} contains non-zero reserved bytes; this build will not invent their meaning"
            ));
        }
        self.at = end;
        Ok(())
    }

    fn u8(&mut self) -> Result<u8, PopulationRefusal> {
        Ok(self.bytes::<1>()?.first().copied().unwrap_or(0))
    }

    fn u32(&mut self) -> Result<u32, PopulationRefusal> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }

    fn u64(&mut self) -> Result<u64, PopulationRefusal> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }

    fn i64(&mut self) -> Result<i64, PopulationRefusal> {
        Ok(i64::from_le_bytes(self.bytes()?))
    }

    fn array_32(&mut self) -> Result<[u8; 32], PopulationRefusal> {
        self.bytes()
    }

    fn finish(self) -> Result<(), PopulationRefusal> {
        if self.at == self.raw.len() {
            Ok(())
        } else {
            Err(format!(
                "fixed decoder consumed {} of {} bytes",
                self.at,
                self.raw.len()
            ))
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::panic,
    reason = "fixtures must fail loudly and mutate exact positions in fixed canonical records"
)]
mod tests {
    use super::{
        AdmissionStatusV1, AdmissionV1, CompletionReceiptV1, CompletionReceiptV2,
        CompletionReceiptV3, CompletionReceiptV4, CompletionReconciliationV1,
        CompletionReconciliationV2, ExitCellsPerMaskV2, ExitCoordinateV1, InstrumentFamilyV1,
        LongShortExitGridIdentitiesV2, PopulationCommit, PopulationIdentitiesV1,
        PopulationIdentitiesV2, PopulationLedger, PopulationRowV1, RequestedSpanIdentityV1,
        SideExitGridIdentityV2, TopMetricsV1, TradeDirectionV1,
    };
    use crate::stored::{CompleteCalendarReceiptV2, calendar_receipt_v2};
    use std::fs::OpenOptions;
    use std::io::{Seek as _, SeekFrom, Write as _};

    fn root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "brutex-population-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ))
    }

    const fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    const fn legacy_identities(seed: u8) -> PopulationIdentitiesV1 {
        PopulationIdentitiesV1 {
            run_identity: digest(seed),
            data_digest: digest(seed.wrapping_add(1)),
            feed_digest: digest(seed.wrapping_add(2)),
            source_commit_digest: digest(seed.wrapping_add(3)),
            vocabulary_digest: digest(seed.wrapping_add(4)),
            evaluation_policy_digest: digest(seed.wrapping_add(5)),
            exit_grid_policy_digest: digest(seed.wrapping_add(6)),
            resolved_exit_grid_digest: digest(seed.wrapping_add(7)),
            admission_policy_digest: digest(seed.wrapping_add(8)),
            ranking_policy_digest: digest(seed.wrapping_add(9)),
            calendar_policy_digest: digest(seed.wrapping_add(10)),
            daily_reference_policy_digest: digest(seed.wrapping_add(11)),
        }
    }

    const fn identities(seed: u8) -> PopulationIdentitiesV2 {
        PopulationIdentitiesV2 {
            run_identity: digest(seed),
            data_digest: digest(seed.wrapping_add(1)),
            feed_digest: digest(seed.wrapping_add(2)),
            source_commit_digest: digest(seed.wrapping_add(3)),
            vocabulary_digest: digest(seed.wrapping_add(4)),
            evaluation_policy_digest: digest(seed.wrapping_add(5)),
            exit_grids: LongShortExitGridIdentitiesV2 {
                long: SideExitGridIdentityV2 {
                    policy_digest: digest(seed.wrapping_add(6)),
                    resolved_digest: digest(seed.wrapping_add(7)),
                },
                short: SideExitGridIdentityV2 {
                    policy_digest: digest(seed.wrapping_add(8)),
                    resolved_digest: digest(seed.wrapping_add(9)),
                },
            },
            admission_policy_digest: digest(seed.wrapping_add(10)),
            ranking_policy_digest: digest(seed.wrapping_add(11)),
            calendar_policy_digest: digest(seed.wrapping_add(12)),
            daily_reference_policy_digest: digest(seed.wrapping_add(13)),
        }
    }

    const fn admitted() -> AdmissionV1 {
        AdmissionV1 {
            status: AdmissionStatusV1::Admitted,
            reasons: 0,
            failed: 0,
            unmeasured: 0,
            refused: 0,
        }
    }

    const fn rejected(bit: u64) -> AdmissionV1 {
        AdmissionV1 {
            status: AdmissionStatusV1::Rejected,
            reasons: bit,
            failed: bit,
            unmeasured: 0,
            refused: 0,
        }
    }

    fn row(population_id: [u8; 32], sequence: u64, direction: TradeDirectionV1) -> PopulationRowV1 {
        PopulationRowV1 {
            population_id,
            sequence,
            strategy_digest: digest(
                u8::try_from(sequence)
                    .unwrap_or(0)
                    .wrapping_add(80)
                    .wrapping_add(direction.byte()),
            ),
            mask_words: [7, 0, 0, 0, 0, 0],
            direction,
            instrument_family: InstrumentFamilyV1::Nifty,
            closure: super::ClosureV1::Closed,
            rung_seconds: 300,
            support_hits: 41,
            exit: ExitCoordinateV1 {
                stop: Some(u32::try_from(sequence).unwrap_or(0)),
                target: Some(2),
                tsl: None,
                ttp: Some((1, 1)),
            },
            metrics: TopMetricsV1 {
                drawdown: 1_000 + sequence,
                worst_loss: 500,
                losing_rate_ppm: 250_000,
                losing_trades: 1,
                loss_ratio_ppm: Some(100_000),
                pessimistic_profit: 8_000,
                winning_trades: 3,
                win_rate_ppm: 750_000,
                reward_to_risk_ppm: Some(2_000_000),
                average_win: 3_000,
                average_loss: 1_000,
                assurance_ppm: 700_000,
            },
            admission: if sequence == 0 {
                admitted()
            } else {
                rejected(1)
            },
        }
    }

    fn two_rows(population_id: [u8; 32]) -> [PopulationRowV1; 2] {
        [
            row(population_id, 0, TradeDirectionV1::Long),
            row(population_id, 1, TradeDirectionV1::Short),
        ]
    }

    const fn legacy_reconciliation(rows: u64) -> CompletionReconciliationV1 {
        CompletionReconciliationV1 {
            sweep_trials: 3,
            frequent_itemsets: 1,
            infrequent_itemsets: 2,
            closed_itemsets: 1,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            direction_members_expected: 2,
            exit_cells_expected: rows,
            extinction_depth: 2,
            extinction_complete: true,
            closure_complete: true,
        }
    }

    fn reconciliation(rows: &[PopulationRowV1]) -> CompletionReconciliationV2 {
        let long = u64::try_from(
            rows.iter()
                .filter(|row| row.direction == TradeDirectionV1::Long)
                .count(),
        )
        .expect("long fixture count");
        let short = u64::try_from(
            rows.iter()
                .filter(|row| row.direction == TradeDirectionV1::Short)
                .count(),
        )
        .expect("short fixture count");
        CompletionReconciliationV2 {
            sweep_trials: 3,
            frequent_itemsets: 1,
            infrequent_itemsets: 2,
            closed_itemsets: 1,
            redundant_itemsets: 0,
            unknown_closure_itemsets: 0,
            exit_cells_per_mask: ExitCellsPerMaskV2::new(long, short).expect("both fixture sides"),
            extinction_depth: 2,
            extinction_complete: true,
            closure_complete: true,
        }
    }

    fn receipt(population_id: [u8; 32], rows: &[PopulationRowV1]) -> CompletionReceiptV2 {
        CompletionReceiptV2::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            rows,
            reconciliation(rows),
            identities(10),
        )
        .expect("canonical receipt")
    }

    fn zero_receipt(population_id: [u8; 32]) -> CompletionReceiptV2 {
        CompletionReceiptV2::for_rows(
            population_id,
            InstrumentFamilyV1::BankNifty,
            60,
            &[],
            CompletionReconciliationV2 {
                sweep_trials: 6,
                frequent_itemsets: 0,
                infrequent_itemsets: 6,
                closed_itemsets: 0,
                redundant_itemsets: 0,
                unknown_closure_itemsets: 0,
                exit_cells_per_mask: ExitCellsPerMaskV2::new(1, 1).expect("nonempty grids"),
                extinction_depth: 1,
                extinction_complete: true,
                closure_complete: true,
            },
            identities(30),
        )
        .expect("zero-row receipt")
    }

    fn span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2019, 1, 2026, 8).expect("canonical requested span")
    }

    fn receipt_v3(
        population_id: [u8; 32],
        rows: &[PopulationRowV1],
        requested_span: RequestedSpanIdentityV1,
    ) -> CompletionReceiptV3 {
        CompletionReceiptV3::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            rows,
            requested_span,
            reconciliation(rows),
            identities(10),
        )
        .expect("canonical V3 receipt")
    }

    fn month_span() -> RequestedSpanIdentityV1 {
        RequestedSpanIdentityV1::new(2026, 7, 2026, 7).expect("measured full-month span")
    }

    fn complete_calendar(
        requested_span: RequestedSpanIdentityV1,
        rung_seconds: u32,
    ) -> CompleteCalendarReceiptV2 {
        const MICROS_PER_MINUTE: i64 = 60_000_000;
        const MICROS_PER_DAY: i64 = 86_400_000_000;
        let (first_day, last_day) =
            super::requested_span_day_bounds(requested_span).expect("civil day bounds");
        let width = i64::from(rung_seconds / 60);
        let last_bucket = (929_i64 - 555) / width;
        let mut timestamps = Vec::new();
        for day in first_day..=last_day {
            match pull::calendar::kind_of(day) {
                pull::calendar::DayKind::Open(session) => {
                    for bucket in 0..=last_bucket {
                        let start = 555_i64 + bucket * width;
                        let end = start + width - 1;
                        let intersects = session
                            .windows
                            .iter()
                            .take(usize::from(session.count))
                            .any(|window| {
                                start <= i64::from(window.to) && end >= i64::from(window.from)
                            });
                        if intersects {
                            let timestamp = day * MICROS_PER_DAY + start * MICROS_PER_MINUTE
                                - indicators::IST_OFFSET_MICROS;
                            timestamps.push(timestamp);
                        }
                    }
                }
                pull::calendar::DayKind::Closed => {}
                other => panic!("fixture month must be fully measured, found {other:?} on {day}"),
            }
        }
        calendar_receipt_v2(&timestamps, rung_seconds, first_day, last_day)
            .expect("calendar receipt")
            .require_complete()
            .expect("complete measured month")
    }

    fn receipt_v4(population_id: [u8; 32], rows: &[PopulationRowV1]) -> CompletionReceiptV4 {
        let requested_span = month_span();
        CompletionReceiptV4::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            rows,
            requested_span,
            reconciliation(rows),
            identities(10),
            complete_calendar(requested_span, 300),
            complete_calendar(requested_span, 60),
        )
        .expect("canonical V4 receipt")
    }

    fn reseal_row(raw: &mut [u8]) {
        let payload = raw
            .get(..super::ROW_PAYLOAD_BYTES)
            .expect("row payload")
            .to_vec();
        raw.get_mut(super::ROW_PAYLOAD_BYTES..)
            .expect("row seal")
            .copy_from_slice(&super::seal(&payload));
    }

    fn reseal_receipt(raw: &mut [u8]) {
        let payload = raw
            .get(..super::RECEIPT_PAYLOAD_BYTES)
            .expect("receipt payload")
            .to_vec();
        raw.get_mut(super::RECEIPT_PAYLOAD_BYTES..)
            .expect("receipt seal")
            .copy_from_slice(&super::seal(&payload));
    }

    fn reseal_receipt_v3(raw: &mut [u8]) {
        let payload = raw
            .get(..super::RECEIPT_V3_PAYLOAD_BYTES)
            .expect("V3 receipt payload")
            .to_vec();
        raw.get_mut(super::RECEIPT_V3_PAYLOAD_BYTES..)
            .expect("V3 receipt seal")
            .copy_from_slice(&super::seal(&payload));
    }

    fn reseal_receipt_v4(raw: &mut [u8]) {
        let payload = raw
            .get(..super::RECEIPT_V4_PAYLOAD_BYTES)
            .expect("V4 receipt payload")
            .to_vec();
        raw.get_mut(super::RECEIPT_V4_PAYLOAD_BYTES..)
            .expect("V4 receipt seal")
            .copy_from_slice(&super::seal(&payload));
    }

    fn rewrite(path: &std::path::Path, at: u64, raw: &[u8]) {
        let mut file = OpenOptions::new()
            .write(true)
            .open(path)
            .expect("fixture file");
        file.seek(SeekFrom::Start(at)).expect("fixture seek");
        file.write_all(raw).expect("fixture rewrite");
        file.sync_all().expect("fixture sync");
    }

    #[test]
    fn row_payload_digest_is_deterministic_domain_separated_and_load_bearing() {
        let original = row(digest(90), 0, TradeDirectionV1::Long);
        let payload = original.payload_bytes().expect("canonical row payload");
        let mut expected = brutex_core::blake3::Hasher::new();
        expected.update(super::ROW_PAYLOAD_DIGEST_DOMAIN);
        expected.update(&payload);
        assert_eq!(
            original.payload_digest().expect("row digest"),
            expected.finalize()
        );
        assert_eq!(
            original.payload_digest().expect("first digest"),
            original.payload_digest().expect("identical digest")
        );

        let mut changed = original;
        changed.support_hits = changed.support_hits.saturating_add(1);
        assert_ne!(
            original.payload_digest().expect("original digest"),
            changed.payload_digest().expect("changed digest"),
            "a semantic row change must alter the admission-sidecar binding"
        );
    }

    #[test]
    fn fixed_row_and_receipt_codecs_round_trip_every_optional_domain() {
        let population_id = digest(1);
        let mut first = row(population_id, 0, TradeDirectionV1::Long);
        first.metrics.loss_ratio_ppm = None;
        first.metrics.reward_to_risk_ppm = None;
        first.exit = ExitCoordinateV1 {
            stop: None,
            target: Some(0),
            tsl: None,
            ttp: None,
        };
        let raw = first.to_bytes().expect("row bytes");
        assert_eq!(raw.len(), super::ROW_STRIDE_BYTES);
        assert_eq!(PopulationRowV1::from_bytes(&raw).expect("row"), first);

        let rows = two_rows(population_id);
        let legacy = CompletionReceiptV1::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            &rows,
            legacy_reconciliation(2),
            legacy_identities(10),
        )
        .expect("legacy canonical receipt");
        let raw = legacy.to_bytes().expect("legacy receipt bytes");
        assert_eq!(raw.len(), super::RECEIPT_V1_STRIDE_BYTES);
        assert_eq!(
            CompletionReceiptV1::from_bytes(&raw).expect("legacy receipt"),
            legacy
        );

        let receipt = receipt(population_id, &rows);
        let raw = receipt.to_bytes().expect("V2 receipt bytes");
        assert_eq!(raw.len(), super::RECEIPT_STRIDE_BYTES);
        assert_eq!(
            CompletionReceiptV2::from_bytes(&raw).expect("V2 receipt"),
            receipt
        );
        let receipt_v3 =
            CompletionReceiptV3::from_v2(receipt, span()).expect("V3 receipt construction");
        let raw = receipt_v3.to_bytes().expect("V3 receipt bytes");
        assert_eq!(raw.len(), super::RECEIPT_V3_STRIDE_BYTES);
        assert_eq!(
            CompletionReceiptV3::from_bytes(&raw).expect("V3 receipt"),
            receipt_v3
        );
        let receipt_v4 = receipt_v4(population_id, &rows);
        let raw = receipt_v4.to_bytes().expect("V4 receipt bytes");
        assert_eq!(raw.len(), super::RECEIPT_V4_STRIDE_BYTES);
        assert_eq!(
            CompletionReceiptV4::from_bytes(&raw).expect("V4 receipt"),
            receipt_v4
        );
        assert_eq!(receipt_v4.v3().payload_bytes().unwrap(), raw[..760]);
        assert_ne!(
            PopulationLedger::receipt_path(std::path::Path::new("root")),
            PopulationLedger::legacy_receipt_path_v1(std::path::Path::new("root")),
            "V1 and V2 completion files must never alias"
        );
        assert_ne!(
            PopulationLedger::receipt_v3_path(std::path::Path::new("root")),
            PopulationLedger::receipt_path(std::path::Path::new("root")),
            "V2 audit and V3 authoritative files must never alias"
        );
        assert_ne!(
            PopulationLedger::receipt_v4_path(std::path::Path::new("root")),
            PopulationLedger::receipt_v3_path(std::path::Path::new("root")),
            "V3 audit and V4 authoritative files must never alias"
        );
    }

    #[test]
    fn durable_rows_refuse_sealed_tombstoned_void_and_unallocated_masks() {
        let base = row(digest(119), 0, TradeDirectionV1::Long);
        for (bit, needle) in [
            (6_u32, "tombstoned bit 6"),
            (237, "void bit 237"),
            (380, "unallocated bit 380"),
        ] {
            let mut offered = base;
            offered.mask_words = [0; 6];
            offered.mask_words[usize::try_from(bit / 64).expect("word")] = 1_u64 << (bit % 64);
            let why = offered
                .to_bytes()
                .expect_err("invalid live mask cannot encode");
            assert!(why.contains(needle), "{why}");

            let mut raw = base.to_bytes().expect("canonical row");
            let mask_at = 32 + 8 + 32;
            raw[mask_at..mask_at + 48].fill(0);
            let word_at = mask_at + usize::try_from(bit / 64).expect("word") * 8;
            raw[word_at..word_at + 8].copy_from_slice(&(1_u64 << (bit % 64)).to_le_bytes());
            reseal_row(&mut raw);
            let why = PopulationRowV1::from_bytes(&raw)
                .expect_err("a valid seal cannot authorize an invalid vocabulary bit");
            assert!(why.contains(needle), "{why}");
        }
    }

    #[test]
    fn requested_span_is_checked_canonical_and_every_endpoint_is_load_bearing() {
        for (from_year, from_month, to_year, to_month, needle) in [
            (1969, 1, 2024, 10, "from_year"),
            (2020, 1, 10_000, 10, "to_year"),
            (2020, 0, 2024, 10, "from_month"),
            (2020, 13, 2024, 10, "from_month"),
            (2020, 2, 2024, 0, "to_month"),
            (2020, 2, 2024, 13, "to_month"),
            (2024, 11, 2024, 10, "steps backwards"),
        ] {
            assert!(
                RequestedSpanIdentityV1::new(from_year, from_month, to_year, to_month)
                    .expect_err("invalid span")
                    .contains(needle)
            );
        }

        let base = RequestedSpanIdentityV1::new(2020, 2, 2024, 10).expect("base span");
        assert_eq!(base.from_year(), 2020);
        assert_eq!(base.from_month(), 2);
        assert_eq!(base.to_year(), 2024);
        assert_eq!(base.to_month(), 10);
        let variants = [
            RequestedSpanIdentityV1::new(2019, 2, 2024, 10).expect("from year"),
            RequestedSpanIdentityV1::new(2020, 1, 2024, 10).expect("from month"),
            RequestedSpanIdentityV1::new(2020, 2, 2025, 10).expect("to year"),
            RequestedSpanIdentityV1::new(2020, 2, 2024, 11).expect("to month"),
        ];
        let population_id = digest(122);
        let rows = two_rows(population_id);
        let base_receipt = receipt_v3(population_id, &rows, base);
        for variant in variants {
            assert_ne!(variant.canonical_bytes(), base.canonical_bytes());
            assert_ne!(variant.digest(), base.digest());
            assert_ne!(
                receipt_v3(population_id, &rows, variant)
                    .content_digest()
                    .expect("variant content digest"),
                base_receipt.content_digest().expect("base content digest")
            );
        }
    }

    #[test]
    fn nifty_and_banknifty_expose_only_an_exact_shared_requested_span() {
        let shared = span();
        let nifty_id = digest(123);
        let nifty_rows = two_rows(nifty_id);
        let nifty = receipt_v3(nifty_id, &nifty_rows, shared);
        let bank = CompletionReceiptV3::from_v2(zero_receipt(digest(124)), shared)
            .expect("BANKNIFTY V3 receipt");
        let shared_term = nifty
            .requested_span()
            .require_same(bank.requested_span())
            .expect("same inclusive request");
        assert_eq!(shared_term, shared);
        assert_eq!(shared_term.digest(), nifty.requested_span().digest());
        assert_eq!(shared_term.digest(), bank.requested_span().digest());

        let different = CompletionReceiptV3::from_v2(
            zero_receipt(digest(125)),
            RequestedSpanIdentityV1::new(2019, 1, 2026, 7).expect("different end month"),
        )
        .expect("different BANKNIFTY V3 receipt");
        assert!(
            nifty
                .requested_span()
                .require_same(different.requested_span())
                .expect_err("mismatched span must refuse downstream derivation")
                .contains("spans differ")
        );
    }

    #[test]
    fn v3_span_digest_reserve_and_seal_mutations_refuse() {
        let population_id = digest(126);
        let rows = two_rows(population_id);
        let receipt = receipt_v3(population_id, &rows, span());
        let canonical = receipt.to_bytes().expect("canonical V3 receipt bytes");

        let mut broken_seal = canonical;
        broken_seal[super::RECEIPT_V3_PAYLOAD_BYTES] ^= 1;
        assert!(
            CompletionReceiptV3::from_bytes(&broken_seal)
                .expect_err("seal mutation")
                .contains("BLAKE3 seal")
        );

        let mut corrupted_payload = canonical;
        corrupted_payload[0] ^= 1;
        assert!(
            CompletionReceiptV3::from_bytes(&corrupted_payload)
                .expect_err("unsealed payload mutation")
                .contains("BLAKE3 seal")
        );

        let span_digest_at = super::RECEIPT_PAYLOAD_BYTES + super::REQUESTED_SPAN_IDENTITY_BYTES;
        let mut different_span_digest = canonical;
        different_span_digest[span_digest_at] ^= 1;
        reseal_receipt_v3(&mut different_span_digest);
        assert!(
            CompletionReceiptV3::from_bytes(&different_span_digest)
                .expect_err("span digest mutation")
                .contains("disagree")
        );

        let reserve_at = span_digest_at + 32;
        let mut reserve = canonical;
        reserve[reserve_at] = 1;
        reseal_receipt_v3(&mut reserve);
        assert!(
            CompletionReceiptV3::from_bytes(&reserve)
                .expect_err("V3 reserve mutation")
                .contains("reserved")
        );
    }

    #[test]
    fn v4_requires_typed_full_month_signal_and_one_minute_coverage() {
        let population_id = digest(127);
        let rows = two_rows(population_id);
        let requested_span = month_span();
        let v3 = CompletionReceiptV3::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            &rows,
            requested_span,
            reconciliation(&rows),
            identities(10),
        )
        .expect("V3 facts");
        let signal = complete_calendar(requested_span, 300);
        let execution = complete_calendar(requested_span, 60);
        let receipt = CompletionReceiptV4::from_complete_v3(&v3, signal, execution)
            .expect("typed complete coverage");
        assert_eq!(receipt.v3(), v3);
        assert_eq!(receipt.coverage().signal_rung_seconds(), 300);
        assert_eq!(receipt.coverage().execution_rung_seconds(), 60);
        assert_ne!(
            receipt.coverage().signal_complete_receipt_digest(),
            receipt.coverage().execution_complete_receipt_digest()
        );

        let why = CompletionReceiptV4::from_complete_v3(&v3, execution, signal)
            .expect_err("swapped typed receipts cannot authorize V4");
        assert!(why.contains("signal-calendar rung 60"), "{why}");
        let why = CompletionReceiptV4::from_complete_v3(
            &v3,
            complete_calendar(requested_span, 120),
            execution,
        )
        .expect_err("another signal rung cannot authorize this population");
        assert!(why.contains("does not equal population rung"), "{why}");
        let why = CompletionReceiptV4::from_complete_v3(&v3, signal, signal)
            .expect_err("coarse coverage cannot stand in for one-minute execution");
        assert!(why.contains("requires exactly 60"), "{why}");

        let june = RequestedSpanIdentityV1::new(2026, 6, 2026, 6).expect("June span");
        let why = CompletionReceiptV4::from_complete_v3(
            &v3,
            complete_calendar(june, 300),
            complete_calendar(june, 60),
        )
        .expect_err("observed endpoints cannot shrink the requested civil months");
        assert!(why.contains("full requested civil-month span"), "{why}");
        let why = CompletionReceiptV4::from_complete_v3(&v3, signal, complete_calendar(june, 60))
            .expect_err("signal and execution bounds must be exact peers");
        assert!(why.contains("covers IST days"), "{why}");
    }

    #[test]
    fn v4_coverage_codec_fails_closed_on_seal_version_rungs_bounds_digests_and_reserve() {
        let population_id = digest(128);
        let rows = two_rows(population_id);
        let receipt = receipt_v4(population_id, &rows);
        let canonical = receipt.to_bytes().expect("canonical V4 bytes");

        let mut broken_seal = canonical;
        broken_seal[super::RECEIPT_V4_PAYLOAD_BYTES] ^= 1;
        assert!(
            CompletionReceiptV4::from_bytes(&broken_seal)
                .expect_err("broken V4 seal")
                .contains("BLAKE3 seal")
        );

        let coverage_at = super::RECEIPT_V3_PAYLOAD_BYTES;
        for (offset, bytes, needle) in [
            (0_usize, 2_u32.to_le_bytes(), "version 2 is unknown"),
            (4, 120_u32.to_le_bytes(), "does not equal population rung"),
            (8, 300_u32.to_le_bytes(), "requires exactly 60"),
            (12, 1_u32.to_le_bytes(), "reserve is 1"),
        ] {
            let mut raw = canonical;
            raw[coverage_at + offset..coverage_at + offset + 4].copy_from_slice(&bytes);
            reseal_receipt_v4(&mut raw);
            let why = CompletionReceiptV4::from_bytes(&raw).expect_err("invalid coverage field");
            assert!(why.contains(needle), "{why}");
        }

        for (digest_at, field) in [
            (coverage_at + 32, "signal_complete_receipt_digest"),
            (coverage_at + 64, "execution_complete_receipt_digest"),
        ] {
            let mut raw = canonical;
            raw[digest_at..digest_at + 32].fill(0);
            reseal_receipt_v4(&mut raw);
            let why = CompletionReceiptV4::from_bytes(&raw).expect_err("zero coverage digest");
            assert!(why.contains(field), "{why}");
            assert!(why.contains("all zero"), "{why}");
        }

        let mut raw = canonical;
        raw[coverage_at + 16] ^= 1;
        reseal_receipt_v4(&mut raw);
        let why = CompletionReceiptV4::from_bytes(&raw).expect_err("wrong first civil day");
        assert!(why.contains("full requested civil-month span"), "{why}");
    }

    #[test]
    fn one_side_omission_and_copied_side_identity_refuse() {
        assert!(
            ExitCellsPerMaskV2::new(1, 0)
                .expect_err("missing short grid")
                .contains("both long and short")
        );

        let mut copied = identities(70);
        copied.exit_grids.short = copied.exit_grids.long;
        assert!(
            copied
                .exit_grids
                .composite_digest()
                .expect_err("copied side")
                .contains("omitted or copied")
        );

        let population_id = digest(71);
        let rows = [row(population_id, 0, TradeDirectionV1::Long)];
        let why = CompletionReceiptV2::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            &rows,
            CompletionReconciliationV2 {
                sweep_trials: 1,
                frequent_itemsets: 1,
                infrequent_itemsets: 0,
                closed_itemsets: 1,
                redundant_itemsets: 0,
                unknown_closure_itemsets: 0,
                exit_cells_per_mask: ExitCellsPerMaskV2::new(1, 1).expect("shape"),
                extinction_depth: 1,
                extinction_complete: true,
                closure_complete: true,
            },
            identities(70),
        )
        .expect_err("short rows omitted");
        assert!(
            why.contains("direction") || why.contains("long, short and total"),
            "{why}"
        );
    }

    #[test]
    fn swapped_sides_change_identity_and_exact_reuse_refuses() {
        let canonical = identities(80);
        let mut swapped = canonical;
        std::mem::swap(&mut swapped.exit_grids.long, &mut swapped.exit_grids.short);
        assert_ne!(
            canonical
                .exit_grids
                .composite_digest()
                .expect("canonical composite"),
            swapped
                .exit_grids
                .composite_digest()
                .expect("swapped composite")
        );

        let root = root("swapped-side-identity");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(81);
        let rows = two_rows(population_id);
        let shape = reconciliation(&rows);
        let first = CompletionReceiptV2::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            &rows,
            shape,
            canonical,
        )
        .expect("canonical receipt");
        let second = CompletionReceiptV2::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            &rows,
            shape,
            swapped,
        )
        .expect("swapped receipt is a different explicit claim");
        assert_ne!(
            first.to_bytes().expect("first"),
            second.to_bytes().expect("second")
        );
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        ledger.append_complete(&rows, &first).expect("first commit");
        let why = ledger
            .append_complete(&rows, &second)
            .expect_err("swapped exact reuse");
        assert!(why.contains("different completion receipt"), "{why}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn v2_sweep_trials_count_each_infrequent_itemset_once() {
        let population_id = digest(200);
        let rows = two_rows(population_id);
        let receipt = receipt(population_id, &rows);
        assert_eq!(receipt.frequent_itemsets, 1);
        assert_eq!(receipt.infrequent_itemsets, 2);
        assert_eq!(receipt.sweep_trials, 3);

        let mut duplicate_counted = receipt;
        duplicate_counted.sweep_trials = 5;
        assert_eq!(
            duplicate_counted.validate_semantics(),
            Err("frequent + infrequent itemsets is 3, not sweep_trials 5".to_owned())
        );
    }

    #[test]
    fn typed_population_arithmetic_refuses_every_overflow_boundary() {
        let population_id = digest(82);
        for (name, closed, long, short) in [
            ("directions", u64::MAX, 1, 1),
            ("long cells", 2, u64::MAX, 1),
            ("short cells", 2, 1, u64::MAX),
        ] {
            let why = CompletionReceiptV2::for_rows(
                population_id,
                InstrumentFamilyV1::Nifty,
                300,
                &[],
                CompletionReconciliationV2 {
                    sweep_trials: closed,
                    frequent_itemsets: closed,
                    infrequent_itemsets: 0,
                    closed_itemsets: closed,
                    redundant_itemsets: 0,
                    unknown_closure_itemsets: 0,
                    exit_cells_per_mask: ExitCellsPerMaskV2::new(long, short)
                        .expect("nonzero overflow fixture"),
                    extinction_depth: 1,
                    extinction_complete: true,
                    closure_complete: true,
                },
                identities(82),
            )
            .expect_err(name);
            assert!(why.contains("overflow"), "{name}: {why}");
        }
    }

    #[test]
    fn partial_coordinate_population_refuses_even_when_all_totals_match() {
        let population_id = digest(83);
        let mask_a = [7, 0, 0, 0, 0, 0];
        let mask_b = [11, 0, 0, 0, 0, 0];
        let specs = [
            (mask_a, TradeDirectionV1::Long),
            (mask_a, TradeDirectionV1::Short),
            (mask_b, TradeDirectionV1::Long),
            (mask_b, TradeDirectionV1::Long),
            (mask_b, TradeDirectionV1::Long),
            (mask_b, TradeDirectionV1::Short),
        ];
        let mut rows = Vec::new();
        for (sequence, (mask, direction)) in specs.into_iter().enumerate() {
            let sequence = u64::try_from(sequence).expect("small sequence");
            let mut candidate = row(population_id, sequence, direction);
            candidate.mask_words = mask;
            candidate.strategy_digest =
                digest(100_u8.saturating_add(u8::try_from(sequence).expect("small digest seed")));
            rows.push(candidate);
        }
        let why = CompletionReceiptV2::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            &rows,
            CompletionReconciliationV2 {
                sweep_trials: 2,
                frequent_itemsets: 2,
                infrequent_itemsets: 0,
                closed_itemsets: 2,
                redundant_itemsets: 0,
                unknown_closure_itemsets: 0,
                exit_cells_per_mask: ExitCellsPerMaskV2::new(2, 1).expect("shape"),
                extinction_depth: 2,
                extinction_complete: true,
                closure_complete: true,
            },
            identities(83),
        )
        .expect_err("one long cell moved between masks");
        assert!(why.contains("partial/non-uniform"), "{why}");
    }

    #[test]
    fn every_side_specific_grid_digest_is_composite_and_receipt_load_bearing() {
        let base = identities(90);
        let base_composite = base.exit_grids.composite_digest().expect("base composite");
        let population_id = digest(90);
        let rows = two_rows(population_id);
        let base_receipt = CompletionReceiptV2::for_rows(
            population_id,
            InstrumentFamilyV1::Nifty,
            300,
            &rows,
            reconciliation(&rows),
            base,
        )
        .expect("base receipt")
        .to_bytes()
        .expect("base receipt bytes");

        let mut variants = [base; 4];
        variants[0].exit_grids.long.policy_digest = digest(201);
        variants[1].exit_grids.long.resolved_digest = digest(202);
        variants[2].exit_grids.short.policy_digest = digest(203);
        variants[3].exit_grids.short.resolved_digest = digest(204);
        for (nth, variant) in variants.into_iter().enumerate() {
            assert_ne!(
                variant
                    .exit_grids
                    .composite_digest()
                    .expect("variant composite"),
                base_composite,
                "side digest {nth} was absent from the composite"
            );
            let bytes = CompletionReceiptV2::for_rows(
                population_id,
                InstrumentFamilyV1::Nifty,
                300,
                &rows,
                reconciliation(&rows),
                variant,
            )
            .expect("variant receipt")
            .to_bytes()
            .expect("variant receipt bytes");
            assert_ne!(
                bytes, base_receipt,
                "side digest {nth} was absent from durable V2 receipt bytes"
            );
        }
    }

    #[test]
    fn coordinate_sentinel_and_impossible_ttp_geometry_refuse_before_encoding() {
        let population_id = digest(19);
        for (name, exit) in [
            (
                "stop sentinel",
                ExitCoordinateV1 {
                    stop: Some(u32::MAX),
                    target: None,
                    tsl: None,
                    ttp: None,
                },
            ),
            (
                "target sentinel",
                ExitCoordinateV1 {
                    stop: None,
                    target: Some(u32::MAX),
                    tsl: None,
                    ttp: None,
                },
            ),
            (
                "tsl sentinel",
                ExitCoordinateV1 {
                    stop: None,
                    target: None,
                    tsl: Some(u32::MAX),
                    ttp: None,
                },
            ),
            (
                "ttp arm sentinel",
                ExitCoordinateV1 {
                    stop: None,
                    target: None,
                    tsl: None,
                    ttp: Some((u32::MAX, 0)),
                },
            ),
            (
                "ttp trail sentinel",
                ExitCoordinateV1 {
                    stop: None,
                    target: None,
                    tsl: None,
                    ttp: Some((0, u32::MAX)),
                },
            ),
            (
                "ttp wakes at fixed target",
                ExitCoordinateV1 {
                    stop: None,
                    target: Some(2),
                    tsl: None,
                    ttp: Some((2, 0)),
                },
            ),
            (
                "ttp no tighter than live tsl",
                ExitCoordinateV1 {
                    stop: None,
                    target: None,
                    tsl: Some(1),
                    ttp: Some((0, 1)),
                },
            ),
        ] {
            let mut candidate = row(population_id, 0, TradeDirectionV1::Long);
            candidate.exit = exit;
            let why = candidate.to_bytes().expect_err(name);
            assert!(
                why.contains("reserved") || why.contains("must be below"),
                "{name}: {why}"
            );
        }

        let mut valid = row(population_id, 0, TradeDirectionV1::Long);
        valid.exit = ExitCoordinateV1 {
            stop: Some(0),
            target: Some(2),
            tsl: Some(2),
            ttp: Some((1, 1)),
        };
        assert_eq!(
            PopulationRowV1::from_bytes(&valid.to_bytes().expect("valid coordinate"))
                .expect("valid decoded coordinate"),
            valid
        );
    }

    #[test]
    fn trade_count_rates_are_exact_floor_projections_including_zero_and_overflow() {
        let population_id = digest(20);
        let mut non_divisible = row(population_id, 0, TradeDirectionV1::Long);
        non_divisible.metrics.winning_trades = 2;
        non_divisible.metrics.losing_trades = 1;
        non_divisible.metrics.win_rate_ppm = 666_666;
        non_divisible.metrics.losing_rate_ppm = 333_333;
        assert!(non_divisible.to_bytes().is_ok(), "canonical floors pass");

        let mut bad_win = non_divisible;
        bad_win.metrics.win_rate_ppm = 666_667;
        assert!(
            bad_win
                .to_bytes()
                .expect_err("rounded win rate")
                .contains("canonical floor")
        );
        let mut bad_loss = non_divisible;
        bad_loss.metrics.losing_rate_ppm = 333_334;
        assert!(
            bad_loss
                .to_bytes()
                .expect_err("rounded losing rate")
                .contains("canonical floor")
        );

        let mut empty = row(population_id, 0, TradeDirectionV1::Long);
        empty.metrics.winning_trades = 0;
        empty.metrics.losing_trades = 0;
        empty.metrics.win_rate_ppm = 0;
        empty.metrics.losing_rate_ppm = 0;
        assert!(
            empty.to_bytes().is_ok(),
            "zero counts have zero projections"
        );
        empty.metrics.win_rate_ppm = 1;
        assert!(
            empty
                .to_bytes()
                .expect_err("rate without trades")
                .contains("canonical floor projection is 0")
        );

        let mut overflow = row(population_id, 0, TradeDirectionV1::Long);
        overflow.metrics.winning_trades = u64::MAX;
        overflow.metrics.losing_trades = 1;
        assert!(
            overflow
                .to_bytes()
                .expect_err("count overflow")
                .contains("overflow")
        );
    }

    #[test]
    fn complete_block_is_receipt_last_replayable_and_page_bounded() {
        let root = root("commit-page");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(2);
        let rows = two_rows(population_id);
        let receipt = receipt(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        assert_eq!(
            ledger.append_complete(&rows, &receipt).expect("commit"),
            PopulationCommit::Written
        );
        assert_eq!(
            std::fs::metadata(PopulationLedger::row_path(&root))
                .expect("row metadata")
                .len(),
            super::HEADER + 2 * super::ROW_STRIDE
        );
        assert_eq!(
            std::fs::metadata(PopulationLedger::receipt_path(&root))
                .expect("receipt metadata")
                .len(),
            super::HEADER + super::RECEIPT_STRIDE
        );
        drop(ledger);

        let mut reader = PopulationLedger::open_read(&root).expect("reader");
        assert_eq!(reader.populations(), 1);
        assert_eq!(reader.receipt(&population_id), Some(receipt));
        assert!(
            reader
                .append_complete(&rows, &receipt)
                .expect_err("read-only commit")
                .contains("read-only population handle")
        );
        assert_eq!(
            reader.block(&population_id),
            Some(super::PopulationBlock { first: 0, count: 2 })
        );
        let page = reader
            .page(&population_id, 1, 25)
            .expect("page")
            .expect("population");
        assert_eq!(page.total, 2);
        assert_eq!(page.offset, 1);
        assert_eq!(page.rows, vec![rows[1]]);
        assert_eq!(
            reader.row(&population_id, 0).expect("row lookup"),
            Some(rows[0])
        );
        assert!(
            reader
                .page(&population_id, 3, 1)
                .expect_err("past end")
                .contains("past")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn page_limit_is_a_hard_versioned_ceiling_before_any_large_allocation() {
        let root = root("page-ceiling");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(21);
        let rows = two_rows(population_id);
        let receipt = receipt(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        ledger.append_complete(&rows, &receipt).expect("commit");
        let before = std::fs::read(PopulationLedger::row_path(&root)).expect("row bytes");

        let why = ledger
            .page(&population_id, 0, super::MAX_PAGE_ROWS_V1.saturating_add(1))
            .expect_err("over-limit page");
        assert!(why.contains("version-one maximum"), "{why}");
        assert_eq!(
            ledger
                .page(&population_id, 0, super::MAX_PAGE_ROWS_V1)
                .expect("maximum page")
                .expect("population")
                .rows,
            rows
        );
        assert_eq!(
            std::fs::read(PopulationLedger::row_path(&root)).expect("row bytes"),
            before,
            "a refused read never mutates the ledger"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn row_writes_cross_fixed_chunks_and_exact_reuse_streams_without_a_block_buffer() {
        let root = root("chunked-rows");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(22);
        let mut rows = Vec::new();
        for sequence in 0..65_u64 {
            let direction = if sequence.is_multiple_of(2) {
                TradeDirectionV1::Long
            } else {
                TradeDirectionV1::Short
            };
            let mut candidate = row(population_id, sequence, direction);
            candidate.strategy_digest = digest(
                u8::try_from(sequence)
                    .expect("small fixture sequence")
                    .saturating_add(100),
            );
            rows.push(candidate);
        }
        let receipt = receipt(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        assert_eq!(
            ledger.append_complete(&rows, &receipt).expect("commit"),
            PopulationCommit::Written
        );
        let before = std::fs::read(PopulationLedger::row_path(&root)).expect("row bytes");
        assert_eq!(
            before.len(),
            super::HEADER_BYTES + 65 * super::ROW_STRIDE_BYTES
        );
        assert_eq!(
            ledger.append_complete(&rows, &receipt).expect("reuse"),
            PopulationCommit::Reused
        );
        assert_eq!(
            std::fs::read(PopulationLedger::row_path(&root)).expect("row bytes"),
            before
        );
        assert_eq!(
            ledger.row(&population_id, 64).expect("last row"),
            rows.get(64).copied()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn exact_rerun_reuses_without_one_new_byte_and_changed_rows_refuse() {
        let root = root("reuse-mismatch");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(3);
        let rows = two_rows(population_id);
        let receipt = receipt(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        assert_eq!(
            ledger.append_complete(&rows, &receipt).expect("first"),
            PopulationCommit::Written
        );
        let before_rows = std::fs::read(PopulationLedger::row_path(&root)).expect("rows");
        let before_receipts =
            std::fs::read(PopulationLedger::receipt_path(&root)).expect("receipts");
        assert_eq!(
            ledger.append_complete(&rows, &receipt).expect("reuse"),
            PopulationCommit::Reused
        );
        assert_eq!(
            std::fs::read(PopulationLedger::row_path(&root)).expect("rows"),
            before_rows
        );
        assert_eq!(
            std::fs::read(PopulationLedger::receipt_path(&root)).expect("receipts"),
            before_receipts
        );

        let mut changed = rows;
        changed[1].metrics.drawdown = changed[1].metrics.drawdown.saturating_add(1);
        let why = ledger
            .append_complete(&changed, &receipt)
            .expect_err("same identity changed row");
        assert!(why.contains("receipt") || why.contains("digest"), "{why}");
        assert_eq!(
            std::fs::read(PopulationLedger::row_path(&root)).expect("rows"),
            before_rows
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn same_id_with_different_policy_receipt_never_rewrites() {
        let root = root("receipt-mismatch");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(4);
        let rows = two_rows(population_id);
        let first = receipt(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        ledger.append_complete(&rows, &first).expect("first");
        let before = std::fs::read(PopulationLedger::receipt_path(&root)).expect("receipt bytes");
        let mut changed = first;
        changed.identities.ranking_policy_digest = digest(99);
        let why = ledger
            .append_complete(&rows, &changed)
            .expect_err("different policy receipt");
        assert!(why.contains("different completion receipt"), "{why}");
        assert_eq!(
            std::fs::read(PopulationLedger::receipt_path(&root)).expect("receipt bytes"),
            before
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn zero_row_population_is_committed_reused_and_read_as_empty() {
        let root = root("zero");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(5);
        let receipt = zero_receipt(population_id);
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        assert_eq!(
            ledger.append_complete(&[], &receipt).expect("zero commit"),
            PopulationCommit::Written
        );
        assert_eq!(
            ledger.append_complete(&[], &receipt).expect("zero reuse"),
            PopulationCommit::Reused
        );
        assert_eq!(
            ledger.block(&population_id),
            Some(super::PopulationBlock { first: 0, count: 0 })
        );
        assert!(
            ledger
                .page(&population_id, 0, 25)
                .expect("zero page")
                .expect("committed")
                .rows
                .is_empty()
        );
        assert_eq!(
            std::fs::metadata(PopulationLedger::row_path(&root))
                .expect("row metadata")
                .len(),
            super::HEADER
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn exact_orphan_block_is_recovered_but_different_orphan_is_not_blessed() {
        let root = root("orphan");
        let _ = std::fs::remove_dir_all(&root);
        drop(PopulationLedger::open(&root).expect("headers"));
        let population_id = digest(6);
        let rows = two_rows(population_id);
        let mut row_file = OpenOptions::new()
            .append(true)
            .open(PopulationLedger::row_path(&root))
            .expect("row file");
        for row in rows {
            row_file
                .write_all(&row.to_bytes().expect("row bytes"))
                .expect("orphan row");
        }
        row_file.sync_all().expect("orphan sync");
        drop(row_file);

        let receipt = receipt(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("orphan reader");
        assert!(ledger.receipt(&population_id).is_none());
        assert_eq!(
            ledger
                .append_complete(&rows, &receipt)
                .expect("recover exact orphan"),
            PopulationCommit::Written
        );
        let _ = std::fs::remove_dir_all(&root);

        let mismatch_root = self::root("orphan-mismatch");
        let _ = std::fs::remove_dir_all(&mismatch_root);
        drop(PopulationLedger::open(&mismatch_root).expect("headers"));
        let mut row_file = OpenOptions::new()
            .append(true)
            .open(PopulationLedger::row_path(&mismatch_root))
            .expect("row file");
        for row in rows {
            row_file
                .write_all(&row.to_bytes().expect("row bytes"))
                .expect("orphan row");
        }
        row_file.sync_all().expect("orphan sync");
        drop(row_file);
        let mut changed = rows;
        changed[1].metrics.average_loss = changed[1].metrics.average_loss.saturating_add(1);
        let changed_receipt = self::receipt(population_id, &changed);
        let mut ledger = PopulationLedger::open(&mismatch_root).expect("orphan reader");
        let why = ledger
            .append_complete(&changed, &changed_receipt)
            .expect_err("different orphan");
        assert!(why.contains("different row facts"), "{why}");
        assert_eq!(
            std::fs::metadata(PopulationLedger::receipt_path(&mismatch_root))
                .expect("receipt metadata")
                .len(),
            super::HEADER,
            "a mismatched orphan gets no commit receipt"
        );
        let _ = std::fs::remove_dir_all(&mismatch_root);
    }

    #[test]
    fn ragged_row_and_receipt_tails_are_never_ignored() {
        for (name, path_of) in [
            (
                "ragged-row",
                PopulationLedger::row_path as fn(&std::path::Path) -> std::path::PathBuf,
            ),
            ("ragged-receipt", PopulationLedger::receipt_path),
        ] {
            let root = root(name);
            let _ = std::fs::remove_dir_all(&root);
            drop(PopulationLedger::open(&root).expect("headers"));
            OpenOptions::new()
                .append(true)
                .open(path_of(&root))
                .expect("tail file")
                .write_all(&[0xff])
                .expect("ragged byte");
            let why = PopulationLedger::open_read(&root).expect_err("ragged tail");
            assert!(why.contains("torn/ragged"), "{why}");
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn corrupt_row_and_receipt_seals_are_both_refused() {
        for (name, corrupt_receipt) in [("seal-row", false), ("seal-receipt", true)] {
            let root = root(name);
            let _ = std::fs::remove_dir_all(&root);
            let population_id = digest(if corrupt_receipt { 8 } else { 7 });
            let rows = two_rows(population_id);
            let receipt = receipt(population_id, &rows);
            let mut ledger = PopulationLedger::open(&root).expect("ledger");
            ledger.append_complete(&rows, &receipt).expect("commit");
            drop(ledger);
            let path = if corrupt_receipt {
                PopulationLedger::receipt_path(&root)
            } else {
                PopulationLedger::row_path(&root)
            };
            let mut bytes = std::fs::read(&path).expect("file bytes");
            let payload = bytes.get_mut(super::HEADER_BYTES).expect("payload byte");
            *payload ^= 1;
            std::fs::write(&path, bytes).expect("corrupt fixture");
            let why = PopulationLedger::open_read(&root).expect_err("bad seal");
            assert!(why.contains("seal"), "{why}");
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn unknown_version_and_header_reserve_are_named_before_stride_guessing() {
        for (name, receipt_file) in [("version-row", false), ("version-receipt", true)] {
            let root = root(name);
            let _ = std::fs::remove_dir_all(&root);
            drop(PopulationLedger::open(&root).expect("headers"));
            let path = if receipt_file {
                PopulationLedger::receipt_path(&root)
            } else {
                PopulationLedger::row_path(&root)
            };
            let mut bytes = std::fs::read(&path).expect("header bytes");
            bytes
                .get_mut(8..12)
                .expect("version")
                .copy_from_slice(&9_u32.to_le_bytes());
            std::fs::write(&path, &bytes).expect("version fixture");
            let why = PopulationLedger::open_read(&root).expect_err("unknown version");
            assert!(why.contains("version 9"), "{why}");

            bytes.get_mut(8..12).expect("version").copy_from_slice(
                &if receipt_file {
                    super::RECEIPT_V2_VERSION
                } else {
                    super::ROW_VERSION
                }
                .to_le_bytes(),
            );
            *bytes.get_mut(12).expect("header reserve") = 1;
            std::fs::write(&path, bytes).expect("reserve fixture");
            let why = PopulationLedger::open_read(&root).expect_err("header reserve");
            assert!(why.contains("reserved header"), "{why}");
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn sealed_unknown_tags_noncanonical_options_and_reserved_bytes_refuse() {
        let population_id = digest(9);
        let base = row(population_id, 0, TradeDirectionV1::Long)
            .to_bytes()
            .expect("row bytes");
        for (name, mutate) in [
            ("direction", (120_usize, 9_u8)),
            ("family", (121, 9)),
            ("closure", (122, 9)),
            ("status", (123, 9)),
            ("coordinate reserve", (156, 1)),
            ("loss option tag", (192, 9)),
        ] {
            let mut raw = base;
            *raw.get_mut(mutate.0).expect("mutated byte") = mutate.1;
            reseal_row(&mut raw);
            let why = PopulationRowV1::from_bytes(&raw).expect_err(name);
            assert!(
                why.contains("unknown") || why.contains("reserved"),
                "{name}: {why}"
            );
        }

        let mut noncanonical = base;
        *noncanonical.get_mut(192).expect("loss option tag") = 0;
        noncanonical
            .get_mut(200..208)
            .expect("loss option value")
            .copy_from_slice(&5_u64.to_le_bytes());
        reseal_row(&mut noncanonical);
        assert!(
            PopulationRowV1::from_bytes(&noncanonical)
                .expect_err("noncanonical none")
                .contains("non-zero payload")
        );

        for (name, absent) in [
            ("missing ttp arm", 148_usize..152_usize),
            ("missing ttp trail", 152_usize..156_usize),
        ] {
            let mut half_ttp = base;
            half_ttp
                .get_mut(absent)
                .expect("ttp half")
                .copy_from_slice(&u32::MAX.to_le_bytes());
            reseal_row(&mut half_ttp);
            assert!(
                PopulationRowV1::from_bytes(&half_ttp)
                    .expect_err(name)
                    .contains("half-present")
            );
        }
    }

    #[test]
    fn sealed_unknown_reason_bits_and_receipt_reserves_refuse() {
        let population_id = digest(10);
        let rows = two_rows(population_id);
        let mut row_raw = rows[0].to_bytes().expect("row bytes");
        row_raw
            .get_mut(272..280)
            .expect("reason union")
            .copy_from_slice(&(1_u64 << 44).to_le_bytes());
        row_raw
            .get_mut(280..288)
            .expect("failed reasons")
            .copy_from_slice(&(1_u64 << 44).to_le_bytes());
        *row_raw.get_mut(123).expect("status") = 1;
        reseal_row(&mut row_raw);
        assert!(
            PopulationRowV1::from_bytes(&row_raw)
                .expect_err("unknown reason")
                .contains("unknown version-one reason")
        );

        let receipt = receipt(population_id, &rows);
        let base = receipt.to_bytes().expect("receipt bytes");
        for (name, at) in [("flag reserve", 235_usize), ("trailing reserve", 688)] {
            let mut raw = base;
            *raw.get_mut(at).expect("receipt reserve") = 1;
            reseal_receipt(&mut raw);
            let why = CompletionReceiptV2::from_bytes(&raw).expect_err(name);
            assert!(why.contains("reserved"), "{name}: {why}");
        }
    }

    #[test]
    fn duplicate_and_interleaved_row_blocks_and_duplicate_receipts_refuse() {
        let root = root("interleaved");
        let _ = std::fs::remove_dir_all(&root);
        drop(PopulationLedger::open(&root).expect("headers"));
        let a = row(digest(11), 0, TradeDirectionV1::Long);
        let b = row(digest(12), 0, TradeDirectionV1::Long);
        let mut file = OpenOptions::new()
            .append(true)
            .open(PopulationLedger::row_path(&root))
            .expect("row file");
        for candidate in [a, b, a] {
            file.write_all(&candidate.to_bytes().expect("row bytes"))
                .expect("row");
        }
        file.sync_all().expect("rows sync");
        drop(file);
        let why = PopulationLedger::open_read(&root).expect_err("interleaved blocks");
        assert!(
            why.contains("non-contiguous") || why.contains("interleaves"),
            "{why}"
        );
        let _ = std::fs::remove_dir_all(&root);

        let receipt_root = self::root("duplicate-receipt");
        let _ = std::fs::remove_dir_all(&receipt_root);
        let population_id = digest(13);
        let rows = two_rows(population_id);
        let receipt = receipt(population_id, &rows);
        let mut ledger = PopulationLedger::open(&receipt_root).expect("ledger");
        ledger.append_complete(&rows, &receipt).expect("commit");
        drop(ledger);
        OpenOptions::new()
            .append(true)
            .open(PopulationLedger::receipt_path(&receipt_root))
            .expect("receipt file")
            .write_all(&receipt.to_bytes().expect("receipt bytes"))
            .expect("duplicate receipt");
        let why = PopulationLedger::open_read(&receipt_root).expect_err("duplicate receipt");
        assert!(why.contains("duplicate completion receipt"), "{why}");
        let _ = std::fs::remove_dir_all(&receipt_root);
    }

    #[test]
    fn bad_sequence_crossed_family_and_incomplete_reconciliation_never_commit() {
        let population_id = digest(14);
        let mut bad_sequence = two_rows(population_id);
        bad_sequence[1].sequence = 4;
        assert!(
            CompletionReceiptV2::for_rows(
                population_id,
                InstrumentFamilyV1::Nifty,
                300,
                &bad_sequence,
                reconciliation(&bad_sequence),
                identities(10),
            )
            .expect_err("sequence")
            .contains("canonical sequence")
        );
        let mut crossed = two_rows(population_id);
        crossed[1].instrument_family = InstrumentFamilyV1::BankNifty;
        assert!(
            CompletionReceiptV2::for_rows(
                population_id,
                InstrumentFamilyV1::Nifty,
                300,
                &crossed,
                reconciliation(&crossed),
                identities(10),
            )
            .expect_err("crossed family")
            .contains("crossed instrument")
        );
        let rows = two_rows(population_id);
        let mut incomplete = reconciliation(&rows);
        incomplete.extinction_complete = false;
        assert!(
            CompletionReceiptV2::for_rows(
                population_id,
                InstrumentFamilyV1::Nifty,
                300,
                &rows,
                incomplete,
                identities(10),
            )
            .expect_err("incomplete extinction")
            .contains("incomplete extinction")
        );
    }

    #[test]
    fn v3_is_authoritative_reopens_and_v2_remains_auditable() {
        let root = root("v3-commit-reopen");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(127);
        let rows = two_rows(population_id);
        let receipt = receipt_v3(population_id, &rows, span());

        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        assert_eq!(
            ledger
                .append_complete_v3(&rows, &receipt)
                .expect("V3 commit"),
            PopulationCommit::Written
        );
        assert_eq!(ledger.populations(), 0, "V2 remains audit-only");
        assert_eq!(ledger.authoritative_populations(), 1);
        assert_eq!(ledger.receipt(&population_id), None);
        assert_eq!(ledger.receipt_v3(&population_id), Some(receipt));
        assert_eq!(
            ledger.block_v3(&population_id),
            Some(super::PopulationBlock { first: 0, count: 2 })
        );
        assert_eq!(
            std::fs::metadata(PopulationLedger::receipt_v3_path(&root))
                .expect("V3 receipt metadata")
                .len(),
            super::HEADER + super::RECEIPT_V3_STRIDE
        );
        drop(ledger);

        let mut reopened = PopulationLedger::open_read(&root).expect("V3 reader");
        assert_eq!(reopened.receipt_v3(&population_id), Some(receipt));
        assert_eq!(reopened.authoritative_populations(), 1);
        assert_eq!(
            reopened
                .page(&population_id, 0, 2)
                .expect("V3 page")
                .expect("authoritative population")
                .rows,
            rows
        );
        drop(reopened);

        let mut audit_writer = PopulationLedger::open(&root).expect("V2 audit writer");
        assert_eq!(
            audit_writer
                .append_complete(&rows, &receipt.v2())
                .expect("matching V2 audit receipt"),
            PopulationCommit::Written
        );
        drop(audit_writer);
        let reopened = PopulationLedger::open_read(&root).expect("V2 plus V3 reader");
        assert_eq!(reopened.receipt(&population_id), Some(receipt.v2()));
        assert_eq!(reopened.receipt_v3(&population_id), Some(receipt));
        drop(reopened);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pre_v3_audit_ledger_reopens_without_blessing_v2() {
        let root = root("pre-v3-audit");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(128);
        let rows = two_rows(population_id);
        let receipt = receipt(population_id, &rows);
        let mut writer = PopulationLedger::open(&root).expect("writer");
        writer
            .append_complete(&rows, &receipt)
            .expect("V2 audit receipt");
        drop(writer);
        std::fs::remove_file(PopulationLedger::receipt_v3_path(&root))
            .expect("model a pre-V3 ledger");

        let reader = PopulationLedger::open_read(&root).expect("legacy audit reader");
        assert_eq!(reader.receipt(&population_id), Some(receipt));
        assert_eq!(reader.receipt_v3(&population_id), None);
        assert_eq!(reader.populations(), 1);
        assert_eq!(reader.authoritative_populations(), 0);
        drop(reader);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn same_population_id_cannot_reuse_a_different_requested_span() {
        let root = root("v3-span-conflict");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(129);
        let rows = two_rows(population_id);
        let receipt = receipt_v3(population_id, &rows, span());
        let mut ledger = PopulationLedger::open(&root).expect("ledger");
        ledger
            .append_complete_v3(&rows, &receipt)
            .expect("first V3 commit");
        let before =
            std::fs::read(PopulationLedger::receipt_v3_path(&root)).expect("V3 receipt bytes");
        let changed = CompletionReceiptV3::from_v2(
            receipt.v2(),
            RequestedSpanIdentityV1::new(2019, 1, 2026, 7).expect("changed span"),
        )
        .expect("changed V3 value");
        assert!(
            ledger
                .append_complete_v3(&rows, &changed)
                .expect_err("same ID, different span")
                .contains("different authoritative V3")
        );
        assert_eq!(
            std::fs::read(PopulationLedger::receipt_v3_path(&root))
                .expect("unchanged V3 receipt bytes"),
            before
        );
        drop(ledger);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn v3_header_reserve_mutation_refuses_reopen() {
        let root = root("v3-header-reserve");
        let _ = std::fs::remove_dir_all(&root);
        drop(PopulationLedger::open(&root).expect("headers"));
        rewrite(&PopulationLedger::receipt_v3_path(&root), 12, &[1_u8]);
        assert!(
            PopulationLedger::open_read(&root)
                .expect_err("V3 header reserve")
                .contains("reserved header")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn every_population_companion_byte_preserves_the_original_authority_or_refuses() {
        let root = root("v4-every-companion-byte");
        std::fs::create_dir(&root).expect("exclusively owned population fixture");
        let population_id = digest(132);
        let rows = two_rows(population_id);
        let receipt = receipt_v4(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("owned V4 ledger");
        assert_eq!(
            ledger.append_complete_v4(&rows, &receipt),
            Ok(PopulationCommit::Written)
        );
        drop(ledger);

        let mut changed_bytes = 0;
        for (path, stride) in [
            (PopulationLedger::row_path(&root), super::ROW_STRIDE_BYTES),
            (
                PopulationLedger::receipt_path(&root),
                super::RECEIPT_STRIDE_BYTES,
            ),
            (
                PopulationLedger::receipt_v3_path(&root),
                super::RECEIPT_V3_STRIDE_BYTES,
            ),
            (
                PopulationLedger::receipt_v4_path(&root),
                super::RECEIPT_V4_STRIDE_BYTES,
            ),
        ] {
            let original = std::fs::read(&path).expect("committed population companion");
            for offset in 0..original.len() {
                let mut changed = original.clone();
                changed[offset] ^= 1;
                if offset >= super::HEADER_BYTES {
                    let start =
                        super::HEADER_BYTES + (offset - super::HEADER_BYTES) / stride * stride;
                    let seal_at = start + stride - super::SEAL_BYTES;
                    if offset < seal_at {
                        let seal = super::seal(&changed[start..seal_at]);
                        changed[seal_at..start + stride].copy_from_slice(&seal);
                    }
                }
                std::fs::write(&path, changed).expect("owned changed companion");
                let accepted_original = match PopulationLedger::open_read(&root) {
                    Ok(mut reader) if reader.receipt_v4(&population_id) == Some(receipt) => {
                        matches!(
                            reader.page_v4(&population_id, 0, 2),
                            Ok(Some(page)) if page.rows == rows
                        )
                    }
                    Ok(_) | Err(_) => false,
                };
                assert!(
                    !accepted_original,
                    "{} byte {offset} silently preserved the original authority",
                    path.display()
                );
                changed_bytes += 1;
            }
            std::fs::write(&path, &original).expect("restore exact population companion");
            let mut restored = PopulationLedger::open_read(&root)
                .expect("restored population authenticates freshly");
            assert_eq!(restored.receipt_v4(&population_id), Some(receipt));
            assert_eq!(
                restored
                    .page_v4(&population_id, 0, 2)
                    .expect("restored page reads")
                    .expect("original authority exists")
                    .rows,
                rows
            );
            assert_eq!(std::fs::read(path).expect("read-only companion"), original);
        }
        assert_eq!(changed_bytes, 3_024);
        std::fs::remove_dir_all(root).expect("remove only this owned fixture");
    }

    #[test]
    fn v4_commits_all_audit_layers_reuses_exactly_and_authorizes_bounded_pages() {
        let root = root("v4-commit-reopen");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(130);
        let rows = two_rows(population_id);
        let receipt = receipt_v4(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("V4 ledger");
        assert_eq!(
            ledger
                .append_complete_v4(&rows, &receipt)
                .expect("V4 commit"),
            PopulationCommit::Written
        );
        assert_eq!(ledger.receipt(&population_id), Some(receipt.v3().v2()));
        assert_eq!(ledger.receipt_v3(&population_id), Some(receipt.v3()));
        assert_eq!(ledger.receipt_v4(&population_id), Some(receipt));
        assert_eq!(ledger.authoritative_v4_populations(), 1);
        assert_eq!(
            ledger.block_v4(&population_id),
            Some(super::PopulationBlock { first: 0, count: 2 })
        );
        assert_eq!(
            ledger
                .page_v4(&population_id, 0, 2)
                .expect("V4 page")
                .expect("V4 authority")
                .rows,
            rows
        );
        for (path, stride) in [
            (PopulationLedger::receipt_path(&root), super::RECEIPT_STRIDE),
            (
                PopulationLedger::receipt_v3_path(&root),
                super::RECEIPT_V3_STRIDE,
            ),
            (
                PopulationLedger::receipt_v4_path(&root),
                super::RECEIPT_V4_STRIDE,
            ),
        ] {
            assert_eq!(
                std::fs::metadata(path).expect("receipt metadata").len(),
                super::HEADER + stride
            );
        }
        let before = std::fs::read(PopulationLedger::receipt_v4_path(&root)).expect("V4 bytes");
        assert_eq!(
            ledger
                .append_complete_v4(&rows, &receipt)
                .expect("exact V4 rerun"),
            PopulationCommit::Reused
        );
        assert_eq!(
            std::fs::read(PopulationLedger::receipt_v4_path(&root)).expect("reused V4 bytes"),
            before
        );

        let mut different_coverage = receipt;
        different_coverage.coverage.signal_complete_receipt_digest[0] ^= 1;
        let why = ledger
            .append_complete_v4(&rows, &different_coverage)
            .expect_err("same population cannot alias different complete-calendar evidence");
        assert!(why.contains("different authoritative V4"), "{why}");
        drop(ledger);

        let mut reopened = PopulationLedger::open_read(&root).expect("reopened V4 ledger");
        assert_eq!(reopened.receipt_v4(&population_id), Some(receipt));
        assert_eq!(
            reopened
                .page_v4(&population_id, 1, 1)
                .expect("reopened V4 page")
                .expect("V4 authority")
                .rows,
            vec![rows[1]]
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn two_nonempty_v4_populations_commit_at_their_exact_sequential_locations() {
        let root = root("two-nonempty-v4-blocks");
        let _ = std::fs::remove_dir_all(&root);
        let nifty_id = digest(138);
        let nifty_rows = two_rows(nifty_id);
        let nifty_receipt = receipt_v4(nifty_id, &nifty_rows);
        let bank_id = digest(139);
        let mut bank_rows = two_rows(bank_id);
        for row in &mut bank_rows {
            row.instrument_family = InstrumentFamilyV1::BankNifty;
        }
        let requested_span = month_span();
        let bank_receipt = CompletionReceiptV4::for_rows(
            bank_id,
            InstrumentFamilyV1::BankNifty,
            300,
            &bank_rows,
            requested_span,
            reconciliation(&bank_rows),
            identities(10),
            complete_calendar(requested_span, 300),
            complete_calendar(requested_span, 60),
        )
        .expect("BANKNIFTY V4 receipt");

        let mut ledger = PopulationLedger::open(&root).expect("V4 ledger");
        assert_eq!(
            ledger
                .append_complete_v4(&nifty_rows, &nifty_receipt)
                .expect("first non-empty V4 population"),
            PopulationCommit::Written
        );
        assert_eq!(
            ledger
                .append_complete_v4(&bank_rows, &bank_receipt)
                .expect("second non-empty V4 population"),
            PopulationCommit::Written
        );
        assert_eq!(
            ledger.block_v4(&nifty_id),
            Some(super::PopulationBlock { first: 0, count: 2 })
        );
        assert_eq!(
            ledger.block_v4(&bank_id),
            Some(super::PopulationBlock { first: 2, count: 2 })
        );
        assert_eq!(
            ledger
                .page_v4(&bank_id, 0, 2)
                .expect("BANKNIFTY V4 page")
                .expect("BANKNIFTY V4 authority")
                .rows,
            bank_rows
        );
        drop(ledger);

        let reopened = PopulationLedger::open_read(&root).expect("reopened two-population V4");
        assert_eq!(reopened.receipt_v4(&nifty_id), Some(nifty_receipt));
        assert_eq!(reopened.receipt_v4(&bank_id), Some(bank_receipt));
        drop(reopened);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pre_v4_ledger_remains_audit_readable_but_never_gains_v4_authority() {
        let root = root("pre-v4-audit");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(131);
        let rows = two_rows(population_id);
        let receipt = receipt_v3(population_id, &rows, month_span());
        let mut writer = PopulationLedger::open(&root).expect("writer");
        writer
            .append_complete_v3(&rows, &receipt)
            .expect("V3 audit completion");
        drop(writer);
        std::fs::remove_file(PopulationLedger::receipt_v4_path(&root))
            .expect("model a pre-V4 ledger");
        let mut reader = PopulationLedger::open_read(&root).expect("pre-V4 audit reader");
        assert_eq!(reader.receipt_v3(&population_id), Some(receipt));
        assert_eq!(reader.receipt_v4(&population_id), None);
        assert_eq!(reader.authoritative_v4_populations(), 0);
        assert_eq!(
            reader.page_v4(&population_id, 0, 2).expect("no V4 page"),
            None
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn v4_reopen_refuses_missing_v3_bad_header_ragged_tail_and_bad_seal() {
        for (name, damage) in [
            ("v4-missing-v3", 0_u8),
            ("v4-bad-header", 1),
            ("v4-ragged", 2),
            ("v4-bad-seal", 3),
        ] {
            let root = root(name);
            let _ = std::fs::remove_dir_all(&root);
            let population_id = digest(132);
            let rows = two_rows(population_id);
            let receipt = receipt_v4(population_id, &rows);
            let mut writer = PopulationLedger::open(&root).expect("writer");
            writer
                .append_complete_v4(&rows, &receipt)
                .expect("V4 fixture");
            drop(writer);
            match damage {
                0 => std::fs::remove_file(PopulationLedger::receipt_v3_path(&root))
                    .expect("remove V3 audit"),
                1 => rewrite(&PopulationLedger::receipt_v4_path(&root), 12, &[1]),
                2 => {
                    let path = PopulationLedger::receipt_v4_path(&root);
                    let mut file = OpenOptions::new()
                        .append(true)
                        .open(&path)
                        .expect("V4 fixture file");
                    file.write_all(&[0]).expect("ragged byte");
                    file.sync_all().expect("ragged sync");
                }
                3 => {
                    let path = PopulationLedger::receipt_v4_path(&root);
                    let bytes = std::fs::read(&path).expect("V4 bytes");
                    let at = usize::try_from(super::HEADER)
                        .expect("header")
                        .saturating_add(super::RECEIPT_V4_PAYLOAD_BYTES);
                    let changed = bytes[at] ^ 1;
                    rewrite(
                        &path,
                        super::HEADER
                            + u64::try_from(super::RECEIPT_V4_PAYLOAD_BYTES)
                                .expect("V4 payload width fits u64"),
                        &[changed],
                    );
                }
                _ => unreachable!(),
            }
            let why = PopulationLedger::open_read(&root).expect_err("damaged V4 must refuse");
            let needle = match damage {
                0 => "without its V3 audit receipt",
                1 => "reserved header",
                2 => "ragged",
                3 => "BLAKE3 seal",
                _ => unreachable!(),
            };
            assert!(why.contains(needle), "{why}");
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn two_stale_writers_converge_on_one_exact_population() {
        let root = root("concurrent");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(15);
        let rows = two_rows(population_id);
        let receipt = receipt(population_id, &rows);
        let mut one = PopulationLedger::open(&root).expect("one");
        let mut two = PopulationLedger::open(&root).expect("two");
        let first = std::thread::spawn(move || one.append_complete(&rows, &receipt));
        let second = std::thread::spawn(move || two.append_complete(&rows, &receipt));
        let states = [
            first.join().expect("first thread").expect("first append"),
            second
                .join()
                .expect("second thread")
                .expect("second append"),
        ];
        assert!(states.contains(&PopulationCommit::Written));
        assert!(states.contains(&PopulationCommit::Reused));
        assert_eq!(
            std::fs::metadata(PopulationLedger::row_path(&root))
                .expect("row metadata")
                .len(),
            super::HEADER + 2 * super::ROW_STRIDE
        );
        assert_eq!(
            std::fs::metadata(PopulationLedger::receipt_path(&root))
                .expect("receipt metadata")
                .len(),
            super::HEADER + super::RECEIPT_STRIDE
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stale_writer_absorbs_a_distinct_complete_population_before_append() {
        let root = root("stale-distinct");
        let _ = std::fs::remove_dir_all(&root);
        let a_id = digest(16);
        let b_id = digest(17);
        let a_rows = two_rows(a_id);
        let b_rows = two_rows(b_id);
        let mut first = PopulationLedger::open(&root).expect("first");
        let mut stale = PopulationLedger::open(&root).expect("stale");
        first
            .append_complete(&a_rows, &receipt(a_id, &a_rows))
            .expect("first population");
        stale
            .append_complete(&b_rows, &receipt(b_id, &b_rows))
            .expect("stale distinct append");
        let reader = PopulationLedger::open_read(&root).expect("reader");
        assert_eq!(reader.populations(), 2);
        assert!(reader.receipt(&a_id).is_some());
        assert!(reader.receipt(&b_id).is_some());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn same_length_mutation_of_each_population_file_refuses_cached_append() {
        enum Target {
            Rows,
            ReceiptV2,
            ReceiptV3,
            ReceiptV4,
        }

        for (name, target) in [
            ("same-length-rows", Target::Rows),
            ("same-length-v2", Target::ReceiptV2),
            ("same-length-v3", Target::ReceiptV3),
            ("same-length-v4", Target::ReceiptV4),
        ] {
            let root = root(name);
            let _ = std::fs::remove_dir_all(&root);
            let population_id = digest(161);
            let rows = two_rows(population_id);
            let mut ledger = PopulationLedger::open(&root).expect("population writer");
            match target {
                Target::ReceiptV2 => {
                    ledger
                        .append_complete(&rows, &receipt(population_id, &rows))
                        .expect("V2 fixture commit");
                }
                Target::Rows | Target::ReceiptV3 => {
                    ledger
                        .append_complete_v3(&rows, &receipt_v3(population_id, &rows, span()))
                        .expect("V3 fixture commit");
                }
                Target::ReceiptV4 => {
                    ledger
                        .append_complete_v4(&rows, &receipt_v4(population_id, &rows))
                        .expect("V4 fixture commit");
                }
            }
            let path = match target {
                Target::Rows => PopulationLedger::row_path(&root),
                Target::ReceiptV2 => PopulationLedger::receipt_path(&root),
                Target::ReceiptV3 => PopulationLedger::receipt_v3_path(&root),
                Target::ReceiptV4 => PopulationLedger::receipt_v4_path(&root),
            };
            let before = std::fs::metadata(&path).expect("population metadata").len();
            let original = std::fs::read(&path).expect("population bytes");
            let changed = original
                .get(usize::try_from(super::HEADER).expect("header offset fits usize"))
                .copied()
                .expect("first record byte")
                ^ 1;
            rewrite(&path, super::HEADER, &[changed]);

            let next_id = digest(162);
            let next_rows = two_rows(next_id);
            let why = match target {
                Target::ReceiptV2 => ledger
                    .append_complete(&next_rows, &receipt(next_id, &next_rows))
                    .expect_err("same-length V2 mutation must refuse"),
                Target::Rows | Target::ReceiptV3 => ledger
                    .append_complete_v3(&next_rows, &receipt_v3(next_id, &next_rows, span()))
                    .expect_err("same-length V3 authority mutation must refuse"),
                Target::ReceiptV4 => ledger
                    .append_complete_v4(&next_rows, &receipt_v4(next_id, &next_rows))
                    .expect_err("same-length V4 authority mutation must refuse"),
            };
            assert!(why.contains("filesystem generation changed"), "{why}");
            assert_eq!(
                std::fs::metadata(&path).expect("population metadata").len(),
                before
            );
            drop(ledger);
            let _ = std::fs::remove_dir_all(&root);
        }
    }

    #[test]
    fn bounded_open_refuses_before_building_either_partial_index() {
        let root = root("bounded");
        let _ = std::fs::remove_dir_all(&root);
        drop(PopulationLedger::open(&root).expect("headers"));
        let why = PopulationLedger::open_read_bounded(&root, super::HEADER.saturating_sub(1))
            .expect_err("over limit");
        assert!(why.contains("No partial index was built"), "{why}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn page_exposes_no_valid_prefix_after_post_open_damage() {
        let root = root("page-damage");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(18);
        let rows = two_rows(population_id);
        let mut writer = PopulationLedger::open(&root).expect("writer");
        writer
            .append_complete(&rows, &receipt(population_id, &rows))
            .expect("commit");
        drop(writer);
        let mut reader = PopulationLedger::open_read(&root).expect("reader");
        let mut raw = rows[1].to_bytes().expect("row bytes");
        *raw.get_mut(0).expect("payload byte") ^= 1;
        rewrite(
            &PopulationLedger::row_path(&root),
            super::HEADER + super::ROW_STRIDE,
            &raw,
        );
        let why = reader.page(&population_id, 0, 2).expect_err("damaged page");
        assert!(why.contains("seal"), "{why}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn v4_page_refuses_post_open_same_length_authority_mutation() {
        let root = root("v4-page-generation");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(133);
        let rows = two_rows(population_id);
        let receipt = receipt_v4(population_id, &rows);
        let mut writer = PopulationLedger::open(&root).expect("writer");
        writer
            .append_complete_v4(&rows, &receipt)
            .expect("V4 commit");
        drop(writer);
        let mut reader = PopulationLedger::open_read(&root).expect("V4 reader");
        let path = PopulationLedger::receipt_v4_path(&root);
        let original = std::fs::read(&path).expect("V4 bytes");
        let changed = original[usize::try_from(super::HEADER).expect("header")] ^ 1;
        rewrite(&path, super::HEADER, &[changed]);
        let why = reader
            .page_v4(&population_id, 0, 1)
            .expect_err("changed V4 generation cannot serve a cached page");
        assert!(why.contains("filesystem generation changed"), "{why}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn v4_page_shared_writer_lock_excludes_an_append_boundary() {
        let root = root("v4-page-shared-writer-lock");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(134);
        let rows = two_rows(population_id);
        let receipt = receipt_v4(population_id, &rows);
        let mut ledger = PopulationLedger::open(&root).expect("V4 ledger");
        ledger
            .append_complete_v4(&rows, &receipt)
            .expect("V4 commit");
        let contender = OpenOptions::new()
            .read(true)
            .write(true)
            .open(PopulationLedger::lock_path(&root))
            .expect("independent writer-lock handle");

        ledger
            .with_shared_writer_lock("the V4 page lock-boundary test", |_| {
                assert!(
                    contender.try_lock().is_err(),
                    "an exclusive population append lock must not cross the V4 page boundary"
                );
                Ok(())
            })
            .expect("shared V4 page lock boundary");
        contender
            .try_lock()
            .expect("the V4 page boundary must release its shared lock");
        contender.unlock().expect("contender unlock");
        assert_eq!(
            ledger
                .page_v4(&population_id, 0, 1)
                .expect("locked V4 page")
                .expect("V4 authority")
                .rows,
            rows[..1]
        );
        drop(ledger);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn v4_page_refuses_stale_absence_after_another_writer_appends() {
        let root = root("v4-page-stale-absence");
        let _ = std::fs::remove_dir_all(&root);
        let first_id = digest(135);
        let first_rows = two_rows(first_id);
        let mut first_writer = PopulationLedger::open(&root).expect("first writer");
        first_writer
            .append_complete_v4(&first_rows, &receipt_v4(first_id, &first_rows))
            .expect("first V4 commit");
        drop(first_writer);

        let mut stale_reader = PopulationLedger::open_read(&root).expect("reader before append");
        let next_id = digest(136);
        let next_rows = two_rows(next_id);
        let mut next_writer = PopulationLedger::open(&root).expect("second writer");
        next_writer
            .append_complete_v4(&next_rows, &receipt_v4(next_id, &next_rows))
            .expect("second V4 commit");
        drop(next_writer);

        let why = stale_reader
            .page_v4(&next_id, 0, 1)
            .expect_err("a stale reader cannot report a newly appended V4 population absent");
        assert!(why.contains("filesystem generation changed"), "{why}");
        drop(stale_reader);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn v4_page_refuses_a_receipt_file_that_appeared_after_legacy_open() {
        let root = root("v4-page-appeared-file");
        let _ = std::fs::remove_dir_all(&root);
        let population_id = digest(137);
        let rows = two_rows(population_id);
        let mut writer = PopulationLedger::open(&root).expect("legacy writer");
        writer
            .append_complete_v3(&rows, &receipt_v3(population_id, &rows, month_span()))
            .expect("V3 audit commit");
        drop(writer);
        std::fs::remove_file(PopulationLedger::receipt_v4_path(&root))
            .expect("remove empty V4 file for legacy fixture");
        let mut stale_reader = PopulationLedger::open_read(&root).expect("legacy reader");

        drop(PopulationLedger::open(&root).expect("writer recreates the V4 file"));
        let why = stale_reader
            .page_v4(&population_id, 0, 1)
            .expect_err("a newly appeared V4 file must invalidate cached legacy absence");
        assert!(
            why.contains("appeared after this population handle"),
            "{why}"
        );
        drop(stale_reader);
        let _ = std::fs::remove_dir_all(&root);
    }
}
