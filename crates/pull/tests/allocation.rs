//! Heap allocations, counted: `pull::csv::decode` allocates nothing per row.
//!
//! # Why this file holds the workspace's one unsafe exception
//!
//! `crates/pull/src/csv.rs` says a row allocates nothing whatever its line
//! holds. Until D-0721 that sentence stood over a `Vec<&str>` collected for
//! every line. D-0721's first proof read `decode_rows`'s source text for a
//! collect, and a review put a per-row allocation back twice, once as a
//! turbofish collect in `decode_rows` and once as a collect inside
//! `fields_of`, with every CSV test still green. A text search can only find
//! the spellings it knows. An allocation is what the claim is about, so an
//! allocation is what this file counts.
//!
//! Counting allocations needs a global allocator, and implementing
//! `GlobalAlloc` is `unsafe`. The workspace denies `unsafe_code` as a lint
//! level and every crate root under `crates/*/src` forbids it outright (Gate
//! 16). This integration test crate is neither of those roots, so the lint
//! exception below applies to this file alone, and Gate 5 counts it as one of
//! the three it permits. D-0724.
//!
//! # What the unsafe code does
//!
//! Each method forwards to [`System`] with exactly the arguments it was given,
//! so the caller's obligations reach `System` unchanged. Before forwarding, an
//! allocating method adds one to a per-thread call count and the bytes it asked
//! for to a per-thread byte count. The counts are thread-local so tests running
//! on other threads of this binary cannot move them. They are `Cell<usize>`
//! with a `const` initialiser and no destructor, so reading one inside the
//! allocator allocates nothing and cannot recurse.
//!
//! The same counts also keep the bytes live on this thread and their peak, so
//! a test can MEASURE the high-water mark of a decode rather than argue it
//! (o1api-33, D-2291). `dealloc` subtracts the layout it frees and `realloc`
//! moves the live count from the old size to the new one.

// THE ONE UNSAFE EXCEPTION IN THE WORKSPACE, AND IT IS SCOPED TO THIS TEST
// CRATE. D-0724. Written on one line, as Gate 5 counts it.
#![allow(unsafe_code)]
// The exceptions every test crate here takes: a test that cannot panic cannot
// fail.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use pull::csv::{Columns, CsvError, decode};
use pull::http::decode_body;
use pull::vendor::{Feed, Listing, Transport};

thread_local! {
    /// Allocating calls made on this thread: `alloc`, `alloc_zeroed` and
    /// `realloc`.
    static CALLS: Cell<usize> = const { Cell::new(0) };
    /// Bytes those calls asked for: the layout's size, or a `realloc`'s new
    /// size.
    static BYTES: Cell<usize> = const { Cell::new(0) };
    /// Bytes allocated on this thread and not yet freed on it.
    static LIVE: Cell<usize> = const { Cell::new(0) };
    /// The highest [`LIVE`] has been since [`peak_of`] last reset it.
    static PEAK: Cell<usize> = const { Cell::new(0) };
}

/// Adds one call of `size` bytes to this thread's counts.
fn note(size: usize) {
    let _counted = CALLS.try_with(|calls| calls.set(calls.get().saturating_add(1)));
    let _sized = BYTES.try_with(|bytes| bytes.set(bytes.get().saturating_add(size)));
    grow(size);
}

/// `size` more bytes live on this thread, and the peak raised to meet them.
fn grow(size: usize) {
    let _live = LIVE.try_with(|live| {
        let now = live.get().saturating_add(size);
        live.set(now);
        let _peak = PEAK.try_with(|peak| peak.set(peak.get().max(now)));
    });
}

/// `size` bytes freed on this thread.
fn shrink(size: usize) {
    let _live = LIVE.try_with(|live| live.set(live.get().saturating_sub(size)));
}

/// [`System`], counted.
struct Counting;

// SAFETY: every method forwards to `System` with the arguments it was given,
// so `System` receives exactly the caller's pointer, layout and size, and each
// caller's `GlobalAlloc` obligations pass through unchanged. `note` touches
// only a `const`-initialised thread-local `Cell<usize>`, which neither
// allocates nor frees.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: forwarded unchanged; the caller upholds `alloc`'s contract.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        note(layout.size());
        // SAFETY: forwarded unchanged; the caller upholds `alloc_zeroed`'s
        // contract.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        shrink(layout.size());
        note(new_size);
        // SAFETY: forwarded unchanged; the caller upholds `realloc`'s
        // contract, including that `ptr` came from this allocator with
        // `layout`, and every block this allocator hands out is `System`'s.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        shrink(layout.size());
        // SAFETY: forwarded unchanged; `ptr` came from `System` through this
        // allocator with `layout`, which is `dealloc`'s contract.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// What `run` returned, and the calls and bytes it allocated on this thread.
fn counted<T>(run: impl FnOnce() -> T) -> (T, usize, usize) {
    let calls = CALLS.with(Cell::get);
    let bytes = BYTES.with(Cell::get);
    let out = run();
    let calls = CALLS.with(Cell::get).saturating_sub(calls);
    let bytes = BYTES.with(Cell::get).saturating_sub(bytes);
    (out, calls, bytes)
}

/// What `run` returned, and the most bytes it held live on this thread at
/// once, above what was live when it started.
fn peak_of<T>(run: impl FnOnce() -> T) -> (T, usize) {
    let start = LIVE.with(Cell::get);
    PEAK.with(|peak| peak.set(start));
    let out = run();
    let peak = PEAK.with(Cell::get).saturating_sub(start);
    (out, peak)
}

/// One `TrueData` index row, as observed beside `Columns::TrueDataIndex`.
const ROW: &str = "20221003,09:15:01,38444.90,0,0\n";

/// **TWICE THE ROWS COST NO ALLOCATION PER ROW.** D-0721, D-0724.
///
/// 4,096 rows and then 8,192 are decoded, and the second may make at most four
/// more allocating calls than the first. A row that allocated would add at
/// least 4,096. What the added rows may cost is the row vector itself: at
/// D-0721 `decode_rows` started from `Vec::new()`, so a longer body could take
/// another doubling; since D-1203 it is reserved once from the body's newline
/// count, so the slack is unused. One untimed decode runs first, so a one-time setup anywhere under
/// `decode` is not charged to the smaller body.
#[test]
fn twice_the_rows_cost_no_allocation_per_row() {
    let warm = decode(ROW, Columns::TrueDataIndex).map(|rows| rows.len());
    assert_eq!(warm, Ok(1), "the fixture row decodes");
    let few = ROW.repeat(4_096);
    let many = ROW.repeat(8_192);

    let (few_rows, few_calls, _) = counted(|| decode(&few, Columns::TrueDataIndex));
    let (many_rows, many_calls, _) = counted(|| decode(&many, Columns::TrueDataIndex));

    assert_eq!(few_rows.map(|rows| rows.len()), Ok(4_096));
    assert_eq!(many_rows.map(|rows| rows.len()), Ok(8_192));
    let added = many_calls.saturating_sub(few_calls);
    assert!(
        added <= 4,
        "4,096 more rows made {added} more allocating calls ({few_calls} for \
         4,096 rows, {many_calls} for 8,192): a row allocates"
    );
}

/// **A LINE OF A MILLION COMMAS IS REFUSED WITHOUT ALLOCATING FOR ITS
/// FIELDS.** D-0721, D-0724.
///
/// Every field is counted, so the refusal reports the line's true width. The
/// fields are borrowed into a fixed array, so the bytes allocated on the way to
/// that refusal must be fewer than the line's own bytes. A vector holding the
/// line's 1,000,001 fields holds one `&str` per field, each larger than the
/// one-byte comma it would point past.
#[test]
fn a_line_of_a_million_commas_is_refused_without_allocating_for_its_fields() {
    let warm = decode(ROW, Columns::TrueDataIndex).map(|rows| rows.len());
    assert_eq!(warm, Ok(1), "the fixture row decodes");
    let wide = ",".repeat(1_000_000);

    let (outcome, _, bytes) = counted(|| decode(&wide, Columns::TrueDataIndex));

    assert_eq!(
        outcome,
        Err(CsvError::FieldCount {
            line: 1,
            got: 1_000_001,
            want: 5,
        }),
        "the refusal reports the line's true width"
    );
    assert!(
        bytes < wide.len(),
        "refusing a {}-byte line allocated {bytes} bytes: its fields were held",
        wide.len()
    );
}

/// The shipped HTTP descriptor of `feed`.
fn http_spec(feed: Feed) -> pull::vendor::HttpSpec {
    match feed.descriptor().transport {
        Transport::Http(spec) => spec,
        Transport::LocalArchive(_) => panic!("{feed:?} is an HTTP feed"),
    }
}

/// A Dhan index answer of `bars` one-minute bars, as the vendor quotes it:
/// rupee prices with paise, a zero volume and an epoch-second stamp.
fn dhan_body(bars: usize) -> String {
    let column = |each: &dyn Fn(usize) -> String| (0..bars).map(each).collect::<Vec<_>>().join(",");
    let price = |i: usize| format!("{}.{:02}", 24_000 + i % 997, i % 100);
    format!(
        "{{\"open\":[{p}],\"high\":[{p}],\"low\":[{p}],\"close\":[{p}],\
         \"volume\":[{v}],\"timestamp\":[{t}]}}",
        p = column(&price),
        v = column(&|_| "0".to_owned()),
        t = column(&|i| (1_751_337_900 + 60 * i).to_string()),
    )
}

/// A Zerodha equity answer of `bars` positional rows, the vendor's own row
/// shape (charter §4z): an ISO stamp with its offset, four prices, a volume.
fn zerodha_body(bars: usize) -> String {
    let rows = (0..bars)
        .map(|i| {
            let price = format!("{}.{:02}", 1_700 + i % 97, i % 100);
            format!(
                "[\"2017-12-15T09:15:00+0530\",{price},{price},{price},{price},{}]",
                2_000 + i
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"status\":\"success\",\"data\":{{\"candles\":[{rows}]}}}}")
}

/// **THE PEAK MEMORY OF A JSON DECODE, MEASURED (o1api-33, D-2291).**
///
/// `pull::http::decode_body` parses the whole body into a `serde_json::Value`
/// tree before reading a field, and until D-2291 its peak was an argued bound
/// ("~16x, ~32x with digits") that no allocator had counted. This counts every
/// byte the decoding thread holds live, so the figure is the decode's own
/// high-water mark above the body it was handed: the tree, every transient
/// buffer and the decoded columns, in bytes ASKED of the allocator (its own
/// per-block overhead is not visible here, and that is the one part still
/// argued).
///
/// Three bodies: a Dhan 90-day one-minute chunk (34,000 bars) and a Zerodha
/// answer of the same size, both as the vendors quote them, and the cheapest
/// text per node a hostile answer can send, one array of 1,000,000 zeros,
/// which the decode refuses only after the tree is built. Each peak is held
/// under the ratio `docs/06-limits.md` states for it, and printed.
#[test]
fn a_json_decodes_peak_memory_is_measured_against_its_body() {
    let dhan = http_spec(Feed::Dhan);
    let zerodha = http_spec(Feed::Zerodha);
    let warm = decode_body(&dhan_body(2), &dhan, Listing::Index).map(|w| w.rows.len());
    assert_eq!(warm.ok(), Some(2), "the Dhan fixture decodes");

    let body = dhan_body(34_000);
    let (window, dhan_peak) = peak_of(|| decode_body(&body, &dhan, Listing::Index));
    assert_eq!(window.map(|w| w.rows.len()).ok(), Some(34_000));
    let dhan_ratio = dhan_peak / body.len();

    let positional = zerodha_body(34_000);
    let (window, zerodha_peak) = peak_of(|| decode_body(&positional, &zerodha, Listing::Equity));
    assert_eq!(window.map(|w| w.rows.len()).ok(), Some(34_000));
    let zerodha_ratio = zerodha_peak / positional.len();

    let hostile = format!("{{\"open\":[{}0]}}", "0,".repeat(999_999));
    let (refused, hostile_peak) = peak_of(|| decode_body(&hostile, &dhan, Listing::Index));
    assert!(refused.is_err(), "one array of zeros is not a window");
    let hostile_ratio = hostile_peak / hostile.len();

    println!(
        "decode_body peak above the body: dhan {dhan_peak} B for {} B ({dhan_ratio}x); \
         zerodha {zerodha_peak} B for {} B ({zerodha_ratio}x); \
         hostile {hostile_peak} B for {} B ({hostile_ratio}x)",
        body.len(),
        positional.len(),
        hostile.len()
    );
    // MEASURED 2026-10-04: dhan 20,489,303 B for 1,666,062 B (12x), zerodha
    // 18,036,440 B for 2,270,041 B (7x), hostile 34,555,384 B for 2,000,010 B
    // (17x). Allocation sizes do not depend on the machine, so the ceilings
    // sit one step above what was measured and fail on a regression.
    assert!(dhan_ratio <= 13, "dhan {dhan_ratio}x");
    assert!(zerodha_ratio <= 8, "zerodha {zerodha_ratio}x");
    assert!(hostile_ratio <= 18, "hostile {hostile_ratio}x");
    assert!(
        hostile_ratio >= 8,
        "the hostile body is the one the tree costs most on, {hostile_ratio}x"
    );
}
