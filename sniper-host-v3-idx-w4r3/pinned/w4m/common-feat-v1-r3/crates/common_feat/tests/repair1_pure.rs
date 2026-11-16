//! REPAIR 1 red cells of the pure module (check-build-feat-v1.md B2): the candle index of a Zerodha candle that starts
//! BEFORE its session's open (the Muhurat 17:30 open on Zerodha's 09:15-anchored 60-minute grid) is counted from the
//! session table's open, the first candle that overlaps the segment being k = 1 (K14). Written against the v1 API
//! before the fix: v1 gave k = 0 and 1 (RED), the repaired module gives 1 and 2. The aligned grid (every regular
//! session) is unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, missing_docs)]

use common_feat::cal::{at, candle_index, in_halt, parse_ymd};
use common_feat::mtf::MtfSlot;

#[test]
fn candle_index_first_candle_overlapping_the_open_is_1() {
    let d = parse_ymd("2018-11-07").unwrap(); // Muhurat, the session table's open 17:30
    let open = at(d, 17, 30, 0);
    assert_eq!(candle_index(at(d, 17, 15, 0), open, 3600), 1, "60min 17:15-18:15 overlaps the 17:30 open: k = 1");
    assert_eq!(candle_index(at(d, 18, 15, 0), open, 3600), 2);
    assert_eq!(candle_index(at(d, 17, 29, 0), open, 120), 1, "2min 17:29-17:31: k = 1");
    assert_eq!(candle_index(at(d, 17, 31, 0), open, 120), 2, "2min 17:31-17:33: k = 2 (never a second k = 1)");
    assert_eq!(candle_index(at(d, 17, 25, 0), open, 600), 1, "10min 17:25-17:35: k = 1");
    // the aligned grid is unchanged
    let r = parse_ymd("2026-04-01").unwrap();
    let o = at(r, 9, 15, 0);
    for k in 1..=125 {
        assert_eq!(candle_index(o + (k - 1) * 180, o, 180), k);
    }
    // the DR Saturday's second segment keeps the session grid (09:15 open)
    let dr = parse_ymd("2024-03-02").unwrap();
    assert_eq!(candle_index(at(dr, 11, 15, 0), at(dr, 9, 15, 0), 3600), 3);
}

// ---------------- B1 (K13): a higher timeframe never reads a candle from before the decision stream's reset ----------------
// Written with the fix (new API `MtfSlot::reset_at`); its red evidence is the mutant `m_mtf_reset_cross` and feat_io's
// `b1_*` vectors, which failed on the v1 code.
#[test]
fn mtf_never_reads_across_a_cont_reset() {
    let fri = parse_ymd("2025-11-14").unwrap();
    let mon = parse_ymd("2025-11-17").unwrap(); // the rights ex-date
    let mut m: MtfSlot<i32> = MtfSlot::new();
    m.put(at(fri, 15, 15, 0), 1);
    m.put(at(fri, 15, 30, 0), 2); // the last 15-minute candle before the ex-date
    assert_eq!(m.at(at(mon, 9, 18, 0)), Some(2), "without a reset the slot reads the last completed candle");
    m.reset_at(at(mon, 9, 15, 0)); // the decision stream resets at the ex-date session's open
    assert_eq!(m.at(at(mon, 9, 18, 0)), None, "after the reset the pre-ex-date candle is never read");
    assert_eq!(m.at(at(mon, 9, 27, 0)), None);
    assert_eq!(m.reset_refused(), 2);
    m.put(at(mon, 9, 30, 0), 3); // the first post-ex-date 15-minute candle
    assert_eq!(m.at(at(mon, 9, 27, 0)), None, "still forming at 09:27; the one before it is pre-reset");
    assert_eq!(m.at(at(mon, 9, 30, 0)), Some(3), "ended at 09:30: read");
    m.reset_at(at(fri, 9, 15, 0));
    assert_eq!(m.at(at(mon, 9, 33, 0)), Some(3), "a floor never falls back");
}

// ---------------- B3 (K11, plan 2.1 HALTS): the halt window is [from, resume) ----------------
#[test]
fn halt_window_is_from_inclusive_resume_exclusive() {
    let d = parse_ymd("2020-03-13").unwrap();
    let w = [(at(d, 9, 21, 9), at(d, 10, 21, 1))];
    assert!(!in_halt(at(d, 9, 21, 0), &w), "3min 09:18 candle, END 09:21:00: before the halt");
    assert!(!in_halt(at(d, 9, 21, 8), &w));
    assert!(in_halt(at(d, 9, 21, 9), &w), "from is inclusive");
    assert!(in_halt(at(d, 9, 24, 0), &w), "3min 09:21 candle, END 09:24:00: inside");
    assert!(in_halt(at(d, 10, 21, 0), &w), "3min 10:18 candle, END 10:21:00: inside");
    assert!(!in_halt(at(d, 10, 21, 1), &w), "resume is exclusive");
    assert!(!in_halt(at(d, 10, 24, 0), &w), "3min 10:21 candle, END 10:24:00: after the resumption");
    assert!(!in_halt(at(d, 9, 24, 0), &[]), "no halt rows, no refusal");
}
