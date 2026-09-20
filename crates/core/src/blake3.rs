//! BLAKE3, written here rather than taken as a dependency.
//!
//! # Why this is not `blake3 = "1"`
//!
//! `docs/00-charter.md` §5 names BLAKE3 as the run-identity hash, and
//! `CLAUDE.md` §3 rule 3 repeats it. The published crate cannot be taken:
//! measured against version 1.8.6 in the local registry, it ships **25 non-Rust
//! source files** — eleven `.c`, two `.h`, six `.S` and four `.asm` — and its
//! `build.rs` calls `cc::Build::new()` on the default feature set, because the
//! `pure` feature *disables* the C path rather than enabling it.
//!
//! `CLAUDE.md` §2 forbids "any `build.rs` that invokes an external process" and
//! "any vendored binding to another language", **without exception**, and
//! instructs a task that appears to need one to stop and say so. CI gate 2
//! would not have caught it: it walks `git ls-files '*build.rs'`, which is
//! tracked files only, so a dependency's build script is invisible to it and
//! the workspace would have gone green on a §2 breach.
//!
//! This module is the alternative that keeps the rule: no dependency, no
//! lockfile entry, no change to the 187-package fingerprint
//! `vocab::tests::the_dependency_set_has_not_moved_without_review` pins, and
//! nothing added to the DECLARED non-Rust table beside `ring`.
//!
//! It follows the precedent this workspace already set twice — `pull::totp`
//! implements RFC 6238 from scratch against its published vectors, and
//! `crates/greeks` implements Black-Scholes-Merton with zero dependencies.
//!
//! # Correctness is by published vector, not by inspection
//!
//! An implementation of a hash that has never reproduced a vector it did not
//! generate itself is worth nothing — it is self-consistent and possibly
//! self-consistently wrong. The tests below check the official unkeyed vectors
//! from the BLAKE3 reference test set, at lengths that exercise every structural
//! boundary: the empty input, a partial block, exactly one block, one block plus
//! one byte, a chunk less one, exactly one chunk, and one chunk plus one — which
//! is the first input that builds a parent node at all.
//!
//! # Wrapping is explicit, and the profile is why
//!
//! `Cargo.toml` sets `overflow-checks = true` on the release profile, so a plain
//! `+` **panics** on overflow rather than wrapping: a wrapped price is a
//! plausible number that is wrong, which `CLAUDE.md` §4 bans. BLAKE3's
//! compression is modular arithmetic by definition, so every addition here is
//! spelled `wrapping_add`. That is the difference the profile exists to make
//! visible — silent wrapping is refused, declared wrapping is stated.
//!
//! # Cost
//!
//! One compression per 64-byte block, each a fixed seven rounds over a fixed
//! sixteen-word state, plus at most one parent compression per completed chunk.
//! No loop here has a bound that depends on anything but the input length, and
//! no allocation happens per block. UNVERIFIED as a measured per-byte figure:
//! this module ships no bench, and `docs/06-limits.md` is where that is recorded
//! rather than a number being asserted here.

/// Bytes of digest this module produces.
pub const OUT_LEN: usize = 32;

/// Bytes per compression input block.
const BLOCK_LEN: usize = 64;

/// Bytes per chunk. A chunk is sixteen blocks.
const CHUNK_LEN: usize = 1024;

/// Set on the first block of a chunk.
const CHUNK_START: u32 = 1;
/// Set on the last block of a chunk.
const CHUNK_END: u32 = 2;
/// Set when compressing two child chaining values into a parent.
const PARENT: u32 = 4;
/// Set on the final compression, whose output is the digest.
const ROOT: u32 = 8;

/// The SHA-256 initialisation vector, which BLAKE3 reuses.
const IV: [u32; 8] = [
    0x6A09_E667,
    0xBB67_AE85,
    0x3C6E_F372,
    0xA54F_F53A,
    0x510E_527F,
    0x9B05_688C,
    0x1F83_D9AB,
    0x5BE0_CD19,
];

/// The quarter-round.
///
/// Takes and returns four words rather than indexing a state array, because
/// `indexing_slicing` is denied workspace-wide and the textbook spelling
/// (`state[a] = state[a].wrapping_add(state[b])`) indexes by a runtime value. By
/// value there is no index to be out of bounds and the lint has nothing to
/// refuse — the same arithmetic, said in a way the compiler can check.
const fn g(a: u32, b: u32, c: u32, d: u32, mx: u32, my: u32) -> (u32, u32, u32, u32) {
    let a = a.wrapping_add(b).wrapping_add(mx);
    let d = (d ^ a).rotate_right(16);
    let c = c.wrapping_add(d);
    let b = (b ^ c).rotate_right(12);
    let a = a.wrapping_add(b).wrapping_add(my);
    let d = (d ^ a).rotate_right(8);
    let c = c.wrapping_add(d);
    let b = (b ^ c).rotate_right(7);
    (a, b, c, d)
}

/// One round: four column quarter-rounds, then four diagonal ones.
///
/// Every index is a literal, so this is bounds-checked at compile time.
const fn round(s: [u32; 16], m: [u32; 16]) -> [u32; 16] {
    let (s0, s4, s8, s12) = g(s[0], s[4], s[8], s[12], m[0], m[1]);
    let (s1, s5, s9, s13) = g(s[1], s[5], s[9], s[13], m[2], m[3]);
    let (s2, s6, s10, s14) = g(s[2], s[6], s[10], s[14], m[4], m[5]);
    let (s3, s7, s11, s15) = g(s[3], s[7], s[11], s[15], m[6], m[7]);

    let (s0, s5, s10, s15) = g(s0, s5, s10, s15, m[8], m[9]);
    let (s1, s6, s11, s12) = g(s1, s6, s11, s12, m[10], m[11]);
    let (s2, s7, s8, s13) = g(s2, s7, s8, s13, m[12], m[13]);
    let (s3, s4, s9, s14) = g(s3, s4, s9, s14, m[14], m[15]);

    [
        s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15,
    ]
}

/// The message schedule, written out so every index is a literal.
const fn permute(m: [u32; 16]) -> [u32; 16] {
    [
        m[2], m[6], m[3], m[10], m[7], m[0], m[4], m[13], m[1], m[11], m[12], m[5], m[9], m[14],
        m[15], m[8],
    ]
}

/// The compression function: seven rounds, then the two feed-forward folds.
fn compress(
    chaining_value: &[u32; 8],
    block_words: &[u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    // `try_from` on a masked value cannot fail, and `unwrap_or` keeps the
    // impossible arm inside `core` rather than leaving an uncoverable region
    // here — the same argument this workspace makes for `.expect` over
    // `unreachable!`. A truncating `as` cast is denied outright.
    let counter_low = u32::try_from(counter & 0xFFFF_FFFF).unwrap_or(0);
    let counter_high = u32::try_from(counter >> 32).unwrap_or(0);

    let mut state: [u32; 16] = [
        chaining_value[0],
        chaining_value[1],
        chaining_value[2],
        chaining_value[3],
        chaining_value[4],
        chaining_value[5],
        chaining_value[6],
        chaining_value[7],
        IV[0],
        IV[1],
        IV[2],
        IV[3],
        counter_low,
        counter_high,
        block_len,
        flags,
    ];

    let mut m = *block_words;
    // Six rounds with a following permutation, then the seventh round. Keeping
    // the final round outside the loop avoids a branch whose mutation only
    // permutes an already-consumed message and cannot affect the digest.
    let mut r: u32 = 0;
    while r < 6 {
        state = round(state, m);
        m = permute(m);
        r = r.saturating_add(1);
    }
    state = round(state, m);

    // The feed-forward. Written with literal indices for the same reason `round`
    // is: no runtime index means no lint and no bounds check to get wrong.
    [
        state[0] ^ state[8],
        state[1] ^ state[9],
        state[2] ^ state[10],
        state[3] ^ state[11],
        state[4] ^ state[12],
        state[5] ^ state[13],
        state[6] ^ state[14],
        state[7] ^ state[15],
        state[8] ^ chaining_value[0],
        state[9] ^ chaining_value[1],
        state[10] ^ chaining_value[2],
        state[11] ^ chaining_value[3],
        state[12] ^ chaining_value[4],
        state[13] ^ chaining_value[5],
        state[14] ^ chaining_value[6],
        state[15] ^ chaining_value[7],
    ]
}

/// The first eight words of a compression output — a chaining value.
const fn first_eight(s: [u32; 16]) -> [u32; 8] {
    [s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]
}

/// A 64-byte block as sixteen little-endian words.
fn words_of(block: &[u8; BLOCK_LEN]) -> [u32; 16] {
    let mut out = [0_u32; 16];
    // `chunks_exact(4)` yields slices of exactly four, so `copy_from_slice`
    // cannot mismatch. Zipping avoids indexing either side.
    for (word, four) in out.iter_mut().zip(block.chunks_exact(4)) {
        let mut le = [0_u8; 4];
        le.copy_from_slice(four);
        *word = u32::from_le_bytes(le);
    }
    out
}

/// A compression that has not been performed yet, kept so the ROOT flag can be
/// applied to whichever one turns out to be last.
///
/// The root flag cannot be decided while hashing: it belongs to the final
/// compression, and which one that is depends on how much input arrives after.
/// So the last compression is deferred rather than performed and patched.
struct Output {
    chaining_value: [u32; 8],
    block_words: [u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
}

impl Output {
    fn chaining_value(&self) -> [u32; 8] {
        first_eight(compress(
            &self.chaining_value,
            &self.block_words,
            self.counter,
            self.block_len,
            self.flags,
        ))
    }

    /// The digest, which is this compression performed with [`ROOT`] set.
    fn root_bytes(&self) -> [u8; OUT_LEN] {
        let words = compress(
            &self.chaining_value,
            &self.block_words,
            0,
            self.block_len,
            self.flags | ROOT,
        );
        let mut out = [0_u8; OUT_LEN];
        for (slot, word) in out.chunks_exact_mut(4).zip(words.iter().take(8)) {
            slot.copy_from_slice(&word.to_le_bytes());
        }
        out
    }
}

/// One chunk being filled, block by block.
struct ChunkState {
    chaining_value: [u32; 8],
    counter: u64,
    block: [u8; BLOCK_LEN],
    block_len: usize,
    blocks_compressed: usize,
}

impl ChunkState {
    const fn new(chaining_value: [u32; 8], counter: u64) -> Self {
        Self {
            chaining_value,
            counter,
            block: [0; BLOCK_LEN],
            block_len: 0,
            blocks_compressed: 0,
        }
    }

    const fn len(&self) -> usize {
        BLOCK_LEN
            .saturating_mul(self.blocks_compressed)
            .saturating_add(self.block_len)
    }

    /// [`CHUNK_START`] belongs to the first block of the chunk and no other.
    const fn start_flag(&self) -> u32 {
        if self.blocks_compressed == 0 {
            CHUNK_START
        } else {
            0
        }
    }

    fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            // A full block is only compressed once it is known NOT to be the
            // last: the last block of a chunk carries CHUNK_END, and whether
            // this is the last is not known until more input arrives or does
            // not. So a full buffer is flushed on the next byte, never on the
            // byte that filled it.
            if self.block_len == BLOCK_LEN {
                let words = words_of(&self.block);
                let flags = self.start_flag();
                self.chaining_value = first_eight(compress(
                    &self.chaining_value,
                    &words,
                    self.counter,
                    // A compressed interior block is always full.
                    u32::try_from(BLOCK_LEN).unwrap_or(0),
                    flags,
                ));
                self.blocks_compressed = self.blocks_compressed.saturating_add(1);
                self.block = [0; BLOCK_LEN];
                self.block_len = 0;
            }

            let room = BLOCK_LEN.saturating_sub(self.block_len);
            let take = room.min(input.len());
            let (head, tail) = input.split_at(take);
            // Copied by zipping rather than through `get_mut(range)`, and the
            // reason is coverage rather than taste: an `if let Some(slot) = ..`
            // whose range is valid by construction leaves an else-arm no input can
            // reach, and llvm-cov counts it as a region forever uncovered. Two
            // bounded iterators have no arm to miss -- `skip` cannot run past the
            // end and `zip` stops at the shorter side.
            for (slot, byte) in self.block.iter_mut().skip(self.block_len).zip(head) {
                *slot = *byte;
            }
            self.block_len = self.block_len.saturating_add(take);
            input = tail;
        }
    }

    fn output(&self) -> Output {
        Output {
            chaining_value: self.chaining_value,
            block_words: words_of(&self.block),
            counter: self.counter,
            block_len: u32::try_from(self.block_len).unwrap_or(0),
            flags: self.start_flag() | CHUNK_END,
        }
    }
}

/// Two chaining values folded into their parent.
fn parent_output(left: [u32; 8], right: [u32; 8]) -> Output {
    let mut block_words = [0_u32; 16];
    for (slot, word) in block_words.iter_mut().zip(left.iter().chain(right.iter())) {
        *slot = *word;
    }
    Output {
        chaining_value: IV,
        block_words,
        counter: 0,
        block_len: u32::try_from(BLOCK_LEN).unwrap_or(0),
        flags: PARENT,
    }
}

/// An incremental BLAKE3 hasher.
///
/// Feed it with [`Hasher::update`] as many times as convenient; the digest does
/// not depend on how the input was split, which
/// [`crate::blake3::tests::the_digest_does_not_depend_on_how_the_input_is_split`]
/// proves.
pub struct Hasher {
    chunk: ChunkState,
    /// Completed subtree chaining values awaiting a partner. Pre-sized: a
    /// 2^64-byte input needs at most 54 entries, so this never reallocates.
    stack: Vec<[u32; 8]>,
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher {
    /// A hasher over no input yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            chunk: ChunkState::new(IV, 0),
            stack: Vec::with_capacity(54),
        }
    }

    /// Folds a finished chunk's chaining value into the tree.
    ///
    /// A subtree is complete exactly when the count of chunks so far has a zero
    /// in the corresponding bit, which is why the merge condition is arithmetic
    /// on the counter rather than a search of the stack.
    fn add_chunk(&mut self, mut cv: [u32; 8], total_chunks: u64) {
        // How many subtrees close at this chunk: one per trailing zero bit.
        let mut merges: usize = 0;
        let mut counter = total_chunks;
        while counter & 1 == 0 {
            merges = merges.saturating_add(1);
            counter >>= 1;
        }
        // `drain(..).rev()` rather than repeated `pop()`, for the same reason
        // `update` zips: `pop()` yields an `Option` whose `None` arm cannot happen
        // here -- the stack always holds one entry per set bit of the count -- and
        // an arm that cannot happen is a region that can never be covered. Draining
        // a suffix has no such arm, allocates nothing, and says the invariant
        // out loud: `min` is what keeps the range valid rather than a branch.
        let keep = self
            .stack
            .len()
            .saturating_sub(merges.min(self.stack.len()));
        for left in self.stack.drain(keep..).rev() {
            cv = parent_output(left, cv).chaining_value();
        }
        self.stack.push(cv);
    }

    /// Adds `input` to the hash.
    pub fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            if self.chunk.len() == CHUNK_LEN {
                let cv = self.chunk.output().chaining_value();
                let next = self.chunk.counter.saturating_add(1);
                self.add_chunk(cv, next);
                self.chunk = ChunkState::new(IV, next);
            }
            let room = CHUNK_LEN.saturating_sub(self.chunk.len());
            let take = room.min(input.len());
            let (head, tail) = input.split_at(take);
            self.chunk.update(head);
            input = tail;
        }
    }

    /// The 32-byte digest of everything fed so far.
    #[must_use]
    pub fn finalize(&self) -> [u8; OUT_LEN] {
        let mut output = self.chunk.output();
        // Fold the stack from the top down. Whatever is left when the stack is
        // empty is the root, and only then does ROOT apply.
        for left in self.stack.iter().rev() {
            output = parent_output(*left, output.chaining_value());
        }
        output.root_bytes()
    }
}

/// The BLAKE3 digest of `input`, in one call.
#[must_use]
pub fn hash(input: &[u8]) -> [u8; OUT_LEN] {
    let mut hasher = Hasher::new();
    hasher.update(input);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::{CHUNK_END, CHUNK_LEN, CHUNK_START, Hasher, IV, Output, ROOT, compress, hash};

    /// The reference test set's input: byte `i` is `i % 251`.
    fn vector_input(len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        let mut i = 0_usize;
        while i < len {
            let b = u8::try_from(i % 251).unwrap_or(0);
            out.push(b);
            i = i.saturating_add(1);
        }
        out
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        let mut s = String::with_capacity(bytes.len().saturating_mul(2));
        for b in bytes {
            // `write!` into a String cannot fail, and it is what
            // `clippy::format_push_string` asks for in place of
            // `push_str(&format!(..))` -- one allocation instead of one per byte.
            let _ = write!(s, "{b:02x}");
        }
        s
    }

    /// The published unkeyed vectors, at every structural boundary.
    ///
    /// These are NOT generated by this implementation — that would be circular
    /// and would prove only that the code agrees with itself. Reproducing a
    /// digest this module did not produce is the whole of the evidence that it
    /// is BLAKE3 and not merely a deterministic function of the input.
    ///
    /// # How they were obtained, and why that matters
    ///
    /// From an independent BLAKE3 implementation run outside this repository,
    /// over the reference input `byte[i] = i % 251`. That provenance is written
    /// down because the first draft of this test carried a 1025-byte digest
    /// recalled from memory whose first 38 hex characters were right and whose
    /// tail was invented. This implementation disagreed with it, the
    /// implementation was correct, and the vector was wrong — which is the exact
    /// failure mode a hash test exists to catch, arriving from the direction
    /// nobody guards.
    ///
    /// The lengths past 1024 are the load-bearing ones: below that the whole
    /// input is a single chunk and the tree is never built, so a parent
    /// compression is never performed. 1025, 2049 and 10,000 each leave a
    /// different partial subtree on the stack.
    #[test]
    fn the_published_vectors_reproduce() {
        let cases: [(usize, &str); 12] = [
            (
                0,
                "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
            ),
            (
                1,
                "2d3adedff11b61f14c886e35afa036736dcd87a74d27b5c1510225d0f592e213",
            ),
            (
                63,
                "e9bc37a594daad83be9470df7f7b3798297c3d834ce80ba85d6e207627b7db7b",
            ),
            (
                64,
                "4eed7141ea4a5cd4b788606bd23f46e212af9cacebacdc7d1f4c6dc7f2511b98",
            ),
            (
                65,
                "de1e5fa0be70df6d2be8fffd0e99ceaa8eb6e8c93a63f2d8d1c30ecb6b263dee",
            ),
            (
                1023,
                "10108970eeda3eb932baac1428c7a2163b0e924c9a9e25b35bba72b28f70bd11",
            ),
            (
                1024,
                "42214739f095a406f3fc83deb889744ac00df831c10daa55189b5d121c855af7",
            ),
            // Past here the input spans more than one chunk, so the root is a
            // PARENT compression and the tree structure itself is under test.
            (
                1025,
                "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444",
            ),
            (
                2048,
                "e776b6028c7cd22a4d0ba182a8bf62205d2ef576467e838ed6f2529b85fba24a",
            ),
            (
                2049,
                "5f4d72f40d7a5f82b15ca2b2e44b1de3c2ef86c426c95c1af0b6879522563030",
            ),
            (
                4096,
                "015094013f57a5277b59d8475c0501042c0b642e531b0a1c8f58d2163229e969",
            ),
            (
                10_000,
                "5f81f9e4ab67627b6b036d5d4e3bc40d9d3daa6fcc2b6dd07ab2bbf0a877da54",
            ),
        ];
        for (len, want) in cases {
            let got = hex(&hash(&vector_input(len)));
            assert_eq!(got, want, "BLAKE3 of the {len}-byte reference input");
        }
    }

    #[test]
    fn the_digest_does_not_depend_on_how_the_input_is_split() {
        // §3 rule 5 in miniature: the same bytes must give the same digest
        // however they arrive, or a streamed digest over bars would depend on
        // the read size.
        let input = vector_input(2500);
        let whole = hash(&input);
        for split in [1_usize, 63, 64, 65, 1023, 1024, 1025, 2499] {
            let mut h = Hasher::new();
            // `split_at` and not two `get(..)` calls: it returns a pair rather
            // than two Options, so there is no arm this loop never takes.
            let (a, b) = input.split_at(split);
            h.update(a);
            h.update(b);
            assert_eq!(h.finalize(), whole, "split at {split} changed the digest");
        }
        // And byte at a time, which exercises every partial-block path.
        let mut one_at_a_time = Hasher::new();
        for b in &input {
            one_at_a_time.update(&[*b]);
        }
        assert_eq!(one_at_a_time.finalize(), whole);
    }

    #[test]
    fn one_differing_byte_re_keys_the_digest() {
        // The property `docs/00-charter.md` §5 is actually asking for: "One
        // differing bar re-keys the identity. That is the point."
        let a = vector_input(CHUNK_LEN.saturating_mul(3));
        let last = a.len().saturating_sub(1);
        // Built by mapping rather than by `last_mut()`, so both arms of the
        // condition are taken and no region is left unreachable.
        let b: Vec<u8> = a
            .iter()
            .enumerate()
            .map(|(i, byte)| if i == last { byte ^ 1 } else { *byte })
            .collect();
        assert_ne!(hash(&a), hash(&b), "a one-bit change must re-key");
    }

    #[test]
    fn root_output_keeps_an_already_present_root_flag() {
        // Internal flag composition must set ROOT, not toggle it. Current
        // constructors defer ROOT; this additionally checks pre-marked output.
        let output = Output {
            chaining_value: IV,
            block_words: [0; 16],
            counter: 0,
            block_len: 0,
            flags: CHUNK_START | CHUNK_END | ROOT,
        };
        assert_eq!(
            hex(&output.root_bytes()),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn complete_compression_output_matches_the_official_empty_vector() {
        // First 64 output bytes of the upstream empty-input unkeyed vector;
        // see docs/00-charter.md. The public digest uses only the first 32.
        let words = compress(&IV, &[0; 16], 0, 0, CHUNK_START | CHUNK_END | ROOT);
        let bytes: Vec<_> = words.iter().flat_map(|word| word.to_le_bytes()).collect();
        assert_eq!(
            hex(&bytes),
            concat!(
                "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
                "e00f03e7b69af26b7faaf09fcd333050338ddfe085b8cc869ca98b206c08243a"
            )
        );
    }

    #[test]
    fn compression_preserves_both_halves_of_large_chunk_counters() {
        // Small published inputs never reach a chunk counter above 32 bits.
        // Exercise that boundary directly without allocating terabytes of data.
        let counters = [
            0,
            1,
            u64::from(u32::MAX),
            1_u64 << 32,
            (1_u64 << 32) + 1,
            u64::MAX,
        ];
        for (index, counter) in counters.iter().enumerate() {
            let actual = compress(&IV, &[0; 16], *counter, 64, CHUNK_START);
            for previous in counters.iter().take(index) {
                assert_ne!(
                    actual,
                    compress(&IV, &[0; 16], *previous, 64, CHUNK_START),
                    "distinct chunk counters {counter} and {previous} collapsed"
                );
            }
        }
    }

    #[test]
    fn an_empty_hasher_and_an_empty_slice_agree() {
        assert_eq!(Hasher::new().finalize(), hash(&[]));
        assert_eq!(Hasher::default().finalize(), hash(&[]));
    }
}
