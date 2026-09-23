//! Unit behaviour of the store crate: `store::unit::*`.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

use brutex_core::instrument::{Exchange, Expiry, InstrumentKey, Kind, OptionSide, Segment};
use brutex_core::price::Paisa;
use brutex_core::symbol::{SYMBOL_CAPACITY, Symbol};
use brutex_core::vendor::Vendor;

use store::block;
use store::crc::{CHECK_VALUE, crc32c, crc32c_split};
use store::format::{
    BLOCK_LEN, Bar, FLAG_CHECKSUMS, FORMAT_VERSION, FormatError, HEADER_LEN, MAGIC, MAGIC_FAMILY,
    MAX_SLOT_COUNT, OI_NULL, RECORD_LEN, RECORD_STRIDE, RECORDS_PER_BLOCK, RETIRED_VERSIONS,
    SLOT_COUNT, SLOT_LEN, SLOT_STRIDE,
};
use store::header::Header;
use store::layout::Layout;
use store::path::{
    FileKind, MAX_EXTENSION_LEN, MAX_LEN, MAX_SEGMENT_LEN, MAX_TIMEFRAME_LEN, MAX_VENDOR_LEN,
    PathError, PathParts, STORE_ROOT, SegmentCase, StorePath, Timeframe, YearMonth, check_segment,
};

// ===========================================================================
// The checksum
// ===========================================================================

/// [`BLOCK_LEN`] as a length, for building test inputs of exactly one block.
const BLOCK_LEN_USIZE: usize = 4088;
const _: () = assert!(BLOCK_LEN_USIZE as u64 == BLOCK_LEN);

#[test]
fn the_crc_matches_the_published_check_value() {
    // The check value is published beside the polynomial; this compares the
    // implementation against it, not the other way round.
    assert_eq!(crc32c(b"123456789"), CHECK_VALUE);
    assert_eq!(CHECK_VALUE, 0xE306_9283);
    // The empty input is the initial value inverted, and one byte differs from
    // it -- so the loop body runs and is not skipped.
    assert_eq!(crc32c(&[]), 0);
    assert_ne!(crc32c(&[0u8]), 0);
    assert_ne!(crc32c(&[0u8]), crc32c(&[1u8]));
}

/// `0..n`, the input every ladder vector below is taken over.
fn seq(n: usize) -> Vec<u8> {
    (0..n).map(|i| u8::try_from(i % 256).unwrap()).collect()
}

/// `i % 251` — a stride coprime with 8 and with 256, so no lane of the
/// slice-by-8 kernel ever sees a repeating byte pattern aligned to its width.
fn ramp(n: usize) -> Vec<u8> {
    (0..n).map(|i| u8::try_from(i % 251).unwrap()).collect()
}

#[test]
fn the_crc_reproduces_a_hardcoded_value_at_every_length_that_can_break() {
    // WHY THIS TEST EXISTS. Until D-0032 the only value pinned anywhere in the
    // repository was `b"123456789"` -- NINE bytes. Every other checksum test is
    // a round trip (`seal` then `verify`, `reseal` then `decode`), and a round
    // trip passes under ANY deterministic function. A kernel that consumes
    // eight bytes at a time and gets the recombination wrong is correct on
    // every input shorter than its stride, so the old set could not have
    // detected a wrong fast path on a single real store input: the shortest
    // thing this store checksums is a 60-byte header domain and the commonest
    // is a 4,088-byte block.
    //
    // The constants below are NOT read out of this implementation. They were
    // produced independently -- by two engines written from the polynomial's
    // published normal form 0x1EDC_6F41 and by its reflected 256-entry table --
    // and they agree with the published check value at 9 bytes. CLAUDE.md S2
    // forbids a tracked binary fixture, so a Rust constant is the only place a
    // golden value can live.
    //
    // The ladder is chosen at the boundaries a wide kernel breaks on: below
    // one stride, exactly one stride, the two lengths either side of it, the
    // header domain, a full block, and a block either side of exact.
    let cases: [(Vec<u8>, u32); 15] = [
        (Vec::new(), 0x0000_0000),
        (b"123456789".to_vec(), 0xE306_9283),
        (vec![0x00], 0x527D_5351),
        (vec![0xFF], 0xFF00_0000),
        (seq(8), 0x8A2C_BC3B),
        (seq(15), 0x68EF_03F6),
        (seq(16), 0xD9C9_08EB),
        (seq(56), 0x01FD_9F28),
        (seq(60), 0x3F69_148B),
        (seq(64), 0xFB6D_36EB),
        (ramp(BLOCK_LEN_USIZE - 1), 0xF723_9185),
        (ramp(BLOCK_LEN_USIZE), 0x703C_26F3),
        (ramp(BLOCK_LEN_USIZE + 1), 0x7991_B685),
        // The lost-write case block.rs exists for: a freshly allocated extent
        // reads as zeros, and every field in it is "sane".
        (vec![0x00; BLOCK_LEN_USIZE], 0x07B4_FA1C),
        (vec![0xFF; BLOCK_LEN_USIZE], 0x8547_5C61),
    ];
    for (input, want) in cases {
        assert_eq!(
            crc32c(&input),
            want,
            "length {} disagrees with its golden value",
            input.len()
        );
    }
}

/// The bit-by-bit kernel this store ran until D-0032, kept as a reference.
///
/// It is deliberately the SLOW shape: one bit at a time, no table, nothing
/// shared with the implementation it checks. Two kernels that agree on
/// randomised input for every length across a stride boundary is the evidence
/// the fast one is not merely fast.
fn crc32c_reference(bytes: &[u8]) -> u32 {
    const POLYNOMIAL: u32 = 0x82F6_3B78;
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8u8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (POLYNOMIAL & mask);
        }
    }
    !crc
}

#[test]
fn the_fast_kernel_agrees_with_a_bit_by_bit_reference_on_every_length() {
    // CI runs on x86_64 and the operator's machine is aarch64. This kernel is
    // ONE body on both -- no `target_arch`, no `target_feature`, no runtime
    // detection -- so this comparison is the same comparison on both hosts, and
    // what CI measures is what the operator runs. See D-0032.
    //
    // xorshift64 rather than a dependency: the sequence has to be identical on
    // every machine and every run, or a failure is not reproducible.
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    // Every length from empty to two full strides plus a tail, then a spread of
    // longer ones including the block size.
    let lengths = (0..=24usize).chain([55, 56, 57, 59, 60, 61, 63, 64, 65, 4087, 4088, 4089]);
    for n in lengths {
        let data: Vec<u8> = (0..n)
            .map(|_| u8::try_from(next() >> 56).unwrap())
            .collect();
        assert_eq!(
            crc32c(&data),
            crc32c_reference(&data),
            "the fast kernel and the reference disagree at length {n}"
        );
    }
}

#[test]
fn splitting_the_input_anywhere_gives_the_same_checksum() {
    // What the header slot depends on: its domain is discontiguous, so the
    // split entry point has to be the same computation as the whole.
    let whole = ramp(137);
    for cut in 0..=whole.len() {
        let (head, tail) = whole.split_at(cut);
        assert_eq!(
            crc32c_split(head, tail),
            crc32c(&whole),
            "splitting at {cut} changed the answer"
        );
    }
}

// ===========================================================================
// The record
// ===========================================================================

#[test]
fn the_record_is_exactly_the_documented_shape() {
    // S-01. Compile-time assertions carry this; the numbers are written out
    // here so the invariant table has a named test beside it.
    assert_eq!(size_of::<Bar>(), 56);
    assert_eq!(align_of::<Bar>(), 8);
    assert_eq!(u64::try_from(size_of::<Bar>()), Ok(RECORD_STRIDE));
    assert_eq!(&MAGIC, b"BRUTEXB2");
    assert_eq!(&MAGIC[..7], &MAGIC_FAMILY[..]);
    assert_eq!(FORMAT_VERSION, 2);
    assert_eq!(FLAG_CHECKSUMS, 1);
}

#[test]
fn the_geometry_is_version_two_and_version_one_is_retired_not_reused() {
    // CLAUDE.md §3 rule 8: store format versions are never mutated in place.
    // The header region, every field offset, the checksum width, the checksum
    // domain and the block length all changed, so this is a new version --
    // not an edit to version 1 wearing version 1's magic.
    assert_eq!(FORMAT_VERSION, 2);
    assert_eq!(&MAGIC, b"BRUTEXB2");
    assert_ne!(&MAGIC, b"BRUTEXB1");
    assert_eq!(RETIRED_VERSIONS, [1]);
    assert_eq!(Layout::for_version(1), Err(FormatError::RetiredVersion(1)));

    // Every geometric number that moved, stated against version 1's.
    assert_eq!((64u64, HEADER_LEN), (64, 32_768), "header region");
    assert_eq!((1u64, SLOT_COUNT), (1, 2), "header slots");
    assert_eq!((4_096u64, BLOCK_LEN), (4_096, 4_088), "checksum block");
    assert_eq!(RECORD_STRIDE, 56, "the one number that did NOT move");
}

#[test]
fn oi_sentinel_distinct() {
    // S-08. Spot indices store the sentinel; a derivative may store a real
    // zero. Conflating them makes the two series indistinguishable on the one
    // field that tells them apart.
    let spot = Bar {
        open_interest: OI_NULL,
        ..Bar::default()
    };
    let derivative = Bar {
        open_interest: 0,
        ..Bar::default()
    };
    assert!(spot.oi_is_null());
    assert!(!derivative.oi_is_null());
    assert_eq!(spot.oi(), None);
    assert_eq!(derivative.oi(), Some(0));
    assert_ne!(spot, derivative);
    assert_eq!(OI_NULL, i64::MIN);
    assert!(format!("{spot:?}").contains("open_interest"));

    // And the default record -- the one a lost write leaves behind -- carries
    // a REAL zero, not the sentinel. That is why a block checksum is the only
    // thing that can tell a flat bar from an extent that was never written.
    assert!(!Bar::default().oi_is_null());
    assert!(Bar::default().ohlc_is_sane());
}

#[test]
fn ohlc_sanity_accepts_real_bars_and_rejects_impossible_ones() {
    // The real first bar of 2024-06-03 from the lake, in paisa.
    let real = Bar {
        ts_micros: 1_717_386_300_000_000,
        open: 2_333_870,
        high: 2_333_870,
        low: 2_308_370,
        close: 2_310_955,
        volume: 0,
        open_interest: OI_NULL,
    };
    assert!(real.ohlc_is_sane());
    assert_eq!(real.close_price(), Paisa::from_raw(2_310_955));

    // A flat bar is legal: high == low == open == close.
    assert!(Bar::default().ohlc_is_sane());

    // Each of the five comparisons, failed on its own.
    assert!(!Bar { high: 1, ..real }.ohlc_is_sane(), "high below open");
    assert!(
        !Bar {
            high: 2_308_369,
            open: 0,
            close: 0,
            ..real
        }
        .ohlc_is_sane(),
        "high below low",
    );
    assert!(
        !Bar {
            close: 9_999_999,
            ..real
        }
        .ohlc_is_sane(),
        "high below close",
    );
    assert!(
        !Bar {
            low: 9_999_999,
            ..real
        }
        .ohlc_is_sane(),
        "low above open",
    );
    assert!(
        !Bar {
            open: 2_310_954,
            low: 2_310_955,
            ..real
        }
        .ohlc_is_sane(),
        "low above close",
    );
}

/// D-0143 — every price is `>= 0`, and the containment clauses never said so.
#[test]
fn negative_prices_are_not_sane_however_well_ordered() {
    // THE BAR THAT USED TO PASS. Every one of the five containment comparisons
    // is satisfied -- high is the highest, low is the lowest -- because all
    // five are RELATIVE and this bar is perfectly ordered. It was appended,
    // the header advanced, the CRC was right, the count was right, and the
    // month was recorded as good.
    let all_negative = Bar {
        ts_micros: 1_717_386_300_000_000,
        open: -100,
        high: -100,
        low: -100,
        close: -100,
        volume: 0,
        open_interest: OI_NULL,
    };
    assert!(
        !all_negative.ohlc_is_sane(),
        "a well-ordered bar whose every price is below zero is still impossible",
    );

    // Each field alone, so no clause can hide behind another. `low` is the one
    // the ordering would have caught anyway if the others were positive, and it
    // is asserted here for the same reason the other three are: this predicate
    // must not depend on a clause of itself being true.
    let real = Bar {
        ts_micros: 1_717_386_300_000_000,
        open: 2_333_870,
        high: 2_333_870,
        low: 2_308_370,
        close: 2_310_955,
        volume: 0,
        open_interest: OI_NULL,
    };
    assert!(real.ohlc_is_sane());
    for (name, bar) in [
        (
            "open",
            Bar {
                open: -1,
                low: -2,
                ..real
            },
        ),
        (
            "high",
            Bar {
                high: -1,
                open: -1,
                low: -2,
                close: -1,
                ..real
            },
        ),
        ("low", Bar { low: -1, ..real }),
        (
            "close",
            Bar {
                close: -1,
                low: -2,
                ..real
            },
        ),
    ] {
        assert!(!bar.ohlc_is_sane(), "{name} below zero must be refused");
    }

    // Zero itself is a price, not an absence -- the boundary is `< 0`, and a
    // bar of four zeroes is the `Bar::default()` a lost write leaves behind,
    // which the block checksum is responsible for, not this predicate.
    assert!(Bar::default().ohlc_is_sane());

    // AND THE CONSEQUENCE THE OVERFLOW GUARDS ELSEWHERE DEPEND ON: with
    // `high >= low >= 0`, the span cannot wrap. See `indicators::Corrupt`.
    assert!(real.high.checked_sub(real.low).is_some());
}

/// A count is never negative, and the one legal negative is the OI sentinel.
#[test]
fn a_negative_count_is_refused_and_the_oi_sentinel_is_not() {
    let real = Bar {
        ts_micros: 1_717_386_300_000_000,
        open: 2_333_870,
        high: 2_333_870,
        low: 2_308_370,
        close: 2_310_955,
        volume: 41_250,
        open_interest: OI_NULL,
    };
    assert!(real.ohlc_is_sane() && real.counts_are_sane());

    // ZERO IS A REAL COUNT, on both fields. §7: zero means zero.
    assert!(
        Bar {
            volume: 0,
            open_interest: 0,
            ..real
        }
        .counts_are_sane()
    );

    // THE SENTINEL IS THE ONE LEGAL NEGATIVE, and it is `i64::MIN` -- so a
    // naive `>= 0` on open interest would have refused every bar this store
    // holds for a cash equity.
    assert_eq!(OI_NULL, i64::MIN);
    assert!(
        Bar {
            open_interest: OI_NULL,
            ..real
        }
        .counts_are_sane()
    );
    assert!(
        Bar {
            open_interest: OI_NULL,
            ..real
        }
        .oi_is_null()
    );

    // AND EVERYTHING ELSE BELOW ZERO IS REFUSED.
    for (what, bar) in [
        ("volume", Bar { volume: -1, ..real }),
        (
            "volume at the floor",
            Bar {
                volume: i64::MIN,
                ..real
            },
        ),
        (
            "open interest",
            Bar {
                open_interest: -1,
                ..real
            },
        ),
        // One short of the sentinel: the nearest legal-looking impostor.
        (
            "open interest beside the sentinel",
            Bar {
                open_interest: i64::MIN + 1,
                ..real
            },
        ),
    ] {
        assert!(!bar.counts_are_sane(), "{what} below zero must be refused");
        // ...and the OHLC predicate still says yes, which is exactly why this
        // is a second question and not a clause of the first.
        assert!(
            bar.ohlc_is_sane(),
            "{what} does not make the OHLC impossible"
        );
    }
}

// ===========================================================================
// The record's 56 bytes — the encoder, pinned
// ===========================================================================
//
// WHY THIS SECTION EXISTS. `Bar::image` documents itself as "the single
// authoritative encoder for the 56 bytes on disk" and argues that a second
// encoder "would be a second definition of the format, free to drift". An
// audit measured what the code actually had:
//
//   - nothing in the repository called `Bar::image`, so it carried zero
//     coverage;
//   - replacing its entire body with `[0u8; 56]` left all 79 store tests
//     green;
//   - `store::fault` re-implemented the encoder privately — the exact
//     duplication the comment forbids.
//
// A comment asserting a property the code does not have is worse than an
// untested function, because it stops anyone looking. The private copy is
// deleted, `store::fault::bitflip_detected` now builds its bytes through
// `Bar::image`, and the tests below pin the output so the zero mutant, a
// swapped field pair, a moved offset and a flipped byte order each fail.
// D-0039.

/// The real first bar of 2024-06-03 from the lake, in paisa.
///
/// Spot, so its open interest is [`OI_NULL`] and its volume is a real zero —
/// the two values a record can carry that mean something other than a number.
const REAL_BAR: Bar = Bar {
    ts_micros: 1_717_386_300_000_000,
    open: 2_333_870,
    high: 2_333_870,
    low: 2_308_370,
    close: 2_310_955,
    volume: 0,
    open_interest: OI_NULL,
};

/// Where each field's eight bytes begin in the image, in the order
/// `Bar::image` writes them.
const FIELD_OFFSETS: [usize; 7] = [0, 8, 16, 24, 32, 40, 48];

/// How many fields one record has.
const FIELDS: usize = 7;

const _: () = assert!(FIELD_OFFSETS.len() == FIELDS);

/// `base` with field `index` replaced by `value`.
///
/// The seven-arm match is the point: it names the fields in the order the
/// image writes them, so a field added to `Bar` without a place in the format
/// stops compiling here rather than silently going unencoded.
fn with(base: Bar, index: usize, value: i64) -> Bar {
    let mut bar = base;
    match index {
        0 => bar.ts_micros = value,
        1 => bar.open = value,
        2 => bar.high = value,
        3 => bar.low = value,
        4 => bar.close = value,
        5 => bar.volume = value,
        6 => bar.open_interest = value,
        _ => panic!("a record has {FIELDS} fields; {index} is not one of them"),
    }
    bar
}

/// A record with `value` in field `index` and zero in every other field.
fn only(index: usize, value: i64) -> Bar {
    with(Bar::default(), index, value)
}

/// A deterministic 64-bit stream.
///
/// Not a random source — a *reproducible* one. `CLAUDE.md` §3 rule 5: same
/// inputs, same outputs. A test that samples a different set on every run
/// produces a failure nobody can reproduce, which is a test that reports
/// nothing.
struct Xorshift(u64);

impl Xorshift {
    /// The next value, as an `i64` with the same bits.
    ///
    /// Through `to_le_bytes`/`from_le_bytes` rather than a cast: this
    /// workspace denies casts that can wrap, and the reinterpretation is what
    /// is wanted — every 64-bit pattern, including the negative half.
    fn draw(&mut self) -> i64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        i64::from_le_bytes(self.0.to_le_bytes())
    }
}

#[test]
fn the_record_image_is_the_pinned_bytes_of_a_known_bar() {
    // THE TEST THE ALL-ZERO MUTANT CANNOT SURVIVE. A round trip cannot catch
    // that mutant: `decode(image(r)) == r` is an identity under any pair of
    // mutually inverse functions, and under an all-zero encoder the decode of
    // zeros is a legal flat bar. Only literal bytes can catch it.
    //
    // These 56 bytes ARE the format. If this array has to change, a format
    // version changes with it — CLAUDE.md §3 rule 8, and the reason
    // `RETIRED_VERSIONS` exists.
    #[rustfmt::skip]
    const PINNED: [u8; RECORD_LEN] = [
        // ts_micros     1_717_386_300_000_000 == 0x0006_19F4_285A_8700
        0x00, 0x87, 0x5A, 0x28, 0xF4, 0x19, 0x06, 0x00,
        // open                    2_333_870 == 0x0000_0000_0023_9CAE
        0xAE, 0x9C, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
        // high                    2_333_870 == 0x0000_0000_0023_9CAE
        0xAE, 0x9C, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
        // low                     2_308_370 == 0x0000_0000_0023_3912
        0x12, 0x39, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
        // close                   2_310_955 == 0x0000_0000_0023_432B
        0x2B, 0x43, 0x23, 0x00, 0x00, 0x00, 0x00, 0x00,
        // volume                          0 -- a REAL zero
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        // open_interest  OI_NULL == i64::MIN == 0x8000_0000_0000_0000
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
    ];

    assert_eq!(REAL_BAR.image(), PINNED);
    assert_eq!(REAL_BAR.image().len(), 56);
    assert_eq!(RECORD_LEN, 56);
    assert_eq!(u64::try_from(RECORD_LEN), Ok(RECORD_STRIDE));

    // The mutant the audit actually planted, refused by name.
    assert_ne!(REAL_BAR.image(), [0u8; RECORD_LEN], "the all-zero body");

    // The two bytes that carry a meaning rather than a magnitude, on their
    // own: the sentinel's sign bit is the LAST byte of the record, and a real
    // zero volume is eight zero bytes and not an absence.
    assert_eq!(PINNED[55], 0x80, "OI_NULL's sign bit, at byte 55");
    assert_eq!(&PINNED[40..48], &[0u8; 8][..], "volume 0 IS eight zeros");

    // The inverse taken on the PINNED bytes rather than on the encoder's
    // output, so the decoder is pinned to the same 56 bytes and not merely to
    // whatever `image` happens to return.
    assert_eq!(Bar::decode(&PINNED), Ok(REAL_BAR));
    assert_eq!(
        Bar::decode(&[0u8; RECORD_LEN]),
        Ok(Bar::default()),
        "zeros decode to a flat bar with a REAL zero open interest",
    );
    assert_ne!(Bar::decode(&[0u8; RECORD_LEN]), Ok(REAL_BAR));
}

#[test]
fn the_image_is_little_endian_and_each_field_owns_its_own_offset() {
    // `Bar::image`'s doc comment claims little-endian. Stated here WITHOUT
    // `to_le_bytes`, so the test does not prove the claim by restating it:
    // the LEAST significant byte comes first.
    let stepped = Bar {
        ts_micros: 0x0102_0304_0506_0708,
        ..Bar::default()
    };
    let image = stepped.image();
    assert_eq!(
        &image[..8],
        &[0x08u8, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01][..],
        "little-endian: least significant byte first",
    );
    assert_ne!(
        &image[..8],
        &[0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08][..],
        "big-endian, named so the claim cannot pass on a symmetric value",
    );
    assert_eq!(
        &image[8..],
        &[0u8; 48][..],
        "one field set, one field moved"
    );

    // Every field, every byte of it: 7 x 8 = 56 placements, each asserted
    // against all 56 bytes of the image. A swapped pair of fields, an offset
    // off by one, or a byte order flipped inside a SINGLE field fails here.
    let mut placements = 0usize;
    for (field, &offset) in FIELD_OFFSETS.iter().enumerate() {
        for byte in 0..8usize {
            let mut wanted = [0u8; 8];
            wanted[byte] = 0xA5;
            let bar = only(field, i64::from_le_bytes(wanted));
            let placed = bar.image();
            for (at, got) in placed.iter().enumerate() {
                let expect = if at == offset + byte { 0xA5u8 } else { 0u8 };
                assert_eq!(*got, expect, "field {field} byte {byte}: image[{at}]");
            }
            assert_eq!(Bar::decode(&placed), Ok(bar), "and back again");
            placements += 1;
        }
    }
    assert_eq!(placements, 56, "every byte of every field was placed");
    assert_eq!(FIELD_OFFSETS, [0, 8, 16, 24, 32, 40, 48]);
    assert_eq!(
        FIELD_OFFSETS[FIELDS - 1] + 8,
        RECORD_LEN,
        "no slack, no pad"
    );
}

#[test]
fn decoding_the_image_returns_the_record_byte_for_byte() {
    // Every boundary a 64-bit field has. i64::MIN is OI_NULL -- the one value
    // whose meaning is "absent" rather than a number -- so it appears in EVERY
    // field, not only in the field that gives it that meaning.
    const CORNERS: [i64; 12] = [
        i64::MIN,
        i64::MIN + 1,
        -2_147_483_649,
        -1,
        0,
        1,
        255,
        256,
        2_147_483_648,
        0x0102_0304_0506_0708,
        i64::MAX - 1,
        i64::MAX,
    ];
    const DRAWS: usize = 4_096;

    assert_eq!(CORNERS[0], OI_NULL, "the null sentinel is a corner");
    assert_eq!(CORNERS[4], 0, "and so is a real zero");

    let mut cases: Vec<Bar> = Vec::new();
    // One corner in one field at a time, twice: once against a real bar, so a
    // field that is never written is caught, and once against zeros, so a
    // field written into the WRONG offset is caught.
    for field in 0..FIELDS {
        for &corner in &CORNERS {
            cases.push(with(REAL_BAR, field, corner));
            cases.push(only(field, corner));
        }
    }
    // Every field at the same corner at once.
    for &corner in &CORNERS {
        let mut bar = Bar::default();
        for field in 0..FIELDS {
            bar = with(bar, field, corner);
        }
        cases.push(bar);
    }
    // And 4,096 records whose every field is an unrelated 64-bit pattern.
    let mut rng = Xorshift(0x2545_F491_4F6C_DD1D);
    for _ in 0..DRAWS {
        cases.push(Bar {
            ts_micros: rng.draw(),
            open: rng.draw(),
            high: rng.draw(),
            low: rng.draw(),
            close: rng.draw(),
            volume: rng.draw(),
            open_interest: rng.draw(),
        });
    }
    assert_eq!(
        cases.len(),
        FIELDS * CORNERS.len() * 2 + CORNERS.len() + DRAWS
    );
    assert_eq!(cases.len(), 4_276);

    // `seen` maps an image back to the record that produced it; `distinct`
    // counts the records themselves. If the two sizes agree, no two different
    // records share one image -- the encoder is injective on this whole set,
    // which is the property that makes a file's bytes mean one thing.
    let mut seen: HashMap<[u8; RECORD_LEN], Bar> = HashMap::new();
    let mut distinct: HashSet<[i64; 7]> = HashSet::new();
    for bar in cases {
        let image = bar.image();
        assert_eq!(image.len(), RECORD_LEN);

        // decode(image(r)) == r, EXACTLY.
        let back = Bar::decode(&image).expect("56 bytes is a whole record");
        assert_eq!(back, bar, "decode(image(r)) != r for {bar:?}");
        // image(decode(image(r))) == image(r), byte for byte -- so the two are
        // inverse in both directions, not merely in one.
        assert_eq!(back.image(), image, "image(decode(image(r))) drifted");
        // Field by field, so `PartialEq` alone is not what is being trusted.
        assert_eq!(back.ts_micros, bar.ts_micros);
        assert_eq!(back.open, bar.open);
        assert_eq!(back.high, bar.high);
        assert_eq!(back.low, bar.low);
        assert_eq!(back.close, bar.close);
        assert_eq!(back.volume, bar.volume);
        assert_eq!(back.open_interest, bar.open_interest);
        assert_eq!(back.oi_is_null(), bar.oi_is_null(), "the sentinel survived");
        assert_eq!(back.oi(), bar.oi());

        // A longer buffer decodes the FIRST record and never reads the tail.
        let mut long = image.to_vec();
        long.extend([0xFFu8; RECORD_LEN]);
        assert_eq!(Bar::decode(&long), Ok(bar), "the tail reached the record");

        if let Some(previous) = seen.insert(image, bar) {
            assert_eq!(previous, bar, "two different records share one image");
        }
        distinct.insert([
            bar.ts_micros,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
            bar.open_interest,
        ]);
    }
    assert_eq!(seen.len(), distinct.len(), "the encoder is not injective");
    assert!(distinct.len() > DRAWS, "the sample did not collapse");
}

#[test]
fn a_short_record_is_refused_and_never_completed_with_zeros() {
    let image = REAL_BAR.image();

    // Every length a ragged tail can leave behind, zero included.
    for len in 0..RECORD_LEN {
        assert_eq!(
            Bar::decode(&image[..len]),
            Err(FormatError::RecordTooShort { len }),
            "a {len}-byte tail must be named, not completed",
        );
    }
    assert_eq!(Bar::decode(&image), Ok(REAL_BAR), "56 bytes is enough");
    assert_eq!(
        Bar::decode(&[]),
        Err(FormatError::RecordTooShort { len: 0 }),
        "an empty tail is a tail",
    );

    // Measured against the specific wrong answer rather than against "not
    // ok". The sentinel's sign bit is the LAST byte of the record, so a
    // 55-byte tail completed with one zero would decode as a bar whose open
    // interest is a REAL zero -- the field that tells a derivative series
    // from an index series, inverted by an invented byte.
    let mut completed = image;
    completed[RECORD_LEN - 1] = 0;
    let invented = Bar::decode(&completed).expect("56 bytes");
    assert!(
        !invented.oi_is_null(),
        "this is what zero-filling would say"
    );
    assert_eq!(invented.oi(), Some(0));
    assert_ne!(invented, REAL_BAR);
    assert!(REAL_BAR.oi_is_null(), "and this is what the record says");

    // The refusal names the length it saw, so the operator is not sent to a
    // hex dump to find out how ragged the tail was.
    let refusal = FormatError::RecordTooShort { len: 55 };
    assert!(refusal.to_string().contains("55"));
    assert!(refusal.to_string().contains("56"), "and the length needed");
}

#[test]
fn decoding_reads_exactly_fifty_six_bytes_however_long_the_buffer_is() {
    // CLAUDE.md §3 rule 4: bar lookup is O(1). `Bar::decode`'s copy loop is
    // `image.iter_mut().zip(bytes.iter())` -- `image` is 56 bytes, so the zip
    // stops at 56 whatever `bytes.len()` is. That is the structural argument;
    // this is the observable form of it.
    //
    // NOT A TIMING MEASUREMENT. It proves the ANSWER does not depend on the
    // buffer's length, which is why the trip count cannot either. Gate 8's
    // bench measures header read and block seal, not this.
    let image = REAL_BAR.image();
    let mut haystack = image.to_vec();
    // A whole month file's worth of bytes past the record, every one of them
    // 0xFF -- the value that would corrupt any field it reached.
    haystack.extend(std::iter::repeat_n(0xFFu8, 1 << 20));
    assert_eq!(haystack.len(), RECORD_LEN + 1_048_576);

    let from_long = Bar::decode(&haystack).expect("the first 56 bytes");
    let from_exact = Bar::decode(&image).expect("the same 56 bytes");
    assert_eq!(from_long, from_exact, "byte 57 onward changed the answer");
    assert_eq!(from_long, REAL_BAR);
    assert_eq!(from_long.image(), image, "and re-encodes to the same bytes");

    // The same at the two lengths either side of the stride, so the boundary
    // itself is walked and not stepped over.
    assert_eq!(Bar::decode(&haystack[..RECORD_LEN]), Ok(REAL_BAR));
    assert_eq!(Bar::decode(&haystack[..=RECORD_LEN]), Ok(REAL_BAR));
    assert_eq!(
        Bar::decode(&haystack[..RECORD_LEN - 1]),
        Err(FormatError::RecordTooShort { len: 55 }),
    );
}

// ===========================================================================
// Version dispatch
// ===========================================================================

#[test]
fn the_constants_are_the_current_versions_layout() {
    let v2 = Layout::V2;
    assert_eq!(v2.version(), FORMAT_VERSION);
    assert_eq!(v2.magic(), MAGIC);
    assert_eq!(v2.header_len(), HEADER_LEN);
    assert_eq!(v2.slot_count(), SLOT_COUNT);
    assert_eq!(v2.slot_stride(), SLOT_STRIDE);
    assert_eq!(v2.record_stride(), RECORD_STRIDE);
    assert_eq!(v2.records_per_block(), RECORDS_PER_BLOCK);
    assert_eq!(v2.block_len(), BLOCK_LEN);
    assert_eq!(Layout::CURRENT, v2);
    // THREE GEOMETRIES, NOT ONE. `KNOWN` answers "which geometries can this
    // build read", and two of the three are sidecars: the overlay's 24-byte
    // records at version 9 and the computed greeks' 80 at version 8, beside the
    // bar's 56 at version 2. Resolution is by the file's own version number, so
    // none can be confused by a reader that reads the header it was handed.
    assert_eq!(Layout::KNOWN, &[v2, Layout::OVERLAY, Layout::GREEKS]);
    assert_eq!(Layout::OVERLAY.record_stride(), 24);
    assert_eq!(Layout::GREEKS.record_stride(), 80);

    // EVERY VERSION IN THE LIST IS DISTINCT, and every stride with it. A
    // duplicate version makes resolution pick whichever row is first, and a
    // duplicate stride makes two geometries indistinguishable to a reader that
    // resolved correctly — asserted as a property over the whole list rather
    // than as a pair, so a fourth row is checked against all three.
    for (index, one) in Layout::KNOWN.iter().enumerate() {
        for other in Layout::KNOWN.iter().skip(index + 1) {
            assert_ne!(
                one.version(),
                other.version(),
                "two geometries share a version, which is what resolves them apart"
            );
            assert_ne!(
                one.record_stride(),
                other.record_stride(),
                "two geometries share a stride, so a record count computed for \
                 one would be right for the other"
            );
        }
    }
    const { assert!(SLOT_COUNT <= MAX_SLOT_COUNT) }

    // The header region is exactly `slot_count` slots at SLOT_STRIDE spacing,
    // checked rather than cast, and each slot's fields fit in SLOT_LEN.
    assert_eq!(SLOT_COUNT * SLOT_STRIDE, HEADER_LEN);
    assert_eq!(u64::try_from(SLOT_LEN), Ok(64));
    assert!(u64::try_from(SLOT_LEN).expect("fits") < SLOT_STRIDE);

    // And the const row agrees with the declared form, byte for byte.
    assert_eq!(
        Layout::declare(2, MAGIC, SLOT_COUNT, RECORD_STRIDE, RECORDS_PER_BLOCK),
        Ok(v2),
    );
}

#[test]
fn every_known_row_is_one_declare_would_admit() {
    // `Layout::KNOWN` is const rows, and `declare`'s guards are only reachable
    // from outside the module -- so a hand-written row could have bypassed
    // every one of them, including `slot_count >= 2`, which IS the crash
    // guarantee. `declared` makes that a compile error; this makes it a test
    // failure too, so neither door is the only one.
    for row in Layout::KNOWN {
        assert_eq!(
            Layout::declare(
                row.version(),
                row.magic(),
                row.slot_count(),
                row.record_stride(),
                row.records_per_block(),
            ),
            Ok(*row),
            "a KNOWN row must be a geometry `declare` admits",
        );
        assert!(row.slot_count() >= 2, "one slot cannot be updated safely");
        assert!(row.slot_count() <= MAX_SLOT_COUNT);
        assert!(row.records_per_block() > 0, "block_of would divide by zero");
        assert_eq!(row.block_len() % row.record_stride(), 0);
        assert_eq!(row.header_len(), row.slot_count() * row.slot_stride());
    }

    // The two doors agree on a geometry neither of them has seen.
    assert_eq!(
        Layout::declare(3, *b"BRUTEXB3", 2, 72, 57),
        Ok(Layout::declared(3, *b"BRUTEXB3", 2, 72, 57)),
    );
}

#[test]
fn unknown_version_refuses() {
    // S-09. A future version has its own layout; guessing at it would read
    // fields from the wrong offsets and return plausible nonsense.
    // 9 IS NO LONGER A STRANGER. The overlay sidecar took it, so a test that
    // wants "a version this build does not know" has to pick one that stays
    // unknown — 3 and 255 and u16::MAX still are, and 7 replaces the 9.
    for version in [0u16, 3, 7, 255, u16::MAX] {
        let refusal = Layout::for_version(version);
        assert_eq!(refusal, Err(FormatError::UnknownVersion(version)));
        let rendered = refusal.unwrap_err().to_string();
        assert!(
            rendered.contains(&version.to_string()),
            "the refusal must name the version it saw, got {rendered:?}",
        );
    }
    // A retired version is a different answer from an unknown one: unknown
    // means the file is newer than the build, retired means it is older.
    let retired = Layout::for_version(1).unwrap_err();
    assert_eq!(retired, FormatError::RetiredVersion(1));
    assert!(retired.to_string().contains("retired"));
    assert_ne!(
        retired.to_string(),
        FormatError::UnknownVersion(1).to_string()
    );

    // Every version in the table resolves, and nothing else does.
    for layout in Layout::KNOWN {
        assert_eq!(Layout::for_version(layout.version()), Ok(*layout));
    }
    assert_eq!(Layout::for_version(2).map(Layout::record_stride), Ok(56));
}

#[test]
fn a_known_version_keeps_its_stride_after_a_new_one_exists() {
    // A hypothetical version 3 at a different stride and a different block
    // size, declared through the same door a real one would be, and resolved
    // by the same function the read path calls.
    let v3 = Layout::declare(3, *b"BRUTEXB3", 2, 72, 57).expect("a legal geometry");
    let table = [Layout::V2, v3];

    // Version 2 is untouched: same stride, same header length, same blocks.
    let v2 = Layout::resolve(&table, 2).expect("still there");
    assert_eq!(v2, Layout::V2);
    assert_eq!(v2.record_stride(), 56);
    assert_eq!(v2.header_len(), 32_768);
    assert_eq!(v2.records_per_block(), 73);
    assert_eq!(v2.offset_of(145), Ok(40_888));

    // And version 3 is read at its own stride, not at version 2's.
    let read3 = Layout::resolve(&table, 3).expect("declared above");
    assert_eq!(read3.record_stride(), 72);
    assert_eq!(read3.block_len(), 72 * 57);
    assert_eq!(read3.offset_of(145), Ok(32_768 + 145 * 72));
    assert_ne!(read3.offset_of(145), v2.offset_of(145));

    // A version in neither table entry is still refused by number.
    assert_eq!(
        Layout::resolve(&table, 4),
        Err(FormatError::UnknownVersion(4)),
    );
    // The production table does not silently gain the hypothetical version.
    assert_eq!(
        Layout::for_version(3),
        Err(FormatError::UnknownVersion(3)),
        "KNOWN is the only table the read path consults",
    );
    // And a retired version cannot be resurrected by a caller's own table:
    // the retirement is checked before the table is searched.
    assert_eq!(
        Layout::resolve(&table, 1),
        Err(FormatError::RetiredVersion(1)),
    );
}

#[test]
fn a_degenerate_layout_is_refused_at_declaration() {
    let bad = |field| Err(FormatError::DegenerateLayout { field });
    assert_eq!(
        Layout::declare(0, MAGIC, 2, 56, 73),
        bad("version"),
        "zero is not a version a file can name",
    );
    assert_eq!(
        Layout::declare(1, *b"BRUTEXB1", 2, 56, 73),
        bad("version"),
        "a retired number is never reused for a new geometry",
    );
    assert_eq!(Layout::declare(3, MAGIC, 2, 0, 73), bad("record_stride"));
    assert_eq!(
        Layout::declare(3, MAGIC, 2, 56, 0),
        bad("records_per_block")
    );
    assert_eq!(
        Layout::declare(3, MAGIC, 2, u64::MAX, 2),
        bad("block_len"),
        "a block length that does not fit u64 is not a geometry",
    );
    assert_eq!(
        Layout::declare(3, *b"NOTBRUTE", 2, 56, 73),
        bad("magic"),
        "a magic outside the family is not this format",
    );
    // Every one of the seven family bytes, on its own. A check that only ever
    // sees a magic differing in several places cannot tell "all seven must
    // match" from "any one of them may".
    for position in 0..7usize {
        let mut magic = *b"BRUTEXB3";
        magic[position] ^= 0x20; // still ASCII, differs in exactly one byte
        assert_eq!(
            Layout::declare(3, magic, 2, 56, 73),
            bad("magic"),
            "family byte {position} was not required to match",
        );
    }
    // The eighth byte is the version's own, not the family's, so it is free.
    assert!(Layout::declare(3, *b"BRUTEXB3", 2, 56, 73).is_ok());
    assert!(Layout::declare(3, *b"BRUTEXBZ", 2, 56, 73).is_ok());
    assert_eq!(
        Layout::declare(3, MAGIC, 1, 56, 73),
        bad("slot_count"),
        "one slot cannot be updated without overwriting the previous commit",
    );
    assert_eq!(
        Layout::declare(3, MAGIC, 0, 56, 73),
        bad("slot_count"),
        "no slot at all is not a header",
    );
    assert_eq!(
        Layout::declare(3, MAGIC, MAX_SLOT_COUNT + 1, 56, 73),
        bad("slot_count"),
        "a slot past the family bound is a slot no reader looks at",
    );
    assert_eq!(
        Layout::declare(3, MAGIC, u64::MAX, 56, 73),
        bad("slot_count"),
    );
    assert_eq!(
        Layout::declare(3, MAGIC, 2, 56, 73).map(Layout::version),
        Ok(3)
    );
}

// ===========================================================================
// Geometry arithmetic that the geometry suite does not reach
// ===========================================================================

#[test]
fn an_overflowing_index_is_refused_not_wrapped() {
    // A wrapped offset addresses a DIFFERENT, valid-looking record. That is
    // worse than an error, so it must never happen silently.
    let v2 = Layout::V2;
    assert_eq!(v2.offset_of(u64::MAX), Err(FormatError::OffsetOverflow));

    // The multiply survives but the header add does not.
    let biggest_product = u64::MAX / RECORD_STRIDE;
    assert!(biggest_product.checked_mul(RECORD_STRIDE).is_some());
    assert_eq!(
        v2.offset_of(biggest_product),
        Err(FormatError::OffsetOverflow)
    );

    // The exact boundary: the largest index whose offset still fits.
    let last_ok = (u64::MAX - HEADER_LEN) / RECORD_STRIDE;
    assert!(v2.offset_of(last_ok).is_ok());
    assert_eq!(v2.offset_of(last_ok + 1), Err(FormatError::OffsetOverflow));

    // A record whose start fits but whose end does not.
    assert!(v2.record_byte_range(last_ok).is_err());
    assert_eq!(
        v2.record_byte_range(u64::MAX),
        Err(FormatError::OffsetOverflow),
    );
    assert_eq!(
        v2.record_byte_range(0),
        Ok((HEADER_LEN, HEADER_LEN + RECORD_STRIDE))
    );
}

#[test]
fn an_overflowing_block_is_refused_not_wrapped() {
    let v2 = Layout::V2;
    assert_eq!(
        v2.block_byte_range(0),
        Ok((HEADER_LEN, HEADER_LEN + BLOCK_LEN))
    );

    // Each of the three ways a block range can leave u64, nominal and
    // covered, so no arm is an arm nothing reaches.
    assert_eq!(
        v2.block_byte_range(u64::MAX),
        Err(FormatError::OffsetOverflow),
        "the block index times the block length overflows",
    );
    assert_eq!(
        v2.covered_byte_range(u64::MAX / RECORDS_PER_BLOCK, u64::MAX),
        Err(FormatError::OffsetOverflow),
        "committed, and still the product overflows",
    );

    let past_header = u64::MAX / BLOCK_LEN;
    assert!(past_header.checked_mul(BLOCK_LEN).is_some());
    assert_eq!(
        v2.block_byte_range(past_header),
        Err(FormatError::OffsetOverflow),
        "the product fits but adding the header does not",
    );
    assert_eq!(
        v2.covered_byte_range(past_header, u64::MAX),
        Err(FormatError::OffsetOverflow),
    );

    let last_block = (u64::MAX - HEADER_LEN - BLOCK_LEN) / BLOCK_LEN;
    assert!(v2.block_byte_range(last_block).is_ok());
    assert_eq!(
        v2.block_byte_range(last_block + 1),
        Err(FormatError::OffsetOverflow),
        "the start fits but the end does not",
    );
    assert_eq!(
        v2.covered_byte_range(last_block + 1, u64::MAX),
        Err(FormatError::OffsetOverflow),
    );

    // A one-byte block reaches the same arms from the other side.
    let thin = Layout::declare(3, *b"BRUTEXB3", 2, 1, 1).expect("a legal geometry");
    assert_eq!(thin.block_len(), 1);
    assert_eq!(thin.block_byte_range(0), Ok((HEADER_LEN, HEADER_LEN + 1)));
    assert_eq!(
        thin.block_byte_range(u64::MAX),
        Err(FormatError::OffsetOverflow),
    );
    assert_eq!(
        thin.block_byte_range(u64::MAX - HEADER_LEN),
        Err(FormatError::OffsetOverflow),
        "the start fits exactly and the end does not",
    );
}

#[test]
fn a_block_checksum_binds_the_bytes_and_not_the_position() {
    // PINNED, BECAUSE IT IS A PROPERTY AND NOT AN ACCIDENT. `block::seal` uses
    // the block index only to compute the covered length and to name an error;
    // the number it returns is `crc32c(bytes)` and nothing else. So two blocks
    // holding identical bytes seal to an identical checksum, and a block
    // transposed from another position -- or copied out of a DIFFERENT
    // instrument's file -- verifies clean.
    //
    // S-06 is about a flipped bit, and a flipped bit IS detected. Transposition
    // is a different threat and this format does not defend against it. Giving
    // it one means seeding the checksum with the block index, which changes the
    // bytes on disk: a new format version, never an edit to this one
    // (CLAUDE.md section 3.8). Recorded in docs/06-limits.md section 14, and
    // asserted here so the day someone changes it is a deliberate day.
    let v2 = Layout::V2;
    let records = 2 * 73;
    let block_bytes = vec![9u8; 73 * 56];
    let a = block::seal(v2, records, 0, &block_bytes).expect("block 0 is whole");
    let b = block::seal(v2, records, 1, &block_bytes).expect("block 1 is whole");
    assert_eq!(
        a, b,
        "the checksum is a pure function of the bytes -- position is not bound in",
    );

    // And it does still bind the BYTES: one flipped bit anywhere changes it.
    let mut flipped = block_bytes.clone();
    flipped[0] ^= 1;
    assert_ne!(
        block::seal(v2, records, 0, &flipped).expect("still whole"),
        a,
        "a flipped bit must change the checksum -- S-06",
    );
}

#[test]
fn capacity_and_ragged_tail_agree_with_the_bytes() {
    let v2 = Layout::V2;
    assert_eq!(v2.capacity_for(0), 0);
    assert_eq!(v2.capacity_for(HEADER_LEN - 1), 0);
    assert_eq!(v2.capacity_for(HEADER_LEN), 0);
    assert_eq!(v2.capacity_for(HEADER_LEN + 55), 0);
    assert_eq!(v2.capacity_for(HEADER_LEN + 56), 1);
    assert_eq!(v2.capacity_for(HEADER_LEN + 56 * 100), 100);

    // AND THE RAGGED TAIL, which this test is named for and never asked
    // about -- `ragged_tail_bytes` was exercised only in fault.rs. A name that
    // overstates its body is how docs/04-invariants.md acquires an unearned ✓,
    // because that file maps rows to tests BY NAME.
    assert_eq!(v2.ragged_tail_bytes(0), 0);
    assert_eq!(
        v2.ragged_tail_bytes(HEADER_LEN - 1),
        HEADER_LEN - 1,
        "a file shorter than its header is all tail"
    );
    assert_eq!(v2.ragged_tail_bytes(HEADER_LEN), 0);
    assert_eq!(v2.ragged_tail_bytes(HEADER_LEN + 55), 55);
    assert_eq!(v2.ragged_tail_bytes(HEADER_LEN + 56), 0);
    assert_eq!(v2.ragged_tail_bytes(HEADER_LEN + 57), 1);
    assert_eq!(v2.ragged_tail_bytes(HEADER_LEN + 56 * 100 + 13), 13);

    // The two agree: capacity counts the whole records, the tail is what is
    // left over, and together they account for every byte past the header.
    for len in [
        HEADER_LEN,
        HEADER_LEN + 1,
        HEADER_LEN + 55,
        HEADER_LEN + 56,
        HEADER_LEN + 4_242,
    ] {
        assert_eq!(
            v2.capacity_for(len) * 56 + v2.ragged_tail_bytes(len),
            len - HEADER_LEN,
            "capacity and tail must account for every byte at {len}"
        );
    }
}

#[test]
fn commits_alternate_between_the_slots() {
    let v2 = Layout::V2;
    assert_eq!(v2.slot_offset(0), 0);
    assert_eq!(v2.slot_offset(1), SLOT_STRIDE);
    assert_eq!(v2.slot_offset(2), 0);
    assert_eq!(v2.slot_offset(u64::MAX), SLOT_STRIDE);
}

// ===========================================================================
// The header slot
// ===========================================================================

/// Recomputes a slot's checksum after a byte has been patched.
///
/// The checksum covers every byte of the slot except the four it occupies, so
/// this is the same two ranges the decoder reads.
fn reseal(slot: &mut [u8; SLOT_LEN]) {
    let crc = crc32c_split(&slot[..56], &slot[60..]);
    slot[56..60].copy_from_slice(&crc.to_le_bytes());
}

#[test]
fn the_covered_domain_is_the_slot_minus_its_checksum() {
    // WHAT THIS PINS, AND WHY IT IS NOT A ROUND TRIP. The domain is 60 bytes
    // with a four-byte hole in the middle. The obvious simplification when a
    // kernel wants one contiguous buffer is to zero the checksum field and
    // cover all 64 bytes -- a DIFFERENT number over a DIFFERENT domain. Every
    // round-trip test in this file would still pass, because `reseal`
    // recomputes with whatever the domain has become; only a hardcoded value
    // can catch it. D-0032.
    let slot = genesis_slot();
    let stored = u32::from_le_bytes(slot[56..60].try_into().unwrap());

    let gathered: Vec<u8> = slot
        .iter()
        .take(56)
        .chain(slot.iter().skip(60))
        .copied()
        .collect();
    assert_eq!(gathered.len(), 60);
    assert_eq!(
        crc32c(&gathered),
        stored,
        "the header covers 0..56 || 60..64"
    );

    // The shortcut, spelled out so its answer is on the record as different.
    let mut filled = slot;
    filled[56..60].copy_from_slice(&[0u8; 4]);
    assert_ne!(
        crc32c(&filled),
        stored,
        "covering all 64 bytes with the field zeroed is a different domain"
    );

    // And an independent value for a 60-byte input, so the domain's LENGTH is
    // pinned to a constant rather than to whatever the slot happens to hold.
    assert_eq!(crc32c(&seq(60)), 0x3F69_148B);
}

fn genesis_slot() -> [u8; SLOT_LEN] {
    Header::genesis(7, 60, FLAG_CHECKSUMS)
        .commit()
        .expect("v2")
        .bytes
}

#[test]
fn a_slot_round_trips_every_field_at_its_documented_offset() {
    let header = Header {
        flags: FLAG_CHECKSUMS,
        generation: 0x0102_0304_0506_0708,
        n_valid: 4_242,
        first_ts_micros: 1_717_386_300_000_000,
        last_ts_micros: 1_717_389_300_000_000,
        symbol_id: 0x1234_5678,
        timeframe_secs: 60,
        ..Header::genesis(0, 0, 0)
    };
    let commit = header.commit().expect("v2");
    assert_eq!(Header::decode(&commit.bytes), Ok(header));
    assert_eq!(commit.header, header);

    // The bytes, at the offsets docs/02-store-format.md gives them.
    let b = &commit.bytes;
    assert_eq!(&b[0..8], &MAGIC[..]);
    assert_eq!(u16::from_le_bytes([b[8], b[9]]), 2, "format_version");
    assert_eq!(u16::from_le_bytes([b[10], b[11]]), 56, "record_stride");
    assert_eq!(
        u64::from_le_bytes(b[16..24].try_into().unwrap()),
        header.generation
    );
    assert_eq!(u64::from_le_bytes(b[24..32].try_into().unwrap()), 4_242);
    assert_eq!(
        i64::from_le_bytes(b[32..40].try_into().unwrap()),
        header.first_ts_micros,
        "first_ts_micros is written, not left at the epoch",
    );
    assert_eq!(
        i64::from_le_bytes(b[40..48].try_into().unwrap()),
        header.last_ts_micros
    );
    // OFFSETS 12, 48 AND 52 WERE NEVER ASSERTED. `flags`, `symbol_id` and
    // `timeframe_secs` are all named in docs/02-store-format.md and none of
    // them was pinned to a byte position -- and `symbol_id` and
    // `timeframe_secs` are adjacent `u32`s, so SWAPPING them in both `image`
    // and the offset constants changes the bytes on disk while
    // `decode(commit().bytes) == header` still holds. Every test passed.
    // Those two fields DID move between v1 and v2 (docs/02 records them at 40
    // and 44), so this is not hypothetical, and CLAUDE.md section 3.8 forbids
    // mutating a format in place.
    assert_eq!(
        u32::from_le_bytes(b[12..16].try_into().unwrap()),
        header.flags,
        "flags at offset 12"
    );
    assert_eq!(
        u32::from_le_bytes(b[48..52].try_into().unwrap()),
        header.symbol_id,
        "symbol_id at offset 48"
    );
    assert_eq!(
        u32::from_le_bytes(b[52..56].try_into().unwrap()),
        header.timeframe_secs,
        "timeframe_secs at offset 52"
    );
    // The two are distinguishable: a swap would make these equal and pass.
    assert_ne!(
        header.symbol_id, header.timeframe_secs,
        "the fixture must tell the two adjacent u32s apart"
    );
    assert_eq!(&b[60..64], &[0, 0, 0, 0], "reserved stays zero");

    // Idempotent: the same header always produces the same 64 bytes.
    assert_eq!(header.commit().expect("v2").bytes, commit.bytes);
}

#[test]
fn a_slot_caught_mid_write_fails_its_checksum_rather_than_mixing_two_commits() {
    // `decode` copies the 64 bytes once and checks the copy, so the checksum
    // covers exactly the bytes the returned Header carries. Over the
    // MAP_SHARED mapping the module endorses, with a writer pwriting the same
    // file, the observable consequence is this: at EVERY split point a
    // mixture of two images is either one of the two images exactly, or a
    // refusal. Never a third header that no checksum ever covered.
    let old = Header::genesis(7, 60, FLAG_CHECKSUMS);
    let new = old
        .advance(100, 1_717_386_300_000_000, 1_717_392_240_000_000)
        .expect("fits")
        .advance(50, 1_717_392_300_000_000, 1_717_395_240_000_000)
        .expect("fits");
    let old_bytes = old.commit().expect("v2").bytes;
    let new_bytes = new.commit().expect("v2").bytes;
    assert_eq!(old.generation % 2, new.generation % 2, "the same slot");

    let mut refusals = 0u32;
    for split in 0..=SLOT_LEN {
        let mut mixed = new_bytes;
        for (dst, src) in mixed
            .iter_mut()
            .skip(split)
            .zip(old_bytes.iter().skip(split))
        {
            *dst = *src;
        }
        match Header::decode(&mixed) {
            Ok(read) => assert!(
                read == old || read == new,
                "split {split} produced a header nobody ever wrote: {read:?}",
            ),
            Err(refusal) => {
                assert!(
                    matches!(refusal, FormatError::SlotChecksum { .. }),
                    "split {split}: {refusal}",
                );
                refusals += 1;
            }
        }
    }
    assert!(
        refusals > 0,
        "if no mixture is ever refused this test proves nothing",
    );
}

#[test]
fn a_slot_shorter_than_a_slot_is_refused() {
    assert_eq!(
        Header::decode(&[]),
        Err(FormatError::SlotTooShort { len: 0 })
    );
    assert_eq!(
        Header::decode(&[0u8; 63]),
        Err(FormatError::SlotTooShort { len: 63 }),
    );
    // Longer is fine: only the first slot's worth is read.
    let mut long = vec![0u8; 200];
    long[..SLOT_LEN].copy_from_slice(&genesis_slot());
    // Was `assert!(..).is_ok()`, which holds for a decoder that read the WRONG
    // window and happened to return a different-but-valid header. Pin it to
    // the genesis slot's own answer instead.
    assert_eq!(Header::decode(&long), Header::decode(&genesis_slot()));
}

#[test]
fn a_slot_that_is_not_a_bar_file_is_refused_before_its_version_is_read() {
    let mut slot = genesis_slot();
    slot[0] = b'X';
    assert_eq!(Header::decode(&slot), Err(FormatError::NotABarFile));
    assert_eq!(
        Header::decode(&[0u8; SLOT_LEN]),
        Err(FormatError::NotABarFile)
    );
}

#[test]
fn a_slot_naming_an_unknown_version_is_refused_by_number() {
    let mut slot = genesis_slot();
    // 7, NOT 9. The overlay sidecar took version 9, so a slot naming it is a
    // real geometry and this test needs one that is genuinely unknown.
    slot[8..10].copy_from_slice(&7u16.to_le_bytes());
    assert_eq!(Header::decode(&slot), Err(FormatError::UnknownVersion(7)));
}

#[test]
fn a_version_one_file_is_named_retired_rather_than_reported_as_destroyed() {
    // A file written to the geometry docs/02-store-format.md described before
    // this change: a 64-byte header, magic BRUTEXB1, version 1, stride 56,
    // n_valid at offset 16, an 8-byte header_crc at 48 over bytes 0..48.
    let mut v1 = vec![0u8; 64 + 10 * 56];
    v1[0..8].copy_from_slice(b"BRUTEXB1");
    v1[8..10].copy_from_slice(&1u16.to_le_bytes());
    v1[10..12].copy_from_slice(&56u16.to_le_bytes());
    v1[16..24].copy_from_slice(&10u64.to_le_bytes());

    // The version decides before the checksum is read, so the refusal names
    // the format rather than the damage. Reading it at version 2's offsets
    // would lift `generation` out of `n_valid` and hand back plausible
    // integers, and reporting NoValidHeader would be a false diagnosis of an
    // intact file.
    assert_eq!(Header::decode(&v1), Err(FormatError::RetiredVersion(1)));
    let len = u64::try_from(v1.len()).expect("fits");
    assert_eq!(
        Header::read_region(&v1, len),
        Err(FormatError::RetiredVersion(1)),
    );
    assert!(
        FormatError::RetiredVersion(1)
            .to_string()
            .contains("version 1"),
    );
}

#[test]
fn a_magic_that_disagrees_with_the_version_field_is_refused() {
    // Two independent statements of the same fact. When they differ the file
    // is not what either of them claims, and reading it at either version's
    // stride is a guess.
    let mut slot = genesis_slot();
    slot[7] = b'9';
    assert_eq!(
        Header::decode(&slot),
        Err(FormatError::MagicVersionMismatch(2)),
    );
}

#[test]
fn a_slot_whose_checksum_does_not_match_its_bytes_is_refused() {
    let mut slot = genesis_slot();
    let good = Header::decode(&slot).expect("v2");
    slot[24] ^= 0x01; // one bit of n_valid
    let refusal = Header::decode(&slot).expect_err("a flipped counter bit");
    assert!(matches!(refusal, FormatError::SlotChecksum { .. }));
    assert!(refusal.to_string().contains("checksum"));

    // Resealing makes it decode again -- which is the point: the checksum is
    // what stands between a flipped bit and a plausible wrong number.
    reseal(&mut slot);
    let tampered = Header::decode(&slot).expect("resealed");
    assert_ne!(tampered.n_valid, good.n_valid);
}

#[test]
fn a_slot_naming_the_wrong_stride_for_its_version_is_refused() {
    let mut slot = genesis_slot();
    slot[10..12].copy_from_slice(&64u16.to_le_bytes());
    reseal(&mut slot);
    assert_eq!(Header::decode(&slot), Err(FormatError::StrideMismatch(64)));
}

#[test]
fn a_header_this_build_cannot_write_is_refused_at_commit() {
    let unknown = Header {
        format_version: 7,
        ..Header::genesis(7, 60, 0)
    };
    assert_eq!(unknown.commit(), Err(FormatError::UnknownVersion(7)));

    let retired = Header {
        format_version: 1,
        ..Header::genesis(7, 60, 0)
    };
    assert_eq!(retired.commit(), Err(FormatError::RetiredVersion(1)));

    let wrong_stride = Header {
        record_stride: 64,
        ..Header::genesis(7, 60, 0)
    };
    assert_eq!(wrong_stride.commit(), Err(FormatError::StrideMismatch(64)));
    assert_eq!(
        wrong_stride.validate(Layout::V2, HEADER_LEN),
        Err(FormatError::StrideMismatch(64)),
    );
}

#[test]
fn a_genesis_header_describes_an_empty_file() {
    let header = Header::genesis(7, 60, FLAG_CHECKSUMS);
    assert_eq!(header.format_version, 2);
    assert_eq!(header.record_stride, 56);
    assert_eq!(header.generation, 0);
    assert_eq!(header.n_valid, 0);
    assert_eq!(header.symbol_id, 7);
    assert_eq!(header.timeframe_secs, 60);
    assert_eq!(header.flags & FLAG_CHECKSUMS, FLAG_CHECKSUMS);
    assert!(header.checksums_present());
    assert!(!Header::genesis(7, 60, 0).checksums_present());
    assert_eq!(header.validate(Layout::V2, HEADER_LEN), Ok(()));

    let commit = header.commit().expect("v2");
    assert_eq!((commit.slot, commit.offset), (0, 0));
    assert_eq!(commit.durable_through, HEADER_LEN);

    // The first append to an empty file is what sets first_ts_micros, and
    // nothing moves it afterwards.
    let first = header.advance(3, 99, 300).expect("fits");
    assert_eq!(
        first,
        Header {
            generation: 1,
            n_valid: 3,
            first_ts_micros: 99,
            last_ts_micros: 300,
            ..header
        },
    );
    let second = first.advance(2, 360, 420).expect("fits");
    assert_eq!(second.first_ts_micros, 99, "record 0 never moves");
    assert_eq!(second.last_ts_micros, 420);
    assert_eq!(second.n_valid, 5);
}

// ===========================================================================
// The block checksum
// ===========================================================================

#[test]
fn a_block_outside_the_commit_is_refused_by_both_doors() {
    // The geometry refuses before a byte is hashed, and `verify` propagates
    // that refusal rather than swallowing it into "the checksum did not
    // match" -- two different problems that would send an operator to two
    // different places.
    let v2 = Layout::V2;
    let header = Header::genesis(7, 60, FLAG_CHECKSUMS)
        .advance(1, 100, 100)
        .expect("fits");
    let record = [7u8; 56];
    let past = Err(FormatError::BlockNotCommitted {
        block: 1,
        blocks: 1,
    });

    assert_eq!(block::seal(v2, header.n_valid, 1, &record), past);
    assert_eq!(block::verify(&header, v2, 1, &record, 0), past.map(|_| ()));
    assert_eq!(
        block::verify(&header, v2, 0, &record[..55], 0),
        Err(FormatError::BlockLengthMismatch {
            block: 0,
            len: 55,
            need: 56,
        }),
    );

    // And the happy path, so this is not only a list of refusals: one record
    // committed, one block, and the seal is what verify accepts.
    let sealed = block::seal(v2, header.n_valid, 0, &record).expect("one record, one block");
    assert_eq!(block::verify(&header, v2, 0, &record, sealed), Ok(()));
    assert_eq!(
        block::seal(v2, header.n_valid, 0, &record),
        Ok(sealed),
        "sealing is idempotent, byte for byte",
    );
}

/// `count` distinct 56-byte records, laid end to end as the file holds them.
fn records(count: usize) -> Vec<u8> {
    (0..count * RECORD_LEN)
        .map(|at| u8::try_from((at * 31 + at / RECORD_LEN) % 251).expect("below 251"))
        .collect()
}

/// A sealed header at `n_valid` records, for `verify_through`.
fn sealed_at(n_valid: u64) -> Header {
    Header::genesis(7, 60, FLAG_CHECKSUMS)
        .advance(n_valid, 100, 100 + i64::try_from(n_valid).expect("small"))
        .expect("fits")
}

/// A tail entry sealed past the commit is admitted only on a positive proof.
///
/// Every checksum below is `crc32c` over bytes this test lays out itself, never
/// a number the function under test produced, so an acceptance is only ever the
/// stored number being the checksum of the committed bytes FOLLOWED BY records
/// handed in as `past`. D-0688.
#[test]
fn a_tail_checksum_sealed_past_the_commit_is_admitted_only_on_proof() {
    let v2 = Layout::V2;
    let file = records(80);
    let committed = &file[..2 * RECORD_LEN];
    let past = &file[2 * RECORD_LEN..5 * RECORD_LEN];
    let header = sealed_at(2);

    // THE COMMITTED EXTENT, and the interrupted append's two longer ones.
    assert_eq!(
        block::verify_through(&header, v2, 0, committed, past, crc32c(committed)),
        Ok(2),
        "a match at the commit needs no proof and names the commit"
    );
    for through in 3..=5usize {
        assert_eq!(
            block::verify_through(
                &header,
                v2,
                0,
                committed,
                past,
                crc32c(&file[..through * RECORD_LEN])
            ),
            Ok(u64::try_from(through).expect("small")),
            "sealed over {through} records, two of them committed"
        );
    }

    // `verify` is `verify_through` with nothing past the commit, so the same
    // entry that a proof admits is still refused there, and the number it
    // names is the COMMITTED extent's.
    let sealed_through_four = crc32c(&file[..4 * RECORD_LEN]);
    let refusal = Err(FormatError::BlockChecksum {
        block: 0,
        stored: sealed_through_four,
        computed: crc32c(committed),
    });
    assert_eq!(
        block::verify(&header, v2, 0, committed, sealed_through_four),
        refusal
    );
    assert_eq!(
        block::verify_through(
            &header,
            v2,
            0,
            committed,
            &past[..RECORD_LEN],
            sealed_through_four
        ),
        refusal.map(|()| 0),
        "records the file does not hold prove nothing"
    );
}

/// What the proof refuses: a torn record, a damaged committed byte, anything
/// past the block's nominal end, and a file born without checksums. D-0688.
#[test]
fn a_tail_checksum_the_proof_cannot_reach_is_still_refused() {
    let v2 = Layout::V2;
    let file = records(80);
    let committed = &file[..2 * RECORD_LEN];
    let past = &file[2 * RECORD_LEN..5 * RECORD_LEN];
    let header = sealed_at(2);
    let sealed_through_four = crc32c(&file[..4 * RECORD_LEN]);

    // A PARTIAL RECORD IS NEVER A CANDIDATE: no writer seals one.
    let torn = crc32c(&file[..3 * RECORD_LEN + 10]);
    assert!(
        matches!(
            block::verify_through(&header, v2, 0, committed, &past[..RECORD_LEN + 10], torn),
            Err(FormatError::BlockChecksum { .. })
        ),
        "a checksum over a torn record is refused"
    );

    // A FLIPPED COMMITTED BYTE is inside every candidate, so the extent that
    // was really sealed cannot match and the refusal names the damaged bytes.
    let mut damaged = committed.to_vec();
    damaged[20] ^= 0b0000_0100;
    assert_eq!(
        block::verify_through(&header, v2, 0, &damaged, past, sealed_through_four),
        Err(FormatError::BlockChecksum {
            block: 0,
            stored: sealed_through_four,
            computed: crc32c(&damaged),
        })
    );

    // NEVER PAST THE BLOCK'S NOMINAL END. 72 committed leaves room for one
    // record, so a checksum over 74 is refused although the bytes are there,
    // and a FULL block — the non-tail case — has no room at all.
    let at_72 = sealed_at(72);
    let first_block = &file[..72 * RECORD_LEN];
    let beyond = &file[72 * RECORD_LEN..74 * RECORD_LEN];
    assert_eq!(
        block::verify_through(
            &at_72,
            v2,
            0,
            first_block,
            beyond,
            crc32c(&file[..73 * RECORD_LEN])
        ),
        Ok(73),
        "the last record of the block is inside it"
    );
    assert!(
        matches!(
            block::verify_through(
                &at_72,
                v2,
                0,
                first_block,
                beyond,
                crc32c(&file[..74 * RECORD_LEN])
            ),
            Err(FormatError::BlockChecksum { .. })
        ),
        "a checksum reaching into the next block is not this block's"
    );
    let full = sealed_at(80);
    assert!(
        matches!(
            block::verify_through(
                &full,
                v2,
                0,
                &file[..73 * RECORD_LEN],
                &file[73 * RECORD_LEN..74 * RECORD_LEN],
                crc32c(&file[..74 * RECORD_LEN])
            ),
            Err(FormatError::BlockChecksum { .. })
        ),
        "a full block has no room past its end"
    );

    // AND NO FLAG, NO VERIFICATION, whatever is past the commit.
    let plain = Header::genesis(7, 60, 0)
        .advance(2, 100, 102)
        .expect("fits");
    assert_eq!(
        block::verify_through(&plain, v2, 0, committed, past, crc32c(committed)),
        Err(FormatError::ChecksumsAbsent)
    );
}

// ===========================================================================
// Paths
// ===========================================================================

fn parts(vendor: Vendor, symbol: &str) -> PathParts<'_> {
    PathParts {
        vendor,
        exchange: "NSE",
        segment: "INDEX",
        symbol,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2024, 6).expect("a real month"),
        file: FileKind::Bars,
    }
}

#[test]
fn vendor_prefix_isolated() {
    // X-12, LEXICALLY. Each vendor writes only under its own prefix, and no
    // vendor can reach another's -- as text. Enforced by construction: the
    // first segment is a Vendor rather than a string, the renderer writes it
    // before anything that varies, and no segment can hold a separator.
    // What this does NOT cover is a symlink; see StorePath::to_path_buf.
    let mut seen = HashSet::new();
    for vendor in Vendor::ALL {
        let path = StorePath::new(parts(vendor, "NIFTY")).expect("a legal path");
        let rendered = path.to_string();
        let components: Vec<&str> = rendered.split('/').collect();

        assert_eq!(components[0], STORE_ROOT, "the store root comes first");
        assert_eq!(
            components[1],
            vendor.as_str(),
            "the vendor is the first segment under the root",
        );
        assert_eq!(path.vendor(), vendor);
        assert_eq!(path.vendor_segment(), vendor.as_str());
        assert!(
            components
                .iter()
                .all(|c| !c.is_empty() && *c != ".." && *c != "."),
            "no component can climb: {rendered}",
        );
        assert!(seen.insert(rendered), "two vendors produced one path");
    }
    assert_eq!(seen.len(), Vendor::ALL.len());

    // The same instrument in two vendors differs only in that one segment, and
    // neither path is a prefix of the other's vendor directory.
    let groww = StorePath::new(parts(Vendor::Groww, "NIFTY"))
        .expect("legal")
        .to_string();
    let dhan = StorePath::new(parts(Vendor::Dhan, "NIFTY"))
        .expect("legal")
        .to_string();
    assert_eq!(groww, "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.bin");
    assert_eq!(dhan, "bars/dhan/NSE/INDEX/NIFTY/1min/2024-06.bin");
    assert!(!groww.starts_with("bars/dhan/"));
    assert!(!dhan.starts_with("bars/groww/"));

    // And on a real filesystem path, no component is a parent reference, so
    // joining onto a root cannot leave it -- lexically.
    let joined = StorePath::new(parts(Vendor::Groww, "NIFTY"))
        .expect("legal")
        .to_path_buf(Path::new("/var/lib/brutex"));
    assert_eq!(
        joined,
        Path::new("/var/lib/brutex/bars/groww/NSE/INDEX/NIFTY/1min/2024-06.bin"),
    );
    assert!(joined.starts_with("/var/lib/brutex/bars/groww"));
    assert!(
        joined
            .components()
            .all(|c| c != std::path::Component::ParentDir),
    );
}

#[test]
fn every_vendor_is_a_legal_segment() {
    // StorePath::new does not re-check the vendor: it is a closed set of
    // lower-case literals, and a runtime check would carry a refusal arm no
    // input could reach, which coverage could never close. This drives the
    // SAME check_segment every other segment goes through, over the whole
    // table, so the property is proven rather than assumed.
    for vendor in Vendor::ALL {
        assert_eq!(
            check_segment("vendor", vendor.as_str(), SegmentCase::Lower),
            Ok(()),
            "{} is not a legal path segment",
            vendor.as_str(),
        );
        assert!(vendor.as_str().len() <= MAX_VENDOR_LEN);
    }
    // 8, for `truedata`. It was 5 for `groww` until the archive feeds gained
    // store prefixes of their own — without one, `run_local` filed every GDFL
    // bar under `bars/dhan/`. The bound is the longest vendor segment and this
    // asserts it EXACTLY, so a drift in either direction fails here.
    assert_eq!(MAX_VENDOR_LEN, 8, "truedata");
    const { assert!(MAX_VENDOR_LEN <= MAX_SEGMENT_LEN) }

    // And the check is not vacuous: a vendor segment in the wrong case would
    // be refused if one ever appeared.
    assert_eq!(
        check_segment("vendor", "GROWW", SegmentCase::Lower),
        Err(PathError::NotCanonicalCase {
            field: "vendor",
            byte: b'G',
        }),
    );
}

#[test]
fn the_segment_cap_is_cores_symbol_capacity() {
    // A const assertion carries this; the numbers are written out so the
    // invariant table has a named test. Raising SYMBOL_CAPACITY without
    // raising MAX_SEGMENT_LEN would make the store path a second, quieter
    // limit on what can be stored -- and crates/core is not this crate's to
    // watch.
    assert_eq!(MAX_SEGMENT_LEN, SYMBOL_CAPACITY);
    let longest = "A".repeat(SYMBOL_CAPACITY);
    let symbol = Symbol::new(&longest).expect("core admits a symbol at its cap");
    assert!(
        StorePath::new(parts(Vendor::Groww, symbol.as_str())).is_ok(),
        "so must the path that has to hold it",
    );
}

#[test]
fn a_case_variant_is_refused_not_a_second_prefix() {
    // Measured: on this machine (APFS, case-insensitive) bars/groww/... and
    // bars/GROWW/... are ONE file, so writing the second destroys the first.
    // On the CI runner (ext4) they are TWO trees, which silently splits one
    // series in half. Same inputs, two different on-disk results chosen by
    // the host -- so neither variant is allowed to be constructible.
    assert_eq!(
        StorePath::new(parts(Vendor::Groww, "nifty")).err(),
        Some(PathError::NotCanonicalCase {
            field: "symbol",
            byte: b'n',
        }),
    );
    assert_eq!(
        StorePath::new(parts(Vendor::Groww, "NiFtY")).err(),
        Some(PathError::NotCanonicalCase {
            field: "symbol",
            byte: b'i',
        }),
    );
    for (field, hostile) in [
        (
            "exchange",
            PathParts {
                exchange: "nse",
                ..parts(Vendor::Groww, "NIFTY")
            },
        ),
        (
            "segment",
            PathParts {
                segment: "index",
                ..parts(Vendor::Groww, "NIFTY")
            },
        ),
    ] {
        let err = StorePath::new(hostile).expect_err("must refuse");
        assert!(
            err.to_string().contains(field) && err.to_string().contains("case"),
            "the refusal must name the segment and the reason, got {err}",
        );
    }

    // The vendor segment cannot be mis-cased at all: there is no field to put
    // "GROWW" in, because it is a Vendor and not a string. That is the door
    // X-12 needed closed -- code holding Vendor::Groww could previously write
    // `vendor: "dhan"` and nothing would refuse it.
    assert_eq!(
        StorePath::new(parts(Vendor::Groww, "NIFTY"))
            .expect("legal")
            .vendor(),
        Vendor::Groww,
    );

    // And core's normalisation is no longer routed around: `for_key` takes a
    // Symbol, which upper-cases, so one instrument has exactly one path.
    let month = YearMonth::new(2024, 6).expect("a real month");
    let lower = InstrumentKey::index(Exchange::Nse, "nifty").expect("core normalises");
    let upper = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("a real index");
    assert_eq!(lower, upper);
    assert_eq!(
        StorePath::for_key(
            Vendor::Groww,
            &lower,
            Timeframe::MINUTE_1,
            month,
            FileKind::Bars
        )
        .expect("legal")
        .to_string(),
        "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.bin",
    );
}

#[test]
fn every_byte_class_the_allowlist_admits_is_actually_accepted() {
    // THE ACCEPTING SIDE OF THE ALLOWLIST, which nothing exercised. The byte
    // filter admits `A-Z a-z 0-9 - _ &`, and every segment any store test ever
    // built was purely alphabetic -- "NIFTY", "RELIANCE", "A".repeat(n). So
    // deleting `| b'&'` or `| b'0'..=b'9'` from that pattern left the entire
    // suite green, while `&` is load-bearing for a real NSE symbol: `M&M` is
    // an F&O underlying and a NIFTY Total Market constituent. The api crate
    // tests it; the path type never did.
    for accepted in [
        "M&M",        // ampersand -- a real listed company
        "BAJAJ-AUTO", // hyphen -- a real listed company
        "NIFTY50",    // digits
        "L_T",        // underscore
        "M&M-FIN2",   // every admitted class at once
    ] {
        assert!(
            StorePath::new(parts(Vendor::Groww, accepted)).is_ok(),
            "symbol {accepted:?} uses only admitted bytes and must be accepted",
        );
    }
}

#[test]
fn a_segment_that_could_escape_the_vendor_prefix_is_refused() {
    // Each refusal, by name. Nothing is sanitised: silently stripping a
    // character maps two different instruments onto one path, which is the
    // exact failure the vendor prefix exists to prevent.
    let cases: [(&str, PathError); 9] = [
        ("", PathError::EmptySegment { field: "symbol" }),
        ("..", PathError::Traversal { field: "symbol" }),
        (".", PathError::Traversal { field: "symbol" }),
        (
            "a/b",
            PathError::IllegalByte {
                field: "symbol",
                byte: b'/',
            },
        ),
        (
            "/etc",
            PathError::IllegalByte {
                field: "symbol",
                byte: b'/',
            },
        ),
        (
            "..\\..",
            PathError::IllegalByte {
                field: "symbol",
                byte: b'.',
            },
        ),
        (
            "a\0b",
            PathError::IllegalByte {
                field: "symbol",
                byte: 0,
            },
        ),
        (
            "NIFTY 50",
            PathError::IllegalByte {
                field: "symbol",
                byte: b' ',
            },
        ),
        (
            "2024-06.bin",
            PathError::IllegalByte {
                field: "symbol",
                byte: b'.',
            },
        ),
    ];
    for (symbol, expected) in cases {
        assert_eq!(
            StorePath::new(parts(Vendor::Groww, symbol)).err(),
            Some(expected),
            "symbol {symbol:?} was not refused as expected",
        );
    }

    // Over-long, on the field whose cap is load-bearing.
    let long = "A".repeat(MAX_SEGMENT_LEN + 1);
    assert_eq!(
        StorePath::new(parts(Vendor::Groww, &long)).err(),
        Some(PathError::SegmentTooLong {
            field: "symbol",
            len: MAX_SEGMENT_LEN + 1,
        }),
    );
    assert!(StorePath::new(parts(Vendor::Groww, &"A".repeat(MAX_SEGMENT_LEN))).is_ok());

    // Every caller-supplied segment is checked, not only the symbol.
    for (field, hostile) in [
        (
            "exchange",
            PathParts {
                exchange: "..",
                ..parts(Vendor::Groww, "NIFTY")
            },
        ),
        (
            "segment",
            PathParts {
                segment: "",
                ..parts(Vendor::Groww, "NIFTY")
            },
        ),
        (
            "symbol",
            PathParts {
                symbol: "../DHAN",
                contract: None,
                ..parts(Vendor::Groww, "NIFTY")
            },
        ),
    ] {
        let err = StorePath::new(hostile).expect_err("must refuse");
        assert!(
            err.to_string().contains(field),
            "the refusal must name the segment, got {err}",
        );
    }
}

#[test]
fn a_timeframe_and_a_month_are_values_not_strings() {
    assert_eq!(Timeframe::from_secs(60), Ok(Timeframe::MINUTE_1));
    assert_eq!(Timeframe::MINUTE_1.secs(), 60);
    assert_eq!(Timeframe::MINUTE_1.as_str(), "1min");

    // THE DAILY RUNG, added because a backfill lands it first: 14 windows per
    // instrument against 81 at one minute, for the same 2020-to-yesterday span.
    // D-0015 built the seam and D-0054 uses it — `timeframe_secs` was already
    // a u32 of seconds and the path already had a `<tf>` segment, so this is a
    // new directory and nothing else.
    assert_eq!(Timeframe::from_secs(86_400), Ok(Timeframe::DAY_1));
    assert_eq!(Timeframe::DAY_1.secs(), 86_400);
    assert_eq!(Timeframe::DAY_1.as_str(), "1day");

    // Every rung, and NO OTHERS. Asserted as the whole list rather than as a
    // handful of `contains` calls, so a rung added without a decision entry
    // fails here rather than appearing quietly in a path. The intraday rungs
    // arrived with D-0077.
    assert_eq!(
        Timeframe::KNOWN,
        &[
            Timeframe::DAY_1,
            Timeframe::SECOND_1,
            Timeframe::MINUTE_1,
            Timeframe::MINUTE_2,
            Timeframe::MINUTE_3,
            Timeframe::MINUTE_5,
            Timeframe::MINUTE_10,
            Timeframe::MINUTE_15,
            Timeframe::MINUTE_30,
            Timeframe::MINUTE_60,
        ],
        "every timeframe this build stores, in the order a backfill writes them"
    );

    // AND THEY DO NOT COLLIDE. Two rungs sharing a path segment would file one
    // over the other; two sharing a length would make `from_secs` ambiguous.
    assert_ne!(Timeframe::DAY_1.as_str(), Timeframe::MINUTE_1.as_str());
    assert_ne!(Timeframe::DAY_1.secs(), Timeframe::MINUTE_1.secs());
    assert!(
        Timeframe::DAY_1.as_str().len() <= MAX_TIMEFRAME_LEN,
        "`1day` is four bytes; D-0077's `15min` is the five that sets the bound"
    );
    // D-0077 added the intraday rungs, so 300s is now the legal `5min` and no
    // longer a refusal. 45s is absent from the table and takes its place.
    assert_eq!(
        Timeframe::from_secs(45),
        Err(PathError::UnknownTimeframe { secs: 45 }),
    );
    assert_eq!(Timeframe::from_secs(300), Ok(Timeframe::MINUTE_5));
    assert_eq!(
        Timeframe::from_secs(0),
        Err(PathError::UnknownTimeframe { secs: 0 }),
    );

    let june = YearMonth::new(2024, 6).expect("a real month");
    assert_eq!((june.year(), june.month()), (2024, 6));
    assert_eq!(june.to_string(), "2024-06");
    assert_eq!(
        YearMonth::new(1999, 12).expect("a real month").to_string(),
        "1999-12",
    );
    assert_eq!(
        YearMonth::new(2024, 0),
        Err(PathError::MonthOutOfRange { month: 0 }),
    );
    assert_eq!(
        YearMonth::new(2024, 13),
        Err(PathError::MonthOutOfRange { month: 13 }),
    );
    // No bar can predate the epoch its timestamp counts from.
    assert_eq!(
        YearMonth::new(1969, 1),
        Err(PathError::YearOutOfRange { year: 1969 }),
    );
    assert_eq!(
        YearMonth::new(10_000, 1),
        Err(PathError::YearOutOfRange { year: 10_000 }),
    );
    assert!(YearMonth::new(1970, 1).is_ok());
    assert!(YearMonth::new(9999, 12).is_ok());
}

#[test]
fn the_sibling_files_of_a_month_share_every_segment_but_the_extension() {
    let base = parts(Vendor::Groww, "NIFTY");
    let mut rendered = Vec::new();
    for file in FileKind::ALL {
        let path = StorePath::new(PathParts { file, ..base }).expect("legal");
        assert_eq!(path.file(), file);
        rendered.push(path.to_string());
    }
    assert_eq!(
        rendered,
        vec![
            "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.bin".to_owned(),
            "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.crc".to_owned(),
            "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.ovl".to_owned(),
            "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.grk".to_owned(),
            "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.lock".to_owned(),
            "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.ovl.crc".to_owned(),
            "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.grk.crc".to_owned(),
        ],
    );

    // The lock is a sibling formed by the one renderer, not a string a caller
    // concatenates -- docs/02 §9's "one writer per file, enforced by an
    // advisory lock" needs a name that cannot drift from the file it guards.
    assert_eq!(FileKind::Lock.extension(), ".lock");
    // Each of the three record families has an independent integrity file;
    // the existing month lock still serializes their writers.
    assert_eq!(FileKind::ALL.len(), 7);

    let bars = StorePath::new(base).expect("legal");
    assert_eq!(bars.timeframe(), Timeframe::MINUTE_1);
    assert_eq!(bars.month(), YearMonth::new(2024, 6).expect("a real month"));
}

#[test]
fn a_path_is_built_from_the_canonical_identity_not_from_loose_strings() {
    let month = YearMonth::new(2024, 6).expect("a real month");
    let nifty = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("a real index");
    let path = StorePath::for_key(
        Vendor::Groww,
        &nifty,
        Timeframe::MINUTE_1,
        month,
        FileKind::Bars,
    )
    .expect("a legal path");
    assert_eq!(
        path.to_string(),
        "bars/groww/NSE/INDEX/NIFTY/1min/2024-06.bin"
    );

    let equity = InstrumentKey {
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        underlying: Symbol::new("RELIANCE").expect("a real symbol"),
        kind: Kind::Equity,
    };
    assert_eq!(
        StorePath::for_key(
            Vendor::Dhan,
            &equity,
            Timeframe::MINUTE_1,
            month,
            FileKind::Bars
        )
        .expect("a legal path")
        .to_string(),
        "bars/dhan/NSE/CASH/RELIANCE/1min/2024-06.bin",
    );

    // A future and an option are filed under their CONTRACT, which this
    // builder does not form -- so it refuses by name rather than filing every
    // expiry under the underlying and silently merging distinct series.
    let expiry = Expiry::new(2024, 6, 27).expect("a real expiry");
    // AND THE CONTRACT SEGMENT NOW EXISTS, which is what D-0019 asked for and
    // what this used to assert the ABSENCE of. Each derivative gets its own
    // directory under the underlying, so two expiries of one symbol — and two
    // strikes of one expiry — can never share a bar file.
    for (kind, expected) in [
        (Kind::Future { expiry }, "2024-06-27-FUT"),
        (
            Kind::Option {
                expiry,
                strike: Paisa::from_raw(2_500_000),
                side: OptionSide::Call,
            },
            "2024-06-27-2500000-CE",
        ),
    ] {
        let contract = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("NIFTY").expect("a real symbol"),
            kind,
        };
        let path = StorePath::for_key(
            Vendor::Groww,
            &contract,
            Timeframe::MINUTE_1,
            month,
            FileKind::Bars,
        )
        .expect("a derivative has a contract path now");
        assert_eq!(
            path.to_string(),
            format!("bars/groww/NSE/FNO/NIFTY/{expected}/1min/2024-06.bin"),
            "the contract is a segment of its OWN, below the underlying"
        );
        // THE STRIKE IS PAISA AND CARRIES NO DECIMAL POINT. `CLAUDE.md` §7:
        // prices are i64 paisa, never a float. 2,500,000 paisa is 25,000
        // rupees, and rendering it as `25000.00` would put a `.` in a path
        // segment and invite a reader to parse it back as one.
        assert!(
            !expected.contains('.'),
            "a strike rendered with a decimal point invites a reader to parse \
             it back as a float: {expected}"
        );
    }
}

#[test]
fn a_maximal_path_fits_the_declared_bound() {
    let longest = "A".repeat(MAX_SEGMENT_LEN);
    // The widest vendor and the longest extension, found in their tables
    // rather than written down, so the bound cannot drift away from them.
    let widest = Vendor::ALL
        .into_iter()
        .max_by_key(|v| v.as_str().len())
        .expect("a non-empty table");
    assert_eq!(widest.as_str().len(), MAX_VENDOR_LEN);
    let fattest = FileKind::ALL
        .into_iter()
        .max_by_key(|f| f.extension().len())
        .expect("a non-empty table");
    assert_eq!(fattest.extension().len(), MAX_EXTENSION_LEN);
    // And the longest RUNG, found the same way. It was hardcoded to `1min`
    // while every rung was four bytes, which made the maximal path one byte
    // short of MAX_LEN the moment D-0077 added `15min`.
    let slowest_name = Timeframe::KNOWN
        .iter()
        .copied()
        .max_by_key(|tf| tf.as_str().len())
        .expect("a non-empty table");
    assert_eq!(slowest_name.as_str().len(), MAX_TIMEFRAME_LEN);

    let path = StorePath::new(PathParts {
        vendor: widest,
        exchange: &longest,
        segment: &longest,
        symbol: &longest,
        contract: None,
        timeframe: slowest_name,
        month: YearMonth::new(9999, 12).expect("a real month"),
        file: fattest,
    })
    .expect("every segment is at the cap");

    // Exactly, not merely within: the bound is the length of the longest legal
    // path, so a bound that drifted in either direction fails here.
    assert_eq!(path.to_string().len(), MAX_LEN);
    // The independent .ovl.crc and .grk.crc siblings add three bytes to the
    // previous 107-byte bound. Keep the exact maximum pinned to its derivation.
    assert_eq!(MAX_LEN, 110);
    assert_eq!(
        path.to_string(),
        format!(
            "bars/{}/{longest}/{longest}/{longest}/{}/9999-12{}",
            widest.as_str(),
            slowest_name.as_str(),
            fattest.extension(),
        ),
    );

    // The timeframe bound is the table's, not the segment cap's.
    assert!(
        Timeframe::KNOWN
            .iter()
            .all(|tf| tf.as_str().len() <= MAX_TIMEFRAME_LEN),
    );
    assert!(
        Timeframe::KNOWN
            .iter()
            .any(|tf| tf.as_str().len() == MAX_TIMEFRAME_LEN),
        "the bound must be reached by something, or it is not a bound",
    );

    // Every shorter path is shorter, so the bound really is the maximum.
    assert!(
        StorePath::new(parts(Vendor::Groww, "NIFTY"))
            .expect("legal")
            .to_string()
            .len()
            < MAX_LEN,
    );
}

// ===========================================================================
// Every refusal says something different
// ===========================================================================

#[test]
fn every_format_error_renders_a_distinct_reason() {
    let all = [
        FormatError::OffsetOverflow,
        FormatError::SlotTooShort { len: 63 },
        // A short RECORD and a short SLOT are two different files being wrong
        // in two different places, and the enum says so -- but until D-0039
        // this list omitted the record arm, so `Display` for it had never
        // run. An arm nobody renders is an arm free to render as another
        // arm's message.
        FormatError::RecordTooShort { len: 55 },
        FormatError::HeaderRegionTooShort { slots: 1, need: 2 },
        FormatError::NotABarFile,
        FormatError::UnknownVersion(7),
        FormatError::RetiredVersion(7),
        FormatError::MagicVersionMismatch(3),
        FormatError::StrideMismatch(64),
        FormatError::TooShortForHeader,
        FormatError::CounterExceedsFile,
        FormatError::CounterOverflow,
        FormatError::GenerationExhausted,
        FormatError::TimestampsOutOfOrder {
            previous: 9,
            next: 8,
        },
        FormatError::SlotChecksum {
            stored: 1,
            computed: 2,
        },
        FormatError::BlockChecksum {
            block: 4,
            stored: 1,
            computed: 2,
        },
        FormatError::BlockNotCommitted {
            block: 4,
            blocks: 3,
        },
        FormatError::BlockLengthMismatch {
            block: 4,
            len: 55,
            need: 56,
        },
        FormatError::ChecksumsAbsent,
        FormatError::SlotPositionMismatch {
            expected: 1,
            found: 0,
        },
        FormatError::DegenerateLayout { field: "magic" },
        FormatError::NoValidHeader,
    ];
    assert_distinct(&all);
    assert_eq!(all.len(), 22, "every variant the enum has, rendered");
    assert!(all[1].to_string().contains("63"), "the length is visible");
    assert!(all[2].to_string().contains("55"), "the length is visible");
    assert_ne!(
        all[1].to_string(),
        all[2].to_string(),
        "a short slot and a short record must not share one message",
    );
    assert!(all[5].to_string().contains('7'), "the version is visible");
    assert!(all[6].to_string().contains('7'), "the version is visible");
    assert!(all[8].to_string().contains("64"), "the stride is visible");
    assert!(all[15].to_string().contains('4'), "the block is visible");
    assert!(
        all[20].to_string().contains("magic"),
        "the field is visible"
    );
    let as_error: &dyn std::error::Error = &FormatError::NotABarFile;
    assert!(as_error.source().is_none());
}

#[test]
fn every_path_error_renders_a_distinct_reason() {
    let all = [
        PathError::EmptySegment { field: "vendor" },
        PathError::Traversal { field: "vendor" },
        PathError::IllegalByte {
            field: "vendor",
            byte: b'/',
        },
        PathError::NotCanonicalCase {
            field: "vendor",
            byte: b'G',
        },
        PathError::SegmentTooLong {
            field: "vendor",
            len: 99,
        },
        PathError::UnknownTimeframe { secs: 300 },
        PathError::MonthOutOfRange { month: 13 },
        PathError::YearOutOfRange { year: 1969 },
        PathError::ContractPathUnsupported,
    ];
    assert_distinct(&all);
    assert!(all[4].to_string().contains("99"), "the length is visible");
    assert!(
        all[5].to_string().contains("300"),
        "the seconds are visible"
    );
    assert!(all[3].to_string().contains("case"), "the reason is visible");
    let as_error: &dyn std::error::Error = &PathError::ContractPathUnsupported;
    assert!(as_error.source().is_none());
}

/// Every rendering differs from every other, and none is empty.
fn assert_distinct<E: std::fmt::Display + std::fmt::Debug>(all: &[E]) {
    let rendered: Vec<String> = all.iter().map(ToString::to_string).collect();
    for (i, a) in rendered.iter().enumerate() {
        assert!(!a.is_empty(), "variant {i} renders as nothing");
        assert!(
            !format!("{:?}", all[i]).is_empty(),
            "variant {i} has no Debug",
        );
        for (j, b) in rendered.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "variants {i} and {j} render identically");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The intraday rungs added beside 1min and 1day. D-0015 built the seam for
// exactly this: a rung is a directory name and a `timeframe_secs`, so nothing
// below the path layer changes.
// ---------------------------------------------------------------------------

/// Every rung's name and seconds agree, and `from_secs` finds each one.
#[test]
fn every_known_timeframe_round_trips_through_its_own_seconds() {
    for tf in Timeframe::KNOWN {
        let found = Timeframe::from_secs(tf.secs()).expect("a KNOWN rung resolves by its seconds");
        assert_eq!(found, *tf, "{} did not resolve to itself", tf.as_str());
    }
}

/// No two rungs share a length or a directory name. A collision would put two
/// bar lengths in one directory, and the header would be the only thing that
/// disagreed.
#[test]
fn no_two_timeframes_share_a_length_or_a_name() {
    let n = Timeframe::KNOWN.len();
    for i in 0..n {
        for j in (i + 1)..n {
            let (a, b) = (Timeframe::KNOWN[i], Timeframe::KNOWN[j]);
            assert_ne!(
                a.secs(),
                b.secs(),
                "{} and {} share a length",
                a.as_str(),
                b.as_str()
            );
            assert_ne!(
                a.as_str(),
                b.as_str(),
                "two rungs share the name {}",
                a.as_str()
            );
        }
    }
}

/// An unknown length is refused by name rather than given a derived directory.
/// `docs/02-store-format.md` — a directory no reader looks for is data written
/// into a hole.
#[test]
fn an_unlisted_length_is_refused_and_not_invented() {
    // 120 LEFT THIS LIST BECAUSE IT BECAME A RUNG. It was here as "a plausible
    // length nobody stores"; two minutes is stored now, so asserting it is
    // refused would assert the opposite of the table. 90 and 240 take its place
    // — still plausible, still absent, and still refused by name.
    // 1 LEFT THIS LIST BECAUSE IT BECAME A RUNG, exactly as 120 did before it:
    // one second is what the archive feeds' files hold and the store carries it
    // now. 2 and 45 take its place — still plausible, still absent, still
    // refused by name.
    for secs in [0_u32, 2, 45, 90, 240, 7_200, 86_399] {
        assert!(
            Timeframe::from_secs(secs).is_err(),
            "{secs}s resolved to a rung that is not in KNOWN"
        );
    }
}

/// The alignment split the gap-leg rule depends on. The fold grid is anchored
/// at IST midnight and the open is 555 minutes past it, so a rung aligns
/// exactly when its length divides 555. This is arithmetic, not a convention,
/// and a rule that reads "the first candle of the day" reads a STUB on the two
/// rungs that fail it.
#[test]
fn only_the_rungs_that_divide_555_start_a_session_on_time() {
    assert!(Timeframe::MINUTE_1.aligns_with_the_open());
    assert!(Timeframe::MINUTE_3.aligns_with_the_open(), "555/3 = 185");
    assert!(Timeframe::MINUTE_5.aligns_with_the_open(), "555/5 = 111");
    assert!(Timeframe::MINUTE_15.aligns_with_the_open(), "555/15 = 37");

    assert!(
        !Timeframe::MINUTE_30.aligns_with_the_open(),
        "555/30 = 18.5"
    );
    assert!(
        !Timeframe::MINUTE_60.aligns_with_the_open(),
        "555/60 = 9.25"
    );
    assert!(
        !Timeframe::DAY_1.aligns_with_the_open(),
        "a day does not start at 09:15"
    );

    // And the property the assertions above are instances of.
    for tf in Timeframe::KNOWN {
        let by_arithmetic = tf.secs() % 60 == 0 && 555 % (tf.secs() / 60) == 0;
        assert_eq!(
            tf.aligns_with_the_open(),
            by_arithmetic,
            "{} disagrees with 555 % minutes == 0",
            tf.as_str()
        );
    }
}

/// **Every rung's length, pinned exactly.** `MINUTE_30` could be changed from
/// 1,800 seconds to 1,860 and every other test still passed — a 31-minute bar
/// filed in the directory called `30min`, with the header agreeing and nothing
/// disagreeing. The name and the arithmetic must both be nailed, because the
/// directory name is what a reader trusts and the seconds are what the fold uses.
#[test]
fn every_rung_length_and_name_is_pinned_exactly() {
    for (tf, secs, name) in [
        (Timeframe::SECOND_1, 1_u32, "1s"),
        (Timeframe::MINUTE_1, 60, "1min"),
        (Timeframe::MINUTE_2, 120, "2min"),
        (Timeframe::MINUTE_3, 180, "3min"),
        (Timeframe::MINUTE_5, 300, "5min"),
        (Timeframe::MINUTE_10, 600, "10min"),
        (Timeframe::MINUTE_15, 900, "15min"),
        (Timeframe::MINUTE_30, 1_800, "30min"),
        (Timeframe::MINUTE_60, 3_600, "60min"),
        (Timeframe::DAY_1, 86_400, "1day"),
    ] {
        assert_eq!(tf.secs(), secs, "{name} is not {secs} seconds");
        assert_eq!(tf.as_str(), name, "the rung of {secs}s is not named {name}");
        // And the name agrees with the arithmetic, so a renamed rung is caught
        // even if its seconds are right.
        if name.ends_with("min") {
            let minutes: u32 = name
                .trim_end_matches("min")
                .parse()
                .expect("a minute count");
            assert_eq!(minutes * 60, secs, "{name} does not mean {minutes} minutes");
        }
    }
    assert_eq!(
        Timeframe::KNOWN.len(),
        10,
        "a rung was added or removed; D-0077 is the entry that has to change"
    );
    // EVERY RUNG IN THE TABLE IS ALSO IN `KNOWN`, and the count above only
    // catches a rung added to one of the two. This catches it added to the
    // other — a `MINUTE_2` const with no `KNOWN` entry is a directory name
    // `from_secs` can never return, which is data written into a hole.
    for (tf, _, name) in [
        (Timeframe::SECOND_1, 1_u32, "1s"),
        (Timeframe::MINUTE_2, 120, "2min"),
        (Timeframe::MINUTE_10, 600, "10min"),
    ] {
        assert!(
            Timeframe::KNOWN.contains(&tf),
            "{name} is a const with no place in KNOWN"
        );
    }
}
