//! Tests of `pull::gdfl_run::preflight` (IA-3b). Every source is invented
//! at run time with `crate::gdfl_fixtures`' builders: no vendor byte, every
//! price one no index or option printed. Every refusal test asserts that no
//! import-journal line, no progress record and no lock file was written.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use super::*;
use crate::gdfl_cm::{CmKind, day_folder_name};
use crate::gdfl_fixtures::{Method, bts, csv, put, row, scratch, zip};
use crate::gdfl_import::{Durability, Workers, journal_path};
use crate::gdfl_run::{
    By, ClaimAsk, Kinds, LogState, Plan, SourceCache, SourceWord, claim_template, plan,
};

// ── fixtures ────────────────────────────────────────────────────────────────

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// 2024-04-01, a Monday the exchange traded.
fn d1() -> Day {
    day(2024, 4, 1)
}

/// 2024-04-02, the next trading day.
fn d2() -> Day {
    day(2024, 4, 2)
}

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

/// An invented index file of `stem` on `d`: a row every 997 seconds of the
/// session at a level no index printed, and one after the close.
fn index_csv(stem: &str, d: Day) -> Vec<u8> {
    let mut rows: Vec<String> = (0..22_500)
        .step_by(997)
        .map(|s| row(stem, d, NINE_FIFTEEN + s, "11111.25", 0, 0))
        .collect();
    rows.push(row(stem, d, CLOSE + 60, "11111.50", 0, 0));
    csv(&rows)
}

/// The index day zip of `d`: its folder entry, NIFTY 50 and NIFTY BANK
/// (the two swept indices, so a clean day names no absence), deflated.
fn index_day_zip(d: Day) -> Vec<u8> {
    let folder = day_folder_name(CmKind::Indices, d);
    let (nifty, bank) = (
        format!("{folder}/NIFTY 50.NSE_IDX.csv"),
        format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
    );
    zip(&[
        (&format!("{folder}/"), b"", Method::Stored),
        (&nifty, &index_csv("NIFTY 50.NSE_IDX", d), Method::Deflated),
        (&bank, &index_csv("NIFTY BANK.NSE_IDX", d), Method::Deflated),
    ])
}

/// An outer capital-market archive at `root/cm.zip` holding `days`' index
/// day zips (each `Some(bytes)`, or the day zip of `index_day_zip`).
fn cm_archive(root: &Path, days: &[(Day, Option<Vec<u8>>)]) -> PathBuf {
    let members: Vec<(String, Vec<u8>)> = days
        .iter()
        .map(|(d, bytes)| {
            (
                crate::gdfl_archive::day_zip_name(CmKind::Indices, *d),
                bytes.clone().unwrap_or_else(|| index_day_zip(*d)),
            )
        })
        .collect();
    let refs: Vec<(&str, &[u8], Method)> = members
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(root, "cm.zip", &zip(&refs))
}

/// One invented options file of `ticker` on `d`.
fn option_csv(ticker: &str, d: Day) -> Vec<u8> {
    let stem = format!("{ticker}.NFO");
    let rows: Vec<String> = (0..22_500)
        .step_by(2_003)
        .map(|s| row(&stem, d, NINE_FIFTEEN + s, "101.25", 25, 1_000))
        .collect();
    csv(&rows)
}

/// The options day zip of `d`: one invented NIFTY call.
fn options_day_zip(d: Day) -> Vec<u8> {
    let ticker = "NIFTY04APR2422000CE";
    let name = format!(
        "{}\\Options\\{ticker}.NFO.csv",
        crate::gdfl_nfo::day_folder_name(d)
    );
    zip(&[(&name, &option_csv(ticker, d), Method::Deflated)])
}

/// The year zip bytes holding `days`' options day zips (each `Some(bytes)`,
/// or the day zip of `options_day_zip`).
fn year_zip_of(days: &[(Day, Option<Vec<u8>>)]) -> Vec<u8> {
    let members: Vec<(String, Vec<u8>)> = days
        .iter()
        .map(|(d, bytes)| {
            (
                crate::gdfl_nfo::NfoZips::day_zip_name(*d),
                bytes.clone().unwrap_or_else(|| options_day_zip(*d)),
            )
        })
        .collect();
    let refs: Vec<(&str, &[u8], Method)> = members
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    zip(&refs)
}

/// The tick store's `cm` day file of the index tree on `d` under `ts`.
fn tick_day(ts: &Path, d: Day, entries: &[(&str, &[u8])]) -> PathBuf {
    let zip_name = crate::gdfl_archive::day_zip_name(CmKind::Indices, d);
    let rel = format!("cm/{}.bts", zip_name.strip_suffix(".zip").unwrap());
    put(ts, &rel, &bts(entries))
}

/// A tick store under `root/ts` with its `FORMAT.md` (invented text, so its
/// digest is neither the charter's) and NIFTY 50 on `days`.
fn tick_store(root: &Path, days: &[Day]) -> PathBuf {
    let ts = root.join("ts");
    put(&ts, "FORMAT.md", b"# an invented format, for a test\n");
    for d in days {
        let folder = day_folder_name(CmKind::Indices, *d);
        let name = format!("{folder}/NIFTY 50.NSE_IDX.csv");
        let body = index_csv("NIFTY 50.NSE_IDX", *d);
        let _path = tick_day(&ts, *d, &[(&name, &body)]);
    }
    ts
}

/// An empty store under `root/store`, with its `imports/` directory.
fn store_at(root: &Path) -> PathBuf {
    let store = root.join("store");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    store
}

/// What the store holds that a run writes before its first day: the
/// journal, any lock file and any progress record. A refusal leaves none.
fn nothing_written(store: &Path) {
    assert!(!journal_path(store).exists(), "a journal was written");
    let imports = store.join("imports");
    let names: Vec<String> = std::fs::read_dir(&imports)
        .map(|names| {
            names
                .map(|name| name.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        names.iter().all(|name| !Path::new(name)
            .extension()
            .is_some_and(|ext| ext == "lock" || ext == "progress")),
        "{names:?}"
    );
}

fn env_of(pairs: Vec<(&'static str, PathBuf)>) -> impl Fn(&str) -> Option<OsString> {
    move |var| {
        pairs
            .iter()
            .find(|(k, _)| *k == var)
            .map(|(_, v)| v.clone().into_os_string())
    }
}

/// A request of one kind from `source` over `from..=to`.
fn asked(kind: ImportKind, source: SourceWord, (from, to): (Day, Day)) -> Request {
    Request {
        workers: Workers::new(2).unwrap(),
        durability: Durability::DEFAULT,
        by: By::Cli,
        ..Request::new(Kinds::one(kind), source, (from, to))
    }
}

/// `request` planned against `store` with `env`, as the cli plans it.
fn planned(
    request: Request,
    store: &Path,
    env: Vec<(&'static str, PathBuf)>,
) -> Result<Plan, RunRefusal> {
    let log = LogState::Installed(store.join("logs"));
    let process = Process {
        store_root: store,
        commit: Some("0123456789abcdef0123456789abcdef01234567"),
        log: &log,
        unstamped: None,
    };
    plan(request, &process, &env_of(env), &mut SourceCache::default())
}

/// A host answering what the test says.
#[derive(Debug, Clone)]
struct Fake {
    volume: Result<Volume, String>,
    open_files: Option<u64>,
    space: Result<Space, String>,
}

impl Fake {
    fn local() -> Self {
        Self {
            volume: Ok(Volume {
                read_only: false,
                local: Some(true),
                fs: "apfs".to_owned(),
            }),
            open_files: None,
            space: Ok(Space {
                total: 1 << 40,
                free: 1 << 39,
            }),
        }
    }
}

impl Host for Fake {
    fn volume(&self, _path: &Path) -> Result<Volume, String> {
        self.volume.clone()
    }

    fn open_files(&self) -> Option<u64> {
        self.open_files
    }

    fn space(&self, _path: &Path) -> Result<Space, String> {
        self.space.clone()
    }
}

/// `plan`'s kinds re-checked under `host` (the seam the real host never
/// answers on a developer's machine).
fn checked_under(plan: &Plan, host: &dyn Host) -> Result<Vec<Checked>, RunRefusal> {
    let log = LogState::Installed(plan.store_root.join("logs"));
    let process = Process {
        store_root: &plan.store_root,
        commit: None,
        log: &log,
        unstamped: None,
    };
    check_with(&plan.request, &process, &plan.kinds, host)
}

/// A census file at `path` whose only slot (generation 0) is `header`, its
/// entries region sparse at the length `n_valid` entries take.
fn census_file(path: &Path, header: crate::manifest::ManifestHeader) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = vec![0_u8; usize::try_from(crate::manifest::HEADER_LEN).unwrap()];
    bytes[..crate::manifest::IMAGE_LEN].copy_from_slice(&header.image());
    std::fs::write(path, &bytes).unwrap();
    let file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.set_len(crate::manifest::HEADER_LEN + header.n_valid * crate::manifest::ENTRY_STRIDE)
        .unwrap();
}

/// A v3 census on `store` holding `n_valid` entries of `n_keys` keys.
fn v3_census(store: &Path, n_valid: u64, n_keys: u64) {
    let header = crate::manifest::ManifestHeader {
        n_valid,
        n_keys,
        ..crate::manifest::ManifestHeader::genesis(Vendor::Gdfl)
    };
    census_file(&crate::manifest::manifest_path(store, Vendor::Gdfl), header);
}

/// A v4 partition of `segment` on `store`, epoch 1, of `capacity` holding
/// `n_valid` entries of `n_keys` keys.
fn v4_census(store: &Path, segment: Segment, (capacity, n_valid, n_keys): (u64, u64, u64)) {
    std::fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    let header = crate::manifest::ManifestHeader {
        n_valid,
        n_keys,
        ..crate::manifest::ManifestHeader::genesis_partition(Vendor::Gdfl, segment, capacity)
            .unwrap()
    };
    census_file(
        &crate::manifest::epoch_path(store, Vendor::Gdfl, segment, 1),
        header,
    );
}

/// An options root under `root/nfo` holding `zips` (a name and its bytes).
fn options_root(root: &Path, zips: &[(&str, Vec<u8>)]) -> PathBuf {
    let dir = root.join("nfo");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, bytes) in zips {
        let _path = put(&dir, name, bytes);
    }
    dir
}

/// An options run of `range` from `nfo` into a fresh store under `root`.
fn options_plan(root: &Path, nfo: &Path, range: (Day, Day)) -> (PathBuf, Result<Plan, RunRefusal>) {
    let store = store_at(root);
    let planned = planned(
        asked(ImportKind::Options, SourceWord::Zip, range),
        &store,
        vec![(super::super::NFO_ARCHIVES_VAR, nfo.to_path_buf())],
    );
    (store, planned)
}

// ═══ 1, 2: the year zips (FM-060's remainder, A9, A11, A23, RA-322) ═════════

/// A9: a year of the range with no `<yyyy>.zip` is refused before anything,
/// naming the names beside it that look like that year's zip (at most four,
/// in name order), never a `.DS_Store` or another year's zip; a range inside
/// the years the root holds passes.
#[test]
fn a_missing_or_misspelt_year_zip_is_refused_with_its_near_misses() {
    let root = scratch("ia3b-near-miss");
    let held = year_zip_of(&[(d1(), None)]);
    let nfo = options_root(
        &root,
        &[
            ("2024.zip", held.clone()),
            ("2020.ZIP", held.clone()),
            ("2020 (1).zip", held.clone()),
            ("2020-21.zip", held.clone()),
            ("2020.zip.part", held.clone()),
            (".DS_Store", b"\0\0".to_vec()),
            ("2019.zip", held.clone()),
        ],
    );
    let (store, refused) = options_plan(&root, &nfo, (day(2020, 3, 2), day(2024, 4, 1)));
    // 2019.zip is not 2020's; 2020.zip.part does not end .zip.
    assert_eq!(
        refused.unwrap_err(),
        RunRefusal::YearZipMissing {
            year: 2020,
            near_miss: vec![
                "2020 (1).zip".to_owned(),
                "2020-21.zip".to_owned(),
                "2020.ZIP".to_owned()
            ],
        }
    );
    nothing_written(&store);
    let words = RunRefusal::YearZipMissing {
        year: 2017,
        near_miss: Vec::new(),
    }
    .to_string();
    assert!(
        words.starts_with("year_zip_missing: the options root holds no 2017.zip"),
        "{words}"
    );
    let (_, passed) = options_plan(&scratch("ia3b-near-miss-ok"), &nfo, (d1(), d1()));
    assert!(passed.is_ok(), "{passed:?}");
}

/// The near misses are capped at four however many there are, in name order
/// (extreme: many duplicates of one year's name).
#[test]
fn at_most_four_near_misses_are_named_in_name_order() {
    let root = scratch("ia3b-near-cap");
    let held = year_zip_of(&[(d1(), None)]);
    let names = [
        "2023 (5).zip",
        "2023 (1).zip",
        "2023 (3).zip",
        "2023.Zip",
        "2023 (2).zip",
        "2023 (4).zip",
    ];
    let mut zips: Vec<(&str, Vec<u8>)> = names.iter().map(|n| (*n, held.clone())).collect();
    zips.push(("2024.zip", held));
    let nfo = options_root(&root, &zips);
    let (_, refused) = options_plan(&root, &nfo, (day(2023, 12, 29), d1()));
    let RunRefusal::YearZipMissing { year, near_miss } = refused.unwrap_err() else {
        panic!("not year_zip_missing");
    };
    assert_eq!(year, 2023);
    assert_eq!(
        near_miss,
        [
            "2023 (1).zip",
            "2023 (2).zip",
            "2023 (3).zip",
            "2023 (4).zip"
        ]
    );
}

/// A root holding no `<yyyy>.zip` at all is AF-01's `source_root_unknown`,
/// said by the cli's door: the preflight leaves it alone rather than say a
/// second, worse-worded refusal (D-7000).
#[test]
fn a_root_with_no_year_zip_at_all_is_left_to_the_door() {
    let root = scratch("ia3b-no-year");
    let nfo = options_root(&root, &[("19.zip", zip(&[])), ("notes.txt", b"x".to_vec())]);
    std::fs::create_dir_all(nfo.join("2019.zip")).unwrap();
    let (_, passed) = options_plan(&root, &nfo, (d1(), d1()));
    assert!(passed.is_ok(), "{passed:?}");
    let (_, absent) = options_plan(&root, &root.join("typo"), (d1(), d1()));
    assert!(absent.is_ok(), "{absent:?}");
}

/// A11: a year zip this process cannot open is refused before the run,
/// naming its path and mode. A process the host lets read anything (root)
/// reads a mode-0000 file, so there the end-to-end half is NOT PROVED and
/// says so; the refusal itself is proved on a name that cannot be opened.
#[test]
fn an_unreadable_year_zip_is_refused_before_the_run() {
    let root = scratch("ia3b-unreadable");
    let nfo = options_root(&root, &[("2024.zip", year_zip_of(&[(d1(), None)]))]);
    let zip_path = nfo.join("2024.zip");
    std::fs::set_permissions(&zip_path, std::fs::Permissions::from_mode(0o000)).unwrap();
    let (store, answer) = options_plan(&root, &nfo, (d1(), d1()));
    let readable = std::fs::File::open(&zip_path).is_ok();
    std::fs::set_permissions(&zip_path, std::fs::Permissions::from_mode(0o600)).unwrap();
    if readable {
        assert!(answer.is_ok(), "{answer:?}");
    } else {
        assert_eq!(
            answer.unwrap_err(),
            RunRefusal::SourceUnreadable {
                path: zip_path.clone(),
                mode: "0000".to_owned(),
            }
        );
        nothing_written(&store);
    }
    // The refusal on a name no process can open, whoever it runs as.
    let blocked = put(&root, "plain", b"x").join("2024.zip");
    let refused = year_zip(2024, &blocked).unwrap_err();
    let RunRefusal::SourceUnreadable { path, mode } = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(path, &blocked);
    assert!(mode.starts_with("unmeasured ("), "{mode}");
    let words = refused.to_string();
    assert!(
        words.starts_with("source_unreadable: ") && words.contains("cannot be read"),
        "{words}"
    );
}

/// A23 (RA-322): a year zip whose end record states zero entries, or a
/// zero-length one, is refused `year_zip_empty` before any line, never read
/// as a year whose every day the source lacks. A zip whose end record cannot
/// be found (a copy still being written) is left to the day reader.
#[test]
fn an_empty_year_zip_is_refused_before_any_line() {
    for (tag, bytes) in [("entries", zip(&[])), ("bytes", Vec::new())] {
        let root = scratch(&format!("ia3b-empty-year-{tag}"));
        let nfo = options_root(&root, &[("2024.zip", bytes)]);
        let (store, refused) = options_plan(&root, &nfo, (d1(), d1()));
        assert_eq!(
            refused.unwrap_err(),
            RunRefusal::YearZipEmpty { year: 2024 },
            "{tag}"
        );
        nothing_written(&store);
    }
    let root = scratch("ia3b-empty-year-torn");
    let whole = year_zip_of(&[(d1(), None)]);
    let nfo = options_root(&root, &[("2024.zip", whole[..whole.len() - 30].to_vec())]);
    let (_, passed) = options_plan(&root, &nfo, (d1(), d1()));
    assert!(passed.is_ok(), "{passed:?}");
    assert!(
        RunRefusal::YearZipEmpty { year: 2024 }
            .to_string()
            .contains("2024.zip holds no entry")
    );
}

/// A year boundary (extreme): a range from 31 Dec to 2 Jan needs both
/// years' zips, and names the later one when only it is missing.
#[test]
fn a_range_across_new_year_needs_both_year_zips() {
    let root = scratch("ia3b-new-year");
    let nfo = options_root(
        &root,
        &[("2023.zip", year_zip_of(&[(day(2023, 12, 29), None)]))],
    );
    let (_, refused) = options_plan(&root, &nfo, (day(2023, 12, 29), day(2024, 1, 2)));
    assert_eq!(
        refused.unwrap_err(),
        RunRefusal::YearZipMissing {
            year: 2024,
            near_miss: Vec::new()
        }
    );
}

// ═══ 2: an empty container refuses its day (RA-322, B28a) ═══════════════════

/// The last line closing `kind`'s `d` in `store`'s journal.
fn close_of(store: &Path, kind: &str, d: Day) -> String {
    let text = std::fs::read_to_string(journal_path(store)).unwrap();
    text.lines()
        .rfind(|line| {
            ["done", "incomplete", "refused"]
                .iter()
                .any(|verb| line.starts_with(&format!("{verb} {kind} {d} ")))
        })
        .unwrap_or_else(|| panic!("no close of {kind} {d} in {text}"))
        .to_owned()
}

/// `kind`'s `d` is not done in `store`'s journal.
fn open_day(store: &Path, kind: ImportKind, d: Day) -> bool {
    !super::super::JournalReader::open(store)
        .unwrap()
        .is_done(kind, d, "*")
}

/// An index run of `d1..d2` from `source` into `store`, twice: the faulty
/// `d1` is refused by `code` both times and never done, and `d2` is done.
fn faulty_index_day_stays_open<S>(source: &S, store: &Path, code: &str)
where
    S: CmSource + Sync,
    S::Locator: Sync,
{
    let run = crate::gdfl_import::Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: store,
    };
    for pass in 0..2 {
        let report = crate::gdfl_import::run_cm(source, &run).unwrap();
        assert!(
            !report.failures.is_empty(),
            "{code} pass {pass}: the faulty day named no failure"
        );
    }
    let line = close_of(store, "indices", d1());
    assert!(line.contains(&format!("reason={code}")), "{code}: {line}");
    assert!(open_day(store, ImportKind::Indices, d1()), "{code}: {line}");
    assert!(!open_day(store, ImportKind::Indices, d2()), "{code}");
}

/// B28a: a day zip nested in the capital-market archive that lists zero
/// entries refuses its own day `day_zip_empty`, by name, and the day stays
/// open across two runs while the next day closes done; the same for an
/// options day zip nested in a year zip.
#[test]
fn an_empty_nested_day_zip_refuses_its_day_and_stays_open() {
    let root = scratch("ia3b-empty-nested");
    let path = cm_archive(&root, &[(d1(), Some(zip(&[]))), (d2(), None)]);
    let archive = Archive::open(&path).unwrap();
    assert_eq!(
        archive.day(CmKind::Indices, d1()).unwrap_err(),
        CmRefusal::DayZipEmpty {
            name: "GFDLCM_INDICES_TICK_01042024.zip".to_owned()
        }
    );
    let store = store_at(&root);
    faulty_index_day_stays_open(&archive, &store, "day_zip_empty");
    assert_eq!(
        crate::gdfl_import::cm_cause(&CmRefusal::DayZipEmpty {
            name: String::new()
        }),
        crate::gdfl_import::Cause::open(crate::gdfl_import::Stage::Vendor, "day_zip_empty")
    );

    let nfo_root = scratch("ia3b-empty-nested-nfo");
    let nfo = options_root(
        &nfo_root,
        &[(
            "2024.zip",
            year_zip_of(&[(d1(), Some(zip(&[]))), (d2(), None)]),
        )],
    );
    let zips = NfoZips::new(&nfo);
    assert_eq!(
        zips.day(d1()).unwrap_err(),
        CmRefusal::DayZipEmpty {
            name: "GFDLNFO_TICK_01042024.zip".to_owned()
        }
    );
    let store = store_at(&nfo_root);
    let run = crate::gdfl_import::Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: &store,
    };
    for _ in 0..2 {
        let _report = crate::gdfl_import::run_nfo(&zips, &run).unwrap();
    }
    assert!(close_of(&store, "options", d1()).contains("reason=day_zip_empty"));
    assert!(open_day(&store, ImportKind::Options, d1()));
    assert!(!open_day(&store, ImportKind::Options, d2()));
}

/// B28a, the tick store: a day file whose index lists zero entries refuses
/// its day `day_zip_empty` naming the file under the root, and stays open.
#[test]
fn an_empty_tick_store_day_refuses_its_day_and_stays_open() {
    let root = scratch("ia3b-empty-bts");
    let ts = tick_store(&root, &[d2()]);
    let _empty = tick_day(&ts, d1(), &[]);
    let store_reader = TickStore::new(&ts);
    let refused = store_reader.day(CmKind::Indices, d1()).unwrap_err();
    // The vendor day zip's own name, as the archive names it: two sources
    // of one day report one run (D-2806).
    assert_eq!(
        refused,
        CmRefusal::DayZipEmpty {
            name: "GFDLCM_INDICES_TICK_01042024.zip".to_owned()
        }
    );
    let store = store_at(&root);
    faulty_index_day_stays_open(&store_reader, &store, "day_zip_empty");
}

// ═══ 3: member_too_large (RA-238, B28) ══════════════════════════════════════

/// Bytes that count every read made of them.
struct Counting {
    bytes: Vec<u8>,
    reads: std::sync::atomic::AtomicUsize,
}

impl Counting {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            reads: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    fn reads(&self) -> usize {
        self.reads.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl crate::gdfl_archive::ReadAt for Counting {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let _before = self
            .reads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.bytes.read_at(offset, buf)
    }
}

/// A zip of one stored member whose central record states `stated` as its
/// uncompressed size (one patch of the record's field at offset 24).
fn zip_stating(stated: u32) -> Vec<u8> {
    let mut bytes = zip(&[("F/x.csv", b"tiny", Method::Stored)]);
    let end = bytes.len() - 22;
    let central = usize::try_from(u32::from_le_bytes(
        bytes[end + 16..end + 20].try_into().unwrap(),
    ))
    .unwrap();
    bytes[central + 24..central + 28].copy_from_slice(&stated.to_le_bytes());
    bytes
}

/// B28: a member whose record states more than `MEMBER_CAP` unpacked bytes
/// is refused `member_too_large{name, size}` (R4-08's code and fields)
/// before any byte of it is read: not its local header, not its data. One
/// byte at the cap is not refused by the cap.
#[test]
fn a_member_stating_more_than_the_cap_is_refused_before_any_read() {
    let cap = crate::tickstore_write::MEMBER_CAP;
    let over = u32::try_from(cap + 1).unwrap();
    let bytes = zip_stating(over);
    let entry = crate::gdfl_archive::zip_entries(&bytes, 0, len_of(&bytes))
        .unwrap()
        .remove(0);
    assert_eq!(entry.len, cap + 1);
    let source = Counting::new(bytes);
    let refused = crate::gdfl_archive::member_bytes(&source, entry.locator, &entry.name, entry.len)
        .unwrap_err();
    assert_eq!(
        refused,
        CmRefusal::MemberTooLarge {
            name: "F/x.csv".to_owned(),
            size: cap + 1
        }
    );
    assert_eq!(source.reads(), 0, "no byte of the member was read");
    assert_eq!(
        refused.to_string(),
        format!("`F/x.csv` states {} bytes, over {cap}", cap + 1)
    );
    // At the cap the cap does not refuse: the read goes on (and the member,
    // 4 bytes long, is then simply short, which the verifier names).
    let at_cap = zip_stating(u32::try_from(cap).unwrap());
    let entry = crate::gdfl_archive::zip_entries(&at_cap, 0, len_of(&at_cap))
        .unwrap()
        .remove(0);
    let source = Counting::new(at_cap);
    let read = crate::gdfl_archive::member_bytes(&source, entry.locator, &entry.name, entry.len);
    assert_eq!(read.unwrap(), b"tiny");
    assert!(source.reads() > 0);
}

fn len_of(bytes: &[u8]) -> u64 {
    u64::try_from(bytes.len()).unwrap()
}

/// B28, the tick store: an entry whose index states a rebuilt size, or a
/// block, past the cap is refused before its block is read or a frame
/// inflated; a directory entry still rebuilds to nothing.
#[test]
fn a_tick_store_entry_rebuilding_past_the_cap_is_refused() {
    let cap = crate::tickstore_write::MEMBER_CAP;
    let entry = |size: u64, len: u64, kind: u8| crate::gdfl_tickstore::IndexEntry {
        name: "F/x.csv".to_owned(),
        kind,
        off: 8,
        len,
        size,
        crc: 0,
        rows: 0,
        dos_time: 0,
    };
    for (size, len) in [(cap + 1, 16), (16, cap + 1), (u64::MAX, u64::MAX)] {
        let source = Counting::new(vec![0; 64]);
        let refused = crate::gdfl_tickstore::rebuild(&source, &entry(size, len, 2)).unwrap_err();
        assert_eq!(
            refused,
            CmRefusal::MemberTooLarge {
                name: "F/x.csv".to_owned(),
                size: size.max(len)
            }
        );
        assert_eq!(source.reads(), 0, "{size} {len}");
    }
    let source = Counting::new(Vec::new());
    assert_eq!(
        crate::gdfl_tickstore::rebuild(&source, &entry(cap + 1, 0, 0)),
        Ok(Vec::new())
    );
    assert_eq!(
        crate::gdfl_import::cm_cause(&CmRefusal::MemberTooLarge {
            name: String::new(),
            size: 0
        }),
        crate::gdfl_import::Cause::open(crate::gdfl_import::Stage::Vendor, "member_too_large")
    );
}

/// The year-zip check's entry count reads the zip's tail and never its
/// directory: one zip of one member and one of 2,000 cost the same reads,
/// and each states its own count.
#[test]
fn the_stated_entry_count_reads_the_tail_alone() {
    let reads_for = |members: usize| {
        let names: Vec<String> = (0..members).map(|n| format!("F/{n}.csv")).collect();
        let bytes = zip(&names
            .iter()
            .map(|name| (name.as_str(), &b"x"[..], Method::Stored))
            .collect::<Vec<_>>());
        let len = len_of(&bytes);
        let source = Counting::new(bytes);
        let stated = crate::gdfl_archive::stated_entries(&source, len).unwrap();
        (stated, source.reads())
    };
    let (one, one_reads) = reads_for(1);
    let (many, many_reads) = reads_for(2_000);
    assert_eq!((one, many), (1, 2_000));
    assert_eq!(one_reads, many_reads);
    assert!(one_reads <= 3, "{one_reads}");
}

// ═══ 4: source_changing and source_busy (RA-323, RA-304) ═══════════════════

/// The outer archive, grown by `grow` bytes at the first member read, as a
/// copy still being written grows: the source a run reads mid-copy.
struct Growing {
    inner: Archive<std::fs::File>,
    path: PathBuf,
    grown: std::sync::atomic::AtomicBool,
}

impl CmSource for Growing {
    type Locator = crate::gdfl_archive::ZipLocator;

    fn day(
        &self,
        kind: CmKind,
        d: Day,
    ) -> Result<Option<crate::gdfl_cm::DayListing<Self::Locator>>, CmRefusal> {
        self.inner.day(kind, d)
    }

    fn fetch(
        &self,
        listing: &crate::gdfl_cm::DayListing<Self::Locator>,
        file: &crate::gdfl_cm::ListedFile<Self::Locator>,
    ) -> Result<Vec<u8>, CmRefusal> {
        if !self.grown.swap(true, std::sync::atomic::Ordering::SeqCst) {
            use std::io::Write as _;
            let mut more = std::fs::OpenOptions::new()
                .append(true)
                .open(&self.path)
                .unwrap();
            more.write_all(&[0_u8; 64]).unwrap();
        }
        self.inner.fetch(listing, file)
    }
}

/// RA-323: a zip that grows while a run reads it refuses every day read from
/// it after it grew `source_changing`, naming the file and both identities;
/// no such day is closed done, and the run names the file among its
/// failures. Once the copy is whole (the bytes it ends with), the next run
/// reads both days and closes them done.
#[test]
fn a_zip_that_is_still_being_copied_is_never_closed() {
    let root = scratch("ia3b-copying");
    let path = cm_archive(&root, &[(d1(), None), (d2(), None)]);
    let whole = std::fs::metadata(&path).unwrap().len();
    let store = store_at(&root);
    let run = crate::gdfl_import::Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: &store,
    };
    let growing = Growing {
        inner: Archive::open(&path).unwrap(),
        path: path.clone(),
        grown: std::sync::atomic::AtomicBool::new(false),
    };
    let report = crate::gdfl_import::run_cm(&growing, &run).unwrap();
    assert!(
        report
            .failures
            .iter()
            .any(|f| f.why.contains(&path.display().to_string()) && f.why.contains("changed")),
        "{:?}",
        report.failures
    );
    for d in [d1(), d2()] {
        let line = close_of(&store, "indices", d);
        assert!(line.contains("reason=source_changing"), "{line}");
        assert!(open_day(&store, ImportKind::Indices, d), "{line}");
    }
    // The copy finishes as the bytes it was: the next run reads it whole.
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(whole)
        .unwrap();
    let again = Archive::open(&path).unwrap();
    let report = crate::gdfl_import::run_cm(&again, &run).unwrap();
    assert!(report.failures.is_empty(), "{:?}", report.failures);
    for d in [d1(), d2()] {
        assert!(!open_day(&store, ImportKind::Indices, d));
    }
    assert_eq!(
        crate::gdfl_import::cm_cause(&CmRefusal::SourceChanging {
            path: String::new(),
            before: String::new(),
            after: String::new()
        }),
        crate::gdfl_import::Cause::open(crate::gdfl_import::Stage::Vendor, "source_changing")
    );
}

/// RA-323, a year zip: rewritten in place (its modification time moved,
/// its bytes kept) between a day's listing and a member read, the read is
/// refused `source_changing`; a fresh source over the same bytes reads.
#[test]
fn a_year_zip_rewritten_between_listing_and_read_is_source_changing() {
    let root = scratch("ia3b-year-rewritten");
    let nfo = options_root(&root, &[("2024.zip", year_zip_of(&[(d1(), None)]))]);
    let zips = NfoZips::new(&nfo);
    let listing = zips.day(d1()).unwrap().unwrap();
    let file = &listing.entries()[0];
    let handle = std::fs::OpenOptions::new()
        .write(true)
        .open(nfo.join("2024.zip"))
        .unwrap();
    handle
        .set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000))
        .unwrap();
    let refused = zips.fetch(file).unwrap_err();
    let CmRefusal::SourceChanging {
        path,
        before,
        after,
    } = &refused
    else {
        panic!("{refused:?}");
    };
    assert!(path.ends_with("2024.zip"), "{path}");
    assert_ne!(before, after);
    assert!(after.contains("mtime=1000."), "{after}");
    let fresh = NfoZips::new(&nfo);
    let listing = fresh.day(d1()).unwrap().unwrap();
    assert!(fresh.fetch(&listing.entries()[0]).is_ok());
}

/// RA-304: a tick-store day file replaced by rename between its index read
/// and a block read, by a file of the same length and modification time (a
/// new inode), refuses the read `source_changing`: the handle still reads
/// the old file, and the path names the new one.
#[test]
fn a_tick_store_day_file_replaced_by_rename_mid_read_is_source_changing() {
    let root = scratch("ia3b-renamed");
    let ts = tick_store(&root, &[]);
    let folder = day_folder_name(CmKind::Indices, d1());
    let (a, b) = (
        format!("{folder}/NIFTY 50.NSE_IDX.csv"),
        format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
    );
    let (body_a, body_b) = (
        index_csv("NIFTY 50.NSE_IDX", d1()),
        index_csv("NIFTY BANK.NSE_IDX", d1()),
    );
    let path = tick_day(&ts, d1(), &[(&a, &body_a), (&b, &body_b)]);
    let reader = TickStore::new(&ts);
    let listing = reader.day(CmKind::Indices, d1()).unwrap().unwrap();
    assert!(reader.fetch(&listing, &listing.entries()[0]).is_ok());
    // The writer publishes a copy by rename: same bytes, same length, the
    // same modification time, another inode.
    let was = std::fs::metadata(&path).unwrap();
    let copy = path.with_extension("bts.tmp");
    std::fs::copy(&path, &copy).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&copy)
        .unwrap()
        .set_modified(was.modified().unwrap())
        .unwrap();
    std::fs::rename(&copy, &path).unwrap();
    let now = std::fs::metadata(&path).unwrap();
    assert_eq!(
        (now.len(), now.modified().unwrap()),
        (was.len(), was.modified().unwrap())
    );
    let refused = reader.fetch(&listing, &listing.entries()[1]).unwrap_err();
    let CmRefusal::SourceChanging { before, after, .. } = &refused else {
        panic!("{refused:?}");
    };
    let ino = |text: &str| {
        text.split(' ')
            .find(|w| w.starts_with("ino="))
            .unwrap()
            .to_owned()
    };
    assert_ne!(ino(before), ino(after), "{before} / {after}");
    // A listing read after the rename reads the new file whole.
    let fresh = reader.day(CmKind::Indices, d1()).unwrap().unwrap();
    assert!(reader.fetch(&fresh, &fresh.entries()[1]).is_ok());
}

/// The one identity: its fields, its words, and a remount's new device
/// changing nothing (there is no device in it).
#[test]
fn the_source_identity_is_length_inode_and_both_times_only() {
    let root = scratch("ia3b-identity");
    let path = put(&root, "f", b"abc");
    let meta = std::fs::metadata(&path).unwrap();
    let identity = SourceIdentity::of(&meta);
    assert_eq!(identity.len, 3);
    let words = identity.to_string();
    for field in ["len=3 ", "ino=", " mtime=", " ctime="] {
        assert!(words.contains(field), "{words}");
    }
    assert!(!words.contains("dev"), "{words}");
    let handle = std::sync::Arc::new(std::fs::File::open(&path).unwrap());
    let watch = Watch::new(&path, handle).unwrap();
    assert_eq!(watch.listed(), identity);
    assert_eq!(watch.path(), path);
    assert_eq!(watch.check(), Ok(()));
    // Gone from its path (the handle still reads): unreachable, never
    // "changed", so the run's loss probe asks the root.
    std::fs::remove_file(&path).unwrap();
    assert_eq!(
        watch.check(),
        Err(WatchFault::Unreachable(std::io::ErrorKind::NotFound))
    );
}

/// A tick store whose writer holds its root's `.lock` is refused
/// `source_busy` before any line; a probe of a store with no `.lock` never
/// creates one, and a released lock lets the run plan.
#[test]
fn a_held_tick_store_lock_is_source_busy_and_never_created() {
    let root = scratch("ia3b-busy");
    let ts = tick_store(&root, &[d1()]);
    let store = store_at(&root);
    let lock = ts.join(crate::tickstore_write::LOCK_NAME);
    let request = || asked(ImportKind::Indices, SourceWord::TickStore, (d1(), d1()));
    let env = || vec![(super::super::TICKSTORE_VAR, ts.clone())];
    assert!(planned(request(), &store, env()).is_ok());
    assert!(!lock.exists(), "the probe created the writer's lock");
    let held = std::fs::File::create(&lock).unwrap();
    held.try_lock().unwrap();
    let refused = planned(request(), &store, env()).unwrap_err();
    let RunRefusal::SourceBusy { root: busy, holder } = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(busy, &ts);
    assert!(holder.contains("exclusively"), "{holder}");
    assert!(refused.to_string().starts_with("source_busy: "));
    nothing_written(&store);
    held.unlock().unwrap();
    assert!(planned(request(), &store, env()).is_ok());
}

// ═══ 5: census room (D-4802, D-4850, IA-GR-15, IA-GR-26, FM-096) ═══════════

/// A tick-store indices run of `d1` into `store`, which plans or refuses.
fn ts_plan(root: &Path, store: &Path) -> Result<Plan, RunRefusal> {
    let ts = tick_store(root, &[d1(), d2()]);
    let mut request = asked(ImportKind::Indices, SourceWord::TickStore, (d1(), d2()));
    request.free_floor = 0;
    planned(request, store, vec![(super::super::TICKSTORE_VAR, ts)])
}

/// IA-GR-15: a v3 census whose free room is short of the first planned
/// day's wanted files (here: one, NIFTY 50) is refused `census_room_short`
/// before any line, naming both rooms and the need; one free entry, exactly
/// the need, plans (the boundary), and the plan says the room it found.
#[test]
fn a_census_without_room_for_the_first_day_is_refused_before_any_line() {
    let root = scratch("ia3b-room-v3");
    let store = store_at(&root);
    v3_census(&store, crate::manifest::MAX_ENTRIES, 7);
    let refused = ts_plan(&root, &store).unwrap_err();
    assert_eq!(
        refused,
        RunRefusal::CensusRoomShort {
            layout: "v3",
            free_now: 0,
            free_after: 0,
            need_first_day: 1
        }
    );
    assert!(refused.to_string().contains("the v3 census has room for 0"));
    nothing_written(&store);
    v3_census(&store, crate::manifest::MAX_ENTRIES - 1, 7);
    let plan = ts_plan(&scratch("ia3b-room-v3-one"), &store).unwrap();
    let checked = &plan.checks[0];
    assert_eq!(
        checked.room,
        Room::Measured {
            layout: "v3",
            free_now: 1,
            free_after: 1,
            need_first_day: 1,
            compaction: false,
            // One entry for two planned days of one file each: the second
            // day is where the room would run out, an extrapolation.
            projected_stop: Some(d2()),
        }
    );
    assert!(
        checked
            .line()
            .contains("census.projected_stop=2024-04-02 (extrapolation from the first day)"),
        "{}",
        checked.line()
    );
}

/// IA-GR-26 (FM-096): a v4 partition that is FULL now but whose one
/// compaction (one row per key, into `epoch_capacity(segment, n_keys)` by the
/// writer's own function) leaves room is NOT refused; one with no room even
/// after it is, naming both rooms.
#[test]
fn the_v4_preflight_models_the_compaction_the_writer_makes() {
    let root = scratch("ia3b-room-v4");
    let store = store_at(&root);
    let floor = crate::manifest::SPOT_FLOOR;
    v4_census(&store, Segment::Index, (floor, floor, 10));
    let plan = ts_plan(&root, &store).unwrap();
    let Room::Measured {
        layout,
        free_now,
        free_after,
        compaction,
        ..
    } = plan.checks[0].room
    else {
        panic!("{:?}", plan.checks[0].room);
    };
    assert_eq!((layout, free_now, compaction), ("v4", 0, true));
    assert_eq!(
        free_after,
        crate::manifest::epoch_capacity(Segment::Index, 10).unwrap() - 10
    );
    // The same census, a first day wanting one more than the compaction
    // leaves: refused, before any line.
    let looked = Looked {
        span: Some((d1(), d2())),
        first: Some((d1(), Some(free_after + 1))),
    };
    let request = asked(ImportKind::Indices, SourceWord::TickStore, (d1(), d2()));
    let refused = room(
        &store,
        GdflLayout::V4,
        ImportKind::Indices,
        &looked,
        &request,
        &Fake::local(),
    )
    .unwrap_err();
    assert_eq!(
        refused,
        RunRefusal::CensusRoomShort {
            layout: "v4",
            free_now: 0,
            free_after,
            need_first_day: free_after + 1
        }
    );
    nothing_written(&store);
}

/// FM-096's disk half: when only the compaction gives the room and its new
/// epoch (`HEADER_LEN` + `ENTRY_STRIDE` x its capacity) does not fit above
/// the run's floor on the store's volume, the run is refused
/// `census_compaction_no_disk` naming the epoch's bytes, the free bytes and
/// the floor; with a byte more free it plans.
#[test]
fn a_compaction_the_disk_cannot_hold_is_refused_before_any_line() {
    let root = scratch("ia3b-room-disk");
    let store = store_at(&root);
    let floor = crate::manifest::SPOT_FLOOR;
    v4_census(&store, Segment::Index, (floor, floor, 10));
    let plan = ts_plan(&root, &store).unwrap();
    let capacity = crate::manifest::epoch_capacity(Segment::Index, 10).unwrap();
    let need_bytes = crate::manifest::HEADER_LEN + crate::manifest::ENTRY_STRIDE * capacity;
    let mut request = plan.request.clone();
    request.free_floor = 1 << 20;
    let looked = Looked {
        span: Some((d1(), d2())),
        first: Some((d1(), Some(1))),
    };
    let short = Fake {
        space: Ok(Space {
            total: 1 << 40,
            free: need_bytes + (1 << 20) - 1,
        }),
        ..Fake::local()
    };
    let refused = room(
        &store,
        GdflLayout::V4,
        ImportKind::Indices,
        &looked,
        &request,
        &short,
    )
    .unwrap_err();
    assert_eq!(
        refused,
        RunRefusal::CensusCompactionNoDisk {
            need_bytes,
            free_bytes: need_bytes + (1 << 20) - 1,
            floor: 1 << 20
        }
    );
    assert!(
        refused
            .to_string()
            .starts_with("census_compaction_no_disk: ")
    );
    let enough = Fake {
        space: Ok(Space {
            total: 1 << 40,
            free: need_bytes + (1 << 20),
        }),
        ..Fake::local()
    };
    assert!(
        room(
            &store,
            GdflLayout::V4,
            ImportKind::Indices,
            &looked,
            &request,
            &enough
        )
        .is_ok()
    );
    // A meter that answers what no disk has is refused too, never passed.
    let nonsense = Fake {
        space: Ok(Space { total: 0, free: 0 }),
        ..Fake::local()
    };
    assert!(matches!(
        room(
            &store,
            GdflLayout::V4,
            ImportKind::Indices,
            &looked,
            &request,
            &nonsense
        ),
        Err(RunRefusal::CensusCompactionNoDisk { .. })
    ));
    nothing_written(&store);
}

/// A census header whose newer slot cannot hold its count (a header made
/// durable before its entries, the crash the writer falls back from) is read
/// at its older slot; neither slot decoding is `census_unreadable`, by name
/// (an extreme the census writes by design, out of the plan's list).
#[test]
fn a_torn_census_header_is_read_at_its_older_slot_or_refused_by_name() {
    let root = scratch("ia3b-torn-census");
    let store = store_at(&root);
    let path = crate::manifest::manifest_path(&store, Vendor::Gdfl);
    let older = crate::manifest::ManifestHeader {
        n_valid: 5,
        n_keys: 5,
        ..crate::manifest::ManifestHeader::genesis(Vendor::Gdfl)
    };
    census_file(&path, older);
    // Generation 1 in slot 1, counting entries the file does not hold.
    let newer = crate::manifest::ManifestHeader {
        generation: 1,
        n_valid: 1_000,
        n_keys: 1_000,
        ..older
    };
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    std::os::unix::fs::FileExt::write_all_at(&file, &newer.image(), SLOT_STRIDE).unwrap();
    let header = census_header(&store, GdflLayout::V3, ImportKind::Indices)
        .unwrap()
        .unwrap();
    assert_eq!(header.n_valid, 5);
    std::os::unix::fs::FileExt::write_all_at(&file, &[0xff; 64], 0).unwrap();
    let refused = census_header(&store, GdflLayout::V3, ImportKind::Indices).unwrap_err();
    assert!(
        matches!(&refused, RunRefusal::CensusUnreadable { why } if why.contains("neither header slot")),
        "{refused:?}"
    );
    // No census written yet: the whole capacity is the room.
    let fresh = store_at(&scratch("ia3b-no-census"));
    assert_eq!(
        census_header(&fresh, GdflLayout::V3, ImportKind::Indices),
        Ok(None)
    );
}

// ═══ 6: the legacy cutover (D-4864, D-4908, IA-GR-20) ═══════════════════════

/// A store whose journal holds `text` and no lock file.
fn legacy_store(root: &Path, text: &str) -> PathBuf {
    let store = store_at(root);
    std::fs::write(journal_path(&store), text).unwrap();
    store
}

/// IA-GR-20: the first plan on a store no claim lock ever held, whose
/// journal's options key ends in `begin`, is refused `legacy_run_may_be_live`
/// naming the key, unless acknowledged; the acknowledgement is carried by
/// the request into the claim (`legacy_ack=1`). An indices run on a v4
/// store is not refused for an options key (the check is per kind, D-7003),
/// and on v3 it is (kinds share one claim there).
#[test]
fn the_first_claim_asks_about_a_legacy_run() {
    let root = scratch("ia3b-legacy");
    let journal = "run 1700000000 source=zip\nbegin options 2024-04-01 *\n";
    let store = legacy_store(&root, journal);
    let nfo = options_root(&root, &[("2024.zip", year_zip_of(&[(d1(), None)]))]);
    let request = asked(ImportKind::Options, SourceWord::Zip, (d1(), d1()));
    let env = || vec![(super::super::NFO_ARCHIVES_VAR, nfo.clone())];
    let refused = planned(request.clone(), &store, env()).unwrap_err();
    assert_eq!(
        refused,
        RunRefusal::LegacyRunMayBeLive {
            key: "options 2024-04-01 *".to_owned()
        }
    );
    assert!(refused.to_string().contains("--legacy-run-not-live"));
    assert_eq!(
        std::fs::read_to_string(journal_path(&store)).unwrap(),
        journal
    );
    let mut acked = request;
    acked.legacy_ack = true;
    let plan = planned(acked, &store, env()).unwrap();
    assert!(plan.checks[0].legacy_ack);
    let claim = claim_template(
        &plan,
        &plan.kinds[0],
        &ClaimAsk {
            by: By::Cli,
            pid: 1,
            host: "",
            resumes: 0,
            resume_day: None,
            auto_resume: false,
            legacy_ack: plan.request.legacy_ack,
            unstamped: false,
            retry: 0,
        },
    );
    assert!(claim.legacy_ack);
    assert!(claim.line().contains(" legacy_ack=1"), "{}", claim.line());
    // Per kind on v4: an indices run is not asked about an options key.
    std::fs::create_dir_all(store.join("manifest/v4/gdfl")).unwrap();
    let ts = tick_store(&root, &[d1()]);
    let indices = asked(ImportKind::Indices, SourceWord::TickStore, (d1(), d1()));
    assert!(
        planned(
            indices.clone(),
            &store,
            vec![(super::super::TICKSTORE_VAR, ts.clone())]
        )
        .is_ok()
    );
    // On v3 every kind shares the one claim, so it is.
    let v3 = legacy_store(&scratch("ia3b-legacy-v3"), journal);
    assert!(matches!(
        planned(indices, &v3, vec![(super::super::TICKSTORE_VAR, ts)]),
        Err(RunRefusal::LegacyRunMayBeLive { .. })
    ));
}

/// D-4908: "has claimed" is the lock file's existence, asked by one
/// `metadata` per name: the probe never creates a lock file, and a store
/// that holds one (a run's or a census verb's) is never asked again,
/// whatever its journal says, nor is a journal whose open key closed.
#[test]
fn the_legacy_probe_never_creates_a_lock_file() {
    let root = scratch("ia3b-legacy-probe");
    let journal = "run 1700000000 source=zip\nbegin options 2024-04-01 *\n";
    let store = legacy_store(&root, journal);
    for _ in 0..3 {
        assert!(legacy(&store, GdflLayout::V4, ImportKind::Options, false).is_err());
    }
    nothing_written_but_journal(&store);
    for name in [
        super::super::locks::STORE_LOCK.to_owned(),
        super::super::locks::kind_lock("options"),
    ] {
        let held = legacy_store(&scratch("ia3b-legacy-held"), journal);
        std::fs::write(held.join(&name), b"").unwrap();
        assert_eq!(
            legacy(&held, GdflLayout::V4, ImportKind::Options, false),
            Ok(())
        );
    }
    let closed = legacy_store(
        &scratch("ia3b-legacy-closed"),
        "run 1700000000 source=zip\nbegin options 2024-04-01 *\n\
         refused options 2024-04-01 * definition=4 stage=vendor reason=day_not_held failures=0\n",
    );
    assert_eq!(
        legacy(&closed, GdflLayout::V4, ImportKind::Options, false),
        Ok(())
    );
}

/// The store holds no lock file and no progress record; its journal is
/// untouched by the caller.
fn nothing_written_but_journal(store: &Path) {
    let names: Vec<String> = std::fs::read_dir(store.join("imports"))
        .unwrap()
        .map(|name| name.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["gdfl.journal"], "{names:?}");
}

// ═══ 7: the store the run writes (RA-314, RA-340, RA-306) ═══════════════════

/// RA-314: a store on a read-only volume is refused `store_read_only` before
/// any line, in the words IA-16's `store_not_writable` ends in: the volume's
/// flag when no lock file exists (a seam answering it), and the lock file
/// opened for writing without create when one does. Nothing is created.
#[test]
fn a_read_only_store_is_refused_before_any_line() {
    let root = scratch("ia3b-ro");
    let store = store_at(&root);
    let plan = ts_plan(&root, &store).unwrap();
    let read_only = Fake {
        volume: Ok(Volume {
            read_only: true,
            local: Some(true),
            fs: "apfs".to_owned(),
        }),
        ..Fake::local()
    };
    let refused = checked_under(&plan, &read_only).unwrap_err();
    assert_eq!(
        refused,
        RunRefusal::StoreReadOnly {
            root: store.clone()
        }
    );
    let words = refused.to_string();
    assert!(words.ends_with(NOT_WRITABLE_FIX), "{words}");
    nothing_written(&store);
    // A lock file the process may not write: EACCES, refused the same way
    // (as root a mode-0444 file still opens for writing: NOT PROVED there).
    let lock = store.join(super::super::locks::STORE_LOCK);
    std::fs::write(&lock, b"").unwrap();
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o444)).unwrap();
    let writable = std::fs::OpenOptions::new().write(true).open(&lock).is_ok();
    let answer = writable_check(&store, &Fake::local());
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o644)).unwrap();
    if writable {
        assert_eq!(answer, Ok(()));
    } else {
        assert_eq!(
            answer,
            Err(RunRefusal::StoreReadOnly {
                root: store.clone()
            })
        );
    }
    assert_eq!(
        std::fs::read(&lock).unwrap(),
        b"",
        "the lock file was never written"
    );
}

fn writable_check(store: &Path, host: &dyn Host) -> Result<(), RunRefusal> {
    writable(
        store,
        GdflLayout::V3,
        ImportKind::Indices,
        host.volume(store).as_ref().ok(),
    )
}

/// RA-314 on a real read-only volume (the Mac: an APFS image attached
/// read-only, its root named by `BRUTEX_IA3B_RO_ROOT`; NEEDS THE MAC M8).
#[test]
#[ignore = "needs a read-only volume: hdiutil attach -readonly (NEEDS THE MAC M8)"]
fn a_read_only_store_is_refused_before_any_line_on_a_real_read_only_volume() {
    let root = PathBuf::from(std::env::var_os("BRUTEX_IA3B_RO_ROOT").unwrap());
    let volume = Os.volume(&root).unwrap();
    assert!(volume.read_only, "{volume:?}");
    assert_eq!(
        writable(&root, GdflLayout::V3, ImportKind::Indices, Some(&volume)),
        Err(RunRefusal::StoreReadOnly { root: root.clone() })
    );
}

/// RA-340: a store whose volume says it is not local is refused
/// `store_not_local{root, fs}`; one this host cannot tell about plans and
/// says `store_local=unmeasured`.
#[test]
fn a_network_store_is_refused_store_not_local() {
    let root = scratch("ia3b-nfs");
    let store = store_at(&root);
    let plan = ts_plan(&root, &store).unwrap();
    let network = Fake {
        volume: Ok(Volume {
            read_only: false,
            local: Some(false),
            fs: "smbfs".to_owned(),
        }),
        ..Fake::local()
    };
    let refused = checked_under(&plan, &network).unwrap_err();
    assert_eq!(
        refused,
        RunRefusal::StoreNotLocal {
            root: store.clone(),
            fs: "smbfs".to_owned()
        }
    );
    assert!(
        refused
            .to_string()
            .contains("on a smbfs volume that is not local")
    );
    nothing_written(&store);
    let unknown = Fake {
        volume: Ok(Volume {
            read_only: false,
            local: None,
            fs: "f_type 0xef53".to_owned(),
        }),
        ..Fake::local()
    };
    let found = checked_under(&plan, &unknown).unwrap();
    assert!(
        found[0].line().contains("store_local=unmeasured"),
        "{}",
        found[0].line()
    );
    let unmeasured = Fake {
        volume: Err("statvfs refused".to_owned()),
        ..Fake::local()
    };
    let found = checked_under(&plan, &unmeasured).unwrap();
    assert!(
        found[0]
            .line()
            .contains("store_volume=unmeasured (statvfs refused)")
    );
}

/// RA-306: under a soft open-file limit of 256 (macOS's default for a fresh
/// Terminal, measured 20 Oct) a run is refused `open_files_short` exactly
/// when its workers need more than 256 by the measured model; one below the
/// need is refused at every worker count, naming both numbers and the hand
/// step; exactly the need plans (the boundary), and so does an unlimited
/// limit. By the model as measured, no worker count the importer allows
/// needs more than 256 (`files_needed(Workers::MAX)`): the answer the Mac's
/// M7 re-measures with the cli's own descriptors.
#[test]
fn a_run_under_an_open_file_limit_of_256_is_refused_open_files_short() {
    let root = scratch("ia3b-nofile");
    let store = store_at(&root);
    let mut plan = ts_plan(&root, &store).unwrap();
    let at = |have: u64| Fake {
        open_files: Some(have),
        ..Fake::local()
    };
    for workers in 1..=Workers::MAX {
        plan.request.workers = Workers::new(workers).unwrap();
        let need = files_needed(workers);
        assert_eq!(
            checked_under(&plan, &at(256)).is_err(),
            need > 256,
            "{workers} workers need {need}"
        );
        let refused = checked_under(&plan, &at(need - 1)).unwrap_err();
        assert_eq!(
            refused,
            RunRefusal::OpenFilesShort {
                have: need - 1,
                need
            }
        );
        assert!(checked_under(&plan, &at(need)).is_ok(), "{workers}");
    }
    let words = RunRefusal::OpenFilesShort {
        have: 256,
        need: 300,
    }
    .to_string();
    assert!(words.contains("ulimit -n 300") && words.contains("launchctl limit maxfiles 300"));
    assert!(checked_under(&plan, &Fake::local()).is_ok(), "unlimited");
    assert!(
        files_needed(Workers::MAX) <= 256,
        "{}",
        files_needed(Workers::MAX)
    );
    nothing_written(&store);
    // The need never overflows, whatever is asked.
    assert!(files_needed(Workers::MAX) > files_needed(1));
    assert_eq!(files_needed(usize::MAX), u64::MAX);
}

/// RA-306's other half: the store's refusal carrying `EMFILE`'s number is
/// named `open_files_exhausted`, never the general `store_io`, with the hand
/// step in its words; any other number stays `store_io`.
#[test]
fn an_emfile_during_a_run_is_named_open_files_exhausted() {
    use crate::ingest::FailKind;
    let io = |code| store::file::StoreError::Io {
        path: PathBuf::from("/s/bars/x.bin"),
        action: store::file::Action::Open,
        kind: std::io::ErrorKind::Other,
        code,
    };
    let emfile = rustix::io::Errno::MFILE.raw_os_error();
    let kind = FailKind::of_store(&io(Some(emfile)));
    assert_eq!(kind, FailKind::OpenFilesExhausted);
    assert_eq!(kind.code(), "open_files_exhausted");
    assert!(!kind.is_hole());
    assert_eq!(FailKind::of_store(&io(Some(emfile + 1))).code(), "store_io");
    assert_eq!(FailKind::of_store(&io(None)).code(), "store_io");
    // std names EMFILE with no kind of its own: only the number tells it.
    let host = std::io::Error::from_raw_os_error(emfile);
    assert_eq!(host.raw_os_error(), Some(emfile));
    assert!(!matches!(
        host.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::StorageFull
    ));
}

/// Two extremes at once (a combination): a store that is read-only AND on
/// a network AND short of descriptors names the read-only volume first, the
/// order the module states; a missing year zip on such a store names the
/// source first, before any store check.
#[test]
fn refusals_come_in_their_stated_order_when_several_hold_at_once() {
    let root = scratch("ia3b-order");
    let store = store_at(&root);
    let plan = ts_plan(&root, &store).unwrap();
    let everything = Fake {
        volume: Ok(Volume {
            read_only: true,
            local: Some(false),
            fs: "nfs".to_owned(),
        }),
        open_files: Some(1),
        ..Fake::local()
    };
    assert!(matches!(
        checked_under(&plan, &everything),
        Err(RunRefusal::StoreReadOnly { .. })
    ));
    let network_short = Fake {
        volume: Ok(Volume {
            read_only: false,
            local: Some(false),
            fs: "nfs".to_owned(),
        }),
        open_files: Some(1),
        ..Fake::local()
    };
    assert!(matches!(
        checked_under(&plan, &network_short),
        Err(RunRefusal::StoreNotLocal { .. })
    ));
    // The legacy cutover before the store: an open key on a read-only store.
    let legacy = legacy_store(
        &scratch("ia3b-order-legacy"),
        "run 1700000000 source=zip\nbegin indices 2024-04-01 *\n",
    );
    let mut first = plan.clone();
    first.store_root = legacy.clone();
    assert!(matches!(
        checked_under(&first, &everything),
        Err(RunRefusal::LegacyRunMayBeLive { .. })
    ));
    // The source before everything: a missing year zip.
    let nfo = options_root(&root, &[("2024.zip", year_zip_of(&[(d1(), None)]))]);
    let refused = planned(
        asked(
            ImportKind::Options,
            SourceWord::Zip,
            (day(2023, 6, 1), d1()),
        ),
        &legacy,
        vec![(super::super::NFO_ARCHIVES_VAR, nfo)],
    );
    assert!(matches!(
        refused,
        Err(RunRefusal::YearZipMissing { year: 2023, .. })
    ));
}

// ═══ 8: where each source begins (FM-133) ═══════════════════════════════════

/// The calendar-open days of `from..=to` before `first`, by the calendar
/// itself.
fn trading_days_before(from: Day, first: Day) -> u32 {
    let mut count = 0;
    let mut d = from;
    while d < first {
        if matches!(
            crate::calendar::sourced_kind_of(i64::from(d.days_from_epoch())),
            crate::calendar::DayKind::Open(_) | crate::calendar::DayKind::OpenLengthUnmeasured
        ) {
            count += 1;
        }
        d = d.succ().unwrap();
    }
    count
}

/// FM-133: the plan states each source's first and last day from the day
/// table it builds, and counts the asked calendar-open days before the first
/// (and after the last) as `outside_source`, never holes; the journal is
/// unchanged. For every source kind: the tick store's two trees, the
/// capital-market zip and the year zips.
#[test]
fn days_before_a_sources_first_day_are_named_once() {
    let from = day(2024, 3, 18);
    let root = scratch("ia3b-span");
    let store = store_at(&root);
    let before = trading_days_before(from, d1());
    assert!(before >= 5, "{before}");
    let ts = tick_store(&root, &[d1(), d2()]);
    let cm = cm_archive(&root, &[(d1(), None), (d2(), None)]);
    let nfo = options_root(
        &root,
        &[("2024.zip", year_zip_of(&[(d1(), None), (d2(), None)]))],
    );
    let cases = [
        (
            ImportKind::Indices,
            SourceWord::TickStore,
            super::super::TICKSTORE_VAR,
            ts.clone(),
        ),
        (
            ImportKind::Indices,
            SourceWord::Zip,
            super::super::CM_ARCHIVE_VAR,
            cm,
        ),
        (
            ImportKind::Options,
            SourceWord::Zip,
            super::super::NFO_ARCHIVES_VAR,
            nfo,
        ),
    ];
    for (kind, source, var, path) in cases {
        let mut request = asked(kind, source, (from, day(2024, 4, 5)));
        request.free_floor = 0;
        let plan = planned(request, &store, vec![(var, path)]).unwrap();
        let checked = &plan.checks[0];
        assert_eq!(checked.span, Some((d1(), d2())), "{kind:?} {source:?}");
        assert_eq!(checked.first_day, Some(d1()));
        // 2024-04-03..05 are after the last day held.
        assert_eq!(checked.outside_source, before + 3, "{kind:?} {source:?}");
        let line = checked.line();
        assert!(
            line.contains("source_span=2024-04-01..2024-04-02")
                && line.contains(&format!("outside_source={}", before + 3)),
            "{line}"
        );
    }
    assert!(!journal_path(&store).exists(), "the journal is unchanged");
    // A source holding one day: its span is that day both ends.
    let one = tick_store(&scratch("ia3b-span-one"), &[d2()]);
    let plan = planned(
        asked(ImportKind::Indices, SourceWord::TickStore, (d2(), d2())),
        &store,
        vec![(super::super::TICKSTORE_VAR, one)],
    )
    .unwrap();
    assert_eq!(plan.checks[0].span, Some((d2(), d2())));
    assert_eq!(plan.checks[0].outside_source, 0);
    // A source that cannot be opened says nothing it did not measure.
    assert_eq!(outside(None, (from, d2())), 0);
}

// ═══ 9: the tick store's format digest (D-4819) ═════════════════════════════

/// D-4819: a `FORMAT.md` whose sha256 is neither of the charter's two is not
/// refused: the plan says `format=unpinned`, the cli warns, and the run line
/// carries `format_digest_unpinned=1`; a pinned or unmeasured digest, or a
/// zip source, carries no such field. The two pinned digests are the
/// charter's and the tick-store reader's own, copy for copy.
#[test]
fn an_unpinned_tick_store_format_digest_warns_and_marks_the_run_line() {
    let root = scratch("ia3b-digest");
    let store = store_at(&root);
    let plan = ts_plan(&root, &store).unwrap();
    let checked = &plan.checks[0];
    let Format::Unpinned(hex) = &checked.format else {
        panic!("{:?}", checked.format);
    };
    assert_eq!(hex, &plan.kinds[0].origin.digest_hex());
    assert!(checked.line().contains("format=unpinned"));
    let ends = super::super::execute(&plan, None);
    assert_eq!(ends.len(), 1);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    let run = text.lines().find(|l| l.starts_with("run ")).unwrap();
    assert!(
        run.contains(" format_digest_unpinned=1 definition="),
        "{run}"
    );
    let mut pinned = plan.kinds[0].origin.clone();
    for digest in PINNED_FORMAT_DIGESTS {
        let bytes: Vec<u8> = (0..32)
            .map(|i| u8::from_str_radix(&digest[i * 2..i * 2 + 2], 16).unwrap())
            .collect();
        pinned.digest = Some(bytes.try_into().unwrap());
        assert!(!unpinned(&pinned));
        assert_eq!(Format::of(&pinned), Format::Pinned);
        assert_eq!(pinned.digest_hex(), digest);
    }
    pinned.digest = None;
    assert_eq!(Format::of(&pinned), Format::Unmeasured);
    assert_eq!(Format::Unmeasured.word(), "unmeasured");
    pinned.source = SourceKind::Zip;
    assert_eq!(Format::of(&pinned), Format::NotATickStore);
    assert_eq!(Format::NotATickStore.word(), "-");
    assert!(!unpinned(&pinned));
    let read = |rel: &str| {
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
    };
    let (charter, reader) = (
        read("../../docs/00-charter.md"),
        read("src/gdfl_tickstore.rs"),
    );
    for digest in PINNED_FORMAT_DIGESTS {
        assert!(charter.contains(digest), "the charter lacks {digest}");
        assert!(reader.contains(digest), "gdfl_tickstore.rs lacks {digest}");
    }
}

// ═══ The request line and the words ═════════════════════════════════════════

/// `legacy_ack` crosses the request line only when set: every line without
/// it is byte for byte an earlier build's, and a line carrying it reads back
/// whole; a second acknowledgement is a field too many.
#[test]
fn the_acknowledgement_crosses_the_request_line_only_when_given() {
    let mut request = asked(ImportKind::Options, SourceWord::Zip, (d1(), d1()));
    let plain = request.to_line().unwrap();
    assert!(
        plain.ends_with(&format!(" free_floor={}", request.free_floor)),
        "{plain}"
    );
    request.legacy_ack = true;
    let line = request.to_line().unwrap();
    assert_eq!(line, format!("{plain} legacy_ack=1"));
    assert_eq!(Request::from_line(&line).unwrap(), request);
    assert!(matches!(
        Request::from_line(&format!("{line} legacy_ack=1")),
        Err(RunRefusal::RequestLineInvalid { .. })
    ));
    assert!(matches!(
        Request::from_line(&format!("{plain} legacy_ack=0")),
        Err(RunRefusal::RequestLineInvalid { .. })
    ));
}

/// Every new code is in `RunRefusal::CODES`, once, in the enum's order, and
/// none is a misuse; `say` writes the bare code for a refusal it has no
/// words of its own for.
#[test]
fn every_preflight_code_is_named_once_and_is_not_a_misuse() {
    struct Bare<'a>(&'a RunRefusal);
    impl core::fmt::Display for Bare<'_> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            say(self.0, f)
        }
    }
    let refusals = [
        RunRefusal::YearZipMissing {
            year: 1,
            near_miss: vec!["a".to_owned()],
        },
        RunRefusal::SourceUnreadable {
            path: PathBuf::from("/x"),
            mode: "0600".to_owned(),
        },
        RunRefusal::YearZipEmpty { year: 1 },
        RunRefusal::SourceBusy {
            root: PathBuf::from("/t"),
            holder: "h".to_owned(),
        },
        RunRefusal::CensusRoomShort {
            layout: "v3",
            free_now: 0,
            free_after: 0,
            need_first_day: 1,
        },
        RunRefusal::CensusCompactionNoDisk {
            need_bytes: 1,
            free_bytes: 0,
            floor: 0,
        },
        RunRefusal::LegacyRunMayBeLive {
            key: "k".to_owned(),
        },
        RunRefusal::StoreReadOnly {
            root: PathBuf::from("/s"),
        },
        RunRefusal::StoreNotLocal {
            root: PathBuf::from("/s"),
            fs: "nfs".to_owned(),
        },
        RunRefusal::OpenFilesShort { have: 1, need: 2 },
    ];
    let codes = RunRefusal::CODES;
    let tail = &codes[codes.len() - refusals.len()..];
    for (refusal, code) in refusals.iter().zip(tail) {
        assert_eq!(refusal.code(), *code);
        assert!(!refusal.misuse(), "{code}");
        let words = refusal.to_string();
        assert!(
            words.starts_with(&format!("{code}: ")) && words.len() > code.len() + 10,
            "{words}"
        );
    }
    let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();
    assert_eq!(unique.len(), codes.len());
    let busy = RunRefusal::RunBusy {
        lock: "l".to_owned(),
        holder: "h".to_owned(),
    };
    assert_eq!(Bare(&busy).to_string(), "run_busy");
}

// ═══ L-IA-35: the descriptors a run holds, measured ═════════════════════════

/// This process's open descriptors now, as the host lists them (Linux's
/// `/proc/self/fd`, macOS's `/dev/fd`), less the one the listing holds.
fn open_descriptors() -> usize {
    let dir = if Path::new("/proc/self/fd").is_dir() {
        "/proc/self/fd"
    } else {
        "/dev/fd"
    };
    std::fs::read_dir(dir).map_or(0, |names| names.count().saturating_sub(1))
}

/// An invented options day of `files` contracts (distinct strikes, so each
/// files its own contract-month) in a tick store under `root`.
fn options_tick_day(root: &Path, files: usize) -> PathBuf {
    let ts = root.join("ts");
    let d = d1();
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let bodies: Vec<(String, Vec<u8>)> = (0..files)
        .map(|i| {
            let ticker = format!("NIFTY04APR24{}CE", 21_000 + 50 * i);
            (
                format!("{folder}/Options/{ticker}.NFO.csv"),
                option_csv(&ticker, d),
            )
        })
        .collect();
    let refs: Vec<(&str, &[u8])> = bodies
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let path = NfoTickStore::new(&ts).day_path(d);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bts(&refs)).unwrap();
    put(&ts, "FORMAT.md", b"# an invented format, for a test\n");
    ts
}

/// The most descriptors this process held at once while it imported an
/// invented options day of `files` contracts with `workers` workers into a
/// fresh store, less what it held before: sampled as fast as one thread
/// lists them, so a lower bound of the true peak.
fn peak_descriptors(workers: usize, files: usize) -> usize {
    let root = scratch(&format!("ia3b-fds-{workers}"));
    let ts = options_tick_day(&root, files);
    let store = store_at(&root);
    let before = open_descriptors();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let sampler = {
        let done = std::sync::Arc::clone(&done);
        std::thread::spawn(move || {
            let mut most = 0;
            while !done.load(std::sync::atomic::Ordering::Relaxed) {
                most = most.max(open_descriptors());
            }
            most
        })
    };
    let run = crate::gdfl_import::Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    let options = crate::gdfl_import::RunOptions {
        workers: Workers::new(workers).unwrap(),
        free_floor: 0,
        ..crate::gdfl_import::RunOptions::default()
    };
    let report = crate::gdfl_import::run_nfo_from(&NfoTickStore::new(&ts), &run, options).unwrap();
    done.store(true, std::sync::atomic::Ordering::Relaxed);
    let most = sampler.join().unwrap();
    assert_eq!(report.files, files, "{:?}", report.failures);
    most.saturating_sub(before)
}

/// L-IA-35, IA-GR-08's count: the descriptors an import run holds at one and
/// at four workers on an invented options day of 400 contracts, sampled; the
/// run's own and each worker's are what [`FILES_PER_RUN`] and
/// [`FILES_PER_WORKER`] say, or fewer. Run alone (`--test-threads=1`), since
/// any other test's open files would be counted too.
#[test]
#[ignore = "a measurement: run alone with --ignored --test-threads=1 --nocapture"]
fn the_descriptors_a_run_holds_are_measured_at_one_and_four_workers() {
    let files = 400;
    let one = peak_descriptors(1, files);
    let four = peak_descriptors(4, files);
    let twelve = peak_descriptors(Workers::MAX, files);
    println!(
        "IA3B-FDS files={files} peak_w1={one} peak_w4={four} peak_w{}={twelve} \
         (descriptors beyond the {} held before)",
        Workers::MAX,
        open_descriptors()
    );
    for (workers, peak) in [(1, one), (4, four), (Workers::MAX, twelve)] {
        assert!(
            u64::try_from(peak).unwrap() <= files_needed(workers),
            "{workers} workers held {peak}, the model says {}",
            files_needed(workers)
        );
    }
}

// ═══ NEEDS THE MAC: the real roots, read only ═══════════════════════════════

/// The operator's roots from the three source variables, each `None` when
/// unset.
fn real_roots() -> (Option<PathBuf>, Option<PathBuf>, Option<PathBuf>) {
    let var = |name: &str| std::env::var_os(name).map(PathBuf::from);
    (
        var(crate::gdfl_run::TICKSTORE_VAR),
        var(crate::gdfl_run::CM_ARCHIVE_VAR),
        var(crate::gdfl_run::NFO_ARCHIVES_VAR),
    )
}

/// M1 and M6: every preflight check's answer and its time on the real roots,
/// for indices, stocks and options from each source set, over
/// 2018-09-03..2026-09-24. Read only: the store is `BRUTEX_IA3B_REAL_STORE`
/// when set (the preflight writes nothing on it, IA-GR-07; the log goes to a
/// scratch folder) or a fresh scratch store. The legacy acknowledgement is
/// given because the real journal's key `options 2020-09-16` ends in `begin`;
/// M5 proves that refusal apart. A source variable left unset is a source not
/// measured, said on its own line.
#[test]
#[ignore = "real data, NEEDS THE MAC M1: the three source variables and BRUTEX_IA3B_REAL_STORE"]
fn ia3b_real_preflight_prints_every_check_and_its_time() {
    let (ts, cm, nfo) = real_roots();
    let store = std::env::var_os("BRUTEX_IA3B_REAL_STORE")
        .map_or_else(|| store_at(&scratch("ia3b-real")), PathBuf::from);
    let log = LogState::Installed(scratch("ia3b-real-logs").join("logs"));
    let process = Process {
        store_root: &store,
        commit: None,
        log: &log,
        unstamped: None,
    };
    let range = (day(2018, 9, 3), day(2026, 9, 24));
    let mut env: Vec<(&'static str, PathBuf)> = Vec::new();
    for (name, root) in [
        (crate::gdfl_run::TICKSTORE_VAR, &ts),
        (crate::gdfl_run::CM_ARCHIVE_VAR, &cm),
        (crate::gdfl_run::NFO_ARCHIVES_VAR, &nfo),
    ] {
        match root {
            Some(root) => env.push((name, root.clone())),
            None => println!("IA3B-REAL {name} unset: its source is not measured"),
        }
    }
    let env = env_of(env);
    let mut planned_kinds = 0;
    for source in SourceWord::ALL {
        for kind in [ImportKind::Indices, ImportKind::Stocks, ImportKind::Options] {
            let held = match (source, kind) {
                (SourceWord::TickStore, _) => ts.is_some(),
                (SourceWord::Zip, ImportKind::Options) => nfo.is_some(),
                (SourceWord::Zip, _) => cm.is_some(),
            };
            if !held {
                continue;
            }
            let request = Request {
                legacy_ack: true,
                ..asked(kind, source, range)
            };
            let start = std::time::Instant::now();
            let answer = plan(request, &process, &env, &mut SourceCache::default());
            let took = start.elapsed();
            match &answer {
                Ok(plan) => {
                    for checked in &plan.checks {
                        println!(
                            "IA3B-REAL {} {:?}: {}",
                            source.as_str(),
                            took,
                            checked.line()
                        );
                    }
                }
                Err(why) => println!("IA3B-REAL {} {kind:?} {took:?}: {why}", source.as_str()),
            }
            assert!(
                answer.is_ok(),
                "{kind:?} from {}: {answer:?}",
                source.as_str()
            );
            planned_kinds += 1;
        }
    }
    println!("IA3B-REAL planned {planned_kinds} kind-source pairs");
}

/// A listing's stated lengths and entry names, as M4 reads them.
fn listed<L>(entries: &[crate::gdfl_cm::ListedFile<L>]) -> Vec<(u64, String)> {
    entries
        .iter()
        .map(|file| (file.len, file.entry.to_string()))
        .collect()
}

/// One container's day listings, as M4 reads them: each day's stated
/// lengths and entry names, `Ok(None)` for a day it holds no zip for.
type Lens<'a> = dyn FnMut(Day) -> Result<Option<Vec<(u64, String)>>, CmRefusal> + 'a;

/// M4: the largest member each real container states, beside the cap item 3
/// sets (`tickstore_write::MEMBER_CAP`): every day of 2018-09-03..2026-09-24
/// listed from the capital-market archive (both trees), the options year
/// zips and the tick store (all three trees), read only. Prints the largest
/// stated length and its entry per container and every day a listing refused
/// (counted and the first named, never skipped silently); fails if a member
/// is at or past the cap, since then a real day would be refused
/// `member_too_large`.
#[test]
#[ignore = "real data, NEEDS THE MAC M4: the three source variables"]
fn ia3b_real_largest_member_of_each_container() {
    use crate::gdfl_cm::CmSource as _;
    use crate::gdfl_nfo::NfoSource as _;
    let (ts, cm, nfo) = real_roots();
    let cap = crate::tickstore_write::MEMBER_CAP;
    let (from, to) = (day(2018, 9, 3), day(2026, 9, 24));
    let largest = |name: &str, lens: &mut Lens<'_>| {
        let (mut top, mut entry) = (0_u64, String::new());
        let (mut held, mut refused, mut first_refusal) = (0_u32, 0_u32, None);
        let mut d = from;
        while d <= to {
            match lens(d) {
                Ok(Some(files)) => {
                    held += 1;
                    for (len, at) in files {
                        if len > top {
                            (top, entry) = (len, at);
                        }
                    }
                }
                Ok(None) => {}
                Err(why) => {
                    refused += 1;
                    first_refusal.get_or_insert(format!("{d}: {why}"));
                }
            }
            d = d.succ().unwrap();
        }
        println!(
            "IA3B-MEMBER {name}: largest {top} B ({entry}), cap {cap} B, {held} days held, \
             {refused} refused (first: {first_refusal:?})"
        );
        assert!(
            top < cap,
            "{name}: {entry} states {top} B, at or past the cap"
        );
    };
    if let Some(cm) = &cm {
        let archive = crate::gdfl_archive::Archive::open(cm).unwrap();
        for kind in [CmKind::Indices, CmKind::Stocks] {
            largest(&format!("cm archive {kind:?}"), &mut |d| {
                archive
                    .day(kind, d)
                    .map(|listing| listing.map(|listing| listed(listing.entries())))
            });
        }
    }
    if let Some(nfo) = &nfo {
        let zips = crate::gdfl_nfo::NfoZips::new(nfo);
        largest("options year zips", &mut |d| {
            zips.day(d)
                .map(|listing| listing.map(|listing| listed(listing.entries())))
        });
    }
    if let Some(ts) = &ts {
        let store = crate::gdfl_tickstore::TickStore::new(ts);
        for kind in [CmKind::Indices, CmKind::Stocks] {
            largest(&format!("tick store {kind:?}"), &mut |d| {
                store
                    .day(kind, d)
                    .map(|listing| listing.map(|listing| listed(listing.entries())))
            });
        }
        let options = crate::gdfl_nfo::NfoTickStore::new(ts);
        largest("tick store options", &mut |d| {
            options
                .day(d)
                .map(|listing| listing.map(|listing| listed(listing.entries())))
        });
    }
}
