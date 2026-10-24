//! C-CHK-01 (CC-20, FM-615, D-6800): `pull::gdfl_archive::Archive::from_source`,
//! the capital-market outer archive's central directory read ONCE PER RUN and
//! filed into its dense day table (two passes over the directory in memory,
//! no map, no sort). It is O(entries) by design and is not a per-day cost:
//! this bench times it at 1x, 10x and 100x the entries (about 100, 1,000 and
//! 10,000), prints the per-run time at each, and holds the cost PER ENTRY to
//! the ceiling across each tenfold step ([`linear_per_unit`], D-6800), so the
//! claim is "once per run, O(entries), this many microseconds", never "O(1)".
//! The operator's real archive holds 4,209 entries (`docs/06-limits.md`, 13
//! Oct), inside the measured range, so its figure is not an extrapolation of
//! the shape.
//!
//! The per-day lookup AFTER the read is C-GI-04's (`ratio/day_table.rs`), one
//! index into the day table and that day zip's own directory: cited, not
//! proved again here.
//!
//! The archive mixes the shapes a run meets: an index and a stock day zip for
//! every calendar day from 2018-09-03 (month and year boundaries every 30 and
//! 365 entries), a folder entry where each month begins (named `other` and
//! counted, never filed), and at every size the same mix, so the cost per
//! entry compares like with like. Every byte is invented here, in memory, and
//! read through a borrowed slice: no disk and no copy of the bytes is in the
//! figure. The harness, `crates/pull/benches/ratio.rs`, fails on a ratio above
//! its ceiling and prints p50, p99 and max beside it.

use std::hint::black_box;

use pull::gdfl_archive::{Archive, ReadAt, crc32, day_zip_name};
use pull::gdfl_cm::{CmKind, CmSource, day_folder_name};
use pull::session::Day;

use super::{cost_ps, ratio, refuse, spread};

/// Calendar days at 1x; 10x and 100x hold ten and a hundred times as many.
const BASE_DAYS: u32 = 50;

/// THE RULE FOR A COST THAT IS O(n) ONCE PER RUN (D-6800), shared by every
/// CC-20 row that claims one: the cost per unit at 1x, 10x and 100x, held to
/// the ceiling across EACH TENFOLD STEP (1x to 10x, 10x to 100x), and the
/// 1x-to-100x figure printed beside them, not gated.
///
/// An O(1) claim is held from its 1x baseline, because nothing about it may
/// grow. A linear one is not: one pass whose working set grows with n can
/// cost more per unit at 100x than at 1x without a second pass anywhere. The
/// census load did, on the cloud host of 24 Oct 2026 (`ratio/census_load.rs`,
/// about 3.8x per entry from 1,000 to 100,000 entries, and its cause, cache
/// residency or the faulting-in of fresh memory, was NOT measured), and a gate
/// from 1x would read that as a breach of a bound the code never claimed.
/// What the per-step ceiling refuses is the regression the claim forbids: a
/// second pass, a sort per entry or a walk per entry makes the cost per unit
/// grow about tenfold per tenfold step.
pub(crate) fn linear_per_unit(label: &str, [one, ten, hundred]: [u128; 3]) -> bool {
    let first = ratio(&format!("{label}, 1x -> 10x"), one, ten);
    let second = ratio(&format!("{label}, 10x -> 100x"), ten, hundred);
    let permille = hundred * 1_000 / one.max(1);
    println!(
        "  {label}, 1x -> 100x (context, not gated): {}.{:03}x",
        permille / 1_000,
        permille % 1_000
    );
    first & second
}

/// Archive bytes read in place, as `Archive<File>` reads a file: one
/// positional copy per read, nothing owned, so timing a fresh read never
/// times a copy of the whole archive first.
struct Borrowed<'a>(&'a [u8]);

impl ReadAt for Borrowed<'_> {
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        let from = usize::try_from(offset).map_err(|_| std::io::ErrorKind::UnexpectedEof)?;
        let held = self
            .0
            .get(from..)
            .and_then(|rest| rest.get(..buf.len()))
            .ok_or(std::io::ErrorKind::UnexpectedEof)?;
        buf.copy_from_slice(held);
        Ok(())
    }
}

fn le16(out: &mut Vec<u8>, v: usize) {
    let v = u16::try_from(v).unwrap_or_else(|_| refuse("a zip field past 16 bits"));
    out.extend_from_slice(&v.to_le_bytes());
}

fn le32(out: &mut Vec<u8>, v: usize) {
    let v = u32::try_from(v).unwrap_or_else(|_| refuse("a zip field past 32 bits"));
    out.extend_from_slice(&v.to_le_bytes());
}

/// A plain stored zip of `members`, in order (the shape of `day_table.rs`'s,
/// shared with `nfo_year.rs`, which writes its year zip with it).
pub(crate) fn stored_zip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
    let (mut out, mut central) = (Vec::new(), Vec::new());
    for (name, data) in members {
        let offset = out.len();
        let crc = usize::try_from(crc32(data)).unwrap_or_else(|_| refuse("a CRC"));
        out.extend_from_slice(&0x0403_4b50_u32.to_le_bytes());
        for v in [20, 0, 0, 0, 0] {
            le16(&mut out, v);
        }
        le32(&mut out, crc);
        le32(&mut out, data.len());
        le32(&mut out, data.len());
        le16(&mut out, name.len());
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);
        central.extend_from_slice(&0x0201_4b50_u32.to_le_bytes());
        for v in [20, 20, 0, 0, 0, 0] {
            le16(&mut central, v);
        }
        le32(&mut central, crc);
        le32(&mut central, data.len());
        le32(&mut central, data.len());
        le16(&mut central, name.len());
        for v in [0, 0, 0, 0] {
            le16(&mut central, v);
        }
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let at = out.len();
    out.extend_from_slice(&central);
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    for v in [0, 0, members.len(), members.len()] {
        le16(&mut out, v);
    }
    le32(&mut out, central.len());
    le32(&mut out, at);
    le16(&mut out, 0);
    out
}

/// The `n`th calendar day from 2018-09-03.
fn nth_day(n: u32) -> Day {
    let first = Day::new(2018, 9, 3).unwrap_or_else(|_| refuse("a real day"));
    Day::from_days(first.days_from_epoch() + n).unwrap_or_else(|_| refuse("a day in range"))
}

/// One small stored day zip of `kind` on `day`: its folder and one file of
/// made-up rows.
fn day_zip(kind: CmKind, day: Day) -> Vec<u8> {
    let folder = day_folder_name(kind, day);
    stored_zip(&[
        (format!("{folder}/"), Vec::new()),
        (
            format!("{folder}/INVENTED.NSE.csv"),
            b"Ticker,Date,Time,LTP\nINVENTED,01/01/2000,09:15:00,1.25\n".to_vec(),
        ),
    ])
}

/// An outer archive of `days` calendar days, its bytes, how many entries its
/// directory holds and how many of them are month folders.
fn archive_bytes(days: u32) -> (Vec<u8>, usize, usize) {
    let mut members = Vec::new();
    let mut folders = 0;
    for n in 0..days {
        let day = nth_day(n);
        for kind in [CmKind::Indices, CmKind::Stocks] {
            let name = day_zip_name(kind, day);
            if day.day() == 1 || n == 0 {
                let Some((folder, _)) = name.rsplit_once('/') else {
                    refuse("a day zip name with a folder")
                };
                members.push((format!("{folder}/"), Vec::new()));
                folders += 1;
            }
            members.push((name, day_zip(kind, day)));
        }
    }
    let entries = members.len();
    (stored_zip(&members), entries, folders)
}

/// Reads `bytes` as an archive, refusing what does not open.
fn open(bytes: &[u8]) -> Archive<Borrowed<'_>> {
    let len = u64::try_from(bytes.len()).unwrap_or_else(|_| refuse("a length"));
    Archive::from_source(Borrowed(bytes), len)
        .unwrap_or_else(|why| refuse(&format!("the invented archive: {why}")))
}

/// What a read must reach before it is timed: every day filed in both trees,
/// and every folder counted as ignored.
fn check(days: u32, bytes: &[u8], folders: usize) {
    let archive = open(bytes);
    if archive.ignored() != folders {
        refuse("the month folders counted as ignored");
    }
    for n in [0, days / 2, days - 1] {
        for kind in [CmKind::Indices, CmKind::Stocks] {
            if !matches!(archive.day(kind, nth_day(n)), Ok(Some(l)) if l.entries().len() == 2) {
                refuse("an invented day zip that does not list its two entries");
            }
        }
    }
    // A day before the first and after the last is no day, never a refusal.
    let before = Day::new(2018, 9, 2).unwrap_or_else(|_| refuse("a real day"));
    for outside in [before, nth_day(days)] {
        if !matches!(archive.day(CmKind::Indices, outside), Ok(None)) {
            refuse("a day outside the archive answered something");
        }
    }
}

/// The extremes a run can hand the read: no entry at all, and one entry.
fn the_extremes_open() {
    let empty = stored_zip(&[]);
    let archive = open(&empty);
    if archive.ignored() != 0
        || !matches!(archive.day(CmKind::Indices, nth_day(0)), Ok(None))
    {
        refuse("an archive of no entries opens empty");
    }
    let day = nth_day(0);
    let one = stored_zip(&[(day_zip_name(CmKind::Indices, day), day_zip(CmKind::Indices, day))]);
    if !matches!(open(&one).day(CmKind::Indices, day), Ok(Some(l)) if l.entries().len() == 2) {
        refuse("an archive of one day zip files it");
    }
    // Bytes that are no zip are refused by name, never read as empty.
    if Archive::from_source(Borrowed(b"not a zip at all"), 16).is_ok() {
        refuse("bytes with no end record were read as an archive");
    }
}

/// C-CHK-01: the central directory read per entry at 1x, 10x and 100x.
pub(crate) fn directory_read_is_flat_per_entry() -> bool {
    the_extremes_open();
    let mut per_entry = Vec::with_capacity(3);
    let mut largest = None;
    for days in [BASE_DAYS, BASE_DAYS * 10, BASE_DAYS * 100] {
        let (bytes, entries, folders) = archive_bytes(days);
        check(days, &bytes, folders);
        let reps = (BASE_DAYS * 100 / days).max(1);
        let whole = cost_ps(reps, || {
            open(black_box(&bytes)).ignored()
        });
        println!(
            "  C-CHK-01 directory read once at {entries:>6} entries ({} bytes): {} us per run",
            bytes.len(),
            whole / 1_000_000
        );
        per_entry.push(whole / u128::try_from(entries).unwrap_or(u128::MAX).max(1));
        largest = Some((bytes, entries));
    }
    let Ok(per_entry) = <[u128; 3]>::try_from(per_entry) else {
        refuse("three sizes measured")
    };
    let ok = linear_per_unit("C-CHK-01 directory read per entry", per_entry);
    let Some((bytes, entries)) = largest else {
        refuse("the largest archive")
    };
    spread(
        &format!("C-CHK-01 directory read, {entries} entries"),
        200,
        || {
            black_box(open(black_box(&bytes)).ignored());
        },
    );
    ok
}
