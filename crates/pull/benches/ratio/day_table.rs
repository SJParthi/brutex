//! C-GI-04 (IA-3, design §10.2 "Day lookup in the CM archive", critic
//! IA-1-5, rules audit RA-111): finding one day inside the capital-market
//! outer archive is one index into its dense day table, then that day zip's
//! own local header and directory, so the cost is flat at 1x, 10x and 100x
//! the days the archive holds. The archive is opened once per size (its
//! directory read once, O(entries), which this ratio does not time).
//!
//! Every byte is invented here, in memory: each day zip holds one folder
//! entry and one small file of made-up rows, stored, as the vendor's outer
//! archive stores its day zips. Nothing is written to disk.

use pull::gdfl_archive::{Archive, crc32, day_zip_name};
use pull::gdfl_cm::{CmKind, CmSource, day_folder_name};
use pull::session::Day;

use super::{cost_ps, ratio, refuse, spread};

/// Days at 1x; 10x and 100x are ten and a hundred times it.
const BASE_DAYS: u32 = 100;
/// Calls per timed repetition.
const REPS: u32 = 2_000;

fn le16(out: &mut Vec<u8>, v: usize) {
    let v = u16::try_from(v).unwrap_or_else(|_| refuse("a field past 16 bits"));
    out.extend_from_slice(&v.to_le_bytes());
}

fn le32(out: &mut Vec<u8>, v: usize) {
    let v = u32::try_from(v).unwrap_or_else(|_| refuse("a field past 32 bits"));
    out.extend_from_slice(&v.to_le_bytes());
}

/// A plain stored zip of `members`, in order.
fn stored_zip(members: &[(String, Vec<u8>)]) -> Vec<u8> {
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

/// The `n`th invented day, counted from 2018-09-03.
fn nth_day(n: u32) -> Day {
    let first = Day::new(2018, 9, 3).unwrap_or_else(|_| refuse("a real day"));
    Day::from_days(first.days_from_epoch() + n).unwrap_or_else(|_| refuse("a day in range"))
}

/// An outer archive of `days` index day zips, every calendar day from
/// 2018-09-03, opened from memory.
fn archive(days: u32) -> Archive<Vec<u8>> {
    let members: Vec<(String, Vec<u8>)> = (0..days)
        .map(|n| {
            let day = nth_day(n);
            let folder = day_folder_name(CmKind::Indices, day);
            let inner = stored_zip(&[
                (format!("{folder}/"), Vec::new()),
                (
                    format!("{folder}/INVENTED.NSE_IDX.csv"),
                    b"Ticker,Date,Time,LTP\nINVENTED,01/01/2000,09:15:00,1.25\n".to_vec(),
                ),
            ]);
            (day_zip_name(CmKind::Indices, day), inner)
        })
        .collect();
    let bytes = stored_zip(&members);
    let len = u64::try_from(bytes.len()).unwrap_or_else(|_| refuse("a length"));
    Archive::from_source(bytes, len).unwrap_or_else(|why| refuse(&format!("the archive: {why}")))
}

/// C-GI-04: one day's listing out of the outer archive, the middle day of
/// each size, flat at 1x, 10x and 100x the days.
pub(crate) fn day_lookup_is_flat() -> bool {
    let archives = [BASE_DAYS, BASE_DAYS * 10, BASE_DAYS * 100].map(|n| (n, archive(n)));
    let probe = |(n, a): &(u32, Archive<Vec<u8>>)| {
        let day = nth_day(n / 2);
        cost_ps(
            REPS,
            || matches!(a.day(CmKind::Indices, day), Ok(Some(listing)) if listing.entries().len() == 2),
        )
    };
    // Every size answers its middle day with its two entries, or the ratio
    // below would time a refusal.
    for (n, a) in &archives {
        if !matches!(a.day(CmKind::Indices, nth_day(n / 2)), Ok(Some(l)) if l.entries().len() == 2)
        {
            refuse("an invented day did not list its two entries");
        }
    }
    let [one, ten, hundred] = &archives;
    let base = probe(one);
    let a = ratio("C-GI-04 cm archive day, 10x days", base, probe(ten));
    let b = ratio("C-GI-04 cm archive day, 100x days", base, probe(hundred));
    let mid = nth_day(hundred.0 / 2);
    spread("C-GI-04 cm archive day, 100x days", 20_000, || {
        let _listed = std::hint::black_box(hundred.1.day(CmKind::Indices, mid));
    });
    a && b
}
