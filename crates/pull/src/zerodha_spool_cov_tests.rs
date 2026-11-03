//! BASE-COV-PULL: a spool's bytes of any other shape (Route B, recipe v4). Each
//! header below carries a true CRC, so each refusal is reached on its own.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_fixtures::scratch;

fn id() -> SpoolId {
    SpoolId {
        run: 7,
        segment: Segment::Cash,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 3).unwrap(),
    }
}

/// `header` with `at..` overwritten and its CRC made true again.
fn resealed(header: [u8; 64], at: usize, bytes: &[u8]) -> [u8; 64] {
    let mut out = header;
    put(&mut out, at, bytes);
    let crc = crc32c(out.get(..60).unwrap_or_default());
    put(&mut out, 60, &crc.to_le_bytes());
    out
}

fn refused(bytes: &[u8]) -> String {
    let path = Path::new("/invented/spool");
    decode_header(bytes, path).unwrap_err().to_string()
}

#[test]
fn a_header_is_refused_for_each_field_on_its_own() {
    let good = encode_header(&id(), Mode::ZerodhaMerge, 1_700_000_000);
    let read = decode_header(&good, Path::new("/x")).unwrap().unwrap();
    assert_eq!((read.id, read.mode), (id(), Mode::ZerodhaMerge));
    assert_eq!(decode_header(&[0; 64], Path::new("/x")).unwrap(), None);
    assert_eq!(decode_header(&good[..63], Path::new("/x")).unwrap(), None);
    let mut rotted = good;
    rotted[20] ^= 1;
    assert!(refused(&rotted).contains("its CRC-32C does not match"));
    for (at, bytes, word) in [
        (0, &b"NOTASPOO"[..], "does not begin BRUTEXVS"),
        (8, &[2, 0][..], "its version is not 1"),
        (10, &[1, 0][..], "its stride is not 160"),
        (12, &[250][..], "names a feed other than zerodha"),
        (13, &[250][..], "names no segment"),
        (14, &[250][..], "timeframe code and seconds disagree"),
        (15, &[250][..], "its mode is not a Zerodha pass"),
        (18, &[13][..], "names no month"),
    ] {
        let said = refused(&resealed(good, at, bytes));
        assert!(said.contains(word), "{word}: {said}");
    }
}

#[test]
fn a_log_record_is_trusted_only_whole_and_with_a_true_crc() {
    let record = LogRecord {
        day: 19_000,
        generation: 3,
        state: LogState::Committed,
        exchange: 1,
    };
    let bytes = record.encode();
    assert_eq!(LogRecord::decode(&bytes), Some(record));
    assert_eq!(LogRecord::decode(&bytes[..15]), None, "a byte short");
    let mut rotted = bytes;
    rotted[0] ^= 1;
    assert_eq!(LogRecord::decode(&rotted), None);
    let mut reserved = bytes;
    reserved[10] = 1;
    let crc = crc32c(&reserved[..12]);
    put(&mut reserved, 12, &crc.to_le_bytes());
    assert_eq!(
        LogRecord::decode(&reserved),
        None,
        "a true CRC, reserved set"
    );
    let mut state = bytes;
    state[8] = 250;
    let crc = crc32c(&state[..12]);
    put(&mut state, 12, &crc.to_le_bytes());
    assert_eq!(LogRecord::decode(&state), None, "a true CRC, no such state");
}

/// A spool past its cap is refused before a byte of it is read.
#[test]
fn a_spool_past_its_byte_cap_is_refused_unread() {
    let root = scratch("cov-spool-cap");
    let path = root.join("2024-03.vsp");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_SPOOL_BYTES + 1).unwrap();
    let said = read_spool(&path, &id()).unwrap_err().to_string();
    assert!(said.contains("is past the"), "{said}");
    std::fs::remove_dir_all(root).unwrap();
}

// ---------------------------------------------------------------------------
// BASE-COV-PULL-PS2 (3 Nov 2026): the arms Route B found unreached on START
// (2e7a0b6d3b56), each driven by input or a direct call and asserted. Keys and
// details are invented.
// ---------------------------------------------------------------------------

const KEY_A: &str = "NSE-CASH-ZZSPOOLA";
const KEY_B: &str = "NSE-CASH-ZZSPOOLB";

/// One spooled verdict of `key` on a day of `id()`'s month, with `details`
/// invented details.
fn spooled(key: &str, details: usize) -> SpoolRecord {
    SpoolRecord {
        exchange: Exchange::Nse,
        day: Day::new(2024, 3, 4).unwrap(),
        key: Key::instrument(key).unwrap(),
        facts: Facts::of(Code::Match),
        receipt: 1,
        details: (0..details)
            .map(|i| Detail {
                kind: 1,
                payload: vec![u8::try_from(i).unwrap(); 3],
            })
            .collect(),
    }
}

/// Record `index` (160 bytes) of `bytes`.
fn slot(bytes: &[u8], index: usize) -> [u8; 160] {
    bytes[index * 160..(index + 1) * 160].try_into().unwrap()
}

/// `bytes` with its CRC at 156 made true again.
fn resealed_record(mut bytes: [u8; 160]) -> [u8; 160] {
    let crc = crc32c(&bytes[..156]);
    put(&mut bytes, 156, &crc.to_le_bytes());
    bytes
}

/// `id()`'s spool under `root`, born with its header and then `records`.
fn spool_of(root: &Path, records: &[[u8; 160]]) -> PathBuf {
    let path = id().path(root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = encode_header(&id(), Mode::ZerodhaMerge, 1_700_000_000).to_vec();
    for record in records {
        bytes.extend_from_slice(record);
    }
    fs::write(&path, bytes).unwrap();
    path
}

fn writer(root: &Path) -> SpoolWriter {
    SpoolWriter::new(root, 7, Mode::ZerodhaMerge, 1_700_000_000)
}

/// PS2-SP-01: a record of any shape no writer writes, its CRC true, is refused
/// by name: reserved bytes set, a detail count past `MAX_DETAILS`, a count on a
/// detail, a key hash that is not its text's, a detail of kind 0 or past
/// `DETAIL_PAYLOAD` bytes, and a kind byte no writer writes. `MAX_DETAILS`
/// itself is read.
#[test]
fn a_record_of_any_other_shape_is_refused_by_name() {
    let both = encode_record(&spooled(KEY_A, 1), 7).unwrap();
    assert_eq!(both.len(), 320);
    let (verdict, detail) = (slot(&both, 0), slot(&both, 1));
    assert!(matches!(
        decode_record(&verdict),
        Ok(Decoded::Verdict(_, 7, 1))
    ));
    assert!(matches!(decode_record(&detail), Ok(Decoded::Detail(..))));
    let refused = |bytes: [u8; 160]| decode_record(&resealed_record(bytes)).err();
    let reserved = Some("has reserved bytes that are not zero");
    let mut bytes = verdict;
    bytes[155] = 1;
    assert_eq!(refused(bytes), reserved, "a reserved byte");
    let mut bytes = verdict;
    put(&mut bytes, 150, &33_u16.to_le_bytes());
    assert_eq!(refused(bytes), reserved, "33 details declared");
    let mut bytes = detail;
    put(&mut bytes, 150, &1_u16.to_le_bytes());
    assert_eq!(refused(bytes), reserved, "a count on a detail");
    let mut bytes = verdict;
    bytes[0] ^= 1;
    assert_eq!(
        refused(bytes),
        Some("holds a key whose hash is not its text's")
    );
    let no_writer = Some("holds a detail no writer writes");
    let mut bytes = detail;
    put(&mut bytes, 8, &0_u16.to_le_bytes());
    assert_eq!(refused(bytes), no_writer, "kind 0");
    let mut bytes = detail;
    put(&mut bytes, 10, &129_u16.to_le_bytes());
    assert_eq!(refused(bytes), no_writer, "129 bytes");
    let mut bytes = detail;
    bytes[149] = 3;
    assert_eq!(refused(bytes), Some("names no record kind"));
    let mut bytes = verdict;
    put(&mut bytes, 150, &32_u16.to_le_bytes());
    assert!(matches!(
        decode_record(&resealed_record(bytes)),
        Ok(Decoded::Verdict(_, 7, 32))
    ));
}

/// PS2-SP-02: a detail no spool can hold (kind 0, or one byte past
/// `DETAIL_PAYLOAD`) is refused `record_refused` before a byte is written,
/// the writer's whole call with it; exactly `DETAIL_PAYLOAD` bytes is kept.
#[test]
fn a_detail_no_spool_can_hold_is_refused_before_a_byte() {
    let mut zero = spooled(KEY_A, 1);
    zero.details[0].kind = 0;
    let mut wide = spooled(KEY_A, 1);
    wide.details[0].payload = vec![9; DETAIL_PAYLOAD + 1];
    for bad in [&zero, &wide] {
        let said = encode_record(bad, 7).unwrap_err().to_string();
        assert!(
            said.starts_with("record_refused: detail kind")
                && said.contains("a spool detail is a kind above 0 and at most 128 bytes"),
            "{said}"
        );
    }
    let mut widest = spooled(KEY_A, 1);
    widest.details[0].payload = vec![9; DETAIL_PAYLOAD];
    let bytes = encode_record(&widest, 7).unwrap();
    let Ok(Decoded::Detail(_, kept)) = decode_record(&bytes[160..]) else {
        panic!("the widest detail decodes");
    };
    assert_eq!(kept.payload, vec![9; DETAIL_PAYLOAD]);
    let root = scratch("ps2-spool-detail");
    let refused = writer(&root).append(
        Segment::Cash,
        Timeframe::MINUTE_1,
        &[spooled(KEY_B, 0), zero],
    );
    assert!(
        matches!(refused, Err(SpoolError::Record { .. })),
        "{refused:?}"
    );
    assert!(!root.join(SPOOL_DIR).exists() && !root.join(LIVE_DIR).exists());
    fs::remove_dir_all(root).unwrap();
}

/// PS2-SP-03: a run's live lock whose name holds a directory is refused `io:
/// open` with the lock opener's words, by the merger's probe and by the
/// writer's first append, which writes no spool.
#[test]
fn a_live_lock_name_that_holds_no_lock_file_is_refused_by_name() {
    let root = scratch("ps2-live-dir");
    let path = live_lock_path(&root, 7);
    fs::create_dir_all(&path).unwrap();
    let probed = live_probe(&root, 7);
    let Err(SpoolError::Io {
        path: named,
        action,
        source,
    }) = probed
    else {
        panic!("{probed:?}");
    };
    assert_eq!((named, action), (path.clone(), "open"));
    assert!(
        source
            .to_string()
            .contains("a directory and cannot be opened"),
        "{source}"
    );
    let refused = writer(&root).append(Segment::Cash, Timeframe::MINUTE_1, &[spooled(KEY_A, 0)]);
    assert!(
        matches!(&refused, Err(SpoolError::Io { action: "open", path: p, .. }) if *p == path),
        "{refused:?}"
    );
    assert!(!id().path(&root).exists(), "no spool written");
    fs::remove_dir_all(root).unwrap();
}

/// PS2-SP-04: a run whose live lock another holder keeps past [`LIVE_WAIT`]
/// is refused `run_live_busy` naming the run, after the whole wait and with
/// nothing appended; once the holder lets go the same writer appends. This
/// test waits `LIVE_WAIT` (30 s) by design: the bound is a constant.
#[test]
fn a_run_held_live_past_the_wait_is_refused_busy() {
    let root = scratch("ps2-live-busy");
    let Live::Taken(held) = live_probe(&root, 7).unwrap() else {
        panic!("a fresh run's lock is free");
    };
    let spool = writer(&root);
    let started = std::time::Instant::now();
    let refused = spool.append(Segment::Cash, Timeframe::MINUTE_1, &[spooled(KEY_A, 0)]);
    let waited = started.elapsed();
    let Err(SpoolError::Busy { run, holder }) = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(*run, 7);
    assert!(!holder.is_empty());
    assert!(waited >= LIVE_WAIT, "it waited its whole bound: {waited:?}");
    let said = refused.unwrap_err().to_string();
    assert!(
        said.starts_with("run_live_busy: run 7's live lock"),
        "{said}"
    );
    assert!(!id().path(&root).exists(), "nothing appended");
    drop(held);
    let done = spool
        .append(Segment::Cash, Timeframe::MINUTE_1, &[spooled(KEY_A, 0)])
        .unwrap();
    assert_eq!((done.records, done.born), (1, 1));
    drop(spool);
    fs::remove_dir_all(root).unwrap();
}

/// PS2-SP-05: a spool name the host will not measure (its folder's search bit
/// is off, where the permission bits bind) is refused `io: stat` with the
/// host's own kind, never taken for absent and never created over.
#[test]
fn a_spool_name_the_host_will_not_measure_is_refused_with_its_kind() {
    crate::support::where_permission_binds(
        "zerodha_verify::spool::cov_tests::a_spool_name_the_host_will_not_measure_is_refused_with_its_kind",
        unmeasured_spool_body,
    );
}

fn unmeasured_spool_body() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = scratch("ps2-spool-eacces");
    let path = id().path(&root);
    let dir = path.parent().unwrap().to_path_buf();
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o600)).unwrap();
    let refused = writer(&root).append(Segment::Cash, Timeframe::MINUTE_1, &[spooled(KEY_A, 0)]);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    let Err(SpoolError::Io {
        path: named,
        action,
        source,
    }) = refused
    else {
        panic!("{refused:?}");
    };
    assert_eq!(
        (named, action, source.kind()),
        (path.clone(), "stat", ErrorKind::PermissionDenied)
    );
    assert!(!path.exists(), "never created over");
    fs::remove_dir_all(root).unwrap();
}

/// PS2-SP-06: an unborn header (all zeros) with bytes after it holds nothing:
/// the bytes are named torn from offset 64, never read as records; with none
/// after it there is no torn tail.
#[test]
fn an_unborn_header_with_bytes_after_it_names_them_torn() {
    let root = scratch("ps2-spool-unborn");
    let path = id().path(&root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut bytes = vec![0_u8; 64];
    fs::write(&path, &bytes).unwrap();
    let read = read_spool(&path, &id()).unwrap();
    assert_eq!((read.header, read.torn), (None, None));
    bytes.extend_from_slice(&[0x5a; 10]);
    fs::write(&path, &bytes).unwrap();
    let read = read_spool(&path, &id()).unwrap();
    assert_eq!(read.header, None);
    assert_eq!(read.torn, Some((64, 10)));
    assert!(read.records.is_empty());
    fs::remove_dir_all(root).unwrap();
}

/// PS2-SP-07: the record order a writer never leaves is damage, named at the
/// record: a verdict record after one still owed a detail, a detail whose key
/// is not the record's before it, and a detail with no record before it at
/// all. The whole spool is refused; nothing of it is merged.
#[test]
fn records_in_an_order_no_writer_leaves_are_damage_by_name() {
    let a_owing = slot(&encode_record(&spooled(KEY_A, 1), 7).unwrap(), 0);
    let a_whole = slot(&encode_record(&spooled(KEY_A, 0), 7).unwrap(), 0);
    let b_detail = slot(&encode_record(&spooled(KEY_B, 1), 7).unwrap(), 1);
    for (records, at, why) in [
        (
            vec![a_owing, a_whole],
            1,
            "follows a record that lacks a detail it declares",
        ),
        (
            vec![a_owing, b_detail],
            1,
            "is a detail with no record of its key before it",
        ),
        (
            vec![b_detail, a_whole],
            0,
            "is a detail with no record of its key before it",
        ),
    ] {
        let root = scratch("ps2-spool-order");
        let path = spool_of(&root, &records);
        let refused = read_spool(&path, &id());
        let Err(SpoolError::Damaged {
            path: named,
            record,
            why: said,
        }) = refused
        else {
            panic!("{why}: {refused:?}");
        };
        assert_eq!((named, record, said), (path.clone(), at, why));
        fs::remove_dir_all(root).unwrap();
    }
}

/// PS2-SP-08: a run resumed over a spool holding only its header appends at
/// byte 64; over a spool whose one record never finished its sync (a bad CRC)
/// it writes over that record, so the spool reads whole with nothing torn or
/// unsynced.
#[test]
fn a_resumed_run_appends_after_its_header_or_over_an_unsynced_record() {
    for torn in [false, true] {
        let root = scratch("ps2-spool-resume");
        let records = if torn {
            vec![[0x5a_u8; 160]]
        } else {
            Vec::new()
        };
        let path = spool_of(&root, &records);
        if torn {
            assert_eq!(read_spool(&path, &id()).unwrap().unsynced, Some(64));
        }
        let spool = writer(&root);
        let done = spool
            .append(Segment::Cash, Timeframe::MINUTE_1, &[spooled(KEY_A, 1)])
            .unwrap();
        assert_eq!(
            (done.records, done.slots, done.born),
            (1, 2, 0),
            "torn {torn}"
        );
        drop(spool);
        let read = read_spool(&path, &id()).unwrap();
        assert_eq!(read.records, vec![spooled(KEY_A, 1)], "torn {torn}");
        assert_eq!((read.torn, read.unsynced), (None, None), "torn {torn}");
        assert_eq!(fs::metadata(&path).unwrap().len(), 64 + 2 * 160);
        fs::remove_dir_all(root).unwrap();
    }
}

/// PS2-SP-09: a merge log the host will not measure (a path through a regular
/// file) is refused `io: stat` with its kind; a directory at the log's name is
/// refused `not_regular`. Neither is created over and the log learns nothing.
#[test]
fn a_merge_log_name_that_is_no_file_is_refused_by_name() {
    let root = scratch("ps2-log-name");
    let file = root.join("a-file");
    fs::write(&file, b"invented").unwrap();
    let record = LogRecord {
        day: 19_786,
        generation: 1,
        state: LogState::Committed,
        exchange: 1,
    };
    let mut log = LogRead::default();
    let through = file.join("2024-03.vsp.merged");
    let refused = append_log(&through, &mut log, record);
    let Err(SpoolError::Io { action, source, .. }) = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!((*action, source.kind()), ("stat", ErrorKind::NotADirectory));
    let dir = root.join("2024-04.vsp.merged");
    fs::create_dir(&dir).unwrap();
    let refused = append_log(&dir, &mut log, record);
    assert!(
        matches!(&refused, Err(SpoolError::NotRegular { path }) if *path == dir),
        "{refused:?}"
    );
    assert_eq!(refused.unwrap_err().code(), "not_regular");
    assert_eq!((log.next, log.records.len()), (0, 0), "learned nothing");
    fs::remove_dir_all(root).unwrap();
}

/// PS2-SP-10: every name the spool walk reads past is named stray, at each
/// level: a file where a run's segment folder goes, a folder no segment is, a
/// folder no timeframe is, a month whose number or year is not two or four
/// digits, a folder where a spool goes; the one spool among them is listed.
/// A spool tree whose top is a file is refused `io: list` with its kind.
#[test]
fn every_name_the_spool_walk_reads_past_is_stray() {
    let root = scratch("ps2-spool-walk");
    let tf = id().path(&root).parent().unwrap().to_path_buf();
    fs::create_dir_all(&tf).unwrap();
    let run = tf.parent().unwrap().parent().unwrap().to_path_buf();
    let stray_paths = [
        run.join("a-file"),
        run.join("BOGUS"),
        run.join("CASH").join("nine-minutes"),
        tf.join("2024-3.vsp"),
        tf.join("20x4-03.vsp"),
        tf.join("2024-0x.vsp"),
        tf.join("2024-04.vsp"),
    ];
    fs::write(id().path(&root), b"").unwrap();
    fs::write(&stray_paths[0], b"").unwrap();
    fs::create_dir(&stray_paths[1]).unwrap();
    fs::create_dir(&stray_paths[2]).unwrap();
    for file in &stray_paths[3..6] {
        fs::write(file, b"").unwrap();
    }
    fs::create_dir(&stray_paths[6]).unwrap();
    let mut found = waiting(&root).unwrap();
    assert_eq!(found.spools, vec![id()]);
    found.stray.sort();
    let mut expected = stray_paths.to_vec();
    expected.sort();
    assert_eq!(found.stray, expected);
    let flat = scratch("ps2-spool-flat");
    fs::create_dir_all(flat.join("verify")).unwrap();
    fs::write(flat.join(SPOOL_DIR), b"").unwrap();
    let refused = waiting(&flat);
    let Err(SpoolError::Io { action, source, .. }) = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!((*action, source.kind()), ("list", ErrorKind::NotADirectory));
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(flat).unwrap();
}

/// PS2-SP-11: the byte helpers at their edges, by direct call: the empty text
/// is no run of digits, and a `put` that would end past its buffer writes
/// nothing at all, while one that ends exactly at its end writes whole.
#[test]
fn the_byte_helpers_hold_at_their_edges() {
    assert!(!all_digits(""));
    assert!(all_digits("0"));
    assert!(!all_digits("+5"));
    let mut out = [0_u8; 4];
    put(&mut out, 2, &[1, 2, 3]);
    assert_eq!(out, [0; 4], "nothing past the end, and nothing before it");
    put(&mut out, 1, &[7, 8, 9]);
    assert_eq!(out, [0, 7, 8, 9]);
    put(&mut out, 4, &[]);
    assert_eq!(out, [0, 7, 8, 9]);
}

/// PS2-SP-12: a merge log or a spool the host refuses to open keeps the host's
/// words: a path through a regular file is `io: open` with its kind (code
/// `io`, said with the action and path), a directory at the name is
/// `not_regular`; an absent log is empty, never refused.
#[test]
fn a_log_or_spool_the_host_refuses_to_open_keeps_its_words() {
    let root = scratch("ps2-log-open");
    let file = root.join("a-file");
    fs::write(&file, b"invented").unwrap();
    let through = file.join("2024-03.vsp.merged");
    let refused = read_log(&through).unwrap_err();
    let SpoolError::Io {
        path,
        action,
        source,
    } = &refused
    else {
        panic!("{refused:?}");
    };
    assert_eq!(
        (path, *action, source.kind()),
        (&through, "open", ErrorKind::NotADirectory)
    );
    assert_eq!(refused.code(), "io");
    assert!(
        refused
            .to_string()
            .starts_with(&format!("io: open {}: ", through.display())),
        "{refused}"
    );
    let dir = root.join("2024-04.vsp.merged");
    fs::create_dir(&dir).unwrap();
    assert!(
        matches!(read_log(&dir), Err(SpoolError::NotRegular { path }) if path == dir),
        "a directory at a log's name"
    );
    assert!(
        matches!(read_spool(&dir, &id()), Err(SpoolError::NotRegular { path }) if path == dir),
        "a directory at a spool's name"
    );
    let absent = read_log(&root.join("2024-05.vsp.merged")).unwrap();
    assert_eq!((absent.next, absent.records.len()), (0, 0));
    fs::remove_dir_all(root).unwrap();
}
