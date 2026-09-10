#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::*;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-invocation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("private fixture");
        Self(root)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn exact_status_survives_reopen_and_never_equates_a_start_with_liveness() {
    let root = Scratch::new();
    let mut attempt = begin(&root.0, Origin::Browser, "range-all").unwrap();
    assert_eq!(attempt.id(), ID_BASE + 1);
    let initial = read(&root.0, ID_BASE + 1).unwrap().unwrap();
    assert_eq!(initial.phase.label(), "unconfirmed");
    assert_eq!(initial.completed_boundaries, 0);
    attempt.progress().unwrap();
    attempt.progress().unwrap();
    attempt.finish(Phase::Completed, 0).unwrap();
    drop(attempt);
    let final_record = read(&root.0, ID_BASE + 1).unwrap().unwrap();
    assert_eq!(final_record.phase, Phase::Completed);
    assert_eq!(final_record.completed_boundaries, 2);
    assert_eq!(final_record.origin, Origin::Browser);
    assert_eq!(final_record.label, "range-all");
    assert_eq!(
        fs::metadata(own(&base(&root.0), ID_BASE + 1))
            .unwrap()
            .len(),
        STRIDE * 4
    );
    assert_eq!(read(&root.0, ID_BASE + 2).unwrap(), None);
}

#[test]
fn a_second_terminal_or_later_progress_never_appends() {
    let root = Scratch::new();
    let mut attempt = begin(&root.0, Origin::Cli, "audit-range").unwrap();
    assert!(attempt.finish(Phase::Progress, 0).is_err());
    attempt.finish(Phase::Refused, 0).unwrap();
    assert!(attempt.finish(Phase::Completed, 0).is_err());
    assert!(attempt.progress().is_err());
    assert_eq!(
        fs::metadata(own(&base(&root.0), ID_BASE + 1))
            .unwrap()
            .len(),
        STRIDE * 2
    );
    assert_eq!(
        read(&root.0, ID_BASE + 1).unwrap().unwrap().phase,
        Phase::Refused
    );
}

#[test]
fn dropped_and_panicking_owners_record_distinct_terminal_facts() {
    let root = Scratch::new();
    drop(begin(&root.0, Origin::Browser, "sweep").unwrap());
    let panicked = std::panic::catch_unwind(|| {
        let attempt = begin(&root.0, Origin::Cli, "audit-range").unwrap();
        attempt.enter(|| {
            assert_eq!(current_id(), Some(ID_BASE + 2));
            panic!("private fault injection");
        });
    });
    assert!(panicked.is_err());
    assert_eq!(current_id(), None);
    assert_eq!(
        read(&root.0, ID_BASE + 1).unwrap().unwrap().phase,
        Phase::Cancelled
    );
    assert_eq!(
        read(&root.0, ID_BASE + 2).unwrap().unwrap().phase,
        Phase::Failed
    );
}

#[test]
fn thread_context_is_scoped_and_counts_only_explicit_boundaries() {
    let root = Scratch::new();
    let mut first = begin(&root.0, Origin::Cli, "range-all").unwrap();
    let mut second = begin(&root.0, Origin::Browser, "descent").unwrap();
    completed_boundary();
    first.enter(|| {
        assert_eq!(current_id(), Some(first.id()));
        completed_boundary();
        second.enter(|| {
            assert_eq!(current_id(), Some(second.id()));
            completed_boundary();
        });
        assert_eq!(current_id(), Some(first.id()));
    });
    assert_eq!(current_id(), None);
    first.finish(Phase::Completed, 0).unwrap();
    second.finish(Phase::Completed, 0).unwrap();
    assert_eq!(
        read(&root.0, ID_BASE + 1)
            .unwrap()
            .unwrap()
            .completed_boundaries,
        1
    );
    assert_eq!(
        read(&root.0, ID_BASE + 2)
            .unwrap()
            .unwrap()
            .completed_boundaries,
        1
    );
}

#[test]
fn busy_index_refuses_without_reserving_or_dispatching_an_invocation() {
    let root = Scratch::new();
    drop(begin(&root.0, Origin::Cli, "range-all").unwrap());
    let index = File::open(base(&root.0).join("index.bin")).unwrap();
    index.try_lock().unwrap();
    match begin(&root.0, Origin::Cli, "range-all") {
        Err(why) => assert!(is_busy(&why)),
        Ok(_) => panic!("a held index must refuse before admission"),
    }
    assert_eq!(index.metadata().unwrap().len(), STRIDE);
    index.unlock().unwrap();
    assert_eq!(
        begin(&root.0, Origin::Cli, "range-all").unwrap().id(),
        ID_BASE + 2
    );
}

#[test]
fn busy_or_changed_detail_poisons_the_writer_without_a_success_fallback() {
    let root = Scratch::new();
    let mut attempt = begin(&root.0, Origin::Cli, "range-all").unwrap();
    let file = File::open(own(&base(&root.0), ID_BASE + 1)).unwrap();
    file.try_lock_shared().unwrap();
    assert!(attempt.progress().is_err());
    file.unlock().unwrap();
    assert!(attempt.finish(Phase::Completed, 0).is_err());
    assert_eq!(
        read(&root.0, ID_BASE + 1).unwrap().unwrap().phase,
        Phase::Started
    );
}

#[test]
fn exact_progress_read_does_not_acquire_the_active_invocation_writer_lock() {
    let root = Scratch::new();
    let mut attempt = begin(&root.0, Origin::Cli, "range-all").unwrap();
    let file = File::open(own(&base(&root.0), attempt.id())).unwrap();
    file.try_lock().unwrap();
    assert_eq!(
        read(&root.0, attempt.id()).unwrap().unwrap().phase,
        Phase::Started
    );
    file.unlock().unwrap();
    attempt.progress().unwrap();
    attempt.finish(Phase::Completed, 0).unwrap();
    assert_eq!(
        read(&root.0, attempt.id()).unwrap().unwrap().phase,
        Phase::Completed
    );
    assert!(!is_busy(&error("corrupt data")));
}

#[test]
fn torn_index_and_torn_invocation_are_preserved_and_refused() {
    let root = Scratch::new();
    let mut attempt = begin(&root.0, Origin::Cli, "range-all").unwrap();
    let path = own(&base(&root.0), ID_BASE + 1);
    OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(&[9])
        .unwrap();
    assert!(attempt.finish(Phase::Completed, 0).is_err());
    assert!(read(&root.0, ID_BASE + 1).is_err());
    assert_eq!(fs::metadata(path).unwrap().len(), STRIDE + 1);
    OpenOptions::new()
        .append(true)
        .open(base(&root.0).join("index.bin"))
        .unwrap()
        .write_all(&[7])
        .unwrap();
    assert!(begin(&root.0, Origin::Cli, "range-all").is_err());
    assert_eq!(
        fs::metadata(base(&root.0).join("index.bin")).unwrap().len(),
        STRIDE + 1
    );
}

#[test]
fn incomplete_creation_retains_a_durable_unconfirmed_start() {
    let root = Scratch::new();
    let mut attempt = begin(&root.0, Origin::Browser, "sweep").unwrap();
    attempt.armed = false;
    fs::remove_file(own(&base(&root.0), ID_BASE + 1)).unwrap();
    assert_eq!(
        read(&root.0, ID_BASE + 1).unwrap().unwrap().phase,
        Phase::Started
    );
    assert_eq!(
        begin(&root.0, Origin::Browser, "sweep").unwrap().id(),
        ID_BASE + 2
    );
}

#[test]
fn public_labels_refuse_secret_bearing_queries_and_unknown_shapes() {
    let root = Scratch::new();
    for label in [
        "GET /backtest.json?token=secret",
        "header:secret",
        "body%20secret",
        "",
        &"x".repeat(97),
        "café",
    ] {
        assert!(begin(&root.0, Origin::Http, label).is_err(), "{label}");
    }
    assert!(page(&root.0, None, MAX_PAGE).unwrap().is_empty());
    let mut attempt = begin(&root.0, Origin::Http, "GET /backtest.json").unwrap();
    attempt.finish(Phase::Completed, 200).unwrap();
    let record = read(&root.0, attempt.id()).unwrap().unwrap();
    assert_eq!(record.response_status, 200);
}

#[test]
fn bounded_pages_address_old_invocations_without_replacing_their_id() {
    let root = Scratch::new();
    for _ in 0..35 {
        drop(begin(&root.0, Origin::Http, "GET /backtest.json").unwrap());
    }
    let first = page(&root.0, None, MAX_PAGE).unwrap();
    assert_eq!(first.len(), 32);
    assert_eq!(first.first().unwrap().id, ID_BASE + 35);
    assert_eq!(first.last().unwrap().id, ID_BASE + 4);
    let next = page(&root.0, Some(ID_BASE + 4), MAX_PAGE).unwrap();
    assert_eq!(
        next.iter().map(|row| row.id).collect::<Vec<_>>(),
        [ID_BASE + 3, ID_BASE + 2, ID_BASE + 1]
    );
    assert!(page(&root.0, Some(ID_BASE + 1), 1).unwrap().is_empty());
    assert!(page(&root.0, Some(0), 1).is_err());
    assert!(page(&root.0, None, 0).is_err());
    assert!(page(&root.0, None, MAX_PAGE + 1).is_err());
    assert!(read(&root.0, 0).is_err());
    assert!(
        read(&root.0, 1).is_err(),
        "a legacy telemetry token cannot select a new invocation"
    );
    assert!(read(&root.0.join("absent"), ID_BASE + 1).is_err());
    assert!(page(&root.0.join("absent"), None, 1).is_err());
}

#[test]
fn unknown_versions_corruption_padding_and_wrong_identity_are_rejected() {
    let root = Scratch::new();
    let mut attempt = begin(&root.0, Origin::Browser, "sweep").unwrap();
    attempt.finish(Phase::Completed, 0).unwrap();
    let record = read(&root.0, ID_BASE + 1).unwrap().unwrap();
    let valid = record.encode().unwrap();
    for byte in [0, 8, 40, 41, 44, 46, 150, 255] {
        let mut corrupt = valid;
        corrupt[byte] ^= 255;
        assert!(Record::decode(&corrupt).is_err());
    }
    for (offset, value) in [(0, 9), (40, 9), (41, 9), (44, 255), (46, 1), (150, 1)] {
        let mut corrupt = valid;
        corrupt[offset] = value;
        let crc = store::crc::crc32c(&corrupt[..CRC]);
        put(&mut corrupt, CRC, &crc.to_le_bytes());
        assert!(Record::decode(&corrupt).is_err());
    }
    let path = own(&base(&root.0), ID_BASE + 1);
    let mut wrong = record;
    wrong.id = ID_BASE + 2;
    OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(&wrong.encode().unwrap())
        .unwrap();
    assert!(read(&root.0, ID_BASE + 1).is_err());
}

struct FailingWriter {
    bytes: Vec<u8>,
    fail_after: usize,
    sync_fails: bool,
    synced: bool,
}
impl io::Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let left = self.fail_after.saturating_sub(self.bytes.len());
        if left == 0 {
            return Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "private full-disk injection",
            ));
        }
        let keep = left.min(bytes.len());
        self.bytes.extend_from_slice(&bytes[..keep]);
        Ok(keep)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl DurableWrite for FailingWriter {
    fn sync(&mut self) -> io::Result<()> {
        self.synced = true;
        if self.sync_fails {
            Err(io::Error::other("private sync failure"))
        } else {
            Ok(())
        }
    }
}

#[test]
fn disk_full_partial_write_and_sync_failure_are_never_acknowledged() {
    for fail_after in [0, 7, 255] {
        let mut writer = FailingWriter {
            bytes: Vec::new(),
            fail_after,
            sync_fails: false,
            synced: false,
        };
        assert!(write_synced(&mut writer, &[0; BYTES]).is_err());
        assert_eq!(writer.bytes.len(), fail_after);
        assert!(!writer.synced);
    }
    let mut writer = FailingWriter {
        bytes: Vec::new(),
        fail_after: BYTES,
        sync_fails: true,
        synced: false,
    };
    assert!(write_synced(&mut writer, &[0; BYTES]).is_err());
    assert!(writer.synced);
    assert_eq!(
        writer.bytes.len(),
        BYTES,
        "a failed sync does not claim that the bytes are absent"
    );
}

#[test]
fn final_component_symlinks_are_refused_without_touching_the_target() {
    use std::os::unix::fs::symlink;
    let root = Scratch::new();
    directory(&base(&root.0)).unwrap();
    let target = root.0.join("untouched");
    fs::write(&target, b"unchanged").unwrap();
    symlink(&target, base(&root.0).join("index.bin")).unwrap();
    assert!(begin(&root.0, Origin::Cli, "sweep").is_err());
    assert!(read(&root.0, ID_BASE + 1).is_err());
    assert_eq!(fs::read(target).unwrap(), b"unchanged");
}

#[test]
fn durable_id_boundaries_are_exact_before_and_after_index_creation() {
    let root = Scratch::new();
    for id in [0, 1, ID_BASE - 1, ID_BASE] {
        assert!(read(&root.0, id).is_err(), "reserved ID {id}");
        assert!(page(&root.0, Some(id), 1).is_err(), "reserved cursor {id}");
    }
    assert_eq!(read(&root.0, ID_BASE + 1).unwrap(), None);
    assert_eq!(read(&root.0, u64::MAX).unwrap(), None);

    let mut attempt = begin(&root.0, Origin::Cli, "sweep").unwrap();
    assert_eq!(attempt.id(), ID_BASE + 1);
    attempt.finish(Phase::Completed, 0).unwrap();
    for id in [0, 1, ID_BASE - 1, ID_BASE] {
        assert!(read(&root.0, id).is_err(), "reserved ID {id} with an index");
        assert!(page(&root.0, Some(id), 1).is_err(), "reserved cursor {id}");
    }
    let first = read(&root.0, ID_BASE + 1).unwrap().unwrap();
    assert_eq!(first.id, ID_BASE + 1);
    assert_eq!(read(&root.0, u64::MAX).unwrap(), None);
    assert_eq!(
        page(&root.0, Some(u64::MAX), MAX_PAGE)
            .unwrap()
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        [ID_BASE + 1]
    );
    let mut maximum = first;
    maximum.id = u64::MAX;
    assert_eq!(Record::decode(&maximum.encode().unwrap()).unwrap(), maximum);
}
