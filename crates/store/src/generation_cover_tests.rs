#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
//! BASE-COV-LIBS: the pointer's bounded field write, `describes` against a
//! header that differs in each of its four numbers, and `served`'s and
//! `promote`'s refusals for a pointer or a path that is not what they read.

use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::*;
use crate::path::{PathParts, Timeframe, YearMonth};

struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-generation-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn month(file: FileKind) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: "QFIXG",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2019, 7).unwrap(),
        file,
    })
    .unwrap()
}

fn pointer() -> Pointer {
    Pointer {
        revision: Revision::new(3).unwrap(),
        previous: 2,
        n_valid: 40,
        first_ts_micros: 1_561_000_000_000_000,
        last_ts_micros: 1_561_000_900_000_000,
        header_generation: 6,
        promoted_at_micros: 1_791_500_000_000_000,
        record: 1,
    }
}

#[test]
fn a_record_field_is_never_written_past_its_buffer() {
    let mut short = [9u8; 4];
    put_field(&mut short, 2, &[1, 2, 3]);
    assert_eq!(short, [9; 4]);
    put_field(&mut short, usize::MAX, &[1]);
    assert_eq!(short, [9; 4]);
    put_field(&mut short, 2, &[1, 2]);
    assert_eq!(short, [9, 9, 1, 2]);
}

#[test]
fn a_pointer_describes_only_the_header_it_promoted() {
    let held = pointer();
    let header = Header {
        generation: held.header_generation,
        n_valid: held.n_valid,
        first_ts_micros: held.first_ts_micros,
        last_ts_micros: held.last_ts_micros,
        ..Header::genesis(7, 60, 0)
    };
    assert!(held.describes(&header));
    let moved: [Header; 5] = [
        Header {
            n_valid: header.n_valid + 1,
            ..header
        },
        Header {
            first_ts_micros: header.first_ts_micros - 1,
            ..header
        },
        Header {
            last_ts_micros: header.last_ts_micros + 1,
            ..header
        },
        Header {
            generation: header.generation + 1,
            ..header
        },
        Header {
            n_valid: 0,
            generation: 0,
            ..header
        },
    ];
    for (at, other) in moved.iter().enumerate() {
        assert!(!held.describes(other), "header {at}");
    }
}

#[test]
fn a_pointer_that_is_no_file_or_too_long_is_refused_and_a_short_one_serves_the_original() {
    let root = Root::new("served");
    let path = month(FileKind::Bars);
    let at = pointer_path(&root.0, path);
    fs::create_dir_all(&at).unwrap();
    assert!(matches!(
        served(&root.0, path),
        Err(StoreError::Generation { reason, .. }) if reason == "it is not a regular file"
    ));
    fs::remove_dir(&at).unwrap();
    let file = File::create(&at).unwrap();
    file.set_len(MAX_LEN + 1).unwrap();
    assert!(matches!(
        served(&root.0, path),
        Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
    ));
    // Exactly the longest a pointer may be is read, not refused by length.
    file.set_len(MAX_LEN).unwrap();
    assert!(!matches!(
        served(&root.0, path),
        Err(StoreError::Generation { reason, .. }) if reason.contains("past the")
    ));
    // Fewer bytes than one record: a torn first write, the original served.
    file.set_len(RECORD_LEN - 1).unwrap();
    assert_eq!(served(&root.0, path), Ok(Served::Original));
    // A path that is not a month's bars has no pointer at all.
    assert_eq!(
        served(&root.0, month(FileKind::Checksums)),
        Ok(Served::Original)
    );
}

#[test]
fn only_a_months_bars_are_promoted_and_a_leaf_outside_the_root_has_no_link() {
    let root = Root::new("promote");
    let refused = promote(
        &root.0,
        month(FileKind::Checksums),
        7,
        Revision::new(2).unwrap(),
    );
    assert!(matches!(
        refused,
        Err(RepairError::Store(StoreError::NotABarPath {
            found: FileKind::Checksums
        }))
    ));
    assert!(
        fs::read_dir(&root.0).unwrap().next().is_none(),
        "nothing written"
    );
    assert!(refuse_links(&root.0, Path::new("/elsewhere/entirely")).is_ok());
}
