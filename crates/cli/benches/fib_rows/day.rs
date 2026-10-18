//! The bench rows of the day input, the authorities, the FIB run files and
//! the reference stamps (FIB-8; D-6440 to D-6451), through FIB-1's harness.
//!
//! | Row | What one call does | Class | Axis |
//! |---|---|---|---|
//! | C-F8-01 | one minute of a 375-minute day: gate, split, fold, closes, verdict — the day's work paid at its close | AMORTISED, period 375 | months held for the symbol |
//! | C-F8-02 | the same on 360-minute closing-auction days | AMORTISED, period 360 | months held |
//! | C-F8-03 | `CalendarAuthority::tradable` (interim) | WORST-CASE | fixed state |
//! | C-F8-04 | `DaySlots::slot_of` (FM-F086) | WORST-CASE | members of the pool |
//! | C-F8-05 | the historical basis verdict (interim inputs) | WORST-CASE | fixed state |
//! | C-F8-06 | every other interim authority's answer | WORST-CASE | fixed state |
//! | C-F8-07 | one candidate record encoded and decoded | WORST-CASE | fixed state |
//! | C-F8-08 | one record read from a sealed FIB file (FM-F091) | EXPECTED | records in the file |
//! | C-F8-09 | one GDFL second looked up (`SecondsDay::at`) | WORST-CASE | days of seconds held |
//! | C-F8-10 | one reference stamp joined (FM-F087) | WORST-CASE | indices in the table |
//!
//! The store read itself (`cli::stored::load`) is not in these rows: it is
//! the store's own cost and its own rows measure it.

use std::fmt::Write as _;
use std::hint::black_box;
use std::path::{Path, PathBuf};

use cli::fib::authorities::{Authorities, BasisVerdictAuthority as _, DaySlots, Member, SeriesId};
use cli::fib::day::{
    MonthSpec, PreviousReference, SecondsMonth, StockState, build_month, doubling_ladder, instant,
    seconds_of,
};
use cli::fib::files::{CandidateRecord, FibFile, Header, Kind, Put, Record as _, Scope};
use cli::fib::reference::{IndexSource, ReferenceMonth, build_reference};
use cli::stored::CashCloses;
use indicators::Candle;

use crate::harness::{Axis, Class, Host, Row, run};

/// A minute of a synthetic day: a range around `price`.
fn minute(day: i64, minute_of_day: u32, price: i64) -> Candle {
    Candle {
        ts_micros: instant(day, minute_of_day * 60),
        open: price,
        high: price + 50,
        low: price - 50,
        close: price + 10,
        volume: 7,
        open_interest: i64::MIN,
    }
}

/// The IST epoch day of a civil date.
fn day_of(year: u16, month: u8, of_month: u8) -> i64 {
    i64::from(
        pull::session::Day::new(year, month, of_month)
            .unwrap()
            .days_from_epoch(),
    )
}

/// One month's tradable days under the interim calendar.
fn tradable_days(authorities: &Authorities, year: u16, month: u8) -> Vec<i64> {
    let first = pull::session::Day::new(year, month, 1).unwrap();
    let from = i64::from(first.days_from_epoch());
    let to = i64::from(first.end_of_month().days_from_epoch());
    (from..=to)
        .filter(|day| authorities.calendar.tradable(SeriesId(0), *day).admits())
        .collect()
}

/// The day input's state: `months` months of synthetic days, each day built
/// when its last minute is reached.
pub(crate) struct DayRun {
    days: Vec<(u16, u8, Vec<Candle>)>,
    authorities: Authorities,
    cash: CashCloses,
    widths: Vec<u16>,
    state: StockState,
    minutes: u64,
    period: u64,
    _store: Option<Scratch>,
}

/// A scratch directory removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str, salt: u64) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let dir = std::env::temp_dir().join(format!(
            "brutex-fib8-bench-{name}-{}-{salt}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The calendar months a 375-minute row cycles through: 2018-01 to 2026-07.
fn regular_months() -> Vec<(u16, u8)> {
    let mut out = Vec::new();
    for year in 2018..=2026_u16 {
        for month in 1..=12_u8 {
            if (year, month) <= (2026, 7) {
                out.push((year, month));
            }
        }
    }
    out
}

fn regular_run(months: u64) -> DayRun {
    let authorities = Authorities::interim(&|_: &str| true).unwrap();
    let calendar = regular_months();
    let mut days = Vec::new();
    for n in 0..months {
        let (year, month) = calendar[usize::try_from(n).unwrap() % calendar.len()];
        for day in tradable_days(&authorities, year, month) {
            let base = 1_000_000 + i64::try_from(n).unwrap() * 37;
            let bars = (555..=929)
                .map(|at| minute(day, at, base + i64::from(at)))
                .collect();
            days.push((year, month, bars));
        }
    }
    let cash = cli::stored::load_cash_closes(
        Path::new("/nonexistent-fib8-bench"),
        "SBIN",
        std::iter::empty(),
    )
    .unwrap();
    DayRun {
        days,
        authorities,
        cash,
        widths: doubling_ladder(),
        state: StockState::first_day(),
        minutes: 0,
        period: 375,
        _store: None,
    }
}

/// 360-minute days of 2026-08 for an eligible share, its masters installed in
/// a scratch store.
fn auction_run(months: u64) -> DayRun {
    use std::io::Write as _;
    let authorities = Authorities::interim(&|_: &str| true).unwrap();
    let store = Scratch::new("cas", months);
    let days_of_month = tradable_days(&authorities, 2026, 8);
    let isin = brutex_core::universe::nse_isin("TECHM").unwrap();
    for day in &days_of_month {
        let csv = format!(
            "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n7777,TECHM,EQ,{},1\n",
            isin.as_str()
        );
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(csv.as_bytes()).unwrap();
        let civil = pull::session::Day::from_days(u32::try_from(*day).unwrap()).unwrap();
        pull::cash_session_cache::install(
            &store.0.join("session-masters"),
            civil,
            &gz.finish().unwrap(),
        )
        .unwrap();
    }
    let cash =
        cli::stored::load_cash_closes(&store.0, "TECHM", days_of_month.iter().copied()).unwrap();
    let mut days = Vec::new();
    for n in 0..months {
        for day in &days_of_month {
            let base = 2_000_000 + i64::try_from(n).unwrap() * 41;
            let bars = (555..=914)
                .map(|at| minute(*day, at, base + i64::from(at)))
                .collect();
            days.push((2026, 8, bars));
        }
    }
    DayRun {
        days,
        authorities,
        cash,
        widths: doubling_ladder(),
        state: StockState::first_day(),
        minutes: 0,
        period: 360,
        _store: Some(store),
    }
}

fn day_op(run: &mut DayRun, i: u64) -> u64 {
    if !i.is_multiple_of(run.period) {
        return run.minutes;
    }
    let at = usize::try_from(i / run.period).unwrap() % run.days.len();
    let (year, month, bars) = &run.days[at];
    let spec = MonthSpec {
        symbol: "SBIN",
        series: SeriesId(1),
        year: *year,
        month: *month,
        widths: &run.widths,
        reference: PreviousReference::TradableDay,
        daily: None,
        rename_conflicts: &[],
    };
    let built = build_month(&spec, bars, &run.cash, &run.authorities, &mut run.state).unwrap();
    black_box(&built);
    run.minutes += u64::try_from(bars.len()).unwrap();
    run.minutes
}

fn day_bytes(run: &DayRun) -> u64 {
    run.days
        .iter()
        .map(|(_, _, bars)| bars.len() * size_of::<Candle>())
        .sum::<usize>() as u64
}

/// The slot tables of 1,000 days of a pool of `members` members whose
/// series ids are spread sixteen apart (a point-in-time pool's ids are
/// sparse), one table a day.
pub(crate) struct Slots {
    days: Vec<DaySlots>,
    probes: u64,
    members: u64,
}

/// Days of slot tables held: enough that the largest pool's tables pass the
/// last-level cache.
const SLOT_DAYS: u64 = 1_000;

fn slots(members: u64) -> Slots {
    let width = u32::try_from(members).unwrap();
    let tables = (0..SLOT_DAYS)
        .map(|day| {
            let pool: Vec<Member> = (0..width)
                .map(|n| Member {
                    series_id: SeriesId(n * 16 + u32::try_from(day % 16).unwrap()),
                    symbol_on_day: String::new(),
                    zerodha_series_present: true,
                    in_current_list: true,
                })
                .collect();
            DaySlots::build(&pool).unwrap()
        })
        .collect();
    Slots {
        days: tables,
        probes: 0,
        members: members * 16,
    }
}

fn slot_op(state: &mut Slots, i: u64) -> u64 {
    let mixed = i.wrapping_mul(2_654_435_761);
    let day = usize::try_from(mixed % state.days.len() as u64).unwrap();
    let series = SeriesId(u32::try_from((mixed >> 20) % state.members).unwrap());
    state.probes += 1;
    u64::from(state.days[day].slot_of(series).unwrap_or(u16::MAX))
}

/// The interim set and a counter.
pub(crate) struct Interim {
    authorities: Authorities,
    days: u64,
    calls: u64,
}

fn interim(days: u64) -> Interim {
    Interim {
        authorities: Authorities::interim(&|_: &str| true).unwrap(),
        days,
        calls: 0,
    }
}

fn first_day() -> i64 {
    day_of(2018, 1, 1)
}

fn calendar_op(state: &mut Interim, i: u64) -> u64 {
    let day = first_day() + i64::try_from(i % state.days).unwrap();
    state.calls += 1;
    u64::from(
        state
            .authorities
            .calendar
            .tradable(SeriesId(1), day)
            .admits(),
    )
}

fn basis_op(state: &mut Interim, i: u64) -> u64 {
    let day = first_day() + i64::try_from(i % state.days).unwrap() + 1;
    state.calls += 1;
    let verdict = state.authorities.basis().verdict(SeriesId(1), day - 1, day);
    u64::from(verdict.outcome.is_ok())
}

fn constants_op(state: &mut Interim, i: u64) -> u64 {
    let day = first_day() + i64::try_from(i % state.days).unwrap();
    let series = SeriesId(1);
    let a = &state.authorities;
    state.calls += 1;
    let factor = a
        .factor
        .factor(series, day)
        .map_or(0, |pair| pair.gdfl_close);
    let seconds =
        u64::from(a.seconds.status(series, day) == cli::fib::authorities::SecondsStatus::Unchecked);
    let flow =
        u64::from(a.order_flow.window(series, day, 0, 60) == vocab::fib_cube::OrderFlowWindow::Off);
    let tail =
        u64::from(a.tail.after(series, day, 0) == cli::fib::authorities::TailReading::Unknown);
    let screen =
        u64::from(a.screen.screen("NIFTY", day) == cli::fib::authorities::IndexDay::Unscreened);
    let expiry = u64::from(matches!(
        a.expiry.expiry(series, day),
        cli::fib::authorities::ExpiryStamp::Absent(_)
    ));
    u64::try_from(factor).unwrap_or(0) + seconds + flow + tail + screen + expiry
}

/// Candidate records cycled through the codec.
pub(crate) struct Codec {
    records: Vec<CandidateRecord>,
    bytes: u64,
}

fn codec(records: u64) -> Codec {
    use indicators::fibflag::types::{BaseKind, BaseSnapshot, Candidate, EntryStyle};
    let records = (0..records)
        .map(|n| CandidateRecord {
            candidate: Candidate {
                stock_slot: u16::try_from(n % 250).unwrap(),
                ist_day: 20_000 + i64::try_from(n).unwrap(),
                tf_minutes: 5,
                decision_close_ts_micros: 1_700_000_000_000_000 + i64::try_from(n).unwrap(),
                kind: BaseKind::Micro2,
                style: EntryStyle::Break,
                direction: vocab::fib_cube::Search::Long,
                base: BaseSnapshot {
                    kind: BaseKind::Micro2,
                    direction: vocab::fib_cube::Search::Long,
                    high: 1_000_100,
                    low: 1_000_000,
                    candles: 2,
                    first_close_ts_micros: 1,
                    last_close_ts_micros: 2,
                },
            },
            coords: [1; vocab::fib_cube::ROW_COUNT],
        })
        .collect();
    Codec { records, bytes: 0 }
}

fn codec_op(state: &mut Codec, i: u64) -> u64 {
    let record = &state.records[usize::try_from(i % state.records.len() as u64).unwrap()];
    let mut out = Put::default();
    record.encode(&mut out);
    let back = CandidateRecord::decode(&out.0).unwrap();
    state.bytes += out.0.len() as u64;
    u64::from(back.candidate.tf_minutes)
}

/// A sealed FIB file of `records` records and an open handle.
pub(crate) struct Sealed {
    handle: FibFile,
    records: u64,
    _dir: Scratch,
}

fn sealed(records: u64) -> Sealed {
    let dir = Scratch::new("file", records);
    let path = dir.0.join("candidates.fib");
    let stride = CandidateRecord::stride();
    let header = Header {
        kind: Kind::Candidates,
        record_version: 1,
        stride,
        schema_digest: [1; 32],
        run_identity: [2; 32],
        scope: Scope::StockMonth {
            series: SeriesId(1),
            year: 2025,
            month: 3,
        },
        profile: Vec::new(),
    };
    let mut probe = Put::default();
    codec(1).records[0].encode(&mut probe);
    let record = probe.0;
    cli::fib::files::write_file(&path, &header, (0..records).map(|_| Ok(record.as_slice())))
        .unwrap();
    Sealed {
        handle: FibFile::open(&path).unwrap(),
        records,
        _dir: dir,
    }
}

fn sealed_op(state: &mut Sealed, i: u64) -> u64 {
    let index = i.wrapping_mul(2_654_435_761) % state.records;
    u64::try_from(state.handle.record(index).unwrap().len()).unwrap()
}

/// `days` days of GDFL seconds, every second of the continuous session held.
pub(crate) struct Seconds {
    month: SecondsMonth,
    days: Vec<i64>,
    lookups: u64,
}

fn seconds(days: u64) -> Seconds {
    let authorities = Authorities::interim(&|_: &str| true).unwrap();
    let mut held = Vec::new();
    for (year, month) in regular_months() {
        for day in tradable_days(&authorities, year, month) {
            if held.len() as u64 == days {
                break;
            }
            held.push(day);
        }
    }
    let bars: Vec<Candle> = held
        .iter()
        .flat_map(|day| {
            (33_300..55_800).map(move |second| Candle {
                ts_micros: instant(*day, second),
                open: 1_000_000,
                high: 1_000_000,
                low: 1_000_000,
                close: 1_000_000,
                volume: 1,
                open_interest: i64::MIN,
            })
        })
        .collect();
    let cash = cli::stored::load_cash_closes(
        Path::new("/nonexistent-fib8-bench"),
        "SBIN",
        std::iter::empty(),
    )
    .unwrap();
    Seconds {
        month: seconds_of(&bars, &held, &cash),
        days: held,
        lookups: 0,
    }
}

fn seconds_op(state: &mut Seconds, i: u64) -> u64 {
    let mixed = i.wrapping_mul(2_654_435_761);
    let day = state.days[usize::try_from(mixed % state.days.len() as u64).unwrap()];
    let second = 33_300 + u32::try_from((mixed >> 16) % 22_500).unwrap();
    state.lookups += 1;
    state
        .month
        .day(day)
        .and_then(|held| held.at(second))
        .map_or(0, |bar| u64::try_from(bar.close).unwrap())
}

/// A month's reference table of `indices` indices.
pub(crate) struct Table {
    table: ReferenceMonth,
    stamps: u64,
}

fn table(indices: u64) -> Table {
    let first = day_of(2025, 3, 1);
    let sources: Vec<(String, IndexSource)> = (0..indices)
        .map(|n| {
            let bars: Vec<Candle> = (0..31_i64)
                .flat_map(|d| {
                    (555..=929).map(move |at| minute(first + d, at, 2_000_000 + i64::from(at)))
                })
                .collect();
            (format!("INDEX{n:04}"), IndexSource::Minutes(bars))
        })
        .collect();
    let authorities = Authorities::interim(&|_: &str| true).unwrap();
    let table = build_reference(
        (2025, 3),
        &sources,
        &std::collections::HashMap::new(),
        authorities.screen.as_ref(),
        authorities.calendar.as_ref(),
        None,
    );
    Table { table, stamps: 0 }
}

fn table_op(state: &mut Table, i: u64) -> u64 {
    let mixed = i.wrapping_mul(2_654_435_761);
    let day = day_of(2025, 3, 1) + i64::try_from(mixed % 31).unwrap();
    let ts = instant(day, (555 + u32::try_from((mixed >> 8) % 375).unwrap()) * 60);
    let slot = usize::try_from((mixed >> 24) % state.table.indices.len() as u64).unwrap();
    state.stamps += 1;
    u64::from(state.table.stamp(ts, slot).is_some())
}

/// The rows, run and reported. False when any row failed.
#[expect(
    clippy::too_many_lines,
    reason = "ten rows, each one literal; split, they would be ten one-call functions"
)]
pub(crate) fn report(host: &Host, out: &mut String) -> bool {
    let mut verdicts = Vec::new();
    let regular = Row {
        id: "C-F8-01",
        name: "one minute of a 375-minute day input (gate, split, fold to the ladder, closes, verdict)",
        class: Some(Class::Amortised { period: 375 }),
        axis: Some(Axis::Data {
            unit: "months held for the symbol (2018-01..2026-07 cycled)",
            production: Some(105),
        }),
        scales: [1, 11, 110],
        setup: regular_run,
        op: day_op,
        work: |run| run.minutes,
        bytes: day_bytes,
    };
    verdicts.push(run(&regular, host, out));
    let auction = Row {
        id: "C-F8-02",
        name: "one minute of a 360-minute closing-auction day input",
        class: Some(Class::Amortised { period: 360 }),
        axis: Some(Axis::Data {
            unit: "months held for the symbol (2026-08 cycled)",
            production: Some(105),
        }),
        scales: [1, 11, 110],
        setup: auction_run,
        op: day_op,
        work: |run| run.minutes,
        bytes: day_bytes,
    };
    verdicts.push(run(&auction, host, out));
    let calendar = Row {
        id: "C-F8-03",
        name: "CalendarAuthority::tradable (interim, H5)",
        class: Some(Class::WorstCase),
        axis: Some(Axis::FixedState {
            why: "fixed calendar tables in pull::calendar",
        }),
        scales: [30, 300, 3_000],
        setup: interim,
        op: calendar_op,
        work: |state| state.calls,
        bytes: |state| state.days,
    };
    verdicts.push(run(&calendar, host, out));
    let pool = Row {
        id: "C-F8-04",
        name: "DaySlots::slot_of (FM-F086)",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "members of the pool (1,000 days of tables held)",
            production: Some(250),
        }),
        scales: [25, 250, 2_500],
        setup: slots,
        op: slot_op,
        work: |state| state.probes,
        bytes: |state| state.days.len() as u64 * state.members * size_of::<Option<u16>>() as u64,
    };
    verdicts.push(run(&pool, host, out));
    let basis = Row {
        id: "C-F8-05",
        name: "the historical basis verdict (single landing, identity factor)",
        class: Some(Class::WorstCase),
        axis: Some(Axis::FixedState {
            why: "two interim authorities answering constants",
        }),
        scales: [30, 300, 3_000],
        setup: interim,
        op: basis_op,
        work: |state| state.calls,
        bytes: |state| state.days,
    };
    verdicts.push(run(&basis, host, out));
    let constants = Row {
        id: "C-F8-06",
        name: "the interim factor, seconds, order-flow, tail, screen and expiry answers",
        class: Some(Class::WorstCase),
        axis: Some(Axis::FixedState { why: "constants" }),
        scales: [30, 300, 3_000],
        setup: interim,
        op: constants_op,
        work: |state| state.calls,
        bytes: |state| state.days,
    };
    verdicts.push(run(&constants, host, out));
    let codec_row = Row {
        id: "C-F8-07",
        name: "one CandidateRecord v1 encoded and decoded",
        class: Some(Class::WorstCase),
        axis: Some(Axis::FixedState {
            why: "one fixed-stride record",
        }),
        scales: [1, 10, 100],
        setup: codec,
        op: codec_op,
        work: |state| state.bytes,
        bytes: |state| state.records.len() as u64 * u64::from(CandidateRecord::stride()),
    };
    verdicts.push(run(&codec_row, host, out));
    let file = Row {
        id: "C-F8-08",
        name: "one record read from a sealed FIB file, its block verified on first touch",
        class: Some(Class::Expected),
        axis: Some(Axis::Data {
            unit: "records in one candidates file",
            production: None,
        }),
        scales: [10_000, 100_000, 1_000_000],
        setup: sealed,
        op: sealed_op,
        work: |state| state.handle.bytes_read(),
        bytes: |state| state.records * u64::from(CandidateRecord::stride()),
    };
    verdicts.push(run(&file, host, out));
    let second = Row {
        id: "C-F8-09",
        name: "SecondsDay::at, one GDFL second",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "days of continuous seconds held",
            production: Some(23),
        }),
        scales: [1, 10, 100],
        setup: seconds,
        op: seconds_op,
        work: |state| state.lookups,
        bytes: |state| {
            state.days.len() as u64
                * 22_500
                * (size_of::<Candle>() + size_of::<Option<u32>>()) as u64
        },
    };
    verdicts.push(run(&second, host, out));
    let reference = Row {
        id: "C-F8-10",
        name: "ReferenceMonth::stamp, one (trade, index) join",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "indices in one month's reference table",
            production: Some(138),
        }),
        scales: [2, 20, 200],
        setup: table,
        op: table_op,
        work: |state| state.stamps,
        bytes: |state| state.table.indices.len() as u64 * 31 * 375 * 24,
    };
    verdicts.push(run(&reference, host, out));
    let _ = writeln!(out);
    verdicts.iter().all(|verdict| !verdict.fails())
}
