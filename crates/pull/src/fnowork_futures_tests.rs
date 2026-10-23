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

/// The plan for 2026-01-01..09 asked on 2026-01-12 from one snapshot of
/// 2026-01-02 listing the January and February futures: each contract from
/// the snapshot's day, the continuous series through January's token.
fn january_asks() -> Vec<FuturesAsk> {
    let contract_ask = |token, y, m, d| FuturesAsk {
        token,
        series: FuturesSeries::Contract {
            contract: contract(y, m, d),
            expiry: day(y, m, d),
        },
        window: window(day(2026, 1, 2), day(2026, 1, 9)),
    };
    vec![
        contract_ask(8_000_001, 2026, 1, 27),
        contract_ask(8_000_002, 2026, 2, 24),
        FuturesAsk {
            token: 8_000_001,
            series: FuturesSeries::Continuous,
            window: window(day(2026, 1, 1), day(2026, 1, 9)),
        },
    ]
}

/// Plans `ZQIDX` over `asked` from the snapshots in `masters`, its last day
/// settled, and pulls it into `store_root` from a vendor answering `reply`.
fn plan_and_pull(
    masters: &Path,
    store_root: &Path,
    asked: Window,
    today: Day,
    reply: fn(&str) -> Result<String, Refusal>,
) -> (FuturesPlan, FuturesLanded) {
    let book = Book::open(masters).unwrap();
    let plan = futures_plan(&book, route(), "ZQIDX", asked, today, asked.to()).unwrap();
    let vendor = Vendorish {
        seen: RefCell::new(Vec::new()),
        reply,
    };
    let landed = block_on(pull_futures_day(
        &vendor,
        &spec(),
        &plan,
        store_root,
        Vendor::Zerodha,
        "test://futures",
    ));
    (plan, landed)
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
    assert_eq!(plan.asks, january_asks());
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
    assert_counted(
        &store_root,
        month,
        &[
            ("ZQIDX", Some(contract(2026, 1, 27))),
            ("ZQIDX", Some(contract(2026, 2, 24))),
            ("ZQIDX-FUT-C1", None),
        ],
    );
}

/// THE CENSUS COUNTS EACH CONTRACT BY UNDERLYING AND EXPIRY, AND THE
/// CONTINUOUS SERIES UNDER ITS OWN NAME, so the coverage board and `/db` read
/// them as any other held series.
fn assert_counted(store_root: &Path, month: YearMonth, series: &[(&str, Option<Contract>)]) {
    let census = crate::ingest::census_store::read_vendor(store_root, Vendor::Zerodha).unwrap();
    for &(symbol, contract) in series {
        let key = EntryKey {
            contract,
            exchange: brutex_core::instrument::Exchange::Nse,
            segment: Segment::Fno,
            symbol: Symbol::new(symbol).unwrap(),
            timeframe: Timeframe::DAY_1,
            month,
        };
        assert!(
            census.iter().any(|read| read.census.held(&key).is_some()),
            "{key:?} is counted"
        );
    }
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
    let jan = window(day(2026, 1, 1), day(2026, 1, 9));
    let (_, first) = plan_and_pull(&masters, &store_root, jan, day(2026, 1, 12), january_reply);
    assert!(first.failed.is_empty(), "{:?}", first.failed);
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
    let asked = window(day(2026, 1, 20), day(2026, 2, 6));
    let (second, landed) = plan_and_pull(
        &masters,
        &store_root,
        asked,
        day(2026, 2, 10),
        february_reply,
    );
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
        landed.failed[0].contains("vendor_refused"),
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

/// R4-28 repair 1 (D-6894): a plan reads the first-listed index, never the
/// older snapshots. Two older snapshots rotted on disk after they were written
/// leave the plan untouched, and the contracts they first listed are still
/// asked from their own days. Before the index, every plan opened, unpacked
/// and parsed every snapshot on or before the window's end, and these two
/// would have been refused and their contracts never asked.
#[test]
fn a_futures_plan_opens_no_snapshot_the_index_already_holds() {
    let masters = scratch("index-only");
    let jan = fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27");
    let feb = fut(8_000_002, "ZQIDX26FEBFUT", "2026-02-24");
    snapshot_on(&masters, day(2026, 1, 2), std::slice::from_ref(&jan));
    snapshot_on(&masters, day(2026, 1, 5), &[jan.clone(), feb.clone()]);
    snapshot_on(&masters, day(2026, 1, 12), &[jan, feb]);
    // ROT ON DISK: neither older body matches its receipt any more.
    for on in [day(2026, 1, 2), day(2026, 1, 5)] {
        let stamp = master_snapshots::Stamp {
            day: on,
            ordinal: 1,
        };
        std::fs::write(
            masters.join(master_snapshots::file_name(Master::Kite, stamp)),
            b"rotted",
        )
        .unwrap();
    }
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
    let spans: Vec<(u64, Window)> = plan
        .asks
        .iter()
        .map(|ask| (ask.token, ask.window))
        .collect();
    assert_eq!(
        spans,
        vec![
            (8_000_001, window(day(2026, 1, 2), day(2026, 1, 9))),
            (8_000_002, window(day(2026, 1, 5), day(2026, 1, 9))),
            (8_000_001, window(day(2026, 1, 1), day(2026, 1, 9))),
        ],
        "each contract from the day its first snapshot listed it, the continuous series last"
    );
}

/// R4-28 repair 3: identity, not trading. The newest snapshot may still list
/// a contract past its expiry, which `token_on` answers `token_expired` on
/// that day (PREC-10), and the plan still asks that contract's history. A
/// Kite write cut before its receipt is named by the plan, never read.
#[test]
fn a_plan_asks_a_contract_still_listed_past_its_expiry_and_names_a_cut_snapshot() {
    let masters = scratch("late-flush");
    let jan = fut(8_000_011, "ZQIDX26JANFUT", "2026-01-27");
    let feb = fut(8_000_012, "ZQIDX26FEBFUT", "2026-02-24");
    snapshot_on(&masters, day(2026, 1, 5), &[jan.clone(), feb.clone()]);
    // A LATE FLUSH: the newest snapshot, after JAN's expiry, still lists it.
    snapshot_on(&masters, day(2026, 1, 30), &[jan, feb]);
    let cut = master_snapshots::file_name(
        Master::Kite,
        master_snapshots::Stamp {
            day: day(2026, 1, 31),
            ordinal: 1,
        },
    );
    std::fs::write(masters.join(&cut), b"a body cut before its receipt").unwrap();
    let book = Book::open(&masters).unwrap();
    assert_eq!(
        book.token_on(day(2026, 1, 30), "NFO", 8_000_011).word(),
        "token_expired"
    );
    let plan = futures_plan(
        &book,
        route(),
        "ZQIDX",
        window(day(2026, 1, 1), day(2026, 1, 29)),
        day(2026, 1, 30),
        day(2026, 1, 29),
    )
    .unwrap();
    assert!(plan.refused.is_empty(), "{:?}", plan.refused);
    let contracts: Vec<(u64, Window)> = plan
        .asks
        .iter()
        .filter(|ask| matches!(ask.series, FuturesSeries::Contract { .. }))
        .map(|ask| (ask.token, ask.window))
        .collect();
    assert_eq!(
        contracts,
        vec![
            (8_000_011, window(day(2026, 1, 5), day(2026, 1, 27))),
            (8_000_012, window(day(2026, 1, 5), day(2026, 1, 29))),
        ]
    );
    assert_eq!(plan.incomplete.len(), 1, "{:?}", plan.incomplete);
    assert!(
        plan.incomplete[0].starts_with(&cut) && plan.incomplete[0].contains("never read"),
        "{:?}",
        plan.incomplete
    );
}

/// R4-28 repair 1: snapshots held before the index existed (or written by a
/// process killed before it took them in) are taken in by the next plan, once,
/// and the plan equals the one over an index filled as they were written.
#[test]
fn a_snapshot_held_before_its_index_existed_is_taken_in_by_the_next_plan_once() {
    let indexed = scratch("held-indexed");
    snapshot_on(
        &indexed,
        day(2026, 1, 2),
        &[
            fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27"),
            fut(8_000_002, "ZQIDX26FEBFUT", "2026-02-24"),
        ],
    );
    let bare = scratch("held-bare");
    let stamp = master_snapshots::Stamp {
        day: day(2026, 1, 2),
        ordinal: 1,
    };
    for name in [
        master_snapshots::file_name(Master::Kite, stamp),
        master_snapshots::receipt_name(Master::Kite, stamp),
    ] {
        std::fs::copy(indexed.join(&name), bare.join(&name)).unwrap();
    }
    let index = bare.join(master_snapshots::first_listed::DIR);
    assert!(!index.exists(), "no index yet");
    let asked = window(day(2026, 1, 1), day(2026, 1, 9));
    let plan_of = |root: &Path| {
        futures_plan(
            &Book::open(root).unwrap(),
            route(),
            "ZQIDX",
            asked,
            day(2026, 1, 12),
            asked.to(),
        )
        .unwrap()
    };
    let first = plan_of(&bare);
    assert_eq!(first.asks, january_asks());
    assert_eq!(first, plan_of(&indexed));
    let log_path = master_snapshots::first_listed::generation_dir(&bare)
        .unwrap()
        .join(master_snapshots::first_listed::ABSORBED);
    let log = std::fs::read_to_string(&log_path).unwrap();
    assert_eq!(log.lines().count(), 1, "{log}");
    assert_eq!(plan_of(&bare), first);
    assert_eq!(
        std::fs::read_to_string(&log_path).unwrap(),
        log,
        "taken in once, never again"
    );
}

/// A book built in memory has no snapshots directory, so no index: refused
/// by name rather than planned from nothing.
#[test]
fn a_futures_plan_over_a_book_with_no_directory_is_refused_by_name() {
    let csv = format!(
        "{}\n{}\n",
        crate::vendor::KITE_MASTER_HEADER,
        fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27")
    );
    let stamp = master_snapshots::Stamp {
        day: day(2026, 1, 2),
        ordinal: 1,
    };
    let book = Book::of(vec![
        master_snapshots::Snapshot::parse(stamp, &csv).unwrap(),
    ])
    .unwrap();
    let why = futures_plan(
        &book,
        route(),
        "ZQIDX",
        window(day(2026, 1, 1), day(2026, 1, 9)),
        day(2026, 1, 12),
        day(2026, 1, 9),
    )
    .unwrap_err();
    assert!(
        why.starts_with("futures_index: a book built in memory has no snapshots directory"),
        "{why}"
    );
}

/// R4-28 repair 1, measured on a real-size dump: `futures_plan` for NIFTY at
/// 1, 10 and 100 snapshots of one dump, each a request's whole work (the book
/// opened, the plan made), n = 50 single calls a size. Needs
/// `BRUTEX_KITE_DUMP`, the path of a COPY of a Kite instrument dump: it is read,
/// never written and never copied into the repository. The snapshots go under
/// the system temporary directory and stay there. Held to the ratio harness's
/// ceiling (3x) at 100 snapshots against 1.
#[test]
#[ignore = "needs BRUTEX_KITE_DUMP, the path of a copy of a real Kite instrument dump"]
fn a_real_size_plan_does_not_grow_with_the_snapshots_held() {
    let path = std::env::var_os("BRUTEX_KITE_DUMP").expect("BRUTEX_KITE_DUMP");
    let body = std::fs::read(path).expect("the dump reads");
    let root = scratch("real-size");
    let first = day(2026, 10, 6);
    let asked = window(day(2026, 10, 1), day(2026, 12, 31));
    let mut held = 0_u32;
    let mut p50s = Vec::new();
    for count in [1_u32, 10, 100] {
        while held < count {
            let on = Day::from_days(first.days_from_epoch() + held).expect("a real day");
            let started = std::time::Instant::now();
            master_snapshots::snapshot(&root, Master::Kite, on, URL, &body).expect("a snapshot");
            println!(
                "MEASURED snapshot {} written, its futures taken in: {} ms",
                held + 1,
                started.elapsed().as_millis()
            );
            held += 1;
        }
        let mut ns: Vec<u128> = (0..50)
            .map(|_| {
                let started = std::time::Instant::now();
                let book = Book::open(&root).expect("a book");
                let plan = futures_plan(
                    &book,
                    route(),
                    "NIFTY",
                    asked,
                    day(2026, 10, 22),
                    day(2026, 12, 31),
                )
                .expect("a plan");
                assert!(plan.refused.is_empty(), "{:?}", plan.refused);
                assert!(!plan.asks.is_empty(), "NIFTY futures are asked");
                started.elapsed().as_nanos()
            })
            .collect();
        ns.sort_unstable();
        println!(
            "MEASURED futures_plan NIFTY at {count} snapshots: p50 {} us, p99 {} us, max {} us, n={}",
            ns[24] / 1_000,
            ns[48] / 1_000,
            ns[49] / 1_000,
            ns.len()
        );
        p50s.push(ns[24]);
    }
    assert!(
        p50s[2] <= p50s[0] * 3,
        "100 snapshots cost {} ns a plan against {} ns at 1",
        p50s[2],
        p50s[0]
    );
}

// ───────────── R4-28 repair 2: an index that does not match its commit log ─────────────

/// The one index file of `name` under the futures index, wherever the index
/// keeps it.
fn index_file_of(root: &Path, name: &str) -> PathBuf {
    let wanted = format!("{name}.idx");
    let mut todo = vec![root.join(master_snapshots::first_listed::DIR)];
    let mut found = Vec::new();
    while let Some(dir) = todo.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                todo.push(path);
            } else if path.file_name().and_then(|file| file.to_str()) == Some(wanted.as_str()) {
                found.push(path);
            }
        }
    }
    assert_eq!(found.len(), 1, "one index file of {name}: {found:?}");
    found.pop().unwrap()
}

/// What a plan over a damaged index found: the plan, and the damaged file's
/// bytes before and after it (`None` for no file).
struct AfterDamage {
    plan: FuturesPlan,
    damaged: Option<String>,
    left: Option<String>,
}

/// JAN listed from 2026-01-02 and FEB from 2026-01-05, with `damage` done to
/// the index file of ZQIDX after both snapshots were taken in; then `more`
/// snapshots are written, and the plan over 2026-01-01..09 is made.
fn plan_after_damage(name: &str, damage: impl FnOnce(&Path), more: &[Day]) -> AfterDamage {
    let masters = scratch(name);
    let jan = fut(8_000_001, "ZQIDX26JANFUT", "2026-01-27");
    let feb = fut(8_000_002, "ZQIDX26FEBFUT", "2026-02-24");
    snapshot_on(&masters, day(2026, 1, 2), std::slice::from_ref(&jan));
    snapshot_on(&masters, day(2026, 1, 5), &[jan.clone(), feb.clone()]);
    let file = index_file_of(&masters, "ZQIDX");
    damage(&file);
    let damaged = std::fs::read_to_string(&file).ok();
    for &on in more {
        snapshot_on(&masters, on, &[jan.clone(), feb.clone()]);
    }
    let plan = futures_plan(
        &Book::open(&masters).unwrap(),
        route(),
        "ZQIDX",
        window(day(2026, 1, 1), day(2026, 1, 9)),
        day(2026, 1, 12),
        day(2026, 1, 9),
    )
    .unwrap();
    AfterDamage {
        plan,
        damaged,
        left: std::fs::read_to_string(&file).ok(),
    }
}

/// Every contract from the day its first snapshot listed it, as an intact
/// index plans it, the continuous series last; and the damaged file left
/// exactly as it was found.
fn assert_planned_whole(after: &AfterDamage) {
    let plan = &after.plan;
    assert!(plan.refused.is_empty(), "{:?}", plan.refused);
    let spans: Vec<(u64, Window)> = plan
        .asks
        .iter()
        .map(|ask| (ask.token, ask.window))
        .collect();
    assert_eq!(
        spans,
        vec![
            (8_000_001, window(day(2026, 1, 2), day(2026, 1, 9))),
            (8_000_002, window(day(2026, 1, 5), day(2026, 1, 9))),
            (8_000_001, window(day(2026, 1, 1), day(2026, 1, 9))),
        ],
        "no contract dropped, none re-dated: {plan:?}"
    );
    assert_eq!(after.left, after.damaged, "never rewritten in place");
}

/// The plan says the index was rebuilt, and why, in words that are true.
fn assert_rebuilt_because(plan: &FuturesPlan, why: &str) {
    let said = plan.rebuilt.as_deref().unwrap_or_default();
    assert!(
        said.starts_with("generation g1 of the futures index did not match its commit log")
            && said.contains(why)
            && said.contains("generation g2 was started"),
        "{said}"
    );
}

/// MISSING: the underlying's file is gone while its commit log stands.
#[test]
fn a_futures_index_file_gone_while_its_commit_log_stands_never_plans_short() {
    let after = plan_after_damage(
        "damage-missing",
        |file| std::fs::remove_file(file).unwrap(),
        &[],
    );
    assert_planned_whole(&after);
    assert_rebuilt_because(
        &after.plan,
        "is missing, so holds 0 checked line(s) of ZQIDX",
    );
}

/// TRUNCATED: the file cut at a line boundary, FEB's line lost.
#[test]
fn a_futures_index_file_cut_at_a_line_boundary_never_plans_short() {
    let after = plan_after_damage(
        "damage-cut",
        |file| {
            let text = std::fs::read_to_string(file).unwrap();
            let first = text.split_inclusive('\n').next().unwrap().to_owned();
            assert!(first.contains("|8000001|"), "{text}");
            std::fs::write(file, first).unwrap();
        },
        &[],
    );
    assert_planned_whole(&after);
    assert_rebuilt_because(
        &after.plan,
        "holds 1 checked line(s) of ZQIDX where the commit log records 2",
    );
}

/// GARBLED: one committed line changed in place, its length kept.
#[test]
fn a_garbled_committed_futures_index_line_never_plans_short() {
    let after = plan_after_damage(
        "damage-garbled",
        |file| {
            let text = std::fs::read_to_string(file).unwrap();
            let garbled = text.replacen("|2026-01-02|", "|2026-01-03|", 1);
            assert_ne!(garbled, text);
            assert_eq!(garbled.len(), text.len());
            std::fs::write(file, garbled).unwrap();
        },
        &[],
    );
    assert_planned_whole(&after);
    assert_rebuilt_because(&after.plan, "has 1 line(s) that do not check");
}

/// TRUNCATED, THEN AN ORDINARY WRITE: the next snapshot must not re-file the
/// lost contract under its own, later day. The write finds the damage and
/// rebuilds, so the plan reads a generation that matches its log.
#[test]
fn a_snapshot_written_over_a_cut_futures_index_never_re_dates_a_contract() {
    let after = plan_after_damage(
        "damage-cut-then-write",
        |file| {
            let text = std::fs::read_to_string(file).unwrap();
            let first = text.split_inclusive('\n').next().unwrap().to_owned();
            std::fs::write(file, first).unwrap();
        },
        &[day(2026, 1, 7)],
    );
    assert_planned_whole(&after);
    assert_eq!(after.plan.rebuilt, None, "the write rebuilt it already");
}
