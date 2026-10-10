//! `/chain.json` over a generated census, generated spot bars and a generated
//! lineage ledger (D-3329). No row here is market data: the strikes sit near
//! 4,000 and the spot at 4,123.45, figures chosen to be no real NIFTY level of
//! the dates they are stamped on.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to keep panics \
              out of the crate rather than out of its tests"
)]

use super::*;
use crate::server::tests::{agreeing, site};
use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest, manifest_path};
use pull::session::Day;
use serde_json::{Value, json};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, StorePath};

/// `hh:mm:ss` IST on `y-m-d`, in UTC seconds.
fn ist(y: u16, m: u8, d: u8, hh: i64, mm: i64, ss: i64) -> i64 {
    i64::from(Day::new(y, m, d).expect("a day").days_from_epoch()) * 86_400 - 19_800
        + hh * 3_600
        + mm * 60
        + ss
}

/// The instant most answers are asked at: 2018-09-19 14:28:15 IST.
fn t1() -> i64 {
    ist(2018, 9, 19, 14, 28, 15)
}

/// A month later, after more strikes were first stored.
fn t2() -> i64 {
    ist(2018, 10, 22, 10, 0, 0)
}

/// The invented spot, paisa: 4,123.45.
const SPOT: i64 = 412_345;

fn month(y: u16, m: u8) -> YearMonth {
    YearMonth::new(y, m).expect("a month")
}

/// One FNO key of `symbol` on `exchange` at 1s.
fn fno(exchange: Exchange, symbol: &str, contract: &str, at: YearMonth) -> EntryKey {
    EntryKey {
        contract: Some(Contract::parse(contract).expect("a contract")),
        exchange,
        segment: Segment::Fno,
        symbol: Symbol::new(symbol).expect("a symbol"),
        timeframe: Timeframe::SECOND_1,
        month: at,
    }
}

/// A census row first stamped at the UTC second `first`.
fn held(key: EntryKey, first: i64) -> Held {
    Held::new(
        Entry {
            key,
            rows: 3,
            first_ts_micros: first * MICROS,
            last_ts_micros: (first + 60) * MICROS,
        },
        Closes::known(1_005, 1_007).expect("a close"),
    )
}

/// NIFTY 2019-03-28: three strikes first stored by [`t1`] (4,000 CE, 4,100
/// PE, 4,300 CE) and three first stored only in October (4,150 CE, 4,200 CE,
/// 4,250 PE), a future, a one-strike 2018-09-27 expiry and a three-strike
/// 2023-12-28 expiry whose strikes lie far apart.
fn census_rows() -> Vec<Held> {
    let sep = month(2018, 9);
    let oct = month(2018, 10);
    let n = |contract: &str, at: YearMonth| fno(Exchange::Nse, "NIFTY", contract, at);
    vec![
        held(n("2019-03-28-400000-CE", sep), ist(2018, 9, 3, 9, 20, 0)),
        held(n("2019-03-28-410000-PE", sep), ist(2018, 9, 10, 10, 0, 0)),
        held(n("2019-03-28-430000-CE", sep), ist(2018, 9, 5, 11, 0, 0)),
        held(n("2019-03-28-415000-CE", oct), ist(2018, 10, 1, 9, 30, 0)),
        held(n("2019-03-28-420000-CE", oct), ist(2018, 10, 5, 9, 30, 0)),
        held(n("2019-03-28-425000-PE", oct), ist(2018, 10, 8, 9, 30, 0)),
        held(n("2019-03-28-FUT", sep), ist(2018, 9, 3, 9, 15, 0)),
        held(n("2018-09-27-410000-CE", sep), ist(2018, 9, 3, 9, 16, 0)),
        held(n("2023-12-28-300000-CE", sep), ist(2018, 9, 3, 9, 17, 0)),
        held(n("2023-12-28-470000-PE", sep), ist(2018, 9, 4, 9, 17, 0)),
        held(n("2023-12-28-500000-CE", sep), ist(2018, 9, 5, 9, 17, 0)),
    ]
}

/// Writes `bars` (UTC second, close) to `segment`/`symbol`'s 1s file of
/// `at` on GDFL.
fn write_spot(
    root: &std::path::Path,
    segment: &str,
    symbol: &str,
    at: YearMonth,
    bars: &[(i64, i64)],
) {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: at,
        file: FileKind::Bars,
    })
    .expect("generated path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the same low 32 bits `api::bars::open_classified` checks"
    )]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let mut file = BarFile::open_or_create(root, path, id).expect("generated writer");
    let rows: Vec<Bar> = bars
        .iter()
        .map(|&(secs, close)| Bar {
            ts_micros: secs * MICROS,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1,
            open_interest: OI_NULL,
        })
        .collect();
    file.append(&rows).expect("generated records");
}

/// Writes `bars` (UTC second, close) to `symbol`'s option `contract` 1s file
/// of `at` on GDFL.
fn write_option(
    root: &std::path::Path,
    symbol: &str,
    contract: &str,
    at: YearMonth,
    bars: &[(i64, i64)],
) {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol,
        contract: Some(Contract::parse(contract).expect("a contract")),
        timeframe: Timeframe::SECOND_1,
        month: at,
        file: FileKind::Bars,
    })
    .expect("generated path");
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the same low 32 bits `api::bars::open_classified` checks"
    )]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let mut file = BarFile::open_or_create(root, path, id).expect("generated writer");
    let rows: Vec<Bar> = bars
        .iter()
        .map(|&(secs, close)| Bar {
            ts_micros: secs * MICROS,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1,
            open_interest: 0,
        })
        .collect();
    file.append(&rows).expect("generated records");
}

/// A site whose GDFL census holds `rows`, with NIFTY spot at [`SPOT`] at
/// [`t1`] and [`t2`].
fn fixture(name: &str, rows: &[Held]) -> Site {
    let dir = agreeing(name);
    let built = site(name, &dir);
    let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    for row in rows {
        let _append = manifest.record_held(*row).expect("records");
    }
    std::fs::write(
        manifest_path(&built.store_root, Vendor::Gdfl),
        manifest.image(),
    )
    .expect("a census on disk");
    write_spot(
        &built.store_root,
        "INDEX",
        "NIFTY",
        month(2018, 9),
        &[(t1(), SPOT)],
    );
    write_spot(
        &built.store_root,
        "INDEX",
        "NIFTY",
        month(2018, 10),
        &[(t2(), SPOT)],
    );
    built
}

fn ask(site: &Site, query: &str) -> (axum::http::StatusCode, Value) {
    let (status, headers, body) = answer(site, query);
    assert_eq!(
        headers[axum::http::header::CONTENT_TYPE],
        "application/json; charset=utf-8"
    );
    (status, serde_json::from_str(&body).expect("chain JSON"))
}

const BASE: &str = "feed=gdfl&underlying=NIFTY&timeframe=1s";

/// **DB-037, DB-008: A RUNG IS COUNTED OVER THE STRIKES STORED BY THEN.**
///
/// The NIFTY 2019-03-28 11,500 CE read OTM+5 over the whole census and is
/// OTM+1 over the strikes stored by its instant. Here the 4,300 CE at spot
/// 4,123.45 on 2018-09-19 is OTM+1 over [4,000, 4,100, 4,300]; a month later,
/// with 4,150, 4,200 and 4,250 stored, the same spot puts it at OTM+4. Before
/// D-3329 both instants counted the October strikes, so the September bar
/// read OTM+4 too. No strike first stored after the instant is named, and no
/// first stamp after it is shown.
#[test]
fn a_rung_counts_only_the_strikes_stored_by_then() {
    let built = fixture("chain-stored-by", &census_rows());
    let (status, body) = ask(&built, &format!("{BASE}&expiry=2019-03-28&at={}", t1()));
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["basis"], "stored_by_then");
    assert_eq!(body["exchange"], "NSE");
    assert_eq!(body["census"]["state"], "held");
    assert!(body["census"]["degraded"].is_null(), "{body}");
    assert!(body["generation"].is_u64(), "{body}");
    assert_eq!(
        body["spot_series"],
        json!({"segment": "INDEX", "key": "NIFTY", "asof": "exact"})
    );
    assert_eq!(body["lineage"]["state"], "absent");
    let chain = &body["chains"][0];
    assert_eq!(chain["at"], t1());
    assert_eq!(chain["spot"], SPOT);
    assert_eq!(chain["spot_t"], t1());
    assert_eq!(chain["spot_age_s"], 0);
    assert!(chain["refusal"].is_null(), "{chain}");
    assert_eq!(chain["n_strikes"], 3);
    assert_eq!(chain["atm"], 410_000);
    assert_eq!(chain["step_local"], 10_000);
    assert_eq!(
        chain["strikes"],
        json!([
            {"strike": 400_000, "k": -1, "ce": "ITM-1", "pe": "OTM+1",
             "ce_first": ist(2018, 9, 3, 9, 20, 0), "pe_first": null},
            {"strike": 410_000, "k": 0, "ce": "ATM", "pe": "ATM",
             "ce_first": null, "pe_first": ist(2018, 9, 10, 10, 0, 0)},
            {"strike": 430_000, "k": 1, "ce": "OTM+1", "pe": "ITM-1",
             "ce_first": ist(2018, 9, 5, 11, 0, 0), "pe_first": null},
        ])
    );
    let text = body.to_string();
    for later in ["415000", "420000", "425000"] {
        assert!(!text.contains(later), "a later strike leaked into {text}");
    }

    // THE PICK FORM, at both instants: the rung moves only because strikes
    // were stored in between, never because the census grew after the bar.
    let (status, body) = ask(
        &built,
        &format!(
            "{BASE}&expiry=2019-03-28&strike=430000&side=CE&at={},{}",
            t1(),
            t2()
        ),
    );
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(
        body["chains"][0]["pick"],
        json!({"strike": 430_000, "side": "CE", "k": 1, "rung": "OTM+1", "why": null})
    );
    assert_eq!(body["chains"][0]["n_strikes"], 3);
    assert!(body["chains"][0].get("strikes").is_none());
    assert_eq!(
        body["chains"][1]["pick"],
        json!({"strike": 430_000, "side": "CE", "k": 4, "rung": "OTM+4", "why": null})
    );
    assert_eq!(body["chains"][1]["n_strikes"], 6);
    assert_eq!(body["chains"][1]["step_local"], 5_000);
    // A PUT ONE STEP BELOW THE MONEY IS OUT OF IT.
    let (_, body) = ask(
        &built,
        &format!("{BASE}&expiry=2019-03-28&strike=400000&side=PE&at={}", t1()),
    );
    assert_eq!(body["chains"][0]["pick"]["rung"], "OTM+1");
    // A strike not stored by then is said so, never ranked.
    let (_, body) = ask(
        &built,
        &format!("{BASE}&expiry=2019-03-28&strike=420000&side=CE&at={}", t1()),
    );
    assert_eq!(
        body["chains"][0]["pick"],
        json!({"strike": 420_000, "side": "CE", "k": null, "rung": null,
               "why": "strike_not_stored_by_then"})
    );
}

/// **DB-G62: A LADDER TOO SPARSE TO HAVE A MONEY IS REFUSED, NOT CALLED ATM.**
///
/// The 2023-12-28 chain of [9,000, 12,700, 13,000] called the 12,700 PE "ATM"
/// at spot 11,945.30. Here [3,000, 4,700, 5,000] at 4,123.45: the nearest
/// strike is 576.55 from spot against a local step of 300.00. A chain of one
/// strike has no step at all.
#[test]
fn a_sparse_ladder_is_refused_not_called_atm() {
    let built = fixture("chain-sparse", &census_rows());
    let (status, body) = ask(
        &built,
        &format!("{BASE}&expiry=2023-12-28&strike=470000&side=PE&at={}", t1()),
    );
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let chain = &body["chains"][0];
    assert_eq!(chain["refusal"], "ladder_too_sparse", "{chain}");
    assert!(
        chain["detail"].as_str().expect("said").contains("57655"),
        "{chain}"
    );
    assert_eq!(chain["step_local"], 30_000);
    assert_eq!(chain["n_strikes"], 3);
    assert!(chain["atm"].is_null());
    assert_eq!(
        chain["pick"],
        json!({"strike": 470_000, "side": "PE", "k": null, "rung": null, "why": null})
    );
    let (_, body) = ask(&built, &format!("{BASE}&expiry=2018-09-27&at={}", t1()));
    let chain = &body["chains"][0];
    assert_eq!(chain["refusal"], "ladder_too_sparse");
    assert!(chain["step_local"].is_null());
    assert_eq!(chain["strikes"][0]["strike"], 410_000);
    assert!(
        chain["strikes"][0]["k"].is_null(),
        "no rung on a refused ladder"
    );
}

/// **DB-G04: WHILE THE CENSUS IS BEHIND, AN INSTANT IT DOES NOT COVER IS
/// REFUSED `census_behind`** -- and an instant before the first uncounted day
/// is answered, because every strike stored before it was counted.
#[test]
fn a_census_behind_refuses_only_the_instants_it_does_not_cover() {
    let built = fixture("chain-behind", &census_rows());
    let journal = pull::gdfl_import::journal_path(&built.store_root);
    std::fs::create_dir_all(journal.parent().expect("imports/")).expect("mkdir");
    std::fs::write(
        &journal,
        "\
begin options 2018-10-01 *
incomplete options 2018-10-01 * definition=2 files=9 seconds=99 failures=1 late=0 late_unresolved=0 max_back_s=0 stage=census reason=too_many_entries run=1791300000
",
    )
    .expect("a journal");
    let (status, body) = ask(
        &built,
        &format!(
            "{BASE}&expiry=2019-03-28&strike=430000&side=CE&at={},{}",
            t1(),
            t2()
        ),
    );
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["census"]["state"], "behind");
    assert_eq!(body["census"]["behind_from"], "2018-10-01");
    assert_eq!(body["census"]["behind_days"], 1);
    assert_eq!(body["chains"][0]["pick"]["rung"], "OTM+1");
    assert!(body["chains"][0]["refusal"].is_null());
    assert_eq!(body["chains"][1]["refusal"], "census_behind");
    assert!(body["chains"][1]["pick"]["rung"].is_null());
    assert_eq!(body["chains"][1]["spot"], SPOT, "the spot is still shown");
    // AN EXPIRY BEFORE THE FIRST UNCOUNTED DAY IS WHOLE in the census.
    let (_, body) = ask(&built, &format!("{BASE}&expiry=2018-09-27&at={}", t2()));
    assert_eq!(body["chains"][0]["refusal"], "ladder_too_sparse");
}

/// **A RECOVERED CENSUS IS SAID AND NOT RANKED ON** (`CLAUDE.md` §4, D-3327):
/// a census that loaded by stepping over a damaged header is an earlier
/// generation, so a strike a later one counted may be missing from it. Every
/// instant is refused `census_degraded` with the reason, and the spot is still
/// shown; the census object carries the same reason. Before, nothing in the
/// chain could tell a short census from a whole one.
#[test]
fn a_degraded_census_refuses_every_instant_by_name() {
    let built = fixture("chain-degraded", &census_rows());
    let mut damaged = pull::manifest::ManifestHeader::genesis(Vendor::Gdfl).image();
    damaged[16] ^= 0xFF;
    let good = pull::manifest::ManifestHeader {
        generation: 1,
        ..pull::manifest::ManifestHeader::genesis(Vendor::Gdfl)
    }
    .image();
    let mut bytes = vec![0u8; 32_768];
    bytes[..64].copy_from_slice(&damaged);
    bytes[16_384..16_448].copy_from_slice(&good);
    std::fs::write(manifest_path(&built.store_root, Vendor::Gdfl), &bytes).expect("damaged");
    let (status, body) = ask(
        &built,
        &format!(
            "{BASE}&exchange=NSE&expiry=2019-03-28&strike=430000&side=CE&at={},{}",
            t1(),
            t2()
        ),
    );
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    let why = body["census"]["degraded"]
        .as_str()
        .expect("the census object names what was stepped over");
    assert!(!why.is_empty(), "{body}");
    for chain in body["chains"].as_array().expect("instants") {
        assert_eq!(chain["refusal"], "census_degraded", "{chain}");
        assert!(
            chain["detail"].as_str().expect("said").contains(why),
            "{chain}"
        );
        assert!(chain["atm"].is_null() && chain["pick"]["rung"].is_null());
        assert_eq!(chain["spot"], SPOT, "the spot is still shown");
    }
}

/// **D-3328 ON THE CHAIN: NO SPOT, NO MONEY; `asof=prev` SAYS HOW OLD.**
#[test]
fn the_spot_rule_is_the_spot_routes_and_its_age_is_shown() {
    let built = fixture("chain-spot", &census_rows());
    let gap = t1() + 7;
    let (_, body) = ask(
        &built,
        &format!("{BASE}&expiry=2019-03-28&strike=430000&side=CE&at={gap}"),
    );
    let chain = &body["chains"][0];
    assert_eq!(chain["refusal"], "no_spot_bar");
    assert_eq!(chain["spot_why"], "no_spot_bar");
    assert!(chain["spot"].is_null() && chain["spot_t"].is_null());
    let (_, body) = ask(
        &built,
        &format!("{BASE}&expiry=2019-03-28&strike=430000&side=CE&asof=prev&at={gap}"),
    );
    assert_eq!(body["spot_series"]["asof"], "prev");
    let chain = &body["chains"][0];
    assert!(chain["refusal"].is_null(), "{chain}");
    assert_eq!(chain["spot"], SPOT);
    assert_eq!(chain["spot_t"], t1());
    assert_eq!(chain["spot_age_s"], 7);
    assert_eq!(chain["pick"]["rung"], "OTM+1");
    // The day before has no bar of its own session: refused, never
    // yesterday's close.
    let (_, body) = ask(
        &built,
        &format!(
            "{BASE}&expiry=2019-03-28&asof=prev&at={}",
            ist(2018, 9, 20, 9, 15, 0)
        ),
    );
    assert_eq!(body["chains"][0]["refusal"], "no_spot_in_session");
    // A month with no spot file.
    let (_, body) = ask(
        &built,
        &format!("{BASE}&expiry=2019-03-28&at={}", ist(2018, 11, 1, 10, 0, 0)),
    );
    assert_eq!(body["chains"][0]["refusal"], "no_spot_file");
}

/// **D-3331 ON THE CHAIN: FROM AN EX-DATE THE LEDGER DATES, A KEY FIRST
/// STORED BEFORE IT LEAVES THE CHAIN, UNLESS A RECORD SAYS IT HOLDS A CONTRACT
/// FROM THEN** (DB-G73).
///
/// NTPC 2019-03-28 was ranked after its ex-date against its whole pre-ex 2.50
/// grid. The ledger links only the contracts that traded across the ex-date,
/// and D-3331 records that the vendor renames every strike, so here, from the
/// ex-date 2018-09-20:
///
/// * the 4,300 CE, renamed by a `lineage` record, and the 4,100 CE, pre-ex
///   and named by no record, leave the chain;
/// * the 2,000 CE (stored before, as the old 2,000 CE) is the NEW key of a
///   rename, and the 4,200 CE a `stitched` key whose `after` names the
///   contract renamed onto it: both hold a contract from the ex-date's first
///   instant;
/// * the 4,000 CE is a `stitched` key whose `after` is `-`, a contract NEWLY
///   listed on it: it is back from its first stored bar on or after the
///   ex-date (10:00 that day), read from its own month file, and not before;
/// * the 2,150 CE is first stored on the ex-date, a key of its own.
///
/// The day before the ex-date keeps the whole pre-ex chain: no record dated
/// after the instant's day is read.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one ledger, then the chain on each side of the ex-date in time order: the \
              day before, the ex-date before and at the relisting, and the day after"
)]
fn a_lineage_record_takes_a_renamed_strike_out_from_its_ex_date() {
    let symbol = Symbol::new("ZEDCO").expect("a symbol");
    let sep = month(2018, 9);
    let z = |contract: &str| fno(Exchange::Nse, "ZEDCO", contract, sep);
    let relisted = ist(2018, 9, 20, 10, 0, 0);
    let rows = vec![
        held(z("2018-09-27-200000-CE"), ist(2018, 9, 4, 9, 20, 0)),
        held(z("2018-09-27-400000-CE"), ist(2018, 9, 3, 9, 20, 0)),
        held(z("2018-09-27-410000-CE"), ist(2018, 9, 3, 9, 21, 0)),
        held(z("2018-09-27-420000-CE"), ist(2018, 9, 3, 9, 23, 0)),
        held(z("2018-09-27-430000-CE"), ist(2018, 9, 3, 9, 22, 0)),
        held(z("2018-09-27-215000-CE"), ist(2018, 9, 20, 9, 22, 0)),
    ];
    let built = fixture("chain-lineage", &rows);
    write_spot(
        &built.store_root,
        "CASH",
        "ZEDCO",
        sep,
        &[(t1(), SPOT), (ist(2018, 9, 21, 10, 0, 0), SPOT / 2)],
    );
    // THE 4,000 CE's OWN MONTH FILE: its pre-ex bar, and the first bar of the
    // contract newly listed on its key.
    write_option(
        &built.store_root,
        "ZEDCO",
        "2018-09-27-400000-CE",
        sep,
        &[(ist(2018, 9, 3, 9, 20, 0), 1_005), (relisted, 2_005)],
    );
    let contract = |text: &str| Contract::parse(text).expect("a contract");
    let ex = Day::new(2018, 9, 20).expect("a day");
    let rename = |old: &str, new: &str| {
        LineageRecord {
            underlying: symbol,
            old: contract(old),
            new: contract(new),
            ex_date: ex,
            factor: (2, 1),
            lot_old: 500,
            lot_new: 1_000,
            oi_unadjusted_until: 0,
        }
        .line()
    };
    let stitch = |key: &str, after: Option<&str>| {
        StitchedRecord {
            underlying: symbol,
            key: contract(key),
            ex_date: ex,
            before: None,
            after: after.map(contract),
        }
        .line()
    };
    // THE RE-KEY'S TWO RECORDS (D-3331; WP-12c) are read records, never
    // "unread": the census epoch installed with them lists neither key.
    let superseded = pull::gdfl_lineage::SupersededRecord {
        underlying: symbol,
        old: contract("2023-06-29-400000-CE"),
        new: contract("2023-06-28-400000-CE"),
        kind: pull::expiry_alias::AliasKind::VendorRedated,
        months: 1,
        bars: 2,
        digest: [5; 32],
    }
    .line();
    let retired = pull::gdfl_lineage::RetiredRecord {
        underlying: symbol,
        key: contract("2018-09-27-90000000-CE"),
        why: pull::gdfl_lineage::RetiredWhy::StrikeImplausible,
        day: Day::new(2018, 9, 3).expect("a day"),
        median: Some(410_000),
        days: 1,
        bars: 1,
    }
    .line();
    let lines = [
        rename("2018-09-27-430000-CE", "2018-09-27-215000-CE"),
        rename("2018-09-27-400000-CE", "2018-09-27-200000-CE"),
        stitch("2018-09-27-400000-CE", None),
        stitch("2018-09-27-420000-CE", Some("2018-09-27-840000-CE")),
        superseded,
        retired,
        "a line this build does not read".to_owned(),
    ];
    let ledger = pull::gdfl_lineage::lineage_path(&built.store_root);
    std::fs::create_dir_all(ledger.parent().expect("imports/")).expect("mkdir");
    std::fs::write(&ledger, lines.join("\n")).expect("a ledger");
    let query = "feed=gdfl&underlying=ZEDCO&timeframe=1s&expiry=2018-09-27";
    let at = |t: i64| ask(&built, &format!("{query}&at={t}")).1;
    let body = at(t1());
    assert_eq!(
        body["lineage"]["state"], "partial",
        "the unread line is said"
    );
    assert_eq!(body["lineage"]["records"], 6);
    assert_eq!(body["lineage"]["unread_lines"], 1);
    assert_eq!(body["spot_series"]["segment"], "CASH");
    let strikes = |body: &Value| -> Vec<i64> {
        body["chains"][0]["strikes"]
            .as_array()
            .expect("strikes")
            .iter()
            .map(|s| s["strike"].as_i64().expect("paisa"))
            .collect()
    };
    let first_of = |body: &Value, strike: i64| -> Value {
        body["chains"][0]["strikes"]
            .as_array()
            .expect("strikes")
            .iter()
            .find(|s| s["strike"] == strike)
            .map_or(Value::Null, |s| s["ce_first"].clone())
    };
    // BEFORE THE EX-DATE: the pre-ex chain, the adjusted strike not yet
    // stored.
    let pre_ex = [200_000, 400_000, 410_000, 420_000, 430_000];
    assert_eq!(strikes(&body), pre_ex);
    assert_eq!(body["chains"][0]["atm"], 410_000, "{body}");
    assert_eq!(
        strikes(&at(ist(2018, 9, 19, 15, 29, 0))),
        pre_ex,
        "the day before keeps the whole pre-ex chain"
    );
    // ON THE EX-DATE, BEFORE THE RELISTING: renamed and unheld keys out; the
    // reused keys held from the day's first instant; the 4,000 CE not yet.
    let ex_start = ist(2018, 9, 20, 0, 0, 0);
    let body = at(ist(2018, 9, 20, 9, 30, 0));
    assert_eq!(strikes(&body), [200_000, 215_000, 420_000]);
    assert_eq!(first_of(&body, 200_000), ex_start);
    assert_eq!(first_of(&body, 420_000), ex_start);
    // FROM THE RELISTING'S FIRST STORED BAR the 4,000 CE is back, counted
    // from that bar.
    let body = at(relisted);
    assert_eq!(strikes(&body), [200_000, 215_000, 400_000, 420_000]);
    assert_eq!(first_of(&body, 400_000), relisted);
    let body = at(ist(2018, 9, 21, 10, 0, 0));
    assert_eq!(strikes(&body), [200_000, 215_000, 400_000, 420_000]);
    assert_eq!(body["chains"][0]["atm"], 200_000, "{body}");
    assert_eq!(body["chains"][0]["step_local"], 15_000, "{body}");
    assert!(body["faults"].is_null(), "{body}");
}

/// The census's own states: absent is said per instant, unreadable is a 503,
/// an expiry it does not hold is named, and an underlying two exchanges hold
/// must be named with `exchange=`.
#[test]
fn the_census_state_is_said_and_never_read_as_an_empty_chain() {
    let built = fixture("chain-census", &census_rows());
    let (status, body) = ask(&built, &format!("{BASE}&expiry=2019-04-25&at={}", t1()));
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(body["chains"][0]["refusal"], "expiry_not_in_census");
    assert_eq!(body["chains"][0]["spot"], SPOT);

    let (status, body) = ask(
        &built,
        &format!(
            "feed=zerodha&underlying=NIFTY&timeframe=1min&expiry=2019-03-28&at={}",
            t1()
        ),
    );
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["census"]["state"], "absent");
    assert!(body["generation"].is_null());
    assert_eq!(body["lineage"]["state"], "not_kept");
    assert_eq!(body["chains"][0]["refusal"], "census_absent");

    std::fs::write(
        manifest_path(&built.store_root, Vendor::Gdfl),
        b"not a manifest",
    )
    .expect("damaged");
    let (status, body) = ask(&built, &format!("{BASE}&expiry=2019-03-28&at={}", t1()));
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
    assert_eq!(body["error"], "census_unreadable");

    let mut rows = census_rows();
    rows.push(held(
        fno(
            Exchange::Bse,
            "NIFTY",
            "2019-03-28-400000-CE",
            month(2018, 9),
        ),
        ist(2018, 9, 3, 9, 20, 0),
    ));
    let both = fixture("chain-two-exchanges", &rows);
    let (status, body) = ask(&both, &format!("{BASE}&expiry=2019-03-28&at={}", t1()));
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert!(body["error"].as_str().expect("said").contains("exchange="));
    let (status, body) = ask(
        &both,
        &format!("{BASE}&exchange=NSE&expiry=2019-03-28&at={}", t1()),
    );
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["chains"][0]["n_strikes"], 3);
}

/// A malformed question is a 400 naming the field, before any file is read.
#[test]
fn malformed_questions_are_refused_by_name() {
    let built = fixture("chain-malformed", &census_rows());
    let base = format!("{BASE}&expiry=2019-03-28&at={}", t1());
    let nine = vec![t1().to_string(); MAX_FULL_CHAINS + 1].join(",");
    let many = vec![t1().to_string(); MAX_CHAIN_TIMES + 1].join(",");
    for (query, word) in [
        (base.replace("feed=gdfl", "feed=nobody"), "feed"),
        (
            base.replace("underlying=NIFTY", "underlying="),
            "underlying is required",
        ),
        (
            base.replace("expiry=2019-03-28", "expiry=2019-3-28"),
            "expiry",
        ),
        (
            base.replace("timeframe=1s", "timeframe="),
            "timeframe is required",
        ),
        (format!("{base}&strike=430000"), "side"),
        (format!("{base}&strike=430000&side=XE"), "side"),
        (format!("{base}&strike=-5&side=CE"), "strike"),
        (format!("{base}&asof=later"), "asof"),
        (base.replace(&format!("&at={}", t1()), ""), "at is required"),
        (
            base.replace(&format!("at={}", t1()), &format!("at={nine}")),
            "chunks",
        ),
        (
            format!("{base}&strike=430000&side=CE")
                .replace(&format!("at={}", t1()), &format!("at={many}")),
            "chunks",
        ),
        (format!("{base}&exchange=NYSE"), "exchange"),
    ] {
        let (status, body) = ask(&built, &query);
        assert_eq!(
            status,
            axum::http::StatusCode::BAD_REQUEST,
            "{query}: {body}"
        );
        assert!(
            body["error"].as_str().expect("named").contains(word),
            "{query}: {body}"
        );
    }
}

/// **DB-066, DB-G80: THE HANDLER READS THE STORE ON THE BLOCKING POOL,
/// ADMITTED** -- the body, read from source, hands `answer` to
/// `run_store_read` and nothing else; and through the handler the answer is
/// the same one.
#[tokio::test]
async fn the_handler_answers_from_the_store_read_pool() {
    let source = include_str!("chain.rs");
    let handler = source
        .split("pub(super) async fn chain_json(")
        .nth(1)
        .expect("the handler");
    assert!(
        handler.contains("crate::detail::run_store_read(move || answer(&site, &query))"),
        "{handler}"
    );
    assert_eq!(handler.matches("answer(").count(), 1, "and nowhere else");
    let _apart = crate::detail::apart_from_slot_owners().await;
    let built = std::sync::Arc::new(fixture("chain-handler", &census_rows()));
    let uri: axum::http::Uri = format!("/chain.json?{BASE}&expiry=2019-03-28&at={}", t1())
        .parse()
        .expect("URI");
    let (status, _, body) = chain_json(axum::extract::State(built), uri).await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert!(body.contains(r#""atm":410000"#), "{body}");
}

/// The rung names, both sides: a call below the money is in it.
#[test]
fn a_rung_is_named_the_way_the_page_reads_it() {
    assert_eq!(rung(0, true), "ATM");
    assert_eq!(rung(-2, true), "ITM-2");
    assert_eq!(rung(3, true), "OTM+3");
    assert_eq!(rung(-2, false), "OTM+2");
    assert_eq!(rung(3, false), "ITM-3");
    assert_eq!(expiry_param("2019-03-28"), Expiry::new(2019, 3, 28).ok());
    for bad in ["2019-03-28-x", "19-03-28", "2019-3-28", "2019-02-30", ""] {
        assert_eq!(expiry_param(bad), None, "{bad}");
    }
}
