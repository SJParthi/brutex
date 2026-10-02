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

thread_local! {
    /// Allocating calls made on this thread: `alloc`, `alloc_zeroed` and
    /// `realloc`.
    static CALLS: Cell<usize> = const { Cell::new(0) };
    /// Bytes those calls asked for: the layout's size, or a `realloc`'s new
    /// size.
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

/// Adds one call of `size` bytes to this thread's counts.
fn note(size: usize) {
    let _counted = CALLS.try_with(|calls| calls.set(calls.get().saturating_add(1)));
    let _sized = BYTES.try_with(|bytes| bytes.set(bytes.get().saturating_add(size)));
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
        note(new_size);
        // SAFETY: forwarded unchanged; the caller upholds `realloc`'s
        // contract, including that `ptr` came from this allocator with
        // `layout`, and every block this allocator hands out is `System`'s.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
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
