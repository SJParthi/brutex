#![cfg(test)]
//! Round-3 attack tests of the GDFL data path. Round 2 changed the journal's
//! torn-line rule (D-3169), the filter's claim on undecodable names (D-3167)
//! and the per-entry count (D-3168); this round attacks those changes and
//! what rounds 1 and 2 did not reach: a journal torn more than once, the
//! shape of an undecodable name, a columnar tick-store block's own size,
//! random index worlds, filtered stock worlds, and a run of filters over one
//! store. Every value is invented at run time; no vendor row is quoted
//! (`gdfl_cm::tests::fixtures_are_built_not_pasted`). Every property draws
//! from a fixed-seed splitmix64, so a rerun is the same run (`CLAUDE.md` §3
//! rule 5).
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail; the reference indexes on purpose"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_archive::{Archive, MONTHS, day_zip_name};
use crate::gdfl_fixtures::{Method, bts, csv, hms, put, scratch, zip};
use crate::gdfl_nfo::NfoTickStore;
use crate::gdfl_tickstore::{IndexEntry, TickStore, rebuild};

// ── shared helpers ─────────────────────────────────────────────────────────

/// splitmix64.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn upto(&mut self, n: usize) -> usize {
        usize::try_from(self.below(n as u64)).unwrap()
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.upto(from.len())]
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn ddmmyyyy(on: Day) -> String {
    format!("{:02}{:02}{:04}", on.day(), on.month(), on.year())
}

fn month_name(on: Day) -> &'static str {
    MONTHS[usize::from(on.month()) - 1]
}

const OPEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

/// Days of April 2024 a world may use; each is asserted a regular session.
const APRIL: [u8; 8] = [1, 2, 3, 4, 5, 8, 9, 10];

fn april_is_regular() {
    for &d in &APRIL {
        assert!(
            crate::gdfl_cm::session_gate(day(2024, 4, d)).is_ok(),
            "2024-04-{d:02} is a regular session"
        );
    }
}

/// The UTC microsecond of second `sod` of IST day `on`, written here.
fn utc_micros(on: Day, sod: u32) -> i64 {
    (i64::from(on.days_from_epoch()) * 86_400 + i64::from(sod) - 19_800) * 1_000_000
}

fn price_text(paisa: i64) -> String {
    format!("{}.{:02}", paisa / 100, paisa % 100)
}

/// One invented row.
#[derive(Clone, Debug)]
struct Row {
    sod: u32,
    ltp: i64,
    ltp_text: String,
    ltq: u64,
    oi: u64,
}

fn line(stem: &str, on: Day, r: &Row) -> String {
    format!(
        "{stem},{:02}/{:02}/{:04},{},{},0,0,0,0,{},{}",
        on.day(),
        on.month(),
        on.year(),
        hms(r.sod),
        r.ltp_text,
        r.ltq,
        r.oi
    )
}

/// Random rows: repeats, steps, back-steps, forward spikes, untraded rows,
/// stamps either side of the session; `overflow` adds two quantities whose
/// sum (or one alone) is past `i64::MAX`; `oi` keeps or zeroes the open
/// interest.
fn random_rows(rng: &mut Rng, overflow: bool, oi: bool) -> Vec<Row> {
    let n = rng.below(40);
    let mut sod = OPEN - 120 + u32::try_from(rng.below(400)).unwrap();
    let mut out = Vec::new();
    for _ in 0..n {
        match rng.below(100) {
            0..=34 => {}
            35..=64 => sod += u32::try_from(1 + rng.below(3)).unwrap(),
            65..=82 => sod = sod.saturating_sub(u32::try_from(1 + rng.below(6)).unwrap()),
            83..=90 => sod += u32::try_from(60 + rng.below(9_000)).unwrap(),
            _ => sod = sod.saturating_sub(u32::try_from(rng.below(90)).unwrap()),
        }
        sod = sod.min(86_399);
        let ltq = if rng.below(3) == 0 {
            0
        } else {
            1 + rng.below(5_000)
        };
        let ltp = 5 * (1 + i64::try_from(rng.below(200_000)).unwrap());
        out.push(Row {
            sod,
            ltp,
            ltp_text: price_text(ltp),
            ltq,
            oi: if oi { rng.below(10_000_000) } else { 0 },
        });
    }
    if overflow {
        let at = OPEN + 600 + u32::try_from(rng.below(600)).unwrap();
        let big = u64::try_from(i64::MAX).unwrap() - rng.below(2);
        for ltq in [big, 2] {
            out.push(Row {
                sod: at,
                ltp: 500,
                ltp_text: price_text(500),
                ltq,
                oi: 1,
            });
        }
        out.push(Row {
            sod: 86_399,
            ltp: 500,
            ltp_text: price_text(500),
            ltq: 1,
            oi: 1,
        });
    }
    out
}

/// What the reference expects one file to give.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Expect {
    rows: usize,
    zero: usize,
    late: usize,
    unresolved: usize,
    back: u32,
    bars: Vec<Bar>,
}

/// The agreed bar, the slow way (as round 2's reference, written again
/// here): the running maximum over every row before row `i`, a late kept row
/// placed at the next in-order row's stamp, one bar per placed second.
/// `None` when a placed quantity or a second's sum does not fit an `i64`.
fn reference(on: Day, rows: &[Row], every_row: bool, oi: bool) -> Option<Expect> {
    let n = rows.len();
    let in_order = |i: usize| (0..i).all(|j| rows[j].sod <= rows[i].sod);
    let mut out = Expect {
        rows: n,
        ..Expect::default()
    };
    let mut placed: Vec<(u32, &Row)> = Vec::new();
    for (i, r) in rows.iter().enumerate() {
        if !every_row && r.ltq == 0 {
            out.zero += 1;
            continue;
        }
        if in_order(i) {
            placed.push((r.sod, r));
            continue;
        }
        let high = (0..i).map(|j| rows[j].sod).max().unwrap();
        out.late += 1;
        out.back = out.back.max(high - r.sod);
        let mut next = None;
        for j in i + 1..n {
            if in_order(j) {
                next = Some(j);
                break;
            }
        }
        match next {
            Some(j) => placed.push((rows[j].sod, r)),
            None => out.unresolved += 1,
        }
    }
    let mut sums: Vec<u128> = Vec::new();
    for (second, r) in placed {
        if !every_row && i64::try_from(r.ltq).is_err() {
            return None;
        }
        let volume = if every_row { 0 } else { u128::from(r.ltq) };
        let oi = if oi {
            i64::try_from(r.oi).unwrap()
        } else {
            OI_NULL
        };
        let ts = utc_micros(on, second);
        match out.bars.last_mut() {
            Some(bar) if bar.ts_micros == ts => {
                bar.high = bar.high.max(r.ltp);
                bar.low = bar.low.min(r.ltp);
                bar.close = r.ltp;
                bar.open_interest = oi;
                *sums.last_mut().unwrap() += volume;
            }
            _ => {
                out.bars.push(Bar {
                    ts_micros: ts,
                    open: r.ltp,
                    high: r.ltp,
                    low: r.ltp,
                    close: r.ltp,
                    volume: 0,
                    open_interest: oi,
                });
                sums.push(volume);
            }
        }
    }
    for (bar, sum) in out.bars.iter_mut().zip(sums) {
        bar.volume = i64::try_from(sum).ok()?;
    }
    Some(out)
}

fn in_session(on: Day, bar: &Bar) -> bool {
    bar.ts_micros >= utc_micros(on, OPEN) && bar.ts_micros < utc_micros(on, CLOSE)
}

/// Every file under `dir` whose relative path starts with `under`.
fn tree(dir: &Path, under: &str) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&at) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(dir).unwrap().to_path_buf();
                if rel.starts_with(under) {
                    out.insert(rel, std::fs::read(&path).unwrap());
                }
            }
        }
    }
    out
}

/// Every one-second bar of one instrument in April 2024, or `None`.
fn held(store: &Path, segment: &str, symbol: &str, contract: Option<Contract>) -> Option<Vec<Bar>> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract,
        timeframe: Timeframe::SECOND_1,
        month: day(2024, 4, 1).year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let file = BarFile::open_existing(store, path, id).ok()?;
    Some(
        (0..file.header().n_valid)
            .map(|i| file.read_record(i).unwrap())
            .collect(),
    )
}

/// The totals a report should carry.
#[derive(Debug, Default, PartialEq, Eq)]
struct Totals {
    days_imported: usize,
    files: usize,
    files_skipped: usize,
    files_refused: usize,
    rows: usize,
    ltq_zero_dropped: usize,
    late_rows: usize,
    late_unresolved: usize,
    max_back_s: u32,
    seconds: usize,
    outside_session: usize,
}

fn totals_of(report: &Report) -> Totals {
    Totals {
        days_imported: report.days_imported,
        files: report.files,
        files_skipped: report.files_skipped,
        files_refused: report.files_refused,
        rows: report.rows,
        ltq_zero_dropped: report.ltq_zero_dropped,
        late_rows: report.late_rows,
        late_unresolved: report.late_unresolved,
        max_back_s: report.max_back_s,
        seconds: report.seconds,
        outside_session: report.outside_session,
    }
}

impl Totals {
    fn file(&mut self, want: &Expect, on: Day) {
        self.files += 1;
        self.rows += want.rows;
        self.ltq_zero_dropped += want.zero;
        self.late_rows += want.late;
        self.late_unresolved += want.unresolved;
        self.max_back_s = self.max_back_s.max(want.back);
        let kept = want.bars.iter().filter(|b| in_session(on, b)).count();
        self.seconds += kept;
        self.outside_session += want.bars.len() - kept;
    }
}

/// The failures of a report as `(instrument, why)`, sorted.
fn failures_of(report: &Report) -> Vec<(String, String)> {
    let mut got: Vec<(String, String)> = report
        .failures
        .iter()
        .map(|f| (f.instrument.clone(), f.why.clone()))
        .collect();
    got.sort();
    got
}

/// Every reported failure is the expected one, by name and by the words its
/// reason must hold.
fn same_failures(ctx: &str, got: &[(String, String)], want: &mut [(String, &'static str)]) {
    want.sort_unstable();
    assert_eq!(got.len(), want.len(), "{ctx}: {got:?} vs {want:?}");
    for ((name, why), (want_name, want_why)) in got.iter().zip(want.iter()) {
        // A capital-market failure names the file, its extension included.
        assert!(
            name == want_name || name.starts_with(&format!("{want_name}.")),
            "{ctx}: {name} vs {want_name}"
        );
        assert!(why.contains(want_why), "{ctx}: {why} lacks {want_why:?}");
    }
}

fn truncate(path: &Path, len: u64) {
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_len(len)
        .unwrap();
}

fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).map_or(0, |m| m.len())
}

// ── 1. the journal torn more than once ─────────────────────────────────────

fn run_stocks_on(store: &Path) -> Result<Report, ImportRefusal> {
    let run = Run {
        kind: ImportKind::Stocks,
        from: day(2024, 4, 1),
        to: day(2024, 4, 1),
        only: &[],
        store_root: store,
    };
    drive(&run, |_, _| DayRead {
        held: true,
        ..DayRead::default()
    })
}

/// A crash tears a line to `d`; the next load closes it with ` (torn)`, and
/// a second crash cuts that close short, leaving `d (t`. Neither is foreign:
/// each byte is one the journal itself wrote. Before the fix the tail was
/// judged foreign, so every later run was refused for ever (D-3196), the
/// very thing D-3173 exists to prevent.
#[test]
fn r3_01_a_journal_whose_torn_close_was_itself_torn_never_stops_a_later_run() {
    // `definition=` names the bar definition that built the day (D-3191).
    let base = b"done stocks 2024-04-01 * definition=2 files=0\n";
    for tail in [
        &b"d (t"[..],
        b"d (",
        b"d ",
        b"do (torn)",
        b"b (to",
        b"inc (torn",
        b"incomplete (",
        b"d (torn) (to",
        b"be (t (torn)",
    ] {
        let root = scratch("r3-journal-hand");
        let mut text = base.to_vec();
        text.extend_from_slice(tail);
        put(&root, "imports/gdfl.journal", &text);
        for attempt in 0..3 {
            let got = run_stocks_on(&root);
            assert_eq!(
                got.as_ref().map(|r| r.days_skipped),
                Ok(1),
                "{:?}, run {attempt}: {got:?}",
                String::from_utf8_lossy(tail)
            );
        }
        let mut want = text.clone();
        want.extend_from_slice(b" (torn)\n");
        assert_eq!(
            std::fs::read(journal_path(&root)).unwrap(),
            want,
            "{:?}: closed exactly once",
            String::from_utf8_lossy(tail)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    // Pieces of the mark with nothing the journal writes before them, and
    // foreign text a piece follows, stay foreign: refused, never written.
    for tail in [
        &b" "[..],
        b"  ",
        b" (t",
        b"x (torn",
        b"hello (",
        b" (torn)\n",
        b"x (torn)\n",
        b"(torn)",
    ] {
        let root = scratch("r3-journal-hand");
        let mut text = base.to_vec();
        text.extend_from_slice(tail);
        put(&root, "imports/gdfl.journal", &text);
        for attempt in 0..2 {
            let got = run_stocks_on(&root);
            assert!(
                matches!(got, Err(ImportRefusal::Journal { .. })),
                "{:?}, run {attempt}: {got:?}",
                String::from_utf8_lossy(tail)
            );
            assert_eq!(std::fs::read(journal_path(&root)).unwrap(), text);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// 1,500 random journal histories: runs append `begin`, `done` and
/// `incomplete` lines through `Journal::append`, a crash cuts any write
/// (a line, or the ` (torn)` close a load makes) to any strict prefix, and
/// the next run loads what is left. Every load succeeds, and its done and
/// open sets are exactly those of the lines that were written whole; a load
/// after a load writes nothing.
#[test]
fn r3_02_random_crash_histories_always_load_to_the_lines_written_whole() {
    let mut rng = Rng(0x7233_4a4f_5552_4e4c);
    let kinds = ["indices", "stocks", "options"];
    let days = ["2024-04-01", "2024-04-02"];
    let filters = ["*", "NIFTY", "BANKNIFTY,NIFTY"];
    let (mut loads, mut line_tears, mut close_tears) = (0_usize, 0_usize, 0_usize);
    for case in 0..1_500 {
        let root = scratch("r3-journal-fuzz");
        let path = journal_path(&root);
        let (mut done, mut open) = (BTreeSet::new(), BTreeSet::new());
        let mut history: Vec<String> = Vec::new();
        for step in 0..12 {
            let before = file_len(&path);
            let got = Journal::load(path.clone());
            loads += 1;
            let ctx = format!(
                "case {case} step {step} history {history:?} file {:?}",
                String::from_utf8_lossy(&std::fs::read(&path).unwrap_or_default())
            );
            assert!(got.is_ok(), "{ctx}: refused {got:?}");
            let journal = got.unwrap();
            let got_done: BTreeSet<String> = journal.done.iter().cloned().collect();
            let got_open: BTreeSet<String> = journal.open.iter().cloned().collect();
            assert_eq!(got_done, done, "{ctx}");
            assert_eq!(got_open, open, "{ctx}");
            let after = file_len(&path);
            if after > before && rng.below(3) == 0 {
                // A crash while the load closed the torn tail.
                let cut = 1 + rng.below(after - before);
                truncate(&path, after - cut);
                close_tears += 1;
                history.push(format!("close cut {cut}"));
                continue;
            }
            for _ in 0..=rng.below(3) {
                let key = format!(
                    "{} {} {}",
                    rng.pick(&kinds),
                    rng.pick(&days),
                    rng.pick(&filters)
                );
                let verb = rng.below(3);
                let text = match verb {
                    0 => format!("begin {key}"),
                    1 => format!(
                        "done {key} definition={BAR_DEFINITION} files=1 seconds=2 failures=0"
                    ),
                    _ => format!(
                        "incomplete {key} definition={BAR_DEFINITION} files=1 seconds=0 failures=1"
                    ),
                };
                let at = file_len(&path);
                journal.append(&text).unwrap();
                if rng.below(4) == 0 {
                    // A crash mid-write: a strict prefix of the line and its
                    // newline, short ones as often as long ones.
                    let whole = text.len() as u64 + 1;
                    let keep = if rng.below(2) == 0 {
                        rng.below(whole.min(12))
                    } else {
                        rng.below(whole)
                    };
                    truncate(&path, at + keep);
                    line_tears += 1;
                    history.push(format!("{text:?} cut to {keep}"));
                    break;
                }
                history.push(format!("{text:?}"));
                match verb {
                    0 => {
                        open.insert(key);
                    }
                    1 => {
                        open.remove(&key);
                        done.insert(key);
                    }
                    _ => {
                        open.remove(&key);
                    }
                }
            }
        }
        // A load after a load writes nothing.
        let first = Journal::load(path.clone()).unwrap();
        let settled = std::fs::read(&path).unwrap_or_default();
        let second = Journal::load(path.clone()).unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap_or_default(),
            settled,
            "case {case}"
        );
        assert_eq!(first.done, second.done, "case {case}");
        std::fs::remove_dir_all(root).unwrap();
    }
    assert!(
        loads > 15_000 && line_tears > 2_000 && close_tears > 500,
        "{loads} loads, {line_tears} line tears, {close_tears} close tears"
    );
}

// ── 2. an undecodable name's own shape names its underlying ────────────────

fn nfo_name(on: Day, ticker: &str) -> String {
    format!("GFDLNFO_TICK_{}\\Options\\{ticker}.NFO.csv", ddmmyyyy(on))
}

/// `LTI` and `NIFTYIT` are underlyings no longer in the F&O list, each
/// beginning with one that is (`LT`, `NIFTY`). A name of theirs that does not
/// decode (a Saturday expiry) is theirs by its own shape: an `LT` or a
/// `NIFTY` run must skip it, never claim it as its own broken file. Before
/// the fix the longest-prefix rule read `LTI06APR24…` as `LT`'s (D-3197);
/// DPT-07 says an undecodable name belongs to its WHOLE underlying.
#[test]
fn r3_03_an_undecodable_name_belongs_to_the_underlying_its_shape_spells() {
    let on = day(2024, 4, 1);
    let tickers = [
        "LTI06APR24100CE",
        "NIFTYIT06APR24100CE",
        "LT06APR24100CE",
        "NIFTY06APR24100PE",
        "NIFTYJUNK",
    ];
    let files: Vec<(String, Vec<u8>)> = tickers
        .iter()
        .map(|t| {
            let r = Row {
                sod: OPEN,
                ltp: 5,
                ltp_text: price_text(5),
                ltq: 1,
                oi: 0,
            };
            (nfo_name(on, t), csv(&[line(&format!("{t}.NFO"), on, &r)]))
        })
        .collect();
    for t in tickers {
        assert!(decode_ticker(t, on).is_err(), "{t} does not decode");
    }
    let refs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let root = scratch("r3-shape");
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&refs),
    );
    let source = NfoTickStore::new(&root.join("ts"));
    for (only, refused) in [
        (vec!["LT"], vec!["LT06APR24100CE"]),
        (vec!["LTI"], vec!["LTI06APR24100CE"]),
        (vec!["NIFTY"], vec!["NIFTY06APR24100PE", "NIFTYJUNK"]),
        (vec!["NIFTYIT"], vec!["NIFTYIT06APR24100CE"]),
        (
            vec!["LT", "NIFTYIT"],
            vec!["LT06APR24100CE", "NIFTYIT06APR24100CE"],
        ),
        (Vec::new(), tickers.to_vec()),
    ] {
        let only: Vec<String> = only.iter().map(|s| (*s).to_owned()).collect();
        let store = scratch("r3-shape-store");
        let run = Run {
            kind: ImportKind::Options,
            from: on,
            to: on,
            only: &only,
            store_root: &store,
        };
        let got = run_nfo(&source, &run).unwrap();
        let mut names: Vec<String> = got
            .failures
            .iter()
            .map(|f| f.instrument.trim_start_matches("2024-04-01 ").to_owned())
            .collect();
        names.sort();
        let mut want: Vec<String> = refused.iter().map(|s| (*s).to_owned()).collect();
        want.sort();
        assert_eq!(names, want, "{only:?}: {got:?}");
        assert_eq!(
            (got.files_refused, got.files_skipped, got.files),
            (want.len(), tickers.len() - want.len(), 0),
            "{only:?}"
        );
        std::fs::remove_dir_all(store).unwrap();
    }
    std::fs::remove_dir_all(root).unwrap();
}

// ── 3. a columnar block is held to its stated size ─────────────────────────

fn zstd(bytes: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(bytes, ruzstd::encoding::CompressionLevel::Fastest)
}

/// A columnar (kind 1) block of `header` and `cols`, each `(tag, payload,
/// text_len)`, stated to hold `rows` rows.
fn columnar_block(header: &str, rows: u64, cols: &[(u8, Vec<u8>, u64)]) -> Vec<u8> {
    let mut out = vec![1, 0, 1];
    out.extend(u32::try_from(header.len()).unwrap().to_le_bytes());
    out.extend(header.as_bytes());
    out.extend(u32::try_from(cols.len()).unwrap().to_le_bytes());
    out.extend(rows.to_le_bytes());
    for (tag, payload, text_len) in cols {
        let frame = zstd(payload);
        out.push(*tag);
        out.extend((payload.len() as u64).to_le_bytes());
        out.extend(text_len.to_le_bytes());
        out.extend((frame.len() as u64).to_le_bytes());
        out.extend(frame);
    }
    out
}

/// The entry of `block` placed at offset 8 of `day`, stating `size` and
/// `rows`.
fn entry_of(block: &[u8], size: u64, rows: u64, crc: u32) -> (Vec<u8>, IndexEntry) {
    let mut day = b"BRTXTS01".to_vec();
    day.extend_from_slice(block);
    let entry = IndexEntry {
        name: "GFDLCM_STOCK_TICK_01042024/RELIANCE.NSE.csv".to_owned(),
        kind: 1,
        off: 8,
        len: block.len() as u64,
        size,
        crc,
        rows,
        dos_time: 0,
    };
    (day, entry)
}

/// A raw block stops at its stated size (one byte past it, round 1); a
/// columnar block did not: its rows, its columns and their texts were
/// rebuilt whatever the entry stated, and only `verify`, AFTER the rebuild,
/// compared the length. A block of a few kilobytes stating 64 bytes built
/// tens of megabytes first (D-3198). Every block below states a size far
/// smaller than what it would rebuild, and must be refused before it is
/// built; a block that rebuilds to exactly its stated size still rebuilds.
#[test]
fn r3_04_a_columnar_block_never_builds_past_its_stated_size() {
    // One numeric column of a million rows, each rendering as seventeen
    // bytes (`0.000000000000000`) from one shape byte and no plane bytes.
    let rows = 1_000_000_u64;
    let mut payload = vec![0x0F_u8; usize::try_from(rows).unwrap()];
    payload.extend([0, 0]);
    let text_len = rows * 17 + rows - 1;
    let block = columnar_block("a", rows, &[(1, payload, text_len)]);
    let (day, entry) = entry_of(&block, 64, rows, 0);
    let got = rebuild(&day, &entry);
    assert!(
        got.is_err(),
        "a {}-byte block stating 64 bytes rebuilt {:?} bytes",
        block.len(),
        got.as_ref().map(Vec::len)
    );
    // Many text columns of empty fields: the rows times the columns, from a
    // block whose every column is a run of newlines.
    let (cols, rows) = (1_500_usize, 1_500_u64);
    let header = vec!["c"; cols].join(",");
    let newlines = vec![b'\n'; usize::try_from(rows - 1).unwrap()];
    let columns: Vec<(u8, Vec<u8>, u64)> =
        (0..cols).map(|_| (0, newlines.clone(), rows - 1)).collect();
    let block = columnar_block(&header, rows, &columns);
    let (day, entry) = entry_of(&block, 64, rows, 0);
    let got = rebuild(&day, &entry);
    assert!(
        got.is_err(),
        "a {}-byte block stating 64 bytes rebuilt {:?} bytes",
        block.len(),
        got.as_ref().map(Vec::len)
    );
    // An honest block: rebuilt exactly, at exactly its size.
    let fields = [vec!["X", "Y", "Z"], vec!["1", "22", "333"]];
    let file = b"h1,h2\nX,1\nY,22\nZ,333\n".to_vec();
    let cols: Vec<(u8, Vec<u8>, u64)> = fields
        .iter()
        .map(|f| {
            let text = f.join("\n");
            let len = text.len() as u64;
            (0, text.into_bytes(), len)
        })
        .collect();
    let block = columnar_block("h1,h2", 3, &cols);
    let crc = crate::gdfl_archive::crc32(&file);
    let (day, entry) = entry_of(&block, file.len() as u64, 3, crc);
    assert_eq!(rebuild(&day, &entry).unwrap(), file);
    // One byte short of its size it is refused, one byte long it is too.
    for size in [file.len() as u64 - 1, file.len() as u64 + 1] {
        let (day, entry) = entry_of(&block, size, 3, crc);
        let got = rebuild(&day, &entry);
        let rebuilt_whole = got.as_ref().is_ok_and(|b| *b == file);
        assert!(
            got.is_err() || rebuilt_whole,
            "size {size}: rebuilt {:?}",
            got.map(|b| b.len())
        );
    }
}

// ── 4. random capital-market worlds: indices, and stocks under filters ─────

/// One entry of a capital-market day.
#[derive(Clone, Debug)]
enum Cm {
    /// A swept instrument's file: its file stem's name part, its rows, the
    /// `.CSV` spelling, and whether one row is bent.
    File {
        name: &'static str,
        rows: Vec<Row>,
        upper: bool,
        bent: bool,
    },
    /// A file of the tree that is not swept.
    Unswept { name: &'static str },
    /// A member outside the day folder.
    Stray,
}

const INDEX_NAMES: [(&str, &str); 2] = [("NIFTY 50", "NIFTY"), ("NIFTY BANK", "BANKNIFTY")];
const INDEX_OTHERS: [&str; 2] = ["INDIA VIX", "NIFTY IT"];
const SHARES: [&str; 5] = ["RELIANCE", "SBIN", "M&M", "BAJAJ-AUTO", "360ONE"];
const SHARE_OTHERS: [&str; 2] = ["RELIANCE.BE", "SBIN.SM"];

fn suffix(kind: CmKind) -> &'static str {
    match kind {
        CmKind::Indices => ".NSE_IDX",
        CmKind::Stocks => ".NSE",
    }
}

fn cm_bytes(kind: CmKind, on: Day, folder: &str, entry: &Cm) -> (String, Vec<u8>) {
    let one = Row {
        sod: OPEN,
        ltp: 5,
        ltp_text: price_text(5),
        ltq: 1,
        oi: 0,
    };
    match entry {
        Cm::File {
            name,
            rows,
            upper,
            bent,
        } => {
            let stem = format!("{name}{}", suffix(kind));
            let mut lines: Vec<String> = rows.iter().map(|r| line(&stem, on, r)).collect();
            if *bent {
                let mut r = one;
                r.ltp_text = "0.051".to_owned();
                lines.insert(0, line(&stem, on, &r));
            }
            (
                format!("{folder}/{stem}.{}", if *upper { "CSV" } else { "csv" }),
                csv(&lines),
            )
        }
        Cm::Unswept { name } => {
            let stem = format!("{name}{}", suffix(kind));
            (
                format!("{folder}/{stem}.csv"),
                csv(&[line(&stem, on, &one)]),
            )
        }
        Cm::Stray => (format!("{folder}.txt"), Vec::new()),
    }
}

/// The symbol a swept name is filed under.
fn symbol_static(kind: CmKind, name: &'static str) -> &'static str {
    match kind {
        CmKind::Indices => {
            for (vendor, symbol) in INDEX_NAMES {
                if vendor == name {
                    return symbol;
                }
            }
            unreachable!("only swept index names are drawn")
        }
        CmKind::Stocks => name,
    }
}

/// Rows for a capital-market file of `kind`: an index file is closed by a
/// row at or after 15:30 three times in four, so both its refusals and its
/// files are reached.
fn cm_rows(rng: &mut Rng, kind: CmKind, overflow: bool) -> Vec<Row> {
    let mut rows = random_rows(rng, overflow && kind == CmKind::Stocks, false);
    if kind == CmKind::Indices && rng.below(4) != 0 {
        let ltp = 5 * (1 + i64::try_from(rng.below(1_000)).unwrap());
        rows.push(Row {
            sod: CLOSE + u32::try_from(rng.below(600)).unwrap(),
            ltp,
            ltp_text: price_text(ltp),
            ltq: rng.below(3),
            oi: 0,
        });
    }
    rows
}

fn random_cm_world(rng: &mut Rng, kind: CmKind) -> Vec<(Day, Vec<Cm>)> {
    let span = 1 + rng.upto(3);
    let first = rng.upto(APRIL.len() - span + 1);
    let mut world = Vec::new();
    for &d in &APRIL[first..first + span] {
        let mut entries = Vec::new();
        for _ in 0..rng.below(7) {
            let swept = |rng: &mut Rng| match kind {
                CmKind::Indices => rng.pick(&INDEX_NAMES).0,
                CmKind::Stocks => rng.pick(&SHARES),
            };
            entries.push(match rng.below(12) {
                0..=6 => Cm::File {
                    name: swept(rng),
                    rows: cm_rows(rng, kind, false),
                    upper: rng.below(5) == 0,
                    bent: false,
                },
                7 => Cm::File {
                    name: swept(rng),
                    rows: cm_rows(rng, kind, true),
                    upper: false,
                    bent: false,
                },
                8 => Cm::File {
                    name: swept(rng),
                    rows: cm_rows(rng, kind, false),
                    upper: false,
                    bent: true,
                },
                9 | 10 => Cm::Unswept {
                    name: match kind {
                        CmKind::Indices => rng.pick(&INDEX_OTHERS),
                        CmKind::Stocks => rng.pick(&SHARE_OTHERS),
                    },
                },
                _ => Cm::Stray,
            });
        }
        world.push((day(2024, 4, d), entries));
    }
    world
}

/// Writes a capital-market world as a tick store under `root/ts` and as one
/// outer archive `root/zips/cm.zip`.
fn write_cm_world(root: &Path, kind: CmKind, world: &[(Day, Vec<Cm>)]) {
    let mut outer: Vec<(String, Vec<u8>)> = Vec::new();
    for (on, entries) in world {
        let folder = crate::gdfl_cm::day_folder_name(kind, *on);
        let files: Vec<(String, Vec<u8>)> = entries
            .iter()
            .map(|e| cm_bytes(kind, *on, &folder, e))
            .collect();
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let zip_name = day_zip_name(kind, *on);
        let stem = zip_name.strip_suffix(".zip").unwrap();
        put(root, &format!("ts/cm/{stem}.bts"), &bts(&refs));
        let members: Vec<(&str, &[u8], Method)> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
            .collect();
        outer.push((zip_name, zip(&members)));
    }
    let members: Vec<(&str, &[u8], Method)> = outer
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(root, "zips/cm.zip", &zip(&members));
}

/// What the reference expects of a capital-market world under `only`.
struct WantCm {
    totals: Totals,
    failures: Vec<(String, &'static str)>,
    series: BTreeMap<&'static str, Vec<Bar>>,
    clean_days: usize,
}

/// The reference: per day, each swept ticker's FIRST entry is the one the
/// day offers, a later name of it a skip; a ticker the filter does not want
/// is a skip; two names of one ticker are twins, refused once (D-3174); a
/// bent row refuses at line 2; a stock with no traded in-session row, or an
/// index with no in-session row or none at or after 15:30, refuses; a
/// quantity past `i64` refuses.
fn want_cm(kind: CmKind, world: &[(Day, Vec<Cm>)], only: &[&str]) -> WantCm {
    let mut want = WantCm {
        totals: Totals::default(),
        failures: Vec::new(),
        series: BTreeMap::new(),
        clean_days: 0,
    };
    let every_row = kind == CmKind::Indices;
    for (on, entries) in world {
        want.totals.days_imported += 1;
        let before = want.failures.len();
        let mut count: BTreeMap<&str, usize> = BTreeMap::new();
        for e in entries {
            if let Cm::File { name, .. } = e {
                *count.entry(*name).or_default() += 1;
            }
        }
        let mut seen: Vec<&str> = Vec::new();
        let mut files: Vec<(&'static str, Expect)> = Vec::new();
        let mut late: Vec<(String, &'static str)> = Vec::new();
        for e in entries {
            let Cm::File {
                name, rows, bent, ..
            } = e
            else {
                want.totals.files_skipped += 1;
                continue;
            };
            if seen.contains(name) {
                want.totals.files_skipped += 1;
                continue;
            }
            seen.push(name);
            let symbol = symbol_static(kind, name);
            if !(only.is_empty() || only.contains(&symbol)) {
                want.totals.files_skipped += 1;
                continue;
            }
            let file = format!("{on} {name}{}", suffix(kind));
            let usable = rows
                .iter()
                .any(|r| (every_row || r.ltq > 0) && (OPEN..CLOSE).contains(&r.sod));
            let closed = rows.iter().any(|r| r.sod >= CLOSE);
            if count[name] > 1 {
                want.totals.files_refused += 1;
                late.push((file, "two names for one ticker"));
            } else if *bent {
                want.totals.files_refused += 1;
                late.push((file, "line 2"));
            } else if !usable {
                want.totals.files_refused += 1;
                late.push((file, "no usable row"));
            } else if every_row && !closed {
                want.totals.files_refused += 1;
                late.push((file, "ends inside the session"));
            } else if let Some(expect) = reference(*on, rows, every_row, false) {
                files.push((symbol, expect));
            } else {
                want.totals.files_refused += 1;
                want.failures.push((file, "does not fit an i64"));
            }
        }
        for (symbol, expect) in files {
            want.totals.file(&expect, *on);
            want.series
                .entry(symbol)
                .or_default()
                .extend(expect.bars.into_iter().filter(|b| in_session(*on, b)));
        }
        want.failures.extend(late);
        if want.failures.len() == before {
            want.clean_days += 1;
        }
    }
    want
}

fn run_cm_world(
    root: &Path,
    kind: CmKind,
    world: &[(Day, Vec<Cm>)],
    zips: bool,
    store: &Path,
    only: &[String],
) -> Report {
    let run = Run {
        kind: match kind {
            CmKind::Indices => ImportKind::Indices,
            CmKind::Stocks => ImportKind::Stocks,
        },
        from: world[0].0,
        to: world[world.len() - 1].0,
        only,
        store_root: store,
    };
    if zips {
        run_cm(&Archive::open(&root.join("zips/cm.zip")).unwrap(), &run).unwrap()
    } else {
        run_cm(&TickStore::new(&root.join("ts")), &run).unwrap()
    }
}

/// Random index and filtered stock worlds through both sources: both report
/// alike and leave byte-identical stores; every total, named failure and
/// stored bar equals the reference; every entry is one file, skip or
/// refusal; a rerun skips exactly the clean days and writes no bar.
fn cm_worlds(kind: CmKind, seed: u64, cases: usize) -> (usize, usize, usize) {
    april_is_regular();
    let mut rng = Rng(seed);
    let (mut bars_checked, mut refusals, mut filtered) = (0_usize, 0_usize, 0_usize);
    let names: Vec<&str> = match kind {
        CmKind::Indices => INDEX_NAMES.iter().map(|(_, s)| *s).collect(),
        CmKind::Stocks => SHARES.to_vec(),
    };
    for case in 0..cases {
        let world = random_cm_world(&mut rng, kind);
        let only: Vec<&str> = if rng.below(2) == 0 {
            filtered += 1;
            let mut only = vec![rng.pick(&names)];
            if rng.below(3) == 0 {
                only.push(rng.pick(&names));
            }
            only
        } else {
            Vec::new()
        };
        let owned: Vec<String> = only.iter().map(|s| (*s).to_owned()).collect();
        let root = scratch("r3-cm-world");
        write_cm_world(&root, kind, &world);
        let (a, b) = (root.join("A"), root.join("B"));
        let ra = run_cm_world(&root, kind, &world, false, &a, &owned);
        let rb = run_cm_world(&root, kind, &world, true, &b, &owned);
        let ctx = format!("{kind:?} case {case} only {only:?} world {world:?}");
        assert_eq!(ra, rb, "{ctx}: the two sources report one run");
        assert_eq!(tree(&a, ""), tree(&b, ""), "{ctx}: one store");
        let mut want = want_cm(kind, &world, &only);
        assert_eq!(totals_of(&ra), want.totals, "{ctx}: {ra:?}");
        let entries: usize = world.iter().map(|(_, e)| e.len()).sum();
        assert_eq!(
            ra.files + ra.files_skipped + ra.files_refused,
            entries,
            "{ctx}"
        );
        let got = failures_of(&ra);
        refusals += got.len();
        same_failures(&ctx, &got, &mut want.failures);
        let segment = match kind {
            CmKind::Indices => "INDEX",
            CmKind::Stocks => "CASH",
        };
        for (symbol, bars) in &want.series {
            let got = held(&a, segment, symbol, None);
            if bars.is_empty() {
                assert!(got.is_none(), "{ctx}: {symbol}");
            } else {
                assert_eq!(got.as_ref(), Some(bars), "{ctx}: {symbol}");
                bars_checked += bars.len();
            }
        }
        let before = tree(&a, "bars");
        let again = run_cm_world(&root, kind, &world, false, &a, &owned);
        assert_eq!(again.days_skipped, want.clean_days, "{ctx}: {again:?}");
        assert_eq!(again.seconds_committed, 0, "{ctx}");
        assert_eq!(before, tree(&a, "bars"), "{ctx}");
        std::fs::remove_dir_all(root).unwrap();
    }
    (bars_checked, refusals, filtered)
}

#[test]
fn r3_05_random_index_worlds_match_the_reference_through_both_sources() {
    let (bars, refusals, filtered) = cm_worlds(CmKind::Indices, 0x7233_494e_4458, 400);
    assert!(
        bars > 1_000 && refusals > 300 && filtered > 150,
        "{bars} bars, {refusals} refusals, {filtered} filtered"
    );
}

#[test]
fn r3_06_random_filtered_stock_worlds_match_the_reference_through_both_sources() {
    let (bars, refusals, filtered) = cm_worlds(CmKind::Stocks, 0x7233_5354_4b46, 400);
    assert!(
        bars > 1_000 && refusals > 300 && filtered > 150,
        "{bars} bars, {refusals} refusals, {filtered} filtered"
    );
}

// ── 5. options: a run of filters over one store converges ──────────────────

const UNDERS: [&str; 5] = ["NIFTY", "NIFTYNXT50", "M&M", "LT", "LTI"];

#[derive(Clone, Debug)]
enum Nfo {
    /// A dated option file of the day.
    File {
        under: &'static str,
        expiry: u8,
        call: bool,
        rows: Vec<Row>,
        bent: bool,
        upper: bool,
    },
    /// A name that does not decode: a Saturday expiry.
    Broken { under: &'static str },
    /// Not an option file.
    Other,
}

fn nfo_ticker(under: &str, expiry: u8, call: bool) -> String {
    format!(
        "{under}{expiry:02}APR24100{}",
        if call { "CE" } else { "PE" }
    )
}

fn nfo_bytes(on: Day, entry: &Nfo) -> (String, Vec<u8>) {
    match entry {
        Nfo::File {
            under,
            expiry,
            call,
            rows,
            bent,
            upper,
        } => {
            let t = nfo_ticker(under, *expiry, *call);
            let stem = format!("{t}.NFO");
            let mut lines: Vec<String> = rows.iter().map(|r| line(&stem, on, r)).collect();
            if *bent {
                let mut r = rows.first().cloned().unwrap_or(Row {
                    sod: OPEN,
                    ltp: 5,
                    ltp_text: String::new(),
                    ltq: 1,
                    oi: 0,
                });
                r.ltp_text = format!("{}1", price_text(r.ltp));
                lines.push(line(&stem, on, &r));
            }
            let mut name = nfo_name(on, &t);
            if *upper {
                name = name.replace(".NFO.csv", ".NFO.CSV");
            }
            (name, csv(&lines))
        }
        Nfo::Broken { under } => {
            let t = format!("{under}06APR24100CE");
            let r = Row {
                sod: OPEN,
                ltp: 5,
                ltp_text: price_text(5),
                ltq: 1,
                oi: 0,
            };
            (nfo_name(on, &t), csv(&[line(&format!("{t}.NFO"), on, &r)]))
        }
        Nfo::Other => (
            format!("GFDLNFO_TICK_{}\\Futures\\NIFTY-I.NFO.csv", ddmmyyyy(on)),
            csv(&[]),
        ),
    }
}

fn random_nfo_world(rng: &mut Rng) -> Vec<(Day, Vec<Nfo>)> {
    let span = 1 + rng.upto(3);
    let first = rng.upto(APRIL.len() - span + 1);
    let mut world = Vec::new();
    for &d in &APRIL[first..first + span] {
        let mut entries = Vec::new();
        for _ in 0..rng.below(8) {
            entries.push(match rng.below(14) {
                0..=8 => Nfo::File {
                    under: rng.pick(&UNDERS),
                    expiry: rng.pick(&[11_u8, 18, 25]),
                    call: rng.below(2) == 0,
                    rows: {
                        let overflow = rng.below(10) == 0;
                        random_rows(rng, overflow, true)
                    },
                    bent: rng.below(10) == 0,
                    upper: rng.below(6) == 0,
                },
                9 | 10 => Nfo::Broken {
                    under: rng.pick(&UNDERS),
                },
                _ => Nfo::Other,
            });
        }
        world.push((day(2024, 4, d), entries));
    }
    world
}

fn write_nfo_world(root: &Path, world: &[(Day, Vec<Nfo>)]) {
    for (on, entries) in world {
        let files: Vec<(String, Vec<u8>)> = entries.iter().map(|e| nfo_bytes(*on, e)).collect();
        let refs: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        put(
            root,
            &format!(
                "ts/options/{:04}/{}_{:04}/GFDLNFO_TICK_{}.bts",
                on.year(),
                month_name(*on),
                on.year(),
                ddmmyyyy(*on)
            ),
            &bts(&refs),
        );
    }
}

fn run_nfo_world(root: &Path, world: &[(Day, Vec<Nfo>)], store: &Path, only: &[String]) -> Report {
    let run = Run {
        kind: ImportKind::Options,
        from: world[0].0,
        to: world[world.len() - 1].0,
        only,
        store_root: store,
    };
    run_nfo(&NfoTickStore::new(&root.join("ts")), &run).unwrap()
}

/// 120 random options worlds over underlyings that begin with one another
/// (`NIFTY`/`NIFTYNXT50`, `LT`/`LTI`), with broken names of each: a store
/// built by three filtered runs and then an unfiltered one holds byte for
/// byte the bars of a store built by the unfiltered run alone; across the
/// filtered runs every entry is claimed by at most one filter, and each
/// run's entries are files, skips and refusals exactly; the final rerun
/// skips every clean day and writes nothing.
#[test]
fn r3_07_a_run_of_filters_over_one_store_converges_on_the_unfiltered_store() {
    april_is_regular();
    let mut rng = Rng(0x7233_4649_4c54);
    let mut claimed_total = 0_usize;
    for case in 0..120 {
        let world = random_nfo_world(&mut rng);
        let root = scratch("r3-filters");
        write_nfo_world(&root, &world);
        let entries: usize = world.iter().map(|(_, e)| e.len()).sum();
        let ctx = format!("case {case} world {world:?}");
        let (s1, s2) = (root.join("S1"), root.join("S2"));
        // Every underlying once, in a random order, each its own filter.
        let mut order: Vec<&str> = UNDERS.to_vec();
        for i in (1..order.len()).rev() {
            order.swap(i, rng.upto(i + 1));
        }
        let mut claimed = 0_usize;
        for under in &order {
            let only = vec![(*under).to_owned()];
            let got = run_nfo_world(&root, &world, &s1, &only);
            assert_eq!(
                got.files + got.files_skipped + got.files_refused,
                entries,
                "{ctx} {under}"
            );
            claimed += got.files + got.files_refused;
        }
        let all = run_nfo_world(&root, &world, &s1, &[]);
        assert_eq!(
            all.files + all.files_skipped + all.files_refused,
            entries,
            "{ctx}"
        );
        // Each option or broken file is its one underlying's: the filters
        // between them claim exactly what the unfiltered run does.
        assert_eq!(
            claimed,
            all.files + all.files_refused,
            "{ctx}: the filters claimed {claimed}, the whole run {}",
            all.files + all.files_refused
        );
        claimed_total += claimed;
        let alone = run_nfo_world(&root, &world, &s2, &[]);
        assert_eq!(all.files, alone.files, "{ctx}");
        assert_eq!(all.files_refused, alone.files_refused, "{ctx}");
        assert_eq!(failures_of(&all), failures_of(&alone), "{ctx}");
        assert_eq!(tree(&s1, "bars"), tree(&s2, "bars"), "{ctx}");
        let again = run_nfo_world(&root, &world, &s1, &[]);
        let clean = world.len() - {
            let mut dirty = BTreeSet::new();
            for f in &all.failures {
                dirty.insert(f.instrument.get(..10).unwrap_or_default().to_owned());
            }
            dirty.len()
        };
        assert_eq!(again.days_skipped, clean, "{ctx}: {again:?}");
        assert_eq!(again.seconds_committed, 0, "{ctx}");
        std::fs::remove_dir_all(root).unwrap();
    }
    assert!(claimed_total > 500, "{claimed_total}");
}
