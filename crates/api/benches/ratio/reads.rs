//! Gate 8 for the `/db` reads no other module times: a month's bars, its
//! gaps, the feeds holding a series, and the census as the store page reads
//! it (R3-14, RA-064).
//!
//! TWO OF THESE ARE NOT O(1) A REQUEST, BY DESIGN, and are gated per row:
//!
//! * a whole month, read the way a client must: page by page through
//!   `/bars/window.json` (`paged`), its cost per row read gated; and one day
//!   of it in one `/bars.json` answer, its cost per answered bar gated. One
//!   `/bars.json` answer serves at most 60,000 rows (R3-08), so the whole
//!   month, 102,400 bars at 100x, is never asked of it: that ask was this
//!   module's, the route refused it and gate 8 exited 1 (BASE-BENCH-1028,
//!   D-7400). The refusal is timed instead (`refusals_are_flat`): `400
//!   too_many_rows` naming the 60,000 cap, its p50 per request gated from
//!   60,001 rows to 102,400 and to 2,592,000;
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

use super::paged::{self, MONTH_ANSWER_CAP_ROWS};
use super::served::{Per, Served, Target, flat, lone_month, ok_per, ok_with};
use super::{RATIO_CEILING_PERMILLE, ratio, refuse};

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

/// The same month as the window's query, which [`paged::read_month`] pages.
const WINDOW: &str =
    "feed=gdfl&exchange=NSE&segment=INDEX&symbol=NIFTY&timeframe=1s&from=2024-06&to=2024-06";

/// The months the `/bars.json` refusal is timed on, in rows: one row past
/// the cap, the bench's own 100x month, and every second of June (30 x
/// 86,400, the fullest one-second month the store files). Every one is
/// past [`MONTH_ANSWER_CAP_ROWS`], so every one is refused; 1x to 43.2x.
const REFUSED_SIZES: [usize; 3] = [MONTH_ANSWER_CAP_ROWS + 1, 102_400, 30 * 86_400];

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
            // THE WHOLE MONTH, READ THE WAY A CLIENT MUST (BASE-BENCH-1028,
            // D-7400): page by page through `/bars/window.json`, 1,000 rows a
            // page. It was one `/bars.json` answer, which serves at most
            // 60,000 rows (R3-08) and refused the 102,400-bar month, so the
            // 100x case measured nothing and gate 8 exited 1. The refusal
            // itself is timed below, in [`refusals_are_flat`].
            ok_per(
                "bars, the whole month, paged",
                WINDOW.to_owned(),
                "\"total\":",
                Per::Paged,
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

/// Requests timed for each refused month, and asked untimed before them.
const REFUSALS_TIMED: usize = 200;

/// THE CAP IS KEPT, AND ITS REFUSAL IS O(1) (BASE-BENCH-1028, D-7400): a
/// whole-month `/bars.json` ask of a month past the 60,000 rows one answer
/// serves is refused `400 too_many_rows`, naming that cap, and the refusal
/// costs the same however large the month is.
///
/// The refusal cannot be timed on the 1x and 10x stores the other rows use
/// (1,024 and 10,240 bars answer whole, as they should), so it has stores of
/// its own: one month each of [`REFUSED_SIZES`] rows and no census. Its p50
/// per request is gated at [`RATIO_CEILING_PERMILLE`] from the smallest
/// refused month to each larger one; the labels name the sizes, not 1x / 10x
/// / 100x, because the largest month a one-second June can hold is 43.2x the
/// smallest one refused.
///
/// ALL THREE ARE BUILT, THEN WARMED, THEN TIMED IN ROUNDS. Timed one store
/// after another, straight after its server started, the first one's p50
/// was 9.1 ms against 0.12 ms for the other two (this bench, 29 Oct 2026,
/// load about 60): the server's start-up work and the machine's load, not
/// the refusal, so a ratio from it would pass whatever the refusal cost.
/// Each store now answers [`REFUSALS_TIMED`] untimed asks first, and each
/// timed round asks every store once, so a change in the machine's load
/// falls on all three alike.
pub(crate) fn refusals_are_flat() -> bool {
    let needle = paged::cap_sentence(MONTH_ANSWER_CAP_ROWS);
    let path = format!("/bars.json?{MONTH}");
    let mut stores: Vec<(usize, Served, String)> = REFUSED_SIZES
        .iter()
        .map(|&rows| {
            let mut served = lone_month(rows);
            let first = served.ask(&path, "");
            if first.status != 400
                || !first.body.contains("too_many_rows")
                || !first.body.contains(&needle)
            {
                let shown: String = first.body.chars().take(400).collect();
                refuse(&format!(
                    "the whole-month /bars.json ask of {rows} rows answered {} without \
                     too_many_rows and {needle:?}: {shown}",
                    first.status
                ));
            }
            let shown: String = first.body.chars().take(120).collect();
            (rows, served, shown)
        })
        .collect();
    let mut times: Vec<Vec<u128>> = vec![Vec::with_capacity(REFUSALS_TIMED); stores.len()];
    for round in 0..2 * REFUSALS_TIMED {
        for ((rows, served, _), took) in stores.iter_mut().zip(times.iter_mut()) {
            let asked = std::time::Instant::now();
            let answer = served.ask(&path, "");
            let nanos = asked.elapsed().as_nanos();
            if answer.status != 400 {
                refuse(&format!(
                    "the {rows}-row refusal answered {} while timed",
                    answer.status
                ));
            }
            if round >= REFUSALS_TIMED {
                took.push(nanos);
            }
        }
    }
    let mut p50s = Vec::with_capacity(stores.len());
    for ((rows, served, shown), took) in stores.iter().zip(times.iter_mut()) {
        took.sort_unstable();
        let at = |q: usize| {
            let k = (took.len() * q).div_ceil(100).max(1) - 1;
            took.get(k).copied().unwrap_or(0)
        };
        let (p50, p99, max) = (at(50), at(99), took.last().copied().unwrap_or(0));
        println!(
            "  reads refused /bars.json month {rows:>8} rows  p50 {p50:>9} ns  p99 {p99:>9} ns  \
             max {max:>9} ns  n={}  answered 400: {shown}",
            took.len()
        );
        p50s.push(p50 * 1_000);
        let _ignored = std::fs::remove_dir_all(&served.root);
    }
    let mut ok = true;
    if let Some((&base, rest)) = p50s.split_first() {
        for (&at, rows) in rest.iter().zip(REFUSED_SIZES.iter().skip(1)) {
            ok &= ratio(
                &format!(
                    "reads /bars.json refusal p50 {}->{rows} rows",
                    MONTH_ANSWER_CAP_ROWS + 1
                ),
                base,
                at,
                RATIO_CEILING_PERMILLE,
            );
        }
    }
    ok
}
