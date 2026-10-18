//! Tests of the FIB run files v1 (FIB-8; D-6444).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]

use std::fs;
use std::io::{Read as _, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use indicators::fibflag::types::{BaseKind, BaseSnapshot, Candidate, EntryStyle};
use runner::fib::types::{
    ExitReason, ExitRule, SlotKey, SlotOutcome, TradeFlag, TradeFlags, TradeSize,
};
use vocab::fib_cube::Search;

use super::*;

/// A fresh scratch directory, removed on drop.
pub(crate) struct Scratch(pub(crate) PathBuf);

impl Scratch {
    pub(crate) fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "brutex-fib8-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const RUN: [u8; DIGEST] = [0xBB; DIGEST];
const SCHEMA: [u8; DIGEST] = [0xAA; DIGEST];

fn scope_of(kind: ScopeKind) -> Scope {
    match kind {
        ScopeKind::StockMonth => Scope::StockMonth {
            series: SeriesId(17),
            year: 2091,
            month: 4,
        },
        ScopeKind::Month => Scope::Month {
            year: 2091,
            month: 4,
        },
        ScopeKind::Partition => Scope::Partition { partition: 9 },
        ScopeKind::PartitionMonth => Scope::PartitionMonth {
            partition: 9,
            year: 2091,
            month: 4,
        },
        ScopeKind::Run => Scope::Run,
    }
}

fn header(kind: Kind, stride: u32) -> Header {
    Header {
        kind,
        record_version: 1,
        stride,
        schema_digest: SCHEMA,
        run_identity: RUN,
        scope: scope_of(kind.scope()),
        profile: b"synthetic profile".to_vec(),
    }
}

/// `n` synthetic records of `stride` bytes, each distinct.
fn records(n: u64, stride: u32) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| {
            (0..stride)
                .map(|b| (i.wrapping_mul(31).wrapping_add(u64::from(b) * 7) % 251) as u8)
                .collect()
        })
        .collect()
}

fn write(path: &Path, header: &Header, records: &[Vec<u8>]) -> Result<Written, FileError> {
    write_file(path, header, records.iter().map(|r| Ok(r.as_slice())))
}

fn candidate(n: u16) -> CandidateRecord {
    CandidateRecord {
        candidate: Candidate {
            stock_slot: n,
            ist_day: 40_000 + i64::from(n),
            tf_minutes: 7,
            decision_close_ts_micros: 3_456_789_012_000_000 + i64::from(n),
            kind: BaseKind::Micro3,
            style: EntryStyle::Retest,
            direction: Search::Short,
            base: BaseSnapshot {
                kind: BaseKind::Micro3,
                direction: Search::Short,
                high: 987_654_321,
                low: 987_600_001,
                candles: 3,
                first_close_ts_micros: 3_456_789_000_000_000,
                last_close_ts_micros: 3_456_789_011_000_000,
            },
        },
        coords: [3; ROW_COUNT],
    }
}

fn deep(n: u32) -> DeepRecord {
    DeepRecord {
        ordinal: n,
        slot: SlotKey {
            exit: ExitRule {
                exit_set_version: 2,
                variant: 5,
            },
            size: TradeSize::Large,
        },
        outcome: SlotOutcome {
            filled_qty: 123,
            entry_value_paisa: -1_234_567_890_123,
            exit_value_paisa: 9_876_543_210_987,
            risk_per_share_paisa: 4_321,
            first_entry_second: 34_000,
            last_entry_second: 34_001,
            exit_second: 54_545,
            exit_reason: ExitReason::TrailingTakeProfit,
            mfe_to_exit_paisa: 11,
            mae_to_exit_paisa: -22,
            mfe_to_horizon_paisa: 33,
            mae_to_horizon_paisa: -44,
            flags: TradeFlags::NONE.with(TradeFlag::GapThroughStop),
        },
    }
}

/// EVERY FILE KIND ROUND-TRIPS, several across more than one block.
#[test]
fn every_file_kind_round_trips() {
    let scratch = Scratch::new("kinds");
    for (at, kind) in Kind::ALL.iter().enumerate() {
        let stride = kind.stride().unwrap_or(24);
        let n = if at % 4 == 0 {
            block_records(stride) * 2 + 3
        } else {
            u64::try_from(at).unwrap() + 1
        };
        let body = records(n, stride);
        let path = scratch.0.join(format!("kind-{at}.fib"));
        let written = write(&path, &header(*kind, stride), &body).unwrap();
        assert_eq!(written.records, n, "{}", kind.name());
        assert_eq!(written.path, path);
        assert_eq!(written.cleared, None);
        let mut file = FibFile::open(&path).unwrap();
        assert_eq!(file.records(), n);
        assert_eq!(file.sealed().header, header(*kind, stride));
        assert_eq!(file.sealed().block_records, block_records(stride));
        for (i, record) in body.iter().enumerate() {
            assert_eq!(
                &file.record(i as u64).unwrap(),
                record,
                "{} {i}",
                kind.name()
            );
        }
        let verified = verify_whole(&path).unwrap();
        assert_eq!(verified.body_digest, written.body_digest);
        assert_eq!(verified.sealed, *file.sealed());
        assert!(!scratch.0.join(format!("kind-{at}.fib.tmp")).exists());
    }
}

#[test]
fn every_fib_8_record_round_trips_through_its_file() {
    let scratch = Scratch::new("records");
    let closes = vec![
        CloseRecord::Minute {
            day: 30_000,
            slot: 0,
            close: MinuteClose::Real(123_456_789),
        },
        CloseRecord::Minute {
            day: 30_000,
            slot: 1,
            close: MinuteClose::Stale(123_456_789),
        },
        CloseRecord::Minute {
            day: 30_000,
            slot: 2,
            close: MinuteClose::Absent,
        },
        CloseRecord::Previous {
            day: 30_000,
            close: 98_765_431,
            from_day: 29_999,
            regime_last_minute: 929,
            verdict: CloseVerdict::Accepted,
        },
        CloseRecord::Previous {
            day: 30_001,
            close: 98_765_431,
            from_day: 29_998,
            regime_last_minute: 914,
            verdict: CloseVerdict::Refused(0),
        },
        CloseRecord::NoPrevious { day: 30_002 },
    ];
    let cells = vec![
        ReferenceCell::Exact {
            close: 2_222_222,
            previous_close: 2_111_111,
        },
        ReferenceCell::NoPrevious { close: 3_333_333 },
        ReferenceCell::Absent(0),
        ReferenceCell::Absent(4),
    ];
    let candidates: Vec<CandidateRecord> = (0..5).map(candidate).collect();
    let rows: Vec<RowRecord> = (0..5)
        .map(|n| RowRecord {
            series: SeriesId(17),
            year: 2091,
            month: 4,
            ordinal: n,
            coords: [u8::try_from(n).unwrap(); ROW_COUNT],
        })
        .collect();
    let deeps: Vec<DeepRecord> = (0..4).map(deep).collect();
    let lock = ExamLock {
        from_month: month_key(2090, 11),
        to_month: month_key(2091, 10),
        last_covered_day: 44_444,
        coverage_digest: [7; DIGEST],
    };
    macro_rules! round_trip {
        ($records:expr, $scope:expr, $name:literal) => {{
            let path = scratch.0.join($name);
            write_records(&path, RUN, SCHEMA, $scope, Vec::new(), &$records).unwrap();
            let mut file = FibFile::open(&path).unwrap();
            for (i, record) in $records.iter().enumerate() {
                assert_eq!(&read_like(&mut file, i as u64, record), record, $name);
            }
        }};
    }
    round_trip!(closes, scope_of(ScopeKind::StockMonth), "closes.fib");
    round_trip!(cells, scope_of(ScopeKind::Month), "reference.fib");
    round_trip!(
        candidates,
        scope_of(ScopeKind::StockMonth),
        "candidates.fib"
    );
    round_trip!(rows, scope_of(ScopeKind::PartitionMonth), "rows.fib");
    round_trip!(deeps, scope_of(ScopeKind::StockMonth), "deep.fib");
    round_trip!(
        [MarksWord(5), MarksWord(u64::MAX)],
        scope_of(ScopeKind::StockMonth),
        "marks.fib"
    );
    round_trip!([lock], Scope::Run, "lock.fib");
    // Every arm of every decode refuses bytes that are no record of its kind.
    let mut bad = Put::default();
    bad.u8(u8::MAX);
    bad.bytes(&[0; 29]);
    assert!(
        CloseRecord::decode(&bad.0)
            .unwrap_err()
            .starts_with("not a closes record")
    );
    let mut bad = Put::default();
    bad.u8(1);
    bad.i64(5);
    bad.i64(6);
    assert!(
        ReferenceCell::decode(&bad.0)
            .unwrap_err()
            .starts_with("not a reference cell")
    );
    assert!(
        MarksWord::decode(&[0; 9])
            .unwrap_err()
            .starts_with("trailing")
    );
    assert!(MarksWord::decode(&[0; 7]).unwrap_err().starts_with("short"));
    let mut wrong = Put::default();
    candidate(1).encode(&mut wrong);
    wrong.0[20] = 9;
    assert!(
        CandidateRecord::decode(&wrong.0)
            .unwrap_err()
            .starts_with("not a base kind")
    );
    let mut wrong = Put::default();
    candidate(1).encode(&mut wrong);
    wrong.0[21] = 9;
    assert!(
        CandidateRecord::decode(&wrong.0)
            .unwrap_err()
            .starts_with("not an entry style")
    );
    let mut wrong = Put::default();
    candidate(1).encode(&mut wrong);
    wrong.0[22] = 9;
    assert!(
        CandidateRecord::decode(&wrong.0)
            .unwrap_err()
            .starts_with("not a direction")
    );
    let mut wrong = Put::default();
    deep(1).encode(&mut wrong);
    wrong.0[10] = 9;
    assert!(
        DeepRecord::decode(&wrong.0)
            .unwrap_err()
            .starts_with("not a trade size")
    );
    let mut wrong = Put::default();
    deep(1).encode(&mut wrong);
    let reason_at = 4 + 4 + 2 + 1 + 8 + 16 + 16 + 8 + 4 + 4 + 4;
    wrong.0[reason_at] = 9;
    assert!(
        DeepRecord::decode(&wrong.0)
            .unwrap_err()
            .starts_with("not an exit reason")
    );
    // Every exit reason, size and flag value survives.
    for (n, reason) in [
        ExitReason::StopLoss,
        ExitReason::TrailingStopLoss,
        ExitReason::TrailingTakeProfit,
        ExitReason::TakeProfit,
        ExitReason::Time,
        ExitReason::Forced,
    ]
    .into_iter()
    .enumerate()
    {
        let mut record = deep(u32::try_from(n).unwrap());
        record.outcome.exit_reason = reason;
        record.slot.size = if n % 2 == 0 {
            TradeSize::Small
        } else {
            TradeSize::Large
        };
        let mut out = Put::default();
        record.encode(&mut out);
        assert_eq!(DeepRecord::decode(&out.0).unwrap(), record);
    }
    for (kind, style) in [
        (BaseKind::Micro2, EntryStyle::Close),
        (BaseKind::Rest, EntryStyle::Break),
    ] {
        let mut record = candidate(2);
        record.candidate.kind = kind;
        record.candidate.style = style;
        record.candidate.direction = Search::Long;
        let mut out = Put::default();
        record.encode(&mut out);
        assert_eq!(CandidateRecord::decode(&out.0).unwrap(), record);
    }
}

/// THE HEADER, BYTE FOR BYTE.
#[test]
fn the_header_is_byte_golden() {
    let golden = Header {
        kind: Kind::Closes,
        record_version: 1,
        stride: CloseRecord::stride(),
        schema_digest: [0x11; DIGEST],
        run_identity: [0x22; DIGEST],
        scope: Scope::StockMonth {
            series: SeriesId(0x0102_0304),
            year: 2091,
            month: 7,
        },
        profile: b"P".to_vec(),
    };
    let bytes = header_bytes(&golden, 0x0A0B).unwrap();
    // The closes record: tag 1, day 8, slot 2, close 8, from day 8, regime 2,
    // verdict 1.
    assert_eq!(CloseRecord::stride(), 30);
    let mut expected: Vec<u8> = Vec::new();
    expected.extend_from_slice(b"BTXFIBF1"); // 0..8 magic
    expected.extend_from_slice(&[1, 0, 0, 0]); // 8..12 format version
    expected.extend_from_slice(&[1, 0]); // 12..14 kind code: closes
    expected.extend_from_slice(&[1, 0]); // 14..16 record version
    expected.extend_from_slice(&[1, 0, 0, 0]); // 16..20 FIB_CUBE_VERSION
    expected.extend_from_slice(&[30, 0, 0, 0]); // 20..24 stride
    expected.extend_from_slice(&[0x88, 0x08, 0, 0, 0, 0, 0, 0]); // 24..32 records per block: 65,536 / 30 = 2,184
    expected.extend_from_slice(&[0x0B, 0x0A, 0, 0, 0, 0, 0, 0]); // 32..40 record count
    expected.extend_from_slice(&[0x11; 32]); // 40..72 schema digest
    expected.extend_from_slice(&[0x22; 32]); // 72..104 run identity
    expected.extend_from_slice(&[0, 4, 3, 2, 1, 0x2B, 0x08, 7, 0, 0, 0, 0]); // 104..116 scope: stock-month, series, 2091, 7, no partition
    expected.extend_from_slice(&[1, 0, 0, 0]); // 116..120 profile length
    expected.push(b'P'); // 120 profile
    assert_eq!(fixed_header_len(), 120);
    let digest = brutex_core::blake3::hash(&expected);
    expected.extend_from_slice(&digest); // 121..153 header digest
    assert_eq!(bytes, expected);
    // The digest itself, pinned: any change of the layout changes it.
    let hex: String = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex, GOLDEN_HEADER_DIGEST_PREFIX,
        "the header layout changed"
    );
    assert_eq!(footer_len(), 8 + 32 + 8);
    assert_eq!(MAGIC, b"BTXFIBF1");
    assert_eq!(FOOTER_MAGIC, b"BTXFIBE1");
    assert_eq!(FIB_FILE_BLOCK_BYTES, 65_536);
    assert_eq!(block_records(30), 2_184);
    assert_eq!(
        block_records(1 << 20),
        1,
        "a block holds at least one record"
    );
}
const GOLDEN_HEADER_DIGEST_PREFIX: &str = "f652e8802fbb6478";

#[test]
fn a_truncated_or_padded_file_is_detected() {
    let scratch = Scratch::new("truncated");
    let path = scratch.0.join("t.fib");
    let body = records(10, 24);
    write(&path, &header(Kind::Downside, 24), &body).unwrap();
    let length = fs::metadata(&path).unwrap().len();
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(length - 1).unwrap();
    let cut = FibFile::open(&path).unwrap_err();
    assert_eq!(cut.word, "truncated", "{cut}");
    assert!(cut.detail.contains("its header implies"));
    file.set_len(length + 1).unwrap();
    assert_eq!(FibFile::open(&path).unwrap_err().word, "truncated");
    file.set_len(40).unwrap();
    let inside = FibFile::open(&path).unwrap_err();
    assert_eq!(inside.word, "truncated");
    assert!(inside.detail.contains("holds no whole header"), "{inside}");
    let fixed = u64::try_from(fixed_header_len()).unwrap();
    file.set_len(fixed + 3).unwrap();
    let in_profile = FibFile::open(&path).unwrap_err();
    assert_eq!(in_profile.word, "truncated");
    assert!(
        in_profile.detail.contains("ends inside its header"),
        "{in_profile}"
    );
    let missing = FibFile::open(&scratch.0.join("absent.fib")).unwrap_err();
    assert_eq!(missing.word, "io");
}

/// A WRITE THAT FAILS PART WAY LEAVES NO VISIBLE HALF FILE.
#[test]
fn a_failed_write_leaves_no_visible_half_file() {
    let scratch = Scratch::new("failed");
    let path = scratch.0.join("f.fib");
    let temporary = scratch.0.join("f.fib.tmp");
    let body = records(50, 24);
    let failing = body.iter().enumerate().map(|(i, r)| {
        if i == 30 {
            Err("the producer stopped".to_owned())
        } else {
            Ok(r.as_slice())
        }
    });
    let refused = write_file(&path, &header(Kind::Pairs, 24), failing).unwrap_err();
    assert_eq!(refused.word, "producer");
    assert_eq!(refused.detail, "record 30: the producer stopped");
    assert!(!path.exists() && !temporary.exists());
    let short: Vec<Vec<u8>> = vec![vec![0; 24], vec![0; 23]];
    let refused = write(&path, &header(Kind::Pairs, 24), &short).unwrap_err();
    assert_eq!(refused.word, "record_length");
    assert!(!path.exists() && !temporary.exists());
    // A missing directory: the staging door refuses and nothing is made.
    let nowhere = scratch.0.join("no").join("such").join("f.fib");
    let refused = write(&nowhere, &header(Kind::Pairs, 24), &body).unwrap_err();
    assert_eq!(refused.word, "io");
    assert!(!nowhere.exists());
    // A sealed file is never overwritten.
    write(&path, &header(Kind::Pairs, 24), &body).unwrap();
    let before = fs::read(&path).unwrap();
    let again = write(&path, &header(Kind::Pairs, 24), &records(3, 24)).unwrap_err();
    assert_eq!(again.word, "already_sealed");
    assert_eq!(fs::read(&path).unwrap(), before);
    // A dead writer's leftover temporary is cleared and reported.
    let other = scratch.0.join("g.fib");
    fs::write(scratch.0.join("g.fib.tmp"), b"left by a dead writer").unwrap();
    let written = write(&other, &header(Kind::Pairs, 24), &body).unwrap();
    assert_eq!(written.cleared, Some(21));
    verify_whole(&other).unwrap();
}

/// A sink that refuses its `fail_at`-th operation.
struct Failing {
    inner: std::io::Cursor<Vec<u8>>,
    calls: usize,
    fail_at: usize,
}

impl Failing {
    fn tick(&mut self) -> std::io::Result<()> {
        self.calls += 1;
        if self.calls == self.fail_at {
            Err(std::io::Error::other("the disk refused"))
        } else {
            Ok(())
        }
    }
}

impl Write for Failing {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.tick()?;
        self.inner.write(bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.tick()?;
        self.inner.flush()
    }
}

impl Seek for Failing {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        self.tick()?;
        self.inner.seek(to)
    }
}

/// Every write step of the fill names its refusal, and with no refusal the
/// sink holds exactly the file `write_file` seals.
#[test]
fn every_step_of_the_fill_refuses_by_name() {
    let body = records(3, 24);
    let head = header(Kind::Pairs, 24);
    let actions = [
        "write the header of",
        "write a record of",
        "write a record of",
        "write a record of",
        "write the block table of",
        "write the footer of",
        "seek to the header of",
        "rewrite the header of",
        "flush",
    ];
    for (at, action) in actions.iter().enumerate() {
        let mut sink = Failing {
            inner: std::io::Cursor::new(Vec::new()),
            calls: 0,
            fail_at: at + 1,
        };
        let refused = fill(
            &mut sink,
            Path::new("/scratch/x.tmp"),
            &head,
            body.iter().map(|r| Ok(r.as_slice())),
        )
        .unwrap_err();
        assert_eq!(refused.word, "io");
        assert_eq!(
            refused.detail,
            format!("{action} /scratch/x.tmp: the disk refused"),
            "{at}"
        );
    }
    let mut sink = Failing {
        inner: std::io::Cursor::new(Vec::new()),
        calls: 0,
        fail_at: usize::MAX,
    };
    let filled = fill(
        &mut sink,
        Path::new("/scratch/x.tmp"),
        &head,
        body.iter().map(|r| Ok(r.as_slice())),
    )
    .unwrap();
    let scratch = Scratch::new("fill");
    let path = scratch.0.join("x.fib");
    let written = write(&path, &head, &body).unwrap();
    assert_eq!(sink.inner.into_inner(), fs::read(&path).unwrap());
    assert_eq!(filled.file_digest, written.file_digest);
    assert_eq!(filled.body_digest, written.body_digest);
    // The directory sync names its refusal.
    let refused = sync_directory(&scratch.0.join("gone").join("x.fib")).unwrap_err();
    assert_eq!(refused.word, "io");
    assert!(
        refused.detail.starts_with("sync the directory"),
        "{refused}"
    );
    sync_directory(Path::new("relative-name.fib")).unwrap();
}

/// FM-F091: flip a byte in block 7. Open succeeds, block 0 reads, block 7 is
/// refused by name and never served; the whole-file check refuses too.
#[test]
fn a_flipped_byte_refuses_its_block_only() {
    let scratch = Scratch::new("flip");
    let path = scratch.0.join("m.fib");
    let words = MarksWord::stride();
    let per_block = block_records(words);
    let n = per_block * 9;
    let body = records(n, words);
    write(&path, &header(Kind::Marks, words), &body).unwrap();
    let header_len = FibFile::open(&path).unwrap().sealed().header_len;
    let at = header_len + 7 * per_block * u64::from(words) + 5;
    let mut file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    file.seek(SeekFrom::Start(at)).unwrap();
    let mut byte = [0];
    file.read_exact(&mut byte).unwrap();
    file.seek(SeekFrom::Start(at)).unwrap();
    file.write_all(&[byte[0] ^ 0x5A]).unwrap();
    drop(file);
    let mut handle = FibFile::open(&path).unwrap();
    assert_eq!(handle.record(0).unwrap(), body[0]);
    assert_eq!(
        handle.record(per_block * 8).unwrap(),
        body[(per_block * 8) as usize]
    );
    let refused = handle.record(per_block * 7 + 3).unwrap_err();
    assert_eq!(refused.word, "block_corrupt");
    assert!(refused.detail.starts_with("block 7 of "), "{refused}");
    // Never served on a second ask either.
    assert_eq!(
        handle.record(per_block * 7).unwrap_err().word,
        "block_corrupt"
    );
    assert_eq!(verify_whole(&path).unwrap_err().word, "block_corrupt");
    let outside = handle.record(n).unwrap_err();
    assert_eq!(outside.word, "record_out_of_range");
}

/// Reader bytes per request stay at header + one block + its checksum, at
/// three file sizes spanning 100x.
#[test]
fn reader_bytes_per_request_stay_header_page_and_block() {
    let scratch = Scratch::new("bytes");
    let stride = MarksWord::stride();
    let per_block = block_records(stride);
    for blocks in [1, 10, 100] {
        let path = scratch.0.join(format!("b{blocks}.fib"));
        let n = per_block * blocks;
        let body = records(n, stride);
        write(&path, &header(Kind::Marks, stride), &body).unwrap();
        let mut handle = FibFile::open(&path).unwrap();
        let header_len = handle.sealed().header_len;
        assert_eq!(
            handle.bytes_read(),
            header_len,
            "open reads only the header"
        );
        let last = n - 1;
        assert_eq!(handle.record(last).unwrap(), body[last as usize]);
        let block = per_block * u64::from(stride);
        assert_eq!(
            handle.bytes_read(),
            header_len + block + 32,
            "{blocks} blocks"
        );
        // The same block again is served from the verified page.
        assert_eq!(handle.record(last - 1).unwrap(), body[(last - 1) as usize]);
        assert_eq!(handle.bytes_read(), header_len + block + 32);
        if blocks > 1 {
            // A verified block no longer cached costs one record.
            handle.record(0).unwrap();
            handle.record(last).unwrap();
            assert_eq!(
                handle.bytes_read(),
                header_len + 2 * (block + 32) + u64::from(stride)
            );
        }
    }
}

#[test]
fn every_header_refusal_is_named() {
    let scratch = Scratch::new("header");
    let path = scratch.0.join("h.fib");
    let body = records(4, 24);
    write(&path, &header(Kind::Census, 24), &body).unwrap();
    let pristine = fs::read(&path).unwrap();
    let patched = |at: usize, value: u8| {
        let mut bytes = pristine.clone();
        bytes[at] = value;
        fs::write(&path, &bytes).unwrap();
        FibFile::open(&path).unwrap_err()
    };
    assert_eq!(patched(0, b'X').word, "bad_magic");
    assert_eq!(patched(8, 2).word, "bad_version");
    assert_eq!(patched(12, 200).word, "unknown_kind");
    assert_eq!(patched(16, 2).word, "bad_version", "cube version");
    assert_eq!(patched(20, 25).word, "bad_block_length");
    assert_eq!(patched(28, 3).word, "bad_block_length");
    assert_eq!(
        patched(44, 0x5A).word,
        "header_digest",
        "a schema digest byte"
    );
    assert_eq!(patched(104, 9).word, "bad_scope", "an unknown scope byte");
    assert_eq!(
        patched(105, 1).word,
        "bad_scope",
        "a run scope carries a series"
    );
    assert_eq!(patched(120, b'X').word, "header_digest", "a profile byte");
    fs::write(&path, &pristine).unwrap();
    assert!(FibFile::open(&path).is_ok());
    // Stride and scope are checked against the kind on both sides.
    let zero = write(&scratch.0.join("z.fib"), &header(Kind::Census, 0), &[]).unwrap_err();
    assert_eq!(zero.word, "stride_zero");
    let wrong = write(&scratch.0.join("w.fib"), &header(Kind::Marks, 9), &[]).unwrap_err();
    assert_eq!(wrong.word, "stride_mismatch");
    let mut scoped = header(Kind::Census, 24);
    scoped.scope = Scope::Month {
        year: 2091,
        month: 1,
    };
    let refused = write(&scratch.0.join("s.fib"), &scoped, &[]).unwrap_err();
    assert_eq!(refused.word, "scope_mismatch");
    // A file read as the wrong record type.
    let mut census = FibFile::open(&path).unwrap();
    let refused = census.read::<MarksWord>(0).unwrap_err();
    assert_eq!(refused.word, "wrong_kind");
    // A record that is not one of its type.
    let marks = scratch.0.join("k.fib");
    let mut bad = header(Kind::Closes, CloseRecord::stride());
    bad.profile.clear();
    write(&marks, &bad, &records(1, CloseRecord::stride())).unwrap();
    let refused = FibFile::open(&marks)
        .unwrap()
        .read::<CloseRecord>(0)
        .unwrap_err();
    assert_eq!(refused.word, "record_decode");
    // Short reads and flags.
    assert!(Take::new(&[1]).u16().unwrap_err().starts_with("short"));
    assert_eq!(flag(0), Ok(false));
    assert_eq!(flag(1), Ok(true));
    assert_eq!(flag(2), Err("not_a_flag: byte 2".to_owned()));
    let error = FileError {
        word: "w",
        detail: "d".to_owned(),
    };
    assert_eq!(error.to_string(), "w: d");
}

#[test]
fn a_footer_or_body_that_disagrees_is_refused_by_the_whole_check() {
    let scratch = Scratch::new("footer");
    let path = scratch.0.join("f.fib");
    let body = records(5, 24);
    write(&path, &header(Kind::Steps, 24), &body).unwrap();
    let pristine = fs::read(&path).unwrap();
    let mut bytes = pristine.clone();
    let end = bytes.len();
    bytes[end - 1] ^= 1;
    fs::write(&path, &bytes).unwrap();
    assert_eq!(verify_whole(&path).unwrap_err().word, "footer");
    let mut bytes = pristine.clone();
    bytes[end - 48] ^= 1;
    fs::write(&path, &bytes).unwrap();
    assert_eq!(verify_whole(&path).unwrap_err().word, "footer", "count");
    let mut bytes = pristine.clone();
    bytes[end - 40] ^= 1;
    fs::write(&path, &bytes).unwrap();
    assert_eq!(verify_whole(&path).unwrap_err().word, "body_digest");
}

/// FM-F088: a mark is the bit of the candidate's ordinal, one record read
/// (timed by `crates/cli/benches/fib.rs`).
#[test]
fn marks_bit_equals_candidate_ordinal() {
    let scratch = Scratch::new("marks");
    let candidates = 1_000;
    let marked: Vec<u64> = (0..candidates).filter(|i| i % 7 == 3 || *i == 64).collect();
    let words = MarksWord::bitset(candidates, &marked);
    assert_eq!(words.len(), 16);
    let path = scratch.0.join("marks.fib");
    write_records(
        &path,
        RUN,
        SCHEMA,
        scope_of(ScopeKind::StockMonth),
        Vec::new(),
        &words,
    )
    .unwrap();
    let mut file = FibFile::open(&path).unwrap();
    for ordinal in 0..candidates {
        assert_eq!(
            is_marked(&mut file, ordinal).unwrap(),
            marked.contains(&ordinal),
            "{ordinal}"
        );
    }
    assert_eq!(MarksWord::position(64), (1, 1));
    assert_eq!(MarksWord::position(63), (0, 1 << 63));
    assert_eq!(MarksWord::bitset(0, &[5]), Vec::<MarksWord>::new());
}

/// FM-F088: a row's outcome index is (candidates file, ordinal), and it finds
/// its candidate.
#[test]
fn row_outcome_index_round_trips() {
    let scratch = Scratch::new("rows");
    let candidates: Vec<CandidateRecord> = (0..40).map(candidate).collect();
    let cpath = scratch.0.join("candidates-17-2091-04.fib");
    write_records(
        &cpath,
        RUN,
        SCHEMA,
        scope_of(ScopeKind::StockMonth),
        Vec::new(),
        &candidates,
    )
    .unwrap();
    let rows: Vec<RowRecord> = [3, 17, 39]
        .iter()
        .map(|ordinal| RowRecord {
            series: SeriesId(17),
            year: 2091,
            month: 4,
            ordinal: *ordinal,
            coords: candidates[*ordinal as usize].coords,
        })
        .collect();
    append_segment(&scratch.0, 9, (2091, 4), RUN, SCHEMA, &rows).unwrap();
    let mut file = FibFile::open(&cpath).unwrap();
    for (i, row) in rows.iter().enumerate() {
        let read = seek_row(&scratch.0, 9, (2091, 4), i as u64).unwrap();
        assert_eq!(read, *row);
        let found: CandidateRecord = file.read(u64::from(read.ordinal)).unwrap();
        assert_eq!(found, candidates[read.ordinal as usize]);
        assert_eq!(found.coords, read.coords);
    }
}

/// FM-F089: a new month appends one segment and one index entry; every old
/// byte and digest is untouched.
#[test]
fn adding_month_appends_segment_only() {
    let scratch = Scratch::new("segments");
    let rows = |n: u32| -> Vec<RowRecord> {
        (0..n)
            .map(|ordinal| RowRecord {
                series: SeriesId(1),
                year: 2091,
                month: 1,
                ordinal,
                coords: [1; ROW_COUNT],
            })
            .collect()
    };
    let first = append_segment(&scratch.0, 2, (2091, 1), RUN, SCHEMA, &rows(5)).unwrap();
    let second = append_segment(&scratch.0, 2, (2091, 2), RUN, SCHEMA, &rows(7)).unwrap();
    assert_eq!((first.offset, first.count), (0, 5));
    assert_eq!((second.offset, second.count), (5, 7));
    let old_segment = fs::read(segment_path(&scratch.0, 2, 2091, 1)).unwrap();
    let index_path = segment_index_path(&scratch.0, 2);
    let old_index = fs::read(&index_path).unwrap();
    let third = append_segment(&scratch.0, 2, (2091, 4), RUN, SCHEMA, &rows(2)).unwrap();
    assert_eq!((third.offset, third.count), (12, 2));
    assert_eq!(
        fs::read(segment_path(&scratch.0, 2, 2091, 1)).unwrap(),
        old_segment
    );
    let new_index = fs::read(&index_path).unwrap();
    assert_eq!(&new_index[..old_index.len()], &old_index[..]);
    assert_eq!(new_index.len() - old_index.len(), SegmentEntry::stride());
    let entries = read_segment_index(&index_path).unwrap();
    assert_eq!(entries, [first, second, third]);
    assert_eq!(entries[1].month_key, month_key(2091, 2));
    assert_eq!(seek_row(&scratch.0, 2, (2091, 4), 1).unwrap().ordinal, 1);
    // A month not after the last is refused.
    let again = append_segment(&scratch.0, 2, (2091, 3), RUN, SCHEMA, &rows(1)).unwrap_err();
    assert_eq!(again.word, "month_not_after");
    // A torn index and a tampered entry are refused by name.
    let mut torn = new_index.clone();
    torn.push(0);
    fs::write(&index_path, &torn).unwrap();
    assert_eq!(
        read_segment_index(&index_path).unwrap_err().word,
        "segment_index_torn"
    );
    let mut tampered = new_index.clone();
    tampered[5] ^= 1;
    fs::write(&index_path, &tampered).unwrap();
    assert_eq!(
        read_segment_index(&index_path).unwrap_err().word,
        "segment_entry_digest"
    );
    let refused = append_segment(&scratch.0, 2, (2091, 9), RUN, SCHEMA, &rows(1)).unwrap_err();
    assert_eq!(refused.word, "segment_entry_digest");
    assert_eq!(
        read_segment_index(&scratch.0.join("none.segments"))
            .unwrap_err()
            .word,
        "io"
    );
    assert_eq!(month_key(2091, 1), 2091 * 12);
}

/// FIB-1 R6: the exam lock is written once; a lock that differs is refused.
#[test]
fn the_exam_lock_is_sealed_once_and_a_differing_lock_is_refused() {
    let scratch = Scratch::new("lock");
    let lock = ExamLock {
        from_month: month_key(2090, 11),
        to_month: month_key(2091, 10),
        last_covered_day: 44_444,
        coverage_digest: [9; DIGEST],
    };
    let digest = seal_exam_lock(&scratch.0, RUN, SCHEMA, lock).unwrap();
    assert_eq!(check_exam_lock(&scratch.0, lock), Ok(lock));
    let moved = ExamLock {
        from_month: lock.from_month + 1,
        ..lock
    };
    let refused = check_exam_lock(&scratch.0, moved).unwrap_err();
    assert_eq!(refused.word, "exam_lock_differs");
    assert!(refused.detail.contains("the exam never moves"));
    let twice = seal_exam_lock(&scratch.0, RUN, SCHEMA, moved).unwrap_err();
    assert_eq!(twice.word, "already_sealed");
    assert_eq!(
        seal_exam_lock(&scratch.0.join("x"), RUN, SCHEMA, lock)
            .unwrap_err()
            .word,
        "io"
    );
    assert_ne!(digest, [0; DIGEST]);
    // A lock of two records, or a corrupt one, is unreadable.
    let other = Scratch::new("lock2");
    write_records(
        &other.0.join(EXAM_LOCK_FILE),
        RUN,
        SCHEMA,
        Scope::Run,
        Vec::new(),
        &[lock, lock],
    )
    .unwrap();
    let refused = check_exam_lock(&other.0, lock).unwrap_err();
    assert_eq!(refused.word, "exam_lock_unreadable");
    assert!(refused.detail.contains("2 records, not one"), "{refused}");
    let empty = Scratch::new("lock3");
    assert_eq!(
        check_exam_lock(&empty.0, lock).unwrap_err().word,
        "exam_lock_unreadable"
    );
}

/// FIB-7's touch-point: the downside kinds take records of one fixed stride.
#[test]
fn downside_records_fixed_stride() {
    let scratch = Scratch::new("downside");
    for kind in [Kind::Downside, Kind::DownsideRefs, Kind::GateRefusalCensus] {
        assert_eq!(kind.stride(), None, "FIB-7 owns the layout");
        assert_eq!(kind.owner(), "FIB-7");
        let stride = 40;
        let path = scratch.0.join(format!("{}.fib", kind.code()));
        let body = records(9, stride);
        write(&path, &header(kind, stride), &body).unwrap();
        let mut file = FibFile::open(&path).unwrap();
        for i in 0..9 {
            assert_eq!(file.record(i).unwrap().len(), 40);
        }
        let mut mixed = body.clone();
        mixed.push(vec![0; 41]);
        let other = scratch.0.join(format!("{}-mixed.fib", kind.code()));
        assert_eq!(
            write(&other, &header(kind, stride), &mixed)
                .unwrap_err()
                .word,
            "record_length"
        );
    }
    assert_eq!(Kind::Downside.name(), "downside v1");
    assert_eq!(Kind::DownsideRefs.name(), "downside refs v1");
    assert_eq!(Kind::GateRefusalCensus.name(), "gate refusal census v2");
}

/// Every v1 kind keeps its code after the downside block was appended: the
/// codes are the positions, pinned, and a file of each decodes.
#[test]
fn v1_kinds_decode_after_block() {
    let codes: Vec<(u16, &str, ScopeKind, &str)> = Kind::ALL
        .iter()
        .map(|kind| (kind.code(), kind.name(), kind.scope(), kind.owner()))
        .collect();
    assert_eq!(
        codes,
        [
            (1, "closes", ScopeKind::StockMonth, "FIB-8"),
            (2, "rank", ScopeKind::Month, "FIB-2"),
            (3, "reference", ScopeKind::Month, "FIB-8"),
            (4, "candidates", ScopeKind::StockMonth, "FIB-8"),
            (5, "rows", ScopeKind::PartitionMonth, "FIB-8"),
            (6, "survivors", ScopeKind::Partition, "FIB-7"),
            (7, "marks", ScopeKind::StockMonth, "FIB-8"),
            (8, "deep", ScopeKind::StockMonth, "FIB-8"),
            (9, "winners", ScopeKind::Run, "FIB-14"),
            (10, "keepers", ScopeKind::Run, "FIB-14"),
            (11, "steps", ScopeKind::Run, "FIB-13"),
            (12, "diagnostics", ScopeKind::Run, "FIB-14"),
            (13, "luck receipt", ScopeKind::Run, "FIB-7"),
            (14, "census", ScopeKind::Run, "FIB-9"),
            (15, "exam lock", ScopeKind::Run, "FIB-8"),
            (16, "side passers", ScopeKind::Partition, "FIB-7"),
            (17, "pairs", ScopeKind::Run, "FIB-7"),
            (18, "histogram checkpoint", ScopeKind::StockMonth, "FIB-2"),
            (19, "seconds summary", ScopeKind::StockMonth, "FIB-9"),
            (20, "downside v1", ScopeKind::Partition, "FIB-7"),
            (21, "downside refs v1", ScopeKind::PartitionMonth, "FIB-7"),
            (22, "gate refusal census v2", ScopeKind::Run, "FIB-7"),
        ]
    );
    for kind in Kind::ALL {
        assert_eq!(Kind::of_code(kind.code()), Some(*kind));
    }
    assert_eq!(Kind::of_code(0), None);
    assert_eq!(Kind::of_code(23), None);
    let strides: Vec<(&str, Option<u32>)> = Kind::ALL
        .iter()
        .filter(|kind| kind.stride().is_some())
        .map(|kind| (kind.name(), kind.stride()))
        .collect();
    assert_eq!(
        strides,
        [
            ("closes", Some(30)),
            ("reference", Some(17)),
            ("candidates", Some(83)),
            ("rows", Some(35)),
            ("marks", Some(8)),
            ("deep", Some(108)),
            ("exam lock", Some(48)),
        ]
    );
    let scopes: Vec<u8> = ScopeKind::ALL.iter().map(|kind| *kind as u8).collect();
    assert_eq!(scopes, [0, 1, 2, 3, 4]);
}

/// NO TYPED NUMBERS: the shipping part of this file holds no numeric
/// literal but 0 and 1.
#[test]
fn files_rs_types_no_number() {
    let source = include_str!("files.rs");
    let typed = vocab::fib_cube::untyped_numbers(source);
    assert!(typed.is_empty(), "{typed:?}");
}

/// Record `index` of `file`, read as the type of `_like`.
fn read_like<R: Record>(file: &mut FibFile, index: u64, _like: &R) -> R {
    file.read(index).unwrap()
}
