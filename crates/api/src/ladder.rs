//! THE PULL ORDER, ENFORCED — cheap pass first, and nothing expensive runs on
//! an unproven feed.
//!
//! # The operator's rule, 15 Aug 2026, verbatim
//!
//! > "except truedata and gdfl, always first of all one and only when the
//! > entire underlying spot one day gets pulled, one and only then one minute
//! > should be pulled. And once this is finished … always pull spot as the
//! > first and then expired futures as the next step, and one and only when all
//! > these are entirely extremely fully successful alone only then go ahead with
//! > the expired options. Even in all these three also always pull one day as
//! > the first step and one and only when it fully pulled entirely successful
//! > alone only then one minute."
//!
//! # Why order is worth enforcing rather than recommending
//!
//! `crates/store` already carries the arithmetic: the stated backfill is **81
//! windows per instrument at one minute and 14 at day level — 5.8× fewer
//! requests for the same span**. The day pass is therefore the cheap way to
//! discover that a feed, a symbol or a date range is wrong, and running it first
//! is the difference between finding out after 14 requests or after 81.
//!
//! That sequencing has been written down since D-0054 and nothing enforced it.
//! A rule that lives only in a doc comment is a rule the form does not have.
//!
//! # THE GATE READS THE STORE, NOT THE JOURNAL
//!
//! The obvious implementation asks the audit journal "did the day pass
//! succeed", and it is the wrong one twice over.
//!
//! **The journal cannot answer.** [`crate::audit::Record`] carries `scope`,
//! `source` and `outcome` and **no rung** — there is no field that distinguishes
//! a day run from a minute one, so the question cannot be put to it without a
//! journal format change.
//!
//! **And the store is the better witness anyway.** A journal says a run once
//! *claimed* success. The census says the bars are *there*, keyed on
//! `(exchange, segment, symbol, timeframe, month)` — exactly the tuple this
//! question is about. A day pass that reported `Stored` and wrote nothing —
//! [`crate::audit::Outcome::Empty`], which balances trivially at `0 = 0 + 0 + 0`
//! — passes a journal check and fails this one. That case is the whole reason
//! the gate exists.
//!
//! # Cost
//!
//! One hash probe per (symbol, month) — [`Manifest::entry`] is a map lookup and
//! this walks the caller's own list rather than the census. `O(names × months)`
//! probes, each `O(1)`, and it opens no file: the census is already loaded by
//! the caller that is about to write to it.
//!
//! **The `O(1)` per probe is `UNVERIFIED` and the multiplier is not.** Taking
//! the two halves separately, because only one of them is measured:
//!
//! - `months` is proven. `api::ladder::a_window_names_every_month_file_it_touches`
//!   asserts the enumeration returns exactly the months a window touches — one
//!   month inside a month, two across a month boundary, and the year boundary
//!   where a naive `+1` breaks. So the multiplier is the size of the ASK.
//! - `names` is structural, not measured: this iterates the caller's own list
//!   and the census is never walked, which is visible in the loop and is why
//!   the store's size does not appear in the bound at all.
//! - The per-probe `O(1)` is the unmeasured half. No test here isolates one
//!   `HashMap::get`, and `docs/04-invariants.md` C-03 says outright that the
//!   nearest measurement "does NOT isolate a probe's own cost".
//!
//! Recorded in `docs/06-limits.md`. Naming a test that proves something
//! adjacent would be the exact defect gate 12 was written to catch.

use brutex_core::instrument::{Exchange, Kind, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{EntryKey, Manifest};
use pull::vendor::{Feed, SourceKind};
use store::path::{Timeframe, YearMonth};

/// What stands between a request and the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    /// Nothing. The request may run.
    Open,
    /// A RUNG the operator's order puts first is not fully held.
    ///
    /// Carries the counts rather than a sentence, so the caller can render the
    /// refusal in its own words and a test can assert the arithmetic.
    RungFirst {
        /// The rung that has to land first.
        needs: Timeframe,
        /// Instrument-months of `needs` that are missing.
        missing: usize,
        /// Instrument-months the window asks for in total.
        of: usize,
    },
    /// A LEG the operator's order puts first is not COMPLETE.
    ///
    /// Complete means at both rungs. The operator's words are "one and only
    /// when all these are entirely extremely fully successful alone only then
    /// go ahead", so a futures pass does not begin because spot's day landed.
    LegFirst {
        /// The leg that has to finish first.
        needs: Leg,
        /// The rung of that leg which is short.
        at: Timeframe,
        /// Instrument-months of `needs` missing at `at`.
        missing: usize,
        /// Instrument-months this request asks for in that leg.
        of: usize,
    },
    /// A SEGMENT the operator's order puts first is not fully held.
    SegmentFirst {
        /// The segment that has to land first.
        needs: Segment,
        /// The rung it is missing at.
        at: Timeframe,
        /// Instrument-months of that segment missing at that rung.
        missing: usize,
        /// Instrument-months the window asks for in total.
        of: usize,
    },
}

impl Gate {
    /// Whether the request may proceed.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(*self, Self::Open)
    }
}

/// THE RUNG THAT MUST LAND BEFORE `rung`, or [`None`] where `rung` is first.
///
/// The operator's ladder is one day, then one minute. Everything finer than a
/// minute is an ARCHIVE rung — the per-second files `TrueData` and GDFL sell —
/// and those feeds are exempt from the whole order, so a sub-minute rung has no
/// predecessor here and never reaches this function on a gated feed.
///
/// Everything coarser than a minute is DERIVED from it rather than pulled
/// (`pull::ingest::derived_from`), so it is never requested and has no place in
/// a pull order either.
#[must_use]
pub fn precedes(rung: Timeframe) -> Option<Timeframe> {
    (rung == Timeframe::MINUTE_1).then_some(Timeframe::DAY_1)
}

/// THE SEGMENT THAT MUST LAND BEFORE `segment`, or [`None`] where it is first.
///
/// Spot before derivatives, which is the operator's order and is also the only
/// one that can be checked: everything else prices off the underlying, so a
/// derivative window with no spot behind it cannot be verified against anything.
///
/// # Futures before options is NOT expressed here, and the absence is honest
///
/// The operator's order is spot → expired futures → expired options.
/// [`Segment`] has three variants — `Index`, `Cash`, `Fno` — and BOTH
/// derivative legs are `Fno`. The store keys a month on that enum, so it cannot
/// tell a futures month from an options month and this function cannot ask it
/// to. Splitting them is a `brutex_core::instrument` change with a store-path
/// consequence, and inventing a distinction here that the census cannot answer
/// would be a gate that reports on a fact nobody records.
#[must_use]
pub fn segment_precedes(segment: Segment) -> Option<Segment> {
    matches!(segment, Segment::Fno).then_some(Segment::Index)
}

/// One step of the operator's order, coarser than a rung and finer than a
/// segment.
///
/// # Why this exists, and why `Segment` could not carry it
///
/// The operator's order, 15 Aug 2026, is **spot → expired futures → expired
/// options**, and each of those three finishes its DAY pass before its MINUTE
/// pass. Six steps, strictly ordered.
///
/// [`Segment`] has three variants — `Index`, `Cash`, `Fno` — and **both**
/// derivative legs are `Fno`. So a segment cannot tell a futures month from an
/// options month, and this module said so in as many words: *"inventing a
/// distinction here that the census cannot answer would be a gate that reports
/// on a fact nobody records."*
///
/// That was true and it is no longer. `brutex_core::instrument::Kind`
/// distinguishes `Future` from `Option`, and since the contract path landed the
/// store records the difference too: a future files under `<expiry>-FUT` and an
/// option under `<expiry>-<strike>-<side>`. The fact is recorded now, so the
/// gate may read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Leg {
    /// The underlying — an index or a cash equity.
    Spot,
    /// An expired futures contract.
    Future,
    /// An expired options contract.
    Option,
}

impl Leg {
    /// The three legs, in the order the operator gave them.
    pub const ORDER: [Self; 3] = [Self::Spot, Self::Future, Self::Option];

    /// Which leg an instrument belongs to.
    #[must_use]
    pub const fn of(kind: Kind) -> Self {
        match kind {
            Kind::Index | Kind::Equity => Self::Spot,
            Kind::Future { .. } => Self::Future,
            Kind::Option { .. } => Self::Option,
        }
    }

    /// Every leg that must be COMPLETE before this one may start.
    ///
    /// Complete means at **both** rungs, which is the operator's own words:
    /// *"one and only when all these are entirely extremely fully successful
    /// alone only then go ahead"*. A futures pass does not begin because spot's
    /// day landed — it begins when spot's day AND spot's minute have.
    #[must_use]
    pub fn before(self) -> &'static [Self] {
        match self {
            Self::Spot => &[],
            Self::Future => &[Self::Spot],
            Self::Option => &[Self::Spot, Self::Future],
        }
    }

    /// The word an operator reads.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Spot => "spot",
            Self::Future => "expired futures",
            Self::Option => "expired options",
        }
    }
}

/// Every month a request for `leg` must actually cover, given the months its
/// window names.
///
/// # A FUTURES PULL REACHES BACK ONE MONTH, and it has to
///
/// The operator's rule, 15 Aug 2026: *"when we start to pull futures always
/// ensure to fetch it for the current month and as well as even the previous
/// month also"*.
///
/// The reason is the instrument, not a preference. A futures contract expiring
/// in September was LISTED and TRADING through August — the front month rolls
/// on the last Thursday, so for most of a contract's life the month named on it
/// is not the month it traded in. A window that asked only for September would
/// fetch the last few sessions of a contract that had been trading for weeks,
/// and the bars before the roll — where the volume actually is — would never be
/// asked for at all.
///
/// Spot has no such thing: an index trades in the month it is named for and in
/// no other, so its month set is exactly the window's.
///
/// Options are left at the window's own months. They are not exempted from the
/// idea; the rule as stated names futures, and widening a fetch for an
/// instrument the operator did not name would be inventing scope — the reach
/// for an option chain is a per-expiry question this build has no measurement
/// for yet.
///
/// # Cost
///
/// At most one extra month per month asked for, so the set at most doubles and
/// is never a function of the store. The predecessor is arithmetic on a month
/// ordinal, not a calendar walk.
#[must_use]
pub fn months_for(leg: Leg, months: &[YearMonth]) -> Vec<YearMonth> {
    let mut out: Vec<YearMonth> = Vec::with_capacity(months.len().saturating_mul(2));
    for &m in months {
        if leg == Leg::Future
            && let Some(prev) = month_before(m)
        {
            out.push(prev);
        }
        out.push(m);
    }
    // ONE ROW PER MONTH, AND NO SORT.
    //
    // The result is already ascending, so a sort would be work that changes
    // nothing — and gate 11 rule 4 bans one here for exactly that reason: a
    // sort on a request path is the construct that turned a bounded page into
    // an O(universe) one elsewhere in this build.
    //
    // Why it is already ordered. `months` is ascending. For consecutive entries
    // m1 < m2 the emitted run is `prev(m1), m1, prev(m2), m2`, and
    // `m1 <= prev(m2)` in every case: if m2 is the very next month then
    // prev(m2) IS m1, and if m2 is further out then prev(m2) is later than m1.
    // So the sequence never steps backwards, and the only repeat it can produce
    // is ADJACENT — which is precisely what `dedup_by_key` removes.
    out.dedup_by_key(|m| (m.year(), m.month()));
    out
}

/// The month before `m`, or [`None`] at the store's floor.
///
/// Arithmetic on the ordinal rather than a calendar step, so January is the
/// same cost as June and the year boundary cannot be got wrong by a `- 1`.
#[must_use]
pub fn month_before(m: YearMonth) -> Option<YearMonth> {
    let ordinal = u32::from(m.year()) * 12 + u32::from(m.month() - 1);
    let back = ordinal.checked_sub(1)?;
    let year = u16::try_from(back / 12).ok()?;
    let month = u8::try_from(back % 12 + 1).ok()?;
    YearMonth::new(year, month).ok()
}

/// The six steps of the order, in sequence.
///
/// `(leg, rung)` — spot day, spot minute, futures day, futures minute, options
/// day, options minute. The whole rule in one table, which is what a test can
/// assert against and what a page can draw.
#[must_use]
pub fn steps() -> Vec<(Leg, Timeframe)> {
    let mut out = Vec::with_capacity(6);
    for leg in Leg::ORDER {
        out.push((leg, Timeframe::DAY_1));
        out.push((leg, Timeframe::MINUTE_1));
    }
    out
}

/// One instrument-month the caller is about to ask for.
///
/// # The segment is PER INSTRUMENT, and it has to be
///
/// The first draft took one `Segment` for the whole request, which reads as
/// obvious and is wrong for two of the nine spot targets. `SpotTarget::Fno` is
/// the 213 F&O **underlyings** — NIFTY and BANKNIFTY are indices and the other
/// 211 are equities — and `Everything` is the reference index series and the
/// NIFTY Total Market constituents together. Both span `Index` and `Cash`.
///
/// The store keys a month on the segment, so probing a target's whole name list
/// under one of them would look for equities in the index directory and find
/// nothing: the gate would report every constituent as missing and refuse a run
/// that was ready. The caller knows each instrument's segment because the
/// catalogue it resolved the names from carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wanted {
    /// Which step of the order this instrument belongs to.
    ///
    /// From `Leg::of(key.kind)` at the call site — the instrument's own kind,
    /// never the request's, because one request can name instruments in more
    /// than one leg and the order is about each of them separately.
    pub leg: Leg,
    /// The instrument.
    pub symbol: Symbol,
    /// Which venue the store files it under.
    ///
    /// Per instrument for the same reason the segment is, and it was very
    /// nearly not: the caller's list is a filter over the merged universe by
    /// `catalog::tracked`, which selects on the UNIVERSE flags and never on the
    /// exchange. Nothing on that path makes a batch single-venue, so taking the
    /// first target's exchange for all of them would be an assumption the code
    /// does not enforce — and the store keys a month on it, so a wrong one
    /// probes a directory the bars were never written to and reports a held
    /// month as missing.
    pub exchange: Exchange,
    /// Which segment the store files it under.
    pub segment: Segment,
    /// The month its bars fall in.
    pub month: YearMonth,
}

/// Whether `feed` is subject to the order at all.
///
/// The operator exempted the two archives by name — *"except truedata and
/// gdfl"* — and the reason is structural rather than a favour: a folder feed
/// issues no request. Its input is the per-second file already on disk, every
/// coarser rung is folded from it in one pass, and there is no expensive second
/// call for a cheap first one to protect. An order exists to decide what a
/// mistake costs, and here a mistake costs a re-read.
#[must_use]
pub fn is_gated(feed: Feed) -> bool {
    matches!(feed.source_kind(), SourceKind::Rest)
}

/// Every month a window touches, in order.
///
/// The store addresses one month per file, so a window is a set of month files
/// and the gate's question is asked once per (instrument, month). Walked from
/// the window's own ends rather than counted, because a month's length is not a
/// constant and `end_of_month` is the calendar's answer rather than arithmetic
/// on 30.
///
/// # Errors
///
/// None — a window whose ends are already `Day`s cannot name a month that is not
/// a month. A day that will not convert is skipped rather than refusing the
/// walk: it cannot happen for a constructed `Window`, and a gate that refused a
/// whole run over an unrepresentable month would be a refusal nobody could act
/// on.
#[must_use]
pub fn months_of(window: pull::session::Window) -> Vec<YearMonth> {
    let mut out = Vec::new();
    let mut at = window.from();
    loop {
        if let Ok(ym) = at.year_month() {
            out.push(ym);
        }
        let last = at.end_of_month();
        if last.days_from_epoch() >= window.to().days_from_epoch() {
            return out;
        }
        let Ok(next) = pull::session::Day::from_days(last.days_from_epoch().saturating_add(1))
        else {
            return out;
        };
        at = next;
    }
}

/// May this request run?
///
/// # Errors
///
/// None — a refusal is a [`Gate`] variant carrying its own counts, never an
/// `Err`. The caller decides whether a closed gate is a `409`, a warning or a
/// disabled button, and it needs the numbers either way.
#[must_use]
pub fn gate<F>(held: F, feed: Feed, rung: Timeframe, wanted: &[Wanted]) -> Gate
where
    F: Fn(&EntryKey) -> bool,
{
    // AN ARCHIVE IS NOT GATED, AND AN EMPTY ASK IS NOT REFUSED.
    //
    // A window naming nothing has no prerequisite that could be missing, and
    // refusing it here would refuse it for the wrong reason: the form already
    // says a request with no instrument-month is not a request.
    if !is_gated(feed) || wanted.is_empty() {
        return Gate::Open;
    }

    // THE LEG ORDER, CHECKED FIRST AND AT BOTH RUNGS.
    //
    // Spot, then expired futures, then expired options — and each COMPLETE
    // before the next begins, which is what the operator's "entirely extremely
    // fully successful" means and why this checks DAY_1 and MINUTE_1 rather
    // than the rung being asked for. A futures minute pass that started because
    // spot's day had landed would be exactly the order the rule forbids.
    //
    // Reported ahead of the rung check for the same reason the segment one was:
    // a request failing both would otherwise send the operator to pull the day
    // rung of a leg they should not be on yet.
    //
    // The earliest unfinished predecessor wins, so the operator is sent to the
    // FIRST thing that is missing rather than the last.
    for leg in Leg::ORDER {
        let mine: Vec<&Wanted> = wanted.iter().filter(|w| w.leg == leg).collect();
        if mine.is_empty() {
            continue;
        }
        for needs in leg.before() {
            for at in [Timeframe::DAY_1, Timeframe::MINUTE_1] {
                // EVERY MONTH THIS REQUEST TOUCHES, under the earlier leg. The
                // instruments are this request's own — the gate never walks the
                // store — so an underlying the operator did not ask for is not
                // demanded of them.
                // UNDER THE PREDECESSOR'S OWN SEGMENT, not this one's. A
                // futures contract is filed under FNO and the spot bars it
                // waits for are not — they are under INDEX or CASH. Probing
                // FNO for them finds nothing and would refuse a leg that was
                // ready, every time.
                let missing = mine
                    .iter()
                    .filter(|w| {
                        let seg = match needs {
                            Leg::Spot => segment_precedes(w.segment).unwrap_or(w.segment),
                            Leg::Future | Leg::Option => w.segment,
                        };
                        !held(&key(w.exchange, seg, at, w.symbol, w.month))
                    })
                    .count();
                if missing > 0 {
                    return Gate::LegFirst {
                        needs: *needs,
                        at,
                        missing,
                        of: mine.len(),
                    };
                }
            }
        }
    }

    // THE SEGMENT GATE, and it fires only where an instrument is a derivative.
    //
    // Checked first, and the order matters: a derivative window with no spot
    // behind it fails BOTH gates, and reporting the rung one would send the
    // operator to pull the day rung of a segment he should not be on yet.
    //
    // Every derivative in `wanted` is checked against its own underlying
    // segment. On a SPOT request this loop finds nothing — `/pull/spot` names
    // index and cash instruments and `segment_precedes` answers `None` for both
    // — which is correct rather than a gap: the order's derivative step belongs
    // to `/pull/fno`, whose transport does not exist in this build.
    let derivative: Vec<Wanted> = wanted
        .iter()
        .filter(|w| segment_precedes(w.segment).is_some())
        .copied()
        .collect();
    if let Some(first) = derivative.first().and_then(|w| segment_precedes(w.segment)) {
        // AT THE RUNG BEING ASKED FOR, not at the finest one. Requiring spot's
        // MINUTE bars before an F&O day pull would be stricter than the rule,
        // and a gate stricter than its rule refuses work nobody called wrong.
        let missing = derivative
            .iter()
            .filter(|w| !held(&key(w.exchange, first, rung, w.symbol, w.month)))
            .count();
        if missing > 0 {
            return Gate::SegmentFirst {
                needs: first,
                at: rung,
                missing,
                of: derivative.len(),
            };
        }
    }

    if let Some(first) = precedes(rung) {
        let missing = wanted
            .iter()
            .filter(|w| !held(&key(w.exchange, w.segment, first, w.symbol, w.month)))
            .count();
        if missing > 0 {
            return Gate::RungFirst {
                needs: first,
                missing,
                of: wanted.len(),
            };
        }
    }

    Gate::Open
}

/// The one instrument-month a probe asks about, at `(segment, rung)`.
///
/// Every field is named. The five are positional at the two call sites above
/// and the two orders differ — the segment gate asks about the SPOT segment at
/// the rung being pulled, the rung gate about this instrument's OWN segment at
/// the rung below — so a struct with named fields is what keeps that difference
/// legible rather than a pair of five-argument calls nobody can read.
const fn key(
    exchange: Exchange,
    segment: Segment,
    rung: Timeframe,
    symbol: Symbol,
    month: YearMonth,
) -> EntryKey {
    EntryKey {
        contract: None,
        exchange,
        segment,
        symbol,
        timeframe: rung,
        month,
    }
}

/// Whether the census holds a given instrument-month — [`gate`]'s `held`.
///
/// One map probe. Callers walk their OWN list, so the count is the size of the
/// ask and never the size of the store.
///
/// The probe is [`Manifest::entry`], a single `HashMap::get`. Its `O(1)` is
/// `UNVERIFIED` for the reason the module doc gives: no test here isolates one
/// lookup, and `docs/04-invariants.md` C-03 states that the nearest measurement
/// does not isolate a probe's own cost either. What IS proven is the count —
/// see `api::ladder::a_window_names_every_month_file_it_touches`.
///
/// # Absent is the whole test, and it covers the dangerous case
///
/// A draft also treated a month held with ZERO rows as missing, to catch
/// `audit::Outcome::Empty` — the run that completes clean and stores nothing,
/// balances trivially at `0 = 0 + 0 + 0`, and is the one outcome that looks
/// exactly like success. That clause is unreachable: `Manifest::record_held`
/// REFUSES an entry with no rows outright, as `EmptyEntry`, so a zero-row month
/// cannot be in the census to be read. An `Empty` run leaves NO entry, and this
/// refuses it by absence — see `a_month_with_no_bars_cannot_even_be_recorded`.
///
/// # Why a closure and not a `&Manifest`
///
/// Because a census that is **absent** is not a census that is empty of
/// meaning: `census::Census` has three states and only one of them carries a
/// `Manifest`. `Absent` is the ordinary state before the first ingest and it
/// answers `|_| false` — nothing is held, so a rung with a prerequisite is
/// refused and one without is not. Passing a `&Manifest` would force the caller
/// to either fabricate an empty one or re-implement this module's arithmetic,
/// and `Unreadable` has no honest answer here at all — it is refused by the
/// caller before a probe is ever built. Same shape as
/// `autopilot::next_window`, for the same reason.
pub fn probing(census: &Manifest) -> impl Fn(&EntryKey) -> bool + '_ {
    |k| census.entry(k).is_some()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that asserts nothing is banned, and a test that cannot \
              fail loudly is a test that asserts nothing"
)]
mod tests {
    use super::*;
    use brutex_core::vendor::Vendor;
    use pull::manifest::{Entry, Held};

    fn month() -> YearMonth {
        YearMonth::new(2026, 8).expect("a real month")
    }

    fn sym(name: &str) -> Symbol {
        Symbol::new(name).expect("a legal symbol")
    }

    /// Instruments in the INDEX segment, which is what a spot request names.
    fn want(names: &[&str]) -> Vec<Wanted> {
        want_in(Segment::Index, names)
    }

    fn want_in(segment: Segment, names: &[&str]) -> Vec<Wanted> {
        names
            .iter()
            .map(|n| Wanted {
                leg: Leg::Spot,
                symbol: sym(n),
                exchange: Exchange::Nse,
                segment,
                month: month(),
            })
            .collect()
    }

    /// A census holding `rows` bars for each name, at one segment and rung.
    fn census_of(held: &[(&str, Segment, Timeframe, u64)]) -> Manifest {
        // `open` with two empty slices IS the genesis census — its own body
        // says so — and it is the public door. A test that reached for the
        // private constructor would be asserting against a shape no caller can
        // build.
        let mut m = Manifest::open(Vendor::Groww, &[], &[]).expect("a genesis census");
        for &(name, segment, timeframe, rows) in held {
            m.record_held(Held::unknown(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment,
                    symbol: sym(name),
                    timeframe,
                    month: month(),
                },
                rows,
                first_ts_micros: 1,
                last_ts_micros: 2,
            }))
            .expect("the census has room");
        }
        m
    }

    // ======================================================================
    // The rung gate — one day before one minute
    // ======================================================================

    /// THE DAY RUNG IS FIRST AND HAS NOTHING IN FRONT OF IT.
    #[test]
    fn the_day_rung_is_never_gated() {
        assert_eq!(precedes(Timeframe::DAY_1), None);
        let empty = census_of(&[]);
        assert_eq!(
            gate(
                probing(&empty),
                Feed::Groww,
                Timeframe::DAY_1,
                &want(&["NIFTY"])
            ),
            Gate::Open,
            "nothing precedes the cheap pass, so an empty store cannot block it"
        );
    }

    /// THE MINUTE RUNG IS REFUSED UNTIL EVERY MONTH IS HELD AT DAY LEVEL, and
    /// the refusal carries the arithmetic rather than a sentence.
    #[test]
    fn the_minute_rung_waits_for_the_day_pass_and_names_what_is_missing() {
        assert_eq!(precedes(Timeframe::MINUTE_1), Some(Timeframe::DAY_1));
        let two = want(&["NIFTY", "BANKNIFTY"]);

        // Neither held.
        let none = census_of(&[]);
        assert_eq!(
            gate(probing(&none), Feed::Groww, Timeframe::MINUTE_1, &two),
            Gate::RungFirst {
                needs: Timeframe::DAY_1,
                missing: 2,
                of: 2
            }
        );

        // PARTIAL IS STILL CLOSED, and this is the case a boolean would lose.
        // One of two months held is not "the day pass ran": the minute pass
        // would spend 5.8x the requests on a window half of which has never
        // been proved.
        let half = census_of(&[("NIFTY", Segment::Index, Timeframe::DAY_1, 10)]);
        assert_eq!(
            gate(probing(&half), Feed::Groww, Timeframe::MINUTE_1, &two),
            Gate::RungFirst {
                needs: Timeframe::DAY_1,
                missing: 1,
                of: 2
            }
        );

        // Both held — open.
        let both = census_of(&[
            ("NIFTY", Segment::Index, Timeframe::DAY_1, 10),
            ("BANKNIFTY", Segment::Index, Timeframe::DAY_1, 10),
        ]);
        assert!(gate(probing(&both), Feed::Groww, Timeframe::MINUTE_1, &two).is_open());
    }

    /// THE DANGEROUS OUTCOME LEAVES NO ENTRY AT ALL, which is why `is_none` is
    /// the whole test.
    ///
    /// `audit::Outcome::Empty` is a run that completes clean and stores
    /// nothing: no member errored, the books balance at 0 = 0 + 0 + 0, and it
    /// is the one outcome that looks exactly like success. A journal-based gate
    /// would pass it.
    ///
    /// It cannot fool this one, and not because the gate checks for it — the
    /// CENSUS refuses to record a month with no rows, as `EmptyEntry`. So such
    /// a run leaves no entry, `is_none` refuses it, and the guarantee lives one
    /// layer down where it cannot be forgotten. Asserted here so that if the
    /// census ever starts accepting an empty row, this fails rather than the
    /// minute pass quietly opening.
    #[test]
    fn a_month_with_no_bars_cannot_even_be_recorded() {
        let mut census = Manifest::open(Vendor::Groww, &[], &[]).expect("a genesis census");
        let why = census
            .record_held(Held::unknown(Entry {
                key: EntryKey {
                    contract: None,
                    exchange: Exchange::Nse,
                    segment: Segment::Index,
                    symbol: sym("NIFTY"),
                    timeframe: Timeframe::DAY_1,
                    month: month(),
                },
                rows: 0,
                first_ts_micros: 1,
                last_ts_micros: 2,
            }))
            .expect_err("a month with no bars is not a month the census records");
        assert!(
            format!("{why:?}").contains("Empty"),
            "the refusal names the emptiness: {why:?}"
        );

        // AND SO THE GATE STAYS SHUT, by absence rather than by inspection.
        assert_eq!(
            gate(
                probing(&census),
                Feed::Groww,
                Timeframe::MINUTE_1,
                &want(&["NIFTY"])
            ),
            Gate::RungFirst {
                needs: Timeframe::DAY_1,
                missing: 1,
                of: 1
            },
            "an Empty run left no entry, so the day pass has not landed"
        );
    }

    // ======================================================================
    // The segment gate — spot before derivatives
    // ======================================================================

    /// F&O WAITS FOR SPOT, AND THE SEGMENT REFUSAL WINS OVER THE RUNG ONE.
    ///
    /// A derivative window with no spot behind it fails both gates. Reporting
    /// the rung one would send the operator to pull the day rung of a segment
    /// he should not be on yet.
    #[test]
    fn derivatives_wait_for_spot_and_that_refusal_is_reported_first() {
        assert_eq!(segment_precedes(Segment::Fno), Some(Segment::Index));
        assert_eq!(segment_precedes(Segment::Index), None);

        let nothing = census_of(&[]);
        assert_eq!(
            gate(
                probing(&nothing),
                Feed::Groww,
                Timeframe::MINUTE_1,
                &want_in(Segment::Fno, &["NIFTY"])
            ),
            Gate::SegmentFirst {
                needs: Segment::Index,
                at: Timeframe::MINUTE_1,
                missing: 1,
                of: 1
            },
            "spot first — not the day rung of a segment that should not be running"
        );
    }

    /// AND ONCE SPOT IS HELD AT THAT RUNG, THE RUNG GATE TAKES OVER.
    #[test]
    fn with_spot_held_the_derivative_still_owes_its_own_day_pass() {
        let spot_only = census_of(&[
            ("NIFTY", Segment::Index, Timeframe::MINUTE_1, 375),
            ("NIFTY", Segment::Index, Timeframe::DAY_1, 10),
        ]);
        assert_eq!(
            gate(
                probing(&spot_only),
                Feed::Groww,
                Timeframe::MINUTE_1,
                &want_in(Segment::Fno, &["NIFTY"])
            ),
            Gate::RungFirst {
                needs: Timeframe::DAY_1,
                missing: 1,
                of: 1
            },
            "spot is satisfied; the derivative's own day pass is not"
        );
    }

    // ======================================================================
    // The exemptions
    // ======================================================================

    /// THE TWO ARCHIVES ARE EXEMPT BY NAME, and the reason is structural.
    ///
    /// A folder feed issues no request: its input is the per-second file
    /// already on disk and every coarser rung is folded from it in one pass.
    /// An order exists to decide what a mistake costs, and here it costs a
    /// re-read.
    #[test]
    fn a_folder_feed_is_never_gated() {
        let nothing = census_of(&[]);
        for feed in [Feed::TrueData, Feed::Gdfl] {
            assert!(!is_gated(feed), "{feed} is a folder");
            assert!(
                gate(
                    probing(&nothing),
                    feed,
                    Timeframe::MINUTE_1,
                    &want_in(Segment::Fno, &["NIFTY"])
                )
                .is_open(),
                "{feed} runs whatever the store holds"
            );
        }
        for feed in [Feed::Dhan, Feed::Groww, Feed::Zerodha] {
            assert!(is_gated(feed), "{feed} is a broker and is gated");
        }
    }

    /// AN EMPTY ASK IS OPEN, not refused. A window naming nothing has no
    /// prerequisite that could be missing, and the form already refuses a
    /// request with no instrument-month for its own reason.
    #[test]
    fn a_request_naming_nothing_is_not_refused_by_the_ladder() {
        let nothing = census_of(&[]);
        assert!(gate(probing(&nothing), Feed::Groww, Timeframe::MINUTE_1, &[]).is_open());
    }

    /// EVERY RUNG THAT IS NEITHER FIRST NOR THE MINUTE HAS NO PREDECESSOR.
    ///
    /// The derived rungs are folded rather than pulled, and the sub-minute one
    /// belongs to feeds this order exempts — so neither appears in a pull
    /// order, and `precedes` says so for all of them rather than for the two
    /// that happened to be tested.
    #[test]
    fn only_the_minute_rung_has_a_predecessor() {
        for tf in Timeframe::KNOWN {
            let expected = (*tf == Timeframe::MINUTE_1).then_some(Timeframe::DAY_1);
            assert_eq!(
                precedes(*tf),
                expected,
                "{} has the wrong predecessor",
                tf.as_str()
            );
        }
    }

    /// AND THE GATE OPENS FOR EVERY UNGATED RUNG WHATEVER THE STORE HOLDS.
    #[test]
    fn an_ungated_rung_runs_against_an_empty_store() {
        let nothing = census_of(&[]);
        for tf in Timeframe::KNOWN.iter().filter(|t| precedes(**t).is_none()) {
            assert!(
                gate(probing(&nothing), Feed::Groww, *tf, &want(&["NIFTY"])).is_open(),
                "{} has no predecessor and must not be blocked",
                tf.as_str()
            );
        }
    }

    /// A TARGET THAT SPANS TWO SEGMENTS IS PROBED IN BOTH, and this is the case
    /// a request-level segment could not express.
    ///
    /// `SpotTarget::Fno` is the 213 F&O **underlyings**: NIFTY and BANKNIFTY are
    /// indices, the other 211 are equities. `Everything` mixes the same two.
    /// The store keys a month on the segment, so probing that whole list under
    /// one of them looks for equities in the index directory, finds nothing,
    /// and reports every constituent as missing — refusing a run that was ready.
    #[test]
    fn a_mixed_target_is_probed_under_each_instruments_own_segment() {
        // The day pass landed: the index under INDEX, the equity under CASH.
        let both = census_of(&[
            ("NIFTY", Segment::Index, Timeframe::DAY_1, 10),
            ("RELIANCE", Segment::Cash, Timeframe::DAY_1, 10),
        ]);
        let mixed = vec![
            Wanted {
                leg: Leg::Spot,
                symbol: sym("NIFTY"),
                exchange: Exchange::Nse,
                segment: Segment::Index,
                month: month(),
            },
            Wanted {
                leg: Leg::Spot,
                symbol: sym("RELIANCE"),
                exchange: Exchange::Nse,
                segment: Segment::Cash,
                month: month(),
            },
        ];
        assert!(
            gate(probing(&both), Feed::Groww, Timeframe::MINUTE_1, &mixed).is_open(),
            "both landed, each in its own segment — the minute pass may run"
        );

        // AND THE SAME NAMES UNDER ONE SEGMENT WOULD HAVE REPORTED A LIE. This
        // is the old shape, kept as an assertion so the reason the signature
        // changed cannot be forgotten: probing the equity under INDEX finds
        // nothing.
        let as_if_all_index = want_in(Segment::Index, &["NIFTY", "RELIANCE"]);
        assert_eq!(
            gate(
                probing(&both),
                Feed::Groww,
                Timeframe::MINUTE_1,
                &as_if_all_index
            ),
            Gate::RungFirst {
                needs: Timeframe::DAY_1,
                missing: 1,
                of: 2
            },
            "the equity is not in the index directory, and never was"
        );
    }

    /// THE VENUE IS READ FROM THE INSTRUMENT TOO, and this is what proves it.
    ///
    /// `Wanted::exchange` was added after `segment`, for the same reason and
    /// with the same failure mode — the store keys a month on it, so probing
    /// the wrong venue reports a held month as missing. Without this test the
    /// field could be ignored entirely and every existing assertion would still
    /// pass, because they are all `Nse` on both sides.
    #[test]
    fn a_month_held_at_one_venue_is_not_held_at_another() {
        let nse = census_of(&[("NIFTY", Segment::Index, Timeframe::DAY_1, 10)]);
        let elsewhere = vec![Wanted {
            leg: Leg::Spot,
            symbol: sym("NIFTY"),
            exchange: Exchange::Bse,
            segment: Segment::Index,
            month: month(),
        }];
        assert_eq!(
            gate(probing(&nse), Feed::Groww, Timeframe::MINUTE_1, &elsewhere),
            Gate::RungFirst {
                needs: Timeframe::DAY_1,
                missing: 1,
                of: 1
            },
            "the day pass landed on NSE; a BSE month of the same name and \
             segment is a different file and the census does not hold it"
        );
        // AND THE SAME ASK AT THE VENUE IT LANDED ON IS OPEN — so the refusal
        // above is the exchange and nothing else about this fixture.
        assert!(
            gate(
                probing(&nse),
                Feed::Groww,
                Timeframe::MINUTE_1,
                &want(&["NIFTY"])
            )
            .is_open(),
            "same name, same segment, same month, correct venue"
        );
    }

    /// THE SIX STEPS, IN THE OPERATOR'S ORDER, ASSERTED AS A SEQUENCE.
    ///
    /// spot day → spot minute → futures day → futures minute → options day →
    /// options minute. This is the whole rule of 15 Aug 2026 in one assertion.
    #[test]
    fn the_order_is_six_steps_and_each_leg_finishes_before_the_next_begins() {
        assert_eq!(
            steps(),
            vec![
                (Leg::Spot, Timeframe::DAY_1),
                (Leg::Spot, Timeframe::MINUTE_1),
                (Leg::Future, Timeframe::DAY_1),
                (Leg::Future, Timeframe::MINUTE_1),
                (Leg::Option, Timeframe::DAY_1),
                (Leg::Option, Timeframe::MINUTE_1),
            ],
            "spot, then expired futures, then expired options — day before \
             minute inside each"
        );
        assert_eq!(Leg::Spot.before(), &[]);
        assert_eq!(Leg::Future.before(), &[Leg::Spot]);
        assert_eq!(
            Leg::Option.before(),
            &[Leg::Spot, Leg::Future],
            "options wait for BOTH, not only for futures"
        );
    }

    /// A LEG IS NOT STARTED BY ITS PREDECESSOR'S DAY PASS ALONE.
    ///
    /// This is the clause that makes the order mean what the operator said:
    /// "one and only when all these are entirely extremely fully successful".
    /// Spot's day landing is not spot finishing.
    #[test]
    fn futures_wait_for_spot_to_finish_both_rungs_not_just_its_day_pass() {
        let want_fut = |names: &[&str]| -> Vec<Wanted> {
            names
                .iter()
                .map(|n| Wanted {
                    leg: Leg::Future,
                    symbol: sym(n),
                    exchange: Exchange::Nse,
                    segment: Segment::Fno,
                    month: month(),
                })
                .collect()
        };

        // Spot's DAY pass has landed and its MINUTE pass has not.
        let half = census_of(&[("NIFTY", Segment::Index, Timeframe::DAY_1, 10)]);
        assert_eq!(
            gate(
                probing(&half),
                Feed::Groww,
                Timeframe::DAY_1,
                &want_fut(&["NIFTY"])
            ),
            Gate::LegFirst {
                needs: Leg::Spot,
                at: Timeframe::MINUTE_1,
                missing: 1,
                of: 1
            },
            "the day pass alone does not open the futures leg — and the refusal \
             names the MINUTE rung, which is the one still owed"
        );

        // Both rungs of spot held: the futures leg opens.
        let both = census_of(&[
            ("NIFTY", Segment::Index, Timeframe::DAY_1, 10),
            ("NIFTY", Segment::Index, Timeframe::MINUTE_1, 3750),
        ]);
        assert!(
            gate(
                probing(&both),
                Feed::Groww,
                Timeframe::DAY_1,
                &want_fut(&["NIFTY"])
            )
            .is_open(),
            "spot finished, so the futures day pass may run"
        );
    }

    /// OPTIONS WAIT FOR FUTURES, AND THE REFUSAL NAMES THE EARLIEST GAP.
    #[test]
    fn options_are_refused_for_the_first_unfinished_leg_not_the_nearest_one() {
        let want_opt = vec![Wanted {
            leg: Leg::Option,
            symbol: sym("NIFTY"),
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            month: month(),
        }];
        // NOTHING held at all: the refusal must name SPOT, the first step, and
        // not FUTURES, the nearest one. An operator sent to the wrong step
        // spends a whole pass discovering it.
        let nothing = census_of(&[]);
        assert_eq!(
            gate(probing(&nothing), Feed::Groww, Timeframe::DAY_1, &want_opt),
            Gate::LegFirst {
                needs: Leg::Spot,
                at: Timeframe::DAY_1,
                missing: 1,
                of: 1
            },
            "the EARLIEST unfinished predecessor wins"
        );
    }

    /// EVERY KIND LANDS IN EXACTLY ONE LEG.
    #[test]
    fn every_instrument_kind_belongs_to_one_leg_of_the_order() {
        use brutex_core::instrument::{Expiry, OptionSide};
        use brutex_core::price::Paisa;
        let expiry = Expiry::new(2025, 9, 30).expect("a real expiry");
        assert_eq!(Leg::of(Kind::Index), Leg::Spot);
        assert_eq!(Leg::of(Kind::Equity), Leg::Spot);
        assert_eq!(Leg::of(Kind::Future { expiry }), Leg::Future);
        assert_eq!(
            Leg::of(Kind::Option {
                expiry,
                strike: Paisa::from_raw(2_465_000),
                side: OptionSide::Call,
            }),
            Leg::Option
        );
    }

    /// EVERY LEG NAMES ITSELF, AND NO TWO NAME THE SAME THING.
    ///
    /// `label` is the only place the order speaks to an operator — it is what
    /// `ladder_refusal` puts in "spot comes first, and it has to FINISH". A
    /// label that is empty, or that two legs share, turns a refusal that names
    /// the step you are missing into one that names nothing.
    ///
    /// Asserted here because nothing else does: the sentence is built in
    /// `crates/api/src/server.rs` and no test reads it back, so a mutation to
    /// any of these three strings survives the whole suite.
    #[test]
    fn every_leg_names_itself_and_no_two_legs_share_a_name() {
        assert_eq!(Leg::Spot.label(), "spot");
        assert_eq!(Leg::Future.label(), "expired futures");
        assert_eq!(Leg::Option.label(), "expired options");

        let names: Vec<&str> = Leg::ORDER.iter().map(|l| l.label()).collect();
        for (i, a) in names.iter().enumerate() {
            assert!(!a.is_empty(), "leg {i} names itself with nothing");
            for b in names.iter().skip(i + 1) {
                assert_ne!(a, b, "two legs share a name, so a refusal cannot say which");
            }
        }
        // AND THE ORDER IS THE ORDER. `ORDER` is what `steps` and the leg gate
        // both walk, so a reordering here silently reorders the whole rule.
        assert_eq!(Leg::ORDER, [Leg::Spot, Leg::Future, Leg::Option]);
    }

    /// A FUTURES PULL REACHES BACK ONE MONTH; SPOT DOES NOT.
    ///
    /// The operator's rule of 15 Aug 2026, and the instrument's own shape: a
    /// September future was listed and trading through August, so a window
    /// naming only September would miss the sessions before the roll — which is
    /// where the volume is.
    #[test]
    fn a_futures_pull_covers_the_previous_month_and_spot_covers_only_its_own() {
        let m = |y, mo| YearMonth::new(y, mo).expect("a real month");

        assert_eq!(
            months_for(Leg::Spot, &[m(2025, 9)]),
            vec![m(2025, 9)],
            "an index trades in the month it is named for and no other"
        );
        assert_eq!(
            months_for(Leg::Future, &[m(2025, 9)]),
            vec![m(2025, 8), m(2025, 9)],
            "a future reaches back one month, in order"
        );
        assert_eq!(
            months_for(Leg::Option, &[m(2025, 9)]),
            vec![m(2025, 9)],
            "options are left at the window's own months — the rule named \
             futures, and widening a fetch nobody asked to widen is invention"
        );

        // THE YEAR BOUNDARY, where a naive `month - 1` gives month zero.
        assert_eq!(
            months_for(Leg::Future, &[m(2025, 1)]),
            vec![m(2024, 12), m(2025, 1)],
            "January reaches back into the previous December"
        );

        // CONSECUTIVE MONTHS ARE NOT FETCHED TWICE. August is September's
        // predecessor and also its own month; asking twice would fetch and
        // count it twice.
        assert_eq!(
            months_for(Leg::Future, &[m(2025, 8), m(2025, 9)]),
            vec![m(2025, 7), m(2025, 8), m(2025, 9)],
            "three months, each once"
        );

        // AND THE STORE'S FLOOR IS NOT STEPPED OVER. 1970-01 has no
        // predecessor a `YearMonth` can hold, so the set is just itself.
        assert_eq!(
            months_for(Leg::Future, &[m(1970, 1)]),
            vec![m(1970, 1)],
            "no month before the epoch, and no panic reaching for one"
        );
    }

    // ======================================================================
    // The month walk
    // ======================================================================

    /// A WINDOW IS A SET OF MONTH FILES, and the walk names each one once.
    #[test]
    fn a_window_names_every_month_file_it_touches() {
        let day = |y, m, d| pull::session::Day::new(y, m, d).expect("a real day");
        let win = |a, b| pull::session::Window::new(a, b).expect("a forward window");

        // Inside one month.
        assert_eq!(
            months_of(win(day(2026, 8, 3), day(2026, 8, 14))),
            vec![YearMonth::new(2026, 8).expect("a month")]
        );
        // Across a boundary — both files, not one.
        assert_eq!(
            months_of(win(day(2026, 8, 28), day(2026, 9, 2))),
            vec![
                YearMonth::new(2026, 8).expect("a month"),
                YearMonth::new(2026, 9).expect("a month")
            ]
        );
        // Across a YEAR boundary, which is where a naive +1 on the month breaks.
        assert_eq!(
            months_of(win(day(2025, 12, 30), day(2026, 1, 2))),
            vec![
                YearMonth::new(2025, 12).expect("a month"),
                YearMonth::new(2026, 1).expect("a month")
            ]
        );
        // One day is one month.
        assert_eq!(
            months_of(win(day(2026, 2, 28), day(2026, 2, 28))),
            vec![YearMonth::new(2026, 2).expect("a month")]
        );
        // A long span names them all, in order, with no gap.
        let long = months_of(win(day(2026, 1, 1), day(2026, 12, 31)));
        assert_eq!(long.len(), 12, "twelve month files: {long:?}");
    }
}
