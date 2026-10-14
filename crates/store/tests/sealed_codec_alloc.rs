#![allow(clippy::expect_used, clippy::panic, clippy::indexing_slicing)]
//! Codec 1's decode bound, seen through the caller's buffer (FM-284).
//!
//! A sealed month's frame states its own rows. A head or a stream that lies —
//! bit rot that keeps the checksum, or a writer bug that wrote a checksum over
//! wrong bytes — is refused by name before the decoder reserves anything, and
//! an accepted frame grows the caller's buffer by its checked rows and no more.
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

#[test]
fn an_accepted_block_grows_the_buffer_by_its_rows() {
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
