#![cfg(test)]
//! R4-22: a vendor format change in either GDFL reader (ERR-11, D-7312) and
//! the refused rows' one audited line (IMPORT-BAD-ROW, D-7313).
//!
//! Every value is invented at run time and no vendor row is quoted
//! (`gdfl_cm::tests::fixtures_are_built_not_pasted` walks this file). Each
//! renamed header carries a column name no other test uses, because the
//! once-per-shape memory is one per process and the tests share it.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::gdfl_archive::day_zip_name;
use crate::gdfl_fixtures::{bts, put, row, scratch};
use crate::gdfl_format::{
    FORMAT_TARGET, MAX_SHAPES_REMEMBERED, first_sight, note_columns_changed, note_format_changed,
};
use crate::gdfl_import::{
    ImportKind, Report, Run, Stage, cm_cause, journal_path, nfo_cause, run_cm,
};
use crate::gdfl_nfo::{NfoRefusal, NfoTickStore};
use crate::gdfl_tickstore::TickStore;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// An invented index level: far above any real one, two places.
fn level(n: u32) -> String {
    format!("{}.{:02}", 77_000 + n / 100, n % 100)
}

/// A whole index day file of `stem` under `header`: 120 seconds from
/// 09:15:00 and one row at 15:31:59, so it ends past the session.
fn index_file(header: &str, stem: &str, d: Day) -> Vec<u8> {
    let mut text = format!("{header}\r\n");
    for i in 0..120 {
        text.push_str(&row(stem, d, at(9, 15, 0) + i, &level(i * 7), 0, 0));
        text.push_str("\r\n");
    }
    text.push_str(&row(stem, d, at(15, 31, 59), &level(3), 0, 0));
    text.push_str("\r\n");
    text.into_bytes()
}

/// The modern header with its last column renamed `name`: ten columns still,
/// so every row would decode.
fn renamed(name: &str) -> String {
    HEADER_OPEN_INTEREST.replace("OpenInterest", name)
}

/// Writes one tick-store indices day holding NIFTY 50 and NIFTY BANK under
/// the headers given.
fn indices_day(root: &Path, d: Day, (nifty, bank): (&str, &str)) {
    let folder = day_folder_name(CmKind::Indices, d);
    let files = [
        (
            format!("{folder}/NIFTY 50.NSE_IDX.csv"),
            index_file(nifty, "NIFTY 50.NSE_IDX", d),
        ),
        (
            format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
            index_file(bank, "NIFTY BANK.NSE_IDX", d),
        ),
    ];
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let zip_name = day_zip_name(CmKind::Indices, d);
    let stem = zip_name.strip_suffix(".zip").unwrap();
    put(root, &format!("ts/cm/{stem}.bts"), &bts(&refs));
}

/// One indices run over the tick store under `root`, swept scope.
fn import_indices(root: &Path, store: &Path, (from, to): (Day, Day)) -> Report {
    let run = Run {
        kind: ImportKind::Indices,
        from,
        to,
        only: &[],
        store_root: store,
    };
    run_cm(
        &TickStore::new(&root.join("ts")).with_scope(Arc::new(CmScope::swept())),
        &run,
    )
    .unwrap()
}

/// **ERR-11: A RENAMED HEADER COLUMN LEAVES THE DAY INCOMPLETE AND ALERTS
/// ONCE.** NIFTY BANK's header renames one column (ten columns still: its
/// rows would decode). The file is refused by name, `header_unknown`, its
/// words carrying the shape's digest; the cause is open, so the day closes
/// `incomplete` (NIFTY 50 was read clean) and is never `done`; the shape is
/// said once in the process, naming the first file. A second day of the same
/// shape, the first of the next month (so the import's month hold-back,
/// D-4851, does not keep it unread), is refused the same way and the shape is
/// NOT said again. Before
/// R4-22 the cause closed the day `done with refusals`, so the next run never
/// read it again, and no line named the shape.
#[test]
fn a_renamed_header_column_leaves_the_day_incomplete_and_alerts_once() {
    let root = scratch("r422-renamed");
    let (d1, d2) = (day(2024, 4, 30), day(2024, 5, 2));
    let header = renamed("OpenInterestR422A");
    indices_day(&root, d1, (HEADER_OPEN_INTEREST, &header));
    indices_day(&root, d2, (HEADER_OPEN_INTEREST, &header));
    let shape = HeaderShape::of(header.as_bytes());
    assert_eq!(shape.columns, 10);
    assert_ne!(shape, HeaderShape::of(HEADER_OPEN_INTEREST.as_bytes()));
    let store = root.join("S");
    let got = import_indices(&root, &store, (d1, d2));
    assert_eq!((got.files, got.files_refused), (2, 2), "{:?}", got.failures);
    assert_eq!(got.closed_incomplete, vec![d1, d2], "{got:?}");
    assert!(got.closed_with_refusals.is_empty() && got.closed_refused.is_empty());
    for failure in &got.failures {
        assert!(
            failure.instrument.contains("NIFTY BANK.NSE_IDX.csv"),
            "{failure:?}"
        );
        assert!(failure.why.contains(&shape.hex()), "{failure:?}");
        assert!(failure.why.contains("10 column(s)"), "{failure:?}");
    }
    // Said once, in the run: the same shape asked again is not said.
    assert!(!note_format_changed("indices", d2, "again", &shape));
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(!journal.contains("done indices 2024-04-30"), "{journal}");
    assert!(!journal.contains("done indices 2024-05-02"), "{journal}");
    assert!(journal.contains("header_unknown"), "{journal}");
    // The next run reads both days again: an open cause is never skipped.
    let again = import_indices(&root, &store, (d1, d2));
    assert_eq!(again.days_skipped, 0, "{again:?}");
    let named = again
        .failures
        .iter()
        .filter(|f| f.why.contains(&shape.hex()))
        .count();
    assert_eq!(named, 2, "{again:?}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A day whose every wanted file is format-changed: no file was read clean,
/// so the day closes `refused` (D-3317 item 3) with the open cause, and is
/// read again by the next run, never `done`.
#[test]
fn a_day_of_format_changed_files_is_never_closed_done() {
    let root = scratch("r422-all-renamed");
    let d = day(2024, 4, 4);
    let header = renamed("OpenInterestR422B");
    indices_day(&root, d, (&header, &header));
    let store = root.join("S");
    let got = import_indices(&root, &store, (d, d));
    assert_eq!((got.files, got.files_refused), (0, 2), "{:?}", got.failures);
    assert_eq!(got.closed_refused, vec![d]);
    assert!(got.closed_with_refusals.is_empty());
    let again = import_indices(&root, &store, (d, d));
    assert_eq!(
        (again.days_skipped, again.files_refused),
        (0, 2),
        "{again:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A header renamed on ONE day only. On the last day of a month: that day is
/// refused and left incomplete, and the days around it (one of them the next
/// month's first) import clean. Inside a month: the same, except that the
/// changed series' LATER days of that month are held back by name (the
/// import's month rule, D-4851: no later day is filed before the open one, so
/// the month stays append-only) while the other index lands every day. This
/// build does not stop the run at a format change (the run's halt is R4-05's
/// automation piece, cut from this package on 18 Oct 2026).
#[test]
fn a_header_renamed_on_one_day_only_leaves_that_day_alone_incomplete() {
    // The last day of April, between clean days either side of the month.
    let root = scratch("r422-one-day");
    let (d1, d2, d3) = (day(2024, 4, 29), day(2024, 4, 30), day(2024, 5, 2));
    let header = renamed("OpenInterestR422C");
    indices_day(&root, d1, (HEADER_OPEN_INTEREST, HEADER_OPEN_INTEREST));
    indices_day(&root, d2, (HEADER_OPEN_INTEREST, &header));
    indices_day(&root, d3, (HEADER_OPEN_INTEREST, HEADER_OPEN_INTEREST));
    let store = root.join("S");
    let got = import_indices(&root, &store, (d1, d3));
    assert_eq!((got.files, got.files_refused), (5, 1), "{:?}", got.failures);
    assert_eq!(got.closed_incomplete, vec![d2]);
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(journal.contains("done indices 2024-04-29 * "), "{journal}");
    assert!(journal.contains("done indices 2024-05-02 * "), "{journal}");
    std::fs::remove_dir_all(root).unwrap();

    // Inside one month: NIFTY lands all three days; BANKNIFTY's later day is
    // held back, named, and its day stays open too.
    let root = scratch("r422-one-day-inside");
    let (d1, d2, d3) = (day(2024, 4, 8), day(2024, 4, 9), day(2024, 4, 10));
    let header = renamed("OpenInterestR422K");
    indices_day(&root, d1, (HEADER_OPEN_INTEREST, HEADER_OPEN_INTEREST));
    indices_day(&root, d2, (HEADER_OPEN_INTEREST, &header));
    indices_day(&root, d3, (HEADER_OPEN_INTEREST, HEADER_OPEN_INTEREST));
    let store = root.join("S");
    let got = import_indices(&root, &store, (d1, d3));
    assert_eq!((got.files, got.files_refused), (4, 2), "{:?}", got.failures);
    assert_eq!(got.closed_incomplete, vec![d2, d3]);
    let shape = HeaderShape::of(header.as_bytes()).hex();
    assert!(
        got.failures
            .iter()
            .any(|f| f.instrument.contains("2024-04-09") && f.why.contains(&shape)),
        "{:?}",
        got.failures
    );
    assert!(
        got.failures
            .iter()
            .any(|f| f.instrument.contains("2024-04-10") && f.why.contains("held back")),
        "{:?}",
        got.failures
    );
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(journal.contains("done indices 2024-04-08 * "), "{journal}");
    std::fs::remove_dir_all(root).unwrap();
}

/// **ERR-11: THE SAME HEADER DIGEST ALERTS ONCE.** The shape is the names
/// alone: two files of one header have one shape whatever their rows; a
/// renamed, added, dropped or reordered column, a byte-order mark, a changed
/// separator or case is another shape; the length prefix keeps `A,BC` from
/// `AB,C`. One shape is said once; another is said; and past
/// [`MAX_SHAPES_REMEMBERED`] a new shape is still said every time (memory
/// bounded, never a dropped line) while a remembered one stays quiet.
#[test]
fn the_same_header_digest_alerts_once() {
    let known = HeaderShape::of(HEADER_OPEN_INTEREST.as_bytes());
    let spaced = HeaderShape::of(HEADER_OPEN_INTEREST_SPACED.as_bytes());
    assert_ne!(known, spaced);
    assert_eq!(known.columns, 10);
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice(HEADER_OPEN_INTEREST.as_bytes());
    let variants = [
        renamed("OI").into_bytes(),
        format!("{HEADER_OPEN_INTEREST},Extra").into_bytes(),
        HEADER_OPEN_INTEREST
            .replace(",OpenInterest", "")
            .into_bytes(),
        HEADER_OPEN_INTEREST
            .replace("Date,Time", "Time,Date")
            .into_bytes(),
        bom,
        HEADER_OPEN_INTEREST.replace(',', ";").into_bytes(),
        HEADER_OPEN_INTEREST.to_lowercase().into_bytes(),
        b"A,BC".to_vec(),
        b"AB,C".to_vec(),
        Vec::new(),
    ];
    let shapes: HashSet<HeaderShape> = variants.iter().map(|v| HeaderShape::of(v)).collect();
    assert_eq!(
        shapes.len(),
        variants.len(),
        "every variant is its own shape"
    );
    assert!(!shapes.contains(&known) && !shapes.contains(&spaced));
    assert_eq!(HeaderShape::of(b"").columns, 0);
    assert_eq!(HeaderShape::of(b",").columns, 2);
    assert_eq!(HeaderShape::of(b"A,BC"), HeaderShape::of(b"A,BC"));
    assert_eq!(known.hex().len(), 64);
    assert!(
        known
            .hex()
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    // Once per shape, through the process's own memory.
    let mine = HeaderShape::of(renamed("OpenInterestR422D").as_bytes());
    assert!(note_format_changed(
        "stocks",
        day(2024, 4, 2),
        "ZZ.NSE.csv",
        &mine
    ));
    assert!(!note_format_changed(
        "stocks",
        day(2024, 4, 3),
        "ZZ2.NSE.csv",
        &mine
    ));
    let other = HeaderShape::of(renamed("OpenInterestR422E").as_bytes());
    assert!(note_format_changed(
        "stocks",
        day(2024, 4, 3),
        "ZZ3.NSE.csv",
        &other
    ));
    // The bound, on a set of the test's own.
    let mut seen = HashSet::new();
    for n in 0..MAX_SHAPES_REMEMBERED {
        let digest = HeaderShape::of(format!("R422F{n}").as_bytes()).digest;
        assert!(first_sight(&mut seen, digest));
        assert!(!first_sight(&mut seen, digest));
    }
    assert_eq!(seen.len(), MAX_SHAPES_REMEMBERED);
    let past = HeaderShape::of(b"R422G").digest;
    assert!(first_sight(&mut seen, past), "past the bound: said");
    assert!(
        first_sight(&mut seen, past),
        "and said again, never dropped"
    );
    assert_eq!(seen.len(), MAX_SHAPES_REMEMBERED, "the memory did not grow");
    let remembered = HeaderShape::of(b"R422F0").digest;
    assert!(!first_sight(&mut seen, remembered));
}

/// The cause of an unknown header is OPEN in both readers, at stage vendor,
/// under one code; a series refusal closes (G3). The causes table is the
/// one place either is decided.
#[test]
fn an_unknown_header_is_an_open_cause_in_both_readers() {
    let shape = HeaderShape::of(b"Ticker,When");
    for cause in [
        cm_cause(&CmRefusal::HeaderUnknown { shape }),
        nfo_cause(&NfoRefusal::HeaderUnknown { shape }),
        nfo_cause(&NfoRefusal::Source(CmRefusal::HeaderUnknown { shape })),
    ] {
        assert_eq!(
            (cause.stage, cause.code, cause.closes),
            (Stage::Vendor, "header_unknown", false)
        );
        assert!(!cause.is_hole(), "a format change is no store hole");
    }
}

/// The options reader: a renamed column refuses the file by its shape, a
/// header with up to eight empty trailing fields (D-4829) is no format change,
/// nine are, and an empty file is the shape of no column. Through the
/// import, the options day's file is refused `header_unknown` (open) and the
/// shape is said once.
#[test]
fn an_options_header_change_is_refused_by_its_shape_and_said_once() {
    let d = day(2024, 4, 2);
    let stem = "ZZOPT04APR2477000CE.NFO";
    let one = row(stem, d, at(9, 15, 0), "4.35", 75, 1_000);
    let header = renamed("OpenInterestR422H");
    let bytes = format!("{header}\r\n{one}\r\n");
    assert_eq!(
        crate::gdfl_nfo::decode(bytes.as_bytes(), stem, d),
        Err(NfoRefusal::HeaderUnknown {
            shape: HeaderShape::of(header.as_bytes())
        })
    );
    let padded = format!("{HEADER_OPEN_INTEREST},,,,\r\n{one},,,,\r\n");
    assert_eq!(
        crate::gdfl_nfo::decode(padded.as_bytes(), stem, d)
            .unwrap()
            .trailing_empty,
        4,
        "D-4829's trailing empties are no format change"
    );
    let nine = format!("{HEADER_OPEN_INTEREST},,,,,,,,,\r\n{one},,,,,,,,,\r\n");
    match crate::gdfl_nfo::decode(nine.as_bytes(), stem, d) {
        Err(NfoRefusal::HeaderUnknown { shape }) => assert_eq!(shape.columns, 19),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        crate::gdfl_nfo::decode(b"", stem, d),
        Err(NfoRefusal::HeaderUnknown {
            shape: HeaderShape::of(b"")
        })
    );
    // Through the options import.
    let root = scratch("r422-nfo-header");
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let ticker = "NIFTY04APR2477000CE";
    let file_stem = format!("{ticker}.NFO");
    let rows: Vec<String> = (0..30)
        .map(|i| {
            row(
                &file_stem,
                d,
                at(9, 20, 0) + i * 60,
                "4.35",
                75,
                1_000 + u64::from(i),
            )
        })
        .collect();
    let mut text = format!("{header}\r\n");
    for r in &rows {
        text.push_str(r);
        text.push_str("\r\n");
    }
    let entry = format!("{folder}\\Options\\{file_stem}.csv");
    let month = format!("{}_{:04}", crate::gdfl_fixtures::mon(d), d.year());
    put(
        &root,
        &format!(
            "ts/options/{:04}/{month}/GFDLNFO_TICK_{:02}{:02}{:04}.bts",
            d.year(),
            d.day(),
            d.month(),
            d.year()
        ),
        &bts(&[(entry.as_str(), text.as_bytes())]),
    );
    let store = root.join("S");
    let run = Run {
        kind: ImportKind::Options,
        from: d,
        to: d,
        only: &[],
        store_root: &store,
    };
    let got = crate::gdfl_import::run_nfo(&NfoTickStore::new(&root.join("ts")), &run).unwrap();
    assert_eq!((got.files, got.files_refused), (0, 1), "{:?}", got.failures);
    assert!(
        got.failures[0]
            .why
            .contains(&HeaderShape::of(header.as_bytes()).hex())
    );
    assert_eq!(got.closed_refused, vec![d], "never done");
    assert!(!note_format_changed(
        "options",
        d,
        "again",
        &HeaderShape::of(header.as_bytes())
    ));
    std::fs::remove_dir_all(root).unwrap();
}

/// **IMPORT-BAD-ROW: THE REFUSED ROWS' LINE REACHES THE DURABLE AUDIT
/// STREAM.** The one line per file naming every refused row's line is an
/// `error` (D-7313), which the audit stream keeps (`telemetry::is_audited`)
/// and the `warn` flood ration never drops; at `warn` it reached the rolling
/// ring alone. The vendor format change's line is audited the same way. The
/// emit itself is read off this crate's source so a level changed back fails
/// here; `crates/pull/tests/gdfl_bad_row_audit.rs` reads the lines back off a
/// real audit stream.
#[test]
fn the_refused_rows_line_and_the_format_line_are_audited() {
    use telemetry::Level;
    assert!(telemetry::is_audited(
        Level::Error,
        crate::gdfl_import::TARGET,
        "rows refused, file read"
    ));
    assert!(!telemetry::is_audited(
        Level::Warn,
        crate::gdfl_import::TARGET,
        "rows refused, file read"
    ));
    assert!(telemetry::is_audited(
        Level::Error,
        FORMAT_TARGET,
        "vendor format changed"
    ));
    let source = include_str!("gdfl_cm.rs");
    let format = include_str!("gdfl_format.rs");
    assert!(source.contains(
        "telemetry::Event::error(crate::gdfl_import::TARGET, \"rows refused, file read\")"
    ));
    assert!(format.contains("telemetry::Event::error(FORMAT_TARGET, \"vendor format changed\")"));
}

/// Concurrent: the import's workers read a day's files on several threads,
/// so one new shape can meet the memory from many at once. Exactly one of
/// eight threads says it; the seven others are told it was said.
#[test]
fn one_shape_met_by_eight_threads_at_once_is_said_once() {
    let shape = HeaderShape::of(renamed("OpenInterestR422J").as_bytes());
    let said: usize = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|n| {
                scope.spawn(move || {
                    note_format_changed(
                        "stocks",
                        day(2024, 4, 2),
                        &format!("ZZ{n}.NSE.csv"),
                        &shape,
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| usize::from(handle.join().unwrap()))
            .sum()
    });
    assert_eq!(said, 1);
}

/// **FM-101 (IMPORT-BAD-ROW, D-7314): A TRAILING ROW WITH THE NEXT DAY'S
/// DATE IS REFUSED AS A ROW.** An index file whose first row carries the day
/// before's date at 09:07 (before the open) and whose last carries the next
/// day's at 23:59 keeps every other row, refuses those two by name as
/// `wrong_date` at their lines, and its largest stamp is a kept row's. A
/// row of another date INSIDE the session, a Date that is no real
/// `DD/MM/YYYY`, and a file whose every row is of another date still refuse
/// the whole file; past D-3318's bound the file is `TooManyBadRows`. Before
/// R4-22 the one trailing row refused the whole NIFTY 50 file, and with it
/// every option bar's spot that day.
#[test]
fn a_trailing_row_with_the_next_days_date_is_refused_as_a_row() {
    let d = day(2024, 4, 1);
    let stem = "NIFTY 50.NSE_IDX";
    let expect = Expect {
        kind: CmKind::Indices,
        stem,
        day: d,
    };
    let other = |date: Day, sod: u32| row(stem, date, sod, &level(9), 0, 0);
    let good: Vec<String> = (0..120)
        .map(|i| row(stem, d, at(9, 15, 0) + i, &level(i), 0, 0))
        .collect();
    let mut rows = vec![other(day(2024, 3, 28), at(9, 7, 1))];
    rows.extend(good.iter().cloned());
    rows.push(row(stem, d, at(15, 31, 59), &level(3), 0, 0));
    rows.push(other(day(2024, 4, 2), at(23, 59, 59)));
    let text = |rows: &[String]| {
        let mut text = format!("{HEADER_OPEN_INTEREST}\r\n");
        for r in rows {
            text.push_str(r);
            text.push_str("\r\n");
        }
        text.into_bytes()
    };
    let file = decode(&text(&rows), &expect).unwrap();
    assert_eq!(file.rows.len(), 121, "every other row read");
    assert_eq!((file.refused.wrong_date, file.refused.total()), (2, 2));
    assert_eq!(file.refused.named_text(), "2:wrong_date 124:wrong_date");
    assert_eq!(file.max_sod, at(15, 31, 59), "a refused row raises nothing");
    // Inside the session: another day's file, refused whole.
    let mut inside = good.clone();
    inside.insert(40, other(day(2024, 4, 2), at(10, 0, 0)));
    inside.push(row(stem, d, at(15, 31, 59), &level(3), 0, 0));
    assert_eq!(
        decode(&text(&inside), &expect),
        Err(CmRefusal::DateMismatch { line: 42 })
    );
    // A Date that is no real DD/MM/YYYY, outside the session: refused whole.
    for bent in ["2024-04-02", "31/02/2024", "02/04/24", "0x/04/2024"] {
        let mut odd = good.clone();
        odd.push(row(stem, d, at(15, 31, 59), &level(3), 0, 0));
        odd.push(row(stem, d, at(23, 59, 59), &level(9), 0, 0).replacen("01/04/2024", bent, 1));
        assert_eq!(
            decode(&text(&odd), &expect),
            Err(CmRefusal::DateMismatch { line: 123 }),
            "{bent}"
        );
    }
    // Every row of another date, all outside the session: the file names
    // its first refused row's refusal.
    let all: Vec<String> = (0..5)
        .map(|i| other(day(2024, 4, 2), at(20, 0, 0) + i))
        .collect();
    assert_eq!(
        decode(&text(&all), &expect),
        Err(CmRefusal::DateMismatch { line: 2 })
    );
    // Past D-3318's bound (more than 64 refused AND one in every 1,000):
    // the file is refused whole, with both counts.
    let mut many = good.clone();
    many.push(row(stem, d, at(15, 31, 59), &level(3), 0, 0));
    many.extend((0..70).map(|i| other(day(2024, 4, 2), at(20, 0, 0) + i)));
    assert_eq!(
        decode(&text(&many), &expect),
        Err(CmRefusal::TooManyBadRows {
            refused: 70,
            rows: 191
        })
    );
}

/// FM-101 through the import: the day closes `done`, the in-session
/// seconds are filed, and the refused row is counted, never a bar.
#[test]
fn a_file_with_a_next_day_trailing_row_lands_its_day() {
    let root = scratch("r422-fm101");
    let d = day(2024, 4, 1);
    let folder = day_folder_name(CmKind::Indices, d);
    let file = |stem: &str| {
        let mut text = format!("{HEADER_OPEN_INTEREST}\r\n");
        for i in 0..120 {
            text.push_str(&row(stem, d, at(9, 15, 0) + i, &level(i), 0, 0));
            text.push_str("\r\n");
        }
        text.push_str(&row(stem, d, at(15, 31, 59), &level(3), 0, 0));
        text.push_str("\r\n");
        text.push_str(&row(stem, day(2024, 4, 2), at(23, 59, 59), &level(9), 0, 0));
        text.push_str("\r\n");
        text.into_bytes()
    };
    let files = [
        (
            format!("{folder}/NIFTY 50.NSE_IDX.csv"),
            file("NIFTY 50.NSE_IDX"),
        ),
        (
            format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
            file("NIFTY BANK.NSE_IDX"),
        ),
    ];
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let zip_name = day_zip_name(CmKind::Indices, d);
    put(
        &root,
        &format!("ts/cm/{}.bts", zip_name.strip_suffix(".zip").unwrap()),
        &bts(&refs),
    );
    let store = root.join("S");
    let got = import_indices(&root, &store, (d, d));
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.files, got.files_refused, got.days_imported), (2, 0, 1));
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(journal.contains("done indices 2024-04-01 * "), "{journal}");
    std::fs::remove_dir_all(root).unwrap();
}

/// A NIFTY 50 day of 2024-04-01 for the column-set tests: 120 rows a second
/// from 09:15:00 and one at 15:31:59, each with `tail` written after its
/// tenth field (`""` for none, `",7"` for an eleventh) or, for `cut`, its
/// last field dropped. Every value is invented.
fn nifty_rows(tail: &str, cut: bool) -> Vec<String> {
    let d = day(2024, 4, 1);
    let stem = "NIFTY 50.NSE_IDX";
    (0..121)
        .map(|i| {
            let (sod, ltp) = if i < 120 {
                (at(9, 15, 0) + i, level(i))
            } else {
                (at(15, 31, 59), level(3))
            };
            let whole = row(stem, d, sod, &ltp, 0, 0);
            let base = if cut {
                whole.rsplit_once(',').unwrap().0.to_owned()
            } else {
                whole
            };
            format!("{base}{tail}")
        })
        .collect()
}

/// `rows` under the modern header, CRLF, with or without a newline after
/// the last.
fn nifty_text(rows: &[String], final_newline: bool) -> Vec<u8> {
    let mut text = format!("{HEADER_OPEN_INTEREST}\r\n");
    for (n, r) in rows.iter().enumerate() {
        text.push_str(r);
        if final_newline || n + 1 < rows.len() {
            text.push_str("\r\n");
        }
    }
    text.into_bytes()
}

/// The decode of a NIFTY 50 file of 2024-04-01.
fn nifty_decode(bytes: &[u8]) -> Result<CmFile, CmRefusal> {
    let expect = Expect {
        kind: CmKind::Indices,
        stem: "NIFTY 50.NSE_IDX",
        day: day(2024, 4, 1),
    };
    decode(bytes, &expect)
}

/// **ERR-11 (D-7312 item 5): EVERY ROW OF ONE OTHER FIELD COUNT IS A
/// COLUMN-SET CHANGE.** The vendor keeps the header and changes the rows'
/// column set: every row of an index file carries an eleventh field, or
/// drops its last. The file is refused `columns_changed`, an OPEN cause, by
/// the rows' shape (the header's names with the rows' count, never a
/// header's own shape), its last line with no newline after it counted as the
/// shape, not a cut. Before, every such row was a malformed row and the file
/// `TooManyBadRows`, a cause that CLOSED the day.
#[test]
fn every_row_of_one_other_field_count_is_a_column_set_change() {
    let header = HEADER_OPEN_INTEREST.as_bytes();
    let wide = HeaderShape::of_rows(header, 11);
    assert_eq!(wide.columns, 11);
    assert_ne!(wide, HeaderShape::of(header), "never a header's own shape");
    assert_ne!(wide, HeaderShape::of_rows(header, 9));
    assert_ne!(
        wide,
        HeaderShape::of_rows(HEADER_OPEN_INTEREST_SPACED.as_bytes(), 11)
    );
    let every = nifty_rows(",7", false);
    let changed = CmRefusal::ColumnsChanged(ColumnSet {
        shape: wide,
        header: 10,
        rows: 121,
    });
    assert_eq!(
        nifty_decode(&nifty_text(&every, true)),
        Err(changed.clone())
    );
    assert_eq!(
        nifty_decode(&nifty_text(&every, false)),
        Err(changed.clone()),
        "a last line of the changed count is the shape, not a cut"
    );
    let words = changed.to_string();
    assert!(
        words.starts_with(
            "columns_changed: every one of 121 row(s) has 11 field(s) where the header names 10"
        ),
        "{words}"
    );
    assert!(words.contains(&wide.hex()), "{words}");
    assert_eq!(
        nifty_decode(&nifty_text(&nifty_rows("", true), true)),
        Err(CmRefusal::ColumnsChanged(ColumnSet {
            shape: HeaderShape::of_rows(header, 9),
            header: 10,
            rows: 121
        })),
        "every row one field short"
    );
    let cause = cm_cause(&changed);
    assert_eq!(
        (cause.stage, cause.code, cause.closes),
        (Stage::Vendor, "columns_changed", false)
    );
    assert!(!cause.is_hole(), "a format change is no store hole");
}

/// The edges a column-set change must NOT swallow: one long row among good
/// rows is a refused row and the file read; rows of two other counts and
/// blank lines are bad rows (`TooManyBadRows`); a file of a few such rows,
/// up to D-3318's bound of 64 (an illiquid contract's one row with a stray
/// comma), is a bad file named by its first row, as before, and one more is
/// the change; a good file whose last line, with no newline, has an eleventh
/// field is a cut file refused whole (CM-07), as before.
#[test]
fn a_few_bent_rows_or_two_counts_stay_bad_rows_never_a_format_change() {
    let good = nifty_rows("", false);
    let every = nifty_rows(",7", false);
    let short = nifty_rows("", true);
    let mut one = good.clone();
    one[30].push_str(",7");
    let file = nifty_decode(&nifty_text(&one, true)).unwrap();
    assert_eq!((file.refused.malformed, file.refused.total()), (1, 1));
    assert_eq!(file.refused.named_text(), "32:malformed");
    let mixed: Vec<String> = every
        .iter()
        .zip(&short)
        .enumerate()
        .map(|(n, (w, s))| if n % 2 == 0 { w.clone() } else { s.clone() })
        .collect();
    assert_eq!(
        nifty_decode(&nifty_text(&mixed, true)),
        Err(CmRefusal::TooManyBadRows {
            refused: 121,
            rows: 121
        })
    );
    assert_eq!(
        nifty_decode(&nifty_text(
            &(0..70).map(|_| String::new()).collect::<Vec<_>>(),
            true
        )),
        Err(CmRefusal::TooManyBadRows {
            refused: 70,
            rows: 70
        }),
        "blank lines are no row of this file"
    );
    assert_eq!(
        nifty_decode(&nifty_text(&every[..3], true)),
        Err(CmRefusal::MalformedRow { line: 2 })
    );
    assert_eq!(
        nifty_decode(&nifty_text(&every[..3], false)),
        Err(CmRefusal::MalformedRow { line: 4 }),
        "a few rows: the last with no newline is a cut row"
    );
    let bound = usize::try_from(MAX_BAD_ROWS).unwrap();
    assert_eq!(
        nifty_decode(&nifty_text(&every[..bound], true)),
        Err(CmRefusal::MalformedRow { line: 2 }),
        "at the bound: still a bad file"
    );
    assert!(matches!(
        nifty_decode(&nifty_text(&every[..=bound], true)),
        Err(CmRefusal::ColumnsChanged(ColumnSet { rows: 65, .. }))
    ));
    let mut cut = good;
    cut.last_mut().unwrap().push_str(",7");
    assert_eq!(
        nifty_decode(&nifty_text(&cut, false)),
        Err(CmRefusal::MalformedRow { line: 122 })
    );
}

/// The options reader, under a header of four empty trailing fields
/// (D-4829): every row with a fifteenth field is the change, `columns_changed`
/// by the rows' shape with the header's fourteen; one such row among good
/// rows is a refused row and the file read. The cause is open.
#[test]
fn the_options_reader_names_a_column_set_change_the_same_way() {
    let d = day(2024, 4, 1);
    let option = "ZZOPT01APR2477000CE.NFO";
    let padded = format!("{HEADER_OPEN_INTEREST},,,,");
    let option_rows = |odd: Option<u32>| {
        let mut text = format!("{padded}\r\n");
        for i in 0..80 {
            let r = row(
                option,
                d,
                at(9, 15, 0) + i,
                "4.35",
                75,
                1_000 + u64::from(i),
            );
            text.push_str(&r);
            text.push_str(if odd.is_none_or(|n| n == i) {
                ",,,,,"
            } else {
                ",,,,"
            });
            text.push_str("\r\n");
        }
        text.into_bytes()
    };
    let changed = NfoRefusal::ColumnsChanged(ColumnSet {
        shape: HeaderShape::of_rows(padded.as_bytes(), 15),
        header: 14,
        rows: 80,
    });
    assert_eq!(
        crate::gdfl_nfo::decode(&option_rows(None), option, d),
        Err(changed.clone())
    );
    assert!(changed.to_string().starts_with(
        "columns_changed: every one of 80 row(s) has 15 field(s) where the header names 14"
    ));
    let kept = crate::gdfl_nfo::decode(&option_rows(Some(5)), option, d).unwrap();
    assert_eq!((kept.refused.malformed, kept.rows.len()), (1, 79));
    assert_eq!(kept.refused.named_text(), "7:malformed");
    let cause = nfo_cause(&changed);
    assert_eq!(
        (cause.stage, cause.code, cause.closes),
        (Stage::Vendor, "columns_changed", false)
    );
}

/// A column-set change through the import: NIFTY BANK's rows each carry two
/// more fields under the known header. The file is refused by name,
/// `columns_changed`, its words naming the rows' shape; the cause is open,
/// so the day closes `incomplete` (NIFTY 50 was read clean) and the next run
/// reads it again; the shape is said once in the process.
#[test]
fn a_column_set_change_leaves_the_day_incomplete_and_is_said_once() {
    let root = scratch("r422-columns");
    let d = day(2024, 4, 3);
    let folder = day_folder_name(CmKind::Indices, d);
    let plain = index_file(HEADER_OPEN_INTEREST, "NIFTY BANK.NSE_IDX", d);
    let plain = String::from_utf8(plain).unwrap();
    let mut lines = plain.split("\r\n");
    let mut wide = format!("{}\r\n", lines.next().unwrap());
    for line in lines.filter(|line| !line.is_empty()) {
        wide.push_str(line);
        wide.push_str(",8,9\r\n");
    }
    let files = [
        (
            format!("{folder}/NIFTY 50.NSE_IDX.csv"),
            index_file(HEADER_OPEN_INTEREST, "NIFTY 50.NSE_IDX", d),
        ),
        (
            format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
            wide.into_bytes(),
        ),
    ];
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let zip_name = day_zip_name(CmKind::Indices, d);
    put(
        &root,
        &format!("ts/cm/{}.bts", zip_name.strip_suffix(".zip").unwrap()),
        &bts(&refs),
    );
    let shape = HeaderShape::of_rows(HEADER_OPEN_INTEREST.as_bytes(), 12);
    let store = root.join("S");
    let got = import_indices(&root, &store, (d, d));
    assert_eq!((got.files, got.files_refused), (1, 1), "{:?}", got.failures);
    assert_eq!(got.closed_incomplete, vec![d], "{got:?}");
    let failure = &got.failures[0];
    assert!(
        failure.instrument.contains("NIFTY BANK.NSE_IDX.csv"),
        "{failure:?}"
    );
    assert!(
        failure
            .why
            .contains("columns_changed: every one of 121 row(s) has 12 field(s)"),
        "{failure:?}"
    );
    assert!(failure.why.contains(&shape.hex()), "{failure:?}");
    assert!(
        !note_columns_changed("indices", d, "again", &shape),
        "said once"
    );
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(!journal.contains("done indices 2024-04-03"), "{journal}");
    assert!(journal.contains("columns_changed"), "{journal}");
    let again = import_indices(&root, &store, (d, d));
    assert_eq!(again.days_skipped, 0, "{again:?}");
    assert_eq!(again.closed_incomplete, vec![d], "{again:?}");
    std::fs::remove_dir_all(root).unwrap();
}

/// The header shape's cost, MEASURED (p50, p99 and max over n = 200 each,
/// printed for the report): O(header bytes), once per refused file. A known
/// ten-column header and a 4 KiB line of garbage are each digested 200 times
/// and every digest is the same as the first, so the loop asserts the shape
/// is a pure function of the line.
#[test]
fn the_header_shape_costs_one_pass_over_the_line_measured() {
    let known = HEADER_OPEN_INTEREST.as_bytes().to_vec();
    let garbage: Vec<u8> = (0..4_096_u32)
        .map(|i| {
            if i % 9 == 0 {
                b','
            } else {
                b'a' + u8::try_from(i % 26).unwrap()
            }
        })
        .collect();
    for (what, line) in [("known", &known), ("4KiB", &garbage)] {
        let first = HeaderShape::of(line);
        let mut took: Vec<u128> = (0..200)
            .map(|_| {
                let started = std::time::Instant::now();
                let shape = HeaderShape::of(line);
                let ns = started.elapsed().as_nanos();
                assert_eq!(shape, first, "{what}");
                ns
            })
            .collect();
        took.sort_unstable();
        eprintln!(
            "HeaderShape::of {what} ({} bytes) ns, n=200: p50 {} p99 {} max {}",
            line.len(),
            took[100],
            took[197],
            took[199]
        );
    }
}
