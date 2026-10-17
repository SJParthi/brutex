//! R3-35 (the audit's IA-00, RA-142): the importer's file split by its seams
//! into the day loop, the journal, the filing, the census and the causes,
//! MOVED and not changed. Two proofs, one per half of that sentence:
//!
//! - [`the_split_importer_writes_the_same_bytes`]: one import of invented
//!   NIFTY, BANKNIFTY, stock and option days, a special session among them,
//!   writes the bytes the unsplit importer wrote — every month, checksum and
//!   time index, the census, the journal, the day ledger, the lineage and lot
//!   ledgers and every provenance record, less the wall clock. The digests
//!   below were taken by this same test on the unsplit file (`feat/db-fix`
//!   at `d7eb0ecf`, the test file and its one `mod` line added) and again on
//!   the split, and are equal.
//! - [`the_split_moves_lines_and_changes_none`]: the non-blank lines of the
//!   six files are the unsplit file's lines as a multiset, apart from
//!   module, use and visibility lines, and it counts the lines that differ
//!   (zero).
//!
//! Every fixture value is invented (D-2800): no vendor row is quoted, and
//! the prices are made up at run time.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use sha2::Digest as _;

use super::*;
use crate::gdfl_archive::Archive;
use crate::gdfl_cm::CmKind;
use crate::gdfl_fixtures::{Method, as_compared, csv, mon, put, row, scratch, zip};
use crate::gdfl_nfo::NfoZips;

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

/// An invented build commit in the shape a provenance record takes.
const COMMIT: &str = "0123abcd0123abcd0123abcd0123abcd0123abcd";

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// A regular session (Monday 1 April 2024).
fn regular() -> Day {
    day(2024, 4, 1)
}

/// A special session (Saturday 1 February 2020, `calendar::SPECIAL_SESSIONS`).
fn special() -> Day {
    day(2020, 2, 1)
}

fn ddmmyyyy(d: Day) -> String {
    format!("{:02}{:02}{:04}", d.day(), d.month(), d.year())
}

/// An invented index file: a pre-open row, three rows in one second, a late
/// row, rows across the session and one after the close.
fn index_file(stem: &str, d: Day, base: u32) -> Vec<u8> {
    let p = |v: u32| format!("{}.{:02}", base + v / 100, v % 100);
    let mut rows = vec![row(stem, d, NINE_FIFTEEN - 300, &p(1), 0, 0)];
    rows.push(row(stem, d, NINE_FIFTEEN, &p(5), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(55), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(15), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 2, &p(20), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 1, &p(30), 0, 0));
    for s in (60..22_500).step_by(613) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, &p(s % 700), 0, 0));
    }
    rows.push(row(stem, d, CLOSE + 60, &p(7), 0, 0));
    csv(&rows)
}

/// An invented share file: an untraded and a traded row each step.
fn stock_file(stem: &str, d: Day) -> Vec<u8> {
    let mut rows = Vec::new();
    for s in (0..22_500).step_by(907) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, "613.40", 0, 0));
        let ltp = format!("61{}.05", s % 10);
        rows.push(row(
            stem,
            d,
            NINE_FIFTEEN + s,
            &ltp,
            u64::from(s % 7 + 1),
            0,
        ));
    }
    csv(&rows)
}

/// An invented option file: traded and untraded rows, an open interest that
/// moves, `step` seconds apart.
fn option_file(ticker: &str, d: Day, ltp: &str, step: u32) -> Vec<u8> {
    let name = format!("{ticker}.NFO");
    let mut rows = Vec::new();
    for s in (0..22_500).step_by(usize::try_from(step).unwrap()) {
        let ltq = u64::from(s % 3) * 15;
        rows.push(row(
            &name,
            d,
            NINE_FIFTEEN + s,
            ltp,
            ltq,
            700 + u64::from(s),
        ));
    }
    csv(&rows)
}

/// The capital-market entries of one day of `kind`.
fn cm_entries(kind: CmKind, d: Day) -> Vec<(String, Vec<u8>)> {
    let folder = crate::gdfl_cm::day_folder_name(kind, d);
    match kind {
        CmKind::Indices => vec![
            (format!("{folder}/"), Vec::new()),
            (
                format!("{folder}/NIFTY 50.NSE_IDX.csv"),
                index_file("NIFTY 50.NSE_IDX", d, 1_234),
            ),
            (
                format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
                index_file("NIFTY BANK.NSE_IDX", d, 4_321),
            ),
        ],
        CmKind::Stocks => vec![
            (
                format!("{folder}/RELIANCE.NSE.csv"),
                stock_file("RELIANCE.NSE", d),
            ),
            (format!("{folder}/SBIN.NSE.csv"), stock_file("SBIN.NSE", d)),
        ],
    }
}

/// The options entries of one day: NIFTY strikes both sides, a BANKNIFTY and
/// a share option, an illiquid strike with one trade, and one file whose
/// price has three decimals (refused by name, so the day closes done with a
/// refusal).
fn option_entries(d: Day) -> Vec<(String, Vec<u8>)> {
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let entry = |ticker: &str| format!("{folder}\\Options\\{ticker}.NFO.csv");
    let mut out = Vec::new();
    for (i, strike) in [21_000_u32, 21_050, 21_100].into_iter().enumerate() {
        for side in ["CE", "PE"] {
            let ticker = format!("NIFTY04APR24{strike}{side}");
            let ltp = format!("{}.{:02}", 31 + i, i * 5);
            out.push((entry(&ticker), option_file(&ticker, d, &ltp, 1_201)));
        }
    }
    let bank = "BANKNIFTY10APR2447000CE";
    out.push((entry(bank), option_file(bank, d, "77.15", 1_499)));
    let share = "RELIANCE25APR242900CE";
    out.push((entry(share), option_file(share, d, "12.35", 2_003)));
    let idle = "NIFTY04APR2425000CE";
    let mut rows: Vec<String> = (0..4)
        .map(|s| {
            row(
                &format!("{idle}.NFO"),
                d,
                NINE_FIFTEEN + s * 60,
                "0.55",
                0,
                300,
            )
        })
        .collect();
    rows.push(row(
        &format!("{idle}.NFO"),
        d,
        NINE_FIFTEEN + 900,
        "0.60",
        15,
        315,
    ));
    out.push((entry(idle), csv(&rows)));
    let bad = "NIFTY04APR2422975CE";
    out.push((entry(bad), option_file(bad, d, "101.505", 3_001)));
    out
}

/// The vendor's two trees under `root`: `zips/cm.zip` holding each
/// capital-market day zip, and `zips/options/<year>.zip` holding each
/// options day zip, as the import reads them.
fn world(root: &Path) {
    let day_zip = |entries: &[(String, Vec<u8>)]| {
        let members: Vec<(&str, &[u8], Method)> = entries
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
            .collect();
        zip(&members)
    };
    let month = |d: Day| format!("{}_{:04}", mon(d), d.year());
    let mut cm: Vec<(String, Vec<u8>)> = Vec::new();
    for d in [special(), regular()] {
        cm.push((
            format!(
                "INDICES/{:04}/{}/GFDLCM_INDICES_TICK_{}.zip",
                d.year(),
                month(d),
                ddmmyyyy(d)
            ),
            day_zip(&cm_entries(CmKind::Indices, d)),
        ));
    }
    let d = regular();
    cm.push((
        format!(
            "STOCKS/{:04}/{}/GFDLCM_STOCK_TICK_{}.zip",
            d.year(),
            month(d),
            ddmmyyyy(d)
        ),
        day_zip(&cm_entries(CmKind::Stocks, d)),
    ));
    let stored: Vec<(&str, &[u8], Method)> = cm
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(root, "zips/cm.zip", &zip(&stored));
    let options = [(
        format!("{}/GFDLNFO_TICK_{}.zip", month(d), ddmmyyyy(d)),
        day_zip(&option_entries(d)),
    )];
    let stored: Vec<(&str, &[u8], Method)> = options
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(
        root,
        &format!("zips/options/{:04}.zip", d.year()),
        &zip(&stored),
    );
}

/// Every file under `dir`, by its path under it.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(dir).unwrap().to_path_buf();
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

/// Each file of the store at `store` as SHA-256 hex of its bytes less the
/// wall clock ([`as_compared`]; a generation record's promotion time and
/// the seal over it zeroed, as `gdfl_import_tests::store_bytes` does).
fn digests(store: &Path) -> Vec<(String, String)> {
    tree(store)
        .into_iter()
        .map(|(rel, bytes)| {
            let mut bytes = as_compared(&rel, bytes);
            if rel
                .extension()
                .is_some_and(|ext| ext == store::generation::EXTENSION)
            {
                for record in bytes.chunks_mut(64) {
                    for at in (48..56).chain(60..64) {
                        if let Some(byte) = record.get_mut(at) {
                            *byte = 0;
                        }
                    }
                }
            }
            let mut hex = String::with_capacity(64);
            for b in sha2::Sha256::digest(&bytes) {
                let _infallible = write!(hex, "{b:02x}");
            }
            (rel.to_string_lossy().replace('\\', "/"), hex)
        })
        .collect()
}

/// The six runs of the proof, in order, into one store: NIFTY and BANKNIFTY
/// spot over a span holding the special session, the weekend and the
/// holiday before the regular day, and the regular day; the shares; the
/// options; and the options again (a re-run, which skips its done day and
/// writes nothing).
fn import_all(root: &Path, store: &Path) -> Vec<String> {
    let cm_path = root.join("zips/cm.zip");
    let nfo_path = root.join("zips/options");
    let origin = |kind: SourceKind, path: &Path| Origin {
        started: 1_790_000_000,
        ..Origin::measure(kind, path, Some(COMMIT))
    };
    let cm_origin = origin(SourceKind::Zip, &cm_path);
    let nfo_origin = origin(SourceKind::Zip, &nfo_path);
    let archive = Archive::open(&cm_path).unwrap();
    let options = NfoZips::new(&nfo_path);
    let run = |kind, (from, to): (Day, Day)| Run {
        kind,
        from,
        to,
        only: &[],
        store_root: store,
    };
    let with = |origin| RunOptions {
        origin: Some(origin),
        ..RunOptions::default()
    };
    let mut said = Vec::new();
    for (kind, span) in [
        (ImportKind::Indices, (day(2020, 1, 31), day(2020, 2, 3))),
        (ImportKind::Indices, (day(2024, 3, 28), regular())),
        (ImportKind::Stocks, (regular(), regular())),
    ] {
        let report = run_cm_from(&archive, &run(kind, span), with(&cm_origin)).unwrap();
        said.push(said_of(
            &format!("{kind:?} {}..{}", span.0, span.1),
            &report,
        ));
    }
    for _ in 0..2 {
        let span = (regular(), regular());
        let report =
            run_nfo_from(&options, &run(ImportKind::Options, span), with(&nfo_origin)).unwrap();
        said.push(said_of(&format!("Options {}..{}", span.0, span.1), &report));
    }
    said
}

/// What a run's report says, less the words of its failures (which name the
/// scratch directory): its day, file, row and second counts and the days it
/// closed each way.
fn said_of(label: &str, r: &Report) -> String {
    format!(
        "{label}: imported {} skipped {} closed {} missing {:?} refused {} files {} \
         skipped {} refused {} rows {} ltq0 {} late {} unresolved {} outside {} seconds {} \
         committed {} failures {} with-refusals {:?} incomplete {:?} refused {:?} flags {} \
         lineage {} lots {} provenance {}",
        r.days_imported,
        r.days_skipped,
        r.days_closed,
        r.days_missing,
        r.days_refused,
        r.files,
        r.files_skipped,
        r.files_refused,
        r.rows,
        r.ltq_zero_dropped,
        r.late_rows,
        r.late_unresolved,
        r.outside_session,
        r.seconds,
        r.seconds_committed,
        r.failures.len(),
        r.closed_with_refusals,
        r.closed_incomplete,
        r.closed_refused,
        r.flags_lines,
        r.lineage_lines,
        r.lot_lines,
        r.provenance_records
    )
}

/// What each run said, taken on the unsplit importer and on the split.
const SAID: [&str; 5] = [
    "Indices 2020-01-31..2020-02-03: imported 1 skipped 0 closed 1 missing [Day { year: 2020, month: 1, day: 31 }, Day { year: 2020, month: 2, day: 3 }] refused 0 files 2 skipped 1 refused 0 rows 88 ltq0 0 late 2 unresolved 0 outside 4 seconds 78 committed 78 failures 0 with-refusals [] incomplete [] refused [] flags 2 lineage 0 lots 0 provenance 2",
    "Indices 2024-03-28..2024-04-01: imported 1 skipped 0 closed 3 missing [Day { year: 2024, month: 3, day: 28 }] refused 0 files 2 skipped 1 refused 0 rows 88 ltq0 0 late 2 unresolved 0 outside 4 seconds 78 committed 78 failures 0 with-refusals [] incomplete [] refused [] flags 2 lineage 0 lots 0 provenance 2",
    "Stocks 2024-04-01..2024-04-01: imported 1 skipped 0 closed 0 missing [] refused 0 files 2 skipped 0 refused 0 rows 100 ltq0 50 late 0 unresolved 0 outside 0 seconds 50 committed 50 failures 0 with-refusals [] incomplete [] refused [] flags 1 lineage 0 lots 0 provenance 2",
    "Options 2024-04-01..2024-04-01: imported 1 skipped 0 closed 0 missing [] refused 0 files 9 skipped 0 refused 1 rows 147 ltq0 56 late 0 unresolved 0 outside 0 seconds 91 committed 91 failures 1 with-refusals [Day { year: 2024, month: 4, day: 1 }] incomplete [] refused [] flags 1 lineage 49 lots 9 provenance 9",
    "Options 2024-04-01..2024-04-01: imported 0 skipped 1 closed 0 missing [] refused 0 files 0 skipped 0 refused 0 rows 0 ltq0 0 late 0 unresolved 0 outside 0 seconds 0 committed 0 failures 0 with-refusals [] incomplete [] refused [] flags 0 lineage 0 lots 0 provenance 0",
];

/// Every file the runs leave and its digest, taken on the unsplit importer
/// and on the split.
const LEFT: [(&str, &str); 67] = [
    (
        "bars/gdfl/NSE/CASH/RELIANCE/1s/2024-04.bin",
        "3a3bb183760e4e4ca59a7c5af3a35bf854d624200fb0e9026da106bf1dce4432",
    ),
    (
        "bars/gdfl/NSE/CASH/RELIANCE/1s/2024-04.crc",
        "b98708cef2b9c60d065f7faacd6e89d9821e0185239c614b1fc5297aeb57570d",
    ),
    (
        "bars/gdfl/NSE/CASH/RELIANCE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/CASH/RELIANCE/1s/2024-04.prv",
        "cd3a877fb035f9dcbae336e9544dcc0ead267943a6d9c98fe66035de8ae457ad",
    ),
    (
        "bars/gdfl/NSE/CASH/SBIN/1s/2024-04.bin",
        "c2c3d8d0629d5f775382eedea38d53e186f48fd78accacfc6220126012c5824d",
    ),
    (
        "bars/gdfl/NSE/CASH/SBIN/1s/2024-04.crc",
        "b98708cef2b9c60d065f7faacd6e89d9821e0185239c614b1fc5297aeb57570d",
    ),
    (
        "bars/gdfl/NSE/CASH/SBIN/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/CASH/SBIN/1s/2024-04.prv",
        "cd3a877fb035f9dcbae336e9544dcc0ead267943a6d9c98fe66035de8ae457ad",
    ),
    (
        "bars/gdfl/NSE/FNO/BANKNIFTY/2024-04-10-4700000-CE/1s/2024-04.bin",
        "43524e497b1d7dac2893caed24feeccad445bf31c5b93ac963e9be9302a9164a",
    ),
    (
        "bars/gdfl/NSE/FNO/BANKNIFTY/2024-04-10-4700000-CE/1s/2024-04.crc",
        "9342f6944fc93100bc5c122d8ec905f879c5d2804f7ed4af3f7eb969b40eb297",
    ),
    (
        "bars/gdfl/NSE/FNO/BANKNIFTY/2024-04-10-4700000-CE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/BANKNIFTY/2024-04-10-4700000-CE/1s/2024-04.prv",
        "a09e946439116fc9740e86a3fd0b39d26653013293e00d6728b5b91bd84fe9b2",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-CE/1s/2024-04.bin",
        "41acca9b3fd0d1ad271d153f3be61c37398dbf5c5394125ddbba6ee32b0f557d",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-CE/1s/2024-04.crc",
        "6e13e27f25f8774dd9e7182924a3ccb2ef743b9051905cd5f0106bd519879545",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-CE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-CE/1s/2024-04.prv",
        "9da6934f9ad5a61389b50d1c1c807cff202914b60c0300249747af15fbb1bb8f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-PE/1s/2024-04.bin",
        "41acca9b3fd0d1ad271d153f3be61c37398dbf5c5394125ddbba6ee32b0f557d",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-PE/1s/2024-04.crc",
        "6e13e27f25f8774dd9e7182924a3ccb2ef743b9051905cd5f0106bd519879545",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-PE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2100000-PE/1s/2024-04.prv",
        "9da6934f9ad5a61389b50d1c1c807cff202914b60c0300249747af15fbb1bb8f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-CE/1s/2024-04.bin",
        "3e760bc47fa6f994449b1edf785ca1fb68e8bdfa2ffcc75602cba4a9ac0d8e9f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-CE/1s/2024-04.crc",
        "1bab33b795dd01ce92cadfbff261b0e051ea56e2da0a1b665803fceec5ad4912",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-CE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-CE/1s/2024-04.prv",
        "9da6934f9ad5a61389b50d1c1c807cff202914b60c0300249747af15fbb1bb8f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-PE/1s/2024-04.bin",
        "3e760bc47fa6f994449b1edf785ca1fb68e8bdfa2ffcc75602cba4a9ac0d8e9f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-PE/1s/2024-04.crc",
        "1bab33b795dd01ce92cadfbff261b0e051ea56e2da0a1b665803fceec5ad4912",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-PE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2105000-PE/1s/2024-04.prv",
        "9da6934f9ad5a61389b50d1c1c807cff202914b60c0300249747af15fbb1bb8f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-CE/1s/2024-04.bin",
        "6f8de0d6203e7afbbbd4bb4b326e0c93d2781bb130a6a48c5ac3beb471693f5d",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-CE/1s/2024-04.crc",
        "dcc95aec52d08ef56d524868290c94d9d39c17d0add026fc23a9f7646f540a5a",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-CE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-CE/1s/2024-04.prv",
        "9da6934f9ad5a61389b50d1c1c807cff202914b60c0300249747af15fbb1bb8f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-PE/1s/2024-04.bin",
        "6f8de0d6203e7afbbbd4bb4b326e0c93d2781bb130a6a48c5ac3beb471693f5d",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-PE/1s/2024-04.crc",
        "dcc95aec52d08ef56d524868290c94d9d39c17d0add026fc23a9f7646f540a5a",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-PE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2110000-PE/1s/2024-04.prv",
        "9da6934f9ad5a61389b50d1c1c807cff202914b60c0300249747af15fbb1bb8f",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2500000-CE/1s/2024-04.bin",
        "bc62ef81a157900cc8e50909c7ee12281643ad0c7c6053a38989b6993ffba007",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2500000-CE/1s/2024-04.crc",
        "1ab0a4a4a64336fa38284829bb013417dbcc6f339d020f7ab87aa944fc8ddb04",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2500000-CE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/NIFTY/2024-04-04-2500000-CE/1s/2024-04.prv",
        "5213dc2a68e4f91554680585a051c38b0c274a91a32b77ada803e018c5dad918",
    ),
    (
        "bars/gdfl/NSE/FNO/RELIANCE/2024-04-25-290000-CE/1s/2024-04.bin",
        "98ff7dcc6039b75f337e2ce20043aef3bc774e1777c57ce159a20ae612738522",
    ),
    (
        "bars/gdfl/NSE/FNO/RELIANCE/2024-04-25-290000-CE/1s/2024-04.crc",
        "b136c7bd913ad4c7bc4ae152fc54e8973f9056299c6e51901560535ba0a7db82",
    ),
    (
        "bars/gdfl/NSE/FNO/RELIANCE/2024-04-25-290000-CE/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/FNO/RELIANCE/2024-04-25-290000-CE/1s/2024-04.prv",
        "70f94ed624f91aa9e167c590ee3f44229b02648c7d98939a9cbf2f41f804e20a",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2020-02.bin",
        "f8235bf9abfa263dfd4e6a3ebae134260d9f85d8b8a1825b2be32a49e1f69353",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2020-02.crc",
        "3f108ccfd91394ee4246563eaa9e69751f27a5f689b1e9aed88d4b6ce8a23174",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2020-02.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2020-02.prv",
        "aa8fd4cbd5c35f8f4b121bd15cce77b8437ffe9d9b9da7378fb00dc56c28957f",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2024-04.bin",
        "7712e1f8934f824d0c800fc99f66d548e0532b1db213502340c4fce7a62f0435",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2024-04.crc",
        "d733dcc29fa6f7c0333965f4f3ca80bf1448a63ab6c6d8799aae8d0fff9758d8",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/INDEX/BANKNIFTY/1s/2024-04.prv",
        "388a2caddc70cb43371bf5c4a6803a7e19030fa4a123cb47d6f66ab2d9d2d1b1",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2020-02.bin",
        "dd12925ec88e7c0141151cb7f0ae1df98de408f451a2e078ecb68d5f7e2b46c7",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2020-02.crc",
        "a83a32e7742e703f92df61ad5b33cd19d9830d3c4a1c4ab4930a85292b286d2f",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2020-02.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2020-02.prv",
        "aa8fd4cbd5c35f8f4b121bd15cce77b8437ffe9d9b9da7378fb00dc56c28957f",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2024-04.bin",
        "c6749fae99c0e77e25a6d863c2c1efa8cc11858beb6af120694b41c74c22a30e",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2024-04.crc",
        "babee9129a00885ae06840a7e227bb429ff2236da1aa0d8ba700099f8f6c519c",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2024-04.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
    (
        "bars/gdfl/NSE/INDEX/NIFTY/1s/2024-04.prv",
        "388a2caddc70cb43371bf5c4a6803a7e19030fa4a123cb47d6f66ab2d9d2d1b1",
    ),
    (
        "imports/gdfl.dayflags",
        "4466ea7b57d0aaef4fab0a26ee4ee4921d97b205b730240a034a688e07377b0e",
    ),
    (
        "imports/gdfl.journal",
        "092de752457d5a24633f4ffb4c772e1ca82cf37a8bf251d34309a5a374b5f84d",
    ),
    (
        "imports/gdfl.lineage",
        "86ba5082e993fadf6350c09ac6c5a3ab83e0dd55dbbdabe81734493458ef7b17",
    ),
    (
        "imports/gdfl.lots",
        "37ea6b45af4709ef4a2412f52fd2e69d8d0c6099e7bb41456eca6529775135ad",
    ),
    (
        "imports/gdfl.scopes",
        "df4192bbddb0847da9fe2df1ff0e924c0c780396fbb87d82e313f7592e046993",
    ),
    (
        "manifest/gdfl.man",
        "3aacb0c212ff75436f24f4505a2cd723d28c3a7bb4ef21b03592c6f661eb125a",
    ),
    (
        "manifest/gdfl.man.lock",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ),
];

/// R3-35: THE SPLIT IMPORTER WRITES THE SAME BYTES. Six runs over invented
/// NIFTY, BANKNIFTY, share and option days (a special session, a weekend
/// and a holiday among them, a refused file, an illiquid strike and a
/// re-run) leave exactly the files, and the bytes, the unsplit importer
/// left: the months, the census, the journal, the day ledger, the lineage
/// and lot ledgers and the provenance records, less the wall clock.
#[test]
fn the_split_importer_writes_the_same_bytes() {
    let root = scratch("r3-35-bytes");
    world(&root);
    let store = root.join("store");
    let said = import_all(&root, &store);
    let left = digests(&store);
    assert_eq!(said, SAID, "what the runs said");
    let want: Vec<(String, String)> = LEFT
        .iter()
        .map(|(rel, hex)| ((*rel).to_owned(), (*hex).to_owned()))
        .collect();
    assert_eq!(left, want, "the files the runs left, and their bytes");
}

// ── the lines: moved, none changed ──────────────────────────────────────────

/// The unsplit file, `crates/pull/src/gdfl_import.rs` at `d7eb0ecf` (the head
/// of `feat/db-fix` the split was cut from): its git blob.
const BEFORE_BLOB: &str = "1c349b131e44b6a8551aad620311fbc46838ac78";

/// The unsplit file's non-blank lines less its module and use lines, each
/// with its visibility taken out ([`counted`]): how many, and the SHA-256 of
/// them sorted, one per line.
const BEFORE: (usize, &str) = (
    7_261,
    "30a2642ccc1a09d1aa8d2cccecc0aff369de7c33866bc6fa27204d1716ceb1c5",
);

/// What [`counted`] sets aside of the six files the split left, summed:
/// module lines (the `mod` declarations with their `#[path]`, `#[cfg]` and
/// doc lines, and each new file's `//!` header), use lines, and kept lines
/// that carried a visibility (the unsplit file gives 24, 23 and 154). Every
/// other line is one of [`BEFORE`]'s 7,261, so 0 lines differ.
const SET_ASIDE: (usize, usize, usize) = (54, 97, 311);

/// The six files as the split left them: the path under `src/`, the git
/// blob and the SHA-256 of the bytes.
const AFTER: [(&str, &str, &str); 6] = [
    (
        "gdfl_import.rs",
        "44ee3c9254a6ede2ad9374f20f1787717778d98f",
        "0e864f9317fb56469745034f8787e4b1c1684ad3f4bba8fe1c0a975b7d0a86cf",
    ),
    (
        "gdfl_import_causes.rs",
        "8866b39498c126c1d356dc8984560b884a5672dc",
        "fd176295a7490ee47b20598202fd4442ad7130666c29562f01129c618e8b52b2",
    ),
    (
        "gdfl_import_census.rs",
        "1762b2a7bd9b0e0af47e597a7a0f5fec60167d6e",
        "6bbfef351c8409306dc80beb0abe0459eb96cdab7efacf23e806a6566fad99aa",
    ),
    (
        "gdfl_import_day.rs",
        "3c28b1c296a76c3bab4860a75e5117fe436cc248",
        "b8d2b8dad44456f94b1bd0822e25bb0d5c7561d4ea05b795fcc06166ad7a9de4",
    ),
    (
        "gdfl_import_filing.rs",
        "d8c65e047228e1cb52b3c1de487989491e23b184",
        "cbbc783ec2b5cb7bf84c88ac57a1ad95c5b2f8cf502cba5177059cf5bd72efbe",
    ),
    (
        "gdfl_import_journal.rs",
        "d64ace8e822ab8acdcda467d43be9918113d2da2",
        "1eb96f76e48fa860de14d8a1643575ed83c7cf15fba6fc8e7412e0d03660125a",
    ),
];

/// What [`counted`] set aside of a file and what it kept.
#[derive(Debug, Default)]
struct Counted {
    /// Every kept line, its visibility taken out, and how many times.
    kept: BTreeMap<String, usize>,
    /// Module lines: a `mod` declaration with its attributes and docs, and a
    /// new file's `//!` header.
    module: usize,
    /// Use lines: a `use` declaration, every line of it, with its
    /// attributes.
    uses: usize,
    /// Kept lines whose visibility was taken out.
    visibility: usize,
}

impl Counted {
    fn add(&mut self, other: Self) {
        for (line, n) in other.kept {
            *self.kept.entry(line).or_default() += n;
        }
        self.module += other.module;
        self.uses += other.uses;
        self.visibility += other.visibility;
    }

    fn lines(&self) -> usize {
        self.kept.values().sum()
    }

    /// The SHA-256 of the kept lines, sorted, one per line.
    fn digest(&self) -> String {
        let mut hash = sha2::Sha256::new();
        for (line, n) in &self.kept {
            for _ in 0..*n {
                hash.update(line.as_bytes());
                hash.update(b"\n");
            }
        }
        let mut hex = String::with_capacity(64);
        for b in hash.finalize() {
            let _infallible = write!(hex, "{b:02x}");
        }
        hex
    }
}

/// `line` with a leading `pub`, `pub(crate)`, `pub(super)` or `pub(self)` of
/// its first token taken out, and its trailing white space.
fn unvisible(line: &str) -> String {
    let body = line.trim_start();
    let indent = &line[..line.len() - body.len()];
    for vis in ["pub(crate) ", "pub(super) ", "pub(self) ", "pub "] {
        if let Some(rest) = body.strip_prefix(vis) {
            return format!("{indent}{rest}").trim_end().to_owned();
        }
    }
    line.trim_end().to_owned()
}

/// The non-blank lines of `text`, less its module and use lines, each with
/// its visibility taken out. `new_file` sets aside the `//!` header a file
/// the split made opens with. An attribute or doc line belongs to the item
/// after it: to a `mod` or `use` declaration's count when it precedes one.
fn counted(text: &str, new_file: bool) -> Counted {
    let mut out = Counted::default();
    let mut header = new_file;
    let mut in_use = false;
    let mut held: Vec<&str> = Vec::new();
    let keep = |out: &mut Counted, line: &str| {
        let plain = unvisible(line);
        if plain != line.trim_end() {
            out.visibility += 1;
        }
        *out.kept.entry(plain).or_default() += 1;
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if header && t.starts_with("//!") {
            out.module += 1;
            continue;
        }
        header = false;
        if in_use {
            out.uses += 1;
            in_use = !t.contains(';');
            continue;
        }
        if t.starts_with("#[") || t.starts_with("///") {
            held.push(line);
            continue;
        }
        let plain = unvisible(t);
        if plain.starts_with("use ") {
            out.uses += 1 + held.len();
            held.clear();
            in_use = !t.contains(';');
            continue;
        }
        let is_mod = plain
            .strip_prefix("mod ")
            .and_then(|rest| rest.strip_suffix(';'))
            .is_some_and(|name| name.bytes().all(|b| b == b'_' || b.is_ascii_alphanumeric()));
        if is_mod {
            out.module += 1 + held.len();
            held.clear();
            continue;
        }
        for h in held.drain(..) {
            keep(&mut out, h);
        }
        keep(&mut out, line);
    }
    for h in held {
        keep(&mut out, h);
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(64);
    for b in sha2::Sha256::digest(bytes) {
        let _infallible = write!(hex, "{b:02x}");
    }
    hex
}

/// The blob `id` from the repository holding this crate, `None` where `git`
/// cannot be run, the tree holds no `.git` (`cargo-mutants` copies it
/// without one) or the history does not hold the blob.
fn blob(id: &str) -> Option<String> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(crate_dir)
        .args(["cat-file", "blob", id])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap())
}

/// R3-35: THE SPLIT MOVES LINES AND CHANGES NONE. The non-blank lines of the
/// six files the split left are the unsplit file's non-blank lines as a
/// multiset, apart from module lines (the five `mod` declarations, their
/// `#[path]` attributes and each new file's `//!` header), use lines and
/// visibility (a `pub(super)` an item, field or method needs to be reached
/// from the file beside it, taken out before comparing). It counts the lines
/// that differ (0) and pins what it set aside ([`SET_ASIDE`]).
///
/// The six files are read from the tree while they are the split's own
/// bytes, and from the history by their blobs once a later package edits
/// one; where neither holds them (no `git`, no `.git`, a history without the
/// split's commit) it says so and stops, as `store/tests/cited_commits.rs`
/// does. Where the history holds the unsplit file, it is counted too and
/// must give [`BEFORE`], so the pinned count is the file's and not a copy.
#[test]
fn the_split_moves_lines_and_changes_none() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut after = Counted::default();
    for (i, (name, blob_id, sha)) in AFTER.iter().enumerate() {
        let here = std::fs::read(src.join(name)).unwrap();
        let text = if sha256_hex(&here) == *sha {
            String::from_utf8(here).unwrap()
        } else if let Some(text) = blob(blob_id) {
            assert_eq!(sha256_hex(text.as_bytes()), *sha, "{name}: blob {blob_id}");
            text
        } else {
            println!(
                "SKIPPING: {name} is no longer the split's own bytes and its blob {blob_id} \
                 is not in this history, so the split cannot be counted here. The commit \
                 that made the split carries this proof."
            );
            return;
        };
        after.add(counted(&text, i > 0));
    }
    if let Some(before_text) = blob(BEFORE_BLOB) {
        let before = counted(&before_text, false);
        let mut gone: Vec<&str> = Vec::new();
        for (line, n) in &before.kept {
            let have = after.kept.get(line).copied().unwrap_or(0);
            if have < *n {
                gone.push(line);
            }
        }
        let mut new: Vec<&str> = Vec::new();
        for (line, n) in &after.kept {
            if before.kept.get(line).copied().unwrap_or(0) < *n {
                new.push(line);
            }
        }
        assert!(
            gone.is_empty() && new.is_empty(),
            "lines the split lost: {gone:#?}\nlines the split added: {new:#?}"
        );
        assert_eq!(
            (before.lines(), before.digest().as_str()),
            BEFORE,
            "the unsplit file counted from its blob"
        );
        assert_eq!(
            (before.module, before.uses, before.visibility),
            (24, 23, 154),
            "the module, use and visibility lines of the unsplit file"
        );
    }
    assert_eq!(
        (after.module, after.uses, after.visibility),
        SET_ASIDE,
        "the module, use and visibility lines the split set aside"
    );
    assert_eq!(
        (after.lines(), after.digest().as_str()),
        BEFORE,
        "the six files' lines, less module, use and visibility lines, against the \
         unsplit file's"
    );
}

/// The counter itself: a module line, a use line over three lines with its
/// attribute, a header, a visibility and a changed line each land where the
/// proof above says, so a counter that set aside everything could not pass.
#[test]
fn the_line_counter_sets_aside_only_module_use_and_visibility() {
    let before = "fn a() {}\nstruct B {\n    c: u8,\n}\n\nconst D: u8 = 1;\n";
    let after = "//! A new file.\n\n#[cfg(test)]\nuse super::{\n    a,\n};\n\
                 #[path = \"x.rs\"]\nmod x;\npub(super) fn a() {}\nstruct B {\n    \
                 pub(super) c: u8,\n}\nconst D: u8 = 1;\n";
    let b = counted(before, false);
    let a = counted(after, true);
    assert_eq!((b.lines(), b.module, b.uses, b.visibility), (5, 0, 0, 0));
    assert_eq!((a.lines(), a.module, a.uses, a.visibility), (5, 3, 4, 2));
    assert_eq!(a.digest(), b.digest());
    let changed = counted(&after.replace("const D: u8 = 1;", "const D: u8 = 2;"), true);
    assert_ne!(changed.digest(), b.digest(), "a changed line is counted");
}
