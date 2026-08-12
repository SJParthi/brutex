//! The join between an NSE constituent list and one vendor's instrument ids.
//!
//! # What was missing, and what it cost
//!
//! `brutex_core::universe` carries six published lists — the NIFTY 50, 100,
//! 200 and 500, the Total Market and the F&O underlyings — and
//! [`crate::master`] parses both vendor masters. **Nothing joined them.** A
//! tier could be named, counted and stamped onto a merged row, and there was
//! still no answer to the only question a pull actually asks: *which vendor
//! instrument ids does this tier resolve to for this feed?* The `/ingest`
//! universe menu showed every NIFTY tier as `no target`, which was the correct
//! rendering of a set nothing could turn into a request.
//!
//! # THE KEY IS NSE'S OWN ISIN, AT BOTH ENDS. THERE IS NO SYMBOL STEP.
//!
//! A constituent's identity is the ISIN **the exchange prints beside that
//! symbol in its own constituent file** — transcribed into `core` by D-0122
//! and read by [`brutex_core::universe::nse_isin`] in constant time. That ISIN
//! is matched against the ISIN column of the vendor's instrument master. Both
//! ends of the join are ISINs. The published symbol is the argument to
//! `nse_isin` and is a key to nothing.
//!
//! Until D-0125 the first half was a **symbol step**: the published name was
//! looked up in the merged vendor universe on `(exchange, segment, symbol,
//! kind)`, and whatever ISIN the vendors had filed there became the thing
//! joined on. It resolved 850 of 850 names across the four NIFTY tiers, and it
//! answered a different question from the one asked — *what do the brokers
//! think this ticker is*, rather than *what does the exchange say this
//! constituent is*. A name both masters spelled the same way and got wrong the
//! same way was invisible to it, and `docs/06-limits.md` §62 said so in as
//! many words.
//!
//! That step is **deleted, not demoted.** There is no symbol lookup behind the
//! ISIN one, on any path, for any bucket. A constituent whose NSE ISIN no
//! vendor master carries is [`TierJoin::lacks`] — it is never retried by name.
//! A fallback that fires silently is the `CLAUDE.md` §4 failure this module
//! exists to refuse, and a fallback that fires loudly is still a second answer
//! to a question that already has one.
//!
//! # Nobody arbitrates between two vendors any more, because the exchange does
//!
//! The old join met a constituent its two masters gave different ISINs for and
//! refused it outright — `DisputedIsin`, on the D-0020 ground that choosing
//! between two vendors without evidence is not a decision. That variant is
//! gone, because the evidence now exists: NSE's own column says which of the
//! two ISINs belongs to that symbol. The vendor whose master carries it
//! matches; the vendor whose master carries the other one **lacks** it, by
//! name, with NSE's ISIN printed on the row. No id is substituted and no
//! vendor is preferred — the one authority that outranks both is simply read.
//!
//! # The partition sums, and it gained a fifth bucket
//!
//! For one `(vendor, tier)` every published constituent lands in exactly one
//! of five, and their sizes add up to [`Tier::published`]:
//!
//! ```text
//! matched + lacks + ambiguous + malformed + no_nse_isin == published
//! ```
//!
//! * [`TierJoin::matched`] — NSE's ISIN, and exactly one row of this vendor's
//!   master carries it.
//! * [`TierJoin::lacks`] — NSE's ISIN, and no row of this vendor's master
//!   carries it.
//! * [`TierJoin::ambiguous`] — NSE's ISIN, and two or more of this vendor's
//!   rows carry it. Every claimant is named and none is chosen.
//! * [`TierJoin::malformed`] — the published NAME cannot be a key at all: not
//!   a legal [`Symbol`], or an NSE placeholder scrip.
//! * [`TierJoin::no_nse_isin`] — **the new one.** The name is fine and the
//!   exchange's own file names no ISIN for it. Two ways, kept apart:
//!   [`NoIsin::NoRowInTheExchangesFile`] (the five F&O index underlyings, which
//!   are not shares) and [`NoIsin::TheExchangesRowNamesNoIsin`] (`AGL`, the one
//!   Total Market position with an empty ISIN cell — `docs/06-limits.md` §11).
//!   It exists as its own bucket because "this build cannot name an NSE ISIN
//!   for it" is a fact about the EXCHANGE'S file, and folding it in with a
//!   malformed name would blame the transcription for the exchange's own gap.
//!
//! A join that quietly drops a name is the `CLAUDE.md` §4 "fallback that hides
//! a failure" in its most expensive form: the operator asks for 500
//! instruments, 486 are fetched, and nothing anywhere says which fourteen went
//! missing or why. The sum is asserted for every vendor and every tier by
//! `api::constituents::every_constituent_lands_in_exactly_one_bucket`, and the
//! buckets are asserted DISJOINT with their union equal to the published list
//! name for name — a sum alone would pass a join that dropped one name and
//! double-counted another.
//!
//! # What corroboration now means, and what it no longer means
//!
//! `Matched::witness` and `TierJoin::unwitnessed` used to count *the vendors
//! that asserted the identity*, because the identity came from the vendors.
//! Under NSE's own key that measurement is meaningless — every identity has
//! exactly one source and it is not a vendor — so it is gone, replaced by
//! [`Matched::corroboration`]: **which mastered vendors' files carry a row for
//! NSE's ISIN.** It is not evidence for the join, which does not need any; it
//! is the cross-check on NSE's column, and [`TierJoin::uncorroborated`] lists
//! the rows exactly one master's file agrees with.
//!
//! # Cost
//!
//! Built once, from the merged universe, beside [`crate::catalog::Catalog`] —
//! `docs/05-decisions.md` D-0039 and D-0042 put every whole-universe pass at
//! startup, and this is one. Afterwards both questions are O(1): `(vendor,
//! tier)` is an index into a flat array of `Vendor::ALL.len() *
//! Tier::ALL.len()` entries, and `(vendor, exchange, ISIN)` is a single hash
//! probe on a `Copy` key. Neither reads a master file and neither walks the
//! universe. Proved by `api::constituents::the_two_lookups_do_not_grow_with_the_universe`,
//! which measures both against universes 40× apart in size.

use std::collections::HashMap;
use std::fmt::Write as _;

use brutex_core::instrument::Exchange;
use brutex_core::isin::Isin;
use brutex_core::symbol::Symbol;
use brutex_core::universe::{self, Universe};
use brutex_core::vendor::{Vendor, VendorId, VendorSet};

use crate::merge::Merged;

/// The NSE placeholder scrips, named as placeholders rather than resolved.
///
/// The exchange's own constituent files carry these during a corporate action,
/// wearing identifiers that are **not ISINs**: `DUM510W01014` and
/// `DUM256C01024`, the real ISINs of `INOXGREEN` and `TRIVENI` with `INE`
/// replaced by `DUM`. Both fail the ISO 6166 check digit, both real
/// constituents appear separately in the same file, and D-0122 transcribed
/// neither the names nor the pseudo-ISINs. Recorded in `docs/00-charter.md`
/// §4c and D-0089, which is why naming them here is transcription and not
/// invention.
///
/// D-0089 dropped them at transcription, so no list in
/// [`brutex_core::universe`] contains one today and this check fires on none of
/// them — asserted by
/// `api::constituents::no_tier_this_build_carries_holds_a_placeholder_scrip`,
/// which is a statement about the current snapshot and not a licence to remove
/// the check. A rebalance is a new transcription, and the next one may arrive
/// mid-corporate-action.
pub const NSE_PLACEHOLDER_SCRIPS: [&str; 2] = ["DUMMYINXGN", "DUMMYTRVN"];

/// One published constituent list this repository carries.
///
/// Five come from the exchange's own CSVs (`docs/00-charter.md` §4c); the
/// sixth, [`Self::FnoUnderlyings`], is derived from the derivative rows of both
/// masters and cross-checked between them (§4a). The distinction is not
/// cosmetic — it is why [`Self::published`] says where each count comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// The published NIFTY 50 constituents.
    Nifty50,
    /// The published NIFTY 100 constituents.
    Nifty100,
    /// The published NIFTY 200 constituents.
    Nifty200,
    /// The published NIFTY 500 constituents.
    Nifty500,
    /// The published NIFTY Total Market constituents.
    TotalMarket,
    /// The F&O underlyings both masters name, which is a DERIVED list rather
    /// than a published file, and contains index underlyings that are not
    /// shares at all.
    FnoUnderlyings,
}

impl Tier {
    /// Every tier, widest last so the report reads as a ladder.
    ///
    /// **APPEND ONLY.** [`Join::tier`] indexes a flat array by
    /// `vendor as usize * Tier::ALL.len() + tier as usize`, so reordering this
    /// array hands one tier's answer to another tier's caller.
    pub const ALL: [Self; 6] = [
        Self::Nifty50,
        Self::Nifty100,
        Self::Nifty200,
        Self::Nifty500,
        Self::TotalMarket,
        Self::FnoUnderlyings,
    ];

    /// The constituent list, borrowed from `core` and never copied.
    ///
    /// A second copy in this crate would be a second answer to "who is in the
    /// NIFTY 200", and the stale one would be whichever nobody remembered to
    /// rebalance — `CLAUDE.md` §3 rule 1.
    #[must_use]
    pub const fn members(self) -> &'static [&'static str] {
        match self {
            Self::Nifty50 => &universe::NIFTY_50,
            Self::Nifty100 => &universe::NIFTY_100,
            Self::Nifty200 => &universe::NIFTY_200,
            Self::Nifty500 => &universe::NIFTY_500,
            Self::TotalMarket => &universe::NIFTY_TOTAL_MARKET,
            Self::FnoUnderlyings => &universe::FNO_UNDERLYINGS,
        }
    }

    /// How many constituents the source states this tier has.
    ///
    /// Written as a literal on purpose, and checked against
    /// [`Self::members`]`.len()` by
    /// `api::constituents::every_tiers_published_count_matches_the_list_it_borrows`.
    /// The count is the cheapest check that the list came from the file it
    /// claims: a truncated transcription still compiles, still nests, and
    /// still answers every membership question — it just answers for fewer
    /// names than the index has.
    ///
    /// The first five are the exchange's own row counts. The sixth is what
    /// **both** masters name, which is a measurement rather than a
    /// publication; `docs/00-charter.md` §4a records it as
    /// `UNVERIFIED against an NSE circular`.
    #[must_use]
    pub const fn published(self) -> usize {
        match self {
            Self::Nifty50 => 50,
            Self::Nifty100 => 100,
            Self::Nifty200 => 200,
            Self::Nifty500 => 500,
            // 752 rows in the exchange's file, two of which are the
            // placeholder scrips above. D-0089.
            Self::TotalMarket => 750,
            Self::FnoUnderlyings => 213,
        }
    }

    /// What a report calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Nifty50 => "NIFTY 50",
            Self::Nifty100 => "NIFTY 100",
            Self::Nifty200 => "NIFTY 200",
            Self::Nifty500 => "NIFTY 500",
            Self::TotalMarket => "NIFTY Total Market",
            Self::FnoUnderlyings => "F&O underlyings",
        }
    }

    /// The membership bit `core` sets for this tier.
    ///
    /// Carried so a caller can cross-check this join against
    /// [`brutex_core::universe::of_equity`] — the two answers are built from
    /// the same lists by different routes, and
    /// `api::constituents::every_matched_row_also_carries_the_tiers_universe_bit`
    /// is what holds them together.
    #[must_use]
    pub const fn universe(self) -> Universe {
        match self {
            Self::Nifty50 => Universe::NIFTY_50,
            Self::Nifty100 => Universe::NIFTY_100,
            Self::Nifty200 => Universe::NIFTY_200,
            Self::Nifty500 => Universe::NIFTY_500,
            Self::TotalMarket => Universe::TOTAL_MARKET,
            Self::FnoUnderlyings => Universe::FNO,
        }
    }
}

/// Why this build holds no ISIN to join a published constituent on.
///
/// Every variant is a **result**, never a failure: the name is reported with
/// the reason beside it and nothing is synthesised to fill the hole. The
/// variants split into two buckets and [`Self::is_a_malformed_name`] is the
/// split — see the module header for why the exchange's own gap is not filed
/// under the transcription's.
///
/// Three variants of the previous set are **gone**, and their absence is the
/// point of D-0125:
///
/// * `NoListingNamesIt` — "no vendor master names this symbol" was the old
///   join's way of saying the symbol step found nothing. Under NSE's key it is
///   [`TierJoin::lacks`], per vendor, with the ISIN the master does not carry
///   printed on the row.
/// * `IndexHasNoIsin` — an inference from a merged row's missing ISIN. Now
///   read off the exchange's file directly as
///   [`Self::NoRowInTheExchangesFile`], which is what the file actually says.
/// * `DisputedIsin` — two vendors disagreeing was a refusal because there was
///   no third opinion. There is one now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoIsin {
    /// An NSE placeholder scrip — see [`NSE_PLACEHOLDER_SCRIPS`]. Its
    /// identifier is a real ISIN with `INE` overwritten by `DUM`, so it is not
    /// an ISIN and the paper it points at is listed separately anyway.
    PlaceholderScrip,
    /// The published name is not a legal [`Symbol`], so it cannot even be
    /// looked up. A transcription accident rather than an exchange fact.
    NotASymbol,
    /// The exchange's Total Market file — the one file whose ISIN column this
    /// build carries — has no row for this name at all.
    ///
    /// Expected on [`Tier::FnoUnderlyings`] and nowhere else: the five index
    /// underlyings of the option chains are not shares, no numbering agency
    /// issues an ISIN for a computed level, and NSE lists none of them in a
    /// constituent file. `a_name_the_exchanges_file_has_no_row_for_is_its_own_bucket`
    /// asserts that those five are the whole of this bucket on the real lists.
    NoRowInTheExchangesFile,
    /// The exchange's file HAS a row for this name and the row's ISIN cell is
    /// empty.
    ///
    /// One position of 750 on the lists this build carries: `AGL`, recorded in
    /// `docs/06-limits.md` §11. Kept apart from
    /// [`Self::NoRowInTheExchangesFile`] because a missing row and an empty
    /// cell are different facts about the file, and because writing a
    /// neighbouring row's ISIN into the empty cell is precisely what would make
    /// the WRONG row join.
    TheExchangesRowNamesNoIsin,
}

impl NoIsin {
    /// The reason, as a report prints it.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::PlaceholderScrip => "an NSE placeholder scrip, not a constituent",
            Self::NotASymbol => "the published name is not a legal symbol",
            Self::NoRowInTheExchangesFile => {
                "the exchange's own constituent file has no row for this name"
            }
            Self::TheExchangesRowNamesNoIsin => {
                "the exchange's own row for this name names no ISIN"
            }
        }
    }

    /// Whether the published NAME is what is unusable, rather than the
    /// exchange's ISIN column being empty.
    ///
    /// The two are one `match` apart and they are not the same complaint.
    /// `true` puts the row in [`TierJoin::malformed`] — this repository holds a
    /// name it cannot make a key of. `false` puts it in
    /// [`TierJoin::no_nse_isin`] — the name is fine, and NSE prints no ISIN
    /// beside it. Reporting the second as the first would tell an operator to
    /// go and fix a transcription that is correct.
    #[must_use]
    pub const fn is_a_malformed_name(self) -> bool {
        match self {
            Self::PlaceholderScrip | Self::NotASymbol => true,
            Self::NoRowInTheExchangesFile | Self::TheExchangesRowNamesNoIsin => false,
        }
    }
}

/// A constituent this vendor lists, with the id a request must name it by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Matched {
    /// The constituent, as the exchange publishes it.
    pub symbol: &'static str,
    /// **NSE's own ISIN for this symbol**, which is what the join was keyed on.
    pub isin: Isin,
    /// The vendor's own id — `groww_symbol`, or Dhan's `securityId`.
    pub id: VendorId,
    /// Which mastered vendors' files carry a row for [`Self::isin`].
    ///
    /// Not evidence for the join — the join needs none, because the ISIN is
    /// the exchange's own. It is the cross-check ON the exchange's column: a
    /// row only one master's file agrees with is a row where NSE's transcribed
    /// ISIN has one independent confirmation rather than two. Always contains
    /// the vendor whose answer this is. See [`TierJoin::uncorroborated`].
    pub corroboration: VendorSet,
}

/// A constituent whose NSE ISIN this vendor's master does not carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lacks {
    /// The constituent, as the exchange publishes it.
    pub symbol: &'static str,
    /// NSE's own ISIN for it, which this vendor's master has no row for.
    pub isin: Isin,
    /// Which mastered vendors' files DO carry a row for it. Empty when no
    /// master this build reads has ever seen the ISIN the exchange printed.
    pub corroboration: VendorSet,
}

/// A constituent more than one row of this vendor's master claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ambiguous {
    /// The constituent, as the exchange publishes it.
    pub symbol: &'static str,
    /// NSE's own ISIN for it, which two or more of this vendor's rows carry.
    pub isin: Isin,
    /// Every id that claims it. **All of them**, because picking one is how
    /// the wrong `securityId` reaches a request.
    pub ids: Vec<VendorId>,
}

/// A published constituent with no ISIN to join on, and why.
///
/// One type for both of the reasons a name never reaches the index — the
/// malformed name and the exchange's empty column — because the row is the
/// same shape either way and [`NoIsin::is_a_malformed_name`] is what decides
/// which bucket it is filed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unjoinable {
    /// The constituent, as the exchange publishes it.
    pub symbol: &'static str,
    /// Why nothing could be joined on.
    pub why: NoIsin,
}

/// One `(vendor, tier)` answer: the ids, and the whole of what did not resolve.
#[derive(Debug, Default)]
pub struct TierJoin {
    /// Constituents this vendor lists under NSE's ISIN, one row each.
    pub matched: Vec<Matched>,
    /// Constituents whose NSE ISIN this vendor's master does not carry.
    pub lacks: Vec<Lacks>,
    /// Constituents more than one of this vendor's rows claims.
    pub ambiguous: Vec<Ambiguous>,
    /// Published names this build cannot make a key of at all.
    pub malformed: Vec<Unjoinable>,
    /// Published names the exchange's own file names no ISIN for.
    ///
    /// The bucket D-0125 added. It is the same size for every vendor, because
    /// it is a fact about NSE's file and not about any master — asserted by
    /// `the_exchanges_own_gap_is_the_same_for_every_feed`.
    pub no_nse_isin: Vec<Unjoinable>,
    /// The ids [`Self::matched`] resolved to, in list order, so a caller takes
    /// a slice rather than folding the rows again.
    ids: Vec<VendorId>,
}

/// The empty answer, returned for an index no vendor slot occupies.
///
/// A `static` rather than a constructed value so [`Join::tier`] can hand back a
/// reference without allocating, and so the eager argument of `unwrap_or` costs
/// one pointer.
static NOTHING: TierJoin = TierJoin {
    matched: Vec::new(),
    lacks: Vec::new(),
    ambiguous: Vec::new(),
    malformed: Vec::new(),
    no_nse_isin: Vec::new(),
    ids: Vec::new(),
};

impl TierJoin {
    /// The vendor ids this tier resolves to, in published order.
    #[must_use]
    pub fn ids(&self) -> &[VendorId] {
        &self.ids
    }

    /// How many constituents were accounted for, across all five buckets.
    ///
    /// Equal to [`Tier::published`] for every vendor and every tier, and that
    /// equality is the invariant this module exists to hold.
    #[must_use]
    pub fn accounted(&self) -> usize {
        self.matched.len()
            + self.lacks.len()
            + self.ambiguous.len()
            + self.malformed.len()
            + self.no_nse_isin.len()
    }

    /// Matched rows exactly ONE master's file corroborates.
    ///
    /// What [`Matched::corroboration`] documents, listed row by row. This is
    /// **not** a weakness in the join — the join is keyed on the exchange's own
    /// ISIN and needs no vendor to agree with it. It is the one cross-check
    /// available on NSE's transcribed column: an ISIN two independently
    /// published master files both carry is an ISIN two files agree the
    /// exchange got right. An empty result means every ISIN this tier joined on
    /// appears in both masters.
    #[must_use]
    pub fn uncorroborated(&self) -> Vec<&Matched> {
        self.matched
            .iter()
            .filter(|m| {
                Vendor::MASTERED
                    .iter()
                    .filter(|v| m.corroboration.contains(**v))
                    .count()
                    < Vendor::MASTERED.len()
            })
            .collect()
    }
}

/// Every id that claims one `(exchange, ISIN)`, by vendor.
///
/// A `Vec` per vendor rather than one id: **two rows of the same master can
/// carry the same ISIN** — a company listed on two series is one paper with two
/// tickers — and collapsing them to the first is how a request comes to name
/// the wrong one of them. An empty `Vec` allocates nothing, so the common case
/// costs four pointers and one allocation.
#[derive(Debug, Default, Clone)]
struct Claim {
    per_vendor: [Vec<VendorId>; Vendor::ALL.len()],
}

impl Claim {
    /// The ids one vendor claims this ISIN with. Empty when it lists none.
    fn of(&self, vendor: Vendor) -> &[VendorId] {
        self.per_vendor
            .get(vendor as usize)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Which mastered vendors' files carry a row for this ISIN.
    ///
    /// A fold over two, decided while the tier is bucketed rather than stored,
    /// because it is one bit per mastered vendor and the `Vec`s are already in
    /// hand.
    fn corroboration(&self) -> VendorSet {
        Vendor::MASTERED
            .into_iter()
            .filter(|v| !self.of(*v).is_empty())
            .fold(VendorSet::EMPTY, VendorSet::with)
    }

    /// Records one vendor's id for this ISIN, when that vendor has one.
    ///
    /// Takes the `Option` rather than being called inside an `if let`, and
    /// both callers hand it one: the agreeing-vendor loop passes `None` for
    /// every vendor that does not list the key, so the empty arm is a path
    /// ordinary data takes. Lifting the check to the disputing caller instead
    /// would put a second `let` on a value `merge` sets in the same iteration
    /// it records the conflict -- a false arm no test could ever enter, which
    /// is the uncoverable region `CLAUDE.md` S9 forbids.
    fn push(&mut self, vendor: Vendor, id: Option<VendorId>) {
        if let Some(id) = id
            && let Some(slot) = self.per_vendor.get_mut(vendor as usize)
        {
            slot.push(id);
        }
    }
}

/// One published name and the ISIN behind it, resolved once.
///
/// A named type rather than a tuple because it crosses two functions and is
/// held for every constituent of every tier: the resolution is a fact about the
/// exchange's own file alone, so it is computed once and reused for all four
/// vendors rather than recomputed per vendor. Under D-0125 it does not consult
/// the merged universe at all, which is why the type no longer carries a
/// [`VendorSet`] beside the ISIN.
#[derive(Debug, Clone, Copy)]
struct Resolved {
    /// The constituent, as the exchange publishes it.
    symbol: &'static str,
    /// NSE's own ISIN for it, or why the exchange's file names none.
    identity: Result<Isin, NoIsin>,
}

/// The built join: one index, and one answer per `(vendor, tier)`.
///
/// Constructed by [`Join::build`] and never field by field, for the reason
/// [`crate::server::Read`] gives about its own catalog: a caller that filled
/// the fields by hand would get a lookup that silently went back to scanning.
#[derive(Debug)]
pub struct Join {
    /// Every `(exchange, ISIN)` any vendor asserted, and who claims it.
    by_isin: HashMap<(Exchange, Isin), Claim>,
    /// One answer per `(vendor, tier)`, in one flat array, indexed by
    /// `vendor as usize * Tier::ALL.len() + tier as usize`.
    tiers: Vec<TierJoin>,
}

impl Join {
    /// Builds the index and every `(vendor, tier)` answer, once.
    ///
    /// # Cost
    ///
    /// One pass over the merged universe to build the ISIN index, then one
    /// constant-time resolution per constituent per tier (1,813 on the six
    /// lists this build carries) and one probe per constituent per vendor.
    /// Nothing here is on a request path: `Read::new` calls it where
    /// `Catalog::build` is called, which is once per process — D-0039.
    ///
    /// The constant-time part is proved by
    /// `api::constituents::the_two_lookups_do_not_grow_with_the_universe`, which
    /// holds both lookups to a ratio ceiling across two universe sizes rather
    /// than to a duration.
    #[must_use]
    pub fn build(merged: &Merged) -> Self {
        let by_isin = index_by_isin(merged);

        // THE IDENTITY IS RESOLVED ONCE PER CONSTITUENT, NOT ONCE PER VENDOR.
        // It is a fact about the exchange's own constituent file and it does
        // not change when the vendor asking does — which is truer under D-0125
        // than it was before it, because the merged universe is no longer
        // consulted to establish it.
        let mut tiers = Vec::with_capacity(Vendor::ALL.len() * Tier::ALL.len());
        let resolved: Vec<Vec<Resolved>> = Tier::ALL
            .into_iter()
            .map(|tier| {
                tier.members()
                    .iter()
                    .map(|name| Resolved {
                        symbol: name,
                        identity: nse_identity_of(name),
                    })
                    .collect()
            })
            .collect();
        // `resolved` is in `Tier::ALL` order and stays in it, which is what
        // makes the flat index in `tier` correct rather than merely plausible.
        for vendor in Vendor::ALL {
            for names in &resolved {
                tiers.push(bucket(&by_isin, vendor, names));
            }
        }
        Self { by_isin, tiers }
    }

    /// What one tier resolves to for one vendor. **One array index.**
    #[must_use]
    pub fn tier(&self, vendor: Vendor, tier: Tier) -> &TierJoin {
        self.tiers
            .get(vendor as usize * Tier::ALL.len() + tier as usize)
            .unwrap_or(&NOTHING)
    }

    /// The vendor's id for one `(exchange, ISIN)`. **One hash probe.**
    ///
    /// `None` when that vendor lists no row for the ISIN **and** when more than
    /// one of its rows claims it. The two are different facts and a caller that
    /// must tell them apart reads [`TierJoin::lacks`] and
    /// [`TierJoin::ambiguous`]; what this must never do is return one of two
    /// candidates, because filing a request under the wrong `securityId` is
    /// indistinguishable from success until the bars are wrong.
    #[must_use]
    pub fn id(&self, vendor: Vendor, exchange: Exchange, isin: Isin) -> Option<VendorId> {
        let ids = self
            .by_isin
            .get(&(exchange, isin))
            .map(|claim| claim.of(vendor))
            .unwrap_or_default();
        let mut found = ids.iter();
        match (found.next(), found.next()) {
            (Some(only), None) => Some(*only),
            _ => None,
        }
    }

    /// The vendor ids ONE SPOT TARGET resolves to for ONE FEED, in published
    /// order. **One array index.**
    ///
    /// This is the question `/ingest` asks and had no answer to: the operator
    /// picks a feed and a universe, and what a pull needs from those two words
    /// is a list of that vendor's own ids. [`crate::ingest::SpotTarget::tier`]
    /// says which published list defines the target, [`Self::tier`] says what
    /// that list resolved to for the vendor, and this composes them so a caller
    /// holds one call rather than two and cannot pair the wrong halves.
    ///
    /// # `None` is not an empty list
    ///
    /// Two of the seven targets are not defined by a published list at all —
    /// the swept pair is the engine surface and the reference indices are
    /// whatever a master calls an index series — so there is no join to read
    /// and `None` says exactly that. An empty slice would say "this feed
    /// reaches nothing in it", which is a different and false claim; those two
    /// are counted off the merged universe by [`crate::coverage::Coverage`].
    #[must_use]
    pub fn ids_for(
        &self,
        vendor: Vendor,
        target: crate::ingest::SpotTarget,
    ) -> Option<&[VendorId]> {
        Some(self.tier(vendor, target.tier()?).ids())
    }

    /// How many distinct `(exchange, ISIN)` pairs the index holds.
    #[must_use]
    pub fn indexed(&self) -> usize {
        self.by_isin.len()
    }

    /// One line per `(vendor, tier)`, plus a line for every bucket that is not
    /// empty.
    ///
    /// A tier that resolves to nothing is a RESULT and says so with its
    /// reasons, rather than being absent from the report — an absent line is
    /// how "no target" came to be the only thing an operator was told.
    #[must_use]
    pub fn notes(&self) -> Vec<String> {
        let mut out = Vec::new();
        for vendor in Vendor::MASTERED {
            for tier in Tier::ALL {
                let join = self.tier(vendor, tier);
                let mut line = format!(
                    "{} · {}: {} matched, {} vendor lacks, {} ambiguous, {} malformed, \
                     {} no NSE ISIN — {} of {} published",
                    vendor.as_str(),
                    tier.label(),
                    join.matched.len(),
                    join.lacks.len(),
                    join.ambiguous.len(),
                    join.malformed.len(),
                    join.no_nse_isin.len(),
                    join.accounted(),
                    tier.published(),
                );
                let alone = join.uncorroborated().len();
                if alone > 0 {
                    let _ = write!(
                        line,
                        "; {alone} joined on an NSE ISIN only ONE master's file also carries"
                    );
                }
                out.push(line);
                out.extend(unresolved_lines(vendor, tier, join));
            }
        }
        out
    }
}

/// The rows of one `(vendor, tier)` that did not become an id, named.
fn unresolved_lines(vendor: Vendor, tier: Tier, join: &TierJoin) -> Vec<String> {
    let mut out = Vec::new();
    let mut lacks: Vec<&str> = join.lacks.iter().map(|l| l.symbol).collect();
    lacks.sort_unstable();
    if !lacks.is_empty() {
        out.push(format!(
            "{} · {} NOT LISTED · {} name(s) whose NSE ISIN this master has no row for: {}",
            vendor.as_str(),
            tier.label(),
            lacks.len(),
            lacks.join(", ")
        ));
    }
    for row in &join.ambiguous {
        let ids: Vec<&str> = row.ids.iter().map(VendorId::as_str).collect();
        out.push(format!(
            "{} · {} AMBIGUOUS · {} carries NSE ISIN {} on {} rows: {} — no id is chosen",
            vendor.as_str(),
            tier.label(),
            row.symbol,
            row.isin.as_str(),
            row.ids.len(),
            ids.join(" ")
        ));
    }
    out.extend(reason_lines(vendor, tier, "NO ISIN", &join.malformed));
    out.extend(reason_lines(vendor, tier, "NO NSE ISIN", &join.no_nse_isin));
    out
}

/// One line per distinct reason in an unjoinable bucket, names sorted.
///
/// Shared by the two unjoinable buckets rather than written twice: the grouping
/// is the same operation and a second copy is free to drift into printing one
/// bucket's names under the other's heading.
fn reason_lines(vendor: Vendor, tier: Tier, heading: &str, rows: &[Unjoinable]) -> Vec<String> {
    let mut by_reason: std::collections::BTreeMap<&'static str, Vec<&str>> =
        std::collections::BTreeMap::new();
    for row in rows {
        by_reason
            .entry(row.why.reason())
            .or_default()
            .push(row.symbol);
    }
    by_reason
        .into_iter()
        .map(|(reason, mut names)| {
            names.sort_unstable();
            format!(
                "{} · {} {} · {} name(s) — {}: {}",
                vendor.as_str(),
                tier.label(),
                heading,
                names.len(),
                reason,
                names.join(", ")
            )
        })
        .collect()
}

/// Every `(exchange, ISIN)` a vendor master asserted, and which of that
/// vendor's ids assert it.
///
/// # Each id is filed under the ISIN ITS OWN vendor spelled
///
/// [`crate::merge::Entry`] keeps one ISIN per instrument key plus the different
/// one a second vendor gave for the same key ([`crate::merge::Entry::conflict`],
/// D-0020). The old join filed every vendor's id under the FIRST ISIN, which
/// was harmless while the join started from that same entry — it was going to
/// arrive back at the row it came from either way. Under NSE's key it is not
/// harmless: a disputed name would hand back the id of a vendor whose master
/// spells that paper with a different ISIN entirely, which is a request filed
/// against the wrong instrument. So the disputing vendor's id is filed under
/// the ISIN it actually gave, and nothing is filed under an ISIN its own master
/// did not name.
fn index_by_isin(merged: &Merged) -> HashMap<(Exchange, Isin), Claim> {
    let mut by_isin: HashMap<(Exchange, Isin), Claim> = HashMap::with_capacity(merged.by_key.len());
    for (key, entry) in &merged.by_key {
        // An index has no ISIN and is not skipped quietly — it is the
        // `no_nse_isin` bucket, decided at resolution off the exchange's own
        // file, where the name that asked for it is still in hand.
        let Some((_, isin)) = entry.isin else {
            continue;
        };
        let disputing = entry.conflict.map(|(vendor, _)| vendor);
        let claim = by_isin.entry((key.exchange, isin)).or_default();
        for vendor in Vendor::ALL {
            if Some(vendor) == disputing {
                continue;
            }
            claim.push(vendor, entry.ids.get(vendor as usize).copied().flatten());
        }
        if let Some((vendor, disputed)) = entry.conflict {
            by_isin
                .entry((key.exchange, disputed))
                .or_default()
                .push(vendor, entry.ids.get(vendor as usize).copied().flatten());
        }
    }
    by_isin
}

/// The ISIN **the exchange itself** prints beside one published name, or why it
/// prints none.
///
/// # Two constant probes, and why not one
///
/// [`brutex_core::universe::nse_isin`] answers `None` for a name the file does
/// not list AND for a name whose row names no ISIN, because a caller that wants
/// something to join on wants the same answer to both. This module is the
/// caller that wants them apart — one is the exchange not listing indices among
/// its share constituents and the other is `docs/06-limits.md` §11's single
/// empty cell — so it asks
/// [`brutex_core::universe::NTM_INDEX`]`::position` first and `nse_isin`
/// second. Both are hash-mask-probe with a bound `core` pins, so the pair is
/// still constant.
///
/// The alternative is one probe plus `NIFTY_TOTAL_MARKET_ISIN.get(at)`, which
/// costs a `None` arm no test could ever enter — the uncoverable region
/// `CLAUDE.md` §9's 100% floor forbids, and the same trade `core` states in
/// `nse_isin`'s own `expect` attribute.
fn nse_identity_of(name: &str) -> Result<Isin, NoIsin> {
    if NSE_PLACEHOLDER_SCRIPS.contains(&name) {
        return Err(NoIsin::PlaceholderScrip);
    }
    if Symbol::new(name).is_err() {
        return Err(NoIsin::NotASymbol);
    }
    if universe::NTM_INDEX.position(name).is_none() {
        return Err(NoIsin::NoRowInTheExchangesFile);
    }
    universe::nse_isin(name).ok_or(NoIsin::TheExchangesRowNamesNoIsin)
}

/// Sorts one tier's resolved names into the five buckets for one vendor.
fn bucket(
    by_isin: &HashMap<(Exchange, Isin), Claim>,
    vendor: Vendor,
    names: &[Resolved],
) -> TierJoin {
    let mut out = TierJoin::default();
    for Resolved { symbol, identity } in names.iter().copied() {
        let isin = match identity {
            Ok(found) => found,
            Err(why) if why.is_a_malformed_name() => {
                out.malformed.push(Unjoinable { symbol, why });
                continue;
            }
            Err(why) => {
                out.no_nse_isin.push(Unjoinable { symbol, why });
                continue;
            }
        };
        // THE ONE PROBE. The key is the exchange's ISIN and the answer is this
        // vendor's own rows; nothing here knows what the vendor calls it.
        let claim = by_isin.get(&(Exchange::Nse, isin));
        let corroboration = claim.map_or(VendorSet::EMPTY, Claim::corroboration);
        let ids = claim.map(|c| c.of(vendor)).unwrap_or_default();
        let mut found = ids.iter();
        match (found.next(), found.next()) {
            (None, _) => out.lacks.push(Lacks {
                symbol,
                isin,
                corroboration,
            }),
            (Some(only), None) => {
                out.ids.push(*only);
                out.matched.push(Matched {
                    symbol,
                    isin,
                    id: *only,
                    corroboration,
                });
            }
            (Some(_), Some(_)) => out.ambiguous.push(Ambiguous {
                symbol,
                isin,
                ids: ids.to_vec(),
            }),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::merge::{Entry, Source, merge};
    use brutex_core::instrument::{InstrumentKey, Kind, Segment};
    use brutex_core::vendor::Listing;
    use std::hint::black_box;
    use std::time::Instant;

    /// The ISINs NSE ITSELF prints beside these four names, read out of
    /// `crates/core/src/universe.rs` — which transcribed them from the
    /// exchange's own files under D-0122. They are the values the join keys on,
    /// so a fixture that used anything else would be testing a join nobody
    /// runs.
    const RELIANCE: &str = "INE002A01018";
    const TCS: &str = "INE467B01029";
    const INFY: &str = "INE009A01021";
    const HDFCBANK: &str = "INE040A01034";
    /// A real ISIN belonging to a company **no list this build carries names**
    /// — `GRINDWELL`, which is in neither the Total Market array nor any NIFTY
    /// tier. It is what the disputing vendor's row wears in the fixture below,
    /// so the dispute cannot accidentally resolve some other constituent.
    const OFF_THE_LISTS: &str = "INE536A01023";

    fn isin(text: &str) -> Isin {
        Isin::new(text).expect("a real ISIN verifies")
    }

    fn key(symbol: &str, kind: Kind) -> InstrumentKey {
        InstrumentKey {
            exchange: Exchange::Nse,
            segment: if kind == Kind::Index {
                Segment::Index
            } else {
                Segment::Cash
            },
            underlying: Symbol::new(symbol).expect("a legal symbol"),
            kind,
        }
    }

    fn equity(symbol: &str, code: &str, id: &str) -> Listing {
        Listing {
            vendor_id: VendorId::new(id).expect("a legal id"),
            key: key(symbol, Kind::Equity),
            isin: Some(isin(code)),
            unsuffixed: None,
        }
    }

    fn spot_index(symbol: &str, id: &str) -> Listing {
        Listing {
            vendor_id: VendorId::new(id).expect("a legal id"),
            key: key(symbol, Kind::Index),
            isin: None,
            unsuffixed: None,
        }
    }

    fn source(vendor: Vendor, kept: Vec<Listing>) -> Source {
        Source {
            vendor,
            kept,
            declined: Vec::new(),
        }
    }

    /// A universe carrying every shape the buckets exist for.
    ///
    /// * `RELIANCE` — both masters, NSE's ISIN. Matched for both.
    /// * `TCS` — TWO Groww rows on one ISIN, one Dhan row. Ambiguous for
    ///   Groww, matched for Dhan.
    /// * `INFY` — Dhan only. Matched for Dhan with ONE file corroborating,
    ///   lacked by Groww.
    /// * `HDFCBANK` — **the dispute.** Groww's row wears NSE's ISIN for it and
    ///   Dhan's row wears an ISIN belonging to a company on no list at all. NSE
    ///   decides: Groww matches, Dhan lacks, and the Dhan row is reachable only
    ///   under the ISIN Dhan actually gave it.
    /// * `NIFTY` — a spot index, listed by both, with no ISIN anywhere.
    fn universe_with_every_shape() -> Merged {
        merge(&[
            source(
                Vendor::Groww,
                vec![
                    equity("RELIANCE", RELIANCE, "NSE-RELIANCE"),
                    // TWO GROWW ROWS, ONE ISIN. A company listed on two series
                    // is one paper with two tickers, and this is what makes the
                    // join ambiguous for Groww and not for Dhan.
                    equity("TCS", TCS, "NSE-TCS"),
                    equity("TCS-BE", TCS, "NSE-TCS-BE"),
                    equity("HDFCBANK", HDFCBANK, "NSE-HDFCBANK"),
                    spot_index("NIFTY", "NSE-NIFTY"),
                ],
            ),
            source(
                Vendor::Dhan,
                vec![
                    equity("RELIANCE", RELIANCE, "2885"),
                    equity("TCS", TCS, "11536"),
                    // Only Dhan lists INFY, so NSE's ISIN for it has one
                    // corroborating file rather than two.
                    equity("INFY", INFY, "1594"),
                    // THE DISPUTE. Dhan spells HDFCBANK with another company's
                    // ISIN entirely.
                    equity("HDFCBANK", OFF_THE_LISTS, "1333"),
                    spot_index("NIFTY", "13"),
                ],
            ),
        ])
    }

    /// A universe both masters agree about completely — one name, one ISIN,
    /// two files. Nothing in it is uncorroborated, which is what makes it the
    /// negative case for the note clause.
    fn universe_both_files_corroborate() -> Merged {
        merge(&[
            source(
                Vendor::Groww,
                vec![equity("RELIANCE", RELIANCE, "NSE-RELIANCE")],
            ),
            source(Vendor::Dhan, vec![equity("RELIANCE", RELIANCE, "2885")]),
        ])
    }

    #[test]
    fn every_tiers_published_count_matches_the_list_it_borrows() {
        // THE CHEAPEST CHECK THAT THE LIST CAME FROM THE FILE IT CLAIMS. A
        // truncated transcription still compiles and still answers every
        // membership question — for fewer names than the index has.
        // Every message below is built BEFORE its assertion. A format argument
        // written inside the macro is only evaluated when the assertion fails,
        // which is a region no passing run enters — and the coverage floor this
        // repository holds does not have an exemption for "it is only a test's
        // failure message".
        for tier in Tier::ALL {
            let named = tier.label();
            assert!(!named.is_empty());
            assert_eq!(
                tier.published(),
                tier.members().len(),
                "{named}: the published count and the borrowed list disagree"
            );
        }
        assert_eq!(Tier::ALL.len(), 6, "six lists, and the flat index knows it");
        // `ALL` is append-only because `Join::tier` indexes by position. A
        // reorder hands one tier's answer to another tier's caller, so the
        // order is pinned here rather than trusted.
        for (slot, tier) in Tier::ALL.into_iter().enumerate() {
            let named = tier.label();
            assert_eq!(slot, tier as usize, "{named} moved");
        }
    }

    #[test]
    fn no_tier_this_build_carries_holds_a_placeholder_scrip() {
        // D-0089 dropped `DUMMYINXGN` and `DUMMYTRVN` at transcription. This
        // asserts the drop rather than trusting it — and the check in
        // `nse_identity_of` stays, because the next rebalance is a new
        // transcription and may arrive mid-corporate-action.
        for tier in Tier::ALL {
            let named = tier.label();
            for name in tier.members() {
                assert!(
                    !NSE_PLACEHOLDER_SCRIPS.contains(name),
                    "{named}: {name} is a placeholder scrip, not a constituent"
                );
            }
        }
    }

    #[test]
    fn every_constituent_lands_in_exactly_one_bucket() {
        // THE PARTITION, FOR EVERY VENDOR AND EVERY TIER, NOW WITH FIVE
        // BUCKETS. Not just the sum: they are asserted DISJOINT and their union
        // asserted equal to the published list itself, name for name. A sum
        // alone would pass a join that dropped one name and double-counted
        // another.
        let join = Join::build(&universe_with_every_shape());
        for vendor in Vendor::ALL {
            for tier in Tier::ALL {
                let answer = join.tier(vendor, tier);
                // Built eagerly, for the reason
                // `every_tiers_published_count_matches_the_list_it_borrows`
                // gives: a message formatted inside the macro is a region no
                // passing run enters.
                let at = format!("{} · {}", vendor.as_str(), tier.label());
                // THE ARITHMETIC, PRINTED. `cargo test -- --nocapture` shows
                // the five addends and the total for all twenty-four rows, so
                // the partition is readable and not merely asserted.
                println!(
                    "  {at}: {} matched + {} lacks + {} ambiguous + {} malformed \
                     + {} no-nse-isin = {} of {} published",
                    answer.matched.len(),
                    answer.lacks.len(),
                    answer.ambiguous.len(),
                    answer.malformed.len(),
                    answer.no_nse_isin.len(),
                    answer.accounted(),
                    tier.published(),
                );
                let mut seen: Vec<&str> = Vec::new();
                seen.extend(answer.matched.iter().map(|r| r.symbol));
                seen.extend(answer.lacks.iter().map(|r| r.symbol));
                seen.extend(answer.ambiguous.iter().map(|r| r.symbol));
                seen.extend(answer.malformed.iter().map(|r| r.symbol));
                seen.extend(answer.no_nse_isin.iter().map(|r| r.symbol));
                assert_eq!(
                    answer.accounted(),
                    tier.published(),
                    "{at}: the buckets do not sum to the published count"
                );
                assert_eq!(
                    seen.len(),
                    tier.published(),
                    "{at}: a name was dropped or counted twice"
                );
                let mut sorted = seen.clone();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(
                    sorted.len(),
                    seen.len(),
                    "{at}: a name is in two buckets at once"
                );
                let mut published: Vec<&str> = tier.members().to_vec();
                published.sort_unstable();
                assert_eq!(
                    sorted, published,
                    "{at}: the buckets are not the published list"
                );
                // The ids are the matched rows, in list order, and nothing else.
                assert_eq!(answer.ids().len(), answer.matched.len());
            }
        }
    }

    #[test]
    fn every_matched_row_joined_on_the_isin_the_exchange_itself_prints() {
        // THE WHOLE OF D-0125 IN ONE ASSERTION. Whatever a vendor's master
        // spells, the key on the row is `core::universe::nse_isin` — NSE's own
        // column — for every matched and every lacked row of every tier.
        let join = Join::build(&universe_with_every_shape());
        let mut checked = 0usize;
        for vendor in Vendor::ALL {
            for tier in Tier::ALL {
                let answer = join.tier(vendor, tier);
                for row in &answer.matched {
                    let symbol = row.symbol;
                    assert_eq!(
                        Some(row.isin),
                        universe::nse_isin(symbol),
                        "{symbol}: matched on an ISIN the exchange did not print"
                    );
                    checked += 1;
                }
                for row in &answer.lacks {
                    let symbol = row.symbol;
                    assert_eq!(
                        Some(row.isin),
                        universe::nse_isin(symbol),
                        "{symbol}: lacked an ISIN the exchange did not print"
                    );
                    checked += 1;
                }
                for row in &answer.ambiguous {
                    let symbol = row.symbol;
                    assert_eq!(Some(row.isin), universe::nse_isin(symbol), "{symbol}");
                    checked += 1;
                }
            }
        }
        assert!(
            checked > 5_000,
            "every joinable row of every tier: {checked}"
        );
    }

    #[test]
    fn a_spot_target_resolves_to_one_feeds_ids_and_the_two_that_cannot_say_so() {
        // THE CALL `/ingest` NEEDED AND HAD NO WAY TO MAKE: a form field
        // naming a universe, plus a feed, becoming that vendor's own ids.
        // `SpotTarget::tier` and `Join::tier` are the two halves and this
        // composes them, so a caller cannot pair one target with another
        // tier's answer.
        let join = Join::build(&universe_with_every_shape());
        for target in crate::ingest::SpotTarget::ALL {
            let slug = target.slug();
            let Some(tier) = target.tier() else {
                // NOT AN EMPTY SLICE. `None` is "no published list defines
                // this", which is a different claim from "this feed reaches
                // nothing in it" and must not be rendered as one.
                assert_eq!(
                    join.ids_for(Vendor::Groww, target),
                    None,
                    "{slug} is not defined by a published list"
                );
                continue;
            };
            let joins = format!("{slug} joins through {}", tier.label());
            let ids = join.ids_for(Vendor::Groww, target).expect(&joins);
            assert_eq!(
                ids,
                join.tier(Vendor::Groww, tier).ids(),
                "{slug} must resolve to ITS tier's ids and not a neighbour's"
            );
        }
        // AND THE IDS ARE THE FEED'S OWN. RELIANCE is in all four tiers and
        // the two masters spell it differently, which is the whole reason the
        // question is asked per feed.
        let groww = join
            .ids_for(Vendor::Groww, crate::ingest::SpotTarget::Nifty50)
            .expect("the NIFTY 50 is a published list");
        let dhan = join
            .ids_for(Vendor::Dhan, crate::ingest::SpotTarget::Nifty50)
            .expect("the NIFTY 50 is a published list");
        assert!(
            groww.iter().any(|id| id.as_str() == "NSE-RELIANCE"),
            "{groww:?}"
        );
        assert!(dhan.iter().any(|id| id.as_str() == "2885"), "{dhan:?}");
        assert!(
            !groww.iter().any(|id| id.as_str() == "2885"),
            "one feed's ids never leak into another's: {groww:?}"
        );
    }

    #[test]
    fn a_constituent_both_masters_list_resolves_to_that_vendors_own_id() {
        // THE POINT OF THE JOIN: a tier becomes the ids a request must name,
        // and the two vendors' ids for one ISIN are different strings.
        let join = Join::build(&universe_with_every_shape());
        let dhan = join.tier(Vendor::Dhan, Tier::Nifty50);
        let groww = join.tier(Vendor::Groww, Tier::Nifty50);
        let dhan_reliance = dhan
            .matched
            .iter()
            .find(|r| r.symbol == "RELIANCE")
            .expect("Dhan lists RELIANCE");
        let groww_reliance = groww
            .matched
            .iter()
            .find(|r| r.symbol == "RELIANCE")
            .expect("Groww lists RELIANCE");
        assert_eq!(dhan_reliance.id.as_str(), "2885");
        assert_eq!(groww_reliance.id.as_str(), "NSE-RELIANCE");
        assert_eq!(dhan_reliance.isin, isin(RELIANCE), "one key, both vendors");
        assert_eq!(groww_reliance.isin, isin(RELIANCE));
        // Both files carry NSE's ISIN, so the exchange's column has two
        // independent confirmations here rather than one.
        for vendor in Vendor::MASTERED {
            assert!(dhan_reliance.corroboration.contains(vendor));
        }
        assert!(
            join.tier(Vendor::Dhan, Tier::Nifty50)
                .ids()
                .iter()
                .any(|id| id.as_str() == "2885"),
            "the id is in the tier's answer, not only on the row"
        );
        // And the same question asked by ISIN is one probe with the same answer.
        assert_eq!(
            join.id(Vendor::Dhan, Exchange::Nse, isin(RELIANCE))
                .map(|id| id.as_str().to_owned()),
            Some("2885".to_owned())
        );
    }

    #[test]
    fn a_constituent_one_master_lacks_is_reported_and_never_dropped() {
        // Only Dhan carries NSE's ISIN for INFY. Groww's answer must SAY SO on
        // a row — a join that quietly shortened Groww's list by one is the
        // failure this module exists to make impossible.
        let join = Join::build(&universe_with_every_shape());
        let groww = join.tier(Vendor::Groww, Tier::Nifty50);
        let missing = groww
            .lacks
            .iter()
            .find(|r| r.symbol == "INFY")
            .expect("Groww carries no row for INFY's ISIN");
        assert_eq!(missing.isin, isin(INFY), "NSE's ISIN is on the row");
        assert!(missing.corroboration.contains(Vendor::Dhan));
        assert!(!missing.corroboration.contains(Vendor::Groww));
        assert!(
            !groww.matched.iter().any(|r| r.symbol == "INFY"),
            "a name this vendor lacks is never also matched"
        );
        assert_eq!(
            join.id(Vendor::Groww, Exchange::Nse, isin(INFY)),
            None,
            "no id is substituted from the other vendor"
        );
        // Dhan's own row for the same name IS matched, and it is the one only
        // one published file corroborates.
        let dhan = join.tier(Vendor::Dhan, Tier::Nifty50);
        let alone = dhan.uncorroborated();
        assert!(
            alone.iter().any(|r| r.symbol == "INFY"),
            "a single-file corroboration is named on its own row: {alone:?}"
        );
        assert!(
            !join
                .tier(Vendor::Dhan, Tier::Nifty50)
                .uncorroborated()
                .iter()
                .any(|r| r.symbol == "RELIANCE"),
            "a row both files carry is not in that list"
        );
    }

    #[test]
    fn the_symbol_step_is_gone_and_a_vendor_row_spelled_right_still_lacks() {
        // THE TEST D-0125 EXISTS FOR. Dhan's master HAS a row whose symbol is
        // `HDFCBANK` — the old join found it by name, took whatever ISIN was on
        // it and matched. Under NSE's key that row wears another company's
        // ISIN, so Dhan LACKS HDFCBANK and its id is unreachable from that
        // name. A symbol fallback, silent or loud, would put `1333` back.
        let join = Join::build(&universe_with_every_shape());
        let dhan = join.tier(Vendor::Dhan, Tier::Nifty50);
        let lacked = dhan
            .lacks
            .iter()
            .find(|r| r.symbol == "HDFCBANK")
            .expect("Dhan's row for that name wears a different ISIN");
        assert_eq!(lacked.isin, isin(HDFCBANK), "the row names NSE's ISIN");
        assert!(
            !dhan.matched.iter().any(|r| r.symbol == "HDFCBANK"),
            "no fallback to the symbol"
        );
        // Built BEFORE the assertion, like every other message in this file: a
        // format argument written inside the macro is only evaluated when the
        // assertion fails, which is a region no passing run enters.
        let filed = format!(
            "and the id never reaches the request list: {:?}",
            dhan.ids()
        );
        assert!(
            !dhan.ids().iter().any(|id| id.as_str() == "1333"),
            "{filed}"
        );
        assert_eq!(join.id(Vendor::Dhan, Exchange::Nse, isin(HDFCBANK)), None);

        // NSE DECIDED THE DISPUTE, AND IT DECIDED FOR GROWW ON EVIDENCE. The
        // old join refused both sides as `DisputedIsin` because there was no
        // third opinion; there is one now, and it is the exchange's.
        let groww = join.tier(Vendor::Groww, Tier::Nifty50);
        let won = groww
            .matched
            .iter()
            .find(|r| r.symbol == "HDFCBANK")
            .expect("Groww's row wears NSE's ISIN for it");
        assert_eq!(won.id.as_str(), "NSE-HDFCBANK");
        assert_eq!(won.isin, isin(HDFCBANK));
        assert!(
            !won.corroboration.contains(Vendor::Dhan),
            "and the disagreement is on the row: only one file corroborates"
        );
        // The disputing vendor's id is still reachable — under the ISIN THAT
        // VENDOR gave it, which is the only honest place for it.
        assert_eq!(
            join.id(Vendor::Dhan, Exchange::Nse, isin(OFF_THE_LISTS))
                .map(|id| id.as_str().to_owned()),
            Some("1333".to_owned()),
            "nothing is dropped; it is filed where its own master put it"
        );
    }

    #[test]
    fn two_rows_of_one_master_claiming_one_isin_are_ambiguous_and_no_id_is_chosen() {
        let join = Join::build(&universe_with_every_shape());
        let groww = join.tier(Vendor::Groww, Tier::Nifty50);
        let row = groww
            .ambiguous
            .iter()
            .find(|r| r.symbol == "TCS")
            .expect("two Groww rows carry the TCS ISIN");
        assert_eq!(row.isin, isin(TCS));
        let mut ids: Vec<&str> = row.ids.iter().map(VendorId::as_str).collect();
        ids.sort_unstable();
        assert_eq!(
            ids,
            vec!["NSE-TCS", "NSE-TCS-BE"],
            "ALL the claimants, because picking one is how a request names the wrong instrument"
        );
        assert!(
            !groww
                .ids()
                .iter()
                .any(|id| id.as_str().starts_with("NSE-TCS")),
            "an ambiguous name contributes no id to the tier's answer"
        );
        assert_eq!(
            join.id(Vendor::Groww, Exchange::Nse, isin(TCS)),
            None,
            "two candidates is not an answer"
        );
        // The same ISIN is unambiguous for Dhan, which lists it once. The
        // ambiguity is a fact about ONE master, never about the ISIN.
        assert_eq!(
            join.id(Vendor::Dhan, Exchange::Nse, isin(TCS))
                .map(|id| id.as_str().to_owned()),
            Some("11536".to_owned())
        );
    }

    #[test]
    fn the_exchanges_own_gap_is_its_own_bucket_and_the_same_for_every_feed() {
        // THE BUCKET D-0125 ADDED, ON THE REAL LISTS. Six of 1,813 positions
        // carry no NSE ISIN, and which six is a fact about the exchange's file
        // — so the bucket is identical for all four vendors, including the two
        // that publish no master at all.
        let join = Join::build(&universe_with_every_shape());
        let names = |vendor: Vendor, tier: Tier| -> Vec<&str> {
            let mut out: Vec<&str> = join
                .tier(vendor, tier)
                .no_nse_isin
                .iter()
                .map(|r| r.symbol)
                .collect();
            out.sort_unstable();
            out
        };
        // The five F&O index underlyings: not shares, no numbering agency
        // issues an ISIN for a computed level, and NSE lists none of them in a
        // constituent file at all.
        assert_eq!(
            names(Vendor::Groww, Tier::FnoUnderlyings),
            vec!["BANKNIFTY", "FINNIFTY", "MIDCPNIFTY", "NIFTY", "NIFTYNXT50"]
        );
        // And the one Total Market position whose ISIN cell is empty —
        // `docs/06-limits.md` §11. It is NOT the same reason as the five.
        assert_eq!(names(Vendor::Groww, Tier::TotalMarket), vec!["AGL"]);
        let agl = join
            .tier(Vendor::Groww, Tier::TotalMarket)
            .no_nse_isin
            .first()
            .copied()
            .expect("one row");
        assert_eq!(agl.why, NoIsin::TheExchangesRowNamesNoIsin);
        let nifty = join
            .tier(Vendor::Groww, Tier::FnoUnderlyings)
            .no_nse_isin
            .iter()
            .find(|r| r.symbol == "NIFTY")
            .copied()
            .expect("one row");
        assert_eq!(nifty.why, NoIsin::NoRowInTheExchangesFile);
        for vendor in Vendor::ALL {
            for tier in Tier::ALL {
                let at = format!("{} · {}", vendor.as_str(), tier.label());
                assert_eq!(
                    names(vendor, tier),
                    names(Vendor::Groww, tier),
                    "{at}: the exchange's gap cannot depend on the feed"
                );
            }
        }
        // The four NIFTY tiers have no gap at all: every one of their 850
        // positions carries an NSE-issued ISIN.
        for tier in [
            Tier::Nifty50,
            Tier::Nifty100,
            Tier::Nifty200,
            Tier::Nifty500,
        ] {
            let at = tier.label();
            assert!(names(Vendor::Dhan, tier).is_empty(), "{at}");
        }
    }

    #[test]
    fn a_name_that_cannot_be_a_key_is_malformed_and_never_probed() {
        // The OTHER unjoinable bucket, and the reason it is not the same one.
        // No list this build carries holds such a name — `bucket` is therefore
        // handed the rows directly rather than through a tier, because a test
        // that could only run after a bad rebalance is a test that never runs.
        let merged = universe_with_every_shape();
        for name in NSE_PLACEHOLDER_SCRIPS {
            assert_eq!(
                nse_identity_of(name),
                Err(NoIsin::PlaceholderScrip),
                "{name} is a placeholder"
            );
        }
        // A space is not in `Symbol`'s alphabet, so this can never become a
        // key — and a name that cannot become a key is reported rather than
        // silently skipped.
        assert_eq!(nse_identity_of("RELIANCE LTD"), Err(NoIsin::NotASymbol));
        assert_eq!(nse_identity_of(""), Err(NoIsin::NotASymbol));

        let rows: Vec<Resolved> = ["DUMMYINXGN", "RELIANCE LTD", "RELIANCE"]
            .into_iter()
            .map(|name| Resolved {
                symbol: name,
                identity: nse_identity_of(name),
            })
            .collect();
        let answer = bucket(&index_by_isin(&merged), Vendor::Groww, &rows);
        let mut named: Vec<&str> = answer.malformed.iter().map(|r| r.symbol).collect();
        named.sort_unstable();
        assert_eq!(named, vec!["DUMMYINXGN", "RELIANCE LTD"]);
        assert!(
            answer.no_nse_isin.is_empty(),
            "a bad NAME is not the exchange's empty column"
        );
        assert_eq!(answer.matched.len(), 1, "and the good name still resolves");
        assert_eq!(answer.accounted(), 3, "the partition holds off a tier too");

        // The report names them under their own heading, grouped by reason.
        let lines = unresolved_lines(Vendor::Groww, Tier::Nifty50, &answer);
        assert!(
            lines
                .iter()
                .any(|l| l.contains("NO ISIN") && l.contains("DUMMYINXGN")),
            "{lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|l| l.contains("NO ISIN") && l.contains("not a legal symbol")),
            "{lines:?}"
        );
    }

    #[test]
    fn every_reason_prints_a_sentence_and_says_which_bucket_it_belongs_to() {
        // The split `bucket` reads. A reason that answered the wrong side of
        // it would file the exchange's own gap under this repository's
        // transcription, or the reverse.
        for (why, malformed) in [
            (NoIsin::PlaceholderScrip, true),
            (NoIsin::NotASymbol, true),
            (NoIsin::NoRowInTheExchangesFile, false),
            (NoIsin::TheExchangesRowNamesNoIsin, false),
        ] {
            let said = why.reason();
            assert!(!said.is_empty(), "{why:?} prints nothing");
            assert_eq!(
                why.is_a_malformed_name(),
                malformed,
                "{why:?} is filed in the wrong bucket"
            );
        }
        assert_eq!(
            NoIsin::PlaceholderScrip.reason(),
            "an NSE placeholder scrip, not a constituent"
        );
        assert_eq!(
            NoIsin::NotASymbol.reason(),
            "the published name is not a legal symbol"
        );
        assert_eq!(
            NoIsin::NoRowInTheExchangesFile.reason(),
            "the exchange's own constituent file has no row for this name"
        );
        assert_eq!(
            NoIsin::TheExchangesRowNamesNoIsin.reason(),
            "the exchange's own row for this name names no ISIN"
        );
    }

    #[test]
    fn a_vendor_with_no_master_lacks_every_name_and_the_sum_still_holds() {
        // An archive publishes no master, so it claims no ISIN. Every name the
        // exchange gives an ISIN for is `lacks` and nothing is matched — which
        // is a RESULT, printed, rather than an empty answer that reads like
        // success.
        let join = Join::build(&universe_with_every_shape());
        let archive = join.tier(Vendor::TrueData, Tier::Nifty50);
        assert!(archive.matched.is_empty());
        assert!(archive.ids().is_empty());
        assert!(archive.ambiguous.is_empty());
        assert!(archive.uncorroborated().is_empty(), "nothing is matched");
        assert_eq!(archive.accounted(), Tier::Nifty50.published());
        assert_eq!(
            archive.lacks.len(),
            50,
            "every NIFTY 50 name has an NSE ISIN this feed cannot carry"
        );
        assert!(
            archive.lacks.iter().any(|r| r.symbol == "RELIANCE"),
            "a name with a known ISIN this feed does not carry"
        );
        assert_eq!(
            join.id(Vendor::TrueData, Exchange::Nse, isin(RELIANCE)),
            None
        );
    }

    #[test]
    fn every_matched_row_also_carries_the_tiers_universe_bit() {
        // The join and `core::universe::of_equity` are built from the same six
        // lists by different routes. If they ever disagree, one of them is
        // reading a list the other is not.
        let join = Join::build(&universe_with_every_shape());
        for vendor in Vendor::MASTERED {
            for tier in Tier::ALL {
                let named = tier.label();
                for row in &join.tier(vendor, tier).matched {
                    let symbol = row.symbol;
                    assert!(
                        universe::of_equity(symbol).contains(tier.universe()),
                        "{symbol} is matched in {named} and carries no {named} bit"
                    );
                }
            }
        }
    }

    #[test]
    fn the_notes_name_the_five_buckets_their_sum_and_every_unresolved_row() {
        let join = Join::build(&universe_with_every_shape());
        let notes = join.notes();
        assert!(
            join.indexed() >= 3,
            "the ISIN index holds what it was given"
        );
        let head = notes
            .iter()
            .find(|l| l.starts_with("groww · NIFTY 50:"))
            .expect("one line per vendor per tier");
        assert!(
            head.contains("50 of 50 published"),
            "the sum is on the line: {head}"
        );
        assert!(head.contains("matched") && head.contains("vendor lacks"));
        assert!(head.contains("ambiguous") && head.contains("malformed"));
        assert!(
            head.contains("no NSE ISIN"),
            "the fifth bucket is on the line too: {head}"
        );
        // The F&O line is where the fifth bucket is not zero.
        let fno = notes
            .iter()
            .find(|l| l.starts_with("groww · F&O underlyings:"))
            .expect("one line per vendor per tier");
        assert!(fno.contains("5 no NSE ISIN"), "{fno}");
        assert!(
            notes.iter().any(|l| l.contains("NO NSE ISIN")
                && l.contains("has no row for this name")
                && l.contains("BANKNIFTY")),
            "the exchange's gap is named under its own heading"
        );
        assert!(
            notes
                .iter()
                .any(|l| l.contains("NO NSE ISIN") && l.contains("AGL")),
            "and so is the empty cell"
        );
        assert!(
            notes
                .iter()
                .any(|l| l.contains("AMBIGUOUS") && l.contains("NSE-TCS-BE")),
            "every claimant is named"
        );
        assert!(
            notes
                .iter()
                .any(|l| l.contains("NOT LISTED") && l.contains("INFY")),
            "a name a feed lacks is named, never a bare count"
        );
        // The corroboration clause is on the line it AFFECTED. Groww matched
        // HDFCBANK, which only its own file carries, so its NIFTY 50 line
        // carries the clause.
        assert!(
            head.contains("1 joined on an NSE ISIN only ONE master's file also carries"),
            "{head}"
        );
        // AND ON NO OTHER. A universe both files corroborate produces the same
        // line without the clause — a warning that is always printed is a
        // warning nobody reads.
        let quiet = Join::build(&universe_both_files_corroborate());
        let quiet_head = quiet
            .notes()
            .into_iter()
            .find(|l| l.starts_with("groww · NIFTY 50:"))
            .expect("one line per vendor per tier");
        assert!(
            quiet_head.contains("1 matched") && quiet_head.contains("50 of 50 published"),
            "{quiet_head}"
        );
        assert!(
            !quiet_head.contains("only ONE master's file"),
            "a line with nothing uncorroborated must not carry the clause: {quiet_head}"
        );
        // Every tier of every mastered vendor gets a line, so a tier that
        // resolves to nothing cannot be silently absent from the report.
        for vendor in Vendor::MASTERED {
            for tier in Tier::ALL {
                let want = format!("{} · {}:", vendor.as_str(), tier.label());
                assert!(
                    notes.iter().any(|l| l.starts_with(&want)),
                    "no line for {want}"
                );
            }
        }
    }

    /// How many times each measurement repeats before the minimum is taken.
    /// The minimum is the least-disturbed run, which is what makes this
    /// survivable on a shared machine.
    const TRIALS: u32 = 9;
    /// Lookups per trial.
    const REPS: u32 = 200_000;
    /// The most the big universe may cost per lookup against the small one,
    /// in thousandths. 4.0x — three times the 3.0x `crates/api/benches/ratio.rs`
    /// holds page rendering to, because a single probe is nanoseconds and the
    /// noise floor is a larger share of it.
    const CEILING_PERMILLE: u128 = 4_000;

    /// Picoseconds per operation, best of [`TRIALS`].
    fn cost_ps(mut op: impl FnMut() -> usize) -> u128 {
        let mut best = u128::MAX;
        for _ in 0..TRIALS {
            let started = Instant::now();
            for _ in 0..REPS {
                black_box(op());
            }
            let ns = started.elapsed().as_nanos();
            best = best.min(ns * 1_000 / u128::from(REPS));
        }
        best
    }

    /// A universe of `n` instruments, in the shape the bench builds one.
    ///
    /// The ISIN body is a counter, so roughly one in ten passes the check
    /// digit and the rest are `None` — a real state a master row reaches. What
    /// matters here is that the big universe indexes thousands of ISINs and
    /// the small one indexes a handful.
    fn universe_of(n: usize) -> Merged {
        let mut by_key = std::collections::HashMap::with_capacity(n);
        for i in 0..n {
            by_key.insert(
                key(&format!("S{i:07}"), Kind::Equity),
                Entry {
                    ids: [None; Vendor::ALL.len()],
                    vendors: VendorSet::EMPTY.with(Vendor::Dhan),
                    isin: Isin::new(&format!("INE{:09}", i % 1_000_000_000))
                        .ok()
                        .map(|x| (Vendor::Dhan, x)),
                    conflict: None,
                    universe: Universe::TOTAL_MARKET,
                },
            );
        }
        Merged {
            by_key,
            conflicts: Vec::new(),
            eligibility: Vec::new(),
        }
    }

    #[test]
    fn the_two_lookups_do_not_grow_with_the_universe() {
        // GATE ON THE STANDING REQUIREMENT. `(vendor, tier)` is an index into a
        // flat array and `(vendor, exchange, ISIN)` is one hash probe, so
        // neither may notice that the universe behind it grew 40x. A
        // per-request scan of a 34 MB master is what this refuses.
        let small = Join::build(&universe_of(1_000));
        let big = Join::build(&universe_of(40_000));
        let sizes = format!("{} vs {}", small.indexed(), big.indexed());
        assert!(
            big.indexed() > small.indexed() * 20,
            "the two universes must differ, or this measures nothing: {sizes}"
        );
        let probe = isin(RELIANCE);
        for (what, small_ps, big_ps) in [
            (
                "(vendor, tier) -> ids",
                cost_ps(|| small.tier(Vendor::Dhan, Tier::Nifty50).ids().len()),
                cost_ps(|| big.tier(Vendor::Dhan, Tier::Nifty50).ids().len()),
            ),
            (
                "(vendor, exchange, isin) -> id",
                cost_ps(|| {
                    small
                        .id(Vendor::Dhan, Exchange::Nse, probe)
                        .is_some()
                        .into()
                }),
                cost_ps(|| big.id(Vendor::Dhan, Exchange::Nse, probe).is_some().into()),
            ),
        ] {
            println!(
                "  {what}: {small_ps} ps at {} ISINs, {big_ps} ps at {} ISINs",
                small.indexed(),
                big.indexed()
            );
            assert!(
                big_ps * 1_000 <= small_ps * CEILING_PERMILLE,
                "{what} grew with the universe: {small_ps} ps -> {big_ps} ps"
            );
        }
    }
}
