//! The loose day zips (D-4815, D-4834; design §8 A4, A13, A16 to A20, B27),
//! built from invented rows only: every price here is a round number no
//! vendor file carries, and no real vendor row is copied.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;
use crate::gdfl_archive::{Archive, crc32};
use crate::gdfl_cm::{HEADER_OPEN_INTEREST, ListingSum, read_day};
use crate::gdfl_nfo::NfoZips;
use brutex_core::instrument::{Exchange, InstrumentKey};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A STORED zip of `entries`, plain (no ZIP64), as a loose day zip or an
/// outer archive holding stored day zips is.
pub(super) fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in entries {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let len = data.len() as u32;
        let name_len = (name.len() as u16).to_le_bytes();
        out.extend(0x0403_4b50_u32.to_le_bytes());
        out.extend([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        out.extend(crc.to_le_bytes());
        out.extend(len.to_le_bytes());
        out.extend(len.to_le_bytes());
        out.extend(name_len);
        out.extend([0, 0]);
        out.extend(name.as_bytes());
        out.extend(*data);
        central.extend(0x0201_4b50_u32.to_le_bytes());
        central.extend([20, 3, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend(crc.to_le_bytes());
        central.extend(len.to_le_bytes());
        central.extend(len.to_le_bytes());
        central.extend(name_len);
        central.extend([0; 12]);
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let (cd_at, cd_len) = (out.len() as u32, central.len() as u32);
    let count = (entries.len() as u16).to_le_bytes();
    out.extend(central);
    out.extend(0x0605_4b50_u32.to_le_bytes());
    out.extend([0; 4]);
    out.extend(count);
    out.extend(count);
    out.extend(cd_len.to_le_bytes());
    out.extend(cd_at.to_le_bytes());
    out.extend([0, 0]);
    out
}

/// 2026-09-25, the first day the loose zips hold that the archives do not.
fn at() -> Day {
    Day::new(2026, 9, 25).unwrap()
}

/// An invented NIFTY 50 day: a session row and two after the close, so the
/// file's largest stamp is past 15:30:00 as every regular index day's is.
fn nifty_csv(day: Day) -> Vec<u8> {
    let date = format!("{:02}/{:02}/{:04}", day.day(), day.month(), day.year());
    let row = |time: &str, ltp: &str| format!("NIFTY 50.NSE_IDX,{date},{time},{ltp},0,0,0,0,0,0");
    [
        HEADER_OPEN_INTEREST.to_owned(),
        row("09:15:00", "10000"),
        row("16:00:00", "10001.5"),
        row("16:00:01", "10002.5"),
    ]
    .join("\r\n")
    .into_bytes()
}

/// The index day zip of `day`: its folder entry, NIFTY's file and a file
/// off the swept surface, as the vendor's day zips lay them out.
fn index_day_zip(day: Day) -> Vec<u8> {
    let folder = crate::gdfl_cm::day_folder_name(CmKind::Indices, day);
    let nifty = format!("{folder}/NIFTY 50.NSE_IDX.csv");
    let other = format!("{folder}/NIFTY IT.NSE_IDX.csv");
    let csv = nifty_csv(day);
    zip(&[
        (&format!("{folder}/"), b""),
        (&nifty, &csv),
        (&other, b"Ticker\r\n"),
    ])
}

fn nifty() -> InstrumentKey {
    InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()
}

/// A fresh, empty directory of its own for one test: never reused, never
/// removed (the operator's rule: nothing is deleted).
fn fresh(tag: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "gdfl_dayzips_{}_{}_{tag}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `bytes` at `rel` under `root`, its folders made.
fn put(root: &Path, rel: &str, bytes: &[u8]) {
    let path = root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// A1 / A4: the same day zip read from the loose root and from an outer
/// archive gives the same listing, byte for byte the same digest, and the
/// same decoded file.
#[test]
fn loose_day_zips_import_like_the_archive_day_they_copy() {
    let day_zip = index_day_zip(at());
    let root = fresh("copy");
    put(&root, &LooseKind::Indices.path_of(at()), &day_zip);
    let loose = DayZips::open(&root).unwrap();
    assert_eq!(loose.census().held, 1);
    let outer = zip(&[(
        &crate::gdfl_archive::day_zip_name(CmKind::Indices, at()),
        &day_zip,
    )]);
    let len = outer.len() as u64;
    let archive = Archive::from_source(outer, len).unwrap();
    let from_loose = CmSource::day(&loose, CmKind::Indices, at())
        .unwrap()
        .unwrap();
    let from_archive = archive.day(CmKind::Indices, at()).unwrap().unwrap();
    assert_eq!(
        ListingSum::of(from_loose.entries()),
        ListingSum::of(from_archive.entries())
    );
    assert_eq!(from_loose.member_bytes(), Some(day_zip.len() as u64));
    let read_loose = read_day(&loose, &nifty(), at()).unwrap().unwrap();
    let read_archive = read_day(&archive, &nifty(), at()).unwrap().unwrap();
    assert_eq!(read_loose, read_archive);
    assert_eq!(&*read_loose.bytes, nifty_csv(at()).as_slice());
    // A day the root does not hold is absent, and a tree it holds no zip of
    // is absent: neither is invented.
    let next = Day::new(2026, 9, 28).unwrap();
    assert!(
        CmSource::day(&loose, CmKind::Indices, next)
            .unwrap()
            .is_none()
    );
    assert!(
        CmSource::day(&loose, CmKind::Stocks, at())
            .unwrap()
            .is_none()
    );
}

/// The options tree reads the same as a year zip that files the same day
/// zip: the same entries, sizes, CRCs and order.
#[test]
fn a_loose_options_day_lists_like_the_year_zip_that_files_it() {
    let folder = crate::gdfl_nfo::day_folder_name(at());
    let name = format!("{folder}\\Options\\NIFTY25SEP2610000CE.NFO.csv");
    let day_zip = zip(&[(&format!("{folder}/"), b""), (&name, b"x\r\n")]);
    let root = fresh("nfo");
    put(&root, &LooseKind::Options.path_of(at()), &day_zip);
    let loose = DayZips::open(&root).unwrap();
    let years = fresh("nfo_years");
    let year = zip(&[(&format!("SEP_2026/{folder}.zip"), &day_zip)]);
    std::fs::write(years.join("2026.zip"), year).unwrap();
    let zips = NfoZips::new(&years);
    let from_loose = NfoSource::day(&loose, at()).unwrap().unwrap();
    let from_year = NfoSource::day(&zips, at()).unwrap().unwrap();
    assert_eq!(
        ListingSum::of(from_loose.entries()),
        ListingSum::of(from_year.entries())
    );
    assert_eq!(from_loose.member_bytes(), from_year.member_bytes());
    let file = &from_loose.entries()[1];
    assert_eq!(NfoSource::fetch(&loose, file).unwrap(), b"x\r\n");
}

/// A16 and A19: a zip in another kind's folder or another month's is named
/// where it was found and where it belongs, for its day only; a stray file
/// is counted and named, never opened.
#[test]
fn a_loose_zip_in_the_wrong_kind_or_month_folder_is_named() {
    let root = fresh("misfiled");
    let next = Day::new(2026, 9, 28).unwrap();
    let wrong_kind = "STOCKS/SEP_2026/GFDLCM_INDICES_TICK_25092026.zip";
    let wrong_month = "INDICES/OCT_2026/GFDLCM_INDICES_TICK_28092026.zip";
    put(&root, wrong_kind, &index_day_zip(at()));
    put(&root, wrong_month, &index_day_zip(next));
    put(&root, "INDICES/SEP_2026/notes.txt", b"not a zip");
    put(&root, "README", b"");
    let third = Day::new(2026, 9, 29).unwrap();
    put(
        &root,
        &LooseKind::Indices.path_of(third),
        &index_day_zip(third),
    );
    let loose = DayZips::open(&root).unwrap();
    assert_eq!(
        CmSource::day(&loose, CmKind::Indices, at()).err(),
        Some(CmRefusal::DayZipMisfiled {
            found: wrong_kind.to_owned(),
            expected: "INDICES/SEP_2026/GFDLCM_INDICES_TICK_25092026.zip".to_owned(),
        })
    );
    assert_eq!(
        CmSource::day(&loose, CmKind::Indices, next).err(),
        Some(CmRefusal::DayZipMisfiled {
            found: wrong_month.to_owned(),
            expected: "INDICES/SEP_2026/GFDLCM_INDICES_TICK_28092026.zip".to_owned(),
        })
    );
    // Another day of the same root is untouched by either.
    assert!(read_day(&loose, &nifty(), third).unwrap().is_some());
    let census = loose.census();
    assert_eq!((census.held, census.misfiled, census.stray), (1, 2, 2));
    let mut stray = census.stray_named.clone();
    stray.sort();
    assert_eq!(stray, ["INDICES/SEP_2026/notes.txt", "README"]);
    // The refusal leaves the day open (D-4818): a moved zip fills it.
    let cause =
        crate::gdfl_import::cm_cause(&CmSource::day(&loose, CmKind::Indices, at()).unwrap_err());
    assert_eq!((cause.code, cause.closes), ("day_zip_misfiled", false));
}

/// A17: the same day twice, in two month folders or as a `(1)` copy beside
/// it, refuses that day only, as a duplicate that never closes it.
#[test]
fn a_loose_day_given_twice_is_refused_for_that_day() {
    let root = fresh("twice");
    let next = Day::new(2026, 9, 28).unwrap();
    put(
        &root,
        &LooseKind::Indices.path_of(at()),
        &index_day_zip(at()),
    );
    put(
        &root,
        "INDICES/SEP_2026/GFDLCM_INDICES_TICK_25092026 (1).zip",
        &index_day_zip(at()),
    );
    put(
        &root,
        &LooseKind::Indices.path_of(next),
        &index_day_zip(next),
    );
    put(
        &root,
        "INDICES/OCT_2026/GFDLCM_INDICES_TICK_28092026.zip",
        &index_day_zip(next),
    );
    let third = Day::new(2026, 9, 29).unwrap();
    put(
        &root,
        &LooseKind::Indices.path_of(third),
        &index_day_zip(third),
    );
    let loose = DayZips::open(&root).unwrap();
    for day in [at(), next] {
        let why = CmSource::day(&loose, CmKind::Indices, day).unwrap_err();
        assert_eq!(why, CmRefusal::ArchiveDuplicateDay { day });
        let cause = crate::gdfl_import::cm_cause(&why);
        assert_eq!((cause.code, cause.closes), ("archive_duplicate_day", false));
    }
    assert!(read_day(&loose, &nifty(), third).unwrap().is_some());
}

/// A20 and C2: a name whose day is impossible, or far outside the
/// calendar's span, is named in the census and never filed, so it can size
/// nothing; the real day beside it reads.
#[test]
fn a_far_dated_loose_zip_name_never_sizes_the_day_table() {
    let root = fresh("far");
    put(
        &root,
        "INDICES/DEC_9999/GFDLCM_INDICES_TICK_31129999.zip",
        b"",
    );
    put(
        &root,
        "INDICES/APR_2024/GFDLCM_INDICES_TICK_31042024.zip",
        b"",
    );
    put(
        &root,
        "INDICES/JAN_1970/GFDLCM_INDICES_TICK_0101197.zip",
        b"",
    );
    // RA-305: 29 February of a plain year is no day; of a leap year it is.
    put(
        &root,
        "INDICES/FEB_2023/GFDLCM_INDICES_TICK_29022023.zip",
        b"",
    );
    let leap = Day::new(2024, 2, 29).unwrap();
    put(
        &root,
        &LooseKind::Indices.path_of(leap),
        &index_day_zip(leap),
    );
    put(
        &root,
        &LooseKind::Indices.path_of(at()),
        &index_day_zip(at()),
    );
    let loose = DayZips::open(&root).unwrap();
    assert_eq!(loose.census().invalid, 4);
    assert_eq!(loose.days.len(), 2, "only the real days are filed");
    assert!(read_day(&loose, &nifty(), leap).unwrap().is_some());
    assert!(read_day(&loose, &nifty(), at()).unwrap().is_some());
    let far = Day::new(9_999, 12, 31).unwrap();
    assert!(
        CmSource::day(&loose, CmKind::Indices, far)
            .unwrap()
            .is_none()
    );
}

/// B27: a file that is not a zip at all, an error page saved under a day
/// zip's name, refuses that day by what is wrong with it, and never closes.
#[test]
fn a_loose_zip_that_is_not_a_zip_is_refused_for_its_day() {
    let root = fresh("html");
    put(
        &root,
        &LooseKind::Indices.path_of(at()),
        b"<html><body>Too many requests</body></html>",
    );
    let empty = LooseKind::Indices
        .path_of(at())
        .replace("25092026", "28092026");
    put(&root, &empty, &[]);
    let loose = DayZips::open(&root).unwrap();
    for day in [at(), Day::new(2026, 9, 28).unwrap()] {
        let why = CmSource::day(&loose, CmKind::Indices, day).unwrap_err();
        assert!(
            matches!(why, CmRefusal::ArchiveMalformed { .. }),
            "{day}: {why}"
        );
        assert!(!crate::gdfl_import::cm_cause(&why).closes);
    }
}

/// A zip gone between the walk and the read is unavailable by name, never
/// a day the source does not hold; a link at a day zip's name is refused by
/// the reading door, never followed.
#[test]
fn a_loose_zip_gone_or_linked_is_refused_never_absent() {
    let root = fresh("gone");
    let path = LooseKind::Indices.path_of(at());
    put(&root, &path, &index_day_zip(at()));
    let next = Day::new(2026, 9, 28).unwrap();
    let elsewhere = fresh("gone_target").join("day.zip");
    std::fs::write(&elsewhere, index_day_zip(next)).unwrap();
    std::os::unix::fs::symlink(&elsewhere, root.join(LooseKind::Indices.path_of(next))).unwrap();
    let loose = DayZips::open(&root).unwrap();
    std::fs::rename(root.join(&path), root.join("moved.bin")).unwrap();
    assert_eq!(
        CmSource::day(&loose, CmKind::Indices, at()).err(),
        Some(CmRefusal::ArchiveUnavailable {
            kind: std::io::ErrorKind::NotFound
        })
    );
    assert_eq!(
        CmSource::day(&loose, CmKind::Indices, next).err(),
        Some(CmRefusal::ArchiveMalformed {
            what: NOT_A_ZIP_FILE
        })
    );
}

/// An empty root holds no day; a root that does not exist is refused by
/// name before any day is asked.
#[test]
fn an_empty_or_missing_root_is_named() {
    let root = fresh("empty");
    let loose = DayZips::open(&root).unwrap();
    assert_eq!(loose.census(), &DayZipsCensus::default());
    assert!(
        CmSource::day(&loose, CmKind::Indices, at())
            .unwrap()
            .is_none()
    );
    assert_eq!(NfoSource::day(&loose, at()).unwrap().map(|d| d.day()), None);
    assert_eq!(
        DayZips::open(&root.join("absent")).err(),
        Some(CmRefusal::ArchiveUnavailable {
            kind: std::io::ErrorKind::NotFound
        })
    );
}

/// A13: the placeholder shapes. `outer.zip`, a streamed zip cut off before
/// its central directory (a local header at 0 and no end record), is a
/// stray name under the root and, read as a zip, `archive_malformed` with no
/// end record; an options placeholder whose one entry sits outside its day
/// folder lists no option file, so it is never a day read whole.
#[test]
fn the_placeholder_shapes_are_refused_by_name() {
    let whole = zip(&[("a.txt", b"placeholder")]);
    let cut = whole[..whole.len() - 22 - 46 - 5].to_vec();
    assert_eq!(&cut[..4], &0x0403_4b50_u32.to_le_bytes());
    let root = fresh("placeholder");
    put(&root, "outer.zip", &cut);
    let monday = Day::new(2019, 1, 7).unwrap();
    let folder = crate::gdfl_nfo::day_folder_name(monday);
    put(
        &root,
        &LooseKind::Options.path_of(monday),
        &zip(&[("Options/S.NFO.csv", b"x")]),
    );
    let loose = DayZips::open(&root).unwrap();
    assert_eq!(loose.census().stray_named, ["outer.zip"]);
    let len = cut.len() as u64;
    assert_eq!(
        zip_entries(&cut, 0, len).err(),
        Some(CmRefusal::ArchiveMalformed {
            what: "no end-of-central-directory record ends it"
        })
    );
    let listing = NfoSource::day(&loose, monday).unwrap().unwrap();
    assert_eq!(listing.entries().len(), 1);
    assert_eq!(listing.ticker_of(&listing.entries()[0]), None, "{folder}");
}

/// The walk stops past its cap and files nothing: a root that is not a
/// directory of day zips is refused whole, never half read.
#[test]
fn a_root_past_the_walk_cap_is_refused_whole() {
    let root = fresh("cap");
    // Four entries: the INDICES folder, its month folder, a day zip and a
    // stray file.
    put(
        &root,
        &LooseKind::Indices.path_of(at()),
        &index_day_zip(at()),
    );
    put(&root, "INDICES/SEP_2026/notes.txt", b"");
    assert_eq!(
        DayZips::open_capped(&root, 3).err(),
        Some(CmRefusal::ArchiveMalformed {
            what: TOO_MANY_ENTRIES
        })
    );
    let at_cap = DayZips::open_capped(&root, 4).unwrap();
    assert_eq!((at_cap.census().held, at_cap.census().stray), (1, 1));
    assert!(TOO_MANY_ENTRIES.starts_with("dayzips_too_many_entries"));
}
