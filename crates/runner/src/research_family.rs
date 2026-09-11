//! Explicit research families without reinterpreting legacy two-index records.
//!
//! Cash eligibility is the existing F&O/total-market snapshot intersection.
//! It is not historical point-in-time membership or institutional admission.
//! This capability names scope only: stored receipts, policy and completed
//! statistical/execution authorities remain separate requirements.

use brutex_core::blake3::Hasher;
use brutex_core::instrument::{Exchange, InstrumentKey, Kind, Segment};
use brutex_core::universe::{FNO_INDEX, FNO_UNDERLYINGS, NIFTY_TOTAL_MARKET, NTM_INDEX};
use std::fmt;
use std::sync::OnceLock;

use crate::exit_grid_policy::{InstrumentFamilyV1, instrument_digest_v1};

/// Fixed width of the new scope-only family encoding.
pub const RESEARCH_FAMILY_BYTES_V1: usize = 128;
/// Physical membership-slot ceiling, derived from the authoritative tables.
pub const RESEARCH_FAMILY_CAPACITY_V1: usize = FNO_UNDERLYINGS.len() + InstrumentKey::SWEPT.len();

/// Explicit refusal at a family or scope boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResearchFamilyErrorV1 {
    /// Not an exact approved index or eligible NSE cash-stock key.
    UnsupportedInstrument,
    /// A campaign must name at least one family.
    EmptyScope,
    /// Offered family count exceeds the explicit or physical limit.
    ScopeLimitExceeded {
        /// Number of offered instrument keys.
        requested: usize,
        /// Smaller of the explicit limit and physical membership capacity.
        maximum: usize,
    },
    /// A family appeared more than once; no entry was silently discarded.
    DuplicateInstrument,
    /// The bounded family vector could not be allocated.
    AllocationRefused,
    /// Wrong width, version, padding, symbol, identity or membership snapshot.
    InvalidEncoding,
}

impl fmt::Display for ResearchFamilyErrorV1 {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedInstrument => out.write_str("unsupported research instrument; require an exact NSE index or F&O/total-market cash intersection member"),
            Self::EmptyScope => out.write_str("research scope is empty"),
            Self::ScopeLimitExceeded { requested, maximum } => write!(out, "research scope offers {requested} families beyond its {maximum} family limit"),
            Self::DuplicateInstrument => out.write_str("research scope repeats an instrument"),
            Self::AllocationRefused => out.write_str("research scope allocation refused"),
            Self::InvalidEncoding => out.write_str("research family V1 encoding or membership snapshot is invalid"),
        }
    }
}

impl std::error::Error for ResearchFamilyErrorV1 {}

/// Scope-only proof of one complete instrument identity.
///
/// Index mapping is available only for the two actual legacy indices. A cash
/// family has no legacy mapping and cannot be passed off as either index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResearchFamilyV1 {
    instrument: InstrumentKey,
    tag: u8,
    slot: usize,
    membership_digest: [u8; 32],
    digest: [u8; 32],
}

impl ResearchFamilyV1 {
    /// Validate a runtime key against the existing research surface.
    ///
    /// # Errors
    /// Refuses derivatives, references, other exchanges, mismatched key shapes,
    /// nonmembers and F&O index names masquerading as cash stocks.
    pub fn new(instrument: InstrumentKey) -> Result<Self, ResearchFamilyErrorV1> {
        let (tag, slot) = classify(&instrument)?;
        let membership_digest = if tag == 3 {
            membership_snapshot_digest_v1()
        } else {
            [0; 32]
        };
        let mut hash = Hasher::new();
        hash.update(b"brutex.runner.research-family.v1\0");
        hash.update(&[tag]);
        hash.update(&instrument_digest_v1(&instrument));
        hash.update(&membership_digest);
        Ok(Self {
            instrument,
            tag,
            slot,
            membership_digest,
            digest: hash.finalize(),
        })
    }

    /// Full canonical key; a family label never substitutes for it.
    #[must_use]
    pub const fn instrument(self) -> InstrumentKey {
        self.instrument
    }

    /// New family identity, separate from every legacy run/record identity.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }

    /// Cash membership snapshot; zero means not applicable to a legacy index.
    #[must_use]
    pub const fn membership_digest(self) -> [u8; 32] {
        self.membership_digest
    }

    /// Whether this names the stock's cash series rather than a spot index.
    #[must_use]
    pub const fn is_cash(self) -> bool {
        self.tag == 3
    }

    /// Exact optional mapping, preserving both existing V1 family values.
    #[must_use]
    pub const fn legacy_index_family(self) -> Option<InstrumentFamilyV1> {
        match self.tag {
            1 => Some(InstrumentFamilyV1::Nifty),
            2 => Some(InstrumentFamilyV1::BankNifty),
            _ => None,
        }
    }

    /// Canonical new fixed record. This is scope, not a stored-data receipt.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "literal codec ranges fit the fixed 128-byte output; the symbol type bounds its sole variable copy to 24 bytes"
    )]
    pub fn encode(self) -> [u8; RESEARCH_FAMILY_BYTES_V1] {
        let mut out = [0; RESEARCH_FAMILY_BYTES_V1];
        out[..8].copy_from_slice(b"BTX-FAM1");
        out[8] = self.tag;
        let symbol = self.instrument.underlying.as_str().as_bytes();
        out[16..16 + symbol.len()].copy_from_slice(symbol);
        out[40..72].copy_from_slice(&self.membership_digest);
        out[72..104].copy_from_slice(&self.digest);
        out
    }

    /// Decode by rebuilding current eligibility and comparing every byte.
    ///
    /// # Errors
    /// Refuses noncanonical bytes and stale cash membership identities.
    pub fn decode(bytes: &[u8]) -> Result<Self, ResearchFamilyErrorV1> {
        let invalid = ResearchFamilyErrorV1::InvalidEncoding;
        let bytes: &[u8; RESEARCH_FAMILY_BYTES_V1] = bytes.try_into().map_err(|_| invalid)?;
        let name = bytes.get(16..40).ok_or(invalid)?;
        let length = name
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(name.len());
        let name = std::str::from_utf8(name.get(..length).ok_or(invalid)?).map_err(|_| invalid)?;
        let instrument = match bytes.get(8) {
            Some(1 | 2) => InstrumentKey::index(Exchange::Nse, name),
            Some(3) => InstrumentKey::cash(Exchange::Nse, name),
            _ => return Err(invalid),
        }
        .map_err(|_| invalid)?;
        let family = Self::new(instrument).map_err(|_| invalid)?;
        // The typed width above and this exact re-encoding jointly validate the
        // format, including its magic, every reserved byte and both identities.
        if family.encode().as_slice() != bytes.as_slice() {
            return Err(invalid);
        }
        Ok(family)
    }
}

fn classify(instrument: &InstrumentKey) -> Result<(u8, usize), ResearchFamilyErrorV1> {
    let refused = ResearchFamilyErrorV1::UnsupportedInstrument;
    if instrument.exchange != Exchange::Nse {
        return Err(refused);
    }
    let symbol = instrument.underlying.as_str();
    match (instrument.segment, instrument.kind) {
        (Segment::Index, Kind::Index) => match symbol {
            "NIFTY" => Ok((1, 0)),
            "BANKNIFTY" => Ok((2, 1)),
            _ => Err(refused),
        },
        (Segment::Cash, Kind::Equity) if NTM_INDEX.contains(symbol) => {
            let slot = FNO_INDEX.position(symbol).ok_or(refused)?;
            Ok((3, slot + InstrumentKey::SWEPT.len()))
        }
        _ => Err(refused),
    }
}

/// Identity of the two existing ordered membership tables, not a dated history.
///
/// The first call hashes the finite tables once. Later calls copy 32 bytes;
/// membership probes and duplicate rejection never scan either table.
#[must_use]
pub fn membership_snapshot_digest_v1() -> [u8; 32] {
    static DIGEST: OnceLock<[u8; 32]> = OnceLock::new();
    *DIGEST.get_or_init(|| {
        let mut hash = Hasher::new();
        hash.update(b"brutex.runner.cash-membership-snapshot.v1\0FNO\0");
        for name in FNO_UNDERLYINGS {
            hash.update(name.as_bytes());
            hash.update(&[0]);
        }
        hash.update(b"NTM\0");
        for name in NIFTY_TOTAL_MARKET {
            hash.update(name.as_bytes());
            hash.update(&[0]);
        }
        hash.finalize()
    })
}

/// An explicit, bounded, canonical cohort of runtime research families.
///
/// Constructing/sorting a scope costs O(F log F) and retains O(F) families.
/// Membership, ordinal mapping and duplicate rejection use fixed table slots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResearchScopeV1 {
    families: Vec<ResearchFamilyV1>,
    positions: [Option<usize>; RESEARCH_FAMILY_CAPACITY_V1],
    digest: [u8; 32],
}

impl ResearchScopeV1 {
    /// Admit exactly the supplied instruments, with no default or truncation.
    ///
    /// # Errors
    /// Refuses empty, duplicate, unsupported, over-budget or unallocatable scope.
    pub fn new(
        instruments: &[InstrumentKey],
        max_members: usize,
    ) -> Result<Self, ResearchFamilyErrorV1> {
        if instruments.is_empty() {
            return Err(ResearchFamilyErrorV1::EmptyScope);
        }
        let maximum = max_members.min(RESEARCH_FAMILY_CAPACITY_V1);
        if instruments.len() > maximum {
            return Err(ResearchFamilyErrorV1::ScopeLimitExceeded {
                requested: instruments.len(),
                maximum,
            });
        }
        let mut families = Vec::new();
        families
            .try_reserve_exact(instruments.len())
            .map_err(|_| ResearchFamilyErrorV1::AllocationRefused)?;
        let mut positions = [None; RESEARCH_FAMILY_CAPACITY_V1];
        for instrument in instruments {
            let family = ResearchFamilyV1::new(*instrument)?;
            let slot = positions
                .get_mut(family.slot)
                .ok_or(ResearchFamilyErrorV1::UnsupportedInstrument)?;
            if slot.replace(families.len()).is_some() {
                return Err(ResearchFamilyErrorV1::DuplicateInstrument);
            }
            families.push(family);
        }
        families.sort_unstable_by_key(|family| (family.tag, family.instrument.underlying));
        let mut hash = Hasher::new();
        hash.update(b"brutex.runner.research-scope.v1\0");
        for (position, family) in families.iter().enumerate() {
            *positions
                .get_mut(family.slot)
                .ok_or(ResearchFamilyErrorV1::UnsupportedInstrument)? = Some(position);
            hash.update(&family.encode());
        }
        Ok(Self {
            families,
            positions,
            digest: hash.finalize(),
        })
    }

    /// Canonical order: NIFTY, BANKNIFTY, then cash symbols alphabetically.
    #[must_use]
    pub fn families(&self) -> &[ResearchFamilyV1] {
        &self.families
    }

    /// Identity of the exact canonical full-width scope, independent of input order.
    #[must_use]
    pub const fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Fixed-slot membership; malformed or storage-only keys never match.
    #[must_use]
    pub fn contains(&self, instrument: &InstrumentKey) -> bool {
        self.position(instrument).is_some()
    }

    /// Constant lookup of the canonical in-scope ordinal. This ordinal is not
    /// a persisted family identity; callers must retain the full family digest.
    #[must_use]
    pub fn position(&self, instrument: &InstrumentKey) -> Option<usize> {
        classify(instrument)
            .ok()
            .and_then(|(_, slot)| self.positions.get(slot).copied().flatten())
    }
}
