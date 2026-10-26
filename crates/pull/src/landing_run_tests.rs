#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail; and one test per test id, \
              each holding every arm of its row."
)]

//! IA-10c: every Zerodha landing's run id (IA-GR-43), both landing doors kept
//! and counted (IA-RK-07), the repair door's window kept and verified (§8
//! Z16), the vendor's landing claim (FM-163), the gap ledger under the audit
//! ledger lock (IA-GR-41, the gap ledger's half), `origin=store`, and one
//! month landed through the pull's door then verified cold as `match` for
//! every minute. Every bar and body here is invented: the instrument is
//! `ZZIDX` and its prices sit near 11,111.00, which no real print names
//! (CLAUDE.md: no vendor row in the repository).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use brutex_core::instrument::{Exchange, InstrumentKey};
use store::path::{Timeframe, YearMonth};
use store::verdict::Code;

use super::*;
use crate::fetch::{BarRequest, RawWindow};
use crate::gdfl_run::ledger;
use crate::ingest::{self, DoorLanding, Plan};
use crate::response_keep::{self, Door, Kept};
use crate::session::{Day, Window};
use crate::vendor::{Feed, Listing, Transport};
use crate::zerodha_verify::{ColdRequest, cold, merge_all};

// ------------------------------------------------------------- fixtures --

fn scratch(tag: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let root = std::env::temp_dir().join(format!(
        "brutex-ia10c-{tag}-{}-{}-{nanos}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// The Zerodha descriptor's HTTP spec: the one the pull decodes with.
fn spec() -> crate::vendor::HttpSpec {
    match Feed::Zerodha.descriptor().transport {
        Transport::Http(spec) => spec,
        Transport::LocalArchive(_) => panic!("Zerodha is an HTTP feed"),
    }
}

/// The invented price of minute `m` past IST midnight.
fn price(m: u32) -> i64 {
    1_111_100 + i64::from(m)
}

fn rupees(paisa: i64) -> String {
    format!("{}.{:02}", paisa / 100, paisa % 100)
}

/// One Kite-shaped candle at `on` `hh:mm:ss`, priced by `p` paisa.
fn candle(on: Day, minute: u32, second: u32, p: i64) -> String {
    format!(
        "[\"{on}T{:02}:{:02}:{second:02}+0530\",{},{},{},{},{}]",
        minute / 60,
        minute % 60,
        rupees(p),
        rupees(p + 50),
        rupees(p - 50),
        rupees(p + 10),
        100 + minute
    )
}

/// Every minute 09:15..15:29 of each day in `days`, plus `extra` candles.
fn kite_body(days: &[Day], extra: &[String]) -> String {
    let mut rows = Vec::new();
    for on in days {
        for m in 555..930 {
            rows.push(candle(*on, m, 0, price(m)));
        }
    }
    rows.extend(extra.iter().cloned());
    format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        rows.join(",")
    )
}

/// The sessions of record in `month` whose length is the regular 375
/// minutes (the fixture's bars fill exactly those).
fn regular_days(month: YearMonth) -> Vec<Day> {
    let first = day(month.year(), month.month(), 1);
    let mut out = Vec::new();
    let mut on = first;
    while on.year_month().unwrap() == month {
        if let crate::calendar::DayKind::Open(session) =
            crate::calendar::kind_of(i64::from(on.days_from_epoch()))
            && session.bars() == 375
        {
            out.push(on);
        }
        on = Day::from_days(on.days_from_epoch() + 1).unwrap();
    }
    out
}

fn key() -> InstrumentKey {
    InstrumentKey::index(Exchange::Nse, "ZZIDX").unwrap()
}

/// The answer as the pull's wire hands it over: its body decoded by the
/// one shared decoder, the rows moved out to the bodies (the api's split).
fn answered(body: &str) -> (RawWindow, response_keep::Answered) {
    let decoded = crate::http::decode_body(body, &spec(), Listing::Index).unwrap();
    let mut answered = response_keep::Answered::of(decoded, body.to_owned(), 200);
    let rows = std::mem::take(&mut answered.window);
    (rows, answered)
}

/// Lands `rows` of `window` for ZZIDX through `door` under run `run`, as the
/// api's `land_bodies_observed` does.
fn land(root: &Path, rows: &RawWindow, window: Window, door: Door, run: u64) -> DoorLanding {
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window,
        granularity: crate::vendor::Granularity::Minute1,
    };
    let plan = Plan {
        calendar: crate::calendar::Runtime::default(),
        cash_schedule: None,
        columns: crate::csv::Columns::Gdfl,
        request: &request,
        encoding: spec().timestamps,
        scale: crate::http::DECODED_PRICE_SCALE,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
        run,
    };
    match door {
        Door::FromWindow => {
            ingest::from_window_with_refusals(rows, "ZZIDX", "https://kite.invalid", root, plan)
        }
        Door::RepairWindow => ingest::repair_window_with_refusals(
            rows,
            "ZZIDX",
            "https://kite.invalid",
            root,
            plan,
            "fixture repair",
        ),
    }
}

/// Lands and hands the window to `hook`: the whole of one body's road.
fn land_and_hook(
    root: &Path,
    hook: &RunHook,
    body: &str,
    window: Window,
    door: Door,
) -> (DoorLanding, Hooked) {
    let (rows, answered) = answered(body);
    let landing = land(root, &rows, window, door, hook.run());
    let key = key();
    let hooked = hook.after_landing(&Landed {
        key: &key,
        granularity: crate::vendor::Granularity::Minute1,
        asked: window,
        answered: &answered,
        rows: &rows.rows,
        landing: &landing,
        door,
        encoding: spec().timestamps,
    });
    (landing, hooked)
}

fn ledger_text(root: &Path, name: &str) -> String {
    std::fs::read_to_string(root.join("audit").join(format!("zerodha.{name}"))).unwrap_or_default()
}

// ------------------------------------------------------------- IA-GR-43 --

/// IA-GR-43 (D-4880, D-6260): every Zerodha landing takes one runs-ledger id
/// with its mode: `pull` (a `/pull/run` at its claim), `pull-spot`,
/// `pull-fno`, `pull-repair` and `zerodha-repair`; no two share one, across
/// a "restart" too (the ledger is the only memory: a fresh call after the
/// last still goes up), and the id is what a landing's ledger lines name.
#[test]
fn every_zerodha_landing_takes_one_runs_ledger_id() {
    let root = scratch("ids");
    let mut ids = Vec::new();
    for (mode, by) in [
        (Mode::Run, By::Api),
        (Mode::Spot, By::Api),
        (Mode::Fno, By::Api),
        (Mode::Repair, By::Api),
        (Mode::StoreRepair, By::Cli),
    ] {
        ids.push(allocate(&root, mode, by, kind_of(Vendor::Zerodha)).unwrap());
    }
    // A RESTART: nothing in this process remembers; the ledger does.
    let after = allocate(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha)).unwrap();
    ids.push(after);
    let mut sorted = ids.clone();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        ids.len(),
        "no two landings share an id: {ids:?}"
    );
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]), "{ids:?}");
    let records: Vec<ledger::Entry> = ledger::read_all(&root)
        .unwrap()
        .into_iter()
        .map(Result::unwrap)
        .filter(|entry| entry.mode != "seed")
        .collect();
    let words: Vec<(&str, &str, &str)> = records
        .iter()
        .map(|e| (e.mode.as_str(), e.kind.as_str(), e.by.as_str()))
        .collect();
    assert_eq!(
        words,
        [
            ("pull", "zerodha", "api"),
            ("pull-spot", "zerodha", "api"),
            ("pull-fno", "zerodha", "api"),
            ("pull-repair", "zerodha", "api"),
            ("zerodha-repair", "zerodha", "cli"),
            ("pull-spot", "zerodha", "api"),
        ]
    );
    assert_eq!(
        records.iter().map(|e| e.id).collect::<Vec<_>>(),
        ids,
        "each id is the ledger's own record"
    );
    // The kinds a vendor maps to: Zerodha's, and `-` for one the ledger
    // names no kind for; a word the ledger does not hold is refused by name.
    assert_eq!(kind_of(Vendor::Gdfl), "-");
    let refused = allocate(&root, Mode::Spot, By::Api, "nonsense").unwrap_err();
    assert_eq!(refused.code(), "runs_ledger_entry", "{refused}");
    // A store root that is gone takes no id and creates nothing.
    let gone = root.join("unplugged");
    assert!(allocate(&gone, Mode::Spot, By::Api, "zerodha").is_err());
    assert!(!gone.exists(), "a gone root is never recreated");
}

/// IA-GR-36, IA-10c's half (D-4880, D-6260): ONE RUNS LEDGER SERVES BOTH
/// FEEDS AND SKIPS A TORN TAIL. A GDFL import and every Zerodha landing mode,
/// taken side by side from two threads after a crash left a torn tail, never
/// get one id: the tail is written over, never read as an id, and every
/// record reads back whole with its own feed's words.
#[test]
fn one_runs_ledger_serves_both_feeds_and_skips_a_torn_tail() {
    const EACH: usize = 20;
    const MODES: [Mode; 5] = [
        Mode::Run,
        Mode::Spot,
        Mode::Fno,
        Mode::Repair,
        Mode::StoreRepair,
    ];
    let root = scratch("feeds");
    let gdfl_import = || ledger::Entry {
        id: 0,
        mode: "import".to_owned(),
        kind: "options".to_owned(),
        by: "cli".to_owned(),
        pid: std::process::id(),
        started: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        resumes: 0,
    };
    let zerodha_landing = |n: usize| {
        let mode = MODES[n % MODES.len()];
        let by = if mode == Mode::StoreRepair {
            By::Cli
        } else {
            By::Api
        };
        allocate(&root, mode, by, kind_of(Vendor::Zerodha)).unwrap()
    };
    let first_gdfl = ledger::allocate(&root, &gdfl_import()).unwrap();
    let first_zerodha = zerodha_landing(0);
    assert_ne!(first_gdfl, first_zerodha);

    // A CRASH'S TORN TAIL: bytes past the last whole 128-byte record.
    let path = root.join(ledger::LEDGER);
    let mut bytes = std::fs::read(&path).unwrap();
    let whole = bytes.len();
    assert_eq!(whole % 128, 0);
    bytes.extend_from_slice(b"99999999999 import options cli");
    std::fs::write(&path, &bytes).unwrap();

    // TWO FEEDS AT ONCE: a GDFL import thread and a Zerodha landing thread.
    let (gdfl, zerodha) = std::thread::scope(|scope| {
        let gdfl = scope.spawn(|| {
            (0..EACH)
                .map(|_| ledger::allocate(&root, &gdfl_import()).unwrap())
                .collect::<Vec<u64>>()
        });
        let zerodha = scope.spawn(|| (1..=EACH).map(zerodha_landing).collect::<Vec<u64>>());
        (gdfl.join().unwrap(), zerodha.join().unwrap())
    });
    let mut ids: Vec<u64> = [first_gdfl, first_zerodha]
        .into_iter()
        .chain(gdfl.iter().copied())
        .chain(zerodha.iter().copied())
        .collect();
    let taken = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids.len(),
        taken,
        "a GDFL run and a Zerodha landing never share an id: {gdfl:?} {zerodha:?}"
    );
    assert!(
        gdfl.iter().chain(&zerodha).all(|id| *id > first_zerodha),
        "every id after the tear is past the last whole record's"
    );
    assert!(
        gdfl.windows(2).all(|pair| pair[0] < pair[1])
            && zerodha.windows(2).all(|pair| pair[0] < pair[1]),
        "each writer's ids rise"
    );

    // THE TAIL WAS WRITTEN OVER: whole records only, each read back whole.
    let len = std::fs::metadata(&path).unwrap().len();
    assert_eq!(len % 128, 0, "no torn byte is left: {len}");
    let records: Vec<ledger::Entry> = ledger::read_all(&root)
        .unwrap()
        .into_iter()
        .map(|record| record.expect("every record reads back whole"))
        .filter(|entry| entry.mode != "seed")
        .collect();
    assert_eq!(records.len(), taken);
    let of_feed = |kind: &str| {
        records
            .iter()
            .filter(|entry| entry.kind == kind)
            .map(|entry| entry.id)
            .collect::<Vec<u64>>()
    };
    let mut gdfl_ids: Vec<u64> = [first_gdfl].into_iter().chain(gdfl).collect();
    gdfl_ids.sort_unstable();
    let mut zerodha_ids: Vec<u64> = [first_zerodha].into_iter().chain(zerodha).collect();
    zerodha_ids.sort_unstable();
    assert_eq!(
        of_feed("options"),
        gdfl_ids,
        "the GDFL records are the GDFL ids"
    );
    assert_eq!(
        of_feed("zerodha"),
        zerodha_ids,
        "the Zerodha records are its ids"
    );
    for mode in MODES {
        assert!(
            records
                .iter()
                .any(|entry| entry.mode == mode.word() && entry.kind == "zerodha"),
            "{} is on the ledger",
            mode.word()
        );
    }
    assert!(
        records
            .iter()
            .filter(|entry| entry.kind == "options")
            .all(|entry| entry.mode == "import" && entry.by == "cli"),
        "a GDFL record keeps its own words"
    );
}

/// Every word of every mode is a word the runs ledger holds (a mode the
/// ledger did not hold would refuse every landing of it).
#[test]
fn every_landing_mode_is_a_word_the_runs_ledger_holds() {
    for mode in [
        Mode::Run,
        Mode::Spot,
        Mode::Fno,
        Mode::Repair,
        Mode::StoreRepair,
    ] {
        assert!(ledger::MODES.contains(&mode.word()), "{}", mode.word());
    }
    for by in [By::Api, By::Cli] {
        assert!(ledger::BY.contains(&by.word()), "{}", by.word());
    }
    assert!(ledger::KINDS.contains(&kind_of(Vendor::Zerodha)));
    assert!(ledger::KINDS.contains(&kind_of(Vendor::Dhan)));
}

// ------------------------------------ the runs ledger's lost last record --

/// A clock inside the calendar's span: 2026-10-01 00:00 UTC (IA-2's own).
const CLOCK: u64 = 1_790_812_800;

/// A store where Zerodha landing `used`, given at `CLOCK` in the same second as
/// `prev`, landed one day through the pull's door with the hook: its
/// receipt, its spool directory and its ledger record all say `used`.
/// `via_ledger` takes `used` the way a cold verification or a merge takes
/// its id, through the runs ledger itself rather than [`allocate_at`].
fn a_store_a_landing_used(tag: &str, via_ledger: bool) -> (PathBuf, u64, u64) {
    let root = scratch(tag);
    let prev = allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
    let used = if via_ledger {
        ledger::allocate(
            &root,
            &ledger::Entry {
                id: 0,
                mode: "pull-spot".to_owned(),
                kind: "zerodha".to_owned(),
                by: "api".to_owned(),
                pid: 4242,
                started: CLOCK,
                resumes: 0,
            },
        )
        .unwrap()
    } else {
        allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap()
    };
    assert_eq!((prev, used), (CLOCK, CLOCK + 1), "two ids in one second");
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let hook = RunHook::new(&root, used, Vendor::Zerodha, 1);
    let (landing, hooked) = land_and_hook(
        &root,
        &hook,
        &kite_body(&[on], &[]),
        window,
        Door::FromWindow,
    );
    assert_eq!(landing.ingested.bars_committed, 375, "{landing:?}");
    assert_eq!(
        u64::from(hooked.kept.unwrap().first.receipt.run),
        used,
        "the receipt names the landing"
    );
    assert!(hooked.spooled.unwrap().unwrap().records > 0);
    drop(hook);
    assert!(root.join("verify/spool").join(used.to_string()).is_dir());
    (root, prev, used)
}

/// The second agent's round-2 BLOCKING 2 (D-6269; D-4880, D-5901): A DAMAGED
/// LAST RECORD THAT A ZERODHA LANDING USED IS NEVER HANDED OUT AGAIN. A
/// landing was given `used`, kept its answer under it (the receipt's offset
/// 60), spooled into `verify/spool/<used>`, and then its ledger record
/// rotted (its CRC fails). With the clock at that very second, or ten
/// minutes behind it, the next allocation writes over the bad record and is
/// raised past every id the runs ledger has given, which its claims
/// (`audit/runs.claims`) hold: never `used` again, so a second landing never
/// appends into the first one's spool. Before D-6269 the raise read only the
/// GDFL journal, which no Zerodha landing writes, and the next id was `used`
/// itself. Taken through [`allocate`] and the way a cold verification or a
/// merge takes its id (the ledger directly), which the claims cover alike.
#[test]
fn a_damaged_last_record_that_a_zerodha_landing_used_is_never_handed_out_again() {
    for via_ledger in [false, true] {
        for behind in [0, 600] {
            let (root, prev, used) = a_store_a_landing_used("rotted", via_ledger);
            let path = root.join(ledger::LEDGER);
            let mut bytes = std::fs::read(&path).unwrap();
            let len = bytes.len();
            bytes[len - 60] ^= 1;
            std::fs::write(&path, &bytes).unwrap();
            let again = allocate_at(
                &root,
                Mode::Spot,
                By::Api,
                kind_of(Vendor::Zerodha),
                CLOCK - behind,
            )
            .unwrap();
            let case = format!("via_ledger={via_ledger}, the clock {behind} s behind");
            assert!(
                again > used,
                "{case}: {again} was handed out again (prev {prev}, used {used})"
            );
            assert!(
                !root.join("verify/spool").join(again.to_string()).exists(),
                "{case}: the new id names a spool that is already there"
            );
            assert_eq!(
                std::fs::read(&path).unwrap().len(),
                len,
                "{case}: written over"
            );
            let ids: Vec<u64> = ledger::read_all(&root)
                .unwrap()
                .into_iter()
                .map(|record| record.unwrap().id)
                .collect();
            assert_eq!(ids.last(), Some(&again), "{case}: {ids:?}");
        }
    }
}

/// D-6269, the seed's half: AN UNBORN LEDGER IS SEEDED PAST EVERY ID A
/// ZERODHA LANDING USED. The ledger of a store whose landing used `used` is
/// emptied, cut short of one record, or left as one record whose CRC fails
/// (a store put back without its ledger, a torn first write): each is unborn
/// and is seeded again past the largest id the claims hold, never at the
/// clock below it. Before D-6269 the seed was the GDFL journal's largest run
/// (none here), and the next id was `prev`, the clock's own second, again.
#[test]
fn an_unborn_ledger_is_seeded_past_every_id_a_zerodha_landing_used() {
    for shape in [
        "emptied whole",
        "short of one record",
        "one record whose CRC fails",
    ] {
        let (root, prev, used) = a_store_a_landing_used("unborn", false);
        let path = root.join(ledger::LEDGER);
        let bytes = std::fs::read(&path).unwrap();
        let unborn = match shape {
            "emptied whole" => Vec::new(),
            "short of one record" => bytes[..100].to_vec(),
            _ => {
                let mut one = bytes[bytes.len() - 128..].to_vec();
                one[10] ^= 1;
                one
            }
        };
        std::fs::write(&path, &unborn).unwrap();
        let again =
            allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
        assert!(
            again > used,
            "{shape}: {again} was handed out again (prev {prev}, used {used})"
        );
        let records: Vec<ledger::Entry> = ledger::read_all(&root)
            .unwrap()
            .into_iter()
            .map(Result::unwrap)
            .collect();
        assert_eq!(
            records.first().map(|seed| (seed.mode.as_str(), seed.id)),
            Some(("seed", used)),
            "{shape}: seeded at the largest id the claims hold"
        );
        assert_eq!(records.last().map(|entry| entry.id), Some(again), "{shape}");
    }
}

/// The ids of the ledger's records, in order.
fn ledger_ids(root: &Path) -> Vec<u64> {
    ledger::read_all(root)
        .unwrap()
        .into_iter()
        .map(|record| record.unwrap().id)
        .collect()
}

/// The alerts of kind `runs_ledger_damaged` put for a ledger behind its
/// claims at run `claimed`: (their count, the text of the first).
fn behind_alerts(root: &Path, claimed: u64) -> (usize, Option<String>) {
    let key = crate::alerts::key_hash(&format!("runs_ledger_behind_claims:{claimed}"));
    let page = crate::alerts::window(root).unwrap();
    let hits: Vec<&crate::alerts::Alert> = page
        .alerts
        .iter()
        .filter(|alert| alert.record.kind == 8 && alert.record.key == key)
        .collect();
    assert!(
        hits.iter().all(|alert| alert.record.run == claimed),
        "the alert names the claimed run: {hits:?}"
    );
    (
        hits.len(),
        hits.first().map(|alert| alert.record.text.clone()),
    )
}

/// The second agent's repair-round-3 BLOCKING (D-6269, repair round 3): A
/// LEDGER THAT LOST ITS LAST RECORDS WHOLE NEVER HANDS THEIR IDS OUT AGAIN.
/// A landing was given `used`, kept its answer under it (the receipt's
/// offset 60) and spooled into `verify/spool/<used>`; then
/// `audit/runs.ledger` lost its last record whole, cut at a record boundary
/// (a drive that dropped the last flushed append, a ledger put back from a
/// copy one record older), or its last two (a copy two records older, which
/// leaves the seed alone), while `audit/runs.claims` kept every line. The
/// ledger's own last record then verifies and says nothing of the loss.
/// With the clock at that very second or ten minutes behind it, the next id
/// is past `used`, names no spool already there and is the ledger's new
/// last record; the claims gain its line; the loss is said by one alert of
/// kind `runs_ledger_damaged` naming the claimed run, and the allocation
/// after it, the ledger no longer behind, puts none and takes the next id.
/// Taken through [`allocate`] and the way a cold verification or a merge
/// takes its id (the ledger directly). Before repair round 3 the claims were
/// read only for a record whose CRC failed or an unborn ledger, and the next
/// id was `used` itself, so a second landing appended into the first one's
/// spool (probe P1). Then a GDFL import's id lost the same way is never
/// given to the next Zerodha landing (IA-GR-36's two feeds, one ledger).
#[test]
fn a_ledger_that_lost_its_last_records_whole_never_hands_their_ids_out_again() {
    for via_ledger in [false, true] {
        for lost in [1_u64, 2] {
            for behind in [0, 600] {
                let case = format!(
                    "via_ledger={via_ledger}, {lost} record(s) lost whole, the clock {behind} s \
                     behind"
                );
                let (root, prev, used) = a_store_a_landing_used("lost-whole", via_ledger);
                let path = root.join(ledger::LEDGER);
                assert_eq!(
                    ledger_ids(&root),
                    [0, prev, used],
                    "{case}: seed, prev, used"
                );
                let len = std::fs::metadata(&path).unwrap().len();
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .set_len(len - lost * ledger::RECORD)
                    .unwrap();
                let left = if lost == 1 { vec![0, prev] } else { vec![0] };
                assert_eq!(ledger_ids(&root), left, "{case}: cut at a record boundary");
                let claims_path = root.join(ledger::CLAIMS);
                let claims = std::fs::read_to_string(&claims_path).unwrap();
                assert_eq!(
                    claims.lines().count(),
                    2,
                    "{case}: the claims kept: {claims}"
                );
                let again = allocate_at(
                    &root,
                    Mode::Spot,
                    By::Api,
                    kind_of(Vendor::Zerodha),
                    CLOCK - behind,
                )
                .unwrap();
                assert!(
                    again > used,
                    "{case}: {again} was handed out again (prev {prev}, used {used})"
                );
                assert!(
                    !root.join("verify/spool").join(again.to_string()).exists(),
                    "{case}: the new id names a spool that is already there"
                );
                let mut ids = left.clone();
                ids.push(again);
                assert_eq!(ledger_ids(&root), ids, "{case}: appended after the cut");
                let grown = std::fs::read_to_string(&claims_path).unwrap();
                assert!(
                    grown.starts_with(&claims)
                        && grown.lines().count() == 3
                        && grown
                            .lines()
                            .last()
                            .unwrap()
                            .starts_with(&format!("run {again} pull-spot zerodha api ")),
                    "{case}: the claims gain the new id's line: {grown}"
                );
                let (alerts, text) = behind_alerts(&root, used);
                assert_eq!(alerts, 1, "{case}: one alert");
                assert_eq!(
                    text.as_deref(),
                    Some(
                        format!(
                            "ledger ends at run {}; its claims name run {used}",
                            left[left.len() - 1]
                        )
                        .as_str()
                    ),
                    "{case}"
                );
                let next = allocate_at(
                    &root,
                    Mode::Spot,
                    By::Api,
                    kind_of(Vendor::Zerodha),
                    CLOCK - behind,
                )
                .unwrap();
                assert_eq!(next, again + 1, "{case}: no longer behind");
                assert_eq!(behind_alerts(&root, used).0, 1, "{case}: no second alert");
            }
        }
    }

    // A GDFL import's id, which its journal line names too, lost whole.
    let root = scratch("lost-whole");
    let import = ledger::allocate(
        &root,
        &ledger::Entry {
            id: 0,
            mode: "import".to_owned(),
            kind: "options".to_owned(),
            by: "cli".to_owned(),
            pid: 4242,
            started: CLOCK,
            resumes: 0,
        },
    )
    .unwrap();
    assert_eq!(import, CLOCK);
    let journal = crate::gdfl_import::journal_path(&root);
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    std::fs::write(
        &journal,
        format!("run {import} import options cli 4242 {CLOCK}\n"),
    )
    .unwrap();
    let path = root.join(ledger::LEDGER);
    let len = std::fs::metadata(&path).unwrap().len();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(len - ledger::RECORD)
        .unwrap();
    let landing = allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
    assert!(
        landing > import,
        "a GDFL run's lost id {import} was given to a Zerodha landing"
    );
    assert_eq!(behind_alerts(&root, import).0, 1);
}

/// D-6269, repair round 3: A CLAIMS LINE IS TRUSTED ONLY WHEN ITS CRC
/// VERIFIES, so one rotted byte of the claims can neither lift every later
/// id nor hide one. The claims' last line of a healthy store rots: the first
/// digit of its id turned to `9` (a run past what a receipt's 32-bit field
/// holds), or a digit of its id turned to `0` (an id below the ledger's).
/// Either way the next id is the ledger's own next, the rotted line is
/// never read for an id, and no alert is put (the ledger is not behind).
/// A torn fragment after the last line (no line break) is passed over, and
/// the append closes it as a line of its own.
#[test]
fn a_rotted_claims_line_neither_lifts_nor_hides_an_id() {
    for (shape, from, to) in [("lifted", "run 1", "run 9"), ("lowered", "801 ", "001 ")] {
        let root = scratch("claims");
        let prev =
            allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
        let used =
            allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
        assert_eq!((prev, used), (CLOCK, CLOCK + 1));
        let claims_path = root.join(ledger::CLAIMS);
        let claims = std::fs::read_to_string(&claims_path).unwrap();
        let cut = claims.trim_end_matches('\n').rfind('\n').unwrap() + 1;
        let (head, last) = claims.split_at(cut);
        assert!(last.starts_with(&format!("run {used} ")), "{last}");
        let rotted = last.replacen(from, to, 1);
        assert_ne!(rotted, last, "{shape}: the fixture rots the line");
        std::fs::write(&claims_path, format!("{head}{rotted}run 17")).unwrap();
        let next =
            allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
        assert_eq!(next, used + 1, "{shape}: the ledger's own next id");
        assert_eq!(ledger_ids(&root), [0, prev, used, next], "{shape}");
        assert_eq!(behind_alerts(&root, used).0, 0, "{shape}: not behind");
        let grown = std::fs::read_to_string(&claims_path).unwrap();
        assert!(
            grown.starts_with(&format!(
                "{head}{rotted}run 17\nrun {next} pull-spot zerodha api "
            )),
            "{shape}: the torn fragment closed, the new line after it: {grown}"
        );
    }
}

/// IA-10c repair 3 (the second agent's repair-round-3 probe P4c): CLAIMS
/// THAT ARE NOT A REGULAR FILE ARE REFUSED AT ONCE, NEVER WAITED ON UNDER
/// THE LEDGER'S LOCK. A pipe at `audit/runs.claims`, with the ledger's last
/// record healthy (its tail is read) and with it rotted (the claims are
/// read whole): each allocation is refused `runs_ledger_io` naming the
/// claims as not a regular file, within seconds, and writes no record. A
/// link at the name is refused the same way, and the file it names is
/// untouched. Before repair round 3 the rotted case opened the pipe with a
/// blocking read and waited, holding `audit/runs.ledger.lock`, until a
/// writer came.
#[test]
fn claims_that_are_not_a_regular_file_are_refused_at_once() {
    for (shape, rot) in [
        ("pipe, healthy record", false),
        ("pipe, rotted record", true),
        ("link", false),
    ] {
        let root = scratch("claims-blocked");
        let given: Vec<u64> = (0..2)
            .map(|_| {
                allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap()
            })
            .collect();
        assert_eq!(given, [CLOCK, CLOCK + 1], "{shape}");
        let claims_path = root.join(ledger::CLAIMS);
        let victim = root.join("victim");
        std::fs::rename(&claims_path, &victim).unwrap();
        if shape == "link" {
            std::os::unix::fs::symlink(&victim, &claims_path).unwrap();
        } else {
            // `mkfifo` is the external program, run from a test only.
            let made = std::process::Command::new("mkfifo")
                .arg(&claims_path)
                .status()
                .expect("mkfifo ran");
            assert!(made.success(), "{shape}: mkfifo");
        }
        let path = root.join(ledger::LEDGER);
        if rot {
            let mut bytes = std::fs::read(&path).unwrap();
            let len = bytes.len();
            bytes[len - 60] ^= 1;
            std::fs::write(&path, &bytes).unwrap();
        }
        let ledger_before = std::fs::read(&path).unwrap();
        let victim_before = std::fs::read(&victim).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let asked = root.clone();
        std::thread::spawn(move || {
            let _ignored = tx.send(allocate_at(
                &asked,
                Mode::Spot,
                By::Api,
                kind_of(Vendor::Zerodha),
                CLOCK,
            ));
        });
        let refused = rx
            .recv_timeout(std::time::Duration::from_secs(20))
            .unwrap_or_else(|_| panic!("{shape}: no answer in 20 s, the allocation waits"))
            .unwrap_err();
        assert_eq!(refused.code(), "runs_ledger_io", "{shape}: {refused}");
        assert!(
            refused.to_string().contains("runs.claims"),
            "{shape}: names the claims: {refused}"
        );
        assert_eq!(
            std::fs::read(&path).unwrap(),
            ledger_before,
            "{shape}: no record written"
        );
        assert_eq!(
            std::fs::read(&victim).unwrap(),
            victim_before,
            "{shape}: untouched"
        );
    }
}

/// IA-10c repair 3 (the second agent's repair-round-3 probe P6): AN ID A
/// RECEIPT CANNOT CARRY IS REFUSED BEFORE THE VENDOR IS ASKED. A store whose
/// GDFL journal names run 4,294,967,294 seeds an unborn ledger past it, and
/// the next landing takes 4,294,967,295, the largest a receipt's 32-bit run
/// field holds; the landing after it would take 4,294,967,296 and is refused
/// `runs_ledger_entry`, naming the id and the field, and so is every later
/// one (each id stays spent). Before repair round 3 such a landing stored its
/// bars with no kept answer for every window.
#[test]
fn a_landing_id_a_receipt_cannot_carry_is_refused_before_the_vendor_is_asked() {
    let root = scratch("wide");
    let journal = crate::gdfl_import::journal_path(&root);
    std::fs::create_dir_all(journal.parent().unwrap()).unwrap();
    let largest = u64::from(u32::MAX);
    std::fs::write(
        &journal,
        format!("run {} import options cli 4242 {CLOCK}\n", largest - 1),
    )
    .unwrap();
    let last = allocate_at(&root, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
    assert_eq!(last, largest, "the largest id a receipt holds is given");
    for wider in [largest + 1, largest + 2] {
        let refused = allocate_at(
            &root,
            Mode::Repair,
            By::Api,
            kind_of(Vendor::Zerodha),
            CLOCK,
        )
        .unwrap_err();
        assert_eq!(refused.code(), "runs_ledger_entry", "{refused}");
        let said = refused.to_string();
        assert!(
            said.contains(&format!("run {wider} is past {largest}"))
                && said.contains("32-bit run field (offset 60)")
                && said.contains("nothing is asked of the vendor"),
            "{said}"
        );
        assert_eq!(ledger_ids(&root).last(), Some(&wider), "the id stays spent");
    }
}

// --------------------------------------------- the landing's run on lines --

/// The landing's id is every line's `run=` (D-6260): the gap ledger's and the
/// repair journal's, through either door, never the sink's run or 0; and the
/// scope is put back when the landing returns.
#[test]
fn a_landing_writes_its_own_run_on_every_ledger_line() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    // A day missing its first five minutes: a `gap` line.
    let short: Vec<String> = (560..930).map(|m| candle(on, m, 0, price(m))).collect();
    let body = format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        short.join(",")
    );
    let root = scratch("lines");
    let (rows, _) = answered(&body);
    let landed = land(&root, &rows, window, Door::FromWindow, 4_242_424_242);
    assert!(landed.ingested.bars_committed > 0, "{landed:?}");
    let gaps = ledger_text(&root, "gaps");
    let gap = gaps
        .lines()
        .find(|line| line.starts_with("gap NSE-INDEX-ZZIDX 2024-04-23 "))
        .unwrap_or_else(|| panic!("{gaps}"));
    assert!(gap.ends_with(" run=4242424242"), "{gap}");
    let derives: Vec<&str> = gaps.lines().filter(|l| l.starts_with("derive ")).collect();
    assert!(!derives.is_empty(), "{gaps}");
    assert!(
        derives.iter().all(|l| l.ends_with(" run=4242424242")),
        "{derives:?}"
    );
    assert_eq!(
        super::super::request_minutes::run_ordinal(),
        0,
        "the scope is put back"
    );

    // THE REPAIR DOOR'S JOURNAL TOO: the morning lands into a month already
    // holding the afternoon, so the month is repaired to a new generation.
    let root = scratch("lines-repair");
    let afternoon: Vec<String> = (700..930).map(|m| candle(on, m, 0, price(m))).collect();
    let later = format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        afternoon.join(",")
    );
    let (rows, _) = answered(&later);
    let _ = land(&root, &rows, window, Door::FromWindow, 7);
    let (rows, _) = answered(&kite_body(&[on], &[]));
    let repaired = land(&root, &rows, window, Door::RepairWindow, 8_888);
    assert!(repaired.ingested.bars_committed > 0, "{repaired:?}");
    let journal = ledger_text(&root, "repairs");
    let repair = journal
        .lines()
        .find(|line| line.starts_with("repair NSE-INDEX-ZZIDX 1min 2024-04 "))
        .unwrap_or_else(|| panic!("{journal}"));
    assert!(repair.ends_with(" run=8888"), "{repair}");
    // Every line of the repair (the minute month and each derived rung it
    // refolded) names the landing's run, never the sink's.
    for line in journal.lines() {
        assert!(line.ends_with(" run=8888"), "{line}");
    }
    // A landing whose day a repair filled says so with its run; a vendor
    // landing never says `origin=store`.
    let gaps = ledger_text(&root, "gaps");
    let filled = gaps
        .lines()
        .rfind(|line| line.starts_with("gap NSE-INDEX-ZZIDX 2024-04-23 "))
        .unwrap_or_else(|| panic!("{gaps}"));
    assert!(filled.contains(" why=repaired "), "{filled}");
    assert!(filled.ends_with(" run=8888"), "{filled}");
    assert!(!gaps.contains("origin=store"), "{gaps}");
}

/// `origin=store` (D-6261): a landing whose origin is another store's bars
/// (`cli zerodha-repair --from-store`) writes it on the `repaired` line, and
/// the verifier's own ledger reader reads it as such (IA-ZV-11).
#[test]
fn a_repair_from_another_store_says_origin_store() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let root = scratch("origin");
    let afternoon: Vec<String> = (700..930).map(|m| candle(on, m, 0, price(m))).collect();
    let (rows, _) = answered(&format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        afternoon.join(",")
    ));
    let _ = land(&root, &rows, window, Door::FromWindow, 7);
    let (rows, _) = answered(&kite_body(&[on], &[]));
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window,
        granularity: crate::vendor::Granularity::Minute1,
    };
    let plan = Plan {
        calendar: crate::calendar::Runtime::default(),
        cash_schedule: None,
        columns: crate::csv::Columns::Gdfl,
        request: &request,
        encoding: spec().timestamps,
        scale: crate::http::DECODED_PRICE_SCALE,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
        run: 99,
    };
    let origin = format!("{}{}", ingest::STORE_ORIGIN, "/somewhere/else");
    let done = ingest::repair_window(&rows, "ZZIDX", &origin, &root, plan, "from a scratch store");
    assert!(done.bars_committed > 0, "{done:?}");
    let gaps = ledger_text(&root, "gaps");
    let filled = gaps
        .lines()
        .rfind(|line| line.starts_with("gap NSE-INDEX-ZZIDX 2024-04-23 "))
        .unwrap_or_else(|| panic!("{gaps}"));
    assert!(
        filled.contains(" why=repaired ") && filled.ends_with(" run=99 origin=store"),
        "{filled}"
    );
    let read = crate::zerodha_verify::GapLedger::read(&root).unwrap();
    let line = read.gap("NSE-INDEX-ZZIDX", on).unwrap();
    assert!(line.origin_store, "{line:?}");
    assert!(
        read.faults.is_empty(),
        "the field is one its readers pass over"
    );
}

// ------------------------------------------------------------- IA-RK-07 --

/// IA-RK-07 (its half): a window landed through EITHER door is kept and
/// spooled, the receipt's rows-refused field equals the landing's named
/// refusals (one candle off the grid, two that disagree at one stamp: three),
/// each refused row is kept with its cause, and the door byte says which door.
#[test]
fn both_landing_doors_keep_and_count_their_refusals() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let extra = vec![
        // Off the minute grid.
        candle(on, 600, 30, price(600)),
        // Two candles at 10:30 that disagree with the one already there:
        // every candle of that stamp is refused (D-3351).
        candle(on, 630, 0, price(630) + 7),
    ];
    let body = kite_body(&[on], &extra);
    for door in [Door::FromWindow, Door::RepairWindow] {
        let root = scratch("doors");
        let hook = RunHook::new(&root, 31_337, Vendor::Zerodha, 1);
        let (landing, hooked) = land_and_hook(&root, &hook, &body, window, door);
        assert_eq!(landing.rows_refused(), 3, "{door:?}: {:?}", landing.refused);
        let causes: Vec<crate::fetch::RowCause> =
            landing.refused.iter().map(|row| row.cause).collect();
        assert_eq!(
            causes
                .iter()
                .filter(|c| **c == crate::fetch::RowCause::OffGrid)
                .count(),
            1,
            "{causes:?}"
        );
        assert_eq!(
            causes
                .iter()
                .filter(|c| **c == crate::fetch::RowCause::Conflicting)
                .count(),
            2,
            "{causes:?}"
        );
        let outcome = hooked.kept.unwrap();
        let receipt = outcome.first.receipt;
        assert_eq!(receipt.rows_refused, 3, "{door:?}");
        assert_eq!(receipt.rows_refused, landing.rows_refused());
        assert_eq!(receipt.door, door);
        assert_eq!(receipt.run, 31_337);
        assert_eq!(receipt.kept, Kept::Kept);
        assert_eq!(receipt.rules_version, ingest::LANDING_RULES);
        assert_eq!(
            u64::from(receipt.rows_decoded),
            375 + 2,
            "every decoded row, refused ones included"
        );
        let pack = response_keep::Pack {
            vendor: Vendor::Zerodha,
            key: &key(),
            timeframe: Timeframe::MINUTE_1,
        };
        let kept_rows =
            response_keep::refused_rows(&root, pack, on.year_month().unwrap(), 0, &receipt)
                .unwrap();
        assert_eq!(kept_rows.len(), 3, "each refused row kept with its cause");
        let appended = hooked.spooled.expect("Zerodha spools").unwrap();
        assert!(appended.records > 0, "{door:?}: {appended:?}");
        assert_eq!(hooked.not_spooled, None);
        assert!(
            root.join("verify/spool/31337").is_dir(),
            "the spool directory is the run's id"
        );
    }
}

/// §8 Z16 (with IA-9): a window landed through the repair door is kept
/// (`door=2`) and verified like any other: after the repair the day reads
/// `match` cold, over every one of its minutes.
#[test]
fn a_repaired_window_is_kept_and_verified() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let root = scratch("z16");
    // The afternoon is held, landed before any answer was kept (a month of
    // the store's history); the morning can only arrive as a repair.
    let afternoon: Vec<String> = (700..930).map(|m| candle(on, m, 0, price(m))).collect();
    let (rows, _) = answered(&format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        afternoon.join(",")
    ));
    let first = land(&root, &rows, window, Door::FromWindow, 11);
    assert_eq!(first.ingested.bars_committed, 230);
    let hook = RunHook::new(&root, 12, Vendor::Zerodha, 1);
    let (landing, hooked) = land_and_hook(
        &root,
        &hook,
        &kite_body(&[on], &[]),
        window,
        Door::RepairWindow,
    );
    assert!(landing.ingested.bars_committed > 0, "{landing:?}");
    let receipt = hooked.kept.unwrap().first.receipt;
    assert_eq!(receipt.door, Door::RepairWindow);
    assert_eq!(receipt.rows_landed, 145, "the morning, added by the repair");
    assert!(hooked.spooled.unwrap().unwrap().records > 0);
    // A NEW GENERATION, so every older verdict of the month reads stale by
    // its generation (IA-9's `staleness`, IA-ZV-10).
    assert!(
        ledger_text(&root, "repairs")
            .contains("repair NSE-INDEX-ZZIDX 1min 2024-04 v=1 from_generation=0 to_generation=1"),
        "{}",
        ledger_text(&root, "repairs")
    );
    drop(hook);
    merge_all(&root, None, None).unwrap();
    // COLD: the kept answers are read back minute by minute.
    let writer = crate::zerodha_verify::spool::SpoolWriter::new(
        &root,
        13,
        store::verdict::Mode::ZerodhaCold,
        1,
    );
    let report = cold(
        &root,
        &ColdRequest {
            from: on,
            to: on,
            only: vec!["ZZIDX".to_owned()],
        },
        &writer,
        None,
    )
    .unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let minute: Vec<(Code, u64)> = report
        .by_rung
        .iter()
        .filter(|(rung, ..)| rung == "1min")
        .map(|(_, code, n)| (*code, *n))
        .collect();
    assert_eq!(minute, [(Code::Match, 1)], "{report:?}");
}

// ------------------------------------------------------------ end to end --

/// THE WHOLE ROAD (IA-10c, with IA-8 and IA-9): one synthetic month landed
/// through the pull's own door with the hook, one answer per day, then IA-9's
/// cold verification answers `match` for every day of every minute, and
/// never `no_response_kept` (D-6262).
#[test]
fn a_month_landed_through_the_pull_door_verifies_cold_as_match() {
    let month = YearMonth::new(2024, 4).unwrap();
    let days = regular_days(month);
    assert!(days.len() >= 18, "{days:?}");
    let root = scratch("month");
    let hook = RunHook::new(&root, 202_404, Vendor::Zerodha, 1);
    for on in &days {
        let window = Window::new(*on, *on).unwrap();
        let (landing, hooked) = land_and_hook(
            &root,
            &hook,
            &kite_body(&[*on], &[]),
            window,
            Door::FromWindow,
        );
        assert_eq!(landing.ingested.bars_committed, 375, "{on}");
        assert_eq!(landing.rows_refused(), 0);
        assert!(hooked.kept.is_ok(), "{on}");
        assert!(hooked.spooled.unwrap().is_ok(), "{on}");
    }
    drop(hook);
    let writer = crate::zerodha_verify::spool::SpoolWriter::new(
        &root,
        202_405,
        store::verdict::Mode::ZerodhaCold,
        1,
    );
    let report = cold(
        &root,
        &ColdRequest {
            from: days[0],
            to: days[days.len() - 1],
            only: vec!["ZZIDX".to_owned()],
        },
        &writer,
        None,
    )
    .unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    let minute: Vec<(Code, u64)> = report
        .by_rung
        .iter()
        .filter(|(rung, ..)| rung == "1min")
        .map(|(_, code, n)| (*code, *n))
        .collect();
    assert_eq!(
        minute,
        [(Code::Match, days.len() as u64)],
        "every day of every minute matches its kept answer: {report:?}"
    );
    assert_eq!(
        report.by_code[usize::from(Code::NoResponseKept.code())],
        0,
        "{report:?}"
    );
}

// ------------------------------------ held windows: the main path (D-6267) --

/// Every bar file's bytes under `root` (`.bin`, `.crc`, `.srt`, `.tix`,
/// `.prv`, every generation's too), the kept answers (`responses/`) and the
/// verdict spool (`verify/`) aside: what "no bar byte was written" compares.
fn bar_bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path != root.join(response_keep::KEEP_ROOT) && path != root.join("verify") {
                    stack.push(path);
                }
            } else if matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("bin" | "crc" | "srt" | "tix" | "prv")
            ) {
                let bytes = std::fs::read(&path).unwrap();
                out.push((path, bytes));
            }
        }
    }
    out.sort();
    out
}

/// IA-9's cold verification of ZZIDX over `from..=to`: each `1min` code
/// with its count of days.
fn cold_minutes_of(root: &Path, from: Day, to: Day, run: u64) -> Vec<(Code, u64)> {
    let writer = crate::zerodha_verify::spool::SpoolWriter::new(
        root,
        run,
        store::verdict::Mode::ZerodhaCold,
        1,
    );
    let report = cold(
        root,
        &ColdRequest {
            from,
            to,
            only: vec!["ZZIDX".to_owned()],
        },
        &writer,
        None,
    )
    .unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    report
        .by_rung
        .iter()
        .filter(|(rung, ..)| rung == "1min")
        .map(|(_, code, n)| (*code, *n))
        .collect()
}

/// NO-FRESH-ROOT correction (2), D-6267: a re-pull over minutes the store
/// already holds, byte for byte, is the MAIN path of a mode (i) re-pull into
/// the existing store. `BarFile::append` answers `AlreadyPresent` and writes
/// no bar byte; the hook keeps and spools the answer all the same, and the
/// receipt says which: `rows_landed` 0 with `rows_refused` 0 and no refusal
/// of the door, beside a decoded count of every row. Through both doors.
#[test]
fn a_window_over_identical_held_minutes_writes_no_bar_byte_and_is_kept_and_spooled() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let body = kite_body(&[on], &[]);
    for door in [Door::FromWindow, Door::RepairWindow] {
        let root = scratch("held-same");
        // HELD BEFORE ANY ANSWER WAS KEPT: the store's own history.
        let (rows, _) = answered(&body);
        let held = land(&root, &rows, window, Door::FromWindow, 40);
        assert_eq!(held.ingested.bars_committed, 375);
        assert!(!root.join(response_keep::KEEP_ROOT).exists());
        let before = bar_bytes(&root);
        assert!(
            before.len() >= 4,
            "the month and its derived rungs: {}",
            before.len()
        );

        let hook = RunHook::new(&root, 41, Vendor::Zerodha, 1);
        let (landing, hooked) = land_and_hook(&root, &hook, &body, window, door);
        assert_eq!(landing.ingested.bars_committed, 0, "{door:?}: {landing:?}");
        assert_eq!(landing.rows_refused(), 0, "{door:?}");
        assert_eq!(landing.ingested.door_kind, None, "{door:?}");
        assert!(
            landing.ingested.failures.is_empty(),
            "{door:?}: {:?}",
            landing.ingested.failures
        );
        assert!(
            bar_bytes(&root) == before,
            "{door:?}: a bar byte was written"
        );

        let receipt = hooked.kept.unwrap().first.receipt;
        assert_eq!(receipt.kept, Kept::Kept, "{door:?}");
        assert_eq!(receipt.run, 41);
        assert_eq!(receipt.door, door);
        assert_eq!(receipt.rows_decoded, 375);
        assert_eq!(receipt.rows_landed, 0, "AlreadyPresent: no row landed");
        assert_eq!(receipt.rows_refused, 0, "and none refused");
        let appended = hooked.spooled.expect("Zerodha spools").unwrap();
        assert!(appended.records > 0, "{door:?}: {appended:?}");
        assert!(root.join("verify/spool/41").is_dir());
        drop(hook);
        assert_eq!(
            cold_minutes_of(&root, on, on, 43),
            [(Code::Match, 1)],
            "{door:?}: the kept answer matches the held minutes"
        );
    }
}

/// NO-FRESH-ROOT correction (2) and (3), D-6267: a re-pull over held minutes
/// that DIFFER (a back-adjusted basis, FM-479) is refused by name and never
/// overwrites a held bar: no bar byte is written, the landing names the month
/// and the held file it restates, and the answer is kept and spooled all the
/// same, so IA-9 reads the difference from the kept body (`value_mismatch`)
/// rather than not knowing of it (`no_response_kept`). The
/// append door and the repair door refuse alike: a held bar changes only as a
/// new generation after the operator's yes, never through either door here.
#[test]
fn a_window_over_differing_held_minutes_is_refused_by_name_and_its_answer_is_kept() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let held_body = kite_body(&[on], &[]);
    // The same minutes, every price 3 paisa higher from 12:00 on.
    let differing: Vec<String> = (555..930)
        .map(|m| candle(on, m, 0, price(m) + if m >= 720 { 3 } else { 0 }))
        .collect();
    let differing = format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        differing.join(",")
    );
    for door in [Door::FromWindow, Door::RepairWindow] {
        let root = scratch("held-differs");
        let (rows, _) = answered(&held_body);
        assert_eq!(
            land(&root, &rows, window, Door::FromWindow, 50)
                .ingested
                .bars_committed,
            375
        );
        let before = bar_bytes(&root);

        let hook = RunHook::new(&root, 51, Vendor::Zerodha, 1);
        let (landing, hooked) = land_and_hook(&root, &hook, &differing, window, door);
        assert_eq!(landing.ingested.bars_committed, 0, "{door:?}: {landing:?}");
        // REFUSED BY NAME: one failure, of this month, naming the held bar
        // file and the store's own word for a held record offered otherwise.
        let month = on.year_month().unwrap();
        let named: Vec<&ingest::Failure> = landing
            .ingested
            .failures
            .iter()
            .filter(|f| f.instrument == "ZZIDX" && f.month == Some(month))
            .collect();
        assert_eq!(named.len(), 1, "{door:?}: {landing:?}");
        assert!(
            named[0].why.contains("1min/2024-04.bin") && named[0].why.contains("restates"),
            "{door:?}: {named:?}"
        );
        // MEASURED, AND NAMED AS A LIMIT (L-IA10C-07): the window door does
        // not type this refusal (`door_kind` stays `None`; the member path
        // carries words), and the receipt has no byte for the door's
        // outcome, so `rows_landed` 0 and `rows_refused` 0 read alike for an
        // `AlreadyPresent` and for this. What tells them apart is the named
        // failure above and IA-9's verdict below.
        assert_eq!(landing.ingested.door_kind, None, "{door:?}");
        assert_eq!(landing.rows_refused(), 0, "{door:?}");
        assert!(
            bar_bytes(&root) == before,
            "{door:?}: a held bar was overwritten"
        );

        let receipt = hooked.kept.unwrap().first.receipt;
        assert_eq!(receipt.kept, Kept::Kept, "{door:?}: the answer is kept");
        assert_eq!(receipt.run, 51);
        assert_eq!(receipt.door, door);
        assert_eq!(receipt.rows_decoded, 375);
        assert_eq!(receipt.rows_landed, 0, "{door:?}: nothing landed");
        assert_eq!(receipt.rows_refused, 0, "{door:?}");
        let appended = hooked.spooled.expect("Zerodha spools").unwrap();
        assert!(appended.records > 0, "{door:?}: {appended:?}");
        drop(hook);
        // THE KEPT ANSWER IS READ AGAINST THE HELD BARS: the day differs, by
        // value, and is never `no_response_kept` (the answer was kept).
        assert_eq!(
            cold_minutes_of(&root, on, on, 53),
            [(Code::ValueMismatch, 1)],
            "{door:?}: the held day differs from its kept answer"
        );
    }
}

/// THE END-TO-END TEST'S HELD-MONTH TWIN (D-6267): a month the store held
/// before any answer was kept, re-landed day by day through the pull's own
/// door with the hook and the identical answers, writes no bar byte, and
/// IA-9's cold verification then answers `match` for every day of every
/// minute, never `no_response_kept`: what a mode (i) re-pull into the
/// existing store proves.
#[test]
fn a_held_month_relanded_with_identical_answers_writes_no_bar_byte_and_verifies_cold_as_match() {
    let month = YearMonth::new(2024, 4).unwrap();
    let days = regular_days(month);
    let root = scratch("held-month");
    for on in &days {
        let (rows, _) = answered(&kite_body(&[*on], &[]));
        let window = Window::new(*on, *on).unwrap();
        assert_eq!(
            land(&root, &rows, window, Door::FromWindow, 60)
                .ingested
                .bars_committed,
            375,
            "{on}"
        );
    }
    assert_eq!(
        cold_minutes_of(&root, days[0], days[days.len() - 1], 61),
        [(Code::NoResponseKept, days.len() as u64)],
        "before the re-pull nothing was kept"
    );
    let before = bar_bytes(&root);
    let hook = RunHook::new(&root, 62, Vendor::Zerodha, 1);
    for on in &days {
        let window = Window::new(*on, *on).unwrap();
        let (landing, hooked) = land_and_hook(
            &root,
            &hook,
            &kite_body(&[*on], &[]),
            window,
            Door::FromWindow,
        );
        assert_eq!(landing.ingested.bars_committed, 0, "{on}");
        assert_eq!(landing.ingested.door_kind, None, "{on}");
        let receipt = hooked.kept.unwrap().first.receipt;
        assert_eq!((receipt.rows_landed, receipt.rows_refused), (0, 0), "{on}");
        assert!(hooked.spooled.unwrap().is_ok(), "{on}");
    }
    drop(hook);
    assert!(bar_bytes(&root) == before, "a held bar byte was written");
    assert_eq!(
        cold_minutes_of(&root, days[0], days[days.len() - 1], 63),
        [(Code::Match, days.len() as u64)],
        "every day of every minute matches its kept answer"
    );
}

// --------------------------------------------- the hook's named refusals --

/// A vendor with no verifier is kept and never spooled, said so; a run id
/// past the receipt's 32 bits is refused by name, never truncated into
/// another run's; a rung with no store directory keeps nothing.
#[test]
fn the_hook_says_why_it_kept_or_spooled_nothing() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let body = kite_body(&[on], &[]);

    let root = scratch("noverifier");
    let hook = RunHook::new(&root, 5, Vendor::Dhan, 1);
    let (_, hooked) = land_and_hook(&root, &hook, &body, window, Door::FromWindow);
    assert!(hooked.kept.is_ok());
    assert!(hooked.spooled.is_none());
    assert_eq!(hooked.not_spooled, Some("no_verifier_for_vendor"));
    assert!(!root.join("verify/spool").exists());

    let root = scratch("wide");
    let hook = RunHook::new(&root, u64::from(u32::MAX) + 1, Vendor::Zerodha, 1);
    let (_, hooked) = land_and_hook(&root, &hook, &body, window, Door::FromWindow);
    assert!(
        matches!(hooked.kept, Err(NotKept::RunTooWide { run }) if run == u64::from(u32::MAX) + 1)
    );
    assert_eq!(hooked.not_spooled, Some("run_too_wide"));
    assert!(!root.join(response_keep::KEEP_ROOT).exists());

    let root = scratch("rung");
    let hook = RunHook::new(&root, 6, Vendor::Zerodha, 1);
    let (rows, answered) = answered(&body);
    let landing = DoorLanding::default();
    let hooked = hook.after_landing(&Landed {
        key: &key(),
        granularity: crate::vendor::Granularity::Tick,
        asked: window,
        answered: &answered,
        rows: &rows.rows,
        landing: &landing,
        door: Door::FromWindow,
        encoding: spec().timestamps,
    });
    assert_eq!(crate::vendor::Granularity::Tick.store_timeframe(), None);
    assert!(matches!(hooked.kept, Err(NotKept::RungUnstored)));
    assert_eq!(hooked.not_spooled, Some("rung_unstored"));
    assert!(!root.join(response_keep::KEEP_ROOT).exists());

    // A pack whose receipts are full keeps nothing more and spools nothing:
    // the keeper's refusal is the hook's, by name.
    let root = scratch("full");
    let hook = RunHook::new(&root, 7, Vendor::Zerodha, 1);
    let dir = response_keep::pack_dir(
        &root,
        response_keep::Pack {
            vendor: Vendor::Zerodha,
            key: &key(),
            timeframe: Timeframe::MINUTE_1,
        },
    )
    .unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    // A RECEIPT FILE THAT IS A DIRECTORY cannot be appended: refused by name.
    let names = response_keep::names(&dir, on.year_month().unwrap(), 0);
    std::fs::create_dir_all(&names.receipts).unwrap();
    let (_, hooked) = land_and_hook(&root, &hook, &body, window, Door::FromWindow);
    assert!(
        matches!(hooked.kept, Err(NotKept::Refused(_))),
        "{hooked:?}"
    );
    assert_eq!(hooked.not_spooled, Some("not_kept"));
}

// --------------------------------------------------- FM-163: the claim --

/// FM-163: the vendor's landing claim. An api landing holds it shared (two
/// api requests at once), a `cli zerodha-repair --from-store` exclusive, and
/// the one refused names the holder as it wrote itself, never waiting.
#[test]
fn zerodha_repair_refuses_while_a_pull_run_holds_the_vendor() {
    let root = scratch("claim");
    let spot = claim(
        &root,
        Vendor::Zerodha,
        crate::gdfl_run::locks::Hold::Shared,
        "api pid 1 run 41 (pull-spot)",
    )
    .unwrap();
    let fno = claim(
        &root,
        Vendor::Zerodha,
        crate::gdfl_run::locks::Hold::Shared,
        "api pid 1 run 42 (pull-fno)",
    )
    .unwrap();
    let refused = claim(
        &root,
        Vendor::Zerodha,
        crate::gdfl_run::locks::Hold::Exclusive,
        "cli pid 2 zerodha-repair",
    )
    .unwrap_err();
    assert_eq!(refused.code(), "run_busy", "{refused}");
    assert!(
        refused.to_string().contains("api pid 1 run 42 (pull-fno)"),
        "the holder is named: {refused}"
    );
    // Another vendor's claim is its own.
    assert!(
        claim(
            &root,
            Vendor::Dhan,
            crate::gdfl_run::locks::Hold::Exclusive,
            "cli"
        )
        .is_ok()
    );
    drop((spot, fno));
    let repair = claim(
        &root,
        Vendor::Zerodha,
        crate::gdfl_run::locks::Hold::Exclusive,
        "cli pid 2 run 43 (zerodha-repair)",
    )
    .unwrap();
    let pull = claim(
        &root,
        Vendor::Zerodha,
        crate::gdfl_run::locks::Hold::Shared,
        "api pid 1 (pull-spot)",
    )
    .unwrap_err();
    assert!(
        pull.to_string()
            .contains("cli pid 2 run 43 (zerodha-repair)"),
        "{pull}"
    );
    drop(repair);
    // A store root that is gone takes no claim.
    assert!(
        claim(
            &root.join("gone"),
            Vendor::Zerodha,
            crate::gdfl_run::locks::Hold::Shared,
            "api"
        )
        .is_err()
    );
}

/// The second agent's follow-up on round 1 (D-6268): shared holders of one
/// vendor's landing claim noting at once never splice their notes. Eight
/// holders of one store, released together by a barrier, each note who they
/// are, thirty rounds; after every round the lock file is one whole-width
/// record and reads back as exactly one holder's note, never pieces of two,
/// and the exclusive taker it refuses names that holder whole. Before D-6268
/// each note truncated the file and then wrote, and two notes at once spliced
/// (18 of 30 rounds in the second agent's probe).
///
/// What a holder is named as is its note with its trailing blanks dropped:
/// the record pads a note with blanks, so blanks at a note's end cannot be
/// told from the padding and the reader (`locks::head_of`) drops both.
/// Holder 0's note ends in a blank, so the comparison is against each note as
/// the reader names it (repair round 2: compared with the untrimmed notes,
/// the test failed whenever holder 0 happened to note last, 30 of 30 runs at
/// the second agent's load, with no splice in any of them), and holder 0 is
/// made the last to note once at the end so that case is always taken.
#[test]
fn shared_holders_noting_at_once_never_splice_their_notes() {
    use crate::gdfl_run::locks::{self, Hold};
    const HOLDERS: usize = 8;
    let root = scratch("claim");
    let lock = root.join(landing_lock(Vendor::Zerodha));
    // Notes of very different lengths, so a splice of two reads as neither.
    let notes: Vec<String> = (0..HOLDERS)
        .map(|n| format!("api pid 1 run {n} (pull-spot) {}", "=".repeat(n * 97)))
        .collect();
    let named_as: Vec<&str> = notes.iter().map(|note| note.trim_ascii_end()).collect();
    assert!(
        notes[0].ends_with(' ') && !named_as[0].ends_with(' '),
        "the fixture holds a note that ends in a blank: {:?}",
        notes[0]
    );
    let claims: Vec<VendorClaim> = notes
        .iter()
        .map(|note| claim(&root, Vendor::Zerodha, Hold::Shared, note).unwrap())
        .collect();
    for round in 0..30 {
        let barrier = std::sync::Barrier::new(HOLDERS);
        std::thread::scope(|scope| {
            for (held, note) in claims.iter().zip(&notes) {
                let barrier = &barrier;
                scope.spawn(move || {
                    barrier.wait();
                    held.note(note);
                });
            }
        });
        let bytes = std::fs::read(&lock).unwrap();
        assert_eq!(bytes.len(), locks::PROBE_READ, "round {round}: one record");
        let named = locks::head_of(&std::fs::File::open(&lock).unwrap());
        assert!(
            named_as.contains(&named.as_str()),
            "round {round}: the lock names no one holder whole: {named:?}"
        );
        let refused = claim(&root, Vendor::Zerodha, Hold::Exclusive, "cli").unwrap_err();
        assert_eq!(refused.code(), "run_busy", "{refused}");
        assert!(
            refused.to_string().ends_with(&format!("is held: {named}")),
            "round {round}: {refused}"
        );
    }
    // THE HOLDER WHOSE NOTE ENDS IN A BLANK, LAST: named whole, its blank
    // dropped with the padding, whatever order the rounds above took.
    claims[0].note(&notes[0]);
    let named = locks::head_of(&std::fs::File::open(&lock).unwrap());
    assert_eq!(named, named_as[0]);
    let refused = claim(&root, Vendor::Zerodha, Hold::Exclusive, "cli").unwrap_err();
    assert!(
        refused
            .to_string()
            .ends_with(&format!("is held: {}", named_as[0])),
        "{refused}"
    );
}

/// The second agent's repair-round-2 follow-up (probe A2; D-6269): a sharer
/// that lets go while others still hold leaves the note of one that still
/// holds, its latest, so the exclusive taker refused after it never names a
/// holder that has finished. Before, the file kept the last note written:
/// with run 100 `pull-spot` holding, the refusal named run 200 `pull-fno`,
/// which had dropped. The last holder to let go writes nothing, and the
/// exclusive taker then takes the claim; another store's lock file is its
/// own.
#[test]
fn a_sharer_that_lets_go_leaves_the_note_of_one_that_still_holds() {
    use crate::gdfl_run::locks::Hold;
    let root = scratch("claim");
    let other = scratch("claim");
    let refusal = |root: &Path| {
        claim(
            root,
            Vendor::Zerodha,
            Hold::Exclusive,
            "cli pid 2 (zerodha-repair)",
        )
        .unwrap_err()
        .to_string()
    };
    let held_by = |root: &Path, who: &str| {
        let refused = refusal(root);
        assert!(
            refused.ends_with(&format!("is held: {who}")),
            "named {who:?}? {refused}"
        );
    };
    let spot = claim(
        &root,
        Vendor::Zerodha,
        Hold::Shared,
        "api pid 1 run 100 (pull-spot)",
    )
    .unwrap();
    let fno = claim(
        &root,
        Vendor::Zerodha,
        Hold::Shared,
        "api pid 1 run 200 (pull-fno)",
    )
    .unwrap();
    let elsewhere = claim(
        &other,
        Vendor::Zerodha,
        Hold::Shared,
        "api pid 1 run 900 (pull-spot)",
    )
    .unwrap();
    held_by(&root, "api pid 1 run 200 (pull-fno)");
    // THE LATER SHARER FINISHES FIRST: the one still holding is named.
    drop(fno);
    held_by(&root, "api pid 1 run 100 (pull-spot)");
    // A third sharer, noted twice (at its claim and with its run id); the
    // first sharer notes again. Each holder's LATEST note is what is
    // written back for it.
    let repair = claim(
        &root,
        Vendor::Zerodha,
        Hold::Shared,
        "api pid 1 (pull-repair)",
    )
    .unwrap();
    repair.note("api pid 1 run 300 (pull-repair)");
    spot.note("api pid 1 run 101 (pull-spot)");
    held_by(&root, "api pid 1 run 101 (pull-spot)");
    drop(spot);
    held_by(&root, "api pid 1 run 300 (pull-repair)");
    // Another store's holders are its own: dropping one writes nothing here.
    drop(elsewhere);
    held_by(&root, "api pid 1 run 300 (pull-repair)");
    // THE LAST TO LET GO WRITES NOTHING, and the claim is free.
    drop(repair);
    let taken = claim(
        &root,
        Vendor::Zerodha,
        Hold::Exclusive,
        "cli pid 2 run 400 (zerodha-repair)",
    )
    .unwrap();
    let refused = claim(
        &root,
        Vendor::Zerodha,
        Hold::Shared,
        "api pid 1 (pull-spot)",
    )
    .unwrap_err();
    assert!(
        refused
            .to_string()
            .ends_with("is held: cli pid 2 run 400 (zerodha-repair)"),
        "{refused}"
    );
    drop(taken);
    assert!(claim(&other, Vendor::Zerodha, Hold::Exclusive, "cli").is_ok());
}

/// D-6268: a note is one record of exactly the probe's width. A note longer
/// than that is cut at a character boundary, never inside a character; an
/// empty one is padding only, which the reader reads as no holder; and a
/// short note after a long one replaces the whole of it, with no truncation
/// first.
#[test]
fn a_note_is_one_whole_width_record_cut_at_a_character() {
    use crate::gdfl_run::locks::{self, Hold};
    let width = locks::PROBE_READ;
    // A three-byte character straddling the width.
    let long = format!("{}\u{20ac}", "=".repeat(width - 1));
    let record = note_record(&long);
    assert_eq!(record.len(), width);
    assert_eq!(&record[..width - 1], "=".repeat(width - 1).as_bytes());
    assert_eq!(
        record[width - 1],
        b' ',
        "the cut character is dropped whole"
    );
    assert_eq!(note_record(""), vec![b' '; width]);
    let exact = "=".repeat(width);
    assert_eq!(note_record(&exact), exact.as_bytes());
    let root = scratch("claim");
    let held = claim(&root, Vendor::Zerodha, Hold::Shared, &long).unwrap();
    let lock = root.join(landing_lock(Vendor::Zerodha));
    let named = || locks::head_of(&std::fs::File::open(&lock).unwrap());
    assert_eq!(named(), "=".repeat(width - 1));
    held.note("api pid 1 run 2 (pull-spot)");
    assert_eq!(named(), "api pid 1 run 2 (pull-spot)");
    assert_eq!(std::fs::read(&lock).unwrap().len(), width);
    held.note("");
    assert_eq!(named(), "", "an empty note names no holder");
}

/// Gate 11 rule 3, docs/07 law 2 (D-6268): a run's first-day map is
/// pre-sized to the series its caller plans to land (none, one, the F&O
/// universe's 213, a walk's thousands), so the first window of each planned
/// series inserts into room that is already there.
#[test]
fn a_run_hook_is_presized_to_the_series_it_plans() {
    let root = scratch("claim");
    for planned in [0_usize, 1, 213, 4_096] {
        let hook = RunHook::new(&root, 1, Vendor::Zerodha, planned);
        assert!(
            hook.series_room() >= planned,
            "{planned} planned, room for {}",
            hook.series_room()
        );
    }
}

// ------------------------------------------- IA-GR-41: the gap ledger --

/// IA-GR-41, the gap ledger's half (D-4888, closes L-IA-38): two writers of
/// `audit/zerodha.gaps` at once, each a landing appending its day lines,
/// never join or lose a line; the append takes `audit/zerodha.gaps.lock`;
/// a torn tail a crash left is closed with a newline by the next landing's
/// append, read as one malformed line and named by the reader.
#[test]
fn two_writers_of_an_audit_ledger_never_join_a_line() {
    let on = day(2024, 4, 23);
    let window = Window::new(on, on).unwrap();
    let root = scratch("gr41");
    // A crash's torn tail.
    std::fs::create_dir_all(root.join("audit")).unwrap();
    std::fs::write(
        root.join("audit/zerodha.gaps"),
        b"gap NSE-INDEX-ZZTORN 2024",
    )
    .unwrap();
    // Each body is short by five minutes on a day of its own, so every
    // landing writes one `gap` line; 2 x 40 landings of two symbols.
    let threads: Vec<_> = ["ZZA", "ZZB"]
        .into_iter()
        .map(|symbol| {
            let root = root.clone();
            std::thread::spawn(move || {
                for n in 0..40_u32 {
                    let short: Vec<String> =
                        (560..930).map(|m| candle(on, m, 0, price(m))).collect();
                    let body = format!(
                        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
                        short.join(",")
                    );
                    let (rows, _) = answered(&body);
                    let request = BarRequest {
                        instrument_id: String::new(),
                        listing: Listing::Index,
                        window,
                        granularity: crate::vendor::Granularity::Minute1,
                    };
                    let plan = Plan {
                        calendar: crate::calendar::Runtime::default(),
                        cash_schedule: None,
                        columns: crate::csv::Columns::Gdfl,
                        request: &request,
                        encoding: spec().timestamps,
                        scale: crate::http::DECODED_PRICE_SCALE,
                        vendor: Vendor::Zerodha,
                        exchange: "NSE",
                        segment: "INDEX",
                        contract: None,
                        run: u64::from(n) + 1,
                    };
                    let _ = ingest::from_window(&rows, symbol, "https://kite.invalid", &root, plan);
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    assert!(
        root.join("audit/zerodha.gaps.lock").is_file(),
        "the audit lock was taken"
    );
    let text = ledger_text(&root, "gaps");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0], "gap NSE-INDEX-ZZTORN 2024",
        "the fragment stays one line"
    );
    let gap_lines: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|l| l.starts_with("gap NSE-INDEX-ZZA ") || l.starts_with("gap NSE-INDEX-ZZB "))
        .collect();
    assert_eq!(gap_lines.len(), 80, "no line lost: {text}");
    for line in &gap_lines {
        assert_eq!(line.matches("gap ").count(), 1, "never joined: {line}");
        assert!(line.contains(" why=vendor_empty run="), "{line}");
    }
    let read = crate::zerodha_verify::GapLedger::read(&root).unwrap();
    assert_eq!(read.faults, [1], "the torn fragment is named, once");
}

// ---------------------------------------------------------------- timings --

/// **IA-10c timings** (run with `--ignored --nocapture`; set
/// `BRUTEX_IA10C_TIMING_DIR` to time on another drive): a landing's run id
/// from a growing runs ledger (n=1,000); beside 50 MB of claims (repair
/// round 3: the seed's one whole pass, then a run id n=200, then the whole
/// pass a damage event takes n=20); the vendor's landing claim taken shared
/// (n=1,000), one gap-ledger append through the audit ledger lock (n=1,000,
/// RA-087's bench with its max) and the hook after one landed day (keep and
/// spool, n=200). Printed for `docs/06-limits.md` and the package report,
/// never asserted: a timing on a shared machine is evidence, not a gate.
#[test]
#[ignore = "timings, printed"]
fn ia10c_timings() {
    use std::fmt::Write as _;
    fn pct(mut v: Vec<std::time::Duration>) -> String {
        v.sort_unstable();
        let n = v.len();
        format!(
            "n={n} p50={:?} p99={:?} max={:?}",
            v[n / 2],
            v[n * 99 / 100],
            v[n - 1]
        )
    }
    let base = std::env::var_os("BRUTEX_IA10C_TIMING_DIR")
        .map_or_else(|| scratch("timings"), PathBuf::from);
    std::fs::create_dir_all(&base).unwrap();
    let root = base.join(format!("ia10c-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    println!("timing under {}", root.display());
    let mut took = Vec::new();
    for _ in 0..1_000 {
        let began = std::time::Instant::now();
        allocate(&root, Mode::Spot, By::Api, "zerodha").unwrap();
        took.push(began.elapsed());
    }
    println!(
        "landing run id (runs ledger, 1..1,000 records): {}",
        pct(took)
    );
    // THE CLAIMS AT 50 MB (repair round 3, D-6269): an id reads only the
    // claims' tail, so it costs the same beside 50 MB of them; the pass a
    // seed or a damaged last record takes reads them whole, once per event.
    let big = base.join(format!("ia10c-claims-{}", std::process::id()));
    std::fs::create_dir_all(big.join("audit")).unwrap();
    let mut text = String::with_capacity(52 << 20);
    let mut id = CLOCK;
    while text.len() < 50 << 20 {
        let record = ledger::Entry {
            id,
            mode: Mode::Spot.word().to_owned(),
            kind: kind_of(Vendor::Zerodha).to_owned(),
            by: By::Api.word().to_owned(),
            pid: 4242,
            started: CLOCK,
            resumes: 0,
        }
        .record()
        .unwrap();
        let body = std::str::from_utf8(&record[..114]).unwrap().trim_end();
        let crc = std::str::from_utf8(&record[114..126]).unwrap();
        writeln!(text, "run {body} {crc}").unwrap();
        id += 1;
    }
    std::fs::write(big.join(ledger::CLAIMS), &text).unwrap();
    let began = std::time::Instant::now();
    let seeded = allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
    let seed_took = began.elapsed();
    assert_eq!(seeded, id, "seeded past the last claim");
    println!(
        "seed past {} claims lines ({} bytes, one whole pass): {seed_took:?}",
        id - CLOCK,
        text.len()
    );
    let mut took = Vec::new();
    for _ in 0..200 {
        let began = std::time::Instant::now();
        allocate_at(&big, Mode::Spot, By::Api, kind_of(Vendor::Zerodha), CLOCK).unwrap();
        took.push(began.elapsed());
    }
    println!(
        "landing run id beside 50 MB of claims (the tail read): {}",
        pct(took)
    );
    let mut took = Vec::new();
    for _ in 0..20 {
        let began = std::time::Instant::now();
        let largest = ledger::largest_claimed_run(&big).unwrap();
        took.push(began.elapsed());
        assert!(largest >= seeded);
    }
    println!(
        "the claims' whole pass at 50 MB (a damage event): {}",
        pct(took)
    );
    let mut took = Vec::new();
    for n in 0..1_000 {
        let began = std::time::Instant::now();
        let held = claim(
            &root,
            Vendor::Zerodha,
            crate::gdfl_run::locks::Hold::Shared,
            &format!("api pid 1 run {n}"),
        )
        .unwrap();
        took.push(began.elapsed());
        drop(held);
    }
    println!("vendor landing claim, shared: {}", pct(took));
    let mut took = Vec::new();
    for n in 0..1_000 {
        let line = vec![format!(
            "gap NSE-INDEX-ZZT{n} 2024-04-23 v=1 rung=1min owed=375 held=370 missing=5 first_missing=09:15 runs=1 longest=5 longest_at=09:15 why=vendor_empty run=1"
        )];
        let began = std::time::Instant::now();
        crate::ingest::audit_ledger::append(&root, Vendor::Zerodha, "gaps", &line).unwrap();
        took.push(began.elapsed());
    }
    println!(
        "gap ledger append through the audit lock (1..1,000 lines): {}",
        pct(took)
    );
    let month = YearMonth::new(2024, 4).unwrap();
    let days = regular_days(month);
    let hook = RunHook::new(&root, 77, Vendor::Zerodha, 1);
    let mut took = Vec::new();
    for n in 0..200_usize {
        let on = days[n % days.len()];
        let window = Window::new(on, on).unwrap();
        let body = kite_body(&[on], &[]);
        let (rows, answered) = answered(&body);
        let landing = land(&root, &rows, window, Door::FromWindow, 77);
        let key = key();
        let began = std::time::Instant::now();
        let hooked = hook.after_landing(&Landed {
            key: &key,
            granularity: crate::vendor::Granularity::Minute1,
            asked: window,
            answered: &answered,
            rows: &rows.rows,
            landing: &landing,
            door: Door::FromWindow,
            encoding: spec().timestamps,
        });
        took.push(began.elapsed());
        assert!(hooked.kept.is_ok());
    }
    println!(
        "hook after one landed day (keep + verify + spool): {}",
        pct(took)
    );
}

// ------------------------------------------------------------ NIFTY first --

/// One series a test lands: its store symbol, segment, contract and key.
struct Named {
    symbol: &'static str,
    segment: &'static str,
    listing: Listing,
    key: InstrumentKey,
}

impl Named {
    fn index(symbol: &'static str) -> Self {
        Self {
            symbol,
            segment: "INDEX",
            listing: Listing::Index,
            key: InstrumentKey::index(Exchange::Nse, symbol).unwrap(),
        }
    }

    fn option(symbol: &'static str, kind: brutex_core::instrument::Kind) -> Self {
        Self {
            symbol,
            segment: "FNO",
            listing: Listing::Derivative,
            key: InstrumentKey {
                exchange: Exchange::Nse,
                segment: brutex_core::instrument::Segment::Fno,
                underlying: brutex_core::symbol::Symbol::new(symbol).unwrap(),
                kind,
            },
        }
    }
}

/// Lands `body` for `named` through the append door under `hook`'s run and
/// hands it to the hook, as the api's door does.
fn land_named(
    root: &Path,
    hook: &RunHook,
    named: &Named,
    body: &str,
    on: Day,
) -> (DoorLanding, Hooked) {
    let window = Window::new(on, on).unwrap();
    let decoded = crate::http::decode_body(body, &spec(), named.listing).unwrap();
    let mut answered = response_keep::Answered::of(decoded, body.to_owned(), 200);
    let rows = std::mem::take(&mut answered.window);
    let request = BarRequest {
        instrument_id: String::new(),
        listing: named.listing,
        window,
        granularity: crate::vendor::Granularity::Minute1,
    };
    let plan = Plan {
        calendar: crate::calendar::Runtime::default(),
        cash_schedule: None,
        columns: crate::csv::Columns::Gdfl,
        request: &request,
        encoding: spec().timestamps,
        scale: crate::http::DECODED_PRICE_SCALE,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: named.segment,
        contract: brutex_core::instrument::Contract::of(named.key.kind),
        run: hook.run(),
    };
    let landing =
        ingest::from_window_with_refusals(&rows, named.symbol, "https://kite.invalid", root, plan);
    let hooked = hook.after_landing(&Landed {
        key: &named.key,
        granularity: crate::vendor::Granularity::Minute1,
        asked: window,
        answered: &answered,
        rows: &rows.rows,
        landing: &landing,
        door: Door::FromWindow,
        encoding: spec().timestamps,
    });
    (landing, hooked)
}

/// The 1min verdicts IA-9's cold verification gives `symbol` on `on`.
fn cold_minutes(root: &Path, symbol: &str, on: Day, run: u64) -> Vec<(Code, u64)> {
    let writer = crate::zerodha_verify::spool::SpoolWriter::new(
        root,
        run,
        store::verdict::Mode::ZerodhaCold,
        1,
    );
    let report = cold(
        root,
        &ColdRequest {
            from: on,
            to: on,
            only: vec![symbol.to_owned()],
        },
        &writer,
        None,
    )
    .unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    report
        .by_rung
        .iter()
        .filter(|(rung, ..)| rung == "1min")
        .map(|(_, code, n)| (*code, *n))
        .collect()
}

/// A Kite-shaped body of every minute `on`'s session of record owes
/// (a special session's recorded windows included), invented prices.
fn session_body(on: Day) -> (String, u32) {
    let number = i64::from(on.days_from_epoch());
    let crate::calendar::DayKind::Open(session) =
        crate::session::with_recorded_window(number, crate::calendar::sourced_kind_of(number))
    else {
        panic!("{on} is not a session of record");
    };
    let rows: Vec<String> = (0..1_440_u16)
        .filter(|m| session.expects(*m))
        .map(|m| candle(on, u32::from(m), 0, price(u32::from(m))))
        .collect();
    let count = u32::try_from(rows.len()).unwrap();
    (
        format!(
            "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
            rows.join(",")
        ),
        count,
    )
}

/// NIFTY FIRST (the operator's order): one NIFTY spot day, one NIFTY options
/// day and one special session (the Diwali Muhurat of 12 Nov 2023, a Sunday
/// whose recorded window is short of 375 minutes), each landed through the
/// pull's door with the hook under one run; then the same shared path on one
/// BANKNIFTY day and one far out-of-the-money (illiquid) NIFTY strike of a few
/// prints. Spot days verify cold `match`; an option's answer is kept in its
/// own contract's pack (receipt with the run's id) and its verdicts spooled.
/// Prices are invented (near 11,111.00); no vendor row is in the repository.
#[test]
fn nifty_first_a_spot_day_an_options_day_and_a_special_session() {
    use brutex_core::instrument::{Expiry, Kind, OptionSide};
    let root = scratch("nifty");
    let hook = RunHook::new(&root, 1_700_000_001, Vendor::Zerodha, 1);
    let regular = day(2024, 4, 23);
    let muhurat = day(2023, 11, 12);

    // NIFTY spot, a regular day.
    let nifty = Named::index("NIFTY");
    let (landing, hooked) = land_named(&root, &hook, &nifty, &kite_body(&[regular], &[]), regular);
    assert_eq!(landing.ingested.bars_committed, 375, "{landing:?}");
    assert_eq!(hooked.kept.unwrap().first.receipt.run, 1_700_000_001);
    assert!(hooked.spooled.unwrap().unwrap().records > 0);

    // NIFTY spot, the special session.
    let (body, owed) = session_body(muhurat);
    assert!(owed > 0 && owed < 375, "a short special session: {owed}");
    let (landing, hooked) = land_named(&root, &hook, &nifty, &body, muhurat);
    assert_eq!(
        landing.ingested.bars_committed,
        usize::try_from(owed).unwrap(),
        "{landing:?}"
    );
    assert!(hooked.kept.is_ok() && hooked.spooled.unwrap().is_ok());

    // NIFTY options: one contract day, and one illiquid strike of 3 prints.
    let expiry = Expiry::new(2024, 4, 25).unwrap();
    let atm = Named::option(
        "NIFTY",
        Kind::Option {
            expiry,
            strike: brutex_core::price::Paisa::from_raw(1_111_100),
            side: OptionSide::Call,
        },
    );
    let (landing, hooked) = land_named(&root, &hook, &atm, &kite_body(&[regular], &[]), regular);
    assert!(landing.ingested.bars_committed > 0, "{landing:?}");
    let receipt = hooked.kept.unwrap().first.receipt;
    assert_eq!(
        (receipt.run, receipt.door),
        (1_700_000_001, Door::FromWindow)
    );
    assert!(hooked.spooled.unwrap().is_ok());
    let dir = response_keep::pack_dir(
        &root,
        response_keep::Pack {
            vendor: Vendor::Zerodha,
            key: &atm.key,
            timeframe: Timeframe::MINUTE_1,
        },
    )
    .unwrap();
    let contract = brutex_core::instrument::Contract::of(atm.key.kind).unwrap();
    assert!(
        dir.ends_with(format!("FNO/NIFTY/{}/1min", contract.as_str())),
        "{}",
        dir.display()
    );
    let far = Named::option(
        "NIFTY",
        Kind::Option {
            expiry,
            strike: brutex_core::price::Paisa::from_raw(9_999_900),
            side: OptionSide::Put,
        },
    );
    let sparse: Vec<String> = [600_u32, 721, 899]
        .iter()
        .map(|m| candle(regular, *m, 0, price(*m)))
        .collect();
    let (landing, hooked) = land_named(
        &root,
        &hook,
        &far,
        &format!(
            "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
            sparse.join(",")
        ),
        regular,
    );
    assert_eq!(landing.ingested.bars_committed, 3, "{landing:?}");
    assert_eq!(hooked.kept.unwrap().first.receipt.rows_landed, 3);

    // BANKNIFTY, the same path.
    let bank = Named::index("BANKNIFTY");
    let (landing, hooked) = land_named(&root, &hook, &bank, &kite_body(&[regular], &[]), regular);
    assert_eq!(landing.ingested.bars_committed, 375);
    assert!(hooked.spooled.unwrap().is_ok());
    drop(hook);

    assert_eq!(
        cold_minutes(&root, "NSE-INDEX-NIFTY", regular, 91),
        [(Code::Match, 1)]
    );
    assert_eq!(
        cold_minutes(&root, "NSE-INDEX-NIFTY", muhurat, 92),
        [(Code::Match, 1)]
    );
    assert_eq!(
        cold_minutes(&root, "NSE-INDEX-BANKNIFTY", regular, 93),
        [(Code::Match, 1)]
    );
}
