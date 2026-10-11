//! Tests of the alert queue (IA-AL-01 to IA-AL-16; D-4883, D-4893, D-4895).
//!
//! Every fixture is invented: run ids, days, keys and texts. Every store root
//! is a scratch directory this file makes and removes.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail; and one test per test id, \
              each holding every arm of its row."
)]

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::FileExt as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use super::*;

/// A scratch store root, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-alerts-{tag}-{}-{}",
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

    fn len(&self) -> u64 {
        fs::metadata(self.names().queue).unwrap().len()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _gone = fs::remove_dir_all(&self.0);
    }
}

const NOW: u64 = 1_791_234_567;

fn alert<'a>(kind: u8, key: &'a str, text: &'a str) -> NewAlert<'a> {
    NewAlert {
        kind,
        key,
        day: 18_682,
        run: 4_242,
        text,
    }
}

fn written(put: Put) -> u32 {
    match put {
        Put::Written { ordinal, .. } => ordinal,
        Put::Duplicate { ordinal } => panic!("a duplicate of {ordinal}, not a write"),
    }
}

/// An alert record as a raw queue holds it.
fn raw_alert(key: &str, open_after: u32) -> Record {
    Record {
        kind_of_record: TYPE_ALERT,
        kind: 6,
        flags: 0,
        day: 18_682,
        run: 4_242,
        put_at: NOW,
        key: key_hash(key),
        acked: 0,
        open_after,
        text: format!("invented {key}"),
    }
}

fn raw_ack(of: u32, open_after: u32) -> Record {
    Record {
        kind_of_record: TYPE_ACK,
        kind: 0,
        flags: 0,
        day: 0,
        run: 0,
        put_at: NOW,
        key: [0; 16],
        acked: of,
        open_after,
        text: "cli".to_owned(),
    }
}

/// Writes a queue whole from records, without the writer under test.
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

/// `n` open alerts with keys `k1..kn`.
fn open_alerts(n: u32) -> Vec<Record> {
    (1..=n).map(|i| raw_alert(&format!("k{i}"), i)).collect()
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

fn append_bytes(scratch: &Scratch, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .append(true)
        .open(scratch.names().queue)
        .unwrap();
    file.write_all(bytes).unwrap();
}

// IA-AL-01.
#[test]
fn an_alert_is_put_once_per_key_across_restarts() {
    let scratch = Scratch::new("once");
    let first = put_at(
        scratch.root(),
        &alert(6, "hole:4242:futures:no_zip", "a"),
        NOW,
    )
    .unwrap();
    assert_eq!(written(first), 1);
    let len = scratch.len();
    assert_eq!(len, 64 + 128, "born whole, then one record");
    // Nothing is kept in memory, so a second call is what a restarted api or a
    // second process makes: the file alone answers.
    let again = put_at(
        scratch.root(),
        &alert(6, "hole:4242:futures:no_zip", "b"),
        NOW + 9,
    );
    assert_eq!(again, Ok(Put::Duplicate { ordinal: 1 }));
    assert_eq!(scratch.len(), len, "a duplicate writes nothing");
    let other = put_at(
        scratch.root(),
        &alert(6, "hole:4242:futures:empty", "c"),
        NOW,
    )
    .unwrap();
    assert_eq!(written(other), 2);

    // THE WINDOW'S EDGE (L-IA-30). With 1,023 records after it the key is still
    // inside the newest 1,024 and is a duplicate; with 1,024 after it, it is
    // outside and is put again: a repeat, never a loss.
    for (newer, repeat) in [(1_023_u32, false), (1_024, true)] {
        let edge = Scratch::new("edge");
        let mut records = vec![raw_alert("the-key", 1)];
        records.extend((0..newer).map(|i| raw_alert(&format!("n{i}"), i + 2)));
        raw_queue(&edge, &records);
        let got = put_at(edge.root(), &alert(6, "the-key", "x"), NOW).unwrap();
        if repeat {
            assert_eq!(written(got), newer + 2);
        } else {
            assert_eq!(got, Put::Duplicate { ordinal: 1 });
        }
    }
}

// IA-AL-02.
#[test]
fn a_torn_or_damaged_alert_is_named_never_lost() {
    // A torn append (bytes past the last whole record) is written over.
    let torn = Scratch::new("torn");
    raw_queue(&torn, &open_alerts(3));
    append_bytes(&torn, &[0xAB; 57]);
    let page = window(torn.root()).unwrap();
    assert_eq!((page.records, page.torn, page.alerts.len()), (3, 57, 3));
    assert!(page.damaged.is_empty(), "a torn tail is not a record");
    let got = put_at(torn.root(), &alert(5, "disk_full:1", "t"), NOW).unwrap();
    assert!(
        matches!(
            got,
            Put::Written {
                ordinal: 4,
                torn: 57,
                wrote_over: None,
                ..
            }
        ),
        "{got:?}"
    );
    assert_eq!(
        torn.len(),
        64 + 128 * 4,
        "the torn bytes are gone under the record"
    );
    let page = window(torn.root()).unwrap();
    assert_eq!((page.torn, page.alerts.len()), (0, 4));
    assert_eq!(page.alerts[0].record.open_after, 4);

    // A whole last record that fails its CRC is written over, and the put says
    // at which offset; "open after" comes from the record before it.
    let unsynced = Scratch::new("unsynced");
    raw_queue(&unsynced, &open_alerts(3));
    spoil(&unsynced, 3);
    assert_eq!(window(unsynced.root()).unwrap().damaged, vec![3]);
    let got = put_at(unsynced.root(), &alert(5, "disk_full:2", "u"), NOW).unwrap();
    let Put::Written {
        ordinal,
        wrote_over,
        ..
    } = got
    else {
        panic!("{got:?}")
    };
    assert_eq!((ordinal, wrote_over), (3, Some(offset_of(3))));
    assert_eq!(unsynced.len(), 64 + 128 * 3);
    let page = window(unsynced.root()).unwrap();
    assert!(page.damaged.is_empty());
    assert_eq!(
        page.alerts[0].record.open_after, 3,
        "2 before it, plus this one"
    );

    // The same with a single record: nothing before it, so the count restarts.
    let only = Scratch::new("only");
    raw_queue(&only, &open_alerts(1));
    spoil(&only, 1);
    let got = put_at(only.root(), &alert(5, "disk_full:3", "o"), NOW).unwrap();
    assert!(matches!(
        got,
        Put::Written {
            ordinal: 1,
            wrote_over: Some(64),
            ..
        }
    ));
    assert_eq!(window(only.root()).unwrap().alerts[0].record.open_after, 1);

    // Two bad records at the tail cannot come from one interrupted append: the
    // put refuses by name and writes nothing; every reader shows the damage.
    let two = Scratch::new("two");
    raw_queue(&two, &open_alerts(5));
    spoil(&two, 4);
    spoil(&two, 5);
    let before = fs::read(two.names().queue).unwrap();
    let refused = put_at(two.root(), &alert(5, "disk_full:4", "x"), NOW);
    assert_eq!(
        refused,
        Err(AlertError::Damaged {
            offset: offset_of(4)
        })
    );
    assert_eq!(refused.unwrap_err().code(), "alerts_damaged");
    assert_eq!(
        fs::read(two.names().queue).unwrap(),
        before,
        "nothing written"
    );
    let page = window(two.root()).unwrap();
    assert_eq!(page.damaged, vec![5, 4]);
    assert_eq!(page.alerts.len(), 3);
    assert_eq!(
        ack_at(two.root(), 1, "cli", Scope::Whole, NOW),
        Err(AlertError::Damaged {
            offset: offset_of(4)
        }),
        "an acknowledgement is a write too"
    );

    // A record damaged in place INSIDE the window, with a good tail: the put
    // goes on (it checks only the last whole record), never writes over a good
    // record, and the reader names the damaged one instead of dropping it.
    let inside = Scratch::new("inside");
    raw_queue(&inside, &open_alerts(6));
    spoil(&inside, 2);
    let good_tail = fs::read(inside.names().queue).unwrap();
    let got = put_at(inside.root(), &alert(5, "disk_full:5", "i"), NOW).unwrap();
    assert!(matches!(
        got,
        Put::Written {
            ordinal: 7,
            wrote_over: None,
            ..
        }
    ));
    let after = fs::read(inside.names().queue).unwrap();
    assert_eq!(
        &after[..good_tail.len()],
        &good_tail[..],
        "no earlier byte moved"
    );
    let page = window(inside.root()).unwrap();
    assert_eq!(page.damaged, vec![2]);
    assert_eq!(page.alerts.len(), 6);
    assert_eq!(
        page.older_unacknowledged, 1,
        "the count includes the damaged one"
    );

    // THE PUT READS THE WINDOW AND NOTHING OLDER. 3,000 records of which every
    // one older than the newest 1,024 fails its CRC: a put that scanned the
    // file would refuse or miscount; this one appends and counts from the tail.
    let long = Scratch::new("long");
    raw_queue(&long, &open_alerts(3_000));
    for ordinal in 1..=(3_000 - 1_024) {
        spoil(&long, ordinal);
    }
    let got = put_at(long.root(), &alert(5, "disk_full:6", "l"), NOW).unwrap();
    assert!(matches!(got, Put::Written { ordinal: 3_001, .. }));
    let page = window(long.root()).unwrap();
    assert!(
        page.damaged.is_empty(),
        "the damage is older than the one read"
    );
    assert_eq!(page.alerts.len(), 1_024);
    assert_eq!(page.older_unacknowledged, 3_001 - 1_024);
}

// IA-AL-03.
#[test]
fn an_acknowledged_alert_is_recorded_and_hidden() {
    let scratch = Scratch::new("ack");
    for key in ["a", "b", "c"] {
        put_at(scratch.root(), &alert(9, key, key), NOW).unwrap();
    }
    let acked = ack_at(scratch.root(), 2, "api", Scope::Window, NOW + 5).unwrap();
    assert_eq!(
        acked,
        Acked {
            ordinal: 2,
            record: 4,
            kind: 9,
            run: 4_242,
            open_after: 2
        }
    );
    assert_eq!(
        scratch.len(),
        64 + 128 * 4,
        "the acknowledgement is itself a record"
    );
    let page = window(scratch.root()).unwrap();
    let listed: Vec<u32> = page.alerts.iter().map(|a| a.ordinal).collect();
    assert_eq!(listed, vec![3, 1], "newest first, 2 hidden");
    assert_eq!(page.older_unacknowledged, 0);

    // The record says who and when, not only the event.
    let mut kept = Vec::new();
    of_file(&scratch.names().queue, &mut |k| kept.push(k)).unwrap();
    let Kept::Good(4, record) = &kept[3] else {
        panic!("{:?}", kept[3])
    };
    assert_eq!(
        (
            record.kind_of_record,
            record.acked,
            record.text.as_str(),
            record.put_at
        ),
        (TYPE_ACK, 2, "api", NOW + 5)
    );
    assert_eq!(record.key, [0; 16]);

    // Twice is refused and writes nothing.
    assert_eq!(
        ack_at(scratch.root(), 2, "api", Scope::Window, NOW),
        Err(AlertError::AlreadyAcknowledged { ordinal: 2 })
    );
    assert_eq!(scratch.len(), 64 + 128 * 4);

    // Outside the window: the route's scope refuses by name, the cli's does it.
    let far = Scratch::new("ack-far");
    raw_queue(&far, &open_alerts(1_030));
    let refused = ack_at(far.root(), 3, "api", Scope::Window, NOW);
    assert_eq!(refused, Err(AlertError::AckOutsideWindow { ordinal: 3 }));
    assert!(
        refused
            .unwrap_err()
            .to_string()
            .contains("cli alerts --ack 3")
    );
    assert_eq!(far.len(), 64 + 128 * 1_030, "a refusal writes nothing");
    let done = ack_at(far.root(), 3, "cli", Scope::Whole, NOW).unwrap();
    assert_eq!((done.record, done.open_after), (1_031, 1_029));
    assert_eq!(
        ack_at(far.root(), 3, "cli", Scope::Whole, NOW),
        Err(AlertError::AlreadyAcknowledged { ordinal: 3 })
    );
    // Once 1,100 more records push that acknowledgement out of the window too,
    // the cli still finds it in the records after the alert.
    for i in 0..1_100 {
        put_at(far.root(), &alert(6, &format!("later{i}"), "x"), NOW).unwrap();
    }
    assert_eq!(
        ack_at(far.root(), 3, "cli", Scope::Whole, NOW),
        Err(AlertError::AlreadyAcknowledged { ordinal: 3 })
    );
}

// IA-AL-04.
#[test]
fn an_older_unacknowledged_alert_never_leaves_view() {
    let scratch = Scratch::new("older");
    let stuck = alert(8, "runs_ledger:4096", "the runs ledger fails at 4096");
    assert_eq!(written(put_at(scratch.root(), &stuck, NOW).unwrap()), 1);
    // 60 runs put 550 alerts and acknowledge each: 1,100 newer records.
    for i in 0..550_u32 {
        let run = u64::from(i % 60) + 1;
        let key = format!("mismatch:{run}:{i}");
        let new = NewAlert {
            kind: 9,
            key: &key,
            day: 0,
            run,
            text: "x",
        };
        let ordinal = written(put_at(scratch.root(), &new, NOW).unwrap());
        ack_at(scratch.root(), ordinal, "cli", Scope::Window, NOW).unwrap();
    }
    let page = window(scratch.root()).unwrap();
    assert_eq!(page.records, 1_101);
    assert!(
        page.alerts.is_empty(),
        "every alert in the window is acknowledged"
    );
    assert_eq!(page.older_unacknowledged, 1, "exactly the one pushed out");
    assert!(page.first > 1, "ordinal 1 is outside the one read");

    let listed = older(scratch.root()).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].ordinal, listed[0].word().as_ref()),
        (1, "runs_ledger_damaged")
    );

    ack_at(scratch.root(), 1, "cli", Scope::Whole, NOW).unwrap();
    assert_eq!(window(scratch.root()).unwrap().older_unacknowledged, 0);
    assert!(older(scratch.root()).unwrap().is_empty());
}

// IA-AL-05.
#[test]
fn the_alert_queue_format_is_v1_byte_for_byte() {
    // The header, every offset of section 7.1.
    let head = header(0x0102_0304_0506_0708);
    assert_eq!(head.len(), 64);
    assert_eq!(&head[0..8], b"BRUTEXAL");
    assert_eq!(&head[8..10], &[1, 0], "version 1");
    assert_eq!(&head[10..12], &[128, 0], "record stride");
    assert_eq!(&head[12..14], &[64, 0], "header length");
    assert_eq!(&head[14..16], &[0, 0], "reserved");
    assert_eq!(
        &head[16..24],
        &[8, 7, 6, 5, 4, 3, 2, 1],
        "created, little-endian"
    );
    assert_eq!(&head[24..60], &[0_u8; 36], "reserved");
    assert_eq!(&head[60..64], &crc32c(&head[..60]).to_le_bytes());

    // A record, every offset.
    let record = Record {
        kind_of_record: TYPE_ALERT,
        kind: 6,
        flags: FLAG_TEXT_CUT,
        day: 0x0A0B_0C0D,
        run: 0x1112_1314_1516_1718,
        put_at: 0x2122_2324_2526_2728,
        key: key_hash("hole:7:options:no_zip"),
        acked: 0x3132_3334,
        open_after: 0x4142_4344,
        text: "héllo".to_owned(),
    };
    let bytes = record.encode();
    assert_eq!(bytes.len(), 128);
    assert_eq!(&bytes[0..4], &[1, 6, 1, 0], "type, kind, flags, reserved");
    assert_eq!(&bytes[4..8], &[0x0D, 0x0C, 0x0B, 0x0A]);
    assert_eq!(
        &bytes[8..16],
        &[0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11]
    );
    assert_eq!(
        &bytes[16..24],
        &[0x28, 0x27, 0x26, 0x25, 0x24, 0x23, 0x22, 0x21]
    );
    assert_eq!(
        &bytes[24..40],
        &brutex_core::blake3::hash(b"hole:7:options:no_zip")[..16]
    );
    assert_eq!(
        &bytes[40..44],
        &[0x34, 0x33, 0x32, 0x31],
        "the acknowledged ordinal"
    );
    assert_eq!(&bytes[44..48], &[0x44, 0x43, 0x42, 0x41], "open after");
    assert_eq!(bytes[48], 6, "text length in bytes");
    assert_eq!(&bytes[49..55], "héllo".as_bytes());
    assert_eq!(&bytes[55..124], &[0_u8; 69], "zero-padded");
    assert_eq!(&bytes[124..128], &crc32c(&bytes[..124]).to_le_bytes());
    assert_eq!(Record::decode(&bytes), Some(record));
    assert_eq!(
        (HEADER_LEN, RECORD_LEN, WINDOW, TEXT_MAX),
        (64, 128, 1_024, 75)
    );
    assert_eq!(offset_of(1), 64);
    assert_eq!(offset_of(41), 64 + 128 * 40);

    // The kind table: append-only, so pinned word for word.
    let words: Vec<(u8, &str)> = KINDS.to_vec();
    assert_eq!(
        words,
        vec![
            (1, "interrupted_twice"),
            (2, "store_lost"),
            (3, "build_skew"),
            (4, "calendar_horizon"),
            (5, "disk_full"),
            (6, "hole"),
            (7, "store_damaged"),
            (8, "runs_ledger_damaged"),
            (9, "mismatch"),
            (10, "spool_damaged"),
            (11, "verdict_refused"),
            (12, "queue_replaced"),
            (13, "run_ended_early"),
            (14, "interrupted"),
            (15, "stalled"),
            (16, "autopilot_stalled"),
            (17, "disk_low"),
            (18, "file_shrank"),
            (19, "month_damaged"),
            (20, "index_refused"),
        ]
    );
    for (kind, word) in KINDS {
        assert_eq!(kind_word(kind), Some(word));
        assert_eq!(kind_of(word), Some(kind));
    }
    assert_eq!(
        (kind_word(0), kind_word(21), kind_of("nothing")),
        (None, None, None)
    );

    // Born whole: what the first put leaves is exactly header + one record,
    // and the bytes on disk are the encoder's.
    let scratch = Scratch::new("format");
    put_at(
        scratch.root(),
        &alert(6, "hole:7:options:no_zip", "héllo"),
        NOW,
    )
    .unwrap();
    let disk = fs::read(scratch.names().queue).unwrap();
    assert_eq!(disk.len(), 192);
    assert_eq!(&disk[..64], &header(NOW)[..]);
    assert_eq!(&disk[64..68], &[1, 6, 0, 0]);
    assert_eq!(&disk[64 + 44..64 + 48], &[1, 0, 0, 0], "open after 1");
    assert!(
        !scratch
            .names()
            .dir
            .join(format!("alerts.{}.tmp", std::process::id()))
            .exists()
    );

    // A file shorter than 64 bytes, or a header that fails, cannot come from
    // genesis: refused by name at offset 0, by the writer and the readers.
    for bytes in [&disk[..63], &[][..]] {
        fs::write(scratch.names().queue, bytes).unwrap();
        assert_eq!(
            window(scratch.root()),
            Err(AlertError::Damaged { offset: 0 })
        );
        assert_eq!(
            put_at(scratch.root(), &alert(6, "k", "t"), NOW),
            Err(AlertError::Damaged { offset: 0 })
        );
    }
    let mut bad = disk.clone();
    bad[20] ^= 1;
    fs::write(scratch.names().queue, &bad).unwrap();
    assert_eq!(
        window(scratch.root()),
        Err(AlertError::Damaged { offset: 0 })
    );
}

// IA-AL-06.
#[test]
fn a_reader_never_meets_a_half_written_alert() {
    let scratch = Scratch::new("reader");
    put_at(scratch.root(), &alert(6, "first", "first"), NOW).unwrap();
    let names = scratch.names();

    // A put in flight, by hand: the exclusive lock is held and the record is
    // on disk in a state no reader may see (its bytes written, its CRC not).
    let lock_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&names.lock)
        .unwrap();
    let held = Flock::lock(lock_file, names.lock.clone()).unwrap();
    let mut second = raw_alert("second", 2).encode();
    let good = second.clone();
    second[124] ^= 0xFF;
    append_bytes(&scratch, &second);

    let (said, heard) = mpsc::channel();
    let root = scratch.root().to_path_buf();
    let reader = std::thread::spawn(move || said.send(window(&root)).unwrap());
    assert!(
        heard.recv_timeout(Duration::from_millis(300)).is_err(),
        "the reader waits for the put instead of reading its half-written record"
    );
    // The put finishes: the record is whole, then the lock is released.
    let file = OpenOptions::new().write(true).open(&names.queue).unwrap();
    file.write_all_at(&good, offset_of(2)).unwrap();
    held.release().unwrap();

    let page = heard
        .recv_timeout(Duration::from_secs(20))
        .unwrap()
        .unwrap();
    reader.join().unwrap();
    assert!(page.damaged.is_empty(), "never listed as damaged");
    assert_eq!(
        page.alerts.iter().map(|a| a.ordinal).collect::<Vec<_>>(),
        vec![2, 1]
    );

    // Readers share: two at once do not wait for each other.
    let lock_file = OpenOptions::new().read(true).open(&names.lock).unwrap();
    let shared = Flock::lock_shared(lock_file, names.lock.clone()).unwrap();
    assert_eq!(window(scratch.root()).unwrap().alerts.len(), 2);
    shared.release().unwrap();
}

// IA-AL-07.
#[test]
fn a_damaged_queue_is_replaced_and_its_open_alerts_stay_in_view() {
    let scratch = Scratch::new("replace");
    let names = scratch.names();
    // A healthy queue is never replaced; nor is one that does not exist.
    assert_eq!(
        replace_damaged_at(scratch.root(), NOW),
        Err(AlertError::NotDamaged)
    );
    raw_queue(&scratch, &open_alerts(5));
    assert_eq!(
        replace_damaged_at(scratch.root(), NOW),
        Err(AlertError::NotDamaged)
    );
    assert_eq!(scratch.len(), 64 + 128 * 5);

    spoil(&scratch, 4);
    spoil(&scratch, 5);
    let damaged = fs::read(&names.queue).unwrap();
    assert!(put_at(scratch.root(), &alert(6, "k", "t"), NOW).is_err());

    let done = replace_damaged_at(scratch.root(), NOW).unwrap();
    let kept = names.dir.join(format!("alerts.damaged-{NOW}"));
    assert_eq!(
        done,
        Replaced {
            kept: kept.clone(),
            open: Some(3),
            ordinal: 1
        }
    );
    assert_eq!(fs::read(&kept).unwrap(), damaged, "kept byte for byte");

    // The new queue: born by genesis, one alert of kind 12 that stays open.
    let page = window(scratch.root()).unwrap();
    assert_eq!((page.records, page.alerts.len()), (1, 1));
    let first = &page.alerts[0];
    assert_eq!(first.word(), "queue_replaced");
    assert_eq!(
        first.record.text,
        format!("kept alerts.damaged-{NOW}; open alerts it held: 3")
    );
    assert!(!first.record.text_cut());
    assert_eq!(
        written(put_at(scratch.root(), &alert(6, "k", "t"), NOW).unwrap()),
        2
    );

    // The kept file's good records are listed and its damaged ones named.
    let mut seen = Vec::new();
    let shape = of_file(&kept, &mut |k| seen.push(k)).unwrap();
    assert_eq!(
        shape,
        KeptShape {
            header_ok: true,
            records: 5,
            torn: 0
        }
    );
    assert_eq!(seen.len(), 5);
    assert!(matches!(&seen[0], Kept::Good(1, r) if r.text == "invented k1"));
    assert!(matches!(&seen[2], Kept::Good(3, r) if r.open_after == 3));
    assert_eq!(&seen[3..], &[Kept::Damaged(4), Kept::Damaged(5)]);

    // A second replacement in the SAME second (a clock stuck or far behind)
    // never writes over the first kept file: it takes the next name.
    spoil(&scratch, 1);
    spoil(&scratch, 2);
    let again = replace_damaged_at(scratch.root(), NOW).unwrap();
    assert_eq!(
        again.kept,
        names.dir.join(format!("alerts.damaged-{NOW}.2"))
    );
    assert_eq!(again.open, None, "no record of it passes its CRC");
    assert_eq!(
        fs::read(&kept).unwrap(),
        damaged,
        "the first kept file is untouched"
    );
    let page = window(scratch.root()).unwrap();
    assert!(
        page.alerts[0]
            .record
            .text
            .ends_with("open alerts it held: unknown")
    );

    // A queue whose header is gone is damaged too, and is replaced.
    fs::write(&names.queue, b"short").unwrap();
    let third = replace_damaged_at(scratch.root(), NOW + 1).unwrap();
    assert_eq!(third.open, None);
    let mut none = Vec::new();
    let shape = of_file(&third.kept, &mut |k| none.push(k)).unwrap();
    assert_eq!(
        shape,
        KeptShape {
            header_ok: false,
            records: 0,
            torn: 0
        }
    );
    assert!(none.is_empty());
}

// IA-AL-08.
#[test]
fn an_alert_text_is_cut_at_a_character_boundary() {
    let ascii = "x".repeat(80);
    for (lead, wide, kept_bytes) in [
        (74_usize, "é", 74_usize), // 2 bytes over byte 75: one step back
        (73, "€", 73),             // 3 bytes: two steps back
        (72, "😀", 72),            // 4 bytes: three steps back
        (73, "é", 75),             // ends exactly at 75: kept whole
    ] {
        let text = format!("{}{wide}{}", &ascii[..lead], &ascii[..10]);
        let (fitted, cut) = fit_text(&text);
        assert!(cut);
        assert_eq!(fitted.len(), kept_bytes, "{lead} + {wide}");
        assert!(
            text.starts_with(&fitted),
            "whole characters of the original"
        );
    }
    assert_eq!(fit_text(&ascii[..75]), (ascii[..75].to_owned(), false));
    assert_eq!(fit_text(&ascii[..76]), (ascii[..75].to_owned(), true));
    assert_eq!(fit_text(""), (String::new(), false));

    // Through the queue: the flag is on the record and the field decodes.
    let scratch = Scratch::new("cut");
    let long = format!("{}€€€", &ascii[..73]);
    let got = put_at(scratch.root(), &alert(6, "k", &long), NOW).unwrap();
    assert!(matches!(got, Put::Written { text_cut: true, .. }));
    let page = window(scratch.root()).unwrap();
    assert!(page.alerts[0].record.text_cut());
    assert_eq!(page.alerts[0].record.text, ascii[..73]);
    let disk = fs::read(scratch.names().queue).unwrap();
    assert_eq!((disk[64 + 2], disk[64 + 48]), (FLAG_TEXT_CUT, 73));
    assert!(std::str::from_utf8(&disk[64 + 49..64 + 49 + 73]).is_ok());
}

// IA-AL-09.
#[test]
fn a_first_put_creates_the_queue_and_its_directory() {
    let scratch = Scratch::new("first");
    let names = scratch.names();
    assert!(!names.dir.exists(), "a fresh store has no audit/");
    // Reading a store with no queue is no alert, and creates nothing.
    assert_eq!(window(scratch.root()), Ok(Page::default()));
    assert!(older(scratch.root()).unwrap().is_empty());
    assert!(!names.dir.exists(), "a read never writes");

    let got = put_at(scratch.root(), &alert(2, "store_lost:9", "t"), NOW).unwrap();
    assert!(
        matches!(
            got,
            Put::Written {
                ordinal: 1,
                created: true,
                reserve: Reserve::Held,
                ..
            }
        ),
        "{got:?}"
    );
    assert!(names.queue.is_file() && names.lock.is_file());
    assert_eq!(fs::metadata(&names.reserve).unwrap().len(), 512 * 128);

    // A store root that does not exist is refused by name and never created.
    let missing = scratch.root().join("no-such-store");
    let refused = put_at(&missing, &alert(2, "store_lost:9", "t"), NOW);
    assert_eq!(
        refused,
        Err(AlertError::Unavailable {
            root: missing.clone()
        })
    );
    assert_eq!(refused.unwrap_err().code(), "alerts_unavailable");
    assert_eq!(
        window(&missing),
        Err(AlertError::Unavailable {
            root: missing.clone()
        })
    );
    assert!(!missing.exists());
}

// IA-AL-10.
#[test]
fn garbage_alert_parameters_are_refused_by_name() {
    let scratch = Scratch::new("garbage");
    // (b) A kind outside the table; an empty key. Nothing is created.
    for kind in [0_u8, 21, 255] {
        let refused = put_at(scratch.root(), &alert(kind, "k", "t"), NOW);
        assert_eq!(refused, Err(AlertError::KindUnknown { kind }));
        assert_eq!(refused.unwrap_err().code(), "alert_kind_unknown");
    }
    assert_eq!(
        put_at(scratch.root(), &alert(6, "", "t"), NOW),
        Err(AlertError::KeyEmpty)
    );
    assert!(
        !scratch.names().dir.exists(),
        "a refused put creates nothing"
    );
    // An acknowledgement on a store with no queue: no such alert, no file.
    assert_eq!(
        ack_at(scratch.root(), 1, "cli", Scope::Whole, NOW),
        Err(AlertError::NoSuchAlert { ordinal: 1 })
    );
    assert!(!scratch.names().dir.exists());

    // (b) Control characters and newlines become spaces before the cut: the
    // text is shown on a page.
    put_at(
        scratch.root(),
        &alert(6, "ctl", "a\nb\tc\u{7}d\r\u{1b}[0m"),
        NOW,
    )
    .unwrap();
    let page = window(scratch.root()).unwrap();
    assert_eq!(page.alerts[0].record.text, "a b c d  [0m");

    // (c) Ordinal 0, one past the last record, and an acknowledgement's own
    // ordinal: all three answer no_such_alert and write nothing.
    ack_at(scratch.root(), 1, "cli", Scope::Window, NOW).unwrap();
    let len = scratch.len();
    for ordinal in [0_u32, 3, 2, u32::MAX] {
        let refused = ack_at(scratch.root(), ordinal, "cli", Scope::Whole, NOW);
        assert_eq!(
            refused,
            Err(AlertError::NoSuchAlert { ordinal }),
            "{ordinal}"
        );
        assert_eq!(refused.unwrap_err().code(), "no_such_alert");
    }
    assert_eq!(scratch.len(), len);

    // (e) "Open after" already at 0 when an acknowledgement arrives (a count a
    // damaged tail restarted): saturating, never below 0, never a panic.
    let zero = Scratch::new("zero");
    raw_queue(&zero, &[raw_alert("k1", 0)]);
    let done = ack_at(zero.root(), 1, "cli", Scope::Window, NOW).unwrap();
    assert_eq!(done.open_after, 0);
    assert_eq!(window(zero.root()).unwrap().older_unacknowledged, 0);

    // A `before` no page has: empty, not an error; a huge one is the newest.
    assert!(page_is_empty(&page_of(&zero, Some(0))));
    assert!(page_is_empty(&page_of(&zero, Some(1))));
    assert_eq!(page_of(&zero, Some(u32::MAX)).first, 1);
}

fn page_of(scratch: &Scratch, before: Option<u32>) -> Page {
    page(scratch.root(), before).unwrap()
}

fn page_is_empty(page: &Page) -> bool {
    page.exists && page.alerts.is_empty() && page.first == 0 && page.damaged.is_empty()
}

// IA-AL-11.
#[test]
fn an_acknowledged_fault_that_persists_is_raised_again() {
    let scratch = Scratch::new("again");
    let fault = alert(8, "runs_ledger:512", "the ledger fails at 512");
    assert_eq!(written(put_at(scratch.root(), &fault, NOW).unwrap()), 1);
    assert_eq!(
        put_at(scratch.root(), &fault, NOW),
        Ok(Put::Duplicate { ordinal: 1 })
    );
    ack_at(scratch.root(), 1, "api", Scope::Window, NOW).unwrap();
    // The fault is still there the next time its condition is met: it comes
    // back as a new alert, and is then once more a duplicate while open.
    assert_eq!(
        written(put_at(scratch.root(), &fault, NOW + 60).unwrap()),
        3
    );
    assert_eq!(
        put_at(scratch.root(), &fault, NOW + 61),
        Ok(Put::Duplicate { ordinal: 3 })
    );
    let page = window(scratch.root()).unwrap();
    assert_eq!(
        page.alerts.iter().map(|a| a.ordinal).collect::<Vec<_>>(),
        vec![3]
    );
    assert_eq!(page.alerts[0].record.open_after, 1);
}

// IA-AL-12 (the rules audit's RA-310).
#[test]
fn a_later_builds_records_are_named_never_dropped() {
    let scratch = Scratch::new("later");
    let mut new_kind = raw_alert("k1", 1);
    new_kind.kind = 77;
    let mut new_type = raw_alert("k2", 1);
    new_type.kind_of_record = 9;
    let mut new_flags = raw_alert("k3", 2);
    new_flags.flags = 0b1000_0010;
    raw_queue(&scratch, &[new_kind, new_type, new_flags]);

    let page = window(scratch.root()).unwrap();
    assert!(page.damaged.is_empty(), "they pass their CRC: not damage");
    let shown: Vec<(u32, String)> = page
        .alerts
        .iter()
        .map(|a| (a.ordinal, a.word().into_owned()))
        .collect();
    assert_eq!(
        shown,
        vec![(3, "hole".to_owned()), (1, "kind_77".to_owned())]
    );
    assert_eq!(
        page.unknown,
        vec![(2, 9)],
        "a record type this build cannot name"
    );
    assert!(
        !page.alerts[0].record.text_cut(),
        "bit 0 is clear; bits it cannot name are kept"
    );
    assert_eq!(page.alerts[0].record.flags, 0b1000_0010);
    // An alert of a kind this build cannot name can still be acknowledged.
    assert_eq!(
        ack_at(scratch.root(), 1, "cli", Scope::Window, NOW)
            .unwrap()
            .kind,
        77
    );
    // But the writer names what it writes.
    assert_eq!(
        put_at(scratch.root(), &alert(77, "k", "t"), NOW),
        Err(AlertError::KindUnknown { kind: 77 })
    );

    // A later FORMAT: a header that passes its CRC and says version 2 is
    // refused by name, by every door, and nothing is written.
    let mut head = header(NOW);
    head[8] = 2;
    let crc = crc32c(&head[..60]).to_le_bytes();
    head[60..64].copy_from_slice(&crc);
    fs::write(scratch.names().queue, &head).unwrap();
    let version = AlertError::VersionUnknown { found: 2 };
    assert_eq!(window(scratch.root()), Err(version.clone()));
    assert_eq!(
        put_at(scratch.root(), &alert(6, "k", "t"), NOW),
        Err(version.clone())
    );
    assert_eq!(
        ack_at(scratch.root(), 1, "cli", Scope::Whole, NOW),
        Err(version.clone())
    );
    assert_eq!(
        replace_damaged_at(scratch.root(), NOW),
        Err(version.clone())
    );
    assert_eq!(version.code(), "alerts_version_unknown");
    assert_eq!(fs::read(scratch.names().queue).unwrap(), head);
}

// IA-AL-13 (RA-161, D-4895 (d)).
#[test]
fn a_full_disk_spends_the_reserve_and_the_alert_is_in_the_queue() {
    let scratch = Scratch::new("full");
    let names = scratch.names();
    put_at(scratch.root(), &alert(6, "first", "t"), NOW).unwrap();
    assert!(names.reserve.is_file());

    // The disk is full for one write: the reserve is given up and the alert,
    // `disk_full` itself, is in the queue.
    FULL_WRITES.with(|left| left.set(1));
    let got = put_at(scratch.root(), &alert(5, "disk_full:4242", "no space"), NOW).unwrap();
    assert!(
        matches!(
            got,
            Put::Written {
                ordinal: 2,
                reserve: Reserve::Spent,
                ..
            }
        ),
        "{got:?}"
    );
    assert!(
        !names.reserve.exists(),
        "its 64 KiB went back to the volume"
    );
    let page = window(scratch.root()).unwrap();
    assert_eq!(page.alerts[0].word(), "disk_full");

    // The next put that finds room holds a reserve again.
    let got = put_at(scratch.root(), &alert(6, "later", "t"), NOW).unwrap();
    assert!(matches!(
        got,
        Put::Written {
            ordinal: 3,
            reserve: Reserve::Held,
            ..
        }
    ));
    assert_eq!(fs::metadata(&names.reserve).unwrap().len(), 512 * 128);

    // Still full after the reserve: an error by name, never a panic, nothing
    // half-written, and the queue takes the next alert when there is room.
    FULL_WRITES.with(|left| left.set(2));
    let refused = put_at(scratch.root(), &alert(5, "disk_full:4243", "no space"), NOW);
    let Err(AlertError::Io { kind, op, .. }) = &refused else {
        panic!("{refused:?}")
    };
    assert_eq!(
        (*kind, *op),
        (
            io::ErrorKind::StorageFull,
            "append after spending the reserve"
        )
    );
    assert_eq!(refused.unwrap_err().code(), "alerts_io");
    assert_eq!(scratch.len(), 64 + 128 * 3);
    assert_eq!(
        written(put_at(scratch.root(), &alert(5, "disk_full:4243", "x"), NOW).unwrap()),
        4
    );
}

// IA-AL-14 (RA-264).
#[test]
fn an_older_page_is_one_read_at_any_depth() {
    let scratch = Scratch::new("paging");
    // 2,500 records: alerts, with every 7th alert acknowledged 1,300 records
    // later (so most acknowledgements sit in a newer page than their alert).
    let mut records: Vec<Record> = Vec::with_capacity(2_500);
    let mut open = 0_u32;
    let mut open_set = std::collections::BTreeSet::new();
    for i in 1..=2_500_u32 {
        let target = i.saturating_sub(1_300);
        let is_alert = |r: &Record| r.kind_of_record == TYPE_ALERT;
        if target >= 1 && target % 7 == 0 && is_alert(&records[(target - 1) as usize]) {
            open -= 1;
            open_set.remove(&target);
            records.push(raw_ack(target, open));
        } else {
            open += 1;
            open_set.insert(i);
            records.push(raw_alert(&format!("k{i}"), open));
        }
    }
    raw_queue(&scratch, &records);

    // Walk backwards a page at a time, carrying the acknowledged ordinals.
    let mut carried = std::collections::BTreeSet::new();
    let mut found = std::collections::BTreeSet::new();
    let mut before = None;
    let mut firsts = Vec::new();
    loop {
        let part = page_of(&scratch, before);
        if part.first == 0 {
            break;
        }
        firsts.push(part.first);
        for a in &part.alerts {
            if !carried.contains(&a.ordinal) {
                found.insert(a.ordinal);
            }
        }
        carried.extend(part.acked_older.iter().copied());
        before = Some(part.first);
    }
    assert_eq!(
        firsts,
        vec![1_477, 453, 1],
        "each page is the 1,024 before the last"
    );
    assert_eq!(
        found, open_set,
        "paging backwards finds exactly the open alerts"
    );

    // `older` is that walk without the newest page.
    let newest = page_of(&scratch, None);
    let in_window: std::collections::BTreeSet<u32> =
        newest.alerts.iter().map(|a| a.ordinal).collect();
    let mut want: Vec<u32> = open_set.difference(&in_window).copied().collect();
    want.reverse();
    let got: Vec<u32> = older(scratch.root())
        .unwrap()
        .iter()
        .map(|a| a.ordinal)
        .collect();
    assert_eq!(got, want);
    assert_eq!(
        newest.older_unacknowledged as usize,
        want.len(),
        "the O(1) count is exact"
    );
}

// IA-AL-15 (D-4893).
#[test]
fn every_alert_event_is_audited() {
    for (level, target, message) in EVENTS {
        assert!(
            telemetry::is_audited(level, target, message),
            "{message} at {target} would not reach the audit stream"
        );
        assert_eq!(target, "pull.alerts.lifecycle");
    }
    // What the design first wrote is NOT audited, which is why the target
    // changed: a warn at `pull.alerts` with this message rolls out of the ring.
    assert!(!telemetry::is_audited(
        telemetry::Level::Warn,
        "pull.alerts",
        "alert queue replaced"
    ));
    assert!(!telemetry::is_audited(
        telemetry::Level::Info,
        "pull.alerts",
        "alert acknowledged"
    ));
    let messages: Vec<&str> = EVENTS.iter().map(|e| e.2).collect();
    for needed in ["alert queue replaced", "alert acknowledged"] {
        assert!(messages.contains(&needed), "{needed}");
    }
}

// IA-AL-16 (RA-287, RA-317): planted links and things that are not files.
#[test]
fn a_link_or_a_directory_at_a_queue_name_is_refused_or_never_followed() {
    // A planted link at the temporary name genesis uses: nothing behind it is
    // written, and the queue is born a regular file.
    let scratch = Scratch::new("planted");
    let names = scratch.names();
    fs::create_dir_all(&names.dir).unwrap();
    let victim = scratch.root().join("victim");
    fs::write(&victim, b"not yours").unwrap();
    let tmp = names.dir.join(format!("alerts.{}.tmp", std::process::id()));
    std::os::unix::fs::symlink(&victim, &tmp).unwrap();
    std::os::unix::fs::symlink(&victim, &names.reserve).unwrap();
    put_at(scratch.root(), &alert(6, "k", "t"), NOW).unwrap();
    assert_eq!(fs::read(&victim).unwrap(), b"not yours");
    assert!(
        fs::symlink_metadata(&names.queue)
            .unwrap()
            .file_type()
            .is_file()
    );
    assert!(
        fs::symlink_metadata(&names.reserve)
            .unwrap()
            .file_type()
            .is_file()
    );
    assert!(
        fs::symlink_metadata(&tmp).is_err(),
        "the temporary name is gone"
    );

    // A link, or a directory, AT the queue's own name or its lock's: refused
    // by name by the writer and the readers; nothing behind it is read.
    for plant_dir in [false, true] {
        for at_lock in [false, true] {
            let odd = Scratch::new("odd");
            let names = odd.names();
            put_at(odd.root(), &alert(6, "k", "t"), NOW).unwrap();
            let name = if at_lock { &names.lock } else { &names.queue };
            let aside = odd.root().join("aside");
            fs::rename(name, &aside).unwrap();
            if plant_dir {
                fs::create_dir(name).unwrap();
            } else {
                std::os::unix::fs::symlink(&aside, name).unwrap();
            }
            let refusal = AlertError::NotAFile { path: name.clone() };
            assert_eq!(
                window(odd.root()),
                Err(refusal.clone()),
                "{plant_dir} {at_lock}"
            );
            assert_eq!(
                put_at(odd.root(), &alert(6, "k2", "t"), NOW),
                Err(refusal.clone())
            );
            assert_eq!(
                ack_at(odd.root(), 1, "cli", Scope::Whole, NOW),
                Err(refusal.clone())
            );
            assert_eq!(refusal.code(), "alerts_not_a_file");
        }
    }
}

// Concurrent writers: nothing lost, nothing doubled, the count exact.
#[test]
fn concurrent_puts_and_readers_lose_nothing() {
    let scratch = Scratch::new("concurrent");
    let root = scratch.root().to_path_buf();
    let writers: Vec<_> = (0..8_u32)
        .map(|w| {
            let root = root.clone();
            std::thread::spawn(move || {
                for i in 0..25_u32 {
                    // Every writer also offers one key all eight share.
                    let shared = alert(3, "build_skew:a:b", "shared");
                    put_at(&root, &shared, NOW).unwrap();
                    let key = format!("hole:{w}:{i}");
                    let new = NewAlert {
                        kind: 6,
                        key: &key,
                        day: 0,
                        run: u64::from(w),
                        text: "x",
                    };
                    assert!(matches!(
                        put_at(&root, &new, NOW).unwrap(),
                        Put::Written { .. }
                    ));
                    let seen = window(&root).unwrap();
                    assert!(
                        seen.damaged.is_empty(),
                        "a reader met a half-written record"
                    );
                }
            })
        })
        .collect();
    for writer in writers {
        writer.join().unwrap();
    }
    let page = window(scratch.root()).unwrap();
    assert_eq!(
        page.records, 201,
        "200 distinct keys and the shared one, once"
    );
    assert_eq!((page.torn, page.damaged.len()), (0, 0));
    assert_eq!(page.alerts.len(), 201);
    assert_eq!(page.alerts[0].record.open_after, 201);
    let shared = page.alerts.iter().filter(|a| a.record.kind == 3).count();
    assert_eq!(shared, 1);
    let mut ordinals: Vec<u32> = page.alerts.iter().map(|a| a.ordinal).collect();
    ordinals.sort_unstable();
    assert_eq!(ordinals, (1..=201).collect::<Vec<u32>>());
}

// IA-AL-07's race: a put waiting on the lock while a queue is replaced.
#[test]
fn a_replacement_holds_the_lock_through_its_first_alert() {
    for _ in 0..20 {
        let scratch = Scratch::new("replace-race");
        raw_queue(&scratch, &open_alerts(3));
        spoil(&scratch, 2);
        spoil(&scratch, 3);
        let root = scratch.root().to_path_buf();
        // Puts hammer the queue from another thread: refused while it is
        // damaged, written once it is replaced.
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let seen_stop = std::sync::Arc::clone(&stop);
        let hammer = std::thread::spawn(move || {
            let mut n = 0_u32;
            while !seen_stop.load(Ordering::Relaxed) {
                let key = format!("racer:{n}");
                let _either = put_at(&root, &alert(6, &key, "x"), NOW);
                let seen = window(&root).unwrap();
                assert!(
                    seen.exists,
                    "a reader was told there is no queue mid-replacement"
                );
                n += 1;
            }
        });
        let done = replace_damaged_at(scratch.root(), NOW).unwrap();
        stop.store(true, Ordering::Relaxed);
        hammer.join().unwrap();
        assert_eq!(
            done.ordinal, 1,
            "the replacement's alert is the new queue's first record"
        );
        let mut first = None;
        of_file(&scratch.names().queue, &mut |kept| {
            if first.is_none() {
                first = Some(kept);
            }
        })
        .unwrap();
        assert!(
            matches!(&first, Some(Kept::Good(1, r)) if r.kind == KIND_QUEUE_REPLACED),
            "{first:?}"
        );
    }

    // A store that cannot be written can still be read: the lock is opened
    // read-only by a reader.
    let scratch = Scratch::new("readonly");
    put_at(scratch.root(), &alert(6, "k", "t"), NOW).unwrap();
    let names = scratch.names();
    fs::set_permissions(&names.lock, fs::Permissions::from_mode(0o444)).unwrap();
    fs::set_permissions(&names.queue, fs::Permissions::from_mode(0o444)).unwrap();
    assert_eq!(window(scratch.root()).unwrap().alerts.len(), 1);
    fs::set_permissions(&names.lock, fs::Permissions::from_mode(0o644)).unwrap();
    fs::set_permissions(&names.queue, fs::Permissions::from_mode(0o644)).unwrap();
}
