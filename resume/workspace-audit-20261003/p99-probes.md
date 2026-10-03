# p99 probe sources (were crates/*/tests/zz_p99_*.rs, never committed). Copy back to rerun.

## p99/api.rs
```rust

use api::census::{Census, Series, VendorCensus, held_page};
use api::server::param;
use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

#[test]
fn p99_store_held_page() {
    // 248,000 held entries: the scale figure census.rs's own cost note uses.
    let n = 248_000usize;
    let symbols = ["NIFTY", "BANKNIFTY", "RELIANCE", "TCS", "INFY", "HDFCBANK", "SBIN", "ITC"];
    let entries: Vec<(Series, YearMonth)> = (0..n)
        .map(|i| {
            let series = Series {
                contract: None,
                exchange: Exchange::Nse,
                segment: Segment::Cash,
                symbol: Symbol::new(symbols[i % symbols.len()]).unwrap(),
                timeframe: Timeframe::MINUTE_1,
            };
            let m = (i / symbols.len()) % 600;
            (series, YearMonth::new(2000 + (m / 12) as u16, (m % 12) as u8 + 1).unwrap())
        })
        .collect();
    // Absent censuses: rows_for answers None without a probe, so this times
    // the paging walk and the per-row Vec, not a manifest hash probe.
    let censuses: Vec<VendorCensus> = [Vendor::Groww, Vendor::Zerodha]
        .into_iter()
        .map(|vendor| VendorCensus { vendor, path: "/nonexistent".into(), state: Census::Absent })
        .collect();
    let take = 200usize;
    for (label, skip) in [("offset 0 of 248,000", 0usize), ("offset 247,800 of 248,000", n - take)] {
        let v = per_call(200, SAMPLES, || {
            black_box(held_page(black_box(&entries), black_box(&censuses), black_box(skip), take));
        });
        row("api::census::held_page (200 rows)", label, v);
    }
}

#[test]
fn p99_query_param() {
    let small = "q=NIFTY&page=3".to_owned();
    let mut big = String::with_capacity(8192);
    let mut i = 0;
    while big.len() < 8192 - 64 {
        big.push_str(&format!("f{i}=abcdefghij%20klm&"));
        i += 1;
    }
    let big_last = format!("{big}q=NI%46TY&page=3");
    let big_first = format!("page=3&q=NIFTY&{big}");
    for (label, raw) in [
        (format!("{} B, key last", small.len()), &small),
        (format!("{} B, key first", big_first.len()), &big_first),
        (format!("{} B, key last", big_last.len()), &big_last),
    ] {
        let v = per_call(500, SAMPLES, || {
            black_box(param(black_box(raw), black_box("page")));
        });
        row("api::server::param", &label, v);
    }
}
```

## p99/cli.rs
```rust

use cli::results::{Record, Results, field};
use std::io::Write;

fn record(i: u64) -> Record {
    let mut identity = [0u8; 32];
    identity[..8].copy_from_slice(&i.to_le_bytes());
    identity[8..16].copy_from_slice(&i.wrapping_mul(0x9E37_79B9_7F4A_7C15).to_le_bytes());
    identity[31] = 0xA5;
    Record {
        identity,
        finished_micros: 1_717_180_200_000_000 + i as i64,
        feed: field("zerodha"),
        underlying: field("NIFTY"),
        timeframe: field("5min"),
        from_year: 2024,
        from_month: 1,
        to_year: 2024,
        to_month: 6,
        months_asked: 6,
        months_found: 6,
        bars: 10_000,
        min_hits: 30,
        combinations: 1_000,
        depth: 3,
        halted: 0,
        trades: 40,
        pessimistic: 1_000,
        optimistic: 2_000,
        worst_trade: -500,
        max_drawdown: -900,
        winner_mae: 100,
        winner_mfe: 200,
        all_mae: 300,
        exit_rungs: [-1; 5],
        mask_words: [i, 0, 0, 0, 0, 0],
    }
}

/// A ledger with `n` committed runs. The first is appended through the real
/// API (it writes and fsyncs the header); the rest are written as raw sealed
/// records in one write, so setup does not pay n fsyncs.
fn ledger(name: &str, n: u64) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("brutex-p99-cli-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    {
        let mut r = Results::open(&root).unwrap();
        r.append(&record(0)).unwrap();
    }
    let mut bytes = Vec::with_capacity(n as usize * 261);
    for i in 1..n {
        bytes.extend_from_slice(&record(i).to_bytes());
    }
    let mut f = std::fs::OpenOptions::new().append(true).open(Results::path(&root)).unwrap();
    f.write_all(&bytes).unwrap();
    f.sync_all().unwrap();
    root
}

#[test]
fn p99_results_append_and_open() {
    for (label, n) in [("1,000 existing runs", 1_000u64), ("50,000 existing runs", 50_000u64)] {
        let root = ledger(if n == 1_000 { "small" } else { "large" }, n);

        // Results::open: a full scan (known O(n)); 200 opens is enough to
        // see its distribution and keeps the run bounded.
        let v = per_call(5, 200, || {
            let r = Results::open(black_box(&root)).unwrap();
            assert_eq!(r.len().unwrap(), n);
            black_box(r);
        });
        row("cli::results::Results::open (full scan, n=200 calls)", label, v);

        let mut r = Results::open(&root).unwrap();
        let mut next = n;
        let v = per_call(50, SAMPLES, || {
            r.append(black_box(&record(next))).unwrap();
            next += 1;
        });
        row("cli::results::Results::append (sync_all per call)", label, v);
        drop(r);
        let _ = std::fs::remove_dir_all(&root);
    }
}
```

## p99/common.rs
```rust
#![allow(
    missing_docs,
    unreachable_pub,
    clippy::all,
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::float_arithmetic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
// p99 latency probe. Not a gate: it prints `P99ROW|...` lines for a human.
// Per-call times use std::time::Instant around ONE call unless the op label
// says `[batch=K]`, in which case K calls were timed together and the value is
// batch/K (one call is below the timer's own ~20-40 ns resolution).

use std::hint::black_box;
use std::time::Instant;

/// Timed samples per (operation, size).
const SAMPLES: usize = 10_000;

fn row(op: &str, size: &str, mut v: Vec<f64>) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    let q = |p: f64| v[(((n - 1) as f64) * p).round() as usize];
    println!(
        "P99ROW|{op}|{size}|{n}|{:.1}|{:.1}|{:.1}|{:.1}",
        q(0.50),
        q(0.99),
        q(0.999),
        v[n - 1]
    );
}

/// One Instant pair around each call.
fn per_call<F: FnMut()>(warm: usize, n: usize, mut f: F) -> Vec<f64> {
    for _ in 0..warm {
        f();
    }
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let t = Instant::now();
        f();
        out.push(t.elapsed().as_nanos() as f64);
    }
    out
}

/// One Instant pair around K calls; the sample is the per-call mean of the batch.
fn batched<F: FnMut()>(warm: usize, n: usize, k: usize, mut f: F) -> Vec<f64> {
    for _ in 0..warm * k {
        f();
    }
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let t = Instant::now();
        for _ in 0..k {
            f();
        }
        out.push(t.elapsed().as_nanos() as f64 / k as f64);
    }
    out
}

#[allow(dead_code)]
fn keep<T>(x: T) -> T {
    black_box(x)
}
```

## p99/core.rs
```rust

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::universe::{FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS};

#[test]
fn p99_is_sweepable() {
    const K: usize = 256;
    let shares: Vec<&str> = FNO_UNDERLYINGS
        .iter()
        .copied()
        .filter(|s| !FNO_INDEX_UNDERLYINGS.contains(s))
        .collect();
    let first = *shares.first().unwrap();
    let last = *shares.last().unwrap();
    let cases = [
        (format!("first share {first}"), InstrumentKey::cash(Exchange::Nse, first).unwrap()),
        (format!("last share {last}"), InstrumentKey::cash(Exchange::Nse, last).unwrap()),
        ("index NIFTY".to_owned(), InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()),
        ("miss cash ZZZZZZ".to_owned(), InstrumentKey::cash(Exchange::Nse, "ZZZZZZ").unwrap()),
    ];
    for (label, key) in &cases {
        let mut acc = 0u64;
        let v = batched(100, SAMPLES * 10, K, || {
            acc += u64::from(black_box(key).is_sweepable());
        });
        black_box(acc);
        row(&format!("core::InstrumentKey::is_sweepable [batch={K}]"), label, v);
    }
}
```

## p99/engine.rs
```rust

use engine::column::Column;
use vocab::ConditionMask;
use std::collections::HashSet;

fn rows(n: usize) -> Vec<ConditionMask> {
    let mut s = 99u64;
    let mut next = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        s
    };
    (0..n)
        .map(|_| ConditionMask::from_words([next(), next(), next(), next(), next(), next()]))
        .collect()
}

#[test]
fn p99_column_support_per_bar() {
    let cand = ConditionMask::ZERO.with_bit(3).with_bit(70);
    // 1k bars: 10,000 whole-column calls. 1M bars: 2,000 whole-column calls
    // (each ~1 ms; 10,000 would be ~10 s of a busy box, 2,000 keeps it bounded).
    for (label, n, calls) in [("1k bars", 1_000usize, SAMPLES), ("1M bars", 1_000_000usize, 2_000usize)] {
        let col = Column::from_rows(&rows(n));
        let mut acc = 0u64;
        let v = per_call(20, calls, || {
            acc = acc.wrapping_add(black_box(&col).support(black_box(&cand)));
        });
        black_box(acc);
        let per_bar: Vec<f64> = v.iter().map(|ns| ns / n as f64).collect();
        row("engine::Column::support per BAR (call/bars)", label, per_bar);
        row("engine::Column::support whole call", label, v);
    }
}

#[test]
fn p99_k1_dedup_offer() {
    const K: usize = 8;
    for (label, width) in [("100 offered", 100usize), ("100,000 offered", 100_000usize)] {
        // Positions are distinct, scrambled u32s; each round is a fresh set
        // pre-sized to `width` (the production shape), built outside timing.
        let positions: Vec<u32> = (0..width as u32)
            .map(|i| i.wrapping_mul(2_654_435_761))
            .collect();
        let rounds = SAMPLES.div_ceil(width / K) + 1;
        let mut samples = Vec::with_capacity(SAMPLES + width);
        for r in 0..rounds {
            let mut set: HashSet<u32> = HashSet::with_capacity(width);
            let mut chunks = positions.chunks_exact(K);
            for chunk in &mut chunks {
                let t = std::time::Instant::now();
                for &p in chunk {
                    black_box(engine::primitives::offer(&mut set, black_box(p)));
                }
                if r > 0 {
                    samples.push(t.elapsed().as_nanos() as f64 / K as f64);
                }
            }
            black_box(set.len());
        }
        row(&format!("engine::primitives::offer k=1 dedup [batch={K}]"), label, samples);
        // A duplicate offer (the reject path) against the full set.
        let mut set: HashSet<u32> = HashSet::with_capacity(width);
        for &p in &positions {
            engine::primitives::offer(&mut set, p);
        }
        let mut i = 0usize;
        let v = batched(100, SAMPLES, K, || {
            black_box(engine::primitives::offer(&mut set, black_box(positions[i % width])));
            i += 1;
        });
        row(&format!("engine::primitives::offer duplicate reject [batch={K}]"), label, v);
    }
}
```

## p99/pull.rs
```rust

use pull::calendar::{self, DayKind, FIRST_DAY, LAST_DAY};

fn open_day(range: impl Iterator<Item = i64>) -> i64 {
    for d in range {
        if matches!(calendar::kind_of(d), DayKind::Open(_)) {
            return d;
        }
    }
    panic!("no open day");
}

#[test]
fn p99_calendar_kind_of() {
    const K: usize = 256;
    let early = open_day(FIRST_DAY..=LAST_DAY);
    let late = open_day((FIRST_DAY..=LAST_DAY).rev());
    let closed = (FIRST_DAY..=LAST_DAY)
        .rev()
        .find(|&d| matches!(calendar::kind_of(d), DayKind::Closed))
        .unwrap();
    for (label, day) in [
        (format!("early open day {early}"), early),
        (format!("late open day {late}"), late),
        (format!("late closed day {closed}"), closed),
        ("outside table".to_owned(), LAST_DAY + 1000),
    ] {
        let mut acc = 0u64;
        let v = batched(100, SAMPLES * 10, K, || {
            acc += u64::from(matches!(calendar::kind_of(black_box(day)), DayKind::Open(_)));
        });
        black_box(acc);
        row(&format!("pull::calendar::kind_of [batch={K}]"), &label, v);
    }
}
```

## p99/runner.rs
```rust

use vocab::ConditionMask;
use vocab::expression::{Expression, MAX_INSTRUCTIONS};
use vocab::table;

fn len_of(e: &Expression) -> usize {
    let enc = e.encode();
    usize::from(u16::from_le_bytes([enc[2], enc[3]]))
}

fn bars() -> Vec<ConditionMask> {
    let mut s = 5u64;
    let mut next = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        s
    };
    (0..4096)
        .map(|_| ConditionMask::from_words([next(), next(), next(), next(), next(), next()]))
        .collect()
}

/// The longest program the wire format admits: 576 leaves and 575 joins =
/// 1,151 instructions, alternating AND/OR over single-digit live bits.
fn longest() -> Expression {
    let live: Vec<u16> = (0..10u16).filter(|&b| table::is_live(b)).collect();
    let mut src = String::new();
    for i in 0..576usize {
        if i > 0 {
            src.push(if i % 3 == 0 { '|' } else { '&' });
        }
        src.push_str(&live[i % live.len()].to_string());
    }
    Expression::parse(&src).unwrap()
}

#[test]
fn p99_expression_evaluate_per_bar() {
    let live: Vec<u16> = (0..10u16).filter(|&b| table::is_live(b)).collect();
    let short = Expression::parse(&format!("{}&!{}", live[0], live[1])).unwrap();
    let long = longest();
    assert_eq!(len_of(&long), MAX_INSTRUCTIONS);
    let known = vocab::table::LIVE;
    let rows = bars();
    for (label, e, k) in [
        (format!("short program, {} instructions", len_of(&short)), &short, 64usize),
        (format!("max program, {} instructions", len_of(&long)), &long, 1usize),
    ] {
        let mut i = 0usize;
        let mut acc = 0u64;
        let v = batched(100, SAMPLES, k, || {
            let t = e.evaluate(black_box(rows[i & 4095]), black_box(known));
            acc += u64::from(t == vocab::expression::Truth::True);
            i += 1;
        });
        black_box(acc);
        row(&format!("vocab::Expression::evaluate per bar (runner path) [batch={k}]"), &label, v);
    }
}
```

## p99/store.rs
```rust

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// 2024-06-01 00:00 IST in epoch microseconds (the store bench's anchor).
const JUNE_2024_IST_START: i64 = 1_717_180_200_000_000;

fn path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

fn loaded(name: &str, n: u64) -> (BarFile, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("brutex-p99-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let mut file = BarFile::open_or_create(&root, path(), 7).unwrap();
    let mut i = 0u64;
    while i < n {
        let end = (i + 500_000).min(n);
        let batch: Vec<Bar> = (i..end)
            .map(|i| {
                let raw = i as i64;
                Bar {
                    ts_micros: JUNE_2024_IST_START + raw * 1_000_000,
                    open: 2_000_000 + raw,
                    high: 2_000_100 + raw,
                    low: 1_999_900 + raw,
                    close: 2_000_050 + raw,
                    volume: 1_000,
                    open_interest: 0,
                }
            })
            .collect();
        file.append(&batch).unwrap();
        i = end;
    }
    assert_eq!(file.records(), n);
    (file, root)
}

#[test]
fn p99_store_lookup() {
    // The largest month the store admits at any rung: one-second bars over a
    // 30-day month, 30 * 86,400 = 2,592,000 records (admission refuses a
    // stamp outside the month or off the grid; nothing else caps the count).
    for (label, n) in [("100 bars", 100u64), ("2,592,000 bars (1s x 30d, max month)", 2_592_000u64)] {
        let (file, root) = loaded(if n == 100 { "small" } else { "large" }, n);
        let layout = file.layout();

        let mut acc = 0u64;
        let v = batched(100, SAMPLES * 10, 256, || {
            acc = acc.wrapping_add(layout.offset_of(black_box(n - 1)).unwrap_or(0));
        });
        black_box(acc);
        row("store::Layout::offset_of [batch=256]", label, v);

        // Warm: the same (last) index every call, so its block is cached.
        let v = per_call(1_000, SAMPLES, || {
            black_box(file.read_record(black_box(n - 1)).unwrap());
        });
        row("store::BarFile::read_record warm same index", label, v);

        // Spread: indices scattered over the whole file (page cache warm after
        // a first pass, but the checksum block changes almost every call).
        for i in (0..n).step_by(64) {
            black_box(file.read_record(i).unwrap());
        }
        let mut s = 1u64;
        let v = per_call(1_000, SAMPLES, || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            black_box(file.read_record(black_box((s >> 11) % n)).unwrap());
        });
        row("store::BarFile::read_record scattered index", label, v);

        let mut s = 3u64;
        let v = per_call(1_000, SAMPLES, || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let at = JUNE_2024_IST_START + (((s >> 11) % n) as i64) * 1_000_000;
            black_box(file.first_at_or_after(black_box(at)).unwrap());
        });
        row("store::BarFile::first_at_or_after (O(log n) by design)", label, v);

        drop(file);
        let _ = std::fs::remove_dir_all(&root);
    }
}
```

## p99/telemetry.rs
```rust

use telemetry::{Config, DEFAULT_MAX_FILE_BYTES, Event, Sink};

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("brutex-p99-tel-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn event() -> Event<'static> {
    Event::info("p99.probe", "one event").with("n", 42_u64).with("what", "probe")
}

fn measure(label: &str, sink: &Sink) {
    let v = per_call(500, SAMPLES, || {
        black_box(sink.emit(black_box(&event())));
    });
    row("telemetry::Sink::emit written (write(2), no fsync)", label, v);
    let filtered = Event::debug("p99.probe", "filtered").with("n", 1_u64);
    let v = batched(100, SAMPLES, 64, || {
        black_box(sink.emit(black_box(&filtered)));
    });
    row("telemetry::Sink::emit filtered [batch=64]", label, v);
}

#[test]
fn p99_telemetry_emit() {
    // Empty log.
    let d = dir("empty");
    let sink = Sink::open(&Config::new(&d)).unwrap();
    measure("empty log (8 MiB bound)", &sink);
    drop(sink);
    let _ = std::fs::remove_dir_all(&d);

    // Near rotation: prefill through the sink itself until the file holds
    // the bound minus room for warm-up + timed events, so no roll happens
    // inside the timed window.
    let d = dir("near");
    let sink = Sink::open(&Config::new(&d)).unwrap();
    let before = sink.health().current_bytes;
    sink.emit(&event());
    let per_event = (sink.health().current_bytes - before).max(1);
    let room = per_event * (SAMPLES as u64 + 2_000) + 4096;
    while sink.health().current_bytes + room < DEFAULT_MAX_FILE_BYTES {
        sink.emit(&event());
    }
    let label = format!(
        "near rotation ({} of {} bytes)",
        sink.health().current_bytes,
        DEFAULT_MAX_FILE_BYTES
    );
    let rot_before = sink.health().rotations;
    measure(&label, &sink);
    assert_eq!(sink.health().rotations, rot_before, "a roll landed in the timed window");
    drop(sink);
    let _ = std::fs::remove_dir_all(&d);

    // Rolling: a 64 KiB bound so the timed window crosses many rolls; the
    // tail here IS the rotation (rename chain over keep_files + reopen).
    let d = dir("roll");
    let sink = Sink::open(&Config::new(&d).with_max_file_bytes(64 * 1024)).unwrap();
    let v = per_call(500, SAMPLES, || {
        black_box(sink.emit(black_box(&event())));
    });
    let rolls = sink.health().rotations;
    row(
        "telemetry::Sink::emit written",
        &format!("64 KiB bound, {rolls} rolls in window"),
        v,
    );
    drop(sink);
    let _ = std::fs::remove_dir_all(&d);
}
```

## p99/vocab.rs
```rust

use vocab::ConditionMask;
use vocab::table;

/// 4,096 varied bar masks so the loop cannot be hoisted; an LCG fills them.
fn bars(seed: u64) -> Vec<ConditionMask> {
    let mut s = seed;
    let mut next = || {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        s
    };
    (0..4096)
        .map(|_| ConditionMask::from_words([next(), next(), next(), next(), next(), next()]))
        .collect()
}

fn candidate(bits: u32) -> ConditionMask {
    (0..bits).fold(ConditionMask::ZERO, |m, b| m.with_bit(b))
}

#[test]
fn p99_timer_overhead() {
    row("timer_overhead_empty_closure", "-", per_call(1_000, SAMPLES * 10, || {}));
}

#[test]
fn p99_condition_mask_hits() {
    const K: usize = 256;
    for (label, bits) in [("1 bit set", 1u32), ("300 bits set", 300u32)] {
        let rows = bars(7);
        let cand = candidate(bits);
        assert_eq!(cand.popcount(), bits);
        let mut i = 0usize;
        let mut acc = 0u64;
        let v = batched(100, SAMPLES * 10, K, || {
            let r = black_box(&rows[i & 4095]);
            acc += u64::from(r.hits(black_box(&cand)));
            i = i.wrapping_add(1);
        });
        black_box(acc);
        row(&format!("vocab::ConditionMask::hits [batch={K}]"), label, v);
    }
}

#[test]
fn p99_table_lookup() {
    const K: usize = 256;
    for (label, index) in [("bit 0", 0u16), ("bit 369 (last)", 369u16)] {
        let mut acc = 0usize;
        let v = batched(100, SAMPLES * 10, K, || {
            acc += table::definition(black_box(index)).map_or(0, |d| d.name.len());
        });
        black_box(acc);
        row(&format!("vocab::table::definition [batch={K}]"), label, v);
    }
    let first = table::name(0).unwrap();
    let last = table::name(369).unwrap();
    for (label, name) in [("first name", first), ("last name", last), ("miss 'zzzz'", "zzzz_not_a_name")] {
        let mut acc = 0u32;
        let v = batched(100, SAMPLES * 10, 32, || {
            acc += table::index_of(black_box(name)).map_or(0, u32::from);
        });
        black_box(acc);
        row("vocab::table::index_of [batch=32]", label, v);
    }
}
```
