//! The condition vocabulary: the bit table, the 256-bit mask, and nothing else.
//!
//! This crate depends on nothing. `CLAUDE.md` §5 permits it one arrow, to
//! `core`, and it does not take it -- a position is a `u16` and a name is a
//! `&'static str`, so there is no price, no symbol and no calendar in the
//! surface. A crate with no arrow cannot be in a cycle.
//!
//! # What is here
//!
//! | Module | Owns |
//! |---|---|
//! | [`mask`] | [`ConditionMask`], the four-word condition mask and the hit test |
//! | [`table`] | the 274 positions, their names, and the three tombstones |
//! | [`tolerance`] | the `near_*` band half-width, which is UNPINNED |
//! | [`error`] | every refusal the two above can produce |
//!
//! # The three rules this crate exists to keep
//!
//! 1. **The index is the identity.** Positions are never renumbered, reused or
//!    reordered. A retired condition keeps its position forever as a tombstone
//!    that always evaluates false -- [`table::BitStatus`] models that, so a
//!    later append cannot fall into the hole a retirement did not leave.
//! 2. **The hit test does the same work for every input.** `(bar & candidate)
//!    == candidate` over four words, with no loop and no early return. See
//!    [`ConditionMask::hits`].
//! 3. **No invented numbers.** Seventy-five live positions are `near_*`
//!    conditions and no document defined their band. There are **two** bands,
//!    because they are fractions of different quantities:
//!    [`tolerance::TOL_FIB_MILLI`] = 10 thousandths of the session range
//!    (D-0076, the `near_*` band, measured) and [`tolerance::TOL_PIVOT_MILLI`] = 500 thousandths
//!    of the CPR width (D-0079, read off the design source). Returning either to
//!    the sentinel makes those positions unreachable loudly, with a reason,
//!    rather than quietly deciding false.
//!
//! # Integers only
//!
//! There is no `f32` and no `f64` anywhere in this crate, in shipping code, in
//! a test or in a doc example. `CLAUDE.md` §7, and `tests/no_float.rs` reads
//! this crate's own source to prove it rather than trusting the sentence.

#![forbid(unsafe_code)]

pub mod error;
pub mod mask;
pub mod table;
pub mod tolerance;

pub use error::VocabError;
pub use mask::ConditionMask;
pub use table::{BitDef, BitStatus, Kind};
pub use tolerance::Tolerance;

/// The version of the bit table, and an input to run identity.
///
/// # Bumping this re-keys every historical run
///
/// `CLAUDE.md` §3 rule 3 identifies a run by
/// `blake3(mask ‖ direction ‖ instrument ‖ timeframe ‖ params ‖ data_digest ‖
/// vocab_version ‖ commit)`. This constant is the `vocab_version` term, so a
/// bump changes the identity of **every** run ever recorded: the same sweep
/// over the same bars produces a different key, nothing that was stored can be
/// found by the identity a rerun computes, and the two sets of results cannot
/// be compared row by row. That is correct -- a different vocabulary is a
/// different experiment -- and it is expensive, which is why it is written
/// here rather than discovered.
///
/// | Version | Table |
/// |---:|---|
/// | 1 | the shipped 74 conditions in a `u128`, `docs/03-vocabulary.md` |
/// | 3 | 274 positions in a [`ConditionMask`], six words wide, three of them tombstones |
///
/// Appending a condition at the next free position does **not** bump it: an
/// append leaves every existing mask meaning exactly what it meant, which is
/// the whole point of §3.8. Retiring, renaming or renumbering does — and so does
/// **widening the mask**, because the width is part of the `mask` term of the
/// run-identity hash (`CLAUDE.md` §3 rule 3), so a stored 128-bit mask and a
/// 384-bit one are different bytes for the same set.
///
/// **2 → 3 by D-0080.** Three things happened at once and each alone would have
/// required it: the mask went `[u64; 4]` to `[u64; 6]`; three positions were
/// retired; thirty-nine were voided. Leaving it at 2 would have let two
/// genuinely different vocabularies stamp results identically.
pub const VOCAB_VERSION: u32 = 3;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_the_widened_table() {
        // Still 3, and that is the rule rather than an oversight. 274 and 275 were
        // APPENDED at NEXT_FREE, and this constant's own documentation says an append
        // does not bump it: every mask recorded before today still means exactly what
        // it meant, so two vocabularies differing only by an append are not
        // "genuinely different" in the sense the version exists to separate.
        // Retiring, renaming, renumbering or widening the mask would bump it.
        assert_eq!(VOCAB_VERSION, 3);
        assert_eq!(table::COUNT, 280);
        assert_eq!(ConditionMask::BITS, 384);
    }

    #[test]
    fn the_re_exports_are_the_types_the_modules_own() {
        let plain: Kind = Kind::Plain;
        assert_eq!(plain, table::Kind::Plain);
        assert_eq!(
            BitStatus::Retired { duplicate_of: 62 },
            table::BitStatus::Retired { duplicate_of: 62 }
        );
        // The annotation is the assertion: `Option<&BitDef>` names the
        // re-export, `table::definition` returns the module's own type, and the
        // two have to be one type or this line does not compile.
        //
        // Read through `Option::map` rather than through `let Some(d) = .. else
        // { unreachable!("position 0 exists") }`. That `else` arm was a panic
        // inside this crate on a path a correct table can never take, so
        // `cargo llvm-cov` counted a region no test run could ever close. The
        // comparison below makes the same claim as an assertion that can fail:
        // an absent position 0 renders as `None` and fails here.
        let zero: Option<&BitDef> = table::definition(0);
        assert_eq!(
            zero.map(|d| d.name),
            Some("close_above_ema20"),
            "position 0 is the first row of the table and it is what the \
             re-exported `BitDef` has to describe"
        );
        assert_eq!(
            VocabError::NoSuchBit { index: 1 },
            error::VocabError::NoSuchBit { index: 1 }
        );
        assert_eq!(
            Tolerance::from_milli(i64::MIN),
            Err(VocabError::ToleranceUnpinned)
        );
    }
}
