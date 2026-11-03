#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the provenance door's answers no other test reached:
//! a sidecar of another month of the same series, a sidecar name that holds
//! no regular file, a served generation that cannot be resolved, a sealed
//! form that cannot be opened, a sealed month's own records, a sidecar the
//! host will not open, and a landing recorded inside a durable batch. Every
//! bar, landing and name is invented.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use super::*;
use crate::file::{BatchFlush, DurableBatch};
use crate::format::{Bar, OI_NULL};
use crate::path::{PathParts, Timeframe, YearMonth};

const SYMBOL: u32 = 0x9A11;
const MINUTE: i64 = 60_000_000;
const COMMIT: &str = "89abcdef0123456789abcdef0123456789abcdef";

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-prv-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn file(&self, path: StorePath<'_>, kind: FileKind) -> PathBuf {
        path.with_file(kind).to_path_buf(&self.0)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.0);
    }
}

fn month(year: u16, m: u8) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "QPRVC",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(year, m).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// `n` invented one-minute bars on day 3 of the month, from 09:15 IST.
fn bars(path: StorePath<'_>, n: i64) -> Vec<Bar> {
    let origin = path.month().ist_bounds_micros().0 + (2 * 1_440 + 555) * MINUTE;
    (0..n)
        .map(|k| Bar {
            ts_micros: origin + k * MINUTE,
            open: 4_404_000 + k,
            high: 4_404_500,
            low: 4_403_500,
            close: 4_404_100,
            volume: 3,
            open_interest: OI_NULL,
        })
        .collect()
}

fn landing(path: StorePath<'_>, journal: u64) -> Landing<'static> {
    let month = path.month();
    Landing {
        source: Source {
            kind: SourceKind::ArchiveMember,
            name: "cover.zip!D03.zip!QPRVC.csv",
            bytes: 4_096,
            crc32: 0x0BAD_F00D,
        },
        day: Day::new(month.year(), month.month(), 3).unwrap(),
        bar_definition: 2,
        journal_ordinal: journal,
        commit: Some(COMMIT),
        landed_at_micros: 1_791_547_200_000_000,
    }
}

/// A month of `n` bars with one recorded landing.
fn landed(root: &Root, path: StorePath<'_>, n: i64) -> BarFile {
    let mut file = BarFile::open_or_create(&root.0, path, SYMBOL).unwrap();
    let appended = file.append(&bars(path, n)).unwrap();
    record(&root.0, &file, path, appended, &landing(path, 7)).unwrap();
    file
}

/// **A SIDECAR OF ANOTHER MONTH OF THE SAME SERIES IS NAMED BY ITS MONTH.**
/// Same vendor, same bar length: the first field that differs is the month,
/// whether the month or only the year differs. Its own month's sidecar is
/// `ThisMonth`.
#[test]
fn a_sidecar_of_another_month_of_the_series_is_named_by_its_month() {
    let root = Root::new("another-month");
    let june = month(2024, 6);
    drop(landed(&root, june, 3));
    let held = fs::read(root.file(june, FileKind::Provenance)).unwrap();
    assert_eq!(written_for(&root.0, june), WrittenFor::ThisMonth);
    for other in [month(2024, 7), month(2025, 6)] {
        let at = root.file(other, FileKind::Provenance);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(&at, &held).unwrap();
        assert_eq!(
            written_for(&root.0, other),
            WrittenFor::Another {
                sidecar: at,
                field: "month"
            }
        );
    }
}

/// **A SIDECAR NAME THAT HOLDS NO REGULAR FILE SAYS NOTHING.** A directory
/// at the `.prv` name is opened by the no-follow door and is not a file: the
/// month is `Unrecorded`, never another month's and never this one's.
#[test]
fn a_sidecar_name_that_holds_no_regular_file_says_nothing() {
    let root = Root::new("not-a-file");
    let june = month(2024, 6);
    fs::create_dir_all(root.file(june, FileKind::Provenance)).unwrap();
    assert_eq!(written_for(&root.0, june), WrittenFor::Unrecorded);
}

/// **A SERVED GENERATION THAT CANNOT BE RESOLVED REFUSES THE READ, NAMING THE
/// POINTER.** A directory where the month's generation pointer goes is no
/// pointer: the read is refused by the pointer's own path, never answered
/// from the original's sidecar.
#[test]
fn a_served_generation_that_cannot_be_resolved_refuses_the_read() {
    let root = Root::new("pointer");
    let june = month(2024, 6);
    drop(landed(&root, june, 3));
    let pointer = crate::generation::pointer_path(&root.0, june);
    fs::create_dir_all(&pointer).unwrap();
    match read(&root.0, june.with_file(FileKind::Provenance)) {
        Err(ProvenanceError::Io {
            path,
            action: "resolving the month's served generation",
            ..
        }) => assert_eq!(path, pointer),
        other => panic!("the pointer refuses the read: {other:?}"),
    }
}

/// **A SEALED FORM THAT CANNOT BE OPENED REFUSES THE READ.** The door asks
/// the sealed form first; a directory at the `.sbar` name is refused by that
/// name, and the raw sidecar beside it is not read in its place.
#[test]
fn a_sealed_form_that_cannot_be_opened_refuses_the_read() {
    let root = Root::new("sbar-dir");
    let june = month(2024, 6);
    drop(landed(&root, june, 3));
    let sbar = root.file(june, FileKind::SealedBars);
    fs::create_dir_all(&sbar).unwrap();
    match read(&root.0, june) {
        Err(ProvenanceError::Io {
            path,
            action: "opening the sealed month",
            ..
        }) => assert_eq!(path, sbar),
        other => panic!("the sealed form refuses the read: {other:?}"),
    }
}

/// **A SEALED MONTH CARRIES ITS OWN RECORDS TO THE READER.** With the raw
/// form and its sidecar gone, the records come from the image packed inside
/// the `.sbar`, exactly the records the raw sidecar held.
#[test]
fn a_sealed_month_alone_carries_its_records_to_the_reader() {
    let root = Root::new("sealed-alone");
    let june = month(2024, 6);
    let file = landed(&root, june, 5);
    let header = file.header();
    let held = file.read_range(0, file.records()).unwrap();
    drop(file);
    let prv_at = root.file(june, FileKind::Provenance);
    let image = fs::read(&prv_at).unwrap();
    let Found::Present(raw) = read(&root.0, june).unwrap() else {
        panic!("the raw sidecar holds a record");
    };
    let sealed_bytes = crate::sealed::build(june, &header, &held, Some(&image), None)
        .unwrap()
        .to_bytes();
    fs::write(root.file(june, FileKind::SealedBars), sealed_bytes).unwrap();
    for kind in [FileKind::Bars, FileKind::Provenance] {
        fs::remove_file(root.file(june, kind)).unwrap();
    }
    match read(&root.0, june).unwrap() {
        Found::Present(sealed) => {
            assert_eq!(sealed.records, raw.records);
            assert_eq!(sealed.records.len(), 1);
        }
        other => panic!("the sealed image's record is served: {other:?}"),
    }
}

/// **A SIDECAR THE HOST WILL NOT OPEN IS REFUSED BY ITS OPEN.** A mode-000
/// `.prv` is the host's refusal, named for the file and the open, never an
/// absent sidecar. Root opens a mode-000 file, so the body runs where the
/// mode bits bind: as `nobody` when this process is root (D-0995).
#[test]
fn a_sidecar_the_host_will_not_open_is_refused_by_its_open() {
    crate::support::where_permission_binds(
        "provenance::cover_tests::a_sidecar_the_host_will_not_open_is_refused_by_its_open",
        a_sidecar_the_host_will_not_open_is_refused_by_its_open_body,
    );
}

fn a_sidecar_the_host_will_not_open_is_refused_by_its_open_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = Root::new("closed-prv");
    let june = month(2024, 6);
    drop(landed(&root, june, 3));
    let at = root.file(june, FileKind::Provenance);
    fs::set_permissions(&at, fs::Permissions::from_mode(0o000)).unwrap();
    let refused = read(&root.0, june);
    fs::set_permissions(&at, fs::Permissions::from_mode(0o644)).unwrap();
    match refused {
        Err(ProvenanceError::Io {
            path,
            action: "open",
            source,
        }) => {
            assert_eq!(path, at);
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        other => panic!("the open is refused by name: {other:?}"),
    }
}

/// **A LANDING RECORDED INSIDE A DURABLE BATCH LEAVES ITS FLUSHES TO THE
/// BATCH.** The bars are written and flushed outside it; inside it the first
/// landing makes the sidecar (one new name) and writes its header and record.
/// The batch holds exactly that file and that directory, and making the
/// batch durable reports them.
#[test]
fn a_landing_inside_a_durable_batch_leaves_its_flushes_to_the_batch() {
    let root = Root::new("batch");
    let june = month(2024, 6);
    let mut file = BarFile::open_or_create(&root.0, june, SYMBOL).unwrap();
    let appended = file.append(&bars(june, 4)).unwrap();
    let batch = DurableBatch::new(BatchFlush::EveryFile);
    {
        let _inside = batch.enter();
        let recorded = record(&root.0, &file, june, appended, &landing(june, 9)).unwrap();
        assert!(matches!(recorded, Recorded::Written(_)), "{recorded:?}");
    }
    let made = batch.make_durable(1).unwrap();
    assert_eq!((made.files, made.directories), (1, 1), "{made:?}");
    let Found::Present(sidecar) = read(&root.0, june).unwrap() else {
        panic!("the batch's record is on disk");
    };
    assert_eq!(sidecar.records.len(), 1);
}

/// **A BATCH SEALED UNDER A WRITER LEAVES THE FLUSH TO THE WRITER.** Once a
/// batch is made durable it holds nothing more: the sidecar's data flush and
/// its directory's flush are issued by the writer itself, and succeed.
#[test]
fn a_batch_sealed_under_a_writer_leaves_the_flush_to_the_writer() {
    let root = Root::new("sealed-batch");
    let june = month(2024, 6);
    drop(landed(&root, june, 3));
    let at = root.file(june, FileKind::Provenance);
    let batch = DurableBatch::new(BatchFlush::EveryFile);
    let made = batch.make_durable(1).unwrap();
    assert_eq!((made.files, made.directories), (0, 0));
    let file = fs::File::options()
        .read(true)
        .write(true)
        .open(&at)
        .unwrap();
    assert!(sync_data(&file, &at, Some(&batch)).is_ok());
    assert!(sync_parent(&at, Some(&batch)).is_ok());
    // And an open batch holds both, saying so.
    let open = DurableBatch::new(BatchFlush::EveryFile);
    assert!(sync_data(&file, &at, Some(&open)).is_ok());
    assert!(sync_parent(&at, Some(&open)).is_ok());
    let held = open.make_durable(1).unwrap();
    assert_eq!((held.files, held.directories), (1, 1), "{held:?}");
}
