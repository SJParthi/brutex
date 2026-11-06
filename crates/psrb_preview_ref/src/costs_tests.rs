#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

/// The side of an order in a contract-note vector.
#[derive(Clone, Copy)]
enum Side {
    Buy,
    Sell,
}

/// One of COSTS section 9's 24 contract notes (`costs-table.json` `test_vectors`), every
/// amount in paisa; lines in the order brokerage, exchange + IPFT, SEBI, STT, stamp, GST,
/// DP, DP GST, square-off, square-off GST, total.
struct Vector {
    id: &'static str,
    day: (i32, u32, u32),
    orders: &'static [(Side, bool, &'static [(i64, i64)])],
    dated_precise: [i128; 11],
    dated_worst: [i128; 11],
    today_precise: [i128; 11],
    today_worst: [i128; 11],
    headline: i128,
    net: i128,
}

const VECTORS: [Vector; 24] = [
    Vector { id: "CN01", day: (2018, 9, 3), orders: &[(Side::Buy, false, &[(400, 125000)]), (Side::Sell, false, &[(400, 126250)])], dated_precise: [4000, 3266, 151, 12600, 10000, 1308, 0, 0, 0, 0, 31325], dated_worst: [4000, 3266, 151, 12600, 10000, 1308, 1250, 225, 0, 0, 32800], today_precise: [4000, 3085, 101, 12600, 1500, 1293, 0, 0, 0, 0, 22579], today_worst: [4000, 3085, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24055], headline: 32800, net: 467200 },
    Vector { id: "CN02", day: (2019, 3, 29), orders: &[(Side::Sell, false, &[(615, 81235)]), (Side::Buy, false, &[(615, 80500)])], dated_precise: [4000, 3233, 149, 12400, 9947, 1302, 0, 0, 0, 0, 31031], dated_worst: [4000, 3233, 149, 12500, 9947, 1302, 1250, 225, 0, 0, 32606], today_precise: [4000, 3054, 99, 12400, 1500, 1288, 0, 0, 0, 0, 22341], today_worst: [4000, 3054, 100, 12500, 1500, 1288, 1250, 225, 0, 0, 23917], headline: 32606, net: 419419 },
    Vector { id: "CN03", day: (2019, 4, 1), orders: &[(Side::Buy, false, &[(213, 234565)]), (Side::Sell, false, &[(213, 233010)])], dated_precise: [4000, 3237, 100, 12400, 9959, 1303, 0, 0, 0, 0, 30999], dated_worst: [4000, 3237, 100, 12400, 10000, 1303, 1250, 225, 0, 0, 32515], today_precise: [4000, 3058, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22346], today_worst: [4000, 3058, 100, 12400, 1500, 1288, 1250, 225, 0, 0, 23821], headline: 32515, net: -363730 },
    Vector { id: "CN04", day: (2020, 5, 29), orders: &[(Side::Buy, false, &[(3197, 15640)]), (Side::Sell, false, &[(3197, 15985)])], dated_precise: [4000, 3286, 101, 12600, 10000, 1311, 0, 0, 0, 0, 31298], dated_worst: [4000, 3286, 101, 12800, 10000, 1312, 1250, 225, 0, 0, 32974], today_precise: [4000, 3104, 101, 12600, 1500, 1297, 0, 0, 0, 0, 22602], today_worst: [4000, 3104, 101, 12800, 1500, 1297, 1250, 225, 0, 0, 24277], headline: 32974, net: 1069991 },
    Vector { id: "CN05", day: (2020, 6, 1), orders: &[(Side::Sell, false, &[(124, 402000)]), (Side::Buy, false, &[(124, 399055)])], dated_precise: [4000, 3228, 50, 12400, 9933, 1301, 0, 0, 0, 0, 30912], dated_worst: [4000, 3228, 50, 12500, 9933, 1302, 1250, 225, 0, 0, 32488], today_precise: [4000, 3049, 99, 12400, 1500, 1287, 0, 0, 0, 0, 22335], today_worst: [4000, 3049, 99, 12500, 1500, 1287, 1250, 225, 0, 0, 23910], headline: 32488, net: 332692 },
    Vector { id: "CN06", day: (2020, 6, 30), orders: &[(Side::Buy, false, &[(833, 60000)]), (Side::Sell, false, &[(833, 59405)])], dated_precise: [4000, 3233, 50, 12400, 9946, 1302, 0, 0, 0, 0, 30931], dated_worst: [4000, 3233, 50, 12400, 9946, 1302, 1250, 225, 0, 0, 32406], today_precise: [4000, 3054, 99, 12400, 1500, 1288, 0, 0, 0, 0, 22341], today_worst: [4000, 3054, 99, 12400, 1500, 1288, 1250, 225, 0, 0, 23816], headline: 32406, net: -528041 },
    Vector { id: "CN07", day: (2020, 7, 1), orders: &[(Side::Buy, false, &[(833, 60000)]), (Side::Sell, false, &[(833, 60600)])], dated_precise: [4000, 3265, 50, 12600, 1500, 1308, 0, 0, 0, 0, 22723], dated_worst: [4000, 3265, 50, 12600, 1500, 1308, 1250, 225, 0, 0, 24198], today_precise: [4000, 3084, 100, 12600, 1500, 1293, 0, 0, 0, 0, 22577], today_worst: [4000, 3084, 100, 12600, 1500, 1294, 1250, 225, 0, 0, 24053], headline: 24198, net: 475602 },
    Vector { id: "CN08", day: (2020, 12, 31), orders: &[(Side::Buy, false, &[(500, 100000)]), (Side::Sell, false, &[(500, 101500)])], dated_precise: [4000, 3274, 50, 12600, 1500, 1309, 0, 0, 0, 0, 22733], dated_worst: [4000, 3274, 50, 12700, 1500, 1310, 1250, 225, 0, 0, 24309], today_precise: [4000, 3093, 101, 12600, 1500, 1295, 0, 0, 0, 0, 22589], today_worst: [4000, 3093, 101, 12700, 1500, 1295, 1250, 225, 0, 0, 24164], headline: 24309, net: 725691 },
    Vector { id: "CN09", day: (2021, 1, 1), orders: &[(Side::Sell, false, &[(500, 100000)]), (Side::Buy, false, &[(500, 101200)])], dated_precise: [4000, 3471, 50, 12600, 1500, 1345, 0, 0, 0, 0, 22966], dated_worst: [4000, 3471, 50, 12600, 1500, 1345, 1250, 225, 0, 0, 24441], today_precise: [4000, 3088, 101, 12600, 1500, 1294, 0, 0, 0, 0, 22583], today_worst: [4000, 3088, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24058], headline: 24441, net: -624441 },
    Vector { id: "CN10", day: (2021, 3, 31), orders: &[(Side::Buy, false, &[(1000, 49995)]), (Side::Sell, false, &[(1000, 50500)])], dated_precise: [4000, 3467, 50, 12600, 1500, 1344, 0, 0, 0, 0, 22961], dated_worst: [4000, 3467, 50, 12600, 1500, 1344, 1250, 225, 0, 0, 24436], today_precise: [4000, 3085, 100, 12600, 1500, 1293, 0, 0, 0, 0, 22578], today_worst: [4000, 3085, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24055], headline: 24436, net: 480564 },
    Vector { id: "CN11", day: (2021, 4, 1), orders: &[(Side::Buy, false, &[(1000, 49995)]), (Side::Sell, false, &[(1000, 50500)])], dated_precise: [4000, 3467, 100, 12600, 1500, 1344, 0, 0, 0, 0, 23011], dated_worst: [4000, 3467, 101, 12600, 1500, 1344, 1250, 225, 0, 0, 24487], today_precise: [4000, 3085, 100, 12600, 1500, 1293, 0, 0, 0, 0, 22578], today_worst: [4000, 3085, 101, 12600, 1500, 1294, 1250, 225, 0, 0, 24055], headline: 24487, net: 480513 },
    Vector { id: "CN12", day: (2022, 7, 15), orders: &[(Side::Buy, false, &[(200, 250000)]), (Side::Sell, false, &[(200, 253750)])], dated_precise: [4000, 3476, 101, 12600, 1500, 1346, 0, 0, 0, 0, 23023], dated_worst: [4000, 3476, 101, 12700, 1500, 1346, 1250, 225, 0, 0, 24598], today_precise: [4000, 3093, 101, 12600, 1500, 1295, 0, 0, 0, 0, 22589], today_worst: [4000, 3093, 101, 12700, 1500, 1295, 1250, 225, 0, 0, 24164], headline: 24598, net: 725402 },
    Vector { id: "CN13", day: (2022, 7, 18), orders: &[(Side::Buy, false, &[(200, 250000)]), (Side::Sell, false, &[(200, 253750)])], dated_precise: [4000, 3476, 101, 12600, 1500, 1364, 0, 0, 0, 0, 23041], dated_worst: [4000, 3476, 101, 12700, 1500, 1364, 1250, 225, 0, 0, 24616], today_precise: [4000, 3093, 101, 12600, 1500, 1295, 0, 0, 0, 0, 22589], today_worst: [4000, 3093, 101, 12700, 1500, 1295, 1250, 225, 0, 0, 24164], headline: 24616, net: 725384 },
    Vector { id: "CN14", day: (2023, 3, 31), orders: &[(Side::Sell, false, &[(6423, 7785)]), (Side::Buy, false, &[(6423, 7630)])], dated_precise: [4000, 3416, 99, 12400, 1500, 1353, 0, 0, 0, 0, 22768], dated_worst: [4000, 3416, 99, 12500, 1500, 1353, 1250, 225, 0, 0, 24343], today_precise: [4000, 3040, 99, 12400, 1500, 1285, 0, 0, 0, 0, 22324], today_worst: [4000, 3040, 99, 12500, 1500, 1286, 1250, 225, 0, 0, 23900], headline: 24343, net: 971222 },
    Vector { id: "CN15", day: (2023, 4, 3), orders: &[(Side::Sell, false, &[(6423, 7785)]), (Side::Buy, false, &[(6423, 7630)])], dated_precise: [4000, 3317, 99, 12400, 1500, 1335, 0, 0, 0, 0, 22651], dated_worst: [4000, 3317, 99, 12500, 1500, 1335, 1250, 225, 0, 0, 24226], today_precise: [4000, 3040, 99, 12400, 1500, 1285, 0, 0, 0, 0, 22324], today_worst: [4000, 3040, 99, 12500, 1500, 1286, 1250, 225, 0, 0, 23900], headline: 24226, net: 971339 },
    Vector { id: "CN16", day: (2024, 3, 28), orders: &[(Side::Buy, false, &[(312, 160100)]), (Side::Sell, false, &[(312, 164000)])], dated_precise: [4000, 3387, 101, 12600, 1500, 1348, 0, 0, 0, 0, 22936], dated_worst: [4000, 3387, 101, 12800, 1500, 1348, 1250, 225, 0, 0, 24611], today_precise: [4000, 3104, 101, 12600, 1500, 1297, 0, 0, 0, 0, 22602], today_worst: [4000, 3105, 101, 12800, 1500, 1298, 1250, 225, 0, 0, 24279], headline: 24611, net: 1192189 },
    Vector { id: "CN17", day: (2024, 4, 1), orders: &[(Side::Buy, false, &[(312, 160100)]), (Side::Sell, false, &[(312, 164000)])], dated_precise: [4000, 3357, 101, 12600, 1500, 1342, 0, 0, 0, 0, 22900], dated_worst: [4000, 3357, 101, 12800, 1500, 1342, 1250, 225, 0, 0, 24575], today_precise: [4000, 3104, 101, 12600, 1500, 1297, 0, 0, 0, 0, 22602], today_worst: [4000, 3105, 101, 12800, 1500, 1298, 1250, 225, 0, 0, 24279], headline: 24575, net: 1192225 },
    Vector { id: "CN18", day: (2024, 9, 30), orders: &[(Side::Buy, false, &[(51, 987500)]), (Side::Sell, false, &[(51, 980000)])], dated_precise: [4000, 3331, 100, 12500, 1500, 1338, 0, 0, 0, 0, 22769], dated_worst: [4000, 3331, 100, 12500, 1500, 1338, 1250, 225, 0, 0, 24244], today_precise: [4000, 3081, 100, 12500, 1500, 1293, 0, 0, 0, 0, 22474], today_worst: [4000, 3081, 100, 12500, 1500, 1293, 1250, 225, 0, 0, 23949], headline: 24244, net: -406744 },
    Vector { id: "CN19", day: (2024, 10, 1), orders: &[(Side::Buy, false, &[(51, 987500)]), (Side::Sell, false, &[(51, 980000)])], dated_precise: [4000, 3081, 100, 12500, 1500, 1293, 0, 0, 0, 0, 22474], dated_worst: [4000, 3081, 100, 12500, 1500, 1293, 1250, 225, 0, 0, 23949], today_precise: [4000, 3081, 100, 12500, 1500, 1293, 0, 0, 0, 0, 22474], today_worst: [4000, 3081, 100, 12500, 1500, 1293, 1250, 225, 0, 0, 23949], headline: 23949, net: -406449 },
    Vector { id: "CN20", day: (2026, 2, 27), orders: &[(Side::Sell, false, &[(40, 500000), (35, 499950), (25, 499900)]), (Side::Buy, false, &[(60, 495000), (40, 495150)])], dated_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343], dated_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998], today_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343], today_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998], headline: 30998, net: 458752 },
    Vector { id: "CN21", day: (2026, 3, 2), orders: &[(Side::Sell, false, &[(40, 500000), (35, 499950), (25, 499900)]), (Side::Buy, false, &[(60, 495000), (40, 495150)])], dated_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343], dated_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998], today_precise: [4000, 3055, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22343], today_worst: [10000, 3055, 100, 12500, 1500, 2368, 1250, 225, 0, 0, 30998], headline: 30998, net: 458752 },
    Vector { id: "CN22", day: (2026, 10, 30), orders: &[(Side::Buy, false, &[(40, 160000), (40, 160005), (40, 160010), (40, 160010), (40, 160020), (40, 160025), (40, 160030), (33, 160040)]), (Side::Sell, false, &[(120, 161700), (80, 161650)]), (Side::Sell, true, &[(113, 161105)])], dated_precise: [6000, 3089, 101, 12600, 1500, 1654, 0, 0, 2000, 360, 27304], dated_worst: [21024, 3090, 101, 12700, 1500, 4359, 1250, 225, 2000, 360, 46609], today_precise: [6000, 3089, 101, 12600, 1500, 1654, 0, 0, 2000, 360, 27304], today_worst: [21024, 3090, 101, 12700, 1500, 4359, 1250, 225, 2000, 360, 46609], headline: 46609, net: 408936 },
    Vector { id: "CN23", day: (2026, 10, 30), orders: &[(Side::Buy, false, &[(40, 160000), (40, 160005), (40, 160010), (40, 160010), (40, 160020), (40, 160025), (40, 160030), (32, 160040)]), (Side::Sell, false, &[(120, 161700), (80, 161650)]), (Side::Sell, true, &[(112, 161105)])], dated_precise: [6000, 3079, 100, 12500, 1500, 1652, 0, 0, 2000, 360, 27191], dated_worst: [20976, 3080, 100, 12600, 1500, 4348, 1250, 225, 2000, 360, 46439], today_precise: [6000, 3079, 100, 12500, 1500, 1652, 0, 0, 2000, 360, 27191], today_worst: [20976, 3080, 100, 12600, 1500, 4348, 1250, 225, 2000, 360, 46439], headline: 46439, net: 408041 },
    Vector { id: "CN24", day: (2026, 10, 30), orders: &[(Side::Buy, false, &[(313, 16000), (313, 16000), (313, 16005), (312, 16005), (312, 16010), (312, 16010), (312, 16015), (312, 16015), (313, 16020), (313, 16020)]), (Side::Sell, false, &[(3125, 15840)])], dated_precise: [4000, 3056, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22344], dated_worst: [17009, 3056, 100, 12400, 1500, 3630, 1250, 225, 0, 0, 39170], today_precise: [4000, 3056, 100, 12400, 1500, 1288, 0, 0, 0, 0, 22344], today_worst: [17009, 3056, 100, 12400, 1500, 3630, 1250, 225, 0, 0, 39170], headline: 39170, net: -570415 },
];

fn orders_of(v: &Vector) -> Vec<Order> {
    v.orders
        .iter()
        .map(|(side, by_broker, parts)| Order {
            buy: matches!(side, Side::Buy),
            by_broker: *by_broker,
            parts: parts.iter().map(|(qty, price)| Part { qty: *qty, price: *price }).collect(),
        })
        .collect()
}

fn with_total(lines: &Lines) -> [i128; 11] {
    let a = lines.as_array();
    [a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7], a[8], a[9], lines.total()]
}

fn gross_of(orders: &[Order]) -> i128 {
    orders.iter().map(|o| if o.buy { -o.value() } else { o.value() }).sum()
}

#[test]
fn every_contract_note_vector_matches_line_by_line() {
    let mut misses = Vec::new();
    for v in &VECTORS {
        let day = Day::from_ymd(v.day.0, v.day.1, v.day.2).unwrap();
        let orders = orders_of(v);
        let p = price_trade(&orders, day).unwrap();
        let checks = [
            ("dated precise", with_total(&p.dated_precise), v.dated_precise),
            ("dated worst", with_total(&p.dated_worst), v.dated_worst),
            ("today precise", with_total(&p.today_precise), v.today_precise),
            ("today worst", with_total(&p.today_worst), v.today_worst),
        ];
        for (what, got, want) in checks {
            if got != want {
                misses.push(format!("{} {what}: got {got:?} want {want:?}", v.id));
            }
        }
        if p.headline != v.headline {
            misses.push(format!("{} headline: got {} want {}", v.id, p.headline, v.headline));
        }
        if gross_of(&orders) - p.headline != v.net {
            misses.push(format!("{} net: got {} want {}", v.id, gross_of(&orders) - p.headline, v.net));
        }
    }
    assert!(misses.is_empty(), "{} misses:\n{}", misses.len(), misses.join("\n"));
}
