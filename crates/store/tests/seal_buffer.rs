//! Sealing a block after an append allocates nothing: `store::seal_buffer::*`.
//! OD-7, D-2376, invariant AFG-76.
//!
//! `BarFile::seal_committed` re-reads every block an append touched and
//! writes its CRC-32C into the sidecar. It read each block into
//! `vec![0u8; span]` — one heap allocation per sealed block per append, on the
//! write path. It now reads every block into one `[0u8; MAX_BLOCK_LEN]` stack
//! array through `slice_of`, the door the cold read path already uses (D-0914).
//!
//! As `cold_read.rs` says, `#![deny(unsafe_code)]` rules out a counting global
//! allocator in this crate's tests, so the absence is proved on the source
//! shape, and named as that. The bytes are proved unchanged by measurement:
//! a month appended in batches that land on, across and one past every kind
//! of block edge yields a sidecar equal, entry by entry, to `store::block::seal`
//! applied to the bar file's own bytes, equal to the one-shot write's, and
//! equal to a digest pinned from the build before D-2376.

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
use store::file::{Appended, BarFile};
use store::format::{Bar, OI_NULL};
use store::layout::Layout;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

const SYMBOL: u32 = 26_000;
const MINUTE: i64 = 60_000_000;
/// The open of the first one-minute bar of 2024-06-03, in microseconds.
const T0: i64 = 1_717_386_300_000_000;
const FILE_RS: &str = include_str!("../src/file.rs");

/// 64-bit FNV-1a of the sidecar written by the build before D-2376, for the
/// batched month below. Pinned so a change to what is sealed fails here.
const SIDECAR_FNV_BEFORE_D2376: u64 = 4_413_894_360_075_297_543;

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

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-store-seal-buffer-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ignored = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a scratch store root");
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ignored = fs::remove_dir_all(&self.0);
    }
}

/// Writes `batches` in order and returns (bar file bytes, sidecar bytes).
fn written(tag: &str, batches: &[u64]) -> (Vec<u8>, Vec<u8>) {
    let scratch = Scratch::new(tag);
    let mut writer = BarFile::open_or_create(&scratch.0, month(), SYMBOL).expect("opens");
    let mut next = 0;
    for &len in batches {
        let batch: Vec<Bar> = (next..next + len).map(bar).collect();
        assert_eq!(
            writer.append(&batch),
            Ok(Appended::Committed {
                first_index: next,
                n_valid: next + len
            })
        );
        next += len;
    }
    drop(writer);
    let bars = month().to_path_buf(&scratch.0);
    (
        fs::read(&bars).expect("the bar file"),
        fs::read(bars.with_extension("crc")).expect("the sidecar"),
    )
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The body of `fn seal_committed` in `file.rs`.
fn seal_body() -> &'static str {
    let start = FILE_RS
        .find("    fn seal_committed(")
        .expect("`fn seal_committed` is in crates/store/src/file.rs");
    let rest = &FILE_RS[start..];
    &rest[..rest.find("\n    }\n").expect("`fn seal_committed` closes")]
}

/// Failed before D-2376: the body held `vec![0u8; span]`.
#[test]
fn sealing_reads_every_block_into_one_stack_buffer() {
    let body = seal_body();
    for heap in ["vec!", "Vec::", "Vec<", "to_vec", "Box::", "String::"] {
        assert!(
            !body.contains(heap),
            "`fn seal_committed` must not allocate on the heap, and holds `{heap}`"
        );
    }
    assert!(body.contains("[0u8; MAX_BLOCK_LEN]"), "one stack array");
    assert!(
        body.contains("slice_of("),
        "entered through the refusing door"
    );
}

/// The sidecar is what `block::seal` says it is, batched or one-shot, and is
/// the same bytes the build before D-2376 wrote.
#[test]
fn the_sealed_bytes_are_unchanged_at_every_block_edge() {
    // 73 records per block: lands inside, exactly on, one past, and across
    // several edges, ending mid-block.
    let batches = [1, 71, 1, 1, 73, 74, 146, 8];
    let n: u64 = batches.iter().sum();
    let (bars, sidecar) = written("batched", &batches);
    let (one_bars, one_sidecar) = written("one-shot", &[n]);
    assert_eq!(sidecar, one_sidecar, "batched seals equal one-shot seals");
    assert_eq!(bars.len(), one_bars.len());

    let layout = Layout::V3;
    let blocks = layout.blocks_for(n);
    assert_eq!(sidecar.len() as u64, blocks * 4, "one entry per block");
    for block in 0..blocks {
        let (start, end) = layout.covered_byte_range(block, n).expect("covered");
        let covered = &bars[usize::try_from(start).unwrap()..usize::try_from(end).unwrap()];
        let sum = store::block::seal(layout, n, block, covered).expect("seals");
        let at = usize::try_from(block * 4).unwrap();
        assert_eq!(
            sidecar[at..at + 4],
            sum.to_le_bytes(),
            "block {block} of {blocks}"
        );
    }
    assert_eq!(
        fnv1a(&sidecar),
        SIDECAR_FNV_BEFORE_D2376,
        "the sidecar differs from the one the build before D-2376 wrote"
    );
}
