//! Tests of the alert queue (IA-AL-01 to IA-AL-22; D-4883, D-4893, D-4895).
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
    // Acknowledging the damaged record is refused under its own name: the
    // queue still takes puts, so the refusal must not send the operator to a
    // replacement that would answer `alerts_not_damaged`.
    let refused = ack_at(inside.root(), 2, "cli", Scope::Window, NOW).unwrap_err();
    assert_eq!(
        refused,
        AlertError::RecordDamaged {
            ordinal: 2,
            offset: offset_of(2)
        }
    );
    assert_eq!(refused.code(), "alert_record_damaged");
    assert!(
        !refused.to_string().contains("replace-damaged"),
        "{refused}"
    );
    assert_eq!(
        replace_damaged_at(inside.root(), NOW),
        Err(AlertError::NotDamaged)
    );
    assert!(
        AlertError::Damaged { offset: 0 }
            .to_string()
            .contains("cli alerts --replace-damaged")
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
    // The cli's acknowledgement of a damaged record older than the window.
    assert_eq!(
        ack_at(long.root(), 5, "cli", Scope::Whole, NOW),
        Err(AlertError::RecordDamaged {
            ordinal: 5,
            offset: offset_of(5)
        })
    );
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

    // Three open alerts INSIDE the window: they are listed, and the older
    // count is "open after" LESS the alerts listed, so it is still exactly 1
    // (a reader that showed "open after" itself would say 4).
    for key in ["hole:a", "hole:b", "hole:c"] {
        put_at(scratch.root(), &alert(6, key, "y"), NOW).unwrap();
    }
    let page = window(scratch.root()).unwrap();
    assert_eq!(page.alerts.len(), 3);
    assert_eq!(page.alerts[0].record.open_after, 4);
    assert_eq!(page.older_unacknowledged, 1, "open after 4, less 3 listed");

    let walked = older(scratch.root()).unwrap();
    assert!(walked.exists && walked.damaged.is_empty() && walked.unknown.is_empty());
    assert_eq!(
        (walked.first, walked.records, walked.newer_damaged),
        (81, 1_104, 0)
    );
    let listed = walked.alerts;
    assert_eq!(listed.len(), 1);
    assert_eq!(
        (listed[0].ordinal, listed[0].word().as_ref()),
        (1, "runs_ledger_damaged")
    );

    ack_at(scratch.root(), 1, "cli", Scope::Whole, NOW).unwrap();
    assert_eq!(window(scratch.root()).unwrap().older_unacknowledged, 0);
    assert!(older(scratch.root()).unwrap().alerts.is_empty());
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
    // A line or paragraph separator, a bidirectional override or isolate and
    // the zero-width characters would break or reorder the alert's one line
    // on a page: each becomes a space, as a control character does.
    assert_eq!(
        fit_text("a\nb\u{2028}c\u{2029}d\u{202E}e\u{2066}f\u{200B}g\u{FEFF}h\u{061C}i\u{00AD}j"),
        ("a b c d e f g h i j".to_owned(), false)
    );
    assert_eq!(fit_text("é€😀 ok"), ("é€😀 ok".to_owned(), false));

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
    assert_eq!(older(scratch.root()), Ok(Older::default()));
    assert!(!Older::default().exists);
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

/// 2,500 records: alerts, with every 7th alert acknowledged 1,300 records
/// later (so most acknowledgements sit in a newer page than their alert).
/// Returns the ordinals left open.
fn mixed_queue(scratch: &Scratch) -> std::collections::BTreeSet<u32> {
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
    raw_queue(scratch, &records);
    open_set
}

// IA-AL-18: the page before an ordinal, as the cli shows it.
#[test]
fn a_settled_page_never_lists_an_alert_a_later_record_acknowledges() {
    let scratch = Scratch::new("settled");
    let open_set = mixed_queue(&scratch);
    let mut differed = 0_u32;
    for before in [
        2_u32,
        8,
        453,
        454,
        1_000,
        1_024,
        1_025,
        1_300,
        1_477,
        1_478,
        2_000,
        2_500,
        2_501,
        9_999,
        u32::MAX,
    ] {
        let settled = page_settled(scratch.root(), before).unwrap();
        let last = (before - 1).min(2_500);
        let first = last.saturating_sub(1_023).max(1);
        assert_eq!(settled.first, first, "before {before}");
        let want: Vec<u32> = open_set.range(first..=last).rev().copied().collect();
        let got: Vec<u32> = settled.alerts.iter().map(|a| a.ordinal).collect();
        assert_eq!(
            got, want,
            "the open alerts of {first}..={last}, newest first"
        );
        assert_eq!(
            settled.older_unacknowledged as usize,
            open_set.range(..first).count(),
            "the open alerts older than #{first} (before {before})"
        );
        assert_eq!((settled.later_damaged, settled.records), (0, 2_500));
        // What it carried on: the acknowledged ordinals below the page.
        assert!(settled.acked_older.iter().all(|k| *k < first));
        // The one read alone has not seen the later acknowledgements.
        let raw = page_of(&scratch, Some(before));
        assert!(raw.alerts.len() >= settled.alerts.len());
        differed += u32::from(raw.alerts.len() > settled.alerts.len());
    }
    assert!(
        differed >= 5,
        "the case is real: the one read lists acknowledged alerts ({differed} pages)"
    );
    // It agrees with `older` and with the newest page, page for page.
    let mut walked: Vec<u32> = Vec::new();
    let mut before = window(scratch.root()).unwrap().first;
    while before > 1 {
        let part = page_settled(scratch.root(), before).unwrap();
        // Each page starts strictly before the ordinal it was asked before,
        // so the walk ends; a page that did not walked for ever (GF-1).
        assert!(
            part.first < before,
            "the page before {before} starts at {}",
            part.first
        );
        walked.extend(part.alerts.iter().map(|a| a.ordinal));
        before = part.first;
    }
    let listed: Vec<u32> = older(scratch.root())
        .unwrap()
        .alerts
        .iter()
        .map(|a| a.ordinal)
        .collect();
    assert_eq!(walked, listed);

    // Nothing before the first record, and nothing at all without a queue.
    for before in [0, 1] {
        let none = page_settled(scratch.root(), before).unwrap();
        assert!(none.exists && none.alerts.is_empty() && none.first == 0);
    }
    let empty = Scratch::new("settled-none");
    assert_eq!(page_settled(empty.root(), 5), Ok(Page::default()));

    // An acknowledgement damaged in place can no longer be applied: the page
    // says how many later records fail, and the alert is listed again rather
    // than hidden on the word of a record nobody can read.
    let ack_of_7 = 1_307;
    let before_damage = page_settled(scratch.root(), 100).unwrap();
    assert!(before_damage.alerts.iter().all(|a| a.ordinal != 7));
    spoil(&scratch, ack_of_7);
    let after_damage = page_settled(scratch.root(), 100).unwrap();
    assert_eq!(after_damage.later_damaged, 1);
    assert!(after_damage.alerts.iter().any(|a| a.ordinal == 7));
    assert_eq!(
        after_damage.alerts.len(),
        before_damage.alerts.len() + 1,
        "only the one whose acknowledgement is unreadable"
    );

    // A single record, and a page whose every later record is a put.
    let one = Scratch::new("settled-one");
    raw_queue(&one, &open_alerts(1));
    let page = page_settled(one.root(), 2).unwrap();
    assert_eq!((page.alerts.len(), page.older_unacknowledged), (1, 0));
}

// IA-AL-14 (RA-264).
#[test]
fn an_older_page_is_one_read_at_any_depth() {
    let scratch = Scratch::new("paging");
    let open_set = mixed_queue(&scratch);

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
        // Each page starts strictly before the ordinal it was asked before,
        // so the walk ends; a page that did not walked for ever (GF-1).
        assert!(
            before.is_none_or(|asked| part.first < asked),
            "the page before {before:?} starts at {}",
            part.first
        );
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
        .alerts
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
                // The damaged queue, or the new one WITH its first record:
                // never a queue that does not yet say what it replaced.
                let replaced = seen
                    .alerts
                    .last()
                    .is_some_and(|a| a.ordinal == 1 && a.record.kind == KIND_QUEUE_REPLACED);
                assert!(
                    seen.damaged == [3, 2] || replaced,
                    "neither queue: {seen:?}"
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

/// The names in `audit/` that keep a damaged queue.
fn kept_names(scratch: &Scratch) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(scratch.names().dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("alerts.damaged-"))
        .collect();
    names.sort();
    names
}

/// Seven open alerts with the last two spoiled: a queue every put refuses.
fn damaged_queue(tag: &str) -> (Scratch, Vec<u8>) {
    let scratch = Scratch::new(tag);
    raw_queue(&scratch, &open_alerts(7));
    spoil(&scratch, 6);
    spoil(&scratch, 7);
    let bytes = fs::read(scratch.names().queue).unwrap();
    (scratch, bytes)
}

// IA-AL-19: the replacement is all or nothing, at every step it can stop.
#[test]
fn a_replacement_stopped_at_any_step_leaves_the_damaged_queue_or_the_new_one() {
    let kept_name = format!("alerts.damaged-{NOW}");
    let says = format!("kept {kept_name}; open alerts it held: 5");
    for step in 1..=5_u8 {
        let (scratch, damaged) = damaged_queue("stopped");
        let names = scratch.names();
        REPLACEMENT_STOPS_AFTER.with(|s| s.set(step));
        let stopped = replace_damaged_at(scratch.root(), NOW);
        REPLACEMENT_STOPS_AFTER.with(|s| s.set(0));
        assert!(
            matches!(
                &stopped,
                Err(AlertError::Io {
                    kind: std::io::ErrorKind::Interrupted,
                    ..
                })
            ),
            "step {step}: {stopped:?}"
        );

        // What a reader sees right after the stop: one whole queue or the
        // other, and the damaged bytes are already kept under their name.
        let seen = window(scratch.root()).unwrap();
        assert!(seen.exists, "step {step}: no queue");
        assert_eq!(kept_names(&scratch), vec![kept_name.clone()], "step {step}");
        assert_eq!(fs::read(names.dir.join(&kept_name)).unwrap(), damaged);
        let finished = step >= 4;
        if finished {
            assert_eq!((seen.records, seen.alerts.len()), (1, 1), "step {step}");
            assert_eq!(seen.alerts[0].record.kind, KIND_QUEUE_REPLACED);
            assert_eq!(seen.alerts[0].record.text, says);
            assert_eq!(seen.alerts[0].record.open_after, 1);
        } else {
            assert_eq!(fs::read(&names.queue).unwrap(), damaged, "step {step}");
            assert_eq!(seen.damaged, vec![7, 6]);
            assert!(put_at(scratch.root(), &alert(6, "k", "t"), NOW).is_err());
        }

        // The next replacement, in a later second: it finishes an unfinished
        // one under the SAME kept name (one damaged file is kept once), and
        // has nothing to do after a finished one.
        let next = replace_damaged_at(scratch.root(), NOW + 9);
        if finished {
            assert_eq!(next, Err(AlertError::NotDamaged), "step {step}");
        } else {
            assert_eq!(
                next,
                Ok(Replaced {
                    kept: names.dir.join(&kept_name),
                    open: Some(5),
                    ordinal: 1
                }),
                "step {step}"
            );
        }
        assert_eq!(kept_names(&scratch), vec![kept_name.clone()], "step {step}");
        assert_eq!(fs::read(names.dir.join(&kept_name)).unwrap(), damaged);
        let page = window(scratch.root()).unwrap();
        assert_eq!((page.records, page.alerts.len()), (1, 1), "step {step}");
        assert_eq!(page.alerts[0].record.text, says);
        // The new queue takes puts, numbered after its first record, and the
        // reserve is in place whichever step the stop took.
        assert_eq!(
            written(put_at(scratch.root(), &alert(6, "k", "t"), NOW).unwrap()),
            2
        );
        assert_eq!(
            fs::metadata(&names.reserve).unwrap().len(),
            RESERVE_RECORDS * RECORD_LEN
        );
    }

    // Without the seam: the host refuses the new queue's temporary name (a
    // directory sits there). The damaged queue must still be the queue; a
    // replacement that retired it first would leave a reader with none.
    let (scratch, damaged) = damaged_queue("refused");
    let names = scratch.names();
    let tmp = names.dir.join(format!("alerts.{}.tmp", std::process::id()));
    fs::create_dir(&tmp).unwrap();
    let refused = replace_damaged_at(scratch.root(), NOW);
    assert!(
        matches!(
            &refused,
            Err(AlertError::Io {
                op: "write the new queue",
                ..
            })
        ),
        "{refused:?}"
    );
    let seen = window(scratch.root()).unwrap();
    assert!(
        seen.exists,
        "the damaged queue left its name with nothing in its place"
    );
    assert_eq!(seen.damaged, vec![7, 6]);
    assert_eq!(fs::read(&names.queue).unwrap(), damaged);
    fs::remove_dir(&tmp).unwrap();
    let done = replace_damaged_at(scratch.root(), NOW + 1).unwrap();
    assert_eq!(done.kept, names.dir.join(&kept_name));
    assert_eq!(kept_names(&scratch), vec![kept_name.clone()]);
    assert_eq!(window(scratch.root()).unwrap().alerts[0].record.text, says);

    // A second link that is NOT a kept name (someone linked the queue
    // elsewhere) is not mistaken for one.
    let (scratch, _damaged) = damaged_queue("linked");
    let names = scratch.names();
    fs::hard_link(&names.queue, names.dir.join("alerts.copy")).unwrap();
    let done = replace_damaged_at(scratch.root(), NOW).unwrap();
    assert_eq!(done.kept, names.dir.join(&kept_name));

    // A failed header does not hide the records behind it: the open count is
    // read from them, and is unknown only when none passes its CRC.
    let scratch = Scratch::new("header");
    raw_queue(&scratch, &open_alerts(5));
    let queue = OpenOptions::new()
        .write(true)
        .open(scratch.names().queue)
        .unwrap();
    queue.write_all_at(&[0xEE], 20).unwrap();
    assert_eq!(
        window(scratch.root()),
        Err(AlertError::Damaged { offset: 0 })
    );
    let done = replace_damaged_at(scratch.root(), NOW).unwrap();
    assert_eq!(done.open, Some(5));
    assert!(
        window(scratch.root()).unwrap().alerts[0]
            .record
            .text
            .ends_with("open alerts it held: 5")
    );
}

// IA-AL-20: a key of any length is put once, and an event carries a bounded
// part of it.
#[test]
fn a_long_key_is_hashed_whole_and_cut_in_its_events() {
    let scratch = Scratch::new("longkey");
    let long = "k".repeat(100_000);
    let other = format!("{}j", &long[..99_999]);
    assert_eq!(
        written(put_at(scratch.root(), &alert(6, &long, "t"), NOW).unwrap()),
        1
    );
    assert_eq!(
        put_at(scratch.root(), &alert(6, &long, "t"), NOW),
        Ok(Put::Duplicate { ordinal: 1 })
    );
    assert_eq!(
        written(put_at(scratch.root(), &alert(6, &other, "t"), NOW).unwrap()),
        2,
        "the last byte of a long key still tells two alerts apart"
    );
    assert_eq!(event_key(&long).len(), EVENT_KEY_MAX);
    assert_eq!(event_key("short"), "short");
    assert_eq!(event_key(""), "");
    // Never part of a character: 100 three-byte characters cut at 159 bytes.
    let wide = "€".repeat(100);
    assert_eq!(event_key(&wide).len(), 159);
    assert!(wide.starts_with(event_key(&wide)));
    assert!(matches!(key_bytes(&long), telemetry::Value::Uint(100_000)));
}

/// The ordinals of a list of alerts.
fn ordinals(alerts: &[Alert]) -> Vec<u32> {
    alerts.iter().map(|a| a.ordinal).collect()
}

// IA-AL-21: the walk names every record it could not read or name.
#[test]
fn the_older_walk_names_every_record_it_cannot_read() {
    // A damaged OPEN ALERT older than the window: the O(1) count includes it,
    // the walk cannot list it and says which record it is.
    let scratch = Scratch::new("older-damaged");
    raw_queue(&scratch, &open_alerts(1_200));
    spoil(&scratch, 50);
    let newest = window(scratch.root()).unwrap();
    assert_eq!(
        (newest.alerts.len(), newest.older_unacknowledged),
        (1_024, 176)
    );
    assert!(newest.damaged.is_empty(), "the window itself is whole");
    let walked = older(scratch.root()).unwrap();
    assert_eq!(walked.alerts.len(), 175);
    assert_eq!(walked.damaged, vec![50], "named, not skipped");
    assert!(walked.alerts.iter().all(|a| a.ordinal != 50));
    assert_eq!(
        walked.alerts.len() + walked.damaged.len(),
        newest.older_unacknowledged as usize,
        "the count, the list and the damage agree"
    );
    assert_eq!(
        (
            walked.exists,
            walked.first,
            walked.records,
            walked.newer_damaged
        ),
        (true, 177, 1_200, 0)
    );
    assert!(walked.unknown.is_empty());
    // The page that holds it says the same.
    let page = page_settled(scratch.root(), 177).unwrap();
    assert_eq!(page.damaged, walked.damaged);
    assert_eq!(ordinals(&page.alerts), ordinals(&walked.alerts));

    // A damaged ACKNOWLEDGEMENT older than the window, beside a later build's
    // record: 60 alerts, #61 of a type this build cannot name, #62 the
    // acknowledgement of #5, then alerts up to #1300.
    let scratch = Scratch::new("older-ack");
    let mut records = open_alerts(60);
    let mut later_type = raw_alert("later", 60);
    later_type.kind_of_record = 3;
    records.push(later_type);
    records.push(raw_ack(5, 59));
    for i in 63..=1_300_u32 {
        records.push(raw_alert(&format!("k{i}"), i - 3));
    }
    raw_queue(&scratch, &records);
    let newest = window(scratch.root()).unwrap();
    assert_eq!((newest.first, newest.older_unacknowledged), (277, 273));
    let whole = older(scratch.root()).unwrap();
    assert_eq!(whole.alerts.len(), 273);
    assert!(whole.alerts.iter().all(|a| a.ordinal != 5), "acknowledged");
    assert_eq!(whole.unknown, vec![(61, 3)], "a later build's, shown");
    assert!(whole.damaged.is_empty());
    spoil(&scratch, 62);
    let walked = older(scratch.root()).unwrap();
    assert_eq!(walked.damaged, vec![62]);
    assert_eq!(walked.unknown, vec![(61, 3)]);
    assert_eq!(
        walked.alerts.len(),
        274,
        "its alert is listed again, not hidden on the word of a record nobody can read"
    );
    assert!(walked.alerts.iter().any(|a| a.ordinal == 5));
    let page = page_settled(scratch.root(), 277).unwrap();
    assert_eq!((page.damaged, page.unknown), (vec![62], vec![(61, 3)]));
    assert_eq!(ordinals(&page.alerts), ordinals(&walked.alerts));

    // Several, in more than one older page: newest first, none lost.
    let scratch = Scratch::new("older-many");
    raw_queue(&scratch, &open_alerts(3_100));
    for k in [1_u64, 2, 52, 1_052, 1_053, 2_076] {
        spoil(&scratch, k);
    }
    let walked = older(scratch.root()).unwrap();
    assert_eq!(walked.damaged, vec![2_076, 1_053, 1_052, 52, 2, 1]);
    assert_eq!(walked.alerts.len(), 2_076 - 6);
    assert_eq!((walked.first, walked.newer_damaged), (2_077, 0));

    // Damage INSIDE the newest window: an acknowledgement of an older alert
    // there cannot be applied, and the walk says how many such records.
    let scratch = Scratch::new("older-newer");
    let mut records = open_alerts(1_100);
    records.push(raw_ack(3, 1_099));
    raw_queue(&scratch, &records);
    let whole = older(scratch.root()).unwrap();
    assert_eq!((whole.alerts.len(), whole.newer_damaged), (76, 0));
    assert!(whole.alerts.iter().all(|a| a.ordinal != 3));
    spoil(&scratch, 1_101);
    let walked = older(scratch.root()).unwrap();
    assert_eq!(walked.newer_damaged, 1);
    assert!(walked.damaged.is_empty(), "nothing older is damaged");
    assert_eq!(walked.alerts.len(), 77);
    assert!(walked.alerts.iter().any(|a| a.ordinal == 3));
    assert_eq!(window(scratch.root()).unwrap().damaged, vec![1_101]);

    // The edges: a header alone, one record, exactly a window, one more.
    let scratch = Scratch::new("older-edges");
    raw_queue(&scratch, &[]);
    let none = older(scratch.root()).unwrap();
    assert!(none.exists && none.alerts.is_empty() && none.damaged.is_empty());
    assert_eq!((none.first, none.records), (0, 0));
    raw_queue(&scratch, &open_alerts(1));
    assert_eq!(older(scratch.root()).unwrap().first, 1);
    raw_queue(&scratch, &open_alerts(1_024));
    spoil(&scratch, 1);
    let exact = older(scratch.root()).unwrap();
    assert!(exact.alerts.is_empty() && exact.damaged.is_empty());
    assert_eq!((exact.first, exact.newer_damaged), (1, 1));
    raw_queue(&scratch, &open_alerts(1_025));
    spoil(&scratch, 1);
    let one_more = older(scratch.root()).unwrap();
    assert!(one_more.alerts.is_empty());
    assert_eq!((one_more.damaged, one_more.first), (vec![1], 2));

    // An acknowledgement that names no record the writer could have meant
    // (ordinal 0, or one that does not exist yet) takes nothing off a count.
    let scratch = Scratch::new("older-crafted");
    let mut records = open_alerts(1_030);
    records.push(raw_ack(0, 1_030));
    records.push(raw_ack(4_000_000, 1_030));
    records.push(raw_ack(u32::MAX, 1_030));
    raw_queue(&scratch, &records);
    let newest = window(scratch.root()).unwrap();
    assert!(newest.acked_older.is_empty(), "{:?}", newest.acked_older);
    assert_eq!(older(scratch.root()).unwrap().alerts.len(), 9);
    let page = page_settled(scratch.root(), 8).unwrap();
    assert_eq!((page.alerts.len(), page.older_unacknowledged), (7, 0));
    let page = page_settled(scratch.root(), 1_031).unwrap();
    assert_eq!(
        (page.alerts.len(), page.older_unacknowledged),
        (1_024, 6),
        "three crafted acknowledgements lower no older count"
    );
    assert!(page.acked_older.is_empty());
}

/// GF-1: `last + 1` mutated to `last * 1` in `page_settled` survived. The
/// walk after the page then starts AT the page's own newest record, and
/// when that record acknowledges an alert older than the page, the
/// acknowledgement the page already applied is carried a second time:
/// one more alert off the older count, and the ordinal listed twice.
#[test]
fn a_pages_own_newest_acknowledgement_of_an_older_alert_counts_once() {
    let scratch = Scratch::new("GF1-SETTLED-OWN-ACK");
    let mut records = open_alerts(1_029);
    // Ordinal 1_030 acknowledges alert 3, older than the page of the 1_024
    // records before 1_031 (ordinals 7 ..= 1_030): 1_028 left open.
    records.push(raw_ack(3, 1_028));
    raw_queue(&scratch, &records);
    let page = page_settled(scratch.root(), 1_031).unwrap();
    assert_eq!(page.first, 7);
    assert_eq!(page.alerts.len(), 1_023, "alerts 7 ..= 1_029, all open");
    assert_eq!(
        page.older_unacknowledged, 5,
        "alerts 1, 2, 4, 5 and 6: the acknowledged 3 is taken off once"
    );
    assert_eq!(page.acked_older, vec![3], "named once");
    // The same acknowledgement met AFTER a page is carried onto it, once.
    let before = page_settled(scratch.root(), 1_030).unwrap();
    assert_eq!((before.first, before.alerts.len()), (6, 1_024));
    assert_eq!(before.older_unacknowledged, 4, "alerts 1, 2, 4 and 5");
    assert_eq!(before.acked_older, vec![3]);
}

/// GF-1: the same `last + 1` to `last * 1` survived again once the walk
/// counted its reads up front, because the walk's length no longer reads
/// `from` and the test above has no record after its page. With records
/// after the page, the mutated first read starts at the page's own newest
/// record, so a damaged newest record of the page is counted as a damaged
/// LATER record, which it is not.
#[test]
fn a_damaged_newest_record_of_the_page_is_not_a_later_damaged_record() {
    let scratch = Scratch::new("GF1-SETTLED-OWN-DAMAGE");
    raw_queue(&scratch, &open_alerts(150));
    spoil(&scratch, 99);
    let page = page_settled(scratch.root(), 100).unwrap();
    assert_eq!(page.first, 1, "the page is records 1 ..= 99");
    assert_eq!(page.later_damaged, 0, "records 100 ..= 150 are all whole");
    // The other side of the line: a damaged record just after the page is.
    spoil(&scratch, 100);
    assert_eq!(page_settled(scratch.root(), 100).unwrap().later_damaged, 1);
}

/// Arms the walk seam: `step` runs once before this thread's next later read.
fn between_reads(step: impl FnOnce(&Path) + 'static) {
    BETWEEN_READS.with(|held| *held.borrow_mut() = Some(Box::new(step)));
}

fn seam_is_spent() -> bool {
    BETWEEN_READS.with(|held| held.borrow().is_none())
}

// IA-AL-22: a walk is one view of one queue, whatever lands between its reads.
#[test]
fn a_walk_is_one_queue_across_a_replacement() {
    // 3,000 open alerts, the last two records bad: the put refuses the queue,
    // the readers still list it, and the operator may replace it.
    let build = |tag: &str| {
        let scratch = Scratch::new(tag);
        raw_queue(&scratch, &open_alerts(3_000));
        spoil(&scratch, 2_999);
        spoil(&scratch, 3_000);
        scratch
    };

    // `older`: the replacement lands after the first read and before the
    // second. The listing is the replaced queue's, whole.
    let scratch = build("walk-older");
    let whole = older(scratch.root()).unwrap();
    assert_eq!((whole.alerts.len(), whole.newer_damaged), (1_976, 2));
    between_reads(|root| {
        let done = replace_damaged_at(root, NOW + 1).unwrap();
        assert_eq!(done.ordinal, 1);
        // And the new queue fills at once, past where the walk reads next.
        for i in 0..5 {
            put_at(root, &alert(6, &format!("after-{i}"), "t"), NOW + 2).unwrap();
        }
    });
    let across = older(scratch.root()).unwrap();
    assert!(seam_is_spent(), "the replacement ran inside the walk");
    assert_eq!(across, whole, "one queue: the one the first read opened");
    assert!(
        across
            .alerts
            .iter()
            .all(|a| a.record.kind != KIND_QUEUE_REPLACED),
        "no record of the new queue is in the old one's listing"
    );
    // The next call is the new queue's, whole as well.
    let after = older(scratch.root()).unwrap();
    assert_eq!((after.records, after.first, after.alerts.len()), (6, 1, 0));
    let newest = window(scratch.root()).unwrap();
    assert_eq!(newest.alerts.len(), 6);
    assert_eq!(
        newest.alerts.last().unwrap().record.kind,
        KIND_QUEUE_REPLACED
    );

    // `page_settled`: the same landing between the page and the records
    // after it. No refusal and no mixed page.
    let scratch = build("walk-page");
    let whole = page_settled(scratch.root(), 101).unwrap();
    assert_eq!(
        (whole.alerts.len(), whole.later_damaged, whole.records),
        (100, 2, 3_000)
    );
    between_reads(|root| {
        replace_damaged_at(root, NOW + 1).unwrap();
        for i in 0..200 {
            put_at(root, &alert(6, &format!("after-{i}"), "t"), NOW + 2).unwrap();
        }
    });
    let across = page_settled(scratch.root(), 101).unwrap();
    assert!(seam_is_spent());
    assert_eq!(across, whole);
    let after = page_settled(scratch.root(), 101).unwrap();
    assert_eq!((after.records, after.alerts.len()), (201, 100));
    assert_eq!(
        after.alerts.last().unwrap().record.kind,
        KIND_QUEUE_REPLACED
    );

    // A put and an acknowledgement that land mid-walk on a healthy queue:
    // the answer is the queue as the first read counted it.
    let scratch = Scratch::new("walk-put");
    raw_queue(&scratch, &open_alerts(2_500));
    let whole = older(scratch.root()).unwrap();
    assert_eq!(whole.alerts.len(), 1_476);
    between_reads(|root| {
        put_at(root, &alert(6, "late", "t"), NOW + 1).unwrap();
        ack_at(root, 7, "cli", Scope::Whole, NOW + 1).unwrap();
    });
    assert_eq!(older(scratch.root()).unwrap(), whole);
    assert!(seam_is_spent());
    let later = older(scratch.root()).unwrap();
    assert_eq!((later.records, later.alerts.len()), (2_502, 1_477));
    assert!(later.alerts.iter().all(|a| a.ordinal != 7));
    let settled = page_settled(scratch.root(), 101).unwrap();
    between_reads(|root| {
        ack_at(root, 9, "cli", Scope::Whole, NOW + 2).unwrap();
    });
    assert_eq!(page_settled(scratch.root(), 101).unwrap(), settled);
    assert_eq!(
        page_settled(scratch.root(), 101).unwrap().alerts.len(),
        settled.alerts.len() - 1
    );

    // The one record that is not append-only: a bad LAST record, which the
    // next put writes over. A later read meets the bad record or the whole
    // new one, under the lock, never a half.
    let scratch = Scratch::new("walk-over");
    raw_queue(&scratch, &open_alerts(1_500));
    spoil(&scratch, 1_500);
    assert_eq!(page_settled(scratch.root(), 101).unwrap().later_damaged, 1);
    between_reads(|root| {
        let put = put_at(root, &alert(6, "over", "t"), NOW + 1).unwrap();
        assert_eq!(written(put), 1_500, "written over the bad last record");
    });
    let across = page_settled(scratch.root(), 101).unwrap();
    assert_eq!((across.later_damaged, across.records), (0, 1_500));

    // One read needs no later read, so the seam stays armed and the page is
    // the one read's.
    between_reads(|_| panic!("a single read has no second read"));
    assert_eq!(window(scratch.root()).unwrap().records, 1_500);
    assert_eq!(page_of(&scratch, Some(101)).alerts.len(), 100);
    assert!(!seam_is_spent());
    BETWEEN_READS.with(|held| held.borrow_mut().take());
}

/// p50, p99 and max of a set of timings, in microseconds.
fn spread(mut micros: Vec<u128>) -> (u128, u128, u128) {
    micros.sort_unstable();
    let at = |q: usize| micros[(micros.len() * q / 100).min(micros.len() - 1)];
    (at(50), at(99), micros[micros.len() - 1])
}

// docs/06 L-IA-34, measured: the operator's two steps this module bounds by
// the records after a page, and by nothing. Ignored because it times a drive;
// it still asserts what each step returned.
#[test]
#[ignore = "timing: prints p50, p99 and max; run by hand with --ignored --nocapture"]
fn the_operators_older_steps_timed() {
    let scratch = Scratch::new("timed");
    let records = 102_400_u32;
    raw_queue(&scratch, &open_alerts(records));
    for (label, before, later) in [
        ("1,024 records after the page", records - 1_023, 1_024_u32),
        ("51,201 records after the page", 51_200, 51_201),
        ("101,376 records after the page", 1_025, 101_376),
    ] {
        let mut micros = Vec::with_capacity(200);
        for _ in 0..200 {
            let start = std::time::Instant::now();
            let page = page_settled(scratch.root(), before).unwrap();
            micros.push(start.elapsed().as_micros());
            assert_eq!(page.alerts.len(), 1_024);
            assert_eq!(page.older_unacknowledged, before - 1 - 1_024);
            assert_eq!(u64::from(later), page.records - u64::from(before) + 1);
        }
        let (p50, p99, max) = spread(micros);
        println!("page_settled, {label}: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");
    }
    let mut one_read = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = std::time::Instant::now();
        let page = page_of(&scratch, Some(51_200));
        one_read.push(start.elapsed().as_micros());
        assert_eq!(page.alerts.len(), 1_024);
    }
    let (p50, p99, max) = spread(one_read);
    println!(
        "page (one read) at the same depth: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)"
    );

    // The whole walk: every page before the newest, one open file, the lock
    // taken shared for each read.
    let mut walk = Vec::with_capacity(200);
    for _ in 0..200 {
        let start = std::time::Instant::now();
        let found = older(scratch.root()).unwrap();
        walk.push(start.elapsed().as_micros());
        assert_eq!(found.alerts.len(), 101_376);
        assert!(found.damaged.is_empty() && found.newer_damaged == 0);
    }
    let (p50, p99, max) = spread(walk);
    println!("older, 102,400 records: p50 {p50} us, p99 {p99} us, max {max} us (n = 200)");

    // A put while walks run back to back on another thread: what a put waits
    // for is one read, not a walk.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let walker = {
        let stop = std::sync::Arc::clone(&stop);
        let root = scratch.root().to_path_buf();
        std::thread::spawn(move || {
            let mut walks = 0_u32;
            while !stop.load(Ordering::Relaxed) {
                assert!(older(&root).unwrap().alerts.len() >= 101_376);
                walks += 1;
            }
            walks
        })
    };
    let mut puts = Vec::with_capacity(200);
    for i in 0..200_u32 {
        let key = format!("timed-put-{i}");
        let start = std::time::Instant::now();
        let put = put_at(scratch.root(), &alert(6, &key, "t"), NOW).unwrap();
        puts.push(start.elapsed().as_micros());
        assert_eq!(written(put), records + 1 + i);
    }
    stop.store(true, Ordering::Relaxed);
    let walks = walker.join().unwrap();
    let (p50, p99, max) = spread(puts);
    println!(
        "put while {walks} walks of 102,400 records ran: p50 {p50} us, p99 {p99} us, max {max} us \
         (n = 200)"
    );

    // The replacement, on a small and on a long damaged queue.
    for size in [7_u32, 102_400] {
        let n: u64 = if size == 7 { 100 } else { 30 };
        let mut micros = Vec::with_capacity(100);
        for i in 0..n {
            let scratch = Scratch::new("timed-replace");
            raw_queue(&scratch, &open_alerts(size));
            spoil(&scratch, u64::from(size) - 1);
            spoil(&scratch, u64::from(size));
            let start = std::time::Instant::now();
            let done = replace_damaged_at(scratch.root(), NOW + i).unwrap();
            micros.push(start.elapsed().as_micros());
            assert_eq!((done.open, done.ordinal), (Some(size - 2), 1));
        }
        let (p50, p99, max) = spread(micros);
        println!(
            "replace_damaged, {size} records: p50 {p50} us, p99 {p99} us, max {max} us (n = {n})"
        );
    }
}

// GF-1: the mutant `present || !create` in `open_lock` survived the round's
// mutation run. A queue whose lock file has gone (a store copied without it,
// or the name removed by hand) is the case it changes: a reader and an
// acknowledgement still take the lock by making it, so the alerts stay
// visible and acknowledgeable. The mutant opened the absent name read-only,
// which a reader took for "no queue": every alert vanished from the page.
#[test]
fn a_queue_whose_lock_file_is_gone_is_still_read_and_acknowledged() {
    let scratch = Scratch::new("GF1-LOCK-GONE");
    for key in ["a", "b"] {
        put_at(scratch.root(), &alert(9, key, key), NOW).unwrap();
    }
    let names = scratch.names();
    fs::remove_file(&names.lock).unwrap();
    assert!(names.queue.is_file() && !names.lock.exists(), "the premise");

    let page = window(scratch.root()).unwrap();
    let listed: Vec<u32> = page.alerts.iter().map(|a| a.ordinal).collect();
    assert_eq!(listed, vec![2, 1], "a missing lock is not a missing queue");
    assert!(names.lock.is_file(), "the reader made the lock it needed");
    assert!(
        older(scratch.root()).unwrap().exists,
        "and the older walk sees it"
    );

    fs::remove_file(&names.lock).unwrap();
    let acked = ack_at(scratch.root(), 1, "cli", Scope::Whole, NOW + 1).unwrap();
    assert_eq!(acked.ordinal, 1, "{acked:?}");
    assert!(names.lock.is_file(), "the acknowledgement made it too");
    let page = window(scratch.root()).unwrap();
    let listed: Vec<u32> = page.alerts.iter().map(|a| a.ordinal).collect();
    assert_eq!(listed, vec![2], "the acknowledged alert is hidden");
}

// GF-1: `unix_now` replaced by `0` survived; nothing read the clock it gives.
#[test]
fn unix_now_is_the_clock_in_whole_seconds() {
    let secs = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    };
    let before = secs();
    let now = unix_now();
    let after = secs();
    assert!(
        before <= now && now <= after,
        "{before} <= {now} <= {after}"
    );
    assert!(
        now > 1_767_225_600,
        "this build runs after 2026-01-01: {now}"
    );
}

// GF-1: `||` mutated to `&&` in `shape`'s length check survived. A header
// whose CRC passes but which names a record length other than 128, or a
// header length other than 64, is damaged, each on its own.
#[test]
fn a_header_that_passes_its_crc_with_a_wrong_record_or_header_length_is_damaged() {
    let scratch = Scratch::new("GF1-LENGTHS");
    put_at(scratch.root(), &alert(6, "k", "t"), NOW).unwrap();
    for (at, value) in [(10_usize, 64_u16), (12, 128)] {
        let mut head = header(NOW);
        head[at..at + 2].copy_from_slice(&value.to_le_bytes());
        let crc = crc32c(&head[..60]).to_le_bytes();
        head[60..64].copy_from_slice(&crc);
        fs::write(scratch.names().queue, &head).unwrap();
        assert_eq!(
            window(scratch.root()),
            Err(AlertError::Damaged { offset: 0 }),
            "the field at byte {at} reads {value}"
        );
    }
}

// GF-1: `&&` mutated to `||` in `damaged_open_count` survived. ONE bad record
// at the end is a torn append the next put writes over, and one bad record
// before a good last one is not the two-bad tail either: neither queue is
// damaged, so neither is replaced.
#[test]
fn one_bad_record_at_or_before_the_tail_is_not_a_damaged_queue() {
    for spoiled in [5_u64, 4] {
        let scratch = Scratch::new(&format!("onebad{spoiled}"));
        raw_queue(&scratch, &open_alerts(5));
        spoil(&scratch, spoiled);
        assert_eq!(
            replace_damaged_at(scratch.root(), NOW),
            Err(AlertError::NotDamaged),
            "record {spoiled} alone fails its CRC"
        );
        assert_eq!(scratch.len(), 64 + 128 * 5, "and nothing was rewritten");
    }
}

// GF-1: `&&` mutated to `||` in `keep` survived. A replacement stopped after
// its keep step leaves the queue with a second name; the next one must keep
// THAT name, found by device and inode, and never another `alerts.damaged-*`
// file that merely sorts first. Sixteen unrelated earlier names make the
// directory order unable to hide the difference.
#[test]
fn a_stopped_replacement_keeps_the_queues_own_second_name_and_no_other() {
    let scratch = Scratch::new("GF1-KEEP-OWN");
    let names = scratch.names();
    raw_queue(&scratch, &open_alerts(5));
    spoil(&scratch, 4);
    spoil(&scratch, 5);
    let damaged = fs::read(&names.queue).unwrap();
    for n in 0..16 {
        fs::write(
            names.dir.join(format!("alerts.damaged-0{n:02}")),
            b"an unrelated earlier file",
        )
        .unwrap();
    }
    let own = names.dir.join(format!("alerts.damaged-{NOW}"));
    fs::hard_link(&names.queue, &own).unwrap();
    // Two earlier stopped replacements in the same second left `.2` to `.9`
    // as well (`path < kept` mutated to `==` survived): the first name is kept.
    for n in 2..10 {
        fs::hard_link(
            &names.queue,
            names.dir.join(format!("alerts.damaged-{NOW}.{n}")),
        )
        .unwrap();
    }

    let done = replace_damaged_at(scratch.root(), NOW + 60).unwrap();
    assert_eq!(done.kept, own, "the queue's own second name is kept");
    assert_eq!(fs::read(&own).unwrap(), damaged, "kept byte for byte");
    for n in 0..16 {
        assert_eq!(
            fs::read(names.dir.join(format!("alerts.damaged-0{n:02}"))).unwrap(),
            b"an unrelated earlier file"
        );
    }
    assert!(
        !names
            .dir
            .join(format!("alerts.damaged-{}", NOW + 60))
            .exists(),
        "and no second copy of the same damaged file"
    );
}

// GF-1: the guard `why.kind() == AlreadyExists` mutated to `true` in
// `create_fresh` survived. Only a name that is already there is removed and
// made again; any other refusal is the host's own, returned as it came. A
// directory this process may not write gives PermissionDenied, which the
// mutant turned into the NotFound of removing a name that was never there.
#[test]
fn create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it() {
    let scratch = Scratch::new("fresh");
    let shut = scratch.root().join("SHUT");
    fs::create_dir_all(&shut).unwrap();
    fs::set_permissions(&shut, fs::Permissions::from_mode(0o555)).unwrap();
    let refused = create_fresh(&shut.join("x")).map(drop);
    fs::set_permissions(&shut, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(
        refused.map_err(|why| why.kind()),
        Err(io::ErrorKind::PermissionDenied)
    );
    // And a stale name is replaced by a new, empty file.
    let stale = scratch.root().join("stale");
    fs::write(&stale, b"left by a killed run").unwrap();
    drop(create_fresh(&stale).unwrap());
    assert_eq!(fs::metadata(&stale).unwrap().len(), 0);
}

// GF-1: four mutants of `hold_reserve`'s guard survived: `&&` to `||`, the
// whole guard to `false`, and `RESERVE_RECORDS * RECORD_LEN` to `+` and `/`.
// A whole reserve is held and never written again (its inode stays); a
// reserve file of 1,000 bytes, longer than 512 + 128 and shorter than its
// 64 KiB, is not held: the next put writes it whole.
#[test]
fn a_whole_reserve_is_kept_and_a_short_one_is_written_whole_by_the_next_put() {
    use std::os::unix::fs::MetadataExt as _;
    let scratch = Scratch::new("GF1-SHORT-RESERVE");
    let names = scratch.names();
    put_at(scratch.root(), &alert(6, "a", "t"), NOW).unwrap();
    let made = fs::metadata(&names.reserve).unwrap().ino();
    put_at(scratch.root(), &alert(6, "b", "t"), NOW + 1).unwrap();
    assert_eq!(
        fs::metadata(&names.reserve).unwrap().ino(),
        made,
        "a whole reserve is held, not written again"
    );
    fs::write(&names.reserve, vec![0_u8; 1_000]).unwrap();
    put_at(scratch.root(), &alert(6, "c", "t"), NOW + 2).unwrap();
    assert_eq!(
        fs::metadata(&names.reserve).unwrap().len(),
        RESERVE_RECORDS * RECORD_LEN
    );
}

// GF-1: `+` mutated to `-` in `page_settled`'s span count survived, because
// the one test whose loop reaches a short tail is ignored as slow. A page
// with two newer records, the newest acknowledging an alert on the page,
// must read both: the acknowledged alert leaves the page.
#[test]
fn a_page_reads_every_newer_record_to_its_last() {
    let scratch = Scratch::new("GF1-SETTLED-TAIL");
    for key in ["a", "b", "c"] {
        put_at(scratch.root(), &alert(9, key, key), NOW).unwrap();
    }
    ack_at(scratch.root(), 1, "cli", Scope::Whole, NOW + 1).unwrap();
    let page = page_settled(scratch.root(), 3).unwrap();
    let listed: Vec<u32> = page.alerts.iter().map(|a| a.ordinal).collect();
    assert_eq!(listed, vec![2], "#1 was acknowledged by the newest record");
    assert_eq!(page.records, 4);
}

// GF-1: `||` mutated to `&&` in `ack_at`'s range check survived. Ordinal 0
// and an ordinal past the last record are each no such alert, on a queue
// that exists.
#[test]
fn an_acknowledgement_of_ordinal_zero_or_past_the_end_names_no_such_alert() {
    let scratch = Scratch::new("GF1-ACK-RANGE");
    for key in ["a", "b"] {
        put_at(scratch.root(), &alert(9, key, key), NOW).unwrap();
    }
    for ordinal in [0_u32, 3, u32::MAX] {
        assert_eq!(
            ack_at(scratch.root(), ordinal, "cli", Scope::Whole, NOW),
            Err(AlertError::NoSuchAlert { ordinal })
        );
    }
    assert_eq!(scratch.len(), 64 + 128 * 2, "nothing was written");
}

// GF-1: `<` mutated to `<=` in `listed` survived. An acknowledgement of the
// window's own first record is applied inside the window, never carried as
// an older acknowledgement.
#[test]
fn an_acknowledgement_of_the_windows_first_record_is_not_an_older_one() {
    let scratch = Scratch::new("GF1-ACK-FIRST");
    for key in ["a", "b", "c"] {
        put_at(scratch.root(), &alert(9, key, key), NOW).unwrap();
    }
    ack_at(scratch.root(), 1, "cli", Scope::Whole, NOW + 1).unwrap();
    let page = window(scratch.root()).unwrap();
    assert_eq!(page.first, 1);
    assert!(page.acked_older.is_empty(), "{:?}", page.acked_older);
    let listed: Vec<u32> = page.alerts.iter().map(|a| a.ordinal).collect();
    assert_eq!(listed, vec![3, 2]);
}

// GF-1: `-` mutated to `+` in `of_file` survived: no test read a file past
// one window. 1,100 records are read in two spans, every one of them.
#[test]
fn a_file_longer_than_one_window_is_read_whole() {
    let scratch = Scratch::new("GF1-OF-FILE");
    raw_queue(&scratch, &open_alerts(1_100));
    let mut seen = 0_u32;
    let shape = of_file(&scratch.names().queue, &mut |k| {
        assert!(matches!(k, Kept::Good(..)), "{k:?}");
        seen += 1;
    })
    .unwrap();
    assert_eq!(seen, 1_100);
    assert_eq!(shape.records, 1_100);
}

// GF-1: the guard `AlreadyExists && attempt < 64` in `keep` mutated to
// `true` and to `||` survived. Sixty-four taken names are the bound: the
// replacement is refused by name, the queue is left as it was, and no 65th
// name is made.
#[test]
fn a_damaged_queue_with_every_kept_name_taken_is_refused_not_kept_under_a_65th() {
    let scratch = Scratch::new("GF1-KEEP-64");
    let names = scratch.names();
    raw_queue(&scratch, &open_alerts(5));
    spoil(&scratch, 4);
    spoil(&scratch, 5);
    let damaged = fs::read(&names.queue).unwrap();
    fs::write(names.dir.join(format!("alerts.damaged-{NOW}")), b"TAKEN").unwrap();
    for n in 2..=64 {
        fs::write(
            names.dir.join(format!("alerts.damaged-{NOW}.{n}")),
            b"TAKEN",
        )
        .unwrap();
    }
    let refused = replace_damaged_at(scratch.root(), NOW);
    assert!(
        matches!(&refused, Err(AlertError::Io { op, .. }) if *op == "keep the damaged queue"),
        "{refused:?}"
    );
    assert!(!names.dir.join(format!("alerts.damaged-{NOW}.65")).exists());
    assert_eq!(fs::read(&names.queue).unwrap(), damaged);
}

// GF-1: the guard `kind == NotFound` in `regular` mutated to `true`
// survived. A stat that fails for any other reason is that failure, never
// "absent": with `audit` a regular file, `audit/alerts.lock` cannot be
// looked at, and the reader says so instead of answering "no queue".
#[test]
fn a_stat_that_fails_other_than_not_found_is_refused_never_read_as_absent() {
    let scratch = Scratch::new("GF1-STAT-FAIL");
    let names = scratch.names();
    fs::write(&names.dir, b"a file where audit/ belongs").unwrap();
    let read = window(scratch.root());
    assert!(
        matches!(&read, Err(AlertError::Io { op: "stat", .. })),
        "{read:?}"
    );
}

// GF-1: deleting either `!` of `!present && !create` in `open_lock`
// survived, because every "a read never writes" test had no `audit/` at all
// and the lock could not be made there anyway. With `audit/` present and
// empty, a reader still answers "no queue" and makes no lock.
#[test]
fn a_reader_of_an_empty_audit_directory_makes_no_lock() {
    let scratch = Scratch::new("GF1-EMPTY-AUDIT");
    let names = scratch.names();
    fs::create_dir_all(&names.dir).unwrap();
    assert_eq!(window(scratch.root()), Ok(Page::default()));
    assert_eq!(older(scratch.root()), Ok(Older::default()));
    assert!(
        !names.lock.exists(),
        "a read never writes, not even the lock"
    );
    assert!(!names.queue.exists());
}

// GF-1: the guard `why.kind() == StorageFull` in `write_record` mutated to
// `true` survived: every refused write in these tests was the seam's full
// disk. Only a full disk spends the reserve. A write refused for any other
// reason (here a handle opened read-only) is the append's own failure, and
// the reserve stays exactly as it was.
#[test]
fn a_write_refused_other_than_full_is_the_appends_failure_and_keeps_the_reserve() {
    let scratch = Scratch::new("GF1-NOT-FULL");
    let names = scratch.names();
    fs::create_dir_all(&names.dir).unwrap();
    assert_eq!(hold_reserve(&names), Reserve::Held);
    let reserve = fs::read(&names.reserve).unwrap();
    fs::write(&names.queue, header(NOW)).unwrap();
    let read_only = fs::File::open(&names.queue).unwrap();
    let refused = write_record(
        &names,
        &read_only,
        &raw_alert("GF1-K", 1).encode(),
        HEADER_LEN,
    );
    assert!(
        matches!(&refused, Err(AlertError::Io { op: "append", .. })),
        "{refused:?}"
    );
    assert_eq!(
        fs::read(&names.reserve).unwrap(),
        reserve,
        "a refusal that is not a full disk spends nothing"
    );
    assert_eq!(
        fs::read(&names.queue).unwrap(),
        header(NOW),
        "nothing was written"
    );
}

// GF-1: `sync_dir` replaced by `Ok(())` survived: no test met a directory it
// could not sync. One that cannot be opened is the refusal named "sync the
// directory" with the host's own kind, never a quiet success.
#[test]
fn a_directory_that_cannot_be_synced_is_refused_by_name() {
    let scratch = Scratch::new("GF1-SYNC-DIR");
    assert_eq!(sync_dir(scratch.root()), Ok(()));
    let refused = sync_dir(&scratch.root().join("GF1-NO-SUCH-DIR"));
    assert!(
        matches!(
            &refused,
            Err(AlertError::Io {
                op: "sync the directory",
                kind: io::ErrorKind::NotFound,
                ..
            })
        ),
        "{refused:?}"
    );
}

// GF-1, second look: `||` mutated to `&&` in `ack_at`'s range check still
// survived the test above, because without the check ordinal 0 and one past
// the end reach the older walk, which answers "no such alert" as well --
// unless a record already acknowledges that ordinal, as a crafted one can.
// The range check answers first: no such alert, never "already acknowledged".
#[test]
fn a_crafted_acknowledgement_of_ordinal_zero_or_past_the_end_is_still_no_such_alert() {
    let scratch = Scratch::new("GF1-ACK-CRAFTED");
    raw_queue(
        &scratch,
        &[
            raw_alert("a", 1),
            raw_alert("b", 2),
            raw_ack(0, 2),
            raw_ack(9, 2),
        ],
    );
    let before = fs::read(scratch.names().queue).unwrap();
    for ordinal in [0_u32, 9] {
        assert_eq!(
            ack_at(scratch.root(), ordinal, "cli", Scope::Whole, NOW),
            Err(AlertError::NoSuchAlert { ordinal })
        );
    }
    assert_eq!(
        fs::read(scratch.names().queue).unwrap(),
        before,
        "nothing was written"
    );
}

// GF-1: `queue.nlink() > 1` mutated to `>= 1` in `keep` survived. The look
// for a name a stopped replacement already kept is taken only when the queue
// has a second link: a queue with one link is kept without reading audit/'s
// names, so an audit/ that cannot be listed (mode 0300: a name can be made
// in it, none read) does not stop the keep.
#[test]
fn a_queue_with_one_link_is_kept_without_listing_audit() {
    let scratch = Scratch::new("GF1-ONE-LINK");
    let names = scratch.names();
    raw_queue(&scratch, &open_alerts(2));
    fs::set_permissions(&names.dir, fs::Permissions::from_mode(0o300)).unwrap();
    let kept = keep(&names, NOW);
    fs::set_permissions(&names.dir, fs::Permissions::from_mode(0o755)).unwrap();
    let name = names.dir.join(format!("alerts.damaged-{NOW}"));
    assert_eq!(kept, Ok(name.clone()));
    assert_eq!(fs::read(&name).unwrap(), fs::read(&names.queue).unwrap());
}
