#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

//! BASE-COV-PULL-PS2 (D-7336): the arms of the alert queue its other tests
//! leave unreached, each driven by the input that reaches it. Every fixture
//! is invented: run ids, days, keys and texts.

use std::fs::{self, OpenOptions};
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

const NOW: u64 = 1_791_234_567;

/// A scratch store root, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-alerts-ps2-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }

    fn names(&self) -> Names {
        Names::under(&self.0)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn alert<'a>(key: &'a str, text: &'a str) -> NewAlert<'a> {
    NewAlert {
        kind: 6,
        key,
        day: 18_682,
        run: 4_242,
        text,
    }
}

fn raw(kind_of_record: u8, key: &str, acked: u32, open_after: u32) -> Record {
    Record {
        kind_of_record,
        kind: 6,
        flags: 0,
        day: 18_682,
        run: 4_242,
        put_at: NOW,
        key: key_hash(key),
        acked,
        open_after,
        text: format!("invented {key}"),
    }
}

/// Writes a queue whole from records, and its lock file.
fn raw_queue(scratch: &Scratch, records: &[Record]) {
    let names = scratch.names();
    fs::create_dir_all(&names.dir).unwrap();
    let mut bytes = header(NOW);
    for record in records {
        bytes.extend_from_slice(&record.encode());
    }
    fs::write(&names.queue, bytes).unwrap();
    fs::write(&names.lock, b"").unwrap();
}

/// Flips one byte of ordinal `k`, so its CRC fails.
fn spoil(scratch: &Scratch, ordinal: u64) {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(scratch.names().queue)
        .unwrap();
    let at = offset_of(ordinal) + 60;
    let mut one = [0_u8; 1];
    file.read_exact_at(&mut one, at).unwrap();
    file.write_all_at(&[one[0] ^ 0xFF], at).unwrap();
}

/// Flips one header byte, so the header fails its CRC: the queue is damaged.
fn spoil_header(scratch: &Scratch) {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(scratch.names().queue)
        .unwrap();
    let mut one = [0_u8; 1];
    file.read_exact_at(&mut one, 20).unwrap();
    file.write_all_at(&[one[0] ^ 0xFF], 20).unwrap();
}

/// A RECORD IS 128 BYTES EXACTLY: a slice that holds a body but is not 128
/// bytes long decodes to nothing, whatever it carries.
#[test]
fn a_record_of_any_length_but_128_decodes_to_nothing() {
    let whole = raw(TYPE_ALERT, "k1", 0, 1).encode();
    assert!(Record::decode(&whole).is_some());
    let mut long = whole.clone();
    long.push(0);
    assert_eq!(Record::decode(&long), None, "129 bytes");
    assert_eq!(Record::decode(&whole[..125]), None, "125 bytes");
    assert_eq!(Record::decode(&whole[..123]), None, "no body");
}

/// A HEADER WHOSE CRC HOLDS BUT WHOSE MAGIC IS ANOTHER FORMAT'S is damaged at
/// offset 0, never read as this queue.
#[test]
fn a_header_with_a_good_crc_and_another_magic_is_damaged() {
    let scratch = Scratch::new("MAGIC");
    raw_queue(&scratch, &[raw(TYPE_ALERT, "k1", 0, 1)]);
    let names = scratch.names();
    let mut head = header(NOW);
    head[..8].copy_from_slice(b"BRUTEXZZ");
    let crc = crc32c(&head[..60]);
    head[60..64].copy_from_slice(&crc.to_le_bytes());
    let file = OpenOptions::new().write(true).open(&names.queue).unwrap();
    file.write_all_at(&head, 0).unwrap();
    drop(file);
    let read = File::open(&names.queue).unwrap();
    assert!(matches!(
        shape(&read),
        Err(AlertError::Damaged { offset: 0 })
    ));
    assert!(matches!(
        window(scratch.root()),
        Err(AlertError::Damaged { offset: 0 })
    ));
}

/// A RESERVE THAT CANNOT BE MADE IS SAID ABSENT WITH THE HOST'S KIND, and the
/// alert is written anyway: the reserve is room for a later alert, not this
/// one's.
#[test]
fn a_reserve_that_cannot_be_made_is_absent_and_the_alert_lands() {
    let scratch = Scratch::new("RESERVE-DIR");
    let names = scratch.names();
    fs::create_dir_all(names.reserve.join("inside")).unwrap();
    let put = put_at(scratch.root(), &alert("k1", "invented"), NOW).expect("written");
    let Put::Written {
        ordinal, reserve, ..
    } = put
    else {
        panic!("{put:?}");
    };
    assert_eq!(ordinal, 1);
    assert!(matches!(reserve, Reserve::Absent(_)), "{reserve:?}");
    assert!(
        names.reserve.join("inside").is_dir(),
        "the directory is left"
    );
    let page = window(scratch.root()).expect("reads");
    assert_eq!(page.alerts.len(), 1);
}

/// AN ACKNOWLEDGEMENT WITH A LOCK BUT NO QUEUE names the ordinal it was
/// asked for and makes nothing; a reader there sees no queue.
#[test]
fn a_lock_without_a_queue_is_no_alert_and_no_page() {
    let scratch = Scratch::new("LOCK-ONLY");
    let names = scratch.names();
    fs::create_dir_all(&names.dir).unwrap();
    fs::write(&names.lock, b"").unwrap();
    assert!(matches!(
        ack_at(scratch.root(), 3, "cli", Scope::Whole, NOW),
        Err(AlertError::NoSuchAlert { ordinal: 3 })
    ));
    assert!(!names.queue.exists(), "an acknowledgement makes no queue");
    let page = window(scratch.root()).expect("no queue is an empty page");
    assert!(!page.exists && page.alerts.is_empty());
}

/// AN ACKNOWLEDGEMENT PAST A DAMAGED RECORD still finds no earlier
/// acknowledgement of its alert, and lands; its long name is cut and said.
#[test]
fn an_acknowledgement_past_a_damaged_record_lands_with_its_name_cut() {
    let scratch = Scratch::new("ACK-PAST-DAMAGE");
    raw_queue(
        &scratch,
        &[
            raw(TYPE_ALERT, "k1", 0, 1),
            raw(TYPE_ALERT, "k2", 0, 2),
            raw(TYPE_ALERT, "k3", 0, 3),
        ],
    );
    spoil(&scratch, 2);
    let by =
        "an operator whose invented name is far longer than the seventy-five bytes a record holds";
    let acked = ack_at(scratch.root(), 1, by, Scope::Window, NOW).expect("acknowledged");
    assert_eq!(acked.ordinal, 1);
    let file = File::open(scratch.names().queue).unwrap();
    let mut bytes = vec![0_u8; 128];
    file.read_exact_at(&mut bytes, offset_of(4)).unwrap();
    let record = Record::decode(&bytes).expect("the acknowledgement");
    assert_eq!(record.kind_of_record, TYPE_ACK);
    assert_eq!(
        record.flags & FLAG_TEXT_CUT,
        FLAG_TEXT_CUT,
        "the name was cut"
    );
    assert!(by.starts_with(&record.text) && record.text.len() < by.len());
}

/// A RECORD OF A KIND THIS BUILD DOES NOT WRITE is neither an acknowledgement
/// nor an alert to a put: the key it might carry makes no duplicate.
#[test]
fn a_record_of_an_unknown_kind_makes_no_duplicate() {
    let scratch = Scratch::new("UNKNOWN-KIND");
    raw_queue(&scratch, &[raw(3, "k1", 0, 1)]);
    let put = put_at(scratch.root(), &alert("k1", "invented"), NOW).expect("written");
    assert!(
        matches!(put, Put::Written { ordinal: 2, .. }),
        "the unknown record is no open k1: {put:?}"
    );
}

/// ORDINAL ZERO IS NO ALERT, at any depth of the walk back.
#[test]
fn ordinal_zero_is_no_alert_at_any_depth() {
    let scratch = Scratch::new("ZERO");
    raw_queue(&scratch, &[raw(TYPE_ALERT, "k1", 0, 1)]);
    let writer = Writer::open(scratch.root(), NOW, false).expect("a writer");
    assert!(matches!(
        older_alert(&writer, 0),
        Err(AlertError::NoSuchAlert { ordinal: 0 })
    ));
}

/// A KEPT FILE THAT IS NOT THERE is named as not there, and nothing is
/// opened; one that is a directory is no file.
#[test]
fn a_kept_file_that_is_not_there_is_not_read() {
    let scratch = Scratch::new("KEPT-GONE");
    let dir = scratch.root().join("alerts.damaged-1");
    fs::create_dir_all(&dir).unwrap();
    assert!(matches!(
        of_file(&dir, &mut |_| panic!("a directory is not read")),
        Err(AlertError::NotAFile { .. })
    ));
    let kept = scratch.root().join("alerts.damaged-2");
    let mut seen = 0_u32;
    let refused = of_file(&kept, &mut |_| seen += 1).expect_err("refused");
    let AlertError::Io { op, kind, text } = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(op, "open the kept file");
    assert_eq!(kind, io::ErrorKind::NotFound);
    assert_eq!(text, kept.display().to_string());
    assert_eq!(seen, 0);
}

/// SIXTY-FOUR KEPT NAMES TAKEN IN ONE SECOND: the replacement stops at the
/// sixty-fourth and names it, keeping nothing over another file.
#[test]
fn a_replacement_with_every_kept_name_of_its_second_taken_stops() {
    let scratch = Scratch::new("KEEP-64");
    raw_queue(&scratch, &[raw(TYPE_ALERT, "k1", 0, 1)]);
    spoil_header(&scratch);
    let names = scratch.names();
    fs::write(names.dir.join(format!("alerts.damaged-{NOW}")), b"invented").unwrap();
    for n in 2..=64 {
        fs::write(
            names.dir.join(format!("alerts.damaged-{NOW}.{n}")),
            b"invented",
        )
        .unwrap();
    }
    let before = fs::read(&names.queue).unwrap();
    let refused = replace_damaged_at(scratch.root(), NOW).expect_err("no name left");
    let AlertError::Io { op, kind, .. } = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(op, "keep the damaged queue");
    assert_eq!(kind, io::ErrorKind::AlreadyExists);
    assert_eq!(
        fs::read(&names.queue).unwrap(),
        before,
        "the queue is untouched"
    );
}

/// A REPLACEMENT THAT FINDS ITS QUEUE ALREADY KEPT UNDER A LONG NAME keeps
/// that name, and its first alert's text, too long for a record, is cut.
#[test]
fn a_replacement_kept_under_a_long_name_cuts_its_first_text() {
    let scratch = Scratch::new("KEEP-LONG");
    raw_queue(&scratch, &[raw(TYPE_ALERT, "k1", 0, 1)]);
    spoil_header(&scratch);
    let names = scratch.names();
    let long = names.dir.join(format!(
        "alerts.damaged-{NOW}-invented-and-long-enough-to-overflow-a-record"
    ));
    fs::hard_link(&names.queue, &long).unwrap();
    let done = replace_damaged_at(scratch.root(), NOW).expect("replaced");
    assert_eq!(done.kept, long, "the name the queue already had");
    let file = File::open(&names.queue).unwrap();
    let mut bytes = vec![0_u8; 128];
    file.read_exact_at(&mut bytes, offset_of(1)).unwrap();
    let first = Record::decode(&bytes).expect("the replacement's first alert");
    assert_eq!(first.kind, KIND_QUEUE_REPLACED);
    assert_eq!(first.flags & FLAG_TEXT_CUT, FLAG_TEXT_CUT);
}

/// A KEPT NAME THE HOST WILL NOT LINK, for a reason other than the name being
/// taken, stops the replacement at once with the host's kind: only a taken
/// name moves on to the next. Run where the permission bits bind (D-0995).
#[test]
fn a_kept_name_the_host_will_not_link_stops_the_replacement() {
    crate::support::where_permission_binds(
        "alerts::ps2_tests::a_kept_name_the_host_will_not_link_stops_the_replacement",
        unlinkable_body,
    );
}

fn unlinkable_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let scratch = Scratch::new("KEEP-EACCES");
    raw_queue(&scratch, &[raw(TYPE_ALERT, "k1", 0, 1)]);
    spoil_header(&scratch);
    let names = scratch.names();
    let before = fs::read(&names.queue).unwrap();
    fs::set_permissions(&names.dir, fs::Permissions::from_mode(0o555)).unwrap();
    let refused = replace_damaged_at(scratch.root(), NOW);
    fs::set_permissions(&names.dir, fs::Permissions::from_mode(0o755)).unwrap();
    let Err(AlertError::Io { op, kind, .. }) = refused else {
        panic!("{refused:?}");
    };
    assert_eq!(op, "keep the damaged queue");
    assert_eq!(kind, io::ErrorKind::PermissionDenied);
    assert_eq!(
        fs::read(&names.queue).unwrap(),
        before,
        "the queue is untouched"
    );
    let kept = fs::read_dir(&names.dir)
        .unwrap()
        .filter(|entry| {
            entry.as_ref().is_ok_and(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("alerts.damaged-")
            })
        })
        .count();
    assert_eq!(kept, 0, "nothing kept");
}
