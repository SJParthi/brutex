//! The bench rows of the lattice and its scoring pass (FIB-6; D-6400 to
//! D-6410), and the walk's throughput for FIB-12's projection.
//!
//! Harness rows (each op a fixed amount of work, so per-op time over the work
//! counter is the per row-touch time):
//!
//! * `C-F6-01` one node's expansion over 64 contiguous rows of a Zipf-skewed
//!   partition table of 10^4, 10^6 and 2 x 10^7 rows: every free row's counting
//!   pass, its scatter and the prefix scans — the walk's unit of work, on rows
//!   whose cells lie side by side as every child region of a walk does.
//! * `C-F6-02` scoring one mix of 64 rows at eight profiles (four exits, two
//!   sizes) across spans of 10^3, 10^4 and 10^5 days (FM-F050's
//!   `per_mix_cost_fixed_support`, the span axis).
//! * `C-F6-03` the same across pools of 2, 20 and 200 stocks (the pool axis).
//!
//! Then, outside the harness and labelled so: whole walks at 10^4, 10^6 and
//! 2 x 10^7 rows at 1 thread, at this host's worker count and at 12 threads
//! (mixes per second, row-touches per second, scratch bytes per worker), the
//! per-visited-mix time distribution, and the shared traversal at three exit
//! counts (FM-F054). Every input is synthetic.

use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

use runner::fib::lattice::{
    Decision, FREE_ROWS, Node, Partition, RowTable, Schedule, Scratch, Visitor, expand_range, walk,
};
use runner::fib::score::{
    Calendar, CardSink, DayKeys, Gross, OrderedPass, Outcome, Profile, Rungs, Slot, profiles_of,
};
use runner::fib::types::{ExitReason, SlotOutcome, TradeFlags};
use vocab::fib_cube::{
    Coords, CutList, EntryVariants, Generation, GenerationParts, ROW_COUNT, Ratio, RowId, RowKind,
    Search, Source, VariantSet,
};

use crate::harness::{Axis, Class, Host, Row, run};

/// Rows in one bench mix and one bench node.
const SUPPORT: u32 = 64;

/// Exit variants of the bench partition.
const EXITS: u8 = 4;

/// A deterministic generator (splitmix64).
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Zipf weights `1 / (v + 1)` over `n` values, as a cumulative table.
fn zipf_table(n: u8) -> Vec<u64> {
    let mut run = 0;
    (1..=u64::from(n))
        .map(|v| {
            run += (1 << 20) / v;
            run
        })
        .collect()
}

/// A Zipf draw from a cumulative table.
fn zipf(rng: &mut Seeded, table: &[u64]) -> u8 {
    let pick = rng.next() % table.last().copied().unwrap_or(1);
    table.partition_point(|c| *c <= pick) as u8
}

/// The bench generation: three values on every measured row, four mover
/// groups, [`EXITS`] exits, one width.
fn generation() -> Generation {
    let mut cuts = Vec::new();
    for id in RowId::ALL {
        if id.row().source != Source::Measured || id == RowId::MoverRank {
            continue;
        }
        let timeframe = (id == RowId::TimeOfDay).then_some(1);
        let unit = if id == RowId::TimeOfDay { 60 } else { 1 };
        cuts.push(CutList {
            row: id,
            timeframe,
            base_kind: None,
            cuts: vec![Ratio::whole(unit), Ratio::whole(2 * unit)],
        });
    }
    Generation::new(GenerationParts {
        search: Search::Long,
        version: 1,
        birth_step: 0,
        timeframes: vec![1],
        rank_edges: vec![1, 2, 3],
        cuts,
        exit_set: VariantSet {
            version: 1,
            digest: [0x55; 32],
            count: u16::from(EXITS),
        },
        entry_variants: EntryVariants {
            version: 1,
            digest: [0x66; 32],
            styles: vec![0, 1, 2],
        },
        others: Vec::new(),
    })
    .unwrap()
}

/// A Zipf-skewed partition table of `rows` rows: every free row varies, the
/// side over the long search's two values; `per_day` decisions a day spread
/// over `pool` stocks; months of 21 days.
fn table(rows: u64, per_day: u64, pool: u64) -> RowTable {
    let generation = generation();
    let layout = *generation.layout();
    let days = rows.div_ceil(per_day);
    let months = u32::try_from(days.div_ceil(21)).unwrap().max(1);
    let partition = Partition {
        search: Search::Long,
        entry_style: 0,
        timeframe: 0,
        exits: 0..EXITS,
    };
    let mut table = RowTable::new(layout, partition, months).unwrap();
    let free: Vec<(RowId, Vec<u64>)> = RowId::ALL
        .into_iter()
        .filter(|id| id.row().kind != RowKind::Partition && *id != RowId::Side)
        .map(|id| (id, zipf_table(layout.radix(id))))
        .collect();
    let mut rng = Seeded(rows ^ pool);
    let mut id = 0_u64;
    for month in 0..u64::from(months) {
        let first = month * 21 * per_day;
        let last = ((month + 1) * 21 * per_day).min(rows);
        let count = last.saturating_sub(first);
        let mut decisions = Vec::with_capacity(count as usize);
        for n in 0..count {
            let mut cell: Coords = [0; ROW_COUNT];
            for (row, weights) in &free {
                cell[row.index()] = zipf(&mut rng, weights);
            }
            cell[RowId::Side.index()] = if rng.next().is_multiple_of(3) { 3 } else { 0 };
            let slot = (n * pool / count.max(1)) as u32;
            decisions.push(Decision {
                coords: cell,
                slot,
                day: 1_000 + u32::try_from(first / per_day).unwrap() + (rng.next() % 21) as u32,
                second: 33_300 + (rng.next() % 21_000) as u32,
                id,
            });
            id += 1;
        }
        let base = u32::try_from(first).unwrap() * u32::from(EXITS);
        let outcomes: Vec<u32> = (0..count as u32 * u32::from(EXITS))
            .map(|o| base + o)
            .collect();
        table
            .push_month(month as u32, &decisions, &outcomes)
            .unwrap();
    }
    table
}

/// The working-set bytes of a table, per row: its cell, five u32 columns, its
/// id and one u32 per exit.
fn table_bytes(table: &RowTable) -> u64 {
    table.len() as u64 * (size_of::<Coords>() as u64 + 5 * 4 + 8 + 4 * u64::from(EXITS))
}

/// Counts visits; nothing else.
struct Count(u64);

impl Visitor for Count {
    fn visit(&mut self, node: &Node<'_>) {
        self.0 += 1;
        black_box(node.rows().len());
    }

    fn fork(&self) -> Self {
        Self(0)
    }

    fn absorb(&mut self, later: Self) {
        self.0 += later.0;
    }
}

/// `C-F6-01`'s state.
pub(crate) struct Expand {
    table: RowTable,
    scratch: Scratch,
    touches: u64,
}

fn expand_state(rows: u64) -> Expand {
    Expand {
        table: table(rows, 2_000, 208),
        scratch: Scratch::for_rows(SUPPORT as usize),
        touches: 0,
    }
}

fn expand_op(state: &mut Expand, i: u64) -> u64 {
    let rows = state.table.len() as u64 - u64::from(SUPPORT);
    let start = ((i * u64::from(SUPPORT)) % rows) as u32;
    let mut count = Count(0);
    let tally = expand_range(
        &state.table,
        start..start + SUPPORT,
        1,
        &mut state.scratch,
        &mut count,
    )
    .unwrap();
    state.touches += tally.row_touches;
    count.0
}

/// One synthetic priced slot: invented paisa, risk 100 a share.
fn priced(rng: &mut Seeded, second: u32) -> Slot {
    let qty = 1 + rng.next() % 200;
    let entry = 100_000 + i128::from((rng.next() % 50_000) as u32);
    let gain = i128::from((rng.next() % 500) as u32) - 200;
    Slot::Priced(SlotOutcome {
        filled_qty: qty,
        entry_value_paisa: entry * i128::from(qty),
        exit_value_paisa: (entry + gain) * i128::from(qty),
        risk_per_share_paisa: 100,
        first_entry_second: second,
        last_entry_second: second,
        exit_second: second + 1 + (rng.next() % 3_600) as u32,
        exit_reason: ExitReason::Forced,
        mfe_to_exit_paisa: i64::try_from(rng.next() % 400).unwrap(),
        mae_to_exit_paisa: -i64::try_from(rng.next() % 200).unwrap(),
        mfe_to_horizon_paisa: 0,
        mae_to_horizon_paisa: 0,
        flags: TradeFlags::NONE,
    })
}

/// Outcome records for every (row, exit) of a table.
fn outcomes(table: &RowTable) -> Vec<Outcome> {
    let mut rng = Seeded(table.len() as u64);
    (0..table.len() as u32)
        .flat_map(|r| {
            let second = table.row(r).unwrap().second;
            (0..EXITS).map(move |_| second)
        })
        .map(|second| Outcome {
            slots: [priced(&mut rng, second), priced(&mut rng, second)],
        })
        .collect()
}

/// A calendar of every day of a table, with synthetic period keys.
fn calendar(table: &RowTable) -> Calendar {
    let last = table.row(table.len() as u32 - 1).unwrap().day;
    let days = (0..=last - 1_000)
        .map(|d| {
            Some(DayKeys {
                tradable_index: d,
                week: d / 7,
                fortnight: d / 15,
                month: d / 21,
                quarter: d / 63,
                half_year: d / 126,
                year: d / 252,
            })
        })
        .collect();
    Calendar::new(1_000, days).unwrap()
}

/// `C-F6-02` and `C-F6-03`'s state.
pub(crate) struct Score {
    table: RowTable,
    outcomes: Vec<Outcome>,
    calendar: Calendar,
    pass: OrderedPass<Gross>,
    touched: u64,
}

fn score_state(rows: u64, per_day: u64, pool: u64) -> Score {
    let table = table(rows, per_day, pool);
    let outcomes = outcomes(&table);
    let calendar = calendar(&table);
    let profiles: Vec<Profile> = profiles_of(&table);
    let pass = OrderedPass::new(
        &table,
        &profiles,
        Rungs::new(
            vec![Ratio::whole(1), Ratio::whole(3)],
            vec![Ratio::whole(2)],
        )
        .unwrap(),
        vec![Gross; profiles.len()],
    )
    .unwrap();
    Score {
        table,
        outcomes,
        calendar,
        pass,
        touched: 0,
    }
}

fn score_op(state: &mut Score, i: u64) -> u64 {
    let rows = state.table.len() as u64 - u64::from(SUPPORT);
    let start = ((i * u64::from(SUPPORT)) % rows) as usize;
    let block = &state.table.order()[start..start + SUPPORT as usize];
    let cards = state
        .pass
        .score(&state.table, block, &state.outcomes, &state.calendar)
        .unwrap();
    let raw: u64 = cards.iter().map(|card| card.counts.raw).sum();
    state.touched += raw;
    cards[0].counts.trades
}

fn score_bytes(state: &Score) -> u64 {
    table_bytes(&state.table) + state.outcomes.len() as u64 * size_of::<Outcome>() as u64
}

/// One progress line on stderr, so a long stage is visible while the report
/// itself is held until the end.
fn progress(stage: &str) {
    eprintln!("[fib-6 bench] {stage}");
}

/// The harness rows, run and reported. False when any row failed.
fn harness_rows(host: &Host, out: &mut String) -> bool {
    let expand = Row {
        id: "C-F6-01",
        name: "one node's expansion over 64 contiguous rows (every free row: counting pass, scatter, prefix scans)",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "rows in the partition table (Zipf-skewed cells)",
            production: None,
        }),
        scales: [10_000, 1_000_000, 20_000_000],
        setup: expand_state,
        op: expand_op,
        work: |state| state.touches,
        bytes: |state| table_bytes(&state.table),
    };
    let span = Row {
        id: "C-F6-02",
        name: "scoring one mix of 64 rows at 8 profiles, across spans (16 decisions a day)",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "days in the span",
            production: None,
        }),
        scales: [1_000, 10_000, 100_000],
        setup: |days| score_state(days * 16, 16, 208),
        op: score_op,
        work: |state| state.touched,
        bytes: score_bytes,
    };
    let pool = Row {
        id: "C-F6-03",
        name: "scoring one mix of 64 rows at 8 profiles, across pools (10^6 rows)",
        class: Some(Class::WorstCase),
        axis: Some(Axis::Data {
            unit: "stocks in the pool",
            production: Some(208),
        }),
        scales: [2, 20, 200],
        setup: |stocks| score_state(1_000_000, 100, stocks),
        op: score_op,
        work: |state| state.touched,
        bytes: score_bytes,
    };
    let mut ok = true;
    progress("C-F6-01");
    ok &= !run(&expand, host, out).fails();
    progress("C-F6-02");
    ok &= !run(&span, host, out).fails();
    progress("C-F6-03");
    ok &= !run(&pool, host, out).fails();
    ok
}

/// The nearest-rank quantile of a sorted list.
fn nearest(sorted: &[u64], num: usize, den: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (sorted.len() * num).div_ceil(den).max(1);
    sorted[rank - 1]
}

/// Times every visited mix's scoring at eight profiles.
struct Timed<'a> {
    pass: OrderedPass<Gross>,
    outcomes: &'a [Outcome],
    calendar: &'a Calendar,
    times_ns: Vec<u64>,
    supports: Vec<u64>,
}

impl Visitor for Timed<'_> {
    fn visit(&mut self, node: &Node<'_>) {
        let start = Instant::now();
        let cards = self
            .pass
            .score(node.table(), node.rows(), self.outcomes, self.calendar)
            .unwrap();
        black_box(cards.len());
        self.times_ns.push(start.elapsed().as_nanos() as u64);
        self.supports.push(node.support());
    }

    fn fork(&self) -> Self {
        Self {
            pass: self.pass.clone(),
            outcomes: self.outcomes,
            calendar: self.calendar,
            times_ns: Vec::new(),
            supports: Vec::new(),
        }
    }

    fn absorb(&mut self, later: Self) {
        self.times_ns.extend(later.times_ns);
        self.supports.extend(later.supports);
    }
}

/// Whole walks: throughput at 1 thread, this host's workers and 12 threads.
fn throughput(host: &Host, out: &mut String) {
    let _ = writeln!(
        out,
        "\nWALK THROUGHPUT (outside the harness: one run each, wall clock; FIB-12 projects from these)"
    );
    for rows in [10_000_u64, 1_000_000, 20_000_000] {
        progress(&format!("build, {rows} rows"));
        let built = Instant::now();
        let table = table(rows, 2_000, 208);
        let build_ms = built.elapsed().as_millis();
        let min = (rows / 100).max(1);
        let scratch_bytes = rows * FREE_ROWS as u64 * (4 + size_of::<Coords>() as u64);
        let _ = writeln!(
            out,
            "  rows {rows}: table {} bytes, built in {build_ms} ms; floor {min} raw trades (1% of the rows); \
             scratch reserved per worker <= {scratch_bytes} bytes (21 regions x rows x 28 bytes, zeroed lazily)",
            table_bytes(&table)
        );
        let mut threads = vec![1, host.workers, 12];
        threads.dedup();
        for threads in threads {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            let schedule = if threads == 1 {
                Schedule::Sequential
            } else {
                Schedule::Parallel {
                    split_support: (rows / 64).max(256),
                    batch_budget_bytes: 4 << 30,
                }
            };
            progress(&format!("walk, {rows} rows, {threads} threads"));
            let mut count = Count(0);
            let start = Instant::now();
            let walked = pool.install(|| walk(&table, min, schedule, &mut count).unwrap());
            let ns = start.elapsed().as_nanos().max(1);
            let mixes = walked.tally.mixes(usize::from(EXITS));
            let _ = writeln!(
                out,
                "    {threads:>2} threads: {} ms; {} nodes, {} mixes (x{EXITS} exits); {} row-touches; \
                 {} mixes/s; {} row-touches/s; {} ps per row-touch; tasks {}, inline {}",
                ns / 1_000_000,
                walked.tally.nodes(),
                mixes,
                walked.tally.row_touches,
                u128::from(mixes) * 1_000_000_000 / ns,
                u128::from(walked.tally.row_touches) * 1_000_000_000 / ns,
                ns * 1_000 / u128::from(walked.tally.row_touches.max(1)),
                walked.spread.tasks,
                walked.spread.inline_for_budget
            );
        }
        progress(&format!("per-mix times, {rows} rows"));
        if rows <= 1_000_000 {
            let outcomes = outcomes(&table);
            let calendar = calendar(&table);
            let profiles = profiles_of(&table);
            let mut visitor = Timed {
                pass: OrderedPass::new(
                    &table,
                    &profiles,
                    Rungs::new(Vec::new(), Vec::new()).unwrap(),
                    vec![Gross; profiles.len()],
                )
                .unwrap(),
                outcomes: &outcomes,
                calendar: &calendar,
                times_ns: Vec::new(),
                supports: Vec::new(),
            };
            walk(&table, min, Schedule::Sequential, &mut visitor).unwrap();
            let mut per_row: Vec<u64> = visitor
                .times_ns
                .iter()
                .zip(&visitor.supports)
                .map(|(t, s)| t * 1_000 / (s * profiles.len() as u64).max(1))
                .collect();
            per_row.sort_unstable();
            let mut times = visitor.times_ns.clone();
            times.sort_unstable();
            let _ = writeln!(
                out,
                "    per visited mix, scored at {} profiles (sequential): {} mixes; ns per mix p50 {} p99 {} max {}; \
                 ps per (row, profile) p50 {} p99 {} max {}",
                profiles.len(),
                times.len(),
                nearest(&times, 1, 2),
                nearest(&times, 99, 100),
                times.last().copied().unwrap_or(0),
                nearest(&per_row, 1, 2),
                nearest(&per_row, 99, 100),
                per_row.last().copied().unwrap_or(0)
            );
        }
    }
}

/// FM-F054: the shared traversal at three exit counts.
fn shared_exits(out: &mut String) {
    let _ = writeln!(
        out,
        "\nSHARED TRAVERSAL AT THREE EXIT COUNTS (FM-F054; outside the harness, one run each, 100,000 rows, floor 1,000)"
    );
    let base = table(100_000, 2_000, 208);
    for exits in [1_u8, 2, 4] {
        progress(&format!("shared traversal, {exits} exits"));
        let partition = Partition {
            exits: 0..exits,
            ..base.partition().clone()
        };
        let mut table = RowTable::new(*base.layout(), partition, base.months()).unwrap();
        let mut month_rows: Vec<(u32, Vec<Decision>, Vec<u32>)> = Vec::new();
        for r in 0..base.len() as u32 {
            let row = base.row(r).unwrap();
            if month_rows.last().is_none_or(|(m, _, _)| *m != row.month) {
                month_rows.push((row.month, Vec::new(), Vec::new()));
            }
            let (_, decisions, outcomes) = month_rows.last_mut().unwrap();
            decisions.push(Decision {
                coords: *row.coords,
                slot: row.slot,
                day: row.day,
                second: row.second,
                id: row.id,
            });
            outcomes.extend(row.outcomes.iter().take(usize::from(exits)));
        }
        for (month, mut decisions, outcomes) in month_rows {
            let mut paired: Vec<(Decision, &[u32])> = decisions
                .drain(..)
                .zip(outcomes.chunks(usize::from(exits)))
                .collect();
            paired.sort_by_key(|(d, _)| (d.slot, d.id));
            let sorted: Vec<Decision> = paired.iter().map(|(d, _)| *d).collect();
            let flat: Vec<u32> = paired.iter().flat_map(|(_, o)| o.iter().copied()).collect();
            table.push_month(month, &sorted, &flat).unwrap();
        }
        let outcomes = outcomes(&base);
        let calendar = calendar(&table);
        let profiles = profiles_of(&table);
        let mut sink = CardSink::new(
            &table,
            &profiles,
            Rungs::new(Vec::new(), Vec::new()).unwrap(),
            vec![Gross; profiles.len()],
            &outcomes,
            &calendar,
        )
        .unwrap();
        let start = Instant::now();
        let walked = walk(&table, 1_000, Schedule::Sequential, &mut sink).unwrap();
        let ms = start.elapsed().as_millis();
        let _ = writeln!(
            out,
            "  {exits} exits: {ms} ms; {} nodes visited (the same at every exit count), {} mixes scored, {} profiles, {} card bytes",
            walked.tally.nodes(),
            sink.mixes(),
            profiles.len(),
            sink.bytes().len()
        );
    }
}

/// The rows, run and reported. False when any harness row failed.
pub(crate) fn report(host: &Host, out: &mut String) -> bool {
    let ok = harness_rows(host, out);
    throughput(host, out);
    shared_exits(out);
    let _ = writeln!(out);
    ok
}
