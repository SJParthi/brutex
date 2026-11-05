#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    reason = "the same exception every test module in this workspace takes: a test that cannot \
              panic cannot fail"
)]
//! BASE-COV-LIBS-LS1: the sealing fold's time-table bound, asked directly.
//! Every caller builds a fold from a head whose timeframe is a real rung, so
//! a table slot always has width there; the fold's own guard is what bounds
//! its table when one does not. Every stamp and price is invented (2031).

use super::Fold;
use crate::format::{Bar, OI_NULL};

/// 00:00 IST on 2031-02-01, epoch microseconds: the month's origin.
const FEB: i64 = 1_927_650_600_000_000;

/// 00:00 IST on 2031-03-01: the month's end.
const MARCH: i64 = 1_930_069_800_000_000;

const fn bar(ts_micros: i64) -> Bar {
    Bar {
        ts_micros,
        open: 410_125,
        high: 410_750,
        low: 409_700,
        close: 410_275,
        volume: 9,
        open_interest: OI_NULL,
    }
}

/// **A FOLD WHOSE TABLE SLOT HAS NO WIDTH STILL STOPS AT ITS TABLE'S LAST
/// ROW**: with a zero timeframe the next slot never moves past a bar, so the
/// walk that fills the time table ends only because the table is full, at
/// its one row, and every later bar adds none. Beside it, a real rung's
/// fold never fills its last row from a bar (that row starts at or past the
/// month's end): `finish` writes it, as the block count.
#[test]
fn a_fold_whose_table_slot_has_no_width_stops_at_its_last_row() {
    let mut fold = Fold::new(FEB, MARCH, 0, 256);
    assert_eq!(fold.table_rows, 1);
    for k in 0..3 {
        fold.push(&bar(FEB + k * 1_000_000)).unwrap();
        assert_eq!(fold.table, vec![0], "bar {k}");
    }
    let folded = fold.finish(1);
    assert_eq!((folded.rows, folded.table), (3, vec![0]));

    let mut minute = Fold::new(FEB, MARCH, 60, 256);
    assert_eq!(minute.table_rows, 11, "ceil(28 days / 4,096 minutes) + 1");
    minute.push(&bar(MARCH - 60_000_000)).unwrap();
    assert_eq!(minute.table, vec![0; 10], "every row but the last");
    let folded = minute.finish(1);
    assert_eq!(
        folded.table.last(),
        Some(&1),
        "the last row is the block count"
    );
    assert_eq!(folded.table.len(), 11);
}
