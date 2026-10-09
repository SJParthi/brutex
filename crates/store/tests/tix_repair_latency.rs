//! Wall-clock latency of an append that finds its `.tix` entry torn — D-3134's
//! measurement of the repair that replaced D-3302's O(`n_valid`) rebuild.
//!
//! `#[ignore]`d: it writes months of up to 10^6 one-second bars, so it is run
//! on purpose, in release:
//!
//! ```text
//! cargo test -p store --release --test tix_repair_latency -- --ignored --nocapture
//! ```
//!
//! It asserts that every timed append committed and left the index ready, and
//! nothing about time: a shared host's numbers are not a gate. The read bound
//! itself is asserted, not timed, by
//! `time_index::tests::recover_rebuilds_the_last_entry_exactly_from_at_most_65_reads`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::print_stdout
)]

use std::fs;
use std::os::unix::fs::FileExt as _;
use std::path::PathBuf;
use std::time::Instant;

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, TimeLookup};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// 2024-06-01 00:00 IST.
const JUNE_START: i64 = 1_717_180_200_000_000;
const MICROS: i64 = 1_000_000;
const SAMPLES: usize = 101;

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

fn bar(k: i64) -> Bar {
    Bar {
        ts_micros: JUNE_START + k * MICROS,
        open: 2_000_000,
        high: 2_000_100,
        low: 1_999_900,
        close: 2_000_050,
        volume: 10,
        open_interest: OI_NULL,
    }
}

#[test]
#[ignore = "writes up to 10^6 bars; run on purpose, in release"]
fn a_torn_entry_append_costs_the_same_at_every_month_size() {
    for n in [1_000_i64, 10_000, 100_000, 1_000_000] {
        let root =
            std::env::temp_dir().join(format!("brutex-tix-repair-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let tix: PathBuf = path().with_file(FileKind::TimeIndex).to_path_buf(&root);
        let mut writer = BarFile::open_or_create(&root, path(), 7).unwrap();
        let month: Vec<Bar> = (0..n).map(bar).collect();
        for chunk in month.chunks(50_000) {
            assert!(matches!(
                writer.append(chunk),
                Ok(Appended::Committed { .. })
            ));
        }
        let index = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&tix)
            .unwrap();
        let mut ns = Vec::with_capacity(SAMPLES);
        for k in 0..SAMPLES {
            let last = n + i64::try_from(k).unwrap() - 1;
            // Tear the entry the next append resumes from.
            let at = 64 + (u64::try_from(last).unwrap() / 64) * 16;
            let mut entry = [0u8; 16];
            index.read_exact_at(&mut entry, at).unwrap();
            entry[3] ^= 0x5A;
            index.write_all_at(&entry, at).unwrap();
            let next = [bar(last + 1)];
            let start = Instant::now();
            let appended = writer.append(&next);
            ns.push(start.elapsed().as_nanos());
            assert!(matches!(appended, Ok(Appended::Committed { .. })));
        }
        assert_eq!(writer.time_lookup(), TimeLookup::Indexed);
        ns.sort_unstable();
        let rank = |permille: usize| ns[(ns.len() * permille / 1_000).min(ns.len() - 1)];
        println!(
            "torn-entry append n={n:>9}  p50 {:>9} ns  p99 {:>9} ns  max {:>9} ns",
            rank(500),
            rank(990),
            ns[ns.len() - 1]
        );
        drop(writer);
        let _ = fs::remove_dir_all(&root);
    }
}
