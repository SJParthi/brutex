//! F02 `membership::probe` and F03 `membership::may_trade`: dated F&O membership, or
//! today's list when the dated folder fails its probe (PREVIEW-SPEC section 3.3).

use std::collections::{BTreeMap, BTreeSet};

use crate::time::Day;

/// One answer of the dated folder: `Ok(true)`, `Ok(false)` or a refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Answer {
    /// `Ok(true)`: a member that day.
    Yes,
    /// `Ok(false)`: not a member that day.
    No,
    /// A refusal: membership unknown.
    Unknown,
}

/// Which list decides membership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The dated folder answers per day.
    Dated,
    /// Every listed name is a member on every day ("survivorship: today's list").
    TodaysList,
}

/// The probe of section 3.3: the folder opens, and its three answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe {
    /// `Membership::open` succeeded.
    pub opened: bool,
    /// `is_member("RELIANCE", 2018-09-03)`, expected `Ok(true)`.
    pub reliance_first: Answer,
    /// `is_member("RELIANCE", 2026-09-30)`, expected `Ok(true)`.
    pub reliance_last: Answer,
    /// `is_member("SYRMA", 2026-09-30)`, expected anything but `Ok(true)`.
    pub syrma_last: Answer,
}

/// The label every output carries in either mode.
pub const SURVIVORSHIP: &str = "survivorship: only today's 218 names can trade; a share that left F&O before 30 Oct 2026 has no stored series";

/// The probe's verdict: the mode and every failing probe, in order.
#[must_use]
pub fn probe(p: &Probe) -> (Mode, Vec<&'static str>) {
    let mut reasons = Vec::new();
    if !p.opened {
        reasons.push("the folder refused to open");
    }
    if p.reliance_first != Answer::Yes {
        reasons.push("RELIANCE 2018-09-03 is not Ok(true)");
    }
    if p.reliance_last != Answer::Yes {
        reasons.push("RELIANCE 2026-09-30 is not Ok(true)");
    }
    if p.syrma_last == Answer::Yes {
        reasons.push("SYRMA 2026-09-30 is Ok(true)");
    }
    let mode = if reasons.is_empty() {
        Mode::Dated
    } else {
        Mode::TodaysList
    };
    (mode, reasons)
}

/// The dated answers of one run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership {
    /// The probe the run starts with.
    pub probe: Probe,
    /// Every name the universe lists.
    pub listed: BTreeSet<String>,
    /// Answers that differ from `Yes`, by name and day.
    pub answers: BTreeMap<(String, Day), Answer>,
    /// The last day the folder holds: a later day asks this day (October 2026 asks 2026-09-30).
    pub carry_from: Option<Day>,
}

/// Why a membership question was refused before the folder was asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The day lies outside the window.
    OutsideWindow,
}

impl Membership {
    /// Every listed name is a member on every day, and the probe passes.
    #[must_use]
    pub fn all_members(names: &[&str]) -> Self {
        Self {
            probe: Probe {
                opened: true,
                reliance_first: Answer::Yes,
                reliance_last: Answer::Yes,
                syrma_last: Answer::No,
            },
            listed: names.iter().map(|n| (*n).to_owned()).collect(),
            answers: BTreeMap::new(),
            carry_from: None,
        }
    }

    /// The day the folder is asked for `day` (the carry rule).
    #[must_use]
    pub fn asked_day(&self, day: Day) -> Day {
        match self.carry_from {
            Some(last) if day > last => last,
            _ => day,
        }
    }

    /// F03: may `name` trade on `day`, for a window `first..=last`?
    ///
    /// # Errors
    /// [`Refusal::OutsideWindow`] for a day outside the window: the folder is never asked.
    pub fn may_trade(
        &self,
        name: &str,
        day: Day,
        first: Day,
        last: Day,
    ) -> Result<Answer, Refusal> {
        if day < first || day > last {
            return Err(Refusal::OutsideWindow);
        }
        if !self.listed.contains(name) {
            return Ok(Answer::No);
        }
        if probe(&self.probe).0 == Mode::TodaysList {
            return Ok(Answer::Yes);
        }
        let asked = self.asked_day(day);
        Ok(self
            .answers
            .get(&(name.to_owned(), asked))
            .copied()
            .unwrap_or(Answer::Yes))
    }
}

#[cfg(test)]
#[path = "membership_tests.rs"]
mod tests;
