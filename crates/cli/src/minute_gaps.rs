//! Days whose one-minute series has a hole, and what withholding them costs.
//!
//! # The refusal this removes, and it is not the charter's
//!
//! `SWEPT_SERIES_CALENDAR_POLICY` withholds the nine days `docs/00-charter.md`
//! names as non-regular sessions. This is a different fault with the same shape:
//! an ORDINARY trading day whose one-minute series is missing a minute the
//! vendor never served.
//!
//! MEASURED on the operator's freshly pulled zerodha NIFTY store, by decoding
//! every one-minute timestamp of all 82 months and looking for a gap inside a
//! session — **28 minutes across 12 days**:
//!
//! | IST day | date | minutes lost |
//! |---|---|---|
//! | 18 305 | 2020-02-13 | 1 |
//! | 18 325 | 2020-03-04 | 1 |
//! | 18 501 | 2020-08-27 | 2 |
//! | 18 568 | 2020-11-02 | 1 |
//! | 18 981 | 2021-12-20 | 1 |
//! | 19 058 | 2022-03-07 | 3 |
//! | 19 513 | 2023-06-05 | 1 |
//! | 19 522 | 2023-06-14 | 7 |
//! | 19 550 | 2023-07-12 | 1 |
//! | 19 562 | 2023-07-24 | 3 |
//! | 19 576 | 2023-08-07 | 1 |
//! | 19 836 | 2024-04-23 | 6 |
//!
//! A coarse rung's bucket closes on a minute that must exist:
//! `overlay_exact_minute_gapfib` demands the minute opening at the signal bar's
//! close and refuses `MissingClosingMinute` when it is absent. One such minute
//! anywhere in the span refuses the WHOLE span, so six of the eight rungs could
//! not run over 2020-01..2026-07 at all — for 28 minutes in 618,000.
//!
//! # Re-pulling does not fix it, which is why this exists
//!
//! MEASURED: the operator wiped the store and pulled zerodha from scratch. The
//! same 28 minutes came back missing. `crates/pull/src/gaps.rs` names the reason
//! for a `VendorHole` — *"It cannot be repaired from the same vendor: a re-run
//! returns the same nothing"* — and 2022-03-07 is absent from dhan too, so that
//! one is exchange-side and unrecoverable from any feed.
//!
//! # Why withholding the DAY and not substituting the minute
//!
//! Four tests exist to stop exactly that substitution —
//! `exact_minute_overlay_never_substitutes_a_later_minute` and its three
//! siblings — and their reasoning is right: a minute other sessions hold,
//! missing from this one, is a hole, and papering over it is what `CLAUDE.md`
//! §4 forbids. **None of them is weakened here.** A hole still refuses when its
//! day is swept. What changes is that the day is not swept: it is removed from
//! the sample, counted, named by date, and bound into the run identity, exactly
//! as a charter session is.
//!
//! The distinction that makes this honest rather than a fallback: substituting
//! would answer a question with the wrong bar. Withholding declines to answer
//! it, out loud.
//!
//! # Removed from the SAMPLE, not from the HISTORY (p11num-1, D-1781)
//!
//! Until D-1781 the paragraph above was false. Every door cut the holed day
//! out of the signal bars before the indicator fold, so the day left the
//! evaluator's input as well as the sample: every family that carries state
//! across sessions -- EMA20/200 (0-5), ATR and `SuperTrend` (64-65), swings, BOS
//! and `CHoCH` (56-59, 72-73), structure (278-279) -- was computed on day D-1
//! spliced onto D+1, and the swept days after D carried masks no unspliced run
//! produces. Nothing said so. MEASURED on the finding's repro (14 sessions of
//! 375 bars, one interior minute missing): 40 of the 1,125 later bars differed
//! on trend bits.
//!
//! Now the day is folded and not swept. [`withhold`] still produces the swept
//! slice, but the column is built over the WHOLE signal series by
//! `stored_anchored_column_withholding`, which steps the holed day through
//! the evaluator and gives its bars no row
//! (`indicators::column::AnchoredColumn::build_required_withholding`). The
//! state a later day starts from is the fold of every bar the store holds,
//! the holed day's included -- with its missing minute still missing, because
//! nothing reconstructs it. [`MINUTE_GAP_POLICY`] was 2 from D-1781 (now 4), and every
//! withholding door binds the withheld days into its data term with
//! [`bind_withheld`], so no run made under the splice shares an identity with
//! one made under the fold on a span that withheld a day.
//!
//! # Measured, never written down
//!
//! The twelve days above are evidence, not a table. [`days_with_interior_gaps`]
//! derives them from the minute stream it is handed, so a re-pull that recovers
//! a minute silently stops withholding that day, and a new hole is caught
//! without an edit. A hardcoded list would be the invention `CLAUDE.md` §3
//! rule 1 forbids and would rot the first time the store changed.
//!
//! # Cost
//!
//! One pass over the minute slice comparing each bar to its predecessor —
//! O(minutes), the same order as the load that produced it, with no allocation
//! beyond the day list. Withholding is one pass per slice. Neither is one of the
//! five operations §3 rule 4 bounds. **UNVERIFIED as a measured bound**; read
//! off the source per §3 rule 6.

use indicators::Candle;

/// Version of the rule that keeps a holed session out of a swept series.
///
/// # Why a term of its own beside `SWEPT_SERIES_CALENDAR_POLICY`
///
/// The two withhold for different reasons and can change independently. The
/// charter policy answers *"is this session non-regular?"* from a fixed list of
/// exchange facts; this answers *"can this day's minutes price a coarse bucket's
/// close?"* from the bytes actually on disk. A build could change either without
/// the other, and under one version number a later change to this would be
/// indistinguishable from a change to that.
///
/// # Version 2 (D-1662)
///
/// The census also withholds a day whose demanded closing minute is absent
/// ([`days_with_minute_holes`]), so a holed span withholds more days than
/// version 1 did and must not share its identity.
///
/// # Version 3 (D-1781, p11num-1, D-1934)
///
/// Versions 1 and 2 removed a holed day from the indicator fold as well as
/// from the sample, splicing D-1 onto D+1. Version 3 folds the day and
/// withholds only its rows, so the masks on later days change. A second
/// branch numbered that change 2 while D-1662 also took 2; the merge gives the
/// combined computation its own number so neither version-2 run shares it.
/// Earlier runs keep their recorded identities and stay valid under them
/// (`CLAUDE.md` §3 rule 8); they are not this computation.
///
/// # Version 4 (lookahead-oos-1, g-zerodha-universe-census-3)
///
/// Versions 1 to 3 withheld EVERY row of a holed session, so a 10:00 row
/// disappeared because the vendor lost a minute at 14:02: which rows exist
/// depended on minutes after them, the look-ahead `CLAUDE.md` §3 rule 7 forbids
/// (as D-0943, D-1182 and D-1514 read it). Version 4 is causal: a holed day
/// is CUT at its first hole -- the first missing minute, the first signal
/// bucket whose demanded closing minute is absent, or the start of a recorded
/// halt ([`pull::calendar::halt_on`], the one authority) -- and every row
/// whose bucket lies wholly before the cut is admitted, while every row from
/// it to the session end is withheld. The fold stays whole (D-1781).
/// [`bind_withheld`] binds each day's cut, so no version-3 run shares an
/// identity with a version-4 run that withheld anything.
pub const MINUTE_GAP_POLICY: u32 = 4;

/// Bind the cuts a door applied into its data term. D-1781, version 4.
///
/// Since D-1781 a withholding door digests the WHOLE folded signal series,
/// so the withheld rows no longer leave a mark on its data term by being
/// absent from it. This puts them back, with the rule's version: a holed
/// span's term is `blake3(tag ‖ MINUTE_GAP_POLICY ‖ digest ‖ count ‖ cuts)`,
/// each cut its day, its cut instant, its first withheld bar-open and its
/// cause, so two runs that cut different days, or one day at different
/// instants, or under a different rule, cannot share it.
///
/// A span that withheld nothing returns `digest` unchanged. Its column is the
/// same fold under either version -- nothing was cut and nothing is left out
/// -- so its data term has nothing new to say; `sweep-stored`, which binds the
/// version as a params term of its own (`stored_month_params`), still moves.
#[must_use]
pub fn bind_withheld(digest: [u8; 32], cuts: &[Cut]) -> [u8; 32] {
    if cuts.is_empty() {
        return digest;
    }
    let mut hash = brutex_core::blake3::Hasher::new();
    hash.update(b"brutex-minute-gap-causal-cuts\0");
    hash.update(&MINUTE_GAP_POLICY.to_le_bytes());
    hash.update(&digest);
    hash.update(&u64::try_from(cuts.len()).unwrap_or(u64::MAX).to_le_bytes());
    for cut in cuts {
        hash.update(&cut.day.to_le_bytes());
        hash.update(&cut.at_micros.to_le_bytes());
        hash.update(&cut.from_micros.to_le_bytes());
        hash.update(&[cut.cause.code()]);
    }
    hash.finalize()
}

/// One minute, in microseconds.
const MINUTE_MICROS: i64 = 60_000_000;

/// One day, in microseconds.
const DAY_MICROS: i64 = 86_400_000_000;

/// What cut a holed day where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CutCause {
    /// A minute the series skips inside the session: the first missing one.
    Hole,
    /// A signal bucket whose demanded closing minute
    /// ([`indicators::anchored::exact_closing_minute`]) is not held.
    ClosingMinute,
    /// A recorded market-wide halt ([`pull::calendar::HALTS`]): from the first
    /// whole halted minute, whatever bars the feed served in it.
    Halt,
    /// The exact-minute overlay could not source a minute the census did not
    /// foresee: the retry loops' defence, cut at the minute the refusal named.
    Unsourceable,
}

impl CutCause {
    /// The word a report prints for it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Hole => "minute hole",
            Self::ClosingMinute => "closing minute absent",
            Self::Halt => "halt",
            Self::Unsourceable => "unsourceable minute",
        }
    }

    /// Its byte in [`bind_withheld`].
    const fn code(self) -> u8 {
        match self {
            Self::Hole => 1,
            Self::ClosingMinute => 2,
            Self::Halt => 3,
            Self::Unsourceable => 4,
        }
    }
}

/// One holed day and where it is cut: rows whose bucket lies wholly before
/// [`Self::at_micros`] are admitted, every later row of the day is withheld.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Cut {
    /// The IST day ([`indicators::ist_day`]).
    pub day: i64,
    /// The cut instant: the first missing minute's open, the demanded closing
    /// minute that is absent, or the halt's first whole minute.
    pub at_micros: i64,
    /// The first signal-bar open this rung withholds: `at - length + 1`, so a
    /// bar is withheld exactly when its bucket reaches past the cut.
    pub from_micros: i64,
    /// Why the day is cut.
    pub cause: CutCause,
    /// Rows of this day admitted before the cut (filled by [`counted`]).
    pub admitted: u64,
    /// Rows of this day withheld from the cut on (filled by [`counted`]).
    pub withheld: u64,
}

impl Cut {
    /// The cut of `day` at `at_micros` for a rung of `signal_length_micros`.
    #[must_use]
    pub const fn new(day: i64, at_micros: i64, cause: CutCause, signal_length_micros: i64) -> Self {
        Self {
            day,
            at_micros,
            from_micros: at_micros
                .saturating_sub(signal_length_micros)
                .saturating_add(1),
            cause,
            admitted: 0,
            withheld: 0,
        }
    }

    /// Whether this cut withholds the signal bar opening at `ts_micros`.
    #[must_use]
    pub fn withholds(&self, ts_micros: i64) -> bool {
        indicators::ist_day(ts_micros) == self.day && ts_micros >= self.from_micros
    }

    /// `HH:MM` IST of the cut instant.
    #[must_use]
    pub fn clock(&self) -> String {
        let minute = self
            .at_micros
            .saturating_add(indicators::IST_OFFSET_MICROS)
            .rem_euclid(DAY_MICROS)
            / MINUTE_MICROS;
        format!("{:02}:{:02}", minute / 60, minute % 60)
    }
}

/// The admission rule of a list of cuts, as one O(1) probe per bar: a map from
/// each cut day to its first withheld bar-open. Built once per span.
#[derive(Clone, Debug, Default)]
pub struct Admission {
    from: std::collections::HashMap<i64, i64>,
}

impl Admission {
    /// The rule `cuts` describe. A day cut twice keeps its EARLIER cut.
    #[must_use]
    pub fn of(cuts: &[Cut]) -> Self {
        let mut from = std::collections::HashMap::with_capacity(cuts.len());
        for cut in cuts {
            from.entry(cut.day)
                .and_modify(|held: &mut i64| *held = (*held).min(cut.from_micros))
                .or_insert(cut.from_micros);
        }
        Self { from }
    }

    /// Whether the signal bar opening at `ts_micros` is admitted.
    #[must_use]
    pub fn admits(&self, ts_micros: i64) -> bool {
        self.from.is_empty()
            || self
                .from
                .get(&indicators::ist_day(ts_micros))
                .is_none_or(|from| ts_micros < *from)
    }

    /// The `(day, first withheld bar-open)` pairs the indicator fold takes.
    #[must_use]
    pub fn pairs(cuts: &[Cut]) -> Vec<(i64, i64)> {
        cuts.iter().map(|cut| (cut.day, cut.from_micros)).collect()
    }
}

/// Which days a run cut for a minute gap or a halt, and what they cost it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GapExclusion {
    /// The cuts, ascending by day, each counted over the folded series.
    cuts: Vec<Cut>,
    /// Signal bars withheld.
    signal_bars: u64,
    /// Exact one-minute bars withheld.
    minute_bars: u64,
}

impl GapExclusion {
    /// Nothing was withheld.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            cuts: Vec::new(),
            signal_bars: 0,
            minute_bars: 0,
        }
    }

    /// What a door that withholds from its SIGNAL bars alone removed, on a
    /// coarse rung, counted over `folded`. D-0694.
    ///
    /// On a coarse rung the separately loaded execution minutes stay whole:
    /// no signal row exists from a cut on, so none of those minutes is
    /// looked up for an entry. So no minute bar was removed, and this says so
    /// rather than counting one. On `1min` there is no separate series, and
    /// [`Self::one_series`] is the answer.
    #[must_use]
    pub fn signal_only(cuts: &[Cut], folded: &[Candle]) -> Self {
        let cuts = counted(folded, cuts);
        let signal_bars = cuts.iter().map(|cut| cut.withheld).sum();
        Self {
            cuts,
            signal_bars,
            minute_bars: 0,
        }
    }

    /// What a door removed from a series that is both its signal and its
    /// execution: the `1min` rung, counted over `folded`. D-0694.
    #[must_use]
    pub fn one_series(cuts: &[Cut], folded: &[Candle]) -> Self {
        let mut exclusion = Self::signal_only(cuts, folded);
        exclusion.minute_bars = exclusion.signal_bars;
        exclusion
    }

    /// How many days were cut.
    #[must_use]
    pub fn days(&self) -> usize {
        self.cuts.len()
    }

    /// The cut IST days, ascending.
    #[must_use]
    pub fn day_numbers(&self) -> Vec<i64> {
        self.cuts.iter().map(|cut| cut.day).collect()
    }

    /// The cuts, ascending by day, counted.
    #[must_use]
    pub fn cuts(&self) -> &[Cut] {
        &self.cuts
    }

    /// Signal bars removed.
    #[must_use]
    pub const fn signal_bars(&self) -> u64 {
        self.signal_bars
    }

    /// Exact one-minute bars removed.
    #[must_use]
    pub const fn minute_bars(&self) -> u64 {
        self.minute_bars
    }

    /// Whether this run withheld nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cuts.is_empty()
    }

    /// Each cut day as `YYYY-MM-DD from HH:MM (cause): A admitted before, W
    /// withheld`, ascending.
    ///
    /// Named and not merely counted, for the reason `CLAUDE.md` §3 rule 2
    /// gives: an operator reading "twelve days were withheld" cannot check that
    /// against anything, and a day that will not convert falls back to its raw
    /// number rather than being dropped.
    #[must_use]
    pub fn day_names(&self) -> Vec<String> {
        self.cuts.iter().map(describe).collect()
    }
}

/// One cut as a report names it.
#[must_use]
pub fn describe(cut: &Cut) -> String {
    format!(
        "{} from {} ({}): {} admitted before, {} withheld",
        date_of(cut.day),
        cut.clock(),
        cut.cause.word(),
        cut.admitted,
        cut.withheld
    )
}

/// `YYYY-MM-DD`, or the raw day number when it will not convert.
fn date_of(day: i64) -> String {
    u32::try_from(day)
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
        .map_or_else(|| day.to_string(), |date| date.to_string())
}

/// The cut days, ascending.
#[must_use]
pub fn days_of(cuts: &[Cut]) -> Vec<i64> {
    cuts.iter().map(|cut| cut.day).collect()
}

/// Days whose one-minute series skips a minute inside a session.
///
/// A gap is a minute the calendar's session expects and the series lacks:
/// before the day's first held bar (a missing 09:15), or between two ADJACENT
/// bars on the SAME IST day. The same-day test is what makes this a hole
/// rather than a night: the step from 15:29 to the next morning's 09:15 is
/// enormous and is not a fault. See [`first_interior_holes`].
///
/// The result is ascending and holds each day once.
#[must_use]
pub fn days_with_interior_gaps(minutes: &[Candle]) -> Vec<i64> {
    first_interior_holes(minutes)
        .into_iter()
        .map(|(day, _)| day)
        .collect()
}

/// Each holed day once, with the open of its FIRST missing minute.
///
/// A minute is MISSING when the exchange calendar's session for its day
/// ([`pull::calendar::kind_of`]) expects it and the series does not hold it.
/// Two walks see that, in one pass:
///
/// - **The leading hole.** A day whose first held minute is later than its
///   session's first minute is holed AT the session's first minute, so a day
///   with no 09:15 bar is cut at 09:15 and every row of it is withheld on
///   every rung. The interior walk below cannot see this: the step into a
///   day's first bar comes from the previous night and fails its same-day
///   test. Found by AH-01's first re-check (132 of 415,277 real Zerodha
///   share-days lacked the open minute and were admitted after the hole).
/// - **The interior hole.** Between two ADJACENT bars of the same IST day, the
///   first minute the session expects after the earlier one, when it is
///   before the later one. The same-day test keeps the step from 15:29 to the
///   next morning's 09:15 from being a fault, and the calendar keeps the gap
///   between a two-window session's windows (the disaster-recovery Saturdays,
///   09:15-09:59 and 11:30-12:29) from being one: no minute in it is expected.
///
/// A traded day OUTSIDE the calendar's measured range ([`pull::calendar::FIRST_DAY`]
/// 2019-12-02 to [`pull::calendar::LAST_DAY`] 2026-09-04, `Unmeasured`) holds
/// bars, so it traded; it is held to the regular session
/// ([`pull::calendar::Session::full`], 09:15-15:29), whose first minute it
/// must hold. Zerodha's `1min` reaches back to 2018-01 and on to 2026-09-30, so
/// without this a missing 09:15 before 2019-12 or after 2026-09-04 was never a
/// hole. A special session there that began late would be withheld whole and
/// named, never admitted after a hole (`docs/06-limits.md` L-AH1-11).
///
/// A day the calendar says was closed, or a session whose length was never
/// measured (the five pre-2025 Muhurats), has no known first minute, so it
/// gets no leading check, and its interior check is the rule the
/// calendar-free walk always applied: any same-day step longer than a minute
/// is a hole at the minute after the earlier bar.
///
/// The scan visits bars in time order, so the list is already ascending and a
/// day's first hole is the earliest. A hole is revealed by the first bar after
/// it, or by the calendar alone, and the cut is AT the hole, so no row before
/// the hole depends on it. One `kind_of` per day (O(1), at most nine
/// comparisons) and O(1) per bar.
fn first_interior_holes(minutes: &[Candle]) -> Vec<(i64, i64)> {
    let mut holes: Vec<(i64, i64)> = Vec::new();
    let mut previous: Option<&Candle> = None;
    // The day the walk is on and its calendar session, looked up once a day.
    let mut session: Option<(i64, Option<pull::calendar::Session>)> = None;
    for bar in minutes {
        let day = indicators::ist_day(bar.ts_micros);
        let earlier = previous
            .filter(|before| indicators::ist_day(before.ts_micros) == day)
            .map(|before| minute_of_day(before.ts_micros));
        previous = Some(bar);
        if holes.last().map(|hole| hole.0) == Some(day) {
            continue;
        }
        let calendar = match session {
            Some((held, calendar)) if held == day => calendar,
            _ => {
                let calendar = match pull::calendar::kind_of(day) {
                    pull::calendar::DayKind::Open(open) => Some(open),
                    pull::calendar::DayKind::Unmeasured => Some(pull::calendar::Session::full()),
                    pull::calendar::DayKind::Closed
                    | pull::calendar::DayKind::OpenLengthUnmeasured => None,
                };
                session = Some((day, calendar));
                calendar
            }
        };
        let minute = minute_of_day(bar.ts_micros);
        // The first minute expected after the earlier bar (or the session's
        // first minute, for the day's first bar). Without a calendar there is
        // no first minute to expect, and the gap rule is the calendar-free one.
        let expected = match calendar {
            Some(open) => first_expected_after(open, earlier),
            None => earlier.map(|before| before.saturating_add(1)),
        };
        if let Some(missing) = expected.filter(|next| *next < minute) {
            holes.push((
                day,
                day_open_micros(day).saturating_add(missing.saturating_mul(MINUTE_MICROS)),
            ));
        }
    }
    holes
}

/// The IST minute of day `ts_micros` opens in.
fn minute_of_day(ts_micros: i64) -> i64 {
    ts_micros
        .saturating_add(indicators::IST_OFFSET_MICROS)
        .rem_euclid(DAY_MICROS)
        / MINUTE_MICROS
}

/// The instant IST midnight opens `day`.
fn day_open_micros(day: i64) -> i64 {
    day.saturating_mul(DAY_MICROS)
        .saturating_sub(indicators::IST_OFFSET_MICROS)
}

/// The first minute `session` expects strictly after `earlier`, or its very
/// first minute when `earlier` is `None` (the day's first held bar). `None`
/// when the session expects nothing later. At most [`pull::calendar::MAX_WINDOWS`]
/// windows, so O(1).
fn first_expected_after(session: pull::calendar::Session, earlier: Option<i64>) -> Option<i64> {
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .find_map(|window| {
            let (from, to) = (i64::from(window.from), i64::from(window.to));
            let candidate = earlier.map_or(from, |before| from.max(before.saturating_add(1)));
            (candidate <= to).then_some(candidate)
        })
}

/// The cuts a stored span applies before its column is built: for each day,
/// the EARLIEST of its first interior hole ([`days_with_interior_gaps`]'s
/// walk), its first signal bar whose demanded closing minute the minute
/// stream does not hold (W2-cli9-3, D-1662), and the start of a halt
/// [`pull::calendar::halt_on`] records for it. Version 4 of
/// [`MINUTE_GAP_POLICY`].
///
/// # The holes the interior walk could not see
///
/// The interior walk sees a day's missing first minute and every gap between
/// adjacent minutes on the SAME day, so a session
/// that stops early (15:25 to 15:29 missing) steps from its last minute to the
/// next morning's 09:15, fails the same-day test, and is never flagged. A day
/// holding signal bars and no minutes at all is invisible to it for the same
/// reason. For each signal bar this computes the minute
/// [`indicators::anchored::exact_closing_minute`] says the overlay will demand,
/// with `session_close`, which every caller takes from
/// [`crate::stored::session_close_for`] -- the same close the stored column
/// passes the overlay (D-2102) -- and checks that exact stamp is held. So the
/// census and the join cannot disagree about which bucket is unsourceable, and
/// the retry loops behind it become a defence.
///
/// # Recorded halts are holes from their start (census-3)
///
/// A feed may serve frozen or auction-indicative index minutes through a
/// market-wide halt. They are not continuous trading, so every bucket that
/// overlaps the halt's first whole minute is cut, whatever the minute stream
/// holds. `HALTS` is the one authority; no second list exists here.
/// 2021-02-24 never reaches this: the charter's non-regular days are withheld
/// whole when the month is decoded (`indicators::evaluator::
/// CHARTER_NON_REGULAR_IST_DAYS`).
///
/// # Causal
///
/// Every cut instant is read from minutes at or before itself (a hole is seen
/// by the minute after it, and the cut is AT the hole), from the bar's own
/// demanded minute, or from the calendar. Rows before the cut never depend on
/// anything after it: the prefix property
/// `minute_gaps_causal_tests::a_row_before_a_later_hole_is_admitted` checks.
///
/// # Cost
///
/// O(signal + minutes): one interior pass, one cursor walk that only moves
/// forward because the demanded minute is non-decreasing in signal order, one
/// `halt_on` (three entries) per signal day, and two O(d) merges of ascending
/// per-day lists (no sort).
#[must_use]
pub fn days_with_minute_holes(
    signal: &[Candle],
    minutes: &[Candle],
    signal_length_micros: i64,
    session_close: impl Fn(i64) -> Option<u16>,
) -> Vec<Cut> {
    let interior: Vec<Cut> = first_interior_holes(minutes)
        .into_iter()
        .map(|(day, at)| Cut::new(day, at, CutCause::Hole, signal_length_micros))
        .collect();
    let mut edges: Vec<Cut> = Vec::new();
    let mut halts: Vec<Cut> = Vec::new();
    let mut cursor = 0_usize;
    let mut seen_day: Option<i64> = None;
    for bar in signal {
        let day = indicators::ist_day(bar.ts_micros);
        if seen_day != Some(day) {
            seen_day = Some(day);
            if let Some(halt) = pull::calendar::halt_on(day) {
                let start = day_open_micros(day)
                    .saturating_add(i64::from(halt.halted.from).saturating_mul(MINUTE_MICROS));
                halts.push(Cut::new(day, start, CutCause::Halt, signal_length_micros));
            }
        }
        let expected = indicators::anchored::exact_closing_minute(
            bar.ts_micros,
            signal_length_micros,
            session_close(day),
        );
        while minutes
            .get(cursor)
            .is_some_and(|minute| minute.ts_micros < expected)
        {
            cursor = cursor.saturating_add(1);
        }
        if minutes
            .get(cursor)
            .is_none_or(|minute| minute.ts_micros != expected)
            && edges.last().map(|cut| cut.day) != Some(day)
        {
            edges.push(Cut::new(
                day,
                expected,
                CutCause::ClosingMinute,
                signal_length_micros,
            ));
        }
    }
    merge_cuts(&merge_cuts(&interior, &edges), &halts)
}

/// The ascending union of two ascending per-day cut lists, each day once,
/// keeping the EARLIER cut of a day both name. A tie keeps `left`'s, except
/// that a recorded HALT wins it: a feed that lost its minutes from the halt's
/// first whole minute is explained by the halt, and naming it a minute hole
/// would send the operator to the vendor for an exchange event.
///
/// Both inputs are built by a forward walk over time-ordered bars, so each is
/// already ascending; a two-cursor merge keeps that order in O(a + b) without
/// the sort gate 11 rule 4 refuses on this path.
fn merge_cuts(left: &[Cut], right: &[Cut]) -> Vec<Cut> {
    let mut out: Vec<Cut> = Vec::with_capacity(left.len().saturating_add(right.len()));
    let (mut l, mut r) = (left.iter().peekable(), right.iter().peekable());
    loop {
        let next = match (l.peek(), r.peek()) {
            (Some(a), Some(b)) if a.day <= b.day => l.next().copied(),
            (Some(_) | None, Some(_)) => r.next().copied(),
            (Some(_), None) => l.next().copied(),
            (None, None) => break,
        };
        if let Some(cut) = next {
            match out.last_mut() {
                Some(last) if last.day == cut.day => {
                    if cut.at_micros < last.at_micros
                        || (cut.at_micros == last.at_micros && cut.cause == CutCause::Halt)
                    {
                        *last = cut;
                    }
                }
                _ => out.push(cut),
            }
        }
    }
    out
}

/// Add `cut` to `cuts`, or move its day's cut EARLIER. Returns `false` when
/// the day already holds a cut at or before it, which means withholding did
/// not remove the bar that needed the minute: a different defect, and the
/// retry loops refuse on it rather than spin. O(d) for d cut days.
#[must_use]
pub fn tighten(cuts: &mut Vec<Cut>, cut: Cut) -> bool {
    let at = cuts
        .iter()
        .position(|held| held.day >= cut.day)
        .unwrap_or(cuts.len());
    match cuts.get_mut(at) {
        Some(held) if held.day == cut.day => {
            if cut.from_micros < held.from_micros {
                *held = cut;
                true
            } else {
                false
            }
        }
        _ => {
            cuts.insert(at, cut);
            true
        }
    }
}

/// Remove every signal bar a cut withholds, counting what went.
///
/// Returns the kept bars and the number removed. One [`Admission`] probe per
/// bar: a hash lookup of the bar's day, O(1) however many days are cut, so the
/// loop is O(bars) (the reasoning the old day SET gave, kept).
#[must_use]
pub fn withhold(bars: &[Candle], cuts: &[Cut]) -> (Vec<Candle>, u64) {
    if cuts.is_empty() {
        return (bars.to_vec(), 0);
    }
    let admission = Admission::of(cuts);
    let mut kept: Vec<Candle> = Vec::with_capacity(bars.len());
    let mut removed = 0_u64;
    for bar in bars {
        if admission.admits(bar.ts_micros) {
            kept.push(*bar);
        } else {
            removed = removed.saturating_add(1);
        }
    }
    (kept, removed)
}

/// `cuts` with each day's rows admitted before and withheld after its cut,
/// counted over `folded`. One pass with one hash probe per bar.
#[must_use]
pub fn counted(folded: &[Candle], cuts: &[Cut]) -> Vec<Cut> {
    let mut out: Vec<Cut> = cuts.to_vec();
    let index: std::collections::HashMap<i64, usize> = out
        .iter()
        .enumerate()
        .map(|(position, cut)| (cut.day, position))
        .collect();
    for cut in &mut out {
        cut.admitted = 0;
        cut.withheld = 0;
    }
    for bar in folded {
        if let Some(&position) = index.get(&indicators::ist_day(bar.ts_micros))
            && let Some(cut) = out.get_mut(position)
        {
            if bar.ts_micros >= cut.from_micros {
                cut.withheld = cut.withheld.saturating_add(1);
            } else {
                cut.admitted = cut.admitted.saturating_add(1);
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "minute_gaps_causal_tests.rs"]
mod causal_tests;

#[cfg(test)]
#[path = "minute_gaps_measure_tests.rs"]
mod measure_tests;

#[cfg(test)]
mod tests {

    /// `days_with_minute_holes` unions its ascending per-day lists by a merge,
    /// not a sort (gate 11 rule 4, D-1662): ascending, each day once, the
    /// EARLIER cut of a day both name, on every interleaving including empty
    /// sides and full overlap.
    #[test]
    fn the_flagged_day_union_is_ascending_and_each_day_once() {
        use super::{Cut, CutCause, merge_cuts};
        let cut = |day: i64, at: i64| Cut::new(day, at, CutCause::Hole, 60_000_000);
        let days = |cuts: &[Cut]| {
            cuts.iter()
                .map(|c| (c.day, c.at_micros))
                .collect::<Vec<_>>()
        };
        assert!(merge_cuts(&[], &[]).is_empty());
        assert_eq!(days(&merge_cuts(&[cut(3, 9)], &[])), vec![(3, 9)]);
        assert_eq!(days(&merge_cuts(&[], &[cut(3, 9)])), vec![(3, 9)]);
        assert_eq!(
            days(&merge_cuts(
                &[cut(1, 5), cut(4, 7), cut(9, 1)],
                &[cut(2, 2), cut(4, 3), cut(10, 0)]
            )),
            vec![(1, 5), (2, 2), (4, 3), (9, 1), (10, 0)]
        );
        assert_eq!(
            days(&merge_cuts(
                &[cut(5, 1), cut(6, 1)],
                &[cut(1, 1), cut(2, 1)]
            )),
            vec![(1, 1), (2, 1), (5, 1), (6, 1)]
        );
        assert_eq!(days(&merge_cuts(&[cut(7, 8)], &[cut(7, 9)])), vec![(7, 8)]);
        assert_eq!(days(&merge_cuts(&[cut(7, 9)], &[cut(7, 8)])), vec![(7, 8)]);
        // Against a reference over a deterministic sweep: per day, the minimum.
        for seed in 0_i64..64 {
            let left: Vec<Cut> = (0..seed % 7)
                .map(|k| cut(k * 3 + seed % 2, seed - k))
                .collect();
            let right: Vec<Cut> = (0..seed % 5)
                .map(|k| cut(k * 2 + seed % 3, k + seed % 4))
                .collect();
            let mut expected: std::collections::BTreeMap<i64, i64> =
                std::collections::BTreeMap::new();
            for c in left.iter().chain(&right) {
                expected
                    .entry(c.day)
                    .and_modify(|at| *at = (*at).min(c.at_micros))
                    .or_insert(c.at_micros);
            }
            assert_eq!(
                days(&merge_cuts(&left, &right)),
                expected.into_iter().collect::<Vec<_>>(),
                "seed {seed}"
            );
        }
    }
    use super::*;

    /// A one-minute bar `minute` minutes past 09:15 IST on `day`.
    fn bar_on(day: i64, minute: i64) -> Candle {
        const IST_OFFSET_MICROS: i64 = 19_800 * 1_000_000;
        const OPEN_MINUTE: i64 = 555;
        let ts = day
            .saturating_mul(86_400)
            .saturating_add(OPEN_MINUTE.saturating_mul(60))
            .saturating_mul(1_000_000)
            .saturating_sub(IST_OFFSET_MICROS)
            .saturating_add(minute.saturating_mul(MINUTE_MICROS));
        Candle {
            ts_micros: ts,
            open: 100,
            high: 110,
            low: 90,
            close: 105,
            volume: 1,
            open_interest: i64::MIN,
        }
    }

    /// A contiguous session has no gap, and an overnight step is not one.
    #[test]
    fn a_contiguous_series_and_an_overnight_step_are_both_gap_free() {
        let mut bars: Vec<Candle> = (0..30).map(|m| bar_on(19_000, m)).collect();
        bars.extend((0..30).map(|m| bar_on(19_001, m)));
        assert!(
            days_with_interior_gaps(&bars).is_empty(),
            "the night between two sessions is not a hole"
        );
    }

    /// One missing minute names its day, once, however wide the hole.
    #[test]
    fn an_interior_hole_names_its_day_exactly_once() {
        let mut bars: Vec<Candle> = Vec::new();
        for m in 0..10 {
            bars.push(bar_on(19_522, m));
        }
        // 19_522 is 2023-06-14, where the operator's store loses seven minutes.
        for m in 18..30 {
            bars.push(bar_on(19_522, m));
        }
        assert_eq!(days_with_interior_gaps(&bars), vec![19_522]);

        // A second, separate hole on the same day still names it once.
        let mut twice: Vec<Candle> = Vec::new();
        for m in [0, 1, 5, 6, 20, 21] {
            twice.push(bar_on(19_522, m));
        }
        assert_eq!(days_with_interior_gaps(&twice), vec![19_522]);
    }

    /// Several holed days come back ascending, and clean days are untouched.
    #[test]
    fn every_holed_day_is_named_ascending_and_clean_days_are_not() {
        let mut bars: Vec<Candle> = Vec::new();
        // clean
        bars.extend((0..20).map(|m| bar_on(18_305, m)));
        // holed
        bars.extend([0, 1, 2, 9, 10].map(|m| bar_on(18_501, m)));
        // clean
        bars.extend((0..20).map(|m| bar_on(18_568, m)));
        // holed
        bars.extend([0, 7, 8].map(|m| bar_on(19_058, m)));
        assert_eq!(days_with_interior_gaps(&bars), vec![18_501, 19_058]);
    }

    /// The signal and a one-minute stream are cut at the SAME instant, each
    /// withholding exactly the rows whose bucket reaches past it.
    #[test]
    fn the_signal_and_the_minute_stream_lose_the_same_days() {
        let mut minutes: Vec<Candle> = Vec::new();
        minutes.extend((0..20).map(|m| bar_on(19_000, m)));
        minutes.extend([0, 1, 12, 13].map(|m| bar_on(19_001, m))); // holed from minute 2
        minutes.extend((0..20).map(|m| bar_on(19_002, m)));

        // A five-minute signal series over the same three days.
        let mut signal: Vec<Candle> = Vec::new();
        for day in [19_000, 19_001, 19_002] {
            signal.extend((0..4).map(|b| bar_on(day, b * 5)));
        }
        let five = 5 * MINUTE_MICROS;
        let cuts = days_with_minute_holes(&signal, &minutes, five, |_| None);
        assert_eq!(days_of(&cuts), vec![19_001]);
        assert_eq!(
            cuts.first().map(|cut| cut.at_micros),
            Some(bar_on(19_001, 2).ts_micros)
        );
        let excluded = GapExclusion::signal_only(&cuts, &signal);
        assert_eq!(excluded.day_numbers(), &[19_001]);
        assert_eq!(excluded.days(), 1);
        // The 09:15 bucket holds the hole (09:17), so the whole day's signal goes.
        assert_eq!(excluded.signal_bars(), 4, "every bucket reaches past 09:17");
        assert_eq!(excluded.minute_bars(), 0, "a coarse rung keeps its minutes");
        assert!(!excluded.is_empty());

        // On one minute the two rows before the hole are admitted.
        let minute_cuts = days_with_minute_holes(&minutes, &minutes, MINUTE_MICROS, |_| None);
        let (kept, removed) = withhold(&minutes, &minute_cuts);
        assert_eq!(removed, 2, "09:27 and 09:28 are after the hole");
        assert!(kept.contains(&bar_on(19_001, 0)) && kept.contains(&bar_on(19_001, 1)));
        let one = GapExclusion::one_series(&minute_cuts, &minutes);
        assert_eq!((one.signal_bars(), one.minute_bars()), (2, 2));
        assert_eq!(
            one.cuts().first().map(|cut| (cut.admitted, cut.withheld)),
            Some((2, 2))
        );
    }

    /// A gap-free span withholds nothing and copies the slice unchanged.
    #[test]
    fn a_gap_free_span_withholds_nothing() {
        let minutes: Vec<Candle> = (0..40).map(|m| bar_on(19_000, m)).collect();
        let signal: Vec<Candle> = (0..8).map(|b| bar_on(19_000, b * 5)).collect();
        let cuts = days_with_minute_holes(&signal, &minutes, 5 * MINUTE_MICROS, |_| None);
        assert!(cuts.is_empty());
        let (kept, removed) = withhold(&signal, &cuts);
        assert_eq!((kept, removed), (signal.clone(), 0));
        let excluded = GapExclusion::signal_only(&cuts, &signal);
        assert!(excluded.is_empty());
        assert_eq!(excluded.days(), 0);
        assert_eq!(excluded.signal_bars(), 0);
        assert_eq!(excluded.minute_bars(), 0);
        assert!(excluded.day_names().is_empty());
        assert_eq!(bind_withheld([7; 32], &cuts), [7; 32]);
    }

    /// The cut days are named as dates with their cut time, cause and counts.
    #[test]
    fn withheld_days_are_named_as_dates() {
        let minutes: Vec<Candle> = [0_i64, 1, 9, 10]
            .iter()
            .map(|m| bar_on(19_522, *m))
            .collect();
        let cuts = days_with_minute_holes(&minutes, &minutes, MINUTE_MICROS, |_| None);
        let excluded = GapExclusion::one_series(&cuts, &minutes);
        assert_eq!(
            excluded.day_names(),
            vec!["2023-06-14 from 09:17 (minute hole): 2 admitted before, 2 withheld".to_owned()]
        );
    }
}
