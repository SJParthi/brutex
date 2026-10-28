#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]
//! BASE-COV-LIBS (D-7324): every arm of the quarantine's byte codec, its
//! header check, its run walk and its refusals that the module's own tests
//! never drove, each driven by the input that reaches it. Every key and
//! stamp is invented.

use std::os::unix::fs::PermissionsExt as _;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::file::StoreError;
use crate::path::{PathParts, Timeframe};
use crate::repair::RepairError;

static NEXT: AtomicU64 = AtomicU64::new(0);
const FEED: Vendor = Vendor::Zerodha;
const KEY: &str = "NSE/CASH/QFIXA/1min";

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-quarantine-cover-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a scratch root");
        Self(root)
    }

    fn audit(&self) -> PathBuf {
        let dir = self.0.join(DIR);
        fs::create_dir_all(&dir).expect("the audit directory");
        dir
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _cleanup = fs::remove_dir_all(&self.0);
    }
}

fn march() -> YearMonth {
    YearMonth::new(2018, 3).expect("2018-03")
}

fn minute(symbol: &'static str, month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: FEED,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

fn entry(scope: Scope, cause: Cause, bars: u64) -> Entry {
    Entry {
        scope,
        cause,
        causes: Causes::of(cause),
        bars,
        ledger_at: 64,
        revision: 2,
        written_at: 1_700_000_000_000_000,
    }
}

fn sealed(body: &str) -> String {
    format!("{body}\t{:08x}", crc32c(body.as_bytes()))
}

const GOOD: &str = "1700000000000000\tzerodha\tNSE/CASH/QFIXA/1min\t2018-03\tdropped\tzero_price\t2\t3\tabcd\ttest\tinvented\t1,2";

#[test]
fn bytes_no_writer_wrote_are_none_and_a_cause_set_says_itself() {
    assert_eq!(Cause::of_byte(0), None);
    assert_eq!(Cause::of_byte(4), None);
    assert_eq!(Cause::of_byte(u8::MAX), None);
    for cause in Cause::ALL {
        assert_eq!(Cause::of_byte(cause.byte()), Some(cause));
    }
    assert_eq!(Scope::of_byte(0), None);
    assert_eq!(Scope::of_byte(3), None);
    assert_eq!(Scope::Dropping.byte(), 0);
    assert_eq!(Scope::of_byte(Scope::WholeMonth.byte()), Some(Scope::WholeMonth));
    assert_eq!(Scope::of_byte(Scope::Dropped.byte()), Some(Scope::Dropped));
    assert_eq!(Causes::of_byte(0), None);
    assert_eq!(Causes::of_byte(0b1000), None);
    // A known bit beside an unknown one is still no set a writer wrote.
    assert_eq!(Causes::of_byte(0b1001), None);
    assert_eq!(Causes::of_byte(u8::MAX), None);
    assert_eq!(Causes::of_byte(0b111), Some(Causes(0b111)));
    assert_eq!(Causes(0).said(), "");
    assert_eq!(Causes::of(Cause::ZeroPrice).said(), "zero_price");
    assert_eq!(Causes(0b011).said(), "before_listing and foreign_level");
    assert_eq!(
        Causes(0b111).said(),
        "before_listing, foreign_level and zero_price"
    );
}

#[test]
fn a_field_read_or_write_never_reaches_past_its_buffer() {
    assert_eq!(field::<4>(&[1, 2], 0), [0; 4]);
    assert_eq!(field::<2>(&[1, 2, 3], 1), [2, 3]);
    assert_eq!(field::<2>(&[1, 2, 3], usize::MAX), [0; 2]);
    assert_eq!(field::<0>(&[], 0), [0u8; 0]);
    let mut short = [7u8; 3];
    write_at(&mut short, 2, &[1, 2]);
    assert_eq!(short, [7; 3]);
    write_at(&mut short, usize::MAX, &[1]);
    assert_eq!(short, [7; 3]);
    write_at(&mut short, 1, &[1, 2]);
    assert_eq!(short, [7, 1, 2]);
    write_at(&mut short, 3, &[]);
    assert_eq!(short, [7, 1, 2]);
    // An image too short for a header or a slot is written nowhere: never a
    // panic, never a byte past its end.
    let mut tiny = vec![0u8; 10];
    put(
        &mut tiny,
        0,
        &[9; 16],
        5,
        None,
        entry(Scope::Dropped, Cause::ZeroPrice, 1),
    );
    set_described(&mut tiny, 77);
    assert_eq!(tiny, vec![0u8; 10]);
    // A header and no slot: the slot is not written, the count is.
    let mut slotless = vec![0u8; HEADER];
    put(
        &mut slotless,
        0,
        &[9; 16],
        5,
        None,
        entry(Scope::Dropped, Cause::ZeroPrice, 1),
    );
    assert_eq!(u64::from_le_bytes(field(&slotless, 32)), 1);
    assert_eq!(
        u32::from_le_bytes(field(&slotless, 60)),
        crc32c(&slotless[..60])
    );
}

#[test]
fn a_header_wrong_in_any_one_field_does_not_check() {
    let image = empty_image();
    assert_eq!(image.len() as u64, INDEX_LEN);
    assert_eq!(described(&image), Some(0));
    assert!(image_trusted(&image));
    for (at, len) in [(0usize, 8usize), (8, 4), (12, 4), (16, 8), (24, 8)] {
        let mut bad = image.clone();
        bad[at + len - 1] ^= 0x40;
        seal_header(&mut bad);
        assert_eq!(described(&bad), None, "the field at {at}");
        assert!(!image_trusted(&bad), "the field at {at}");
    }
    let mut unsealed = image.clone();
    unsealed[40] = 1;
    assert_eq!(described(&unsealed), None);
    let mut level = image.clone();
    set_described(&mut level, u64::MAX);
    assert_eq!(described(&level), Some(u64::MAX));
    assert_eq!(described(&image[..10]), None);
    assert_eq!(described(&[]), None);
}

#[test]
fn a_run_answers_clear_past_other_months_and_damage_for_a_slot_without_scope() {
    let hash = hash_of(KEY, march());
    let other = hash_of("NSE/CASH/QFIXB/1min", march());
    let month = month_code(march());
    let home = home_of(&hash);
    let index = PathBuf::from("/brutex-cover/absent/index");
    let held = entry(Scope::Dropped, Cause::ForeignLevel, 3);
    let mut run = vec![0u8; BUCKET_BYTES];
    run[..SLOT].copy_from_slice(&encode_slot(&other, month, &held, home));
    assert_eq!(
        find_in_run(&run, home, &hash, month, &index),
        Ok(Probe::Clear)
    );
    run[..SLOT].copy_from_slice(&encode_slot(&hash, month + 1, &held, home));
    assert_eq!(
        find_in_run(&run, home, &hash, month, &index),
        Ok(Probe::Clear)
    );
    for (k, slot) in run.chunks_exact_mut(SLOT).enumerate() {
        slot.copy_from_slice(&encode_slot(&other, month, &held, home + k as u64));
    }
    // Every slot of the run held by another key: the run's end is clear.
    assert_eq!(
        find_in_run(&run, home, &hash, month, &index),
        Ok(Probe::Clear)
    );
    // The key's own slot with no scope byte (an intent is never indexed so).
    let intent = entry(Scope::Dropping, Cause::ForeignLevel, 0);
    run[..SLOT].copy_from_slice(&encode_slot(&hash, month, &intent, home));
    assert_eq!(
        find_in_run(&run, home, &hash, month, &index),
        Err(QuarantineError::Damaged {
            path: index.clone(),
            slot: home,
        })
    );
    // The writer's walk: a foreign month is passed over, the next free slot
    // answers, then the key's own month is found behind the foreign one.
    let mut image = empty_image();
    put(&mut image, home, &other, month, None, held);
    assert_eq!(find_slot(&image, &hash, month, &index), Ok((home + 1, None)));
    let own = entry(Scope::WholeMonth, Cause::ZeroPrice, 9);
    put(&mut image, home + 1, &hash, month, None, own);
    assert_eq!(
        find_slot(&image, &hash, month, &index),
        Ok((home + 1, Some(own)))
    );
    assert_eq!(u64::from_le_bytes(field(&image, 32)), 2);
    write_at(
        &mut image,
        slot_at(home),
        &encode_slot(&other, month, &intent, home),
    );
    assert_eq!(
        find_slot(&image, &hash, month, &index),
        Err(QuarantineError::Damaged { path: index, slot: home })
    );
    assert!(!image_trusted(&image));
}

#[test]
fn an_index_or_ledger_that_is_no_regular_file_is_refused_by_name() {
    let store = Scratch::new("notfile");
    store.audit();
    fs::create_dir(index_path(&store.0, FEED)).expect("a directory where the index goes");
    let refused = probe_at(&store.0, FEED, KEY, march()).expect_err("a directory is no index");
    assert!(
        matches!(
            &refused,
            QuarantineError::Io {
                operation: "open the quarantine index",
                kind: io::ErrorKind::InvalidInput,
                ..
            }
        ),
        "{refused:?}"
    );
    assert_eq!(refused.code(), "quarantine_io");
    assert!(matches!(
        Snapshot::load(&store.0, FEED),
        Err(QuarantineError::Io {
            operation: "open the quarantine index",
            kind: io::ErrorKind::InvalidInput,
            ..
        })
    ));
    fs::create_dir(ledger_path(&store.0, FEED)).expect("a directory where the ledger goes");
    assert!(matches!(
        read_ledger(&store.0, FEED),
        Err(QuarantineError::Io {
            operation: "open the quarantine ledger",
            kind: io::ErrorKind::InvalidInput,
            ..
        })
    ));
    assert!(matches!(
        ledger_len(&store.0, FEED),
        Err(QuarantineError::Io {
            operation: "inspect the quarantine ledger (not a regular file)",
            kind: io::ErrorKind::InvalidInput,
            ..
        })
    ));
    // A reader's stamps and the listing refuse the same way: never empty.
    assert!(matches!(
        ledger_stamps(&store.0, minute("QFIXA", march())),
        Err(QuarantineError::Io { .. })
    ));
    assert!(matches!(
        Summary::load(&store.0, FEED),
        Err(QuarantineError::Io { .. })
    ));
}

#[test]
fn an_index_the_host_will_not_open_is_refused_with_the_hosts_kind() {
    let store = Scratch::new("denied");
    store.audit();
    let index = index_path(&store.0, FEED);
    fs::write(&index, [0u8; 10]).expect("an index");
    fs::set_permissions(&index, fs::Permissions::from_mode(0o000)).expect("mode 000");
    let probed = probe_at(&store.0, FEED, KEY, march());
    let loaded = Snapshot::load(&store.0, FEED);
    fs::set_permissions(&index, fs::Permissions::from_mode(0o644)).expect("mode back");
    for refused in [probed.map(|_| ()), loaded.map(|_| ())] {
        assert!(
            matches!(
                refused,
                Err(QuarantineError::Io {
                    operation: "open the quarantine index",
                    kind: io::ErrorKind::PermissionDenied,
                    ..
                })
            ),
            "{refused:?}"
        );
    }
    // Readable again but torn: refused by its length, both ways.
    assert!(matches!(
        Snapshot::load(&store.0, FEED),
        Err(QuarantineError::Torn { len: 10, .. })
    ));
}

#[test]
fn an_audit_name_that_is_no_directory_is_refused_by_the_host() {
    let store = Scratch::new("auditfile");
    fs::write(store.0.join(DIR), b"not a directory").expect("a file named audit");
    let refused = ledger_len(&store.0, FEED).expect_err("no ledger under a file");
    assert!(
        matches!(
            &refused,
            QuarantineError::Io {
                operation: "inspect the quarantine ledger",
                ..
            }
        ),
        "{refused:?}"
    );
    // A root that is a file: the audit directory cannot even be looked at.
    let as_root = store.0.join(DIR);
    assert!(matches!(
        audit_dir(&as_root),
        Err(QuarantineError::Io {
            operation: "inspect the audit directory",
            ..
        })
    ));
    // A root whose parent is gone: the audit directory cannot be made.
    let gone = store.0.join("gone").join("deeper");
    assert!(matches!(
        audit_dir(&gone),
        Err(QuarantineError::Io {
            operation: "create the audit directory",
            kind: io::ErrorKind::NotFound,
            ..
        })
    ));
    assert!(!gone.exists());
    // No lock to open is no writer.
    assert!(!writer_active(&gone, FEED));
    assert_eq!(
        impatient_kind(&Impatient::Host(io::Error::from(io::ErrorKind::Unsupported))),
        io::ErrorKind::Unsupported
    );
    assert_eq!(
        impatient_kind(&Impatient::Held {
            bound: Duration::ZERO,
            waited: Duration::ZERO,
            attempts: 1,
            ahead: 0,
            holder: None,
        }),
        io::ErrorKind::WouldBlock
    );
}

#[test]
fn a_store_never_quarantined_answers_empty_everywhere() {
    let store = Scratch::new("empty");
    let snapshot = Snapshot::load(&store.0, FEED).expect("no index is an empty snapshot");
    assert!(snapshot.is_empty());
    assert_eq!(snapshot.len(), 0);
    let summary = Summary::load(&store.0, FEED).expect("no ledger is an empty summary");
    assert!(summary.is_empty());
    assert!(summary.instrument("NSE", "CASH", "QFIXA").is_none());
    assert!(summary.month(minute("QFIXA", march())).is_none());
    assert!(!writer_active(&store.0, FEED));
    // A catch-up of nothing rebuilds nothing, and writes only the lock.
    assert_eq!(catch_up(&store.0, FEED), Ok(None));
    assert!(!index_path(&store.0, FEED).exists());
    assert!(!ledger_path(&store.0, FEED).exists());
}

#[test]
fn an_act_with_a_bad_writer_or_unordered_stamps_is_refused_before_a_byte() {
    let store = Scratch::new("acts");
    let path = minute("QFIXA", march());
    let cases: [(&str, &[i64], &str); 7] = [
        ("", &[1], "writer's name"),
        ("a\tb", &[1], "writer's name"),
        ("a\nb", &[1], "writer's name"),
        ("a\rb", &[1], "writer's name"),
        ("ok", &[5, 3], "strictly ascending"),
        ("ok", &[4, 4], "strictly ascending"),
        ("ok", &[i64::MIN, i64::MAX, i64::MAX], "strictly ascending"),
    ];
    for (who, stamps, said) in cases {
        let refused = record(
            &store.0,
            &Record {
                path,
                scope: Scope::Dropped,
                cause: Cause::ZeroPrice,
                stamps,
                bars: stamps.len() as u64,
                revision: 1,
                source_digest: [0; 32],
                who,
                reason: "invented",
            },
        )
        .expect_err("refused");
        assert!(
            matches!(&refused, QuarantineError::Reason(text) if text.contains(said)),
            "{who:?} {stamps:?}: {refused:?}"
        );
    }
    assert!(!store.0.join(DIR).exists(), "nothing written, not even the lock");
    assert!(ascending(&[]));
    assert!(ascending(&[7]));
    assert!(ascending(&[i64::MIN, 0, i64::MAX]));
    assert!(!ascending(&[i64::MAX, i64::MIN]));
}

fn file_with(dir: &Path, name: &str, bytes: &[u8]) -> (PathBuf, File) {
    let path = dir.join(name);
    fs::write(&path, bytes).expect("a ledger file");
    let file = File::open(&path).expect("opened to read");
    (path, file)
}

#[test]
fn a_ledger_start_that_is_no_head_is_damage_at_its_offset() {
    let store = Scratch::new("headed");
    let dir = store.audit();
    let head = format!("{LEDGER_HEAD}\n");
    let damaged = |at: u64, path: &Path| QuarantineError::LedgerDamaged {
        path: path.to_path_buf(),
        offsets: vec![at],
    };
    let cases: [(&str, Vec<u8>, Result<bool, u64>); 7] = [
        ("empty", Vec::new(), Ok(false)),
        ("head", head.clone().into_bytes(), Ok(true)),
        ("prefix", b"brutex quarantine".to_vec(), Ok(false)),
        ("garbage", b"garbage\n".to_vec(), Err(0)),
        ("torn-then-garbage", b"x\ttorn\ngarbage\n".to_vec(), Err(7)),
        ("torn-then-head", format!("x\ttorn\n{head}").into_bytes(), Ok(true)),
        ("no-prefix", b"zzz".to_vec(), Err(0)),
    ];
    for (name, bytes, want) in cases {
        let (path, file) = file_with(&dir, name, &bytes);
        let got = headed(&file, &path, bytes.len() as u64);
        match want {
            Ok(flag) => assert_eq!(got, Ok(flag), "{name}"),
            Err(at) => assert_eq!(got, Err(damaged(at, &path)), "{name}"),
        }
    }
    // Ended torn heads filling the whole scan, and no head inside it: damage
    // where the scan ended, never a head guessed past it.
    let many = b"ab\ttorn\n".repeat(625);
    let (path, file) = file_with(&dir, "many", &many);
    assert_eq!(
        headed(&file, &path, many.len() as u64),
        Err(damaged(HEAD_SCAN, &path))
    );
}

#[test]
fn a_ledger_handle_the_host_will_not_read_or_write_is_refused_by_operation() {
    let store = Scratch::new("handles");
    let dir = store.audit();
    let path = dir.join("ledger");
    fs::write(&path, b"abc").expect("bytes");
    let write_only = OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("write only");
    assert!(matches!(
        tail_of(&write_only, &path),
        Err(QuarantineError::Io {
            operation: "read the ledger tail",
            ..
        })
    ));
    assert!(matches!(
        headed(&write_only, &path, 3),
        Err(QuarantineError::Io {
            operation: "read the ledger head",
            ..
        })
    ));
    let mut read_only = File::open(&path).expect("read only");
    let tail = Tail {
        len: 3,
        prefix: String::new(),
        ends_torn: false,
    };
    assert!(matches!(
        write_tail(&mut read_only, &path, &tail, "line\n"),
        Err(QuarantineError::Io {
            operation: "append to the quarantine ledger",
            ..
        })
    ));
    assert_eq!(fs::read(&path).expect("bytes"), b"abc");
}

#[test]
fn a_ledger_line_in_any_shape_no_writer_wrote_is_no_line() {
    let line = parse_line(&sealed(GOOD), 9).expect("the good line");
    assert_eq!(line.at, 9);
    assert_eq!(line.stamps, vec![1, 2]);
    assert_eq!(line.scope, Scope::Dropped);
    let none = GOOD.replace("\t1,2", "\t-");
    assert_eq!(
        parse_line(&sealed(&none), 0).expect("no stamps").stamps,
        Vec::<i64>::new()
    );
    let crc = crc32c(GOOD.as_bytes());
    let shapes = [
        format!("{GOOD}\t{}", &format!("{crc:08x}")[1..]),
        format!("{GOOD}\tzzzzzzzz"),
        format!("{GOOD}\t{:08x}", crc ^ 1),
        sealed(&GOOD.replace("\tinvented", "")),
        sealed(&GOOD.replace("2018-03", "2018-13")),
        sealed(&GOOD.replace("2018-03", "201803")),
        sealed(&GOOD.replace("\t1,2", "\t2,1")),
        sealed(&GOOD.replace("\t1,2", "\t1,x")),
        sealed(&GOOD.replace("dropped", "dropt")),
        sealed(&GOOD.replace("zero_price", "zero")),
        "no tab at all".to_owned(),
    ];
    for shape in &shapes {
        assert_eq!(parse_line(shape, 0), None, "{shape}");
    }
    // The head line that is not the head is damage at 0; a line that is not
    // UTF-8 is damage at its own offset; the act between them is read.
    let mut bytes = b"not the head\n".to_vec();
    let act = format!("{}\n", sealed(GOOD));
    bytes.extend_from_slice(act.as_bytes());
    let bad_at = bytes.len() as u64;
    bytes.extend_from_slice(b"\xff\xfe\n");
    bytes.extend_from_slice(b"torn");
    let ledger = parse_ledger(&bytes);
    assert_eq!(ledger.damaged, vec![0, bad_at]);
    assert_eq!(ledger.lines.len(), 1);
    assert_eq!(ledger.lines[0].at, 13);
    assert!(ledger.torn_tail);
    assert_eq!(ledger.len, bytes.len() as u64);
}

#[test]
fn an_image_built_from_a_line_whose_month_is_no_month_takes_no_slot() {
    let store = Scratch::new("image");
    let ledger = Ledger {
        lines: vec![Line {
            at: 28,
            written_at: 1,
            key: KEY.to_owned(),
            month: "2018-3x".to_owned(),
            scope: Scope::Dropped,
            cause: Cause::ZeroPrice,
            bars: 2,
            revision: 1,
            digest: String::new(),
            who: "t".to_owned(),
            reason: "r".to_owned(),
            stamps: Vec::new(),
        }],
        len: 99,
        ..Ledger::default()
    };
    let image = image_of(&ledger, &store.0, FEED).expect("an image");
    assert_eq!(described(&image), Some(99));
    assert_eq!(u64::from_le_bytes(field(&image, 32)), 0);
    assert!(image.get(HEADER..).expect("slots").iter().all(|byte| *byte == 0));
}

#[test]
fn a_writer_names_why_the_published_image_is_behind() {
    let store = Scratch::new("level");
    store.audit();
    let index = index_path(&store.0, FEED);
    fs::write(&index, [0u8; 10]).expect("a torn index");
    match level(&store.0, FEED, 5) {
        Ok(Level::Behind(why)) => assert_eq!(why, format!("it was 10 bytes, not {INDEX_LEN}")),
        Ok(Level::Current(_)) => panic!("a torn image is never current"),
        Err(refused) => panic!("a torn image is rebuilt, not refused: {refused}"),
    }
    let mut image = empty_image();
    image[0] ^= 1;
    fs::write(&index, &image).expect("an index whose header fails");
    match level(&store.0, FEED, 5) {
        Ok(Level::Behind(why)) => assert_eq!(why, "its header did not check"),
        Ok(Level::Current(_)) => panic!("a damaged header is never current"),
        Err(refused) => panic!("a damaged header is rebuilt, not refused: {refused}"),
    }
}

#[test]
fn a_staging_name_held_by_a_directory_refuses_the_publish() {
    let store = Scratch::new("staging");
    let dir = store.audit();
    fs::create_dir(dir.join(format!(
        ".{}.quarantine.{}.staging",
        FEED.as_str(),
        std::process::id()
    )))
    .expect("a directory on the staging name");
    let refused = publish_image(&store.0, FEED, &empty_image()).expect_err("refused");
    assert!(
        matches!(
            refused,
            QuarantineError::Io {
                operation: "create the index staging file",
                kind: io::ErrorKind::AlreadyExists,
                ..
            }
        ),
        "{refused:?}"
    );
    assert!(!index_path(&store.0, FEED).exists());
}

#[test]
fn two_months_of_one_instrument_list_every_cause_once() {
    let store = Scratch::new("summary");
    let april = YearMonth::new(2018, 4).expect("2018-04");
    for (month, cause, stamps) in [
        (march(), Cause::ZeroPrice, &[11_i64][..]),
        (april, Cause::ForeignLevel, &[12, 13][..]),
    ] {
        record(
            &store.0,
            &Record {
                path: minute("QFIXS", month),
                scope: Scope::Dropped,
                cause,
                stamps,
                bars: stamps.len() as u64,
                revision: 1,
                source_digest: [0; 32],
                who: "test",
                reason: "invented",
            },
        )
        .expect("recorded");
    }
    let summary = Summary::load(&store.0, FEED).expect("a summary");
    assert!(!summary.is_empty());
    let row = summary
        .instrument("NSE", "CASH", "QFIXS")
        .expect("the instrument");
    assert_eq!(row.months, 2);
    assert_eq!(row.whole_months, 0);
    assert_eq!(row.bars, 3);
    assert_eq!(row.causes, vec![Cause::ForeignLevel, Cause::ZeroPrice]);
    assert!(summary.instrument("NSE", "CASH", "QFIXT").is_none());
    assert_eq!(
        summary.month(minute("QFIXS", april)).map(|entry| entry.bars),
        Some(2)
    );
    let snapshot = Snapshot::load(&store.0, FEED).expect("a snapshot");
    assert_eq!(snapshot.len(), 2);
    assert!(!snapshot.is_empty());
}

#[test]
fn a_store_refusal_reaches_apply_as_a_repair_refusal_in_its_own_words() {
    let refused: ApplyError = StoreError::NotABarPath {
        found: FileKind::Bars,
    }
    .into();
    let direct = RepairError::Store(StoreError::NotABarPath {
        found: FileKind::Bars,
    });
    assert_eq!(refused.to_string(), direct.to_string());
    assert!(matches!(
        refused,
        ApplyError::Repair(RepairError::Store(StoreError::NotABarPath { .. }))
    ));
}
