//! `facets.rs`'s tests: the index, its moves, the route's answers.

use super::*;
use crate::server::tests::{agreeing, site};
use brutex_core::instrument::Contract;
use pull::manifest::{Closes, Entry, Manifest, manifest_path};

/// 2019-03-28 09:15:00 IST, in microseconds: a stamp inside the month.
const MARCH: i64 = 1_553_744_700_000_000;

fn month(y: u16, m: u8) -> YearMonth {
    YearMonth::new(y, m).expect("a month")
}

/// One FNO key of NIFTY: `contract` in the store's spelling, at `tf`.
fn fno(contract: &str, tf: Timeframe, at: YearMonth) -> EntryKey {
    EntryKey {
        contract: Some(Contract::parse(contract).expect("a contract")),
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: Symbol::new("NIFTY").expect("a symbol"),
        timeframe: tf,
        month: at,
    }
}

/// One spot key.
fn spot(segment: Segment, symbol: &str, tf: Timeframe, at: YearMonth) -> EntryKey {
    EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment,
        symbol: Symbol::new(symbol).expect("a symbol"),
        timeframe: tf,
        month: at,
    }
}

/// A row: `rows` bars from `first` to `last` seconds after [`MARCH`], closing
/// at `close` paisa (an invented price).
fn held(key: EntryKey, rows: u64, first: i64, last: i64, close: i64) -> Held {
    Held::new(
        Entry {
            key,
            rows,
            first_ts_micros: MARCH + first * 1_000_000,
            last_ts_micros: MARCH + last * 1_000_000,
        },
        Closes::known(close, close).expect("a close"),
    )
}

/// The fixture chain: NIFTY 2019-03-28 with two strikes, a CE and a PE at
/// one, a future, and a weekly 2019-03-07 with one strike; plus spot NIFTY
/// at two rungs.
fn chain() -> Vec<Held> {
    let march = month(2019, 3);
    vec![
        held(
            fno("2019-03-28-1150000-CE", Timeframe::SECOND_1, march),
            10,
            0,
            100,
            7_001,
        ),
        held(
            fno("2019-03-28-1150000-PE", Timeframe::SECOND_1, march),
            20,
            5,
            200,
            3_002,
        ),
        held(
            fno("2019-03-28-1160000-CE", Timeframe::SECOND_1, march),
            30,
            7,
            300,
            2_003,
        ),
        held(
            fno("2019-03-28-FUT", Timeframe::SECOND_1, march),
            40,
            1,
            400,
            1_150_004,
        ),
        held(
            fno("2019-03-07-1100000-PE", Timeframe::SECOND_1, march),
            50,
            2,
            150,
            1_005,
        ),
        held(
            spot(Segment::Index, "NIFTY", Timeframe::MINUTE_1, march),
            60,
            0,
            500,
            1_150_006,
        ),
        held(
            spot(Segment::Index, "NIFTY", Timeframe::DAY_1, march),
            3,
            0,
            500,
            1_150_006,
        ),
    ]
}

/// A slot over an in-memory single-file census holding `rows`.
fn slot_of(rows: &[Held]) -> Arc<Slot> {
    let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    for row in rows {
        let _append = manifest.record_held(*row).expect("records");
    }
    Slot::detached(census::VendorCensus {
        vendor: Vendor::Gdfl,
        path: std::path::PathBuf::from("/nonexistent/facets/gdfl.man"),
        state: census::Census::Held {
            manifest: census::Manifests::single(manifest),
        },
    })
}

fn built(segment: Segment, slot: &Arc<Slot>) -> Index {
    let mut index = Index::default();
    advance(&mut index, segment, slot);
    index
}

fn asked_for(query: &str) -> Asked {
    asked(query).expect("a well-formed query")
}

fn json(text: &str) -> serde_json::Value {
    serde_json::from_str(text).expect("JSON")
}

#[test]
fn a_contract_is_read_from_the_stores_own_spelling_and_nothing_else_is_guessed() {
    let expiry = Day::new(2019, 3, 28).expect("a day");
    assert_eq!(
        contract_of("2019-03-28-FUT"),
        Some((expiry, ContractId::Future))
    );
    assert_eq!(
        contract_of("2019-03-28-1150000-CE"),
        Some((
            expiry,
            ContractId::Option {
                strike: 1_150_000,
                call: true
            }
        ))
    );
    assert_eq!(
        contract_of("2019-03-28-1150000-PE"),
        Some((
            expiry,
            ContractId::Option {
                strike: 1_150_000,
                call: false
            }
        ))
    );
    for refused in [
        "III",
        "2019-03-28",
        "2019-02-30-FUT",
        "2019-03-28-1150000-XX",
        "2019-03-28-X-CE",
    ] {
        assert_eq!(contract_of(refused), None, "{refused}");
    }
}

#[test]
fn the_fno_selection_answers_underlyings_then_expiries_then_strikes() {
    let slot = slot_of(&chain());
    let index = built(Segment::Fno, &slot);

    // NO UNDERLYING: the underlyings, each with its counts, and nothing else.
    let top = json(
        &render_answer(&index, &asked_for("feed=gdfl&segment=FNO"), "held", None).expect("answers"),
    );
    assert_eq!(top["underlyings"].as_array().expect("a list").len(), 1);
    let nifty = &top["underlyings"][0];
    assert_eq!(nifty["symbol"], "NIFTY");
    assert_eq!(nifty["expiries"], 2);
    assert_eq!(nifty["contracts"], 5);
    assert_eq!(nifty["rows"]["1s"], 150, "10+20+30+40+50");
    assert!(
        nifty.get("last_close").is_none(),
        "a chain has no one close"
    );
    assert_eq!(top["expiries"], serde_json::json!([]));
    assert_eq!(top["strikes"], serde_json::json!([]));

    // THE UNDERLYING: its months and its expiries, each with weekday and span.
    let under = json(
        &render_answer(
            &index,
            &asked_for("feed=gdfl&segment=FNO&underlying=NIFTY"),
            "held",
            None,
        )
        .expect("answers"),
    );
    let expiries = under["expiries"].as_array().expect("a list");
    assert_eq!(expiries.len(), 2);
    assert_eq!(expiries[0]["expiry"], "2019-03-07");
    assert_eq!(expiries[0]["weekday"], "Thu");
    assert_eq!(expiries[1]["expiry"], "2019-03-28");
    assert_eq!(expiries[1]["weekday"], "Thu");
    assert_eq!(expiries[1]["contracts"], 4);
    assert_eq!(expiries[1]["strikes"], 2);
    assert_eq!(expiries[1]["future"], true);
    assert_eq!(expiries[1]["first_ts"], MARCH);
    assert_eq!(expiries[1]["last_ts"], MARCH + 400_000_000);
    assert_eq!(under["months"][0]["month"], "2019-03");
    assert_eq!(under["months"][0]["rows"]["1s"], 150);

    // THE EXPIRY: each strike with CE and PE side by side, and the future.
    let strikes = json(
        &render_answer(
            &index,
            &asked_for("feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28"),
            "held",
            None,
        )
        .expect("answers"),
    );
    let rows = strikes["strikes"].as_array().expect("a list");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["strike"], 1_150_000);
    assert_eq!(rows[0]["ce"]["rows"]["1s"], 10);
    assert_eq!(rows[0]["ce"]["last_close"], 7_001);
    assert_eq!(rows[0]["pe"]["rows"]["1s"], 20);
    assert_eq!(rows[1]["strike"], 1_160_000);
    assert_eq!(
        rows[1]["pe"],
        serde_json::Value::Null,
        "no PE at this strike"
    );
    assert_eq!(strikes["futures"][0]["contract"], "2019-03-28-FUT");
    assert_eq!(strikes["futures"][0]["rows"]["1s"], 40);
    assert_eq!(
        strikes["months"][0]["rows"]["1s"], 100,
        "this chain's March only"
    );
}

#[test]
fn a_spot_underlying_carries_rows_per_rung_and_its_newest_close() {
    let slot = slot_of(&chain());
    let index = built(Segment::Index, &slot);
    let answer = json(
        &render_answer(
            &index,
            &asked_for("feed=gdfl&segment=INDEX&underlying=NIFTY"),
            "held",
            None,
        )
        .expect("answers"),
    );
    let nifty = &answer["underlyings"][0];
    assert_eq!(nifty["rows"]["1min"], 60);
    assert_eq!(nifty["rows"]["1day"], 3);
    assert_eq!(nifty["last_close"], 1_150_006);
    assert_eq!(nifty["expiries"], 0);
    assert_eq!(answer["months"][0]["rows"]["1min"], 60);
    assert_eq!(answer["months"][0]["last_close"], 1_150_006);
    assert!(
        index
            .underlyings()
            .all(|((_, symbol), _)| symbol.as_str() == "NIFTY"),
        "the INDEX index holds no FNO key"
    );
}

#[test]
fn a_query_that_cannot_be_read_is_refused_by_name() {
    for (query, why) in [
        ("segment=FNO", "feed"),
        ("feed=gdfl", "segment"),
        ("feed=gdfl&segment=SPOT", "segment"),
        (
            "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-13-01",
            "expiry",
        ),
        (
            "feed=gdfl&segment=INDEX&underlying=NIFTY&expiry=2019-03-28",
            "expiry",
        ),
        ("feed=gdfl&segment=FNO&expiry=2019-03-28", "expiry"),
        ("feed=gdfl&segment=FNO&since=soon", "since"),
        ("feed=gdfl&segment=FNO&exchange=MCX", "exchange"),
    ] {
        let (code, body) = asked(query).expect_err(query);
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{query}");
        assert!(body.contains(why), "{query}: {body}");
    }
}

/// A census written to disk, appended to in place the way the writer
/// appends, read through `read_vendor_after` as the per-vendor cache reads it.
struct OnDisk {
    root: std::path::PathBuf,
    manifest: Manifest,
}

impl OnDisk {
    fn new(name: &str, rows: &[Held]) -> Self {
        let root = crate::scratch::path(&format!("facets-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("manifest")).expect("mkdir");
        let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
        for row in rows {
            let _append = manifest.record_held(*row).expect("records");
        }
        std::fs::write(manifest_path(&root, Vendor::Gdfl), manifest.image()).expect("written");
        Self { root, manifest }
    }

    /// Appends `rows` as the writer does: each entry at its offset, then the
    /// header slot, positionally, into the same file.
    fn append(&mut self, rows: &[Held]) {
        use std::os::unix::fs::FileExt as _;
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(manifest_path(&self.root, Vendor::Gdfl))
            .expect("open");
        for row in rows {
            let append = self.manifest.record_held(*row).expect("records");
            file.write_all_at(&append.bytes, append.offset)
                .expect("entry");
            file.write_all_at(&append.commit.bytes, append.commit.offset)
                .expect("slot");
        }
    }

    fn read(&self, previous: Option<&census::VendorCensus>) -> Arc<Slot> {
        Slot::detached(census::read_vendor_after(
            &self.root,
            Vendor::Gdfl,
            previous,
        ))
    }
}

#[test]
fn a_moved_index_equals_a_fresh_build() {
    let rows = chain();
    let (first, rest) = rows.split_at(3);
    let mut disk = OnDisk::new("moved", first);
    let before = disk.read(None);
    let mut index = built(Segment::Fno, &before);

    // MORE ROWS LAND, AND ONE ROW GROWS: the 1150000 CE gains bars.
    let march = month(2019, 3);
    let mut appended: Vec<Held> = rest.to_vec();
    appended.push(held(
        fno("2019-03-28-1150000-CE", Timeframe::SECOND_1, march),
        15,
        0,
        120,
        7_101,
    ));
    disk.append(&appended);
    let after = disk.read(Some(before.census()));
    assert!(
        census::manifest_reads::tails_of(&manifest_path(&disk.root, Vendor::Gdfl))
            .contains(&u64::try_from(appended.len()).expect("small")),
        "the census was extended by its tail, not read whole"
    );

    advance(&mut index, Segment::Fno, &after);
    let fresh = built(Segment::Fno, &after);
    for query in [
        "feed=gdfl&segment=FNO",
        "feed=gdfl&segment=FNO&underlying=NIFTY",
        "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28",
        "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-07",
    ] {
        assert_eq!(
            render_answer(&index, &asked_for(query), "held", None),
            render_answer(&fresh, &asked_for(query), "held", None),
            "{query}: a moved index answers what a fresh build answers"
        );
    }
    let strikes = json(
        &render_answer(
            &index,
            &asked_for("feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28"),
            "held",
            None,
        )
        .expect("answers"),
    );
    assert_eq!(
        strikes["strikes"][0]["ce"]["rows"]["1s"], 15,
        "grown, not added twice"
    );
    assert_eq!(strikes["strikes"][0]["ce"]["last_close"], 7_101);
}

#[test]
fn a_delta_answers_only_what_changed_and_an_old_generation_is_refused() {
    let rows = chain();
    let (first, rest) = rows.split_at(4);
    let mut disk = OnDisk::new("since", first);
    let before = disk.read(None);
    let generation = built(Segment::Fno, &before).generation;
    let weekly = rest.first().copied().expect("the weekly row");
    disk.append(&[weekly]);
    let after = disk.read(Some(before.census()));
    let index = built(Segment::Fno, &after);

    // ALREADY CURRENT: nothing changed after the newest generation.
    let current = json(
        &render_answer(
            &index,
            &asked_for(&format!(
                "feed=gdfl&segment=FNO&underlying=NIFTY&since={}",
                index.generation
            )),
            "held",
            None,
        )
        .expect("answers"),
    );
    assert_eq!(current["underlyings"], serde_json::json!([]));

    // THE DELTA AFTER THE OLDER GENERATION: the weekly expiry only.
    let delta = json(
        &render_answer(
            &index,
            &asked_for(&format!(
                "feed=gdfl&segment=FNO&underlying=NIFTY&since={generation}"
            )),
            "held",
            None,
        )
        .expect("answers"),
    );
    let expiries = delta["expiries"].as_array().expect("a list");
    assert_eq!(expiries.len(), 1, "{delta}");
    assert_eq!(expiries[0]["expiry"], "2019-03-07");
    assert_eq!(delta["since"], generation);
    assert_eq!(
        delta["underlyings"][0]["contracts"], 5,
        "with its current totals"
    );

    // A GENERATION BEFORE THE CENSUS WAS LAST READ WHOLE IS NOT HELD.
    let (code, body) = render_answer(
        &index,
        &asked_for(&format!(
            "feed=gdfl&segment=FNO&since={}",
            generation.checked_sub(1).expect("a generation")
        )),
        "held",
        None,
    )
    .expect_err("refused");
    assert_eq!(code, axum::http::StatusCode::GONE);
    assert!(body.contains("since_too_old"), "{body}");
    let (code, _) = render_answer(
        &index,
        &asked_for(&format!(
            "feed=gdfl&segment=FNO&since={}",
            index.generation + 1
        )),
        "held",
        None,
    )
    .expect_err("a generation never published is refused too");
    assert_eq!(code, axum::http::StatusCode::GONE);
}

#[test]
fn the_route_answers_from_the_site_and_refuses_an_unreadable_census() {
    let dir = agreeing("facets-route");
    let built_site = site("facets-route", &dir);
    let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    for row in chain() {
        let _append = manifest.record_held(row).expect("records");
    }
    std::fs::write(
        manifest_path(&built_site.store_root, Vendor::Gdfl),
        manifest.image(),
    )
    .expect("written");
    let (code, headers, body) = answer(
        &built_site,
        "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28",
    );
    assert_eq!(code, axum::http::StatusCode::OK, "{body}");
    assert_eq!(headers["x-brutex-census-state"], "held");
    let value = json(&body);
    assert_eq!(value["strikes"].as_array().expect("a list").len(), 2);
    assert_eq!(value["generation"], 7);
    assert!(body.len() < 50_000);

    // An empty feed is answered with empty lists, its state named.
    let (code, headers, body) = answer(&built_site, "feed=zerodha&segment=INDEX");
    assert_eq!(code, axum::http::StatusCode::OK);
    assert_eq!(headers["x-brutex-census-state"], "absent");
    assert_eq!(json(&body)["underlyings"], serde_json::json!([]));

    // A census that will not load is a 503 naming it, never an empty list.
    std::fs::write(
        manifest_path(&built_site.store_root, Vendor::Zerodha),
        b"not a manifest",
    )
    .expect("written");
    let (code, headers, body) = answer(&built_site, "feed=zerodha&segment=INDEX");
    assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(headers["x-brutex-census-state"], "unreadable");
    assert!(body.contains("census_unreadable"), "{body}");
}

/// **A CENSUS RECOVERED FROM AN OLDER GENERATION SAYS SO IN THE BODY, NOT
/// ONLY IN A HEADER** (D-3327; `CLAUDE.md` §4: a fallback never hides a
/// failure).
///
/// One header slot damaged, the other a valid generation 1 counting nothing:
/// the census loads from the survivor and every list is empty because the
/// committed months are past it. A reader of the lists alone must be able to
/// tell that from an empty store, so the body carries `degraded` with the
/// refusal, and the header says it too.
#[test]
fn a_degraded_census_says_so_in_the_body() {
    let dir = agreeing("facets-degraded");
    let built_site = site("facets-degraded", &dir);
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
    std::fs::write(manifest_path(&built_site.store_root, Vendor::Gdfl), &bytes).expect("written");

    let (code, headers, body) = answer(&built_site, "feed=gdfl&segment=FNO");
    assert_eq!(code, axum::http::StatusCode::OK, "{body}");
    let value = json(&body);
    let why = value["degraded"]
        .as_str()
        .expect("the body names what was stepped over");
    assert!(!why.is_empty(), "{body}");
    assert!(
        !headers["x-brutex-census-degraded"].is_empty(),
        "and the header says it too"
    );
    assert_eq!(value["underlyings"], serde_json::json!([]));

    // A CLEAN CENSUS SAYS `null`, never an empty string.
    let clean = render_answer(
        &Index::default(),
        &asked_for("feed=gdfl&segment=FNO"),
        "held",
        None,
    )
    .expect("answers");
    assert_eq!(json(&clean)["degraded"], serde_json::Value::Null);
}

#[test]
fn the_weekday_of_a_day_is_counted_from_a_thursday() {
    for (y, m, d, want) in [
        (1970, 1, 1, "Thu"),
        (2019, 3, 28, "Thu"),
        (2020, 9, 15, "Tue"),
        (2026, 10, 4, "Sun"),
    ] {
        assert_eq!(
            weekday(Day::new(y, m, d).expect("a day")),
            want,
            "{y}-{m}-{d}"
        );
    }
}
