//! The results ledger, over HTTP.
//!
//! # What this closes
//!
//! `crates/cli/src/results.rs` records every completed sweep into one
//! append-only file, and until this module nothing served by this crate could
//! read it. `/audit` is the INGEST console — what a PULL did — and the sweep
//! had no page at all. So "which rung carries the edge on NIFTY" was a question
//! answerable only by rerunning `cli results` in a terminal, while the
//! application that operators actually look at said nothing about the engine's
//! output.
//!
//! # This is a READER. The writer is `cli`, and there is exactly one
//!
//! The byte layout below is re-declared rather than imported, and that is a
//! deliberate choice with a cost. `api`'s dependency set is `core, pull, store,
//! telemetry`; taking `cli` to borrow one struct would pull `engine`,
//! `indicators`, `vocab`, `costs` and `runner` behind it — five crates this
//! surface never calls, into the crate that must build fastest because every
//! page waits on it.
//!
//! The cost is that two files now state one layout, and a format with two
//! statements of itself is a format that can diverge. Three things hold it
//! together:
//!
//! 1. **Nothing here writes.** [`File::open`] is read-only and there is no
//!    append path in this module. A reader that drifts renders wrong; a writer
//!    that drifts corrupts. Only one crate may write, and it is `cli`.
//! 2. **The stride is asserted against the field sum at compile time** —
//!    [`FIELD_SUM`] below — which is the same check
//!    `the_stride_is_exactly_what_the_writer_writes` makes on the writer's
//!    side. A field added there and not here changes the sum, and this crate
//!    stops compiling.
//! 3. **The version is checked at open** and a file this build does not know is
//!    REFUSED with a sentence, never parsed hopefully. `CLAUDE.md` §3 rule 8
//!    makes a new field a new version, so the version is the guard rail that
//!    catches a divergence the compile-time assertion cannot see.
//!
//! # `halted` is the field that decides whether the others mean anything
//!
//! `halted == 1` means a budget stopped that ladder short. `depth` is then
//! PARTIAL and `combinations` covers LESS of the search while reading LARGER,
//! so a halted row is not comparable with a complete one and must never be
//! ranked against one. [`Ledger::best_complete`] filters them out and returns
//! [`None`] when none survives, which the page renders as a sentence rather
//! than as an empty slot.
//!
//! # Cost
//!
//! | Operation | Cost | How |
//! |---|---|---|
//! | count | **O(1)** | `(file_len - 16) / stride`, no walk |
//! | read record *i* | **O(1)** | seek to `16 + i·stride`, one read |
//! | one request | **O(limit)** | bounded by [`MAX_RUNS`], and it says when it stopped |
//!
//! `stride` is [`stride_of`] the file's own version, read from its header
//! before any address is computed. It is not a literal here because it is not a
//! literal in the code: this reader knows two versions, and writing one of
//! their strides into the table is how the previous two of these comments came
//! to state a number the writer had stopped using.
//!
//! The request is bounded at both ends by construction, which is what makes it
//! safe to expose: a query string cannot make this server read the disk without
//! a ceiling. When the ceiling bites, `hit_scan_cap` says so and the page
//! prints it — the same contract [`crate::logs`] keeps, and for the same
//! reason: "what I read" and "everything there is" are different answers and
//! only one of them is true.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

use std::fmt::Write as _;
use std::fs::File;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use crate::render;

/// `BRUTEXRS`, so a file that is not this one is refused before it is parsed.
///
/// Matched byte for byte against `cli::results::MAGIC`. A store root pointed at
/// the wrong directory is a real operator mistake, and eight bytes is what
/// separates "no runs yet" from "this is somebody else's file".
const MAGIC: [u8; 8] = *b"BRUTEXRS";

/// The newest version this build reads, and the layout every [`Run`] is
/// widened INTO.
///
/// A file at a version this build does not know is REFUSED, not parsed.
/// `CLAUDE.md` §3 rule 8 makes a new field a new file version at its own
/// stride, so an unknown version has a stride this build does not know, and
/// every address computed from the wrong one would land mid-record. It would
/// still PARSE — every byte pattern is a legal value of its type — and produce
/// a page of confident nonsense, which is the failure wearing a success's
/// clothes `CLAUDE.md` §4 bans.
///
/// **Two versions are known, not one, and that is a deliberate reversal.** This
/// constant was 2 and the file refused everything else, which was correct while
/// 2 was the newest. When `cli` went to 3 the refusal stopped protecting the
/// operator and started blanking their page: the only ledger in existence is a
/// version-2 file holding whole records that `cli results` reads perfectly. A
/// reader that refuses data a writer in the same workspace still reads is not
/// being careful, it is disagreeing with itself — and this module exists
/// precisely so that two decoders of one file do not disagree.
const VERSION: u32 = 3;

/// The version before this one, which is READ and never written.
///
/// `CLAUDE.md` §3 rule 8 is append-only history: store format versions are
/// never mutated in place. Refusing to READ one is a different act from
/// refusing to MUTATE it, and `cli` drew that distinction first — see
/// `cli::results`, whose own note says a version-2 record "simply predates the
/// mask". This crate follows the writer's reasoning rather than contradicting
/// it.
///
/// This crate never writes to the ledger at all, so there is no matching
/// "never appended to" clause here: the whole file is read-only from `api`.
const VERSION_V2: u32 = 2;

/// Magic, version, and four reserved bytes.
const HEADER: u64 = 16;

/// [`HEADER`] as a `usize`, for the header array.
///
/// Declared rather than cast: `HEADER as usize` is a narrowing on a 32-bit
/// target and `cast_possible_truncation` is denied workspace-wide. Two
/// constants that must agree, and a `const` assertion that they do.
const HEADER_BYTES: usize = 16;

const _: () = assert!(HEADER_BYTES as u64 == HEADER);

/// Bytes per record at [`VERSION`]: 253 of fields and 8 of seal.
///
/// The number is `cli::results::STRIDE`, and it is PINNED to it rather than
/// merely copied from it — see the assertion below.
const STRIDE: u64 = 261;

/// [`STRIDE`] as a `usize`, for the record array. Same reason as
/// [`HEADER_BYTES`].
const STRIDE_BYTES: usize = 261;

const _: () = assert!(STRIDE_BYTES as u64 == STRIDE);

/// Bytes per record at [`VERSION_V2`].
///
/// **Frozen, and unpinnable — for once those are the same sentence.** Every
/// other layout constant here is checked against `cli`'s, but `cli` keeps its
/// version-2 constants private, so there is nothing to name. That is safe in
/// the one direction that matters: a released format version is history, and
/// §3 rule 8 forbids mutating it in place. 213 cannot become anything else
/// without a version 2 that is not the version 2 already on disk.
///
/// The pin that DOES exist is the one that catches the real failure — if `cli`
/// ships a version 4, [`STRIDE_BYTES`] stops equalling
/// `cli::results::STRIDE_BYTES` and this crate fails to compile, which is
/// exactly how the drift this constant was added during got caught.
const STRIDE_V2: u64 = 213;

/// [`STRIDE_V2`] as a `usize`, for the version-2 record array.
const STRIDE_BYTES_V2: usize = 213;

const _: () = assert!(STRIDE_BYTES_V2 as u64 == STRIDE_V2);

/// The stride a given version addresses records at.
///
/// One function rather than an `if version ==` at each call site, for the
/// reason `cli::results::stride_of` gives: two copies of a branch that decides
/// how to interpret bytes on disk is how a reader comes to decode one version
/// with the other's offsets, which parses cleanly and renders as a run that
/// never happened.
///
/// An unknown version cannot reach here — [`read_from`] refuses it before any
/// address is computed — so the fallback arm is the older format and never a
/// guess.
const fn stride_of(version: u32) -> u64 {
    if version == VERSION {
        STRIDE
    } else {
        STRIDE_V2
    }
}

/// **The link to the writer, which until version 2 was a claim rather than a
/// check — and the claim was false.**
///
/// The comment on [`FIELD_SUM`] used to argue that `cli` could not change the
/// stride without breaking this crate's build. It could, and it did. When `cli`
/// went from 205 to 213 nothing here referenced its constant, so `STRIDE_BYTES`
/// and `FIELD_SUM` went on agreeing with EACH OTHER and this crate compiled
/// perfectly while reading a stride the writer had stopped using. Two
/// declarations that are only checked against themselves are not cross-checked
/// at all.
///
/// What actually caught it was the version check in [`read_from`], which
/// refused the file and printed a sentence naming both versions — the loud
/// degrade `CLAUDE.md` §4 demands, doing exactly its job. But a refusal is the
/// LAST line, not the first: it fires at runtime, on the operator's screen,
/// after a release. This assertion fires at compile time, in CI, before one.
///
/// `api` already depends on `cli` — that arrow was added for `sweeprun` — so
/// the pin costs one `const` and no new dependency. The layout stays
/// independently declared, because importing `cli`'s READER would import its
/// O(runs) duplicate-detection pass at open, which this crate deliberately does
/// not pay. Independent decode, pinned constants.
const _: () = assert!(STRIDE_BYTES == cli::results::STRIDE_BYTES);

/// Bytes of a record that the fields occupy: everything before the seal.
///
/// 253, and version 2's is 205. Neither bump MOVED a field: version 2 appended
/// a seal and version 3 appended the mask as the last field before it, which is
/// why every offset in [`Run::from_bytes`] is unchanged across both and why
/// [`widen_v2`] can be a copy rather than a re-layout.
const PAYLOAD_BYTES: usize = STRIDE_BYTES - SEAL_BYTES;

/// Version 2's payload: its stride less the same eight-byte seal.
const PAYLOAD_BYTES_V2: usize = STRIDE_BYTES_V2 - SEAL_BYTES;

/// **Version 2's payload is EXACTLY version 3's, up to the mask.**
///
/// This is the assumption [`widen_v2`] rests on, checked at compile time rather
/// than trusted. The mask was APPENDED as the last field before the seal, so
/// every version-2 offset is unchanged in version 3. Were a field ever inserted
/// rather than appended, this fails the build instead of the reader silently
/// decoding one field into another.
///
/// The same assertion stands in `cli::results`. Two independent decoders each
/// proving the append separately is the point of the second declaration — a
/// check that lives only beside the writer proves nothing about the reader.
const _: () = assert!(PAYLOAD_BYTES_V2 + 8 * 6 == PAYLOAD_BYTES);

/// Bytes of `blake3` kept as the per-record seal, matching `cli::results`.
const SEAL_BYTES: usize = 8;

/// Every field's width, added up in the writer's own order.
///
/// **This is the check that makes a second declaration of the layout safe.**
/// `identity` 32, `finished_micros` 8, three 16-byte text fields, the span's
/// `u16 + u8` twice, `months_asked` and `months_found` 4 each, `bars`,
/// `min_hits` and `combinations` 8 each, `depth` 4, `halted` 1, `trades` 8,
/// seven `i64` figures at 8, five `i16` exit rungs at 2, and six `u64` mask
/// words at 8.
///
/// The mask summand is LAST because the writer writes it last. Reading this
/// sum in a different order from `cli::results::Record::to_bytes` would make
/// the two impossible to diff by eye, which is the only way a human ever checks
/// a second declaration of a layout.
///
/// Checked against [`PAYLOAD_BYTES`], not [`STRIDE_BYTES`]: the seal is not a
/// field and no offset below addresses it.
///
/// **This assertion proves the fields agree with each other. It does NOT prove
/// they agree with the writer** — that is what the `cli::results::STRIDE_BYTES`
/// pin above is for, and the absence of that pin is how version 2 shipped past
/// a build that had every reason to look green.
const FIELD_SUM: usize = 32
    + 8
    + 16
    + 16
    + 16
    + (2 + 1)
    + (2 + 1)
    + 4
    + 4
    + 8
    + 8
    + 8
    + 4
    + 1
    + 8
    + 8
    + 8
    + 8
    + 8
    + 8
    + 8
    + 8
    + (5 * 2)
    + (6 * 8);

const _: () = assert!(FIELD_SUM == PAYLOAD_BYTES);

/// Eight bytes of `blake3` over the record's payload, matching
/// `cli::results::seal_of` — same hasher, same 253 bytes, same truncation.
///
/// Recomputed here rather than imported because `cli`'s is private, and it is
/// four lines. If the two ever disagree the seal check below fails on every
/// record and the page says so loudly, which is a visible failure rather than a
/// silent one.
fn seal_of(raw: &[u8; STRIDE_BYTES]) -> [u8; SEAL_BYTES] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(&raw[..PAYLOAD_BYTES]);
    let full = hasher.finalize();
    let mut out = [0_u8; SEAL_BYTES];
    out.copy_from_slice(&full[..SEAL_BYTES]);
    out
}

/// Whether a version-2 record matches the seal written beside it.
///
/// **Separate from [`seal_of`] because the two hash different lengths**, and
/// that is not a tidiness point: version 2's seal covers 205 bytes and version
/// 3's covers 253. One function taking a length would be the dynamic schema
/// `CLAUDE.md` §4 bans; two constants and two functions is the whole of it.
fn seal_matches_v2(raw: &[u8; STRIDE_BYTES_V2]) -> bool {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(&raw[..PAYLOAD_BYTES_V2]);
    let full = hasher.finalize();
    full[..SEAL_BYTES] == raw[PAYLOAD_BYTES_V2..STRIDE_BYTES_V2]
}

/// A version-2 record laid out as version 3's bytes.
///
/// The payload is copied verbatim and the six mask words are left zero, which
/// is legal ONLY because the mask was appended rather than inserted — the
/// `PAYLOAD_BYTES_V2 + 8 * 6 == PAYLOAD_BYTES` assertion above is that proof,
/// checked by the compiler rather than by this comment.
///
/// **The seal is deliberately NOT copied.** It covers 205 bytes and version 3's
/// covers 253, so carrying it across would make every widened record fail its
/// own check and the page would mark three healthy runs as damaged. The seal is
/// verified against the ORIGINAL bytes by [`seal_matches_v2`], before this
/// runs, and the result is carried separately — see [`read_from`].
fn widen_v2(raw: &[u8; STRIDE_BYTES_V2]) -> [u8; STRIDE_BYTES] {
    let mut wide = [0_u8; STRIDE_BYTES];
    wide[..PAYLOAD_BYTES_V2].copy_from_slice(&raw[..PAYLOAD_BYTES_V2]);
    wide
}

/// One record at a byte offset, widened to [`VERSION`]'s layout, and whether
/// its seal held.
///
/// **One function for both versions, so the branch that decides how to
/// interpret bytes on disk exists exactly once.** Two copies of it is how a
/// reader comes to decode one version with the other's offsets — which parses
/// cleanly, renders as a run that never happened, and is the failure this whole
/// module is shaped to refuse.
///
/// The seal is RETURNED rather than acted on, because the caller and this
/// function want different things from a bad one: a damaged record still
/// occupies its stride and the records around it are still addressable, so
/// [`read_from`] marks the row and keeps going rather than emptying the page.
fn read_at<R: std::io::Read + std::io::Seek>(
    src: &mut R,
    at: u64,
    version: u32,
) -> std::io::Result<([u8; STRIDE_BYTES], bool)> {
    if version == VERSION {
        let mut raw = [0_u8; STRIDE_BYTES];
        src.seek(SeekFrom::Start(at))?;
        src.read_exact(&mut raw)?;
        let sealed = seal_of(&raw) == raw[PAYLOAD_BYTES..STRIDE_BYTES];
        return Ok((raw, sealed));
    }
    let mut raw = [0_u8; STRIDE_BYTES_V2];
    src.seek(SeekFrom::Start(at))?;
    src.read_exact(&mut raw)?;
    // SEALED AGAINST ITS OWN LENGTH, AND ONLY THEN WIDENED. The other order
    // hashes 253 bytes of which 48 are zeroes this crate invented, and every
    // version-2 record in existence fails a check it was never given.
    let sealed = seal_matches_v2(&raw);
    Ok((widen_v2(&raw), sealed))
}

/// The most records one request will read.
///
/// **A record count, and it is no longer a byte budget — it was, and saying so
/// is the point.** The original rationale was that 20,000 × 205 bytes is
/// 4.1 MB, deliberately matching the budget [`crate::logs::SCAN_BYTES`] gives
/// one log request. That parity did not survive the stride: at [`STRIDE`] the
/// same 20,000 records are 5,220,000 bytes, which OVERSHOOTS `SCAN_BYTES`
/// (4 MiB, 4,194,304) by about a quarter.
///
/// The number is left where it is rather than quietly retuned to 16,069,
/// because the ceiling is a promise about how many rows a page may ask for and
/// moving it is a behaviour change that belongs in `docs/05-decisions.md`, not
/// in a comment repair. What is fixed here is the CLAIM: the two limits are
/// related in origin and no longer equal, and a reader who needs them equal now
/// has to choose.
///
/// The ledger on disk holds three records today — the earlier text said zero,
/// which stopped being true the first time a sweep was recorded — and the
/// ceiling exists so that a query string can never make this server read the
/// disk without a bound somebody chose.
///
/// When it bites, the answer says `hit_scan_cap: true` and reports `total`
/// separately from `scanned`, so the page can say "the newest 20,000 of
/// 31,904" rather than quietly presenting a window as the whole.
pub const MAX_RUNS: usize = 20_000;

/// One completed run, as the ledger stores it.
///
/// Field for field `cli::results::Record`, with two differences that are both
/// about being read rather than written: `halted` is a [`bool`] because nothing
/// here needs the byte, and the three text fields are [`String`] because a
/// page renders text and not padding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Run {
    /// Where in the file this record sits. The ledger is append-only, so the
    /// index is also the order it was recorded in and never changes.
    pub index: u64,
    /// `blake3` of the nine terms `CLAUDE.md` §3 rule 3 names, as lowercase
    /// hex. The only thing that actually names a run.
    pub identity: String,
    /// When the run finished, microseconds since the epoch.
    pub finished_micros: i64,
    /// The feed's directory word.
    pub feed: String,
    /// The instrument.
    pub underlying: String,
    /// The SIGNAL rung. Execution is always one-minute.
    pub timeframe: String,
    /// First month of the span asked for.
    pub from_year: u16,
    /// First month of the span asked for.
    pub from_month: u8,
    /// Last month of the span asked for.
    pub to_year: u16,
    /// Last month of the span asked for.
    pub to_month: u8,
    /// Months the span asked for.
    pub months_asked: u32,
    /// Months the store actually held. Below `months_asked` means a HOLE, and
    /// every figure in this record is over a SHORTER sample — not a corrected
    /// one.
    pub months_found: u32,
    /// Signal bars swept.
    pub bars: u64,
    /// The support threshold applied.
    pub min_hits: u64,
    /// Combinations the ladder produced.
    pub combinations: u64,
    /// Deepest level reached. PARTIAL when [`Self::halted`].
    pub depth: u32,
    /// A budget stopped the walk short. **The field that decides whether the
    /// others mean anything** — see this module's header.
    pub halted: bool,
    /// Round trips the chosen combination took.
    pub trades: u64,
    /// Its total under worst-case fills, in paisa. **The figure selection ranks
    /// on**, everywhere in this workspace.
    pub pessimistic: i64,
    /// Its total under best-case fills, in paisa. The flattering number, and
    /// never the one a page leads with.
    pub optimistic: i64,
    /// Worst single round trip, in paisa.
    pub worst_trade: i64,
    /// Worst peak-to-trough, in paisa.
    pub max_drawdown: i64,
    /// Winners' adverse excursion, ppm — the tightest stop that would not have
    /// killed a winner.
    pub winner_mae: i64,
    /// Winners' favourable excursion, ppm.
    pub winner_mfe: i64,
    /// Every trade's adverse excursion, ppm.
    pub all_mae: i64,
    /// Chosen exit rungs, `-1` for "no rung": stop, target, TSL, TTP arm, TTP
    /// trail.
    pub exit_rungs: [i16; 5],
    /// The combination itself, as its six mask words — version 3's whole
    /// difference from version 2.
    ///
    /// Before it, a record carried `identity` — a `blake3` over the nine terms
    /// §3 rule 3 names — which identifies the RUN and cannot be turned back
    /// into the conditions. The ledger could say what a combination was worth
    /// and never which conditions made it.
    ///
    /// **Six zero words are ambiguous on their own, and [`Ledger::version`] is
    /// how this crate refuses to pretend otherwise.** A version-2 record
    /// widened by [`widen_v2`] has an all-zero mask because version 2 had no
    /// mask; a version-3 record may have an all-zero mask because that run
    /// genuinely recorded no combination. The bytes are identical and no amount
    /// of looking at them separates the two. Rendering both as "no combination"
    /// would be the fallback that hides a failure `CLAUDE.md` §4 bans, so the
    /// version travels with the ledger and the page says "this ledger predates
    /// the mask" for the one and "no combination was recorded" for the other.
    ///
    /// Not decoded to condition names here: that needs `vocab`, and `api` does
    /// not depend on it. Adding the arrow to render a field would be a §5 crate
    /// graph change made for a convenience, so the words are served raw and the
    /// front end names them.
    pub mask_words: [u64; 6],
    /// Whether the record's own `blake3` seal matches the bytes read back.
    ///
    /// **A false here does not mean the numbers above are wrong — it means they
    /// cannot be trusted to be right**, which is a different claim and the only
    /// honest one. Every byte pattern is a legal value of its type, so a record
    /// damaged after it was written parses cleanly and renders as a run that
    /// never happened. Version 1 had no way to tell the two apart.
    ///
    /// Reported per record rather than fatally, for the same reason
    /// `partial_tail` is: one damaged record must not empty the page of the
    /// good ones beside it. The page shows the row and marks it, which is the
    /// loud degrade `CLAUDE.md` §4 requires — a silently dropped row would be
    /// the fallback that hides a failure it bans.
    pub sealed: bool,
}

impl Run {
    /// The record from its exact [`STRIDE`] bytes, at the index it was read
    /// from.
    ///
    /// Infallible by construction, exactly as the writer's `from_bytes` is:
    /// every field is fixed-width and every byte pattern is a legal value of
    /// its type. A record whose CONTENT is nonsensical is a separate question,
    /// and pretending the parse can fail is not how it gets answered.
    ///
    /// # Why `sealed` is a PARAMETER and not computed here
    ///
    /// It used to be computed, as `seal_of(raw) == raw[PAYLOAD_BYTES..]`, and
    /// that was correct while one version existed. It cannot survive two: a
    /// version-2 record arrives here already widened by [`widen_v2`], with its
    /// seal slot zeroed and 48 bytes of mask this crate invented. Hashing those
    /// 253 bytes answers a question nobody asked, and would answer it "damaged"
    /// for every version-2 record in existence — three healthy runs marked as
    /// corrupt on the operator's page.
    ///
    /// The seal must be checked against the bytes as they were WRITTEN, at the
    /// length that version sealed. [`read_from`] does that before widening and
    /// passes the verdict down. Same split, and for the same reason, as
    /// `cli::results::read_at` returning its seal alongside its bytes.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "every index is a constant offset into a fixed-size array \
                  whose length is const-asserted against the field sum by \
                  FIELD_SUM, so no offset here can be out of bounds"
    )]
    fn from_bytes(index: u64, raw: &[u8; STRIDE_BYTES], sealed: bool) -> Self {
        let mut at = 0_usize;
        let take = |n: usize, at: &mut usize| {
            let slice = &raw[*at..*at + n];
            *at += n;
            slice
        };
        let mut identity = String::with_capacity(64);
        for byte in take(32, &mut at) {
            let _ = write!(identity, "{byte:02x}");
        }
        let finished_micros = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let feed = text(take(16, &mut at));
        let underlying = text(take(16, &mut at));
        let timeframe = text(take(16, &mut at));
        let from_year = u16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        let from_month = take(1, &mut at).first().copied().unwrap_or(0);
        let to_year = u16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        let to_month = take(1, &mut at).first().copied().unwrap_or(0);
        let months_asked = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let months_found = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let bars = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let min_hits = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let combinations = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let depth = u32::from_le_bytes(take(4, &mut at).try_into().unwrap_or([0; 4]));
        let halted = take(1, &mut at).first().copied().unwrap_or(0) != 0;
        let trades = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let pessimistic = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let optimistic = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let worst_trade = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let max_drawdown = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let winner_mae = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mfe_winners = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let all_mae = i64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        let mut exit_rungs = [0_i16; 5];
        for slot in &mut exit_rungs {
            *slot = i16::from_le_bytes(take(2, &mut at).try_into().unwrap_or([0; 2]));
        }
        // LAST, matching the writer. A version-2 record reaches here already
        // widened, so these six words read the zeroes `widen_v2` left and not
        // whatever followed the record on disk.
        let mut mask_words = [0_u64; 6];
        for slot in &mut mask_words {
            *slot = u64::from_le_bytes(take(8, &mut at).try_into().unwrap_or([0; 8]));
        }
        Self {
            index,
            identity,
            finished_micros,
            feed,
            underlying,
            timeframe,
            from_year,
            from_month,
            to_year,
            to_month,
            months_asked,
            months_found,
            bars,
            min_hits,
            combinations,
            depth,
            halted,
            trades,
            pessimistic,
            optimistic,
            worst_trade,
            max_drawdown,
            winner_mae,
            winner_mfe: mfe_winners,
            all_mae,
            exit_rungs,
            mask_words,
            sealed,
        }
    }

    /// Whether the span the run asked for was entirely on disk.
    ///
    /// A run over a span with a hole is a SHORTER sample, and every figure it
    /// carries is over that shorter sample. The page shows both numbers wherever
    /// a run is shown, so this is for the one-word verdict beside them.
    #[must_use]
    pub fn whole_span(&self) -> bool {
        self.months_found >= self.months_asked
    }

    /// This run as one JSON object.
    ///
    /// Numbers stay numbers. A count that arrives as `1200` and leaves as
    /// `"1200"` makes every consumer parse it back, and the first one to forget
    /// compares a string — the defect [`crate::logs::value_json`]'s own header
    /// records. Paisa are emitted as the integers they are and divided by the
    /// page: `CLAUDE.md` §7 keeps money in `i64` and this crate denies
    /// `float_arithmetic` outright.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(640);
        out.push('{');
        let _ = write!(out, r#""index":{}"#, self.index);
        let _ = write!(
            out,
            r#","identity":{}"#,
            render::json_string(&self.identity)
        );
        let _ = write!(out, r#","finished_micros":{}"#, self.finished_micros);
        let _ = write!(out, r#","feed":{}"#, render::json_string(&self.feed));
        let _ = write!(
            out,
            r#","underlying":{}"#,
            render::json_string(&self.underlying)
        );
        let _ = write!(
            out,
            r#","timeframe":{}"#,
            render::json_string(&self.timeframe)
        );
        let _ = write!(out, r#","from_year":{}"#, self.from_year);
        let _ = write!(out, r#","from_month":{}"#, self.from_month);
        let _ = write!(out, r#","to_year":{}"#, self.to_year);
        let _ = write!(out, r#","to_month":{}"#, self.to_month);
        let _ = write!(out, r#","months_asked":{}"#, self.months_asked);
        let _ = write!(out, r#","months_found":{}"#, self.months_found);
        let _ = write!(out, r#","whole_span":{}"#, self.whole_span());
        let _ = write!(out, r#","bars":{}"#, self.bars);
        let _ = write!(out, r#","min_hits":{}"#, self.min_hits);
        let _ = write!(out, r#","combinations":{}"#, self.combinations);
        let _ = write!(out, r#","depth":{}"#, self.depth);
        let _ = write!(out, r#","halted":{}"#, self.halted);
        let _ = write!(out, r#","sealed":{}"#, self.sealed);
        let _ = write!(out, r#","trades":{}"#, self.trades);
        let _ = write!(out, r#","pessimistic":{}"#, self.pessimistic);
        let _ = write!(out, r#","optimistic":{}"#, self.optimistic);
        let _ = write!(out, r#","worst_trade":{}"#, self.worst_trade);
        let _ = write!(out, r#","max_drawdown":{}"#, self.max_drawdown);
        let _ = write!(out, r#","winner_mae":{}"#, self.winner_mae);
        let _ = write!(out, r#","winner_mfe":{}"#, self.winner_mfe);
        let _ = write!(out, r#","all_mae":{}"#, self.all_mae);
        out.push_str(r#","exit_rungs":["#);
        for (n, rung) in self.exit_rungs.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(out, "{rung}");
        }
        out.push(']');
        // **STRINGS, AND THIS IS THE ONE PLACE THE "numbers stay numbers" RULE
        // ABOVE MUST NOT APPLY.** A JSON number is an IEEE-754 double
        // everywhere it is parsed, so it carries 53 bits exactly. A mask word
        // is 64, and the vocabulary sets high bits — `1 << 63` alone is
        // 9,223,372,036,854,775,808, which a browser reads back as
        // 9,223,372,036,854,775,808 rounded to the nearest double and decodes
        // into a DIFFERENT set of conditions. It would not throw; it would
        // quietly name the wrong bits, which is the confident nonsense §4 bans.
        //
        // The rule this departs from is about counts that get COMPARED, where a
        // forgotten parse silently compares strings. A mask is never compared
        // as a magnitude — it is decoded — so the harm the rule prevents is not
        // available here, and the harm it would cause is. Decimal, so
        // `BigInt(w)` takes it as it stands.
        out.push_str(r#","mask_words":["#);
        for (n, word) in self.mask_words.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            let _ = write!(out, r#""{word}""#);
        }
        out.push_str("]}");
        out
    }
}

/// A fixed-width text field with its zero padding removed.
///
/// Lossy rather than refusing: these fields are for a human reading a listing
/// and the identity is what names the run. Throwing away a completed sweep
/// because an instrument name held a stray byte would be the fallback that
/// hides a failure in reverse — a refusal that hides a result.
fn text(raw: &[u8]) -> String {
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(raw.get(..end).unwrap_or(&[])).into_owned()
}

/// What one request read, and every fact about what it could not.
///
/// **Every field here is a fact the page prints.** There is no field whose
/// absence renders as a blank: an empty ledger, a partial tail and a refused
/// open each carry their own sentence, because `CLAUDE.md` §4 bans a fallback
/// that hides a failure and a blank cell is the quietest fallback there is.
#[derive(Clone, Debug, Default)]
pub struct Ledger {
    /// Where the file is, so a page can say which one it read. Present even
    /// when the read failed — "which file" is the first question a refusal
    /// raises.
    pub path: PathBuf,
    /// The format version the file declared in its own header.
    ///
    /// **Carried because an all-zero mask means two different things and the
    /// bytes cannot tell them apart.** A version-2 record predates
    /// [`Run::mask_words`] entirely and is widened with six zero words; a
    /// version-3 record may hold six zero words because that run recorded no
    /// combination. Rendering both as "no combination" would be the fallback
    /// that hides a failure `CLAUDE.md` §4 bans — one is an absent field, the
    /// other is a present and empty one.
    ///
    /// Zero when the read was refused before the header could be believed, and
    /// [`Self::refusal`] is what the page should show then. Zero is not a
    /// format version this repository has ever written, so it cannot collide
    /// with a real one.
    pub version: u32,
    /// Records the file holds, from its length. **O(1)** and independent of
    /// how many were read.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    pub total: u64,
    /// Records actually read into [`Self::runs`].
    pub scanned: u64,
    /// [`MAX_RUNS`] stopped the read short, so [`Self::runs`] is the newest
    /// window and not the whole ledger.
    pub hit_scan_cap: bool,
    /// The trailing bytes are not a whole record — a writer interrupted
    /// mid-append, or a file truncated. The whole records before it are still
    /// read and this says the tail was ignored.
    pub partial_tail: bool,
    /// The runs, **newest first**.
    pub runs: Vec<Run>,
    /// Why nothing could be read, in the operator's words. [`None`] when the
    /// read succeeded — including when it succeeded and found nothing, which is
    /// a different state and gets its own sentence on the page.
    pub refusal: Option<String>,
}

impl Ledger {
    /// The best COMPLETE run, by the figure selection ranks on.
    ///
    /// **Halted rows are excluded, and that is the whole point of the
    /// function. UNSEALED rows are excluded for a stronger reason.** A halted
    /// run is honest and incomplete; an unsealed one is bytes that parse and
    /// may describe a run that never happened. Ranking is the one place a
    /// damaged record does maximum harm — it does not add a bad row to a table
    /// the operator can scan past, it names the ANSWER — and a corrupted
    /// `pessimistic` is as likely to be enormous as tiny, so the damaged record
    /// is disproportionately likely to win. It is excluded here and still shown
    /// in the table, marked.
    ///
    /// A halted ladder stopped short, so its `combinations` covers
    /// less of the search while reading larger, and crowning it would propose a
    /// winner no other surface in this workspace agrees with. `cli`'s
    /// `best_complete_line` does exactly this and says `NO COMPLETE RUN` when
    /// nothing survives; [`None`] is that sentence's shape here.
    ///
    /// Ranks on `pessimistic` — worst-case fills — because that is what
    /// selection ranks on everywhere else. Ranking on `optimistic` here would
    /// make this page name a different winner from the CLI, and the CLI reads
    /// the same file.
    ///
    /// Ties break toward the LOWER index, which is the earlier run: two
    /// identical totals are the same answer, and the one that was found first
    /// is the one that has been on disk longest.
    ///
    /// **Stated as a sort key rather than as a comparison chain, and the
    /// reason is a surviving mutant.** The obvious `reduce` spells the rule as
    /// `run.pessimistic > best.pessimistic || (== && run.index < best.index)`,
    /// and `cargo mutants` turns that final `<` into `<=` without a single test
    /// noticing. Nothing is wrong with the suite: the two operators differ only
    /// when two runs share an index, and [`Ledger::read`] numbers them by
    /// position, so they never do. It is an EQUIVALENT mutant, and the honest
    /// options were to test an impossible state or to remove the operator.
    ///
    /// This removes it. `(pessimistic, Reverse(index))` is a total order — the
    /// key is unique because the index is — so there is no tie left to break
    /// and no tie-breaking operator left to mutate. It also reads as the rule
    /// the doc comment above states, which the chain did not.
    #[must_use]
    pub fn best_complete(&self) -> Option<&Run> {
        self.runs
            .iter()
            .filter(|run| !run.halted && run.sealed)
            .max_by_key(|run| (run.pessimistic, std::cmp::Reverse(run.index)))
    }

    /// How many of the runs read are halted.
    ///
    /// Surfaced as its own number so the page can say "4 of 31 are not
    /// comparable" rather than leaving the operator to count amber rails.
    #[must_use]
    pub fn halted_count(&self) -> usize {
        self.runs.iter().filter(|run| run.halted).count()
    }

    /// How many of the runs read failed their integrity seal.
    ///
    /// Its own number for the same reason `halted_count` is: the operator needs
    /// to know that some rows are untrustworthy WITHOUT reading every row to
    /// find out. Zero is the answer on every healthy ledger, which is what
    /// makes a non-zero one worth a banner.
    ///
    /// Distinct from `halted_count` and never merged with it. A halted run is
    /// one the ENGINE stopped short — the bytes are perfect and the run is
    /// simply incomplete. An unsealed run is one the bytes cannot vouch for,
    /// and it may be neither halted nor complete but noise that parses.
    #[must_use]
    pub fn unsealed_count(&self) -> usize {
        self.runs.iter().filter(|run| !run.sealed).count()
    }

    /// The whole answer as JSON.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::with_capacity(1024 + self.runs.len() * 640);
        out.push('{');
        let _ = write!(
            out,
            r#""path":{}"#,
            render::json_string(&self.path.display().to_string())
        );
        let _ = write!(out, r#","version":{}"#, self.version);
        // WHETHER THE MASK IS A FIELD OR AN ABSENCE, said once for the ledger
        // rather than guessed per row. A version-2 file has no mask at all, so
        // `mask_words` on every run below it is six zeroes this crate wrote and
        // not six zeroes the sweep recorded.
        let _ = write!(
            out,
            r#","has_mask":{}"#,
            self.version >= VERSION && self.refusal.is_none()
        );
        let _ = write!(out, r#","total":{}"#, self.total);
        let _ = write!(out, r#","scanned":{}"#, self.scanned);
        let _ = write!(out, r#","hit_scan_cap":{}"#, self.hit_scan_cap);
        let _ = write!(out, r#","partial_tail":{}"#, self.partial_tail);
        let _ = write!(out, r#","max_runs":{MAX_RUNS}"#);
        let _ = write!(out, r#","halted":{}"#, self.halted_count());
        let _ = write!(out, r#","unsealed":{}"#, self.unsealed_count());
        match self.best_complete() {
            Some(run) => {
                let _ = write!(out, r#","best_complete":{}"#, run.index);
            }
            None => out.push_str(r#","best_complete":null"#),
        }
        match self.refusal {
            Some(ref why) => {
                let _ = write!(out, r#","refusal":{}"#, render::json_string(why));
            }
            None => out.push_str(r#","refusal":null"#),
        }
        out.push_str(r#","runs":["#);
        for (n, run) in self.runs.iter().enumerate() {
            if n > 0 {
                out.push(',');
            }
            out.push_str(&run.to_json());
        }
        out.push_str("]}");
        out
    }

    /// A ledger that could not be read, carrying the reason and the path.
    fn refused(path: PathBuf, why: String) -> Self {
        Self {
            path,
            refusal: Some(why),
            ..Self::default()
        }
    }
}

/// Where the ledger lives beneath a store root: `<root>/results/runs.bin`.
///
/// The same two segments `cli::results::Results::path` joins. Stated here as
/// its own function so a test can name the file without reconstructing the
/// path, and so the refusal above can print it.
#[must_use]
pub fn path_in(root: &Path) -> PathBuf {
    root.join("results").join("runs.bin")
}

/// The newest `limit` runs beneath a store root, and every fact about the read.
///
/// **Newest first, and by seeking rather than by sorting.** The file is
/// append-only, so index order IS recording order; the newest `limit` records
/// are the last `limit` addresses, and each is one seek. Reading the whole file
/// and reversing it would be O(total) for an answer this returns in O(limit),
/// and on a ledger of 31,904 runs that is the difference between 4 MB and 6.5
/// MB read per refresh of a page an operator leaves open.
///
/// Every failure returns a [`Ledger`] carrying a sentence, never an empty one:
/// "there are no runs" and "I could not read the runs" are different facts and
/// a page that renders both as an empty table has told the operator a lie about
/// one of them.
///
/// This function owns exactly one decision — whether the file could be OPENED —
/// and hands everything else to [`read_from`]. See that function for why the
/// split exists.
#[must_use]
pub fn read(root: &Path, limit: usize) -> Ledger {
    let path = path_in(root);
    match File::open(&path) {
        Ok(mut file) => read_from(path, &mut file, limit),
        // NOT AN ERROR, AND THE SENTENCE SAYS SO. A store that has never been
        // swept has no ledger, and rendering that as a failure would teach an
        // operator to distrust a correct answer. Separated from every other
        // open failure by `ErrorKind` rather than by string matching.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ledger::refused(
            path.clone(),
            format!(
                "no results file at {}. Nothing has been swept into this store \
                 yet — the ledger is created by the first run `cli` records, not \
                 by this server. This is not an error.",
                path.display()
            ),
        ),
        Err(why) => Ledger::refused(
            path.clone(),
            format!("{} could not be opened: {why}", path.display()),
        ),
    }
}

/// [`read`], over any source a caller already has open.
///
/// # Why this is split out
///
/// **So a test can state a state it cannot cause.** [`crate::logs`] took this
/// same split for the same reason, in its own words: `hit_scan_cap` needs four
/// megabytes of log and an unreadable file is not something a unit test can
/// arrange, *"so every branch that reports one would otherwise render for the
/// first time in production"*.
///
/// The failures here are worse than that, because each one is a sentence an
/// operator reads at the moment something has already gone wrong. A refusal
/// whose first execution is in front of a person is a refusal nobody has
/// checked. Over a generic `R` every branch below is reachable: a
/// `std::io::Cursor` states the file contents exactly, and a reader that fails
/// on demand states the I/O errors exactly.
///
/// The LENGTH is taken by seeking to the end rather than from
/// `File::metadata`, and that is also for reachability: a metadata call on an
/// already-open handle has no failure a test can produce, so a branch reporting
/// it could never be exercised. A seek has one, and it is the same failure.
#[expect(
    clippy::too_many_lines,
    reason = "the length is a sequence of REFUSALS, one per way the file can be \
              wrong, each with the sentence an operator reads. Splitting them \
              into helpers would scatter the seven refusals across seven \
              functions and hide the order they are checked in, which is the \
              one thing a reader of this function needs"
)]
fn read_from<R: std::io::Read + std::io::Seek>(path: PathBuf, src: &mut R, limit: usize) -> Ledger {
    let len = match src.seek(SeekFrom::End(0)) {
        Ok(len) => len,
        Err(why) => {
            return Ledger::refused(
                path.clone(),
                format!("{} could not be measured: {why}", path.display()),
            );
        }
    };

    if len < HEADER {
        return Ledger::refused(
            path.clone(),
            format!(
                "{} is {len} bytes and the header alone is {HEADER}. A file \
                 shorter than its own header has no version to check and no \
                 record to read, so nothing is parsed from it.",
                path.display()
            ),
        );
    }

    let mut header = [0_u8; HEADER_BYTES];
    if let Err(why) = src
        .seek(SeekFrom::Start(0))
        .and_then(|_| src.read_exact(&mut header))
    {
        return Ledger::refused(
            path.clone(),
            format!("the header of {} could not be read: {why}", path.display()),
        );
    }
    if header.get(..8) != Some(&MAGIC) {
        return Ledger::refused(
            path.clone(),
            format!(
                "{} is not a brutex results file: its first eight bytes are not \
                 `BRUTEXRS`. Nothing was parsed from it. Check that the store \
                 root is the one the sweep wrote to.",
                path.display()
            ),
        );
    }
    let version = u32::from_le_bytes(
        header
            .get(8..12)
            .and_then(|s| s.try_into().ok())
            .unwrap_or([0; 4]),
    );
    if version != VERSION && version != VERSION_V2 {
        return Ledger::refused(
            path.clone(),
            format!(
                "{} is version {version}. This build reads versions {VERSION_V2} \
                 and {VERSION}. A new field is a new file version at its own \
                 stride, so this file's records are neither {STRIDE_V2} nor \
                 {STRIDE} bytes and every address computed from either would \
                 land mid-record. Nothing was parsed.",
                path.display()
            ),
        );
    }
    // EVERY ADDRESS BELOW COMES FROM THE FILE'S OWN VERSION, not from the
    // newest one. Reading a version-2 file at version 3's stride is the exact
    // failure the refusal above prevents for an UNKNOWN version, and it would
    // be no less wrong for a known one.
    let stride = stride_of(version);

    let body = len.saturating_sub(HEADER);
    let total = body / stride;
    // A RAGGED TAIL IS REPORTED, NOT REPAIRED AND NOT FATAL. `cli` appends one
    // whole stride and flushes, so a partial tail means the writer was
    // interrupted — a full disk, a kill. The whole records before it are
    // perfectly good and are served; the bytes after them are named and left
    // alone, because this crate never writes to this file.
    let partial_tail = body % stride != 0;

    let want = u64::try_from(limit.min(MAX_RUNS)).unwrap_or(0);
    let take = want.min(total);
    let hit_scan_cap = total > take;

    let mut runs = Vec::with_capacity(usize::try_from(take).unwrap_or(0));
    let mut scanned = 0_u64;
    // NEWEST FIRST, BY WALKING BACKWARD FROM THE LAST ADDRESS. `total - 1` is
    // the newest record because the file is append-only; `n` counts how many
    // have been taken, so the loop is O(take) and never touches the rest.
    for n in 0..take {
        let index = total.saturating_sub(1).saturating_sub(n);
        let at = HEADER.saturating_add(index.saturating_mul(stride));
        let (raw, sealed) = match read_at(src, at, version) {
            Ok(pair) => pair,
            Err(why) => {
                // PARTIAL, NOT NOTHING. The records already read are returned
                // alongside the sentence saying where the read stopped. Throwing
                // them away would turn one unreadable record into an empty page.
                return Ledger {
                    path: path.clone(),
                    version,
                    total,
                    scanned,
                    hit_scan_cap,
                    partial_tail,
                    runs,
                    refusal: Some(format!(
                        "record {index} of {} could not be read at byte {at}: \
                         {why}. The {scanned} newer records above it were read \
                         and are shown.",
                        path.display()
                    )),
                };
            }
        };
        runs.push(Run::from_bytes(index, &raw, sealed));
        scanned = scanned.saturating_add(1);
    }

    Ledger {
        path,
        version,
        total,
        scanned,
        hit_scan_cap,
        partial_tail,
        runs,
        refusal: None,
    }
}

/// How many runs one request returns when it does not say.
///
/// 500 covers every ledger this store has held and every one it will hold for
/// years of operating, so in practice the default IS the whole file and
/// `hit_scan_cap` stays false. It is a default rather than "all" because a
/// route with no ceiling is a route whose cost is decided by the disk.
pub const DEFAULT_LIMIT: usize = 500;

/// The JSON content type every answer from this module carries.
type JsonHeaders = [(axum::http::HeaderName, &'static str); 1];

/// `GET /backtest.json` — every recorded run, newest first.
///
/// # Why this is a JSON route and `/backtest` is NOT registered here
///
/// The front end owns `/backtest`, served by [`crate::assets`] through the
/// router's fallback, exactly as it owns `/db`, `/ingest` and `/autopilot`.
/// Registering a Rust page at that path would put TWO applications on one URL:
/// a registered route beats `Router::fallback` unconditionally, so clicking the
/// nav item would render the Svelte page (the client router never consults the
/// server) while a reload or a bookmark would render the Rust one — different
/// navigation, no feed picker, no theme. That is not hypothetical; it is the
/// defect `web/vite.config.js` records at length against `/audit`, where it is
/// still live. One path, one application.
///
/// # Cost
///
/// `limit` is clamped into `1..=`[`MAX_RUNS`] before it reaches the disk, so a
/// query string cannot ask this server for an unbounded read. The answer
/// reports `total` beside `scanned` and sets `hit_scan_cap` when the two differ,
/// so the page can never present a window as the whole ledger.
///
/// The body is [`respond`], for the reason that function's header gives.
pub async fn backtest_json(uri: axum::http::Uri) -> (axum::http::StatusCode, JsonHeaders, String) {
    respond(crate::server::store_dir(), uri.query().unwrap_or(""))
}

/// [`backtest_json`], over a store root the caller has already resolved.
///
/// **Split so the refusal is testable.** Resolving the root reads the
/// environment, and a test cannot set an environment variable — `set_var` is
/// `unsafe` under edition 2024 and this crate forbids `unsafe`. Taking the
/// `Result` as an argument moves the only untestable line into the handler,
/// which is then one expression long. [`crate::logs::logs_json`] is split
/// exactly here and for exactly this reason.
fn respond(
    root: Result<PathBuf, String>,
    query: &str,
) -> (axum::http::StatusCode, JsonHeaders, String) {
    let json = [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )];
    let root = match root {
        Ok(root) => root,
        // 503, NOT 500. The server is working; its configuration is not, and
        // the two are different things to an operator reading a status code.
        // The body keeps the shape the page parses, so a refusal renders as a
        // sentence rather than as a parse failure on top of a configuration
        // failure.
        Err(why) => {
            return (
                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                json,
                format!(
                    r#"{{"path":"","total":0,"scanned":0,"hit_scan_cap":false,"partial_tail":false,"max_runs":{MAX_RUNS},"halted":0,"unsealed":0,"best_complete":null,"refusal":{},"runs":[]}}"#,
                    render::json_string(&why)
                ),
            );
        }
    };
    (
        axum::http::StatusCode::OK,
        json,
        read(&root, limit_asked(query)).to_json(),
    )
}

/// The `limit` a query string asked for, clamped to what this route will read.
///
/// Clamped rather than refused: a bookmarked `?limit=99999` is not an error, it
/// is an operator who wants everything, and the honest answer is everything up
/// to the ceiling plus the flag that says the ceiling was reached. An
/// unparseable value takes [`DEFAULT_LIMIT`] for the same reason.
fn limit_asked(raw: &str) -> usize {
    crate::server::param(raw, "limit")
        .parse::<usize>()
        .unwrap_or(DEFAULT_LIMIT)
        .clamp(1, MAX_RUNS)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the two exceptions a test module takes here: a test that cannot \
              panic cannot fail, and every index below is a constant offset \
              into a fixture array of known length"
)]
mod tests {
    use super::{
        DEFAULT_LIMIT, HEADER_BYTES, Ledger, MAX_RUNS, PAYLOAD_BYTES, PAYLOAD_BYTES_V2,
        STRIDE_BYTES, VERSION, VERSION_V2, limit_asked, path_in, read, read_from, respond, seal_of,
        text,
    };
    use std::io::{Cursor, Read, Seek, SeekFrom};

    /// A ledger file's exact bytes, built the way `cli::results` builds them.
    ///
    /// Written here rather than imported: importing it would be the `api -> cli`
    /// arrow this module exists to avoid, and a fixture that agrees with the
    /// reader by construction proves nothing. This lays the fields out from the
    /// documented order, so a reader that drifts fails against it.
    fn header(version: u32) -> [u8; HEADER_BYTES] {
        let mut out = [0_u8; HEADER_BYTES];
        out[..8].copy_from_slice(b"BRUTEXRS");
        out[8..12].copy_from_slice(&version.to_le_bytes());
        out
    }

    /// The six mask words [`record`] writes for a given seed.
    ///
    /// Named rather than inlined because three places need the SAME answer: the
    /// writer helper, the round-trip assertion, and the JSON assertion. Two
    /// copies of a literal that must agree is the defect this whole module is
    /// about.
    fn mask_of(n: u8) -> [u64; 6] {
        [u64::from(n), 0, u64::from(n) << 32, 0, 1 << 63, 7]
    }

    /// One record's bytes. `n` seeds every numeric field so a mis-ordered read
    /// lands on the wrong value rather than on a plausible one.
    fn record(n: u8, halted: bool, pessimistic: i64) -> [u8; STRIDE_BYTES] {
        let mut out = [0_u8; STRIDE_BYTES];
        let mut at = 0_usize;
        let mut put = |bytes: &[u8], at: &mut usize| {
            out[*at..*at + bytes.len()].copy_from_slice(bytes);
            *at += bytes.len();
        };
        put(&[n; 32], &mut at); // identity
        put(
            &(1_700_000_000_000_000_i64 + i64::from(n)).to_le_bytes(),
            &mut at,
        );
        let mut feed = [0_u8; 16];
        feed[..7].copy_from_slice(b"zerodha");
        put(&feed, &mut at);
        let mut under = [0_u8; 16];
        under[..5].copy_from_slice(b"NIFTY");
        put(&under, &mut at);
        let mut rung = [0_u8; 16];
        rung[..4].copy_from_slice(b"1day");
        put(&rung, &mut at);
        put(&2019_u16.to_le_bytes(), &mut at); // from_year
        put(&[12], &mut at); // from_month
        put(&2026_u16.to_le_bytes(), &mut at); // to_year
        put(&[8], &mut at); // to_month
        put(&81_u32.to_le_bytes(), &mut at); // months_asked
        put(&u32::from(n).to_le_bytes(), &mut at); // months_found
        put(&1671_u64.to_le_bytes(), &mut at); // bars
        put(&300_u64.to_le_bytes(), &mut at); // min_hits
        put(&458_379_u64.to_le_bytes(), &mut at); // combinations
        put(&15_u32.to_le_bytes(), &mut at); // depth
        put(&[u8::from(halted)], &mut at);
        put(&302_u64.to_le_bytes(), &mut at); // trades
        put(&pessimistic.to_le_bytes(), &mut at);
        put(&86_474_i64.to_le_bytes(), &mut at); // optimistic
        put(&(-500_i64).to_le_bytes(), &mut at); // worst_trade
        put(&(-1_200_i64).to_le_bytes(), &mut at); // max_drawdown
        put(&3_400_i64.to_le_bytes(), &mut at); // winner_mae
        put(&9_100_i64.to_le_bytes(), &mut at); // winner_mfe
        put(&5_600_i64.to_le_bytes(), &mut at); // all_mae
        for rung in [-1_i16, -1, 1, 0, 0] {
            put(&rung.to_le_bytes(), &mut at);
        }
        // LAST, as the writer writes it. The fourth word sets BIT 63 on purpose:
        // that is the value a JSON number cannot carry, so every test that round
        // trips this record through `to_json` exercises the one case where
        // emitting the mask as a number would silently decode to other
        // conditions.
        for word in mask_of(n) {
            put(&word.to_le_bytes(), &mut at);
        }
        // THE FIELDS MUST END EXACTLY WHERE THE SEAL BEGINS. Asserted rather
        // than trusted: if a field above were the wrong width, the seal would
        // be written over a field and every record would fail its own check
        // for a reason that has nothing to do with corruption.
        assert_eq!(at, PAYLOAD_BYTES, "every field is written before the seal");
        let seal = seal_of(&out);
        out[PAYLOAD_BYTES..STRIDE_BYTES].copy_from_slice(&seal);
        out
    }

    /// [`record`], with one payload byte flipped AFTER the seal was computed.
    ///
    /// This is what damage looks like: the record still parses, every field is
    /// still a legal value of its type, and only the seal knows.
    fn damaged(n: u8, halted: bool, pessimistic: i64) -> [u8; STRIDE_BYTES] {
        let mut out = record(n, halted, pessimistic);
        out[0] ^= 0b1000_0000;
        out
    }

    /// A whole file: header plus the records given, in append order.
    fn file(version: u32, records: &[[u8; STRIDE_BYTES]]) -> Vec<u8> {
        let mut out = header(version).to_vec();
        for rec in records {
            out.extend_from_slice(rec);
        }
        out
    }

    /// One VERSION-2 record's bytes, as a version-2 writer laid them out.
    ///
    /// **Built by truncating [`record`], and the truncation is the assertion.**
    /// Version 3 APPENDED the mask, so a version-2 record is byte-for-byte
    /// version 3's payload up to the mask, resealed at its own shorter length.
    /// Deriving it this way rather than re-listing 23 fields means a field that
    /// ever MOVED — rather than being appended after — makes these fixtures
    /// disagree with the reader instead of quietly agreeing with it.
    fn record_v2(n: u8, halted: bool, pessimistic: i64) -> [u8; super::STRIDE_BYTES_V2] {
        let wide = record(n, halted, pessimistic);
        let mut out = [0_u8; super::STRIDE_BYTES_V2];
        out[..PAYLOAD_BYTES_V2].copy_from_slice(&wide[..PAYLOAD_BYTES_V2]);
        // SEALED OVER 205, which is what makes this a version-2 record and not
        // a truncated version-3 one. The reader must hash the same length back.
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(&out[..PAYLOAD_BYTES_V2]);
        let full = hasher.finalize();
        out[PAYLOAD_BYTES_V2..].copy_from_slice(&full[..super::SEAL_BYTES]);
        out
    }

    /// A whole version-2 ledger file.
    fn file_v2(records: &[[u8; super::STRIDE_BYTES_V2]]) -> Vec<u8> {
        let mut out = header(VERSION_V2).to_vec();
        for rec in records {
            out.extend_from_slice(rec);
        }
        out
    }

    fn over(bytes: Vec<u8>, limit: usize) -> Ledger {
        read_from(
            std::path::PathBuf::from("/fixture/runs.bin"),
            &mut Cursor::new(bytes),
            limit,
        )
    }

    /// A source whose Nth I/O call fails, so every error branch in
    /// [`read_from`] is reachable without an unreadable file on disk.
    ///
    /// This is the whole reason `read_from` is generic. Without it the three
    /// refusals below would first execute in front of an operator.
    struct FailsAt {
        inner: Cursor<Vec<u8>>,
        fail_seek_after: usize,
        seeks: usize,
        fail_read_after: usize,
        reads: usize,
    }

    impl FailsAt {
        fn new(bytes: Vec<u8>) -> Self {
            Self {
                inner: Cursor::new(bytes),
                fail_seek_after: usize::MAX,
                seeks: 0,
                fail_read_after: usize::MAX,
                reads: 0,
            }
        }
        fn seek_fails_after(mut self, n: usize) -> Self {
            self.fail_seek_after = n;
            self
        }
        fn read_fails_after(mut self, n: usize) -> Self {
            self.fail_read_after = n;
            self
        }
    }

    impl Seek for FailsAt {
        fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
            self.seeks += 1;
            if self.seeks > self.fail_seek_after {
                return Err(std::io::Error::other("the seek was refused"));
            }
            self.inner.seek(to)
        }
    }

    impl Read for FailsAt {
        fn read(&mut self, into: &mut [u8]) -> std::io::Result<usize> {
            self.reads += 1;
            if self.reads > self.fail_read_after {
                return Err(std::io::Error::other("the read was refused"));
            }
            self.inner.read(into)
        }
    }

    fn over_failing(src: FailsAt, limit: usize) -> Ledger {
        let mut src = src;
        read_from(
            std::path::PathBuf::from("/fixture/runs.bin"),
            &mut src,
            limit,
        )
    }

    /* ==================== the layout ==================== */

    #[test]
    fn every_field_lands_where_the_writer_put_it() {
        let ledger = over(file(VERSION, &[record(7, false, 34_302)]), 10);
        assert_eq!(ledger.refusal, None, "a good file refuses nothing");
        let run = ledger.runs.first().expect("one run");
        assert_eq!(run.index, 0);
        assert_eq!(run.identity, "07".repeat(32));
        assert_eq!(run.finished_micros, 1_700_000_000_000_007);
        assert_eq!(run.feed, "zerodha");
        assert_eq!(run.underlying, "NIFTY");
        assert_eq!(run.timeframe, "1day");
        assert_eq!((run.from_year, run.from_month), (2019, 12));
        assert_eq!((run.to_year, run.to_month), (2026, 8));
        assert_eq!(run.months_asked, 81);
        assert_eq!(run.months_found, 7);
        assert_eq!(run.bars, 1671);
        assert_eq!(run.min_hits, 300);
        assert_eq!(run.combinations, 458_379);
        assert_eq!(run.depth, 15);
        assert!(!run.halted);
        assert_eq!(run.trades, 302);
        assert_eq!(run.pessimistic, 34_302);
        assert_eq!(run.optimistic, 86_474);
        assert_eq!(run.worst_trade, -500);
        assert_eq!(run.max_drawdown, -1_200);
        assert_eq!(run.winner_mae, 3_400);
        assert_eq!(run.winner_mfe, 9_100);
        assert_eq!(run.all_mae, 5_600);
        assert_eq!(run.exit_rungs, [-1, -1, 1, 0, 0]);
    }

    #[test]
    fn a_halted_record_reads_as_halted() {
        let ledger = over(file(VERSION, &[record(1, true, 10)]), 10);
        assert!(ledger.runs.first().expect("one run").halted);
        assert_eq!(ledger.halted_count(), 1);
    }

    #[test]
    fn a_short_span_is_not_a_whole_span() {
        // `months_found` is seeded from `n`, so 81 is whole and 7 is not.
        assert!(!over(file(VERSION, &[record(7, false, 1)]), 10).runs[0].whole_span());
        assert!(over(file(VERSION, &[record(81, false, 1)]), 10).runs[0].whole_span());
    }

    #[test]
    fn a_text_field_stops_at_its_padding() {
        assert_eq!(text(b"NIFTY\0\0\0\0\0\0\0\0\0\0\0"), "NIFTY");
        // SIXTEEN BYTES WITH NO NUL. The `position` probe returns None and the
        // whole field is the value -- the branch a shorter fixture never takes.
        assert_eq!(text(b"ABCDEFGHIJKLMNOP"), "ABCDEFGHIJKLMNOP");
        // INVALID UTF-8 IS LOSSY, NOT FATAL. Throwing away a completed sweep
        // because a name held a stray byte is a refusal that hides a result.
        assert_eq!(text(&[0xFF, 0xFE, 0]), "\u{fffd}\u{fffd}");
        assert_eq!(text(&[]), "");
    }

    /* ==================== newest first ==================== */

    #[test]
    fn runs_arrive_newest_first() {
        let ledger = over(
            file(
                VERSION,
                &[
                    record(1, false, 10),
                    record(2, false, 20),
                    record(3, false, 30),
                ],
            ),
            10,
        );
        let order: Vec<u64> = ledger.runs.iter().map(|r| r.index).collect();
        assert_eq!(order, vec![2, 1, 0], "the last appended is the first shown");
        assert_eq!(ledger.total, 3);
        assert_eq!(ledger.scanned, 3);
        assert!(!ledger.hit_scan_cap);
    }

    #[test]
    fn a_limit_takes_the_newest_and_says_it_stopped() {
        let ledger = over(
            file(
                VERSION,
                &[
                    record(1, false, 10),
                    record(2, false, 20),
                    record(3, false, 30),
                ],
            ),
            2,
        );
        assert_eq!(ledger.total, 3, "total is the whole file, not the window");
        assert_eq!(ledger.scanned, 2);
        assert!(
            ledger.hit_scan_cap,
            "a window must never pass for the whole"
        );
        assert_eq!(
            ledger.runs.iter().map(|r| r.index).collect::<Vec<_>>(),
            vec![2, 1]
        );
    }

    /* ==================== best complete ==================== */

    #[test]
    fn the_best_complete_run_is_the_highest_worst_case_total() {
        let ledger = over(
            file(
                VERSION,
                &[
                    record(1, false, 100),
                    record(2, false, 900), // best, and complete
                    record(3, false, 300),
                ],
            ),
            10,
        );
        assert_eq!(ledger.best_complete().expect("one survives").index, 1);
    }

    #[test]
    fn a_halted_run_is_never_crowned_however_large_its_total() {
        let ledger = over(
            file(
                VERSION,
                &[
                    record(1, false, 100),
                    record(2, true, 9_000_000), // enormous, and HALTED
                ],
            ),
            10,
        );
        let best = ledger.best_complete().expect("the complete one survives");
        assert_eq!(best.index, 0, "the halted row is excluded, not ranked");
        assert_eq!(best.pessimistic, 100);
        assert_eq!(ledger.halted_count(), 1);
    }

    #[test]
    fn no_complete_run_is_none_rather_than_a_fabricated_winner() {
        let ledger = over(
            file(VERSION, &[record(1, true, 10), record(2, true, 20)]),
            10,
        );
        assert!(ledger.best_complete().is_none());
        assert_eq!(ledger.halted_count(), 2);
        assert!(ledger.to_json().contains(r#""best_complete":null"#));
    }

    #[test]
    fn an_empty_ledger_has_no_best_and_no_refusal() {
        let ledger = over(file(VERSION, &[]), 10);
        assert_eq!(ledger.total, 0);
        assert!(ledger.runs.is_empty());
        assert!(ledger.best_complete().is_none());
        assert_eq!(
            ledger.refusal, None,
            "an empty ledger is a fact, not a failure"
        );
    }

    #[test]
    fn a_tie_breaks_toward_the_earlier_run() {
        // Two identical totals are the same answer; the one on disk longest wins.
        let ledger = over(
            file(VERSION, &[record(1, false, 500), record(2, false, 500)]),
            10,
        );
        assert_eq!(ledger.best_complete().expect("a winner").index, 0);
    }

    /* ==================== every refusal ==================== */

    #[test]
    fn a_missing_file_says_so_and_says_it_is_not_an_error() {
        let root = std::path::PathBuf::from("/nonexistent-brutex-root-for-a-test");
        let ledger = read(&root, 10);
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("no results file"), "{why}");
        assert!(why.contains("This is not an error"), "{why}");
        assert!(ledger.runs.is_empty());
        assert_eq!(ledger.path, path_in(&root), "the path is named");
    }

    #[test]
    fn a_file_shorter_than_its_header_parses_nothing() {
        let ledger = over(vec![0_u8; 4], 10);
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("4 bytes"), "{why}");
        assert!(why.contains("header alone is 16"), "{why}");
    }

    #[test]
    fn a_foreign_file_is_refused_before_it_is_parsed() {
        let mut bytes = vec![0_u8; 16];
        bytes[..8].copy_from_slice(b"NOTBRTEX");
        let ledger = over(bytes, 10);
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("not a brutex results file"), "{why}");
        assert!(why.contains("BRUTEXRS"), "{why}");
        assert!(why.contains("store root"), "{why}");
    }

    #[test]
    fn a_record_this_build_wrote_carries_a_seal_that_checks_out() {
        let ledger = over(file(VERSION, &[record(1, false, 10)]), 10);
        assert!(ledger.runs[0].sealed, "an undamaged record must verify");
        assert_eq!(ledger.unsealed_count(), 0);
    }

    #[test]
    fn one_flipped_bit_is_caught_even_though_every_field_still_parses() {
        let ledger = over(file(VERSION, &[damaged(1, false, 10)]), 10);
        // THE POINT OF THE SEAL, IN ONE ASSERTION. The record read back
        // perfectly: it is present, it is the right length, and every field
        // holds a legal value of its type. Version 1 had no way to know it was
        // not what the writer wrote.
        assert_eq!(ledger.scanned, 1, "the record parses, damage and all");
        assert_eq!(ledger.runs[0].pessimistic, 10, "the fields decode fine");
        assert!(!ledger.runs[0].sealed, "and the seal is what knows better");
        assert_eq!(ledger.unsealed_count(), 1);
    }

    #[test]
    fn a_damaged_record_is_shown_and_marked_rather_than_dropped() {
        let ledger = over(
            file(VERSION, &[record(1, false, 10), damaged(2, false, 20)]),
            10,
        );
        // NOT 1. Dropping the bad row would be the fallback that hides a
        // failure §4 bans, and it would also make `total` disagree with the
        // number of rows on the page for a reason nothing on the page states.
        assert_eq!(ledger.runs.len(), 2, "both rows are served");
        assert_eq!(ledger.total, 2);
        assert_eq!(ledger.unsealed_count(), 1, "and one of them is marked");
    }

    #[test]
    fn a_damaged_record_can_never_be_crowned_however_good_it_looks() {
        // The damaged record has the HIGHER pessimistic, so it wins on every
        // rule except the one that matters. Corruption is as likely to inflate
        // a figure as to deflate it, which makes the damaged row
        // disproportionately likely to top a ranking — the one place it does
        // the most harm, because it is named as the answer.
        let ledger = over(
            file(VERSION, &[record(1, false, 10), damaged(2, false, 9_999)]),
            10,
        );
        let best = ledger.best_complete().expect("the sound record wins");
        assert_eq!(best.pessimistic, 10, "the sealed row, not the larger one");
        assert!(best.sealed);
    }

    #[test]
    fn a_ledger_of_nothing_but_damage_names_no_winner_at_all() {
        // `None` rather than "the least bad of them". There is no complete,
        // trustworthy run, and saying so is the whole contract.
        let ledger = over(
            file(VERSION, &[damaged(1, false, 10), damaged(2, false, 20)]),
            10,
        );
        assert!(ledger.best_complete().is_none());
        assert_eq!(ledger.unsealed_count(), 2);
    }

    #[test]
    fn halted_and_unsealed_are_counted_apart_because_they_mean_apart() {
        // A halted run is honest and incomplete. An unsealed one is bytes that
        // cannot be vouched for. Merging the two counts would tell the operator
        // "3 rows are odd" when the truth is "1 stopped early and 2 may be
        // fiction", and only one of those is worth waking up for.
        let ledger = over(
            file(
                VERSION,
                &[
                    record(1, true, 10),
                    damaged(2, false, 20),
                    damaged(3, false, 30),
                ],
            ),
            10,
        );
        assert_eq!(ledger.halted_count(), 1);
        assert_eq!(ledger.unsealed_count(), 2);
    }

    #[test]
    fn the_seal_covers_the_payload_and_stops_short_of_itself() {
        // A seal that covered itself could not be written: computing it would
        // change the bytes it was computed over. Flipping a byte INSIDE the
        // seal slot must therefore still be caught -- by mismatch, not by
        // recursion -- while the payload is untouched.
        let mut raw = record(1, false, 10);
        assert_eq!(seal_of(&raw), raw[PAYLOAD_BYTES..STRIDE_BYTES]);
        raw[PAYLOAD_BYTES] ^= 0b1000_0000;
        assert_ne!(seal_of(&raw), raw[PAYLOAD_BYTES..STRIDE_BYTES]);
        assert!(!over(file(VERSION, &[raw]), 10).runs[0].sealed);
    }

    #[test]
    fn the_json_carries_the_seal_so_the_page_can_mark_the_row() {
        // A flag the page cannot read is a flag that does not exist.
        let json = over(
            file(VERSION, &[record(1, false, 10), damaged(2, false, 20)]),
            10,
        )
        .to_json();
        assert!(json.contains(r#""unsealed":1"#), "{json}");
        assert!(json.contains(r#""sealed":true"#), "{json}");
        assert!(json.contains(r#""sealed":false"#), "{json}");
    }

    #[test]
    fn an_unknown_version_is_refused_rather_than_guessed_at() {
        let ledger = over(file(9, &[record(1, false, 10)]), 10);
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("version 9"), "{why}");
        assert!(
            why.contains(&format!("reads versions {VERSION_V2} and {VERSION}")),
            "the refusal must name BOTH versions it knows, or an operator \
             cannot tell whether their file is old or simply wrong: {why}"
        );
        assert!(
            why.contains("land mid-record"),
            "the refusal must say WHY guessing is worse: {why}"
        );
        assert!(ledger.runs.is_empty(), "nothing is parsed from it");
    }

    #[test]
    fn a_ragged_tail_is_named_and_the_whole_records_are_still_served() {
        let mut bytes = file(VERSION, &[record(1, false, 10), record(2, false, 20)]);
        bytes.extend_from_slice(&[0_u8; 30]); // an interrupted third append
        let ledger = over(bytes, 10);
        assert!(ledger.partial_tail, "the ragged tail is reported");
        assert_eq!(ledger.total, 2, "only whole records are counted");
        assert_eq!(ledger.runs.len(), 2, "and they are still served");
        assert_eq!(ledger.refusal, None, "a ragged tail is not fatal");
    }

    #[test]
    fn a_length_that_cannot_be_taken_refuses_with_the_path() {
        let ledger = over_failing(FailsAt::new(file(VERSION, &[])).seek_fails_after(0), 10);
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("could not be measured"), "{why}");
        assert!(why.contains("/fixture/runs.bin"), "{why}");
    }

    #[test]
    fn an_unreadable_header_refuses_with_the_path() {
        let ledger = over_failing(
            FailsAt::new(file(VERSION, &[record(1, false, 10)])).read_fails_after(0),
            10,
        );
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("header of"), "{why}");
        assert!(why.contains("could not be read"), "{why}");
    }

    #[test]
    fn a_record_that_cannot_be_read_keeps_the_records_above_it() {
        // Read 1 is the header; read 2 is the newest record; read 3 fails.
        let ledger = over_failing(
            FailsAt::new(file(
                VERSION,
                &[
                    record(1, false, 10),
                    record(2, false, 20),
                    record(3, false, 30),
                ],
            ))
            .read_fails_after(2),
            10,
        );
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("record 1 of"), "{why}");
        assert!(
            why.contains("The 1 newer records above it were read and are shown"),
            "a partial read must say what it kept: {why}"
        );
        assert_eq!(ledger.runs.len(), 1, "partial, never nothing");
        assert_eq!(ledger.scanned, 1);
        assert_eq!(ledger.total, 3, "total is still honest");
    }

    #[test]
    fn a_file_that_cannot_be_opened_at_all_names_the_reason() {
        // A path whose PARENT is a file, not a directory. `File::open` then
        // fails with NotADirectory rather than NotFound, which is the branch
        // every open failure other than "no ledger yet" takes.
        let root = std::env::temp_dir().join(format!(
            "brutex-backtest-blocked-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(&root);
        std::fs::create_dir_all(&root).expect("a temp root");
        // `results` is a FILE, so `results/runs.bin` cannot be opened.
        std::fs::write(root.join("results"), b"not a directory").expect("the blocker");
        let ledger = read(&root, 10);
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("could not be opened"), "{why}");
        assert!(
            !why.contains("This is not an error"),
            "a real open failure must NOT wear the empty-ledger sentence: {why}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_real_file_on_disk_reads_the_same_as_the_cursor() {
        let root = std::env::temp_dir().join(format!(
            "brutex-backtest-real-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("results")).expect("a temp root");
        std::fs::write(path_in(&root), file(VERSION, &[record(4, false, 44)])).expect("the ledger");
        let ledger = read(&root, 10);
        assert_eq!(ledger.refusal, None);
        assert_eq!(ledger.total, 1);
        assert_eq!(ledger.runs.first().expect("one run").pessimistic, 44);
        let _ = std::fs::remove_dir_all(&root);
    }

    /* ==================== the JSON ==================== */

    #[test]
    fn the_json_carries_every_field_as_a_number_not_a_string() {
        let ledger = over(file(VERSION, &[record(7, false, 34_302)]), 10);
        let json = ledger.to_json();
        for fragment in [
            r#""index":0"#,
            r#""finished_micros":1700000000000007"#,
            r#""feed":"zerodha""#,
            r#""underlying":"NIFTY""#,
            r#""timeframe":"1day""#,
            r#""from_year":2019"#,
            r#""from_month":12"#,
            r#""to_year":2026"#,
            r#""to_month":8"#,
            r#""months_asked":81"#,
            r#""months_found":7"#,
            r#""whole_span":false"#,
            r#""bars":1671"#,
            r#""min_hits":300"#,
            r#""combinations":458379"#,
            r#""depth":15"#,
            r#""halted":false"#,
            r#""trades":302"#,
            r#""pessimistic":34302"#,
            r#""optimistic":86474"#,
            r#""worst_trade":-500"#,
            r#""max_drawdown":-1200"#,
            r#""winner_mae":3400"#,
            r#""winner_mfe":9100"#,
            r#""all_mae":5600"#,
            r#""exit_rungs":[-1,-1,1,0,0]"#,
            r#""best_complete":0"#,
            r#""refusal":null"#,
            r#""total":1"#,
            r#""scanned":1"#,
            r#""hit_scan_cap":false"#,
            r#""partial_tail":false"#,
            r#""halted":0"#,
        ] {
            assert!(json.contains(fragment), "missing {fragment} in {json}");
        }
    }

    #[test]
    fn a_refusal_reaches_the_json_as_a_sentence() {
        let json = over(vec![0_u8; 4], 10).to_json();
        assert!(json.contains(r#""refusal":"#));
        assert!(json.contains("header alone is 16"));
        assert!(json.contains(r#""runs":[]"#));
        assert!(json.contains(r#""best_complete":null"#));
    }

    #[test]
    fn several_runs_are_comma_separated_and_not_trailing() {
        let json = over(
            file(VERSION, &[record(1, false, 10), record(2, false, 20)]),
            10,
        )
        .to_json();
        assert!(json.contains("},{"), "two objects, one comma");
        assert!(!json.contains(",]"), "no trailing comma: {json}");
    }

    /* ==================== the route ==================== */

    #[test]
    fn a_limit_is_clamped_at_both_ends_and_never_refused() {
        assert_eq!(limit_asked(""), DEFAULT_LIMIT, "absent takes the default");
        assert_eq!(limit_asked("limit=notanumber"), DEFAULT_LIMIT);
        assert_eq!(limit_asked("limit=25"), 25);
        assert_eq!(limit_asked("limit=0"), 1, "zero would answer nothing");
        assert_eq!(
            limit_asked("limit=99999999"),
            MAX_RUNS,
            "a bookmarked huge limit is clamped, not refused"
        );
    }

    #[test]
    fn an_unresolvable_store_root_answers_503_with_the_shape_the_page_parses() {
        let (status, headers, body) = respond(Err("no store root, and here is why".to_owned()), "");
        assert_eq!(status, axum::http::StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        assert!(body.contains("no store root, and here is why"));
        assert!(
            body.contains(r#""runs":[]"#) && body.contains(r#""best_complete":null"#),
            "a configuration failure must not also be a parse failure: {body}"
        );
    }

    #[test]
    fn a_resolvable_root_answers_200_and_the_ledger() {
        let root = std::env::temp_dir().join(format!(
            "brutex-backtest-route-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("results")).expect("a temp root");
        std::fs::write(path_in(&root), file(VERSION, &[record(2, false, 22)])).expect("the ledger");
        let (status, headers, body) = respond(Ok(root.clone()), "limit=1");
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        assert!(body.contains(r#""pessimistic":22"#), "{body}");
        assert!(body.contains(r#""total":1"#));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn the_handler_answers_over_the_real_environment() {
        // The only line `respond` cannot cover: reading the environment. It
        // answers one of exactly two statuses and nothing else.
        let (status, _, body) = super::backtest_json(
            "/backtest.json?limit=1"
                .parse::<axum::http::Uri>()
                .expect("a uri"),
        )
        .await;
        assert!(
            status == axum::http::StatusCode::OK
                || status == axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
        assert!(body.contains(r#""runs":"#), "{body}");
    }

    /* ==================== the guard rails ==================== */

    #[test]
    fn the_stride_is_the_sum_of_the_fields_the_writer_writes() {
        // The const assertions above already fail the BUILD if these disagree.
        // This states the numbers in a third place so the intent survives a
        // careless edit to any one constant.
        assert_eq!(super::STRIDE_BYTES, 261, "253 of fields and 8 of seal");
        assert_eq!(super::FIELD_SUM, 253);
        assert_eq!(PAYLOAD_BYTES, 253);
        assert_eq!(super::SEAL_BYTES, 8);
        assert_eq!(super::HEADER_BYTES, 16);
        // VERSION 2, WHICH IS FROZEN AND MUST NEVER MOVE. It is history, and
        // §3 rule 8 forbids mutating a released format in place. If a careless
        // edit ever changes one of these, the three records on disk decode into
        // nonsense with nothing else to catch it -- `cli` keeps its version-2
        // constants private, so there is no writer-side pin available here.
        assert_eq!(super::STRIDE_BYTES_V2, 213);
        assert_eq!(PAYLOAD_BYTES_V2, 205, "version 1's whole record");
        // AND THE ONE THAT WOULD HAVE CAUGHT BOTH DRIFTS BEFORE THEY SHIPPED.
        // Everything above is this crate agreeing with itself, which it did
        // throughout both incidents: 205 and 205 stayed equal while the writer
        // moved to 213, then 213 and 213 stayed equal while it moved to 261.
        // Only a line naming `cli`'s constant can fail when the WRITER changes,
        // and that is the whole lesson -- twice.
        assert_eq!(super::STRIDE_BYTES, cli::results::STRIDE_BYTES);
    }

    /* ============ the version this build no longer writes ============ */

    #[test]
    fn a_version_2_ledger_is_read_rather_than_refused() {
        // THE WHOLE POINT OF THE DUAL READ. The only ledger that exists on the
        // operator's disk is a version-2 file, and `cli results` reads it. A
        // page that refused it would have this crate and that one disagreeing
        // about a file they both open, which is the failure this module exists
        // to prevent.
        let ledger = over(
            file_v2(&[record_v2(1, false, 10), record_v2(2, false, 20)]),
            10,
        );
        assert_eq!(ledger.refusal, None, "a known version is not a refusal");
        assert_eq!(ledger.version, VERSION_V2);
        assert_eq!(ledger.total, 2, "addressed at 213, not at 261");
        assert_eq!(ledger.scanned, 2);
        assert!(
            !ledger.partial_tail,
            "639 % 213 == 0 and 639 % 261 does not"
        );
    }

    #[test]
    fn a_version_2_record_keeps_its_seal_because_it_is_checked_at_its_own_length() {
        // THE ORDERING BUG THIS PAIR OF FUNCTIONS EXISTS TO AVOID. Widening
        // first and sealing after would hash 253 bytes of which 48 are zeroes
        // this crate invented, and EVERY healthy version-2 record would be
        // marked damaged on the page.
        let ledger = over(file_v2(&[record_v2(1, false, 10)]), 10);
        assert!(
            ledger.runs[0].sealed,
            "a healthy version-2 record must not read as damaged"
        );
        assert_eq!(ledger.unsealed_count(), 0);
    }

    #[test]
    fn a_damaged_version_2_record_is_still_caught() {
        // The seal must still DO something at version 2 — a check that passes
        // everything is not a check. Flip a payload byte after sealing.
        let mut rec = record_v2(1, false, 10);
        rec[0] ^= 0b1000_0000;
        let ledger = over(file_v2(&[rec]), 10);
        assert!(!ledger.runs[0].sealed, "damage must still be visible");
        assert_eq!(ledger.unsealed_count(), 1);
    }

    #[test]
    fn a_version_2_run_reads_an_empty_mask_and_the_ledger_says_it_is_absent() {
        // SIX ZERO WORDS MEAN TWO DIFFERENT THINGS and the bytes cannot tell
        // them apart. `has_mask` is the only thing separating "this ledger
        // predates the mask" from "this run recorded no combination", and
        // rendering both the same way is the fallback §4 bans.
        let old = over(file_v2(&[record_v2(1, false, 10)]), 10);
        assert_eq!(old.runs[0].mask_words, [0; 6], "version 2 had no mask");
        let json = old.to_json();
        assert!(json.contains(r#""version":2"#), "{json}");
        assert!(json.contains(r#""has_mask":false"#), "{json}");

        let new = over(file(VERSION, &[record(1, false, 10)]), 10);
        assert_eq!(new.runs[0].mask_words, mask_of(1), "version 3 carries it");
        let json = new.to_json();
        assert!(json.contains(r#""version":3"#), "{json}");
        assert!(json.contains(r#""has_mask":true"#), "{json}");
    }

    #[test]
    fn a_version_2_record_decodes_every_other_field_exactly_as_version_3_does() {
        // The append is only safe if NOTHING before it moved. Read the same
        // seed at both versions and require every field but the mask to match.
        let old = over(file_v2(&[record_v2(7, true, 34_302)]), 10);
        let new = over(file(VERSION, &[record(7, true, 34_302)]), 10);
        let (a, b) = (&old.runs[0], &new.runs[0]);
        assert_eq!(a.identity, b.identity);
        assert_eq!(a.finished_micros, b.finished_micros);
        assert_eq!(
            (&a.feed, &a.underlying, &a.timeframe),
            (&b.feed, &b.underlying, &b.timeframe)
        );
        assert_eq!(
            (a.from_year, a.from_month, a.to_year, a.to_month),
            (b.from_year, b.from_month, b.to_year, b.to_month)
        );
        assert_eq!(
            (a.months_asked, a.months_found),
            (b.months_asked, b.months_found)
        );
        assert_eq!(
            (a.bars, a.min_hits, a.combinations, a.depth),
            (b.bars, b.min_hits, b.combinations, b.depth)
        );
        assert_eq!((a.halted, a.trades), (b.halted, b.trades));
        assert_eq!(
            (a.pessimistic, a.optimistic, a.worst_trade),
            (b.pessimistic, b.optimistic, b.worst_trade)
        );
        assert_eq!(
            (a.max_drawdown, a.winner_mae, a.winner_mfe, a.all_mae),
            (b.max_drawdown, b.winner_mae, b.winner_mfe, b.all_mae)
        );
        assert_eq!(a.exit_rungs, b.exit_rungs);
        assert_ne!(a.mask_words, b.mask_words, "and ONLY the mask differs");
    }

    #[test]
    fn a_ragged_version_2_tail_is_measured_against_213() {
        // A tail is ragged relative to the FILE's stride. Measured against 261
        // a whole version-2 file looks ragged and a ragged one can look whole.
        let mut bytes = file_v2(&[record_v2(1, false, 10)]);
        bytes.extend_from_slice(&[0_u8; 9]);
        let ledger = over(bytes, 10);
        assert_eq!(ledger.total, 1, "one whole record before the stray bytes");
        assert!(ledger.partial_tail, "and the tail is named");
    }

    #[test]
    fn a_record_seek_that_fails_keeps_what_was_read_at_either_version() {
        // `read_at` seeks before it reads, at BOTH versions, and a seek can
        // fail on its own — a closed handle, a device error. Each version has
        // its own seek call, so one test per version or one of them is a path
        // nothing has ever taken.
        for bytes in [
            file(VERSION, &[record(1, false, 10), record(2, false, 20)]),
            file_v2(&[record_v2(1, false, 10), record_v2(2, false, 20)]),
        ] {
            // Seek 1 measures the length, seek 2 is the header's, seek 3 is
            // the first record's — which is the one `read_at` owns.
            let ledger = over_failing(FailsAt::new(bytes).seek_fails_after(2), 10);
            let why = ledger.refusal.expect("a sentence naming where it stopped");
            assert!(why.contains("could not be read at byte"), "{why}");
            assert_eq!(
                ledger.total, 2,
                "the count came from the length, not the read"
            );
            assert!(
                ledger.runs.is_empty(),
                "the failure was on the first record"
            );
        }
    }

    #[test]
    fn a_version_2_record_read_that_fails_keeps_the_records_above_it() {
        // The version-2 arm has its OWN `read_exact`, on its own shorter array.
        // Read 1 is the header's, read 2 the newest record's, read 3 the next.
        let bytes = file_v2(&[record_v2(1, false, 10), record_v2(2, false, 20)]);
        let ledger = over_failing(FailsAt::new(bytes).read_fails_after(2), 10);
        let why = ledger.refusal.expect("a sentence");
        assert!(why.contains("could not be read at byte"), "{why}");
        assert_eq!(ledger.scanned, 1, "the newer record was kept");
        assert_eq!(ledger.runs.len(), 1, "PARTIAL, not nothing");
        assert!(ledger.runs[0].sealed, "and what was kept is still whole");
    }

    #[test]
    fn the_mask_survives_json_as_a_string_because_a_number_would_round() {
        // BIT 63 IS THE CASE THAT BREAKS. As a JSON number, 2^63 is parsed into
        // an IEEE-754 double everywhere it lands; the bits that fall off name
        // DIFFERENT conditions, and nothing throws. The fixture sets that bit
        // on purpose — see `mask_of`.
        let json = over(file(VERSION, &[record(3, false, 10)]), 10).to_json();
        assert!(
            json.contains(r#""mask_words":["3","0","12884901888","0","9223372036854775808","7"]"#),
            "every word quoted, and 2^63 exact: {json}"
        );
        assert_eq!(mask_of(3)[4], 1 << 63, "the fixture must exercise bit 63");
    }

    #[test]
    fn the_mask_is_exactly_what_version_3_appended_to_version_2() {
        // The append is what makes `widen_v2` a copy rather than a re-layout,
        // and it is the reason every version-2 offset still lands. Stated here
        // as arithmetic so the intent survives an edit to either constant.
        assert_eq!(PAYLOAD_BYTES - PAYLOAD_BYTES_V2, 48, "six u64 mask words");
        assert_eq!(super::STRIDE_BYTES - super::STRIDE_BYTES_V2, 48);
        assert_eq!(super::stride_of(VERSION), super::STRIDE);
        assert_eq!(super::stride_of(VERSION_V2), super::STRIDE_V2);
    }

    #[test]
    fn the_ledger_lives_where_cli_puts_it() {
        assert_eq!(
            path_in(std::path::Path::new("/store")),
            std::path::PathBuf::from("/store/results/runs.bin")
        );
    }

    #[test]
    fn a_record_survives_a_round_trip_through_its_own_index() {
        // Record 0 read from a two-record file must be record 0, not record 1
        // read at the wrong offset -- the failure a wrong stride produces
        // silently.
        let ledger = over(
            file(VERSION, &[record(1, false, 111), record(2, false, 222)]),
            10,
        );
        let older = ledger.runs.iter().find(|r| r.index == 0).expect("record 0");
        assert_eq!(older.pessimistic, 111);
        assert_eq!(older.identity, "01".repeat(32));
        let newer = ledger.runs.iter().find(|r| r.index == 1).expect("record 1");
        assert_eq!(newer.pessimistic, 222);
        assert_eq!(newer.identity, "02".repeat(32));
    }

    #[test]
    fn the_runs_array_has_no_comma_before_its_first_element() {
        // MUTATION TESTING FOUND THIS. `if n > 0` guards the separator; the
        // mutant `n >= 0` emits a comma before the FIRST run too, producing
        // `"runs":[,{…}]` — a document no parser accepts. The existing test
        // checked for a TRAILING comma and for `},{` between elements, and a
        // leading one is neither.
        let one = over(file(VERSION, &[record(1, false, 10)]), 10).to_json();
        assert!(one.contains(r#""runs":[{"#), "no leading comma: {one}");
        assert!(!one.contains(r#""runs":[,"#), "{one}");

        let many = over(
            file(VERSION, &[record(1, false, 10), record(2, false, 20)]),
            10,
        )
        .to_json();
        assert!(many.contains(r#""runs":[{"#), "{many}");
        assert!(many.contains("},{"), "one separator between two");
        assert!(!many.contains(",]"), "and none at the end");

        // An empty ledger emits an empty array, not one holding a comma.
        assert!(
            over(file(VERSION, &[]), 10)
                .to_json()
                .contains(r#""runs":[]"#)
        );
    }

    #[test]
    fn the_best_complete_run_does_not_depend_on_the_order_it_is_given() {
        // `read` always returns newest-first, so `best_complete`'s tie-break
        // and a bare `>=` agree on every payload this crate produces — which
        // is why mutation testing could flip the comparison and see nothing
        // fail. The reduce is `pub`, so it must not quietly depend on its
        // caller's ordering: the same runs in EITHER order must name the same
        // winner, and on a tie that winner is the LOWER index.
        let newest_first = over(
            file(VERSION, &[record(1, false, 500), record(2, false, 500)]),
            10,
        );
        assert_eq!(
            newest_first.best_complete().expect("a winner").index,
            0,
            "newest-first: the tie goes to the earlier run"
        );

        // The same two runs, handed over ascending.
        let mut ascending = newest_first.clone();
        ascending.runs.reverse();
        assert_eq!(
            ascending.best_complete().expect("a winner").index,
            0,
            "ascending: the SAME winner, or the reduce depends on its input order"
        );

        // And a clear winner is found from either end.
        let mut mixed = over(
            file(
                VERSION,
                &[
                    record(1, false, 100),
                    record(2, false, 900),
                    record(3, false, 300),
                ],
            ),
            10,
        );
        assert_eq!(mixed.best_complete().expect("a winner").index, 1);
        mixed.runs.reverse();
        assert_eq!(mixed.best_complete().expect("a winner").index, 1);
    }
}
