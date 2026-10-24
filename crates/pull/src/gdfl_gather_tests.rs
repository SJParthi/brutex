//! IA-17b: THE ONCE-PER-MONTH GATHER AND THE CASH `.auc` OF A REPAIRED MONTH
//! (D-4875 (3), (5); D-4899; FM-092, FM-098; critic IA-17-3).
//!
//! A repair keeps a whole copy of its month for ever, and the back-fill door
//! repaired one file at a time, so a month that held only a later day took
//! one generation for every other day of it. These tests hold the import to
//! one generation per (instrument, month) per run, with every day of it the
//! run reads gathered into that one repair; to the days' closes waiting for
//! it and written in day order; to a stop, a crash and a full disk never
//! stranding a day (the days are closed by the stop, or left begun for the
//! next run, never closed over bars that are in no month); and, for a CASH
//! share of the closing auction's era, to the new generation's `.auc`
//! written whole before the generation is served.
//!
//! Every price, row, symbol file and master line is invented at run time;
//! no vendor row is quoted (`gdfl_cm::tests::fixtures_are_built_not_pasted`
//! walks this file too).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_run::stop::StopControl;
use crate::ingest::FailKind;

/// The reason every repairing run of these tests carries (D-4889).
const REASON: &str = "IA-17b test: gather the days the month lacks";

/// A run over `root`'s tick store with `options`.
fn import_with(
    root: &Path,
    (kind, from, to): (ImportKind, Day, Day),
    store: &Path,
    options: RunOptions<'_>,
) -> Result<Report, ImportRefusal> {
    let run = Run {
        kind,
        from,
        to,
        only: &[],
        store_root: store,
    };
    match kind {
        ImportKind::Options => run_nfo_from(&NfoTickStore::new(&root.join("ts")), &run, options),
        _ => run_cm_from(&TickStore::new(&root.join("ts")), &run, options),
    }
}

/// A run with no repair reason.
fn plain(root: &Path, kind: ImportKind, (from, to): (Day, Day), store: &Path) -> Report {
    import_with(root, (kind, from, to), store, RunOptions::default()).unwrap()
}

/// A run carrying [`REASON`].
fn repairing(root: &Path, kind: ImportKind, (from, to): (Day, Day), store: &Path) -> Report {
    let options = RunOptions {
        repair_reason: Some(REASON),
        ..RunOptions::default()
    };
    import_with(root, (kind, from, to), store, options).unwrap()
}

/// The one-second month of `symbol` in `segment` that holds `on`.
fn month_of(segment: &'static str, symbol: &'static str, on: Day) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: on.year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Every bar of the month at `path` as every reader opens it, or none.
fn served(store: &Path, path: StorePath<'_>, symbol: &str) -> Vec<Bar> {
    match BarFile::open_existing(store, path, filing::symbol_id_of(symbol)) {
        Ok(file) => (0..file.header().n_valid)
            .map(|index| file.read_record(index).unwrap())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// The generation of the month at `path` every reader opens.
fn generation(store: &Path, path: StorePath<'_>) -> u32 {
    store::generation::served(store, path).unwrap().ordinal()
}

/// The repair journal's lines whose first word is `verb`.
fn repair_lines(store: &Path, verb: &str) -> Vec<String> {
    std::fs::read_to_string(store.join("audit").join("gdfl.repairs"))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.split(' ').next() == Some(verb))
        .map(str::to_owned)
        .collect()
}

/// The import journal's whole text.
fn journal(store: &Path) -> String {
    std::fs::read_to_string(journal_path(store)).unwrap_or_default()
}

/// The keys `text` holds a `begin` for and no close after it: the days a
/// run left begun.
fn begun_not_closed(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut open = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let Some(key) = line.strip_prefix("begin ") else {
            continue;
        };
        let closed = lines[at + 1..].iter().any(|later| {
            ["done ", "incomplete ", "refused "].iter().any(|verb| {
                later
                    .strip_prefix(verb)
                    .and_then(|rest| rest.strip_prefix(key))
                    .is_some_and(|rest| rest.starts_with(' '))
            })
        });
        if !closed && !open.iter().any(|seen| seen == key) {
            open.push(key.to_owned());
        }
    }
    open
}

/// The line number of the first line of `text` starting with `head`.
fn line_at(text: &str, head: &str) -> usize {
    text.lines()
        .position(|line| line.starts_with(head))
        .unwrap_or_else(|| panic!("no line starting {head:?} in {text}"))
}

/// How many `repair` lines name `month`'s `instrument` (`NSE-INDEX-NIFTY`).
fn repairs_of(store: &Path, instrument: &str, month: &str) -> usize {
    let head = format!("repair {instrument} 1s {month} ");
    repair_lines(store, "repair")
        .iter()
        .filter(|line| line.starts_with(&head))
        .count()
}

// ── IA-RP-03: one generation per (instrument, month) per run ───────────────

/// **IA-RP-03 (D-4899; the 67 alias months' shape, RA-266, FM-092).** Two
/// months each held only by a later day, as the six alias indices hold only
/// a special session: a repair-reason run over every other day of both
/// months makes ONE generation per (instrument, month), April's repaired
/// when the run leaves April and May's when the run ends; `months_repaired`
/// counts months, not days; every gathered day closes `done`, in day order,
/// and none is left begun; each month holds exactly what an in-order import
/// holds; the originals are byte-identical; and the same run again writes
/// nothing. Before IA-17b each day was its own generation: April went to
/// generation 4.
#[test]
fn a_repair_reason_run_repairs_each_month_once() {
    let root = scratch("ia17b-once");
    let (may_2, may_3, may_6) = (day(2024, 5, 2), day(2024, 5, 3), day(2024, 5, 6));
    world(&root, &[d1(), d2(), d3(), d4(), d5(), may_2, may_3, may_6]);
    let (sparse, whole) = (root.join("sparse"), root.join("whole"));
    for on in [d5(), may_6] {
        let late = plain(&root, ImportKind::Indices, (on, on), &sparse);
        assert!(late.failures.is_empty(), "{on}: {:?}", late.failures);
    }
    let in_order = plain(&root, ImportKind::Indices, (d1(), may_6), &whole);
    assert!(in_order.failures.is_empty(), "{:?}", in_order.failures);
    let originals = tree(&sparse.join("bars"));
    let instruments = in_order.files / 8;
    assert!(instruments >= 2, "NIFTY and BANKNIFTY at least");

    let got = repairing(&root, ImportKind::Indices, (d1(), may_3), &sparse);

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.days_imported, 6, "{got:?}");
    assert_eq!(
        got.months_repaired,
        2 * instruments,
        "one generation per instrument and month, never one per day"
    );
    assert_eq!(got.seconds_committed, got.seconds, "every bar added");
    for (segment, symbol, name) in [
        ("INDEX", "NIFTY", "NSE-INDEX-NIFTY"),
        ("INDEX", "BANKNIFTY", "NSE-INDEX-BANKNIFTY"),
    ] {
        for (on, month) in [(d1(), "2024-04"), (may_2, "2024-05")] {
            let path = month_of(segment, symbol, on);
            assert_eq!(generation(&sparse, path), 1, "{symbol} {month}");
            assert_eq!(
                served(&sparse, path, symbol),
                served(&whole, path, symbol),
                "{symbol} {month}: what an in-order import holds"
            );
            assert_eq!(repairs_of(&sparse, name, month), 1, "{symbol} {month}");
        }
    }
    assert_eq!(
        repair_lines(&sparse, "repair").len(),
        got.months_repaired,
        "one repair line per repaired month"
    );
    assert_eq!(
        repair_lines(&sparse, "repairing").len(),
        got.months_repaired,
        "one intent per repaired month, written before it"
    );
    // THE ONE REPAIR NAMES THE GATHERED DAYS: the operator's reason first.
    let april = repair_lines(&sparse, "repair")
        .into_iter()
        .find(|line| line.starts_with("repair NSE-INDEX-NIFTY 1s 2024-04 "))
        .unwrap();
    assert!(
        april.contains(&format!(
            "reason={}___gdfl-import_indices_2024-04-01..2024-04-04_4_files_run:",
            REASON.replace(' ', "_")
        )),
        "{april}"
    );
    let text = journal(&sparse);
    assert!(begun_not_closed(&text).is_empty(), "{text}");
    let closes: Vec<usize> = ["2024-04-01", "2024-04-02", "2024-04-03", "2024-04-04"]
        .iter()
        .chain(["2024-05-02", "2024-05-03"].iter())
        .map(|on| line_at(&text, &format!("done indices {on} * ")))
        .collect();
    assert!(
        closes.windows(2).all(|pair| pair[0] < pair[1]),
        "closed in day order: {closes:?}"
    );
    // APRIL IS REPAIRED AND CLOSED BEFORE MAY IS BEGUN: the run left April.
    assert!(
        line_at(&text, "done indices 2024-04-04 * ") < line_at(&text, "begin indices 2024-05-02 *"),
        "{text}"
    );
    assert_eq!(tree(&sparse.join("bars")), originals, "never in place");

    // THE SAME RUN AGAIN: every day done, nothing written, nothing journaled.
    let store_before = (tree(&sparse), journal(&sparse));
    let again = repairing(&root, ImportKind::Indices, (d1(), may_3), &sparse);
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!((again.months_repaired, again.seconds_committed), (0, 0));
    assert_eq!((tree(&sparse), journal(&sparse)), store_before);
    std::fs::remove_dir_all(root).unwrap();
}

/// **FM-092 / CONC-007: `a_hole_early_in_a_month_costs_at_most_one_generation`.**
/// D is missing from a month that holds D-1 and D+1; one run over D and the
/// five days after it lands D, finds D+1 held and appends D+2 to D+5 to the
/// original: ONE generation of the month, holding what an in-order import
/// holds. Before IA-17b D was repaired at once, the month was then served
/// from a revision, and every later day was a generation of its own: five.
#[test]
fn a_hole_early_in_a_month_costs_at_most_one_generation() {
    let root = scratch("ia17b-fm092");
    let (apr_8, apr_9) = (day(2024, 4, 8), day(2024, 4, 9));
    world(&root, &[d1(), d2(), d3(), d4(), d5(), apr_8, apr_9]);
    let (holed, whole) = (root.join("holed"), root.join("whole"));
    for on in [d1(), d3()] {
        assert!(
            plain(&root, ImportKind::Indices, (on, on), &holed)
                .failures
                .is_empty()
        );
    }
    assert!(
        plain(&root, ImportKind::Indices, (d1(), apr_9), &whole)
            .failures
            .is_empty()
    );

    let got = repairing(&root, ImportKind::Indices, (d2(), apr_9), &holed);

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    for symbol in ["NIFTY", "BANKNIFTY"] {
        let path = month_of("INDEX", symbol, d1());
        assert!(
            generation(&holed, path) <= 1,
            "{symbol}: at most one generation"
        );
        assert_eq!(generation(&holed, path), 1, "{symbol}: D lands");
        assert_eq!(
            served(&holed, path, symbol),
            served(&whole, path, symbol),
            "{symbol}"
        );
    }
    assert!(begun_not_closed(&journal(&holed)).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// The seconds of one invented index file of a special session: three rows
/// before `open`, rows from `open` every seven seconds, three after `end`.
fn special_file(stem: &str, d: Day, base: u32, (open, end): (u32, u32)) -> Vec<u8> {
    let p = |v: u32| format!("{}.{:02}", base + v / 100, v % 100);
    let mut rows = Vec::new();
    for early in 0..3 {
        rows.push(row(stem, d, open - 300 + early, &p(early), 0, 0));
    }
    let mut at = open;
    while at < end {
        rows.push(row(stem, d, at, &p(at % 400), 0, 0));
        at += 7;
    }
    for late in 0..3 {
        rows.push(row(stem, d, end + 300 + late, &p(late), 0, 0));
    }
    csv(&rows)
}

/// **FM-098 / CONC-055: `special_sessions_cost_one_generation_per_month`.**
/// A special session (the 2025-10-21 Muhurat, the calendar of record's
/// window) and a regular day of the same month are imported after the
/// month's later days: one repair-reason run lands both in ONE generation of
/// the month, the Muhurat inside its window, and the month equals an
/// in-order import. Before IA-17b: two generations.
#[test]
fn special_sessions_cost_one_generation_per_month() {
    let root = scratch("ia17b-fm098");
    let (before, muhurat, after, last) = (
        day(2025, 10, 20),
        day(2025, 10, 21),
        day(2025, 10, 23),
        day(2025, 10, 24),
    );
    let window = (hms_at(13, 45, 0), hms_at(14, 45, 0));
    let indices = [("NIFTY 50.NSE_IDX", 31_800), ("NIFTY BANK.NSE_IDX", 61_000)];
    let regular = |d: Day| -> DayPlan {
        (
            d,
            Tree::Indices,
            indices
                .iter()
                .map(|&(stem, base)| (entry_of(Tree::Indices, d, stem), index_file(stem, d, base)))
                .collect(),
        )
    };
    let special: DayPlan = (
        muhurat,
        Tree::Indices,
        indices
            .iter()
            .map(|&(stem, base)| {
                (
                    entry_of(Tree::Indices, muhurat, stem),
                    special_file(stem, muhurat, base, window),
                )
            })
            .collect(),
    );
    world_of(
        &root,
        &[regular(before), special, regular(after), regular(last)],
    );
    let (holed, whole) = (root.join("holed"), root.join("whole"));
    for on in [before, last] {
        assert!(
            plain(&root, ImportKind::Indices, (on, on), &holed)
                .failures
                .is_empty()
        );
    }
    for on in [before, muhurat, after, last] {
        assert!(
            plain(&root, ImportKind::Indices, (on, on), &whole)
                .failures
                .is_empty()
        );
    }

    let got = repairing(&root, ImportKind::Indices, (muhurat, after), &holed);

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.days_imported, got.months_repaired), (2, 2), "{got:?}");
    let on_muhurat = i64::from(muhurat.days_from_epoch());
    for symbol in ["NIFTY", "BANKNIFTY"] {
        let path = month_of("INDEX", symbol, muhurat);
        assert_eq!(generation(&holed, path), 1, "{symbol}: one generation");
        let now = served(&holed, path, symbol);
        assert_eq!(now, served(&whole, path, symbol), "{symbol}");
        for bar in now.iter().filter(|bar| {
            (bar.ts_micros / 1_000_000 + crate::session::IST_OFFSET_SECS).div_euclid(86_400)
                == on_muhurat
        }) {
            let second = second_of(bar.ts_micros, muhurat);
            assert!((window.0..window.1).contains(&second), "{symbol} {second}");
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// **A HELD DAY BETWEEN THE DAYS A MONTH LACKS (spec §8 item 4).** The month
/// holds 2024-04-03 and 2024-04-09 and lacks the two days before the first
/// and the two after it: one span from the first lacked day to the last
/// would cross the held day and be refused as an overlap, so every day is
/// judged alone when it is gathered and the four are offered together, in
/// one repair the store merges in order. One generation; the month equals an
/// in-order import; no day refused `bars_differ_held`.
#[test]
fn a_month_holding_a_day_between_the_days_it_lacks_takes_them_in_one_repair() {
    let root = scratch("ia17b-middle");
    let apr_9 = day(2024, 4, 9);
    world(&root, &[d1(), d2(), d3(), d4(), d5(), apr_9]);
    let (holed, whole) = (root.join("holed"), root.join("whole"));
    for on in [d3(), apr_9] {
        assert!(
            plain(&root, ImportKind::Indices, (on, on), &holed)
                .failures
                .is_empty()
        );
    }
    assert!(
        plain(&root, ImportKind::Indices, (d1(), apr_9), &whole)
            .failures
            .is_empty()
    );

    let got = repairing(&root, ImportKind::Indices, (d1(), d5()), &holed);

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert!(got.closed_incomplete.is_empty(), "{got:?}");
    for symbol in ["NIFTY", "BANKNIFTY"] {
        let path = month_of("INDEX", symbol, d1());
        assert_eq!(generation(&holed, path), 1, "{symbol}");
        assert_eq!(
            served(&holed, path, symbol),
            served(&whole, path, symbol),
            "{symbol}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// **A CRASH BETWEEN THE GATHERED REPAIR'S PUBLICATION AND ITS PROMOTION
/// (IA-RP-04's second half; D-3350 `Published::Reused`).** The revision the
/// gathered repair of three days would publish is complete on disk and
/// unserved, and the three days are begun (their closes never written). The
/// next repair-reason run reads them again, gathers them again, finds the
/// revision at its ordinal byte for byte, promotes it (`revision=reused`)
/// and closes the days: still one generation, never a second.
#[test]
fn a_crash_between_publication_and_promotion_resumes_as_the_same_revision() {
    let root = scratch("ia17b-reused");
    world(&root, &[d1(), d2(), d3(), d4()]);
    let (holed, whole) = (root.join("holed"), root.join("whole"));
    assert!(
        plain(&root, ImportKind::Indices, (d4(), d4()), &holed)
            .failures
            .is_empty()
    );
    assert!(
        plain(&root, ImportKind::Indices, (d1(), d4()), &whole)
            .failures
            .is_empty()
    );
    let nifty = month_of("INDEX", "NIFTY", d1());
    let header = BarFile::open_existing(&holed, nifty, filing::symbol_id_of("NIFTY"))
        .unwrap()
        .header();
    let merged = served(&whole, nifty, "NIFTY");
    let published = store::repair::publish(
        &holed,
        nifty,
        filing::symbol_id_of("NIFTY"),
        store::repair::Revision::new(1).unwrap(),
        header,
        &merged,
    )
    .unwrap();
    assert_eq!(published, store::repair::Published::Created);
    assert_eq!(generation(&holed, nifty), 0, "published, not served");

    let got = repairing(&root, ImportKind::Indices, (d1(), d3()), &holed);

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(generation(&holed, nifty), 1);
    assert!(!holed.join("bar-revisions-v1").join("2").exists());
    assert_eq!(served(&holed, nifty, "NIFTY"), merged);
    let line = repair_lines(&holed, "repair")
        .into_iter()
        .find(|line| line.starts_with("repair NSE-INDEX-NIFTY 1s 2024-04 "))
        .unwrap();
    assert!(line.contains(" revision=reused "), "{line}");
    assert!(begun_not_closed(&journal(&holed)).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

// ── the synthetic driver: stops, crashes, a full disk, the year's end ──────

/// The opening second of the cash session.
const OPEN: u32 = 9 * 3_600 + 15 * 60;

/// An invented index file of `seconds` seconds from the open, its prices
/// made up from the symbol, the day and the second.
fn invented(symbol: &str, on: Day, seconds: u32) -> TickFile {
    let base = 900_000 + i64::from(on.days_from_epoch() % 1_000) * 100;
    let shift = i64::try_from(symbol.len()).unwrap() * 7;
    TickFile {
        symbol: symbol.to_owned(),
        contract: None,
        name: format!("{symbol}.invented"),
        ticks: (0..seconds)
            .map(|at| Tick {
                sod: OPEN + at * 61,
                ltp: base + shift + i64::from(at),
                ltq: 0,
                oi: None,
            })
            .collect(),
    }
}

/// A run of the synthetic driver over `(from, to)` into `store`, each day
/// handing `files(day)`.
fn drive_over(
    store: &Path,
    (from, to): (Day, Day),
    options: RunOptions<'_>,
    files: &dyn Fn(Day) -> Vec<TickFile>,
) -> Result<Report, ImportRefusal> {
    let run = Run {
        kind: ImportKind::Indices,
        from,
        to,
        only: &[],
        store_root: store,
    };
    drive_from(&run, options, |on, sink| {
        for file in files(on) {
            sink(file);
        }
        DayRead {
            held: true,
            ..DayRead::default()
        }
    })
}

/// NIFTY's and BANKNIFTY's four-second files.
fn both(on: Day) -> Vec<TickFile> {
    vec![invented("NIFTY", on, 4), invented("BANKNIFTY", on, 4)]
}

/// One worker, the reason carried.
fn asked<'a>() -> RunOptions<'a> {
    RunOptions {
        repair_reason: Some(REASON),
        workers: Workers::new(1).unwrap(),
        ..RunOptions::default()
    }
}

/// A store holding only `later` of NIFTY and BANKNIFTY.
fn holding(tag: &str, later: &[Day]) -> PathBuf {
    let store = scratch(tag);
    for &on in later {
        let got = drive_over(&store, (on, on), RunOptions::default(), &both).unwrap();
        assert!(got.failures.is_empty(), "{on}: {:?}", got.failures);
    }
    store
}

/// **IA-RP-04: `a_stop_or_a_crash_mid_gather_never_strands_a_begun_day`.**
///
/// - A STOP pressed while the third of four days is read: the two days
///   gathered are offered in one repair and closed `done` before the run
///   ends `Stopped`; the third day wrote nothing and has no line; nothing is
///   left begun. A resume lands the rest in ONE more generation.
/// - A CRASH (the process dies while the third day is read): the two
///   gathered days are left begun, nothing was published; the next
///   repair-reason run names them resumed, reads them again, and lands every
///   day in ONE generation, closing each.
#[test]
fn a_stop_or_a_crash_mid_gather_never_strands_a_begun_day() {
    let days = [day(2024, 4, 1), day(2024, 4, 2), day(2024, 4, 3)];
    let later = day(2024, 4, 4);
    let nifty = month_of("INDEX", "NIFTY", later);

    // ── the stop ──
    let store = holding("ia17b-stop", &[later]);
    let stop = StopControl::new(store.join("imports/gdfl.stop"), 1_790_000_301);
    let options = RunOptions {
        run: Some(1_790_000_301),
        stop: Some(&stop),
        ..asked()
    };
    let ended = drive_over(&store, (days[0], days[2]), options, &|on| {
        if on == days[2] {
            stop.ask("operator", 0);
        }
        both(on)
    })
    .unwrap_err();
    assert_eq!(ended.code(), "stopped", "{ended}");
    let done = ended.report().unwrap();
    assert_eq!(done.months_repaired, 2, "the gathered days were offered");
    assert_eq!(done.days_imported, 2, "{done:?}");
    assert_eq!(generation(&store, nifty), 1);
    let text = journal(&store);
    assert!(begun_not_closed(&text).is_empty(), "{text}");
    assert!(
        text.contains("\ndone indices 2024-04-01 * ")
            && text.contains("\ndone indices 2024-04-02 * "),
        "{text}"
    );
    assert!(!text.contains("indices 2024-04-03 *"), "{text}");
    let resumed = drive_over(&store, (days[0], days[2]), asked(), &both).unwrap();
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.months_repaired, 2, "the third day's one generation");
    assert_eq!(generation(&store, nifty), 2);
    assert!(begun_not_closed(&journal(&store)).is_empty());
    std::fs::remove_dir_all(&store).unwrap();

    // ── the crash ──
    let store = holding("ia17b-crash", &[later]);
    let died = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        drive_over(&store, (days[0], days[2]), asked(), &|on| {
            assert_ne!(on, days[2], "the process dies here");
            both(on)
        })
    }));
    assert!(died.is_err(), "the run died");
    let text = journal(&store);
    assert_eq!(
        begun_not_closed(&text),
        vec![
            "indices 2024-04-01 *".to_owned(),
            "indices 2024-04-02 *".to_owned()
        ],
        "the gathered days are begun, never closed over bars in no month: {text}"
    );
    assert_eq!(generation(&store, nifty), 0, "nothing was published");
    assert!(repair_lines(&store, "repairing").is_empty());
    let resumed = drive_over(&store, (days[0], days[2]), asked(), &both).unwrap();
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.resumed, vec![days[0], days[1]]);
    assert_eq!(resumed.months_repaired, 2);
    assert_eq!(generation(&store, nifty), 1, "one generation for all three");
    assert!(begun_not_closed(&journal(&store)).is_empty());
    std::fs::remove_dir_all(&store).unwrap();
}

/// **IA-RP-05: `a_full_disk_at_publish_leaves_the_gathered_days_to_the_next_run`.**
/// The gathered repair of NIFTY's month meets a full disk: the run ends
/// `disk_full`, no gathered day is closed (each stays begun), and no
/// generation is published for either index. With room again, the next
/// repair-reason run reads the days again and converges: one generation
/// each, every day closed `done`.
#[test]
fn a_full_disk_at_publish_leaves_the_gathered_days_to_the_next_run() {
    let days = (day(2024, 4, 1), day(2024, 4, 3));
    let later = day(2024, 4, 4);
    let store = holding("ia17b-full", &[later]);
    filing::inject::plant_gathered(&store, "NIFTY", FailKind::DiskFull);

    let ended = drive_over(&store, days, asked(), &both).unwrap_err();

    assert_eq!(ended.code(), "disk_full", "{ended}");
    let text = journal(&store);
    assert_eq!(begun_not_closed(&text).len(), 3, "{text}");
    for symbol in ["NIFTY", "BANKNIFTY"] {
        assert_eq!(generation(&store, month_of("INDEX", symbol, later)), 0);
    }
    let resumed = drive_over(&store, days, asked(), &both).unwrap();
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.resumed.len(), 3, "{resumed:?}");
    assert_eq!(resumed.months_repaired, 2);
    for symbol in ["NIFTY", "BANKNIFTY"] {
        assert_eq!(generation(&store, month_of("INDEX", symbol, later)), 1);
    }
    assert!(begun_not_closed(&journal(&store)).is_empty());
    std::fs::remove_dir_all(&store).unwrap();
}

/// **COMBINATION: A MONTH'S END AND A FULL DISK AT ONCE.** April's days hold
/// NIFTY alone and May's NIFTY and BANKNIFTY; BANKNIFTY's gathered repair of
/// May meets a full disk at the run's end. April was repaired when the run
/// left it, so its days stay closed `done`; May's NIFTY repair landed before
/// BANKNIFTY's failed, and every May day is left begun. The next run reads
/// May again: NIFTY's May bars are already served (no second generation),
/// BANKNIFTY's land in one, and every day closes.
#[test]
fn a_full_disk_at_the_runs_end_spares_the_month_it_already_left() {
    let (apr_2, apr_3, apr_4) = (day(2024, 4, 2), day(2024, 4, 3), day(2024, 4, 4));
    let (may_2, may_3) = (day(2024, 5, 2), day(2024, 5, 3));
    let store = holding("ia17b-full-boundary", &[apr_4, day(2024, 5, 6)]);
    let files = |on: Day| {
        if on.month() == 4 {
            vec![invented("NIFTY", on, 4)]
        } else {
            both(on)
        }
    };
    filing::inject::plant_gathered(&store, "BANKNIFTY", FailKind::DiskFull);

    let ended = drive_over(&store, (apr_2, may_3), asked(), &files).unwrap_err();

    assert_eq!(ended.code(), "disk_full", "{ended}");
    let text = journal(&store);
    assert!(text.contains("\ndone indices 2024-04-02 * "), "{text}");
    assert!(text.contains("\ndone indices 2024-04-03 * "), "{text}");
    assert_eq!(
        begun_not_closed(&text),
        vec![
            "indices 2024-05-02 *".to_owned(),
            "indices 2024-05-03 *".to_owned()
        ]
    );
    assert_eq!(generation(&store, month_of("INDEX", "NIFTY", apr_3)), 1);
    assert_eq!(generation(&store, month_of("INDEX", "NIFTY", may_2)), 1);
    assert_eq!(generation(&store, month_of("INDEX", "BANKNIFTY", may_2)), 0);

    let resumed = drive_over(&store, (apr_2, may_3), asked(), &files).unwrap();
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(resumed.resumed, vec![may_2, may_3]);
    assert_eq!(resumed.months_repaired, 1, "BANKNIFTY's May alone");
    assert_eq!(generation(&store, month_of("INDEX", "NIFTY", may_2)), 1);
    assert_eq!(generation(&store, month_of("INDEX", "BANKNIFTY", may_2)), 1);
    assert!(begun_not_closed(&journal(&store)).is_empty());
    std::fs::remove_dir_all(&store).unwrap();
}

/// **THE YEAR'S END.** December 2024 lacks 30 Dec and holds 31 Dec; January
/// 2025 lacks 1 and 2 Jan and holds 3 Jan. One repair-reason run from 30
/// Dec to 2 Jan: December is repaired and its day closed when the run
/// reaches 1 Jan, before 1 Jan is begun; January's two days in one more
/// generation at the run's end. One generation per month.
#[test]
fn a_run_across_the_years_end_repairs_each_month_once() {
    let (dec_30, dec_31) = (day(2024, 12, 30), day(2024, 12, 31));
    let (jan_1, jan_2, jan_3) = (day(2025, 1, 1), day(2025, 1, 2), day(2025, 1, 3));
    let store = holding("ia17b-year", &[dec_31, jan_3]);

    let got = drive_over(&store, (dec_30, jan_2), asked(), &both).unwrap();

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.months_repaired, 4, "two indices, two months");
    for symbol in ["NIFTY", "BANKNIFTY"] {
        for on in [dec_30, jan_1] {
            assert_eq!(generation(&store, month_of("INDEX", symbol, on)), 1);
        }
    }
    let text = journal(&store);
    assert!(
        line_at(&text, "done indices 2024-12-30 * ") < line_at(&text, "begin indices 2025-01-01 *"),
        "December closed before January began: {text}"
    );
    assert!(begun_not_closed(&text).is_empty());
    std::fs::remove_dir_all(&store).unwrap();
}

/// **ONE BAR, NO BAR, AND A WHOLE DAY IN ONE MONTH.** A lacked day whose
/// file holds one traded second, a lacked day whose file holds none, and a
/// lacked day of four seconds: the empty day writes nothing and gathers
/// nothing, and the other two land in ONE generation that adds exactly
/// their five bars. Before IA-17b: two generations.
#[test]
fn a_one_bar_day_and_an_empty_day_cost_no_generation_of_their_own() {
    let (one, none, four, later) = (
        day(2024, 4, 1),
        day(2024, 4, 2),
        day(2024, 4, 3),
        day(2024, 4, 4),
    );
    let store = scratch("ia17b-sizes");
    let only_nifty = |on: Day| vec![invented("NIFTY", on, 4)];
    assert!(
        drive_over(&store, (later, later), RunOptions::default(), &only_nifty)
            .unwrap()
            .failures
            .is_empty()
    );
    let files = |on: Day| {
        let seconds = if on == one {
            1
        } else if on == none {
            0
        } else {
            4
        };
        vec![invented("NIFTY", on, seconds)]
    };

    let got = drive_over(&store, (one, four), asked(), &files).unwrap();

    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!(got.months_repaired, 1);
    assert_eq!(got.seconds_committed, 5, "one bar and four");
    let nifty = month_of("INDEX", "NIFTY", later);
    assert_eq!(generation(&store, nifty), 1);
    assert_eq!(served(&store, nifty, "NIFTY").len(), 9);
    assert!(begun_not_closed(&journal(&store)).is_empty());
    std::fs::remove_dir_all(&store).unwrap();
}

/// **OUT OF THE BOX: ONE INSTRUMENT'S MONTH CAN NEVER BE REPAIRED, THE
/// OTHER'S CAN.** The two indices share every gathered day. NIFTY's repair
/// is refused for good (`repair_impossible`, as a held bar that would
/// change or the month's 1,024 revisions spent answer it) and BANKNIFTY's
/// lands. Each shared day closes ONCE, `incomplete reason=repair_impossible`
/// (a hole no run reads again, named on the receipt), never `done` over
/// NIFTY's lost day and never left begun; BANKNIFTY's month is served from
/// its one generation; the refusal is named on every NIFTY file it cost.
#[test]
fn one_instruments_refused_repair_closes_the_shared_days_once_and_spares_the_other() {
    let days = (day(2024, 4, 1), day(2024, 4, 2));
    let later = day(2024, 4, 4);
    let store = holding("ia17b-mixed", &[later]);
    filing::inject::plant_gathered(&store, "NIFTY", FailKind::RepairImpossible);

    let got = drive_over(&store, days, asked(), &both).unwrap();

    assert_eq!(got.closed_incomplete, vec![days.0, days.1], "{got:?}");
    assert_eq!(
        got.holes_made,
        vec![
            (days.0, "repair_impossible".to_owned()),
            (days.1, "repair_impossible".to_owned())
        ]
    );
    assert_eq!(got.months_repaired, 1, "BANKNIFTY's month");
    assert_eq!(
        got.failures
            .iter()
            .filter(|failure| failure.why.contains("repair_impossible"))
            .count(),
        2,
        "named on each NIFTY file: {:?}",
        got.failures
    );
    assert_eq!(generation(&store, month_of("INDEX", "NIFTY", later)), 0);
    assert_eq!(generation(&store, month_of("INDEX", "BANKNIFTY", later)), 1);
    let text = journal(&store);
    assert!(begun_not_closed(&text).is_empty(), "{text}");
    for on in ["2024-04-01", "2024-04-02"] {
        let head = format!("incomplete indices {on} * ");
        assert_eq!(
            text.lines().filter(|line| line.starts_with(&head)).count(),
            1,
            "{text}"
        );
        assert!(
            text.lines()
                .any(|line| line.starts_with(&head) && line.contains(" reason=repair_impossible")),
            "{text}"
        );
    }
    std::fs::remove_dir_all(&store).unwrap();
}

/// **COMBINATION: A STOP AT A MONTH'S BOUNDARY.** The stop is pressed while
/// May's first lacked day is read: April's days were repaired and closed
/// when the run left April, and the stop closes nothing more of May (its day
/// wrote nothing), so every begun day is closed and April is one generation.
#[test]
fn a_stop_on_a_months_first_day_closes_the_month_it_left() {
    let (apr_2, apr_3, may_2) = (day(2024, 4, 2), day(2024, 4, 3), day(2024, 5, 2));
    let store = holding("ia17b-stop-boundary", &[day(2024, 4, 4), day(2024, 5, 3)]);
    let stop = StopControl::new(store.join("imports/gdfl.stop"), 1_790_000_302);
    let options = RunOptions {
        run: Some(1_790_000_302),
        stop: Some(&stop),
        ..asked()
    };

    let ended = drive_over(&store, (apr_2, may_2), options, &|on| {
        if on == may_2 {
            stop.ask("operator", 0);
        }
        both(on)
    })
    .unwrap_err();

    assert_eq!(ended.code(), "stopped", "{ended}");
    assert_eq!(ended.report().unwrap().months_repaired, 2);
    assert_eq!(generation(&store, month_of("INDEX", "NIFTY", apr_3)), 1);
    assert_eq!(generation(&store, month_of("INDEX", "NIFTY", may_2)), 0);
    let text = journal(&store);
    assert!(begun_not_closed(&text).is_empty(), "{text}");
    assert!(!text.contains("indices 2024-05-02 *"), "{text}");
    std::fs::remove_dir_all(&store).unwrap();
}

/// The gathered repair's event is an audited state change (D-4893): a
/// month repaired once for its gathered days reaches the audit stream at
/// `info`, by its lifecycle word.
#[test]
fn the_gathered_repair_event_is_audited() {
    assert!(telemetry::is_audited(
        telemetry::Level::Info,
        TARGET,
        filing::GATHERED_REPAIR_FINISHED
    ));
}

// ── the gather itself ──────────────────────────────────────────────────────

/// `n` invented bars of one second each from `from`.
fn bars_from(from: i64, n: usize) -> Vec<Bar> {
    (0..n)
        .map(|at| Bar {
            ts_micros: (from + i64::try_from(at).unwrap()) * 1_000_000,
            open: 500_000,
            high: 500_004,
            low: 499_996,
            close: 500_001,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect()
}

/// The request of a run over April 2024.
fn april_request() -> crate::fetch::BarRequest {
    crate::fetch::BarRequest {
        instrument_id: String::new(),
        listing: ImportKind::Indices.listing(),
        window: crate::session::Window::new(day(2024, 4, 1), day(2024, 4, 30)).unwrap(),
        granularity: crate::vendor::Granularity::Second1,
    }
}

/// **THE MOST A MONTH HOLDS, AND A SECOND READ TWICE.** An offer is refused
/// by name once it would pass `store::repair::MAX_ROWS` (never cut, the
/// file not gathered); a second gathered twice alike is kept once, and one
/// gathered twice differently is refused, naming the stamp.
#[test]
fn the_gather_refuses_past_the_most_a_month_holds_and_a_second_read_twice_differently() {
    let april = day(2024, 4, 1).year_month().unwrap();
    let key: HeldKey = ("NIFTY".to_owned(), None, april);
    let file = |seconds: Vec<Bar>| filing::GatheredFile::for_test(&key, seconds, april_request());
    let start = 1_711_943_100;

    let mut gather = filing::Gather::default();
    gather
        .take(
            file(bars_from(start, store::repair::MAX_ROWS)),
            ("k1", day(2024, 4, 1)),
        )
        .unwrap();
    let past = gather
        .take(file(bars_from(start - 10, 1)), ("k2", day(2024, 4, 2)))
        .unwrap_err();
    assert_eq!(import_cause(&past).code, "repair_refused", "{past}");
    assert!(past.to_string().contains("never cut"), "{past}");

    let mut gather = filing::Gather::default();
    gather
        .take(file(bars_from(start, 4)), ("k1", day(2024, 4, 1)))
        .unwrap();
    gather
        .take(file(bars_from(start + 2, 4)), ("k2", day(2024, 4, 2)))
        .unwrap();
    assert_eq!(gather.offered(&key), Some(6), "two seconds kept once");
    let mut other = bars_from(start + 1, 1);
    other[0].close += 1;
    let differs = gather
        .take(file(other), ("k3", day(2024, 4, 3)))
        .unwrap_err();
    assert_eq!(import_cause(&differs).code, "bars_differ_held", "{differs}");
    assert_eq!(gather.offered(&key), Some(6), "nothing of it gathered");
}
