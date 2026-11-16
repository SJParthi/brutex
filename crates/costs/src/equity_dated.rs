//! The dated charge table for NSE cash equity, intraday, from 3 Sep 2018 to
//! 30 Oct 2026 — every row of `costs-table.json` version
//! [`EQUITY_DATED_VERSION`], with its source (package FNO-COSTS, D-8110).
//!
//! # What this is
//!
//! The F&O top-mover strategy trades the cash shares of F&O stocks intraday,
//! and every trade is priced twice (operator rule 7, 30 Oct 2026): at the
//! rates in force **on its own day** (the dated clock) and at Dhan's rates of
//! today (the today clock). The dated clock needs every charge's history
//! across the window, and this module is that history: 25 rows, transcribed
//! from `state/fib-strategy-20261030/costs-table.json` (COSTS.md section 3),
//! each with its component, exact rate, base, side, effective dates, the clocks
//! it serves, its source (document, URL, the day it was read and its own
//! words) and whether that source is primary ([`Status::Cited`]) or not
//! ([`Status::Unverified`]).
//!
//! # Exact rates
//!
//! Every rate is a [`Ratio`]. A rate per crore is a numerator of paise over
//! [`PER_CRORE`] (the paise in one crore of rupees), so the Rs 306.99 and the
//! Rs 0.01 of NSE circular FA73061 are exact — the regime tables' one rupee
//! per crore could hold neither (COSTS.md section 7, item 2).
//!
//! # Lookup cost
//!
//! [`rates_on`] picks one row per component. A component's rows form a
//! **series**: an anchor and a fixed array of later rows, each starting the
//! day after the one before ends. The walk is the array's length — at most
//! five — and is fixed at compile time, so the lookup is O(1) by
//! construction; `C-FNO-COSTS-01` in `crates/costs/benches/ratio.rs` measures
//! it. The series are checked at compile time: contiguous, anchored on or
//! before the window's first day, and open after the last row.
//!
//! # The window
//!
//! [`WINDOW_START`] (3 Sep 2018, the operator's first day) to [`READ_ON`]
//! (30 Oct 2026, the day every source was read). A day outside it is refused
//! by name: before it the table's own early rows are not the operator's
//! window, and after it no rate is verified.

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;

use crate::charge_error::ChargeError;
use crate::dated::boundary;
use crate::day::TradeDay;
use crate::error::Refusal;
use crate::exact::Ratio;
use crate::schedule::Status;

/// The version of the dated equity table: `costs-table.json`'s own version.
/// A run identity records it beside the reading set, so two runs priced on
/// different tables never share an identity. Pinned to the rows by
/// `the_table_version_is_pinned_to_its_rows`.
pub const EQUITY_DATED_VERSION: &str = "costs-table/2026-10-30/r1";

/// The first day of the operator's window: 3 Sep 2018.
pub const WINDOW_START: TradeDay = boundary!(2018 - 9 - 3);

/// The day every source of the table was read: 30 Oct 2026. No rate after it
/// is verified, so a later day is refused.
pub const READ_ON: TradeDay = boundary!(2026 - 10 - 30);

/// The first day after [`READ_ON`]: where the refused window after the table
/// begins.
pub const AFTER_READ_ON: TradeDay = boundary!(2026 - 10 - 31);

/// The paise in one crore of rupees. A rate of Rs `x.yz` per crore is
/// `xyz / PER_CRORE`.
pub const PER_CRORE: i64 = 1_000_000_000;

/// What a row charges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Component {
    /// Dhan's brokerage on an executed order.
    Brokerage,
    /// Dhan's auto square-off fee.
    SquareOffFee,
    /// The depository participant's charge on a sell.
    DpCharge,
    /// Dhan's call-and-trade fee (orders through the trade desk; not used).
    CallAndTrade,
    /// NSE's cash-market transaction charge.
    NseTransaction,
    /// NSE's concession for stocks outside NIFTY 50 and Next 50
    /// (informational: lower than the regular rate, never the worst).
    NseConcession,
    /// The contribution to NSE's Investor Protection Fund Trust.
    Ipft,
    /// SEBI's turnover fee.
    SebiFee,
    /// Securities transaction tax on a non-delivery sale.
    Stt,
    /// Stamp duty under each State's law, before 1 Jul 2020.
    StateStamp,
    /// The unified stamp duty on a non-delivery transfer, from 1 Jul 2020.
    Stamp,
    /// GST on brokerage and the exchange charges.
    Gst,
    /// GST on the SEBI fee.
    GstOnSebi,
    /// A clearing charge (none found).
    Clearing,
    /// Interest on intraday leverage (none on a squared-off day).
    LeverageInterest,
}

/// How a row's rate is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RowRate {
    /// `min(cap, ratio × base)`: Dhan's intraday brokerage.
    Capped {
        /// The rate.
        ratio: Ratio,
        /// The cap, per order.
        cap: Paisa,
    },
    /// `ratio × base`, exactly.
    Ratio(Ratio),
    /// A fixed amount per order or per instruction.
    Flat(Paisa),
    /// Each State's own rate: [`STATE_STAMPS`].
    StateTable,
    /// No charge was found; the row records the search and its limits.
    NoneFound,
}

impl RowRate {
    /// The rate as one exact fraction: a ratio as itself, a cap's ratio, a
    /// fixed amount as paise per unit of its base (`amount / 1`), and the
    /// state table and "none found" as zero — neither is priced from here
    /// (the state table through [`worst_state_stamp`], "none found" never).
    #[must_use]
    pub const fn ratio(self) -> Ratio {
        match self {
            Self::Capped { ratio, .. } | Self::Ratio(ratio) => ratio,
            Self::Flat(amount) => Ratio::new(amount.raw(), 1),
            Self::StateTable | Self::NoneFound => Ratio::new(0, 1),
        }
    }
}

/// What a row's rate is levied on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Base {
    /// The value of each executed order.
    OrderValue,
    /// Each order Dhan places to square off an open intraday position.
    SquareOffOrder,
    /// Each scrip-day with a sell.
    ScripDayWithSell,
    /// Each order placed through Dhan's trade desk.
    TradeDeskOrder,
    /// The traded value, each side.
    Turnover,
    /// The traded value of the eligible stocks only.
    EligibleTurnover,
    /// NSE's non-delivery value: the squared quantity at the day's VWAP.
    NonDeliveryValue,
    /// The contract note's value, buy and sell.
    ContractNoteValue,
    /// The FIFO buy value of the non-delivery quantity.
    FifoBuyValue,
    /// Brokerage plus the exchange charge and IPFT (plus Dhan's fees).
    GstBase,
    /// The SEBI turnover fee.
    SebiFee,
    /// Nothing: the row prices nothing.
    NotApplicable,
}

/// Which side pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    /// Every executed order, buy and sell.
    EachOrder,
    /// The square-off order.
    SquareOffOrder,
    /// The seller.
    Sell,
    /// The buyer.
    Buy,
    /// Both sides of the turnover.
    BothSides,
    /// Either side.
    Either,
    /// Nothing is charged.
    NotApplicable,
}

/// Which cost clocks a row serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Clocks {
    /// The rates in force on the trade's own day.
    pub dated: bool,
    /// Dhan's rates as read on [`READ_ON`].
    pub today: bool,
}

/// Both clocks.
const BOTH: Clocks = Clocks {
    dated: true,
    today: true,
};

/// The dated clock only.
const DATED: Clocks = Clocks {
    dated: true,
    today: false,
};

/// The today clock only.
const TODAY: Clocks = Clocks {
    dated: false,
    today: true,
};

/// One row of the dated table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DatedRow {
    /// The row's id in `costs-table.json` (`EX-2021`, `SF-2020`, ...).
    pub id: &'static str,
    /// What it charges.
    pub component: Component,
    /// The exact rate and how it applies.
    pub rate: RowRate,
    /// What it is levied on.
    pub base: Base,
    /// Who pays.
    pub side: Side,
    /// The first day in force. [`TradeDay::MIN`] where the source is undated
    /// or its start was never found; [`Self::from_note`] then says which.
    pub from: TradeDay,
    /// Why [`Self::from`] is [`TradeDay::MIN`], or `None` for a dated start.
    pub from_note: Option<&'static str>,
    /// The last day in force, or `None` while it is in force.
    pub to: Option<TradeDay>,
    /// The cost clocks it serves.
    pub clocks: Clocks,
    /// The document, its URL, the day it was read and its own words.
    pub source: &'static str,
    /// Whether the source is primary.
    pub status: Status,
}

impl DatedRow {
    /// Whether the row is in force on `day`: on or after [`Self::from`] and
    /// on or before [`Self::to`].
    #[must_use]
    pub const fn covers(&self, day: TradeDay) -> bool {
        if day.before(self.from) {
            return false;
        }
        match self.to {
            Some(last) => !last.before(day),
            None => true,
        }
    }
}

/// A rate of `paise` per crore, exactly.
const fn per_crore(paise: i64) -> Ratio {
    Ratio::new(paise, PER_CRORE)
}

/// A percentage `num / den` of a percent, exactly: `pct(25, 1_000)` is 0.025%.
const fn pct(num: i64, den: i64) -> Ratio {
    Ratio::new(num, den * 100)
}

// ---------------------------------------------------------------------------
// The 25 rows, in costs-table.json's order
// ---------------------------------------------------------------------------

/// Dhan's equity intraday brokerage rate, 0.03% of the order value (BRK-1).
pub const BROKERAGE_RATE: Ratio = pct(3, 100);

/// The Rs 20 cap on Dhan's equity intraday brokerage, per order (BRK-1).
pub const BROKERAGE_CAP: Paisa = Paisa::from_raw(20_00);

/// Dhan's auto square-off fee, Rs 20 an order before GST (BRK-SQ).
pub const SQUARE_OFF_FEE: Paisa = Paisa::from_raw(20_00);

/// The DP charge, Rs 12.50 an instruction before GST (BRK-DP).
pub const DP_CHARGE: Paisa = Paisa::from_raw(12_50);

const UNDATED_DHAN: &str = "UNVERIFIED: Dhan's page is undated; the archived Dhan page of \
     2023-02-01 shows the same rate (sources-charges.json B-01); the operator's decision of \
     13 Oct 2026 applies it to every date";

/// BRK-1: Dhan's equity intraday brokerage.
pub const BRK_1: DatedRow = DatedRow {
    id: "BRK-1",
    component: Component::Brokerage,
    rate: RowRate::Capped {
        ratio: BROKERAGE_RATE,
        cap: BROKERAGE_CAP,
    },
    base: Base::OrderValue,
    side: Side::EachOrder,
    from: TradeDay::MIN,
    from_note: Some(UNDATED_DHAN),
    to: None,
    clocks: BOTH,
    source: "Dhan pricing page, Equity tab; https://dhan.co/pricing/; read 2026-10-30; \
             \"₹20 or 0.03% of trade value per executed order, whichever is lower.\"",
    status: Status::Cited,
};

/// BRK-SQ: Dhan's auto square-off fee.
pub const BRK_SQ: DatedRow = DatedRow {
    id: "BRK-SQ",
    component: Component::SquareOffFee,
    rate: RowRate::Flat(SQUARE_OFF_FEE),
    base: Base::SquareOffOrder,
    side: Side::SquareOffOrder,
    from: TradeDay::MIN,
    from_note: Some("UNVERIFIED (undated page)"),
    to: None,
    clocks: BOTH,
    source: "Dhan pricing page, Other Charges; https://dhan.co/pricing/; read 2026-10-30; \
             \"₹ 20 / order + GST. For intraday order automatically squared by our team\"; time: \
             https://dhan.co/support/general/market-session-status-and-timing/\
             what-are-the-dhan-intraday-auto-square-off-timings/ (equity cash intraday at 3:10 PM)",
    status: Status::Cited,
};

/// BRK-DP: the DP transaction charge (on intraday: reading RD-DP).
pub const BRK_DP: DatedRow = DatedRow {
    id: "BRK-DP",
    component: Component::DpCharge,
    rate: RowRate::Flat(DP_CHARGE),
    base: Base::ScripDayWithSell,
    side: Side::Sell,
    from: TradeDay::MIN,
    from_note: Some("UNVERIFIED (undated page)"),
    to: None,
    clocks: BOTH,
    source: "Dhan pricing page, Demat Account Related Charges; https://dhan.co/pricing/; read \
             2026-10-30; \"₹ 12.50 / instruction / ISIN + GST.\" PRIMARY (broker); whether it \
             applies to an intraday day is UNVERIFIED (reading RD-DP)",
    status: Status::Cited,
};

/// BRK-CT: Dhan's call-and-trade fee. Not used: the strategy orders by API.
pub const BRK_CT: DatedRow = DatedRow {
    id: "BRK-CT",
    component: Component::CallAndTrade,
    rate: RowRate::Flat(Paisa::from_raw(50_00)),
    base: Base::TradeDeskOrder,
    side: Side::Either,
    from: TradeDay::MIN,
    from_note: Some("UNVERIFIED (undated page)"),
    to: None,
    clocks: BOTH,
    source: "Dhan pricing page; https://dhan.co/pricing/; read 2026-10-30; \"₹ 50 / order + GST. \
             For every order placed through our trade desk.\" Not used: orders are placed by API",
    status: Status::Cited,
};

/// EX-2009: NSE's cash slabs, top slab Rs 325 per crore.
pub const EX_2009: DatedRow = DatedRow {
    id: "EX-2009",
    component: Component::NseTransaction,
    rate: RowRate::Ratio(per_crore(32_500)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2009 - 10 - 1),
    from_note: None,
    to: Some(boundary!(2020 - 12 - 31)),
    clocks: DATED,
    source: "NSE/F&A/13028, 7 Sep 2009; https://nsearchives.nseindia.com/content/circulars/\
             faac13028.pdf; read 2026-10-30; \"Up to First Rs. 1250 cores Rs. 3.25 each side\" — \
             member slabs Rs 3.25..3.00 per lakh, top slab used (RD-SLAB)",
    status: Status::Cited,
};

/// EX-2020C: NSE's concession for stocks outside NIFTY 50 and Next 50.
/// Informational: never the worst, never selected.
pub const EX_2020C: DatedRow = DatedRow {
    id: "EX-2020C",
    component: Component::NseConcession,
    rate: RowRate::Ratio(per_crore(27_500)),
    base: Base::EligibleTurnover,
    side: Side::BothSides,
    from: boundary!(2020 - 8 - 1),
    from_note: None,
    to: Some(boundary!(2023 - 3 - 31)),
    clocks: DATED,
    source: "NSE/FA/45147, 46225, 47844, 49796, 50832, 51855, 53913; \
             https://nsearchives.nseindia.com/content/circulars/FA53913.pdf; read 2026-10-30; \
             \"applicable for a period of six months from October 1, 2022 to March 31, 2023\" — \
             informational, lower than the regular slab, never the worst reading",
    status: Status::Cited,
};

/// EX-2021: NSE's cash slabs, top slab Rs 345 per crore.
pub const EX_2021: DatedRow = DatedRow {
    id: "EX-2021",
    component: Component::NseTransaction,
    rate: RowRate::Ratio(per_crore(34_500)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2021 - 1 - 1),
    from_note: None,
    to: Some(boundary!(2023 - 3 - 31)),
    clocks: DATED,
    source: "NSE/FA/46730, 18 Dec 2020; https://nsearchives.nseindia.com/content/circulars/\
             FA46730.pdf; read 2026-10-30; \"will be applicable with effect from January 01, \
             2021.\" — slabs Rs 3.45..3.20 per lakh, top slab used",
    status: Status::Cited,
};

/// EX-2023: NSE's cash slabs, top slab Rs 325 per crore.
pub const EX_2023: DatedRow = DatedRow {
    id: "EX-2023",
    component: Component::NseTransaction,
    rate: RowRate::Ratio(per_crore(32_500)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2023 - 4 - 1),
    from_note: None,
    to: Some(boundary!(2024 - 3 - 31)),
    clocks: DATED,
    source: "NSE/FA/56129, 24 Mar 2023; https://nsearchives.nseindia.com/content/circulars/\
             FA56129.pdf; read 2026-10-30; \"will be applicable with effect from April 01, \
             2023.\" — slabs Rs 3.25..3.00 per lakh, top slab used",
    status: Status::Cited,
};

/// EX-2024A: NSE's cash slabs, top slab Rs 322 per crore.
pub const EX_2024A: DatedRow = DatedRow {
    id: "EX-2024A",
    component: Component::NseTransaction,
    rate: RowRate::Ratio(per_crore(32_200)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2024 - 4 - 1),
    from_note: None,
    to: Some(boundary!(2024 - 9 - 30)),
    clocks: DATED,
    source: "NSE/FA/61137, 14 Mar 2024; https://nsearchives.nseindia.com/content/circulars/\
             FA61137.pdf; read 2026-10-30; \"will be applicable with effect from April 01, \
             2024.\" — slabs Rs 3.22..2.97 per lakh, top slab used",
    status: Status::Cited,
};

/// EX-2024B: NSE's one rate for every member, Rs 297 per crore.
pub const EX_2024B: DatedRow = DatedRow {
    id: "EX-2024B",
    component: Component::NseTransaction,
    rate: RowRate::Ratio(per_crore(29_700)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2024 - 10 - 1),
    from_note: None,
    to: Some(boundary!(2026 - 2 - 28)),
    clocks: DATED,
    source: "NSE/FA/64232, 27 Sep 2024; https://nsearchives.nseindia.com/content/circulars/\
             FA64232.pdf; read 2026-10-30; \"Cash Market: Rs. 2.97 each side per lakh of traded \
             value\" — uniform for all members",
    status: Status::Cited,
};

/// EX-2026: NSE's Rs 306.99 per crore.
pub const EX_2026: DatedRow = DatedRow {
    id: "EX-2026",
    component: Component::NseTransaction,
    rate: RowRate::Ratio(per_crore(30_699)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2026 - 3 - 1),
    from_note: None,
    to: None,
    clocks: BOTH,
    source: "NSE/FA/73061, 27 Feb 2026; https://nsearchives.nseindia.com/content/circulars/\
             FA73061.pdf; read 2026-10-30; \"The above revision will be applicable with effect \
             from March 1, 2026.\"; Dhan: \"NSE: 0.0030699%\"",
    status: Status::Cited,
};

/// IP-OLD: the IPFT contribution at Rs 0.01 per crore, start not found.
pub const IP_OLD: DatedRow = DatedRow {
    id: "IP-OLD",
    component: Component::Ipft,
    rate: RowRate::Ratio(per_crore(1)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: TradeDay::MIN,
    from_note: Some(
        "UNVERIFIED start: FA56129 calls Rs 0.01 the \"existing\" level on 24 Mar 2023 and \
         FA73061 the level \"levied prior to April 1, 2023\"; no earlier circular was found by \
         subject search; applied on every earlier date (under one paisa a trade)",
    ),
    to: Some(boundary!(2023 - 3 - 31)),
    clocks: DATED,
    source: "NSE/FA/56129; https://nsearchives.nseindia.com/content/circulars/FA56129.pdf; read \
             2026-10-30; \"from existing Rs.0.01 per crore of the traded value to Rs.10\"",
    status: Status::Cited,
};

/// IP-2023: the IPFT contribution at Rs 10 per crore.
pub const IP_2023: DatedRow = DatedRow {
    id: "IP-2023",
    component: Component::Ipft,
    rate: RowRate::Ratio(per_crore(10_00)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2023 - 4 - 1),
    from_note: None,
    to: Some(boundary!(2026 - 2 - 28)),
    clocks: DATED,
    source: "NSE/FA/56129; https://nsearchives.nseindia.com/content/circulars/FA56129.pdf; read \
             2026-10-30; \"Rs.10 per crore of the traded value in Cash Equities Market Segment\"",
    status: Status::Cited,
};

/// IP-2026: the IPFT contribution at Rs 0.01 per crore.
pub const IP_2026: DatedRow = DatedRow {
    id: "IP-2026",
    component: Component::Ipft,
    rate: RowRate::Ratio(per_crore(1)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2026 - 3 - 1),
    from_note: None,
    to: None,
    clocks: BOTH,
    source: "NSE/FA/73061; https://nsearchives.nseindia.com/content/circulars/FA73061.pdf; read \
             2026-10-30; \"Rs. 0.01 each side\"; Dhan: \"IPFT Contribution 0.0000001% of the \
             turnover\"",
    status: Status::Cited,
};

/// SF-2017: SEBI's fee at Rs 15 per crore.
pub const SF_2017: DatedRow = DatedRow {
    id: "SF-2017",
    component: Component::SebiFee,
    rate: RowRate::Ratio(per_crore(15_00)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2017 - 3 - 6),
    from_note: None,
    to: Some(boundary!(2019 - 3 - 31)),
    clocks: DATED,
    source: "SEBI/LAD/NRO/GN/2016-17/037, 6 Mar 2017, annexed to NSE/FA/34501; \
             https://nsearchives.nseindia.com/content/circulars/FA34501.zip; read 2026-10-30; \
             \"0.00015 per cent of the price at which the securities are purchased or sold\"",
    status: Status::Cited,
};

/// SF-2019: SEBI's fee at Rs 10 per crore.
pub const SF_2019: DatedRow = DatedRow {
    id: "SF-2019",
    component: Component::SebiFee,
    rate: RowRate::Ratio(per_crore(10_00)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2019 - 4 - 1),
    from_note: None,
    to: Some(boundary!(2020 - 5 - 31)),
    clocks: DATED,
    source: "SEBI/L.A.D.-N.R.O./G.N./2019/03, 22 Mar 2019, annexed to NSE/FA/40632; \
             https://nsearchives.nseindia.com/content/circulars/FA40632.pdf; read 2026-10-30; \
             \"They shall come into force with effect from April 01, 2019.\"",
    status: Status::Cited,
};

/// SF-2020: SEBI's COVID relief, Rs 5 per crore.
pub const SF_2020: DatedRow = DatedRow {
    id: "SF-2020",
    component: Component::SebiFee,
    rate: RowRate::Ratio(per_crore(5_00)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2020 - 6 - 1),
    from_note: None,
    to: Some(boundary!(2021 - 3 - 31)),
    clocks: DATED,
    source: "SEBI/LAD-NRO/GN/2020/011, 8 May 2020, annexed to NSE/FA/44514; \
             https://nsearchives.nseindia.com/content/circulars/FA44514.pdf; read 2026-10-30; \
             \"pay only 50% (fifty percent) of fees as calculated therein\" — the relief is to the \
             broker; whether it reached clients is not recorded, and the today clock (Rs 10) is \
             the worse",
    status: Status::Cited,
};

/// SF-2021: SEBI's fee back at Rs 10 per crore.
pub const SF_2021: DatedRow = DatedRow {
    id: "SF-2021",
    component: Component::SebiFee,
    rate: RowRate::Ratio(per_crore(10_00)),
    base: Base::Turnover,
    side: Side::BothSides,
    from: boundary!(2021 - 4 - 1),
    from_note: None,
    to: None,
    clocks: BOTH,
    source: "NSE levies page (updated 17/04/2026); https://www.nseindia.com/static/invest/\
             first-time-investor-sebi-turnover-fees-stt-other-levies; read 2026-10-30; \"0.0001 \
             per cent of the price at which the securities are purchased or sold (Rs.10 per \
             crore)\"; Dhan: \"0.0001% of the turnover\"",
    status: Status::Cited,
};

/// STT-ND: STT on equity sold otherwise than by delivery.
pub const STT_ND: DatedRow = DatedRow {
    id: "STT-ND",
    component: Component::Stt,
    rate: RowRate::Ratio(pct(25, 1_000)),
    base: Base::NonDeliveryValue,
    side: Side::Sell,
    from: boundary!(2013 - 6 - 1),
    from_note: None,
    to: None,
    clocks: BOTH,
    source: "Finance Act 2013 via NSE/FATAX/23500; unchanged in FATAX/56235, 63809, 73524; \
             https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf; read 2026-10-30; \
             \"settled otherwise than by the actual delivery or transfer of such share or unit\"; \
             method NSE/CMTR/5588 (VWAP to two decimals, each security to two decimals, the \
             client's day to the rupee)",
    status: Status::Cited,
};

/// SD-STATE: stamp duty under each State's law, before 1 Jul 2020.
pub const SD_STATE: DatedRow = DatedRow {
    id: "SD-STATE",
    component: Component::StateStamp,
    rate: RowRate::StateTable,
    base: Base::ContractNoteValue,
    side: Side::BothSides,
    from: TradeDay::MIN,
    from_note: Some("in force before the window; its start is not part of this table"),
    to: Some(boundary!(2020 - 6 - 30)),
    clocks: DATED,
    source: "PIB release 30 Jun 2020 (PRID 1635399); \
             https://www.pib.gov.in/PressReleasePage.aspx?PRID=1635399; read 2026-10-30; \
             \"stamp duty was payable by both seller and buyer\"; rates SECONDARY (STATE_STAMPS); \
             the client's state is unknown, so the worst listed state is used (RD-STATE, \
             operator decision 9.5 of 30 Oct 2026)",
    status: Status::Unverified,
};

/// SD-ND: the unified 0.003% stamp duty on the buyer, from 1 Jul 2020.
pub const SD_ND: DatedRow = DatedRow {
    id: "SD-ND",
    component: Component::Stamp,
    rate: RowRate::Ratio(pct(3, 1_000)),
    base: Base::FifoBuyValue,
    side: Side::Buy,
    from: boundary!(2020 - 7 - 1),
    from_note: None,
    to: None,
    clocks: BOTH,
    source: "Indian Stamp Act 1899 as amended by Finance Act 2019; Stamp Rules 2019 (G.S.R. \
             901(E)); NCL/CMPT/43115; start 1 Jul 2020 by NCL/CMPT/44032; \
             https://nsearchives.nseindia.com/content/circulars/CMPT43115.zip; read 2026-10-30; \
             \"Transfer of Security other than debenture on non-delivery basis 0.003% Buyer\"",
    status: Status::Cited,
};

/// GST-1: GST at 18% on brokerage and the exchange charges.
pub const GST_1: DatedRow = DatedRow {
    id: "GST-1",
    component: Component::Gst,
    rate: RowRate::Ratio(Ratio::new(18, 100)),
    base: Base::GstBase,
    side: Side::BothSides,
    from: boundary!(2017 - 7 - 1),
    from_note: None,
    to: None,
    clocks: BOTH,
    source: "Notification 11/2017-Central Tax (Rate), Sl. 15 heading 9971 (v), 9% central tax; \
             https://gstcouncil.gov.in/sites/default/files/2024-05/download_4.pdf; read \
             2026-10-30; \"This notification shall come into force with effect from 1st day of \
             July, 2017.\"; NSE levies page \"Stock brokers Services 18.00% (GST)\"",
    status: Status::Cited,
};

/// GST-SEBI: GST at 18% on the SEBI fee, from 18 Jul 2022.
pub const GST_SEBI: DatedRow = DatedRow {
    id: "GST-SEBI",
    component: Component::GstOnSebi,
    rate: RowRate::Ratio(Ratio::new(18, 100)),
    base: Base::SebiFee,
    side: Side::BothSides,
    from: boundary!(2022 - 7 - 18),
    from_note: None,
    to: None,
    clocks: BOTH,
    source: "NSE/FATAX/52993, 18 Jul 2022 (Notification 04/2022-Central Tax (Rate)); \
             https://nsearchives.nseindia.com/content/circulars/FATAX52993.pdf; read 2026-10-30; \
             \"levy GST @ 18% on the fees payable to SEBI, from their clients from July 18, \
             2022\"",
    status: Status::Cited,
};

/// OT-CLR: a clearing charge — none found.
pub const OT_CLR: DatedRow = DatedRow {
    id: "OT-CLR",
    component: Component::Clearing,
    rate: RowRate::NoneFound,
    base: Base::NotApplicable,
    side: Side::NotApplicable,
    from: TradeDay::MIN,
    from_note: Some("none found, so no dates"),
    to: None,
    clocks: BOTH,
    source: "NSE/FA/64232 and FA73061 list only transaction charges and IPFT; Dhan's page lists \
             none; subject search of NSE circulars 2008-2026; \
             https://nsearchives.nseindia.com/content/circulars/FA73061.pdf; read 2026-10-30; \
             \"levy the transaction charges and contribution to NSE IPFT\" — absence is not proof",
    status: Status::Unverified,
};

/// OT-INT: interest on intraday leverage — none on a squared-off day.
pub const OT_INT: DatedRow = DatedRow {
    id: "OT-INT",
    component: Component::LeverageInterest,
    rate: RowRate::NoneFound,
    base: Base::NotApplicable,
    side: Side::NotApplicable,
    from: TradeDay::MIN,
    from_note: Some("none found, so no dates"),
    to: None,
    clocks: TODAY,
    source: "Dhan pricing page lists interest only on non-MTF debit and 50:50 shortfall; \
             https://dhan.co/pricing/; read 2026-10-30; \"0.0438% per day on outstanding non MTF \
             debit\" — none for a position squared off the same day (inference)",
    status: Status::Unverified,
};

/// Every row of `costs-table.json` version [`EQUITY_DATED_VERSION`], in its
/// order.
pub const EQUITY_DATED_ROWS: [DatedRow; 25] = [
    BRK_1, BRK_SQ, BRK_DP, BRK_CT, EX_2009, EX_2020C, EX_2021, EX_2023, EX_2024A, EX_2024B,
    EX_2026, IP_OLD, IP_2023, IP_2026, SF_2017, SF_2019, SF_2020, SF_2021, STT_ND, SD_STATE, SD_ND,
    GST_1, GST_SEBI, OT_CLR, OT_INT,
];

// ---------------------------------------------------------------------------
// The series: one per dated component, anchor first
// ---------------------------------------------------------------------------

/// NSE's transaction charge after [`EX_2009`].
const NSE_LATER: [&DatedRow; 5] = [&EX_2021, &EX_2023, &EX_2024A, &EX_2024B, &EX_2026];
/// The IPFT after [`IP_OLD`].
const IPFT_LATER: [&DatedRow; 2] = [&IP_2023, &IP_2026];
/// SEBI's fee after [`SF_2017`].
const SEBI_LATER: [&DatedRow; 3] = [&SF_2019, &SF_2020, &SF_2021];
/// Stamp duty after [`SD_STATE`].
const STAMP_LATER: [&DatedRow; 1] = [&SD_ND];

/// Whether `next` starts the day after `prev` ends.
#[must_use]
pub const fn follows(prev: &DatedRow, next: &DatedRow) -> bool {
    match prev.to {
        Some(last) => last.ordinal() + 1 == next.from.ordinal(),
        None => false,
    }
}

/// Whether a series is contiguous, anchored on or before [`WINDOW_START`],
/// and open after its last row: what makes the lookup total over the window.
#[must_use]
pub const fn is_series(anchor: &DatedRow, later: &[&DatedRow]) -> bool {
    if WINDOW_START.before(anchor.from) {
        return false;
    }
    let mut prev = anchor;
    let mut rest = later;
    while let [next, tail @ ..] = rest {
        if !follows(prev, next) {
            return false;
        }
        prev = next;
        rest = tail;
    }
    prev.to.is_none()
}

const _: () = assert!(is_series(&EX_2009, &NSE_LATER));
const _: () = assert!(is_series(&IP_OLD, &IPFT_LATER));
const _: () = assert!(is_series(&SF_2017, &SEBI_LATER));
const _: () = assert!(is_series(&SD_STATE, &STAMP_LATER));
const _: () = assert!(WINDOW_START.before(READ_ON));
const _: () = assert!(READ_ON.ordinal() + 1 == AFTER_READ_ON.ordinal());

/// Whether every per-crore row shares [`PER_CRORE`], so the transaction
/// charge and the IPFT add as numerators, and every rate is well-shaped.
#[must_use]
pub const fn is_per_crore(row: &DatedRow) -> bool {
    let ratio = row.rate.ratio();
    ratio.is_well_shaped() && ratio.denominator() == PER_CRORE
}

const _: () = assert!(is_per_crore(&EX_2009));
const _: () = assert!(is_per_crore(&EX_2021));
const _: () = assert!(is_per_crore(&EX_2023));
const _: () = assert!(is_per_crore(&EX_2024A));
const _: () = assert!(is_per_crore(&EX_2024B));
const _: () = assert!(is_per_crore(&EX_2026));
const _: () = assert!(is_per_crore(&IP_OLD));
const _: () = assert!(is_per_crore(&IP_2023));
const _: () = assert!(is_per_crore(&IP_2026));
const _: () = assert!(is_per_crore(&SF_2017));
const _: () = assert!(is_per_crore(&SF_2019));
const _: () = assert!(is_per_crore(&SF_2020));
const _: () = assert!(is_per_crore(&SF_2021));

/// The row of a series in force on `day`: the anchor, or the last later row
/// that has started. Total by construction ([`is_series`]); the walk is the
/// array's fixed length.
fn in_force(
    anchor: &'static DatedRow,
    later: &[&'static DatedRow],
    day: TradeDay,
) -> &'static DatedRow {
    let mut pick = anchor;
    for &row in later {
        if !day.before(row.from) {
            pick = row;
        }
    }
    pick
}

// ---------------------------------------------------------------------------
// The rates one clock prices a day at
// ---------------------------------------------------------------------------

/// The row of every priced component, as one clock reads it on one day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EquityRates {
    /// Dhan's brokerage ([`BRK_1`]).
    pub brokerage: &'static DatedRow,
    /// Dhan's auto square-off fee ([`BRK_SQ`]).
    pub square_off: &'static DatedRow,
    /// The DP charge ([`BRK_DP`]).
    pub dp: &'static DatedRow,
    /// NSE's transaction charge.
    pub nse: &'static DatedRow,
    /// The IPFT contribution.
    pub ipft: &'static DatedRow,
    /// SEBI's turnover fee.
    pub sebi: &'static DatedRow,
    /// STT ([`STT_ND`]).
    pub stt: &'static DatedRow,
    /// Stamp duty: [`SD_STATE`] before 1 Jul 2020, [`SD_ND`] from it.
    pub stamp: &'static DatedRow,
    /// GST ([`GST_1`]).
    pub gst: &'static DatedRow,
    /// GST on the SEBI fee ([`GST_SEBI`]) from 18 Jul 2022, `None` before.
    pub gst_on_sebi: Option<&'static DatedRow>,
}

impl EquityRates {
    /// NSE's transaction charge and the IPFT as one exact rate, the way the
    /// contract note's one exchange line charges them.
    #[must_use]
    pub const fn exchange_rate(&self) -> Ratio {
        Ratio::new(
            self.nse.rate.ratio().numerator() + self.ipft.rate.ratio().numerator(),
            PER_CRORE,
        )
    }

    /// Whether stamp duty is the State levy on both sides (before 1 Jul 2020).
    #[must_use]
    pub const fn stamp_is_state(&self) -> bool {
        matches!(self.stamp.component, Component::StateStamp)
    }

    /// The ids of the rows priced, in a fixed order, `-` for no GST on the
    /// SEBI fee: what a run records beside each trade.
    #[must_use]
    pub const fn row_ids(&self) -> [&'static str; 10] {
        let gst_on_sebi = match self.gst_on_sebi {
            Some(row) => row.id,
            None => "-",
        };
        [
            self.brokerage.id,
            self.square_off.id,
            self.dp.id,
            self.nse.id,
            self.ipft.id,
            self.sebi.id,
            self.stt.id,
            self.stamp.id,
            self.gst.id,
            gst_on_sebi,
        ]
    }
}

/// The rates in force on `day`, without the window check: every day selects
/// a row of every series ([`is_series`]).
pub(crate) fn in_force_on(day: TradeDay) -> EquityRates {
    EquityRates {
        brokerage: &BRK_1,
        square_off: &BRK_SQ,
        dp: &BRK_DP,
        nse: in_force(&EX_2009, &NSE_LATER, day),
        ipft: in_force(&IP_OLD, &IPFT_LATER, day),
        sebi: in_force(&SF_2017, &SEBI_LATER, day),
        stt: &STT_ND,
        stamp: in_force(&SD_STATE, &STAMP_LATER, day),
        gst: &GST_1,
        gst_on_sebi: if day.before(GST_SEBI.from) {
            None
        } else {
            Some(&GST_SEBI)
        },
    }
}

/// Dhan's rates as read on [`READ_ON`], applied to any trade day: the today
/// clock. Every row here serves the today clock
/// (`the_today_clock_is_the_rows_that_serve_it`).
pub const TODAY_RATES: EquityRates = EquityRates {
    brokerage: &BRK_1,
    square_off: &BRK_SQ,
    dp: &BRK_DP,
    nse: &EX_2026,
    ipft: &IP_2026,
    sebi: &SF_2021,
    stt: &STT_ND,
    stamp: &SD_ND,
    gst: &GST_1,
    gst_on_sebi: Some(&GST_SEBI),
};

/// What the refusal of a day outside the window names.
const TABLE_CHARGE: &str = "NSE cash-equity intraday charge table (costs-table/2026-10-30/r1)";

/// Why a day before the window is refused.
const BEFORE_WINDOW: &str = "the operator's window starts on 2018-09-03 (requirement of \
     30 Oct 2026); the table's rows before it (IP-OLD's start, SD-STATE's states) were never \
     verified for an earlier day";

/// Why a day after the read day is refused.
const AFTER_READ: &str = "every source of the table was read on 2026-10-30; no rate after it is \
     verified, and a circular dated later could change any row";

/// How to close either refusal.
const TABLE_REMEDY: &str = "re-read every source of costs-table.json for the new day, add the \
     dated rows to crates/costs/src/equity_dated.rs and move EQUITY_DATED_VERSION, with a \
     docs/05-decisions.md entry";

/// Refuses a day outside [`WINDOW_START`]..=[`READ_ON`] by name.
///
/// # Errors
///
/// [`ChargeError::Cost`] carrying a [`Refusal`] that names the window.
pub fn check_window(day: TradeDay) -> Result<(), ChargeError> {
    if day.before(WINDOW_START) {
        return Err(Refusal::with_remediation(
            TABLE_CHARGE,
            Some(Exchange::Nse),
            day,
            TradeDay::MIN,
            Some(WINDOW_START),
            BEFORE_WINDOW,
            TABLE_REMEDY,
        )
        .into());
    }
    if READ_ON.before(day) {
        return Err(Refusal::with_remediation(
            TABLE_CHARGE,
            Some(Exchange::Nse),
            day,
            AFTER_READ_ON,
            None,
            AFTER_READ,
            TABLE_REMEDY,
        )
        .into());
    }
    Ok(())
}

/// The rates in force on `day`: the dated clock.
///
/// # Errors
///
/// [`ChargeError::Cost`] carrying a [`Refusal`] for a day before
/// [`WINDOW_START`] or after [`READ_ON`] ([`check_window`]).
///
/// # Examples
///
/// ```
/// use costs::day::TradeDay;
/// use costs::equity_dated::rates_on;
///
/// let rates = rates_on(TradeDay::new(2021, 1, 1)?)?;
/// assert_eq!(rates.nse.id, "EX-2021");
/// assert_eq!(rates.sebi.id, "SF-2020");
/// assert!(rates.gst_on_sebi.is_none());
/// assert!(rates_on(TradeDay::new(2026, 10, 31)?).is_err());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn rates_on(day: TradeDay) -> Result<EquityRates, ChargeError> {
    check_window(day)?;
    Ok(in_force_on(day))
}

// ---------------------------------------------------------------------------
// Stamp duty under State law, before 1 Jul 2020 (SECONDARY)
// ---------------------------------------------------------------------------

/// Where the State rates were read: a broker's archived calculator, so
/// SECONDARY.
pub const STATE_STAMP_SOURCE: &str = "Zerodha's archived calculator script \
     http://web.archive.org/web/20190403072919js_/https://zerodha.com/static/app.js?v=7.0.7 \
     (EQ Intraday column; state codes from the 2018-07-14 snapshot of \
     zerodha.com/brokerage-calculator); read 2026-10-30; SECONDARY; extracts in \
     state/fib-strategy-20261030/costs-evidence/zerodha-archived-state-*.txt";

/// How a State levies stamp duty on an intraday contract note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateLevy {
    /// A rate of the contract-note value, with an optional cap.
    Rate {
        /// The rate.
        rate: Ratio,
        /// The cap, if any.
        cap: Option<Paisa>,
        /// Whether the source states the cap per day (else per note).
        cap_per_day: bool,
    },
    /// A fixed amount a day.
    FlatPerDay(Paisa),
}

/// One row of the State table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateStamp {
    /// The calculator's code for the row.
    pub code: &'static str,
    /// The States the row covers.
    pub states: &'static str,
    /// The levy.
    pub levy: StateLevy,
}

/// A State rate with no cap.
const fn uncapped(code: &'static str, states: &'static str, rate: Ratio) -> StateStamp {
    StateStamp {
        code,
        states,
        levy: StateLevy::Rate {
            rate,
            cap: None,
            cap_per_day: false,
        },
    }
}

/// A State rate with a cap.
const fn capped(
    code: &'static str,
    states: &'static str,
    rate: Ratio,
    cap: i64,
    cap_per_day: bool,
) -> StateStamp {
    StateStamp {
        code,
        states,
        levy: StateLevy::Rate {
            rate,
            cap: Some(Paisa::from_raw(cap)),
            cap_per_day,
        },
    }
}

/// The 15 rows of the State table, in `costs-table.json`'s order.
pub const STATE_STAMPS: [StateStamp; 15] = [
    capped(
        "AP_OD",
        "Andhra Pradesh, Odisha",
        pct(5, 1_000),
        50_00,
        false,
    ),
    uncapped("AR", "Arunachal Pradesh", pct(3, 1_000)),
    capped("AS", "Assam", pct(18, 1_000), 49_50, false),
    uncapped(
        "DL_GJ_KL_MH",
        "Delhi, Gujarat, Kerala, Maharashtra",
        pct(2, 1_000),
    ),
    uncapped("GA", "Goa", pct(5, 1_000)),
    capped("HR", "Haryana", pct(2, 1_000), 20_000, false),
    StateStamp {
        code: "HP",
        states: "Himachal Pradesh",
        levy: StateLevy::FlatPerDay(Paisa::from_raw(50_00)),
    },
    uncapped("KA_ML_OT", "Karnataka, Meghalaya, others", pct(3, 1_000)),
    uncapped("MP", "Madhya Pradesh", pct(2, 1_000)),
    uncapped("RJ", "Rajasthan", pct(3, 1_000)),
    uncapped("TN", "Tamil Nadu", pct(6, 1_000)),
    capped("UP", "Uttar Pradesh", pct(2, 1_000), 100_000, true),
    uncapped("WB", "West Bengal", pct(2, 1_000)),
    capped("TS", "Telangana", pct(1, 100), 10_000, true),
    uncapped("JK", "Jammu and Kashmir", pct(3, 1_000)),
];

/// `numerator / denominator`, half-up, for a positive denominator: the
/// rounding of every charge line (`crate::exact`). Every shipped ratio's
/// denominator is positive ([`is_per_crore`], `Ratio::is_well_shaped`).
pub(crate) fn half_up(numerator: i128, denominator: i128) -> i128 {
    let quotient = numerator.div_euclid(denominator);
    if numerator.rem_euclid(denominator) * 2 >= denominator {
        quotient + 1
    } else {
        quotient
    }
}

/// `value × rate`, half-up to the paisa, in `i128`. A value is at most a
/// few times `i64::MAX` paise and a numerator under 2^31, so the product
/// stays far inside `i128`.
pub(crate) fn levy(value: i128, rate: Ratio) -> i128 {
    half_up(
        value * i128::from(rate.numerator()),
        i128::from(rate.denominator()),
    )
}

impl StateStamp {
    /// This State's stamp duty on a contract-note value, half-up to the
    /// paisa and capped, in `i128` paise.
    pub(crate) fn levy_on(&self, value: i128) -> i128 {
        match self.levy {
            StateLevy::FlatPerDay(amount) => i128::from(amount.raw()),
            StateLevy::Rate { rate, cap, .. } => {
                let duty = levy(value, rate);
                match cap {
                    Some(cap) => duty.min(i128::from(cap.raw())),
                    None => duty,
                }
            }
        }
    }

    /// This State's stamp duty on `value`, half-up to the paisa and capped.
    ///
    /// The duty is a fraction of the value under one, or a fixed amount, so
    /// it is never larger in magnitude than the value or the fixed amount, and
    /// always fits.
    ///
    /// # Examples
    ///
    /// ```
    /// use brutex_core::price::Paisa;
    /// use costs::equity_dated::STATE_STAMPS;
    ///
    /// let telangana = STATE_STAMPS[13];
    /// assert_eq!(telangana.code, "TS");
    /// assert_eq!(telangana.on(Paisa::from_raw(994_643_65)).raw(), 99_46);
    /// assert_eq!(telangana.on(Paisa::from_raw(1_005_000_00)).raw(), 100_00); // capped
    /// ```
    #[must_use]
    pub fn on(&self, value: Paisa) -> Paisa {
        narrow_fitting(self.levy_on(i128::from(value.raw())))
    }
}

/// An `i128` the caller has bounded inside `i64`, as [`Paisa`]. Every caller
/// passes a fraction under one of an `i64` value, a fixed amount, or a sum
/// bounded in `crate::equity_readings` (`MAX_FILLS`); the saturation is the
/// bound's statement, not a fallback any input reaches.
pub(crate) fn narrow_fitting(value: i128) -> Paisa {
    Paisa::from_raw(i64::try_from(value).unwrap_or(if value < 0 { i64::MIN } else { i64::MAX }))
}

/// The State whose stamp duty on a day's contract-note value is the highest,
/// and that duty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateCharge {
    /// The State row.
    pub state: &'static StateStamp,
    /// Its duty, in paise.
    pub amount: Paisa,
}

/// The worst listed State for one day's contract-note value (RD-STATE): the
/// highest duty of the 15 rows, the first in table order on a tie.
///
/// Below Rs 5 lakh a day that is Himachal Pradesh's fixed Rs 50; up to
/// Rs 16.67 lakh, Telangana's 0.01% capped at Rs 100; above it, Tamil Nadu's
/// 0.006% (D-8114). The walk is the table's fixed 15 rows.
///
/// # Examples
///
/// ```
/// use brutex_core::price::Paisa;
/// use costs::equity_dated::worst_state_stamp;
///
/// assert_eq!(worst_state_stamp(Paisa::from_raw(200_000_00)).state.code, "HP");
/// assert_eq!(worst_state_stamp(Paisa::from_raw(1_005_000_00)).state.code, "TS");
/// assert_eq!(worst_state_stamp(Paisa::from_raw(2_000_000_00)).state.code, "TN");
/// ```
#[must_use]
pub fn worst_state_stamp(value: Paisa) -> StateCharge {
    let (state, amount) = worst_state_units(i128::from(value.raw()));
    StateCharge {
        state,
        amount: narrow_fitting(amount),
    }
}

/// [`worst_state_stamp`] in `i128` paise.
pub(crate) fn worst_state_units(value: i128) -> (&'static StateStamp, i128) {
    let [first, rest @ ..] = &STATE_STAMPS;
    let mut worst = (first, first.levy_on(value));
    for state in rest {
        let duty = state.levy_on(value);
        if duty > worst.1 {
            worst = (state, duty);
        }
    }
    worst
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::inconsistent_digit_grouping
)]
mod tests {
    use super::*;

    fn day(year: u16, month: u8, d: u8) -> TradeDay {
        TradeDay::new(year, month, d).expect("a real date")
    }

    /// Paise per crore of a row's rate.
    fn per_crore_of(row: &DatedRow) -> i64 {
        let ratio = row.rate.ratio();
        assert_eq!(ratio.denominator(), PER_CRORE, "{}", row.id);
        ratio.numerator()
    }

    #[test]
    fn the_table_is_costs_table_json_row_for_row() {
        // The ids of costs-table.json r1, in its order (jq -r '.rows[].id').
        let ids = [
            "BRK-1", "BRK-SQ", "BRK-DP", "BRK-CT", "EX-2009", "EX-2020C", "EX-2021", "EX-2023",
            "EX-2024A", "EX-2024B", "EX-2026", "IP-OLD", "IP-2023", "IP-2026", "SF-2017",
            "SF-2019", "SF-2020", "SF-2021", "STT-ND", "SD-STATE", "SD-ND", "GST-1", "GST-SEBI",
            "OT-CLR", "OT-INT",
        ];
        assert_eq!(EQUITY_DATED_ROWS.map(|row| row.id), ids);
        // Rates per crore, in paise, exactly as the table's rate_per_crore.
        let per_crore_rows = [
            (EX_2009, 325_00),
            (EX_2020C, 275_00),
            (EX_2021, 345_00),
            (EX_2023, 325_00),
            (EX_2024A, 322_00),
            (EX_2024B, 297_00),
            (EX_2026, 306_99),
            (IP_OLD, 1),
            (IP_2023, 10_00),
            (IP_2026, 1),
            (SF_2017, 15_00),
            (SF_2019, 10_00),
            (SF_2020, 5_00),
            (SF_2021, 10_00),
        ];
        for (row, paise) in per_crore_rows {
            assert_eq!(per_crore_of(&row), paise, "{}", row.id);
        }
        // The percentage rows and the flat fees.
        assert_eq!(STT_ND.rate.ratio(), Ratio::new(25, 100_000));
        assert_eq!(SD_ND.rate.ratio(), Ratio::new(3, 100_000));
        assert_eq!(GST_1.rate.ratio(), Ratio::new(18, 100));
        assert_eq!(GST_SEBI.rate.ratio(), Ratio::new(18, 100));
        assert_eq!(
            BRK_1.rate,
            RowRate::Capped {
                ratio: Ratio::new(3, 10_000),
                cap: Paisa::from_raw(20_00)
            }
        );
        assert_eq!(BRK_SQ.rate, RowRate::Flat(Paisa::from_raw(20_00)));
        assert_eq!(BRK_DP.rate, RowRate::Flat(Paisa::from_raw(12_50)));
        assert_eq!(BRK_CT.rate, RowRate::Flat(Paisa::from_raw(50_00)));
        assert_eq!(SD_STATE.rate, RowRate::StateTable);
        assert_eq!(OT_CLR.rate, RowRate::NoneFound);
        assert_eq!(OT_INT.rate, RowRate::NoneFound);
        // Every row names a source, and the two rows that are not primary
        // and the two that found nothing say so.
        for row in EQUITY_DATED_ROWS {
            assert!(row.source.contains("read 2026-10-30"), "{}", row.id);
            let unverified = matches!(row.id, "SD-STATE" | "OT-CLR" | "OT-INT");
            assert_eq!(row.status == Status::Unverified, unverified, "{}", row.id);
            // A row with no dated start says why.
            assert_eq!(
                row.from == TradeDay::MIN,
                row.from_note.is_some(),
                "{}",
                row.id
            );
        }
    }

    #[test]
    fn the_const_builders_build_what_the_rows_say() {
        // The tables call these at compile time only, so run them here too.
        assert_eq!(per_crore(32_500), EX_2009.rate.ratio());
        assert_eq!(pct(25, 1_000), STT_ND.rate.ratio());
        assert_eq!(pct(3, 100), BROKERAGE_RATE);
        assert_eq!(
            uncapped("TN", "Tamil Nadu", pct(6, 1_000)),
            STATE_STAMPS[10]
        );
        assert_eq!(
            capped("TS", "Telangana", pct(1, 100), 100_00, true),
            STATE_STAMPS[13]
        );
    }

    #[test]
    fn every_row_shape_reads_as_one_exact_fraction() {
        assert_eq!(BRK_1.rate.ratio(), Ratio::new(3, 10_000));
        assert_eq!(EX_2026.rate.ratio(), Ratio::new(306_99, PER_CRORE));
        assert_eq!(BRK_SQ.rate.ratio(), Ratio::new(20_00, 1));
        assert_eq!(SD_STATE.rate.ratio(), Ratio::new(0, 1));
        assert_eq!(OT_CLR.rate.ratio(), Ratio::new(0, 1));
        // The compile-time checks, run: every per-crore row passes, and a
        // percentage row or a malformed one does not.
        assert!(is_per_crore(&EX_2009));
        assert!(is_per_crore(&IP_2026));
        assert!(!is_per_crore(&STT_ND));
        let malformed = DatedRow {
            rate: RowRate::Ratio(Ratio::new(1, 0)),
            ..EX_2009
        };
        assert!(!is_per_crore(&malformed));
    }

    #[test]
    fn every_series_is_contiguous_anchored_and_open_and_a_broken_one_is_not() {
        assert!(is_series(&EX_2009, &NSE_LATER));
        assert!(is_series(&IP_OLD, &IPFT_LATER));
        assert!(is_series(&SF_2017, &SEBI_LATER));
        assert!(is_series(&SD_STATE, &STAMP_LATER));
        assert!(follows(&EX_2024B, &EX_2026));
        // A gap, an overlap, an open row followed by another, a late anchor
        // and a closed last row are each refused.
        assert!(!follows(&EX_2009, &EX_2023));
        assert!(!follows(&EX_2020C, &EX_2021));
        assert!(!follows(&EX_2026, &EX_2026));
        assert!(!is_series(&EX_2021, &[&EX_2023]));
        assert!(!is_series(&EX_2009, &[&EX_2023]));
        assert!(!is_series(&EX_2009, &[&EX_2021]));
        assert!(!is_series(&EX_2026, &[]));
        assert!(is_series(&STT_ND, &[]));
    }

    #[test]
    fn a_row_covers_its_own_days_and_no_other() {
        assert!(EX_2021.covers(day(2021, 1, 1)));
        assert!(EX_2021.covers(day(2023, 3, 31)));
        assert!(!EX_2021.covers(day(2020, 12, 31)));
        assert!(!EX_2021.covers(day(2023, 4, 1)));
        assert!(EX_2026.covers(TradeDay::MAX));
        assert!(BRK_1.covers(TradeDay::MIN));
    }

    #[test]
    fn every_day_of_the_window_reads_exactly_the_row_that_covers_it() {
        // Every calendar day from the window's first to the read day: each
        // dated component's row is the one row of its series covering the
        // day, and no other row of that component covers it.
        let mut on = WINDOW_START;
        let mut days = 0;
        while !READ_ON.before(on) {
            let rates = rates_on(on).expect("inside the window");
            for (component, row) in [
                (Component::NseTransaction, rates.nse),
                (Component::Ipft, rates.ipft),
                (Component::SebiFee, rates.sebi),
            ] {
                assert!(row.covers(on), "{} on {on}", row.id);
                let covering: Vec<&str> = EQUITY_DATED_ROWS
                    .iter()
                    .filter(|r| r.component == component && r.covers(on))
                    .map(|r| r.id)
                    .collect();
                assert_eq!(covering, [row.id], "{on}");
            }
            assert!(rates.stamp.covers(on));
            assert_eq!(rates.gst_on_sebi.is_some(), !on.before(GST_SEBI.from));
            on = on.plus_days(1).expect("inside the calendar");
            days += 1;
        }
        // 2018-09-03 to 2026-10-30 inclusive.
        assert_eq!(days, 2_980);
    }

    #[test]
    fn each_rate_change_is_read_on_the_day_before_and_the_day_itself() {
        let ids = |y, m, d| {
            let rates = rates_on(day(y, m, d)).expect("inside the window");
            (
                rates.nse.id,
                rates.ipft.id,
                rates.sebi.id,
                rates.stamp.id,
                rates.gst_on_sebi.map(|row| row.id),
            )
        };
        // The window's first day.
        assert_eq!(
            ids(2018, 9, 3),
            ("EX-2009", "IP-OLD", "SF-2017", "SD-STATE", None)
        );
        // SEBI 15 -> 10.
        assert_eq!(ids(2019, 3, 31).2, "SF-2017");
        assert_eq!(ids(2019, 4, 1).2, "SF-2019");
        // SEBI relief from 2020-06-01, to 2021-03-31.
        assert_eq!(ids(2020, 5, 31).2, "SF-2019");
        assert_eq!(ids(2020, 6, 1).2, "SF-2020");
        assert_eq!(ids(2021, 3, 31).2, "SF-2020");
        assert_eq!(ids(2021, 4, 1).2, "SF-2021");
        // Stamp: State law to 2020-06-30, unified from 2020-07-01.
        assert_eq!(ids(2020, 6, 30).3, "SD-STATE");
        assert_eq!(ids(2020, 7, 1).3, "SD-ND");
        // NSE 325 -> 345.
        assert_eq!(ids(2020, 12, 31).0, "EX-2009");
        assert_eq!(ids(2021, 1, 1).0, "EX-2021");
        // GST on the SEBI fee from 2022-07-18.
        assert_eq!(ids(2022, 7, 17).4, None);
        assert_eq!(ids(2022, 7, 18).4, Some("GST-SEBI"));
        // NSE 345 + 0.01 -> 325 + 10.
        assert_eq!(
            (ids(2023, 3, 31).0, ids(2023, 3, 31).1),
            ("EX-2021", "IP-OLD")
        );
        assert_eq!(
            (ids(2023, 4, 1).0, ids(2023, 4, 1).1),
            ("EX-2023", "IP-2023")
        );
        // 325 -> 322.
        assert_eq!(ids(2024, 3, 31).0, "EX-2023");
        assert_eq!(ids(2024, 4, 1).0, "EX-2024A");
        // Slabs -> flat.
        assert_eq!(ids(2024, 9, 30).0, "EX-2024A");
        assert_eq!(ids(2024, 10, 1).0, "EX-2024B");
        // 297 + 10 -> 306.99 + 0.01.
        assert_eq!(
            (ids(2026, 2, 28).0, ids(2026, 2, 28).1),
            ("EX-2024B", "IP-2023")
        );
        assert_eq!(
            (ids(2026, 3, 1).0, ids(2026, 3, 1).1),
            ("EX-2026", "IP-2026")
        );
        // The read day is today's set exactly.
        assert_eq!(rates_on(READ_ON).expect("the read day"), TODAY_RATES);
    }

    #[test]
    fn the_exchange_line_adds_the_transaction_charge_and_the_ipft() {
        let paise = |y, m, d| {
            rates_on(day(y, m, d))
                .expect("inside the window")
                .exchange_rate()
                .numerator()
        };
        assert_eq!(paise(2020, 12, 31), 325_01);
        assert_eq!(paise(2021, 1, 1), 345_01);
        assert_eq!(paise(2023, 4, 1), 335_00);
        assert_eq!(paise(2024, 4, 1), 332_00);
        assert_eq!(paise(2024, 10, 1), 307_00);
        assert_eq!(paise(2026, 3, 1), 307_00);
        assert_eq!(
            TODAY_RATES.exchange_rate(),
            crate::schedule::NSE_EQUITY_EXCHANGE.rate()
        );
    }

    #[test]
    fn the_today_clock_is_the_rows_that_serve_it() {
        let rows = [
            TODAY_RATES.brokerage,
            TODAY_RATES.square_off,
            TODAY_RATES.dp,
            TODAY_RATES.nse,
            TODAY_RATES.ipft,
            TODAY_RATES.sebi,
            TODAY_RATES.stt,
            TODAY_RATES.stamp,
            TODAY_RATES.gst,
            TODAY_RATES.gst_on_sebi.expect("GST on the SEBI fee today"),
        ];
        for row in rows {
            assert!(row.clocks.today, "{}", row.id);
            assert!(row.covers(READ_ON), "{}", row.id);
        }
        // Every row that serves the today clock and prices something is one
        // of them (BRK-CT is not used; OT-CLR and OT-INT price nothing).
        for row in EQUITY_DATED_ROWS.iter().filter(|r| r.clocks.today) {
            let priced = rows.iter().any(|r| r.id == row.id);
            let unpriced = matches!(row.id, "BRK-CT" | "OT-CLR" | "OT-INT");
            assert!(priced != unpriced, "{}", row.id);
        }
        assert_eq!(
            TODAY_RATES.row_ids(),
            [
                "BRK-1", "BRK-SQ", "BRK-DP", "EX-2026", "IP-2026", "SF-2021", "STT-ND", "SD-ND",
                "GST-1", "GST-SEBI"
            ]
        );
        assert_eq!(
            rates_on(WINDOW_START).expect("the first day").row_ids()[9],
            "-"
        );
        assert!(!TODAY_RATES.stamp_is_state());
        assert!(
            rates_on(WINDOW_START)
                .expect("the first day")
                .stamp_is_state()
        );
        // Today's clock is the figures today's schedule carries.
        assert_eq!(
            STT_ND.rate.ratio(),
            crate::schedule::STT_EQUITY_NON_DELIVERY.rate()
        );
        assert_eq!(
            SD_ND.rate.ratio(),
            crate::schedule::STAMP_EQUITY_NON_DELIVERY.rate()
        );
        assert_eq!(GST_1.rate.ratio(), crate::schedule::GST.rate());
        // Rs 10 per crore both ways: 10_00 / 10^9 == 10 / 10^7.
        let sebi = crate::schedule::SEBI_FEE.rate();
        assert_eq!(
            i128::from(per_crore_of(&SF_2021)) * i128::from(sebi.denominator()),
            i128::from(sebi.numerator()) * i128::from(PER_CRORE)
        );
        assert_eq!(
            BRK_1.rate,
            RowRate::Capped {
                ratio: crate::schedule::DHAN_EQUITY_BROKERAGE_RATE.rate(),
                cap: crate::schedule::DHAN_EQUITY_BROKERAGE_CAP,
            }
        );
        assert_eq!(
            BRK_SQ.rate,
            RowRate::Flat(crate::schedule::DHAN_SQUARE_OFF_FEE)
        );
        assert_eq!(BRK_DP.rate, RowRate::Flat(crate::schedule::DHAN_DP_CHARGE));
    }

    #[test]
    fn a_day_outside_the_window_is_refused_by_name() {
        let before = rates_on(day(2018, 9, 2)).expect_err("before the window");
        assert_eq!(
            before,
            ChargeError::from(Refusal::with_remediation(
                TABLE_CHARGE,
                Some(Exchange::Nse),
                day(2018, 9, 2),
                TradeDay::MIN,
                Some(WINDOW_START),
                BEFORE_WINDOW,
                TABLE_REMEDY,
            ))
        );
        assert!(before.to_string().contains("2018-09-03"));
        let after = rates_on(day(2026, 10, 31)).expect_err("after the read day");
        assert_eq!(
            after,
            ChargeError::from(Refusal::with_remediation(
                TABLE_CHARGE,
                Some(Exchange::Nse),
                day(2026, 10, 31),
                AFTER_READ_ON,
                None,
                AFTER_READ,
                TABLE_REMEDY,
            ))
        );
        assert!(after.to_string().contains("2026-10-30"));
        assert!(rates_on(TradeDay::MIN).is_err());
        assert!(rates_on(TradeDay::MAX).is_err());
        assert!(rates_on(WINDOW_START).is_ok());
        assert!(rates_on(READ_ON).is_ok());
    }

    #[test]
    fn the_worst_state_is_the_highest_duty_of_the_fifteen() {
        let worst = |rupees: i64| {
            let charge = worst_state_stamp(Paisa::from_raw(rupees * 100));
            (charge.state.code, charge.amount.raw())
        };
        // Below Rs 5 lakh: Himachal Pradesh's flat Rs 50 (Assam's cap is
        // Rs 49.50).
        assert_eq!(worst(1), ("HP", 50_00));
        assert_eq!(worst(4_99_999), ("HP", 50_00));
        // At Rs 5 lakh Telangana's 0.01% equals Rs 50: the first in table
        // order is named.
        assert_eq!(worst(5_00_000), ("HP", 50_00));
        assert_eq!(worst(5_00_001), ("HP", 50_00));
        assert_eq!(worst(5_00_050), ("TS", 50_01));
        assert_eq!(worst(9_94_643), ("TS", 99_46));
        // Telangana's cap, then Tamil Nadu from about Rs 16.67 lakh (Tamil
        // Nadu comes first in the table, so it is named on the tie).
        assert_eq!(worst(10_05_000), ("TS", 100_00));
        assert_eq!(worst(16_66_583), ("TS", 100_00));
        assert_eq!(worst(16_66_584), ("TN", 100_00));
        assert_eq!(worst(20_00_000), ("TN", 120_00));
        // Every State, once: the levy as its row states it.
        let at = Paisa::from_raw(10_00_000_00);
        let levies: Vec<i64> = STATE_STAMPS.iter().map(|s| s.on(at).raw()).collect();
        assert_eq!(
            levies,
            [
                50_00, 30_00, 49_50, 20_00, 50_00, 20_00, 50_00, 30_00, 20_00, 30_00, 60_00, 20_00,
                20_00, 100_00, 30_00
            ]
        );
        // Under the caps: Assam, Uttar Pradesh and Haryana uncapped.
        assert_eq!(
            STATE_STAMPS[2].on(Paisa::from_raw(1_00_000_00)).raw(),
            18_00
        );
        assert_eq!(
            STATE_STAMPS[11].on(Paisa::from_raw(1_00_00_000_00)).raw(),
            200_00
        );
        assert_eq!(
            STATE_STAMPS[11].on(Paisa::from_raw(100_00_00_000_00)).raw(),
            1_000_00
        );
        assert!(STATE_STAMPS.iter().all(|s| !s.states.is_empty()));
        assert!(STATE_STAMP_SOURCE.contains("SECONDARY"));
        // The labelled cap scopes.
        assert!(matches!(
            STATE_STAMPS[13].levy,
            StateLevy::Rate {
                cap_per_day: true,
                ..
            }
        ));
    }

    #[test]
    fn the_rounding_and_the_narrowing_are_the_crates() {
        assert_eq!(half_up(5, 2), 3);
        assert_eq!(half_up(4, 2), 2);
        assert_eq!(half_up(7, 3), 2);
        assert_eq!(levy(1_005_000_00, Ratio::new(15_00, PER_CRORE)), 1_51);
        assert_eq!(narrow_fitting(42).raw(), 42);
        assert_eq!(narrow_fitting(i128::MAX).raw(), i64::MAX);
        assert_eq!(narrow_fitting(i128::MIN).raw(), i64::MIN);
    }

    /// FNV-1a over a stream of integers and bytes.
    fn fold(hash: u64, bytes: &[u8]) -> u64 {
        bytes.iter().fold(hash, |h, byte| {
            (h ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        })
    }

    fn table_fingerprint() -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325;
        for row in EQUITY_DATED_ROWS {
            hash = fold(hash, row.id.as_bytes());
            let ratio = row.rate.ratio();
            hash = fold(hash, &ratio.numerator().to_le_bytes());
            hash = fold(hash, &ratio.denominator().to_le_bytes());
            hash = fold(hash, &row.from.ordinal().to_le_bytes());
            hash = fold(hash, &row.to.map_or(-1, TradeDay::ordinal).to_le_bytes());
            hash = fold(
                hash,
                &[u8::from(row.clocks.dated), u8::from(row.clocks.today)],
            );
        }
        for state in STATE_STAMPS {
            hash = fold(hash, state.code.as_bytes());
            hash = fold(hash, &state.levy_on(1_000_000_00).to_le_bytes());
        }
        hash
    }

    #[test]
    fn the_table_version_is_pinned_to_its_rows() {
        // Change a rate, a date or a clock and this fails until
        // EQUITY_DATED_VERSION moves and the fingerprint is re-pinned.
        assert_eq!(
            (EQUITY_DATED_VERSION, table_fingerprint()),
            ("costs-table/2026-10-30/r1", TABLE_FINGERPRINT)
        );
    }

    /// `table_fingerprint()` on r1.
    const TABLE_FINGERPRINT: u64 = 17_155_455_334_856_687_734;
}
