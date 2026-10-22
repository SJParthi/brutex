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
