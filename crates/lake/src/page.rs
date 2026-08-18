//! A [`PageReader`] that decompresses ZSTD in Rust.
//!
//! # Why this file exists at all
//!
//! `parquet`'s own page reader resolves its codec through
//! `parquet::compression::create_codec`, a free function whose arms are
//! `#[cfg]`-gated on cargo features. There is no registry and no injection
//! point, so the only way to decode a ZSTD page with `parquet`'s value
//! decoders is to enable its `zstd` feature — which pulls `zstd-sys`, 101
//! vendored C files and a `build.rs` running `cc` and `bindgen`. `CLAUDE.md`
//! §2 forbids that without exception.
//!
//! The page *layer*, however, is public. So this type does the two things the
//! disabled feature would have done — parse the page header, decompress the
//! page body — and hands `parquet` a fully formed [`Page`]. Everything below
//! that point (RLE definition levels, PLAIN, RLE\_DICTIONARY) is `parquet`'s
//! own code, unmodified.
//!
//! Headers come from `parquet-format-safe` because `parquet::format` was
//! removed in 59.x and `parquet_thrift` is private. Bodies come from `ruzstd`.
//!
//! # Cost
//!
//! Decompressing a page is O(page bytes) and nothing here pretends otherwise;
//! there is no constant-time claim on this path. `crate::batch::row` is the
//! operation with a bound, and it is proved by
//! `lake::batch::row_lookup_does_not_scan`.
//!
//! # Memory
//!
//! O(page bytes) is not the same as "however many bytes a header asked for".
//! `MAX_PAGE_BYTES` is the ceiling on both the reservation and the
//! decompressed output, and the comment on it says why an unbounded one is an
//! abort rather than a refusal.

use core::fmt;
use std::io::{Cursor, Read, Seek};

use bytes::Bytes;
use parquet::basic::Encoding;
use parquet::column::page::{Page, PageMetadata, PageReader};
use parquet::errors::{ParquetError, Result as PqResult};
use parquet_format_safe::thrift::protocol::TCompactInputProtocol;
use parquet_format_safe::{PageHeader, PageType};

/// The compression codecs this reader implements.
///
/// Deliberately two. Every other codec is refused by name in
/// [`crate::reader`] rather than decoded by a C library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Codec {
    /// Pages are stored as-is.
    Uncompressed,
    /// Pages are ZSTD frames, decoded by `ruzstd`.
    Zstd,
}

impl fmt::Display for Codec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Uncompressed => "UNCOMPRESSED",
            Self::Zstd => "ZSTD",
        })
    }
}

/// The most one page may claim to decompress to, and the most this reader
/// will let one produce.
///
/// # Why a ceiling exists at all
///
/// `uncompressed_page_size` is a number the FILE chose, and until this
/// constant existed the ZSTD arm of [`LakePageReader::decode`] spent it twice
/// before anything checked it: `Vec::with_capacity(expect)` reserved it, and
/// `read_to_end` then grew that vector to whatever the frame decoded to. The
/// only guard was [`length`], which rejects a negative and nothing else.
///
/// AN OVERSIZED RESERVATION IS NOT A REFUSAL. `Vec::with_capacity` reports
/// nothing: when the allocator says no it calls
/// `std::alloc::handle_alloc_error`, which ABORTS rather than panics, so
/// neither `catch_unwind` nor a test harness can see it, and the root
/// `Cargo.toml`'s `panic = "abort"` removes even the theoretical unwind.
/// `CLAUDE.md` §4 — degrade loudly and name the reason, or refuse — and an
/// abort does neither. This is the same fault `crate::reader` bounds on a row
/// group's `num_rows`; the comment there is the longer version of this one and
/// the two are deliberately the same shape.
///
/// # The two halves are not the same size
///
/// The header field is an `i32`, so the largest lie it can tell is `i32::MAX`,
/// about 2 GiB. That is far short of the 8 TiB an `i64` row count could ask
/// for, and the outcome is MACHINE-DEPENDENT rather than certain: with this
/// bound removed, the 2,147,483,647-byte reservation was observed SUCCEEDING
/// on the darwin development machine, which overcommits, and the read then
/// refused for the ordinary length disagreement two GiB later. A guard whose
/// effect depends on the allocator's mood is not a guard; the point is that
/// the size is decided here, where it can be named, and not there.
///
/// **The decompressed output is the unbounded half.** No header field caps it.
/// A ZSTD RLE block costs four bytes and regenerates up to 128 KiB, so a page
/// body of B bytes can decode to roughly `32768 * B`, and the length check
/// that would catch the disagreement runs only AFTER `read_to_end` has
/// finished growing. The `Read::take` in `decode` is what stops that, one byte
/// past what the header promised, so the buffer never grows to the frame's own
/// size.
///
/// # Why the ceiling is a constant and not the chunk's own length
///
/// `crate::reader` bounds a row count by `self.bytes.len()`, the file the
/// count came from, because a row cannot be smaller than nothing. That ceiling
/// cannot be transplanted here: a compressed page decompresses to MANY times
/// the bytes that carry it — that is what compression is for — and the frame
/// in `a_zstd_frame_that_outgrows_its_header_is_stopped_one_byte_past_it` is
/// 13 bytes that decode to 131,072, a ratio of 10,082 to 1. The chunk's length,
/// or any fixed multiple of it, would be a guess about compressibility, and a
/// guess that came in low would refuse real pages.
///
/// A page SIZE is the thing that is actually bounded. 64 MiB is 8,388,608
/// values of the widest lake column — `i64` and `f64` are both eight bytes —
/// in a single page, against a real lake row group of 2,480 rows, the count
/// `tests/real_lake.rs` names for the sampled F&O file, and against a Parquet
/// writer's own page cap — `parquet::file::properties::DEFAULT_PAGE_SIZE` is
/// `1024 * 1024` in 59.2, and the dictionary limit defaults to the same. The
/// margin is
/// three orders of magnitude, and no lake file was measured against it: the
/// lake is 40 GB of operator data that is not on a CI runner, so the figure
/// above is an argument from the format and the writer, NOT a measurement.
///
/// # What this does NOT fix
///
/// It bounds ONE page. A chunk holding thousands of tiny page headers, each
/// with a bomb body, still pays a bounded allocation per page in sequence, and
/// the total decompression WORK across such a chunk is bounded only by the
/// chunk's own length, which is bounded only by the file. That is a time cost
/// rather than a memory one and it is UNMEASURED.
///
/// Nor does it make allocation failure impossible: a legitimate multi-megabyte
/// page still allocates, and a machine already at its limit can still lose
/// that. What it removes is the case where one corrupt header field, or one
/// four-byte block, demands gigabytes.
const MAX_PAGE_BYTES: usize = 64 << 20;

/// Walks the pages of one column chunk, decompressing each.
pub(crate) struct LakePageReader {
    chunk: Bytes,
    pos: usize,
    values_read: i64,
    total_values: i64,
    codec: Codec,
}

impl LakePageReader {
    /// Wraps one column chunk's bytes.
    pub(crate) const fn new(chunk: Bytes, total_values: i64, codec: Codec) -> Self {
        Self {
            chunk,
            pos: 0,
            values_read: 0,
            total_values,
            codec,
        }
    }

    /// Decompresses one page body, insisting the result is the size the header
    /// promised.
    ///
    /// The length check is not belt-and-braces: `ruzstd` will happily return a
    /// short buffer for a truncated frame, and a short page decodes into
    /// plausible-looking values for the rows that survived. Comparing against
    /// the header turns silent corruption into a refusal.
    ///
    /// Nor is the ceiling [`MAX_PAGE_BYTES`] imposes. A length check can only
    /// run once the buffer exists, and the buffer used to be sized — and then
    /// grown — by numbers the file supplied. A refusal that arrives after an
    /// allocation abort is not a refusal.
    fn decompress(&self, body: &[u8], expect: usize) -> PqResult<Bytes> {
        let out = self.decode(body, expect);
        if let Err(why) = &out {
            note_body(self.codec, body.len(), expect, &why.to_string());
        }
        out
    }

    /// The decompression itself.
    ///
    /// Split out of [`LakePageReader::decompress`] so the refusal is recorded
    /// in one place instead of at each of the four `return`s below. Nothing is
    /// recorded on the success arms, and that asymmetry is the whole design:
    /// this function runs once per PAGE, a page holds thousands of rows, and a
    /// line per decoded page would roll a backfill's own beginning out of the
    /// 64 MiB window long before it finished. D-0075.
    fn decode(&self, body: &[u8], expect: usize) -> PqResult<Bytes> {
        match self.codec {
            Codec::Uncompressed => {
                if body.len() != expect {
                    return Err(ParquetError::General(format!(
                        "uncompressed page is {} bytes, header says {expect}",
                        body.len()
                    )));
                }
                Ok(Bytes::copy_from_slice(body))
            }
            Codec::Zstd => {
                // BOUND THE HEADER'S NUMBER BEFORE IT BECOMES A CAPACITY.
                // Everything below this line spends `expect`: the reservation
                // takes it directly and the read cap is derived from it.
                // [`MAX_PAGE_BYTES`] holds the reasoning — why an unbounded
                // reservation is an abort rather than a refusal, and why the
                // ceiling is a constant instead of the chunk's own length.
                //
                // THE UNCOMPRESSED ARM ABOVE NEEDS NO SUCH CHECK and does not
                // get one. It never spends `expect`; it only compares it, and
                // the bytes it copies are a slice of a column chunk already
                // resident in memory. Giving it the ceiling too would read as
                // one law but would refuse a legitimately large uncompressed
                // page for a danger that arm does not have.
                if expect > MAX_PAGE_BYTES {
                    return Err(ParquetError::General(format!(
                        "zstd page header claims {expect} uncompressed bytes, past the \
                         {MAX_PAGE_BYTES}-byte ceiling this reader will materialise for one \
                         page; refusing rather than reserving it"
                    )));
                }
                let dec = ruzstd::decoding::StreamingDecoder::new(body)
                    .map_err(|e| ParquetError::General(format!("zstd frame refused: {e}")))?;

                // ONE BYTE PAST THE PROMISE, and that byte is the whole
                // mechanism: a cap of exactly `expect` would let a frame with
                // more to give fill the buffer and then pass the length check
                // below as though it had decoded correctly. The capacity is
                // the same number, so the refusing read does not double the
                // allocation on its way to failing.
                //
                // `usize` is no wider than `u64` on any target this workspace
                // builds for, and `cap` is under the ceiling besides;
                // `unwrap_or` gives that a total answer rather than an
                // unreachable panic the coverage floor would then count.
                let cap = expect.saturating_add(1);
                let mut out = Vec::with_capacity(cap);
                dec.take(u64::try_from(cap).unwrap_or(u64::MAX))
                    .read_to_end(&mut out)
                    .map_err(|e| ParquetError::General(format!("zstd decode refused: {e}")))?;
                if out.len() != expect {
                    // ONE `!=`, two reasons. Written this way rather than as
                    // two guards so the comparison that decides whether a page
                    // is accepted stays a single mutable point: gate 18 turning
                    // it into `==` must break both tests below, not one.
                    return Err(ParquetError::General(if out.len() > expect {
                        format!(
                            "zstd page decodes to more than the {expect} bytes its header \
                             promised; the read stopped at {} rather than growing to whatever \
                             the frame would have produced",
                            out.len()
                        )
                    } else {
                        format!(
                            "zstd page decoded to {} bytes, header says {expect}",
                            out.len()
                        )
                    }));
                }
                Ok(Bytes::from(out))
            }
        }
    }
}

/// Records a page body that would not decompress, at the moment it refuses.
///
/// **THIS IS THE SILENT-CORRUPTION BOUNDARY AND IT USED TO BE MUTE.** `ruzstd`
/// answers a truncated frame with a short buffer, and a short page decodes into
/// plausible prices for the rows that survived; the length check above is what
/// turns that into a refusal, and this line is what leaves a record of it. The
/// refusal itself reaches a caller as a `ParquetError` wrapped in
/// [`crate::error::LakeError::PageDecode`] — but a caller that logs only the
/// file, or discards the error to move to the next one, leaves nothing on disk
/// saying a page of an irreplaceable file failed to decode.
///
/// The compressed and expected sizes are on the line because together they say
/// which fault it is: a body far shorter than the chunk claimed is a truncated
/// write, and a body of the right size that decodes to the wrong length is bit
/// rot.
///
/// **Bounded by failure, not by data.** Only the refusing arm calls this, and a
/// refusal ends the row group, the column and the read — so the worst case is
/// one line per file that will not decode, never one per page.
///
/// Both halves are proved. That a refusal writes exactly this line:
/// `lake::page::a_refused_page_writes_its_reason_to_the_log`. That a refusal
/// really does end the read rather than being skipped past — which is what
/// turns "one per page" into "one per file":
/// `lake::page::a_truncated_chunk_is_refused_rather_than_read_short`.
fn note_body(codec: Codec, compressed: usize, expect: usize, why: &str) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error("lake.page", "page body would not decompress")
            .with("codec", telemetry::Value::Str(&codec.to_string()))
            .with(
                "compressed_bytes",
                telemetry::Value::Uint(compressed as u64),
            )
            .with("want_bytes", telemetry::Value::Uint(expect as u64))
            .with("why", telemetry::Value::Str(why)),
    );
}

/// Maps a thrift encoding id onto `parquet`'s enum.
///
/// Written as a match on the raw id rather than on `parquet`'s constants
/// because `Encoding::BIT_PACKED` is `#[deprecated]`: naming it would emit a
/// warning, and CI builds with `RUSTFLAGS: -D warnings`. Id 4 is therefore
/// refused by name without the constant being written.
fn encoding(e: parquet_format_safe::Encoding) -> PqResult<Encoding> {
    Ok(match e.0 {
        0 => Encoding::PLAIN,
        2 => Encoding::PLAIN_DICTIONARY,
        3 => Encoding::RLE,
        4 => {
            return Err(ParquetError::General(
                "BIT_PACKED level encoding is deprecated and appears in no lake file; refusing"
                    .to_owned(),
            ));
        }
        5 => Encoding::DELTA_BINARY_PACKED,
        6 => Encoding::DELTA_LENGTH_BYTE_ARRAY,
        7 => Encoding::DELTA_BYTE_ARRAY,
        8 => Encoding::RLE_DICTIONARY,
        9 => Encoding::BYTE_STREAM_SPLIT,
        other => {
            return Err(ParquetError::General(format!(
                "unknown parquet encoding id {other}; refusing rather than guessing"
            )));
        }
    })
}

/// `usize` from a thrift `i32`, refusing a negative length.
fn length(what: &str, v: i32) -> PqResult<usize> {
    usize::try_from(v).map_err(|_| ParquetError::General(format!("{what} is {v}, not a length")))
}

impl Iterator for LakePageReader {
    type Item = PqResult<Page>;

    fn next(&mut self) -> Option<Self::Item> {
        self.get_next_page().transpose()
    }
}

impl PageReader for LakePageReader {
    fn get_next_page(&mut self) -> PqResult<Option<Page>> {
        // Not a loop: every arm below either returns a page or refuses, so
        // there is no path that falls through to a second iteration. Written
        // as an `if` because that is what it is.
        if self.values_read < self.total_values {
            let Some(rest) = self.chunk.get(self.pos..) else {
                return Err(ParquetError::General(
                    "page offset runs past the end of the column chunk".to_owned(),
                ));
            };
            if rest.is_empty() {
                return Ok(None);
            }

            let mut cursor = Cursor::new(rest);
            let header = {
                let mut proto = TCompactInputProtocol::new(&mut cursor, rest.len());
                PageHeader::read_from_in_protocol(&mut proto)
                    .map_err(|e| ParquetError::General(format!("page header refused: {e}")))?
            };
            let consumed = cursor
                .stream_position()
                .map_err(|e| ParquetError::General(format!("cursor refused: {e}")))?;
            let header_len = usize::try_from(consumed).map_err(|_| {
                ParquetError::General("page header length does not fit in usize".to_owned())
            })?;

            let compressed = length("compressed_page_size", header.compressed_page_size)?;
            let uncompressed = length("uncompressed_page_size", header.uncompressed_page_size)?;

            let start = self
                .pos
                .checked_add(header_len)
                .ok_or_else(|| ParquetError::General("page offset overflow".to_owned()))?;
            let end = start
                .checked_add(compressed)
                .ok_or_else(|| ParquetError::General("page extent overflow".to_owned()))?;
            let Some(body) = self.chunk.get(start..end) else {
                return Err(ParquetError::General(format!(
                    "page body {start}..{end} runs past the column chunk end {}",
                    self.chunk.len()
                )));
            };
            self.pos = end;

            match header.type_ {
                PageType::DICTIONARY_PAGE => {
                    let d = header.dictionary_page_header.as_ref().ok_or_else(|| {
                        ParquetError::General("dictionary page carries no header".to_owned())
                    })?;
                    let buf = self.decompress(body, uncompressed)?;
                    return Ok(Some(Page::DictionaryPage {
                        buf,
                        num_values: u32::try_from(d.num_values).map_err(|_| {
                            ParquetError::General(
                                "dictionary page value count is negative".to_owned(),
                            )
                        })?,
                        encoding: encoding(d.encoding)?,
                        is_sorted: d.is_sorted.unwrap_or(false),
                    }));
                }
                PageType::DATA_PAGE => {
                    let d = header.data_page_header.as_ref().ok_or_else(|| {
                        ParquetError::General("data page carries no header".to_owned())
                    })?;
                    self.values_read += i64::from(d.num_values);
                    let buf = self.decompress(body, uncompressed)?;
                    return Ok(Some(Page::DataPage {
                        buf,
                        num_values: u32::try_from(d.num_values).map_err(|_| {
                            ParquetError::General("data page value count is negative".to_owned())
                        })?,
                        encoding: encoding(d.encoding)?,
                        def_level_encoding: encoding(d.definition_level_encoding)?,
                        rep_level_encoding: encoding(d.repetition_level_encoding)?,
                        statistics: None,
                    }));
                }
                other => {
                    // Data page v2 lands here. No lake file sampled — 200 of
                    // them, across F&O, cash and index — contains one, and
                    // guessing at a layout this reader has never seen would be
                    // the silent fallback CLAUDE.md section 4 bans.
                    //
                    // NO EVENT HERE, AND THE REASON IS THE COVERAGE GATE, not
                    // the emit rules — a page-type refusal is terminal for the
                    // chunk, the row group and the read, so one line would be
                    // bounded exactly as `note_body` below is. But no test
                    // reaches this arm: no lake file holds a v2 page, and
                    // building one synthetically belongs in
                    // `crates/lake/tests/refusals.rs`. An emit added ahead of
                    // that test is eight lines and one function that
                    // `--fail-under-lines 100` counts as uncovered. The event
                    // and the test that reaches it must land together.
                    return Err(ParquetError::General(format!(
                        "page type {} is not one this reader implements; refusing rather than skipping the page",
                        other.0
                    )));
                }
            }
        }
        Ok(None)
    }

    fn peek_next_page(&mut self) -> PqResult<Option<PageMetadata>> {
        // Only the record-skipping paths in `parquet` call this, and this
        // crate reads whole row groups. Refusing is honest; returning a
        // fabricated `None` would tell a caller the chunk was exhausted.
        Err(ParquetError::General(
            "peek_next_page is not implemented by the lake page reader".to_owned(),
        ))
    }

    fn skip_next_page(&mut self) -> PqResult<()> {
        Err(ParquetError::General(
            "skip_next_page is not implemented by the lake page reader".to_owned(),
        ))
    }

    fn at_record_boundary(&mut self) -> PqResult<bool> {
        // Every lake column is a flat, non-repeated leaf: max repetition level
        // is 0, so every value is its own record and every page boundary is a
        // record boundary.
        Ok(true)
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "the same exception every test module in this workspace takes"
)]
mod tests {
    use super::*;

    /// AN UNCOMPRESSED PAGE MUST BE EXACTLY THE LENGTH ITS HEADER CLAIMS.
    ///
    /// CI gate 18 mutated this `!=` to `==` and the whole suite stayed green,
    /// which meant the check decided nothing. The consequence is not cosmetic:
    /// a page shorter than its header claims is a **silent short read** — the
    /// column decodes fewer values than the row group says it holds, and this
    /// crate's own `refusals.rs` header records what that cost last time
    /// (2,080 fabricated nulls, the first at a row whose true value was 1,400).
    ///
    /// Both directions are driven, because `!=` is two failures: a body
    /// shorter than the header and a body longer than it are each a
    /// disagreement, and `<` would have caught only one.
    #[test]
    fn an_uncompressed_page_whose_length_disagrees_with_its_header_is_refused() {
        let reader = LakePageReader::new(Bytes::new(), 0, Codec::Uncompressed);
        let body = [1u8, 2, 3, 4];

        // The agreeing case, which is what stops the whole body being replaced
        // by an unconditional refusal.
        let ok = reader
            .decode(&body, body.len())
            .expect("4 bytes, header says 4");
        assert_eq!(&ok[..], &body[..], "the bytes pass through untouched");

        // SHORT. The header promises more than the page carries.
        let short = reader.decode(&body, body.len() + 1).unwrap_err();
        assert!(
            short.to_string().contains("4 bytes") && short.to_string().contains('5'),
            "the refusal names BOTH numbers so an operator need not guess \
             which side is wrong, got: {short}"
        );

        // LONG. The page carries more than the header accounts for — equally a
        // disagreement, and the reason this is `!=` and not `<`.
        let long = reader.decode(&body, body.len() - 1).unwrap_err();
        assert!(
            long.to_string().contains("4 bytes") && long.to_string().contains('3'),
            "a long page disagrees exactly as much as a short one, got: {long}"
        );
    }

    /// AND THE ZSTD PATH HAS ITS OWN LENGTH CHECK, WHICH ALSO DECIDED NOTHING.
    ///
    /// A second `!=`, on a different arm, and CI gate 18 mutated it to `==`
    /// with the suite still green. It matters more than the uncompressed one:
    /// a decompressor can legitimately produce a different length from the
    /// header, and if that disagreement is not refused the column decodes a
    /// short page and the reader fabricates the rest.
    ///
    /// The frame below is a minimal zstd frame built BY HAND — magic, a
    /// single-segment header, one last raw block — because `ruzstd` decodes and
    /// does not encode, and adding an encoder to reach one branch would be a
    /// dependency bought for a test.
    #[test]
    fn a_zstd_page_that_decodes_to_the_wrong_length_is_refused() {
        // 28 B5 2F FD  magic
        // 20           frame header: single segment, 1-byte content size
        // 04           content size = 4
        // 21 00 00     block header: last block, raw, size 4  (1 | 4<<3 = 33)
        // 41 42 43 44  "ABCD"
        let frame: &[u8] = &[
            0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x04, 0x21, 0x00, 0x00, b'A', b'B', b'C', b'D',
        ];
        let reader = LakePageReader::new(Bytes::new(), 0, Codec::Zstd);

        let ok = reader
            .decode(frame, 4)
            .expect("the hand-built frame decodes to its four bytes");
        assert_eq!(&ok[..], b"ABCD", "and to exactly those bytes");

        let wrong = reader.decode(frame, 5).unwrap_err();
        assert!(
            wrong.to_string().contains("decoded to 4 bytes") && wrong.to_string().contains('5'),
            "the refusal names what was decoded AND what was promised, got: {wrong}"
        );
    }

    /// THE EVENT ITSELF REACHES A FILE — and until this test, nothing in the
    /// workspace asserted that of any of its 56 emit sites.
    ///
    /// CI gate 18 mutated `note_body` to `()` — deleting the emit outright —
    /// and the entire suite stayed green. That was a true statement about
    /// every event this repository writes: each one could have been removed
    /// and no gate would have noticed. An observation nothing observes is
    /// worth what an untested branch is worth, and `CLAUDE.md` §4's ban on a
    /// test that asserts nothing is the same rule wearing the other mask.
    ///
    /// # Why a global sink is safe HERE
    ///
    /// `telemetry::install` writes a per-process `OnceLock`, so a test that
    /// installs one races anything else in the same binary that asserts on the
    /// global — which is why `crates/telemetry` gives that scenario its own
    /// test binary. **No other test in THIS binary — `crates/lake`'s lib
    /// target — installs, reads or asserts on the global**, so there is
    /// nothing here to race. If one ever does, this test moves to its own
    /// binary for the reason that file states.
    ///
    /// `tests/events.rs` installs one too, and that is not a collision:
    /// cargo compiles every file under `tests/` into its own binary and runs
    /// it as its own process, so its `OnceLock` is a different one. It covers
    /// the crate's other two emit sites, in `schema.rs` and `reader.rs`, which
    /// this test's opening paragraph was true of when it was written.
    #[test]
    fn a_refused_page_writes_its_reason_to_the_log() {
        let dir = std::env::temp_dir().join(format!("brutex-lake-log-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&dir);
        // `Trace` so the FLOOR is not what decides the outcome: this test is
        // about whether the event is emitted at all, and filtering is already
        // `telemetry::sink`'s to prove.
        let Ok(sink) = telemetry::install(
            &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
        ) else {
            // Another binary in this process installed first. Refusing to
            // assert is right: a test that silently measures somebody else's
            // sink is worse than one that does not run.
            return;
        };

        let reader = LakePageReader::new(Bytes::new(), 0, Codec::Uncompressed);
        let refused = reader.decompress(&[1, 2, 3, 4], 5).unwrap_err();
        assert!(
            refused.to_string().contains("4 bytes"),
            "the premise: the page was refused, got {refused}"
        );

        let text = std::fs::read_to_string(sink.path()).expect("the sink wrote a file");
        assert!(
            text.contains(r#""target":"lake.page""#),
            "THE EVENT REACHED THE FILE. Without this line the emit is \
             deletable and every gate stays green. File held:\n{text}"
        );
        assert!(
            text.contains(r#""want_bytes":5"#) && text.contains(r#""compressed_bytes":4"#),
            "and carried BOTH numbers, so an operator need not guess which \
             side is wrong: {text}"
        );
        assert!(
            text.contains("UNCOMPRESSED"),
            "and the codec, because zstd and uncompressed fail differently: \
             {text}"
        );
        let _ignored = std::fs::remove_dir_all(&dir);
    }

    // -----------------------------------------------------------------------
    // A PAGE HEADER THAT LIES ABOUT `uncompressed_page_size` USED TO BE
    // BELIEVED TWICE, AND NEITHER BELIEF COULD BE OBSERVED FAILING.
    //
    // The ZSTD arm reserved the field and then let `read_to_end` grow past it,
    // so a corrupt or hostile header reached `Vec::with_capacity` and the
    // allocator, not a comparison. `Vec::with_capacity` cannot refuse — its
    // failure is `handle_alloc_error`, an ABORT — so there is no assertion that
    // could be written against the old behaviour on the abort itself. What CAN
    // be asserted is the refusal that replaces it, which is what these two
    // tests do: one for the header's claim, one for the frame's output.
    //
    // Both fixtures are built IN MEMORY. CI gate 1 walks every tracked file and
    // allows no `.parquet` in this repository, and a page body is not a file
    // anyway — a column chunk is a thrift page header followed by bytes, and
    // that is exactly what `one_page_chunk` writes.
    // -----------------------------------------------------------------------

    use parquet_format_safe::DataPageHeader;
    use parquet_format_safe::thrift::protocol::TCompactOutputProtocol;

    /// The 13-byte hand-built frame from
    /// `a_zstd_page_that_decodes_to_the_wrong_length_is_refused`, which decodes
    /// to `ABCD`.
    ///
    /// Repeated rather than hoisted out of that test: its byte-by-byte
    /// annotation is what makes it readable, and moving the bytes away from the
    /// annotation would cost more than thirteen bytes of duplication.
    const SMALL_FRAME: &[u8] = &[
        0x28, 0xB5, 0x2F, 0xFD, 0x20, 0x04, 0x21, 0x00, 0x00, b'A', b'B', b'C', b'D',
    ];

    /// A frame of the same THIRTEEN bytes that decodes to 131,072 of them.
    ///
    /// 28 B5 2F FD  magic
    /// A0           frame header: single segment, 4-byte content size
    /// 00 00 02 00  content size = 131,072
    /// 03 00 10     block header: last block, RLE, regenerated size 131,072
    ///              (1 | 1<<1 | 131072<<3 = 0x100003)
    /// 41           the byte to repeat, `A`
    ///
    /// This is not an exotic construction — it is one legal ZSTD block, the
    /// cheapest expansion the format allows. It is here because a decompressor
    /// bounded only by "what the frame decodes to" is bounded by nothing, and a
    /// four-byte block is the proof.
    const BOMB_FRAME: &[u8] = &[
        0x28, 0xB5, 0x2F, 0xFD, 0xA0, 0x00, 0x00, 0x02, 0x00, 0x03, 0x00, 0x10, b'A',
    ];

    /// What [`BOMB_FRAME`] decodes to, in bytes.
    const BOMB_BYTES: usize = 131_072;

    /// Serialises one v1 `DATA_PAGE` header in front of `body`, the way a column
    /// chunk carries it, so `get_next_page` can be driven end to end.
    ///
    /// `compressed_page_size` is taken from `body` rather than passed, because
    /// a body extent that disagrees with the header is a DIFFERENT fault with
    /// its own refusal (`page body {start}..{end} runs past the column chunk
    /// end`) and would refuse these fixtures before the field under test was
    /// ever read.
    fn one_page_chunk(uncompressed_page_size: i32, body: &[u8]) -> Bytes {
        let header = PageHeader {
            type_: PageType::DATA_PAGE,
            uncompressed_page_size,
            compressed_page_size: i32::try_from(body.len()).expect("a test body fits an i32"),
            crc: None,
            data_page_header: Some(DataPageHeader {
                num_values: 1,
                // PLAIN values, RLE levels: the encodings `encoding()` above
                // maps, so the page that comes back is a real `Page::DataPage`
                // and not an encoding refusal wearing the wrong name.
                encoding: parquet_format_safe::Encoding(0),
                definition_level_encoding: parquet_format_safe::Encoding(3),
                repetition_level_encoding: parquet_format_safe::Encoding(3),
                statistics: None,
            }),
            index_page_header: None,
            dictionary_page_header: None,
            data_page_header_v2: None,
        };

        let mut out: Vec<u8> = Vec::new();
        {
            let mut proto = TCompactOutputProtocol::new(&mut out);
            header
                .write_to_out_protocol(&mut proto)
                .expect("a page header this test just built serialises");
        }
        out.extend_from_slice(body);
        Bytes::from(out)
    }

    /// **A PAGE HEADER CLAIMING MORE THAN THE CEILING IS REFUSED RATHER THAN
    /// RESERVED.**
    ///
    /// `i32::MAX` is chosen because it is the widest lie the field can tell,
    /// not because it is a round number: `uncompressed_page_size` is an `i32`,
    /// so 2,147,483,647 is the largest value that survives `length()`'s sign
    /// check, and `Vec::<u8>::with_capacity` of it asks for 2 GiB.
    ///
    /// **What the unbounded reader does with this input was run, once, by
    /// deleting the bound and re-running this test.** It did not abort: darwin
    /// overcommits, the 2 GiB reservation succeeded, and the refusal that
    /// eventually arrived was the ordinary length disagreement — `zstd page
    /// decoded to 4 bytes, header says 2147483647`. That is the honest result
    /// and it is why the assertions below are written against the CEILING's
    /// wording rather than against "an error happened": on a machine that does
    /// not overcommit, or with a claim the allocator will not pretend to
    /// satisfy, the same input is `handle_alloc_error` and there is no error to
    /// assert on at all. Neither outcome is a refusal an operator can read.
    ///
    /// The sound half is not decoration. If the fixture stopped decoding — a
    /// thrift change, a `ruzstd` change — the refusal below would still fire,
    /// for a different reason, and would prove nothing.
    #[test]
    fn a_page_header_claiming_more_than_the_ceiling_is_refused_rather_than_reserved() {
        let sound = one_page_chunk(4, SMALL_FRAME);
        let mut reader = LakePageReader::new(sound, 1, Codec::Zstd);
        match reader.get_next_page().expect("the sound page decodes") {
            Some(Page::DataPage {
                buf, num_values, ..
            }) => {
                assert_eq!(&buf[..], b"ABCD", "the fixture is a real, decodable page");
                assert_eq!(num_values, 1);
            }
            other => panic!("expected a data page, got {other:?}"),
        }

        // ONLY `uncompressed_page_size` CHANGES. Every other byte is the one
        // the sound fixture carried, so this is a header that lies rather than
        // a chunk that is short — the two are different faults and the second
        // already has its own refusal.
        let lying = one_page_chunk(i32::MAX, SMALL_FRAME);
        let mut reader = LakePageReader::new(lying, 1, Codec::Zstd);
        let refused = reader
            .get_next_page()
            .expect_err("a 2 GiB claim over a 13-byte body must be refused");
        let text = refused.to_string();
        assert!(
            text.contains(&i32::MAX.to_string()),
            "the refusal must report the number the header gave, got: {text}"
        );
        assert!(
            text.contains(&MAX_PAGE_BYTES.to_string()),
            "and the ceiling it broke, so an operator can see which side moved, \
             got: {text}"
        );
        assert!(
            !text.contains("decoded to"),
            "and it must land BEFORE the frame is decoded — a refusal that \
             arrives after the allocation is the abort this bound exists to \
             remove, got: {text}"
        );
    }

    /// **A FRAME THAT OUTGROWS ITS HEADER IS STOPPED ONE BYTE PAST IT.**
    ///
    /// This is the half of the defect no header field bounds. `expect` is
    /// capped by [`MAX_PAGE_BYTES`], but the OUTPUT of `read_to_end` was capped
    /// by nothing at all: it grew to whatever the frame produced, and the
    /// length check that would have caught the disagreement ran only once that
    /// growth had finished. [`BOMB_FRAME`] is thirteen bytes; the vector had
    /// reached 131,072 before anything compared it to the four the header
    /// promised, and thirteen bytes is not the limit of that construction.
    ///
    /// The first assertion is the premise: the frame really is valid ZSTD and
    /// really does expand 10,082-fold. Without it a refusal below could mean
    /// nothing more than "those bytes are not a frame".
    #[test]
    fn a_zstd_frame_that_outgrows_its_header_is_stopped_one_byte_past_it() {
        let r = LakePageReader::new(Bytes::new(), 0, Codec::Zstd);

        let whole = r
            .decode(BOMB_FRAME, BOMB_BYTES)
            .expect("the hand-built bomb is a valid zstd frame");
        assert_eq!(
            whole.len(),
            BOMB_BYTES,
            "13 bytes in, 131,072 out — the premise of this test"
        );
        assert!(
            whole.iter().all(|b| *b == b'A'),
            "and every one of them the RLE byte"
        );

        // The same frame behind a header that promises four bytes.
        let refused = r
            .decode(BOMB_FRAME, 4)
            .expect_err("a frame that outgrows its header must be refused");
        let text = refused.to_string();
        assert!(
            text.contains("stopped at 5"),
            "the read must halt one byte past the promise, which is the only \
             evidence available that it did not keep going, got: {text}"
        );
        assert!(
            !text.contains(&BOMB_BYTES.to_string()),
            "and it must NOT have materialised all 131,072 bytes to find that \
             out — that number in the message means the cap did nothing: {text}"
        );
    }

    #[test]
    fn codec_renders_its_name() {
        assert_eq!(Codec::Uncompressed.to_string(), "UNCOMPRESSED");
        assert_eq!(Codec::Zstd.to_string(), "ZSTD");
    }

    #[test]
    fn known_encodings_map_and_unknown_ones_are_refused() {
        use parquet_format_safe::Encoding as Raw;
        assert_eq!(encoding(Raw(0)).unwrap(), Encoding::PLAIN);
        assert_eq!(encoding(Raw(2)).unwrap(), Encoding::PLAIN_DICTIONARY);
        assert_eq!(encoding(Raw(3)).unwrap(), Encoding::RLE);
        assert_eq!(encoding(Raw(5)).unwrap(), Encoding::DELTA_BINARY_PACKED);
        assert_eq!(encoding(Raw(6)).unwrap(), Encoding::DELTA_LENGTH_BYTE_ARRAY);
        assert_eq!(encoding(Raw(7)).unwrap(), Encoding::DELTA_BYTE_ARRAY);
        assert_eq!(encoding(Raw(8)).unwrap(), Encoding::RLE_DICTIONARY);
        assert_eq!(encoding(Raw(9)).unwrap(), Encoding::BYTE_STREAM_SPLIT);
    }

    #[test]
    fn the_deprecated_level_encoding_is_refused_by_name() {
        let e = encoding(parquet_format_safe::Encoding(4)).unwrap_err();
        assert!(
            e.to_string().contains("BIT_PACKED"),
            "the refusal must name the encoding, got: {e}"
        );
    }

    #[test]
    fn an_unknown_encoding_id_is_refused_rather_than_guessed() {
        let e = encoding(parquet_format_safe::Encoding(99)).unwrap_err();
        assert!(e.to_string().contains("99"), "got: {e}");
        // Id 1 was never assigned by the format.
        assert!(encoding(parquet_format_safe::Encoding(1)).is_err());
    }

    #[test]
    fn a_negative_length_is_refused() {
        assert!(length("compressed_page_size", -1).is_err());
        assert_eq!(length("compressed_page_size", 7).unwrap(), 7);
    }

    #[test]
    fn an_uncompressed_page_of_the_wrong_size_is_refused() {
        let r = LakePageReader::new(Bytes::new(), 0, Codec::Uncompressed);
        assert!(r.decompress(&[1, 2, 3], 4).is_err());
        assert_eq!(
            r.decompress(&[1, 2, 3], 3).unwrap(),
            Bytes::from_static(&[1, 2, 3])
        );
    }

    #[test]
    fn a_body_that_is_not_a_zstd_frame_is_refused() {
        let r = LakePageReader::new(Bytes::new(), 0, Codec::Zstd);
        let err = r.decompress(&[0, 0, 0, 0], 8).unwrap_err();
        assert!(err.to_string().contains("zstd"), "got: {err}");
    }

    #[test]
    fn a_reader_with_no_values_yields_nothing() {
        let mut r = LakePageReader::new(Bytes::from_static(b"junk"), 0, Codec::Zstd);
        assert!(r.get_next_page().unwrap().is_none());
    }

    #[test]
    fn a_truncated_chunk_is_refused_rather_than_read_short() {
        // A chunk that promises values but holds no parseable page header.
        let mut r = LakePageReader::new(Bytes::from_static(&[0xff, 0xff]), 10, Codec::Zstd);
        assert!(r.get_next_page().is_err());
    }

    #[test]
    fn the_unimplemented_paths_refuse_rather_than_lie() {
        let mut r = LakePageReader::new(Bytes::new(), 0, Codec::Zstd);
        assert!(r.peek_next_page().is_err());
        assert!(r.skip_next_page().is_err());
        assert!(r.at_record_boundary().unwrap());
    }
}
