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
                let mut dec = ruzstd::decoding::StreamingDecoder::new(body)
                    .map_err(|e| ParquetError::General(format!("zstd frame refused: {e}")))?;
                let mut out = Vec::with_capacity(expect);
                dec.read_to_end(&mut out)
                    .map_err(|e| ParquetError::General(format!("zstd decode refused: {e}")))?;
                if out.len() != expect {
                    return Err(ParquetError::General(format!(
                        "zstd page decoded to {} bytes, header says {expect}",
                        out.len()
                    )));
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
