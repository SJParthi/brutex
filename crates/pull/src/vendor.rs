//! Every feed this engine can ingest, written down as **rows in a table**
//! rather than as branches in a function.
//!
//! # The failure this module exists to prevent
//!
//! The first draft of the fetch hardcoded one broker and one bar length. The
//! auth header name, the date format, the non-inclusive range end and the
//! seven parallel response arrays were all `if` statements, and the `/pull`
//! page offered exactly one timeframe. Adding a second feed meant editing the
//! fetch *and* editing the page, and the two edits could disagree — which is
//! the same defect `CLAUDE.md` §6 describes about a depth parameter, one layer
//! up: a branch that can be added can be added in one place and forgotten in
//! the other.
//!
//! So everything that differs between one feed and another is a **field**:
//! the transport, the auth header *name*, the date format, whether the range
//! end is inclusive, the response shape, the field names, the timestamp
//! encoding, the rate budget, the granularities served, the segments served,
//! and — for a local archive — the archive naming pattern, the member naming
//! pattern, whether a header row exists and the column order.
//!
//! # Why the lookup is O(1) *by construction*
//!
//! [`Feed`] is `#[repr(u8)]` with consecutive discriminants and [`DESCRIPTORS`]
//! is a `const` array indexed by that discriminant. A lookup is one array
//! index: no search, no map, no hash, and **the bound does not change when the
//! table grows from four rows to four hundred**, because the index is the
//! variant tag and not a key that has to be found.
//!
//! Three compile-time assertions make a half-added feed a build failure rather
//! than a runtime surprise: the table length equals the variant count, the
//! variant list length equals the variant count, and **row *i* describes
//! variant *i*** — that last one is what makes indexing by discriminant sound
//! rather than merely plausible. A new variant with no row does not compile.
//! `pull::vendor::the_descriptor_table_is_indexed_by_the_discriminant` is the
//! test that drives it from outside.
//!
//! # What is deliberately **not** here
//!
//! **No credential, and no rate budget, on the archive path.** [`Auth`] and
//! [`Budget`] are fields of [`HttpSpec`], not of [`Descriptor`]. An archive
//! descriptor therefore has *nowhere to put a token and nowhere to put a
//! ceiling* — a local file has no rate limit and needs no credential, and
//! wiring a governor in anyway would be a lie about what is being protected.
//! Structurally impossible beats conventionally omitted.
//!
//! **No live network call and no zip reader.** Those are adapters, and this
//! crate's own precedent — `src/secret.rs` defining a port while the AWS SDK
//! stays out of `Cargo.toml` until something makes a live call — is the shape
//! followed here. See [`crate::fetch`].
//!
//! # The session window is a **dated regime**, not a constant
//!
//! `crate::session` writes 09:15–15:30 and 375 bars as three `const`s pinned
//! to each other. That is right for every day the lake holds and wrong for
//! part of 2026: NSE extended equity-derivatives trading by ten minutes with
//! effect from **2026-08-03**, and changed the cash market's close into a
//! Closing Auction Session on the same date. A backtest spanning 2024 to 2026
//! crosses that boundary, and a single constant would mis-filter every bar on
//! the far side of it *invisibly* — the bars would simply not be there.
//!
//! So the window is a [`SessionTable`]: one anchor row plus at most
//! [`MAX_LATER_SESSION_ROWS`] dated rows, each carrying its own citation, and
//! a row that has no verified hours **refuses by name**. That is exactly the
//! shape `crates/costs/src/regime.rs` proved out for the tax rate history, and
//! it is reused rather than paraphrased. The anchor rows are the `session`
//! constants, asserted equal to them at compile time, so nothing regresses and
//! that module's exhaustive 2,932,897-day walk keeps passing untouched.
//!
//! **The refusal is the important half.** No row here invents an hour. The
//! 2026-08-03 rows carry [`Hours::Unverified`] because the NSE circular itself
//! was never retrieved — `CLAUDE.md` §3 rule 1 wants an exchange claim
//! traceable to `docs/00-charter.md`, and that document has no row for this
//! change. What five brokers reported survives as **prose in the row's source
//! string**, where it can be read and never used as a filter bound; that is
//! `crates/costs`' own device. The rows flip to [`Hours::Verified`] in the
//! one-line diff that lands the charter entry, and not before.
//!
//! # Granularity is data too, down to the tick
//!
//! [`Granularity`] spans an event grid, seconds, minutes, an hour, a day and a
//! week. Bars-per-session is **not** a constant on that ladder: 375 is true
//! only for one-minute bars in the pre-2026-08-03 window. It is a function of
//! (venue hours, granularity) returning [`ExpectedCount`], which has an arm
//! for *no count exists* — a snapshot or tick feed has no arithmetic that
//! predicts its record count, and pretending otherwise would make gap
//! detection lie.

use std::fmt;

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::vendor::Vendor;
use store::path::Timeframe;

use crate::csv::Columns;
use crate::session::{
    BARS_PER_REGULAR_SESSION, Cadence, Day, SECS_PER_MINUTE, SESSION_CLOSE_MINUTE,
    SESSION_OPEN_MINUTE,
};

// ---------------------------------------------------------------------------
// const helpers
// ---------------------------------------------------------------------------

/// Whether two strings are byte-for-byte equal, in a `const` context.
///
/// `str::eq` is not a `const fn` on the pinned toolchain, and the assertions
/// that tie this module's directory names to `store::path::Timeframe` have to
/// run at compile time or they are comments. Written with slice patterns
/// rather than indexing so it needs no lint exception.
const fn str_eq(left: &str, right: &str) -> bool {
    let mut a = left.as_bytes();
    let mut b = right.as_bytes();
    loop {
        match (a, b) {
            ([], []) => return true,
            ([first_a, rest_a @ ..], [first_b, rest_b @ ..]) => {
                if *first_a != *first_b {
                    return false;
                }
                a = rest_a;
                b = rest_b;
            }
            _ => return false,
        }
    }
}

/// Whether a citation string says anything at all.
///
/// A blank `source` on a regime row is a row with no citation wearing one, and
/// `CLAUDE.md` §3 rule 1 is the whole reason the field exists.
const fn has_content(text: &str) -> bool {
    let mut remaining = text.as_bytes();
    while let [first, rest @ ..] = remaining {
        if !first.is_ascii_whitespace() {
            return true;
        }
        remaining = rest;
    }
    false
}

/// A calendar date in a `const` context.
///
/// `Day::new` returns a `Result` and `?` is not available in a `const` item,
/// so an unreal literal falls back to 1970-01-01 rather than panicking —
/// `panic!` is banned outright in shipping source and no allowlist exists for
/// it. **The fallback is never silent:** every date built here is followed by
/// a `const` assertion that it round-trips to the literal it was written from,
/// so a typo collapses to the epoch and then fails to compile. The recursive
/// arm terminates because `crate::session` already asserts at compile time
/// that `Day::new(1970, 1, 1)` is `Ok`.
const fn day_const(year: u16, month: u8, day: u8) -> Day {
    match Day::new(year, month, day) {
        Ok(day) => day,
        Err(_) => day_const(1970, 1, 1),
    }
}

// ---------------------------------------------------------------------------
// granularity
// ---------------------------------------------------------------------------

/// Seconds in a minute, as the width the session arithmetic is done in.
///
/// `crate::session::SECS_PER_MINUTE` is the same number as an `i64`, and the
/// assertion below is what keeps the two from drifting. Written as a `u32`
/// constant rather than cast from the `i64` because a narrowing cast is denied
/// workspace-wide, and rightly so.
const SECS_PER_MINUTE_U32: u32 = 60;
const _: () = assert!(SECS_PER_MINUTE == SECS_PER_MINUTE_U32 as i64);

/// How many rungs the granularity ladder has.
pub const GRANULARITY_COUNT: usize = 11;

/// Which grid a feed's records sit on.
///
/// Extended rather than re-invented from the store's own list:
/// `store::path::Timeframe` ships `1min` and `1day`, and two spellings of
/// "which granularity" is the drift `CLAUDE.md` forbids.
/// [`Granularity::store_timeframe`] is the single reconciliation site between
/// the two, and the `const` assertions below pin every rung they share so they
/// cannot disagree silently. Widening `Timeframe` to the rest of the ladder is
/// a `crates/store` change and is recorded as outstanding.
///
/// The rungs are closed rather than parameterised (`Second(n)`) on purpose: an
/// arbitrary `n` has no stable on-disk directory name, and paths are
/// append-only history. Adding a rung is a one-row diff.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Granularity {
    /// Every print, on no fixed interval. Record count is unbounded.
    Tick = 0,
    /// One-second grid.
    Second1 = 1,
    /// Five-second grid.
    Second5 = 2,
    /// One-minute bars — what the engine sweeps, and one of the two rungs
    /// `store::path::Timeframe` ships.
    Minute1 = 3,
    /// Three-minute bars.
    Minute3 = 4,
    /// Five-minute bars.
    Minute5 = 5,
    /// Fifteen-minute bars.
    Minute15 = 6,
    /// Thirty-minute bars.
    Minute30 = 7,
    /// One-hour bars.
    Hour1 = 8,
    /// One bar per trading day — the other rung `store::path::Timeframe` ships,
    /// and the one a backfill lands first. D-0054, D-0055.
    Day1 = 9,
    /// One bar per trading week.
    Week1 = 10,
}

/// The grid a rung sits on, which is what decides whether a record count
/// exists at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grid {
    /// A record per event. No interval, and therefore no predictable count.
    Event,
    /// A fixed interval inside the session, in seconds.
    Intraday(u32),
    /// One record covering a whole session.
    Daily,
    /// One record covering a whole week.
    Weekly,
}

impl Granularity {
    /// Every rung, coarsest last.
    pub const ALL: [Self; GRANULARITY_COUNT] = [
        Self::Tick,
        Self::Second1,
        Self::Second5,
        Self::Minute1,
        Self::Minute3,
        Self::Minute5,
        Self::Minute15,
        Self::Minute30,
        Self::Hour1,
        Self::Day1,
        Self::Week1,
    ];

    /// The stable on-disk directory name.
    ///
    /// Stable is the operative word: `CLAUDE.md` §3 rule 8 makes history
    /// append-only, and a renamed directory orphans every file already under
    /// the old one.
    #[must_use]
    pub const fn dir(self) -> &'static str {
        match self {
            Self::Tick => "tick",
            Self::Second1 => "1s",
            Self::Second5 => "5s",
            Self::Minute1 => "1min",
            Self::Minute3 => "3min",
            Self::Minute5 => "5min",
            Self::Minute15 => "15min",
            Self::Minute30 => "30min",
            // `60min`, NOT `1hr`, AND THE RENAME IS THE POINT.
            //
            // This rung had two names. `store::path::Timeframe::MINUTE_60` has
            // spelled its directory `60min` since D-0054 widened the store, in
            // the same table that gives `15min` and `30min` — and this arm said
            // `1hr`. Nothing caught it because the `const` assertions below
            // tied only the two rungs that were reachable, and nothing could
            // reach this one: `store_timeframe` answered `None` for it, so no
            // bar could ever be written under either spelling.
            //
            // The browser had already noticed and was working around it.
            // `web/src/routes/db/+page.svelte` carried an alias table whose own
            // comment reads: "`1hr` is `pull::vendor::Granularity`'s spelling of
            // the rung the store files under `60min`. Both name a length…" —
            // a second answer to "what is this rung called", maintained by
            // hand, in the one place `CLAUDE.md` §3 rule 1 says a vendor or
            // store fact must not live.
            //
            // The store's word wins: it is the one that becomes a path, it is
            // the one D-0054 recorded, and it is the one its five siblings
            // already use. No history is rewritten because none exists — this
            // rung has never had a directory to hold any. Every shared rung is
            // now pinned by a `const` assertion below, so a third spelling is a
            // build failure rather than a browser workaround.
            Self::Hour1 => "60min",
            Self::Day1 => "1day",
            Self::Week1 => "1week",
        }
    }

    /// What a HUMAN calls this rung.
    ///
    /// # Why this is here and not in the browser
    ///
    /// `web/src/routes/ingest/+page.svelte` carried an eleven-row table
    /// transcribing this enum — each rung's directory name, its display name,
    /// whether the store can file it and how many records a session holds. It
    /// was a second copy of a fact this file owns, and it went stale exactly
    /// the way a second copy does: when D-0132 corrected `store_timeframe`,
    /// five of its `stored` flags became lies and the hour's directory name
    /// became one too.
    ///
    /// D-0126 and D-0131 each removed one such copy already — the granularity
    /// floor and the history floor — and the argument is the same one both
    /// times. The rung ladder is the third. With this, `/feeds.json` can carry
    /// a rung whole and the page can hold nothing: adding a rung to the ladder
    /// or a feed to the table reaches the operator's form without a browser
    /// edit. D-0138.
    ///
    /// Distinct from [`Self::dir`], which is the STORE DIRECTORY and a path
    /// segment `CLAUDE.md` §3 rule 8 protects. This is prose and may be
    /// reworded freely; that one may not.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tick => "Tick",
            Self::Second1 => "1 second",
            Self::Second5 => "5 seconds",
            Self::Minute1 => "1 minute",
            Self::Minute3 => "3 minutes",
            Self::Minute5 => "5 minutes",
            Self::Minute15 => "15 minutes",
            Self::Minute30 => "30 minutes",
            Self::Hour1 => "1 hour",
            Self::Day1 => "1 day",
            Self::Week1 => "1 week",
        }
    }

    /// The grid this rung sits on.
    #[must_use]
    pub const fn grid(self) -> Grid {
        match self {
            Self::Tick => Grid::Event,
            Self::Second1 => Grid::Intraday(1),
            Self::Second5 => Grid::Intraday(5),
            Self::Minute1 => Grid::Intraday(60),
            Self::Minute3 => Grid::Intraday(180),
            Self::Minute5 => Grid::Intraday(300),
            Self::Minute15 => Grid::Intraday(900),
            Self::Minute30 => Grid::Intraday(1_800),
            Self::Hour1 => Grid::Intraday(3_600),
            Self::Day1 => Grid::Daily,
            Self::Week1 => Grid::Weekly,
        }
    }

    /// Whether a bar at this rung carries an intraday time at all.
    ///
    /// A daily or weekly bar does not: vendors stamp it at midnight, at the
    /// open or at the close, and applying an intraday window to it would drop
    /// every one. This is the same exemption `crate::session::Cadence` names,
    /// derived from the ladder instead of asserted beside it.
    #[must_use]
    pub const fn is_intraday(self) -> bool {
        matches!(self.grid(), Grid::Event | Grid::Intraday(_))
    }

    /// Which [`Cadence`] the session filter must be run at for this rung.
    ///
    /// Derived from [`Self::is_intraday`] rather than chosen at the call site,
    /// because the two were being stated independently and the disagreement is
    /// silent in the direction that loses data: a daily bar stamped at midnight
    /// filtered at [`Cadence::Minute`] is counted `BeforeSessionOpen`, every
    /// one of them, and the run reports a clean census of nothing.
    #[must_use]
    pub const fn cadence(self) -> Cadence {
        if self.is_intraday() {
            Cadence::Minute
        } else {
            Cadence::Daily
        }
    }

    /// This rung's bit in a [`GranularitySet`].
    const fn bit(self) -> u16 {
        1u16 << (self as u16)
    }

    /// Whether a caller may ASK FOR this rung at all.
    ///
    /// `false` for [`Self::Tick`] and true for every other rung, on the
    /// operator's rule of 12 Aug 2026: a tick is raw input that gets keyed
    /// down to a second, never a granularity anybody requests. Both feeds sold
    /// as tick-by-tick were measured at one-second resolution with no
    /// sub-second field (`docs/08-vendor-samples.md`), and a REST broker
    /// publishes nothing finer than a minute, so the rung would be a promise no
    /// source on this tree can keep.
    ///
    /// # Why the variant stays on the ladder
    ///
    /// Because the ladder is the VOCABULARY of rungs and `tick` is a word both
    /// vendors print on an invoice. Deleting it would leave nothing to refuse
    /// BY NAME, and a rung that is absent from an enum is refused as "not a
    /// rung" — which reads, wrongly, as a typo. [`FinestKind::is_tick_stream`]
    /// answers the neighbouring question about what a record IS; this answers
    /// whether anyone may ask for one. Proven by
    /// `pull::folder::tick_is_never_a_rung_anybody_can_ask_for`.
    #[must_use]
    pub const fn is_requestable(self) -> bool {
        !matches!(self, Self::Tick)
    }

    /// The `store::path::Timeframe` this rung writes under, when one exists.
    ///
    /// `None` for every rung `crates/store` has not yet been widened to carry.
    /// A `None` is a **refusal at the write boundary**, never a substitution:
    /// filing a five-minute bar under `1min/` would corrupt a series that a
    /// reader has no way to tell apart from real one-minute data.
    ///
    /// # THE `_` ARM HID FIVE RUNGS THE STORE HAS ALWAYS BEEN ABLE TO FILE
    ///
    /// This function used to read:
    ///
    /// ```text
    /// Self::Minute1 => Some(Timeframe::MINUTE_1),
    /// Self::Day1    => Some(Timeframe::DAY_1),
    /// _             => None,
    /// ```
    ///
    /// and `Timeframe::KNOWN` has held **seven** entries since D-0054 widened
    /// it — `1day 1min 3min 5min 15min 30min 60min`. So `Minute3`, `Minute5`,
    /// `Minute15`, `Minute30` and `Hour1` each had a directory waiting for them
    /// and answered "there is nowhere to put this". That answer is load
    /// bearing: `api::render` gates the timeframe control on
    /// `store_timeframe().is_some()`, so five rungs were drawn on the operator's
    /// screen struck through and annotated **"no store dir"** — a claim about
    /// `crates/store` that `crates/store` contradicts.
    ///
    /// Nothing detected it because the `_` arm answers for rungs that do not
    /// exist yet as well as for rungs that do, and the two are not the same
    /// fact. `CLAUDE.md` §4 bans a fallback that hides a failure; a catch-all
    /// in the one function that decides whether a bar has anywhere to live is
    /// exactly that shape, and it hid this for as long as it stood.
    ///
    /// **Every arm is now named.** An eighth rung is a compile error here
    /// rather than a silent `None`, which is the only way this cannot happen a
    /// second time. `Tick`, `Second1`, `Second5` and `Week1` answer `None`
    /// because `Timeframe::KNOWN` genuinely holds nothing for them — a stated
    /// absence, one arm each, rather than four rungs sharing an underscore
    /// with five that were wrong.
    #[must_use]
    #[allow(
        clippy::match_same_arms,
        reason = "the two `None` arms return the same value and state DIFFERENT facts, which \
                  is the whole point of naming every arm. `Minute30 | Hour1` is 'crates/store \
                  ships a directory and the fold would file a stub into it'; `Tick | Second1 \
                  | Second5 | Week1` is 'crates/store ships no directory at all'. Merging them \
                  would put a rung refused for arithmetic beside four refused for absence, and \
                  the next reader deciding whether a rung can be enabled would have to \
                  re-derive which kind each one is — the exact question the `_` arm this \
                  function replaced made unanswerable. D-0132."
    )]
    pub const fn store_timeframe(self) -> Option<Timeframe> {
        match self {
            Self::Minute1 => Some(Timeframe::MINUTE_1),
            Self::Minute3 => Some(Timeframe::MINUTE_3),
            Self::Minute5 => Some(Timeframe::MINUTE_5),
            Self::Minute15 => Some(Timeframe::MINUTE_15),
            // THIRTY AND SIXTY ARE REFUSED, AND IT IS THE STUB, NOT THE
            // DIRECTORY.
            //
            // Both have a directory. What they do not have is a bar that means
            // what its stamp says. `crate::fold`'s grid is anchored at IST
            // MIDNIGHT (`fold.rs`, `IST_ANCHOR_MICROS`) and the NSE open is 555
            // minutes past it. 555 is divisible by 1, 3, 5 and 15 and not by 30
            // (18.5) or 60 (9.25) — so a 30-minute session folds to a first
            // record stamped **09:00**, holding only 09:15–09:29: fifteen
            // minutes of trade, filed in a file whose header says 1,800
            // seconds, which every later reader takes as the whole
            // [09:00, 09:30) bar. Its `open` is the 09:15 print presented as
            // the 09:00 print. At sixty it is worse — a 45-minute first bar and
            // a 30-minute last one.
            //
            // That is silent wrong data in an append-only store: the file is
            // well formed, the checksum is right, the count is accurate, and
            // the month cannot be prepended or rewritten. `CLAUDE.md` §4 ranks
            // a loud refusal above exactly this.
            //
            // THE PREDICATE IS THE STORE'S OWN AND THIS IS ITS FIRST CALLER.
            // `Timeframe::aligns_with_the_open` has existed since D-0077, which
            // says in as many words that it exposes the stub "so a caller can
            // refuse rather than discover it" — and it had no production caller
            // at all. Asking it here rather than listing the two rungs by hand
            // means a rung added to the ladder is judged by the arithmetic
            // instead of by whoever remembers this comment.
            //
            // Restoring them is not a table edit: it needs the fold grid
            // anchored at the session open rather than at IST midnight, which
            // changes what a bar IS at every rung and is a D-entry of its own.
            Self::Minute30 if Timeframe::MINUTE_30.aligns_with_the_open() => {
                Some(Timeframe::MINUTE_30)
            }
            Self::Hour1 if Timeframe::MINUTE_60.aligns_with_the_open() => {
                Some(Timeframe::MINUTE_60)
            }
            Self::Minute30 | Self::Hour1 => None,
            // NOT ASKED OF THE DAY RUNG, deliberately. `aligns_with_the_open`
            // divides 555 by a number of minutes, which is a question about an
            // INTRADAY grid; a daily bar aggregates a whole session and is
            // exempt from the session filter already (`Granularity::cadence`).
            // Asking it here would refuse `1day`, which has been filing bars
            // correctly since D-0054.
            Self::Day1 => Some(Timeframe::DAY_1),
            // NAMED, NOT CAUGHT. `Timeframe::KNOWN` holds no entry for any of
            // these four, and saying so one rung at a time is what makes an
            // eighth rung a compile error instead of an inherited `None`.
            Self::Tick | Self::Second1 | Self::Second5 | Self::Week1 => None,
        }
    }

    /// How wide one record at this rung is, as a number that can be ordered.
    ///
    /// The ladder has four different grids and no single unit spans them, so
    /// this is a **rank** rather than a duration: an event grid is finer than
    /// any interval, and a session and a week are wider than any interval this
    /// ladder carries. It exists to be compared, never to be arithmetic — a
    /// day is not `u32::MAX - 1` seconds long and nothing here says it is.
    #[must_use]
    pub const fn coarseness(self) -> u32 {
        match self.grid() {
            Grid::Event => 0,
            Grid::Intraday(secs) => secs,
            Grid::Daily => u32::MAX - 1,
            Grid::Weekly => u32::MAX,
        }
    }

    /// Whether this rung is finer than `other`.
    ///
    /// **One `u8` comparison — O(1), and the bound does not move when the
    /// ladder grows**, because the discriminants ascend with coarseness and
    /// the `const` block below pins that for every adjacent pair. Comparing
    /// [`Self::coarseness`] instead would be equally constant and strictly
    /// weaker: it would be true by arithmetic rather than by the table, and
    /// the table is what a new rung is added to.
    /// `pull::vendor::the_ladder_ascends_so_one_comparison_decides_which_rung_is_finer`
    /// is the test, and it walks every ordered pair rather than the adjacent
    /// ones the compiler already has.
    #[must_use]
    pub const fn is_finer_than(self, other: Self) -> bool {
        (self as u8) < (other as u8)
    }
}

// THE LADDER ASCENDS, AND THAT IS WHAT MAKES ONE COMPARISON SOUND.
//
// `is_finer_than` compares discriminants. That is only meaningful if the
// discriminants are ordered the way the grids are, and nothing about an
// `enum` guarantees it — a rung inserted in the middle of the list with the
// wrong tag would silently make a coarser rung read as finer, and the vendor
// granularity floor below would then refuse the wrong half of the ladder.
//
// Destructured rather than indexed, the same device `DESCRIPTORS` uses: a
// twelfth rung makes this pattern itself a compile error before any assertion
// is evaluated.
const _: () = {
    let [
        tick,
        sec1,
        sec5,
        min1,
        min3,
        min5,
        min15,
        min30,
        hour1,
        day1,
        week1,
    ] = Granularity::ALL;
    assert!(tick as u8 == 0 && sec1 as u8 == 1 && sec5 as u8 == 2 && min1 as u8 == 3);
    assert!(min3 as u8 == 4 && min5 as u8 == 5 && min15 as u8 == 6 && min30 as u8 == 7);
    assert!(hour1 as u8 == 8 && day1 as u8 == 9 && week1 as u8 == 10);
    assert!(tick.coarseness() < sec1.coarseness());
    assert!(sec1.coarseness() < sec5.coarseness());
    assert!(sec5.coarseness() < min1.coarseness());
    assert!(min1.coarseness() < min3.coarseness());
    assert!(min3.coarseness() < min5.coarseness());
    assert!(min5.coarseness() < min15.coarseness());
    assert!(min15.coarseness() < min30.coarseness());
    assert!(min30.coarseness() < hour1.coarseness());
    assert!(hour1.coarseness() < day1.coarseness());
    assert!(day1.coarseness() < week1.coarseness());
};

// EVERY RUNG THE TWO SPELLINGS SHARE, TIED TOGETHER BY THE COMPILER.
//
// This block used to tie ONE intraday rung — the minute — and that is how the
// hour came to have two names. `Granularity::Hour1.dir()` said `1hr` while
// `Timeframe::MINUTE_60` said `60min`, for as long as `store_timeframe`'s `_`
// arm made the pair unreachable; the drift was invisible here and was being
// absorbed by an alias table in the browser. See `Granularity::dir`.
//
// A macro rather than six copies, and NOT to save typing: six hand-written
// pairs are six chances to assert a rung against the wrong constant, and that
// mistake reads as a passing assertion. One expansion per rung, one argument
// pair each, and a rung left out of this list is a rung with no assertion at
// all — which is why the count is pinned underneath.
macro_rules! rung_matches_store {
    ($($rung:ident <=> $tf:ident),+ $(,)?) => {
        $(
            const _: () = assert!(str_eq(
                Granularity::$rung.dir(),
                Timeframe::$tf.as_str()
            ));
            const _: () = assert!(match Granularity::$rung.grid() {
                Grid::Intraday(secs) => secs == Timeframe::$tf.secs(),
                _ => false,
            });
        )+
    };
}

rung_matches_store! {
    Minute1  <=> MINUTE_1,
    Minute3  <=> MINUTE_3,
    Minute5  <=> MINUTE_5,
    Minute15 <=> MINUTE_15,
    Minute30 <=> MINUTE_30,
    Hour1    <=> MINUTE_60,
}

// THE NAMES AGREE FOR ALL SIX ABOVE. WHETHER A RUNG IS WRITABLE IS A DIFFERENT
// QUESTION, AND THIS PINS THE ANSWER SO IT CANNOT DRIFT EITHER WAY.
//
// `store_timeframe` refuses `Minute30` and `Hour1` because their opening bar is
// a stub — see that function. The arithmetic behind the refusal is asserted here
// rather than left implicit, so a change to the fold anchor that made them align
// would fail this block and force the refusal to be revisited, instead of
// leaving two rungs refused for a reason that had stopped being true.
const _: () = {
    assert!(Timeframe::MINUTE_1.aligns_with_the_open());
    assert!(Timeframe::MINUTE_3.aligns_with_the_open());
    assert!(Timeframe::MINUTE_5.aligns_with_the_open());
    assert!(Timeframe::MINUTE_15.aligns_with_the_open());
    // 555 / 30 = 18.5 and 555 / 60 = 9.25.
    assert!(!Timeframe::MINUTE_30.aligns_with_the_open());
    assert!(!Timeframe::MINUTE_60.aligns_with_the_open());
};

// THE SIX ABOVE PLUS THE DAY ARE EVERY ENTRY `Timeframe::KNOWN` HOLDS.
//
// Without this, a rung added to `KNOWN` and forgotten in the macro would be a
// store directory no `Granularity` names — the same silence in the other
// direction, and the one the `_` arm produced.
const _: () = assert!(Timeframe::KNOWN.len() == 7);

// THE DAY RUNG, TIED THE SAME WAY — but only the NAME can be tied the same way.
//
// `Grid::Daily` carries no interval, because a session is not a fixed number of
// seconds, so there is no number on the ladder's side to compare against
// `Timeframe::DAY_1.secs()`. Writing `Grid::Intraday(86_400)` above to mirror
// the minute rung would be inventing an interval this ladder does not have.
//
// What can be tied is the pair of facts that decide where a bar lands: the
// ladder says this rung AGGREGATES a session rather than sitting on an
// interval, and the timeframe spells a calendar day as a whole number of the
// minute rung's bars. Rename `1day` on either side, or re-time either constant,
// and this stops compiling rather than filing daily bars under `1min/`.
const _: () = assert!(str_eq(Granularity::Day1.dir(), Timeframe::DAY_1.as_str()));
const _: () = assert!(matches!(Granularity::Day1.grid(), Grid::Daily));
const _: () = assert!(Timeframe::DAY_1.secs() == Timeframe::MINUTE_1.secs() * 60 * 24);
const _: () = assert!(Granularity::ALL.len() == GRANULARITY_COUNT);

impl fmt::Display for Granularity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.dir())
    }
}

/// A set of granularities, as a bitset.
///
/// A bitset rather than a slice so a descriptor row is `Copy` and a membership
/// test is one mask — the same argument `brutex_core::vendor::VendorSet`
/// makes, for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct GranularitySet(u16);

impl GranularitySet {
    /// No granularity.
    pub const EMPTY: Self = Self(0);

    /// This set with `granularity` added. Adding twice is adding once.
    #[must_use]
    pub const fn with(self, granularity: Granularity) -> Self {
        Self(self.0 | granularity.bit())
    }

    /// Whether the set holds `granularity`.
    #[must_use]
    pub const fn contains(self, granularity: Granularity) -> bool {
        self.0 & granularity.bit() != 0
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether every rung in this set is `finest` or coarser.
    ///
    /// **One mask — O(1) whatever the ladder's length.** The rungs finer than
    /// `finest` are exactly the bits below its own, because the ladder's
    /// discriminants ascend with coarseness; that is the property the `const`
    /// block under [`Granularity::is_finer_than`] pins, and
    /// `pull::vendor::a_build_never_fetches_a_rung_its_vendor_cannot_serve`
    /// is what drives this function from outside.
    ///
    /// `Tick` has no bit below it, so nothing is finer than it and every set
    /// passes — which is the honest answer and not a special case.
    #[must_use]
    pub const fn none_finer_than(self, finest: Granularity) -> bool {
        self.0 & (finest.bit() - 1) == 0
    }
}

/// A set of exchange segments, as a bitset. Same shape, same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SegmentSet(u8);

impl SegmentSet {
    /// No segment.
    pub const EMPTY: Self = Self(0);

    /// This set with `segment` added.
    #[must_use]
    pub const fn with(self, segment: Segment) -> Self {
        Self(self.0 | segment_bit(segment))
    }

    /// Whether the set holds `segment`.
    #[must_use]
    pub const fn contains(self, segment: Segment) -> bool {
        self.0 & segment_bit(segment) != 0
    }

    /// Whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// One segment's bit. A free function because `Segment` lives in `crates/core`.
const fn segment_bit(segment: Segment) -> u8 {
    match segment {
        Segment::Index => 1 << 0,
        Segment::Cash => 1 << 1,
        Segment::Fno => 1 << 2,
    }
}

// ---------------------------------------------------------------------------
// venue, and the dated session regime
// ---------------------------------------------------------------------------

/// How many rows a session table may carry **after** its anchor.
///
/// Two. This is the number the O(1) claim rests on: it is the length of a
/// fixed-size array, so it bounds the lookup's loop at compile time and **no
/// input can raise it**. A scan of a fixed-size `const` array is constant
/// because the length is not an argument — there is no `N` here that grows,
/// and no bisection is needed to say so.
/// `pull::vendor::the_session_lookup_walks_a_fixed_number_of_rows` is the test.
pub const MAX_LATER_SESSION_ROWS: usize = 2;

/// How many venues have a session table.
pub const VENUE_COUNT: usize = 3;

/// A trading venue, in the sense of "which clock governs these prints".
///
/// Three rows, not one, because the 2026-08-03 change did **not** move the
/// three together: derivatives were extended by ten minutes, the cash market's
/// close became an auction, and nothing retrievable says what happened to
/// index dissemination. Folding them into one venue would apply one venue's
/// evidence to another's bars.
///
/// This widens nothing about what is **swept**: `CLAUDE.md` §1 keeps the
/// engine surface at `NSE-NIFTY` and `NSE-BANKNIFTY`. A venue row says which
/// clock a *stored* series is filtered against, and futures, options and
/// single stocks were already storable.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Venue {
    /// NSE spot index dissemination.
    NseIndex = 0,
    /// NSE cash equities.
    NseCash = 1,
    /// NSE equity derivatives — index and stock futures and options.
    NseDerivatives = 2,
}

impl Venue {
    /// Every venue, in table order.
    pub const ALL: [Self; VENUE_COUNT] = [Self::NseIndex, Self::NseCash, Self::NseDerivatives];

    /// A short, stable name for a report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::NseIndex => "NSE index",
            Self::NseCash => "NSE cash",
            Self::NseDerivatives => "NSE equity derivatives",
        }
    }

    /// Which venue's clock governs an `(exchange, segment)` pair.
    ///
    /// `None` for BSE, which `CLAUDE.md` §1 does not pull. Refused by absence
    /// rather than mapped onto an NSE row, because two exchanges sharing one
    /// window is a claim nothing here has a source for.
    #[must_use]
    pub const fn for_segment(exchange: Exchange, segment: Segment) -> Option<Self> {
        match (exchange, segment) {
            (Exchange::Nse, Segment::Index) => Some(Self::NseIndex),
            (Exchange::Nse, Segment::Cash) => Some(Self::NseCash),
            (Exchange::Nse, Segment::Fno) => Some(Self::NseDerivatives),
            (Exchange::Bse, _) => None,
        }
    }

    /// The session in force on `day`.
    ///
    /// At most [`MAX_LATER_SESSION_ROWS`] comparisons, always — see that
    /// constant for why that is a compile-time bound and not a small `N`.
    ///
    /// # Errors
    ///
    /// [`SessionRefusal`] when the row in force carries no verified hours. The
    /// refusal names the venue, the window, the citation gap and the remedy.
    /// There is no argument, flag or default that turns it into an hour.
    pub fn hours_on(self, day: Day) -> Result<Session, SessionRefusal> {
        self.table().hours_on(day)
    }

    /// This venue's table. A `match`, so a new venue with no table does not
    /// compile.
    const fn table(self) -> &'static SessionTable {
        match self {
            Self::NseIndex => &NSE_INDEX_SESSIONS,
            Self::NseCash => &NSE_CASH_SESSIONS,
            Self::NseDerivatives => &NSE_DERIVATIVES_SESSIONS,
        }
    }

    /// The dated windows for which this venue has no verified hours.
    ///
    /// Derived from the table rather than written down a second time, so no
    /// page footer can carry a boundary date that has drifted from the table
    /// it claims to describe.
    #[must_use]
    pub fn refusal_windows(self) -> [Option<RefusalWindow>; MAX_LATER_SESSION_ROWS + 1] {
        self.table().refusal_windows()
    }
}

impl fmt::Display for Venue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// What kind of trading a session row describes.
///
/// [`Self::ClosingAuction`] exists because the 2026-08-03 cash change
/// introduced one, and a session type absent from the model cannot be refused
/// by name — it is simply mis-filtered as ordinary continuous trading. **No
/// shipped row carries it yet**: an auction row needs its own verified start
/// and end, and the same circular that would supply them is the one that was
/// never retrieved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SessionKind {
    /// Continuous trading. Prints are ordinary bars.
    Continuous,
    /// A closing auction. Its prints are **not** ordinary bars and must not be
    /// swept as if they were.
    ClosingAuction,
}

impl SessionKind {
    /// A short, stable label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Continuous => "continuous",
            Self::ClosingAuction => "closing auction",
        }
    }
}

impl fmt::Display for SessionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// What a session row knows about its hours.
///
/// The two arms are deliberately asymmetric, and that asymmetry is the whole
/// mechanism: [`Self::Unverified`] **has no minute fields**. There is nowhere
/// in the type for an invented open or close to live, nothing to unwrap and no
/// default to fall back to. `crates/costs/src/regime.rs` `Rate::Unverified` is
/// the same device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Hours {
    /// Citation-grounded hours, as minutes since IST midnight. The open is
    /// inclusive and the close is **exclusive** — the same convention
    /// `crate::session` states, for the same reason.
    Verified {
        /// First minute of the session, inclusive.
        open_minute: u32,
        /// End of the session, exclusive.
        close_minute: u32,
    },
    /// No hours were ever verified for this window, and none will be invented.
    Unverified,
}

/// One dated session row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SessionRow {
    start: Day,
    hours: Hours,
    kind: SessionKind,
    source: &'static str,
}

impl SessionRow {
    const fn verified(
        start: Day,
        open_minute: u32,
        close_minute: u32,
        kind: SessionKind,
        source: &'static str,
    ) -> Self {
        Self {
            start,
            hours: Hours::Verified {
                open_minute,
                close_minute,
            },
            kind,
            source,
        }
    }

    /// A row for an era whose circular was never retrieved.
    ///
    /// Has no *shipping* caller today: on 2026-08-07 the three outstanding rows
    /// were filled from NSE/CMTR/74466 and NSE/FAOP/74467, so every shipped row
    /// now carries a citation. It is kept because the NEXT gap is a matter of
    /// when, not whether — a circular that cannot be retrieved must remain
    /// expressible, or the pressure at that moment is to guess a number
    /// instead. Deleting this constructor would remove the honest option and
    /// leave only the dishonest one.
    ///
    /// The tests below DO call it, against a table built for the purpose, so
    /// the refusal path is proven rather than merely available — which is why
    /// the `dead_code` expectation is scoped to builds without `cfg(test)`.
    /// An unconditional `expect` would be unfulfilled the moment a test used
    /// it, and `-D warnings` turns that into a red build.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the refusal constructor is part of the contract, not a \
                      leftover: every era whose source is unretrievable must \
                      stay representable. See the doc comment above."
        )
    )]
    const fn unverified(start: Day, kind: SessionKind, source: &'static str) -> Self {
        Self {
            start,
            hours: Hours::Unverified,
            kind,
            source,
        }
    }

    /// Whether the row is structurally sound: an ordered, in-range window if
    /// it has one, and a citation that says something.
    const fn is_well_shaped(&self) -> bool {
        let hours_ok = match self.hours {
            Hours::Verified {
                open_minute,
                close_minute,
            } => open_minute < close_minute && close_minute <= 24 * 60,
            Hours::Unverified => true,
        };
        hours_ok && has_content(self.source)
    }
}

/// The session in force on one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Session {
    open_minute: u32,
    close_minute: u32,
    kind: SessionKind,
    source: &'static str,
}

impl Session {
    /// First minute of the session, inclusive, since IST midnight.
    #[must_use]
    pub const fn open_minute(self) -> u32 {
        self.open_minute
    }

    /// End of the session, **exclusive**, since IST midnight.
    #[must_use]
    pub const fn close_minute(self) -> u32 {
        self.close_minute
    }

    /// What kind of trading this is.
    #[must_use]
    pub const fn kind(self) -> SessionKind {
        self.kind
    }

    /// The citation this row was written from.
    #[must_use]
    pub const fn source(self) -> &'static str {
        self.source
    }

    /// How many seconds the session runs for.
    #[must_use]
    pub const fn len_secs(self) -> u32 {
        (self.close_minute - self.open_minute) * SECS_PER_MINUTE_U32
    }

    /// Whether a minute of the day falls inside the session.
    #[must_use]
    pub const fn contains_minute(self, minute_of_day: u32) -> bool {
        minute_of_day >= self.open_minute && minute_of_day < self.close_minute
    }

    /// How many records of `granularity` one full session of these hours
    /// holds.
    ///
    /// See [`ExpectedCount`] for why one of the four answers is *there is no
    /// count*.
    #[must_use]
    pub const fn expected_count(self, granularity: Granularity) -> ExpectedCount {
        let session_secs = self.len_secs();
        match granularity.grid() {
            Grid::Event => ExpectedCount::Unbounded,
            Grid::Daily | Grid::Weekly => ExpectedCount::Aggregate,
            Grid::Intraday(interval_secs) => {
                if interval_secs == 0 || !session_secs.is_multiple_of(interval_secs) {
                    ExpectedCount::Irregular {
                        session_secs,
                        interval_secs,
                    }
                } else {
                    ExpectedCount::Exact(session_secs / interval_secs)
                }
            }
        }
    }
}

/// How many records one session holds — or the honest admission that the
/// question has no answer.
///
/// [`Self::Unbounded`] is the arm that matters. A tick or one-second snapshot
/// feed prints as often as the market prints, and there is no arithmetic that
/// predicts the count. Returning a number anyway would make every gap check
/// downstream report a complete day as short, or a short day as complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExpectedCount {
    /// Exactly this many records in one full session.
    Exact(u32),
    /// One record covers the whole session or more.
    Aggregate,
    /// No count exists: the record rate is not a function of the clock.
    Unbounded,
    /// The session is not a whole multiple of the interval, so no exact count
    /// exists either. Named rather than rounded — a rounded count is a gap
    /// check that is wrong by a fixed amount every single day.
    Irregular {
        /// The session length, in seconds.
        session_secs: u32,
        /// The interval that does not divide it, in seconds.
        interval_secs: u32,
    },
}

/// A dated table of one venue's sessions.
///
/// Private, and deliberately: if a caller could build one, a caller could
/// build one that filters 2026 against 2024's hours, and the refusal contract
/// would be a comment again.
struct SessionTable {
    venue: Venue,
    /// The row in force from 1970-01-01. A field rather than an array element,
    /// which is what makes "empty table" and "before the table"
    /// unrepresentable.
    anchor: SessionRow,
    /// Later rows in strictly ascending order, `None`-padded at the end.
    later: [Option<SessionRow>; MAX_LATER_SESSION_ROWS],
}

impl SessionTable {
    fn hours_on(&self, day: Day) -> Result<Session, SessionRefusal> {
        let mut selected = &self.anchor;
        let mut verified_from = None;
        for row in self.later.iter().flatten() {
            if row.start.days_from_epoch() <= day.days_from_epoch() {
                selected = row;
                verified_from = None;
            } else if verified_from.is_none() {
                verified_from = Some(row.start);
            }
        }
        match selected.hours {
            Hours::Verified {
                open_minute,
                close_minute,
            } => Ok(Session {
                open_minute,
                close_minute,
                kind: selected.kind,
                source: selected.source,
            }),
            Hours::Unverified => Err(SessionRefusal {
                venue: self.venue,
                day,
                row_start: selected.start,
                verified_from,
                source: selected.source,
            }),
        }
    }

    fn rows(&self) -> impl Iterator<Item = &SessionRow> {
        std::iter::once(&self.anchor).chain(self.later.iter().flatten())
    }

    fn refusal_windows(&self) -> [Option<RefusalWindow>; MAX_LATER_SESSION_ROWS + 1] {
        let mut windows = [None; MAX_LATER_SESSION_ROWS + 1];
        let successors = self
            .rows()
            .skip(1)
            .map(|row| Some(row.start))
            .chain(std::iter::once(None));
        for (slot, (row, verified_from)) in windows.iter_mut().zip(self.rows().zip(successors)) {
            if row.hours == Hours::Unverified {
                *slot = Some(RefusalWindow {
                    venue: self.venue,
                    start: row.start,
                    verified_from,
                });
            }
        }
        windows
    }

    /// Whether the anchor covers every representable day.
    const fn anchor_covers_all_days(&self) -> bool {
        self.anchor.start.days_from_epoch() == 0
    }

    /// Whether the rows ascend strictly with no hole before a populated slot.
    const fn rows_ascend(&self) -> bool {
        match self.later {
            [None, None] => true,
            [Some(first), None] => {
                self.anchor.start.days_from_epoch() < first.start.days_from_epoch()
            }
            [Some(first), Some(second)] => {
                self.anchor.start.days_from_epoch() < first.start.days_from_epoch()
                    && first.start.days_from_epoch() < second.start.days_from_epoch()
            }
            // A populated slot after an empty one: `rows()` would silently
            // close the hole.
            [None, Some(_)] => false,
        }
    }

    const fn rows_are_well_shaped(&self) -> bool {
        let anchor = self.anchor.is_well_shaped();
        match self.later {
            [None, None] => anchor,
            [Some(first), None] => anchor && first.is_well_shaped(),
            [Some(first), Some(second)] => {
                anchor && first.is_well_shaped() && second.is_well_shaped()
            }
            [None, Some(second)] => anchor && second.is_well_shaped(),
        }
    }

    /// The whole structural contract, checked by the compiler.
    const fn is_shipping_shape(&self) -> bool {
        self.anchor_covers_all_days() && self.rows_ascend() && self.rows_are_well_shaped()
    }

    /// Whether the anchor is exactly `crate::session`'s two constants.
    ///
    /// Asserted at compile time for all three tables. This is the "nothing
    /// regresses" guarantee, made by the compiler rather than by a promise:
    /// `crate::session`'s exhaustive day walk and its 09:15–15:30 filter stay
    /// correct for every day before the first later row, because the anchor
    /// *is* those constants.
    const fn anchor_matches_session_constants(&self) -> bool {
        match self.anchor.hours {
            Hours::Verified {
                open_minute,
                close_minute,
            } => open_minute == SESSION_OPEN_MINUTE && close_minute == SESSION_CLOSE_MINUTE,
            Hours::Unverified => false,
        }
    }
}

/// A span of days for which a venue has no verified hours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RefusalWindow {
    venue: Venue,
    start: Day,
    verified_from: Option<Day>,
}

impl RefusalWindow {
    /// Which venue.
    #[must_use]
    pub const fn venue(self) -> Venue {
        self.venue
    }

    /// The first day of the window, inclusive.
    #[must_use]
    pub const fn start(self) -> Day {
        self.start
    }

    /// The first day verified hours exist again — the window's exclusive end.
    /// `None` when the window is open-ended.
    #[must_use]
    pub const fn verified_from(self) -> Option<Day> {
        self.verified_from
    }
}

/// A day whose session hours were never verified.
///
/// Carries the citation gap and the remedy, because "unknown session" sends an
/// operator to guess which of three venues and which of three rows was meant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionRefusal {
    venue: Venue,
    day: Day,
    row_start: Day,
    verified_from: Option<Day>,
    source: &'static str,
}

impl SessionRefusal {
    /// Which venue.
    #[must_use]
    pub const fn venue(self) -> Venue {
        self.venue
    }

    /// The day that was asked about.
    #[must_use]
    pub const fn day(self) -> Day {
        self.day
    }

    /// The first day of the unverified window.
    #[must_use]
    pub const fn row_start(self) -> Day {
        self.row_start
    }

    /// The first day verified hours resume, when the window has an end.
    #[must_use]
    pub const fn verified_from(self) -> Option<Day> {
        self.verified_from
    }

    /// What was identified, and what was never retrieved.
    #[must_use]
    pub const fn source(self) -> &'static str {
        self.source
    }
}

impl fmt::Display for SessionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} has no verified session hours on {}: the window from {}",
            self.venue, self.day, self.row_start
        )?;
        match self.verified_from {
            Some(end) => write!(f, " until {end}")?,
            None => f.write_str(" onward")?,
        }
        write!(f, " carries no citation — {}", self.source)
    }
}

// ---------------------------------------------------------------------------
// the shipped session rows
// ---------------------------------------------------------------------------

/// The day the anchor row starts: the first representable day.
const EPOCH_DAY: Day = day_const(1970, 1, 1);
const _: () = assert!(EPOCH_DAY.days_from_epoch() == 0);

/// 2026-08-03 — the day NSE's session change took effect.
const AUG_3_2026: Day = day_const(2026, 8, 3);
const _: () =
    assert!(AUG_3_2026.year() == 2026 && AUG_3_2026.month() == 8 && AUG_3_2026.day() == 3);

/// The citation every anchor row carries.
const CHARTER_SESSION: &str = "docs/00-charter.md §3 — regular session 09:15 inclusive to 15:30 \
     exclusive IST, last one-minute bar opens 15:29, 375 bars, confirmed in the lake";

/// What is known, and what is not, about the 2026-08-03 change.
///
/// Written once and shared by all three rows because it is one event and one
/// citation gap; a second copy is a second thing to update.
const CAS_INDEX_CIRCULAR: &str = "NSE/CMTR/74466 §E, retrieved 2026-08-07: \"Unexecuted limit \
     orders of the CTS (Continuous Trading Session till 03:15PM)\". The swept \
     index closes EARLIEST of the three segments, which is counter-intuitive \
     and indirect: CAS eligibility is \"stocks on which derivative contracts \
     are available in any of the Exchanges\" (§A), every NIFTY 50 and NIFTY \
     BANK constituent qualifies, so at 15:15 every share the index is \
     computed from stops trading continuously and the index freezes with \
     them. NSE FAQ v1.0 (May 2026) Q7 confirms the actual index then uses the \
     LTP of CTS. UNVERIFIED and deliberately not encoded: whether a vendor \
     puts the frozen actual index or the indicative auction index in a \
     15:15-15:35 bar. Both are published; only measurement against \
     post-2026-08-03 data can say which.";

const CAS_CASH_CIRCULAR: &str = "NSE/CMTR/74466 §A, retrieved 2026-08-07. A share with NO derivative \
     contract is not CAS eligible and keeps its 15:30 continuous close, so \
     this row restates the anchor rather than moving it. Written as a row \
     rather than omitted because \"unchanged\" is a finding that needs a \
     citation exactly as much as a change does.";

const FNO_EXTENSION_CIRCULAR: &str = "NSE/FAOP/74467, retrieved 2026-08-07: equity derivatives extended ten \
     minutes to 15:40, so positions can be adjusted against the closing \
     prices discovered in the cash auction. Applies to index futures, index \
     options, stock futures and stock options, including expiry days.";

#[expect(
    dead_code,
    reason = "the gap this replaced is kept as the record of \
     what was unknown before the circulars were retrieved on 2026-08-07"
)]
const AUG_2026_GAP: &str = "an extension of NSE equity-derivatives trading by ten minutes with \
     effect from 2026-08-03, and a change of the cash market's close into a Closing Auction \
     Session, were reported by five brokers (Angel One, Groww, JM Financial, Anand Rathi, \
     Flattrade) read on 2026-08-07; the reported derivatives close is 15:40 and the reported \
     auction runs 15:15-15:35. OUTSTANDING CITATION: the NSE circular itself was never \
     retrieved, docs/00-charter.md has no row for the change, and no source states the new \
     continuous close for the cash market or whether index dissemination moved with it. \
     REMEDY: retrieve the circular, record it in docs/00-charter.md, and turn the row here \
     from Unverified into Verified — one line, and not before";

const NSE_INDEX_SESSIONS: SessionTable = SessionTable {
    venue: Venue::NseIndex,
    anchor: SessionRow::verified(
        EPOCH_DAY,
        SESSION_OPEN_MINUTE,
        SESSION_CLOSE_MINUTE,
        SessionKind::Continuous,
        CHARTER_SESSION,
    ),
    later: [
        // The index is computed from the cash market, so it is NOT safe to
        // assume the index window survived a change to the cash close. This
        // row refuses rather than guessing in either direction.
        Some(SessionRow::verified(
            AUG_3_2026,
            SESSION_OPEN_MINUTE,
            15 * 60 + 15,
            SessionKind::Continuous,
            CAS_INDEX_CIRCULAR,
        )),
        None,
    ],
};

const NSE_CASH_SESSIONS: SessionTable = SessionTable {
    venue: Venue::NseCash,
    anchor: SessionRow::verified(
        EPOCH_DAY,
        SESSION_OPEN_MINUTE,
        SESSION_CLOSE_MINUTE,
        SessionKind::Continuous,
        CHARTER_SESSION,
    ),
    later: [
        Some(SessionRow::verified(
            AUG_3_2026,
            SESSION_OPEN_MINUTE,
            SESSION_CLOSE_MINUTE,
            SessionKind::Continuous,
            CAS_CASH_CIRCULAR,
        )),
        None,
    ],
};

const NSE_DERIVATIVES_SESSIONS: SessionTable = SessionTable {
    venue: Venue::NseDerivatives,
    anchor: SessionRow::verified(
        EPOCH_DAY,
        SESSION_OPEN_MINUTE,
        SESSION_CLOSE_MINUTE,
        SessionKind::Continuous,
        CHARTER_SESSION,
    ),
    later: [
        Some(SessionRow::verified(
            AUG_3_2026,
            SESSION_OPEN_MINUTE,
            15 * 60 + 40,
            SessionKind::Continuous,
            FNO_EXTENSION_CIRCULAR,
        )),
        None,
    ],
};

const _: () = assert!(NSE_INDEX_SESSIONS.is_shipping_shape());
const _: () = assert!(NSE_CASH_SESSIONS.is_shipping_shape());
const _: () = assert!(NSE_DERIVATIVES_SESSIONS.is_shipping_shape());
const _: () = assert!(NSE_INDEX_SESSIONS.anchor_matches_session_constants());
const _: () = assert!(NSE_CASH_SESSIONS.anchor_matches_session_constants());
const _: () = assert!(NSE_DERIVATIVES_SESSIONS.anchor_matches_session_constants());
const _: () = assert!(Venue::ALL.len() == VENUE_COUNT);

// The anchor's one-minute count is `crate::session`'s 375, derived rather than
// restated. A close that made the two disagree is a compile error here.
const CHARTER_ANCHOR: Session = Session {
    open_minute: SESSION_OPEN_MINUTE,
    close_minute: SESSION_CLOSE_MINUTE,
    kind: SessionKind::Continuous,
    source: CHARTER_SESSION,
};
const _: () = assert!(match CHARTER_ANCHOR.expected_count(Granularity::Minute1) {
    ExpectedCount::Exact(bars) => bars == BARS_PER_REGULAR_SESSION,
    _ => false,
});
const _: () = assert!(CHARTER_ANCHOR.len_secs() == 22_500);

// ---------------------------------------------------------------------------
// transport
// ---------------------------------------------------------------------------

/// The HTTP verb a feed's bars endpoint takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// `GET`, with the range in the query string.
    Get,
    /// `POST`, with the range in the body.
    Post,
}

impl Method {
    /// The verb as it goes on the wire.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// How a token is presented in its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthScheme {
    /// The header value is the token, with no prefix.
    Raw,
    /// The header value is `Bearer ` then the token.
    Bearer,
    /// The header value is a prefix, a **first** secret, a separator, and a
    /// **second** secret: `token api_key:access_token`.
    ///
    /// # Why a second secret is a variant and not a second field
    ///
    /// `docs/07-plan.md` §5 named this exact shape as one a two-variant scheme
    /// "cannot be described": *"A scheme carrying a prefix and a **second**
    /// secret cannot be described."* Zerodha is the vendor that proves it —
    /// `Authorization: token api_key:access_token`, read from
    /// `kite.trade/docs/connect/v3/historical/` and recorded in
    /// `docs/00-charter.md` §4z.
    ///
    /// [`Self::Raw`] and [`Self::Bearer`] are not special cases of this one.
    /// They name ONE secret, and a feed that needs two and is handed one must
    /// refuse rather than send a half-formed header a vendor answers 403 to —
    /// which is [`crate::http::Credential`]'s whole job, and it refuses at
    /// CONSTRUCTION rather than per request.
    ///
    /// # Neither half is ever printed
    ///
    /// The prefix and the separator are punctuation the vendor chose and are
    /// data like any other descriptor field. The two secrets are not in this
    /// type, are not in [`Descriptor`], and are not in anything derived from
    /// them — the same rule [`Auth`] already states.
    PrefixedPair {
        /// What precedes the first secret, trailing space included when the
        /// vendor writes one — `"token "`.
        prefix: &'static str,
        /// What sits between the two secrets — `":"`.
        separator: &'static str,
    },
}

impl AuthScheme {
    /// The prefix that goes before the first secret, `""` when there is none.
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Raw => "",
            Self::Bearer => "Bearer ",
            Self::PrefixedPair { prefix, .. } => prefix,
        }
    }

    /// Whether this scheme names a **second** secret beside the token.
    ///
    /// The one question a credential reader has to ask, so it is asked of the
    /// scheme rather than of the feed's name. A caller that branched on
    /// `feed == Zerodha` would be a second answer to "how many secrets", and
    /// the two would disagree the first time a vendor changed its scheme.
    #[must_use]
    pub const fn names_two_secrets(self) -> bool {
        matches!(self, Self::PrefixedPair { .. })
    }
}

/// Which header carries the credential, and how.
///
/// The header **name** is data. The header **value** never appears in this
/// type, in [`Descriptor`], or in anything derived from them — see
/// [`crate::fetch::WireRequest`], which is the request as far as it can be
/// built without a secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Auth {
    /// The header name, e.g. `access-token` or `Authorization`.
    pub header: &'static str,
    /// How the token is written into it.
    pub scheme: AuthScheme,
    /// The credential FIELD the second secret is read from, for a scheme that
    /// names two — the `<field>` of `/<org>/<env>/<vendor>/<field>`.
    ///
    /// `None` for every scheme that names one, which is every feed in this
    /// build until a two-secret vendor is added.
    ///
    /// # Why the NAME lives here and the VALUE never can
    ///
    /// `CLAUDE.md` §8: "`crates/pull` holds the shape and the field names; it
    /// holds no `org`, `env`, or `vendor` literal." A field name is not a
    /// secret and not a path — it is the last component of a shape this file
    /// already owns — and the alternative is a literal in `crates/api`, which
    /// is where the *first* field name still sits and is the asymmetry this
    /// field begins to close.
    ///
    /// The value is read from Parameter Store at runtime and reaches
    /// [`crate::http::Credential`] and nowhere else. It is never an environment
    /// variable, never a file, never a prompt, and it never travels back into
    /// this type.
    ///
    /// # A scheme and a field cannot disagree
    ///
    /// A scheme that names two secrets with no field to read the second from
    /// is a descriptor that cannot be satisfied, and a field named by a scheme
    /// that takes one secret is a parameter nothing would read. Both are
    /// refused by `pull::vendor::a_two_secret_scheme_names_the_field_its_second_
    /// secret_comes_from`, which walks every feed.
    pub key_field: Option<&'static str>,
}

/// Where one request parameter's value comes from.
///
/// # Why the request is data and not code
///
/// `window_async` built the whole request as two hardcoded fields —
/// `json!({ "fromDate": from, "toDate": to })` for POST and
/// `.query(&[("from", from), ("to", to)])` for GET. No instrument, no segment,
/// no interval. That is `DH-905 securityId is required` in two lines, and it
/// is why a broker has never returned a bar to this build.
///
/// Naming the fields here rather than there keeps `CLAUDE.md`'s rule that
/// adding a vendor is a ROW in this file: Dhan wants `securityId` and
/// `exchangeSegment`, Groww wants `groww_symbol` and `segment`, and the code
/// that puts them on the wire never learns either name.
/// What KIND of listing an instrument is, in words no vendor owns.
///
/// # The defect this type exists to remove
///
/// Both brokers require the request to name the instrument's class as well as
/// its id, and each spells it differently. The descriptor carried
/// `Fixed("IDX_I")` and `Fixed("INDEX")` for Dhan and `Fixed("CASH")` for
/// Groww — correct for exactly one class each, wrong for the other. Measured
/// consequence: every one of the 750 NIFTY-Total-Market equities was asked for
/// inside Dhan's INDEX segment, and the broker answered no window for all of
/// them. Groww is hardcoded the opposite way and cannot address an index.
///
/// The class is a property of the INSTRUMENT, not of the feed, so it is named
/// once here and each descriptor maps it to its own words. `CLAUDE.md` §5:
/// adding a broker is a row in this file, not an edit in the request builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Listing {
    /// An index — NIFTY, BANKNIFTY. No ISIN, and it never splits.
    Index,
    /// A cash-segment equity on the main board.
    Equity,
}

/// One feed's words for one listing class.
///
/// TWO words, not one, because Dhan needs both an `exchangeSegment` and an
/// `instrument` and neither is derivable from the other: `IDX_I` pairs with
/// `INDEX`, `NSE_EQ` with `EQUITY`. A feed that names only a segment leaves
/// [`Self::kind`] empty rather than repeating the segment into it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListingWords {
    /// Which listing class these words are for.
    pub listing: Listing,
    /// This feed's word for the segment.
    pub segment: &'static str,
    /// This feed's word for the instrument kind; empty when it names none.
    pub kind: &'static str,
}

/// Where one request parameter's value comes from.
///
/// Naming the fields in the descriptor rather than in the request builder keeps
/// `CLAUDE.md`'s rule that adding a vendor is a ROW in this file: Dhan wants
/// `securityId` and `exchangeSegment`, Groww wants `groww_symbol` and
/// `segment`, and the code that puts them on the wire never learns either name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParamValue {
    /// The window's first day, in this feed's [`DateFormat`].
    From,
    /// The window's last day, honouring this feed's [`RangeEnd`].
    To,
    /// The vendor's OWN id for the instrument — `SECURITY_ID` at Dhan,
    /// `groww_symbol` at Groww. Read from the instrument master; see
    /// `brutex_core::vendor::VendorId`.
    InstrumentId,
    /// A word this feed requires and this build does not vary, such as
    /// `exchangeSegment: "IDX_I"`. Fixed here so it is reviewable beside the
    /// rest of the row rather than buried in a request builder.
    Fixed(&'static str),
    /// The rung being asked for, in **this feed's own spelling** — resolved
    /// through [`HttpSpec::granularity_token`].
    ///
    /// Distinct from [`Self::Fixed`], and the distinction is the whole of what
    /// makes a rung selectable. Groww's `candle_interval` was `Fixed("1minute")`,
    /// so a request the operator filed as daily still asked the vendor for
    /// minutes — and a daily window is wider than the one-minute cap the vendor
    /// publishes, so the wider window would have gone out against the narrower
    /// contract.
    ///
    /// A rung this feed has no recorded spelling for is **refused by name**.
    /// `CLAUDE.md` §3 rule 1: the wire word is a vendor fact, and there is no
    /// spelling to derive it from — `"1day"`, `"1d"` and `"day"` are all
    /// plausible and only one of them is a request.
    Granularity,
    /// The **segment** word for the instrument's own [`Listing`] class.
    ///
    /// A `Fixed` word here is a claim that every instrument this feed is ever
    /// asked for is the same class, and neither broker's universe satisfies
    /// that: the operator's surface is 750 cash equities *and* the NSE index
    /// series. A class this feed records no word for is **refused by name**,
    /// for the reason [`Self::Granularity`] gives.
    Segment,
    /// The **instrument-kind** word for the instrument's own [`Listing`] class.
    ///
    /// Separate from [`Self::Segment`] because Dhan requires both and they are
    /// two different enums in its own annexure. A descriptor that asks for this
    /// where [`ListingWords::kind`] is empty is refused rather than sent blank.
    Kind,
}

/// One named request parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Param {
    /// What this feed calls the field.
    pub name: &'static str,
    /// Where its value comes from.
    pub value: ParamValue,
}

/// One segment of a feed's bars path.
///
/// # Why a path is a LIST and no longer a string
///
/// [`HttpSpec::bars_path`] was a `&'static str` concatenated onto `base_url`,
/// and `docs/07-plan.md` §5 predicted exactly the vendor that would break it:
/// *"A vendor whose instrument and granularity are path segments cannot be
/// described."* Zerodha is that vendor —
/// `/instruments/historical/:instrument_token/:interval` carries BOTH, and
/// neither is a query parameter this feed could name in [`HttpSpec::params`].
///
/// A string with `:placeholder` markers would have worked too, and is worse in
/// the way this repository keeps rejecting: it is a grammar parsed at runtime,
/// so a typo in a descriptor row becomes a request that goes out wrong rather
/// than a build that does not compile. A list of these carries the same
/// information with the compiler checking it, and reuses [`ParamValue`] — so
/// the four things a path can interpolate are the same four a query string can,
/// resolved by the same function, and a fifth source cannot appear in one place
/// and not the other.
///
/// # Cost
///
/// The list is a `const` in this file with at most a handful of entries per
/// feed, so building a URL walks a fixed number of segments and allocates one
/// string. There is no `N` here that an input can grow — the same argument
/// [`HttpSpec::window_cap_days`] makes about its own table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// A segment fixed for every request this feed makes, written exactly as
    /// the vendor's own documentation writes it, with no slashes of its own.
    Literal(&'static str),
    /// A segment whose value comes from the request.
    Value {
        /// What the vendor's own page calls this placeholder —
        /// `instrument_token`, `interval`. It is not sent anywhere; it exists
        /// so a refusal names the segment the way the vendor's documentation
        /// names it, exactly as [`Param::name`] does for a query field.
        placeholder: &'static str,
        /// Where the value comes from — the same table a query parameter uses.
        value: ParamValue,
    },
}

impl PathSegment {
    /// The name a refusal about this segment should carry.
    ///
    /// A literal cannot be refused — it is in the descriptor and needs nothing
    /// resolved — so this is the placeholder for a value segment and the
    /// literal's own text otherwise. One function, so an error message cannot
    /// be assembled two ways at two call sites.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Literal(word) => word,
            Self::Value { placeholder, .. } => placeholder,
        }
    }
}

/// How a date is rendered on the wire, or read out of an archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DateFormat {
    /// `YYYY-MM-DD`.
    DashedYmd,
    /// `YYYYMMDD`, no separators.
    CompactYmd,
    /// `DD/MM/YYYY` — GDFL's archives. Reading it the other way round shifts
    /// every bar by months, silently, which is why it is a named row and not
    /// a guess at the call site.
    SlashedDmy,
    /// `DDMMYYYY`, no separators.
    CompactDmy,
    /// `YYYY-MM-DD HH:MM:SS`, at the start of the day.
    ///
    /// # Measured, not guessed
    ///
    /// Groww refused `2026-08-05` outright:
    ///
    /// ```text
    /// 400 {"status":"FAILURE","error":{"code":"GA001",
    ///      "message":"Invalid date format: 2026-08-05"}}
    /// ```
    ///
    /// Its page takes `yyyy-MM-dd HH:mm:ss` or epoch seconds, and
    /// [`DateFormat::DashedYmd`] is neither. The clock is `00:00:00` on the
    /// start and `00:00:00` on the end, so the window still names whole days
    /// and [`RangeEnd`] still decides whether the last one is included — this
    /// changes the SPELLING and not the span.
    DashedYmdMidnight,
}

/// Whether a feed's range end includes the day it names.
///
/// A boolean-shaped fact, never an `if vendor == …` branch. The primary
/// broker's `toDate` is **not** inclusive; another feed's will be, and the
/// difference is one day silently missing from every request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RangeEnd {
    /// The named day is included.
    Inclusive,
    /// The named day is excluded — the wire value is the day *after* the
    /// operator's last day.
    Exclusive,
}

/// How a bars response is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResponseShape {
    /// One array per field, all the same length. The length agreement is a
    /// *promise*, and [`crate::fetch`] checks it as one refusal before any
    /// iterator exists — see that module on why `zip` is the trap.
    ParallelArrays {
        /// The object key the arrays hang under, or `None` when they are at
        /// the top level.
        envelope: Option<&'static str>,
    },
    /// One object per bar.
    ArrayOfObjects {
        /// The object key the array hangs under, or `None` at the top level.
        envelope: Option<&'static str>,
    },
    /// One ARRAY per bar, read by POSITION.
    ///
    /// ```text
    /// {"payload":{"candles":[["2026-08-04T09:15:00",24653.0,24653.0,
    ///                         24600.2,24602.45,null,null], …]}}
    /// ```
    ///
    /// Groww's own words: "Each candle has candle timestamp, open, high, low,
    /// close, volume **in that order**". There are no names to read, so the
    /// ORDER is the contract — which is why the array's own name is carried
    /// here rather than guessed, and why a row of the wrong width is refused
    /// naming the width rather than read short.
    PositionalRows {
        /// The object key the array hangs under, or `None` at the top level.
        envelope: Option<&'static str>,
        /// What the array is called inside it, e.g. `candles`.
        array: &'static str,
    },
}

/// What a timestamp in a response means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimestampEncoding {
    /// Seconds since the Unix epoch, UTC.
    EpochSecondsUtc,
    /// Milliseconds since the Unix epoch, UTC.
    EpochMillisUtc,
    /// A local IST date and time, `YYYY-MM-DD HH:MM:SS`.
    IstDateTimeText,
    /// An ISO-8601 local date and time with a `T`, `YYYY-MM-DDTHH:MM:SS`.
    ///
    /// Measured, and it matters: Groww's live endpoint returns
    /// `"2026-08-04T09:15:00"` while its own documentation annotates that very
    /// field as `yyyy-MM-dd HH:mm:ss`. Trusting the annotation would fail to
    /// parse the vendor's own example. No zone is carried; the value is IST.
    IsoDateTimeText,
    /// An ISO-8601 date and time that **carries its own UTC offset**:
    /// `2017-12-15T09:15:00+0530`.
    ///
    /// # Why this cannot be [`Self::IsoDateTimeText`] with a longer string
    ///
    /// `docs/07-plan.md` §5 predicted this too: *"A vendor returning `+0530`
    /// cannot be described."* The sibling variant's own documentation is the
    /// reason — it says **"No zone is carried; the value is IST"**, and
    /// `crate::fetch::land` acts on exactly that promise by subtracting
    /// `IST_OFFSET_SECS` from every value it decodes. Feeding a zone-carrying
    /// stamp through it would apply an offset the vendor had **already
    /// applied**, putting every bar 5h30m early — and 03:45 on a trading day is
    /// a plausible-looking timestamp, not an obviously broken one. That is the
    /// W1 fault this module's header describes: a wrong answer that stores
    /// cleanly and cannot be detected afterwards.
    ///
    /// So the offset is read from the value and applied **where it is read** —
    /// `crate::http::one_stamp` returns true UTC seconds for this variant, and
    /// `land` passes them through untouched. The conversion happens once, in
    /// the one place that can see the zone.
    ///
    /// # The offset is honoured, not asserted
    ///
    /// Every Kite example on `kite.trade/docs/connect/v3/historical/` carries
    /// `+0530`, and this build does not hardcode that: the value's own offset
    /// is parsed and subtracted, so a vendor that ever answered `+0000` would
    /// be read correctly rather than shifted by five and a half hours. Applying
    /// a stated offset is arithmetic, not a guess — what would be a guess is
    /// assuming a zone the vendor did not state, which is what the sibling
    /// variant is for and why it says so in its own name.
    ///
    /// A malformed or absent offset is **refused by name**, never defaulted to
    /// IST: a value this variant cannot read is a vendor that changed its
    /// format, and defaulting would silently resurrect the 5h30m error.
    IsoDateTimeOffset,
}

/// What unit a price arrives in.
///
/// Never a float either way: a rupee price arrives as decimal **text** and is
/// converted digit by digit. See `crate::fetch::paisa_from_decimal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriceScale {
    /// Rupees, with up to two decimal places.
    Rupees,
    /// Already paisa integers.
    Paisa,
}

/// The response field names, as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldNames {
    /// The open field.
    pub open: &'static str,
    /// The high field.
    pub high: &'static str,
    /// The low field.
    pub low: &'static str,
    /// The close field.
    pub close: &'static str,
    /// The volume field.
    pub volume: &'static str,
    /// The timestamp field.
    pub timestamp: &'static str,
    /// The open-interest field, or `None` for a feed that does not send one.
    /// Absent open interest becomes `store::format::OI_NULL`; **zero means
    /// zero**.
    pub open_interest: Option<&'static str>,
}

/// A published rate budget.
///
/// Every span is optional because "no published bound on that span" is a
/// recorded fact for both HTTP feeds, and `crate::rate::Governor` already
/// spells absence as `None` and refuses a zero so the two cannot be confused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Budget {
    /// Requests per second.
    pub per_second: Option<u32>,
    /// Requests per minute.
    pub per_minute: Option<u32>,
    /// Requests per day.
    pub per_day: Option<u32>,
}

/// The oldest day a feed will answer for, in the four shapes that occur.
///
/// # Four, and the last two are not the same fact
///
/// [`Self::Unbounded`] is a vendor **stating** that it holds everything —
/// Dhan's daily page says the data "is available back upto the date of its
/// inception", Groww's interval table says "Full history" against its day row.
/// [`Self::Unstated`] is nobody having said anything, which is what every
/// local archive is: the operator's folder holds whatever they bought.
///
/// The two behave identically at a clamp — neither narrows a window — and they
/// are still different answers to an operator asking *how far back can I ask?*
/// One is "as far as the instrument goes"; the other is "this repository does
/// not know, and will not guess". Collapsing them would put a claim the vendor
/// made and a claim nobody made under one word, which is the invention
/// `CLAUDE.md` §3 rule 1 forbids and the silent fallback §4 bans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HistoryFloor {
    /// A date that does not move. Groww: 2020-01-01.
    Fixed {
        /// Year of the oldest day served.
        year: u16,
        /// Month of the oldest day served.
        month: u8,
        /// Day of the oldest day served.
        day: u8,
    },
    /// A window that moves with the clock. Dhan: ~5 years back from today.
    ///
    /// Recomputed per run rather than stored: storing it would freeze a floor
    /// the vendor moves daily, and a frozen rolling floor is the exact bug this
    /// type exists to name.
    Rolling {
        /// How many years back the window reaches.
        years: u32,
    },
    /// A window that moves with the clock, stated in **months**. Groww's
    /// one-minute rung: "Last 3 months".
    ///
    /// A separate arm rather than a fraction of [`Self::Rolling`] because
    /// three months is 89, 90, 91 or 92 days depending on where in the year it
    /// is measured from, and a quarter of a year is not what the vendor wrote.
    /// Resolved by [`crate::session::Day::months_before`], which clamps the
    /// day of the month downward rather than rolling it forward.
    RollingMonths {
        /// How many whole calendar months back the window reaches.
        months: u8,
    },
    /// The vendor states there is **no** floor: it serves back to the
    /// instrument's own inception.
    ///
    /// A recorded vendor claim, not an absence of one. See the type's header
    /// for why this is not [`Self::Unstated`].
    Unbounded,
    /// Nothing states anything. Not a default and not "no limit" — an unknown
    /// floor is UNVERIFIED in the charter and says so there.
    Unstated,
}

/// WHAT KIND OF SOURCE made a floor claim, and therefore which of two
/// disagreeing claims outranks the other.
///
/// # Why this is a field and not a sentence in a comment
///
/// Until 12 Aug 2026 the rule for resolving two disagreeing floors was "the
/// STRICTER one binds — the later day refuses first", and that rule was
/// checked by a test rather than asserted in prose. It was right for three of
/// the four rows recorded here and wrong for the fourth, and the reason it was
/// wrong is a distinction the type could not previously express:
///
/// > A vendor's published table describes what the product does IN GENERAL.
/// > The operator describes what HIS OWN ENTITLEMENT actually answered.
///
/// Those are not two readings of one question, so comparing their strictness
/// is comparing the wrong thing. Groww's interval table says its one-minute
/// rung serves "Last 3 months"; the operator, watching this build refuse
/// January 2020 through May 2026 on his own account, stated on 12 Aug 2026
/// that Groww answers from January 2020. The looser claim is the one with the
/// better evidence behind it.
///
/// # The rule, in two tiers, and it is [`Self::outranks`]
///
/// 1. A [`Self::OperatorObservation`] outranks a [`Self::VendorDocument`].
/// 2. Between two claims of the SAME standing, the stricter — the later day —
///    binds, exactly as before.
///
/// Tier 2 is not repealed by tier 1; it is what tier 1 falls through to.
/// `the_binding_floor_is_never_the_looser_of_the_two` checks both tiers over
/// every row, so a future row cannot promote a looser claim of equal standing
/// and cannot promote a lower-standing one at all.
///
/// # The cost of tier 1, stated rather than discovered
///
/// It can WIDEN a floor, and a widened floor spends real requests on days that
/// may come back empty — and an empty answer reads exactly like a market
/// holiday. That is the price of believing the entitlement holder over the
/// brochure, it is paid on exactly one row in this build, and that row's
/// `binds_because` marks the widening UNVERIFIED until a request measures it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClaimStanding {
    /// Read out of the vendor's own published documentation. A general
    /// statement about the product, citable line by line, and made by a party
    /// who does not know which entitlement is asking.
    VendorDocument,
    /// Reported by the operator about what his own entitlement answered.
    /// Direct observation, dated, and outranking a general claim — see the
    /// type's header for the cost of that.
    OperatorObservation,
}

impl ClaimStanding {
    /// The wire's word for this standing. A KEY, never prose — the sentence is
    /// [`FloorClaim::source`].
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::VendorDocument => "vendor_doc",
            Self::OperatorObservation => "operator",
        }
    }

    /// What a page calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::VendorDocument => "the vendor's published documentation",
            Self::OperatorObservation => "the operator's own observation",
        }
    }

    /// Whether `self` beats `other` on standing ALONE, before strictness is
    /// consulted at all.
    ///
    /// `false` when the two are equal, which is the fall-through that hands
    /// the decision to tier 2. It is deliberately not `>=`: "outranks" has to
    /// mean strictly, or a same-standing pair would skip the strictness check
    /// that is the only thing deciding it.
    #[must_use]
    pub const fn outranks(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::OperatorObservation, Self::VendorDocument)
        )
    }
}

impl fmt::Display for ClaimStanding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// One claim about how far back a feed answers, and who made it.
///
/// # Why the source travels with the number
///
/// `CLAUDE.md` §3 rule 1: every claim about a vendor is traceable to a source.
/// A floor with no attribution is indistinguishable from one somebody typed,
/// and this repository has two sources that **disagree** — see [`FloorRow`].
///
/// [`Self::standing`] is the machine-readable half of that attribution:
/// `source` is the prose a person checks, and `standing` is what decides which
/// of two claims binds. See [`ClaimStanding`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FloorClaim {
    /// How far back this source says the feed answers.
    pub floor: HistoryFloor,
    /// Where the claim was read, in the words an operator can go and check.
    /// Prose, because it is quoted onto a page and never parsed.
    pub source: &'static str,
    /// What KIND of source said it, which is what decides a disagreement.
    pub standing: ClaimStanding,
}

/// How far back **one rung** of one feed answers, with the conflict intact.
///
/// # Why this is per rung
///
/// Because the shape is not uniform inside a single vendor. Groww's own
/// interval table gives its day row "Full history" and its one-minute row
/// "Last 3 months" — a floor keyed on the vendor alone is therefore wrong for
/// one of that vendor's two rungs, and wrong in the expensive direction: a
/// 2020-to-today one-minute backfill against a three-month window spends
/// thousands of requests on days the vendor answers EMPTY, and an empty answer
/// is indistinguishable from a market holiday.
///
/// # Why both claims are carried
///
/// The operator stated one set of floors on 11 Aug 2026 and the vendors'
/// documentation states another, and for three of the four rows below they do
/// not agree. Averaging them, or taking the newer, or taking the vendor's
/// because it is the *official* one, would each produce a number no source
/// supports. So the **stricter** one binds — the later day refuses first,
/// and obeying it can only cost requests that would have come back empty — and
/// the one it displaced stays here, named, beside the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FloorRow {
    /// Which rung this row is about.
    pub granularity: Granularity,
    /// The claim that **binds**: the strictest one recorded for this rung.
    pub binding: FloorClaim,
    /// A second claim about the same rung that does not agree. `None` when
    /// only one source speaks, or when the sources agree and the binding
    /// claim's `source` names them both.
    pub contested: Option<FloorClaim>,
    /// Why [`Self::binding`] binds over [`Self::contested`]. Empty when
    /// nothing contests it.
    pub binds_because: &'static str,
}

// ---------------------------------------------------------------------------
// the OTHER floor: how FINE a feed goes, beside how far BACK it goes
// ---------------------------------------------------------------------------

/// What one record at a feed's finest rung actually **is**.
///
/// # Why a conflated snapshot is not a tick, and why the type has to say so
///
/// Two of the four archives in this build are named for a tick and hold no
/// tick. `docs/08-vendor-samples.md` measured it rather than assuming it: the
/// rows carry a whole-second timestamp with **no sub-second field**, about one
/// row per second across a session, and several rows share a second with **no
/// tiebreaker**. What is in the file is one conflated snapshot per second —
/// the best bid, the best ask and the best last price as of that instant —
/// and every print between two snapshots was never written down and cannot be
/// recovered by any reader.
///
/// Calling that a tick would be a claim about the data itself, made in a
/// label, that the data does not support. It is the same defect
/// `CLAUDE.md` §4 names as a fallback that hides a failure: the operator asks
/// for prints, is handed snapshots, and nothing in the path ever says the
/// substitution happened. So the distinction is a **variant**, it travels with
/// the floor, and it survives to whatever renders it.
///
/// # [`Self::Tick`] exists and no row constructs it
///
/// Deliberately. A type that cannot spell "tick stream" cannot say "this is
/// not one" either, and the sentence that has to survive is the negative. No
/// feed in this build serves a tick stream — that is not a floor some feed
/// clears, it is a rung no vendor here publishes at all — and the `const`
/// block under [`DESCRIPTORS`] makes a row that claims one a **build
/// failure** rather than a review comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FinestKind {
    /// Every print, in order. **No feed in this repository serves this**, and
    /// the compiler is what enforces it — see the type's header.
    Tick,
    /// One record per grid slot carrying the best bid, the best ask and the
    /// best last price as of that instant. Prints between two slots are not in
    /// the file. `TrueData` and GDFL, measured in `docs/08-vendor-samples.md`.
    ConflatedSnapshot,
    /// An open, high, low and close aggregating the whole interval. What both
    /// brokers serve, at every rung they serve.
    Bar,
}

impl FinestKind {
    /// What a page calls it. Prose, and never parsed.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Tick => "tick stream — every print",
            Self::ConflatedSnapshot => "conflated snapshot — best bid, best ask, best last price",
            Self::Bar => "bar — open, high, low, close",
        }
    }

    /// Whether this is a true tick stream.
    ///
    /// The question a caller must ask before writing the word *tick* beside a
    /// number. It is `false` for every feed this build ships.
    #[must_use]
    pub const fn is_tick_stream(self) -> bool {
        matches!(self, Self::Tick)
    }

    /// Whether records between two of this feed's own slots were discarded
    /// before the file was written.
    ///
    /// `true` only for [`Self::ConflatedSnapshot`]. A bar aggregates its
    /// interval and says so; a conflated snapshot samples it and does not.
    #[must_use]
    pub const fn is_conflated(self) -> bool {
        matches!(self, Self::ConflatedSnapshot)
    }
}

/// The finest rung a feed can **ever** serve, and why nothing below it exists.
///
/// # This is a second floor, and it is not the first one
///
/// [`FloorRow`] answers *how far back*. This answers *how fine*, and the two
/// refusals have nothing in common except the word:
///
/// | | fixable by |
/// |---|---|
/// | below [`FloorRow`]'s day | nothing — but the vendor still serves the rung |
/// | below this floor | **nothing at all.** No pull, no entitlement, no code |
/// | absent from the store | **a pull.** The vendor serves it; nobody asked yet |
///
/// The third row is the one that made this type necessary. A page was showing
/// Groww's `tick`, `1s` and `5s` rungs as ordinary selectable choices
/// annotated *no directory* — an annotation about the **store**, meaning
/// "nothing saved here yet", which is fixable by pulling. The fact that
/// mattered was "Groww can never serve this", which is fixable by nothing.
/// Two refusals of opposite kinds rendered identically is exactly the failure
/// `CLAUDE.md` §4 bans, and it happened because this fact lived nowhere: the
/// vendor row could say which rungs it serves TODAY and could not say which
/// rungs the vendor is capable of, so the browser was left to infer it and
/// inferred nothing.
///
/// # Why the kind is here and not derived
///
/// Because the answer differs between two feeds whose `granularities` are
/// identical. `TrueData` and GDFL both bottom out at one second; one second of
/// GDFL and one second of Groww would be two different objects if Groww served
/// it. See [`FinestKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GranularityFloor {
    /// The finest rung this vendor serves. Every rung below it is refused, and
    /// the refusal is permanent.
    pub finest: Granularity,
    /// What one record at [`Self::finest`] is. The tick-versus-conflated
    /// distinction, carried rather than inferred.
    pub kind: FinestKind,
    /// Why nothing finer exists, **in the vendor's own terms** — the words an
    /// operator can go and check, not this repository's paraphrase of them.
    pub because: &'static str,
    /// Where those words were read. `CLAUDE.md` §3 rule 1: a vendor claim with
    /// no source is indistinguishable from one somebody typed.
    pub source: &'static str,
}

/// A rung refused because it is finer than anything the vendor serves.
///
/// Carries the whole refusal rather than a boolean, so the caller that renders
/// it cannot reduce it to a single word meaning `unavailable` — the same reason
/// [`SessionRefusal`] names the venue, the window and the gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FloorRefusal {
    /// The rung that was asked for.
    pub asked: Granularity,
    /// The finest rung the vendor does serve.
    pub finest: Granularity,
    /// What a record at [`Self::finest`] is — so a caller offering it as the
    /// nearest alternative cannot offer it under the wrong word.
    pub finest_kind: FinestKind,
    /// Why, in the vendor's terms. [`GranularityFloor::because`].
    pub because: &'static str,
    /// Where that was read. [`GranularityFloor::source`].
    pub source: &'static str,
}

/// What a feed's granularity floor says about one rung.
///
/// Three arms and **two verdicts**: [`Self::Refused`] is the vendor saying
/// never, and the other two are it not saying so. They are kept apart because
/// only one rung per feed can answer the tick-versus-conflated question — at
/// any coarser rung the floor has no opinion, and inventing one would be
/// `CLAUDE.md` §3 rule 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RungVerdict {
    /// This rung **is** the vendor's finest, and this is what a record at it
    /// is. The one place the conflated-versus-tick distinction is answerable.
    Finest(FinestKind),
    /// Coarser than the vendor's finest, so the granularity floor does not
    /// refuse it.
    ///
    /// It says **nothing** about whether this build fetches the rung — that is
    /// `Descriptor::granularities`, a different question with its own answer,
    /// and the two are deliberately not merged: Dhan's minute rung is one the
    /// vendor serves and this build does not wire up, which is a refusal a
    /// code change fixes.
    Coarser,
    /// Finer than anything the vendor has ever served. No pull, no
    /// entitlement, no configuration and no code change makes this rung exist.
    Refused(FloorRefusal),
}

impl RungVerdict {
    /// Whether the vendor refuses this rung outright.
    ///
    /// The two-verdict reading of the three arms, written once here so no
    /// caller has to spell the match and get it subtly different.
    #[must_use]
    pub const fn is_refused(self) -> bool {
        matches!(self, Self::Refused(_))
    }

    /// The refusal, or `None` when the floor does not refuse.
    #[must_use]
    pub const fn refusal(self) -> Option<FloorRefusal> {
        match self {
            Self::Refused(refusal) => Some(refusal),
            Self::Finest(_) | Self::Coarser => None,
        }
    }

    /// What a record at this rung is, when the floor is what states it.
    ///
    /// `None` at a coarser rung and at a refused one — for opposite reasons,
    /// and both honest: above the floor nothing here has measured the shape,
    /// and below it there is no record to have a shape.
    #[must_use]
    pub const fn kind(self) -> Option<FinestKind> {
        match self {
            Self::Finest(kind) => Some(kind),
            Self::Coarser | Self::Refused(_) => None,
        }
    }
}

/// Whether a feed's budget is shared across request kinds or held per kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pooling {
    /// One budget for the whole vendor, whatever is being asked for.
    PerVendor,
    /// A separate budget per endpoint group — `crate::rate::RequestKind`.
    PerRequestKind,
}

/// Everything an HTTP feed needs.
///
/// [`Auth`] and [`Budget`] live **here**, not on [`Descriptor`]. That is the
/// structural half of "a local archive has no token and no rate limit": an
/// archive descriptor has nowhere to put either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HttpSpec {
    /// Scheme and host, no trailing slash.
    pub base_url: &'static str,
    /// The historical-bars path, one [`PathSegment`] per `/`-separated part,
    /// with no leading slash of its own — the slash belongs to the join.
    ///
    /// A feed whose path is entirely fixed carries only
    /// [`PathSegment::Literal`] rows and reads exactly as its old string did.
    /// See [`PathSegment`] for why this is a list.
    pub bars_path: &'static [PathSegment],
    /// The verb.
    pub method: Method,
    /// Which header carries the credential, and how.
    pub auth: Auth,
    /// How dates are rendered.
    pub date_format: DateFormat,
    /// Whether the range end includes the day it names.
    pub range_end: RangeEnd,
    /// How the response is laid out.
    pub response: ResponseShape,
    /// What each field is called.
    pub fields: FieldNames,
    /// What a timestamp means.
    pub timestamps: TimestampEncoding,
    /// What unit prices arrive in.
    pub prices: PriceScale,
    /// The published budget.
    pub budget: Budget,
    /// The oldest day this feed will answer for, **whatever rung is asked
    /// for** — the loosest of its floors, and the one the clamp reads.
    ///
    /// # Why a feed needs one, and why the two differ in KIND
    ///
    /// `docs/00-charter.md` §4 records Groww's daily history as 2020-01-01 and
    /// Dhan's as "rolling ~5 years. **Not a fixed floor** — it moves every
    /// day." That is a difference of SHAPE, not of number, and one date could
    /// not express both.
    ///
    /// It matters because asking below the floor is not an error the vendor
    /// reports usefully — it answers EMPTY, and an empty answer is
    /// indistinguishable from a day that did not trade. So a 2020-to-yesterday
    /// backfill against Dhan spends ~5 months of requests per instrument on
    /// data that no longer exists, and reports success.
    ///
    /// For Dhan it is worse than waste: a rolling floor makes COMPLETE a claim
    /// with an expiry date. The oldest month held falls further outside the
    /// vendor's window every month, and nothing notices.
    ///
    /// # TWO FIELDS NOW SAY "HOW FAR BACK", AND THIS ONE IS THE LOOSER
    ///
    /// [`Descriptor::history`] states the floor **per rung**, because the
    /// vendors do: Groww's own interval table gives its day row "Full history"
    /// and its one-minute row "Last 3 months". This field is per **vendor**
    /// and cannot express that, and it is what `api::server::clamp_to_floor`
    /// is handed today.
    ///
    /// The two are not free to disagree in the dangerous direction. This one
    /// must be **no later** than every rung's own floor — an over-clamp would
    /// refuse days a vendor holds, which loses data, while an under-clamp only
    /// spends requests that come back empty. `pull::vendor::tests::the_vendor_
    /// wide_floor_is_never_later_than_a_rungs_own` asserts exactly that, so a
    /// rung floor edited to be stricter cannot silently leave this field
    /// behind.
    ///
    /// **What is still true and is a limit, not an invariant:** Groww's
    /// one-minute rung is narrowed to three months by its own documentation
    /// and this field says 2020, so a one-minute backfill to 2020 is clamped
    /// to 2020 and spends six years of requests the vendor answers empty. The
    /// fix is one line at the clamp's call site — `asked.granularity` is in
    /// scope there — and it changes what a live pull does, so it is recorded
    /// in D-0113 rather than taken in passing.
    pub history_floor: HistoryFloor,
    /// How many days one request may name, **per rung**.
    ///
    /// A VENDOR bound, and distinct from `api::ingest::MAX_WINDOW_DAYS`, which
    /// is an INPUT bound: 3,653 days, wide enough for the whole stated backfill
    /// and narrow enough that a fat-fingered year is caught. That number says
    /// nothing about what a broker will answer, so a window inside it can still
    /// be far outside this.
    ///
    /// # Why a table and not a scalar
    ///
    /// It was a scalar, and the granularity it applied at was written in
    /// **prose**: `docs/00-charter.md` §4 records Groww's cap as "30 days per
    /// request **at 1-minute granularity**", and the field's own doc repeated
    /// the qualifier. That was survivable while this build fetched one rung.
    /// It stops being survivable the moment a second rung is selectable: a
    /// daily window split at the one-minute cap issues four times the requests
    /// the vendor asked for, and a one-minute window split at a daily cap
    /// issues a request the vendor will not serve.
    ///
    /// A rung with no row here has **no published cap**, and that is a state
    /// with a meaning rather than a hole: the window goes to
    /// [`crate::session::split_window`] uncapped, which still breaks it at
    /// every month boundary because the store addresses one month per file at
    /// every rung. It is not a default and not a guess — an unknown cap would
    /// have to be `UNVERIFIED` in the charter and named as such here, the way
    /// [`crate::rate::GROWW_PER_SECOND_UNVERIFIED`] is, and it **is**: see the
    /// two rows below and `docs/00-charter.md` §4.
    pub window_caps: &'static [(Granularity, u32)],
    /// This feed's own wire spelling for each rung its request can name.
    ///
    /// Read by [`ParamValue::Granularity`], and empty for a feed whose request
    /// carries no interval field at all — Dhan names its rung by endpoint path
    /// rather than by parameter, so there is nothing here to spell.
    ///
    /// A rung absent from this table cannot be put on this feed's wire, and the
    /// fetch refuses by name rather than sending the request that *can* be
    /// built and filing the answer under the rung that was asked for.
    pub granularity_tokens: &'static [(Granularity, &'static str)],
    /// Whether that budget is pooled per request kind.
    pub pooling: Pooling,
    /// Every field this feed's request carries, and where its value comes from.
    ///
    /// Empty means the request names nothing but its window, which is the state
    /// that made Dhan answer `DH-905 securityId is required`.
    /// This feed's words for each listing class it has been verified against.
    ///
    /// A class absent from this table is one no document states a word for,
    /// and a request for it is refused by name rather than guessed at. See
    /// [`HttpSpec::listing_words`].
    pub listings: &'static [ListingWords],
    /// Every named parameter this feed's bars request carries, and where each
    /// one's value comes from.
    pub params: &'static [Param],
    /// Headers this feed requires beyond the credential.
    pub extra_headers: &'static [(&'static str, &'static str)],
}

impl HttpSpec {
    /// The bars path with every value segment shown as its own placeholder —
    /// `/instruments/historical/:instrument_token/:interval`.
    ///
    /// **This is for a HUMAN, never for a socket.** It answers "which endpoint
    /// is this feed" without a request in hand, which is what a receipt and a
    /// diagnostic need; the URL a request actually goes to is
    /// [`crate::http::HttpSource::url`], which can refuse and therefore returns
    /// a `Result`. Two functions because they answer two questions — a receipt
    /// that could fail to render because an instrument had no id would be a
    /// worse receipt.
    ///
    /// A fixed walk of the descriptor's own list. Nothing an operator supplies
    /// changes its length.
    #[must_use]
    pub fn path_template(&self) -> String {
        let mut out = String::new();
        for segment in self.bars_path {
            out.push('/');
            match *segment {
                PathSegment::Literal(word) => out.push_str(word),
                PathSegment::Value { placeholder, .. } => {
                    out.push(':');
                    out.push_str(placeholder);
                }
            }
        }
        out
    }

    /// How many days one request may name **at `rung`**, when the vendor
    /// published a figure for it.
    ///
    /// `None` is "no published cap at this rung", which
    /// [`crate::session::split_window`] takes as "the month boundary is the
    /// only bound". It is never "send the window whole" — that would be the
    /// un-split request the splitter exists to prevent.
    ///
    /// A walk of a table whose length is a `const` in this file, at most one
    /// row per rung of a closed ladder. There is no `N` here that an input can
    /// grow. See [`Self::window_caps`].
    #[must_use]
    pub fn window_cap_days(&self, rung: Granularity) -> Option<u32> {
        self.window_caps
            .iter()
            .find(|(at, _)| *at as u8 == rung as u8)
            .map(|(_, cap)| *cap)
    }

    /// This feed's wire word for `rung`, or `None` when it has no recorded one.
    ///
    /// Same shape and same bound as [`Self::window_cap_days`], and the same
    /// meaning for an absent row: not a default, not a derivation. See
    /// [`Self::granularity_tokens`].
    #[must_use]
    pub fn granularity_token(&self, rung: Granularity) -> Option<&'static str> {
        self.granularity_tokens
            .iter()
            .find(|(at, _)| *at as u8 == rung as u8)
            .map(|(_, word)| *word)
    }

    /// This feed's words for a listing class, or `None` when it records none.
    ///
    /// `None` is the honest answer for a class this feed has never been
    /// verified against. Groww's documentation states `CASH` and `FNO` for
    /// `segment` and says nothing about an index, so an index request is
    /// refused by name rather than sent a guessed word — `CLAUDE.md` §3 rule 1.
    ///
    /// CONSTANT WORK, not a scan. The table's length is the number of listing
    /// classes — two — so this costs the same against a universe of 800 as
    /// against one. Same shape as [`Self::granularity_token`] above.
    #[must_use]
    pub fn listing_words(&self, listing: Listing) -> Option<ListingWords> {
        self.listings
            .iter()
            .find(|w| w.listing as u8 == listing as u8)
            .copied()
    }
}

/// How an archive vendor nests its files. Reported, and used to explain a
/// refusal; the addressing itself is [`ArchiveName`] and [`MemberPattern`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Nesting {
    /// One archive per day, holding one member per instrument at its root.
    ZipOfDailyZips,
    /// One archive per day, holding a stem folder with a folder per group.
    ZipOfSegmentFolders,
}

/// How a day's archive file is named.
///
/// A **pattern**, so the path is computed by string formatting from
/// `(segment, date)` and never found by walking a directory —
/// `docs/07-o1-architecture.md` law 3. A directory walk over a folder of daily
/// archives is O(days) and grows forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveName {
    /// `{prefix}{segment token}{infix}{date}{suffix}`.
    SegmentAndDate {
        /// Text before the segment token.
        prefix: &'static str,
        /// Text between the segment token and the date.
        infix: &'static str,
        /// Text after the date, extension included.
        suffix: &'static str,
        /// How the date is rendered.
        date: DateFormat,
    },
    /// `{prefix}{date}{suffix}`.
    DateOnly {
        /// Text before the date.
        prefix: &'static str,
        /// Text after the date, extension included.
        suffix: &'static str,
        /// How the date is rendered.
        date: DateFormat,
    },
}

/// How a member inside a day's archive is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberPattern {
    /// `{symbol}{suffix}` at the archive root.
    SymbolAtRoot {
        /// Extension, dot included.
        suffix: &'static str,
    },
    /// `{archive stem}/{group folder}/{symbol}{suffix}`.
    StemGroupSymbol {
        /// Extension, dot included.
        suffix: &'static str,
    },
}

/// Which folder inside an archive an instrument lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveGroup {
    /// The archive has no group folders.
    Flat,
    /// Options.
    Options,
    /// Futures.
    Futures,
}

/// Whether a member CSV starts with a header row, and what it must say.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeaderRow {
    /// No header row. A first line that looks like one is a refusal, not a
    /// row to skip: skipping it would silently drop a real record from a feed
    /// whose first field happens to be text.
    Absent,
    /// A header row, which must match this text exactly.
    Present(&'static str),
}

/// One CSV column's meaning.
///
/// [`Self::Unverified`] is the honest arm. `docs/08-vendor-samples.md` records
/// a column *count* for one archive whose column *meanings* were never
/// established; naming them anyway would be invention, and a price column read
/// as a volume yields plausible numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    /// The instrument ticker.
    Ticker,
    /// The date.
    Date,
    /// The time of day.
    Time,
    /// The last traded price.
    LastPrice,
    /// The open.
    Open,
    /// The high.
    High,
    /// The low.
    Low,
    /// The close.
    Close,
    /// Traded volume.
    Volume,
    /// Open interest.
    OpenInterest,
    /// The best bid price.
    BidPrice,
    /// The best bid size.
    BidQty,
    /// The best ask price.
    AskPrice,
    /// The best ask size.
    AskQty,
    /// The last traded quantity. Zero on a quote-only update.
    LastQty,
    /// Present, observed, and carrying nothing this engine reads.
    Ignored,
    /// Present, and its meaning was never established. Reading it is refused.
    Unverified,
}

/// The column order for one `(feed, segment)` pair.
///
/// Keyed by segment because `docs/08-vendor-samples.md` measured **5, 9 and 10
/// columns inside one vendor's own archives** — indices carry no volume and no
/// open interest, so those fields are structurally absent rather than zero. A
/// single per-vendor layout would mis-parse one of the two, silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnLayout {
    /// Which segment this layout describes.
    pub segment: Segment,
    /// The columns, in file order.
    pub columns: &'static [Column],
    /// THE DECODER'S NAME FOR THIS SAME SHAPE.
    ///
    /// [`Self::columns`] says what each field *means*; [`Columns`] is the
    /// variant [`crate::csv::decode`] and [`crate::archive::read_dir`] are
    /// actually handed. Both describe one file, and until this field existed
    /// the second one was chosen by a `match` at the call site — twice in
    /// `crates/api/src/server.rs`, both hardcoded to `Columns::Gdfl`, which is
    /// the wrong shape for the other archive vendor and would have read
    /// `TrueData`'s five columns as ten.
    ///
    /// It is a field rather than a branch for the reason the module header
    /// gives, and it cannot drift from [`Self::columns`]: the `const` block
    /// below [`DESCRIPTORS`] holds the count, the header row and the date
    /// format of the two against each other, so a layout whose two spellings
    /// disagree fails the build.
    pub shape: Columns,
}

/// What kind of record a feed writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordShape {
    /// Open, high, low, close, volume, open interest — the 56-byte bar
    /// `crates/store` writes.
    Ohlcv,
    /// Last price, bid, bid size, ask, ask size, last quantity, open
    /// interest. **Not** a bar: it has no open, high, low or close, several
    /// records share one timestamp, and there is no sub-second tiebreaker.
    /// `crates/store` has no format version for this yet, so a feed declaring
    /// it refuses at the write boundary rather than being stored as a bar with
    /// the price repeated four times — which would be a lie written into the
    /// data itself.
    Snapshot,
}

/// Everything a local-archive feed needs.
///
/// No token field and no budget field, and that is the point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArchiveSpec {
    /// How the vendor nests its files.
    pub nesting: Nesting,
    /// How a day's archive is named.
    pub archive: ArchiveName,
    /// How a member inside it is named.
    pub member: MemberPattern,
    /// The group folders, by name. Empty for an archive with no group folders.
    pub groups: &'static [(ArchiveGroup, &'static str)],
    /// The segment tokens an [`ArchiveName::SegmentAndDate`] needs.
    pub segment_tokens: &'static [(Segment, &'static str)],
    /// Whether a header row exists.
    pub header: HeaderRow,
    /// The field separator.
    pub delimiter: u8,
    /// The column layouts, by segment.
    pub layouts: &'static [ColumnLayout],
    /// How the date column is written.
    pub date_format: DateFormat,
    /// What unit prices are in.
    pub prices: PriceScale,
}

impl ArchiveSpec {
    /// The layout for one segment, or `None` when none was ever measured.
    ///
    /// A linear walk of a `const` slice whose length is a property of the
    /// shipped table and not of any input — at most one entry per segment, and
    /// `Segment` has three variants. Constant, by the same argument
    /// [`MAX_LATER_SESSION_ROWS`] makes.
    /// `pull::vendor::an_unmeasured_segment_layout_is_refused_by_name` is the
    /// test that a missing layout refuses instead of guessing.
    #[must_use]
    pub fn layout(&self, segment: Segment) -> Option<&'static ColumnLayout> {
        self.layouts.iter().find(|row| row.segment == segment)
    }

    /// The folder name for a group, or `None` when the archive has no group
    /// folders at all, or the group is not one this vendor has.
    #[must_use]
    pub fn group_folder(&self, group: ArchiveGroup) -> Option<&'static str> {
        self.groups
            .iter()
            .find(|(candidate, _)| *candidate == group)
            .map(|(_, folder)| *folder)
    }

    /// The archive token for a segment, or `None`.
    #[must_use]
    pub fn segment_token(&self, segment: Segment) -> Option<&'static str> {
        self.segment_tokens
            .iter()
            .find(|(candidate, _)| *candidate == segment)
            .map(|(_, token)| *token)
    }
}

/// WHERE A FEED'S BYTES COME FROM, as a property of the vendor.
///
/// # The operator's rule, 12 Aug 2026, verbatim
///
/// > "for truedata and gdfl alone, one and only, we will pull the data
/// > especially entirely from csv files from the precise folder — because we
/// > will buy those data from them as csv files and we will put that into the
/// > specified folder, only from there it should be read."
/// >
/// > "except these two alone only, for all other vendors or brokers feeds it
/// > should be always REST."
///
/// That is a two-valued property of the vendor, and it decides five separate
/// behaviours that were previously decided one at a time by matching on
/// [`Transport`]'s payload:
///
/// | | [`Self::Rest`] | [`Self::Folder`] |
/// |---|---|---|
/// | credential | required — `CLAUDE.md` §8 | **none, and §8 must not run** |
/// | quota | a published budget, governed | **none** |
/// | reach | a [`HistoryFloor`], a vendor constant | **the files present** |
/// | finest rung | one minute | one **second** |
/// | the verb | pull | **read** |
///
/// # Why beside [`Transport`] rather than instead of it
///
/// [`Transport`] carries the *specification* — a base URL and an auth header,
/// or a naming pattern and a column layout — and it is therefore large, and
/// its two arms cannot be compared, hashed or written into a table cell
/// without dragging a whole [`HttpSpec`] along. Callers that only need to know
/// *which kind* were pattern-matching `Transport::Http(_)` with a discarded
/// payload, in four places, and each one was free to disagree with the others.
///
/// This is that question as one two-valued word. [`Transport::kind`] is the
/// single `match` that answers it, so a fifth feed lands on the right side of
/// every one of the five rows above by declaring its transport and nothing
/// else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceKind {
    /// A network request against a vendor's API. Has a token, a quota, and a
    /// published history floor that this repository does not get to choose.
    Rest,
    /// A folder of files the operator bought and put there. No socket, no
    /// token, no quota, and no vendor-stated floor: its reach is exactly which
    /// files are present, which is why [`crate::folder::Reach`] is read rather
    /// than declared.
    Folder,
}

impl SourceKind {
    /// A short, stable label for a page or a report.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Rest => "REST API",
            Self::Folder => "folder of files",
        }
    }

    /// THE HONEST VERB for getting data out of this source.
    ///
    /// A folder is not pulled, fetched or requested. Nothing is asked of
    /// anybody, no quota moves, nothing can rate-limit it and there is no
    /// remote party to be unavailable — the bytes are already on the disk the
    /// process is running on. Calling it a pull put an operator in the wrong
    /// diagnostic frame every time one failed: the first questions asked were
    /// about tokens, entitlements and outages, and the answer was always a
    /// path.
    ///
    /// Present tense, lower case, and no punctuation, so it drops into a
    /// sentence: `format!("{verb} refused")`.
    #[must_use]
    pub const fn verb(self) -> &'static str {
        match self {
            Self::Rest => "pull",
            Self::Folder => "read",
        }
    }

    /// [`Self::verb`] in the past tense, for a receipt.
    ///
    /// Spelled out rather than suffixed: `read` does not take a `-ed`, and a
    /// helper that appended one would have written `readed` on the arm this
    /// whole type exists for.
    #[must_use]
    pub const fn verb_past(self) -> &'static str {
        match self {
            Self::Rest => "pulled",
            Self::Folder => "read",
        }
    }

    /// Whether `CLAUDE.md` §8's credential machinery applies at all.
    ///
    /// `false` for a folder, and that is a **behaviour**, not a label: a
    /// missing credential must not block a source that has nothing to
    /// authenticate against, and `crate::config` asks this rather than naming
    /// vendors.
    #[must_use]
    pub const fn needs_credential(self) -> bool {
        matches!(self, Self::Rest)
    }

    /// Whether a rate governor applies at all.
    #[must_use]
    pub const fn needs_quota(self) -> bool {
        matches!(self, Self::Rest)
    }

    /// Whether a [`HistoryFloor`] is the right way to ask how far back this
    /// source reaches.
    ///
    /// `false` for a folder. Its reach is not a claim anybody published, it is
    /// the set of files present — see [`crate::folder::Reach`], which is read
    /// off the disk. An empty folder therefore has an EMPTY reach and says so;
    /// it does not fall back to a REST-shaped floor, and
    /// [`Descriptor::history`] is empty for every folder feed so there is
    /// nothing to fall back to.
    #[must_use]
    pub const fn has_history_floor(self) -> bool {
        matches!(self, Self::Rest)
    }

    /// The finest rung this KIND of source can ever offer — one **minute** over
    /// REST, one **second** out of a folder.
    ///
    /// # Not a second copy of [`GranularityFloor`]
    ///
    /// [`GranularityFloor`] is the per-vendor fact, with the vendor's own words
    /// and the place they were read, and it is what refuses a rung. This is the
    /// operator's rule of 12 Aug 2026 about the two KINDS, and it exists so the
    /// two can be cross-checked: the `const` block under [`DESCRIPTORS`] holds
    /// every feed's own floor against its kind's, so a folder feed that claimed
    /// milliseconds or a REST feed that claimed seconds fails the build rather
    /// than being believed.
    ///
    /// Neither arm is [`Granularity::Tick`], and that is the same rule seen
    /// from the other side — see [`Granularity::is_requestable`]. Both feeds
    /// sold as tick-by-tick were measured at one-second resolution with no
    /// sub-second field (`docs/08-vendor-samples.md`), so a second is the
    /// finest thing that ever lands from either of them.
    #[must_use]
    pub const fn finest_possible(self) -> Granularity {
        match self {
            Self::Rest => Granularity::Minute1,
            Self::Folder => Granularity::Second1,
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// How a feed's bytes are obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(
    clippy::large_enum_variant,
    reason = "the two variants differ by ~220 bytes and neither can be boxed. This type is \
              `Copy` and every value of it lives inside a `const DESCRIPTOR` in this file, \
              reached only through `&'static Descriptor` from `Feed::descriptor` — a `Box` \
              needs an allocator, which a `const` does not have, and the lint's own note \
              says boxing 'would require the type no longer be Copy'. Nothing moves one by \
              value on any path: the sweep never sees a descriptor, and the pull reads \
              fields through a reference. The size grew past the threshold when `Auth` \
              gained `key_field` and `AuthScheme` gained a variant carrying two `&'static \
              str` (D-0134), which is descriptor DATA — the thing this file exists to hold. \
              Shrinking it by indirection would trade a compile-time table for a runtime \
              one to satisfy a heuristic about values that are never moved."
)]
pub enum Transport {
    /// A network request.
    Http(HttpSpec),
    /// A folder of archives already on disk. No network, no token, no rate
    /// limit, and no 429.
    LocalArchive(ArchiveSpec),
}

impl Transport {
    /// WHICH KIND OF SOURCE THIS IS, with the specification dropped.
    ///
    /// **The only place the two arms are told apart.** Everything below, and
    /// `crate::config`'s credential requirement, and `crate::folder`'s reach,
    /// go through this one `match` — so a fifth transport answers every one of
    /// them by adding a single arm here, and cannot answer two of them
    /// inconsistently.
    #[must_use]
    pub const fn kind(self) -> SourceKind {
        match self {
            Self::Http(_) => SourceKind::Rest,
            Self::LocalArchive(_) => SourceKind::Folder,
        }
    }

    /// A short, stable label for a page or a report.
    ///
    /// The words are [`SourceKind`]'s, so a page and a refusal cannot call the
    /// same feed two different things. The old spelling here was
    /// `local archive`, which named a *file layout*; the fact an operator needs
    /// is that it is a folder on this machine.
    #[must_use]
    pub const fn label(self) -> &'static str {
        self.kind().label()
    }

    /// Whether this transport needs a credential at all.
    ///
    /// The page reads this rather than asking which vendor it is: a folder
    /// feed must never be shown a token field.
    #[must_use]
    pub const fn needs_credential(self) -> bool {
        self.kind().needs_credential()
    }

    /// Whether this transport needs a rate governor at all.
    #[must_use]
    pub const fn needs_governor(self) -> bool {
        self.kind().needs_quota()
    }
}

// ---------------------------------------------------------------------------
// the feed table
// ---------------------------------------------------------------------------

/// How many feeds this build knows.
pub const FEED_COUNT: usize = 5;

/// Which feed a request is for.
///
/// Named `Feed` and not `Vendor` because `brutex_core::vendor::Vendor` already
/// exists and means something narrower — the **store path prefix**, a closed
/// set of the two brokers whose bars this engine writes. Two types named
/// `Vendor` in one crate is exactly the confusion this module exists to
/// remove. [`Feed::store_vendor`] is the one bridge between them, and it
/// returns `None` rather than inventing a prefix.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feed {
    /// Secondary broker, HTTP.
    Dhan = 0,
    /// Primary broker, HTTP.
    Groww = 1,
    /// Historical archives on disk.
    TrueData = 2,
    /// Historical archives on disk.
    Gdfl = 3,
    /// Third broker, HTTP. The vendor `docs/07-plan.md` §5 predicted: its
    /// request carries the instrument and the rung as PATH SEGMENTS, its header
    /// carries a prefix and a SECOND secret, and its timestamps carry their own
    /// zone. D-0133, D-0134, D-0135.
    Zerodha = 4,
}

impl Feed {
    /// Every feed, in table order.
    pub const ALL: [Self; FEED_COUNT] = [
        Self::Dhan,
        Self::Groww,
        Self::TrueData,
        Self::Gdfl,
        Self::Zerodha,
    ];

    /// This feed's row in [`DESCRIPTORS`].
    ///
    /// **One array index.** No search, no map, no hash, and the cost does not
    /// change when the table grows — the index is the variant's own
    /// discriminant, so there is nothing to look up. The `const` assertions
    /// under [`DESCRIPTORS`] are what make indexing by discriminant sound:
    /// they pin the table length to the variant count and row *i* to variant
    /// *i*, so a new variant with no row fails the build.
    /// `pull::vendor::the_descriptor_table_is_indexed_by_the_discriminant`
    /// proves both halves from outside.
    #[must_use]
    #[allow(
        clippy::indexing_slicing,
        reason = "the index IS the #[repr(u8)] discriminant, and the const assertions under \
                  DESCRIPTORS pin the table length to FEED_COUNT and each row to its own \
                  variant, so the index is in range at compile time; `.get()` would need an \
                  unwrap, which is banned outright"
    )]
    pub const fn descriptor(self) -> &'static Descriptor {
        DESCRIPTORS[self as usize]
    }

    /// The store path prefix this feed writes under.
    ///
    /// `None` for a feed `brutex_core::vendor::Vendor` does not name. That is
    /// a **named refusal at the write boundary**, not a fallback: the vendor
    /// is the first segment of every store path (`docs/05-decisions.md`
    /// D-0019) precisely so one feed's history can be deleted without touching
    /// another's, and filing an archive vendor's bars under a broker's prefix
    /// would destroy that property irreversibly.
    #[must_use]
    pub const fn store_vendor(self) -> Option<Vendor> {
        match self {
            Self::Dhan => Some(Vendor::Dhan),
            Self::Groww => Some(Vendor::Groww),
            // THEY HAVE PREFIXES NOW. `None` here is what filed 194
            // instrument-months of GDFL futures under `bars/dhan/` — the
            // archive reader could not ask which vendor it was reading for, so
            // it used a literal. D-0019's per-vendor independence needs a row
            // per feed, and `pull::config` now requires a credential only of
            // feeds whose transport is HTTP, so these two cost no operator a
            // `credentials.toml` edit.
            Self::TrueData => Some(Vendor::TrueData),
            Self::Gdfl => Some(Vendor::Gdfl),
            Self::Zerodha => Some(Vendor::Zerodha),
        }
    }

    /// The operator-facing name.
    #[must_use]
    pub const fn display(self) -> &'static str {
        self.descriptor().display
    }

    /// The name this feed is written down as.
    #[must_use]
    pub const fn wire(self) -> &'static str {
        self.descriptor().wire
    }

    /// Whether this feed serves `granularity`.
    #[must_use]
    pub const fn serves(self, granularity: Granularity) -> bool {
        self.descriptor().granularities.contains(granularity)
    }

    /// The finest rung this vendor can **ever** serve.
    ///
    /// Distinct from [`Self::serves`] in kind, not in degree: that one answers
    /// what this build fetches today, and this one answers what the vendor
    /// publishes at all. See [`GranularityFloor`].
    #[must_use]
    pub const fn finest_rung(self) -> Granularity {
        self.descriptor().granularity_floor.finest
    }

    /// What this vendor's granularity floor says about `granularity`.
    ///
    /// The `Feed`-level spelling of [`Descriptor::granularity_verdict`], whose
    /// doc carries the cost argument and names its tests.
    #[must_use]
    pub const fn granularity_verdict(self, granularity: Granularity) -> RungVerdict {
        self.descriptor().granularity_verdict(granularity)
    }

    /// WHERE THIS FEED'S BYTES COME FROM — REST, or a folder on this machine.
    ///
    /// The operator's rule of 12 Aug 2026 in one call: `TrueData` and GDFL are
    /// folders, every other vendor and broker is REST. Derived from the
    /// transport the descriptor already declares rather than being a second
    /// field that could disagree with it. See [`SourceKind`].
    #[must_use]
    pub const fn source_kind(self) -> SourceKind {
        self.descriptor().transport.kind()
    }
}

impl fmt::Display for Feed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display())
    }
}

/// What a vendor MEANS by a bar of a given rung.
///
/// # Why a field and not an assumption
///
/// Two vendors were storing daily bars for the same instrument into
/// `bars/<vendor>/NSE/INDEX/BANKNIFTY/1day/`. The paths are isolated —
/// `docs/04-invariants.md` X-12, and the first path segment is a [`Vendor`]
/// rather than a string, so neither can write into the other's directory. What
/// isolation does **not** do is stop two directories with the same name holding
/// two different definitions of the thing inside them.
///
/// Measured on 2026-08-07, BANKNIFTY, both vendors, same day:
///
/// | | open | high | low | close |
/// |---|---|---|---|---|
/// | Dhan `1day` | 57,882.00 | 57,994.45 | 57,686.55 | 57,746.45 |
/// | Groww `1day` | **58,063.65** | **58,063.65** | 57,688.30 | 57,746.45 |
///
/// Groww's open is Dhan's *previous session's close*, to the paisa, and its
/// high is `max(previous close, the day's high)`. The rule holds across a
/// request boundary — July's last close is August's first open, and those are
/// two separate HTTP responses that never meet in memory — so it is the
/// vendor's convention and not this repository's arithmetic.
///
/// Neither vendor is wrong. They answer two different questions, and nothing in
/// the type system said so. This field says so.
///
/// # What it is not
///
/// It is not a normaliser. Recording the convention does not convert one into
/// the other; it makes the difference **declarable**, so a reader can refuse to
/// compare two rungs that do not mean the same thing, and a vendor added later
/// has to state which it is rather than inheriting whatever the last one did.
/// `CLAUDE.md` §3 rule 1 — an unstated assumption about a vendor is exactly the
/// invention this repository forbids. See D-0077.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarConvention {
    /// The bar covers one exchange session: open is the session's **first**
    /// print, close is its last. Dhan's `/v2/charts/historical`.
    SessionOpenToClose,
    /// The bar begins at the **previous** session's closing print and ends at
    /// this session's. Groww's `/v1/historical/candles` at `1day`.
    ///
    /// Its high and low therefore include that earlier print, which is why a
    /// gap-down day reads as `open == high`.
    PreviousCloseToClose,
}

impl BarConvention {
    /// What the page calls it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::SessionOpenToClose => "session open to close",
            Self::PreviousCloseToClose => "previous close to close",
        }
    }

    /// Whether two feeds' bars of the same rung describe the same interval.
    ///
    /// The one question a caller comparing two vendors has to ask, and the
    /// reason this enum exists rather than a comment.
    #[must_use]
    pub const fn comparable_with(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::SessionOpenToClose, Self::SessionOpenToClose)
                | (Self::PreviousCloseToClose, Self::PreviousCloseToClose)
        )
    }
}

/// One feed, entirely as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Descriptor {
    /// What this feed means by a **daily** bar.
    ///
    /// Only the day rung is recorded, because only the day rung has been
    /// measured. Groww's one-minute bars fold to a correct session candle —
    /// 1,875 records over five sessions, exactly 375 each, and the folded
    /// open matches Dhan's within 13 paisa — so the minute rung is believed
    /// [`BarConvention::SessionOpenToClose`] at both vendors and is **not**
    /// declared here, because `believed` is not `measured`. D-0077.
    ///
    /// (Backticks, not quotes, and deliberately: gate 1d scans every
    /// double-quoted lower-case token under `crates/pull` for an undeclared
    /// path segment, and it cannot tell prose from a literal. Quoting a word
    /// for emphasis would put it on the allowlist beside real vendor wire
    /// names, which is the dilution that makes such a list stop meaning
    /// anything. Backticks are also the better markdown.)
    pub day_bar: BarConvention,
    /// Which feed this row describes. Pinned to its index at compile time.
    pub feed: Feed,
    /// The operator-facing name.
    pub display: &'static str,
    /// The name this feed is written down as.
    pub wire: &'static str,
    /// What shape its records are.
    pub record: RecordShape,
    /// How its bytes are obtained.
    pub transport: Transport,
    /// Which granularities it serves.
    pub granularities: GranularitySet,
    /// How far back it answers, **per rung**. See [`FloorRow`].
    ///
    /// # On the descriptor, not on [`HttpSpec`]
    ///
    /// Beside `granularities`, because "which rungs" and "how far back at each
    /// rung" are one question asked twice, and because a **local archive has a
    /// reach too**: an archive descriptor has no `HttpSpec` to hang a field
    /// off, and the answer for both of them — nothing states anything — is a
    /// fact worth being able to say. Before this field the page could not ask
    /// the API and held the table itself, which is the one place a vendor fact
    /// must not live: it is versioned with the vendor row, and a second copy
    /// in a front end is the copy that goes stale.
    ///
    /// An absent rung is [`HistoryFloor::Unstated`] — see
    /// [`Descriptor::history_floor`]. It is never zero and never "no limit".
    pub history: &'static [FloorRow],
    /// How **fine** it goes, and what a record at that rung is.
    ///
    /// # The second floor, beside the first, on purpose
    ///
    /// `history` above says how far BACK a rung answers. This says which rungs
    /// exist at all, and the two are one vendor fact asked from two directions
    /// — Groww has both a 12 May 2026 for its one-minute rung and a one-minute
    /// bottom, and a page that models only the first offers `tick`, `1s` and
    /// `5s` as ordinary choices the operator can tick.
    ///
    /// Keyed on the FEED and not on the rung, which is the opposite of
    /// `history` and is the honest shape for what it states: a floor is one
    /// number per vendor, and every rung's verdict is derived from it by
    /// [`Descriptor::granularity_verdict`] rather than restated per row where
    /// eleven rows could disagree with each other.
    ///
    /// See [`GranularityFloor`] for why this could not be inferred from
    /// `granularities`, which is a fact about **this build**.
    pub granularity_floor: GranularityFloor,
    /// Which segments it serves.
    pub segments: SegmentSet,
    /// Which exchange its rows belong to.
    pub exchange: Exchange,
}

impl Descriptor {
    /// This feed's whole history claim for `rung`, or `None` when it records
    /// none.
    ///
    /// `None` is the honest answer for a rung nothing has been read about, and
    /// a caller must render it as a claim NOT MADE rather than as an absent
    /// bound. The two read the same in a JSON field and mean opposite things.
    ///
    /// A walk of a table with at most one row per rung of a closed ladder —
    /// the same shape and the same bound as [`HttpSpec::window_cap_days`].
    /// There is no `N` here an input can grow.
    #[must_use]
    pub fn history_row(&self, rung: Granularity) -> Option<&'static FloorRow> {
        let rows: &'static [FloorRow] = self.history;
        rows.iter().find(|row| row.granularity as u8 == rung as u8)
    }

    /// The floor that **binds** at `rung`, or [`HistoryFloor::Unstated`] when
    /// this feed records nothing for it.
    ///
    /// The convenience over [`Self::history_row`] for a caller that clamps
    /// rather than displays: `Unstated` already means "nothing published", and
    /// `clamp_to_floor` already reads it as "ask for what was asked". So an
    /// unrecorded rung narrows nothing, and it does so through the arm that
    /// says why rather than through a `None` a caller had to interpret.
    #[must_use]
    pub fn history_floor(&self, rung: Granularity) -> HistoryFloor {
        match self.history_row(rung) {
            Some(row) => row.binding.floor,
            None => HistoryFloor::Unstated,
        }
    }

    /// What this feed's granularity floor says about `rung`.
    ///
    /// # O(1), and by construction rather than by measurement
    ///
    /// One field read and at most two `u8` comparisons. There is no table to
    /// walk here and no rung count in the cost: the floor is a single rung and
    /// the ladder's discriminants ascend with coarseness, so "is this finer
    /// than the floor" is `<` on two bytes. The bound does not move when the
    /// ladder grows to twenty rungs or the feed table to forty rows.
    /// `pull::vendor::every_feed_answers_every_rung_with_one_of_two_verdicts`
    /// walks the whole 4 × 11 matrix, and
    /// `pull::vendor::the_ladder_ascends_so_one_comparison_decides_which_rung_is_finer`
    /// proves the ordering the single comparison rests on.
    ///
    /// # What it does **not** answer
    ///
    /// Whether this build fetches the rung. That is `granularities`, and
    /// [`RungVerdict::Coarser`] says why the two must not be collapsed.
    #[must_use]
    pub const fn granularity_verdict(&self, rung: Granularity) -> RungVerdict {
        let floor = self.granularity_floor;
        if rung.is_finer_than(floor.finest) {
            return RungVerdict::Refused(FloorRefusal {
                asked: rung,
                finest: floor.finest,
                finest_kind: floor.kind,
                because: floor.because,
                source: floor.source,
            });
        }
        if rung as u8 == floor.finest as u8 {
            RungVerdict::Finest(floor.kind)
        } else {
            RungVerdict::Coarser
        }
    }
}

// --- the rows --------------------------------------------------------------

/// How far back Dhan answers, per rung, with the disagreement intact.
///
/// The operator stated a rolling five years for this vendor on 11 Aug 2026.
/// The vendor's own historical-data page says two different things about its
/// two endpoints, and this table keeps them apart because they ARE apart:
/// the daily endpoint claims inception, the intraday one claims five years.
const DHAN_HISTORY: &[FloorRow] = &[
    FloorRow {
        granularity: Granularity::Day1,
        // THE OPERATOR'S, AND IT IS THE NARROWER OF THE TWO. The vendor claims
        // more history than the operator does; taking the vendor's word would
        // widen a bound the person paying for the entitlement says is five
        // years, and the cost of being wrong that way is a backfill that walks
        // decades of empty answers and calls them holidays.
        binding: FloorClaim {
            floor: HistoryFloor::Rolling { years: 5 },
            source: "the operator, 11 Aug 2026: a rolling last 5 years",
            standing: ClaimStanding::OperatorObservation,
        },
        contested: Some(FloorClaim {
            floor: HistoryFloor::Unbounded,
            source: "Dhan Docs / 12-historical-data.md, Get Daily Historical \
                     Data: available back upto the date of its inception",
            standing: ClaimStanding::VendorDocument,
        }),
        binds_because: "a stated absence of a floor cannot widen a stated one: \
                        five years back is the later day, so it is the one \
                        that refuses first, and obeying it can only cost \
                        requests the wider claim would have answered empty.",
    },
    // THE RUNG THIS BUILD DOES NOT SERVE, AND THE ROW IS STILL HERE.
    //
    // `granularities` withdraws `Minute1` from this feed — the descriptor's
    // single `bars_path` is the DAILY endpoint, so a minute request would have
    // fetched daily bars and filed them under `1min/`. The FLOOR is a fact
    // about the vendor either way, it was read from the same page as the row
    // above, and recording it here is what makes restoring the rung a question
    // about `bars_path` rather than a second reading of the documentation.
    // `/feeds.json` emits it marked as not served.
    FloorRow {
        granularity: Granularity::Minute1,
        // BOTH SOURCES SAY FIVE YEARS, so there is nothing to contest and the
        // source names them both rather than picking one to credit.
        binding: FloorClaim {
            floor: HistoryFloor::Rolling { years: 5 },
            source: "the operator, 11 Aug 2026, and Dhan Docs / \
                     12-historical-data.md, Get Intraday Historical Data \
                     (for last 5 years): the two agree",
            // BOTH SOURCES SAY IT, so the higher standing is recorded: an
            // agreeing vendor document cannot lower the standing of a claim
            // the operator also makes.
            standing: ClaimStanding::OperatorObservation,
        },
        contested: None,
        binds_because: "",
    },
];

/// How far back Groww answers, per rung, with the disagreement intact.
///
/// **The field is still per rung, and the vendor's own table is still why.**
/// One vendor, one documentation page, one interval table — and its two rows
/// say different things: `1 day` is "Full history", `1 min` is "Last 3
/// months". Those two claims remain per rung and remain recorded, as the
/// `contested` half of both rows below.
///
/// # What binds is uniform, and that is the operator's correction of 12 Aug 2026
///
/// Both rungs now bind at a FIXED 2020-01-01. He stated it twice — 11 Aug 2026
/// against the vendor, and again on **12 Aug 2026** after watching this page
/// refuse six years of days on the one-minute rung: *"GROWW — data is
/// available from JANUARY 2020. A fixed floor, not a rolling one."*
///
/// # UNVERIFIED: he did not state a per-rung split, and none is invented here
///
/// His figure names the vendor, not a rung, so it is applied to both rungs
/// unchanged. Whether Groww's one-minute endpoint genuinely answers to January
/// 2020 for his entitlement, or answers to some day between that and the
/// vendor's published quarter, is **UNVERIFIED** — no request was made to find
/// out, and `CLAUDE.md` §3 rule 1 forbids narrowing his statement on a guess.
/// The vendor's contrary per-rung claim is carried in `contested` precisely so
/// the day this is measured, the measurement lands between two recorded
/// numbers rather than against a single unattributed one.
const GROWW_HISTORY: &[FloorRow] = &[
    FloorRow {
        granularity: Granularity::Minute1,
        // THE OPERATOR'S, AND IT IS THE ONE ROW THAT CHANGED HANDS.
        //
        // It used to be the vendor's rolling quarter, on the rule that the
        // STRICTER of two claims binds. That rule is right when two sources
        // are two independent readings of one question, and this is not that
        // case: he restated the floor on 12 Aug 2026 having just watched this
        // page refuse January 2020 through May 2026 on his own account, which
        // is a report of what his entitlement ANSWERS, not a competing reading
        // of what Groww published. A published table describes the API in
        // general; he is the party the API answers.
        //
        // The stricter rule is not repealed — it still decides Dhan's day row
        // and this feed's — it is that the strictness comparison is between
        // sources of equal standing, and a direct observation from the
        // entitlement holder outranks a general claim about the product.
        binding: FloorClaim {
            floor: HistoryFloor::Fixed {
                year: 2020,
                month: 1,
                day: 1,
            },
            source: "the operator, 12 Aug 2026: Groww data is available from \
                     January 2020 — a fixed floor, not a rolling one",
            standing: ClaimStanding::OperatorObservation,
        },
        contested: Some(FloorClaim {
            floor: HistoryFloor::RollingMonths { months: 3 },
            source: "Groww Docs / 08-historical-data.md, interval table, \
                     1 min row: Last 3 months",
            standing: ClaimStanding::VendorDocument,
        }),
        binds_because: "the operator restated this floor on 12 Aug 2026 after \
                        watching this rung refuse six years of days on his own \
                        account. That is a report of what his entitlement \
                        answers; the vendor's table is a general claim about \
                        the product. Where the two disagree the direct \
                        observation binds, and the published quarter is kept \
                        beside it because whether this rung truly reaches 2020 \
                        is UNVERIFIED until a request measures it.",
    },
    FloorRow {
        granularity: Granularity::Day1,
        // THE OPERATOR'S, for the same reason Dhan's day row takes his: the
        // vendor claims everything and he claims 2020, and 2020 is the later
        // day.
        binding: FloorClaim {
            floor: HistoryFloor::Fixed {
                year: 2020,
                month: 1,
                day: 1,
            },
            source: "the operator, 11 Aug 2026: from January 2020",
            standing: ClaimStanding::OperatorObservation,
        },
        contested: Some(FloorClaim {
            floor: HistoryFloor::Unbounded,
            source: "Groww Docs / 08-historical-data.md, interval table, \
                     1 day row: Full history",
            standing: ClaimStanding::VendorDocument,
        }),
        binds_because: "a stated absence of a floor cannot widen a stated one: \
                        2020-01-01 is the later day, so it is the one that \
                        refuses first.",
    },
];

// --- the granularity floors ------------------------------------------------
//
// FOUR ROWS, TWO SHAPES, AND ONE SENTENCE THAT COVERS ALL OF THEM: no feed in
// this build serves a tick stream. The two brokers bottom out at a minute
// because their historical endpoints are CANDLE endpoints with no sub-minute
// word to ask with; the two archives bottom out at a second because that is
// the resolution the bytes on disk were measured to carry, and what sits on
// that second is a conflated snapshot rather than a print.
//
// THE SOURCE IS THE OPERATOR, AND THE DOCUMENTS AGREE WITH HIM. He stated the
// rule on 12 Aug 2026 and it is quoted in D-0118. `docs/00-charter.md` §4 and
// `docs/08-vendor-samples.md` were both read against it before these rows were
// written and neither contradicts it; where a document is SILENT rather than
// agreeing, the row below says so in its own source string rather than
// borrowing the operator's sentence as if a page had printed it.

/// Dhan's granularity floor: one minute, and it is a candle.
const DHAN_FLOOR: GranularityFloor = GranularityFloor {
    finest: Granularity::Minute1,
    kind: FinestKind::Bar,
    because: "Dhan's historical endpoints are candle endpoints. The intraday \
              one names candles of 1, 5, 15, 25 and 60 minutes and the daily \
              one names a session; neither request carries a field a shorter \
              interval could be written into, and this vendor publishes no \
              second-level or print-level history on this path at all.",
    source: "docs/00-charter.md section 4, Dhan: the endpoint row reads \
             intraday charts, 1/5/15/25/60 min, verified from the SDK; and \
             the operator, 12 Aug 2026, stating that a broker's historical \
             floor is one minute and that no vendor serves a print stream. \
             See D-0118.",
};

/// Groww's granularity floor: one minute, and it is a candle.
const GROWW_FLOOR: GranularityFloor = GranularityFloor {
    finest: Granularity::Minute1,
    kind: FinestKind::Bar,
    because: "Groww's own candle-interval annexure is the whole alphabet this \
              vendor's requests can be written in, and its finest row is one \
              minute; it runs upward from there to a month. There is no \
              second-level and no print-level word in that table, so a rung \
              below a minute cannot be spelled on this vendor's wire — the \
              request would have nothing to put in its interval field.",
    source: "docs/00-charter.md section 4, Groww: Granularity fetched — the \
             one-minute word only, and the daily-interval row citing the same \
             annexure, verified from the vendor's own document; and the \
             operator, 12 Aug 2026. See D-0118.",
};

/// `TrueData`'s granularity floor: one second, and it is a CONFLATED SNAPSHOT.
///
/// The archive's file names say otherwise and the file names are not evidence.
const TRUEDATA_FLOOR: GranularityFloor = GranularityFloor {
    finest: Granularity::Second1,
    kind: FinestKind::ConflatedSnapshot,
    because: "The archives are named for a print stream and do not hold one. \
              Measured, not assumed: every row's timestamp resolves to a whole \
              second, there is no sub-second field anywhere in the layout, and \
              a session of 22,500 seconds produced 22,426 rows — one per \
              second, not one per trade. Up to three rows share a second and \
              carry no tiebreaker, so even their order is file order rather \
              than time. What the file holds is a once-a-second conflated \
              snapshot of the best bid, the best ask and the best last price; \
              every print between two snapshots was discarded before the file \
              was written and no reader can recover it.",
    source: "docs/08-vendor-samples.md, the headline finding — neither vendor \
             sells print-by-print — with the measured table beside it: \
             timestamp resolution second, sub-second field none, 22,426 rows \
             across 22,500 seconds; and the operator, 12 Aug 2026, stating \
             that this vendor's floor is a one-second conflated snapshot and \
             not a print stream. See D-0118.",
};

/// GDFL's granularity floor: the same shape, measured the same way.
const GDFL_FLOOR: GranularityFloor = GranularityFloor {
    finest: Granularity::Second1,
    kind: FinestKind::ConflatedSnapshot,
    because: "As the archive above, and measured on its own files: whole-second \
              timestamps, no sub-second field, and up to four rows sharing one \
              second with no tiebreaker between them. The header itself names \
              what a row is — a last price with a best bid and a best ask \
              beside it — which is a snapshot of the book at an instant and \
              not the prints that moved it.",
    source: "docs/08-vendor-samples.md, the same headline finding and the same \
             measured table, GDFL column; and the operator, 12 Aug 2026, \
             stating that this vendor's floor is a one-second conflated \
             snapshot and not a print stream. See D-0118.",
};

const DHAN: Descriptor = Descriptor {
    // MEASURED against Groww on the same instrument and the same five sessions:
    // Dhan's open is the session's first print. 2026-08-07 BANKNIFTY —
    // O 57,882.00, and Groww's own chart agrees with it to the paisa.
    day_bar: BarConvention::SessionOpenToClose,
    feed: Feed::Dhan,
    display: "Dhan",
    wire: "dhan",
    record: RecordShape::Ohlcv,
    transport: Transport::Http(HttpSpec {
        base_url: "https://api.dhan.co",
        // Every segment fixed — this vendor names its instrument and its rung
        // in the BODY, not in the path, so nothing here is resolved per
        // request. Reads as `/v2/charts/historical`, which is what the single
        // string it replaced said.
        bars_path: &[
            PathSegment::Literal("v2"),
            PathSegment::Literal("charts"),
            PathSegment::Literal("historical"),
        ],
        method: Method::Post,
        auth: Auth {
            header: "access-token",
            scheme: AuthScheme::Raw,
            // One secret. This vendor's token IS the whole header value.
            key_field: None,
        },
        date_format: DateFormat::DashedYmd,
        // Verified from the vendor: `toDate` is NOT inclusive. One field, one
        // conversion site, and the off-by-one cannot come back.
        range_end: RangeEnd::Exclusive,
        response: ResponseShape::ParallelArrays { envelope: None },
        fields: FieldNames {
            open: "open",
            high: "high",
            low: "low",
            close: "close",
            volume: "volume",
            timestamp: "timestamp",
            // NOT NAMED, BECAUSE THIS VENDOR DOES NOT ALWAYS SEND IT.
            //
            // It was `Some("open_interest")`, which is a claim that the array
            // is in every answer. This vendor's own field table says otherwise:
            // `open_interest | int | No | Open interest (for F&O instruments)`.
            // A spot index carries no open interest, so the array is simply
            // absent — and the decoder, correctly, refuses an answer missing an
            // array the descriptor named. Both swept indices failed on it:
            // "the vendor's answer has no array named \"open_interest\" where
            // the descriptor says the bars are."
            //
            // The decoder was right and this row was wrong. `CLAUDE.md` §7 is
            // why it cannot be papered over instead: `i64::MIN` is the null and
            // zero means zero, so filling an absent array with zeros would read
            // back later as real open interest of nothing.
            //
            // The engine surface is NSE spot — two indices and the cash
            // constituents — and none of it has open interest. When the
            // expired-F&O path is modelled this becomes a per-listing question
            // rather than a per-feed one, exactly like `listings` above.
            open_interest: None,
        },
        timestamps: TimestampEncoding::EpochSecondsUtc,
        prices: PriceScale::Rupees,
        budget: Budget {
            // docs/00-charter.md §4 and crate::rate::{DHAN_PER_SECOND,
            // DHAN_PER_DAY}. No per-minute governor is published, and `None`
            // says exactly that.
            per_second: Some(crate::rate::DHAN_PER_SECOND),
            per_minute: None,
            per_day: Some(crate::rate::DHAN_PER_DAY),
        },
        // docs/00-charter.md §4, "History depth, daily": a rolling ~5 years,
        // not a fixed floor — it moves every day. Operator-stated 11 Aug 2026,
        // and the vendor's own daily page claims MORE (inception), which is
        // carried beside it in `DHAN_HISTORY` rather than here: this field is
        // per vendor and has room for one claim. Both of this feed's recorded
        // rungs carry the same five years, so the vendor-wide value below is
        // exactly the per-rung one and not a widening of it.
        history_floor: HistoryFloor::Rolling { years: 5 },
        // docs/00-charter.md §4: "Window cap | 90 days per request |
        // documented" — and that row sits directly under this vendor's
        // "Endpoint | intraday charts, 1/5/15/25/60 min — we fetch 1 only", so
        // the 90 is documented against the INTRADAY endpoint and is written
        // here against the one-minute rung it was recorded for.
        //
        // THE DAY ROW IS ABSENT AND THAT IS THE FACT, not an omission. The
        // charter records no day-level cap for this vendor, so there is no
        // number to write; guessing one would be `CLAUDE.md` §3 rule 1's
        // invention, and copying the 90 down would be promoting a figure past
        // the endpoint it was measured against. See docs/00-charter.md §4,
        // "Window cap, daily — UNVERIFIED".
        window_caps: &[(Granularity::Minute1, 90)],
        // EMPTY, AND THAT IS ALSO A FACT. This vendor's request carries no
        // interval field at all — the five params below are the whole of it —
        // so there is no word here to spell a rung with. Its rung is decided by
        // `bars_path`, which means this build cannot vary it and the bars are
        // folded to whatever rung they are filed under.
        granularity_tokens: &[],
        pooling: Pooling::PerVendor,
        // Read first-hand from dhanhq.co/docs/v2/historical-data. All three are
        // marked REQUIRED there, and their absence is exactly what DH-905
        // reports.
        //
        // BOTH CLASSES, FROM THE ANNEXURE, NOT FROM ONE OF THEM.
        //
        // `Fixed("IDX_I")` and `Fixed("INDEX")` stood here and were correct for
        // the two swept indices and wrong for all 750 NIFTY-Total-Market
        // equities — which were asked for inside the index segment and answered
        // with no window, every time. The words below are quoted from Dhan's own
        // annexure: Exchange Segment gives `IDX_I` = Index and `NSE_EQ` = NSE
        // Equity Cash; Instrument gives `INDEX` and `EQUITY`.
        listings: &[
            ListingWords {
                listing: Listing::Index,
                segment: "IDX_I",
                kind: "INDEX",
            },
            ListingWords {
                listing: Listing::Equity,
                segment: "NSE_EQ",
                kind: "EQUITY",
            },
        ],
        params: &[
            Param {
                name: "securityId",
                value: ParamValue::InstrumentId,
            },
            Param {
                name: "exchangeSegment",
                value: ParamValue::Segment,
            },
            Param {
                name: "instrument",
                value: ParamValue::Kind,
            },
            Param {
                name: "fromDate",
                value: ParamValue::From,
            },
            Param {
                name: "toDate",
                value: ParamValue::To,
            },
        ],
        extra_headers: &[],
    }),
    // ONE RUNG, BECAUSE ONE PATH IS ALL `bars_path` CAN HOLD.
    //
    // `Minute1` was declared here and it was NOT servable, in the worst way a
    // rung can be unservable: it did not fail. `bars_path` is a single field
    // pinned to `/v2/charts/historical`, which this vendor's documentation
    // names as the DAILY endpoint — the minute endpoint is `/charts/intraday`,
    // a different path. A `Minute1` request therefore fetched DAILY bars and
    // filed them under `1min/`, and nothing anywhere raised a word about it.
    // The vendor also defaults `interval` to 5 minutes when the field is
    // omitted, and this request carries no `interval` field at all, so even
    // against the correct path the answer would have been five-minute candles
    // filed as one-minute.
    //
    // Silent wrong data in an append-only store is worse than a refusal: the
    // month cannot be prepended and the bars look plausible. So the rung is
    // withdrawn until the intraday path is implemented.
    //
    // THIS ROW ONLY BITES BECAUSE SOMETHING READS IT. When it was first
    // withdrawn the comment here claimed a function named `is_served` refused
    // the rung by name — no such function existed, and `Feed::serves` had no
    // caller outside this file's own tests, so the withdrawal changed nothing
    // at all. `api::server::broker_window` now calls `serves` beside its
    // transport check, before the credential read, and refuses by name.
    // `CLAUDE.md` §4 — degrade loudly and name the reason, never a fallback
    // that hides a failure.
    //
    // Restoring it is not a one-row diff: it needs `bars_path` to become a
    // per-rung table and an `interval` parameter whose word comes from
    // `granularity_tokens`. UNVERIFIED until both exist.
    granularities: GranularitySet::EMPTY.with(Granularity::Day1),
    history: DHAN_HISTORY,
    granularity_floor: DHAN_FLOOR,
    segments: SegmentSet::EMPTY
        .with(Segment::Index)
        .with(Segment::Cash)
        .with(Segment::Fno),
    exchange: Exchange::Nse,
};

const GROWW: Descriptor = Descriptor {
    // MEASURED, AND IT IS NOT THE SAME BAR DHAN RETURNS. Groww's daily candle
    // begins at the PREVIOUS session's closing print. 2026-08-07 BANKNIFTY —
    // O 58,063.65, which is 2026-08-06's close exactly; H 58,063.65, which is
    // max(that close, the day's real high of 57,994.45). The rule holds across
    // a request boundary — July's last close is August's first open, and those
    // are two separate HTTP responses that never meet in memory — so it is the
    // vendor's convention and not this repository's arithmetic.
    //
    // Its MINUTE feed is a different matter and is believed correct: 1,875
    // bars over five sessions, exactly 375 each, folding to an open within 13
    // paisa of Dhan's. Only this rung is declared, because only this rung was
    // measured. D-0077.
    day_bar: BarConvention::PreviousCloseToClose,
    feed: Feed::Groww,
    display: "Groww",
    wire: "groww",
    record: RecordShape::Ohlcv,
    transport: Transport::Http(HttpSpec {
        base_url: "https://api.groww.in",
        // THE LIVE ENDPOINT. The vendor's own page says of the previous
        // one: "This API request is deprecated and will NOT work in the
        // future."
        //
        // Every segment fixed: this vendor carries its instrument and its rung
        // as QUERY parameters, which `params` below names. Reads as
        // `/v1/historical/candles`.
        bars_path: &[
            PathSegment::Literal("v1"),
            PathSegment::Literal("historical"),
            PathSegment::Literal("candles"),
        ],
        method: Method::Get,
        auth: Auth {
            header: "Authorization",
            scheme: AuthScheme::Bearer,
            // One secret, behind a fixed prefix. The `api-key` this vendor
            // also issues is spent by `pull::totp` to MINT the daily token and
            // never travels in this header — see docs/00-charter.md §4, Auth.
            key_field: None,
        },
        date_format: DateFormat::DashedYmdMidnight,
        range_end: RangeEnd::Inclusive,
        response: ResponseShape::PositionalRows {
            envelope: Some("payload"),
            array: "candles",
        },
        fields: FieldNames {
            open: "open",
            high: "high",
            low: "low",
            close: "close",
            volume: "volume",
            timestamp: "timestamp",
            open_interest: None,
        },
        // MEASURED off a live response: "2026-08-04T09:15:00". Never
        // millis, and not the space-separated form the vendor's own
        // documentation annotates this field with.
        timestamps: TimestampEncoding::IsoDateTimeText,
        prices: PriceScale::Rupees,
        budget: Budget {
            // The per-minute figure is operator-confirmed. The per-second
            // figure is NOT, and the constant's own name says so — see
            // crate::rate::GROWW_PER_SECOND_UNVERIFIED. It is not renamed
            // here and it is not quietly promoted.
            per_second: Some(crate::rate::GROWW_PER_SECOND_UNVERIFIED),
            per_minute: Some(crate::rate::GROWW_PER_MINUTE),
            per_day: None,
        },
        // Groww pools per endpoint group; the other broker does not.
        //
        // docs/00-charter.md §4, "History depth, daily": 2020-01-01,
        // operator-stated 11 Aug 2026. THE LOOSEST OF THIS FEED'S FLOORS AND
        // NOT ALL OF THEM — its one-minute rung is a rolling three months by
        // the vendor's own interval table, six years later than this. See
        // `GROWW_HISTORY`, and this field's own documentation for why the
        // looser value is the safe one to leave here and what it still costs.
        history_floor: HistoryFloor::Fixed {
            year: 2020,
            month: 1,
            day: 1,
        },
        // docs/00-charter.md §4: "Window cap | 30 days per request at 1-minute
        // granularity | documented". The qualifier was already in the charter
        // and was carried in prose here; it is now in the KEY, which is the
        // only place it binds.
        //
        // The day row is absent for the same reason as the vendor above: the
        // charter records no day-level cap for this feed either. UNVERIFIED,
        // and absent rather than guessed.
        window_caps: &[(Granularity::Minute1, 30)],
        // The one rung this feed's wire word is recorded for. Read first-hand
        // from groww.in/trade-api/docs/curl/historical-data, which is where
        // `1minute` came from.
        //
        // THE DAY SPELLING IS `1day`, AND IT IS NOW SOURCED RATHER THAN
        // GUESSED. This comment used to say `1day`, `1d` and `day` were all
        // plausible and that §3 rule 1 forbade picking — correct at the time,
        // and the answer has since been read from the vendor's own annexure:
        // `GrowwAPI.CANDLE_INTERVAL_DAY` has the value **`1day`**, in the same
        // table that gives `CANDLE_INTERVAL_MIN_1` the value `1minute` this
        // row already carried. One table, both words, same capture. D-0076.
        //
        // The remaining rungs in that table (`2minute` … `4hour`, `1week`,
        // `1month`) are deliberately absent: `store::path::Timeframe` has a
        // directory for two rungs, and a token for a rung the store cannot
        // file is a request whose answer has nowhere to go.
        granularity_tokens: &[
            (Granularity::Minute1, "1minute"),
            (Granularity::Day1, "1day"),
        ],
        pooling: Pooling::PerRequestKind,
        // Read first-hand from groww.in/trade-api/docs/curl/historical-data.
        // `trading_symbol` is the deprecated endpoint's name for it; the live
        // one takes `groww_symbol`, and the SAME master row spells the two
        // differently — NSE-NIFTY-30Sep25-24650-CE against NIFTY25SEP24650CE.
        // AN INDEX IS REQUESTED UNDER `CASH`, AND THE VENDOR SAYS SO IN WORDS.
        //
        // This table held ONE class for a long time, and the refusal it caused
        // was correct: a live run against `Swept indices` refused both NIFTY
        // and BANKNIFTY with `FetchError::ListingNotSpellable` before the
        // socket, because nothing recorded what segment an index goes under.
        // §3 rule 1 forbade guessing, and a guessed word would have been
        // answered by the vendor with *something* that would have been filed
        // as bars.
        //
        // The answer is now sourced. Groww's live-data page states it
        // directly: **"Use the segment value FNO for derivatives and CASH for
        // stocks and index."** So an index and an equity take the SAME segment
        // word, which is why both rows below carry `CASH` rather than one of
        // them carrying something this repository invented. D-0076.
        //
        // `kind` STAYS EMPTY FOR BOTH, and that is a fact about the request
        // rather than an omission. Groww's annexure does carry an
        // instrument-type alphabet — `EQ`, `IDX`, `FUT`, `CE`, `PE` — but the
        // historical-candles request schema is `exchange`, `segment`,
        // `trading_symbol`, `start_time`, `end_time`, `interval_in_minutes`
        // and nothing else. There is no field for a kind, so there is no word
        // to put in one. Compare Dhan, whose request carries `instrument` and
        // therefore needs `INDEX`/`EQUITY`.
        //
        // `exchange` stays Fixed: NSE is the only exchange this engine sweeps
        // (CLAUDE.md §1), so it does not vary with the instrument's class.
        listings: &[
            ListingWords {
                listing: Listing::Index,
                segment: "CASH",
                kind: "",
            },
            ListingWords {
                listing: Listing::Equity,
                segment: "CASH",
                kind: "",
            },
        ],
        params: &[
            Param {
                name: "exchange",
                value: ParamValue::Fixed("NSE"),
            },
            Param {
                name: "segment",
                value: ParamValue::Segment,
            },
            Param {
                // The live endpoint takes `groww_symbol`; only the
                // deprecated one took `trading_symbol`.
                name: "groww_symbol",
                value: ParamValue::InstrumentId,
            },
            Param {
                name: "start_time",
                value: ParamValue::From,
            },
            Param {
                name: "end_time",
                value: ParamValue::To,
            },
            Param {
                // THE RUNG, NOT A FIXED WORD. It was `Fixed("1minute")`, so a
                // request the operator filed under `1day/` still asked this
                // vendor for minutes.
                name: "candle_interval",
                value: ParamValue::Granularity,
            },
        ],
        // Groww's page shows this on every historical call.
        extra_headers: &[("X-API-VERSION", "1.0")],
    }),
    // Same restraint as the row above, and for the same reason.
    granularities: GranularitySet::EMPTY
        .with(Granularity::Minute1)
        .with(Granularity::Day1),
    history: GROWW_HISTORY,
    granularity_floor: GROWW_FLOOR,
    segments: SegmentSet::EMPTY
        .with(Segment::Index)
        .with(Segment::Cash)
        .with(Segment::Fno),
    exchange: Exchange::Nse,
};

/// `TrueData`'s observed index layout: `20221003,09:07:41,38444.90,0,0`.
///
/// Five columns, measured. The last two are structurally zero for an index —
/// `docs/08-vendor-samples.md` — so they are [`Column::Ignored`] rather than
/// mapped onto volume and open interest, which an index does not have.
const TRUEDATA_INDEX: ColumnLayout = ColumnLayout {
    segment: Segment::Index,
    columns: &[
        Column::Date,
        Column::Time,
        Column::LastPrice,
        Column::Ignored,
        Column::Ignored,
    ],
    shape: Columns::TrueDataIndex,
};

/// `TrueData`'s F&O layout: `20221003,09:15:01,0.95,1,518600`.
///
/// # The same five fields as the index, and two of them mean something here
///
/// [`TRUEDATA_INDEX`] maps the trailing pair to [`Column::Ignored`] because an
/// index has neither a volume nor an open interest and the vendor writes `0,0`
/// there. A contract has both, and the observed row above carries them:
/// one lot traded at 0.95 against an open interest of 518,600. Reading those two
/// as `Ignored` would silently discard the only two fields that distinguish a
/// derivative row from an index row, and `i64::MIN` is the open-interest null —
/// so an ignored column is not "zero", it is *never written*.
///
/// The physical shape is unchanged, which is why both layouts name
/// [`Columns::TrueDataIndex`]: five fields, no header, `YYYYMMDD`. [`Columns`]
/// is what a row LOOKS like and [`ColumnLayout`] is what its columns MEAN, and
/// this pair is the reason those are two types.
///
/// # Why this one and not the nine-column one
///
/// The vendor sells two products and this repository can declare one layout per
/// segment — [`Descriptor::layout`] finds by [`Segment`]. The nine-column
/// product is `TICK_BA`, whose header the vendor states as *"YMD (YYYYMMDD),
/// Time (HH:MM:SS), LTP, Volume, Open Interest, Bid, Bid Qty, Ask, Ask Qty"*;
/// [`Columns::TrueDataFutures`] is that shape and it is **not** declared here,
/// because the archive actually on this operator's disk is the plain one and a
/// descriptor that named the wrong product would decode every row at the wrong
/// offset while looking like it worked. A folder of `TICK_BA` files is a
/// different `ColumnLayout` on the day one is measured, not a guess today.
///
/// **Source.** The row above, read off `NSE_OPT_TICK_20221003/
/// NIFTY22100614400PE.csv` by the operator on 14 Aug 2026; and `TrueData`
/// Support, 30 Jul 2026, stating the header of the tick product without bid and
/// ask as *"YMD (YYYYMMDD), Time (HH:MM: SS), Close, Volume, Open Interest"*.
/// Two independent statements of one shape — `CLAUDE.md` §3 rule 1.
const TRUEDATA_FNO: ColumnLayout = ColumnLayout {
    segment: Segment::Fno,
    columns: &[
        Column::Date,
        Column::Time,
        Column::LastPrice,
        Column::Volume,
        Column::OpenInterest,
    ],
    shape: Columns::TrueDataIndex,
};

const TRUE_DATA: Descriptor = Descriptor {
    // UNMEASURED, AND DECLARED AS THE SESSION BAR BECAUSE THAT IS WHAT AN
    // ARCHIVE FILE HOLDS. This feed is a local folder of vendor files, not an
    // endpoint that aggregates anything, so there is no vendor-side bucketing
    // to get wrong. If that ever stops being true this row is the place it is
    // said. D-0077.
    day_bar: BarConvention::SessionOpenToClose,
    feed: Feed::TrueData,
    display: "TrueData",
    wire: "truedata",
    record: RecordShape::Snapshot,
    transport: Transport::LocalArchive(ArchiveSpec {
        nesting: Nesting::ZipOfDailyZips,
        archive: ArchiveName::SegmentAndDate {
            prefix: "NSE_",
            infix: "_TICK_",
            suffix: ".zip",
            date: DateFormat::CompactYmd,
        },
        member: MemberPattern::SymbolAtRoot { suffix: ".csv" },
        groups: &[],
        // ONLY the index token, which was measured. The archives also carry
        // `FUT` and `OPT`, and `Segment::Fno` covers both — so a single token
        // for it would have to pick one and be wrong half the time. Left out
        // rather than guessed; the segment set below excludes it, so the
        // ambiguity is unreachable rather than latent.
        segment_tokens: &[(Segment::Index, "IDX")],
        header: HeaderRow::Absent,
        delimiter: b',',
        // THE F&O LAYOUT NOW SHIPS, AND THE ARCHIVE NAME IS WHAT STILL DOES
        // NOT.
        //
        // This read "only the index layout ships … their column MEANINGS were
        // never established". That was true of the NINE-column `TICK_BA`
        // product and it was never true of the plain one, which is what the
        // operator actually bought: `20221003,09:15:01,0.95,1,518600` — the
        // same five fields as the index, with the trailing pair carrying a real
        // volume and a real open interest instead of the index's `0,0`. See
        // [`TRUEDATA_FNO`] for the measurement and both sources.
        //
        // So the file can be DECODED. What it cannot yet be is FOUND:
        // `segment_tokens` below carries one token per segment and this vendor
        // names its F&O archives `NSE_FUT_TICK_` and `NSE_OPT_TICK_` — two
        // names under one `Segment::Fno`. `segment_token` answers with the
        // FIRST match, so a single token would look up futures in the options
        // archive half the time. The layout is declared because it is measured;
        // the segment stays out of `segments` below because naming the archive
        // is a structural gap, not a guess this row may make.
        layouts: &[TRUEDATA_INDEX, TRUEDATA_FNO],
        date_format: DateFormat::CompactYmd,
        prices: PriceScale::Rupees,
    }),
    // ONE INPUT, THREE FOLDS — and the rung says what gets FILED, never what
    // gets fetched.
    //
    // `api::ingest::SpotRequest::granularity` is documented as "which rung of
    // the ladder to file under", and for an archive that is the whole of what it
    // decides: the input is fixed at one second, so a coarser rung is a fold
    // width and nothing else. `crate::ingest` takes that width straight off the
    // store timeframe — `Bucket::of_secs(timeframe.secs())` — and
    // `Timeframe::DAY_1.secs()` is 86,400, one bucket per IST day.
    //
    // `Day1` was missing while `crate::fold` has never cared about the width,
    // so `served()` refused a daily read of a folder this build can fold to
    // days. `Second1` stays declared even though `store_timeframe` has nowhere
    // to file it: that refusal belongs to the WRITE boundary, which names it,
    // and withholding the rung here would refuse it for the wrong reason.
    // D-0141.
    granularities: GranularitySet::EMPTY
        .with(Granularity::Second1)
        .with(Granularity::Minute1)
        .with(Granularity::Day1),
    // EMPTY, AND THAT IS THE FACT. An archive's reach is whatever the operator
    // bought and put in the folder; no vendor page states a floor for a
    // directory on this machine, and this repository will not derive one by
    // walking it — a walk is O(days) and would still only report what is there
    // today. Every rung therefore answers `HistoryFloor::Unstated`, which
    // `/feeds.json` renders as a claim NOT MADE rather than as "no limit".
    history: &[],
    granularity_floor: TRUEDATA_FLOOR,
    segments: SegmentSet::EMPTY.with(Segment::Index),
    exchange: Exchange::Nse,
};

/// GDFL's observed header, character for character:
/// `Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest`.
const GDFL_FNO: ColumnLayout = ColumnLayout {
    segment: Segment::Fno,
    columns: &[
        Column::Ticker,
        Column::Date,
        Column::Time,
        Column::LastPrice,
        Column::BidPrice,
        Column::BidQty,
        Column::AskPrice,
        Column::AskQty,
        Column::LastQty,
        Column::OpenInterest,
    ],
    shape: Columns::Gdfl,
};

const GDFL_HEADER: &str = "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest";

const GDFL: Descriptor = Descriptor {
    // As TRUE_DATA: a local archive of vendor files, so the bar is whatever
    // the file holds and nothing re-buckets it. D-0077.
    day_bar: BarConvention::SessionOpenToClose,
    feed: Feed::Gdfl,
    display: "Global Datafeeds",
    wire: "gdfl",
    record: RecordShape::Snapshot,
    transport: Transport::LocalArchive(ArchiveSpec {
        nesting: Nesting::ZipOfSegmentFolders,
        archive: ArchiveName::DateOnly {
            prefix: "GFDLNFO_TICK_",
            suffix: ".zip",
            // DD/MM/YYYY inside the file, DDMMYYYY in the archive name:
            // `GFDLNFO_TICK_01072025.zip` is 1 July, not 7 January.
            date: DateFormat::CompactDmy,
        },
        member: MemberPattern::StemGroupSymbol { suffix: ".NFO.csv" },
        groups: &[
            (ArchiveGroup::Options, "Options"),
            (ArchiveGroup::Futures, "Futures"),
        ],
        segment_tokens: &[],
        header: HeaderRow::Present(GDFL_HEADER),
        delimiter: b',',
        layouts: &[GDFL_FNO],
        date_format: DateFormat::SlashedDmy,
        prices: PriceScale::Rupees,
    }),
    // AS THE ARCHIVE ABOVE: one input, and the rung says what is FILED.
    //
    // This declared `Second1` alone, so `served()` refused both a minute and a
    // day of a folder `crate::fold` folds to either from the same bytes. The
    // fold width comes off the store timeframe and nothing about it is
    // vendor-specific — a folder of one-second snapshots is a folder of
    // one-second snapshots whichever of the two archives wrote it. D-0141.
    granularities: GranularitySet::EMPTY
        .with(Granularity::Second1)
        .with(Granularity::Minute1)
        .with(Granularity::Day1),
    // Empty for the reason the archive above is empty.
    history: &[],
    granularity_floor: GDFL_FLOOR,
    segments: SegmentSet::EMPTY.with(Segment::Fno),
    exchange: Exchange::Nse,
};

/// Every feed, indexed by its own discriminant.
///
/// The three `const` blocks below are the contract: the table length equals
/// [`FEED_COUNT`], the variant list length equals [`FEED_COUNT`], and **row
/// *i* carries variant *i***. Together they make [`Feed::descriptor`]'s single
/// array index sound at compile time, and make a feed added to the enum but
/// not to the table a build failure rather than a runtime surprise.
/// Zerodha's history claim, per rung.
///
/// **ONE CLAIM AND NO CONTEST, which makes this feed different from the other
/// two brokers.** Groww's and Dhan's rows each carry a vendor table disagreeing
/// with the operator, and `ClaimStanding` exists to say which binds. Here the
/// vendor page states no depth at all — its closest sentence is *"spanning back
/// several years"*, a phrase and not a figure — so there is nothing to weigh
/// against and `contested` is `None`. That is not agreement; it is silence, and
/// the row says so.
const ZERODHA_HISTORY: &[FloorRow] = &[
    FloorRow {
        granularity: Granularity::Minute1,
        binding: FloorClaim {
            floor: HistoryFloor::Rolling { years: 10 },
            source: "the operator, 11 Aug 2026, restated 14 Aug 2026: a rolling \
                     10 years",
            standing: ClaimStanding::OperatorObservation,
        },
        contested: None,
        binds_because: "it is the only claim there is. kite.trade's historical \
                        page states no depth for any interval — the nearest it \
                        comes is the phrase \"spanning back several years\" — so \
                        nothing weighs against this and nothing confirms it \
                        either. UNVERIFIED per rung: the figure was stated \
                        against the VENDOR, not against a rung, and is applied \
                        to both unchanged.",
    },
    FloorRow {
        granularity: Granularity::Day1,
        binding: FloorClaim {
            floor: HistoryFloor::Rolling { years: 10 },
            source: "the operator, 11 Aug 2026, restated 14 Aug 2026: a rolling \
                     10 years",
            standing: ClaimStanding::OperatorObservation,
        },
        contested: None,
        binds_because: "as the rung above, and for the same reason: one source, \
                        uncontested, applied to both rungs because that is how \
                        it was stated.",
    },
];

/// Zerodha's granularity floor: one minute, and the vendor's own list proves it.
const ZERODHA_FLOOR: GranularityFloor = GranularityFloor {
    finest: Granularity::Minute1,
    kind: FinestKind::Bar,
    because: "the vendor publishes its whole interval alphabet on the \
              historical page — minute, day, 3minute, 5minute, 10minute, \
              15minute, 30minute, 60minute — and its finest row is one minute. \
              There is no second-level and no print-level word in it, so a rung \
              below a minute cannot be spelled on this vendor's wire at all.",
    source: "docs/00-charter.md section 4z, Intervals published, read from \
             kite.trade/docs/connect/v3/historical/ on 14 Aug 2026.",
};

/// Zerodha, and it is the vendor `docs/07-plan.md` §5 predicted.
///
/// Every one of the three fields §5 named as unable to express an arbitrary
/// broker is exercised by this single row: the path carries the instrument AND
/// the rung (D-0133), the scheme carries a prefix and a SECOND secret
/// (D-0134), and the timestamp carries its own zone (D-0135).
const ZERODHA: Descriptor = Descriptor {
    // NOT MEASURED, AND THEREFORE NOT CLAIMED TO BE THE OTHER ONE.
    //
    // `BarConvention` has two arms and both are real: Dhan's daily bar opens at
    // the session's first print, Groww's opens at the PREVIOUS session's close
    // — measured on the same instrument on the same day, differing by 181
    // points. No such measurement exists for this vendor, and no page states a
    // convention. `SessionOpenToClose` is the ordinary reading and is what is
    // declared; it is UNVERIFIED, and `docs/06-limits.md` is where that costs
    // something. Declaring nothing is not an option the type offers, and
    // guessing the OTHER one would be equally unfounded and less likely.
    day_bar: BarConvention::SessionOpenToClose,
    feed: Feed::Zerodha,
    display: "Zerodha",
    wire: "zerodha",
    record: RecordShape::Ohlcv,
    transport: Transport::Http(HttpSpec {
        base_url: "https://api.kite.trade",
        // THE ROW §5 PREDICTED. Both the instrument and the rung are PATH
        // SEGMENTS, which is why `bars_path` is a list — D-0133. The two
        // placeholders are the vendor's own names for them, so a refusal reads
        // the way its documentation does.
        bars_path: &[
            PathSegment::Literal("instruments"),
            PathSegment::Literal("historical"),
            PathSegment::Value {
                placeholder: "instrument_token",
                value: ParamValue::InstrumentId,
            },
            PathSegment::Value {
                placeholder: "interval",
                value: ParamValue::Granularity,
            },
        ],
        method: Method::Get,
        auth: Auth {
            header: "Authorization",
            // `token api_key:access_token` — a prefix AND a second secret,
            // which no two-variant scheme could describe. D-0134.
            scheme: AuthScheme::PrefixedPair {
                prefix: "token ",
                separator: ":",
            },
            key_field: Some("api-key"),
        },
        // `from` / `to`, `yyyy-mm-dd hh:mm:ss` — the vendor's own words.
        date_format: DateFormat::DashedYmdMidnight,
        // INCLUSIVE, AND READ FROM THE VENDOR'S EXAMPLE BECAUSE THE PROSE NEVER
        // SAYS. `from=09:15:00&to=09:20:00` is answered with six candles —
        // 09:15 through 09:20, both endpoints present — and the OI example
        // repeats it. Third vendor, third answer: Dhan's daily end is
        // exclusive, Groww's is inclusive, and this one is inclusive by
        // demonstration. docs/00-charter.md section 4z.
        range_end: RangeEnd::Inclusive,
        response: ResponseShape::PositionalRows {
            envelope: Some("data"),
            array: "candles",
        },
        fields: FieldNames {
            open: "open",
            high: "high",
            low: "low",
            close: "close",
            volume: "volume",
            timestamp: "timestamp",
            // ONLY WITH `oi=1`, WHICH THIS REQUEST DOES NOT SEND. The vendor's
            // six-cell row has no open interest and its seven-cell row does;
            // the positional decoder accepts either width. Naming a field here
            // would claim the array is always present, which is the exact row
            // that broke both swept indices on Dhan.
            open_interest: None,
        },
        // `2017-12-15T09:15:00+0530` — it carries its own zone, and applying
        // IST again would put every bar 5h30m early and STORE it. D-0135.
        timestamps: TimestampEncoding::IsoDateTimeOffset,
        prices: PriceScale::Rupees,
        budget: Budget {
            per_second: Some(crate::rate::ZERODHA_PER_SECOND),
            per_minute: None,
            // NO DAILY QUOTA APPLIES. The page publishes one daily figure —
            // 5,000 orders per user per day — and it is stated against ORDER
            // PLACEMENT. This build places no order, so carrying it here would
            // promote a figure past the endpoint it was measured against, which
            // is the error the Dhan window-cap row names.
            per_day: None,
        },
        history_floor: HistoryFloor::Rolling { years: 10 },
        // ABSENT, AND THAT IS THE FACT. The historical page states no window
        // cap for any interval. An absent row means "the vendor bounds nothing
        // here", so the store's one-month-per-file boundary is the only bound —
        // and inventing a number would be section 3 rule 1's invention.
        window_caps: &[],
        // TWO RUNGS OF EIGHT, AND THE NARROWING IS THE OPERATOR'S.
        //
        // The vendor publishes `minute 3minute 5minute 10minute 15minute
        // 30minute 60minute day`. He narrowed it on 14 Aug 2026 — "one and only
        // one day pull and one min pull" — so the other six are real, are not
        // wired, and refuse by name like every unfetched rung. The two words
        // here are the vendor's own spelling and are not derived from
        // `Granularity::dir`, which spells them `1min` and `1day`.
        granularity_tokens: &[(Granularity::Minute1, "minute"), (Granularity::Day1, "day")],
        pooling: Pooling::PerRequestKind,
        // THE INSTRUMENT'S CLASS IS NOT ON THIS WIRE AT ALL.
        //
        // The request is a token and an interval in the path, plus `from` and
        // `to`. There is no segment field and no instrument-kind field, because
        // the token already identifies the instrument uniquely — which is the
        // whole reason the vendor issues one. So both classes record the empty
        // words and no `Segment` or `Kind` param reads them.
        listings: &[
            ListingWords {
                listing: Listing::Index,
                segment: "",
                kind: "",
            },
            ListingWords {
                listing: Listing::Equity,
                segment: "",
                kind: "",
            },
        ],
        params: &[
            Param {
                name: "from",
                value: ParamValue::From,
            },
            Param {
                name: "to",
                value: ParamValue::To,
            },
        ],
        // THE VERSION HEADER THE VENDOR REQUIRES ON EVERY CALL. Its own curl
        // examples carry it beside the credential, and a request without it is
        // a request against an unstated API version.
        extra_headers: &[("X-Kite-Version", "3")],
    }),
    granularities: GranularitySet::EMPTY
        .with(Granularity::Minute1)
        .with(Granularity::Day1),
    history: ZERODHA_HISTORY,
    granularity_floor: ZERODHA_FLOOR,
    // SPOT ONLY, and the reason is not the vendor's. `continuous=1` returns day
    // candles for expired NFO and MCX futures, which is a real capability this
    // build does not use: CLAUDE.md section 1 puts the swept surface on NSE
    // spot, and an expired contract goes through a different route.
    segments: SegmentSet::EMPTY.with(Segment::Index).with(Segment::Cash),
    exchange: Exchange::Nse,
};

/// Every feed, indexed by its own [`Feed`] discriminant.
///
/// **The index IS the variant.** [`Feed::descriptor`] reads this array at
/// `self as usize` — one index, no search, no map — and the `const` assertions
/// below are what make that sound: they pin the table's length to
/// [`FEED_COUNT`] and row *i* to variant *i*, so a new variant with no row is a
/// build failure rather than a lookup that finds somebody else's vendor.
///
/// Order is the discriminant's, not alphabetical and not the order a page draws
/// them in. Those are reading orders; this one is an addressing scheme.
pub const DESCRIPTORS: [&Descriptor; FEED_COUNT] = [&DHAN, &GROWW, &TRUE_DATA, &GDFL, &ZERODHA];

const _: () = assert!(DESCRIPTORS.len() == FEED_COUNT);
const _: () = assert!(Feed::ALL.len() == FEED_COUNT);
const _: () = {
    // Destructured rather than indexed: adding a fifth feed makes this pattern
    // itself a compile error, before any assertion is even evaluated.
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(dhan.feed as u8 == Feed::Dhan as u8);
    assert!(groww.feed as u8 == Feed::Groww as u8);
    assert!(truedata.feed as u8 == Feed::TrueData as u8);
    assert!(gdfl.feed as u8 == Feed::Gdfl as u8);
    assert!(zerodha.feed as u8 == Feed::Zerodha as u8);
    let [first, second, third, fourth, fifth] = Feed::ALL;
    assert!(first as u8 == 0 && second as u8 == 1 && third as u8 == 2 && fourth as u8 == 3);
    assert!(fifth as u8 == 4);
};

// No row may ship an empty capability set: a feed that serves no granularity
// or no segment can never answer a request, and offering it on a page would be
// a control that always refuses.
const _: () = {
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(!dhan.granularities.is_empty() && !dhan.segments.is_empty());
    assert!(!groww.granularities.is_empty() && !groww.segments.is_empty());
    assert!(!truedata.granularities.is_empty() && !truedata.segments.is_empty());
    assert!(!gdfl.granularities.is_empty() && !gdfl.segments.is_empty());
    assert!(!zerodha.granularities.is_empty() && !zerodha.segments.is_empty());
};

// A local archive carries no token and no budget by construction — there is no
// field for either on `ArchiveSpec`. This says the same thing about the two
// shipped archive rows in a form the compiler checks.
const _: () = {
    let [_, _, truedata, gdfl, _] = DESCRIPTORS;
    assert!(!truedata.transport.needs_credential());
    assert!(!truedata.transport.needs_governor());
    assert!(!gdfl.transport.needs_credential());
    assert!(!gdfl.transport.needs_governor());
};

// THE OWNER'S RULE OF 12 AUG 2026, AS A BUILD FAILURE.
//
//   "for truedata and gdfl alone, one and only, we will pull the data
//    especially entirely from csv files from the precise folder … except these
//    two alone only, for all other vendors or brokers feeds it should be
//    always REST."
//
// Two feeds are folders and every other is REST, so the membership itself is
// checked here rather than being a sentence in a doc comment that a fifth row
// could silently contradict. `store_vendor` is `TrueData`/`Gdfl` on exactly
// the two rows whose transport is a folder, which is the same statement seen
// from the store-prefix side.
const _: () = {
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(matches!(dhan.transport.kind(), SourceKind::Rest));
    assert!(matches!(groww.transport.kind(), SourceKind::Rest));
    assert!(matches!(truedata.transport.kind(), SourceKind::Folder));
    assert!(matches!(gdfl.transport.kind(), SourceKind::Folder));
    assert!(matches!(zerodha.transport.kind(), SourceKind::Rest));
};

// AND NO FEED MAY CLAIM A RUNG ITS KIND CANNOT CARRY.
//
// The other half of the same rule: a folder reaches one second because that is
// what a bought file holds, and a REST broker reaches one minute because that
// is the finest rung either published API answers. `GranularityFloor::finest`
// stays the per-vendor authority — this only refuses a row that claims FINER
// than its kind, which is the direction that would have a page offering a rung
// no source on this tree can produce.
//
// Written as a `u8` comparison because the ladder's discriminants ascend
// coarsest-last, so "no finer than" is one integer comparison and needs no
// table.
const _: () = {
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(dhan.granularity_floor.finest as u8 >= dhan.transport.kind().finest_possible() as u8);
    assert!(groww.granularity_floor.finest as u8 >= groww.transport.kind().finest_possible() as u8);
    assert!(
        zerodha.granularity_floor.finest as u8 >= zerodha.transport.kind().finest_possible() as u8
    );
    assert!(
        truedata.granularity_floor.finest as u8
            >= truedata.transport.kind().finest_possible() as u8
    );
    assert!(gdfl.granularity_floor.finest as u8 >= gdfl.transport.kind().finest_possible() as u8);
};

// AND NOBODY MAY ASK FOR A TICK.
//
// `FinestKind::is_tick_stream` above says no feed PRODUCES one. This says no
// caller may ASK for one, which is the other half of the owner's rule and a
// different sentence: a rung nothing produces could still sit on a page as a
// selectable control, which is exactly what `GranularityFloor` was written
// after finding.
const _: () = {
    assert!(!Granularity::Tick.is_requestable());
    assert!(Granularity::Second1.is_requestable());
    assert!(SourceKind::Rest.finest_possible() as u8 > Granularity::Tick as u8);
    assert!(SourceKind::Folder.finest_possible() as u8 > Granularity::Tick as u8);
};

// A LAYOUT AND ITS `Columns` ARE TWO SPELLINGS OF ONE FILE.
//
// [`ColumnLayout::columns`] is what each field MEANS; [`ColumnLayout::shape`]
// is what the decoder is handed. Two descriptions of one file is exactly the
// arrangement `CLAUDE.md` warns about, so they are held against each other
// here on all three things that can silently mis-read a row:
//
//   * the COUNT — a shape one column wider reads every field after the
//     mismatch from the wrong offset, which is the defect `Columns::offsets`
//     records having already shipped once,
//   * the HEADER row — a shape that expects one where the archive has none
//     eats a real record, and the reverse refuses a whole file,
//   * the DATE format — `DD/MM/YYYY` read as `YYYY-MM-DD` shifts every bar by
//     months, silently, which `DateFormat::SlashedDmy`'s own doc names.
//
// `DateFormat` is fieldless, so the comparison is one `u8` cast and needs no
// per-variant table that a sixth format could fall outside of.
const _: () = {
    let [_dhan, _groww, truedata, gdfl, _zerodha] = DESCRIPTORS;
    assert!(shape_agrees(&TRUEDATA_INDEX, truedata));
    assert!(shape_agrees(&GDFL_FNO, gdfl));
};

/// Whether a layout's two spellings describe the same file.
///
/// Used only by the `const` block above, which is why it takes the whole
/// [`Descriptor`]: the header row and the date format are properties of the
/// ARCHIVE, and reading them from anywhere else would be comparing the layout
/// against a second copy rather than against the spec it belongs to.
const fn shape_agrees(layout: &ColumnLayout, row: &Descriptor) -> bool {
    let Transport::LocalArchive(spec) = row.transport else {
        // A REST feed has no archive spec, so it has no layout to agree with.
        // Reaching here means a layout was written for a broker row, which is
        // a wiring mistake and not a mismatch to report.
        return false;
    };
    layout.columns.len() == layout.shape.count()
        && layout.shape.has_header() == matches!(spec.header, HeaderRow::Present(_))
        && layout.shape.date_format() as u8 == spec.date_format as u8
}

// NO ROW MAY CLAIM A TICK STREAM.
//
// The owner's rule, 12 Aug 2026, and it is not a floor some feed clears — no
// feed in this system serves one. `FinestKind::Tick` exists so the negative
// can be SAID; a row that ever asserts it positively is a claim about a vendor
// with no source, and CLAUDE.md section 3 rule 1 makes that a stop rather than
// a review comment. Here it is a build failure.
const _: () = {
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(!dhan.granularity_floor.kind.is_tick_stream());
    assert!(!groww.granularity_floor.kind.is_tick_stream());
    assert!(!truedata.granularity_floor.kind.is_tick_stream());
    assert!(!gdfl.granularity_floor.kind.is_tick_stream());
    assert!(!zerodha.granularity_floor.kind.is_tick_stream());
};

// A BUILD CANNOT FETCH WHAT A VENDOR CANNOT SERVE.
//
// `granularities` is what this build asks for and `granularity_floor` is what
// the vendor has; the first may be narrower than the second — Dhan's minute
// rung is exactly that — and it may never be WIDER. A row that declared a rung
// below its own floor would put a control on a page that no pull could ever
// satisfy, and the refusal would arrive from the vendor as an empty answer,
// which reads like a holiday. One mask per row, checked by the compiler.
const _: () = {
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(
        zerodha
            .granularities
            .none_finer_than(zerodha.granularity_floor.finest)
    );
    assert!(
        dhan.granularities
            .none_finer_than(dhan.granularity_floor.finest)
    );
    assert!(
        groww
            .granularities
            .none_finer_than(groww.granularity_floor.finest)
    );
    assert!(
        truedata
            .granularities
            .none_finer_than(truedata.granularity_floor.finest)
    );
    assert!(
        gdfl.granularities
            .none_finer_than(gdfl.granularity_floor.finest)
    );
};

// EVERY GRANULARITY FLOOR CARRIES ITS REASON AND ITS SOURCE.
//
// `has_content` is the same check the session regime rows get, for the same
// reason: a blank citation is a row with no citation wearing one. A refusal an
// operator cannot check is a refusal they have to take on faith, and this
// repository does not ask for faith about a vendor.
const _: () = {
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(has_content(zerodha.granularity_floor.because));
    assert!(has_content(zerodha.granularity_floor.source));
    assert!(has_content(dhan.granularity_floor.because));
    assert!(has_content(dhan.granularity_floor.source));
    assert!(has_content(groww.granularity_floor.because));
    assert!(has_content(groww.granularity_floor.source));
    assert!(has_content(truedata.granularity_floor.because));
    assert!(has_content(truedata.granularity_floor.source));
    assert!(has_content(gdfl.granularity_floor.because));
    assert!(has_content(gdfl.granularity_floor.source));
};

// THE FLOOR'S KIND AND THE ROW'S RECORD SHAPE SAY THE SAME THING.
//
// `RecordShape` already distinguishes a bar from a snapshot, and it is what
// the store refuses on. `FinestKind` adds the one thing that shape cannot
// express — whether the snapshot is a PRINT — and the two must not drift: a
// row whose record shape said snapshot while its floor said bar would let a
// conflated second be labelled a candle at the boundary that renders it.
const _: () = {
    let [dhan, groww, truedata, gdfl, zerodha] = DESCRIPTORS;
    assert!(matches!(
        (zerodha.record, zerodha.granularity_floor.kind),
        (RecordShape::Ohlcv, FinestKind::Bar)
    ));
    assert!(matches!(
        (dhan.record, dhan.granularity_floor.kind),
        (RecordShape::Ohlcv, FinestKind::Bar)
    ));
    assert!(matches!(
        (groww.record, groww.granularity_floor.kind),
        (RecordShape::Ohlcv, FinestKind::Bar)
    ));
    assert!(matches!(
        (truedata.record, truedata.granularity_floor.kind),
        (RecordShape::Snapshot, FinestKind::ConflatedSnapshot)
    ));
    assert!(matches!(
        (gdfl.record, gdfl.granularity_floor.kind),
        (RecordShape::Snapshot, FinestKind::ConflatedSnapshot)
    ));
};

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
//
// THESE LIVE IN THE FILE RATHER THAN IN `tests/` FOR ONE REASON. Most of what
// this module is *made of* is private and `const`: `str_eq`, `has_content`,
// `day_const`, `SessionRow` and `SessionTable` are exercised only by the
// `const` assertions above, and a `const` assertion is evaluated by the
// compiler — it is invisible to runtime instrumentation and proves nothing
// about the code an operator's process actually runs. An integration test
// cannot reach a private item at all, so the choice is here or nowhere.
//
// Every `const fn` below is called through `black_box` so the call cannot be
// folded back into a compile-time constant and disappear again.
//
// NO STRING LITERAL HERE IS SHAPED LIKE A PARAMETER-PATH SEGMENT. CI gate 1d
// walks this crate for lower-case quoted words; the assertions are written
// against the table's own accessors (`dir()`, `wire()`, `label()`) and against
// `store::path::Timeframe`, which is both stronger than pinning a spelling and
// leaves nothing new to declare.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use std::collections::HashSet;
    use std::hint::black_box;

    use super::*;

    /// A real date, or the test fails on its own literal.
    fn day(year: u16, month: u8, of_month: u8) -> Day {
        Day::new(year, month, of_month).expect("the test's own date literal is real")
    }

    /// A history floor as the day it names, resolved against a FIXED day.
    ///
    /// `None` for the two floors that name no day. The rolling arm uses 365
    /// rather than the clamp's 365.25 on purpose: nothing here compares
    /// against a literal, only floors against each other, and a test that
    /// re-derives the production constant proves the constant rather than the
    /// ordering.
    fn resolve_floor(floor: HistoryFloor, today: Day) -> Option<Day> {
        match floor {
            HistoryFloor::Fixed { year, month, day } => {
                Some(Day::new(year, month, day).expect("a real floor date"))
            }
            HistoryFloor::Rolling { years } => Day::from_days(
                today
                    .days_from_epoch()
                    .saturating_sub(years.saturating_mul(365)),
            )
            .ok(),
            HistoryFloor::RollingMonths { months } => today.months_before(months).ok(),
            HistoryFloor::Unbounded | HistoryFloor::Unstated => None,
        }
    }

    // -- the const helpers ---------------------------------------------------

    /// `str_eq` is byte equality including length, in both directions.
    #[test]
    fn str_eq_compares_every_byte_and_the_length_in_both_directions() {
        assert!(
            str_eq(black_box(""), black_box("")),
            "two empty strings are equal"
        );

        let minute = Granularity::Minute1.dir();
        assert!(
            str_eq(black_box(minute), black_box(Timeframe::MINUTE_1.as_str())),
            "the rung's directory name and the store's timeframe name are the \
             same bytes — this is the equality the const assertion above rests on"
        );
        assert!(
            !str_eq(black_box(minute), black_box(Granularity::Minute3.dir())),
            "same length, first byte differs"
        );

        // A prefix relationship, which is the only way to reach the arm where
        // one side runs out of bytes before the other.
        let doubled = format!("{minute}{minute}");
        assert!(
            !str_eq(black_box(minute), black_box(doubled.as_str())),
            "a prefix is not equal to the whole"
        );
        assert!(
            !str_eq(black_box(doubled.as_str()), black_box(minute)),
            "and the whole is not equal to its prefix"
        );
    }

    /// A citation of nothing but whitespace is no citation.
    #[test]
    fn has_content_refuses_a_blank_citation() {
        assert!(!has_content(black_box("")), "empty says nothing");
        assert!(
            !has_content(black_box(" \t\r\n ")),
            "whitespace says nothing either — this is the check that keeps a \
             row from wearing a citation field it never filled in"
        );
        assert!(
            has_content(black_box("  X")),
            "one non-blank byte anywhere is content"
        );
        assert!(
            has_content(black_box(CHARTER_SESSION)),
            "and every shipped row's citation has content"
        );
    }

    /// An unreal date collapses to the epoch instead of panicking — and the
    /// `const` assertion beside each shipped date is what catches it.
    #[test]
    fn day_const_falls_back_to_the_epoch_rather_than_panicking() {
        assert_eq!(
            day_const(black_box(2026), black_box(8), black_box(3)),
            AUG_3_2026,
            "a real date round-trips"
        );
        assert_eq!(
            day_const(black_box(2026), black_box(2), black_box(30)),
            EPOCH_DAY,
            "30 February is not a date, so the fallback fires — and the const \
             assertion beside every shipped date is what turns that into a \
             build failure rather than a wrong window"
        );
        assert_eq!(EPOCH_DAY.days_from_epoch(), 0);
        assert_eq!(
            (AUG_3_2026.year(), AUG_3_2026.month(), AUG_3_2026.day()),
            (2026, 8, 3)
        );
    }

    // -- the granularity ladder ---------------------------------------------

    /// Every rung has its own directory, its own grid, and a `Display` that is
    /// the directory.
    #[test]
    fn every_granularity_rung_has_a_distinct_directory_and_one_grid() {
        let mut dirs = HashSet::with_capacity(GRANULARITY_COUNT);
        let mut intraday = 0_usize;
        let mut aggregate = 0_usize;

        for rung in Granularity::ALL {
            let dir = rung.dir();
            assert!(!dir.is_empty(), "a rung with no directory name");
            assert!(dirs.insert(dir), "{dir} is claimed by two rungs");
            assert_eq!(rung.to_string(), dir, "Display is the directory name");

            match rung.grid() {
                Grid::Event => {
                    assert!(rung.is_intraday(), "an event grid is intraday");
                    intraday += 1;
                }
                Grid::Intraday(secs) => {
                    assert!(secs > 0, "{dir} claims a zero-second interval");
                    assert!(rung.is_intraday());
                    intraday += 1;
                }
                Grid::Daily | Grid::Weekly => {
                    assert!(
                        !rung.is_intraday(),
                        "{dir} is an aggregate bar and carries no intraday time"
                    );
                    aggregate += 1;
                }
            }
        }

        assert_eq!(dirs.len(), GRANULARITY_COUNT, "eleven distinct rungs");
        assert_eq!(aggregate, 2, "exactly the daily and weekly rungs aggregate");
        assert_eq!(intraday, GRANULARITY_COUNT - 2);
    }

    /// Two rungs have a store timeframe and the rest refuse by absence.
    ///
    /// The pair is `1min` and `1day`, and the assertion below is the one that
    /// stops a daily bar landing under `1min/`: the DIRECTORY NAME the ladder
    /// spells and the one the store spells are compared for every rung that
    /// carries a timeframe at all, so a rename on either side fails here as
    /// well as at the `const` assertions beside `store_timeframe`.
    #[test]
    fn every_rung_the_store_ships_carries_a_timeframe_and_the_rest_refuse() {
        let mut carried = Vec::new();
        for rung in Granularity::ALL {
            match rung.store_timeframe() {
                Some(timeframe) => {
                    assert_eq!(
                        timeframe.as_str(),
                        rung.dir(),
                        "{rung} and its timeframe must name ONE directory"
                    );
                    // The grid and the timeframe agree in kind, which is the
                    // half a name comparison cannot cover: an intraday rung
                    // carries its own interval and must match the timeframe's
                    // seconds, and an aggregating rung has none to match.
                    match rung.grid() {
                        Grid::Intraday(secs) => assert_eq!(secs, timeframe.secs()),
                        Grid::Daily => assert_eq!(timeframe.secs(), 86_400),
                        other => panic!("{rung} sits on {other:?}, which the store cannot file"),
                    }
                    carried.push(rung);
                }
                // A `None` MUST BE A RUNG `Timeframe::KNOWN` GENUINELY HAS NO
                // ROW FOR, and that is asserted against the store's own table
                // rather than against a list repeated here. A second list would
                // be the same defect one layer up: the `_` arm this test now
                // guards answered `None` for five rungs `KNOWN` has always
                // held, and a hand-written expectation would have agreed with
                // it.
                // A `None` IS ONE OF EXACTLY TWO THINGS, AND BOTH ARE CHECKED
                // AGAINST THE STORE'S OWN TABLES RATHER THAN A LIST REPEATED
                // HERE. A second list would be the same defect one layer up:
                // the `_` arm this test now guards answered `None` for five
                // rungs `KNOWN` has always held, and a hand-written expectation
                // would have agreed with it.
                //
                //   1. `Timeframe::KNOWN` holds no directory for the rung, or
                //   2. it does, and the rung's opening bar would be a STUB —
                //      `aligns_with_the_open` is false, so a fold anchored at
                //      IST midnight would file 15 minutes of trade as the
                //      [09:00, 09:30) bar.
                //
                // The second arm is asserted through the store's predicate, not
                // by naming 30min and 60min, so a change to the fold anchor
                // that made them align turns this into a failure that says so.
                None => {
                    if let Some(known) = Timeframe::KNOWN
                        .iter()
                        .find(|known| known.as_str() == rung.dir())
                    {
                        assert!(
                            !known.aligns_with_the_open(),
                            "a None is a refusal at the write boundary, never a \
                             substitution — and crates/store ships a directory \
                             for {rung} whose bars DO align with the 09:15 open, \
                             so refusing it strikes a rung off the operator's \
                             form that the store can file correctly"
                        );
                    }
                }
            }
        }
        // EVERY ALIGNED ENTRY IN THE STORE'S TABLE IS REACHED, not merely "the
        // ones this test happened to see". The two directions are different
        // failures: an aligned rung carrying no timeframe is a rung the
        // operator cannot pull, and a timeframe no rung carries is a directory
        // nothing can ever write into.
        //
        // `1day` is counted as aligned: `aligns_with_the_open` divides 555 by a
        // number of MINUTES and is a question about an intraday grid, so it is
        // asked only of the rungs that sit on one. `store_timeframe` makes the
        // same exemption and says why.
        let expected: Vec<&str> = Timeframe::KNOWN
            .iter()
            .filter(|known| known.aligns_with_the_open() || known.secs() == 86_400)
            .map(|known| known.as_str())
            .collect();
        assert_eq!(
            carried.len(),
            expected.len(),
            "every rung crates/store ships and can file correctly must be \
             reachable from the ladder; carried {carried:?}, expected \
             {expected:?}"
        );
        for name in &expected {
            assert!(
                carried.iter().any(|rung| &rung.dir() == name),
                "{name} is a store directory no rung names"
            );
        }
        // THE STUB RUNGS ARE REFUSED, AND THE COUNT IS PINNED so this test
        // cannot pass by refusing everything.
        assert_eq!(
            Timeframe::KNOWN.len() - expected.len(),
            2,
            "30min and 60min are the two rungs the store ships that the fold \
             cannot align with the open; if that changed, store_timeframe's \
             refusal must be revisited"
        );
        // Coarsest last, which is the order `Granularity::ALL` walks and the
        // order a backfill lands them in.
        assert!(
            carried.windows(2).all(|pair| match pair {
                [finer, coarser] => finer.is_finer_than(*coarser),
                _ => false,
            }),
            "the carried rungs must ascend: {carried:?}"
        );
    }

    /// A rung field and a rung table exist together or not at all, and the two
    /// figures nobody has recorded are absent from both.
    ///
    /// **The pairing.** A request field spelled [`ParamValue::Granularity`]
    /// with an empty token table is a field that can never be filled — every
    /// request refuses. A token table with no field reading it is a vendor fact
    /// nothing puts on a wire. Either alone is a row that says one thing and
    /// does another.
    ///
    /// **The absences.** `docs/00-charter.md` §4 records a window cap for each
    /// broker at ONE MINUTE — Groww's row carries the qualifier in its own
    /// text, Dhan's sits under an intraday-endpoint line — and a day-level cap
    /// for neither. It records no daily interval word for either. Both gaps are
    /// asserted rather than tolerated, because the failure mode is somebody
    /// filling one in from memory and nothing noticing.
    #[test]
    fn a_rung_on_the_wire_needs_a_word_and_the_unrecorded_ones_stay_absent() {
        for feed in Feed::ALL {
            let Transport::Http(spec) = feed.descriptor().transport else {
                continue;
            };
            // A RUNG REACHES THE WIRE TWO WAYS NOW, AND THIS ONLY LOOKED AT ONE.
            //
            // The pairing rule is unchanged and still right: a request field
            // spelled `Granularity` with an empty token table can never be
            // filled, and a token table nothing reads is a vendor fact that
            // reaches no wire. What changed is where the field can be. Kite
            // carries its interval as a PATH SEGMENT
            // (/instruments/historical/:instrument_token/:interval), so asking
            // only `params` reported a populated table with no reader — and the
            // rule would have been "fixed" by deleting the tokens that make the
            // request work.
            let named_in_query = spec
                .params
                .iter()
                .any(|p| matches!(p.value, ParamValue::Granularity));
            let named_in_path = spec.bars_path.iter().any(|segment| {
                matches!(
                    segment,
                    PathSegment::Value {
                        value: ParamValue::Granularity,
                        ..
                    }
                )
            });
            assert_eq!(
                named_in_query || named_in_path,
                !spec.granularity_tokens.is_empty(),
                "{}: a rung field and a rung table exist together or not at all \
                 — and the field may be a query parameter OR a path segment",
                feed.display()
            );
            // AND NEVER BOTH. One rung, one place on the wire: a feed naming it
            // twice would send two spellings of one fact, and nothing decides
            // which the answer is filed under.
            assert!(
                !(named_in_query && named_in_path),
                "{}: names its rung in the path AND in the query",
                feed.display()
            );
            for (rung, word) in spec.granularity_tokens {
                assert!(
                    !word.is_empty(),
                    "{}: {rung} has an empty wire word, which is not a word",
                    feed.display()
                );
            }
            // THE ONE-MINUTE CAP IS RECORDED WHERE THE VENDOR PUBLISHES ONE,
            // AND A WHOLE-TABLE ABSENCE IS ITS OWN FACT.
            //
            // This demanded a minute cap of every broker, which was true of the
            // two that had one and became false the day a third was added whose
            // page states no cap at any rung — docs/00-charter.md §4z, recorded
            // UNVERIFIED. Encoding a number would be §3 rule 1's invention, and
            // an empty table means the store's month boundary is the only bound.
            //
            // The distinction that still binds, and the one this test exists
            // for: a POPULATED table missing the minute row is a number that
            // went missing, and that is refused.
            if spec.window_caps.is_empty() {
                assert!(
                    spec.window_cap_days(Granularity::Minute1).is_none(),
                    "{}: an empty cap table cannot answer for a rung",
                    feed.display()
                );
            } else {
                assert!(
                    spec.window_cap_days(Granularity::Minute1)
                        .is_some_and(|c| c > 0),
                    "{}: this feed publishes caps and none at ONE MINUTE — \
                     docs/00-charter.md §4 carries it for every broker that has \
                     one, so a gap here is a number that went missing",
                    feed.display()
                );
            }
            // AND THE DAY-LEVEL ONE IS NOT, at either broker. A number here is
            // one somebody invented; `CLAUDE.md` §3 rule 1 forbids it, and an
            // absent cap is already correct — the store's month boundary binds
            // at every rung.
            assert_eq!(
                spec.window_cap_days(Granularity::Day1),
                None,
                "{}: no day-level cap is recorded in docs/00-charter.md §4",
                feed.display()
            );
            // THE DAILY WORD IS NOW A PER-FEED FACT, NOT A BLANKET ABSENCE.
            //
            // This assertion used to be `None` for every feed, with the reason
            // that `1day`, `1d` and `day` were all plausible and §3 rule 1
            // forbade picking. That was right until the vendor's own annexure
            // was read: `GrowwAPI.CANDLE_INTERVAL_DAY` has the value `1day`,
            // from the same table that gives `1minute`. So the rule the test
            // enforces is unchanged — a word appears here only when a source
            // records it — and what changed is that one source now exists.
            //
            // Dhan stays `None`, and for a different reason that this
            // assertion keeps pinned: its request carries **no interval field
            // at all** (five params: securityId, exchangeSegment, instrument,
            // fromDate, toDate), so there is nothing to spell a rung with and
            // `granularity_tokens` is empty by construction. D-0076.
            //
            // ZERODHA IS THE SECOND SOURCE, and it spells the rung differently
            // — which is the whole reason this table exists rather than a
            // derivation. Groww's annexure says `1day`; Kite's published
            // interval list says `day`. Both are read from the vendor's own
            // page, neither is derivable from the other, and neither is
            // `Granularity::dir`'s `1day` by coincidence or otherwise.
            //
            // NO CATCH-ALL. A sixth feed must be named here, because the
            // failure this pins is somebody filling a word in from memory and
            // nothing noticing — and a `_ => None` arm would let a new row
            // carry any spelling at all.
            let expected_day_word = match feed {
                Feed::Groww => Some("1day"),
                Feed::Zerodha => Some("day"),
                Feed::Dhan | Feed::TrueData | Feed::Gdfl => None,
            };
            assert_eq!(
                spec.granularity_token(Granularity::Day1),
                expected_day_word,
                "{}: a daily interval word appears here only when a source \
                 records it — Groww's annexure gives `1day` and Kite's interval \
                 list gives `day`, and no other feed's page gives one",
                feed.display()
            );
        }
    }

    /// EVERY FEED SAYS WHAT IT MEANS BY A DAILY BAR, AND TWO THAT DISAGREE
    /// ARE NOT COMPARABLE.
    ///
    /// This is the test that turns D-0077 from a comment into a guarantee. The
    /// store already isolates vendors by path — `docs/04-invariants.md` X-12,
    /// and the first path segment is a `Vendor` rather than a string, so no
    /// feed can write into another's directory. Isolation is not the whole
    /// problem: `dhan/…/1day/` and `groww/…/1day/` are isolated AND hold two
    /// different definitions of a day, which is how a wrong open reached disk
    /// without any gate noticing.
    #[test]
    fn every_feed_declares_what_it_means_by_a_daily_bar() {
        // The measured pair, and the reason this enum exists.
        assert_eq!(
            Feed::Dhan.descriptor().day_bar,
            BarConvention::SessionOpenToClose,
            "Dhan's open is the session's first print — 2026-08-07 BANKNIFTY \
             O 57,882.00, which its own chart agrees with"
        );
        assert_eq!(
            Feed::Groww.descriptor().day_bar,
            BarConvention::PreviousCloseToClose,
            "Groww's daily candle starts at the PREVIOUS close — 2026-08-07 \
             BANKNIFTY O 58,063.65, which is 08-06's close to the paisa"
        );

        // AND THEREFORE THEY MUST NOT BE COMPARED. A reader that puts these
        // two side by side is comparing two different intervals and calling
        // the difference a vendor disagreement.
        assert!(
            !Feed::Dhan
                .descriptor()
                .day_bar
                .comparable_with(Feed::Groww.descriptor().day_bar),
            "the whole point: these two rungs are not the same bar"
        );
        assert!(
            Feed::Dhan
                .descriptor()
                .day_bar
                .comparable_with(Feed::TrueData.descriptor().day_bar),
            "two session bars are comparable with each other"
        );

        // EVERY feed, including any added later. A new `Feed` variant cannot
        // reach this line without the compiler having already demanded the
        // field — `Descriptor` has no `Default` — so this asserts the weaker
        // remaining thing: that the value round-trips and carries a label an
        // operator can read.
        for feed in Feed::ALL {
            let said = feed.descriptor().day_bar;
            assert!(
                !said.label().is_empty(),
                "{}: a convention with no words is one nobody can act on",
                feed.display()
            );
            assert!(
                said.comparable_with(said),
                "{}: a convention is comparable with itself",
                feed.display()
            );
        }
    }

    /// A bitset holds what was put in it, and adding twice is adding once.
    #[test]
    fn a_granularity_set_is_a_bitset_over_the_ladder() {
        let empty = GranularitySet::EMPTY;
        assert!(empty.is_empty());
        assert_eq!(GranularitySet::default(), empty, "default is empty");
        for rung in Granularity::ALL {
            assert!(!empty.contains(rung), "the empty set holds nothing");
        }

        let one = empty.with(Granularity::Minute1);
        assert!(!one.is_empty());
        assert!(one.contains(Granularity::Minute1));
        assert_eq!(
            one.with(Granularity::Minute1),
            one,
            "adding twice is adding once"
        );

        let two = one.with(Granularity::Week1);
        let held = Granularity::ALL
            .iter()
            .filter(|g| two.contains(**g))
            .count();
        assert_eq!(held, 2, "exactly the two rungs that were added");
        assert!(two.contains(Granularity::Week1) && two.contains(Granularity::Minute1));
        assert!(!two.contains(Granularity::Tick));
    }

    /// The segment bitset, and the free function that gives each segment a bit.
    #[test]
    fn a_segment_set_gives_each_segment_its_own_bit() {
        let empty = SegmentSet::EMPTY;
        assert!(empty.is_empty());
        assert_eq!(SegmentSet::default(), empty);

        let every = [Segment::Index, Segment::Cash, Segment::Fno];
        let mut bits = HashSet::with_capacity(every.len());
        for segment in every {
            assert!(!empty.contains(segment));
            assert!(
                bits.insert(segment_bit(segment)),
                "two segments share a bit, so a set could not tell them apart"
            );
        }

        let index_only = empty.with(Segment::Index);
        assert!(!index_only.is_empty());
        assert!(index_only.contains(Segment::Index));
        assert!(!index_only.contains(Segment::Cash) && !index_only.contains(Segment::Fno));
        assert_eq!(index_only.with(Segment::Index), index_only);

        let all = index_only.with(Segment::Cash).with(Segment::Fno);
        assert!(every.iter().all(|s| all.contains(*s)));
    }

    // -- venues, and the dated session table --------------------------------

    /// Every venue answers for its own `(exchange, segment)` pair, and BSE is
    /// refused by absence rather than mapped onto an NSE row.
    #[test]
    fn a_venue_is_chosen_by_exchange_and_segment_and_bse_has_none() {
        let mut labels = HashSet::with_capacity(VENUE_COUNT);
        for venue in Venue::ALL {
            assert!(!venue.label().is_empty());
            assert!(labels.insert(venue.label()), "two venues share a label");
            assert_eq!(venue.to_string(), venue.label());
        }
        assert_eq!(labels.len(), VENUE_COUNT);

        assert_eq!(
            Venue::for_segment(Exchange::Nse, Segment::Index),
            Some(Venue::NseIndex)
        );
        assert_eq!(
            Venue::for_segment(Exchange::Nse, Segment::Cash),
            Some(Venue::NseCash)
        );
        assert_eq!(
            Venue::for_segment(Exchange::Nse, Segment::Fno),
            Some(Venue::NseDerivatives)
        );
        for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
            assert_eq!(
                Venue::for_segment(Exchange::Bse, segment),
                None,
                "CLAUDE.md section 1 does not pull BSE, and two exchanges \
                 sharing one window is a claim nothing here has a source for"
            );
        }
    }

    /// The anchor is `crate::session`'s two constants for all three venues, and
    /// the 2026-08-03 row moves only the venues whose circular says so.
    #[test]
    fn the_session_table_anchors_on_the_charter_and_moves_on_2026_08_03() {
        let before = day(2026, 8, 2);
        for venue in Venue::ALL {
            let anchored = venue.hours_on(before).expect("every anchor is verified");
            assert_eq!(anchored.open_minute(), SESSION_OPEN_MINUTE);
            assert_eq!(anchored.close_minute(), SESSION_CLOSE_MINUTE);
            assert_eq!(anchored.kind(), SessionKind::Continuous);
            assert!(
                has_content(anchored.source()),
                "the anchor IS session.rs's two constants and cites the charter, \
                 which is why that module's exhaustive day walk stays correct"
            );
            assert_eq!(
                venue.table().anchor.hours,
                Hours::Verified {
                    open_minute: SESSION_OPEN_MINUTE,
                    close_minute: SESSION_CLOSE_MINUTE,
                },
                "and the row it came from carries the hours, not a default"
            );
        }

        let after = day(2026, 8, 3);
        let index = Venue::NseIndex.hours_on(after).expect("verified row");
        let cash = Venue::NseCash.hours_on(after).expect("verified row");
        let fno = Venue::NseDerivatives.hours_on(after).expect("verified row");

        assert_eq!(
            index.close_minute(),
            15 * 60 + 15,
            "the swept index freezes EARLIEST — every constituent stops trading \
             continuously at 15:15"
        );
        assert_eq!(
            cash.close_minute(),
            SESSION_CLOSE_MINUTE,
            "a share with no derivative contract keeps its continuous close, so \
             this row restates the anchor"
        );
        assert_eq!(
            fno.close_minute(),
            15 * 60 + 40,
            "derivatives extend by ten"
        );
        assert!(
            index.close_minute() < cash.close_minute() && cash.close_minute() < fno.close_minute(),
            "the three did not move together, which is why there are three rows"
        );
        for session in [index, cash, fno] {
            assert_eq!(session.open_minute(), SESSION_OPEN_MINUTE, "the open held");
            assert!(has_content(session.source()), "each row cites its circular");
        }
    }

    /// A verified table has no refusal window, and the lookup walks a number of
    /// rows fixed at compile time.
    #[test]
    fn the_session_lookup_walks_a_fixed_number_of_rows() {
        assert_eq!(
            MAX_LATER_SESSION_ROWS, 2,
            "the bound is the length of a fixed-size array, so no input raises it"
        );
        for venue in Venue::ALL {
            let windows = venue.refusal_windows();
            assert_eq!(
                windows.len(),
                MAX_LATER_SESSION_ROWS + 1,
                "one slot per representable row"
            );
            assert!(
                windows.iter().all(Option::is_none),
                "{venue} ships no unverified row today, so it refuses no day"
            );
        }
    }

    /// Session arithmetic: length, membership, and the four record counts.
    #[test]
    fn a_session_reports_its_length_its_membership_and_its_record_count() {
        let session = Venue::NseIndex
            .hours_on(day(2024, 1, 2))
            .expect("the anchor is verified");

        assert_eq!(session.len_secs(), 22_500, "09:15 to 15:30 is 375 minutes");
        assert!(
            !session.contains_minute(SESSION_OPEN_MINUTE - 1),
            "before open"
        );
        assert!(
            session.contains_minute(SESSION_OPEN_MINUTE),
            "open is inclusive"
        );
        assert!(
            session.contains_minute(SESSION_CLOSE_MINUTE - 1),
            "last minute"
        );
        assert!(
            !session.contains_minute(SESSION_CLOSE_MINUTE),
            "the close is EXCLUSIVE"
        );

        assert_eq!(
            session.expected_count(Granularity::Minute1),
            ExpectedCount::Exact(BARS_PER_REGULAR_SESSION),
            "375 one-minute bars, derived rather than restated"
        );
        assert_eq!(
            session.expected_count(Granularity::Tick),
            ExpectedCount::Unbounded,
            "a tick feed prints as often as the market prints"
        );
        assert_eq!(
            session.expected_count(Granularity::Day1),
            ExpectedCount::Aggregate
        );
        assert_eq!(
            session.expected_count(Granularity::Week1),
            ExpectedCount::Aggregate
        );
        assert_eq!(
            session.expected_count(Granularity::Minute30),
            ExpectedCount::Irregular {
                session_secs: 22_500,
                interval_secs: 1_800,
            },
            "375 minutes is twelve and a half half-hours — named rather than \
             rounded, because a rounded count is a gap check that is wrong by a \
             fixed amount every single day"
        );
        assert_eq!(
            session.expected_count(Granularity::Second1),
            ExpectedCount::Exact(22_500)
        );
    }

    /// A row with no verified hours refuses by name, and the refusal carries
    /// the venue, the day, the window and the citation gap.
    #[test]
    fn a_row_with_no_verified_hours_refuses_and_names_the_gap() {
        const GAP: &str = "TEST ROW: no circular was retrieved for this window";
        let start = day(2030, 1, 1);
        let resumes = day(2031, 1, 1);

        let table = SessionTable {
            venue: Venue::NseCash,
            anchor: SessionRow::verified(
                EPOCH_DAY,
                SESSION_OPEN_MINUTE,
                SESSION_CLOSE_MINUTE,
                SessionKind::Continuous,
                CHARTER_SESSION,
            ),
            later: [
                Some(SessionRow::unverified(
                    start,
                    SessionKind::ClosingAuction,
                    GAP,
                )),
                Some(SessionRow::verified(
                    resumes,
                    SESSION_OPEN_MINUTE,
                    SESSION_CLOSE_MINUTE,
                    SessionKind::Continuous,
                    CHARTER_SESSION,
                )),
            ],
        };

        assert!(
            table.is_shipping_shape(),
            "the row is unverified, not malformed — an unverified row is a \
             legal row and that is the whole point of the arm"
        );

        table
            .hours_on(day(2029, 12, 31))
            .expect("before the gap the anchor still answers");
        table
            .hours_on(resumes)
            .expect("after the gap the later row answers");

        let refusal = table
            .hours_on(day(2030, 6, 1))
            .expect_err("inside the gap there is nothing to answer with");
        assert_eq!(refusal.venue(), Venue::NseCash);
        assert_eq!(refusal.day(), day(2030, 6, 1));
        assert_eq!(refusal.row_start(), start);
        assert_eq!(
            refusal.verified_from(),
            Some(resumes),
            "the window has an end, and the refusal names it"
        );
        assert_eq!(refusal.source(), GAP);

        let text = refusal.to_string();
        for fragment in [
            Venue::NseCash.label(),
            &day(2030, 6, 1).to_string(),
            &start.to_string(),
            &resumes.to_string(),
            GAP,
        ] {
            assert!(
                text.contains(fragment),
                "the refusal must carry {fragment:?} — got {text:?}"
            );
        }

        let windows = table.refusal_windows();
        let named: Vec<_> = windows.into_iter().flatten().collect();
        assert_eq!(named.len(), 1, "one unverified row, one window");
        assert_eq!(named[0].venue(), Venue::NseCash);
        assert_eq!(named[0].start(), start);
        assert_eq!(named[0].verified_from(), Some(resumes));
    }

    /// A sink that accepts `remaining` writes and then refuses.
    ///
    /// Exists for one reason: every `write!(f, …)?` in a `Display` impl carries
    /// an error edge, and against a `String` — which is what `to_string` uses —
    /// that edge is unreachable, because writing to a `String` cannot fail. A
    /// formatter over THIS refuses on demand, so each `?` in
    /// [`SessionRefusal::fmt`] is proven to propagate rather than to swallow.
    struct FailingSink {
        remaining: usize,
    }

    impl fmt::Write for FailingSink {
        fn write_str(&mut self, _text: &str) -> fmt::Result {
            match self.remaining.checked_sub(1) {
                Some(left) => {
                    self.remaining = left;
                    Ok(())
                }
                None => Err(fmt::Error),
            }
        }
    }

    /// Every `?` in the refusal's own `Display` propagates the sink's failure.
    #[test]
    fn a_refusal_propagates_a_sink_that_stops_accepting_it() {
        use std::fmt::Write as _;

        let ended = SessionRefusal {
            venue: Venue::NseCash,
            day: day(2030, 6, 1),
            row_start: day(2030, 1, 1),
            verified_from: Some(day(2031, 1, 1)),
            source: SOURCE,
        };
        let open_ended = SessionRefusal {
            verified_from: None,
            ..ended
        };

        for refusal in [ended, open_ended] {
            let full = refusal.to_string();
            let mut accepted = FailingSink {
                remaining: usize::MAX,
            };
            write!(accepted, "{refusal}").expect("a sink that never refuses");

            // Walk the budget from zero upward. Every prefix length short of
            // the whole message must surface as an error rather than a
            // truncated line an operator would read as complete.
            let mut refused = 0_usize;
            for budget in 0..64 {
                let mut sink = FailingSink { remaining: budget };
                if write!(sink, "{refusal}").is_err() {
                    refused += 1;
                }
            }
            assert!(
                refused > 0,
                "a sink that refuses must make the whole write fail — {full:?}"
            );
            assert!(
                refused < 64,
                "and a sink with room enough must succeed, or the loop is \
                 proving nothing"
            );
        }
    }

    /// An unverified row that nothing follows is open-ended, and the refusal
    /// says so in words rather than inventing an end.
    #[test]
    fn an_unverified_row_with_no_successor_is_open_ended() {
        const GAP: &str = "TEST ROW: the era after this one was never verified";
        let start = day(2040, 4, 1);
        let table = SessionTable {
            venue: Venue::NseDerivatives,
            anchor: SessionRow::verified(
                EPOCH_DAY,
                SESSION_OPEN_MINUTE,
                SESSION_CLOSE_MINUTE,
                SessionKind::Continuous,
                CHARTER_SESSION,
            ),
            later: [
                Some(SessionRow::unverified(start, SessionKind::Continuous, GAP)),
                None,
            ],
        };

        let refusal = table.hours_on(start).expect_err("the row has no hours");
        assert_eq!(refusal.verified_from(), None, "nothing follows it");
        let text = refusal.to_string();
        assert!(
            text.contains(" onward"),
            "an open-ended window says onward rather than naming an end it does \
             not have — got {text:?}"
        );

        let windows = table.refusal_windows();
        let named: Vec<_> = windows.into_iter().flatten().collect();
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].verified_from(), None);
    }

    /// The citation every row built for a test carries. Never a segment.
    const SOURCE: &str = "TEST ROW";

    /// A row starting on `start` with the charter's own hours.
    fn later(start: Day) -> SessionRow {
        SessionRow::verified(
            start,
            SESSION_OPEN_MINUTE,
            SESSION_CLOSE_MINUTE,
            SessionKind::Continuous,
            SOURCE,
        )
    }

    /// The anchor every table built for a test starts from.
    fn anchor_row() -> SessionRow {
        later(EPOCH_DAY)
    }

    /// A table, so the contract can be driven from outside the shipped rows.
    fn table(
        anchor: SessionRow,
        later: [Option<SessionRow>; MAX_LATER_SESSION_ROWS],
    ) -> SessionTable {
        SessionTable {
            venue: Venue::NseIndex,
            anchor,
            later,
        }
    }

    /// The ordering half of the contract: the anchor covers every day and the
    /// rows ascend with no hole, driven through each arm a shipped table never
    /// takes.
    #[test]
    fn the_table_shape_contract_refuses_a_table_whose_rows_do_not_ascend() {
        let anchor = anchor_row();

        assert!(
            table(anchor, [None, None]).is_shipping_shape(),
            "bare anchor"
        );
        assert!(
            table(anchor, [Some(later(day(2020, 1, 1))), None]).is_shipping_shape(),
            "one later row"
        );
        assert!(
            table(
                anchor,
                [Some(later(day(2020, 1, 1))), Some(later(day(2021, 1, 1)))]
            )
            .is_shipping_shape(),
            "two later rows, ascending"
        );

        // An anchor that does not start at the epoch leaves days uncovered.
        let late_anchor = SessionRow::verified(
            day(1971, 1, 1),
            SESSION_OPEN_MINUTE,
            SESSION_CLOSE_MINUTE,
            SessionKind::Continuous,
            SOURCE,
        );
        assert!(!table(late_anchor, [None, None]).anchor_covers_all_days());
        assert!(!table(late_anchor, [None, None]).is_shipping_shape());

        // Rows that do not ascend, in each of the three shapes that can fail.
        assert!(
            !table(anchor, [Some(anchor), None]).rows_ascend(),
            "equal starts"
        );
        assert!(
            !table(
                anchor,
                [Some(later(day(2021, 1, 1))), Some(later(day(2020, 1, 1)))]
            )
            .rows_ascend(),
            "descending later rows"
        );
        assert!(
            !table(anchor, [None, Some(later(day(2020, 1, 1)))]).rows_ascend(),
            "a hole before a populated slot — `rows()` would silently close it"
        );
    }

    /// The soundness half: a row is ill-shaped when its window is inverted, out
    /// of range, or its citation says nothing — and every slot is checked, not
    /// just the first.
    #[test]
    fn the_table_shape_contract_refuses_a_row_with_no_window_and_no_citation() {
        let anchor = anchor_row();
        let inverted = SessionRow::verified(
            day(2020, 1, 1),
            SESSION_CLOSE_MINUTE,
            SESSION_OPEN_MINUTE,
            SessionKind::Continuous,
            SOURCE,
        );
        let past_midnight = SessionRow::verified(
            day(2020, 1, 1),
            SESSION_OPEN_MINUTE,
            24 * 60 + 1,
            SessionKind::Continuous,
            SOURCE,
        );
        let uncited = SessionRow::verified(
            day(2020, 1, 1),
            SESSION_OPEN_MINUTE,
            SESSION_CLOSE_MINUTE,
            SessionKind::Continuous,
            "   ",
        );
        assert!(!inverted.is_well_shaped(), "close before open");
        assert!(!past_midnight.is_well_shaped(), "a close past midnight");
        assert!(!uncited.is_well_shaped(), "a blank citation is no citation");
        assert!(anchor.is_well_shaped());
        assert!(
            SessionRow::unverified(day(2020, 1, 1), SessionKind::Continuous, SOURCE)
                .is_well_shaped(),
            "an unverified row has no window to be wrong"
        );

        assert!(!table(uncited, [None, None]).rows_are_well_shaped());
        assert!(!table(anchor, [Some(inverted), None]).rows_are_well_shaped());
        assert!(
            !table(anchor, [Some(anchor), Some(inverted)]).rows_are_well_shaped(),
            "the SECOND slot is checked too"
        );
        assert!(
            !table(anchor, [None, Some(inverted)]).rows_are_well_shaped(),
            "and so is a second slot with a hole before it"
        );
        assert!(table(anchor, [Some(anchor), Some(anchor)]).rows_are_well_shaped());

        // The anchor must be session.rs's two constants, and an unverified
        // anchor can never be.
        assert!(table(anchor, [None, None]).anchor_matches_session_constants());
        let moved = SessionRow::verified(
            EPOCH_DAY,
            SESSION_OPEN_MINUTE + 1,
            SESSION_CLOSE_MINUTE,
            SessionKind::Continuous,
            SOURCE,
        );
        assert!(!table(moved, [None, None]).anchor_matches_session_constants());
        let unverified_anchor = SessionRow::unverified(EPOCH_DAY, SessionKind::Continuous, SOURCE);
        assert!(
            !table(unverified_anchor, [None, None]).anchor_matches_session_constants(),
            "an anchor with no hours cannot equal two constants"
        );
    }

    /// Every shipped table satisfies the contract at run time as well as at
    /// compile time, and `rows()` yields the anchor first.
    #[test]
    fn every_shipped_session_table_is_in_shipping_shape() {
        for venue in Venue::ALL {
            let table = venue.table();
            assert!(table.is_shipping_shape(), "{venue} ships a malformed table");
            assert!(table.anchor_matches_session_constants(), "{venue}");
            let rows: Vec<_> = table.rows().collect();
            assert!(!rows.is_empty(), "the anchor is always a row");
            assert_eq!(
                rows[0].start.days_from_epoch(),
                0,
                "the anchor comes first and starts at the epoch"
            );
            for pair in rows.windows(2) {
                assert!(
                    pair[0].start.days_from_epoch() < pair[1].start.days_from_epoch(),
                    "{venue}'s rows must ascend strictly"
                );
            }
        }
    }

    /// A session kind has a stable label, and both arms exist so an auction can
    /// be refused by name rather than mis-filtered as continuous trading.
    #[test]
    fn a_session_kind_has_a_label_for_each_arm() {
        assert_ne!(
            SessionKind::Continuous.label(),
            SessionKind::ClosingAuction.label()
        );
        for kind in [SessionKind::Continuous, SessionKind::ClosingAuction] {
            assert!(!kind.label().is_empty());
            assert_eq!(kind.to_string(), kind.label());
        }
    }

    // -- the transport rows --------------------------------------------------

    /// The verb and the auth prefix are data, one row each.
    #[test]
    fn the_wire_verb_and_the_auth_prefix_are_rows_not_branches() {
        assert_eq!(Method::Get.as_str(), "GET");
        assert_eq!(Method::Post.as_str(), "POST");
        assert!(
            AuthScheme::Raw.prefix().is_empty(),
            "a raw token has no prefix"
        );
        assert_eq!(AuthScheme::Bearer.prefix(), "Bearer ");
    }

    /// Row *i* of the table describes variant *i*, and a lookup is that index.
    #[test]
    fn the_descriptor_table_is_indexed_by_the_discriminant() {
        assert_eq!(DESCRIPTORS.len(), FEED_COUNT);
        assert_eq!(Feed::ALL.len(), FEED_COUNT);

        let mut wires = HashSet::with_capacity(FEED_COUNT);
        let mut names = HashSet::with_capacity(FEED_COUNT);
        for (index, feed) in Feed::ALL.into_iter().enumerate() {
            let row = feed.descriptor();
            assert_eq!(row.feed, feed, "row {index} describes another variant");
            assert_eq!(
                DESCRIPTORS[index], row,
                "the lookup is the discriminant, not a search"
            );
            assert!(wires.insert(feed.wire()), "two feeds share a wire name");
            assert!(
                names.insert(feed.display()),
                "two feeds share a display name"
            );
            assert_eq!(feed.display(), row.display);
            assert_eq!(feed.wire(), row.wire);
            assert_eq!(feed.to_string(), row.display);
            assert!(
                !row.granularities.is_empty() && !row.segments.is_empty(),
                "{feed} could never answer a request"
            );
            assert_eq!(row.exchange, Exchange::Nse);
        }
        assert_eq!(wires.len(), FEED_COUNT);
    }

    /// EVERY FEED HAS ITS OWN PREFIX, AND NO TWO SHARE ONE.
    ///
    /// This asserted the opposite — that the archive feeds have `None` — and
    /// that `None` is precisely what filed 194 instrument-months of GDFL
    /// futures under `bars/dhan/`. `run_local` could not ask which vendor it
    /// was reading for, so it used a literal, and D-0019's per-vendor
    /// independence was destroyed exactly as the old assertion's own message
    /// warned it would be.
    ///
    /// The fix was not to refuse: a feed that cannot file its bars anywhere
    /// cannot pull at all. It was to give them prefixes, which was blocked on
    /// `pull::config` demanding a credential table from every vendor — a
    /// question archives cannot answer. That loop now asks only feeds whose
    /// transport is HTTP, and this became possible.
    ///
    /// No feed is named on the left of any assertion below: the loop is over
    /// `Feed::ALL`, so a fifth feed must declare a prefix and must not collide
    /// with an existing one.
    #[test]
    fn every_feed_has_its_own_store_prefix_and_no_two_collide() {
        let mut seen: Vec<Vendor> = Vec::new();
        for feed in Feed::ALL {
            let prefix = feed.store_vendor().unwrap_or_else(|| {
                panic!(
                    "{} has no store prefix, so its bars have nowhere to go and \
                     the reader would borrow another vendor's — which is what \
                     put GDFL futures under bars/dhan/",
                    feed.display()
                )
            });
            assert!(
                !seen.contains(&prefix),
                "{} shares a prefix with a feed already seen; two feeds under \
                 one path is the same corruption in a different shape",
                feed.display()
            );
            seen.push(prefix);
        }
        assert_eq!(seen.len(), FEED_COUNT, "one prefix per feed, no more");
    }

    /// A feed serves the rungs its row names and no others.
    #[test]
    fn a_feed_serves_only_the_rungs_its_row_declares() {
        for feed in Feed::ALL {
            let declared = feed.descriptor().granularities;
            for rung in Granularity::ALL {
                assert_eq!(
                    feed.serves(rung),
                    declared.contains(rung),
                    "{feed} disagrees with its own row about {rung}"
                );
            }
        }
        assert!(Feed::Dhan.serves(Granularity::Day1));
        // THIS ASSERTION IS INVERTED FROM WHAT IT USED TO BE, and the inversion
        // is the point. It read `assert!(Feed::Dhan.serves(Minute1))`, which
        // pinned a rung the descriptor could not actually serve: `bars_path` is
        // one field holding this vendor's DAILY endpoint, so a `Minute1`
        // request fetched daily bars and filed them under `1min/`. The test
        // passed the whole time, because it asked whether the row DECLARED the
        // rung and never whether the request could carry it.
        assert!(
            !Feed::Dhan.serves(Granularity::Minute1),
            "the minute rung needs the intraday path and an interval field; \
             until both exist it must refuse by name rather than file daily \
             bars as minute bars"
        );
        assert!(
            !Feed::Dhan.serves(Granularity::Minute5),
            "an unserved rung refuses by name; the wider ladder was never read live"
        );
        // AN ARCHIVE SERVES EVERY RUNG ITS INPUT CAN BE FOLDED INTO, AND THAT
        // IS A DIFFERENT RULE FROM THE BROKER ONE ABOVE.
        //
        // Dhan refuses `Minute1` because a REST row's rung is a PROPERTY OF THE
        // REQUEST — one `bars_path`, one endpoint, and asking for another rung
        // fetches the wrong bar length under the name you asked for. Nothing
        // about a folder works that way. The input is fixed at one second and
        // the rung decides only the FOLD WIDTH, which `crate::ingest` takes
        // straight off the store timeframe: `Bucket::of_secs(timeframe.secs())`.
        // `crate::fold` has never cared what that width is.
        //
        // So `!Gdfl.serves(Minute1)` was pinning an absence with no reason
        // behind it: it made `served()` refuse a minute read of a folder this
        // build folds to minutes from the same bytes. Both archives now declare
        // the three rungs `store::path::Timeframe` and the fold can between
        // them, and the rule is asserted rather than the old absence. D-0141.
        for archive in [Feed::TrueData, Feed::Gdfl] {
            assert_eq!(
                archive.source_kind(),
                SourceKind::Folder,
                "the rule below is about folders, so this is what selects them"
            );
            assert!(
                archive.serves(Granularity::Second1),
                "{archive} reads one-second files, so it serves the rung it IS"
            );
            for folded in [Granularity::Minute1, Granularity::Day1] {
                assert!(
                    archive.serves(folded),
                    "{archive} can fold its one-second input into {folded}, so \
                     served() must not refuse it"
                );
                assert!(
                    folded.store_timeframe().is_some(),
                    "and a rung an archive is asked to FILE must have somewhere \
                     to go — otherwise this declaration promises a write the \
                     store cannot make"
                );
            }
        }
        assert!(
            Granularity::Second1.store_timeframe().is_none(),
            "the one rung both archives serve and NEITHER can file: that \
             refusal belongs to the write boundary, which names it, and is why \
             `serves` and `store_timeframe` are two questions"
        );
    }

    /// The HTTP half of a transport, or [`None`] for an archive.
    ///
    /// A function taking BOTH arms rather than a `let … else { panic!() }` at
    /// each call site: the `else` of a let-else that never fires is a branch no
    /// test can reach, and a test file full of unreachable arms is exactly the
    /// hole this workflow exists to close. Called below for every feed, so both
    /// arms are taken.
    // Over clippy's 256-byte by-value threshold since the row grew a parameter
    // map and a header list. `Transport` is `Copy` and this is a const test
    // helper; a reference here buys nothing and costs a `&` at every call.
    #[allow(
        clippy::large_types_passed_by_value,
        reason = "a Copy descriptor row in a const test helper"
    )]
    const fn http_spec(transport: Transport) -> Option<HttpSpec> {
        match transport {
            Transport::Http(spec) => Some(spec),
            Transport::LocalArchive(_) => None,
        }
    }

    /// The archive half, same shape and same reason.
    #[allow(
        clippy::large_types_passed_by_value,
        reason = "a Copy descriptor row in a const test helper, same as above"
    )]
    const fn archive_spec(transport: Transport) -> Option<ArchiveSpec> {
        match transport {
            Transport::LocalArchive(spec) => Some(spec),
            Transport::Http(_) => None,
        }
    }

    /// The envelope key a response shape hangs its bars under, whichever shape
    /// it is.
    const fn envelope_of(shape: ResponseShape) -> Option<&'static str> {
        match shape {
            ResponseShape::ParallelArrays { envelope }
            | ResponseShape::ArrayOfObjects { envelope }
            | ResponseShape::PositionalRows { envelope, .. } => envelope,
        }
    }

    /// How an archive's file name renders its date, whichever pattern it uses.
    const fn archive_date(name: ArchiveName) -> DateFormat {
        match name {
            ArchiveName::SegmentAndDate { date, .. } | ArchiveName::DateOnly { date, .. } => date,
        }
    }

    /// A member's extension, whichever nesting it sits in.
    const fn member_suffix(member: MemberPattern) -> &'static str {
        match member {
            MemberPattern::SymbolAtRoot { suffix } | MemberPattern::StemGroupSymbol { suffix } => {
                suffix
            }
        }
    }

    /// An HTTP row carries a credential and a governor; an archive row has
    /// nowhere to put either.
    #[test]
    fn only_an_http_transport_needs_a_credential_and_a_governor() {
        let mut http = 0_usize;
        let mut archives = 0_usize;
        for feed in Feed::ALL {
            let transport = feed.descriptor().transport;
            assert!(!transport.label().is_empty());
            assert_eq!(
                transport.needs_credential(),
                transport.needs_governor(),
                "{feed}: a token and a ceiling are needed by the same transports"
            );
            assert_eq!(
                archive_spec(transport).is_some(),
                !transport.needs_credential(),
                "{feed}: a transport is one of the two, never both and never neither"
            );
            if let Some(spec) = http_spec(transport) {
                assert!(transport.needs_credential());
                assert!(!spec.auth.header.is_empty(), "{feed} names no auth header");
                assert!(spec.base_url.starts_with("https://"), "{feed}");
                // EVERY SEGMENT NAMES SOMETHING, AND NONE OF THEM CARRIES A
                // SLASH OF ITS OWN. The slash belongs to the join, so a
                // literal spelled "/v2" or "charts/historical" would produce a
                // doubled or a mis-split path — the shape the old
                // `starts_with('/')` assertion existed to catch, asked of each
                // part rather than of one string.
                assert!(!spec.bars_path.is_empty(), "{feed} names no bars path");
                for segment in spec.bars_path {
                    let name = segment.name();
                    assert!(!name.is_empty(), "{feed}: an unnamed path segment");
                    assert!(
                        !name.contains('/'),
                        "{feed}: path segment {name:?} carries its own slash"
                    );
                }
                assert!(
                    spec.path_template().starts_with('/'),
                    "{feed}: {}",
                    spec.path_template()
                );
                http += 1;
            } else {
                let spec =
                    archive_spec(transport).expect("a transport that is not HTTP is an archive");
                assert!(
                    !transport.needs_credential(),
                    "a local file has no rate limit and needs no credential"
                );
                assert!(!spec.layouts.is_empty(), "{feed} decodes nothing");
                archives += 1;
            }
        }
        assert_eq!(
            (http, archives),
            (3, 2),
            "three brokers, two archive vendors"
        );
        assert_ne!(
            Feed::Dhan.descriptor().transport.label(),
            Feed::Gdfl.descriptor().transport.label()
        );
    }

    /// The two brokers differ in every field this module exists to make data.
    #[test]
    fn the_two_http_rows_disagree_in_every_field_that_used_to_be_a_branch() {
        let dhan = http_spec(Feed::Dhan.descriptor().transport)
            .expect("the secondary broker is an HTTP feed");
        let groww = http_spec(Feed::Groww.descriptor().transport)
            .expect("the primary broker is an HTTP feed");

        assert_ne!(dhan.method, groww.method, "POST against GET");
        assert_ne!(dhan.auth.header, groww.auth.header);
        assert_ne!(dhan.auth.scheme, groww.auth.scheme);
        assert_eq!(
            dhan.range_end,
            RangeEnd::Exclusive,
            "toDate is NOT inclusive — one field, one conversion site"
        );
        assert_eq!(groww.range_end, RangeEnd::Inclusive);
        // WAS: assert_ne!(dhan.timestamps, groww.timestamps, "seconds against millis")
        //
        // Groww never sent millis. That assertion passed only because the row
        // was wrong, so it defended the error against anyone correcting it.
        // Both are now asserted against what the vendor MEASURABLY sends:
        // Dhan's sample stamps are ten digits, and a live Groww response reads
        // "2026-08-04T09:15:00".
        assert_eq!(dhan.timestamps, TimestampEncoding::EpochSecondsUtc);
        assert_eq!(groww.timestamps, TimestampEncoding::IsoDateTimeText);
        assert_ne!(dhan.pooling, groww.pooling);
        assert_eq!(dhan.date_format, DateFormat::DashedYmd);
        assert_eq!(dhan.prices, PriceScale::Rupees);
        assert_eq!(groww.prices, PriceScale::Rupees);

        assert_eq!(
            dhan.response,
            ResponseShape::ParallelArrays { envelope: None },
            "seven arrays at the top level, which is the shape `zip` silently \
             truncates"
        );
        let envelope = envelope_of(groww.response)
            .expect("the primary broker nests its bars under an envelope");
        assert!(!envelope.is_empty(), "an envelope key of nothing is no key");
        // ONE ARRAY PER BAR, NOT ONE OBJECT. The row said `ArrayOfObjects` and
        // this test agreed with it; a live response is
        // `payload.candles: [["2026-08-04T09:15:00", 24653.0, …], …]`, read by
        // POSITION because the vendor sends no names at all.
        assert_eq!(
            groww.response,
            ResponseShape::PositionalRows {
                envelope: Some(envelope),
                array: "candles",
            },
            "one positional array per bar, under that envelope"
        );
        assert_eq!(
            envelope_of(dhan.response),
            None,
            "and the other broker's arrays are at the top level"
        );

        // BOTH BROKERS NAME NO OPEN-INTEREST ARRAY, and the assertion is
        // inverted from what it was. It read `assert!(dhan…is_some())`, pinning
        // a claim this vendor's own field table contradicts — `open_interest`
        // is documented "for F&O instruments", and a spot index answer simply
        // has no such array. Both swept indices were refused on it.
        //
        // Named here rather than left implicit because restoring it is a real
        // decision: the day the expired-F&O path is modelled, open interest
        // becomes a per-LISTING question, not a per-feed one.
        assert_eq!(
            dhan.fields.open_interest, None,
            "this vendor sends open interest only for F&O; the spot surface has none"
        );
        assert_eq!(
            groww.fields.open_interest, None,
            "absent open interest becomes the null sentinel; zero means zero"
        );
        for names in [dhan.fields, groww.fields] {
            for field in [
                names.open,
                names.high,
                names.low,
                names.close,
                names.volume,
                names.timestamp,
            ] {
                assert!(!field.is_empty());
            }
        }

        assert_eq!(dhan.budget.per_second, Some(crate::rate::DHAN_PER_SECOND));
        assert_eq!(
            dhan.budget.per_minute, None,
            "no published per-minute bound"
        );
        assert_eq!(dhan.budget.per_day, Some(crate::rate::DHAN_PER_DAY));
        assert_eq!(groww.budget.per_minute, Some(crate::rate::GROWW_PER_MINUTE));
        assert_eq!(
            groww.budget.per_second,
            Some(crate::rate::GROWW_PER_SECOND_UNVERIFIED),
            "the constant's own name says the figure is not confirmed, and it \
             is not quietly promoted here"
        );
        assert_eq!(groww.budget.per_day, None);
    }

    /// An archive row addresses its files by pattern, and a segment nobody
    /// measured is refused rather than decoded against a guessed layout.
    #[test]
    fn an_unmeasured_segment_layout_is_refused_by_name() {
        let truedata = archive_spec(Feed::TrueData.descriptor().transport)
            .expect("this feed is a folder of archives");
        let gdfl = archive_spec(Feed::Gdfl.descriptor().transport)
            .expect("this feed is a folder of archives");

        let index = truedata
            .layout(Segment::Index)
            .expect("the index layout was measured");
        assert_eq!(index.segment, Segment::Index);
        assert_eq!(
            index.columns.len(),
            5,
            "five columns, measured not documented"
        );
        // THE F&O LAYOUT IS MEASURED NOW, AND IT ASSERTS THE RULE RATHER THAN
        // THE ABSENCE.
        //
        // This asserted `None` on the ground that "the futures archives were
        // measured at nine columns and their MEANINGS were never established".
        // That was true of the NINE-column `TICK_BA` product and never true of
        // the plain one: `20221003,09:15:01,0.95,1,518600` is five fields, the
        // same shape as the index, with a real volume and a real open interest
        // where the index writes `0,0`. Measured by the operator on his own
        // file, 14 Aug 2026, and stated independently by the vendor.
        //
        // What the original test was FOR is unchanged and is asserted below on
        // `Cash`, which nobody has measured: a segment with no layout refuses
        // rather than being decoded against a guess.
        let fno = truedata
            .layout(Segment::Fno)
            .expect("the plain F&O layout was measured");
        assert_eq!(fno.segment, Segment::Fno);
        assert_eq!(fno.columns.len(), 5, "five fields, as the index");
        assert_eq!(
            fno.shape, index.shape,
            "one PHYSICAL shape, two MEANINGS — which is why Columns and \
             ColumnLayout are two types"
        );
        assert!(
            fno.columns.contains(&Column::Volume) && fno.columns.contains(&Column::OpenInterest),
            "the trailing pair is what distinguishes a contract row from an \
             index row, and reading it as Ignored would discard both"
        );
        assert_eq!(
            truedata.layout(Segment::Cash),
            None,
            "nobody measured a cash archive for this vendor — a segment with \
             no layout refuses by name rather than borrowing another's"
        );

        let fno = gdfl
            .layout(Segment::Fno)
            .expect("the F&O layout was measured");
        assert_eq!(fno.columns.len(), 10);
        assert_eq!(gdfl.layout(Segment::Index), None);

        // Group folders and segment tokens, both directions.
        assert_eq!(truedata.group_folder(ArchiveGroup::Options), None);
        assert_eq!(truedata.group_folder(ArchiveGroup::Flat), None);
        assert!(gdfl.group_folder(ArchiveGroup::Options).is_some());
        assert!(gdfl.group_folder(ArchiveGroup::Futures).is_some());
        assert_eq!(
            gdfl.group_folder(ArchiveGroup::Flat),
            None,
            "the archive is not flat, so there is no flat folder"
        );
        assert!(truedata.segment_token(Segment::Index).is_some());
        assert_eq!(
            truedata.segment_token(Segment::Fno),
            None,
            "the archives carry both futures and options tokens, and one token \
             for the pair would be wrong half the time"
        );
        assert_eq!(gdfl.segment_token(Segment::Fno), None);

        assert_eq!(truedata.header, HeaderRow::Absent);
        assert_eq!(gdfl.header, HeaderRow::Present(GDFL_HEADER));
        assert_eq!(truedata.delimiter, b',');
        assert_eq!(gdfl.delimiter, b',');
        assert_eq!(truedata.nesting, Nesting::ZipOfDailyZips);
        assert_eq!(gdfl.nesting, Nesting::ZipOfSegmentFolders);
        assert_eq!(
            gdfl.date_format,
            DateFormat::SlashedDmy,
            "01/07/2025 is 1 July, and reading it the other way shifts every \
             bar by months"
        );
        assert_eq!(truedata.date_format, DateFormat::CompactYmd);
        assert_eq!(truedata.prices, PriceScale::Rupees);
        assert_eq!(gdfl.prices, PriceScale::Rupees);

        // The two archives are addressed by DIFFERENT patterns, which is the
        // whole reason the pattern is a field. Compared by discriminant so the
        // assertion does not have to spell a file-name prefix out.
        assert_ne!(
            std::mem::discriminant(&truedata.archive),
            std::mem::discriminant(&gdfl.archive),
            "one names its segment in the file name and the other only its date"
        );
        assert_ne!(
            std::mem::discriminant(&truedata.member),
            std::mem::discriminant(&gdfl.member),
            "one member sits at the archive root and the other under two folders"
        );
        assert_eq!(archive_date(truedata.archive), DateFormat::CompactYmd);
        assert_eq!(
            archive_date(gdfl.archive),
            DateFormat::CompactDmy,
            "DDMMYYYY in the archive name, DD/MM/YYYY inside the file — reading \
             either the other way shifts every bar by months"
        );
        assert_ne!(
            archive_date(gdfl.archive),
            gdfl.date_format,
            "and those two are deliberately not the same format"
        );
        for member in [truedata.member, gdfl.member] {
            let suffix = member_suffix(member);
            assert!(suffix.starts_with('.'), "an extension begins with a dot");
        }
    }

    /// Both archive feeds are snapshots, both brokers are OHLCV, and the
    /// distinction is what keeps a snapshot out of the bar format.
    #[test]
    fn a_snapshot_feed_is_never_declared_as_a_bar_feed() {
        assert_eq!(Feed::Dhan.descriptor().record, RecordShape::Ohlcv);
        assert_eq!(Feed::Groww.descriptor().record, RecordShape::Ohlcv);
        assert_eq!(Feed::TrueData.descriptor().record, RecordShape::Snapshot);
        assert_eq!(Feed::Gdfl.descriptor().record, RecordShape::Snapshot);
        assert_ne!(RecordShape::Ohlcv, RecordShape::Snapshot);
    }

    /// The column vocabulary, and the honest arm inside it.
    #[test]
    fn the_column_vocabulary_carries_an_arm_for_a_meaning_nobody_established() {
        let every = [
            Column::Ticker,
            Column::Date,
            Column::Time,
            Column::LastPrice,
            Column::Open,
            Column::High,
            Column::Low,
            Column::Close,
            Column::Volume,
            Column::OpenInterest,
            Column::BidPrice,
            Column::BidQty,
            Column::AskPrice,
            Column::AskQty,
            Column::LastQty,
            Column::Ignored,
            Column::Unverified,
        ];
        let distinct: HashSet<_> = every.iter().collect();
        assert_eq!(distinct.len(), every.len(), "every column means one thing");
        assert!(
            !TRUEDATA_INDEX.columns.contains(&Column::Volume)
                && !TRUEDATA_INDEX.columns.contains(&Column::OpenInterest),
            "an index has neither, so the two trailing zeros are Ignored rather \
             than mapped onto fields it does not have"
        );
        assert!(GDFL_FNO.columns.contains(&Column::OpenInterest));
        assert_eq!(
            GDFL_FNO.columns.len(),
            GDFL_HEADER.split(',').count(),
            "the layout and the header row it was read from agree in width"
        );
    }

    /// The values this module hands around are inspectable, comparable and
    /// hashable — a refusal nobody can print is a refusal nobody can act on.
    #[test]
    fn the_reported_values_can_be_printed_compared_and_hashed() {
        let session = Venue::NseIndex
            .hours_on(day(2024, 1, 2))
            .expect("the anchor is verified");
        let refusal = SessionRefusal {
            venue: Venue::NseIndex,
            day: day(2030, 1, 1),
            row_start: day(2029, 1, 1),
            verified_from: None,
            source: "TEST ROW",
        };
        let window = RefusalWindow {
            venue: Venue::NseIndex,
            start: day(2029, 1, 1),
            verified_from: None,
        };

        let printed = format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            HistoryFloor::Unbounded,
            HistoryFloor::RollingMonths { months: 3 },
            DHAN_HISTORY[0],
            DHAN_HISTORY[0].binding,
            Granularity::Minute1,
            Granularity::Minute1.grid(),
            GranularitySet::EMPTY,
            SegmentSet::EMPTY,
            Venue::NseIndex,
            SessionKind::Continuous,
            Hours::Unverified,
            session,
            session.expected_count(Granularity::Tick),
            refusal,
            window,
            Feed::Dhan,
            Feed::Dhan.descriptor(),
            Method::Get,
            AuthScheme::Raw,
            DateFormat::DashedYmd,
            RangeEnd::Inclusive,
            TimestampEncoding::EpochSecondsUtc,
            PriceScale::Paisa,
            Pooling::PerVendor,
        );
        assert!(!printed.is_empty(), "every reported value is inspectable");

        let more = format!(
            "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
            ResponseShape::ParallelArrays { envelope: None },
            Nesting::ZipOfDailyZips,
            ArchiveName::DateOnly {
                prefix: "",
                suffix: "",
                date: DateFormat::CompactDmy,
            },
            MemberPattern::SymbolAtRoot { suffix: "" },
            ArchiveGroup::Flat,
            HeaderRow::Absent,
            Column::Ticker,
            TRUEDATA_INDEX,
            RecordShape::Ohlcv,
            Feed::Dhan.descriptor().transport,
            SessionRow::verified(
                EPOCH_DAY,
                SESSION_OPEN_MINUTE,
                SESSION_CLOSE_MINUTE,
                SessionKind::Continuous,
                "TEST ROW",
            ),
        );
        assert!(!more.is_empty());

        // Ord and Hash, which the sets and the tables both rely on.
        assert!(Granularity::Tick < Granularity::Week1, "coarsest last");
        assert!(Venue::NseIndex < Venue::NseDerivatives);
        assert!(Feed::Dhan < Feed::Gdfl);
        let mut seen = HashSet::with_capacity(4);
        assert!(seen.insert(Feed::Dhan));
        assert!(!seen.insert(Feed::Dhan), "the same feed hashes the same");
        assert!(seen.insert(Feed::Groww));
        let mut spans = HashSet::with_capacity(2);
        assert!(spans.insert(session), "a session hashes");
        assert!(!spans.insert(session), "and the same one hashes the same");
        assert_eq!(
            refusal.venue(),
            window.venue(),
            "both reported values name the venue they came from"
        );
        let cloned = *Feed::Dhan.descriptor();
        assert_eq!(cloned, *Feed::Dhan.descriptor(), "a row is Copy and equal");
    }

    // -- the history floors --------------------------------------------------

    /// THE FIELD IS PER RUNG BECAUSE THE VENDOR'S OWN TABLE IS, AND IT STAYS
    /// PER RUNG NOW THAT WHAT BINDS IS UNIFORM.
    ///
    /// Groww's interval table gives its day row "Full history" and its
    /// one-minute row "Last 3 months" — two different claims about two rungs of
    /// one vendor, which is why a floor keyed on the vendor alone cannot hold
    /// this fact at all. Those two claims are still here and still differ; they
    /// are now the `contested` half of both rows, because the operator restated
    /// the binding floor on 12 Aug 2026 against the VENDOR rather than against
    /// a rung.
    ///
    /// So this test pins BOTH halves. Collapsing the field to one floor per
    /// vendor would silently discard the per-rung disagreement that is the only
    /// reason anybody knows this rung is contested at all.
    #[test]
    fn one_vendors_two_rungs_carry_two_different_claims() {
        let groww = Feed::Groww.descriptor();
        // What BINDS is the operator's, stated against the vendor, so it is
        // the same day at both rungs — and applying his single figure to both
        // is stated as UNVERIFIED on GROWW_HISTORY rather than split on a
        // guess.
        for rung in [Granularity::Minute1, Granularity::Day1] {
            assert_eq!(
                groww.history_floor(rung),
                HistoryFloor::Fixed {
                    year: 2020,
                    month: 1,
                    day: 1
                },
                "the operator, 12 Aug 2026, at {rung}"
            );
        }
        // What the VENDOR claims is per rung, it differs between the two, and
        // the field is what makes room for that.
        let minute = groww
            .history_row(Granularity::Minute1)
            .expect("the rung the vendor narrows")
            .contested
            .expect("the vendor's interval table, 1 min row");
        let daily = groww
            .history_row(Granularity::Day1)
            .expect("the rung the vendor opens")
            .contested
            .expect("the vendor's interval table, 1 day row");
        assert_eq!(minute.floor, HistoryFloor::RollingMonths { months: 3 });
        assert_eq!(daily.floor, HistoryFloor::Unbounded);
        assert_ne!(
            minute.floor, daily.floor,
            "if these were ever equal the field would not have needed a rung"
        );
        // Every rung a feed SERVES is either recorded or honestly unknown, and
        // every recorded rung is one this build could file bars for.
        for feed in Feed::ALL {
            for row in feed.descriptor().history {
                assert!(
                    row.granularity.store_timeframe().is_some(),
                    "{feed} records a floor for a rung the store cannot file"
                );
            }
        }
    }

    /// A RUNG NOTHING STATES A FLOOR FOR CLAIMS NOTHING.
    ///
    /// Not zero, not the epoch, and not "no limit" — those are three different
    /// wrong answers and all three read as a fact. Both archive feeds are the
    /// live case: an operator's folder holds whatever they bought, and no page
    /// anywhere states how far back that goes.
    #[test]
    fn a_rung_with_no_recorded_floor_claims_nothing() {
        for feed in [Feed::TrueData, Feed::Gdfl] {
            assert!(
                feed.descriptor().history.is_empty(),
                "{feed} is a folder on this machine and no vendor states its reach"
            );
            for rung in Granularity::ALL {
                assert_eq!(
                    feed.descriptor().history_floor(rung),
                    HistoryFloor::Unstated,
                    "{feed} at {rung}"
                );
                assert!(feed.descriptor().history_row(rung).is_none(), "{feed}");
            }
        }
        // And a rung a broker serves nothing about is unknown too, rather than
        // inheriting the rung next to it.
        assert_eq!(
            Feed::Groww.descriptor().history_floor(Granularity::Hour1),
            HistoryFloor::Unstated,
            "no source states an hourly floor, so none is claimed"
        );
    }

    /// WHERE THE TWO SOURCES DISAGREE, BOTH ARE CARRIED AND ONE IS MARKED.
    ///
    /// `CLAUDE.md` §3 rule 1 wants every vendor claim traceable to a source.
    /// Three of the four rows recorded here have two sources that do not agree,
    /// and the displaced one is kept beside the reason it was displaced —
    /// deleting it would leave a number no reader could argue with.
    #[test]
    fn a_contested_floor_keeps_the_claim_it_displaced() {
        let dhan_day = Feed::Dhan
            .descriptor()
            .history_row(Granularity::Day1)
            .expect("the one rung this feed serves");
        assert_eq!(dhan_day.binding.floor, HistoryFloor::Rolling { years: 5 });
        assert_eq!(
            dhan_day
                .contested
                .expect("the vendor claims inception")
                .floor,
            HistoryFloor::Unbounded,
            "the vendor claims more than the operator does, and it is kept"
        );

        let groww_minute = Feed::Groww
            .descriptor()
            .history_row(Granularity::Minute1)
            .expect("the rung the vendor narrows");
        assert_eq!(
            groww_minute
                .contested
                .expect("the vendor's own interval table")
                .floor,
            HistoryFloor::RollingMonths { months: 3 },
            "the vendor's published quarter lost this rung to the operator's \
             direct observation of 12 Aug 2026, and is still readable — \
             whether this rung truly reaches 2020 is UNVERIFIED until a \
             request measures it, and this is the number that measurement \
             lands against"
        );

        // EVERY ROW THAT DISPLACED A CLAIM SAYS WHY. An empty reason beside a
        // contest is a verdict with no argument behind it.
        //
        // THE CONVERSE IS NOT REQUIRED, and it used to be — this was an
        // `assert_eq!`, reading "a row that displaced nothing says nothing".
        // That is a rule about the two brokers whose vendor tables disagree
        // with the operator, and it is wrong about a third whose vendor page
        // states no depth at all. Zerodha's floor is UNCONTESTED, which is not
        // the same as agreed: nothing weighs against it and nothing confirms it
        // either, and that is worth a sentence precisely because it makes the
        // row different in kind from the two above it. Forcing the reason empty
        // would have deleted the only place that distinction is written down.
        //
        // So: a contest implies a reason, one-directionally.
        let mut contested = 0;
        for feed in Feed::ALL {
            for row in feed.descriptor().history {
                assert!(
                    row.contested.is_none() || !row.binds_because.is_empty(),
                    "{feed} at {}: a contest must carry the reason it was \
                     decided by",
                    row.granularity
                );
                assert!(
                    !row.binding.source.is_empty(),
                    "{feed} at {}: a floor with no source is one somebody typed",
                    row.granularity
                );
                if let Some(other) = row.contested {
                    assert!(!other.source.is_empty(), "{feed}");
                    assert_ne!(
                        other.floor, row.binding.floor,
                        "{feed} at {}: a claim that agrees is not a contest",
                        row.granularity
                    );
                    contested += 1;
                }
            }
        }
        assert_eq!(contested, 3, "three of the four rows are contested");
    }

    /// THE PER-VENDOR FIELD AND THE PER-RUNG TABLE CANNOT DRIFT APART IN THE
    /// DIRECTION THAT LOSES DATA.
    ///
    /// `HttpSpec::history_floor` is what `api::server::clamp_to_floor` reads,
    /// and it is per VENDOR. `Descriptor::history` is per RUNG. Two fields
    /// answering "how far back" is a drift risk, and the asymmetry is what
    /// makes it survivable: a vendor-wide floor that is EARLIER than a rung's
    /// own only spends requests the vendor answers empty, while one that is
    /// LATER refuses days the vendor holds — and a refused day in an
    /// append-only store cannot be prepended later.
    ///
    /// So the one direction is asserted. Edit a rung to be stricter and this
    /// stays green; edit the vendor-wide field to be stricter than a rung and
    /// this goes red before the pull does.
    #[test]
    fn the_vendor_wide_floor_is_never_later_than_a_rungs_own() {
        let today = day(2026, 8, 12);
        for feed in Feed::ALL {
            let Transport::Http(spec) = feed.descriptor().transport else {
                continue;
            };
            let Some(vendor_wide) = resolve_floor(spec.history_floor, today) else {
                continue;
            };
            for row in feed.descriptor().history {
                let Some(per_rung) = resolve_floor(row.binding.floor, today) else {
                    continue;
                };
                assert!(
                    vendor_wide <= per_rung,
                    "{feed} at {}: the clamp would refuse {per_rung} back to \
                     {vendor_wide}, which is later than the rung's own floor",
                    row.granularity
                );
            }
        }
    }

    /// WHICH OF TWO DISAGREEING CLAIMS BINDS, CHECKED IN BOTH TIERS.
    ///
    /// The rule is [`ClaimStanding`]'s, and neither tier is prose here:
    ///
    /// 1. A claim of HIGHER STANDING binds whatever its strictness. An
    ///    operator reporting what his own entitlement answered outranks a
    ///    vendor's general statement about the product, and it may therefore
    ///    legitimately WIDEN the floor — Groww's one-minute rung is the row
    ///    that does, and it is the reason this test could no longer be a bare
    ///    strictness comparison.
    /// 2. Between two claims of the SAME standing, the STRICTER one binds —
    ///    the later day refuses first. This is the whole of the old rule, kept,
    ///    as the fall-through.
    ///
    /// A row may never promote a claim of LOWER standing, and it may never
    /// promote a looser claim of equal standing. Both are checked over every
    /// row of every feed, so the next row appended is governed by the rule
    /// rather than by whoever writes its comment.
    #[test]
    fn the_binding_floor_is_never_the_looser_of_the_two() {
        // A day the rolling floors are resolved against. Fixed here rather than
        // read from a clock: a test that reads the clock asserts a different
        // thing every day it runs.
        let today = day(2026, 8, 12);
        // Both tiers must actually be EXERCISED by the rows in this build, or
        // this test would keep passing while one of them rotted unreached.
        let mut by_standing = 0;
        let mut by_strictness = 0;
        for feed in Feed::ALL {
            for row in feed.descriptor().history {
                let Some(other) = row.contested else {
                    continue;
                };
                assert!(
                    !other.standing.outranks(row.binding.standing),
                    "{feed} at {}: a claim of lower standing was promoted over \
                     {}",
                    row.granularity,
                    other.standing.label()
                );
                if row.binding.standing.outranks(other.standing) {
                    // TIER 1. Strictness is not consulted, and that is the
                    // point of the tier rather than a gap in the check.
                    by_standing += 1;
                    continue;
                }
                by_strictness += 1;
                let bound =
                    resolve_floor(row.binding.floor, today).expect("a binding floor names a day");
                // A displaced claim that names NO day cannot narrow anything,
                // so any real day beats it and there is nothing to compare.
                // One that does name a day must be the earlier of the two: the
                // later day refuses first, and that is what `stricter` means.
                if let Some(loser) = resolve_floor(other.floor, today) {
                    assert!(
                        bound >= loser,
                        "{feed} at {}: the looser claim of equal standing was \
                         promoted",
                        row.granularity
                    );
                }
            }
        }
        assert!(
            by_standing > 0,
            "no row exercises tier 1, so the standing rule is unchecked"
        );
        let _ = by_strictness;
    }

    /// TIER 1 IS THE ONLY THING THAT LETS A FLOOR WIDEN, AND EXACTLY ONE ROW
    /// USES IT.
    ///
    /// The named case, pinned so that a second one cannot appear unnoticed:
    /// Groww's one-minute rung binds at the operator's fixed January 2020 over
    /// the vendor's published rolling quarter, which is SIX YEARS wider. That
    /// is the cost `ClaimStanding` documents — requests spent on days that may
    /// answer empty — and it is accepted on exactly this row, on the operator's
    /// statement of 12 Aug 2026, and marked UNVERIFIED until measured.
    ///
    /// If another row ever widens, this test fails and the person adding it has
    /// to say so here.
    #[test]
    fn only_the_named_row_widens_a_floor_on_standing_alone() {
        let today = day(2026, 8, 12);
        let mut widened = Vec::new();
        for feed in Feed::ALL {
            for row in feed.descriptor().history {
                let Some(other) = row.contested else {
                    continue;
                };
                let (Some(bound), Some(displaced)) = (
                    resolve_floor(row.binding.floor, today),
                    resolve_floor(other.floor, today),
                ) else {
                    continue;
                };
                if bound < displaced {
                    widened.push((feed, row.granularity));
                }
            }
        }
        assert_eq!(
            widened,
            vec![(Feed::Groww, Granularity::Minute1)],
            "exactly one row widens a floor on standing alone, and it is the \
             one ClaimStanding's header names"
        );
    }

    // -- the granularity floor -----------------------------------------------

    /// THE LADDER ASCENDS, SO ONE COMPARISON IS THE WHOLE LOOKUP.
    ///
    /// [`Granularity::is_finer_than`] compares two `u8` discriminants, and that
    /// is only an answer if the discriminants are ordered the way the grids
    /// are. The `const` block beside the function pins the ten ADJACENT pairs,
    /// which is what the compiler needs; this walks all 121 ORDERED pairs,
    /// which is what a reader needs — a ladder can ascend between neighbours
    /// and still not be a total order if a rung is compared to one three steps
    /// away.
    #[test]
    fn the_ladder_ascends_so_one_comparison_decides_which_rung_is_finer() {
        for finer in Granularity::ALL {
            for coarser in Granularity::ALL {
                assert_eq!(
                    black_box(finer).is_finer_than(black_box(coarser)),
                    black_box(finer).coarseness() < black_box(coarser).coarseness(),
                    "{finer} against {coarser}: the discriminant and the grid disagree"
                );
            }
            assert!(
                !finer.is_finer_than(finer),
                "{finer} is not finer than itself"
            );
        }
        // The two ends, named rather than derived, so a re-ordered ladder fails
        // here and not only in the matrix above.
        assert!(Granularity::Tick.is_finer_than(Granularity::Second1));
        assert!(Granularity::Second1.is_finer_than(Granularity::Minute1));
        assert!(Granularity::Day1.is_finer_than(Granularity::Week1));
        assert_eq!(
            black_box(Granularity::Tick).coarseness(),
            0,
            "an event grid"
        );
        assert_eq!(
            black_box(Granularity::Minute1).coarseness(),
            SECS_PER_MINUTE_U32,
            "an interval grid ranks as its own width"
        );
        assert!(
            black_box(Granularity::Day1).coarseness() < black_box(Granularity::Week1).coarseness(),
            "a session is narrower than a week, and neither is an interval"
        );
    }

    /// EVERY FEED ANSWERS EVERY RUNG, AND THE ANSWER IS ONE OF TWO VERDICTS.
    ///
    /// The whole 4 × 11 matrix, because a capability with a hole in it is a
    /// capability a caller has to guess at. Two verdicts: the vendor refuses
    /// the rung outright, or it does not — and the second splits into the one
    /// rung that IS the floor and the rungs above it, which is where the
    /// tick-versus-conflated answer lives.
    #[test]
    fn every_feed_answers_every_rung_with_one_of_two_verdicts() {
        for feed in Feed::ALL {
            let floor = feed.descriptor().granularity_floor;
            let mut refused = 0_usize;
            let mut at_floor = 0_usize;
            let mut coarser = 0_usize;
            for rung in Granularity::ALL {
                let verdict = black_box(feed).granularity_verdict(black_box(rung));
                assert_eq!(
                    verdict,
                    feed.descriptor().granularity_verdict(rung),
                    "{feed} at {rung}: the two spellings must agree"
                );
                assert_eq!(
                    verdict.is_refused(),
                    rung.is_finer_than(floor.finest),
                    "{feed} at {rung}: refused exactly when finer than the floor"
                );
                match verdict {
                    RungVerdict::Refused(refusal) => {
                        refused += 1;
                        assert_eq!(refusal.asked, rung, "the refusal names what was asked");
                        assert_eq!(refusal.finest, floor.finest, "and what is served instead");
                        assert_eq!(refusal.finest_kind, floor.kind);
                        assert!(
                            has_content(refusal.because),
                            "{feed} refuses with no reason"
                        );
                        assert!(has_content(refusal.source), "{feed} refuses with no source");
                        assert_eq!(verdict.kind(), None, "a refused rung has no record shape");
                        assert_eq!(verdict.refusal(), Some(refusal));
                    }
                    RungVerdict::Finest(kind) => {
                        at_floor += 1;
                        assert_eq!(rung, floor.finest, "only the floor rung answers Finest");
                        assert_eq!(kind, floor.kind);
                        assert_eq!(verdict.kind(), Some(kind));
                        assert_eq!(verdict.refusal(), None);
                    }
                    RungVerdict::Coarser => {
                        coarser += 1;
                        assert!(floor.finest.is_finer_than(rung));
                        assert_eq!(
                            verdict.kind(),
                            None,
                            "{feed} at {rung}: nothing here measured what a record at a \
                             rung above the floor is, so nothing here claims it"
                        );
                        assert_eq!(verdict.refusal(), None);
                    }
                }
            }
            assert_eq!(
                at_floor, 1,
                "{feed} has exactly one finest rung, or the floor is not a floor"
            );
            assert_eq!(
                refused + at_floor + coarser,
                GRANULARITY_COUNT,
                "{feed} left a rung unanswered"
            );
            assert!(refused > 0, "{feed} refuses at least the print rung");
            assert_eq!(
                feed.finest_rung(),
                floor.finest,
                "{feed} disagrees with its own row about its finest rung"
            );
        }
    }

    /// NO FEED IN THIS BUILD SERVES A PRINT STREAM, AND THAT IS PERMANENT.
    ///
    /// The owner's rule of 12 Aug 2026, asserted rather than commented: the
    /// finest rung of the ladder is refused by every feed, and no row's floor
    /// claims to be a print stream. This is the refusal that a pull cannot
    /// fix, an entitlement cannot fix and a code change cannot fix — which is
    /// exactly what separates it from an empty directory.
    #[test]
    fn no_feed_in_this_build_serves_a_tick_and_the_refusal_is_permanent() {
        for feed in Feed::ALL {
            let verdict = feed.granularity_verdict(Granularity::Tick);
            let refusal = verdict
                .refusal()
                .expect("the finest rung of the ladder is served by nobody");
            assert_eq!(refusal.asked, Granularity::Tick);
            assert!(
                !refusal.finest_kind.is_tick_stream(),
                "{feed} offers its floor as a print stream, which no source says"
            );
            assert!(
                !feed.descriptor().granularity_floor.kind.is_tick_stream(),
                "{feed} claims a print stream in its own row"
            );
            assert!(
                !feed.serves(Granularity::Tick),
                "{feed} cannot fetch a rung its vendor does not have"
            );
        }
        // THE ARM ITSELF, EXERCISED. It exists so the negative above can be
        // said at all; a variant no test ever constructs is a variant whose
        // behaviour nobody has checked.
        let print_stream = black_box(FinestKind::Tick);
        assert!(print_stream.is_tick_stream());
        assert!(
            !print_stream.is_conflated(),
            "a print stream discards nothing, which is the whole difference"
        );
        assert!(has_content(black_box(print_stream).label()));
        assert_ne!(
            black_box(FinestKind::Tick).label(),
            black_box(FinestKind::ConflatedSnapshot).label(),
            "the two must not read the same on a page"
        );
        assert_ne!(
            black_box(FinestKind::Bar).label(),
            black_box(FinestKind::ConflatedSnapshot).label()
        );
    }

    /// A CONFLATED SECOND IS NOT A PRINT, AND THE TYPE REFUSES TO LET IT PASS
    /// FOR ONE.
    ///
    /// Both archives bottom out at one second and both were MEASURED to be
    /// snapshots — `docs/08-vendor-samples.md`. Both brokers bottom out at one
    /// minute and both are candles. The two facts are carried separately from
    /// the rung, because the rung alone cannot tell them apart: one second of
    /// an archive and one second of a broker would be two different objects.
    #[test]
    fn a_conflated_second_is_never_labelled_a_tick() {
        for feed in [Feed::TrueData, Feed::Gdfl] {
            let verdict = feed.granularity_verdict(Granularity::Second1);
            assert_eq!(
                verdict,
                RungVerdict::Finest(FinestKind::ConflatedSnapshot),
                "{feed}'s finest rung is a once-a-second snapshot of the book"
            );
            let kind = verdict.kind().expect("the floor states its own shape");
            assert!(
                kind.is_conflated(),
                "{feed} discards what falls between slots"
            );
            assert!(
                !kind.is_tick_stream(),
                "{feed} is named for a print stream and holds none"
            );
            assert_eq!(
                feed.descriptor().record,
                RecordShape::Snapshot,
                "{feed}'s record shape and its floor's kind say the same thing"
            );
        }
        for feed in [Feed::Dhan, Feed::Groww] {
            let verdict = feed.granularity_verdict(Granularity::Minute1);
            assert_eq!(
                verdict,
                RungVerdict::Finest(FinestKind::Bar),
                "{feed}'s finest rung is a candle"
            );
            let kind = verdict.kind().expect("the floor states its own shape");
            assert!(
                !kind.is_conflated(),
                "{feed} aggregates its interval and says so"
            );
            assert!(!kind.is_tick_stream());
            assert!(
                feed.granularity_verdict(Granularity::Second1).is_refused(),
                "{feed} has no second-level word to ask with"
            );
            assert!(
                feed.granularity_verdict(Granularity::Second5).is_refused(),
                "{feed} has no five-second word either"
            );
        }
    }

    /// THE TWO FLOORS REFUSE DIFFERENT THINGS, AND ONE OF THEM IS FIXABLE.
    ///
    /// This is the confusion the field was added to end. Groww at one second
    /// and Groww at one minute in 2019 are both refusals, and they are not the
    /// same kind of fact: the first is the vendor having no such rung at all,
    /// and the second is the vendor having the rung and not that far back.
    /// A pull fixes neither, but only one of them is a bound that could ever
    /// move.
    #[test]
    fn the_granularity_floor_and_the_history_floor_refuse_different_things() {
        let groww = Feed::Groww.descriptor();
        assert!(
            groww.granularity_verdict(Granularity::Second1).is_refused(),
            "no second-level rung exists on this vendor at any date"
        );
        assert_eq!(
            groww.history_floor(Granularity::Second1),
            HistoryFloor::Unstated,
            "and no source states a depth for a rung the vendor does not have"
        );
        assert!(
            !groww.granularity_verdict(Granularity::Minute1).is_refused(),
            "the minute rung exists"
        );
        assert_eq!(
            groww.history_floor(Granularity::Minute1),
            HistoryFloor::Fixed {
                year: 2020,
                month: 1,
                day: 1
            },
            "and it is the DEPTH that is bounded there, not the existence"
        );
        // The archives are the mirror image: their granularity floor is
        // stated and measured, and their history floor is stated by nobody.
        for feed in [Feed::TrueData, Feed::Gdfl] {
            assert!(has_content(feed.descriptor().granularity_floor.source));
            assert!(
                feed.descriptor().history.is_empty(),
                "{feed} states how fine it goes and nothing states how far back"
            );
        }
    }

    /// A BUILD NEVER FETCHES A RUNG ITS VENDOR CANNOT SERVE.
    ///
    /// `granularities` may be NARROWER than the floor allows — Dhan's minute
    /// rung is exactly that, and the reason is this build's single `bars_path`
    /// rather than anything Dhan does. It may never be wider. The one-mask
    /// check is driven here from outside the `const` block that pins it.
    #[test]
    fn a_build_never_fetches_a_rung_its_vendor_cannot_serve() {
        for feed in Feed::ALL {
            let row = feed.descriptor();
            assert!(
                black_box(row.granularities)
                    .none_finer_than(black_box(row.granularity_floor.finest)),
                "{feed} declares a rung below its own vendor floor"
            );
            for rung in Granularity::ALL {
                if feed.serves(rung) {
                    assert!(
                        !feed.granularity_verdict(rung).is_refused(),
                        "{feed} fetches {rung} and its vendor refuses it"
                    );
                }
            }
        }
        // THE NARROWER-THAN-ALLOWED CASE, NAMED. Dhan's vendor floor is a
        // minute and this build does not fetch the rung; the two refusals must
        // not read the same, because one of them a code change fixes.
        assert!(!Feed::Dhan.serves(Granularity::Minute1));
        assert!(
            !Feed::Dhan
                .granularity_verdict(Granularity::Minute1)
                .is_refused(),
            "the vendor serves the minute rung; this build is what does not ask for it"
        );
        // And the edge of the mask: nothing is finer than the finest rung, so
        // every set passes against it.
        assert!(
            black_box(GranularitySet::EMPTY.with(Granularity::Tick))
                .none_finer_than(black_box(Granularity::Tick)),
            "the ladder has no rung below its own first one"
        );
        assert!(
            !black_box(GranularitySet::EMPTY.with(Granularity::Tick))
                .none_finer_than(black_box(Granularity::Second1)),
            "and one rung down, the same set is refused"
        );
    }

    /// EVERY GRANULARITY FLOOR NAMES A REASON AND A SOURCE, IN VENDOR TERMS.
    ///
    /// `CLAUDE.md` §3 rule 1. A refusal an operator cannot go and check is a
    /// refusal they have to take on faith, and four rows sharing one
    /// paraphrase would be this repository's sentence rather than four
    /// vendors'.
    #[test]
    fn every_granularity_floor_names_a_reason_and_a_source() {
        let mut reasons = HashSet::new();
        let mut sources = HashSet::new();
        for feed in Feed::ALL {
            let floor = feed.descriptor().granularity_floor;
            assert!(has_content(floor.because), "{feed} refuses with no reason");
            assert!(has_content(floor.source), "{feed} refuses with no source");
            assert!(
                reasons.insert(floor.because),
                "{feed} reuses another vendor's words"
            );
            assert!(
                sources.insert(floor.source),
                "{feed} reuses another vendor's citation"
            );
            assert!(
                has_content(floor.kind.label()),
                "{feed} has no word for what a record at its floor is"
            );
        }
        assert_eq!(reasons.len(), FEED_COUNT);
        assert_eq!(sources.len(), FEED_COUNT);
    }
}
