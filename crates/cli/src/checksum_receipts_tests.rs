#![cfg(test)]
#![allow(clippy::expect_used, clippy::indexing_slicing)]
use super::*;
use brutex_core::instrument::Exchange;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    key: InstrumentKey,
}
impl Fixture {
    fn new(count: u64) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-checksum-receipts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("unique scratch");
        let fixture = Self {
            root,
            key: InstrumentKey::index(Exchange::Nse, "NIFTY").expect("key"),
        };
        let mut writer =
            BarFile::open_or_create(&fixture.root, fixture.path(), fixture.symbol_id())
                .expect("writer");
        if count > 0 {
            writer
                .append(&(0..count).map(bar).collect::<Vec<_>>())
                .expect("rows");
        }
        fixture
    }
    fn symbol_id(&self) -> u32 {
        let hash = brutex_core::universe::fnv1a(self.key.underlying.as_str()).to_le_bytes();
        u32::from_le_bytes(hash[..4].try_into().expect("low32"))
    }
    fn path(&self) -> StorePath<'_> {
        StorePath::for_key(
            Vendor::Zerodha,
            &self.key,
            Timeframe::MINUTE_1,
            YearMonth::new(2025, 5).expect("month"),
            FileKind::Bars,
        )
        .expect("path")
    }
    fn request(&self) -> MonthRequest<'_> {
        MonthRequest {
            store_root: &self.root,
            receipt_root: &self.root,
            vendor: Vendor::Zerodha,
            key: &self.key,
            timeframe: Timeframe::MINUTE_1,
            month: YearMonth::new(2025, 5).expect("month"),
            max_bytes: 1_048_576,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn bar(index: u64) -> Bar {
    Bar {
        ts_micros: i64::try_from(index + 1).expect("small") * 60_000_000,
        open: 100,
        high: 120,
        low: 90,
        close: 110,
        volume: 50,
        open_interest: store::format::OI_NULL,
    }
}

#[test]
fn exact_receipts_reopen_with_same_identity_and_keep_every_real_row() {
    for count in [0, 1, 73, 74] {
        let fixture = Fixture::new(count);
        let first = audit_month(fixture.request()).expect("audit");
        let id = first.receipt_identity();
        let bytes = fs::read(first.receipt_path()).expect("durable receipt");
        assert_eq!(bytes.len(), BYTES);
        assert_eq!(&bytes[..8], b"BRHCRC01");
        assert_eq!(&bytes[80..88], b"BRCAS001");
        assert_eq!(&bytes[48..80], &id);
        assert!(bytes[336..SEAL_START].iter().all(|byte| *byte == 0));
        let second = audit_month(fixture.request()).expect("same receipt while source held");
        assert_eq!(second.receipt_identity(), id);
        assert_eq!(fs::read(first.receipt_path()).expect("same file"), bytes);
        let admitted = admit_month(fixture.request(), id).expect("explicit admission");
        for index in 0..count {
            assert_eq!(admitted.read_record(index).expect("exact row"), bar(index));
        }
        assert!(admitted.read_record(count).is_err());
        assert!(admit_month(fixture.request(), [0; 32]).is_err());
        let latest = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
            .expect("status")
            .expect("attempt");
        assert_eq!(
            latest.operation,
            crate::sweep_evidence::Operation::ChecksumAudit
        );
        assert_eq!(
            latest.completion,
            crate::sweep_evidence::Completion::Refused
        );
    }
}

#[test]
fn every_torn_prefix_recovers_only_by_appending_the_exact_suffix() {
    let fixture = Fixture::new(1);
    let admitted = audit_month(fixture.request()).expect("audit");
    let expected: [u8; BYTES] = fs::read(admitted.receipt_path())
        .expect("bytes")
        .try_into()
        .expect("fixed");
    let path = fixture.root.join("prefix.bin");
    for length in 0..=BYTES {
        fs::write(&path, &expected[..length]).expect("scratch torn prefix");
        publish(&path, &expected).expect("exact append recovery");
        assert_eq!(fs::read(&path).expect("recovered"), expected);
        Receipt::open(&path, &expected).expect("readback");
    }
    for offset in 0..BYTES {
        let mut wrong = expected;
        wrong[offset] ^= 1;
        fs::write(&path, wrong).expect("scratch corruption");
        assert!(publish(&path, &expected).is_err());
        assert!(Receipt::open(&path, &expected).is_err());
        assert_eq!(fs::read(&path).expect("not overwritten"), wrong);
    }
    let mut extended = expected.to_vec();
    extended.push(0);
    fs::write(&path, &extended).expect("extended");
    assert!(publish(&path, &expected).is_err());
    assert_eq!(fs::read(&path).expect("not truncated"), extended);
}

#[test]
fn strict_admission_refuses_missing_receipts_and_changed_source_without_repair() {
    let fixture = Fixture::new(74);
    let admitted = audit_month(fixture.request()).expect("audit");
    let id = admitted.receipt_identity();
    let path = admitted.receipt_path().to_path_buf();
    let receipt = fs::read(&path).expect("receipt");
    drop(admitted);
    fs::remove_file(&path).expect("remove scratch receipt");
    assert!(admit_month(fixture.request(), id).is_err());
    assert!(!path.exists());
    fs::write(&path, &receipt).expect("restore scratch");
    let mut writer = BarFile::open_or_create(&fixture.root, fixture.path(), fixture.symbol_id())
        .expect("writer");
    writer.append(&[bar(74)]).expect("append actual source");
    drop(writer);
    assert!(admit_month(fixture.request(), id).is_err());
    assert_eq!(fs::read(&path).expect("old receipt retained"), receipt);
    let next = audit_month(fixture.request()).expect("new exact generation audit");
    assert_ne!(next.receipt_identity(), id);
    assert_eq!(next.evidence().header().n_valid, 75);
}

#[test]
fn warm_receipt_checks_refuse_every_fault_and_never_release_a_row() {
    for fault in 0..6 {
        let fixture = Fixture::new(74);
        let admitted = audit_month(fixture.request()).expect("audit");
        assert_eq!(admitted.read_record(73).expect("warm row"), bar(73));
        let path = admitted.receipt_path();
        let mut bytes = fs::read(path).expect("bytes");
        match fault {
            0 => {
                bytes[100] ^= 1;
                fs::write(path, bytes).expect("corrupt");
            }
            1 => {
                fs::write(path, &bytes[..511]).expect("shrink");
            }
            2 => {
                bytes.push(0);
                fs::write(path, bytes).expect("grow");
            }
            3 => {
                fs::remove_file(path).expect("delete");
            }
            4 => {
                fs::remove_file(path).expect("replace");
                fs::write(path, bytes).expect("same bytes new inode");
            }
            _ => {
                fs::hard_link(path, fixture.root.join("alias.bin")).expect("alias");
            }
        }
        assert!(admitted.require_current().is_err());
        assert!(admitted.read_record(0).is_err());
        assert!(admitted.read_record(73).is_err());
    }
}

#[test]
fn failure_is_durable_and_receipts_cannot_be_borrowed_across_sources_or_aliases() {
    let fixture = Fixture::new(1);
    let mut request = fixture.request();
    request.max_bytes = 1;
    assert!(audit_month(request).is_err());
    let latest = crate::sweep_evidence::latest(&fixture.root, 1_048_576)
        .expect("status")
        .expect("attempt");
    assert_eq!(
        latest.operation,
        crate::sweep_evidence::Operation::ChecksumAudit
    );
    assert_eq!(
        latest.completion,
        crate::sweep_evidence::Completion::Refused
    );
    let admitted = audit_month(fixture.request()).expect("audit");
    let bytes = fs::read(admitted.receipt_path()).expect("receipt");
    let alias = fixture.root.join("receipt-alias.bin");
    std::os::unix::fs::symlink(admitted.receipt_path(), &alias).expect("symlink");
    assert!(Receipt::open(&alias, &bytes.try_into().expect("fixed")).is_err());
    let mut other = fixture.request();
    other.vendor = Vendor::Groww;
    assert!(admit_month(other, admitted.receipt_identity()).is_err());
    assert!(command(&[]).is_err());
    assert!(command(&["zerodha", "NIFTY", "1min", "no-year", "5", "/absent", "1"]).is_err());
}

#[test]
fn visible_complete_bytes_are_not_durable_authority_while_a_publisher_holds_the_lock() {
    let fixture = Fixture::new(1);
    let admitted = audit_month(fixture.request()).expect("audit");
    let path = admitted.receipt_path();
    let expected = fs::read(path)
        .expect("exact bytes")
        .try_into()
        .expect("fixed");
    let writer = open(path, true).expect("writer handle");
    assert!(
        writer.try_lock().is_err(),
        "typed receipt retains its shared lock"
    );
    let path = path.to_path_buf();
    drop(admitted);
    writer
        .try_lock()
        .expect("publication lock after reader release");
    assert!(Receipt::open(&path, &expected).is_err());
    writer.unlock().expect("publication durable and released");
    Receipt::open(&path, &expected).expect("published bytes admitted");
    let admitted = audit_month(fixture.request()).expect("exact publication reuse");
    assert_eq!(
        admitted.read_record(0).expect("row after publication"),
        bar(0)
    );
}

#[test]
fn dropping_receipt_releases_lock_even_when_a_duplicate_descriptor_survives() {
    let fixture = Fixture::new(1);
    let admitted = audit_month(fixture.request()).expect("audit");
    // Model a descriptor retained across a concurrent process spawn. The
    // duplicate owns no typed receipt authority and must not extend its lease.
    let duplicate = admitted.receipt.file.try_clone().expect("duplicate handle");
    let independent = Receipt::open(admitted.receipt_path(), &admitted.receipt.expected)
        .expect("independent reader");
    let writer = open(admitted.receipt_path(), true).expect("publication handle");
    assert!(writer.try_lock().is_err(), "live receipt excludes writers");
    drop(admitted);
    assert!(
        writer.try_lock().is_err(),
        "independent reader keeps its lease"
    );
    drop(independent);
    writer.try_lock().expect("receipt drop releases its lease");
    assert!(
        duplicate
            .metadata()
            .expect("duplicate remains open")
            .is_file()
    );
    writer.unlock().expect("release publication lease");
}

#[test]
fn receipt_fifo_cannot_block_either_read_or_publication() -> Result<(), Box<dyn std::error::Error>>
{
    const PROBE: &str = "BRUTEX_CHECKSUM_RECEIPT_FIFO_PROBE";
    if let Some(path) = std::env::var_os(PROBE) {
        assert!(open(Path::new(&path), false).is_err());
        assert!(open(Path::new(&path), true).is_err());
        return Ok(());
    }
    let fixture = Fixture::new(1);
    let path = fixture.root.join("receipt-fifo");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&path)
            .status()?
            .success()
    );
    let mut child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "checksum_receipts::tests::receipt_fifo_cannot_block_either_read_or_publication",
        ])
        .env(PROBE, &path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let started = std::time::Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            assert!(status.success());
            break;
        }
        if started.elapsed() > std::time::Duration::from_secs(2) {
            child.kill()?;
            child.wait()?;
            return Err("receipt FIFO open waited for a peer".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(())
}

#[test]
fn namespace_aliases_and_missing_strict_namespace_refuse_before_admission() {
    let fixture = Fixture::new(1);
    assert!(admit_month(fixture.request(), [1; 32]).is_err());
    assert!(!fixture.root.join("checksum-receipts-v1").exists());
    assert!(
        !fixture
            .root
            .join("results")
            .join("sweep-evidence-v1")
            .exists()
    );
    for alias in [false, true] {
        let fixture = Fixture::new(1);
        let namespace = fixture.root.join("checksum-receipts-v1");
        let target = fixture.root.join("unrelated-directory");
        fs::create_dir(&target).expect("scratch target");
        if alias {
            std::os::unix::fs::symlink(&target, &namespace).expect("namespace alias");
        } else {
            fs::write(&namespace, b"not a directory").expect("namespace obstruction");
        }
        assert!(audit_month(fixture.request()).is_err());
        assert_eq!(fs::read_dir(&target).expect("unmodified target").count(), 0);
        assert!(
            !fixture
                .root
                .join("results")
                .join("sweep-evidence-v1")
                .exists()
        );
    }
    let alias_root = fixture.root.join("receipt-root-alias");
    std::os::unix::fs::symlink(&fixture.root, &alias_root).expect("root alias");
    let mut request = fixture.request();
    request.receipt_root = &alias_root;
    assert!(audit_month(request).is_err());
    assert!(!fixture.root.join("checksum-receipts-v1").exists());
}

#[test]
fn changed_header_padding_rekeys_receipt_even_when_every_decoded_row_is_identical() {
    let fixture = Fixture::new(74);
    let admitted = audit_month(fixture.request()).expect("original bytes audited");
    let original = admitted.receipt_identity();
    let evidence = admitted.evidence();
    drop(admitted);
    let path = fixture.path().to_path_buf(&fixture.root);
    let mut data = fs::read(&path).expect("header image");
    data[32767] ^= 1;
    fs::write(path, data).expect("unused raw header padding changes");
    assert!(admit_month(fixture.request(), original).is_err());
    let changed = audit_month(fixture.request()).expect("new exact header image");
    assert_ne!(changed.receipt_identity(), original);
    assert_eq!(changed.evidence().header(), evidence.header());
    assert_eq!(changed.evidence().data_digest(), evidence.data_digest());
    assert_ne!(changed.evidence().header_digest(), evidence.header_digest());
    for index in 0..74 {
        assert_eq!(
            changed.read_record(index).expect("unchanged real row"),
            bar(index)
        );
    }
}

#[test]
fn incomplete_locked_publication_refuses_without_accepting_or_repairing_a_prefix() {
    let fixture = Fixture::new(1);
    let admitted = audit_month(fixture.request()).expect("original receipt");
    let expected: [u8; BYTES] = fs::read(admitted.receipt_path())
        .expect("exact bytes")
        .try_into()
        .expect("stride");
    let path = fixture.root.join("locked-prefix.bin");
    fs::write(&path, &expected[..127]).expect("interrupted prefix");
    let writer = open(&path, true).expect("partial publisher");
    writer.try_lock().expect("exclusive publication lease");
    assert!(publish(&path, &expected).is_err());
    assert!(Receipt::open(&path, &expected).is_err());
    assert_eq!(fs::read(&path).expect("prefix preserved"), expected[..127]);
    writer.unlock().expect("publisher finished or abandoned");
    publish(&path, &expected).expect("exact suffix recovery after release");
    assert_eq!(
        fs::read(path).expect("complete immutable receipt"),
        expected
    );
}
