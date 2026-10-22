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

/// One answer's cost, and its spread printed: `token_on(day)` for one token.
fn measured(book: &Book, day: Day, label: &str) -> u128 {
    if !matches!(
        book.token_on(day, "NFO", 9_000_500),
        TokenOn::Resolved { .. }
    ) {
        refuse("the probed token resolves");
    }
    spread(label, 20_000, || {
        black_box(book.token_on(black_box(day), "NFO", black_box(9_000_500)));
    });
    cost_ps(200_000, || {
        matches!(
            black_box(book).token_on(black_box(day), "NFO", black_box(9_000_500)),
            TokenOn::Resolved { .. }
        )
    })
}

/// C-MS-01: `token_on` at 1, 10 and 100 snapshots, in its two shapes. On a
/// snapshot day one snapshot answers (one probe): measured at 1, 10 and 100.
/// Between two snapshots the next one is probed too (two probes): measured at
/// 10 and 100, the sizes that have a day between two snapshots. Each shape is
/// held to the ceiling against its own smallest size.
pub(crate) fn token_on_is_flat() -> bool {
    let mut on_day = Vec::new();
    let mut between = Vec::new();
    for count in [1_u32, 10, 100] {
        let Ok(book) = Book::of((0..count).map(snapshot).collect()) else {
            refuse("the invented book builds")
        };
        let Some(last) = book.stamps().last().copied() else {
            refuse("a book of snapshots")
        };
        on_day.push(measured(
            &book,
            last.day,
            &format!("C-MS-01 token_on, a snapshot day, {count} snapshots"),
        ));
        if count > 1 {
            let Ok(day) = Day::from_days(last.day.days_from_epoch().saturating_sub(3)) else {
                refuse("a real day")
            };
            between.push(measured(
                &book,
                day,
                &format!("C-MS-01 token_on, between two snapshots, {count} snapshots"),
            ));
        }
    }
    let ([one, ten, hundred], [ten_between, hundred_between]) =
        (on_day.as_slice(), between.as_slice())
    else {
        refuse("five sizes measured")
    };
    ratio("C-MS-01 token_on, a snapshot day, 10x", *one, *ten)
        & ratio("C-MS-01 token_on, a snapshot day, 100x", *one, *hundred)
        & ratio(
            "C-MS-01 token_on, between two, 100 vs 10",
            *ten_between,
            *hundred_between,
        )
}
