//! A recorded halt's minutes never reach a coarse rung's execution series
//! (AH-01 repair round 2; g-zerodha-universe-census-3).
//!
//! The door under test is `stored_sweep_inputs`, the one `sweep-stored`
//! reads, over a generated store: every bar is `runner::synthetic`'s invented
//! walk on the exchange calendar's own session minutes of March 2020, with
//! every minute of both halts present, as Zerodha's feed holds them (frozen
//! there, moving here: what matters is that no minute is missing).
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

static NEXT: AtomicU64 = AtomicU64::new(0);

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;

fn epoch_day(year: u16, month: u8, date: u8) -> i64 {
    i64::from(
        pull::session::Day::new(year, month, date)
            .expect("date")
            .days_from_epoch(),
    )
}

/// Every session minute of an open day, as invented bars; empty when closed.
fn session(year: u16, month: u8, date: u8, index: i64) -> Vec<Bar> {
    let day = epoch_day(year, month, date);
    let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
        return Vec::new();
    };
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .flat_map(|window| window.from..=window.to)
        .enumerate()
        .map(|(minute_of_session, minute)| {
            let bar = runner::synthetic::bar(index, minute_of_session);
            Bar {
                ts_micros: day * DAY + i64::from(minute) * MINUTE - indicators::IST_OFFSET_MICROS,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
                open_interest: i64::MIN,
            }
        })
        .collect()
}

fn aggregate(rows: &[Bar]) -> Bar {
    Bar {
        ts_micros: rows[0].ts_micros,
        open: rows[0].open,
        high: rows.iter().map(|bar| bar.high).max().expect("rows"),
        low: rows.iter().map(|bar| bar.low).min().expect("rows"),
        close: rows[rows.len() - 1].close,
        volume: rows.iter().map(|bar| bar.volume).sum(),
        open_interest: i64::MIN,
    }
}

struct HaltStore {
    root: PathBuf,
}

impl HaltStore {
    /// 2020-02-28 as warm-up, then every open session of March 2020 through
    /// the 23rd, each with its `1min`, `day` and `rung`-minute files.
    fn new(rungs: &[(Timeframe, usize)]) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-halted-execution-fixture-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("scratch");
        let store = Self { root };
        let mut index = 0_i64;
        let days = std::iter::once((2_u8, 28_u8)).chain((2..=23).map(|date| (3, date)));
        for (month, date) in days {
            let rows = session(2020, month, date, index);
            if rows.is_empty() {
                continue;
            }
            index += 1;
            store.write(month, Timeframe::MINUTE_1, &rows);
            store.write(month, Timeframe::DAY_1, &[aggregate(&rows)]);
            for &(timeframe, length) in rungs {
                store.write(
                    month,
                    timeframe,
                    &rows.chunks(length).map(aggregate).collect::<Vec<_>>(),
                );
            }
        }
        store
    }

    fn write(&self, month: u8, timeframe: Timeframe, rows: &[Bar]) {
        let key = crate::stored::swept_index("NIFTY").expect("a swept fixture key");
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            timeframe,
            YearMonth::new(2020, month).expect("month"),
            FileKind::Bars,
        )
        .expect("path");
        let hash = brutex_core::universe::fnv1a(key.underlying.as_str()).to_le_bytes();
        let symbol = u32::from_le_bytes(hash[..4].try_into().expect("low32"));
        let mut file = BarFile::open_or_create(&self.root, path, symbol).expect("writer");
        file.append(rows).expect("generated fixture rows");
    }

    fn request<'a>(&self, rung: &'a str) -> crate::StoredSweepRequest<'a> {
        crate::StoredSweepRequest {
            root: self.root.clone(),
            vendor: Vendor::Zerodha,
            underlying: "NIFTY",
            rung,
            year: 2020,
            month: 3,
            min_hits: u64::MAX,
            commit: "generated-halted-execution-fixture",
        }
    }
}

/// The halt windows of March 2020 as `(first halted minute open, last)`.
/// THE RUNNER'S OWN PRIMITIVE, on the door's own column and projection
/// (`stored_month_kernel`'s two calls): every swept entry on a halt day meets a
/// located hole at the first minute after the halt, and its path to the day's
/// last minute is refused. Before the repair the halt's minutes were
/// consecutive, so neither held. Returns how many entries it checked.
fn runner_refuses_paths_through_halts(
    inputs: &crate::StoredMonthInputs,
    length: i64,
    windows: &[(i64, i64, i64)],
    rung: &str,
) -> usize {
    let execution = &inputs.execution_bars.as_ref().expect("a coarse rung").bars;
    let column = super::stored_month_column(
        inputs,
        length,
        crate::stored::vwap_availability(&inputs.loaded.key),
    )
    .expect("the door's column");
    let (trade_bars, scoring, _) = super::project_onto_execution(
        &inputs.loaded.bars,
        &column,
        Some(super::Execution {
            bars: execution,
            signal_length_micros: length,
        }),
        false,
        super::Horizon::DEFAULT,
    )
    .expect("the door's projection");
    let facts = runner::trade::SliceFacts::of(&trade_bars, &scoring);
    let mut checked = 0_usize;
    for &entry in scoring.sources() {
        let at = trade_bars[entry].ts_micros;
        let Some(&(day, from, to)) = windows
            .iter()
            .find(|(day, _, _)| indicators::ist_day(at) == *day)
        else {
            continue;
        };
        let end = trade_bars
            .iter()
            .rposition(|bar| indicators::ist_day(bar.ts_micros) == day)
            .expect("the day's last minute");
        assert!(at < from, "{rung}: an entry at or after the halt");
        let broken = facts.first_missing_within(entry, end);
        assert!(broken.is_some(), "{rung}: the path from {at} meets no hole");
        let broken = broken.expect("checked just above");
        assert!(
            trade_bars[broken - 1].ts_micros < from && trade_bars[broken].ts_micros > to,
            "{rung}: the located hole is the halt"
        );
        assert!(
            !facts.path_accepts(entry, end),
            "{rung}: a path through the halt"
        );
        checked += 1;
    }
    checked
}

fn halts() -> Vec<(i64, i64, i64)> {
    [(2020, 3, 13), (2020, 3, 23)]
        .into_iter()
        .map(|(year, month, date)| {
            let day = epoch_day(year, month, date);
            let halt = pull::calendar::halt_on(day).expect("a recorded halt");
            let open = day * DAY - indicators::IST_OFFSET_MICROS;
            (
                day,
                open + i64::from(halt.halted.from) * MINUTE,
                open + i64::from(halt.halted.to) * MINUTE,
            )
        })
        .collect()
}

/// THE BLOCKING FINDING OF AH-01's SECOND RE-CHECK, on the door itself. On
/// every coarse rung the execution series `sweep-stored` walks holds no minute
/// of either March 2020 halt, the exclusion counts the 117 it removed (59 on
/// the 13th, 58 on the 23rd), and every row admitted before a halt projects
/// onto an execution path that breaks before the halt's first minute: the
/// consecutive run from its entry minute ends strictly before the halt, so
/// `runner::trade::SliceFacts::path_accepts` refuses any path that reaches the
/// halt (D-1514's located hole) and no stop, target or exit is priced on a
/// halted minute. Before the repair the series kept all 117 minutes and the
/// run from a 09:20 entry was consecutive through 10:20.
#[test]
fn a_coarse_rung_trade_never_prices_a_halted_minute() {
    let rungs = [
        (Timeframe::MINUTE_2, 2_usize, "2min"),
        (Timeframe::MINUTE_5, 5, "5min"),
        (Timeframe::MINUTE_15, 15, "15min"),
    ];
    let store = HaltStore::new(&rungs.map(|(timeframe, length, _)| (timeframe, length)));
    let windows = halts();
    // Swept entries on a halt day checked against `SliceFacts`, over every rung.
    let mut entries_checked = 0_usize;
    for (_, length, rung) in rungs {
        let inputs = crate::stored_sweep_inputs(&store.request(rung)).expect("inputs");
        let execution = &inputs.execution_bars.as_ref().expect("a coarse rung").bars;
        for &(_, from, to) in &windows {
            assert!(
                execution
                    .iter()
                    .all(|minute| minute.ts_micros < from || minute.ts_micros > to),
                "{rung}: an execution minute inside a halt"
            );
        }
        let gaps = inputs.minute_gaps.as_ref().expect("the ordinary door");
        assert_eq!(
            gaps.minute_bars(),
            59 + 58,
            "{rung}: halted minutes counted"
        );
        let halt_cuts: Vec<_> = gaps
            .cuts()
            .iter()
            .filter(|cut| cut.cause == crate::minute_gaps::CutCause::Halt)
            .collect();
        assert_eq!(halt_cuts.len(), 2, "{rung}: both halts cut the signal");
        let length = i64::try_from(length).expect("length") * MINUTE;
        let signal = &inputs.loaded.bars;
        let sources: Vec<usize> = (0..signal.len()).collect();
        let alignment = runner::align::onto_execution(signal, &sources, execution, length)
            .expect("an alignment");
        let mut pre_halt_rows = 0_usize;
        for (row, onto) in signal.iter().zip(&alignment.onto) {
            let Some(&(_, from, _)) = windows
                .iter()
                .find(|(day, _, _)| indicators::ist_day(row.ts_micros) == *day)
            else {
                continue;
            };
            assert!(row.ts_micros + length <= from, "{rung}: a row past the cut");
            let Some(entry) = *onto else {
                continue;
            };
            pre_halt_rows += 1;
            let mut last = entry;
            while execution
                .get(last + 1)
                .is_some_and(|next| next.ts_micros - execution[last].ts_micros == MINUTE)
            {
                last += 1;
            }
            assert!(
                execution[last].ts_micros < from,
                "{rung}: the path from {} runs into the halt",
                execution[entry].ts_micros
            );
        }
        assert!(
            pre_halt_rows > 0,
            "{rung}: premise: a row admitted before a halt"
        );
        entries_checked += runner_refuses_paths_through_halts(&inputs, length, &windows, rung);
        let mut report = String::new();
        report.push_str(&crate::minute_gaps::halted_minutes_note(gaps.minute_bars()));
        assert!(
            report.contains("117 one-minute bar(s) inside a recorded halt"),
            "{report}"
        );
    }
    assert!(
        entries_checked > 0,
        "premise: a swept entry on a halt day reached the runner's check"
    );
    // `1min` is its own execution series: the halt's cut already withholds
    // every row from its first minute to the session end, so nothing is
    // counted twice and no halted minute is left.
    let inputs = crate::stored_sweep_inputs(&store.request("1min")).expect("1min inputs");
    assert!(inputs.execution_bars.is_none());
    for &(_, from, to) in &windows {
        assert!(
            inputs
                .loaded
                .bars
                .iter()
                .all(|minute| minute.ts_micros < from || minute.ts_micros > to)
        );
    }
}
