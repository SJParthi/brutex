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

    // A GENERATION BEFORE THE CENSUS WAS LAST READ WHOLE IS READ BACK FROM
    // THE FILE (R3-09, RA-008): the future the base's last entry named, and
    // the weekly appended after it.
    let older = generation.checked_sub(1).expect("a generation");
    let read_back = json(
        &render_answer(
            &index,
            &asked_for(&format!(
                "feed=gdfl&segment=FNO&underlying=NIFTY&since={older}"
            )),
            "held",
            None,
        )
        .expect("read back"),
    );
    let expiries = read_back["expiries"].as_array().expect("a list");
    assert_eq!(expiries.len(), 2, "{read_back}");
    assert_eq!(read_back["since"], older);

    // AND REFUSED, NEVER GUESSED, ONCE THE FILE IS NOT THE ONE THE BASE WAS
    // READ FROM: the same bytes installed by rename are another inode.
    let path = manifest_path(&disk.root, Vendor::Gdfl);
    let bytes = std::fs::read(&path).expect("read");
    let tmp = path.with_extension("man.writing");
    std::fs::write(&tmp, &bytes).expect("written");
    std::fs::rename(&tmp, &path).expect("renamed");
    let (code, body) = render_answer(
        &index,
        &asked_for(&format!("feed=gdfl&segment=FNO&since={older}")),
        "held",
        None,
    )
    .expect_err("refused");
    assert_eq!(code, axum::http::StatusCode::GONE);
    assert!(body.contains("since_too_old"), "{body}");
    // A GENERATION NEVER PUBLISHED IS NOT "TOO OLD" (WP-56; DB-B56): a
    // client that sent the vendor-wide sum, or a value past this part's
    // newest, is told so and given the generation to use, never sent to
    // reload in a loop believing it fell behind.
    for future in [index.generation + 1, 99_999_999_999] {
        let (code, body) = render_answer(
            &index,
            &asked_for(&format!("feed=gdfl&segment=FNO&since={future}")),
            "held",
            None,
        )
        .expect_err("a generation never published is refused");
        assert_eq!(code, axum::http::StatusCode::CONFLICT, "{body}");
        let refusal = json(&body);
        assert_eq!(refusal["error"], "since_in_future", "{body}");
        assert_eq!(refusal["since"], future);
        assert_eq!(refusal["generation"], index.generation);
    }
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
    // ONE SPELLING OF THE GENERATION A `since` TAKES, in the header as in the
    // body (WP-56; DB-B56).
    assert_eq!(headers["x-brutex-census-part-generation"], "7");
    assert_eq!(
        headers["x-brutex-census-part-generations"], "",
        "a single file"
    );
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

/// **THE ORDER AN ANSWER STATES IS THE TYPES' OWN, AND NAMES EVERY VARIANT**
/// (WP-56b). A page orders a series by this statement and keeps no list of
/// its own, so a variant added to either type and not to the list would be a
/// series no page can place. The two `match`es do not compile with a variant
/// unnamed; the sort check holds the list to `Ord`, the census's order.
#[test]
fn the_order_stated_is_the_types_own_and_names_every_variant() {
    let exchange = |exchange: Exchange| match exchange {
        Exchange::Nse => 0_usize,
        Exchange::Bse => 1,
    };
    let segment = |segment: Segment| match segment {
        Segment::Index => 0_usize,
        Segment::Cash => 1,
        Segment::Fno => 2,
    };
    assert_eq!(EXCHANGES.map(exchange), [0, 1]);
    assert_eq!(SEGMENTS.map(segment), [0, 1, 2]);
    assert!(EXCHANGES.is_sorted() && SEGMENTS.is_sorted());
    assert_eq!(
        json(&order_json()),
        serde_json::json!({"exchanges": ["NSE", "BSE"], "segments": ["INDEX", "CASH", "FNO"]})
    );
    // Every selection's answer carries it.
    let slot = slot_of(&chain());
    let answer = json(
        &render_answer(
            &built(Segment::Fno, &slot),
            &asked_for("feed=gdfl&segment=FNO"),
            "held",
            None,
        )
        .expect("answers"),
    );
    assert_eq!(answer["order"], json(&order_json()));
}

/// **THE SEGMENT'S TOTAL IS THE SUM OF ITS UNDERLYINGS, BUILT OR MOVED**
/// (WP-56b): the root answer reads this one node per segment.
#[test]
fn a_segments_total_equals_the_sum_of_its_underlyings() {
    let mut rows = chain();
    rows.push(held(
        spot(
            Segment::Index,
            "BANKNIFTY",
            Timeframe::MINUTE_1,
            month(2019, 3),
        ),
        7,
        0,
        360,
        2_700_001,
    ));
    for segment in SEGMENTS {
        let index = built(segment, &slot_of(&rows));
        let mut summed = Rows::new();
        for (_, under) in index.underlyings() {
            for (rung, n) in &under.agg.rows {
                *summed.entry(*rung).or_insert(0) += n;
            }
        }
        assert_eq!(index.total.rows, summed, "{segment:?}");
        assert_eq!(
            index.total.first,
            index.underlyings().filter_map(|(_, u)| u.agg.first).min(),
            "{segment:?}"
        );
        assert_eq!(
            index.total.last,
            index.underlyings().filter_map(|(_, u)| u.agg.last).max(),
            "{segment:?}"
        );
    }
    // Moved forward by one more month of one key, the total moves by it.
    let mut disk = OnDisk::new("facets-total", &rows);
    let first = disk.read(None);
    let mut index = built(Segment::Index, &first);
    let before = index.total.rows.clone();
    disk.append(&[held(
        spot(
            Segment::Index,
            "BANKNIFTY",
            Timeframe::MINUTE_1,
            month(2019, 4),
        ),
        5,
        3_000_000,
        3_000_300,
        2_700_002,
    )]);
    let second = disk.read(Some(first.census()));
    advance(&mut index, Segment::Index, &second);
    assert_eq!(
        index.total.rows[&Timeframe::MINUTE_1],
        before[&Timeframe::MINUTE_1] + 5
    );
    assert_eq!(index.total.rows, built(Segment::Index, &second).total.rows);
}

/// **THE ROOT: `feed=` ALONE ANSWERS THE SEGMENTS HELD, IN THE CENSUS'S
/// ORDER, AND THE ORDER ITSELF** (WP-56b; DB-060, DB-G70). The first rung of
/// the selection tree, from one node per segment: no whole census, and no
/// segment list in the client.
#[test]
fn the_root_answers_the_segments_a_feed_holds_and_the_order() {
    let dir = agreeing("facets-root");
    let built_site = site("facets-root", &dir);
    let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    for row in chain() {
        let _append = manifest.record_held(row).expect("records");
    }
    std::fs::write(
        manifest_path(&built_site.store_root, Vendor::Gdfl),
        manifest.image(),
    )
    .expect("written");
    let (code, headers, body) = answer(&built_site, "feed=gdfl");
    assert_eq!(code, axum::http::StatusCode::OK, "{body}");
    assert_eq!(headers["x-brutex-census-state"], "held");
    let value = json(&body);
    assert_eq!(value["segment"], serde_json::Value::Null);
    assert_eq!(value["order"], json(&order_json()));
    // The fixture holds INDEX and FNO keys and no CASH key: CASH is not
    // listed, and the two held are in the order stated.
    let listed: Vec<&str> = value["segments"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|row| row["segment"].as_str().expect("a word"))
        .collect();
    assert_eq!(listed, ["INDEX", "FNO"]);
    let index_row = &value["segments"][0];
    assert_eq!(index_row["underlyings"], 1);
    assert_eq!(index_row["rows"]["1min"], 60);
    assert_eq!(index_row["rows"]["1day"], 3);
    assert_eq!(index_row["generation"], 7);
    assert!(index_row["first_ts"].as_i64() <= index_row["last_ts"].as_i64());
    // Each row equals what the segment's own answer sums to.
    let (_, _, fno_body) = answer(&built_site, "feed=gdfl&segment=FNO");
    let fno_rows: u64 = json(&fno_body)["underlyings"]
        .as_array()
        .expect("a list")
        .iter()
        .map(|under| under["rows"]["1s"].as_u64().unwrap_or(0))
        .sum();
    assert!(fno_rows > 0);
    assert_eq!(value["segments"][1]["rows"]["1s"], fno_rows);
    assert!(body.len() < 1_000, "{}", body.len());

    // A feed with no census: no segment, its state named; never a 400.
    let (code, headers, body) = answer(&built_site, "feed=zerodha");
    assert_eq!(code, axum::http::StatusCode::OK, "{body}");
    assert_eq!(headers["x-brutex-census-state"], "absent");
    assert_eq!(json(&body)["segments"], serde_json::json!([]));
    assert_eq!(json(&body)["state"], "absent");

    // Anything that narrows needs its segment, as before; an unknown feed is
    // still refused by name.
    for query in [
        "feed=gdfl&underlying=NIFTY",
        "feed=gdfl&since=1",
        "feed=nope",
        "",
    ] {
        let (code, _, body) = answer(&built_site, query);
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{query}: {body}");
    }

    // A census that will not load is a 503 naming it, never an empty root.
    std::fs::write(
        manifest_path(&built_site.store_root, Vendor::Zerodha),
        b"not a manifest",
    )
    .expect("written");
    let (code, _, body) = answer(&built_site, "feed=zerodha");
    assert_eq!(code, axum::http::StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert!(body.contains("census_unreadable"), "{body}");
}

/* ======================================================================
ONE INSTRUMENT'S CENSUS ROWS (WP-56b; DB-060, DB-G59, DB-G70, DB-A31)
====================================================================== */

/// A site whose GDFL census holds `rows`, written to its store root.
fn site_holding(name: &str, rows: &[Held]) -> crate::server::Site {
    let dir = agreeing(name);
    let built_site = site(name, &dir);
    let mut manifest = Manifest::open(Vendor::Gdfl, &[], &[]).expect("a genesis census");
    for row in rows {
        let _append = manifest.record_held(*row).expect("records");
    }
    std::fs::write(
        manifest_path(&built_site.store_root, Vendor::Gdfl),
        manifest.image(),
    )
    .expect("written");
    built_site
}

/// A tuples body expanded to one JSON row per census row: the instrument,
/// month and rung spelled out, then the seven fields after them.
fn expanded(wire: &serde_json::Value) -> Vec<serde_json::Value> {
    let word = |list: &str, at: &serde_json::Value| {
        wire[list][usize::try_from(at.as_u64().expect("an index")).expect("small")].clone()
    };
    wire["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|row| {
            let reason = |at: usize| {
                if row[at].is_null() {
                    serde_json::Value::Null
                } else {
                    word("reasons", &row[at])
                }
            };
            serde_json::json!([
                word("instruments", &row[0]),
                word("months", &row[1]),
                word("timeframes", &row[2]),
                row[3],
                row[4],
                row[5],
                row[6],
                reason(7),
                row[8],
                reason(9)
            ])
        })
        .collect()
}

/// The whole census of `site`'s GDFL store on the tuples wire, expanded.
fn whole_census(built_site: &crate::server::Site) -> Vec<serde_json::Value> {
    let (_stamps, now) = vendor_census(built_site, Vendor::Gdfl);
    let census = now.census().expect("a census").clone();
    let entries = census::held_entries(std::slice::from_ref(&census));
    let wire =
        crate::server::store_wire::compact(std::slice::from_ref(&census), &entries, Vendor::Gdfl)
            .expect("encodes");
    expanded(&json(&wire))
}

#[test]
fn one_instruments_rows_are_its_slice_of_the_whole_census_with_the_feeds_denominators() {
    // The denominators must count all three segments, not the one asked.
    let rows = three_segments();
    let built_site = site_holding("facets-rows", &rows);
    let whole = whole_census(&built_site);
    assert_eq!(whole.len(), rows.len());

    for (query, keeps) in [
        (
            "feed=gdfl&segment=INDEX&underlying=NIFTY&rows=census-tuples-v1",
            "NSE-INDEX-NIFTY",
        ),
        (
            "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28&rows=census-tuples-v1",
            "NSE-FNO-NIFTY-2019-03-28-",
        ),
        (
            "feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1",
            "NSE-FNO-NIFTY-",
        ),
        (
            "feed=gdfl&segment=CASH&underlying=ACC&rows=census-tuples-v1",
            "NSE-CASH-ACC",
        ),
    ] {
        let (code, headers, body) = answer(&built_site, query);
        assert_eq!(code, axum::http::StatusCode::OK, "{query}: {body}");
        let value = json(&body);
        let got = expanded(&value["census"]);
        let want: Vec<serde_json::Value> = whole
            .iter()
            .filter(|row| {
                let name = row[0].as_str().expect("a name");
                name == keeps || (keeps.ends_with('-') && name.starts_with(keeps))
            })
            .cloned()
            .collect();
        assert!(!want.is_empty(), "{query}: the fixture holds the selection");
        assert_eq!(got, want, "{query}: the rows, in the whole body's order");
        assert_eq!(value["census"]["feed"], "gdfl");
        assert_eq!(value["census"]["encoding"], "census-tuples-v1");
        assert_eq!(value["exchange"], "NSE");
        assert_eq!(value["cap"], ROWS_CAP);
        assert_eq!(value["held"]["months"], got.len());
        let bars: u64 = got.iter().map(|row| row[3].as_u64().expect("rows")).sum();
        assert_eq!(value["held"]["rows"], bars);
        assert_eq!(
            headers["x-brutex-census-part-generation"]
                .to_str()
                .expect("ascii"),
            value["generation"].to_string(),
            "{query}: the part generation, in the header and the body"
        );
        // EVERY (month, rung) the rows name has its denominators, and each
        // equals a recount over the whole census.
        let denominators = value["denominators"].as_array().expect("a list");
        let named: BTreeSet<(String, String)> = got
            .iter()
            .map(|row| {
                (
                    row[1].as_str().expect("m").to_owned(),
                    row[2].as_str().expect("t").to_owned(),
                )
            })
            .collect();
        assert_eq!(denominators.len(), named.len(), "{query}");
        for denominator in denominators {
            let at: Vec<&serde_json::Value> = whole
                .iter()
                .filter(|row| row[1] == denominator[0] && row[2] == denominator[1])
                .collect();
            let fullest = at.iter().map(|row| row[3].as_u64().expect("rows")).max();
            assert_eq!(denominator[2].as_u64(), fullest, "{query}: {denominator}");
            assert_eq!(denominator[3], at.len(), "{query}: {denominator}");
        }
    }
}

/// [`chain`] with a second index fuller than NIFTY at the same month and
/// rung, a cash share there too, and an earlier NIFTY month.
fn three_segments() -> Vec<Held> {
    let march = month(2019, 3);
    let mut rows = chain();
    rows.push(held(
        spot(Segment::Index, "BANKNIFTY", Timeframe::MINUTE_1, march),
        75,
        0,
        600,
        2_900_007,
    ));
    rows.push(held(
        spot(Segment::Cash, "ACC", Timeframe::MINUTE_1, march),
        70,
        0,
        600,
        150_008,
    ));
    rows.push(held(
        spot(Segment::Index, "NIFTY", Timeframe::MINUTE_1, month(2019, 2)),
        55,
        -2_600_000,
        -2_500_000,
        1_140_009,
    ));
    rows
}

#[test]
fn the_denominators_are_the_whole_feeds_and_an_instrument_not_held_answers_no_row() {
    let built_site = site_holding("facets-rows-denoms", &three_segments());
    // NIFTY's 1min March row is not the fullest of its month: BANKNIFTY's is,
    // and three keys are held there (two indices and a share).
    let (_, _, body) = answer(
        &built_site,
        "feed=gdfl&segment=INDEX&underlying=NIFTY&rows=census-tuples-v1",
    );
    let value = json(&body);
    let march_minute = value["denominators"]
        .as_array()
        .expect("a list")
        .iter()
        .find(|row| row[0] == "2019-03" && row[1] == "1min")
        .expect("named");
    assert_eq!(march_minute[2], 75);
    assert_eq!(march_minute[3], 3);

    // AN UNDERLYING THE FEED DOES NOT HOLD: no row and no exchange, said as
    // an answer, so a reader can tell it from a refusal.
    let (code, _, body) = answer(
        &built_site,
        "feed=gdfl&segment=INDEX&underlying=SENSEX&rows=census-tuples-v1",
    );
    assert_eq!(code, axum::http::StatusCode::OK, "{body}");
    let value = json(&body);
    assert_eq!(value["held"]["months"], 0);
    assert_eq!(value["exchange"], serde_json::Value::Null);
    assert_eq!(value["census"]["rows"], serde_json::json!([]));
    // An expiry the underlying does not hold: the same.
    let (code, _, body) = answer(
        &built_site,
        "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-04-25&rows=census-tuples-v1",
    );
    assert_eq!(code, axum::http::StatusCode::OK, "{body}");
    assert_eq!(json(&body)["held"]["months"], 0);
}

#[test]
fn a_rows_request_that_cannot_be_answered_is_refused_by_name() {
    for (query, why) in [
        (
            "feed=gdfl&segment=INDEX&rows=census-tuples-v1",
            "rows needs an underlying",
        ),
        (
            "feed=gdfl&segment=INDEX&underlying=NIFTY&rows=census-tuples-v1&since=3",
            "rows takes no since",
        ),
        (
            "feed=gdfl&segment=INDEX&underlying=NIFTY&rows=1",
            "is not an encoding this route writes",
        ),
        ("feed=gdfl&rows=census-tuples-v1", "segment"),
    ] {
        let (code, body) = asked(query).expect_err(query);
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST, "{query}");
        assert!(body.contains(why), "{query}: {body}");
    }
    // A garbage encoding is repeated capped, never whole.
    let long = "x".repeat(5_000);
    let (_, body) = asked(&format!(
        "feed=gdfl&segment=INDEX&underlying=NIFTY&rows={long}"
    ))
    .expect_err("refused");
    assert!(body.len() < 600, "{}", body.len());
}

#[test]
fn a_rows_answer_is_bounded_by_its_cap_and_refused_past_it() {
    // EXACTLY THE CAP ANSWERS; ONE KEY MORE IS REFUSED. One expiry holding
    // ROWS_CAP option keys, then one more strike.
    let march = month(2019, 3);
    let strike = |n: usize| {
        held(
            fno(
                &format!("2019-03-28-{}-CE", 1_000_000 + n * 5_000),
                Timeframe::SECOND_1,
                march,
            ),
            1,
            0,
            1,
            901,
        )
    };
    let mut rows: Vec<Held> = (0..ROWS_CAP).map(strike).collect();
    let slot = slot_of(&rows);
    let index = built(Segment::Fno, &slot);
    let query = "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28&rows=census-tuples-v1";
    let (exchange, keys, sums) = rows_keys(&index, &asked_for(query)).expect("at the cap");
    assert_eq!(exchange, Some(Exchange::Nse));
    assert_eq!(keys.len(), ROWS_CAP);
    assert_eq!(
        sums[&Timeframe::SECOND_1],
        u64::try_from(ROWS_CAP).expect("small")
    );

    rows.push(strike(ROWS_CAP));
    let slot = slot_of(&rows);
    let index = built(Segment::Fno, &slot);
    for query in [
        query,
        "feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1",
    ] {
        let (code, body) = rows_keys(&index, &asked_for(query)).expect_err("past the cap");
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        let value = json(&body);
        assert_eq!(value["error"], "rows_too_many");
        assert_eq!(value["cap"], ROWS_CAP);
        assert_eq!(value["reached"], ROWS_CAP + 1);
        assert!(
            value["why"].as_str().expect("why").contains("/store.json"),
            "{body}"
        );
    }
}

#[test]
fn a_key_the_index_cannot_name_refuses_the_rows_rather_than_leaving_it_out() {
    // A contract text that is not the store's spelling is counted by the
    // index and never listed: its underlying's rows are refused.
    let mut index = built(Segment::Fno, &slot_of(&chain()));
    assert!(
        rows_keys(
            &index,
            &asked_for("feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1")
        )
        .is_ok()
    );
    // Make the index hold one unread contract, the way `apply` counts one.
    index
        .underlyings
        .get_mut(&(Exchange::Nse, Symbol::new("NIFTY").expect("a symbol")))
        .expect("held")
        .unread_contracts = 1;
    let (code, body) = rows_keys(
        &index,
        &asked_for("feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1"),
    )
    .expect_err("refused");
    assert_eq!(code, axum::http::StatusCode::CONFLICT);
    assert_eq!(json(&body)["error"], "rows_unreadable");
}

#[test]
fn the_denominators_of_a_moved_index_equal_a_fresh_builds() {
    let rows = chain();
    let (first, rest) = rows.split_at(3);
    let mut disk = OnDisk::new("denoms-moved", first);
    let before = disk.read(None);
    let mut index = built(Segment::Fno, &before);
    let march = month(2019, 3);
    assert_eq!(index.denoms[&(march, Timeframe::SECOND_1)], (30, 3));

    // The rest lands and one key GROWS past the fullest: counted once, and
    // the maximum follows it.
    let mut appended: Vec<Held> = rest.to_vec();
    appended.push(held(
        fno("2019-03-28-1150000-CE", Timeframe::SECOND_1, march),
        95,
        0,
        120,
        7_101,
    ));
    disk.append(&appended);
    let after = disk.read(Some(before.census()));
    advance(&mut index, Segment::Fno, &after);
    let fresh = built(Segment::Fno, &after);
    assert_eq!(index.denoms, fresh.denoms);
    assert_eq!(index.denoms[&(march, Timeframe::SECOND_1)], (95, 5));
    // The INDEX keys of the same census are another segment's: not counted here.
    assert!(!index.denoms.contains_key(&(march, Timeframe::MINUTE_1)));
}

/// One NIFTY call of `expiry` at strike number `n`, held in `at` with one
/// bar (an invented price).
fn call_in(expiry: &str, n: usize, at: YearMonth) -> Held {
    held(
        fno(
            &format!("{expiry}-{}-CE", 1_000_000 + n * 5_000),
            Timeframe::SECOND_1,
            at,
        ),
        1,
        0,
        1,
        901,
    )
}

/// `count` months ending at 2020-06, oldest first.
fn months_to_june_2020(count: usize) -> Vec<YearMonth> {
    let mut out = Vec::with_capacity(count);
    let (mut year, mut at) = (2020_u16, 6_u8);
    for _ in 0..count {
        out.push(month(year, at));
        if at == 1 {
            year -= 1;
            at = 12;
        } else {
            at -= 1;
        }
    }
    out.reverse();
    out
}

#[test]
fn a_chain_of_more_contract_months_than_the_cap_answers_when_it_holds_fewer_rows() {
    // THE SECOND CHECK'S BLOCKING CASE (verify-WP-56b-r1, NIFTY 2020-06-25:
    // 233 contracts over a 22-month chain, 1,040 rows held, refused as 'more
    // than 4096 census rows'). Here: 300 strikes over a 22-month chain is
    // 6,600 contract-months, past the cap; each strike trades in two
    // neighbouring months, so the chain HOLDS 600 rows, and one more strike
    // trades in the first and the last month only (2 rows, 22 months apart).
    let chain_months = months_to_june_2020(22);
    let mut rows: Vec<Held> = Vec::new();
    for n in 0..300 {
        let at = n % 21;
        rows.push(call_in("2020-06-25", n, chain_months[at]));
        rows.push(call_in("2020-06-25", n, chain_months[at + 1]));
    }
    rows.push(call_in("2020-06-25", 300, chain_months[0]));
    rows.push(call_in("2020-06-25", 300, chain_months[21]));
    // A second expiry, so the no-expiry answer walks more than one chain.
    rows.push(call_in("2020-05-28", 0, chain_months[20]));
    // The candidates the old cap counted are past it.
    const { assert!(301 * 22 > ROWS_CAP) };

    let slot = slot_of(&rows);
    let index = built(Segment::Fno, &slot);
    let nifty = (Exchange::Nse, Symbol::new("NIFTY").expect("a symbol"));
    let under = &index.underlyings[&nifty];
    assert_eq!(under.agg.keys, 603, "the node counts the rows it holds");
    let june = Day::new(2020, 6, 25).expect("a day");
    assert_eq!(under.expiries[&june].agg.keys, 602);
    assert_eq!(
        under.expiries[&june].agg.span,
        Some((chain_months[0], chain_months[21]))
    );

    let at_expiry =
        "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2020-06-25&rows=census-tuples-v1";
    let (_, keys, sums) = rows_keys(&index, &asked_for(at_expiry)).expect("held rows answer");
    // 600 keys probed for the 300 two-month strikes (none missed) and the
    // 22 months between the sparse strike's first and last.
    assert_eq!(keys.len(), 600 + 22);
    assert_eq!(sums[&Timeframe::SECOND_1], 602);
    let part = part_of(&slot, Segment::Fno).expect("a part");
    let found = keys.iter().filter(|key| part.held(key).is_some()).count();
    assert_eq!(found, 602, "every row the chain holds is among the keys");

    let every = "feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1";
    let (_, keys, sums) = rows_keys(&index, &asked_for(every)).expect("the underlying answers");
    assert_eq!(keys.len(), 600 + 22 + 1);
    assert_eq!(sums[&Timeframe::SECOND_1], 603);

    // END TO END, through the route: the rows, every one, and the count.
    let built_site = site_holding("facets-rows-sparse", &rows);
    let (code, _, body) = answer(&built_site, at_expiry);
    assert_eq!(code, axum::http::StatusCode::OK, "{body}");
    let value = json(&body);
    assert_eq!(value["held"]["months"], 602);
    assert_eq!(value["held"]["rows"], 602);
    assert_eq!(expanded(&value["census"]).len(), 602);
}

#[test]
fn a_refusal_past_the_cap_states_the_rows_held_exactly_and_costs_one_probe() {
    // The count in the refusal is the node's own, not where counting
    // stopped: 37 past the cap reads 37 past the cap.
    let march = month(2019, 3);
    let rows: Vec<Held> = (0..ROWS_CAP + 37)
        .map(|n| call_in("2019-03-28", n, march))
        .collect();
    let index = built(Segment::Fno, &slot_of(&rows));
    for query in [
        "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28&rows=census-tuples-v1",
        "feed=gdfl&segment=FNO&underlying=NIFTY&rows=census-tuples-v1",
    ] {
        let (code, body) = rows_keys(&index, &asked_for(query)).expect_err("past the cap");
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        let value = json(&body);
        assert_eq!(value["error"], "rows_too_many");
        assert_eq!(value["reached"], ROWS_CAP + 37);
        assert_eq!(value["counted"], "census rows");
        let why = value["why"].as_str().expect("why");
        assert!(
            why.contains(&format!("holds {} census rows", ROWS_CAP + 37)),
            "{why}"
        );
        assert_eq!(
            why.contains("narrow it with expiry="),
            !query.contains("expiry=")
        );
    }

    // INDEX and CASH are capped by the same count. Make the node say it
    // holds one row past the cap, the way `apply` counts them.
    let mut index = built(Segment::Index, &slot_of(&chain()));
    let query = "feed=gdfl&segment=INDEX&underlying=NIFTY&rows=census-tuples-v1";
    assert!(rows_keys(&index, &asked_for(query)).is_ok());
    let nifty = (Exchange::Nse, Symbol::new("NIFTY").expect("a symbol"));
    index.underlyings.get_mut(&nifty).expect("held").agg.keys =
        u64::try_from(ROWS_CAP).expect("small") + 1;
    let (_, body) = rows_keys(&index, &asked_for(query)).expect_err("past the cap");
    assert_eq!(json(&body)["reached"], ROWS_CAP + 1);

    assert!(!past_cap(0));
    assert!(!past_cap(u64::try_from(ROWS_CAP).expect("small")));
    assert!(past_cap(u64::try_from(ROWS_CAP).expect("small") + 1));
    assert!(past_cap(u64::MAX));
}

#[test]
fn the_probe_budget_is_its_own_and_its_refusal_says_what_was_counted() {
    // Contracts held in the first and the last month of a 32-month chain:
    // 2 rows and 32 probes each, beside 32 one-month contracts (1 probe
    // each) that give the chain its months. 511 sparse ones are exactly
    // PROBE_CAP probes and answer; one more is refused, and the refusal says
    // the chain holds 1,056 rows (within the cap) and that the probes were
    // what ran out.
    let chain_months = months_to_june_2020(32);
    assert_eq!(512 * 32, PROBE_CAP);
    let sparse = |count: usize| -> Vec<Held> {
        (0..count)
            .flat_map(|n| {
                [
                    call_in("2020-06-25", n, chain_months[0]),
                    call_in("2020-06-25", n, chain_months[31]),
                ]
            })
            // Every month of the chain is named by some contract, so the
            // sparse ones' spans cover 32 chain months.
            .chain((0..32).map(|at| call_in("2020-06-25", 9_000 + at, chain_months[at])))
            .collect()
    };
    let query = "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2020-06-25&rows=census-tuples-v1";

    let index = built(Segment::Fno, &slot_of(&sparse(511)));
    let (_, keys, sums) = rows_keys(&index, &asked_for(query)).expect("inside the budget");
    assert_eq!(keys.len(), 511 * 32 + 32);
    assert_eq!(keys.len(), PROBE_CAP);
    assert_eq!(sums[&Timeframe::SECOND_1], 511 * 2 + 32);

    let index = built(Segment::Fno, &slot_of(&sparse(512)));
    let (code, body) = rows_keys(&index, &asked_for(query)).expect_err("past the budget");
    assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
    let value = json(&body);
    assert_eq!(value["error"], "rows_too_many");
    assert_eq!(value["counted"], "probes");
    assert_eq!(value["reached"], 512 * 2 + 32);
    assert_eq!(value["probe_cap"], PROBE_CAP);
    let why = value["why"].as_str().expect("why");
    assert!(why.contains("holds 1056 census rows"), "{why}");
    assert!(
        why.contains(&format!("more than {PROBE_CAP} probes")),
        "{why}"
    );
}

#[test]
fn the_row_count_and_month_span_of_a_moved_index_equal_a_fresh_builds() {
    // KEYS LAND LATE AND OUT OF ORDER: an older month after a newer one, a
    // key that grows (counted once), a duplicate append of the same key.
    let months = months_to_june_2020(4);
    let first = vec![
        call_in("2020-06-25", 0, months[2]),
        call_in("2020-06-25", 1, months[3]),
    ];
    let mut disk = OnDisk::new("keys-span-moved", &first);
    let before = disk.read(None);
    let mut index = built(Segment::Fno, &before);
    let grown = held(
        fno("2020-06-25-1000000-CE", Timeframe::SECOND_1, months[2]),
        9,
        0,
        50,
        905,
    );
    disk.append(&[
        call_in("2020-06-25", 0, months[0]),
        grown,
        call_in("2020-06-25", 2, months[1]),
    ]);
    let after = disk.read(Some(before.census()));
    advance(&mut index, Segment::Fno, &after);
    let fresh = built(Segment::Fno, &after);
    let nifty = (Exchange::Nse, Symbol::new("NIFTY").expect("a symbol"));
    let june = Day::new(2020, 6, 25).expect("a day");
    let (moved, built_fresh) = (
        &index.underlyings[&nifty].expiries[&june],
        &fresh.underlyings[&nifty].expiries[&june],
    );
    assert_eq!(moved.agg, built_fresh.agg);
    assert_eq!(moved.agg.keys, 4, "the grown key is counted once");
    assert_eq!(moved.agg.span, Some((months[0], months[3])));
    assert_eq!(moved.contracts, built_fresh.contracts);
    assert_eq!(index.underlyings[&nifty].agg, fresh.underlyings[&nifty].agg);
    assert_eq!(index.total, fresh.total);
}

#[test]
fn no_word_of_a_query_is_repeated_whole_in_a_refusal() {
    // GARBAGE PARAMETERS, 5,000 characters each: the refusal names the
    // parameter and repeats a capped piece of what was sent.
    let long = "x".repeat(5_000);
    for query in [
        format!("feed={long}&segment=INDEX"),
        format!("feed=gdfl&segment={long}"),
        format!("feed=gdfl&segment=INDEX&underlying={long}"),
        format!("feed=gdfl&segment=INDEX&underlying={long}&rows=census-tuples-v1"),
        format!("feed=gdfl&segment=INDEX&underlying=NIFTY&exchange={long}"),
        format!("feed=gdfl&segment=FNO&underlying=NIFTY&expiry={long}"),
        format!("feed=gdfl&segment=INDEX&since={long}"),
        format!("feed=gdfl&segment=INDEX&underlying=NIFTY&rows={long}"),
    ] {
        let (code, body) = asked(&query).expect_err("refused");
        assert_eq!(code, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.len() < 1_200, "{} B for {}", body.len(), &query[..40]);
        assert!(body.contains("(+"), "says how much was left out: {body}");
    }
    assert_eq!(capped("abc", 3), "abc");
    assert_eq!(capped("abcd", 3), "abc...(+1 more)");
    assert_eq!(capped("", 0), "");
}

#[test]
fn a_rows_answer_reads_the_same_keys_at_ten_times_the_census() {
    // THE COST OF ONE INSTRUMENT'S ROWS IS THE INSTRUMENT'S, NOT THE STORE'S
    // (RA-005): the keys probed for NIFTY's chain are the same, one for one,
    // whether the census holds 40 other underlyings or 400, each with its
    // own chain at the same expiry.
    let march = month(2019, 3);
    let census_of = |others: usize| -> Vec<Held> {
        let mut rows: Vec<Held> = (0..25).map(|n| call_in("2019-03-28", n, march)).collect();
        for other in 0..others {
            let symbol = Symbol::new(&format!("OTHER{other}")).expect("a symbol");
            for n in 0..8 {
                let mut key = fno(
                    &format!("2019-03-28-{}-PE", 2_000_000 + n * 5_000),
                    Timeframe::SECOND_1,
                    march,
                );
                key.symbol = symbol;
                rows.push(held(key, 1, 0, 1, 903));
            }
        }
        rows
    };
    let query = "feed=gdfl&segment=FNO&underlying=NIFTY&expiry=2019-03-28&rows=census-tuples-v1";
    let small = built(Segment::Fno, &slot_of(&census_of(40)));
    let large = built(Segment::Fno, &slot_of(&census_of(400)));
    assert_eq!(large.total.keys, 25 + 400 * 8);
    let (_, few, sums_small) = rows_keys(&small, &asked_for(query)).expect("answers");
    let (_, many, sums_large) = rows_keys(&large, &asked_for(query)).expect("answers");
    assert_eq!(few.len(), 25);
    assert_eq!(few, many, "the same probes, whatever else the census holds");
    assert_eq!(sums_small, sums_large);
}

/// **A RETIRED INDEX IS TAKEN APART IN PACED STEPS, ITS CENSUS READ LAST**
/// (R3-09 repair 1, blocking item 4). Freed in one burst, GDFL's FNO index
/// stalled every request of the process 67 to 138 ms at each census change.
/// [`dismantle`] pauses after every `step` nodes, frees every node it
/// counts, and lets the census read go only after the last pause: while it
/// runs, the read is still held.
#[test]
fn a_retired_index_is_taken_apart_in_paced_steps_and_its_census_read_last() {
    let slot = slot_of(&chain());
    let index = built(Segment::Fno, &slot);
    let nodes: usize = index
        .underlyings
        .values()
        .map(|one| {
            one.expiries
                .values()
                .map(|expiry| expiry.contracts.len() + expiry.months.len() + 1)
                .sum::<usize>()
                + one.months.len()
                + one.cells.len()
                + 1
        })
        .sum::<usize>()
        + index.denoms.len()
        + index.cells.len();
    assert!(nodes >= 8, "the chain indexes {nodes} nodes");
    let held = Arc::strong_count(&slot);
    assert_eq!(held, 2, "the index holds its read");
    let mut pauses = 0usize;
    let mut held_at_each_pause = Vec::new();
    let freed = dismantle(index, 2, &mut || {
        pauses += 1;
        held_at_each_pause.push(Arc::strong_count(&slot));
    });
    // EVERY NODE COUNTED: each contract, month and expiry, each
    // underlying's months, cells and itself, and the segment's own sums.
    assert_eq!(freed, nodes);
    // A PAUSE AFTER A STEP, AND ONE MORE BEFORE THE CENSUS READ.
    assert!(pauses >= 2, "{pauses} pauses for {freed} nodes");
    assert!(
        held_at_each_pause.iter().all(|&count| count == held),
        "the census read was let go before the nodes: {held_at_each_pause:?}"
    );
    assert_eq!(Arc::strong_count(&slot), 1, "and then let go");
    // A STEP OF ZERO IS A STEP OF ONE, never a division by it: a pause after
    // every expiry, every underlying and the segment's sums, and the last.
    let index = built(Segment::Fno, &slot);
    let steps = index
        .underlyings
        .values()
        .map(|one| one.expiries.len() + 1)
        .sum::<usize>()
        + 1;
    let mut zero_step = 0usize;
    assert_eq!(dismantle(index, 0, &mut || zero_step += 1), freed);
    assert_eq!(zero_step, steps + 1);
    // A STEP NO INDEX REACHES pauses once, before the census read.
    let mut one = 0usize;
    assert_eq!(
        dismantle(built(Segment::Fno, &slot), usize::MAX, &mut || one += 1),
        freed
    );
    assert_eq!(one, 1);
    // AN EMPTY INDEX frees nothing and pauses once, before its (absent) read.
    let mut empty = 0usize;
    assert_eq!(dismantle(Index::default(), 2, &mut || empty += 1), 0);
    assert_eq!(empty, 1);
}

/// What the two counters must equal: every expiry's contracts summed, and
/// each expiry's distinct option strikes, walked the way the answers walked
/// them before the counters existed.
fn walked(index: &Index) -> Vec<(u64, Vec<u64>)> {
    index
        .underlyings()
        .map(|(_, under)| {
            let contracts = under
                .expiries
                .values()
                .map(|chain| u64::try_from(chain.contracts.len()).expect("fits"))
                .sum();
            let strikes = under
                .expiries
                .values()
                .map(|chain| {
                    let set: BTreeSet<i64> = chain
                        .contracts
                        .keys()
                        .filter_map(|id| match *id {
                            ContractId::Option { strike, .. } => Some(strike),
                            ContractId::Future => None,
                        })
                        .collect();
                    u64::try_from(set.len()).expect("fits")
                })
                .collect();
            (contracts, strikes)
        })
        .collect()
}

/// The counters the index keeps.
fn counted(index: &Index) -> Vec<(u64, Vec<u64>)> {
    index
        .underlyings()
        .map(|(_, under)| {
            (
                under.contracts,
                under.expiries.values().map(|chain| chain.strikes).collect(),
            )
        })
        .collect()
}

/// **THE UNDERLYINGS ANSWER READS A COUNTER, NOT EVERY EXPIRY**
/// (`cloud/attack-o1-20261016`).
///
/// With no underlying asked, `render_answer` summed the contracts of every
/// expiry of every underlying, and with no expiry asked it collected every
/// contract of every expiry into a set to count strikes: O(contracts of the
/// segment) per request, growing with every stored month. `Index::apply`
/// now counts both as a contract is first seen. Here the counters equal the
/// walk on a fresh build, after a move that adds rows and grows one, and
/// after a CE's PE lands at a strike already counted; and the answer sites
/// hold no walk, asserted on the source (needles assembled at run time).
#[test]
fn the_contract_and_strike_counts_are_counters_kept_by_apply() {
    let rows = chain();
    let (first, rest) = rows.split_at(1);
    let mut disk = OnDisk::new("counted", first);
    let before = disk.read(None);
    let mut index = built(Segment::Fno, &before);
    assert_eq!(counted(&index), walked(&index));
    assert_eq!(counted(&index), vec![(1, vec![1])], "one CE, one strike");

    // THE PE AT THE SAME STRIKE: one more contract, no more strikes.
    disk.append(&rest[..1]);
    let moved = disk.read(Some(before.census()));
    advance(&mut index, Segment::Fno, &moved);
    assert_eq!(counted(&index), walked(&index));
    assert_eq!(counted(&index), vec![(2, vec![1])]);

    // THE REST, AND A KEY THAT GROWS: a grown key is no new contract.
    let mut grown = rest[1..].to_vec();
    grown.push(held(
        fno("2019-03-28-1150000-CE", Timeframe::SECOND_1, month(2019, 3)),
        11,
        0,
        110,
        7_101,
    ));
    disk.append(&grown);
    let after = disk.read(Some(moved.census()));
    advance(&mut index, Segment::Fno, &after);
    assert_eq!(counted(&index), walked(&index));
    assert_eq!(
        counted(&index),
        vec![(5, vec![1, 2])],
        "five contracts; 03-07 one strike, 03-28 two"
    );
    let fresh = built(Segment::Fno, &after);
    assert_eq!(counted(&fresh), counted(&index), "moved equals fresh");

    let source = include_str!("facets.rs");
    for needle in [
        format!(
            "{}{}",
            ".map(|chain| chain.contracts.len())", "\n            .sum()"
        ),
        format!("{}{}", "let strikes: BTreeSet", "<i64> = chain"),
    ] {
        assert!(
            !source.contains(&needle),
            "an answer walks the contracts again: {needle:?}"
        );
    }
    let _ = std::fs::remove_dir_all(&disk.root);
}
