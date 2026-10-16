#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::inconsistent_digit_grouping,
    clippy::mistyped_literal_suffixes,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes -- a \
              test that cannot panic cannot fail; and a money literal is written \
              `rupees_paisa`, so `1_520_00` reads as the ₹1,520.00 the page showed \
              and `8_64` as ₹8.64, not as a mistyped `i64` suffix"
)]

//! THE DHAN CALCULATOR, AGAINST WHAT ITS PAGE SHOWED, AND THE PRECISE STACK BESIDE IT.
//!
//! Every case below was read from Dhan's public brokerage calculator
//! (<https://dhan.co/calculators/brokerage-calculator/>) through the page UI on
//! 15 and 16 Oct 2026, by the operator, and handed to package CHG-1. The CHG-1
//! brief says 49 cases were measured; it carries these 48, and this file holds
//! every one it carries.
//!
//! * **Every line** the page showed is asserted **exactly**.
//! * **Every total** is asserted within **one paisa**; which cases miss by a
//!   paisa, and in which direction, is pinned by name, so a last-paisa rule
//!   that fits all of them can be proven against this list (D-5112).
//! * Beside each, the **precise** contract-note figure — the default mode —
//!   is pinned too, or the named refusal that stands in its place.
//!
//! These are synthetic inputs in the sense the repository requires: prices a
//! person typed into a public calculator, not vendor market data.

use std::fmt::Write as _;

use brutex_core::instrument::Exchange;
use brutex_core::price::Paisa;
use costs::charge_error::ChargeError;
use costs::day::TradeDay;
use costs::dhan_calc::{DhanQuote, quote};
use costs::rate::OrderSide;
use costs::schedule::{Product, RateSelector, TradeKind};
use costs::stack::{ClientSecurityDay, Order};

const I: TradeKind = TradeKind::Equity(Product::Intraday);
const C: TradeKind = TradeKind::Equity(Product::Delivery);
const F: TradeKind = TradeKind::Equity(Product::Mtf);
const FUT: TradeKind = TradeKind::Futures;
const OPT: TradeKind = TradeKind::Options;
const NSE: Exchange = Exchange::Nse;
const BSE: Exchange = Exchange::Bse;

/// One page reading.
struct Case {
    name: &'static str,
    kind: TradeKind,
    exchange: Exchange,
    quantity: i64,
    buy: Option<i64>,
    sell: Option<i64>,
    /// As the page showed them, in paisa: brokerage, exchange, SEBI, STT,
    /// stamp, GST, DP, total.
    page: [i64; 8],
}

const fn case(
    name: &'static str,
    kind: TradeKind,
    exchange: Exchange,
    quantity: i64,
    buy: Option<i64>,
    sell: Option<i64>,
    page: [i64; 8],
) -> Case {
    Case {
        name,
        kind,
        exchange,
        quantity,
        buy,
        sell,
        page,
    }
}

/// The 48 readings, in the brief's order. Money in paisa.
const GOLDEN: [Case; 48] = [
    case(
        "V1 NSE I 50 1520->1530",
        I,
        NSE,
        50,
        Some(1_520_00),
        Some(1_530_00),
        [40_00, 4_68, 15, 19_00, 2_00, 8_04, 0, 74_28],
    ),
    case(
        "V2 NSE I 1000 1000->1010",
        I,
        NSE,
        1_000,
        Some(1_000_00),
        Some(1_010_00),
        [40_00, 61_71, 2_01, 253_00, 30_00, 18_31, 0, 404_53],
    ),
    case(
        "V3 NSE I 10 100->99",
        I,
        NSE,
        10,
        Some(100_00),
        Some(99_00),
        [60, 6, 0, 0, 0, 12, 0, 1_05],
    ),
    case(
        "V4 NSE I 100000 50->50.5",
        I,
        NSE,
        100_000,
        Some(50_00),
        Some(50_50),
        [40_00, 308_54, 10_05, 1_263_00, 150_00, 62_74, 0, 1_833_82],
    ),
    case(
        "V5 NSE I short 500 sell2000 buy1980",
        I,
        NSE,
        500,
        Some(1_980_00),
        Some(2_000_00),
        [40_00, 61_09, 1_99, 250_00, 30_00, 18_20, 0, 400_98],
    ),
    case(
        "D1 NSE delivery 50 1520->1530",
        C,
        NSE,
        50,
        Some(1_520_00),
        Some(1_530_00),
        [0, 4_68, 15, 153_00, 11_00, 84, 14_75, 184_33],
    ),
    case(
        "M1 NSE MTF 50 1520->1530",
        F,
        NSE,
        50,
        Some(1_520_00),
        Some(1_530_00),
        [40_00, 4_68, 15, 153_00, 11_00, 8_04, 14_75, 231_53],
    ),
    case(
        "B1 BSE I 50 1520->1530",
        I,
        BSE,
        50,
        Some(1_520_00),
        Some(1_530_00),
        [40_00, 5_80, 15, 19_00, 2_00, 8_24, 0, 75_59],
    ),
    case(
        "O1 NSE I buy only 50 @1520",
        I,
        NSE,
        50,
        Some(1_520_00),
        None,
        [20_00, 2_33, 8, 0, 2_00, 4_02, 0, 28_71],
    ),
    case(
        "FUT NIFTY OCT FUT 1 lot (65) 25200->25250",
        FUT,
        NSE,
        65,
        Some(25_200_00),
        Some(25_250_00),
        [40_00, 60_01, 3_28, 821_00, 33_00, 18_00, 0, 974_68],
    ),
    case(
        "OPT NIFTY 19 OCT 25300 CALL 1 lot (65) 100->120",
        OPT,
        NSE,
        65,
        Some(100_00),
        Some(120_00),
        [40_00, 5_08, 1, 12_00, 0, 8_11, 0, 65_11],
    ),
    case(
        "Reliance I 100 1400->1414",
        I,
        NSE,
        100,
        Some(1_400_00),
        Some(1_414_00),
        [40_00, 8_64, 28, 35_00, 4_00, 8_76, 0, 97_22],
    ),
    case(
        "TCS I 20 3000->3030",
        I,
        NSE,
        20,
        Some(3_000_00),
        Some(3_030_00),
        [36_18, 3_70, 12, 15_00, 2_00, 7_18, 0, 64_13],
    ),
    case(
        "Infosys I 200 1500->1520",
        I,
        NSE,
        200,
        Some(1_500_00),
        Some(1_520_00),
        [40_00, 18_54, 60, 76_00, 9_00, 10_54, 0, 154_69],
    ),
    case(
        "SBI I 50 800->792",
        I,
        NSE,
        50,
        Some(800_00),
        Some(792_00),
        [23_88, 2_44, 8, 10_00, 1_00, 4_74, 0, 42_24],
    ),
    case(
        "Yes Bank NSE I 1000 20.5->21.1",
        I,
        NSE,
        1_000,
        Some(20_50),
        Some(21_10),
        [12_48, 1_28, 4, 5_00, 1_00, 2_48, 0, 22_17],
    ),
    case(
        "MRF NSE I 1 130000->131200",
        I,
        NSE,
        1,
        Some(130_000_00),
        Some(131_200_00),
        [40_00, 8_02, 26, 33_00, 4_00, 8_64, 0, 93_62],
    ),
    case(
        "Vodafone Idea NSE I 50000 8.5->8.65",
        I,
        NSE,
        50_000,
        Some(8_50),
        Some(8_65),
        [40_00, 26_33, 86, 108_00, 13_00, 11_94, 0, 200_00],
    ),
    case(
        "Vodafone Idea NSE delivery 50000 8.5->8.65",
        C,
        NSE,
        50_000,
        Some(8_50),
        Some(8_65),
        [0, 26_33, 86, 858_00, 64_00, 4_74, 14_75, 967_92],
    ),
    case(
        "Vodafone Idea NSE MTF 50000 8.5->8.65",
        F,
        NSE,
        50_000,
        Some(8_50),
        Some(8_65),
        [40_00, 26_33, 86, 858_00, 64_00, 11_94, 14_75, 1_015_12],
    ),
    case(
        "Vodafone Idea BSE MTF buy-only 50000 @12.57",
        F,
        BSE,
        50_000,
        Some(12_57),
        None,
        [20_00, 23_88, 63, 629_00, 94_00, 7_90, 0, 775_19],
    ),
    case(
        "Vodafone Idea BSE delivery buy-only 50000 @12.57",
        C,
        BSE,
        50_000,
        Some(12_57),
        None,
        [0, 23_88, 63, 629_00, 94_00, 4_30, 0, 751_59],
    ),
    case(
        "Vodafone Idea BSE I buy-only 50000 @12.57",
        I,
        BSE,
        50_000,
        Some(12_57),
        None,
        [20_00, 23_88, 63, 0, 19_00, 7_90, 0, 71_27],
    ),
    case(
        "Tata Steel NSE I 100 150->148",
        I,
        NSE,
        100,
        Some(150_00),
        Some(148_00),
        [8_94, 91, 3, 4_00, 0, 1_77, 0, 15_81],
    ),
    case(
        "Tata Steel NSE delivery 100 150->148",
        C,
        NSE,
        100,
        Some(150_00),
        Some(148_00),
        [0, 91, 3, 30_00, 2_00, 16, 14_75, 47_91],
    ),
    case(
        "Tata Steel NSE MTF 100 150->148",
        F,
        NSE,
        100,
        Some(150_00),
        Some(148_00),
        [8_94, 91, 3, 30_00, 2_00, 1_77, 14_75, 58_46],
    ),
    case(
        "Tata Steel BSE MTF 100 150->148",
        F,
        BSE,
        100,
        Some(150_00),
        Some(148_00),
        [8_94, 1_13, 3, 30_00, 2_00, 1_81, 14_75, 58_72],
    ),
    case(
        "Tata Steel BSE delivery 100 150->148",
        C,
        BSE,
        100,
        Some(150_00),
        Some(148_00),
        [0, 1_13, 3, 30_00, 2_00, 20, 14_75, 48_17],
    ),
    case(
        "Tata Steel BSE I 100 150->148",
        I,
        BSE,
        100,
        Some(150_00),
        Some(148_00),
        [8_94, 1_13, 3, 4_00, 0, 1_81, 0, 16_07],
    ),
    case(
        "MRF BSE I 1 130000->128500",
        I,
        BSE,
        1,
        Some(130_000_00),
        Some(128_500_00),
        [40_00, 9_82, 26, 32_00, 4_00, 8_97, 0, 95_08],
    ),
    case(
        "MRF BSE delivery 1 130000->128500",
        C,
        BSE,
        1,
        Some(130_000_00),
        Some(128_500_00),
        [0, 9_82, 26, 259_00, 20_00, 1_77, 14_75, 304_60],
    ),
    case(
        "MRF BSE MTF 1 130000->128500",
        F,
        BSE,
        1,
        Some(130_000_00),
        Some(128_500_00),
        [40_00, 9_82, 26, 259_00, 20_00, 8_97, 14_75, 351_80],
    ),
    case(
        "Yes Bank NSE MTF 1 20.55->20.9",
        F,
        NSE,
        1,
        Some(20_55),
        Some(20_90),
        [1, 0, 0, 0, 0, 0, 14_75, 14_81],
    ),
    case(
        "Yes Bank NSE delivery 1 20.55->20.9",
        C,
        NSE,
        1,
        Some(20_55),
        Some(20_90),
        [0, 0, 0, 0, 0, 0, 14_75, 14_79],
    ),
    case(
        "Yes Bank NSE I 1 20.55->20.9",
        I,
        NSE,
        1,
        Some(20_55),
        Some(20_90),
        [1, 0, 0, 0, 0, 0, 0, 2],
    ),
    case(
        "ITC NSE I 160 410->412.35",
        I,
        NSE,
        160,
        Some(410_00),
        Some(412_35),
        [39_47, 4_04, 13, 16_00, 2_00, 7_83, 0, 69_94],
    ),
    case(
        "ITC NSE delivery 160 410->412.35",
        C,
        NSE,
        160,
        Some(410_00),
        Some(412_35),
        [0, 4_04, 13, 132_00, 10_00, 73, 14_75, 161_06],
    ),
    case(
        "ITC NSE MTF 160 410->412.35",
        F,
        NSE,
        160,
        Some(410_00),
        Some(412_35),
        [39_47, 4_04, 13, 132_00, 10_00, 7_83, 14_75, 207_64],
    ),
    case(
        "OPT NIFTY 19 OCT 25000 PUT 2 lots of 65 42.5->55.25",
        OPT,
        NSE,
        130,
        Some(42_50),
        Some(55_25),
        [40_00, 4_51, 1, 11_00, 0, 8_01, 0, 63_48],
    ),
    case(
        "OPT NIFTY 19 OCT 24000 CALL 1 lot of 65 1205->1236.5",
        OPT,
        NSE,
        65,
        Some(1_205_00),
        Some(1_236_50),
        [40_00, 56_39, 16, 121_00, 2_00, 17_35, 0, 236_81],
    ),
    case(
        "OPT NIFTY 19 OCT 27000 CALL 4 lots of 65 0.45->0.85",
        OPT,
        NSE,
        260,
        Some(45),
        Some(85),
        [40_00, 12, 0, 0, 0, 7_22, 0, 47_67],
    ),
    case(
        "OPT BANKNIFTY 27 OCT 56000 CALL 1 lot of 30 520->610.45",
        OPT,
        NSE,
        30,
        Some(520_00),
        Some(610_45),
        [40_00, 12_05, 3, 27_00, 0, 9_37, 0, 89_39],
    ),
    case(
        "OPT BANKNIFTY 27 OCT 56000 PUT 3 lots of 30 480->410.15",
        OPT,
        NSE,
        90,
        Some(480_00),
        Some(410_15),
        [40_00, 28_46, 8, 55_00, 1_00, 12_32, 0, 137_53],
    ),
    case(
        "OPT RELIANCE 27 OCT 1400 CALL 1 lot of 500 24.5->31.2",
        OPT,
        NSE,
        500,
        Some(24_50),
        Some(31_20),
        [40_00, 9_90, 3, 23_00, 0, 8_98, 0, 82_68],
    ),
    case(
        "OPT RELIANCE 27 OCT 1400 CALL 2 lots of 500 6.85->4.1",
        OPT,
        NSE,
        1_000,
        Some(6_85),
        Some(4_10),
        [40_00, 3_89, 1, 6_00, 0, 7_90, 0, 58_15],
    ),
    case(
        "OPT RELIANCE 27 OCT 1400 CALL 3 lots of 500 0.35->0.2",
        OPT,
        NSE,
        1_500,
        Some(35),
        Some(20),
        [40_00, 29, 0, 0, 0, 7_25, 0, 48_02],
    ),
    case(
        "OPT NIFTY 19 OCT 23000 PUT 10 lots of 65 0.15->0.05",
        OPT,
        NSE,
        650,
        Some(15),
        Some(5),
        [40_00, 5, 0, 0, 0, 7_21, 0, 47_30],
    ),
    case(
        "OPT RELIANCE 27 OCT 1400 PUT 1 lot of 500 18->26.4",
        OPT,
        NSE,
        500,
        Some(18_00),
        Some(26_40),
        [40_00, 7_89, 2, 20_00, 0, 8_62, 0, 76_60],
    ),
];

fn paisa(raw: i64) -> Paisa {
    Paisa::from_raw(raw)
}

fn quoted(case: &Case) -> DhanQuote {
    quote(
        case.kind,
        case.exchange,
        case.quantity,
        case.buy.map(paisa),
        case.sell.map(paisa),
    )
    .expect("every golden case is quotable")
}

/// The lines this repository computes, in the page's order, paisa.
fn computed(q: &DhanQuote) -> [i64; 7] {
    [
        q.brokerage().expect("in range").raw(),
        q.exchange_charge().expect("in range").raw(),
        q.sebi().expect("in range").raw(),
        q.stt().expect("in range").raw(),
        q.stamp().expect("in range").raw(),
        q.gst().expect("in range").raw(),
        q.dp().expect("in range").raw(),
    ]
}

#[test]
fn every_calculator_line_matches_the_page_on_every_golden_case() {
    let mut lines = 0;
    for case in &GOLDEN {
        let q = quoted(case);
        let mine = computed(&q);
        let page: [i64; 7] = case.page[..7].try_into().expect("seven lines");
        assert_eq!(mine, page, "{}", case.name);
        assert!(q.to_string().starts_with("Dhan calculator (estimate)"));
        lines += mine.len();
    }
    assert_eq!(GOLDEN.len(), 48);
    assert_eq!(lines, 48 * 7, "every line of every case, not a sample");
}

#[test]
fn the_raw_lines_of_the_fifteen_october_ui_readings_are_exact_to_the_billionth_rupee() {
    // The 15 Oct readings carried the page's unrounded values. Units of
    // 10^-9 rupee: 4.68175 rupees is 4_681_750_000. Columns: brokerage,
    // exchange, SEBI, GST.
    let raw: [(&str, [i128; 4]); 9] = [
        (
            "V1",
            [40_000_000_000, 4_681_750_000, 152_500_000, 8_042_724_000],
        ),
        (
            "V2",
            [
                40_000_000_000,
                61_707_000_000,
                2_010_000_000,
                18_307_260_000,
            ],
        ),
        ("V3", [597_000_000, 61_093_000, 1_990_000, 118_458_000]),
        (
            "V4",
            [
                40_000_000_000,
                308_535_000_000,
                10_050_000_000,
                62_736_300_000,
            ],
        ),
        (
            "V5",
            [
                40_000_000_000,
                61_093_000_000,
                1_990_000_000,
                18_196_740_000,
            ],
        ),
        ("D1", [0, 4_681_750_000, 152_500_000, 842_724_000]),
        (
            "M1",
            [40_000_000_000, 4_681_750_000, 152_500_000, 8_042_724_000],
        ),
        (
            "B1",
            [40_000_000_000, 5_795_000_000, 152_500_000, 8_243_100_000],
        ),
        (
            "O1",
            [20_000_000_000, 2_333_200_000, 76_000_000, 4_019_976_000],
        ),
    ];
    for (at, (name, want)) in raw.iter().enumerate() {
        let case = &GOLDEN[at];
        assert!(case.name.starts_with(name), "{name} is GOLDEN[{at}]");
        let q = quoted(case);
        let lines = q.exact_lines();
        let got = [
            lines[0].1.units(),
            lines[1].1.units(),
            lines[2].1.units(),
            lines[5].1.units(),
        ];
        assert_eq!(&got, want, "{name}");
    }
    // DP is exactly ₹14.75 where it applies, and exactly nothing elsewhere.
    assert_eq!(
        quoted(&GOLDEN[5]).exact_lines()[6].1.units(),
        14_750_000_000
    );
    assert_eq!(quoted(&GOLDEN[0]).exact_lines()[6].1.units(), 0);
}

/// `(case, page total − this repository's total)` for every case that is
/// not exact, in paisa.
fn total_misses() -> Vec<(&'static str, i64)> {
    GOLDEN
        .iter()
        .filter_map(|case| {
            let gap = case.page[7] - quoted(case).total().raw();
            (gap != 0).then_some((case.name, gap))
        })
        .collect()
}

#[test]
fn every_calculator_total_is_within_one_paisa_of_the_page() {
    for case in &GOLDEN {
        let mine = quoted(case).total().raw();
        assert!(
            (case.page[7] - mine).abs() <= 1,
            "{}: page {} vs {mine}",
            case.name,
            case.page[7]
        );
    }
}

#[test]
fn the_one_paisa_total_misses_are_listed_with_their_direction() {
    // `+1`: the page shows one paisa MORE than the exact sum rounded half-up;
    // `-1`: one paisa LESS. The page's own last-paisa rule is UNVERIFIED
    // (D-5112): this list is what any candidate rule has to reproduce.
    assert_eq!(total_misses(), MISSES);
}

const MISSES: [(&str, i64); 15] = [
    ("V2 NSE I 1000 1000->1010", 1),
    ("V3 NSE I 10 100->99", -1),
    ("B1 BSE I 50 1520->1530", -1),
    ("OPT NIFTY 19 OCT 25300 CALL 1 lot (65) 100->120", 1),
    ("Reliance I 100 1400->1414", -1),
    ("Infosys I 200 1500->1520", 1),
    ("Yes Bank NSE I 1000 20.5->21.1", 1),
    ("MRF BSE I 1 130000->128500", 1),
    ("Yes Bank NSE delivery 1 20.55->20.9", -1),
    ("OPT NIFTY 19 OCT 24000 CALL 1 lot of 65 1205->1236.5", 1),
    ("OPT NIFTY 19 OCT 27000 CALL 4 lots of 65 0.45->0.85", -1),
    ("OPT RELIANCE 27 OCT 1400 CALL 1 lot of 500 24.5->31.2", 1),
    ("OPT RELIANCE 27 OCT 1400 CALL 2 lots of 500 6.85->4.1", -1),
    ("OPT RELIANCE 27 OCT 1400 CALL 3 lots of 500 0.35->0.2", 1),
    ("OPT NIFTY 19 OCT 23000 PUT 10 lots of 65 0.15->0.05", -1),
];

/// The precise contract-note total for a golden case, at today's rates on
/// 16 Oct 2026: one client-security-day for intraday and F&O, a buy day and a
/// sell day for delivery and MTF.
fn precise_total(case: &Case) -> Result<Paisa, ChargeError> {
    let on = TradeDay::new(2026, 10, 16).expect("a real date");
    let order =
        |side, price: Option<i64>| price.map(|price| Order::new(side, case.quantity, paisa(price)));
    let buy = order(OrderSide::Buy, case.buy).transpose()?;
    let sell = order(OrderSide::Sell, case.sell).transpose()?;
    let days: Vec<Vec<Order>> = match case.kind {
        TradeKind::Equity(Product::Delivery | Product::Mtf) => {
            [buy, sell].into_iter().flatten().map(|o| vec![o]).collect()
        }
        _ => vec![[buy, sell].into_iter().flatten().collect()],
    };
    let mut total = 0;
    for orders in days {
        let mut day =
            ClientSecurityDay::open(case.kind, case.exchange, on, RateSelector::default())?;
        for o in orders {
            day.add(o)?;
        }
        total += day.settle()?.total().raw();
    }
    Ok(paisa(total))
}

#[test]
fn every_golden_case_has_a_precise_figure_or_a_named_refusal() {
    let got: Vec<(&str, Result<i64, ChargeError>)> = GOLDEN
        .iter()
        .map(|case| (case.name, precise_total(case).map(Paisa::raw)))
        .collect();
    let mut table = String::from(
        "| case | Dhan page total | Dhan-mode total | precise total |\n|---|---|---|---|\n",
    );
    for (case, (_, precise)) in GOLDEN.iter().zip(&got) {
        let shown = match precise {
            Ok(raw) => format!("{}", costs::exact::Rupees(paisa(*raw))),
            Err(ChargeError::OpenIntradayPosition { .. }) => {
                "refused: open intraday position".to_owned()
            }
            Err(other) => format!("refused: {other}"),
        };
        writeln!(
            table,
            "| {} | {} | {} | {} |",
            case.name,
            costs::exact::Rupees(paisa(case.page[7])),
            costs::exact::Rupees(quoted(case).total()),
            shown
        )
        .expect("a String takes every write");
    }
    println!("{table}");
    let pinned: Vec<(&str, Result<i64, ChargeError>)> = PRECISE.to_vec();
    assert_eq!(got, pinned);
}

/// The precise total of every golden case, paisa, at today's rates on 16 Oct
/// 2026 — or the refusal. The two refusals are the buy-only INTRADAY readings:
/// the page prices a lone intraday buy; a contract note never shows one,
/// because the position is squared off the same day (D-5114).
const PRECISE: [(&str, Result<i64, ChargeError>); 48] = [
    ("V1 NSE I 50 1520->1530", Ok(73_90)),
    ("V2 NSE I 1000 1000->1010", Ok(403_39)),
    ("V3 NSE I 10 100->99", Ok(78)),
    ("V4 NSE I 100000 50->50.5", Ok(1_829_14)),
    ("V5 NSE I short 500 sell2000 buy1980", Ok(400_63)),
    ("D1 NSE delivery 50 1520->1530", Ok(184_46)),
    ("M1 NSE MTF 50 1520->1530", Ok(231_66)),
    ("B1 BSE I 50 1520->1530", Ok(75_13)),
    (
        "O1 NSE I buy only 50 @1520",
        Err(ChargeError::OpenIntradayPosition {
            bought: 50,
            sold: 0,
        }),
    ),
    ("FUT NIFTY OCT FUT 1 lot (65) 25200->25250", Ok(975_88)),
    ("OPT NIFTY 19 OCT 25300 CALL 1 lot (65) 100->120", Ok(65_21)),
    ("Reliance I 100 1400->1414", Ok(96_73)),
    ("TCS I 20 3000->3030", Ok(64_20)),
    ("Infosys I 200 1500->1520", Ok(154_79)),
    ("SBI I 50 800->792", Ok(42_15)),
    ("Yes Bank NSE I 1000 20.5->21.1", Ok(22_28)),
    ("MRF NSE I 1 130000->131200", Ok(93_97)),
    ("Vodafone Idea NSE I 50000 8.5->8.65", Ok(199_28)),
    ("Vodafone Idea NSE delivery 50000 8.5->8.65", Ok(968_84)),
    ("Vodafone Idea NSE MTF 50000 8.5->8.65", Ok(1_016_04)),
    ("Vodafone Idea BSE MTF buy-only 50000 @12.57", Ok(775_16)),
    (
        "Vodafone Idea BSE delivery buy-only 50000 @12.57",
        Ok(751_56),
    ),
    (
        "Vodafone Idea BSE I buy-only 50000 @12.57",
        Err(ChargeError::OpenIntradayPosition {
            bought: 50_000,
            sold: 0,
        }),
    ),
    ("Tata Steel NSE I 100 150->148", Ok(15_66)),
    ("Tata Steel NSE delivery 100 150->148", Ok(47_86)),
    ("Tata Steel NSE MTF 100 150->148", Ok(58_41)),
    ("Tata Steel BSE MTF 100 150->148", Ok(58_65)),
    ("Tata Steel BSE delivery 100 150->148", Ok(48_10)),
    ("Tata Steel BSE I 100 150->148", Ok(15_91)),
    ("MRF BSE I 1 130000->128500", Ok(94_94)),
    ("MRF BSE delivery 1 130000->128500", Ok(305_50)),
    ("MRF BSE MTF 1 130000->128500", Ok(352_70)),
    ("Yes Bank NSE MTF 1 20.55->20.9", Ok(14_77)),
    ("Yes Bank NSE delivery 1 20.55->20.9", Ok(14_75)),
    ("Yes Bank NSE I 1 20.55->20.9", Ok(2)),
    ("ITC NSE I 160 410->412.35", Ok(69_50)),
    ("ITC NSE delivery 160 410->412.35", Ok(161_68)),
    ("ITC NSE MTF 160 410->412.35", Ok(208_26)),
    (
        "OPT NIFTY 19 OCT 25000 PUT 2 lots of 65 42.5->55.25",
        Ok(63_53),
    ),
    (
        "OPT NIFTY 19 OCT 24000 CALL 1 lot of 65 1205->1236.5",
        Ok(236_93),
    ),
    (
        "OPT NIFTY 19 OCT 27000 CALL 4 lots of 65 0.45->0.85",
        Ok(47_34),
    ),
    (
        "OPT BANKNIFTY 27 OCT 56000 CALL 1 lot of 30 520->610.45",
        Ok(88_45),
    ),
    (
        "OPT BANKNIFTY 27 OCT 56000 PUT 3 lots of 30 480->410.15",
        Ok(136_88),
    ),
    (
        "OPT RELIANCE 27 OCT 1400 CALL 1 lot of 500 24.5->31.2",
        Ok(81_92),
    ),
    (
        "OPT RELIANCE 27 OCT 1400 CALL 2 lots of 500 6.85->4.1",
        Ok(57_80),
    ),
    (
        "OPT RELIANCE 27 OCT 1400 CALL 3 lots of 500 0.35->0.2",
        Ok(47_54),
    ),
    (
        "OPT NIFTY 19 OCT 23000 PUT 10 lots of 65 0.15->0.05",
        Ok(47_26),
    ),
    (
        "OPT RELIANCE 27 OCT 1400 PUT 1 lot of 500 18->26.4",
        Ok(76_53),
    ),
];

#[test]
fn the_precise_and_calculator_modes_differ_where_the_rules_differ_and_agree_where_they_do_not() {
    // V1 line by line: the calculator taxes the sell value (19.125) and the
    // precise stack the VWAP value (19.0625), both shown as ₹19; the
    // calculator leaves the SEBI fee out of the GST base and does not round
    // STT and stamp inside its total, the precise stack does both. Those two
    // rules are the whole ₹0.38 between ₹74.28 and ₹73.90.
    let v1 = &GOLDEN[0];
    let q = quoted(v1);
    let precise = precise_total(v1).expect("priced");
    assert_eq!(q.total().raw() - precise.raw(), 38);
    // The calculator's unrounded STT and stamp: 19.125 + 2.28 = 21.405; the
    // precise rupees: 19 + 2 = 21. Its GST: 8.042724; precise: 8.07.
    // 0.405 - (8.07 - 8.042724) = 0.377276, and the rest is the last-paisa
    // rounding of each.
    assert_eq!(
        q.exact_lines()[3].1.units() + q.exact_lines()[4].1.units(),
        21_405_000_000
    );
}
