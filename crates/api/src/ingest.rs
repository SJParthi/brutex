//! What the operator asked a pull to do, and every way it is refused **by
//! name**.
//!
//! # Two forms, not one form with a dropdown
//!
//! A spot pull and an expired-derivative pull do not take the same fields. Spot
//! takes a set of instruments and a window; F&O additionally takes an
//! underlying, a series and an expiry, and its expiry is the field that decides
//! whether the request is legal at all. Folding them into one form means every
//! field is optional, which means every field has to be checked for presence
//! twice — once for "did you fill it in" and once for "does this half of the
//! form need it" — and the second check is the one that gets forgotten. Two
//! forms, two parsers, two shapes.
//!
//! # The dates are `YYYY-MM-DD` because that is what the wire takes
//!
//! `<input type="date">` submits exactly that, and it is exactly what
//! [`pull::session::Day`]'s [`std::fmt::Display`] produces — the shape both
//! vendors take. There is no `DD/MM` versus `MM/DD` question anywhere on the
//! path, because no format that could be ambiguous is ever accepted.
//!
//! The text is parsed **here** and validated **there**: this module splits the
//! ten characters into three integers and hands them to [`Day::new`], which
//! owns every calendar rule including the Gregorian leap year. `session.rs`
//! deliberately has no constructor taking text, and this is not one — it is a
//! form field being turned into the three numbers that constructor already
//! refuses.
//!
//! # A live contract cannot be requested, structurally
//!
//! `CLAUDE.md` §1 permits futures and options to be **stored**; the operator's
//! rule is narrower — only expired series. [`parse_fno`] takes the current day
//! as an argument and refuses any expiry that is not strictly behind it, and
//! the form renders `max` on the expiry input from the same value. The form
//! makes it unofferable and the parser makes it unrequestable, so a hand-built
//! `POST` is refused by the same rule that greys the field out.
//!
//! # Two routes that answer without running
//!
//! [`status_json`] says what the next sweep is waiting on, and [`queue`] says
//! whether a selection would be accepted. Both are here rather than in
//! `server.rs` because both are about a request's ADMISSION, which is what this
//! module already owns — and neither of them opens a socket, reads the store or
//! writes a byte. D-0092 and D-0094 say why the first is served from memory and
//! why the second never queues anything.
//!
//! # Cost
//!
//! Every function here is bounded arithmetic over a form body whose length the
//! server caps. Membership of the F&O universe is one probe of a compile-time
//! table (`brutex_core::universe::of_equity`), never a scan. The two routes add
//! one uncontended lock and one pass over the backfill's feed reports — two
//! rows on this build — and no I/O of any kind.

use std::fmt;
use std::time::SystemTime;

use brutex_core::symbol::Symbol;
use brutex_core::universe::{self, Universe};
use pull::session::{Day, IstMoment, SessionError, Window};

use crate::server::param;

/// The longest window one request may ask for, in days.
///
/// `docs/07-o1-architecture.md` law 5 — bound every input at the boundary. Ten
/// years of calendar days, which is longer than either vendor's published
/// history and short enough that the day count cannot be mistaken for a
/// fat-fingered year. A request past it is refused naming both numbers, so an
/// operator who legitimately needs more is told what to raise.
pub const MAX_WINDOW_DAYS: u32 = 3_653;

/// Which instruments a spot pull covers.
///
/// Seven fixed sets, not a free list. `CLAUDE.md` §1 fixes the engine surface
/// at two swept indices; the other six sets are stored and never swept, and
/// saying which is which on the form is the difference between an operator
/// knowing what a run will contain and guessing.
///
/// # PULLABLE IS NOT SWEPT, and appending here cannot make it so
///
/// D-0105 appended the four published NIFTY tiers because the browser could
/// already SEE them and could not ASK for them. `/instruments.json` has emitted
/// a `universes` array naming `n500`, `n200`, `n100` and `n50` per row since
/// D-0089 — measured live at 500/200/100/50 on both feeds — and `/ingest`
/// disabled those four rows for exactly one reason: no slug existed to put in
/// the `target` field, so the page hardcoded a null and said so. That was a gap
/// in the REQUEST vocabulary and in nothing else.
///
/// A pull of `n500` therefore **stores** 500 instruments and **sweeps none of
/// them**. Nothing in this enum is consulted by the sweep: what may be swept is
/// [`brutex_core::instrument::InstrumentKey::is_sweepable`], a two-element table
/// in `core`, and [`Self::Swept`] is the one variant that defers to it rather
/// than keeping a second copy. Widening `CLAUDE.md` §1 is a
/// `docs/05-decisions.md` entry against THAT table; a row here is not one and
/// must never be read as one. `a_nifty_tier_target_stores_and_never_sweeps` is
/// that sentence written as a test.
///
/// # Why the slugs are the wire's own words
///
/// `n50`, `n100`, `n200` and `n500` are already what `/instruments.json` spells
/// these universes as — `server::UNIVERSE_TOKENS`, appended by D-0089. One
/// vocabulary serves both directions, so the token a browser counts a set BY is
/// the token a pull is requested WITH. A second spelling here (`nifty50`,
/// `n_50`) would be the defect that table's own doc comment exists to prevent,
/// arriving from the other end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpotTarget {
    /// `NSE-NIFTY` and `NSE-BANKNIFTY` — the only two the engine sweeps.
    Swept,
    /// Every NSE index series held for reference, including `NSE-INDIAVIX`.
    ///
    /// Stored and stamped onto trades; never in the condition vocabulary, never
    /// in ranking, never in run identity. `CLAUDE.md` §1.
    Indices,
    /// The NIFTY Total Market constituents. Stored, never swept.
    Equities,
    /// The published NIFTY 500 constituents. Stored, never swept.
    Nifty500,
    /// The published NIFTY 200 constituents. Stored, never swept.
    Nifty200,
    /// The published NIFTY 100 constituents. Stored, never swept.
    Nifty100,
    /// The published NIFTY 50 constituents. Stored, never swept.
    ///
    /// Fifty EQUITIES, and not the index of the same name: `NSE-NIFTY` is the
    /// spot series [`Self::Swept`] names, and these are the companies underneath
    /// it. The two sets share a word and no member, which is precisely why both
    /// are spelled out on the form rather than left to be inferred.
    Nifty50,
    /// The 213 F&O underlyings. Stored, never swept.
    ///
    /// Defined by a list this repository has already transcribed —
    /// `core::universe::FNO_UNDERLYINGS`, derived from the derivative rows of
    /// both vendor masters, which agree exactly at 213 (`docs/00-charter.md`
    /// §4a). It is the one target whose join was already built and reachable
    /// from nothing: `constituents::Tier::FnoUnderlyings` has always existed,
    /// declared `published() == 213`, and no `SpotTarget` pointed at it.
    ///
    /// **It NAMES the swept pair, and that is legal.** `NIFTY` and `BANKNIFTY`
    /// are both F&O underlyings, so pulling this set stores them — which
    /// `CLAUDE.md` §1 permits and [`Self::Indices`] has always done. What it
    /// does not do is widen the sweep: `InstrumentKey::SWEPT` is still two
    /// pairs and this target cannot add to it.
    Fno,
    /// Every instrument this build tracks — the reference index series and the
    /// NIFTY Total Market constituents together. Stored, never swept.
    ///
    /// # The one target defined by a PREDICATE and not by a file
    ///
    /// Every other target answers to a published list or to a single named
    /// universe bit. This one is `catalog::tracked` — the union the pull path
    /// already filters by — and it has neither. That is not a gap to be filled:
    /// NSE publishes no file naming "everything this build tracks", so a roster
    /// here would be invented (`CLAUDE.md` §3 rule 1) and would go stale the day
    /// a vendor added a series. [`Self::members`], [`Self::tier`] and
    /// [`Self::universe`] each say so rather than guessing.
    Everything,
}

impl SpotTarget {
    /// Every target, in the order the form lists them.
    ///
    /// **APPEND ONLY.** `server::Site::targets` is one counter per slot and
    /// `spot_answer` reads a slot by position, so reordering this array
    /// relabels counts that are already on an operator's screen. The four tiers
    /// go on the end, widest first, which is also the order
    /// `server::UNIVERSE_TOKENS` lists them in — one reading order for the two
    /// tables that name the same sets.
    pub const ALL: [Self; 9] = [
        Self::Swept,
        Self::Indices,
        Self::Equities,
        Self::Nifty500,
        Self::Nifty200,
        Self::Nifty100,
        Self::Nifty50,
        // APPENDED AT 7 AND 8, AND NOT WHERE THE BROWSER DRAWS THEM.
        //
        // `web/src/routes/ingest/+page.svelte` lists `fno` sixth, between the
        // Total Market row and the index row. Mirroring that reading order here
        // would insert `Fno` at slot 3 and shift all four NIFTY tiers down one —
        // and `Coverage::per` is a flat array read as
        // `vendor * ALL.len() + slot`, so every tier would be handed its
        // neighbour's counter on the form and on the receipt. The array's order
        // is a wire contract with itself; the page's order is a reading order,
        // and they are allowed to differ. D-0136.
        Self::Fno,
        Self::Everything,
    ];

    /// Whether this target names `key`, given the universes it belongs to.
    ///
    /// # The defect this removes
    ///
    /// The run built its instrument list from `catalog::tracked` alone — every
    /// member of `TOTAL_MARKET` or `INDEX`, 765 names — and never consulted the
    /// target the operator chose. Selecting *Swept indices*, which the page
    /// labels "NSE-NIFTY and NSE-BANKNIFTY, the only two swept" beside a
    /// counter reading **2**, still swept all 765 and began at `360ONE`. The
    /// label, the counter and the run were three different answers to one
    /// question.
    ///
    /// TWO CONSTANT-TIME TESTS, not a lookup. `is_sweepable` compares against a
    /// two-element table and `contains` is a bitflag test, so deciding whether
    /// one instrument is in the target costs the same at 800 as at one — this
    /// runs once per candidate while building the list, never inside the pull.
    #[must_use]
    pub fn names(
        self,
        key: &brutex_core::instrument::InstrumentKey,
        universe: brutex_core::universe::Universe,
    ) -> bool {
        match self {
            // The engine's own predicate, not a second copy of the pair.
            // Widening what may be swept is a `docs/05-decisions.md` entry, and
            // it must widen this form at the same moment it widens the engine.
            Self::Swept => key.is_sweepable(),
            Self::Indices => universe.contains(brutex_core::universe::Universe::INDEX),
            Self::Equities => universe.contains(brutex_core::universe::Universe::TOTAL_MARKET),
            // ONE BIT TEST PER TIER, and the bit was set by
            // `core::universe::of_equity` reading that tier's own published
            // file. This asks the constituent list's answer without holding a
            // copy of the list, so a rebalance lands here the moment `core` is
            // updated and cannot land in one place and not the other.
            //
            // NO CATCH-ALL, deliberately. An eighth target has to be a compile
            // error in this match: `names` is the only thing that decides which
            // instruments a request covers, and a `_` arm here would let a new
            // variant silently inherit some other set's membership.
            Self::Nifty500 => universe.contains(Universe::NIFTY_500),
            Self::Nifty200 => universe.contains(Universe::NIFTY_200),
            Self::Nifty100 => universe.contains(Universe::NIFTY_100),
            Self::Nifty50 => universe.contains(Universe::NIFTY_50),
            // ONE BIT, exactly like the four tiers above it.
            Self::Fno => universe.contains(Universe::FNO),
            // THE PULL PATH'S OWN PREDICATE, BORROWED — never `true`.
            //
            // `Site::new` counts a target's population with `names` alone, and
            // `broker_run` builds the run list with
            // `catalog::tracked(..) && names(..)`. For every other target
            // `names` is already a subset of `tracked`, so the two agree.
            // `true` here would break that: the form would count
            // `merged.by_key.len()` — ~2,795 rows including futures, options and
            // BSE listings that `of_instrument` gives `Universe::NONE` — while
            // the run attempted 765. That is exactly the defect `names` was
            // added to remove: the label, the counter and the run being three
            // different answers to one question.
            //
            // Borrowed rather than copied, the same discipline
            // `Self::Swept => key.is_sweepable()` follows, so the `tracked &&`
            // in `broker_run` is idempotent for this target instead of a
            // second, invisible filter.
            Self::Everything => crate::catalog::tracked(universe),
        }
    }

    /// The published constituent list that DEFINES this target, when a
    /// published list is what defines it.
    ///
    /// Borrowed from `brutex_core::universe`, never copied. The four tiers hand
    /// back the very consts D-0089 transcribed from NSE's own CSVs, and
    /// [`Self::Equities`] hands back the Total Market list beside them. A second
    /// copy in this crate would be a second answer to "who is in the NIFTY 200",
    /// and the stale one would be whichever nobody remembered to rebalance —
    /// `CLAUDE.md` §3 rule 1.
    ///
    /// # `None` hides nothing
    ///
    /// It is not a fallback (`CLAUDE.md` §4). Two targets are defined by
    /// something that is not a constituent file, and say so rather than
    /// inventing one:
    ///
    /// * [`Self::Swept`] is the ENGINE SURFACE — `InstrumentKey::SWEPT`, two
    ///   `(exchange, symbol)` pairs. Answering it with a symbol list would put a
    ///   third copy of §1 in a third crate.
    /// * [`Self::Indices`] is whatever the vendor master lists as an index
    ///   series on this build.
    ///
    ///   **THIS ROW USED TO CARRY A REASON THAT WAS FALSE.** It read: *"NSE
    ///   publishes no file naming that set, so a hardcoded one would be
    ///   invention and would go stale the day a vendor added a series."* The
    ///   second half is right and the first half is not. NSE Indices Limited
    ///   publishes a categorised directory of every equity index it computes —
    ///   **148 of them**, across four category pages, each index's own page
    ///   linking its constituent CSV under the same
    ///   `Company Name,Industry,Symbol,Series,ISIN Code` header §4c already
    ///   reads. Read 14 Aug 2026; `docs/00-charter.md` §4d.
    ///
    ///   So `None` here is still correct and its reason has changed. A
    ///   published list exists and **this build has not resolved it**: the
    ///   constituent URL is not derivable from an index's name
    ///   (`ind_niftybanklist.csv` against `ind_niftytotalmarket_list.csv` —
    ///   word-joined against underscore-separated, with no rule between them),
    ///   so the directory has to be crawled rather than composed, and nothing
    ///   crawls it yet. Answering with a transcribed 148-name array would be
    ///   the same photograph-of-a-rebalancing-index this repository already
    ///   holds for the constituent tiers, one level up.
    ///
    ///   Until the resolver exists, this target remains **per feed and
    ///   unverified**: two feeds may legitimately answer differently and
    ///   nothing here can say which is right.
    ///
    /// Membership is decided by [`Self::names`] for all seven either way. This
    /// is the roster, not the predicate, and nothing on the pull path reads it:
    /// it exists so a test can pin what a tier resolves to against the file it
    /// came from.
    #[must_use]
    pub const fn members(self) -> Option<&'static [&'static str]> {
        match self {
            // THREE TARGETS HAVE NO PUBLISHED LIST, AND THEY LACK ONE FOR THREE
            // DIFFERENT REASONS. `Swept` is the engine surface, two
            // `(exchange, symbol)` pairs. `Indices` is whatever a vendor's
            // master calls an index series, which NSE publishes no file for.
            // `Everything` is a PREDICATE over the tracked union — see its own
            // documentation.
            Self::Swept | Self::Indices | Self::Everything => None,
            Self::Equities => Some(&universe::NIFTY_TOTAL_MARKET),
            // THE LIST WAS ALREADY TRANSCRIBED AND HAD NO TARGET POINTING AT
            // IT. 213 names, derived from both vendors' derivative rows, which
            // agree exactly — docs/00-charter.md §4a.
            Self::Fno => Some(&universe::FNO_UNDERLYINGS),
            Self::Nifty500 => Some(&universe::NIFTY_500),
            Self::Nifty200 => Some(&universe::NIFTY_200),
            Self::Nifty100 => Some(&universe::NIFTY_100),
            Self::Nifty50 => Some(&universe::NIFTY_50),
        }
    }

    /// The value this target carries on the wire.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Swept => "swept",
            Self::Indices => "indices",
            Self::Equities => "equities",
            // THE WIRE'S OWN WORDS. These four are `server::UNIVERSE_TOKENS`
            // verbatim — the tokens `/instruments.json` already emits in every
            // row's `universes` array. One vocabulary, both directions.
            Self::Nifty500 => "n500",
            Self::Nifty200 => "n200",
            Self::Nifty100 => "n100",
            Self::Nifty50 => "n50",
            // THE WIRE'S OWN WORD AGAIN: `fno` is bit 1 in
            // `server::UNIVERSE_TOKENS` and is what `/instruments.json` already
            // counts this set by. D-0105's rule leaves no other spelling legal.
            Self::Fno => "fno",
            // AND THE ONE PLACE THAT RULE HAS NO ANSWER. There is no
            // `/instruments.json` token for "everything": the browser has used
            // `*` as a LOCAL sentinel, not a wire word, and `*` is not a slug a
            // query string should carry. `all` is chosen here rather than
            // borrowed, and saying so is the point — D-0136.
            Self::Everything => "all",
        }
    }

    /// What the form calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Swept => "Swept indices",
            Self::Indices => "Reference indices",
            Self::Equities => "NIFTY Total Market equities",
            Self::Nifty500 => "NIFTY 500 equities",
            Self::Nifty200 => "NIFTY 200 equities",
            Self::Nifty100 => "NIFTY 100 equities",
            Self::Nifty50 => "NIFTY 50 equities",
            Self::Fno => "F&O underlyings",
            // UNDER 64 BYTES ON PURPOSE. Every spot record writes
            // `target.label()` into `audit::Record`'s fixed-stride 64-byte
            // `source` field, and the stride is a shipped format §3 rule 8
            // forbids mutating. A descriptive label would be cut to a prefix.
            Self::Everything => "Everything tracked",
        }
    }

    /// One line saying what the engine does with what this pulls.
    #[must_use]
    pub const fn note(self) -> &'static str {
        match self {
            Self::Swept => "NSE-NIFTY and NSE-BANKNIFTY — the only two swept",
            Self::Indices => "stored and stamped onto trades; never swept",
            Self::Equities => "stored, never swept",
            // EVERY TIER SAYS "never swept" IN THE SAME BREATH AS ITS COUNT.
            // The count is the reason an operator picks the row and the four
            // words after it are the reason picking it does not widen §1.
            Self::Nifty500 => "the published 500 constituents — stored, never swept",
            Self::Nifty200 => "the published 200 constituents — stored, never swept",
            Self::Nifty100 => "the published 100 constituents — stored, never swept",
            Self::Nifty50 => {
                "the published 50 constituents — stored, never swept; these are the \
                 companies, not the NIFTY index"
            }
            // BOTH WORDED LIKE `Indices`, NOT LIKE `Equities`, AND THE
            // DIFFERENCE IS FACTUAL.
            //
            // Each of these sets CONTAINS NSE-NIFTY and NSE-BANKNIFTY — both
            // are F&O underlyings and both are tracked index series. "Stored,
            // never swept" is true of the SET but reads as "this set excludes
            // the swept pair", which is false. So they say what `Indices`
            // says: the pull stores them, and the sweep surface is unchanged.
            Self::Fno => "the 213 F&O underlyings — stored; the sweep surface is never swept",
            Self::Everything => {
                "every tracked series and constituent — stored; the sweep surface \
                 is unchanged and never swept"
            }
        }
    }

    /// Which universe bit selects this target's members.
    ///
    /// One NAMED bit each, never a comparison of whole bitsets: an equity in the
    /// NIFTY 50 carries `fno | ntm | n500 | n200 | n100 | n50` at once, and only
    /// the named bit decides.
    #[must_use]
    pub const fn universe(self) -> Universe {
        match self {
            Self::Swept | Self::Indices => Universe::INDEX,
            Self::Equities => Universe::TOTAL_MARKET,
            Self::Nifty500 => Universe::NIFTY_500,
            Self::Nifty200 => Universe::NIFTY_200,
            Self::Nifty100 => Universe::NIFTY_100,
            Self::Nifty50 => Universe::NIFTY_50,
            Self::Fno => Universe::FNO,
            // NO BIT, AND THE HONEST ANSWER IS THE EMPTY ONE.
            //
            // `Everything` is `TOTAL_MARKET | INDEX` and `Universe` has no bit
            // for the union. Inventing one would be wrong in kind: these bits
            // name PUBLISHED universes, one file each, and `tracked` is a union
            // rather than a file. Answering `INDEX` or `TOTAL_MARKET` would be
            // worse than empty — `/universes.json` would then tell a page that
            // this row and the `indices` row count the same instruments, which
            // is the exact confusion the `Swept => null` arm exists to prevent.
            //
            // `Universe::NONE` is only honest while its CONSUMER says so:
            // `coverage::target_json` emits `null` for this target rather than
            // letting the empty bitset become an empty token. The two are a
            // pair. D-0136.
            Self::Everything => Universe::NONE,
        }
    }

    /// The published constituent list whose JOIN answers this target, when one
    /// does.
    ///
    /// # Why this exists beside [`Self::members`] rather than instead of it
    ///
    /// `members` is the roster — the names the exchange published — and it is
    /// the same list for every feed. This is the pointer to the *answer* for
    /// **one** feed: [`crate::constituents::Join::tier`] holds, per vendor, the
    /// ids that roster resolved to and the whole of what did not. A form that
    /// shows a roster length tells the operator what NSE published; a form that
    /// shows the matched bucket tells them what the pull they are about to
    /// start can actually name. Those are different numbers the moment one
    /// master is missing a row, and the second is the one on the button.
    ///
    /// # `None` hides nothing, for the same two targets as `members`
    ///
    /// [`Self::Swept`] is the engine surface — two `(exchange, symbol)` pairs
    /// from `InstrumentKey::SWEPT`, not a constituent file — and
    /// [`Self::Indices`] is whatever a vendor's master calls an index series,
    /// which NSE publishes no file for. Neither has a published count to be a
    /// denominator, so neither gets a join, and
    /// [`crate::server::Coverage`] counts both off the merged universe and says
    /// so on the wire rather than inventing a roster for them.
    ///
    /// **NO CATCH-ALL**, for the reason [`Self::names`] has none: an eighth
    /// variant must be a compile error here, because a `_` arm would silently
    /// hand a new target some other tier's ids.
    #[must_use]
    pub const fn tier(self) -> Option<crate::constituents::Tier> {
        use crate::constituents::Tier;
        match self {
            Self::Swept | Self::Indices | Self::Everything => None,
            Self::Equities => Some(Tier::TotalMarket),
            // THE JOIN WAS ALREADY BUILT AND NOTHING REACHED IT.
            //
            // `Tier::FnoUnderlyings` is in `Tier::ALL`, borrows
            // `universe::FNO_UNDERLYINGS`, declares `published() == 213` and
            // `universe() == Universe::FNO`, and `Join::build` has always
            // resolved it per vendor — with no `SpotTarget` mapping to it, so
            // only `Join::notes` consumed it. Leaving this `None` would route
            // the target through `from_master`: the 213-name denominator
            // disappears, the wire says `counted_from: master`, and the five
            // index underlyings that legitimately have no ISIN get reported as
            // "this feed's master lists no id for it" — blaming the vendor for
            // a cell NSE never filled.
            Self::Fno => Some(Tier::FnoUnderlyings),
            Self::Nifty500 => Some(Tier::Nifty500),
            Self::Nifty200 => Some(Tier::Nifty200),
            Self::Nifty100 => Some(Tier::Nifty100),
            Self::Nifty50 => Some(Tier::Nifty50),
        }
    }

    /// The target a form field names.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.slug() == slug)
    }
}

/// Every legal spot-target slug, comma-separated, for a refusal to name.
///
/// **Generated from [`SpotTarget::ALL`], never written out.** The sentence this
/// replaces read *"is not one of the three spot targets"* and there are seven —
/// a message that counts by hand is a message that is wrong one commit after
/// somebody appends, and this one is the operator's only list of what the
/// `target` field takes. `CLAUDE.md` §4: refuse by name, and the names have to
/// be the names the parser actually accepts.
///
/// Cost: one pass over a compile-time array of seven `&'static str`, paid only
/// on the refusal path.
fn target_slugs() -> String {
    let mut out = String::new();
    for (n, target) in SpotTarget::ALL.into_iter().enumerate() {
        if n > 0 {
            out.push_str(", ");
        }
        out.push_str(target.slug());
    }
    out
}

/// Which derivative series an F&O request is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Series {
    /// The future on that expiry.
    Futures,
    /// Every option strike on that expiry.
    Options,
}

impl Series {
    /// Both series, in the order the form lists them.
    pub const ALL: [Self; 2] = [Self::Futures, Self::Options];

    /// The value this series carries on the wire.
    #[must_use]
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Futures => "fut",
            Self::Options => "opt",
        }
    }

    /// What the form calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Futures => "Future",
            Self::Options => "Option chain",
        }
    }

    /// The series a form field names.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.slug() == slug)
    }
}

/// Why a request was not accepted.
///
/// Every variant carries the value it refused. `CLAUDE.md` §4: degrade loudly
/// and name the reason. A form that answered "invalid input" would send an
/// operator back to guess which of five fields it meant.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Refusal {
    /// A field the form always sends arrived empty or not at all.
    FieldMissing {
        /// Which field.
        field: &'static str,
    },
    /// A date field is not ten characters of `YYYY-MM-DD`.
    DateNotIso {
        /// Which field.
        field: &'static str,
        /// What arrived.
        got: String,
    },
    /// A date field is well shaped and names no day that exists.
    DateImpossible {
        /// Which field.
        field: &'static str,
        /// What arrived.
        got: String,
        /// The calendar's own refusal.
        why: SessionError,
    },
    /// The window's end is before its start.
    ///
    /// Refused rather than silently swapped, for the reason
    /// [`pull::session::SessionError::WindowRunsBackwards`] gives: a pull that
    /// quietly reversed them would fetch a range nobody asked for and report
    /// success.
    WindowBackwards {
        /// The calendar's own refusal, which names both ends.
        why: SessionError,
    },
    /// The window is longer than [`MAX_WINDOW_DAYS`].
    WindowTooLong {
        /// How many days were asked for.
        days: u32,
        /// The bound.
        cap: u32,
    },
    /// The window ends after today. No vendor holds a bar that has not traded.
    ///
    /// This variant exists because the spot form had only the HTML `max`
    /// attribute stopping it, while the F&O form one panel over carried a real
    /// parser gate and the sentence *"an attribute is a courtesy, a parser is a
    /// rule"*. The two disagreed, and the spot side was the courtesy. An
    /// attribute is absent from a `curl`, from a replayed POST, and from any
    /// client that is not the browser the page was rendered for.
    ///
    /// `api::unit::a_window_ending_after_today_is_refused_by_the_parser` is
    /// what holds it up, and it posts a body rather than inspecting the HTML —
    /// the previous test asserted the attribute *string was present*, which is
    /// a different claim and was the reason this went unnoticed.
    WindowInFuture {
        /// The end that has not happened.
        to: Day,
        /// Today in IST, as the clock reported it.
        today: Day,
    },
    /// The window reaches today, whose session may not be finished.
    ///
    /// Distinct from a window in the FUTURE: this one names a day that exists,
    /// and refuses it because a running session yields a partial day the
    /// append-only store cannot later correct. See [`parse_window`].
    WindowReachesToday {
        /// The end that is too recent.
        to: Day,
        /// Today in IST, as the caller reported it.
        today: Day,
    },
    /// An archive feed was asked for with no folder to read.
    ///
    /// `TrueData` and `GDFL` are not endpoints — they are CSV files an operator has
    /// bought and put somewhere. Asking one for a window without saying where
    /// the files are is a request with no subject, and there is nothing to fall
    /// back to: refused by name rather than quietly routed to a broker, which
    /// is what the old folder-emptiness fork did.
    ArchiveFolderMissing {
        /// The feed that was named, for the message.
        feed: &'static str,
    },
    /// The spot form's target is not one of [`SpotTarget::ALL`].
    UnknownTarget {
        /// What arrived.
        got: String,
    },
    /// The F&O form's series is not one of [`Series::ALL`].
    UnknownSeries {
        /// What arrived.
        got: String,
    },
    /// A vendor was named, and it is not one this build has a feed for.
    ///
    /// Distinct from naming none at all, which still defaults — see
    /// [`parse_vendor`] for why that half of the old behaviour is kept.
    UnknownVendor {
        /// What arrived.
        got: String,
    },
    /// A bar length was named, and it is not a rung of the ladder.
    ///
    /// The same rule as [`Self::UnknownVendor`] and for the same reason: absent
    /// defaults to `1min`, present-but-unknown refuses by name. A coerced rung
    /// files bars under a directory the operator did not ask for, and the bar
    /// length lives in the path.
    UnknownGranularity {
        /// What arrived.
        got: String,
    },
    /// The underlying is not a symbol this engine can even name.
    BadUnderlying {
        /// What arrived.
        got: String,
    },
    /// The underlying is a legal symbol with no F&O series on it.
    NotAnFnoUnderlying {
        /// The symbol, canonicalised.
        symbol: String,
    },
    /// **The expiry has not passed.** A live contract is never stored.
    LiveContract {
        /// The expiry that was asked for.
        expiry: Day,
        /// The day it was compared against.
        today: Day,
    },
    /// The window runs past the day the contract stopped trading.
    WindowOutlivesTheContract {
        /// The last day asked for.
        to: Day,
        /// The expiry.
        expiry: Day,
    },
    /// The machine's clock names no date this build can render.
    ///
    /// Not a fall-back to a guessed date: an expiry gate that compared against
    /// an invented "today" would be a gate that passes for the wrong reason.
    ClockUnusable {
        /// The calendar's own refusal.
        why: SessionError,
    },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::FieldMissing { field } => {
                write!(f, "REFUSED · {field} was not filled in")
            }
            Self::DateNotIso { field, ref got } => write!(
                f,
                "REFUSED · {field} {got:?} is not a date — it must be ten \
                 characters of YYYY-MM-DD, which is what the vendors take"
            ),
            Self::DateImpossible {
                field,
                ref got,
                why,
            } => write!(f, "REFUSED · {field} {got:?} is not a day: {why}"),
            Self::WindowBackwards { why } => write!(f, "REFUSED · {why}"),
            Self::WindowTooLong { days, cap } => write!(
                f,
                "REFUSED · the window is {days} days; this build fetches at most {cap}"
            ),
            Self::WindowInFuture { to, today } => write!(
                f,
                "REFUSED · the window ends {to}, which is after {today}. No \
                 vendor holds a bar that has not traded yet. An attribute is a \
                 courtesy, a parser is a rule."
            ),
            Self::WindowReachesToday { to, today } => write!(
                f,
                "REFUSED · the window ends {to} and today is {today}. A session \
                 that is still running yields a partial day, and this store is \
                 append-only — a half day cannot be corrected later, only \
                 refused or left as a gap. Ask for {} or earlier.",
                // The day before today, through the calendar that owns the
                // arithmetic rather than by subtracting one from a field.
                Day::from_days(today.days_from_epoch().saturating_sub(1))
                    .map_or_else(|_| "an earlier day".to_owned(), |d| d.to_string())
            ),
            Self::ArchiveFolderMissing { feed } => write!(
                f,
                "REFUSED · {feed} reads CSV files from a folder you have already \
                 bought — there is no endpoint to call and no credential to \
                 read. Name the folder to pull from it."
            ),
            Self::UnknownTarget { ref got } => write!(
                f,
                "REFUSED · {got:?} is not a spot target. This build takes {}",
                target_slugs()
            ),
            Self::UnknownSeries { ref got } => {
                write!(f, "REFUSED · {got:?} is neither a future nor an option")
            }
            Self::UnknownVendor { ref got } => write!(
                f,
                "REFUSED · {got:?} is not a vendor this build has a feed for. \
                 Nothing was pulled rather than another vendor's bars being \
                 filed under a prefix you did not ask for"
            ),
            Self::UnknownGranularity { ref got } => write!(
                f,
                "REFUSED · {got:?} is not a bar length on this ladder. Nothing \
                 was pulled rather than bars being filed under a rung you did \
                 not ask for — the bar length is the DIRECTORY here, and no \
                 reader can tell a substituted bar from a real one. This build \
                 stores {}.",
                store::path::Timeframe::KNOWN
                    .iter()
                    .map(|tf| tf.as_str())
                    .collect::<Vec<_>>()
                    .join(" and ")
            ),
            Self::BadUnderlying { ref got } => {
                write!(f, "REFUSED · {got:?} is not a symbol this engine can name")
            }
            Self::NotAnFnoUnderlying { ref symbol } => write!(
                f,
                "REFUSED · {symbol} carries no F&O series, so there is no \
                 expired contract on it to store"
            ),
            Self::LiveContract { expiry, today } => write!(
                f,
                "REFUSED · {expiry} has not expired as of {today}. A LIVE \
                 CONTRACT IS NEVER STORED — expired series only"
            ),
            Self::WindowOutlivesTheContract { to, expiry } => write!(
                f,
                "REFUSED · the window ends {to}, after the contract expired on \
                 {expiry}; there are no bars there to fetch"
            ),
            Self::ClockUnusable { why } => write!(
                f,
                "REFUSED · this machine's clock names no usable date ({why}), \
                 so no expiry can be checked against it"
            ),
        }
    }
}

impl std::error::Error for Refusal {}

/// One spot pull, validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpotRequest {
    /// Which set of instruments.
    pub target: SpotTarget,
    /// The operator's inclusive range.
    pub window: Window,
    /// Which feed to ask — broker or archive.
    ///
    /// # Why this is a `Feed` and not a `Vendor`
    ///
    /// `brutex_core::vendor::Vendor` names the two HTTP BROKERS. `Feed` names
    /// every source this build can read, brokers and local CSV archives alike.
    /// The field used to be the former, and the consequence was that the
    /// archive vendors were not selectable at all: the route decided HTTP
    /// versus archive by asking whether a `folder` textbox was blank, so a
    /// blank box meant "broker" and a filled one meant "archive" no matter
    /// which vendor the operator had actually picked.
    ///
    /// That made the transport a property of the FORM rather than of the FEED,
    /// and it is exactly the coupling `CLAUDE.md` says a new vendor must not
    /// require: with it, adding a fifth feed meant teaching a textbox about it.
    /// With a `Feed` here, the descriptor's own `transport` picks the path and
    /// a new row is selectable and correctly routed the day it is added.
    ///
    /// Parsed rather than hardcoded, because `CLAUDE.md` makes adding a vendor
    /// a row in `pull::vendor` — and a route that names one defeats that. An
    /// unstated or unknown vendor is **Dhan**, which is the one whose
    /// descriptor has been verified against a live body; Groww's has not, and
    /// defaulting to the unverified one would make a first-time operator debug
    /// a shape nobody has confirmed.
    pub feed: pull::vendor::Feed,
    /// Which rung of the ladder to file under — `1min` or `1day`.
    ///
    /// # Why a `Granularity` and not a `Timeframe`
    ///
    /// The operator picks a **rung**; the store's directory is what a rung
    /// converts INTO, and that conversion has exactly one site —
    /// [`pull::vendor::Granularity::store_timeframe`] — whose `None` is a
    /// refusal at the write boundary rather than a substitution. Carrying the
    /// timeframe here would move the conversion up to the parser, where a rung
    /// the store cannot file has nothing to refuse with except a guess.
    ///
    /// # Why it defaults rather than being required
    ///
    /// Absent means `1min`, which is what every request meant before this field
    /// existed — `/pull/spot` is a POST an operator can replay, and a body
    /// written last week must not silently change rung. The form always sends
    /// it, so the default is reached only by a hand-built request. Naming an
    /// unknown one still refuses: see [`parse_granularity`].
    pub granularity: pull::vendor::Granularity,
}

/// One expired-derivative pull, validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FnoRequest {
    /// The underlying, canonicalised.
    pub underlying: Symbol,
    /// Future or option chain.
    pub series: Series,
    /// The expiry, which [`parse_fno`] has already proved is behind `today`.
    pub expiry: Day,
    /// The operator's inclusive range.
    pub window: Window,
}

/// One `YYYY-MM-DD` form field, as a validated day.
///
/// Strict on shape before it is strict on value: `2024-2-9` is refused as
/// malformed rather than accepted as February the 9th, because a parser that
/// tolerates a missing digit is a parser that will one day tolerate a missing
/// field.
///
/// # Errors
///
/// [`Refusal::FieldMissing`], [`Refusal::DateNotIso`], or
/// [`Refusal::DateImpossible`] carrying whatever [`Day::new`] refused.
pub fn parse_day(field: &'static str, text: &str) -> Result<Day, Refusal> {
    if text.is_empty() {
        return Err(Refusal::FieldMissing { field });
    }
    let bad = || Refusal::DateNotIso {
        field,
        got: text.to_owned(),
    };
    // Split on the separators the shape declares, then require each piece to be
    // exactly as wide as the shape says. `splitn` alone would accept
    // `2024-2-9`; the width checks are what make the format the format.
    let mut parts = text.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(bad());
    };
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return Err(bad());
    }
    let (Ok(year), Ok(month), Ok(day)) = (y.parse::<u16>(), m.parse::<u8>(), d.parse::<u8>())
    else {
        return Err(bad());
    };
    Day::new(year, month, day).map_err(|why| Refusal::DateImpossible {
        field,
        got: text.to_owned(),
        why,
    })
}

/// One date field, however the client chose to send it.
///
/// **Two spellings, one parser.** The calendar in [`crate::calendar`] posts three
/// integers — `{field}_y`, `{field}_m`, `{field}_d` — because that is what keeps
/// a no-script picker down to 57 controls instead of 4,464. A hand-built POST,
/// `curl`, and every test written before the picker existed send the whole
/// `{field}=YYYY-MM-DD`. Both arrive here.
///
/// The triple is **composed into the ISO string and handed to [`parse_day`]**
/// rather than validated separately. That is the point: one definition of what a
/// date is, so the picker cannot be accepted by rules the wire form is not, and
/// `Day::new` still owns the leap year. Zero-padding on the way in also means a
/// client that posts `m=8` and one that posts `m=08` get the same answer.
///
/// ISO wins when both are present — it is the explicit form, and the picker
/// never sends it.
///
/// # Errors
///
/// [`Refusal::FieldMissing`] when neither spelling is present, and when the
/// triple is there but incomplete — a piece absent and a piece present-but-empty
/// are the same unfinished date. Otherwise whatever [`parse_day`] refuses.
pub fn parse_day_field(body: &str, field: &'static str) -> Result<Day, Refusal> {
    let iso = param(body, field);
    if !iso.is_empty() {
        return parse_day(field, &iso);
    }
    let (y, m, d) = (
        param(body, &format!("{field}_y")),
        param(body, &format!("{field}_m")),
        param(body, &format!("{field}_d")),
    );
    // ANY PIECE MISSING IS A DATE THAT WAS NOT FINISHED, NOT A MALFORMED ONE.
    //
    // This used to require all three to be empty before saying so, and compose
    // whatever was left otherwise — so a triple with the month cleared became
    // the string `-08-06`, which came back as `DateNotIso` naming a format the
    // operator never typed.
    //
    // The picker made that reachable by design. Its drill-down header steps
    // back a pane by checking a radio whose value is the empty string (see
    // `crate::calendar`), because checking a sibling is the only way a
    // `<label>` can uncheck a radio — so a half-picked date now arrives with a
    // piece present and EMPTY rather than absent. Both spellings mean the same
    // thing and both belong in the same refusal.
    if y.is_empty() || m.is_empty() || d.is_empty() {
        return Err(Refusal::FieldMissing { field });
    }
    // Padded so `8` and `08` mean the same August, then parsed by the one
    // parser. A piece that is not a number stays un-padded and fails the width
    // check inside `parse_day`, which is where a malformed date belongs.
    let pad = |text: &str, width: usize| -> String {
        text.parse::<u16>()
            .map_or_else(|_| text.to_owned(), |n| format!("{n:0width$}"))
    };
    parse_day(
        field,
        &format!("{}-{}-{}", pad(&y, 4), pad(&m, 2), pad(&d, 2)),
    )
}

/// The operator's inclusive window, from the two date fields both forms carry.
///
/// # Errors
///
/// Whatever [`parse_day_field`] refuses, [`Refusal::WindowBackwards`], or
/// [`Refusal::WindowTooLong`].
pub fn parse_window(body: &str, today: Day) -> Result<Window, Refusal> {
    let from = parse_day_field(body, "from")?;
    let to = parse_day_field(body, "to")?;
    let window = Window::new(from, to).map_err(|why| Refusal::WindowBackwards { why })?;
    let days = window.days();
    if days > MAX_WINDOW_DAYS {
        return Err(Refusal::WindowTooLong {
            days,
            cap: MAX_WINDOW_DAYS,
        });
    }
    // A WINDOW IN THE FUTURE IS REFUSED HERE, AND A WINDOW REACHING TODAY IS
    // NOT — because this parser is shared.
    //
    // Both ingest paths come through here: the BROKER path, which asks a vendor
    // over HTTP, and the ARCHIVE path, which reads CSV files an operator has
    // already bought and dropped in a folder. A partial session is a hazard of
    // the first and meaningless to the second — a file on disk is whatever it
    // is, and its last day is not a question about the clock.
    //
    // So "not today" belongs on the broker path, beside the other rules that
    // are about talking to a vendor rather than about reading a file: the rate
    // budget, the credential, the market-hours window. See `broker_window`.
    //
    // `today` is an argument rather than a clock call inside, for the same
    // reason `parse_fno` takes it: a gate that reads the clock cannot be tested
    // at its own boundary without waiting for midnight, and an invented "today"
    // would be a gate that passes for the wrong reason.
    if to > today {
        return Err(Refusal::WindowInFuture { to, today });
    }
    Ok(window)
}

/// Which form control a refusal is about, when it is about one.
///
/// The refusal's own text already says what went wrong; this says **where the
/// operator has to go to fix it**, as one token a `grep` can count. "Which
/// field is my form failing on" and "which sentence did it print" are different
/// questions, and only the first survives a message being reworded.
///
/// The match is exhaustive with no wildcard arm on purpose. [`Refusal`] is
/// `#[non_exhaustive]` to the world and total inside this crate, so a variant
/// added later stops the build here rather than quietly logging a refusal with
/// no control named — which is exactly the silent drift `CLAUDE.md` §4 bans.
///
/// [`Refusal::ClockUnusable`] is the one `None`: nothing the operator typed is
/// wrong, the machine's clock is, and naming a field would send them to edit a
/// date that is already fine.
fn refused_field(why: &Refusal) -> Option<&'static str> {
    match *why {
        Refusal::FieldMissing { field }
        | Refusal::DateNotIso { field, .. }
        | Refusal::DateImpossible { field, .. } => Some(field),
        // The pair is wrong, not either end of it. Naming one would be a guess
        // about which of the two the operator meant to change.
        Refusal::WindowBackwards { .. } | Refusal::WindowTooLong { .. } => Some("window"),
        // These three are about the END, and only the end: the start is legal
        // in every one of them.
        Refusal::WindowInFuture { .. }
        | Refusal::WindowReachesToday { .. }
        | Refusal::WindowOutlivesTheContract { .. } => Some("to"),
        Refusal::ArchiveFolderMissing { .. } => Some("folder"),
        Refusal::UnknownTarget { .. } => Some("target"),
        Refusal::UnknownSeries { .. } => Some("series"),
        Refusal::UnknownVendor { .. } => Some("vendor"),
        Refusal::UnknownGranularity { .. } => Some("granularity"),
        Refusal::BadUnderlying { .. } | Refusal::NotAnFnoUnderlying { .. } => Some("underlying"),
        Refusal::LiveContract { .. } => Some("expiry"),
        Refusal::ClockUnusable { .. } => None,
    }
}

/// One refused submission, on the one surface that outlives the browser tab.
///
/// # What was invisible before this
///
/// A refused form got an HTTP 400 and a page of prose, and that was the whole
/// record. The audit journal takes the run's refusals, but an operator
/// debugging a form that **never starts a run** is reading a page that is gone
/// the moment the tab is closed — and a `curl` or a replayed POST never renders
/// it at all. A question as ordinary as *why did nothing happen when I pressed
/// Pull* had no answer after the fact. Now it does: the form, the control, and
/// the refusal's own sentence, timestamped beside the pull events they belong
/// with.
///
/// # Why `Warn` and why one per submission
///
/// `Warn` — the request continued as far as an answer and someone should know
/// it was turned away — sits above the default `Info` floor, so a refusal is
/// visible with no configuration. It is bounded by SUBMISSIONS, not by data:
/// one event per press of the button, emitted once at the form boundary rather
/// than at each of the nested parsers a body passes through, so a refused date
/// inside a spot form is one line and not three. The accept path emits nothing
/// here; `pull.run started` already carries it.
///
/// # Why no secret can reach this
///
/// The fields are a form body's own values — dates, a target slug, a vendor
/// name, a symbol, a folder. `CLAUDE.md` §8 keeps credentials out of this
/// module entirely: a token is read from Parameter Store inside `crates/pull`
/// and never travels through a form. The quoted-back value is bounded twice
/// over — the server caps the body it will read, and the sink cuts a string
/// value at [`telemetry::MAX_STR_VALUE_BYTES`] and says it did.
///
/// The result is discarded under the name the workspace uses for it: a
/// level-filtered event legitimately reaches no file, so asserting it was
/// written would fire on a clean run.
pub(crate) fn note_refused(form: &str, why: &Refusal) {
    let text = why.to_string();
    let event = telemetry::Event::warn("api.ingest", "form refused")
        .with("form", telemetry::Value::Str(form))
        .with("why", telemetry::Value::Str(&text));
    // A refusal that is about one control names it as its own key; one that is
    // about the machine's clock omits the key rather than inventing a control
    // that is not at fault. An absent key and a key reading "-" are different
    // facts and only the first is honest — the same rule as `Int(-1)` for an
    // absent number.
    let event = match refused_field(why) {
        Some(field) => event.with("field", telemetry::Value::Str(field)),
        None => event,
    };
    let _dropped_when_filtered = telemetry::emit(&event);
}

/// One spot request, out of a form body.
///
/// # Errors
///
/// [`Refusal::UnknownTarget`], or whatever [`parse_window`] refuses.
pub fn parse_spot(body: &str, today: Day) -> Result<SpotRequest, Refusal> {
    parse_spot_inner(body, today).inspect_err(|why| note_refused("spot", why))
}

/// [`parse_spot`]'s body, split so the refusal is noted at exactly one place.
///
/// Every early return in here is a refusal, and each one used to need its own
/// emit to be seen. Wrapping the whole parse instead means one event per
/// submission by construction — a rule that cannot be broken by adding a
/// fifteenth `?`.
fn parse_spot_inner(body: &str, today: Day) -> Result<SpotRequest, Refusal> {
    let raw = param(body, "target");
    if raw.is_empty() {
        return Err(Refusal::FieldMissing { field: "target" });
    }
    let target = SpotTarget::from_slug(&raw).ok_or(Refusal::UnknownTarget { got: raw })?;
    Ok(SpotRequest {
        target,
        window: parse_window(body, today)?,
        feed: {
            let raw = param(body, "vendor");
            parse_feed(&raw).ok_or(Refusal::UnknownVendor { got: raw })?
        },
        granularity: {
            let raw = param(body, "granularity");
            parse_granularity(&raw).ok_or(Refusal::UnknownGranularity { got: raw })?
        },
    })
}

/// Which rung a request names: `None` when it named one this ladder has not.
///
/// The same rule as [`parse_feed`] — absent defaults, present-but-unknown
/// refuses by name — and matched on the rung's own **directory name**, which is
/// the value the form puts on the wire and the segment the store writes. One
/// spelling for the control, the request and the path, so a rung added to
/// `pull::vendor::Granularity` is parseable here with no edit to this function.
///
/// Absent is `Minute1` because that is what every request meant before the
/// field existed. `CLAUDE.md` §3 rule 5 makes reruns safe, and a replayed body
/// that silently changed rung would file the same window twice under two
/// directories.
///
/// **Whether the store can carry the rung is not asked here.** That question is
/// answered once, at the write boundary, by
/// [`pull::vendor::Granularity::store_timeframe`] — see `pull::ingest::Plan`.
/// Asking it twice would be two answers to "where does this bar go", and the
/// parser's copy is the one that would drift.
#[must_use]
pub fn parse_granularity(raw: &str) -> Option<pull::vendor::Granularity> {
    if raw.is_empty() {
        return Some(pull::vendor::Granularity::Minute1);
    }
    pull::vendor::Granularity::ALL
        .into_iter()
        .find(|rung| rung.dir().eq_ignore_ascii_case(raw))
}

/// Which broker a request names: `None` when it named one this build cannot serve.
///
/// # An UNNAMED vendor still defaults; a WRONGLY named one no longer does
///
/// The default when the field is absent is **Dhan**, because its descriptor is
/// the one verified against a live body — defaulting to the unverified one
/// would make a first-time operator debug a response shape nobody has
/// confirmed. That half of the original reasoning is sound and is kept.
///
/// The other half is not, and it was not wrong when it was written. It said:
///
/// > The vendor is a *route*, not a claim about the data: naming one that does
/// > not exist cannot corrupt anything.
///
/// That was true while the vendor only chose which host to call. It stopped
/// being true when `Plan.vendor` became the STORE PREFIX: bars land under
/// `bars/<vendor>/…`, so coercing an unrecognised name to Dhan files one
/// broker's prices under another's path — which `pull::vendor::Feed::store_vendor`
/// documents as destroying D-0019's per-vendor independence irreversibly. The
/// justification quietly outlived the fact it rested on.
///
/// The form offering a fixed list was the other prop under that argument, and
/// `/bars` knocks it out: it is a GET whose vendor comes from a hand-typed
/// query string with no list at all.
///
/// So: absent means Dhan and says so; present-but-unknown refuses by name.
#[must_use]
pub fn parse_vendor(raw: &str) -> Option<brutex_core::vendor::Vendor> {
    if raw.is_empty() {
        return Some(brutex_core::vendor::Vendor::Dhan);
    }
    brutex_core::vendor::Vendor::ALL
        .into_iter()
        .find(|v| v.as_str().eq_ignore_ascii_case(raw))
}

/// Which feed a request names: `None` when it named one this build cannot read.
///
/// The same rule as [`parse_vendor`] — absent means Dhan, present-but-unknown
/// refuses by name — over the WIDER set. `Feed::ALL` carries the archive
/// vendors too, so `TrueData` and `GDFL` become selectable here rather than being
/// reachable only as a side effect of typing into a `folder` box.
///
/// A feed is matched on its `wire` name, which lives in its descriptor row, so
/// a feed added to [`pull::vendor::DESCRIPTORS`] is parseable here with no edit
/// to this function.
#[must_use]
pub fn parse_feed(raw: &str) -> Option<pull::vendor::Feed> {
    if raw.is_empty() {
        return Some(pull::vendor::Feed::Dhan);
    }
    pull::vendor::Feed::ALL
        .into_iter()
        .find(|f| f.wire().eq_ignore_ascii_case(raw) || f.display().eq_ignore_ascii_case(raw))
}

/// One expired-derivative request, out of a form body.
///
/// `today` is an argument rather than a call to the clock inside, for the
/// reason `CLAUDE.md` §3 rule 5 gives about reruns: an expiry gate whose answer
/// depends on when it ran is a gate no test can pin. The route supplies the
/// real day; a test supplies the day it means.
///
/// # Errors
///
/// [`Refusal::BadUnderlying`], [`Refusal::NotAnFnoUnderlying`],
/// [`Refusal::UnknownSeries`], [`Refusal::LiveContract`],
/// [`Refusal::WindowOutlivesTheContract`], or whatever [`parse_window`]
/// refuses.
pub fn parse_fno(body: &str, today: Day) -> Result<FnoRequest, Refusal> {
    parse_fno_inner(body, today).inspect_err(|why| note_refused("fno", why))
}

/// [`parse_fno`]'s body, split for the reason [`parse_spot_inner`] is.
fn parse_fno_inner(body: &str, today: Day) -> Result<FnoRequest, Refusal> {
    let typed = param(body, "underlying");
    if typed.is_empty() {
        return Err(Refusal::FieldMissing {
            field: "underlying",
        });
    }
    // Upper-cased before it is validated: an operator types `nifty` and means
    // NIFTY, and `Symbol` is canonical upper case. Refusing the typing rather
    // than the symbol would be a refusal about nothing.
    let upper = typed.to_uppercase();
    let underlying = Symbol::new(&upper).map_err(|_| Refusal::BadUnderlying { got: typed })?;
    // One probe of a compile-time membership table. An underlying with no F&O
    // series has no expired contract to store, so this is a refusal rather than
    // an empty pull that looks like a vendor outage.
    if !universe::of_equity(underlying.as_str()).contains(Universe::FNO) {
        return Err(Refusal::NotAnFnoUnderlying {
            symbol: underlying.as_str().to_owned(),
        });
    }

    let raw = param(body, "series");
    if raw.is_empty() {
        return Err(Refusal::FieldMissing { field: "series" });
    }
    let series = Series::from_slug(&raw).ok_or(Refusal::UnknownSeries { got: raw })?;

    let expiry = parse_day_field(body, "expiry")?;
    // THE GATE. Strictly behind today: a contract expiring today is still
    // trading today, and `CLAUDE.md` §8's argument about a fall-back applies
    // here too — `<=` would admit exactly the case the rule exists to exclude.
    if expiry >= today {
        return Err(Refusal::LiveContract { expiry, today });
    }

    let window = parse_window(body, today)?;
    if window.to() > expiry {
        return Err(Refusal::WindowOutlivesTheContract {
            to: window.to(),
            expiry,
        });
    }
    Ok(FnoRequest {
        underlying,
        series,
        expiry,
        window,
    })
}

/// Epoch seconds of a moment, in either direction.
///
/// Split from [`today_ist`] so both directions are testable without waiting for
/// 1969: a `SystemTime` before the epoch is a value a test constructs, not a
/// machine state it has to arrange.
///
/// Public because [`crate::audit`] stamps the same number into every record it
/// writes, and two spellings of "which second is it" is exactly the second
/// definition `CLAUDE.md` §3 rule 1 is about.
#[must_use]
pub fn epoch_secs(at: SystemTime) -> i64 {
    match at.duration_since(SystemTime::UNIX_EPOCH) {
        // The saturating arm lives inside `Result::unwrap_or`, which is not
        // this repository's code to measure; it is reachable only from a clock
        // 292 billion years out, and saturating there keeps the refusal below
        // honest rather than wrapping into a plausible date.
        Ok(ahead) => i64::try_from(ahead.as_secs()).unwrap_or(i64::MAX),
        Err(behind) => {
            i64::try_from(behind.duration().as_secs()).map_or(i64::MIN, i64::saturating_neg)
        }
    }
}

/// The IST date of a moment.
///
/// # Errors
///
/// [`Refusal::ClockUnusable`] for a clock before 1970 or past 9999. There is no
/// default: an expiry compared against a guessed day is a gate that passes for
/// the wrong reason.
pub fn ist_day(at: SystemTime) -> Result<Day, Refusal> {
    IstMoment::from_epoch_secs(epoch_secs(at))
        .map(IstMoment::day)
        .map_err(|why| Refusal::ClockUnusable { why })
}

/// Today, in IST, from this machine's clock.
///
/// # Errors
///
/// Whatever [`ist_day`] refuses.
pub fn today_ist() -> Result<Day, Refusal> {
    ist_day(SystemTime::now())
}

// ---------------------------------------------------------------------------
// THE TWO INGEST ROUTES THAT ANSWER RATHER THAN RUN
//
// Everything above this line is a parser. The two handlers below are here
// rather than in `server.rs` because both are *about* a request's admission —
// one says what the backfill is waiting on, the other says whether a selection
// would be accepted — and admission is what this module already owns. Neither
// opens a socket, reads the store, or writes a byte.
// ---------------------------------------------------------------------------

/// The JSON content type both routes below answer with.
fn json_headers() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

/// `GET /ingest/status.json` — what the next sweep is waiting on.
///
/// # It reports the backfill's own last survey, and never re-derives one
///
/// The honest answer to "what is the next sweep waiting on" is already computed
/// once per pass by `autopilot::survey` and published to
/// [`crate::autopilot::Status`]. This projects that, and it deliberately does
/// **not** call `census::read_all` or probe a manifest: this is a route a page
/// polls, and re-reading every manifest per request is the O(entries) cost
/// D-0039 exists to remove — the same reason `/autopilot.json` is served from
/// memory. `CLAUDE.md` §3 rule 4.
///
/// The price of that is a real staleness, and it is **stated rather than
/// hidden**: `surveyed` is false until a round finishes, and `blocked_by` then
/// says so in words and names `/store.json` as the route that does read the
/// store. An answer of "nothing is waiting" from a backfill that has never
/// looked would be the §4 fallback.
///
/// # Cost
///
/// One uncontended lock and one pass over the feed reports — two rows on this
/// build. No I/O.
pub async fn status_json(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let paused = site.autopilot.is_paused();
    let seat = site.autopilot.seat_held();
    site.autopilot
        .inspect(|status| {
            (
                axum::http::StatusCode::OK,
                json_headers(),
                waiting_json(status, paused, seat),
            )
        })
        .unwrap_or_else(|| {
            (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                json_headers(),
                format!(
                    r#"{{"surveyed":false,"error":{}}}"#,
                    crate::render::json_string(
                        "the autopilot's status lock is poisoned: a previous publisher \
                         panicked, so what the next sweep is waiting on cannot be read. \
                         Nothing is guessed here — an empty list would read as \"nothing \
                         is outstanding\", which is a different fact. Restart the server."
                    )
                ),
            )
        })
}

/// The payload, over a status a caller supplies.
///
/// Split from the handler so every branch of [`blocked_by`] and every shape of
/// the feed list is drivable from a test without a `Site`, a store or a
/// background task.
fn waiting_json(status: &crate::autopilot::Status, paused: bool, seat: bool) -> String {
    use std::fmt::Write as _;
    let js = crate::render::json_string;
    let mut out = String::with_capacity(1024);
    let _ = write!(
        out,
        r#"{{"surveyed":{},"state":{},"paused":{},"pull_seat":{},"blocked_by":{},"why":{},"cursor":{},"since_unix":{},"due_unix":{},"journal":{},"target":{{"from":{},"to":{},"instruments":{},"timeframe":{},"feed":{}}},"in_flight":"#,
        !status.feeds.is_empty(),
        js(status.state()),
        paused,
        // A FACT, NOT A VERDICT. `autopilot::round` holds the seat for the whole
        // of every pass, so a held seat is the backfill's own tick as often as
        // it is a hand-made pull. Which one is standing off, and why, is
        // `blocked_by`'s job and not this field's.
        js(if seat { "held" } else { "free" }),
        js(&blocked_by(status, paused)),
        js(&status.why()),
        js(&status.cursor),
        status.since_unix,
        status.due_unix,
        js(&status.journal),
        js(&status.target.from),
        js(&status.target.to),
        status.target.instruments,
        js(&status.target.timeframe),
        js(&status.target.feed),
    );
    match status.now {
        None => out.push_str("null"),
        Some(ref now) => {
            let _ = write!(
                out,
                r#"{{"instrument":{},"month":{},"index":{},"of":{}}}"#,
                js(&now.instrument),
                js(&now.month),
                now.index,
                now.of
            );
        }
    }
    out.push_str(r#","waiting_on":["#);
    for (n, feed) in status.feeds.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            r#"{{"feed":{},"vendor":{},"month":{},"window":{},"behind":{},"done":{},"attempts":{},"attempts_max":{},"months_done":{},"months_total":{},"stalled_months":{},"last_reason":{},"halted":{}}}"#,
            js(&feed.feed),
            js(&feed.vendor),
            js(&feed.month),
            js(&feed.window),
            feed.behind,
            feed.done,
            feed.attempts,
            crate::autopilot::MAX_MONTH_ATTEMPTS,
            feed.months_done,
            feed.months_total,
            feed.stalls.len(),
            js(&feed.last_reason),
            js(&feed.halted),
        );
    }
    out.push_str("]}");
    out
}

/// The one sentence that answers the question the route is named for.
///
/// Four cases, in the order they outrank one another, and every one of them is
/// read from a published field rather than inferred:
///
/// 1. **The operator's pause** outranks everything, because it is the only
///    state where nothing is being attempted by choice.
/// 2. **Every feed halted** — a restart is what clears one, so this names it.
/// 3. **Nothing surveyed** — either the task stopped before its first round
///    (phase `halted`), or no round has finished yet. Neither is "nothing is
///    outstanding", and neither is answered with an empty list alone.
/// 4. Otherwise the backfill's own sentence, which is written on every
///    transition and is never empty — see [`crate::autopilot::Status::why`].
fn blocked_by(status: &crate::autopilot::Status, paused: bool) -> String {
    use crate::autopilot::Phase;
    if paused {
        return format!(
            "the operator's pause. Nothing is asked of any vendor until a resume, and \
             nothing is lost by waiting — the resume point is the store's own. What the \
             backfill last said of itself: {}",
            status.why()
        );
    }
    let halted: Vec<&str> = status
        .feeds
        .iter()
        .filter(|feed| !feed.halted.is_empty())
        .map(|feed| feed.feed.as_str())
        .collect();
    if !status.feeds.is_empty() && halted.len() == status.feeds.len() {
        return format!(
            "every feed is halted and a resume does not clear a halt — restarting the \
             server is what does. The reason is on each row below, verbatim. Halted: {}",
            halted.join(", ")
        );
    }
    if status.feeds.is_empty() {
        if status.phase == Phase::Halted {
            return format!(
                "nothing, because the backfill task is not running: {}",
                status.why()
            );
        }
        return format!(
            "not known here yet. No round has finished in this process, so there is no \
             survey to report — and this route never reads the store, deliberately, so it \
             cannot answer from the store either. /store.json is the route that does. What \
             the backfill says of itself: {}",
            status.why()
        );
    }
    status.why()
}

/// `POST /ingest/queue` — **nothing is queued, and this says why.**
///
/// # What this route does
///
/// It takes the same body `/pull/spot` takes, validates it in full through
/// [`parse_spot`], and answers. A selection with a bad field is refused by that
/// field's name. A selection that is legal is echoed back with the exact dates
/// that would go on the wire, and then refused too — because **there is no
/// queue in this build to put it in**, and accepting it would be accepting and
/// dropping it. `CLAUDE.md` §4: degrade loudly and name the reason, or refuse.
/// Never both silently.
///
/// # Why there is no queue, stated in terms a caller can check
///
/// There is exactly one pull seat — [`crate::autopilot::Control::take_seat`], a
/// compare-exchange — and `pull::ingest`'s census lock refuses rather than
/// queues. Nothing in this process drains a pending list, because no pending
/// list exists. A route that answered `202 Accepted` would therefore be
/// answering for work that no code will ever pick up.
///
/// # Why it is `501` and not `503`
///
/// `503` says *try again later*, which is what `/pull/fno` correctly says about
/// a transport that is built but unwired. This is not that: deferral is absent
/// by decision rather than by scheduling, and it stays absent until one is
/// recorded. See D-0094.
///
/// # It cannot start a pull, structurally
///
/// No vendor call, no credential read, no store write and no task spawn appears
/// below. The route is a parser and a sentence.
pub async fn queue(
    axum::extract::State(site): axum::extract::State<crate::server::Loaded>,
    body: String,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let (code, payload) = queue_answer(
        &body,
        ist_day(SystemTime::now()),
        site.autopilot.seat_held(),
    );
    (code, json_headers(), payload)
}

/// [`queue`]'s body, over a day and a seat a caller supplies.
///
/// Split from the handler for the reason every dated thing in this module is:
/// a window gate driven by the machine's clock is a gate no test can pin, and
/// `CLAUDE.md` §3 rule 5 makes reruns mean the same thing twice.
fn queue_answer(
    body: &str,
    today: Result<Day, Refusal>,
    seat: bool,
) -> (axum::http::StatusCode, String) {
    let js = crate::render::json_string;
    let refused = |code: axum::http::StatusCode, why: &Refusal| {
        let field = match refused_field(why) {
            Some(field) => js(field),
            // Not a guess and not a blank: the clock is what is wrong, and
            // naming a control would send an operator to edit a date that is
            // already correct.
            None => String::from("null"),
        };
        (
            code,
            format!(
                r#"{{"queued":false,"why":{},"field":{}}}"#,
                js(&why.to_string()),
                field
            ),
        )
    };
    let today = match today {
        Ok(today) => today,
        // The same code `/pull/spot` answers an unusable clock with, for the
        // same reason: nothing the operator sent is wrong.
        Err(why) => {
            return refused(axum::http::StatusCode::INTERNAL_SERVER_ERROR, &why);
        }
    };
    let asked = match parse_spot(body, today) {
        Ok(asked) => asked,
        Err(why) => return refused(axum::http::StatusCode::BAD_REQUEST, &why),
    };
    (
        axum::http::StatusCode::NOT_IMPLEMENTED,
        format!(
            r#"{{"queued":false,"why":{},"understood":{{"target":{},"from":{},"to":{},"days":{},"feed":{},"granularity":{}}},"pull_seat":{},"instead":{{"now":"POST /pull/spot","backfill":"GET /ingest/status.json"}}}}"#,
            js(NO_QUEUE),
            js(asked.target.slug()),
            js(&asked.window.from().to_string()),
            js(&asked.window.to().to_string()),
            asked.window.days(),
            js(asked.feed.wire()),
            js(asked.granularity.dir()),
            js(if seat { "held" } else { "free" }),
        ),
    )
}

/// The reason a legal selection is still not queued.
///
/// One constant, because it is a claim about this build that must be checkable
/// in one place. Every noun in it is a real name: the seat is
/// [`crate::autopilot::Control::take_seat`], the lock is `pull::ingest`'s, and
/// the two routes that DO something are the two it names.
pub const NO_QUEUE: &str = "REFUSED · the selection is legal and was understood in \
     full, and nothing was queued, because this build has no queue to put it in. It \
     is not being held anywhere and no task will pick it up later: there is one pull \
     seat (api::autopilot::Control::take_seat, a compare-exchange), pull::ingest's \
     census lock refuses rather than queues, and nothing in this process drains a \
     pending list. Answering 202 here would be accepting and dropping it, which \
     CLAUDE.md §4 bans. What exists instead: POST /pull/spot runs exactly this \
     selection now, synchronously, and answers with the receipt; and the autopilot \
     backfills the swept indices oldest-first on its own, with GET \
     /ingest/status.json saying what it is waiting on. Whether a request may be \
     DEFERRED — accepted now and run later against a vendor with nobody watching — is \
     a decision this repository has not recorded, and it is the one that has to exist \
     before this route can answer anything else.";

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).expect("a real date")
    }

    /// A "today" far enough ahead that the future-window gate is inert.
    ///
    /// Every window test below predates this by decades, so they exercise the
    /// rule they were written for and not the new one. The gate gets its own
    /// test at its own boundary rather than being smuggled into theirs — a
    /// shared constant that silently decides two rules is how a test starts
    /// passing for the wrong reason.
    const TEST_TODAY: Day = match Day::new(2099, 12, 31) {
        Ok(d) => d,
        Err(_) => panic!("2099-12-31 is a real date"),
    };

    /// The gate `parse_window` was missing, at both sides of its boundary.
    ///
    /// `to` is **inclusive**, so a window ending today is legal — today has
    /// traded, or is trading. Tomorrow has not.
    ///
    /// This posts a body. The test it replaces asserted that the string `max=`
    /// appeared in the rendered HTML, which proves the browser was asked
    /// nicely and proves nothing at all about `curl`, a replayed POST, or any
    /// client that is not the page. That gap is why this shipped.
    #[test]
    fn a_window_ending_after_today_is_refused_by_the_parser() {
        let today = day(2026, 8, 7);

        let ends_today = parse_window("from=2026-08-01&to=2026-08-07", today)
            .expect("a window ending today is legal — today has traded");
        assert_eq!(ends_today.days(), 7, "1st through 7th inclusive");

        let tomorrow = parse_window("from=2026-08-01&to=2026-08-08", today)
            .expect_err("a window ending tomorrow has no bars to fetch");
        assert_eq!(
            tomorrow,
            Refusal::WindowInFuture {
                to: day(2026, 8, 8),
                today,
            },
            "refused by name, carrying both dates"
        );

        let text = tomorrow.to_string();
        assert!(
            text.contains("2026-08-08") && text.contains("2026-08-07"),
            "the refusal names the end and today, so an operator can see the \
             difference without opening a calendar — got {text:?}"
        );

        let far = parse_window("from=2026-08-01&to=2099-01-01", today)
            .expect_err("a window years ahead is refused too");
        // It breaks both rules, and it is refused for the one that is checked
        // first. Asserted exactly rather than as "one of two": a test that
        // accepts either answer cannot tell the two gates apart, and would
        // still pass if the order silently changed.
        let span = Window::new(day(2026, 8, 1), day(2099, 1, 1))
            .expect("a forward window")
            .days();
        assert!(
            span > MAX_WINDOW_DAYS,
            "the fixture must break the length bound as well as the future one"
        );
        assert_eq!(
            far,
            Refusal::WindowTooLong {
                days: span,
                cap: MAX_WINDOW_DAYS,
            },
            "the length bound is checked before the future gate, so that is \
             the refusal an operator sees — got {far:?}"
        );
    }

    /// The day every F&O test compares against.
    fn today() -> Day {
        day(2026, 8, 7)
    }

    #[test]
    fn a_date_field_is_refused_by_name_whichever_way_it_is_wrong() {
        // The wire shape, accepted.
        assert_eq!(parse_day("from", "2022-01-08"), Ok(day(2022, 1, 8)));
        // A leap day that exists, and one that does not — the calendar rule is
        // `Day::new`'s and this proves it is actually consulted.
        assert_eq!(parse_day("to", "2024-02-29"), Ok(day(2024, 2, 29)));
        assert_eq!(
            parse_day("to", "2023-02-29"),
            Err(Refusal::DateImpossible {
                field: "to",
                got: "2023-02-29".to_owned(),
                why: SessionError::DayOutOfRange {
                    day: 29,
                    month_len: 28
                },
            })
        );

        // Empty is its own refusal: "you left it blank" and "you typed
        // nonsense" send an operator to different places.
        assert_eq!(
            parse_day("from", ""),
            Err(Refusal::FieldMissing { field: "from" })
        );

        // Every malformed shape is DateNotIso, and each names the field.
        for bad in [
            "2024-2-9",     // unpadded — the ambiguity this format exists to kill
            "08/01/2022",   // the format nobody can read the same way twice
            "2022-01-08x",  // trailing rubbish
            "2022-01",      // truncated
            "2022-01-08-1", // one part too many
            "yyyy-mm-dd",   // not digits
            "-001-01-01",   // a sign is not a digit
            "20220-1-08",   // right length, wrong widths
        ] {
            assert_eq!(
                parse_day("expiry", bad),
                Err(Refusal::DateNotIso {
                    field: "expiry",
                    got: bad.to_owned()
                }),
                "{bad:?} must be refused as malformed"
            );
        }

        // Out-of-calendar values reach `Day::new` and are refused there, so the
        // month and year arms are not this parser's second opinion — and the
        // calendar's OWN refusal is what comes back, not a paraphrase.
        assert_eq!(
            parse_day("from", "2022-13-01"),
            Err(Refusal::DateImpossible {
                field: "from",
                got: "2022-13-01".to_owned(),
                why: SessionError::MonthOutOfRange { month: 13 },
            })
        );
        assert_eq!(
            parse_day("from", "1969-12-31"),
            Err(Refusal::DateImpossible {
                field: "from",
                got: "1969-12-31".to_owned(),
                why: SessionError::YearOutOfRange { year: 1969 },
            })
        );

        // Every refusal SAYS which field and what it saw.
        let text = parse_day("expiry", "2024-2-9")
            .expect_err("malformed")
            .to_string();
        assert!(
            text.contains("expiry") && text.contains("2024-2-9"),
            "{text}"
        );
        assert!(text.contains("YYYY-MM-DD"), "and what it wanted: {text}");
    }

    #[test]
    fn a_window_is_inclusive_both_ends_and_a_backwards_one_is_refused_not_swapped() {
        // A single day is legal and is the commonest resume shape.
        let one = parse_window("from=2022-01-08&to=2022-01-08", TEST_TODAY).expect("one day");
        assert_eq!(one.days(), 1);
        assert_eq!(one.from(), one.to());

        // A range across a year boundary is arithmetic, not a special case.
        let over =
            parse_window("from=2021-12-30&to=2022-01-02", TEST_TODAY).expect("across the year");
        assert_eq!(over.days(), 4, "30, 31, 1, 2 — both ends included");
        assert_eq!(over.from().year(), 2021);
        assert_eq!(over.to().year(), 2022);
        // AND THE NON-INCLUSIVE VENDOR `toDate` IS THE DAY AFTER. This is the
        // correction the page tells the operator about rather than performing
        // in silence.
        assert_eq!(over.wire_to().expect("succ").to_string(), "2022-01-03");

        // Backwards is refused, and the refusal names both ends.
        let back =
            parse_window("from=2022-02-08&to=2022-01-08", TEST_TODAY).expect_err("backwards");
        assert_eq!(
            back,
            Refusal::WindowBackwards {
                why: SessionError::WindowRunsBackwards {
                    from: day(2022, 2, 8),
                    to: day(2022, 1, 8),
                },
            }
        );
        let text = back.to_string();
        assert!(
            text.contains("2022-02-08") && text.contains("2022-01-08"),
            "{text}"
        );
        assert!(text.contains("backwards"), "{text}");

        // A missing end is named, not defaulted to today.
        assert_eq!(
            parse_window("to=2022-01-08", TEST_TODAY),
            Err(Refusal::FieldMissing { field: "from" })
        );
        assert_eq!(
            parse_window("from=2022-01-08", TEST_TODAY),
            Err(Refusal::FieldMissing { field: "to" })
        );
    }

    #[test]
    fn the_window_is_bounded_at_the_boundary_and_the_bound_is_tight() {
        // Exactly the cap is accepted; one day more is refused naming both
        // numbers. A bound tested only well past itself is a bound whose
        // comparison can be flipped with the suite green.
        let last = day(2020, 1, 1)
            .days_from_epoch()
            .checked_add(MAX_WINDOW_DAYS - 1)
            .expect("in range");
        let at_cap = Day::from_days(last).expect("in range");
        let body = format!("from=2020-01-01&to={at_cap}");
        assert_eq!(
            parse_window(&body, TEST_TODAY)
                .expect("exactly the cap")
                .days(),
            MAX_WINDOW_DAYS
        );

        let over = Day::from_days(last + 1).expect("in range");
        let body = format!("from=2020-01-01&to={over}");
        assert_eq!(
            parse_window(&body, TEST_TODAY),
            Err(Refusal::WindowTooLong {
                days: MAX_WINDOW_DAYS + 1,
                cap: MAX_WINDOW_DAYS,
            })
        );
        let text = parse_window(&body, TEST_TODAY)
            .expect_err("too long")
            .to_string();
        assert!(text.contains("3654") && text.contains("3653"), "{text}");
    }

    #[test]
    fn a_spot_request_names_its_target_or_is_refused() {
        for target in SpotTarget::ALL {
            let body = format!("target={}&from=2022-01-08&to=2022-02-08", target.slug());
            let asked = parse_spot(&body, TEST_TODAY).expect("a real target");
            assert_eq!(asked.target, target);
            assert_eq!(asked.window.days(), 32);
            // Round-trip: the slug the form emits is the slug the parser reads.
            assert_eq!(SpotTarget::from_slug(target.slug()), Some(target));
        }
        // EVERY LABEL AND NOTE IS PINNED TO ITS TEXT, not merely to being
        // non-empty. These strings are the only thing on the form that says
        // which sets the engine sweeps and which it merely stores, and a
        // "non-empty" assertion is satisfied by any string at all.
        for (target, label, note) in [
            (
                SpotTarget::Swept,
                "Swept indices",
                "NSE-NIFTY and NSE-BANKNIFTY — the only two swept",
            ),
            (
                SpotTarget::Indices,
                "Reference indices",
                "stored and stamped onto trades; never swept",
            ),
            (
                SpotTarget::Equities,
                "NIFTY Total Market equities",
                "stored, never swept",
            ),
            (
                SpotTarget::Nifty500,
                "NIFTY 500 equities",
                "the published 500 constituents — stored, never swept",
            ),
            (
                SpotTarget::Nifty200,
                "NIFTY 200 equities",
                "the published 200 constituents — stored, never swept",
            ),
            (
                SpotTarget::Nifty100,
                "NIFTY 100 equities",
                "the published 100 constituents — stored, never swept",
            ),
            (
                SpotTarget::Nifty50,
                "NIFTY 50 equities",
                "the published 50 constituents — stored, never swept; these are \
                 the companies, not the NIFTY index",
            ),
        ] {
            assert_eq!(target.label(), label);
            assert_eq!(target.note(), note);
        }
        // EVERY SET BUT THE FIRST SAYS "never swept" IN THE TEXT THE FORM SHOWS.
        // `CLAUDE.md` §1 is enforced by `is_sweepable`, but an operator reads
        // the note, and a row that offered 500 instruments without that phrase
        // is the row that gets mistaken for a widened engine surface.
        for target in SpotTarget::ALL {
            if target == SpotTarget::Swept {
                continue;
            }
            assert!(
                target.note().contains("never swept"),
                "{} must say so on the form: {}",
                target.slug(),
                target.note()
            );
        }
        // Only the two swept indices are swept; every other set is stored, and
        // each names ONE bit rather than sharing one.
        assert_eq!(SpotTarget::Swept.universe(), Universe::INDEX);
        assert_eq!(SpotTarget::Indices.universe(), Universe::INDEX);
        assert_eq!(SpotTarget::Equities.universe(), Universe::TOTAL_MARKET);
        assert_eq!(SpotTarget::Nifty500.universe(), Universe::NIFTY_500);
        assert_eq!(SpotTarget::Nifty200.universe(), Universe::NIFTY_200);
        assert_eq!(SpotTarget::Nifty100.universe(), Universe::NIFTY_100);
        assert_eq!(SpotTarget::Nifty50.universe(), Universe::NIFTY_50);

        assert_eq!(
            parse_spot("from=2022-01-08&to=2022-02-08", TEST_TODAY),
            Err(Refusal::FieldMissing { field: "target" })
        );
        assert_eq!(
            parse_spot("target=mcx&from=2022-01-08&to=2022-02-08", TEST_TODAY),
            Err(Refusal::UnknownTarget {
                got: "mcx".to_owned()
            }),
            "MCX is not on the engine surface and is not a target"
        );
        assert_eq!(SpotTarget::from_slug("bse"), None, "BSE is not a target");
        // A bad window still reaches the operator through the spot parser.
        assert_eq!(
            parse_spot("target=swept&from=2022-02-08&to=2022-01-08", TEST_TODAY),
            Err(Refusal::WindowBackwards {
                why: SessionError::WindowRunsBackwards {
                    from: day(2022, 2, 8),
                    to: day(2022, 1, 8),
                },
            })
        );
    }

    /// One equity key, for driving [`SpotTarget::names`] over a real symbol.
    ///
    /// `Segment::Cash` and `Kind::Equity` are what the merged master carries
    /// for a cash-segment listing, and they are also what makes
    /// `is_sweepable` answer false — which is the half these tests are about.
    fn equity(symbol: &str) -> brutex_core::instrument::InstrumentKey {
        brutex_core::instrument::InstrumentKey {
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: brutex_core::instrument::Segment::Cash,
            underlying: Symbol::new(symbol).expect("a published constituent is a legal symbol"),
            kind: brutex_core::instrument::Kind::Equity,
        }
    }

    /// What every NIFTY-tier target has to prove, driven once per tier.
    ///
    /// Four tests call this rather than one test looping, so a failure names
    /// the tier in its own test name and a tier that stops being checked is a
    /// deleted function rather than a shortened array.
    ///
    /// **No fetch, no socket, no store.** Every fact here comes from a
    /// compile-time table in `brutex_core::universe` and from this crate's own
    /// parser.
    fn a_tier_resolves_to_its_published_list(
        target: SpotTarget,
        slug: &str,
        published: &'static [&'static str],
        bit: Universe,
        count: usize,
    ) {
        // THE SLUG IS THE WIRE'S OWN WORD, both directions. `/instruments.json`
        // already spells this universe `slug` in every row's `universes` array
        // (D-0089), and a second spelling here is a set a browser can count and
        // cannot request — which is the whole defect D-0105 closes.
        assert_eq!(target.slug(), slug);
        assert_eq!(SpotTarget::from_slug(slug), Some(target));

        // A REQUEST CARRYING IT PARSES, and carries THIS target rather than a
        // neighbour's: `from_slug` is a linear find over `ALL` and two variants
        // answering one slug would be invisible in a per-variant assertion.
        let body = format!("target={slug}&from=2022-01-08&to=2022-02-08");
        let asked = parse_spot(&body, TEST_TODAY).expect("a published tier is a real target");
        assert_eq!(asked.target, target);
        assert_eq!(asked.window.days(), 32);

        // THE RIGHT COUNT, and the count comes from `core`'s const rather than
        // from a literal that could drift away from it at the next rebalance.
        let members = target
            .members()
            .expect("a published tier has a published list");
        assert_eq!(members.len(), count, "{slug} names {count} instruments");
        assert_eq!(
            published.len(),
            count,
            "and the const it borrows is that long"
        );

        // THE RIGHT NAMES, element by element and in order. Fifty arbitrary
        // strings satisfy the count above; only `core`'s own list satisfies
        // this, and nothing in `crates/api` holds a second copy to satisfy it
        // with (`CLAUDE.md` §3 rule 1).
        assert_eq!(
            members, published,
            "{slug} must BE core's list, not resemble it"
        );

        for name in members {
            // Each name resolves back through core's own membership index to
            // the bit this target selects on — so the roster and the predicate
            // are proved to be the same set, not two lists that agree today.
            let u = universe::of_equity(name);
            assert!(u.contains(bit), "{name} must carry {slug}'s bit");
            assert!(
                target.names(&equity(name), u),
                "{slug} must name {name} on the pull path"
            );
        }
    }

    /// A tier target resolves to the NIFTY 500 and to nothing else.
    #[test]
    fn the_n500_target_resolves_to_the_five_hundred_published_constituents() {
        a_tier_resolves_to_its_published_list(
            SpotTarget::Nifty500,
            "n500",
            &universe::NIFTY_500,
            Universe::NIFTY_500,
            500,
        );
    }

    /// A tier target resolves to the NIFTY 200 and to nothing else.
    #[test]
    fn the_n200_target_resolves_to_the_two_hundred_published_constituents() {
        a_tier_resolves_to_its_published_list(
            SpotTarget::Nifty200,
            "n200",
            &universe::NIFTY_200,
            Universe::NIFTY_200,
            200,
        );
    }

    /// A tier target resolves to the NIFTY 100 and to nothing else.
    #[test]
    fn the_n100_target_resolves_to_the_one_hundred_published_constituents() {
        a_tier_resolves_to_its_published_list(
            SpotTarget::Nifty100,
            "n100",
            &universe::NIFTY_100,
            Universe::NIFTY_100,
            100,
        );
    }

    /// A tier target resolves to the NIFTY 50 and to nothing else.
    #[test]
    fn the_n50_target_resolves_to_the_fifty_published_constituents() {
        a_tier_resolves_to_its_published_list(
            SpotTarget::Nifty50,
            "n50",
            &universe::NIFTY_50,
            Universe::NIFTY_50,
            50,
        );
    }

    /// **A tier is STORED and is never SWEPT.** `CLAUDE.md` §1, as a test.
    ///
    /// This is the row that would be quietly wrong if a later change read a new
    /// target as a widened engine surface. The surface is two instruments and
    /// `InstrumentKey::is_sweepable` is the only thing that says so; a target
    /// is what a PULL covers, and D-0105 added four of them without touching
    /// that table.
    ///
    /// Driven over every member of all four tiers — 850 keys — because "no
    /// constituent is sweepable" is a claim about the whole set and a
    /// spot-check of one name is a claim about one name.
    #[test]
    fn a_nifty_tier_target_stores_and_never_sweeps() {
        for target in [
            SpotTarget::Nifty500,
            SpotTarget::Nifty200,
            SpotTarget::Nifty100,
            SpotTarget::Nifty50,
        ] {
            let members = target.members().expect("a published tier has a list");
            for name in members {
                let key = equity(name);
                assert!(
                    !key.is_sweepable(),
                    "{name} is stored by {}, and the engine sweeps NSE-NIFTY and \
                     NSE-BANKNIFTY only",
                    target.slug()
                );
                // And the swept target does not name it either: the two sets
                // share a word in the label and no member at all.
                assert!(
                    !SpotTarget::Swept.names(&key, universe::of_equity(name)),
                    "{name} is not on the engine surface"
                );
            }
        }

        // THE ENGINE SURFACE ITSELF IS UNCHANGED, counted rather than asserted
        // in prose: two pairs in `core`, and the only variant that reads them.
        assert_eq!(
            brutex_core::instrument::InstrumentKey::SWEPT.len(),
            2,
            "a new spot target must NEVER widen CLAUDE.md §1"
        );
        for (exchange, symbol) in brutex_core::instrument::InstrumentKey::SWEPT {
            let key = brutex_core::instrument::InstrumentKey::index(exchange, symbol)
                .expect("a swept index is a legal key");
            assert!(SpotTarget::Swept.names(&key, universe::of_instrument(&key)));
            // A NIFTY-tier target does not name the INDEX either. `NSE-NIFTY`
            // is the series; the tier is the companies underneath it.
            for tier in [
                SpotTarget::Nifty500,
                SpotTarget::Nifty200,
                SpotTarget::Nifty100,
                SpotTarget::Nifty50,
            ] {
                assert!(
                    !tier.names(&key, universe::of_instrument(&key)),
                    "{symbol} is an index, not a constituent of {}",
                    tier.slug()
                );
            }
        }
    }

    /// The two targets that are NOT defined by a published file say so.
    ///
    /// `None` from [`SpotTarget::members`] is a stated absence, not a fallback
    /// (`CLAUDE.md` §4). `Swept` is the engine surface and `Indices` is
    /// whatever the vendor master lists — inventing a symbol list for either
    /// would be a third copy of §1 and a snapshot that goes stale on the next
    /// vendor addition.
    #[test]
    fn the_three_targets_with_no_published_list_return_none_rather_than_an_invented_one() {
        // THREE, AND EACH LACKS A LIST FOR A DIFFERENT REASON — which is why
        // they are named one at a time rather than counted.
        //
        // `Swept` is the engine surface: two `(exchange, symbol)` pairs from
        // `InstrumentKey::SWEPT`, not a file. `Indices` is whatever a vendor's
        // master calls an index series, and NSE publishes no file naming that
        // set. `Everything` is a PREDICATE — `catalog::tracked`, a union of two
        // bits — and a union is not a publication either.
        assert_eq!(SpotTarget::Swept.members(), None);
        assert_eq!(SpotTarget::Indices.members(), None);
        assert_eq!(SpotTarget::Everything.members(), None);
        // Every other target has one, and the Total Market's is core's too.
        assert_eq!(
            SpotTarget::Equities.members(),
            Some(&universe::NIFTY_TOTAL_MARKET[..])
        );
        // The F&O roster was transcribed long before any target pointed at it.
        assert_eq!(
            SpotTarget::Fno.members(),
            Some(&universe::FNO_UNDERLYINGS[..])
        );
        for target in SpotTarget::ALL {
            let published = target.members().is_some();
            assert_eq!(
                published,
                !matches!(
                    target,
                    SpotTarget::Swept | SpotTarget::Indices | SpotTarget::Everything
                ),
                "{} either has a published list or is one of the three that do not",
                target.slug()
            );
        }
    }

    /// The tier a target JOINS THROUGH, and the two that join through nothing.
    ///
    /// [`SpotTarget::tier`] is the half of the wiring that turns a form field
    /// into `crate::constituents::Join::tier` — the ids one feed can be asked
    /// for. It is pinned against [`SpotTarget::members`] rather than asserted
    /// on its own: the two answer the same question from opposite ends, one
    /// naming the roster and one naming the join over it, and a target that has
    /// a published list and no tier (or the reverse) is the pairing that would
    /// silently hand `/ingest` an empty control.
    #[test]
    fn every_target_with_a_published_list_joins_through_that_lists_tier() {
        for target in SpotTarget::ALL {
            let slug = target.slug();
            assert_eq!(
                target.tier().is_some(),
                target.members().is_some(),
                "{slug}: a target has a tier exactly when it has a published list"
            );
            let Some(tier) = target.tier() else {
                continue;
            };
            // THE SAME LIST, BY BOTH ROUTES. `members` borrows the const from
            // `core` and `tier().members()` borrows it through
            // `constituents::Tier` — if these two ever name different lists,
            // the count on the form and the ids in the request are for
            // different universes.
            assert_eq!(
                target.members(),
                Some(tier.members()),
                "{slug}: the roster and the joined tier are the same list"
            );
            assert_eq!(
                target.universe(),
                tier.universe(),
                "{slug}: and the same membership bit"
            );
        }
        // The two with nothing to join through, named rather than inferred.
        assert_eq!(SpotTarget::Swept.tier(), None);
        assert_eq!(SpotTarget::Indices.tier(), None);
        assert_eq!(
            SpotTarget::Nifty50.tier(),
            Some(crate::constituents::Tier::Nifty50),
            "and no tier is wired to a neighbour's list, which a positional \
             mapping is exactly how you get"
        );
        assert_eq!(
            SpotTarget::Equities.tier(),
            Some(crate::constituents::Tier::TotalMarket)
        );
    }

    /// **An unknown slug is still refused BY NAME**, and the refusal now says
    /// what the legal ones are.
    ///
    /// The four new arms must not turn `UnknownTarget` into a default. This
    /// drives words that are near-misses for the tiers specifically — the
    /// spellings a second vocabulary would have invented — plus the old cases
    /// and the empty field, because a parser that started accepting `nifty50`
    /// would have two names for one set and no way to say which the store filed
    /// under. `CLAUDE.md` §4.
    #[test]
    fn a_slug_that_is_not_a_target_is_refused_by_name_after_the_tiers_were_added() {
        // `fno` AND `all` LEFT THIS LIST ON 14 AUG 2026, and they left it in
        // opposite ways. `fno` became a real slug because
        // `server::UNIVERSE_TOKENS` already spells that bit `fno` and D-0105's
        // rule leaves no other spelling legal. `all` is the one slug this
        // repository CHOSE rather than borrowed — there is no
        // `/instruments.json` token for "everything" — and D-0136 says so.
        //
        // `ntm`, `index` and `*` stay. Each is a near-miss for a set that has a
        // different real name (`equities`, `indices`, and a browser-local
        // sentinel that is not a wire word), and accepting any of them would
        // give one set two names with no way to say which the store filed
        // under.
        for got in [
            "n25",
            "n1000",
            "nifty50",
            "nifty-50",
            "N50",
            "n 50",
            "n50 ",
            "50",
            "ntm",
            "index",
            "*",
            "mcx",
            "bse",
            "fno_underlyings",
            "everything",
        ] {
            assert_eq!(SpotTarget::from_slug(got), None, "{got:?} is not a target");
            let body = format!("target={got}&from=2022-01-08&to=2022-02-08");
            let why = parse_spot(&body, TEST_TODAY).expect_err("not a target");
            assert_eq!(
                why,
                Refusal::UnknownTarget {
                    got: got.to_owned()
                },
                "{got:?} must refuse as UnknownTarget and not as anything softer"
            );
            // BY NAME, both names: what arrived and what would have worked.
            let text = why.to_string();
            assert!(
                text.contains(got),
                "the refusal quotes what arrived: {text}"
            );
            for target in SpotTarget::ALL {
                assert!(
                    text.contains(target.slug()),
                    "the refusal lists {}: {text}",
                    target.slug()
                );
            }
        }
        // AND AN EMPTY FIELD IS STILL ITS OWN REFUSAL. "You named nothing" and
        // "you named a thing that does not exist" are different facts and the
        // tiers did not merge them.
        assert_eq!(
            parse_spot("from=2022-01-08&to=2022-02-08", TEST_TODAY),
            Err(Refusal::FieldMissing { field: "target" })
        );
    }

    /// The rung is a field, it defaults to the minute, and a rung this ladder
    /// has no rung for is refused BY NAME.
    ///
    /// The default is the load-bearing half. `/pull/spot` is a POST an operator
    /// can replay and a body written before this field existed must keep the
    /// meaning it had — `CLAUDE.md` §3 rule 5 makes reruns safe, and a request
    /// that silently changed rung would file the same window twice under two
    /// directories, which the append-only store has no way to undo.
    #[test]
    fn a_spot_request_names_its_bar_length_defaults_to_the_minute_or_is_refused() {
        use pull::vendor::Granularity;

        // ABSENT IS THE MINUTE. This is the body every test written before the
        // field existed sends, and it must still mean what it meant.
        let old = parse_spot("target=swept&from=2022-01-08&to=2022-02-08", TEST_TODAY)
            .expect("a body with no rung is still a request");
        assert_eq!(
            old.granularity,
            Granularity::Minute1,
            "an unstated rung is one minute, which is what it was before there \
             was a field to state"
        );

        // AND EVERY RUNG THE STORE CAN FILE ROUND-TRIPS through the directory
        // name — one spelling for the control, the wire and the path.
        for rung in Granularity::ALL {
            if rung.store_timeframe().is_none() {
                continue;
            }
            let body = format!(
                "target=swept&granularity={}&from=2022-01-08&to=2022-02-08",
                rung.dir()
            );
            let asked = parse_spot(&body, TEST_TODAY).expect("a rung this build stores");
            assert_eq!(asked.granularity, rung);
            assert_eq!(parse_granularity(rung.dir()), Some(rung));
        }

        // A RUNG THAT IS NOT ON THE LADDER AT ALL is refused naming what
        // arrived, never coerced to the default — the coercion would file bars
        // under a directory the operator did not ask for.
        assert_eq!(
            parse_spot(
                "target=swept&granularity=hourly&from=2022-01-08&to=2022-02-08",
                TEST_TODAY
            ),
            Err(Refusal::UnknownGranularity {
                got: "hourly".to_owned()
            })
        );
        assert_eq!(parse_granularity("1 day"), None, "the spelling is exact");
        // A rung that IS on the ladder and has no directory parses here and is
        // refused at the write boundary instead. Two different questions, and
        // this parser answers only the first — see `pull::ingest::Plan`.
        //
        // THE RUNG THIS USED TO NAME WAS `5min`, AND IT WAS THE WRONG EXAMPLE.
        // `crates/store` has shipped a `5min` directory since D-0054 widened
        // `Timeframe::KNOWN` to seven; the rung read as unstorable only because
        // `Granularity::store_timeframe` carried an underscore arm that
        // answered `None` for every rung it had not been told about. So this
        // line asserted the defect rather than the rule. `Week1` is the honest
        // example: it is on the ladder, `Timeframe::KNOWN` genuinely holds no
        // entry for it, and the two questions stay distinguishable.
        assert_eq!(parse_granularity("5min"), Some(Granularity::Minute5));
        assert!(
            Granularity::Minute5.store_timeframe().is_some(),
            "crates/store ships a 5min directory and the parser reaches it"
        );
        assert_eq!(parse_granularity("1week"), Some(Granularity::Week1));
        assert_eq!(Granularity::Week1.store_timeframe(), None);
    }

    #[test]
    fn an_expired_contract_is_accepted_and_a_live_one_can_never_be() {
        let ok = parse_fno(
            "underlying=nifty&series=fut&expiry=2026-07-30&from=2026-07-01&to=2026-07-30",
            today(),
        )
        .expect("an expired series");
        assert_eq!(
            ok.underlying.as_str(),
            "NIFTY",
            "the typing is canonicalised"
        );
        assert_eq!(ok.series, Series::Futures);
        assert_eq!(ok.expiry, day(2026, 7, 30));
        assert_eq!(ok.window.days(), 30);

        // THE GATE. An expiry after today, and an expiry ON today, are both
        // live. `>=` rather than `>` is the whole rule: a contract expiring
        // today is still trading today.
        for live in ["2026-08-08", "2026-08-07", "9998-12-31"] {
            let body =
                format!("underlying=NIFTY&series=opt&expiry={live}&from=2026-07-01&to=2026-07-30");
            let refused = parse_fno(&body, today()).expect_err("a live contract");
            assert_eq!(
                refused,
                Refusal::LiveContract {
                    expiry: parse_day("expiry", live).expect("a real date"),
                    today: today(),
                },
                "{live} must be refused as live"
            );
            let text = refused.to_string();
            assert!(text.contains("LIVE CONTRACT IS NEVER STORED"), "{text}");
            assert!(text.contains(live) && text.contains("2026-08-07"), "{text}");
        }
        // One day before today is the first legal expiry — the bound is tight.
        assert!(
            parse_fno(
                "underlying=NIFTY&series=opt&expiry=2026-08-06&from=2026-08-06&to=2026-08-06",
                today()
            )
            .is_ok(),
            "yesterday's expiry is expired"
        );
    }

    #[test]
    fn an_fno_request_is_refused_by_name_for_every_other_reason_too() {
        let base = "underlying=NIFTY&series=fut&expiry=2026-07-30&from=2026-07-01&to=2026-07-30";

        // A window that runs past the expiry asks for bars that cannot exist.
        let past = parse_fno(
            "underlying=NIFTY&series=fut&expiry=2026-07-30&from=2026-07-01&to=2026-07-31",
            today(),
        )
        .expect_err("past the expiry");
        assert_eq!(
            past,
            Refusal::WindowOutlivesTheContract {
                to: day(2026, 7, 31),
                expiry: day(2026, 7, 30),
            }
        );
        assert!(past.to_string().contains("no bars there"), "{past}");

        // An underlying with no F&O series on it. RAJESHEXPO is a real NSE
        // share and is in neither F&O list.
        let spot_only =
            parse_fno(&base.replace("NIFTY", "RAJESHEXPO"), today()).expect_err("no derivative");
        assert_eq!(
            spot_only,
            Refusal::NotAnFnoUnderlying {
                symbol: "RAJESHEXPO".to_owned()
            }
        );
        assert!(
            spot_only.to_string().contains("no F&O series"),
            "{spot_only}"
        );

        // A symbol this engine cannot even name — a space is not a legal byte.
        assert_eq!(
            parse_fno(&base.replace("NIFTY", "NIFTY+100"), today()),
            Err(Refusal::BadUnderlying {
                got: "NIFTY 100".to_owned()
            }),
            "the form encodes a space as '+', and it is still not a symbol"
        );

        // Every missing field is named as itself.
        assert_eq!(
            parse_fno(
                "series=fut&expiry=2026-07-30&from=2026-07-01&to=2026-07-30",
                today()
            ),
            Err(Refusal::FieldMissing {
                field: "underlying"
            })
        );
        assert_eq!(
            parse_fno(
                "underlying=NIFTY&expiry=2026-07-30&from=2026-07-01&to=2026-07-30",
                today()
            ),
            Err(Refusal::FieldMissing { field: "series" })
        );
        assert_eq!(
            parse_fno(
                "underlying=NIFTY&series=fut&from=2026-07-01&to=2026-07-30",
                today()
            ),
            Err(Refusal::FieldMissing { field: "expiry" })
        );
        assert_eq!(
            parse_fno(&base.replace("series=fut", "series=swap"), today()),
            Err(Refusal::UnknownSeries {
                got: "swap".to_owned()
            })
        );
        // Both series round-trip through the wire value the form emits, and
        // each label is pinned to its text: "Future" and "Option chain" are
        // what an operator picks between, and a non-empty assertion would be
        // satisfied by either of them saying the other.
        for (series, slug, label) in [
            (Series::Futures, "fut", "Future"),
            (Series::Options, "opt", "Option chain"),
        ] {
            assert_eq!(series.slug(), slug);
            assert_eq!(series.label(), label);
            assert_eq!(Series::from_slug(slug), Some(series));
        }
        assert_eq!(
            Series::ALL.len(),
            2,
            "a future and an option chain, no more"
        );
        assert_eq!(Series::from_slug("forward"), None);

        // And a malformed expiry is a date refusal, not a live-contract one:
        // the gate must never be reached with a value nobody validated.
        assert_eq!(
            parse_fno(
                &base.replace("expiry=2026-07-30", "expiry=2026-7-30"),
                today()
            ),
            Err(Refusal::DateNotIso {
                field: "expiry",
                got: "2026-7-30".to_owned(),
            })
        );

        // A window refused AFTER the expiry gate has passed. Without this the
        // early return from `parse_window` inside `parse_fno` is a path no
        // input takes — the expiry is fine and the dates are not.
        assert_eq!(
            parse_fno(
                "underlying=NIFTY&series=fut&expiry=2020-01-30&to=2020-01-30",
                today()
            ),
            Err(Refusal::FieldMissing { field: "from" })
        );
    }

    #[test]
    fn every_refusal_names_itself_and_none_of_them_says_only_invalid_input() {
        // One arm per variant, and each one rendered. A `Display` arm no test
        // enters is a message an operator will one day read for the first time
        // in production — and `Refusal` exists precisely so that "invalid
        // input" is never the answer.
        let day = day(2026, 8, 7);
        let cases: [(Refusal, &str); 12] = [
            (
                Refusal::UnknownGranularity {
                    got: "5min".to_owned(),
                },
                // The rung the operator typed, QUOTED BACK. A message that said
                // only "unsupported bar length" leaves them guessing which of
                // eleven rungs this build refused and which two it takes.
                "\"5min\" is not a bar length",
            ),
            (
                Refusal::FieldMissing { field: "from" },
                "from was not filled in",
            ),
            (
                Refusal::DateNotIso {
                    field: "to",
                    got: "08/01/2022".to_owned(),
                },
                "YYYY-MM-DD",
            ),
            (
                Refusal::DateImpossible {
                    field: "expiry",
                    got: "2023-02-29".to_owned(),
                    why: SessionError::DayOutOfRange {
                        day: 29,
                        month_len: 28,
                    },
                },
                "is not a day",
            ),
            (
                Refusal::WindowBackwards {
                    why: SessionError::WindowRunsBackwards { from: day, to: day },
                },
                "backwards",
            ),
            (
                Refusal::WindowTooLong {
                    days: 5_000,
                    cap: MAX_WINDOW_DAYS,
                },
                "5000 days",
            ),
            (
                Refusal::UnknownTarget {
                    got: "mcx".to_owned(),
                },
                // THE LEGAL SLUGS, not a count. The sentence used to say
                // "one of the three spot targets" and there are seven; the
                // assertion pins the two ends of the generated list so that
                // adding an eighth without adding it to `ALL` is visible here.
                "is not a spot target. This build takes swept, ",
            ),
            (
                Refusal::UnknownSeries {
                    got: "swap".to_owned(),
                },
                "neither a future nor an option",
            ),
            (
                Refusal::BadUnderlying {
                    got: "NIFTY 100".to_owned(),
                },
                "not a symbol this engine can name",
            ),
            (
                Refusal::NotAnFnoUnderlying {
                    symbol: "RAJESHEXPO".to_owned(),
                },
                "no F&O series",
            ),
            (
                Refusal::LiveContract {
                    expiry: day,
                    today: day,
                },
                "LIVE CONTRACT IS NEVER STORED",
            ),
            (
                Refusal::WindowOutlivesTheContract {
                    to: day,
                    expiry: day,
                },
                "no bars there",
            ),
        ];
        for (refusal, expected) in cases {
            let text = refusal.to_string();
            assert!(
                text.starts_with("REFUSED · "),
                "every refusal is loud: {text}"
            );
            assert!(text.contains(expected), "{refusal:?} rendered as {text}");
        }
        // And the thirteenth, which carries a calendar refusal of its own.
        let clock = Refusal::ClockUnusable {
            why: SessionError::BeforeEpoch { secs: -1 },
        };
        assert!(clock.to_string().contains("clock names no usable date"));
        // It is an error, so a caller may propagate it rather than reformat it.
        let as_error: &dyn std::error::Error = &clock;
        assert!(as_error.to_string().contains("REFUSED"));
    }

    /// Every refusal says which control an operator must change, and the one
    /// that is not about a control names none rather than guessing.
    ///
    /// This is the field a log line is filtered on — `field=to` counts every
    /// way one date box was wrong, across seventeen differently worded
    /// sentences. A table rather than a spot check, because an arm that names
    /// the wrong control is a wrong answer that still looks like an answer.
    #[test]
    fn every_refusal_names_the_control_an_operator_must_change() {
        let d = day(2026, 8, 7);
        let backwards = SessionError::WindowRunsBackwards { from: d, to: d };
        let cases: [(Refusal, Option<&str>); 17] = [
            (Refusal::FieldMissing { field: "from" }, Some("from")),
            (
                Refusal::DateNotIso {
                    field: "to",
                    got: "08/01/2022".to_owned(),
                },
                Some("to"),
            ),
            (
                Refusal::DateImpossible {
                    field: "expiry",
                    got: "2023-02-29".to_owned(),
                    why: SessionError::DayOutOfRange {
                        day: 29,
                        month_len: 28,
                    },
                },
                Some("expiry"),
            ),
            (Refusal::WindowBackwards { why: backwards }, Some("window")),
            (
                Refusal::WindowTooLong {
                    days: 5_000,
                    cap: MAX_WINDOW_DAYS,
                },
                Some("window"),
            ),
            (
                Refusal::WindowInFuture { to: d, today: d },
                // The START of every window refusal below is legal; it is the
                // end that is wrong, and that is the box to change.
                Some("to"),
            ),
            (Refusal::WindowReachesToday { to: d, today: d }, Some("to")),
            (
                Refusal::WindowOutlivesTheContract { to: d, expiry: d },
                Some("to"),
            ),
            (
                Refusal::ArchiveFolderMissing { feed: "TrueData" },
                Some("folder"),
            ),
            (
                Refusal::UnknownTarget {
                    got: "mcx".to_owned(),
                },
                Some("target"),
            ),
            (
                Refusal::UnknownSeries {
                    got: "swap".to_owned(),
                },
                Some("series"),
            ),
            (
                Refusal::UnknownVendor {
                    got: "zerodha".to_owned(),
                },
                Some("vendor"),
            ),
            (
                Refusal::UnknownGranularity {
                    got: "hourly".to_owned(),
                },
                Some("granularity"),
            ),
            (
                Refusal::BadUnderlying {
                    got: "NIFTY 100".to_owned(),
                },
                Some("underlying"),
            ),
            (
                Refusal::NotAnFnoUnderlying {
                    symbol: "RAJESHEXPO".to_owned(),
                },
                Some("underlying"),
            ),
            (
                Refusal::LiveContract {
                    expiry: d,
                    today: d,
                },
                Some("expiry"),
            ),
            // THE ONE THAT NAMES NOTHING. The operator's form is fine and the
            // machine's clock is not; sending them to a date box would send
            // them to edit a value that is already correct.
            (
                Refusal::ClockUnusable {
                    why: SessionError::BeforeEpoch { secs: -1 },
                },
                None,
            ),
        ];
        for (refusal, field) in cases {
            assert_eq!(refused_field(&refusal), field, "{refusal:?}");
        }
    }

    #[test]
    fn the_clock_is_read_in_ist_and_a_clock_that_names_no_day_is_refused() {
        // The epoch itself is 05:30 IST on 1 January 1970.
        assert_eq!(
            ist_day(SystemTime::UNIX_EPOCH).expect("the epoch"),
            day(1970, 1, 1)
        );
        // 18:30 UTC is already the next day in IST, which is the whole reason
        // the conversion is not "seconds / 86,400".
        let evening = SystemTime::UNIX_EPOCH + Duration::from_mins(18 * 60 + 30);
        assert_eq!(ist_day(evening).expect("evening"), day(1970, 1, 2));
        assert_eq!(epoch_secs(SystemTime::UNIX_EPOCH), 0);
        assert_eq!(epoch_secs(evening), 66_600);

        // A clock before the epoch is REFUSED, never defaulted. IST is +05:30,
        // so it has to be more than 19,800 seconds behind to name no day.
        let broken = SystemTime::UNIX_EPOCH - Duration::from_hours(24);
        assert_eq!(epoch_secs(broken), -86_400);
        let refused = ist_day(broken).expect_err("no such day");
        assert_eq!(
            refused,
            Refusal::ClockUnusable {
                why: SessionError::BeforeEpoch { secs: -86_400 },
            }
        );
        assert!(refused.to_string().contains("clock"), "{refused}");

        // The real clock is on this machine and this build can name its day.
        assert!(
            today_ist().is_ok(),
            "the host clock names a day this build renders"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod vendor_parse_tests {
    use super::*;

    /// **A NAMED VENDOR IS NEVER SILENTLY REPLACED BY ANOTHER.**
    ///
    /// Two copies of this parse existed. `/bars` compared with `==`,
    /// `parse_vendor` with `eq_ignore_ascii_case`, and both ended
    /// `.unwrap_or(Vendor::Dhan)` — so `?vendor=Groww` resolved differently on
    /// the two routes and *both* answers were Dhan. Asking for one broker's
    /// copy of a month and being served another's, under HTTP 200, is a wrong
    /// answer wearing the shape of a right one.
    ///
    /// The old behaviour was argued for in this file: "the vendor is a route,
    /// not a claim about the data: naming one that does not exist cannot
    /// corrupt anything". True when written. False once `Plan.vendor` became
    /// the store prefix — coercing to Dhan files one broker's prices under
    /// another's path.
    #[test]
    fn a_vendor_this_build_cannot_serve_is_refused_rather_than_replaced() {
        // Absent still defaults, and that half was always sound: Dhan is the
        // descriptor verified against a live body.
        assert_eq!(parse_vendor(""), Some(brutex_core::vendor::Vendor::Dhan));

        // Every real vendor round-trips through its own wire spelling, in any
        // case, because an operator types a query string by hand.
        for vendor in brutex_core::vendor::Vendor::ALL {
            for spelling in [
                vendor.as_str().to_owned(),
                vendor.as_str().to_uppercase(),
                {
                    let mut s = vendor.as_str().to_owned();
                    s[..1].make_ascii_uppercase();
                    s
                },
            ] {
                assert_eq!(
                    parse_vendor(&spelling),
                    Some(vendor),
                    "{spelling:?} must resolve to {vendor:?}, not fall back"
                );
            }
        }

        // AND THE HALF THAT WAS WRONG: a name this build has no feed for is
        // `None`, so a caller must refuse rather than pick something.
        //
        // `zerodha` LEFT THIS LIST on 14 Aug 2026, and it left it by becoming a
        // real feed — it is asserted in the loop above like every other. A test
        // for an unknown vendor has to name one that stays unknown, so the
        // examples are now brokers this build does not carry.
        for wrong in ["upstox", "angelone", "DHANN", "gro ww", "0", "'"] {
            assert_eq!(
                parse_vendor(wrong),
                None,
                "{wrong:?} named a vendor this build cannot serve; returning \
                 Some(_) here is how a month gets served from the wrong prefix"
            );
        }
    }

    /// The refusal names what it refused, so an operator can see the typo.
    #[test]
    fn the_refusal_carries_the_word_that_was_rejected() {
        let refusal = Refusal::UnknownVendor {
            got: "zerodha".to_owned(),
        };
        let said = refusal.to_string();
        assert!(said.contains("zerodha"), "{said}");
        assert!(said.starts_with("REFUSED"), "{said}");
    }
}

/// The two routes this module serves, driven through their own handlers.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod route_tests {
    use super::*;

    use crate::autopilot::{FeedReport, Phase, Status};
    use crate::server::{Loaded, Site};

    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).expect("a real date")
    }

    /// An empty site on a store root of its own.
    ///
    /// **`name` must differ per test**, for the reason `crate::scratch` gives:
    /// the path stamps the process, not the test.
    fn empty_site(name: &str) -> Loaded {
        let dir = crate::scratch::path(&format!("ingest-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        Loaded::new(Site::load(&dir, &dir))
    }

    /// A body the parser accepts, with a window long behind any clock.
    const LEGAL: &str = "target=swept&from=2020-01-01&to=2020-01-31&vendor=groww&granularity=1min";

    /// **A legal selection is not queued, and the answer never once says it
    /// was.**
    ///
    /// The half of `CLAUDE.md` §4 that is easy to fail here is not the refusal
    /// — it is answering `202 Accepted` for work no code will pick up. There is
    /// no pending list in this process, so an acceptance would be a receipt for
    /// nothing.
    #[test]
    fn a_legal_selection_is_refused_by_name_rather_than_accepted_and_dropped() {
        let (code, body) = queue_answer(LEGAL, Ok(day(2026, 8, 10)), false);
        assert_eq!(code, axum::http::StatusCode::NOT_IMPLEMENTED, "{body}");
        assert!(body.contains(r#""queued":false"#), "{body}");
        assert!(!body.contains(r#""queued":true"#), "{body}");
        // IT ECHOES WHAT WOULD HAVE GONE ON THE WIRE, so a caller can see its
        // selection was understood and not merely rejected.
        assert!(body.contains(r#""target":"swept""#), "{body}");
        assert!(body.contains(r#""from":"2020-01-01""#), "{body}");
        assert!(body.contains(r#""to":"2020-01-31""#), "{body}");
        assert!(body.contains(r#""days":31"#), "{body}");
        assert!(body.contains(r#""granularity":"1min""#), "{body}");
        // AND IT NAMES WHAT DOES EXIST. A refusal with no route out of it is a
        // dead end dressed as an answer.
        assert!(body.contains("POST /pull/spot"), "{body}");
        assert!(body.contains("GET /ingest/status.json"), "{body}");
        assert!(body.contains("no queue"), "{body}");
        assert!(body.contains(r#""pull_seat":"free""#), "{body}");

        let (_, held) = queue_answer(LEGAL, Ok(day(2026, 8, 10)), true);
        assert!(
            held.contains(r#""pull_seat":"held""#),
            "the caller is told a pull is already running: {held}"
        );
    }

    /// Every refusal a spot body can earn arrives with the control that owns
    /// it, so a caller knows where to go.
    #[test]
    fn a_refused_selection_names_the_field_and_the_clock_names_none() {
        let cases = [
            ("from=2020-01-01&to=2020-01-31", "target"),
            ("target=swept&from=&to=2020-01-31", "from"),
            (
                // `zerodha` was this example until 14 Aug 2026, when it became a
                // real feed. A test for an UNKNOWN vendor has to name one that
                // stays unknown.
                "target=swept&from=2020-01-01&to=2020-01-31&vendor=upstox",
                "vendor",
            ),
            ("target=nope&from=2020-01-01&to=2020-01-31", "target"),
        ];
        for (body, field) in cases {
            let (code, answer) = queue_answer(body, Ok(day(2026, 8, 10)), false);
            assert_eq!(
                code,
                axum::http::StatusCode::BAD_REQUEST,
                "{body} was not refused: {answer}"
            );
            assert!(answer.contains(r#""queued":false"#), "{answer}");
            assert!(
                answer.contains(&format!(r#""field":"{field}""#)),
                "{body} must name {field}: {answer}"
            );
            assert!(answer.contains("REFUSED"), "{answer}");
        }

        // THE CLOCK IS THE ONE REFUSAL WITH NO FIELD, and it is `null` rather
        // than a guessed control: nothing the caller typed is wrong.
        let (code, answer) = queue_answer(
            LEGAL,
            Err(Refusal::ClockUnusable {
                why: SessionError::YearOutOfRange { year: 70_000 },
            }),
            false,
        );
        assert_eq!(
            code,
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "{answer}"
        );
        assert!(answer.contains(r#""field":null"#), "{answer}");
        assert!(answer.contains("clock"), "{answer}");
    }

    /// **The route writes nothing and reaches no vendor.** Asserted against the
    /// store root itself rather than against a comment: a queue that "just"
    /// journalled its request would still be a write this route promises not to
    /// make, and a pull started here would spend the operator's rate budget.
    #[tokio::test]
    async fn queueing_touches_neither_the_store_nor_a_vendor() {
        let site = empty_site("queue-writes-nothing");
        let root = site.store_root.clone();
        let before = std::fs::read_dir(&root).expect("the root exists").count();
        let (code, headers, body) =
            queue(axum::extract::State(Loaded::clone(&site)), LEGAL.to_owned()).await;
        assert_eq!(code, axum::http::StatusCode::NOT_IMPLEMENTED, "{body}");
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        let after = std::fs::read_dir(&root).expect("the root exists").count();
        assert_eq!(
            before,
            after,
            "a route that answers must not have written to {}",
            root.display()
        );
        assert!(
            !site.autopilot.seat_held(),
            "the pull seat was taken and not released, which is a pull that started"
        );
    }

    /// A status carrying two feeds, one of which may be halted.
    fn surveyed(halted: &[&str]) -> Status {
        Status {
            phase: Phase::Running,
            detail: String::from("fetching Groww · 2020-01"),
            cursor: String::from("2020-01"),
            feeds: ["Dhan", "Groww"]
                .into_iter()
                .map(|feed| FeedReport {
                    feed: feed.to_owned(),
                    month: String::from("2020-01"),
                    window: String::from("2020-01-01..=2020-01-31"),
                    behind: 2,
                    halted: if halted.contains(&feed) {
                        format!("{feed} said no")
                    } else {
                        String::new()
                    },
                    ..FeedReport::default()
                })
                .collect(),
            ..Status::default()
        }
    }

    /// The payload answers the question the route is named for, from the last
    /// survey and from nothing else.
    #[test]
    fn the_status_reports_the_month_the_window_and_what_is_behind() {
        let json = waiting_json(&surveyed(&[]), false, true);
        assert!(json.contains(r#""surveyed":true"#), "{json}");
        // `waiting`, NOT `running`, and that is `Status::state`'s own rule
        // rather than a slip here: a phase of `Running` with nothing in flight
        // is between two units, and saying `running` would claim a socket that
        // is not open. The in-flight case is asserted below.
        assert!(json.contains(r#""state":"waiting""#), "{json}");
        assert!(json.contains(r#""pull_seat":"held""#), "{json}");
        assert!(json.contains(r#""month":"2020-01""#), "{json}");
        assert!(
            json.contains(r#""window":"2020-01-01..=2020-01-31""#),
            "{json}"
        );
        assert!(json.contains(r#""behind":2"#), "{json}");
        assert!(json.contains(r#""attempts_max":3"#), "{json}");
        assert!(json.contains(r#""feed":"Groww""#), "{json}");
        assert!(json.contains(r#""in_flight":null"#), "{json}");
    }

    /// **Four states, four different sentences, and not one of them is an
    /// empty list meaning "nothing is outstanding".**
    #[test]
    fn what_the_sweep_waits_on_is_named_in_every_state() {
        // 1. The operator's own pause outranks everything.
        let paused = waiting_json(&surveyed(&[]), true, false);
        assert!(paused.contains("the operator's pause"), "{paused}");
        assert!(paused.contains(r#""paused":true"#), "{paused}");

        // 2. Every feed halted names the restart, because a resume does not
        //    clear one.
        let dead = waiting_json(&surveyed(&["Dhan", "Groww"]), false, false);
        assert!(dead.contains("every feed is halted"), "{dead}");
        assert!(dead.contains("restarting the server"), "{dead}");
        assert!(dead.contains("Dhan, Groww"), "{dead}");

        // 3. One halted feed is NOT "every feed", and the backfill's own
        //    sentence is what stands.
        let half = waiting_json(&surveyed(&["Dhan"]), false, false);
        assert!(half.contains("fetching Groww"), "{half}");
        assert!(!half.contains("every feed is halted"), "{half}");

        // 4. Nothing surveyed, and the two reasons for it are different facts.
        let never = waiting_json(&Status::default(), false, false);
        assert!(never.contains(r#""surveyed":false"#), "{never}");
        assert!(never.contains("No round has finished"), "{never}");
        assert!(
            never.contains("/store.json"),
            "and it names the route that does read the store: {never}"
        );
        let stopped = waiting_json(
            &Status {
                phase: Phase::Halted,
                detail: String::from("the clock is unusable"),
                ..Status::default()
            },
            false,
            false,
        );
        assert!(
            stopped.contains("the backfill task is not running"),
            "{stopped}"
        );
        assert!(stopped.contains("the clock is unusable"), "{stopped}");
    }

    /// The route answers over a real site, and an unreadable status refuses
    /// rather than reporting an empty list.
    #[tokio::test]
    async fn the_status_route_answers_and_refuses_an_unreadable_one() {
        let site = empty_site("status-route");
        let (code, headers, body) = status_json(axum::extract::State(Loaded::clone(&site))).await;
        assert_eq!(code, axum::http::StatusCode::OK, "{body}");
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        assert!(body.contains(r#""surveyed":false"#), "{body}");
        assert!(body.contains(r#""state":"starting""#), "{body}");

        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            site.autopilot.publish(|_| panic!("a publisher panicked"));
        }));
        assert!(poisoned.is_err(), "the panic has to reach the lock");
        let (code, _, body) = status_json(axum::extract::State(Loaded::clone(&site))).await;
        assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE, "{body}");
        assert!(body.contains("poisoned"), "{body}");
        assert!(
            !body.contains(r#""waiting_on":[]"#),
            "an empty list would read as \"nothing is outstanding\": {body}"
        );
    }

    /// One HTTP exchange on a fresh connection, blocking on its own thread —
    /// the shape `server.rs`'s own route tests use, for the reason they give:
    /// it needs nothing from `tokio` this crate does not already have.
    async fn exchange(addr: std::net::SocketAddr, request: String) -> String {
        tokio::task::spawn_blocking(move || {
            use std::io::{Read as _, Write as _};
            let mut socket = std::net::TcpStream::connect(addr).expect("connect");
            socket.write_all(request.as_bytes()).expect("write");
            let mut buf = String::new();
            socket.read_to_string(&mut buf).expect("read");
            buf
        })
        .await
        .expect("the client thread must not panic")
    }

    /// **The three routes are registered, and the one that could have shadowed
    /// the front end does not.**
    ///
    /// The last assertion is the load-bearing one. `POST /autopilot` at the
    /// bare path would make `GET /autopilot` answer 405 — axum's method router
    /// answers a matched path itself and never reaches `Router::fallback` — and
    /// that is the operator's autopilot page. The control lives at
    /// `/autopilot/control` because of it, and this is what stops anyone
    /// "tidying" it back.
    #[tokio::test]
    async fn the_three_routes_answer_and_none_of_them_shadows_the_front_end() {
        let site = empty_site("routes-registered");
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let stopper = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let stop_addr = stopper.local_addr().expect("addr");
        let served = tokio::spawn(crate::server::serve(
            listener,
            crate::server::router_serving(
                site,
                std::sync::Arc::new(crate::assets::Assets::new(&crate::scratch::path(
                    "ingest-routes-front",
                ))),
            ),
            Box::pin(async move { stopper.accept().await.map(|_| ()) }),
        ));

        let get = |path: &str| {
            exchange(
                addr,
                format!("GET {path} HTTP/1.1\r\nHost: t\r\nConnection: close\r\n\r\n"),
            )
        };
        let post = |path: &str, form: &'static str| {
            exchange(
                addr,
                format!(
                    "POST {path} HTTP/1.1\r\nHost: t\r\n\
                     Content-Type: application/x-www-form-urlencoded\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{form}",
                    form.len()
                ),
            )
        };

        let status = get("/ingest/status.json").await;
        assert!(status.contains("200 OK"), "{status}");
        assert!(status.contains("application/json"), "{status}");
        assert!(status.contains(r#""blocked_by""#), "{status}");

        let queued = post("/ingest/queue", LEGAL).await;
        assert!(queued.contains("501"), "{queued}");
        assert!(queued.contains(r#""queued":false"#), "{queued}");

        // A GET ON THE QUEUE STARTS NOTHING AND IS NOT A ROUTE. The same rule
        // `/pull/spot` is held to: a crawler follows links.
        let crawled = get("/ingest/queue").await;
        assert!(crawled.contains("405"), "{crawled}");

        let control = post("/autopilot/control", "action=stop").await;
        assert!(control.contains("200 OK"), "{control}");
        assert!(control.contains(r#""accepted":true"#), "{control}");

        let refused = post("/autopilot/control", "action=go").await;
        assert!(refused.contains("400"), "{refused}");
        assert!(refused.contains("start, stop, resume"), "{refused}");

        // THE PAGE IS STILL THE PAGE. Not 405 — whatever the asset layer says
        // about a front end that was never built, it is the front end saying it.
        let page = get("/autopilot").await;
        assert!(
            !page.contains("405"),
            "GET /autopilot must still reach the front end: {page}"
        );

        let _ = std::net::TcpStream::connect(stop_addr);
        served
            .await
            .expect("task")
            .expect("a graceful shutdown is not a failure");
    }

    /// The cell in flight travels, because "waiting on the vendor for X" is the
    /// most common answer of all.
    #[test]
    fn the_cell_in_flight_is_named_when_there_is_one() {
        let mut status = surveyed(&[]);
        status.now = Some(crate::autopilot::InFlight {
            instrument: String::from("NSE-NIFTY"),
            month: String::from("2020-01"),
            timeframe: String::from("1min"),
            feed: String::from("Groww"),
            since: std::time::Instant::now(),
            index: 3,
            of: 9,
        });
        let json = waiting_json(&status, false, true);
        assert!(
            json.contains(r#""state":"running""#),
            "a cell on the wire is what makes it running: {json}"
        );
        assert!(json.contains(r#""instrument":"NSE-NIFTY""#), "{json}");
        assert!(json.contains(r#""index":3"#), "{json}");
        assert!(json.contains(r#""of":9"#), "{json}");
    }
}
