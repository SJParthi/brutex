//! `cli upkeep` on invented stores (R3-17, D-4997): a run that stops and
//! resumes where it stopped, an upkeep and an import shutting each other out
//! of a month and of the store, the sort index at one and eight workers, a
//! failed index open retried, the three impossible strikes retired (DB-027),
//! a whole feed indexed in one run (DB-N14), the recount by month, the plan,
//! the status, Stop, and every misuse. No vendor row is copied: every key,
//! price and count is made up.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::gdfl_import::ImportKind;
use pull::manifest::{Closes, Entry, EntryKey, Held, Manifest, epoch_path};
use pull::session::Day;
use pull::upkeep::{self, How, Kind, Outcome, State};
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::sort_index::{Shape, SortIndex};

use super::{Host, USAGE, all_released, command_with, outcome, rate, run_locks, spans};
use crate::gdfl_import_command::{Holder, RunLock};
use crate::{FAILED, MISUSED, OK};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A scratch store of this test's own, removed when it drops.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-upkeep-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    /// A second store holding a byte-for-byte copy of this one.
    fn twin(&self) -> Self {
        let twin = Self::new();
        copy_tree(&self.0, &twin.0);
        twin
    }

    fn journal(&self, kind: Kind) -> String {
        fs::read_to_string(self.0.join(upkeep::journal_name(Vendor::Gdfl, kind)))
            .unwrap_or_default()
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn copy_tree(from: &Path, to: &Path) {
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            fs::create_dir_all(&target).unwrap();
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).unwrap()
}

fn store_path<'a>(
    segment: &'a str,
    symbol: &'a str,
    contract: Option<&str>,
    month: YearMonth,
    file: FileKind,
) -> StorePath<'a> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: contract.map(|c| Contract::parse(c).unwrap()),
        timeframe: Timeframe::SECOND_1,
        month,
        file,
    })
    .unwrap()
}

/// Writes an invented one-second month of `count` bars in three appends;
/// past 1,024 bars the writer keeps its `.tix`, `.srt` and `.sum`, and
/// `bare` takes them away again: a month written before they existed.
fn month(root: &Path, at: StorePath<'_>, symbol: &str, count: i64, bare: bool) {
    let mut writer =
        BarFile::open_or_create(root, at, crate::srt_builder::symbol_id(symbol)).unwrap();
    let open = at.month().ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
    let contract = at.to_path_buf(root).to_string_lossy().contains("/FNO/");
    let rows: Vec<Bar> = (0..count)
        .map(|k| Bar {
            ts_micros: open + k * 1_000_000,
            open: 3_210_000 + k % 17,
            high: 3_210_050 + k % 17,
            low: 3_209_950 + k % 17,
            close: 3_210_010 + (k * 7) % 13,
            volume: (k * 11) % 23,
            open_interest: if contract { 77 + k % 3 } else { OI_NULL },
        })
        .collect();
    for chunk in rows.chunks(rows.len().div_ceil(3).max(1)) {
        writer.append(chunk).unwrap();
    }
    drop(writer);
    if bare {
        for file in [
            FileKind::TimeIndex,
            FileKind::SortIndex,
            FileKind::MonthSummary,
        ] {
            let _ = fs::remove_file(at.with_file(file).to_path_buf(root));
        }
    }
}

/// `n` bare NIFTY spot months from 2019-01 on, 1,200 bars each.
fn spot_months(root: &Path, n: u8) -> Vec<YearMonth> {
    (1..=n)
        .map(|m| {
            let at = ym(2019, m);
            month(
                root,
                store_path("INDEX", "NIFTY", None, at, FileKind::Bars),
                "NIFTY",
                1_200,
                true,
            );
            at
        })
        .collect()
}

fn srt(root: &Path, at: YearMonth) -> PathBuf {
    store_path("INDEX", "NIFTY", None, at, FileKind::SortIndex).to_path_buf(root)
}

/// Whether a reader finds `at`'s `.srt` as one run over every committed row.
fn indexed(root: &Path, at: StorePath<'_>, symbol: &str) -> bool {
    let file = BarFile::open_existing(root, at, crate::srt_builder::symbol_id(symbol)).unwrap();
    let header = file.header();
    let shape = Shape::of(at.month(), header.timeframe_secs, header.symbol_id);
    SortIndex::open(&file.path().with_extension("srt"), &shape)
        .is_ok_and(|index| index.runs().len() == 1 && index.coverage() == header.n_valid)
}

fn noop(_: &str, _: &Outcome) {}

/// Runs the verb on `root` with a fixed clock, the given Stop and hook.
fn call(
    root: &Path,
    arguments: &[&str],
    stop: &AtomicBool,
    after: &(dyn Fn(&str, &Outcome) + Sync),
) -> (u8, String) {
    let owned = root.to_path_buf();
    let store = move || Ok(owned.clone());
    let host = Host {
        store: &store,
        commit: Some("0123456789abcdef0123456789abcdef01234567"),
        now: &|| 1_000,
        stop,
        after_item: after,
    };
    let mut out = String::new();
    let code = command_with(arguments, &host, &mut out);
    (code, out)
}

fn plain(root: &Path, arguments: &[&str]) -> (u8, String) {
    call(root, arguments, &AtomicBool::new(false), &noop)
}

/// The keys of run `id`'s item lines, in file order.
fn items_of(text: &str, id: u64) -> Vec<String> {
    text.lines()
        .filter_map(upkeep::parse)
        .filter_map(|entry| match entry {
            upkeep::Entry::Item { id: of, key, .. } if of == id => Some(key),
            _ => None,
        })
        .collect()
}

fn key(at: YearMonth) -> String {
    format!("bars/gdfl/NSE/INDEX/NIFTY/1s/{at}.bin")
}

// ═══ the named tests ════════════════════════════════════════════════════════

/// **AN UPKEEP RUN STOPS AND RESUMES WHERE IT STOPPED.** A sort-index run
/// over eight bare months is stopped after its third: it ends `stopped`,
/// three months are indexed, five are not, and the journal says so line by
/// line. The resume, named by the run's id, builds exactly the five — none of
/// the three again — and ends clean, and the store is then byte for byte what
/// one uninterrupted run makes. With `--all`, the resume skips the three by
/// the chain's own lines.
#[test]
fn an_upkeep_run_stops_and_resumes_where_it_stopped() {
    let root = Root::new();
    let months = spot_months(&root.0, 8);
    let whole = root.twin();
    let stop = AtomicBool::new(false);
    let built = AtomicUsize::new(0);
    let stopping = |_: &str, _: &Outcome| {
        if built.fetch_add(1, Ordering::Relaxed) + 1 == 3 {
            stop.store(true, Ordering::Release);
        }
    };
    let (code, out) = call(
        &root.0,
        &["run", "srt", "gdfl", "NSE/INDEX/NIFTY"],
        &stop,
        &stopping,
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("ENDED stopped: a Stop was asked; 5 item(s) left for a resume of run 1000"),
        "{out}"
    );
    assert!(out.contains("(3 built, 3 clean, 5 left)"), "{out}");
    assert!(
        out.contains("resume with: cli upkeep resume srt gdfl 1000"),
        "{out}"
    );
    for (at, &month) in months.iter().enumerate() {
        assert_eq!(srt(&root.0, month).exists(), at < 3, "{month}");
    }
    let text = root.journal(Kind::SortIndex);
    assert_eq!(
        items_of(&text, 1_000),
        months[..3].iter().map(|&m| key(m)).collect::<Vec<_>>()
    );
    let (code, out) = plain(&root.0, &["status", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("run 1000 ended stopped: 3 of 8 built, 3 clean"),
        "{out}"
    );
    // THE RESUME.
    let (code, out) = plain(
        &root.0,
        &["resume", "srt", "gdfl", "1000", "NSE/INDEX/NIFTY"],
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("run 1001 (resumes run 1000): 1 worker(s), 5 item(s) given"),
        "{out}"
    );
    assert!(out.contains("ENDED done"), "{out}");
    let text = root.journal(Kind::SortIndex);
    assert_eq!(
        items_of(&text, 1_001),
        months[3..].iter().map(|&m| key(m)).collect::<Vec<_>>()
    );
    // AS ONE UNINTERRUPTED RUN WOULD HAVE LEFT IT.
    let (code, out) = plain(&whole.0, &["run", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    for &month in &months {
        assert_eq!(
            fs::read(srt(&root.0, month)).unwrap(),
            fs::read(srt(&whole.0, month)).unwrap(),
            "{month}"
        );
    }
    // `--all`: every month is an item, and the chain's clean lines are skipped.
    stop.store(false, Ordering::Release);
    built.store(0, Ordering::Relaxed);
    let (code, out) = call(&root.0, &["run", "srt", "gdfl", "--all"], &stop, &stopping);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("run 1002: 1 worker(s), 8 item(s) given"),
        "{out}"
    );
    let (code, out) = plain(&root.0, &["resume", "srt", "gdfl", "1002", "--all"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("5 item(s) given, 3 skipped as closed clean"),
        "{out}"
    );
    assert!(out.contains("outcomes            kept 5"), "{out}");
}

/// **AN UPKEEP AND AN IMPORT EXCLUDE EACH OTHER ON A MONTH**, and on the
/// store. A month an import's writer holds is refused by the store's own
/// lock and named; every other month is built; the run is not clean, and its
/// resume builds the month once the writer is gone. While an import holds its
/// run lock, an upkeep run is refused `run_busy` and writes nothing; while an
/// upkeep run holds the locks, an import of every kind is refused `run_busy`
/// naming the upkeep — on the version-3 census's one lock and on version 4's
/// three.
#[test]
fn upkeep_and_an_import_exclude_each_other_on_a_month() {
    let root = Root::new();
    let months = spot_months(&root.0, 3);
    let held_month = store_path("INDEX", "NIFTY", None, months[1], FileKind::Bars);
    // THE IMPORT'S WRITER HOLDS THE MONTH. Its open keeps the month's own
    // `.srt` (the writer's door), so the run is asked for every month.
    let writer =
        BarFile::open_or_create(&root.0, held_month, crate::srt_builder::symbol_id("NIFTY"))
            .unwrap();
    let kept = fs::read(srt(&root.0, months[1])).unwrap();
    let (code, out) = plain(&root.0, &["run", "srt", "gdfl", "--all"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("NOT CLEAN bars/gdfl/NSE/INDEX/NIFTY/1s/2019-02.bin"),
        "{out}"
    );
    assert!(out.contains("is held by another process"), "{out}");
    assert!(out.contains("(3 built, 2 clean, 0 left)"), "{out}");
    assert!(srt(&root.0, months[0]).exists() && srt(&root.0, months[2]).exists());
    assert_eq!(
        fs::read(srt(&root.0, months[1])).unwrap(),
        kept,
        "nothing written to a month a writer holds"
    );
    drop(writer);
    let (code, out) = plain(&root.0, &["resume", "srt", "gdfl", "1000", "--all"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("1 item(s) given"), "{out}");
    assert!(indexed(&root.0, held_month, "NIFTY"));
    // THE RUN LOCK, v3: one for the store.
    let import = Holder {
        pid: 4_242,
        run_started: 7,
        kind: ImportKind::Options,
        verb: "gdfl-import options zip 2019-01-01 2019-01-31".to_owned(),
    };
    let lock = RunLock::take(&root.0, &import).unwrap();
    let before = root.journal(Kind::DayIndex);
    let (code, out) = plain(&root.0, &["run", "tix", "gdfl", "--all"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("REFUSED: run_busy: another options run holds imports/gdfl.lock: pid 4242"),
        "{out}"
    );
    assert_eq!(root.journal(Kind::DayIndex), before, "nothing written");
    lock.release().unwrap();
    let held = run_locks(&root.0, Vendor::Gdfl, "upkeep run srt gdfl", 9).unwrap();
    assert_eq!(held.len(), 1, "v3: the one store lock");
    let refused = RunLock::take(&root.0, &import).unwrap_err();
    assert_eq!(refused.code, "run_busy");
    assert!(
        refused.words.contains("verb=upkeep run srt gdfl"),
        "{}",
        refused.words
    );
    drop(held);
    // v4: one lock a kind, and the upkeep takes all three.
    fs::create_dir_all(root.0.join("manifest/v4/gdfl")).unwrap();
    let held = run_locks(&root.0, Vendor::Gdfl, "upkeep run recount gdfl", 9).unwrap();
    assert_eq!(held.len(), 3);
    for kind in ImportKind::ALL {
        let contender = Holder {
            kind,
            ..import.clone()
        };
        let refused = RunLock::take(&root.0, &contender).unwrap_err();
        assert_eq!(refused.code, "run_busy", "{kind:?}");
        assert!(
            refused
                .words
                .contains(&format!("imports/gdfl.{}.lock", kind.as_str())),
            "{}",
            refused.words
        );
    }
    drop(held);
    let stocks = RunLock::take(
        &root.0,
        &Holder {
            kind: ImportKind::Stocks,
            ..import.clone()
        },
    )
    .unwrap();
    let why = run_locks(&root.0, Vendor::Gdfl, "upkeep run srt gdfl", 9).unwrap_err();
    assert!(
        why.starts_with("run_busy: another stocks run holds imports/gdfl.stocks.lock"),
        "{why}"
    );
    stocks.release().unwrap();
    // Zerodha has no import run lock.
    assert!(
        run_locks(&root.0, Vendor::Zerodha, "upkeep", 9)
            .unwrap()
            .is_empty()
    );
}

/// **THE SORT INDEX IS THE SAME BYTES AT 1 AND 8 WORKERS**, and so are the
/// day index and the month summary: two copies of one store, one built by one
/// worker and one by eight, hold the same files byte for byte, and neither
/// changed a bar.
#[test]
fn the_sort_index_is_the_same_bytes_at_1_and_8_workers() {
    let one = Root::new();
    let months = spot_months(&one.0, 12);
    for (at, contract) in ["2019-03-28-1150000-CE", "2019-03-28-1160000-PE"]
        .into_iter()
        .enumerate()
    {
        let count = 1_100 + i64::try_from(at).unwrap() * 37;
        month(
            &one.0,
            store_path("FNO", "NIFTY", Some(contract), ym(2019, 3), FileKind::Bars),
            "NIFTY",
            count,
            true,
        );
    }
    let eight = one.twin();
    let bars_before: Vec<Vec<u8>> = months
        .iter()
        .map(|&m| {
            fs::read(store_path("INDEX", "NIFTY", None, m, FileKind::Bars).to_path_buf(&one.0))
                .unwrap()
        })
        .collect();
    for kind in ["srt", "tix", "msum"] {
        let (code, out) = plain(&one.0, &["run", kind, "gdfl", "--workers", "1"]);
        assert_eq!(code, OK, "{kind}: {out}");
        assert!(out.contains("1 worker(s), 14 item(s) given"), "{out}");
        let (code, out) = plain(&eight.0, &["run", kind, "gdfl", "--workers", "8"]);
        assert_eq!(code, OK, "{kind}: {out}");
        assert!(out.contains("8 worker(s), 14 item(s) given"), "{out}");
        assert!(out.contains("outcomes            built 14"), "{out}");
    }
    let files = |root: &Path| -> BTreeMap<PathBuf, Vec<u8>> {
        let mut found = BTreeMap::new();
        let mut stack = vec![root.join("bars")];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .is_some_and(|e| e == "srt" || e == "tix" || e == "sum" || e == "bin")
                {
                    found.insert(
                        path.strip_prefix(root).unwrap().to_path_buf(),
                        fs::read(&path).unwrap(),
                    );
                }
            }
        }
        found
    };
    let (a, b) = (files(&one.0), files(&eight.0));
    assert_eq!(a.keys().collect::<Vec<_>>(), b.keys().collect::<Vec<_>>());
    assert_eq!(
        a.keys()
            .filter(|p| p.extension().is_some_and(|e| e == "srt"))
            .count(),
        14
    );
    for (path, bytes) in &a {
        assert_eq!(Some(bytes), b.get(path), "{}", path.display());
    }
    for (&m, before) in months.iter().zip(&bars_before) {
        let now =
            fs::read(store_path("INDEX", "NIFTY", None, m, FileKind::Bars).to_path_buf(&one.0))
                .unwrap();
        assert_eq!(&now, before, "the bars of {m} never change");
    }
}

/// **A FAILED INDEX OPEN IS RETRIED.** A month whose index the store's door
/// cannot open — its `.srt` name held by a directory — is refused by name, is
/// NOT closed clean, and so is built by the resume once the name is free; the
/// run never remembers a failure as done. (The reader half — the API's view
/// cache never keeps a host's refusal — is
/// `api::bars_sort_tests::a_host_refusal_of_an_index_is_not_remembered`.)
#[test]
fn a_failed_index_open_is_retried() {
    let root = Root::new();
    let months = spot_months(&root.0, 2);
    let name = srt(&root.0, months[0]);
    fs::create_dir_all(&name).unwrap();
    let (code, out) = plain(&root.0, &["run", "srt", "gdfl", "--all"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("NOT CLEAN bars/gdfl/NSE/INDEX/NIFTY/1s/2019-01.bin"),
        "{out}"
    );
    assert!(out.contains("(2 built, 1 clean, 0 left)"), "{out}");
    let text = root.journal(Kind::SortIndex);
    assert!(
        text.contains(&format!("clean=0 word=REFUSED key={}", key(months[0]))),
        "{text}"
    );
    // A plain rerun does not see the month as lacking — its name is held —
    // and the resume does: the month was never closed clean.
    let aside = name.with_extension("srt-aside");
    fs::rename(&name, &aside).unwrap();
    let (code, out) = plain(&root.0, &["resume", "srt", "gdfl", "1000", "--all"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("1 item(s) given, 1 skipped as closed clean"),
        "{out}"
    );
    assert!(indexed(
        &root.0,
        store_path("INDEX", "NIFTY", None, months[0], FileKind::Bars),
        "NIFTY"
    ));
    let text = root.journal(Kind::SortIndex);
    assert_eq!(items_of(&text, 1_001), [key(months[0])]);
}

// ═══ DB-027: the three impossible strikes ═══════════════════════════════════

fn option(root: &Path, contract: &str, day: Day, second: u32) -> Held {
    option_of(root, "NIFTY", contract, day, second)
}

fn option_of(root: &Path, underlying: &str, contract: &str, day: Day, second: u32) -> Held {
    let at = day.year_month().unwrap();
    let ts = (i64::from(day.days_from_epoch()) * 86_400 - 19_800 + i64::from(second)) * 1_000_000;
    let bar = Bar {
        ts_micros: ts,
        open: 2_100,
        high: 2_100,
        low: 2_100,
        close: 2_100,
        volume: 1,
        open_interest: 525,
    };
    let path = store_path("FNO", underlying, Some(contract), at, FileKind::Bars);
    let mut file =
        BarFile::open_or_create(root, path, crate::srt_builder::symbol_id(underlying)).unwrap();
    file.append(&[bar]).unwrap();
    let header = file.header();
    Held::new(
        Entry {
            key: EntryKey {
                contract: Contract::parse(contract),
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                symbol: Symbol::new(underlying).unwrap(),
                timeframe: Timeframe::SECOND_1,
                month: at,
            },
            rows: header.n_valid,
            first_ts_micros: header.first_ts_micros,
            last_ts_micros: header.last_ts_micros,
        },
        Closes::known(2_100, 2_100).unwrap(),
    )
}

const IMPOSSIBLE: [&str; 3] = [
    "2020-06-25-191250000-CE",
    "2020-06-25-191270000-CE",
    "2020-06-25-191300000-CE",
];

/// **DB-027.** The three NIFTY `25JUN20` strikes of 2019-05-30 the
/// pre-D-3331 import stored (₹19,12,500, 19,12,700 and 19,13,000 with NIFTY
/// near 11,950) are listed by the census until an upkeep re-key run of NIFTY
/// retires them: then the lineage ledger names each `retired` and the
/// census's next FNO epoch lists none of them, while the plausible strikes of
/// that day stay. A second run changes nothing.
#[test]
fn the_three_impossible_strikes_are_retired_by_an_upkeep_rekey_run() {
    let root = Root::new();
    let day = Day::new(2019, 5, 30).unwrap();
    let mut rows = Vec::new();
    for (at, contract) in [
        "2019-05-30-1150000-CE",
        "2019-05-30-1170000-PE",
        "2019-05-30-1180000-CE",
    ]
    .into_iter()
    .chain(IMPOSSIBLE)
    .enumerate()
    {
        rows.push(option(
            &root.0,
            contract,
            day,
            37_845 + u32::try_from(at).unwrap(),
        ));
    }
    // An underlying whose ladder is all plausible: nothing to change.
    rows.push(option_of(
        &root.0,
        "BANKNIFTY",
        "2019-05-30-2900000-CE",
        day,
        37_900,
    ));
    rows.push(option_of(
        &root.0,
        "BANKNIFTY",
        "2019-05-30-3000000-PE",
        day,
        37_901,
    ));
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 64).unwrap();
    for row in rows {
        census.record_held(row).unwrap();
    }
    let path = epoch_path(&root.0, Vendor::Gdfl, Segment::Fno, 1);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, census.image()).unwrap();
    let listed = |epoch: u32| -> Vec<String> {
        let bytes = fs::read(epoch_path(&root.0, Vendor::Gdfl, Segment::Fno, epoch)).unwrap();
        let census = Manifest::open_image(Vendor::Gdfl, &bytes).unwrap();
        census
            .held_keys()
            .filter_map(|key| key.contract.map(|c| c.as_str().to_owned()))
            .collect()
    };
    for impossible in IMPOSSIBLE {
        assert!(
            listed(1).iter().any(|c| c == impossible),
            "before: {impossible} is listed"
        );
    }
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl", "--underlying", "NIFTY"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("outcomes            applied 1"), "{out}");
    let text = root.journal(Kind::Rekey);
    assert!(
        text.contains(
            "word=applied key=NIFTY why=6 key(s) in 6 month file(s): 3 kept, 0 to move, 3 to retire"
        ),
        "{text}"
    );
    let ledger = fs::read_to_string(pull::gdfl_lineage::lineage_path(&root.0)).unwrap();
    for impossible in IMPOSSIBLE {
        assert!(
            ledger
                .lines()
                .any(|line| line.contains("retired") && line.contains(impossible)),
            "{ledger}"
        );
        assert!(
            !listed(2).iter().any(|c| c == impossible),
            "after: {impossible} is not listed"
        );
    }
    assert_eq!(
        listed(2).len(),
        5,
        "the plausible strikes stay, of both underlyings"
    );
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("outcomes            kept 2"), "{out}");
    let text = root.journal(Kind::Rekey);
    assert!(text.contains("key=BANKNIFTY why=2 key(s) in 2 month file(s): 2 kept, 0 to move, 0 to retire; nothing to change"), "{text}");
    assert!(text.contains("key=NIFTY why=6 key(s) in 6 month file(s): 3 kept, 0 to move, 3 to retire; already applied, nothing was written"), "{text}");
    assert_eq!(
        fs::read_to_string(pull::gdfl_lineage::lineage_path(&root.0)).unwrap(),
        ledger
    );
}

/// A re-key item whose survey cannot read a file writes nothing and is not
/// clean; an underlying the store holds no directory for is refused.
#[test]
fn a_rekey_that_cannot_read_every_file_writes_nothing() {
    let root = Root::new();
    let day = Day::new(2019, 5, 30).unwrap();
    let _ = option(&root.0, IMPOSSIBLE[0], day, 40_000);
    let _ = option(&root.0, "2019-05-30-1150000-CE", day, 40_001);
    let damaged = store_path(
        "FNO",
        "NIFTY",
        Some("2019-05-30-1150000-CE"),
        ym(2019, 5),
        FileKind::Bars,
    );
    fs::write(damaged.to_path_buf(&root.0), b"not a bar file").unwrap();
    // Beside the underlyings: a file and a name no symbol spells. Neither is
    // an item.
    fs::write(root.0.join("bars/gdfl/NSE/FNO/README"), "").unwrap();
    fs::create_dir_all(root.0.join("bars/gdfl/NSE/FNO/nifty")).unwrap();
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("NOT_APPLIED 1"), "{out}");
    assert!(out.contains("1 file(s) unreadable"), "{out}");
    assert!(
        !pull::gdfl_lineage::lineage_path(&root.0).exists(),
        "nothing written"
    );
    let (code, out) = plain(
        &root.0,
        &["run", "rekey", "gdfl", "--underlying", "BANKNIFTY"],
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("holds no option directory for BANKNIFTY"),
        "{out}"
    );
    let empty = Root::new();
    let (code, out) = plain(&empty.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("cannot list"), "{out}");
}

// ═══ DB-N14: a whole feed's sort index ══════════════════════════════════════

/// **DB-N14, the store half.** One run over the whole feed gives every month
/// past the small-month threshold its sort index — spot months of two
/// indices and option contracts alike — so a sorted page reads its window
/// through it and no month is scanned; a month under 1,024 bars is `small`
/// and clean. Afterwards the plan finds nothing left to build.
#[test]
fn a_whole_feed_gets_its_sort_index_in_one_run() {
    let root = Root::new();
    let months = spot_months(&root.0, 4);
    let bank = store_path("INDEX", "BANKNIFTY", None, ym(2020, 3), FileKind::Bars);
    month(&root.0, bank, "BANKNIFTY", 1_500, true);
    let contract = store_path(
        "FNO",
        "NIFTY",
        Some("2019-03-28-1150000-CE"),
        ym(2019, 3),
        FileKind::Bars,
    );
    month(&root.0, contract, "NIFTY", 1_300, true);
    let small = store_path("INDEX", "NIFTY", None, ym(2020, 1), FileKind::Bars);
    month(&root.0, small, "NIFTY", 40, true);
    let (code, out) = plain(&root.0, &["plan", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("TOTAL lacks .tix 7, .srt 7, .sum 7 of 7 month(s)"),
        "{out}"
    );
    let (code, out) = plain(&root.0, &["run", "srt", "gdfl", "--workers", "4"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("outcomes            built 6, small 1"),
        "{out}"
    );
    assert!(out.contains("item(s) a second (7 in "), "{out}");
    for &m in &months {
        assert!(
            indexed(
                &root.0,
                store_path("INDEX", "NIFTY", None, m, FileKind::Bars),
                "NIFTY"
            ),
            "{m}"
        );
    }
    assert!(indexed(&root.0, bank, "BANKNIFTY"));
    assert!(indexed(&root.0, contract, "NIFTY"));
    assert!(
        !small
            .with_file(FileKind::SortIndex)
            .to_path_buf(&root.0)
            .exists(),
        "D-4933"
    );
    let (code, out) = plain(&root.0, &["plan", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("TOTAL lacks .tix 7, .srt 1, .sum 7 of 7 month(s)"),
        "{out}"
    );
    assert!(
        out.contains("srt      run 1000 ended done: 7 of 7 built, 7 clean"),
        "{out}"
    );
}

// ═══ the recount ════════════════════════════════════════════════════════════

/// The recount runs one calendar month of the days asked an item, writes
/// the census rows each month's headers give, stops between months and
/// resumes with the months left; a span is split at month ends.
#[test]
fn a_recount_runs_one_month_of_days_an_item_and_resumes() {
    let root = Root::new();
    for at in [ym(2019, 11), ym(2019, 12)] {
        let path = store_path(
            "FNO",
            "NIFTY",
            Some("2019-12-26-1200000-CE"),
            at,
            FileKind::Bars,
        );
        month(&root.0, path, "NIFTY", 30, false);
    }
    let stop = AtomicBool::new(false);
    let once = |_: &str, _: &Outcome| stop.store(true, Ordering::Release);
    let arguments = [
        "run",
        "recount",
        "gdfl",
        "--from",
        "2019-11-14",
        "--to",
        "2019-12-31",
    ];
    let (code, out) = call(&root.0, &arguments, &stop, &once);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("ENDED stopped"), "{out}");
    let text = root.journal(Kind::Recount);
    assert!(
        text.contains("word=recounted key=2019-11-14..2019-11-30 why="),
        "{text}"
    );
    let (code, out) = plain(
        &root.0,
        &[
            "resume",
            "recount",
            "gdfl",
            "1000",
            "--from",
            "2019-11-14",
            "--to",
            "2019-12-31",
        ],
    );
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("1 item(s) given, 1 skipped"), "{out}");
    assert!(
        root.journal(Kind::Recount)
            .contains("key=2019-12-01..2019-12-31")
    );
    let census = fs::read(pull::manifest::manifest_path(&root.0, Vendor::Gdfl)).unwrap();
    let census = Manifest::open_image(Vendor::Gdfl, &census).unwrap();
    assert_eq!(census.held_keys().count(), 2, "both months recorded");
    let day = |text: &str| crate::census_recount::day(text).unwrap();
    let split = spans(day("2019-11-14"), day("2020-01-02"));
    let words: Vec<String> = split
        .iter()
        .map(|span| format!("{}..{}", span.from, span.to.unwrap()))
        .collect();
    assert_eq!(
        words,
        [
            "2019-11-14..2019-11-30",
            "2019-12-01..2019-12-31",
            "2020-01-01..2020-01-02"
        ]
    );
    assert_eq!(spans(day("2019-11-14"), day("2019-11-14")).len(), 1);
    assert_eq!(
        spans(day("9999-12-31"), day("9999-12-31")).len(),
        1,
        "the last day a Day names"
    );
    // A recount the census refuses is not clean.
    fs::write(
        pull::manifest::manifest_path(&root.0, Vendor::Gdfl),
        b"BRUTEXM damaged",
    )
    .unwrap();
    let (code, out) = plain(&root.0, &arguments);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("REFUSED 2"), "{out}");
}

// ═══ the plan, the status, Stop and misuse ══════════════════════════════════

/// The plan names each series once, its months and what they lack, counts
/// contracts under their symbol, names what the walk left alone and how each
/// kind's last run ended, and writes nothing.
#[test]
fn the_plan_lists_what_each_series_lacks_and_writes_nothing() {
    let root = Root::new();
    spot_months(&root.0, 2);
    month(
        &root.0,
        store_path("INDEX", "NIFTY", None, ym(2019, 3), FileKind::Bars),
        "NIFTY",
        1_200,
        false,
    );
    month(
        &root.0,
        store_path(
            "FNO",
            "NIFTY",
            Some("2019-03-28-1150000-CE"),
            ym(2019, 3),
            FileKind::Bars,
        ),
        "NIFTY",
        10,
        false,
    );
    fs::write(root.0.join("bars/gdfl/NSE/INDEX/stray.txt"), "").unwrap();
    let (code, out) = plain(&root.0, &["plan", "gdfl", "NSE/INDEX"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("UPKEEP PLAN — gdfl NSE/INDEX under"), "{out}");
    assert!(out.contains("NSE/INDEX/NIFTY 1s"), "{out}");
    assert!(
        out.contains("3 month(s); lacks .tix      2, .srt      2, .sum      2"),
        "{out}"
    );
    assert!(
        out.contains("NOT WALKED: 0 symbolic link(s), never followed, and 1 name(s)"),
        "{out}"
    );
    assert!(out.contains("stray.txt"), "{out}");
    assert!(out.contains("rekey: 1 option underlying(s)"), "{out}");
    assert!(out.contains("tix      no run yet"), "{out}");
    let (code, out) = plain(
        &root.0,
        &[
            "plan", "gdfl", "--rung", "1s", "--from", "2019-03", "--to", "2019-03",
        ],
    );
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("(the whole feed)"), "{out}");
    assert!(out.contains("NSE/FNO/NIFTY/* 1s"), "{out}");
    assert!(!root.0.join("audit").exists(), "the plan writes nothing");
    // Many series: past the shown ones they are counted.
    for symbol in 0..(super::SHOWN + 2) {
        let name = format!("S{symbol:04}");
        let at = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "CASH",
            symbol: &name,
            contract: None,
            timeframe: Timeframe::DAY_1,
            month: ym(2019, 1),
            file: FileKind::Bars,
        })
        .unwrap();
        let open = ym(2019, 1).ist_bounds_micros().0 + 86_400 * 1_000_000;
        let mut writer =
            BarFile::open_or_create(&root.0, at, crate::srt_builder::symbol_id(&name)).unwrap();
        writer
            .append(&[Bar {
                ts_micros: open,
                open: 1,
                high: 1,
                low: 1,
                close: 1,
                volume: 1,
                open_interest: OI_NULL,
            }])
            .unwrap();
    }
    let (code, out) = plain(&root.0, &["plan", "gdfl", "NSE/CASH"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("... and 2 more series"), "{out}");
    // A plan of a feed the store does not hold, and a journal unreadable.
    let (code, out) = plain(&root.0, &["plan", "zerodha"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("UPKEEP PLAN REFUSED"), "{out}");
    fs::create_dir_all(root.0.join(upkeep::journal_name(Vendor::Gdfl, Kind::Rekey))).unwrap();
    let (code, out) = plain(&root.0, &["plan", "gdfl", "NSE/INDEX"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("rekey    UNREADABLE"), "{out}");
}

/// Status reads the newest run; Stop asks the running run by name, refuses
/// to ask when none runs, and says when the run it was told to stop is not
/// running.
#[test]
fn status_and_stop_read_and_ask_the_newest_run() {
    let root = Root::new();
    let (code, out) = plain(&root.0, &["status", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("srt      no run yet"), "{out}");
    let (code, out) = plain(&root.0, &["stop", "srt", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("no srt run of gdfl is running; nothing was asked"),
        "{out}"
    );
    spot_months(&root.0, 1);
    let (code, _) = plain(&root.0, &["run", "srt", "gdfl"]);
    assert_eq!(code, OK);
    let (code, out) = plain(&root.0, &["stop", "srt", "gdfl", "1000"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("for run 1000, which is NOT running: it stops nothing"),
        "{out}"
    );
    // A run that holds its journal: the stop names it.
    let path = root
        .0
        .join(upkeep::journal_name(Vendor::Gdfl, Kind::SortIndex));
    let file = fs::OpenOptions::new()
        .read(true)
        .append(true)
        .open(&path)
        .unwrap();
    (&file).write_all(b"run 2000 v1 vendor=gdfl kind=srt commit=x workers=1 asked=1 items=9 skipped=0 plan=0 resume=- at=2000\nitem 2000 n=3 ok=3 of=9 t=6 clean=1 word=built key=k why=w\n").unwrap();
    file.lock().unwrap();
    let (code, out) = plain(&root.0, &["status", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("run 2000 RUNNING: 3 of 9 built, 3 clean, 6 s, about 12 s left"),
        "{out}"
    );
    let (code, out) = plain(&root.0, &["stop", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("asked run 2000 to stop"), "{out}");
    assert_eq!(
        fs::read_to_string(
            root.0
                .join(upkeep::stop_name(Vendor::Gdfl, Kind::SortIndex))
        )
        .unwrap(),
        "stop 2000\n"
    );
    drop(file);
    let (code, out) = plain(&root.0, &["status", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("run 2000 DIED (no end line, and its lock is free)"),
        "{out}"
    );
    // An unreadable journal refuses status and stop; a stop name that cannot
    // be written refuses the stop.
    let bad = root
        .0
        .join(upkeep::journal_name(Vendor::Gdfl, Kind::DayIndex));
    fs::create_dir_all(&bad).unwrap();
    let (code, out) = plain(&root.0, &["status", "tix", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("UPKEEP STATUS REFUSED"), "{out}");
    let (code, out) = plain(&root.0, &["stop", "tix", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("UPKEEP STOP REFUSED"), "{out}");
    let staging = root
        .0
        .join(upkeep::stop_name(Vendor::Gdfl, Kind::MonthSummary))
        .with_extension("stop.new");
    fs::create_dir_all(&staging).unwrap();
    let (code, out) = plain(&root.0, &["stop", "msum", "gdfl", "5"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("UPKEEP STOP REFUSED"), "{out}");
}

/// Every misuse is refused by name with the usage, before anything is
/// opened; a store root that cannot be had is refused after.
#[test]
fn a_misuse_is_refused_before_anything_is_opened() {
    let root = Root::new();
    for arguments in [
        &[][..],
        &["plan"],
        &["walk", "srt", "gdfl"],
        &["run", "sort", "gdfl"],
        &["run", "srt", "nse"],
        &["run", "srt", "gdfl", "NSE/INDEX", "NSE/FNO"],
        &["run", "srt", "gdfl", "NSE//NIFTY"],
        &["run", "srt", "gdfl", "NSE/../X"],
        &["run", "srt", "gdfl", "NSE/./X"],
        &["run", "srt", "gdfl", "A/B/C/D/E"],
        &["run", "srt", "gdfl", "--rung", "1s", "--rung", "1s"],
        &["run", "srt", "gdfl", "--rung", "7s"],
        &["run", "srt", "gdfl", "--rung"],
        &["run", "srt", "gdfl", "--from", "2019-13"],
        &["run", "srt", "gdfl", "--to", "2019"],
        &["run", "srt", "gdfl", "--from", "2019-05", "--to", "2019-04"],
        &["run", "srt", "gdfl", "--workers", "0"],
        &["run", "srt", "gdfl", "--workers", "65"],
        &["run", "srt", "gdfl", "--workers", "x"],
        &["run", "srt", "gdfl", "--underlying", "NIFTY"],
        &["run", "srt", "gdfl", "--fast"],
        &["plan", "gdfl", "--workers", "2"],
        &["plan", "gdfl", "--all"],
        &["run", "recount", "gdfl"],
        &["run", "recount", "gdfl", "--from", "2019-11-14"],
        &[
            "run",
            "recount",
            "gdfl",
            "--from",
            "2019-11-14",
            "--to",
            "2019-11-01",
        ],
        &[
            "run",
            "recount",
            "gdfl",
            "--from",
            "2019-11-31",
            "--to",
            "2019-12-01",
        ],
        &[
            "run",
            "recount",
            "gdfl",
            "--rung",
            "1s",
            "--from",
            "2019-11-01",
            "--to",
            "2019-11-02",
        ],
        &[
            "run",
            "recount",
            "gdfl",
            "NSE",
            "--from",
            "2019-11-01",
            "--to",
            "2019-11-02",
        ],
        &[
            "run",
            "recount",
            "gdfl",
            "--all",
            "--from",
            "2019-11-01",
            "--to",
            "2019-11-02",
        ],
        &["run", "rekey", "zerodha"],
        &["run", "rekey", "gdfl", "--from", "2019-11-01"],
        &["run", "rekey", "gdfl", "--underlying", "nifty"],
        &["run", "rekey", "gdfl", "--underlying", "NIFTY,"],
        &["resume", "srt", "gdfl"],
        &["resume", "srt", "gdfl", "x"],
        &["stop", "srt", "gdfl", "1", "2"],
        &["stop", "srt", "gdfl", "x"],
        &["status", "srt"],
        &["status", "srt", "gdfl", "extra"],
    ] {
        let (code, out) = plain(&root.0, arguments);
        assert_eq!(code, MISUSED, "{arguments:?}: {out}");
        assert!(
            out.starts_with("refused: ") && out.contains(USAGE),
            "{arguments:?}: {out}"
        );
    }
    assert!(!root.0.join("audit").exists() && !root.0.join("imports").exists());
    let store = || Err("no store here".to_owned());
    let host = Host {
        store: &store,
        commit: None,
        now: &|| 1,
        stop: &AtomicBool::new(false),
        after_item: &noop,
    };
    let mut out = String::new();
    assert_eq!(command_with(&["plan", "gdfl"], &host, &mut out), FAILED);
    assert!(
        out.contains("UPKEEP REFUSED: no store here. Nothing was read or written."),
        "{out}"
    );
}

/// A run whose items cannot be found, or whose journal refuses, is refused
/// by name; an unstamped build's run says `unstamped`.
#[test]
fn a_run_that_cannot_begin_is_refused_by_name() {
    let root = Root::new();
    let (code, out) = plain(&root.0, &["run", "srt", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("UPKEEP RUN — srt for gdfl REFUSED") && out.contains("holds no gdfl bars"),
        "{out}"
    );
    spot_months(&root.0, 1);
    fs::create_dir_all(
        root.0
            .join(upkeep::journal_name(Vendor::Gdfl, Kind::SortIndex)),
    )
    .unwrap();
    let (code, out) = plain(&root.0, &["run", "srt", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("upkeep.gdfl.srt.log"), "{out}");
    let (code, out) = plain(&root.0, &["resume", "tix", "gdfl", "77"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(
        out.contains("UPKEEP RESUME — tix for gdfl REFUSED: no run 77 in this journal"),
        "{out}"
    );
    let owned = root.0.clone();
    let store = move || Ok(owned.clone());
    let host = Host {
        store: &store,
        commit: None,
        now: &|| 5,
        stop: &AtomicBool::new(false),
        after_item: &noop,
    };
    let mut out = String::new();
    assert_eq!(
        command_with(&["run", "msum", "gdfl"], &host, &mut out),
        OK,
        "{out}"
    );
    assert!(
        root.journal(Kind::MonthSummary)
            .contains(" commit=unstamped ")
    );
    // Many months not clean: past the shown ones they are counted.
    let many = Root::new();
    for m in 0..(super::SHOWN + 3) {
        let at = ym(
            2000 + u16::try_from(m / 12).unwrap(),
            u8::try_from(m % 12 + 1).unwrap(),
        );
        let path = store_path("INDEX", "NIFTY", None, at, FileKind::Bars);
        month(&many.0, path, "NIFTY", 2, true);
        fs::create_dir_all(
            path.with_file(FileKind::SortIndex)
                .to_path_buf(&many.0)
                .with_extension("new.srt"),
        )
        .unwrap();
    }
    let (code, out) = plain(&many.0, &["run", "srt", "gdfl", "--all", "--workers", "4"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("... and 3 more not clean"), "{out}");
}

/// The outcome helper takes the word after the label and keeps the line;
/// the rate is said to a tenth with no float.
#[test]
fn an_outcome_is_the_receipt_line_and_a_rate_is_a_tenth() {
    let built = outcome(
        true,
        "2019-03",
        "  2019-03  built         1200 bars  because absent\n",
    );
    assert_eq!(built.word, "built");
    assert_eq!(
        built.why,
        "2019-03  built         1200 bars  because absent"
    );
    assert!(built.clean);
    let not = outcome(false, "2019-03", "  2019-03  NOT INDEXED   1200 bars  x");
    assert_eq!(not.word, "NOT_INDEXED");
    let bare = outcome(false, "other", "word  rest");
    assert_eq!(bare.word, "word");
    assert_eq!(rate(7, 0), "7 item(s) in under 1 ms");
    assert_eq!(rate(7, 2_000), "3.5 item(s) a second (7 in 2000 ms)");
    assert_eq!(rate(1, 3), "333.3 item(s) a second (1 in 3 ms)");
    assert_eq!(State::Ended(How::Done), State::Ended(How::Done));
}

// ═══ every arm the receipt and the builders have ════════════════════════════

/// The binary hands `upkeep` and its arguments to this verb.
#[test]
fn the_binary_dispatches_upkeep_with_its_arguments() {
    let mut out = String::new();
    let code = crate::dispatch(&["upkeep".to_owned(), "walk".to_owned()], &mut out);
    assert_eq!(code, MISUSED, "{out}");
    assert!(
        out.contains("upkeep takes plan, run, resume, stop or status"),
        "{out}"
    );
}

/// Locks that would not be released are each named; none refused is none.
#[test]
fn every_lock_that_would_not_be_released_is_named() {
    assert_eq!(all_released([Ok(()), Ok(())]), Ok(()));
    assert_eq!(
        all_released([Ok(()), Err("a".to_owned()), Ok(()), Err("b".to_owned())]),
        Err("a; b".to_owned())
    );
}

/// A torn journal tail is said in the receipt; a run with nothing to build
/// says `none` and is whole; a plan of a store with no options directory says
/// why it cannot count underlyings.
#[test]
fn a_torn_tail_an_empty_run_and_a_store_with_no_options_are_said() {
    let root = Root::new();
    spot_months(&root.0, 1);
    let journal = root
        .0
        .join(upkeep::journal_name(Vendor::Gdfl, Kind::SortIndex));
    fs::create_dir_all(journal.parent().unwrap()).unwrap();
    fs::write(&journal, "item 5 n=1 ok=1 of=2 t=0 cle").unwrap();
    let (code, out) = plain(&root.0, &["run", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("torn tail           28 byte(s) a killed run left were ended as a void line"),
        "{out}"
    );
    let (code, out) = plain(&root.0, &["run", "srt", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("0 item(s) given"), "{out}");
    assert!(out.contains("outcomes            none"), "{out}");
    assert!(!out.contains("resume with"), "{out}");
    let (code, out) = plain(&root.0, &["plan", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("rekey: cannot list"), "{out}");
}

/// A recount that cannot read a file in its span is not clean.
#[test]
fn a_recount_that_cannot_read_a_file_is_not_clean() {
    let root = Root::new();
    let path = store_path(
        "FNO",
        "NIFTY",
        Some("2019-12-26-1200000-CE"),
        ym(2019, 11),
        FileKind::Bars,
    );
    fs::create_dir_all(path.to_path_buf(&root.0).parent().unwrap()).unwrap();
    fs::write(path.to_path_buf(&root.0), b"not a bar file").unwrap();
    let (code, out) = plain(
        &root.0,
        &[
            "run",
            "recount",
            "gdfl",
            "--from",
            "2019-11-01",
            "--to",
            "2019-11-30",
        ],
    );
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("NOT_CLEAN 1"), "{out}");
    assert!(root.journal(Kind::Recount).contains("1 unreadable"));
}

const STATED: &str = "2023-06-29-1000000-PE";
const SETTLED: &str = "2023-06-28-1000000-PE";

/// Writes `rows` as epoch 1 of the GDFL FNO census (version 4).
fn census_of(root: &Path, rows: Vec<Held>) {
    let mut census = Manifest::partition_genesis(Vendor::Gdfl, Segment::Fno, 64).unwrap();
    for row in rows {
        census.record_held(row).unwrap();
    }
    let path = epoch_path(root, Vendor::Gdfl, Segment::Fno, 1);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, census.image()).unwrap();
}

/// The re-key's every outcome as an upkeep item: a key filed under its
/// stated expiry MOVED (a new month file written), the same move REFUSED
/// where its target holds other bars, a key with days refused and days
/// admitted NOT APPLIED, a ledger that cannot be read REFUSED, and a census
/// that is not partitioned STOPPED — none of the last four writes a byte.
#[test]
fn every_rekey_outcome_is_an_item() {
    let moved_day = Day::new(2020, 6, 15).unwrap();
    // MOVED.
    let root = Root::new();
    census_of(&root.0, vec![option(&root.0, STATED, moved_day, 34_000)]);
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("outcomes            applied 1"), "{out}");
    assert!(
        root.journal(Kind::Rekey)
            .contains("1 moved (1 month file(s) written, 0 already held exactly)"),
        "{}",
        root.journal(Kind::Rekey)
    );
    assert!(
        store_path("FNO", "NIFTY", Some(SETTLED), ym(2020, 6), FileKind::Bars)
            .to_path_buf(&root.0)
            .exists()
    );
    // REFUSED: the target month already holds other bars.
    let root = Root::new();
    let stated = option(&root.0, STATED, moved_day, 34_000);
    let other = option(&root.0, SETTLED, Day::new(2020, 6, 16).unwrap(), 34_500);
    census_of(&root.0, vec![stated, other]);
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("REFUSED 1"), "{out}");
    assert!(
        root.journal(Kind::Rekey).contains("move(s) refused: "),
        "{}",
        root.journal(Kind::Rekey)
    );
    // NOT APPLIED: refused on a day with a ladder, admitted on a day alone.
    let root = Root::new();
    let on = Day::new(2019, 5, 30).unwrap();
    let mut rows = Vec::new();
    for (at, contract) in [
        "2019-05-30-1150000-CE",
        "2019-05-30-1170000-PE",
        "2019-05-30-1180000-CE",
    ]
    .into_iter()
    .enumerate()
    {
        rows.push(option(
            &root.0,
            contract,
            on,
            37_845 + u32::try_from(at).unwrap(),
        ));
    }
    rows.push(option(&root.0, IMPOSSIBLE[0], on, 37_900));
    rows.push(option(
        &root.0,
        IMPOSSIBLE[0],
        Day::new(2019, 6, 3).unwrap(),
        37_900,
    ));
    census_of(&root.0, rows);
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("NOT_APPLIED 1"), "{out}");
    assert!(
        out.contains("0 file(s) unreadable and 1 key(s) unresolved"),
        "{out}"
    );
    assert!(
        !pull::gdfl_lineage::lineage_path(&root.0).exists(),
        "nothing written"
    );
    // REFUSED: a lineage ledger that cannot be read.
    let root = Root::new();
    census_of(&root.0, vec![option(&root.0, STATED, moved_day, 34_000)]);
    fs::create_dir_all(pull::gdfl_lineage::lineage_path(&root.0)).unwrap();
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("REFUSED 1"), "{out}");
    assert!(root.journal(Kind::Rekey).contains("nothing was written"));
    // STOPPED: a census that is not partitioned.
    let root = Root::new();
    let _ = option(&root.0, STATED, moved_day, 34_000);
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, FAILED, "{out}");
    assert!(out.contains("STOPPED 1"), "{out}");
}

/// A re-key that finds its ledger lines written and its months held but the
/// retired keys listed again (a later write put them back in the census)
/// writes no line and no month, installs the next epoch again, and says
/// `applied`.
#[test]
fn a_rekey_that_only_withdraws_again_says_applied() {
    let root = Root::new();
    let day = Day::new(2019, 5, 30).unwrap();
    let mut rows = Vec::new();
    for (at, contract) in [
        "2019-05-30-1150000-CE",
        "2019-05-30-1170000-PE",
        "2019-05-30-1180000-CE",
    ]
    .into_iter()
    .enumerate()
    {
        rows.push(option(
            &root.0,
            contract,
            day,
            37_845 + u32::try_from(at).unwrap(),
        ));
    }
    let impossible = option(&root.0, IMPOSSIBLE[0], day, 37_900);
    rows.push(impossible);
    census_of(&root.0, rows);
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    pull::ingest::census_store::record(&root.0, Vendor::Gdfl, &[impossible]).unwrap();
    let (code, out) = plain(&root.0, &["run", "rekey", "gdfl"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("outcomes            applied 1"), "{out}");
    assert!(
        root.journal(Kind::Rekey).contains(
            "0 moved (0 month file(s) written, 0 already held exactly), 0 ledger line(s) appended"
        ),
        "{}",
        root.journal(Kind::Rekey)
    );
}

/// A Zerodha plan names no re-key: the re-key is GDFL's alone.
#[test]
fn a_zerodha_plan_names_no_rekey() {
    let root = Root::new();
    let at = StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: ym(2024, 1),
        file: FileKind::Bars,
    })
    .unwrap();
    let open = ym(2024, 1).ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
    let mut writer =
        BarFile::open_or_create(&root.0, at, crate::srt_builder::symbol_id("NIFTY")).unwrap();
    writer
        .append(&[Bar {
            ts_micros: open,
            open: 1,
            high: 1,
            low: 1,
            close: 1,
            volume: 1,
            open_interest: OI_NULL,
        }])
        .unwrap();
    drop(writer);
    let (code, out) = plain(&root.0, &["plan", "zerodha"]);
    assert_eq!(code, OK, "{out}");
    assert!(out.contains("NSE/INDEX/NIFTY 1min"), "{out}");
    assert!(!out.contains("rekey:"), "{out}");
    assert!(
        out.contains("cli upkeep run recount zerodha --from DAY --to DAY"),
        "{out}"
    );
}
