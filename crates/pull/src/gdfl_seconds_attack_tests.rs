#![cfg(test)]
//! Attack tests of the GDFL one-second build, its readers and its runtime
//! (round 1, area `gdfl-seconds`). Every value is invented at run time; no
//! vendor row is quoted (`gdfl_cm::tests::fixtures_are_built_not_pasted`).
//! Property tests draw from a fixed-seed splitmix64, so every rerun is byte
//! for byte the same run (`CLAUDE.md` §3 rule 5).
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::needless_range_loop,
    clippy::naive_bytecount,
    clippy::too_many_lines,
    clippy::float_arithmetic,
    clippy::panic,
    reason = "a test that cannot panic cannot fail; the reference indexes on purpose, slowly"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_archive::{Archive, crc32, member_bytes, zip_entries};
use crate::gdfl_cm::{CmFile, Expect, day_folder_name, decode};
use crate::gdfl_fixtures::{Method, bts, csv, hms, put, row, scratch, zip};
use crate::gdfl_tickstore::{TickStore, read_index, rebuild};

// ── shared helpers ─────────────────────────────────────────────────────────

/// splitmix64, the fixed-seed generator every property here draws from.
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
}

fn d1() -> Day {
    Day::new(2024, 4, 1).unwrap()
}

const T: u32 = 9 * 3_600 + 15 * 60;

fn hm(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

fn tick(sod: u32, ltp: i64, ltq: u64) -> Tick {
    Tick {
        sod,
        ltp,
        ltq,
        oi: None,
    }
}

/// The naive reference: the agreed bar (module doc, D-2802) with D-3170's
/// placement, written the slow way (a quadratic scan per late row, a map
/// per second, an `i128` volume) so it shares no code with `convert`.
fn reference(kind: ImportKind, day: Day, ticks: &[Tick]) -> Result<Converted, ()> {
    let counts = |t: &Tick| kind.every_row_counts() || t.ltq > 0;
    let n = ticks.len();
    // prefix[j] is the largest stamp of rows 0..j, every row counted.
    let mut prefix: Vec<Option<u32>> = vec![None; n + 1];
    for j in 0..n {
        prefix[j + 1] = Some(prefix[j].map_or(ticks[j].sod, |m| m.max(ticks[j].sod)));
    }
    let in_order = |j: usize| prefix[j].is_none_or(|m| ticks[j].sod >= m);
    let mut placement = Placement {
        rows: n,
        ..Placement::default()
    };
    let mut placed: Vec<(u32, Tick)> = Vec::new();
    for i in 0..n {
        let t = ticks[i];
        if !counts(&t) {
            placement.ltq_zero_dropped += 1;
            continue;
        }
        if in_order(i) {
            placed.push((t.sod, t));
            continue;
        }
        let max = prefix[i].unwrap();
        placement.late_rows += 1;
        placement.max_back_s = placement.max_back_s.max(max - t.sod);
        let mut target = None;
        for j in i + 1..n {
            if in_order(j) {
                target = Some(ticks[j].sod);
                break;
            }
        }
        match target {
            Some(s) => placed.push((s, t)),
            None => placement.late_unresolved += 1,
        }
    }
    // No look-ahead: every kept row lands at or after every stamp the file
    // showed before it (D-3170).
    for (k, (s, _)) in placed.iter().enumerate() {
        assert!(k == 0 || placed[k - 1].0 <= *s, "placement runs forward");
    }
    let mut seconds: BTreeMap<u32, (Bar, i128)> = BTreeMap::new();
    for (s, t) in placed {
        if !kind.every_row_counts() && i64::try_from(t.ltq).is_err() {
            return Err(());
        }
        let volume = if kind.every_row_counts() {
            0
        } else {
            i128::from(t.ltq)
        };
        let oi = t.oi.unwrap_or(OI_NULL);
        seconds
            .entry(s)
            .and_modify(|(bar, sum)| {
                bar.high = bar.high.max(t.ltp);
                bar.low = bar.low.min(t.ltp);
                bar.close = t.ltp;
                *sum += volume;
                if oi != OI_NULL {
                    bar.open_interest = oi;
                }
            })
            .or_insert((
                Bar {
                    ts_micros: micros_at(day, s),
                    open: t.ltp,
                    high: t.ltp,
                    low: t.ltp,
                    close: t.ltp,
                    volume: 0,
                    open_interest: oi,
                },
                volume,
            ));
    }
    let mut out = Vec::new();
    for (_, (mut bar, sum)) in seconds {
        bar.volume = i64::try_from(sum).map_err(|_| ())?;
        out.push(bar);
    }
    Ok(Converted {
        seconds: out,
        placement,
    })
}

/// One random file of ticks: mostly forward, with repeats, back-steps,
/// forward spikes, untraded rows and, rarely, quantities at the `i64` edge.
fn random_ticks(rng: &mut Rng, kind: ImportKind) -> Vec<Tick> {
    let n = rng.below(33);
    let mut sod = T + u32::try_from(rng.below(100)).unwrap();
    let mut out = Vec::new();
    for _ in 0..n {
        match rng.below(100) {
            0..=39 => {}
            40..=69 => sod += u32::try_from(1 + rng.below(2)).unwrap(),
            70..=86 => sod = sod.saturating_sub(u32::try_from(1 + rng.below(5)).unwrap()),
            87..=91 => sod += u32::try_from(30 + rng.below(300)).unwrap(),
            _ => sod = sod.saturating_sub(u32::try_from(rng.below(40)).unwrap()),
        }
        let ltq = match rng.below(1_000) {
            0..=349 => 0,
            350..=989 => 1 + rng.below(1_000),
            990..=994 => u64::try_from(i64::MAX).unwrap() - rng.below(3),
            _ => u64::MAX - rng.below(3),
        };
        let oi =
            (kind == ImportKind::Options).then(|| i64::try_from(rng.below(1_000_000)).unwrap());
        out.push(Tick {
            sod,
            ltp: 5 + i64::try_from(rng.below(10_000_000)).unwrap(),
            ltq,
            oi,
        });
    }
    out
}

/// Every one-second bar of one instrument-month of the test day.
fn stored(store: &Path, segment: &str, symbol: &str) -> Vec<Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let file = BarFile::open_existing(store, path, id).unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

/// Every file under `dir`, relative path to bytes.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let entries =
            std::fs::read_dir(&at).unwrap_or_else(|why| panic!("{}: {why}", at.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|why| panic!("{}: {why}", at.display()))
                .path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let bytes =
                    std::fs::read(&path).unwrap_or_else(|why| panic!("{}: {why}", path.display()));
                out.insert(path.strip_prefix(dir).unwrap().to_path_buf(), bytes);
            }
        }
    }
    out
}

/// `drive` over the test day with `files` handed on as a source would.
fn drive_files(
    kind: ImportKind,
    store: &Path,
    only: &[String],
    files: &[TickFile],
) -> Result<Report, ImportRefusal> {
    let run = Run {
        kind,
        from: d1(),
        to: d1(),
        only,
        store_root: store,
    };
    drive(&run, |_, sink| {
        for file in files {
            sink(file.clone());
        }
        DayRead {
            held: true,
            ..DayRead::default()
        }
    })
}

fn spot(symbol: &str, ticks: Vec<Tick>) -> TickFile {
    TickFile {
        symbol: symbol.to_owned(),
        contract: None,
        name: format!("{symbol}.invented"),
        ticks,
    }
}

// ── the one-second build against a naive reference ─────────────────────────

/// 100,000 random files of every kind: `convert` and the slow reference
/// agree on every bar and every count, or both refuse the volume.
#[test]
fn the_one_second_build_matches_a_naive_reference_over_100k_random_files() {
    let mut rng = Rng(0x6744_464C_5345_4353);
    let (mut agreed, mut refused, mut bars) = (0_usize, 0_usize, 0_usize);
    for case in 0..100_000 {
        let kind = ImportKind::ALL[usize::try_from(rng.below(3)).unwrap()];
        let ticks = random_ticks(&mut rng, kind);
        match (convert(kind, d1(), &ticks), reference(kind, d1(), &ticks)) {
            (Ok(got), Ok(want)) => {
                assert_eq!(got, want, "case {case} {kind:?} {ticks:?}");
                bars += got.seconds.len();
                agreed += 1;
            }
            (Err(ImportRefusal::VolumeOverflow { .. }), Err(())) => refused += 1,
            (got, want) => panic!("case {case} {kind:?} {ticks:?}: {got:?} against {want:?}"),
        }
    }
    assert_eq!(agreed + refused, 100_000);
    assert!(
        refused > 100 && agreed > 90_000 && bars > 200_000,
        "{agreed} {refused} {bars}"
    );
}

/// The open is the first traded row of the second, the close the last, the
/// high and low the extremes and the volume the sum, for every sub-second
/// order of three traded rows among untraded ones: all 3! orders of the
/// traded prices, each with an untraded row before, between and after.
#[test]
fn a_mixed_second_uses_only_its_traded_rows_in_every_order() {
    let prices = [700_i64, 650, 720];
    let orders = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for order in orders {
        for kind in [ImportKind::Stocks, ImportKind::Options] {
            let mut ticks = vec![tick(T, 1, 0)];
            for (k, &at) in order.iter().enumerate() {
                ticks.push(tick(T, prices[at], u64::try_from(k).unwrap() + 1));
                ticks.push(tick(T, 9_999_999, 0));
            }
            let got = convert(kind, d1(), &ticks).unwrap();
            assert_eq!(got.seconds.len(), 1);
            let bar = got.seconds[0];
            let first = prices[order[0]];
            let last = prices[order[2]];
            assert_eq!(
                (bar.open, bar.high, bar.low, bar.close, bar.volume),
                (first, 720, 650, last, 6)
            );
            assert_eq!(got.placement.ltq_zero_dropped, 4);
        }
    }
    // A second of only untraded rows has no bar, and its neighbours keep theirs.
    let got = convert(
        ImportKind::Stocks,
        d1(),
        &[
            tick(T, 5, 1),
            tick(T + 1, 6, 0),
            tick(T + 1, 7, 0),
            tick(T + 2, 8, 1),
        ],
    )
    .unwrap();
    let secs: Vec<i64> = got.seconds.iter().map(|b| b.ts_micros).collect();
    assert_eq!(secs, vec![micros_at(d1(), T), micros_at(d1(), T + 2)]);
}

/// D-3170 regression. An untraded row stamped 09:15:05 is written before a
/// trade stamped 09:15:00: the trade was not known before 09:15:05, and it
/// was filed at 09:15:00, a print in a bar five seconds before the file
/// showed it. It now lands at the next in-order stamp.
#[test]
fn an_untraded_rows_stamp_is_evidence_of_time_and_no_trade_lands_before_it() {
    let ticks = [tick(T + 5, 100, 0), tick(T, 90, 5), tick(T + 6, 110, 5)];
    for kind in [ImportKind::Stocks, ImportKind::Options] {
        let got = convert(kind, d1(), &ticks).unwrap();
        assert_eq!(got.seconds.len(), 1, "{:?}", got.seconds);
        let bar = got.seconds[0];
        assert_eq!(bar.ts_micros, micros_at(d1(), T + 6), "never at 09:15:00");
        assert_eq!(
            (bar.open, bar.high, bar.low, bar.close, bar.volume),
            (90, 110, 90, 110, 10)
        );
        assert_eq!(got.placement.late_rows, 1);
        assert_eq!(got.placement.max_back_s, 5);
        assert_eq!(got.placement.ltq_zero_dropped, 1);
    }
    // The next in-order row may itself be untraded: it bounds the trade
    // before it just as well, and the trade lands there.
    let got = convert(
        ImportKind::Stocks,
        d1(),
        &[
            tick(T + 5, 100, 0),
            tick(T, 90, 5),
            tick(T + 5, 1, 0),
            tick(T + 9, 110, 5),
        ],
    )
    .unwrap();
    assert_eq!(got.seconds[0].ts_micros, micros_at(d1(), T + 5));
    // Untraded late rows are not counted as late kept rows.
    let quiet = convert(
        ImportKind::Stocks,
        d1(),
        &[tick(T + 5, 1, 7), tick(T, 1, 0)],
    )
    .unwrap();
    assert_eq!(
        (quiet.placement.late_rows, quiet.placement.late_unresolved),
        (0, 0)
    );
}

/// Hand-picked extremes: the first and last second of the day, the largest
/// price, one row, duplicate rows, and every row late.
#[test]
fn extreme_stamps_prices_and_duplicates_build_exactly() {
    for kind in ImportKind::ALL {
        for ticks in [
            vec![tick(0, 5, 1)],
            vec![tick(86_399, i64::MAX, 1), tick(86_399, 5, 1)],
            vec![tick(T, 100, 3); 1_000],
            vec![tick(86_399, 7, 1), tick(0, 8, 1), tick(1, 9, 1)],
            vec![
                tick(0, 7, 1),
                tick(86_399, 8, 1),
                tick(0, 9, 1),
                tick(86_399, 10, 1),
            ],
        ] {
            let got = convert(kind, d1(), &ticks).unwrap();
            assert_eq!(
                Ok(got),
                reference(kind, d1(), &ticks)
                    .map_err(|()| ImportRefusal::Fold { why: String::new() })
            );
        }
    }
    let dup = convert(ImportKind::Stocks, d1(), &vec![tick(T, 100, 3); 1_000]).unwrap();
    assert_eq!(
        dup.seconds[0].volume, 3_000,
        "a duplicate row is a print; the file cannot tell them apart"
    );
    let late = convert(
        ImportKind::Indices,
        d1(),
        &[tick(86_399, 7, 0), tick(0, 8, 0), tick(1, 9, 0)],
    )
    .unwrap();
    assert_eq!((late.seconds.len(), late.placement.late_unresolved), (1, 2));
}

// ── the runtime: boundaries, counts, the journal ───────────────────────────

/// Seconds at 00:00:00, in the pre-open, at 09:14:59, 09:15:00, 15:29:59,
/// 15:30:00, 15:30:01 and 23:59:59, through the whole runtime into the
/// store: only the session's seconds are filed, every other one is counted,
/// and offered seconds balance stored plus declined.
#[test]
fn session_edges_are_filed_or_counted_and_the_counts_balance() {
    let stamps = [
        0,
        hm(9, 0, 0),
        hm(9, 7, 59),
        hm(9, 8, 0),
        hm(9, 14, 59),
        hm(9, 15, 0),
        hm(12, 0, 0),
        hm(15, 29, 59),
        hm(15, 30, 0),
        hm(15, 30, 1),
        hm(23, 59, 59),
    ];
    for (kind, segment, symbol) in [
        (ImportKind::Stocks, "CASH", "RELIANCE"),
        (ImportKind::Indices, "INDEX", "NIFTY"),
    ] {
        let root = scratch("seconds-edges");
        let mut ticks: Vec<Tick> = stamps
            .iter()
            .map(|&s| tick(s, 1_000 + i64::from(s % 97), 2))
            .collect();
        ticks.push(tick(hm(12, 0, 0), 5, 0));
        let offered = convert(kind, d1(), &ticks).unwrap().seconds.len();
        let report = drive_files(kind, &root, &[], &[spot(symbol, ticks.clone())]).unwrap();
        let held = stored(&root, segment, symbol);
        let at: Vec<i64> = held.iter().map(|b| b.ts_micros).collect();
        let want: Vec<i64> = [hm(9, 15, 0), hm(12, 0, 0), hm(15, 29, 59)]
            .iter()
            .map(|&s| micros_at(d1(), s))
            .collect();
        assert_eq!(at, want, "{kind:?}");
        assert_eq!(report.seconds, 3);
        assert_eq!(
            report.seconds + report.outside_session,
            offered,
            "{kind:?} {report:?}"
        );
        assert_eq!(report.rows, ticks.len());
        let zero = usize::from(kind != ImportKind::Indices);
        assert_eq!(report.ltq_zero_dropped, zero);
        assert!(report.failures.is_empty(), "{:?}", report.failures);
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// D-3172 regression. One forward-stamped row at 14:00:00 among 10:00 rows
/// makes the rest late, and every one of them lands in one second at
/// 14:00:01. That is the stated rule (D-2802); what was missing is its size:
/// the report carried no back-step, the day was journalled `done` with no
/// trace of it. Now the run's largest back-step is reported and journalled.
#[test]
fn a_forward_spike_collapses_hours_into_one_second_and_says_how_far() {
    let root = scratch("seconds-spike");
    let mut ticks = Vec::new();
    for i in 0..60 {
        let sod = if i == 10 { hm(14, 0, 0) } else { hm(10, 0, i) };
        ticks.push(tick(sod, 1_000 + i64::from(i), 0));
    }
    ticks.push(tick(hm(14, 0, 1), 2_000, 0));
    ticks.push(tick(hm(15, 30, 5), 2_001, 0));
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    let landed = got
        .seconds
        .iter()
        .copied()
        .filter(|b| b.ts_micros == micros_at(d1(), hm(14, 0, 1)))
        .collect::<Vec<_>>();
    assert_eq!(landed.len(), 1);
    assert_eq!(
        (landed[0].open, landed[0].high, landed[0].close),
        (1_011, 2_000, 2_000)
    );
    assert_eq!(
        got.seconds.len(),
        10 + 1 + 1 + 1,
        "ten 10:00 seconds, the spike, 14:00:01, 15:30:05"
    );
    let report = drive_files(ImportKind::Indices, &root, &[], &[spot("NIFTY", ticks)]).unwrap();
    assert_eq!(report.late_rows, 49);
    assert_eq!(report.max_back_s, hm(14, 0, 0) - hm(10, 0, 11));
    let text = std::fs::read_to_string(journal_path(&root)).unwrap();
    assert!(
        text.contains(&format!(
            "late=49 late_unresolved=0 max_back_s={}",
            hm(14, 0, 0) - hm(10, 0, 11)
        )),
        "{text}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// D-3171 regression. A filter name the store never files matched no file,
/// recorded the day `done` under a key it shares with another filter, and
/// the other filter's later run then skipped the day unread: `*` is the
/// whole tree's key, and `NIFTY BANK` split on its space keys as `NIFTY`.
#[test]
fn a_filter_name_that_is_not_a_filed_symbol_is_refused_before_the_journal() {
    let long = "A".repeat(25);
    for name in [
        "*",
        "NIFTY BANK",
        "A,B",
        "",
        "nifty",
        "NIFTY\n",
        "NIFTY (torn)",
        long.as_str(),
    ] {
        let root = scratch("seconds-filter");
        let got = drive_files(
            ImportKind::Indices,
            &root,
            &[name.to_owned()],
            &[spot("NIFTY", vec![tick(T, 5, 0)])],
        );
        assert_eq!(
            got,
            Err(ImportRefusal::FilterName {
                name: name.to_owned()
            })
        );
        assert!(
            !journal_path(&root).exists(),
            "nothing journalled for {name:?}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    for name in ["NIFTY", "M&M", "BAJAJ-AUTO", "A_B"] {
        let root = scratch("seconds-filter-ok");
        assert!(
            drive_files(ImportKind::Stocks, &root, &[name.to_owned()], &[]).is_ok(),
            "{name}"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    assert!(
        ImportRefusal::FilterName { name: "*".into() }
            .to_string()
            .contains("\"*\"")
    );
}

/// D-3173 regression. A journal line torn by a crash was closed with a bare
/// newline, which made the fragment a whole line, and the NEXT run refused
/// the journal as foreign: one torn write stopped every later run. It is
/// closed with a mark now and skipped on every load, also when the crash
/// tore the mark itself.
#[test]
fn a_torn_journal_line_stays_ignored_on_every_later_run() {
    for torn in [
        &b"done ind"[..],
        b"begin indices 2024-04-01 *",
        b"done ind (to",
    ] {
        let root = scratch("seconds-torn");
        // `definition=` names the bar definition that built the day (D-3191).
        let mut text = b"done indices 2024-04-01 * definition=2 files=2\n".to_vec();
        text.extend_from_slice(torn);
        put(&root, "imports/gdfl.journal", &text);
        for run in 0..3 {
            let got = drive_files(ImportKind::Indices, &root, &[], &[]);
            assert!(got.is_ok(), "run {run} after {torn:?}: {got:?}");
            assert_eq!(got.unwrap().days_skipped, 1);
        }
        let after = std::fs::read_to_string(journal_path(&root)).unwrap();
        assert!(after.ends_with(" (torn)\n"), "{after:?}");
        assert_eq!(after.matches('\n').count(), 2, "closed once: {after:?}");
        std::fs::remove_dir_all(root).unwrap();
    }
}

/// D-3174 regression. A stock day naming one ticker twice (`.csv` and
/// `.CSV`): the first name is refused as twins and the second was counted
/// nowhere, so files, skips and refusals did not add up to the entries.
#[test]
fn every_capital_market_entry_is_a_file_a_skip_or_a_refusal() {
    let root = scratch("seconds-twins");
    let folder = day_folder_name(CmKind::Stocks, d1());
    let stock = |stem: &str| {
        csv(&[
            row(stem, d1(), T, "731.40", 3, 0),
            row(stem, d1(), T + 1, "731.45", 0, 0),
            row(stem, d1(), T + 2, "731.50", 2, 0),
        ])
    };
    let entries = [
        (format!("{folder}/"), Vec::new()),
        (format!("{folder}/RELIANCE.NSE.csv"), stock("RELIANCE.NSE")),
        (format!("{folder}/RELIANCE.NSE.CSV"), stock("RELIANCE.NSE")),
        (format!("{folder}/SBIN.NSE.csv"), stock("SBIN.NSE")),
        (format!("{folder}/../SBIN.NSE.csv"), stock("SBIN.NSE")),
        (format!("{folder}/sub/SBIN.NSE.csv"), stock("SBIN.NSE")),
    ];
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    put(
        &root,
        "ts/cm/STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024.bts",
        &bts(&refs),
    );
    let run = Run {
        kind: ImportKind::Stocks,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &root.join("S"),
    };
    let got = run_cm(&TickStore::new(&root.join("ts")), &run).unwrap();
    assert_eq!((got.files, got.files_refused), (1, 1), "{got:?}");
    assert_eq!(
        got.files + got.files_skipped + got.files_refused,
        entries.len(),
        "{got:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A rerun is a no-op on the store, two fresh runs over one input leave
/// byte-identical stores, and a day with a refused file, rerun after the
/// source is mended, ends byte for byte where a clean run ends.
#[test]
fn reruns_and_mended_partial_days_converge_on_the_clean_store() {
    let root = scratch("seconds-idem");
    let good: Vec<Tick> = (0..300)
        .map(|s| tick(T + s, 1_000 + i64::from(s % 13), u64::from(s % 3)))
        .collect();
    let other: Vec<Tick> = (0..300)
        .map(|s| tick(T + 2 * s, 500 + i64::from(s % 7), 1))
        .collect();
    let bad = vec![tick(T, 5, u64::MAX)];
    let clean = root.join("clean");
    let both = [spot("RELIANCE", good.clone()), spot("SBIN", other.clone())];
    drive_files(ImportKind::Stocks, &clean, &[], &both).unwrap();
    let twin = root.join("twin");
    drive_files(ImportKind::Stocks, &twin, &[], &both).unwrap();
    assert_eq!(
        tree(&clean),
        tree(&twin),
        "two runs over one input, one store"
    );
    let before = tree(&clean);
    let again = drive_files(ImportKind::Stocks, &clean, &[], &both).unwrap();
    assert_eq!((again.days_skipped, again.seconds_committed), (1, 0));
    assert_eq!(tree(&clean), before);
    // Partial: SBIN refused (a quantity past i64), RELIANCE lands.
    let mended = root.join("mended");
    let partial = drive_files(
        ImportKind::Stocks,
        &mended,
        &[],
        &[spot("RELIANCE", good), spot("SBIN", bad)],
    )
    .unwrap();
    // The refused file is counted as refused only, never also as read
    // (D-3168; this line said (2, 1) and encoded the double count).
    assert_eq!((partial.files, partial.files_refused), (1, 1));
    assert!(
        std::fs::read_to_string(journal_path(&mended))
            .unwrap()
            .contains("incomplete stocks")
    );
    let rerun = drive_files(ImportKind::Stocks, &mended, &[], &both).unwrap();
    assert!(rerun.failures.is_empty(), "{:?}", rerun.failures);
    let bars = |s: &Path, sym: &str| stored(s, "CASH", sym);
    assert_eq!(bars(&mended, "RELIANCE"), bars(&clean, "RELIANCE"));
    assert_eq!(bars(&mended, "SBIN"), bars(&clean, "SBIN"));
    // Every bar file byte for byte. The census is not compared byte for byte:
    // the mended history installed it twice, and its second slot differs
    // (measured: the files part at byte 16,384).
    let files = |s: &Path| -> BTreeMap<PathBuf, Vec<u8>> {
        tree(s)
            .into_iter()
            .filter(|(p, _)| p.starts_with("bars"))
            .collect()
    };
    assert_eq!(files(&mended), files(&clean));
    assert!(!files(&clean).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

// ── the capital-market reader ──────────────────────────────────────────────

fn expect(stem: &str) -> Expect<'_> {
    Expect {
        kind: CmKind::Stocks,
        stem,
        day: d1(),
    }
}

fn session_file(rows: &[String]) -> Vec<u8> {
    csv(rows)
}

/// Every hostile shape of a row refuses by name, and the LTP edges decode to
/// the exact paisa: no NaN, sign, exponent, blank, third decimal or sub-tick
/// price is ever a price.
#[test]
fn hostile_rows_refuse_by_name_and_edge_prices_decode_exactly() {
    let stem = "RELIANCE.NSE";
    let good = row(stem, d1(), T, "731.40", 1, 0);
    let one = |ltp: &str| {
        decode(
            &session_file(&[row(stem, d1(), T, ltp, 1, 0)]),
            &expect(stem),
        )
    };
    for bad in [
        "NaN",
        "nan",
        "inf",
        "-inf",
        "1e3",
        "+5.00",
        "-5.00",
        " 5.00",
        "5.00 ",
        "5.000",
        "0.045",
        "0.04",
        "0.00",
        "0",
        "5.",
        ".5",
        "",
        "5,00",
        "99999999999999999.99",
        "92233720368547758.08",
    ] {
        let got = one(bad);
        assert!(
            matches!(
                got,
                Err(CmRefusal::PriceRefused { line: 2 } | CmRefusal::MalformedRow { line: 2 })
            ),
            "{bad:?}: {got:?}"
        );
    }
    for (text, paisa) in [
        ("0.05", 5),
        ("0.1", 10),
        ("731.4", 73_140),
        ("92233720368547758.07", i64::MAX),
    ] {
        assert_eq!(one(text).unwrap().rows[0].ltp, paisa, "{text}");
    }
    // The largest LTQ decodes; the runtime then refuses it, never saturates.
    let max = decode(
        &session_file(&[row(stem, d1(), T, "5", u64::MAX, 0)]),
        &expect(stem),
    )
    .unwrap();
    assert_eq!(max.rows[0].ltq, u64::MAX);
    let as_ticks: Vec<Tick> = max.rows.iter().map(|r| tick(r.sod, r.ltp, r.ltq)).collect();
    assert_eq!(
        convert(ImportKind::Stocks, d1(), &as_ticks),
        Err(ImportRefusal::VolumeOverflow { sod: T })
    );
    let over = format!("{stem},01/04/2024,09:15:00,5,0,0,0,0,18446744073709551616,0");
    assert_eq!(
        decode(&session_file(&[over]), &expect(stem)),
        Err(CmRefusal::MalformedRow { line: 2 })
    );
    // Times that are not a real HH:MM:SS.
    for time in [
        "24:00:00",
        "09:60:00",
        "09:15:60",
        "9:15:00",
        "09:15:00.5",
        "09-15-00",
        "",
    ] {
        let text = format!("{stem},01/04/2024,{time},5,0,0,0,0,1,0");
        assert_eq!(
            decode(&session_file(&[text]), &expect(stem)),
            Err(CmRefusal::MalformedRow { line: 2 }),
            "{time:?}"
        );
    }
    // A BOM, an empty file, a header in the middle, a column short or extra.
    let mut bom = vec![0xEF, 0xBB, 0xBF];
    bom.extend_from_slice(&session_file(std::slice::from_ref(&good)));
    assert_eq!(decode(&bom, &expect(stem)), Err(CmRefusal::HeaderUnknown));
    assert_eq!(decode(b"", &expect(stem)), Err(CmRefusal::HeaderUnknown));
    let header = crate::gdfl_cm::HEADER_OPEN_INTEREST.to_owned();
    assert_eq!(
        decode(&session_file(&[good.clone(), header]), &expect(stem)),
        Err(CmRefusal::TickerMismatch { line: 3 })
    );
    assert_eq!(
        decode(
            &session_file(&[good.clone(), format!("{good},0")]),
            &expect(stem)
        ),
        Err(CmRefusal::MalformedRow { line: 3 })
    );
    let short = good.rsplit_once(',').unwrap().0.to_owned();
    assert_eq!(
        decode(&session_file(&[short, good.clone()]), &expect(stem)),
        Err(CmRefusal::MalformedRow { line: 2 })
    );
    // LF and CRLF lines mixed in one file read alike.
    let mut mixed = session_file(std::slice::from_ref(&good));
    mixed.extend_from_slice(format!("{good}\n").as_bytes());
    assert_eq!(decode(&mixed, &expect(stem)).unwrap().rows.len(), 2);
    // The row cap is the reader's (CM-05); a file is never read past it.
    assert_eq!(crate::fetch::MAX_ROWS, 1_000_000);
}

/// 100,000 random mutations of a valid file (a byte flipped, dropped,
/// inserted from the row alphabet, or the file cut) never panic the
/// reader, and whatever decodes holds the reader's invariants.
#[test]
fn a_mutated_file_never_panics_the_reader_and_what_decodes_is_sound() {
    let stem = "SBIN.NSE";
    let mut rows = Vec::new();
    for s in 0..12 {
        rows.push(row(
            stem,
            d1(),
            T - 2 + s * 7,
            &format!("6{s}.{s}5"),
            u64::from(s % 3),
            0,
        ));
    }
    let base = session_file(&rows);
    let alphabet = b"0123456789,.:/\r\n SBIN.NSE-";
    let mut rng = Rng(0x5345_434F_4E44_5331);
    let (mut ok, mut refused) = (0_usize, 0_usize);
    for _ in 0..100_000 {
        let mut bytes = base.clone();
        for _ in 0..=rng.below(3) {
            let at = usize::try_from(rng.below(u64::try_from(bytes.len()).unwrap() + 1)).unwrap();
            match rng.below(4) {
                0 if at < bytes.len() => bytes[at] ^= 1 << rng.below(8),
                1 if at < bytes.len() => {
                    bytes.remove(at);
                }
                2 => bytes.insert(
                    at,
                    alphabet[usize::try_from(rng.below(alphabet.len() as u64)).unwrap()],
                ),
                _ => bytes.truncate(at),
            }
        }
        match decode(&bytes, &expect(stem)) {
            Ok(file) => {
                ok += 1;
                sound(&file, &bytes);
            }
            Err(why) => {
                refused += 1;
                assert!(!why.to_string().is_empty());
            }
        }
    }
    assert_eq!(ok + refused, 100_000);
    assert!(ok > 100 && refused > 10_000, "{ok} {refused}");
}

fn sound(file: &CmFile, bytes: &[u8]) {
    let lines = bytes.iter().filter(|&&b| b == b'\n').count() + 1;
    assert!(file.rows.len() < lines);
    let total = usize::try_from(file.rows_pre + file.rows_in_session + file.rows_post).unwrap();
    assert_eq!(total, file.rows.len());
    let mut max = 0;
    for r in &file.rows {
        assert!(r.ltp >= 5 && r.sod < 86_400);
        max = max.max(r.sod);
    }
    assert_eq!(max, file.max_sod);
    assert_eq!(file.source_crc32, crc32(bytes));
}

// ── the tick store ─────────────────────────────────────────────────────────

/// A `.bts` holding one stock day: the folder entry and two files.
fn one_day_store() -> (Vec<u8>, Vec<(String, Vec<u8>)>) {
    let folder = day_folder_name(CmKind::Stocks, d1());
    let file = |stem: &str| {
        csv(&[
            row(stem, d1(), T, "731.40", 3, 0),
            row(stem, d1(), T + 4, "731.45", 0, 0),
            row(stem, d1(), T + 9, "731.50", 2, 0),
        ])
    };
    let entries = vec![
        (format!("{folder}/"), Vec::new()),
        (format!("{folder}/RELIANCE.NSE.csv"), file("RELIANCE.NSE")),
        (format!("{folder}/SBIN.NSE.csv"), file("SBIN.NSE")),
    ];
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    (bts(&refs), entries)
}

/// The day file cut at every length refuses, never panics; the whole file
/// reads, and every rebuilt member is its original byte for byte.
#[test]
fn a_tick_store_day_cut_anywhere_refuses_by_name() {
    let (whole, entries) = one_day_store();
    let len = u64::try_from(whole.len()).unwrap();
    let index = read_index(&whole, len).unwrap();
    for (entry, (name, bytes)) in index.iter().zip(&entries) {
        assert_eq!(&entry.name, name);
        assert_eq!(&rebuild(&whole, entry).unwrap(), bytes);
    }
    for cut in 0..whole.len() {
        let short = whole[..cut].to_vec();
        let got = read_index(&short, u64::try_from(cut).unwrap());
        assert!(
            matches!(got, Err(CmRefusal::TickStoreMalformed { .. })),
            "cut {cut}: {got:?}"
        );
    }
    // A length stated past the bytes is a failed read, not a short buffer.
    assert!(matches!(
        read_index(&whole, len + 1),
        Err(CmRefusal::TickStoreUnavailable { .. } | CmRefusal::TickStoreMalformed { .. })
    ));
}

/// 100,000 random byte flips of a day file: no panic, and any member that
/// rebuilds to its stated size and CRC-32 is the original, so the CM-09
/// check is what stands between a flipped byte and a row.
#[test]
fn a_flipped_tick_store_byte_never_reaches_a_row_unnoticed() {
    let (whole, entries) = one_day_store();
    let len = u64::try_from(whole.len()).unwrap();
    let mut rng = Rng(0x4254_5354_4F52_4531);
    let (mut listed, mut rebuilt_clean) = (0_usize, 0_usize);
    for _ in 0..100_000 {
        let mut bytes = whole.clone();
        for _ in 0..=rng.below(2) {
            let at = usize::try_from(rng.below(len)).unwrap();
            bytes[at] ^= 1 << rng.below(8);
        }
        let Ok(index) = read_index(&bytes, len) else {
            continue;
        };
        listed += 1;
        for entry in &index {
            let Ok(rebuilt) = rebuild(&bytes, entry) else {
                continue;
            };
            if u64::try_from(rebuilt.len()).unwrap() == entry.size && crc32(&rebuilt) == entry.crc {
                let mut original = None;
                for (name, data) in &entries {
                    if *name == entry.name {
                        original = Some(data);
                    }
                }
                assert_eq!(Some(&rebuilt), original, "{}", entry.name);
                rebuilt_clean += 1;
            }
        }
    }
    assert!(
        listed > 1_000 && rebuilt_clean > 1_000,
        "{listed} {rebuilt_clean}"
    );
}

/// A day file built from a raw index, for footer and entry fields no
/// encoder would write.
fn store_with_index(blocks: &[u8], index: &[u8], raw_len: u64, n: u32) -> Vec<u8> {
    let mut out = b"BRTXTS01".to_vec();
    out.extend_from_slice(blocks);
    let index_off = u64::try_from(out.len()).unwrap();
    let frame =
        ruzstd::encoding::compress_to_vec(index, ruzstd::encoding::CompressionLevel::Fastest);
    out.extend_from_slice(&frame);
    out.extend_from_slice(&index_off.to_le_bytes());
    out.extend_from_slice(&u64::try_from(frame.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&raw_len.to_le_bytes());
    out.extend_from_slice(&crc32(index).to_le_bytes());
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(b"BRTXTSE1");
    out
}

fn entry_bytes(name: &str, kind: u8, off: u64, len: u64, size: u64, rows: u64) -> Vec<u8> {
    let mut e = u16::try_from(name.len()).unwrap().to_le_bytes().to_vec();
    e.extend_from_slice(name.as_bytes());
    e.push(kind);
    for v in [off, len, size] {
        e.extend_from_slice(&v.to_le_bytes());
    }
    e.extend_from_slice(&0_u32.to_le_bytes());
    e.extend_from_slice(&rows.to_le_bytes());
    e.extend_from_slice(&0_u32.to_le_bytes());
    e
}

/// One zstd frame of RLE blocks that inflates to `blocks` × 128 KiB of
/// one byte: a few bytes per 128 KiB, the bomb shape.
fn zstd_bomb(blocks: u32) -> Vec<u8> {
    let mut frame = vec![0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x38];
    for k in 0..blocks {
        let last = u32::from(k + 1 == blocks);
        let header = ((128 * 1_024) << 3) | (1 << 1) | last;
        frame.extend_from_slice(&header.to_le_bytes()[..3]);
        frame.push(b'7');
    }
    frame
}

/// Offsets and lengths at the `u64` edge, entry counts past the index, and a
/// bomb under a small stated size: each refuses by name, nothing wraps,
/// nothing is inflated past one byte over what the index states.
#[test]
fn tick_store_offsets_at_the_edge_and_bombs_refuse_without_wrapping() {
    let block = {
        let mut b = vec![2_u8];
        b.extend_from_slice(&zstd_bomb(512));
        b
    };
    let blen = u64::try_from(block.len()).unwrap();
    let malformed = |bytes: &[u8]| read_index(&bytes.to_vec(), u64::try_from(bytes.len()).unwrap());
    // An entry whose block runs past u64.
    let e = entry_bytes("F/X.csv", 2, u64::MAX, 2, 1, 0);
    assert_eq!(
        malformed(&store_with_index(
            &block,
            &e,
            u64::try_from(e.len()).unwrap(),
            1
        )),
        Err(CmRefusal::TickStoreMalformed {
            what: "a block does not lie inside the blocks area"
        })
    );
    let e = entry_bytes("F/X.csv", 2, 8, u64::MAX - 7, 1, 0);
    assert!(matches!(
        malformed(&store_with_index(
            &block,
            &e,
            u64::try_from(e.len()).unwrap(),
            1
        )),
        Err(CmRefusal::TickStoreMalformed { .. })
    ));
    // u32::MAX entries stated, one held.
    let e = entry_bytes("F/X.csv", 2, 8, blen, 1, 0);
    assert!(matches!(
        malformed(&store_with_index(
            &block,
            &e,
            u64::try_from(e.len()).unwrap(),
            u32::MAX
        )),
        Err(CmRefusal::TickStoreMalformed { .. })
    ));
    // An index stated at u64::MAX bytes decodes to what it holds and refuses.
    assert!(matches!(
        malformed(&store_with_index(&block, &e, u64::MAX, 1)),
        Err(CmRefusal::TickStoreMalformed { .. })
    ));
    // The footer's offset at the edge.
    let mut edge = store_with_index(&block, &e, u64::try_from(e.len()).unwrap(), 1);
    let at = edge.len() - 40;
    edge[at..at + 8].copy_from_slice(&(u64::MAX - 3).to_le_bytes());
    assert!(matches!(
        malformed(&edge),
        Err(CmRefusal::TickStoreMalformed { .. })
    ));
    // The bomb: 64 MiB under a stated 100 bytes is refused after 101.
    let small = store_with_index(&block, &entry_bytes("F/X.csv", 2, 8, blen, 100, 0), 0, 0);
    let fine = store_with_index(&block, &e, u64::try_from(e.len()).unwrap(), 1);
    assert!(small.len() < 2_200);
    let index = read_index(&fine, u64::try_from(fine.len()).unwrap()).unwrap();
    let mut bomb = index[0].clone();
    bomb.size = 100;
    let started = Instant::now();
    assert!(matches!(
        rebuild(&fine, &bomb),
        Err(CmRefusal::TickStoreMalformed { .. })
    ));
    assert!(
        started.elapsed().as_secs() < 5,
        "stopped one byte past the stated size"
    );
    // And stated whole, it inflates exactly to what it claims.
    bomb.size = 512 * 128 * 1_024;
    assert_eq!(rebuild(&fine, &bomb).unwrap().len(), 512 * 128 * 1_024);
}

// ── the zips ───────────────────────────────────────────────────────────────

/// One outer archive with one stored indices day zip whose members are
/// `members`.
fn outer(members: &[(&str, &[u8], Method)]) -> Vec<u8> {
    let day_zip = zip(members);
    let name = crate::gdfl_archive::day_zip_name(CmKind::Indices, d1());
    zip(&[(name.as_str(), day_zip.as_slice(), Method::Stored)])
}

fn nifty_file() -> Vec<u8> {
    let stem = "NIFTY 50.NSE_IDX";
    csv(&[
        row(stem, d1(), T, "22000.05", 0, 0),
        row(stem, d1(), T + 1, "22001.10", 0, 0),
        row(stem, d1(), hm(15, 30, 2), "22002.00", 0, 0),
    ])
}

/// Names that climb out of the day folder, start at the root, use a
/// backslash or sit in a subfolder are listed and counted, never filed, so
/// no such member is ever read as the ticker's file; the exact name is. A
/// NUL after the extension is a third spelling of it and refuses the ticker
/// `ExtensionUnknown`, loudly, never read either.
#[test]
fn member_names_that_climb_or_hide_are_never_the_tickers_file() {
    let folder = day_folder_name(CmKind::Indices, d1());
    let data = nifty_file();
    let names = [
        format!("{folder}/../NIFTY 50.NSE_IDX.csv"),
        format!("/{folder}/NIFTY 50.NSE_IDX.csv"),
        format!("{folder}\\NIFTY 50.NSE_IDX.csv"),
        format!("{folder}/sub/NIFTY 50.NSE_IDX.csv"),
        format!("{folder}/./NIFTY 50.NSE_IDX.csv"),
        "../NIFTY 50.NSE_IDX.csv".to_owned(),
    ];
    let members: Vec<(&str, &[u8], Method)> = names
        .iter()
        .map(|n| (n.as_str(), data.as_slice(), Method::Deflated))
        .collect();
    let bytes = outer(&members);
    let len = u64::try_from(bytes.len()).unwrap();
    let archive = Archive::from_source(bytes, len).unwrap();
    let listing = archive.day(CmKind::Indices, d1()).unwrap().unwrap();
    assert_eq!(listing.entries().len(), names.len());
    assert_eq!(listing.unresolved(), names.len());
    assert!(listing.locate("NIFTY 50.NSE_IDX").unwrap().is_none());
    let nul = format!("{folder}/NIFTY 50.NSE_IDX.csv\0");
    let bytes = outer(&[(nul.as_str(), data.as_slice(), Method::Stored)]);
    let len = u64::try_from(bytes.len()).unwrap();
    let poisoned = Archive::from_source(bytes, len)
        .unwrap()
        .day(CmKind::Indices, d1())
        .unwrap()
        .unwrap();
    assert_eq!(
        poisoned.locate("NIFTY 50.NSE_IDX"),
        Err(CmRefusal::ExtensionUnknown)
    );
    // The exact name beside them is the one read.
    let exact = format!("{folder}/NIFTY 50.NSE_IDX.csv");
    let mut with: Vec<(&str, &[u8], Method)> = members.clone();
    with.push((exact.as_str(), data.as_slice(), Method::Stored));
    let bytes = outer(&with);
    let len = u64::try_from(bytes.len()).unwrap();
    let archive = Archive::from_source(bytes, len).unwrap();
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .unwrap();
    let got = crate::gdfl_cm::read_day(&archive, &key, d1())
        .unwrap()
        .unwrap();
    assert_eq!(got.file.rows.len(), 3);
    // An outer name that reaches the day zip's name through `..` is no day zip.
    let day_zip = zip(&with);
    let sneaky = "INDICES/2024/APR_2024/../APR_2024/GFDLCM_INDICES_TICK_01042024.zip";
    let bytes = zip(&[(sneaky, day_zip.as_slice(), Method::Stored)]);
    let len = u64::try_from(bytes.len()).unwrap();
    let archive = Archive::from_source(bytes, len).unwrap();
    assert_eq!(archive.ignored(), 1);
    assert!(archive.day(CmKind::Indices, d1()).unwrap().is_none());
}

/// A deflated member that inflates to 16 MiB under a stated 100 bytes is
/// read to 101 bytes and refused by length; stated whole it reads whole.
#[test]
fn a_deflate_bomb_is_inflated_one_byte_past_its_stated_length_and_no_further() {
    let big = vec![0_u8; 16 << 20];
    let bytes = zip(&[("bomb.csv", big.as_slice(), Method::Deflated)]);
    assert!(bytes.len() < 100_000, "{}", bytes.len());
    let len = u64::try_from(bytes.len()).unwrap();
    let entries = zip_entries(&bytes, 0, len).unwrap();
    let started = Instant::now();
    let read = member_bytes(&bytes, entries[0].locator, "bomb.csv", 100).unwrap();
    assert_eq!(read.len(), 101);
    assert!(started.elapsed().as_secs() < 5);
    let whole = member_bytes(&bytes, entries[0].locator, "bomb.csv", entries[0].len).unwrap();
    assert_eq!(whole.len(), 16 << 20);
}

/// 100,000 random byte flips of an outer archive: no panic, and any file
/// that is read through `read_day` (so past the CM-09 check) decodes to
/// exactly the clean file's rows.
#[test]
fn a_flipped_archive_byte_never_reaches_a_row_unnoticed() {
    let folder = day_folder_name(CmKind::Indices, d1());
    let data = nifty_file();
    let exact = format!("{folder}/NIFTY 50.NSE_IDX.csv");
    let whole = outer(&[(exact.as_str(), data.as_slice(), Method::Deflated)]);
    let len = u64::try_from(whole.len()).unwrap();
    let key = brutex_core::instrument::InstrumentKey::index(
        brutex_core::instrument::Exchange::Nse,
        "NIFTY",
    )
    .unwrap();
    let clean = crate::gdfl_cm::read_day(
        &Archive::from_source(whole.clone(), len).unwrap(),
        &key,
        d1(),
    )
    .unwrap()
    .unwrap();
    let mut rng = Rng(0x5A49_5046_4C49_5031);
    let (mut read, mut refused) = (0_usize, 0_usize);
    for _ in 0..100_000 {
        let mut bytes = whole.clone();
        for _ in 0..=rng.below(2) {
            let at = usize::try_from(rng.below(len)).unwrap();
            bytes[at] ^= 1 << rng.below(8);
        }
        let Ok(archive) = Archive::from_source(bytes, len) else {
            refused += 1;
            continue;
        };
        match crate::gdfl_cm::read_day(&archive, &key, d1()) {
            Ok(Some(got)) => {
                assert_eq!(got.file.rows, clean.file.rows);
                read += 1;
            }
            Ok(None) | Err(_) => refused += 1,
        }
    }
    assert_eq!(read + refused, 100_000);
    assert!(read > 1_000 && refused > 1_000, "{read} {refused}");
}

// ── cost ───────────────────────────────────────────────────────────────────

fn percentiles(mut samples: Vec<f64>) -> (f64, f64, f64) {
    samples.sort_by(f64::total_cmp);
    let at = |q: f64| {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss,
            reason = "an index into the samples"
        )]
        let i = ((samples.len() - 1) as f64 * q).round() as usize;
        samples[i]
    };
    (at(0.5), at(0.99), at(1.0))
}

/// Per-tick build cost at 10^3 to 10^6 ticks (each sample is one whole
/// `convert` divided by its ticks, so the max is the worst whole run, not
/// the worst single tick), and the per-second lookup cost in a one-second
/// month of 10^3 to 10^6 bars (each sample one `first_at_or_after`).
/// Printed for the report; asserted only for correctness.
#[test]
fn per_tick_build_and_per_second_lookup_cost() {
    let mut rng = Rng(0x434F_5354_5331_0001);
    for n in [1_000_u32, 10_000, 100_000, 1_000_000] {
        // Twelve ticks a second from midnight, so 10^6 ticks stay inside the
        // day: three a second from 09:15 ran to second 366,633, a stamp the
        // runtime now refuses (D-3194).
        let ticks: Vec<Tick> = (0..n)
            .map(|i| tick(i / 12, 5 + i64::from(i % 1_000), u64::from(i % 4)))
            .collect();
        let reps = (2_000_000 / n).clamp(5, 200);
        let mut samples = Vec::new();
        let mut seconds = 0;
        for _ in 0..reps {
            let started = Instant::now();
            let got = convert(ImportKind::Stocks, d1(), &ticks).unwrap();
            #[expect(clippy::cast_precision_loss, reason = "a timing")]
            samples.push(started.elapsed().as_nanos() as f64 / f64::from(n));
            seconds = got.seconds.len();
        }
        assert_eq!(seconds, usize::try_from(n.div_ceil(12)).unwrap());
        let (p50, p99, max) = percentiles(samples);
        eprintln!("build n={n} reps={reps} ns/tick p50={p50:.1} p99={p99:.1} max={max:.1}");
    }
    let root = scratch("seconds-lookup");
    for (k, n) in [1_000_u32, 10_000, 100_000, 1_000_000]
        .into_iter()
        .enumerate()
    {
        let symbol = ["LA", "LB", "LC", "LD"][k];
        let first = micros_at(d1(), 0);
        let bars: Vec<Bar> = (0..n)
            .map(|i| Bar {
                ts_micros: first + i64::from(i) * 2_000_000,
                open: 5,
                high: 5,
                low: 5,
                close: 5,
                volume: 1,
                open_interest: OI_NULL,
            })
            .collect();
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "CASH",
            symbol,
            contract: None,
            timeframe: Timeframe::SECOND_1,
            month: d1().year_month().unwrap(),
            file: FileKind::Bars,
        })
        .unwrap();
        let mut file = BarFile::open_or_create(&root, path, 7).unwrap();
        file.append(&bars).unwrap();
        let mut samples = Vec::new();
        for _ in 0..2_000 {
            let want = rng.below(u64::from(n) * 2);
            let ts = first + i64::try_from(want).unwrap() * 1_000_000;
            let started = Instant::now();
            let at = file.first_at_or_after(ts).unwrap();
            #[expect(clippy::cast_precision_loss, reason = "a timing")]
            samples.push(started.elapsed().as_nanos() as f64);
            assert_eq!(
                at,
                want.div_ceil(2),
                "a second with no bar answers the next bar, never a neighbour before it"
            );
        }
        let (p50, p99, max) = percentiles(samples);
        eprintln!("lookup n={n} ns p50={p50:.0} p99={p99:.0} max={max:.0}");
    }
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(hms(T), "09:15:00");
}

/// D-3175 regression. An options name that does not decode was wanted by a
/// filter whenever the name merely STARTED with a filter name, so a run
/// filtered to `NIFTY` refused `NIFTYNXT50…` names, another underlying's,
/// as its own failures. The whole underlying is compared now: the longest
/// F&O underlying the name starts with.
#[test]
fn an_undecodable_option_name_is_wanted_only_by_its_whole_underlying() {
    let root = scratch("seconds-nfo-prefix");
    let folder = crate::gdfl_nfo::day_folder_name(d1());
    let body = |t: &str| csv(&[row(&format!("{t}.NFO"), d1(), T, "5", 25, 1)]);
    let names = ["NIFTYJUNK", "NIFTYNXT50JUNK", "BANKNIFTYJUNK", "ZZZJUNK"];
    let entries: Vec<(String, Vec<u8>)> = names
        .iter()
        .map(|t| (format!("{folder}\\Options\\{t}.NFO.csv"), body(t)))
        .collect();
    let refs: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&refs),
    );
    let source = crate::gdfl_nfo::NfoTickStore::new(&root.join("ts"));
    let go = |only: &[String], store: &str| {
        let run = Run {
            kind: ImportKind::Options,
            from: d1(),
            to: d1(),
            only,
            store_root: &root.join(store),
        };
        run_nfo(&source, &run).unwrap()
    };
    let nifty = go(&["NIFTY".to_owned()], "A");
    assert_eq!(
        (nifty.files_refused, nifty.files_skipped),
        (1, 3),
        "{:?}",
        nifty.failures
    );
    assert!(nifty.failures[0].instrument.contains("NIFTYJUNK"));
    let next = go(&["NIFTYNXT50".to_owned()], "B");
    assert_eq!(
        (next.files_refused, next.files_skipped),
        (1, 3),
        "{:?}",
        next.failures
    );
    assert!(next.failures[0].instrument.contains("NIFTYNXT50JUNK"));
    let all = go(&[], "C");
    assert_eq!(
        all.files_refused, 4,
        "with no filter every undecodable name is refused"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// D-3176. At the 2019-02-01 cutover the vendor renamed long-dated
/// contracts (operator's measurement on the real archive: one contract
/// trades as `ACC19FEB1260PE` to 2019-01-31 and as `ACC28FEB191260PE` from
/// 2019-02-01; 6,253 of 7,685 have such a twin; the names here are invented
/// in that shape). Four facts, pinned:
/// 1. The series is keyed by the DECODED contract (underlying, expiry,
///    strike, side), never by ticker text: one contract on two days lands
///    in one contract directory, one month file, with no ticker text in
///    its path.
/// 2. Since D-3165 the pre-cutover monthly name decodes with its month's
///    sourced day (February 2019 -> 2019-02-28), so the two names hand
///    `nfo_day`'s sink ONE (symbol, contract): the series joins across the
///    rename, keyed by the decoded contract, never by ticker text.
/// 3. Every GDFL day before the calendar's first measured day
///    (2019-12-02) is refused `CalendarUnmeasured`, the cutover included, so
///    the pair cannot reach the store through `run_nfo` today: the join is
///    proven at `nfo_day`, the step that hands the filing path its key, and
///    the store's keying by that contract is proven by (1).
/// 4. Two names of one contract can only meet in one second on ONE day
///    (a second belongs to its day), and there both are refused,
///    `TickerAmbiguous`, never merged: `nfo_day` counts decoded contracts
///    first. Shown with the NIFTY monthly and dated spellings of the January
///    2019 contract on 2019-01-15 (D-3176). On each side of the cutover the
///    ACC name of the other era is refused by name, never read as the same
///    contract.
#[test]
fn a_renamed_contract_is_keyed_by_its_decoded_contract_never_its_name() {
    let root = scratch("seconds-rename");
    let (d1, d2) = (d1(), Day::new(2024, 4, 2).unwrap());
    let jan = Day::new(2019, 1, 31).unwrap();
    let feb = Day::new(2019, 2, 1).unwrap();
    let file = |t: &str, d: Day| {
        csv(&[
            row(&format!("{t}.NFO"), d, T, "12.5", 75, 1),
            row(&format!("{t}.NFO"), d, T + 3, "12.5", 75, 2),
        ])
    };
    let day_store = |d: Day, names: &[&str]| {
        let folder = crate::gdfl_nfo::day_folder_name(d);
        let entries: Vec<(String, Vec<u8>)> = names
            .iter()
            .map(|t| (format!("{folder}\\Options\\{t}.NFO.csv"), file(t, d)))
            .collect();
        let refs: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let rel = format!(
            "ts/options/{:04}/{}_{:04}/GFDLNFO_TICK_{:02}{:02}{:04}.bts",
            d.year(),
            crate::gdfl_fixtures::mon(d),
            d.year(),
            d.day(),
            d.month(),
            d.year()
        );
        put(&root, &rel, &bts(&refs));
    };
    let source = crate::gdfl_nfo::NfoTickStore::new(&root.join("ts"));
    let import = |from: Day, to: Day, store: &str| {
        let run = Run {
            kind: ImportKind::Options,
            from,
            to,
            only: &[],
            store_root: &root.join(store),
        };
        run_nfo(&source, &run).unwrap()
    };
    let bins = |store: &str| -> Vec<String> {
        tree(&root.join(store))
            .into_keys()
            .map(|p| p.to_string_lossy().into_owned())
            .filter(|p| {
                p.starts_with("bars") && Path::new(p).extension().is_some_and(|e| e == "bin")
            })
            .collect()
    };
    // 1. One contract, two days, one series keyed by the decoded contract.
    let (one, two) = ("NIFTY04APR2422000CE", "NIFTY04APR2422000CE");
    let decoded = |t: &str, d: Day| crate::gdfl_nfo::decode_ticker(t, d).unwrap();
    assert_eq!(decoded(one, d1), decoded(two, d2));
    day_store(d1, &[one]);
    day_store(d2, &[two]);
    let got = import(d1, d2, "A");
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    assert_eq!((got.files, got.seconds), (2, 4));
    let held = bins("A");
    assert_eq!(held.len(), 1, "{held:?}");
    assert!(
        held[0].contains(&decoded(one, d1).contract.to_string()),
        "{held:?}"
    );
    assert!(!held[0].contains(one), "no ticker text in the path");
    // 2. The cutover pair: both names decode to one contract, and the
    // listing hands the filing path one (symbol, contract) for both days.
    let (monthly, dated) = ("ACC19FEB1260PE", "ACC28FEB191260PE");
    day_store(jan, &[monthly]);
    day_store(feb, &[dated]);
    let (old, new) = (decoded(monthly, jan), decoded(dated, feb));
    assert_eq!(old, new, "one contract across the rename");
    assert_eq!(old.underlying.as_str(), "ACC");
    assert_eq!(old.contract.as_str(), "2019-02-28-126000-PE");
    let listed = |day: Day, only: &[String]| {
        let run = Run {
            kind: ImportKind::Options,
            from: day,
            to: day,
            only,
            store_root: &root.join("unused"),
        };
        let mut files: Vec<(String, Option<Contract>, String, usize)> = Vec::new();
        let read = nfo_day(&source, &run, day, &mut |file| {
            files.push((file.symbol, file.contract, file.name, file.ticks.len()));
        });
        (files, read)
    };
    let (jan_files, jan_read) = listed(jan, &[]);
    let (feb_files, feb_read) = listed(feb, &["ACC".to_owned()]);
    assert!(jan_read.refused.is_empty() && feb_read.refused.is_empty());
    assert_eq!(jan_files.len(), 1, "{jan_files:?}");
    assert_eq!(feb_files.len(), 1, "{feb_files:?}");
    assert_eq!(
        (&jan_files[0].0, &jan_files[0].1),
        (&feb_files[0].0, &feb_files[0].1),
        "one series key on both sides of the cutover"
    );
    assert_eq!(jan_files[0].1, Some(old.contract));
    assert_ne!(jan_files[0].2, feb_files[0].2, "two vendor names");
    // Each name on the other side of the cutover is refused by name: the
    // dated twin before it reads as the monthly form of February 2028 (no
    // sourced day), the monthly name after it as a dated 2012-02-19.
    assert!(matches!(
        crate::gdfl_nfo::decode_ticker(dated, jan),
        Err(NfoRefusal::MonthlyExpiryUnstated { .. })
    ));
    assert!(matches!(
        crate::gdfl_nfo::decode_ticker(monthly, feb),
        Err(NfoRefusal::ExpiryRefused { .. })
    ));
    day_store(jan, &[monthly, dated]);
    let (both_jan, both_jan_read) = listed(jan, &[]);
    assert_eq!(both_jan.len(), 1, "{both_jan:?}");
    assert_eq!(both_jan[0].2, monthly);
    assert_eq!(both_jan_read.refused.len(), 1);
    assert_eq!(both_jan_read.refused[0].instrument, dated);
    // 4. Two spellings of ONE contract on one day: both refused, no file
    // reaches the sink, so no second holds rows of two names.
    let mid = Day::new(2019, 1, 15).unwrap();
    let pair = ["NIFTY19JAN10500CE", "NIFTY31JAN1910500CE"];
    assert_eq!(decoded(pair[0], mid), decoded(pair[1], mid));
    day_store(mid, &pair);
    let (same_day, same_read) = listed(mid, &[]);
    assert!(same_day.is_empty(), "{same_day:?}");
    assert_eq!(same_read.refused.len(), 2, "{:?}", same_read.refused);
    for (failure, name) in same_read.refused.iter().zip(pair) {
        assert_eq!(failure.instrument, name);
        assert!(
            failure.why.contains("names the same contract"),
            "{failure:?}"
        );
    }
    // And neither day is imported at all today: both lie before the
    // calendar's first measured day (`calendar::FIRST_DAY`, 2019-12-02), so
    // the session gate refuses them by name before any file is read.
    day_store(jan, &[monthly]);
    let cut = import(jan, feb, "B");
    assert_eq!(
        (cut.days_refused, cut.files, cut.seconds),
        (2, 0, 0),
        "{cut:?}"
    );
    assert!(
        cut.failures
            .iter()
            .all(|f| f.instrument.ends_with("calendar")),
        "{:?}",
        cut.failures
    );
    assert!(!root.join("B").join("bars").exists());
    std::fs::remove_dir_all(root).unwrap();
}
