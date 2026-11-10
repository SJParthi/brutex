//! The o1p99 attack on `crates/pull` (cloud lane `cloud/o1p99-attack-20261110`).
//!
//! `benches/ratio.rs` times most rows as the minimum of a few trials of a
//! per-call mean, on one hot key, at 1x/10x/100x of a base that stays under
//! about 10^5. This file re-times four claims per call, prints p50 and p99,
//! and reads them at 1, 10^2, 10^4 and 10^6 of the size each claim names,
//! or as far as the fixture fits a bench run. A GATED row fails when its
//! quietest-round p99 at ANY size exceeds the smallest size's by more than
//! 3.0x (the shared-CI ceiling). See `o1p99/harness.rs` and
//! `O1-ATTACK-REPORT.md` at the repository root.
//!
//! * `O1P99-PULL-01` `fnowork::futures_plan` (the book opened as a request
//!   opens it), by dated snapshots held: 1, 10, 100 and 1,000 DAILY
//!   snapshots of a 40-row dump. C-MS-02 times 1, 10 and 100 WEEKLY
//!   snapshots of a 2,000-row dump, where parsing the newest snapshot is
//!   most of every call. PRINTED, NOT GATED: it grows with the snapshots
//!   held (finding O1P99-F-PULL-02, `tests/o1p99_findings.rs`).
//! * `O1P99-PULL-02` `gdfl_dropbox::status`, by journal lines: printed from 1
//!   to 10^6, GATED from 1,000 lines (the first journal longer than the
//!   64 KiB window) to 10^6; and the same call when the journal's tail holds
//!   lines this build cannot parse (70 KiB and 4 MiB of them), PRINTED: the
//!   window doubles up to `MAX_STATUS_WINDOW`, re-reading the tail each time.
//! * `O1P99-PULL-03` `alerts::fit_text` by text bytes: `TEXT_MAX` is 75, and
//!   the whole text is cleaned and copied before it is cut. PRINTED, NOT
//!   GATED (finding O1P99-F-PULL-01, `tests/o1p99_findings.rs`).
//! * `O1P99-PULL-04` `Manifest::entry` on a loaded census of 1 to 10^6
//!   entries: GATED over 64 held keys walked in turn (the probe count), and
//!   PRINTED for a uniformly random held key beside the memory floor of a
//!   table that size (the cache hierarchy, which C-12's one hot key never
//!   reaches).

use std::fmt::Write as _;
use std::hint::black_box;
use std::path::PathBuf;

#[allow(
    dead_code,
    reason = "the harness is copied into every crate's o1p99 bench and not every bench calls every helper"
)]
#[path = "o1p99/harness.rs"]
mod harness;

use brutex_core::instrument::{Exchange, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use harness::{FACTOR_PERMILLE, Rng, measure, verdict};
use pull::manifest::{Commit, Entry, EntryKey, Manifest};
use pull::master_snapshots::{self, Book, Kind};
use pull::session::Day;
use store::path::{Timeframe, YearMonth};

/// A setup failure this bench cannot measure past.
fn refuse(why: &str) -> ! {
    println!("BENCH SETUP FAILED — {why}");
    std::process::exit(1)
}

/// A fresh scratch directory for one row.
fn scratch(name: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("brutex-o1p99-pull-{name}-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&root);
    if std::fs::create_dir_all(&root).is_err() {
        refuse("a scratch root")
    }
    root
}

// ── O1P99-PULL-01 ───────────────────────────────────────────────────────────

/// A 40-row Kite dump: three futures for each of ten `ZQ` underlyings and
/// ten options, the shape C-MS-02's dump has at a fiftieth of its rows.
fn small_dump() -> String {
    let mut csv = String::from(pull::vendor::KITE_MASTER_HEADER);
    for row in 0..40_u64 {
        let underlying = row % 10;
        let token = 9_100_000 + row;
        let _infallible = if row < 30 {
            let month = row / 10 + 1;
            write!(
                csv,
                "\n{token},{row},ZQ{underlying:02}F{month},\"ZQ{underlying:02}\",0,2030-0{month}-28,0,0.05,75,FUT,NFO-FUT,NFO"
            )
        } else {
            write!(
                csv,
                "\n{token},{row},ZQ{underlying:02}C{row},\"ZQ{underlying:02}\",0,2030-01-28,{row},0.05,75,CE,NFO-OPT,NFO"
            )
        };
    }
    csv
}

/// `O1P99-PULL-01`: printed only.
fn futures_plan_by_snapshots(clock: u128) {
    const SIZES: [u64; 4] = [1, 10, 100, 1_000];
    let (Ok(first), Ok(last), Ok(today)) = (
        Day::new(2024, 1, 1),
        Day::new(2026, 12, 31),
        Day::new(2027, 1, 1),
    ) else {
        refuse("real days")
    };
    let Ok(window) = pull::session::Window::new(first, last) else {
        refuse("an ordered window")
    };
    let body = small_dump();
    let mut roots: Vec<PathBuf> = SIZES
        .iter()
        .map(|&count| {
            let root = scratch(&format!("cms02-{count}"));
            for d in 0..u32::try_from(count).unwrap_or(0) {
                let Ok(on) = Day::from_days(first.days_from_epoch() + d) else {
                    refuse("a real day")
                };
                if master_snapshots::snapshot(
                    &root,
                    Kind::Kite,
                    on,
                    "bench://o1p99",
                    body.as_bytes(),
                )
                .is_err()
                {
                    refuse("the invented snapshot is written");
                }
            }
            root
        })
        .collect();
    let tails = measure(
        &mut roots,
        4,
        100,
        1,
        clock,
        |_| {},
        |root, _| {
            let Ok(book) = Book::open(root) else {
                refuse("the book opens")
            };
            match pull::fnowork::futures_plan(
                &book,
                pull::vendor::ZERODHA_CONTINUOUS,
                "ZQ03",
                window,
                today,
                window.to(),
            ) {
                Ok(plan) => u64::try_from(plan.asks.len()).unwrap_or(0),
                Err(_) => refuse("the plan answers"),
            }
        },
    );
    // PRINTED, NOT GATED: it grows with the snapshots held, which is
    // finding O1P99-F-PULL-02 (`tests/o1p99_findings.rs`).
    let _printed = verdict(
        "O1P99-PULL-01",
        "Book::open + futures_plan by daily snapshots held, 40-row dump (printed; see O1P99-F-PULL-02)",
        &SIZES,
        &tails,
        u128::MAX,
    );
    for root in roots {
        let _ignored = std::fs::remove_dir_all(root);
    }
}

// ── O1P99-PULL-02 ───────────────────────────────────────────────────────────

/// A dropbox journal of `lines` parseable lines, then `junk` bytes of lines
/// this build does not parse, under a fresh store root.
fn journal(lines: u64, junk: u64) -> PathBuf {
    let root = scratch(&format!("dp02-{lines}-{junk}"));
    let path = pull::gdfl_dropbox::journal_path(&root);
    let Some(parent) = path.parent() else {
        refuse("the journal has no folder")
    };
    if std::fs::create_dir_all(parent).is_err() {
        refuse("the journal folder")
    }
    let mut text = String::new();
    for n in 0..lines {
        let _infallible = writeln!(
            text,
            "v=1 listed run=r at_ms={n} name=\"GFDLNFO_TICK_{:08}.zip\" bytes={n}",
            n % 100_000_000
        );
    }
    // Exactly `junk` more bytes (to the line), and none when it is zero.
    let target = u64::try_from(text.len())
        .unwrap_or(u64::MAX)
        .saturating_add(junk);
    let mut n = 0_u64;
    while u64::try_from(text.len()).unwrap_or(u64::MAX) < target {
        let _infallible = writeln!(text, "v=9 a line from a newer build {n}");
        n += 1;
    }
    if std::fs::write(&path, text).is_err() {
        refuse("the journal could not be written")
    }
    root
}

/// `O1P99-PULL-02`.
fn dropbox_status_by_journal(clock: u128) -> bool {
    // The adversarial tails: no junk, then junk the 64 KiB window must
    // double past once, then junk it doubles past up to its 4 MiB cap.
    const JUNK: [u64; 3] = [0, 70 * 1_024, 4 * 1_024 * 1_024];
    const SIZES: [u64; 6] = [1, 100, 1_000, 10_000, 100_000, 1_000_000];
    let mut roots: Vec<PathBuf> = SIZES.iter().map(|&n| journal(n, 0)).collect();
    let tails = measure(
        &mut roots,
        5,
        1_000,
        1,
        clock,
        |_| {},
        |root, _| u64::from(matches!(pull::gdfl_dropbox::status(root), Ok(Some(_)))),
    );
    // Every size PRINTED: under about 900 lines the journal is shorter than
    // the 64 KiB window and costs less. GATED from 1,000 lines, the first
    // size whose window is full, because the claim is O(1) in the journal's
    // length once the window is what is read.
    let _printed = verdict(
        "O1P99-PULL-02",
        "gdfl_dropbox::status by journal lines, every size (printed)",
        &SIZES,
        &tails,
        u128::MAX,
    );
    let ok = verdict(
        "O1P99-PULL-02",
        "gdfl_dropbox::status by journal lines, window full",
        SIZES.get(2..).unwrap_or(&[]),
        tails.get(2..).unwrap_or(&[]),
        FACTOR_PERMILLE,
    );
    // The adversarial tail: 1,000 good lines, then junk the window must
    // double past. The base is the same journal with no junk.
    let mut hostile: Vec<PathBuf> = JUNK.iter().map(|&j| journal(1_000, j)).collect();
    let tails = measure(
        &mut hostile,
        4,
        200,
        1,
        clock,
        |_| {},
        |root, _| u64::from(matches!(pull::gdfl_dropbox::status(root), Ok(Some(_)))),
    );
    let _printed = verdict(
        "O1P99-PULL-02",
        "gdfl_dropbox::status by unparseable tail bytes (printed)",
        &JUNK,
        &tails,
        u128::MAX,
    );
    for root in roots.into_iter().chain(hostile) {
        let _ignored = std::fs::remove_dir_all(root);
    }
    ok
}

// ── O1P99-PULL-03 ───────────────────────────────────────────────────────────

/// `O1P99-PULL-03`: printed only.
fn fit_text_by_text_bytes(clock: u128) {
    const SIZES: [u64; 4] = [1, 100, 10_000, 1_000_000];
    let mut texts: Vec<String> = SIZES
        .iter()
        .map(|&n| {
            "an invented alert "
                .chars()
                .cycle()
                .take(usize::try_from(n).unwrap_or(0))
                .collect()
        })
        .collect();
    let tails = measure(
        &mut texts,
        5,
        400,
        1,
        clock,
        |_| {},
        |t, _| {
            let (kept, cut) = pull::alerts::fit_text(black_box(t.as_str()));
            u64::try_from(kept.len()).unwrap_or(0) + u64::from(cut)
        },
    );
    let _printed = verdict(
        "O1P99-PULL-03",
        "alerts::fit_text by text bytes (printed; see O1P99-F-PULL-01)",
        &SIZES,
        &tails,
        u128::MAX,
    );
}

// ── O1P99-PULL-04 ───────────────────────────────────────────────────────────

/// The census key of entry `i`: up to 96,360 months a symbol, so a
/// million keys spread over eleven symbols.
fn census_key(i: u64) -> EntryKey {
    const MONTHS: u64 = (9_999 - 1_970 + 1) * 12;
    let j = i % MONTHS;
    let (Ok(year), Ok(month)) = (u16::try_from(1_970 + j / 12), u8::try_from(j % 12 + 1)) else {
        refuse("a year and month in range")
    };
    let (Ok(symbol), Ok(month)) = (
        Symbol::new(&format!("Z{:05}", i / MONTHS)),
        YearMonth::new(year, month),
    ) else {
        refuse("a symbol and a month this build admits")
    };
    EntryKey {
        contract: None,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol,
        timeframe: Timeframe::MINUTE_1,
        month,
    }
}

/// The header region holding one commit in its slot, as `ratio.rs` builds it.
fn header_region(commit: &Commit) -> Vec<u8> {
    const HEADER_LEN_LEN: usize = 32_768;
    const SLOT_STRIDE_LEN: usize = 16_384;
    let mut region: Vec<u8> = Vec::with_capacity(HEADER_LEN_LEN);
    if commit.slot != 0 {
        region.resize(SLOT_STRIDE_LEN, 0);
    }
    region.extend_from_slice(&commit.bytes);
    region.resize(HEADER_LEN_LEN, 0);
    region
}

/// A census of `count` entries as a reader loads it.
fn loaded_census(count: u64) -> Manifest {
    let Ok(mut writer) = Manifest::open(Vendor::Groww, &[], &[]) else {
        refuse("a genesis manifest")
    };
    let mut data = Vec::new();
    let mut published: Option<Commit> = None;
    for i in 0..count {
        let entry = Entry {
            key: census_key(i),
            rows: 7_312,
            first_ts_micros: 1_000,
            last_ts_micros: 2_000,
        };
        match writer.record(entry) {
            Ok(append) => {
                data.extend_from_slice(&append.bytes);
                published = Some(append.commit);
            }
            Err(e) => refuse(&e.to_string()),
        }
    }
    let Some(commit) = published else {
        refuse("a census of no months")
    };
    match Manifest::load(Vendor::Groww, &header_region(&commit), &data) {
        Ok(m) => m,
        Err(e) => refuse(&e.to_string()),
    }
}

/// `O1P99-PULL-04`.
fn manifest_entry_by_census(clock: u128) -> bool {
    const SIZES: [u64; 4] = [1, 100, 10_000, 1_000_000];
    let mut censuses: Vec<(Manifest, Vec<EntryKey>, u64, Rng)> = SIZES
        .iter()
        .map(|&n| {
            let mut r = Rng::new(n);
            let hot: Vec<EntryKey> = (0..64).map(|_| census_key(r.below(n))).collect();
            (loaded_census(n), hot, n, Rng::new(n ^ 0x0f))
        })
        .collect();
    let tails = measure(
        &mut censuses,
        5,
        2_000,
        16,
        clock,
        |_| {},
        |(m, hot, _, _), i| {
            let at = usize::try_from(i % 64).unwrap_or(0);
            hot.get(at)
                .and_then(|k| m.entry(black_box(k)))
                .map_or(0, |e| e.rows)
        },
    );
    let ok = verdict(
        "O1P99-PULL-04",
        "Manifest::entry, 64 held keys in turn, by census",
        &SIZES,
        &tails,
        FACTOR_PERMILLE,
    );
    // Random held keys: the key is made outside the timer, one per sample.
    let random = measure(
        &mut censuses,
        5,
        2_000,
        1,
        clock,
        |(_, hot, n, r)| {
            if let Some(slot) = hot.first_mut() {
                *slot = census_key(r.below(*n));
            }
        },
        |(m, hot, _, _), _| {
            hot.first()
                .and_then(|k| m.entry(black_box(k)))
                .map_or(0, |e| e.rows)
        },
    );
    let _printed = verdict(
        "O1P99-PULL-04",
        "Manifest::entry, a random held key, by census (printed)",
        &SIZES,
        &random,
        u128::MAX,
    );
    // About 300 bytes of index an entry (`pull::manifest`'s own figure).
    let bytes: Vec<u64> = SIZES.iter().map(|n| n.saturating_mul(300)).collect();
    let floor = harness::memory_floor(&bytes, 5, 2_000, clock);
    let _printed = verdict(
        "O1P99-PULL-04",
        "memory floor, one random load in a table that size (printed)",
        &SIZES,
        &floor,
        u128::MAX,
    );
    ok
}

fn main() {
    let clock = harness::clock_ps();
    println!(
        "o1p99 attack — crates/pull, clock {} ns a pair",
        harness::ns(clock)
    );
    let mut ok = true;
    futures_plan_by_snapshots(clock);
    ok &= dropbox_status_by_journal(clock);
    fit_text_by_text_bytes(clock);
    ok &= manifest_entry_by_census(clock);
    if ok {
        println!("every gated o1p99 row held its ceiling");
    } else {
        println!("AN o1p99 ROW GREW PAST ITS CEILING — see the VERDICT lines marked GROWS.");
        std::process::exit(1);
    }
}
