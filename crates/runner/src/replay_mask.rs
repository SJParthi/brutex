//! Fail-closed reconstruction of a condition mask stored in a durable row.
//!
//! A stored row carries six raw words rather than a [`ConditionMask`]. Turning
//! those bytes back into the type with [`ConditionMask::from_words`] alone
//! would also make retired, void, and not-yet-allocated positions reachable.
//! [`from_stored_words`] first proves that every offered bit belongs to
//! [`vocab::table::LIVE`] and refuses the lowest invalid position by category.
//!
//! # Cost
//!
//! This gate performs exactly six mask comparisons and at most six fixed word
//! checks. It allocates nothing and never walks the vocabulary or a data-sized
//! collection, so its time and space are O(1) at the fixed V1 mask width.

use core::fmt;

use vocab::ConditionMask;
use vocab::table::{BitStatus, LIVE, definition};

// This is a V1 durable representation, not an open-ended array walk. A mask
// widening changes stored bytes and must add a new adapter instead of leaving
// later words unchecked here.
const _: () = assert!(vocab::mask::WORDS == 6);

/// Why raw words from a reopened durable row are not a legal live mask.
///
/// The lowest rejected bit is reported when several invalid positions are
/// present. That precedence is stable and independent of how the words were
/// produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReplayMaskRefusalV1 {
    /// A historical position was retired and may never become true again.
    Tombstoned {
        /// The retired bit present in the stored row.
        bit: u32,
        /// The live bit that the vocabulary records as its replacement.
        duplicate_of: u16,
    },
    /// A definitionally constant vocabulary position was set.
    Void {
        /// The void bit present in the stored row.
        bit: u32,
    },
    /// Mask headroom outside the allocated vocabulary was set.
    Unallocated {
        /// The unallocated bit present in the stored row.
        bit: u32,
    },
    /// The literal live mask and the typed vocabulary table disagreed.
    ///
    /// The vocabulary crate proves these two views equal. Keeping this refusal
    /// explicit makes a future drift fail closed instead of relabelling a live
    /// condition as void or unallocated.
    LiveMaskMismatch {
        /// The live table position missing from [`LIVE`].
        bit: u32,
    },
}

impl ReplayMaskRefusalV1 {
    /// The lowest invalid position carried by the stored words.
    #[must_use]
    pub const fn bit(self) -> u32 {
        match self {
            Self::Tombstoned { bit, .. }
            | Self::Void { bit }
            | Self::Unallocated { bit }
            | Self::LiveMaskMismatch { bit } => bit,
        }
    }
}

impl fmt::Display for ReplayMaskRefusalV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Tombstoned { bit, duplicate_of } => write!(
                f,
                "stored mask sets tombstoned bit {bit}; live bit {duplicate_of} is its recorded replacement"
            ),
            Self::Void { bit } => {
                write!(
                    f,
                    "stored mask sets void bit {bit}, which carries no information"
                )
            }
            Self::Unallocated { bit } => write!(
                f,
                "stored mask sets unallocated bit {bit} in vocabulary headroom"
            ),
            Self::LiveMaskMismatch { bit } => write!(
                f,
                "vocabulary invariant failed: live table bit {bit} is absent from the canonical live mask"
            ),
        }
    }
}

impl std::error::Error for ReplayMaskRefusalV1 {}

/// Reconstruct a canonical live mask from one exact six-word durable value.
///
/// Empty masks remain empty. Invalid bits are never cleared or redirected:
/// the lowest such position is returned as a typed refusal, so reopening
/// corrupt or obsolete bytes cannot silently change a strategy's identity.
///
/// # Errors
///
/// [`ReplayMaskRefusalV1`] when any offered bit is tombstoned, void,
/// unallocated, or exposes a disagreement between the table and [`LIVE`].
pub fn from_stored_words(
    words: [u64; vocab::mask::WORDS],
) -> Result<ConditionMask, ReplayMaskRefusalV1> {
    let [word_0, word_1, word_2, word_3, word_4, word_5] = words;
    let [live_0, live_1, live_2, live_3, live_4, live_5] = LIVE.words();
    let rejected = [
        word_0 & !live_0,
        word_1 & !live_1,
        word_2 & !live_2,
        word_3 & !live_3,
        word_4 & !live_4,
        word_5 & !live_5,
    ];
    if let Some(bit) = first_set_bit(rejected) {
        return Err(refusal_for_rejected_bit(bit));
    }
    Ok(ConditionMask::from_words(words))
}

/// Return the exact low-word-first representation used by durable rows.
///
/// This helper keeps callers such as `cli` independent of the mask's private
/// representation and makes a reopened value's byte-for-byte round trip
/// explicit.
#[must_use]
pub const fn stored_words(mask: &ConditionMask) -> [u64; vocab::mask::WORDS] {
    mask.words()
}

/// Lowest set bit across exactly six words, without a vocabulary-sized scan.
fn first_set_bit(words: [u64; vocab::mask::WORDS]) -> Option<u32> {
    let [word_0, word_1, word_2, word_3, word_4, word_5] = words;
    if word_0 != 0 {
        Some(word_0.trailing_zeros())
    } else if word_1 != 0 {
        Some(64 + word_1.trailing_zeros())
    } else if word_2 != 0 {
        Some(128 + word_2.trailing_zeros())
    } else if word_3 != 0 {
        Some(192 + word_3.trailing_zeros())
    } else if word_4 != 0 {
        Some(256 + word_4.trailing_zeros())
    } else if word_5 != 0 {
        Some(320 + word_5.trailing_zeros())
    } else {
        None
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "a set bit in the fixed 384-bit ConditionMask is at most 383 and therefore fits u16"
)]
fn refusal_for_rejected_bit(bit: u32) -> ReplayMaskRefusalV1 {
    let index = bit as u16;
    match definition(index).map(|row| row.status) {
        Some(BitStatus::Retired { duplicate_of }) => {
            ReplayMaskRefusalV1::Tombstoned { bit, duplicate_of }
        }
        Some(BitStatus::Void { .. }) => ReplayMaskRefusalV1::Void { bit },
        None => ReplayMaskRefusalV1::Unallocated { bit },
        Some(BitStatus::Live) => ReplayMaskRefusalV1::LiveMaskMismatch { bit },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ReplayMaskRefusalV1, first_set_bit, from_stored_words, refusal_for_rejected_bit,
        stored_words,
    };
    use vocab::ConditionMask;
    use vocab::table::LIVE;

    fn only(bit: u32) -> [u64; vocab::mask::WORDS] {
        ConditionMask::ZERO.with_bit(bit).words()
    }

    #[test]
    fn zero_and_every_live_position_round_trip_exactly() {
        assert_eq!(
            from_stored_words([0; vocab::mask::WORDS]),
            Ok(ConditionMask::ZERO)
        );
        assert_eq!(stored_words(&ConditionMask::ZERO), [0; vocab::mask::WORDS]);
        assert_eq!(from_stored_words(LIVE.words()), Ok(LIVE));
        assert_eq!(stored_words(&LIVE), LIVE.words());
    }

    #[test]
    fn every_word_boundary_is_classified_by_its_exact_position() {
        for bit in [0, 63, 64, 127, 128, 191, 192, 234, 274, 319, 320, 369] {
            let expected = ConditionMask::ZERO.with_bit(bit);
            assert_eq!(from_stored_words(only(bit)), Ok(expected), "live bit {bit}");
        }
        for bit in [235, 255, 256, 273] {
            assert_eq!(
                from_stored_words(only(bit)),
                Err(ReplayMaskRefusalV1::Void { bit })
            );
        }
        for bit in [370, 383] {
            assert_eq!(
                from_stored_words(only(bit)),
                Err(ReplayMaskRefusalV1::Unallocated { bit })
            );
        }
    }

    #[test]
    fn all_three_tombstones_refuse_with_their_recorded_replacements() {
        for (bit, duplicate_of) in [(6, 62), (19, 17), (25, 18)] {
            let refusal = ReplayMaskRefusalV1::Tombstoned { bit, duplicate_of };
            assert_eq!(from_stored_words(only(bit)), Err(refusal));
            assert_eq!(refusal.bit(), bit);
        }
    }

    #[test]
    fn the_lowest_invalid_bit_has_stable_precedence() {
        let words = ConditionMask::ZERO
            .with_bit(383)
            .with_bit(256)
            .with_bit(25)
            .words();
        assert_eq!(
            from_stored_words(words),
            Err(ReplayMaskRefusalV1::Tombstoned {
                bit: 25,
                duplicate_of: 18,
            })
        );
    }

    #[test]
    fn fixed_word_probe_names_the_first_set_position_and_empty_input() {
        assert_eq!(first_set_bit([0; vocab::mask::WORDS]), None);
        for bit in [0, 64, 128, 192, 256, 320] {
            assert_eq!(first_set_bit(only(bit)), Some(bit));
        }
    }

    #[test]
    fn every_refusal_is_typed_readable_and_names_its_bit() {
        let cases = [
            (
                ReplayMaskRefusalV1::Tombstoned {
                    bit: 6,
                    duplicate_of: 62,
                },
                "stored mask sets tombstoned bit 6; live bit 62 is its recorded replacement",
            ),
            (
                ReplayMaskRefusalV1::Void { bit: 235 },
                "stored mask sets void bit 235, which carries no information",
            ),
            (
                ReplayMaskRefusalV1::Unallocated { bit: 370 },
                "stored mask sets unallocated bit 370 in vocabulary headroom",
            ),
            (
                ReplayMaskRefusalV1::LiveMaskMismatch { bit: 0 },
                "vocabulary invariant failed: live table bit 0 is absent from the canonical live mask",
            ),
        ];
        for (refusal, message) in cases {
            assert_eq!(refusal_for_rejected_bit(refusal.bit()), refusal);
            assert_eq!(refusal.to_string(), message);
        }
        assert_eq!(
            refusal_for_rejected_bit(0),
            ReplayMaskRefusalV1::LiveMaskMismatch { bit: 0 }
        );
    }
}
