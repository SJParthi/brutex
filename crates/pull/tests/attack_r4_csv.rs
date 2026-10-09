//! ATTACK ROUND 4, CSV: the other skip on the line D-3125 fixed.
//!
//! D-3125 carried a row `csv::decode` skips for a negative VOLUME into
//! `DecodeSkips`, so the archive door's receipt counts it. A row skipped for a
//! negative OPEN INTEREST (D-2683) is skipped by the same pass and was counted
//! only into the "file decoded" log line: `decode_counted` built its
//! `DecodeSkips` from the volume count alone, so `rows_read` came up one short
//! and `balances` held over a row that was on no line of the receipt (D-3133).
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use pull::csv::{Columns, decode_counted};

#[test]
fn a_row_skipped_for_negative_open_interest_is_counted_as_a_decoder_skip() {
    // Two rows offered: one kept, one skipped for an open interest of -5.
    let body = "20221003,09:15:01,38445.65,0,-5\n20221003,09:15:02,38419.40,0,7\n";
    let (rows, skipped) = decode_counted(body, Columns::TrueDataFno).expect("one bad row");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(skipped.negative_open_interest, 1, "{skipped:?}");
    assert_eq!(skipped.negative_volume, 0, "{skipped:?}");
    assert_eq!(
        rows.len() + skipped.total(),
        2,
        "every offered row is accounted for"
    );

    // BOTH REASONS ON ONE FILE, each counted once under its own name.
    let body = "20221003,09:15:01,38445.65,-1,7\n20221003,09:15:02,38419.40,0,-5\n\
                20221003,09:15:03,38419.40,0,9\n";
    let (rows, skipped) = decode_counted(body, Columns::TrueDataFno).expect("two bad rows");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(
        (skipped.negative_volume, skipped.negative_open_interest),
        (1, 1),
        "{skipped:?}"
    );
    assert_eq!(rows.len() + skipped.total(), 3);

    // A CLEAN FILE SKIPS NOTHING, and a zero open interest is zero, not a skip.
    let (rows, skipped) =
        decode_counted("20221003,09:15:01,38445.65,0,0\n", Columns::TrueDataFno).expect("clean");
    assert_eq!((rows.len(), skipped.total()), (1, 0), "{skipped:?}");
}
