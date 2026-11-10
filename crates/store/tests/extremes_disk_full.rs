//! A REAL full disk under the store's writer, on demand (cloud extremes round
//! of 10 Nov 2026, `EXTREMES-REPORT.md`).
//!
//! `write.rs` says a full disk cannot be produced without privileged mounting
//! and drives the classifier with a constructed `std::io::Error`; `file.rs`
//! proves the kernel's own `ENOSPC` from `/dev/full` reaches
//! `StoreError::DiskFull`. Neither runs an APPEND into a filesystem that fills
//! up part way through it. This test does, at every page boundary of one
//! append's writes, on a filesystem the operator provides:
//!
//! ```text
//! mount -t tmpfs -o size=4m tmpfs <dir>      # as root, or any small mount
//! BRUTEX_SMALL_FS=<dir> cargo test -p store --test extremes_disk_full -- --ignored
//! ```
//!
//! It is `#[ignore]`d because CI has no small mount and no privilege to make
//! one: run without the variable it fails loudly rather than pass having
//! tested nothing. It is NOT a finding.
//!
//! For each amount of free space left (0 to 40 pages of 4 KiB), one ordinary
//! append of a 375-bar session onto a 1,125-bar month must either commit or be
//! refused by name; the month must then read back at exactly the committed
//! prefix; and once space is freed, re-offering the session must land every
//! bar file byte-identical to a month written on a roomy disk.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const SYMBOL: u32 = 26_000;
const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
/// 2024-06-03 09:15 IST.
const T0: i64 = 1_717_386_300_000_000;
const PAGE: u64 = 4_096;

fn session(day: i64) -> Vec<Bar> {
    (0..375)
        .map(|i| {
            let seed = day * 1_000 + i;
            Bar {
                ts_micros: T0 + day * DAY + i * MINUTE,
                open: 3_000_000 + seed,
                high: 3_000_900 + seed,
                low: 2_999_100 + seed,
                close: 3_000_300 + seed,
                volume: 11 + seed,
                open_interest: OI_NULL,
            }
        })
        .collect()
}

fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("a legal month"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

/// The month's files that hold bars or prove them: the records, the checksum
/// sidecar and the time index. The `.srt` and `.sum` are advisory and their
/// state after a full disk is reported separately.
fn proof_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let dir = month().to_path_buf(root).parent().unwrap().to_path_buf();
    let mut out = BTreeMap::new();
    for ext in ["bin", "crc", "tix"] {
        let path = dir.join(format!("2024-06.{ext}"));
        if let Ok(bytes) = fs::read(&path) {
            out.insert(ext.to_owned(), bytes);
        }
    }
    out
}

fn write_days(root: &Path, days: std::ops::Range<i64>) -> Result<(), StoreError> {
    let mut file = BarFile::open_or_create(root, month(), SYMBOL)?;
    for day in days {
        file.append(&session(day))?;
    }
    Ok(())
}

fn read_back(root: &Path) -> Result<Vec<Bar>, StoreError> {
    let file = BarFile::open_existing(root, month(), SYMBOL)?;
    (0..file.records()).map(|i| file.read_record(i)).collect()
}

/// Fills the filesystem holding `dir` with one file, then gives back `pages`
/// pages of it. Returns the filler's path.
fn fill_leaving(dir: &Path, pages: u64) -> PathBuf {
    let filler = dir.join("filler");
    let mut file = fs::File::create(&filler).expect("the filler");
    let chunk = vec![0x5a_u8; 65_536];
    loop {
        match file.write(&chunk) {
            Ok(0) => break,
            Ok(_) => {}
            Err(why) if why.kind() == std::io::ErrorKind::StorageFull => break,
            Err(why) => panic!("filling the disk met {why}"),
        }
    }
    // A short write can leave the last chunk partial; one byte more proves
    // the disk is full rather than assumed.
    let full = file.write(&[0x5a]);
    assert!(
        matches!(&full, Err(why) if why.kind() == std::io::ErrorKind::StorageFull),
        "the filesystem is full: {full:?}"
    );
    let len = file.metadata().expect("the filler's size").len();
    let keep = len.saturating_sub(pages * PAGE) / PAGE * PAGE;
    file.set_len(keep).expect("free the pages asked for");
    file.sync_all().expect("the filler syncs");
    filler
}

#[test]
#[ignore = "on demand: needs BRUTEX_SMALL_FS, a small writable mount (see the module doc)"]
fn on_demand_a_disk_that_fills_mid_append_leaves_a_committed_prefix_and_resumes_clean() {
    let small = PathBuf::from(
        std::env::var_os("BRUTEX_SMALL_FS").expect("BRUTEX_SMALL_FS names a small mount"),
    );
    let roomy = std::env::temp_dir().join(format!("brutex-xd-clean-{}", std::process::id()));
    drop(fs::remove_dir_all(&roomy));
    fs::create_dir_all(&roomy).unwrap();
    write_days(&roomy, 0..4).expect("the roomy month");
    let clean = proof_files(&roomy);
    let all: Vec<Bar> = (0..4).flat_map(session).collect();

    let mut failures = String::new();
    let mut outcomes = BTreeMap::new();
    for pages in (0..=40).collect::<Vec<u64>>() {
        let root = small.join(format!("brutex-xd-{}-{pages}", std::process::id()));
        drop(fs::remove_dir_all(&root));
        fs::create_dir_all(&root).unwrap();
        write_days(&root, 0..3).expect("the first three sessions fit");
        let filler = fill_leaving(&small, pages);

        let got = BarFile::open_or_create(&root, month(), SYMBOL)
            .and_then(|mut file| file.append(&session(3)));
        let committed = match &got {
            Ok(Appended::Committed { n_valid: 1_500, .. }) => 1_500,
            Err(
                StoreError::DiskFull { .. }
                | StoreError::ShortWrite { .. }
                | StoreError::TimeIndex { .. },
            ) => 1_125,
            other => {
                let _ = writeln!(failures, "{pages} pages: the append answered {other:?}");
                continue;
            }
        };
        outcomes.insert(
            pages,
            format!("{got:?}").chars().take(60).collect::<String>(),
        );
        match read_back(&root) {
            Ok(bars) if bars[..] == all[..committed] => {}
            Ok(bars) => {
                let _ = writeln!(
                    failures,
                    "{pages} pages: {} records read, {committed} committed",
                    bars.len()
                );
            }
            Err(why) => {
                let _ = writeln!(failures, "{pages} pages: the reader refused: {why}");
            }
        }
        fs::remove_file(&filler).expect("space is given back");
        if let Err(why) = write_days(&root, 0..4) {
            let _ = writeln!(failures, "{pages} pages: the resume refused: {why}");
        }
        if proof_files(&root) != clean {
            let _ = writeln!(
                failures,
                "{pages} pages: the resumed month differs from a roomy one"
            );
        }
        drop(fs::remove_dir_all(&root));
    }
    drop(fs::remove_dir_all(&roomy));
    assert!(failures.is_empty(), "{failures}\noutcomes: {outcomes:?}");
    assert!(
        outcomes.values().any(|o| o.starts_with("Err"))
            && outcomes.values().any(|o| o.starts_with("Ok")),
        "the sweep crossed from a refused append to a committed one: {outcomes:?}"
    );
}
