//! R4-28 (CON-12, FUT-C1): futures day candles through a token, stored and
//! never swept.
//!
//! Every row and price here is invented: the underlying `ZQIDX`, tokens in the
//! eight millions and prices near ₹100, which no vendor publishes for a real
//! future. The header is Kite's published column list, a schema and not a row.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use super::*;
use crate::chain::{Discovery, Refusal};
use crate::master_snapshots::{self, Book, Kind as Master};
use brutex_core::instrument::{Contract, Expiry, InstrumentKey, Kind};
use brutex_core::vendor::Vendor;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const URL: &str = "https://api.kite.trade/instruments";

fn scratch(name: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir().join(format!(
        "brutex-r428-fut-{name}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

fn window(from: Day, to: Day) -> Window {
    Window::new(from, to).expect("an ordered window")
}

fn route() -> ContinuousSpec {
    crate::vendor::ZERODHA_CONTINUOUS
}

fn spec() -> HttpSpec {
    let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Zerodha.descriptor().transport
    else {
        panic!("Zerodha is an HTTP feed");
    };
    spec
}

fn fut(token: u64, symbol: &str, expiry: &str) -> String {
    format!(
        "{token},{},{symbol},\"ZQIDX\",0,{expiry},0,0.05,75,FUT,NFO-FUT,NFO",
        token / 256
    )
}

fn snapshot_on(root: &Path, on: Day, rows: &[String]) {
    let mut body = String::from(crate::vendor::KITE_MASTER_HEADER);
    for one in rows {
        body.push('\n');
        body.push_str(one);
    }
    body.push('\n');
    master_snapshots::snapshot(root, Master::Kite, on, URL, body.as_bytes()).expect("a snapshot");
}

/// The token a URL asks for, and whether it asks for the continuous series.
fn token_of(url: &str) -> (u64, bool) {
    let token = url
        .split("/historical/")
        .nth(1)
        .and_then(|rest| rest.split('/').next())
        .and_then(|token| token.parse().ok())
        .expect("a token in the path");
    (token, url.contains("continuous=1"))
}

/// One invented day candle: the price tells the series apart, the open
/// interest tells the token and the day apart.
fn candle(on: Day, price: u32, oi: u64) -> String {
    format!(
        "[\"{on}T00:00:00+0530\",{price}.5,{}.0,{}.25,{price}.75,1200,{oi}]",
        price + 1,
        price - 1
    )
}

fn answer(candles: &[String]) -> String {
    format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        candles.join(",")
    )
}

/// A transport standing in for the vendor: it records every URL and answers
/// from `reply`.
struct Vendorish {
    seen: RefCell<Vec<String>>,
    reply: fn(&str) -> Result<String, Refusal>,
}

impl Discovery for Vendorish {
    async fn get(&self, url: &str) -> Result<String, Refusal> {
        self.seen.borrow_mut().push(url.to_owned());
        (self.reply)(url)
    }
}

fn block_on<F: core::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(future)
}

/// The five weekdays 2026-01-05..09.
fn january() -> Vec<Day> {
    (5..=9).map(|d| day(2026, 1, d)).collect()
}

fn january_reply(url: &str) -> Result<String, Refusal> {
    let (token, continuous) = token_of(url);
    let (price, base) = match (token, continuous) {
        (8_000_001, false) => (100, 1_000),
        (8_000_002, false) => (110, 2_000),
        (8_000_001, true) => (120, 3_000),
        _ => {
            return Err(Refusal::answered(
                400,
                "InputException: unknown token".to_owned(),
            ));
        }
    };
    let candles: Vec<String> = january()
        .into_iter()
        .map(|on| candle(on, price, base + u64::from(on.day())))
        .collect();
    Ok(answer(&candles))
}

/// February, after the token was reused for April.
fn february_reply(url: &str) -> Result<String, Refusal> {
    let (token, continuous) = token_of(url);
    let base = match (token, continuous) {
        (8_000_001, false) => 4_000,
        (8_000_002, false) => 5_000,
        (8_000_002, true) => 6_000,
        _ => return Err(Refusal::answered(400, "InputException".to_owned())),
    };
    let candles: Vec<String> = (2..=6)
        .map(|d| candle(day(2026, 2, d), 130, base + u64::from(d)))
        .collect();
    Ok(answer(&candles))
}

/// A vendor that answers every request with no candle.
#[allow(clippy::unnecessary_wraps, reason = "the transport's own signature")]
fn empty_reply(_url: &str) -> Result<String, Refusal> {
    Ok(answer(&[]))
}

/// The continuous series across the 2026-01-26 holiday and the roll after it.
#[allow(clippy::unnecessary_wraps, reason = "the transport's own signature")]
fn rolling_reply(url: &str) -> Result<String, Refusal> {
    assert!(url.contains("continuous=1"), "{url}");
    let days = [21, 22, 23, 27, 28, 29, 30];
    let candles: Vec<String> = days
        .iter()
        .map(|d| {
            candle(
                day(2026, 1, *d),
                if *d < 27 { 100 } else { 140 },
                7_000 + u64::from(*d),
            )
        })
        .collect();
    Ok(answer(&candles))
}

fn bars_of(
    root: &Path,
    symbol: &str,
    contract: Option<Contract>,
    month: YearMonth,
) -> Vec<store::format::Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "FNO",
        symbol,
        contract,
        timeframe: Timeframe::DAY_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a store path");
    #[expect(clippy::cast_possible_truncation, reason = "the store's own fold")]
    let symbol_id = brutex_core::universe::fnv1a(symbol) as u32;
    let file =
        store::file::BarFile::open_existing(root, path, symbol_id).expect("the month is held");
    file.read_range(0, file.records()).expect("every bar")
}

fn contract(y: u16, m: u8, d: u8) -> Contract {
    Contract::of(Kind::Future {
        expiry: Expiry::new(y, m, d).expect("an expiry"),
    })
    .expect("a future renders")
}

#[test]
fn the_futures_url_spells_the_vendors_own_words() {
    let jan = window(day(2026, 1, 5), day(2026, 1, 9));
    assert_eq!(
        crate::fno::day_candles_url(&spec(), 8_000_001, jan, false).unwrap(),
        "https://api.kite.trade/instruments/historical/8000001/day?from=2026-01-05%2000%3A00%3A00&to=2026-01-09%2023%3A59%3A59&oi=1"
    );
    assert_eq!(
        crate::fno::day_candles_url(&spec(), 8_000_001, jan, true).unwrap(),
        "https://api.kite.trade/instruments/historical/8000001/day?from=2026-01-05%2000%3A00%3A00&to=2026-01-09%2023%3A59%3A59&oi=1&continuous=1"
    );
    // A feed with no token route builds nothing.
    let crate::vendor::Transport::Http(groww) = crate::vendor::Feed::Groww.descriptor().transport
    else {
        panic!("Groww is an HTTP feed");
    };
    assert!(
        crate::fno::day_candles_url(&groww, 1, jan, false)
            .unwrap_err()
            .contains("no futures token route")
    );
}

#[test]
fn a_futures_day_pull_stores_contracts_with_expiry_and_oi() {
    let (masters, store_root) = (scratch("masters"), scratch("store"));
    snapshot_on(
        &masters,
        day(2026, 1, 2),
        &[
            fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27"),
            fut(8_000_002, "ZQIDX26FEBFUT", "2026-02-24"),
            // AN OPTION AND A SHARE: never futures, never planned.
            "8000003,31250,ZQIDX26JAN100CE,\"ZQIDX\",0,2026-01-27,100,0.05,75,CE,NFO-OPT,NFO"
                .to_owned(),
            "8000004,31250,ZQIDX,\"ZQIDX\",0,,0,0.05,1,EQ,NSE,NSE".to_owned(),
        ],
    );
    let book = Book::open(&masters).unwrap();
    let plan = futures_plan(
        &book,
        route(),
        "ZQIDX",
        window(day(2026, 1, 1), day(2026, 1, 9)),
        day(2026, 1, 12),
        day(2026, 1, 9),
    )
    .unwrap();
    assert!(plan.refused.is_empty(), "{:?}", plan.refused);
    assert_eq!(
        plan.asks,
        vec![
            FuturesAsk {
                token: 8_000_001,
                series: FuturesSeries::Contract {
                    contract: contract(2026, 1, 27),
                    expiry: day(2026, 1, 27),
                },
                window: window(day(2026, 1, 2), day(2026, 1, 9)),
            },
            FuturesAsk {
                token: 8_000_002,
                series: FuturesSeries::Contract {
                    contract: contract(2026, 2, 24),
                    expiry: day(2026, 2, 24),
                },
                window: window(day(2026, 1, 2), day(2026, 1, 9)),
            },
            FuturesAsk {
                token: 8_000_001,
                series: FuturesSeries::Continuous,
                window: window(day(2026, 1, 1), day(2026, 1, 9)),
            },
        ]
    );
    let vendor = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply: january_reply,
    };
    let landed = block_on(pull_futures_day(
        &vendor,
        &spec(),
        &plan,
        &store_root,
        Vendor::Zerodha,
        "test://futures",
    ));
    assert!(landed.failed.is_empty(), "{:?}", landed.failed);
    assert_eq!(
        (landed.requests, landed.rows_read, landed.bars_stored),
        (3, 15, 15)
    );
    let month = YearMonth::new(2026, 1).unwrap();
    // EACH CONTRACT UNDER ITS OWN EXPIRY, WITH ITS OWN OPEN INTEREST.
    let jan = bars_of(&store_root, "ZQIDX", Some(contract(2026, 1, 27)), month);
    let feb = bars_of(&store_root, "ZQIDX", Some(contract(2026, 2, 24)), month);
    let c1 = bars_of(&store_root, "ZQIDX-FUT-C1", None, month);
    assert_eq!(
        jan.iter().map(|b| b.open_interest).collect::<Vec<_>>(),
        [1_005, 1_006, 1_007, 1_008, 1_009]
    );
    assert_eq!(
        feb.iter().map(|b| b.open_interest).collect::<Vec<_>>(),
        [2_005, 2_006, 2_007, 2_008, 2_009]
    );
    assert_eq!(
        c1.iter().map(|b| b.open_interest).collect::<Vec<_>>(),
        [3_005, 3_006, 3_007, 3_008, 3_009]
    );
    assert_eq!(jan[0].open, 10_050, "₹100.50 in paisa");
    assert_eq!(c1[0].close, 12_075);
    let seen = vendor.seen.borrow();
    assert_eq!(
        seen.iter()
            .filter(|url| url.contains("continuous=1"))
            .count(),
        1
    );
    assert!(seen.iter().all(|url| url.contains("&oi=1")), "{seen:?}");
}

#[test]
fn two_contracts_reusing_a_token_on_different_dates_stay_separate() {
    let (masters, store_root) = (scratch("reuse-masters"), scratch("reuse-store"));
    snapshot_on(
        &masters,
        day(2026, 1, 2),
        &[fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27")],
    );
    // PULL 1, while the token names JANUARY.
    let first = futures_plan(
        &Book::open(&masters).unwrap(),
        route(),
        "ZQIDX",
        window(day(2026, 1, 1), day(2026, 1, 9)),
        day(2026, 1, 12),
        day(2026, 1, 9),
    )
    .unwrap();
    let vendor = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply: january_reply,
    };
    block_on(pull_futures_day(
        &vendor,
        &spec(),
        &first,
        &store_root,
        Vendor::Zerodha,
        "test://futures",
    ));
    let month = YearMonth::new(2026, 1).unwrap();
    let january_bars = bars_of(&store_root, "ZQIDX", Some(contract(2026, 1, 27)), month);
    assert_eq!(january_bars.len(), 5);

    // THE EXCHANGE REUSES THE TOKEN FOR APRIL; A SECOND SNAPSHOT SAYS SO.
    snapshot_on(
        &masters,
        day(2026, 2, 2),
        &[
            fut(8_000_001, "ZQIDX26APRFUT", "2026-04-28"),
            fut(8_000_002, "ZQIDX26FEBFUT", "2026-02-24"),
        ],
    );
    let book = Book::open(&masters).unwrap();
    assert!(matches!(
        book.token_on(day(2026, 1, 20), "NFO", 8_000_001),
        master_snapshots::TokenOn::Ambiguous { .. }
    ));
    let second = futures_plan(
        &book,
        route(),
        "ZQIDX",
        window(day(2026, 1, 20), day(2026, 2, 6)),
        day(2026, 2, 10),
        day(2026, 2, 6),
    )
    .unwrap();
    // JANUARY IS NOT ASKED THROUGH A TOKEN THAT NOW NAMES APRIL.
    assert_eq!(second.refused.len(), 1, "{:?}", second.refused);
    assert!(
        second.refused[0].starts_with("token_reused:"),
        "{:?}",
        second.refused
    );
    assert!(
        second.refused[0].contains("ZQIDX26APRFUT"),
        "{:?}",
        second.refused
    );
    let series: Vec<FuturesSeries> = second.asks.iter().map(|ask| ask.series).collect();
    assert_eq!(
        series,
        [
            FuturesSeries::Contract {
                contract: contract(2026, 2, 24),
                expiry: day(2026, 2, 24),
            },
            FuturesSeries::Contract {
                contract: contract(2026, 4, 28),
                expiry: day(2026, 4, 28),
            },
            FuturesSeries::Continuous,
        ]
    );
    assert_eq!(second.asks[1].token, 8_000_001);
    assert_eq!(
        second.asks[1].window,
        window(day(2026, 2, 2), day(2026, 2, 6))
    );
    assert_eq!(
        second.asks[2].token, 8_000_002,
        "the near month on the day asked"
    );
    let vendor = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply: february_reply,
    };
    let landed = block_on(pull_futures_day(
        &vendor,
        &spec(),
        &second,
        &store_root,
        Vendor::Zerodha,
        "test://futures",
    ));
    assert!(landed.failed.is_empty(), "{:?}", landed.failed);
    // JANUARY IS EXACTLY WHAT PULL 1 FILED; APRIL HOLDS ONLY ITS OWN DAYS.
    assert_eq!(
        bars_of(&store_root, "ZQIDX", Some(contract(2026, 1, 27)), month),
        january_bars
    );
    let april = bars_of(
        &store_root,
        "ZQIDX",
        Some(contract(2026, 4, 28)),
        YearMonth::new(2026, 2).unwrap(),
    );
    assert_eq!(
        april.iter().map(|b| b.open_interest).collect::<Vec<_>>(),
        [4_002, 4_003, 4_004, 4_005, 4_006]
    );
}

#[test]
fn a_future_is_never_sweepable() {
    let expiry = Expiry::new(2026, 1, 27).unwrap();
    for underlying in ["NIFTY", "BANKNIFTY", "RELIANCE", "ZQIDX"] {
        let future = InstrumentKey {
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new(underlying).unwrap(),
            kind: Kind::Future { expiry },
        };
        assert!(!future.is_sweepable(), "{underlying} future");
        assert!(future.require_sweepable().is_err(), "{underlying} future");
        // THE CONTINUOUS SERIES IS AN FNO KEY, AND NO FNO KEY IS SWEPT.
        let continuous = InstrumentKey {
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: Segment::Fno,
            underlying: continuous_symbol(underlying).unwrap(),
            kind: Kind::Index,
        };
        assert!(
            !continuous.is_sweepable(),
            "{underlying}{CONTINUOUS_SUFFIX}"
        );
        assert!(!brutex_core::universe::FNO_INDEX.contains(continuous.underlying.as_str()));
    }
    // NOT VACUOUS: the spot index the futures are written on is swept.
    let spot = InstrumentKey {
        exchange: brutex_core::instrument::Exchange::Nse,
        segment: Segment::Index,
        underlying: Symbol::new("NIFTY").unwrap(),
        kind: Kind::Index,
    };
    assert!(spot.is_sweepable());
}

#[test]
fn a_futures_plan_before_any_snapshot_is_refused_by_name() {
    let book = Book::open(&scratch("none")).unwrap();
    let why = futures_plan(
        &book,
        route(),
        "ZQIDX",
        window(day(2026, 1, 1), day(2026, 1, 9)),
        day(2026, 1, 12),
        day(2026, 1, 9),
    )
    .unwrap_err();
    assert!(why.starts_with("no_snapshot:"), "{why}");
}

#[test]
fn an_underlying_with_no_live_future_names_the_continuous_series_it_cannot_ask() {
    let masters = scratch("no-live");
    snapshot_on(
        &masters,
        day(2026, 1, 2),
        &[fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27")],
    );
    let book = Book::open(&masters).unwrap();
    // An underlying the snapshot does not list, and garbage words.
    for underlying in ["ZQNONE", "", "zqidx"] {
        let plan = futures_plan(
            &book,
            route(),
            underlying,
            window(day(2026, 1, 1), day(2026, 1, 9)),
            day(2026, 1, 12),
            day(2026, 1, 9),
        )
        .unwrap();
        assert!(plan.asks.is_empty(), "{underlying}: {:?}", plan.asks);
        assert!(
            plan.refused
                .iter()
                .any(|why| why.starts_with("no_live_future:")),
            "{underlying}"
        );
    }
    // AFTER THE ONLY FUTURE EXPIRED: its own days are still owed, the
    // continuous series has no live token.
    let late = futures_plan(
        &book,
        route(),
        "ZQIDX",
        window(day(2026, 1, 20), day(2026, 1, 30)),
        day(2026, 2, 2),
        day(2026, 1, 30),
    )
    .unwrap();
    assert_eq!(late.asks.len(), 1);
    assert_eq!(
        late.asks[0].window,
        window(day(2026, 1, 20), day(2026, 1, 27)),
        "never past its expiry"
    );
    assert!(
        late.refused
            .iter()
            .any(|why| why.starts_with("no_live_future:"))
    );
}

#[test]
fn an_expired_token_the_vendor_no_longer_serves_ends_its_history_by_name() {
    let (masters, store_root) = (scratch("ends-masters"), scratch("ends-store"));
    snapshot_on(
        &masters,
        day(2026, 1, 2),
        &[
            fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27"),
            fut(8_000_009, "ZQIDX26MARFUT", "2026-03-31"),
        ],
    );
    let plan = futures_plan(
        &Book::open(&masters).unwrap(),
        route(),
        "ZQIDX",
        window(day(2026, 1, 1), day(2026, 1, 9)),
        day(2026, 1, 12),
        day(2026, 1, 9),
    )
    .unwrap();
    let vendor = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply: january_reply,
    };
    let landed = block_on(pull_futures_day(
        &vendor,
        &spec(),
        &plan,
        &store_root,
        Vendor::Zerodha,
        "test://futures",
    ));
    // THE MARCH TOKEN IS REFUSED BY THE VENDOR; THE OTHER TWO SERIES LAND.
    assert_eq!(landed.failed.len(), 1, "{:?}", landed.failed);
    assert!(
        landed.failed[0].contains("history_ends"),
        "{:?}",
        landed.failed
    );
    assert!(
        landed.failed[0].contains("2026-03-31-FUT"),
        "{:?}",
        landed.failed
    );
    assert_eq!(landed.bars_stored, 10);
    let quiet = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply: empty_reply,
    };
    let none = block_on(pull_futures_day(
        &quiet,
        &spec(),
        &plan,
        &scratch("ends-empty"),
        Vendor::Zerodha,
        "test://futures",
    ));
    // AN EMPTY ANSWER ENDS A CONTRACT'S HISTORY BY NAME; THE CONTINUOUS
    // SERIES SIMPLY HOLDS NOTHING FOR THOSE DAYS.
    assert_eq!(
        none.failed
            .iter()
            .filter(|why| why.contains("answered no candle"))
            .count(),
        2
    );
    assert_eq!(none.bars_stored, 0);
}

#[test]
fn a_continuous_series_across_a_holiday_roll_is_filed_as_answered_and_never_split() {
    let (masters, store_root) = (scratch("roll-masters"), scratch("roll-store"));
    snapshot_on(
        &masters,
        day(2026, 1, 2),
        &[fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27")],
    );
    let book = Book::open(&masters).unwrap();
    let mut plan = futures_plan(
        &book,
        route(),
        "ZQIDX",
        window(day(2026, 1, 21), day(2026, 1, 30)),
        day(2026, 1, 12),
        day(2026, 1, 30),
    )
    .unwrap();
    plan.asks
        .retain(|ask| ask.series == FuturesSeries::Continuous);
    let vendor = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply: rolling_reply,
    };
    let landed = block_on(pull_futures_day(
        &vendor,
        &spec(),
        &plan,
        &store_root,
        Vendor::Zerodha,
        "test://futures",
    ));
    assert!(landed.failed.is_empty(), "{:?}", landed.failed);
    let c1 = bars_of(
        &store_root,
        "ZQIDX-FUT-C1",
        None,
        YearMonth::new(2026, 1).unwrap(),
    );
    assert_eq!(
        c1.iter().map(|b| b.open_interest).collect::<Vec<_>>(),
        [7_021, 7_022, 7_023, 7_027, 7_028, 7_029, 7_030]
    );
    // NOTHING WAS FILED UNDER A CONTRACT FROM THE CONTINUOUS ANSWER.
    let contract_dir = store_root.join("bars/zerodha/NSE/FNO/ZQIDX/2026-01-27-FUT");
    assert!(!contract_dir.exists(), "{}", contract_dir.display());
}
