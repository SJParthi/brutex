//! The headline law and its comparisons behind one call (design 4.2 `Law`, `walk_law`): `req70-2s` (THE STANDARD), `req70-2s-cap` (the
//! same blocks with the old HARD money cap kept: "with money cap") and the crate's one old-rule engine `req58b-core@cf2` ([`crate::cmp`]).
//! Every law returns the same [`LevelFill`] / [`crate::BlockFill`] shapes.

use crate::cmp::{CmpIn, LAW_REQ58B_CORE, walk_seconds};
use crate::std70::{LevelFill, Order, Recorder, walk};
use crate::tape::{Row, Tape};
use crate::{Dial, FillErr, LAW_STD, LAW_STD_CAP, LevelMask, MAXL, Paisa, Units};

/// A fill law of this crate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Law {
    /// `req70-2s` (THE STANDARD; crate v2).
    Std70,
    /// `req70-2s-cap`: the same blocks with the old HARD money cap kept (a block priced above `cap` fills nothing there, both sides).
    Std70Cap {
        /// The cap price (paisa a unit).
        cap: Paisa,
    },
    /// `req58b-core@cf2`: the per-second REQ 58b core (beside, labelled "old rule (REQ 58b core)"; the red build's reading).
    Req58bCore(CmpIn),
}

impl Law {
    /// The law's name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Std70 => LAW_STD,
            Self::Std70Cap { .. } => LAW_STD_CAP,
            Self::Req58bCore(_) => LAW_REQ58B_CORE,
        }
    }

    /// The cap price, for `req70-2s-cap`.
    #[must_use]
    pub const fn cap(&self) -> Option<Paisa> {
        match self {
            Self::Std70 | Self::Req58bCore(_) => None,
            Self::Std70Cap { cap } => Some(*cap),
        }
    }

    /// The label every result of this law carries (design S21).
    #[must_use]
    pub const fn words(&self) -> &'static str {
        match self {
            Self::Std70 => "req70-2s (THE STANDARD: REQ 70 + 70a + 70b + 70c)",
            Self::Std70Cap { .. } => "req70-2s-cap (REQ 70 blocks with the old money cap: a labelled comparison)",
            Self::Req58bCore(_) => "req58b-core@cf2 (old rule (REQ 58b core): a labelled comparison, not a lane's full old law)",
        }
    }
}

/// One call for every law of the crate.
///
/// # Errors
/// As [`walk`].
#[allow(clippy::too_many_arguments, reason = "walk's arguments")]
pub fn walk_law<R: Row>(law: &Law, tape: &Tape<'_, R>, o: &Order, dial: &Dial, mask: LevelMask, want: &mut [Units; MAXL], money: Option<&[Option<Paisa>; MAXL]>, out: &mut [LevelFill; MAXL], rec: Option<Recorder<'_>>) -> Result<(), FillErr> {
    match law {
        Law::Std70 | Law::Std70Cap { .. } => walk(tape, o, dial, mask, want, money, law.cap(), out, rec),
        Law::Req58bCore(cin) => walk_seconds(tape, o, dial, mask, want, money, None, out, rec, cin),
    }
}
