#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::single_match_else
)]
//! Proofs for codec 1 (D-4939, SC-1), beside the code they pin so that
//! `cargo mutants` over the library's own tests reaches them. Every fixture is
//! invented: no row here is a vendor's, and the prices cannot be mistaken for
//! real ones. The real-month acceptance run is
//! `crates/store/tests/sealed_codec_samples.rs`.

use super::{
    BLOCK_ROWS, CODEC, COLUMNS, Column, EncodeError, Encoded, FLAG_FIRST_OI_NULL,
    FLAG_WHOLE_SECONDS, FrameError, GCD_CEILING, GCD_STEPS, KNOWN_FLAGS, MAX_FRAME_LEN,
    MAX_STREAM_LEN, Part, Plan, SECOND_MICROS, bit_len, decode_frame, decode_stamps, encode_frame,
    gcd, low_u64, mask, max_frame_len, max_stream_len, pack, packed_len, position_bits, put_varint,
    residuals_of, unzigzag, varint_len, zigzag,
};
use crate::crc::crc32c;
use crate::format::{Bar, OI_NULL};

/// 00:00 IST on 1 August 2031, an invented month: every fixture sits after it.
const ORIGIN: i64 = 1_943_382_600_000_000;

/// splitmix64: deterministic, no dependency, every run the same cases.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn i64(&mut self) -> i64 {
        self.next().cast_signed()
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn signed(&mut self, magnitude: i64) -> i64 {
        (self.below(2 * magnitude.cast_unsigned() + 1)).cast_signed() - magnitude
    }
}

fn bar(ts: i64, open: i64, high: i64, low: i64, close: i64, volume: i64, oi: i64) -> Bar {
    Bar {
        ts_micros: ts,
        open,
        high,
        low,
        close,
        volume,
        open_interest: oi,
    }
}

/// Encodes `rows` at the default geometry and returns the frame and its entry.
fn sealed(rows: &[Bar]) -> (Vec<u8>, Encoded) {
    let mut frame = Vec::new();
    let encoded = encode_frame(rows, ORIGIN, BLOCK_ROWS, &mut frame).expect("encodes");
    assert_eq!(encoded.len, frame.len());
    assert_eq!(encoded.crc, crc32c(&frame));
    (frame, encoded)
}

/// Encodes, decodes both ways, and proves the rows come back bit for bit.
fn round_trip(rows: &[Bar]) -> Vec<u8> {
    let (frame, encoded) = sealed(rows);
    assert!(
        frame.len() <= max_frame_len(BLOCK_ROWS),
        "{} bytes",
        frame.len()
    );
    let mut back = Vec::new();
    let n = decode_frame(&frame, encoded.crc, ORIGIN, BLOCK_ROWS, &mut back).expect("decodes");
    assert_eq!(n, rows.len());
    assert_eq!(back, rows);
    let mut stamps = Vec::new();
    let n = decode_stamps(&frame, encoded.crc, ORIGIN, BLOCK_ROWS, &mut stamps).expect("stamps");
    assert_eq!(n, rows.len());
    let expected: Vec<i64> = rows.iter().map(|b| b.ts_micros).collect();
    assert_eq!(stamps, expected);
    frame
}

/// A one-second session of invented prices: a random walk near 500 rupees.
fn walk(rng: &mut Rng, rows: usize, oi: bool) -> Vec<Bar> {
    let mut close = 50_000_i64;
    let mut open_interest = 1_200_i64 * 75;
    (0..rows)
        .map(|i| {
            let open = close + rng.signed(5) * 5;
            close = open + rng.signed(20) * 5;
            let high = open.max(close) + rng.signed(3).abs() * 5;
            let low = open.min(close) - rng.signed(3).abs() * 5;
            if oi && rng.below(4) == 0 {
                open_interest += rng.signed(10) * 75;
            }
            bar(
                ORIGIN + 33_300_000_000 + i as i64 * SECOND_MICROS,
                open,
                high,
                low,
                close,
                rng.below(400).cast_signed() * 75,
                if oi { open_interest } else { OI_NULL },
            )
        })
        .collect()
}

/// Rows whose every field is any `i64` at all.
fn noise(rng: &mut Rng, rows: usize) -> Vec<Bar> {
    (0..rows)
        .map(|_| {
            bar(
                rng.i64(),
                rng.i64(),
                rng.i64(),
                rng.i64(),
                rng.i64(),
                rng.i64(),
                rng.i64(),
            )
        })
        .collect()
}

/// The values that break arithmetic, in every field.
const EDGES: [i64; 9] = [
    0,
    1,
    -1,
    i64::MAX,
    i64::MIN,
    i64::MIN + 1,
    i64::MAX - 1,
    1 << 62,
    -(1 << 62),
];

fn edges(rng: &mut Rng, rows: usize) -> Vec<Bar> {
    let mut pick = || EDGES[rng.below(EDGES.len() as u64) as usize];
    (0..rows)
        .map(|_| bar(pick(), pick(), pick(), pick(), pick(), pick(), pick()))
        .collect()
}

// ---------------------------------------------------------------------------
// The shape of a frame, pinned byte for byte.
// ---------------------------------------------------------------------------

#[test]
fn one_row_is_its_varints_and_seven_empty_streams() {
    let one = bar(60_000_000, 10, 12, 9, 11, 0, OI_NULL);
    let mut frame = Vec::new();
    let encoded = encode_frame(&[one], 0, BLOCK_ROWS, &mut frame).expect("encodes");
    let mut expected = vec![CODEC, FLAG_WHOLE_SECONDS | FLAG_FIRST_OI_NULL, 1, 0];
    // (ts - origin) / 1 s = 60, close 11, open - close -1, high - close 1, low - close -2, volume 0.
    expected.extend([120, 22, 1, 2, 3, 0]);
    for _ in 0..COLUMNS {
        // mode 0, width 0; no exceptions; base 0; gcd 1.
        expected.extend([0, 0, 0, 1]);
    }
    assert_eq!(frame, expected);
    assert_eq!(
        encoded,
        Encoded {
            len: 38,
            crc: crc32c(&expected)
        }
    );

    // With an open interest and a stamp off the second, both flags clear.
    let two = bar(1, 10, 12, 9, 11, 0, 5);
    let mut frame = Vec::new();
    encode_frame(&[two], 0, BLOCK_ROWS, &mut frame).expect("encodes");
    assert_eq!(&frame[..11], &[CODEC, 0, 1, 0, 2, 22, 1, 2, 3, 0, 10]);
    assert_eq!(frame.len(), 11 + 4 * COLUMNS);
    round_trip(&[one]);
}

#[test]
fn a_three_row_frame_is_exactly_the_bytes_the_specification_computes() {
    let rows = [
        bar(0, 100, 100, 100, 100, 5, OI_NULL),
        bar(1_000_000, 100, 102, 99, 101, 7, OI_NULL),
        bar(2_000_000, 101, 103, 101, 103, 7, OI_NULL),
    ];
    let mut frame = Vec::new();
    encode_frame(&rows, 0, BLOCK_ROWS, &mut frame).expect("encodes");
    let expected: Vec<u8> = [
        &[CODEC, FLAG_WHOLE_SECONDS | FLAG_FIRST_OI_NULL, 3, 0][..],
        // Row 0: 0 s, close 100 (zigzag 200 = c8 01), open, high and low at the close, volume 5.
        &[0, 0xC8, 0x01, 0, 0, 0, 10],
        // ts: two residuals of 1 s: gcd 1,000,000 (c0 84 3d), base 2, width 0.
        &[0, 0, 2, 0xC0, 0x84, 0x3D],
        // close: +1, +2: zigzag 2, 4; base 2; width 2 holds 0 and 2.
        &[2, 0, 2, 1, 0b0000_1000],
        // open: mode 0 (the previous close) predicts both exactly.
        &[0, 0, 0, 1],
        // high: max(open, close) + 1, + 0: zigzag 2, 0, width 2.
        &[2, 0, 0, 1, 0b0000_0010],
        // low: min(open, close) - 1, + 0: zigzag 1, 0, width 1.
        &[1, 0, 0, 1, 0b0000_0001],
        // volume: mode 0 (zero): 7, 7 is gcd 7, base 2, width 0.
        &[0, 0, 2, 7],
        // open interest: mode 0 (the previous row's): null to null is 0.
        &[0, 0, 0, 1],
    ]
    .concat();
    assert_eq!(frame, expected);
    let mut back = Vec::new();
    decode_frame(&frame, crc32c(&frame), 0, BLOCK_ROWS, &mut back).expect("decodes");
    assert_eq!(back, rows);
}

#[test]
fn an_exception_is_written_where_it_is_shorter_than_plain_packing() {
    // 255 closes that move by 5 paisa and one that jumps by 2^40: one
    // exception at full width, the rest at 1 bit after the gcd of 5.
    let mut rows: Vec<Bar> = (0..256)
        .map(|i| bar(ORIGIN + i * SECOND_MICROS, 0, 0, 0, 0, 0, 0))
        .collect();
    let mut close = 0;
    for (i, row) in rows.iter_mut().enumerate().skip(1) {
        close += if i == 100 { 5 << 40 } else { 5 };
        row.close = close;
        row.open = close;
        row.high = close;
        row.low = close;
    }
    let frame = round_trip(&rows);
    // The close stream sits after the head, row 0 (4 + 7 + 3 bytes) and the ts stream.
    let ts_len = 3 + 3; // width 0, n_exc 0, base 2, gcd 1,000,000 in three bytes
    let close = &frame[4 + 7 + ts_len..];
    assert_eq!(
        close[0], 0,
        "mode 0, width 0: every ordinary residual is the base"
    );
    assert_eq!(close[1], 1, "one exception");
    assert_eq!(
        close[2], 41,
        "the exception's width: zigzag(2^40) - 2 needs 41 bits"
    );
}

// ---------------------------------------------------------------------------
// Round trips: the cases the design names, then properties.
// ---------------------------------------------------------------------------

#[test]
fn a_sparse_month_round_trips() {
    // One bar a day for 31 days, at 09:15 IST: one frame of 31 rows.
    let rows: Vec<Bar> = (0..31)
        .map(|d| {
            let close = 1_000_000 + d * 7;
            bar(
                ORIGIN + d * 86_400 * SECOND_MICROS + 33_300 * SECOND_MICROS,
                close - 3,
                close + 9,
                close - 11,
                close,
                d * 1_000,
                OI_NULL,
            )
        })
        .collect();
    let frame = round_trip(&rows);
    assert!(
        frame.len() < 31 * 8,
        "{} bytes for 31 daily bars",
        frame.len()
    );
}

#[test]
fn an_all_null_column_costs_no_bits() {
    let rows = walk(&mut Rng(7), 256, false);
    let frame = round_trip(&rows);
    // The last stream is open interest: mode 0, width 0, no exceptions, base 0, gcd 1.
    assert_eq!(&frame[frame.len() - 4..], &[0, 0, 0, 1]);
}

#[test]
fn null_open_interest_inside_a_block_round_trips() {
    let mut rows = walk(&mut Rng(8), 256, true);
    for i in [0, 1, 17, 128, 254, 255] {
        rows[i].open_interest = OI_NULL;
    }
    round_trip(&rows);
    // Null first, then real: flag bit 1 set and the rest are residuals.
    let mut rows = walk(&mut Rng(9), 40, true);
    rows[0].open_interest = OI_NULL;
    let frame = round_trip(&rows);
    assert_eq!(frame[1] & FLAG_FIRST_OI_NULL, FLAG_FIRST_OI_NULL);
}

#[test]
fn a_jump_across_the_whole_range_is_an_exception() {
    let mut rows = walk(&mut Rng(10), 256, true);
    for (i, value) in [
        (50, i64::MAX),
        (51, i64::MIN + 1),
        (52, 0),
        (53, i64::MIN),
        (54, i64::MAX),
    ] {
        rows[i].close = value;
        rows[i].open = value;
        rows[i].high = value;
        rows[i].low = value;
        rows[i].volume = value;
        rows[i].open_interest = value;
    }
    round_trip(&rows);
}

#[test]
fn any_rows_round_trip_bit_for_bit() {
    let mut rng = Rng(0x5EA1);
    for case in 0..400 {
        let m = match case % 8 {
            0 => 1,
            1 => 2,
            2 => 255,
            3 => 256,
            _ => 1 + rng.below(256) as usize,
        };
        let rows = match case % 5 {
            0 => noise(&mut rng, m),
            1 => edges(&mut rng, m),
            2 => walk(&mut rng, m, true),
            3 => walk(&mut rng, m, false),
            _ => {
                // Stamps off the second, equal and out of order; high below open.
                let mut rows = walk(&mut rng, m, true);
                for row in &mut rows {
                    row.ts_micros += rng.signed(1_500_000);
                    row.high = row.open - rng.below(100).cast_signed();
                    row.volume = -row.volume;
                }
                rows
            }
        };
        round_trip(&rows);
    }
}

#[test]
fn no_frame_exceeds_the_bound() {
    assert_eq!(max_frame_len(BLOCK_ROWS), MAX_FRAME_LEN);
    assert_eq!(max_stream_len(BLOCK_ROWS), MAX_STREAM_LEN);
    assert_eq!(MAX_FRAME_LEN, 4 + 70 + 7 * 2_062);
    let mut rng = Rng(0xB0B);
    let mut largest = 0;
    for _ in 0..200 {
        let (frame, _) = sealed(&noise(&mut rng, 256));
        largest = largest.max(frame.len());
        let (frame, _) = sealed(&edges(&mut rng, 256));
        largest = largest.max(frame.len());
    }
    assert!(largest <= MAX_FRAME_LEN, "{largest}");
    // Random fields are incompressible: at most 57 bytes a bar (L-SC-12), and
    // never far below the 56 raw bytes.
    assert!(largest <= 57 * 256, "{largest}");
    assert!(largest > 50 * 256, "{largest}");
}

#[test]
fn stamps_off_the_second_equal_stamps_and_a_negative_offset_round_trip() {
    let base = walk(&mut Rng(11), 5, true);
    for first in [
        ORIGIN,
        ORIGIN + 1,
        ORIGIN - SECOND_MICROS,
        ORIGIN - 1,
        ORIGIN + 999_999,
        i64::MIN,
        i64::MAX,
    ] {
        let mut rows = base.clone();
        rows[0].ts_micros = first;
        rows[1].ts_micros = first;
        let frame = round_trip(&rows);
        let whole = first.wrapping_sub(ORIGIN) % SECOND_MICROS == 0;
        assert_eq!(frame[1] & FLAG_WHOLE_SECONDS != 0, whole, "{first}");
    }
}

#[test]
fn the_encoder_is_deterministic() {
    let rows = walk(&mut Rng(12), 256, true);
    let (a, _) = sealed(&rows);
    let (b, _) = sealed(&rows);
    assert_eq!(a, b);
}

#[test]
fn a_frame_appends_and_a_refusal_leaves_the_buffer_as_it_was() {
    let rows = walk(&mut Rng(13), 100, true);
    let mut out = vec![7u8; 3];
    let encoded = encode_frame(&rows, ORIGIN, BLOCK_ROWS, &mut out).expect("encodes");
    assert_eq!(&out[..3], &[7, 7, 7]);
    let frame = out[3..].to_vec();
    assert_eq!(
        encoded,
        Encoded {
            len: frame.len(),
            crc: crc32c(&frame)
        }
    );

    let held = rows[..2].to_vec();
    let mut bars = held.clone();
    decode_frame(&frame, encoded.crc, ORIGIN, BLOCK_ROWS, &mut bars).expect("decodes");
    assert_eq!(&bars[..2], &held[..]);
    assert_eq!(&bars[2..], &rows[..]);

    // A refusal found only after every stream was decoded still appends nothing.
    let mut long = frame.clone();
    long.push(0);
    let mut bars = held.clone();
    let refused = decode_frame(&long, crc32c(&long), ORIGIN, BLOCK_ROWS, &mut bars);
    assert_eq!(refused, Err(FrameError::Trailing { bytes: 1 }));
    assert_eq!(bars, held);
    let mut stamps = vec![1, 2];
    let refused = decode_stamps(&long, crc32c(&long), ORIGIN, BLOCK_ROWS, &mut stamps);
    assert_eq!(refused, Err(FrameError::Trailing { bytes: 1 }));
    assert_eq!(stamps, [1, 2]);
}

#[test]
fn a_frame_holds_from_one_row_to_block_rows_and_no_more() {
    let rows = walk(&mut Rng(14), 257, true);
    let mut out = Vec::new();
    assert_eq!(
        encode_frame(&[], ORIGIN, BLOCK_ROWS, &mut out),
        Err(EncodeError::Empty)
    );
    assert_eq!(
        encode_frame(&rows, ORIGIN, BLOCK_ROWS, &mut out),
        Err(EncodeError::TooManyRows {
            rows: 257,
            block_rows: BLOCK_ROWS
        })
    );
    assert!(out.is_empty(), "a refused encode appends nothing");
    round_trip(&rows[..256]);

    // Another geometry, as a head may state it: one row a frame, and the largest.
    let mut out = Vec::new();
    let one = encode_frame(&rows[..1], ORIGIN, 1, &mut out).expect("one row");
    let mut back = Vec::new();
    decode_frame(&out, one.crc, ORIGIN, 1, &mut back).expect("decodes at 1");
    assert_eq!(back, rows[..1]);
    assert_eq!(
        encode_frame(&rows[..2], ORIGIN, 1, &mut Vec::new()),
        Err(EncodeError::TooManyRows {
            rows: 2,
            block_rows: 1
        })
    );
    let big: Vec<Bar> = walk(&mut Rng(15), usize::from(u16::MAX) + 1, true);
    assert_eq!(
        encode_frame(&big, ORIGIN, u16::MAX, &mut Vec::new()),
        Err(EncodeError::TooManyRows {
            rows: 65_536,
            block_rows: u16::MAX
        })
    );
    let mut frame = Vec::new();
    let encoded = encode_frame(&big[..65_535], ORIGIN, u16::MAX, &mut frame).expect("largest");
    assert!(frame.len() <= max_frame_len(u16::MAX));
    let mut back = Vec::new();
    decode_frame(&frame, encoded.crc, ORIGIN, u16::MAX, &mut back).expect("decodes largest");
    assert_eq!(back, big[..65_535]);
    // A head that says fewer rows than the frame holds refuses it.
    assert_eq!(
        decode_frame(&frame, encoded.crc, ORIGIN, u16::MAX - 1, &mut Vec::new()),
        Err(FrameError::Rows {
            rows: u16::MAX,
            block_rows: u16::MAX - 1
        })
    );
}

#[test]
fn a_zero_block_rows_refuses_every_frame_and_every_encode() {
    // A head that states 0 rows a frame is garbage: nothing fits it.
    let rows = walk(&mut Rng(22), 3, true);
    assert_eq!(
        encode_frame(&rows[..1], ORIGIN, 0, &mut Vec::new()),
        Err(EncodeError::TooManyRows {
            rows: 1,
            block_rows: 0
        })
    );
    let (frame, encoded) = sealed(&rows);
    assert_eq!(
        decode_frame(&frame, encoded.crc, ORIGIN, 0, &mut Vec::new()),
        Err(FrameError::Rows {
            rows: 3,
            block_rows: 0
        })
    );
    assert_eq!(
        decode_stamps(&frame, encoded.crc, ORIGIN, 0, &mut Vec::new()),
        Err(FrameError::Rows {
            rows: 3,
            block_rows: 0
        })
    );
    assert_eq!(max_frame_len(0), max_frame_len(1));
}

#[test]
fn frames_decode_the_same_on_many_threads_at_once() {
    // The codec holds no state between calls: concurrent readers of one
    // frame each get the whole frame, and nothing one does reaches another.
    let mut rng = Rng(23);
    let months: Vec<Vec<Bar>> = (0..16)
        .map(|i| walk(&mut rng, 1 + i * 16, i % 2 == 0))
        .collect();
    let frames: Vec<(Vec<u8>, Encoded)> = months.iter().map(|rows| sealed(rows)).collect();
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                for _ in 0..50 {
                    for (rows, (frame, encoded)) in months.iter().zip(&frames) {
                        let mut out = Vec::new();
                        decode_frame(frame, encoded.crc, ORIGIN, BLOCK_ROWS, &mut out)
                            .expect("decodes");
                        assert_eq!(&out, rows);
                        let mut again = Vec::new();
                        encode_frame(rows, ORIGIN, BLOCK_ROWS, &mut again).expect("encodes");
                        assert_eq!(&again, frame);
                    }
                }
            });
        }
    });
}

// ---------------------------------------------------------------------------
// The encoder's choice: the shortest stream, ties to mode 0 and the smaller width.
// ---------------------------------------------------------------------------

/// Every (mode, width) the encoder could have written, by writing each.
fn every_choice(rows: &[Bar], column: Column) -> Vec<(usize, u8, u32)> {
    let mut lens = Vec::new();
    let mut residuals = Vec::new();
    for mode in 0..if column.has_mode_one() { 2 } else { 1 } {
        residuals_of(rows, column, mode, &mut residuals);
        let planned = Plan::of(&residuals, mode);
        for width in 0..=planned.top {
            let limit = mask(width);
            let exceptions = residuals
                .iter()
                .filter(|&&r| zigzag(r / planned.gcd.cast_signed()) - planned.base > limit)
                .count();
            let forced = Plan {
                width,
                exceptions,
                ..planned
            };
            let mut out = Vec::new();
            forced.emit(&residuals, &mut out);
            lens.push((out.len(), mode, width));
        }
    }
    lens
}

#[test]
fn the_encoder_writes_the_shortest_stream_and_breaks_ties_the_stated_way() {
    let mut rng = Rng(0xC0DE);
    let mut ties = 0;
    let mut mode_one = [0; COLUMNS];
    for case in 0..300 {
        let m = 2 + rng.below(40) as usize;
        let rows = match case % 3 {
            0 => walk(&mut rng, m, true),
            1 => {
                // Small values, so different widths often cost the same bytes.
                (0..m)
                    .map(|_| {
                        let s = |r: &mut Rng| r.below(8).cast_signed();
                        bar(
                            s(&mut rng),
                            s(&mut rng),
                            s(&mut rng),
                            s(&mut rng),
                            s(&mut rng),
                            s(&mut rng),
                            s(&mut rng),
                        )
                    })
                    .collect()
            }
            _ => edges(&mut rng, m),
        };
        let mut residuals = Vec::new();
        for (c, column) in Column::ORDER.into_iter().enumerate() {
            let choices = every_choice(&rows, column);
            let best = *choices.iter().min().expect("one choice at least");
            ties += choices.iter().filter(|c| c.0 == best.0).count() - 1;
            residuals_of(&rows, column, 0, &mut residuals);
            let zero = Plan::of(&residuals, 0);
            let chosen = if column.has_mode_one() {
                residuals_of(&rows, column, 1, &mut residuals);
                let one = Plan::of(&residuals, 1);
                if one.len < zero.len { one } else { zero }
            } else {
                zero
            };
            assert_eq!(
                (chosen.len, chosen.mode, chosen.width),
                best,
                "{column:?} case {case}"
            );
            mode_one[c] += usize::from(chosen.mode);
        }
    }
    assert!(ties > 100, "the data must exercise ties: {ties}");
    // Each mode-1 column chose mode 1 somewhere, and no other column ever did.
    assert_eq!(
        mode_one.iter().map(|&n| n > 0).collect::<Vec<_>>(),
        [false, false, true, false, false, true, true]
    );
}

#[test]
fn each_column_decodes_through_both_of_its_modes() {
    // open = close on every row with a jumping close: open mode 1 (this close).
    // volume rising by one: volume mode 1 (the previous volume).
    // open interest of either sign near a million: mode 1 (zero) needs about
    // 11 bits where the previous row's needs 22.
    let mut rng = Rng(16);
    let rows: Vec<Bar> = (0..256)
        .map(|i| {
            let close = 1_000_000 + rng.signed(400_000);
            let near = 1_000_000 + rng.below(1_000).cast_signed();
            let oi = if rng.below(2) == 0 { near } else { -near };
            bar(
                ORIGIN + i * SECOND_MICROS,
                close,
                close + 1,
                close - 1,
                close,
                9_000_000 + i,
                oi,
            )
        })
        .collect();
    let frame = round_trip(&rows);
    let mut mode = [0u8; COLUMNS];
    let mut residuals = Vec::new();
    for (c, column) in Column::ORDER.into_iter().enumerate() {
        residuals_of(&rows, column, 0, &mut residuals);
        let zero = Plan::of(&residuals, 0);
        if column.has_mode_one() {
            residuals_of(&rows, column, 1, &mut residuals);
            mode[c] = u8::from(Plan::of(&residuals, 1).len < zero.len);
        }
    }
    assert_eq!(mode, [0, 0, 1, 0, 0, 1, 1]);
    assert!(frame.len() < 256 * 12, "{}", frame.len());
    // And the opposite data takes mode 0 in all three.
    round_trip(&walk(&mut Rng(17), 256, true));
}

#[test]
fn a_tie_between_modes_goes_to_mode_zero() {
    // open = close = a constant: both open predictors are exact.
    let rows: Vec<Bar> = (0..10)
        .map(|i| bar(ORIGIN + i * SECOND_MICROS, 500, 500, 500, 500, 0, 0))
        .collect();
    let frame = round_trip(&rows);
    // Head 4, row 0 (0, 1000 as 2 bytes, 0, 0, 0, 0, 0), ts stream 6, close stream 4: then open.
    let open = 4 + 8 + 6 + 4;
    assert_eq!(frame[open], 0, "open: mode 0 on a tie");
    assert_eq!(
        &frame[frame.len() - 8..],
        &[0, 0, 0, 1, 0, 0, 0, 1],
        "volume and open interest: mode 0"
    );
}

// ---------------------------------------------------------------------------
// Refusals: one malformed frame a refusal, never a panic, never rows.
// ---------------------------------------------------------------------------

/// A frame assembled by hand, so each refusal is reached on purpose.
fn assemble(head: [u8; 4], first: &[u8], streams: &[&[u8]]) -> Vec<u8> {
    let mut frame = head.to_vec();
    frame.extend_from_slice(first);
    for stream in streams {
        frame.extend_from_slice(stream);
    }
    frame
}

/// Row 0 with every varint 0 and the null-OI flag, for hand-made frames.
const FIRST: [u8; 6] = [0; 6];
const NULL_OI: u8 = FLAG_FIRST_OI_NULL;
const EMPTY: &[u8] = &[0, 0, 0, 1];

fn refuse(frame: &[u8]) -> FrameError {
    let crc = crc32c(frame);
    let mut bars = Vec::new();
    let full = decode_frame(frame, crc, 0, BLOCK_ROWS, &mut bars).expect_err("refused");
    assert!(bars.is_empty());
    let mut stamps = Vec::new();
    let only = decode_stamps(frame, crc, 0, BLOCK_ROWS, &mut stamps).expect_err("refused");
    assert!(stamps.is_empty());
    assert_eq!(full, only, "both decoders refuse a frame the same way");
    full
}

#[test]
fn each_malformed_frame_is_refused_by_name() {
    let seven = [EMPTY; 7];
    let good = assemble([CODEC, NULL_OI, 4, 0], &FIRST, &seven);
    let mut bars = Vec::new();
    assert_eq!(
        decode_frame(&good, crc32c(&good), 0, BLOCK_ROWS, &mut bars),
        Ok(4)
    );
    assert_eq!(bars, [bar(0, 0, 0, 0, 0, 0, OI_NULL); 4]);

    // The checksum, before anything else: even a frame of garbage.
    let crc = crc32c(&good);
    assert_eq!(
        decode_frame(&good, crc ^ 1, 0, BLOCK_ROWS, &mut Vec::new()),
        Err(FrameError::Checksum {
            expected: crc ^ 1,
            found: crc
        })
    );
    assert_eq!(
        decode_stamps(&[9, 9], 0, 0, BLOCK_ROWS, &mut Vec::new()),
        Err(FrameError::Checksum {
            expected: 0,
            found: crc32c(&[9, 9])
        })
    );

    let head = Part::Head;
    assert_eq!(refuse(&[]), FrameError::Truncated { part: head });
    assert_eq!(refuse(&[CODEC]), FrameError::Truncated { part: head });
    assert_eq!(refuse(&[CODEC, 0]), FrameError::Truncated { part: head });
    assert_eq!(refuse(&[CODEC, 0, 1]), FrameError::Truncated { part: head });
    assert_eq!(refuse(&[2, 0, 1, 0]), FrameError::Codec { found: 2 });
    assert_eq!(refuse(&[0, 0, 1, 0]), FrameError::Codec { found: 0 });
    for flag in [0b100, 0b1000_0000, 0b111] {
        assert_eq!(
            refuse(&[CODEC, flag, 1, 0]),
            FrameError::Flags { found: flag }
        );
    }
    assert_eq!(
        refuse(&[CODEC, 0, 0, 0]),
        FrameError::Rows {
            rows: 0,
            block_rows: BLOCK_ROWS
        }
    );
    assert_eq!(
        refuse(&[CODEC, 0, 1, 1]),
        FrameError::Rows {
            rows: 257,
            block_rows: BLOCK_ROWS
        }
    );
    let full = assemble([CODEC, NULL_OI, 0, 1], &FIRST, &[EMPTY]);
    assert_ne!(
        refuse(&full),
        FrameError::Rows {
            rows: 256,
            block_rows: BLOCK_ROWS
        },
        "256 rows is a frame"
    );

    // Row 0.
    let first = Part::FirstRow;
    assert_eq!(
        refuse(&[CODEC, 0, 1, 0]),
        FrameError::Truncated { part: first }
    );
    assert_eq!(
        refuse(&[CODEC, 0, 1, 0, 0, 0, 0, 0, 0, 0]),
        FrameError::Truncated { part: first },
        "the open interest varint is missing"
    );
    assert_eq!(
        refuse(&[CODEC, 0, 1, 0, 0x80]),
        FrameError::Truncated { part: first }
    );
    let long = [
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x80, 0x00,
    ];
    assert_eq!(
        refuse(&assemble([CODEC, NULL_OI, 1, 0], &long, &[])),
        FrameError::VarintTooLong { part: first }
    );
    let wide = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02];
    assert_eq!(
        refuse(&assemble([CODEC, NULL_OI, 1, 0], &wide, &[])),
        FrameError::VarintTooLong { part: first }
    );
    // The widest legal varint, u64::MAX, is not refused: it is zigzag(i64::MIN).
    let widest = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
    let frame = assemble(
        [CODEC, NULL_OI | FLAG_WHOLE_SECONDS, 1, 0],
        &[&widest[..], &[0; 5]].concat(),
        &seven,
    );
    let mut bars = Vec::new();
    decode_frame(&frame, crc32c(&frame), 0, BLOCK_ROWS, &mut bars).expect("u64::MAX is a varint");
    assert_eq!(bars[0].ts_micros, i64::MIN.wrapping_mul(SECOND_MICROS));

    // Streams, by column.
    let ts = Part::Stream(Column::Stamp);
    let at = |streams: &[&[u8]]| assemble([CODEC, NULL_OI, 4, 0], &FIRST, streams);
    assert_eq!(refuse(&at(&[])), FrameError::Truncated { part: ts });
    assert_eq!(refuse(&at(&[&[0]])), FrameError::Truncated { part: ts });
    assert_eq!(refuse(&at(&[&[0, 0]])), FrameError::Truncated { part: ts });
    assert_eq!(
        refuse(&at(&[&[0, 0, 0]])),
        FrameError::Truncated { part: ts }
    );
    assert_eq!(
        refuse(&at(&[&[65, 0, 0, 1]])),
        FrameError::Width {
            column: Column::Stamp,
            width: 65
        }
    );
    assert_eq!(
        refuse(&at(&[&[0x7F, 0, 0, 1]])),
        FrameError::Width {
            column: Column::Stamp,
            width: 127
        }
    );
    // Width 64 is legal: three residuals of 8 bytes.
    let w64: &[u8] = &[[64u8, 0, 0, 1].as_slice(), &[0; 24]].concat();
    assert_eq!(
        decode_frame(
            &at(&[w64, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY]),
            crc32c(&at(&[w64, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY])),
            0,
            BLOCK_ROWS,
            &mut Vec::new()
        ),
        Ok(4)
    );
    assert_eq!(
        refuse(&at(&[&[64, 0, 0, 1, 0]])),
        FrameError::Truncated { part: ts },
        "width 64 needs 24 bytes"
    );
    // Mode 1 exists for open, volume and open interest only.
    for (i, column) in Column::ORDER.into_iter().enumerate() {
        let mut streams = seven;
        let moded: &[u8] = &[0x80, 0, 0, 1];
        streams[i] = moded;
        if column.has_mode_one() {
            let frame = at(&streams);
            assert_eq!(
                decode_frame(&frame, crc32c(&frame), 0, BLOCK_ROWS, &mut Vec::new()),
                Ok(4),
                "{column:?}"
            );
        } else {
            assert_eq!(refuse(&at(&streams)), FrameError::Mode { column });
        }
    }
    assert_eq!(
        refuse(&at(&[&[0, 4, 8, 0, 1]])),
        FrameError::ExceptionCount {
            column: Column::Stamp,
            count: 4,
            residuals: 3
        }
    );
    let huge = [
        0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
    ];
    assert_eq!(
        refuse(&at(&[&huge])),
        FrameError::ExceptionCount {
            column: Column::Stamp,
            count: u64::MAX,
            residuals: 3
        }
    );
    assert_eq!(
        refuse(&at(&[&[0, 0x80]])),
        FrameError::Truncated { part: ts }
    );
    assert_eq!(
        refuse(&at(&[&[0, 1]])),
        FrameError::Truncated { part: ts },
        "the exception width is missing"
    );
    assert_eq!(
        refuse(&at(&[&[0, 1, 65, 0, 1, 0, 0]])),
        FrameError::ExceptionWidth {
            column: Column::Stamp,
            width: 65
        }
    );
    assert_eq!(
        refuse(&at(&[&[0, 0, 0, 0]])),
        FrameError::Gcd {
            column: Column::Stamp,
            gcd: 0
        }
    );
    let mut ceiling = vec![0, 0, 0];
    put_varint(GCD_CEILING, &mut ceiling);
    assert_eq!(
        refuse(&at(&[&ceiling])),
        FrameError::Gcd {
            column: Column::Stamp,
            gcd: GCD_CEILING
        }
    );
    let mut below = vec![0, 0, 0];
    put_varint(GCD_CEILING - 1, &mut below);
    let frame = at(&[&below, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY]);
    assert_eq!(
        decode_frame(&frame, crc32c(&frame), 0, BLOCK_ROWS, &mut Vec::new()),
        Ok(4),
        "2^62 - 1 is a gcd"
    );
    assert_eq!(
        refuse(&at(&[&[0, 0, 0x80, 1]])),
        FrameError::Truncated { part: ts },
        "a base cut short"
    );
    // Exceptions: three residuals, positions of 2 bits, values of 8 bits.
    let exception = |positions: u8, count: u8| vec![0, count, 8, 0, 1, positions, 0xAA, 0xBB];
    let one = exception(0b10, 1);
    let one = &one[..7];
    let frame = at(&[one, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY]);
    let mut bars = Vec::new();
    assert_eq!(
        decode_frame(&frame, crc32c(&frame), 0, BLOCK_ROWS, &mut bars),
        Ok(4)
    );
    // Residual 2 is row 3: unzigzag(0xAA) = 85, so ts runs 0, 0, 0, 85.
    assert_eq!(
        bars.iter().map(|b| b.ts_micros).collect::<Vec<_>>(),
        [0, 0, 0, 85]
    );
    assert_eq!(
        refuse(&at(&[&exception(0b11, 1)[..7]])),
        FrameError::ExceptionPosition {
            column: Column::Stamp,
            position: 3,
            residuals: 3
        }
    );
    assert_eq!(
        refuse(&at(&[&exception(0b0101, 2)])),
        FrameError::ExceptionOrder {
            column: Column::Stamp,
            position: 1,
            previous: 1
        }
    );
    assert_eq!(
        refuse(&at(&[&exception(0b0110, 2)])),
        FrameError::ExceptionOrder {
            column: Column::Stamp,
            position: 1,
            previous: 2
        }
    );
    assert_eq!(
        refuse(&at(&[&exception(0b10, 1)[..6]])),
        FrameError::Truncated { part: ts },
        "the exception value is missing"
    );
    assert_eq!(
        refuse(&at(&[&exception(0b10, 1)[..5]])),
        FrameError::Truncated { part: ts },
        "the positions are missing"
    );
    // Bits after the last exception position are padding: never read as a
    // position, even when they are not zero (here they spell position 2).
    let padded: &[u8] = &[8, 1, 8, 0, 1, 2, 4, 6, 0b0000_1000, 0x0A];
    let frame = at(&[padded, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY]);
    let mut bars = Vec::new();
    assert_eq!(
        decode_frame(&frame, crc32c(&frame), 0, BLOCK_ROWS, &mut bars),
        Ok(4)
    );
    assert_eq!(
        bars.iter().map(|b| b.ts_micros).collect::<Vec<_>>(),
        [0, 5, 7, 10]
    );
    // An exception may be 64 bits wide.
    let full: &[u8] = &[[0u8, 1, 64, 0, 1, 0b10].as_slice(), &[0xFF; 8]].concat();
    let frame = at(&[full, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY, EMPTY]);
    let mut bars = Vec::new();
    assert_eq!(
        decode_frame(&frame, crc32c(&frame), 0, BLOCK_ROWS, &mut bars),
        Ok(4)
    );
    assert_eq!(
        bars.iter().map(|b| b.ts_micros).collect::<Vec<_>>(),
        [0, 0, 0, i64::MIN]
    );
    // Every later column is named in its own refusal.
    for (i, column) in Column::ORDER.into_iter().enumerate().skip(1) {
        let streams = &seven[..i];
        assert_eq!(
            refuse(&at(streams)),
            FrameError::Truncated {
                part: Part::Stream(column)
            }
        );
    }
    // Bytes after the seventh stream.
    let mut long = good.clone();
    long.extend([0, 0]);
    assert_eq!(refuse(&long), FrameError::Trailing { bytes: 2 });
    // A frame with one row and a stream that claims an exception it cannot have.
    let one_row = assemble([CODEC, NULL_OI, 1, 0], &FIRST, &[&[0, 1, 8, 0, 1, 0, 0]]);
    assert_eq!(
        refuse(&one_row),
        FrameError::ExceptionCount {
            column: Column::Stamp,
            count: 1,
            residuals: 0
        }
    );
}

#[test]
fn any_corruption_with_a_matching_checksum_is_refused_or_decoded_never_a_panic() {
    let mut rng = Rng(18);
    let rows = walk(&mut rng, 40, true);
    let (frame, encoded) = sealed(&rows);
    // Every flipped bit fails the checksum the entry holds.
    for byte in 0..frame.len() {
        for bit in 0..8 {
            let mut bad = frame.clone();
            bad[byte] ^= 1 << bit;
            let mut out = Vec::new();
            assert!(matches!(
                decode_frame(&bad, encoded.crc, ORIGIN, BLOCK_ROWS, &mut out),
                Err(FrameError::Checksum { .. })
            ));
            assert!(out.is_empty());
        }
    }
    // With the checksum recomputed (an encoder bug, a forged entry), every
    // byte value at every offset decodes or is refused; neither panics.
    let mut refused = 0;
    for byte in 0..frame.len() {
        for value in [0x00, 0x01, 0x40, 0x7F, 0x80, 0xFF, rng.below(256) as u8] {
            let mut bad = frame.clone();
            bad[byte] = value;
            let crc = crc32c(&bad);
            let mut out = Vec::new();
            match decode_frame(&bad, crc, ORIGIN, BLOCK_ROWS, &mut out) {
                Ok(n) => assert_eq!(out.len(), n),
                Err(_) => {
                    assert!(out.is_empty());
                    refused += 1;
                }
            }
            let mut stamps = Vec::new();
            match decode_stamps(&bad, crc, ORIGIN, BLOCK_ROWS, &mut stamps) {
                Ok(n) => assert_eq!(stamps.len(), n),
                Err(_) => assert!(stamps.is_empty()),
            }
        }
        // Cut short at every length.
        let cut = &frame[..byte];
        assert!(decode_frame(cut, crc32c(cut), ORIGIN, BLOCK_ROWS, &mut Vec::new()).is_err());
    }
    assert!(refused > frame.len(), "{refused}");
}

#[test]
fn every_refusal_has_its_own_name_and_says_where() {
    let column = Column::Volume;
    let all = [
        FrameError::Checksum {
            expected: 1,
            found: 2,
        },
        FrameError::Codec { found: 2 },
        FrameError::Flags { found: 4 },
        FrameError::Rows {
            rows: 0,
            block_rows: 256,
        },
        FrameError::Truncated { part: Part::Head },
        FrameError::VarintTooLong {
            part: Part::FirstRow,
        },
        FrameError::Width { column, width: 65 },
        FrameError::Mode {
            column: Column::High,
        },
        FrameError::ExceptionCount {
            column,
            count: 9,
            residuals: 3,
        },
        FrameError::ExceptionWidth { column, width: 70 },
        FrameError::Gcd { column, gcd: 0 },
        FrameError::ExceptionPosition {
            column,
            position: 5,
            residuals: 3,
        },
        FrameError::ExceptionOrder {
            column,
            position: 1,
            previous: 2,
        },
        FrameError::Trailing { bytes: 3 },
    ];
    let names: Vec<&str> = all.iter().map(FrameError::reason).collect();
    assert_eq!(
        names,
        [
            "checksum",
            "codec",
            "flags",
            "rows",
            "truncated",
            "varint_too_long",
            "width",
            "mode",
            "exception_count",
            "exception_width",
            "gcd",
            "exception_position",
            "exception_order",
            "trailing_bytes"
        ]
    );
    let said: Vec<String> = all.iter().map(ToString::to_string).collect();
    for (text, name) in said.iter().zip(&names) {
        assert!(text.starts_with(&format!("{name}: ")), "{text}");
    }
    assert_eq!(
        said[0],
        "checksum: the frame hashes to 0x00000002, its entry holds 0x00000001"
    );
    assert_eq!(said[1], "codec: codec 2, this build reads 1");
    assert_eq!(said[2], "flags: flags 0x04 set a bit outside 0x03");
    assert_eq!(
        said[3],
        "rows: the frame states 0 rows, a frame holds 1 to 256"
    );
    assert_eq!(said[4], "truncated: the frame head runs past the frame");
    assert_eq!(
        said[5],
        "varint_too_long: a varint in row 0 runs past 64 bits"
    );
    assert_eq!(
        said[6],
        "width: the volume stream packs 65 bits, at most 64"
    );
    assert_eq!(
        said[7],
        "mode: the high stream states mode 1, which that column does not have"
    );
    assert_eq!(
        said[8],
        "exception_count: the volume stream states 9 exceptions among 3 residuals"
    );
    assert_eq!(
        said[9],
        "exception_width: the volume stream's exceptions are 70 bits, at most 64"
    );
    assert_eq!(
        said[10],
        format!(
            "gcd: the volume stream's gcd is 0, it must be 1 to {}",
            GCD_CEILING - 1
        )
    );
    assert_eq!(
        said[11],
        "exception_position: the volume stream has an exception at 5, past its 3 residuals"
    );
    assert_eq!(
        said[12],
        "exception_order: the volume stream's exception at 1 does not follow the one at 2"
    );
    assert_eq!(
        said[13],
        "trailing_bytes: 3 bytes follow the seventh stream"
    );
    assert_eq!(
        FrameError::Truncated {
            part: Part::Stream(Column::OpenInterest)
        }
        .to_string(),
        "truncated: the open_interest stream runs past the frame"
    );
    let names: Vec<&str> = Column::ORDER.into_iter().map(Column::name).collect();
    assert_eq!(
        names,
        [
            "ts",
            "close",
            "open",
            "high",
            "low",
            "volume",
            "open_interest"
        ]
    );

    assert_eq!(EncodeError::Empty.reason(), "empty");
    assert_eq!(
        EncodeError::Empty.to_string(),
        "empty: a frame holds at least one row"
    );
    let many = EncodeError::TooManyRows {
        rows: 300,
        block_rows: 256,
    };
    assert_eq!(many.reason(), "too_many_rows");
    assert_eq!(
        many.to_string(),
        "too_many_rows: 300 rows offered to a frame of at most 256"
    );
}

// ---------------------------------------------------------------------------
// The arithmetic underneath, pinned at its edges.
// ---------------------------------------------------------------------------

#[test]
fn zigzag_is_a_bijection_at_every_edge() {
    let pairs = [
        (0, 0),
        (-1, 1),
        (1, 2),
        (-2, 3),
        (i64::MAX, u64::MAX - 1),
        (i64::MIN, u64::MAX),
    ];
    for (signed, unsigned) in pairs {
        assert_eq!(zigzag(signed), unsigned);
        assert_eq!(unzigzag(unsigned), signed);
    }
    let mut rng = Rng(19);
    for _ in 0..10_000 {
        let x = rng.i64();
        assert_eq!(unzigzag(zigzag(x)), x);
        let u = rng.next();
        assert_eq!(zigzag(unzigzag(u)), u);
    }
}

#[test]
fn a_varint_is_as_long_as_it_says() {
    for (value, len) in [
        (0, 1),
        (1, 1),
        (127, 1),
        (128, 2),
        (16_383, 2),
        (16_384, 3),
        (u64::MAX >> 1, 9),
        (u64::MAX, 10),
    ] {
        let mut out = Vec::new();
        put_varint(value, &mut out);
        assert_eq!((out.len(), varint_len(value)), (len, len), "{value}");
    }
    let mut out = Vec::new();
    put_varint(300, &mut out);
    assert_eq!(out, [0xAC, 0x02]);
    let mut rng = Rng(20);
    for _ in 0..2_000 {
        let value = rng.next() >> rng.below(64);
        let mut out = Vec::new();
        put_varint(value, &mut out);
        assert_eq!(out.len(), varint_len(value));
    }
}

#[test]
fn bits_and_widths_are_exact_at_their_edges() {
    assert_eq!(
        [
            bit_len(0),
            bit_len(1),
            bit_len(2),
            bit_len(255),
            bit_len(256),
            bit_len(u64::MAX)
        ],
        [0, 1, 2, 8, 9, 64]
    );
    assert_eq!(
        [mask(0), mask(1), mask(7), mask(63), mask(64)],
        [0, 1, 127, u64::MAX >> 1, u64::MAX]
    );
    assert_eq!(
        [0, 1, 2, 3, 4, 5, 255, 256, 257].map(position_bits),
        [1, 1, 1, 2, 2, 3, 8, 8, 9],
        "max(1, bit length of residuals - 1)"
    );
    assert_eq!(
        [
            packed_len(0, 64),
            packed_len(1, 1),
            packed_len(8, 1),
            packed_len(9, 1),
            packed_len(255, 64),
            packed_len(3, 3)
        ],
        [0, 1, 1, 2, 2_040, 2]
    );
    assert_eq!(
        [
            gcd(0, 0),
            gcd(0, 6),
            gcd(12, 18),
            gcd(7, 13),
            gcd(1 << 62, 1 << 61)
        ],
        [0, 6, 6, 1, 1 << 61]
    );
    assert_eq!(low_u64(u128::MAX), u64::MAX);
    assert_eq!(low_u64((7u128 << 64) | 9), 9);
    assert_eq!(
        [max_stream_len(0), max_stream_len(1), max_stream_len(2)],
        [22, 22, 30]
    );
    assert_eq!(max_frame_len(1), 4 + 70 + 7 * 22);
    assert_eq!(KNOWN_FLAGS, 0b11);
    assert_eq!(
        [Column::Open, Column::Volume, Column::OpenInterest].map(Column::has_mode_one),
        [true; 3]
    );
    assert_eq!(
        [Column::Stamp, Column::Close, Column::High, Column::Low].map(Column::has_mode_one),
        [false; 4]
    );
}

#[test]
fn packing_writes_low_bits_first_and_pads_with_zeros() {
    let mut out = Vec::new();
    pack([1, 0, 1, 1, 0, 0, 0, 1, 1].into_iter(), 1, &mut out);
    assert_eq!(out, [0b1000_1101, 0b0000_0001]);
    let mut out = Vec::new();
    pack([0x3, 0x1, 0x2].into_iter(), 2, &mut out);
    assert_eq!(out, [0b0010_0111]);
    let mut out = Vec::new();
    pack([u64::MAX, 1].into_iter(), 64, &mut out);
    assert_eq!(
        out,
        [[0xFF; 8].as_slice(), &[1, 0, 0, 0, 0, 0, 0, 0]].concat()
    );
    let mut out = Vec::new();
    pack([5, 5, 5].into_iter(), 0, &mut out);
    assert!(out.is_empty(), "width 0 writes nothing");
    let mut out = Vec::new();
    pack([0x1FF, 0x0AB].into_iter(), 9, &mut out);
    assert_eq!(out, [0xFF, 0x57, 0x01]);
}

#[test]
fn a_plan_states_the_exact_length_it_writes() {
    let mut rng = Rng(21);
    for _ in 0..500 {
        let n = rng.below(256) as usize;
        let shift = rng.below(64);
        let residuals: Vec<i64> = (0..n)
            .map(|_| {
                if rng.below(10) == 0 {
                    rng.i64()
                } else {
                    rng.i64() >> shift
                }
            })
            .collect();
        for mode in [0, 1] {
            let plan = Plan::of(&residuals, mode);
            let mut out = Vec::new();
            plan.emit(&residuals, &mut out);
            assert_eq!(out.len(), plan.len);
            assert!(plan.len <= MAX_STREAM_LEN);
            assert_eq!(out[0] >> 7, mode);
            assert!(plan.gcd >= 1 && plan.gcd < GCD_CEILING);
        }
    }
    // All zero, and all i64::MIN: gcd 1, never 2^63.
    assert_eq!(Plan::of(&[0, 0], 0).gcd, 1);
    assert_eq!(Plan::of(&[i64::MIN, i64::MIN], 0).gcd, 1);
    assert_eq!(Plan::of(&[1 << 62, -(1 << 62)], 0).gcd, 1);
    assert_eq!(Plan::of(&[(1 << 61) * 3, 1 << 61], 0).gcd, 1 << 61);
    assert_eq!(Plan::of(&[], 0).len, 4);
}

#[test]
fn euclid_finishes_the_worst_pair_inside_its_bound() {
    // The two largest consecutive Fibonacci numbers a u64 holds are the pair
    // Euclid takes longest on: 91 steps, inside the 128 the loop allows. A
    // bound that cut the walk short would return a Fibonacci number, not 1.
    const F93: u64 = 12_200_160_415_121_876_738;
    const F92: u64 = 7_540_113_804_746_346_429;
    let (mut a, mut b, mut steps) = (F93, F92, 0);
    while b != 0 {
        (a, b) = (b, a % b);
        steps += 1;
    }
    assert_eq!((a, steps), (1, 91));
    assert!(steps < GCD_STEPS);
    assert_eq!(gcd(F93, F92), 1);
    assert_eq!(gcd(F92, F93), 1);
    // A common factor survives the longest walk there is.
    assert_eq!(gcd(F92 * 2, F93), 2, "F93 is even, F92 odd");
    assert_eq!(gcd(u64::MAX, u64::MAX), u64::MAX);
    assert_eq!(gcd(u64::MAX, 0), u64::MAX);
    assert_eq!(gcd(0, u64::MAX), u64::MAX);
    // The longest varint is ten bytes, its last carrying bit 63 alone, and
    // every varint ends on a byte without the continuation bit.
    let mut out = Vec::new();
    put_varint(u64::MAX, &mut out);
    assert_eq!(
        out,
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01]
    );
    for value in [0, 0x7F, 0x80, 0xFF, 1 << 63, u64::MAX >> 1] {
        let mut out = Vec::new();
        put_varint(value, &mut out);
        let (last, rest) = out.split_last().expect("one byte at least");
        assert!(
            last & 0x80 == 0 && rest.iter().all(|b| b & 0x80 != 0),
            "{value}"
        );
    }
}
