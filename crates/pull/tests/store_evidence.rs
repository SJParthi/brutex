//! READ-ONLY MEASUREMENTS OF WP-13b AGAINST A STORE THAT HOLDS REAL MINUTES.
//!
//! Ignored by default: it needs a store root, named by the environment
//! variable `BRUTEX_EVIDENCE_STORE`, and it reads only. Each case opens one
//! stored Zerodha one-minute month with `BarFile::open_existing` (which
//! creates nothing, not even a lock), replays its bars as a Zerodha minute
//! answer through `pull::ingest::from_window` into a SCRATCH store under the
//! temp directory, and compares what this build derives and records there
//! with what the given store holds. Nothing is written beside the given
//! store, and the scratch store is removed afterwards; no row is printed,
//! only counts.
//!
//! Run: `BRUTEX_EVIDENCE_STORE=<root> cargo test -p pull --test store_evidence
//! -- --ignored --nocapture`.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use pull::session::{Day, IST_OFFSET_SECS, Window};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

fn store_root() -> PathBuf {
    PathBuf::from(
        std::env::var_os("BRUTEX_EVIDENCE_STORE")
            .expect("BRUTEX_EVIDENCE_STORE names the store root to read"),
    )
}

/// Every bar of one stored month, or none when the month file is absent.
fn read_month(
    root: &Path,
    segment: &str,
    symbol: &str,
    tf: Timeframe,
    month: YearMonth,
) -> Vec<store::format::Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment,
        symbol,
        contract: None,
        timeframe: tf,
        month,
        file: FileKind::Bars,
    })
    .unwrap();
    let id = u32::try_from(brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX)).unwrap();
    let Ok(file) = store::file::BarFile::open_existing(root, path, id) else {
        return Vec::new();
    };
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

/// Bars of `bars` on `day`.
fn on_day(day: Day, bars: &[store::format::Bar]) -> usize {
    let midnight = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
    bars.iter()
        .filter(|b| (midnight..midnight + 86_400_000_000).contains(&b.ts_micros))
        .count()
}

/// What replaying one stored minute month through this build gives.
struct Replay {
    scratch: PathBuf,
    done: pull::ingest::Ingested,
    ledger: String,
}

impl Drop for Replay {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.scratch);
    }
}

fn replay(root: &Path, segment: &str, symbol: &str, month: YearMonth) -> Replay {
    let minutes = read_month(root, segment, symbol, Timeframe::MINUTE_1, month);
    assert!(
        !minutes.is_empty(),
        "{segment} {symbol} {month}: no stored minutes"
    );
    let rows: Vec<pull::fetch::RawRow> = minutes
        .iter()
        .map(|b| pull::fetch::RawRow {
            timestamp: b.ts_micros.div_euclid(1_000_000),
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
            open_interest: None,
        })
        .collect();
    let first = Day::new(month.year(), month.month(), 1).unwrap();
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: if segment == "INDEX" {
            pull::vendor::Listing::Index
        } else {
            pull::vendor::Listing::Equity
        },
        window: Window::new(first, first.end_of_month()).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    };
    let plan = pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment,
        contract: None,
    };
    let scratch = std::env::temp_dir().join(format!(
        "brutex-EVIDENCE-{symbol}-{month}-{}",
        std::process::id()
    ));
    let _fresh = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).unwrap();
    let done = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows },
        symbol,
        "STORE EVIDENCE",
        &scratch,
        plan,
    );
    let ledger = gap_ledger(&scratch);
    Replay {
        scratch,
        done,
        ledger,
    }
}

/// The gap ledger a replay wrote under `scratch`, refused by name when it is
/// absent or unreadable. It was read with `unwrap_or_default`, so a replay
/// that wrote no ledger read as one that named no gap, and every check below
/// it passed (AF-07, law-7; `CLAUDE.md` section 4: no fallback that hides a
/// failure).
fn gap_ledger(scratch: &Path) -> String {
    let path = scratch.join("audit").join("zerodha.gaps");
    fs::read_to_string(&path).unwrap_or_else(|why| {
        panic!(
            "the replay wrote no readable gap ledger at {}: {why}",
            path.display()
        )
    })
}

/// A ledger that is not there is a failure naming its file, never an empty
/// ledger (AF-07, law-7). Runs without a store: an empty scratch directory is
/// a replay that wrote nothing.
#[test]
#[should_panic(expected = "zerodha.gaps")]
fn a_missing_gap_ledger_fails_naming_its_file() {
    let scratch =
        std::env::temp_dir().join(format!("brutex-EVIDENCE-NO-LEDGER-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    // The empty directory goes when the panic unwinds; nothing else is in it.
    let _gone = Emptied(scratch.clone());
    let _never = gap_ledger(&scratch);
}

/// Removes an empty directory when dropped, a panic's unwinding included.
struct Emptied(PathBuf);

impl Drop for Emptied {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir(&self.0);
    }
}

/// What the gap ledger owes one day of a replayed month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owed {
    /// The day's stored minutes fall short of the session it owes: one line.
    Short { owed: u16, stored: usize },
    /// A traded session whose length no record states: the replay withholds
    /// the day and writes one `why=withheld_unmeasured_session` line for it.
    Unsized { stored: usize },
}

/// The days of `month` the gap ledger must name, judged by the calendar the
/// replay is planned with: `pull::calendar::Runtime::default()`, with a
/// special session's recorded window in place of an unsized answer
/// (`pull::session::with_recorded_window`, D-3319), as `request_minutes`
/// plans each day. A day that is short of what it owes is `Short`; a day of a
/// traded session no record sizes is `Unsized`; a day the calendar cannot
/// attest at all (outside the record) owes no line and is not listed. Until
/// AF-07's repair this read the calendar without the recorded window, so a
/// Muhurat of 2020 to 2024 was skipped and its own gap line never asserted.
fn owed_days(month: YearMonth, minutes: &[store::format::Bar]) -> Vec<(Day, Owed)> {
    let first = Day::new(month.year(), month.month(), 1).unwrap();
    let calendar = pull::calendar::Runtime::default();
    let mut out = Vec::new();
    let mut day = first;
    while day.month() == first.month() {
        let at = i64::from(day.days_from_epoch());
        let stored = on_day(day, minutes);
        match pull::session::with_recorded_window(at, calendar.kind_of(at)) {
            pull::calendar::DayKind::Open(session) if stored < usize::from(session.bars()) => {
                out.push((
                    day,
                    Owed::Short {
                        owed: session.bars(),
                        stored,
                    },
                ));
            }
            pull::calendar::DayKind::OpenLengthUnmeasured => {
                out.push((day, Owed::Unsized { stored }));
            }
            pull::calendar::DayKind::Open(_)
            | pull::calendar::DayKind::Closed
            | pull::calendar::DayKind::Unmeasured => {}
        }
        day = day.succ().unwrap();
    }
    out
}

/// The ledger's `gap` lines naming `key` on `day`.
fn gap_lines<'a>(ledger: &'a str, key: &str, day: Day) -> Vec<&'a str> {
    let prefix = format!("gap {key} {day} ");
    ledger
        .lines()
        .filter(|line| line.starts_with(&prefix))
        .collect()
}

fn derive_line<'a>(ledger: &'a str, rung: &str) -> &'a str {
    ledger
        .lines()
        .find(|line| line.starts_with("derive ") && line.contains(&format!(" {rung} ")))
        .unwrap_or("(no derive line)")
}

/// DB-013 / D-3321: the disaster-recovery Saturday derives at every rung.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn banknifty_2024_03_02_derives_from_its_stored_minutes() {
    let root = store_root();
    let month = YearMonth::new(2024, 3).unwrap();
    let saturday = Day::new(2024, 3, 2).unwrap();
    let stored_minutes = read_month(&root, "INDEX", "BANKNIFTY", Timeframe::MINUTE_1, month);
    let replay = replay(&root, "INDEX", "BANKNIFTY", month);
    eprintln!(
        "BANKNIFTY 2024-03-02 1min stored: {}",
        on_day(saturday, &stored_minutes)
    );
    for tf in [
        Timeframe::MINUTE_2,
        Timeframe::MINUTE_5,
        Timeframe::MINUTE_15,
        Timeframe::MINUTE_60,
    ] {
        let stored = read_month(&root, "INDEX", "BANKNIFTY", tf, month);
        let derived = read_month(&replay.scratch, "INDEX", "BANKNIFTY", tf, month);
        eprintln!(
            "BANKNIFTY 2024-03 {}: stored {} bars ({} on 03-02); this build derives {} ({} on 03-02)",
            tf.as_str(),
            stored.len(),
            on_day(saturday, &stored),
            derived.len(),
            on_day(saturday, &derived)
        );
    }
    eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
    let derived = read_month(
        &replay.scratch,
        "INDEX",
        "BANKNIFTY",
        Timeframe::MINUTE_5,
        month,
    );
    assert_eq!(on_day(saturday, &stored_minutes), 105);
    assert_eq!(on_day(saturday, &derived), 21);
}

/// DB-G12 / DB-G35 / D-3346: NIFTY 2026-09 derives every session, and the
/// month's line says so.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn nifty_2026_09_derives_every_session() {
    let root = store_root();
    let month = YearMonth::new(2026, 9).unwrap();
    let replay = replay(&root, "INDEX", "NIFTY", month);
    for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
        let stored = read_month(&root, "INDEX", "NIFTY", tf, month);
        let derived = read_month(&replay.scratch, "INDEX", "NIFTY", tf, month);
        eprintln!(
            "NIFTY 2026-09 {}: stored {} bars, last close {}; this build derives {}, last close {}",
            tf.as_str(),
            stored.len(),
            stored.last().map_or(0, |b| b.close),
            derived.len(),
            derived.last().map_or(0, |b| b.close)
        );
    }
    eprintln!("ledger: {}", derive_line(&replay.ledger, "5min"));
    eprintln!("failures: {}", replay.done.failures.len());
    for failure in replay.done.failures.iter().take(5) {
        eprintln!("  {}", failure.why.chars().take(240).collect::<String>());
    }
    assert!(derive_line(&replay.ledger, "5min").contains(" days=21 "));
}

/// DB-012 / DB-G33 / DB-G37: in a stored month around a Muhurat, every day
/// the calendar and the stored minutes say the ledger owes ([`owed_days`]: a
/// day short of its session, the Muhurat's recorded window included, and a
/// traded session no record sizes) is named by exactly one `gap` line of the
/// ledger the replay writes, and no other day is named at all. The ledger
/// must exist: a replay that wrote none fails naming the file
/// ([`gap_ledger`]). It printed the lines and asserted nothing until AF-07
/// (law-7). Prints counts and the ledger's own lines, never a price.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn the_gap_ledger_names_each_short_day_of_a_stored_month() {
    let root = store_root();
    let calendar = pull::calendar::Runtime::default();
    let mut checked = 0_usize;
    let mut special = Vec::new();
    for (symbol, year, month) in [("NIFTY", 2020, 11), ("NIFTY", 2024, 11), ("NIFTY", 2021, 2)] {
        let ym = YearMonth::new(year, month).unwrap();
        let minutes = read_month(&root, "INDEX", symbol, Timeframe::MINUTE_1, ym);
        let replay = replay(&root, "INDEX", symbol, ym);
        let key = format!("NSE-INDEX-{symbol}");
        let owed = owed_days(ym, &minutes);
        let gaps = replay
            .ledger
            .lines()
            .filter(|line| line.starts_with(&format!("gap {key} ")))
            .count();
        eprintln!(
            "{symbol} {ym}: {} owed day(s), {gaps} gap line(s), {} receipt failure(s)",
            owed.len(),
            replay.done.failures.len()
        );
        for (day, what) in &owed {
            let named = gap_lines(&replay.ledger, &key, *day);
            eprintln!("  {day} {what:?}: {named:?}");
            assert_eq!(
                named.len(),
                1,
                "{symbol} {day} ({what:?}) is named by the gap ledger {} time(s), not once",
                named.len()
            );
            if let Owed::Unsized { .. } = what {
                assert!(
                    named[0].contains(" why=withheld_unmeasured_session "),
                    "{symbol} {day} is a session no record sizes, and its line says otherwise: {}",
                    named[0]
                );
            }
            let at = i64::from(day.days_from_epoch());
            let regular = matches!(
                calendar.kind_of(at),
                pull::calendar::DayKind::Open(session)
                    if session == pull::calendar::Session::full()
            );
            if !regular {
                special.push(*day);
            }
            checked += 1;
        }
        // NO OTHER DAY: a line naming a day the ledger does not owe is as
        // wrong as a missing one.
        let mut day = Day::new(year, month, 1).unwrap();
        while day.month() == month {
            let named = gap_lines(&replay.ledger, &key, day);
            let expected = usize::from(owed.iter().any(|(owed_day, _)| *owed_day == day));
            assert_eq!(
                named.len(),
                expected,
                "{symbol} {day} is named by {} gap line(s); the calendar and the stored minutes \
                 owe {expected}: {named:?}",
                named.len()
            );
            day = day.succ().unwrap();
        }
        eprintln!("  {}", derive_line(&replay.ledger, "5min"));
    }
    // NOT VACUOUS: the three months were chosen because they hold short days,
    // and the first two because each holds a Muhurat, whose own line is
    // checked above (it was skipped before AF-07's repair).
    eprintln!("{checked} day(s) checked; special sessions among them: {special:?}");
    assert!(checked > 0, "no owed day in three months chosen for theirs");
    for year in [2020, 2024] {
        assert!(
            special
                .iter()
                .any(|day| day.year() == year && day.month() == 11),
            "no special session of {year}-11 was among the checked days: {special:?}"
        );
    }
}

/// DB-G36 / D-3321: a share's no-trade minutes fold into partial buckets.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn an_illiquid_share_folds_its_no_trade_minutes_into_partial_buckets() {
    let root = store_root();
    let month = YearMonth::new(2023, 2).unwrap();
    let replay = replay(&root, "CASH", "360ONE", month);
    for tf in [Timeframe::MINUTE_5, Timeframe::MINUTE_60] {
        let stored = read_month(&root, "CASH", "360ONE", tf, month);
        let derived = read_month(&replay.scratch, "CASH", "360ONE", tf, month);
        eprintln!(
            "360ONE 2023-02 {}: stored {} bars; this build derives {}",
            tf.as_str(),
            stored.len(),
            derived.len()
        );
        eprintln!("  {}", derive_line(&replay.ledger, tf.as_str()));
    }
    let gaps = replay
        .ledger
        .lines()
        .filter(|l| l.starts_with("gap "))
        .count();
    eprintln!(
        "360ONE 2023-02: {gaps} gap line(s); failures {}",
        replay.done.failures.len()
    );
    let stored = read_month(&root, "CASH", "360ONE", Timeframe::MINUTE_60, month);
    let derived = read_month(
        &replay.scratch,
        "CASH",
        "360ONE",
        Timeframe::MINUTE_60,
        month,
    );
    assert!(derived.len() > stored.len());
}

/// The cost of the derive fold this build runs per rung-month, measured on a
/// stored month: `derive_minutes` (the aggregated form the derive calls)
/// against `complete_minutes_with_calendar` (the detailed form it called),
/// 200 calls each at five minutes. Run in release for a number worth quoting.
#[test]
#[ignore = "reads the store named by BRUTEX_EVIDENCE_STORE"]
fn the_derive_fold_costs_per_month() {
    let root = store_root();
    for (segment, symbol, year, month) in [("INDEX", "NIFTY", 2026, 9), ("CASH", "360ONE", 2023, 2)]
    {
        let ym = YearMonth::new(year, month).unwrap();
        let minutes = read_month(&root, segment, symbol, Timeframe::MINUTE_1, ym);
        let venue = if segment == "INDEX" {
            pull::vendor::Venue::NseIndex
        } else {
            pull::vendor::Venue::NseCash
        };
        let bucket = pull::fold::Bucket::of_secs(300).unwrap();
        let runtime = pull::calendar::Runtime::default();
        let mut aggregated = Vec::with_capacity(200);
        let mut detailed = Vec::with_capacity(200);
        for _ in 0..200 {
            let start = std::time::Instant::now();
            let derived =
                pull::fold::derive_minutes(&minutes, bucket, venue, None, runtime).unwrap();
            aggregated.push(start.elapsed());
            std::hint::black_box(derived);
            let start = std::time::Instant::now();
            let folded =
                pull::fold::complete_minutes_with_calendar(&minutes, bucket, venue, None, runtime)
                    .unwrap();
            detailed.push(start.elapsed());
            std::hint::black_box(folded);
        }
        aggregated.sort_unstable();
        detailed.sort_unstable();
        eprintln!(
            "{symbol} {ym} 5min fold of {} minutes, n=200: derive_minutes p50 {:?} p99 {:?}; complete_minutes_with_calendar p50 {:?} p99 {:?}",
            minutes.len(),
            aggregated[99],
            aggregated[197],
            detailed[99],
            detailed[197]
        );
        assert!(!minutes.is_empty());
    }
}
