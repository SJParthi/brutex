#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
//! Codec 1's decode bound, seen through the caller's buffer (FM-284).
//!
//! A sealed month's frame states its own rows. A head or a stream that lies —
//! bit rot that keeps the checksum, or a writer bug that wrote a checksum over
//! wrong bytes — is refused by name before the decoder reserves anything, and
//! an accepted frame then makes one `Vec::reserve` of its checked rows. That
//! grows the buffer by `Vec`'s amortised rule, not by the rows (D-5902,
//! g-sc1-codec-3): nothing into a buffer with the room, and onto one short of
//! it the largest of twice its capacity, its length plus the rows, and 4 — so
//! exactly the rows into a buffer that never allocated when they are 4 or
//! more, and a capacity of 4 for 1 to 3.
//!
//! The decoders' only allocation is the caller's buffer: a stream is three
//! slices of the frame. So the buffer's capacity is what a refusal could have
//! grown, and it must still be zero after one. The same cases, counted by a
//! global allocator that sees every byte the thread asks for, are in
//! `crates/pull/tests/allocation.rs` (`mod sealed_codec`), the one test root
//! Gate 16 lets implement `GlobalAlloc`.
//!
//! Every frame here is built from invented bars.

use store::crc::crc32c;
use store::format::{Bar, OI_NULL};
use store::sealed_codec::{
    BLOCK_ROWS, Column, FrameError, Part, decode_frame, decode_stamps, encode_frame,
};

/// One decoder's answer and the capacity its buffer was left with.
type Grown = (Result<usize, FrameError>, usize);

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

/// A full block of invented bars whose every column moves, so every stream
/// has a packed region to cut short.
fn full_frame() -> Vec<u8> {
    frame_of(BLOCK_ROWS)
}

/// A frame of the first `rows` of [`full_frame`]'s invented bars.
fn frame_of(rows: u16) -> Vec<u8> {
    let rows: Vec<Bar> = (0..i64::from(rows))
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

/// The head's row count rewritten; the checksum is taken over the result.
fn with_rows(frame: &[u8], rows: u16) -> Vec<u8> {
    let mut lied = frame.to_vec();
    lied[2..4].copy_from_slice(&rows.to_le_bytes());
    lied
}

/// Both decoders over `frame` under its own checksum, each into an empty
/// buffer; each answer with the capacity it left.
fn decode_both(frame: &[u8], block_rows: u16) -> (Grown, Grown) {
    let crc = crc32c(frame);
    let mut bars: Vec<Bar> = Vec::new();
    let mut stamps: Vec<i64> = Vec::new();
    let full = decode_frame(frame, crc, 0, block_rows, &mut bars);
    let only = decode_stamps(frame, crc, 0, block_rows, &mut stamps);
    ((full, bars.capacity()), (only, stamps.capacity()))
}

#[test]
fn a_refused_head_or_cut_never_grows_the_buffer() {
    for rows in [BLOCK_ROWS + 1, u16::MAX] {
        let (full, only) = decode_both(&with_rows(&one_row_frame(), rows), BLOCK_ROWS);
        let named = Err(FrameError::Rows {
            rows,
            block_rows: BLOCK_ROWS,
        });
        assert_eq!(full, (named, 0), "decode_frame, {rows} rows");
        assert_eq!(only, (named, 0), "decode_stamps, {rows} rows");
    }

    // A full block's honest head over bytes cut short at every length. The
    // decoder used to reserve the head's 256 rows before the first stream and
    // give them back only as a truncation, leaving the room behind.
    let full = full_frame();
    let mut in_a_stream = 0;
    for len in 0..full.len() {
        let (frame_answer, stamp_answer) = decode_both(&full[..len], BLOCK_ROWS);
        let (Err(FrameError::Truncated { part }), 0) = frame_answer else {
            panic!("a frame cut to {len} bytes left {frame_answer:?}");
        };
        assert_eq!(stamp_answer, frame_answer, "decode_stamps, cut at {len}");
        if matches!(part, Part::Stream(_)) {
            in_a_stream += 1;
        }
    }
    assert!(
        in_a_stream > 1_000 && in_a_stream + 20 > full.len(),
        "{in_a_stream} of {} cuts landed in a stream",
        full.len()
    );
}

#[test]
fn a_lying_stream_header_never_grows_the_buffer() {
    let one = one_row_frame();
    // The ts stream starts after the 4-byte head and six one-byte varints.
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
        (trailing, FrameError::Trailing { bytes: 1 }),
    ] {
        let (full, only) = decode_both(&frame, BLOCK_ROWS);
        assert_eq!(full, (Err(named), 0), "decode_frame");
        assert_eq!(only, (Err(named), 0), "decode_stamps");
    }
}

/// An accepted block grows an EMPTY buffer by exactly its rows, up to the
/// 65,535-row ceiling. Named `an_accepted_block_grows_the_buffer_by_its_rows`
/// until D-5902: onto a buffer short of the room the growth is `Vec`'s
/// amortised rule, not the rows (`a_full_buffer_grows_by_amortised_doubling_not_by_rows`).
#[test]
fn an_accepted_block_grows_an_empty_buffer_by_its_rows() {
    let rows = usize::from(BLOCK_ROWS);
    let (full, only) = decode_both(&full_frame(), BLOCK_ROWS);
    assert_eq!(full, (Ok(rows), rows), "decode_frame");
    assert_eq!(only, (Ok(rows), rows), "decode_stamps");

    // The ceiling a month head stating `u16::MAX` rows a block would allow:
    // seven width-0 streams hold any number of rows in no bytes.
    let most = usize::from(u16::MAX);
    let (full, only) = decode_both(&with_rows(&one_row_frame(), u16::MAX), u16::MAX);
    assert_eq!(full, (Ok(most), most), "decode_frame");
    assert_eq!(only, (Ok(most), most), "decode_stamps");
}

/// The capacity `Vec::reserve` gives today's `std` when `rows` more are asked
/// of a buffer of `len` rows and `capacity`: none when they fit, else the
/// largest of twice the capacity, the length plus the rows, and 4.
fn amortised(len: usize, capacity: usize, rows: usize) -> usize {
    if capacity - len >= rows {
        capacity
    } else {
        (2 * capacity).max(len + rows).max(4)
    }
}

/// `frame` decoded by both decoders onto buffers already holding `len` rows in
/// a capacity of `capacity`; each answer with the capacity it left, and the
/// length.
fn decode_onto(frame: &[u8], len: usize, capacity: usize) -> ((Grown, usize), (Grown, usize)) {
    let crc = crc32c(frame);
    let mut bars: Vec<Bar> = Vec::with_capacity(capacity);
    bars.resize(len, Bar::default());
    let mut stamps: Vec<i64> = Vec::with_capacity(capacity);
    stamps.resize(len, 0);
    assert_eq!(
        (bars.capacity(), stamps.capacity()),
        (capacity, capacity),
        "the buffers start at exactly {capacity}"
    );
    let full = decode_frame(frame, crc, 0, BLOCK_ROWS, &mut bars);
    let only = decode_stamps(frame, crc, 0, BLOCK_ROWS, &mut stamps);
    (
        ((full, bars.capacity()), bars.len()),
        ((only, stamps.capacity()), stamps.len()),
    )
}

/// **A ONE-ROW FRAME INTO AN EMPTY BUFFER TAKES THE MINIMUM CAPACITY.**
/// g-sc1-codec-3, D-5902, SC1-09.
///
/// `Vec` never gives a 56-byte or an 8-byte element a capacity below 4 once it
/// allocates, so a frame of 1 to 3 rows leaves an empty buffer at capacity 4,
/// not at its rows; from 4 rows the capacity is the rows exactly. The extremes
/// of a frame's rows: 1, 2, 3, 4, 95, 96, 97, 171, 172, 255, 256, and the
/// specification's 38-byte one-row frame.
#[test]
fn a_one_row_frame_into_an_empty_buffer_takes_the_minimum_capacity() {
    let (full, only) = decode_both(&one_row_frame(), BLOCK_ROWS);
    assert_eq!(full, (Ok(1), 4), "decode_frame, the 38-byte frame");
    assert_eq!(only, (Ok(1), 4), "decode_stamps, the 38-byte frame");
    for rows in [1_u16, 2, 3, 4, 95, 96, 97, 171, 172, 255, 256] {
        let m = usize::from(rows);
        let (full, only) = decode_both(&frame_of(rows), BLOCK_ROWS);
        let capacity = m.max(4);
        assert_eq!(full, (Ok(m), capacity), "decode_frame, {m} rows");
        assert_eq!(only, (Ok(m), capacity), "decode_stamps, {m} rows");
        assert_eq!(capacity, amortised(0, 0, m));
    }
}

/// **A FULL BUFFER GROWS BY AMORTISED DOUBLING, NOT BY THE FRAME'S ROWS.**
/// g-sc1-codec-3, D-5902, SC1-09.
///
/// Until D-5902 SC1-09, L-SC-1, D-4939 #5 and the decoders' own docs said an
/// accepted frame grows the caller's buffer by its checked rows and no more.
/// `Vec::reserve` promises at least the rows: onto a full 10,000-row buffer a
/// 256-row frame leaves capacity 20,000, which written against the old text
/// fails by 9,744 rows. The bound that holds is the capacity `amortised`
/// states. The cases: a full buffer, a small full one (where the length plus
/// the rows is the larger), exactly the room, one row short of it, an empty
/// buffer at capacity 0, and a CLEARED buffer (length 0, capacity kept) short
/// of the room, which doubles like any other; then frames of 1 and 3 rows
/// onto buffers of capacity 1 to 3, where `Vec`'s minimum of 4 is the
/// largest term; through both decoders (stamps are 8 bytes).
#[test]
fn a_full_buffer_grows_by_amortised_doubling_not_by_rows() {
    let frame = full_frame();
    let rows = usize::from(BLOCK_ROWS);
    for (len, capacity, grown) in [
        (10_000, 10_000, 20_000),
        (100, 100, 356),
        (10_000, 10_256, 10_256),
        (10_000, 10_255, 20_510),
        (0, 0, 256),
        (0, 200, 400),
        (0, 100, 256),
        (0, 256, 256),
    ] {
        assert_eq!(amortised(len, capacity, rows), grown, "{len} of {capacity}");
        let (full, only) = decode_onto(&frame, len, capacity);
        let want = ((Ok(rows), grown), len + rows);
        assert_eq!(full, want, "decode_frame onto {len} of {capacity}");
        assert_eq!(only, want, "decode_stamps onto {len} of {capacity}");
        assert!(grown <= (2 * capacity).max(len + rows).max(4));
    }
    // Small buffers: the minimum capacity of 4 is a term of the rule, not
    // only of an empty buffer's first call.
    for (m, len, capacity, grown) in [(1_u16, 1, 1, 4), (1, 3, 3, 6), (3, 0, 2, 4), (3, 1, 1, 4)] {
        let frame = frame_of(m);
        let m = usize::from(m);
        assert_eq!(
            amortised(len, capacity, m),
            grown,
            "{m} onto {len} of {capacity}"
        );
        let (full, only) = decode_onto(&frame, len, capacity);
        let want = ((Ok(m), grown), len + m);
        assert_eq!(full, want, "decode_frame, {m} onto {len} of {capacity}");
        assert_eq!(only, want, "decode_stamps, {m} onto {len} of {capacity}");
    }
    // The old claim, written as a test: the full buffer grows by its rows.
    let ((_, grown), _) = decode_onto(&frame, 10_000, 10_000).0;
    assert_ne!(grown, 10_000 + rows, "a full buffer grows by doubling");
    assert_eq!(grown - (10_000 + rows), 9_744, "rows more than asked");
}

/// The table row of `doc` whose first cell is `id`.
fn row<'a>(doc: &'a str, id: &str) -> &'a str {
    let head = format!("| {id} |");
    let mut rows = doc.lines().filter(|line| line.starts_with(&head));
    let found = rows.next().unwrap_or_else(|| panic!("no row {id}"));
    assert!(rows.next().is_none(), "row {id} is written twice");
    found
}

/// **THE DOCUMENTS STATE THE BOUNDS THE CODEC KEEPS.** g-sc1-codec-3,
/// g-sc1-codec-4, D-5902.
///
/// Both findings were claims, not code: the codec was right and six places
/// said a growth bound `Vec` does not keep, and three said a per-bar bound a
/// short frame does not keep. The two tests above prove the true bounds; this
/// one fails while a document still states the false ones. `docs/02` is not
/// pinned here: its sentence (one reserve of the head's rows) is true and is
/// AF-01's file.
#[test]
fn the_documents_state_the_bounds_the_codec_keeps() {
    const INVARIANTS: &str = include_str!("../../../docs/04-invariants.md");
    const LIMITS: &str = include_str!("../../../docs/06-limits.md");
    const SOURCE: &str = include_str!("../src/sealed_codec.rs");

    let growth = [
        ("SC1-09", row(INVARIANTS, "SC1-09")),
        ("L-SC-1", row(LIMITS, "L-SC-1")),
    ];
    for (id, text) in growth {
        assert!(
            !text.contains("by its checked rows and no more")
                && !text.contains("checked rows times 56 bytes, once"),
            "{id} still says the buffer grows by the rows"
        );
        assert!(text.contains("amortised"), "{id} names Vec's growth");
        assert!(text.contains("20,000"), "{id} gives the full-buffer case");
        // The rule has three terms: `Vec`'s minimum of 4 holds on every
        // growth, not only on an empty buffer's first (`amortised` above).
        assert!(
            text.contains("the largest of twice") && text.contains("and 4"),
            "{id} states all three terms of the growth rule"
        );
        // An "empty" buffer is one that never allocated: a cleared buffer
        // keeps its capacity and doubles (256 rows into capacity 200: 400).
        assert!(
            text.contains("capacity 0") && text.contains("cleared"),
            "{id} tells a never-allocated buffer from a cleared one"
        );
    }
    let bytes = [
        ("SC1-02", row(INVARIANTS, "SC1-02")),
        ("L-SC-12", row(LIMITS, "L-SC-12")),
    ];
    for (id, text) in bytes {
        assert!(
            !text.contains("random fields cost at most 57 bytes a bar")
                && !text.contains("at most 57 bytes a bar, 1 more than raw"),
            "{id} still says 57 bytes a bar for any frame"
        );
        assert!(text.contains("56m + 172"), "{id} states max_frame_len(m)");
        assert!(text.contains("102 bytes"), "{id} gives the one-bar case");
        assert!(
            text.contains("every `m` from 1 to 100"),
            "{id} states the measured crossover the codec's own test pins"
        );
        assert!(text.contains("56n + 172f"), "{id} bounds a whole month");
    }
    for stale in [
        "incompressible bars cost at most 57 bytes each",
        "reserves exactly the rows its checked head states",
        "the most one call reserves is the checked head's rows",
        "allocates nothing beyond the rows it appends",
        "the larger of twice its capacity and its length plus",
    ] {
        assert!(!SOURCE.contains(stale), "sealed_codec.rs: {stale}");
    }
    assert!(
        SOURCE.contains("56m + 172"),
        "the module doc states the bound"
    );
    assert!(
        SOURCE.contains("amortised"),
        "the decoders name Vec's growth"
    );
    assert!(
        SOURCE.contains("the largest of twice") && SOURCE.contains("capacity 0"),
        "the decoders state the three-term rule and what an empty buffer is"
    );
}
