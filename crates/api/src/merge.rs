//! Merging every vendor's master into one map, and cross-checking it.
//!
//! # What the merge is
//!
//! Two vendors naming the same contract produce the same
//! [`brutex_core::instrument::InstrumentKey`], so they collide
//! by design and that collision **is** the deduplication —
//! `docs/05-decisions.md` D-0015. This module is where the collision happens
//! and where the two vendors are made to agree about what collided.
//!
//! # The cross-check, and why the ISIN is not simply part of the key
//!
//! The ISIN is the one field a broker does not mint: it comes from a national
//! numbering agency, so both masters carry the same string for the same paper.
//! That makes it the ideal check on a merge — and a catastrophic key
//! component. As a field of the key, a vendor disagreement would produce two
//! different keys, the map would hold the instrument twice, and nobody would
//! ever learn there was a disagreement. Beside the key, the same disagreement
//! is one loud line naming the key and both ISINs. See [`brutex_core::isin`].
//!
//! # Two kinds of disagreement, and only one of them used to be looked for
//!
//! A merge that compares the ISINs of rows **both vendors kept** cannot see
//! the disagreement that actually existed in this data: one vendor calling a
//! security an equity while the other called it a bond. Those rows never
//! reached the merge at all — the reader counted them and dropped them — so
//! the report printed `0 isin conflicts` while the two masters disagreed about
//! the eligibility of 62 instruments, every one of which carried the **same
//! ISIN in both files**. The check had the key it needed and never looked.
//!
//! So there are two, and they are counted separately because they mean
//! different things:
//!
//! | | What disagreed | Held in |
//! |---|---|---|
//! | Identity | one key, two different ISINs | [`Merged::conflicts`] |
//! | Eligibility | one ISIN, kept by one vendor and declined by another | [`Merged::eligibility`] |
//!
//! Under D-0025 both are **zero** on the real masters — the gate reads one
//! exchange-issued series code from both vendors, so their verdicts agree on
//! 4,080 of 4,080 shared ISINs. That is the point: the checks are what make
//! the zero worth anything.
//!
//! # What a disagreement does
//!
//! It **refuses**, and the refusal is a returned value rather than a log line:
//! [`Merged::verdict`] is [`Verdict::Disputed`], the caller's exit code is
//! non-zero and `/health` stops returning 200. D-0026. Earlier text here
//! claimed a non-empty conflicts vector "is a refusal to believe the merge",
//! and cited D-0020 for it, while the code printed a line, kept the
//! instrument, and exited 0 — the "primary vendor wins, log the difference"
//! shape D-0020 exists to reject. Naming a thing a refusal does not make it
//! one.
//!
//! # The suffix, and why it is confirmed rather than normalised
//!
//! Groww leaks `internal_trading_symbol` into `trading_symbol` on exactly 209
//! of the 4,080 ISINs the two masters share — `BLUECHIP-BE`, `CBAZAAR-ST`,
//! `HDFCLIQUID-EQ`, `LOWVOL-EQ` — and the rule is exact with zero residual:
//! `groww.trading_symbol == dhan.UNDERLYING_SYMBOL + "-" + series`. It would
//! be very easy, and wrong, to strip a trailing `-XX` everywhere:
//! `BAJAJ-AUTO`, `NAM-INDIA` and `M&M` are real tickers, and
//! `crates/core/src/symbol.rs` argues at length that blind normalisation
//! manufactures the collision it exists to prevent.
//!
//! So the strip needs **two** independent agreements before it is applied: the
//! trailing segment must be the row's own series (decided in
//! [`brutex_core::vendor`], which is where the series is), and some vendor
//! must have asserted the stripped identity under the **same ISIN** (decided
//! here, which is where both vendors are). Neither alone is enough, and where
//! they do not both hold the symbol is left exactly as the vendor wrote it.

use brutex_core::instrument::InstrumentKey;
use brutex_core::isin::Isin;
use brutex_core::universe::{self, Universe};
use brutex_core::vendor::{Listing, Skip, Vendor, VendorId, VendorSet};
use std::collections::{BTreeMap, HashMap, HashSet};

/// One instrument, after every vendor has had its say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entry {
    /// The id each vendor uses for this instrument, indexed by
    /// `Vendor as usize`, so a request can NAME it.
    ///
    /// This is the last link in the chain that `DH-905 securityId is required`
    /// reports as broken: the master carries the id, the merge indexes it by
    /// key, and `pull::vendor::ParamValue::InstrumentId` puts it on the wire.
    /// `None` where that vendor does not list the instrument, which a caller
    /// must refuse on rather than substitute another vendor's.
    pub ids: [Option<brutex_core::vendor::VendorId>; brutex_core::vendor::Vendor::ALL.len()],
    /// Each vendor's own ISIN; absent or ambiguous assertions remain `None`.
    pub vendor_isins: [Option<Isin>; Vendor::ALL.len()],
    /// Vendors whose same-key or alias assertions disagree on id or ISIN.
    pub ambiguous: VendorSet,
    /// Which vendors listed it. Seeing two here is the deduplication, on
    /// screen.
    pub vendors: VendorSet,
    /// A deterministic display representative, never per-vendor provenance.
    /// Use `vendor_isin` for resolution and `Merged::assertions` for evidence.
    pub isin: Option<(Vendor, Isin)>,
    /// A different ISIN for display. Complete evidence is in `Merged::assertions`.
    pub conflict: Option<(Vendor, Isin)>,
    /// Which of the engine's universes this instrument is in.
    pub universe: Universe,
}

impl Entry {
    /// The ISIN this vendor unambiguously supplied, never another vendor's.
    #[must_use]
    pub fn vendor_isin(&self, vendor: Vendor) -> Option<Isin> {
        self.vendor_isins.get(vendor as usize).copied().flatten()
    }
}

/// One vendor's whole contribution to the merge.
///
/// Both halves are needed: what a vendor **kept** builds the map, and what it
/// **declined** is the only way another vendor's keep can be recognised as a
/// disagreement rather than as a row nobody mentioned.
#[derive(Debug)]
pub struct Source {
    /// Whose rows these are.
    pub vendor: Vendor,
    /// The instruments this vendor listed.
    pub kept: Vec<Listing>,
    /// The ISINs this vendor declined, and why.
    pub declined: Vec<(Isin, Skip)>,
}

/// Whether the merged universe may be believed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Every vendor that spoke agreed. The normal state.
    Clean,
    /// At least one cross-vendor disagreement survived the merge.
    ///
    /// The instruments are still in the map and still on the page — dropping
    /// them would hide the disagreement, which is the failure `CLAUDE.md` §4
    /// forbids — but the *universe as a whole* is refused: a caller that turns
    /// this into an exit code must not return success. D-0026.
    Disputed,
}

/// The merged universe, and everything that disagreed while it was built.
#[derive(Debug, Default)]
pub struct Merged {
    /// Exact vendor-native ids, built from original listing keys at construction.
    /// Empty in default-built fixtures; use [`Self::native_id`] for lookup.
    pub native_ids: NativeIds,
    /// All distinct vendor assertions beside their resolved key. Retained even
    /// when an ambiguous entry refuses to expose a request id.
    pub assertions: Vec<(Vendor, InstrumentKey, Listing)>,
    /// Which mastered vendors actually supplied rows to this merge.
    ///
    /// # Why "confirmed" cannot be asked of `Vendor::MASTERED`
    ///
    /// It was, and adding a third mastered vendor broke it. `MASTERED` is the
    /// set that PUBLISHES a master; this is the set whose master was READ. A
    /// vendor with no file on disk contributes no rows, so
    /// `MASTERED.all(contains)` went false for every instrument the moment a
    /// third name joined the list — flipping the entire universe from
    /// *confirmed by every vendor* to *asserted by one*, loudly and wrongly.
    ///
    /// A vendor that supplied nothing can neither confirm nor deny an
    /// instrument. Cross-checking is a claim about the masters that were
    /// actually compared, and this is that set.
    pub contributed: Vec<Vendor>,
    /// One entry per distinct instrument.
    pub by_key: HashMap<InstrumentKey, Entry>,
    /// One line per key two vendors gave different ISINs for, naming the key
    /// and both ISINs. Empty is the normal state.
    pub conflicts: Vec<String>,
    /// One line per ISIN one vendor kept and another declined, naming the
    /// ISIN, both vendors and the decline reason.
    ///
    /// Separate from [`Merged::conflicts`] because it is a different
    /// disagreement: not "which paper is this" but "is this paper an equity at
    /// all". Folding the two together would let a count of one hide the other.
    pub eligibility: Vec<String>,
}

impl Merged {
    /// An unambiguous id for this vendor's exact original [`Listing::key`].
    /// One expected O(1) hash lookup; never scans assertions or resolves aliases.
    /// This is vendor-symbol evidence only, not independent identity evidence.
    /// Missing keys, conflicting ids, and ids shared by different native keys
    /// return `None`. Normal merged identity lookup is unchanged.
    #[must_use]
    pub fn native_id(
        &self,
        vendor: Vendor,
        key: impl std::borrow::Borrow<InstrumentKey>,
    ) -> Option<VendorId> {
        self.native_ids
            .by_key
            .get(&(vendor, *key.borrow()))
            .copied()
    }

    /// The number of distinct instruments.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// Whether nothing merged at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// Whether this universe may be believed.
    #[must_use]
    pub fn verdict(&self) -> Verdict {
        if self.conflicts.is_empty() && self.eligibility.is_empty() {
            Verdict::Clean
        } else {
            Verdict::Disputed
        }
    }

    /// How many members of each universe the merge resolved, and how many of
    /// those two vendors both named.
    ///
    /// The second number is the one that means something. An instrument named
    /// by one vendor rests entirely on that vendor's gate; an instrument named
    /// by both has been cross-checked. Reporting only the first would let two
    /// vendors' worth of confidence and one vendor's worth look identical.
    #[must_use]
    pub fn universe_census(&self) -> Vec<(&'static str, usize, usize)> {
        let mut out = Vec::new();
        for (name, bit) in [
            ("F&O underlyings", Universe::FNO),
            ("NIFTY Total Market", Universe::TOTAL_MARKET),
        ] {
            let members = self.by_key.iter().filter(|(_, e)| e.universe.contains(bit));
            let mut present = 0;
            let mut both = 0;
            for (_, e) in members {
                present += 1;
                if self.confirmed_by_all(e) {
                    both += 1;
                }
            }
            out.push((name, present, both));
        }
        out
    }

    /// Whether every master that was actually READ names this instrument.
    ///
    /// Asked of [`Self::contributed`] and never of `Vendor::MASTERED` — see
    /// that field for the defect this distinction removes.
    ///
    /// **An empty `contributed` answers `true`**, and that is deliberate rather
    /// than an accident of `all`: with no master read there is nothing to
    /// cross-check, and reporting every instrument as *only one vendor named
    /// it* would be a finding about masters that were never compared. The
    /// merge is empty in that case anyway, so nothing is reported either way.
    #[must_use]
    fn confirmed_by_all(&self, entry: &Entry) -> bool {
        self.contributed
            .iter()
            .all(|vendor| entry.vendors.contains(*vendor))
    }

    /// Universe members only one vendor named, by universe and key.
    ///
    /// An index carries no ISIN, so nothing cross-checks its identity at all —
    /// and the two masters spell most index names differently. This is where
    /// that shows up instead of being invisible: `MIDCPNIFTY` and `NIFTYNXT50`
    /// are F&O underlyings Dhan names and Groww calls `NIFTYMIDSELECT` and
    /// `NIFTYJR`. `docs/06-limits.md` §12.
    #[must_use]
    pub fn single_vendor_members(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .by_key
            .iter()
            .filter(|(_, e)| !e.universe.is_none() && !self.confirmed_by_all(e))
            .map(|(k, e)| {
                let who: Vec<&str> = self
                    .contributed
                    .iter()
                    .filter(|v| e.vendors.contains(**v))
                    .map(|v| v.as_str())
                    .collect();
                format!("{k} ({})", who.join(" "))
            })
            .collect();
        out.sort_unstable();
        out
    }
}

/// Prebuilt exact-native lookup. Private entries prevent callers from bypassing
/// the construction-time checks; `Default` is an empty index.
#[derive(Debug, Default)]
pub struct NativeIds {
    by_key: HashMap<(Vendor, InstrumentKey), VendorId>,
}

impl NativeIds {
    fn build(sources: &[Source], capacity: usize) -> Self {
        let mut forward = HashMap::with_capacity(capacity);
        let mut reverse = HashMap::with_capacity(capacity);
        for source in sources {
            for listing in &source.kept {
                forward
                    .entry((source.vendor, listing.key))
                    .and_modify(|id| {
                        if *id != Some(listing.vendor_id) {
                            *id = None;
                        }
                    })
                    .or_insert(Some(listing.vendor_id));
                reverse
                    .entry((source.vendor, listing.vendor_id))
                    .and_modify(|key| {
                        if *key != Some(listing.key) {
                            *key = None;
                        }
                    })
                    .or_insert(Some(listing.key));
            }
        }
        let mut by_key = HashMap::with_capacity(capacity);
        for ((vendor, key), id) in forward {
            if let Some(id) = id
                && reverse.get(&(vendor, id)) == Some(&Some(key))
            {
                by_key.insert((vendor, key), id);
            }
        }
        Self { by_key }
    }
}

/// Merges every vendor's kept listings into one map, and cross-checks it.
///
/// Two passes, because the confirmation a strip needs may come from a vendor
/// that has not been read yet. The first pass records every identity any
/// vendor asserted; the second resolves each listing against it.
///
/// # Three maps, and only two of them are hash probes
///
/// This used to say "**both** are O(1) per listing — a hash probe on
/// fixed-width `Copy` keys, never a scan", and of the three maps here that was
/// true of one. It is corrected rather than left to be believed.
///
/// * `asserted` and `by_key` are `HashMap`/`HashSet` **pre-sized from
///   `capacity`**. Probes have expected O(1) cost, not a guaranteed worst-case
///   bound: reservation avoids growth but cannot eliminate hash collisions.
///   The argument for that reservation is written out below, and
///   `api::bench::every_order_and_pill_is_flat` measures the request path these
///   two feed at 2,787 and at 50,000 instruments.
/// * `kept_isins` and `disputes` are `BTreeMap`, so they are **O(log n)
///   comparisons per listing, not a probe**. That is deliberate and is not a
///   defect to repair: their ordering *is* the output order of the conflict and
///   eligibility lines, which `CLAUDE.md` §3 rule 5 requires to be identical
///   between two runs. A `HashMap` would trade a stated determinism guarantee
///   for a bound nobody measured.
///
/// `kept_isins` used to say it "allocates a `Vec<Vendor>` for each new ISIN",
/// and it did — a `Vec` is a **multiset**, and the question asked of it is
/// *which vendors kept this ISIN*, which is a **set**. One vendor keeping the
/// same paper twice — the NSE row and the BSE row of one company carry one
/// ISIN, and `InstrumentKey` carries the exchange, so both are kept and neither
/// is a duplicate — pushed that vendor twice, and the eligibility walk below
/// then emitted the identical dispute line once per push. The value is now a
/// [`VendorSet`], the `u8` bitset this file already uses for `Merged::vendors`
/// and `contributed`, so a repeat keep is idempotent by construction rather
/// than by a `dedup` nobody wrote. It also removes the per-ISIN heap
/// allocation, though that is a side effect and not the reason.
///
/// Neither `BTreeMap` is on a request path. `merge` runs where the masters are
/// parsed — `server::universe`, reached from `server::Site::load` once per
/// process — which is the read D-0039 moved out of the request at 150 ms, and
/// is why `server::instruments_html_from` takes an already-loaded universe.
/// Retained assertions are sorted once at startup (O(n log n)) so diagnostic
/// representatives and evidence order do not depend on source or row order.
#[must_use]
pub fn merge(sources: &[Source]) -> Merged {
    // THE BOUND, TAKEN ONCE AND USED BY BOTH MAPS BELOW.
    //
    // No more distinct entries can exist in either than there are kept
    // listings, so this one sum reserves both. It used to be computed 33 lines
    // further down, beside `by_key` alone, under a comment arguing at length
    // that "the upper bound is known exactly before the loop starts" — and
    // `asserted`, filled by the identical loop over the identical listings,
    // started at `HashSet::new()` two lines above it and rehashed its way up.
    // The argument was already written; it just was not applied to the map
    // that came first. `docs/07-o1-architecture.md` law 2.
    let capacity = sources.iter().map(|s| s.kept.len()).sum();

    // Pass 1. Every (identity, ISIN) pair anybody asserted. A row can never
    // confirm ITSELF: it asserts its raw key, and the candidate it offers is
    // by construction a different symbol.
    //
    // Pre-sized from `capacity`: a listing contributes at most one pair, and
    // only when it carries an ISIN, so the set is a subset of the listings and
    // this reservation cannot be exceeded.
    let mut asserted: HashSet<(InstrumentKey, Isin)> = HashSet::with_capacity(capacity);
    // And every ISIN each vendor KEPT, which is what a decline is checked
    // against. `BTreeMap` rather than `HashMap` so the conflict lines come out
    // in a stable order and two runs produce byte-identical output.
    //
    // A SET PER ISIN, NOT A LIST. `VendorSet::with` is `|` on a `u8`, so the
    // second time a vendor keeps the same ISIN it changes nothing — which is
    // the property the eligibility walk below needs and a `Vec` did not have.
    // `or_default` is `VendorSet::EMPTY` -- not assumed, but pinned by
    // `core::vendor::tests::
    //  a_vendor_set_is_a_set_and_every_vendor_has_its_own_bit`, which asserts
    // both that `default()` is `EMPTY` and that adding twice is adding once.
    let mut kept_isins: BTreeMap<Isin, VendorSet> = BTreeMap::new();
    let mut raw_identity = HashMap::with_capacity(capacity);
    let mut raw_ambiguous = HashSet::with_capacity(capacity);
    for s in sources {
        for l in &s.kept {
            let identity = (l.vendor_id, l.isin);
            if *raw_identity.entry((s.vendor, l.key)).or_insert(identity) != identity {
                raw_ambiguous.insert((s.vendor, l.key));
            }
            if let Some(i) = l.isin {
                asserted.insert((l.key, i));
                let keepers = kept_isins.entry(i).or_default();
                *keepers = keepers.with(s.vendor);
            }
        }
    }

    // PRE-SIZED, so a probe is O(1) in the WORST case and not merely on
    // average.
    //
    // A `HashMap` that grows reallocates and rehashes every key it holds. That
    // is amortised O(1) per insert, which is the honest description — but it
    // means one insert in every doubling costs O(n), and the spike lands
    // wherever the doubling lands rather than anywhere predictable. With 200,000
    // rows arriving from two vendors that is roughly eighteen rehashes, the last
    // of which moves every key.
    //
    // The upper bound is known exactly before the loop starts: no more distinct
    // keys can exist than there are kept listings. Reserving that much means the
    // map need not grow during insertion. This does not provide worst-case
    // constant-time collision handling. `capacity` is taken at the top
    // of this function, because `asserted` needs the same number.
    //
    // It over-reserves when two vendors name the same instrument — which is the
    // common case, and the point of merging. That is bounded waste (one entry
    // per duplicate, freed when the map is dropped) traded for a bound that
    // holds in the worst case rather than on average.
    // WHICH MASTERS WERE ACTUALLY READ — the set every cross-check is asked
    // of. Taken from the sources handed in, never from `Vendor::MASTERED`: a
    // vendor that publishes a master and did not supply one here can neither
    // confirm nor deny an instrument, and asking it to would make every
    // instrument in the tree read as single-sourced. See `Merged::contributed`.
    // A MASTER THAT LISTS NOTHING HAS COMPARED NOTHING.
    //
    // Filtered on `kept`, not on the source merely being present. A file that
    // was read and held no rows is indistinguishable, for cross-checking, from
    // one that was never read: it confirms nothing and it denies nothing.
    // Counting it as a comparer makes every instrument in the tree read as
    // unconfirmed — a finding about a file with no contents, reported as a
    // disagreement between vendors.
    // A SET, HELD AS A SET. This was a `collect` followed by a sort and a
    // `dedup` — a uniqueness pass hand-rolled out of an ordering pass, when
    // `VendorSet` is a u8 bitset whose whole job is this. It is imported at the
    // top of this file already and is the type `Merged::vendors` already is, so
    // removing the sort introduces nothing new.
    //
    // THE RESULT IS THE SAME VECTOR, ELEMENT FOR ELEMENT. `dedup` after a sort
    // by `*v as u8` leaves the distinct vendors in ascending discriminant
    // order, and `Vendor::ALL` IS that order — "APPENDED, NEVER INSERTED", the
    // same property `Entry::ids` already indexes on with `vendor as usize`.
    // Filtering `ALL` by membership reproduces it without comparing anything.
    // This matters because the order is RENDERED: `single_vendor_members`
    // walks `contributed` to build the `UNCHECKED IDENTITY` note on `/health`.
    //
    // AND THE COST STOPS DEPENDING ON THE CALLER. The sort was over one element
    // per `Source`, and nothing in `&[Source]` bounds that count — only the
    // fact that `server::universe`, the one production caller today, builds one
    // per entry of `Vendor::MASTERED`. That is a caller's habit, not a bound,
    // and CI gate 11 rule 4 exists to refuse exactly that kind of claim. The
    // fold is one pass over `sources` the loop below already makes, and the
    // filter walks a five-element const array. No caller can grow either.
    let seen = sources
        .iter()
        .filter(|s| !s.kept.is_empty())
        .fold(VendorSet::EMPTY, |set, s| set.with(s.vendor));
    let contributed: Vec<Vendor> = Vendor::ALL
        .into_iter()
        .filter(|&v| seen.contains(v))
        .collect();
    let mut out = Merged {
        native_ids: NativeIds::build(sources, capacity),
        by_key: HashMap::with_capacity(capacity),
        contributed,
        ..Merged::default()
    };
    let assertions = ordered_assertions(sources, &asserted);
    for &(vendor, key, l) in &assertions {
        merge_assertion(
            &mut out,
            vendor,
            key,
            l,
            raw_ambiguous.contains(&(vendor, l.key)),
        );
    }
    out.assertions = assertions;
    out.conflicts.sort_unstable();
    out.conflicts.dedup();

    // THE ELIGIBILITY CHECK. One vendor declined this ISIN; did another keep
    // it? Sorted by ISIN, so the output does not depend on map iteration
    // order. This is what used to be structurally impossible: the declined
    // rows were dropped before the merge ever saw them.
    let mut disputes: BTreeMap<Isin, Vec<String>> = BTreeMap::new();
    for s in sources {
        for &(isin, reason) in &s.declined {
            // Only a decline that judges the PAPER can contradict a keep. A
            // decline for the venue cannot: `RELIANCE` is `INE002A01018` on
            // both NSE and BSE, so one vendor declining the BSE row while the
            // other keeps the NSE row is two correct decisions, not a
            // disagreement — and counting it as one produced 3,000 false
            // conflicts on the real masters.
            if !reason.judges_the_paper() {
                continue;
            }
            let Some(&keepers) = kept_isins.get(&isin) else {
                continue;
            };
            // ONE LINE PER KEEPER, AND A KEEPER IS COUNTED ONCE.
            //
            // The walk is over `Vendor::ALL` filtered by membership — a
            // five-element const array, exactly the shape `contributed` above
            // is built with, and for the same two reasons. It cannot repeat a
            // vendor, so a vendor that kept this ISIN under two exchanges
            // produces one dispute line and not two; and the order is
            // `Vendor::ALL`'s, so it no longer depends on the order the CALLER
            // handed the sources in. That is strictly more of what `CLAUDE.md`
            // §3 rule 5 asks for, and the one production caller
            // (`server::universe`, over `Vendor::MASTERED`) already passed them
            // in ascending discriminant order, so no rendered line moves.
            for keeper in Vendor::ALL.into_iter().filter(|&v| keepers.contains(v)) {
                // A vendor listing the same ISIN twice — once kept, once
                // declined — is a row shape neither master has, but it would
                // be a statement about ONE vendor and not a cross-vendor
                // disagreement, so it is not one.
                if keeper != s.vendor {
                    disputes.entry(isin).or_default().push(format!(
                        "{isin}: {} kept it, {} declined it as {}",
                        keeper.as_str(),
                        s.vendor.as_str(),
                        reason.reason()
                    ));
                }
            }
        }
    }
    out.eligibility = disputes.into_values().flatten().collect();
    out.eligibility.sort_unstable();
    out.eligibility.dedup();
    // THE MERGED UNIVERSE, AND THE TWO WAYS IT CAN BE WRONG.
    //
    // Everything downstream — the target filter, the sweep surface, what a
    // pull asks the vendor for — is decided by `by_key`. A conflict means two
    // vendors gave one key two different ISINs; an eligibility dispute means
    // one vendor kept an instrument the other declined. Neither refuses the
    // run: both are recorded and the run proceeds, which is right, and is
    // exactly why they must be visible somewhere that survives the page.
    //
    // `Warn` when either list is non-empty, because a silently disputed
    // universe is a run whose scope nobody agreed on. `Info` when clean.
    //
    // Counts only — the lists themselves are unbounded and already rendered on
    // `/instruments`. A log line must not carry a list whose length is the
    // universe's.
    let any_disagreement = !out.conflicts.is_empty() || !out.eligibility.is_empty();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            if any_disagreement {
                telemetry::Level::Warn
            } else {
                telemetry::Level::Info
            },
            "api.merge",
            "universe merged",
        )
        .with("sources", telemetry::Value::Uint(sources.len() as u64))
        .with("keys", telemetry::Value::Uint(out.by_key.len() as u64))
        .with(
            "conflicts",
            telemetry::Value::Uint(out.conflicts.len() as u64),
        )
        .with(
            "eligibility_disputes",
            telemetry::Value::Uint(out.eligibility.len() as u64),
        ),
    );
    out
}

/// Retains distinct assertions in a stable order after alias resolution.
fn ordered_assertions(
    sources: &[Source],
    asserted: &HashSet<(InstrumentKey, Isin)>,
) -> Vec<(Vendor, InstrumentKey, Listing)> {
    let mut assertions: Vec<_> = sources
        .iter()
        .flat_map(|s| s.kept.iter().map(|l| (s.vendor, resolve(l, asserted), *l)))
        .collect();
    assertions.sort_unstable_by(|(av, ak, a), (bv, bk, b)| {
        (
            *av as usize,
            ak,
            a.key,
            a.isin,
            a.vendor_id.as_str(),
            a.unsuffixed,
        )
            .cmp(&(
                *bv as usize,
                bk,
                b.key,
                b.isin,
                b.vendor_id.as_str(),
                b.unsuffixed,
            ))
    });
    assertions.dedup();
    assertions
}

/// Records one assertion without allowing ambiguity to restore a request id.
fn merge_assertion(
    out: &mut Merged,
    vendor: Vendor,
    key: InstrumentKey,
    listing: Listing,
    raw_ambiguous: bool,
) {
    let e = out.by_key.entry(key).or_default();
    let previously_seen = e.vendors.contains(vendor);
    e.vendors = e.vendors.with(vendor);
    e.universe = universe::of_instrument(&key);
    if let Some(slot) = e.ids.get_mut(vendor as usize) {
        if raw_ambiguous
            || (previously_seen
                && (*slot != Some(listing.vendor_id)
                    || e.vendor_isins.get(vendor as usize).copied().flatten() != listing.isin))
        {
            e.ambiguous = e.ambiguous.with(vendor);
            out.conflicts.push(format!(
                "{key}: {} has ambiguous id/ISIN assertions",
                vendor.as_str()
            ));
        }
        *slot = if e.ambiguous.contains(vendor) {
            None
        } else {
            Some(listing.vendor_id)
        };
    }
    if let Some(slot) = e.vendor_isins.get_mut(vendor as usize) {
        *slot = if e.ambiguous.contains(vendor) {
            None
        } else {
            listing.isin
        };
    }
    match (e.isin, listing.isin) {
        (None, given) => e.isin = given.map(|i| (vendor, i)),
        (Some((first_vendor, first)), Some(given)) if first != given => {
            e.conflict = Some((vendor, given));
            out.conflicts.push(format!(
                "{key}: {} says {first}, {} says {given}",
                first_vendor.as_str(),
                vendor.as_str()
            ));
        }
        _ => {}
    }
}

/// The identity to file a listing under.
///
/// The candidate key from the decoder is adopted **only** when some vendor has
/// asserted that identity under this row's own ISIN. Without that confirmation
/// the vendor's symbol stands, suffix and all: an unmerged row is a visible
/// duplicate, while a wrongly merged one is two instruments silently becoming
/// one.
fn resolve(l: &Listing, asserted: &HashSet<(InstrumentKey, Isin)>) -> InstrumentKey {
    match (l.unsuffixed, l.isin) {
        (Some(candidate), Some(isin)) if asserted.contains(&(candidate, isin)) => candidate,
        _ => l.key,
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use brutex_core::instrument::{Exchange, Kind, Segment};
    use brutex_core::symbol::Symbol;

    fn key(sym: &str, kind: Kind) -> InstrumentKey {
        InstrumentKey {
            exchange: Exchange::Nse,
            segment: if kind == Kind::Index {
                Segment::Index
            } else {
                Segment::Cash
            },
            underlying: Symbol::new(sym).expect("valid"),
            kind,
        }
    }

    fn equity(sym: &str, isin: &str) -> Listing {
        Listing {
            vendor_id: brutex_core::vendor::VendorId::new("1333").expect("a legal id"),
            key: key(sym, Kind::Equity),
            isin: Some(Isin::new(isin).expect("valid")),
            unsuffixed: None,
        }
    }

    fn index(sym: &str) -> Listing {
        Listing {
            vendor_id: brutex_core::vendor::VendorId::new("1333").expect("a legal id"),
            key: key(sym, Kind::Index),
            isin: None,
            unsuffixed: None,
        }
    }

    /// One vendor that declined nothing, which is what most tests need.
    fn from(vendor: Vendor, kept: Vec<Listing>) -> Source {
        Source {
            vendor,
            kept,
            declined: Vec::new(),
        }
    }

    #[test]
    fn native_ids_accept_duplicates_and_keep_vendors_separate() {
        let a = equity("RELIANCE", "INE002A01018");
        let mut b = a;
        b.vendor_id = VendorId::new("other").unwrap();
        let mut c = equity("BLUECHIP", "INE657B01025");
        c.vendor_id = a.vendor_id;
        let merged = merge(&[
            from(Vendor::Zerodha, vec![a, a]),
            from(Vendor::Groww, vec![b, c]),
        ]);
        assert_eq!(merged.native_id(Vendor::Zerodha, a.key), Some(a.vendor_id));
        assert_eq!(merged.native_id(Vendor::Groww, a.key), Some(b.vendor_id));
        assert_eq!(merged.native_id(Vendor::Groww, c.key), Some(c.vendor_id));
        assert_eq!(merged.native_id(Vendor::Zerodha, c.key), None);
        assert_eq!(merged.native_id(Vendor::Dhan, a.key), None);
        assert_eq!(Merged::default().native_id(Vendor::Zerodha, a.key), None);
    }

    #[test]
    fn native_conflicts_in_either_direction_stay_refused_under_permutation() {
        let a = equity("RELIANCE", "INE002A01018");
        let mut different_id = a;
        different_id.vendor_id = VendorId::new("other").unwrap();
        let mut different_key = equity("BLUECHIP", "INE657B01025");
        different_key.vendor_id = different_id.vendor_id;
        let mut third_key = equity("CHOLAFIN", "INE121A01024");
        third_key.vendor_id = different_id.vendor_id;
        let rows = [a, different_id, different_key, third_key, a];
        for shift in 0..rows.len() {
            for reversed in [false, true] {
                let mut permutation = rows.to_vec();
                permutation.rotate_left(shift);
                if reversed {
                    permutation.reverse();
                }
                let mut sources = vec![
                    from(Vendor::Zerodha, permutation),
                    from(Vendor::Dhan, vec![a]),
                ];
                if reversed {
                    sources.reverse();
                }
                let merged = merge(&sources);
                for key in [a.key, different_key.key, third_key.key] {
                    assert_eq!(merged.native_id(Vendor::Zerodha, key), None);
                }
                assert_eq!(merged.native_id(Vendor::Dhan, a.key), Some(a.vendor_id));
                assert_eq!(merged.by_key[&a.key].ids[Vendor::Zerodha as usize], None);
            }
        }
    }

    #[test]
    fn native_lookup_never_uses_the_confirmed_unsuffixed_alias() {
        let canonical = equity("BLUECHIP", "INE657B01025");
        let mut alias = equity("BLUECHIP-BE", "INE657B01025");
        alias.unsuffixed = Some(canonical.key);
        alias.vendor_id = VendorId::new("native-alias").unwrap();
        let merged = merge(&[
            from(Vendor::Dhan, vec![canonical]),
            from(Vendor::Groww, vec![alias]),
        ]);
        assert_eq!(
            merged.native_id(Vendor::Groww, alias.key),
            Some(alias.vendor_id)
        );
        assert_eq!(merged.native_id(Vendor::Groww, canonical.key), None);
        assert_eq!(
            merged.by_key[&canonical.key].ids[Vendor::Groww as usize],
            Some(alias.vendor_id)
        );

        // Two distinct native mappings may collapse onto one merged key. The
        // opt-in native lookup must not restore the ambiguous merged id.
        let merged = merge(&[
            from(Vendor::Dhan, vec![canonical]),
            from(Vendor::Groww, vec![alias, canonical]),
        ]);
        assert_eq!(
            merged.native_id(Vendor::Groww, alias.key),
            Some(alias.vendor_id)
        );
        assert_eq!(
            merged.native_id(Vendor::Groww, canonical.key),
            Some(canonical.vendor_id)
        );
        assert_eq!(
            merged.by_key[&canonical.key].ids[Vendor::Groww as usize],
            None
        );
        assert!(
            merged.by_key[&canonical.key]
                .ambiguous
                .contains(Vendor::Groww)
        );
    }

    #[test]
    fn conflicts_and_assertions_are_independent_of_source_and_row_order() {
        let a = equity("RELIANCE", "INE002A01018");
        let mut b = a;
        b.vendor_id = brutex_core::vendor::VendorId::new("other").expect("id");
        let c = equity("RELIANCE", "INE009A01021");
        let forward = merge(&[
            from(Vendor::Groww, vec![a, b, c]),
            from(Vendor::Dhan, vec![a]),
            from(Vendor::Zerodha, vec![c]),
        ]);
        let reversed = merge(&[
            from(Vendor::Zerodha, vec![c]),
            from(Vendor::Dhan, vec![a]),
            from(Vendor::Groww, vec![c, b, a]),
        ]);
        assert_eq!(forward.by_key, reversed.by_key);
        assert_eq!(forward.assertions, reversed.assertions);
        assert_eq!(forward.conflicts, reversed.conflicts);
        let entry = forward.by_key[&a.key];
        assert_eq!(entry.ids[Vendor::Groww as usize], None);
        assert_eq!(entry.vendor_isin(Vendor::Dhan), a.isin);
        assert_eq!(entry.vendor_isin(Vendor::Zerodha), c.isin);
        assert_eq!(forward.assertions.len(), 5);
    }

    #[test]
    fn conflicting_raw_alias_rows_remain_refused_when_their_resolutions_differ() {
        let canonical = equity("RELIANCE", "INE002A01018");
        let mut alias = equity("RELIANCE-EQ", "INE002A01018");
        alias.unsuffixed = Some(canonical.key);
        let mut conflicting = alias;
        conflicting.isin = Isin::new("INE009A01021").ok();
        for rows in [vec![alias, conflicting], vec![conflicting, alias]] {
            let merged = merge(&[
                from(Vendor::Dhan, vec![canonical]),
                from(Vendor::Groww, rows),
            ]);
            for key in [canonical.key, alias.key] {
                let entry = merged.by_key[&key];
                assert!(entry.ambiguous.contains(Vendor::Groww));
                assert_eq!(entry.ids[Vendor::Groww as usize], None);
                assert_eq!(entry.vendor_isin(Vendor::Groww), None);
            }
            assert_eq!(
                merged.by_key[&canonical.key].ids[Vendor::Dhan as usize],
                Some(canonical.vendor_id)
            );
        }
    }

    #[test]
    fn one_instrument_named_by_both_vendors_is_one_entry_with_two_tags() {
        let m = merge(&[
            from(Vendor::Groww, vec![equity("RELIANCE", "INE002A01018")]),
            from(Vendor::Dhan, vec![equity("RELIANCE", "INE002A01018")]),
        ]);
        assert_eq!(m.len(), 1, "the collision IS the deduplication");
        assert!(!m.is_empty());
        let e = m.by_key[&key("RELIANCE", Kind::Equity)];
        assert!(e.vendors.contains(Vendor::Groww));
        assert!(e.vendors.contains(Vendor::Dhan));
        assert_eq!(e.conflict, None);
        assert!(m.conflicts.is_empty(), "agreement is silent");
        assert!(m.eligibility.is_empty());
        assert_eq!(m.verdict(), Verdict::Clean, "agreement is believable");
        assert!(
            e.universe.contains(Universe::FNO) && e.universe.contains(Universe::TOTAL_MARKET),
            "the merge stamps the universe it resolved into"
        );
    }

    #[test]
    fn a_cross_vendor_isin_conflict_is_reported_and_neither_side_is_dropped() {
        // The whole reason the ISIN is beside the key rather than in it: as a
        // key field this would be two entries and no one would ever know.
        let m = merge(&[
            from(Vendor::Groww, vec![equity("CHOLAFIN", "INE121A01024")]),
            from(Vendor::Dhan, vec![equity("CHOLAFIN", "INE121A08PJ0")]),
        ]);
        assert_eq!(m.len(), 1, "one key, not two");
        assert_eq!(m.conflicts.len(), 1);
        let line = &m.conflicts[0];
        assert!(line.contains("NSE-CHOLAFIN"), "the key is named: {line}");
        assert!(
            line.contains("INE121A01024"),
            "both ISINs are named: {line}"
        );
        assert!(
            line.contains("INE121A08PJ0"),
            "both ISINs are named: {line}"
        );
        assert!(line.contains("groww") && line.contains("dhan"), "{line}");

        let e = m.by_key[&key("CHOLAFIN", Kind::Equity)];
        assert_eq!(
            e.isin.map(|(_, i)| i.to_string()).as_deref(),
            Some("INE121A01024"),
            "the first ISIN is kept"
        );
        assert_eq!(
            e.conflict.map(|(_, i)| i.to_string()).as_deref(),
            Some("INE121A08PJ0"),
            "and so is the second -- neither is dropped, neither wins"
        );
        assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
        assert_eq!(
            m.verdict(),
            Verdict::Disputed,
            "a named disagreement REFUSES the universe; it is not a log line"
        );
    }

    #[test]
    fn one_vendor_keeping_what_another_declined_is_a_named_disagreement() {
        // THE CHECK THAT WAS STRUCTURALLY IMPOSSIBLE. Every declined row was
        // dropped at the reader, so a disagreement about ELIGIBILITY -- the
        // disagreement that actually existed -- produced no conflict, no note
        // and no count, and the report read `0 isin conflicts` while the two
        // masters disagreed about 62 instruments carrying the SAME ISIN in
        // both files.
        //
        // INF090I01VS3 is one of them: a Franklin fund plan Dhan filed as an
        // `ETF` and Groww declined as series `MF`.
        let disputed = Isin::new("INF090I01VS3").expect("valid");
        let m = merge(&[
            from(Vendor::Dhan, vec![equity("FISTIPD3GP", "INF090I01VS3")]),
            Source {
                vendor: Vendor::Groww,
                kept: Vec::new(),
                declined: vec![(disputed, Skip::NotEquityListing)],
            },
        ]);
        assert!(
            m.conflicts.is_empty(),
            "the ISINs agree; the verdicts do not"
        );
        assert_eq!(m.eligibility.len(), 1);
        let line = &m.eligibility[0];
        assert!(line.contains("INF090I01VS3"), "{line}");
        assert!(line.contains("dhan kept it"), "{line}");
        assert!(line.contains("groww declined it"), "{line}");
        assert!(line.contains("not an equity listing"), "{line}");
        assert_eq!(m.verdict(), Verdict::Disputed);
        // The instrument stays in the map. Dropping it would hide the
        // disagreement, which is the failure the check exists to prevent.
        assert_eq!(m.len(), 1);
    }

    #[test]
    fn a_decline_about_the_venue_is_not_a_disagreement_about_the_paper() {
        // RELIANCE is INE002A01018 on NSE and on BSE. One vendor declining the
        // BSE row for being BSE while the other keeps the NSE row is two
        // CORRECT decisions about two different rows -- and counting it as a
        // conflict produced 3,000 false lines on the real masters, which is a
        // check nobody reads twice.
        let reliance = Isin::new("INE002A01018").expect("valid");
        for venue in [
            Skip::ForeignExchange,
            Skip::ForeignSegment,
            Skip::LiveContract,
            Skip::TestInstrument,
        ] {
            let m = merge(&[
                from(Vendor::Dhan, vec![equity("RELIANCE", "INE002A01018")]),
                Source {
                    vendor: Vendor::Groww,
                    kept: vec![equity("RELIANCE", "INE002A01018")],
                    declined: vec![(reliance, venue)],
                },
            ]);
            // Bound rather than called inside the failure message: a call
            // there is a region only a FAILING assertion runs.
            let what = venue.reason();
            assert!(
                m.eligibility.is_empty(),
                "{what} judges the venue, not the paper"
            );
            assert_eq!(m.verdict(), Verdict::Clean);
        }
        // The three that DO judge the paper are the three that can conflict.
        for paper in [
            Skip::NotEquityListing,
            Skip::SmeBoard,
            Skip::UnrecognisedListingClass,
        ] {
            let m = merge(&[
                from(Vendor::Dhan, vec![equity("RELIANCE", "INE002A01018")]),
                Source {
                    vendor: Vendor::Groww,
                    kept: Vec::new(),
                    declined: vec![(reliance, paper)],
                },
            ]);
            let what = paper.reason();
            assert_eq!(m.eligibility.len(), 1, "{what} judges the paper");
        }
    }

    #[test]
    fn one_vendor_keeping_an_isin_on_two_venues_disputes_it_once() {
        // THE MULTISET USED AS A SET. `kept_isins` answers "which vendors kept
        // this ISIN", which is a SET question, and it was a `Vec<Vendor>` that
        // was PUSHED to once per kept listing. A vendor keeping the same paper
        // on two venues is not a duplicate row and not an error: RELIANCE is
        // INE002A01018 on NSE and on BSE, `InstrumentKey` carries the exchange,
        // so both rows are legitimate keeps under one ISIN. That put `dhan` in
        // the vector twice, and the eligibility walk emitted the SAME sentence
        // once per entry.
        //
        // Without the `VendorSet` this asserts 2, and the operator's page
        // carried a disagreement that was counted twice -- `Merged::eligibility`
        // is what `/health` and the exit code count, so a doubled line is a
        // doubled conflict count for one disagreement.
        let reliance = Isin::new("INE002A01018").expect("valid");
        let bse = Listing {
            vendor_id: brutex_core::vendor::VendorId::new("500325").expect("a legal id"),
            key: InstrumentKey {
                exchange: Exchange::Bse,
                segment: Segment::Cash,
                underlying: Symbol::new("RELIANCE").expect("valid"),
                kind: Kind::Equity,
            },
            isin: Some(reliance),
            unsuffixed: None,
        };
        let m = merge(&[
            from(Vendor::Dhan, vec![equity("RELIANCE", "INE002A01018"), bse]),
            Source {
                vendor: Vendor::Groww,
                kept: Vec::new(),
                declined: vec![(reliance, Skip::NotEquityListing)],
            },
        ]);
        // Two keys -- the venue is part of the identity and the two rows do not
        // merge. That is the input shape, asserted so a future change that
        // merged them would fail here rather than make this test vacuous.
        assert_eq!(m.len(), 2, "NSE and BSE are two instruments");
        assert_eq!(
            m.eligibility.len(),
            1,
            "one keeper, one decliner, ONE line: {:?}",
            m.eligibility
        );
        let line = m.eligibility.first().expect("the one line");
        assert!(line.contains("dhan kept it"), "{line}");
        assert!(line.contains("groww declined it"), "{line}");
        assert_eq!(m.verdict(), Verdict::Disputed);
    }

    #[test]
    fn every_keeper_of_a_declined_isin_is_named_once_in_vendor_order() {
        // THE OTHER HALF: making the multiset a set must not LOSE a keeper.
        // Two different vendors keep the paper, a third declines it, and the
        // walk over `Vendor::ALL` must produce one line per keeper -- in
        // `Vendor::ALL` order, which no longer depends on the order the caller
        // handed the sources in. Passed here DESCENDING (Zerodha, Dhan) on
        // purpose; the lines still come out Dhan then Zerodha.
        let disputed = Isin::new("INF090I01VS3").expect("valid");
        let m = merge(&[
            from(Vendor::Zerodha, vec![equity("FISTIPD3GP", "INF090I01VS3")]),
            from(Vendor::Dhan, vec![equity("FISTIPD3GP", "INF090I01VS3")]),
            Source {
                vendor: Vendor::Groww,
                kept: Vec::new(),
                declined: vec![(disputed, Skip::NotEquityListing)],
            },
        ]);
        assert_eq!(m.eligibility.len(), 2, "{:?}", m.eligibility);
        assert!(
            m.eligibility
                .first()
                .is_some_and(|l| l.contains("dhan kept it")),
            "Dhan is before Zerodha in `Vendor::ALL`, whatever order the \
             sources arrived in: {:?}",
            m.eligibility
        );
        assert!(
            m.eligibility
                .get(1)
                .is_some_and(|l| l.contains("zerodha kept it")),
            "{:?}",
            m.eligibility
        );
    }

    #[test]
    fn a_decline_nobody_else_kept_is_not_a_disagreement() {
        // A master is mostly declines. Only a decline that some OTHER vendor
        // contradicts is news, or every bond in the file would be a conflict.
        let m = merge(&[
            from(Vendor::Dhan, vec![equity("RELIANCE", "INE002A01018")]),
            Source {
                vendor: Vendor::Groww,
                kept: vec![equity("RELIANCE", "INE002A01018")],
                declined: vec![
                    (
                        Isin::new("INE121A08PJ0").expect("valid"),
                        Skip::NotEquityListing,
                    ),
                    (
                        Isin::new("IN1520250086").expect("valid"),
                        Skip::NotEquityListing,
                    ),
                ],
            },
        ]);
        assert!(
            m.eligibility.is_empty(),
            "nobody kept those, so nobody disagrees"
        );
        assert_eq!(m.verdict(), Verdict::Clean);

        // And one vendor declining what IT ALSO kept is a statement about one
        // vendor, not a cross-vendor disagreement.
        let m = merge(&[Source {
            vendor: Vendor::Groww,
            kept: vec![equity("RELIANCE", "INE002A01018")],
            declined: vec![(Isin::new("INE002A01018").expect("valid"), Skip::SmeBoard)],
        }]);
        assert!(m.eligibility.is_empty());
        assert_eq!(m.verdict(), Verdict::Clean);
    }

    #[test]
    fn the_census_separates_what_two_vendors_confirmed_from_what_one_asserted() {
        // 211 of the 213 F&O underlyings are named by both masters; MIDCPNIFTY
        // and NIFTYNXT50 are Dhan's spellings for indices Groww calls
        // NIFTYMIDSELECT and NIFTYJR, and an index carries no ISIN, so nothing
        // cross-checks them at all. Reporting only "213 present" would make
        // one vendor's word look like two vendors' agreement.
        let m = merge(&[
            from(
                Vendor::Groww,
                vec![
                    index("NIFTY"),
                    index("NIFTYJR"),
                    equity("RELIANCE", "INE002A01018"),
                ],
            ),
            from(
                Vendor::Dhan,
                vec![
                    index("NIFTY"),
                    index("NIFTYNXT50"),
                    equity("RELIANCE", "INE002A01018"),
                ],
            ),
        ]);
        let census = m.universe_census();
        assert_eq!(
            census,
            vec![
                // NIFTY, NIFTYNXT50 and RELIANCE are F&O underlyings; only
                // NIFTY and RELIANCE are named by both. NIFTYJR is in neither
                // list under that spelling, which is exactly the finding.
                ("F&O underlyings", 3, 2),
                ("NIFTY Total Market", 1, 1),
            ]
        );
        // BOTH spellings surface, because both are indices named by one vendor
        // only and an index has no ISIN to reconcile them with. Naming them is
        // the whole substitute for a cross-check that cannot exist.
        let single = m.single_vendor_members();
        assert_eq!(
            single,
            vec![
                "NSE-NIFTYJR (groww)".to_owned(),
                "NSE-NIFTYNXT50 (dhan)".to_owned()
            ]
        );
    }

    #[test]
    fn a_suffixed_symbol_merges_only_when_the_isin_confirms_it() {
        // Groww says BLUECHIP-BE, Dhan says BLUECHIP, and the ISIN says they
        // are one instrument.
        let suffixed = Listing {
            vendor_id: brutex_core::vendor::VendorId::new("1333").expect("a legal id"),
            key: key("BLUECHIP-BE", Kind::Equity),
            isin: Some(Isin::new("INE657B01025").expect("valid")),
            unsuffixed: Some(key("BLUECHIP", Kind::Equity)),
        };
        let m = merge(&[
            from(Vendor::Groww, vec![suffixed]),
            from(Vendor::Dhan, vec![equity("BLUECHIP", "INE657B01025")]),
        ]);
        assert_eq!(m.len(), 1, "confirmed, so they merge");
        let e = m.by_key[&key("BLUECHIP", Kind::Equity)];
        assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
        assert!(m.conflicts.is_empty());
        assert_eq!(m.verdict(), Verdict::Clean);
    }

    #[test]
    fn an_unconfirmed_suffix_is_left_exactly_as_the_vendor_wrote_it() {
        // Same shape, but no vendor asserts BLUECHIP under that ISIN. An
        // unmerged row is a visible duplicate; a wrongly merged one is two
        // instruments silently becoming one.
        let suffixed = Listing {
            vendor_id: brutex_core::vendor::VendorId::new("1333").expect("a legal id"),
            key: key("BLUECHIP-BE", Kind::Equity),
            isin: Some(Isin::new("INE657B01025").expect("valid")),
            unsuffixed: Some(key("BLUECHIP", Kind::Equity)),
        };
        let m = merge(&[from(Vendor::Groww, vec![suffixed])]);
        assert_eq!(m.len(), 1);
        assert!(m.by_key.contains_key(&key("BLUECHIP-BE", Kind::Equity)));

        // And a candidate confirmed under a DIFFERENT ISIN is not confirmed at
        // all -- this is the case that separates "same ticker" from "same
        // paper", and it must not merge.
        let m = merge(&[
            from(Vendor::Groww, vec![suffixed]),
            from(Vendor::Dhan, vec![equity("BLUECHIP", "INE002A01018")]),
        ]);
        assert_eq!(m.len(), 2, "different ISIN, so different instruments");
        assert!(m.conflicts.is_empty(), "they never met, so nothing clashed");
    }

    #[test]
    fn an_index_carries_no_isin_and_still_merges_on_identity() {
        let m = merge(&[
            from(Vendor::Groww, vec![index("NIFTY")]),
            from(Vendor::Dhan, vec![index("NIFTY")]),
        ]);
        assert_eq!(m.len(), 1);
        let e = m.by_key[&key("NIFTY", Kind::Index)];
        assert_eq!(e.isin, None, "no sentinel ISIN is invented for NIFTY");
        assert!(e.vendors.contains(Vendor::Groww) && e.vendors.contains(Vendor::Dhan));
        assert!(m.conflicts.is_empty());
        assert!(e.universe.contains(Universe::INDEX));
        assert!(
            m.single_vendor_members().is_empty(),
            "both vendors spell NIFTY the same way, so nothing rests on one"
        );
    }

    #[test]
    fn a_vendor_with_nothing_to_add_neither_overwrites_nor_conflicts() {
        // One vendor knows the ISIN, the other does not carry one. That is not
        // a disagreement, and the known value must survive it.
        let bare = Listing {
            vendor_id: brutex_core::vendor::VendorId::new("1333").expect("a legal id"),
            key: key("RELIANCE", Kind::Equity),
            isin: None,
            unsuffixed: None,
        };
        let m = merge(&[
            from(Vendor::Groww, vec![equity("RELIANCE", "INE002A01018")]),
            from(Vendor::Dhan, vec![bare]),
        ]);
        let e = m.by_key[&key("RELIANCE", Kind::Equity)];
        assert_eq!(
            e.isin.map(|(v, _)| v),
            Some(Vendor::Groww),
            "the vendor that knew it is recorded"
        );
        assert_eq!(e.conflict, None);
        assert!(m.conflicts.is_empty());

        // And in the other order, the entry starts empty and is filled later.
        let m = merge(&[
            from(Vendor::Dhan, vec![bare]),
            from(Vendor::Groww, vec![equity("RELIANCE", "INE002A01018")]),
        ]);
        assert_eq!(
            m.by_key[&key("RELIANCE", Kind::Equity)]
                .isin
                .map(|(v, _)| v),
            Some(Vendor::Groww)
        );
    }

    #[test]
    fn merging_nothing_produces_nothing_rather_than_a_default_row() {
        let m = merge(&[]);
        assert!(m.is_empty());
        assert_eq!(m.len(), 0);
        assert!(m.conflicts.is_empty());
        assert!(m.eligibility.is_empty());
        assert_eq!(
            m.verdict(),
            Verdict::Clean,
            "nothing said, nothing disputed"
        );
        assert_eq!(
            m.universe_census(),
            vec![("F&O underlyings", 0, 0), ("NIFTY Total Market", 0, 0)]
        );
        assert!(m.single_vendor_members().is_empty());
    }
}
