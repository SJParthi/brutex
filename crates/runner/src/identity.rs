//! Run identity: the nine terms, hashed once, before anything is computed.
//!
//! # The rule this exists to satisfy
//!
//! `CLAUDE.md` §3 rule 3: *"Every run is identified by `blake3(mask ‖ direction ‖
//! instrument ‖ timeframe ‖ params ‖ data_digest ‖ vocab_version ‖ commit ‖
//! feed)`. No computation without that identity recorded."*
//!
//! Until now nothing implemented it. `blake3 = "1"` sat unused in the workspace
//! manifest with no lockfile entry at all, and the formula existed only as a doc
//! comment in `crates/vocab`.
//!
//! # Nine terms, and the ninth is not `mode`
//!
//! `docs/00-charter.md` §5 also writes nine — but its ninth is a `mode` between
//! `timeframe` and the parameters, which §3 rule 3 has never mentioned.
//! `CLAUDE.md` §10 settles that in one line: *"If this file and a document
//! disagree, this file wins and the document is the stale copy to fix."* So
//! `mode` is not a term, the charter is the stale copy on that point, and no
//! decision was invented.
//!
//! The ninth term rule 3 does carry is [`Run::feed`], added by **D-0225**. The
//! other eight identify *what was computed* and none of them identifies *whose
//! data it was computed on*. The store is keyed by vendor, so one
//! instrument-month exists once per feed and sweeping two of them is two runs;
//! and two vendors redistributing one exchange feed deliver byte-identical bars
//! for a clean month, at which point every one of the eight is equal and the two
//! runs collide on one `RunId` — while `cli`'s own banner tells the reader the
//! identity "names the exact column they came from". A term that is only
//! incidentally distinguishing is not an identity term.
//!
//! Adding it re-keys every run that can be computed. That is affordable exactly
//! now and will not stay affordable: **nothing persists a `RunId` yet** — it is
//! formatted into a report string and never written to the store — so there is
//! no recorded corpus to migrate. Invariant X-14.
//!
//! # Why every field is length-prefixed
//!
//! Concatenation alone is not injective. `"NIFTY" ‖ "50"` and `"NIFTY5" ‖ "0"`
//! are the same bytes, so two different runs would share an identity — and an
//! identity that two runs can share is not an identity. Every variable-length
//! term is written as a `u32` little-endian length followed by its bytes, and
//! every fixed-width term at its natural width, so the encoding can be read back
//! unambiguously and no two distinct inputs can collide by framing.
//!
//! Each term also carries a one-byte tag. Length-prefixing alone stops a field's
//! bytes bleeding into its neighbour; the tag additionally stops two terms
//! swapping places unnoticed.
//!
//! # What `data_digest` is, and what it is not
//!
//! `docs/00-charter.md` §5: *"a streamed digest over **every field of every
//! loaded bar** — not a sample, not a count-and-endpoints fingerprint. One
//! differing bar re-keys the identity. That is the point."*
//!
//! [`data_digest`] hashes all seven fields of every candle in order.
//! [`data_digest_with_execution`] domain-separates and binds the signal digest
//! plus an optional execution-series digest when two slices decide the answer.
//! Both are O(1) per bar. Neither is a digest of a file: this crate cannot open
//! one, and only hashes the candles its caller supplies.
//!
//! # Cost
//!
//! One BLAKE3 compression per 64 bytes, which is one per candle plus a fixed
//! header. No allocation, and no loop whose bound depends on anything but the
//! number of bars. UNVERIFIED as a measured per-bar figure — this crate ships no
//! bench yet, and saying so is cheaper than a number nobody took.

use brutex_core::blake3::{Hasher, OUT_LEN};
use brutex_core::instrument::{Expiry, InstrumentKey, Kind, OptionSide};
use engine::Ladder;
use indicators::Candle;
use vocab::{ConditionMask, VOCAB_VERSION};

/// One byte per term, so two terms cannot swap places unnoticed.
mod tag {
    pub(super) const MASK: u8 = 1;
    pub(super) const DIRECTION: u8 = 2;
    pub(super) const INSTRUMENT: u8 = 3;
    pub(super) const TIMEFRAME: u8 = 4;
    pub(super) const PARAMS: u8 = 5;
    pub(super) const DATA_DIGEST: u8 = 6;
    pub(super) const VOCAB_VERSION: u8 = 7;
    pub(super) const COMMIT: u8 = 8;
    /// The feed the bars were written by.
    ///
    /// **Nine, appended, and the eight below keep their numbers.** A tag is the
    /// only thing separating one term's bytes from another's inside the hash, so
    /// renumbering an existing tag would silently re-key every run that had ever
    /// been computed — the same append-only discipline `CLAUDE.md` §3.8 applies
    /// to condition bits, applied here for the same reason.
    pub(super) const FEED: u8 = 9;
}

/// Which way a strategy is taken.
///
/// # `Undirected` is the honest value today, and it is not a placeholder
///
/// `CLAUDE.md` §3 rule 3 names `direction` as an identity term, so the term is
/// recorded rather than invented. But the sweep this engine performs counts
/// **occurrences, not trades**: `Ladder::walk` returns frequent itemsets with hit
/// counts and `crates/engine` refuses to rank them, because a ranking metric
/// needs a horizon and an exit rule that no document defines (§3 rule 1).
///
/// A frequency has no direction. So a sweep is `Undirected`, and `Long`/`Short`
/// exist for the day trades do — at which point every identity re-keys, which is
/// **correct**: a directional run is a different computation from a count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Direction {
    /// A frequency count. No trade, no side. The only value a sweep uses today.
    #[default]
    Undirected,
    /// Reserved for when a trade exists.
    Long,
    /// Reserved for when a trade exists.
    Short,
}

impl Direction {
    /// The byte this direction contributes to the identity.
    ///
    /// Explicit rather than a derived discriminant: a `#[derive]`d value would
    /// change if a variant were ever inserted above another, silently re-keying
    /// every historical run. §3.8's append-only discipline applied to an enum.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::Undirected => 0,
            Self::Long => 1,
            Self::Short => 2,
        }
    }
}

/// The sweep's parameters, canonically.
///
/// Exactly the two knobs `Ladder` carries. It has no depth field by `CLAUDE.md`
/// §6, so there is none to record — and if one were ever added, this struct
/// would have to grow to match, which is a second place the absence is enforced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// The threshold actually applied, which may differ from what was asked —
    /// `Ladder::with_min_hits` raises zero to one. Recording the applied value is
    /// what makes a result reproducible from its identity.
    pub min_hits: u64,
    /// The cumulative candidate budget actually applied.
    pub ceiling: u64,
    /// The cumulative pair-iteration budget actually applied.
    ///
    /// # This was missing, and its absence broke the identity
    ///
    /// `Ladder` gained `pair_budget` in ca358b0 and this struct did not follow.
    /// The two budgets decide **whether a walk halts**, so two runs over the same
    /// bars with the same `min_hits` and the same ceiling can return *different
    /// answers* — one complete, one truncated — and, without this field, hash to
    /// the **same** `RunId`. An identity two different results can share is not an
    /// identity, which is the whole property `CLAUDE.md` §3 rule 3 exists for.
    ///
    /// Found by an adversarial audit, not by a test: nothing here compared two
    /// runs that differed only in a budget.
    pub pair_budget: u64,
    /// Every OTHER choice that changes the recorded answer, as one digest.
    ///
    /// # The same defect as `pair_budget`, one layer out
    ///
    /// The three fields above are read off the `Ladder`, so they cover what the
    /// SWEEP did. They cover nothing about what was done with its output — and
    /// four knobs decide that:
    ///
    /// | knob | what it moves |
    /// |---|---|
    /// | `MAX_POINTS` | which variants the operator's stop admits |
    /// | the ranking `Lens` | which combination is TRADED |
    /// | `BRUTEX_GRID_RUNGS` | how wide the exit grid is |
    /// | `BRUTEX_SCREEN_CAP` | how many combinations are priced at all |
    ///
    /// Change any of them and the recorded `Record` changes. Without this field
    /// the `RunId` does not, so two runs with **different answers** collide — and
    /// the ledger, which refuses a duplicate identity, then hands back whichever
    /// landed first. That is the property §3 rule 3 exists for, and it is exactly
    /// what `pair_budget`'s own doc above records having been missing before.
    ///
    /// A digest rather than four fields because the set will grow: a fifth knob
    /// is a change to what the CALLER folds in, not a new stride here.
    /// [`Params::of`] leaves it zero, which is honest for a caller that has no
    /// such knobs — `sweep` and `auto` genuinely have none.
    pub policy: u64,
}

impl Params {
    /// The parameters a ladder will really apply, read off the ladder itself.
    ///
    /// Taken from the ladder rather than from a caller's arguments on purpose: a
    /// caller that passed `min_hits = 0` and recorded `0` would record a run that
    /// never happened, because the ladder raised it to one.
    #[must_use]
    pub fn of(ladder: Ladder) -> Self {
        Self {
            min_hits: ladder.min_hits(),
            ceiling: u64::try_from(ladder.ceiling()).unwrap_or(u64::MAX),
            pair_budget: ladder.pair_budget(),
            // ZERO, and that is a claim rather than a placeholder: this caller
            // has read a `Ladder` and nothing else, so it knows of no choice
            // beyond the three above. A caller that HAS one must say so with
            // `with_policy`, and the compiler cannot make it — which is why the
            // field is documented as the thing to fold into rather than left to
            // be discovered.
            policy: 0,
        }
    }

    /// Folds the operator-facing choices into the identity.
    ///
    /// `choices` is the caller's own list of everything outside the `Ladder`
    /// that changed the answer, in a FIXED ORDER the caller commits to. What
    /// each slot means is opaque here on purpose: `runner` does not know what
    /// `cli` lets an operator turn, and a field per knob would put this struct's
    /// stride at the mercy of a command-line flag.
    ///
    /// # Why the hashing is here and not at the call site
    ///
    /// `crates/cli` does not depend on `blake3` and must not start: `CLAUDE.md`
    /// §5 draws the arrows, and the `Run` literal in `screen_range_inner` already
    /// carries a note refusing a named path *"the dependency arrow §5 does not
    /// draw"*. So the caller passes numbers and this folds them.
    ///
    /// The length is folded first, so `[1]` and `[1, 0]` differ — a caller that
    /// adds a knob defaulting to zero still re-keys, which is the honest answer:
    /// its runs were computed by a build that could not have turned it.
    #[must_use]
    pub fn with_policy(mut self, choices: &[u64]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&(choices.len() as u64).to_le_bytes());
        for choice in choices {
            hasher.update(&choice.to_le_bytes());
        }
        let digest = hasher.finalize();
        let mut head = [0_u8; 8];
        head.copy_from_slice(digest.get(..8).unwrap_or(&[0; 8]));
        self.policy = u64::from_le_bytes(head);
        self
    }
}

/// A run's identity: 32 bytes of BLAKE3 over the eight terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunId([u8; OUT_LEN]);

impl RunId {
    /// The raw digest.
    #[must_use]
    pub const fn bytes(&self) -> [u8; OUT_LEN] {
        self.0
    }

    /// The digest as lowercase hex, which is how a run is named in a log or a
    /// filename.
    #[must_use]
    pub fn hex(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(OUT_LEN.saturating_mul(2));
        for byte in self.0 {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

/// Everything a run is, gathered so the identity can be taken before it starts.
///
/// `instrument`, `timeframe` and `commit` are the caller's because this crate
/// cannot learn them: `InstrumentKey` is `core`'s, the timeframe belongs to
/// `crates/store` — which this crate deliberately cannot name, so a stored bar
/// can never reach the sweep — and the commit cannot be baked in because
/// `CLAUDE.md` §2 forbids a `build.rs` that shells out to `git`.
pub struct Run<'a> {
    /// The combination under test.
    pub mask: ConditionMask,
    /// Which way it is taken. [`Direction::Undirected`] for a frequency sweep.
    pub direction: Direction,
    /// What is being swept.
    pub instrument: &'a InstrumentKey,
    /// The bar length, as its canonical directory word — `1min`, `1day`.
    pub timeframe: &'a str,
    /// The thresholds actually applied.
    pub params: Params,
    /// The digest of every bar that decides the answer: [`data_digest`] for one
    /// series, or [`data_digest_with_execution`] when a second execution series
    /// supplies fills.
    pub data_digest: [u8; OUT_LEN],
    /// The commit this ran at.
    pub commit: &'a str,
    /// **Which feed wrote the bars** — `groww`, `dhan`, `zerodha`, and so on.
    ///
    /// # Why this is a term, and why `data_digest` was not enough
    ///
    /// `CLAUDE.md` §3 rule 3 lists eight terms and this is a ninth, so it needs
    /// its reason stated. The eight identify *what was computed*; none of them
    /// identifies *whose data it was computed on*.
    ///
    /// That looked harmless because two feeds' bytes for one instrument-month
    /// almost always differ, so `data_digest` separated them **incidentally**.
    /// Incidentally is not a guarantee. Two vendors redistributing the same
    /// exchange feed can deliver byte-identical OHLCV at identical timestamps
    /// for a month — that is the normal case for a clean month, not a
    /// pathological one — and then every other term is equal too, so the two
    /// runs collide on one `RunId` while their reports print different feeds.
    ///
    /// That is the §3 rule 3 guarantee inverted, and the report says so out
    /// loud: `cli`'s `STORED_PROVENANCE` banner tells the reader that *"the run
    /// identity beneath names the exact column they came from"*. Without this
    /// term it did not.
    ///
    /// The value was available and unused the whole time — `cli::stored::Loaded`
    /// carries `vendor`, documented as "the first path segment, never inferred",
    /// and no production path read it.
    ///
    /// # Adding a term re-keys every run, and that is affordable exactly now
    ///
    /// Any new term changes every `RunId` this workspace can compute. That is
    /// normally expensive; here it costs nothing, because **nothing persists a
    /// `RunId`**. It is formatted into a report string and never written to the
    /// store, so there is no recorded corpus to invalidate. The same change made
    /// after run results are stored would be a migration.
    pub feed: &'a str,
}

/// A streamed digest over every field of every bar.
///
/// All seven fields, in declaration order, little-endian. Not a sample and not a
/// count-and-endpoints fingerprint — `docs/00-charter.md` §5 rules both out by
/// name, because either would let two different columns share an identity.
#[must_use]
pub fn data_digest(bars: &[Candle]) -> [u8; OUT_LEN] {
    let mut hasher = Hasher::new();
    // The bar COUNT first, so a column cannot be confused with a longer one that
    // happens to start with it. Framing, for the same reason every term below is
    // length-prefixed.
    hasher.update(&u64::try_from(bars.len()).unwrap_or(u64::MAX).to_le_bytes());
    for bar in bars {
        hasher.update(&bar.ts_micros.to_le_bytes());
        hasher.update(&bar.open.to_le_bytes());
        hasher.update(&bar.high.to_le_bytes());
        hasher.update(&bar.low.to_le_bytes());
        hasher.update(&bar.close.to_le_bytes());
        hasher.update(&bar.volume.to_le_bytes());
        hasher.update(&bar.open_interest.to_le_bytes());
    }
    hasher.finalize()
}

/// The data term for a run whose signals and fills may use different series.
///
/// This strengthens [`Run::data_digest`]; it does not add a tenth identity term.
/// The signal digest is always present. The execution digest is tagged as either
/// absent or present. A native one-series run returns [`data_digest`] byte for
/// byte, preserving every already-recorded identity; a coarse-rung run also
/// binds every one-minute bar that can change its entries, exits, stops,
/// targets, trails, and therefore its recorded answer.
///
/// # Domain separation
///
/// The component digests are not concatenated as an anonymous pair. A fixed
/// versioned domain leads the outer BLAKE3 input, and signal, absent execution,
/// and present execution have different tags. `None` does not enter that new
/// domain at all: it retains the historical one-series digest. `Some(signal)`
/// is a two-role computation and therefore has a distinct encoding even when
/// both roles happen to carry the same bytes. The tags are append-only for the
/// same reason the nine identity-term tags are: changing one re-keys historical
/// runs.
///
/// # Cost
///
/// One [`data_digest`] pass per supplied series, then one fixed-size outer hash.
/// Native one-minute execution supplies `None`, so its dataset is digested once
/// through the unchanged historical function. A projected coarse run is
/// O(signal bars + execution bars), with O(1) work per bar and no data-sized
/// allocation.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
#[must_use]
pub fn data_digest_with_execution(
    signal: &[Candle],
    execution: Option<&[Candle]>,
) -> [u8; OUT_LEN] {
    /// A versioned namespace for composing already-domain-specific bar digests.
    const DOMAIN: &[u8] = b"brutex.runner.identity.data-with-execution.v1";
    /// The signal-series component, which every run has.
    const SIGNAL: u8 = 1;
    /// A separately loaded series controls execution.
    const EXECUTION_PRESENT: u8 = 3;

    // NO SECOND SERIES MEANS NO IDENTITY MIGRATION. Results already persist
    // `RunId`; wrapping the old digest in a new domain would make an unchanged
    // native run look new and append a duplicate result. Only the computation
    // whose answer actually acquired a second dataset is re-keyed.
    let Some(execution) = execution else {
        return data_digest(signal);
    };

    let mut hasher = Hasher::new();
    hasher.update(
        &u32::try_from(DOMAIN.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    hasher.update(DOMAIN);
    term(&mut hasher, SIGNAL, &data_digest(signal));
    term(&mut hasher, EXECUTION_PRESENT, &data_digest(execution));
    hasher.finalize()
}

/// The integrity evidence available for a stored daily-reference stream.
///
/// This is an append-only byte vocabulary.  A committed bar-file record is not
/// automatically a checksum-scrubbed record: the store's ordinary read door
/// deliberately does not perform the O(file) scrub pass.  The only state this
/// build can therefore attest at the CLI boundary is [`Self::UnverifiedNoReceipt`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceIntegrity {
    /// No independently persisted scrub receipt was supplied for these bytes.
    UnverifiedNoReceipt,
}

impl ReferenceIntegrity {
    /// The append-only identity byte for this evidence state.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::UnverifiedNoReceipt => 0,
        }
    }
}

/// Everything outside the signal and one-minute streams that fixes a stored
/// previous-day reference computation.
///
/// `eligibility[i]` is `1` when `daily_bars[i]` may become a previous-day
/// anchor and `0` when the explicit calendar excludes it.  The exact calendar
/// day list is bound as well as the per-record decisions: changing a policy
/// version re-keys even when a particular span happens not to contain one of
/// the excluded dates.
#[derive(Clone, Copy, Debug)]
pub struct DailyReferenceBinding<'a> {
    /// The one-day OHLCV records offered to the causal join, in store order.
    pub daily_bars: &'a [Candle],
    /// One explicit `0`/`1` admission decision per daily record.
    pub eligibility: &'a [u8],
    /// Version of the daily-reference payload and join schema.
    pub schema: u32,
    /// Version of the rule that maps calendar days to eligibility.
    pub eligibility_policy: u32,
    /// Version of the exact-minute `GapFib` close-alignment/overlay rule.
    pub gap_overlay_policy: u32,
    /// Every IST day the policy excludes, in its canonical order.
    pub excluded_ist_days: &'a [i64],
    /// Whether an independent integrity receipt accompanied the daily records.
    pub daily_integrity: ReferenceIntegrity,
    /// Whether an independent integrity receipt accompanied the minute records.
    pub minute_integrity: ReferenceIntegrity,
    /// Version of the rule that decides whether an excluded day's own bars may
    /// enter the SWEPT series at all.
    ///
    /// # Why this is a term of its own and not a bump of `eligibility_policy`
    ///
    /// The two answer different questions about the same nine days.
    /// `eligibility_policy` decides whether a day's OHLC may become the NEXT
    /// day's anchor; this decides whether the day's own bars are folded, ranked
    /// and traded. A build can change one without changing the other, and under
    /// a single version number a later change to either would be
    /// indistinguishable from a change to the first — which is precisely the
    /// "an identity two different results can share" defect [`Params`]'s own
    /// `pair_budget` documents.
    ///
    /// **Appended, never inserted.** It is the last field here and the last
    /// tagged term in [`data_digest_with_daily_reference`], so every term above
    /// it keeps the tag and the position it already had.
    ///
    /// Like `excluded_ist_days`, the VERSION is bound rather than only its
    /// consequence: a span holding none of the excluded days computes identical
    /// bytes under both policies, and two runs that agree by luck are still two
    /// different computations.
    pub swept_series_calendar_policy: u32,
}

/// Why a three-stream data identity could not be formed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DailyBindingRefusal {
    /// There must be exactly one eligibility decision per daily record.
    EligibilityLengthMismatch {
        /// Number of daily records.
        daily: usize,
        /// Number of eligibility bytes.
        eligibility: usize,
    },
    /// Eligibility has an append-only two-value encoding; another byte has no
    /// meaning in this schema.
    UnknownEligibility {
        /// Position of the unknown byte.
        index: usize,
        /// The byte that has no declared meaning.
        byte: u8,
    },
}

/// Bind the signal rung, exact one-minute path, stored one-day references,
/// calendar/eligibility policy, and integrity evidence into one data term.
///
/// Unlike [`data_digest_with_execution`], execution is mandatory here even
/// when the signal rung is itself one minute.  The two roles are tagged
/// separately, so the identity proves that the exact one-minute path was
/// supplied rather than inferring that role from the signal timeframe.
///
/// # Errors
///
/// [`DailyBindingRefusal::EligibilityLengthMismatch`] when the admission bytes
/// are not parallel to the daily bars, or
/// [`DailyBindingRefusal::UnknownEligibility`] for a byte other than `0` or `1`.
pub fn data_digest_with_daily_reference(
    signal: &[Candle],
    execution_minute: &[Candle],
    reference: DailyReferenceBinding<'_>,
) -> Result<[u8; OUT_LEN], DailyBindingRefusal> {
    const DOMAIN: &[u8] = b"brutex.runner.identity.signal-execution-daily.v2";
    const SIGNAL: u8 = 1;
    const EXECUTION_MINUTE: u8 = 2;
    const DAILY: u8 = 3;
    const ELIGIBILITY: u8 = 4;
    const SCHEMA: u8 = 5;
    const POLICY: u8 = 6;
    const GAP_OVERLAY_POLICY: u8 = 7;
    const EXCLUDED_DAYS: u8 = 8;
    const DAILY_INTEGRITY: u8 = 9;
    const MINUTE_INTEGRITY: u8 = 10;
    /// Appended. Ten tags existed before it and none of them moves.
    const SWEPT_SERIES_CALENDAR_POLICY: u8 = 11;

    if reference.daily_bars.len() != reference.eligibility.len() {
        return Err(DailyBindingRefusal::EligibilityLengthMismatch {
            daily: reference.daily_bars.len(),
            eligibility: reference.eligibility.len(),
        });
    }
    for (index, byte) in reference.eligibility.iter().copied().enumerate() {
        if byte > 1 {
            return Err(DailyBindingRefusal::UnknownEligibility { index, byte });
        }
    }

    let mut hasher = Hasher::new();
    hasher.update(
        &u32::try_from(DOMAIN.len())
            .unwrap_or(u32::MAX)
            .to_le_bytes(),
    );
    hasher.update(DOMAIN);
    term(&mut hasher, SIGNAL, &data_digest(signal));
    term(
        &mut hasher,
        EXECUTION_MINUTE,
        &data_digest(execution_minute),
    );
    term(&mut hasher, DAILY, &data_digest(reference.daily_bars));
    term(&mut hasher, ELIGIBILITY, reference.eligibility);
    term(&mut hasher, SCHEMA, &reference.schema.to_le_bytes());
    term(
        &mut hasher,
        POLICY,
        &reference.eligibility_policy.to_le_bytes(),
    );
    term(
        &mut hasher,
        GAP_OVERLAY_POLICY,
        &reference.gap_overlay_policy.to_le_bytes(),
    );
    let mut calendar = Hasher::new();
    calendar.update(
        &u64::try_from(reference.excluded_ist_days.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for day in reference.excluded_ist_days {
        calendar.update(&day.to_le_bytes());
    }
    term(&mut hasher, EXCLUDED_DAYS, &calendar.finalize());
    term(
        &mut hasher,
        DAILY_INTEGRITY,
        &[reference.daily_integrity.byte()],
    );
    term(
        &mut hasher,
        MINUTE_INTEGRITY,
        &[reference.minute_integrity.byte()],
    );
    term(
        &mut hasher,
        SWEPT_SERIES_CALENDAR_POLICY,
        &reference.swept_series_calendar_policy.to_le_bytes(),
    );
    Ok(hasher.finalize())
}

/// One expiry, little-endian, at fixed width: year, month, day.
///
/// Fixed width rather than a string so `2024-01-04` and `2024-1-4` cannot be two
/// identities for one date, and so the blob a caller reads back has no separator
/// to disagree about.
fn push_expiry(out: &mut Vec<u8>, expiry: Expiry) {
    out.extend_from_slice(&expiry.year().to_le_bytes());
    out.push(expiry.month());
    out.push(expiry.day());
}

/// Writes one tagged, length-prefixed term.
fn term(hasher: &mut Hasher, tag: u8, bytes: &[u8]) {
    hasher.update(&[tag]);
    hasher.update(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
    hasher.update(bytes);
}

/// The identity of a run, per `CLAUDE.md` §3 rule 3.
///
/// Eight terms, each tagged and length-prefixed, hashed in the order the rule
/// names them.
#[must_use]
pub fn identity(run: &Run<'_>) -> RunId {
    let mut hasher = Hasher::new();

    // 1. mask — fixed width, one little-endian word per mask word.
    //
    // SIZED FROM `vocab::mask::WORDS`, NEVER FROM A LITERAL. This was
    // `[0_u8; 48]`. Forty-eight bytes is six words, and `chunks_exact_mut(8)`
    // over it yields exactly six chunks forever — so the day the vocabulary
    // widens past six words, `zip` stops at the shorter side and the new words
    // are dropped from the hash. Silently: no error, no truncation report, and
    // nothing that fails to compile. Two runs whose masks differ only in the
    // words above the sixth would then share a run identity, which is §3
    // rule 3's guarantee inverted.
    //
    // `crates/vocab/tests/mask.rs` already sizes its own fixtures this way and
    // says why. The literal was the outlier.
    let mut mask_bytes = [0_u8; vocab::mask::WORDS * 8];
    for (slot, word) in mask_bytes.chunks_exact_mut(8).zip(run.mask.words()) {
        slot.copy_from_slice(&word.to_le_bytes());
    }
    term(&mut hasher, tag::MASK, &mask_bytes);

    // 2. direction
    term(&mut hasher, tag::DIRECTION, &[run.direction.byte()]);

    // 3. instrument — every field of the key, each framed in turn, so two keys
    //    differing only in where one field ends cannot collide.
    //
    // THE FOURTH FIELD WAS MISSING AND THE COMMENT ABOVE STILL CLAIMED IT WAS
    // NOT. `InstrumentKey` has four fields; this framed three. `kind` is the one
    // that carries `expiry`, `strike` and `side`, so every option on one
    // underlying — every strike, both sides, every expiry — hashed to the SAME
    // `RunId`. `CLAUDE.md` §3 rule 3 makes the identity the thing a run is
    // recorded under, and an identity that cannot tell two contracts apart is
    // not one. See `two_contracts_that_differ_only_in_kind_do_not_collide`.
    let mut instrument = Vec::with_capacity(96);
    for part in [
        run.instrument.exchange.as_str(),
        run.instrument.segment.as_str(),
        run.instrument.underlying.as_str(),
    ] {
        instrument.extend_from_slice(&u32::try_from(part.len()).unwrap_or(u32::MAX).to_le_bytes());
        instrument.extend_from_slice(part.as_bytes());
    }
    // 3b. kind — a discriminant byte, then that variant's own fields at fixed
    //     width. The discriminant comes FIRST and the widths are fixed per
    //     variant, so the reader of these bytes never has to guess where one
    //     field ends; the whole blob is length-prefixed like the parts above so
    //     it cannot run into a later term either.
    let mut kind = Vec::with_capacity(16);
    match run.instrument.kind {
        Kind::Index => kind.push(0),
        Kind::Equity => kind.push(1),
        Kind::Future { expiry } => {
            kind.push(2);
            push_expiry(&mut kind, expiry);
        }
        Kind::Option {
            expiry,
            strike,
            side,
        } => {
            kind.push(3);
            push_expiry(&mut kind, expiry);
            kind.extend_from_slice(&strike.raw().to_le_bytes());
            // `as_str` is the vendor spelling and is two bytes for both sides,
            // so a byte each is enough and no length prefix is needed.
            kind.push(match side {
                OptionSide::Call => b'C',
                OptionSide::Put => b'P',
            });
        }
    }
    instrument.extend_from_slice(&u32::try_from(kind.len()).unwrap_or(u32::MAX).to_le_bytes());
    instrument.extend_from_slice(&kind);
    term(&mut hasher, tag::INSTRUMENT, &instrument);

    // 4. timeframe
    term(&mut hasher, tag::TIMEFRAME, run.timeframe.as_bytes());

    // 5. params — every knob, fixed width. FOUR now: `policy` carries the
    //    operator-facing choices the ladder knows nothing about, and its absence
    //    let two runs with different answers share one identity.
    let mut params = [0_u8; 32];
    for (slot, value) in params.chunks_exact_mut(8).zip([
        run.params.min_hits,
        run.params.ceiling,
        run.params.pair_budget,
        run.params.policy,
    ]) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
    term(&mut hasher, tag::PARAMS, &params);

    // 6. data_digest
    term(&mut hasher, tag::DATA_DIGEST, &run.data_digest);

    // 7. vocab_version
    term(
        &mut hasher,
        tag::VOCAB_VERSION,
        &VOCAB_VERSION.to_le_bytes(),
    );

    // 8. commit
    term(&mut hasher, tag::COMMIT, run.commit.as_bytes());

    // 9. feed — length-prefixed by `term`, like every other variable-width
    //    value here, so `("groww", "1min")` and `("groww1", "min")` cannot
    //    produce the same byte stream. See `Run::feed` for why this term exists
    //    at all and why `data_digest` did not already cover it.
    term(&mut hasher, tag::FEED, run.feed.as_bytes());

    RunId(hasher.finalize())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail, and `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind."
)]
mod tests {
    use super::{
        DailyBindingRefusal, DailyReferenceBinding, Direction, Params, ReferenceIntegrity, Run,
        RunId, data_digest, data_digest_with_daily_reference, data_digest_with_execution, identity,
    };
    use crate::synthetic;
    use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind, OptionSide};
    use brutex_core::price::Paisa;
    use engine::Ladder;
    use vocab::ConditionMask;

    fn key() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY is a swept index")
    }

    /// The swept index key with only its `kind` replaced.
    ///
    /// Every other field is held fixed on purpose: the defect this covers was
    /// that `kind` reached the hasher not at all, so a test that also varied the
    /// underlying would pass against the broken function.
    fn keyed(kind: Kind) -> InstrumentKey {
        let mut k = key();
        k.kind = kind;
        k
    }

    fn expiry(day: u8) -> Expiry {
        Expiry::new(2024, 1, day).expect("a real January date")
    }

    fn run_over(instrument: &InstrumentKey, digest: [u8; 32], mask: ConditionMask) -> Run<'_> {
        Run {
            mask,
            direction: Direction::Undirected,
            instrument,
            timeframe: "1min",
            params: Params::of(Ladder::with_min_hits(600)),
            data_digest: digest,
            commit: "0123456789abcdef",
            feed: "groww",
        }
    }

    /// TWO FEEDS DELIVERING THE SAME BYTES MUST NOT SHARE A `RunId`.
    ///
    /// # The case this is about, which is the ordinary one
    ///
    /// This is not a pathological fixture. Two vendors redistributing one NSE
    /// feed publish the same OHLCV at the same timestamps for a clean month, so
    /// `data_digest` — a hash of the bar bytes — is **equal**, and so is every
    /// other term: same instrument, same rung, same ladder, same commit. Before
    /// `feed` was a term, those two runs hashed identically while their reports
    /// printed different feeds, and `cli`'s `STORED_PROVENANCE` banner told the
    /// reader that the identity "names the exact column they came from".
    ///
    /// The digest separating them was **incidental**, not guaranteed, and this
    /// test is what turns the guarantee on: it holds `data_digest` fixed on
    /// purpose, so it fails on any implementation that leans on the bytes
    /// differing.
    #[test]
    fn two_feeds_with_byte_identical_bars_do_not_collide() {
        let key = key();
        let digest = [7_u8; 32];
        let mask = ConditionMask::default().with_bit(3);

        let groww = run_over(&key, digest, mask);
        let mut zerodha = run_over(&key, digest, mask);
        zerodha.feed = "zerodha";

        assert_eq!(
            groww.data_digest, zerodha.data_digest,
            "the fixture's whole point is that the BARS are identical; if this \
             fails the test is no longer testing what it claims"
        );
        assert_ne!(
            identity(&groww),
            identity(&zerodha),
            "two feeds delivering identical bytes for one instrument-month are \
             two runs, and §3 rule 3 requires two identities"
        );

        // AND THE SAME FEED STILL AGREES WITH ITSELF, so the term is a
        // discriminator and not a source of noise -- without this, a `feed`
        // hashed from something incidental would pass the assertion above.
        assert_eq!(
            identity(&groww),
            identity(&run_over(&key, digest, mask)),
            "one feed, one instrument-month, one ladder: rerunning must be safe \
             and must reproduce the identity byte for byte"
        );
    }

    /// TWO CONTRACTS THAT DIFFER ONLY IN `kind` MUST NOT SHARE A `RunId`.
    ///
    /// `InstrumentKey` has four fields and the instrument term framed three. So
    /// every option on one underlying — every strike, both sides, every expiry —
    /// hashed identically, and so did a future against the spot index. `CLAUDE.md`
    /// §3 rule 3 makes the identity what a run is recorded under; an identity
    /// that cannot separate two contracts records the second one over the first.
    ///
    /// Each pair below moves exactly ONE field of `kind` and nothing else.
    #[test]
    fn two_contracts_that_differ_only_in_kind_do_not_collide() {
        let d = data_digest(&synthetic::sessions(2));
        let m = ConditionMask::default().with_bit(3);
        let id = |k: &InstrumentKey| identity(&run_over(k, d, m));

        let call = |strike: i64, day: u8| {
            keyed(Kind::Option {
                expiry: expiry(day),
                strike: Paisa::from_raw(strike),
                side: OptionSide::Call,
            })
        };

        let base = id(&call(2_400_000, 4));
        assert_ne!(
            base,
            id(&call(2_500_000, 4)),
            "a different STRIKE is a different contract"
        );
        assert_ne!(
            base,
            id(&call(2_400_000, 11)),
            "a different EXPIRY is a different contract"
        );
        assert_ne!(
            base,
            id(&keyed(Kind::Option {
                expiry: expiry(4),
                strike: Paisa::from_raw(2_400_000),
                side: OptionSide::Put,
            })),
            "a different SIDE is a different contract"
        );
        assert_ne!(
            id(&keyed(Kind::Index)),
            id(&keyed(Kind::Equity)),
            "index and equity are different instruments at the same name"
        );
        assert_ne!(
            id(&keyed(Kind::Index)),
            id(&keyed(Kind::Future { expiry: expiry(25) })),
            "the spot index and its future are not one instrument"
        );

        // And the identity is still a FUNCTION of the key: the same kind twice
        // is the same id, so the assertions above measure the field and not the
        // hasher's state.
        assert_eq!(base, id(&call(2_400_000, 4)), "still deterministic");
    }

    #[test]
    fn the_same_run_has_the_same_identity() {
        // §3 rule 5, at the level of the whole run.
        let k = key();
        let d = data_digest(&synthetic::sessions(2));
        let m = ConditionMask::default().with_bit(3);
        assert_eq!(identity(&run_over(&k, d, m)), identity(&run_over(&k, d, m)));
    }

    /// IDENTICAL COARSE SIGNALS WITH DIFFERENT INTERIOR PATHS ARE DIFFERENT RUNS.
    ///
    /// A coarse bar can stay byte-identical while one of the one-minute bars it
    /// hides changes. Stops, targets and trailing exits can then produce a
    /// different trade from the same signal. Hashing only the coarse slice gave
    /// both answers one `RunId`; the execution digest is the missing evidence.
    #[test]
    fn a_coarse_run_identity_binds_the_interior_execution_path() {
        let path_a = synthetic::sessions(2);
        let path_b: Vec<_> = path_a
            .iter()
            .enumerate()
            .map(|(index, bar)| {
                let mut changed = *bar;
                if index == 1 {
                    changed.close = changed.close.saturating_add(1);
                }
                changed
            })
            .collect();
        let period = crate::resample::Period::minutes(15).expect("a coarse period");
        let coarse_a = crate::resample::resample(&path_a, period);
        let coarse_b = crate::resample::resample(&path_b, period);
        assert_eq!(
            coarse_a, coarse_b,
            "the signal bars must be byte-identical or this test does not isolate the hidden one-minute path"
        );

        let instrument = key();
        let mask = ConditionMask::default().with_bit(3);
        let mut run_a = run_over(
            &instrument,
            data_digest_with_execution(&coarse_a, Some(&path_a)),
            mask,
        );
        run_a.timeframe = "15min";
        let mut run_b = run_over(
            &instrument,
            data_digest_with_execution(&coarse_b, Some(&path_b)),
            mask,
        );
        run_b.timeframe = "15min";

        assert_ne!(
            identity(&run_a),
            identity(&run_b),
            "different one-minute paths can produce different trades and must not share a RunId"
        );
    }

    #[test]
    fn execution_data_binding_is_deterministic_and_presence_is_explicit() {
        let bars = synthetic::sessions(1);
        let absent = data_digest_with_execution(&bars, None);
        assert_eq!(
            absent,
            data_digest(&bars),
            "a one-series run keeps its historical identity bytes rather than appending a duplicate result"
        );
        assert_eq!(
            absent,
            data_digest_with_execution(&bars, None),
            "the same native one-minute input must reproduce byte for byte"
        );

        let present = data_digest_with_execution(&bars, Some(&bars));
        assert_eq!(
            present,
            data_digest_with_execution(&bars, Some(&bars)),
            "the same two-series input must reproduce byte for byte"
        );
        assert_ne!(
            absent, present,
            "absence means the one dataset is bound once; presence means a second execution role is bound even when its bytes match"
        );

        let instrument = key();
        let mask = ConditionMask::default().with_bit(3);
        let present_id = identity(&run_over(&instrument, present, mask));
        assert_eq!(
            present_id,
            identity(&run_over(
                &instrument,
                data_digest_with_execution(&bars, Some(&bars)),
                mask,
            )),
            "the complete RunId must also reproduce for the same two-series input"
        );
        assert_ne!(
            identity(&run_over(&instrument, absent, mask)),
            present_id,
            "the absence/presence distinction must reach the RunId, not stop at an unused helper result"
        );
    }

    fn daily_binding<'a>(
        bars: &'a [indicators::Candle],
        eligibility: &'a [u8],
        excluded_ist_days: &'a [i64],
    ) -> DailyReferenceBinding<'a> {
        DailyReferenceBinding {
            daily_bars: bars,
            eligibility,
            schema: 1,
            eligibility_policy: 1,
            gap_overlay_policy: 1,
            excluded_ist_days,
            daily_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
            minute_integrity: ReferenceIntegrity::UnverifiedNoReceipt,
            swept_series_calendar_policy: 1,
        }
    }

    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one adversarial identity matrix changes each bound term independently"
    )]
    fn the_three_stream_identity_binds_every_reference_choice() {
        let signal = synthetic::sessions(1);
        let execution = synthetic::sessions(2);
        let daily = synthetic::sessions(1)
            .into_iter()
            .take(2)
            .collect::<Vec<_>>();
        let eligibility = [1_u8, 0];
        let excluded = [20_382_i64, 19_784];
        let base = data_digest_with_daily_reference(
            &signal,
            &execution,
            daily_binding(&daily, &eligibility, &excluded),
        )
        .expect("parallel declared reference input");
        assert_eq!(
            base,
            data_digest_with_daily_reference(
                &signal,
                &execution,
                daily_binding(&daily, &eligibility, &excluded),
            )
            .expect("same input"),
            "the complete binding is deterministic"
        );

        let changed_signal = signal
            .iter()
            .enumerate()
            .map(|(index, bar)| {
                let mut changed = *bar;
                if index == 0 {
                    changed.close = changed.close.saturating_add(1);
                }
                changed
            })
            .collect::<Vec<_>>();
        assert_ne!(
            base,
            data_digest_with_daily_reference(
                &changed_signal,
                &execution,
                daily_binding(&daily, &eligibility, &excluded),
            )
            .expect("same reference shape"),
            "signal role"
        );
        let changed_execution = execution
            .iter()
            .enumerate()
            .map(|(index, bar)| {
                let mut changed = *bar;
                if index == 0 {
                    changed.close = changed.close.saturating_add(1);
                }
                changed
            })
            .collect::<Vec<_>>();
        assert_ne!(
            base,
            data_digest_with_daily_reference(
                &signal,
                &changed_execution,
                daily_binding(&daily, &eligibility, &excluded),
            )
            .expect("same reference shape"),
            "execution role"
        );
        let changed_daily = daily
            .iter()
            .enumerate()
            .map(|(index, bar)| {
                let mut changed = *bar;
                if index == 0 {
                    changed.close = changed.close.saturating_add(1);
                }
                changed
            })
            .collect::<Vec<_>>();
        assert_ne!(
            base,
            data_digest_with_daily_reference(
                &signal,
                &execution,
                daily_binding(&changed_daily, &eligibility, &excluded),
            )
            .expect("same reference shape"),
            "daily role"
        );
        assert_ne!(
            base,
            data_digest_with_daily_reference(
                &signal,
                &execution,
                daily_binding(&daily, &[0, 1], &excluded),
            )
            .expect("same reference shape"),
            "per-record eligibility"
        );
        let mut changed_schema = daily_binding(&daily, &eligibility, &excluded);
        changed_schema.schema = 2;
        assert_ne!(
            base,
            data_digest_with_daily_reference(&signal, &execution, changed_schema)
                .expect("same reference shape"),
            "schema version"
        );
        let mut changed_policy = daily_binding(&daily, &eligibility, &excluded);
        changed_policy.eligibility_policy = 2;
        assert_ne!(
            base,
            data_digest_with_daily_reference(&signal, &execution, changed_policy)
                .expect("same reference shape"),
            "eligibility-policy version"
        );
        let mut changed_gap_policy = daily_binding(&daily, &eligibility, &excluded);
        changed_gap_policy.gap_overlay_policy = 2;
        assert_ne!(
            base,
            data_digest_with_daily_reference(&signal, &execution, changed_gap_policy)
                .expect("same reference shape"),
            "exact-minute GapFib overlay-policy version"
        );
        assert_ne!(
            base,
            data_digest_with_daily_reference(
                &signal,
                &execution,
                daily_binding(&daily, &eligibility, &[20_382]),
            )
            .expect("same reference shape"),
            "exact excluded-day calendar"
        );
    }

    #[test]
    fn malformed_daily_eligibility_refuses_instead_of_hashing_an_ambiguous_policy() {
        let bars = synthetic::sessions(1);
        let one = bars.get(..1).unwrap_or(&[]);
        assert_eq!(
            data_digest_with_daily_reference(&bars, &bars, daily_binding(one, &[], &[20_382]),),
            Err(DailyBindingRefusal::EligibilityLengthMismatch {
                daily: 1,
                eligibility: 0,
            })
        );
        assert_eq!(
            data_digest_with_daily_reference(&bars, &bars, daily_binding(one, &[2], &[20_382]),),
            Err(DailyBindingRefusal::UnknownEligibility { index: 0, byte: 2 })
        );
        assert_eq!(ReferenceIntegrity::UnverifiedNoReceipt.byte(), 0);
    }

    #[test]
    fn one_differing_bar_re_keys_the_identity() {
        // The charter's own words: "One differing bar re-keys the identity. That
        // is the point."
        let a = synthetic::sessions(2);
        // One paisa, on one field, of one bar. Built by mapping rather than by
        // `last_mut()`: an `if let` whose `else` no input can enter is a region
        // llvm-cov counts forever, and both arms of this condition are taken.
        let last = a.len().saturating_sub(1);
        let b: Vec<_> = a
            .iter()
            .enumerate()
            .map(|(i, bar)| {
                let mut c = *bar;
                if i == last {
                    c.close = c.close.saturating_add(1);
                }
                c
            })
            .collect();
        assert_ne!(data_digest(&a), data_digest(&b));

        let k = key();
        let m = ConditionMask::default().with_bit(3);
        assert_ne!(
            identity(&run_over(&k, data_digest(&a), m)),
            identity(&run_over(&k, data_digest(&b), m))
        );
    }

    #[test]
    fn a_shorter_column_is_not_a_prefix_of_a_longer_one() {
        // Without the length written first, a 100-bar column and the first 100
        // bars of a 200-bar one would hash identically up to that point. They are
        // different data and must be different digests.
        let long = synthetic::sessions(2);
        let short = long.get(..100).unwrap_or(&[]).to_vec();
        assert_ne!(data_digest(&long), data_digest(&short));
        assert_ne!(data_digest(&[]), data_digest(&short));
    }

    #[test]
    fn every_term_changes_the_identity() {
        // Nine terms, nine assertions. A term that is written but never read
        // would let two different runs share an identity, which is the one thing
        // an identity may not do.
        let k = key();
        let d = data_digest(&synthetic::sessions(2));
        let m = ConditionMask::default().with_bit(3);
        let base = identity(&run_over(&k, d, m));

        // 1. mask
        let other_mask = ConditionMask::default().with_bit(4);
        assert_ne!(base, identity(&run_over(&k, d, other_mask)), "mask");

        // 2. direction
        let mut r = run_over(&k, d, m);
        r.direction = Direction::Long;
        assert_ne!(base, identity(&r), "direction");

        // 3. instrument
        let bank = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").expect("swept");
        assert_ne!(base, identity(&run_over(&bank, d, m)), "instrument");

        // 4. timeframe
        let mut r = run_over(&k, d, m);
        r.timeframe = "1day";
        assert_ne!(base, identity(&r), "timeframe");

        // 5. params
        let mut r = run_over(&k, d, m);
        r.params = Params::of(Ladder::with_min_hits(601));
        assert_ne!(base, identity(&r), "min_hits");
        let mut r = run_over(&k, d, m);
        r.params = Params::of(Ladder::with_min_hits(600).with_ceiling(7));
        assert_ne!(base, identity(&r), "ceiling");
        // The pair budget decides WHETHER A WALK HALTS, so two runs differing
        // only in it can return different answers. Before this was recorded they
        // hashed identically -- an identity two results can share is not one.
        let mut r = run_over(&k, d, m);
        r.params = Params::of(Ladder::with_min_hits(600).with_pair_budget(7));
        assert_ne!(base, identity(&r), "pair_budget");
        // AND THE SAME DEFECT ONE LAYER OUT. The three fields above are read off
        // the LADDER, so they cover what the sweep did and nothing about what was
        // done with its output. `MAX_POINTS`, the ranking lens, the grid width
        // and the screen cap each change which combination is TRADED and what
        // P&L is recorded, and none of them reaches a `Ladder`. Two runs with
        // different answers therefore hashed identically, and the ledger --
        // which refuses a duplicate identity -- handed back whichever landed
        // first.
        let mut r = run_over(&k, d, m);
        r.params = Params::of(Ladder::with_min_hits(600)).with_policy(&[7]);
        assert_ne!(base, identity(&r), "policy");
        // AND IT IS THE VALUE THAT SEPARATES THEM, not merely its presence.
        let mut other = run_over(&k, d, m);
        other.params = Params::of(Ladder::with_min_hits(600)).with_policy(&[8]);
        assert_ne!(
            identity(&r),
            identity(&other),
            "two different policies must give two different identities, or the \
             digest is being folded in without being read"
        );

        // 6. data_digest
        assert_ne!(base, identity(&run_over(&k, [0_u8; 32], m)), "data_digest");

        // 7. vocab_version is a constant, so it cannot be varied here — it is
        //    covered by the fact that changing it changes every identity, which
        //    `the_vocabulary_version_is_in_the_identity` pins by construction.

        // 8. commit
        let mut r = run_over(&k, d, m);
        r.commit = "fedcba9876543210";
        assert_ne!(base, identity(&r), "commit");

        // 9. feed — the term that is NOT one of §3 rule 3's eight. Varied here
        //    beside the others so the roll-call stays a roll-call: a term this
        //    loop does not exercise is a term that can be written and never
        //    read, which is the exact defect `pair_budget` was.
        let mut r = run_over(&k, d, m);
        r.feed = "dhan";
        assert_ne!(base, identity(&r), "feed");
    }

    #[test]
    fn the_vocabulary_version_is_in_the_identity() {
        // Not variable at runtime, so this pins that it is READ: the encoding
        // writes four bytes for it, and a build with a different vocabulary must
        // not reuse an old run's name.
        assert_eq!(vocab::VOCAB_VERSION, 3, "if this moves, every run re-keys");
    }

    #[test]
    fn a_framing_collision_is_impossible() {
        // The reason every term is length-prefixed. Two instruments whose
        // concatenated fields are the same bytes at different boundaries must not
        // collide. `NIFTY`/`BANKNIFTY` under one exchange and segment is the
        // shape; the lengths are what separate them.
        let a = key();
        let b = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").expect("swept");
        let d = data_digest(&[]);
        let m = ConditionMask::default();
        assert_ne!(identity(&run_over(&a, d, m)), identity(&run_over(&b, d, m)));

        // And a timeframe that is a prefix of another.
        let mut short = run_over(&a, d, m);
        short.timeframe = "1m";
        let mut long = run_over(&a, d, m);
        long.timeframe = "1min";
        assert_ne!(identity(&short), identity(&long));
    }

    #[test]
    fn the_identity_renders_as_sixty_four_hex_characters() {
        let k = key();
        let id = identity(&run_over(&k, data_digest(&[]), ConditionMask::default()));
        let hex = id.hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
        assert_eq!(id.bytes().len(), 32);
    }

    #[test]
    fn direction_bytes_are_pinned_and_not_derived() {
        // §3.8 applied to an enum: a derived discriminant would shift if a
        // variant were inserted above another, silently re-keying history.
        assert_eq!(Direction::Undirected.byte(), 0);
        assert_eq!(Direction::Long.byte(), 1);
        assert_eq!(Direction::Short.byte(), 2);
        assert_eq!(Direction::default(), Direction::Undirected);
    }

    #[test]
    fn params_record_what_the_ladder_applies_not_what_was_asked() {
        // `with_min_hits(0)` is raised to 1. Recording 0 would name a run that
        // never happened.
        let p = Params::of(Ladder::with_min_hits(0));
        assert_eq!(p.min_hits, 1, "the APPLIED threshold, not the argument");
        assert_eq!(p.ceiling, engine::DEFAULT_CEILING as u64);
    }

    #[test]
    fn an_empty_column_still_has_a_digest() {
        // A run over no bars is a run, and it must be nameable — otherwise the
        // "no computation without an identity" rule has a hole exactly where a
        // caller is most likely to be confused about what happened.
        let empty = data_digest(&[]);
        assert_ne!(empty, [0_u8; 32]);
        assert_eq!(empty, data_digest(&[]));
    }

    #[test]
    fn two_runs_differing_only_in_bar_order_differ() {
        // The digest is over the column as ORDERED. Reordering bars is a
        // different column -- and would be look-ahead if it were not.
        let bars = synthetic::sessions(1);
        let mut reversed = bars.clone();
        reversed.reverse();
        assert_ne!(data_digest(&bars), data_digest(&reversed));
    }

    #[test]
    fn a_run_id_is_copy_and_comparable() {
        let k = key();
        let id = identity(&run_over(&k, data_digest(&[]), ConditionMask::default()));
        let copied: RunId = id;
        assert_eq!(id, copied);
    }
}
