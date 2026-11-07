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

/// An `event` and a `stopped` record of the lineage ledger (D-3355) are read
/// by the chain's index and move no rung: before they were known here each
/// would have counted as unread and turned the chain's lineage state to
/// `partial`. Any other unknown line still does.
#[test]
fn the_event_and_stopped_records_are_read_and_move_no_rung() {
    let text = "event NSE-FNO-NIFTY ex=2020-06-16 kind=relisted shift=0 lot_old=75 lot_new=75 paired=0 stopped=1 v=1\n\
                stopped NSE-FNO-NIFTY-2020-06-25-1000000-CE ex=2020-06-16 v=1\n";
    let index = LineageIndex::of(text);
    assert_eq!((index.records, index.unread), (2, 0));
    let torn = LineageIndex::of(&format!("{text}stopped NSE-FNO-NIFTY ex=2020-06-16 v=1\n"));
    assert_eq!((torn.records, torn.unread), (2, 1));
}

/// An expiry with a SIGNED field is refused, never read as another spelling
/// of a day (cloud-attack-law-20261016, lens 8): the width check let
/// `2019-+3-28` through, and `str::parse` read it as 2019-03-28.
#[test]
fn an_expiry_with_a_signed_field_is_refused_not_read_as_another_spelling() {
    assert_eq!(expiry_param("2019-03-28"), Expiry::new(2019, 3, 28).ok());
    for bad in [
        "2019-+3-28",
        "2019-03-+8",
        "+019-03-28",
        "2019-3-28",
        "2019-03-2",
    ] {
        assert_eq!(expiry_param(bad), None, "{bad}");
    }
}

/// **A CHAIN BESIDE A WRITER SAYS "BEING WRITTEN", NEVER `spot_unreadable`**
/// (R3-08 repair 2, verify-R3-08-r2 blocking 3; NEW-21). With the
/// underlying's month held by a writer, `/chain.json` answered 206, every
/// instant `spot_why` `spot_unreadable`, ATM null and no `Retry-After` —
/// measured on a copy of gdfl NIFTY 1s 2019-11 held by `lockf(1)` — so /db's
/// chain and moneyness showed damage and never asked again. Now:
///
/// * the writer as the only fault: the one being-written 503, its sentence
///   naming the holder, no rung and no `spot_unreadable`;
/// * real damage beside it (a spot month that is not a bar file): the 206
///   that names both, the held instant `spot_being_written` and the damaged
///   one `spot_unreadable`;
/// * the writer gone: 200 and the ladder counted from the invented spot.
#[test]
fn a_chain_beside_a_writer_says_being_written_never_spot_unreadable() {
    let built = fixture("chain-beside-writer", &census_rows());
    let spot_path = |at: YearMonth| {
        StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "INDEX",
            symbol: "NIFTY",
            contract: None,
            timeframe: Timeframe::SECOND_1,
            month: at,
            file: FileKind::Bars,
        })
        .expect("generated path")
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the same low 32 bits `api::bars::open_classified` checks"
    )]
    let id = brutex_core::universe::fnv1a("NIFTY") as u32;
    let held = spot_path(month(2018, 9));
    let writer_root = built.store_root.clone();
    let (holding_tx, holding_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
    let writer = std::thread::Builder::new()
        .name("r3-08-chain-writer".to_owned())
        .spawn(move || {
            let file = BarFile::open_or_create(&writer_root, held, id).expect("the writer");
            holding_tx.send(()).expect("the test waits for the writer");
            let _ = release_rx.recv();
            drop(file);
        })
        .expect("a writer thread");
    holding_rx.recv().expect("the writer holds the month");
    // THE ROUTE'S OWN PATIENCE: every store read is admitted with it.
    let patient = |query: &str| {
        store::flock::with_patience(
            std::time::Duration::from_millis(crate::detail::READ_PATIENCE_MS),
            || ask(&built, query),
        )
    };
    let (status, body) = patient(&format!("{BASE}&expiry=2019-03-28&at={}", t1()));
    assert_eq!(
        status,
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "{body}"
    );
    assert_eq!(body["being_written"], true, "{body}");
    let error = body["error"].as_str().expect("the sentence");
    assert!(error.contains(store::flock::BEING_WRITTEN), "{body}");
    assert!(
        error.contains("r3-08-chain-writer"),
        "names the holder: {body}"
    );
    assert!(body.get("chains").is_none(), "no rung is sent: {body}");
    assert!(!body.to_string().contains("unreadable"), "{body}");

    // REAL DAMAGE BESIDE THE WRITER: November's spot month is not a bar file.
    let damaged = spot_path(month(2018, 11)).to_path_buf(&built.store_root);
    std::fs::create_dir_all(damaged.parent().expect("a parent")).expect("a directory");
    std::fs::write(&damaged, b"not a bar file, invented for this test").expect("damage");
    let november = ist(2018, 11, 1, 10, 0, 0);
    let (status, body) = patient(&format!("{BASE}&expiry=2019-03-28&at={},{november}", t1()));
    assert_eq!(status, axum::http::StatusCode::PARTIAL_CONTENT, "{body}");
    assert_eq!(
        body["chains"][0]["spot_why"], "spot_being_written",
        "{body}"
    );
    assert_eq!(body["chains"][1]["spot_why"], "spot_unreadable", "{body}");
    let faults = body["faults"].as_str().expect("the faults");
    assert!(faults.contains(store::flock::BEING_WRITTEN), "{faults}");

    release_tx.send(()).expect("the writer lets go");
    writer.join().expect("the writer thread");
    let (status, body) = ask(&built, &format!("{BASE}&expiry=2019-03-28&at={}", t1()));
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["chains"][0]["spot"], SPOT, "{body}");
    assert_eq!(body["chains"][0]["atm"], 410_000, "{body}");
}

/// **A LEDGER RECORD THIS BUILD CANNOT KEY IS UNREAD, AN ALIAS IS READ, AND A
/// MARK MOVES ONLY WHAT ITS OWN RECORDS NAME** (BASE-COV-API repair 3,
/// D-8661). `LineageIndex::of` and `LineageIndex::mark`, driven directly.
/// Extremes: a lineage record whose new key is struck at 0 paisa, which the
/// ledger's own reader accepts and the option reader refuses (parsed, not
/// keyed: unread); an expiry alias record (read, no rung moved); an empty
/// line (passed over, neither read nor unread); a plain rename (read); a mark
/// asked for an underlying the ledger never names (nothing moves); a renamed
/// key whose side was first stored at the ex-date's first instant, not before
/// it (still marked, held by nothing); a renamed key's other side, first
/// stored before the ex-date (marked the same); the reused key with no first
/// stamp (never marked); and, at once, both keys of one record struck at 0
/// (one unread line, never two), and every side of two strikes marked in one
/// pass with only the renamed strike's sides moved; out of the box, a strike
/// of 0 paisa.
#[test]
fn a_ledger_record_this_build_cannot_key_is_unread_and_a_mark_moves_only_its_own() {
    let zedco = Symbol::new("ZEDCO").expect("a symbol");
    let contract = |text: &str| Contract::parse(text).expect("a contract");
    let ex = Day::new(2018, 9, 20).expect("a day");
    let rename = |old: &str, new: &str| {
        LineageRecord {
            underlying: zedco,
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
    let alias = pull::expiry_alias::AliasRecord {
        underlying: Some(zedco),
        stated: Day::new(2018, 9, 27).expect("a day"),
        settled: Day::new(2018, 9, 26).expect("a day"),
        kind: pull::expiry_alias::AliasKind::ClosedDay,
        first_day: Day::new(2018, 9, 3).expect("a day"),
        contracts: 3,
    }
    .line();
    let lines = [
        rename("2018-09-27-0-CE", "2018-09-27-0-PE"),
        rename("2018-09-27-410000-CE", "2018-09-27-0-CE"),
        alias,
        String::new(),
        rename("2018-09-27-400000-CE", "2018-09-27-200000-CE"),
    ];
    for line in &lines[..2] {
        assert!(
            LineageRecord::parse(line).is_some(),
            "the ledger reads its own line: {line}"
        );
    }
    let index = LineageIndex::of(&lines.join("\n"));
    assert_eq!(
        (index.records, index.unread),
        (2, 2),
        "the alias and the plain rename read; each record struck at 0 unread once"
    );

    let expiry = Expiry::new(2018, 9, 27).expect("an expiry");
    let ex_day = i64::from(ex.days_from_epoch());
    let from = ex_start_micros(ex_day);
    let side = |first: Option<i64>| Side {
        first,
        adjusted: Vec::new(),
    };
    let fresh = || {
        vec![
            Strike {
                paisa: 200_000,
                call: side(None),
                put: side(None),
            },
            Strike {
                paisa: 400_000,
                call: side(Some(from)),
                put: side(Some(from - MICROS)),
            },
        ]
    };
    // AN UNDERLYING THE LEDGER NEVER NAMES: nothing moves, nothing is asked.
    let mut other = fresh();
    index.mark(
        Symbol::new("OTHERCO").expect("a symbol"),
        expiry,
        &mut other,
        &mut |_, _, _| unreachable!("no listed key"),
    );
    assert_eq!(other, fresh());
    // ZEDCO: the renamed 4,000 CE first stored at the ex-date's first instant
    // (not before it) is marked; its put, first stored before it, too; the
    // reused 2,000 CE with no first stamp is not.
    let mut marked = fresh();
    index.mark(zedco, expiry, &mut marked, &mut |_, _, _| {
        unreachable!("no listed key")
    });
    assert_eq!(
        marked[0],
        fresh()[0],
        "the reused key, never stored, moves nothing"
    );
    assert_eq!(marked[1].call.adjusted, vec![(ex_day, None)]);
    assert_eq!(marked[1].put.adjusted, vec![(ex_day, None)]);
}

/// **A LISTED KEY'S FIRST STAMP IS READ FROM ITS OWN MONTHS, A MONTH AT A
/// TIME, AND A MONTH THAT WILL NOT READ IS A NAMED FAULT** (BASE-COV-API
/// repair 3, D-8661). `first_stored_from`, driven directly over month files
/// written here. Extremes: a first month holding only bars before the
/// instant asked (passed over), an absent month after it (passed over) and
/// the expiry's month holding the first bar after it (answered); an instant
/// after the last bar of the expiry's month (none, no fault); an instant in
/// a month past the expiry's (none, no month opened); a key with no month
/// file at all (none, no fault); a month file that is not a bar file
/// (refused: none, the fault names the file); an instant at exactly a bar's
/// stamp (that bar); and, at once, an unreadable month before a readable
/// one (the fault stops the walk: none, never the later bar), and an
/// instant one microsecond after the expiry month's last bar (none); out of
/// the box, the instant `i64::MIN`, which no calendar places.
#[test]
fn a_listed_keys_first_stamp_is_read_month_by_month_and_a_month_that_will_not_read_is_named() {
    let root = crate::scratch::path("chain-first-stored");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let key = |text: &str| OptionKey {
        root: &root,
        feed: Vendor::Gdfl,
        exchange: "NSE",
        underlying: "ZEDCO",
        contract: Contract::parse(text).expect("a contract"),
        timeframe: Timeframe::SECOND_1,
    };
    let expiry = Expiry::new(2018, 11, 29).expect("an expiry");
    let (early, late) = (ist(2018, 9, 3, 9, 20, 0), ist(2018, 11, 5, 9, 20, 0));
    write_option(
        &root,
        "ZEDCO",
        "2018-11-29-400000-PE",
        month(2018, 9),
        &[(early, 1_005)],
    );
    write_option(
        &root,
        "ZEDCO",
        "2018-11-29-400000-PE",
        month(2018, 11),
        &[(late, 1_006)],
    );
    let mut faults = Vec::new();
    let mut first =
        |text: &str, from: i64| first_stored_from(&key(text), from, expiry, &mut faults);
    let pe = "2018-11-29-400000-PE";
    assert_eq!(
        first(pe, ist(2018, 9, 20, 0, 0, 0) * MICROS),
        Some(late * MICROS)
    );
    assert_eq!(
        first(pe, early * MICROS),
        Some(early * MICROS),
        "at a bar's stamp"
    );
    assert_eq!(first(pe, (late + 1) * MICROS), None, "after the last bar");
    assert_eq!(
        first(pe, late * MICROS + 1),
        None,
        "one microsecond after it"
    );
    assert_eq!(
        first(pe, ist(2018, 12, 3, 9, 15, 0) * MICROS),
        None,
        "past the expiry"
    );
    assert_eq!(first(pe, i64::MIN), None, "an instant no calendar places");
    assert_eq!(
        first("2018-11-29-390000-PE", early * MICROS),
        None,
        "no file"
    );
    // A MONTH THAT IS NOT A BAR FILE, before a readable one.
    let ce = "2018-11-29-410000-CE";
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "ZEDCO",
        contract: Some(Contract::parse(ce).expect("a contract")),
        timeframe: Timeframe::SECOND_1,
        month: month(2018, 10),
        file: FileKind::Bars,
    })
    .expect("a path")
    .to_path_buf(&root);
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("mkdir");
    std::fs::write(&path, b"not a bar file").expect("written");
    write_option(&root, "ZEDCO", ce, month(2018, 11), &[(late, 2_006)]);
    assert_eq!(first(ce, ist(2018, 10, 1, 9, 15, 0) * MICROS), None);
    assert_eq!(faults.len(), 1, "{faults:?}");
    assert!(
        faults[0].contains(&path.display().to_string()),
        "the fault names the file: {faults:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// **THE CHAIN SAYS ITS LEDGER AS IT WAS READ, READS A LISTED PUT FROM ITS
/// OWN MONTHS, AND OPENS A SPOT MONTH ONCE FOR EVERY INSTANT IN IT**
/// (BASE-COV-API repair 3, D-8661). `/chain.json` through `answer`, then
/// `render_instant` driven directly. Extremes: a ledger whose every line
/// reads (`read`, no unread line); a ledger that is not UTF-8 (`unreadable`,
/// its reason said, every instant still answered); a stitched record listing
/// a put first stored before its ex-date (its first stamp read from its own
/// month file, after the ex-date); the ledger's records and unread lines
/// counted (one, none); two instants in one spot month (both answered off
/// one open); the second instant, with no bar a minute later, said
/// `no_spot_bar`; a ladder position past the strikes it indexes
/// (no row written for it); a ladder over no strikes at all (an empty list);
/// and, at once, the listed put and the two instants in one answer, and the
/// unreadable ledger with two instants; out of the box, a ledger of one
/// byte that is not UTF-8.
#[test]
fn the_chain_says_its_ledger_reads_a_listed_put_and_opens_a_spot_month_once() {
    let built = fixture("chain-ledger-states", &census_rows());
    let ledger = pull::gdfl_lineage::lineage_path(&built.store_root);
    std::fs::create_dir_all(ledger.parent().expect("imports/")).expect("mkdir");
    let nifty = Symbol::new("NIFTY").expect("a symbol");
    let ex = Day::new(2018, 9, 17).expect("a day");
    // THE 4,100 PE, first stored 2018-09-10, listed again from the ex-date:
    // its own month file holds its first bar after it.
    let listed = StitchedRecord {
        underlying: nifty,
        key: Contract::parse("2019-03-28-410000-PE").expect("a contract"),
        ex_date: ex,
        before: None,
        after: None,
    }
    .line();
    let relisted = ist(2018, 9, 18, 9, 30, 0);
    write_option(
        &built.store_root,
        "NIFTY",
        "2019-03-28-410000-PE",
        month(2018, 9),
        &[(ist(2018, 9, 10, 10, 0, 0), 1_005), (relisted, 1_015)],
    );
    std::fs::write(&ledger, &listed).expect("a ledger");
    let two = format!("{BASE}&expiry=2019-03-28&at={},{}", t1(), t1() + 60);
    let (status, body) = ask(&built, &two);
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["lineage"]["state"], "read", "{body}");
    assert_eq!(body["lineage"]["records"], 1);
    assert_eq!(body["lineage"]["unread_lines"], 0);
    let chains = body["chains"].as_array().expect("chains");
    assert_eq!(chains.len(), 2, "both instants of the one month: {body}");
    assert_eq!(chains[0]["spot"], SPOT, "{body}");
    assert_eq!(
        chains[1]["spot_why"], "no_spot_bar",
        "no bar a minute later, read off the same open: {body}"
    );
    for chain in chains {
        let put = chain["strikes"]
            .as_array()
            .expect("strikes")
            .iter()
            .find(|strike| strike["strike"] == 410_000)
            .expect("the 4,100 strike");
        assert_eq!(
            put["pe_first"], relisted,
            "the listed put's first stamp is its own month's, after the ex-date: {chain}"
        );
    }

    // A LEDGER THAT IS NOT UTF-8: said, and every instant still answered.
    std::fs::write(&ledger, [0xFF]).expect("a ledger");
    let (status, body) = ask(&built, &two);
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["lineage"]["state"], "unreadable", "{body}");
    assert!(
        body["lineage"]["error"]
            .as_str()
            .is_some_and(|why| why.contains("not UTF-8")),
        "{body}"
    );
    assert_eq!(body["chains"].as_array().expect("chains").len(), 2);

    // A LADDER POSITION PAST ITS STRIKES writes no row; no strikes, none.
    let asked = Asked::parse(&format!("{BASE}&expiry=2019-03-28&at={}", t1())).expect("asked");
    for (stored, rows) in [(vec![0, 5], 1), (vec![], 0)] {
        let ladder = Ladder {
            stored,
            atm: None,
            step_local: None,
            refusal: Some(("spot_unreadable", "a test's ladder".to_owned())),
        };
        let strikes = [Strike {
            paisa: 400_000,
            call: Side::default(),
            put: Side::default(),
        }];
        let mut out = String::new();
        render_instant(&mut out, &asked, &strikes, &ladder, t1() * MICROS, None);
        let rendered: Value = serde_json::from_str(&out).expect("an instant");
        assert_eq!(
            rendered["strikes"].as_array().expect("strikes").len(),
            rows,
            "{out}"
        );
    }
}

/// Changes the last byte of record `index` of the month file at `path`: a
/// byte under the checksum block that covers that record, so a month whose
/// damage lies outside its tail block still opens, and the block will not
/// read.
fn damage(path: &std::path::Path, index: u64) {
    use std::os::unix::fs::FileExt as _;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .expect("the month file");
    let at = store::format::HEADER_LEN
        + index * store::format::RECORD_STRIDE
        + (store::format::RECORD_STRIDE - 1);
    let mut byte = [0_u8; 1];
    file.read_exact_at(&mut byte, at)
        .expect("the record's last byte");
    file.write_all_at(&[!byte[0]], at)
        .expect("the byte changed");
}

/// The 1s month file of `contract` (an option of `symbol`, or the spot when
/// `None`) of `at` under `root`.
fn month_file(
    root: &std::path::Path,
    segment: &str,
    symbol: &str,
    contract: Option<&str>,
    at: YearMonth,
) -> std::path::PathBuf {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: contract.map(|text| Contract::parse(text).expect("a contract")),
        timeframe: Timeframe::SECOND_1,
        month: at,
        file: FileKind::Bars,
    })
    .expect("a path")
    .to_path_buf(root)
}

/// **A MONTH THAT OPENS AND THEN WILL NOT READ IS A NAMED FAULT, FOR AN
/// OPTION'S FIRST STAMP AND FOR THE SPOT** (BASE-COV-API repair 3, D-8661).
/// Month files written here, then one byte of a record past their tail block
/// changed under its checksum: the month opens and the damaged block refuses
/// every read that meets it. `first_stored_from` driven directly, the spot
/// through `/chain.json`. Extremes: an option month's first stamp asked from
/// before its first bar (the search meets the damage: none, the fault naming
/// the file); one asked from inside its tail block, past the damage (that
/// bar); one asked at its last bar's stamp (that bar); one asked after its
/// last bar (none, no fault: the damage was never read); a spot instant whose
/// read meets the damaged block (`spot_unreadable`, the fault naming the file
/// and the instant, `206`); a spot instant past the damage (priced, `200`);
/// September's undamaged spot month beside it (priced); a month whose only
/// block, its tail, is the damaged one (refused, the fault named, nothing
/// served); and, at once, one request asking September's instant, the
/// damaged one and the one past it (priced, unreadable, priced, `206`), and a
/// first stamp asked from October, a month with no file, over the damaged
/// November (the absent month passed over, the damage named); out of the box,
/// the changed byte is the record's open interest, a field no answer here
/// reads (refused all the same: the checksum covers every byte).
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "an option month and a spot month, each asked on both sides of its damage"
)]
fn a_month_that_opens_and_will_not_read_is_a_named_fault_for_a_first_stamp_and_for_the_spot() {
    // AN OPTION MONTH OF 150 BARS: BLOCKS 0 AND 1 ARE WHOLE, 146-149 ITS TAIL.
    let root = crate::scratch::path("chain-damaged-block");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    let pe = "2018-11-29-400000-PE";
    let open = ist(2018, 11, 5, 9, 15, 0);
    let bars: Vec<(i64, i64)> = (0..150).map(|n| (open + n, 1_000 + n)).collect();
    write_option(&root, "ZEDCO", pe, month(2018, 11), &bars);
    let path = month_file(&root, "FNO", "ZEDCO", Some(pe), month(2018, 11));
    damage(&path, 0);
    let key = |text: &str| OptionKey {
        root: &root,
        feed: Vendor::Gdfl,
        exchange: "NSE",
        underlying: "ZEDCO",
        contract: Contract::parse(text).expect("a contract"),
        timeframe: Timeframe::SECOND_1,
    };
    let expiry = Expiry::new(2018, 11, 29).expect("an expiry");
    // READ AS AN ARRAY, never a lazy `&&` whose second side only a failing
    // assertion reaches (D-8661 item 8).
    let named = |faults: &[String], file: &std::path::Path| {
        let prefix = format!("{}: ", file.display());
        [
            faults.len() == 1,
            faults.iter().all(|fault| fault.starts_with(&prefix)),
        ] == [true, true]
    };
    let mut faults = Vec::new();
    assert_eq!(
        first_stored_from(&key(pe), (open - 3_600) * MICROS, expiry, &mut faults),
        None,
        "the damage is met"
    );
    assert!(named(&faults, &path), "{faults:?}");
    let mut clean = Vec::new();
    for (from, answered) in [
        (open + 146, Some(open + 146)),
        (open + 149, Some(open + 149)),
        (open + 150, None),
    ] {
        assert_eq!(
            first_stored_from(&key(pe), from * MICROS, expiry, &mut clean),
            answered.map(|secs| secs * MICROS),
            "from {from}"
        );
    }
    assert!(clean.is_empty(), "the damage was never read: {clean:?}");
    let mut walked = Vec::new();
    assert_eq!(
        first_stored_from(
            &key(pe),
            ist(2018, 10, 15, 9, 15, 0) * MICROS,
            expiry,
            &mut walked
        ),
        None,
        "October has no file; November's damage is met"
    );
    assert!(named(&walked, &path), "{walked:?}");

    // A MONTH OF ONE BLOCK, THE TAIL, DAMAGED.
    let ce = "2018-11-29-410000-CE";
    write_option(
        &root,
        "ZEDCO",
        ce,
        month(2018, 11),
        &[(open, 1_000), (open + 1, 1_001)],
    );
    damage(
        &month_file(&root, "FNO", "ZEDCO", Some(ce), month(2018, 11)),
        0,
    );
    let mut tail = Vec::new();
    assert_eq!(
        first_stored_from(&key(ce), (open - 3_600) * MICROS, expiry, &mut tail),
        None,
        "nothing is served from a damaged tail"
    );
    assert_eq!(tail.len(), 1, "the damage is named: {tail:?}");

    // THE SPOT: NOVEMBER'S MONTH OF 160 BARS, ITS FIRST BLOCK DAMAGED.
    let built = fixture("chain-damaged-spot", &census_rows());
    let start = ist(2018, 11, 6, 10, 0, 0);
    let spot: Vec<(i64, i64)> = (0..160).map(|n| (start + n, SPOT)).collect();
    write_spot(&built.store_root, "INDEX", "NIFTY", month(2018, 11), &spot);
    let spot_path = month_file(&built.store_root, "INDEX", "NIFTY", None, month(2018, 11));
    damage(&spot_path, 0);
    let (in_damage, past) = (start + 10, start + 150);
    let (status, body) = ask(&built, &format!("{BASE}&expiry=2019-03-28&at={in_damage}"));
    assert_eq!(status, axum::http::StatusCode::PARTIAL_CONTENT, "{body}");
    assert_eq!(body["chains"][0]["spot_why"], "spot_unreadable", "{body}");
    let said = body["faults"].as_str().expect("the faults");
    assert!(
        said.contains(&format!("{} t={in_damage}: ", spot_path.display())),
        "{said}"
    );
    let (status, body) = ask(&built, &format!("{BASE}&expiry=2019-03-28&at={past}"));
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert_eq!(body["chains"][0]["spot"], SPOT, "{body}");
    let (status, body) = ask(
        &built,
        &format!("{BASE}&expiry=2019-03-28&at={},{in_damage},{past}", t1()),
    );
    assert_eq!(status, axum::http::StatusCode::PARTIAL_CONTENT, "{body}");
    assert_eq!(body["chains"][0]["spot"], SPOT, "September: {body}");
    assert_eq!(body["chains"][1]["spot_why"], "spot_unreadable", "{body}");
    assert_eq!(body["chains"][2]["spot"], SPOT, "past the damage: {body}");
    let _ = std::fs::remove_dir_all(&root);
}
