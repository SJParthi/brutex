//! What ONE FEED can actually name in each spot target, and every name it
//! cannot.
//!
//! # The number on the button was never the number the pull would fetch
//!
//! [`crate::server::Site::targets`] counts merged rows — every instrument in
//! the universe that a target names, whichever vendor listed it. That is a fact
//! about NSE and it is the wrong fact for a form: the operator picks a feed
//! first, and a feed can only fetch what its own master gives it an id for. On
//! the masters read on 2026-08-12 the two answers differ by more than half:
//! `indices` counts **35**, of which Groww lists 24 and Dhan 15. A form that
//! renders 35 beside a Groww run promises eleven instruments that will be
//! refused one at a time, by name, after the run has started — which is the
//! shape of the operator's own months-old question, *"why for groww nifty 50
//! only 47 instruments are shown"*. The count and the run were answering
//! different questions and only one of them was on screen.
//!
//! This module holds the other one, per `(vendor, target)`, computed once.
//!
//! # Two ways to count, and which is used is on the wire
//!
//! * **`join`** — the five targets a published constituent list DEFINES.
//!   [`crate::constituents::Join`] already resolved every published name to
//!   that vendor's id through NSE's OWN ISIN (D-0125), so [`Covered::matched`] is
//!   the LENGTH OF THE ID LIST a pull would name — `Join::ids_for` — and the
//!   other four buckets are the reason the rest are missing. Nothing is
//!   recounted here, and the number on the control is the array the request is
//!   built from rather than a tally kept beside it.
//! * **`master`** — [`SpotTarget::Swept`] and [`SpotTarget::Indices`]. Neither
//!   is a published list: the first is the engine surface (`CLAUDE.md` §1, two
//!   `(exchange, symbol)` pairs) and the second is whatever a vendor's master
//!   calls an index series, which NSE publishes no file for. There is no
//!   denominator to be a fraction of, so [`Covered::published`] is `None` and
//!   the count is a fold over the merged universe with the feed's own id
//!   column as the test.
//!
//! [`Covered::counted_from`] says which, on every row, so a reader never has to
//! infer it from a `null`.
//!
//! # A feed with no master gets `None`, not a zero
//!
//! `TrueData` and `GDFL` are archive vendors: a folder of CSVs is its own
//! listing and neither publishes an instrument master
//! ([`Vendor::publishes_master`]). Counting them against the master would report
//! `0 matched, 35 lacks` — a measurement of a file they do not have, which
//! reads as "this feed has nothing" and is `CLAUDE.md` §4's fallback that hides
//! a failure. [`Coverage::of`] answers `None` for them and the wire says
//! `"counted_from":"no master"` with null counts.
//!
//! # Cost
//!
//! Built once, in [`crate::server::Site::new`], beside the join it reads —
//! `docs/05-decisions.md` D-0039 and D-0042 put every whole-universe pass at
//! startup. The five join-backed targets are four `Vec::len()` calls each; the
//! two master-counted ones are one fold over the merged universe apiece, per
//! mastered vendor. Afterwards [`Coverage::of`] is a single index into a flat
//! `Vendor::ALL.len() × SpotTarget::ALL.len()` array — no scan, no probe, no
//! master read. `api::coverage::the_lookup_does_not_grow_with_the_universe` is
//! the measurement.

use std::fmt::Write as _;

use brutex_core::vendor::{Vendor, VendorId};

use crate::constituents::{Join, TierJoin};
use crate::ingest::SpotTarget;
use crate::merge::Merged;
use crate::render;

/// Which bucket a name that did not become an id fell into.
///
/// The four are not interchangeable and collapsing them to "missing" is how an
/// operator is told a number and not a cause: `Lacks` is the vendor's problem,
/// `Ambiguous` is this build refusing to guess between two of the vendor's own
/// rows, `NoNseIsin` is the exchange's own file naming no ISIN for the name,
/// and `Malformed` is a published name this build cannot make a key of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bucket {
    /// This feed's master carries no row for the name's NSE ISIN.
    Lacks,
    /// Two or more of this feed's rows claim the name's NSE ISIN, so no single
    /// id can be named. See [`crate::constituents::Join::id`].
    Ambiguous,
    /// The published name cannot be a key at all — not a legal symbol, or an
    /// NSE placeholder scrip. Nothing to join on, for any feed.
    Malformed,
    /// **The bucket D-0125 added.** The name is fine and the exchange's own
    /// constituent file names no ISIN beside it, so there is nothing to join
    /// on and it is nobody's master's fault.
    ///
    /// Its own word on the wire because a page that drew it as `malformed`
    /// would tell an operator to go and fix a transcription that is correct.
    /// Six names on the lists this build carries: five F&O index underlyings,
    /// which are not shares, and `AGL`, whose row in the exchange's file has an
    /// empty ISIN cell (`docs/06-limits.md` §11).
    NoNseIsin,
}

impl Bucket {
    /// The word this bucket travels as.
    #[must_use]
    pub const fn wire(self) -> &'static str {
        match self {
            Self::Lacks => "lacks",
            Self::Ambiguous => "ambiguous",
            Self::Malformed => "malformed",
            Self::NoNseIsin => "no_nse_isin",
        }
    }
}

/// One published name this feed could not be asked for, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    /// The name, as the exchange publishes it — or, for a master-counted
    /// target, as the merged universe keys it.
    pub symbol: String,
    /// Which of the three it is.
    pub bucket: Bucket,
    /// The sentence an operator reads instead of a number.
    pub why: String,
}

/// What one feed reaches in one target, and the whole of what it does not.
///
/// `matched + lacks + ambiguous + malformed + no_nse_isin == published` for
/// every join-backed target — that partition is [`crate::constituents`]'s own
/// invariant and this type carries it forward rather than re-deriving it. For a master-counted
/// target `published` is `None` and the sum is `matched + lacks`, which is the
/// size of the set the merged universe names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Covered {
    /// Instruments this feed can be asked for by id. **The number that belongs
    /// on the control.**
    pub matched: usize,
    /// Names whose identity is known and which this feed's master does not
    /// carry.
    pub lacks: usize,
    /// Names more than one of this feed's rows claims.
    pub ambiguous: usize,
    /// Names this build cannot make a key of at all — not a legal symbol, or
    /// an NSE placeholder scrip.
    pub malformed: usize,
    /// Names the EXCHANGE'S OWN file names no ISIN for.
    ///
    /// The bucket D-0125 added. Identical for every feed, because it is a fact
    /// about NSE's constituent file and about no master: five F&O index
    /// underlyings and `AGL`. See `docs/06-limits.md` §11.
    pub no_nse_isin: usize,
    /// What the source states the set's size is, when a source states one.
    pub published: Option<usize>,
    /// Every name behind the four non-matched counts, with its reason.
    ///
    /// Bounded by the largest published list this build carries (750), and
    /// empty whenever the feed reaches everything. Emitted whole and never
    /// truncated: the operator asked for 500 and got 486, and the fourteen are
    /// the answer.
    pub unresolved: Vec<Unresolved>,
}

impl Covered {
    /// How the counts above were arrived at: `join` or `master`.
    #[must_use]
    pub const fn counted_from(&self) -> &'static str {
        if self.published.is_some() {
            "join"
        } else {
            "master"
        }
    }

    /// Every name accounted for, matched or not.
    #[must_use]
    pub const fn accounted(&self) -> usize {
        self.matched
            .saturating_add(self.lacks)
            .saturating_add(self.ambiguous)
            .saturating_add(self.malformed)
            .saturating_add(self.no_nse_isin)
    }
}

/// The one answer for every `(vendor, target)`, decided at load.
#[derive(Debug)]
pub struct Coverage {
    /// `Vendor::ALL.len() * SpotTarget::ALL.len()` slots, indexed by
    /// `vendor as usize * SpotTarget::ALL.len() + slot`. `None` is a vendor
    /// that publishes no master — see the module header.
    per: Vec<Option<Covered>>,
}

impl Coverage {
    /// Builds every `(vendor, target)` answer once.
    ///
    /// # Cost
    ///
    /// Four `len()` calls per join-backed target, and ONE fold over the merged
    /// universe per `(mastered vendor, master-counted target)` — four folds,
    /// because two vendors publish a master and two targets are defined by no
    /// published list. That is 4 × 2,795 row-visits at startup on the masters
    /// read on 2026-08-12, and it is not on a request path: `Read::new` calls
    /// this where `Catalog::build` runs, once per process (D-0039/D-0042).
    ///
    /// The folds are written as four rather than as one shared pass carrying
    /// four accumulators. The row-visits are the same either way, and the
    /// shared version needs a fallible index into the accumulator array on
    /// every row — a branch that cannot be taken, which is a branch no test can
    /// cover and a place a wrong index would land silently.
    #[must_use]
    pub fn build(merged: &Merged, join: &Join) -> Self {
        let mut per = Vec::with_capacity(Vendor::ALL.len() * SpotTarget::ALL.len());
        for vendor in Vendor::ALL {
            for target in SpotTarget::ALL {
                if !vendor.publishes_master() {
                    // NOT A ZERO. See the module header: an archive vendor has
                    // no master, so the master has nothing to say about it and
                    // says nothing rather than saying "none".
                    per.push(None);
                    continue;
                }
                // `if let ... else` rather than a match, because one of the
                // two arms is a compound expression clippy would rather see
                // named. The two shapes are the module's whole subject: a
                // target a published list defines, and a target only the
                // master can count.
                let answer = if let Some(tier) = target.tier() {
                    // THE COUNT AND THE REQUEST COME FROM ONE ARRAY.
                    //
                    // `matched` is the length of the id list `Join::ids_for`
                    // hands a pull, not a second tally of the matched rows
                    // beside it. The two are equal by construction — the join
                    // appends to both in the same arm — and taking the count
                    // from the ids is what keeps them equal when only one of
                    // them is edited. `every_matched_row_contributes_exactly_one_id`
                    // is the assertion; this line is the reason it can never
                    // matter here.
                    from_join(
                        join.tier(vendor, tier),
                        join.ids_for(vendor, target).unwrap_or_default(),
                        tier.published(),
                    )
                } else {
                    from_master(merged, vendor, target)
                };
                per.push(Some(answer));
            }
        }
        Self { per }
    }

    /// What one feed reaches in one target. **One array index.**
    ///
    /// `None` is a vendor that publishes no instrument master, which is a
    /// different fact from "reaches nothing" and must not be rendered as one.
    #[must_use]
    pub fn of(&self, vendor: Vendor, target: SpotTarget) -> Option<&Covered> {
        self.per
            .get(vendor as usize * SpotTarget::ALL.len() + target_slot(target))
            .and_then(Option::as_ref)
    }

    /// One line per `(vendor, target)` whose feed-aware count is SHORT of the
    /// set the universe names.
    ///
    /// Only the short ones, and that is the point: a note per target would be
    /// fourteen lines an operator scrolls past, and the whole content of this
    /// module is the rows where the two numbers disagree. A build where nothing
    /// is short produces no lines, which is the correct amount to say about it.
    #[must_use]
    pub fn notes(&self) -> Vec<String> {
        let mut out = Vec::new();
        // EVERY VENDOR, INCLUDING THE ARCHIVES, and they are skipped BY THE
        // ANSWER rather than by the loop bound. `Coverage::of` returns `None`
        // for a feed that publishes no master, and walking `Vendor::MASTERED`
        // here would encode that same fact a second time — so a fifth vendor
        // would have to be added to two lists to be treated correctly, and the
        // one it was left out of would be silent.
        for vendor in Vendor::ALL {
            for target in SpotTarget::ALL {
                let Some(covered) = self.of(vendor, target) else {
                    continue;
                };
                let short = covered.accounted().saturating_sub(covered.matched);
                if short == 0 {
                    continue;
                }
                let mut line = format!(
                    "{} · {}: {} of {} reachable — {} lacks, {} ambiguous, {} malformed, \
                     {} no NSE ISIN. \
                     A run of target={} STILL ATTEMPTS ALL {} and refuses these one at a \
                     time by name, because `server::broker_run` filters by the universe \
                     and not by the feed — docs/06-limits.md §63",
                    vendor.as_str(),
                    target.label(),
                    covered.matched,
                    covered.accounted(),
                    covered.lacks,
                    covered.ambiguous,
                    covered.malformed,
                    covered.no_nse_isin,
                    target.slug(),
                    covered.accounted(),
                );
                let names: Vec<&str> = covered
                    .unresolved
                    .iter()
                    .map(|u| u.symbol.as_str())
                    .collect();
                let _ = write!(line, ": {}", names.join(", "));
                out.push(line);
            }
        }
        out
    }

    /// The whole answer for one feed, as the JSON `/universes.json` returns.
    ///
    /// Hand-written for the reason every other document in this crate is: a
    /// serialiser would be a dependency for eleven fields whose shapes are all
    /// known here. Every string goes through [`render::json_string`].
    #[must_use]
    pub fn json(&self, vendor: Vendor) -> String {
        let mut out = format!(
            r#"{{"feed":{},"mastered":{},"targets":["#,
            render::json_string(vendor.as_str()),
            vendor.publishes_master(),
        );
        for (n, target) in SpotTarget::ALL.into_iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            out.push_str(&target_json(self.of(vendor, target), target));
        }
        out.push_str("]}");
        out
    }
}

/// Where one target sits in [`SpotTarget::ALL`].
///
/// A search over a compile-time array of seven rather than `target as usize`,
/// because the enum's discriminant and its position in `ALL` are two different
/// things that happen to agree, and the day somebody appends a variant in the
/// middle of the enum they stop agreeing silently.
/// `api::coverage::every_target_indexes_its_own_slot` pins the pair.
fn target_slot(target: SpotTarget) -> usize {
    SpotTarget::ALL
        .into_iter()
        .position(|t| t == target)
        .unwrap_or(0)
}

/// One join-backed target's answer, read off the buckets the join already
/// sorted.
fn from_join(join: &TierJoin, ids: &[VendorId], published: usize) -> Covered {
    let mut unresolved = Vec::new();
    for row in &join.lacks {
        unresolved.push(Unresolved {
            symbol: row.symbol.to_owned(),
            bucket: Bucket::Lacks,
            why: format!(
                "this feed's master carries no row for ISIN {}",
                row.isin.as_str()
            ),
        });
    }
    for row in &join.ambiguous {
        let ids: Vec<&str> = row.ids.iter().map(VendorId::as_str).collect();
        unresolved.push(Unresolved {
            symbol: row.symbol.to_owned(),
            bucket: Bucket::Ambiguous,
            why: format!(
                "{} of this feed's rows carry ISIN {} ({}) — no id is chosen, because \
                 filing a request under the wrong one is indistinguishable from success \
                 until the bars are wrong",
                row.ids.len(),
                row.isin.as_str(),
                ids.join(" ")
            ),
        });
    }
    for row in &join.malformed {
        unresolved.push(Unresolved {
            symbol: row.symbol.to_owned(),
            bucket: Bucket::Malformed,
            why: row.why.reason().to_owned(),
        });
    }
    // THE EXCHANGE'S OWN GAP, AS ITS OWN ROWS. Same shape as the malformed
    // ones and a different word, because the two blame different files and an
    // operator acts on them differently: one is a transcription to fix here,
    // the other is a cell NSE left empty and nothing in this repository may
    // fill.
    for row in &join.no_nse_isin {
        unresolved.push(Unresolved {
            symbol: row.symbol.to_owned(),
            bucket: Bucket::NoNseIsin,
            why: row.why.reason().to_owned(),
        });
    }
    Covered {
        matched: ids.len(),
        lacks: join.lacks.len(),
        ambiguous: join.ambiguous.len(),
        malformed: join.malformed.len(),
        no_nse_isin: join.no_nse_isin.len(),
        published: Some(published),
        unresolved,
    }
}

/// One master-counted target's answer, folded off the merged universe.
///
/// The two targets no published list defines. There is nothing to join and no
/// denominator to be a fraction of, so the whole answer is "which rows does
/// this target name, and does this vendor's master give each of them an id".
fn from_master(merged: &Merged, vendor: Vendor, target: SpotTarget) -> Covered {
    let mut matched = 0usize;
    let mut missing: Vec<String> = Vec::new();
    for (key, entry) in &merged.by_key {
        if !target.names(key, entry.universe) {
            continue;
        }
        // ONE ARRAY INDEX PER ROW. `ids` is indexed by the vendor's own
        // discriminant, which is what makes "does this feed list it" a constant
        // rather than a lookup.
        if entry.ids.get(vendor as usize).copied().flatten().is_some() {
            matched = matched.saturating_add(1);
        } else {
            missing.push(key.underlying.to_string());
        }
    }
    // SORTED, because a `HashMap` walk is not ordered and a reason list that
    // reshuffles between restarts is a list an operator cannot diff.
    // `CLAUDE.md` §3 rule 5.
    missing.sort_unstable();
    Covered {
        matched,
        lacks: missing.len(),
        ambiguous: 0,
        malformed: 0,
        // NOT A JOIN, SO NEITHER UNJOINABLE BUCKET CAN FILL. No published list
        // defines this target, so no published name was ever looked up in the
        // exchange's ISIN column.
        no_nse_isin: 0,
        // NO DENOMINATOR, AND THAT IS THE FACT. NSE publishes no file naming
        // this set, so there is no published count to be short of and a number
        // here would be invention — `CLAUDE.md` §3 rule 1.
        published: None,
        unresolved: missing
            .into_iter()
            .map(|symbol| Unresolved {
                symbol,
                bucket: Bucket::Lacks,
                why: format!(
                    "{}'s instrument master lists no id for it, so a request cannot \
                     name it and this build refuses rather than sending another \
                     vendor's id",
                    vendor.as_str()
                ),
            })
            .collect(),
    }
}

/// One target's row of `/universes.json`.
///
/// The FOUR IDENTITY FIELDS ARE ALWAYS PRESENT, including for a feed with no
/// master: a page draws a row per target and must be able to label and enable
/// it before it knows whether the counts exist. Only the counts go null.
fn target_json(covered: Option<&Covered>, target: SpotTarget) -> String {
    let mut out = format!(
        r#"{{"target":{},"label":{},"note":{},"universe":{}"#,
        render::json_string(target.slug()),
        render::json_string(target.label()),
        render::json_string(target.note()),
        // THE BIT, NOT THE SET. `Swept` is `is_sweepable` — two pairs — and its
        // universe accessor answers `INDEX`, which is a wider set than the
        // target names. Emitting that token would tell a page these two rows
        // count the same instruments, so the one whose definition is not a bit
        // says so with a null.
        match target {
            SpotTarget::Swept => "null".to_owned(),
            _ => render::json_string(&universe_token(target)),
        },
    );
    match covered {
        // EVERY COUNT NULL, AND THE WORD THAT EXPLAINS ALL OF THEM. A zero here
        // would be a measurement of a file this feed does not publish.
        None => out.push_str(
            r#","counted_from":"no master","published":null,"matched":null,"lacks":null,"#,
        ),
        Some(c) => {
            let _ = write!(
                out,
                r#","counted_from":{},"published":{},"matched":{},"lacks":{},"#,
                render::json_string(c.counted_from()),
                c.published
                    .map_or_else(|| "null".to_owned(), |n| n.to_string()),
                c.matched,
                c.lacks,
            );
        }
    }
    match covered {
        // EVERY COUNT NULL, AND THE WORD THAT EXPLAINS ALL OF THEM.
        None => out.push_str(r#""ambiguous":null,"malformed":null,"no_nse_isin":null"#),
        Some(c) => {
            let _ = write!(
                out,
                r#""ambiguous":{},"malformed":{},"no_nse_isin":{}"#,
                c.ambiguous, c.malformed, c.no_nse_isin
            );
        }
    }
    // THE REASON LIST IS OPENED AND CLOSED ONCE, FOR BOTH ARMS. A feed with no
    // master emits an EMPTY array rather than omitting the field: a page that
    // reads `unresolved` must find a list there whatever the counts say, and a
    // second `push(']')` under a `covered.is_none()` guard is one brace edit
    // away from emitting an unclosed array on the arm nobody re-read.
    out.push_str(r#","unresolved":["#);
    let rows: &[Unresolved] = covered.map_or(&[], |c| &c.unresolved);
    for (n, row) in rows.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"symbol":{},"bucket":{},"why":{}}}"#,
            render::json_string(&row.symbol),
            render::json_string(row.bucket.wire()),
            render::json_string(&row.why),
        );
    }
    out.push(']');
    out.push('}');
    out
}

/// The `/instruments.json` token for the bit that defines a target.
///
/// Read out of [`crate::server::universe_token_of`] rather than spelled again:
/// two lists of these words is how one says `ntm` and the other says
/// `total_market` and a browser filter matches nothing.
fn universe_token(target: SpotTarget) -> String {
    crate::server::universe_token_of(target.universe()).to_owned()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use crate::constituents::{NoIsin, Unjoinable};
    use crate::merge::{Merged, Source, merge};
    use brutex_core::instrument::{Exchange, InstrumentKey, Kind, Segment};
    use brutex_core::isin::Isin;
    use brutex_core::symbol::Symbol;
    use brutex_core::vendor::Listing;
    use std::hint::black_box;
    use std::time::Instant;

    /// The real ISINs of four NIFTY 50 constituents, read from the two masters
    /// on 2026-08-12. Real rather than invented because `Isin::new` verifies
    /// the check digit, and because a fixture that could not exist proves
    /// nothing about a join that will meet the real file.
    const RELIANCE: &str = "INE002A01018";
    const TCS: &str = "INE467B01029";
    const INFY: &str = "INE009A01021";
    const HDFCBANK: &str = "INE040A01034";
    /// A real ISIN belonging to a company **no list this build carries names**
    /// — `GRINDWELL`. It is what the disputing vendor's row wears below, so the
    /// dispute cannot accidentally resolve some other constituent.
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
            vendor_id: brutex_core::vendor::VendorId::new(id).expect("a legal id"),
            key: key(symbol, Kind::Equity),
            isin: Some(isin(code)),
            unsuffixed: None,
        }
    }

    fn spot_index(symbol: &str, id: &str) -> Listing {
        Listing {
            vendor_id: brutex_core::vendor::VendorId::new(id).expect("a legal id"),
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

    /// A universe carrying every shape a bucket exists for.
    ///
    /// Four NIFTY 50 constituents, of which one is listed by Dhan alone
    /// (`INFY`), one is claimed by two Groww rows (`TCS`/`TCS-BE`, one ISIN),
    /// and one is disputed between the masters (`HDFCBANK`, where Dhan's row
    /// wears another company's ISIN entirely — so NSE's own column decides it
    /// and Dhan LACKS the name its master spells correctly). Three index
    /// series, of which Groww lists two and Dhan one — the shape that makes
    /// `indices` a different number per feed, which is this module's subject.
    fn universe_with_every_shape() -> Merged {
        merge(&[
            source(
                Vendor::Groww,
                vec![
                    equity("RELIANCE", RELIANCE, "NSE-RELIANCE"),
                    equity("TCS", TCS, "NSE-TCS"),
                    equity("TCS-BE", TCS, "NSE-TCS-BE"),
                    equity("HDFCBANK", HDFCBANK, "NSE-HDFCBANK"),
                    spot_index("NIFTY", "NSE-NIFTY"),
                    spot_index("BANKNIFTY", "NSE-BANKNIFTY"),
                ],
            ),
            source(
                Vendor::Dhan,
                vec![
                    equity("RELIANCE", RELIANCE, "2885"),
                    equity("TCS", TCS, "11536"),
                    equity("INFY", INFY, "1594"),
                    equity("HDFCBANK", OFF_THE_LISTS, "1333"),
                    spot_index("NIFTY", "13"),
                    spot_index("INDIAVIX", "21"),
                ],
            ),
        ])
    }

    fn built() -> (Merged, Coverage) {
        let merged = universe_with_every_shape();
        let join = Join::build(&merged);
        let coverage = Coverage::build(&merged, &join);
        (merged, coverage)
    }

    #[test]
    fn every_target_indexes_its_own_slot() {
        // `Coverage::of` reads a flat array by position in `SpotTarget::ALL`,
        // and `SpotTarget::ALL` is append-only for exactly this reason. The
        // pairing of a variant with its slot is pinned here rather than
        // trusted, because a variant inserted in the middle of the enum makes
        // `target as usize` and `position` disagree silently and hands one
        // target another one's counts.
        for (slot, target) in SpotTarget::ALL.into_iter().enumerate() {
            let named = target.slug();
            assert_eq!(target_slot(target), slot, "{named} moved");
        }
        assert_eq!(
            SpotTarget::ALL.len(),
            7,
            "seven targets, and the index knows"
        );
    }

    #[test]
    fn the_five_list_defined_targets_read_the_join_and_the_two_others_do_not() {
        let (_, coverage) = built();
        for target in SpotTarget::ALL {
            let named = target.slug();
            // `expect` with a message built here, never `unwrap_or_else(||
            // panic!(..))`: the closure is a project region no passing run
            // enters, and the 100% floor has no exemption for a test's own
            // failure path.
            let absent = format!("{named}: Groww publishes a master");
            let covered = coverage.of(Vendor::Groww, target).expect(&absent);
            if let Some(tier) = target.tier() {
                assert_eq!(
                    covered.published,
                    Some(tier.published()),
                    "{named}: a list-defined target carries its denominator"
                );
                assert_eq!(
                    covered.counted_from(),
                    "join",
                    "{named}: and says where the count came from"
                );
                assert_eq!(
                    covered.accounted(),
                    tier.published(),
                    "{named}: the five buckets are the published list"
                );
            } else {
                assert_eq!(
                    covered.published, None,
                    "{named}: NSE publishes no file naming this set, so there is \
                         no denominator and inventing one would be §3 rule 1"
                );
                assert_eq!(covered.counted_from(), "master", "{named}");
                assert_eq!(
                    covered.ambiguous, 0,
                    "{named}: a master fold has no ISIN to be ambiguous about"
                );
                assert_eq!(covered.malformed, 0, "{named}");
                assert_eq!(
                    covered.no_nse_isin, 0,
                    "{named}: no published name was looked up in NSE's column"
                );
            }
        }
    }

    #[test]
    fn the_two_feeds_reach_different_index_sets_and_the_counts_say_so() {
        // THE LIVE SYMPTOM, IN A FIXTURE. Three index series exist; Groww
        // lists NIFTY and BANKNIFTY, Dhan lists NIFTY and INDIAVIX. The
        // universe count is 3 for both and neither feed reaches 3. A form that
        // renders the universe count beside a chosen feed promises an
        // instrument the run will refuse by name.
        let (merged, coverage) = built();
        let in_universe = merged
            .by_key
            .iter()
            .filter(|(key, entry)| SpotTarget::Indices.names(key, entry.universe))
            .count();
        assert_eq!(in_universe, 3, "three index series in the merged universe");

        let groww = coverage
            .of(Vendor::Groww, SpotTarget::Indices)
            .expect("Groww publishes a master");
        assert_eq!(groww.matched, 2, "Groww lists NIFTY and BANKNIFTY");
        assert_eq!(groww.lacks, 1);
        assert_eq!(
            groww
                .unresolved
                .iter()
                .map(|u| u.symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["INDIAVIX"],
            "and the one it does not list is named, not counted"
        );

        let dhan = coverage
            .of(Vendor::Dhan, SpotTarget::Indices)
            .expect("Dhan publishes a master");
        assert_eq!(dhan.matched, 2, "Dhan lists NIFTY and INDIAVIX");
        assert_eq!(dhan.lacks, 1);
        assert_eq!(
            dhan.unresolved
                .iter()
                .map(|u| u.symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["BANKNIFTY"]
        );
        assert_eq!(
            groww.accounted(),
            in_universe,
            "the fold accounts for every row the target names"
        );
        assert_eq!(dhan.accounted(), in_universe);
    }

    #[test]
    fn the_swept_pair_is_counted_per_feed_like_everything_else() {
        // `Swept` is `is_sweepable`, not a bit test, and it is the one target
        // whose membership is decided by a table in `core`. It still gets a
        // per-feed answer, because the engine surface being two instruments
        // does not mean a given broker lists both.
        let (_, coverage) = built();
        let groww = coverage
            .of(Vendor::Groww, SpotTarget::Swept)
            .expect("Groww publishes a master");
        assert_eq!(groww.matched, 2, "Groww lists both swept series");
        assert_eq!(groww.lacks, 0);
        let dhan = coverage
            .of(Vendor::Dhan, SpotTarget::Swept)
            .expect("Dhan publishes a master");
        assert_eq!(dhan.matched, 1, "Dhan lists NIFTY and not BANKNIFTY");
        assert_eq!(
            dhan.unresolved
                .iter()
                .map(|u| u.symbol.as_str())
                .collect::<Vec<_>>(),
            vec!["BANKNIFTY"],
            "so a swept run on this feed is one instrument, and it says which is missing"
        );
    }

    #[test]
    fn the_matched_count_is_the_length_of_the_ids_a_pull_would_name() {
        // The number on the control and the array the request is built from
        // are ONE value. Two tallies of the same fact drift the first time one
        // of them is edited, and the visible half is the one nobody edits.
        let merged = universe_with_every_shape();
        let join = Join::build(&merged);
        let coverage = Coverage::build(&merged, &join);
        for vendor in Vendor::MASTERED {
            for target in SpotTarget::ALL {
                let named = format!("{} · {}", vendor.as_str(), target.slug());
                let absent = format!("{named}: a mastered vendor is counted");
                let covered = coverage.of(vendor, target).expect(&absent);
                match join.ids_for(vendor, target) {
                    Some(ids) => assert_eq!(ids.len(), covered.matched, "{named}"),
                    None => assert!(
                        target.tier().is_none(),
                        "{named}: only the two master-counted targets have no ids"
                    ),
                }
            }
        }
    }

    #[test]
    fn a_feed_that_publishes_no_master_is_not_reported_as_empty() {
        // AN ARCHIVE HAS NO MASTER. Answering `0 matched` would be a
        // measurement of a file that does not exist, and it reads as "this
        // feed has nothing" — `CLAUDE.md` §4's fallback that hides a failure,
        // arriving as a plausible number.
        let (_, coverage) = built();
        for vendor in Vendor::ALL {
            for target in SpotTarget::ALL {
                // Built BEFORE the assertion: a message formatted inside the
                // macro is a region no passing run enters, and the coverage
                // floor this repository holds has no exemption for "it is only
                // a failure message".
                let at = format!("{} · {}", vendor.as_str(), target.slug());
                assert_eq!(
                    coverage.of(vendor, target).is_some(),
                    vendor.publishes_master(),
                    "{at}"
                );
            }
        }
        let json = coverage.json(Vendor::TrueData);
        assert!(
            json.contains(r#""mastered":false"#),
            "the wire says it, once, for the whole feed: {json}"
        );
        assert!(
            json.contains(r#""counted_from":"no master","published":null,"matched":null,"lacks":null,"ambiguous":null,"malformed":null,"no_nse_isin":null,"unresolved":[]"#),
            "and every count is null rather than zero: {json}"
        );
        assert!(
            json.contains(r#""target":"n50""#),
            "the row is still drawn and still names its slug, because a page \
             labels a control before it knows the counts: {json}"
        );
    }

    #[test]
    fn the_wire_carries_the_slug_the_count_and_every_reason() {
        let (_, coverage) = built();
        let json = coverage.json(Vendor::Groww);
        assert!(
            json.starts_with(r#"{"feed":"groww","mastered":true,"targets":["#),
            "{json}"
        );
        assert!(json.ends_with("]}"), "{json}");
        // THE FOUR TIERS THE `/ingest` MENU DREW AS `no target`. Each one now
        // carries the slug to POST, the token that picks its rows out of
        // `/instruments.json`'s `universes` array, and a real count.
        for slug in ["n50", "n100", "n200", "n500"] {
            assert!(
                json.contains(&format!(r#""target":"{slug}""#)),
                "{slug} is requestable: {json}"
            );
            assert!(
                json.contains(&format!(r#""universe":"{slug}""#)),
                "{slug} names the bit that selects its rows: {json}"
            );
        }
        assert!(
            json.contains(r#""target":"equities","label":"NIFTY Total Market equities""#),
            "{json}"
        );
        assert!(
            json.contains(r#""target":"swept","label":"Swept indices","note":"NSE-NIFTY and NSE-BANKNIFTY — the only two swept","universe":null"#),
            "the engine surface is not a universe bit and does not claim to be: {json}"
        );
        assert!(
            json.contains(r#""universe":"index""#),
            "the reference indices ARE a bit: {json}"
        );
        // Every bucket, with a reason a human reads rather than a code.
        assert!(
            json.contains(r#""symbol":"TCS","bucket":"ambiguous""#),
            "two Groww rows carry one ISIN: {json}"
        );
        // NSE'S OWN GAP TRAVELS UNDER ITS OWN WORD. `AGL` is the one Total
        // Market position whose row in the exchange's file has an empty ISIN
        // cell; drawing it as `malformed` would blame this repository's
        // transcription for the exchange's own gap.
        assert!(
            json.contains(r#""symbol":"AGL","bucket":"no_nse_isin","why":"the exchange's own row for this name names no ISIN""#),
            "{json}"
        );
        assert!(
            json.contains(r#""malformed":0,"no_nse_isin":1,"#),
            "and it is counted in its own bucket, not folded into another: {json}"
        );
        assert!(
            json.contains(r#""symbol":"INFY","bucket":"lacks","why":"this feed's master carries no row for ISIN INE009A01021""#),
            "Dhan alone lists INFY, so Groww lacks it and the ISIN is named: {json}"
        );
        assert!(
            json.contains(
                r#""bucket":"lacks","why":"groww's instrument master lists no id for it"#
            ),
            "and a master-counted miss says which feed did not list it: {json}"
        );

        // THE SYMBOL STEP IS GONE, AND THE WIRE SHOWS IT. Dhan's master has a
        // row spelled `HDFCBANK` wearing another company's ISIN. Under NSE's
        // own key Dhan LACKS the name, and the ISIN on the row is the one the
        // exchange prints — never the one Dhan filed.
        let dhan = coverage.json(Vendor::Dhan);
        assert!(
            dhan.contains(r#""symbol":"HDFCBANK","bucket":"lacks","why":"this feed's master carries no row for ISIN INE040A01034""#),
            "{dhan}"
        );
        assert!(
            !dhan.contains("INE536A01023"),
            "the id filed under another company's ISIN never reaches this answer: {dhan}"
        );
    }

    #[test]
    fn the_notes_name_only_the_targets_a_feed_is_short_of() {
        let (_, coverage) = built();
        let notes = coverage.notes();
        assert!(!notes.is_empty(), "this fixture is short on every target");
        for line in &notes {
            assert!(
                line.contains("reachable"),
                "every line says how many of how many: {line}"
            );
            assert!(
                line.contains("target="),
                "and names the slug a run would use: {line}"
            );
            // AND SAYS WHAT THE RUN ACTUALLY DOES, which is not yet what this
            // count says it should. `broker_run` filters by the universe, so
            // the shortfall is refused instrument by instrument rather than
            // never attempted; the note states that gap rather than implying a
            // filter nobody wrote. docs/06-limits.md §63.
            assert!(
                line.contains("STILL ATTEMPTS ALL"),
                "the note must not claim a filter the pull path does not have: {line}"
            );
        }
        assert!(
            notes
                .iter()
                .any(|l| l.contains("groww · Reference indices: 2 of 3 reachable")),
            "{notes:?}"
        );
        assert!(
            notes.iter().any(|l| l.contains("INDIAVIX")),
            "the missing name is on the line, not just its count: {notes:?}"
        );

        // A BUILD THAT IS SHORT OF NOTHING SAYS NOTHING. A note per target
        // would be fourteen lines to scroll past on a healthy start, and the
        // whole content of this module is the rows where two numbers disagree.
        let whole = merge(&[
            source(Vendor::Groww, vec![spot_index("NIFTY", "NSE-NIFTY")]),
            source(Vendor::Dhan, vec![spot_index("NIFTY", "13")]),
        ]);
        let join = Join::build(&whole);
        let quiet = Coverage::build(&whole, &join);
        let lines = quiet.notes();
        let indices: Vec<&String> = lines
            .iter()
            .filter(|l| l.contains("Reference indices"))
            .collect();
        assert!(
            indices.is_empty(),
            "both feeds reach the one index series, so nothing is said about it: {lines:?}"
        );
    }

    #[test]
    fn the_two_unjoinable_buckets_travel_under_different_words() {
        // No published list this build carries holds a name that cannot be a
        // key, so no tier ever produces a `malformed` row — and a bucket only a
        // bad rebalance could fill is a bucket no test would reach THROUGH a
        // tier. `from_join` is handed the rows directly instead.
        //
        // The two are not interchangeable and the split is the point:
        // `malformed` is a transcription to fix in THIS repository,
        // `no_nse_isin` is a cell the EXCHANGE left empty and nothing here may
        // fill. One word for both would send an operator to fix a
        // transcription that is correct.
        let mut tier = TierJoin::default();
        tier.malformed.push(Unjoinable {
            symbol: "DUMMYINXGN",
            why: NoIsin::PlaceholderScrip,
        });
        tier.no_nse_isin.push(Unjoinable {
            symbol: "AGL",
            why: NoIsin::TheExchangesRowNamesNoIsin,
        });
        let covered = from_join(&tier, &[], 2);
        assert_eq!(covered.matched, 0);
        assert_eq!(covered.malformed, 1);
        assert_eq!(covered.no_nse_isin, 1);
        assert_eq!(covered.accounted(), 2, "the partition sums off a tier too");
        let rows: Vec<(&str, &str, &str)> = covered
            .unresolved
            .iter()
            .map(|u| (u.symbol.as_str(), u.bucket.wire(), u.why.as_str()))
            .collect();
        assert_eq!(
            rows,
            vec![
                (
                    "DUMMYINXGN",
                    "malformed",
                    "an NSE placeholder scrip, not a constituent"
                ),
                (
                    "AGL",
                    "no_nse_isin",
                    "the exchange's own row for this name names no ISIN"
                ),
            ],
            "each row carries its own word and its own sentence"
        );
    }

    #[test]
    fn every_bucket_word_is_distinct_and_lower_case() {
        // The four words go on the wire and a page switches on them, so they
        // are pinned here rather than left to a `Debug` rendering that changes
        // with a rename.
        let words: Vec<&str> = [
            Bucket::Lacks,
            Bucket::Ambiguous,
            Bucket::Malformed,
            Bucket::NoNseIsin,
        ]
        .into_iter()
        .map(Bucket::wire)
        .collect();
        assert_eq!(
            words,
            vec!["lacks", "ambiguous", "malformed", "no_nse_isin"]
        );
        let mut sorted = words.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), words.len(), "four words, not three");
    }

    #[test]
    fn the_lookup_does_not_grow_with_the_universe() {
        // `Coverage::of` is one array index and must stay one. Measured
        // against two universes an order apart in rows: a lookup that went
        // back to folding the merged map would show it here, and nowhere else
        // until an operator's page slowed down.
        //
        // The published lists are compile-time constants, so the join-backed
        // half of the answer is the same size in both — what changes is the
        // merged universe the master-counted half was folded from, which is
        // the only thing a regression could start scanning.
        let small = merge(&[source(Vendor::Groww, vec![spot_index("NIFTY", "1")])]);
        let mut many = vec![spot_index("NIFTY", "1")];
        for (n, name) in brutex_core::universe::NIFTY_500.iter().enumerate() {
            // Every published name is a legal symbol — `core`'s own lists are
            // transcribed from the exchange's files and `Symbol::new` accepts
            // them — so this expects rather than skipping. A name that failed
            // here would be a defect in the transcription, and swallowing it
            // would hide that from the one test that walks all 500.
            let symbol = Symbol::new(name).expect("a published constituent is a legal symbol");
            many.push(Listing {
                vendor_id: brutex_core::vendor::VendorId::new(&format!("id{n}"))
                    .expect("a legal id"),
                key: InstrumentKey {
                    exchange: Exchange::Nse,
                    segment: Segment::Index,
                    underlying: symbol,
                    kind: Kind::Index,
                },
                isin: None,
                unsuffixed: None,
            });
        }
        let large = merge(&[source(Vendor::Groww, many)]);
        let sizes = format!("{} against {}", large.by_key.len(), small.by_key.len());
        assert!(
            large.by_key.len() > small.by_key.len() * 40,
            "the two universes are 40x apart: {sizes}"
        );

        let a = Coverage::build(&small, &Join::build(&small));
        let b = Coverage::build(&large, &Join::build(&large));
        let time = |c: &Coverage| {
            let rounds = 20_000;
            // Warm, so the first measurement is not the one that paid for the
            // page faults.
            for _ in 0..rounds {
                black_box(c.of(black_box(Vendor::Groww), black_box(SpotTarget::Nifty50)));
            }
            let at = Instant::now();
            for _ in 0..rounds {
                black_box(c.of(black_box(Vendor::Groww), black_box(SpotTarget::Nifty50)));
            }
            at.elapsed().as_nanos().max(1) / rounds
        };
        let (fast, slow) = (time(&a), time(&b));
        let grew =
            format!("the lookup grew with the universe: {fast} ns against {slow} ns, {sizes}");
        assert!(slow <= fast.saturating_mul(4).max(4), "{grew}");
    }
}
