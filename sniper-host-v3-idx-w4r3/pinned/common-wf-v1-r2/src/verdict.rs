//! The verdict of one window row on one pick (P10, P11, 6.4): four mask operations decide a split verdict; a pick carries a
//! PASSED / NOT PASSED word with years ONLY for its OWN split window; every other row gives figures or a "not run" word. The kind
//! of pick (split, hindsight, way A, FIXED, a CHOSEN rule) is set at offer (Q2), so every reader of a `Verdict` sees it.

use crate::acc::Facts;
use crate::rules::{HoleSource, RunRules};
use crate::views::Code;
use crate::windows::{WinKind, Windows};
use crate::{Mk, PartMask, RED, SetId, WfErr, WinId};

/// What kind of pick (Q2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    /// A pick of a split window's view.
    Split,
    /// A HS view's pick.
    Hindsight,
    /// A way-A view's pick.
    WayA,
    /// A FIXED subject (never offered, P13).
    Fixed {
        /// Its FIXED grid.
        grid: u8,
    },
    /// The view row, its file version, the window row or a partition value is CHOSEN (X4, Z1, A2): W27 on every split row.
    ChosenRule {
        /// The run that showed the months.
        run: u32,
    },
}
/// The pick's own window, usable from any set (R5), and its code (W23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickWin {
    /// The set of the pick's own window.
    pub set: SetId,
    /// The pick's own window.
    pub win: WinId,
    /// Its learning months.
    pub learn: (Mk, Mk),
    /// Its kind.
    pub kind: PickKind,
    /// Its code.
    pub code: Code,
}
/// How a row reads a pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// The pick's own split window.
    Own,
    /// Another split row: figures only (W05).
    CrossFigures,
    /// Another split row whose test holds the pick's learning months: "not run" (W04).
    CrossNotRun,
    /// A hindsight reading.
    Hindsight,
    /// A way-A reading (figures only).
    WayA,
    /// The PAPER reading (figures only).
    Paper,
    /// A comparison partition (figures only, W11).
    Comparison,
    /// A non-real world (W23).
    World,
    /// A FIXED subject (W26).
    Fixed,
    /// An L replay of the pick (REQ 74).
    Replay,
}
/// Why a row's verdict is "not run".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotRun {
    /// Picked on all years (W06).
    Hindsight,
    /// A pick of another protocol whose learning months this test holds (W04): the overlap.
    OtherProtocol {
        /// The overlap months.
        overlap: (Mk, Mk),
    },
    /// No pick (W07).
    NoPick,
    /// No learning month in this run (W08).
    NoLearnMonth,
    /// No test month in this run (W08).
    NoTestMonth,
    /// A way-A pick (W09).
    WayA,
    /// A row over 32 pieces (W08).
    SetTooBig,
    /// A FIXED subject (W26).
    Fixed {
        /// Its FIXED grid.
        grid: u8,
    },
    /// A CHOSEN rule (W27).
    ChosenRule {
        /// The run that showed the months.
        run: u32,
    },
}
/// A verdict (6.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every test unit positive as booked and in the second reading.
    Passed {
        /// `hole_source = none`: "booked reading only".
        booked_only: bool,
    },
    /// Units not positive as booked.
    NotPassedYears {
        /// The failing units.
        bad: PartMask,
        /// Of them, with no walked session.
        missing: PartMask,
        /// Of them, outside the data span.
        nodata: PartMask,
        /// The failing L replay (REQ 74), if any.
        at_level: Option<u8>,
    },
    /// Units positive as booked but not in the second reading (W03).
    NotPassedHole {
        /// The units.
        bad2: PartMask,
    },
    /// Not run.
    NotRun(NotRun),
    /// Figures only.
    Figures,
}

/// The verdict of window row `v` on `pick`, from `f` (O(1): four mask operations decide a split verdict).
///
/// # Errors
/// `SetMismatch` (facts or window of another set), `NoPartitionRow`.
pub fn verdict(w: &Windows, v: WinId, pick: PickWin, f: &Facts, r: &RunRules) -> Result<(Reading, Verdict), WfErr> {
    if f.set != w.set_id() {
        return Err(WfErr::SetMismatch);
    }
    let part = r.partition(f.partition.0)?;
    if r.is_comparison(part) {
        return Ok((Reading::Comparison, Verdict::Figures));
    }
    if part.world() != "real" {
        return Ok((Reading::World, Verdict::Figures));
    }
    let s = w.spec(v).ok_or(WfErr::SetMismatch)?;
    match s.kind {
        WinKind::WayA => return Ok((Reading::WayA, Verdict::Figures)),
        WinKind::Paper => return Ok((Reading::Paper, Verdict::Figures)),
        WinKind::Hindsight => return Ok((Reading::Hindsight, Verdict::Figures)),
        WinKind::Split => {}
    }
    match pick.kind {
        PickKind::Fixed { grid } => return Ok((Reading::Fixed, Verdict::NotRun(NotRun::Fixed { grid }))),
        PickKind::Hindsight => return Ok((Reading::Hindsight, Verdict::NotRun(NotRun::Hindsight))),
        PickKind::WayA => return Ok((Reading::WayA, Verdict::NotRun(NotRun::WayA))),
        PickKind::ChosenRule { run } => return Ok((Reading::Own, Verdict::NotRun(NotRun::ChosenRule { run }))),
        PickKind::Split => {}
    }
    let own = v == pick.win;
    if !own && !RED && !cfg!(feature = "m11") {
        let (tf, tt) = s.test.ok_or(WfErr::SetMismatch)?;
        let (a, b) = (pick.learn.0.max(tf), pick.learn.1.min(tt));
        if a <= b {
            return Ok((Reading::CrossNotRun, Verdict::NotRun(NotRun::OtherProtocol { overlap: (a, b) })));
        }
        return Ok((Reading::CrossFigures, Verdict::Figures));
    }
    let reading = if own { Reading::Own } else { Reading::CrossFigures };
    let test = s.test_units;
    let mut bad = test & !f.booked;
    if cfg!(feature = "m1") {
        bad &= f.touched;
    }
    if RED || cfg!(feature = "m3") {
        bad &= !f.missing;
    }
    let bad2 = if RED { 0 } else { test & f.booked & !f.second };
    if bad == 0 && bad2 == 0 {
        return Ok((reading, Verdict::Passed { booked_only: r.hole_source == HoleSource::None }));
    }
    if bad != 0 {
        return Ok((reading, Verdict::NotPassedYears { bad: PartMask(bad), missing: PartMask(bad & f.missing), nodata: PartMask(bad & f.nodata), at_level: None }));
    }
    Ok((reading, Verdict::NotPassedHole { bad2: PartMask(bad2) }))
}

/// REQ 74's every-L rule (Q3): the pick's own split window is PASSED only if its own reading AND every replay's are; else the first
/// failing replay's verdict, with `at_level` = that replay's level index (4 mask operations per replay, a count fixed by RULES).
///
/// # Errors
/// As [`verdict`].
pub fn verdict_with_replays(w: &Windows, v: WinId, pick: PickWin, own: &Facts, replays: &[(u8, Facts)], r: &RunRules) -> Result<(Reading, Verdict), WfErr> {
    let base = verdict(w, v, pick, own, r)?;
    if !matches!(base.1, Verdict::Passed { .. }) {
        return Ok(base);
    }
    for (lvl, f) in replays {
        let (rd, vd) = verdict(w, v, pick, f, r)?;
        match vd {
            Verdict::Passed { .. } => {}
            Verdict::NotPassedYears { bad, missing, nodata, .. } => return Ok((rd, Verdict::NotPassedYears { bad, missing, nodata, at_level: Some(*lvl) })),
            other => return Ok((rd, other)),
        }
    }
    Ok(base)
}
