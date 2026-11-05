#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the summary decoder's refusals no other test reached,
//! each made by changing one field of a real summary's own bytes: a day
//! entry whose seconds and bar count disagree in each way, a reserved
//! version byte, a payload past the largest month, a payload cut inside its
//! day entries; the padded-minute count of a summary that counts decreasing
//! stamps; and a kept file the host will not open. Every bar is invented.

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::format::OI_NULL;

const SYMBOL: u32 = 0x5D3;

fn may() -> YearMonth {
    YearMonth::new(2031, 5).unwrap()
}

/// 09:15 IST on day `day` of May 2031, plus `minute` minutes.
fn at(day: i64, minute: i64) -> i64 {
    ((first_day_of(may()) + day - 1) * 86_400 + 33_300 - 19_800 + minute * 60) * 1_000_000
}

/// A version-2 one-minute summary holding days 5 and 6 (days 1 to 4 hold
/// nothing).
fn sample() -> MonthSummary {
    let mut summary = MonthSummary::new(SYMBOL, 60, may());
    for day in [5, 6] {
        for minute in 0..30 {
            summary.fold(&Bar {
                ts_micros: at(day, minute),
                open: 1_111_000,
                high: 1_111_900,
                low: 1_110_100,
                close: 1_111_500 + minute,
                volume: 4,
                open_interest: OI_NULL,
            });
        }
    }
    summary
}

/// `payload` with day `day`'s entry (bars, first second, last second) set.
fn with_entry(payload: &[u8], day: usize, entry: (u32, u32, u32)) -> Vec<u8> {
    let mut out = payload.to_vec();
    let at = HEAD_LEN + DAY_LEN * day;
    out[at..at + 4].copy_from_slice(&entry.0.to_le_bytes());
    out[at + 4..at + 8].copy_from_slice(&entry.1.to_le_bytes());
    out[at + 8..at + 12].copy_from_slice(&entry.2.to_le_bytes());
    out
}

/// **A DAY ENTRY WHOSE SECONDS AND BAR COUNT DISAGREE IS REFUSED, IN EACH WAY
/// THEY CAN.** An empty day is "no first, no last, no bars"; a held day has a
/// first second at or before its last, inside the day, and bars. Day 1 holds
/// nothing in the sample; each entry below breaks exactly one of those rules
/// there and is refused by name. The sample itself decodes (the control).
#[test]
fn a_day_entry_whose_seconds_and_bar_count_disagree_is_refused_each_way() {
    let payload = sample().payload();
    assert_eq!(MonthSummary::decode(VERSION, &payload), Ok(sample()));
    let cases = [
        ("bars on a day with no seconds", (1, NO_SECOND, NO_SECOND)),
        ("a first second after the last", (1, 600, 300)),
        ("a last second past the day", (1, 300, 86_400)),
        ("seconds with no bars", (0, 300, 600)),
    ];
    for (why, entry) in cases {
        assert_eq!(
            MonthSummary::decode(VERSION, &with_entry(&payload, 0, entry)),
            Err(Refused::Shape("a day's first and last second disagree")),
            "{why}"
        );
    }
}

/// **A SECTION WITH A RESERVED VERSION BYTE SET IS REFUSED AS ITS VERSION.**
/// The checksum holds over the changed bytes, so the reserved byte, not the
/// checksum, refuses it; the untouched section decodes.
#[test]
fn a_section_with_a_reserved_version_byte_set_is_refused_as_its_version() {
    let section = sample().section();
    assert_eq!(MonthSummary::from_section(&section), Ok(sample()));
    let mut body = section[..section.len() - 4].to_vec();
    body[3] = 0x01;
    let mut changed = body.clone();
    changed.extend_from_slice(&crc32c(&body).to_le_bytes());
    assert_eq!(
        MonthSummary::from_section(&changed),
        Err(Refused::Version(VERSION))
    );
}

/// **A PAYLOAD PAST THE LARGEST MONTH OR CUT INSIDE ITS DAY ENTRIES IS
/// REFUSED.** One byte more than the largest month's payload is no head and
/// days; a payload ending two bytes into day 3's entry reads zeros there, and
/// a day of zero seconds with no bars is no day at all.
#[test]
fn a_payload_past_the_largest_month_or_cut_inside_its_days_is_refused() {
    assert_eq!(
        MonthSummary::decode(VERSION, &vec![0_u8; MAX_PAYLOAD_LEN + 1]),
        Err(Refused::Shape("the payload is not a head and its days"))
    );
    let payload = sample().payload();
    let cut = &payload[..HEAD_LEN + DAY_LEN * 2 + 2];
    assert_eq!(
        MonthSummary::decode(VERSION, cut),
        Err(Refused::Shape("a day's first and last second disagree"))
    );
}

/// **A SUMMARY THAT COUNTS DECREASING STAMPS COUNTS NO PADDED MINUTE.** "The
/// row before" is not one minute's row when a stamp went backwards, so the
/// carry is undefined, whether or not any stamp was off the grid.
#[test]
fn a_summary_that_counts_decreasing_stamps_counts_no_padded_minute() {
    let mut summary = sample();
    let day = summary.first_day() + 4;
    assert!(summary.padded_on(day, &[(555, 585)]).is_ok());
    summary.counts.invalid = 0;
    summary.counts.decreasing = 1;
    assert_eq!(
        summary.padded_on(day, &[(555, 585)]),
        Err(NotCounted::InvalidStamps)
    );
}

/// **A KEPT SUMMARY THE HOST WILL NOT OPEN IS UNREADABLE, IN THE HOST'S
/// WORDS.** Not a link, not absent: the open's own refusal names the file.
/// Root opens a mode-000 file, so the body runs where the mode bits bind: as
/// `nobody` when this process is root (D-0995).
#[test]
fn a_kept_summary_the_host_will_not_open_is_unreadable_in_its_words() {
    crate::support::where_permission_binds(
        "month_summary::decode_cover_tests::a_kept_summary_the_host_will_not_open_is_unreadable_in_its_words",
        a_kept_summary_the_host_will_not_open_is_unreadable_in_its_words_body,
    );
}

fn a_kept_summary_the_host_will_not_open_is_unreadable_in_its_words_body() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir: PathBuf = std::env::temp_dir().join(format!(
        "brutex-msum-decode-cover-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let bars = dir.join("2031-05.bin");
    let kept = path_beside(&bars);
    std::fs::write(&kept, sample().file_image()).unwrap();
    std::fs::set_permissions(&kept, std::fs::Permissions::from_mode(0o000)).unwrap();
    let refused = read_kept(&bars, 2_000);
    std::fs::set_permissions(&kept, std::fs::Permissions::from_mode(0o644)).unwrap();
    let _cleanup = std::fs::remove_dir_all(&dir);
    match refused {
        Err(Refused::Unreadable(why)) => {
            assert!(why.starts_with(&kept.display().to_string()), "{why}");
            assert!(!why.contains("symbolic link"), "{why}");
        }
        other => panic!("the open is refused in the host's words: {other:?}"),
    }
}

/// **A BIT PAST A BITMAP'S END IS DROPPED, NEVER WRAPPED INTO IT** (both
/// bit writers' own contracts, asked directly: every caller stays inside
/// its bitmap or refuses first). Setting bit 64 of a one-word bitmap, or
/// OR-ing a byte from bit 64, changes nothing; a byte from bit 60 keeps its
/// four low bits and drops the four that run past the end.
#[test]
fn a_bit_past_a_bitmaps_end_is_dropped_never_wrapped() {
    let mut words = [0_u64; 1];
    set(&mut words, 64);
    set(&mut words, 1_000_000);
    assert_eq!(words, [0], "set past the end");
    or_byte(&mut words, 64, 0xFF);
    or_byte(&mut words, 1_000, 0xFF);
    assert_eq!(words, [0], "or past the end");
    or_byte(&mut words, 60, 0xFF);
    assert_eq!(words, [0xF << 60], "the bits inside are kept");
    set(&mut words, 3);
    assert_eq!(words, [(0xF << 60) | 0b1000]);
}
