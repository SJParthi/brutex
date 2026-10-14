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

// ---------------------------------------------------------------------------
// `store::sealed_codec`: codec 1's decode bound, counted (FM-284, D-4939).
//
// WHY THESE ARE HERE AND NOT IN `crates/store/tests`. Counting allocations
// needs the global allocator above, and this file is the one test root Gate 16
// lets implement it (`--allow-unsafe-test-root`). `pull` depends on `store`, so
// the codec is reachable from here, and a second unsafe test root would be a
// second exception for one more allocator. `crates/store/tests/sealed_codec_alloc.rs`
// proves the same refusals through the buffer's capacity, which needs no
// unsafe and runs with the store suite; these count every byte the thread
// asks for, which the capacity cannot see.
//
// A sealed month's frame states its own rows. A head or a stream that lies —
// bit rot that keeps the checksum, or a writer bug that wrote a checksum over
// wrong bytes — must be refused by name before the decoder reserves anything,
// and an accepted frame reserves its checked rows times `size_of::<Bar>()` and
// no more. Every frame here is built from invented bars.
// ---------------------------------------------------------------------------

mod sealed_codec {
    use std::mem::size_of;

    use store::crc::crc32c;
    use store::format::{Bar, OI_NULL};
    use store::sealed_codec::{
        BLOCK_ROWS, Column, FrameError, Part, decode_frame, decode_stamps, encode_frame,
    };

    use super::counted;

    /// One decoder's answer, then the calls and bytes it allocated.
    type Counted = (Result<usize, FrameError>, usize, usize);

    /// The specification's one-row frame: head, six one-byte varints (open
    /// interest null rides in the flags), seven four-byte empty streams.
    fn one_row_frame() -> Vec<u8> {
        let bar = Bar {
            ts_micros: 0,
            open: 0,
            high: 0,
            low: 0,
            close: 0,
            volume: 0,
            open_interest: OI_NULL,
        };
        let mut frame = Vec::new();
        encode_frame(&[bar], 0, BLOCK_ROWS, &mut frame).expect("one row");
        assert_eq!(frame.len(), 38, "the specification's one-row frame");
        frame
    }

    /// A full block of invented bars whose every column moves, so every
    /// stream has a packed region to cut short.
    fn full_frame() -> Vec<u8> {
        let rows: Vec<Bar> = (0..i64::from(BLOCK_ROWS))
            .map(|i| {
                let close = 1_111_100 + (i * 7_919) % 4_001;
                Bar {
                    ts_micros: 60_000_000 * (i + 1) + i % 3,
                    open: close - (i * 31) % 97,
                    high: close + 50 + (i * 13) % 211,
                    low: close - 150 - (i * 17) % 173,
                    close,
                    volume: 1 + (i * 104_729) % 65_537,
                    open_interest: 3_000 + (i * 61) % 509,
                }
            })
            .collect();
        let mut frame = Vec::new();
        encode_frame(&rows, 0, BLOCK_ROWS, &mut frame).expect("full");
        frame
    }

    /// The head's row count rewritten; the checksum is taken over the result,
    /// so only the codec's own checks stand between the lie and the rows.
    fn with_rows(frame: &[u8], rows: u16) -> Vec<u8> {
        let mut lied = frame.to_vec();
        lied[2..4].copy_from_slice(&rows.to_le_bytes());
        lied
    }

    /// Both decoders over `frame` under its own checksum, each into an empty
    /// buffer, counted.
    fn decode_both(frame: &[u8], block_rows: u16) -> (Counted, Counted) {
        let crc = crc32c(frame);
        let mut bars: Vec<Bar> = Vec::new();
        let mut stamps: Vec<i64> = Vec::new();
        let full = counted(|| decode_frame(frame, crc, 0, block_rows, &mut bars));
        let only = counted(|| decode_stamps(frame, crc, 0, block_rows, &mut stamps));
        (full, only)
    }

    /// **A HEAD STATING MORE ROWS THAN ITS BYTES HOLD ALLOCATES NOTHING.**
    /// FM-284's own test. A head above the month's `block_rows` is refused
    /// before row 0 is read. A full block's honest head over bytes cut short
    /// at every length is the case the decoder used to pay for: it reserved
    /// the head's 256 rows (14,336 bytes) before reading the first stream, and
    /// gave the room back only as a truncation.
    #[test]
    fn a_block_stating_more_than_its_rows_is_refused_without_allocating() {
        for rows in [BLOCK_ROWS + 1, u16::MAX] {
            let lied = with_rows(&one_row_frame(), rows);
            let (full, only) = decode_both(&lied, BLOCK_ROWS);
            let named = Err(FrameError::Rows {
                rows,
                block_rows: BLOCK_ROWS,
            });
            assert_eq!(full, (named, 0, 0), "decode_frame, {rows} rows");
            assert_eq!(only, (named, 0, 0), "decode_stamps, {rows} rows");
        }

        let full = full_frame();
        let mut in_a_stream = 0;
        for len in 0..full.len() {
            let (frame_answer, stamp_answer) = decode_both(&full[..len], BLOCK_ROWS);
            let Err(FrameError::Truncated { part }) = frame_answer.0 else {
                panic!("a frame cut to {len} bytes was {:?}", frame_answer.0);
            };
            assert_eq!(
                (frame_answer.1, frame_answer.2),
                (0, 0),
                "decode_frame allocated before refusing a cut at {len}"
            );
            assert_eq!(
                stamp_answer,
                (frame_answer.0, 0, 0),
                "decode_stamps, cut at {len}"
            );
            if matches!(part, Part::Stream(_)) {
                in_a_stream += 1;
            }
        }
        // Every cut past the head and row 0 lands in a stream.
        assert!(
            in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
            "{in_a_stream} of {} cuts landed in a stream",
            full.len()
        );
    }

    /// **A STREAM HEADER THAT LIES IS REFUSED BY NAME, ALLOCATING NOTHING.** An
    /// exception count of 2^64 − 1, a width of 64 bits over a region the frame
    /// does not hold, a width past 64, a byte after the seventh stream, and a
    /// checksum that does not match.
    #[test]
    fn a_stream_header_that_lies_is_refused_without_allocating() {
        let one = one_row_frame();
        // The ts stream starts after the 4-byte head and six one-byte varints:
        // lead, exception count, base, gcd.
        let ts = 10;
        assert_eq!(
            one[ts..ts + 4],
            [0, 0, 0, 1],
            "ts: width 0, none, base 0, gcd 1"
        );

        let mut count = one[..=ts].to_vec();
        count.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]);
        count.extend_from_slice(&one[ts + 2..]);
        let mut wide = with_rows(&one, BLOCK_ROWS);
        wide[ts] = 64;
        let mut wider = one.clone();
        wider[ts] = 65;
        let mut trailing = one.clone();
        trailing.push(0);
        let stamp = Column::Stamp;
        for (frame, named) in [
            (
                count,
                FrameError::ExceptionCount {
                    column: stamp,
                    count: u64::MAX,
                    residuals: 0,
                },
            ),
            (
                wide,
                FrameError::Truncated {
                    part: Part::Stream(stamp),
                },
            ),
            (
                wider,
                FrameError::Width {
                    column: stamp,
                    width: 65,
                },
            ),
            (trailing, FrameError::Trailing { bytes: 1 }),
        ] {
            let (full, only) = decode_both(&frame, BLOCK_ROWS);
            assert_eq!(full, (Err(named), 0, 0), "decode_frame");
            assert_eq!(only, (Err(named), 0, 0), "decode_stamps");
        }

        let mut bars: Vec<Bar> = Vec::new();
        let bad = crc32c(&one) ^ 1;
        let (answer, calls, bytes) = counted(|| decode_frame(&one, bad, 0, BLOCK_ROWS, &mut bars));
        assert!(
            matches!(answer, Err(FrameError::Checksum { .. })),
            "{answer:?}"
        );
        assert_eq!((calls, bytes), (0, 0), "a bad checksum allocated");
    }

    /// **AN ACCEPTED BLOCK RESERVES ITS STATED ROWS, ONCE, AND NO MORE.** A
    /// full block into an empty buffer is one call of 256 × 56 bytes; into a
    /// buffer with the room, nothing. The ceiling: a month head stating
    /// `u16::MAX` rows a block (the month head's reader refuses that; the
    /// codec is bounded without it). Seven width-0 streams hold any number of
    /// rows in no bytes, so the 38-byte frame relabelled decodes to 65,535
    /// rows and reserves exactly 3,669,960 bytes, the most one call can ask.
    #[test]
    fn an_accepted_block_reserves_its_stated_rows_and_no_more() {
        let stride = size_of::<Bar>();
        assert_eq!(stride, 56, "seven i64 fields");
        let rows = usize::from(BLOCK_ROWS);

        let full = full_frame();
        let (whole, stamps) = decode_both(&full, BLOCK_ROWS);
        assert_eq!(whole, (Ok(rows), 1, rows * stride), "decode_frame");
        assert_eq!(
            stamps,
            (Ok(rows), 1, rows * size_of::<i64>()),
            "decode_stamps"
        );

        let crc = crc32c(&full);
        let mut bars: Vec<Bar> = Vec::with_capacity(rows);
        let mut stamps: Vec<i64> = Vec::with_capacity(rows);
        let room = counted(|| decode_frame(&full, crc, 0, BLOCK_ROWS, &mut bars));
        assert_eq!(room, (Ok(rows), 0, 0), "decode_frame with room");
        let room = counted(|| decode_stamps(&full, crc, 0, BLOCK_ROWS, &mut stamps));
        assert_eq!(room, (Ok(rows), 0, 0), "decode_stamps with room");

        let most = usize::from(u16::MAX);
        let (whole, stamps) = decode_both(&with_rows(&one_row_frame(), u16::MAX), u16::MAX);
        assert_eq!(whole, (Ok(most), 1, 3_669_960), "decode_frame");
        assert_eq!(most * stride, 3_669_960);
        assert_eq!(
            stamps,
            (Ok(most), 1, most * size_of::<i64>()),
            "decode_stamps"
        );
    }
}
