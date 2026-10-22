//! C-QU-01 and C-QU-02: the quarantine probe (R4-18, D-6892).
//!
//! C-QU-01 times `store::quarantine::probe` against an index holding 1x, 10x
//! and 100x quarantined months (12, 120 and 1,200 months of one share across
//! ten rungs' worth of keys), probing a month that IS quarantined and one
//! that is not: the probe is one open, one `fstat` and one 512-byte
//! positional read, so its cost must not grow with the months held. Each
//! line then prints p50, p99 and max of single calls, n = 300.
//!
//! C-QU-02 prints, unratioed, `Snapshot::load` (one read of the whole index)
//! at the same three sizes: it is O(index), once per cached body, never per
//! row, and is said so rather than ratioed.

use std::path::PathBuf;

use brutex_core::vendor::Vendor;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::quarantine::{Cause, Record, Scope};

use super::{cost_ps, ratio, refuse, spread};

const SIZES: [u16; 3] = [12, 120, 1_200];
const CALLS: u32 = 300;

fn root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("brutex-bench-qu-{name}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(e) = std::fs::create_dir_all(&root) {
        refuse(&format!("the quarantine bench root would not create: {e}"));
    }
    root
}

/// The `n`-th month of key `symbol`, from 2000-01.
fn path(symbol: &'static str, n: u16) -> StorePath<'static> {
    let month = YearMonth::new(2000 + n / 12, u8::try_from(n % 12 + 1).unwrap_or(1))
        .unwrap_or_else(|e| refuse(&format!("a bench month: {e}")));
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap_or_else(|e| refuse(&format!("a bench path: {e}")))
}

const SYMBOLS: [&str; 10] = [
    "QBA", "QBB", "QBC", "QBD", "QBE", "QBF", "QBG", "QBH", "QBI", "QBJ",
];

fn filled(months: u16) -> PathBuf {
    let root = root(&format!("m{months}"));
    let per = months / 10;
    for symbol in SYMBOLS {
        for n in 0..per {
            let done = store::quarantine::record(
                &root,
                &Record {
                    path: path(symbol, n),
                    scope: Scope::WholeMonth,
                    cause: Cause::BeforeListing,
                    stamps: &[],
                    bars: 1,
                    revision: 0,
                    source_digest: [0; 32],
                    who: "bench",
                    reason: "C-QU-01",
                },
            );
            if let Err(e) = done {
                refuse(&format!("a quarantine record would not write: {e}"));
            }
        }
    }
    root
}

pub(super) fn run() -> bool {
    println!("C-QU quarantine probe (R4-18)");
    let roots: Vec<PathBuf> = SIZES.iter().map(|months| filled(*months)).collect();
    let mut ok = true;
    let mut costs = Vec::new();
    for (root, months) in roots.iter().zip(SIZES) {
        let held = path("QBA", 0);
        let clear = path("QZZ", 0);
        let hit = cost_ps(200, || store::quarantine::probe(root, held));
        let miss = cost_ps(200, || store::quarantine::probe(root, clear));
        spread(
            &format!("C-QU-01 probe, quarantined month, {months} held"),
            CALLS,
            || {
                let _ = std::hint::black_box(store::quarantine::probe(root, held));
            },
        );
        spread(
            &format!("C-QU-01 probe, clear month, {months} held"),
            CALLS,
            || {
                let _ = std::hint::black_box(store::quarantine::probe(root, clear));
            },
        );
        spread(
            &format!("C-QU-02 Snapshot::load, {months} held (once per body)"),
            50,
            || {
                let _ =
                    std::hint::black_box(store::quarantine::Snapshot::load(root, Vendor::Zerodha));
            },
        );
        costs.push((hit, miss));
    }
    if let (Some(base), Some(mid), Some(top)) = (costs.first(), costs.get(1), costs.last()) {
        ok &= ratio("C-QU-01 probe hit, 10x months", base.0, mid.0);
        ok &= ratio("C-QU-01 probe hit, 100x months", base.0, top.0);
        ok &= ratio("C-QU-01 probe miss, 10x months", base.1, mid.1);
        ok &= ratio("C-QU-01 probe miss, 100x months", base.1, top.1);
    }
    for root in &roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

/// C-QU-03: `repair::drop_month` at 183, 7,875 and 472,445 bars (an hour
/// month, a minute month, a one-second month), one bar named each time: the
/// door reads and writes the whole month, O(month), and says so with its
/// three sizes rather than a ratio.
pub(super) fn drop_is_linear() {
    use store::file::BarFile;
    use store::format::Bar;
    for (rung, bars) in [
        (
            Timeframe::from_secs(3_600).unwrap_or(Timeframe::MINUTE_1),
            183_i64,
        ),
        (Timeframe::MINUTE_1, 7_875),
        (Timeframe::SECOND_1, 472_445),
    ] {
        let mut ns = Vec::new();
        for trial in 0..5 {
            let root = root(&format!("drop-{bars}-{trial}"));
            let month = YearMonth::new(2025, 3).unwrap_or_else(|e| refuse(&format!("{e}")));
            let path = StorePath::new(PathParts {
                vendor: Vendor::Zerodha,
                exchange: "NSE",
                segment: "CASH",
                symbol: "QBA",
                contract: None,
                timeframe: rung,
                month,
                file: FileKind::Bars,
            })
            .unwrap_or_else(|e| refuse(&format!("{e}")));
            // 09:15 IST on the 1st: on every rung's grid, the hour's included
            // (it is aligned to the open).
            let first = month.ist_bounds_micros().0 + 33_300_000_000;
            let step = i64::from(rung.secs()) * 1_000_000;
            let rows: Vec<Bar> = (0..bars)
                .map(|i| Bar {
                    ts_micros: first + i * step,
                    open: 1_000,
                    high: 1_010,
                    low: 990,
                    close: 1_005,
                    volume: 1,
                    open_interest: store::format::OI_NULL,
                })
                .collect();
            let mut writer = BarFile::open_or_create(&root, path, 7)
                .unwrap_or_else(|e| refuse(&format!("the bench month: {e}")));
            for batch in rows.chunks(65_536) {
                if let Err(e) = writer.append(batch) {
                    refuse(&format!("the bench rows: {e}"));
                }
            }
            drop(writer);
            let named = [rows.get(rows.len() / 2).map_or(first, |bar| bar.ts_micros)];
            let started = std::time::Instant::now();
            if let Err(e) = store::repair::drop_month(&root, path, 7, &named, "C-QU-03") {
                refuse(&format!("drop_month: {e}"));
            }
            ns.push(started.elapsed().as_nanos());
            let _ignored = std::fs::remove_dir_all(&root);
        }
        ns.sort_unstable();
        println!(
            "  C-QU-03 drop_month, {bars:>7} bars ({:<4})            p50 {:>11} ns  max {:>11} ns  n=5 (O(month), not ratioed)",
            rung.as_str(),
            ns.get(2).copied().unwrap_or(0),
            ns.last().copied().unwrap_or(0)
        );
    }
}
