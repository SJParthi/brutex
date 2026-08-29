//! Why every absent bar is absent — classified, not guessed.
//!
//! # The question this answers
//!
//! On 2026-08-22 an operator asked why bars were missing from a completed pull.
//! Answering took an afternoon of decoding fixed-stride records by hand, and the
//! answer was that **almost nothing was missing**: of 1,204 absent minutes in a
//! 623,546-bar series, 1,176 belonged to sessions that were never 375 minutes
//! long — an exchange outage, two disaster-recovery Saturdays and a Muhurat hour
//! — and 28 were minutes the vendor did not send.
//!
//! Every one of those numbers was recoverable from the store. None of them was
//! **reported** by it. That is the gap this module closes: the store knew, and
//! nothing asked it.
//!
//! # The taxonomy is exhaustive by construction
//!
//! An absent minute is in exactly one of four states, and [`Reason`] has exactly
//! four variants because there is no fifth. A minute is either on a day the
//! exchange did not trade, or on a traded day but outside its windows, or inside
//! a window and simply not there, or on a day this build has never measured.
//! `match` is therefore total and a new case cannot be added without the
//! compiler naming every site that must handle it.
//!
//! **Only the third is a loss.** Conflating it with the other three is what made
//! six complete series report `SHORT`, and separating them is the whole point.
//!
//! # Cost
//!
//! **A two-cursor merge, O(1) per minute and per bar, with no lookup structure
//! at all.** Stored bars arrive in timestamp order — the store's own append
//! rule guarantees it — so the expected minutes and the stored bars are walked
//! together and each side advances at most once per step. There is no set, no
//! map and no allocation per minute; the only allocation is the answer itself,
//! and that is bounded by [`MAX_GAPS`].
//!
//! A `HashSet` of stored minutes would have been the obvious shape and is
//! strictly worse: it costs a hash per bar to answer a question the ordering
//! already answers for free.
//!
//! **UNVERIFIED as a measurement.** The bound above is argued from the SHAPE of
//! the walk — two cursors, each advancing at most once per step — and no bench
//! in this workspace times it. `crates/pull/benches/ratio.rs` covers the
//! manifest's census and entry lookup and nothing here. The argument is the
//! stronger kind, because a merge that cannot revisit a position has no input
//! that makes it quadratic, but `CLAUDE.md` §3 rule 6 does not let a structural
//! argument be reported as a measured one. Closing it is a ratio row over
//! `classify` at 1x/10x/100x the minute count, beside the rows that already
//! exist for the manifest.

use crate::calendar::{self, Calendar, DayKind};

/// The most gaps one classification will report.
///
/// **A bound, because an unbounded answer is the failure it is describing.** A
/// store that lost a whole year would otherwise produce a `Vec` of ~94,000
/// entries on a path an operator polls, and a report that cannot be rendered is
/// not a report. Past this the walk stops and says so — see [`Ledger::truncated`]
/// — which is `CLAUDE.md` §4's "degrade loudly and name the reason" rather than
/// a silent `take(N)`.
pub const MAX_GAPS: usize = 4_096;

/// Why a minute is not in the store.
///
/// Four variants and no fifth: see the module header for why the taxonomy is
/// exhaustive by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The exchange did not trade that day at all — a weekend or a holiday.
    ///
    /// Not a loss. This is the reason that made every complete series read
    /// `SHORT` when it was counted as one.
    Closed,
    /// The day traded, and this minute was outside its windows.
    ///
    /// Muhurat is one hour in the afternoon; a disaster-recovery Saturday has a
    /// two-hour hole in the middle by design. Neither is a loss, and both look
    /// exactly like one to an arithmetic count.
    OutsideWindow,
    /// Inside a window the exchange traded, and the bar is not there.
    ///
    /// **This is the only variant that is a real loss**, and the only one worth
    /// an operator's attention. It cannot be repaired from the same vendor: a
    /// re-run returns the same nothing, because the vendor does not hold it.
    /// The route to zero is another feed's copy of the same minute.
    VendorHole,
    /// The calendar does not cover this day, so no claim is made.
    ///
    /// Deliberately not folded into [`Self::Closed`]: "the exchange was shut"
    /// and "nobody has looked" are different facts, and reporting the second as
    /// the first is the invention `CLAUDE.md` §3 rule 1 bans.
    Unmeasured,
}

impl Reason {
    /// Whether this absence is a loss an operator should act on.
    #[must_use]
    pub const fn is_loss(self) -> bool {
        matches!(self, Self::VendorHole)
    }

    /// A short, stable word for a report or a log line.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::OutsideWindow => "outside-window",
            Self::VendorHole => "vendor-hole",
            Self::Unmeasured => "unmeasured",
        }
    }
}

/// One run of consecutive absent minutes sharing a reason.
///
/// Runs rather than minutes, because 1,176 of the 1,204 absences measured were
/// four contiguous events. A per-minute list of the same facts is 43× longer and
/// says less.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// Days since the epoch.
    pub day: i64,
    /// Minute-of-day the run starts.
    pub from: u16,
    /// Minute-of-day the run ends, inclusive.
    pub to: u16,
    /// Why.
    pub reason: Reason,
}

impl Gap {
    /// How many minutes this run covers.
    #[must_use]
    pub const fn minutes(self) -> u32 {
        (self.to as u32)
            .saturating_sub(self.from as u32)
            .saturating_add(1)
    }
}

/// What one classification found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ledger {
    /// The runs, in day then minute order.
    pub gaps: Vec<Gap>,
    /// Bars the calendar says the range owed.
    pub expected: u32,
    /// Bars the store actually held.
    pub held: u32,
    /// Whether [`MAX_GAPS`] stopped the walk before it finished.
    ///
    /// **Never silent.** A truncated ledger that read like a complete one would
    /// tell an operator their store is healthier than it is.
    pub truncated: bool,
}

impl Ledger {
    /// Minutes absent for a reason that is a real loss.
    #[must_use]
    pub fn lost_minutes(&self) -> u32 {
        self.gaps
            .iter()
            .filter(|g| g.reason.is_loss())
            .map(|g| g.minutes())
            .fold(0_u32, u32::saturating_add)
    }

    /// Minutes absent for every reason, loss or not.
    #[must_use]
    pub fn absent_minutes(&self) -> u32 {
        self.gaps
            .iter()
            .map(|gap| gap.minutes())
            .fold(0_u32, u32::saturating_add)
    }

    /// Minutes this build makes **no claim** about, either way.
    ///
    /// [`Reason::Unmeasured`] alone: a day outside `FIRST_DAY..=LAST_DAY`, or a
    /// Muhurat whose length the minute series does not reach. Neither a loss
    /// nor a clean bill.
    ///
    /// # Why this needed its own accessor
    ///
    /// It is the number that explains a `held` LARGER than an `expected`, and
    /// without it that pair reads as an arithmetic bug. Measured on the
    /// operator's store: NIFTY at one minute for 2026-08 answered
    /// `expected 5,625, held 7,500, lost 0` — twenty trading days of bars
    /// against fifteen the calendar could vouch for, because `LAST_DAY` is
    /// 2026-08-21 and the run was read on 2026-08-29. Every day past the
    /// table is `Unmeasured` and adds nothing to `expected`, which is correct
    /// and looks exactly like a defect until this number is beside it.
    #[must_use]
    pub fn unmeasured_minutes(&self) -> u32 {
        self.gaps
            .iter()
            .filter(|g| matches!(g.reason, Reason::Unmeasured))
            .map(|gap| gap.minutes())
            .fold(0_u32, u32::saturating_add)
    }
}

/// Minutes since IST midnight for a micros-since-epoch stamp, and its day.
///
/// IST is UTC+5:30 with no daylight saving, so the shift is a constant and
/// there is no zone table to consult — one add and two divides.
fn ist(ts_micros: i64) -> (i64, u16) {
    const IST_OFFSET_SECS: i64 = 5 * 3600 + 30 * 60;
    let secs = ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS;
    let day = secs.div_euclid(86_400);
    // `rem_euclid` is non-negative and under 86,400, so this is 0..=1439 and
    // `try_from` cannot fail. `u16::MAX` is the fallback rather than a panic
    // because it is the SAFE direction: 65,535 is not a minute-of-day, so an
    // impossible stamp matches no expected minute and is reported as a hole
    // rather than silently satisfying one.
    let minute = u16::try_from(secs.rem_euclid(86_400) / 60).unwrap_or(u16::MAX);
    (day, minute)
}

/// Classify every minute the calendar owes across `first ..= last`.
///
/// `stored` is the timestamps the store holds for that range, **in ascending
/// order** — which is what the store's append rule already guarantees, and what
/// lets this be a merge rather than a lookup.
///
/// # Cost
///
/// One step per expected minute and one per stored bar, each O(1). No set, no
/// map, no per-minute allocation.
///
/// **UNVERIFIED as a measurement** — see the module's own Cost note. The bound
/// is a property of the merge rather than a timing anyone took.
///
/// # Panics
///
/// Never. Every index is bounds-checked and every arithmetic is saturating.
#[must_use]
pub fn classify(stored: &[i64], first: i64, last: i64) -> Ledger {
    classify_against(stored, first, last, None)
}

/// [`classify`], against a calendar the caller supplies.
///
/// # Why this exists, and it is not a convenience
///
/// There are **two calendars in this workspace**, and this module's free
/// functions read the one that goes stale. `crate::calendar`'s table is typed
/// and bounded by [`crate::calendar::LAST_DAY`]; `api::calendar_of` DERIVES a
/// [`Calendar`] from the store's own daily and minute rungs and widens by
/// itself the moment an earlier month lands. `calendar_of`'s own header names
/// the hazard outright: *"Three tables of one fact is a drift waiting for a
/// date. This is the one answer both should read."*
///
/// Measured on 2026-08-29, eight days past `LAST_DAY = 2026-08-21`: an audit of
/// NIFTY at one minute for 2026-08 answered `expected 5,625, held 7,500,
/// lost 0` — twenty trading days of bars against fifteen the TABLE could vouch
/// for. Nothing was wrong with the store and nothing was wrong with the
/// classification; the denominator was reading a calendar that had run out.
///
/// So the caller with a derived calendar passes it, and the caller without one
/// gets the table. `None` is not a default that hides anything: it is the
/// honest answer for a caller that has no store to derive from, and the
/// `Unmeasured` runs it produces say so per day.
///
/// # Panics
///
/// Never. Same bounds as [`classify`].
#[must_use]
pub fn classify_against(
    stored: &[i64],
    first: i64,
    last: i64,
    against: Option<&Calendar>,
) -> Ledger {
    let mut ledger = Ledger {
        held: u32::try_from(stored.len()).unwrap_or(u32::MAX),
        ..Ledger::default()
    };
    // THE ONE CURSOR. It only ever moves forward, which is what makes the whole
    // walk linear in the two inputs rather than quadratic.
    let mut cursor = 0_usize;
    // The run being accumulated, so contiguous minutes of one reason collapse.
    let mut run: Option<Gap> = None;

    let mut day = first;
    while day <= last {
        // THE SUPPLIED CALENDAR WINS. A caller with a store to derive from has
        // a calendar that widens with it; the table is the fallback for one
        // that does not, and it is bounded by `LAST_DAY`.
        let kind = against.map_or_else(|| calendar::kind_of(day), |cal| cal.kind_of(day));
        // ADVANCE PAST ANY STORED BAR BEFORE THIS DAY. A bar the caller handed
        // us from outside the range is skipped rather than counted against it.
        while cursor < stored.len() && stored.get(cursor).copied().is_some_and(|t| ist(t).0 < day) {
            cursor = cursor.saturating_add(1);
        }

        match kind {
            // BOTH REPORT `Unmeasured`, AND NEITHER ADDS TO `expected`.
            //
            // `Unmeasured` is a day outside the calendar's range.
            // `OpenLengthUnmeasured` is a Muhurat the exchange traded and the
            // minute series does not reach — five days, every Diwali before
            // 2025. Calling those a `VendorHole` would claim a loss whose SIZE
            // this build cannot state, and calling them `Closed` would deny a
            // session the daily bar proves happened. They are flagged, and the
            // caller decides.
            DayKind::Unmeasured | DayKind::OpenLengthUnmeasured => {
                push(&mut ledger, &mut run, day, 0, 1_439, Reason::Unmeasured);
            }
            DayKind::Closed => {
                // A closed day owes nothing, so it produces one run and no
                // expected bars. Reported rather than skipped: an operator
                // asking "where did February go" deserves the answer.
                push(&mut ledger, &mut run, day, 0, 1_439, Reason::Closed);
            }
            DayKind::Open(session) => {
                ledger.expected = ledger.expected.saturating_add(u32::from(session.bars()));
                let mut minute = 0_u16;
                while minute <= 1_439 {
                    let held = stored
                        .get(cursor)
                        .copied()
                        .map(ist)
                        .is_some_and(|(d, m)| d == day && m == minute);
                    if held {
                        cursor = cursor.saturating_add(1);
                    } else if session.expects(minute) {
                        push(
                            &mut ledger,
                            &mut run,
                            day,
                            minute,
                            minute,
                            Reason::VendorHole,
                        );
                    } else {
                        push(
                            &mut ledger,
                            &mut run,
                            day,
                            minute,
                            minute,
                            Reason::OutsideWindow,
                        );
                    }
                    minute = minute.saturating_add(1);
                }
            }
        }
        // A RUN NEVER SPANS TWO DAYS. 15:29 on Friday and 09:15 on Monday are
        // not contiguous in any sense an operator cares about.
        flush(&mut ledger, &mut run);
        day += 1;
    }
    flush(&mut ledger, &mut run);
    ledger
}

/// Extend the open run, or close it and start another.
fn push(ledger: &mut Ledger, run: &mut Option<Gap>, day: i64, from: u16, to: u16, reason: Reason) {
    match run {
        Some(open)
            if open.day == day && open.reason == reason && open.to.saturating_add(1) == from =>
        {
            open.to = to;
        }
        _ => {
            flush(ledger, run);
            *run = Some(Gap {
                day,
                from,
                to,
                reason,
            });
        }
    }
}

/// Commit the open run, respecting [`MAX_GAPS`].
fn flush(ledger: &mut Ledger, run: &mut Option<Gap>) {
    let Some(gap) = run.take() else {
        return;
    };
    if ledger.gaps.len() >= MAX_GAPS {
        // LOUD, NOT SILENT. See `Ledger::truncated`.
        ledger.truncated = true;
        return;
    }
    ledger.gaps.push(gap);
}

/// Minutes this instrument lacks that a peer on the same feed **did** trade.
///
/// # Why a peer is the only witness that costs nothing
///
/// A calendar derived from one instrument cannot tell a scheduled break from a
/// hole: both are simply minutes with no bar. [`crate::calendar_of`] measures
/// that exactly — it reports 623,546 owed for NIFTY, which is what NIFTY holds,
/// so all 28 of its real holes are invisible.
///
/// A peer breaks the tie **for free**, because the bars are already on disk. If
/// BANKNIFTY traded 12:41 on a day NIFTY did not, the exchange was open at 12:41
/// and NIFTY is missing a bar. No vendor request, no calendar, no table.
///
/// # What it finds, and what it cannot — measured
///
/// On the operator's store, of the interior gap-minutes in each series:
///
/// | Series | Gap-minutes | Provable by a peer |
/// |---|---|---|
/// | NIFTY | 1,204 | **3** |
/// | BANKNIFTY | 1,252 | **51** |
/// | INDIAVIX | 1,203 | **2** |
/// | *common to all three* | *1,201* | *0 — see below* |
///
/// **1,201 of them are shared by all three**, and this function is silent about
/// every one. That is not a weakness in the rule; it is what the evidence
/// supports. A minute no instrument on the feed traded is either a scheduled
/// break or a feed-wide outage, and the bars cannot say which — 2023-06-14
/// 12:41–12:47 is missing from all three identically, which is the signature of
/// Zerodha's pipeline stopping rather than of three separate holes.
///
/// **Only another VENDOR settles those.** If Dhan or Groww holds 12:41 that day,
/// the exchange was open and Zerodha lost it.
///
/// So this is a strict lower bound on loss, and it is the right direction: it
/// never reports a hole that is not one. BANKNIFTY's 51 are real bars, absent,
/// and reported by nothing before this.
///
/// # Cost
///
/// One pass over the day's minutes, and one `expects` per peer per minute —
/// each a bounded walk of at most [`crate::calendar::MAX_WINDOWS`]. O(1) per
/// minute per peer, no allocation beyond the answer.
///
/// **UNVERIFIED as a measurement** — see the module's own Cost note. The per-peer
/// factor is real and is stated rather than folded into the O(1): this is
/// constant per minute PER PEER, so three feeds cost three walks. The bound on
/// each walk is `MAX_WINDOWS`, a `const`, which is what makes the inner one
/// constant rather than a search.
#[must_use]
pub fn provable_holes(
    day: i64,
    mine: Option<crate::calendar::Session>,
    peers: &[Option<crate::calendar::Session>],
) -> Vec<Gap> {
    let mut out = Vec::new();
    let mut run: Option<Gap> = None;
    let mut ledger = Ledger::default();
    for minute in 0..=1_439_u16 {
        let i_traded = mine.is_some_and(|s| s.expects(minute));
        // A PEER IS A WITNESS, AND ONE IS ENOUGH. Requiring a majority would
        // discard the case this exists for: a hole in two of three series is
        // still a hole in both.
        let peer_traded = peers
            .iter()
            .any(|peer| peer.is_some_and(|s| s.expects(minute)));
        if !i_traded && peer_traded {
            push(
                &mut ledger,
                &mut run,
                day,
                minute,
                minute,
                Reason::VendorHole,
            );
        } else {
            flush(&mut ledger, &mut run);
        }
    }
    flush(&mut ledger, &mut run);
    out.append(&mut ledger.gaps);
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod tests {
    use super::*;

    /// Micros-since-epoch for a day and an IST minute-of-day.
    fn at(day: i64, minute: u16) -> i64 {
        const IST_OFFSET_SECS: i64 = 5 * 3600 + 30 * 60;
        (day * 86_400 + i64::from(minute) * 60 - IST_OFFSET_SECS) * 1_000_000
    }

    /// Every bar a full session owes, for one day.
    fn full_day(day: i64) -> Vec<i64> {
        (calendar::OPEN_MINUTE..=calendar::LAST_MINUTE)
            .map(|m| at(day, m))
            .collect()
    }

    /// **A COMPLETE DAY REPORTS NOTHING MISSING — WHICH IS THE WHOLE POINT.**
    ///
    /// The defect this module exists to fix is a report that cried wolf on
    /// complete data. If a full 375-bar session produced a single gap, every
    /// alarm downstream would be noise again.
    #[test]
    fn a_complete_session_reports_no_loss_and_no_outside_window_noise() {
        // 2026-08-03, an ordinary Monday.
        let day = 20_668;
        let ledger = classify(&full_day(day), day, day);
        assert_eq!(ledger.held, 375);
        assert_eq!(ledger.expected, 375);
        assert_eq!(ledger.lost_minutes(), 0, "nothing was lost");
        assert!(
            ledger
                .gaps
                .iter()
                .all(|g| g.reason == Reason::OutsideWindow),
            "the only absences on a complete day are the 1,065 minutes outside \
             09:15-15:29, which are not losses"
        );
        assert!(!ledger.truncated);
    }

    /// **THE 2023-06-14 HOLE, REPRODUCED FROM THE OPERATOR'S OWN STORE.**
    ///
    /// Seven consecutive minutes at 12:41 were absent from a 368-bar day. They
    /// are mid-session, so nothing dropped them — the vendor did not send them —
    /// and they must be reported as the one reason that is a loss, collapsed
    /// into a single run rather than seven rows.
    #[test]
    fn a_mid_session_hole_is_one_run_of_vendor_hole_and_not_seven_rows() {
        let day = 19_522; // 2023-06-14
        let missing = 12 * 60 + 41;
        let bars: Vec<i64> = (calendar::OPEN_MINUTE..=calendar::LAST_MINUTE)
            .filter(|m| !(missing..missing + 7).contains(m))
            .map(|m| at(day, m))
            .collect();
        assert_eq!(bars.len(), 368, "the count the store actually holds");

        let ledger = classify(&bars, day, day);
        assert_eq!(ledger.held, 368);
        assert_eq!(ledger.expected, 375);
        assert_eq!(ledger.lost_minutes(), 7);

        let losses: Vec<&Gap> = ledger.gaps.iter().filter(|g| g.reason.is_loss()).collect();
        assert_eq!(losses.len(), 1, "ONE run, not seven rows");
        let hole = losses.first().expect("the run");
        assert_eq!(hole.from, missing, "12:41");
        assert_eq!(hole.to, missing + 6, "through 12:47");
        assert_eq!(hole.minutes(), 7);
    }

    /// **A MUHURAT HOUR IS NOT 315 MISSING BARS.**
    ///
    /// 2025-10-21 traded 13:45–14:44 and nothing else. An arithmetic count
    /// expecting 375 reports 315 losses; this must report **zero**, because the
    /// exchange owed sixty bars and delivered sixty.
    #[test]
    fn a_muhurat_session_owes_sixty_bars_and_loses_none() {
        let day = 20_382;
        let bars: Vec<i64> = (13 * 60 + 45..=14 * 60 + 44).map(|m| at(day, m)).collect();
        assert_eq!(bars.len(), 60);

        let ledger = classify(&bars, day, day);
        assert_eq!(ledger.expected, 60, "sixty owed, not 375");
        assert_eq!(ledger.held, 60);
        assert_eq!(
            ledger.lost_minutes(),
            0,
            "a short session is not a short store — this is the false alarm the \
             module exists to stop"
        );
    }

    /// **THE TWO-WINDOW SATURDAY HAS A HOLE THAT IS NOT A GAP.**
    ///
    /// 2024-03-02 traded 09:15–09:59 and 11:30–12:29. The ninety minutes
    /// between are `OutsideWindow` — never owed — while a minute absent from
    /// either window would be a loss. Both must be distinguishable on one day.
    #[test]
    fn the_gap_between_two_windows_is_not_a_loss_but_a_hole_inside_one_is() {
        let day = 19_784;
        let mut bars: Vec<i64> = (555..=599).map(|m| at(day, m)).collect();
        bars.extend((690..=749).map(|m| at(day, m)));
        assert_eq!(bars.len(), 105);

        let clean = classify(&bars, day, day);
        assert_eq!(clean.expected, 105);
        assert_eq!(clean.lost_minutes(), 0, "the midday break is not a loss");
        assert!(
            clean
                .gaps
                .iter()
                .any(|g| g.reason == Reason::OutsideWindow && g.from == 600),
            "and it IS reported, as outside-window: {:?}",
            clean.gaps
        );

        // NOW REMOVE ONE BAR FROM INSIDE THE SECOND WINDOW.
        let holed: Vec<i64> = bars
            .iter()
            .copied()
            .filter(|t| *t != at(day, 700))
            .collect();
        let ledger = classify(&holed, day, day);
        assert_eq!(ledger.lost_minutes(), 1, "and that one IS a loss");
    }

    /// **A CLOSED DAY AND AN UNMEASURED DAY ARE DIFFERENT ANSWERS.**
    ///
    /// Reporting "nobody has looked" as "the exchange was shut" is the
    /// invention `CLAUDE.md` §3 rule 1 bans, and it is exactly what an operator
    /// backfilling 2015 would be told by a calendar that guessed.
    #[test]
    fn a_holiday_and_an_unmeasured_day_do_not_share_a_reason() {
        // 2026-01-26, Republic Day — inside the calendar, closed.
        let holiday = classify(&[], 20_479, 20_479);
        assert_eq!(holiday.expected, 0, "a closed day owes nothing");
        assert_eq!(holiday.lost_minutes(), 0);
        assert_eq!(holiday.gaps.first().map(|g| g.reason), Some(Reason::Closed));

        // 2015-01-01 — before anything was measured.
        let unmeasured = classify(&[], 16_436, 16_436);
        assert_eq!(unmeasured.expected, 0);
        assert_eq!(unmeasured.lost_minutes(), 0, "not a loss — a non-claim");
        assert_eq!(
            unmeasured.gaps.first().map(|g| g.reason),
            Some(Reason::Unmeasured),
            "and NOT Closed"
        );
    }

    /// **A SUPPLIED CALENDAR OVERRIDES THE TABLE, AND THAT IS THE WHOLE POINT
    /// OF `classify_against`.**
    ///
    /// The table is bounded by [`calendar::LAST_DAY`], and on 2026-08-29 — eight
    /// days past it — an audit of NIFTY at one minute for 2026-08 answered
    /// `expected 5,625, held 7,500, lost 0`: twenty trading days of bars against
    /// fifteen the table could vouch for. Nothing was wrong with the store and
    /// nothing was wrong with the classification; the DENOMINATOR was reading a
    /// calendar that had run out.
    ///
    /// `api::calendar_of` derives this same type from the store's own rungs and
    /// widens by itself, so a caller with a store passes one. Driven here on a
    /// day the TABLE calls `Unmeasured`, because that is the case where the two
    /// sources give different answers and the supplied one must win — a test on
    /// a day they agree about would pass with the parameter ignored entirely.
    #[test]
    fn a_supplied_calendar_answers_for_a_day_the_table_has_never_heard_of() {
        // One day past the table's last, so the table alone can only shrug.
        let past = calendar::LAST_DAY + 1;
        let table_only = classify(&[], past, past);
        assert_eq!(
            table_only.gaps.first().map(|g| g.reason),
            Some(Reason::Unmeasured),
            "the table has nothing to say about a day it does not cover"
        );
        assert_eq!(
            table_only.expected, 0,
            "so it owes nothing, which is what makes `held` exceed `expected`"
        );

        // A derived calendar that DID see that day trade a full session.
        let observed =
            calendar::Observed::from_runs(past, &[(calendar::OPEN_MINUTE, calendar::LAST_MINUTE)]);
        let derived = calendar::Calendar::from_observed(&[observed]);
        let against = classify_against(&[], past, past, Some(&derived));
        assert_eq!(
            against.expected,
            u32::from(calendar::FULL_BARS),
            "the supplied calendar owes a full session where the table owed nothing"
        );
        assert_eq!(
            against.lost_minutes(),
            u32::from(calendar::FULL_BARS),
            "and an empty store against a day that traded is a real loss, which \
             the table could not have called"
        );
        assert_eq!(
            against.unmeasured_minutes(),
            0,
            "nothing is unclaimed once a calendar can speak for the day"
        );
    }

    /// **TRUNCATION IS ANNOUNCED, NEVER SILENT.**
    ///
    /// A ledger cut short that read like a complete one would tell an operator
    /// their store is healthier than it is — the §4 failure wearing a success's
    /// clothes, on the very report built to prevent it.
    #[test]
    fn a_truncated_ledger_says_so() {
        // Every day of the calendar, holding nothing: far past MAX_GAPS.
        let ledger = classify(&[], calendar::FIRST_DAY, calendar::LAST_DAY);
        assert!(ledger.truncated, "the walk hit the bound");
        assert_eq!(ledger.gaps.len(), MAX_GAPS, "and stopped exactly there");
    }

    /// **A PEER THAT TRADED IS A WITNESS, AND ONE IS ENOUGH.**
    ///
    /// The case this exists for: BANKNIFTY holds 51 gap-minutes that NIFTY and
    /// INDIAVIX do not share, measured on the operator's store. Every one is a
    /// bar the exchange offered and Zerodha did not deliver, and nothing
    /// reported them before — `calendar_of` reads 623,546 owed for NIFTY, which
    /// is exactly what NIFTY holds, so a single-instrument calendar is blind to
    /// its own holes by construction.
    #[test]
    fn a_minute_a_peer_traded_and_this_one_did_not_is_a_provable_hole() {
        let full = crate::calendar::Session::full();
        // MINE IS MISSING 12:41-12:47; the peer holds the whole session.
        let mine = crate::calendar::Observed::from_runs(19_522, &[(555, 760), (768, 929)]).session;

        let holes = provable_holes(19_522, mine, &[Some(full)]);
        assert_eq!(holes.len(), 1, "ONE run, not seven rows: {holes:?}");
        let hole = holes.first().expect("the run");
        assert_eq!(hole.from, 761, "12:41");
        assert_eq!(hole.to, 767, "through 12:47");
        assert_eq!(hole.minutes(), 7);
        assert_eq!(hole.reason, Reason::VendorHole);
        assert!(hole.reason.is_loss());
    }

    /// **A MINUTE NO PEER TRADED IS NOT REPORTED, AND THAT IS THE POINT.**
    ///
    /// 1,201 of the operator's gap-minutes are shared by all three series —
    /// 2023-06-14 12:41–12:47 is missing from every one of them identically,
    /// which is Zerodha's pipeline stopping rather than three separate holes.
    /// The bars cannot distinguish that from a scheduled break, so this
    /// function says nothing about it.
    ///
    /// Silence is the safe direction: it never reports a hole that is not one.
    /// Only another VENDOR settles those minutes.
    #[test]
    fn a_minute_no_peer_traded_is_silence_and_not_a_guess() {
        let broken =
            crate::calendar::Observed::from_runs(19_522, &[(555, 760), (768, 929)]).session;

        // EVERY PEER HAS THE SAME GAP — a feed-wide outage.
        let holes = provable_holes(19_522, broken, &[broken, broken]);
        assert!(
            holes.is_empty(),
            "a gap every series shares is not provable from bars alone: {holes:?}"
        );

        // AND THE MIDDAY BREAK OF A DR SATURDAY IS THE SAME SHAPE. All three
        // stop at 09:59 and resume at 11:30, so nothing is claimed.
        let dr = crate::calendar::Observed::from_runs(19_784, &[(555, 599), (690, 749)]).session;
        assert!(provable_holes(19_784, dr, &[dr, dr]).is_empty());
    }

    /// **A DAY THIS SERIES HAS NO MINUTE BARS FOR IS EVERY PEER MINUTE.**
    ///
    /// The five pre-2025 Muhurats arrive as `None`. Where a peer DID trade that
    /// hour, every minute of it is a provable hole; where no peer did — which is
    /// the real case, since Zerodha's minute history reaches none of them — the
    /// answer is silence.
    #[test]
    fn a_day_with_no_minute_series_is_all_hole_or_all_silence() {
        let hour = crate::calendar::Observed::from_runs(20_028, &[(825, 884)]).session;

        // A peer traded the Muhurat hour and this series holds nothing.
        let holes = provable_holes(20_028, None, &[hour]);
        assert_eq!(holes.len(), 1);
        assert_eq!(holes.first().map(|g| g.minutes()), Some(60));

        // No peer traded it either — the operator's actual case.
        assert!(provable_holes(20_028, None, &[None, None]).is_empty());
    }
}
