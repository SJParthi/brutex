//! Gate 8 rows R3-20 adds to `crates/store`: the month recovery door after a
//! power cut (`store::recover::recover`, D-7093; `docs/06-limits.md`
//! L-IA-40).
//!
//! The door is NOT O(1) and says so: it reads the one month it recovers
//! through the verified read path and writes its kept bars as a new
//! revision, so its time follows that month's bytes. What must not move it
//! is the rest of the store, and these rows hold both halves:
//!
//! * **C-RC-01, one torn month recovered beside 1, 10 and 100 months of its
//!   series.** The same month (one torn commit of 100 bars past a durable
//!   one) is recovered with 1, 10 and 100 sibling month files in its rung
//!   directory. ASSERTED flat (min-of-trials ratio, gate 8's ceiling): the
//!   door opens the month and its siblings by exact name and lists no
//!   directory. Each size also prints p50, p99 and max over [`SPREAD`]
//!   single recoveries, every one on a fresh copy of the torn month, made
//!   outside the timer.
//! * **C-RC-02, the cost of one kept bar at a month of N and 4N bars.**
//!   ASSERTED flat per bar: the door's time follows the month's bytes, with
//!   no step that grows faster than the bars it keeps.
//!
//! The measured target of the goal — a full one-second month on the
//! operator's drive, n = 20 — is the Mac's (`NEEDS THE MAC` in
//! `reports/cloud-db-R3-20.md`): these rows are invented fixtures on
//! whatever disk the bench runs on. These rows run from
//! `crates/store/benches/ratio.rs`; what each recovery keeps and drops is
//! proved, not timed, by
//! `store::recover::tests::a_month_cut_at_every_synced_write_recovers_to_its_last_good_commit`
//! (invariant R320-01).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::recover::{Outcome, recover};

use super::{ratio, refuse, spread_prepared};

/// Sibling months beside the recovered one: 1x, 10x and 100x.
const SIBLINGS: [usize; 3] = [1, 10, 100];

/// Bars the recovered month keeps in C-RC-01.
const KEPT: i64 = 22_500;

/// Bars in each sibling month: the smallest indexed month (D-4933).
const SIBLING_BARS: i64 = 1_024;

/// Bars the torn commit published and the power cut lost.
const TORN: i64 = 100;

/// Trials whose minimum each asserted ratio compares: a recovery is tens of
/// milliseconds, so fewer than the cheap rows' [`super::TRIALS`].
const RC_TRIALS: u32 = 5;

/// Single recoveries per spread line: the goal's n = 20.
const SPREAD: u32 = 20;

/// The symbol id every fixture file is stamped with.
const ID: u32 = 11;

/// The `i`th month from January 2010; the recovered month is the newest.
fn nth_month(i: usize) -> YearMonth {
    let (Ok(years), Ok(month0)) = (u16::try_from(i / 12), u8::try_from(i % 12)) else {
        refuse("a fixture month index fits a year and a month");
    };
    YearMonth::new(2010 + years, month0 + 1)
        .unwrap_or_else(|_| refuse("a fixture month is a valid month"))
}

fn path(month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "RCBENCH",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month,
        file: FileKind::Bars,
    })
    .unwrap_or_else(|_| refuse("the fixture path is a store path"))
}

/// `n` invented one-second bars of `month` from `offset` seconds past its
/// first 09:15 IST, near 12,345 rupees.
fn bars(month: YearMonth, offset: i64, n: i64) -> Vec<Bar> {
    let start = month.ist_bounds_micros().0 + 555 * 60_000_000;
    (offset..offset + n)
        .map(|k| {
            let price = 1_234_500 + (k % 50) * 5;
            Bar {
                ts_micros: start + k * 1_000_000,
                open: price,
                high: price + 40,
                low: price - 40,
                close: price + 10,
                volume: 7 + k % 3,
                open_interest: OI_NULL,
            }
        })
        .collect()
}

fn commit(root: &Path, month: YearMonth, rows: &[Bar]) {
    let mut writer = BarFile::open_or_create(root, path(month), ID)
        .unwrap_or_else(|why| refuse(&format!("a fixture month opens: {why}")));
    if let Err(why) = writer.append(rows) {
        refuse(&format!("a fixture month commits: {why}"));
    }
}

/// The torn month's three files, as a power cut inside its last batch left
/// them: its newest header slot on the disk, that commit's records and
/// checksums not.
struct Torn {
    bin: Vec<u8>,
    crc: Vec<u8>,
    lock: Vec<u8>,
}

fn read(at: &Path) -> Vec<u8> {
    fs::read(at).unwrap_or_else(|why| refuse(&format!("{} reads: {why}", at.display())))
}

/// A root holding `siblings - 1` sibling months and the torn month of
/// `kept` bars, and the torn month's bytes to put back before each trial.
fn root_with(name: &str, siblings: usize, kept: i64) -> (PathBuf, Torn, YearMonth) {
    let root = std::env::temp_dir().join(format!("brutex-bench-rc-{name}-{}", std::process::id()));
    let _ignored = fs::remove_dir_all(&root);
    if let Err(why) = fs::create_dir_all(&root) {
        refuse(&format!("the recovery bench root would not create: {why}"));
    }
    for i in 0..siblings.saturating_sub(1) {
        commit(&root, nth_month(i), &bars(nth_month(i), 0, SIBLING_BARS));
    }
    let month = nth_month(siblings.saturating_sub(1));
    let target = path(month);
    commit(&root, month, &bars(month, 0, kept));
    let bin = read(&target.to_path_buf(&root));
    let crc = read(&target.with_file(FileKind::Checksums).to_path_buf(&root));
    commit(&root, month, &bars(month, kept, TORN));
    let after = read(&target.to_path_buf(&root));
    let mut torn = bin;
    // THE NEWEST SLOT, and nothing else of the torn commit.
    for (at, slot) in [(0usize, 0..64), (16_384, 16_384..16_448)] {
        if after.get(slot.clone()) != torn.get(slot.clone()) {
            let (Some(dst), Some(src)) = (torn.get_mut(at..at + 64), after.get(slot)) else {
                refuse("a header slot is 64 bytes inside the region");
            };
            dst.copy_from_slice(src);
        }
    }
    let lock = read(&target.with_file(FileKind::Lock).to_path_buf(&root));
    (
        root,
        Torn {
            bin: torn,
            crc,
            lock,
        },
        month,
    )
}

/// Puts `torn`'s bytes in place of the month and removes what a recovery
/// wrote, so each trial recovers the same refused month.
fn reset(root: &Path, month: YearMonth, torn: &Torn) {
    let target = path(month);
    for dir in [
        "bar-revisions-v1",
        "bar-generations-v1",
        "bar-recoveries-v1",
    ] {
        let _ignored = fs::remove_dir_all(root.join(dir));
    }
    for (kind, bytes) in [
        (FileKind::Bars, &torn.bin),
        (FileKind::Checksums, &torn.crc),
        (FileKind::Lock, &torn.lock),
    ] {
        if let Err(why) = fs::write(target.with_file(kind).to_path_buf(root), bytes) {
            refuse(&format!("the torn month resets: {why}"));
        }
    }
}

/// One recovery, which must recover the month.
fn recover_once(root: &Path, month: YearMonth) {
    match recover(root, path(month), ID) {
        Ok(Outcome::Recovered(_)) => {}
        other => refuse(&format!("the torn month recovers: {other:?}")),
    }
}

/// The minimum over [`RC_TRIALS`] of one recovery, in picoseconds.
fn min_ps(root: &Path, month: YearMonth, torn: &Torn) -> u128 {
    let mut best = u128::MAX;
    for _ in 0..RC_TRIALS {
        reset(root, month, torn);
        let start = Instant::now();
        recover_once(root, month);
        best = best.min(start.elapsed().as_nanos().saturating_mul(1_000));
    }
    best
}

/// C-RC-01 and C-RC-02.
pub(crate) fn run() -> bool {
    let mut ok = true;
    let mut times = Vec::new();
    for siblings in SIBLINGS {
        let (root, torn, month) = root_with(&format!("x{siblings}"), siblings, KEPT);
        times.push(min_ps(&root, month, &torn));
        spread_prepared(
            &format!("C-RC-01 recover a torn month, {siblings:>3} month(s) of it"),
            SPREAD,
            || reset(&root, month, &torn),
            |()| recover_once(&root, month),
        );
        let _ignored = fs::remove_dir_all(&root);
    }
    if let [base, ten, hundred] = times[..] {
        ok &= ratio("C-RC-01 recovery, 10x months stored", base, ten);
        ok &= ratio("C-RC-01 recovery, 100x months stored", base, hundred);
    }
    let per_bar = |kept: i64| {
        let (root, torn, month) = root_with(&format!("n{kept}"), 1, kept);
        let total = min_ps(&root, month, &torn);
        let _ignored = fs::remove_dir_all(&root);
        total / u128::try_from(kept).unwrap_or(1)
    };
    let (small, large) = (per_bar(KEPT), per_bar(4 * KEPT));
    println!(
        "  C-RC-02 recovery a kept bar: {small} ps at {KEPT} bars, {large} ps at {} bars",
        4 * KEPT
    );
    ok &= ratio("C-RC-02 recovery a kept bar, 4x month", small, large);
    ok
}
