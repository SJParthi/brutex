//! Lot arithmetic: how many units a contract carries, and what that is worth.
//!
//! # Why the lot size is dated
//!
//! It moved seven times between 2021 and 2026. A backtest that used one lot
//! size across that window would mis-size a NIFTY position by **three times**
//! between April and November 2024 (25 units against 75) and a BANKNIFTY one by
//! more than two (15 against 35). Every charge in this crate that scales with
//! quantity would be wrong by the same factor, in the same direction, on every
//! trade in the window — a systematic error, not noise that averages out.
//!
//! So the lot size is a dated table with the same refusal contract as a
//! statutory rate: before the source's recorded history begins there is **no
//! lot size**, and a date landing there produces a [`Refusal`] rather than the
//! first recorded value stretched backwards.
//!
//! That last point is a deliberate divergence from the source's *documentation*
//! and a faithful port of its *code*. The source's own module docstring says
//! "for dates before the first transition, returns the first recorded value";
//! its `get_lot_size` raises instead. The two disagree, and this port follows
//! the code, because the docstring describes exactly the silent extrapolation
//! `CLAUDE.md` §4 bans. Recorded in `docs/06-limits.md`.
//!
//! # The arithmetic is multiplication and nothing else
//!
//! `quantity = lots × lot_size`, `notional = unit price × quantity`. Two
//! `checked_mul`s. There is no loop, no accumulation over legs and no summing
//! over a chain — a position of one lot and a position of a thousand cost the
//! same to compute. Every overflow is refused by name; nothing wraps and
//! nothing saturates, because a saturated quantity is a position size nobody
//! asked for.

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::price::Paisa;

use crate::dated::{Dated, DatedRow, DatedTable, boundary};
use crate::day::TradeDay;
use crate::error::{CostError, Refusal};
use crate::venue::SweptSlot;

/// How to close a lot-size refusal.
const LOT_REMEDIATION: &str = "source the exchange circular or bulletin fixing the lot size for \
     the window and add ONE dated row (effective date, lot size, citation) to the table in \
     crates/costs/src/lot.rs, with a docs/05-decisions.md ledger entry — zero mechanism change";

/// The first day the shipped lot sizes are citation-grounded.
///
/// The source's recorded history begins here, and it labels this first row a
/// "pre-Jul 2021 default (extended back to start of backfill)" — a value it
/// carried backwards to its own data floor. This crate does not carry it
/// further: [`TradeDay`] reaches to 1990 because `core`'s expiry calendar does,
/// and 1990..2020 has no evidence at all.
pub const LOT_VERIFIED_FROM: TradeDay = boundary!(2021 - 1 - 1);

/// The prose carried by every lot-size refusal row.
const LOT_GAP: &str = "UNVERIFIED — the options lot size before 2021-01-01. The source's recorded \
     history (`brutex/options/lot_size_history.py`, TRACK2_OPTIONS_SPEC §8.3, citation-grounded \
     2026-05-16) begins 2021-01-01 and labels that first row itself a value extended back to its \
     own backfill floor; no exchange circular fixing an earlier lot size was retrieved. \
     `TradeDay` reaches back to 1990 because `core`'s expiry calendar does, so this window has no \
     row. NOTE: the source's module docstring says a pre-history date returns the first recorded \
     value; its code raises instead, and this port follows the code — the docstring describes the \
     silent extrapolation `CLAUDE.md` §4 bans.";

/// How many units of the underlying one options contract carries.
///
/// Strictly positive by construction. A lot of zero would make every position
/// worth nothing and every charge zero, which is a bug that looks like a
/// profitable strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct LotSize(u32);

impl LotSize {
    /// Wraps a lot size. Crate-private, because the only lot sizes that exist
    /// are the ones the dated table below was built from, and a caller that
    /// could mint one could size a position off a number no circular carries.
    const fn new_const(units: u32) -> Self {
        Self(units)
    }

    /// The number of units, always strictly positive.
    #[must_use]
    pub const fn units(self) -> u32 {
        self.0
    }

    /// The number of units, widened for the arithmetic that consumes it.
    // A widening cast from an unsigned type: no truncation and no sign loss are
    // possible, so only the stylistic `cast_lossless` needs excusing — and
    // `i64::from` is not callable in a `const fn` on the pinned toolchain.
    #[allow(clippy::cast_lossless)]
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

impl core::fmt::Display for LotSize {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The lot-size history of each swept underlying, keyed on `core`'s own order.
///
/// Every row is the source's `_LOT_HISTORY_*` table with its citation carried
/// across, and the SENSEX table the source also holds is deliberately **not**
/// here: `CLAUDE.md` §1 fixes the engine surface at two NSE instruments, and a
/// third row would be a scope change smuggled in as data. See
/// [`crate::venue`], which makes the same argument about the venue map.
///
/// The array's length is [`SweptSlot::COUNT`], so widening the engine surface
/// stops this file compiling until it gains its table.
const LOT_SIZES: [DatedTable<LotSize>; SweptSlot::COUNT] = [
    DatedTable {
        subject: "options lot size (NIFTY)",
        exchange: Some(Exchange::Nse),
        remediation: LOT_REMEDIATION,
        anchor: DatedRow::unverified(TradeDay::MIN, LOT_GAP),
        later: [
            Some(DatedRow::verified(
                LOT_VERIFIED_FROM,
                LotSize::new_const(75),
                "75 — the pre-Jul-2021 NSE historic lot, which the source extended back to its \
                 backfill floor; TRACK2_OPTIONS_SPEC §8.3 row 1 (`NSE historic`).",
            )),
            Some(DatedRow::verified(
                boundary!(2021 - 7 - 29),
                LotSize::new_const(50),
                "50 — the July 2021 expiry rebasing; TRACK2_OPTIONS_SPEC §8.3 (`NSE bulletin`).",
            )),
            Some(DatedRow::verified(
                boundary!(2024 - 4 - 26),
                LotSize::new_const(25),
                "25 — effective 26-Apr-2024; TRACK2_OPTIONS_SPEC §8.3 (`NSE FAOP circular`).",
            )),
            Some(DatedRow::verified(
                boundary!(2024 - 11 - 20),
                LotSize::new_const(75),
                "75 — effective 20-Nov-2024, the SEBI minimum-notional rule; \
                 `SEBI/HO/MRD-PoD2/CIR/P/2024/00181`.",
            )),
            Some(DatedRow::verified(
                boundary!(2026 - 1 - 1),
                LotSize::new_const(65),
                "65 — January 2026 onward; TRACK2_OPTIONS_SPEC §8.3 (`NSE Jan-2026 circular`).",
            )),
        ],
    },
    DatedTable {
        subject: "options lot size (BANKNIFTY)",
        exchange: Some(Exchange::Nse),
        remediation: LOT_REMEDIATION,
        anchor: DatedRow::unverified(TradeDay::MIN, LOT_GAP),
        later: [
            Some(DatedRow::verified(
                LOT_VERIFIED_FROM,
                LotSize::new_const(25),
                "25 — the pre-Jun-2023 lot, which the source extended back to its backfill floor; \
                 TRACK2_OPTIONS_SPEC §8.3 row 1 (`NSE historic`).",
            )),
            Some(DatedRow::verified(
                boundary!(2023 - 6 - 30),
                LotSize::new_const(15),
                "15 — effective 30-Jun-2023; TRACK2_OPTIONS_SPEC §8.3 \
                 (`Zerodha Marketintel #353422`).",
            )),
            Some(DatedRow::verified(
                boundary!(2024 - 11 - 20),
                LotSize::new_const(30),
                "30 — effective 20-Nov-2024, the SEBI minimum-notional rule; \
                 `SEBI/HO/MRD-PoD2/CIR/P/2024/00181`.",
            )),
            Some(DatedRow::verified(
                boundary!(2025 - 7 - 1),
                LotSize::new_const(35),
                "35 — the July 2025 transitional lot; TRACK2_OPTIONS_SPEC §8.3 \
                 (`NSE bulletins`).",
            )),
            Some(DatedRow::verified(
                boundary!(2025 - 12 - 30),
                LotSize::new_const(30),
                "30 — the 30-Dec-2025 end-of-day revert; TRACK2_OPTIONS_SPEC §8.3 \
                 (`NSE FAOP70616`).",
            )),
        ],
    },
];

// ---------------------------------------------------------------------------
// COMPILE-TIME structural proof, and the slot binding.
//
// A table that anchors late, does not ascend, has a hole or carries a blank
// citation does not compile. And slot 0 must be the underlying `core` names
// first: a reordered SWEPT would size a BANKNIFTY position with NIFTY's lot,
// which on 2024-06-01 is 15 units charged as 25. It stops compiling instead.
// ---------------------------------------------------------------------------
const _: () = assert!(LOT_SIZES[0].is_shipping_shape());
const _: () = assert!(LOT_SIZES[1].is_shipping_shape());
const _: () = assert!(crate::dated::str_eq(InstrumentKey::SWEPT[0].1, "NIFTY"));
const _: () = assert!(crate::dated::str_eq(InstrumentKey::SWEPT[1].1, "BANKNIFTY"));

/// The lot size in force for `slot` on `day`, for contracts INTRODUCED that
/// day. While a revision is in transition older contracts carry the previous
/// lot: a caller that knows the contract asks [`lot_size_of`].
///
/// # Errors
///
/// [`Refusal`] for a day before [`LOT_VERIFIED_FROM`], naming the window, the
/// citation gap and the remedy. There is no default lot size.
///
/// # Examples
///
/// ```
/// use brutex_core::symbol::Symbol;
/// use costs::day::TradeDay;
/// use costs::lot::lot_size_on;
/// use costs::venue::swept_slot;
///
/// let nifty = swept_slot(Symbol::new("NIFTY")?)?;
///
/// // The 2024 rebasings, either side of the boundary that tripled the lot.
/// assert_eq!(lot_size_on(nifty, TradeDay::new(2024, 11, 19)?)?.units(), 25);
/// assert_eq!(lot_size_on(nifty, TradeDay::new(2024, 11, 20)?)?.units(), 75);
///
/// // Before the recorded history there is a refusal, never a guess.
/// assert!(lot_size_on(nifty, TradeDay::new(2020, 12, 31)?).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
// The slot's inner index is produced only by `venue::swept_slot`, which returns
// an index into `InstrumentKey::SWEPT`, and the array above has exactly that
// length by its own type. The access is in bounds by construction.
#[allow(clippy::indexing_slicing)]
pub fn lot_size_on(slot: SweptSlot, day: TradeDay) -> Result<LotSize, Refusal> {
    LOT_SIZES[slot.index()].value_on(day)
}

// ---------------------------------------------------------------------------
// The lot of a CONTRACT (DB-B04, WP-12e).
//
// A revision does not land on a trade day. Each circular says which contracts
// take the revised lot: the ones introduced from its effective day; the
// weekly and monthly contracts that already existed keep the existing lot to
// their expiry; the quarterly and half-yearly ones switch at the end of one
// named day. So on one trade day two lots are in force, and the trade-day
// table above answered one of them for every contract: on 2024-12-10 it said
// 75 for NIFTY while every December weekly, the 30-Jan-2025 monthly and every
// long-dated contract carried 25 (the audit measured 3x and 2x errors from
// Nov 2024 to Jan 2025, and 15% in Oct-Dec 2025).
// ---------------------------------------------------------------------------

/// One revision's rule for the contracts trading while both lots were in
/// force. Every date is the circular's own; an expiry the exchange later
/// re-dated is written under both days, because a caller may hold either.
#[derive(Debug, Clone, Copy)]
struct ContractRule {
    /// The row of the trade-day table this rule explains: the day that
    /// table turns from the existing lot to the revised one.
    boundary: TradeDay,
    /// The first trade day both lots are in force: contracts introduced from
    /// it carry the revised lot.
    from: TradeDay,
    /// The last trade day the rule answers. After it every contract carries
    /// the lot the trade-day table gives.
    through: TradeDay,
    /// The existing lot.
    old: LotSize,
    /// The revised lot.
    new: LotSize,
    /// Every contract expiring on or before this day keeps the existing lot.
    old_through_expiry: TradeDay,
    /// Later expiries that keep the existing lot to expiry: the circular's
    /// "last monthly expiry with existing lot size".
    old_expiries: &'static [TradeDay],
    /// The first expiry of the quarterly and half-yearly contracts that
    /// already existed.
    far_from_expiry: TradeDay,
    /// Those contracts carry the existing lot through this trade day (end of
    /// day) and the revised one after it.
    far_switch: TradeDay,
    /// The circular, and what it says.
    citation: &'static str,
}

impl ContractRule {
    /// The lot of the contract expiring on `expiry`, on a trade day the rule
    /// answers. Four compares and one bounded walk over `old_expiries`.
    fn lot_of(&self, expiry: TradeDay, day: TradeDay) -> LotSize {
        let keeps_old = expiry <= self.old_through_expiry
            || self.old_expiries.contains(&expiry)
            || (expiry >= self.far_from_expiry && day <= self.far_switch);
        if keeps_old { self.old } else { self.new }
    }
}

/// What `NSE/FAOP/64625` (18 Oct 2024) says, for both underlyings.
const FAOP64625: &str = "NSE/FAOP/64625 (18 Oct 2024,      https://nsearchives.nseindia.com/content/circulars/FAOP64625.pdf): the revised lot is      'effective for all new index derivatives contracts ... introduced from November 20, 2024';      'the existing weekly and monthly expiry contracts will continue with the existing lot size      till its respective expiry date'; existing quarterly and half-yearly contracts move 'on      December 24, 2024, end of the day for BANKNIFTY and December 26, 2024, end of the day for      NIFTY'. Annexure: NIFTY 30-Jan-2025 and BANKNIFTY 29-Jan-2025 are the last monthly expiries      with the existing lot; 27-Mar-2025 and 26-Mar-2025 the first existing quarterly ones. Read      by the audit and its second agent (2026-10-10) and by WP-12e (2026-10-11); docs/00-charter.md,      'NSE F&O circulars read for the options lineage', D-3357.";

/// What `NSE/FAOP/70616` (3 Oct 2025) says, for both underlyings.
const FAOP70616: &str = "NSE/FAOP/70616 (3 Oct 2025,      https://nsearchives.nseindia.com/content/circulars/FAOP70616.pdf): 'This circular shall come      into effect from October 28, 2025 (EOD). The existing lot size will be applicable for weekly      and monthly contracts till December 30, 2025 expiry date. Lot size of existing quarterly and      half yearly contracts will be revised on 30-Dec-2025 EOD.' Annexure: first monthly expiry      with the revised lot 27-Jan-2026; first existing quarterly expiry 31-Mar-2026. Read by the      audit and its second agent (2026-10-10) and by WP-12e (2026-10-11); docs/00-charter.md,      'NSE F&O circulars read for the options lineage', D-3357.";

/// How many sourced rules an underlying has: the bound of [`lot_size_of`]'s
/// walk.
const RULES_PER_SLOT: usize = 2;

/// The sourced contract rules, in [`LOT_SIZES`]' slot order. The revisions of
/// July 2021, June 2023, April 2024 and July 2025 have NO rule: their
/// circulars were not read, so [`lot_size_of`] REFUSES a contract's lot near
/// them ([`UNREAD_REVISION_DAYS`]) rather than answer the trade-day table
/// under a verified label. The audit measured contracts off that table
/// inside them (NIFTY 2021-07-15, BANKNIFTY 2023-06-15 and 2025-06-27).
const CONTRACT_RULES: [[ContractRule; RULES_PER_SLOT]; SweptSlot::COUNT] = [
    [
        ContractRule {
            boundary: boundary!(2024 - 11 - 20),
            from: boundary!(2024 - 11 - 20),
            through: boundary!(2025 - 1 - 30),
            old: LotSize::new_const(25),
            new: LotSize::new_const(75),
            old_through_expiry: boundary!(2024 - 12 - 26),
            old_expiries: &[boundary!(2025 - 1 - 30)],
            far_from_expiry: boundary!(2025 - 3 - 27),
            far_switch: boundary!(2024 - 12 - 26),
            citation: FAOP64625,
        },
        ContractRule {
            boundary: boundary!(2026 - 1 - 1),
            from: boundary!(2025 - 10 - 29),
            // The trade-day table turns to 65 on 2026-01-01; the circular's
            // last existing-lot contract expired on 2025-12-30, so the rule
            // answers 2025-12-31 too.
            through: boundary!(2025 - 12 - 31),
            old: LotSize::new_const(75),
            new: LotSize::new_const(65),
            old_through_expiry: boundary!(2025 - 12 - 30),
            old_expiries: &[],
            // Stated 31-Mar-2026; the contract settled on 30-Mar-2026.
            far_from_expiry: boundary!(2026 - 3 - 30),
            far_switch: boundary!(2025 - 12 - 30),
            citation: FAOP70616,
        },
    ],
    [
        ContractRule {
            boundary: boundary!(2024 - 11 - 20),
            from: boundary!(2024 - 11 - 20),
            through: boundary!(2025 - 1 - 30),
            old: LotSize::new_const(15),
            new: LotSize::new_const(30),
            // The December monthly: stated 26-Dec-2024, settled 24-Dec-2024.
            old_through_expiry: boundary!(2024 - 12 - 26),
            // Stated 29-Jan-2025; the contract settled on 30-Jan-2025.
            old_expiries: &[boundary!(2025 - 1 - 29), boundary!(2025 - 1 - 30)],
            // Stated 26-Mar-2025; the contract settled on 27-Mar-2025.
            far_from_expiry: boundary!(2025 - 3 - 26),
            far_switch: boundary!(2024 - 12 - 24),
            citation: FAOP64625,
        },
        ContractRule {
            boundary: boundary!(2025 - 12 - 30),
            from: boundary!(2025 - 10 - 29),
            through: boundary!(2025 - 12 - 30),
            old: LotSize::new_const(35),
            new: LotSize::new_const(30),
            old_through_expiry: boundary!(2025 - 12 - 30),
            old_expiries: &[],
            // Stated 31-Mar-2026; the contract settled on 30-Mar-2026.
            far_from_expiry: boundary!(2026 - 3 - 30),
            far_switch: boundary!(2025 - 12 - 30),
            citation: FAOP70616,
        },
    ],
];

/// How many calendar days either side of a lot revision with no sourced
/// contract rule a contract's lot is UNVERIFIED (D-3357). CHOSEN, not an
/// exchange fact: it is wider than both windows the read circulars state (71
/// days for `NSE/FAOP/64625`, 62 for `NSE/FAOP/70616`) and than the audit's
/// three contracts measured off the table (4 to 15 days before the row).
/// Outside it the trade-day row is answered, and says so.
pub const UNREAD_REVISION_DAYS: i32 = 92;

/// The citation gap [`lot_size_of`] refuses with inside an unread revision.
const UNREAD_REVISION: &str = "UNVERIFIED — the lot of a CONTRACT near a lot revision whose \
     circular was not read. The trade-day table changes rows here, the circular that says which \
     contracts carry which lot is not a row of CONTRACT_RULES, and the audit measured contracts \
     off the table inside such windows (NIFTY 2021-07-15: 50 against the table's 75; BANKNIFTY \
     2023-06-15: 15 against 25; BANKNIFTY 2025-06-27: 35 against 30).";

/// How to close that gap.
const UNREAD_REMEDIATION: &str = "read the revision's NSE F&O circular, record it in \
     docs/00-charter.md, and add ONE ContractRule row (its dates and the two lots) to \
     CONTRACT_RULES in crates/costs/src/lot.rs, with a docs/05-decisions.md entry — zero \
     mechanism change";

/// Where a contract's lot came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LotBasis {
    /// A circular's rule for the contracts trading while two lots were in
    /// force; the text is the citation.
    ContractRule(&'static str),
    /// The trade-day table, on a day more than [`UNREAD_REVISION_DAYS`] from
    /// every revision with no contract rule and outside every rule's window:
    /// one lot is in force for the contracts the read circulars speak of.
    /// Not proved per contract by a circular of its own.
    TradeDayRow,
}

/// A contract's lot and where it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractLot {
    /// The lot.
    pub lot: LotSize,
    /// Its basis.
    pub basis: LotBasis,
}

/// The revision of `slot`'s trade-day table that `day` lies within
/// [`UNREAD_REVISION_DAYS`] of and no contract rule explains: a row that
/// follows a verified row (the first verified row revises nothing) and is
/// no rule's `boundary`. At most `MAX_LATER_ROWS` rows, each one compare and
/// a walk over `RULES_PER_SLOT` rules.
// In bounds by construction, as in `lot_size_on`.
#[allow(clippy::indexing_slicing)]
fn unread_revision(slot: SweptSlot, day: TradeDay) -> Option<TradeDay> {
    let table = &LOT_SIZES[slot.index()];
    let mut previous = &table.anchor;
    for row in table.later.iter().flatten() {
        let revises =
            matches!(previous.value, Dated::Verified(_)) && matches!(row.value, Dated::Verified(_));
        let ruled = CONTRACT_RULES[slot.index()]
            .iter()
            .any(|rule| rule.boundary.same_day(row.start));
        let near = day
            .ordinal()
            .saturating_sub(row.start.ordinal())
            .saturating_abs()
            <= UNREAD_REVISION_DAYS;
        if revises && !ruled && near {
            return Some(row.start);
        }
        previous = row;
    }
    None
}

/// The lot of the `slot` contract expiring on `expiry`, on trade day `day`
/// (DB-B04, D-3357): the circular's contract rule while two lots were in
/// force, the trade-day table otherwise. `expiry` may be the day the name
/// states or the day the contract settled, except on a trade day after the
/// stated day of a contract re-dated later, where only the settled day is
/// on or after the trade day.
///
/// O(1): one compare, at most `RULES_PER_SLOT` window tests, a bounded walk
/// over the table's rows, then four compares or [`lot_size_on`]'s bounded
/// walk.
///
/// # Errors
///
/// * [`CostError::ExpiryBeforeTrade`] when `expiry` is before `day`: a
///   contract has no lot after it expires, and the rule in force that day
///   would otherwise answer for it.
/// * [`CostError::Unverified`] for a day within [`UNREAD_REVISION_DAYS`] of a
///   revision whose circular was not read (July 2021, June 2023, April 2024,
///   July 2025), naming the window and the remedy; and for a day before
///   [`LOT_VERIFIED_FROM`], exactly as [`lot_size_on`].
///
/// # Examples
///
/// ```
/// use brutex_core::symbol::Symbol;
/// use costs::day::TradeDay;
/// use costs::lot::lot_size_of;
/// use costs::venue::swept_slot;
///
/// let nifty = swept_slot(Symbol::new("NIFTY")?)?;
/// let on = TradeDay::new(2024, 12, 10)?;
/// // One trade day, two lots: the January monthly existed before the
/// // revision and keeps 25; the 2-Jan weekly was introduced after it.
/// assert_eq!(lot_size_of(nifty, TradeDay::new(2025, 1, 30)?, on)?.lot.units(), 25);
/// assert_eq!(lot_size_of(nifty, TradeDay::new(2025, 1, 2)?, on)?.lot.units(), 75);
/// // A revision whose circular was not read is refused, never guessed.
/// let unread = TradeDay::new(2021, 7, 15)?;
/// assert!(lot_size_of(nifty, TradeDay::new(2021, 7, 29)?, unread).is_err());
/// // And so is a contract asked about after it expired.
/// assert!(lot_size_of(nifty, TradeDay::new(2020, 1, 1)?, on).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
// In bounds by construction, as in `lot_size_on`.
#[allow(clippy::indexing_slicing)]
pub fn lot_size_of(
    slot: SweptSlot,
    expiry: TradeDay,
    day: TradeDay,
) -> Result<ContractLot, CostError> {
    if expiry.before(day) {
        return Err(CostError::ExpiryBeforeTrade { expiry, day });
    }
    for rule in &CONTRACT_RULES[slot.index()] {
        if rule.from <= day && day <= rule.through {
            return Ok(ContractLot {
                lot: rule.lot_of(expiry, day),
                basis: LotBasis::ContractRule(rule.citation),
            });
        }
    }
    if let Some(revision) = unread_revision(slot, day) {
        let table = &LOT_SIZES[slot.index()];
        let edge = |days: i32| revision.plus_days(days).unwrap_or(revision);
        return Err(CostError::Unverified(Refusal::with_remediation(
            table.subject,
            table.exchange,
            day,
            edge(-UNREAD_REVISION_DAYS),
            Some(edge(UNREAD_REVISION_DAYS.saturating_add(1))),
            UNREAD_REVISION,
            UNREAD_REMEDIATION,
        )));
    }
    Ok(ContractLot {
        lot: lot_size_on(slot, day)?,
        basis: LotBasis::TradeDayRow,
    })
}

/// How many units of the underlying `lots` contracts carry.
///
/// One multiplication.
///
/// # Errors
///
/// * [`CostError::NotPositive`] when `lots` is zero or negative. A trade of no
///   contracts is not a trade, and a negative lot count is a short expressed in
///   the wrong place — the side belongs on the order, not on the quantity.
/// * [`CostError::Overflow`] when the product leaves `i64`. Refused, never
///   saturated.
///
/// # Examples
///
/// ```
/// use costs::lot::contract_quantity;
/// # use brutex_core::symbol::Symbol;
/// # use costs::day::TradeDay;
/// # use costs::venue::swept_slot;
/// let lot = costs::lot::lot_size_on(
///     swept_slot(Symbol::new("NIFTY")?)?,
///     TradeDay::new(2024, 11, 20)?,
/// )?;
/// assert_eq!(contract_quantity(4, lot)?, 300, "four lots of 75");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn contract_quantity(lots: i64, lot_size: LotSize) -> Result<i64, CostError> {
    if lots <= 0 {
        return Err(CostError::NotPositive {
            quantity: "lots",
            value: lots,
        });
    }
    lots.checked_mul(lot_size.as_i64())
        .ok_or(CostError::Overflow {
            operation: "lots x lot size",
        })
}

/// What `quantity` units are worth at `unit_price`.
///
/// One multiplication. The result is paisa, like every other money value in
/// this repository.
///
/// # Errors
///
/// * [`CostError::NotPositive`] when `quantity` or `unit_price` is not strictly
///   positive. A zero premium is not a quotable price — the option tick is five
///   paisa ([`crate::rate::TICK`]), so the smallest quotable premium is five
///   paisa, and zero is refused rather than priced.
/// * [`CostError::Overflow`] when the product leaves `i64`.
///
/// # Examples
///
/// ```
/// use brutex_core::price::Paisa;
/// use costs::lot::notional;
///
/// // A premium of 125.50 on 75 units is 9,412.50.
/// assert_eq!(notional(Paisa::from_raw(125_50), 75)?.raw(), 9_412_50);
/// # Ok::<(), costs::error::CostError>(())
/// ```
pub fn notional(unit_price: Paisa, quantity: i64) -> Result<Paisa, CostError> {
    if unit_price.raw() <= 0 {
        return Err(CostError::NotPositive {
            quantity: "unit price",
            value: unit_price.raw(),
        });
    }
    if quantity <= 0 {
        return Err(CostError::NotPositive {
            quantity: "quantity",
            value: quantity,
        });
    }
    unit_price
        .raw()
        .checked_mul(quantity)
        .map(Paisa::from_raw)
        .ok_or(CostError::Overflow {
            operation: "unit price x quantity",
        })
}

// ===========================================================================
// THE LOT TABLE AS STORE DATA (R3-30; the store side of DB-G76).
//
// The tables above are compiled: a lot revision the exchange publishes needs
// a code change and a rebuild, and no other underlying can have a lot at all.
// [`LotTable`] is the same dated history as DATA: `pull::reference_data`
// reads the store's `lot_sizes` table (source, checksum and date on the file
// and on every row) and hands its rows here, so this crate names `core`
// alone and never `pull` (CLAUDE.md section 5). A row added to that table
// changes the next lookup with no code change.
//
// THE RULE EVERY ROW IS HELD TO (CLAUDE.md section 3 rule 1): a row is
// APPLIED only when it is `sourced` -- its source is a document recorded in
// `docs/00-charter.md`, with the sha256 of the bytes read and the day they
// were read. Any other row is listed, so a reader sees it, and ABSENT for a
// lookup: the answer names the row and why it does not apply. An underlying
// or a day no row covers is a named absence too. Nothing here infers a lot
// (the vendor's `ltq` GCD and its `inferred` label are composed downstream,
// R3-10-OPT) and nothing stretches a row backwards: before an underlying's
// first row the answer is [`LotAbsence::BeforeFirstRow`], which for NIFTY and
// BANKNIFTY carries the compiled `LOT_GAP` text unchanged.
// ===========================================================================

/// The most rows one underlying may hold in a [`LotTable`]: a lookup is one
/// hash probe and a binary search of at most this many rows (at most seven
/// compares), so its cost does not grow with the table.
pub const MAX_LOT_ROWS_PER_UNDERLYING: usize = 64;

/// The most rows a [`LotTable`] may hold.
pub const MAX_LOT_ROWS: usize = 100_000;

/// The longest underlying symbol, citation or digest a row may carry, in
/// bytes.
pub const MAX_LOT_TEXT_BYTES: usize = 2_048;

/// The sha256 of `NSE/FAOP/64625` as read, recorded in `docs/00-charter.md`
/// ("NSE F&O circulars read for the options lineage", D-3357).
pub const FAOP64625_SHA256: &str =
    "ee1169239eed9cc06519537c6af53751e031903fe709b2bf296283412dc028f8";

/// The sha256 of `NSE/FAOP/70616` as read, recorded in `docs/00-charter.md`
/// ("NSE F&O circulars read for the options lineage", D-3357).
pub const FAOP70616_SHA256: &str =
    "0718cf0cc7e6c74105dc946497369270fca807f59912586560c9552cad8050ef";

/// The day both circulars were read, as the charter records it.
const CIRCULARS_READ: TradeDay = boundary!(2026 - 10 - 10);

/// The citation of a compiled row with no circular recorded in the charter.
const NOT_IN_CHARTER: &str = "UNVERIFIED -- no circular fixing this revision is recorded in \
     docs/00-charter.md (its section 'NSE F&O circulars read for the options lineage' says so \
     for July 2021, June 2023, April 2024 and July 2025, and the 2021-01-01 row is a value the \
     source extended back to its own floor). Listed, never applied.";

/// What a lot row rests on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LotStanding {
    /// Its source is recorded in `docs/00-charter.md`, with the digest of the
    /// bytes read and the day: the row is applied.
    Sourced,
    /// Listed so a reader sees it; never applied.
    Unverified,
}

impl LotStanding {
    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Sourced => "sourced",
            Self::Unverified => "unverified",
        }
    }

    /// The standing a word names, or `None`.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        match word {
            "sourced" => Some(Self::Sourced),
            "unverified" => Some(Self::Unverified),
            _ => None,
        }
    }
}

/// One dated lot row: from `effective` until the underlying's next row,
/// contracts introduced on a day carry `units` per lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotRow {
    /// The underlying the row is for, as its derivatives name it.
    pub underlying: String,
    /// The first trade day the row governs.
    pub effective: TradeDay,
    /// Units per lot; strictly positive.
    pub units: u32,
    /// What the row rests on.
    pub standing: LotStanding,
    /// The citation: the document and what it says.
    pub source: String,
    /// The sha256 of the source bytes as read; required for a sourced row.
    pub source_sha256: Option<String>,
    /// The day the source was read; required for a sourced row.
    pub read: Option<TradeDay>,
}

/// Why a [`LotTable`] was refused, naming the row (one-based, in the order
/// handed in).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LotTableError {
    /// More rows than [`MAX_LOT_ROWS`].
    TooManyRows {
        /// How many were handed in.
        rows: usize,
    },
    /// An underlying, a citation or a digest is empty, padded or longer than
    /// [`MAX_LOT_TEXT_BYTES`].
    Text {
        /// The row.
        row: usize,
        /// The column.
        column: &'static str,
    },
    /// A lot of zero units.
    ZeroLot {
        /// The row.
        row: usize,
    },
    /// A sourced row without the digest of its source or the day it was read,
    /// or a digest that is not 64 lower-case hex.
    SourcedWithoutReceipt {
        /// The row.
        row: usize,
    },
    /// The underlying's rows do not strictly ascend by effective day.
    NotAscending {
        /// The row.
        row: usize,
    },
    /// More rows for one underlying than [`MAX_LOT_ROWS_PER_UNDERLYING`].
    TooManyForUnderlying {
        /// The row.
        row: usize,
    },
}

impl core::fmt::Display for LotTableError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooManyRows { rows } => {
                write!(
                    f,
                    "the lot table holds {rows} rows, more than {MAX_LOT_ROWS}"
                )
            }
            Self::Text { row, column } => write!(
                f,
                "lot row {row}: {column} is empty, padded or longer than {MAX_LOT_TEXT_BYTES} bytes"
            ),
            Self::ZeroLot { row } => write!(f, "lot row {row}: a lot of zero units"),
            Self::SourcedWithoutReceipt { row } => write!(
                f,
                "lot row {row}: a sourced row needs the sha256 of its source and the day it was read"
            ),
            Self::NotAscending { row } => write!(
                f,
                "lot row {row}: the underlying's rows do not strictly ascend by effective day"
            ),
            Self::TooManyForUnderlying { row } => write!(
                f,
                "lot row {row}: more than {MAX_LOT_ROWS_PER_UNDERLYING} rows for one underlying"
            ),
        }
    }
}

impl core::error::Error for LotTableError {}

/// Why a lookup has no lot: a NAMED absence, never a guessed, inferred or
/// stretched lot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LotAbsence {
    /// The table holds no row for the underlying.
    NoRow {
        /// The underlying asked.
        underlying: String,
    },
    /// The day is before the underlying's first row. `gap` is the compiled
    /// `LOT_GAP` text for NIFTY and BANKNIFTY, and a generic statement for
    /// every other underlying.
    BeforeFirstRow {
        /// The underlying asked.
        underlying: String,
        /// The day asked.
        day: TradeDay,
        /// The underlying's first row.
        first: TradeDay,
        /// Why nothing earlier is known.
        gap: &'static str,
    },
    /// The row in force is UNVERIFIED: listed, never applied.
    Unverified {
        /// The underlying asked.
        underlying: String,
        /// The day asked.
        day: TradeDay,
        /// The row's effective day.
        effective: TradeDay,
        /// The row's units, shown and not applied.
        units: u32,
        /// The row's citation.
        source: String,
    },
}

impl LotAbsence {
    /// The wire word: `no_row`, `before_first_row` or `unverified`.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::NoRow { .. } => "no_row",
            Self::BeforeFirstRow { .. } => "before_first_row",
            Self::Unverified { .. } => "unverified",
        }
    }
}

impl core::fmt::Display for LotAbsence {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoRow { underlying } => write!(
                f,
                "no sourced lot: the lot table holds no row for {underlying}; no lot is inferred"
            ),
            Self::BeforeFirstRow {
                underlying,
                day,
                first,
                gap,
            } => write!(
                f,
                "no sourced lot for {underlying} on {day}: its first row is {first}; {gap}"
            ),
            Self::Unverified {
                underlying,
                day,
                effective,
                units,
                source,
            } => write!(
                f,
                "no sourced lot for {underlying} on {day}: the row of {effective} ({units} units) \
                 is unverified and is not applied: {source}"
            ),
        }
    }
}

impl core::error::Error for LotAbsence {}

/// A lot the table answers, and the row that gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LotAnswer<'a> {
    /// The lot.
    pub lot: LotSize,
    /// The sourced row in force.
    pub row: &'a LotRow,
}

/// The dated lot history of every underlying the store's table holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LotTable {
    rows: Vec<LotRow>,
    /// Per underlying, the range of `rows` holding its rows, ascending.
    by_underlying: std::collections::HashMap<String, core::ops::Range<usize>>,
}

/// Text a row may carry: not empty, not padded, no line break, bounded.
fn lot_text_ok(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= MAX_LOT_TEXT_BYTES
        && text.trim() == text
        && !text.contains(['\n', '\r'])
}

fn is_sha256_hex(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

impl LotTable {
    /// Builds the table from `rows`, in any underlying order; each
    /// underlying's own rows must strictly ascend. O(rows), once.
    ///
    /// # Errors
    ///
    /// Every [`LotTableError`], naming the row. Nothing is repaired.
    pub fn from_rows(rows: Vec<LotRow>) -> Result<Self, LotTableError> {
        if rows.len() > MAX_LOT_ROWS {
            return Err(LotTableError::TooManyRows { rows: rows.len() });
        }
        for (at, row) in rows.iter().enumerate() {
            let number = at + 1;
            if !lot_text_ok(&row.underlying) {
                return Err(LotTableError::Text {
                    row: number,
                    column: "underlying",
                });
            }
            if !lot_text_ok(&row.source) {
                return Err(LotTableError::Text {
                    row: number,
                    column: "source",
                });
            }
            if row.units == 0 {
                return Err(LotTableError::ZeroLot { row: number });
            }
            let receipt_ok =
                row.source_sha256.as_deref().is_some_and(is_sha256_hex) && row.read.is_some();
            if row.standing == LotStanding::Sourced && !receipt_ok {
                return Err(LotTableError::SourcedWithoutReceipt { row: number });
            }
        }
        // Stable by underlying, so each underlying's rows keep the order they
        // were handed in, which must already ascend.
        let mut order: Vec<usize> = (0..rows.len()).collect();
        order.sort_by(|a, b| {
            let (left, right) = (rows.get(*a), rows.get(*b));
            left.map(|r| r.underlying.as_str())
                .cmp(&right.map(|r| r.underlying.as_str()))
                .then(a.cmp(b))
        });
        let mut slots: Vec<Option<LotRow>> = rows.into_iter().map(Some).collect();
        let mut sorted = Vec::with_capacity(slots.len());
        let mut by_underlying: std::collections::HashMap<String, core::ops::Range<usize>> =
            std::collections::HashMap::new();
        for at in order {
            let Some(row) = slots.get_mut(at).and_then(Option::take) else {
                continue;
            };
            let number = at + 1;
            let range = by_underlying
                .entry(row.underlying.clone())
                .or_insert(sorted.len()..sorted.len());
            if range.end > range.start
                && sorted
                    .last()
                    .is_some_and(|previous: &LotRow| !previous.effective.before(row.effective))
            {
                return Err(LotTableError::NotAscending { row: number });
            }
            if range.len() >= MAX_LOT_ROWS_PER_UNDERLYING {
                return Err(LotTableError::TooManyForUnderlying { row: number });
            }
            range.end += 1;
            sorted.push(row);
        }
        Ok(Self {
            rows: sorted,
            by_underlying,
        })
    }

    /// Every row, grouped by underlying, each underlying's rows ascending.
    #[must_use]
    pub fn rows(&self) -> &[LotRow] {
        &self.rows
    }

    /// The rows of `underlying`, ascending; empty when it has none.
    #[must_use]
    pub fn rows_of(&self, underlying: &str) -> &[LotRow] {
        self.by_underlying
            .get(underlying)
            .and_then(|range| self.rows.get(range.clone()))
            .unwrap_or(&[])
    }

    /// How many underlyings the table names.
    #[must_use]
    pub fn underlyings(&self) -> usize {
        self.by_underlying.len()
    }

    /// The lot of contracts introduced on `day` for `underlying`: the
    /// underlying's latest row on or before `day`, when that row is sourced.
    ///
    /// # Errors
    ///
    /// [`LotAbsence`], by name: no row for the underlying, a day before its
    /// first row, or an unverified row in force. There is no default lot.
    ///
    /// # Cost
    ///
    /// One hash probe and one binary search of at most
    /// [`MAX_LOT_ROWS_PER_UNDERLYING`] rows: bounded, whatever the table
    /// holds. Measured by `costs::lot::table_tests::a_lookup_is_flat_in_the_table`.
    pub fn lot_on(&self, underlying: &str, day: TradeDay) -> Result<LotAnswer<'_>, LotAbsence> {
        let rows = self.rows_of(underlying);
        let Some(first) = rows.first() else {
            return Err(LotAbsence::NoRow {
                underlying: underlying.to_owned(),
            });
        };
        let after = rows.partition_point(|row| !day.before(row.effective));
        let Some(row) = after.checked_sub(1).and_then(|at| rows.get(at)) else {
            return Err(LotAbsence::BeforeFirstRow {
                underlying: underlying.to_owned(),
                day,
                first: first.effective,
                gap: if InstrumentKey::SWEPT
                    .iter()
                    .any(|(_, swept)| *swept == underlying)
                {
                    LOT_GAP
                } else {
                    "no sourced row fixes the lot before it, and none is stretched backwards"
                },
            });
        };
        if row.standing != LotStanding::Sourced {
            return Err(LotAbsence::Unverified {
                underlying: underlying.to_owned(),
                day,
                effective: row.effective,
                units: row.units,
                source: row.source.clone(),
            });
        }
        Ok(LotAnswer {
            lot: LotSize::new_const(row.units),
            row,
        })
    }
}

/// The NIFTY and BANKNIFTY rows of the compiled [`LOT_SIZES`], UNCHANGED in
/// day, units and citation, as table rows: what `pull::reference_data` seeds
/// the store's `lot_sizes` table with. A row is `sourced` exactly when a
/// sourced contract rule of [`CONTRACT_RULES`] turns at its day (its circular
/// is recorded in `docs/00-charter.md` with the digest of the bytes read);
/// every other row is `unverified` and carries [`NOT_IN_CHARTER`] beside its
/// own citation. The compiled `LOT_GAP` anchor is not a row: it is the
/// [`LotAbsence::BeforeFirstRow`] answer before the first one.
#[must_use]
pub fn compiled_rows() -> Vec<LotRow> {
    let mut out = Vec::new();
    for (slot, table) in LOT_SIZES.iter().enumerate() {
        let Some((_, underlying)) = InstrumentKey::SWEPT.get(slot) else {
            continue;
        };
        let rules = CONTRACT_RULES.get(slot);
        for row in std::iter::once(&table.anchor).chain(table.later.iter().flatten()) {
            let Dated::Verified(lot) = row.value else {
                continue;
            };
            let rule = rules.and_then(|rules| rules.iter().find(|rule| rule.boundary == row.start));
            let (standing, source, sha, read) = match rule {
                Some(rule) => (
                    LotStanding::Sourced,
                    format!("{} Contract rule: {}", row.source, rule.citation),
                    Some(
                        if rule.citation == FAOP64625 {
                            FAOP64625_SHA256
                        } else {
                            FAOP70616_SHA256
                        }
                        .to_owned(),
                    ),
                    Some(CIRCULARS_READ),
                ),
                None => (
                    LotStanding::Unverified,
                    format!("{} {NOT_IN_CHARTER}", row.source),
                    None,
                    None,
                ),
            };
            out.push(LotRow {
                underlying: (*underlying).to_owned(),
                effective: row.start,
                units: lot.units(),
                standing,
                source: source.split_whitespace().collect::<Vec<_>>().join(" "),
                source_sha256: sha,
                read,
            });
        }
    }
    out
}

#[cfg(test)]
#[path = "lot_table_tests.rs"]
mod table_tests;

#[cfg(test)]
// A money literal is written `rupees_paisa` — `24_012_00` reads as the twelve
// rupees over twenty-four thousand a circular or a screen would show, where
// `2_401_200` reads as nothing at all. The grouping is deliberate.
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::inconsistent_digit_grouping
)]
mod tests {
    use super::*;

    use brutex_core::symbol::Symbol;

    use crate::dated::Dated;
    use crate::venue::swept_slot;

    fn day(year: u16, month: u8, d: u8) -> TradeDay {
        TradeDay::new(year, month, d).expect("a real date")
    }

    fn slot(text: &str) -> SweptSlot {
        swept_slot(Symbol::new(text).expect("a valid symbol")).expect("a swept underlying")
    }

    #[test]
    fn every_transition_and_the_day_before_it_are_the_source_figures() {
        // Both tables, every boundary, on the day and the day before. The
        // boundary is INCLUSIVE: the transition date is the first day of the
        // new lot, not the last day of the old one.
        // (underlying, transition date, lot on the day, lot the day before —
        // zero meaning the day before is the pre-history refusal).
        type LotPin = (&'static str, (u16, u8, u8), u32, u32);
        let pins: &[LotPin] = &[
            ("NIFTY", (2021, 1, 1), 75, 0), // 0 = a refusal on the day before
            ("NIFTY", (2021, 7, 29), 50, 75),
            ("NIFTY", (2024, 4, 26), 25, 50),
            ("NIFTY", (2024, 11, 20), 75, 25),
            ("NIFTY", (2026, 1, 1), 65, 75),
            ("BANKNIFTY", (2021, 1, 1), 25, 0),
            ("BANKNIFTY", (2023, 6, 30), 15, 25),
            ("BANKNIFTY", (2024, 11, 20), 30, 15),
            ("BANKNIFTY", (2025, 7, 1), 35, 30),
            ("BANKNIFTY", (2025, 12, 30), 30, 35),
        ];
        for &(underlying, (y, m, d), on, before) in pins {
            let subject = slot(underlying);
            let boundary = day(y, m, d);
            assert_eq!(
                lot_size_on(subject, boundary).map(LotSize::units),
                Ok(on),
                "{underlying} on {boundary}"
            );
            let yesterday = boundary.plus_days(-1).expect("inside the window");
            if before == 0 {
                let refusal = lot_size_on(subject, yesterday)
                    .expect_err("the day before recorded history refuses");
                assert_eq!(refusal.verified_from(), Some(boundary));
            } else {
                assert_eq!(
                    lot_size_on(subject, yesterday).map(LotSize::units),
                    Ok(before),
                    "{underlying} on {yesterday}"
                );
            }
        }
    }

    #[test]
    fn the_two_underlyings_carry_different_lots_on_the_same_day() {
        // A single hardcoded lot would size one of the two wrongly on every
        // trade. On 2024-06-01 the gap is 25 against 15 — a 67% error.
        let on = day(2024, 6, 1);
        assert_eq!(lot_size_on(slot("NIFTY"), on).map(LotSize::units), Ok(25));
        assert_eq!(
            lot_size_on(slot("BANKNIFTY"), on).map(LotSize::units),
            Ok(15)
        );
        // And on 2025-08-01, 75 against 35.
        let later = day(2025, 8, 1);
        assert_eq!(
            lot_size_on(slot("NIFTY"), later).map(LotSize::units),
            Ok(75)
        );
        assert_eq!(
            lot_size_on(slot("BANKNIFTY"), later).map(LotSize::units),
            Ok(35)
        );
    }

    #[test]
    fn the_pre_history_window_refuses_on_every_day_of_it_and_never_after() {
        let mut refused = 0u32;
        let mut priced = 0u32;
        for underlying in ["NIFTY", "BANKNIFTY"] {
            let subject = slot(underlying);
            for today in crate::day::every_representable_day() {
                match lot_size_on(subject, today) {
                    Ok(lot) => {
                        assert!(!today.before(LOT_VERIFIED_FROM), "{today} is pre-history");
                        assert!(lot.units() > 0, "a lot of zero is not a contract");
                        priced += 1;
                    }
                    Err(refusal) => {
                        assert!(today.before(LOT_VERIFIED_FROM), "{today} has a row");
                        assert_eq!(refusal.window_start(), TradeDay::MIN);
                        assert_eq!(refusal.verified_from(), Some(LOT_VERIFIED_FROM));
                        assert_eq!(refusal.remediation(), LOT_REMEDIATION);
                        assert!(refusal.to_string().contains(underlying));
                        refused += 1;
                    }
                }
            }
        }
        // 1990-01-01 .. 2020-12-31 is 11,323 days, on each of two underlyings.
        assert_eq!(refused, 2 * 11_323);
        assert_eq!(priced, 2 * (40_542 - 11_323));
        assert_eq!(LOT_VERIFIED_FROM, day(2021, 1, 1));
    }

    #[test]
    fn every_shipped_lot_row_is_positive_and_keyed_on_cores_order() {
        assert_eq!(LOT_SIZES.len(), SweptSlot::COUNT);
        let mut verified_rows = 0u32;
        for (index, table) in LOT_SIZES.iter().enumerate() {
            assert!(table.is_shipping_shape(), "table {index}");
            assert!(
                table.subject.contains(InstrumentKey::SWEPT[index].1),
                "table {index} names the wrong underlying: {}",
                table.subject
            );
            assert_eq!(table.rows().count(), 6, "one refusal anchor and five rows");
            for row in table.rows() {
                match row.value {
                    Dated::Verified(lot) => {
                        assert!(lot.units() > 0, "a lot of zero is not a contract");
                        assert!(lot.units() <= 100, "no shipped lot is that large");
                        verified_rows += 1;
                    }
                    Dated::Unverified => assert!(row.start.same_day(TradeDay::MIN)),
                }
            }
        }
        assert_eq!(verified_rows, 10);
    }

    #[test]
    fn the_quantity_is_the_product_and_nothing_else() {
        let lot = LotSize::new_const(75);
        assert_eq!(contract_quantity(1, lot), Ok(75));
        assert_eq!(contract_quantity(4, lot), Ok(300));
        assert_eq!(contract_quantity(1_000, lot), Ok(75_000));
        // Every lot count from 1 to 1,000 is exactly lots x units — the
        // statement that this does not loop, accumulate or round.
        for lots in 1i64..=1_000 {
            assert_eq!(contract_quantity(lots, lot), Ok(lots * 75));
        }
        // And it scales with the lot, which is the whole reason the lot is
        // dated: the same order is 25 units in June 2024 and 75 in December.
        let june = lot_size_on(slot("NIFTY"), day(2024, 6, 1)).expect("priced");
        let december = lot_size_on(slot("NIFTY"), day(2024, 12, 1)).expect("priced");
        assert_eq!(contract_quantity(2, june), Ok(50));
        assert_eq!(contract_quantity(2, december), Ok(150));
    }

    #[test]
    fn a_lot_count_that_is_not_a_trade_is_refused_by_name() {
        let lot = LotSize::new_const(75);
        for lots in [0i64, -1, -75, i64::MIN] {
            assert_eq!(
                contract_quantity(lots, lot),
                Err(CostError::NotPositive {
                    quantity: "lots",
                    value: lots
                }),
                "{lots} lots is not a trade"
            );
        }
    }

    #[test]
    fn a_quantity_past_i64_is_refused_rather_than_wrapped() {
        // The overflow the operator asked for by name. A wrapped quantity is a
        // negative position; a saturated one is a size nobody ordered.
        let lot = LotSize::new_const(75);
        assert_eq!(
            contract_quantity(i64::MAX, lot),
            Err(CostError::Overflow {
                operation: "lots x lot size"
            })
        );
        assert_eq!(
            contract_quantity(i64::MAX / 75 + 1, lot),
            Err(CostError::Overflow {
                operation: "lots x lot size"
            })
        );
        // One lot below the edge still answers, which proves the refusal is the
        // boundary and not the neighbourhood.
        assert_eq!(
            contract_quantity(i64::MAX / 75, lot),
            Ok((i64::MAX / 75) * 75)
        );
        // A lot of one can never overflow, whatever the lot count.
        assert_eq!(
            contract_quantity(i64::MAX, LotSize::new_const(1)),
            Ok(i64::MAX)
        );
    }

    #[test]
    fn the_notional_is_the_product_of_price_and_quantity() {
        assert_eq!(
            notional(Paisa::from_raw(125_50), 75),
            Ok(Paisa::from_raw(9_412_50))
        );
        assert_eq!(notional(Paisa::from_raw(5), 1), Ok(Paisa::from_raw(5)));
        // The tick is five paisa, so the smallest quotable premium on the
        // largest shipped lot is a real, small number.
        assert_eq!(
            notional(crate::rate::TICK, LotSize::new_const(75).as_i64()),
            Ok(Paisa::from_raw(375))
        );
        // A whole round trip's worth of arithmetic, composed: lots -> quantity
        // -> notional, with the lot read off the dated table.
        let lot = lot_size_on(slot("BANKNIFTY"), day(2025, 8, 1)).expect("priced");
        let quantity = contract_quantity(3, lot).expect("a real trade");
        assert_eq!(quantity, 105, "three lots of 35");
        assert_eq!(
            notional(Paisa::from_raw(200_00), quantity),
            Ok(Paisa::from_raw(21_000_00))
        );
    }

    #[test]
    fn a_non_positive_price_or_quantity_is_refused_by_name() {
        for bad in [0i64, -1, i64::MIN] {
            assert_eq!(
                notional(Paisa::from_raw(bad), 75),
                Err(CostError::NotPositive {
                    quantity: "unit price",
                    value: bad
                })
            );
            assert_eq!(
                notional(Paisa::from_raw(100), bad),
                Err(CostError::NotPositive {
                    quantity: "quantity",
                    value: bad
                })
            );
        }
    }

    #[test]
    fn a_notional_past_i64_is_refused_rather_than_wrapped() {
        assert_eq!(
            notional(Paisa::from_raw(i64::MAX), 2),
            Err(CostError::Overflow {
                operation: "unit price x quantity"
            })
        );
        assert_eq!(
            notional(Paisa::from_raw(2), i64::MAX),
            Err(CostError::Overflow {
                operation: "unit price x quantity"
            })
        );
        assert_eq!(
            notional(Paisa::from_raw(1), i64::MAX),
            Ok(Paisa::from_raw(i64::MAX))
        );
    }

    #[test]
    fn a_lot_size_reads_renders_and_hashes_as_the_number_it_is() {
        use std::collections::HashSet;

        let lot = LotSize::new_const(75);
        assert_eq!(lot.units(), 75);
        assert_eq!(lot.as_i64(), 75);
        assert_eq!(lot.to_string(), "75");
        assert_eq!(format!("{lot:?}"), "LotSize(75)");
        assert!(LotSize::new_const(25) < lot);
        let mut set = HashSet::new();
        assert!(set.insert(lot));
        assert!(!set.insert(LotSize::new_const(75)));
        assert!(set.insert(LotSize::new_const(25)));
        assert_eq!(set.len(), 2);
    }

    /// DB-B04. The lot is the CONTRACT's: on each trade day the audit
    /// measured (LTQ GCD mode per expiry, from the vendor's rows), every
    /// expiry carries the lot the circular's rule gives. `table` is what the
    /// trade-day table answered for all of them before.
    #[test]
    fn a_contracts_lot_follows_the_circulars_rule_while_two_lots_are_in_force() {
        type Row = ((u16, u8, u8), u32);
        type Case = (&'static str, (u16, u8, u8), u32, &'static [Row]);
        let cases: [Case; 6] = [
            // FAOP64625, NIFTY 25 -> 75. Table: 75 (3x on every 25).
            (
                "NIFTY",
                (2024, 12, 10),
                75,
                &[
                    ((2024, 12, 12), 25),
                    ((2024, 12, 19), 25),
                    ((2024, 12, 26), 25),
                    ((2025, 1, 2), 75),
                    ((2025, 1, 9), 75),
                    ((2025, 1, 30), 25),
                    ((2025, 2, 27), 75),
                    ((2025, 3, 27), 25),
                    ((2025, 6, 26), 25),
                    ((2029, 12, 27), 25),
                ],
            ),
            // The existing quarterlies move at 26 Dec 2024 EOD.
            (
                "NIFTY",
                (2024, 12, 27),
                75,
                &[((2025, 1, 30), 25), ((2025, 3, 27), 75), ((2025, 1, 2), 75)],
            ),
            // FAOP64625, BANKNIFTY 15 -> 30, both spellings of a re-dated
            // expiry. Table: 30 (2x on every 15).
            (
                "BANKNIFTY",
                (2024, 12, 10),
                30,
                &[
                    ((2024, 12, 24), 15),
                    ((2024, 12, 26), 15),
                    ((2025, 1, 29), 15),
                    ((2025, 1, 30), 15),
                    ((2025, 2, 26), 30),
                    ((2025, 2, 27), 30),
                    ((2025, 3, 26), 15),
                    ((2025, 3, 27), 15),
                    ((2025, 6, 25), 15),
                ],
            ),
            // FAOP70616, NIFTY 75 -> 65. Table: 75.
            (
                "NIFTY",
                (2025, 11, 14),
                75,
                &[
                    ((2025, 11, 18), 75),
                    ((2025, 12, 30), 75),
                    ((2026, 1, 6), 65),
                    ((2026, 1, 27), 65),
                    ((2026, 3, 30), 75),
                    ((2026, 3, 31), 75),
                    ((2026, 6, 30), 75),
                ],
            ),
            // After 30 Dec 2025 EOD every NIFTY contract is 65; the table
            // said 75 until 2026-01-01.
            (
                "NIFTY",
                (2025, 12, 31),
                75,
                &[((2026, 1, 6), 65), ((2026, 1, 27), 65), ((2026, 3, 30), 65)],
            ),
            // FAOP70616, BANKNIFTY 35 -> 30, on the last day of the old lot.
            // Table: 30.
            (
                "BANKNIFTY",
                (2025, 12, 30),
                30,
                &[
                    ((2025, 12, 30), 35),
                    ((2026, 1, 27), 30),
                    ((2026, 2, 24), 30),
                    ((2026, 3, 30), 35),
                ],
            ),
        ];
        let mut off_table = 0;
        for (subject, (y, m, d), table, rows) in cases {
            let on = day(y, m, d);
            assert_eq!(
                lot_size_on(slot(subject), on).map(LotSize::units),
                Ok(table),
                "{subject} {on:?}: the trade-day table"
            );
            for &((ey, em, ed), want) in rows {
                let got = lot_size_of(slot(subject), day(ey, em, ed), on).expect("priced");
                assert_eq!(got.lot.units(), want, "{subject} {ey}-{em}-{ed} on {on:?}");
                let LotBasis::ContractRule(citation) = got.basis else {
                    panic!("{subject} {on:?}: {:?}", got.basis);
                };
                assert!(citation.starts_with("NSE/FAOP/"), "{citation}");
                off_table += usize::from(want != table);
            }
        }
        assert_eq!(off_table, 22, "contracts the trade-day table mis-sized");
    }

    /// Outside a sourced transition every contract carries the trade-day
    /// lot, whatever its expiry, and the answer says which it is; the first
    /// and last days of each rule are the circular's, and the day after a
    /// rule ends agrees with the table for every contract.
    #[test]
    fn outside_a_transition_the_contract_lot_is_the_trade_day_row_and_says_so() {
        for (subject, (y, m, d)) in [
            ("NIFTY", (2024, 11, 19)),
            ("NIFTY", (2025, 1, 31)),
            ("NIFTY", (2025, 10, 28)),
            ("NIFTY", (2026, 1, 1)),
            ("BANKNIFTY", (2024, 11, 19)),
            ("BANKNIFTY", (2025, 1, 31)),
            ("BANKNIFTY", (2025, 10, 28)),
            ("BANKNIFTY", (2025, 12, 31)),
        ] {
            let on = day(y, m, d);
            let table = lot_size_on(slot(subject), on).expect("priced");
            for ahead in [0, 7, 45, 400, 1_500] {
                let expiry = on.plus_days(ahead).expect("a real date");
                assert_eq!(
                    lot_size_of(slot(subject), expiry, on),
                    Ok(ContractLot {
                        lot: table,
                        basis: LotBasis::TradeDayRow
                    }),
                    "{subject} {on:?} +{ahead}"
                );
            }
        }
        // The first day of each rule is inside it.
        for (subject, (y, m, d)) in [
            ("NIFTY", (2024, 11, 20)),
            ("BANKNIFTY", (2024, 11, 20)),
            ("NIFTY", (2025, 10, 29)),
            ("BANKNIFTY", (2025, 10, 29)),
        ] {
            let on = day(y, m, d);
            let got = lot_size_of(slot(subject), on, on).expect("priced");
            assert!(matches!(got.basis, LotBasis::ContractRule(_)), "{subject}");
        }
        // Before the recorded history there is a refusal, never a guess.
        let early = day(2020, 12, 31);
        assert!(lot_size_of(slot("NIFTY"), early, early).is_err());
        // Every rule is ordered and its lots are the table's on either side.
        for (rules, subject) in CONTRACT_RULES.iter().zip(["NIFTY", "BANKNIFTY"]) {
            for rule in rules {
                assert!(rule.from <= rule.far_switch && rule.far_switch <= rule.through);
                assert!(rule.old_through_expiry < rule.far_from_expiry);
                assert_ne!(rule.old, rule.new);
                // The rule explains one row of the trade-day table: that
                // row's day, with the two lots on either side of it.
                let before = rule.boundary.plus_days(-1).expect("a real date");
                assert_eq!(lot_size_on(slot(subject), before), Ok(rule.old));
                assert_eq!(lot_size_on(slot(subject), rule.boundary), Ok(rule.new));
                assert_eq!(unread_revision(slot(subject), rule.boundary), None);
                let eve = rule.from.plus_days(-1).expect("a real date");
                assert_eq!(lot_size_on(slot(subject), eve), Ok(rule.old), "{subject}");
                let after = rule.through.plus_days(1).expect("a real date");
                assert_eq!(lot_size_on(slot(subject), after), Ok(rule.new), "{subject}");
            }
        }
    }

    /// DB-B04, second pass: the audit measured three contracts off the
    /// trade-day table inside revisions whose circular was not read. The
    /// table's lot was answered for them under the label a verified day
    /// carries; it is a refusal now, naming the window and the remedy.
    #[test]
    fn a_revision_whose_circular_was_not_read_refuses_the_contract_lot() {
        // (underlying, expiry, trade day, the table's lot, the measured lot)
        for (subject, expiry, on, table, measured) in [
            ("NIFTY", (2021, 7, 29), (2021, 7, 15), 75, 50),
            ("BANKNIFTY", (2023, 7, 27), (2023, 6, 15), 25, 15),
            ("BANKNIFTY", (2025, 7, 31), (2025, 6, 27), 30, 35),
        ] {
            let (expiry, on) = (day(expiry.0, expiry.1, expiry.2), day(on.0, on.1, on.2));
            assert_eq!(
                lot_size_on(slot(subject), on).map(LotSize::units),
                Ok(table)
            );
            assert_ne!(table, measured, "the table is wrong for this contract");
            let Err(CostError::Unverified(refusal)) = lot_size_of(slot(subject), expiry, on) else {
                panic!("{subject} {on:?}: an unread revision answered a lot");
            };
            assert_eq!(refusal.on(), on);
            assert!(refusal.to_string().contains("UNVERIFIED"), "{refusal}");
            assert!(refusal.to_string().contains("CONTRACT_RULES"), "{refusal}");
        }
        // Exactly the four revisions with no rule, each refused through the
        // chosen bound on both sides and answered one day past it.
        let unread = [
            ("NIFTY", (2021, 7, 29)),
            ("NIFTY", (2024, 4, 26)),
            ("BANKNIFTY", (2023, 6, 30)),
            ("BANKNIFTY", (2025, 7, 1)),
        ];
        for (subject, (y, m, d)) in unread {
            let row = day(y, m, d);
            for offset in [-UNREAD_REVISION_DAYS, -1, 0, 1, UNREAD_REVISION_DAYS] {
                let on = row.plus_days(offset).expect("a real date");
                assert_eq!(
                    unread_revision(slot(subject), on),
                    Some(row),
                    "{subject} {offset}"
                );
                assert!(
                    matches!(
                        lot_size_of(slot(subject), on, on),
                        Err(CostError::Unverified(_))
                    ),
                    "{subject} {on:?}"
                );
            }
            for offset in [-UNREAD_REVISION_DAYS - 1, UNREAD_REVISION_DAYS + 1] {
                let on = row.plus_days(offset).expect("a real date");
                assert_eq!(
                    unread_revision(slot(subject), on),
                    None,
                    "{subject} {offset}"
                );
                let got = lot_size_of(slot(subject), on, on).expect("outside the window");
                assert_eq!(got.basis, LotBasis::TradeDayRow, "{subject} {on:?}");
                assert_eq!(Ok(got.lot), lot_size_on(slot(subject), on));
            }
        }
        // Every other row of both tables is the first verified row or a
        // rule's: counted, so a new row without a rule is refused by
        // construction and shows up here.
        for (table, subject) in LOT_SIZES.iter().zip(["NIFTY", "BANKNIFTY"]) {
            let refused = table
                .later
                .iter()
                .flatten()
                .filter(|row| unread_revision(slot(subject), row.start) == Some(row.start))
                .count();
            assert_eq!(refused, 2, "{subject}");
        }
        // The first verified row revises nothing: the day it starts is the
        // table's, and the day before it is the pre-history refusal.
        let first = lot_size_of(slot("NIFTY"), LOT_VERIFIED_FROM, LOT_VERIFIED_FROM);
        assert_eq!(first.map(|lot| lot.basis), Ok(LotBasis::TradeDayRow));
    }

    /// A contract has no lot after it expires. Before this the rule in force
    /// on the trade day answered for it: `(NIFTY, expiry 2020-01-01, day
    /// 2024-12-10)` was 25 with a circular's citation.
    #[test]
    fn an_expiry_before_the_trade_day_is_refused_by_name() {
        let on = day(2024, 12, 10);
        for subject in ["NIFTY", "BANKNIFTY"] {
            for expiry in [day(2020, 1, 1), day(2024, 12, 9)] {
                assert_eq!(
                    lot_size_of(slot(subject), expiry, on),
                    Err(CostError::ExpiryBeforeTrade { expiry, day: on }),
                    "{subject} {expiry:?}"
                );
            }
            // The expiry day itself is a trade day of the contract.
            assert!(lot_size_of(slot(subject), on, on).is_ok(), "{subject}");
        }
        // Outside every rule and every unread window too.
        let (expiry, later) = (day(2022, 6, 30), day(2022, 7, 1));
        assert_eq!(
            lot_size_of(slot("NIFTY"), expiry, later),
            Err(CostError::ExpiryBeforeTrade { expiry, day: later })
        );
    }
}
