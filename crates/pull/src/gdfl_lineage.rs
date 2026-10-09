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
//!   basis=ltq_gcd v=1`, in `<store>/imports/gdfl.lots`.
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
//! [`day_lot`] is O(contracts). [`detect`] returns after the two lots when
//! they are equal, which is every day but an ex-date; on an ex-date it is
//! O(contracts · log contracts) for the underlying, once. [`Lineage::stale_oi`]
//! is O(the key's ex-date rows), once per renamed contract. An import-path
//! cost, not a query: no reader calls any of it.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Expiry, Kind, OptionSide};
use brutex_core::price::Paisa;
use brutex_core::symbol::Symbol;

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
    let expiry = parse_day(&format!("{yyyy}-{mm}-{dd}"))?;
    let side = match side {
        "CE" => OptionSide::Call,
        "PE" => OptionSide::Put,
        _ => return None,
    };
    let strike = i64::try_from(digits(strike)?).ok()?;
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

/// One underlying's adjustment on one ex-date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorporateAction {
    /// The underlying.
    pub underlying: Symbol,
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
/// day lot did not change or the strikes did not move with it (a lot
/// revision, or a day whose lot reads wrongly). Module doc, step 4.
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
        return None;
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
        ex_date,
        lot_before,
        lot_after,
        lineage,
        unpaired,
        stitched,
    })
}

#[cfg(test)]
#[path = "gdfl_lineage_tests.rs"]
mod tests;
