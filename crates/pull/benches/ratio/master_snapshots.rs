//! C-MS-01 (R4-28, DLY-09): `master_snapshots::Book::token_on` at 1, 10 and
//! 100 dated snapshots.
//!
//! One answer is one day-table index and at most two hash probes, so its cost
//! must not grow with the number of snapshots the book holds. Each probe asks
//! a day between two snapshots, which is the costliest answer: the governing
//! snapshot and the next later one are both probed. p50, p99 and max of
//! single calls are printed beside each ratio.
//!
//! Every input is invented here (tokens in the nine millions, `ZQ` names).
//! C-MS-01 runs in memory. C-MS-02 (`futures_plan` at 1, 10 and 100 snapshots,
//! R4-28 repair 1) writes its snapshots under the system temporary directory
//! and leaves them there: no file is removed.

use std::fmt::Write as _;
use std::hint::black_box;

use pull::master_snapshots::{self, Book, Kind, Snapshot, Stamp, TokenOn};
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

/// C-MS-02's underlyings, and the futures each lists.
const UNDERLYINGS: u64 = 10;
const FUTURES_EACH: u64 = 3;

/// One invented dump of [`ROWS`] rows: three futures for each of ten `ZQ`
/// underlyings, every other row an option of one of them.
fn dump() -> String {
    let mut csv = String::from(pull::vendor::KITE_MASTER_HEADER);
    for row in 0..ROWS {
        let underlying = row % UNDERLYINGS;
        let token = 9_100_000 + row;
        let _infallible = if row < UNDERLYINGS * FUTURES_EACH {
            let month = row / UNDERLYINGS + 1;
            write!(
                csv,
                "\n{token},{row},ZQ{underlying:02}F{month},\"ZQ{underlying:02}\",0,2030-0{month}-28,0,0.05,75,FUT,NFO-FUT,NFO"
            )
        } else {
            write!(
                csv,
                "\n{token},{row},ZQ{underlying:02}C{row},\"ZQ{underlying:02}\",0,2030-01-28,{row},0.05,75,CE,NFO-OPT,NFO"
            )
        };
    }
    csv
}

/// C-MS-02 (R4-28 repair 1): `futures_plan` for one underlying, the book
/// opened as a request opens it, at 1, 10 and 100 dated snapshots on disk.
///
/// Before the first-listed index a plan opened, unpacked and parsed every
/// snapshot on or before the window's end; a second agent measured that
/// linear on the real dump (240 ms at 1, 5,008 ms at 40). Now it reads the
/// index and parses the newest snapshot alone, so its cost must not grow with
/// the snapshots held. The snapshots are written once, weekly, under the
/// system's temporary directory and left there; nothing is removed.
pub(crate) fn futures_plan_is_flat() -> bool {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let root =
        std::env::temp_dir().join(format!("brutex-bench-cms02-{}-{nanos}", std::process::id()));
    let (Ok(first), Ok(last), Ok(today)) = (
        Day::new(2024, 1, 1),
        Day::new(2025, 12, 31),
        Day::new(2026, 1, 1),
    ) else {
        refuse("real days")
    };
    let Ok(window) = pull::session::Window::new(first, last) else {
        refuse("an ordered window")
    };
    let body = dump();
    let plan = || {
        let Ok(book) = Book::open(&root) else {
            refuse("the book opens")
        };
        pull::fnowork::futures_plan(
            &book,
            pull::vendor::ZERODHA_CONTINUOUS,
            "ZQ03",
            window,
            today,
            window.to(),
        )
        .map(|plan| (plan.asks.len(), plan.refused.len()))
    };
    let mut held = 0_u32;
    let mut costs = Vec::new();
    for count in [1_u32, 10, 100] {
        while held < count {
            let Ok(on) = Day::from_days(first.days_from_epoch() + held * 7) else {
                refuse("a real day")
            };
            if master_snapshots::snapshot(&root, Kind::Kite, on, "bench://c-ms-02", body.as_bytes())
                .is_err()
            {
                refuse("the invented snapshot is written");
            }
            held += 1;
        }
        if plan() != Ok((4, 0)) {
            refuse("the plan asks three contracts and the continuous series, refusing none");
        }
        spread(
            &format!("C-MS-02 futures_plan, {count} snapshots"),
            200,
            || {
                let _measured = black_box(plan());
            },
        );
        costs.push(cost_ps(3, plan));
    }
    let [one, ten, hundred] = costs.as_slice() else {
        refuse("three sizes measured")
    };
    ratio("C-MS-02 futures_plan, 10x snapshots", *one, *ten)
        & ratio("C-MS-02 futures_plan, 100x snapshots", *one, *hundred)
}
