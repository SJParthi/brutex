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

//! IA-RK-03..22 (IA-RK-01 and -02 are in `http.rs`) and the matrix rows of
//! §8 Z7 and Z11 for the kept answers
//! (IA-8). Every body here is invented: the prices cannot be mistaken for a
//! real print (CLAUDE.md, no vendor row in the repository).

use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind, OptionSide, Segment};
use brutex_core::price::Paisa;
use brutex_core::vendor::Vendor;
use store::path::{Timeframe, YearMonth};

use super::*;
use crate::fetch::{RawWindow, RowCause};
use crate::session::{Day, Window};

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-keep-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn root(&self) -> &Path {
        &self.0
    }
}

fn nifty() -> InstrumentKey {
    InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()
}

fn option(strike_rupees: i64, side: OptionSide) -> InstrumentKey {
    let mut key = nifty();
    key.segment = Segment::Fno;
    key.kind = Kind::Option {
        expiry: Expiry::new(2019, 6, 27).unwrap(),
        strike: Paisa::from_raw(strike_rupees * 100),
        side,
    };
    key
}

fn pack(key: &InstrumentKey) -> Pack<'_> {
    Pack {
        vendor: Vendor::Zerodha,
        key,
        timeframe: Timeframe::MINUTE_1,
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn landing(from: Day, to: Day) -> Landing<'static> {
    Landing {
        window: Window::new(from, to).unwrap(),
        first_stamp: 1_561_607_100,
        last_stamp: 1_561_629_540,
        rows_decoded: 375,
        rows_landed: 375,
        rows_refused: 0,
        refused: &[],
        run: 7,
        rung: crate::vendor::Granularity::Minute1 as u8,
        rules_version: 1,
        door: Door::FromWindow,
    }
}

fn june() -> Landing<'static> {
    landing(day(2019, 6, 3), day(2019, 6, 28))
}

/// An invented body: JSON-shaped, with prices nobody traded at.
fn invented(tag: u32) -> String {
    format!(
        "{{\"open\":[1.11,{tag}.01],\"high\":[1.12,{tag}.02],\"low\":[1.10,{tag}.00],\
         \"close\":[1.11,{tag}.01],\"volume\":[0,0],\"timestamp\":[1,2]}}"
    )
}

fn answered(text: &str) -> Answered {
    Answered::of(RawWindow::default(), text.to_owned(), 200)
}

fn june_month() -> YearMonth {
    YearMonth::new(2019, 6).unwrap()
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(files_under(&path));
        } else {
            out.push(path);
        }
    }
    out.sort();
    out
}

#[test]
fn a_receipt_round_trips_and_a_flipped_bit_is_refused() {
    let receipt = Receipt {
        asked_from: -1,
        asked_to: i64::MAX,
        first_stamp: 3,
        last_stamp: 4,
        body_offset: u64::MAX,
        deflated_len: 6,
        raw_len: 7,
        rows_decoded: 8,
        rows_landed: 9,
        rows_refused: 10,
        run: 11,
        status: 200,
        rung: 3,
        kept: Kept::Duplicate,
        reason: Reason::DuplicateBody,
        rules_version: 1,
        door: Door::RepairWindow,
        segment: 7,
        digest: [9; 32],
        key_hash: 12,
        body_of: 13,
        refusals_from: 14,
        refusals: 15,
    };
    let bytes = receipt.encode();
    assert_eq!(bytes.len(), 128);
    assert_eq!(Receipt::decode(&bytes), Some(receipt));
    // The offsets of §36, read back by hand.
    assert_eq!(&bytes[56..60], &10_u32.to_le_bytes(), "rows refused at 56");
    assert_eq!(&bytes[60..64], &11_u32.to_le_bytes(), "run at 60");
    assert_eq!(bytes[67], 2, "kept at 67");
    assert_eq!(bytes[70], 2, "door at 70");
    assert_eq!(&bytes[72..104], &[9; 32], "digest at 72");
    for at in [0, 67, 70, 100, 123, 127] {
        let mut bad = bytes;
        bad[at] ^= 0x10;
        assert_eq!(Receipt::decode(&bad), None, "a flipped bit at {at}");
    }
    assert_eq!(Receipt::decode(&bytes[..127]), None, "a short receipt");
    // An unknown code byte with a matching CRC is refused too.
    let mut unknown = bytes;
    unknown[67] = 9;
    let crc = store::crc::crc32c(&unknown[..124]);
    unknown[124..].copy_from_slice(&crc.to_le_bytes());
    assert_eq!(Receipt::decode(&unknown), None);
    for code in 0..=255_u8 {
        if let Some(cause) = cause_of(code) {
            assert_eq!(cause_code(cause), code);
        }
    }
}

#[test]
fn a_month_is_read_in_ist() {
    // 2019-12-31T18:30:00Z is 2020-01-01 00:00 IST.
    assert_eq!(
        month_of_unix(1_577_817_000).unwrap(),
        YearMonth::new(2020, 1).unwrap()
    );
    assert_eq!(
        month_of_unix(1_577_816_999).unwrap(),
        YearMonth::new(2019, 12).unwrap()
    );
    let (from, to) = asked_span(Window::new(day(2020, 1, 1), day(2020, 1, 1)).unwrap());
    assert_eq!((from, to), (1_577_817_000, 1_577_817_000 + 86_399));
    assert_eq!(
        month_of_unix(from).unwrap(),
        YearMonth::new(2020, 1).unwrap()
    );
    let months = months_of(Window::new(day(2019, 11, 15), day(2020, 2, 10)).unwrap()).unwrap();
    assert_eq!(
        months.iter().map(ToString::to_string).collect::<Vec<_>>(),
        ["2019-11", "2019-12", "2020-01", "2020-02"]
    );
}

/// IA-RK-03 (§8 Z11): a full pack is named, never silently dropped.
#[test]
fn a_full_pack_is_named_never_silently_dropped() {
    let scratch = Scratch::new("full");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let month = june_month();
    let filler = |segment: u8, count: u32, body_end: u64| {
        let mut bytes = Vec::new();
        for i in 0..count {
            let mut r = Receipt::decode(
                &Receipt {
                    asked_from: 0,
                    asked_to: 0,
                    first_stamp: 0,
                    last_stamp: 0,
                    body_offset: 0,
                    deflated_len: 0,
                    raw_len: 0,
                    rows_decoded: 0,
                    rows_landed: 0,
                    rows_refused: 0,
                    run: 1,
                    status: 200,
                    rung: 3,
                    kept: Kept::NotKept,
                    reason: Reason::PackFull,
                    rules_version: 1,
                    door: Door::FromWindow,
                    segment,
                    digest: [0; 32],
                    key_hash: 0,
                    body_of: i,
                    refusals_from: 0,
                    refusals: 0,
                }
                .encode(),
            )
            .unwrap();
            if i + 1 == count && body_end > 0 {
                r.kept = Kept::Kept;
                r.reason = Reason::Kept;
                r.body_offset = body_end - 10;
                r.deflated_len = 10;
                r.digest = [7; 32];
            }
            bytes.extend_from_slice(&r.encode());
        }
        fs::write(names(&dir, month, segment).receipts, bytes).unwrap();
    };

    // (a) Segment 0 holds 1,024 receipts: the append ROLLS to segment 1, and
    // the receipt there says so.
    filler(0, MAX_RECEIPTS, 0);
    let text = invented(1);
    let a = answered(&text);
    let outcome = append(scratch.root(), pack(&key), Offered::Answer(&a), &june()).unwrap();
    assert_eq!(outcome.first.segment, 1);
    assert!(outcome.first.rolled, "a roll is named");
    assert_eq!(outcome.first.receipt.kept, Kept::Kept);
    assert_eq!(
        body(scratch.root(), pack(&key), month, &outcome.first.receipt).unwrap(),
        text
    );

    // (b) Segment 1's bodies reach 64 MiB less 10 bytes: rolls again.
    filler(1, 1, MAX_PACK_BYTES);
    let second = answered(&invented(2));
    let rolled = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&second),
        &june(),
    )
    .unwrap();
    assert_eq!((rolled.first.segment, rolled.first.rolled), (2, true));

    // (c) The LAST segment's bodies are at the cap: a receipt says
    // `kept=0 reason=pack_full`, and no byte of the body is written.
    for segment in 2..MAX_SEGMENTS {
        filler(segment, 1, MAX_PACK_BYTES);
    }
    let third = answered(&invented(3));
    let full = append(scratch.root(), pack(&key), Offered::Answer(&third), &june()).unwrap();
    let last = MAX_SEGMENTS - 1;
    assert_eq!(full.first.segment, last);
    assert_eq!(
        (full.first.receipt.kept, full.first.receipt.reason),
        (Kept::NotKept, Reason::PackFull)
    );
    assert!(
        !names(&dir, month, last).bodies.exists(),
        "no body byte written"
    );
    assert_eq!(
        body(scratch.root(), pack(&key), month, &full.first.receipt),
        Err(KeepError::NotKept {
            reason: Reason::PackFull
        })
    );

    // (d) The last segment holds 1,024 receipts: NOTHING is written, and the
    // refusal names the pack.
    filler(last, MAX_RECEIPTS, 0);
    let before: Vec<(PathBuf, u64)> = files_under(&dir)
        .into_iter()
        .map(|p| {
            let len = fs::metadata(&p).unwrap().len();
            (p, len)
        })
        .collect();
    let refused = append(scratch.root(), pack(&key), Offered::Answer(&third), &june());
    let Err(KeepError::ReceiptsFull { pack: named }) = &refused else {
        panic!("{refused:?}");
    };
    assert_eq!(*named, names(&dir, month, last).receipts);
    assert_eq!(refused.as_ref().unwrap_err().word(), "receipts_full");
    assert!(refused.unwrap_err().to_string().contains("nothing written"));
    let after: Vec<(PathBuf, u64)> = files_under(&dir)
        .into_iter()
        .map(|p| {
            let len = fs::metadata(&p).unwrap().len();
            (p, len)
        })
        .collect();
    assert_eq!(before, after, "nothing was written");
}

/// IA-RK-04: a torn body or receipt tail is overwritten by the next append;
/// a reader trusts receipts only; damage before the tail is refused.
#[test]
fn an_interrupted_keep_is_repaired_by_the_next() {
    let scratch = Scratch::new("torn");
    let key = nifty();
    let month = june_month();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    let at = names(&dir, month, 0);
    let one = invented(1);
    let first = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&one)),
        &june(),
    )
    .unwrap()
    .first;
    let end = first.receipt.body_offset + u64::from(first.receipt.deflated_len);

    // A body appended and synced, its receipt never written; and half a
    // receipt.
    OpenOptions::new()
        .append(true)
        .open(&at.bodies)
        .unwrap()
        .write_all(&[0xAB; 999])
        .unwrap();
    OpenOptions::new()
        .append(true)
        .open(&at.receipts)
        .unwrap()
        .write_all(&[0xCD; 57])
        .unwrap();
    let seen = read_segment(&at).unwrap();
    assert_eq!(seen.receipts.len(), 1, "a reader trusts receipts only");
    assert_eq!(seen.torn_receipt_bytes, 57);
    assert_eq!(seen.body_end, end, "the torn body is not vouched for");

    let two = invented(2);
    let second = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&two)),
        &june(),
    )
    .unwrap()
    .first;
    assert_eq!(second.ordinal, 1);
    assert_eq!(
        second.receipt.body_offset, end,
        "the torn body was overwritten"
    );
    assert_eq!(
        fs::metadata(&at.receipts).unwrap().len(),
        2 * RECEIPT_LEN as u64,
        "the torn receipt was overwritten"
    );
    assert_eq!(
        fs::metadata(&at.bodies).unwrap().len(),
        end + u64::from(second.receipt.deflated_len),
        "the file ends where the last receipt says"
    );
    assert_eq!(
        body(scratch.root(), pack(&key), month, &first.receipt).unwrap(),
        one
    );
    assert_eq!(
        body(scratch.root(), pack(&key), month, &second.receipt).unwrap(),
        two
    );

    // A whole-length LAST receipt whose CRC fails (the length grew, the bytes
    // did not land) is torn too, and overwritten.
    OpenOptions::new()
        .append(true)
        .open(&at.receipts)
        .unwrap()
        .write_all(&[0; RECEIPT_LEN])
        .unwrap();
    assert_eq!(read_segment(&at).unwrap().receipts.len(), 2);
    let three = invented(3);
    let third = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&three)),
        &june(),
    )
    .unwrap()
    .first;
    assert_eq!(third.ordinal, 2);
    assert_eq!(
        body(scratch.root(), pack(&key), month, &third.receipt).unwrap(),
        three
    );

    // A bad CRC BEFORE the last receipt is damage: refused by name, nothing
    // overwritten.
    let mut bytes = fs::read(&at.receipts).unwrap();
    bytes[5] ^= 1;
    fs::write(&at.receipts, &bytes).unwrap();
    let damaged = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(4))),
        &june(),
    );
    assert!(
        matches!(damaged, Err(KeepError::Damaged { ordinal: 0, .. })),
        "{damaged:?}"
    );
    assert_eq!(
        fs::read(&at.receipts).unwrap(),
        bytes,
        "damage is never overwritten"
    );

    // A body whose bytes changed under its receipt is refused, not served.
    let scratch = Scratch::new("torn-body");
    let kept = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&one)),
        &june(),
    )
    .unwrap()
    .first;
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    let bodies = names(&dir, month, 0).bodies;
    let mut raw = fs::read(&bodies).unwrap();
    raw[0] ^= 0xFF;
    fs::write(&bodies, raw).unwrap();
    assert!(matches!(
        body(scratch.root(), pack(&key), month, &kept.receipt),
        Err(KeepError::BodyMismatch { .. })
    ));
}

/// IA-RK-05: two contracts of one underlying keep two packs, beside spot.
#[test]
fn each_contract_keeps_its_own_pack() {
    let scratch = Scratch::new("contracts");
    let spot = nifty();
    let call = option(12_000, OptionSide::Call);
    let put = option(12_000, OptionSide::Put);
    let other = option(11_900, OptionSide::Call);
    // The same path for every other kind of selection: BANKNIFTY spot, an
    // F&O share's cash series and a far, illiquid NIFTY strike.
    let bank = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap();
    let share = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
    let far = option(9_000, OptionSide::Put);
    let mut dirs = Vec::new();
    for (i, key) in [&spot, &call, &put, &other, &bank, &share, &far]
        .into_iter()
        .enumerate()
    {
        let text = invented(u32::try_from(i).unwrap() + 10);
        let outcome = append(
            scratch.root(),
            pack(key),
            Offered::Answer(&answered(&text)),
            &june(),
        )
        .unwrap();
        assert_eq!(outcome.first.ordinal, 0, "a pack of its own");
        let dir = pack_dir(scratch.root(), pack(key)).unwrap();
        assert_eq!(
            body(
                scratch.root(),
                pack(key),
                june_month(),
                &outcome.first.receipt
            )
            .unwrap(),
            text
        );
        dirs.push(dir);
    }
    let shown: Vec<String> = dirs
        .iter()
        .map(|d| {
            d.strip_prefix(scratch.root())
                .unwrap()
                .display()
                .to_string()
        })
        .collect();
    assert_eq!(shown[0], "responses/zerodha/NSE/INDEX/NIFTY/1min");
    assert!(
        shown[1].starts_with("responses/zerodha/NSE/FNO/NIFTY/2019-06-27-"),
        "{shown:?}"
    );
    assert!(
        shown[1].ends_with("-CE/1min") && shown[2].ends_with("-PE/1min"),
        "{shown:?}"
    );
    assert_eq!(shown[4], "responses/zerodha/NSE/INDEX/BANKNIFTY/1min");
    assert_eq!(shown[5], "responses/zerodha/NSE/CASH/RELIANCE/1min");
    assert!(shown[6].ends_with("-900000-PE/1min"), "{shown:?}");
    let unique: std::collections::BTreeSet<&String> = shown.iter().collect();
    assert_eq!(unique.len(), 7, "seven packs: {shown:?}");
    // The receipt carries the key's hash, so a pack moved under another name
    // is detectable.
    let r = &receipts(scratch.root(), pack(&call), june_month()).unwrap()[0]
        .1
        .receipts[0];
    assert_eq!(
        r.key_hash,
        store::verdict::key_hash(call.to_string().as_bytes())
    );
    assert_ne!(
        r.key_hash,
        store::verdict::key_hash(put.to_string().as_bytes())
    );
}

/// IA-RK-06: a re-pull that returns the same body writes a receipt only.
#[test]
fn a_repeated_body_is_kept_once() {
    let scratch = Scratch::new("dup");
    let key = nifty();
    let month = june_month();
    let text = invented(5);
    let a = answered(&text);
    let first = append(scratch.root(), pack(&key), Offered::Answer(&a), &june())
        .unwrap()
        .first;
    let at = names(&pack_dir(scratch.root(), pack(&key)).unwrap(), month, 0);
    let len = fs::metadata(&at.bodies).unwrap().len();
    let again = append(scratch.root(), pack(&key), Offered::Answer(&a), &june())
        .unwrap()
        .first;
    assert_eq!(
        (
            again.receipt.kept,
            again.receipt.reason,
            again.receipt.body_of
        ),
        (Kept::Duplicate, Reason::DuplicateBody, first.ordinal)
    );
    assert_eq!(again.receipt.kept.code(), 2, "kept=2 on disk");
    assert_eq!(
        fs::metadata(&at.bodies).unwrap().len(),
        len,
        "no body appended"
    );
    assert_eq!(
        body(scratch.root(), pack(&key), month, &again.receipt).unwrap(),
        text
    );
    let other = invented(6);
    let third = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&other)),
        &june(),
    )
    .unwrap();
    assert_eq!(
        third.first.receipt.kept,
        Kept::Kept,
        "a different body is kept"
    );
    assert!(fs::metadata(&at.bodies).unwrap().len() > len);
    assert_eq!(
        read_segment(&at).unwrap().receipts.len(),
        3,
        "every pass on record"
    );

    // A segment that could not take the body a second time still takes its
    // receipt: the duplicate check comes before any roll.
    let mut filler = read_segment(&at).unwrap().receipts[2];
    filler.digest = [3; 32];
    filler.body_offset = MAX_PACK_BYTES - 10;
    filler.deflated_len = 10;
    OpenOptions::new()
        .append(true)
        .open(&at.receipts)
        .unwrap()
        .write_all(&filler.encode())
        .unwrap();
    let repeat = append(scratch.root(), pack(&key), Offered::Answer(&a), &june())
        .unwrap()
        .first;
    assert_eq!(
        (repeat.segment, repeat.rolled, repeat.receipt.kept),
        (0, false, Kept::Duplicate)
    );
    let fresh = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(7))),
        &june(),
    )
    .unwrap()
    .first;
    assert_eq!((fresh.segment, fresh.rolled), (1, true), "a new body rolls");
}

/// IA-RK-07 (IA-8's half): a window landed through either door is kept, and
/// the receipt's rows-refused field and records are the landing's.
#[test]
fn both_landing_doors_keep_and_count_their_refusals() {
    let scratch = Scratch::new("doors");
    let key = nifty();
    let month = june_month();
    let refused = [
        RefusedRow {
            local_second: 1_559_554_500,
            cause: RowCause::OffGrid,
        },
        RefusedRow {
            local_second: 1_559_554_560,
            cause: RowCause::Conflicting,
        },
        RefusedRow {
            local_second: 1_559_554_560,
            cause: RowCause::Conflicting,
        },
    ];
    for (door, tag) in [(Door::FromWindow, 21), (Door::RepairWindow, 22)] {
        let text = invented(tag);
        let mut l = june();
        l.door = door;
        l.rows_landed = 372;
        l.rows_refused = 3;
        l.refused = &refused;
        let placed = append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&text)),
            &l,
        )
        .unwrap()
        .first;
        assert_eq!(placed.receipt.door, door);
        assert_eq!(placed.receipt.rows_refused, 3);
        assert_eq!(placed.receipt.refusals, 3);
        assert_eq!(
            refused_rows(
                scratch.root(),
                pack(&key),
                month,
                placed.ordinal,
                &placed.receipt
            )
            .unwrap(),
            refused,
            "every refused row with its cause (DB-G46)"
        );
    }
    let segment = &receipts(scratch.root(), pack(&key), month).unwrap()[0].1;
    assert_eq!(segment.refusals_end, 6);
    assert_eq!(segment.receipts[1].refusals_from, 3);
    // A refused row's record names its own receipt: one moved under another
    // receipt is damage.
    let wrong = refused_rows(scratch.root(), pack(&key), month, 1, &segment.receipts[0]);
    assert!(matches!(wrong, Err(KeepError::Damaged { .. })), "{wrong:?}");
    // A caller that knows the count but not the rows still records the count.
    let mut counted = june();
    counted.rows_refused = 9;
    let only_count = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(23))),
        &counted,
    )
    .unwrap()
    .first;
    assert_eq!(
        (only_count.receipt.rows_refused, only_count.receipt.refusals),
        (9, 0)
    );
}

/// IA-RK-08 (critic IA-8-2): two landings of one month never interleave a
/// pack: 2 threads, 1,000 appends each, every receipt's body verifies.
#[test]
fn two_landings_of_one_month_never_interleave_a_pack() {
    let scratch = Scratch::new("race");
    let key = nifty();
    let month = june_month();
    std::thread::scope(|scope| {
        for thread in 0..2_u32 {
            let root = scratch.root();
            let key = &key;
            scope.spawn(move || {
                for i in 0..1_000_u32 {
                    let text = invented(thread * 10_000 + i);
                    append(root, pack(key), Offered::Answer(&answered(&text)), &june()).unwrap();
                }
            });
        }
    });
    let segments = receipts(scratch.root(), pack(&key), month).unwrap();
    let total: usize = segments.iter().map(|(_, s)| s.receipts.len()).sum();
    assert_eq!(total, 2_000);
    assert_eq!(segments.len(), 2, "1,024 receipts, then a roll");
    assert_eq!(segments[0].1.receipts.len(), MAX_RECEIPTS as usize);
    let mut seen = std::collections::HashSet::new();
    for (index, segment) in &segments {
        let mut end = 0;
        for receipt in &segment.receipts {
            assert_eq!(receipt.segment, *index);
            assert_eq!(
                receipt.body_offset, end,
                "bodies are contiguous, never interleaved"
            );
            end += u64::from(receipt.deflated_len);
            let text = body(scratch.root(), pack(&key), month, receipt).unwrap();
            assert!(seen.insert(text), "each body once");
        }
    }
}

/// RA-299 (IA-RK-09): the head of a refused answer is kept with its status.
#[test]
fn a_refused_answer_keeps_its_head() {
    let scratch = Scratch::new("refused");
    let key = nifty();
    // A login wall, invented; a two-byte character straddles the cut.
    let mut page = "<html><body>please sign in".to_owned();
    while page.len() < REFUSED_HEAD_BYTES - 1 {
        page.push('.');
    }
    page.push('é');
    page.push_str(&"x".repeat(10_000));
    let refused = RefusedAnswer::of(200, &page);
    assert_eq!(
        refused.head.len(),
        REFUSED_HEAD_BYTES - 1,
        "cut at a character"
    );
    assert_eq!(refused.len, page.len() as u64);
    let placed = append(
        scratch.root(),
        pack(&key),
        Offered::Refused(&refused),
        &june(),
    )
    .unwrap()
    .first;
    assert_eq!(
        (
            placed.receipt.kept,
            placed.receipt.reason,
            placed.receipt.status
        ),
        (Kept::RefusedHead, Reason::RefusedAnswer, 200)
    );
    assert_eq!(
        placed.receipt.first_stamp,
        i64::try_from(page.len()).unwrap(),
        "the whole length"
    );
    assert_eq!(
        body(scratch.root(), pack(&key), june_month(), &placed.receipt).unwrap(),
        *refused.head
    );
    // Empty and single-character refusals are kept as what they are.
    for text in ["", "x"] {
        let r = RefusedAnswer::of(503, text);
        let p = append(scratch.root(), pack(&key), Offered::Refused(&r), &june()).unwrap();
        assert_eq!(
            body(scratch.root(), pack(&key), june_month(), &p.first.receipt).unwrap(),
            text
        );
    }
    // The same refusal twice is one head and two receipts.
    let again = append(
        scratch.root(),
        pack(&key),
        Offered::Refused(&refused),
        &june(),
    )
    .unwrap();
    assert_eq!(again.first.receipt.kept, Kept::Duplicate);
}

/// §8 Z7: a body over `MAX_RESPONSE_BYTES` is refused and nothing is kept.
#[test]
fn an_oversized_body_is_refused_and_nothing_is_kept() {
    let scratch = Scratch::new("oversize");
    let key = nifty();
    let huge = "x".repeat(crate::http::MAX_RESPONSE_BYTES + 1);
    let refused = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&huge)),
        &june(),
    );
    assert_eq!(
        refused,
        Err(KeepError::TooLarge {
            len: crate::http::MAX_RESPONSE_BYTES as u64 + 1
        })
    );
    assert_eq!(refused.unwrap_err().word(), "source_refused");
    assert!(!scratch.root().join(KEEP_ROOT).exists(), "nothing kept");
}

/// A window over four months: the body once, in its first month, and a
/// receipt in each further month that finds it.
#[test]
fn a_window_over_months_keeps_its_body_once() {
    let scratch = Scratch::new("months");
    let key = nifty();
    let text = invented(31);
    let rows = [RefusedRow {
        local_second: 1_577_000_000,
        cause: RowCause::Price,
    }];
    let mut l = landing(day(2019, 11, 15), day(2020, 2, 10));
    l.rung = crate::vendor::Granularity::Day1 as u8;
    l.refused = &rows;
    let day_pack = Pack {
        timeframe: Timeframe::DAY_1,
        ..pack(&key)
    };
    let outcome = append(
        scratch.root(),
        day_pack,
        Offered::Answer(&answered(&text)),
        &l,
    )
    .unwrap();
    assert_eq!(outcome.further.len(), 3);
    let dir = pack_dir(scratch.root(), day_pack).unwrap();
    for placed in &outcome.further {
        assert_eq!(placed.receipt.kept, Kept::InFirstMonth);
        assert_eq!(placed.receipt.reason, Reason::LaterMonth);
        assert!(
            !names(&dir, placed.month, 0).bodies.exists(),
            "no second body"
        );
        assert_eq!(
            body(scratch.root(), day_pack, placed.month, &placed.receipt).unwrap(),
            text
        );
        assert_eq!(
            refused_rows(
                scratch.root(),
                day_pack,
                placed.month,
                placed.ordinal,
                &placed.receipt
            )
            .unwrap(),
            rows
        );
    }
    assert_eq!(outcome.further[2].month, YearMonth::new(2020, 2).unwrap());
}

/// Disk full, killed mid-write: a body the disk refuses leaves a receipt
/// `kept=0 reason=storage_full`; a receipt the disk refuses is an error and
/// the next append writes over what it left.
#[test]
fn a_full_disk_is_named_and_the_next_append_recovers() {
    let scratch = Scratch::new("disk");
    let key = nifty();
    let month = june_month();
    FAULT.with(|f| f.set(Some(1)));
    let placed = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(41))),
        &june(),
    );
    FAULT.with(|f| f.set(None));
    let placed = placed.unwrap().first;
    assert_eq!(
        (placed.receipt.kept, placed.receipt.reason),
        (Kept::NotKept, Reason::StorageFull)
    );
    FAULT.with(|f| f.set(Some(3)));
    let refused = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(42))),
        &june(),
    );
    FAULT.with(|f| f.set(None));
    assert!(
        matches!(
            refused,
            Err(KeepError::Io {
                kind: io::ErrorKind::StorageFull,
                ..
            })
        ),
        "{refused:?}"
    );
    // The body of the failed append is on disk with no receipt: the next
    // append overwrites it.
    let text = invented(43);
    let next = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&text)),
        &june(),
    )
    .unwrap()
    .first;
    assert_eq!((next.ordinal, next.receipt.body_offset), (1, 0));
    assert_eq!(
        body(scratch.root(), pack(&key), month, &next.receipt).unwrap(),
        text
    );
}

/// A lock held past the wait is refused by name, never waited for forever.
#[test]
fn a_pack_lock_held_past_the_wait_is_refused_by_name() {
    let scratch = Scratch::new("lock");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let held = lock(&dir).unwrap();
    let refused = lock_within(&dir, Duration::from_millis(60));
    let Err(KeepError::LockStalled {
        lock: named,
        waited_ms,
    }) = refused
    else {
        panic!("{refused:?}");
    };
    assert_eq!(named, dir.join("keep.lock"));
    assert!(waited_ms >= 60, "it waited its whole bound: {waited_ms}");
    drop(held);
    assert!(lock_within(&dir, Duration::from_millis(60)).is_ok());
}

/// Garbage at a pack's name: a symbolic link or a directory is refused, and
/// nothing is written through it.
#[test]
fn a_link_or_a_directory_at_a_pack_name_is_refused() {
    let scratch = Scratch::new("odd");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let target = scratch.root().join("elsewhere");
    fs::write(&target, b"untouched").unwrap();
    let at = names(&dir, june_month(), 0);
    std::os::unix::fs::symlink(&target, &at.receipts).unwrap();
    let refused = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(51))),
        &june(),
    );
    assert!(
        matches!(refused, Err(KeepError::NotRegular { .. })),
        "{refused:?}"
    );
    assert_eq!(fs::read(&target).unwrap(), b"untouched");

    let scratch = Scratch::new("odd-dir");
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(names(&dir, june_month(), 0).receipts).unwrap();
    let refused = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(52))),
        &june(),
    );
    assert!(refused.is_err(), "{refused:?}");
}

/// Empty and single answers, and a body that does not compress.
#[test]
fn empty_single_and_incompressible_bodies_round_trip() {
    let scratch = Scratch::new("shapes");
    let key = nifty();
    let mut noise = String::new();
    let mut x = 0x2545_F491_u32;
    for _ in 0..50_000 {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        noise.push(char::from(b'!' + u8::try_from(x % 90).unwrap()));
    }
    for text in [String::new(), "[]".to_owned(), noise] {
        let placed = append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&text)),
            &june(),
        )
        .unwrap()
        .first;
        assert_eq!(placed.receipt.raw_len as usize, text.len());
        assert_eq!(
            body(scratch.root(), pack(&key), june_month(), &placed.receipt).unwrap(),
            text
        );
    }
}

/// NIFTY first, then the same path elsewhere (operator, 13 Oct): a special
/// session, the daily rung and an answer that holds no rows each keep like
/// any other window, and "never asked", "asked, the vendor sent nothing" and
/// "asked, the vendor sent rows" are three different things on disk (F7).
#[test]
fn a_special_session_the_daily_rung_and_an_empty_answer_keep_like_any_window() {
    let scratch = Scratch::new("ia8-session");
    let spot = nifty();
    // A Saturday evening session (an invented hour's stamps on 2020-11-14,
    // a Saturday): the keeper takes the day as asked and never consults a
    // trading calendar, so a special session is not a special case here.
    let saturday = day(2020, 11, 14);
    let midnight_ist = 1_605_292_200; // 2020-11-14 00:00 IST
    let evening = Landing {
        first_stamp: midnight_ist + 18 * 3_600 + 15 * 60,
        last_stamp: midnight_ist + 19 * 3_600 + 14 * 60,
        rows_decoded: 60,
        rows_landed: 60,
        ..landing(saturday, saturday)
    };
    let text = invented(41);
    let placed = append(
        scratch.root(),
        pack(&spot),
        Offered::Answer(&answered(&text)),
        &evening,
    )
    .unwrap();
    let november = YearMonth::new(2020, 11).unwrap();
    assert!(placed.further.is_empty(), "one day is one month");
    assert_eq!(placed.first.month, november);
    let r = placed.first.receipt;
    assert_eq!(
        (r.asked_from, r.asked_to),
        (midnight_ist, midnight_ist + 86_399)
    );
    assert_eq!(
        (r.first_stamp, r.last_stamp, r.rows_decoded),
        (evening.first_stamp, evening.last_stamp, 60)
    );
    assert_eq!(
        body(scratch.root(), pack(&spot), november, &r).unwrap(),
        text
    );

    // THE DAILY RUNG: its own pack beside the minute's, never mixed into it.
    let daily = Pack {
        timeframe: Timeframe::DAY_1,
        ..pack(&spot)
    };
    let month_of_days = Landing {
        rung: crate::vendor::Granularity::Day1 as u8,
        rows_decoded: 21,
        rows_landed: 21,
        ..landing(day(2020, 11, 1), day(2020, 11, 30))
    };
    let days_text = invented(42);
    let day_placed = append(
        scratch.root(),
        daily,
        Offered::Answer(&answered(&days_text)),
        &month_of_days,
    )
    .unwrap();
    assert_eq!(
        pack_dir(scratch.root(), daily)
            .unwrap()
            .strip_prefix(scratch.root())
            .unwrap()
            .display()
            .to_string(),
        "responses/zerodha/NSE/INDEX/NIFTY/1day"
    );
    assert_eq!(day_placed.first.ordinal, 0, "the daily pack is its own");
    assert_eq!(
        day_placed.first.receipt.rung,
        crate::vendor::Granularity::Day1 as u8
    );
    let minute_receipts = receipts(scratch.root(), pack(&spot), november).unwrap();
    assert_eq!(
        minute_receipts[0].1.receipts.len(),
        1,
        "the minute pack is untouched"
    );

    // AN ILLIQUID STRIKE THE VENDOR ANSWERED WITH NO ROWS: kept, with a
    // receipt that says so, so it is never read as "not asked".
    let far = option(7_000, OptionSide::Put);
    let empty = "{\"open\":[],\"high\":[],\"low\":[],\"close\":[],\"volume\":[],\"timestamp\":[]}";
    let nothing = Landing {
        first_stamp: 0,
        last_stamp: 0,
        rows_decoded: 0,
        rows_landed: 0,
        ..june()
    };
    let placed = append(
        scratch.root(),
        pack(&far),
        Offered::Answer(&answered(empty)),
        &nothing,
    )
    .unwrap()
    .first;
    assert_eq!(placed.receipt.kept, Kept::Kept);
    assert_eq!(
        (placed.receipt.rows_decoded, placed.receipt.first_stamp),
        (0, 0),
        "asked, and the vendor sent nothing"
    );
    assert_eq!(
        body(scratch.root(), pack(&far), june_month(), &placed.receipt).unwrap(),
        empty
    );
    // A month nobody asked for has no receipt at all: "not asked".
    let july = YearMonth::new(2019, 7).unwrap();
    let never = receipts(scratch.root(), pack(&far), july).unwrap();
    assert!(
        never.iter().all(|(_, s)| s.receipts.is_empty()),
        "{never:?}"
    );
}

/// IA-RK-14 (DB-G46): every row a landing refused is kept with its OWN
/// cause, read from the landing's own record. One day's rows of two causes
/// interleave, so the per-day groups cannot name a row's cause: expanding
/// them would file the second row (conflicting) as off-grid and the third
/// (off-grid) as conflicting.
#[test]
fn every_refused_row_keeps_its_own_cause_from_the_landing_record() {
    let scratch = Scratch::new("ia8-causes");
    let key = nifty();
    let mut landing_record = crate::fetch::Refused::default();
    let seconds = [1_559_554_500, 1_559_554_560, 1_559_554_620, 1_559_640_900];
    let causes = [
        RowCause::OffGrid,
        RowCause::Conflicting,
        RowCause::OffGrid,
        RowCause::Price,
    ];
    for (i, (second, cause)) in seconds.iter().zip(causes).enumerate() {
        landing_record.note(cause, (i, *second), *second, || format!("row {i}"));
    }
    // The groups are per (day, cause): two on the first day, one on the next.
    assert_eq!(
        landing_record
            .groups
            .iter()
            .map(|g| (g.cause, g.rows))
            .collect::<Vec<_>>(),
        [
            (RowCause::OffGrid, 2),
            (RowCause::Conflicting, 1),
            (RowCause::Price, 1)
        ]
    );
    let rows = RefusedRow::all_of(&landing_record).unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| (r.local_second, r.cause))
            .collect::<Vec<_>>(),
        seconds.iter().copied().zip(causes).collect::<Vec<_>>(),
        "each row its own cause, in answer order"
    );
    let mut l = june();
    l.rows_landed = 371;
    l.rows_refused = u32::try_from(landing_record.rows()).unwrap();
    l.refused = &rows;
    let placed = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(51))),
        &l,
    )
    .unwrap()
    .first;
    assert_eq!(
        (placed.receipt.rows_refused, placed.receipt.refusals),
        (4, 4)
    );
    let back = refused_rows(
        scratch.root(),
        pack(&key),
        june_month(),
        placed.ordinal,
        &placed.receipt,
    )
    .unwrap();
    assert_eq!(back, rows, "the stored hole is explained minute by minute");

    // Two answers' records absorbed into one keep both lists in step.
    let mut other = crate::fetch::Refused::default();
    other.note(RowCause::Timestamp, (0, 7), 1_559_727_300, || {
        "t".to_owned()
    });
    landing_record.absorb(other);
    let joined = RefusedRow::all_of(&landing_record).unwrap();
    assert_eq!(joined.len(), 5);
    assert_eq!(joined[4].cause, RowCause::Timestamp);

    // A caller that took the seconds (the gap ledger's) and not the causes is
    // refused by name, never zipped short.
    let mut taken = landing_record.clone();
    let _gap_ledger = std::mem::take(&mut taken.local_seconds);
    let refused = RefusedRow::all_of(&taken).unwrap_err();
    assert_eq!(
        refused,
        ListsDiffer {
            seconds: 0,
            causes: 5
        }
    );
    assert!(
        refused
            .to_string()
            .contains("0 refused seconds and 5 causes")
    );
}

/// A CRC-valid receipt as a writer would leave it, owning `refusals` records
/// from `refusals_from` (no body).
fn crafted(segment: u8, refusals_from: u32, refusals: u32) -> Receipt {
    Receipt {
        asked_from: 0,
        asked_to: 0,
        first_stamp: 0,
        last_stamp: 0,
        body_offset: 0,
        deflated_len: 0,
        raw_len: 0,
        rows_decoded: refusals,
        rows_landed: 0,
        rows_refused: refusals,
        run: 1,
        status: 200,
        rung: 3,
        kept: Kept::NotKept,
        reason: Reason::PackFull,
        rules_version: 1,
        door: Door::FromWindow,
        segment,
        digest: [0; 32],
        key_hash: 0,
        body_of: 0,
        refusals_from,
        refusals,
    }
}

/// Five invented refused rows, two causes interleaved.
fn five_refused() -> Vec<RefusedRow> {
    (0..5_i64)
        .map(|i| RefusedRow {
            local_second: 1_559_554_500 + 60 * i,
            cause: if i % 2 == 0 {
                RowCause::OffGrid
            } else {
                RowCause::Conflicting
            },
        })
        .collect()
}

fn lengths_under(dir: &Path) -> Vec<(PathBuf, u64)> {
    files_under(dir)
        .into_iter()
        .map(|p| {
            let len = fs::metadata(&p).unwrap().len();
            (p, len)
        })
        .collect()
}

/// IA-RK-15 (DB-G46, verify round 1): refused rows past a segment's `.rsx`
/// room are never dropped silently. A window whose rows do not fit rolls the
/// month to the next segment, a repeated body included; with no segment able
/// to take them nothing is written and the append is refused
/// `refusals_full`, naming the file and both counts. Before this the receipt
/// was written `Ok` with `rows_refused=5` and no record of any of the five.
#[test]
fn refused_rows_past_the_rsx_room_roll_or_are_refused_by_name() {
    let scratch = Scratch::new("rsx");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let month = june_month();
    let room = u32::try_from(REFUSAL_ROOM).unwrap();
    assert_eq!(REFUSAL_ROOM, 2_796_202, "64 MiB of 24-byte records");
    let rows = five_refused();
    let mut with_rows = june();
    with_rows.rows_refused = 5;
    with_rows.refused = &rows;

    // (a) A body kept in segment 0, then a receipt that takes the `.rsx` to
    // two records short of its room: the same body again with five refused
    // rows is NOT a `kept=2` there (the rows cannot fit); it rolls to segment
    // 1, keeps the body there and every row with its cause.
    let text = invented(31);
    let first = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&text)),
        &june(),
    )
    .unwrap()
    .first;
    assert_eq!((first.segment, first.receipt.kept), (0, Kept::Kept));
    let mut receipts_0 = fs::read(names(&dir, month, 0).receipts).unwrap();
    receipts_0.extend_from_slice(&crafted(0, room - 2, 2).encode());
    fs::write(names(&dir, month, 0).receipts, &receipts_0).unwrap();
    assert_eq!(
        read_segment(&names(&dir, month, 0)).unwrap().refusals_end,
        room
    );
    let again = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&text)),
        &with_rows,
    )
    .unwrap()
    .first;
    assert_eq!(
        (again.segment, again.rolled, again.receipt.kept),
        (1, true, Kept::Kept),
        "a full .rsx rolls the month, a repeated body included"
    );
    assert_eq!((again.receipt.rows_refused, again.receipt.refusals), (5, 5));
    assert_eq!(again.receipt.refusals_from, 0);
    assert_eq!(
        refused_rows(
            scratch.root(),
            pack(&key),
            month,
            again.ordinal,
            &again.receipt
        )
        .unwrap(),
        rows,
        "every refused row with its own cause"
    );
    assert_eq!(
        body(scratch.root(), pack(&key), month, &again.receipt).unwrap(),
        text
    );

    // (b) A new body whose rows do not fit the newest segment rolls too.
    let mut receipts_1 = fs::read(names(&dir, month, 1).receipts).unwrap();
    receipts_1.extend_from_slice(&crafted(1, 5, room - 6).encode());
    fs::write(names(&dir, month, 1).receipts, &receipts_1).unwrap();
    let rolled = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(32))),
        &with_rows,
    )
    .unwrap()
    .first;
    assert_eq!((rolled.segment, rolled.rolled), (2, true));
    assert_eq!(rolled.receipt.refusals, 5);

    // (c) The LAST segment's `.rsx` is two records short of its room. A
    // window that refused no row is still kept there: the cap is on the
    // rows, not on the pack.
    for segment in 3..MAX_SEGMENTS {
        fs::write(
            names(&dir, month, segment).receipts,
            crafted(segment, room - 2, 2).encode(),
        )
        .unwrap();
    }
    let last = MAX_SEGMENTS - 1;
    let clean_text = invented(34);
    let clean = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&clean_text)),
        &june(),
    )
    .unwrap()
    .first;
    assert_eq!(
        (clean.segment, clean.receipt.kept, clean.receipt.refusals),
        (last, Kept::Kept, 0)
    );

    // (d) There, a window WITH refused rows writes nothing, a new body and
    // one the segment already holds (which was a `kept=2` with no record of
    // its rows) alike, and the refusal names the file and both counts.
    let before = lengths_under(&dir);
    for offered in [invented(33), clean_text] {
        let refused = append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&offered)),
            &with_rows,
        );
        assert_eq!(
            refused,
            Err(KeepError::RefusalsFull {
                pack: names(&dir, month, last).refusals,
                held: room,
                rows: 5,
            }),
            "a new body and a repeated one alike"
        );
        let why = refused.unwrap_err();
        assert_eq!(why.word(), "refusals_full");
        assert!(why.to_string().contains(&format!(
            "holds {room} refused-row records of at most {REFUSAL_ROOM} and no segment can take 5 more; nothing written"
        )));
    }
    assert_eq!(before, lengths_under(&dir), "nothing was written");

    // (e) More rows than any segment holds: no segment is spent on a roll
    // that could not help, and nothing is written.
    let fresh = Scratch::new("rsx");
    let too_many: Vec<RefusedRow> = (0..=REFUSAL_ROOM)
        .map(|i| RefusedRow {
            local_second: 1_559_554_500 + i64::try_from(i).unwrap(),
            cause: RowCause::OffGrid,
        })
        .collect();
    let mut flood = june();
    flood.refused = &too_many;
    let refused = append(
        fresh.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(35))),
        &flood,
    );
    let fresh_dir = pack_dir(fresh.root(), pack(&key)).unwrap();
    assert_eq!(
        refused,
        Err(KeepError::RefusalsFull {
            pack: names(&fresh_dir, month, 0).refusals,
            held: 0,
            rows: REFUSAL_ROOM + 1,
        })
    );
    assert_eq!(
        files_under(&fresh_dir),
        vec![fresh_dir.join(LOCK_NAME)],
        "no pack file was born"
    );
}

/// IA-RK-16: a CRC-valid receipt that names lengths past the caps is damage,
/// refused before anything is allocated for it (a body of up to 4 GiB, or
/// up to 96 GiB of refused-row records).
#[test]
fn a_receipt_naming_lengths_past_the_caps_is_refused_before_reading() {
    let scratch = Scratch::new("oversize");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let month = june_month();
    fs::write(names(&dir, month, 0).bodies, [0_u8; 64]).unwrap();
    fs::write(names(&dir, month, 0).refusals, [0_u8; 48]).unwrap();
    let kept = |body_offset: u64, deflated_len: u32, raw_len: u32| {
        let mut r = crafted(0, 0, 0);
        r.kept = Kept::Kept;
        r.reason = Reason::Kept;
        r.body_offset = body_offset;
        r.deflated_len = deflated_len;
        r.raw_len = raw_len;
        Receipt::decode(&r.encode()).unwrap()
    };
    let past = Err(KeepError::BodyMismatch {
        path: names(&dir, month, 0).bodies,
        what: "names a length past the pack's caps",
    });
    // A body that would end one byte past the segment's cap.
    assert_eq!(
        body(
            scratch.root(),
            pack(&key),
            month,
            &kept(MAX_PACK_BYTES - 7, 8, 8)
        ),
        past
    );
    // A raw length past the largest answer.
    let raw_past = u32::try_from(crate::http::MAX_RESPONSE_BYTES + 1).unwrap();
    assert_eq!(
        body(scratch.root(), pack(&key), month, &kept(0, 8, raw_past)),
        past
    );
    assert_eq!(
        body(scratch.root(), pack(&key), month, &kept(0, u32::MAX, 8)),
        past
    );
    // At the cap it is read (and here, refused for what it holds, by name).
    assert_eq!(
        body(scratch.root(), pack(&key), month, &kept(0, 8, 8)),
        Err(KeepError::BodyMismatch {
            path: names(&dir, month, 0).bodies,
            what: "does not inflate to its receipt's length",
        })
    );
    // Records past the room.
    let rows_past = |from: u32, count: u32| {
        refused_rows(
            scratch.root(),
            pack(&key),
            month,
            0,
            &Receipt::decode(&crafted(0, from, count).encode()).unwrap(),
        )
    };
    let room = u32::try_from(REFUSAL_ROOM).unwrap();
    assert_eq!(
        rows_past(room, 1),
        Err(KeepError::Damaged {
            path: names(&dir, month, 0).refusals,
            ordinal: u64::from(room),
        })
    );
    assert_eq!(
        rows_past(0, u32::MAX),
        Err(KeepError::Damaged {
            path: names(&dir, month, 0).refusals,
            ordinal: 0,
        })
    );
    // Inside the room, a record that fails its CRC is damage at its own place.
    assert_eq!(
        rows_past(1, 1),
        Err(KeepError::Damaged {
            path: names(&dir, month, 0).refusals,
            ordinal: 1,
        })
    );
}

/// IA-RK-17 (verify round 1): a holder that never lets go leaves at most
/// `MAX_PARKED_WAITERS` waiter threads parked on its lock; the next append is
/// refused `lock_stalled` at once, naming how many are parked, and waits for
/// nothing. Once the holder lets go every parked waiter takes the lock, lets
/// it go and leaves, and the pack locks again.
#[test]
fn waiters_parked_on_a_held_pack_lock_are_bounded() {
    let scratch = Scratch::new("keep-lock");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let lock_file = dir.join(LOCK_NAME);
    let held = lock(&dir).unwrap();
    for parked in 1..=MAX_PARKED_WAITERS {
        let refused = lock_within(&dir, Duration::from_millis(20));
        assert!(
            matches!(refused, Err(KeepError::LockStalled { .. })),
            "{refused:?}"
        );
        assert_eq!(
            parked_on(&lock_file),
            parked,
            "each refused wait stays parked"
        );
    }
    let started = std::time::Instant::now();
    let refused = lock_within(&dir, Duration::from_secs(3));
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "refused at once, not after its wait: {:?}",
        started.elapsed()
    );
    assert_eq!(
        refused.as_ref().unwrap_err(),
        &KeepError::LockWaitersParked {
            lock: lock_file.clone(),
            parked: MAX_PARKED_WAITERS,
        }
    );
    let why = refused.unwrap_err();
    assert_eq!(why.word(), "lock_stalled");
    // The count it names and the bound are two numbers, said apart: the
    // message once read "(at most 4)" beside twelve parked (verify r3).
    assert!(why.to_string().contains(&format!(
        "has {MAX_PARKED_WAITERS} abandoned waits still parked"
    )));
    assert!(why.to_string().contains(&format!(
        "new waits are refused once {MAX_PARKED_WAITERS} are parked"
    )));
    assert!(!why.to_string().contains("at most"));
    assert!(why.to_string().contains("nothing written"));
    assert_eq!(
        parked_on(&lock_file),
        MAX_PARKED_WAITERS,
        "a refused append parks nothing"
    );
    drop(held);
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while parked_on(&lock_file) > 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "the parked waiters left once the holder let go"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(lock_within(&dir, Duration::from_secs(5)).is_ok());
    assert_eq!(parked_on(&lock_file), 0);
}

/// IA-RK-15 (repair round 1): a window with more refused rows than any
/// segment holds is refused `refusals_full` even when the newest segment
/// would roll for another reason, its receipts or its bodies being full. The
/// roll used to hand the rows to an empty segment that could not take them
/// either, and they were written past its room.
#[test]
fn more_refused_rows_than_a_segment_holds_are_refused_before_any_roll() {
    let key = nifty();
    let month = june_month();
    let too_many: Vec<RefusedRow> = (0..=REFUSAL_ROOM)
        .map(|i| RefusedRow {
            local_second: 1_559_554_500 + i64::try_from(i).unwrap(),
            cause: RowCause::OffGrid,
        })
        .collect();
    let mut flood = june();
    flood.refused = &too_many;
    let mut receipts_full = Vec::with_capacity(MAX_RECEIPTS as usize * RECEIPT_LEN);
    for ordinal in 0..MAX_RECEIPTS {
        let mut r = crafted(0, 0, 0);
        r.body_of = ordinal;
        receipts_full.extend_from_slice(&r.encode());
    }
    let mut body_full = crafted(0, 0, 0);
    body_full.kept = Kept::Kept;
    body_full.reason = Reason::Kept;
    body_full.deflated_len = u32::try_from(MAX_PACK_BYTES).unwrap();
    for newest in [receipts_full, body_full.encode().to_vec()] {
        let scratch = Scratch::new("rsx");
        let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(names(&dir, month, 0).receipts, &newest).unwrap();
        drop(lock(&dir).unwrap());
        let before = lengths_under(&dir);
        let refused = append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&invented(36))),
            &flood,
        );
        assert_eq!(
            refused,
            Err(KeepError::RefusalsFull {
                pack: names(&dir, month, 0).refusals,
                held: 0,
                rows: REFUSAL_ROOM + 1,
            }),
            "refused in the segment it was offered to, not rolled"
        );
        assert_eq!(before, lengths_under(&dir), "nothing was written");
        assert!(
            !names(&dir, month, 1).receipts.exists(),
            "no segment was spent on a roll that could not help"
        );
    }
}

/// IA-RK-15 (repair round 1): the WRITER itself refuses refused rows past a
/// segment's room, in every build, rather than trust its callers. It used to
/// hold only a `debug_assert`, so a release build would have written the
/// records past `REFUSAL_ROOM`; here, in a test build, it panicked instead of
/// naming the refusal.
#[test]
fn the_writer_refuses_rows_past_the_room_by_name_in_every_build() {
    let scratch = Scratch::new("rsx");
    let key = nifty();
    let month = june_month();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let at = names(&dir, month, 0);
    let held = u32::try_from(REFUSAL_ROOM).unwrap() - 2;
    let state = super::Segment {
        refusals_end: held,
        ..super::Segment::default()
    };
    let rows = five_refused();
    let got = write_refusals_then_receipt(&at, &state, 0, crafted(0, 0, 0), &rows, 0, month, false);
    assert_eq!(
        got,
        Err(KeepError::RefusalsFull {
            pack: at.refusals.clone(),
            held,
            rows: 5,
        })
    );
    assert_eq!(
        got.unwrap_err().word(),
        "refusals_full",
        "the refusal has its name"
    );
    assert!(lengths_under(&dir).is_empty(), "nothing was written");
}

/// IA-RK-18 (repair round 1): a window over months whose later month cannot
/// take its pointer receipt writes NOTHING. The refusal `receipts_full` says
/// "nothing written", and it used to come only after the first month's body,
/// refused rows and receipt were already kept.
#[test]
fn a_window_over_months_writes_nothing_when_a_later_month_is_full() {
    let scratch = Scratch::new("months");
    let key = nifty();
    let rows = five_refused();
    let mut l = landing(day(2019, 11, 15), day(2020, 2, 10));
    l.rung = crate::vendor::Granularity::Day1 as u8;
    l.refused = &rows;
    let day_pack = Pack {
        timeframe: Timeframe::DAY_1,
        ..pack(&key)
    };
    let dir = pack_dir(scratch.root(), day_pack).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let full_month = YearMonth::new(2020, 1).unwrap();
    let mut full = Vec::with_capacity(MAX_RECEIPTS as usize * RECEIPT_LEN);
    for ordinal in 0..MAX_RECEIPTS {
        let mut r = crafted(0, 0, 0);
        r.body_of = ordinal;
        full.extend_from_slice(&r.encode());
    }
    for segment in 0..MAX_SEGMENTS {
        fs::write(names(&dir, full_month, segment).receipts, &full).unwrap();
    }
    drop(lock(&dir).unwrap());
    let before = lengths_under(&dir);
    let refused = append(
        scratch.root(),
        day_pack,
        Offered::Answer(&answered(&invented(37))),
        &l,
    );
    assert_eq!(
        refused,
        Err(KeepError::ReceiptsFull {
            pack: names(&dir, full_month, MAX_SEGMENTS - 1).receipts,
        })
    );
    assert!(refused.unwrap_err().to_string().contains("nothing written"));
    assert_eq!(
        before,
        lengths_under(&dir),
        "the first month kept no body, no row and no receipt"
    );
    assert!(
        !names(&dir, YearMonth::new(2019, 11).unwrap(), 0)
            .receipts
            .exists()
    );
}

/// IA-RK-16 (repair round 1): the WRITER reads a segment's receipts too, and
/// a CRC-valid receipt naming a body past `MAX_PACK_BYTES` or records past
/// `REFUSAL_ROOM` is damage to it as well, refused `pack_damaged` with
/// nothing written. Past the caps such a receipt used to roll the month on a
/// claim no writer makes, and at the edge of a u32 or u64 its sum overflowed
/// (a panic in a debug build; in a release build a small end that sent the
/// next append over records already kept).
#[test]
fn a_receipt_naming_lengths_past_the_caps_is_damage_to_the_writer() {
    let key = nifty();
    let month = june_month();
    let room = u32::try_from(REFUSAL_ROOM).unwrap();
    let rows = five_refused();
    let mut with_rows = june();
    with_rows.rows_refused = 5;
    with_rows.refused = &rows;
    let kept_at = |body_offset: u64, deflated_len: u32| {
        let mut r = crafted(0, 0, 0);
        r.kept = Kept::Kept;
        r.reason = Reason::Kept;
        r.body_offset = body_offset;
        r.deflated_len = deflated_len;
        r
    };
    let damaged = [
        crafted(0, room, 1),
        crafted(0, u32::MAX - 1, 2),
        crafted(0, 0, room + 1),
        kept_at(MAX_PACK_BYTES, 1),
        kept_at(u64::MAX - 1, 4),
    ];
    for receipt in damaged {
        let scratch = Scratch::new("oversize");
        let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(names(&dir, month, 0).receipts, receipt.encode()).unwrap();
        drop(lock(&dir).unwrap());
        let before = lengths_under(&dir);
        let refused = append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&invented(38))),
            &with_rows,
        );
        assert_eq!(
            refused,
            Err(KeepError::Damaged {
                path: names(&dir, month, 0).receipts,
                ordinal: 0,
            }),
            "{receipt:?}"
        );
        assert_eq!(refused.unwrap_err().word(), "pack_damaged");
        assert_eq!(before, lengths_under(&dir), "nothing was written");
    }
    // At the caps exactly, the receipt is a writer's and the month rolls.
    for receipt in [
        crafted(0, room - 5, 5),
        kept_at(0, u32::try_from(MAX_PACK_BYTES).unwrap()),
    ] {
        let scratch = Scratch::new("oversize");
        let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
        fs::create_dir_all(&dir).unwrap();
        fs::write(names(&dir, month, 0).receipts, receipt.encode()).unwrap();
        let placed = append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&invented(39))),
            &with_rows,
        )
        .unwrap()
        .first;
        assert_eq!((placed.segment, placed.rolled), (1, true), "{receipt:?}");
        assert_eq!(placed.receipt.refusals, 5);
    }
}

/// IA-RK-19 (repair round 2): healthy contention waits, it is not refused.
/// Twenty appends of one pack queue behind a holder that lets go after a
/// moment; every one is kept, none is refused `LockWaitersParked`, and no
/// waiter is left counted as parked. Counting every waiter rather than the
/// abandoned ones refused the sixth at once with a false "holder that has
/// not let go" (measured by the second agent: 20 threads, 6 kept, 14
/// refused).
#[test]
fn healthy_contention_on_one_pack_waits_rather_than_refusing() {
    const APPENDS: u32 = 20;
    let scratch = Scratch::new("healthy-contention");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let lock_file = dir.join(LOCK_NAME);
    let held = lock(&dir).unwrap();
    let ready = std::sync::Barrier::new(APPENDS as usize + 1);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..APPENDS)
            .map(|i| {
                let (root, key, ready) = (scratch.root(), &key, &ready);
                scope.spawn(move || {
                    let text = invented(50_000 + i);
                    ready.wait();
                    append(root, pack(key), Offered::Answer(&answered(&text)), &june())
                })
            })
            .collect();
        ready.wait();
        // Every append is now past its `try_lock` or about to be: the pack
        // is contended twenty deep while the holder is alive.
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(parked_on(&lock_file), 0, "a live wait is not parked");
        drop(held);
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    let refused: Vec<_> = results.iter().filter_map(|r| r.as_ref().err()).collect();
    assert!(
        refused.is_empty(),
        "every healthy wait is kept: {refused:?}"
    );
    let segments = receipts(scratch.root(), pack(&key), june_month()).unwrap();
    let total: usize = segments.iter().map(|(_, s)| s.receipts.len()).sum();
    assert_eq!(total, APPENDS as usize);
    assert_eq!(parked_on(&lock_file), 0, "nobody was abandoned");
}

/// IA-RK-19's other half: a waiter abandoned by its caller is counted the
/// moment the wait runs out and uncounted once the holder lets go, while a
/// live waiter beside it is never counted and takes the lock.
#[test]
fn only_an_abandoned_waiter_is_counted_as_parked() {
    let scratch = Scratch::new("abandoned");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let lock_file = dir.join(LOCK_NAME);
    let held = lock(&dir).unwrap();
    let refused = lock_within(&dir, Duration::from_millis(20));
    assert!(
        matches!(refused, Err(KeepError::LockStalled { .. })),
        "{refused:?}"
    );
    assert_eq!(parked_on(&lock_file), 1, "the abandoned waiter is counted");
    let live = std::thread::scope(|scope| {
        let waiting = scope.spawn(|| lock_within(&dir, Duration::from_secs(20)));
        std::thread::sleep(Duration::from_millis(200));
        assert_eq!(parked_on(&lock_file), 1, "the live waiter is not");
        drop(held);
        waiting.join().unwrap()
    });
    assert!(live.is_ok(), "{live:?}");
    drop(live);
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while parked_on(&lock_file) > 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "the abandoned waiter left once the holder let go"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// IA-RK-20 (follow-up of verify-IA-8-r2): a further month's `kept=0`
/// pointer names the first month's receipt by segment and ordinal, as a
/// `kept=3` pointer does. The first month here has rolled through all eight
/// segments and keeps no body (`pack_full` in segment 7); its five refused
/// rows are still written there, and the July pointer finds them. A pointer
/// that took its own segment (0) named June's segment 0, which holds a
/// different receipt.
#[test]
fn a_later_months_unkept_pointer_names_the_first_months_receipt() {
    let scratch = Scratch::new("unkept-pointer");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let june = june_month();
    for segment in 0..MAX_SEGMENTS {
        let mut full = crafted(segment, 0, 0);
        full.kept = Kept::Kept;
        full.reason = Reason::Kept;
        full.body_offset = MAX_PACK_BYTES - 10;
        full.deflated_len = 10;
        full.raw_len = 10;
        full.digest = [7; 32];
        fs::write(names(&dir, june, segment).receipts, full.encode()).unwrap();
    }
    let rows = five_refused();
    let mut window = landing(day(2019, 6, 28), day(2019, 7, 2));
    window.rows_refused = 5;
    window.refused = &rows;
    let outcome = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(60))),
        &window,
    )
    .unwrap();
    let last = MAX_SEGMENTS - 1;
    assert_eq!(
        (outcome.first.segment, outcome.first.ordinal),
        (last, 1),
        "June's receipt follows the full one in its last segment"
    );
    assert_eq!(
        (outcome.first.receipt.kept, outcome.first.receipt.reason),
        (Kept::NotKept, Reason::PackFull)
    );
    let [july] = outcome.further.as_slice() else {
        panic!("one further month: {outcome:?}");
    };
    assert_eq!(
        (july.month, july.segment, july.ordinal),
        (YearMonth::new(2019, 7).unwrap(), 0, 0)
    );
    let stored = &receipts(scratch.root(), pack(&key), july.month).unwrap()[0]
        .1
        .receipts[0];
    assert_eq!(stored, &july.receipt, "the pointer as written");
    assert_eq!(stored.kept, Kept::NotKept);
    assert_eq!(
        (stored.segment, stored.body_of),
        (last, 1),
        "it names June's receipt, not its own segment"
    );
    assert_eq!(
        (stored.refusals_from, stored.refusals),
        (0, 0),
        "it owns no records"
    );
    // Followed: June, at the segment and ordinal the pointer names, holds
    // the window's receipt and its five rows.
    let home = month_of_unix(stored.asked_from).unwrap();
    assert_eq!(home, june);
    let june_segments = receipts(scratch.root(), pack(&key), home).unwrap();
    let owner = june_segments[usize::from(stored.segment)].1.receipts[stored.body_of as usize];
    assert_eq!(owner, outcome.first.receipt);
    let found = refused_rows(scratch.root(), pack(&key), home, stored.body_of, &owner).unwrap();
    assert_eq!(found, rows);
}

/// Every trusted receipt of `month`, segment by segment, flattened.
fn held_in(root: &Path, key: &InstrumentKey, month: YearMonth) -> Vec<Receipt> {
    receipts(root, pack(key), month)
        .unwrap()
        .into_iter()
        .flat_map(|(_, segment)| segment.receipts)
        .collect()
}

/// The window journal's records, every whole one decoded (`None` = torn).
fn journal_of(root: &Path, key: &InstrumentKey) -> Vec<Option<WindowRecord>> {
    let path = pack_dir(root, pack(key)).unwrap().join(WINDOWS_NAME);
    fs::read(path)
        .unwrap_or_default()
        .chunks_exact(WINDOW_RECORD_LEN)
        .map(WindowRecord::decode)
        .collect()
}

/// Runs one append with the test seam refusing write number `skip` (from 0)
/// of file kind `kind`: a kill at that point, nothing after it written.
fn killed_at(
    root: &Path,
    key: &InstrumentKey,
    (kind, skip): (u8, u32),
    text: &str,
    window: &Landing<'_>,
) -> Result<Outcome, KeepError> {
    FAULT.with(|f| f.set(Some(kind)));
    FAULT_SKIP.with(|s| s.set(skip));
    let result = append(root, pack(key), Offered::Answer(&answered(text)), window);
    FAULT.with(|f| f.set(None));
    FAULT_SKIP.with(|s| s.set(0));
    result
}

fn july_month() -> YearMonth {
    YearMonth::new(2019, 7).unwrap()
}

/// IA-RK-21 (verify-IA-8-r3 blocking 1): a writer killed between a window's
/// first month's receipt and a later month's pointer left that receipt with
/// no pointer in the later month, for good: nothing named it and the next
/// append carried on. Now the pack's window journal holds an `open` record
/// from before the first receipt; a reader names the open window, and the
/// next append, under the pack lock, writes the missing pointer exactly as
/// the killed one would have, closes the record `completed` and says so.
/// BEFORE the fix the August append left July with no receipt at all.
#[test]
fn a_window_over_months_killed_after_its_first_receipt_is_finished_by_the_next() {
    let scratch = Scratch::new("ia8-killed-window");
    let key = nifty();
    let text = invented(70);
    let window = landing(day(2019, 6, 28), day(2019, 7, 2));
    let killed = killed_at(scratch.root(), &key, (3, 1), &text, &window);
    assert!(
        matches!(
            killed,
            Err(KeepError::Io {
                kind: io::ErrorKind::StorageFull,
                ..
            })
        ),
        "the July pointer's write is the kill: {killed:?}"
    );
    let june = held_in(scratch.root(), &key, june_month());
    assert_eq!(june.len(), 1, "June's receipt was committed");
    assert_eq!(june[0].kept, Kept::Kept);
    assert!(
        held_in(scratch.root(), &key, july_month()).is_empty(),
        "and July has no pointer: the orphan the checker found"
    );
    // DETECTABLE BEFORE ANY REPAIR: a reader names the open window by month,
    // segment and ordinal.
    let open = unfinished(scratch.root(), pack(&key)).unwrap().unwrap();
    assert_eq!(
        (open.month, open.segment, open.ordinal, open.state),
        (june_month(), 0, 0, WindowState::Open)
    );
    assert_eq!(
        (open.asked_from, open.asked_to),
        (june[0].asked_from, june[0].asked_to)
    );
    // The next append, of an unrelated month, finishes it first.
    let august = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(71))),
        &landing(day(2019, 8, 5), day(2019, 8, 9)),
    )
    .unwrap();
    assert!(august.further.is_empty());
    let july = held_in(scratch.root(), &key, july_month());
    let [pointer] = july.as_slice() else {
        panic!("July holds exactly the missing pointer: {july:?}");
    };
    assert_eq!(
        (
            pointer.kept,
            pointer.reason,
            pointer.segment,
            pointer.body_of
        ),
        (Kept::InFirstMonth, Reason::LaterMonth, 0, 0)
    );
    assert_eq!(
        body(scratch.root(), pack(&key), july_month(), pointer).unwrap(),
        text,
        "it leads to June's body"
    );
    assert_eq!(unfinished(scratch.root(), pack(&key)).unwrap(), None);
    let journal = journal_of(scratch.root(), &key);
    assert_eq!(
        journal.len(),
        2,
        "open, then the finishing record: {journal:?}"
    );
    let closing = journal[1].unwrap();
    assert_eq!(
        (closing.state, closing.written),
        (WindowState::Completed, 1)
    );
    assert_eq!(
        (closing.month, closing.segment, closing.ordinal),
        (june_month(), 0, 0)
    );
}

/// IA-RK-21, every kill point of a window over three months, and the same
/// window repeated (a `kept=2` first receipt with the same span, body and
/// run as an earlier whole one). After the next append each further month
/// holds exactly one pointer per committed first receipt and the journal's
/// last record says what was found.
#[test]
fn every_kill_point_of_a_window_over_months_is_finished_or_named_abandoned() {
    type KillRow = ((u8, u32), bool, WindowState, u8, usize, usize);
    let window = landing(day(2019, 6, 28), day(2019, 8, 2));
    let august = YearMonth::new(2019, 8).unwrap();
    // (kind, writes let through, repeat first, state, pointers written,
    //  first-month receipts after, pointers per further month after)
    let rows: [KillRow; 6] = [
        ((3, 0), false, WindowState::Abandoned, 0, 0, 0),
        ((3, 1), false, WindowState::Completed, 2, 1, 1),
        ((3, 2), false, WindowState::Completed, 1, 1, 1),
        ((4, 1), false, WindowState::Completed, 0, 1, 1),
        ((3, 1), true, WindowState::Completed, 2, 2, 2),
        ((3, 0), true, WindowState::Abandoned, 0, 1, 1),
    ];
    for (point, repeat, state, written, firsts, pointers) in rows {
        let scratch = Scratch::new("ia8-kill-points");
        let key = nifty();
        let text = invented(72);
        if repeat {
            append(
                scratch.root(),
                pack(&key),
                Offered::Answer(&answered(&text)),
                &window,
            )
            .unwrap();
        }
        let killed = killed_at(scratch.root(), &key, point, &text, &window);
        assert!(killed.is_err(), "{point:?} {repeat}: {killed:?}");
        assert!(
            unfinished(scratch.root(), pack(&key)).unwrap().is_some(),
            "{point:?} {repeat}: the open window is named"
        );
        append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&invented(73))),
            &landing(day(2019, 9, 2), day(2019, 9, 6)),
        )
        .unwrap();
        let journal = journal_of(scratch.root(), &key);
        let last = journal.last().copied().flatten().unwrap();
        assert_eq!(
            (last.state, last.written),
            (state, written),
            "{point:?} {repeat}: {journal:?}"
        );
        assert_eq!(unfinished(scratch.root(), pack(&key)).unwrap(), None);
        let june = held_in(scratch.root(), &key, june_month());
        assert_eq!(june.len(), firsts, "{point:?} {repeat}: {june:?}");
        for month in [july_month(), august] {
            let held = held_in(scratch.root(), &key, month);
            assert_eq!(held.len(), pointers, "{point:?} {repeat} {month}: {held:?}");
            // Pointer n names first receipt n: none is missing, none doubled.
            for (n, pointer) in held.iter().enumerate() {
                assert_eq!(
                    (pointer.kept, pointer.segment, pointer.body_of as usize),
                    (Kept::InFirstMonth, 0, n),
                    "{point:?} {repeat} {month}"
                );
                assert_eq!(
                    body(scratch.root(), pack(&key), month, pointer).unwrap(),
                    text
                );
            }
        }
    }
}

/// The journal under the torn-tail rule: a partial record and a whole last
/// record whose CRC fails are an interrupted write, written over by the
/// next; a bad record before the last is damage, refused by name and never
/// written over.
#[test]
fn a_torn_window_journal_is_written_over_and_a_damaged_one_refused() {
    let scratch = Scratch::new("ia8-torn-journal");
    let key = nifty();
    let window = landing(day(2019, 6, 28), day(2019, 7, 2));
    append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(74))),
        &window,
    )
    .unwrap();
    let path = pack_dir(scratch.root(), pack(&key))
        .unwrap()
        .join(WINDOWS_NAME);
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        2 * WINDOW_RECORD_LEN as u64
    );
    // A partial record, then a whole one that fails its CRC.
    for junk in [vec![0xA5_u8; 10], vec![0xA5_u8; WINDOW_RECORD_LEN]] {
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&junk).unwrap();
        drop(file);
        assert_eq!(unfinished(scratch.root(), pack(&key)).unwrap(), None);
        append(
            scratch.root(),
            pack(&key),
            Offered::Answer(&answered(&invented(
                75 + u32::try_from(junk.len()).unwrap(),
            ))),
            &window,
        )
        .unwrap();
        let journal = journal_of(scratch.root(), &key);
        assert!(journal.iter().all(Option::is_some), "{journal:?}");
        assert_eq!(
            journal.last().copied().flatten().unwrap().state,
            WindowState::Closed
        );
    }
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        6 * WINDOW_RECORD_LEN as u64
    );
    // Damage to the record before a torn last one: refused, nothing more
    // written. (A reader trusts the last two records only: what is before
    // them is history no append reads.)
    let mut bytes = fs::read(&path).unwrap();
    bytes[WINDOW_RECORD_LEN * 5] ^= 0xFF;
    fs::write(&path, &bytes).unwrap();
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    file.write_all(&[0xA5_u8; WINDOW_RECORD_LEN]).unwrap();
    drop(file);
    let july_before = held_in(scratch.root(), &key, july_month()).len();
    let refused = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(90))),
        &window,
    );
    assert_eq!(
        refused,
        Err(KeepError::Damaged {
            path: path.clone(),
            ordinal: 5
        })
    );
    assert_eq!(refused.unwrap_err().word(), "pack_damaged");
    assert_eq!(
        held_in(scratch.root(), &key, july_month()).len(),
        july_before
    );
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        7 * WINDOW_RECORD_LEN as u64
    );
}

/// A journal with no room for a window's two records refuses the window
/// over months `windows_full` before anything is written; a window inside
/// one month needs no record and is kept. The journal here is sparse: one
/// whole `closed` record at its last place but one.
#[test]
fn a_full_window_journal_refuses_a_window_over_months_by_name() {
    let scratch = Scratch::new("ia8-windows-full");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(WINDOWS_NAME);
    let closed = WindowRecord {
        asked_from: 1,
        asked_to: 2,
        digest: [3; 32],
        run: 4,
        month: june_month(),
        segment: 0,
        ordinal: 0,
        state: WindowState::Closed,
        written: 0,
    };
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    let last = (MAX_WINDOW_RECORDS - 2) * WINDOW_RECORD_LEN as u64;
    std::os::unix::fs::FileExt::write_all_at(&file, &closed.encode(), last).unwrap();
    drop(file);
    let refused = append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(91))),
        &landing(day(2019, 6, 28), day(2019, 7, 2)),
    );
    assert_eq!(
        refused,
        Err(KeepError::WindowsFull {
            journal: path.clone()
        })
    );
    let why = refused.unwrap_err();
    assert_eq!(why.word(), "windows_full");
    assert!(why.to_string().contains("nothing written"));
    assert!(held_in(scratch.root(), &key, june_month()).is_empty());
    assert!(held_in(scratch.root(), &key, july_month()).is_empty());
    assert_eq!(
        fs::metadata(&path).unwrap().len(),
        (MAX_WINDOW_RECORDS - 1) * WINDOW_RECORD_LEN as u64
    );
    append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(92))),
        &june(),
    )
    .unwrap();
    assert_eq!(held_in(scratch.root(), &key, june_month()).len(), 1);
}

/// A window-journal record round-trips, and a flipped bit or a state byte
/// no writer writes is refused.
#[test]
fn a_window_record_round_trips_and_a_flipped_bit_is_refused() {
    for state in [
        WindowState::Open,
        WindowState::Closed,
        WindowState::Completed,
        WindowState::Abandoned,
    ] {
        assert_eq!(WindowState::from_code(state.code()), Some(state));
        let record = WindowRecord {
            asked_from: -5,
            asked_to: 1_562_088_599,
            digest: [9; 32],
            run: u32::MAX,
            month: YearMonth::new(2026, 12).unwrap(),
            segment: MAX_SEGMENTS - 1,
            ordinal: 1_023,
            state,
            written: 255,
        };
        let bytes = record.encode();
        assert_eq!(WindowRecord::decode(&bytes), Some(record));
        for bit in [0, 8 * 58, 8 * 63] {
            let mut flipped = bytes;
            flipped[bit / 8] ^= 1 << (bit % 8);
            assert_eq!(WindowRecord::decode(&flipped), None, "bit {bit}");
        }
        assert_eq!(WindowRecord::decode(&bytes[1..]), None);
    }
    assert_eq!(WindowState::from_code(0), None);
    assert_eq!(WindowState::from_code(5), None);
    let words: Vec<_> = [
        WindowState::Open,
        WindowState::Closed,
        WindowState::Completed,
        WindowState::Abandoned,
    ]
    .map(WindowState::word)
    .to_vec();
    assert_eq!(words, ["open", "closed", "completed", "abandoned"]);
}

/// Verify r2 follow-up: every directory the keeper creates is synced into
/// its parent, and every pack file's first bytes sync its directory, a
/// `.rsb` born after its month's `.rsr` included. BEFORE the fix the July
/// append below synced no directory: July's `.rsr` already held the pointer,
/// so only a receipts file born empty was followed by a directory sync.
#[test]
fn every_directory_created_and_every_first_write_syncs_its_parent() {
    let scratch = Scratch::new("ia8-dir-sync");
    let key = nifty();
    let dir = pack_dir(scratch.root(), pack(&key)).unwrap();
    DIR_SYNCS.with(|s| s.borrow_mut().clear());
    append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(93))),
        &landing(day(2019, 6, 28), day(2019, 7, 2)),
    )
    .unwrap();
    let synced = DIR_SYNCS.with(std::cell::RefCell::take);
    // Each created directory's parent, root to pack.
    let mut created = Vec::new();
    let mut at = dir.as_path();
    while at != scratch.root() {
        created.push(at.parent().unwrap().to_path_buf());
        at = at.parent().unwrap();
    }
    created.reverse();
    assert!(
        synced.len() >= created.len(),
        "every created directory's parent is synced: {synced:?}"
    );
    assert_eq!(&synced[..created.len()], created.as_slice(), "{synced:?}");
    assert!(
        synced[created.len()..].iter().all(|d| *d == dir),
        "{synced:?}"
    );
    // June's .rsb and .rsr, July's .rsr, and the journal: four first writes.
    assert_eq!(synced.len() - created.len(), 4, "{synced:?}");
    let july = names(&dir, july_month(), 0);
    assert!(july.receipts.exists() && !july.bodies.exists());
    append(
        scratch.root(),
        pack(&key),
        Offered::Answer(&answered(&invented(94))),
        &landing(day(2019, 7, 8), day(2019, 7, 12)),
    )
    .unwrap();
    assert!(july.bodies.exists());
    let synced = DIR_SYNCS.with(std::cell::RefCell::take);
    assert_eq!(
        synced,
        std::slice::from_ref(&dir),
        "July's late .rsb synced its directory"
    );
}
