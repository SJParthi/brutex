//! One not-owed authority on every owed route (R4-19; findings G4, Z3,
//! COR-07). `/gaps.json` and `/pull/repair.json` read the same FORCEMOT
//! store and agree session for session; R4-19d adds the calendar and the
//! day band to this file, R4-03 the `/store` rows.
//!
//! FORCEMOT, INE451A01017 and the interval 2023-10-26..=2024-02-13 are the
//! documented facts (`docs/00-charter.md`, "FORCEMOT NSE interruption");
//! every price, every master row and every listing day here is invented, and
//! no bar is a vendor's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "test-only assertions"
)]

use std::path::Path;

use brutex_core::vendor::Vendor;
use pull::ingest::repair_plan::{self, Owes};
use pull::session::{Day, IST_OFFSET_SECS};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{Site, gaps_answer};

fn day(year: u16, month: u8, date: u8) -> Day {
    Day::new(year, month, date).unwrap()
}

/// Every minute each sized session of `from..=to` owes a share, outside
/// `skip`, less the one `hole` minute, as `symbol`'s invented Zerodha 1min
/// months.
fn land(
    root: &Path,
    symbol: &str,
    (from, to): (Day, Day),
    skip: Option<(Day, Day)>,
    hole: Option<(Day, u32)>,
) {
    let mut months: Vec<(YearMonth, Vec<Bar>)> = Vec::new();
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let on = Day::from_days(number).unwrap();
        if skip.is_some_and(|(first, last)| on >= first && on <= last) {
            continue;
        }
        let Owes::Windows((windows, count)) = repair_plan::owed("CASH", number) else {
            continue;
        };
        let base = i64::from(number) * 86_400 - IST_OFFSET_SECS;
        let bars: Vec<Bar> = windows[..count]
            .iter()
            .flat_map(|&(open, close)| open..close)
            .filter(|minute| hole != Some((on, *minute)))
            .map(|minute| Bar {
                ts_micros: (base + i64::from(minute) * 60) * 1_000_000,
                open: 55_500,
                high: 55_510,
                low: 55_490,
                close: 55_505,
                volume: 1,
                open_interest: OI_NULL,
            })
            .collect();
        let month = on.year_month().unwrap();
        match months.last_mut() {
            Some((held, rows)) if *held == month => rows.extend(bars),
            _ => months.push((month, bars)),
        }
    }
    for (month, rows) in months {
        let path = StorePath::new(PathParts {
            vendor: Vendor::Zerodha,
            exchange: "NSE",
            segment: "CASH",
            symbol,
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        })
        .unwrap();
        let mut file =
            store::file::BarFile::open_or_create(root, path, repair_plan::symbol_id(symbol))
                .unwrap();
        file.append(&rows).unwrap();
    }
}

/// A served site over `root`, with no instrument masters of its own.
fn site(root: &Path) -> super::Loaded {
    std::sync::Arc::new(Site::load(&root.join("masters-none"), root))
}

/// `/gaps.json?{query}` over `site`, as JSON, refusing anything but a `200`.
async fn gaps(site: &super::Loaded, query: &str) -> serde_json::Value {
    let (code, body) = gaps_answer(std::sync::Arc::clone(site), query).await;
    assert_eq!(code, 200, "{query}: {body}");
    serde_json::from_str(&body).unwrap()
}

/// Every run's reason word in a `/gaps.json` month row, with its day.
fn runs(month: &serde_json::Value) -> Vec<(i64, String)> {
    month["gaps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|run| {
            (
                run["day"].as_i64().unwrap(),
                run["reason"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// **G4 and Z3, on every owed route: a FORCEMOT session NSE withdrew is owed
/// by none.** The operator's store, invented: every session FORCEMOT owed
/// from 2023-09 to 2024-03 held whole but one minute of 2024-02-20, and no
/// file for November to January. Before R4-19 `/gaps.json` answered
/// 2023-12 `expected 7500, lost_minutes 7500` (live `:8080`, 20 Oct) and
/// `/pull/repair.json` named 2024-02 a repair month; now `/gaps.json` owes
/// and loses nothing in the withdrawal, names it with both NSE sources, still
/// loses the one real minute, and the repair plan names no repair month and
/// the same 76 sessions not owed.
#[tokio::test]
async fn forcemot_withdrawal_days_are_not_owed_on_every_route() {
    let root = crate::scratch::path("r4-19-forcemot-routes");
    let _fresh = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let (from, through) = (day(2023, 10, 26), day(2024, 2, 13));
    let hole = (day(2024, 2, 20), 700);
    land(
        &root,
        "FORCEMOT",
        (day(2023, 9, 1), day(2024, 3, 31)),
        Some((from, through)),
        Some(hole),
    );
    let site = site(&root);

    // /gaps.json, the CASH door.
    let answer = gaps(
        &site,
        "feed=zerodha&exchange=NSE&segment=CASH&symbol=FORCEMOT&timeframe=1min&month=2023-10&to=2024-02",
    )
    .await;
    assert_eq!(answer["lost_minutes"], 1, "only the real hole: {answer}");
    let months = answer["month"].as_array().unwrap();
    assert_eq!(months.len(), 5);
    let mut days = 0_u64;
    let mut minutes = 0_u64;
    for month in months {
        let name = month["month"].as_str().unwrap();
        let rows = month["not_owed"].as_array().unwrap();
        assert_eq!(rows.len(), 1, "{name}: one answer: {month}");
        let row = &rows[0];
        assert_eq!(row["code"], "instrument_inactive", "{name}");
        assert_eq!(row["reason"], "instrument-inactive");
        assert_eq!(row["symbol"], "FORCEMOT");
        assert_eq!(
            (row["from"].as_str(), row["through"].as_str()),
            (Some("2023-10-26"), Some("2024-02-13"))
        );
        let sources = row["sources"].as_array().unwrap();
        assert!(
            sources[0].as_str().unwrap().ends_with("/CML58560.pdf"),
            "{row}"
        );
        assert!(
            sources[1]
                .as_str()
                .unwrap()
                .starts_with("https://www.forcemotors.com/"),
            "{row}"
        );
        assert_eq!(
            row["circulars"],
            serde_json::json!(["CML58560", "CML60618"]),
            "both NSE circulars by number: {row}"
        );
        assert_eq!(row["minutes"], month["not_owed_minutes"], "{name}");
        days += row["days"].as_u64().unwrap();
        minutes += row["minutes"].as_u64().unwrap();
        let first = i64::from(from.days_from_epoch());
        let last = i64::from(through.days_from_epoch());
        for (on, word) in runs(month) {
            if (first..=last).contains(&on) {
                assert_ne!(word, "vendor-hole", "{name} {on}: withdrawn is not lost");
            }
        }
    }
    assert_eq!(days, 76, "the withdrawal's sessions of record");
    assert_eq!(answer["not_owed_minutes"], minutes);
    let december = &months[2];
    assert_eq!(
        (
            december["expected"].as_u64(),
            december["lost_minutes"].as_u64()
        ),
        (Some(0), Some(0))
    );
    assert!(
        december["absent_file"].is_string(),
        "the file is still absent, and said"
    );
    let february = &months[4];
    assert_eq!(february["lost_minutes"], 1, "{february}");
    assert!(
        runs(february).contains(&(
            i64::from(hole.0.days_from_epoch()),
            "vendor-hole".to_owned()
        )),
        "the hole after the readmission is still the vendor's"
    );
    // A MONTH FORCEMOT OWED WHOLE is answered as before: nothing excused.
    let september = gaps(
        &site,
        "feed=zerodha&exchange=NSE&segment=CASH&symbol=FORCEMOT&timeframe=1min&month=2023-09",
    )
    .await;
    assert_eq!(september["lost_minutes"], 0);
    assert_eq!(september["not_owed_minutes"], 0);
    assert_eq!(september["month"][0]["not_owed"], serde_json::json!([]));

    // /pull/repair.json, the same store.
    let (status, _, body) = crate::server::repair_route::plan_answer(
        &root,
        "feed=zerodha&instrument=FORCEMOT&segment=CASH&from=2023-09-01&to=2024-03-31",
    );
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let plan: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(plan["totals"]["not_owed_sessions"], 76, "{plan}");
    assert_eq!(
        plan["totals"]["repair_months"], 1,
        "2024-02's interior hole, and only it"
    );
    let feb = &plan["months"][5];
    assert_eq!(feb["month"], "2024-02");
    assert_eq!(feb["not_owed"][0]["from"], "2024-02-01");
    assert_eq!(feb["not_owed"][0]["to"], "2024-02-13");
    assert_eq!(feb["not_owed"][0]["sessions"], 9);
    assert_eq!(feb["not_owed"][0]["code"], "instrument_inactive");
    let evidence = feb["not_owed"][0]["evidence"].as_str().unwrap();
    assert!(
        evidence.contains("CML58560") && evidence.contains("CML60618"),
        "both circulars by number: {evidence}"
    );
    assert!(
        feb["missing"].as_array().unwrap().is_empty(),
        "no session before the 14th is missing: {feb}"
    );
    assert!(
        feb["short"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["day"] == "2024-02-20"),
        "{feb}"
    );
    for index in 2..=4 {
        let month = &plan["months"][index];
        assert_eq!(
            (month["sessions"].as_u64(), month["repair_needed"].as_bool()),
            (Some(0), Some(false)),
            "{month}"
        );
    }
    drop(site);
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// Seconds from 1980-01-01 to `on`, as a master states a lifecycle date.
fn nse_seconds(on: Day) -> u64 {
    u64::from(on.days_from_epoch() - day(1980, 1, 1).days_from_epoch()) * 86_400
}

/// A gzip member of one stored (uncompressed) deflate block: `api` has no
/// compressor, and the master reader only needs a valid member. The helper
/// `bars_change`'s and `dayflags_rules`' tests use.
fn stored_gzip(text: &[u8]) -> Vec<u8> {
    let mut crc = !0u32;
    for &byte in text {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    let len = u16::try_from(text.len()).expect("one stored block");
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 1];
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(text);
    out.extend_from_slice(&(!crc).to_le_bytes());
    out.extend_from_slice(&u32::try_from(text.len()).expect("small").to_le_bytes());
    out
}

/// **COR-07 on `/gaps.json`: a session before the listing day the store's
/// master states reads `before-listing`, owes nothing, and names its
/// master.** An invented master row (the universe's IRFC ISIN, an invented
/// listing day) and invented June 2023 bars from that day.
#[tokio::test]
async fn a_session_before_the_masters_listing_day_is_not_owed_on_gaps() {
    let root = crate::scratch::path("r4-19-listing-gaps");
    let _fresh = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let listed = day(2023, 6, 14);
    let isin = brutex_core::universe::nse_isin("IRFC").unwrap();
    let csv = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n\
         9200,IRFC,EQ,{},1,{},0,0\n",
        isin.as_str(),
        nse_seconds(listed)
    );
    pull::cash_session_cache::install(
        &root.join(pull::listing::MASTER_DIR),
        day(2026, 8, 3),
        &stored_gzip(csv.as_bytes()),
    )
    .unwrap();
    land(&root, "IRFC", (listed, day(2023, 6, 30)), None, None);
    let site = site(&root);
    let answer = gaps(
        &site,
        "feed=zerodha&exchange=NSE&segment=CASH&symbol=IRFC&timeframe=1min&month=2023-06",
    )
    .await;
    let june = &answer["month"][0];
    assert_eq!(june["lost_minutes"], 0, "{june}");
    let row = &june["not_owed"][0];
    assert_eq!(row["code"], "before_listing");
    assert_eq!(row["date"], "2023-06-14");
    assert_eq!(row["days"], 9, "1st to 13th June");
    assert_eq!(row["master"]["file"], "NSE_CM_security_03082026.csv.gz");
    assert_eq!(row["master"]["sha256"].as_str().map(str::len), Some(64));
    assert!(runs(june).iter().all(|(_, word)| word != "vendor-hole"));
    drop(site);
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// **One class for one day on every rung**: a GDFL one-second FORCEMOT
/// month inside the withdrawal, with no file, reads `instrument-inactive`
/// on each session, never `no-print` (the trade-driven door, R4-19).
#[tokio::test]
async fn a_one_second_cash_month_inside_the_withdrawal_reads_inactive_not_no_print() {
    let root = crate::scratch::path("r4-19-forcemot-1s");
    let _fresh = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let site = site(&root);
    let answer = gaps(
        &site,
        "feed=gdfl&exchange=NSE&segment=CASH&symbol=FORCEMOT&timeframe=1s&month=2023-12",
    )
    .await;
    let december = &answer["month"][0];
    let words: Vec<String> = runs(december).into_iter().map(|(_, word)| word).collect();
    assert!(
        words.iter().any(|word| word == "instrument-inactive"),
        "{december}"
    );
    assert!(
        words
            .iter()
            .all(|word| word != "no-print" && word != "vendor-hole"),
        "{december}"
    );
    assert_eq!(
        december["not_owed"][0]["days"], 20,
        "December 2023's sessions"
    );
    drop(site);
    let _cleanup = std::fs::remove_dir_all(&root);
}

/// An answer with the fields that say only HOW it was read (a summary kept or
/// built by whichever request came first, the records that took) removed:
/// what the classification states, which must not depend on who asked first.
fn stated(body: &str) -> serde_json::Value {
    let mut answer: serde_json::Value = serde_json::from_str(body).unwrap();
    if let Some(top) = answer.as_object_mut() {
        top.remove("summaries");
    }
    for month in answer["month"].as_array_mut().unwrap() {
        if let Some(row) = month.as_object_mut() {
            for how in ["summary", "summary_fallback", "bar_records_read"] {
                row.remove(how);
            }
        }
    }
    answer
}

/// **Out of the box: the listing's first load races, and every answer is
/// still the same** (the operator's concurrent case, on the one door). Eight
/// `/gaps.json` requests for one IRFC month at once, over a store whose
/// receipted master no request of this process has loaded: each loads or
/// reads `pull::listing::for_store`'s table, and every one states the same
/// classification — the sessions before the master's listing day not owed,
/// the hole after it the vendor's — whichever request loaded it first. Then
/// a share the master does not name, asked beside it, is excused nothing,
/// and the first answer, asked again from the cached table, is unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_first_requests_over_an_unloaded_master_state_one_answer() {
    let root = crate::scratch::path("r4-19-listing-race");
    let _fresh = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let listed = day(2023, 6, 14);
    let isin = brutex_core::universe::nse_isin("IRFC").unwrap();
    let csv = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n\
         9200,IRFC,EQ,{},1,{},0,0\n",
        isin.as_str(),
        nse_seconds(listed)
    );
    pull::cash_session_cache::install(
        &root.join(pull::listing::MASTER_DIR),
        day(2026, 8, 3),
        &stored_gzip(csv.as_bytes()),
    )
    .unwrap();
    // OWED FROM THE 14TH, AND ONE MINUTE OF THE 20TH NEVER STORED.
    land(
        &root,
        "IRFC",
        (listed, day(2023, 6, 30)),
        None,
        Some((day(2023, 6, 20), 700)),
    );
    let site = site(&root);
    let query = "feed=zerodha&exchange=NSE&segment=CASH&symbol=IRFC&timeframe=1min&month=2023-06";
    let asks: Vec<_> = (0..8)
        .map(|_| {
            let site = std::sync::Arc::clone(&site);
            tokio::spawn(async move { gaps_answer(site, query).await })
        })
        .collect();
    let mut answers = Vec::new();
    for ask in asks {
        let (code, body) = ask.await.unwrap();
        assert_eq!(code, 200, "{body}");
        answers.push(stated(&body));
    }
    assert!(
        answers.windows(2).all(|pair| pair[0] == pair[1]),
        "one classification whoever loaded the master: {answers:?}"
    );
    let june = &answers[0]["month"][0];
    assert_eq!(june["lost_minutes"], 1, "only the 20th's minute: {june}");
    let row = &june["not_owed"][0];
    assert_eq!(
        (
            row["code"].as_str(),
            row["date"].as_str(),
            row["days"].as_u64()
        ),
        (Some("before_listing"), Some("2023-06-14"), Some(9)),
        "{row}"
    );
    assert_eq!(june["not_owed"].as_array().map(Vec::len), Some(1));
    // A NAME THAT IS NOT THE MASTER'S IS EXCUSED NOTHING, asked beside it:
    // an invented share with no file and no ISIN owes June whole and loses
    // it, as before R4-19, and asking it changes nothing of IRFC's answer.
    let other = gaps(
        &site,
        "feed=zerodha&exchange=NSE&segment=CASH&symbol=IRFCZZ&timeframe=1min&month=2023-06",
    )
    .await;
    let other_june = &other["month"][0];
    assert_eq!(
        other_june["not_owed"],
        serde_json::json!([]),
        "{other_june}"
    );
    assert_eq!(other_june["not_owed_minutes"], 0);
    assert!(other_june["expected"].as_u64().unwrap() > 0, "{other_june}");
    assert_eq!(other_june["lost_minutes"], other_june["expected"]);
    let again = gaps(&site, query).await;
    assert_eq!(
        stated(&again.to_string()),
        answers[0],
        "the cached table answers as the racing loads did"
    );
    drop(site);
    let _cleanup = std::fs::remove_dir_all(&root);
}
