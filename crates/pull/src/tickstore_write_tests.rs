#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a test that cannot panic cannot fail"
)]
//! The tick-store writer's tests (E2E-02, R4-08). Every zip here is built in
//! code from invented files: no vendor row is quoted (D-2800), and the prices
//! are plainly not market prices.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use super::*;
use crate::gdfl_cm::CmSource as _;
use crate::gdfl_fixtures::scratch;
use crate::gdfl_nfo::NfoSource as _;

// ── a zip builder with every knob the extreme cases need ───────────────────

/// One member as the builder writes it.
#[derive(Clone)]
struct Member {
    name: Vec<u8>,
    data: Vec<u8>,
    deflate: bool,
    /// DOS time and date.
    time: u16,
    date: u16,
    /// What the central record states instead of the truth, when set.
    crc: Option<u32>,
    len: Option<u64>,
}

fn member(name: &str, data: &[u8]) -> Member {
    Member {
        name: name.as_bytes().to_vec(),
        data: data.to_vec(),
        deflate: false,
        time: 0x7A_2F,
        date: 0x5B_39,
        crc: None,
        len: None,
    }
}

fn deflated(name: &str, data: &[u8]) -> Member {
    Member {
        deflate: true,
        time: 0x6B_10,
        ..member(name, data)
    }
}

/// A zip of `members`; a stated length past `u32` goes in a ZIP64 extra block.
fn zip_of(members: &[Member]) -> Vec<u8> {
    let le16 = |out: &mut Vec<u8>, v: u16| out.extend_from_slice(&v.to_le_bytes());
    let le32 = |out: &mut Vec<u8>, v: u32| out.extend_from_slice(&v.to_le_bytes());
    let mut out = Vec::new();
    let mut central = Vec::new();
    for m in members {
        let packed = if m.deflate {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(&m.data).unwrap();
            enc.finish().unwrap()
        } else {
            m.data.clone()
        };
        let method = if m.deflate { 8 } else { 0 };
        let crc = m.crc.unwrap_or_else(|| crc32(&m.data));
        let len = m.len.unwrap_or(m.data.len() as u64);
        let (stated32, extra) = if len > u64::from(u32::MAX) {
            let mut extra = vec![1, 0, 8, 0];
            extra.extend_from_slice(&len.to_le_bytes());
            (u32::MAX, extra)
        } else {
            (u32::try_from(len).unwrap(), Vec::new())
        };
        let offset = u32::try_from(out.len()).unwrap();
        le32(&mut out, 0x0403_4b50);
        le16(&mut out, 20);
        le16(&mut out, 0);
        le16(&mut out, method);
        le16(&mut out, m.time);
        le16(&mut out, m.date);
        le32(&mut out, crc);
        le32(&mut out, u32::try_from(packed.len()).unwrap());
        le32(&mut out, stated32.min(u32::try_from(m.data.len()).unwrap()));
        le16(&mut out, u16::try_from(m.name.len()).unwrap());
        le16(&mut out, 0);
        out.extend_from_slice(&m.name);
        out.extend_from_slice(&packed);
        le32(&mut central, 0x0201_4b50);
        le16(&mut central, 20);
        le16(&mut central, 45);
        le16(&mut central, 0);
        le16(&mut central, method);
        le16(&mut central, m.time);
        le16(&mut central, m.date);
        le32(&mut central, crc);
        le32(&mut central, u32::try_from(packed.len()).unwrap());
        le32(&mut central, stated32);
        le16(&mut central, u16::try_from(m.name.len()).unwrap());
        le16(&mut central, u16::try_from(extra.len()).unwrap());
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(&m.name);
        central.extend_from_slice(&extra);
    }
    let at = u32::try_from(out.len()).unwrap();
    let count = u16::try_from(members.len()).unwrap();
    out.extend_from_slice(&central);
    le32(&mut out, 0x0605_4b50);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, count);
    le16(&mut out, count);
    le32(&mut out, u32::try_from(central.len()).unwrap());
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

/// An invented regular CSV: a header, then `rows` rows of a text ticker, a
/// text date, a time, a decimal price, a signed integer and a zero-padded
/// code, ended by `terminator` after every line when `trailing`.
fn csv(ticker: &str, rows: u32, terminator: &str, trailing: bool) -> Vec<u8> {
    let mut lines = vec!["Ticker,Date,Time,LTP,Change,Code".to_owned()];
    for r in 0..rows {
        let secs = 33_300 + r * 7;
        lines.push(format!(
            "{ticker},31/12/1999,{:02}:{:02}:{:02},{}.{:02},{},{:03}",
            secs / 3_600,
            secs / 60 % 60,
            secs % 60,
            99_000 + r % 13,
            (r * 37) % 100,
            i64::from(r % 9) - 4,
            r % 1_000,
        ));
    }
    let mut text = lines.join(terminator);
    if trailing {
        text.push_str(terminator);
    }
    text.into_bytes()
}

const IDX: &str = "GFDLCM_INDICES_TICK_25092026";
const NFO: &str = "GFDLNFO_TICK_29092026";

/// The NIFTY spot day: every block kind and line shape in one zip.
fn nifty_spot_members() -> Vec<Member> {
    vec![
        member(&format!("{IDX}/"), b""),
        member(
            &format!("{IDX}/NIFTY.NSE_IDX.csv"),
            &csv("NIFTY", 400, "\r\n", true),
        ),
        deflated(
            &format!("{IDX}/NIFTY BANK.NSE_IDX.csv"),
            &csv("BANKNIFTY", 300, "\r\n", true),
        ),
        member(
            &format!("{IDX}/LF.NSE_IDX.csv"),
            &csv("LF", 50, "\n", false),
        ),
        member(
            &format!("{IDX}/MIXED.NSE_IDX.csv"),
            b"Ticker,Time\r\nMIXED,09:15:00\nMIXED,09:15:01\r\n",
        ),
        member(&format!("{IDX}/EMPTY.NSE_IDX.csv"), b""),
        member(&format!("{IDX}/HEADER.NSE_IDX.csv"), b"Ticker,Time\r\n"),
        member("README.txt", b"invented text\r\noutside the day folder\n"),
    ]
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// `members` zipped as `name` in a fresh scratch folder, and a fresh root.
fn staged(tag: &str, name: &str, members: &[Member]) -> (PathBuf, PathBuf) {
    let dir = scratch(tag);
    let zip = dir.join("in").join(name);
    std::fs::create_dir_all(zip.parent().unwrap()).unwrap();
    std::fs::write(&zip, zip_of(members)).unwrap();
    // The writer never makes a root (`root_missing`): the root is made here.
    let root = dir.join("store");
    std::fs::create_dir_all(&root).unwrap();
    (zip, root)
}

fn options(run: &str) -> WriteOptions {
    WriteOptions::new(run).with_workers(3)
}

/// Every name in `dir`, sorted.
fn names_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|listing| {
            listing
                .flatten()
                .map(|item| item.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn log_of(root: &Path) -> String {
    std::fs::read_to_string(root.join(LOG_NAME)).unwrap_or_default()
}

// ── the failing-first tests the package names ──────────────────────────────

/// **EVERY ENTRY OF THE ZIP COMES BACK BYTE FOR BYTE, THROUGH THE READERS.**
/// NIFTY spot first: the day file lands exactly where `TickStore` opens it,
/// every index field is the zip's, and every rebuilt file is the member.
#[test]
fn a_bts_round_trips_every_entry_byte_identical() {
    let members = nifty_spot_members();
    let (zip, root) = staged("tsw-rt", &format!("{IDX}.zip"), &members);
    let run = write_day(&zip, &root, &options("rt-1"));
    let Ok(Outcome::Written(report)) = &run.result else {
        panic!("written: {:?}", run.result);
    };
    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    assert_eq!(report.path, path);
    assert!(path.ends_with("cm/INDICES/2026/SEP_2026/GFDLCM_INDICES_TICK_25092026.bts"));
    assert_eq!(report.entries, 8);
    assert_eq!(report.columnar, 4, "the four regular CSVs");
    assert_eq!(report.raw_fallbacks, 3, "mixed, empty and the readme");
    assert_eq!(report.rows, 400 + 300 + 50);
    assert_eq!(report.outside_folder, 1);
    assert_eq!(
        report.csv_bytes,
        members.iter().map(|m| m.data.len() as u64).sum::<u64>()
    );
    assert_eq!(report.stored_bytes, std::fs::metadata(&path).unwrap().len());
    // The index, read by the workspace's reader.
    let file = File::open(&path).unwrap();
    let index = read_index(&file, report.stored_bytes).unwrap();
    assert_eq!(index.len(), members.len());
    for (stored, m) in index.iter().zip(&members) {
        assert_eq!(stored.name.as_bytes(), m.name.as_slice());
        assert_eq!(stored.size, m.data.len() as u64);
        assert_eq!(stored.crc, crc32(&m.data));
        assert_eq!(
            stored.dos_time,
            u32::from(m.date) << 16 | u32::from(m.time),
            "DOS time kept"
        );
        assert_eq!(rebuild(&file, stored).unwrap(), m.data, "{}", stored.name);
    }
    assert_eq!(index[0].kind, KIND_DIR);
    assert_eq!(index[1].kind, KIND_COLUMNAR);
    assert_eq!(index[4].kind, KIND_RAW);
    // And through the capital-market source, by its own path rule.
    let store = TickStore::new(&root);
    let listing = store
        .day(CmKind::Indices, day(2026, 9, 25))
        .unwrap()
        .expect("the reader opens the written day");
    assert_eq!(listing.entries().len(), members.len());
    for (listed, m) in listing.entries().iter().zip(&members) {
        assert_eq!(store.fetch(&listing, listed).unwrap(), m.data);
    }
    // Nothing else beside it, and one log line naming this writer.
    assert_eq!(names_in(path.parent().unwrap()), vec![format!("{IDX}.bts")]);
    let log = log_of(&root);
    assert_eq!(log.lines().count(), 1);
    assert!(log.contains("verdict=written"), "{log}");
    assert!(log.contains(WRITER), "{log}");
    assert!(log.contains("raw_fallbacks=3"), "{log}");
}

/// **NIFTY OPTIONS: THE OPTIONS READER OPENS THE WRITTEN DAY.**
#[test]
fn a_nifty_options_day_is_read_back_by_the_options_reader() {
    let call = csv("NIFTY30SEP2699000CE", 200, "\r\n", true);
    let put = csv("NIFTY30SEP2699000PE", 150, "\r\n", true);
    let thin = csv("NIFTY30SEP2699500CE", 1, "\r\n", true);
    let members = vec![
        member(&format!("{NFO}/"), b""),
        member(&format!("{NFO}/Options/"), b""),
        deflated(&format!("{NFO}/Options/NIFTY30SEP2699000CE.NFO.csv"), &call),
        member(&format!("{NFO}/Options/NIFTY30SEP2699000PE.NFO.csv"), &put),
        member(&format!("{NFO}/Options/NIFTY30SEP2699500CE.NFO.csv"), &thin),
    ];
    let (zip, root) = staged("tsw-nfo", &format!("{NFO}.zip"), &members);
    let run = write_day(&zip, &root, &options("nfo-1"));
    assert!(matches!(run.result, Ok(Outcome::Written(_))), "{run:?}");
    let store = NfoTickStore::new(&root);
    assert!(
        store
            .day_path(day(2026, 9, 29))
            .ends_with("options/2026/SEP_2026/GFDLNFO_TICK_29092026.bts")
    );
    let listed = store.day(day(2026, 9, 29)).unwrap().expect("opened");
    for (ticker, want) in [
        ("NIFTY30SEP2699000CE", &call),
        ("NIFTY30SEP2699000PE", &put),
        ("NIFTY30SEP2699500CE", &thin),
    ] {
        let file = listed.locate(ticker).unwrap().expect(ticker);
        assert_eq!(&store.fetch(file).unwrap(), want, "{ticker}");
    }
}

/// **A `.bts.tmp*` LEFT BY A CRASH IS REFUSED BY NAME AND LEFT.** The tool's
/// own `.bts.tmp` and another run's `.bts.tmp.<id>` alike; this code removes
/// no file.
#[test]
fn a_bts_tmp_left_by_a_crash_is_refused_and_left() {
    for leftover in [
        format!("{IDX}.bts.tmp"),
        format!("{IDX}.bts.tmp.crashed-run"),
    ] {
        let (zip, root) = staged("tsw-tmp", &format!("{IDX}.zip"), &nifty_spot_members());
        let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join(&leftover), b"half a day file").unwrap();
        let run = write_day(&zip, &root, &options("after-crash"));
        assert_eq!(
            run.result,
            Err(Refusal::StaleTemp {
                leftovers: vec![(leftover.clone(), 15)]
            })
        );
        assert_eq!(
            std::fs::read(dir.join(&leftover)).unwrap(),
            b"half a day file",
            "left as it was"
        );
        assert!(!path.exists(), "the day is not held");
        assert_eq!(names_in(dir), vec![leftover.clone()]);
        let log = log_of(&root);
        assert!(log.contains("verdict=refused code=stale_temp"), "{log}");
        assert!(log.contains(&leftover), "{log}");
    }
}

/// **A RERUN ON A HELD, VERIFIED DAY WRITES NOTHING**, with the same run id
/// or another one: the day file's bytes and modification time are unchanged,
/// no temporary appears, and the log says `held`.
#[test]
fn a_second_tickstore_run_writes_nothing() {
    let (zip, root) = staged("tsw-rerun", &format!("{IDX}.zip"), &nifty_spot_members());
    let first = write_day(&zip, &root, &options("first"));
    let Ok(Outcome::Written(written)) = first.result else {
        panic!("{first:?}");
    };
    let bytes = std::fs::read(&written.path).unwrap();
    let modified = std::fs::metadata(&written.path)
        .unwrap()
        .modified()
        .unwrap();
    for run_id in ["first", "second"] {
        let again = write_day(&zip, &root, &options(run_id));
        let Ok(Outcome::Held(held)) = again.result else {
            panic!("{again:?}");
        };
        assert_eq!(held.path, written.path);
        assert_eq!(held.entries, written.entries);
        assert_eq!(held.raw_fallbacks, written.raw_fallbacks);
    }
    assert_eq!(std::fs::read(&written.path).unwrap(), bytes);
    assert_eq!(
        std::fs::metadata(&written.path)
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
    assert_eq!(
        names_in(written.path.parent().unwrap()),
        vec![format!("{IDX}.bts")]
    );
    let log = log_of(&root);
    assert_eq!(log.matches("verdict=written").count(), 1, "{log}");
    assert_eq!(log.matches("verdict=held").count(), 2, "{log}");
}

/// **A ZIP CHANGED AFTER ITS DAY FILE FAILS VERIFY, AND THE OLD FILE STAYS.**
#[test]
fn a_changed_zip_fails_verify_and_keeps_the_old_bts() {
    let mut members = nifty_spot_members();
    let (zip, root) = staged("tsw-changed", &format!("{IDX}.zip"), &members);
    let first = write_day(&zip, &root, &options("before"));
    let Ok(Outcome::Written(written)) = first.result else {
        panic!("{first:?}");
    };
    let before = std::fs::read(&written.path).unwrap();
    // A re-delivery: one member's bytes differ, same name, valid CRC.
    members[1].data = csv("NIFTY", 401, "\r\n", true);
    std::fs::write(&zip, zip_of(&members)).unwrap();
    let again = write_day(&zip, &root, &options("after"));
    let Err(Refusal::HeldDayDiffers { path, why }) = &again.result else {
        panic!("{again:?}");
    };
    assert_eq!(path, &written.path);
    assert!(why.contains("NIFTY.NSE_IDX.csv"), "{why}");
    assert_eq!(std::fs::read(&written.path).unwrap(), before, "untouched");
    assert!(log_of(&root).contains("code=held_day_differs"));
}

/// **THE WRITER LINKS NO C.** No `zstd`, `zstd-sys`, `memmap2` or `libc` in
/// `pull`'s dependencies, and none named in this module's code.
#[test]
fn the_writer_links_no_c() {
    let manifest = include_str!("../Cargo.toml");
    let mut section = "";
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line;
            continue;
        }
        if line.starts_with('#') || !section.contains("dependencies") {
            continue;
        }
        let key = line.split(['=', ' ', '.']).next().unwrap_or_default();
        for banned in ["zstd", "zstd-sys", "zstd-safe", "memmap2", "libc"] {
            assert_ne!(key, banned, "{section} names {banned}");
        }
    }
    let code: String = include_str!("tickstore_write.rs")
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    for banned in [
        "memmap2",
        "libc",
        "zstd_sys",
        "zstd_safe",
        "Mmap",
        "unsafe ",
    ] {
        assert!(!code.contains(banned), "the module names {banned}");
    }
    assert!(
        !code.replace("ruzstd::", "").contains("zstd::"),
        "every zstd frame is ruzstd's"
    );
    assert!(code.contains("ruzstd::encoding::{CompressionLevel, compress_to_vec}"));
}

// ── the extreme cases ──────────────────────────────────────────────────────

/// **A BAD CRC IS REFUSED AND NOTHING IS WRITTEN**: no day file, no
/// temporary, not even the day's folder.
#[test]
fn a_bad_crc_is_refused_and_nothing_is_written() {
    let mut members = nifty_spot_members();
    members[2].crc = Some(0xDEAD_BEEF);
    let (zip, root) = staged("tsw-crc", &format!("{IDX}.zip"), &members);
    let run = write_day(&zip, &root, &options("bad-crc"));
    let Err(Refusal::MemberCorrupt { name, why }) = &run.result else {
        panic!("{run:?}");
    };
    assert_eq!(name, &format!("{IDX}/NIFTY BANK.NSE_IDX.csv"));
    assert!(why.contains("deadbeef"), "{why}");
    assert!(!root.join("cm").exists(), "nothing written");
    assert!(log_of(&root).contains("code=member_corrupt"));
}

/// **A MEMBER STATED OVER 4 GiB IS REFUSED BEFORE IT IS READ**, its size read
/// through ZIP64 as a `u64`; and the index carries a size past 4 GiB intact.
#[test]
fn a_member_over_4_gib_is_a_u64_and_is_refused_before_it_is_read() {
    let big = 5_u64 << 30;
    let mut members = nifty_spot_members();
    members[3].len = Some(big);
    let (zip, root) = staged("tsw-big", &format!("{IDX}.zip"), &members);
    let run = write_day(&zip, &root, &options("big"));
    assert_eq!(
        run.result,
        Err(Refusal::MemberTooLarge {
            name: format!("{IDX}/LF.NSE_IDX.csv"),
            size: big
        })
    );
    // The index field is eight bytes: a 5 GiB size reads back as written.
    let source = File::open(&zip).unwrap();
    let len = source.metadata().unwrap().len();
    let entries = zip_entries(&source, 0, len).unwrap();
    let mut items: Vec<Item> = entries
        .iter()
        .map(|_| Item {
            kind: KIND_DIR,
            block: Vec::new(),
            rows: 0,
        })
        .collect();
    let mut raw = vec![KIND_RAW];
    raw.extend_from_slice(&zstd(b"x").unwrap());
    items[3] = Item {
        kind: KIND_RAW,
        block: raw,
        rows: 0,
    };
    let (index, index_off) = index_bytes(&entries, &items);
    let frame = zstd(&index).unwrap();
    let mut file = MAGIC.to_vec();
    for it in &items {
        file.extend_from_slice(&it.block);
    }
    file.extend_from_slice(&frame);
    file.extend_from_slice(&footer(
        index_off,
        frame.len() as u64,
        &index,
        entries.len(),
    ));
    let back = read_index(&file, file.len() as u64).unwrap();
    assert_eq!(back[3].size, big);
}

/// **THE DISK FILLS MID-WRITE**: refused by name with the temporary and its
/// bytes, the temporary left, the day not held; the next run refuses it.
#[test]
fn a_full_disk_leaves_the_temp_named_and_the_next_run_refuses_it() {
    let (zip, root) = staged("tsw-full", &format!("{IDX}.zip"), &nifty_spot_members());
    let faults = Faults {
        full_after: Some(20),
        ..Faults::default()
    };
    let run = write_day(&zip, &root, &options("full").with_faults(faults));
    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    let temp = path.with_file_name(format!("{IDX}.bts.tmp.full"));
    assert_eq!(
        run.result,
        Err(Refusal::DiskFull {
            temp: temp.clone(),
            written: 20
        })
    );
    assert_eq!(std::fs::metadata(&temp).unwrap().len(), 20);
    assert!(!path.exists());
    let next = write_day(&zip, &root, &options("next"));
    assert_eq!(
        next.result,
        Err(Refusal::StaleTemp {
            leftovers: vec![(format!("{IDX}.bts.tmp.full"), 20)]
        })
    );
    assert!(temp.exists(), "still left for the operator");
}

/// **A WRITE THAT DOES NOT READ BACK IS NOT PUBLISHED**: one byte flipped
/// between the fsync and the verify (a torn write, a crash between write and
/// verify that left damage) is caught, the temporary left and named.
#[test]
fn a_write_that_does_not_read_back_is_not_published() {
    for at in [9, 100, 500] {
        let (zip, root) = staged("tsw-flip", &format!("{IDX}.zip"), &nifty_spot_members());
        let faults = Faults {
            flip_before_verify: Some(at),
            ..Faults::default()
        };
        let run = write_day(&zip, &root, &options("flip").with_faults(faults));
        let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
        let Err(Refusal::VerifyFailed { temp, .. }) = &run.result else {
            panic!("byte {at}: {run:?}");
        };
        assert_eq!(temp, &path.with_file_name(format!("{IDX}.bts.tmp.flip")));
        assert!(temp.exists());
        assert!(!path.exists(), "byte {at}: not published");
    }
}

/// **A CONFLICT COPY OR ANY NON-CANONICAL NAME IS REFUSED BY NAME** (FM-024).
#[test]
fn a_name_that_is_not_a_canonical_day_zip_is_refused_by_name() {
    for name in [
        format!("{IDX} (1).zip"),
        format!("{IDX}.ZIP"),
        format!("{IDX}_v2.zip"),
        "GFDLCM_INDICES_TICK_31022026.zip".to_owned(),
        "GFDLNFO_TICK_2909202.zip".to_owned(),
        "GFDLNFO_TICK_2909202x.zip".to_owned(),
        "GFDLCM_STOCK_TICK_00092026.zip".to_owned(),
        "notes.zip".to_owned(),
    ] {
        assert_eq!(
            DayIdentity::of_zip_name(&name),
            Err(Refusal::NameNotCanonical { name: name.clone() }),
            "{name}"
        );
    }
    let stock = DayIdentity::of_zip_name("GFDLCM_STOCK_TICK_28092026.zip").unwrap();
    assert_eq!(stock.tree, Tree::Cm(CmKind::Stocks));
    assert!(
        stock
            .path_under(Path::new("/r"))
            .ends_with("cm/STOCKS/2026/SEP_2026/GFDLCM_STOCK_TICK_28092026.bts")
    );
    let (zip, root) = staged("tsw-name", &format!("{IDX} (1).zip"), &nifty_spot_members());
    let run = write_day(&zip, &root, &options("name"));
    assert_eq!(run.result.unwrap_err().code(), "name_not_canonical");
}

/// **THE ZIP'S ENTRIES ARE CHECKED BEFORE ANYTHING IS READ**: a duplicate
/// name, a name that is not UTF-8, a day folder another day's, no member in
/// the day folder, an empty zip.
#[test]
fn entries_the_readers_could_not_tell_apart_are_refused_by_name() {
    let mut twice = nifty_spot_members();
    twice.push(member(&format!("{IDX}/NIFTY.NSE_IDX.csv"), b"x"));
    let mut not_utf8 = nifty_spot_members();
    not_utf8[3].name = b"GFDLCM_INDICES_TICK_25092026/\xFFX.csv".to_vec();
    let mut other_day = nifty_spot_members();
    other_day.push(member("GFDLCM_INDICES_TICK_26092026/A.csv", b"a"));
    let cases: Vec<(Vec<Member>, &str)> = vec![
        (twice, "duplicate_entry"),
        (not_utf8, "name_not_utf8"),
        (other_day, "inner_folder_mismatch"),
        (
            vec![member("elsewhere/A.csv", b"a")],
            "inner_folder_mismatch",
        ),
        (Vec::new(), "inner_folder_mismatch"),
    ];
    for (members, code) in cases {
        let (zip, root) = staged("tsw-entries", &format!("{IDX}.zip"), &members);
        let run = write_day(&zip, &root, &options("entries"));
        let why = run.result.unwrap_err();
        assert_eq!(why.code(), code, "{why}");
        assert!(!root.join("cm").exists());
    }
    let mut not_utf8 = nifty_spot_members();
    not_utf8[3].name = b"GFDLCM_INDICES_TICK_25092026/\xFF.csv".to_vec();
    let (zip, root) = staged("tsw-hex", &format!("{IDX}.zip"), &not_utf8);
    assert_eq!(
        write_day(&zip, &root, &options("hex")).result,
        Err(Refusal::NameNotUtf8 {
            raw_hex: "4746444c434d5f494e44494345535f5449434b5f32353039323032362fff2e637376"
                .to_owned()
        })
    );
}

/// **A ZIP THE ARCHIVE READER REFUSES IS REFUSED WITH ITS OWN WORDS** (FM-238:
/// one zip authority): a cut zip, a missing one.
#[test]
fn an_unreadable_zip_is_refused_with_the_archive_readers_refusal() {
    let (zip, root) = staged("tsw-cut", &format!("{IDX}.zip"), &nifty_spot_members());
    let bytes = std::fs::read(&zip).unwrap();
    std::fs::write(&zip, &bytes[..bytes.len() - 30]).unwrap();
    let source = File::open(&zip).unwrap();
    let len = source.metadata().unwrap().len();
    let reader = zip_entries(&source, 0, len).unwrap_err();
    assert_eq!(
        write_day(&zip, &root, &options("cut")).result,
        Err(Refusal::ZipRefused { why: reader })
    );
    let missing = zip.with_file_name("GFDLCM_INDICES_TICK_26092026.zip");
    assert_eq!(
        write_day(&missing, &root, &options("missing")).result,
        Err(Refusal::ZipUnavailable {
            path: missing.clone(),
            kind: std::io::ErrorKind::NotFound
        })
    );
}

/// **A RUN ID THAT COULD NOT BE PART OF A FILE NAME IS REFUSED.**
#[test]
fn a_run_id_outside_its_alphabet_is_refused() {
    let (zip, root) = staged("tsw-run", &format!("{IDX}.zip"), &nifty_spot_members());
    for bad in ["", "../x", "a b", &"r".repeat(65)] {
        assert_eq!(
            write_day(&zip, &root, &options(bad)).result,
            Err(Refusal::RunIdInvalid {
                run_id: bad.to_owned()
            })
        );
    }
    assert!(run_id_ok(&"r".repeat(64)));
    assert!(run_id_ok("2026-10-14_a"));
}

/// **ANOTHER WRITER HOLDS THE ROOT**: refused by name, nothing written.
#[test]
fn a_locked_root_is_refused_and_nothing_is_written() {
    let (zip, root) = staged("tsw-lock", &format!("{IDX}.zip"), &nifty_spot_members());
    std::fs::create_dir_all(&root).unwrap();
    let holder = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join(LOCK_NAME))
        .unwrap();
    holder.try_lock().unwrap();
    assert_eq!(
        write_day(&zip, &root, &options("locked")).result,
        Err(Refusal::DayLocked {
            lock: root.join(LOCK_NAME)
        })
    );
    assert!(!root.join("cm").exists());
    drop(holder);
    assert!(matches!(
        write_day(&zip, &root, &options("unlocked")).result,
        Ok(Outcome::Written(_))
    ));
}

/// **TWO WRITERS OF ONE DAY LEAVE ONE VERIFIED FILE** (CONC-042): one writes,
/// the other is refused by the lock or finds the day held; no temporary is
/// left either way.
#[test]
fn two_writers_of_one_day_leave_one_verified_file() {
    let (zip, root) = staged("tsw-two", &format!("{IDX}.zip"), &nifty_spot_members());
    let results: Vec<Run> = std::thread::scope(|scope| {
        let a = scope.spawn(|| write_day(&zip, &root, &options("writer-a")));
        let b = scope.spawn(|| write_day(&zip, &root, &options("writer-b")));
        vec![a.join().unwrap(), b.join().unwrap()]
    });
    let written = results
        .iter()
        .filter(|run| matches!(run.result, Ok(Outcome::Written(_))))
        .count();
    assert_eq!(written, 1, "{results:?}");
    for run in &results {
        assert!(
            matches!(
                run.result,
                Ok(Outcome::Written(_) | Outcome::Held(_)) | Err(Refusal::DayLocked { .. })
            ),
            "{run:?}"
        );
    }
    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    assert_eq!(names_in(path.parent().unwrap()), vec![format!("{IDX}.bts")]);
}

/// **A DAY FILE FROM ANOTHER WRITER ANSWERS THE SAME ENTRIES** (FM-249): the
/// verify is writer-blind. A raw-only day file built by the test fixtures'
/// writer is held as verified, and its bytes differ from this writer's.
#[test]
fn a_bts_from_another_writer_answers_the_same_entries() {
    let files: Vec<(String, Vec<u8>)> = vec![
        (format!("{IDX}/"), Vec::new()),
        (
            format!("{IDX}/NIFTY.NSE_IDX.csv"),
            csv("NIFTY", 90, "\r\n", true),
        ),
        (format!("{IDX}/RAW.NSE_IDX.csv"), b"a\r\nb\n".to_vec()),
    ];
    let members: Vec<Member> = files
        .iter()
        .map(|(name, data)| Member {
            time: 0,
            date: 0,
            ..member(name, data)
        })
        .collect();
    let (zip, root) = staged("tsw-other", &format!("{IDX}.zip"), &members);
    let theirs: Vec<(&str, &[u8])> = files
        .iter()
        .map(|(name, data)| (name.as_str(), data.as_slice()))
        .collect();
    let other = crate::gdfl_fixtures::bts(&theirs);
    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &other).unwrap();
    let held = write_day(&zip, &root, &options("blind"));
    let Ok(Outcome::Held(report)) = &held.result else {
        panic!("{held:?}");
    };
    assert_eq!(report.raw_fallbacks, 2, "the other writer stored them raw");
    let ours_root = root.with_file_name("ours");
    std::fs::create_dir_all(&ours_root).unwrap();
    let ours = write_day(&zip, &ours_root, &options("ours"));
    let Ok(Outcome::Written(mine)) = &ours.result else {
        panic!("{ours:?}");
    };
    assert_eq!(mine.columnar, 1);
    assert_ne!(std::fs::read(&mine.path).unwrap(), other, "different bytes");
    let (a, b) = (File::open(&path).unwrap(), File::open(&mine.path).unwrap());
    let ia = read_index(&a, other.len() as u64).unwrap();
    let ib = read_index(&b, mine.stored_bytes).unwrap();
    for (x, y) in ia.iter().zip(&ib) {
        assert_eq!((&x.name, x.size, x.crc), (&y.name, y.size, y.crc));
        assert_eq!(rebuild(&a, x).unwrap(), rebuild(&b, y).unwrap());
    }
}

/// **TWO ENTRIES SHARING ONE BLOCK ARE REFUSED** (FM-248): `read_index` checks
/// only that a block lies inside the blocks area; the verify also holds each
/// block after the one before it.
#[test]
fn a_bts_whose_two_entries_share_a_block_is_refused() {
    let data = b"same bytes\n".to_vec();
    let members = vec![
        member(&format!("{IDX}/A.csv"), &data),
        member(&format!("{IDX}/B.csv"), &data),
    ];
    let (zip, root) = staged("tsw-share", &format!("{IDX}.zip"), &members);
    let mut raw = vec![KIND_RAW];
    raw.extend_from_slice(&zstd(&data).unwrap());
    let source = File::open(&zip).unwrap();
    let len = source.metadata().unwrap().len();
    let entries = zip_entries(&source, 0, len).unwrap();
    let items = vec![
        Item {
            kind: KIND_RAW,
            block: raw.clone(),
            rows: 0,
        },
        Item {
            kind: KIND_RAW,
            block: raw.clone(),
            rows: 0,
        },
    ];
    // The index as written, then entry B pointed at entry A's block.
    let (mut index, _) = index_bytes(&entries, &items);
    let second_off = 2 + entries[0].raw_name.len() + 41 + 2 + entries[1].raw_name.len() + 1;
    index[second_off..second_off + 8].copy_from_slice(&8_u64.to_le_bytes());
    let mut file = MAGIC.to_vec();
    file.extend_from_slice(&raw);
    let index_off = file.len() as u64;
    let frame = zstd(&index).unwrap();
    file.extend_from_slice(&frame);
    file.extend_from_slice(&footer(index_off, frame.len() as u64, &index, 2));
    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &file).unwrap();
    assert!(
        read_index(&file, file.len() as u64).is_ok(),
        "the reader alone accepts it"
    );
    let run = write_day(&zip, &root, &options("share"));
    let Err(Refusal::HeldDayDiffers { why, .. }) = &run.result else {
        panic!("{run:?}");
    };
    assert!(why.contains("block_overlap"), "{why}");
}

/// **A WORKER THAT PANICS IS A REFUSAL, NOT A DEAD PROCESS.**
#[test]
fn a_panicking_worker_is_a_named_refusal() {
    let result = parallel(5, 2, &|at| {
        assert!(at != 3, "an invented fault");
        Ok(at)
    });
    assert!(
        matches!(result, Err(Refusal::EncodeFailed { .. })),
        "{result:?}"
    );
    assert_eq!(parallel(4, 3, &|at| Ok(at * 2)), Ok(vec![0, 2, 4, 6]));
    assert_eq!(parallel(0, 3, &|at| Ok(at)), Ok(Vec::new()));
    let first = parallel(6, 1, &|at| {
        if at >= 2 {
            Err(Refusal::DayTooLarge { bytes: at as u64 })
        } else {
            Ok(at)
        }
    });
    assert_eq!(first, Err(Refusal::DayTooLarge { bytes: 2 }));
}

// ── the encoder against §4 ─────────────────────────────────────────────────

/// `bytes` encoded and rebuilt through the reader: its kind.
fn round_trip(bytes: &[u8]) -> u8 {
    let encoded = encode(bytes).unwrap();
    let entry = IndexEntry {
        name: "x".to_owned(),
        kind: encoded.kind,
        off: 0,
        len: encoded.block.len() as u64,
        size: bytes.len() as u64,
        crc: crc32(bytes),
        rows: encoded.rows,
        dos_time: 0,
    };
    assert_eq!(rebuild(&encoded.block, &entry).unwrap(), bytes, "{bytes:?}");
    encoded.kind
}

/// The kind §4 assigns each line shape, and every one rebuilt exactly.
#[test]
fn every_line_shape_rebuilds_exactly_in_the_kind_section_4_assigns() {
    let columnar: [&[u8]; 8] = [
        b"a,b\r\n1,2\r\n3,4\r\n",
        b"a,b\r\n1,2\r\n3,4",
        b"a,b\n1,2\n,\n",
        b"a,b\r\n",
        b"a,b",
        b"\n\n",
        b"t,p,q\r\n09:15:00,2.2,-5\r\n09:15:01,0.05,007\r\n23:59:59,-0.0,0\r\n",
        b"p\n1.\n.5\n-\n1e5\n00\n-0\n99999999999999999\n",
    ];
    for bytes in columnar {
        assert_eq!(round_trip(bytes), KIND_COLUMNAR, "{bytes:?}");
    }
    let raw: [&[u8]; 7] = [
        b"a,b\r\n1,2\n",
        b"a,b\r\n1,2,3\r\n",
        b"a,b\r\n1\r\n",
        b"a,b\n1,2\r\n",
        b"a,b\r\n1,2\r\n\r\n",
        b"",
        b"a,b\r\n1,2\r\r\n",
    ];
    for bytes in raw {
        assert_eq!(round_trip(bytes), KIND_RAW, "{bytes:?}");
    }
    assert_eq!(round_trip(b"a,b\r"), KIND_RAW, "a lone CR on the only line");
}

/// Numbers that re-render exactly are numeric; any other is text.
#[test]
fn a_field_is_numeric_only_when_it_re_renders_byte_for_byte() {
    for field in [
        &b"2.2"[..],
        b"0.05",
        b"-0.0",
        b"100",
        b"09:15:00",
        b"0",
        b"12.340",
        b"-17",
        b"99:59:59",
        b"00:00:00",
        b"12345678901234567",
        b"1.000000000000001",
    ] {
        let (shape, value) = parse_field(field).unwrap();
        let mut out = Vec::new();
        render(&mut out, shape, value);
        assert_eq!(out, field, "{:?}", String::from_utf8_lossy(field));
    }
    for field in [
        &b""[..],
        b"-",
        b"1.",
        b"1.2.3",
        b"1e5",
        b"123456789012345678",
        b"0.1234567890123456",
        b"9a:00:00",
        b"x",
    ] {
        assert_eq!(
            parse_field(field),
            None,
            "{:?}",
            String::from_utf8_lossy(field)
        );
    }
    // Fields that parse but render otherwise stay text.
    assert!(parse_column(b"007\n1", 2).is_none());
    assert!(parse_column(b"99:99:99", 1).is_none());
    assert!(parse_column(b"12:75:00", 1).is_none());
    assert!(parse_column(b"1\n2", 3).is_none(), "a row short");
    assert!(parse_column(b"", 0).is_none());
    let parsed = parse_column(b"5\n-3\n09:00:00", 3).unwrap();
    assert_eq!(parsed.values, vec![5, -3, 32_400]);
    assert_eq!(parsed.shapes, vec![0, SHAPE_NEG, SHAPE_TIME]);
}

/// The numeric payload's planes: width 0 for all zeros, delta differences.
#[test]
fn a_numeric_payload_is_shapes_flag_width_and_planes() {
    let zeros = Parsed {
        shapes: vec![0, 0],
        values: vec![0, 0],
    };
    assert_eq!(numeric_payload(&zeros, false), vec![0, 0, 0, 0]);
    let rising = Parsed {
        shapes: vec![0, 0, 0],
        values: vec![300, 301, 303],
    };
    // Plain: zigzag 600, 602, 606 in two planes.
    assert_eq!(
        numeric_payload(&rising, false),
        vec![0, 0, 0, 0, 2, 0x58, 0x5A, 0x5E, 2, 2, 2]
    );
    // Delta: 300, 1, 2 -> zigzag 600, 2, 4.
    assert_eq!(
        numeric_payload(&rising, true),
        vec![0, 0, 0, 1, 2, 0x58, 2, 4, 2, 0, 0]
    );
}

/// A wide regular CSV takes the smallest representation per column and the
/// numeric column decodes through the reader.
#[test]
fn a_long_numeric_column_is_stored_numeric() {
    let text = csv("NIFTY", 5_000, "\r\n", true);
    let encoded = encode(&text).unwrap();
    assert_eq!(encoded.kind, KIND_COLUMNAR);
    assert_eq!(encoded.rows, 5_000);
    assert!(
        encoded.block.len() * 10 < text.len(),
        "{} of {}",
        encoded.block.len(),
        text.len()
    );
    assert_eq!(round_trip(&text), KIND_COLUMNAR);
}

/// Every refusal's code and words, one line each.
#[test]
fn every_refusal_names_its_code_in_its_words() {
    let kind = std::io::ErrorKind::NotFound;
    let all = vec![
        Refusal::NameNotCanonical { name: "n".into() },
        Refusal::RunIdInvalid { run_id: "r".into() },
        Refusal::ZipUnavailable {
            path: "z".into(),
            kind,
        },
        Refusal::ZipRefused {
            why: CmRefusal::FolderUnknown,
        },
        Refusal::NameNotUtf8 {
            raw_hex: "ff".into(),
        },
        Refusal::NameTooLong { len: 70_000 },
        Refusal::DuplicateEntry { name: "d".into() },
        Refusal::InnerFolderMismatch {
            stem: "s".into(),
            found: "f".into(),
        },
        Refusal::TooManyEntries { entries: 1 },
        Refusal::MemberTooLarge {
            name: "m".into(),
            size: 1,
        },
        Refusal::MemberCorrupt {
            name: "m".into(),
            why: "w".into(),
        },
        Refusal::EncodeFailed {
            name: "m".into(),
            why: "w".into(),
        },
        Refusal::DayTooLarge { bytes: 1 },
        Refusal::DayLocked { lock: "l".into() },
        Refusal::LockFailed {
            lock: "l".into(),
            kind,
        },
        Refusal::StaleTemp {
            leftovers: vec![("t".into(), 1), ("u".into(), 2)],
        },
        Refusal::HeldDayDiffers {
            path: "p".into(),
            why: "w".into(),
        },
        Refusal::DiskFull {
            temp: "t".into(),
            written: 1,
        },
        Refusal::WriteFailed {
            path: "p".into(),
            step: "write",
            kind,
        },
        Refusal::VerifyFailed {
            temp: "t".into(),
            why: "w".into(),
        },
        Refusal::CommitInvalid { commit: "c".into() },
        Refusal::RootMissing {
            root: "r".into(),
            what: "w".into(),
        },
        Refusal::NotRegularFile {
            path: "p".into(),
            found: Found::Pipe,
        },
        Refusal::NoFileInDay { stem: "s".into() },
        Refusal::ZipCorruptOnVerify {
            path: "p".into(),
            why: "w".into(),
        },
        Refusal::TempNotCreated {
            temp: "t".into(),
            word: "staging_linked",
            why: "w".into(),
        },
        Refusal::TempRaced {
            temp: "t".into(),
            cleared: 1,
        },
        Refusal::LinkedFolder { path: "l".into() },
    ];
    let mut codes = std::collections::BTreeSet::new();
    for why in &all {
        let text = why.to_string();
        assert!(text.starts_with(why.code()), "{text}");
        assert!(!text.contains('\n'), "{text}");
        codes.insert(why.code());
    }
    assert_eq!(codes.len(), all.len(), "every code is its own");
    assert!(all[15].to_string().ends_with("t (1 bytes) u (2 bytes)"));
}

/// The many-entries bound: an index's count is a `u32`, and a name's length
/// a `u16`; both refused past them before anything is read.
#[test]
fn the_index_bounds_are_refused_before_anything_is_read() {
    let identity = DayIdentity::of_zip_name(&format!("{IDX}.zip")).unwrap();
    let (zip, _) = staged("tsw-bounds", &format!("{IDX}.zip"), &nifty_spot_members());
    let source = File::open(&zip).unwrap();
    let len = source.metadata().unwrap().len();
    let mut entries = zip_entries(&source, 0, len).unwrap();
    entries[1].raw_name = vec![b'a'; 70_000];
    entries[1].name = "a".repeat(70_000);
    assert_eq!(
        preflight(&identity, &entries),
        Err(Refusal::NameTooLong { len: 70_000 })
    );
}

// ── measurements on real data, run by hand (R4-08 item 3, E2E-05) ──────────

/// `ns` sorted: p50, p99 and max.
fn quantiles(mut ns: Vec<u128>) -> (u128, u128, u128) {
    ns.sort_unstable();
    let at = |q: usize| {
        ns.get(ns.len().saturating_sub(1) * q / 100)
            .copied()
            .unwrap_or(0)
    };
    (at(50), at(99), ns.last().copied().unwrap_or(0))
}

/// A path the measurement may write: never under the operator's store.
fn refuse_the_real_store(path: &Path) {
    let real = Path::new("/Volumes/WD_BLACK/brutex/fresh-20260919/runtime");
    let tick = Path::new("/Volumes/WD_BLACK/brutex/tickstore-data");
    assert!(
        !path.starts_with(real) && !path.starts_with(tick),
        "{} is the operator's store; the measurement writes only a scratch root",
        path.display()
    );
}

/// **REAL DAYS, READ-ONLY COPIES**: `BRUTEX_TSW_REAL` is `ZIP=TOOL_BTS;...`
/// (copies of real day zips and the separate tool's day files for the same
/// days), `BRUTEX_TSW_OUT` a scratch root. For each day: this writer's day
/// file is written and verified; the tool's file is verified against the same
/// zip by the same writer-blind rules; the two sizes, the write's MB/s, each
/// entry's rebuild p50/p99/max and the readers' lookup p50/p99/max on the
/// written file are printed. Writes only the scratch root.
#[test]
#[ignore = "a measurement on copies of real day zips, run by hand with its inputs named"]
#[allow(
    clippy::too_many_lines,
    reason = "one measurement, printed in the order it is taken"
)]
fn measure_real_days_against_the_tools_files() {
    let spec = std::env::var("BRUTEX_TSW_REAL").expect("BRUTEX_TSW_REAL names the days");
    let out = PathBuf::from(std::env::var("BRUTEX_TSW_OUT").expect("BRUTEX_TSW_OUT"));
    refuse_the_real_store(&out);
    std::fs::create_dir_all(&out).unwrap();
    for pair in spec.split(';').filter(|pair| !pair.is_empty()) {
        let (zip, tool) = pair.split_once('=').expect("ZIP=TOOL_BTS");
        let (zip, tool) = (Path::new(zip), Path::new(tool));
        let name = zip.file_name().unwrap().to_string_lossy().into_owned();
        let identity = DayIdentity::of_zip_name(&name).unwrap();
        let start = Instant::now();
        let run = write_day(zip, &out, &WriteOptions::new("measure"));
        let write_ns = start.elapsed().as_nanos();
        let mine = match run.result {
            Ok(Outcome::Written(report) | Outcome::Held(report)) => report,
            Err(why) => panic!("{name}: {why}"),
        };
        let source = File::open(zip).unwrap();
        let len = source.metadata().unwrap().len();
        let entries = zip_entries(&source, 0, len).unwrap();
        let start = Instant::now();
        let theirs = verify_file(tool, &source, &entries, 8).expect("the tool's file matches");
        let verify_ns = start.elapsed().as_nanos();
        let largest = entries.iter().map(|e| e.len).max().unwrap_or(0);
        println!(
            "DAY {name}: entries {} columnar {} raw {} rows {} csv {} zip {} largest member {largest}",
            mine.entries,
            mine.columnar,
            mine.raw_fallbacks,
            mine.rows,
            mine.csv_bytes,
            mine.zip_bytes
        );
        println!(
            "  bytes: this writer {} | the tool {} | ratio {} per mille | tool's raw entries {}",
            mine.stored_bytes,
            theirs.stored_bytes,
            u128::from(mine.stored_bytes) * 1_000 / u128::from(theirs.stored_bytes.max(1)),
            theirs.raw_fallbacks
        );
        println!(
            "  write+verify {} ms = {} MB/s of CSV; verify of the tool's file {} ms = {} MB/s",
            write_ns / 1_000_000,
            u128::from(mine.csv_bytes) * 1_000 / write_ns.max(1),
            verify_ns / 1_000_000,
            u128::from(mine.csv_bytes) * 1_000 / verify_ns.max(1)
        );
        for (label, path) in [
            ("this writer", &mine.path),
            ("the tool", &tool.to_path_buf()),
        ] {
            let file = File::open(path).unwrap();
            let size = file.metadata().unwrap().len();
            let index = read_index(&file, size).unwrap();
            let mut ns = Vec::new();
            let mut bytes = 0_u64;
            let start = Instant::now();
            for entry in index.iter().filter(|e| e.kind != KIND_DIR) {
                let t = Instant::now();
                let back = rebuild(&file, entry).unwrap();
                ns.push(t.elapsed().as_nanos());
                bytes += back.len() as u64;
            }
            let all = start.elapsed().as_nanos();
            let (p50, p99, max) = quantiles(ns.clone());
            println!(
                "  rebuild one entry ({label}): p50 {p50} ns p99 {p99} ns max {max} ns n={} ; {} MB/s single-thread",
                ns.len(),
                u128::from(bytes) * 1_000 / all.max(1)
            );
        }
        // The readers' own point lookup on the written file.
        let mut ns = Vec::new();
        match identity.tree {
            Tree::Options => {
                let listed = NfoTickStore::new(&out).day(identity.day).unwrap().unwrap();
                let tickers: Vec<String> = listed
                    .entries()
                    .iter()
                    .filter_map(|f| {
                        let tail = f.entry.rsplit(['/', '\\']).next()?;
                        Some(tail.strip_suffix(".NFO.csv")?.to_owned())
                    })
                    .collect();
                for _ in 0..10 {
                    for ticker in &tickers {
                        let t = Instant::now();
                        std::hint::black_box(listed.locate(ticker).is_ok());
                        ns.push(t.elapsed().as_nanos());
                    }
                }
            }
            Tree::Cm(kind) => {
                let listing = TickStore::new(&out)
                    .day(kind, identity.day)
                    .unwrap()
                    .unwrap();
                let stems: Vec<String> = listing
                    .entries()
                    .iter()
                    .filter_map(|f| {
                        let tail = f.entry.rsplit(['/', '\\']).next()?;
                        Some(tail.strip_suffix(".csv")?.to_owned())
                    })
                    .collect();
                for _ in 0..10 {
                    for stem in &stems {
                        let t = Instant::now();
                        std::hint::black_box(listing.locate(stem).is_ok());
                        ns.push(t.elapsed().as_nanos());
                    }
                }
            }
        }
        let n = ns.len();
        let (p50, p99, max) = quantiles(ns);
        println!(
            "  lookup on the written file (the readers' listing): p50 {p50} ns p99 {p99} ns max {max} ns n={n}"
        );
        assert!(n > 0, "{name}: nothing was looked up");
    }
}

/// **E2E-05: `ruzstd` `Fastest` on the seven sealed-codec sample months.**
/// `BRUTEX_TSW_BAR_MONTHS` is `LABEL=PATH.bin;...` (version-1 bar files: a
/// 32,768-byte header, then 56-byte rows). For each: the rows as one frame,
/// and as 256-bar frames (the only way a page stays O(1)), bytes a bar in
/// thousandths, and one 256-bar frame's decode p50/p99/max over 2,000 random
/// pages. Writes nothing.
#[test]
#[ignore = "a measurement on copies of real bar months, run by hand with its inputs named"]
fn measure_ruzstd_fastest_on_bar_months() {
    const HEADER: usize = 32_768;
    const ROW: usize = 56;
    const PAGE: usize = 256;
    let spec = std::env::var("BRUTEX_TSW_BAR_MONTHS").expect("BRUTEX_TSW_BAR_MONTHS");
    for pair in spec.split(';').filter(|pair| !pair.is_empty()) {
        let (label, path) = pair.split_once('=').expect("LABEL=PATH");
        let bytes = std::fs::read(path).unwrap();
        let rows = &bytes[HEADER..];
        assert_eq!(rows.len() % ROW, 0, "{label}: whole rows");
        let bars = rows.len() / ROW;
        let whole = zstd(rows).unwrap();
        let frames: Vec<Vec<u8>> = rows
            .chunks(ROW * PAGE)
            .map(|chunk| zstd(chunk).unwrap())
            .collect();
        let paged: usize = frames.iter().map(Vec::len).sum::<usize>() + frames.len() * 8;
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut ns = Vec::new();
        for _ in 0..2_000 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let at = usize::try_from(state >> 33).unwrap() % frames.len();
            let want = rows.chunks(ROW * PAGE).nth(at).unwrap();
            let t = Instant::now();
            let mut back = Vec::with_capacity(want.len());
            let mut decoder =
                ruzstd::decoding::StreamingDecoder::new(frames[at].as_slice()).unwrap();
            std::io::Read::read_to_end(&mut decoder, &mut back).unwrap();
            ns.push(t.elapsed().as_nanos());
            assert_eq!(back, want, "{label}: page {at} round trip");
        }
        let (p50, p99, max) = quantiles(ns);
        println!(
            "MONTH {label}: {bars} bars | v1 {} B ({} mB/bar) | one frame {} B ({} mB/bar) | 256-bar frames {} B ({} mB/bar) | page decode p50 {p50} ns p99 {p99} ns max {max} ns n=2000",
            rows.len(),
            rows.len() * 1_000 / bars.max(1),
            whole.len(),
            whole.len() * 1_000 / bars.max(1),
            paged,
            paged * 1_000 / bars.max(1)
        );
    }
}

// ── the second check's blocking issues and follow-ups (verify-R4-08-r1) ────

/// A stamp shaped like the one the build writes.
const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";

/// The `commit` field of an event.
fn commit_of<'a>(event: &telemetry::Event<'a>) -> Option<telemetry::Value<'a>> {
    event
        .fields()
        .iter()
        .find(|(key, _)| *key == "commit")
        .map(|(_, value)| *value)
}

/// **THE LOG LINE AND THE EVENT NAME THE COMMIT OF THE CODE THAT WROTE THE
/// DAY** (D-4967 item 7): the stamp when the caller names one, `unstamped`
/// when it does not, `invalid` (refused `commit_invalid`) when it could not
/// be one field. The zip's and the day's paths are quoted, so a vendor
/// folder with a space stays one field.
#[test]
fn the_log_line_and_the_event_name_the_commit() {
    let (zip, root) = staged("tsw-commit", &format!("{IDX}.zip"), &nifty_spot_members());
    let spaced = zip
        .parent()
        .unwrap()
        .join("vendor download")
        .join(format!("{IDX}.zip"));
    std::fs::create_dir_all(spaced.parent().unwrap()).unwrap();
    std::fs::rename(&zip, &spaced).unwrap();
    let stamped = options("stamped").with_commit(STAMP);
    let run = write_day(&spaced, &root, &stamped);
    assert!(matches!(run.result, Ok(Outcome::Written(_))), "{run:?}");
    let plain = write_day(&spaced, &root, &options("plain"));
    assert!(matches!(plain.result, Ok(Outcome::Held(_))), "{plain:?}");
    let bad_options = options("bad").with_commit("not a stamp");
    let bad = write_day(&spaced, &root, &bad_options);
    assert_eq!(
        bad.result,
        Err(Refusal::CommitInvalid {
            commit: "not a stamp".into()
        })
    );
    let log = log_of(&root);
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(lines.len(), 3, "{log}");
    assert!(
        lines[0].contains(&format!(" commit={STAMP} run=stamped ")),
        "{log}"
    );
    assert!(lines[1].contains(" commit=unstamped run=plain "), "{log}");
    assert!(lines[2].contains(" commit=invalid run=bad "), "{log}");
    assert!(lines[2].contains("`not a stamp`"), "{log}");
    let zip_field = format!(" zip=\"{}\" ", spaced.display());
    assert!(lines.iter().all(|line| line.contains(&zip_field)), "{log}");
    let day_path = run
        .result
        .as_ref()
        .unwrap()
        .report()
        .path
        .display()
        .to_string();
    assert!(lines[0].contains(&format!(" day=\"{day_path}\" ")), "{log}");
    let zip_name = spaced.display().to_string();
    let event = event_of(&run.result, &zip_name, &stamped, 1, &day_path);
    assert_eq!(commit_of(&event), Some(telemetry::Value::Str(STAMP)));
    let plain_options = options("plain");
    let event = event_of(&plain.result, &zip_name, &plain_options, 1, &day_path);
    assert_eq!(commit_of(&event), Some(telemetry::Value::Str(UNSTAMPED)));
    let event = event_of(&bad.result, &zip_name, &bad_options, 1, "why");
    assert_eq!(
        commit_of(&event),
        Some(telemetry::Value::Str("not a stamp"))
    );
}

/// **THE TEMPORARY IS BORN THROUGH THE STAGING DOOR (R3-16, gate 28), AND
/// THE DOOR'S CLEAR PATH IS NAMED WHEN A RACE REACHES IT.** A one-name file
/// put at the run's own temporary name after the leftover check is cleared
/// by the door; the run stops `temp_raced` with the bytes, leaves its empty
/// temporary, and the next run refuses that by name. A second name of
/// another file there is refused `temp_not_created` (`staging_linked`), and
/// the other file is neither cut nor written through.
#[test]
fn the_temporary_is_born_through_the_staging_door_and_a_race_is_named() {
    let (zip, root) = staged("tsw-race", &format!("{IDX}.zip"), &nifty_spot_members());
    let faults = Faults {
        plant_before_create: Some(Plant::File(7)),
        ..Faults::default()
    };
    let run = write_day(&zip, &root, &options("raced").with_faults(faults));
    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    let temp = path.with_file_name(format!("{IDX}.bts.tmp.raced"));
    assert_eq!(
        run.result,
        Err(Refusal::TempRaced {
            temp: temp.clone(),
            cleared: 7
        })
    );
    assert!(!path.exists(), "the day is not held");
    assert_eq!(std::fs::metadata(&temp).unwrap().len(), 0, "left empty");
    assert!(
        log_of(&root).contains("code=temp_raced"),
        "{}",
        log_of(&root)
    );
    let next = write_day(&zip, &root, &options("next"));
    assert_eq!(next.result.unwrap_err().code(), "stale_temp");
    assert!(temp.exists(), "left for the operator");

    let (zip, root) = staged("tsw-linked", &format!("{IDX}.zip"), &nifty_spot_members());
    let faults = Faults {
        plant_before_create: Some(Plant::HardLink),
        ..Faults::default()
    };
    let run = write_day(&zip, &root, &options("linked").with_faults(faults));
    let Err(Refusal::TempNotCreated { word, .. }) = &run.result else {
        panic!("{run:?}");
    };
    assert_eq!(*word, "staging_linked");
    assert_eq!(
        std::fs::read(root.join("planted")).unwrap(),
        b"another file",
        "neither cut nor written through"
    );
    assert!(log_of(&root).contains("code=temp_not_created"));
}

/// **A NAME THE WRITER OPENS THAT IS NOT A REGULAR FILE IS REFUSED, NEVER
/// FOLLOWED AND NEVER WAITED ON.** A pipe at the zip's name (it blocked
/// `File::open` for ever), a dangling link at the held day's name (the old
/// look took it for "no day" and the rename replaced it), a directory at the
/// lock's name, and one at the log's name (the day is written; the run says
/// its line was not).
#[test]
fn a_name_that_is_not_a_regular_file_is_refused_and_never_waited_on() {
    let (zip, root) = staged("tsw-pipe", &format!("{IDX}.zip"), &nifty_spot_members());
    let pipe = zip.with_file_name("GFDLCM_INDICES_TICK_26092026.zip");
    let made = std::process::Command::new("mkfifo")
        .arg(&pipe)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo made {}", pipe.display());
    let (tx, rx) = std::sync::mpsc::channel();
    let (p, r) = (pipe.clone(), root.clone());
    std::thread::spawn(move || {
        let _gone_when_the_test_ended = tx.send(write_day(&p, &r, &options("pipe")).result);
    });
    let result = rx
        .recv_timeout(std::time::Duration::from_mins(1))
        .expect("refused at once, never waited on");
    assert_eq!(
        result,
        Err(Refusal::NotRegularFile {
            path: pipe,
            found: Found::Pipe
        })
    );

    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(root.join("nowhere.bts"), &path).unwrap();
    let run = write_day(&zip, &root, &options("dangling"));
    assert_eq!(
        run.result,
        Err(Refusal::NotRegularFile {
            path: path.clone(),
            found: Found::Symlink
        })
    );
    assert!(
        std::fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink(),
        "left as it is"
    );
    assert_eq!(names_in(path.parent().unwrap()), vec![format!("{IDX}.bts")]);

    let (zip, root) = staged("tsw-lockdir", &format!("{IDX}.zip"), &nifty_spot_members());
    std::fs::create_dir_all(root.join(LOCK_NAME)).unwrap();
    let run = write_day(&zip, &root, &options("lockdir"));
    assert_eq!(
        run.result,
        Err(Refusal::NotRegularFile {
            path: root.join(LOCK_NAME),
            found: Found::Directory
        })
    );

    let (zip, root) = staged("tsw-logdir", &format!("{IDX}.zip"), &nifty_spot_members());
    std::fs::create_dir_all(root.join(LOG_NAME)).unwrap();
    let run = write_day(&zip, &root, &options("logdir"));
    assert!(matches!(run.result, Ok(Outcome::Written(_))), "{run:?}");
    let why = run.log_error.unwrap();
    assert!(why.contains("not a regular file"), "{why}");
}

/// **A ZIP OF ONLY ITS DAY FOLDER IS NOT A DAY**: refused `no_file_in_day`
/// before the lock is taken; a folder with one empty file is a day of 0
/// rows.
#[test]
fn a_zip_of_only_its_day_folder_is_refused() {
    let only = vec![
        member(&format!("{IDX}/"), b""),
        member(&format!("{IDX}/SUB/"), b""),
    ];
    let (zip, root) = staged("tsw-folder", &format!("{IDX}.zip"), &only);
    let run = write_day(&zip, &root, &options("folder"));
    assert_eq!(
        run.result,
        Err(Refusal::NoFileInDay {
            stem: IDX.to_owned()
        })
    );
    assert_eq!(
        names_in(&root),
        vec![LOG_NAME.to_owned()],
        "nothing written"
    );
    let one_empty = vec![
        member(&format!("{IDX}/"), b""),
        member(&format!("{IDX}/EMPTY.NSE_IDX.csv"), b""),
    ];
    let (zip, root) = staged("tsw-empty", &format!("{IDX}.zip"), &one_empty);
    let run = write_day(&zip, &root, &options("empty"));
    let Ok(Outcome::Written(report)) = &run.result else {
        panic!("{run:?}");
    };
    assert_eq!((report.entries, report.rows), (2, 0));
}

/// **WHEN THE ZIP IS THE DAMAGED SIDE, THE HELD DAY IS NOT BLAMED**: one
/// byte of a stored member flipped after its day was held is refused
/// `zip_corrupt_on_verify`, naming the zip reader's refusal; the day file is
/// left as it was. A held day whose zip states another DOS time for one
/// entry, and nothing else, is refused `held_day_differs`.
#[test]
fn a_corrupt_zip_on_a_held_day_is_named_and_the_day_is_not_blamed() {
    let mut members = nifty_spot_members();
    let (zip, root) = staged("tsw-zipcut", &format!("{IDX}.zip"), &members);
    let first = write_day(&zip, &root, &options("before"));
    let Ok(Outcome::Written(written)) = first.result else {
        panic!("{first:?}");
    };
    let before = std::fs::read(&written.path).unwrap();
    let mut bytes = std::fs::read(&zip).unwrap();
    let data = &members[1].data;
    let at = bytes
        .windows(data.len())
        .position(|window| window == data.as_slice())
        .unwrap()
        + data.len() / 2;
    bytes[at] ^= 0x01;
    std::fs::write(&zip, &bytes).unwrap();
    let again = write_day(&zip, &root, &options("after"));
    let Err(Refusal::ZipCorruptOnVerify { path, why }) = &again.result else {
        panic!("{again:?}");
    };
    assert_eq!(path, &written.path);
    assert!(
        why.contains("member_corrupt") && why.contains("CRC-32"),
        "{why}"
    );
    assert_eq!(std::fs::read(&written.path).unwrap(), before, "untouched");
    assert!(log_of(&root).contains("code=zip_corrupt_on_verify"));

    members[1].time ^= 0x0001;
    std::fs::write(&zip, zip_of(&members)).unwrap();
    let dated = write_day(&zip, &root, &options("dated"));
    let Err(Refusal::HeldDayDiffers { why, .. }) = &dated.result else {
        panic!("{dated:?}");
    };
    assert!(
        why.contains("NIFTY.NSE_IDX.csv") && why.contains("DOS time"),
        "{why}"
    );
    assert_eq!(std::fs::read(&written.path).unwrap(), before, "untouched");
}

/// **THE WRITER NEVER MAKES A ROOT**: a root that is not there, or is a
/// regular file, is refused `root_missing`; nothing is made, and the run
/// says its log line could not be written.
#[test]
fn a_missing_root_is_refused_and_never_made() {
    let (zip, root) = staged("tsw-noroot", &format!("{IDX}.zip"), &nifty_spot_members());
    let typo = root.join("typo");
    let run = write_day(&zip, &typo, &options("typo"));
    assert_eq!(run.result.as_ref().unwrap_err().code(), "root_missing");
    assert!(!typo.exists(), "never made");
    assert!(run.log_error.is_some(), "{run:?}");
    let file = root.join("a-file");
    std::fs::write(&file, b"x").unwrap();
    let run = write_day(&zip, &file, &options("file"));
    assert_eq!(
        run.result,
        Err(Refusal::RootMissing {
            root: file.clone(),
            what: "not a directory".to_owned()
        })
    );
    assert_eq!(std::fs::read(&file).unwrap(), b"x");
}

/// **EVERY FOLDER MADE FOR A DAY HAS ITS PARENT FSYNCED**, outermost first,
/// so a power loss cannot take back the folder of a day a log line called
/// written; a folder already there syncs nothing.
#[test]
fn every_new_folder_has_its_parent_fsynced() {
    let base = scratch("tsw-dirs");
    let dir = base.join("cm").join("INDICES").join("2026");
    assert_eq!(
        make_dirs(&dir).unwrap(),
        vec![
            base.clone(),
            base.join("cm"),
            base.join("cm").join("INDICES")
        ]
    );
    assert!(dir.is_dir());
    assert_eq!(make_dirs(&dir).unwrap(), Vec::<PathBuf>::new());
    let month = dir.join("SEP_2026");
    assert_eq!(make_dirs(&month).unwrap(), vec![dir.clone()]);
}

/// **NO FOLDER BELOW THE ROOT IS WRITTEN THROUGH A LINK** (R3-16's rule
/// for a store, RA-298): a shadow root whose `cm` is a link to another
/// store, or whose month folder is one, is refused `linked_folder` and
/// nothing is made through the link; the root itself may be a link.
#[test]
fn a_tick_store_folder_that_is_a_link_is_never_written_through() {
    let (zip, root) = staged("tsw-linkcm", &format!("{IDX}.zip"), &nifty_spot_members());
    let real = root.with_file_name("real");
    std::fs::create_dir_all(real.join("cm")).unwrap();
    std::os::unix::fs::symlink(real.join("cm"), root.join("cm")).unwrap();
    let run = write_day(&zip, &root, &options("linked-cm"));
    assert_eq!(
        run.result,
        Err(Refusal::LinkedFolder {
            path: root.join("cm")
        })
    );
    assert_eq!(
        names_in(&real.join("cm")),
        Vec::<String>::new(),
        "nothing made"
    );

    let (zip, root) = staged(
        "tsw-linkmonth",
        &format!("{IDX}.zip"),
        &nifty_spot_members(),
    );
    let path = TickStore::new(&root).day_path(CmKind::Indices, day(2026, 9, 25));
    let month = path.parent().unwrap();
    std::fs::create_dir_all(month.parent().unwrap()).unwrap();
    let elsewhere = root.with_file_name("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, month).unwrap();
    let run = write_day(&zip, &root, &options("linked-month"));
    assert_eq!(run.result.unwrap_err().code(), "linked_folder");
    assert_eq!(names_in(&elsewhere), Vec::<String>::new(), "nothing made");

    let (zip, root) = staged("tsw-linkroot", &format!("{IDX}.zip"), &nifty_spot_members());
    let via = root.with_file_name("via-link");
    std::os::unix::fs::symlink(&root, &via).unwrap();
    let run = write_day(&zip, &via, &options("linked-root"));
    assert!(matches!(run.result, Ok(Outcome::Written(_))), "{run:?}");
}
