#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the strict audit's refusal of a header or a geometry it
//! cannot hold, asked directly. Every month a door hands it was opened at a
//! bar geometry with its checksums flag set, so these inputs reach it only by
//! a direct call; each is refused before a byte of the month is read. Every
//! price is invented.

use std::sync::atomic::{AtomicU64, Ordering};

use super::{BUFFER_BYTES, Generations, HEADER_BYTES, audit};
use crate::file::BarFile;
use crate::format::{Bar, FLAG_CHECKSUMS, OI_NULL};
use crate::layout::Layout;
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use brutex_core::vendor::Vendor;

static NEXT: AtomicU64 = AtomicU64::new(0);

const REFUSED: &str =
    "strict checksum audit refuses unsealed, unsupported, non-exact or over-limit input extents";

fn path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: "QAUDIT",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2031, 2).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// **THE AUDIT REFUSES A FLAG-CLEAR HEADER AND A BLOCK LONGER THAN ITS
/// BUFFER, BY THE SAME SENTENCE, WITHOUT READING THE MONTH**; the month's own
/// header and geometry audit cleanly beside them. (A header region of another
/// size is no geometry `Layout::declare` builds: it admits two slots only.)
#[test]
fn the_audit_refuses_a_header_or_geometry_it_cannot_hold_before_reading() {
    let root = std::env::temp_dir().join(format!(
        "brutex-checksum-audit-cover-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    let (from, _) = YearMonth::new(2031, 2).unwrap().ist_bounds_micros();
    let rows: Vec<Bar> = (1..=80)
        .map(|k| Bar {
            ts_micros: from + k * 60_000_000,
            open: 31_100,
            high: 31_170,
            low: 31_030,
            close: 31_110,
            volume: 5,
            open_interest: OI_NULL,
        })
        .collect();
    BarFile::open_or_create(&root, path(), 7)
        .unwrap()
        .append(&rows)
        .unwrap();
    let file = BarFile::open_existing(&root, path(), 7).unwrap();
    let input = file.checksum_inputs().unwrap();
    let before = Generations::read(&input).unwrap();
    let (header, layout) = (file.header(), file.layout());
    assert!(audit(&input, header, layout, before, 1 << 20).is_ok());

    let flag_clear = crate::header::Header {
        flags: header.flags & !FLAG_CHECKSUMS,
        ..header
    };
    assert_eq!(layout.header_len(), HEADER_BYTES as u64);
    let longer_blocks = Layout::declare(
        layout.version(),
        layout.magic(),
        layout.slot_count(),
        layout.record_stride(),
        BUFFER_BYTES as u64 / layout.record_stride() + 1,
    )
    .unwrap();
    assert!(longer_blocks.block_len() > BUFFER_BYTES as u64);
    for (case, header, layout) in [
        ("flag clear", flag_clear, layout),
        ("block length", header, longer_blocks),
    ] {
        assert_eq!(
            audit(&input, header, layout, before, 1 << 20)
                .err()
                .as_deref(),
            Some(REFUSED),
            "{case}"
        );
    }
    drop(file);
    let _ = std::fs::remove_dir_all(&root);
}
