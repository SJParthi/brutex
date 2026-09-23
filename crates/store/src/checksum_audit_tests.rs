#![cfg(test)]
#![allow(clippy::expect_used, clippy::indexing_slicing)]
use super::*;
use crate::path::{FileKind, StorePath, Timeframe, YearMonth};
use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    key: InstrumentKey,
}
impl Fixture {
    fn new(count: usize) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-checksum-audit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("unique scratch");
        let result = Self {
            root,
            key: InstrumentKey::index(Exchange::Nse, "NIFTY").expect("test instrument"),
        };
        let mut file =
            BarFile::open_or_create(&result.root, result.path(), 7).expect("fixture month");
        let rows: Vec<_> = (0..count).map(bar).collect();
        if !rows.is_empty() {
            file.append(&rows).expect("fixture committed rows");
        }
        result
    }
    fn path(&self) -> StorePath<'_> {
        StorePath::for_key(
            Vendor::Zerodha,
            &self.key,
            Timeframe::MINUTE_1,
            YearMonth::new(2025, 5).expect("month"),
            FileKind::Bars,
        )
        .expect("store path")
    }
    fn named(&self, kind: FileKind) -> PathBuf {
        self.path().with_file(kind).to_path_buf(&self.root)
    }
    fn open(&self) -> Result<BarFile, crate::file::StoreError> {
        BarFile::open_existing(&self.root, self.path(), 7)
    }
    fn audited(&self) -> Result<AuditedBarFile, String> {
        self.open().map_err(error)?.audit_checksums(1_048_576)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn bar(index: usize) -> Bar {
    Bar {
        ts_micros: i64::try_from(index + 1).expect("small fixture") * 60_000_000,
        open: 100,
        high: 120,
        low: 90,
        close: 110,
        volume: 50,
        open_interest: crate::format::OI_NULL,
    }
}

#[test]
fn strict_open_keeps_the_original_lock_and_refuses_busy_or_nonregular_sources() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new(74);
    let lock = File::open(fixture.named(FileKind::Lock)).expect("month lock");
    lock.try_lock().expect("exclusive writer");
    assert!(BarFile::open_existing_audited(&fixture.root, fixture.path(), 7, 1_048_576).is_err());
    lock.unlock().expect("writer releases");
    let audited = BarFile::open_existing_audited(&fixture.root, fixture.path(), 7, 1_048_576)
        .expect("strict source");
    assert!(
        lock.try_lock().is_err(),
        "audited reader retains its original shared lock"
    );
    assert_eq!(audited.read_record(73).expect("real row"), bar(73));
    drop(audited);
    lock.try_lock().expect("reader release permits writer");
    lock.unlock().expect("unlock");
    for kind in [FileKind::Bars, FileKind::Checksums, FileKind::Lock] {
        let fixture = Fixture::new(1);
        let path = fixture.named(kind);
        fs::remove_file(&path).expect("scratch removal");
        fs::create_dir(&path).expect("directory masquerading as source");
        assert!(
            BarFile::open_existing_audited(&fixture.root, fixture.path(), 7, 1_048_576).is_err()
        );
    }
}

#[test]
fn strict_source_fifo_probe_cannot_wait_for_a_writer() -> Result<(), Box<dyn std::error::Error>> {
    const PROBE: &str = "BRUTEX_CHECKSUM_SOURCE_FIFO_PROBE";
    let _sink_is_mine = crate::emits::hold_the_sink();
    if let Some(root) = std::env::var_os(PROBE) {
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("key");
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            Timeframe::MINUTE_1,
            YearMonth::new(2025, 5).expect("month"),
            FileKind::Bars,
        )
        .expect("path");
        assert!(BarFile::open_existing_audited(Path::new(&root), path, 7, 1_048_576).is_err());
        return Ok(());
    }
    for kind in [FileKind::Bars, FileKind::Checksums, FileKind::Lock] {
        let fixture = Fixture::new(1);
        let path = fixture.named(kind);
        fs::remove_file(&path).expect("scratch remove");
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&path)
                .status()
                .expect("make FIFO")
                .success()
        );
        let mut child =
            std::process::Command::new(std::env::current_exe().expect("test executable"))
                .args([
                    "--exact",
                    "checksum_audit::tests::strict_source_fifo_probe_cannot_wait_for_a_writer",
                ])
                .env(PROBE, &fixture.root)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .expect("bounded child");
        let started = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait().expect("child status") {
                assert!(status.success());
                break;
            }
            if started.elapsed() > std::time::Duration::from_secs(2) {
                child.kill().expect("bounded timeout");
                child.wait().expect("reap child");
                return Err("strict checksum source open waited for a FIFO peer".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    Ok(())
}

#[test]
fn full_audit_matches_exact_file_images_at_all_partial_block_boundaries() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for count in [0, 1, 72, 73, 74, 146, 147] {
        let fixture = Fixture::new(count);
        let audited = fixture.audited().expect("full bounded audit");
        let evidence = audited.evidence();
        let data = fs::read(fixture.named(FileKind::Bars)).expect("raw data");
        let sidecar = fs::read(fixture.named(FileKind::Checksums)).expect("raw sidecar");
        assert_eq!(evidence.header().n_valid, count as u64);
        assert_eq!(evidence.blocks(), (count as u64).div_ceil(73));
        assert_eq!(
            evidence.header_digest(),
            brutex_core::blake3::hash(&data[..HEADER_BYTES])
        );
        assert_eq!(
            evidence.data_digest(),
            brutex_core::blake3::hash(&data[HEADER_BYTES..])
        );
        assert_eq!(evidence.file_digest(), brutex_core::blake3::hash(&data));
        assert_eq!(
            evidence.sidecar_digest(),
            brutex_core::blake3::hash(&sidecar)
        );
        assert_eq!(&evidence.canonical_bytes()[..8], b"BRCAS001");
        assert!(evidence.canonical_bytes()[240..].iter().all(|b| *b == 0));
        for index in 0..count {
            assert_eq!(
                audited
                    .read_record(index as u64)
                    .expect("same verified row"),
                bar(index)
            );
        }
        assert!(audited.read_record(count as u64).is_err());
        assert_eq!(
            fixture.audited().expect("fresh repeat audit").evidence(),
            evidence
        );
    }
}

#[test]
fn strict_extent_and_presence_refusals_never_change_source_bytes() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new(74);
    let data = fs::read(fixture.named(FileKind::Bars)).expect("data");
    let crc = fs::read(fixture.named(FileKind::Checksums)).expect("crc");
    let exact = (data.len() + crc.len()) as u64;
    for limit in [0, 1, exact - 1] {
        assert!(
            fixture
                .open()
                .expect("open")
                .audit_checksums(limit)
                .is_err()
        );
    }
    fixture
        .open()
        .expect("open")
        .audit_checksums(exact)
        .expect("exact bound");
    for kind in [FileKind::Checksums, FileKind::Lock] {
        let path = fixture.named(kind);
        let saved = fs::read(&path).expect("existing");
        fs::remove_file(&path).expect("inject missing");
        assert!(fixture.audited().is_err());
        fs::write(&path, saved).expect("restore");
    }
    for len in [0, crc.len() - 1, crc.len() + 4] {
        let mut changed = crc.clone();
        changed.resize(len, 0);
        fs::write(fixture.named(FileKind::Checksums), &changed).expect("fault crc extent");
        assert!(fixture.audited().is_err());
        assert_eq!(
            fs::read(fixture.named(FileKind::Checksums)).expect("not repaired"),
            changed
        );
    }
    fs::write(fixture.named(FileKind::Checksums), &crc).expect("restore crc");
    for len in [data.len() - 1, data.len() + 1, data.len() + 56] {
        let mut changed = data.clone();
        changed.resize(len, 0);
        fs::write(fixture.named(FileKind::Bars), &changed).expect("fault data extent");
        assert!(fixture.audited().is_err());
        assert_eq!(
            fs::read(fixture.named(FileKind::Bars)).expect("not repaired"),
            changed
        );
    }
}

#[test]
fn every_stored_crc_and_record_byte_corruption_is_refused_in_a_finite_small_month() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    let fixture = Fixture::new(1);
    let data = fs::read(fixture.named(FileKind::Bars)).expect("data");
    let crc = fs::read(fixture.named(FileKind::Checksums)).expect("crc");
    for offset in HEADER_BYTES..data.len() {
        let mut changed = data.clone();
        changed[offset] ^= 1;
        fs::write(fixture.named(FileKind::Bars), changed).expect("fault data");
        assert!(fixture.audited().is_err(), "data byte {offset}");
    }
    fs::write(fixture.named(FileKind::Bars), data).expect("restore data");
    for offset in 0..crc.len() {
        let mut changed = crc.clone();
        changed[offset] ^= 1;
        fs::write(fixture.named(FileKind::Checksums), changed).expect("fault crc");
        assert!(fixture.audited().is_err(), "crc byte {offset}");
    }
}

#[test]
fn warm_authority_refuses_mutation_replacement_and_deletion_of_every_held_file() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for kind in [FileKind::Bars, FileKind::Checksums, FileKind::Lock] {
        for change in 0..3 {
            let fixture = Fixture::new(74);
            let audited = fixture.audited().expect("audit before fault");
            assert_eq!(audited.read_record(0).expect("warm block"), bar(0));
            let path = fixture.named(kind);
            let mut original = fs::read(&path).expect("read fault target");
            match change {
                0 => {
                    original.push(1);
                    fs::write(&path, original).expect("mutate");
                }
                1 => {
                    fs::remove_file(&path).expect("unlink");
                    fs::write(&path, original).expect("same-byte replacement");
                }
                _ => {
                    fs::remove_file(&path).expect("delete");
                }
            }
            assert!(audited.require_current().is_err());
            assert!(audited.read_record(0).is_err());
            assert!(audited.read_record(73).is_err());
        }
    }
}

#[test]
fn aliases_and_unsealed_legacy_data_never_gain_a_strict_receipt() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for kind in [FileKind::Bars, FileKind::Checksums, FileKind::Lock] {
        let fixture = Fixture::new(1);
        let path = fixture.named(kind);
        let other = fixture.root.join("alias");
        fs::hard_link(&path, &other).expect("hardlink fault");
        assert!(fixture.audited().is_err());
        fs::remove_file(&other).expect("remove alias");
        fs::rename(&path, &other).expect("move target");
        std::os::unix::fs::symlink(&other, &path).expect("symlink fault");
        assert!(fixture.audited().is_err());
    }
    let fixture = Fixture::new(1);
    let file = fixture.open().expect("ordinary open");
    let mut header = file.header();
    drop(file);
    header.flags = 0;
    let commit = header.commit().expect("legacy header image");
    let file = fs::OpenOptions::new()
        .write(true)
        .open(fixture.named(FileKind::Bars))
        .expect("fault header");
    file.write_all_at(&commit.bytes, commit.offset)
        .expect("unsealed header");
    assert!(fixture.audited().is_err());
}

#[test]
fn source_changes_after_the_initial_snapshot_cannot_mint_cold_audit_authority() {
    let _sink_is_mine = crate::emits::hold_the_sink();
    for change_header in [false, true] {
        let fixture = Fixture::new(74);
        let file = fixture.open().expect("original held files");
        let input = file.checksum_inputs().expect("same exact handles");
        let before = Generations::read(&input).expect("actual initial generation snapshot");
        let path = fixture.named(FileKind::Bars);
        if change_header {
            let mut header = file.header();
            header.flags = 0;
            let commit = header.commit().expect("valid changed header image");
            let writer = fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .expect("noncooperating fault writer");
            writer
                .write_all_at(&commit.bytes, commit.offset)
                .expect("change header after snapshot");
        } else {
            let mut bytes = fs::read(&path).expect("original file image");
            bytes[HEADER_BYTES - 1] ^= 1;
            fs::write(&path, bytes).expect("change padding after snapshot");
        }
        let changed = fs::read(&path).expect("exact injected file image");
        let refusal = audit(&input, file.header(), file.layout(), before, 1_048_576)
            .expect_err("no authority for bytes changed after the initial snapshot");
        assert!(
            refusal.contains(if change_header {
                "held month header changed before checksum audit"
            } else {
                "checksum-audit input changed during the complete scan"
            }),
            "{refusal}"
        );
        assert_eq!(
            fs::read(path).expect("audit never repairs source bytes"),
            changed
        );
    }
}
