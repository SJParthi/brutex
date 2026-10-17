//! Tests of `pull::gdfl_run` (IA-1). No vendor byte: every source here is an
//! empty tick store, and every journal line is written by hand in the
//! journal's own grammar.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::gdfl_import::{BAR_DEFINITION, Cause, FAILURES_KEPT, RECENT_KEPT, Stage};
use crate::ingest::Failure;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("brutex-gdfl-run-{tag}-{}", std::process::id()));
    let _fresh = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn request(kinds: &[ImportKind]) -> Request {
    Request::new(
        Kinds::of(kinds).unwrap(),
        SourceWord::TickStore,
        (day(2024, 4, 1), day(2024, 4, 1)),
    )
}

fn env_with(pairs: Vec<(&'static str, PathBuf)>) -> impl Fn(&str) -> Option<OsString> {
    move |var| {
        pairs
            .iter()
            .find(|(k, _)| *k == var)
            .map(|(_, v)| v.clone().into_os_string())
    }
}

fn installed() -> LogState {
    LogState::Installed(PathBuf::from("/nowhere/logs/gdfl/indices"))
}

fn process<'a>(store: &'a Path, log: &'a LogState) -> Process<'a> {
    Process {
        store_root: store,
        commit: None,
        log,
        unstamped: None,
    }
}

// ═══ The request line (D-4902) ══════════════════════════════════════════════

/// **A 256-NAME FILTER AND A SNAPSHOT CROSS THE LINE INTACT** (IA-GR-35's
/// addition, D-4902): the line is the api child's only source of its
/// request, so every byte a name can hold (an `&`, a comma, a space, a `%`,
/// a non-ASCII letter) comes back exactly, and the line never holds a space
/// inside a value or a newline.
#[test]
fn the_request_line_carries_a_256_name_filter_and_a_snapshot_intact() {
    let mut asked = request(&[ImportKind::Stocks, ImportKind::Indices]);
    asked.only = (0..ONLY_MAX - 6).map(|i| format!("SYM{i:03}")).collect();
    asked
        .only
        .extend(["M&M", "BAJAJ-AUTO", "A B", "x,y", "100%", "café"].map(str::to_owned));
    assert_eq!(asked.only.len(), ONLY_MAX);
    asked.retry_refused = true;
    asked.workers = Workers::new(3).unwrap();
    asked.durability = Durability::parse("each-write").unwrap();
    asked.by = By::Api;
    asked.snapshot = Some("pre_restate-2026.10.17".to_owned());
    let line = asked.to_line().unwrap();
    assert!(line.len() <= REQUEST_LINE_MAX, "{}", line.len());
    assert!(!line.contains('\n'));
    assert!(line.starts_with("gdfl-request v1 mode=import kinds=indices,stocks "));
    assert!(line.contains(" only=SYM000,SYM001,"), "{line}");
    assert!(line.contains(",M%26M,BAJAJ-AUTO,A%20B,x%2Cy,100%25,caf%C3%A9 "));
    assert_eq!(
        line.split(' ').count(),
        14,
        "two head words and twelve fields"
    );
    let back = Request::from_line(&line).unwrap();
    assert_eq!(back, asked);
    // The order the kinds were named in is not the order they run in.
    assert_eq!(
        back.kinds.iter().collect::<Vec<_>>(),
        [ImportKind::Indices, ImportKind::Stocks]
    );
    // A trailing newline (stdin's) is the same line.
    assert_eq!(Request::from_line(&format!("{line}\n")).unwrap(), asked);
}

/// Every way a line can be wrong is refused by name before any lock, as
/// `request_line_invalid`, or by the misuse code the request itself earns.
#[test]
fn a_request_line_that_is_not_one_is_refused_before_any_lock() {
    let good = request(&[ImportKind::Indices]).to_line().unwrap();
    let swap = |from: &str, to: &str| {
        assert!(good.contains(from), "{from}");
        good.replacen(from, to, 1)
    };
    let invalid = "request_line_invalid";
    let cases: Vec<(String, &str)> = vec![
        ("x".repeat(REQUEST_LINE_MAX + 1), invalid),
        (String::new(), invalid),
        (swap("gdfl-request v1", "gdfl-request v2"), invalid),
        (swap("gdfl-request v1 ", "gdfl-request v1  "), invalid),
        (swap(" mode=import", ""), invalid),
        (swap("mode=import", "mode=verify"), invalid),
        (swap("kinds=indices", "kinds=indices,indices"), invalid),
        (swap("kinds=indices", "kinds="), invalid),
        (swap("kinds=indices", "kinds=futures"), invalid),
        (swap("source=tickstore", "source=dayzips"), invalid),
        (swap("from=2024-04-01", "from=2024-4-01"), invalid),
        (swap("to=2024-04-01", "to=2024-02-30"), invalid),
        (swap("only=", "only=%2c"), invalid),
        (swap("only=", "only=%2"), invalid),
        (swap("only=", "only=a%ZZ"), invalid),
        (swap("only=", "only=%FF"), invalid),
        (swap("only=", "only=a&b"), invalid),
        (swap("stored=0", "stored=2"), invalid),
        (swap("retry_refused=0", "retry_refused=yes"), invalid),
        (
            swap(
                &format!("workers={}", Workers::for_host().get()),
                "workers=0",
            ),
            invalid,
        ),
        (
            swap(
                &format!("workers={}", Workers::for_host().get()),
                "workers=99",
            ),
            invalid,
        ),
        (swap("durability=day", "durability=never"), invalid),
        (swap("by=cli", "by=cron"), invalid),
        (format!("{good} extra=1"), invalid),
        (
            swap("stored=0 retry_refused=0", "retry_refused=0 stored=0"),
            invalid,
        ),
        // The request is well formed and asks what no run may do.
        (
            swap("from=2024-04-01", "from=2024-04-02"),
            "range_backwards",
        ),
        (swap("only=", "only=NIFTY,"), "symbol_empty"),
        (swap("snapshot=", "snapshot=a%20b"), "snapshot_invalid"),
        (
            swap("snapshot=", &format!("snapshot={}", "s".repeat(65))),
            "snapshot_invalid",
        ),
        (
            swap("kinds=indices", "kinds=options").replace("stored=0", "stored=1"),
            "stored_with_options",
        ),
        (
            swap(
                "only=",
                &format!(
                    "only={}",
                    (0..=ONLY_MAX)
                        .map(|i| format!("S{i}"))
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            ),
            "filter_too_long",
        ),
    ];
    for (line, code) in cases {
        let refused = Request::from_line(&line).expect_err(&line);
        assert_eq!(refused.code(), code, "{line:?}: {refused}");
        assert!(refused.to_string().starts_with(&format!("{code}: ")));
        assert!(refused.misuse(), "{refused}");
    }
    // A line that cannot fit is refused writing as well as reading.
    let mut long = request(&[ImportKind::Stocks]);
    long.only = (0..ONLY_MAX).map(|_| "&".repeat(12)).collect();
    assert_eq!(long.to_line().unwrap_err().code(), invalid);
}

// ═══ Refusals before any lock ═══════════════════════════════════════════════

/// **ONE AUTHORITY, ONE CODE** (IA-GR-13): `--stored` with options is
/// refused by the request, by the plan and by the request line alike, as
/// `stored_with_options`, a misuse; the store is not touched.
#[test]
fn stored_with_options_is_refused_by_one_authority() {
    let store = scratch("stored-options");
    let mut asked = request(&[ImportKind::Options]);
    asked.stored = true;
    assert_eq!(asked.misuse(), Some(RunRefusal::StoredWithOptions));
    let log = installed();
    let refused = plan(
        asked.clone(),
        &process(&store, &log),
        &env_with(vec![(TICKSTORE_VAR, store.clone())]),
        &mut SourceCache::default(),
    )
    .unwrap_err();
    assert_eq!(refused.code(), "stored_with_options");
    assert!(refused.misuse());
    // The line an api child would be handed says the same.
    let line = request(&[ImportKind::Options])
        .to_line()
        .unwrap()
        .replace("stored=0", "stored=1");
    assert_eq!(
        Request::from_line(&line).unwrap_err(),
        RunRefusal::StoredWithOptions
    );
    // A capital-market run may say it.
    let mut stocks = request(&[ImportKind::Stocks]);
    stocks.stored = true;
    assert_eq!(stocks.misuse(), None);
    assert_eq!(
        std::fs::read_dir(&store).unwrap().count(),
        0,
        "nothing written"
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// **NO RUN WORKS UNLOGGED** (IA-GR-44, D-4903): a process whose event sink
/// was refused is refused `log_unavailable` by the plan, in the sink's own
/// words, before any lock, root or scope is read; the store is untouched.
#[test]
fn a_run_whose_sink_was_refused_refuses_before_any_lock() {
    let store = scratch("unlogged");
    let log = LogState::Refused("events.lock is held by pid 4242".to_owned());
    let refused = plan(
        request(&[ImportKind::Indices]),
        &process(&store, &log),
        // No source variable at all: the log is asked first.
        &env_with(Vec::new()),
        &mut SourceCache::default(),
    )
    .unwrap_err();
    assert_eq!(refused.code(), "log_unavailable");
    assert!(!refused.misuse());
    let words = refused.to_string();
    assert!(
        words.contains("events.lock is held by pid 4242") && words.contains("D-4903"),
        "{words}"
    );
    assert_eq!(
        std::fs::read_dir(&store).unwrap().count(),
        0,
        "nothing written"
    );
    // `of_process` says so for a process with no sink installed, which this
    // test binary has unless another test installed one.
    if telemetry::global().is_none() {
        assert_eq!(
            LogState::of_process("no sink"),
            LogState::Refused("no sink".to_owned())
        );
    }
    std::fs::remove_dir_all(store).unwrap();
}

/// The plan asks in one order: the request's misuse, the log, the build's
/// stamp, then each kind's root; each refusal names the first thing wrong.
#[test]
fn the_plan_refuses_in_one_order_before_any_lock() {
    let store = scratch("order");
    let refused_log = LogState::Refused("held".to_owned());
    let ok_log = installed();
    let unstamped = Process {
        unstamped: Some("unstamped build (DB-B71)"),
        ..process(&store, &ok_log)
    };
    let backwards = Request {
        from: day(2024, 4, 2),
        ..request(&[ImportKind::Indices])
    };
    let none = env_with(Vec::new());
    let cases: [(Request, Process<'_>, &str); 4] = [
        (
            backwards,
            Process {
                unstamped: Some("x"),
                ..process(&store, &refused_log)
            },
            "range_backwards",
        ),
        (
            request(&[ImportKind::Indices]),
            Process {
                unstamped: Some("x"),
                ..process(&store, &refused_log)
            },
            "log_unavailable",
        ),
        (
            request(&[ImportKind::Indices]),
            unstamped,
            "unstamped_write",
        ),
        (
            request(&[ImportKind::Indices]),
            process(&store, &ok_log),
            "source_unset",
        ),
    ];
    for (asked, proc_, code) in cases {
        let refused = plan(asked, &proc_, &none, &mut SourceCache::default()).unwrap_err();
        assert_eq!(refused.code(), code, "{refused}");
    }
    // The unset root is named, never defaulted, and so is an empty one.
    let empty = env_with(vec![(TICKSTORE_VAR, PathBuf::new())]);
    let refused = plan(
        request(&[ImportKind::Indices]),
        &process(&store, &ok_log),
        &empty,
        &mut SourceCache::default(),
    )
    .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "source_unset: BRUTEX_GDFL_TICKSTORE is not set; it names where the GDFL data is \
         read from and has no default"
    );
    assert_eq!(
        std::fs::read_dir(&store).unwrap().count(),
        0,
        "nothing written"
    );
    std::fs::remove_dir_all(store).unwrap();
}

/// No two refusals share a code, and `CODES` is every code.
#[test]
fn every_run_refusal_has_its_own_code() {
    let all = [
        RunRefusal::RequestLineInvalid { why: String::new() },
        RunRefusal::StoredWithOptions,
        RunRefusal::RangeBackwards {
            from: day(2024, 4, 2),
            to: day(2024, 4, 1),
        },
        RunRefusal::EmptySymbol,
        RunRefusal::FilterTooLong { names: 300 },
        RunRefusal::SnapshotInvalid {
            name: String::new(),
        },
        RunRefusal::LogUnavailable { why: String::new() },
        RunRefusal::Unstamped { why: String::new() },
        RunRefusal::SourceUnset { var: TICKSTORE_VAR },
        RunRefusal::Scope { why: String::new() },
        RunRefusal::SourceUnopened {
            path: PathBuf::new(),
            why: String::new(),
        },
    ];
    let codes: Vec<&str> = all.iter().map(RunRefusal::code).collect();
    assert_eq!(codes, RunRefusal::CODES);
    let unique: std::collections::BTreeSet<&str> = codes.iter().copied().collect();
    assert_eq!(unique.len(), codes.len());
    assert_eq!(
        all.iter().filter(|r| r.misuse()).count(),
        6,
        "the request's six, never the process's"
    );
}

/// `zip` resolves per kind; `tickstore` is every kind's (D-4801).
#[test]
fn a_source_word_resolves_per_kind() {
    assert_eq!(
        resolve_source(SourceWord::Zip, ImportKind::Indices),
        SourceId::CmZip
    );
    assert_eq!(
        resolve_source(SourceWord::Zip, ImportKind::Stocks),
        SourceId::CmZip
    );
    assert_eq!(
        resolve_source(SourceWord::Zip, ImportKind::Options),
        SourceId::NfoZips
    );
    for kind in ImportKind::ALL {
        assert_eq!(
            resolve_source(SourceWord::TickStore, kind),
            SourceId::TickStore
        );
    }
    assert_eq!(SourceId::CmZip.var(), CM_ARCHIVE_VAR);
    assert_eq!(SourceId::NfoZips.var(), NFO_ARCHIVES_VAR);
    assert_eq!(SourceId::TickStore.var(), TICKSTORE_VAR);
    assert_eq!(SourceWord::parse("ftp"), None);
    assert_eq!(SourceWord::parse("zip"), Some(SourceWord::Zip));
}

// ═══ Execution ══════════════════════════════════════════════════════════════

/// What an observer was told, and on which thread.
#[derive(Debug, Default)]
struct Told {
    said: Mutex<Vec<(std::thread::ThreadId, String)>>,
}

impl Told {
    fn note(&self, what: String) {
        self.said
            .lock()
            .unwrap()
            .push((std::thread::current().id(), what));
    }
    fn said(&self) -> Vec<(std::thread::ThreadId, String)> {
        self.said.lock().unwrap().clone()
    }
}

impl Observer for Told {
    fn begun(&self, kind: ImportKind, run: u64) {
        self.note(format!("begun {} {}", kind.as_str(), run > 0));
    }
    fn day_begun(&self, kind: ImportKind, day: Day, (index, total): (usize, usize)) {
        self.note(format!("day {} {day} {index}/{total}", kind.as_str()));
    }
    fn day_passed(&self, kind: ImportKind, day: Day, report: &Report) {
        self.note(format!(
            "passed {} {day} missing={}",
            kind.as_str(),
            report.days_missing.len()
        ));
    }
    fn finished(&self, kind: ImportKind, end: Result<&Report, &ImportRefusal>) {
        self.note(format!("finished {} {}", kind.as_str(), end.is_ok()));
    }
}

/// **THE KINDS RUN IN ORDER, ON THE CALLING THREAD** (round 4, BL-A): a
/// press of two kinds is two runs one after the other, never a thread per
/// kind, and the observer hears every day of each, in order.
#[test]
fn the_kinds_run_in_order_on_the_calling_thread() {
    let root = scratch("kinds");
    let (store, ts) = (root.join("store"), root.join("ts"));
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(&ts).unwrap();
    let mut asked = request(&[ImportKind::Stocks, ImportKind::Indices]);
    asked.to = day(2024, 4, 2);
    let log = installed();
    let planned = plan(
        asked,
        &process(&store, &log),
        &env_with(vec![(TICKSTORE_VAR, ts.clone())]),
        &mut SourceCache::default(),
    )
    .unwrap();
    assert_eq!(planned.kinds.len(), 2);
    assert!(
        planned.kinds[1].scope_words.is_some(),
        "a stocks run says its scope"
    );
    let told = Told::default();
    let ends = execute(&planned, Some(&told));
    assert_eq!(
        ends.iter().map(|(kind, _)| *kind).collect::<Vec<_>>(),
        [ImportKind::Indices, ImportKind::Stocks]
    );
    for (_, end) in &ends {
        let KindEnd::Report(report) = end else {
            panic!("{end:?}");
        };
        assert_eq!(report.days_missing.len(), 2, "{report:?}");
    }
    let said = told.said();
    let me = std::thread::current().id();
    assert!(said.iter().all(|(thread, _)| *thread == me), "{said:?}");
    let words: Vec<&str> = said.iter().map(|(_, w)| w.as_str()).collect();
    assert_eq!(
        words,
        [
            "begun indices true",
            "day indices 2024-04-01 0/2",
            "passed indices 2024-04-01 missing=1",
            "day indices 2024-04-02 1/2",
            "passed indices 2024-04-02 missing=2",
            "finished indices true",
            "begun stocks true",
            "day stocks 2024-04-01 0/2",
            "passed stocks 2024-04-01 missing=1",
            "day stocks 2024-04-02 1/2",
            "passed stocks 2024-04-02 missing=2",
            "finished stocks true",
        ]
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A capital-market zip that cannot be opened ends its kind `Unopened`,
/// named with its path, and the kinds after it are not begun.
#[test]
fn an_unopened_source_stops_the_run_before_the_next_kind() {
    let root = scratch("unopened");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let zip = root.join("absent.zip");
    let mut asked = request(&[ImportKind::Indices, ImportKind::Options]);
    asked.source = SourceWord::Zip;
    let log = installed();
    let planned = plan(
        asked,
        &process(&store, &log),
        &env_with(vec![
            (CM_ARCHIVE_VAR, zip.clone()),
            (NFO_ARCHIVES_VAR, root.clone()),
        ]),
        &mut SourceCache::default(),
    )
    .unwrap();
    assert_eq!(planned.kinds[0].source, SourceId::CmZip);
    assert_eq!(planned.kinds[1].source, SourceId::NfoZips);
    let ends = execute(&planned, None);
    assert_eq!(ends.len(), 1, "{ends:?}");
    let KindEnd::Unopened(refused) = &ends[0].1 else {
        panic!("{ends:?}");
    };
    assert_eq!(refused.code(), "source_unopened");
    assert!(refused.to_string().contains("absent.zip"), "{refused}");
    std::fs::remove_dir_all(root).unwrap();
}

/// **ONE MEASURE PER SOURCE WHILE ITS ROOT IS UNCHANGED** (design §5.1): a
/// second plan of the same root reuses the measure with a start of its own;
/// a root whose modification time or length changed is measured again.
#[test]
fn the_source_cache_measures_again_only_when_the_root_changed() {
    let root = scratch("cache");
    let ts = root.join("ts");
    std::fs::create_dir_all(&ts).unwrap();
    std::fs::write(ts.join("FORMAT.md"), b"synthetic\n").unwrap();
    let mut cache = SourceCache::default();
    let first = cache.origin(SourceId::TickStore, &ts, Some("abc"));
    let again = cache.origin(SourceId::TickStore, &ts, Some("abc"));
    assert_eq!(cache.measures(), 1);
    assert_eq!(first.digest, again.digest);
    assert!(again.started >= first.started);
    // Another build is another origin.
    let _other = cache.origin(SourceId::TickStore, &ts, None);
    assert_eq!(cache.measures(), 2);
    // A changed root: a new entry changes its modification time.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(ts.join("2024"), b"").unwrap();
    let _changed = cache.origin(SourceId::TickStore, &ts, None);
    assert_eq!(cache.measures(), 3);
    // A root that is not there is measured every time, its fault named.
    let gone = root.join("gone");
    let a = cache.origin(SourceId::TickStore, &gone, None);
    let _b = cache.origin(SourceId::TickStore, &gone, None);
    assert_eq!(cache.measures(), 5);
    assert!(a.fault.is_some(), "{a:?}");
    std::fs::remove_dir_all(root).unwrap();
}

/// The run's log directory follows the census layout (D-4862): v3 (no
/// version-4 census) logs where the store lock's holder logs, v4 in the
/// kind's own directory, and an unreadable census is refused, never guessed.
#[test]
fn the_run_directory_follows_the_census_layout() {
    let root = scratch("layout");
    assert_eq!(
        run_dir(&root, ImportKind::Options).unwrap().sub,
        "gdfl/store"
    );
    std::fs::create_dir_all(root.join("manifest").join("v4").join("gdfl")).unwrap();
    for kind in ImportKind::ALL {
        assert_eq!(
            run_dir(&root, kind).unwrap().sub,
            format!("gdfl/{}", kind.as_str())
        );
        assert_eq!(
            progress_path(&root, kind, telemetry::run_dirs::GdflLayout::V4),
            root.join("imports")
                .join(format!("gdfl.{}.progress", kind.as_str()))
        );
    }
    assert_eq!(
        progress_path(
            &root,
            ImportKind::Stocks,
            telemetry::run_dirs::GdflLayout::V3
        ),
        root.join("imports").join("gdfl.progress")
    );
    let file = scratch("layout-file");
    std::fs::create_dir_all(file.join("manifest").join("v4")).unwrap();
    std::fs::write(file.join("manifest").join("v4").join("gdfl"), b"").unwrap();
    assert!(run_dir(&file, ImportKind::Indices).is_err());
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(file).unwrap();
}

// ═══ The progress record (D-4904) ═══════════════════════════════════════════

fn sample() -> Progress {
    Progress {
        state: RunState::Running.code(),
        mode: 1,
        kind: 3,
        source: 2,
        by: 2,
        layout: 4,
        run: 1_791_234_567,
        pid: 5_120,
        workers: 8,
        from: ymd(day(2020, 9, 16)),
        to: ymd(day(2026, 9, 24)),
        day_now: ymd(day(2020, 10, 5)),
        day_index: 13,
        day_total: 1_497,
        files: 52_340,
        files_skipped: 120,
        files_refused: 3,
        rows: 341_000_000,
        seconds_written: 40_200_000,
        refusals_total: 3,
        days_imported: 12,
        days_with_refusals: 1,
        days_incomplete: 0,
        days_all_refused: 0,
        days_skipped: 4,
        days_closed: 4,
        days_not_held: 0,
        days_refused: 0,
        elapsed_s: 8_300,
        started: 1_791_234_567,
        updated: 1_791_240_000,
        clock_held: 2,
        window_days: 14,
        window_s: 8_300,
        log_dropped: 3,
        log_failures: 1,
        refusals: vec![ProgressRefusal {
            day: ymd(day(2020, 9, 17)),
            stage: "vendor".to_owned(),
            code: "malformed_row".to_owned(),
            about: "BANKNIFTY17SEP2023000CE".to_owned(),
            why: "line 451: not ten well-formed fields".to_owned(),
        }],
    }
}

/// **THE PROGRESS RECORD IS V1, BYTE FOR BYTE** (IA-GR-45, D-4904): the
/// offsets are the definition, so each is asserted where `docs/02` §37 puts
/// it; a flipped byte is a CRC fault, another version is named and never
/// guessed at, and the record round-trips.
#[test]
fn the_progress_record_is_v1_byte_for_byte() {
    let record = sample();
    let bytes = record.encode();
    assert_eq!(bytes.len(), 4_096);
    assert_eq!(&bytes[0..8], b"BRUTEXPG");
    assert_eq!(&bytes[8..10], &1_u16.to_le_bytes());
    assert_eq!(
        &bytes[10..16],
        &[1, 1, 3, 2, 2, 4],
        "state mode kind source by layout"
    );
    assert_eq!(&bytes[16..24], &1_791_234_567_u64.to_le_bytes());
    assert_eq!(&bytes[24..28], &5_120_u32.to_le_bytes());
    assert_eq!(&bytes[28..32], &8_u32.to_le_bytes());
    assert_eq!(&bytes[32..36], &20_200_916_u32.to_le_bytes());
    assert_eq!(&bytes[36..40], &20_260_924_u32.to_le_bytes());
    assert_eq!(&bytes[40..44], &20_201_005_u32.to_le_bytes());
    assert_eq!(&bytes[44..48], &13_u32.to_le_bytes());
    assert_eq!(&bytes[48..52], &1_497_u32.to_le_bytes());
    assert_eq!(bytes[52], 1, "one slot used");
    assert_eq!(&bytes[53..56], &[0, 0, 0]);
    let counters = [
        52_340_u64,
        120,
        3,
        341_000_000,
        40_200_000,
        3,
        12,
        1,
        0,
        0,
        4,
        4,
        0,
        0,
        8_300,
        1_791_234_567,
        1_791_240_000,
        2,
        14,
        8_300,
        3,
        1,
    ];
    for (i, value) in counters.into_iter().enumerate() {
        let at = 56 + 8 * i;
        assert_eq!(&bytes[at..at + 8], &value.to_le_bytes(), "counter {i}");
    }
    assert_eq!(56 + 8 * counters.len(), 232);
    assert!(bytes[232..256].iter().all(|b| *b == 0), "reserved");
    // The first slot: day, the four lengths, the four texts.
    assert_eq!(&bytes[256..260], &20_200_917_u32.to_le_bytes());
    assert_eq!(&bytes[260..264], &[6, 13, 23, 36]);
    assert_eq!(&bytes[264..270], b"vendor");
    assert_eq!(&bytes[272..285], b"malformed_row");
    assert_eq!(&bytes[304..327], b"BANKNIFTY17SEP2023000CE");
    assert_eq!(&bytes[368..404], b"line 451: not ten well-formed fields");
    assert!(
        bytes[608..4_092].iter().all(|b| *b == 0),
        "unused slots and tail"
    );
    assert_eq!(
        &bytes[4_092..],
        &store::crc::crc32c(&bytes[..4_092]).to_le_bytes()
    );
    assert_eq!(Progress::decode(&bytes).unwrap(), record);
    // Faults, each by name.
    let mut flipped = bytes;
    flipped[100] ^= 1;
    assert_eq!(Progress::decode(&flipped), Err(ProgressFault::Crc));
    let mut v2 = bytes;
    v2[8] = 2;
    let fault = Progress::decode(&v2).unwrap_err();
    assert_eq!(fault, ProgressFault::Version(2));
    assert_eq!(fault.to_string(), "progress: format 2 unknown");
    assert_eq!(
        Progress::decode(&bytes[..4_095]),
        Err(ProgressFault::Length(4_095))
    );
    let mut magic = bytes;
    magic[0] = b'X';
    assert_eq!(Progress::decode(&magic), Err(ProgressFault::Magic));
    let mut slots = bytes;
    slots[52] = 9;
    let crc = store::crc::crc32c(&slots[..4_092]);
    slots[4_092..].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(
        Progress::decode(&slots),
        Err(ProgressFault::Field("the slot count"))
    );
}

/// A slot keeps the newest eight; a text past its bound is cut on a whole
/// character, never mid-letter.
#[test]
fn the_progress_slots_keep_the_newest_eight_cut_on_whole_characters() {
    let mut record = sample();
    record.refusals = (0..11)
        .map(|i| ProgressRefusal {
            day: ymd(day(2024, 4, 1)) + i,
            stage: "file".to_owned(),
            code: format!("code{i}"),
            about: "é".repeat(40),
            why: "w".repeat(SLOT_WHY),
        })
        .collect();
    let back = Progress::decode(&record.encode()).unwrap();
    assert_eq!(back.refusals.len(), PROGRESS_SLOTS);
    assert_eq!(back.refusals[0].code, "code3");
    assert_eq!(back.refusals[7].code, "code10");
    assert_eq!(
        back.refusals[0].about,
        "é".repeat(32),
        "64 bytes, 32 letters"
    );
    assert_eq!(back.refusals[0].why.len(), SLOT_WHY);
    assert_eq!(cut("aé", 2), "a");
}

/// A clock that steps backwards between writes.
fn stepping_clock() -> u64 {
    static CALLS: AtomicU64 = AtomicU64::new(0);
    match CALLS.fetch_add(1, Ordering::Relaxed) {
        0 => 2_000_000_000,
        1 => 5,
        _ => 0,
    }
}

/// **A CLOCK STEPPED BACK NEVER ABORTS A RUN** (IA-GR-28): the progress
/// file's elapsed time is the monotonic clock's, every difference
/// saturates, and a wall clock that jumps from 2033 to 1970 between writes
/// is only written down, never subtracted.
#[test]
fn a_clock_step_back_never_aborts_a_run() {
    let root = scratch("clock");
    let (store, ts) = (root.join("store"), root.join("ts"));
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(&ts).unwrap();
    let mut asked = request(&[ImportKind::Indices]);
    asked.to = day(2024, 4, 5);
    let log = installed();
    let planned = plan(
        asked,
        &process(&store, &log),
        &env_with(vec![(TICKSTORE_VAR, ts)]),
        &mut SourceCache::default(),
    )
    .unwrap();
    let path = progress_path(
        &store,
        ImportKind::Indices,
        telemetry::run_dirs::GdflLayout::V4,
    );
    let progress =
        ProgressFile::with_clock(path.clone(), &planned, &planned.kinds[0], 4, stepping_clock);
    let ends = execute(&planned, Some(&progress));
    assert!(matches!(ends[0].1, KindEnd::Report(_)), "{ends:?}");
    assert_eq!(progress.fault(), None);
    let read = Progress::read(&path).unwrap();
    assert_eq!(read.state, RunState::Finished.code());
    assert_eq!(read.started, 2_000_000_000, "the first reading");
    assert_eq!(read.updated, 0, "the last reading, written as given");
    assert!(read.elapsed_s < 60, "monotonic: {}", read.elapsed_s);
    assert_eq!(read.days_not_held, 5);
    assert_eq!(read.day_total, 5);
    assert_eq!(read.day_index, 4);
    assert_eq!(read.from, ymd(day(2024, 4, 1)));
    assert_eq!(read.to, ymd(day(2024, 4, 5)));
    assert!(read.run > 0);
    // A day the source does not hold names no failure of its own.
    assert_eq!((read.refusals_total, read.refusals.len()), (0, 0));
    std::fs::remove_dir_all(root).unwrap();
}

/// **A WRITE AT EVERY CLOSED DAY, OTHERWISE AT MOST ONE A SECOND** (C-GR-02's
/// rule): a thousand days skipped as done in a burst write once, each day
/// that closed writes, and the end always writes.
#[test]
fn the_progress_file_writes_each_closed_day_and_at_most_one_a_second_otherwise() {
    let root = scratch("throttle");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let log = installed();
    let planned = plan(
        request(&[ImportKind::Indices]),
        &process(&store, &log),
        &env_with(vec![(TICKSTORE_VAR, root.clone())]),
        &mut SourceCache::default(),
    )
    .unwrap();
    let progress = ProgressFile::new(
        store.join("imports").join("p"),
        &planned,
        &planned.kinds[0],
        4,
    );
    let mut report = Report::default();
    progress.day_begun(ImportKind::Indices, day(2024, 4, 1), (0, 2_000));
    assert_eq!(progress.writes(), 1, "the first write is due");
    for i in 0..1_000 {
        report.days_skipped += 1;
        progress.day_passed(ImportKind::Indices, day(2024, 4, 1), &report);
        progress.day_begun(ImportKind::Indices, day(2024, 4, 1), (i + 1, 2_000));
    }
    assert_eq!(progress.writes(), 1, "skips wait for the second");
    for closed in 1..=5 {
        report.days_imported = closed;
        progress.day_passed(ImportKind::Indices, day(2024, 4, 1), &report);
    }
    assert_eq!(progress.writes(), 6, "each closed day writes");
    progress.finished(ImportKind::Indices, Ok(&report));
    assert_eq!(progress.writes(), 7);
    let read = Progress::read(progress.path()).unwrap();
    assert_eq!((read.days_skipped, read.days_imported), (1_000, 5));
    std::fs::remove_dir_all(root).unwrap();
}

/// A progress file that cannot be written is named once, and the run is not
/// stopped by it: the journal is the run's record, the progress a view.
#[test]
fn a_progress_file_that_cannot_be_written_is_named_and_the_run_goes_on() {
    let root = scratch("unwritable");
    let (store, ts) = (root.join("store"), root.join("ts"));
    std::fs::create_dir_all(&store).unwrap();
    std::fs::create_dir_all(&ts).unwrap();
    let blocker = root.join("blocker");
    std::fs::write(&blocker, b"a file where the directory belongs").unwrap();
    let log = installed();
    let planned = plan(
        request(&[ImportKind::Indices]),
        &process(&store, &log),
        &env_with(vec![(TICKSTORE_VAR, ts)]),
        &mut SourceCache::default(),
    )
    .unwrap();
    let progress = ProgressFile::new(blocker.join("p"), &planned, &planned.kinds[0], 4);
    let ends = execute(&planned, Some(&progress));
    assert!(matches!(ends[0].1, KindEnd::Report(_)), "{ends:?}");
    let fault = progress.fault().expect("named");
    assert!(fault.contains("blocker"), "{fault}");
    assert_eq!(progress.writes(), 0);
    std::fs::remove_dir_all(root).unwrap();
}

// ═══ The failures a report keeps (D-4839) ═══════════════════════════════════

fn failure(i: usize) -> Failure {
    Failure {
        instrument: format!("f{i}"),
        why: "invented".to_owned(),
        month: None,
        day: None,
    }
}

/// **EVERY FAILURE COUNTED, THE NEWEST 1,024 BY NAME** (D-4839): three
/// thousand failures of two causes are counted whole and by cause, the list
/// never holds more than twice the bound while the run goes, and the sealed
/// report keeps exactly the newest 1,024 and the newest eight with causes.
#[test]
fn a_failure_past_the_kept_bound_is_counted_and_never_listed() {
    let mut report = Report::default();
    let vendor = Cause::open(Stage::Vendor, "malformed_row");
    let file = Cause::open(Stage::File, "fold");
    for i in 0..3_000 {
        report.name(if i % 3 == 0 { file } else { vendor }, failure(i));
        assert!(report.failures.len() <= 2 * FAILURES_KEPT);
    }
    report.seal();
    assert_eq!(report.failures_total, 3_000);
    assert_eq!(report.failures.len(), FAILURES_KEPT);
    assert_eq!(report.failures[0].instrument, "f1976");
    assert_eq!(report.failures[FAILURES_KEPT - 1].instrument, "f2999");
    assert_eq!(report.failures_unlisted(), 3_000 - FAILURES_KEPT);
    assert_eq!(report.failures_by_cause[&("file", "fold")], 1_000);
    assert_eq!(
        report.failures_by_cause[&("vendor", "malformed_row")],
        2_000
    );
    assert_eq!(report.recent.len(), RECENT_KEPT);
    assert_eq!(report.recent[RECENT_KEPT - 1].1.instrument, "f2999");
    let sealed = report.clone();
    report.seal();
    assert_eq!(report, sealed, "idempotent");
    // Under the bound nothing is dropped.
    let mut few = Report::default();
    few.name(vendor, failure(0));
    few.seal();
    assert_eq!((few.failures.len(), few.failures_unlisted()), (1, 0));
}

// ═══ The journal read and refreshed (L-IA-2) ════════════════════════════════

fn done(on: &str) -> String {
    format!("done indices {on} * definition={BAR_DEFINITION} files=1\n")
}

const RUN: &str = "run 1700000000 source=unrecorded archive=unrecorded \
                   archive_bytes=unrecorded commit=unrecorded started=1700000000";

/// **A REFRESH READS ONLY WHAT WAS APPENDED, AND WRITES NOTHING** (design
/// §5.1, `Journal::refresh`): whole new lines are read; a torn tail is kept
/// back until its newline arrives (and the reader never closes it); a file
/// that got shorter, or another file at the name, is read again whole and
/// never folded onto the old; a line it does not write refuses, and the
/// next refresh reads the whole file again.
#[test]
fn a_journal_refresh_reads_only_what_was_appended() {
    let store = scratch("refresh");
    let path = journal_path(&store);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let head = format!("{RUN} definition={BAR_DEFINITION}\n{}", done("2024-04-01"));
    std::fs::write(&path, &head).unwrap();
    let on = |d: u8| day(2024, 4, d);
    let mut reader = JournalReader::open(&store).unwrap();
    assert!(reader.is_done(ImportKind::Indices, on(1), "*"));
    assert!(!reader.is_done(ImportKind::Indices, on(2), "*"));
    assert_eq!(reader.loaded(), head.len() as u64);
    assert_eq!(reader.refresh().unwrap(), Refreshed::Unchanged);
    let append = |text: &str| {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(text.as_bytes()).unwrap();
    };
    append(&done("2024-04-02"));
    assert_eq!(reader.refresh().unwrap(), Refreshed::Read { lines: 1 });
    assert!(reader.is_done(ImportKind::Indices, on(2), "*"));
    // A torn tail: kept back, and the file is left exactly as it was.
    append("done indices 2024-04-0");
    let before = std::fs::read(&path).unwrap();
    assert_eq!(reader.refresh().unwrap(), Refreshed::TornKept);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "a reader writes nothing"
    );
    append(&format!("3 * definition={BAR_DEFINITION} files=1\n"));
    assert_eq!(reader.refresh().unwrap(), Refreshed::Read { lines: 1 });
    assert!(reader.is_done(ImportKind::Indices, on(3), "*"));
    append(&format!("{}done indices 2024-04-0", done("2024-04-04")));
    assert_eq!(
        reader.refresh().unwrap(),
        Refreshed::ReadTornKept { lines: 1 }
    );
    // Shorter: read again whole, the lost days no longer done.
    std::fs::write(&path, &head).unwrap();
    assert!(matches!(
        reader.refresh().unwrap(),
        Refreshed::Reloaded { now, .. } if now == head.len() as u64
    ));
    assert!(!reader.is_done(ImportKind::Indices, on(2), "*"));
    // Another file at the name, longer than what was read: read whole.
    let other = path.with_extension("other");
    std::fs::write(
        &other,
        format!("{head}{}{}", done("2024-04-08"), done("2024-04-09")),
    )
    .unwrap();
    std::fs::rename(&other, &path).unwrap();
    assert!(matches!(
        reader.refresh().unwrap(),
        Refreshed::Reloaded { .. }
    ));
    assert!(reader.is_done(ImportKind::Indices, on(9), "*"));
    // A foreign line refuses, and so does every refresh until it is gone.
    append("not a journal line\n");
    assert!(reader.refresh().is_err());
    assert!(
        reader.refresh().is_err(),
        "poisoned: read whole, refused again"
    );
    std::fs::write(&path, &head).unwrap();
    assert!(matches!(
        reader.refresh().unwrap(),
        Refreshed::Reloaded { .. }
    ));
    assert!(reader.is_done(ImportKind::Indices, on(1), "*"));
    // Gone: an empty journal.
    std::fs::remove_file(&path).unwrap();
    assert!(matches!(
        reader.refresh().unwrap(),
        Refreshed::Reloaded { now: 0, .. }
    ));
    assert!(!reader.is_done(ImportKind::Indices, on(1), "*"));
    std::fs::remove_dir_all(store).unwrap();
}

/// **A REFRESH AT ANY CUT ANSWERS AS A WHOLE LOAD** (design §5.1): for every
/// byte at which a journal of mixed lines could have been cut when the
/// reader opened it, opening the prefix and refreshing after the rest is
/// appended answers every key, and the run and open counts, exactly as one
/// load of the whole file.
#[test]
fn a_refresh_after_any_cut_answers_as_one_whole_load() {
    let store = scratch("cuts");
    let path = journal_path(&store);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let whole = format!(
        "{RUN} definition={BAR_DEFINITION}\nbegin indices 2024-04-01 *\n{}begin indices \
         2024-04-02 *\nincomplete indices 2024-04-02 * definition={BAR_DEFINITION} files=1 \
         seconds=1 failures=1 late=0 late_unresolved=0 max_back_s=0 stage=file reason=fold \
         run=1700000000\nbegin stocks 2024-04-03 NIFTY\n{}",
        done("2024-04-01"),
        done("2024-04-05"),
    );
    let keys: Vec<(ImportKind, Day, &str)> = [1, 2, 3, 5]
        .into_iter()
        .flat_map(|d| {
            [
                (ImportKind::Indices, day(2024, 4, d), "*"),
                (ImportKind::Stocks, day(2024, 4, d), "NIFTY"),
            ]
        })
        .collect();
    std::fs::write(&path, &whole).unwrap();
    let full = JournalReader::open(&store).unwrap();
    let answers = |reader: &JournalReader| {
        (
            keys.iter()
                .map(|(kind, on, filter)| reader.is_done(*kind, *on, filter))
                .collect::<Vec<_>>(),
            reader.runs_and_open(),
        )
    };
    let expected = answers(&full);
    assert_eq!(expected.0.iter().filter(|done| **done).count(), 2);
    assert_eq!(
        expected.1,
        (1, 1),
        "one run; one key begun and never closed"
    );
    for cut_at in 0..=whole.len() {
        std::fs::write(&path, &whole.as_bytes()[..cut_at]).unwrap();
        let Ok(mut reader) = JournalReader::open(&store) else {
            panic!("a prefix of a journal is a journal: cut at {cut_at}");
        };
        {
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .unwrap();
            file.write_all(&whole.as_bytes()[cut_at..]).unwrap();
        }
        reader.refresh().unwrap();
        assert_eq!(answers(&reader), expected, "cut at {cut_at}");
        assert_eq!(reader.loaded(), whole.len() as u64, "cut at {cut_at}");
    }
    std::fs::remove_dir_all(store).unwrap();
}

/// The cost of a refresh is the bytes appended, not the journal's length
/// (L-IA-2, RA-091): the journal load and a one-line refresh at 1x, 10x and
/// 100x of a 9,180-line journal (10 Oct's), p50, p99 and max over n reads
/// each; and at the real journal's line length (14 Oct 2026: 14,027,805
/// bytes in 23,270 lines, about 600 bytes a line, its `done` lines carrying
/// filters of up to 170 names) at 1x and 10x of 23,270 lines, invented
/// names of 8 letters (100x, 1.4 GB, is an extrapolation). When
/// `IA1_JOURNAL_SAMPLE` names a journal, a copy of it in a scratch
/// directory is loaded 50 times too (read only; nothing of it is kept).
/// Run on demand: `cargo test -p pull --release -- --ignored
/// journal_load_and_refresh_costs --nocapture`.
#[test]
#[ignore = "a timing, run on demand"]
fn journal_load_and_refresh_costs() {
    let store = scratch("costs");
    let path = journal_path(&store);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let names: Vec<String> = (0..60).map(|i| format!("SYMQ{i:04}")).collect();
    let long_filter = names.join(",");
    // 1x, 10x and 100x of 10 Oct's 9,180 short lines; 1x and 10x of 14
    // Oct's 23,270 lines at the real length.
    let runs: [(&str, usize); 5] = [
        ("*", 9_180),
        ("*", 91_800),
        ("*", 918_000),
        ("long", 23_270),
        ("long", 232_700),
    ];
    for (filter, lines) in runs {
        let line_of = |on: &str| {
            if filter == "*" {
                done(on)
            } else {
                format!(
                    "done indices {on} {long_filter} definition={BAR_DEFINITION} files=60 \
                     seconds=1 failures=0 late=0 late_unresolved=0 max_back_s=0\n"
                )
            }
        };
        let mut text = format!("{RUN} definition={BAR_DEFINITION}\n");
        let mut on = day(1990, 1, 1);
        for _ in 0..lines {
            text.push_str(&line_of(&on.to_string()));
            on = Day::from_days(on.days_from_epoch() + 1).unwrap();
        }
        std::fs::write(&path, &text).unwrap();
        let n = if text.len() > 32 << 20 { 20 } else { 200 };
        let mut loads = Vec::with_capacity(n);
        for _ in 0..n {
            let at = Instant::now();
            let _reader = JournalReader::open(&store).unwrap();
            loads.push(at.elapsed());
        }
        let mut reader = JournalReader::open(&store).unwrap();
        let mut refreshes = Vec::with_capacity(200);
        for _ in 0..200 {
            {
                use std::io::Write as _;
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .unwrap();
                file.write_all(line_of(&on.to_string()).as_bytes()).unwrap();
            }
            on = Day::from_days(on.days_from_epoch() + 1).unwrap();
            let at = Instant::now();
            assert_eq!(reader.refresh().unwrap(), Refreshed::Read { lines: 1 });
            refreshes.push(at.elapsed());
        }
        let pct = |v: &mut Vec<std::time::Duration>| {
            v.sort();
            let at = |q: usize| v[(v.len() * q / 100).min(v.len() - 1)];
            (at(50), at(99), v[v.len() - 1], v.len())
        };
        println!(
            "L-IA-2 {lines} lines, filter {filter} ({} B): load p50/p99/max/n {:?}; one-line refresh {:?}",
            text.len(),
            pct(&mut loads),
            pct(&mut refreshes)
        );
    }
    if let Some(sample) = std::env::var_os("IA1_JOURNAL_SAMPLE") {
        std::fs::copy(sample, &path).unwrap();
        let mut loads = Vec::with_capacity(50);
        let mut loaded = 0;
        for _ in 0..50 {
            let at = Instant::now();
            loaded = JournalReader::open(&store).unwrap().loaded();
            loads.push(at.elapsed());
        }
        println!(
            "L-IA-2 a copy of a real journal ({loaded} B of whole lines): load {}",
            spread(&mut loads)
        );
    }
    std::fs::remove_dir_all(store).unwrap();
}

/// p50, p99 and max of `samples`, and how many.
fn spread(samples: &mut [std::time::Duration]) -> String {
    samples.sort();
    let at = |q: usize| samples[(samples.len() * q / 100).min(samples.len() - 1)];
    format!(
        "p50 {:?} p99 {:?} max {:?} n {}",
        at(50),
        at(99),
        samples[samples.len() - 1],
        samples.len()
    )
}

/// C-GR-02 (the progress write, O(1)) and C-GR-06 (naming a failure, O(1)
/// amortised whatever the run already named, D-4839), each p50, p99 and
/// max. Run on demand: `cargo test -p pull --release -- --ignored
/// progress_and_failure_costs --nocapture`.
#[test]
#[ignore = "a timing, run on demand"]
fn progress_and_failure_costs() {
    let root = scratch("write-costs");
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let log = installed();
    let planned = plan(
        request(&[ImportKind::Options]),
        &process(&store, &log),
        &env_with(vec![(TICKSTORE_VAR, root.clone())]),
        &mut SourceCache::default(),
    )
    .unwrap();
    let progress = ProgressFile::new(
        store.join("imports").join("gdfl.options.progress"),
        &planned,
        &planned.kinds[0],
        4,
    );
    let mut report = Report::default();
    let mut writes = Vec::with_capacity(1_000);
    for closed in 1..=1_000 {
        report.days_imported = closed;
        let at = Instant::now();
        progress.day_passed(ImportKind::Options, day(2024, 4, 1), &report);
        writes.push(at.elapsed());
    }
    assert_eq!(progress.writes(), 1_000);
    println!(
        "C-GR-02 progress write (4 KiB at offset 0): {}",
        spread(&mut writes)
    );
    let cause = Cause::open(Stage::Vendor, "malformed_row");
    let mut named = Report::default();
    let mut said = String::new();
    for total in [FAILURES_KEPT, 10 * FAILURES_KEPT, 100 * FAILURES_KEPT] {
        while named.failures_total < total - 1_000 {
            named.name(cause, failure(named.failures_total));
        }
        let mut each = Vec::with_capacity(1_000);
        for _ in 0..1_000 {
            let one = failure(named.failures_total);
            let at = Instant::now();
            named.name(cause, one);
            each.push(at.elapsed());
        }
        assert!(named.failures.len() <= 2 * FAILURES_KEPT);
        let _ = std::fmt::Write::write_fmt(
            &mut said,
            format_args!("at {total} named: {}; ", spread(&mut each)),
        );
    }
    println!("C-GR-06 Report::name: {said}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A link planted at the progress file's name is refused, never followed:
/// the file it points at is untouched and the fault is named (RA-317).
#[test]
fn a_link_at_the_progress_name_is_refused_and_never_followed() {
    let root = scratch("planted");
    let store = root.join("store");
    std::fs::create_dir_all(store.join("imports")).unwrap();
    let target = root.join("elsewhere");
    std::fs::write(&target, b"keep me").unwrap();
    let name = store.join("imports").join("gdfl.indices.progress");
    std::os::unix::fs::symlink(&target, &name).unwrap();
    let log = installed();
    let planned = plan(
        request(&[ImportKind::Indices]),
        &process(&store, &log),
        &env_with(vec![(TICKSTORE_VAR, root.clone())]),
        &mut SourceCache::default(),
    )
    .unwrap();
    let progress = ProgressFile::new(name, &planned, &planned.kinds[0], 4);
    progress.finished(ImportKind::Indices, Ok(&Report::default()));
    assert_eq!(progress.writes(), 0);
    assert!(progress.fault().is_some());
    assert_eq!(std::fs::read(&target).unwrap(), b"keep me");
    std::fs::remove_dir_all(root).unwrap();
}

/// **NO TWO RUN PROCESSES SHARE A LOG DIRECTORY** (IA-GR-24's other half,
/// D-4862): on either layout two GDFL runs log to one directory only when
/// they hold one lock, which no two processes hold at once; on v4 every kind
/// has its own, and no run logs to `logs/cli`.
#[test]
fn no_two_run_processes_share_a_log_directory() {
    use telemetry::run_dirs::{CLI, GdflLayout, for_gdfl};
    for layout in [GdflLayout::V3, GdflLayout::V4] {
        let dirs: Vec<_> = ImportKind::ALL
            .into_iter()
            .map(|kind| for_gdfl(kind.as_str(), layout).unwrap())
            .collect();
        for a in &dirs {
            assert_ne!(a.sub, CLI);
            for b in &dirs {
                assert_eq!(a.sub == b.sub, a.lock == b.lock, "{a:?} {b:?}");
            }
        }
        let distinct: std::collections::BTreeSet<_> = dirs.iter().map(|d| d.sub).collect();
        assert_eq!(distinct.len(), if layout == GdflLayout::V4 { 3 } else { 1 });
    }
}

/// **A journal past its stated load size is named on every load** (RA-091,
/// `docs/06-limits.md` L-IA-2): at the bound nothing is said; one byte past
/// it one `warn` names the journal, its bytes and lines, the bound and the
/// limit row, and the load itself goes on (a whole load is O(lines), and
/// past the bound it is no longer the measured one).
#[test]
fn a_journal_past_its_stated_size_is_named_on_every_load() {
    use crate::gdfl_import::{JOURNAL_LOAD_WARN_BYTES, load_warning};
    use telemetry::Value;
    assert!(load_warning("j", 0, 0).is_none());
    assert!(load_warning("j", JOURNAL_LOAD_WARN_BYTES, 9).is_none());
    let path = "/s/imports/gdfl.journal";
    let said = load_warning(path, JOURNAL_LOAD_WARN_BYTES + 1, 7).unwrap();
    assert_eq!(said.level(), telemetry::Level::Warn);
    assert_eq!(said.target(), crate::gdfl_import::TARGET);
    assert_eq!(said.message(), "journal past its stated load size");
    assert_eq!(
        said.fields(),
        &[
            ("journal", Value::Str(path)),
            ("bytes", Value::Uint(JOURNAL_LOAD_WARN_BYTES + 1)),
            ("lines", Value::Uint(7)),
            ("bound", Value::Uint(JOURNAL_LOAD_WARN_BYTES)),
            ("limit", Value::Str("L-IA-2")),
        ]
    );
    // The largest journal a u64 can name is said too, never wrapped.
    assert!(load_warning(path, u64::MAX, u64::MAX).is_some());
}
