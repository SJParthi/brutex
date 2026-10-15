#![cfg(test)]
//! The `/bars` page shows the contract it was asked for, and every route reads
//! one month spelling (AF-04: findings app-6 and app-7).
//!
//! Every price and timestamp below is INVENTED. The spot opens at 77,777.00
//! and its options at a few hundred rupees with round counts, so no row here
//! can be mistaken for a vendor's.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use super::*;
use axum::http::StatusCode;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use pull::manifest::{Entry, EntryKey, Manifest, manifest_path};
use std::fs;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The NIFTY option every NIFTY case asks for.
const OPTION: &str = "2024-03-07-2200000-CE";
/// A RELIANCE option, expired years before the fixture's "today".
const STOCK_OPTION: &str = "2018-09-27-100000-CE";
/// NIFTY spot, held as INDEX: what the underlying's page answers.
const SPOT: &str = "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2024-03";
/// The NIFTY option with its whole key.
const ADDRESSED: &str = "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY\
                         &contract=2024-03-07-2200000-CE&timeframe=1s&month=2024-03";
/// One bar more than a page, so the option has a second page to turn to.
const OPTION_BARS: usize = bars::PAGE_BARS + 1;

/// One invented series to plant: its segment, name, contract, month, first
/// open in paisa and how many one-second bars.
struct Planted {
    segment: Segment,
    symbol: &'static str,
    contract: Option<&'static str>,
    month: YearMonth,
    open: i64,
    bars: usize,
}

/// The series every test here reads: two spot months and an option of each.
fn planted() -> [Planted; 4] {
    let march = YearMonth::new(2024, 3).expect("month");
    let september = YearMonth::new(2018, 9).expect("month");
    [
        Planted {
            segment: Segment::Index,
            symbol: "NIFTY",
            contract: None,
            month: march,
            open: 7_777_700,
            bars: 3,
        },
        Planted {
            segment: Segment::Fno,
            symbol: "NIFTY",
            contract: Some(OPTION),
            month: march,
            open: 33_300,
            bars: OPTION_BARS,
        },
        Planted {
            segment: Segment::Cash,
            symbol: "RELIANCE",
            contract: None,
            month: september,
            open: 9_999_900,
            bars: 4,
        },
        Planted {
            segment: Segment::Fno,
            symbol: "RELIANCE",
            contract: Some(STOCK_OPTION),
            month: september,
            open: 44_400,
            bars: 5,
        },
    ]
}

/// A store root of invented GDFL bars with a census that holds them.
struct Fixture {
    root: PathBuf,
    site: Loaded,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = crate::scratch::path(name);
        let masters = root.join("masters");
        fs::create_dir_all(&masters).expect("empty offline masters");
        let site = Loaded::new(Site::load(&masters, &root));
        let fixture = Self { root, site };
        let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis manifest");
        for series in planted() {
            let (first, last) = fixture.plant(&series);
            manifest
                .record(Entry {
                    key: EntryKey {
                        contract: series
                            .contract
                            .map(|c| brutex_core::instrument::Contract::parse(c).expect("c")),
                        exchange: Exchange::Nse,
                        segment: series.segment,
                        symbol: Symbol::new(series.symbol).expect("fixture symbol"),
                        timeframe: Timeframe::SECOND_1,
                        month: series.month,
                    },
                    rows: u64::try_from(series.bars).expect("rows"),
                    first_ts_micros: first,
                    last_ts_micros: last,
                })
                .expect("one held month");
        }
        let path = manifest_path(&fixture.root, Vendor::Gdfl);
        fs::create_dir_all(path.parent().expect("manifest parent")).expect("manifest dir");
        let staged = path.with_extension("staged");
        fs::write(&staged, manifest.image()).expect("stage the manifest");
        fs::rename(&staged, &path).expect("install the manifest");
        fixture
    }

    /// Write one series' month: one-second bars from 09:15 IST on the 1st
    /// weekday the month opens on. Returns the first and last stamps.
    fn plant(&self, series: &Planted) -> (i64, i64) {
        let date = if series.month.year() == 2024 { 1 } else { 3 };
        let day = Day::new(series.month.year(), series.month.month(), date).expect("date");
        let open_micros = i64::from(day.days_from_epoch()) * 86_400_000_000 + 13_500_000_000;
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: series.segment.as_str(),
            symbol: series.symbol,
            contract: series
                .contract
                .map(|c| brutex_core::instrument::Contract::parse(c).expect("contract")),
            timeframe: Timeframe::SECOND_1,
            month: series.month,
            file: FileKind::Bars,
        })
        .expect("fixture path");
        let hash = brutex_core::universe::fnv1a(series.symbol).to_le_bytes();
        let id = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file =
            store::file::BarFile::open_or_create(&self.root, path, id).expect("a bar file");
        let rows: Vec<Bar> = (0..series.bars)
            .map(|n| {
                let n = i64::try_from(n).expect("n");
                Bar {
                    ts_micros: open_micros + n * 1_000_000,
                    open: series.open + n * 100,
                    high: series.open + n * 100 + 500,
                    low: series.open + n * 100 - 500,
                    close: series.open + n * 100,
                    volume: n + 1,
                    open_interest: if series.contract.is_some() {
                        1_000
                    } else {
                        OI_NULL
                    },
                }
            })
            .collect();
        file.append(&rows).expect("invented bars");
        (
            rows.first().expect("one").ts_micros,
            rows.last().expect("one").ts_micros,
        )
    }

    fn bars(&self, query: &str) -> (StatusCode, String) {
        bars_html(&self.site, query)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// How many bars the page says the month holds.
fn total_of(page: &str) -> &str {
    let (before, _) = page.split_once("</b> bar(s)").expect("the bar count line");
    before.rsplit("<b>").next().expect("the count")
}

/// The `/bars` page answers the contract it was asked for: its file, its
/// count, its header — and a page turn keeps it (app-6).
#[test]
fn the_bars_page_shows_the_contract_it_was_asked_for() {
    let fixture = Fixture::new("af04-bars-contract");
    let (status, page) = fixture.bars(ADDRESSED);
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(total_of(&page), OPTION_BARS.to_string());
    assert!(
        page.contains("NIFTY 2024-03-07-2200000-CE<br>as it is stored."),
        "{page}"
    );
    assert!(page.contains("brutex · NIFTY 2024-03-07-2200000-CE · 2024-03"));
    assert!(page.contains("GDFL · FNO · 2024-03 · 1S"));
    // The page turn carries the contract, and page two is the contract's.
    let next = page
        .split("href=\"")
        .map(|rest| rest.split('"').next().unwrap_or(""))
        .find(|href| href.contains("page=1"))
        .expect("a Next link on a two-page month")
        .replace("&amp;", "&");
    assert!(next.contains("&contract=2024-03-07-2200000-CE&"), "{next}");
    let (status, second) = fixture.bars(next.trim_start_matches("/bars?"));
    assert_eq!(status, StatusCode::OK, "{second}");
    assert!(second.contains("page 2 of 2"), "{second}");
    assert_eq!(total_of(&second), OPTION_BARS.to_string());
    // The spot page beside it is unchanged: three bars, no contract named.
    let (status, spot) = fixture.bars(SPOT);
    assert_eq!(status, StatusCode::OK);
    assert_eq!(total_of(&spot), "3");
    assert!(!spot.contains(OPTION));
}

/// A contract the store cannot name is a 400 that names the parameter, never
/// the underlying's page (app-6).
#[test]
fn a_malformed_contract_on_the_bars_page_is_400() {
    let fixture = Fixture::new("af04-bars-malformed");
    for bad in [
        "2024-03-07-2200000-ce",
        "NIFTY-2024-03-07-2200000-CE",
        "2024%2F03",
        "..",
    ] {
        for segment in ["&segment=FNO&exchange=NSE", ""] {
            let query = format!(
                "feed=gdfl{segment}&symbol=NIFTY&contract={bad}&timeframe=1s&month=2024-03"
            );
            let (status, page) = fixture.bars(&query);
            assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {page}");
            assert!(page.contains("contract=&quot;"), "{query}: {page}");
            assert!(page.contains("is not a contract segment"), "{query}");
            assert_eq!(total_of(&page), "0", "{query}");
        }
    }
}

/// With `segment=` omitted, a contract is placed where a contract is filed —
/// never at the underlying's INDEX or CASH series (app-6, the worse case the
/// second skeptic found).
#[test]
fn the_bars_page_never_answers_a_contract_with_its_underlyings_bars() {
    let fixture = Fixture::new("af04-bars-underlying");
    // NIFTY: the index holds 3 bars, the option OPTION_BARS.
    let (status, page) = fixture
        .bars("feed=gdfl&symbol=NIFTY&contract=2024-03-07-2200000-CE&timeframe=1s&month=2024-03");
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(total_of(&page), OPTION_BARS.to_string());
    assert!(page.contains("GDFL · FNO · 2024-03"), "{page}");
    assert!(!page.contains("· INDEX ·"));
    // RELIANCE: cash holds 4 bars, the expired option 5.
    let (status, page) = fixture
        .bars("feed=gdfl&symbol=RELIANCE&contract=2018-09-27-100000-CE&timeframe=1s&month=2018-09");
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(total_of(&page), "5");
    assert!(page.contains("RELIANCE 2018-09-27-100000-CE<br>"), "{page}");
    assert!(!page.contains("· CASH ·"));
    // Exchange given, segment omitted: FNO by the type, no census read.
    let (status, page) = fixture.bars(
        "feed=gdfl&exchange=NSE&symbol=RELIANCE&contract=2018-09-27-100000-CE&timeframe=1s&month=2018-09",
    );
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(total_of(&page), "5");
    // A month outside the contract's life, nothing but the contract given:
    // 404 naming the contract files asked about, never a spot path.
    let (status, page) = fixture
        .bars("feed=gdfl&symbol=NIFTY&contract=2024-03-07-2200000-CE&timeframe=1s&month=2024-04");
    assert_eq!(status, StatusCode::NOT_FOUND, "{page}");
    assert!(
        page.contains("/NSE/FNO/NIFTY/2024-03-07-2200000-CE/1s/2024-04.bin"),
        "{page}"
    );
    assert!(
        page.contains("/BSE/FNO/NIFTY/2024-03-07-2200000-CE/1s/2024-04.bin"),
        "{page}"
    );
    assert!(
        !page.contains("/FNO/NIFTY/1s/") && !page.contains("/INDEX/"),
        "{page}"
    );
}

/// The extreme cases the specification names: an expired contract, a month
/// outside its life with the whole key, `contract=` on a spot segment, an empty
/// `contract=`, and `contract=` given twice.
#[test]
fn the_bars_page_contract_extremes_answer_by_name() {
    let fixture = Fixture::new("af04-bars-extremes");
    // Expired years before today, whole key: its own bars.
    let (status, page) = fixture.bars(
        "feed=gdfl&exchange=NSE&segment=FNO&symbol=RELIANCE&contract=2018-09-27-100000-CE\
         &timeframe=1s&month=2018-09",
    );
    assert_eq!(status, StatusCode::OK, "{page}");
    assert_eq!(total_of(&page), "5");
    // Outside its life, whole key: the 404 names the CONTRACT file.
    let (status, page) = fixture.bars(&ADDRESSED.replace("month=2024-03", "month=2024-05"));
    assert_eq!(status, StatusCode::NOT_FOUND, "{page}");
    assert!(
        page.contains("/FNO/NIFTY/2024-03-07-2200000-CE/1s/2024-05.bin"),
        "{page}"
    );
    assert!(!page.contains("/FNO/NIFTY/1s/"), "{page}");
    // A contract under a segment that files none is refused, both spot kinds.
    for (segment, symbol, contract) in [
        ("INDEX", "NIFTY", OPTION),
        ("CASH", "RELIANCE", STOCK_OPTION),
        ("BOGUS", "NIFTY", OPTION),
    ] {
        let query = format!(
            "feed=gdfl&exchange=NSE&segment={segment}&symbol={symbol}&contract={contract}\
             &timeframe=1s&month=2024-03"
        );
        let (status, page) = fixture.bars(&query);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {page}");
        assert!(page.contains(&format!("contract={contract}")), "{page}");
        assert!(page.contains("files no contracts"), "{page}");
    }
    // A symbol no path can carry, with a contract and nothing else: a 400
    // naming the symbol, never a 404 that "asked about" no file at all.
    for symbol in ["", "..", "NIFTY%2F..%2FX"] {
        let query =
            format!("feed=gdfl&symbol={symbol}&contract={OPTION}&timeframe=1s&month=2024-03");
        let (status, page) = fixture.bars(&query);
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {page}");
        assert!(
            page.contains("is not a name this store files a contract under"),
            "{query}: {page}"
        );
        assert_eq!(total_of(&page), "0", "{query}");
    }
    // An empty contract is absent: the spot page, byte for byte.
    let (status, empty) = fixture.bars(&format!("{SPOT}&contract="));
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty, fixture.bars(SPOT).1);
    // Given twice: refused at the boundary before any handler reads either.
    let headers = axum::http::HeaderMap::new();
    for route in ["/bars", "/bars.json"] {
        let uri: axum::http::Uri = format!("{route}?{ADDRESSED}&contract=2024-03-07-2250000-PE")
            .parse()
            .expect("a target");
        let (status, why) = request_bounds_refusal(&uri, &headers).expect("refused");
        assert_eq!(status, StatusCode::BAD_REQUEST, "{route}: {why}");
        assert!(why.contains("contract"), "{route}: {why}");
    }
}

/// Every contract row on `/store` links to its contract, and the link opens
/// that contract's bars (app-6, the `/store` half).
#[test]
fn a_store_contract_row_links_to_its_contract_and_the_link_opens() {
    let fixture = Fixture::new("af04-store-links");
    let today = Day::new(2026, 10, 14).expect("today");
    let (status, page) = store_html(&fixture.site, today, 0, "feed=gdfl");
    assert_eq!(status, StatusCode::OK, "{page}");
    let links: Vec<String> = page
        .split("href=\"/bars?")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap_or("").replace("&amp;", "&"))
        .collect();
    assert_eq!(links.len(), 4, "one link per held series: {links:?}");
    for planted in planted() {
        let link = links
            .iter()
            .find(|link| {
                link.contains(&format!("&symbol={}&", planted.symbol))
                    && link.contains(&format!("&segment={}&", planted.segment.as_str()))
            })
            .expect("the series' link");
        match planted.contract {
            Some(contract) => assert!(link.contains(&format!("&contract={contract}&")), "{link}"),
            None => assert!(!link.contains("contract="), "{link}"),
        }
        let (status, bars) = fixture.bars(link);
        assert_eq!(status, StatusCode::OK, "{link}: {bars}");
        assert_eq!(total_of(&bars), planted.bars.to_string(), "{link}");
    }
}

/// The parser itself: exactly `YYYY-MM`, digits only, and the range.
#[test]
fn the_one_month_parser_takes_exactly_four_digits_a_dash_and_two() {
    for good in ["2024-03", "1970-01", "9999-12"] {
        assert_eq!(
            bars::month_of_text(good).map(|m| m.to_string()).as_deref(),
            Some(good)
        );
    }
    for bad in [
        "",
        "2024-3",
        "2024-003",
        "2024-0003",
        "02024-03",
        "+2024-03",
        "2024-+3",
        "+024-03",
        "2024-13",
        "2024-00",
        "1969-12",
        "2024-03 ",
        " 2024-03",
        "2024/03",
        "2024-0x",
        "２０２４-03",
        "2024-03-01",
        "-2024-03",
    ] {
        assert_eq!(bars::month_of_text(bad), None, "{bad:?}");
    }
    // The `/store` filter reads the same parser: a misspelt bound narrows
    // nothing (shown as "All"), the one spelling narrows to its month.
    let march = YearMonth::new(2024, 3).expect("month");
    let filter = store_filter("from=2024-03&to=2024-03");
    assert_eq!((filter.from, filter.to), (Some(march), Some(march)));
    for bad in ["2024-3", "2024-003", "02024-03", "%2B2024-03", "2024-%2B3"] {
        let filter = store_filter(&format!("from={bad}&to={bad}"));
        assert_eq!((filter.from, filter.to), (None, None), "{bad}");
    }
}

/// `route?query` through its handler: status and body. A `429` — a shared
/// blocking pool full — is asked again rather than read as the answer.
async fn answer(site: &Loaded, route: &str, query: &str) -> (StatusCode, String) {
    for _ in 0..400 {
        let uri: axum::http::Uri = format!("{route}?{query}").parse().expect("URI");
        let state = axum::extract::State(Loaded::clone(site));
        let (status, body) = match route {
            "/bars.json" => {
                let (status, _, body) = bars_json(state, uri).await;
                (status, body)
            }
            "/gaps.json" => {
                let (status, _, body) = gaps_json(state, uri).await;
                (status, body)
            }
            "/provenance.json" => {
                let (status, _, body) = provenance::provenance_json(state, uri).await;
                (status, body)
            }
            "/spot.json" => {
                let (status, _, body) = spot::spot_json(state, uri).await;
                (status, body)
            }
            "/greeks.json" => {
                let (status, _, body) = optiongreeks::greeks_json(state, uri).await;
                (status, body)
            }
            "/bars" => bars_html(site, query),
            other => panic!("not a route here: {other}"),
        };
        if status != StatusCode::TOO_MANY_REQUESTS {
            return (status, body);
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("{route}: the pool stayed full for two seconds");
}

/// Every route that reads a month refuses each spelling that is not `YYYY-MM`
/// with the sentence it already used, and still answers `2024-03` (app-7).
#[tokio::test]
async fn every_route_refuses_a_month_not_spelled_yyyy_mm() {
    let fixture = Fixture::new("af04-month-spellings");
    // Each route, its query with `{M}` where the month goes, and the field
    // the sentence names.
    let routes = [
        (
            "/bars.json",
            format!("{}&month={{M}}", SPOT.replace("&month=2024-03", "")),
            "month",
        ),
        (
            "/bars",
            format!("{}&month={{M}}", SPOT.replace("&month=2024-03", "")),
            "month",
        ),
        (
            "/spot.json",
            "feed=gdfl&exchange=NSE&symbol=NIFTY&timeframe=1s&month={M}&t=1709264700".to_owned(),
            "month",
        ),
        (
            "/greeks.json",
            "feed=gdfl&exchange=NSE&segment=FNO&symbol=NIFTY&contract=2024-03-07-2200000-CE\
             &timeframe=1s&month={M}&t=1709264700"
                .to_owned(),
            "month",
        ),
        ("/gaps.json", format!("{SPOT}&to={{M}}"), "to"),
        (
            "/provenance.json",
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&timeframe=1s&month={M}".to_owned(),
            "month",
        ),
    ];
    for (route, template, field) in &routes {
        for spelling in [
            "2024-3",
            "2024-003",
            "2024-0003",
            "02024-03",
            "%2B2024-03",
            "2024-%2B3",
        ] {
            let query = template.replace("{M}", spelling);
            let (status, body) = answer(&fixture.site, route, &query).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{route}?{query}: {body}");
            let decoded = percent_decode(spelling);
            let sentence = if *route == "/bars" {
                format!("&quot;{decoded}&quot; is not a YYYY-MM month for &quot;{field}&quot;")
            } else {
                format!("\\\"{decoded}\\\" is not a YYYY-MM month for \\\"{field}\\\"")
            };
            assert!(body.contains(&sentence), "{route}?{query}: {body}");
        }
        // The one spelling is still answered: no month refusal at all.
        let query = template.replace("{M}", "2024-03");
        let (status, body) = answer(&fixture.site, route, &query).await;
        assert!(
            !body.contains("is not a YYYY-MM month"),
            "{route}?{query}: {body}"
        );
        if matches!(*route, "/bars.json" | "/bars") {
            assert_eq!(status, StatusCode::OK, "{route}?{query}: {body}");
        }
    }
    // The canonical month reads the same three bars on both bar routes.
    let (_, json) = answer(&fixture.site, "/bars.json", SPOT).await;
    let rows: serde_json::Value = serde_json::from_str(&json).expect("bars JSON");
    assert_eq!(rows.as_array().map(Vec::len), Some(3), "{json}");
}
