//! The `.prv` provenance sidecar through real files — D-3334, DB-078, DB-G56.
//!
//! Every test builds its own store root under the temporary directory from
//! invented bars (prices no real index ever printed), opens a real month
//! through `BarFile`, and records landings through `store::provenance`. No
//! vendor row is read or copied.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod support;

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::file::{Appended, BarFile};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::provenance::{
    self, Day, Fault, Found, HEADER_LEN, Header, HeaderFault, Landing, LandingFault, MAGIC,
    MAX_FILE_LEN, MAX_RECORDS, NAME_CAP, ProvenanceError, RECORD_STRIDE, Record, Recorded, Source,
    SourceKind, VERSION,
};

const FORMAT_DOC: &str = include_str!("../../../docs/02-store-format.md");

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A store root that removes itself.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-prv-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn writer(&self, path: StorePath<'_>) -> BarFile {
        BarFile::open_or_create(&self.0, path, SYMBOL).unwrap()
    }

    fn prv(&self, path: StorePath<'_>) -> PathBuf {
        path.with_file(FileKind::Provenance).to_path_buf(&self.0)
    }

    fn read(&self, path: StorePath<'_>) -> Result<Found, ProvenanceError> {
        provenance::read(&self.0, path)
    }

    fn present(&self, path: StorePath<'_>) -> provenance::Sidecar {
        match self.read(path).unwrap() {
            Found::Present(sidecar) => sidecar,
            other => panic!("expected a sidecar, found {other:?}"),
        }
    }

    fn record(
        &self,
        bars: &BarFile,
        path: StorePath<'_>,
        appended: Appended,
        landing: &Landing<'_>,
    ) -> Result<Recorded, ProvenanceError> {
        provenance::record(&self.0, bars, path, appended, landing)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SYMBOL: u32 = 4_242;
const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
/// 2024-06-01 00:00 IST.
const JUNE_START: i64 = 1_717_180_200_000_000;
const MINUTE: i64 = 60_000_000;
/// 2026-10-09 12:00 UTC, the time a record is said to be written.
const LANDED: i64 = 1_791_547_200_000_000;

fn month(symbol: &'static str, year: u16, m: u8) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(year, m).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

fn june() -> StorePath<'static> {
    month("NIFTY", 2024, 6)
}

/// `n` invented one-minute bars on day `day` of June 2024, from minute
/// `from` after 09:15 IST.
fn bars(day: i64, from: i64, n: i64) -> Vec<Bar> {
    (from..from + n)
        .map(|k| Bar {
            ts_micros: JUNE_START + day * 86_400_000_000 + 555 * MINUTE + k * MINUTE,
            open: 1_111_100,
            high: 1_111_300,
            low: 1_111_000,
            close: 1_111_200,
            volume: 7,
            open_interest: OI_NULL,
        })
        .collect()
}

fn day(d: u8) -> Day {
    Day::new(2024, 6, d).unwrap()
}

fn landing(name: &str, d: u8, journal: u64) -> Landing<'_> {
    Landing {
        source: Source {
            kind: SourceKind::ArchiveMember,
            name,
            bytes: 9_999,
            crc32: 0xDEAD_BEEF,
        },
        day: day(d),
        bar_definition: 2,
        journal_ordinal: journal,
        commit: Some(COMMIT),
        landed_at_micros: LANDED,
    }
}

/// A byte offset as an index.
fn ix(offset: u64) -> usize {
    usize::try_from(offset).unwrap()
}

fn digest(path: &Path) -> [u8; 32] {
    brutex_core::blake3::hash(&fs::read(path).unwrap())
}

fn written(recorded: Recorded) -> Record {
    match recorded {
        Recorded::Written(record) => record,
        Recorded::NothingAppended => panic!("expected a record"),
    }
}

/// A month with one recorded landing per batch.
fn landed_month(root: &Root, path: StorePath<'_>, batches: &[Vec<Bar>]) -> BarFile {
    let mut file = root.writer(path);
    for (n, batch) in batches.iter().enumerate() {
        let appended = file.append(batch).unwrap();
        let name = format!("2024.zip!D{n:02}.zip!NIFTY.csv");
        let d = u8::try_from(n + 3).unwrap();
        root.record(&file, path, appended, &landing(&name, d, 100 + n as u64))
            .unwrap();
    }
    file
}

/// **The bars are untouched.** The `.bin`, `.crc`, `.tix` and `.lock` have the
/// same digest before and after two records are written beside them, and the
/// sidecar reads back the two landings exactly.
#[test]
fn the_bar_files_are_byte_identical_after_a_record() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let first = file.append(&bars(3, 0, 40)).unwrap();
    let siblings = [
        FileKind::Bars,
        FileKind::Checksums,
        FileKind::TimeIndex,
        FileKind::Lock,
    ];
    let before: Vec<_> = siblings
        .iter()
        .map(|&kind| digest(&path.with_file(kind).to_path_buf(&root.0)))
        .collect();

    let one = written(
        root.record(
            &file,
            path,
            first,
            &landing("2024.zip!D03.zip!NIFTY.csv", 3, 11),
        )
        .unwrap(),
    );
    let after: Vec<_> = siblings
        .iter()
        .map(|&kind| digest(&path.with_file(kind).to_path_buf(&root.0)))
        .collect();
    assert_eq!(before, after, "recording a landing touched a bar sibling");

    let second = file.append(&bars(4, 0, 25)).unwrap();
    let before: Vec<_> = siblings
        .iter()
        .map(|&kind| digest(&path.with_file(kind).to_path_buf(&root.0)))
        .collect();
    let two = written(
        root.record(
            &file,
            path,
            second,
            &landing("2024.zip!D04.zip!NIFTY.csv", 4, 12),
        )
        .unwrap(),
    );
    let after: Vec<_> = siblings
        .iter()
        .map(|&kind| digest(&path.with_file(kind).to_path_buf(&root.0)))
        .collect();
    assert_eq!(before, after, "the second record touched a bar sibling");

    assert_eq!((one.ordinal, one.first_index, one.n_bars), (0, 0, 40));
    assert_eq!((two.ordinal, two.first_index, two.n_bars), (1, 40, 25));
    assert_eq!(one.bar_file_generation + 1, two.bar_file_generation);
    assert_eq!(two.bar_file_generation, file.header().generation);
    assert_eq!(one.vendor, Vendor::Gdfl);

    let sidecar = root.present(path);
    assert_eq!(sidecar.records(), &[one.clone(), two.clone()]);
    assert_eq!(sidecar.tail_bytes(), 0);
    assert_eq!(sidecar.header().symbol_id, SYMBOL);
    assert_eq!(sidecar.header().vendor, Vendor::Gdfl);
    assert_eq!(sidecar.header().timeframe_secs, 60);
    assert_eq!((sidecar.header().year, sidecar.header().month), (2024, 6));
    assert_eq!(
        fs::metadata(root.prv(path)).unwrap().len(),
        HEADER_LEN + 2 * RECORD_STRIDE
    );
    sidecar.check_bars(&root.prv(path), &file).unwrap();

    // Every field the landing stated comes back as stated.
    assert_eq!(one.source_kind, SourceKind::ArchiveMember);
    assert_eq!(one.source_name, "2024.zip!D03.zip!NIFTY.csv");
    assert!(!one.source_name_truncated);
    assert_eq!(
        one.source_name_blake3,
        brutex_core::blake3::hash(b"2024.zip!D03.zip!NIFTY.csv")
    );
    assert_eq!(one.source_name_blake3_hex().len(), 64);
    assert_eq!(one.source_bytes, 9_999);
    assert_eq!(one.source_crc32, 0xDEAD_BEEF);
    assert_eq!(one.day, day(3));
    assert_eq!(one.bar_definition, 2);
    assert_eq!(one.journal_ordinal, 11);
    assert_eq!(one.commit_hex().as_deref(), Some(COMMIT));
    assert_eq!(one.landed_at_micros, LANDED);
}

/// **A re-offer that wrote nothing records nothing and opens nothing.**
#[test]
fn an_already_present_append_records_nothing() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let batch = bars(3, 0, 10);
    file.append(&batch).unwrap();
    let again = file.append(&batch).unwrap();
    assert!(matches!(again, Appended::AlreadyPresent { .. }));
    assert_eq!(
        root.record(&file, path, again, &landing("x.csv", 3, 1))
            .unwrap(),
        Recorded::NothingAppended
    );
    assert!(
        !root.prv(path).exists(),
        "a sidecar was created for nothing"
    );
    assert_eq!(root.read(path).unwrap(), Found::Absent);
}

/// **A month written before D-3334 reads as absent**, through any sibling's
/// path, and a missing month directory is the same answer.
#[test]
fn a_month_without_a_sidecar_reads_absent() {
    let root = Root::new();
    let path = june();
    root.writer(path).append(&bars(3, 0, 5)).unwrap();
    assert_eq!(root.read(path).unwrap(), Found::Absent);
    assert_eq!(
        root.read(path.with_file(FileKind::Checksums)).unwrap(),
        Found::Absent
    );
    assert_eq!(
        root.read(month("BANKNIFTY", 2024, 6)).unwrap(),
        Found::Absent
    );
}

/// **Each bar is answered by the landing that appended it**, a bar no landing
/// recorded is answered by none, and the boundaries are exact.
#[test]
fn covering_names_the_landing_of_each_bar_and_none_for_an_unrecorded_one() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let a = file.append(&bars(3, 0, 10)).unwrap();
    root.record(&file, path, a, &landing("a.csv", 3, 1))
        .unwrap();
    // A landing whose record never reached the disk: bars 10..15 unrecorded.
    file.append(&bars(4, 0, 5)).unwrap();
    let c = file.append(&bars(5, 0, 7)).unwrap();
    root.record(&file, path, c, &landing("c.csv", 5, 3))
        .unwrap();

    let sidecar = root.present(path);
    let name_of = |index: u64| {
        sidecar
            .covering(index)
            .map(|record| record.source_name.clone())
    };
    assert_eq!(name_of(0).as_deref(), Some("a.csv"));
    assert_eq!(name_of(9).as_deref(), Some("a.csv"));
    for unrecorded in 10..15 {
        assert_eq!(name_of(unrecorded), None, "bar {unrecorded}");
    }
    assert_eq!(name_of(15).as_deref(), Some("c.csv"));
    assert_eq!(name_of(21).as_deref(), Some("c.csv"));
    assert_eq!(name_of(22), None, "past the last bar");
    assert_eq!(name_of(u64::MAX), None);
    assert!(sidecar.records()[0].holds(9));
    assert!(!sidecar.records()[0].holds(10));
    assert_eq!(sidecar.records()[1].end(), 22);
}

/// **A landing must be the handle's newest commit**, the handle must be the
/// path's bar file, and the path must name the bars.
#[test]
fn a_landing_is_recorded_only_against_its_own_newest_commit() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let first = file.append(&bars(3, 0, 10)).unwrap();
    file.append(&bars(4, 0, 10)).unwrap();
    match root.record(&file, path, first, &landing("a.csv", 3, 1)) {
        Err(ProvenanceError::NotTheNewestCommit { n_valid, records }) => {
            assert_eq!((n_valid, records), (10, 20));
        }
        other => panic!("expected NotTheNewestCommit, got {other:?}"),
    }
    let newest = Appended::Committed {
        first_index: 10,
        n_valid: 20,
    };
    match root.record(
        &file,
        path.with_file(FileKind::Checksums),
        newest,
        &landing("a.csv", 4, 1),
    ) {
        Err(ProvenanceError::NotABarPath { found }) => assert_eq!(found, FileKind::Checksums),
        other => panic!("expected NotABarPath, got {other:?}"),
    }
    match root.record(
        &file,
        month("BANKNIFTY", 2024, 6),
        newest,
        &landing("a.csv", 4, 1),
    ) {
        Err(ProvenanceError::WrongBarFile { expected, handle }) => {
            assert!(expected.ends_with("BANKNIFTY/1min/2024-06.bin"));
            assert_eq!(handle, file.path());
        }
        other => panic!("expected WrongBarFile, got {other:?}"),
    }
    let empty = Appended::Committed {
        first_index: 20,
        n_valid: 20,
    };
    assert!(matches!(
        root.record(&file, path, empty, &landing("a.csv", 4, 1)),
        Err(ProvenanceError::ImpossibleRange {
            first_index: 20,
            n_valid: 20
        })
    ));
    let reversed = Appended::Committed {
        first_index: 21,
        n_valid: 20,
    };
    assert!(matches!(
        root.record(&file, path, reversed, &landing("a.csv", 4, 1)),
        Err(ProvenanceError::ImpossibleRange { .. })
    ));
    assert!(
        !root.prv(path).exists(),
        "a refused landing created a sidecar"
    );
    assert!(
        root.record(&file, path, newest, &landing("a.csv", 4, 1))
            .is_ok()
    );
}

/// **Records stay in bar order**: a landing that overlaps the last one, or
/// repeats its generation, is refused, and the sidecar is unchanged.
#[test]
fn an_overlapping_or_repeated_landing_is_refused() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let a = file.append(&bars(3, 0, 10)).unwrap();
    root.record(&file, path, a, &landing("a.csv", 3, 1))
        .unwrap();
    let before = fs::read(root.prv(path)).unwrap();
    match root.record(&file, path, a, &landing("a.csv", 3, 1)) {
        Err(ProvenanceError::OutOfOrder {
            ordinal,
            previous_end,
            previous_generation,
            first_index,
            generation,
            ..
        }) => {
            assert_eq!(ordinal, 1);
            assert_eq!((previous_end, first_index), (10, 0));
            assert_eq!(previous_generation, generation);
        }
        other => panic!("expected OutOfOrder, got {other:?}"),
    }
    file.append(&bars(4, 0, 5)).unwrap();
    let claims_old_bars = Appended::Committed {
        first_index: 5,
        n_valid: 15,
    };
    assert!(matches!(
        root.record(&file, path, claims_old_bars, &landing("b.csv", 4, 2)),
        Err(ProvenanceError::OutOfOrder {
            previous_end: 10,
            first_index: 5,
            ..
        })
    ));
    assert_eq!(fs::read(root.prv(path)).unwrap(), before);
}

/// **An interrupted append is tail, never a record, and the next landing
/// writes over it.** A partial record and a whole record of zeros both count.
#[test]
fn a_torn_tail_is_reported_and_overwritten_by_the_next_landing() {
    let root = Root::new();
    let path = june();
    let mut file = landed_month(&root, path, &[bars(3, 0, 10)]);
    let prv = root.prv(path);
    fs::OpenOptions::new()
        .append(true)
        .open(&prv)
        .unwrap()
        .write_all(&[0xA5; 100])
        .unwrap();
    let sidecar = root.present(path);
    assert_eq!(sidecar.records().len(), 1);
    assert_eq!(sidecar.tail_bytes(), 100);

    let b = file.append(&bars(4, 0, 5)).unwrap();
    let two = written(
        root.record(&file, path, b, &landing("b.csv", 4, 2))
            .unwrap(),
    );
    assert_eq!(two.ordinal, 1);
    let sidecar = root.present(path);
    assert_eq!(sidecar.records().len(), 2);
    assert_eq!(sidecar.tail_bytes(), 0);
    assert_eq!(
        fs::metadata(&prv).unwrap().len(),
        HEADER_LEN + 2 * RECORD_STRIDE
    );

    // Two whole records of zeros: the host extended the file and the bytes
    // never arrived. Neither is a record, and neither refuses the sidecar.
    fs::OpenOptions::new()
        .append(true)
        .open(&prv)
        .unwrap()
        .write_all(&[0; 512])
        .unwrap();
    let sidecar = root.present(path);
    assert_eq!(sidecar.records().len(), 2);
    assert_eq!(sidecar.tail_bytes(), 512);
    let c = file.append(&bars(5, 0, 5)).unwrap();
    assert_eq!(
        written(
            root.record(&file, path, c, &landing("c.csv", 5, 3))
                .unwrap()
        )
        .ordinal,
        2
    );
    let sidecar = root.present(path);
    assert_eq!(sidecar.records().len(), 3);
    assert_eq!(
        sidecar.tail_bytes(),
        256,
        "the second zero record is still tail"
    );
}

/// **A damaged last record is tail; a damaged record with a valid one after it
/// refuses the whole sidecar**, for the reader and for the next landing.
#[test]
fn damage_inside_the_history_is_refused_by_ordinal() {
    let root = Root::new();
    let path = june();
    let mut file = landed_month(&root, path, &[bars(3, 0, 4), bars(4, 0, 4), bars(5, 0, 4)]);
    let prv = root.prv(path);
    let pristine = fs::read(&prv).unwrap();

    let mut last = pristine.clone();
    last[ix(HEADER_LEN + 2 * RECORD_STRIDE + 70)] ^= 1;
    fs::write(&prv, &last).unwrap();
    let sidecar = root.present(path);
    assert_eq!(
        sidecar.records().len(),
        2,
        "the damaged last record is not served"
    );
    assert_eq!(sidecar.tail_bytes(), RECORD_STRIDE);

    let mut middle = pristine;
    middle[ix(HEADER_LEN + RECORD_STRIDE + 70)] ^= 1;
    fs::write(&prv, &middle).unwrap();
    match root.read(path) {
        Err(ProvenanceError::Damaged { ordinal, path: at }) => {
            assert_eq!(ordinal, 1);
            assert_eq!(at, prv);
        }
        other => panic!("expected Damaged, got {other:?}"),
    }
    let d = file.append(&bars(6, 0, 4)).unwrap();
    assert!(matches!(
        root.record(&file, path, d, &landing("d.csv", 6, 4)),
        Err(ProvenanceError::Damaged { ordinal: 1, .. })
    ));
    assert_eq!(fs::read(&prv).unwrap(), middle, "a refused landing wrote");
}

/// **A sidecar copied beside another month, instrument or rung is refused by
/// the field that differs**, and the bars' own symbol is checked.
#[test]
fn a_sidecar_is_bound_to_its_month() {
    let root = Root::new();
    let path = june();
    let file = landed_month(&root, path, &[bars(3, 0, 4)]);
    let bytes = fs::read(root.prv(path)).unwrap();

    for (other, field) in [
        (month("NIFTY", 2024, 7), "month"),
        (month("BANKNIFTY", 2024, 6), "store path"),
        (
            StorePath::new(PathParts {
                vendor: Vendor::Gdfl,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "NIFTY",
                contract: None,
                timeframe: Timeframe::MINUTE_5,
                month: YearMonth::new(2024, 6).unwrap(),
                file: FileKind::Bars,
            })
            .unwrap(),
            "timeframe",
        ),
        (
            StorePath::new(PathParts {
                vendor: Vendor::Zerodha,
                exchange: "NSE",
                segment: "INDEX",
                symbol: "NIFTY",
                contract: None,
                timeframe: Timeframe::MINUTE_1,
                month: YearMonth::new(2024, 6).unwrap(),
                file: FileKind::Bars,
            })
            .unwrap(),
            "vendor",
        ),
    ] {
        let at = root.prv(other);
        fs::create_dir_all(at.parent().unwrap()).unwrap();
        fs::write(&at, &bytes).unwrap();
        match root.read(other) {
            Err(ProvenanceError::Header {
                fault: HeaderFault::Mismatch(found),
                ..
            }) => assert_eq!(found, field),
            result => panic!("expected a {field} mismatch, got {result:?}"),
        }
    }

    // The bar file of another symbol id: the writer and `check_bars` refuse.
    let sidecar = root.present(path);
    let other_root = Root::new();
    let mut stranger = BarFile::open_or_create(&other_root.0, path, SYMBOL + 1).unwrap();
    let appended = stranger.append(&bars(3, 0, 4)).unwrap();
    fs::copy(root.prv(path), other_root.prv(path)).unwrap();
    assert!(matches!(
        sidecar.check_bars(&other_root.prv(path), &stranger),
        Err(ProvenanceError::Header {
            fault: HeaderFault::Mismatch("symbol id"),
            ..
        })
    ));
    let next = stranger.append(&bars(4, 0, 4)).unwrap();
    assert!(matches!(
        other_root.record(&stranger, path, next, &landing("x.csv", 4, 9)),
        Err(ProvenanceError::Header {
            fault: HeaderFault::Mismatch("symbol id"),
            ..
        })
    ));
    assert!(matches!(appended, Appended::Committed { .. }));
    drop(file);
}

/// **A record past the bars the bar file committed is refused** by
/// `check_bars`: the bar file was replaced after its landings were recorded.
#[test]
fn check_bars_refuses_a_record_past_the_commit() {
    let root = Root::new();
    let path = june();
    let _file = landed_month(&root, path, &[bars(3, 0, 10)]);
    let sidecar = root.present(path);

    let other = Root::new();
    let mut shorter = other.writer(path);
    shorter.append(&bars(3, 0, 6)).unwrap();
    match sidecar.check_bars(&root.prv(path), &shorter) {
        Err(ProvenanceError::PastTheBars {
            ordinal,
            end,
            n_valid,
            ..
        }) => assert_eq!((ordinal, end, n_valid), (0, 10, 6)),
        other => panic!("expected PastTheBars, got {other:?}"),
    }
}

/// **A sidecar whose header never reached the disk is unborn**, and the next
/// landing writes the header again.
#[test]
fn an_unborn_sidecar_holds_nothing_and_is_born_by_the_next_landing() {
    for seed in [Vec::new(), vec![0u8; 64], vec![0u8; 300], vec![0x5A; 10]] {
        let root = Root::new();
        let path = june();
        let mut file = root.writer(path);
        fs::write(root.prv(path), &seed).unwrap();
        assert_eq!(
            root.read(path).unwrap(),
            Found::Unborn {
                bytes: seed.len() as u64
            }
        );
        let a = file.append(&bars(3, 0, 3)).unwrap();
        let one = written(
            root.record(&file, path, a, &landing("a.csv", 3, 1))
                .unwrap(),
        );
        assert_eq!(one.ordinal, 0);
        let sidecar = root.present(path);
        assert_eq!(sidecar.records(), &[one]);
        assert_eq!(sidecar.header().symbol_id, SYMBOL);
    }
}

/// **A non-zero header that fails its checksum is damage, not an unborn
/// file.**
#[test]
fn a_damaged_header_is_refused_not_reborn() {
    let root = Root::new();
    let path = june();
    let mut file = landed_month(&root, path, &[bars(3, 0, 3)]);
    let prv = root.prv(path);
    let mut bytes = fs::read(&prv).unwrap();
    bytes[13] ^= 0x40;
    fs::write(&prv, &bytes).unwrap();
    assert!(matches!(
        root.read(path),
        Err(ProvenanceError::Header {
            fault: HeaderFault::Checksum,
            ..
        })
    ));
    let b = file.append(&bars(4, 0, 3)).unwrap();
    assert!(matches!(
        root.record(&file, path, b, &landing("b.csv", 4, 2)),
        Err(ProvenanceError::Header {
            fault: HeaderFault::Checksum,
            ..
        })
    ));
    assert_eq!(fs::read(&prv).unwrap(), bytes);
}

/// **Every header field is checked**, each failure named, the checksum first.
#[test]
fn every_header_fault_is_named() {
    let good = Header::of(june(), SYMBOL);
    let image = good.image();
    assert_eq!(Header::decode(&image), Ok(good));
    let resealed = |mut bytes: [u8; 64]| {
        let crc = store::crc::crc32c(&bytes[..60]);
        bytes[60..].copy_from_slice(&crc.to_le_bytes());
        bytes
    };
    let mut magic = image;
    magic[7] = b'9';
    assert_eq!(Header::decode(&resealed(magic)), Err(HeaderFault::Magic));
    let mut version = image;
    version[8..10].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(
        Header::decode(&resealed(version)),
        Err(HeaderFault::Version(2))
    );
    let mut stride = image;
    stride[10..12].copy_from_slice(&128u16.to_le_bytes());
    assert_eq!(
        Header::decode(&resealed(stride)),
        Err(HeaderFault::Stride(128))
    );
    for reserved in [23usize, 56, 59] {
        let mut bytes = image;
        bytes[reserved] = 1;
        assert_eq!(
            Header::decode(&resealed(bytes)),
            Err(HeaderFault::Reserved),
            "byte {reserved}"
        );
    }
    let mut vendor = image;
    vendor[24..32].copy_from_slice(b"nobody\0\0");
    assert_eq!(Header::decode(&resealed(vendor)), Err(HeaderFault::Vendor));
    let mut torn = image;
    torn[40] ^= 1;
    assert_eq!(Header::decode(&torn), Err(HeaderFault::Checksum));

    for (fault, words) in [
        (HeaderFault::Checksum, "fails its checksum"),
        (HeaderFault::Magic, "BRUTEXP1"),
        (HeaderFault::Version(2), "version 2"),
        (HeaderFault::Stride(128), "stride is 128"),
        (HeaderFault::Reserved, "reserved"),
        (HeaderFault::Vendor, "names no vendor"),
        (
            HeaderFault::Mismatch("month"),
            "its month is not this month's",
        ),
    ] {
        assert!(fault.to_string().contains(words), "{fault:?}");
    }
}

/// A record with every field distinct, at ordinal `ordinal`.
fn sample(ordinal: u64) -> Record {
    Record {
        ordinal,
        vendor: Vendor::Zerodha,
        source_kind: SourceKind::VendorEndpoint,
        source_bytes: 0x0102_0304_0506_0708,
        source_crc32: 0x1112_1314,
        source_name: "/instruments/historical/1/minute".to_owned(),
        source_name_truncated: false,
        source_name_blake3: brutex_core::blake3::hash(b"/instruments/historical/1/minute"),
        day: Day::new(2026, 9, 30).unwrap(),
        first_index: 0x2122_2324_2526_2728,
        n_bars: 0x31,
        bar_definition: 0x4142_4344,
        journal_ordinal: 0x5152_5354_5556_5758,
        bar_file_generation: 0x6162_6364_6566_6768,
        commit: Some([0xC0; 20]),
        landed_at_micros: 0x7172_7374_7576_7778,
    }
}

/// The text of §8.3, from its heading to §9.
fn section_8_3() -> &'static str {
    let start = FORMAT_DOC.find("### 8.3 `.prv`").expect("§8.3 exists");
    let rest = &FORMAT_DOC[start..];
    &rest[..rest.find("\n## 9.").expect("§9 follows")]
}

/// **The record's bytes are the table's.** Every row of §8.3's record table
/// names the offset and size its field is encoded at, and the stated
/// constants are the code's.
#[test]
fn the_store_format_doc_states_the_record_this_module_writes() {
    let section = section_8_3();
    assert!(section.contains(&format!(
        "Magic `{}`, version `{VERSION}`, stride `{RECORD_STRIDE}`, at most **{MAX_RECORDS}** records",
        String::from_utf8_lossy(&MAGIC)
    )));
    assert_eq!(MAX_FILE_LEN, 262_208);
    assert!(section.contains("never longer than **262,208** bytes"));
    assert!(section.contains("**Header, 64 bytes at offset 0:**"));
    assert!(section.contains("**Record *n*, 256 bytes at `64 + 256·n`:**"));
    assert_eq!(NAME_CAP, 128);

    let record = sample(5);
    let image = record.image();
    let le = |offset: usize, size: usize| -> u64 {
        let mut word = [0u8; 8];
        word[..size].copy_from_slice(&image[offset..offset + size]);
        u64::from_le_bytes(word)
    };
    assert_eq!(&image[0..8], b"zerodha\0");
    assert_eq!(image[8], 3);
    assert_eq!(image[9], 1, "commit known, name whole");
    assert_eq!(le(10, 2), 32);
    assert_eq!(le(12, 4), 0x4142_4344);
    assert_eq!(le(16, 8), 0x0102_0304_0506_0708);
    assert_eq!(le(24, 4), 0x1112_1314);
    assert_eq!(le(28, 4), 20_260_930);
    assert_eq!(&image[32..64], &record.source_name_blake3);
    assert_eq!(le(64, 8), 0x2122_2324_2526_2728);
    assert_eq!(le(72, 8), 0x31);
    assert_eq!(le(80, 8), 0x5152_5354_5556_5758);
    assert_eq!(le(88, 8), 0x6162_6364_6566_6768);
    assert_eq!(le(96, 8), 0x7172_7374_7576_7778);
    assert_eq!(&image[104..124], &[0xC0; 20]);
    assert_eq!(&image[124..156], b"/instruments/historical/1/minute");
    assert!(image[156..252].iter().all(|&b| b == 0));
    let mut sealed = image[..252].to_vec();
    sealed.extend_from_slice(&5u64.to_le_bytes());
    assert_eq!(le(252, 4), u64::from(store::crc::crc32c(&sealed)));
    for (offset, size, name) in [
        (0, 8, "vendor"),
        (8, 1, "source_kind"),
        (9, 1, "flags"),
        (10, 2, "name_len"),
        (12, 4, "bar_definition"),
        (16, 8, "source_bytes"),
        (24, 4, "source_crc32"),
        (28, 4, "ist_day"),
        (32, 32, "source_name_blake3"),
        (64, 8, "first_index"),
        (72, 8, "n_bars"),
        (80, 8, "journal_ordinal"),
        (88, 8, "bar_file_generation"),
        (96, 8, "landed_at"),
        (104, 20, "commit"),
        (124, 128, "source_name"),
        (252, 4, "record_crc"),
    ] {
        assert!(
            section.contains(&format!("| {offset} | {size} | `{name}` |")),
            "§8.3 must state `{name}` at {offset}, {size} bytes"
        );
    }
}

/// **The header's bytes are the table's**, and §8 lists the sibling.
#[test]
fn the_store_format_doc_states_the_header_this_module_writes() {
    let section = section_8_3();
    let header = Header::of(june(), 0x0A0B_0C0D).image();
    assert_eq!(&header[0..8], b"BRUTEXP1");
    assert_eq!(u16::from_le_bytes([header[8], header[9]]), 1);
    assert_eq!(u16::from_le_bytes([header[10], header[11]]), 256);
    assert_eq!(u32::from_le_bytes(header[12..16].try_into().unwrap()), 60);
    assert_eq!(
        u32::from_le_bytes(header[16..20].try_into().unwrap()),
        0x0A0B_0C0D
    );
    assert_eq!(u16::from_le_bytes([header[20], header[21]]), 2024);
    assert_eq!(header[22], 6);
    assert_eq!(&header[24..32], b"gdfl\0\0\0\0");
    assert_eq!(
        &header[32..56],
        &brutex_core::blake3::hash(b"bars/gdfl/NSE/INDEX/NIFTY/1min/2024-06.bin")[..24]
    );
    assert_eq!(
        u32::from_le_bytes(header[60..64].try_into().unwrap()),
        store::crc::crc32c(&header[..60])
    );
    for row in [
        "| 0 | 8 | magic `BRUTEXP1` |",
        "| 8 | 2 | version `1` |",
        "| 10 | 2 | record stride `256` |",
        "| 12 | 4 | timeframe seconds, as the bar header |",
        "| 16 | 4 | symbol id, as the bar header |",
        "| 20 | 2 | year of the month |",
        "| 22 | 1 | month |",
        "| 23 | 1 | reserved, zero |",
        "| 56 | 4 | reserved, zero |",
        "| 60 | 4 | CRC-32C over bytes `0..60` |",
    ] {
        assert!(section.contains(row), "§8.3 must state {row}");
    }
    assert!(section.contains("| 24 | 8 | vendor, its store name zero-padded"));
    assert!(
        section.contains("| 32 | 24 | the first 24 bytes of BLAKE3 over the bar file's store path")
    );
    assert!(FORMAT_DOC.contains(
        "| `.prv` | where the month's bars came from, one record per landing, version 1 (§8.3, D-3334) |"
    ));
}

/// **A record decodes to itself**, and its checksum binds it to its ordinal.
#[test]
fn a_record_round_trips_and_is_bound_to_its_position() {
    let record = sample(7);
    assert_eq!(Record::decode(&record.image(), 7), Ok(record.clone()));
    assert_eq!(Record::decode(&record.image(), 6), Err(Fault::Checksum));
    let unstamped = Record {
        commit: None,
        ..sample(0)
    };
    let image = unstamped.image();
    assert_eq!(image[9], 0);
    assert!(image[104..124].iter().all(|&b| b == 0));
    assert_eq!(Record::decode(&image, 0), Ok(unstamped.clone()));
    assert_eq!(unstamped.commit_hex(), None);
}

/// Bytes of `record` with `edit` applied and the checksum recomputed, so the
/// decoder's field checks — not its checksum — are what answers.
fn resealed(record: &Record, edit: impl FnOnce(&mut [u8; 256])) -> [u8; 256] {
    let mut bytes = record.image();
    edit(&mut bytes);
    let mut sealed = bytes[..252].to_vec();
    sealed.extend_from_slice(&record.ordinal.to_le_bytes());
    bytes[252..].copy_from_slice(&store::crc::crc32c(&sealed).to_le_bytes());
    bytes
}

/// The decoder's answer for `record` with `edit` applied and resealed.
fn refused(record: &Record, field: &'static str, edit: impl FnOnce(&mut [u8; 256])) {
    assert_eq!(
        Record::decode(&resealed(record, edit), record.ordinal),
        Err(Fault::Invalid(field)),
        "{field}"
    );
}

/// **A sealed record with an impossible field is refused by the field.**
#[test]
fn every_impossible_record_field_is_named() {
    let record = sample(2);
    refused(&record, "vendor", |b| {
        b[0..8].copy_from_slice(b"nobody\0\0");
    });
    refused(&record, "source_kind", |b| b[8] = 0);
    refused(&record, "source_kind", |b| b[8] = 4);
    refused(&record, "flags", |b| b[9] |= 4);
    refused(&record, "name_len", |b| {
        b[10..12].copy_from_slice(&0u16.to_le_bytes());
    });
    refused(&record, "name_len", |b| {
        b[10..12].copy_from_slice(&129u16.to_le_bytes());
    });
    refused(&record, "name_len", |b| b[200] = b'x');
    refused(&record, "name_len", |b| b[9] |= 2);
    refused(&record, "source_name", |b| b[124] = 0xFF);
    refused(&record, "source_name_blake3", |b| b[40] ^= 1);
    refused(&record, "ist_day", |b| {
        b[28..32].copy_from_slice(&20_260_231u32.to_le_bytes());
    });
    refused(&record, "n_bars", |b| {
        b[72..80].copy_from_slice(&0u64.to_le_bytes());
    });
    refused(&record, "n_bars", |b| {
        b[64..72].copy_from_slice(&u64::MAX.to_le_bytes());
    });
    refused(&record, "landed_at", |b| {
        b[96..104].copy_from_slice(&0i64.to_le_bytes());
    });
    refused(&record, "landed_at", |b| {
        b[96..104].copy_from_slice(&(-1i64).to_le_bytes());
    });
    refused(&record, "commit", |b| {
        b[9] &= !1;
        b[110] = 1;
    });
}

/// **A name longer than the cap keeps its longest whole-character prefix**, is
/// flagged, and is identified by the digest of the whole name; a truncated
/// name cut shorter than a character boundary requires is impossible.
#[test]
fn a_long_name_is_kept_as_a_flagged_prefix_and_named_by_its_whole_digest() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    // 126 ASCII bytes, then a four-byte character straddling the cap.
    let long = format!("{}{}{}", "m".repeat(126), '\u{1F600}', "tail.csv");
    let a = file.append(&bars(3, 0, 2)).unwrap();
    let one = written(root.record(&file, path, a, &landing(&long, 3, 1)).unwrap());
    assert!(one.source_name_truncated);
    assert_eq!(one.source_name, "m".repeat(126));
    assert_eq!(
        one.source_name_blake3,
        brutex_core::blake3::hash(long.as_bytes())
    );
    assert_eq!(root.present(path).records(), std::slice::from_ref(&one));

    let exact = "n".repeat(NAME_CAP);
    let b = file.append(&bars(4, 0, 2)).unwrap();
    let two = written(root.record(&file, path, b, &landing(&exact, 4, 2)).unwrap());
    assert!(
        !two.source_name_truncated,
        "a name of exactly the cap is whole"
    );
    assert_eq!(two.source_name, exact);

    let over = "o".repeat(NAME_CAP + 1);
    let c = file.append(&bars(5, 0, 2)).unwrap();
    let three = written(root.record(&file, path, c, &landing(&over, 5, 3)).unwrap());
    assert!(three.source_name_truncated);
    assert_eq!(three.source_name.len(), NAME_CAP);
    assert_eq!(root.present(path).records().len(), 3);

    // A truncated name four bytes short of the cap cannot come from a cut.
    let short = Record {
        source_name: "p".repeat(NAME_CAP - 4),
        source_name_truncated: true,
        ..sample(0)
    };
    assert_eq!(
        Record::decode(&short.image(), 0),
        Err(Fault::Invalid("name_len"))
    );
    let shortest_cut = Record {
        source_name: "p".repeat(NAME_CAP - 3),
        source_name_truncated: true,
        ..sample(0)
    };
    assert_eq!(
        Record::decode(&shortest_cut.image(), 0),
        Ok(shortest_cut.clone())
    );
}

/// **A landing that cannot be written as a record is refused before any file
/// is opened.**
#[test]
fn every_landing_fault_is_refused_before_the_disk() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let a = file.append(&bars(3, 0, 2)).unwrap();
    let endpoint = |name: &'static str| Landing {
        source: Source {
            kind: SourceKind::VendorEndpoint,
            name,
            bytes: 10,
            crc32: 0,
        },
        ..landing("unused", 3, 1)
    };
    for (offered, fault) in [
        (landing("", 3, 1), LandingFault::EmptyName),
        (landing("a\nb.csv", 3, 1), LandingFault::ControlCharacter),
        (landing("a\u{7f}.csv", 3, 1), LandingFault::ControlCharacter),
        (
            endpoint("/instruments/historical/1/minute?from=2024-06-01"),
            LandingFault::EndpointQuery,
        ),
        (
            Landing {
                commit: Some("0123456789ABCDEF0123456789abcdef01234567"),
                ..landing("a.csv", 3, 1)
            },
            LandingFault::Commit,
        ),
        (
            Landing {
                commit: Some("0123456789abcdef0123456789abcdef0123456"),
                ..landing("a.csv", 3, 1)
            },
            LandingFault::Commit,
        ),
        (
            Landing {
                commit: Some("0123456789abcdef0123456789abcdef0123456g"),
                ..landing("a.csv", 3, 1)
            },
            LandingFault::Commit,
        ),
        (
            Landing {
                landed_at_micros: 0,
                ..landing("a.csv", 3, 1)
            },
            LandingFault::LandedAt,
        ),
    ] {
        match root.record(&file, path, a, &offered) {
            Err(ProvenanceError::Landing(found)) => assert_eq!(found, fault),
            other => panic!("expected {fault:?}, got {other:?}"),
        }
        assert!(!fault.to_string().is_empty());
    }
    assert!(
        !root.prv(path).exists(),
        "a refused landing created a sidecar"
    );

    // A query mark in an archive member's name is not an endpoint's query.
    let member = written(
        root.record(&file, path, a, &landing("odd?name.csv", 3, 1))
            .unwrap(),
    );
    assert_eq!(member.source_name, "odd?name.csv");
    // An endpoint path without a query, and an unstamped build, are recorded.
    let b = file.append(&bars(4, 0, 2)).unwrap();
    let unstamped = Landing {
        commit: None,
        bar_definition: 0,
        ..endpoint("/instruments/historical/256265/minute")
    };
    let two = written(root.record(&file, path, b, &unstamped).unwrap());
    assert_eq!(two.commit, None);
    assert_eq!(two.source_kind, SourceKind::VendorEndpoint);
    assert_eq!(two.bar_definition, 0);
    assert_eq!(root.present(path).records()[1], two);
}

/// **A symlink or a directory where the sidecar belongs is refused by name**,
/// for the reader and the writer, and the link's target is not written.
#[test]
fn a_symlink_or_a_directory_is_not_a_sidecar() {
    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let a = file.append(&bars(3, 0, 2)).unwrap();
    let prv = root.prv(path);
    let target = root.0.join("elsewhere.prv");
    fs::write(&target, b"").unwrap();
    std::os::unix::fs::symlink(&target, &prv).unwrap();
    assert!(matches!(
        root.read(path),
        Err(ProvenanceError::Symlinked { .. })
    ));
    assert!(matches!(
        root.record(&file, path, a, &landing("a.csv", 3, 1)),
        Err(ProvenanceError::Symlinked { .. })
    ));
    assert_eq!(fs::read(&target).unwrap(), b"");

    fs::remove_file(&prv).unwrap();
    fs::create_dir(&prv).unwrap();
    assert!(matches!(
        root.read(path),
        Err(ProvenanceError::NotAFile { .. })
    ));
    assert!(matches!(
        root.record(&file, path, a, &landing("a.csv", 3, 1)),
        Err(ProvenanceError::NotAFile { .. })
    ));
}

/// **One writer at a time**: a landing finds the sidecar locked and refuses.
#[test]
fn a_locked_sidecar_refuses_a_second_writer() {
    let root = Root::new();
    let path = june();
    let mut file = landed_month(&root, path, &[bars(3, 0, 2)]);
    let holder = fs::File::open(root.prv(path)).unwrap();
    holder.lock().unwrap();
    let b = file.append(&bars(4, 0, 2)).unwrap();
    assert!(matches!(
        root.record(&file, path, b, &landing("b.csv", 4, 2)),
        Err(ProvenanceError::Locked { .. })
    ));
    holder.unlock().unwrap();
    assert_eq!(
        written(
            root.record(&file, path, b, &landing("b.csv", 4, 2))
                .unwrap()
        )
        .ordinal,
        1
    );
    // A reader takes no lock.
    holder.lock().unwrap();
    assert_eq!(root.present(path).records().len(), 2);
}

/// **A read is bounded**: a file longer than the most a sidecar can be is
/// refused before it is read, and a full sidecar refuses another landing.
#[test]
fn the_sidecar_is_bounded_and_refuses_past_its_capacity() {
    let root = Root::new();
    let path = june();
    let mut file = landed_month(&root, path, &[bars(3, 0, 2)]);
    let prv = root.prv(path);
    let one = fs::read(&prv).unwrap();

    fs::OpenOptions::new()
        .write(true)
        .open(&prv)
        .unwrap()
        .set_len(MAX_FILE_LEN + 1)
        .unwrap();
    match root.read(path) {
        Err(ProvenanceError::TooLarge { bytes, .. }) => assert_eq!(bytes, MAX_FILE_LEN + 1),
        other => panic!("expected TooLarge, got {other:?}"),
    }

    // MAX_RECORDS records in bar order, each one bar at its own generation.
    let mut full = one[..ix(HEADER_LEN)].to_vec();
    for ordinal in 0..MAX_RECORDS {
        let record = Record {
            ordinal,
            vendor: Vendor::Gdfl,
            source_kind: SourceKind::ArchiveMember,
            first_index: ordinal,
            n_bars: 1,
            bar_file_generation: ordinal + 1,
            ..sample(ordinal)
        };
        full.extend_from_slice(&record.image());
    }
    assert_eq!(full.len() as u64, MAX_FILE_LEN);
    fs::write(&prv, &full).unwrap();
    assert_eq!(root.present(path).records().len() as u64, MAX_RECORDS);
    let b = file.append(&bars(4, 0, 2)).unwrap();
    assert!(matches!(
        root.record(&file, path, b, &landing("b.csv", 4, 2)),
        Err(ProvenanceError::Full { .. })
    ));
    assert_eq!(fs::read(&prv).unwrap(), full);
}

/// **Records out of bar order, or of another vendor, are refused when read.**
#[test]
fn a_sidecar_out_of_order_or_of_another_vendor_is_refused() {
    let root = Root::new();
    let path = june();
    let _file = landed_month(&root, path, &[bars(3, 0, 2)]);
    let prv = root.prv(path);
    let header = fs::read(&prv).unwrap()[..ix(HEADER_LEN)].to_vec();
    let at = |ordinal: u64, first_index: u64, generation: u64| Record {
        ordinal,
        vendor: Vendor::Gdfl,
        first_index,
        n_bars: 5,
        bar_file_generation: generation,
        ..sample(ordinal)
    };

    let mut overlap = header.clone();
    overlap.extend_from_slice(&at(0, 0, 1).image());
    overlap.extend_from_slice(&at(1, 4, 2).image());
    fs::write(&prv, &overlap).unwrap();
    assert!(matches!(
        root.read(path),
        Err(ProvenanceError::OutOfOrder {
            ordinal: 1,
            previous_end: 5,
            first_index: 4,
            ..
        })
    ));

    let mut stale = header.clone();
    stale.extend_from_slice(&at(0, 0, 3).image());
    stale.extend_from_slice(&at(1, 5, 3).image());
    fs::write(&prv, &stale).unwrap();
    assert!(matches!(
        root.read(path),
        Err(ProvenanceError::OutOfOrder {
            previous_generation: 3,
            generation: 3,
            ..
        })
    ));

    let mut foreign = header;
    foreign.extend_from_slice(&sample(0).image());
    fs::write(&prv, &foreign).unwrap();
    assert!(matches!(
        root.read(path),
        Err(ProvenanceError::Record {
            ordinal: 0,
            field: "vendor",
            ..
        })
    ));
}

/// **A sealed record with an impossible field refuses the sidecar.**
#[test]
fn a_sealed_impossible_record_refuses_the_sidecar() {
    let root = Root::new();
    let path = june();
    let _file = landed_month(&root, path, &[bars(3, 0, 2)]);
    let prv = root.prv(path);
    let mut bytes = fs::read(&prv).unwrap();
    let record = written(Recorded::Written(root.present(path).records()[0].clone()));
    let damaged = resealed(&record, |b| b[8] = 9);
    bytes[ix(HEADER_LEN)..].copy_from_slice(&damaged);
    fs::write(&prv, &bytes).unwrap();
    match root.read(path) {
        Err(error @ ProvenanceError::Record { .. }) => {
            assert!(error.to_string().contains("source_kind is impossible"));
        }
        other => panic!("expected Record, got {other:?}"),
    }
}

/// **The calendar a record keeps is a real one.**
#[test]
fn a_day_is_a_real_date_and_round_trips_as_yyyymmdd() {
    for (y, m, d, ok) in [
        (2024, 2, 29, true),
        (2023, 2, 29, false),
        (1900, 2, 29, false),
        (2000, 2, 29, true),
        (2024, 4, 31, false),
        (2024, 4, 30, true),
        (2024, 12, 31, true),
        (2024, 13, 1, false),
        (2024, 0, 1, false),
        (2024, 1, 0, false),
        (0, 1, 1, false),
        (9999, 12, 31, true),
        (10_000, 1, 1, false),
    ] {
        assert_eq!(Day::new(y, m, d).is_some(), ok, "{y}-{m}-{d}");
    }
    let day = Day::new(2019, 5, 30).unwrap();
    assert_eq!(day.yyyymmdd(), 20_190_530);
    assert_eq!(Day::from_yyyymmdd(20_190_530), Some(day));
    assert_eq!(Day::from_yyyymmdd(20_190_532), None);
    assert_eq!(Day::from_yyyymmdd(u32::MAX), None);
    assert_eq!((day.year(), day.month(), day.day()), (2019, 5, 30));
    assert_eq!(day.to_string(), "2019-05-30");
}

/// **The three source kinds keep their codes and names.**
#[test]
fn the_source_kinds_keep_their_codes_and_names() {
    let table: Vec<_> = SourceKind::ALL
        .iter()
        .map(|kind| (kind.code(), kind.as_str()))
        .collect();
    assert_eq!(
        table,
        [
            (1, "archive_member"),
            (2, "tickstore_day"),
            (3, "vendor_endpoint")
        ]
    );
    for kind in SourceKind::ALL {
        assert_eq!(SourceKind::from_code(kind.code()), Some(kind));
    }
    assert_eq!(SourceKind::from_code(0), None);
    assert_eq!(SourceKind::from_code(4), None);
}

/// Every refusal, with the words its sentence must carry.
#[allow(clippy::too_many_lines)]
fn refusals() -> Vec<(ProvenanceError, &'static str)> {
    let at = PathBuf::from("/s/bars/gdfl/NSE/INDEX/NIFTY/1min/2024-06.prv");
    vec![
        (
            ProvenanceError::NotABarPath {
                found: FileKind::Checksums,
            },
            "the .crc sibling",
        ),
        (
            ProvenanceError::WrongBarFile {
                expected: PathBuf::from("/a.bin"),
                handle: PathBuf::from("/b.bin"),
            },
            "names /a.bin and the handle holds /b.bin",
        ),
        (
            ProvenanceError::NotTheNewestCommit {
                n_valid: 10,
                records: 20,
            },
            "through bar 10 and the bar file has since committed through 20",
        ),
        (
            ProvenanceError::ImpossibleRange {
                first_index: 3,
                n_valid: 3,
            },
            "from bar 3 to 3 appended nothing",
        ),
        (
            ProvenanceError::Landing(LandingFault::EndpointQuery),
            "cannot be recorded: the endpoint name carries a query string",
        ),
        (
            ProvenanceError::Symlinked { path: at.clone() },
            "2024-06.prv is a symlink",
        ),
        (
            ProvenanceError::NotAFile { path: at.clone() },
            "is not a regular file",
        ),
        (
            ProvenanceError::Locked { path: at.clone() },
            "another writer holds it",
        ),
        (
            ProvenanceError::TooLarge {
                path: at.clone(),
                bytes: 9,
            },
            "is 9 bytes, past the 262208",
        ),
        (
            ProvenanceError::Header {
                path: at.clone(),
                fault: HeaderFault::Magic,
            },
            "2024-06.prv: its magic is not BRUTEXP1",
        ),
        (
            ProvenanceError::Damaged {
                path: at.clone(),
                ordinal: 4,
            },
            "record 4 fails its checksum and a later record passes",
        ),
        (
            ProvenanceError::Record {
                path: at.clone(),
                ordinal: 2,
                field: "flags",
            },
            "record 2 passes its checksum and its flags is impossible",
        ),
        (
            ProvenanceError::OutOfOrder {
                path: at.clone(),
                ordinal: 1,
                previous_end: 5,
                previous_generation: 2,
                first_index: 4,
                generation: 3,
            },
            "record 1 starts at bar 4 at generation 3, and the record before it ends at bar 5 at generation 2",
        ),
        (
            ProvenanceError::Full { path: at.clone() },
            "holds 1024 records",
        ),
        (
            ProvenanceError::PastTheBars {
                path: at.clone(),
                ordinal: 0,
                end: 10,
                n_valid: 6,
            },
            "covers bars up to 10 and the bar file has committed 6",
        ),
        (
            ProvenanceError::Io {
                path: at.clone(),
                action: "read",
                source: std::io::Error::other("gone"),
            },
            "2024-06.prv: read: gone",
        ),
    ]
}

/// **Every refusal names its file and its reason in words.**
#[test]
fn every_refusal_says_what_and_where() {
    for (error, words) in refusals() {
        let text = error.to_string();
        assert!(text.contains(words), "{text:?} must contain {words:?}");
        let has_source = std::error::Error::source(&error).is_some();
        assert_eq!(has_source, matches!(error, ProvenanceError::Io { .. }));
    }
    for fault in [
        LandingFault::EmptyName,
        LandingFault::ControlCharacter,
        LandingFault::Commit,
        LandingFault::LandedAt,
    ] {
        assert!(fault.to_string().starts_with("the "), "{fault:?}");
    }
}

/// **A `.prv` is a sibling of the month, never its records.** The bar door
/// refuses to open one as records, and its name ends with an extension in
/// `FileKind::ALL` — the test `pull::census_store`'s recount walk applies, so
/// a recount counts it as a sibling and not as a stray that would hold a day
/// open.
#[test]
fn a_provenance_sidecar_is_a_sibling_and_never_records() {
    let root = Root::new();
    let path = june();
    // A month with a sidecar beside it; the writer is closed before the
    // doors below are asked.
    drop(landed_month(&root, path, &[bars(3, 0, 2)]));
    match BarFile::open_or_create(&root.0, path.with_file(FileKind::Provenance), SYMBOL) {
        Err(store::file::StoreError::NotABarPath { found }) => {
            assert_eq!(found, FileKind::Provenance);
        }
        other => panic!("expected NotABarPath, got {other:?}"),
    }
    match BarFile::open_existing(&root.0, path.with_file(FileKind::Provenance), SYMBOL) {
        Err(store::file::StoreError::NotABarPath { found }) => {
            assert_eq!(found, FileKind::Provenance);
        }
        other => panic!("expected NotABarPath, got {other:?}"),
    }
    let name = root
        .prv(path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    assert_eq!(name, "2024-06.prv");
    assert!(
        FileKind::ALL
            .iter()
            .any(|kind| name.ends_with(kind.extension())),
        "the recount walk would count {name} as a stray"
    );
    assert_eq!(FileKind::Provenance.extension(), ".prv");
    assert_eq!(FileKind::Provenance.checksums(), None);
}

/// **The first landing makes its directory entry durable, or says it could
/// not.** A month directory that cannot be opened for its sync refuses the
/// landing by name; the header that did reach the file is a valid empty
/// sidecar, and the next landing appends record 0 to it.
#[test]
fn a_directory_that_cannot_be_synced_refuses_the_first_landing() {
    support::where_permission_binds(
        "a_directory_that_cannot_be_synced_refuses_the_first_landing",
        a_directory_that_cannot_be_synced_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
fn a_directory_that_cannot_be_synced_body() {
    use std::os::unix::fs::PermissionsExt;

    let root = Root::new();
    let path = june();
    let mut file = root.writer(path);
    let a = file.append(&bars(3, 0, 2)).unwrap();
    let dir = root.prv(path).parent().unwrap().to_path_buf();
    // Writable and searchable, so the sidecar can be created; not readable,
    // so the directory cannot be opened to be synced.
    fs::set_permissions(&dir, PermissionsExt::from_mode(0o300)).unwrap();
    let refusal = root.record(&file, path, a, &landing("a.csv", 3, 1));
    // Restored before the assertions, so a failure still cleans up.
    fs::set_permissions(&dir, PermissionsExt::from_mode(0o755)).unwrap();
    match refusal {
        Err(ProvenanceError::Io {
            path: at, action, ..
        }) => {
            assert_eq!(at, dir);
            assert_eq!(action, "sync directory");
        }
        other => panic!("expected the directory sync refused, got {other:?}"),
    }
    let sidecar = root.present(path);
    assert!(sidecar.records().is_empty());
    assert_eq!(sidecar.tail_bytes(), 0);
    assert_eq!(
        fs::metadata(root.prv(path)).unwrap().len(),
        HEADER_LEN,
        "the header alone reached the file"
    );
    let one = written(
        root.record(&file, path, a, &landing("a.csv", 3, 1))
            .unwrap(),
    );
    assert_eq!(one.ordinal, 0);
    assert_eq!(root.present(path).records(), std::slice::from_ref(&one));
}

/// **A sidecar the host will not open is a named refusal**, for the reader
/// and for the writer, never an absent one.
#[test]
fn a_sidecar_that_cannot_be_opened_is_refused_by_name() {
    support::where_permission_binds(
        "a_sidecar_that_cannot_be_opened_is_refused_by_name",
        a_sidecar_that_cannot_be_opened_body,
    );
}

/// The test above, run where the mode bits bind (D-0995).
fn a_sidecar_that_cannot_be_opened_body() {
    use std::os::unix::fs::PermissionsExt;

    let root = Root::new();
    let path = june();
    let mut file = landed_month(&root, path, &[bars(3, 0, 2)]);
    let prv = root.prv(path);
    let before = fs::read(&prv).unwrap();
    fs::set_permissions(&prv, PermissionsExt::from_mode(0o000)).unwrap();
    let read = root.read(path);
    let b = file.append(&bars(4, 0, 2)).unwrap();
    let write = root.record(&file, path, b, &landing("b.csv", 4, 2));
    fs::set_permissions(&prv, PermissionsExt::from_mode(0o644)).unwrap();
    for refusal in [read.map(|_| ()), write.map(|_| ())] {
        match refusal {
            Err(ProvenanceError::Io {
                path: at, action, ..
            }) => {
                assert_eq!(at, prv);
                assert_eq!(action, "open");
            }
            other => panic!("expected the open refused, got {other:?}"),
        }
    }
    assert_eq!(fs::read(&prv).unwrap(), before);
}
