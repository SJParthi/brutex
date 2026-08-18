//! Joining the exchange's published constituents against a vendor's master,
//! and saying exactly what agreed.
//!
//! # What this is for
//!
//! [`crate::nse`] turns the exchange's files into rows. This turns two row-sets
//! into a **verdict per instrument**, and that is the step that makes a
//! download a verification rather than a second copy.
//!
//! A count on its own says nothing. "748 of 750" is either two renames or two
//! missing instruments, and those need opposite actions: a rename is absorbed
//! and reported, a miss is a coverage gap worth telling the vendor about. So
//! nothing here returns a number without the bucket it came from.
//!
//! # The partition must sum, and that is the whole safety argument
//!
//! Every published name lands in exactly one bucket, and the buckets add up to
//! the published count. [`IndexResolution::is_sound`] asserts it. A partition
//! that does not sum is a defect in the join — and it is *findable* precisely
//! because it must sum, which is the property a set of independent counters
//! would not have.
//!
//! # No socket, again
//!
//! Like [`crate::nse`], everything here is a pure function over bytes somebody
//! else fetched. `CLAUDE.md` requires this crate to build and test against
//! fakes with no live vendor call, and the join is the part most worth proving
//! that way: every interesting case is a disagreement between two files, and a
//! disagreement is far easier to construct than to wait for.
//!
//! # Cost
//!
//! The vendor side is indexed once into a hash map — O(rows) — and every
//! exchange row is then one probe. So resolving an index of 750 names against a
//! master of 204,819 rows costs 204,819 + 750 operations rather than their
//! product. `docs/07-o1-architecture.md` law 1: the per-instrument cost does
//! not grow with the size of the master.

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::nse::Constituent;

/// Which field a feed's rows are joined on.
///
/// # Two keys, and one of them is weaker
///
/// D-0125 keyed the constituent join on NSE's own ISIN at both ends, and the
/// argument was that a symbol is a label the exchange may reuse while an ISIN
/// is the instrument's identity. That argument is unchanged and
/// [`Self::Isin`] is still the key wherever both sides carry one.
///
/// Zerodha's instrument master **has no ISIN column at all** — twelve columns,
/// none of them one (`docs/00-charter.md` §4z). Its own documentation names the
/// alternative: *"it is recommended to use a combination of exchange and
/// tradingsymbol as the unique key."* So that feed joins on
/// [`Self::TradingSymbol`], which is what the vendor instructs and is
/// **weaker** — and the resolution records which key it used rather than
/// presenting every feed as equally verified.
///
/// The difference is not academic. On a symbol key a rename looks like a
/// missing instrument, and two exchanges' listings of the same ticker look like
/// one instrument. Neither can happen on an ISIN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinKey {
    /// `(exchange, ISIN)` — the identity. Survives a rename.
    Isin,
    /// `(exchange, tradingsymbol)` — a label. Used only where the vendor
    /// publishes no ISIN, and marked as the weaker join wherever it is
    /// reported.
    TradingSymbol,
}

impl JoinKey {
    /// Whether a match on this key proves the two rows are the same instrument.
    ///
    /// `false` for [`Self::TradingSymbol`], and a caller rendering a count must
    /// say so. A page that shows "500 of 500 matched" identically for both keys
    /// is telling the reader something it does not know.
    #[must_use]
    pub const fn is_identity(self) -> bool {
        matches!(self, Self::Isin)
    }

    /// What the wire and a page call it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Isin => "isin",
            Self::TradingSymbol => "symbol",
        }
    }
}

impl fmt::Display for JoinKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// What happened to one published name on one feed.
///
/// **NO CATCH-ALL ANYWHERE THIS IS MATCHED.** A sixth outcome must be a compile
/// error at every reader, because a `_` arm would silently fold a new kind of
/// disagreement into whichever bucket happened to be listed last — and the
/// bucket is the whole of what a reader acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Verdict {
    /// The exchange lists it and this feed can name it. **The only bucket a
    /// pull can actually request.**
    Matched,
    /// The exchange lists it and this feed's master does not carry it at all.
    /// A real coverage gap, and the vendor's.
    Lacks,
    /// This feed lists the symbol but its row carries no ISIN, so identity
    /// cannot be established.
    ///
    /// **Not the same as [`Self::Lacks`], and reporting it as such blames a
    /// vendor for a cell the exchange never filled.** Index rows genuinely have
    /// no ISIN — NSE issues none for an index — so a set containing indices
    /// lands names here as a matter of course rather than as a fault.
    VendorHasNoIsin,
    /// Two vendor rows claim one key, so the name resolves two ways.
    ///
    /// **Refused rather than resolved.** Picking the first is decided by
    /// whatever order a map iterated, which is stable and stably wrong — a
    /// defect this repository has already shipped once in a response decoder,
    /// where `open` came from one object and `close` from another.
    Ambiguous,
}

impl Verdict {
    /// Every verdict, so a reader can build a full table without listing them.
    pub const ALL: [Self; 4] = [
        Self::Matched,
        Self::Lacks,
        Self::VendorHasNoIsin,
        Self::Ambiguous,
    ];

    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Matched => "matched",
            Self::Lacks => "lacks",
            Self::VendorHasNoIsin => "no-isin",
            Self::Ambiguous => "ambiguous",
        }
    }

    /// One sentence an operator can act on.
    #[must_use]
    pub const fn because(self) -> &'static str {
        match self {
            Self::Matched => "the exchange lists it and this feed can name it",
            Self::Lacks => "the exchange lists it and this feed's master does not carry it",
            Self::VendorHasNoIsin => {
                "this feed lists the symbol with no ISIN, so identity cannot be \
                 established — an index has none, and that is the exchange's \
                 doing rather than the vendor's"
            }
            Self::Ambiguous => {
                "two rows in this feed's master claim one key, so the name \
                 resolves two ways and is refused rather than picked"
            }
        }
    }
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.word())
    }
}

/// One published name and what became of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The exchange's symbol for it.
    pub symbol: String,
    /// The exchange's ISIN for it. Never empty — [`crate::nse::constituents`]
    /// refuses a row without one.
    pub isin: String,
    /// What happened.
    pub verdict: Verdict,
    /// The vendor's own id, when there is exactly one.
    ///
    /// `Some` only for [`Verdict::Matched`]: this is the value that reaches a
    /// request, and every other verdict is a state in which no single id
    /// exists. An `Ambiguous` name has two and neither may be chosen.
    pub vendor_id: Option<String>,
}

/// What one index resolved to against one feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexResolution {
    /// The index, as the exchange names it.
    pub index: String,
    /// The feed this was resolved against.
    pub feed: String,
    /// Which key the join used, and therefore how much a match proves.
    pub key: JoinKey,
    /// Every published name, in the exchange's own file order.
    ///
    /// File order rather than sorted: the exchange's order is a fact about the
    /// file and re-ordering it would make two snapshots of the same day differ
    /// for no reason. Idempotence is `CLAUDE.md` §3 rule 5.
    pub rows: Vec<Resolved>,
    /// Names this feed lists in the same exchange and segment that the index
    /// does not contain.
    ///
    /// Either the vendor is ahead of a rebalance, or it is not a member at all.
    /// Both are real and neither is an error, so this is a count beside the
    /// partition rather than a bucket inside it — **it is not part of the sum**,
    /// because it counts rows on the other side of the join.
    pub extra: usize,
}

impl IndexResolution {
    /// How many names the exchange published for this index.
    #[must_use]
    pub fn published(&self) -> usize {
        self.rows.len()
    }

    /// How many landed in `verdict`.
    ///
    /// A walk of the rows, which is bounded by the index's own size. There is
    /// no `N` here an input can grow beyond what the exchange published.
    #[must_use]
    pub fn count(&self, verdict: Verdict) -> usize {
        self.rows.iter().filter(|r| r.verdict == verdict).count()
    }

    /// **The partition sums.**
    ///
    /// Every published name is in exactly one bucket, so the four counts add up
    /// to [`Self::published`]. This is the property that makes a join defect
    /// findable: a set of independent counters can each be wrong quietly, and a
    /// partition cannot.
    ///
    /// It is a method rather than an assertion inside the join so a caller can
    /// check a resolution it did not build — one read back from a snapshot, for
    /// instance, where the arithmetic is the only thing that can be re-verified.
    #[must_use]
    pub fn is_sound(&self) -> bool {
        Verdict::ALL
            .into_iter()
            .map(|v| self.count(v))
            .sum::<usize>()
            == self.published()
    }

    /// The names a pull can actually request, in file order.
    ///
    /// The one number that predicts what comes back — and it is deliberately
    /// not the same as [`Self::published`], which is what the exchange lists.
    /// A form that shows the first tells an operator what NSE published; a form
    /// that shows this tells them what the run they are about to start can
    /// name.
    #[must_use]
    pub fn reachable(&self) -> Vec<&Resolved> {
        self.rows
            .iter()
            .filter(|r| r.verdict == Verdict::Matched)
            .collect()
    }
}

/// One vendor master row, reduced to what the join needs.
///
/// Taken as a distinct type rather than `core::vendor::MasterRow` so this
/// module does not need a vendor's whole row shape — and so a caller can join
/// against a master this crate has never seen the columns of, which is what
/// Zerodha's gzipped CSV will be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorInstrument<'a> {
    /// The vendor's own id — what reaches a request.
    pub vendor_id: &'a str,
    /// The vendor's tradable symbol.
    pub trading_symbol: &'a str,
    /// The vendor's ISIN for it, or empty when it publishes none.
    pub isin: &'a str,
}

/// Where a key resolved to more than one row.
///
/// Held as its own state rather than as an `Option<&Row>` that is `None` twice
/// over: "absent" and "ambiguous" are different facts and the whole point of
/// the [`Verdict::Ambiguous`] bucket is that they must not collapse.
#[derive(Debug, Clone, Copy)]
enum Slot<'a> {
    One(VendorInstrument<'a>),
    Many,
}

/// Every non-empty trading symbol the master lists, in one pre-sized set.
///
/// # Why this is not a `.filter().collect()`
///
/// It was one, and `collect` reserved **nothing**. A `HashSet` built by
/// `FromIterator` reserves the iterator's size-hint *lower* bound, and
/// `Filter` cannot know how many rows will survive its predicate, so that
/// bound is `0` however long the master is. The table therefore starts empty
/// and rehashes its way up — every key inserted so far re-hashed and re-placed
/// at each doubling, ~18 of them on the 204,819-row master this module is
/// sized for, all of it invisible because the answer comes out correct.
///
/// `docs/07-o1-architecture.md` layer 2 is the rule: reserve from a bound that
/// is already known. `master.len()` is known before the walk and is an
/// over-estimate only by the blank symbols, which is the right direction.
///
/// # The bound is the row count, not the row count doubled
///
/// D-0040 gave the manifest's index headroom because entries are appended to it
/// *after* the load, and a reservation of exactly `n_valid` rehashed on the
/// first one. Nothing is ever inserted into this set after it is returned — it
/// is built once and only probed — so exact is exact, and the headroom would
/// buy nothing but memory.
///
/// # What this does not change
///
/// The probe is what it always was, and the set is what it always was: the same
/// symbols, the same absence of the blank one. This is a reservation, not a
/// behaviour, and `the_symbol_index_is_reserved_from_the_master_not_the_survivors`
/// asserts both halves so a future edit cannot buy the capacity by dropping the
/// filter.
fn symbol_index<'a>(master: &[VendorInstrument<'a>]) -> HashSet<&'a str> {
    let mut symbols: HashSet<&'a str> = HashSet::with_capacity(master.len());
    // A BLANK SYMBOL IS AN ABSENCE, NOT A NAME — the same rule the key index
    // above applies to a blank ISIN. Indexing it would make every row that
    // carries no symbol answer "yes, this feed lists it" for a published name
    // that is itself blank, which is the confusion `VendorHasNoIsin` exists to
    // keep apart from a real listing.
    symbols.extend(
        master
            .iter()
            .map(|row| row.trading_symbol)
            .filter(|symbol| !symbol.is_empty()),
    );
    symbols
}

/// Resolve one index's published names against one feed's master.
///
/// # The key is chosen by the FEED, not by the caller
///
/// `key` is [`JoinKey::Isin`] for a vendor that publishes ISINs and
/// [`JoinKey::TradingSymbol`] for one that does not. It travels into the
/// resolution so every reader downstream can see how much a match proves —
/// which is the honest alternative to silently joining on whatever field
/// happened to be populated.
///
/// # Cost
///
/// One pass to index the master, one probe per published name. Resolving 750
/// names against 204,819 rows is 205,569 operations, not 153 million.
#[must_use]
pub fn resolve(
    index: &str,
    feed: &str,
    key: JoinKey,
    published: &[Constituent<'_>],
    master: &[VendorInstrument<'_>],
) -> IndexResolution {
    // THE MASTER IS INDEXED ONCE, AND A REPEATED KEY BECOMES `Many` RATHER THAN
    // OVERWRITING. `insert` returning the previous value is what makes a
    // duplicate visible at all — without this the second row would silently win
    // and the name would resolve to whichever the master happened to list last.
    let mut by_key: HashMap<&str, Slot<'_>> = HashMap::with_capacity(master.len());
    for row in master {
        let field = match key {
            JoinKey::Isin => row.isin,
            JoinKey::TradingSymbol => row.trading_symbol,
        };
        // An empty key joins nothing. Indexing it would make every row with a
        // blank ISIN collide with every other, which reads as ambiguity where
        // the truth is absence — the exact confusion `VendorHasNoIsin` exists
        // to prevent.
        if field.is_empty() {
            continue;
        }
        by_key
            .entry(field)
            .and_modify(|held| *held = Slot::Many)
            .or_insert(Slot::One(*row));
    }

    // Only needed for the ISIN key: it answers "does this feed list the symbol
    // at all", which is what separates a vendor that has never heard of an
    // instrument from one that lists it without an identity. Reserved from
    // `master.len()` rather than collected — see `symbol_index`.
    let by_symbol: HashSet<&str> = symbol_index(master);

    let mut rows = Vec::with_capacity(published.len());
    for row in published {
        let probe = match key {
            JoinKey::Isin => row.isin,
            JoinKey::TradingSymbol => row.symbol,
        };
        let (verdict, vendor_id) = match by_key.get(probe) {
            Some(Slot::One(found)) => (Verdict::Matched, Some(found.vendor_id.to_owned())),
            Some(Slot::Many) => (Verdict::Ambiguous, None),
            // NOT FOUND ON THE KEY — and the reason splits in two.
            //
            // On an ISIN join, a feed that lists the SYMBOL but carries no ISIN
            // for it has not failed to hold the instrument; it has failed to
            // publish its identity, and NSE issues no ISIN for an index at all.
            // Calling that `Lacks` blames the vendor for a cell the exchange
            // never filled. On a symbol join there is no second field to fall
            // back to, so absence is absence.
            None => match key {
                JoinKey::Isin if by_symbol.contains(row.symbol) => (Verdict::VendorHasNoIsin, None),
                JoinKey::Isin | JoinKey::TradingSymbol => (Verdict::Lacks, None),
            },
        };
        rows.push(Resolved {
            symbol: row.symbol.to_owned(),
            isin: row.isin.to_owned(),
            verdict,
            vendor_id,
        });
    }

    // ROWS THE FEED HOLDS THAT THIS INDEX DOES NOT NAME. Counted against the
    // published SYMBOL set whichever key was joined on, because "is this name in
    // the index" is a question about the index's membership and the index names
    // its members by symbol.
    let named: HashSet<&str> = published.iter().map(|c| c.symbol).collect();
    let extra = master
        .iter()
        .filter(|row| !row.trading_symbol.is_empty())
        .filter(|row| !named.contains(row.trading_symbol))
        .count();

    IndexResolution {
        index: index.to_owned(),
        feed: feed.to_owned(),
        key,
        rows,
        extra,
    }
}

/// How far a resolution may shrink before it is refused rather than published,
/// as a fraction — **one quarter**.
///
/// # Why a bound exists at all
///
/// A rebalance moves a handful of names. A fetch that went wrong moves all of
/// them — and the dangerous direction is **shrinking**, because a universe that
/// silently loses 400 names produces a backfill that covers the remaining 350
/// and reports success. `CLAUDE.md` §4: degrade loudly and name the reason.
///
/// The figure is a **policy**, not a measurement, and it says so. No published
/// rebalance rule has been read, so nothing here derives it — it is a number
/// chosen to sit far above any plausible rebalance and far below a broken
/// fetch, and changing it is a deliberate act.
///
/// # A rational, not a float
///
/// `CLAUDE.md` §7 and prohibition 5 keep floating point out of this repository,
/// and the reason generalises past prices: a ratio compared as `f64` is a
/// comparison whose boundary depends on rounding, and this one decides whether
/// a universe is published. Held as a numerator and a denominator, the
/// comparison below is exact integer arithmetic and the boundary is wherever
/// the arithmetic says it is.
pub const MAX_SHRINK_NUMERATOR: usize = 1;
/// The denominator of [`MAX_SHRINK_NUMERATOR`].
pub const MAX_SHRINK_DENOMINATOR: usize = 4;

/// Whether a new resolution may replace the last one, or must halt and say why.
///
/// `Ok(())` when the shrink is within [`MAX_SHRINK_NUMERATOR`] over
/// [`MAX_SHRINK_DENOMINATOR`]. **Growth is never refused**: an index gaining
/// members is ordinary, and a bound on it would refuse the exchange for doing
/// its job.
///
/// The comparison is `lost * DENOM > previous * NUM`, which is
/// `lost / previous > NUM / DENOM` with no division and therefore no rounding.
/// `saturating_mul` rather than `*`: both operands are index sizes, so the
/// product cannot realistically overflow, and a bound that panics on an
/// implausible input is still a panic this workspace denies.
///
/// # Errors
///
/// A sentence naming both counts, how many were lost, and the bound — so an
/// operator can decide whether they are looking at a rebalance or a broken
/// fetch.
pub fn admits_replacement(previous: usize, next: usize) -> Result<(), String> {
    if previous == 0 || next >= previous {
        return Ok(());
    }
    let lost = previous.saturating_sub(next);
    if lost.saturating_mul(MAX_SHRINK_DENOMINATOR) > previous.saturating_mul(MAX_SHRINK_NUMERATOR) {
        return Err(format!(
            "this resolution holds {next} name(s) where the last held {previous} — \
             {lost} fewer, and the bound is {MAX_SHRINK_NUMERATOR} in \
             {MAX_SHRINK_DENOMINATOR}. Nothing was published: a universe that \
             silently loses names produces a backfill that covers the remainder \
             and reports success. If the exchange really did this, the bound is \
             the thing to change, deliberately."
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    fn published() -> Vec<Constituent<'static>> {
        vec![
            Constituent {
                company: "A Ltd.",
                industry: "Fin",
                symbol: "AAA",
                series: "EQ",
                isin: "INE000A01001",
            },
            Constituent {
                company: "B Ltd.",
                industry: "Fin",
                symbol: "BBB",
                series: "EQ",
                isin: "INE000B01002",
            },
            Constituent {
                company: "C Ltd.",
                industry: "IT",
                symbol: "CCC",
                series: "EQ",
                isin: "INE000C01003",
            },
        ]
    }

    fn vendor(
        rows: &[(&'static str, &'static str, &'static str)],
    ) -> Vec<VendorInstrument<'static>> {
        rows.iter()
            .map(|(id, sym, isin)| VendorInstrument {
                vendor_id: id,
                trading_symbol: sym,
                isin,
            })
            .collect()
    }

    #[test]
    fn a_clean_join_matches_every_name_and_the_partition_sums() {
        let master = vendor(&[
            ("1", "AAA", "INE000A01001"),
            ("2", "BBB", "INE000B01002"),
            ("3", "CCC", "INE000C01003"),
        ]);
        let out = resolve("Nifty Test", "groww", JoinKey::Isin, &published(), &master);
        assert_eq!(out.published(), 3);
        assert_eq!(out.count(Verdict::Matched), 3);
        assert!(out.is_sound(), "the partition must sum");
        assert_eq!(out.reachable().len(), 3);
        assert_eq!(out.rows[0].vendor_id.as_deref(), Some("1"));
        assert_eq!(out.extra, 0);
    }

    /// **The case that is the entire argument for the ISIN key.**
    #[test]
    fn a_renamed_symbol_still_matches_because_the_isin_did_not_change() {
        let master = vendor(&[
            ("1", "AAA-RENAMED", "INE000A01001"),
            ("2", "BBB", "INE000B01002"),
            ("3", "CCC", "INE000C01003"),
        ]);
        let out = resolve("Nifty Test", "groww", JoinKey::Isin, &published(), &master);
        assert_eq!(
            out.count(Verdict::Matched),
            3,
            "a rename must be absorbed, not read as a missing instrument"
        );
        // And the same data on a SYMBOL key loses it — which is what "weaker"
        // means, stated as a test rather than as a claim.
        let weak = resolve(
            "Nifty Test",
            "kite",
            JoinKey::TradingSymbol,
            &published(),
            &master,
        );
        assert_eq!(weak.count(Verdict::Matched), 2);
        assert_eq!(weak.count(Verdict::Lacks), 1);
        assert!(weak.is_sound());
        assert!(!weak.key.is_identity(), "a symbol match proves less");
    }

    #[test]
    fn a_symbol_the_vendor_lists_without_an_isin_is_not_blamed_on_the_vendor() {
        let master = vendor(&[
            ("1", "AAA", "INE000A01001"),
            ("2", "BBB", ""),
            ("3", "CCC", "INE000C01003"),
        ]);
        let out = resolve("Nifty Test", "groww", JoinKey::Isin, &published(), &master);
        assert_eq!(out.count(Verdict::Matched), 2);
        assert_eq!(
            out.count(Verdict::VendorHasNoIsin),
            1,
            "the feed lists BBB — it published no identity for it, which is a \
             different fact from not holding it"
        );
        assert_eq!(out.count(Verdict::Lacks), 0);
        assert!(out.is_sound());
        assert!(Verdict::VendorHasNoIsin.because().contains("exchange"));
    }

    /// **A set collected through a filter reserves for the survivors, and the
    /// master is what goes in.**
    ///
    /// `HashSet`'s `FromIterator` reserves the size-hint *lower* bound, and
    /// `Filter`'s lower bound is `0` whatever it is filtering — so
    /// `master.iter().map(..).filter(..).collect()` sized this table for the
    /// symbols that came out and rehashed every key already placed at each
    /// doubling on the way there. Correct, and O(rows) with a growing constant
    /// nobody could see, because the set it produced was identical.
    ///
    /// The two halves are asserted together on purpose. Capacity alone would
    /// pass if a future edit bought it by dropping the `is_empty` filter, and
    /// the membership alone is what the old code already satisfied.
    ///
    /// The row count is deliberately far above the survivor count: at three
    /// survivors a `collect` reserves for three, so `capacity() >= 64` is false
    /// under the defect and true under the fix, with no timing in it.
    #[test]
    fn the_symbol_index_is_reserved_from_the_master_not_the_survivors() {
        let mut master = vendor(&[
            ("1", "AAA", "INE000A01001"),
            ("2", "BBB", "INE000B01002"),
            ("3", "CCC", "INE000C01003"),
        ]);
        // Rows a vendor master really does carry: listed, and named by nothing
        // this join can use.
        for _ in 0..61 {
            master.push(VendorInstrument {
                vendor_id: "x",
                trading_symbol: "",
                isin: "",
            });
        }

        let index = symbol_index(&master);
        assert_eq!(
            index.len(),
            3,
            "a blank symbol is an absence and is not indexed as a name"
        );
        for symbol in ["AAA", "BBB", "CCC"] {
            assert!(index.contains(symbol), "{symbol} is listed by the master");
        }
        assert!(
            !index.contains(""),
            "the blank must not become a name every symbol-less row answers to"
        );
        assert!(
            index.capacity() >= master.len(),
            "the table must be reserved from the {} rows that go in, not from \
             the {} symbols that come out — it holds {}",
            master.len(),
            index.len(),
            index.capacity()
        );

        // AND THE JOIN IS UNCHANGED BY IT. The reservation is a cost, not a
        // behaviour: the same master through `resolve` must still separate a
        // symbol the feed lists without an ISIN from one it does not list.
        let listed_without_isin = vendor(&[
            ("1", "AAA", "INE000A01001"),
            ("2", "BBB", ""),
            ("3", "CCC", "INE000C01003"),
        ]);
        let out = resolve(
            "Nifty Test",
            "groww",
            JoinKey::Isin,
            &published(),
            &listed_without_isin,
        );
        assert_eq!(out.count(Verdict::VendorHasNoIsin), 1);
        assert!(out.is_sound());
    }

    #[test]
    fn a_name_the_vendor_has_never_heard_of_is_a_real_coverage_gap() {
        let master = vendor(&[("1", "AAA", "INE000A01001"), ("3", "CCC", "INE000C01003")]);
        let out = resolve("Nifty Test", "groww", JoinKey::Isin, &published(), &master);
        assert_eq!(out.count(Verdict::Lacks), 1);
        assert_eq!(out.count(Verdict::VendorHasNoIsin), 0);
        assert!(out.is_sound());
    }

    /// Two rows, one key: refused, never resolved to whichever came first.
    #[test]
    fn two_vendor_rows_claiming_one_isin_refuse_rather_than_letting_one_win() {
        let master = vendor(&[
            ("1", "AAA", "INE000A01001"),
            ("999", "AAA-DUP", "INE000A01001"),
            ("2", "BBB", "INE000B01002"),
            ("3", "CCC", "INE000C01003"),
        ]);
        let out = resolve("Nifty Test", "groww", JoinKey::Isin, &published(), &master);
        assert_eq!(out.count(Verdict::Ambiguous), 1);
        assert_eq!(out.count(Verdict::Matched), 2);
        assert!(out.is_sound());
        let row = out.rows.iter().find(|r| r.symbol == "AAA").expect("AAA");
        assert_eq!(
            row.vendor_id, None,
            "an ambiguous name has two ids and neither may be chosen"
        );
        // Order-independence: reversing the master must not change the verdict.
        let mut reversed = master.clone();
        reversed.reverse();
        let again = resolve(
            "Nifty Test",
            "groww",
            JoinKey::Isin,
            &published(),
            &reversed,
        );
        assert_eq!(again.count(Verdict::Ambiguous), 1);
        assert_eq!(
            again.rows, out.rows,
            "the verdict cannot depend on which row the master listed first"
        );
    }

    #[test]
    fn rows_the_feed_holds_that_the_index_does_not_name_are_counted_beside_the_sum() {
        let master = vendor(&[
            ("1", "AAA", "INE000A01001"),
            ("2", "BBB", "INE000B01002"),
            ("3", "CCC", "INE000C01003"),
            ("9", "ZZZ", "INE000Z01009"),
        ]);
        let out = resolve("Nifty Test", "groww", JoinKey::Isin, &published(), &master);
        assert_eq!(out.extra, 1);
        assert_eq!(
            out.published(),
            3,
            "extra counts rows on the OTHER side and is not part of the partition"
        );
        assert!(out.is_sound());
    }

    #[test]
    fn an_empty_isin_on_the_vendor_side_never_collides_with_another_empty_one() {
        // Three blank ISINs would all key on "" and read as ambiguous if the
        // empty key were indexed.
        let master = vendor(&[("1", "AAA", ""), ("2", "BBB", ""), ("3", "CCC", "")]);
        let out = resolve("Nifty Test", "groww", JoinKey::Isin, &published(), &master);
        assert_eq!(out.count(Verdict::Ambiguous), 0, "absence is not ambiguity");
        assert_eq!(out.count(Verdict::VendorHasNoIsin), 3);
        assert!(out.is_sound());
    }

    #[test]
    fn an_index_with_no_members_resolves_to_nothing_and_still_sums() {
        let out = resolve("Empty", "groww", JoinKey::Isin, &[], &vendor(&[]));
        assert_eq!(out.published(), 0);
        assert!(out.is_sound());
        assert!(out.reachable().is_empty());
    }

    #[test]
    fn every_verdict_has_a_distinct_word_and_a_sentence_worth_reading() {
        let mut words = Vec::new();
        for verdict in Verdict::ALL {
            let word = verdict.word();
            assert!(!words.contains(&word), "{word} is claimed twice");
            words.push(word);
            assert_eq!(verdict.to_string(), word);
            assert!(
                verdict.because().len() > 40,
                "{verdict} explains too little"
            );
        }
        assert_eq!(words.len(), 4);
    }

    #[test]
    fn only_the_isin_key_claims_to_prove_identity() {
        assert!(JoinKey::Isin.is_identity());
        assert!(!JoinKey::TradingSymbol.is_identity());
        assert_eq!(JoinKey::Isin.to_string(), "isin");
        assert_eq!(JoinKey::TradingSymbol.to_string(), "symbol");
    }

    // -- the replacement bound ----------------------------------------------

    #[test]
    fn growth_and_an_ordinary_rebalance_are_admitted() {
        assert!(admits_replacement(50, 51).is_ok(), "an index may grow");
        assert!(admits_replacement(50, 50).is_ok());
        assert!(admits_replacement(750, 748).is_ok(), "two names move");
        assert!(
            admits_replacement(0, 0).is_ok(),
            "there is no previous resolution to compare against"
        );
    }

    #[test]
    fn a_collapse_halts_and_names_both_counts() {
        let why = admits_replacement(750, 350).expect_err("400 names is not a rebalance");
        assert!(why.contains("750"), "{why}");
        assert!(why.contains("350"), "{why}");
        assert!(why.contains("400"), "{why}");
        assert!(
            why.contains("reports success"),
            "the refusal has to say what the silent version would have done: {why}"
        );
    }

    #[test]
    fn the_bound_bites_exactly_where_it_says_it_does() {
        // 25% lost is inside; anything past it is not.
        assert!(admits_replacement(100, 75).is_ok(), "exactly at the bound");
        assert!(admits_replacement(100, 74).is_err(), "one past it");
    }
}
