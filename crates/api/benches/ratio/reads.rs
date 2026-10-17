//! Gate 8 for the `/db` reads no other module times: a month's bars, its
//! gaps, the feeds holding a series, and the census as the store page reads
//! it (R3-14, RA-064).
//!
//! TWO OF THESE ARE NOT O(1) A REQUEST, BY DESIGN, and are gated per row:
//!
//! * `/bars.json` answers a whole window, the month or the days asked (the
//!   paged reader is `/bars/window.json`, timed in `window`): its cost per
//!   answered bar is gated, for the whole month and for one day;
//! * `/gaps.json` audits the month's seconds where an index owes them: its
//!   cost per bar of the month is gated.
//!
//! `/store.json` and `/instruments.json` ARE the census too, but a page holds
//! their bodies and asks with `If-None-Match`: that `304` is what is gated,
//! per request. `/instruments.json` was gated per listed instrument while it
//! sent no `ETag`, and that ratio straddled the ceiling on the operator's Mac
//! (0.474x on one run, 3.002x BREACH on the next: a multi-megabyte body's
//! cost per row is the machine's memory, not the route); it now sends one
//! (R3-14 repair 1). A route here that stops sending its `ETag` stops the
//! bench, named (repair 2): it is never quietly timed per row instead.
//! `/sources.json` (every rung each feed holds of NIFTY's index) is gated per
//! request.
//!
//! The FIRST, whole answer of each is still O(rows) by design: `cold` in the
//! printed line, and named in `docs/06-limits.md` (owed by WP-99).
//!
//! The proof of each bound above is the ratio bench, `crates/api/benches/ratio.rs`,
//! whose `reads` module this is, run by `cargo bench -p api --bench ratio`
//! (gate 8): it prints p50, p99 and max at 1x, 10x and 100x and fails on a
//! ratio past its ceiling.

use super::refuse;
use super::served::{Per, Served, Target, flat, ok_per, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &[
    "/bars.json",
    "/gaps.json",
    "/sources.json",
    "/store.json",
    "/instruments.json",
];

/// The month the reads ask.
const MONTH: &str = "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&month=2024-06";

/// The revalidation of `path`: its `ETag`, asked back. A route that sends no
/// `ETag` STOPS the bench (R3-14 repair 2): it used to be timed whole per row
/// instead, which quietly swapped the gate for the one that straddled its
/// ceiling, so a route that lost its validator read as a pass.
fn revalidated(served: &mut Served, label: &str, path: &str) -> Target {
    let answer = served.ask(path, "");
    let tag = answer
        .head
        .lines()
        .find_map(|line| line.strip_prefix("etag:"))
        .map(str::trim)
        .map(str::to_owned);
    match tag {
        Some(tag) => Target {
            label: format!("{label}, revalidated"),
            path: path.to_owned(),
            extra: format!("If-None-Match: {tag}\r\n"),
            status: 304,
            needle: String::new(),
            per: Per::Request,
        },
        None => refuse(&format!(
            "{label}: {path} answered {} with no ETag, so a page cannot revalidate it and its \
             304 cannot be timed; the route lost its validator",
            answer.status
        )),
    }
}

/// Every target at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `main` over the stores `served` built.
pub(crate) fn reads_are_flat(stores: &mut [Served]) -> bool {
    flat("reads", stores, |served| {
        vec![
            ok_per(
                "bars, the whole month",
                format!("/bars.json?{MONTH}"),
                "\"c\":",
                Per::Row("\"t\":"),
            ),
            ok_per(
                "bars, its first day",
                format!("/bars.json?{MONTH}&from=2024-06-03&to=2024-06-03"),
                "\"c\":",
                Per::Row("\"t\":"),
            ),
            ok_per(
                "gaps of the month",
                format!("/gaps.json?{MONTH}"),
                "\"folded_with\"",
                Per::MonthBar,
            ),
            ok_with(
                "sources of NIFTY",
                "/sources.json?exchange=NSE&segment=INDEX&symbol=NIFTY".to_owned(),
                "gdfl",
            ),
            revalidated(served, "store body", "/store.json?feed=gdfl"),
            revalidated(served, "instrument list", "/instruments.json?feed=gdfl"),
        ]
    })
}
