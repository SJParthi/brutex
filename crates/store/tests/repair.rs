//! Revision protocol proofs using isolated temporary stores only.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{BarFile, StoreError};
use store::format::{Bar, OI_NULL};
use store::header::Header;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::repair::{
    self, MAX_REVISIONS, MAX_ROWS, Published, RepairError, Revision, RevisionReader,
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-repair-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn source(&self, rows: &[Bar]) -> Header {
        let mut writer = BarFile::open_or_create(&self.0, path(), 7).unwrap();
        if !rows.is_empty() {
            writer.append(rows).unwrap();
        }
        writer.header()
    }

    fn publish(&self, header: Header, rows: &[Bar]) -> Result<Published, RepairError> {
        repair::publish(&self.0, path(), 7, revision(), header, rows)
    }

    fn revision_root(&self) -> PathBuf {
        self.0.join("bar-revisions-v1/1")
    }

    fn receipt(&self) -> PathBuf {
        path()
            .to_path_buf(&self.revision_root())
            .with_extension("repair-v1")
    }

    fn images(&self) -> (Vec<u8>, Vec<u8>) {
        (
            fs::read(path().to_path_buf(&self.0)).unwrap(),
            fs::read(path().with_file(FileKind::Checksums).to_path_buf(&self.0)).unwrap(),
        )
    }

    fn read(&self) -> Result<RevisionReader, RepairError> {
        RevisionReader::open(&self.0, path(), 7, revision())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Groww,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

fn revision() -> Revision {
    Revision::new(1).unwrap()
}

fn bar(index: i64) -> Bar {
    Bar {
        ts_micros: 1_717_386_300_000_000 + index * 60_000_000,
        open: 100,
        high: 110,
        low: 90,
        close: 105,
        volume: index,
        open_interest: OI_NULL,
    }
}

#[test]
fn complete_merge_preserves_original_bytes_and_concurrent_readers() {
    let fixture = Fixture::new();
    let old = [bar(1), bar(3)];
    let expected = fixture.source(&old);
    let before = fixture.images();
    let original = BarFile::open_existing(&fixture.0, path(), 7).unwrap();
    let mut merged = vec![bar(0), bar(1), bar(2), bar(3), bar(4)];
    merged[1].close = 109;
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            for _ in 0..1000 {
                assert_eq!(original.read_record(0).unwrap(), old[0]);
                assert_eq!(original.read_record(1).unwrap(), old[1]);
            }
        });
        assert_eq!(
            fixture.publish(expected, &merged).unwrap(),
            Published::Created
        );
        reader.join().unwrap();
    });
    assert_eq!(fixture.images(), before);
    assert_eq!(original.records(), 2);
    let revised = fixture.read().unwrap();
    let second_reader = fixture.read().unwrap();
    assert_eq!(revised.source_header(), expected);
    assert_eq!(revised.header().n_valid, 5);
    for (index, row) in merged.iter().enumerate() {
        assert_eq!(revised.read_record(index as u64).unwrap(), *row);
        assert_eq!(second_reader.read_record(index as u64).unwrap(), *row);
    }
    assert_eq!(revised.first_at_or_after(bar(2).ts_micros).unwrap(), 2);
    assert!(matches!(
        revised.read_record(5),
        Err(StoreError::NotCommitted { .. })
    ));
    assert!(matches!(
        BarFile::open_or_create(&fixture.revision_root(), path(), 7),
        Err(StoreError::Locked { .. })
    ));
    assert_eq!(
        BarFile::open_existing(&fixture.0, path(), 7)
            .unwrap()
            .read_record(0)
            .unwrap(),
        old[0]
    );
}

#[test]
fn exact_retry_preserves_every_revision_byte_and_rejects_conflicts() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1)]);
    let merged = [bar(0), bar(1), bar(2)];
    fixture.publish(expected, &merged).unwrap();
    let physical = path().to_path_buf(&fixture.revision_root());
    let paths = [
        physical.clone(),
        physical.with_extension("crc"),
        fixture.receipt(),
    ];
    let images: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
    // Retry identifies the completed request even after ordinary append advances.
    fixture.source(&[bar(3)]);
    assert_eq!(
        fixture.publish(expected, &merged).unwrap(),
        Published::Reused
    );
    assert_eq!(
        paths
            .iter()
            .map(|p| fs::read(p).unwrap())
            .collect::<Vec<_>>(),
        images
    );
    assert_eq!(
        fixture.publish(expected, &[bar(0), bar(1)]),
        Err(RepairError::Conflict)
    );
    let mut different = merged;
    different[1].close = 106;
    assert_eq!(
        fixture.publish(expected, &different),
        Err(RepairError::Conflict)
    );
    let mut header = expected;
    header.generation += 1;
    assert_eq!(fixture.publish(header, &merged), Err(RepairError::Conflict));
}

#[test]
fn invalid_batches_stale_sources_and_deletions_leave_no_revision_tree() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1), bar(3)]);
    let before = fixture.images();
    assert!(matches!(
        fixture.publish(expected, &[]),
        Err(RepairError::Store(StoreError::EmptyBatch))
    ));
    for rows in [vec![bar(3), bar(1)], vec![bar(1), bar(1)]] {
        assert!(matches!(
            fixture.publish(expected, &rows),
            Err(RepairError::Store(StoreError::BatchNotOrdered { .. }))
        ));
    }
    let mut invalid = bar(1);
    invalid.high = 80;
    assert!(matches!(
        fixture.publish(expected, &[invalid]),
        Err(RepairError::Store(StoreError::ImpossibleBar { .. }))
    ));
    invalid = bar(1);
    invalid.volume = -1;
    assert!(matches!(
        fixture.publish(expected, &[invalid]),
        Err(RepairError::Store(StoreError::ImpossibleCount { .. }))
    ));
    assert_eq!(
        fixture.publish(expected, &[bar(0), bar(1), bar(2)]),
        Err(RepairError::MissingTimestamp(bar(3).ts_micros))
    );
    assert_eq!(
        fixture.publish(expected, &[bar(3)]),
        Err(RepairError::MissingTimestamp(bar(1).ts_micros))
    );
    let mut stale = expected;
    stale.generation += 1;
    assert_eq!(
        fixture.publish(stale, &[bar(1), bar(3)]),
        Err(RepairError::StaleSource)
    );
    assert_eq!(fixture.images(), before);
    assert!(!fixture.0.join("bar-revisions-v1").exists());
}

#[test]
fn exact_resource_boundaries_and_bar_only_contract() {
    assert_eq!(Revision::new(0), Err(RepairError::RevisionLimit));
    assert_eq!(
        Revision::new(MAX_REVISIONS + 1),
        Err(RepairError::RevisionLimit)
    );
    assert_eq!(
        Revision::new(MAX_REVISIONS).unwrap().ordinal(),
        MAX_REVISIONS
    );
    let fixture = Fixture::new();
    let expected = fixture.source(&[]);
    let maximum = i64::try_from(MAX_ROWS).unwrap();
    let mut rows: Vec<_> = (0..maximum).map(bar).collect();
    rows.push(bar(maximum));
    assert_eq!(fixture.publish(expected, &rows), Err(RepairError::RowLimit));
    rows.pop();
    assert_eq!(
        fixture.publish(expected, &rows).unwrap(),
        Published::Created
    );
    assert_eq!(fixture.read().unwrap().header().n_valid, MAX_ROWS as u64);
    let mut too_many = expected;
    too_many.n_valid = MAX_ROWS as u64 + 1;
    assert_eq!(
        fixture.publish(too_many, &[bar(0)]),
        Err(RepairError::RowLimit)
    );
    assert!(matches!(
        repair::publish(
            &fixture.0,
            path().with_file(FileKind::Overlay),
            7,
            revision(),
            expected,
            &[bar(0)]
        ),
        Err(RepairError::Store(StoreError::NotABarPath { .. }))
    ));
    assert!(matches!(
        RevisionReader::open(
            &fixture.0,
            path().with_file(FileKind::Overlay),
            7,
            revision()
        ),
        Err(RepairError::Store(StoreError::NotABarPath { .. }))
    ));
}

#[test]
fn corrupt_or_missing_source_evidence_is_never_repaired_or_created() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1)]);
    let checksum = path()
        .with_file(FileKind::Checksums)
        .to_path_buf(&fixture.0);
    fs::write(&checksum, [0; 4]).unwrap();
    let before = fixture.images();
    assert!(matches!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Store(StoreError::BlockChecksum { .. }))
    ));
    assert_eq!(fixture.images(), before);
    fs::remove_file(&checksum).unwrap();
    assert!(matches!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Store(StoreError::ChecksumsMissing { .. }))
    ));
    assert!(!checksum.exists());
    let lock = path().with_file(FileKind::Lock).to_path_buf(&fixture.0);
    fs::remove_file(&lock).unwrap();
    assert!(matches!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        })
    ));
    assert!(!lock.exists());
    assert!(!fixture.0.join("bar-revisions-v1").exists());
}

#[test]
fn source_writer_excludes_publication_and_unsealed_sources_refuse() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1)]);
    let writer = BarFile::open_or_create(&fixture.0, path(), 7).unwrap();
    assert!(matches!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Store(StoreError::Locked { .. }))
    ));
    drop(writer);
    let mut unsealed = expected;
    unsealed.flags = 0;
    assert_eq!(
        fixture.publish(unsealed, &[bar(1)]),
        Err(RepairError::UnsealedSource)
    );
}

#[test]
fn incomplete_and_torn_publications_are_preserved_and_never_served() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1)]);
    assert!(matches!(fixture.read(), Err(RepairError::Incomplete(_))));
    assert!(!fixture.0.join("bar-revisions-v1").exists());
    let physical = path().to_path_buf(&fixture.revision_root());
    fs::create_dir_all(physical.parent().unwrap()).unwrap();
    let reserved = physical.with_extension("reserved-v1");
    fs::write(&reserved, []).unwrap();
    fs::write(&physical, b"interrupted").unwrap();
    assert!(matches!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Incomplete(_))
    ));
    assert_eq!(fs::read(&physical).unwrap(), b"interrupted");
    fs::write(fixture.receipt(), b"BRXREPV1").unwrap();
    assert!(matches!(
        fixture.read(),
        Err(RepairError::InvalidReceipt(_))
    ));
    assert!(matches!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::InvalidReceipt(_))
    ));
    assert_eq!(fs::read(fixture.receipt()).unwrap(), b"BRXREPV1");
}

#[test]
fn unexplained_revision_files_are_not_overwritten() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1)]);
    let physical = path().to_path_buf(&fixture.revision_root());
    fs::create_dir_all(physical.parent().unwrap()).unwrap();
    fs::write(&physical, b"retain me").unwrap();
    assert_eq!(
        fixture.publish(expected, &[bar(1)]),
        Err(RepairError::Incomplete(physical.clone()))
    );
    assert_eq!(fs::read(physical).unwrap(), b"retain me");
}

#[test]
fn receipt_corruption_changed_headers_and_missing_crc_refuse() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1)]);
    fixture.publish(expected, &[bar(0), bar(1)]).unwrap();
    let receipt = fs::read(fixture.receipt()).unwrap();
    for index in [0, 8, 16, 80] {
        let mut damaged = receipt.clone();
        damaged[index] ^= 1;
        fs::write(fixture.receipt(), damaged).unwrap();
        assert!(matches!(
            fixture.read(),
            Err(RepairError::InvalidReceipt(_))
        ));
    }
    fs::write(fixture.receipt(), &receipt).unwrap();
    let mut writer = BarFile::open_or_create(&fixture.revision_root(), path(), 7).unwrap();
    writer.append(&[bar(2)]).unwrap();
    drop(writer);
    assert!(matches!(
        fixture.read(),
        Err(RepairError::InvalidReceipt(_))
    ));
    // A distinct fixture tests missing checksums without a prior header failure.
    let second = Fixture::new();
    let expected = second.source(&[bar(1)]);
    second.publish(expected, &[bar(1)]).unwrap();
    let crc = path()
        .with_file(FileKind::Checksums)
        .to_path_buf(&second.revision_root());
    fs::remove_file(&crc).unwrap();
    assert!(matches!(
        second.read().unwrap().read_record(0),
        Err(StoreError::ChecksumsMissing { .. })
    ));
    assert!(!crc.exists());
}

#[test]
fn competing_publishers_cannot_mix_rows_or_checksums() {
    let fixture = Fixture::new();
    let expected = fixture.source(&[bar(1)]);
    let barrier = std::sync::Barrier::new(2);
    let outcomes = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            fixture.publish(expected, &[bar(0), bar(1)])
        });
        let b = scope.spawn(|| {
            barrier.wait();
            fixture.publish(expected, &[bar(1), bar(2)])
        });
        (a.join().unwrap(), b.join().unwrap())
    });
    assert_eq!(
        usize::from(outcomes.0.is_ok()) + usize::from(outcomes.1.is_ok()),
        1
    );
    let revised = fixture.read().unwrap();
    let rows = [
        revised.read_record(0).unwrap(),
        revised.read_record(1).unwrap(),
    ];
    assert!(rows == [bar(0), bar(1)] || rows == [bar(1), bar(2)]);
}
