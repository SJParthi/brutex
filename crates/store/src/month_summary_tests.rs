//! The month summary's proofs (R3-06, D-4980): written with the month and
//! equal to a summary of a full read; refused by name when torn, foreign,
//! stale or missing; carried unchanged by a sealed month; and every extreme
//! of its input counted rather than smoothed.
//!
//! Every bar is invented: stamps in 2031-05 at prices no index printed. No
//! test removes anything: each root is a new directory of its own.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use brutex_core::vendor::Vendor;

use super::*;
use crate::file::{Appended, BarFile, Summarized};
use crate::format::{Bar, OI_NULL};
use crate::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use crate::time_index::INDEX_MIN_ROWS;

const SYMBOL: u32 = 41;
const MICROS: i64 = 1_000_000;

fn bar(ts_micros: i64, price: i64) -> Bar {
    Bar {
        ts_micros,
        open: price,
        high: price + 10,
        low: price - 10,
        close: price + 1,
        volume: 2,
        open_interest: OI_NULL,
    }
}

fn may() -> YearMonth {
    YearMonth::new(2031, 5).unwrap()
}

/// A new, empty store root of this test's own. Nothing is ever removed.
fn root(tag: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let at = std::env::temp_dir().join(format!(
        "brutex-msum-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&at).unwrap();
    at
}

fn path(timeframe: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "NIFTY",
        contract: None,
        timeframe,
        month: may(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// The stamp of `second` seconds past IST midnight on day `day` (1-based) of
/// May 2031.
fn at(day: i64, second: i64) -> i64 {
    (first_day_of(may()) + day - 1) * 86_400 * MICROS + (second - 19_800) * MICROS
}

/// `count` bars `step` seconds apart from 09:15 on `day`, skipping every
/// `hole`-th one when `hole > 0`.
fn session(day: i64, count: i64, step: i64, hole: i64) -> Vec<Bar> {
    (0..count)
        .filter(|k| hole == 0 || k % hole != hole - 1)
        .map(|k| bar(at(day, 33_300 + k * step), 70_000 + k))
        .collect()
}

fn committed(file: &BarFile) -> Vec<Bar> {
    (0..file.header().n_valid)
        .map(|index| file.read_record(index).unwrap())
        .collect()
}

#[test]
fn a_month_summary_is_written_with_the_month_and_matches_a_full_read() {
    for (timeframe, step) in [(Timeframe::SECOND_1, 1), (Timeframe::MINUTE_1, 60)] {
        let root = root("written");
        let mut writer = BarFile::open_or_create(&root, path(timeframe), SYMBOL).unwrap();
        // UNDER THE THRESHOLD: no `.sum`, and the reader names why.
        let day5 = session(5, 375, step, 7);
        assert!(matches!(
            writer.append(&day5),
            Ok(Appended::Committed { .. })
        ));
        let beside = path_beside(writer.path());
        assert!(
            !beside.exists(),
            "no summary beside a month under 1,024 bars"
        );
        assert_eq!(
            read_beside(writer.path(), &writer.header(), may()),
            Err(Refused::BelowThreshold)
        );
        // THE COMMIT THAT REACHES IT writes the summary whole ...
        let day6 = session(6, 800, step, 11);
        writer.append(&day6).unwrap();
        assert!(writer.header().n_valid >= INDEX_MIN_ROWS);
        let kept = read_beside(writer.path(), &writer.header(), may()).unwrap();
        assert_eq!(
            kept,
            MonthSummary::build(&writer.header(), may(), &committed(&writer))
        );
        // ... and every later commit extends it, to the same bytes a full read
        // builds: a pure function of the committed rows.
        for day in [7, 8, 9] {
            writer.append(&session(day, 375, step, 13)).unwrap();
            let kept = read_beside(writer.path(), &writer.header(), may()).unwrap();
            let full = MonthSummary::build(&writer.header(), may(), &committed(&writer));
            assert_eq!(kept, full, "{timeframe:?} day {day}");
            assert_eq!(fs::read(&beside).unwrap(), full.file_image());
        }
        let rows = committed(&writer);
        let kept = read_beside(writer.path(), &writer.header(), may()).unwrap();
        // THE FACTS THE GAP AUDIT READS, against the rows themselves.
        let mut minutes: Vec<i64> = rows
            .iter()
            .map(|b| b.ts_micros / 60_000_000 * 60_000_000)
            .collect();
        minutes.dedup();
        assert_eq!(kept.minute_stamps(), minutes);
        assert_eq!(
            kept.bars_on(first_day_of(may()) + 5),
            u32::try_from(day6.len()).unwrap()
        );
        assert_eq!(kept.counts.folded, writer.header().n_valid);
        assert_eq!(kept.counts.invalid, 0);
        assert_eq!(kept.extremes.open, rows[0].open);
        assert_eq!(kept.extremes.close, rows.last().unwrap().close);
        assert_eq!(
            kept.extremes.high,
            rows.iter().map(|b| b.high).max().unwrap()
        );
        if timeframe == Timeframe::SECOND_1 {
            assert!(
                kept.keeps_seconds(),
                "a dense one-second month keeps its seconds"
            );
            let day = first_day_of(may()) + 6;
            let held: Vec<u32> = rows
                .iter()
                .filter(|b| ist(b.ts_micros).0 == day)
                .map(|b| ist(b.ts_micros).1)
                .collect();
            assert_eq!(kept.seconds_on(day).unwrap(), held);
        } else {
            assert!(
                !kept.keeps_seconds(),
                "a minute month spans 60 seconds a bar"
            );
            assert_eq!(kept.seconds_on(first_day_of(may())), None);
        }
        // THE REBUILD DOOR AGREES AND CHANGES NOTHING.
        let before = fs::read(&beside).unwrap();
        drop(writer);
        let again = BarFile::month_summary_existing(&root, path(timeframe), SYMBOL).unwrap();
        assert_eq!(again, Summarized::Kept);
        assert_eq!(fs::read(&beside).unwrap(), before);
    }
}

#[test]
fn a_sealed_month_carries_the_summary_section_byte_for_byte() {
    let root = root("sealed");
    let mut writer = BarFile::open_or_create(&root, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    writer.append(&session(12, 1_500, 1, 9)).unwrap();
    let rows = committed(&writer);
    let summary = read_beside(writer.path(), &writer.header(), may()).unwrap();
    let payload = summary.payload();
    let built = crate::sealed::build(
        path(Timeframe::SECOND_1),
        &writer.header(),
        &rows,
        None,
        Some(crate::sealed::Summary {
            version: VERSION,
            bytes: &payload,
        }),
    )
    .unwrap();
    let bytes = built.to_bytes();
    let sealed = crate::sealed::SealedMonth::open(
        bytes.clone(),
        path(Timeframe::SECOND_1).with_file(FileKind::SealedBars),
        SYMBOL,
    )
    .unwrap();
    let (version, carried) = sealed.summary().unwrap().unwrap();
    assert_eq!(version, VERSION);
    let decoded = MonthSummary::decode(version, &carried).unwrap();
    assert_eq!(decoded, summary);
    decoded.describes(&writer.header(), may()).unwrap();
    // THE `.sum` BODY IS THE SECTION: the sealer copies it, changes nothing.
    let image = fs::read(path_beside(writer.path())).unwrap();
    let section = &image[MAGIC.len()..];
    assert!(
        bytes.windows(section.len()).any(|window| window == section),
        "the sealed file holds the `.sum` section byte for byte"
    );
}

#[test]
fn a_torn_or_missing_summary_is_named_and_the_read_falls_back() {
    let root = root("torn");
    let mut writer = BarFile::open_or_create(&root, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    writer.append(&session(5, 1_200, 1, 0)).unwrap();
    let header = writer.header();
    let beside = path_beside(writer.path());
    let good = fs::read(&beside).unwrap();
    let refused = |image: &[u8]| {
        fs::write(&beside, image).unwrap();
        read_beside(writer.path(), &header, may()).unwrap_err()
    };
    assert_eq!(refused(&good[..good.len() - 1]), Refused::Checksum);
    assert_eq!(refused(&good[..10]), Refused::Torn);
    assert_eq!(refused(&[]), Refused::Torn);
    let mut flipped = good.clone();
    flipped[200] ^= 0x10;
    assert_eq!(refused(&flipped), Refused::Checksum);
    let mut magic = good.clone();
    magic[0] = b'X';
    assert_eq!(refused(&magic), Refused::Magic);
    // A WELL-FORMED SUMMARY OF OTHER BARS is refused by the header alone.
    let mut other = MonthSummary::build(&header, may(), &committed(&writer));
    other.n_valid -= 1;
    assert!(matches!(
        refused(&other.file_image()),
        Refused::Stale { .. }
    ));
    let mut foreign = MonthSummary::build(&header, may(), &committed(&writer));
    foreign.symbol_id += 1;
    assert_eq!(refused(&foreign.file_image()), Refused::NotThisMonth);
    // EVERY REFUSAL IS NAMED, AND NONE READS AS "COMPLETE".
    for why in [
        Refused::Torn,
        Refused::Checksum,
        Refused::Magic,
        Refused::Absent,
    ] {
        assert!(why.to_string().starts_with(why.reason()), "{why}");
    }
    // MISSING: named `absent` above the threshold.
    let moved = beside.with_extension("moved");
    fs::rename(&beside, &moved).unwrap();
    assert_eq!(
        read_beside(writer.path(), &header, may()),
        Err(Refused::Absent)
    );
    // THE NEXT WRITER OPEN (or commit) WRITES IT WHOLE AGAIN, over a torn one.
    fs::write(&beside, &good[..good.len() / 2]).unwrap();
    drop(writer);
    let writer = BarFile::open_or_create(&root, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    assert_eq!(fs::read(&beside).unwrap(), good);
    // And the rebuild door names what it found.
    fs::write(&beside, b"BRUTEXU1").unwrap();
    drop(writer);
    let built = BarFile::month_summary_existing(&root, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    assert!(
        matches!(&built, Summarized::Built(why) if why.starts_with("torn")),
        "{built:?}"
    );
    assert_eq!(fs::read(&beside).unwrap(), good);
}

/// Runs `read` on its own thread and fails the test if it has not answered
/// within ten seconds: a read that waits on a pipe is released by opening the
/// pipe's other end, then named.
fn answers_at_once<T: Send + 'static>(
    pipe: &std::path::Path,
    read: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (send, answer) = std::sync::mpsc::channel();
    let _thread = std::thread::spawn(move || send.send(read()));
    let Ok(answered) = answer.recv_timeout(std::time::Duration::from_secs(10)) else {
        let _release = fs::OpenOptions::new().write(true).open(pipe);
        panic!("a read of {} waited for a writer", pipe.display());
    };
    answered
}

/// A `.sum` name held by a link, a directory or a FIFO is refused by name: a
/// link is never followed nor written through, a FIFO never makes a request
/// wait, and the writer's next open rebuilds the summary once the name is a
/// plain file's again. Fails before the reader opened with `O_NOFOLLOW |
/// O_NONBLOCK` and checked the handle's own type: `File::open` followed the
/// link and read a good summary through it, and blocked on the FIFO.
#[test]
fn a_linked_or_special_summary_is_refused_and_never_followed() {
    let root = root("special");
    let mut writer = BarFile::open_or_create(&root, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    writer.append(&session(5, 1_200, 1, 0)).unwrap();
    let header = writer.header();
    let beside = path_beside(writer.path());
    let good = fs::read(&beside).unwrap();
    // A LINK TO A GOOD SUMMARY of these very bars: still refused.
    let elsewhere = root.join("elsewhere.sum");
    fs::rename(&beside, &elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &beside).unwrap();
    let why = read_beside(writer.path(), &header, may()).unwrap_err();
    assert!(
        matches!(&why, Refused::Unreadable(text) if text.contains("symbolic link")),
        "{why}"
    );
    // The writer's step neither follows it nor writes through it, and the
    // bars it follows are committed all the same.
    writer.append(&session(6, 10, 1, 0)).unwrap();
    assert_eq!(writer.header().n_valid, 1_210);
    assert!(
        fs::symlink_metadata(&beside)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&elsewhere).unwrap(), good);
    let header = writer.header();
    // A DIRECTORY at the name: refused before a byte is read.
    fs::rename(&beside, root.join("link.aside")).unwrap();
    fs::create_dir(&beside).unwrap();
    let why = read_beside(writer.path(), &header, may()).unwrap_err();
    assert!(
        matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
        "{why}"
    );
    // A FIFO with no writer: answered at once, refused by name.
    fs::rename(&beside, root.join("dir.aside")).unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(&beside)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made {}", beside.display());
    let (bars, pipe) = (writer.path().to_path_buf(), beside.clone());
    let why = answers_at_once(&pipe, move || {
        read_beside(&bars, &header, may()).unwrap_err()
    });
    assert!(
        matches!(&why, Refused::Unreadable(text) if text.contains("not a regular file")),
        "{why}"
    );
    // A PLAIN NAME AGAIN: the next writer open writes the summary whole, and
    // it is what a full read of the committed bars builds.
    fs::rename(&beside, root.join("fifo.aside")).unwrap();
    drop(writer);
    let writer = BarFile::open_or_create(&root, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    let kept = read_beside(writer.path(), &writer.header(), may()).unwrap();
    assert_eq!(
        kept,
        MonthSummary::build(&writer.header(), may(), &committed(&writer))
    );
}

#[test]
fn a_sparse_month_keeps_no_seconds_until_an_append_makes_it_dense() {
    let root = root("density");
    let mut writer = BarFile::open_or_create(&root, path(Timeframe::SECOND_1), SYMBOL).unwrap();
    // 1,100 prints ten seconds apart: a tenth of the seconds they span.
    writer.append(&session(5, 1_100, 10, 0)).unwrap();
    let sparse = read_beside(writer.path(), &writer.header(), may()).unwrap();
    assert!(!sparse.keeps_seconds() && !sparse.tracks_seconds());
    assert_eq!(sparse.seconds_on(first_day_of(may()) + 4), None);
    let small = sparse.file_image().len();
    // Four dense days: now more than half the spanned seconds are held, and
    // the month is written whole once, with its seconds.
    for day in 6..=9 {
        writer.append(&session(day, 20_000, 1, 0)).unwrap();
    }
    let dense = read_beside(writer.path(), &writer.header(), may()).unwrap();
    assert!(dense.keeps_seconds(), "{:?}", dense.counts);
    assert_eq!(
        dense,
        MonthSummary::build(&writer.header(), may(), &committed(&writer))
    );
    assert!(dense.file_image().len() > small);
}

#[test]
fn every_extreme_of_the_input_is_counted_not_smoothed() {
    let mut summary = MonthSummary::new(SYMBOL, 60, may());
    assert_eq!(summary.minute_stamps(), Vec::<i64>::new(), "empty");
    let minute = at(3, 33_300);
    summary.fold(&bar(minute, 100));
    summary.fold(&bar(minute, 90)); // duplicate stamp
    summary.fold(&bar(minute + 5 * MICROS, 80)); // off the minute grid
    summary.fold(&bar(minute - 60 * MICROS, 70)); // goes backwards
    let mut negative = bar(minute + 120 * MICROS, 60);
    negative.volume = -1;
    summary.fold(&negative);
    summary.fold(&bar(at(40, 33_300), 50)); // a day the month does not have
    assert_eq!(summary.counts.folded, 6);
    assert_eq!(summary.counts.invalid, 3);
    assert_eq!(summary.counts.decreasing, 1);
    assert_eq!(summary.counts.negative_volume, 1);
    assert_eq!(summary.counts.outside_month, 1);
    // Minutes held, counted as the fold buckets them: the duplicate and the
    // off-grid stamp share the first row's minute, the rest are new.
    assert_eq!(summary.counts.held_minutes, 4);
    // The earliest bar holding an extreme is the one named.
    assert_eq!(summary.extremes.high, 110);
    assert_eq!(summary.extremes.high_ts, minute);
    assert_eq!(summary.extremes.low, 40);
    // A single bar.
    let mut one = MonthSummary::new(SYMBOL, 1, may());
    one.fold(&bar(at(31, 86_399), 5));
    assert_eq!(one.minute_stamps(), vec![at(31, 86_340)]);
    assert_eq!(one.seconds_on(first_day_of(may()) + 30), Some(vec![86_399]));
    let round = MonthSummary::from_section(&one.section()).unwrap();
    assert_eq!(round, one);
}

#[test]
fn the_largest_month_fits_the_sealed_section() {
    // Every second of every day of a 31-day month: the worst geometry.
    let mut full = MonthSummary::new(SYMBOL, 1, may());
    for day in 1..=31 {
        for second in 0..86_400 {
            full.fold(&bar(at(day, second), 1));
        }
    }
    let payload = full.payload();
    assert_eq!(payload.len(), MAX_PAYLOAD_LEN);
    assert!(payload.len() <= crate::sealed::MAX_SUMMARY_LEN);
    assert_eq!(MonthSummary::decode(VERSION, &payload).unwrap(), full);
}

#[test]
fn a_payload_that_is_not_its_geometry_is_refused_by_name() {
    let mut summary = MonthSummary::new(SYMBOL, 60, may());
    for bar in session(2, 375, 60, 0) {
        summary.fold(&bar);
    }
    let payload = summary.payload();
    let shape = |bytes: &[u8]| MonthSummary::decode(VERSION, bytes).unwrap_err();
    assert_eq!(
        MonthSummary::decode(2, &payload).unwrap_err(),
        Refused::Version(2)
    );
    assert!(matches!(shape(&payload[..HEAD_LEN - 1]), Refused::Shape(_)));
    assert!(matches!(
        shape(&payload[..payload.len() - 1]),
        Refused::Shape(_)
    ));
    let mut longer = payload.clone();
    longer.push(0);
    assert!(matches!(shape(&longer), Refused::Shape(_)));
    let mut flag = payload.clone();
    flag[12] |= 0b10;
    assert!(matches!(shape(&flag), Refused::Shape(why) if why.contains("flag")));
    let mut reserved = payload.clone();
    reserved[150] = 1;
    assert!(matches!(shape(&reserved), Refused::Shape(why) if why.contains("reserved")));
    // The last minute bitmap's padding bits are not minutes: a set one is
    // refused, so one summary has one encoding.
    let mut padded = payload.clone();
    *padded.last_mut().unwrap() |= 0x80;
    assert!(matches!(shape(&padded), Refused::Shape(why) if why.contains("past its range")));
    let mut days = payload;
    days[11] = 30;
    assert!(matches!(shape(&days), Refused::Shape(why) if why.contains("day count")));
}

/// A deterministic stream of numbers, so the offsets below are many and
/// repeatable.
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state >> 33
}

#[test]
fn a_bitmap_at_any_offset_round_trips_and_lists_exactly_its_bits() {
    // Days whose first held second falls on every residue of a byte and of a
    // word, dense enough that the seconds are kept: the encoder's shifts and
    // the decoder's must agree at every boundary, and the last byte of each
    // range must carry no padding bit.
    let mut state = 7_u64;
    for round in 0..40_i64 {
        let mut summary = MonthSummary::new(SYMBOL, 1, may());
        let mut expected: Vec<Vec<u32>> = vec![Vec::new(); 31];
        for day in 1..=31_i64 {
            if (day + round) % 4 == 0 {
                continue; // a day with no bar
            }
            let first = 33_300 + i64::try_from(lcg(&mut state) % 130).unwrap();
            let len = 1 + i64::try_from(lcg(&mut state) % 700).unwrap();
            for second in first..first + len {
                // Most seconds held, some not: never below half.
                if !lcg(&mut state).is_multiple_of(5)
                    || second == first
                    || second == first + len - 1
                {
                    summary.fold(&bar(at(day, second), 7));
                    expected[usize::try_from(day - 1).unwrap()]
                        .push(u32::try_from(second).unwrap());
                }
            }
        }
        assert!(
            summary.keeps_seconds(),
            "round {round}: {:?}",
            summary.counts
        );
        let decoded = MonthSummary::from_section(&summary.section()).unwrap();
        assert_eq!(decoded, summary, "round {round}");
        for (offset, held) in (0_i64..).zip(&expected) {
            let day = first_day_of(may()) + offset;
            assert_eq!(
                decoded.seconds_on(day).as_ref(),
                Some(held),
                "round {round} day {offset}"
            );
            let mut minutes: Vec<u32> = held.iter().map(|s| s / 60).collect();
            minutes.dedup();
            let base = (day * 86_400 - 19_800) * MICROS;
            let listed: Vec<u32> = decoded
                .minute_stamps()
                .into_iter()
                .filter(|stamp| ist(*stamp).0 == day)
                .map(|stamp| u32::try_from((stamp - base) / (60 * MICROS)).unwrap())
                .collect();
            assert_eq!(listed, minutes, "round {round} day {offset}");
        }
        assert_eq!(
            u64::from(decoded.counts.held_minutes),
            expected
                .iter()
                .map(|held| {
                    let mut minutes: Vec<u32> = held.iter().map(|s| s / 60).collect();
                    minutes.dedup();
                    minutes.len() as u64
                })
                .sum::<u64>()
        );
    }
}
