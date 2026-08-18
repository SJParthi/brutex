//! Every refusal this crate can produce.
//!
//! There is no variant that means "carried on anyway". `CLAUDE.md` §4 bans a
//! fallback that hides a failure, and the two failures this crate has to be
//! able to state -- *that position is a tombstone* and *the tolerance has
//! never been measured* -- are exactly the two a caller would otherwise be
//! tempted to paper over with a `false`.

use crate::tolerance::Base;
use core::fmt;

/// How a band base reads inside a refusal.
///
/// A free function over `Option<Base>` and not a `Display` on [`Base`] itself,
/// because the thing that has to be readable is the **absent** case as much as
/// the two present ones: an operator handed "the tolerance measures against
/// None" learns nothing, and that arm is the one a caller reaches by building a
/// width with no base at all. All three arms are read by
/// `every_refusal_says_what_happened`.
const fn base_name(base: Option<Base>) -> &'static str {
    match base {
        Some(Base::SessionRange) => "the session range",
        Some(Base::CprWidth) => "the CPR width",
        None => "no stated quantity",
    }
}

/// A refusal from the bit table or from the tolerance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VocabError {
    /// The index is not a position in the table. Positions run `0..COUNT`;
    /// the mask is wider than the table on purpose, so a `u16` that a
    /// [`crate::mask::ConditionMask`] would accept can still be no condition.
    NoSuchBit {
        /// The index that was asked for.
        index: u16,
    },
    /// The position is a tombstone. It keeps its index forever and always
    /// evaluates false, so it can be *read* and never *set*.
    /// The position is definitionally constant and carries no information, so
    /// it can never be set. Distinct from [`VocabError::Retired`]: there is no
    /// other position to set instead.
    Void {
        /// The position asked for.
        index: u16,
        /// Why its predicate is a constant.
        reason: &'static str,
    },
    /// The position was retired: it duplicated another and is no longer set.
    ///
    /// Append-only history means a retired bit is never renumbered or reused —
    /// it stays in the table forever, refusing, and naming the live position to
    /// use instead. Distinct from [`VocabError::Void`], where there is no other
    /// position to point at.
    Retired {
        /// The retired position.
        index: u16,
        /// The live position it duplicated, which is what to set instead.
        duplicate_of: u16,
    },
    /// A `near_*` position was set without going through the tolerance.
    NeedsTolerance {
        /// The position that requires one.
        index: u16,
    },
    /// A position that is not a `near_*` was offered a tolerance and a level.
    NotNear {
        /// The position that takes no tolerance.
        index: u16,
    },
    /// A `near_*` position was offered a band measured against the **other**
    /// family's quantity.
    ///
    /// The two are not interchangeable and not comparable:
    /// [`crate::tolerance::TOL_FIB_MILLI`] is thousandths of the session range,
    /// [`crate::tolerance::TOL_PIVOT_MILLI`] is thousandths of the CPR width,
    /// and at today's pins one is fifty times the other. Before this variant
    /// existed the mismatch returned `Ok` and set the bit, and **nothing
    /// downstream could find it** -- a [`crate::ConditionMask`] records which
    /// positions held and not what width decided them, so a result set built on
    /// the wrong band is byte for byte a correct one. `CLAUDE.md` §4: degrade
    /// loudly and name the reason, or refuse.
    WrongBand {
        /// The position that was asked for.
        index: u16,
        /// The base its row declares in [`crate::table::TABLE`].
        expected: Base,
        /// The base the offered tolerance was measured on, or `None` when it
        /// was built by [`crate::tolerance::Tolerance::from_milli`] and named
        /// no base at all.
        got: Option<Base>,
    },
    /// The tolerance has never been measured. See [`crate::tolerance`].
    ToleranceUnpinned,
    /// A tolerance was pinned to a negative width, which is not a band.
    ToleranceNegative {
        /// The value offered.
        milli: i64,
    },
}

impl fmt::Display for VocabError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NoSuchBit { index } => {
                write!(f, "bit {index} is not a position in the vocabulary")
            }
            Self::Void { index, reason } => write!(
                f,
                "position {index} is void and carries no information: {reason}"
            ),
            Self::Retired {
                index,
                duplicate_of,
            } => write!(
                f,
                "bit {index} is retired and always false; it duplicated bit \
                 {duplicate_of}, which is the one to set"
            ),
            Self::NeedsTolerance { index } => write!(
                f,
                "bit {index} is a near_* condition and can only be set through \
                 a measured tolerance"
            ),
            Self::NotNear { index } => write!(
                f,
                "bit {index} is not a near_* condition and takes no tolerance"
            ),
            Self::WrongBand {
                index,
                expected,
                got,
            } => write!(
                f,
                "bit {index} measures its band against {}, and the tolerance \
                 offered measures against {}; the two are fractions of \
                 different quantities and neither width is the other's",
                base_name(Some(expected)),
                base_name(got)
            ),
            Self::ToleranceUnpinned => write!(
                f,
                "the near_* tolerance is UNPINNED: no measurement has fixed it, \
                 and this crate will not invent one"
            ),
            Self::ToleranceNegative { milli } => write!(
                f,
                "a tolerance of {milli} thousandths is negative, and a band \
                 with a negative half-width is not a band"
            ),
        }
    }
}

impl std::error::Error for VocabError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every arm, because an unreachable arm in a `Display` is a message
    /// nobody has ever read and it is usually wrong when they finally do.
    ///
    /// [`VocabError::Void`] **was** that arm: it was the one variant this list
    /// left out, and it is the only arm that has to interpolate two bindings.
    /// Its expectation below names both of them in order, which is what catches
    /// an arm that prints `index` twice, or drops `reason` and leaves an
    /// operator reading "carries no information" with no statement of why.
    #[test]
    fn every_refusal_says_what_happened() {
        let cases = [
            (
                VocabError::NoSuchBit { index: 999 },
                "999 is not a position",
            ),
            (
                VocabError::Void {
                    index: 235,
                    reason: "the predicate is a constant",
                },
                "position 235 is void and carries no information: \
                 the predicate is a constant",
            ),
            (
                VocabError::Retired {
                    index: 6,
                    duplicate_of: 62,
                },
                "bit 62, which is the one to set",
            ),
            (
                VocabError::NeedsTolerance { index: 7 },
                "measured tolerance",
            ),
            (VocabError::NotNear { index: 0 }, "takes no tolerance"),
            // Both `base_name` arms that name a real quantity, in one message
            // and in the right roles. A `Display` that printed `expected`
            // twice, or that swapped the two, renders a sentence that is still
            // grammatical and still names both bands -- so the expectation
            // pins the ORDER, which is the only thing that tells an operator
            // which side was wrong.
            (
                VocabError::WrongBand {
                    index: 7,
                    expected: Base::CprWidth,
                    got: Some(Base::SessionRange),
                },
                "bit 7 measures its band against the CPR width, and the \
                 tolerance offered measures against the session range",
            ),
            // The absent base. This is what `Tolerance::from_milli` produces,
            // and "measures against no stated quantity" is the whole of what
            // the caller did wrong.
            (
                VocabError::WrongBand {
                    index: 22,
                    expected: Base::SessionRange,
                    got: None,
                },
                "bit 22 measures its band against the session range, and the \
                 tolerance offered measures against no stated quantity",
            ),
            (VocabError::ToleranceUnpinned, "UNPINNED"),
            (
                VocabError::ToleranceNegative { milli: -1 },
                "-1 thousandths is negative",
            ),
        ];
        for (err, want) in cases {
            let text = err.to_string();
            assert!(text.contains(want), "{err:?} rendered as {text:?}");
        }
    }

    #[test]
    fn a_refusal_is_an_error_and_compares_by_value() {
        let e = VocabError::NoSuchBit { index: 1 };
        let as_err: &dyn std::error::Error = &e;
        assert!(as_err.source().is_none());
        assert_eq!(e, VocabError::NoSuchBit { index: 1 });
        assert_ne!(e, VocabError::NoSuchBit { index: 2 });
        assert!(format!("{e:?}").contains("NoSuchBit"));
    }
}
