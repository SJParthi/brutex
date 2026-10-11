//! Stock options across a corporate action, detected from the data: the lot
//! each contract-day shows, the strikes the vendor renamed on the ex-date, and
//! the open interest it left unscaled. D-3331 (lineage), D-3343 (inferred
//! lot). Findings DB-G71, DB-G72, DB-G76, DB-G77.
//!
//! # The defect this closes
//!
//! On a bonus or split ex-date NSE adjusts every open stock-option contract:
//! the strike is divided by the adjustment factor and the lot multiplied by
//! it. GDFL renames each file to the adjusted strike. The store keys a
//! contract by (expiry, strike, side) read off each day's name, so:
//!
//! * the pre-ex history of every adjusted contract stops weeks before its
//!   expiry, and the same contract starts again as an unlinked series under
//!   the new strike (DB-G72: BRITANNIA `6000CE` becomes `3000CE` on
//!   2018-11-29);
//! * where the adjusted strike text equals another contract's pre-ex text, one
//!   store series holds two exchange contracts (DB-G71: NTPC's post-ex
//!   `125PE` is the old `150PE`, and its first bar reads +900%);
//! * on 2025-06-16 the vendor's first BAJFINANCE rows carry the pre-ex OI
//!   unscaled for hours (DB-G77).
//!
//! # What is measured here, and how
//!
//! Nothing about a corporate action is sourced: `docs/00-charter.md` names no
//! corporate-action source. Everything below is read off the vendor's rows,
//! and every output says so.
//!
//! 1. **The lot of a contract-day** ([`LotEvidence`], D-3343) is the GREATEST
//!    COMMON DIVISOR of its traded quantities, leaving out the LTQ = 1 marker
//!    rows that lead a file (D-3335's `ltq1_lead_rows`), and is labelled
//!    [`LotEvidence::LABEL`] (`inferred`) with the basis
//!    [`LotEvidence::BASIS`] (`ltq_gcd`). GDFL option quantities are SHARES,
//!    so this is shares per lot. It is an UPPER bound: every traded quantity
//!    is a whole number of lots, so the true lot divides it, and a day whose
//!    trades were all two lots reads twice the lot.
//! 2. **The lot of a contract over days** ([`LotLedger`]) is recorded when
//!    first seen and when it changes; a later day's quantities can only lower
//!    it, to the common divisor of both (the true lot divides each), as
//!    D-3343 says: "a later, smaller value supersedes it within the same
//!    lineage". A key the vendor reused for another contract at an ex-date
//!    starts a new lineage ([`LotLedger::restart`]).
//! 3. **The lot of an underlying-day** ([`day_lot`]) is the most common
//!    contract-day lot (a contract that only ever traded two lots shows twice
//!    the lot; the mode does not). A tie is no lot.
//! 4. **An adjustment** ([`detect`]) is a change of the underlying-day lot from
//!    one vendor day to the next. In every adjustment the audit measured, the
//!    lot step equals the factor and the new strike is the old one divided by
//!    it on the 0.05 grid, so each pre-ex contract is paired with the post-ex
//!    contract of the same expiry and side at `strike × lot_before /
//!    lot_after`, within [`STRIKE_TOLERANCE_PER_MILLE`] (or one grid step,
//!    whichever is wider), and only when exactly one candidate is that close.
//!    A lot REVISION that leaves strikes alone keeps most pre-ex keys on the
//!    next day; an adjustment renames them. So an event needs more pairs than
//!    continuing keys, which a revision cannot reach.
//! 5. **The OI across the pair** ([`OiCarry`]): the post-ex first OI against
//!    the pre-ex close OI, scaled or not. Unscaled is a vendor fault, and
//!    [`Lineage::stale_oi`] finds the ex-date rows that carry it and the
//!    second the vendor's adjustment lands.
//! 6. **Stitched series** ([`CorporateAction::stitched`],
//!    [`CorporateAction::retired`]): an adjustment renames every open
//!    contract, so a pre-ex key traded again from the ex-date on is another
//!    exchange contract under the same store key.
//!
//! # The records (D-3331 decision 3, D-3343)
//!
//! Each is one line of D-3317's rules (one space between fields, every field
//! after the key `name=value`, no value holding a space), written by the
//! import (WP-11b) to an append-only ledger and read back exactly:
//!
//! * [`LineageRecord`]: `lineage <old_key> <new_key> ex=<YYYY-MM-DD>
//!   factor=<num>/<den> lot_old=<n> lot_new=<n>
//!   oi_unadjusted_until=<unix_s|0> v=1`, in `<store>/imports/gdfl.lineage`;
//! * [`StitchedRecord`]: `stitched <key> ex=<YYYY-MM-DD> before=<contract|->
//!   after=<contract|-> v=1`, in the same ledger;
//! * [`LotRecord`]: `lot <instrument_key> from=<YYYY-MM-DD> lot=<n>
//!   basis=ltq_gcd v=1`, in `<store>/imports/gdfl.lots`;
//! * [`SupersededRecord`] and [`RetiredRecord`], the re-key's (WP-12c,
//!   `crate::gdfl_rekey`): a stored key the current decoder files under
//!   another key, and one it refuses on every day, in `gdfl.lineage`.
//!
//! A key is the census's instrument text, `NSE-FNO-<underlying>-<contract>`
//! ([`option_key`]), the text `/store.json` lists.
//!
//! **Checked against the operator's zips, read-only**, by the ignored test
//! `the_real_ex_dates_are_detected_from_the_data_and_ordinary_days_are_not`
//! (re-run 2026-10-09 on this module's GCD lot): BRITANNIA 2018-11-29 (lot
//! 100 to 200), NTPC 2019-03-19 (4,000 to 4,800), HCLTECH 2019-12-05 (700 to
//! 1,400), BAJAJFINSV 2022-09-13 (50 to 500), RELIANCE 2024-10-28 (250 to
//! 500) and BAJFINANCE 2025-06-16 (125 to 1,250) are each detected with that
//! lot step, and the next session of each is not. The audit's counts are
//! reproduced: 56 BRITANNIA series end early, 35 NTPC keys are stitched, and
//! 18 BAJFINANCE pairs carry the pre-ex OI unscaled. Ex-dates are measured,
//! not sourced.
//!
//! # Events that keep the lot (DB-B02, DB-B03, DB-B13; WP-12e)
//!
//! Step 4 saw only a lot step. NSE also renames every open contract with the
//! lot UNCHANGED, and the detector declared nothing:
//!
//! * a **dividend** the exchange adjusts the contracts for: "The full value
//!   of dividend ... would be deducted from all the cum-dividend strike
//!   prices" (`NSE/FAOP/53079`, 25 Jul 2022, VEDL; `docs/00-charter.md`,
//!   "NSE F&O circulars read for the options lineage"), and the circular
//!   states no lot change. Every strike moves by one constant
//!   ([`EventKind::StrikeOffset`]);
//! * a **scheme of arrangement**: every open contract expires on the
//!   cum-date and the same expiries are listed again from the ex-date under
//!   new strikes (`NSE/FAOP/73857`, VEDL 2026). No pre-ex contract continues
//!   ([`EventKind::Relisted`]).
//!
//! Both are found from the same two facts, read off one pass over the day
//! ([`detect`]'s unchanged-lot arm):
//!
//! 1. **The OI names the partner.** A contract's first OI of a day is, on
//!    about a quarter of ordinary contract-days, exactly its previous close
//!    OI (the audit measured 12 of 42, 15 of 69 and 15 of 56). On an offset
//!    ex-date that equality holds on the SHIFTED key and never on the same
//!    text (VEDL 2024-09-10: 0 of 37 same-text, 13 of 43 at minus 20). So
//!    each of the day's keys votes for the shift to the one pre-ex key of its
//!    expiry and side whose close OI it opens on, a value two pre-ex keys
//!    share votes for nothing, and the shift with the most votes is the
//!    event's when it has at least [`MIN_OI_PAIRS`] votes and more than four
//!    times the same-text carries. Strikes alone cannot name the shift: on a
//!    regular grid every multiple of the step pairs most of a ladder.
//! 2. **The names break.** When NONE of the day's keys was a pre-ex key, at
//!    least [`MIN_EVENT_KEYS`] of them are in expiries the underlying already
//!    held, and those expiries held at least as many pre-ex keys, every open
//!    contract was renamed or re-listed. With [`MIN_OI_FRESH`] OI votes for
//!    one shift it is an offset; with none it is a re-listing, which pairs
//!    nothing: the old generation expired, and no arithmetic relates the two.
//!
//! A re-listing whose new strikes REUSE pre-ex texts shows neither fact (the
//! names continue, and an absent carry is what one ordinary day in ten looks
//! like on a thin underlying), so it is declared only from a SOURCED row
//! ([`crate::expiry_alias::relisted_on`]). HINDUNILVR 2025-12-05 is that
//! case and has no row: NOT DETECTED, named for `docs/06-limits.md`.
//!
//! An offset pair is a [`LineageRecord`] with `factor=1/1` and equal lots;
//! the shift is the difference of the two keys' strikes. The CAUSE is never
//! written: no circular for any measured event is recorded, so "dividend" is
//! UNVERIFIED and the record says only what moved.
//!
//! Every event the import meets is one [`EventRecord`] in the ledger (its
//! kind, shift, lots and counts) and every pre-ex key it retired with no
//! partner one [`StoppedRecord`] (D-3355). A re-listing pairs nothing, so
//! without these two it left no line at all, and the keys it retired were
//! known only to the run that met it.
//!
//! # One contract across a symbol change (DB-B05, DB-B11, DB-B62)
//!
//! The decoder took the underlying from each day's file name, so a contract
//! alive across `NIITTECH -> COFORGE` was two store contracts.
//! [`filed_underlying`] is the option side of D-3333: a file named for a
//! symbol the exchange used that day is filed under the NEWEST symbol its
//! dated changes reach (`brutex_core::alias::LINEAGE`, NSE's
//! `symbolchange.csv`), exactly as the cash side files old stems, and a name
//! the exchange was not using that day is refused. A rename with no row in
//! that table (`TATAGLOBAL -> TATACONSUM`, `KPIT -> BSOFT`) keeps both keys:
//! no successor is guessed.
//!
//! # What it does not decide
//!
//! Whether a stitched series is split, refused or flagged, and how a bar's
//! change is labelled, is the import's and the reader's (WP-11b writes the
//! records; WP-31 reads them into `chg_why` / `oichg_why`). Bars are never
//! altered here.
//!
//! # Cost
//!
//! [`ContractDay::from_rows`] is O(rows), one pass, once per file.
//! [`LotLedger::observe`] is one hash probe and one Euclid (O(log lot)).
//! [`day_lot`] is O(contracts). [`detect`] on an ordinary day (equal lots) is
//! one pass over the underlying's pre-ex keys and one over the day's: two
//! hash probes per contract, O(contracts), no sort; on a lot-step ex-date it
//! is O(contracts · log contracts) for the underlying, once. A pre-ex OI
//! value shared by more than [`OI_BUCKET_CAP`] keys of one ladder is not
//! walked. [`filed_underlying`] is at most `1 + R` probes of a compile-time
//! table (`R` its row count). Measured by the ignored test
//! `the_unchanged_lot_arm_costs_one_pass_on_an_ordinary_day`. [`Lineage::stale_oi`]
//! is O(the key's ex-date rows), once per renamed contract. An import-path
//! cost, not a query: no reader calls any of it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use brutex_core::alias::{LINEAGE, Stale, Ymd};
use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;

use crate::expiry_alias::AliasKind;
use crate::gdfl_nfo::OptionTicker;
use crate::session::{Day, IST_OFFSET_SECS, SECS_PER_DAY};

/// The strike grid every adjusted strike is rounded to: 0.05 rupee, paisa.
pub const STRIKE_GRID_PAISA: i64 = 5;

/// How far, per mille of the predicted strike, a post-ex strike may lie from
/// it and still pair. A lot rounded to a whole share (CONCOR `1250` to `1563`
/// for a factor of 1.25) moves the prediction by 0.3 per mille; the tightest
/// real ladder step is about 5 per mille, so 2 pairs the first and never
/// reaches the next strike.
pub const STRIKE_TOLERANCE_PER_MILLE: i64 = 2;

/// The record version every line here carries (`v=1`).
pub const RECORD_VERSION: &str = "1";

/// The greatest common divisor of two quantities (Euclid). `gcd(0, n)` is
/// `n`, so a fold starts from zero.
const fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

/// The lot one contract-day shows (D-3343). Never sourced: inferred from the
/// traded quantities, which for GDFL options are shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LotEvidence {
    /// No row traded a quantity.
    NoTrade,
    /// Every traded row was an LTQ = 1 marker row leading the file (D-3335,
    /// the 30 May 2019 shape): no quantity a lot can be read off.
    MarkersOnly {
        /// The marker rows.
        rows: u32,
    },
    /// The greatest common divisor of the traded quantities, the leading
    /// LTQ = 1 marker rows left out: shares per lot, inferred, an upper bound.
    Inferred(u64),
}

impl LotEvidence {
    /// The label every inferred lot carries on the wire (D-3343).
    pub const LABEL: &'static str = "inferred";

    /// How the lot was inferred, D-3343's `basis=`.
    pub const BASIS: &'static str = "ltq_gcd";

    /// The lot of the traded quantities `ltqs`, in file order (zeros are rows
    /// that traded nothing; the LTQ = 1 rows before the first larger traded
    /// quantity are markers). O(n), one pass.
    #[must_use]
    pub fn of_quantities(ltqs: &[u64]) -> Self {
        let mut fold = LotFold::default();
        for &ltq in ltqs {
            fold.take(ltq);
        }
        fold.evidence()
    }

    /// The inferred lot, when there is one.
    #[must_use]
    pub const fn lot(self) -> Option<u64> {
        match self {
            Self::Inferred(lot) => Some(lot),
            Self::NoTrade | Self::MarkersOnly { .. } => None,
        }
    }

    /// `shares` as whole lots, only where a lot is inferred and `shares` is a
    /// whole number of them (D-3343: "lots = volume / lot(day)" only where a
    /// lot is known). Never a fraction, never a rounded count.
    #[must_use]
    pub const fn lots(self, shares: u64) -> Option<u64> {
        match self {
            Self::Inferred(lot) if shares.is_multiple_of(lot) => Some(shares / lot),
            _ => None,
        }
    }
}

/// The running fold behind [`LotEvidence`]: one pass, constant state.
#[derive(Debug, Default, Clone, Copy)]
struct LotFold {
    /// The common divisor so far, 0 before the first counted quantity.
    divisor: u64,
    /// Leading LTQ = 1 rows seen before any larger traded quantity.
    markers: u32,
}

impl LotFold {
    /// Takes one row's traded quantity.
    const fn take(&mut self, ltq: u64) {
        if ltq == 0 {
            return;
        }
        if ltq == 1 && self.divisor == 0 {
            self.markers = self.markers.saturating_add(1);
            return;
        }
        self.divisor = gcd(self.divisor, ltq);
    }

    /// What the rows showed.
    const fn evidence(self) -> LotEvidence {
        match (self.divisor, self.markers) {
            (0, 0) => LotEvidence::NoTrade,
            (0, rows) => LotEvidence::MarkersOnly { rows },
            (lot, _) => LotEvidence::Inferred(lot),
        }
    }
}

/// One option contract on one vendor day, as the detector needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractDay {
    /// The settled expiry (D-3331).
    pub expiry: Day,
    /// The strike the day's name states, paisa.
    pub strike: i64,
    /// Call or put.
    pub side: OptionSide,
    /// The lot the rows show.
    pub lot: LotEvidence,
    /// The open interest of the day's earliest row (the first of them in file
    /// order when several share its second).
    pub first_oi: Option<i64>,
    /// The open interest of the day's latest row (bar-close semantics: the
    /// last of them in file order).
    pub close_oi: Option<i64>,
}

impl ContractDay {
    /// The contract-day of `ticker` from its rows as `(second of day, LTQ,
    /// OI)`, in file order. O(rows), one pass.
    #[must_use]
    pub fn from_rows(
        ticker: &OptionTicker,
        rows: impl IntoIterator<Item = (u32, u64, i64)>,
    ) -> Self {
        let mut first: Option<(u32, i64)> = None;
        let mut close: Option<(u32, i64)> = None;
        let mut lot = LotFold::default();
        for (sod, ltq, oi) in rows {
            // Strictly earlier replaces the first; at-or-later replaces the
            // close, so the last row of the latest second wins.
            if first.is_none_or(|(at, _)| sod < at) {
                first = Some((sod, oi));
            }
            if close.is_none_or(|(at, _)| sod >= at) {
                close = Some((sod, oi));
            }
            lot.take(ltq);
        }
        Self {
            expiry: ticker.expiry,
            strike: ticker.strike,
            side: ticker.side,
            lot: lot.evidence(),
            first_oi: first.map(|(_, oi)| oi),
            close_oi: close.map(|(_, oi)| oi),
        }
    }

    /// The store's contract segment.
    #[must_use]
    pub fn contract(&self) -> Option<Contract> {
        let expiry =
            Expiry::new(self.expiry.year(), self.expiry.month(), self.expiry.day()).ok()?;
        Contract::of(Kind::Option {
            expiry,
            strike: Paisa::from_raw(self.strike),
            side: self.side,
        })
    }

    /// The (expiry, strike, side) key.
    const fn key(&self) -> (Day, i64, OptionSide) {
        (self.expiry, self.strike, self.side)
    }
}

/// The lot of an underlying on one day: the most common inferred
/// contract-day lot. `None` when no contract-day has one, or two lots tie.
/// O(contracts).
#[must_use]
pub fn day_lot(contracts: &[ContractDay]) -> Option<u64> {
    let mut counts: HashMap<u64, u32> = HashMap::new();
    for lot in contracts.iter().filter_map(|c| c.lot.lot()) {
        *counts.entry(lot).or_default() += 1;
    }
    let best = counts.values().copied().max()?;
    let mut at_best = counts.iter().filter(|&(_, &n)| n == best);
    match (at_best.next(), at_best.next()) {
        (Some((&lot, _)), None) => Some(lot),
        _ => None,
    }
}

/// How the open interest crossed one renamed contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OiCarry {
    /// The post-ex first OI is the pre-ex close OI times the lot step (within
    /// a quarter): the vendor adjusted it.
    Scaled,
    /// The post-ex first OI is the pre-ex close OI unscaled (within 5% of it
    /// and within an eighth of the gap to the scaled value): the vendor served
    /// it UNADJUSTED, so a change measured across it is not a real change
    /// (DB-G77). Measured on BAJFINANCE's 2025-06-16 x10: 18 of its pairs, 14
    /// at exactly the pre-ex close and 4 within 2.7% of it, the audit's 18.
    /// The gap bound keeps a small factor honest: at NTPC's x1.2 a first OI
    /// 2.8% over the close is not called stale.
    Unadjusted,
    /// The two candidate values are within 5% of each other, so the OI
    /// cannot tell them apart.
    Indistinct,
    /// Neither.
    Unexplained,
    /// A side had no OI, or the pre-ex OI was not positive.
    Missing,
}

impl OiCarry {
    /// The wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Scaled => "scaled",
            Self::Unadjusted => "vendor_oi_unadjusted",
            Self::Indistinct => "indistinct",
            Self::Unexplained => "unexplained",
            Self::Missing => "missing",
        }
    }

    /// The word back.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [
            Self::Scaled,
            Self::Unadjusted,
            Self::Indistinct,
            Self::Unexplained,
            Self::Missing,
        ]
        .into_iter()
        .find(|carry| carry.as_str() == text)
    }

    /// The carry of `pre` (pre-ex close OI) to `post` (post-ex first OI) when
    /// the lot moved from `lot_before` to `lot_after`. Integer arithmetic in
    /// i128, so no product can overflow.
    #[must_use]
    pub fn of(pre: Option<i64>, post: Option<i64>, lot_before: u64, lot_after: u64) -> Self {
        let (Some(pre), Some(post)) = (pre, post) else {
            return Self::Missing;
        };
        if pre <= 0 || lot_before == 0 {
            return Self::Missing;
        }
        let (pre, post) = (i128::from(pre), i128::from(post));
        let (before, after) = (i128::from(lot_before), i128::from(lot_after));
        // Everything in units of pre × lot_before, so the scaled value is an
        // integer: raw = pre·b, scaled = pre·a, observed = post·b.
        let raw = pre * before;
        let scaled = pre * after;
        let observed = post * before;
        let gap = (scaled - raw).abs();
        if gap * 20 < raw {
            return Self::Indistinct;
        }
        let off_raw = (observed - raw).abs();
        let off_scaled = (observed - scaled).abs();
        if off_raw < off_scaled && off_raw * 20 <= raw && off_raw * 8 <= gap {
            Self::Unadjusted
        } else if off_scaled <= off_raw && off_scaled * 4 <= scaled {
            Self::Scaled
        } else {
            Self::Unexplained
        }
    }
}

/// One exchange contract renamed on an ex-date: old key to new key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lineage {
    /// The underlying.
    pub underlying: Symbol,
    /// The ex-date: the first vendor day under the new key.
    pub ex_date: Day,
    /// The pre-ex contract segment.
    pub old: Contract,
    /// The post-ex contract segment the same exchange contract continues as.
    pub new: Contract,
    /// The underlying's inferred lot before, shares.
    pub lot_before: u64,
    /// The underlying's inferred lot from the ex-date, shares.
    pub lot_after: u64,
    /// The pre-ex contract's closing OI, the value an unadjusted ex-date OI
    /// repeats.
    pub pre_close_oi: Option<i64>,
    /// How the OI crossed.
    pub oi: OiCarry,
    /// The new key also named a DIFFERENT contract before the ex-date, so one
    /// store series under it holds two exchange contracts (DB-G71).
    pub reused: bool,
}

/// The ex-date rows of a renamed contract whose OI the vendor had not yet
/// adjusted (DB-G77), and the second its adjustment lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaleOi {
    /// Rows, anywhere in the file, whose OI is the pre-ex close unscaled
    /// ([`OiCarry::Unadjusted`] against it).
    pub rows: usize,
    /// The second of day up to which a bar's OI is the vendor's unadjusted
    /// value or the jump off it: the first adjusted row after the last stale
    /// one, or the last stale row when no adjusted row follows it. `None`
    /// when no row is stale.
    pub until_sod: Option<u32>,
}

impl Lineage {
    /// The adjustment factor `lot_after / lot_before`, reduced (D-3331's
    /// `factor=<num>/<den>`): NTPC's 4,000 to 4,800 is `6/5`. A zero lot,
    /// which [`day_lot`] never yields, is `0/1` rather than a division.
    #[must_use]
    pub const fn factor(&self) -> (u64, u64) {
        reduced(self.lot_after, self.lot_before)
    }

    /// The stale ex-date rows of the new key, from its rows as `(second of
    /// day, OI)` in file order. Only a pair whose [`Self::oi`] is
    /// [`OiCarry::Unadjusted`] has any; every other pair answers none without
    /// reading a row. O(rows), two passes.
    #[must_use]
    pub fn stale_oi(&self, rows: &[(u32, i64)]) -> StaleOi {
        let none = StaleOi {
            rows: 0,
            until_sod: None,
        };
        if self.oi != OiCarry::Unadjusted {
            return none;
        }
        let stale = |oi: i64| {
            OiCarry::of(self.pre_close_oi, Some(oi), self.lot_before, self.lot_after)
                == OiCarry::Unadjusted
        };
        let mut count = 0_usize;
        let mut last: Option<u32> = None;
        for &(sod, oi) in rows {
            if stale(oi) {
                count += 1;
                last = Some(last.map_or(sod, |at| at.max(sod)));
            }
        }
        let Some(last) = last else {
            return none;
        };
        let landed = rows
            .iter()
            .filter(|&&(sod, oi)| sod > last && !stale(oi))
            .map(|&(sod, _)| sod)
            .min();
        StaleOi {
            rows: count,
            until_sod: Some(landed.unwrap_or(last)),
        }
    }

    /// D-3331's record of this pair, with `oi_unadjusted_until` read off the
    /// new key's ex-date rows ([`Self::stale_oi`]; pass none for a pair whose
    /// OI carried scaled).
    #[must_use]
    pub fn record(&self, ex_rows: &[(u32, i64)]) -> LineageRecord {
        let until = self
            .stale_oi(ex_rows)
            .until_sod
            .map_or(0, |sod| unix_second(self.ex_date, sod));
        LineageRecord {
            underlying: self.underlying,
            old: self.old,
            new: self.new,
            ex_date: self.ex_date,
            factor: self.factor(),
            lot_old: self.lot_before,
            lot_new: self.lot_after,
            oi_unadjusted_until: until,
        }
    }
}

/// `num / den` in lowest terms; `0/1` when either is zero.
const fn reduced(num: u64, den: u64) -> (u64, u64) {
    if num == 0 || den == 0 {
        return (0, 1);
    }
    let g = gcd(num, den);
    (num / g, den / g)
}

/// The UTC unix second of `sod` seconds into the IST day `day`.
fn unix_second(day: Day, sod: u32) -> i64 {
    i64::from(day.days_from_epoch()) * SECS_PER_DAY + i64::from(sod) - IST_OFFSET_SECS
}

/// The lineage ledger D-3331 names, `<store>/imports/gdfl.lineage`: the
/// `alias`, `lineage` and `stitched` records, append-only, beside the import
/// journal.
#[must_use]
pub fn lineage_path(store_root: &Path) -> PathBuf {
    store_root.join("imports").join("gdfl.lineage")
}

/// The inferred-lot ledger D-3343 names, `<store>/imports/gdfl.lots`.
#[must_use]
pub fn lots_path(store_root: &Path) -> PathBuf {
    store_root.join("imports").join("gdfl.lots")
}

/// The census's instrument text of `underlying`'s option `contract`:
/// `NSE-FNO-<underlying>-<contract>`, the key `/store.json` lists and every
/// record here names.
#[must_use]
pub fn option_key(underlying: Symbol, contract: Contract) -> String {
    format!("NSE-FNO-{}-{}", underlying.as_str(), contract.as_str())
}

/// [`option_key`] read back, or `None` for any other text. The contract is
/// the last five dash-separated fields (`YYYY-MM-DD-<paisa>-<CE|PE>`), so an
/// underlying holding a dash (`BAJAJ-AUTO`) reads back whole.
#[must_use]
pub fn parse_option_key(text: &str) -> Option<(Symbol, Contract)> {
    let rest = text.strip_prefix("NSE-FNO-")?;
    let mut fields = rest.rsplitn(6, '-');
    let side = fields.next()?;
    let strike = fields.next()?;
    let dd = fields.next()?;
    let mm = fields.next()?;
    let yyyy = fields.next()?;
    let under = fields.next()?;
    let (expiry, strike, side) = option_fields(&format!("{yyyy}-{mm}-{dd}"), strike, side)?;
    let contract = ContractDay {
        expiry,
        strike,
        side,
        lot: LotEvidence::NoTrade,
        first_oi: None,
        close_oi: None,
    }
    .contract()?;
    let underlying = Symbol::new(under)
        .ok()
        .filter(|symbol| symbol.as_str() == under)?;
    (option_key(underlying, contract) == text).then_some((underlying, contract))
}

/// The three fields of an option's contract segment, read: the day, the
/// strike in paisa and the side, or `None` when any is not the store's own
/// spelling.
fn option_fields(day: &str, strike: &str, side: &str) -> Option<(Day, i64, OptionSide)> {
    let side = match side {
        "CE" => OptionSide::Call,
        "PE" => OptionSide::Put,
        _ => return None,
    };
    Some((parse_day(day)?, i64::try_from(digits(strike)?).ok()?, side))
}

/// An option's store contract segment (`YYYY-MM-DD-<paisa>-CE|PE`) as its
/// expiry, strike in paisa and side, or `None` for a future or any segment
/// that does not render back to itself. What the re-key (D-3331,
/// `crate::gdfl_rekey`) reads a stored key's identity from. O(segment).
#[must_use]
pub fn contract_parts(contract: Contract) -> Option<(Day, i64, OptionSide)> {
    let text = contract.as_str();
    let (day, rest) = text.split_at_checked(10)?;
    let (strike, side) = rest.strip_prefix('-')?.split_once('-')?;
    let (expiry, strike, side) = option_fields(day, strike, side)?;
    let back = ContractDay {
        expiry,
        strike,
        side,
        lot: LotEvidence::NoTrade,
        first_oi: None,
        close_oi: None,
    }
    .contract()?;
    (back == contract).then_some((expiry, strike, side))
}

/// Plain decimal digits as a number.
fn digits(text: &str) -> Option<u64> {
    (!text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
        .then(|| text.parse().ok())
        .flatten()
}

/// `YYYY-MM-DD` as a day.
pub(crate) fn parse_day(text: &str) -> Option<Day> {
    let (y, rest) = text.split_once('-')?;
    let (m, d) = rest.split_once('-')?;
    if (y.len(), m.len(), d.len()) != (4, 2, 2) {
        return None;
    }
    let year = u16::try_from(digits(y)?).ok()?;
    let month = u8::try_from(digits(m)?).ok()?;
    let day = u8::try_from(digits(d)?).ok()?;
    Day::new(year, month, day).ok()
}

/// The `name=value` fields of a record after its key, in the order given, or
/// `None` when the names or their order differ. D-3317's rules: one space
/// between fields, so a split on it is the whole parse.
fn named<'a, const N: usize>(fields: &[&'a str], names: [&str; N]) -> Option<[&'a str; N]> {
    if fields.len() != N {
        return None;
    }
    let mut out = [""; N];
    for ((slot, field), name) in out.iter_mut().zip(fields).zip(names) {
        *slot = field.strip_prefix(name)?.strip_prefix('=')?;
    }
    Some(out)
}

/// One D-3331 `lineage` record: an exchange contract renamed on an ex-date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LineageRecord {
    /// The underlying both keys name.
    pub underlying: Symbol,
    /// The pre-ex contract segment.
    pub old: Contract,
    /// The post-ex contract segment.
    pub new: Contract,
    /// The ex-date, measured.
    pub ex_date: Day,
    /// The factor, reduced, `(num, den)`.
    pub factor: (u64, u64),
    /// The inferred lot before, shares.
    pub lot_old: u64,
    /// The inferred lot from the ex-date, shares.
    pub lot_new: u64,
    /// The unix second up to which the new key's OI is the vendor's
    /// unadjusted value or the jump off it, 0 when none is.
    pub oi_unadjusted_until: i64,
}

impl LineageRecord {
    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "lineage {} {} ex={} factor={}/{} lot_old={} lot_new={} oi_unadjusted_until={} v={RECORD_VERSION}",
            option_key(self.underlying, self.old),
            option_key(self.underlying, self.new),
            self.ex_date,
            self.factor.0,
            self.factor.1,
            self.lot_old,
            self.lot_new,
            self.oi_unadjusted_until,
        )
    }

    /// The record [`Self::line`] wrote, or `None` for any other text: a
    /// field out of place, a value not in its canonical spelling, or two keys
    /// of different underlyings. Never repaired.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split(' ').collect();
        let (&[kind, old, new], rest) = fields.split_first_chunk::<3>()?;
        if kind != "lineage" {
            return None;
        }
        let [ex, factor, lot_old, lot_new, until, version] = named(
            rest,
            [
                "ex",
                "factor",
                "lot_old",
                "lot_new",
                "oi_unadjusted_until",
                "v",
            ],
        )?;
        let (underlying, old) = parse_option_key(old)?;
        let (also, new) = parse_option_key(new)?;
        let (num, den) = factor.split_once('/')?;
        let record = Self {
            underlying,
            old,
            new,
            ex_date: parse_day(ex)?,
            factor: (digits(num)?, digits(den)?),
            lot_old: digits(lot_old)?,
            lot_new: digits(lot_new)?,
            oi_unadjusted_until: i64::try_from(digits(until)?).ok()?,
        };
        // The factor is the lots' own ratio, reduced: a line that disagrees
        // with its lots, or spells it unreduced, is no line this wrote.
        let consistent = record.factor == reduced(record.lot_new, record.lot_old);
        (also == underlying && consistent && version == RECORD_VERSION && record.line() == line)
            .then_some(record)
    }
}

/// One D-3331 `stitched` record: a store key the vendor reused at an ex-date,
/// so its series holds one exchange contract before `ex` and another from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StitchedRecord {
    /// The underlying.
    pub underlying: Symbol,
    /// The stitched key's contract segment.
    pub key: Contract,
    /// The ex-date.
    pub ex_date: Day,
    /// The segment the key's PRE-ex contract continues under from the
    /// ex-date, or `None` when it was not paired (stopped, or two partners
    /// equally close).
    pub before: Option<Contract>,
    /// The pre-ex segment of the contract the key holds FROM the ex-date, or
    /// `None` when that contract was newly listed (no pre-ex contract was
    /// renamed onto the key).
    pub after: Option<Contract>,
}

/// A contract segment or `-`.
fn or_dash(contract: Option<Contract>) -> String {
    contract.map_or_else(|| "-".to_owned(), |c| c.as_str().to_owned())
}

/// [`or_dash`] read back: `-` is none, anything else must be an option
/// segment of `underlying`'s that renders back to itself (`Err` otherwise).
fn dash_or(underlying: Symbol, text: &str) -> Result<Option<Contract>, ()> {
    if text == "-" {
        return Ok(None);
    }
    match parse_option_key(&format!("NSE-FNO-{}-{text}", underlying.as_str())) {
        Some((under, contract)) if under == underlying => Ok(Some(contract)),
        _ => Err(()),
    }
}

impl StitchedRecord {
    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "stitched {} ex={} before={} after={} v={RECORD_VERSION}",
            option_key(self.underlying, self.key),
            self.ex_date,
            or_dash(self.before),
            or_dash(self.after),
        )
    }

    /// The record [`Self::line`] wrote, or `None` for any other text.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split(' ').collect();
        let (&[kind, key], rest) = fields.split_first_chunk::<2>()?;
        if kind != "stitched" {
            return None;
        }
        let [ex, before, after, version] = named(rest, ["ex", "before", "after", "v"])?;
        let (underlying, key) = parse_option_key(key)?;
        let record = Self {
            underlying,
            key,
            ex_date: parse_day(ex)?,
            before: dash_or(underlying, before).ok()?,
            after: dash_or(underlying, after).ok()?,
        };
        (version == RECORD_VERSION && record.line() == line).then_some(record)
    }
}

/// One D-3343 `lot` record: the inferred lot of one contract from a day on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LotRecord {
    /// The underlying.
    pub underlying: Symbol,
    /// The contract segment.
    pub contract: Contract,
    /// The first day this lot holds from.
    pub from: Day,
    /// Shares per lot, inferred ([`LotEvidence::BASIS`]).
    pub lot: u64,
}

impl LotRecord {
    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "lot {} from={} lot={} basis={} v={RECORD_VERSION}",
            option_key(self.underlying, self.contract),
            self.from,
            self.lot,
            LotEvidence::BASIS,
        )
    }

    /// The record [`Self::line`] wrote, or `None` for any other text.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split(' ').collect();
        let (&[kind, key], rest) = fields.split_first_chunk::<2>()?;
        if kind != "lot" {
            return None;
        }
        let [from, lot, basis, version] = named(rest, ["from", "lot", "basis", "v"])?;
        let (underlying, contract) = parse_option_key(key)?;
        let record = Self {
            underlying,
            contract,
            from: parse_day(from)?,
            lot: digits(lot).filter(|&lot| lot > 0)?,
        };
        (basis == LotEvidence::BASIS && version == RECORD_VERSION && record.line() == line)
            .then_some(record)
    }
}

/// One `superseded` record: a store key the current decoder files under
/// ANOTHER key (D-3331 decision 1, the settled expiry), whose bars the
/// re-key (`crate::gdfl_rekey`, WP-12c) published under that key as new
/// files. The old key's files stay where they are and stay readable; this
/// line is the named pointer from it to the key it continues as. In
/// `<store>/imports/gdfl.lineage`, D-3317's line rules:
///
/// `superseded <old_key> by=<new_key> kind=<closed_day|vendor_redated> months=<n> bars=<n> blake3=<hex> v=1`
///
/// `kind` is why the expiry moved (the settlement's own word); `months` and
/// `bars` are what was published under `by` from `old_key`; `blake3` is the
/// digest of those bars' 56-byte records in the order they were written, so
/// an auditor can prove the new files hold exactly the old ones' bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SupersededRecord {
    /// The underlying both keys name.
    pub underlying: Symbol,
    /// The stored key's contract segment, under its pre-D-3331 expiry.
    pub old: Contract,
    /// The key the current decoder gives, which now holds the bars.
    pub new: Contract,
    /// Why the expiry moved.
    pub kind: AliasKind,
    /// Month files published under `new` from `old`.
    pub months: u64,
    /// Bars published under `new` from `old`.
    pub bars: u64,
    /// BLAKE3 of those bars' records, in write order.
    pub digest: [u8; 32],
}

/// 32 bytes as 64 lowercase hex digits.
fn hex32(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// [`hex32`] read back; `None` for anything but 64 lowercase hex digits.
fn unhex32(text: &str) -> Option<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let nibble = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(bytes.chunks_exact(2)) {
        let (&[hi, lo], _) = pair.split_first_chunk::<2>()?;
        *slot = (nibble(hi)? << 4) | nibble(lo)?;
    }
    Some(out)
}

impl SupersededRecord {
    /// [`Self::digest`] as the 64 lower-case hex digits the line carries.
    #[must_use]
    pub fn digest_hex(&self) -> String {
        hex32(&self.digest)
    }

    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "superseded {} by={} kind={} months={} bars={} blake3={} v={RECORD_VERSION}",
            option_key(self.underlying, self.old),
            option_key(self.underlying, self.new),
            self.kind.as_str(),
            self.months,
            self.bars,
            self.digest_hex(),
        )
    }

    /// The record [`Self::line`] wrote, or `None` for any other text: two
    /// underlyings, a key superseded by itself, nothing published, or a
    /// value not in its canonical spelling. Never repaired.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split(' ').collect();
        let (&[kind, old], rest) = fields.split_first_chunk::<2>()?;
        if kind != "superseded" {
            return None;
        }
        let [by, why, months, bars, digest, version] =
            named(rest, ["by", "kind", "months", "bars", "blake3", "v"])?;
        let (underlying, old) = parse_option_key(old)?;
        let (also, new) = parse_option_key(by)?;
        let record = Self {
            underlying,
            old,
            new,
            kind: AliasKind::parse(why)?,
            months: digits(months).filter(|&n| n > 0)?,
            bars: digits(bars).filter(|&n| n > 0)?,
            digest: unhex32(digest)?,
        };
        (also == underlying && old != new && version == RECORD_VERSION && record.line() == line)
            .then_some(record)
    }
}

/// The word every reader's refusal of a RETIRED key starts with, followed by
/// `:` ([`RetiredRecord::refusal`]): what an HTTP route matches to answer
/// `410 Gone` rather than `400`, so the status cannot drift from the
/// sentence (D-3331; WP-12c, DB-A18).
pub const CONTRACT_REFUSED: &str = "contract_refused";

/// Whether `why` is a [`RetiredRecord::refusal`] sentence: it starts with
/// [`CONTRACT_REFUSED`] and a colon. O(1).
#[must_use]
pub fn is_contract_refused(why: &str) -> bool {
    why.strip_prefix(CONTRACT_REFUSED)
        .is_some_and(|rest| rest.starts_with(':'))
}

/// Why a stored key is retired: the current decoder refuses every day of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetiredWhy {
    /// [`crate::gdfl_nfo::NfoRefusal::StrikeImplausible`] on every bar day.
    StrikeImplausible,
    /// [`crate::gdfl_nfo::NfoRefusal::ExpiryUnsettled`] on every bar day.
    ExpiryUnsettled,
}

impl RetiredWhy {
    /// The wire word: the import journal's cause code for the same refusal
    /// (`crate::gdfl_import::nfo_cause`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StrikeImplausible => "strike_implausible",
            Self::ExpiryUnsettled => "expiry_unsettled",
        }
    }

    /// The word back; `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "strike_implausible" => Some(Self::StrikeImplausible),
            "expiry_unsettled" => Some(Self::ExpiryUnsettled),
            _ => None,
        }
    }
}

/// One `retired` record: a store key whose every bar day the current decoder
/// refuses (D-3331), so no reader serves or lists it. Its files are kept,
/// byte for byte; this line is the named refusal every reader quotes. In
/// `<store>/imports/gdfl.lineage`, D-3317's line rules:
///
/// `retired <key> why=<strike_implausible|expiry_unsettled> day=<YYYY-MM-DD> median=<paisa|-> days=<n> bars=<n> v=1`
///
/// `day` is the first refused day; `median` is that day's lower median strike
/// of the underlying (D-3331 decision 2) for a strike refusal and `-` for an
/// expiry refusal; `days` and `bars` are what the key holds, all refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RetiredRecord {
    /// The underlying.
    pub underlying: Symbol,
    /// The retired key's contract segment.
    pub key: Contract,
    /// Why.
    pub why: RetiredWhy,
    /// The first refused day.
    pub day: Day,
    /// That day's lower median strike, paisa, for a strike refusal.
    pub median: Option<i64>,
    /// IST days the key holds bars on, every one refused.
    pub days: u64,
    /// Bars the key holds.
    pub bars: u64,
}

impl RetiredRecord {
    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "retired {} why={} day={} median={} days={} bars={} v={RECORD_VERSION}",
            option_key(self.underlying, self.key),
            self.why.as_str(),
            self.day,
            self.median
                .map_or_else(|| "-".to_owned(), |median| median.to_string()),
            self.days,
            self.bars,
        )
    }

    /// The sentence a reader refuses the key with.
    #[must_use]
    pub fn refusal(&self) -> String {
        let key = option_key(self.underlying, self.key);
        match (self.why, self.median) {
            (RetiredWhy::StrikeImplausible, Some(median)) => format!(
                "{CONTRACT_REFUSED}: {key} is refused by the current decoder (D-3331): strike \
                 implausible, more than {} times the day's median strike {median} paisa on {} \
                 and on every other day it holds ({} day(s), {} bar(s)); its files are kept \
                 and are not served",
                crate::gdfl_nfo::STRIKE_PLAUSIBLE_MULTIPLE,
                self.day,
                self.days,
                self.bars
            ),
            _ => format!(
                "{CONTRACT_REFUSED}: {key} is refused by the current decoder (D-3331): {} from {} \
                 on every day it holds ({} day(s), {} bar(s)); its files are kept and are not \
                 served",
                self.why.as_str(),
                self.day,
                self.days,
                self.bars
            ),
        }
    }

    /// The record [`Self::line`] wrote, or `None` for any other text: a
    /// median where the reason has none or none where it needs one, a strike
    /// the median admits, nothing held, or a value not in its canonical
    /// spelling. Never repaired.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split(' ').collect();
        let (&[kind, key], rest) = fields.split_first_chunk::<2>()?;
        if kind != "retired" {
            return None;
        }
        let [why, day, median, days, bars, version] =
            named(rest, ["why", "day", "median", "days", "bars", "v"])?;
        let (underlying, key) = parse_option_key(key)?;
        let why = RetiredWhy::parse(why)?;
        let median = match median {
            "-" => None,
            text => Some(i64::try_from(digits(text)?).ok()?),
        };
        let (_, strike, _) = contract_parts(key)?;
        let consistent = match (why, median) {
            (RetiredWhy::StrikeImplausible, Some(median)) => {
                crate::gdfl_nfo::strike_implausible(strike, median)
            }
            (RetiredWhy::ExpiryUnsettled, None) => true,
            _ => false,
        };
        let record = Self {
            underlying,
            key,
            why,
            day: parse_day(day)?,
            median,
            days: digits(days).filter(|&n| n > 0)?,
            bars: digits(bars).filter(|&n| n > 0)?,
        };
        (consistent && version == RECORD_VERSION && record.line() == line).then_some(record)
    }
}

/// The inferred lot of every contract the import has seen, kept so a
/// [`LotRecord`] is written only when a lot is first seen or changes
/// (D-3343). The import holds one for a run and seeds it from the ledger it
/// appends to ([`Self::seed`]), so a re-run writes nothing new.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LotLedger {
    current: HashMap<(Symbol, Contract), u64>,
}

impl LotLedger {
    /// An empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes a record read back from the ledger file: the last record of a
    /// key wins, as D-3317's rules read it.
    pub fn seed(&mut self, record: &LotRecord) {
        self.current
            .insert((record.underlying, record.contract), record.lot);
    }

    /// The lot `underlying`'s `contract` holds now, if any day showed one.
    #[must_use]
    pub fn lot_of(&self, underlying: Symbol, contract: Contract) -> Option<u64> {
        self.current.get(&(underlying, contract)).copied()
    }

    /// One contract-day's evidence. The record to append, when the lot is
    /// first seen or this day lowers it; `None` when it is unchanged or the
    /// day showed no lot. A day's GCD is an upper bound of the true lot, so
    /// the lot kept is the common divisor of every day's: it can only fall
    /// within one lineage. One hash probe and one Euclid.
    pub fn observe(
        &mut self,
        underlying: Symbol,
        contract: Contract,
        on: Day,
        evidence: LotEvidence,
    ) -> Option<LotRecord> {
        let day = evidence.lot()?;
        let slot = self.current.entry((underlying, contract)).or_insert(0);
        let next = gcd(*slot, day);
        if next == *slot {
            return None;
        }
        *slot = next;
        Some(LotRecord {
            underlying,
            contract,
            from: on,
            lot: next,
        })
    }

    /// Forgets `contract`'s lot: the vendor reused its key for another
    /// exchange contract at an ex-date ([`CorporateAction::retired`]), so
    /// its next day starts a new lineage and is recorded as first seen.
    pub fn restart(&mut self, underlying: Symbol, contract: Contract) {
        self.current.remove(&(underlying, contract));
    }
}

/// The fewest contracts on each side of an ex-date that make a name break
/// (module doc): the audit's own bar, "at least 8 live contracts of one
/// underlying stop and the same expiries restart".
pub const MIN_EVENT_KEYS: usize = 8;

/// The fewest OI-carried pairs that prove a strike offset when pre-ex texts
/// trade again on the ex-date (module doc, fact 1).
pub const MIN_OI_PAIRS: usize = 5;

/// The fewest OI-carried pairs that make a name break an offset rather than
/// a re-listing (module doc, fact 2).
pub const MIN_OI_FRESH: usize = 3;

/// How many pre-ex keys of one ladder may share one close OI before the
/// value is not walked: a bound on [`detect`]'s vote, never reached by a
/// value that names a partner (one key).
pub const OI_BUCKET_CAP: usize = 8;

/// What moved on an ex-date. Measured; the cause is never claimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// The underlying-day lot stepped and every strike moved by its ratio
    /// (the shape of a bonus or split; module doc, step 4).
    LotStep,
    /// The lot is unchanged and every strike moved by one constant: the old
    /// strike less the new, paisa (the shape of a dividend adjustment).
    StrikeOffset {
        /// Old strike minus new strike, paisa; never zero.
        shift: i64,
    },
    /// The lot is unchanged, every pre-ex contract stopped and the same
    /// expiries were listed again: nothing is paired (the shape of an early
    /// expiry on a scheme of arrangement).
    Relisted,
}

impl EventKind {
    /// The word a log line or a page names it by.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LotStep => "lot_step",
            Self::StrikeOffset { .. } => "strike_offset",
            Self::Relisted => "relisted",
        }
    }

    /// The strike shift, paisa: the offset's own, 0 for the other kinds.
    #[must_use]
    pub const fn shift(self) -> i64 {
        match self {
            Self::StrikeOffset { shift } => shift,
            Self::LotStep | Self::Relisted => 0,
        }
    }

    /// The kind [`Self::as_str`] and [`Self::shift`] spell, or `None` for a
    /// word that is no kind, an offset of nothing, or a shift on a kind that
    /// has none.
    #[must_use]
    pub fn parse(word: &str, shift: i64) -> Option<Self> {
        match (word, shift) {
            ("lot_step", 0) => Some(Self::LotStep),
            ("relisted", 0) => Some(Self::Relisted),
            ("strike_offset", shift) if shift != 0 => Some(Self::StrikeOffset { shift }),
            _ => None,
        }
    }
}

/// A signed decimal in its canonical spelling: digits, with one leading `-`
/// for a negative.
fn signed(text: &str) -> Option<i64> {
    match text.strip_prefix('-') {
        Some(rest) => i64::try_from(digits(rest)?).ok()?.checked_neg(),
        None => i64::try_from(digits(text)?).ok(),
    }
}

/// One `event` record (D-3355): what [`detect`] declared for one underlying
/// on one ex-date. In `<store>/imports/gdfl.lineage`, D-3317's line rules:
///
/// `event NSE-FNO-<underlying> ex=<YYYY-MM-DD> kind=<lot_step|strike_offset|relisted> shift=<paisa> lot_old=<n> lot_new=<n> paired=<n> stopped=<n> v=1`
///
/// `shift` is the old strike less the new, signed, and 0 unless the kind is
/// `strike_offset`. `paired` counts the pre-ex keys with a `lineage` record
/// of this ex-date and `stopped` those with a [`StoppedRecord`]: together
/// every pre-ex key the event retired. A run that sees more of the pre-ex
/// state than an earlier one appends a second record with the larger counts;
/// the LAST record of an (underlying, ex-date) is the one in force. The cause
/// (bonus, dividend, scheme) is never written: it is not measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventRecord {
    /// The underlying.
    pub underlying: Symbol,
    /// The ex-date.
    pub ex_date: Day,
    /// What moved.
    pub kind: EventKind,
    /// The inferred lot before, shares.
    pub lot_old: u64,
    /// The inferred lot from the ex-date, shares.
    pub lot_new: u64,
    /// Pre-ex keys paired with a post-ex key.
    pub paired: u64,
    /// Pre-ex keys retired with no partner.
    pub stopped: u64,
}

impl EventRecord {
    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "event NSE-FNO-{} ex={} kind={} shift={} lot_old={} lot_new={} paired={} stopped={} v={RECORD_VERSION}",
            self.underlying.as_str(),
            self.ex_date,
            self.kind.as_str(),
            self.kind.shift(),
            self.lot_old,
            self.lot_new,
            self.paired,
            self.stopped,
        )
    }

    /// The record [`Self::line`] wrote, or `None` for any other text: a kind
    /// its lots or its counts contradict (a lot step with equal lots, any
    /// other kind with unequal ones, a re-listing that pairs), an event that
    /// retired nothing, or a value not in its canonical spelling.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split(' ').collect();
        let (&[word, subject], rest) = fields.split_first_chunk::<2>()?;
        if word != "event" {
            return None;
        }
        let [ex, kind, shift, lot_old, lot_new, paired, stopped, version] = named(
            rest,
            [
                "ex", "kind", "shift", "lot_old", "lot_new", "paired", "stopped", "v",
            ],
        )?;
        let under = subject.strip_prefix("NSE-FNO-")?;
        let underlying = Symbol::new(under)
            .ok()
            .filter(|symbol| symbol.as_str() == under)?;
        let record = Self {
            underlying,
            ex_date: parse_day(ex)?,
            kind: EventKind::parse(kind, signed(shift)?)?,
            lot_old: digits(lot_old)?,
            lot_new: digits(lot_new)?,
            paired: digits(paired)?,
            stopped: digits(stopped)?,
        };
        let consistent = match record.kind {
            EventKind::LotStep => record.lot_old != record.lot_new,
            EventKind::StrikeOffset { .. } => record.lot_old == record.lot_new,
            EventKind::Relisted => record.lot_old == record.lot_new && record.paired == 0,
        };
        let retired = record.paired.checked_add(record.stopped)? > 0;
        (consistent && retired && version == RECORD_VERSION && record.line() == line)
            .then_some(record)
    }
}

/// One `stopped` record (D-3355): a pre-ex key an event retired with no
/// partner. The exchange contract under it stopped on the day before `ex`
/// (expired early, or its adjusted key was not found), so a file under the
/// key on the ex-date or later is another exchange contract (a
/// [`StitchedRecord`] is written when one appears). In
/// `<store>/imports/gdfl.lineage`:
///
/// `stopped NSE-FNO-<underlying>-<contract> ex=<YYYY-MM-DD> v=1`
///
/// A key with both a `lineage` record and a `stopped` record of one ex-date
/// is PAIRED: a run that held less of the pre-ex state wrote the `stopped`
/// line, and the pair is the measurement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StoppedRecord {
    /// The underlying.
    pub underlying: Symbol,
    /// The retired key's contract segment.
    pub key: Contract,
    /// The ex-date.
    pub ex_date: Day,
}

impl StoppedRecord {
    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "stopped {} ex={} v={RECORD_VERSION}",
            option_key(self.underlying, self.key),
            self.ex_date,
        )
    }

    /// The record [`Self::line`] wrote, or `None` for any other text.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let fields: Vec<&str> = line.split(' ').collect();
        let (&[word, key], rest) = fields.split_first_chunk::<2>()?;
        if word != "stopped" {
            return None;
        }
        let [ex, version] = named(rest, ["ex", "v"])?;
        let (underlying, key) = parse_option_key(key)?;
        let record = Self {
            underlying,
            key,
            ex_date: parse_day(ex)?,
        };
        (version == RECORD_VERSION && record.line() == line).then_some(record)
    }
}

/// The store underlying an option file named for `stem` on trade day `trade`
/// is filed under (module doc; D-3333): the newest symbol the stem's dated
/// changes reach.
///
/// # Errors
///
/// [`Stale`] when the exchange was not using `stem` on `trade`: an old symbol
/// on or after its change, or a new one before it. Refused, as the cash side
/// refuses it; a reused symbol would name another company.
pub fn filed_underlying(stem: Symbol, trade: Day) -> Result<Symbol, Stale> {
    let on = Ymd::new(trade.year(), trade.month(), trade.day());
    let current = LINEAGE.filed_under(stem.as_str(), on)?;
    // A table symbol `Symbol` cannot hold would be a table fault; the stem
    // itself is then the only honest key, and the core table's own test
    // builds every row.
    Ok(Symbol::new(current).unwrap_or(stem))
}

/// One underlying's adjustment on one ex-date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorporateAction {
    /// The underlying.
    pub underlying: Symbol,
    /// What moved.
    pub kind: EventKind,
    /// The ex-date.
    pub ex_date: Day,
    /// The inferred lot before, shares.
    pub lot_before: u64,
    /// The inferred lot from the ex-date, shares.
    pub lot_after: u64,
    /// Every pre-ex contract paired with its post-ex key, in pre-ex order.
    pub lineage: Vec<Lineage>,
    /// Pre-ex contracts with no single post-ex partner: stopped trading, or
    /// two candidates equally close. Never guessed.
    pub unpaired: Vec<Contract>,
    /// Every post-ex key a pre-ex contract also carried, in post-ex order.
    /// An adjustment renames EVERY open contract, so the post-ex contract
    /// under such a key is a different exchange contract from the pre-ex one,
    /// renamed (`Lineage::reused`) or newly listed, and one store series
    /// under the key holds both (DB-G71). These are the ex-date's; a retired
    /// key listed again on a LATER day is stitched too ([`Self::retired`]).
    pub stitched: Vec<Contract>,
}

impl CorporateAction {
    /// Every pre-ex key the adjustment retired: each pre-ex contract was
    /// renamed (paired) or stopped (unpaired), so a file under one of these
    /// keys on the ex-date or any later day is a different exchange contract,
    /// and its store series is stitched. One hash probe per later file; the
    /// set lives until the last of its expiries. Measured on NTPC's
    /// 2019-03-19 adjustment: 35 retired keys traded again after it, the
    /// audit's count of NTPC's stitched series (DB-G71).
    #[must_use]
    pub fn retired(&self) -> HashSet<Contract> {
        self.lineage
            .iter()
            .map(|pair| pair.old)
            .chain(self.unpaired.iter().copied())
            .collect()
    }

    /// The `stitched` record of `key`, a retired key traded again on the
    /// ex-date or later: where its pre-ex contract went, and which pre-ex
    /// contract (if any) was renamed onto it. `None` for a key the action did
    /// not retire. O(pairs).
    #[must_use]
    pub fn stitched_record(&self, key: Contract) -> Option<StitchedRecord> {
        let retired =
            self.lineage.iter().any(|pair| pair.old == key) || self.unpaired.contains(&key);
        retired.then(|| StitchedRecord {
            underlying: self.underlying,
            key,
            ex_date: self.ex_date,
            before: self
                .lineage
                .iter()
                .find(|pair| pair.old == key)
                .map(|pair| pair.new),
            after: self
                .lineage
                .iter()
                .find(|pair| pair.new == key)
                .map(|pair| pair.old),
        })
    }

    /// The `stitched` records of the ex-date's own stitched keys, in
    /// [`Self::stitched`] order.
    #[must_use]
    pub fn stitched_records(&self) -> Vec<StitchedRecord> {
        self.stitched
            .iter()
            .filter_map(|&key| self.stitched_record(key))
            .collect()
    }
}

/// `strike × before / after` rounded half-up to the 0.05 grid, paisa.
fn predicted(strike: i64, before: u64, after: u64) -> Option<i64> {
    let q = i128::from(strike).checked_mul(i128::from(before))?;
    let unit = i128::from(after).checked_mul(i128::from(STRIKE_GRID_PAISA))?;
    if unit <= 0 || q < 0 {
        return None;
    }
    let steps = (q * 2 + unit) / (unit * 2);
    i64::try_from(steps * i128::from(STRIKE_GRID_PAISA)).ok()
}

/// The one strike of `ladder` (sorted) within tolerance of `want`, or `None`
/// when there is none or more than one.
fn single_near(ladder: &[i64], want: i64) -> Option<i64> {
    let tolerance = (want * STRIKE_TOLERANCE_PER_MILLE / 1_000).max(STRIKE_GRID_PAISA);
    let from = ladder.partition_point(|&k| k < want - tolerance);
    let mut near = ladder
        .get(from..)
        .unwrap_or_default()
        .iter()
        .take_while(|&&k| k <= want + tolerance);
    match (near.next(), near.next()) {
        (Some(&k), None) => Some(k),
        _ => None,
    }
}

/// The adjustment of `underlying` on `ex_date`, from its pre-ex state
/// (`before`) and its contract-days of `ex_date` (`after`), or `None` when the
/// strikes did not move: a lot revision, an ordinary day, or a day whose lot
/// reads wrongly. A lot step is module doc step 4; an unchanged lot is
/// "Events that keep the lot".
///
/// `before` is the latest pre-ex contract-day of every key the underlying
/// holds that has not expired by `ex_date`: the vendor day before for every
/// key that traded on it, an earlier day for one that did not. A key that
/// last traded earlier still pairs, and still makes a post-ex key that reuses
/// its text [`Lineage::reused`]: the NTPC `12500-PE` of DB-G71 last traded
/// four sessions before the ex-date. Given only the vendor day before, the
/// pairs and the reused texts are the subset of those keys.
#[must_use]
pub fn detect(
    underlying: Symbol,
    ex_date: Day,
    before: &[ContractDay],
    after: &[ContractDay],
) -> Option<CorporateAction> {
    let (lot_before, lot_after) = (day_lot(before)?, day_lot(after)?);
    if lot_before == lot_after {
        return detect_unchanged_lot(underlying, ex_date, lot_before, before, after);
    }
    let before_keys: HashSet<(Day, i64, OptionSide)> =
        before.iter().map(ContractDay::key).collect();
    let after_by_key: HashMap<(Day, i64, OptionSide), &ContractDay> =
        after.iter().map(|c| (c.key(), c)).collect();
    let mut ladders: HashMap<(Day, OptionSide), Vec<i64>> = HashMap::new();
    for c in after {
        ladders
            .entry((c.expiry, c.side))
            .or_default()
            .push(c.strike);
    }
    for ladder in ladders.values_mut() {
        ladder.sort_unstable();
        ladder.dedup();
    }
    let stitched: Vec<Contract> = after
        .iter()
        .filter(|c| before_keys.contains(&c.key()))
        .filter_map(ContractDay::contract)
        .collect();
    let continuing = stitched.len();
    let mut lineage = Vec::new();
    let mut unpaired = Vec::new();
    for old in before {
        let partner = predicted(old.strike, lot_before, lot_after)
            .zip(ladders.get(&(old.expiry, old.side)))
            .and_then(|(want, ladder)| single_near(ladder, want))
            .and_then(|strike| after_by_key.get(&(old.expiry, strike, old.side)));
        let (Some(old_contract), Some(new)) = (old.contract(), partner) else {
            unpaired.extend(old.contract());
            continue;
        };
        let Some(new_contract) = new.contract() else {
            unpaired.push(old_contract);
            continue;
        };
        lineage.push(Lineage {
            underlying,
            ex_date,
            old: old_contract,
            new: new_contract,
            lot_before,
            lot_after,
            pre_close_oi: old.close_oi,
            oi: OiCarry::of(old.close_oi, new.first_oi, lot_before, lot_after),
            reused: before_keys.contains(&new.key()),
        });
    }
    (!lineage.is_empty() && lineage.len() > continuing).then_some(CorporateAction {
        underlying,
        kind: EventKind::LotStep,
        ex_date,
        lot_before,
        lot_after,
        lineage,
        unpaired,
        stitched,
    })
}

/// Every pre-ex contract of an unchanged-lot event paired with the day's key
/// `shift` below it (same expiry and side), and those with no such key. With
/// no shift (a re-listing) nothing pairs.
fn pairs_at(
    underlying: Symbol,
    ex_date: Day,
    lot: u64,
    offset: Option<i64>,
    before: &[ContractDay],
    after: &[ContractDay],
) -> (Vec<Lineage>, Vec<Contract>) {
    type Key = (Day, i64, OptionSide);
    let before_keys: HashSet<Key> = before.iter().map(ContractDay::key).collect();
    let after_by_key: HashMap<Key, &ContractDay> = after.iter().map(|c| (c.key(), c)).collect();
    let mut lineage = Vec::new();
    let mut unpaired = Vec::new();
    for old in before {
        let Some(old_contract) = old.contract() else {
            continue;
        };
        let partner = offset
            .and_then(|shift| old.strike.checked_sub(shift))
            .and_then(|strike| after_by_key.get(&(old.expiry, strike, old.side)))
            .and_then(|new| new.contract().map(|contract| (new, contract)));
        let Some((new, new_contract)) = partner else {
            unpaired.push(old_contract);
            continue;
        };
        lineage.push(Lineage {
            underlying,
            ex_date,
            old: old_contract,
            new: new_contract,
            lot_before: lot,
            lot_after: lot,
            pre_close_oi: old.close_oi,
            oi: OiCarry::of(old.close_oi, new.first_oi, lot, lot),
            reused: before_keys.contains(&new.key()),
        });
    }
    (lineage, unpaired)
}

/// [`detect`]'s arm for an unchanged lot (module doc, "Events that keep the
/// lot"): a strike offset proved by the OI, a name break, or nothing. One
/// pass over each side; no sort.
fn detect_unchanged_lot(
    underlying: Symbol,
    ex_date: Day,
    lot: u64,
    before: &[ContractDay],
    after: &[ContractDay],
) -> Option<CorporateAction> {
    type Key = (Day, i64, OptionSide);
    let before_by_key: HashMap<Key, &ContractDay> = before.iter().map(|c| (c.key(), c)).collect();
    // Pre-ex strikes by (expiry, side, close OI): the partner a carried OI
    // names. Only a positive OI can name one.
    let mut by_oi: HashMap<(Day, OptionSide, i64), Vec<i64>> = HashMap::new();
    let mut old_expiries: HashMap<Day, usize> = HashMap::new();
    for c in before {
        *old_expiries.entry(c.expiry).or_default() += 1;
        if let Some(oi) = c.close_oi.filter(|&oi| oi > 0) {
            let bucket = by_oi.entry((c.expiry, c.side, oi)).or_default();
            if bucket.len() <= OI_BUCKET_CAP {
                bucket.push(c.strike);
            }
        }
    }
    let mut continuing = 0_usize;
    let mut carried_same = 0_usize;
    let mut relisted = 0_usize;
    let mut shared: HashSet<Day> = HashSet::new();
    let mut votes: HashMap<i64, usize> = HashMap::new();
    for c in after {
        if old_expiries.contains_key(&c.expiry) {
            relisted += 1;
            shared.insert(c.expiry);
        }
        let Some(first) = c.first_oi.filter(|&oi| oi > 0) else {
            continuing += usize::from(before_by_key.contains_key(&c.key()));
            continue;
        };
        // The same text carrying its OI is the ordinary day's evidence,
        // counted however many keys share the value; a shift is voted for
        // only by a value exactly one pre-ex key of the ladder closed on.
        if let Some(old) = before_by_key.get(&c.key()) {
            continuing += 1;
            carried_same += usize::from(old.close_oi == Some(first));
        }
        let Some(bucket) = by_oi.get(&(c.expiry, c.side, first)) else {
            continue;
        };
        if let [strike] = bucket.as_slice()
            && *strike != c.strike
        {
            *votes.entry(strike - c.strike).or_default() += 1;
        }
    }
    let stopped: usize = shared
        .iter()
        .filter_map(|expiry| old_expiries.get(expiry))
        .sum();
    let name_break = continuing == 0 && relisted >= MIN_EVENT_KEYS && stopped >= MIN_EVENT_KEYS;
    // The shift with the most votes, when no other has as many.
    let best = votes.values().copied().max().unwrap_or(0);
    let mut at_best = votes.iter().filter(|&(_, &n)| n == best);
    let shift = match (at_best.next(), at_best.next()) {
        (Some((&shift, _)), None) => Some(shift),
        _ => None,
    };
    let need = if name_break {
        MIN_OI_FRESH
    } else {
        MIN_OI_PAIRS
    };
    let offset = shift.filter(|_| best >= need && best > carried_same.saturating_mul(4));
    let sourced = crate::expiry_alias::relisted_on(underlying.as_str(), ex_date).is_some();
    let kind = match offset {
        Some(shift) => EventKind::StrikeOffset { shift },
        None if name_break || (sourced && relisted > 0) => EventKind::Relisted,
        None => return None,
    };
    let (lineage, unpaired) = pairs_at(underlying, ex_date, lot, offset, before, after);
    let stitched: Vec<Contract> = after
        .iter()
        .filter(|c| before_by_key.contains_key(&c.key()))
        .filter_map(ContractDay::contract)
        .collect();
    Some(CorporateAction {
        underlying,
        kind,
        ex_date,
        lot_before: lot,
        lot_after: lot,
        lineage,
        unpaired,
        stitched,
    })
}

#[cfg(test)]
#[path = "gdfl_lineage_tests.rs"]
mod tests;
