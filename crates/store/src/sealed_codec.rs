//! Codec 1: one frame of a sealed month, as seven bit-packed column streams.
//!
//! A sealed month (`.sbar`, D-4939 for this codec; the file around it is
//! SC-2's) holds its bars in frames of at most [`BLOCK_ROWS`] rows. This module
//! is the frame and nothing else: it turns up to `block_rows` [`Bar`]s into
//! bytes and those bytes back into the same bars, **bit for bit, for any seven
//! `i64`** — `i64::MIN` open interest, a price that jumps from `i64::MAX` to
//! `i64::MIN`, a high below its open, a negative volume. It reads no file, takes
//! no lock and keeps no state between calls, so the same rows always give the
//! same bytes on any host (a sealed month is a pure function of its inputs).
//!
//! # The frame
//!
//! ```text
//! 0    1   codec = 1
//! 1    1   flags. bit 0: row 0's stamp is a whole number of seconds after origin
//!                 bit 1: row 0's open interest is null (i64::MIN); no varint follows
//!                 any other bit is refused
//! 2    2   rows m, 1..=block_rows, little-endian
//! 4    …   row 0, zigzag LEB128 varints, wrapping i64 arithmetic, in this order:
//!            (ts − origin) / unit      unit = 1,000,000 when flag bit 0, else 1
//!            close, open − close, high − close, low − close, volume,
//!            open interest             absent when flag bit 1
//! …    …   seven streams, in the order ts, close, open, high, low, volume,
//!          open interest; each carries the m − 1 residuals of rows 1..m−1
//! ```
//!
//! One stream:
//!
//! ```text
//! 1 byte     bit 7: predictor mode; bits 0..6: width w, 0..=64
//! varint     n_exc, the number of exceptions
//! [1 byte]   e, the exception width; present only when n_exc > 0
//! varint     base
//! varint     gcd, 1 ..< 2^62
//! ceil((m−1)·w / 8) bytes        packed values, w bits each, least significant bit first
//! ceil(n_exc·p / 8) bytes        exception positions, p = max(1, bit length of m−2), increasing
//! ceil(n_exc·e / 8) bytes        exception values, e bits each
//! ```
//!
//! A value decodes as: the packed value, or the exception value at an
//! exception's position; plus `base`; zigzag undone; times `gcd`; plus the
//! column's predictor. Every step wraps.
//!
//! | Column | Mode 0 predictor | Mode 1 predictor |
//! |---|---|---|
//! | ts | previous row's ts | — |
//! | close | previous row's close | — |
//! | open | previous row's close | this row's close |
//! | high | `max(open, close)` of this row | — |
//! | low | `min(open, close)` of this row | — |
//! | volume | zero | previous row's volume |
//! | open interest | previous row's open interest | zero |
//!
//! # The encoder chooses, exactly and deterministically
//!
//! For every column it computes the stream's exact length in bytes for each
//! mode the column has and every width from 0 to the widest value, and keeps
//! the shortest: ties go to mode 0, then to the smaller width. Plain packing at
//! the widest value is one of the candidates, so exceptions are emitted only
//! when they make the stream SHORTER, and a stream is at most
//! [`MAX_STREAM_LEN`] bytes at 256 rows: a frame is at most [`MAX_FRAME_LEN`]
//! (`4 + 70 + 7 × 2,062 = 14,508`), whatever the rows hold (`docs/06-limits.md`
//! L-SC-12: incompressible bars cost at most 57 bytes each, 1 more than raw).
//!
//! # The decoder refuses by name, never by panic
//!
//! [`decode_frame`] checks the frame's CRC-32C before anything else, then
//! refuses each malformed shape with its own [`FrameError`]: a codec that is
//! not 1, a flag it does not know, `m` of 0 or above `block_rows`, a varint
//! past 10 bytes or past 64 bits, a width or exception width above 64, a mode
//! the column does not have, a gcd of 0 or at least 2^62, more exceptions than
//! residuals, an exception position at or past `m − 1` or not after the one
//! before it, a stream that runs past the frame, and bytes left after the
//! seventh stream. On a refusal nothing is appended to the caller's buffer.
//!
//! # Cost
//!
//! One frame is the unit of every read (`docs/06-limits.md` L-SC-1): serving one
//! row decodes at most `block_rows` rows and checksums at most
//! [`MAX_FRAME_LEN`] bytes, a bound that does not move with the month. The
//! decoder allocates nothing beyond the rows it appends; the encoder holds two
//! scratch vectors of `m − 1` residuals.
//!
//! Specification: `state/design/store-compression-20261012.md` §4.2.5 (SC-1).

use crate::crc::crc32c;
use crate::format::{Bar, OI_NULL};
use std::fmt;

/// The codec byte this module writes and the only one it reads.
pub const CODEC: u8 = 1;

/// Rows in a full frame of a sealed month. The head of the file states it and
/// a reader passes what the head says; this is what the sealer writes.
///
/// Why 256: measured by two proposals independently — 512 rows saves 4 to 7%
/// of the bars' bytes and doubles the page latency; 4,096 costs fifteen times.
pub const BLOCK_ROWS: u16 = 256;

/// Columns in a bar, and streams in a frame.
pub const COLUMNS: usize = 7;

/// Bytes before row 0: codec, flags, rows.
pub const HEAD_LEN: usize = 4;

/// The longest LEB128 varint of a `u64`.
pub const MAX_VARINT_LEN: usize = 10;

/// The longest row 0: seven varints of ten bytes.
pub const MAX_FIRST_ROW_LEN: usize = COLUMNS * MAX_VARINT_LEN;

/// The longest stream of a 256-row frame: the width byte, a one-byte zero
/// exception count, ten bytes of base, ten of gcd and 255 values of 64 bits.
pub const MAX_STREAM_LEN: usize = 2_062;

/// The longest 256-row frame: `4 + 70 + 7 × 2,062`.
pub const MAX_FRAME_LEN: usize = 14_508;

/// Frame flag: row 0's stamp is a whole number of seconds after the origin,
/// and its varint counts seconds.
pub const FLAG_WHOLE_SECONDS: u8 = 0b01;

/// Frame flag: row 0's open interest is [`OI_NULL`] and no varint carries it.
pub const FLAG_FIRST_OI_NULL: u8 = 0b10;

/// Every frame flag codec 1 knows. A frame with any other bit set is refused.
pub const KNOWN_FLAGS: u8 = 0b11;

/// Microseconds in a second: the unit of row 0's stamp under
/// [`FLAG_WHOLE_SECONDS`].
pub const SECOND_MICROS: i64 = 1_000_000;

/// The widest packed value, and the widest exception value.
pub const MAX_WIDTH: u8 = 64;

/// The encoder never writes a gcd at or above this, and the decoder refuses
/// one. A common factor that large exists only for residuals that are all
/// zero or all `±2^62`/`i64::MIN`, and coding those with gcd 1 costs nothing
/// that matters, while it keeps `gcd` a positive `i64` on both sides.
pub const GCD_CEILING: u64 = 1 << 62;

/// Bit 7 of a stream's first byte: the predictor mode.
const MODE_BIT: u8 = 0x80;

/// Bits 0..6 of a stream's first byte: the width.
const WIDTH_MASK: u8 = 0x7F;

/// The shift of each byte of a varint; the tenth carries bit 63 alone.
const VARINT_SHIFTS: [u32; MAX_VARINT_LEN] = [0, 7, 14, 21, 28, 35, 42, 49, 56, 63];

/// The longest frame of `block_rows` rows: [`MAX_FRAME_LEN`] at 256.
///
/// # Examples
///
/// ```
/// # use store::sealed_codec::{max_frame_len, BLOCK_ROWS, MAX_FRAME_LEN};
/// assert_eq!(max_frame_len(BLOCK_ROWS), MAX_FRAME_LEN);
/// assert_eq!(max_frame_len(1), 4 + 70 + 7 * 22);
/// ```
#[must_use]
pub fn max_frame_len(block_rows: u16) -> usize {
    HEAD_LEN + MAX_FIRST_ROW_LEN + COLUMNS * max_stream_len(block_rows)
}

/// The longest stream of a frame of `block_rows` rows: [`MAX_STREAM_LEN`] at
/// 256.
#[must_use]
pub fn max_stream_len(block_rows: u16) -> usize {
    1 + 1 + 2 * MAX_VARINT_LEN + 8 * usize::from(block_rows).saturating_sub(1)
}

/// One column of a bar, in the order its stream is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    /// `ts_micros`.
    Stamp,
    /// `close`.
    Close,
    /// `open`.
    Open,
    /// `high`.
    High,
    /// `low`.
    Low,
    /// `volume`.
    Volume,
    /// `open_interest`.
    OpenInterest,
}

impl Column {
    /// The streams of a frame, in the order they are written and decoded:
    /// close comes before open, high and low because their predictors read it.
    pub const ORDER: [Self; COLUMNS] = [
        Self::Stamp,
        Self::Close,
        Self::Open,
        Self::High,
        Self::Low,
        Self::Volume,
        Self::OpenInterest,
    ];

    /// The column's name as the bar's field calls it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Stamp => "ts",
            Self::Close => "close",
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Volume => "volume",
            Self::OpenInterest => "open_interest",
        }
    }

    /// Whether the column has a mode-1 predictor (open, volume, open interest).
    #[must_use]
    pub const fn has_mode_one(self) -> bool {
        matches!(self, Self::Open | Self::Volume | Self::OpenInterest)
    }
}

/// Where in a frame a refusal was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The four bytes before row 0.
    Head,
    /// Row 0's varints.
    FirstRow,
    /// One column's stream.
    Stream(Column),
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Head => f.write_str("the frame head"),
            Self::FirstRow => f.write_str("row 0"),
            Self::Stream(column) => write!(f, "the {} stream", column.name()),
        }
    }
}

/// Why rows could not be encoded as a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeError {
    /// A frame holds at least one row.
    Empty,
    /// More rows than a frame of `block_rows` holds.
    TooManyRows {
        /// Rows offered.
        rows: usize,
        /// Rows a frame holds.
        block_rows: u16,
    },
}

impl EncodeError {
    /// A stable name for the refusal, for an event or a test.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::TooManyRows { .. } => "too_many_rows",
        }
    }
}

impl fmt::Display for EncodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("empty: a frame holds at least one row"),
            Self::TooManyRows { rows, block_rows } => write!(
                f,
                "too_many_rows: {rows} rows offered to a frame of at most {block_rows}"
            ),
        }
    }
}

impl std::error::Error for EncodeError {}

/// Why a frame was refused. Each shape has its own name; none is a panic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameError {
    /// The frame's bytes do not hash to the checksum its directory entry holds.
    Checksum {
        /// The checksum the caller holds for the frame.
        expected: u32,
        /// The checksum of the bytes handed in.
        found: u32,
    },
    /// The codec byte is not [`CODEC`].
    Codec {
        /// The byte found.
        found: u8,
    },
    /// A flag bit outside [`KNOWN_FLAGS`] is set.
    Flags {
        /// The flags byte found.
        found: u8,
    },
    /// `m` is 0 or above the head's `block_rows`.
    Rows {
        /// Rows the frame states.
        rows: u16,
        /// Rows a frame of this month holds.
        block_rows: u16,
    },
    /// A part runs past the end of the frame.
    Truncated {
        /// The part that was cut short.
        part: Part,
    },
    /// A varint runs past ten bytes or past 64 bits.
    VarintTooLong {
        /// The part holding it.
        part: Part,
    },
    /// A packed width above [`MAX_WIDTH`].
    Width {
        /// The stream.
        column: Column,
        /// The width found.
        width: u8,
    },
    /// Mode 1 on a column that has only mode 0.
    Mode {
        /// The stream.
        column: Column,
    },
    /// More exceptions than the stream has residuals.
    ExceptionCount {
        /// The stream.
        column: Column,
        /// Exceptions stated.
        count: u64,
        /// Residuals in the stream, `m − 1`.
        residuals: usize,
    },
    /// An exception width above [`MAX_WIDTH`].
    ExceptionWidth {
        /// The stream.
        column: Column,
        /// The width found.
        width: u8,
    },
    /// A gcd of 0, or at least [`GCD_CEILING`].
    Gcd {
        /// The stream.
        column: Column,
        /// The gcd found.
        gcd: u64,
    },
    /// An exception position at or past the stream's last residual.
    ExceptionPosition {
        /// The stream.
        column: Column,
        /// The position found.
        position: u64,
        /// Residuals in the stream, `m − 1`.
        residuals: usize,
    },
    /// An exception position that does not come after the one before it.
    ExceptionOrder {
        /// The stream.
        column: Column,
        /// The position found.
        position: u64,
        /// The position before it.
        previous: u64,
    },
    /// Bytes are left after the seventh stream.
    Trailing {
        /// How many.
        bytes: usize,
    },
}

impl FrameError {
    /// A stable name for the refusal, for an event or a test.
    #[must_use]
    pub const fn reason(&self) -> &'static str {
        match self {
            Self::Checksum { .. } => "checksum",
            Self::Codec { .. } => "codec",
            Self::Flags { .. } => "flags",
            Self::Rows { .. } => "rows",
            Self::Truncated { .. } => "truncated",
            Self::VarintTooLong { .. } => "varint_too_long",
            Self::Width { .. } => "width",
            Self::Mode { .. } => "mode",
            Self::ExceptionCount { .. } => "exception_count",
            Self::ExceptionWidth { .. } => "exception_width",
            Self::Gcd { .. } => "gcd",
            Self::ExceptionPosition { .. } => "exception_position",
            Self::ExceptionOrder { .. } => "exception_order",
            Self::Trailing { .. } => "trailing_bytes",
        }
    }
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = self.reason();
        match self {
            Self::Checksum { expected, found } => write!(
                f,
                "{reason}: the frame hashes to {found:#010x}, its entry holds {expected:#010x}"
            ),
            Self::Codec { found } => write!(f, "{reason}: codec {found}, this build reads {CODEC}"),
            Self::Flags { found } => write!(
                f,
                "{reason}: flags {found:#04x} set a bit outside {KNOWN_FLAGS:#04x}"
            ),
            Self::Rows { rows, block_rows } => write!(
                f,
                "{reason}: the frame states {rows} rows, a frame holds 1 to {block_rows}"
            ),
            Self::Truncated { part } => write!(f, "{reason}: {part} runs past the frame"),
            Self::VarintTooLong { part } => {
                write!(f, "{reason}: a varint in {part} runs past 64 bits")
            }
            Self::Width { column, width } => write!(
                f,
                "{reason}: the {} stream packs {width} bits, at most {MAX_WIDTH}",
                column.name()
            ),
            Self::Mode { column } => write!(
                f,
                "{reason}: the {} stream states mode 1, which that column does not have",
                column.name()
            ),
            Self::ExceptionCount {
                column,
                count,
                residuals,
            } => write!(
                f,
                "{reason}: the {} stream states {count} exceptions among {residuals} residuals",
                column.name()
            ),
            Self::ExceptionWidth { column, width } => write!(
                f,
                "{reason}: the {} stream's exceptions are {width} bits, at most {MAX_WIDTH}",
                column.name()
            ),
            Self::Gcd { column, gcd } => write!(
                f,
                "{reason}: the {} stream's gcd is {gcd}, it must be 1 to {}",
                column.name(),
                GCD_CEILING - 1
            ),
            Self::ExceptionPosition {
                column,
                position,
                residuals,
            } => write!(
                f,
                "{reason}: the {} stream has an exception at {position}, past its {residuals} residuals",
                column.name()
            ),
            Self::ExceptionOrder {
                column,
                position,
                previous,
            } => write!(
                f,
                "{reason}: the {} stream's exception at {position} does not follow the one at {previous}",
                column.name()
            ),
            Self::Trailing { bytes } => {
                write!(f, "{reason}: {bytes} bytes follow the seventh stream")
            }
        }
    }
}

impl std::error::Error for FrameError {}

/// What [`encode_frame`] appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Encoded {
    /// Bytes appended: the frame's length.
    pub len: usize,
    /// CRC-32C of those bytes, for the frame's directory entry.
    pub crc: u32,
}

/// Appends rows as one frame to `out`.
///
/// `origin_micros` is the month's origin (00:00 IST on the 1st), the same
/// value the reader will pass to [`decode_frame`]; `block_rows` is the head's.
///
/// # Errors
///
/// [`EncodeError::Empty`] for no rows, [`EncodeError::TooManyRows`] for more
/// than `block_rows`. On an error nothing is appended.
///
/// # Examples
///
/// ```
/// # use store::format::{Bar, OI_NULL};
/// # use store::sealed_codec::{decode_frame, encode_frame, BLOCK_ROWS};
/// let bar = Bar { ts_micros: 60_000_000, open: 10, high: 12, low: 9, close: 11, volume: 0, open_interest: OI_NULL };
/// let mut frame = Vec::new();
/// let encoded = encode_frame(&[bar, bar], 0, BLOCK_ROWS, &mut frame)?;
/// let mut rows = Vec::new();
/// decode_frame(&frame, encoded.crc, 0, BLOCK_ROWS, &mut rows)?;
/// assert_eq!(rows, [bar, bar]);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn encode_frame(
    rows: &[Bar],
    origin_micros: i64,
    block_rows: u16,
    out: &mut Vec<u8>,
) -> Result<Encoded, EncodeError> {
    let Some(first) = rows.first() else {
        return Err(EncodeError::Empty);
    };
    let m = u16::try_from(rows.len())
        .ok()
        .filter(|&m| m <= block_rows)
        .ok_or(EncodeError::TooManyRows {
            rows: rows.len(),
            block_rows,
        })?;
    let start = out.len();

    let offset = first.ts_micros.wrapping_sub(origin_micros);
    let whole = offset.wrapping_rem(SECOND_MICROS) == 0;
    let null_oi = first.open_interest == OI_NULL;
    // A sum of disjoint bits, so no operator here has an equivalent twin.
    let flags =
        if whole { FLAG_WHOLE_SECONDS } else { 0 } + if null_oi { FLAG_FIRST_OI_NULL } else { 0 };
    out.push(CODEC);
    out.push(flags);
    out.extend_from_slice(&m.to_le_bytes());

    let stamp = if whole {
        offset.wrapping_div(SECOND_MICROS)
    } else {
        offset
    };
    put_varint(zigzag(stamp), out);
    put_varint(zigzag(first.close), out);
    put_varint(zigzag(first.open.wrapping_sub(first.close)), out);
    put_varint(zigzag(first.high.wrapping_sub(first.close)), out);
    put_varint(zigzag(first.low.wrapping_sub(first.close)), out);
    put_varint(zigzag(first.volume), out);
    if !null_oi {
        put_varint(zigzag(first.open_interest), out);
    }

    let mut residuals = Vec::with_capacity(rows.len());
    for column in Column::ORDER {
        residuals_of(rows, column, 0, &mut residuals);
        let mut best = Plan::of(&residuals, 0);
        if column.has_mode_one() {
            residuals_of(rows, column, 1, &mut residuals);
            let other = Plan::of(&residuals, 1);
            if other.len < best.len {
                best = other;
            } else {
                residuals_of(rows, column, 0, &mut residuals);
            }
        }
        best.emit(&residuals, out);
    }

    let (_, frame) = out.split_at(start);
    Ok(Encoded {
        len: frame.len(),
        crc: crc32c(frame),
    })
}

/// Decodes one frame and appends its rows to `out`; returns how many.
///
/// `crc` is the checksum the frame's directory entry holds; it is checked
/// before any byte is interpreted. `origin_micros` and `block_rows` are the
/// month head's.
///
/// # Errors
///
/// A [`FrameError`] naming the first malformed part. On an error `out` is
/// left exactly as it was handed in: never a partial frame.
pub fn decode_frame(
    frame: &[u8],
    crc: u32,
    origin_micros: i64,
    block_rows: u16,
    out: &mut Vec<Bar>,
) -> Result<usize, FrameError> {
    let (rows, flags, mut cursor) = open(frame, crc, block_rows)?;
    let first = first_row(&mut cursor, flags, origin_micros)?;
    let start = out.len();
    out.reserve(rows);
    out.push(first);
    out.resize(start + rows, Bar::default());
    fill(&mut cursor, rows - 1, &first, out, start)
        .and_then(|()| finish(&cursor))
        .map(|()| rows)
        .inspect_err(|_| out.truncate(start))
}

/// Decodes only the stamps of one frame and appends them to `out`; returns
/// how many. This is what a time lookup reads.
///
/// Every check [`decode_frame`] makes is made here too — the checksum, the
/// head, row 0 and the shape of all seven streams — so a frame one call
/// accepts the other accepts; only the ts stream is unpacked.
///
/// # Errors
///
/// As [`decode_frame`]; on an error `out` is left as it was.
pub fn decode_stamps(
    frame: &[u8],
    crc: u32,
    origin_micros: i64,
    block_rows: u16,
    out: &mut Vec<i64>,
) -> Result<usize, FrameError> {
    let (rows, flags, mut cursor) = open(frame, crc, block_rows)?;
    let first = first_row(&mut cursor, flags, origin_micros)?;
    let start = out.len();
    stamps(&mut cursor, rows, first.ts_micros, out)
        .map(|()| rows)
        .inspect_err(|_| out.truncate(start))
}

/// Unpacks the ts stream into `out` and walks the other six without
/// unpacking them.
fn stamps(
    cursor: &mut Cursor<'_>,
    rows: usize,
    first: i64,
    out: &mut Vec<i64>,
) -> Result<(), FrameError> {
    let residuals = rows - 1;
    let stream = Stream::parse(cursor, Column::Stamp, residuals)?;
    let mut values = stream.values();
    out.reserve(rows);
    out.push(first);
    let mut last = first;
    for _ in 0..residuals {
        last = last.wrapping_add(values.next_value());
        out.push(last);
    }
    for column in Column::ORDER.into_iter().skip(1) {
        Stream::parse(cursor, column, residuals)?;
    }
    finish(cursor)
}

/// Checks the checksum and the head; returns the rows, the flags and a cursor
/// at row 0.
fn open(frame: &[u8], crc: u32, block_rows: u16) -> Result<(usize, u8, Cursor<'_>), FrameError> {
    let found = crc32c(frame);
    if found != crc {
        return Err(FrameError::Checksum {
            expected: crc,
            found,
        });
    }
    let mut cursor = Cursor { rest: frame };
    let codec = cursor.byte(Part::Head)?;
    if codec != CODEC {
        return Err(FrameError::Codec { found: codec });
    }
    let flags = cursor.byte(Part::Head)?;
    if flags & !KNOWN_FLAGS != 0 {
        return Err(FrameError::Flags { found: flags });
    }
    let rows = u16::from_le_bytes([cursor.byte(Part::Head)?, cursor.byte(Part::Head)?]);
    if rows == 0 || rows > block_rows {
        return Err(FrameError::Rows { rows, block_rows });
    }
    Ok((usize::from(rows), flags, cursor))
}

/// Reads row 0's varints.
fn first_row(cursor: &mut Cursor<'_>, flags: u8, origin_micros: i64) -> Result<Bar, FrameError> {
    let part = Part::FirstRow;
    let stamp = unzigzag(cursor.varint(part)?);
    let unit = if flags & FLAG_WHOLE_SECONDS == 0 {
        1
    } else {
        SECOND_MICROS
    };
    let close = unzigzag(cursor.varint(part)?);
    let open = close.wrapping_add(unzigzag(cursor.varint(part)?));
    let high = close.wrapping_add(unzigzag(cursor.varint(part)?));
    let low = close.wrapping_add(unzigzag(cursor.varint(part)?));
    let volume = unzigzag(cursor.varint(part)?);
    let open_interest = if flags & FLAG_FIRST_OI_NULL == 0 {
        unzigzag(cursor.varint(part)?)
    } else {
        OI_NULL
    };
    Ok(Bar {
        ts_micros: origin_micros.wrapping_add(stamp.wrapping_mul(unit)),
        open,
        high,
        low,
        close,
        volume,
        open_interest,
    })
}

/// Decodes the seven streams into rows `start + 1 ..` of `out`, which hold
/// defaults; row `start` is row 0.
fn fill(
    cursor: &mut Cursor<'_>,
    residuals: usize,
    first: &Bar,
    out: &mut [Bar],
    start: usize,
) -> Result<(), FrameError> {
    for column in Column::ORDER {
        let stream = Stream::parse(cursor, column, residuals)?;
        let mut values = stream.values();
        let rows = out.iter_mut().skip(start + 1);
        match (column, stream.mode) {
            (Column::Stamp, _) => {
                let mut last = first.ts_micros;
                for bar in rows {
                    last = last.wrapping_add(values.next_value());
                    bar.ts_micros = last;
                }
            }
            (Column::Close, _) => {
                let mut last = first.close;
                for bar in rows {
                    last = last.wrapping_add(values.next_value());
                    bar.close = last;
                }
            }
            (Column::Open, 0) => {
                let mut previous_close = first.close;
                for bar in rows {
                    bar.open = previous_close.wrapping_add(values.next_value());
                    previous_close = bar.close;
                }
            }
            (Column::Open, _) => {
                for bar in rows {
                    bar.open = bar.close.wrapping_add(values.next_value());
                }
            }
            (Column::High, _) => {
                for bar in rows {
                    bar.high = bar.open.max(bar.close).wrapping_add(values.next_value());
                }
            }
            (Column::Low, _) => {
                for bar in rows {
                    bar.low = bar.open.min(bar.close).wrapping_add(values.next_value());
                }
            }
            (Column::Volume, 0) => {
                for bar in rows {
                    bar.volume = values.next_value();
                }
            }
            (Column::Volume, _) => {
                let mut last = first.volume;
                for bar in rows {
                    last = last.wrapping_add(values.next_value());
                    bar.volume = last;
                }
            }
            (Column::OpenInterest, 0) => {
                let mut last = first.open_interest;
                for bar in rows {
                    last = last.wrapping_add(values.next_value());
                    bar.open_interest = last;
                }
            }
            (Column::OpenInterest, _) => {
                for bar in rows {
                    bar.open_interest = values.next_value();
                }
            }
        }
    }
    Ok(())
}

/// Refuses bytes after the seventh stream.
const fn finish(cursor: &Cursor<'_>) -> Result<(), FrameError> {
    match cursor.rest.len() {
        0 => Ok(()),
        bytes => Err(FrameError::Trailing { bytes }),
    }
}

/// The unread rest of a frame.
struct Cursor<'a> {
    rest: &'a [u8],
}

impl<'a> Cursor<'a> {
    fn byte(&mut self, part: Part) -> Result<u8, FrameError> {
        let (&byte, rest) = self
            .rest
            .split_first()
            .ok_or(FrameError::Truncated { part })?;
        self.rest = rest;
        Ok(byte)
    }

    fn take(&mut self, len: usize, part: Part) -> Result<&'a [u8], FrameError> {
        let (taken, rest) = self
            .rest
            .split_at_checked(len)
            .ok_or(FrameError::Truncated { part })?;
        self.rest = rest;
        Ok(taken)
    }

    fn varint(&mut self, part: Part) -> Result<u64, FrameError> {
        let mut value = 0;
        for shift in VARINT_SHIFTS {
            let byte = self.byte(part)?;
            let low = u64::from(byte & 0x7F);
            // The tenth byte lands at bit 63 and may carry that bit alone.
            if shift == 63 && low > 1 {
                return Err(FrameError::VarintTooLong { part });
            }
            // The byte's bits land above every earlier byte's: adding is or-ing.
            value += low << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(FrameError::VarintTooLong { part })
    }
}

/// One parsed stream: its header, and the three bit-packed regions.
struct Stream<'a> {
    mode: u8,
    width: u32,
    exceptions: usize,
    exception_width: u32,
    position_width: u32,
    base: u64,
    gcd: i64,
    packed: &'a [u8],
    positions: &'a [u8],
    values: &'a [u8],
}

impl<'a> Stream<'a> {
    /// Reads one stream's header and regions and checks every exception
    /// position, so decoding it cannot fail and cannot read out of place.
    fn parse(
        cursor: &mut Cursor<'a>,
        column: Column,
        residuals: usize,
    ) -> Result<Self, FrameError> {
        let part = Part::Stream(column);
        let lead = cursor.byte(part)?;
        let mode = u8::from(lead & MODE_BIT != 0);
        let width = lead & WIDTH_MASK;
        if width > MAX_WIDTH {
            return Err(FrameError::Width { column, width });
        }
        if mode == 1 && !column.has_mode_one() {
            return Err(FrameError::Mode { column });
        }
        let count = cursor.varint(part)?;
        let exceptions = usize::try_from(count)
            .ok()
            .filter(|&n| n <= residuals)
            .ok_or(FrameError::ExceptionCount {
                column,
                count,
                residuals,
            })?;
        let exception_width = if exceptions == 0 {
            0
        } else {
            let width = cursor.byte(part)?;
            if width > MAX_WIDTH {
                return Err(FrameError::ExceptionWidth { column, width });
            }
            width
        };
        let base = cursor.varint(part)?;
        let gcd = cursor.varint(part)?;
        if gcd == 0 || gcd >= GCD_CEILING {
            return Err(FrameError::Gcd { column, gcd });
        }
        let width = u32::from(width);
        let exception_width = u32::from(exception_width);
        let position_width = position_bits(residuals);
        let packed = cursor.take(packed_len(residuals, width), part)?;
        let positions = cursor.take(packed_len(exceptions, position_width), part)?;
        let values = cursor.take(packed_len(exceptions, exception_width), part)?;

        let mut reader = Bits::new(positions, position_width);
        let mut previous = None;
        for _ in 0..exceptions {
            let position = reader.next_bits();
            if position >= residuals as u64 {
                return Err(FrameError::ExceptionPosition {
                    column,
                    position,
                    residuals,
                });
            }
            if let Some(previous) = previous
                && position <= previous
            {
                return Err(FrameError::ExceptionOrder {
                    column,
                    position,
                    previous,
                });
            }
            previous = Some(position);
        }

        Ok(Self {
            mode,
            width,
            exceptions,
            exception_width,
            position_width,
            base,
            gcd: gcd.cast_signed(),
            packed,
            positions,
            values,
        })
    }

    fn values(&self) -> Values<'a> {
        let mut positions = Bits::new(self.positions, self.position_width);
        let next_exception = if self.exceptions == 0 {
            u64::MAX
        } else {
            positions.next_bits()
        };
        Values {
            packed: Bits::new(self.packed, self.width),
            positions,
            exceptions: Bits::new(self.values, self.exception_width),
            exceptions_left: self.exceptions,
            next_exception,
            index: 0,
            base: self.base,
            gcd: self.gcd,
        }
    }
}

/// The residuals of one parsed stream, in row order.
struct Values<'a> {
    packed: Bits<'a>,
    positions: Bits<'a>,
    exceptions: Bits<'a>,
    exceptions_left: usize,
    next_exception: u64,
    index: u64,
    base: u64,
    gcd: i64,
}

impl Values<'_> {
    /// The next residual. Called exactly once a residual row: the parse has
    /// already proved every region long enough and every position in place.
    fn next_value(&mut self) -> i64 {
        let mut raw = self.packed.next_bits();
        if self.index == self.next_exception {
            raw = self.exceptions.next_bits();
            self.exceptions_left -= 1;
            self.next_exception = if self.exceptions_left == 0 {
                u64::MAX
            } else {
                self.positions.next_bits()
            };
        }
        self.index += 1;
        unzigzag(raw.wrapping_add(self.base)).wrapping_mul(self.gcd)
    }
}

/// A little-endian bit reader over one packed region.
///
/// It refills eight bytes at a time while eight remain, so a value costs one
/// shift and one mask, and at most one refill, whatever its width.
struct Bits<'a> {
    rest: &'a [u8],
    held: u128,
    have: u32,
    width: u32,
    mask: u64,
}

impl<'a> Bits<'a> {
    const fn new(bytes: &'a [u8], width: u32) -> Self {
        Self {
            rest: bytes,
            held: 0,
            have: 0,
            width,
            mask: mask(width),
        }
    }

    /// The next `width` bits. Past the region's end it reads zeros, which a
    /// parsed stream never asks for.
    ///
    /// It tops up only by the bits it lacks, a whole word when eight bytes
    /// remain and the bytes it needs otherwise; the register holds `have`
    /// valid bits and zeros above them, so a top-up adds rather than ors.
    fn next_bits(&mut self) -> u64 {
        let lacking = self.width.saturating_sub(self.have);
        if lacking != 0 {
            // `have` is below `width`, at most 64, so 64 more bits fit 128.
            if let Some((word, rest)) = self.rest.split_first_chunk::<8>() {
                self.held += u128::from(u64::from_le_bytes(*word)) << self.have;
                self.have += 64;
                self.rest = rest;
            } else {
                for _ in 0..lacking.div_ceil(8) {
                    let (byte, rest) = self
                        .rest
                        .split_first()
                        .map_or((0, self.rest), |(&b, r)| (b, r));
                    self.held += u128::from(byte) << self.have;
                    self.have += 8;
                    self.rest = rest;
                }
            }
        }
        let value = low_u64(self.held) & self.mask;
        self.held >>= self.width;
        self.have -= self.width;
        value
    }
}

/// What the encoder chose for one stream, with its exact length.
#[derive(Debug, Clone, Copy)]
struct Plan {
    mode: u8,
    gcd: u64,
    base: u64,
    top: u32,
    width: u32,
    exceptions: usize,
    len: usize,
}

impl Plan {
    /// The shortest stream for these residuals in this mode: every width from
    /// 0 to the widest value, exact byte lengths, ties to the smaller width.
    fn of(residuals: &[i64], mode: u8) -> Self {
        let n = residuals.len();
        let gcd = match residuals.iter().fold(0, |g, r| gcd(g, r.unsigned_abs())) {
            g if g == 0 || g >= GCD_CEILING => 1,
            g => g,
        };
        let divisor = gcd.cast_signed();
        let base = residuals
            .iter()
            .map(|r| zigzag(r / divisor))
            .min()
            .unwrap_or(0);
        let mut histogram = [0usize; 65];
        let mut top = 0;
        for r in residuals {
            let bits = bit_len(zigzag(r / divisor) - base);
            top = top.max(bits);
            // `bits` is at most 64, so the slot always exists: no branch is
            // left untaken that a bit length could reach.
            histogram
                .get_mut(bits as usize)
                .into_iter()
                .for_each(|slot| *slot += 1);
        }
        let fixed = 1 + varint_len(base) + varint_len(gcd);
        let position_width = position_bits(n);
        let mut best = Self {
            mode,
            gcd,
            base,
            top,
            width: top,
            exceptions: 0,
            len: usize::MAX,
        };
        let mut above = n;
        for (width, count) in (0..=top).zip(histogram) {
            above -= count;
            let mut len = fixed + varint_len(above as u64) + packed_len(n, width);
            if above > 0 {
                len += 1 + packed_len(above, position_width) + packed_len(above, top);
            }
            if len < best.len {
                best.width = width;
                best.exceptions = above;
                best.len = len;
            }
        }
        best
    }

    /// Writes the stream for `residuals`, which must be the ones planned.
    fn emit(&self, residuals: &[i64], out: &mut Vec<u8>) {
        let divisor = self.gcd.cast_signed();
        let limit = mask(self.width);
        let value = |r: &i64| zigzag(r / divisor) - self.base;
        // `width` is at most 64, so it fits the seven bits below the mode.
        let [width, ..] = self.width.to_le_bytes();
        out.push(if self.mode == 0 {
            width
        } else {
            MODE_BIT + width
        });
        put_varint(self.exceptions as u64, out);
        if self.exceptions > 0 {
            let [top, ..] = self.top.to_le_bytes();
            out.push(top);
        }
        put_varint(self.base, out);
        put_varint(self.gcd, out);
        pack(
            residuals
                .iter()
                .map(value)
                .map(|v| if v > limit { 0 } else { v }),
            self.width,
            out,
        );
        // With no exceptions both packs below are empty and write nothing:
        // the test only spares two passes over the residuals, so it is
        // spelled `!=`, which has no twin that writes the same bytes.
        if self.exceptions != 0 {
            let above = || {
                residuals
                    .iter()
                    .map(value)
                    .enumerate()
                    .filter(|&(_, v)| v > limit)
            };
            pack(
                above().map(|(i, _)| i as u64),
                position_bits(residuals.len()),
                out,
            );
            pack(above().map(|(_, v)| v), self.top, out);
        }
    }
}

/// The residuals of one column in one mode: each row's value less its
/// predictor, rows 1 to `m − 1`.
fn residuals_of(rows: &[Bar], column: Column, mode: u8, out: &mut Vec<i64>) {
    out.clear();
    out.extend(rows.iter().zip(rows.iter().skip(1)).map(|(previous, bar)| {
        let (value, predictor) = match (column, mode) {
            (Column::Stamp, _) => (bar.ts_micros, previous.ts_micros),
            (Column::Close, _) => (bar.close, previous.close),
            (Column::Open, 0) => (bar.open, previous.close),
            (Column::Open, _) => (bar.open, bar.close),
            (Column::High, _) => (bar.high, bar.open.max(bar.close)),
            (Column::Low, _) => (bar.low, bar.open.min(bar.close)),
            (Column::Volume, 0) => (bar.volume, 0),
            (Column::Volume, _) => (bar.volume, previous.volume),
            (Column::OpenInterest, 0) => (bar.open_interest, previous.open_interest),
            (Column::OpenInterest, _) => (bar.open_interest, 0),
        };
        value.wrapping_sub(predictor)
    }));
}

/// Packs each value's low `width` bits, least significant first, and pads the
/// last byte with zeros. Nothing is written at width 0.
fn pack(values: impl Iterator<Item = u64>, width: u32, out: &mut Vec<u8>) {
    let mut held: u128 = 0;
    let mut have = 0;
    for value in values {
        held += u128::from(value) << have;
        have += width;
        // Every whole byte goes out at once; fewer than 8 bits stay held.
        for _ in 0..have / 8 {
            let [low, ..] = held.to_le_bytes();
            out.push(low);
            held >>= 8;
        }
        have %= 8;
    }
    if have > 0 {
        let [low, ..] = held.to_le_bytes();
        out.push(low);
    }
}

/// Bytes `count` values of `width` bits take.
const fn packed_len(count: usize, width: u32) -> usize {
    (count * width as usize).div_ceil(8)
}

/// Bits an exception position takes in a stream of `residuals`.
const fn position_bits(residuals: usize) -> u32 {
    let bits = usize::BITS - residuals.saturating_sub(1).leading_zeros();
    if bits == 0 { 1 } else { bits }
}

/// The low `width` bits set.
const fn mask(width: u32) -> u64 {
    match 1u64.checked_shl(width) {
        Some(bit) => bit - 1,
        None => u64::MAX,
    }
}

/// The low 64 bits of a 128-bit register.
const fn low_u64(held: u128) -> u64 {
    let [b0, b1, b2, b3, b4, b5, b6, b7, ..] = held.to_le_bytes();
    u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7])
}

/// Bits needed to hold `value`: 0 for 0.
const fn bit_len(value: u64) -> u32 {
    u64::BITS - value.leading_zeros()
}

/// Bytes the varint of `value` takes.
const fn varint_len(value: u64) -> usize {
    let bits = bit_len(value) as usize;
    if bits == 0 { 1 } else { bits.div_ceil(7) }
}

/// Appends `value` as an unsigned LEB128 varint, at most [`MAX_VARINT_LEN`]
/// bytes.
///
/// The loop is bounded by the nine bytes that can carry a continuation bit,
/// not by its own condition alone, so no change to that condition can spin:
/// it writes wrong bytes, which the tests see, instead of never returning.
fn put_varint(mut value: u64, out: &mut Vec<u8>) {
    for _ in 1..MAX_VARINT_LEN {
        if value < 0x80 {
            break;
        }
        let [low, ..] = value.to_le_bytes();
        out.push(low | 0x80);
        value >>= 7;
    }
    let [low, ..] = value.to_le_bytes();
    out.push(low);
}

/// Maps a signed value to an unsigned one with small magnitudes small.
const fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)).cast_unsigned()
}

/// The inverse of [`zigzag`], for every `u64`.
const fn unzigzag(value: u64) -> i64 {
    (value >> 1).cast_signed() ^ (value & 1).cast_signed().wrapping_neg()
}

/// Steps that finish Euclid's algorithm on any two `u64`: the remainder at
/// least halves the larger operand every two steps, so 64 bits take at most
/// 128. The worst pair, two consecutive Fibonacci numbers, takes 91.
const GCD_STEPS: u32 = 2 * u64::BITS;

/// Greatest common divisor; `gcd(0, 0)` is 0.
///
/// Bounded by [`GCD_STEPS`], not by its own condition alone, so no change to
/// a step can spin: it returns a wrong divisor, which the tests see.
fn gcd(mut a: u64, mut b: u64) -> u64 {
    for _ in 0..GCD_STEPS {
        if b == 0 {
            break;
        }
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
#[path = "sealed_codec_tests.rs"]
mod sealed_codec_tests;
