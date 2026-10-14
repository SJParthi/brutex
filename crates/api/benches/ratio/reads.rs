//! Gate 8 for the `/db` reads no other module times: a month's bars, its
//! gaps, the feeds holding a series, and the census as the store page reads
//! it (R3-14, RA-064).
//!
//! THREE OF THESE ARE NOT O(1) A REQUEST, BY DESIGN, and are gated per row:
//!
//! * `/bars.json` answers a whole window, the month or the days asked (the
//!   paged reader is `/bars/window.json`, timed in `window`): its cost per
//!   answered bar is gated, for the whole month and for one day;
//! * `/gaps.json` audits the month's seconds where an index owes them: its
//!   cost per bar of the month is gated;
//! * `/instruments.json` lists every instrument the census holds when the
//!   feed has no master, and sends no `ETag`, so a page cannot revalidate it:
//!   its cost per listed instrument is gated.
//!
//! `/store.json` IS the census too, but a page holds its body and asks with
//! `If-None-Match`: that `304` is what is gated, per request. `/sources.json`
//! (every rung each feed holds of NIFTY's index) is gated per request.
//!
//! The proof of each bound above is the ratio bench, `crates/api/benches/ratio.rs`,
//! whose `reads` module this is, run by `cargo bench -p api --bench ratio`
//! (gate 8): it prints p50, p99 and max at 1x, 10x and 100x and fails on a
//! ratio past its ceiling.

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
/// `ETag` is timed whole instead, and says so in its label.
fn revalidated(served: &mut Served, label: &str, path: &str, needle: &str) -> Target {
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
        None => ok_per(
            &format!("{label}, whole (no ETag)"),
            path.to_owned(),
            needle,
            Per::Row("\"symbol\":"),
        ),
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
            revalidated(served, "store body", "/store.json?feed=gdfl", "NIFTY"),
            revalidated(
                served,
                "instrument list",
                "/instruments.json?feed=gdfl",
                "\"symbol\"",
            ),
        ]
    })
}
