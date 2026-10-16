//! **FRAMES THE OLD DECODER'S OWN ENCODER WROTE, KEPT AS BYTES.**
//!
//! `ruzstd` moved from 0.8.2 to 0.9.1 to clear RUSTSEC-2026-0344 (D-5090). The
//! promise that bump has to keep is that every frame the old version read, the
//! new one reads to the SAME bytes. That cannot be proved by encoding in the
//! test, because the test links only the new version; so the frames below were
//! written once by `ruzstd` 0.8.2's `compress_to_vec` (at `Fastest`, and one at
//! `Uncompressed`), before the bump, and are frozen here as hex. Each decodes
//! to [`input`], which the test regenerates — so a byte that drifted in either
//! the frame or the decoder is a failed comparison, not a plausible-looking
//! page.
//!
//! **How a second agent repeats that.** A throwaway crate outside the
//! repository that names `ruzstd = "=0.8.2"` and includes this file by
//! `#[path]` prints `compress_to_vec(input(name), level)` as hex for each case;
//! all six equal the strings below (measured 2026-10-16, D-5090).
//!
//! **Every input is invented.** Runs of one byte, counter text, xorshift noise
//! and a numeric byte-plane payload shaped like the tick store's columnar one
//! (`/Volumes/WD_BLACK/brutex/tickstore-data/FORMAT.md` §4). No vendor row, no
//! price, nothing that could be mistaken for market data.
//!
//! **One file, two readers.** `lake`'s page path and `pull`'s tick-store
//! `unzstd` are the workspace's only two decode call sites, and each must be
//! held to the same frames; `pull`'s test module includes this file by
//! `#[path]` rather than keeping a second copy that could drift from this one.
//!
//! The crafted frames at the end are not round trips. Each pins one decision a
//! call site makes about a frame the encoder would never write: a block wider
//! than its window (the advisory's own class, refused by 0.9.1 and decoded by
//! 0.8.2) and the window ceiling. **0.8.2 had no window ceiling at either call
//! site, measured:** its 100 MiB `MAXIMUM_ALLOWED_WINDOW_SIZE` was checked only
//! when a decoder was `reset` for a second frame, and both call sites build a
//! fresh decoder per frame, so a frame declaring an 8 GiB window was accepted
//! and its whole output could be held before one byte became collectable. Both
//! call sites now pass [`WINDOW_CEILING`] explicitly; it equals 0.9.1's own
//! `DEFAULT_MAX_WINDOW_SIZE`, so the number is not this repository's guess, and
//! naming it means a later `ruzstd` that moves its default cannot move ours.
#![allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test-only fixtures: a fixture that cannot panic cannot fail"
)]

/// The four bytes every ZSTD frame starts with (RFC 8878 §3.1.1).
pub(crate) const MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

/// One synthetic input and the frame `ruzstd` 0.8.2 wrote for it.
pub(crate) struct Case {
    /// The input's name; [`input`] generates it from this.
    pub(crate) name: &'static str,
    /// The level 0.8.2's `compress_to_vec` was given: `true` for
    /// `Uncompressed` (the only level that writes a Raw block with content),
    /// `false` for `Fastest`.
    pub(crate) uncompressed: bool,
    /// The frame, as lowercase hex, exactly as 0.8.2 wrote it.
    pub(crate) hex: &'static str,
}

/// The frames, one per input. Together they hold every block type the format
/// has (Raw, RLE, Compressed), a frame of no content, and inputs that cross the
/// 128 KiB block boundary; `the_fixtures_cover_every_block_type` checks that by
/// reading the block headers rather than trusting this sentence.
pub(crate) const CASES: [Case; 6] = [
    Case {
        name: "empty",
        uncompressed: false,
        hex: "28b52ffd043801000099e9d851",
    },
    Case {
        name: "run",
        uncompressed: false,
        hex: "28b52ffd04380200103702001037039f0437a76a63c2",
    },
    Case {
        name: "text",
        uncompressed: false,
        hex: "28b52ffd04388407009c050073796e74686574696320726f7720303020616c70686120626574612067616d6d61206465\
            6c74610a3132333435363738393131313131313131313132323232323232323232333333333333333333333434343434\
            343434343439a8007dffef11dc3010ceea24e911fc0ac1ff2784e14f6ce9ef1ba0afa6fdf901bd5aede707fa5fdde707\
            3df5eff9a14fedfce8bd373f7d0d69438a216fc81832861c43c69035640cb96689596a969a65cd52b3d42c35cb9aa566\
            9946ae91d2c86a241a398dac46a291d3c86a249a4c93d6e49a549368d29a5c936a124d9a7ded0da90ddc9c69291b5ded\
            4153d002d6c60280144507000c0600612064656c74610a73796e74686574696320726f7720323720616c706861206265\
            74612067616d6d3839333033313332333333343335333633333334343434343434343434303030303030303030303131\
            31313131313131313232323232323236a8518cbeff6f11dc3010cee27511fc7f42f0cbedfd49076772d05715f4fcd0ab\
            2a747ef4ffd0fcf430a40c5943da9034a40d7943de9037a41bd266b1596e169bc566b1596e969be56671b3b891dbc86e\
            241bb98dec46b291dbc86e241bb94db24936e94dbe4937c926bdc937e926f99a75686fd8ceaf4bc691d83de014b28072\
            600140153661d240",
    },
    Case {
        name: "noise",
        uncompressed: false,
        hex: "28b52ffd04382510000c2000ad763674ec79cfea8b8e1503fd9e1fffb8754f196def1adebde4133f2d7d37f55aeced52\
            f609b3205ec2b9acbbd20d75b9ec5fd926121026a679afc6e3c8174573e5be01a9ce5da9321c80638f4c5e30ea2fe126\
            fa75b544b6bf120c121a1cb23b3ff7ae4b07d5d4592f4d97d06b9da203cb54fe5919cdba91bce112aa298de43fa36aa6\
            373f9724825575917e3e529c8f66c40521093f87f467fbb2cbe6f9ba4371f700ba7bfdd26347990488ed30ee9d0432a8\
            57ad7852ce151ba19c55b7a67d39d54afc65c3e47d6171697db7c6efe06b0f395175535bb384e160f68372ae597b334b\
            c54c02b6278f0650fc9906ac35155b6791d4372d6bd7c0d3fccb7c9857eb862f51ff0e20bcd5b291d6e5620ac0bb40b2\
            2df5b2e1e40b5f31830ec817731567916c3e38d22b4b8786e7622a1020d2ed60ecfb3a8a9bac85e4c9e473a5c4572131\
            992270d2ed00e0a31c969370c893988bd0a15c3aa485c43185b6e11e8c3dbd621c34a84b2b9704aeb3940f799d8c97e6\
            c33ef853bdde19a97648cc678b7efcc74caac3f293b2656f719b4ad63f83144c00f235652d31b35a5430ca5903957a3e\
            a22f81fe2949b9d89350022a36d48764846f3d4567337ba71cfe89e6b37eeab5a87b5fc90034a07b993ccc974e529689\
            342ad6ebe4f13cec61630dabe6d5ba038954f8d30c48160038fc7585800fc9c079e170567048180cf23f1b2b00efafe5\
            a8",
    },
    Case {
        name: "noise-raw",
        uncompressed: true,
        hex: "28b52ffd0438011000ad763674ec79cfea8b8e1503fd9e1fffb8754f196def1adebde4133f2d7d37f55aeced52f609b3\
            205ec2b9acbbd20d75b9ec5fd926121026a679afc6e3c8174573e5be01a9ce5da9321c80638f4c5e30ea2fe126fa75b5\
            44b6bf120c121a1cb23b3ff7ae4b07d5d4592f4d97d06b9da203cb54fe5919cdba91bce112aa298de43fa36aa6373f97\
            24825575917e3e529c8f66c40521093f87f467fbb2cbe6f9ba4371f700ba7bfdd26347990488ed30ee9d0432a857ad78\
            52ce151ba19c55b7a67d39d54afc65c3e47d6171697db7c6efe06b0f395175535bb384e160f68372ae597b334bc54c02\
            b6278f0650fc9906ac35155b6791d4372d6bd7c0d3fccb7c9857eb862f51ff0e20bcd5b291d6e5620ac0bb40b22df5b2\
            e1e40b5f31830ec817731567916c3e38d22b4b8786e7622a1020d2ed60ecfb3a8a9bac85e4c9e473a5c4572131992270\
            d2ed00e0a31c969370c893988bd0a15c3aa485c43185b6e11e8c3dbd621c34a84b2b9704aeb3940f799d8c97e6c33ef8\
            53bdde19a97648cc678b7efcc74caac3f293b2656f719b4ad63f83144c00f235652d31b35a5430ca5903957a3ea22f81\
            fe2949b9d89350022a36d48764846f3d4567337ba71cfe89e6b37eeab5a87b5fc90034a07b993ccc974e529689342ad6\
            ebe4f13cec61630dabe6d5ba038954f8d30c48160038fc7585800fc9c079e170567048180cf23f1b2befafe5a8",
    },
    Case {
        name: "planes",
        uncompressed: false,
        hex: "28b52ffd0438e503002c01000202020202010200020401030100000000001fa810639e03108c222ae9102261489c8414\
            ff43712301511d7d4798d0e084d20a2edc2af090ccaf18e424d9c4e43aed440f804367034aed6720b97d06eddf3344b6\
            330af6c4f63afd64cf846367034ae96720b97d06edd73344b6330af4c4e63aed440f0077023c78f863",
    },
];

/// The bytes the case called `name` was encoded from. Deterministic, invented.
pub(crate) fn input(name: &str) -> Vec<u8> {
    match name {
        "empty" => Vec::new(),
        // 300,000 of one byte: three blocks, the first two at the 128 KiB
        // maximum, so a block boundary falls inside the run.
        "run" => vec![0x37; 300_000],
        // Counter text with a 50-line period, 4,000 lines: compressible, not
        // one byte, and longer than one block.
        "text" => (0..4_000_u32)
            .flat_map(|i| {
                format!("synthetic row {:02} alpha beta gamma delta\n", i % 50).into_bytes()
            })
            .collect(),
        // xorshift64 noise, which no encoder can shrink. At `Fastest` 0.8.2
        // still wrote it as a Compressed block; `noise-raw` is the same bytes
        // at `Uncompressed`, which is a Raw block with content.
        "noise" | "noise-raw" => {
            let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
            (0..512)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    x.to_le_bytes()[0]
                })
                .collect()
        }
        // A columnar numeric payload in the tick store's shape: `rows` shape
        // bytes, a delta flag, a width, then `width` byte planes. Small
        // deltas, so the literals are few distinct symbols.
        "planes" => {
            let rows = 4_000_usize;
            let mut out = vec![0x02; rows];
            out.extend([1, 2]);
            out.extend((0..rows).map(|i| [0, 2, 4, 1, 3][i % 5]));
            out.extend((0..rows).map(|i| u8::from(i % 97 == 0)));
            out
        }
        other => panic!("no synthetic input called {other}"),
    }
}

/// The frame of `case`, from its hex. Panics on an empty string (a case whose
/// frame was never filled in cannot pass as a frame of no bytes) and on a
/// case labelled `uncompressed` whose frame holds anything but Raw blocks,
/// which is the only block type that level writes — so the label is checked
/// against the bytes rather than trusted.
pub(crate) fn frame(case: &Case) -> Vec<u8> {
    let digits: Vec<u8> = case.hex.bytes().filter(u8::is_ascii_hexdigit).collect();
    assert!(!digits.is_empty(), "{}: the frame is filled in", case.name);
    assert_eq!(digits.len() % 2, 0, "{}: whole bytes of hex", case.name);
    let bytes: Vec<u8> = digits
        .chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    if case.uncompressed {
        assert!(
            block_types(&bytes).iter().all(|block| *block == 0),
            "{}: `Uncompressed` writes Raw blocks only",
            case.name
        );
    }
    bytes
}

/// The `Block_Type` of every block in `frame`, in order, read from the frame
/// and block headers alone (RFC 8878 §3.1.1, §3.1.1.2): 0 Raw, 1 RLE,
/// 2 Compressed. Panics unless the walk ends exactly at the frame's last byte,
/// so a fixture that is not one whole frame cannot pass as one.
pub(crate) fn block_types(frame: &[u8]) -> Vec<u8> {
    assert_eq!(frame[..4], MAGIC, "a ZSTD frame");
    let descriptor = frame[4];
    let single_segment = descriptor & 0x20 != 0;
    let checksum = descriptor & 0x04 != 0;
    let dictionary_id = [0, 1, 2, 4][usize::from(descriptor & 0x03)];
    let content_size = match descriptor >> 6 {
        0 => usize::from(single_segment),
        1 => 2,
        2 => 4,
        _ => 8,
    };
    let mut at = 5 + usize::from(!single_segment) + dictionary_id + content_size;
    let mut types = Vec::new();
    loop {
        let header = usize::from(frame[at])
            | usize::from(frame[at + 1]) << 8
            | usize::from(frame[at + 2]) << 16;
        at += 3;
        let block_type = u8::try_from((header >> 1) & 0x03).unwrap();
        types.push(block_type);
        // An RLE block carries its one byte; Raw and Compressed carry
        // `Block_Size` bytes.
        at += if block_type == 1 { 1 } else { header >> 3 };
        if header & 1 == 1 {
            break;
        }
    }
    if checksum {
        at += 4;
    }
    assert_eq!(at, frame.len(), "the walk ends at the frame's last byte");
    types
}

/// A 1 KiB window (descriptor `0x00`, not single-segment) holding one RLE block
/// that regenerates 4,096 bytes: four times RFC 8878 §3.1.1.2.4's
/// `Block_Maximum_Size = min(Window_Size, 128 KiB)`. This is the advisory's
/// class in its smallest form — a block whose OUTPUT nothing bounded. 0.8.2
/// decodes it to 4,096 `Z`s; 0.9.1 refuses it.
///
/// `28 B5 2F FD` magic · `00` descriptor · `00` window 1 KiB ·
/// `03 80 00` last block, RLE, size 4,096 (`1 | 1<<1 | 4096<<3`) · `5A` `Z`.
pub(crate) const RLE_WIDER_THAN_WINDOW: [u8; 10] =
    [0x28, 0xB5, 0x2F, 0xFD, 0x00, 0x00, 0x03, 0x80, 0x00, 0x5A];

/// What [`RLE_WIDER_THAN_WINDOW`] decoded to under 0.8.2, in bytes.
pub(crate) const RLE_WIDER_THAN_WINDOW_BYTES: usize = 4_096;

/// A single-segment frame whose window — its 4-byte content size — is exactly
/// [`WINDOW_CEILING`], 134,217,728 bytes, and whose one block is an empty last
/// Raw block. Accepted by 0.8.2 and by 0.9.1 at the ceiling.
///
/// `28 B5 2F FD` magic · `A0` single segment, 4-byte content size ·
/// `00 00 00 08` 2^27 · `01 00 00` last block, Raw, size 0.
pub(crate) const WINDOW_AT_CEILING: [u8; 12] = [
    0x28, 0xB5, 0x2F, 0xFD, 0xA0, 0x00, 0x00, 0x00, 0x08, 0x01, 0x00, 0x00,
];

/// [`WINDOW_AT_CEILING`] one byte wider: 134,217,729. ACCEPTED by 0.8.2, which
/// checked no window on a fresh decoder, and refused at the ceiling.
pub(crate) const WINDOW_PAST_CEILING: [u8; 12] = [
    0x28, 0xB5, 0x2F, 0xFD, 0xA0, 0x01, 0x00, 0x00, 0x08, 0x01, 0x00, 0x00,
];

/// The window ceiling both call sites pass: 128 MiB, `ruzstd` 0.9.1's
/// `DEFAULT_MAX_WINDOW_SIZE`. Each call site keeps its own constant, and each
/// crate's tests hold it equal to this one.
pub(crate) const WINDOW_CEILING: u64 = 128 * 1024 * 1024;
