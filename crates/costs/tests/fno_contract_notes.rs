//! Package FNO-COSTS: the 24 contract notes of COSTS.md section 9
//! (`costs-table.json` `test_vectors`, version `costs-table/2026-10-30/r1`),
//! each under its four columns to the paisa, and the properties every trade
//! keeps (D-8110..D-8116).
//!
//! The vectors were transcribed from `costs-table.json` with `jq` (paise =
//! rupees x 100, rounded), so every figure below is the table's own.
//!
//! **One paisa on purpose.** COSTS.md's calculator rounds the DP charge's GST
//! once at 18% (Rs 2.25) on every worst column, while it rounds the main GST
//! line and the square-off fee's GST under both RD-GST readings and keeps the
//! larger. Operator rule 7 takes the worse reading on every open choice, and
//! CGST 9% + SGST 9% on Rs 12.50 is Rs 1.13 + Rs 1.13 = Rs 2.26. So the worst
//! columns here are COSTS.md's to the paisa under COSTS.md's own reading set
//! (`dp_gst` once), and the crate's worst is that plus one paisa on the GST
//! on DP line, its total and the headline (D-8113).

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::inconsistent_digit_grouping,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a test that \
              cannot panic cannot fail"
)]

use brutex_core::price::Paisa;
use costs::day::TradeDay;
use costs::equity_clock::{Clock, Fill, HeadlineFrom, Order, TradeFills, price_trade};
use costs::equity_dated::{EQUITY_DATED_VERSION, STATE_STAMPS};
use costs::equity_readings::{
    BrokerageReading, DpReading, ExchangeRounding, GstRounding, NoteInput, ReadingSet,
    SebiRounding, SlabReading, SquareOffReading, SquareOffSplit, StateChoice, StateRounding,
    SttReading, TaxRounding, note, official, worst,
};
use costs::rate::OrderSide::{self, Buy, Sell};

/// A trade's orders: side, whether the broker placed it, and its fills as
/// (quantity, price in paise).
type Orders<'a> = &'a [(OrderSide, bool, &'a [(i64, i64)])];

/// One COSTS.md vector, every amount in paise.
struct Vector {
    id: &'static str,
    date: (u16, u8, u8),
    orders: Orders<'static>,
    quantity: i64,
    buy_value: i64,
    sell_value: i64,
    vwap: i64,
    gross: i64,
    /// brokerage, exchange + IPFT, SEBI, STT, stamp, GST, DP, GST on DP,
    /// square-off fee, GST on it, total.
    dated_precise: [i64; 11],
    dated_worst: [i64; 11],
    today_precise: [i64; 11],
    today_worst: [i64; 11],
    headline: i64,
    net: i64,
}

const VECTORS: [Vector; 24] = [
    Vector {
        id: "CN01",
        date: (2018, 9, 3),
        orders: &[
            (Buy, false, &[(400, 125_000)]),
            (Sell, false, &[(400, 126_250)]),
        ],
        quantity: 400,
        buy_value: 50_000_000,
        sell_value: 50_500_000,
        vwap: 125_625,
        gross: 500_000,
        dated_precise: [4000, 3266, 151, 12_600, 10_000, 1308, 0, 0, 0, 0, 31_325],
        dated_worst: [
            4000, 3266, 151, 12_600, 10_000, 1308, 1250, 225, 0, 0, 32_800,
        ],
        today_precise: [4000, 3085, 101, 12_600, 1500, 1293, 0, 0, 0, 0, 22_579],
        today_worst: [4000, 3085, 101, 12_600, 1500, 1294, 1250, 225, 0, 0, 24_055],
        headline: 32_800,
        net: 467_200,
    },
    Vector {
        id: "CN02",
        date: (2019, 3, 29),
        orders: &[
            (Sell, false, &[(615, 81_235)]),
            (Buy, false, &[(615, 80_500)]),
        ],
        quantity: 615,
        buy_value: 49_507_500,
        sell_value: 49_959_525,
        vwap: 80_868,
        gross: 452_025,
        dated_precise: [4000, 3233, 149, 12_400, 9947, 1302, 0, 0, 0, 0, 31_031],
        dated_worst: [4000, 3233, 149, 12_500, 9947, 1302, 1250, 225, 0, 0, 32_606],
        today_precise: [4000, 3054, 99, 12_400, 1500, 1288, 0, 0, 0, 0, 22_341],
        today_worst: [4000, 3054, 100, 12_500, 1500, 1288, 1250, 225, 0, 0, 23_917],
        headline: 32_606,
        net: 419_419,
    },
    Vector {
        id: "CN03",
        date: (2019, 4, 1),
        orders: &[
            (Buy, false, &[(213, 234_565)]),
            (Sell, false, &[(213, 233_010)]),
        ],
        quantity: 213,
        buy_value: 49_962_345,
        sell_value: 49_631_130,
        vwap: 233_788,
        gross: -331_215,
        dated_precise: [4000, 3237, 100, 12_400, 9959, 1303, 0, 0, 0, 0, 30_999],
        dated_worst: [
            4000, 3237, 100, 12_400, 10_000, 1303, 1250, 225, 0, 0, 32_515,
        ],
        today_precise: [4000, 3058, 100, 12_400, 1500, 1288, 0, 0, 0, 0, 22_346],
        today_worst: [4000, 3058, 100, 12_400, 1500, 1288, 1250, 225, 0, 0, 23_821],
        headline: 32_515,
        net: -363_730,
    },
    Vector {
        id: "CN04",
        date: (2020, 5, 29),
        orders: &[
            (Buy, false, &[(3197, 15_640)]),
            (Sell, false, &[(3197, 15_985)]),
        ],
        quantity: 3197,
        buy_value: 50_001_080,
        sell_value: 51_104_045,
        vwap: 15_813,
        gross: 1_102_965,
        dated_precise: [4000, 3286, 101, 12_600, 10_000, 1311, 0, 0, 0, 0, 31_298],
        dated_worst: [
            4000, 3286, 101, 12_800, 10_000, 1312, 1250, 225, 0, 0, 32_974,
        ],
        today_precise: [4000, 3104, 101, 12_600, 1500, 1297, 0, 0, 0, 0, 22_602],
        today_worst: [4000, 3104, 101, 12_800, 1500, 1297, 1250, 225, 0, 0, 24_277],
        headline: 32_974,
        net: 1_069_991,
    },
    Vector {
        id: "CN05",
        date: (2020, 6, 1),
        orders: &[
            (Sell, false, &[(124, 402_000)]),
            (Buy, false, &[(124, 399_055)]),
        ],
        quantity: 124,
        buy_value: 49_482_820,
        sell_value: 49_848_000,
        vwap: 400_528,
        gross: 365_180,
        dated_precise: [4000, 3228, 50, 12_400, 9933, 1301, 0, 0, 0, 0, 30_912],
        dated_worst: [4000, 3228, 50, 12_500, 9933, 1302, 1250, 225, 0, 0, 32_488],
        today_precise: [4000, 3049, 99, 12_400, 1500, 1287, 0, 0, 0, 0, 22_335],
        today_worst: [4000, 3049, 99, 12_500, 1500, 1287, 1250, 225, 0, 0, 23_910],
        headline: 32_488,
        net: 332_692,
    },
    Vector {
        id: "CN06",
        date: (2020, 6, 30),
        orders: &[
            (Buy, false, &[(833, 60_000)]),
            (Sell, false, &[(833, 59_405)]),
        ],
        quantity: 833,
        buy_value: 49_980_000,
        sell_value: 49_484_365,
        vwap: 59_703,
        gross: -495_635,
        dated_precise: [4000, 3233, 50, 12_400, 9946, 1302, 0, 0, 0, 0, 30_931],
        dated_worst: [4000, 3233, 50, 12_400, 9946, 1302, 1250, 225, 0, 0, 32_406],
        today_precise: [4000, 3054, 99, 12_400, 1500, 1288, 0, 0, 0, 0, 22_341],
        today_worst: [4000, 3054, 99, 12_400, 1500, 1288, 1250, 225, 0, 0, 23_816],
        headline: 32_406,
        net: -528_041,
    },
    Vector {
        id: "CN07",
        date: (2020, 7, 1),
        orders: &[
            (Buy, false, &[(833, 60_000)]),
            (Sell, false, &[(833, 60_600)]),
        ],
        quantity: 833,
        buy_value: 49_980_000,
        sell_value: 50_479_800,
        vwap: 60_300,
        gross: 499_800,
        dated_precise: [4000, 3265, 50, 12_600, 1500, 1308, 0, 0, 0, 0, 22_723],
        dated_worst: [4000, 3265, 50, 12_600, 1500, 1308, 1250, 225, 0, 0, 24_198],
        today_precise: [4000, 3084, 100, 12_600, 1500, 1293, 0, 0, 0, 0, 22_577],
        today_worst: [4000, 3084, 100, 12_600, 1500, 1294, 1250, 225, 0, 0, 24_053],
        headline: 24_198,
        net: 475_602,
    },
    Vector {
        id: "CN08",
        date: (2020, 12, 31),
        orders: &[
            (Buy, false, &[(500, 100_000)]),
            (Sell, false, &[(500, 101_500)]),
        ],
        quantity: 500,
        buy_value: 50_000_000,
        sell_value: 50_750_000,
        vwap: 100_750,
        gross: 750_000,
        dated_precise: [4000, 3274, 50, 12_600, 1500, 1309, 0, 0, 0, 0, 22_733],
        dated_worst: [4000, 3274, 50, 12_700, 1500, 1310, 1250, 225, 0, 0, 24_309],
        today_precise: [4000, 3093, 101, 12_600, 1500, 1295, 0, 0, 0, 0, 22_589],
        today_worst: [4000, 3093, 101, 12_700, 1500, 1295, 1250, 225, 0, 0, 24_164],
        headline: 24_309,
        net: 725_691,
    },
    Vector {
        id: "CN09",
        date: (2021, 1, 1),
        orders: &[
            (Sell, false, &[(500, 100_000)]),
            (Buy, false, &[(500, 101_200)]),
        ],
        quantity: 500,
        buy_value: 50_600_000,
        sell_value: 50_000_000,
        vwap: 100_600,
        gross: -600_000,
        dated_precise: [4000, 3471, 50, 12_600, 1500, 1345, 0, 0, 0, 0, 22_966],
        dated_worst: [4000, 3471, 50, 12_600, 1500, 1345, 1250, 225, 0, 0, 24_441],
        today_precise: [4000, 3088, 101, 12_600, 1500, 1294, 0, 0, 0, 0, 22_583],
        today_worst: [4000, 3088, 101, 12_600, 1500, 1294, 1250, 225, 0, 0, 24_058],
        headline: 24_441,
        net: -624_441,
    },
    Vector {
        id: "CN10",
        date: (2021, 3, 31),
        orders: &[
            (Buy, false, &[(1000, 49_995)]),
            (Sell, false, &[(1000, 50_500)]),
        ],
        quantity: 1000,
        buy_value: 49_995_000,
        sell_value: 50_500_000,
        vwap: 50_248,
        gross: 505_000,
        dated_precise: [4000, 3467, 50, 12_600, 1500, 1344, 0, 0, 0, 0, 22_961],
        dated_worst: [4000, 3467, 50, 12_600, 1500, 1344, 1250, 225, 0, 0, 24_436],
        today_precise: [4000, 3085, 100, 12_600, 1500, 1293, 0, 0, 0, 0, 22_578],
        today_worst: [4000, 3085, 101, 12_600, 1500, 1294, 1250, 225, 0, 0, 24_055],
        headline: 24_436,
        net: 480_564,
    },
    Vector {
        id: "CN11",
        date: (2021, 4, 1),
        orders: &[
            (Buy, false, &[(1000, 49_995)]),
            (Sell, false, &[(1000, 50_500)]),
        ],
        quantity: 1000,
        buy_value: 49_995_000,
        sell_value: 50_500_000,
        vwap: 50_248,
        gross: 505_000,
        dated_precise: [4000, 3467, 100, 12_600, 1500, 1344, 0, 0, 0, 0, 23_011],
        dated_worst: [4000, 3467, 101, 12_600, 1500, 1344, 1250, 225, 0, 0, 24_487],
        today_precise: [4000, 3085, 100, 12_600, 1500, 1293, 0, 0, 0, 0, 22_578],
        today_worst: [4000, 3085, 101, 12_600, 1500, 1294, 1250, 225, 0, 0, 24_055],
        headline: 24_487,
        net: 480_513,
    },
    Vector {
        id: "CN12",
        date: (2022, 7, 15),
        orders: &[
            (Buy, false, &[(200, 250_000)]),
            (Sell, false, &[(200, 253_750)]),
        ],
        quantity: 200,
        buy_value: 50_000_000,
        sell_value: 50_750_000,
        vwap: 251_875,
        gross: 750_000,
        dated_precise: [4000, 3476, 101, 12_600, 1500, 1346, 0, 0, 0, 0, 23_023],
        dated_worst: [4000, 3476, 101, 12_700, 1500, 1346, 1250, 225, 0, 0, 24_598],
        today_precise: [4000, 3093, 101, 12_600, 1500, 1295, 0, 0, 0, 0, 22_589],
        today_worst: [4000, 3093, 101, 12_700, 1500, 1295, 1250, 225, 0, 0, 24_164],
        headline: 24_598,
        net: 725_402,
    },
    Vector {
        id: "CN13",
        date: (2022, 7, 18),
        orders: &[
            (Buy, false, &[(200, 250_000)]),
            (Sell, false, &[(200, 253_750)]),
        ],
        quantity: 200,
        buy_value: 50_000_000,
        sell_value: 50_750_000,
        vwap: 251_875,
        gross: 750_000,
        dated_precise: [4000, 3476, 101, 12_600, 1500, 1364, 0, 0, 0, 0, 23_041],
        dated_worst: [4000, 3476, 101, 12_700, 1500, 1364, 1250, 225, 0, 0, 24_616],
        today_precise: [4000, 3093, 101, 12_600, 1500, 1295, 0, 0, 0, 0, 22_589],
        today_worst: [4000, 3093, 101, 12_700, 1500, 1295, 1250, 225, 0, 0, 24_164],
        headline: 24_616,
        net: 725_384,
    },
    Vector {
        id: "CN14",
        date: (2023, 3, 31),
        orders: &[
            (Sell, false, &[(6423, 7785)]),
            (Buy, false, &[(6423, 7630)]),
        ],
        quantity: 6423,
        buy_value: 49_007_490,
        sell_value: 50_003_055,
        vwap: 7708,
        gross: 995_565,
        dated_precise: [4000, 3416, 99, 12_400, 1500, 1353, 0, 0, 0, 0, 22_768],
        dated_worst: [4000, 3416, 99, 12_500, 1500, 1353, 1250, 225, 0, 0, 24_343],
        today_precise: [4000, 3040, 99, 12_400, 1500, 1285, 0, 0, 0, 0, 22_324],
        today_worst: [4000, 3040, 99, 12_500, 1500, 1286, 1250, 225, 0, 0, 23_900],
        headline: 24_343,
        net: 971_222,
    },
    Vector {
        id: "CN15",
        date: (2023, 4, 3),
        orders: &[
            (Sell, false, &[(6423, 7785)]),
            (Buy, false, &[(6423, 7630)]),
        ],
        quantity: 6423,
        buy_value: 49_007_490,
        sell_value: 50_003_055,
        vwap: 7708,
        gross: 995_565,
        dated_precise: [4000, 3317, 99, 12_400, 1500, 1335, 0, 0, 0, 0, 22_651],
        dated_worst: [4000, 3317, 99, 12_500, 1500, 1335, 1250, 225, 0, 0, 24_226],
        today_precise: [4000, 3040, 99, 12_400, 1500, 1285, 0, 0, 0, 0, 22_324],
        today_worst: [4000, 3040, 99, 12_500, 1500, 1286, 1250, 225, 0, 0, 23_900],
        headline: 24_226,
        net: 971_339,
    },
    Vector {
        id: "CN16",
        date: (2024, 3, 28),
        orders: &[
            (Buy, false, &[(312, 160_100)]),
            (Sell, false, &[(312, 164_000)]),
        ],
        quantity: 312,
        buy_value: 49_951_200,
        sell_value: 51_168_000,
        vwap: 162_050,
        gross: 1_216_800,
        dated_precise: [4000, 3387, 101, 12_600, 1500, 1348, 0, 0, 0, 0, 22_936],
        dated_worst: [4000, 3387, 101, 12_800, 1500, 1348, 1250, 225, 0, 0, 24_611],
        today_precise: [4000, 3104, 101, 12_600, 1500, 1297, 0, 0, 0, 0, 22_602],
        today_worst: [4000, 3105, 101, 12_800, 1500, 1298, 1250, 225, 0, 0, 24_279],
        headline: 24_611,
        net: 1_192_189,
    },
    Vector {
        id: "CN17",
        date: (2024, 4, 1),
        orders: &[
            (Buy, false, &[(312, 160_100)]),
            (Sell, false, &[(312, 164_000)]),
        ],
        quantity: 312,
        buy_value: 49_951_200,
        sell_value: 51_168_000,
        vwap: 162_050,
        gross: 1_216_800,
        dated_precise: [4000, 3357, 101, 12_600, 1500, 1342, 0, 0, 0, 0, 22_900],
        dated_worst: [4000, 3357, 101, 12_800, 1500, 1342, 1250, 225, 0, 0, 24_575],
        today_precise: [4000, 3104, 101, 12_600, 1500, 1297, 0, 0, 0, 0, 22_602],
        today_worst: [4000, 3105, 101, 12_800, 1500, 1298, 1250, 225, 0, 0, 24_279],
        headline: 24_575,
        net: 1_192_225,
    },
    Vector {
        id: "CN18",
        date: (2024, 9, 30),
        orders: &[
            (Buy, false, &[(51, 987_500)]),
            (Sell, false, &[(51, 980_000)]),
        ],
        quantity: 51,
        buy_value: 50_362_500,
        sell_value: 49_980_000,
        vwap: 983_750,
        gross: -382_500,
        dated_precise: [4000, 3331, 100, 12_500, 1500, 1338, 0, 0, 0, 0, 22_769],
        dated_worst: [4000, 3331, 100, 12_500, 1500, 1338, 1250, 225, 0, 0, 24_244],
        today_precise: [4000, 3081, 100, 12_500, 1500, 1293, 0, 0, 0, 0, 22_474],
        today_worst: [4000, 3081, 100, 12_500, 1500, 1293, 1250, 225, 0, 0, 23_949],
        headline: 24_244,
        net: -406_744,
    },
    Vector {
        id: "CN19",
        date: (2024, 10, 1),
        orders: &[
            (Buy, false, &[(51, 987_500)]),
            (Sell, false, &[(51, 980_000)]),
        ],
        quantity: 51,
        buy_value: 50_362_500,
        sell_value: 49_980_000,
        vwap: 983_750,
        gross: -382_500,
        dated_precise: [4000, 3081, 100, 12_500, 1500, 1293, 0, 0, 0, 0, 22_474],
        dated_worst: [4000, 3081, 100, 12_500, 1500, 1293, 1250, 225, 0, 0, 23_949],
        today_precise: [4000, 3081, 100, 12_500, 1500, 1293, 0, 0, 0, 0, 22_474],
        today_worst: [4000, 3081, 100, 12_500, 1500, 1293, 1250, 225, 0, 0, 23_949],
        headline: 23_949,
        net: -406_449,
    },
    Vector {
        id: "CN20",
        date: (2026, 2, 27),
        orders: &[
            (Sell, false, &[(40, 500_000), (35, 499_950), (25, 499_900)]),
            (Buy, false, &[(60, 495_000), (40, 495_150)]),
        ],
        quantity: 100,
        buy_value: 49_506_000,
        sell_value: 49_995_750,
        vwap: 497_509,
        gross: 489_750,
        dated_precise: [4000, 3055, 100, 12_400, 1500, 1288, 0, 0, 0, 0, 22_343],
        dated_worst: [
            10_000, 3055, 100, 12_500, 1500, 2368, 1250, 225, 0, 0, 30_998,
        ],
        today_precise: [4000, 3055, 100, 12_400, 1500, 1288, 0, 0, 0, 0, 22_343],
        today_worst: [
            10_000, 3055, 100, 12_500, 1500, 2368, 1250, 225, 0, 0, 30_998,
        ],
        headline: 30_998,
        net: 458_752,
    },
    Vector {
        id: "CN21",
        date: (2026, 3, 2),
        orders: &[
            (Sell, false, &[(40, 500_000), (35, 499_950), (25, 499_900)]),
            (Buy, false, &[(60, 495_000), (40, 495_150)]),
        ],
        quantity: 100,
        buy_value: 49_506_000,
        sell_value: 49_995_750,
        vwap: 497_509,
        gross: 489_750,
        dated_precise: [4000, 3055, 100, 12_400, 1500, 1288, 0, 0, 0, 0, 22_343],
        dated_worst: [
            10_000, 3055, 100, 12_500, 1500, 2368, 1250, 225, 0, 0, 30_998,
        ],
        today_precise: [4000, 3055, 100, 12_400, 1500, 1288, 0, 0, 0, 0, 22_343],
        today_worst: [
            10_000, 3055, 100, 12_500, 1500, 2368, 1250, 225, 0, 0, 30_998,
        ],
        headline: 30_998,
        net: 458_752,
    },
    Vector {
        id: "CN22",
        date: (2026, 10, 30),
        orders: &[
            (
                Buy,
                false,
                &[
                    (40, 160_000),
                    (40, 160_005),
                    (40, 160_010),
                    (40, 160_010),
                    (40, 160_020),
                    (40, 160_025),
                    (40, 160_030),
                    (33, 160_040),
                ],
            ),
            (Sell, false, &[(120, 161_700), (80, 161_650)]),
            (Sell, true, &[(113, 161_105)]),
        ],
        quantity: 313,
        buy_value: 50_085_320,
        sell_value: 50_540_865,
        vwap: 160_745,
        gross: 455_545,
        dated_precise: [6000, 3089, 101, 12_600, 1500, 1654, 0, 0, 2000, 360, 27_304],
        dated_worst: [
            21_024, 3090, 101, 12_700, 1500, 4359, 1250, 225, 2000, 360, 46_609,
        ],
        today_precise: [6000, 3089, 101, 12_600, 1500, 1654, 0, 0, 2000, 360, 27_304],
        today_worst: [
            21_024, 3090, 101, 12_700, 1500, 4359, 1250, 225, 2000, 360, 46_609,
        ],
        headline: 46_609,
        net: 408_936,
    },
    Vector {
        id: "CN23",
        date: (2026, 10, 30),
        orders: &[
            (
                Buy,
                false,
                &[
                    (40, 160_000),
                    (40, 160_005),
                    (40, 160_010),
                    (40, 160_010),
                    (40, 160_020),
                    (40, 160_025),
                    (40, 160_030),
                    (32, 160_040),
                ],
            ),
            (Sell, false, &[(120, 161_700), (80, 161_650)]),
            (Sell, true, &[(112, 161_105)]),
        ],
        quantity: 312,
        buy_value: 49_925_280,
        sell_value: 50_379_760,
        vwap: 160_745,
        gross: 454_480,
        dated_precise: [6000, 3079, 100, 12_500, 1500, 1652, 0, 0, 2000, 360, 27_191],
        dated_worst: [
            20_976, 3080, 100, 12_600, 1500, 4348, 1250, 225, 2000, 360, 46_439,
        ],
        today_precise: [6000, 3079, 100, 12_500, 1500, 1652, 0, 0, 2000, 360, 27_191],
        today_worst: [
            20_976, 3080, 100, 12_600, 1500, 4348, 1250, 225, 2000, 360, 46_439,
        ],
        headline: 46_439,
        net: 408_041,
    },
    Vector {
        id: "CN24",
        date: (2026, 10, 30),
        orders: &[
            (
                Buy,
                false,
                &[
                    (313, 16_000),
                    (313, 16_000),
                    (313, 16_005),
                    (312, 16_005),
                    (312, 16_010),
                    (312, 16_010),
                    (312, 16_015),
                    (312, 16_015),
                    (313, 16_020),
                    (313, 16_020),
                ],
            ),
            (Sell, false, &[(3125, 15_840)]),
        ],
        quantity: 3125,
        buy_value: 50_031_245,
        sell_value: 49_500_000,
        vwap: 15_925,
        gross: -531_245,
        dated_precise: [4000, 3056, 100, 12_400, 1500, 1288, 0, 0, 0, 0, 22_344],
        dated_worst: [
            17_009, 3056, 100, 12_400, 1500, 3630, 1250, 225, 0, 0, 39_170,
        ],
        today_precise: [4000, 3056, 100, 12_400, 1500, 1288, 0, 0, 0, 0, 22_344],
        today_worst: [
            17_009, 3056, 100, 12_400, 1500, 3630, 1250, 225, 0, 0, 39_170,
        ],
        headline: 39_170,
        net: -570_415,
    },
];

fn day((year, month, d): (u16, u8, u8)) -> TradeDay {
    TradeDay::new(year, month, d).expect("a real date")
}

/// A vector's orders as fills, each order's fills one second apart.
fn trade(orders: Orders<'_>) -> TradeFills {
    TradeFills {
        orders: orders
            .iter()
            .zip(0_i64..)
            .map(|(&(side, by_broker, fills), at)| Order {
                side,
                by_broker,
                fills: fills
                    .iter()
                    .zip(0_i64..)
                    .map(|(&(quantity, price), second)| Fill {
                        second: at * 1_000 + second,
                        quantity,
                        price: Paisa::from_raw(price),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn raw(lines: [Paisa; 11]) -> [i64; 11] {
    lines.map(Paisa::raw)
}

/// COSTS.md's worst column plus the one paisa D-8113 adds: GST on DP at
/// CGST + SGST, and the total.
fn plus_dp_paisa(mut column: [i64; 11]) -> [i64; 11] {
    column[7] += 1;
    column[10] += 1;
    column
}

#[test]
fn every_contract_note_matches_costs_md_to_the_paisa_under_all_four_columns() {
    for v in &VECTORS {
        let on = day(v.date);
        let fills = trade(v.orders);
        let priced = price_trade(&fills, on).expect("a vector prices");
        let input = NoteInput::from_fills(&fills, on).expect("a vector is a trade");
        // The trade itself.
        assert_eq!(priced.day(), on, "{}", v.id);
        assert_eq!(priced.quantity(), v.quantity, "{}", v.id);
        assert_eq!(priced.buy_value().raw(), v.buy_value, "{}", v.id);
        assert_eq!(priced.sell_value().raw(), v.sell_value, "{}", v.id);
        assert_eq!(input.vwap().raw(), v.vwap, "{}", v.id);
        assert_eq!(priced.gross().raw(), v.gross, "{}", v.id);
        assert_eq!(priced.version(), EQUITY_DATED_VERSION);
        // Columns 1 and 3: the official method on each clock.
        assert_eq!(
            raw(priced.dated_official().lines()),
            v.dated_precise,
            "{} dated",
            v.id
        );
        assert_eq!(
            raw(priced.today_official().lines()),
            v.today_precise,
            "{} today",
            v.id
        );
        assert_eq!(official(&input, Clock::Dated), Ok(priced.dated_official()));
        assert_eq!(official(&input, Clock::Today), Ok(priced.today_official()));
        // Columns 2 and 4: COSTS.md's worst reading set to the paisa.
        for (clock, readings, column) in [
            (Clock::Dated, priced.dated_readings(), v.dated_worst),
            (Clock::Today, priced.today_readings(), v.today_worst),
        ] {
            let costs_md = ReadingSet {
                dp_gst: GstRounding::Once,
                ..readings
            };
            let lines = note(&input, clock, &costs_md).expect("in the window");
            assert_eq!(raw(lines.lines()), column, "{} {clock:?} COSTS.md", v.id);
        }
        // The crate's worst: COSTS.md's plus D-8113's paisa.
        assert_eq!(
            raw(priced.dated_worst().lines()),
            plus_dp_paisa(v.dated_worst),
            "{} dated worst",
            v.id
        );
        assert_eq!(
            raw(priced.today_worst().lines()),
            plus_dp_paisa(v.today_worst),
            "{} today worst",
            v.id
        );
        assert_eq!(priced.dated_readings().dp_gst, GstRounding::CgstSgst);
        // The headline and the net.
        assert_eq!(priced.headline().raw(), v.headline + 1, "{}", v.id);
        assert_eq!(priced.net().raw(), v.net - 1, "{}", v.id);
        assert_eq!(priced.net().raw(), v.gross - priced.headline().raw());
        let from = if v.dated_worst[10] > v.today_worst[10] {
            HeadlineFrom::Dated
        } else {
            HeadlineFrom::Both
        };
        assert_eq!(priced.headline_from(), from, "{}", v.id);
        // worst() alone gives the same lines and readings.
        assert_eq!(
            worst(&input, Clock::Dated),
            Ok((priced.dated_worst(), priced.dated_readings()))
        );
        assert_eq!(
            worst(&input, Clock::Today),
            Ok((priced.today_worst(), priced.today_readings()))
        );
    }
}

#[test]
fn each_vector_reads_the_rows_of_its_own_day() {
    // (nse, ipft, sebi, stamp, GST on SEBI) on each vector's day: both sides
    // of every boundary of COSTS.md section 2.
    let want = [
        ["EX-2009", "IP-OLD", "SF-2017", "SD-STATE", "-"],
        ["EX-2009", "IP-OLD", "SF-2017", "SD-STATE", "-"],
        ["EX-2009", "IP-OLD", "SF-2019", "SD-STATE", "-"],
        ["EX-2009", "IP-OLD", "SF-2019", "SD-STATE", "-"],
        ["EX-2009", "IP-OLD", "SF-2020", "SD-STATE", "-"],
        ["EX-2009", "IP-OLD", "SF-2020", "SD-STATE", "-"],
        ["EX-2009", "IP-OLD", "SF-2020", "SD-ND", "-"],
        ["EX-2009", "IP-OLD", "SF-2020", "SD-ND", "-"],
        ["EX-2021", "IP-OLD", "SF-2020", "SD-ND", "-"],
        ["EX-2021", "IP-OLD", "SF-2020", "SD-ND", "-"],
        ["EX-2021", "IP-OLD", "SF-2021", "SD-ND", "-"],
        ["EX-2021", "IP-OLD", "SF-2021", "SD-ND", "-"],
        ["EX-2021", "IP-OLD", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2021", "IP-OLD", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2023", "IP-2023", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2023", "IP-2023", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2024A", "IP-2023", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2024A", "IP-2023", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2024B", "IP-2023", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2024B", "IP-2023", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2026", "IP-2026", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2026", "IP-2026", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2026", "IP-2026", "SF-2021", "SD-ND", "GST-SEBI"],
        ["EX-2026", "IP-2026", "SF-2021", "SD-ND", "GST-SEBI"],
    ];
    for (v, want) in VECTORS.iter().zip(want) {
        let priced = price_trade(&trade(v.orders), day(v.date)).expect("prices");
        let ids = priced.dated_rates().row_ids();
        assert_eq!([ids[3], ids[4], ids[5], ids[7], ids[9]], want, "{}", v.id);
        assert_eq!(priced.today_rates().row_ids()[3], "EX-2026");
    }
}

#[test]
fn the_crates_golden_vectors_v1_to_v5_hold_on_the_today_clock() {
    // crates/costs/src/stack.rs's five golden contract notes (and
    // costs-evidence/calc/golden.js): brokerage, exchange, SEBI, STT, stamp,
    // GST, total, in paise, priced on 2026-10-30 at today's rates.
    let golden: [(Orders<'_>, [i64; 7]); 5] = [
        (
            &[
                (Buy, false, &[(50, 1_520_00)]),
                (Sell, false, &[(50, 1_530_00)]),
            ],
            [40_00, 4_68, 15, 19_00, 2_00, 8_07, 73_90],
        ),
        (
            &[
                (Buy, false, &[(1_000, 1_000_00)]),
                (Sell, false, &[(1_000, 1_010_00)]),
            ],
            [40_00, 61_71, 2_01, 251_00, 30_00, 18_67, 403_39],
        ),
        (
            &[(Buy, false, &[(10, 100_00)]), (Sell, false, &[(10, 99_00)])],
            [60, 6, 0, 0, 0, 12, 78],
        ),
        (
            &[
                (Buy, false, &[(100_000, 50_00)]),
                (Sell, false, &[(100_000, 50_50)]),
            ],
            [40_00, 308_54, 10_05, 1_256_00, 150_00, 64_55, 1_829_14],
        ),
        (
            &[
                (Sell, false, &[(500, 2_000_00)]),
                (Buy, false, &[(500, 1_980_00)]),
            ],
            [40_00, 61_09, 1_99, 249_00, 30_00, 18_55, 400_63],
        ),
    ];
    for (orders, want) in golden {
        let input =
            NoteInput::from_fills(&trade(orders), day((2026, 10, 30))).expect("a golden trade");
        let lines = official(&input, Clock::Today).expect("today").lines();
        assert_eq!(
            [
                lines[0].raw(),
                lines[1].raw(),
                lines[2].raw(),
                lines[3].raw(),
                lines[4].raw(),
                lines[5].raw(),
                lines[10].raw()
            ],
            want
        );
    }
}

#[test]
fn the_quantity_tie_is_priced_both_ways() {
    // Rs 5,00,000 / Rs 1,600 = 312.5: CN22 prices 313, CN23 prices 312
    // (RD-TIE; FNO-NET pins which one a trade takes).
    let cn22 = price_trade(&trade(VECTORS[21].orders), day((2026, 10, 30))).expect("CN22");
    let cn23 = price_trade(&trade(VECTORS[22].orders), day((2026, 10, 30))).expect("CN23");
    assert_eq!((cn22.quantity(), cn23.quantity()), (313, 312));
    assert!(cn22.headline() > cn23.headline());
    assert_eq!(
        cn22.headline().raw() - cn23.headline().raw(),
        466_09 - 464_39
    );
}

#[test]
fn a_day_outside_the_window_is_refused_by_name_on_both_clocks() {
    let fills = trade(VECTORS[0].orders);
    let before = price_trade(&fills, day((2018, 8, 31))).expect_err("before the window");
    assert!(before.to_string().contains("2018-09-03"), "{before}");
    let after = price_trade(&fills, day((2026, 11, 2))).expect_err("after the read day");
    assert!(after.to_string().contains("2026-10-30"), "{after}");
    for clock in [Clock::Dated, Clock::Today] {
        let input = NoteInput::from_fills(&fills, day((2026, 11, 2))).expect("a trade");
        assert!(official(&input, clock).is_err());
        assert!(worst(&input, clock).is_err());
    }
}

/// Each set of `sets` under each of `choices` for one field.
fn widen<T: Copy>(
    sets: Vec<ReadingSet>,
    choices: &[T],
    set: impl Fn(&mut ReadingSet, T),
) -> Vec<ReadingSet> {
    let mut out = Vec::with_capacity(sets.len() * choices.len());
    for base in sets {
        for &choice in choices {
            let mut one = base;
            set(&mut one, choice);
            out.push(one);
        }
    }
    out
}

/// Every reading set: every combination of every field's readings, with
/// `states` as the State choices.
fn every_reading_set(states: &[StateChoice]) -> Vec<ReadingSet> {
    use BrokerageReading as B;
    use ExchangeRounding as X;
    use GstRounding as G;
    use SttReading as T;
    use TaxRounding as R;
    let sets = vec![ReadingSet::OFFICIAL];
    let sets = widen(sets, &[B::PerOrder, B::PerFill], |r, v| r.brokerage = v);
    let sets = widen(
        sets,
        &[T::Vwap, T::SellValue, T::SellValuePerOrder],
        |r, v| {
            r.stt = v;
        },
    );
    let sets = widen(
        sets,
        &[R::ClientDay, R::SecurityDay, R::PerOrder],
        |r, v| {
            r.stamp = v;
        },
    );
    let sets = widen(
        sets,
        &[X::Once, X::Split, X::PerOrder, X::PerOrderSplit],
        |r, v| {
            r.exchange = v;
        },
    );
    let sets = widen(
        sets,
        &[SebiRounding::Once, SebiRounding::PerOrder],
        |r, v| {
            r.sebi = v;
        },
    );
    let sets = widen(sets, &[G::Once, G::CgstSgst], |r, v| r.gst = v);
    let sets = widen(
        sets,
        &[DpReading::NotCharged, DpReading::Intraday],
        |r, v| {
            r.dp = v;
        },
    );
    let sets = widen(sets, &[G::Once, G::CgstSgst], |r, v| r.dp_gst = v);
    let sets = widen(
        sets,
        &[SquareOffReading::OnTop, SquareOffReading::Instead],
        |r, v| {
            r.square_off = v;
        },
    );
    let sets = widen(
        sets,
        &[SquareOffSplit::OnePerOrder, SquareOffSplit::OnePerFill],
        |r, v| {
            r.square_off_split = v;
        },
    );
    let sets = widen(sets, states, |r, v| r.state = v);
    widen(
        sets,
        &[StateRounding::Paisa, StateRounding::Rupee],
        |r, v| {
            r.state_rounding = v;
        },
    )
}

#[test]
fn the_worst_is_at_least_every_reading_set_and_the_worst_set_reproduces_it() {
    // CN01 (State stamp, every State named), CN22 (per-fill brokerage and a
    // square-off) and CN24 (fills under Rs 66,666.67), on both clocks.
    let mut states = vec![StateChoice::Worst];
    states.extend(STATE_STAMPS.iter().map(StateChoice::Named));
    for (v, states) in [
        (&VECTORS[0], &states[..]),
        (&VECTORS[21], &[StateChoice::Worst][..]),
        (&VECTORS[23], &[StateChoice::Worst][..]),
    ] {
        let input = NoteInput::from_fills(&trade(v.orders), day(v.date)).expect("a trade");
        let sets = every_reading_set(states);
        for clock in [Clock::Dated, Clock::Today] {
            let (lines, readings) = worst(&input, clock).expect("in the window");
            assert_eq!(
                note(&input, clock, &readings),
                Ok(lines),
                "{} {clock:?}",
                v.id
            );
            let mut reached = false;
            for set in &sets {
                let each = note(&input, clock, set).expect("in the window");
                for (line, top) in each.lines().iter().zip(lines.lines()) {
                    assert!(line <= &top, "{} {clock:?} {set:?}", v.id);
                }
                reached |= each == lines;
            }
            assert!(reached, "{} {clock:?}: the worst is one reading set", v.id);
        }
    }
}

/// A seeded xorshift: the property tests' only randomness, and no
/// dependency.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: u64) -> i64 {
        i64::try_from(self.next() % bound).unwrap()
    }
}

/// A random squared trade: one to three entry orders, one to three exits,
/// the last exit sometimes the broker's, one to six fills an order.
fn random_trade(rng: &mut XorShift) -> (TradeFills, TradeDay) {
    let (open, close) = if rng.below(2) == 0 {
        (Buy, Sell)
    } else {
        (Sell, Buy)
    };
    let base = 50_00 + rng.below(9_950_00);
    let mut orders = Vec::new();
    let mut second = 0;
    let mut quantity = 0;
    for _ in 0..=rng.below(3) {
        let mut fills = Vec::new();
        for _ in 0..=rng.below(6) {
            let q = 3 + rng.below(400);
            quantity += q;
            second += 1 + rng.below(3);
            fills.push(Fill {
                second,
                quantity: q,
                price: Paisa::from_raw(base + rng.below(500)),
            });
        }
        orders.push(Order {
            side: open,
            by_broker: false,
            fills,
        });
    }
    // At least three units (every fill is), so each of up to three exits
    // takes at least one.
    let exits = 1 + rng.below(3);
    let share = quantity / exits;
    for exit in 0..exits {
        let last = exit == exits - 1;
        let mut left = if last {
            quantity - share * (exits - 1)
        } else {
            share
        };
        let mut fills = Vec::new();
        while left > 0 {
            let q = (1 + rng.below(300)).min(left);
            left -= q;
            second += 1 + rng.below(3);
            fills.push(Fill {
                second,
                quantity: q,
                price: Paisa::from_raw(base * (90 + rng.below(21)) / 100),
            });
        }
        orders.push(Order {
            side: close,
            by_broker: last && rng.below(3) == 0,
            fills,
        });
    }
    let days = 2_980 - 1;
    let on = day((2018, 9, 3))
        .plus_days(i32::try_from(rng.below(days)).unwrap())
        .unwrap();
    (TradeFills { orders }, on)
}

#[test]
fn for_random_trades_the_headline_is_at_least_every_clock_and_worst_at_least_official() {
    for seed in 1..=400_u64 {
        let mut rng = XorShift(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let (fills, on) = random_trade(&mut rng);
        let priced = price_trade(&fills, on).expect("a random squared trade prices");
        let headline = priced.headline();
        for total in [
            priced.dated_official().total,
            priced.dated_worst().total,
            priced.today_official().total,
            priced.today_worst().total,
        ] {
            assert!(headline >= total, "seed {seed}");
        }
        for (official, worst) in [
            (priced.dated_official(), priced.dated_worst()),
            (priced.today_official(), priced.today_worst()),
        ] {
            for (o, w) in official.lines().iter().zip(worst.lines()) {
                assert!(*o <= w, "seed {seed}");
            }
        }
        assert_eq!(priced.net().raw(), priced.gross().raw() - headline.raw());
        assert_eq!(
            headline,
            priced.dated_worst().total.max(priced.today_worst().total)
        );
        // Each line of a note sums to its total.
        for lines in [priced.dated_official(), priced.dated_worst()] {
            let sum: i64 = lines.lines()[..10].iter().map(|line| line.raw()).sum();
            assert_eq!(sum, lines.total.raw(), "seed {seed}");
        }
    }
}

// ---------------------------------------------------------------------------
// Each reading alone, and every pair that interacts. The expected figures were
// computed with COSTS.md's own calculator (costs-evidence/calc/vec.js) and by
// hand, never by this crate.
// ---------------------------------------------------------------------------

/// Two buy orders, the first in two seconds, one own sell order and one the
/// broker's: Rs 1,00,100 bought, Rs 1,01,250 sold, 2026-10-30.
const SPREAD_TRADE: Orders<'static> = &[
    (Buy, false, &[(100, 40_000), (100, 40_050)]),
    (Buy, false, &[(50, 40_100)]),
    (Sell, false, &[(125, 40_525)]),
    (Sell, true, &[(125, 40_475)]),
];

/// `orders` on `date` as a note input.
fn input_of(orders: Orders<'_>, date: (u16, u8, u8)) -> NoteInput {
    NoteInput::from_fills(&trade(orders), day(date)).expect("a squared trade")
}

/// One note's lines, paise, under the today clock.
fn today_lines(input: &NoteInput, readings: ReadingSet) -> [i64; 11] {
    raw(note(input, Clock::Today, &readings)
        .expect("in the window")
        .lines())
}

#[test]
fn each_reading_alone_moves_only_its_own_lines() {
    let input = input_of(SPREAD_TRADE, (2026, 10, 30));
    let base = [5640, 618, 20, 2500, 300, 1130, 0, 0, 2000, 360, 12_568];
    let alone = |readings| today_lines(&input, readings);
    assert_eq!(alone(ReadingSet::OFFICIAL), base);
    // RD-BRK: Rs 20 + Rs 6.02 + Rs 15.20 + Rs 15.18 once per order; Rs 12.00 +
    // Rs 12.02 on the first order's two fills. GST follows its base.
    assert_eq!(
        alone(ReadingSet {
            brokerage: BrokerageReading::PerFill,
            ..ReadingSet::OFFICIAL
        }),
        [6042, 618, 20, 2500, 300, 1202, 0, 0, 2000, 360, 13_042]
    );
    // RD-STT-BASE: the sell value is Rs 25.31, to the rupee the VWAP basis's
    // Rs 25; each sell order alone is Rs 12.66 and Rs 12.65, Rs 13 each.
    assert_eq!(
        alone(ReadingSet {
            stt: SttReading::SellValue,
            ..ReadingSet::OFFICIAL
        }),
        base
    );
    assert_eq!(
        alone(ReadingSet {
            stt: SttReading::SellValuePerOrder,
            ..ReadingSet::OFFICIAL
        }),
        [5640, 618, 20, 2600, 300, 1130, 0, 0, 2000, 360, 12_668]
    );
    // RD-ROUND-TAX: Rs 3.00 the day; Rs 2.40 and Rs 0.60 are Rs 2 + Rs 1.
    for stamp in [TaxRounding::SecurityDay, TaxRounding::PerOrder] {
        assert_eq!(
            alone(ReadingSet {
                stamp,
                ..ReadingSet::OFFICIAL
            }),
            base
        );
    }
    // RD-ROUND-EX: one line and two lines round the same here; per order the
    // four half-ups add a paisa, under GST's rounding.
    assert_eq!(
        alone(ReadingSet {
            exchange: ExchangeRounding::Split,
            ..ReadingSet::OFFICIAL
        }),
        base
    );
    for exchange in [ExchangeRounding::PerOrder, ExchangeRounding::PerOrderSplit] {
        assert_eq!(
            alone(ReadingSet {
                exchange,
                ..ReadingSet::OFFICIAL
            }),
            [5640, 619, 20, 2500, 300, 1130, 0, 0, 2000, 360, 12_569]
        );
    }
    assert_eq!(
        alone(ReadingSet {
            sebi: SebiRounding::PerOrder,
            ..ReadingSet::OFFICIAL
        }),
        base
    );
    // RD-GST on Rs 62.78: 18% is Rs 11.30; 9% is Rs 5.65 twice.
    assert_eq!(
        alone(ReadingSet {
            gst: GstRounding::CgstSgst,
            ..ReadingSet::OFFICIAL
        }),
        base
    );
    // RD-DP: Rs 12.50 and its GST, Rs 2.25 once or Rs 1.13 twice; DP's GST
    // rounding moves nothing while DP is not charged.
    assert_eq!(
        alone(ReadingSet {
            dp: DpReading::Intraday,
            ..ReadingSet::OFFICIAL
        }),
        [5640, 618, 20, 2500, 300, 1130, 1250, 225, 2000, 360, 14_043]
    );
    assert_eq!(
        alone(ReadingSet {
            dp_gst: GstRounding::CgstSgst,
            ..ReadingSet::OFFICIAL
        }),
        base
    );
    assert_eq!(
        alone(ReadingSet {
            dp: DpReading::Intraday,
            dp_gst: GstRounding::CgstSgst,
            ..ReadingSet::OFFICIAL
        }),
        [5640, 618, 20, 2500, 300, 1130, 1250, 226, 2000, 360, 14_044]
    );
    // RD-SQ: the fee instead of the broker order's Rs 15.18.
    assert_eq!(
        alone(ReadingSet {
            square_off: SquareOffReading::Instead,
            ..ReadingSet::OFFICIAL
        }),
        [4122, 618, 20, 2500, 300, 857, 0, 0, 2000, 360, 10_777]
    );
    // The pair: per fill and instead take the broker's fill out of the per-fill
    // sum, and GST follows.
    assert_eq!(
        alone(ReadingSet {
            brokerage: BrokerageReading::PerFill,
            square_off: SquareOffReading::Instead,
            ..ReadingSet::OFFICIAL
        }),
        [4524, 618, 20, 2500, 300, 929, 0, 0, 2000, 360, 11_251]
    );
    // The State readings say nothing on a day of the unified stamp.
    assert_eq!(
        alone(ReadingSet {
            state: StateChoice::Named(&STATE_STAMPS[0]),
            state_rounding: StateRounding::Rupee,
            ..ReadingSet::OFFICIAL
        }),
        base
    );
    // And the worst of them all, with the set that gives it.
    let (lines, readings) = worst(&input, Clock::Today).expect("in the window");
    assert_eq!(
        raw(lines.lines()),
        [6042, 619, 20, 2600, 300, 1203, 1250, 226, 2000, 360, 14_620]
    );
    assert_eq!(
        readings,
        ReadingSet {
            brokerage: BrokerageReading::PerFill,
            stt: SttReading::SellValuePerOrder,
            stamp: TaxRounding::ClientDay,
            exchange: ExchangeRounding::PerOrder,
            sebi: SebiRounding::Once,
            gst: GstRounding::Once,
            dp: DpReading::Intraday,
            dp_gst: GstRounding::CgstSgst,
            square_off: SquareOffReading::OnTop,
            square_off_split: SquareOffSplit::OnePerOrder,
            state: StateChoice::Worst,
            state_rounding: StateRounding::Paisa,
            slab: SlabReading::Top,
        }
    );
}

#[test]
fn per_order_rounding_is_taken_where_it_is_the_larger_and_refused_where_it_is_not() {
    // Rs 20,000 and Rs 20,000 bought, Rs 20,000, Rs 10,000 and Rs 10,000 sold.
    // Stamp Rs 1.20 the day is Rs 1; Rs 0.60 an order is Rs 1 + Rs 1. STT Rs 10
    // the day; Rs 5.00 + Rs 2.50 + Rs 2.50 per order is Rs 5 + Rs 3 + Rs 3.
    // The exchange line per order (Rs 2.45) is under the day's (Rs 2.46), so
    // the day's stays.
    let input = input_of(
        &[
            (Buy, false, &[(20, 100_000)]),
            (Buy, false, &[(20, 100_000)]),
            (Sell, false, &[(20, 100_000)]),
            (Sell, false, &[(10, 100_000)]),
            (Sell, false, &[(10, 100_000)]),
        ],
        (2026, 10, 30),
    );
    assert_eq!(
        raw(official(&input, Clock::Today).expect("today").lines()),
        [2400, 246, 8, 1000, 100, 478, 0, 0, 0, 0, 4232]
    );
    let (lines, readings) = worst(&input, Clock::Today).expect("today");
    assert_eq!(
        raw(lines.lines()),
        [2400, 246, 8, 1100, 200, 478, 1250, 226, 0, 0, 5908]
    );
    assert_eq!(readings.stamp, TaxRounding::PerOrder);
    assert_eq!(readings.stt, SttReading::SellValuePerOrder);
    assert_eq!(readings.exchange, ExchangeRounding::Once);
    assert_eq!(
        today_lines(
            &input,
            ReadingSet {
                exchange: ExchangeRounding::PerOrder,
                ..ReadingSet::OFFICIAL
            }
        )[1],
        245
    );
    // Rs 5,000 an order, six orders. The SEBI fee is half a paisa an order, so
    // one paisa each, against three on the day; GST's base moves with it.
    let small: Orders<'_> = &[
        (Buy, false, &[(5, 100_000)]),
        (Buy, false, &[(5, 100_000)]),
        (Buy, false, &[(5, 100_000)]),
        (Sell, false, &[(5, 100_000)]),
        (Sell, false, &[(5, 100_000)]),
        (Sell, false, &[(5, 100_000)]),
    ];
    let input = input_of(small, (2026, 10, 30));
    assert_eq!(
        raw(official(&input, Clock::Today).expect("today").lines()),
        [900, 92, 3, 400, 0, 179, 0, 0, 0, 0, 1574]
    );
    let (lines, readings) = worst(&input, Clock::Today).expect("today");
    assert_eq!(
        raw(lines.lines()),
        [900, 92, 6, 400, 0, 180, 1250, 226, 0, 0, 3054]
    );
    assert_eq!(readings.sebi, SebiRounding::PerOrder);
    assert_eq!(readings.stt, SttReading::Vwap);
    assert_eq!(readings.stamp, TaxRounding::ClientDay);
}

#[test]
fn the_exchange_line_split_in_two_is_taken_where_it_is_the_larger() {
    // Rs 50 lakh of turnover: Rs 153.495 and Rs 0.005 each round up, against
    // Rs 153.50 on one line.
    let input = input_of(
        &[
            (Buy, false, &[(2_500, 100_000)]),
            (Sell, false, &[(2_500, 100_000)]),
        ],
        (2026, 10, 30),
    );
    let (lines, readings) = worst(&input, Clock::Today).expect("today");
    assert_eq!(lines.exchange.raw(), 15_351);
    assert_eq!(readings.exchange, ExchangeRounding::Split);
    assert_eq!(
        official(&input, Clock::Today)
            .expect("today")
            .exchange
            .raw(),
        15_350
    );
}

#[test]
fn stt_takes_the_sell_value_when_it_is_larger_and_the_vwap_when_it_is_not() {
    // CN04, a winning long: the sell value is above the buy value, Rs 127.76
    // to Rs 128 against the VWAP basis's Rs 126.
    let winner = &VECTORS[3];
    let input = input_of(winner.orders, winner.date);
    assert!(input.sell_value() > input.buy_value());
    let (lines, readings) = worst(&input, Clock::Dated).expect("in the window");
    assert_eq!(
        (lines.stt.raw(), readings.stt),
        (12_800, SttReading::SellValue)
    );
    assert_eq!(
        official(&input, Clock::Dated).expect("window").stt.raw(),
        12_600
    );
    // CN09, a losing short: the buy value is above the sell value, so the VWAP
    // basis's Rs 126 beats the sell value's Rs 125.
    let loser = &VECTORS[8];
    let input = input_of(loser.orders, loser.date);
    assert!(input.buy_value() > input.sell_value());
    let (lines, readings) = worst(&input, Clock::Dated).expect("in the window");
    assert_eq!((lines.stt.raw(), readings.stt), (12_600, SttReading::Vwap));
}

#[test]
fn the_exact_half_paisa_vwap_rounds_up_and_stt_is_taken_at_it() {
    // CN14 and CN15: Rs 9,90,105.45 over 12,846 units is exactly Rs 77.075.
    for v in [&VECTORS[13], &VECTORS[14]] {
        let input = input_of(v.orders, v.date);
        let turnover = input.buy_value().raw() + input.sell_value().raw();
        assert_eq!(turnover, 99_010_545);
        // Twice the VWAP is a whole 15,415 paise: the VWAP is exactly x.5.
        assert_eq!(turnover, 15_415 * input.quantity());
        assert_eq!(input.vwap().raw(), 7_708);
        // 6,423 x Rs 77.08 x 0.025% is Rs 123.77, Rs 124 to the rupee.
        assert_eq!(
            official(&input, Clock::Dated).expect("window").stt.raw(),
            12_400
        );
    }
}

#[test]
fn per_fill_brokerage_counts_under_the_cap_and_stops_at_it() {
    // CN24: ten fills of about Rs 50,000, each under the Rs 66,666.67 that
    // reaches the cap, so per fill is 0.03% of each, Rs 150.09, plus the sell
    // order's Rs 20, against Rs 40 once per order.
    let cn24 = &VECTORS[23];
    let input = input_of(cn24.orders, cn24.date);
    let (lines, readings) = worst(&input, Clock::Dated).expect("in the window");
    assert_eq!(lines.brokerage.raw(), 17_009);
    assert_eq!(readings.brokerage, BrokerageReading::PerFill);
    assert_eq!(
        official(&input, Clock::Dated)
            .expect("window")
            .brokerage
            .raw(),
        4_000
    );
    // The half-up edge of the cap, one fill each side: Rs 66,649.99 is
    // Rs 19.99; Rs 66,650.00 is Rs 19.995, half-up to the cap; Rs 66,666.67
    // and Rs 10 lakh are the cap.
    let sized: Orders<'_> = &[
        (
            Buy,
            false,
            &[
                (1, 6_664_999),
                (1, 6_665_000),
                (1, 6_666_667),
                (1, 100_000_000),
            ],
        ),
        (Sell, false, &[(4, 100_000)]),
    ];
    let input = input_of(sized, (2026, 10, 30));
    assert_eq!(
        input.orders()[0].brokerage_per_fill,
        1_999 + 2_000 + 2_000 + 2_000
    );
    assert_eq!(input.orders()[1].brokerage_per_fill, 120);
    assert_eq!(
        today_lines(
            &input,
            ReadingSet {
                brokerage: BrokerageReading::PerFill,
                ..ReadingSet::OFFICIAL
            }
        )[0],
        7_999 + 120
    );
    assert_eq!(today_lines(&input, ReadingSet::OFFICIAL)[0], 2_000 + 120);
}

#[test]
fn gst_rounding_takes_whichever_is_larger_on_the_actual_base() {
    // CN01 on the today clock: Rs 71.86 is Rs 12.93 at 18% and Rs 6.47 twice,
    // Rs 12.94. CN22: Rs 242.15 is Rs 43.59 at 18% and Rs 21.79 twice, Rs 43.58.
    let cn01 = input_of(VECTORS[0].orders, VECTORS[0].date);
    let (lines, readings) = worst(&cn01, Clock::Today).expect("today");
    assert_eq!(
        (lines.gst.raw(), readings.gst),
        (1_294, GstRounding::CgstSgst)
    );
    let cn22 = input_of(VECTORS[21].orders, VECTORS[21].date);
    let (lines, readings) = worst(&cn22, Clock::Today).expect("today");
    assert_eq!((lines.gst.raw(), readings.gst), (4_359, GstRounding::Once));
    assert_eq!(lines.square_off_gst.raw(), 360);
}

#[test]
fn the_square_off_fee_is_per_broker_order_on_top_and_gst_is_its_own_line() {
    // CN22 and CN23: one broker order, Rs 20 and Rs 3.60, on top of its
    // brokerage.
    for v in [&VECTORS[21], &VECTORS[22]] {
        let input = input_of(v.orders, v.date);
        for clock in [Clock::Dated, Clock::Today] {
            let lines = official(&input, clock).expect("in the window");
            assert_eq!(
                (lines.square_off.raw(), lines.square_off_gst.raw()),
                (2_000, 360)
            );
            assert_eq!(lines.brokerage.raw(), 6_000);
        }
    }
    // Two broker orders pay it twice, however many seconds each filled in; a
    // trade the broker closes alone keeps the entry's brokerage under "instead".
    let closed_by_broker: Orders<'_> = &[
        (Buy, false, &[(100, 50_000)]),
        (Sell, true, &[(30, 49_000), (30, 49_000)]),
        (Sell, true, &[(40, 48_900)]),
    ];
    let input = input_of(closed_by_broker, (2026, 10, 30));
    let lines = official(&input, Clock::Today).expect("today");
    assert_eq!(
        (lines.square_off.raw(), lines.square_off_gst.raw()),
        (4_000, 720)
    );
    assert_eq!(lines.brokerage.raw(), 1_500 + 882 + 587);
    assert_eq!(
        today_lines(
            &input,
            ReadingSet {
                square_off: SquareOffReading::Instead,
                ..ReadingSet::OFFICIAL
            }
        )[0],
        1_500
    );
    assert_eq!(
        today_lines(
            &input,
            ReadingSet {
                brokerage: BrokerageReading::PerFill,
                square_off: SquareOffReading::Instead,
                ..ReadingSet::OFFICIAL
            }
        )[0],
        1_500
    );
    // SPEC.md OP-27: an auto square-off in several seconds as several orders
    // for its fee: three fills, Rs 60 and Rs 10.80. The worst takes it.
    assert_eq!(
        today_lines(
            &input,
            ReadingSet {
                square_off_split: SquareOffSplit::OnePerFill,
                ..ReadingSet::OFFICIAL
            }
        )[8..10],
        [6_000, 1_080]
    );
    let (lines, readings) = worst(&input, Clock::Today).expect("today");
    assert_eq!(
        (lines.square_off.raw(), lines.square_off_gst.raw()),
        (6_000, 1_080)
    );
    assert_eq!(readings.square_off_split, SquareOffSplit::OnePerFill);
    // No broker order, no fee on any reading.
    let plain = input_of(VECTORS[0].orders, VECTORS[0].date);
    assert_eq!(
        official(&plain, Clock::Today)
            .expect("today")
            .square_off
            .raw(),
        0
    );
}

#[test]
fn every_rate_change_moves_exactly_its_own_line_on_the_trading_days_either_side() {
    // Rs 1 crore a side at one price, so every line is the rate itself: the
    // dated official note on the trading day before and after each boundary
    // of COSTS.md section 2, from costs-evidence/calc/vec.js. Brokerage is
    // Rs 40 and STT Rs 25,000 on all of them.
    let crore: Orders<'_> = &[
        (Buy, false, &[(100_000, 100_000)]),
        (Sell, false, &[(100_000, 100_000)]),
    ];
    // (day, exchange + IPFT, SEBI, stamp, GST, total), in paise.
    let table = [
        (
            (2019, 3, 29),
            650_020,
            30_000,
            1_200_000,
            117_724,
            4_501_744,
        ),
        ((2019, 4, 1), 650_020, 20_000, 1_200_000, 117_724, 4_491_744),
        (
            (2020, 5, 29),
            650_020,
            20_000,
            1_200_000,
            117_724,
            4_491_744,
        ),
        ((2020, 6, 1), 650_020, 10_000, 1_200_000, 117_724, 4_481_744),
        (
            (2020, 6, 30),
            650_020,
            10_000,
            1_200_000,
            117_724,
            4_481_744,
        ),
        ((2020, 7, 1), 650_020, 10_000, 300_000, 117_724, 3_581_744),
        ((2020, 12, 31), 650_020, 10_000, 300_000, 117_724, 3_581_744),
        ((2021, 1, 1), 690_020, 10_000, 300_000, 124_924, 3_628_944),
        ((2021, 3, 31), 690_020, 10_000, 300_000, 124_924, 3_628_944),
        ((2021, 4, 1), 690_020, 20_000, 300_000, 124_924, 3_638_944),
        ((2022, 7, 15), 690_020, 20_000, 300_000, 124_924, 3_638_944),
        ((2022, 7, 18), 690_020, 20_000, 300_000, 128_524, 3_642_544),
        ((2023, 3, 31), 690_020, 20_000, 300_000, 128_524, 3_642_544),
        ((2023, 4, 3), 670_000, 20_000, 300_000, 124_920, 3_618_920),
        ((2024, 3, 28), 670_000, 20_000, 300_000, 124_920, 3_618_920),
        ((2024, 4, 1), 664_000, 20_000, 300_000, 123_840, 3_611_840),
        ((2024, 9, 30), 664_000, 20_000, 300_000, 123_840, 3_611_840),
        ((2024, 10, 1), 614_000, 20_000, 300_000, 114_840, 3_552_840),
        ((2026, 2, 27), 614_000, 20_000, 300_000, 114_840, 3_552_840),
        ((2026, 3, 2), 614_000, 20_000, 300_000, 114_840, 3_552_840),
    ];
    let today = [
        4_000, 614_000, 20_000, 2_500_000, 300_000, 114_840, 0, 0, 0, 0, 3_552_840,
    ];
    for (date, exchange, sebi, stamp, gst, total) in table {
        let input = input_of(crore, date);
        let lines = official(&input, Clock::Dated).expect("in the window");
        assert_eq!(
            raw(lines.lines()),
            [
                4_000, exchange, sebi, 2_500_000, stamp, gst, 0, 0, 0, 0, total
            ],
            "{date:?}"
        );
        // The today clock never moves.
        assert_eq!(
            raw(official(&input, Clock::Today).expect("today").lines()),
            today,
            "{date:?}"
        );
    }
    // The split that moved no total: 297 + 10 against 306.99 + 0.01 price one
    // figure, and their rows differ.
    let before = input_of(crore, (2026, 2, 27));
    let after = input_of(crore, (2026, 3, 2));
    assert_eq!(
        official(&before, Clock::Dated),
        official(&after, Clock::Dated)
    );
    let split = ReadingSet {
        exchange: ExchangeRounding::Split,
        ..ReadingSet::OFFICIAL
    };
    assert_eq!(
        note(&before, Clock::Dated, &split),
        note(&after, Clock::Dated, &split)
    );
}

#[test]
fn every_state_stamp_is_priced_and_the_worst_is_no_lower_than_any_of_them() {
    // CN06: Rs 9,94,643.65 on 2020-06-30. Telangana's 0.01% is Rs 99.46; named
    // States price their own rows, to the paisa or the rupee.
    let cn06 = &VECTORS[5];
    let input = input_of(cn06.orders, cn06.date);
    let named = |code: &str, state_rounding| {
        let state = STATE_STAMPS
            .iter()
            .find(|state| state.code == code)
            .expect("a listed State");
        note(
            &input,
            Clock::Dated,
            &ReadingSet {
                state: StateChoice::Named(state),
                state_rounding,
                ..ReadingSet::OFFICIAL
            },
        )
        .expect("in the window")
        .stamp
        .raw()
    };
    assert_eq!(named("TS", StateRounding::Paisa), 9_946);
    assert_eq!(named("TS", StateRounding::Rupee), 9_900);
    assert_eq!(named("TN", StateRounding::Paisa), 5_968);
    assert_eq!(named("TN", StateRounding::Rupee), 6_000);
    assert_eq!(named("DL_GJ_KL_MH", StateRounding::Paisa), 1_989);
    assert_eq!(named("HP", StateRounding::Paisa), 5_000);
    assert_eq!(named("AS", StateRounding::Paisa), 4_950);
    let (lines, readings) = worst(&input, Clock::Dated).expect("in the window");
    assert_eq!(lines.stamp.raw(), 9_946);
    assert_eq!(readings.state_rounding, StateRounding::Paisa);
    for state in &STATE_STAMPS {
        for state_rounding in [StateRounding::Paisa, StateRounding::Rupee] {
            assert!(
                named(state.code, state_rounding) <= lines.stamp.raw(),
                "{}",
                state.code
            );
        }
    }
    // CN03: Telangana's Rs 99.59 rounds up to Rs 100, so the worst takes the
    // rupee; the official keeps the paisa. The day after the State law ended
    // the same trade is the unified 0.003%, and the State readings say nothing.
    let cn03 = input_of(VECTORS[2].orders, VECTORS[2].date);
    let (lines, readings) = worst(&cn03, Clock::Dated).expect("in the window");
    assert_eq!(
        (lines.stamp.raw(), readings.state_rounding),
        (10_000, StateRounding::Rupee)
    );
    assert_eq!(
        official(&cn03, Clock::Dated).expect("window").stamp.raw(),
        9_959
    );
    let unified = input_of(cn06.orders, (2020, 7, 1));
    assert_eq!(
        official(&unified, Clock::Dated)
            .expect("window")
            .stamp
            .raw(),
        1_500
    );
}

#[test]
fn the_smallest_trade_prices_to_nothing_but_the_dp_reading() {
    // One share at one paisa, both ways: every percentage rounds to nothing.
    let input = input_of(
        &[(Buy, false, &[(1, 1)]), (Sell, false, &[(1, 1)])],
        (2026, 10, 30),
    );
    assert_eq!(input.vwap().raw(), 1);
    assert_eq!(
        raw(official(&input, Clock::Today).expect("today").lines()),
        [0; 11]
    );
    assert_eq!(
        raw(worst(&input, Clock::Today).expect("today").0.lines()),
        [0, 0, 0, 0, 0, 0, 1250, 226, 0, 0, 1476]
    );
    // On a State-stamp day Himachal Pradesh's flat Rs 50 is the floor.
    let early = input_of(
        &[(Buy, false, &[(1, 1)]), (Sell, false, &[(1, 1)])],
        (2018, 9, 3),
    );
    assert_eq!(
        official(&early, Clock::Dated).expect("window").stamp.raw(),
        5_000
    );
    // A flat trade's gross is nothing and its net is the headline's negative.
    let priced = price_trade(
        &trade(&[(Buy, false, &[(1, 1)]), (Sell, false, &[(1, 1)])]),
        day((2026, 10, 30)),
    )
    .expect("prices");
    assert_eq!(priced.gross().raw(), 0);
    assert_eq!(priced.net().raw(), -priced.headline().raw());
}

#[test]
fn the_headline_comes_from_the_dated_clock_until_the_rates_agree_on_every_day_of_the_window() {
    // A plain Rs 5 lakh a side, priced on every calendar day of the window:
    // dearer on the dated clock before 2024-10-01 (COSTS.md section 1), the
    // same figure from then on.
    let fills = trade(&[
        (Buy, false, &[(500, 100_000)]),
        (Sell, false, &[(500, 100_000)]),
    ]);
    let flat_from = day((2024, 10, 1));
    let mut on = day((2018, 9, 3));
    let mut days = 0;
    while !day((2026, 10, 30)).before(on) {
        let priced = price_trade(&fills, on).expect("in the window");
        let from = if on.before(flat_from) {
            HeadlineFrom::Dated
        } else {
            HeadlineFrom::Both
        };
        assert_eq!(priced.headline_from(), from, "{on}");
        on = on.plus_days(1).expect("inside the calendar");
        days += 1;
    }
    assert_eq!(days, 2_980);
}

#[test]
fn the_new_note_and_the_crates_own_stack_agree_on_every_vector_they_both_price() {
    use brutex_core::instrument::Exchange;
    use costs::ledger::{DayLedger, ExecutedOrder};
    use costs::schedule::{Mode, Product, RateSelector, TradeKind};

    let lines_of = |day: &costs::stack::DayCharges| {
        [
            day.brokerage().raw(),
            day.exchange_charge().raw(),
            day.sebi().raw(),
            day.stt().raw(),
            day.stamp().raw(),
            day.gst().raw(),
            day.total().raw(),
        ]
    };
    let first_dated = day((2022, 7, 18));
    let mut compared = 0;
    for v in &VECTORS {
        let on = day(v.date);
        let input = input_of(v.orders, v.date);
        let mut ledger = DayLedger::new(TradeKind::Equity(Product::Intraday), Exchange::Nse, on)
            .expect("priced");
        for order in input.orders() {
            ledger
                .add(ExecutedOrder {
                    side: order.side,
                    qty: order.quantity,
                    value_paisa: Paisa::from_raw(order.value),
                    broker_square_off: order.by_broker,
                })
                .expect("in range");
        }
        // The stack's total carries the square-off fee and its GST; so does
        // the note's.
        let note_lines = |clock| {
            let note = raw(official(&input, clock).expect("window").lines());
            [
                note[0], note[1], note[2], note[3], note[4], note[5], note[10],
            ]
        };
        let today = ledger
            .close(Mode::Precise, RateSelector::CurrentOnEveryDate)
            .expect("today's rates price every day");
        assert_eq!(lines_of(&today), note_lines(Clock::Today), "{}", v.id);
        let dated = ledger.close(Mode::Precise, RateSelector::Dated);
        if on.before(first_dated) {
            // D-8111: this stack cannot carry the days before GST on the SEBI
            // fee, and says which function does.
            let refused = dated.expect_err("before 2022-07-18").to_string();
            assert!(refused.contains("price_trade"), "{}: {refused}", v.id);
        } else {
            let dated = dated.expect("a dated intraday day");
            assert_eq!(lines_of(&dated), note_lines(Clock::Dated), "{}", v.id);
            compared += 1;
        }
    }
    assert_eq!(compared, 12);
}

#[test]
fn a_trade_prices_the_same_every_time_and_in_either_order_of_its_same_side_orders() {
    let first = price_trade(&trade(SPREAD_TRADE), day((2026, 10, 30))).expect("prices");
    let again = price_trade(&trade(SPREAD_TRADE), day((2026, 10, 30))).expect("prices");
    assert_eq!(first, again);
    let swapped: Orders<'_> = &[
        (Buy, false, &[(50, 40_100)]),
        (Buy, false, &[(100, 40_000), (100, 40_050)]),
        (Sell, false, &[(125, 40_525)]),
        (Sell, true, &[(125, 40_475)]),
    ];
    let other = price_trade(&trade(swapped), day((2026, 10, 30))).expect("prices");
    assert_eq!(other.today_official(), first.today_official());
    assert_eq!(other.today_worst(), first.today_worst());
    assert_eq!(other.headline(), first.headline());
    // A long and the same prices short: sides swap, so STT and stamp follow
    // the side, not the order they came in.
    let long = trade(&[
        (Buy, false, &[(10, 10_000)]),
        (Sell, false, &[(10, 10_100)]),
    ]);
    let short = trade(&[
        (Sell, false, &[(10, 10_100)]),
        (Buy, false, &[(10, 10_000)]),
    ]);
    let (long, short) = (
        price_trade(&long, day((2026, 10, 30))).expect("prices"),
        price_trade(&short, day((2026, 10, 30))).expect("prices"),
    );
    assert_eq!(long.today_official(), short.today_official());
    assert_eq!(long.gross(), short.gross());
}

#[test]
fn for_random_trades_every_reading_set_on_every_clock_is_under_the_worst() {
    // A coarser walk than the 3 vectors' whole product: one seeded reading set
    // per field per trade, over many trades and days.
    let all = every_reading_set(&[
        StateChoice::Worst,
        StateChoice::Named(&STATE_STAMPS[10]),
        StateChoice::Named(&STATE_STAMPS[13]),
    ]);
    let count = u64::try_from(all.len()).unwrap();
    for seed in 1..=200_u64 {
        let mut rng = XorShift(seed.wrapping_mul(0xD6E8_FEB8_6659_FD93) | 1);
        let (fills, on) = random_trade(&mut rng);
        let input = NoteInput::from_fills(&fills, on).expect("a squared trade");
        for clock in [Clock::Dated, Clock::Today] {
            let (lines, readings) = worst(&input, clock).expect("in the window");
            assert_eq!(note(&input, clock, &readings), Ok(lines), "seed {seed}");
            for _ in 0..32 {
                let set = &all[usize::try_from(rng.next() % count).unwrap()];
                let each = note(&input, clock, set).expect("in the window");
                for (line, top) in each.lines().iter().zip(lines.lines()) {
                    assert!(line <= &top, "seed {seed} {clock:?} {set:?}");
                }
            }
        }
    }
}
