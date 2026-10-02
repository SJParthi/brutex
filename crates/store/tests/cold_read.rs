//! The cold record read, one that verifies its checksum block: `store::cold_read::*`.
//!
//! # What this file closes — D-0914
//!
//! `BarFile` remembers the ONE block it verified last. A read inside that
//! block costs a relaxed load and a 56-byte `pread`; a read anywhere else —
//! every random access and every bisection probe — pays `verify_block_of` in
//! full. The gate-8 rows C-28 and C-29 re-read one fixed index, so only the
//! first path was ever timed, and the second allocated on the heap on every
//! call: `vec![0u8; span]` for the covered bytes and, on the tail block, a
//! second `vec!` for the records past the commit.
//!
//! The first test here reads `crates/store/src/file.rs` and refuses a heap
//! allocation in either function's body, which is the shape of the fix and
//! the thing a revert would undo. `#![deny(unsafe_code)]` is the workspace's
//! rule, so a counting global allocator — which needs `unsafe impl
//! GlobalAlloc` — is not available to prove the absence by measurement; the
//! source shape is the honest substitute, and it is named as one.
//!
//! The rest drive the cold path through the public door at its edges, so the
//! stack buffer is proved to hold every covered range the format produces:
//! the first and last record of every block, a commit counter that ends
//! exactly at a block boundary and one record past it, a single record, an
//! empty month, a damaged block among healthy ones, the tail an interrupted
//! append sealed past the commit, a rerun, and two handles at once.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile, StoreError};
use store::format::{BLOCK_LEN, Bar, HEADER_LEN, OI_NULL, RECORD_STRIDE, RECORDS_PER_BLOCK};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

/// The symbol id every month here is written and reopened under.
const SYMBOL: u32 = 26_000;

/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;

/// The open of the first one-minute bar of 2024-06-03, in microseconds.
const T0: i64 = 1_717_386_300_000_000;

/// `crates/store/src/file.rs`, read at compile time so a rename fails the
/// build rather than skipping the check.
const FILE_RS: &str = include_str!("../src/file.rs");

/// The `index`-th one-minute bar, distinct in every field that is not a flag.
fn bar(index: u64) -> Bar {
    let i = i64::try_from(index).expect("small");
    Bar {
        ts_micros: T0 + i * MINUTE,
        open: 2_345_600 + i,
        high: 2_345_900 + i,
        low: 2_345_100 + i,
        close: 2_345_700 + i,
        volume: 1_000 + i,
        open_interest: OI_NULL,
    }
}

/// The month under test.
fn month() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("2024-06"),
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

/// A scratch root no other test or process names, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-store-cold-read-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ignored = fs::remove_dir_all(&root);
        Self(root)
    }

    /// The bar file's own path under this root.
    fn bars(&self) -> PathBuf {
        month().to_path_buf(&self.0)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.0);
    }
}

/// A month holding `n` committed bars, written in one append and reopened
/// read-only, so the reader's one-block memory starts empty.
fn month_of(scratch: &Scratch, n: u64) -> BarFile {
    let mut writer = BarFile::open_or_create(&scratch.0, month(), SYMBOL).expect("opens");
    if n > 0 {
        let batch: Vec<Bar> = (0..n).map(bar).collect();
        assert_eq!(
            writer.append(&batch),
            Ok(Appended::Committed {
                first_index: 0,
                n_valid: n
            })
        );
    }
    drop(writer);
    BarFile::open_existing(&scratch.0, month(), SYMBOL).expect("reopens")
}

/// The body of `fn <name>` in `file.rs`: from its signature to the first
/// line that closes a method at the impl's indentation.
fn body_of(name: &str) -> &'static str {
    let signature = format!("    fn {name}");
    let start = FILE_RS
        .find(&signature)
        .unwrap_or_else(|| panic!("`fn {name}` is in crates/store/src/file.rs"));
    let rest = &FILE_RS[start..];
    let end = rest
        .find("\n    }\n")
        .unwrap_or_else(|| panic!("`fn {name}` closes"));
    &rest[..end]
}

/// Every first and last record of every block of an `n`-record month, in an
/// order where no two neighbours share a block — so every read is cold.
fn cold_order(n: u64) -> Vec<u64> {
    let blocks = n.div_ceil(RECORDS_PER_BLOCK);
    let mut out = Vec::new();
    // Ends first, then starts, so the wrap from the last block's end to block
    // 0's start is a block change too.
    for block in 0..blocks {
        out.push((block * RECORDS_PER_BLOCK + RECORDS_PER_BLOCK - 1).min(n - 1));
    }
    for block in 0..blocks {
        out.push(block * RECORDS_PER_BLOCK);
    }
    out
}

/// NEITHER FUNCTION ON THE COLD PATH ALLOCATES ON THE HEAP.
///
/// Failed before D-0914: `verify_block_of` held `vec![0u8; span]` and
/// `past_the_commit` held `vec![0u8; span]` and `Vec::new()`. The covered range
/// now goes into the handle's fixed buffer (D-1433) and the records past the
/// commit into a `[0u8; MAX_BLOCK_LEN]` on the stack through `slice_of`.
#[test]
fn the_cold_verify_path_reads_into_the_stack_not_the_heap() {
    for name in ["verify_block_of(", "past_the_commit<"] {
        let body = body_of(name);
        for heap in ["vec!", "Vec::", "Vec<", "to_vec", "Box::", "String::"] {
            assert!(
                !body.contains(heap),
                "`fn {name}` must not allocate on the heap, and holds `{heap}`"
            );
        }
        assert!(
            body.contains("MAX_BLOCK_LEN"),
            "`fn {name}` reads into the block-sized stack buffer"
        );
    }
    assert!(
        body_of("verify_block_of(").contains("[0u8; MAX_BLOCK_LEN]"),
        "the covered bytes and the records past the commit each get a stack array"
    );
    assert!(
        FILE_RS.contains("const _: () = assert!(MAX_BLOCK_LEN == 4_088);"),
        "the buffer is pinned to the bar geometry's block"
    );
    assert_eq!(BLOCK_LEN, 4_088, "and that block is 56 x 73 bytes");
}

/// Every first and last record of every block, read cold, at every commit
/// counter that puts the tail somewhere different relative to a block edge.
#[test]
fn every_block_edge_reads_cold_at_every_tail_shape() {
    // One record; one short of a block; exactly one block; one past it; two
    // whole blocks; and a ragged tail several blocks in.
    for n in [1, 72, 73, 74, 146, 1_000] {
        let scratch = Scratch::new("edges");
        let reader = month_of(&scratch, n);
        let order = cold_order(n);
        let mut previous = u64::MAX;
        for &index in &order {
            let block = index / RECORDS_PER_BLOCK;
            if n > RECORDS_PER_BLOCK {
                assert_ne!(block, previous, "n={n}: the order keeps every read cold");
            }
            previous = block;
            assert_eq!(
                reader.read_record(index),
                Ok(bar(index)),
                "n={n} record {index}"
            );
        }
        assert_eq!(
            reader.read_record(n),
            Err(StoreError::NotCommitted {
                index: n,
                n_valid: n
            }),
            "n={n}: one past the last committed record is refused, not read"
        );
    }
}

/// An empty month reads nothing and refuses index 0 by name.
#[test]
fn an_empty_month_refuses_its_first_index() {
    let scratch = Scratch::new("empty");
    let reader = month_of(&scratch, 0);
    assert_eq!(
        reader.read_record(0),
        Err(StoreError::NotCommitted {
            index: 0,
            n_valid: 0
        })
    );
    assert_eq!(
        reader.read_record(u64::MAX),
        Err(StoreError::NotCommitted {
            index: u64::MAX,
            n_valid: 0
        })
    );
}

/// A damaged block is refused on every cold touch, and its neighbours on
/// either side still read — the stack buffer is not carrying one block's
/// bytes into the next verification.
#[test]
fn a_damaged_block_is_refused_cold_and_its_neighbours_still_read() {
    let scratch = Scratch::new("damaged");
    let n = 1_000;
    drop(month_of(&scratch, n));
    // Flip one bit of the `high` of record 150, inside block 2.
    let path = scratch.bars();
    let mut bytes = fs::read(&path).expect("reads");
    let at = usize::try_from(HEADER_LEN + 150 * RECORD_STRIDE + 16).expect("small");
    bytes[at] ^= 0x20;
    fs::write(&path, &bytes).expect("writes");

    let reader = BarFile::open_existing(&scratch.0, month(), SYMBOL).expect("reopens");
    for round in 0..3 {
        assert_eq!(
            reader.read_record(145),
            Ok(bar(145)),
            "round {round}: block 1"
        );
        let refused = reader.read_record(146);
        assert!(
            matches!(refused, Err(StoreError::BlockChecksum { block: 2, .. })),
            "round {round}: block 2 refused on its first record: {refused:?}"
        );
        assert_eq!(
            reader.read_record(219),
            Ok(bar(219)),
            "round {round}: block 3"
        );
        let refused = reader.read_record(218);
        assert!(
            matches!(refused, Err(StoreError::BlockChecksum { block: 2, .. })),
            "round {round}: block 2 refused on its last record: {refused:?}"
        );
        assert_eq!(
            reader.read_record(n - 1),
            Ok(bar(n - 1)),
            "round {round}: tail"
        );
    }
}

/// The tail an interrupted append sealed past the commit is admitted on the
/// proof, which reads the records past the commit into the second stack
/// buffer — the path that held the second `vec!`.
#[test]
fn a_tail_sealed_past_the_commit_reads_cold_through_the_second_buffer() {
    let scratch = Scratch::new("tail");
    let path = scratch.bars();
    let mut writer = BarFile::open_or_create(&scratch.0, month(), SYMBOL).expect("opens");
    let held: Vec<Bar> = (0..80).map(bar).collect();
    assert!(matches!(
        writer.append(&held),
        Ok(Appended::Committed { n_valid: 80, .. })
    ));
    let region = fs::read(&path).expect("reads")[..32_768].to_vec();
    let lost: Vec<Bar> = (80..145).map(bar).collect();
    assert!(matches!(
        writer.append(&lost),
        Ok(Appended::Committed { n_valid: 145, .. })
    ));
    drop(writer);
    // The crash: the header goes back to before the second commit, while the
    // seal over all 145 and their bytes stay on disk. 145 is one record short
    // of two whole blocks, so the past fills the tail block's room to its
    // last record but one.
    let mut bytes = fs::read(&path).expect("reads");
    bytes[..32_768].copy_from_slice(&region);
    fs::write(&path, &bytes).expect("writes");

    let reader = BarFile::open_existing(&scratch.0, month(), SYMBOL).expect("reopens");
    for round in 0..4u64 {
        assert_eq!(reader.read_record(round), Ok(bar(round)), "block 0");
        assert_eq!(
            reader.read_record(73 + round),
            Ok(bar(73 + round)),
            "the tail"
        );
        assert_eq!(
            reader.read_record(72 - round),
            Ok(bar(72 - round)),
            "block 0"
        );
        assert_eq!(
            reader.read_record(79 - round),
            Ok(bar(79 - round)),
            "the tail"
        );
    }
    assert_eq!(
        reader.read_record(80),
        Err(StoreError::NotCommitted {
            index: 80,
            n_valid: 80
        }),
        "a record past the commit is still not served"
    );
}

/// Two handles reading cold at once, and the same walk twice: the same bars,
/// byte for byte, every time.
#[test]
fn cold_reads_are_idempotent_and_agree_across_concurrent_handles() {
    let scratch = Scratch::new("concurrent");
    let n = 1_000;
    drop(month_of(&scratch, n));
    let order = cold_order(n);
    let walk = || {
        let reader = BarFile::open_existing(&scratch.0, month(), SYMBOL).expect("reopens");
        let first: Vec<_> = order.iter().map(|&i| reader.read_record(i)).collect();
        let second: Vec<_> = order.iter().map(|&i| reader.read_record(i)).collect();
        assert_eq!(first, second, "a rerun on one handle reads the same bars");
        first
    };
    let (left, right) = std::thread::scope(|s| {
        let a = s.spawn(walk);
        let b = s.spawn(walk);
        (a.join().expect("joins"), b.join().expect("joins"))
    });
    assert_eq!(left, right, "two handles agree");
    let expected: Vec<_> = order.iter().map(|&i| Ok(bar(i))).collect();
    assert_eq!(left, expected, "and both read the bars written");
}
