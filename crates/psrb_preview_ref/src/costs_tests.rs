#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unreadable_literal,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

/// The side of an order in a contract-note vector.
#[derive(Clone, Copy)]
enum Side {
    Buy,
    Sell,
}

/// One order of a vector: its side, `by_broker`, and its fills (shares, paisa).
type OrderRow = (Side, bool, &'static [(i64, i64)]);

/// One of COSTS section 9's 24 contract notes (`costs-table.json` `test_vectors`), every
/// amount in paisa; lines in the order brokerage, exchange + IPFT, SEBI, STT, stamp, GST,
/// DP, DP GST, square-off, square-off GST, total.
struct Vector {
    id: &'static str,
    day: (i32, u32, u32),
    orders: &'static [OrderRow],
    dated_precise: [i128; 11],
    dated_worst: [i128; 11],
    today_precise: [i128; 11],
    today_worst: [i128; 11],
    headline: i128,
    net: i128,
}

static VECTORS: [Vector; 24] = [
    Vector {
        id: "CN01",
        day: (2018, 9, 3),
        orders: &[
            (Side::Buy, false, &[(400, 125000)]),
            (Side::Sell, false, &[(400, 126250)]),
        ],
        dated_precise: [4000, 3266, 151, 12600, 10000, 1308, 0, 0, 0, 0, 31325],
        dated_worst: [4000, 3266, 151, 12600, 10000, 1308, 1250, 225, 0, 0, 32800],
        today_precise: [4000, 3085, 101, 12600, 1500, 1293, 0, 0, 0, 0, 22579],
        today_worst: [4000, 3085, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24055],
        headline: 32800,
        net: 467200,
    },
    Vector {
        id: "CN02",
        day: (2019, 3, 29),
        orders: &[
            (Side::Sell, false, &[(615, 81235)]),
            (Side::Buy, false, &[(615, 80500)]),
        ],
        dated_precise: [4000, 3233, 149, 12400, 9947, 1302, 0, 0, 0, 0, 31031],
        dated_worst: [4000, 3233, 149, 12500, 9947, 1302, 1250, 225, 0, 0, 32606],
        today_precise: [4000, 3054, 99, 12400, 1500, 1288, 0, 0, 0, 0, 22341],
        today_worst: [4000, 3054, 100, 12500, 1500, 1288, 1250, 225, 0, 0, 23917],
        headline: 32606,
        net: 419419,
    },
    Vector {
        id: "CN03",
        day: (2019, 4, 1),
        orders: &[
            (Side::Buy, false, &[(213, 234565)]),
            (Side::Sell, false, &[(213, 233010)]),
        ],
        dated_precise: [4000, 3237, 100, 12400, 9959, 1303, 0, 0, 0, 0, 30999],
        dated_worst: [4000, 3237, 100, 12400, 10000, 1303, 1250, 225, 0, 0, 32515],
        today_precise: [4000, 3058, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22346],
        today_worst: [4000, 3058, 100, 12400, 1500, 1288, 1250, 225, 0, 0, 23821],
        headline: 32515,
        net: -363730,
    },
    Vector {
        id: "CN04",
        day: (2020, 5, 29),
        orders: &[
            (Side::Buy, false, &[(3197, 15640)]),
            (Side::Sell, false, &[(3197, 15985)]),
        ],
        dated_precise: [4000, 3286, 101, 12600, 10000, 1311, 0, 0, 0, 0, 31298],
        dated_worst: [4000, 3286, 101, 12800, 10000, 1312, 1250, 225, 0, 0, 32974],
        today_precise: [4000, 3104, 101, 12600, 1500, 1297, 0, 0, 0, 0, 22602],
        today_worst: [4000, 3104, 101, 12800, 1500, 1297, 1250, 225, 0, 0, 24277],
        headline: 32974,
        net: 1069991,
    },
    Vector {
        id: "CN05",
        day: (2020, 6, 1),
        orders: &[
            (Side::Sell, false, &[(124, 402000)]),
            (Side::Buy, false, &[(124, 399055)]),
        ],
        dated_precise: [4000, 3228, 50, 12400, 9933, 1301, 0, 0, 0, 0, 30912],
        dated_worst: [4000, 3228, 50, 12500, 9933, 1302, 1250, 225, 0, 0, 32488],
        today_precise: [4000, 3049, 99, 12400, 1500, 1287, 0, 0, 0, 0, 22335],
        today_worst: [4000, 3049, 99, 12500, 1500, 1287, 1250, 225, 0, 0, 23910],
        headline: 32488,
        net: 332692,
    },
    Vector {
        id: "CN06",
        day: (2020, 6, 30),
        orders: &[
            (Side::Buy, false, &[(833, 60000)]),
            (Side::Sell, false, &[(833, 59405)]),
        ],
        dated_precise: [4000, 3233, 50, 12400, 9946, 1302, 0, 0, 0, 0, 30931],
        dated_worst: [4000, 3233, 50, 12400, 9946, 1302, 1250, 225, 0, 0, 32406],
        today_precise: [4000, 3054, 99, 12400, 1500, 1288, 0, 0, 0, 0, 22341],
        today_worst: [4000, 3054, 99, 12400, 1500, 1288, 1250, 225, 0, 0, 23816],
        headline: 32406,
        net: -528041,
    },
    Vector {
        id: "CN07",
        day: (2020, 7, 1),
        orders: &[
            (Side::Buy, false, &[(833, 60000)]),
            (Side::Sell, false, &[(833, 60600)]),
        ],
        dated_precise: [4000, 3265, 50, 12600, 1500, 1308, 0, 0, 0, 0, 22723],
        dated_worst: [4000, 3265, 50, 12600, 1500, 1308, 1250, 225, 0, 0, 24198],
        today_precise: [4000, 3084, 100, 12600, 1500, 1293, 0, 0, 0, 0, 22577],
        today_worst: [4000, 3084, 100, 12600, 1500, 1294, 1250, 225, 0, 0, 24053],
        headline: 24198,
        net: 475602,
    },
    Vector {
        id: "CN08",
        day: (2020, 12, 31),
        orders: &[
            (Side::Buy, false, &[(500, 100000)]),
            (Side::Sell, false, &[(500, 101500)]),
        ],
        dated_precise: [4000, 3274, 50, 12600, 1500, 1309, 0, 0, 0, 0, 22733],
        dated_worst: [4000, 3274, 50, 12700, 1500, 1310, 1250, 225, 0, 0, 24309],
        today_precise: [4000, 3093, 101, 12600, 1500, 1295, 0, 0, 0, 0, 22589],
        today_worst: [4000, 3093, 101, 12700, 1500, 1295, 1250, 225, 0, 0, 24164],
        headline: 24309,
        net: 725691,
    },
    Vector {
        id: "CN09",
        day: (2021, 1, 1),
        orders: &[
            (Side::Sell, false, &[(500, 100000)]),
            (Side::Buy, false, &[(500, 101200)]),
        ],
        dated_precise: [4000, 3471, 50, 12600, 1500, 1345, 0, 0, 0, 0, 22966],
        dated_worst: [4000, 3471, 50, 12600, 1500, 1345, 1250, 225, 0, 0, 24441],
        today_precise: [4000, 3088, 101, 12600, 1500, 1294, 0, 0, 0, 0, 22583],
        today_worst: [4000, 3088, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24058],
        headline: 24441,
        net: -624441,
    },
    Vector {
        id: "CN10",
        day: (2021, 3, 31),
        orders: &[
            (Side::Buy, false, &[(1000, 49995)]),
            (Side::Sell, false, &[(1000, 50500)]),
        ],
        dated_precise: [4000, 3467, 50, 12600, 1500, 1344, 0, 0, 0, 0, 22961],
        dated_worst: [4000, 3467, 50, 12600, 1500, 1344, 1250, 225, 0, 0, 24436],
        today_precise: [4000, 3085, 100, 12600, 1500, 1293, 0, 0, 0, 0, 22578],
        today_worst: [4000, 3085, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24055],
        headline: 24436,
        net: 480564,
    },
    Vector {
        id: "CN11",
        day: (2021, 4, 1),
        orders: &[
            (Side::Buy, false, &[(1000, 49995)]),
            (Side::Sell, false, &[(1000, 50500)]),
        ],
        dated_precise: [4000, 3467, 100, 12600, 1500, 1344, 0, 0, 0, 0, 23011],
        dated_worst: [4000, 3467, 101, 12600, 1500, 1344, 1250, 225, 0, 0, 24487],
        today_precise: [4000, 3085, 100, 12600, 1500, 1293, 0, 0, 0, 0, 22578],
        today_worst: [4000, 3085, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24055],
        headline: 24487,
        net: 480513,
    },
    Vector {
        id: "CN12",
        day: (2022, 7, 15),
        orders: &[
            (Side::Buy, false, &[(200, 250000)]),
            (Side::Sell, false, &[(200, 253750)]),
        ],
        dated_precise: [4000, 3476, 101, 12600, 1500, 1346, 0, 0, 0, 0, 23023],
        dated_worst: [4000, 3476, 101, 12700, 1500, 1346, 1250, 225, 0, 0, 24598],
        today_precise: [4000, 3093, 101, 12600, 1500, 1295, 0, 0, 0, 0, 22589],
        today_worst: [4000, 3093, 101, 12700, 1500, 1295, 1250, 225, 0, 0, 24164],
        headline: 24598,
        net: 725402,
    },
    Vector {
        id: "CN13",
        day: (2022, 7, 18),
        orders: &[
            (Side::Buy, false, &[(200, 250000)]),
            (Side::Sell, false, &[(200, 253750)]),
        ],
        dated_precise: [4000, 3476, 101, 12600, 1500, 1364, 0, 0, 0, 0, 23041],
        dated_worst: [4000, 3476, 101, 12700, 1500, 1364, 1250, 225, 0, 0, 24616],
        today_precise: [4000, 3093, 101, 12600, 1500, 1295, 0, 0, 0, 0, 22589],
        today_worst: [4000, 3093, 101, 12700, 1500, 1295, 1250, 225, 0, 0, 24164],
        headline: 24616,
        net: 725384,
    },
    Vector {
        id: "CN14",
        day: (2023, 3, 31),
        orders: &[
            (Side::Sell, false, &[(6423, 7785)]),
            (Side::Buy, false, &[(6423, 7630)]),
        ],
        dated_precise: [4000, 3416, 99, 12400, 1500, 1353, 0, 0, 0, 0, 22768],
        dated_worst: [4000, 3416, 99, 12500, 1500, 1353, 1250, 225, 0, 0, 24343],
        today_precise: [4000, 3040, 99, 12400, 1500, 1285, 0, 0, 0, 0, 22324],
        today_worst: [4000, 3040, 99, 12500, 1500, 1286, 1250, 225, 0, 0, 23900],
        headline: 24343,
        net: 971222,
    },
    Vector {
        id: "CN15",
        day: (2023, 4, 3),
        orders: &[
            (Side::Sell, false, &[(6423, 7785)]),
            (Side::Buy, false, &[(6423, 7630)]),
        ],
        dated_precise: [4000, 3317, 99, 12400, 1500, 1335, 0, 0, 0, 0, 22651],
        dated_worst: [4000, 3317, 99, 12500, 1500, 1335, 1250, 225, 0, 0, 24226],
        today_precise: [4000, 3040, 99, 12400, 1500, 1285, 0, 0, 0, 0, 22324],
        today_worst: [4000, 3040, 99, 12500, 1500, 1286, 1250, 225, 0, 0, 23900],
        headline: 24226,
        net: 971339,
    },
    Vector {
        id: "CN16",
        day: (2024, 3, 28),
        orders: &[
            (Side::Buy, false, &[(312, 160100)]),
            (Side::Sell, false, &[(312, 164000)]),
        ],
        dated_precise: [4000, 3387, 101, 12600, 1500, 1348, 0, 0, 0, 0, 22936],
        dated_worst: [4000, 3387, 101, 12800, 1500, 1348, 1250, 225, 0, 0, 24611],
        today_precise: [4000, 3104, 101, 12600, 1500, 1297, 0, 0, 0, 0, 22602],
        today_worst: [4000, 3105, 101, 12800, 1500, 1298, 1250, 225, 0, 0, 24279],
        headline: 24611,
        net: 1192189,
    },
    Vector {
        id: "CN17",
        day: (2024, 4, 1),
        orders: &[
            (Side::Buy, false, &[(312, 160100)]),
            (Side::Sell, false, &[(312, 164000)]),
        ],
        dated_precise: [4000, 3357, 101, 12600, 1500, 1342, 0, 0, 0, 0, 22900],
        dated_worst: [4000, 3357, 101, 12800, 1500, 1342, 1250, 225, 0, 0, 24575],
        today_precise: [4000, 3104, 101, 12600, 1500, 1297, 0, 0, 0, 0, 22602],
        today_worst: [4000, 3105, 101, 12800, 1500, 1298, 1250, 225, 0, 0, 24279],
        headline: 24575,
        net: 1192225,
    },
    Vector {
        id: "CN18",
        day: (2024, 9, 30),
        orders: &[
            (Side::Buy, false, &[(51, 987500)]),
            (Side::Sell, false, &[(51, 980000)]),
        ],
        dated_precise: [4000, 3331, 100, 12500, 1500, 1338, 0, 0, 0, 0, 22769],
        dated_worst: [4000, 3331, 100, 12500, 1500, 1338, 1250, 225, 0, 0, 24244],
        today_precise: [4000, 3081, 100, 12500, 1500, 1293, 0, 0, 0, 0, 22474],
        today_worst: [4000, 3081, 100, 12500, 1500, 1293, 1250, 225, 0, 0, 23949],
        headline: 24244,
        net: -406744,
    },
    Vector {
        id: "CN19",
        day: (2024, 10, 1),
        orders: &[
            (Side::Buy, false, &[(51, 987500)]),
            (Side::Sell, false, &[(51, 980000)]),
        ],
        dated_precise: [4000, 3081, 100, 12500, 1500, 1293, 0, 0, 0, 0, 22474],
        dated_worst: [4000, 3081, 100, 12500, 1500, 1293, 1250, 225, 0, 0, 23949],
        today_precise: [4000, 3081, 100, 12500, 1500, 1293, 0, 0, 0, 0, 22474],
        today_worst: [4000, 3081, 100, 12500, 1500, 1293, 1250, 225, 0, 0, 23949],
        headline: 23949,
        net: -406449,
    },
    Vector {
        id: "CN20",
        day: (2026, 2, 27),
        orders: &[
            (
                Side::Sell,
                false,
                &[(40, 500000), (35, 499950), (25, 499900)],
            ),
            (Side::Buy, false, &[(60, 495000), (40, 495150)]),
        ],
        dated_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343],
        dated_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998],
        today_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343],
        today_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998],
        headline: 30998,
        net: 458752,
    },
    Vector {
        id: "CN21",
        day: (2026, 3, 2),
        orders: &[
            (
                Side::Sell,
                false,
                &[(40, 500000), (35, 499950), (25, 499900)],
            ),
            (Side::Buy, false, &[(60, 495000), (40, 495150)]),
        ],
        dated_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343],
        dated_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998],
        today_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343],
        today_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998],
        headline: 30998,
        net: 458752,
    },
    Vector {
        id: "CN22",
        day: (2026, 10, 30),
        orders: &[
            (
                Side::Buy,
                false,
                &[
                    (40, 160000),
                    (40, 160005),
                    (40, 160010),
                    (40, 160010),
                    (40, 160020),
                    (40, 160025),
                    (40, 160030),
                    (33, 160040),
                ],
            ),
            (Side::Sell, false, &[(120, 161700), (80, 161650)]),
            (Side::Sell, true, &[(113, 161105)]),
        ],
        dated_precise: [6000, 3089, 101, 12600, 1500, 1654, 0, 0, 2000, 360, 27304],
        dated_worst: [
            21024, 3090, 101, 12700, 1500, 4359, 1250, 225, 2000, 360, 46609,
        ],
        today_precise: [6000, 3089, 101, 12600, 1500, 1654, 0, 0, 2000, 360, 27304],
        today_worst: [
            21024, 3090, 101, 12700, 1500, 4359, 1250, 225, 2000, 360, 46609,
        ],
        headline: 46609,
        net: 408936,
    },
    Vector {
        id: "CN23",
        day: (2026, 10, 30),
        orders: &[
            (
                Side::Buy,
                false,
                &[
                    (40, 160000),
                    (40, 160005),
                    (40, 160010),
                    (40, 160010),
                    (40, 160020),
                    (40, 160025),
                    (40, 160030),
                    (32, 160040),
                ],
            ),
            (Side::Sell, false, &[(120, 161700), (80, 161650)]),
            (Side::Sell, true, &[(112, 161105)]),
        ],
        dated_precise: [6000, 3079, 100, 12500, 1500, 1652, 0, 0, 2000, 360, 27191],
        dated_worst: [
            20976, 3080, 100, 12600, 1500, 4348, 1250, 225, 2000, 360, 46439,
        ],
        today_precise: [6000, 3079, 100, 12500, 1500, 1652, 0, 0, 2000, 360, 27191],
        today_worst: [
            20976, 3080, 100, 12600, 1500, 4348, 1250, 225, 2000, 360, 46439,
        ],
        headline: 46439,
        net: 408041,
    },
    Vector {
        id: "CN24",
        day: (2026, 10, 30),
        orders: &[
            (
                Side::Buy,
                false,
                &[
                    (313, 16000),
                    (313, 16000),
                    (313, 16005),
                    (312, 16005),
                    (312, 16010),
                    (312, 16010),
                    (312, 16015),
                    (312, 16015),
                    (313, 16020),
                    (313, 16020),
                ],
            ),
            (Side::Sell, false, &[(3125, 15840)]),
        ],
        dated_precise: [4000, 3056, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22344],
        dated_worst: [17009, 3056, 100, 12400, 1500, 3630, 1250, 225, 0, 0, 39170],
        today_precise: [4000, 3056, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22344],
        today_worst: [17009, 3056, 100, 12400, 1500, 3630, 1250, 225, 0, 0, 39170],
        headline: 39170,
        net: -570415,
    },
];

fn orders_of(v: &Vector) -> Vec<Order> {
    v.orders
        .iter()
        .map(|(side, by_broker, parts)| Order {
            buy: matches!(side, Side::Buy),
            by_broker: *by_broker,
            parts: parts
                .iter()
                .map(|(qty, price)| Part {
                    qty: *qty,
                    price: *price,
                })
                .collect(),
        })
        .collect()
}

fn with_total(lines: &Lines) -> [i128; 11] {
    let a = lines.as_array();
    [
        a[0],
        a[1],
        a[2],
        a[3],
        a[4],
        a[5],
        a[6],
        a[7],
        a[8],
        a[9],
        lines.total(),
    ]
}

fn gross_of(orders: &[Order]) -> i128 {
    orders
        .iter()
        .map(|o| if o.buy { -o.value() } else { o.value() })
        .sum()
}

#[test]
fn every_contract_note_vector_matches_line_by_line() {
    let mut misses = Vec::new();
    for v in &VECTORS {
        let day = Day::from_ymd(v.day.0, v.day.1, v.day.2).unwrap();
        let orders = orders_of(v);
        let p = price_trade(&orders, day).unwrap();
        let checks = [
            (
                "dated precise",
                with_total(&p.dated_precise),
                v.dated_precise,
            ),
            ("dated worst", with_total(&p.dated_worst), v.dated_worst),
            (
                "today precise",
                with_total(&p.today_precise),
                v.today_precise,
            ),
            ("today worst", with_total(&p.today_worst), v.today_worst),
        ];
        for (what, got, want) in checks {
            if got != want {
                misses.push(format!("{} {what}: got {got:?} want {want:?}", v.id));
            }
        }
        if p.headline != v.headline {
            misses.push(format!(
                "{} headline: got {} want {}",
                v.id, p.headline, v.headline
            ));
        }
        if gross_of(&orders) - p.headline != v.net {
            misses.push(format!(
                "{} net: got {} want {}",
                v.id,
                gross_of(&orders) - p.headline,
                v.net
            ));
        }
    }
    assert!(
        misses.is_empty(),
        "{} misses:\n{}",
        misses.len(),
        misses.join("\n")
    );
}

fn day(y: i32, m: u32, d: u32) -> Day {
    Day::from_ymd(y, m, d).unwrap()
}

fn one_trade(long: bool, qty: i64, entry: i64, exit: i64, broker: bool, d: Day) -> Priced {
    price_trade(&trade_orders(long, qty, entry, exit, broker), d).unwrap()
}

/// X-F42..X-F49: each line at its edges.
#[test]
fn x_f42_f49_line_extremes() {
    let d = day(2024, 10, 1);
    // brokerage: 0.03% under Rs 20, the cap above, the cross point at Rs 66,666.67
    assert_eq!(
        one_trade(true, 1, 1_000_000, 1_000_000, false, d)
            .today_worst
            .brokerage,
        600
    );
    assert_eq!(
        one_trade(true, 1, 6_666_667, 6_666_667, false, d)
            .today_worst
            .brokerage,
        4_000
    );
    assert_eq!(
        one_trade(true, 1, 6_666_666, 6_666_666, false, d)
            .today_worst
            .brokerage,
        2 * 2_000
    );
    assert_eq!(
        one_trade(true, 1, 6_666_500, 6_666_500, false, d)
            .today_worst
            .brokerage,
        2 * 2_000
    );
    assert_eq!(one_trade(true, 1, 1, 1, false, d).today_worst.brokerage, 0);
    // combo: one order under the cap and one over
    assert_eq!(
        one_trade(true, 100, 50_000, 700_000, false, d)
            .today_worst
            .brokerage,
        1_500 + 2_000
    );
    // the square-off: Rs 20 + GST on top of the exit's brokerage, only by the broker
    let sq = one_trade(true, 400, 125_000, 125_000, true, d);
    assert_eq!(
        (
            sq.today_worst.square_off,
            sq.today_worst.square_off_gst,
            sq.today_worst.brokerage
        ),
        (2_000, 360, 4_000)
    );
    assert_eq!(
        one_trade(true, 400, 125_000, 125_000, false, d)
            .today_worst
            .square_off,
        0
    );
    // DP: only in the worst reading, Rs 12.50 + Rs 2.25
    assert_eq!(
        (
            sq.today_precise.dp,
            sq.today_worst.dp,
            sq.today_worst.dp_gst
        ),
        (0, 1_250, 225)
    );
    // STT on the sell side only: a short's entry; a tiny sell rounds to 0 rupees
    let short = one_trade(false, 400, 125_000, 124_000, false, d);
    assert_eq!(short.today_worst.stt, 12_500);
    assert_eq!(one_trade(true, 1, 1, 1, false, d).today_worst.stt, 0);
    // stamp: 0.003% of the buy from 2020-07-01; the state levy before
    assert_eq!(
        one_trade(true, 400, 125_000, 130_000, false, d)
            .today_worst
            .stamp,
        1_500
    );
    assert_eq!(
        one_trade(true, 400, 125_000, 130_000, false, day(2020, 6, 30))
            .dated_worst
            .stamp,
        10_000
    );
    // GST on SEBI from 2022-07-18 on the dated clock (combo: the boundary)
    let before = one_trade(true, 400, 125_000, 125_000, false, day(2022, 7, 15)).dated_precise;
    let after = one_trade(true, 400, 125_000, 125_000, false, day(2022, 7, 18)).dated_precise;
    assert_eq!(
        before.gst,
        crate::money::half_up((before.brokerage + before.exchange) * 18, 100)
    );
    assert_eq!(
        after.gst,
        crate::money::half_up((after.brokerage + after.exchange + after.sebi) * 18, 100)
    );
    assert!(after.gst > before.gst);
    // oob: GST never on STT or stamp
    let p = one_trade(true, 400, 125_000, 126_000, false, d);
    assert_eq!(
        p.today_precise.gst,
        crate::money::half_up(
            (p.today_precise.brokerage + p.today_precise.exchange + p.today_precise.sebi) * 18,
            100
        )
    );
}

/// X-F45, X-F46, X-F48: every dated row at its first day and the day before it.
#[test]
fn x_f45_f46_f48_every_era_boundary() {
    let ex = |y, m, dd| dated_rates(day(y, m, dd)).exchange;
    let ip = |y, m, dd| dated_rates(day(y, m, dd)).ipft;
    let sf = |y, m, dd| dated_rates(day(y, m, dd)).sebi;
    assert_eq!(
        [
            ex(2020, 12, 31),
            ex(2021, 1, 1),
            ex(2023, 3, 31),
            ex(2023, 4, 1)
        ],
        [32_500, 34_500, 34_500, 32_500]
    );
    assert_eq!(
        [
            ex(2024, 3, 31),
            ex(2024, 4, 1),
            ex(2024, 9, 30),
            ex(2024, 10, 1)
        ],
        [32_500, 32_200, 32_200, 29_700]
    );
    assert_eq!([ex(2026, 2, 28), ex(2026, 3, 1)], [29_700, 30_699]);
    assert_eq!(
        [
            ip(2023, 3, 31),
            ip(2023, 4, 1),
            ip(2026, 2, 28),
            ip(2026, 3, 1)
        ],
        [1, 1_000, 1_000, 1]
    );
    assert_eq!(
        [
            sf(2019, 3, 31),
            sf(2019, 4, 1),
            sf(2020, 5, 31),
            sf(2020, 6, 1),
            sf(2021, 3, 31),
            sf(2021, 4, 1)
        ],
        [1_500, 1_000, 1_000, 500, 500, 1_000]
    );
    assert!(dated_rates(day(2020, 6, 30)).state_stamp && !dated_rates(day(2020, 7, 1)).state_stamp);
    assert!(
        !dated_rates(day(2022, 7, 17)).gst_on_sebi && dated_rates(day(2022, 7, 18)).gst_on_sebi
    );
    assert_eq!(today_rates(), dated_rates(day(2026, 10, 30)));
}

/// X-F48: the worst listed state across turnovers (COSTS section 3's table).
#[test]
fn x_f48_the_worst_state_by_turnover() {
    // Rs 10 lakh: Telangana's Rs 100 cap
    assert_eq!(state_stamp(100_000_000), 10_000);
    // above Rs 16.67 lakh Tamil Nadu's uncapped 0.006% passes it
    assert_eq!(state_stamp(200_000_000), 12_000);
    // Rs 3 lakh: Himachal's flat Rs 50 a day passes Telangana's Rs 30
    assert_eq!(state_stamp(30_000_000), 5_000);
    assert_eq!(state_stamp(0), 5_000);
    // Rs 6 lakh: Telangana's Rs 60
    assert_eq!(state_stamp(60_000_000), 6_000);
}

/// X-F51, X-F52: the headline, the net and the refusals.
#[test]
fn x_f51_f52_headline_and_net_extremes() {
    // from 2026-03-01 the two clocks agree: the dated clock on a tie (Q8)
    let tie = one_trade(true, 400, 125_000, 125_000, false, day(2026, 10, 30));
    assert_eq!(
        (tie.from, tie.headline),
        (Clock::Dated, tie.today_worst.total())
    );
    // before 2024-10-01 the dated clock is dearer
    assert_eq!(
        one_trade(true, 400, 125_000, 125_000, false, day(2023, 1, 2)).from,
        Clock::Dated
    );
    // the 2020-21 SEBI relief (Rs 5 a crore dated, Rs 10 today): on a Rs 5,065 trade both
    // exchange lines round to 16 paisa and today's SEBI line rounds to 1, so the today
    // clock is dearer by that paisa and is the headline
    let relief = one_trade(true, 1, 253_250, 253_250, false, day(2020, 9, 1));
    assert_eq!(
        (relief.dated_worst.exchange, relief.today_worst.exchange),
        (16, 16)
    );
    assert_eq!((relief.dated_worst.sebi, relief.today_worst.sebi), (0, 1));
    assert_eq!(relief.today_worst.total(), relief.dated_worst.total() + 1);
    assert_eq!(
        (relief.from, relief.headline),
        (Clock::Today, relief.today_worst.total())
    );
    // combo: 2024-10-01..2026-02-28, Rs 50 lakh of turnover: both clocks' rates sum to
    // Rs 307 a crore, but today's split lines round up twice (15,349.5 and 0.5 paisa) to
    // 15,351 against the dated clock's 15,350; GST ties at 3,574 and the today clock wins
    let big = one_trade(true, 100_000, 2_500, 2_500, false, day(2025, 1, 15));
    assert_eq!(
        (big.dated_worst.exchange, big.today_worst.exchange),
        (15_350, 15_351)
    );
    assert_eq!((big.dated_worst.gst, big.today_worst.gst), (3_574, 3_574));
    assert_eq!(big.today_worst.total(), big.dated_worst.total() + 1);
    assert_eq!(big.today_precise.total(), big.dated_precise.total());
    assert_eq!(
        (big.from, big.headline),
        (Clock::Today, big.today_worst.total())
    );
    // the same trade from 2026-03-01: one table, a tie, the dated clock
    let same = one_trade(true, 100_000, 2_500, 2_500, false, day(2026, 3, 2));
    assert_eq!(same.dated_worst, same.today_worst);
    assert_eq!(same.from, Clock::Dated);
    let cheap = price_trade(
        &trade_orders(true, 50_000_000, 1, 1, false),
        day(2020, 9, 1),
    )
    .unwrap();
    assert_eq!(
        cheap.headline,
        cheap.dated_worst.total().max(cheap.today_worst.total())
    );
    // net = gross - headline; a net of exactly 0 is neither win nor loss
    assert_eq!(
        gross_and_net(true, 400, 125_000, 126_000, 300),
        (400_000, 399_700)
    );
    assert_eq!(
        gross_and_net(false, 400, 125_000, 126_000, 300),
        (-400_000, -400_300)
    );
    assert_eq!(gross_and_net(true, 1, 100, 100 + 7, 7), (7, 0));
    // refusals stop the run by name
    assert_eq!(
        price_trade(&trade_orders(true, 1, 1, 1, false), day(2018, 9, 2)),
        Err(ChargeError::OutsideWindow(day(2018, 9, 2)))
    );
    assert_eq!(
        price_trade(&trade_orders(true, 1, 1, 1, false), day(2026, 10, 31)),
        Err(ChargeError::OutsideWindow(day(2026, 10, 31)))
    );
    assert_eq!(price_trade(&[], day(2020, 1, 1)), Err(ChargeError::Empty));
    let no_part = [Order {
        buy: true,
        by_broker: false,
        parts: vec![],
    }];
    assert_eq!(
        price_trade(&no_part, day(2020, 1, 1)),
        Err(ChargeError::Empty)
    );
    let open = [
        Order {
            buy: true,
            by_broker: false,
            parts: vec![Part { qty: 2, price: 5 }],
        },
        Order {
            buy: false,
            by_broker: false,
            parts: vec![Part { qty: 1, price: 5 }],
        },
    ];
    assert_eq!(
        price_trade(&open, day(2020, 1, 1)),
        Err(ChargeError::OpenIntradayPosition)
    );
    assert_eq!(
        check_window(),
        (Some(day(2018, 9, 3)), Some(day(2026, 10, 30)))
    );
}

/// P-F42..P-F52: 10,000 generated trades over the window: every worst line at or above
/// its official one, the totals the sums of their lines, the headline the larger clock.
#[test]
fn p_costs_hold_their_invariants() {
    let first = day(2018, 9, 3).index();
    for seed in crate::generator::SEEDS {
        let mut r = crate::rng::SplitMix64::new(seed);
        for _ in 0..2_500 {
            let d = Day::from_index(first + i32::try_from(r.below(2_980)).unwrap());
            let entry = i64::try_from(r.below(5_000_000)).unwrap() + 1;
            let exit = (entry + r.between(-entry / 10, entry / 10)).max(1);
            let qty = crate::sizing::qty(entry).max(1);
            let long = r.chance(1, 2);
            let p = one_trade(long, qty, entry, exit, r.chance(1, 5), d);
            for (precise, worst) in [
                (p.dated_precise, p.dated_worst),
                (p.today_precise, p.today_worst),
            ] {
                for (a, b) in precise.as_array().iter().zip(worst.as_array()) {
                    assert!(*a <= b, "{d:?} {precise:?} {worst:?}");
                }
                assert_eq!(worst.total(), worst.as_array().iter().sum::<i128>());
            }
            assert_eq!(p.headline, p.dated_worst.total().max(p.today_worst.total()));
            assert_eq!(
                p.from == Clock::Dated,
                p.dated_worst.total() >= p.today_worst.total()
            );
        }
    }
}
