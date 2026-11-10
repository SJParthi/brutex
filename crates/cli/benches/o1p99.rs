//! The o1p99 attack on `crates/cli` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `benches/ratio.rs` times the results ledger as the minimum of 40 trials on
//! ledgers of 64 and 1,024 records. This file re-times the ledger's public
//! calls per call, prints p50 and p99, and reads them on ledgers of 1, 10^2,
//! 10^3 and 10^4 records (and 10^5 where the fixture fits the run). A gated
//! row fails when its quietest-round p99 at ANY size exceeds the smallest
//! size's by more than 3.0x (the shared-CI ceiling). See `o1p99/harness.rs`
//! and `O1-ATTACK-REPORT.md` at the repository root.
//!
//! * `O1P99-CLI-01` `Results::read` at a uniformly random index: "O(1) —
//!   HEADER + index * STRIDE, one seek", which its doc marks UNVERIFIED as a
//!   measurement; no row of `ratio` varies the ledger for it.
//! * `O1P99-CLI-02` `Results::holds`, a hit at a random held identity and a
//!   miss, the duplicate probe C-CLI-03 times only at the LAST record.
//! * `O1P99-CLI-03` `Results::refresh` when nothing changed: "constant
//!   metadata checks when unchanged".
//! * `O1P99-CLI-04` `Results::refresh` after ONE record another handle
//!   appended: its doc says "O(new rows)". PRINTED, NOT GATED: it grows with
//!   the ledger (finding O1P99-F-CLI-01, `tests/o1p99_findings.rs`; the cost
//!   is admitted in `docs/06-limits.md` D-1560, the doc on `refresh` is not).
//!
//! Ledgers live under the system temporary directory; the page cache is warm.

use std::hint::black_box;
use std::path::PathBuf;

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use cli::results::{Record, Results, field};
use harness::{FACTOR_PERMILLE, Rng, measure, verdict};

/// The ledger sizes every row is read at.
const SIZES: [u64; 4] = [1, 100, 1_000, 10_000];

/// A setup failure this bench cannot measure past.
fn refuse(why: &str) -> ! {
    println!("BENCH SETUP FAILED — {why}");
    std::process::exit(1)
}

/// The record `benches/ratio.rs` writes, identity `n`.
fn record(n: u64) -> Record {
    let mut identity = [0_u8; 32];
    if let Some(head) = identity.get_mut(..8) {
        head.copy_from_slice(&n.to_le_bytes());
    }
    Record {
        identity,
        finished_micros: 1_787_509_605_880_000,
        mask_words: [0; 6],
        feed: field("zerodha"),
        underlying: field("NIFTY"),
        timeframe: field("15min"),
        from_year: 2019,
        from_month: 12,
        to_year: 2026,
        to_month: 8,
        months_asked: 81,
        months_found: 81,
        bars: 41_572,
        min_hits: 8_314,
        combinations: 1_024_058,
        depth: 16,
        halted: 0,
        trades: 11_209,
        pessimistic: 2_459_160,
        optimistic: 3_649_640,
        worst_trade: -4_694,
        max_drawdown: 29_163,
        winner_mae: 300,
        winner_mfe: 700,
        all_mae: 350,
        exit_rungs: [0; 5],
    }
}

/// A ledger of `n` records in a fresh directory, with its writer.
fn ledger(tag: &str, n: u64) -> (Results, PathBuf) {
    let root =
        std::env::temp_dir().join(format!("brutex-o1p99-cli-{tag}-{n}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    if let Err(why) = std::fs::create_dir_all(&root) {
        refuse(&format!("a temp root could not be made: {why}"));
    }
    let Ok(mut store) = Results::open(&root) else {
        refuse("a fresh directory refused to open as a ledger")
    };
    for i in 0..n {
        if store.append(&record(i)).is_err() {
            refuse("an append into a fresh ledger refused");
        }
    }
    (store, root)
}

/// One ledger per size: its writer, a read-only handle, the root, `n`, keys.
struct Held {
    writer: Results,
    reader: Results,
    root: PathBuf,
    n: u64,
    next: u64,
    rng: Rng,
}

/// Ledgers of every size in [`SIZES`].
fn ledgers(tag: &str) -> Vec<Held> {
    SIZES
        .iter()
        .map(|&n| {
            let started = std::time::Instant::now();
            let (writer, root) = ledger(tag, n);
            let Ok(reader) = Results::open_read(&root) else {
                refuse("a ledger refused to open read-only")
            };
            println!(
                "O1P99|O1P99-CLI|fixture|n={n}|built in {} ms",
                started.elapsed().as_millis()
            );
            Held {
                writer,
                reader,
                root,
                n,
                next: n,
                rng: Rng::new(n ^ 0xc1),
            }
        })
        .collect()
}

/// `O1P99-CLI-01`.
fn read_is_flat(clock: u128, held: &mut [Held]) -> bool {
    let tails = measure(
        held,
        5,
        2_000,
        1,
        clock,
        |_| {},
        |h, _| {
            let at = h.rng.below(h.n);
            h.reader.read(black_box(at)).map_or(0, |r| r.bars)
        },
    );
    verdict(
        "O1P99-CLI-01",
        "Results::read random index by records",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-CLI-02`.
fn holds_is_flat(clock: u128, held: &mut [Held]) -> bool {
    let hit = measure(
        held,
        5,
        2_000,
        16,
        clock,
        |_| {},
        |h, _| {
            let id = record(h.rng.below(h.n)).identity;
            u64::from(h.reader.holds(black_box(&id)))
        },
    );
    let mut ok = verdict(
        "O1P99-CLI-02",
        "Results::holds hit by records",
        &SIZES,
        &hit,
        FACTOR_PERMILLE,
    );
    let miss = measure(
        held,
        5,
        2_000,
        16,
        clock,
        |_| {},
        |h, i| {
            let id = record(u64::MAX - i).identity;
            u64::from(h.reader.holds(black_box(&id)))
        },
    );
    ok &= verdict(
        "O1P99-CLI-02",
        "Results::holds miss by records",
        &SIZES,
        &miss,
        FACTOR_PERMILLE,
    );
    ok
}

/// `O1P99-CLI-03`.
fn unchanged_refresh_is_flat(clock: u128, held: &mut [Held]) -> bool {
    let tails = measure(
        held,
        5,
        1_000,
        1,
        clock,
        |_| {},
        |h, _| u64::from(h.reader.refresh().is_ok()),
    );
    verdict(
        "O1P99-CLI-03",
        "Results::refresh, nothing appended, by records",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    )
}

/// `O1P99-CLI-04`: printed only.
fn grown_refresh(clock: u128, held: &mut [Held]) {
    // The append happens in `prepare`, outside the timer; only the reader's
    // refresh of that one new record is timed.
    let tails = measure(
        held,
        4,
        50,
        1,
        clock,
        |h| {
            let r = record(h.next);
            h.next = h.next.saturating_add(1);
            if h.writer.append(&r).is_err() {
                refuse("a foreign append refused");
            }
        },
        |h, _| u64::from(h.reader.refresh().is_ok()),
    );
    let _printed = verdict(
        "O1P99-CLI-04",
        "Results::refresh after one foreign append, by records (printed; see O1P99-F-CLI-01)",
        &SIZES,
        &tails,
        u128::MAX,
    );
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/cli, clock {} ns a pair",
        harness::ns(clock)
    );
    let mut held = ledgers("ledger");
    let mut ok = true;
    ok &= read_is_flat(clock, &mut held);
    ok &= holds_is_flat(clock, &mut held);
    ok &= unchanged_refresh_is_flat(clock, &mut held);
    grown_refresh(clock, &mut held);
    for h in held {
        let root = h.root.clone();
        drop(h);
        let _ignored = std::fs::remove_dir_all(root);
    }
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}
