//! C-MS-01 (R4-28, DLY-09): `master_snapshots::Book::token_on` at 1, 10 and
//! 100 dated snapshots.
//!
//! One answer is one day-table index and at most two hash probes, so its cost
//! must not grow with the number of snapshots the book holds. Each probe asks
//! a day between two snapshots, which is the costliest answer: the governing
//! snapshot and the next later one are both probed. p50, p99 and max of
//! single calls are printed beside each ratio.
//!
//! Every input is invented here (tokens in the nine millions, `ZQ` names), in
//! memory: no file is written or removed.

use std::fmt::Write as _;
use std::hint::black_box;

use pull::master_snapshots::{Book, Snapshot, Stamp, TokenOn};
use pull::session::Day;

use super::{cost_ps, ratio, refuse, spread};

/// Futures rows per snapshot.
const ROWS: u64 = 2_000;

/// One invented snapshot taken `offset` weeks after 2024-01-01.
fn snapshot(offset: u32) -> Snapshot {
    let Ok(first) = Day::new(2024, 1, 1) else {
        refuse("a real day")
    };
    let Ok(day) = Day::from_days(first.days_from_epoch() + offset * 7) else {
        refuse("a real day")
    };
    let mut csv = String::from(pull::vendor::KITE_MASTER_HEADER);
    for row in 0..ROWS {
        let _infallible = write!(
            csv,
            "\n{},{},ZQ{row:05}FUT,\"ZQ{row:05}\",0,2030-01-31,0,0.05,75,FUT,NFO-FUT,NFO",
            9_000_000 + row,
            row
        );
    }
    let Ok(parsed) = Snapshot::parse(Stamp { day, ordinal: 1 }, &csv) else {
        refuse("the invented snapshot parses")
    };
    parsed
}

/// C-MS-01: one `token_on` between two snapshots, at 1, 10 and 100 snapshots.
pub(crate) fn token_on_is_flat() -> bool {
    let mut costs = Vec::new();
    for count in [1_u32, 10, 100] {
        let Ok(book) = Book::of((0..count).map(snapshot).collect()) else {
            refuse("the invented book builds")
        };
        let stamps = book.stamps();
        // A DAY BETWEEN THE LAST TWO SNAPSHOTS (or after the only one).
        let Some(last) = stamps.last() else {
            refuse("a book of snapshots")
        };
        let Ok(day) = Day::from_days(last.day.days_from_epoch().saturating_sub(3)) else {
            refuse("a real day")
        };
        let day = if count == 1 { last.day } else { day };
        if !matches!(
            book.token_on(day, "NFO", 9_000_500),
            TokenOn::Resolved { .. }
        ) {
            refuse("the probed token resolves");
        }
        costs.push(cost_ps(200_000, || {
            matches!(
                black_box(&book).token_on(black_box(day), "NFO", black_box(9_000_500)),
                TokenOn::Resolved { .. }
            )
        }));
        spread(
            &format!("C-MS-01 token_on, {count} snapshots"),
            20_000,
            || {
                black_box(book.token_on(black_box(day), "NFO", black_box(9_000_500)));
            },
        );
    }
    let [one, ten, hundred] = costs.as_slice() else {
        refuse("three sizes measured")
    };
    ratio("C-MS-01 token_on, 10 snapshots", *one, *ten)
        & ratio("C-MS-01 token_on, 100 snapshots", *one, *hundred)
}
