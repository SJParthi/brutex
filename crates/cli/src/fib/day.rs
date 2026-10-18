//! The day input every FIB stage reads, and `cli fib day-check` (FIB-8;
//! D-6440, D-6441, D-6445 to D-6451).
//!
//! # Minutes always come from Zerodha, folded here (operator rule a)
//!
//! A stock-month's one-minute bars are read through the one door,
//! `cli::stored::load(root, Vendor::Zerodha, symbol, "1min", year, month)`
//! (classified by [`crate::stored::load_with_class`], FM-F099), split by IST
//! day, and folded IN THE RUN to every width of the run's Generation through
//! `pull::fold::complete_minutes_with_calendar` for the cash venue — the one
//! fold authority, anchored at 09:15 and restarted at each session window. The
//! stored derived Zerodha rungs and GDFL bars are never read for candles, the
//! move or the rank. Zerodha minute VOLUME decides nothing (DATA FLAGS).
//!
//! # A bucket a minute is missing from is withheld by name
//!
//! For a share the fold folds a bucket over the minutes that traded. This
//! module marks every folded candle `complete` only when every continuous
//! minute the bucket owes is a real minute and the bucket is a full width;
//! otherwise the candle is kept with `complete = false` and its
//! `missing_minutes`, and named in its width's `withheld` list. A short last
//! bucket is partial and never a decision candle. A padded no-trade minute
//! (FM-F095) is a hole for the detector and STALE for the rank.
//!
//! # The unit of refusal is the stock-day (FM-F094)
//!
//! A duplicate, unsorted or corrupt minute refuses that day only, by name; a
//! day whose last minute closes before 15:09:00 is `tail_cut` when a witness
//! shows trades after it and `tail_unwitnessed` (labelled) when none can say,
//! and is kept, labelled `no_trades_after`, when the witness shows none. A
//! refused day emits no candle and makes the next day's previous close
//! refused. The minute closes the rank reads are kept: the rank is never
//! removed retroactively.
//!
//! # The previous close (D-6441, FM-F092 as amended by RQ-4)
//!
//! The previous TRADABLE day's last Zerodha minute close, carried across months
//! in [`StockState`] (FIB-9 checkpoints it) with its IST day, its landing and
//! its regime tag (the last minute-of-day of its session, FM-F101). Both
//! references — the previous TRADED SESSION and the previous TRADABLE day — are
//! computed and labelled (ZD-41); only the recorded [`PreviousReference`]
//! decides. A close is accepted only when it is from the reference's previous
//! day (`not_adjacent`), its day was not refused (`previous_day_refused`), it
//! is positive (`not_positive`), and the basis-verdict authority proves the
//! basis (`basis_unproven`, `basis_unreliable`). A refused day has no move and
//! no side, known at 09:15. The labelled variant `zerodha_1day_close` reads the
//! stored Zerodha `1day` bar and never decides.
//!
//! # Look-ahead
//!
//! The fold reads bars one at a time; every candle's completeness reads only
//! its own minutes; the move, the side and the minute closes at slot `s` read
//! minutes up to `s` and the previous day's close. The day-level refusals
//! (FM-F094) and the tail witness are judged over the whole day after its
//! close: a look-ahead in ELIGIBILITY that FIB-15 states and checks; it never
//! reaches a value a decision bar reads.
//!
//! # Cost
//!
//! O(1) per (stock, width) per minute: one pass to split, one fold per width,
//! one prefix count per day for completeness, one slot per minute. Every
//! authority lookup is O(1) (`super::authorities`).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::vendor::Vendor;
use indicators::Candle;
use indicators::fibflag::types::{FibCandle, MoveSide};
use pull::calendar::{CasEligibility, Phase};
use pull::fold::Bucket;
use pull::vendor::Venue;
use store::format::Bar;
use vocab::fib_cube::{
    BP_PER_UNIT, MICROS_PER_SECOND, SECONDS_PER_DAY, SECONDS_PER_MINUTE,
    SESSION_FORCED_WINDOW_START_SECS, SESSION_GRID_ANCHOR_SECS, max_width_minutes,
};

use super::authorities::{
    Authorities, BasisProof, BasisRefusal, BasisVerdictAuthority as _, CalendarCode,
    CalendarReason, ExpiryStamp, FNO_BAN_ABSENT, LandingId, SeriesId, TailReading, Tradable,
    YearCounts, year_of,
};
use crate::stored::{self, CashCloses, LoadClass};

// ----------------------------------------------------------------- time --

/// Microseconds in a minute.
fn micros_per_minute() -> i64 {
    i64::try_from(MICROS_PER_SECOND * SECONDS_PER_MINUTE).unwrap_or(i64::MAX)
}

/// Microseconds in a second.
fn micros_per_second() -> i64 {
    i64::try_from(MICROS_PER_SECOND).unwrap_or(i64::MAX)
}

/// The IST offset in microseconds.
fn ist_offset_micros() -> i64 {
    pull::session::IST_OFFSET_SECS * micros_per_second()
}

/// The IST epoch day and second of the day of an instant.
#[must_use]
pub fn ist_day_second(ts_micros: i64) -> (i64, u32) {
    let local = ts_micros.saturating_add(ist_offset_micros());
    let day_micros = i64::try_from(SECONDS_PER_DAY).unwrap_or(i64::MAX) * micros_per_second();
    let second = local.rem_euclid(day_micros) / micros_per_second();
    (
        local.div_euclid(day_micros),
        u32::try_from(second).unwrap_or(u32::MAX),
    )
}

/// The UTC instant of second `second` of IST day `day`.
#[must_use]
pub fn instant(day: i64, second: u32) -> i64 {
    let day_seconds = i64::try_from(SECONDS_PER_DAY).unwrap_or(i64::MAX);
    (day * day_seconds + i64::from(second)) * micros_per_second() - ist_offset_micros()
}

/// The minute-of-day of the 09:15 grid anchor.
fn anchor_minute() -> u16 {
    u16::try_from(SESSION_GRID_ANCHOR_SECS / SECONDS_PER_MINUTE).unwrap_or(u16::MAX)
}

/// The minute-closes table's slots: the minutes 09:15 up to the one that
/// closes at 15:09:00, the forced window's start.
#[must_use]
pub fn rank_slots() -> usize {
    usize::try_from(
        (SESSION_FORCED_WINDOW_START_SECS - SESSION_GRID_ANCHOR_SECS) / SECONDS_PER_MINUTE,
    )
    .unwrap_or(usize::MAX)
}

/// One minute in seconds.
fn minute_seconds() -> u32 {
    u32::try_from(SECONDS_PER_MINUTE).unwrap_or(u32::MAX)
}

/// The civil day of an IST epoch day.
fn civil(day: i64) -> Option<pull::session::Day> {
    u32::try_from(day)
        .ok()
        .and_then(|days| pull::session::Day::from_days(days).ok())
}

/// An IST epoch day as `YYYY-MM-DD`, or `day N` where no civil date exists.
#[must_use]
pub fn date(day: i64) -> String {
    civil(day).map_or_else(|| format!("day {day}"), |civil| civil.to_string())
}

// --------------------------------------------------------------- window --

/// A cash share's continuous window on one day: the inclusive minute-of-day
/// opens of its first and last minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Window {
    /// The first minute, 09:15.
    pub first: u16,
    /// The last minute: 15:29 before 2026-08-03; from then the share's dated
    /// close less one (15:14 for an eligible share).
    pub last: u16,
}

impl Window {
    /// Minutes in the window.
    #[must_use]
    pub fn minutes(self) -> u16 {
        self.last.saturating_sub(self.first).saturating_add(1)
    }

    /// Whether minute-of-day `minute` is inside it.
    #[must_use]
    pub const fn holds(self, minute: u16) -> bool {
        minute >= self.first && minute <= self.last
    }
}

/// The continuous window of a full session for a cash share on `day`: the
/// calendar's full session before 2026-08-03, and from then the share's dated
/// close from its receipted master (`CashCloses`), never a guess.
///
/// # Errors
///
/// `session_unverified` naming the master's refusal when a dated close is
/// owed and not held.
pub fn window_of(day: i64, cash: &CashCloses) -> Result<Window, String> {
    let full = pull::calendar::Session::full();
    let first = full
        .windows
        .first()
        .map_or(anchor_minute(), |window| window.from);
    let dated = civil(day).is_some_and(pull::vendor::cash_auction_eligibility_required);
    let last = if dated {
        cash.session_close_minute(day).ok_or_else(|| {
            format!(
                "session_unverified: {}: {}",
                date(day),
                cash.unverified_reason(day)
                    .unwrap_or("no dated cash-auction flag is held")
            )
        })?
    } else {
        full.windows.first().map_or(first, |window| window.to)
    };
    Ok(Window { first, last })
}

/// The dated closing-auction flag of a share on `day`, as the calendar's
/// phase input: `Unknown` before 2026-08-03 (never read there) and where no
/// dated flag is held.
#[must_use]
pub fn cas_of(day: i64, cash: &CashCloses) -> CasEligibility {
    let Some(civil) =
        civil(day).filter(|civil| pull::vendor::cash_auction_eligibility_required(*civil))
    else {
        return CasEligibility::Unknown;
    };
    let eligible = Venue::NseCash
        .share_hours_on(civil, CasEligibility::Eligible)
        .ok()
        .and_then(|hours| u16::try_from(hours.close_minute()).ok());
    match (cash.session_close_minute(day), eligible) {
        (Some(last), Some(close)) if last.saturating_add(1) == close => CasEligibility::Eligible,
        (Some(_), _) => CasEligibility::NotEligible,
        (None, _) => CasEligibility::Unknown,
    }
}

// ------------------------------------------------------------- refusals --

/// Why a stock-day's minutes were refused (FM-F094). APPEND-ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DayRefusal {
    /// Two minutes share a timestamp.
    DuplicateMinute,
    /// A minute is earlier than the one before it.
    Unsorted,
    /// A minute's prices or counts are impossible, or it is off the minute grid.
    CorruptMinute,
    /// The last minute closes before 15:09:00 and a witness shows trades after it.
    TailCut,
    /// The last minute closes before 15:09:00 and no witness can say: labelled.
    TailUnwitnessed,
    /// The day's continuous window cannot be attested (no dated close).
    SessionUnverified,
    /// The fold authority refused the day's minutes.
    FoldRefused,
}

impl DayRefusal {
    /// Every refusal, in counting order.
    pub const ALL: &[Self] = &[
        Self::DuplicateMinute,
        Self::Unsorted,
        Self::CorruptMinute,
        Self::TailCut,
        Self::TailUnwitnessed,
        Self::SessionUnverified,
        Self::FoldRefused,
    ];

    /// The counted name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::DuplicateMinute => "duplicate_minute",
            Self::Unsorted => "unsorted",
            Self::CorruptMinute => "corrupt_minute",
            Self::TailCut => "tail_cut",
            Self::TailUnwitnessed => "tail_unwitnessed",
            Self::SessionUnverified => "session_unverified",
            Self::FoldRefused => "fold_refused",
        }
    }

    /// The refusal's position in [`Self::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Why a previous close was not accepted (FM-F092, RQ-4). APPEND-ONLY.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CloseRefusal {
    /// The stock's first day: no previous close at all.
    NoPrevious,
    /// The carried close is not from the reference's previous day.
    NotAdjacent,
    /// The carried close's day was refused (FM-F094).
    PreviousDayRefused,
    /// The carried close is not positive.
    NotPositive,
    /// No basis proof holds.
    BasisUnproven,
    /// DATA-1 or PREC-13 marks the day non-constant or non-proportional.
    BasisUnreliable,
}

impl CloseRefusal {
    /// Every refusal, in counting order.
    pub const ALL: &[Self] = &[
        Self::NoPrevious,
        Self::NotAdjacent,
        Self::PreviousDayRefused,
        Self::NotPositive,
        Self::BasisUnproven,
        Self::BasisUnreliable,
    ];

    /// The counted name; a candidate on the day is refused
    /// `no_previous_close:<name>`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::NoPrevious => "first_day",
            Self::NotAdjacent => "not_adjacent",
            Self::PreviousDayRefused => "previous_day_refused",
            Self::NotPositive => "not_positive",
            Self::BasisUnproven => "basis_unproven",
            Self::BasisUnreliable => "basis_unreliable",
        }
    }

    /// The refusal's position in [`Self::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Which previous day decides the move (ZD-41): both are computed and
/// labelled; the run records one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviousReference {
    /// The calendar's previous traded session (a halt day included).
    TradedSession,
    /// The previous tradable day (operator rule h).
    TradableDay,
}

impl PreviousReference {
    /// The flag word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::TradedSession => "traded-session",
            Self::TradableDay => "tradable-day",
        }
    }

    /// The reference a word names.
    ///
    /// # Errors
    ///
    /// `previous_reference_required` naming both words.
    pub fn of(word: Option<&str>) -> Result<Self, String> {
        match word {
            Some("traded-session") => Ok(Self::TradedSession),
            Some("tradable-day") => Ok(Self::TradableDay),
            _ => Err(
                "previous_reference_required: name `--previous traded-session` or \
                      `--previous tradable-day`; both are computed and labelled, the named \
                      one decides (ZD-41)"
                    .to_owned(),
            ),
        }
    }
}

/// A close carried to a later day: the close, its IST day, its landing and
/// its regime tag (FM-F092, FM-F101).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CarriedClose {
    /// The close, paisa.
    pub close: i64,
    /// Its IST epoch day.
    pub day: i64,
    /// The landing the day came from, per the landing authority.
    pub landing: Option<LandingId>,
    /// The last minute-of-day of its session: the regime tag (15:29 before
    /// 2026-08-03; 15:14 for an eligible share from then).
    pub regime_last_minute: u16,
    /// Whether its day was refused (FM-F094).
    pub refused: bool,
}

/// An accepted previous close: the carried close, scaled exactly by a
/// PREC-10 ratio when one carried it across a basis break.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AcceptedClose {
    /// What was carried.
    pub carried: CarriedClose,
    /// How its basis was proven.
    pub proof: BasisProof,
    /// Whether only the landing test could apply (labelled `landing_only`).
    pub landing_only: bool,
}

impl AcceptedClose {
    /// The close on the day's basis as an exact ratio `(numerator,
    /// denominator)` of paisa.
    #[must_use]
    pub fn scaled(&self) -> (i128, i128) {
        match self.proof {
            BasisProof::BreakRecord(record) => (
                i128::from(self.carried.close) * i128::from(record.numerator),
                i128::from(record.denominator),
            ),
            BasisProof::SameLanding | BasisProof::FactorPairsEqual => {
                (i128::from(self.carried.close), 1)
            }
        }
    }

    /// The move of `close` against it, in basis points, floored: exact, in
    /// `i128`.
    #[must_use]
    pub fn move_bp(&self, close: i64) -> i128 {
        let (numerator, denominator) = self.scaled();
        let bp = i128::from(BP_PER_UNIT);
        ((i128::from(close) * denominator - numerator) * bp).div_euclid(numerator.max(1))
    }

    /// The stock's own move side at a minute closing at `close`.
    #[must_use]
    pub fn side(&self, close: i64) -> MoveSide {
        let (numerator, denominator) = self.scaled();
        match (i128::from(close) * denominator).cmp(&numerator) {
            std::cmp::Ordering::Greater => MoveSide::Gainer,
            std::cmp::Ordering::Less => MoveSide::Loser,
            std::cmp::Ordering::Equal => MoveSide::Flat,
        }
    }
}

/// The verdict on one reference's previous close for a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PreviousClose {
    /// Accepted.
    Accepted(AcceptedClose),
    /// Refused, with what was carried when anything was.
    Refused(CloseRefusal, Option<CarriedClose>),
}

impl PreviousClose {
    /// The accepted close, if it was.
    #[must_use]
    pub const fn accepted(&self) -> Option<&AcceptedClose> {
        match self {
            Self::Accepted(close) => Some(close),
            Self::Refused(..) => None,
        }
    }

    /// The counted word: `accepted` or the refusal's name.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match self {
            Self::Accepted(_) => "accepted",
            Self::Refused(refusal, _) => refusal.name(),
        }
    }

    /// What was carried, accepted or not.
    #[must_use]
    pub const fn carried(&self) -> Option<CarriedClose> {
        match self {
            Self::Accepted(close) => Some(close.carried),
            Self::Refused(_, carried) => *carried,
        }
    }
}

// ---------------------------------------------------------------- state --

/// A share's committed own past for the wick check (FM-F096): the widest
/// wick seen, per direction, as an exact ratio of the minute's close.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WickPast {
    /// The widest upper wick: `(wick, close)` paisa.
    pub up: Option<(i64, i64)>,
    /// The widest lower wick.
    pub down: Option<(i64, i64)>,
}

/// Whether `a.0 / a.1 > b.0 / b.1`, cross-multiplied in `i128`, closes positive.
fn wider(a: (i64, i64), b: (i64, i64)) -> bool {
    i128::from(a.0) * i128::from(b.1) > i128::from(b.0) * i128::from(a.1)
}

/// The per-stock state carried across months; FIB-9 checkpoints it. No
/// `Default`: a stock starts at [`StockState::first_day`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StockState {
    /// The last close of the last day with minutes (the traded-session reference).
    pub session: Option<CarriedClose>,
    /// The last close of the last tradable, admitted day.
    pub tradable: Option<CarriedClose>,
    /// The committed own past of the wick check.
    pub wick: WickPast,
}

impl StockState {
    /// A stock with no past: its first day has no previous close (counted).
    #[must_use]
    pub const fn first_day() -> Self {
        Self {
            session: None,
            tradable: None,
            wick: WickPast {
                up: None,
                down: None,
            },
        }
    }
}

// ------------------------------------------------------------ day input --

/// What one minute slot of the closes table holds.
pub use super::files::MinuteClose;

/// One width's candles on one day.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidthCandles {
    /// The width, whole minutes.
    pub width: u16,
    /// The day's candle count at this width from its session length: FIB-3's
    /// ring capacity.
    pub capacity: u16,
    /// The folded candles, complete and not.
    pub candles: Vec<FibCandle>,
    /// Every bucket withheld, by name: the fold's notes and every incomplete
    /// candle.
    pub withheld: Vec<String>,
}

/// A wick wider than every wick of the stock's committed days, per direction
/// (FM-F096): `suspect_unwitnessed` in stage A, resolved only by deep or a
/// DATA-1 per-minute record. The minute is kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SuspectWick {
    /// Minutes after 09:15.
    pub slot: u16,
    /// Whether it is the upper wick.
    pub upper: bool,
}

/// The halt-day exposure input (FM-F100; FIB-7's touch-point): on a day a
/// halt removed by hindsight, the candles that closed at or before the first
/// halted second — the decisions a live trader had. Never committed to own
/// past, never in a deciding half.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HaltExposure {
    /// The cash market's first halted second of the IST day.
    pub first_halted_second: u32,
    /// Each width's candles closing at or before it.
    pub candles: Vec<WidthCandles>,
}

/// The reference stamps of a stock-day (FM-F098): never a coordinate, never
/// an identity term.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DayStamps {
    /// The expiry stamp.
    pub expiry: ExpiryStamp,
    /// The F&O ban stamp: absent, the ban list is UNVERIFIED.
    pub fno_ban: &'static str,
}

/// Minute counts of one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MinuteCounts {
    /// Real minutes inside the continuous window.
    pub real: u32,
    /// Padded no-trade minutes (FM-F095): holes for the detector.
    pub no_trade: u32,
    /// Zero-volume minutes with a range: kept and counted.
    pub zero_volume_with_range: u32,
    /// Minutes outside the continuous window: counted, never used.
    pub outside_continuous: u32,
}

impl MinuteCounts {
    const NONE: Self = Self {
        real: 0,
        no_trade: 0,
        zero_volume_with_range: 0,
        outside_continuous: 0,
    };
}

/// One stock-day of the day input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DayInput {
    /// The IST epoch day.
    pub day: i64,
    /// The calendar's verdict, market-wide and per instrument.
    pub tradable: Tradable,
    /// Why the day's minutes were refused, if they were.
    pub refusal: Option<DayRefusal>,
    /// Whether a witness showed no trade after an early last minute: kept,
    /// labelled `no_trades_after`.
    pub no_trades_after: bool,
    /// The continuous window, when it could be attested.
    pub window: Option<Window>,
    /// The day's minute counts.
    pub counts: MinuteCounts,
    /// Each width's candles: admitted tradable days only.
    pub candles: Vec<WidthCandles>,
    /// The halt-day exposure input, on a halt day only.
    pub exposure: Option<HaltExposure>,
    /// The minute closes 09:15..15:09 the rank reads.
    pub closes: Vec<MinuteClose>,
    /// The traded-session reference's previous close.
    pub previous_session: PreviousClose,
    /// The tradable-day reference's previous close.
    pub previous_tradable: PreviousClose,
    /// The recorded reference: the one that decides.
    pub reference: PreviousReference,
    /// The stock's own move side per slot, when the deciding previous close
    /// was accepted and the slot holds a close.
    pub sides: Vec<Option<MoveSide>>,
    /// The `zerodha_1day_close` variant: the stored `1day` close of the
    /// deciding reference's previous day, labelled, never deciding.
    pub variant_1day_close: Option<i64>,
    /// Suspect wicks.
    pub suspect: Vec<SuspectWick>,
    /// The day's last real minute close.
    pub last_close: Option<i64>,
    /// The day's reference stamps.
    pub stamps: DayStamps,
}

impl DayInput {
    /// The deciding previous close.
    #[must_use]
    pub const fn previous(&self) -> &PreviousClose {
        match self.reference {
            PreviousReference::TradedSession => &self.previous_session,
            PreviousReference::TradableDay => &self.previous_tradable,
        }
    }

    /// Whether stage A reads the day: tradable, admitted, its minutes not
    /// refused.
    #[must_use]
    pub const fn admitted(&self) -> bool {
        self.tradable.admits() && self.refusal.is_none()
    }

    /// Why a candidate on the day is refused for want of a previous close,
    /// `no_previous_close:<reason>`, or `None` when the move is known.
    #[must_use]
    pub fn candidate_refusal(&self) -> Option<String> {
        match self.previous() {
            PreviousClose::Accepted(_) => None,
            PreviousClose::Refused(refusal, _) => {
                Some(format!("no_previous_close:{}", refusal.name()))
            }
        }
    }
}

/// Counts of one month, per IST year.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonthCounts {
    /// Calendar exclusions, per [`CalendarCode`]; every calendar day walked.
    pub calendar: YearCounts<{ CalendarCode::ALL.len() }>,
    /// Day refusals, per [`DayRefusal`].
    pub refusals: YearCounts<{ DayRefusal::ALL.len() }>,
    /// Deciding previous-close refusals, per [`CloseRefusal`].
    pub previous: YearCounts<{ CloseRefusal::ALL.len() }>,
    /// Tradable days with no minute at all.
    pub tradable_without_minutes: u64,
    /// Days admitted to stage A.
    pub admitted: u64,
    /// Days whose per-instrument session is unchecked (FM-F097), shown on
    /// the banner.
    pub instrument_session_unchecked: u64,
    /// Days kept with `no_trades_after`.
    pub no_trades_after: u64,
    /// Padded no-trade minutes.
    pub no_trade_minutes: u64,
    /// Zero-volume minutes with a range.
    pub zero_volume_with_range: u64,
    /// Minutes outside the continuous window.
    pub outside_continuous: u64,
    /// Suspect wicks marked.
    pub suspect_wicks: u64,
    /// Accepted previous closes proven by the landing test alone.
    pub landing_only: u64,
}

/// One stock-month of the day input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonthInput {
    /// The symbol it loaded under.
    pub symbol: String,
    /// The series.
    pub series: SeriesId,
    /// The year.
    pub year: u16,
    /// The month.
    pub month: u8,
    /// Every IST day of the month that the calendar admits or that holds a
    /// minute, in order.
    pub days: Vec<DayInput>,
    /// Every calendar day the gate excluded, with its reason.
    pub excluded: Vec<(i64, CalendarReason)>,
    /// The counts.
    pub counts: MonthCounts,
}

/// What a month build is asked for.
#[derive(Clone, Copy, Debug)]
pub struct MonthSpec<'a> {
    /// The symbol it loads under.
    pub symbol: &'a str,
    /// The series.
    pub series: SeriesId,
    /// The year.
    pub year: u16,
    /// The month.
    pub month: u8,
    /// The Generation's timeframe set, whole minutes, strictly increasing.
    pub widths: &'a [u16],
    /// The deciding previous-close reference.
    pub reference: PreviousReference,
    /// The stored Zerodha `1day` bars for the labelled variant, or `None`.
    pub daily: Option<&'a [Candle]>,
}

/// Refuses a width set that is empty, not strictly increasing, or holds a
/// width outside `1..=`[`max_width_minutes`] (R7).
///
/// # Errors
///
/// `widths_refused`, naming the set.
pub fn check_widths(widths: &[u16]) -> Result<(), String> {
    let top = max_width_minutes();
    let in_range = widths
        .iter()
        .all(|width| *width >= 1 && u64::from(*width) <= top);
    let increasing = widths.windows(1 + 1).all(|pair| match pair {
        [a, b] => a < b,
        _ => true,
    });
    if widths.is_empty() || !in_range || !increasing {
        return Err(format!(
            "widths_refused: {widths:?}; the timeframe set is whole minutes, strictly \
             increasing, each in 1..={top} (R7)"
        ));
    }
    Ok(())
}

/// The coarse doubling ladder 1, 2, 4, ... up to N_max (R7): `day-check`'s
/// widths when the caller names none.
#[must_use]
pub fn doubling_ladder() -> Vec<u16> {
    let top = max_width_minutes();
    std::iter::successors(Some(1_u16), |width| width.checked_add(*width))
        .take_while(|width| u64::from(*width) <= top)
        .collect()
}

/// The minutes of one IST day, as they streamed in, with what was wrong.
struct DayMinutes {
    minutes: Vec<Candle>,
    refusal: Option<DayRefusal>,
}

/// Builds one stock-month of the day input from its minutes, streaming them
/// once in order, and advances `state`.
///
/// # Errors
///
/// `widths_refused`; `month_refused` for a month with no civil days.
pub fn build_month(
    spec: &MonthSpec<'_>,
    minutes: &[Candle],
    cash: &CashCloses,
    authorities: &Authorities,
    state: &mut StockState,
) -> Result<MonthInput, String> {
    check_widths(spec.widths)?;
    let first = pull::session::Day::new(spec.year, spec.month, 1)
        .map_err(|why| format!("month_refused: {}-{:02}: {why}", spec.year, spec.month))?;
    let first_day = i64::from(first.days_from_epoch());
    let last_day = i64::from(first.end_of_month().days_from_epoch());
    let daily: std::collections::HashMap<i64, i64> = spec
        .daily
        .unwrap_or_default()
        .iter()
        .map(|bar| (ist_day_second(bar.ts_micros).0, bar.close))
        .collect();
    let mut stream = minutes.iter().peekable();
    let mut month = MonthInput {
        symbol: spec.symbol.to_owned(),
        series: spec.series,
        year: spec.year,
        month: spec.month,
        days: Vec::new(),
        excluded: Vec::new(),
        counts: MonthCounts {
            calendar: YearCounts::new(),
            refusals: YearCounts::new(),
            previous: YearCounts::new(),
            tradable_without_minutes: 0,
            admitted: 0,
            instrument_session_unchecked: 0,
            no_trades_after: 0,
            no_trade_minutes: 0,
            zero_volume_with_range: 0,
            outside_continuous: 0,
            suspect_wicks: 0,
            landing_only: 0,
        },
    };
    let mut day = first_day;
    while day <= last_day {
        let mut taken = DayMinutes {
            minutes: Vec::new(),
            refusal: None,
        };
        while let Some(bar) = stream.next_if(|bar| ist_day_second(bar.ts_micros).0 <= day) {
            take_minute(&mut taken, *bar, day);
        }
        if let Some(input) = build_day(
            spec,
            day,
            taken,
            cash,
            authorities,
            state,
            &daily,
            &mut month,
        ) {
            month.days.push(input);
        }
        day += 1;
    }
    Ok(month)
}

/// Takes one streamed minute into its day, naming a duplicate, a disorder or
/// a corrupt minute; a minute of an earlier day than `day` is a disorder of
/// `day` and is dropped.
fn take_minute(taken: &mut DayMinutes, bar: Candle, day: i64) {
    let bar_day = ist_day_second(bar.ts_micros).0;
    let refuse = |taken: &mut DayMinutes, why| {
        taken.refusal.get_or_insert(why);
    };
    if bar_day != day {
        refuse(taken, DayRefusal::Unsorted);
        return;
    }
    if let Some(previous) = taken.minutes.last() {
        if previous.ts_micros == bar.ts_micros {
            refuse(taken, DayRefusal::DuplicateMinute);
            return;
        }
        if previous.ts_micros > bar.ts_micros {
            refuse(taken, DayRefusal::Unsorted);
            return;
        }
    }
    let stored = bar_of(bar);
    let off_grid = bar.ts_micros.rem_euclid(micros_per_minute()) != 0;
    if !stored.ohlc_is_sane() || !stored.counts_are_sane() || off_grid {
        refuse(taken, DayRefusal::CorruptMinute);
    }
    taken.minutes.push(bar);
}

/// A candle as the store's bar.
fn bar_of(candle: Candle) -> Bar {
    Bar {
        ts_micros: candle.ts_micros,
        open: candle.open,
        high: candle.high,
        low: candle.low,
        close: candle.close,
        volume: candle.volume,
        open_interest: candle.open_interest,
    }
}

/// A store bar as a candle.
fn candle_of(bar: Bar) -> Candle {
    Candle {
        ts_micros: bar.ts_micros,
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.volume,
        open_interest: bar.open_interest,
    }
}

/// The minute-of-day of a minute.
fn minute_of(candle: &Candle) -> u16 {
    u16::try_from(ist_day_second(candle.ts_micros).1 / minute_seconds()).unwrap_or(u16::MAX)
}

/// One day of the month: its calendar verdict, its minutes judged, its
/// previous closes, its candles, its closes table. `None` for a day the
/// calendar excludes that holds no minute.
#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "one day's assembly from the month's shared state, in the order a \
              reader checks it"
)]
fn build_day(
    spec: &MonthSpec<'_>,
    day: i64,
    taken: DayMinutes,
    cash: &CashCloses,
    authorities: &Authorities,
    state: &mut StockState,
    daily: &std::collections::HashMap<i64, i64>,
    month: &mut MonthInput,
) -> Option<DayInput> {
    let year = year_of(day).unwrap_or(0);
    let tradable = authorities.calendar.tradable(spec.series, day);
    if let Err(reason) = tradable.market {
        month.excluded.push((day, reason));
        month.counts.calendar.add(year, reason.code.index());
    }
    let DayMinutes {
        minutes,
        mut refusal,
    } = taken;
    if minutes.is_empty() && refusal.is_none() {
        if tradable.admits() {
            month.counts.tradable_without_minutes += 1;
        }
        return None;
    }
    // Known at 09:15: both references' verdicts, before any minute is read.
    let basis = authorities.basis();
    let previous_session = verdict(
        state.session,
        day,
        |d| authorities.calendar.traded(d),
        |carried| basis.verdict(spec.series, carried.day, day),
    );
    let previous_tradable = verdict(
        state.tradable,
        day,
        |d| authorities.calendar.tradable(spec.series, d).admits(),
        |carried| basis.verdict(spec.series, carried.day, day),
    );
    let window = window_of(day, cash);
    if window.is_err() {
        refusal.get_or_insert(DayRefusal::SessionUnverified);
    }
    let window = window.ok();
    let mut counts = MinuteCounts::NONE;
    // The continuous minutes, each judged once: a padded no-trade minute is a
    // hole (FM-F095), a zero-volume minute with a range is kept.
    let mut real: Vec<Candle> = Vec::with_capacity(minutes.len());
    let mut previous_close: Option<i64> = None;
    for minute in &minutes {
        let inside = window.is_some_and(|window| window.holds(minute_of(minute)));
        if !inside {
            counts.outside_continuous += 1;
            continue;
        }
        let flat =
            minute.open == minute.high && minute.high == minute.low && minute.low == minute.close;
        let padded = minute.volume == 0
            && flat
            && previous_close == Some(minute.close)
            && minute_of(minute) != anchor_minute();
        if padded {
            counts.no_trade += 1;
        } else {
            if minute.volume == 0 && !flat {
                counts.zero_volume_with_range += 1;
            }
            counts.real += 1;
            real.push(*minute);
        }
        previous_close = Some(minute.close);
    }
    let last_close = real.last().map(|minute| minute.close);
    // The tail (FM-F094): a last minute closing before 15:09:00 is cut,
    // unwitnessed, or kept with no trades after it.
    let mut no_trades_after = false;
    if refusal.is_none()
        && let Some(last) = real.last()
    {
        let closes_at = ist_day_second(last.ts_micros).1 + minute_seconds();
        let forced = u32::try_from(SESSION_FORCED_WINDOW_START_SECS).unwrap_or(u32::MAX);
        if closes_at < forced {
            match authorities.tail.after(spec.series, day, closes_at) {
                TailReading::TradesAfter => refusal = Some(DayRefusal::TailCut),
                TailReading::Unknown => refusal = Some(DayRefusal::TailUnwitnessed),
                TailReading::NoTradesAfter => no_trades_after = true,
            }
        }
    }
    let reference = spec.reference;
    let deciding = match reference {
        PreviousReference::TradedSession => previous_session,
        PreviousReference::TradableDay => previous_tradable,
    };
    let closes = closes_table(&real, window);
    let sides = closes
        .iter()
        .map(|slot| match (slot, deciding.accepted()) {
            (MinuteClose::Real(close) | MinuteClose::Stale(close), Some(accepted)) => {
                Some(accepted.side(*close))
            }
            _ => None,
        })
        .collect();
    let admitted = tradable.admits() && refusal.is_none();
    // A halt the calendar removed in hindsight keeps its pre-halt candles as
    // the exposure input (FM-F100).
    let halt = tradable
        .market
        .err()
        .filter(|reason| {
            matches!(
                reason.code,
                CalendarCode::Halt | CalendarCode::SystemsOutage
            )
        })
        .and(pull::calendar::halt_on(day));
    let mut candles = Vec::new();
    let mut exposure = None;
    if let (Some(window), true) = (window, admitted || halt.is_some()) {
        match fold_widths(day, &real, window, spec.widths, cash) {
            Ok(folded) => {
                if let Some(halt) = halt {
                    let first_halted_second = halt.seconds(Venue::NseCash).from;
                    exposure = Some(HaltExposure {
                        first_halted_second,
                        candles: pre_halt(&folded, instant(day, first_halted_second)),
                    });
                } else {
                    candles = folded;
                }
            }
            Err(_) if admitted => refusal = Some(DayRefusal::FoldRefused),
            Err(_) => {}
        }
    }
    let admitted = tradable.admits() && refusal.is_none();
    // Wicks against the committed past, committed only on an admitted day.
    let mut suspect = Vec::new();
    if admitted {
        let mut widest = state.wick;
        for minute in &real {
            let (up, down) = wicks(minute);
            let slot = minute_of(minute).saturating_sub(anchor_minute());
            for (wick, upper, past, day_widest) in [
                (up, true, state.wick.up, &mut widest.up),
                (down, false, state.wick.down, &mut widest.down),
            ] {
                if let Some(past) = past
                    && minute.close > 0
                    && wider((wick, minute.close), past)
                {
                    suspect.push(SuspectWick { slot, upper });
                }
                if minute.close > 0
                    && day_widest.is_none_or(|seen| wider((wick, minute.close), seen))
                {
                    *day_widest = Some((wick, minute.close));
                }
            }
        }
        state.wick = widest;
        month.counts.admitted += 1;
        if no_trades_after {
            month.counts.no_trades_after += 1;
        }
    }
    if let Some(why) = refusal {
        month.counts.refusals.add(year, why.index());
    }
    if tradable.market.is_ok()
        && tradable.instrument == super::authorities::InstrumentSession::Unchecked
    {
        month.counts.instrument_session_unchecked += 1;
    }
    match deciding {
        PreviousClose::Refused(why, _) if admitted => month.counts.previous.add(year, why.index()),
        PreviousClose::Accepted(accepted) if admitted && accepted.landing_only => {
            month.counts.landing_only += 1;
        }
        _ => {}
    }
    month.counts.no_trade_minutes += u64::from(counts.no_trade);
    month.counts.zero_volume_with_range += u64::from(counts.zero_volume_with_range);
    month.counts.outside_continuous += u64::from(counts.outside_continuous);
    month.counts.suspect_wicks += u64::try_from(suspect.len()).unwrap_or(u64::MAX);
    // Carry the day's close forward: the traded-session reference from any
    // day with minutes, the tradable reference from an admitted day only.
    if let Some(close) = last_close {
        let carried = CarriedClose {
            close,
            day,
            landing: authorities.landing.landing(spec.series, day),
            regime_last_minute: window.map_or(0, |window| window.last),
            refused: refusal.is_some(),
        };
        state.session = Some(carried);
        if tradable.admits() {
            state.tradable = Some(carried);
        }
    } else if refusal.is_some() {
        // A refused day with no usable minute still refuses the next day's close.
        if let Some(carried) = state.session.as_mut() {
            carried.refused = true;
        }
    }
    let variant_1day_close = deciding
        .carried()
        .and_then(|carried| daily.get(&carried.day).copied());
    Some(DayInput {
        day,
        tradable,
        refusal,
        no_trades_after,
        window,
        counts,
        candles,
        exposure,
        closes,
        previous_session,
        previous_tradable,
        reference,
        sides,
        variant_1day_close,
        suspect,
        last_close,
        stamps: DayStamps {
            expiry: authorities.expiry.expiry(spec.series, day),
            fno_ban: FNO_BAN_ABSENT,
        },
    })
}

/// A minute's upper and lower wick, paisa.
fn wicks(minute: &Candle) -> (i64, i64) {
    let top = minute.open.max(minute.close);
    let bottom = minute.open.min(minute.close);
    (
        minute.high.saturating_sub(top).max(0),
        bottom.saturating_sub(minute.low).max(0),
    )
}

/// One reference's verdict on its carried close for `day` (FM-F092): from
/// the reference's previous day (`is_previous` names that day kind), not
/// refused, positive, and the basis proven.
fn verdict(
    carried: Option<CarriedClose>,
    day: i64,
    is_previous: impl Fn(i64) -> bool,
    basis: impl FnOnce(&CarriedClose) -> super::authorities::BasisVerdict,
) -> PreviousClose {
    let Some(carried) = carried else {
        return PreviousClose::Refused(CloseRefusal::NoPrevious, None);
    };
    // Adjacent when no day of the reference's kind lies strictly between:
    // a walk of the gap since the carried day, each day of it met once.
    let mut between = carried.day + 1;
    while between < day {
        if is_previous(between) {
            return PreviousClose::Refused(CloseRefusal::NotAdjacent, Some(carried));
        }
        between += 1;
    }
    if carried.day >= day {
        return PreviousClose::Refused(CloseRefusal::NotAdjacent, Some(carried));
    }
    if carried.refused {
        return PreviousClose::Refused(CloseRefusal::PreviousDayRefused, Some(carried));
    }
    if carried.close <= 0 {
        return PreviousClose::Refused(CloseRefusal::NotPositive, Some(carried));
    }
    let proven = basis(&carried);
    match proven.outcome {
        Ok(proof) => PreviousClose::Accepted(AcceptedClose {
            carried,
            proof,
            landing_only: proven.landing_only,
        }),
        Err(BasisRefusal::Unreliable) => {
            PreviousClose::Refused(CloseRefusal::BasisUnreliable, Some(carried))
        }
        Err(_) => PreviousClose::Refused(CloseRefusal::BasisUnproven, Some(carried)),
    }
}

/// The closes table 09:15..15:09: a real minute's close at its slot; after
/// the first real minute, a slot with none is STALE at the last real close
/// (a share's missing minute is a minute with no trade); before it, absent.
fn closes_table(real: &[Candle], window: Option<Window>) -> Vec<MinuteClose> {
    let slots = rank_slots();
    let mut out = vec![MinuteClose::Absent; slots];
    let Some(window) = window else {
        return out;
    };
    let mut stream = real.iter().peekable();
    let mut last: Option<i64> = None;
    for (slot, entry) in out.iter_mut().enumerate() {
        let minute = anchor_minute().saturating_add(u16::try_from(slot).unwrap_or(u16::MAX));
        if !window.holds(minute) {
            *entry = last.map_or(MinuteClose::Absent, MinuteClose::Stale);
            continue;
        }
        if let Some(bar) = stream.next_if(|bar| minute_of(bar) == minute) {
            last = Some(bar.close);
            *entry = MinuteClose::Real(bar.close);
        } else {
            *entry = last.map_or(MinuteClose::Absent, MinuteClose::Stale);
        }
    }
    out
}

/// Folds the day's real minutes to every width through the one fold
/// authority, and marks each candle's completeness.
///
/// # Errors
///
/// The fold authority's refusal, by name.
fn fold_widths(
    day: i64,
    real: &[Candle],
    window: Window,
    widths: &[u16],
    cash: &CashCloses,
) -> Result<Vec<WidthCandles>, String> {
    let bars: Vec<Bar> = real.iter().map(|minute| bar_of(*minute)).collect();
    // present[i]: real minutes among the window's first i minutes.
    let mut present = Vec::with_capacity(usize::from(window.minutes()) + 1);
    present.push(0_u32);
    let mut stream = real.iter().peekable();
    let mut count = 0_u32;
    for minute in window.first..=window.last {
        if stream.next_if(|bar| minute_of(bar) == minute).is_some() {
            count += 1;
        }
        present.push(count);
    }
    let day_open = instant(day, u32::from(window.first) * minute_seconds());
    let day_close = instant(day, (u32::from(window.last) + 1) * minute_seconds());
    let mut out = Vec::with_capacity(widths.len());
    for width in widths {
        let bucket = Bucket::of_secs(u32::from(*width) * minute_seconds())
            .ok_or_else(|| format!("fold_refused: width {width}"))?;
        let (folded, notes) = pull::fold::complete_minutes_with_calendar(
            &bars,
            bucket,
            Venue::NseCash,
            Some(cash.schedule()),
            pull::calendar::Runtime::default(),
        )
        .map_err(|why| format!("fold_refused: {why}"))?;
        let mut withheld = notes;
        let mut candles = Vec::with_capacity(folded.len());
        let width_micros = i64::from(*width) * micros_per_minute();
        for bar in folded {
            let start = bar.ts_micros;
            let end = (start + width_micros).min(day_close);
            let from = minutes_between(day_open, start.max(day_open));
            let to = minutes_between(day_open, end);
            let owed = to.saturating_sub(from);
            let have = present
                .get(to)
                .zip(present.get(from))
                .map_or(0, |(to, from)| to.saturating_sub(*from));
            let missing =
                u16::try_from(u32::try_from(owed).unwrap_or(u32::MAX).saturating_sub(have))
                    .unwrap_or(u16::MAX);
            let complete = missing == 0 && end - start == width_micros;
            if !complete {
                withheld.push(format!(
                    "bucket {} width {width}: {} of {owed} minutes, {}",
                    stamp(start),
                    have,
                    if missing > 0 {
                        "incomplete"
                    } else {
                        "partial last bucket"
                    }
                ));
            }
            candles.push(FibCandle {
                candle: candle_of(bar),
                tf_minutes: *width,
                complete,
                missing_minutes: missing,
                close_ts_micros: end,
            });
        }
        out.push(WidthCandles {
            width: *width,
            capacity: window.minutes().div_ceil(*width),
            candles,
            withheld,
        });
    }
    Ok(out)
}

/// Whole minutes from `from` to `to`.
fn minutes_between(from: i64, to: i64) -> usize {
    usize::try_from((to - from) / micros_per_minute()).unwrap_or(0)
}

/// An instant as `HH:MM` IST.
fn stamp(ts_micros: i64) -> String {
    let second = ist_day_second(ts_micros).1;
    let minutes = second / minute_seconds();
    // An hour holds as many minutes as a minute holds seconds.
    let per_hour = minute_seconds();
    format!("{:02}:{:02}", minutes / per_hour, minutes % per_hour)
}

/// Each width's candles that closed at or before `until`.
fn pre_halt(folded: &[WidthCandles], until: i64) -> Vec<WidthCandles> {
    folded
        .iter()
        .map(|width| WidthCandles {
            width: width.width,
            capacity: width.capacity,
            candles: width
                .candles
                .iter()
                .filter(|candle| candle.close_ts_micros <= until)
                .copied()
                .collect(),
            withheld: Vec::new(),
        })
        .collect()
}

// -------------------------------------------------------------- loading --

/// What loading one stock-month found (FM-F099).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MonthMinutes {
    /// The month's minutes.
    Minutes(Vec<Candle>),
    /// The month file is absent while the store root and the symbol's
    /// directory both exist: absence, counted.
    Absent,
    /// The symbol's directory is absent while the root exists:
    /// `store_key_absent`, counted.
    KeyAbsent,
}

/// A refusal that halts a run by name, resumable (FM-F099).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Halt {
    /// The word: `store_root_vanished`, `locked`, `io`, `corrupt`,
    /// `not_sweepable`.
    pub word: &'static str,
    /// What the store said.
    pub detail: String,
}

impl std::fmt::Display for Halt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.word, self.detail)
    }
}

/// The directory a symbol's month files of `rung` live in: the parent of
/// the month's own path.
fn symbol_dir(
    root: &Path,
    vendor: Vendor,
    symbol: &str,
    rung: &str,
    (year, month): (u16, u8),
) -> Option<PathBuf> {
    let key = stored::swept_index(symbol).ok()?;
    let timeframe = stored::rung(rung).ok()?;
    let month = store::path::YearMonth::new(year, month).ok()?;
    let path = store::path::StorePath::for_key(
        vendor,
        &key,
        timeframe,
        month,
        store::path::FileKind::Bars,
    )
    .ok()?
    .to_path_buf(root);
    path.parent().map(Path::to_path_buf)
}

/// Loads one stock-month through the classified door, statting the store
/// root and the symbol's directory first (FM-F099): absence counts only when
/// both exist; a vanished root, a lock, a host refusal and a corrupt month
/// halt by name.
///
/// # Errors
///
/// A [`Halt`].
pub fn load_month(
    root: &Path,
    vendor: Vendor,
    symbol: &str,
    rung: &str,
    (year, month): (u16, u8),
) -> Result<MonthMinutes, Halt> {
    if !root.is_dir() {
        return Err(Halt {
            word: "store_root_vanished",
            detail: format!("{} is not a directory", root.display()),
        });
    }
    match stored::load_with_class(root, vendor, symbol, rung, year, month) {
        Ok(loaded) => Ok(MonthMinutes::Minutes(loaded.bars)),
        Err((LoadClass::Missing, why)) => {
            if !root.is_dir() {
                return Err(Halt {
                    word: "store_root_vanished",
                    detail: why,
                });
            }
            let directory = symbol_dir(root, vendor, symbol, rung, (year, month));
            if directory.is_some_and(|directory| directory.is_dir()) {
                Ok(MonthMinutes::Absent)
            } else {
                Ok(MonthMinutes::KeyAbsent)
            }
        }
        Err((class, why)) => Err(Halt {
            word: class.word(),
            detail: why,
        }),
    }
}

// -------------------------------------------------------------- seconds --

/// One stock-day's GDFL seconds for stage DEEP: the continuous phase only,
/// sorted and unique, behind a dense per-second slot table. A missing second
/// is no value, never zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondsDay {
    /// The IST epoch day.
    pub day: i64,
    first_second: u32,
    slots: Vec<Option<u32>>,
    bars: Vec<Candle>,
}

impl SecondsDay {
    /// The second-of-day `second`'s bar, or `None`: no value. O(1): one
    /// slot read and one bar read.
    #[must_use]
    pub fn at(&self, second: u32) -> Option<&Candle> {
        let offset = usize::try_from(second.checked_sub(self.first_second)?).ok()?;
        let index = usize::try_from((*self.slots.get(offset)?)?).ok()?;
        self.bars.get(index)
    }

    /// Seconds held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Whether no second is held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

/// One stock-month's GDFL seconds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecondsMonth {
    /// Every day with seconds, in order.
    pub days: Vec<SecondsDay>,
    /// Days whose seconds were refused, by name.
    pub refused: Vec<(i64, &'static str)>,
    /// Days with Zerodha minutes and no GDFL second: `no_execution_seconds`,
    /// counted, deep stage only.
    pub no_execution_seconds: Vec<i64>,
    /// Seconds outside the continuous phase: counted, never used.
    pub outside_continuous: u64,
}

impl SecondsMonth {
    /// The seconds of `day`, if any were kept.
    #[must_use]
    pub fn day(&self, day: i64) -> Option<&SecondsDay> {
        self.days.iter().find(|held| held.day == day)
    }
}

/// The seconds of one day while they stream in.
struct SecondsTaken {
    day: i64,
    bars: Vec<Candle>,
    refusal: Option<&'static str>,
}

/// Builds a stock-month's seconds from its GDFL `1s` bars, streaming them
/// once: only seconds the calendar names continuous for the cash venue
/// (`pull::calendar::phase_at_second`, the share's dated flag from `cash`)
/// inside the day's window; a day with a duplicate, unsorted or off-second
/// bar is refused by name; every day of `minute_days` with no kept second is
/// `no_execution_seconds`.
#[must_use]
pub fn seconds_of(bars: &[Candle], minute_days: &[i64], cash: &CashCloses) -> SecondsMonth {
    let mut month = SecondsMonth {
        days: Vec::new(),
        refused: Vec::new(),
        no_execution_seconds: Vec::new(),
        outside_continuous: 0,
    };
    let mut taken: Option<SecondsTaken> = None;
    for bar in bars {
        let (day, second) = ist_day_second(bar.ts_micros);
        if taken.as_ref().is_some_and(|held| held.day != day) {
            if let Some(done) = taken.take() {
                finish_seconds(done, cash, &mut month);
            }
        }
        let held = taken.get_or_insert_with(|| SecondsTaken {
            day,
            bars: Vec::new(),
            refusal: None,
        });
        if held.refusal.is_some() {
            continue;
        }
        if bar.ts_micros.rem_euclid(micros_per_second()) != 0 {
            held.refusal = Some("seconds_corrupt");
            continue;
        }
        if let Some(previous) = held.bars.last() {
            if previous.ts_micros == bar.ts_micros {
                held.refusal = Some("seconds_duplicate");
                continue;
            }
            if previous.ts_micros > bar.ts_micros {
                held.refusal = Some("seconds_unsorted");
                continue;
            }
        }
        let continuous =
            pull::calendar::phase_at_second(Venue::NseCash, day, second, cas_of(day, cash))
                == Phase::Continuous;
        if continuous {
            held.bars.push(*bar);
        } else {
            month.outside_continuous += 1;
        }
    }
    if let Some(done) = taken {
        finish_seconds(done, cash, &mut month);
    }
    for day in minute_days {
        let kept = month.day(*day).is_some_and(|held| !held.is_empty());
        let refused = month.refused.iter().any(|(on, _)| on == day);
        if !kept && !refused {
            month.no_execution_seconds.push(*day);
        }
    }
    month
}

/// Closes one day of seconds: refused by name, or indexed into its dense
/// table over the day's window.
fn finish_seconds(taken: SecondsTaken, cash: &CashCloses, month: &mut SecondsMonth) {
    if let Some(why) = taken.refusal {
        month.refused.push((taken.day, why));
        return;
    }
    let Ok(window) = window_of(taken.day, cash) else {
        month.refused.push((taken.day, "session_unverified"));
        return;
    };
    let first_second = u32::from(window.first) * minute_seconds();
    let span = usize::try_from(u32::from(window.minutes()) * minute_seconds()).unwrap_or(0);
    let mut slots = vec![None; span];
    let mut bars = Vec::with_capacity(taken.bars.len());
    for bar in taken.bars {
        let second = ist_day_second(bar.ts_micros).1;
        let slot = second
            .checked_sub(first_second)
            .and_then(|offset| usize::try_from(offset).ok())
            .and_then(|offset| slots.get_mut(offset));
        if let Some(slot) = slot {
            *slot = u32::try_from(bars.len()).ok();
            bars.push(bar);
        } else {
            month.outside_continuous += 1;
        }
    }
    if bars.is_empty() {
        return;
    }
    month.days.push(SecondsDay {
        day: taken.day,
        first_second,
        slots,
        bars,
    });
}

/// Loads a stock-month's GDFL seconds through the classified door (FM-F099)
/// and indexes them; an absent month leaves every minute day
/// `no_execution_seconds`.
///
/// # Errors
///
/// A [`Halt`].
pub fn load_seconds(
    root: &Path,
    symbol: &str,
    when: (u16, u8),
    minute_days: &[i64],
    cash: &CashCloses,
) -> Result<SecondsMonth, Halt> {
    let bars = match load_month(root, Vendor::Gdfl, symbol, "1s", when)? {
        MonthMinutes::Minutes(bars) => bars,
        MonthMinutes::Absent | MonthMinutes::KeyAbsent => Vec::new(),
    };
    Ok(seconds_of(&bars, minute_days, cash))
}

// ------------------------------------------------------------ day-check --

/// `cli fib day-check SYM YYYY-MM --previous REF (--authorities interim |
/// --<kind> WORD ...) [--json]`: one stock-month's day input, its excluded
/// days and its refusal counts, read-only. The words are judged before the
/// store is looked for, so a misuse is a misuse wherever the store is.
pub(super) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let request = match parse(arguments) {
        Ok(request) => request,
        Err(why) => return crate::refuse(out, &why),
    };
    let root = match crate::store_root() {
        Ok(root) => root,
        Err(why) => return crate::fail(out, &why),
    };
    let present = |name: &str| {
        symbol_dir(&root, Vendor::Zerodha, name, "1min", request.when)
            .is_some_and(|dir| dir.is_dir())
    };
    match Authorities::interim(&present) {
        Ok(authorities) => run(&root, &request, &authorities, out),
        Err(why) => crate::fail(out, &why),
    }
}

/// A `YYYY-MM` word.
fn year_month(word: &str) -> Option<(u16, u8)> {
    let (year, month) = word.split_once('-')?;
    let year: u16 = year.parse().ok()?;
    let month: u8 = month.parse().ok()?;
    pull::session::Day::new(year, month, 1).ok()?;
    Some((year, month))
}

/// The month before `(year, month)`.
fn month_before((year, month): (u16, u8)) -> Option<(u16, u8)> {
    let day = pull::session::Day::new(year, month, 1).ok()?;
    let before = pull::session::Day::from_days(day.days_from_epoch().checked_sub(1)?).ok()?;
    Some((before.year(), before.month()))
}

/// What `day-check` was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    /// The canonical symbol.
    pub symbol: String,
    /// The month.
    pub when: (u16, u8),
    /// The deciding reference.
    pub reference: PreviousReference,
    /// Each authority's flag word.
    pub words: Vec<(super::authorities::AuthorityKind, String)>,
    /// JSON instead of text.
    pub json: bool,
}

/// Reads `day-check`'s words.
///
/// # Errors
///
/// A misuse by name: a missing or malformed month, a missing reference or
/// authority, a word that names no swept share.
pub fn parse(arguments: &[&str]) -> Result<Request, String> {
    let positional: Vec<&str> = positional_words(arguments);
    let [symbol, month] = positional.as_slice() else {
        return Err(
            "`fib day-check` takes SYM YYYY-MM --previous traded-session|tradable-day \
                    and the authorities"
                .to_owned(),
        );
    };
    let when = year_month(month).ok_or_else(|| format!("`{month}` is not a YYYY-MM month"))?;
    let words = super::authorities::words_of(arguments)?;
    let reference = PreviousReference::of(flag_value(arguments, "--previous"))?;
    let key = stored::swept_index(symbol)?;
    if key.kind != brutex_core::instrument::Kind::Equity {
        return Err(format!(
            "`{symbol}` is an index: the FIB day input is for the F&O shares; indices are \
             reference stamps (D-6443)"
        ));
    }
    Ok(Request {
        symbol: key.underlying.as_str().to_owned(),
        when,
        reference,
        words,
        json: arguments.contains(&"--json"),
    })
}

/// [`command`] against an explicit store root and authority set.
pub fn run(root: &Path, request: &Request, authorities: &Authorities, out: &mut String) -> u8 {
    let first = pull::session::Day::new(request.when.0, request.when.1, 1)
        .map_or(0, |day| i64::from(day.days_from_epoch()));
    let Some(series) = authorities
        .pool
        .member_named(&request.symbol, first)
        .map(|member| member.series_id)
    else {
        return crate::fail(
            out,
            &format!(
                "not_in_pool: {} is not a member of {}",
                request.symbol,
                authorities.pool.version().label
            ),
        );
    };
    match day_check(
        root,
        &request.symbol,
        series,
        request.when,
        request.reference,
        authorities,
    ) {
        Ok(report) => {
            if request.json {
                render_json(&report, &request.words, out);
            } else {
                render_text(&report, &request.words, out);
            }
            crate::OK
        }
        Err(halt) => crate::fail(out, &halt.to_string()),
    }
}

/// The words that are not flags or flag values.
fn positional_words<'a>(arguments: &[&'a str]) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut skip = false;
    for word in arguments {
        if skip {
            skip = false;
        } else if *word == "--json" {
        } else if word.starts_with("--") {
            skip = true;
        } else {
            out.push(*word);
        }
    }
    out
}

/// The value after `flag`.
fn flag_value<'a>(arguments: &[&'a str], flag: &str) -> Option<&'a str> {
    arguments
        .iter()
        .position(|word| *word == flag)
        .and_then(|at| arguments.get(at + 1))
        .copied()
}

/// What `day-check` read and built.
#[derive(Clone, Debug)]
pub struct DayCheck {
    /// The month.
    pub month: MonthInput,
    /// The widths folded.
    pub widths: Vec<u16>,
    /// Whether the previous month was read for the carried state.
    pub previous_month: &'static str,
    /// What the month load found.
    pub loaded: &'static str,
    /// The authorities' versions.
    pub versions: Vec<super::authorities::AuthorityVersion>,
    /// The deciding reference.
    pub reference: PreviousReference,
    /// The dated cash closes' digest.
    pub cash_digest: [u8; vocab::fib_cube::DIGEST_BYTES],
}

/// Reads the previous month (for the carried state) and the month, and
/// builds the day input.
///
/// # Errors
///
/// A [`Halt`] from either load.
pub fn day_check(
    root: &Path,
    symbol: &str,
    series: SeriesId,
    when: (u16, u8),
    reference: PreviousReference,
    authorities: &Authorities,
) -> Result<DayCheck, Halt> {
    let widths = doubling_ladder();
    let before = month_before(when);
    let previous_minutes = match before {
        Some(before) => load_month(root, Vendor::Zerodha, symbol, "1min", before)?,
        None => MonthMinutes::Absent,
    };
    let minutes = load_month(root, Vendor::Zerodha, symbol, "1min", when)?;
    let daily: Vec<Candle> = [before, Some(when)]
        .into_iter()
        .flatten()
        .filter_map(|(year, month)| {
            stored::load(root, Vendor::Zerodha, symbol, "1day", year, month).ok()
        })
        .flat_map(|loaded| loaded.bars)
        .collect();
    let days = |minutes: &MonthMinutes| -> Vec<i64> {
        match minutes {
            MonthMinutes::Minutes(bars) => bars
                .iter()
                .map(|bar| ist_day_second(bar.ts_micros).0)
                .collect(),
            MonthMinutes::Absent | MonthMinutes::KeyAbsent => Vec::new(),
        }
    };
    let mut all_days = days(&previous_minutes);
    all_days.extend(days(&minutes));
    let cash = stored::load_cash_closes(root, symbol, all_days).map_err(|why| Halt {
        word: "cash_closes",
        detail: why,
    })?;
    let mut state = StockState::first_day();
    let previous_month = match (&previous_minutes, before) {
        (MonthMinutes::Minutes(bars), Some((year, month))) => {
            let spec = MonthSpec {
                symbol,
                series,
                year,
                month,
                widths: &widths,
                reference,
                daily: Some(&daily),
            };
            build_month(&spec, bars, &cash, authorities, &mut state).map_err(|why| Halt {
                word: "month_refused",
                detail: why,
            })?;
            "read for the carried close"
        }
        _ => "absent: the month's first day has no previous close",
    };
    let (bars, loaded) = match &minutes {
        MonthMinutes::Minutes(bars) => (bars.as_slice(), "minutes"),
        MonthMinutes::Absent => (&[][..], "absent"),
        MonthMinutes::KeyAbsent => (&[][..], "store_key_absent"),
    };
    let spec = MonthSpec {
        symbol,
        series,
        year: when.0,
        month: when.1,
        widths: &widths,
        reference,
        daily: Some(&daily),
    };
    let month = build_month(&spec, bars, &cash, authorities, &mut state).map_err(|why| Halt {
        word: "month_refused",
        detail: why,
    })?;
    Ok(DayCheck {
        month,
        widths,
        previous_month,
        loaded,
        versions: authorities.versions(),
        reference,
        cash_digest: cash.digest(),
    })
}

/// Hex of a digest's first bytes.
fn short_hex(digest: &[u8]) -> String {
    digest
        .iter()
        .take(size_of::<u64>())
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// One width's candles as `width:complete/folded/capacity`.
fn width_line(width: &WidthCandles) -> String {
    let complete = width
        .candles
        .iter()
        .filter(|candle| candle.complete)
        .count();
    format!(
        "{}:{complete}/{}/{}",
        width.width,
        width.candles.len(),
        width.capacity
    )
}

/// The day-check page.
fn render_text(
    report: &DayCheck,
    words: &[(super::authorities::AuthorityKind, String)],
    out: &mut String,
) {
    let month = &report.month;
    let _ = writeln!(
        out,
        "fib day-check {} {}-{:02}: Zerodha 1min through the stored door ({}), folded in-run \
         from 09:15 (D-6440); GDFL never read for candles, the move or the rank",
        month.symbol, month.year, month.month, report.loaded
    );
    let _ = writeln!(
        out,
        "research: equities are labelled research, gross of every charge until FIB-11 \
         (CLAUDE.md §1); never Selection V6 or execution authority"
    );
    for (version, (_, word)) in report.versions.iter().zip(words) {
        let _ = writeln!(
            out,
            "authority {} = {word} [{}{}] {}",
            version.kind.name(),
            short_hex(&version.digest),
            if version.interim {
                ", interim: refused for --out-store"
            } else {
                ""
            },
            version.label
        );
    }
    let widths: Vec<String> = report.widths.iter().map(u16::to_string).collect();
    let _ = writeln!(
        out,
        "widths: {} (R7's coarse doubling ladder up to {}); a bucket a minute is missing from \
         is withheld by name",
        widths.join(","),
        max_width_minutes()
    );
    let _ = writeln!(
        out,
        "previous close: deciding reference {}; both references computed and labelled (ZD-41); \
         previous month {}; cash closes {}",
        report.reference.word(),
        report.previous_month,
        short_hex(&report.cash_digest)
    );
    for day in &month.days {
        let verdict = match day.tradable.market {
            Ok(()) => day.tradable.instrument.word().to_owned(),
            Err(reason) => format!("excluded {reason}"),
        };
        let candles: Vec<String> = day.candles.iter().map(width_line).collect();
        let _ = writeln!(
            out,
            "{} {verdict}; minutes {} (no-trade {}, zero-volume ranged {}, outside {}); {}; \
             previous {} / {} (session / tradable){}; move {}; candles {}; suspect wicks {}; \
             expiry {:?}; ban {}{}",
            date(day.day),
            day.counts.real,
            day.counts.no_trade,
            day.counts.zero_volume_with_range,
            day.counts.outside_continuous,
            day.refusal.map_or_else(
                || if day.no_trades_after {
                    "kept: no_trades_after".to_owned()
                } else {
                    "kept".to_owned()
                },
                |why| format!("refused {}", why.name())
            ),
            day.previous_session.word(),
            day.previous_tradable.word(),
            day.previous()
                .carried()
                .map_or_else(String::new, |carried| format!(
                    " from {} close {} regime {}",
                    date(carried.day),
                    carried.close,
                    carried.regime_last_minute
                )),
            day.candidate_refusal()
                .unwrap_or_else(|| "known".to_owned()),
            candles.join(" "),
            day.suspect.len(),
            day.stamps.expiry,
            day.stamps.fno_ban,
            day.exposure
                .as_ref()
                .map_or_else(String::new, |exposure| format!(
                    "; halt exposure before second {}: {}",
                    exposure.first_halted_second,
                    exposure
                        .candles
                        .iter()
                        .map(width_line)
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
        );
    }
    let excluded: Vec<String> = month
        .excluded
        .iter()
        .map(|(day, reason)| format!("{} {reason}", date(*day)))
        .collect();
    let _ = writeln!(
        out,
        "excluded days: {}",
        if excluded.is_empty() {
            "none".to_owned()
        } else {
            excluded.join("; ")
        }
    );
    render_counts(month, out);
}

/// The month's counts, per year and reason.
fn render_counts(month: &MonthInput, out: &mut String) {
    let c = &month.counts;
    for (year, counts) in c.calendar.years() {
        for (code, count) in CalendarCode::ALL.iter().zip(counts) {
            if *count > 0 {
                let _ = writeln!(out, "count {year} calendar {} {count}", code.name());
            }
        }
    }
    for (year, counts) in c.refusals.years() {
        for (why, count) in DayRefusal::ALL.iter().zip(counts) {
            if *count > 0 {
                let _ = writeln!(out, "count {year} refused {} {count}", why.name());
            }
        }
    }
    for (year, counts) in c.previous.years() {
        for (why, count) in CloseRefusal::ALL.iter().zip(counts) {
            if *count > 0 {
                let _ = writeln!(out, "count {year} previous_close {} {count}", why.name());
            }
        }
    }
    let _ = writeln!(
        out,
        "count admitted {}; tradable without minutes {}; instrument_session_unchecked {}; \
         no_trades_after {}; no_trade_minutes {}; zero_volume_with_range {}; \
         outside_continuous {}; suspect_wicks {}; landing_only {}",
        c.admitted,
        c.tradable_without_minutes,
        c.instrument_session_unchecked,
        c.no_trades_after,
        c.no_trade_minutes,
        c.zero_volume_with_range,
        c.outside_continuous,
        c.suspect_wicks,
        c.landing_only
    );
}

/// A JSON string literal.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 1 + 1);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The day-check as one JSON object.
fn render_json(
    report: &DayCheck,
    words: &[(super::authorities::AuthorityKind, String)],
    out: &mut String,
) {
    let month = &report.month;
    let authorities: Vec<String> = report
        .versions
        .iter()
        .zip(words)
        .map(|(version, (_, word))| {
            format!(
                "{{\"kind\":{},\"word\":{},\"label\":{},\"digest\":{},\"interim\":{}}}",
                json_string(version.kind.name()),
                json_string(word),
                json_string(version.label),
                json_string(&short_hex(&version.digest)),
                version.interim
            )
        })
        .collect();
    let days: Vec<String> = month
        .days
        .iter()
        .map(|day| {
            let candles: Vec<String> = day
                .candles
                .iter()
                .map(|width| json_string(&width_line(width)))
                .collect();
            format!(
                "{{\"day\":{},\"tradable\":{},\"excluded\":{},\"minutes\":{},\"no_trade\":{},\
                 \"refused\":{},\"no_trades_after\":{},\"previous_session\":{},\
                 \"previous_tradable\":{},\"candidate_refusal\":{},\"candles\":[{}],\
                 \"suspect_wicks\":{},\"halt_exposure\":{}}}",
                json_string(&date(day.day)),
                day.tradable.admits(),
                day.tradable.market.err().map_or_else(
                    || "null".to_owned(),
                    |reason| json_string(&reason.to_string())
                ),
                day.counts.real,
                day.counts.no_trade,
                day.refusal
                    .map_or_else(|| "null".to_owned(), |why| json_string(why.name())),
                day.no_trades_after,
                json_string(day.previous_session.word()),
                json_string(day.previous_tradable.word()),
                day.candidate_refusal()
                    .map_or_else(|| "null".to_owned(), |why| json_string(&why)),
                candles.join(","),
                day.suspect.len(),
                day.exposure.is_some()
            )
        })
        .collect();
    let excluded: Vec<String> = month
        .excluded
        .iter()
        .map(|(day, reason)| {
            format!(
                "{{\"day\":{},\"reason\":{},\"known_in_advance\":{}}}",
                json_string(&date(*day)),
                json_string(reason.code.name()),
                reason.known_in_advance
            )
        })
        .collect();
    let mut counts = String::new();
    render_counts(month, &mut counts);
    let widths: Vec<String> = report.widths.iter().map(u16::to_string).collect();
    let _ = writeln!(
        out,
        "{{\"symbol\":{},\"month\":{},\"loaded\":{},\"research\":true,\"reference\":{},\
         \"widths\":[{}],\"authorities\":[{}],\"days\":[{}],\"excluded\":[{}],\"counts\":{}}}",
        json_string(&month.symbol),
        json_string(&format!("{}-{:02}", month.year, month.month)),
        json_string(report.loaded),
        json_string(report.reference.word()),
        widths.join(","),
        authorities.join(","),
        days.join(","),
        excluded.join(","),
        json_string(counts.trim_end())
    );
}

#[cfg(test)]
#[path = "day_tests.rs"]
mod tests;
