//! Real bars, out of the store, in the shape the sweep takes.
//!
//! # The gap this closes
//!
//! `crates/api` declared `store` and not `runner`; `crates/cli` declared
//! `runner` and not `store`. **No crate in this workspace could do both**, so
//! nothing connected a pulled bar to a ranked result — `cli sweep` has only ever
//! swept `runner::synthetic`, and said so in its own banner. `CLAUDE.md` §5 calls
//! that the live gap. This module is the join.
//!
//! # Why the conversion is a struct literal and not a trait
//!
//! `store::format::Bar` and `indicators::Candle` carry the same seven fields
//! with the same names and the same types — `ts_micros`, `open`, `high`, `low`,
//! `close`, `volume`, `open_interest`. They are two names for one record because
//! the crates that own them may not name each other: gate 22 pins `indicators`
//! to `vocab` alone, so it cannot depend on `store`, and a shared type would
//! have to live in a crate both could see. Copying seven fields here is the
//! price of that boundary, and it is the boundary that keeps a bar from reaching
//! the sweep by accident.
//!
//! # What arrives with the bars, and why it matters
//!
//! The store path IS `bars/<vendor>/<exchange>/<segment>/<symbol>/<rung>/<month>`,
//! so a load knows its **vendor, instrument, timeframe and month** before it
//! reads a byte. Those are four of the nine terms `runner::identity` needs, and
//! three of them were the reason a run identity could not be recorded: with
//! synthetic bars there is no instrument to name and naming one would be an
//! invention `CLAUDE.md` §3 rule 1 forbids. A stored load has them all.

use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::vendor::Vendor;
use indicators::Candle;
use indicators::anchored::{DailyEligibility, DailyReference};
use indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS;
use pull::calendar::{DayKind, MAX_WINDOWS, Session};
use std::path::Path;
use store::file::{BarFile, StoreError};
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

/// Version of the rule that keeps charter non-regular sessions out of a swept
/// series.
///
/// # What policy 1 means
///
/// Every bar whose IST day appears in
/// [`indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS`] is dropped as the
/// record is decoded, on **every rung except `1day`** — so the signal series,
/// the exact one-minute `GapFib` context and the one-minute execution path are
/// filtered by one act and cannot disagree about which sessions a run saw. The
/// nine days are the sole source; no session length is measured and no exchange
/// fact is derived, which is what `CLAUDE.md` §3 rule 1 requires.
///
/// # Why `1day` is deliberately exempt
///
/// The daily stream is not swept — it is the previous-day anchor evidence, and
/// [`daily_eligibility_of`] already refuses those records as anchors by marking
/// them [`DailyEligibility::Excluded`]. Dropping them here instead would delete
/// the one place the exclusion is already COUNTED: `daily_reference_note`
/// prints `eligible E, explicitly excluded X` off those bytes, and a filtered
/// daily stream would report `X = 0` on a span that had excluded two days.
///
/// # Why it is bound into the run identity
///
/// The same argument [`DAILY_ELIGIBILITY_POLICY`] carries. Two runs over one
/// span, one that swept a disaster-recovery Saturday and one that did not, are
/// different computations: they fold different bars, produce different masks
/// and can produce different frontiers. Binding the VERSION rather than only
/// the resulting bars re-keys even a span that happens to contain none of the
/// nine days, so a result recorded before this policy existed can never be
/// handed back as the answer to a question asked under it.
pub const SWEPT_SERIES_CALENDAR_POLICY: u32 = 1;

/// Which charter non-regular sessions one load removed, and what they cost.
///
/// # A fixed-size set, and no allocation
///
/// One slot per entry of [`CHARTER_NON_REGULAR_IST_DAYS`], in that array's
/// order, so the same day met twice in two months cannot be counted as two days
/// and the count needs neither a set nor an ordering assumption about the bars.
/// The array is nine long and is checked by a const assertion in
/// `crates/indicators`, so this type's width is fixed at compile time.
///
/// # Cost
///
/// [`Self::excludes`] walks at most nine `i64` comparisons, a compile-time
/// bound, and is called once per decoded record. It is none of the five
/// operations `CLAUDE.md` §3 rule 4 bounds; the loader it sits in is already
/// O(records) and this multiplies that by a constant.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace times it, so
/// the shape above is read from the source rather than measured. §3 rule 6.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CalendarExclusion {
    /// Whether the day at this slot of [`CHARTER_NON_REGULAR_IST_DAYS`] was met.
    removed: [bool; CHARTER_NON_REGULAR_IST_DAYS.len()],
    /// Bars dropped, summed across every removed day.
    bars: u64,
}

impl CalendarExclusion {
    /// Nothing was removed, because nothing was filtered or nothing matched.
    ///
    /// The two are deliberately one value here: a `1day` load and an intraday
    /// load over a clean span both removed nothing, and the caller that
    /// distinguishes them is the one that chose the rung.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            removed: [false; CHARTER_NON_REGULAR_IST_DAYS.len()],
            bars: 0,
        }
    }

    /// Should this bar be kept out of the swept series, and record it if so.
    fn excludes(&mut self, ts_micros: i64) -> bool {
        let day = indicators::ist_day(ts_micros);
        let Some(slot) = CHARTER_NON_REGULAR_IST_DAYS
            .iter()
            .position(|named| *named == day)
        else {
            return false;
        };
        if let Some(seen) = self.removed.get_mut(slot) {
            *seen = true;
        }
        self.bars = self.bars.saturating_add(1);
        true
    }

    /// Fold another load's removals into this one.
    fn absorb(&mut self, other: Self) {
        for (mine, theirs) in self.removed.iter_mut().zip(other.removed) {
            *mine |= theirs;
        }
        self.bars = self.bars.saturating_add(other.bars);
    }

    /// How many distinct charter days were actually removed.
    #[must_use]
    pub fn days(self) -> u32 {
        u32::try_from(self.removed.iter().filter(|seen| **seen).count()).unwrap_or(u32::MAX)
    }

    /// How many bars were removed, across every removed day.
    #[must_use]
    pub const fn bars(self) -> u64 {
        self.bars
    }

    /// Whether this load removed nothing at all.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.bars == 0 && self.days() == 0
    }

    /// The removed days themselves, as IST day numbers, in charter order.
    ///
    /// Named and not merely counted: an operator reading "two days were removed"
    /// cannot check that against the charter, and `CLAUDE.md` §3 rule 2 is about
    /// a scope change being visible rather than merely tallied.
    #[must_use]
    pub fn day_numbers(self) -> Vec<i64> {
        CHARTER_NON_REGULAR_IST_DAYS
            .iter()
            .zip(self.removed)
            .filter_map(|(day, seen)| seen.then_some(*day))
            .collect()
    }

    /// The removed days as `YYYY-MM-DD`, in charter order.
    ///
    /// # Why a date and not the day number
    ///
    /// An operator checking a withheld session against `docs/00-charter.md`
    /// reads dates; `19_784` is checkable only by someone who already knows the
    /// answer. The number remains available through [`Self::day_numbers`] for
    /// the identity and the tests, which want the exact bound value.
    ///
    /// A day that will not convert falls back to its raw number rather than
    /// being dropped or named wrongly. Every charter entry converts today, so
    /// the fallback is unreachable in practice and is here because silently
    /// omitting a withheld day would be the invisible scope change §3 rule 2
    /// forbids.
    #[must_use]
    pub fn day_names(self) -> Vec<String> {
        self.day_numbers()
            .into_iter()
            .map(|day| {
                u32::try_from(day)
                    .ok()
                    .and_then(|days| pull::session::Day::from_days(days).ok())
                    .map_or_else(|| day.to_string(), |date| date.to_string())
            })
            .collect()
    }
}

/// One instrument-month, and everything the store knew about it.
///
/// The fields are the provenance a report needs. They are returned rather than
/// logged because a caller that renders a figure has to be able to say where the
/// figure came from, and a log line cannot be put in a table.
#[derive(Debug, Clone)]
pub struct Loaded {
    /// The bars, oldest first, exactly as the file holds them — less any the
    /// charter's non-regular sessions supplied. See [`Self::excluded`].
    pub bars: Vec<Candle>,
    /// Which feed wrote them. The first path segment, never inferred.
    pub vendor: Vendor,
    /// What they are bars of.
    pub key: InstrumentKey,
    /// The rung, as its canonical directory word — `1min`, `1day`.
    pub timeframe: &'static str,
    /// Which charter non-regular sessions this load withheld, and how many bars
    /// that cost. Always [`CalendarExclusion::none`] on `1day`.
    pub excluded: CalendarExclusion,
}

/// Why a load could not happen, in the operator's words.
///
/// A `String` at the public boundary, deliberately: every arm here is a sentence
/// a person reads once and acts on. The range loader keeps only the one internal
/// distinction it needs — absent versus unsafe — and never exposes that as an
/// invitation for callers to recover from corruption.
pub type Refusal = String;

/// Schema version of [`CalendarReceiptV1`].
pub const CALENDAR_RECEIPT_SCHEMA_V1: u32 = 1;

/// Policy version binding the meaning of the charter calendar receipt.
///
/// Policy 1 means exact one-minute timestamps, IST day/minute conversion,
/// [`pull::calendar::kind_of`] as the sole session authority, an inclusive walk
/// of every day from the first requested day through the last, and fail-closed
/// refusal of timestamps the measured calendar says could not trade.
pub const CALENDAR_RECEIPT_POLICY_V1: u32 = 1;

const MICROS_PER_MINUTE_V1: i64 = 60_000_000;
const MICROS_PER_DAY_V1: i64 = 86_400_000_000;

/// Whether the exact timestamp slice reconciles with the measured calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarStatusV1 {
    /// Every minute in every measured open window was offered exactly once.
    Complete,
    /// At least one minute in a measured open window was absent.
    Incomplete,
    /// At least one day had no measured session length or lay outside the
    /// calendar's measured range, so completeness cannot be claimed.
    Unmeasured,
}

impl CalendarStatusV1 {
    const fn code(self) -> u8 {
        match self {
            Self::Complete => 1,
            Self::Incomplete => 2,
            Self::Unmeasured => 3,
        }
    }
}

/// Calendar attestation for one exact, ordered execution-timestamp slice.
///
/// The digest binds the schema and policy versions, every offered timestamp and
/// its membership decision, every day in the inclusive span, every measured
/// session window, and the final counts/status. It therefore changes when the
/// location of a hole changes even if all counts remain equal.
///
/// Fields are exposed through accessors rather than publicly writable slots so
/// no caller can mutate a count while retaining the old digest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarReceiptV1 {
    schema_version: u32,
    policy_version: u32,
    first_day: i64,
    last_day: i64,
    offered: u64,
    expected: u64,
    missing: u64,
    unexpected: u64,
    status: CalendarStatusV1,
    digest: [u8; 32],
}

impl CalendarReceiptV1 {
    /// Receipt schema version.
    #[must_use]
    pub const fn schema_version(self) -> u32 {
        self.schema_version
    }

    /// Calendar interpretation policy version.
    #[must_use]
    pub const fn policy_version(self) -> u32 {
        self.policy_version
    }

    /// First requested IST day, inclusive.
    #[must_use]
    pub const fn first_day(self) -> i64 {
        self.first_day
    }

    /// Last requested IST day, inclusive.
    #[must_use]
    pub const fn last_day(self) -> i64 {
        self.last_day
    }

    /// Exact one-minute timestamps offered by the caller.
    #[must_use]
    pub const fn offered(self) -> u64 {
        self.offered
    }

    /// Minutes owed by measured open sessions in the inclusive day span.
    #[must_use]
    pub const fn expected(self) -> u64 {
        self.expected
    }

    /// Owed measured minutes absent from the offered slice.
    #[must_use]
    pub const fn missing(self) -> u64 {
        self.missing
    }

    /// Offered timestamps outside a measured session.
    ///
    /// This is zero for every constructed V1 receipt: such a timestamp is a
    /// refusal rather than a receipt. The explicit field keeps that admission
    /// fact in the versioned schema and digest.
    #[must_use]
    pub const fn unexpected(self) -> u64 {
        self.unexpected
    }

    /// Completeness conclusion.
    #[must_use]
    pub const fn status(self) -> CalendarStatusV1 {
        self.status
    }

    /// Full BLAKE3 over the calendar and timestamp-membership decision.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Clone, Copy)]
struct OfferedCalendarFactsV1 {
    offered: u64,
    measured_offered: u64,
}

#[derive(Clone, Copy)]
struct CalendarDayFactsV1 {
    expected: u64,
    unmeasured: bool,
}

/// Exact IST day and minute for an exact UTC-minute timestamp.
fn exact_ist_minute_v1(ts_micros: i64) -> Result<(i64, u16), Refusal> {
    if ts_micros.rem_euclid(MICROS_PER_MINUTE_V1) != 0 {
        return Err(format!(
            "calendar receipt timestamp {ts_micros} is off the exact one-minute grid"
        ));
    }
    let shifted = ts_micros
        .checked_add(indicators::IST_OFFSET_MICROS)
        .ok_or_else(|| {
            format!(
                "calendar receipt timestamp {ts_micros} cannot be shifted to IST without overflow"
            )
        })?;
    let day = shifted.div_euclid(MICROS_PER_DAY_V1);
    let minute = u16::try_from(shifted.rem_euclid(MICROS_PER_DAY_V1) / MICROS_PER_MINUTE_V1)
        .map_err(|_| format!("calendar receipt timestamp {ts_micros} has no minute-of-day"))?;
    Ok((day, minute))
}

/// Validate and hash every offered timestamp without allocating a membership set.
fn hash_offered_calendar_v1(
    timestamps: &[i64],
    first_day: i64,
    last_day: i64,
    hasher: &mut brutex_core::blake3::Hasher,
) -> Result<OfferedCalendarFactsV1, Refusal> {
    let mut previous: Option<i64> = None;
    let mut offered = 0_u64;
    let mut measured_offered = 0_u64;

    for (index, &timestamp) in timestamps.iter().enumerate() {
        let (day, minute) = exact_ist_minute_v1(timestamp)?;
        if let Some(prior) = previous
            && timestamp <= prior
        {
            let fault = if timestamp == prior {
                "duplicate"
            } else {
                "backward"
            };
            return Err(format!(
                "calendar receipt timestamp {index} is {fault}: {prior} then {timestamp}"
            ));
        }
        if day < first_day || day > last_day {
            return Err(format!(
                "calendar receipt timestamp {timestamp} belongs to IST day {day}, outside requested inclusive span {first_day}..={last_day}"
            ));
        }

        let decision = match pull::calendar::kind_of(day) {
            DayKind::Open(session) if session.expects(minute) => {
                measured_offered = measured_offered.checked_add(1).ok_or_else(|| {
                    "calendar receipt measured-offered count overflowed u64".to_owned()
                })?;
                1_u8
            }
            DayKind::Open(_) => {
                return Err(format!(
                    "calendar receipt timestamp {timestamp} is minute {minute} on measured open IST day {day}, but it is outside every measured session window"
                ));
            }
            DayKind::Closed => {
                return Err(format!(
                    "calendar receipt timestamp {timestamp} is on measured closed IST day {day}"
                ));
            }
            DayKind::OpenLengthUnmeasured => 3,
            DayKind::Unmeasured => 4,
        };

        offered = offered
            .checked_add(1)
            .ok_or_else(|| "calendar receipt offered count overflowed u64".to_owned())?;
        let ordinal = u64::try_from(index)
            .map_err(|_| "calendar receipt timestamp index does not fit u64".to_owned())?;
        hasher.update(b"T");
        hasher.update(&ordinal.to_le_bytes());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&day.to_le_bytes());
        hasher.update(&minute.to_le_bytes());
        hasher.update(&[decision]);
        previous = Some(timestamp);
    }

    Ok(OfferedCalendarFactsV1 {
        offered,
        measured_offered,
    })
}

/// Hash one measured session canonically and return how many minutes it owes.
fn hash_open_session_v1(
    day: i64,
    session: Session,
    hasher: &mut brutex_core::blake3::Hasher,
) -> Result<u64, Refusal> {
    let count = usize::from(session.count);
    if count == 0 || count > MAX_WINDOWS {
        return Err(format!(
            "calendar policy produced {count} windows for open IST day {day}; V1 supports 1..={MAX_WINDOWS}"
        ));
    }
    hasher.update(&[session.count]);
    let mut expected = 0_u64;
    let mut prior_to = None;
    for index in 0..count {
        let window = session.windows.get(index).copied().ok_or_else(|| {
            format!("calendar policy omitted window {index} for open IST day {day}")
        })?;
        if window.from > window.to || window.to >= 1_440 {
            return Err(format!(
                "calendar policy window {index} on IST day {day} is invalid: {}..={} is not an ordered minute-of-day range",
                window.from, window.to
            ));
        }
        if prior_to.is_some_and(|prior| window.from <= prior) {
            return Err(format!(
                "calendar policy windows overlap or run backward on IST day {day} at window {index}"
            ));
        }
        let bars = window
            .to
            .checked_sub(window.from)
            .and_then(|width| width.checked_add(1))
            .ok_or_else(|| format!("calendar policy window length overflowed on IST day {day}"))?;
        expected = expected
            .checked_add(u64::from(bars))
            .ok_or_else(|| "calendar receipt expected count overflowed u64".to_owned())?;
        let ordinal = u8::try_from(index)
            .map_err(|_| "calendar policy window index does not fit u8".to_owned())?;
        hasher.update(b"W");
        hasher.update(&[ordinal]);
        hasher.update(&window.from.to_le_bytes());
        hasher.update(&window.to.to_le_bytes());
        prior_to = Some(window.to);
    }
    Ok(expected)
}

/// Hash one complete day-level calendar decision.
fn hash_calendar_day_v1(
    day: i64,
    hasher: &mut brutex_core::blake3::Hasher,
) -> Result<CalendarDayFactsV1, Refusal> {
    hasher.update(b"D");
    hasher.update(&day.to_le_bytes());
    match pull::calendar::kind_of(day) {
        DayKind::Open(session) => {
            hasher.update(&[1]);
            Ok(CalendarDayFactsV1 {
                expected: hash_open_session_v1(day, session, hasher)?,
                unmeasured: false,
            })
        }
        DayKind::Closed => {
            hasher.update(&[2]);
            Ok(CalendarDayFactsV1 {
                expected: 0,
                unmeasured: false,
            })
        }
        DayKind::OpenLengthUnmeasured => {
            hasher.update(&[3]);
            Ok(CalendarDayFactsV1 {
                expected: 0,
                unmeasured: true,
            })
        }
        DayKind::Unmeasured => {
            hasher.update(&[4]);
            Ok(CalendarDayFactsV1 {
                expected: 0,
                unmeasured: true,
            })
        }
    }
}

/// Attest an exact ordered one-minute timestamp slice against the NSE calendar
/// for one explicit inclusive requested IST-day span.
///
/// Missing measured minutes are counted in [`CalendarReceiptV1::missing`]; they
/// are never synthesized and do not make construction fail. Duplicate,
/// backward or off-minute timestamps, a bar on a measured closed day, and a bar
/// outside the requested span or a measured open window are structural
/// contradictions and refuse the receipt. A day whose session length is
/// unmeasured remains accepted evidence, but the whole receipt is
/// [`CalendarStatusV1::Unmeasured`]. An empty offered slice is still measured
/// against the requested span: it is complete only when that span owes no bars.
///
/// # Cost
///
/// O(B + D) time for B offered timestamps and D inclusive calendar days, O(1)
/// auxiliary space, and no hidden collection. Calendar lookup and each day's
/// window walk are bounded by the calendar's fixed [`MAX_WINDOWS`]. This is a
/// structural bound, not a benchmark measurement.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
///
/// # Errors
///
/// A named [`Refusal`] for a backward or over-limit requested span, a malformed
/// timestamp slice, a timestamp outside the requested span or contradicting a
/// measured session, invalid calendar window geometry, or checked arithmetic
/// that cannot be represented.
pub fn calendar_receipt_v1(
    timestamps: &[i64],
    first_day: i64,
    last_day: i64,
) -> Result<CalendarReceiptV1, Refusal> {
    if first_day > last_day {
        return Err(format!(
            "calendar receipt requested span runs backward: {first_day} is after {last_day}"
        ));
    }
    let day_span = last_day
        .checked_sub(first_day)
        .and_then(|span| span.checked_add(1))
        .ok_or_else(|| "calendar receipt inclusive day span overflowed i64".to_owned())?;
    let day_span_usize = usize::try_from(day_span)
        .map_err(|_| "calendar receipt inclusive day span does not fit usize".to_owned())?;
    if day_span_usize > MAX_CALENDAR_RECEIPT_DAYS_V1 {
        return Err(format!(
            "calendar receipt requested span {first_day}..={last_day} is {day_span_usize} days; the existing {MAX_SPAN_MONTHS}-month span contract permits at most {MAX_CALENDAR_RECEIPT_DAYS_V1} days"
        ));
    }

    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(b"brutex.calendar-receipt.v1\0");
    hasher.update(&CALENDAR_RECEIPT_SCHEMA_V1.to_le_bytes());
    hasher.update(&CALENDAR_RECEIPT_POLICY_V1.to_le_bytes());
    hasher.update(&first_day.to_le_bytes());
    hasher.update(&last_day.to_le_bytes());
    hasher.update(&day_span.to_le_bytes());

    let offered = hash_offered_calendar_v1(timestamps, first_day, last_day, &mut hasher)?;
    let mut expected = 0_u64;
    let mut unmeasured = false;
    let mut day = first_day;
    loop {
        let facts = hash_calendar_day_v1(day, &mut hasher)?;
        expected = expected
            .checked_add(facts.expected)
            .ok_or_else(|| "calendar receipt expected count overflowed u64".to_owned())?;
        unmeasured |= facts.unmeasured;
        if day == last_day {
            break;
        }
        day = day
            .checked_add(1)
            .ok_or_else(|| "calendar receipt day walk overflowed i64".to_owned())?;
    }

    let missing = expected
        .checked_sub(offered.measured_offered)
        .ok_or_else(|| {
            "calendar receipt measured offered count exceeds measured expected minutes".to_owned()
        })?;
    let unexpected = 0_u64;
    let status = if unmeasured {
        CalendarStatusV1::Unmeasured
    } else if missing == 0 {
        CalendarStatusV1::Complete
    } else {
        CalendarStatusV1::Incomplete
    };

    hasher.update(b"S");
    hasher.update(&offered.offered.to_le_bytes());
    hasher.update(&expected.to_le_bytes());
    hasher.update(&missing.to_le_bytes());
    hasher.update(&unexpected.to_le_bytes());
    hasher.update(&[status.code()]);
    let digest = hasher.finalize();

    Ok(CalendarReceiptV1 {
        schema_version: CALENDAR_RECEIPT_SCHEMA_V1,
        policy_version: CALENDAR_RECEIPT_POLICY_V1,
        first_day,
        last_day,
        offered: offered.offered,
        expected,
        missing,
        unexpected,
        status,
        digest,
    })
}

/// Schema version of the rung-aware calendar receipt.
pub const CALENDAR_RECEIPT_SCHEMA_V2: u32 = 2;

/// Policy version binding the rung-aware, open-anchored bucket geometry.
///
/// Policy 2 meant the exact eight intraday signal rungs, an anchor at the NSE
/// open (09:15 IST), Euclidean bucket assignment, the measured windows from
/// [`pull::calendar::kind_of`], and a union of the at-most-two bucket intervals
/// that intersect those windows. It does not reinterpret a coarse bar as a
/// one-minute bar: each rung receives its own receipt.
///
/// # What policy 3 adds, and why it could not be a silent change
///
/// Policy 3 keeps all of that and adds one classification ahead of it: an IST
/// day named in [`CHARTER_NON_REGULAR_IST_DAYS`] is **withheld** — it expects
/// zero buckets, and an offered timestamp on it refuses.
///
/// This exists because [`SWEPT_SERIES_CALENDAR_POLICY`] removes those days'
/// bars at the load boundary. Under policy 2 the receipt still expected them,
/// so a complete store measured `Incomplete` and the whole Step 3 chain
/// refused. MEASURED before this arm existed: October 2025 at the 60-second
/// rung reported `offered 7500, expected 7560, missing 60` — the 60 one-minute
/// buckets of the 13:45–14:45 Muhurat session on IST day 20 382.
///
/// **The version is bumped rather than the arm added quietly**, because every
/// receipt digest changes: a withheld day now contributes a different tag and
/// a different expected count. Two receipts over one span under the two rules
/// are different evidence and `CLAUDE.md` §3 rule 8 forbids them sharing a
/// version.
///
/// **A withheld day is NOT hashed as [`DayKind::Closed`].** It gets a tag of
/// its own. A holiday and a withheld drill are different facts about the
/// exchange, and collapsing them would let two different calendars produce one
/// digest — the defect the whole receipt exists to prevent.
pub const CALENDAR_RECEIPT_POLICY_V2: u32 = 3;

const NSE_OPEN_MINUTE_V2: i64 = 555;
const CALENDAR_POLICY_DIGEST_DOMAIN_V2: &[u8] = b"brutex.calendar-policy.v2\0";
const CALENDAR_POLICY_RUNGS_V2: [u32; 8] = [60, 120, 180, 300, 600, 900, 1_800, 3_600];

/// Canonical identity of the complete measured-calendar interpretation policy.
///
/// A calendar *receipt* identifies one offered timestamp slice and requested
/// day span.  This digest instead identifies the policy used to interpret all
/// such slices: the V2 schema/policy versions, exact supported signal rungs,
/// open anchor, measured calendar range, and every measured day's disposition
/// and session windows.  Execution inputs and durable population identities
/// must carry this value rather than an arbitrary caller-chosen nonzero digest.
///
/// # Cost
///
/// O(D) time for the D days in the measured calendar and O(1) auxiliary space.
/// Each open day hashes at most [`MAX_WINDOWS`] windows.  This is run-boundary
/// provenance work, not a sweep inner-loop primitive, and is deliberately not
/// described as total O(1) work.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
#[must_use]
pub fn calendar_policy_digest_v2() -> [u8; 32] {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(CALENDAR_POLICY_DIGEST_DOMAIN_V2);
    hasher.update(&CALENDAR_RECEIPT_SCHEMA_V2.to_le_bytes());
    hasher.update(&CALENDAR_RECEIPT_POLICY_V2.to_le_bytes());
    hasher.update(&NSE_OPEN_MINUTE_V2.to_le_bytes());
    hasher.update(&u64::try_from(MAX_WINDOWS).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(&pull::calendar::FIRST_DAY.to_le_bytes());
    hasher.update(&pull::calendar::LAST_DAY.to_le_bytes());
    hasher.update(
        &u64::try_from(pull::calendar::DAYS)
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    hasher.update(
        &u64::try_from(CALENDAR_POLICY_RUNGS_V2.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for rung_seconds in CALENDAR_POLICY_RUNGS_V2 {
        hasher.update(&rung_seconds.to_le_bytes());
    }

    let mut day = pull::calendar::FIRST_DAY;
    loop {
        hasher.update(&day.to_le_bytes());
        match pull::calendar::kind_of(day) {
            DayKind::Open(session) => {
                hasher.update(&[1]);
                hasher.update(&[session.count]);
                for window in session
                    .windows
                    .iter()
                    .take(usize::from(session.count).min(MAX_WINDOWS))
                {
                    hasher.update(&window.from.to_le_bytes());
                    hasher.update(&window.to.to_le_bytes());
                }
            }
            DayKind::OpenLengthUnmeasured => {
                hasher.update(&[2]);
            }
            DayKind::Closed => {
                hasher.update(&[3]);
            }
            DayKind::Unmeasured => {
                hasher.update(&[4]);
            }
        }
        if day == pull::calendar::LAST_DAY {
            break;
        }
        day = day.saturating_add(1);
    }
    hasher.finalize()
}

/// Calendar attestation for one exact, ordered signal-rung timestamp slice.
///
/// Unlike [`CalendarReceiptV1`], which is deliberately the exact one-minute
/// execution receipt, V2 binds the signal rung. The digest includes the schema
/// and policy, rung width, requested IST-day boundaries, every offered
/// timestamp in order, every measured day and raw session window, the derived
/// open-anchored bucket intervals, and the final counts/status. It deliberately
/// excludes instrument, feed and OHLCV values; those belong to the surrounding
/// population/run identity rather than calendar coverage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarReceiptV2 {
    schema_version: u32,
    policy_version: u32,
    rung_seconds: u32,
    first_day: i64,
    last_day: i64,
    offered: u64,
    expected: u64,
    missing: u64,
    unexpected: u64,
    status: CalendarStatusV1,
    withheld_days: u32,
    withheld_buckets: u64,
    digest: [u8; 32],
}

impl CalendarReceiptV2 {
    /// Charter non-regular IST days inside this span, withheld from the sweep.
    ///
    /// Zero on a span holding none of them. This counts DAYS IN THE SPAN, not
    /// days that happened to carry bars: a withheld day the store never held is
    /// still a day this run declined to sweep, and reporting it only when the
    /// store had bars would make the number depend on the store rather than on
    /// the policy.
    #[must_use]
    pub const fn withheld_days(self) -> u32 {
        self.withheld_days
    }

    /// Buckets those withheld days would have expected at this rung.
    ///
    /// The size of what was removed, in the receipt's own unit. It is NOT
    /// included in [`Self::expected`] — that is the point: `expected` is what
    /// this run must supply, and a withheld day supplies nothing.
    #[must_use]
    pub const fn withheld_buckets(self) -> u64 {
        self.withheld_buckets
    }

    /// Receipt schema version.
    #[must_use]
    pub const fn schema_version(self) -> u32 {
        self.schema_version
    }

    /// Calendar and bucket interpretation policy version.
    #[must_use]
    pub const fn policy_version(self) -> u32 {
        self.policy_version
    }

    /// Width of this exact signal rung, in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }

    /// First requested IST day, inclusive.
    #[must_use]
    pub const fn first_day(self) -> i64 {
        self.first_day
    }

    /// Last requested IST day, inclusive.
    #[must_use]
    pub const fn last_day(self) -> i64 {
        self.last_day
    }

    /// Exact coarse-rung timestamps offered by the caller.
    #[must_use]
    pub const fn offered(self) -> u64 {
        self.offered
    }

    /// Open-anchored buckets intersecting measured session windows.
    #[must_use]
    pub const fn expected(self) -> u64 {
        self.expected
    }

    /// Expected measured buckets absent from the offered slice.
    #[must_use]
    pub const fn missing(self) -> u64 {
        self.missing
    }

    /// Offered buckets contradicting the measured calendar.
    ///
    /// Every public constructor refuses such a timestamp, so this remains zero
    /// for a constructed receipt and is retained as a digest-bound admission
    /// fact.
    #[must_use]
    pub const fn unexpected(self) -> u64 {
        self.unexpected
    }

    /// Completeness conclusion.
    #[must_use]
    pub const fn status(self) -> CalendarStatusV1 {
        self.status
    }

    /// Full BLAKE3 over this rung's calendar-coverage evidence.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }

    /// Project a complete receipt into the opaque capability accepted later.
    ///
    /// The projection keeps the already domain-separated V2 digest unchanged;
    /// it does not create a weaker summary digest. Every completeness predicate
    /// is checked even where the status implies it, so a future decoder cannot
    /// manufacture this capability from internally contradictory fields.
    ///
    /// # Errors
    ///
    /// A named refusal unless the receipt is measured complete, has no missing
    /// or unexpected bucket, and offered exactly the measured denominator.
    pub fn require_complete(self) -> Result<CompleteCalendarReceiptV2, Refusal> {
        if self.status != CalendarStatusV1::Complete
            || self.missing != 0
            || self.unexpected != 0
            || self.offered != self.expected
        {
            return Err(format!(
                "calendar receipt V2 for {} seconds over IST days {}..={} is not complete: status {:?}, offered {}, expected {}, missing {}, unexpected {}",
                self.rung_seconds,
                self.first_day,
                self.last_day,
                self.status,
                self.offered,
                self.expected,
                self.missing,
                self.unexpected
            ));
        }
        Ok(CompleteCalendarReceiptV2 {
            rung_seconds: self.rung_seconds,
            first_day: self.first_day,
            last_day: self.last_day,
            digest: self.digest,
        })
    }
}

/// Opaque proof that one signal rung completely covers its measured calendar.
///
/// Its fields are private and the only constructor is
/// [`CalendarReceiptV2::require_complete`]. Later persistence and selection
/// code can therefore require this type instead of remembering a list of
/// status/count predicates. The digest is the full receipt digest, not a second
/// summary hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompleteCalendarReceiptV2 {
    rung_seconds: u32,
    first_day: i64,
    last_day: i64,
    digest: [u8; 32],
}

impl CompleteCalendarReceiptV2 {
    /// Width of the proved-complete signal rung, in seconds.
    #[must_use]
    pub const fn rung_seconds(self) -> u32 {
        self.rung_seconds
    }

    /// First completely covered IST day, inclusive.
    #[must_use]
    pub const fn first_day(self) -> i64 {
        self.first_day
    }

    /// Last completely covered IST day, inclusive.
    #[must_use]
    pub const fn last_day(self) -> i64 {
        self.last_day
    }

    /// The unchanged, fully bound V2 receipt digest.
    #[must_use]
    pub const fn digest(self) -> [u8; 32] {
        self.digest
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BucketIntervalV2 {
    first: i64,
    last: i64,
}

/// Canonical union of the at-most-two bucket-index intervals in one session.
#[derive(Clone, Copy)]
struct ExpectedBucketsV2 {
    intervals: [Option<BucketIntervalV2>; MAX_WINDOWS],
    count: u8,
    expected: u64,
}

impl ExpectedBucketsV2 {
    const fn empty() -> Self {
        Self {
            intervals: [None; MAX_WINDOWS],
            count: 0,
            expected: 0,
        }
    }

    fn insert(&mut self, interval: BucketIntervalV2, day: i64) -> Result<(), Refusal> {
        if let Some(last_index) = usize::from(self.count).checked_sub(1) {
            let last = self
                .intervals
                .get_mut(last_index)
                .and_then(Option::as_mut)
                .ok_or_else(|| {
                    format!(
                        "calendar receipt V2 lost expected bucket interval {last_index} on IST day {day}"
                    )
                })?;
            if interval.first <= last.last.saturating_add(1) {
                last.last = last.last.max(interval.last);
                return Ok(());
            }
        }

        let slot = self
            .intervals
            .get_mut(usize::from(self.count))
            .ok_or_else(|| {
                format!(
                    "calendar receipt V2 produced more than {MAX_WINDOWS} disjoint bucket intervals on IST day {day}"
                )
            })?;
        *slot = Some(interval);
        self.count = self
            .count
            .checked_add(1)
            .ok_or_else(|| "calendar receipt V2 bucket interval count overflowed u8".to_owned())?;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Refusal> {
        let mut expected = 0_u64;
        for interval in self.intervals.iter().take(usize::from(self.count)) {
            let interval = interval.ok_or_else(|| {
                "calendar receipt V2 expected bucket interval was absent".to_owned()
            })?;
            let width = interval
                .last
                .checked_sub(interval.first)
                .and_then(|span| span.checked_add(1))
                .ok_or_else(|| {
                    "calendar receipt V2 expected bucket interval overflowed i64".to_owned()
                })?;
            let width = u64::try_from(width).map_err(|_| {
                "calendar receipt V2 expected bucket interval does not fit u64".to_owned()
            })?;
            expected = expected.checked_add(width).ok_or_else(|| {
                "calendar receipt V2 expected bucket count overflowed u64".to_owned()
            })?;
        }
        self.expected = expected;
        Ok(())
    }

    fn contains(self, bucket: i64) -> bool {
        self.intervals
            .iter()
            .take(usize::from(self.count))
            .flatten()
            .any(|interval| bucket >= interval.first && bucket <= interval.last)
    }
}

#[derive(Clone, Copy)]
struct OfferedCalendarFactsV2 {
    offered: u64,
    measured_offered: u64,
}

#[derive(Clone, Copy)]
struct CalendarDayFactsV2 {
    expected: u64,
    unmeasured: bool,
    /// Buckets this day WOULD have expected had the charter not withheld it.
    ///
    /// Counted rather than discarded so the receipt can state the size of what
    /// it removed. A withheld day that reported only `expected = 0` would be
    /// indistinguishable from a holiday in the totals, and `CLAUDE.md` §3
    /// rule 2 is about a scope change being visible.
    withheld: u64,
}

/// Is this IST day one the swept-series calendar policy withholds?
///
/// The charter list is the sole authority. No session length is measured and
/// no exchange fact is derived here — §3 rule 1.
fn withheld_by_charter(day: i64) -> bool {
    CHARTER_NON_REGULAR_IST_DAYS.contains(&day)
}

/// Refuse a withheld day's bar that the measured calendar cannot place.
///
/// # Why a withheld bar is CHECKED before it is dropped
///
/// MEASURED, in the operator's own store: dhan's
/// `NIFTY/1min/2024-03.bin` holds **6 857** records against zerodha's 6 855,
/// and the difference is two bars on IST day 19 784 at minutes 600 and 750 —
/// one past each edge of the measured `555..=599` and `690..=749` windows.
///
/// Today those two refuse by name, in [`require_canonical_minute`] and again in
/// `hash_offered_calendar_v2`. **Both of those run after this loader.** A filter
/// that dropped every bar on a withheld day would delete a real vendor defect on
/// its way past and report success — the row `CLAUDE.md` §4 bans outright:
/// *"A fallback that hides a failure."*
///
/// Withholding a session decides which SESSIONS this run sweeps. It is not
/// permission to stop reading the bytes.
///
/// # Why the test is bucket-based and not minute-based
///
/// `session.expects(minute)` is the right predicate at `1min` and WRONG at every
/// coarser rung. On day 19 784 at `60min` the third bucket opens at IST minute
/// 675, which lies inside the 90-minute break and belongs to no window, yet that
/// bucket legitimately intersects `690..=749` and is expected. Testing its left
/// edge against the windows would refuse a correct bar. `expected_buckets_v2` is
/// the same geometry the receipt uses, so the loader and the receipt agree by
/// construction rather than by coincidence.
///
/// # Cost
///
/// Reached only for a bar ON one of the nine charter days — at most a few
/// hundred bars in the whole 2020..2026 span — so the per-bar bucket derivation
/// is not on the ordinary load path at all. UNVERIFIED as a measured bound;
/// read off the source per §3 rule 6.
fn refuse_uncalendared_withheld_bar(
    index: u64,
    ts_micros: i64,
    rung_seconds: u32,
) -> Result<(), Refusal> {
    let Ok(rung_minutes) = rung_minutes_v2(rung_seconds) else {
        // Not one of the eight swept rungs, so no bucket geometry is defined
        // for it and there is nothing to check against. `1s` is the only such
        // rung the store can hold, and it is never swept.
        return Ok(());
    };
    let (day, minute) = exact_ist_minute_v1(ts_micros)?;
    let bucket = bucket_index_v2(i64::from(minute), rung_minutes);
    match pull::calendar::kind_of(day) {
        DayKind::Open(session) => {
            if expected_buckets_v2(day, session, rung_minutes)?.contains(bucket) {
                return Ok(());
            }
            Err(format!(
                "record {index} at timestamp {ts_micros} is bucket {bucket} (IST minute {minute}) on withheld IST day {day}, and that bucket intersects no measured NSE session window. The session is withheld from the sweep, but a bar the calendar cannot place is a store defect and is refused rather than dropped"
            ))
        }
        DayKind::Closed => Err(format!(
            "record {index} at timestamp {ts_micros} is on withheld IST day {day}, which the measured calendar reports closed. A bar on a closed day is a store defect and is refused rather than dropped"
        )),
        // AN UNMEASURED-LENGTH DAY IS DROPPED AND COUNTED, NEVER REFUSED.
        //
        // This arm refused once, and the refusal was wrong twice over.
        //
        // MEASURED: dhan's `NIFTY/1min/2021-11.bin` holds 43 committed records
        // on IST day 18 935 at IST minutes 887..929 (14:47-15:29) -- inside the
        // pull's [09:15, 15:30) window, not the evening Muhurat D-0502 assumed.
        // Refusing them made `load_classified_with_ceiling` refuse the whole
        // SPAN, so every dhan range covering 2021-11 died on all eight rungs.
        // dhan's 1min coverage begins 2021-08, so `range-all dhan NIFTY` could
        // not run at all. That is a regression the charter policy never
        // intended and no operator asked for.
        //
        // It is also the wrong question. Refusing exists to stop a bar the
        // calendar CAN place being deleted unseen -- see the `Open` arm. Here
        // the calendar places nothing: `OpenLengthUnmeasured` means the session
        // length was never measured, so there is no window to be outside of and
        // no defect to name. Refusing would not report a fault, it would report
        // our own missing authority as the vendor's error.
        //
        // The day is withheld either way, so the bar is not swept either way.
        // It is dropped, counted in `CalendarExclusion`, and named by date in
        // the report -- which is the visibility §3 rule 2 asks for.
        DayKind::OpenLengthUnmeasured => Ok(()),
        DayKind::Unmeasured => Err(format!(
            "record {index} at timestamp {ts_micros} is on withheld IST day {day}, outside the canonical NSE calendar's measured range. The bar is refused rather than dropped"
        )),
    }
}

/// Exact eight signal rungs, as whole minutes.
fn rung_minutes_v2(rung_seconds: u32) -> Result<i64, Refusal> {
    match rung_seconds {
        60 | 120 | 180 | 300 | 600 | 900 | 1_800 | 3_600 => Ok(i64::from(rung_seconds / 60)),
        _ => Err(format!(
            "calendar receipt V2 rung {rung_seconds} seconds is not one of the exact signal rungs: 60, 120, 180, 300, 600, 900, 1800, 3600"
        )),
    }
}

const fn bucket_index_v2(minute: i64, rung_minutes: i64) -> i64 {
    minute
        .saturating_sub(NSE_OPEN_MINUTE_V2)
        .div_euclid(rung_minutes)
}

const fn bucket_minute_v2(bucket: i64, rung_minutes: i64) -> i64 {
    NSE_OPEN_MINUTE_V2.saturating_add(bucket.saturating_mul(rung_minutes))
}

/// Derive and validate the canonical expected bucket union for one open day.
fn expected_buckets_v2(
    day: i64,
    session: Session,
    rung_minutes: i64,
) -> Result<ExpectedBucketsV2, Refusal> {
    let count = usize::from(session.count);
    if count == 0 || count > MAX_WINDOWS {
        return Err(format!(
            "calendar policy produced {count} windows for open IST day {day}; V2 supports 1..={MAX_WINDOWS}"
        ));
    }

    let mut expected = ExpectedBucketsV2::empty();
    let mut prior_to = None;
    for index in 0..count {
        let window = session.windows.get(index).copied().ok_or_else(|| {
            format!("calendar policy omitted window {index} for open IST day {day}")
        })?;
        if window.from > window.to || window.to >= 1_440 {
            return Err(format!(
                "calendar policy window {index} on IST day {day} is invalid: {}..={} is not an ordered minute-of-day range",
                window.from, window.to
            ));
        }
        if prior_to.is_some_and(|prior| window.from <= prior) {
            return Err(format!(
                "calendar policy windows overlap or run backward on IST day {day} at window {index}"
            ));
        }
        expected.insert(
            BucketIntervalV2 {
                first: bucket_index_v2(i64::from(window.from), rung_minutes),
                last: bucket_index_v2(i64::from(window.to), rung_minutes),
            },
            day,
        )?;
        prior_to = Some(window.to);
    }
    expected.finish()?;
    Ok(expected)
}

/// Validate and hash every offered rung timestamp without a membership set.
fn hash_offered_calendar_v2<I>(
    timestamps: I,
    rung_minutes: i64,
    first_day: i64,
    last_day: i64,
    hasher: &mut brutex_core::blake3::Hasher,
) -> Result<OfferedCalendarFactsV2, Refusal>
where
    I: IntoIterator<Item = i64>,
{
    let mut previous: Option<i64> = None;
    let mut offered = 0_u64;
    let mut measured_offered = 0_u64;

    for (index, timestamp) in timestamps.into_iter().enumerate() {
        let (day, minute) = exact_ist_minute_v1(timestamp)?;
        if let Some(prior) = previous
            && timestamp <= prior
        {
            let fault = if timestamp == prior {
                "duplicate"
            } else {
                "backward"
            };
            return Err(format!(
                "calendar receipt V2 timestamp {index} is {fault}: {prior} then {timestamp}"
            ));
        }
        if day < first_day || day > last_day {
            return Err(format!(
                "calendar receipt V2 timestamp {timestamp} belongs to IST day {day}, outside requested inclusive span {first_day}..={last_day}"
            ));
        }

        let minute = i64::from(minute);
        let bucket = bucket_index_v2(minute, rung_minutes);
        if bucket_minute_v2(bucket, rung_minutes) != minute {
            let rung_seconds = rung_minutes.saturating_mul(60);
            return Err(format!(
                "calendar receipt V2 timestamp {timestamp} at IST minute {minute} is off the exact {rung_seconds}-second open-anchored grid"
            ));
        }

        // A BAR ON A WITHHELD DAY IS A DEFECT, NOT DATA, AND IT REFUSES.
        //
        // `SWEPT_SERIES_CALENDAR_POLICY` drops these bars at the load boundary,
        // so under a correct build none is ever offered here. That is exactly
        // why the check earns its place: it is the only thing that would catch
        // the loader and the receipt disagreeing about which sessions a run
        // saw, and a silent acceptance would make the receipt's `expected = 0`
        // a lie the digest then certified.
        if withheld_by_charter(day) {
            return Err(format!(
                "calendar receipt V2 timestamp {timestamp} is on IST day {day}, a charter non-regular session that policy {SWEPT_SERIES_CALENDAR_POLICY} withholds from every swept series. The loader did not remove it, so the series and the calendar disagree"
            ));
        }
        let decision = match pull::calendar::kind_of(day) {
            DayKind::Open(session) => {
                let expected = expected_buckets_v2(day, session, rung_minutes)?;
                if !expected.contains(bucket) {
                    return Err(format!(
                        "calendar receipt V2 timestamp {timestamp} is bucket {bucket} on measured open IST day {day}, but that bucket intersects no measured session window"
                    ));
                }
                measured_offered = measured_offered.checked_add(1).ok_or_else(|| {
                    "calendar receipt V2 measured-offered count overflowed u64".to_owned()
                })?;
                1_u8
            }
            DayKind::Closed => {
                return Err(format!(
                    "calendar receipt V2 timestamp {timestamp} is on measured closed IST day {day}"
                ));
            }
            DayKind::OpenLengthUnmeasured => 3,
            DayKind::Unmeasured => 4,
        };

        offered = offered
            .checked_add(1)
            .ok_or_else(|| "calendar receipt V2 offered count overflowed u64".to_owned())?;
        let ordinal = u64::try_from(index)
            .map_err(|_| "calendar receipt V2 timestamp index does not fit u64".to_owned())?;
        hasher.update(b"T");
        hasher.update(&ordinal.to_le_bytes());
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&day.to_le_bytes());
        hasher.update(&minute.to_le_bytes());
        hasher.update(&bucket.to_le_bytes());
        hasher.update(&[decision]);
        previous = Some(timestamp);
    }

    Ok(OfferedCalendarFactsV2 {
        offered,
        measured_offered,
    })
}

/// Hash one measured open session and its derived bucket geometry.
fn hash_open_session_v2(
    day: i64,
    session: Session,
    rung_minutes: i64,
    hasher: &mut brutex_core::blake3::Hasher,
) -> Result<u64, Refusal> {
    let expected = expected_buckets_v2(day, session, rung_minutes)?;
    hasher.update(&[session.count]);
    for (index, window) in session
        .windows
        .iter()
        .take(usize::from(session.count))
        .enumerate()
    {
        let ordinal = u8::try_from(index)
            .map_err(|_| "calendar receipt V2 window index does not fit u8".to_owned())?;
        hasher.update(b"W");
        hasher.update(&[ordinal]);
        hasher.update(&window.from.to_le_bytes());
        hasher.update(&window.to.to_le_bytes());
    }
    hasher.update(b"B");
    hasher.update(&[expected.count]);
    for (index, interval) in expected
        .intervals
        .iter()
        .take(usize::from(expected.count))
        .flatten()
        .enumerate()
    {
        let ordinal = u8::try_from(index)
            .map_err(|_| "calendar receipt V2 bucket interval index does not fit u8".to_owned())?;
        hasher.update(&[ordinal]);
        hasher.update(&interval.first.to_le_bytes());
        hasher.update(&interval.last.to_le_bytes());
    }
    hasher.update(&expected.expected.to_le_bytes());
    Ok(expected.expected)
}

/// Hash one complete day-level V2 calendar decision.
fn hash_calendar_day_v2(
    day: i64,
    rung_minutes: i64,
    hasher: &mut brutex_core::blake3::Hasher,
) -> Result<CalendarDayFactsV2, Refusal> {
    hasher.update(b"D");
    hasher.update(&day.to_le_bytes());
    // THE WITHHELD ARM IS TESTED FIRST, AND ITS TAG IS ITS OWN.
    //
    // Ahead of `kind_of` because a withheld day is Open in the exchange's
    // calendar — 2024-03-02 is a real 105-bar session — and the question here
    // is not whether the exchange traded but whether this run swept it. The
    // day's own geometry is still measured, so `withheld` can state the size
    // of what was removed instead of reporting a bare zero.
    if withheld_by_charter(day) {
        hasher.update(&[5]);
        let withheld = match pull::calendar::kind_of(day) {
            DayKind::Open(session) => hash_open_session_v2(day, session, rung_minutes, hasher)?,
            DayKind::Closed | DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => 0,
        };
        return Ok(CalendarDayFactsV2 {
            expected: 0,
            unmeasured: false,
            withheld,
        });
    }
    match pull::calendar::kind_of(day) {
        DayKind::Open(session) => {
            hasher.update(&[1]);
            Ok(CalendarDayFactsV2 {
                expected: hash_open_session_v2(day, session, rung_minutes, hasher)?,
                unmeasured: false,
                withheld: 0,
            })
        }
        DayKind::Closed => {
            hasher.update(&[2]);
            Ok(CalendarDayFactsV2 {
                expected: 0,
                unmeasured: false,
                withheld: 0,
            })
        }
        DayKind::OpenLengthUnmeasured => {
            hasher.update(&[3]);
            Ok(CalendarDayFactsV2 {
                expected: 0,
                unmeasured: true,
                withheld: 0,
            })
        }
        DayKind::Unmeasured => {
            hasher.update(&[4]);
            Ok(CalendarDayFactsV2 {
                expected: 0,
                unmeasured: true,
                withheld: 0,
            })
        }
    }
}

/// Attest an ordered signal-rung timestamp slice against the measured calendar.
///
/// The rung is one of 60, 120, 180, 300, 600, 900, 1,800 or 3,600 seconds.
/// Every measured session window contributes every open-anchored bucket it
/// intersects; overlapping or adjacent bucket intervals from a split session
/// are merged before the denominator is counted. Missing buckets are counted,
/// never synthesized. Duplicate, backward, off-grid, closed-day and
/// non-intersecting measured-day timestamps refuse the receipt.
///
/// # Cost
///
/// O(B + D) time for B offered timestamps and D inclusive requested calendar
/// days, O(1) auxiliary space. Each operation over a session is bounded by
/// [`MAX_WINDOWS`] (currently two). This is a structural bound, not a measured
/// benchmark result.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
///
/// # Errors
///
/// A named [`Refusal`] for an unsupported rung, backward or over-limit span,
/// malformed timestamps, contradictions with the measured calendar, invalid
/// calendar geometry, or checked arithmetic that cannot be represented.
pub fn calendar_receipt_v2(
    timestamps: &[i64],
    rung_seconds: u32,
    first_day: i64,
    last_day: i64,
) -> Result<CalendarReceiptV2, Refusal> {
    calendar_receipt_v2_from_iter(
        timestamps.iter().copied(),
        rung_seconds,
        first_day,
        last_day,
    )
}

/// Attest exact ordered candle timestamps without allocating a parallel vector.
///
/// The receipt is byte-identical to [`calendar_receipt_v2`] over a separately
/// collected timestamp slice.  Keeping the projection internal to this walk
/// lets `CandidateUniverse` and pre-admission source builders prove that their
/// exact OHLCV streams match their calendar receipts in O(1) auxiliary space.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
///
/// # Errors
///
/// Every refusal made by [`calendar_receipt_v2`], unchanged.
pub fn calendar_receipt_v2_for_bars(
    bars: &[Candle],
    rung_seconds: u32,
    first_day: i64,
    last_day: i64,
) -> Result<CalendarReceiptV2, Refusal> {
    calendar_receipt_v2_from_iter(
        bars.iter().map(|bar| bar.ts_micros),
        rung_seconds,
        first_day,
        last_day,
    )
}

fn calendar_receipt_v2_from_iter<I>(
    timestamps: I,
    rung_seconds: u32,
    first_day: i64,
    last_day: i64,
) -> Result<CalendarReceiptV2, Refusal>
where
    I: IntoIterator<Item = i64>,
{
    let rung_minutes = rung_minutes_v2(rung_seconds)?;
    if first_day > last_day {
        return Err(format!(
            "calendar receipt V2 requested span runs backward: {first_day} is after {last_day}"
        ));
    }
    let day_span = last_day
        .checked_sub(first_day)
        .and_then(|span| span.checked_add(1))
        .ok_or_else(|| "calendar receipt V2 inclusive day span overflowed i64".to_owned())?;
    let day_span_usize = usize::try_from(day_span)
        .map_err(|_| "calendar receipt V2 inclusive day span does not fit usize".to_owned())?;
    if day_span_usize > MAX_CALENDAR_RECEIPT_DAYS_V1 {
        return Err(format!(
            "calendar receipt V2 requested span {first_day}..={last_day} is {day_span_usize} days; the existing {MAX_SPAN_MONTHS}-month span contract permits at most {MAX_CALENDAR_RECEIPT_DAYS_V1} days"
        ));
    }

    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(b"brutex.calendar-receipt.v2\0");
    hasher.update(&CALENDAR_RECEIPT_SCHEMA_V2.to_le_bytes());
    hasher.update(&CALENDAR_RECEIPT_POLICY_V2.to_le_bytes());
    hasher.update(&rung_seconds.to_le_bytes());
    hasher.update(&first_day.to_le_bytes());
    hasher.update(&last_day.to_le_bytes());
    hasher.update(&day_span.to_le_bytes());

    let offered =
        hash_offered_calendar_v2(timestamps, rung_minutes, first_day, last_day, &mut hasher)?;
    let mut expected = 0_u64;
    let mut unmeasured = false;
    let mut withheld_buckets = 0_u64;
    let mut withheld_days = 0_u32;
    let mut day = first_day;
    loop {
        let facts = hash_calendar_day_v2(day, rung_minutes, &mut hasher)?;
        expected = expected
            .checked_add(facts.expected)
            .ok_or_else(|| "calendar receipt V2 expected bucket count overflowed u64".to_owned())?;
        if withheld_by_charter(day) {
            withheld_days = withheld_days.saturating_add(1);
            withheld_buckets = withheld_buckets
                .checked_add(facts.withheld)
                .ok_or_else(|| {
                    "calendar receipt V2 withheld bucket count overflowed u64".to_owned()
                })?;
        }
        unmeasured |= facts.unmeasured;
        if day == last_day {
            break;
        }
        day = day
            .checked_add(1)
            .ok_or_else(|| "calendar receipt V2 day walk overflowed i64".to_owned())?;
    }

    let missing = expected
        .checked_sub(offered.measured_offered)
        .ok_or_else(|| {
            "calendar receipt V2 measured offered count exceeds measured expected buckets"
                .to_owned()
        })?;
    let unexpected = 0_u64;
    let status = if unmeasured {
        CalendarStatusV1::Unmeasured
    } else if missing == 0 {
        CalendarStatusV1::Complete
    } else {
        CalendarStatusV1::Incomplete
    };

    hasher.update(b"S");
    hasher.update(&offered.offered.to_le_bytes());
    hasher.update(&expected.to_le_bytes());
    hasher.update(&missing.to_le_bytes());
    hasher.update(&unexpected.to_le_bytes());
    hasher.update(&[status.code()]);
    // Appended after the status byte, so every term above keeps its position.
    hasher.update(b"X");
    hasher.update(&withheld_days.to_le_bytes());
    hasher.update(&withheld_buckets.to_le_bytes());
    let digest = hasher.finalize();

    Ok(CalendarReceiptV2 {
        schema_version: CALENDAR_RECEIPT_SCHEMA_V2,
        policy_version: CALENDAR_RECEIPT_POLICY_V2,
        rung_seconds,
        first_day,
        last_day,
        offered: offered.offered,
        expected,
        missing,
        unexpected,
        status,
        withheld_days,
        withheld_buckets,
        digest,
    })
}

/// Version of the stored daily-reference payload and causal-join schema.
///
/// This number is folded into every stored run identity.  Changing the meaning
/// of a daily record without incrementing it would let two computations share
/// one identity.
pub const DAILY_REFERENCE_SCHEMA: u32 = 1;

/// Version of the charter-calendar eligibility rule.
///
/// The exact excluded IST-day list is also bound into the identity; the version
/// names the rule that interprets that list.
pub const DAILY_ELIGIBILITY_POLICY: u32 = 1;

/// Version of the exact-minute `GapFib` overlay and close-alignment policy.
///
/// Policy 1 means: fold [`indicators::gap::GapFib`] only over stored `1min`
/// bars from the same feed and instrument, then copy positions 132..=142 from
/// the minute opening at `signal_open + signal_duration - one_minute`.  A later
/// minute is never substituted and signal-local `GapFib` bits are always erased.
pub const EXACT_MINUTE_GAP_POLICY: u32 = 1;

/// The integrity statement the ordinary stored-read path can honestly make.
///
/// `BarFile::open_existing` validates the committed header and geometry but
/// deliberately does not perform the O(file) checksum scrub.  No scrub receipt
/// is supplied to this module, so calling these references integrity-sealed
/// would invent evidence the read did not produce.
pub const DAILY_INTEGRITY_NOTE: &str = "UNVERIFIED -- committed store records were read, but no independent checksum-scrub receipt was supplied";

/// Integrity statement for the exact-minute `GapFib` evidence.
///
/// This is deliberately the same honest limitation as the daily stream, but a
/// separate constant keeps the receipt from implying that one verification
/// covered both files.
pub const EXACT_MINUTE_INTEGRITY_NOTE: &str = "UNVERIFIED -- exact stored 1min records were read, but no independent checksum-scrub receipt was supplied";

/// Stored one-day evidence ready for the causal anchored evaluator.
#[derive(Clone, Debug)]
pub struct DailyContext {
    /// Every one-day OHLCV record that can influence the requested signal span.
    pub bars: Vec<Candle>,
    /// The validated records, parallel to [`Self::bars`].
    pub references: Vec<DailyReference>,
    /// One identity byte per record: `1` eligible, `0` explicitly excluded.
    pub eligibility: Vec<u8>,
    /// Months asked of the daily store, including the preceding warm-up month.
    pub asked: u32,
    /// Months actually found.
    pub found: u32,
}

/// Same-feed, same-instrument stored one-minute evidence for `GapFib`.
#[derive(Clone, Debug)]
pub struct ExactMinuteContext {
    /// Complete one-minute context, including the preceding warm-up month.
    pub bars: Vec<Candle>,
    /// Months asked of the one-minute store.
    pub asked: u32,
    /// Months actually found.
    pub found: u32,
    /// Latest regular IST session strictly before the first signal day.
    pub prior_session_day: i64,
    /// Exact bars observed on that prior session.
    pub prior_session_bars: u32,
    /// Which charter non-regular sessions the one-minute stream withheld.
    ///
    /// Carried so the report can prove the minute context and the signal series
    /// dropped the SAME days. That is the property `overlay_exact_minute_gapfib`
    /// depends on: it maps each signal bar's close to an exact minute, and a day
    /// present in one series and absent from the other would either refuse or,
    /// worse, resolve against a session the signal series never saw.
    pub excluded: CalendarExclusion,
}

/// The only distinction a range loader is allowed to recover from.
///
/// A file that does not exist is a named hole in the requested sample. Every
/// other store failure means bytes exist (or should exist) but cannot be
/// trusted, and continuing would turn corruption, a live writer, or an I/O
/// fault into a silently smaller backtest.
enum LoadFailure {
    Missing(Refusal),
    Refused(Refusal),
}

impl LoadFailure {
    fn message(self) -> Refusal {
        match self {
            Self::Missing(why) | Self::Refused(why) => why,
        }
    }
}

impl From<Refusal> for LoadFailure {
    fn from(why: Refusal) -> Self {
        Self::Refused(why)
    }
}

/// The rung whose directory word is `name`, or the words that do exist.
///
/// `Timeframe::KNOWN` is the store's own declared list, so a rung added there is
/// selectable here with no edit. The refusal names every legal word rather than
/// saying "unknown": a caller who typed `1hour` needs to be told it is `60min`.
///
/// # Ten words, and the engine sweeps EIGHT of them
///
/// This answers *"is this a rung the store carries"* and deliberately not *"is
/// it a rung the engine sweeps"*. `1day` and `1s` are in `Timeframe::KNOWN`,
/// have real directories, and are legitimately read by non-sweep callers —
/// [`load_daily_context`] opens `1day` on every stored sweep. Narrowing this
/// list to [`crate::EVERY_RUNG`] would break those correct callers to fix an
/// incorrect one, so the sweep guard belongs at each sweep ENTRY POINT and not
/// in the shared parser. `batch::swept_rung` is that guard for `sweep-all`,
/// which is why this is crate-visible rather than private.
pub(crate) fn rung(name: &str) -> Result<Timeframe, Refusal> {
    Timeframe::KNOWN
        .iter()
        .copied()
        .find(|t| t.as_str() == name)
        .ok_or_else(|| {
            let known: Vec<&str> = Timeframe::KNOWN.iter().map(|t| t.as_str()).collect();
            // Clipped for the reason [`clipped`] gives, and here as well as in
            // `swept_index` because the rung is the SAME command's next
            // argument: fixing one echo and leaving its neighbour would mean an
            // operator's typo is truncated or not depending on which word they
            // mistyped.
            format!(
                "`{}` is not a rung this store carries. The rungs are: {}.",
                clipped(name),
                known.join(", ")
            )
        })
}

/// The most of a caller-supplied word that belongs inside a refusal.
///
/// # Why a refusal has to cut its own input
///
/// `underlying` arrives from a command line or an HTTP body and is bounded by
/// neither. A 10,000-character argument was echoed back WHOLE into
/// [`swept_index`]'s refusal, and the caller then prints roughly a hundred
/// further lines of usage under it — so the one sentence naming the mistake was
/// pushed off the operator's screen by their own typo. The word is evidence, not
/// a payload: sixty-four characters are enough to recognise what was typed, and
/// short enough that the sentence and the usage block both stay readable.
///
/// A `Symbol` holds at most [`brutex_core::symbol::SYMBOL_CAPACITY`] bytes, so
/// nothing this cut removes could ever have been a legal instrument — the only
/// strings it shortens are ones already being refused.
///
/// Cut at CHARACTERS and never at bytes, so the cut cannot land inside one and
/// produce a refusal that is not valid UTF-8 text. Same reasoning, and the same
/// shape, as `pull::http`'s own body trim.
///
/// # Cost
///
/// O(1) in the length of `word`, not O(len). `take` stops at 64 characters and
/// `nth` stops at the 65th, so a one-megabyte argument is read for 65 characters
/// and dropped. That matters because the input is exactly the thing that is not
/// bounded; a cut that had to walk what it refuses would be a second way to make
/// an oversized word expensive.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
fn clipped(word: &str) -> String {
    /// Characters of the offending word a refusal keeps.
    const KEEP: usize = 64;

    let mut out: String = word.chars().take(KEEP).collect();
    if word.chars().nth(KEEP).is_some() {
        out.push('…');
    }
    out
}

/// One instrument this engine is allowed to sweep: a spot index, or -- since
/// D-0506 -- an F&O cash equity.
///
/// Constructing an [`InstrumentKey`] proves only that the identifier is a
/// well-formed, storable NSE symbol. Reference indices, contracts and stocks
/// outside the F&O universe can therefore have real files in the store without
/// belonging to the sweep surface. Every stored-sweep entry door comes through
/// this helper so the authoritative allow-list remains
/// [`InstrumentKey::is_sweepable`] -- [`InstrumentKey::SWEPT`] for the indices
/// and `FNO_INDEX` for the equities -- not a second string list in the CLI or
/// API.
///
/// # How one word resolves to one of two shapes
///
/// The two indices are tried first, by name. A word that is not one of them is
/// tried as a cash equity. Both keys are built and both are checked, so the
/// refusal a reader sees names the surface as it actually is rather than
/// "not an index".
///
/// Every refusal is ONE SENTENCE, whatever the word did wrong. A malformed
/// word, a well-formed word off the surface and a contract are three causes
/// and one refusal: *`X` is not an instrument this engine sweeps: <cause>. The
/// sweep surface is …*. `an_enormous_instrument_word_is_clipped_out_of_its_own_refusal`
/// pins that a typo and a 10,000-character word are refused by the same
/// sentence, which is what lets an operator grep a log for one phrase.
///
/// The word is named through [`clipped`] rather than interpolated raw. The
/// malformed arm is the one an unbounded argument actually reaches —
/// `Symbol::new` refuses anything past its capacity — and the others are
/// clipped for the same reason rather than because they can grow: three
/// refusal sites for one field must not disagree about how much of it they
/// will print, which is why they share one value rather than each calling the
/// cut.
///
/// That value is built EAGERLY, on the success path too. It is at most 67 bytes
/// and this function runs once per instrument-month load, never per bar and
/// never per candidate, so it is not one of the five operations `CLAUDE.md` §3
/// rule 4 bounds. Buying that with a predictable shape — one clip, one
/// sentence, no way for them to drift — is the better trade.
pub(crate) fn swept_index(underlying: &str) -> Result<InstrumentKey, Refusal> {
    let named = clipped(underlying);
    let refuse = |why: String| {
        let indices = InstrumentKey::SWEPT
            .iter()
            .map(|(_, symbol)| *symbol)
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "`{named}` is not an instrument this engine sweeps: {why}. \
             The sweep surface is the NSE spot indices {indices} and the NSE \
             cash equities of the 213 F&O underlyings (D-0506). Nothing was read."
        )
    };
    // THE INDEX FIRST, BY NAME. NIFTY and BANKNIFTY are indices and nothing
    // else; a word that is one of them never reaches the equity arm.
    let as_index =
        InstrumentKey::index(Exchange::Nse, underlying).map_err(|why| refuse(why.to_string()))?;
    if as_index.is_sweepable() {
        return Ok(as_index);
    }
    // THEN THE CASH EQUITY. Same word, the stock's own price series. Sweepable
    // only when the symbol is one of the 213 F&O underlyings.
    let as_cash =
        InstrumentKey::cash(Exchange::Nse, underlying).map_err(|why| refuse(why.to_string()))?;
    as_cash
        .require_sweepable()
        .map_err(|why| refuse(why.to_string()))?;
    Ok(as_cash)
}

/// One instrument-month of real bars, or the reason there are none.
///
/// # Errors
///
/// Every arm is a [`Refusal`] naming what was asked for and what the store said.
/// A missing file is the ordinary case — it means that month was never pulled —
/// and it is reported as that rather than as an I/O error, because the operator's
/// next action is a pull and not a filesystem check.
pub fn load(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    year: u16,
    month: u8,
) -> Result<Loaded, Refusal> {
    load_classified(root, vendor, underlying, rung_name, year, month).map_err(LoadFailure::message)
}

/// Explicit ceiling for one assembled stored-bar span.
///
/// This type has no `Default`: a production caller must name how many records
/// it is prepared to allocate and hash.  The ceiling is checked against each
/// opened month's committed header count before allocating that month's
/// vector, and against the accumulated span before the next month is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredSpanLoadBoundV1 {
    max_records: u64,
}

impl StoredSpanLoadBoundV1 {
    /// Construct a nonzero record ceiling.
    ///
    /// # Errors
    ///
    /// Zero cannot authorize even one stored record and is refused before any
    /// store path is opened.
    pub fn new(max_records: u64) -> Result<Self, Refusal> {
        if max_records == 0 {
            return Err(
                "stored span record ceiling must be greater than zero; nothing was opened"
                    .to_owned(),
            );
        }
        Ok(Self { max_records })
    }

    /// Maximum number of records the complete assembled span may hold.
    #[must_use]
    pub const fn max_records(self) -> u64 {
        self.max_records
    }
}

/// [`load`] with genuine absence kept distinct from every unsafe refusal.
fn load_classified(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    year: u16,
    month: u8,
) -> Result<Loaded, LoadFailure> {
    load_classified_with_ceiling(root, vendor, underlying, rung_name, year, month, None)
}

/// Classified month load with an optional pre-allocation committed-record cap.
fn load_classified_with_ceiling(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    year: u16,
    month: u8,
    remaining_records: Option<u64>,
) -> Result<Loaded, LoadFailure> {
    let timeframe = rung(rung_name)?;
    let key = swept_index(underlying)?;
    let ym = YearMonth::new(year, month)
        .map_err(|why| format!("{year}-{month:02} is not a month: {why}"))?;
    let path = StorePath::for_key(vendor, &key, timeframe, ym, FileKind::Bars)
        .map_err(|why| format!("no store path for that selection: {why}"))?;

    // THE SYMBOL ID IS THE STORE'S OWN, NOT A NEW ONE. `pull::ingest` writes
    // `fnv1a(symbol) as u32` into the header, and `open_existing` compares what
    // it is handed against what it finds. Computing it any other way here would
    // make every file refuse to open with a mismatch that named nothing real.
    //
    // HASHED FROM `key`, NOT FROM THE CALLER'S STRING, AND THAT WAS A REAL BUG.
    //
    // `swept_index` above normalises through `Symbol::new`, which UPPER-CASES —
    // vendors are not consistent about case and `nifty` and `NIFTY` are the same
    // instrument. `StorePath::for_key` therefore resolved `.../NIFTY/...` while
    // this line hashed the raw `underlying`, so `cli sweep-stored dhan nifty
    // 1min 2026 1` opened the RIGHT file and was then refused by it:
    // `…/NIFTY/1min/2026-01.bin holds symbol 433894009, not 1196260313` — two
    // numbers naming nothing, sending an operator to audit a store that was
    // correct when they had made a typo. `pull::ingest` hashes
    // `symbol.as_str()` off its own `Symbol`, so the canonical form is what is
    // on disk and the canonical form is what must be asked for here.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id IS the low 32 bits of the FNV-1a hash — `pull::ingest` \
                  writes it that way at its own `as u32`, and the header compares \
                  what it was handed against what it finds. Widening or checking \
                  here would compute a different number and every file would \
                  refuse to open with a mismatch that named nothing real."
    )]
    let symbol_id = brutex_core::universe::fnv1a(key.underlying.as_str()) as u32;

    // THE ADVICE USED TO BE WRONG FOR SIX OF THE SEVEN CAUSES.
    //
    // This read "is not in the store for {vendor}: {why}. Nothing was read. Pull
    // that instrument-month first." `open_existing` fails for absence, but also
    // for `Locked` (ANOTHER WRITER HOLDS THIS MONTH), `CounterExceedsFile` (a
    // torn file), a symbol or timeframe mismatch, and plain permission or I/O
    // errors. Every one of them was reported as absence and answered with "pull
    // it", which for a lock is the opposite of what the operator should do.
    //
    // The lock case is not hypothetical and is the likeliest of the seven: it is
    // what a sweep started from the browser hits while a pull is still writing.
    // Following the old advice — pull it again — extends the very lock that
    // caused it.
    //
    // `{why}` already carries the real cause; what was missing was permission to
    // read it as something other than "absent".
    let file = BarFile::open_existing(root, path, symbol_id).map_err(|why| {
        let missing = matches!(why, StoreError::Missing { .. });
        let message = format!(
            "{underlying} {rung_name} {year}-{month:02} could not be read from \
             the store for {}: {why}. Nothing was read. If that reason is \
             absence, pull the instrument-month. IF IT NAMES A LOCK, A WRITER \
             HOLDS IT -- a pull is in flight, and sweeping now would silently \
             leave this month out of the sample. Wait for the pull, then rerun.",
            vendor.as_str()
        );
        if missing {
            LoadFailure::Missing(message)
        } else {
            LoadFailure::Refused(message)
        }
    })?;

    // RESERVED ONCE, FROM THE HEADER'S OWN COUNT. `records()` is a field read,
    // not a walk, so this is one allocation for a known length rather than a
    // doubling per bar.
    let n = file.records();
    if let Some(remaining) = remaining_records
        && n > remaining
    {
        return Err(LoadFailure::Refused(format!(
            "{underlying} {rung_name} {year}-{month:02} declares {n} committed records, exceeding the {remaining}-record remainder of the explicit span ceiling before allocation. Nothing was read"
        )));
    }
    let capacity = usize::try_from(n).map_err(|_| {
        LoadFailure::Refused(format!(
            "{underlying} {rung_name} {year}-{month:02} declares {n} committed records, which cannot fit this machine's address space. Nothing was allocated"
        ))
    })?;
    let mut bars = Vec::new();
    bars.try_reserve_exact(capacity).map_err(|why| {
        LoadFailure::Refused(format!(
            "{underlying} {rung_name} {year}-{month:02} could not reserve space for its {n} committed records: {why}. Nothing was read"
        ))
    })?;
    // THE CHARTER'S NON-REGULAR SESSIONS ARE WITHHELD HERE, IN ONE PLACE.
    //
    // `load` and `load_span` — and therefore `load_daily_context`,
    // `load_exact_minute_context` and both of their bounded twins — all decode
    // through this function, so the signal rung, the exact one-minute `GapFib`
    // context and the one-minute execution path are filtered by one act. That
    // is the whole reason it is here and not at each sweep entry point: a day
    // dropped from the signal series and kept in the minute series would be a
    // worse defect than the refusal this removes, and there is no arrangement
    // of six call sites in which one cannot be forgotten.
    //
    // Reserved before the filter, deliberately: `n` is the committed record
    // count and the reservation is exact for it, so a month that gives up a
    // disaster-recovery Saturday over-reserves by that day's bars and allocates
    // once rather than twice. See `SWEPT_SERIES_CALENDAR_POLICY` for what the
    // rule is and why `1day` is exempt from it.
    // Compared as a TYPE, not as its name. Two rungs cannot share a string
    // today -- `Timeframe::KNOWN`'s ten names are distinct -- but nothing in the
    // workspace asserts that, and a future rung named "1day" at a different
    // width would silently escape this filter while `==` would still catch it.
    let filtered = timeframe != Timeframe::DAY_1;
    let mut excluded = CalendarExclusion::none();
    for i in 0..n {
        let bar = file.read_record(i).map_err(|why| {
            LoadFailure::Refused(format!("record {i} of {n} could not be read: {why}"))
        })?;
        if filtered && excluded.excludes(bar.ts_micros) {
            // CHECKED, THEN DROPPED. Never dropped unchecked -- see
            // `refuse_uncalendared_withheld_bar` for the measured defect that
            // would otherwise vanish here.
            refuse_uncalendared_withheld_bar(i, bar.ts_micros, timeframe.secs())
                .map_err(LoadFailure::Refused)?;
            continue;
        }
        bars.push(Candle {
            ts_micros: bar.ts_micros,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
            open_interest: bar.open_interest,
        });
    }

    Ok(Loaded {
        bars,
        vendor,
        key,
        timeframe: timeframe.as_str(),
        excluded,
    })
}

/// How long one bar of `rung_name` lasts, in microseconds.
///
/// # Taken from the rung, never inferred from the data
///
/// `runner::align` needs this to know when a signal bar CLOSES, which is the
/// instant its mask becomes knowable. The alternative — subtracting one bar's
/// stamp from the next — looks equivalent and is not: a gap between two stamps
/// is a halt, a holiday or a session boundary, not a longer bar. Deriving the
/// length that way would make the deadline move with the data and would place
/// the first entry of every session hours late.
///
/// # Errors
///
/// Every refusal [`rung`] makes, unchanged, so an unknown rung is named the same
/// way here as everywhere else.
pub fn rung_length_micros(rung_name: &str) -> Result<i64, Refusal> {
    let timeframe = rung(rung_name)?;
    Ok(i64::from(timeframe.secs()).saturating_mul(1_000_000))
}
/// A CONTIGUOUS SPAN of real bars, joined across every month it covers.
///
/// # Why this type exists, and why one month was never the unit
///
/// The store keeps one file per `(vendor, instrument, timeframe, MONTH)`, so
/// seven years of one-minute bars is **eighty-four files**. That is a STORAGE
/// layout — it says nothing about what a sweep should cover — and until this
/// type the loader could open exactly one of them, so the largest question the
/// engine could be asked was "what worked in March".
///
/// That is not a smaller version of the real question, it is a DIFFERENT one. A
/// combination that fires on two percent of bars in every single month is not
/// the same as one that fires on two percent of seven years; a trade cannot open
/// in one month and close in the next; and a walk-forward split inside one month
/// tests against days, not against regimes. Sweeping eighty-four months
/// separately and reading the eighty-four answers is not the seven-year answer.
///
/// # What is NOT hidden
///
/// A month the store does not hold is NAMED in [`Self::missing`] and rendered,
/// never skipped quietly. A span with a hole is a shorter sample, not a
/// corrected one, and `CLAUDE.md` §4 bans the fallback that would let it read
/// like a whole one.
#[derive(Clone, Debug)]
pub struct Span {
    /// Every bar in the span, oldest first, monotonic in time across the join.
    pub bars: Vec<Candle>,
    /// Which feed wrote them.
    pub vendor: Vendor,
    /// What they are bars of.
    pub key: InstrumentKey,
    /// The rung, as its canonical directory word.
    pub timeframe: &'static str,
    /// Months the range covers, whether or not the store holds them.
    pub asked: u32,
    /// Months that were actually opened and read.
    pub found: u32,
    /// Months the range covers that the store does not hold, in order.
    ///
    /// Rendered by the caller. A hole moves every figure computed over the span
    /// and the operator has to see it to know that.
    pub missing: Vec<(u16, u8)>,
    /// Which charter non-regular sessions this span withheld, summed over every
    /// month it opened. Always [`CalendarExclusion::none`] on `1day`.
    ///
    /// Rendered by the caller for the same reason `missing` is: a span that
    /// dropped a disaster-recovery Saturday is a SHORTER sample, not a corrected
    /// one, and `CLAUDE.md` §3 rule 2 forbids that being invisible.
    pub excluded: CalendarExclusion,
}

impl Span {
    /// Whether the store held every month the range asked for.
    #[must_use]
    pub const fn complete(&self) -> bool {
        self.missing.is_empty()
    }
}

/// The month after this one, or `None` past December 9999.
///
/// A free function rather than a method on `YearMonth`, because that type lives
/// in `crates/store` and this crate does not own it.
const fn next_month(year: u16, month: u8) -> Option<(u16, u8)> {
    if month < 12 {
        return Some((year, month + 1));
    }
    if year >= 9999 {
        return None;
    }
    Some((year + 1, 1))
}

/// The longest span this command will assemble, in months.
///
/// # A BOUND, BECAUSE THE ARGUMENTS ARE `u16` AND NOTHING ELSE STOPS THEM
///
/// `YEAR` parses as `u16`, so `audit-range ... 1970 1 65535 1` asks for a range
/// of **763,000 months**. Nothing downstream refuses it: `YearMonth::new` is not
/// reached until a path is built, one per month, and every one of those is a
/// failed file open. The walk would grind for minutes and then produce a refusal
/// saying nothing was found — a hang wearing a result's clothes, which is the
/// §4 fallback in its slowest form.
///
/// A hundred years is far past any span this engine will be asked for — seven
/// years is 84 — and it is a NUMBER, so the refusal can name it rather than
/// saying "too long".
const MAX_SPAN_MONTHS: usize = 1_200;

/// Maximum inclusive day span a V1 calendar receipt will walk.
///
/// Derived from the existing [`MAX_SPAN_MONTHS`] input contract at the widest
/// possible civil month, not introduced as a second independent operator limit.
const MAX_CALENDAR_RECEIPT_DAYS_V1: usize = MAX_SPAN_MONTHS * 31;

/// Every month from `from` up to and including `to`, oldest first.
///
/// Bounded twice over: by [`MAX_SPAN_MONTHS`] before the walk starts, and by
/// `next_month` refusing past 9999-12 inside it.
pub(crate) fn months_between(from: (u16, u8), to: (u16, u8)) -> Result<Vec<(u16, u8)>, Refusal> {
    // THE ENDPOINTS ARE MONTHS, AND THAT IS CHECKED HERE RATHER THAN DISCOVERED.
    //
    // `MONTH` parses as `u8`, so 13 through 255 arrive intact. Without this
    // guard they walk straight into the loop, `load` builds no path for them,
    // and `load_span` files them under `missing` -- so `audit-range ... 2019 13
    // 2026 8` would report "2019-13 is missing from the store". It is not
    // missing. It is not a month. Reporting a malformed argument as absent data
    // sends the operator to a pull that can never fix it, which is the §4
    // fallback that hides a failure in its most expensive form.
    //
    // Only the ENDPOINTS need checking: every month between them comes from
    // `next_month`, which yields 1..=12 by construction.
    for (label, (y, m)) in [("FROM", from), ("TO", to)] {
        if m == 0 || m > 12 {
            return Err(format!(
                "{label} month {m} is not a month: {y}-{m:02} does not exist. \
                 MONTH is 1..=12. Nothing was read."
            ));
        }
    }
    if (from.0, from.1) > (to.0, to.1) {
        return Err(format!(
            "the range runs backwards: {}-{:02} is after {}-{:02}. Give FROM \
             first and TO second.",
            from.0, from.1, to.0, to.1
        ));
    }
    // COUNTED BEFORE IT IS WALKED. `to.0 - from.0` is at most 65,535 years, so
    // the product cannot overflow a `usize` on any target this builds for, and
    // the check happens before a single month is pushed.
    let months = usize::from(to.0.saturating_sub(from.0))
        .saturating_mul(12)
        .saturating_add(usize::from(to.1))
        .saturating_sub(usize::from(from.1))
        .saturating_add(1);
    if months > MAX_SPAN_MONTHS {
        return Err(format!(
            "{}-{:02}..{}-{:02} is {months} months. The longest span this \
             command assembles is {MAX_SPAN_MONTHS}. Nothing was read.",
            from.0, from.1, to.0, to.1
        ));
    }
    let mut out = Vec::with_capacity(months);
    let mut at = from;
    loop {
        out.push(at);
        if at == to {
            break;
        }
        let Some(next) = next_month(at.0, at.1) else {
            return Err(format!(
                "the range ran past 9999-12 before reaching {}-{:02}",
                to.0, to.1
            ));
        };
        at = next;
    }
    Ok(out)
}

/// Loads every month in `from ..= to` as ONE series.
///
/// # The join is checked, not assumed
///
/// Two files opened in order are two files, and nothing about the filesystem
/// guarantees the last bar of one precedes the first bar of the next. The whole
/// engine assumes a strictly increasing series: `Column::build` folds bar by bar
/// and `CLAUDE.md` §3 rule 7's no-look-ahead property is held by that shape, so
/// a series that steps backwards at a join would fold a later bar into an
/// earlier state and no test downstream would catch it.
///
/// So the boundary is COMPARED and a non-increasing step REFUSES. That is a
/// corrupt or mis-keyed store and the operator's next action is to look at the
/// files, not to read a number computed over them.
///
/// # Errors
///
/// A backwards range, a bad instrument or rung, a non-increasing join, or an
/// unreadable record. A month the store simply does not hold is NOT an error —
/// it is recorded in [`Span::missing`] and the span continues, because a
/// seven-year request with one month un-pulled should return six years and
/// eleven months and say so, not refuse everything.
///
/// # Cost
///
/// One `open_existing` per month and one `read_record` per bar, each O(1). The
/// month allocation is reserved from that month's validated header and the
/// assembled destination grows with one fallible exact reserve per admitted
/// month. Total work is O(months + total bars), which is the size of the answer and not a
/// per-operation cost: `CLAUDE.md` §3 rule 4 governs bar lookup, condition
/// lookup, mask evaluation, duplicate rejection and result append, and this is
/// none of them. Nothing here scans, sorts or searches.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
pub fn load_span(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    from: (u16, u8),
    to: (u16, u8),
) -> Result<Span, Refusal> {
    load_span_with_optional_bound(root, vendor, underlying, rung_name, from, to, None)
}

/// Load a stored span with an explicit committed-record ceiling.
///
/// Unlike [`load_span`], this door refuses a month from its validated header
/// count before allocating or reading that month's records when the assembled
/// answer would exceed `bound`.  It is the required primitive for later
/// pre-admission source loading; the ordinary operator-facing loader remains
/// available for its existing compatibility callers.
///
/// # Cost
///
/// O(M + B) time for M requested months and B admitted bars, and O(M + B)
/// returned space for the requested/missing month ledgers plus bars. Header
/// admission is O(1) per month. This is a resource ceiling, not a claim that
/// assembling the span is O(1).
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
///
/// # Errors
///
/// Every [`load_span`] refusal plus a named pre-allocation refusal when a
/// committed month would cross the explicit record ceiling.
pub fn load_span_bounded(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    from: (u16, u8),
    to: (u16, u8),
    bound: StoredSpanLoadBoundV1,
) -> Result<Span, Refusal> {
    load_span_with_optional_bound(root, vendor, underlying, rung_name, from, to, Some(bound))
}

/// Everything an empty span must name about the request that produced it.
///
/// Grouped rather than passed as six positional arguments because
/// [`empty_span_refusal`] takes two `&str` and two `(u16, u8)` in a row, and a
/// caller that swapped either pair would compile and report the wrong span.
struct EmptySpan<'a> {
    underlying: &'a str,
    rung_name: &'a str,
    from: (u16, u8),
    to: (u16, u8),
    vendor: Vendor,
    months: usize,
}

/// Why a span came back with no bars, in the operator's words.
///
/// # Why the calendar case is a sentence of its own
///
/// A SPAN EMPTIED BY THE CALENDAR IS NAMED AS THAT, NOT AS AN ABSENCE. Both
/// sentences end in "nothing was read", but the remedies are opposite: an
/// absent month is fixed by pulling it, and a month present but wholly
/// withheld is not a store defect at all. Telling an operator to pull a month
/// that is already on disk would send them to fix a store that is correct —
/// the same objection `load_classified_with_ceiling` answers for a lock.
///
/// The withheld days are NAMED, not merely counted, for the reason
/// [`CalendarExclusion::day_numbers`] carries: a count cannot be checked
/// against the charter, and `CLAUDE.md` §3 rule 2 is about a scope change
/// being visible rather than tallied.
fn empty_span_refusal(span: &EmptySpan<'_>, excluded: CalendarExclusion) -> Refusal {
    let &EmptySpan {
        underlying,
        rung_name,
        from,
        to,
        vendor,
        months,
    } = span;
    if excluded.is_empty() {
        return format!(
            "{underlying} {rung_name} {}-{:02}..{}-{:02} holds no bars for {}: \
             all {months} month(s) are absent from the store. Nothing was read.",
            from.0,
            from.1,
            to.0,
            to.1,
            vendor.as_str(),
        );
    }
    let named = excluded
        .day_numbers()
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{underlying} {rung_name} {}-{:02}..{}-{:02} for {} holds bars only on {} charter \
         non-regular IST session(s) ({named}), and policy {SWEPT_SERIES_CALENDAR_POLICY} keeps \
         those out of every swept series. {} bar(s) were withheld and none remain. \
         Nothing was swept.",
        from.0,
        from.1,
        to.0,
        to.1,
        vendor.as_str(),
        excluded.days(),
        excluded.bars(),
    )
}

fn load_span_with_optional_bound(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    rung_name: &str,
    from: (u16, u8),
    to: (u16, u8),
    bound: Option<StoredSpanLoadBoundV1>,
) -> Result<Span, Refusal> {
    let timeframe = rung(rung_name)?;
    let key = swept_index(underlying)?;
    let wanted = months_between(from, to)?;

    let mut bars: Vec<Candle> = Vec::new();
    let mut missing: Vec<(u16, u8)> = Vec::new();
    let mut found: u32 = 0;
    let mut admitted_records = 0_u64;
    let mut excluded = CalendarExclusion::none();

    for &(year, month) in &wanted {
        let remaining_records = bound
            .map(StoredSpanLoadBoundV1::max_records)
            .map(|maximum| {
                maximum.checked_sub(admitted_records).ok_or_else(|| {
                    format!(
                        "stored span already admitted {admitted_records} records, beyond its explicit {maximum}-record ceiling. Nothing else was read"
                    )
                })
            })
            .transpose()?;
        match load_classified_with_ceiling(
            root,
            vendor,
            underlying,
            rung_name,
            year,
            month,
            remaining_records,
        ) {
            Err(LoadFailure::Missing(_)) => missing.push((year, month)),
            Err(LoadFailure::Refused(why)) => return Err(why),
            Ok(one) => {
                // THE JOIN, CHECKED. Compared against the last bar already held
                // rather than against the previous month's own last bar, so a
                // hole in the middle does not let a backwards step through.
                if let (Some(prev), Some(first)) = (bars.last(), one.bars.first())
                    && first.ts_micros <= prev.ts_micros
                {
                    return Err(format!(
                        "the span steps backwards at {year}-{month:02}: that \
                         month's first bar is stamped {} and the bar before it \
                         is stamped {}. A series that is not strictly \
                         increasing folds a later bar into an earlier state, so \
                         nothing was swept. Check the store for that month.",
                        first.ts_micros, prev.ts_micros
                    ));
                }
                found = found.saturating_add(1);
                // FOLDED PER MONTH, NOT RE-DERIVED FROM THE ASSEMBLED SPAN.
                // The bars a charter day contributed are gone by the time this
                // runs, so a later pass over `bars` could not find them; the
                // month's own census is the only surviving record of what it
                // withheld.
                excluded.absorb(one.excluded);
                let month_records = u64::try_from(one.bars.len()).map_err(|_| {
                    format!(
                        "{underlying} {rung_name} {year}-{month:02} decoded record count does not fit u64"
                    )
                })?;
                admitted_records = admitted_records.checked_add(month_records).ok_or_else(|| {
                    format!(
                        "{underlying} {rung_name} assembled record count overflowed u64 at {year}-{month:02}"
                    )
                })?;
                // `try_reserve` AND NOT `try_reserve_exact`, and the difference
                // is the whole cost of assembling a span.
                //
                // `try_reserve_exact` reserves exactly the additional capacity
                // asked for, so after every month `capacity == len` and the NEXT
                // month's reservation reallocates and memcpys everything
                // accumulated so far. Total bytes copied is B x M / 2 rather
                // than B: over the operator's 81-month one-minute span that is
                // roughly 68 MB of bars copied about forty times, some 2.7 GB of
                // memcpy, twice per rung because `one_rung` loads the span again.
                //
                // It also made this function's own stated cost false. The `Cost`
                // section above says "O(M + B) time", which is what `try_reserve`
                // delivers -- amortised growth, each bar moved a constant number
                // of times -- and not what exact reservation did.
                //
                // The two `try_reserve_exact` calls elsewhere in this module are
                // correct and stay: they reserve a capacity computed ONCE for a
                // vector that is then filled, where exact is the right ask and
                // there is no second reservation to trigger a copy.
                bars.try_reserve(one.bars.len()).map_err(|why| {
                    format!(
                        "{underlying} {rung_name} could not reserve the assembled span for {year}-{month:02}'s {month_records} committed records: {why}. Nothing was swept"
                    )
                })?;
                bars.extend(one.bars);
            }
        }
    }

    if bars.is_empty() {
        return Err(empty_span_refusal(
            &EmptySpan {
                underlying,
                rung_name,
                from,
                to,
                vendor,
                months: wanted.len(),
            },
            excluded,
        ));
    }

    Ok(Span {
        bars,
        vendor,
        key,
        timeframe: timeframe.as_str(),
        asked: u32::try_from(wanted.len()).unwrap_or(u32::MAX),
        found,
        missing,
        excluded,
    })
}

/// The calendar month immediately before `at`.
fn previous_month(at: (u16, u8)) -> Result<(u16, u8), Refusal> {
    let (year, month) = at;
    if !(1..=12).contains(&month) {
        return Err(format!("{year}-{month:02} is not a month"));
    }
    if month > 1 {
        return Ok((year, month - 1));
    }
    let Some(previous_year) = year.checked_sub(1) else {
        return Err(
            "year zero January has no preceding month for daily-reference warm-up".to_owned(),
        );
    };
    Ok((previous_year, 12))
}

/// Classify one observed day under the canonical NSE calendar and the
/// identity-bound previous-day exclusion policy.
fn daily_eligibility_of(day: i64) -> Result<DailyEligibility, Refusal> {
    let explicitly_excluded = CHARTER_NON_REGULAR_IST_DAYS.contains(&day);
    match pull::calendar::kind_of(day) {
        DayKind::Open(_) => Ok(if explicitly_excluded {
            DailyEligibility::Excluded
        } else {
            DailyEligibility::Eligible
        }),
        DayKind::OpenLengthUnmeasured if explicitly_excluded => Ok(DailyEligibility::Excluded),
        DayKind::OpenLengthUnmeasured => Err(format!(
            "canonical NSE calendar marks IST day {day} open with unmeasured session length, but the identity-bound daily exclusion policy does not exclude it; no regular-session eligibility was invented"
        )),
        DayKind::Closed => Err(format!(
            "stored reference evidence contains IST day {day}, which the canonical NSE calendar measures as closed; the stray row was not treated as a trading session"
        )),
        DayKind::Unmeasured => Err(format!(
            "stored reference evidence contains IST day {day}, which is outside the canonical NSE calendar's measured range; session eligibility was not guessed"
        )),
    }
}

/// Turn a complete stored one-day span into explicit causal reference records.
fn daily_context_from_span(daily: Span, signal: &[Candle]) -> Result<DailyContext, Refusal> {
    let Some(first_signal) = signal.first() else {
        return Err(
            "the signal span is empty, so no previous-day reference can be selected".to_owned(),
        );
    };
    let Some(last_signal) = signal.last() else {
        return Err(
            "the signal span is empty, so no previous-day reference can be selected".to_owned(),
        );
    };
    require_complete_daily_span(&daily)?;

    let first_signal_day = indicators::ist_day(first_signal.ts_micros);
    let last_signal_day = indicators::ist_day(last_signal.ts_micros);
    let capacity = daily.bars.len();
    let mut bars = Vec::new();
    bars.try_reserve_exact(capacity).map_err(|why| {
        format!(
            "stored 1day reference conversion could not reserve {capacity} causal bar record(s): {why}. Nothing was swept"
        )
    })?;
    let mut references = Vec::new();
    references.try_reserve_exact(capacity).map_err(|why| {
        format!(
            "stored 1day reference conversion could not reserve {capacity} typed reference record(s): {why}. Nothing was swept"
        )
    })?;
    let mut eligibility = Vec::new();
    eligibility.try_reserve_exact(capacity).map_err(|why| {
        format!(
            "stored 1day reference conversion could not reserve {capacity} eligibility record(s): {why}. Nothing was swept"
        )
    })?;
    for bar in daily.bars {
        let day = indicators::ist_day(bar.ts_micros);
        // A same-day record is not sealed at the instant an intraday signal is
        // evaluated, and a future record is look-ahead.  They are omitted from
        // the offered reference stream rather than relying on the evaluator to
        // ignore bytes the run identity then misleadingly claims it consumed.
        if day >= last_signal_day {
            continue;
        }
        let decision = daily_eligibility_of(day)?;
        let reference = DailyReference::new(bar, decision).map_err(|why| {
            format!(
                "stored 1day record at timestamp {} is not usable reference evidence: {why:?}",
                bar.ts_micros
            )
        })?;
        bars.push(bar);
        references.push(reference);
        eligibility.push(u8::from(decision == DailyEligibility::Eligible));
    }

    if !references.iter().any(|reference| {
        reference.ist_day() < first_signal_day
            && reference.eligibility() == DailyEligibility::Eligible
    }) {
        return Err(format!(
            "the first signal IST day {first_signal_day} has no eligible stored 1day record strictly before it. Same-day OHLCV, a coarse reconstruction, and a guessed holiday are all forbidden; load the preceding daily history"
        ));
    }

    // Every regular signal session that is followed by another observed signal
    // session must itself have a stored daily record.  The signal stream is the
    // evidence that the session existed; no exchange holiday is invented here.
    let mut previous_signal_day = None;
    let mut daily_cursor = 0_usize;
    for bar in signal {
        let day = indicators::ist_day(bar.ts_micros);
        if previous_signal_day.is_some_and(|(previous, _)| previous == day) {
            continue;
        }
        let current_eligibility = daily_eligibility_of(day)?;
        if let Some((previous, previous_eligibility)) = previous_signal_day
            && previous_eligibility == DailyEligibility::Eligible
        {
            while references
                .get(daily_cursor)
                .is_some_and(|reference| reference.ist_day() < previous)
            {
                daily_cursor = daily_cursor.saturating_add(1);
            }
            if references
                .get(daily_cursor)
                .is_none_or(|reference| reference.ist_day() != previous)
            {
                return Err(format!(
                    "signal bars prove IST day {previous} traded, but the same-feed same-instrument stored 1day stream has no record for it before day {day}. The older anchor was not silently reused"
                ));
            }
        }
        previous_signal_day = Some((day, current_eligibility));
    }

    Ok(DailyContext {
        bars,
        references,
        eligibility,
        asked: daily.asked,
        found: daily.found,
    })
}

fn require_complete_daily_span(daily: &Span) -> Result<(), Refusal> {
    if daily.complete() {
        return Ok(());
    }
    let missing = daily
        .missing
        .iter()
        .map(|(year, month)| format!("{year}-{month:02}"))
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "the stored 1day reference stream is incomplete: missing {missing}. A missing daily month cannot be called a holiday or reconstructed from an intraday rung, so nothing was swept"
    ))
}

/// Find the latest canonical session before `first_signal_day` that the
/// previous-day policy allows to seed `GapFib`.
fn prior_accepted_session(first_signal_day: i64) -> Result<(i64, Session), Refusal> {
    let Some(mut day) = first_signal_day.checked_sub(1) else {
        return Err(
            "the first signal IST day has no representable prior day for GapFib".to_owned(),
        );
    };
    loop {
        match pull::calendar::kind_of(day) {
            DayKind::Open(session) => {
                if !CHARTER_NON_REGULAR_IST_DAYS.contains(&day) {
                    return Ok((day, session));
                }
            }
            DayKind::OpenLengthUnmeasured => {
                if !CHARTER_NON_REGULAR_IST_DAYS.contains(&day) {
                    return Err(format!(
                        "canonical NSE calendar marks prior IST day {day} open with unmeasured length, but the identity-bound exclusion policy does not exclude it; the GapFib session was not guessed"
                    ));
                }
            }
            DayKind::Closed => {}
            DayKind::Unmeasured => {
                return Err(format!(
                    "the first signal IST day {first_signal_day} has no prior accepted session inside the canonical NSE calendar's measured range; GapFib history was not guessed"
                ));
            }
        }
        let Some(previous) = day.checked_sub(1) else {
            return Err(format!(
                "the first signal IST day {first_signal_day} has no representable prior accepted session for GapFib"
            ));
        };
        day = previous;
    }
}

/// Require one stored minute to belong to a measured NSE session window.
fn require_canonical_minute(index: usize, bar: &Candle) -> Result<(i64, u16), Refusal> {
    let (day, minute) = exact_ist_minute_v1(bar.ts_micros).map_err(|why| {
        format!(
            "stored exact 1min GapFib record {index} at timestamp {} has no exact IST minute: {why}",
            bar.ts_micros
        )
    })?;
    match pull::calendar::kind_of(day) {
        DayKind::Open(session) if session.expects(minute) => Ok((day, minute)),
        DayKind::Open(_) => Err(format!(
            "stored exact 1min GapFib record {index} at timestamp {} is minute {minute} outside every measured NSE session window on IST day {day}",
            bar.ts_micros
        )),
        DayKind::Closed => Err(format!(
            "stored exact 1min GapFib record {index} at timestamp {} is on canonical measured-closed IST day {day}; a Sunday or closed day was not treated as a prior session",
            bar.ts_micros
        )),
        DayKind::OpenLengthUnmeasured => Err(format!(
            "stored exact 1min GapFib record {index} at timestamp {} is on open IST day {day} whose session length is unmeasured; minute-window authority was not invented",
            bar.ts_micros
        )),
        DayKind::Unmeasured => Err(format!(
            "stored exact 1min GapFib record {index} at timestamp {} is outside the canonical NSE calendar's measured range on IST day {day}",
            bar.ts_micros
        )),
    }
}

/// Load the same-feed, same-instrument stored one-day stream needed by an
/// intraday signal span.
///
/// The preceding calendar month is included as warm-up evidence so the first
/// requested signal day cannot borrow its own OHLC or start with an invented
/// anchor.  Every daily month must be present; unlike the ordinary signal-span
/// loader, this door cannot safely interpret a hole as a holiday.
///
/// # Errors
///
/// Every [`load_span`] refusal, an absent daily month, malformed daily OHLCV,
/// or any observed regular signal day whose daily record is missing.
pub fn load_daily_context(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    signal_months: ((u16, u8), (u16, u8)),
    signal: &[Candle],
) -> Result<DailyContext, Refusal> {
    let (from, to) = signal_months;
    let warm_from = previous_month(from)?;
    let daily = load_span(root, vendor, underlying, "1day", warm_from, to)?;
    daily_context_from_span(daily, signal)
}

/// Load stored one-day reference evidence under a pre-allocation record ceiling.
///
/// This is the bounded production counterpart to [`load_daily_context`].  It
/// deliberately reuses the same private conversion boundary after the bounded
/// span reader has admitted every committed month header.  There is therefore
/// one daily/calendar interpretation, while `bound` remains protective rather
/// than a row count recorded after an unbounded allocation.
///
/// # Cost
///
/// O(M + B) time and O(M + B) returned space for M requested months and B
/// admitted daily records.  The committed record ceiling is checked in O(1)
/// per month before that month's records are allocated.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
///
/// # Errors
///
/// Every [`load_daily_context`] semantic refusal plus the named
/// pre-allocation refusals from [`load_span_bounded`].
pub fn load_daily_context_bounded(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    signal_months: ((u16, u8), (u16, u8)),
    signal: &[Candle],
    bound: StoredSpanLoadBoundV1,
) -> Result<DailyContext, Refusal> {
    let (from, to) = signal_months;
    let warm_from = previous_month(from)?;
    let daily = load_span_bounded(root, vendor, underlying, "1day", warm_from, to, bound)?;
    daily_context_from_span(daily, signal)
}

/// Validate a complete stored one-minute span as `GapFib` context.
fn exact_minute_context_from_span(
    minute: Span,
    signal: &[Candle],
) -> Result<ExactMinuteContext, Refusal> {
    let Some(first_signal) = signal.first() else {
        return Err(
            "the signal span is empty, so exact one-minute GapFib evidence cannot be aligned"
                .to_owned(),
        );
    };
    if !minute.complete() {
        let missing = minute
            .missing
            .iter()
            .map(|(year, month)| format!("{year}-{month:02}"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "the stored exact 1min GapFib stream is incomplete: missing {missing}. A missing minute month cannot be reconstructed from the signal rung or called a holiday, so nothing was swept"
        ));
    }

    let first_signal_day = indicators::ist_day(first_signal.ts_micros);
    let (prior_session_day, prior_session) = prior_accepted_session(first_signal_day)?;
    let mut previous = None;
    let mut prior_session_bars = 0_usize;
    let mut third_last = None;
    let mut second_last = None;
    let mut last = None;
    for (index, bar) in minute.bars.iter().enumerate() {
        bar.check().map_err(|why| {
            format!(
                "stored exact 1min GapFib record {index} at timestamp {} is malformed OHLCV: {why:?}",
                bar.ts_micros
            )
        })?;
        let (day, minute_of_day) = require_canonical_minute(index, bar)?;
        if let Some(prior) = previous {
            let delta = bar.ts_micros.saturating_sub(prior);
            if delta < 60_000_000 || delta.rem_euclid(60_000_000) != 0 {
                return Err(format!(
                    "stored exact 1min GapFib cadence is malformed at record {index}: {prior} then {}",
                    bar.ts_micros
                ));
            }
        }
        previous = Some(bar.ts_micros);
        if day == prior_session_day {
            prior_session_bars = prior_session_bars.saturating_add(1);
            third_last = second_last;
            second_last = last;
            last = Some(minute_of_day);
        }
    }
    if prior_session_bars < 3 {
        return Err(format!(
            "the latest accepted exact 1min session before signal day {first_signal_day} is IST day {prior_session_day}, but it holds only {prior_session_bars} bar(s); GapFib requires its final three and no coarse or daily substitute was used"
        ));
    }
    let final_window = prior_session
        .windows
        .iter()
        .take(usize::from(prior_session.count))
        .next_back()
        .copied()
        .ok_or_else(|| {
            format!(
                "canonical NSE calendar returned no window for accepted prior IST session {prior_session_day}"
            )
        })?;
    let expected_second = final_window.to.checked_sub(1).ok_or_else(|| {
        format!(
            "accepted prior IST session {prior_session_day} has no two terminal minutes for GapFib"
        )
    })?;
    let expected_third = expected_second.checked_sub(1).ok_or_else(|| {
        format!(
            "accepted prior IST session {prior_session_day} has no three terminal minutes for GapFib"
        )
    })?;
    if expected_third < final_window.from
        || third_last != Some(expected_third)
        || second_last != Some(expected_second)
        || last != Some(final_window.to)
    {
        return Err(format!(
            "the accepted prior exact 1min session on IST day {prior_session_day} does not end with canonical terminal-minute geometry {expected_third}, {expected_second}, {}; observed final three were {third_last:?}, {second_last:?}, {last:?}. Early or truncated bars cannot seed GapFib",
            final_window.to
        ));
    }
    let prior_session_bars = u32::try_from(prior_session_bars).map_err(|_| {
        format!("the accepted prior IST session {prior_session_day} record count does not fit u32")
    })?;

    Ok(ExactMinuteContext {
        bars: minute.bars,
        asked: minute.asked,
        found: minute.found,
        prior_session_day,
        prior_session_bars,
        excluded: minute.excluded,
    })
}

/// Load exact stored one-minute evidence for every signal rung's `GapFib` bits.
///
/// The preceding calendar month is required so the first requested signal day
/// can observe the prior session's final three exact minutes. Every requested
/// month must exist; a hole is not interpreted as a holiday. The overlay itself
/// performs the final exact-close join and therefore also refuses an individual
/// missing closing minute.
///
/// # Errors
///
/// Every [`load_span`] refusal, an absent minute month, malformed OHLCV or
/// cadence, or fewer than three bars in the latest accepted prior session.
pub fn load_exact_minute_context(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    signal_months: ((u16, u8), (u16, u8)),
    signal: &[Candle],
) -> Result<ExactMinuteContext, Refusal> {
    let (from, to) = signal_months;
    let warm_from = previous_month(from)?;
    let minute = load_span(root, vendor, underlying, "1min", warm_from, to)?;
    exact_minute_context_from_span(minute, signal)
}

/// Load exact stored one-minute evidence under a pre-allocation record ceiling.
///
/// This is the bounded production counterpart to
/// [`load_exact_minute_context`].  It routes the bounded span through the same
/// private cadence/prior-session converter, so no second minute or calendar
/// authority is introduced and the explicit ceiling is enforced before any
/// committed month is decoded.
///
/// # Cost
///
/// O(M + B) time and O(M + B) returned space for M requested months and B
/// admitted minute records.  The committed record ceiling is checked in O(1)
/// per month before that month's records are allocated.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
///
/// # Errors
///
/// Every [`load_exact_minute_context`] semantic refusal plus the named
/// pre-allocation refusals from [`load_span_bounded`].
pub fn load_exact_minute_context_bounded(
    root: &Path,
    vendor: Vendor,
    underlying: &str,
    signal_months: ((u16, u8), (u16, u8)),
    signal: &[Candle],
    bound: StoredSpanLoadBoundV1,
) -> Result<ExactMinuteContext, Refusal> {
    let (from, to) = signal_months;
    let warm_from = previous_month(from)?;
    let minute = load_span_bounded(root, vendor, underlying, "1min", warm_from, to, bound)?;
    exact_minute_context_from_span(minute, signal)
}
#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    // The seeding helper computes the store's own symbol id the store's own
    // way. Same reason as the production site above.
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;
    use store::format::Bar;

    /// A store root of this test's own, so `cargo test`'s threads cannot collide.
    ///
    /// The same idiom `crates/store`'s own tests use, tagged per test and
    /// suffixed with the process id.
    fn root(tag: &str) -> std::path::PathBuf {
        let p =
            std::env::temp_dir().join(format!("brutex-cli-stored-{tag}-{}", std::process::id()));
        let _ignored = std::fs::remove_dir_all(&p);
        p
    }

    /// `n` bars on a one-minute grid from the 2026-08-03 open, in paisa.
    fn bars(n: i64) -> Vec<Bar> {
        // 2026-08-03 09:15 IST as epoch micros, so the timestamps land inside
        // the month the path names. A bar outside its own month is a refusal the
        // store makes for itself and is not what these tests are about.
        let open = 1_785_727_500_000_000_i64;
        (0..n)
            .map(|i| Bar {
                ts_micros: open + i * 60_000_000,
                open: 2_500_000 + i,
                high: 2_500_100 + i,
                low: 2_499_900 + i,
                close: 2_500_050 + i,
                volume: 0,
                open_interest: i64::MIN,
            })
            .collect()
    }

    /// Writes `n` bars for one storable NSE index in 2026-08.
    fn seeded_symbol(tag: &str, vendor: Vendor, symbol: &str, n: i64) -> std::path::PathBuf {
        let r = root(tag);
        let key = InstrumentKey::index(Exchange::Nse, symbol).expect("a valid stored index");
        let ym = YearMonth::new(2026, 8).expect("a real month");
        let path = StorePath::for_key(vendor, &key, Timeframe::MINUTE_1, ym, FileKind::Bars)
            .expect("a path for a stored index");
        let id = brutex_core::universe::fnv1a(symbol) as u32;
        let mut file = BarFile::open_or_create(&r, path, id).expect("a fresh month opens");
        // An empty batch is refused by the store as `EmptyBatch`, correctly — so
        // thezero -bar case is a file that was created and never appended to, which
        // is exactly the state a reached-but-empty month leaves behind.
        if n > 0 {
            file.append(&bars(n)).expect("and takes its bars");
        }
        r
    }

    /// Writes `n` bars for NIFTY 1min 2026-08 under `vendor`, and returns the root.
    fn seeded(tag: &str, vendor: Vendor, n: i64) -> std::path::PathBuf {
        seeded_symbol(tag, vendor, "NIFTY", n)
    }

    #[test]
    fn a_stored_month_loads_with_its_provenance_attached() {
        let r = seeded("happy", Vendor::Dhan, 5);
        let got = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect("it is in the store");

        assert_eq!(got.bars.len(), 5, "every record is read");
        assert_eq!(
            got.vendor,
            Vendor::Dhan,
            "the feed comes back with the bars"
        );
        assert_eq!(got.timeframe, "1min", "and so does the rung");
        assert_eq!(
            got.key.underlying.as_str(),
            "NIFTY",
            "and the instrument it is bars of"
        );
    }

    #[test]
    fn every_field_survives_the_crossing() {
        // `store::format::Bar` and `indicators::Candle` are two names for one
        // record. Seven fields, copied by hand because the crates that own them
        // may not name each other, so seven fields are asserted by hand.
        let r = seeded("fields", Vendor::Groww, 3);
        let got = load(&r, Vendor::Groww, "NIFTY", "1min", 2026, 8).expect("stored");
        let want = bars(3);
        for (i, (c, b)) in got.bars.iter().zip(want.iter()).enumerate() {
            assert_eq!(c.ts_micros, b.ts_micros, "bar {i} ts");
            assert_eq!(c.open, b.open, "bar {i} open");
            assert_eq!(c.high, b.high, "bar {i} high");
            assert_eq!(c.low, b.low, "bar {i} low");
            assert_eq!(c.close, b.close, "bar {i} close");
            assert_eq!(c.volume, b.volume, "bar {i} volume");
            assert_eq!(
                c.open_interest, b.open_interest,
                "bar {i} open interest — the i64::MIN sentinel must cross intact"
            );
        }
    }

    #[test]
    fn the_bars_come_back_oldest_first() {
        let r = seeded("order", Vendor::Dhan, 8);
        let got = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect("stored");
        assert!(
            got.bars.windows(2).all(|w| w[0].ts_micros < w[1].ts_micros),
            "the sweep reads bars 0..N and a reversed column would silently invert every condition"
        );
    }

    #[test]
    fn a_month_that_was_never_pulled_says_so_and_names_the_pull() {
        let r = root("absent");
        let why = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect_err("nothing is there");
        // THE ABSENCE IS NAMED BY THE STORE, NOT BY THIS SENTENCE.
        //
        // This asserted the phrase "not in the store", which the wrapper used to
        // say about EVERY failure -- a lock, a torn file, a permission error and
        // a genuine absence alike. The wrapper no longer asserts absence it
        // cannot know; it defers to `{why}`, which for this fixture is the
        // store's own "does not exist".
        //
        // So the test now checks what is actually true here — that the file is
        // reported missing — rather than a wording that was a guess in six cases
        // out of seven.
        assert!(
            why.contains("does not exist"),
            "it names the absence, from the store's own error: {why}"
        );
        assert!(
            why.contains("pull the instrument-month"),
            "and tells the operator what to do about a real absence: {why}"
        );
        assert!(
            why.contains("A WRITER HOLDS IT"),
            "while naming the case where pulling again would be exactly wrong: \
             {why}"
        );
        assert!(why.contains("dhan"), "and which feed's store: {why}");
        // AND THE OTHER ACTION, WHICH IS THE OPPOSITE ONE.
        //
        // This asserted "Pull that instrument-month first" -- the old wrapper's
        // single, unconditional instruction. It was right for absence and wrong
        // for the six other causes `open_existing` can report, worst of all a
        // lock: pulling again extends the very lock that blocked the read.
        //
        // Both branches are now stated and both are asserted, so a future edit
        // cannot quietly drop the half that keeps an operator from making it
        // worse.
        assert!(
            why.contains("Wait for the pull, then rerun"),
            "and the action for a lock, which is to wait rather than to pull: \
             {why}"
        );
    }

    #[test]
    fn a_rung_the_store_does_not_carry_lists_the_ones_it_does() {
        let why = load(&root("rung"), Vendor::Dhan, "NIFTY", "1hour", 2026, 8)
            .expect_err("1hour is not a rung");
        assert!(why.contains("`1hour` is not a rung"), "{why}");
        assert!(
            why.contains("60min"),
            "an operator who typed 1hour needs to be told the word is 60min: {why}"
        );
        assert!(
            why.contains("1day"),
            "the list is every rung, not a sample: {why}"
        );
    }

    #[test]
    fn an_existing_reference_index_is_refused_before_its_store_file_is_read() {
        // INDIAVIX is intentionally storable and reference-only. Seed a real,
        // readable file so absence cannot accidentally be the protection. The
        // shared load door must apply core's authoritative sweep predicate
        // before touching those bytes.
        let r = seeded_symbol("reference-only", Vendor::Dhan, "INDIAVIX", 3);
        let why = load(&r, Vendor::Dhan, "INDIAVIX", "1min", 2026, 8)
            .expect_err("reference data must never enter a sweep");
        assert!(
            why.contains("INDIAVIX"),
            "the refusal quotes what was asked: {why}"
        );
        assert!(
            why.contains("storable but not sweepable"),
            "the refusal distinguishes storage from the engine surface: {why}"
        );
        assert!(
            why.contains("NIFTY, BANKNIFTY"),
            "the exact surface is rendered from core's authoritative list: {why}"
        );
        assert!(
            !why.contains("does not exist"),
            "the seeded file exists and absence must not masquerade as the guard: {why}"
        );
    }

    #[test]
    fn exactly_the_two_core_sweep_keys_cross_the_stored_loader_guard() {
        assert_eq!(
            InstrumentKey::SWEPT,
            [(Exchange::Nse, "NIFTY"), (Exchange::Nse, "BANKNIFTY")],
            "the test follows core's exact public engine surface"
        );
        for (_, symbol) in InstrumentKey::SWEPT {
            let why = load(&root(symbol), Vendor::Dhan, symbol, "1min", 2026, 8)
                .expect_err("the fixture deliberately has no file");
            assert!(
                why.contains("does not exist"),
                "{symbol} crossed the sweep guard and reached the empty store: {why}"
            );
            assert!(
                !why.contains("not an instrument this engine sweeps"),
                "{symbol} is one of the exact two: {why}"
            );
        }
    }

    /// **A lower-case instrument loads. It used to refuse as file corruption.**
    ///
    /// `swept_index` normalises the symbol for the PATH — `Symbol::new`
    /// upper-cases, because vendors are not consistent about case — while the
    /// header check hashed the caller's RAW string. So `nifty` resolved to
    /// `.../NIFTY/...`, opened the correct file, and was refused by it with
    /// `holds symbol 433894009, not 1196260313`: two numbers naming nothing,
    /// sending an operator to audit a store that was right when they had made a
    /// typo. The bars come back now, and they are the same bars.
    #[test]
    fn a_lower_case_instrument_reads_the_same_month_as_its_canonical_name() {
        let r = seeded("case", Vendor::Dhan, 5);

        let upper = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8)
            .expect("the canonical name is what the fixture wrote");
        let lower = load(&r, Vendor::Dhan, "nifty", "1min", 2026, 8)
            .expect("the same instrument, typed in the case an operator types");

        assert_eq!(
            lower.bars.len(),
            upper.bars.len(),
            "one file, one answer, whatever case the operator happened to type"
        );
        assert_eq!(
            lower.key.underlying.as_str(),
            "NIFTY",
            "and it comes back under the canonical name the store files it as"
        );
        for (a, b) in lower.bars.iter().zip(upper.bars.iter()) {
            assert_eq!(a.ts_micros, b.ts_micros, "the same records, in order");
            assert_eq!(a.close, b.close);
        }
    }

    /// Mixed case too, and it never names two bare hashes at an operator.
    ///
    /// Separate from the equality above because a regression here would still
    /// OPEN the right file. What it would produce is the `holds symbol` refusal,
    /// and that sentence — not the missing bars — is the defect.
    #[test]
    fn a_mixed_case_instrument_never_refuses_with_a_symbol_id_mismatch() {
        let r = seeded("case-mixed", Vendor::Dhan, 3);
        let loaded = load(&r, Vendor::Dhan, "NiFtY", "1min", 2026, 8)
            .expect("the store is correct and the caller only typed a case");
        assert_eq!(loaded.bars.len(), 3, "mixed case reads the same month");
        assert_eq!(loaded.key.underlying.as_str(), "NIFTY");
    }

    /// **An unbounded argument is cut before it reaches the refusal.**
    ///
    /// The refusal is followed by roughly a hundred lines of usage, so echoing
    /// a 10,000-character word whole pushed the one sentence naming the mistake
    /// off the operator's screen with their own typo.
    #[test]
    fn an_enormous_instrument_word_is_clipped_out_of_its_own_refusal() {
        let huge = "N".repeat(10_000);
        let why = load(&root("huge"), Vendor::Dhan, &huge, "1min", 2026, 8)
            .expect_err("nothing that long is an instrument");

        assert!(
            why.contains("is not an instrument this engine sweeps"),
            "it is still refused by the same named sentence as any typo: {why}"
        );
        assert!(
            why.chars().count() < 400,
            "the whole refusal stays readable, and was {} characters: {why}",
            why.chars().count()
        );
        assert!(why.contains('…'), "and says that it cut something: {why}");
        let untruncated = "N".repeat(100);
        assert!(
            !why.contains(untruncated.as_str()),
            "the argument itself is not echoed back whole: {why}"
        );
    }

    /// The rung word is cut the same way, being the same command's next field.
    ///
    /// Clipping one argument of a six-argument command and not its neighbour
    /// would make truncation depend on which word the operator mistyped.
    #[test]
    fn an_enormous_rung_word_is_clipped_out_of_its_own_refusal_too() {
        let huge = "m".repeat(10_000);
        let why = load(&root("huge-rung"), Vendor::Dhan, "NIFTY", &huge, 2026, 8)
            .expect_err("nothing that long is a rung");
        assert!(why.contains("not a rung this store carries"), "{why}");
        assert!(why.contains('…'), "and the word is marked as cut: {why}");
        assert!(why.chars().count() < 400, "and stays readable: {why}");
    }

    /// A word that fits is printed exactly, with nothing appended.
    ///
    /// The other half of the cut: a clip that marked every refusal would teach
    /// an operator to ignore the mark.
    #[test]
    fn a_short_instrument_word_is_quoted_whole_and_unmarked() {
        let why = load(&root("short"), Vendor::Dhan, "RELIANCE", "1min", 2026, 8)
            .expect_err("an equity is not a swept index");
        assert!(why.contains("RELIANCE"), "quoted in full: {why}");
        assert!(!why.contains('…'), "and never marked as cut: {why}");
    }

    #[test]
    fn a_month_that_is_not_a_month_is_refused_before_any_path_is_built() {
        let why =
            load(&root("month"), Vendor::Dhan, "NIFTY", "1min", 2026, 13).expect_err("no month 13");
        assert!(why.contains("2026-13 is not a month"), "{why}");
    }

    #[test]
    fn an_empty_month_loads_as_empty_and_not_as_an_error() {
        // A file that exists and holds nothing is a real answer: the month was
        // reached and had no session. Reporting it as a refusal would send the
        // operator to pull something that is already there.
        let r = seeded("empty", Vendor::Dhan, 0);
        let got = load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8).expect("the file exists");
        assert!(got.bars.is_empty());
        assert_eq!(
            got.vendor,
            Vendor::Dhan,
            "provenance survives an empty month"
        );
    }

    #[test]
    fn two_feeds_are_two_stores_and_one_does_not_answer_for_the_other() {
        // The whole reason the vendor is the first path segment (D-0019). If
        // this ever passes for the wrong feed, a run could be attributed to a
        // broker that never sent the bars.
        let r = seeded("split", Vendor::Dhan, 4);
        assert_eq!(
            load(&r, Vendor::Dhan, "NIFTY", "1min", 2026, 8)
                .expect("dhan is there")
                .bars
                .len(),
            4
        );
        assert!(
            load(&r, Vendor::Groww, "NIFTY", "1min", 2026, 8).is_err(),
            "groww's month was never written and must not resolve to dhan's"
        );
    }

    /// Days since 1970-01-01 for a civil date, by Howard Hinnant's algorithm.
    ///
    /// Written out rather than pulled in: `CLAUDE.md` §2 allows no new
    /// dependency for a test helper, and the store refuses a bar stamped outside
    /// the month its path names — so a multi-month fixture MUST compute a real
    /// timestamp per month rather than reusing one.
    const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The 3rd of that month at the same time of day the single-month fixture
    /// uses, as epoch micros.
    ///
    /// # 03:25 UTC, and the twenty minutes are worth a sentence
    ///
    /// `bars` above hard-codes `1_785_727_500_000_000` and its comment calls it
    /// "2026-08-03 09:15 IST". That constant is 03:25 UTC, which is 08:55 IST --
    /// twenty minutes before the open the comment names. The first draft of this
    /// helper computed 09:15 honestly and disagreed with the constant by exactly
    /// 1,200,000,000 micros, which is how the gap was found.
    ///
    /// This matches the CONSTANT rather than the comment, deliberately: both
    /// fixtures must stamp bars the same way or a span test and a month test
    /// would be measuring different grids. Whether the comment or the constant
    /// is the thing to change is a question about a fixture that predates this
    /// module and is not one a span test should answer by quietly diverging.
    /// Neither value affects any assertion here -- every bar lands inside the
    /// month its path names either way, which is all the store checks.
    const fn opening_micros(year: i64, month: i64) -> i64 {
        (days_from_civil(year, month, 3) * 86_400 + 3 * 3_600 + 25 * 60) * 1_000_000
    }

    /// `n` bars on a one-minute grid from that month's 3rd, on the same
    /// time-of-day grid as `bars`.
    fn bars_in(year: i64, month: i64, n: i64) -> Vec<Bar> {
        let open = opening_micros(year, month);
        (0..n)
            .map(|i| Bar {
                ts_micros: open + i * 60_000_000,
                open: 2_500_000 + i,
                high: 2_500_100 + i,
                low: 2_499_900 + i,
                close: 2_500_050 + i,
                volume: 0,
                open_interest: i64::MIN,
            })
            .collect()
    }

    /// Writes `n` bars into each of `months` for one NIFTY timeframe.
    fn seeded_timeframe_months(
        tag: &str,
        vendor: Vendor,
        months: &[(u16, u8)],
        timeframe: Timeframe,
        n: i64,
    ) -> std::path::PathBuf {
        let r = root(tag);
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY is swept");
        let id = brutex_core::universe::fnv1a("NIFTY") as u32;
        for &(y, m) in months {
            let ym = YearMonth::new(y, m).expect("a real month");
            let path = StorePath::for_key(vendor, &key, timeframe, ym, FileKind::Bars)
                .expect("a path for a swept index");
            let mut file = BarFile::open_or_create(&r, path, id).expect("a fresh month opens");
            file.append(&bars_in(i64::from(y), i64::from(m), n))
                .expect("and takes its bars");
        }
        r
    }

    /// Writes `n` bars into each of `months` for NIFTY 1min, and returns the root.
    fn seeded_months(
        tag: &str,
        vendor: Vendor,
        months: &[(u16, u8)],
        n: i64,
    ) -> std::path::PathBuf {
        seeded_timeframe_months(tag, vendor, months, Timeframe::MINUTE_1, n)
    }

    /// The date helper agrees with the constant the single-month fixture uses.
    ///
    /// Without this the multi-month fixtures could all be stamped consistently
    /// WRONG and every span test would still pass, because they only ever
    /// compare against each other.
    #[test]
    fn the_date_helper_reproduces_the_fixture_constant() {
        assert_eq!(
            opening_micros(2026, 8),
            1_785_727_500_000_000,
            "2026-08-03 09:15 IST is the timestamp `bars` already uses"
        );
    }

    #[test]
    fn a_range_walks_forward_and_includes_both_ends() {
        assert_eq!(
            months_between((2026, 1), (2026, 3)).expect("forward"),
            vec![(2026, 1), (2026, 2), (2026, 3)]
        );
    }

    #[test]
    fn a_range_of_one_month_is_that_month() {
        assert_eq!(
            months_between((2026, 5), (2026, 5)).expect("one"),
            vec![(2026, 5)]
        );
    }

    #[test]
    fn a_range_rolls_the_year_at_december() {
        assert_eq!(
            months_between((2025, 11), (2026, 2)).expect("rolls"),
            vec![(2025, 11), (2025, 12), (2026, 1), (2026, 2)]
        );
    }

    #[test]
    fn a_backwards_range_is_refused_by_name() {
        let why = months_between((2026, 8), (2026, 1)).expect_err("backwards");
        assert!(
            why.contains("runs backwards"),
            "the refusal must say which way round to give them: {why}"
        );
    }

    #[test]
    fn a_span_longer_than_the_cap_refuses_before_it_walks() {
        // `YEAR` parses as u16, so this is the range an operator can actually
        // type. 763,000 months of failed file opens is not a refusal, it is a
        // hang -- so the count is checked before the first month is pushed.
        let why = months_between((1970, 1), (65535, 1)).expect_err("too long");
        assert!(
            why.contains("1200"),
            "the refusal must name the bound rather than saying `too long`: {why}"
        );
        // And the boundary itself is admitted, so the cap is a cap and not an
        // off-by-one that rejects the longest legal span.
        assert_eq!(
            months_between((2000, 1), (2099, 12))
                .expect("exactly the cap")
                .len(),
            MAX_SPAN_MONTHS
        );
    }

    #[test]
    fn a_span_joins_its_months_into_one_strictly_increasing_series() {
        let r = seeded_months(
            "span-join",
            Vendor::Zerodha,
            &[(2026, 1), (2026, 2), (2026, 3)],
            4,
        );
        let got = load_span(&r, Vendor::Zerodha, "NIFTY", "1min", (2026, 1), (2026, 3))
            .expect("three months are in the store");

        assert_eq!(got.bars.len(), 12, "every bar of every month is present");
        assert_eq!(got.found, 3);
        assert_eq!(got.asked, 3);
        assert!(got.complete(), "nothing is missing");
        assert!(got.missing.is_empty());

        // THE PROPERTY THE WHOLE ENGINE RESTS ON. `Column::build` folds bar by
        // bar and §3 rule 7's no-look-ahead holds by that shape, so a join that
        // stepped backwards would fold a later bar into an earlier state and
        // nothing downstream would notice.
        for pair in got.bars.windows(2) {
            let (a, b) = (
                pair.first().expect("a pair has a first"),
                pair.last().expect("and a last"),
            );
            assert!(
                b.ts_micros > a.ts_micros,
                "the joined series must be strictly increasing at every step, \
                 including across a month boundary: {} then {}",
                a.ts_micros,
                b.ts_micros
            );
        }
    }

    #[test]
    fn bounded_span_refuses_from_header_before_crossing_its_record_ceiling() {
        let r = seeded_months(
            "span-bounded",
            Vendor::Zerodha,
            &[(2026, 1), (2026, 2), (2026, 3)],
            4,
        );

        let why = load_span_bounded(
            &r,
            Vendor::Zerodha,
            "NIFTY",
            "1min",
            (2026, 1),
            (2026, 3),
            StoredSpanLoadBoundV1::new(11).expect("a nonzero bound"),
        )
        .expect_err("the third four-record header crosses an eleven-record ceiling");
        assert!(
            why.contains("2026-03"),
            "the refusing month is named: {why}"
        );
        assert!(
            why.contains("declares 4 committed records") && why.contains("3-record remainder"),
            "the refusal reports header count and remaining ceiling before allocation: {why}"
        );

        let exact = load_span_bounded(
            &r,
            Vendor::Zerodha,
            "NIFTY",
            "1min",
            (2026, 1),
            (2026, 3),
            StoredSpanLoadBoundV1::new(12).expect("the exact total is a valid bound"),
        )
        .expect("a ceiling admits exactly its declared records");
        assert_eq!(exact.bars.len(), 12);
        assert!(exact.complete());
    }

    #[test]
    fn typed_daily_and_minute_context_doors_keep_the_header_bound_protective() {
        let months = [(2026, 7), (2026, 8)];
        let signal = [candle_on_ist_day(20_668, 2_600_000)];
        let bound = StoredSpanLoadBoundV1::new(7).expect("a nonzero bound");

        let daily_root = seeded_timeframe_months(
            "daily-context-bounded",
            Vendor::Zerodha,
            &months,
            Timeframe::DAY_1,
            4,
        );
        let daily_why = load_daily_context_bounded(
            &daily_root,
            Vendor::Zerodha,
            "NIFTY",
            ((2026, 8), (2026, 8)),
            &signal,
            bound,
        )
        .expect_err("the requested daily month crosses the remaining header ceiling");
        assert!(
            daily_why.contains("2026-08")
                && daily_why.contains("declares 4 committed records")
                && daily_why.contains("3-record remainder"),
            "the daily typed door refuses at the bounded header: {daily_why}"
        );

        let minute_root = seeded_months("minute-context-bounded", Vendor::Zerodha, &months, 4);
        let minute_why = load_exact_minute_context_bounded(
            &minute_root,
            Vendor::Zerodha,
            "NIFTY",
            ((2026, 8), (2026, 8)),
            &signal,
            bound,
        )
        .expect_err("the requested minute month crosses the remaining header ceiling");
        assert!(
            minute_why.contains("2026-08")
                && minute_why.contains("declares 4 committed records")
                && minute_why.contains("3-record remainder"),
            "the minute typed door refuses at the bounded header: {minute_why}"
        );
    }

    #[test]
    fn stored_span_bound_has_no_zero_or_implicit_default() {
        let why = StoredSpanLoadBoundV1::new(0).expect_err("zero cannot authorize a read");
        assert!(
            why.contains("greater than zero") && why.contains("nothing was opened"),
            "zero refuses before any store access: {why}"
        );
        assert_eq!(
            StoredSpanLoadBoundV1::new(7)
                .expect("an explicit ceiling")
                .max_records(),
            7
        );
    }

    #[test]
    fn a_month_the_store_lacks_is_named_and_the_span_continues() {
        // February is absent. A seven-year request with one month un-pulled must
        // return six years and eleven months AND SAY SO -- refusing everything
        // would be worse, and skipping it silently is the §4 fallback.
        let r = seeded_months("span-hole", Vendor::Zerodha, &[(2026, 1), (2026, 3)], 5);
        let got = load_span(&r, Vendor::Zerodha, "NIFTY", "1min", (2026, 1), (2026, 3))
            .expect("two of the three months are there");

        assert_eq!(got.bars.len(), 10, "only the months that exist contribute");
        assert_eq!(got.asked, 3);
        assert_eq!(got.found, 2);
        assert_eq!(got.missing, vec![(2026, 2)], "the hole is named, in order");
        assert!(!got.complete(), "and the span knows it is not whole");
    }

    #[test]
    fn a_corrupt_middle_month_refuses_the_whole_span_instead_of_becoming_missing() {
        let r = seeded_months(
            "span-corrupt",
            Vendor::Zerodha,
            &[(2026, 1), (2026, 2), (2026, 3)],
            5,
        );
        let key = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY is swept");
        let ym = YearMonth::new(2026, 2).expect("a real month");
        let path = StorePath::for_key(
            Vendor::Zerodha,
            &key,
            Timeframe::MINUTE_1,
            ym,
            FileKind::Bars,
        )
        .expect("a path for the middle month")
        .to_path_buf(&r);
        std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("the seeded file exists")
            .set_len(1)
            .expect("the fixture can tear the file");

        let why = load_span(&r, Vendor::Zerodha, "NIFTY", "1min", (2026, 1), (2026, 3))
            .expect_err("corrupt bytes are not an absent month");
        assert!(
            why.contains("2026-02"),
            "the damaged member is named: {why}"
        );
        assert!(
            !why.contains("all 3 month(s) are absent"),
            "the failure must not be rewritten as missing: {why}"
        );
    }

    #[test]
    fn a_span_the_store_holds_nothing_for_refuses_rather_than_returning_empty() {
        let r = root("span-empty");
        let why = load_span(&r, Vendor::Zerodha, "NIFTY", "1min", (2026, 1), (2026, 3))
            .expect_err("nothing is stored");
        assert!(
            why.contains("all 3 month(s) are absent"),
            "the refusal must say how many months it looked for: {why}"
        );
    }

    #[test]
    fn a_span_refuses_a_rung_and_an_instrument_the_store_cannot_carry() {
        let r = seeded_months("span-bad", Vendor::Zerodha, &[(2026, 1)], 2);
        assert!(
            load_span(&r, Vendor::Zerodha, "NIFTY", "7min", (2026, 1), (2026, 1))
                .expect_err("no such rung")
                .contains("7min"),
            "an unknown rung is refused before any month is opened"
        );
        assert!(
            load_span(
                &r,
                Vendor::Zerodha,
                "../../etc",
                "1min",
                (2026, 1),
                (2026, 1)
            )
            .expect_err("not an index")
            .contains("is not an instrument this engine sweeps"),
            "a path-shaped name is refused as an instrument, not walked"
        );
    }

    #[test]
    fn a_month_outside_one_to_twelve_is_refused_and_never_reported_as_missing() {
        // FOUND BY A TEST, NOT BY REVIEW. `MONTH` parses as `u8`, so 13..=255
        // arrive intact, walk into the loop, build no path, and land in
        // `Span::missing`. The operator would read "2019-13 is missing from the
        // store" and go pull a month that cannot exist.
        for bad in [0_u8, 13, 99, 255] {
            let why = months_between((2019, bad), (2026, 8))
                .expect_err("a month outside 1..=12 is not a month");
            assert!(
                why.contains("is not a month"),
                "FROM month {bad} must be refused as malformed, not walked: {why}"
            );
            assert!(
                why.contains("FROM"),
                "and the refusal must say WHICH end was wrong: {why}"
            );
            let why =
                months_between((2019, 1), (2026, bad)).expect_err("the far end is checked too");
            assert!(
                why.contains("TO"),
                "the TO end must be named just as clearly: {why}"
            );
        }
        // And the legal ends are still admitted, so the guard is a guard and not
        // an off-by-one that rejects January or December.
        assert_eq!(
            months_between((2026, 1), (2026, 12)).expect("legal").len(),
            12
        );
    }

    fn candle_on_ist_day(day: i64, close: i64) -> Candle {
        let ts_micros = day
            .saturating_mul(86_400_000_000)
            .saturating_sub(indicators::IST_OFFSET_MICROS);
        Candle {
            ts_micros,
            open: close,
            high: close.saturating_add(100),
            low: close.saturating_sub(100),
            close,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    const OPEN_MONDAY_2026_08_03: i64 = 20_668;
    const OPEN_TUESDAY_2026_08_04: i64 = 20_669;
    const OPEN_WEDNESDAY_2026_08_05: i64 = 20_670;
    const CLOSED_SUNDAY_2026_08_02: i64 = 20_667;
    const CLOSED_REPUBLIC_DAY_2026_01_26: i64 = 20_479;

    fn accepted_open_before(day: i64) -> i64 {
        (pull::calendar::FIRST_DAY..day)
            .rev()
            .find(|candidate| {
                matches!(pull::calendar::kind_of(*candidate), DayKind::Open(_))
                    && !CHARTER_NON_REGULAR_IST_DAYS.contains(candidate)
            })
            .expect("a measured accepted session precedes the fixture day")
    }

    fn accepted_open_after(day: i64) -> i64 {
        (day.saturating_add(1)..=pull::calendar::LAST_DAY)
            .find(|candidate| {
                matches!(pull::calendar::kind_of(*candidate), DayKind::Open(_))
                    && !CHARTER_NON_REGULAR_IST_DAYS.contains(candidate)
            })
            .expect("a measured accepted session follows the fixture day")
    }

    fn daily_span(bars: Vec<Candle>) -> Span {
        Span {
            bars,
            vendor: Vendor::Zerodha,
            key: InstrumentKey::index(Exchange::Nse, "NIFTY").expect("swept fixture"),
            timeframe: "1day",
            asked: 2,
            found: 2,
            missing: Vec::new(),
            excluded: CalendarExclusion::none(),
        }
    }

    #[test]
    fn daily_context_is_strictly_prior_parallel_and_explicitly_unverified() {
        let daily = daily_span(vec![
            candle_on_ist_day(OPEN_MONDAY_2026_08_03, 2_500_000),
            candle_on_ist_day(OPEN_TUESDAY_2026_08_04, 2_500_100),
            candle_on_ist_day(OPEN_WEDNESDAY_2026_08_05, 2_500_200),
        ]);
        let signal = [
            candle_on_ist_day(OPEN_TUESDAY_2026_08_04, 2_600_000),
            candle_on_ist_day(OPEN_WEDNESDAY_2026_08_05, 2_600_100),
        ];
        let got = daily_context_from_span(daily, &signal).expect("complete causal daily stream");
        assert_eq!(got.bars.len(), got.references.len());
        assert_eq!(got.bars.len(), got.eligibility.len());
        assert!(
            got.references
                .iter()
                .all(|reference| reference.ist_day() < OPEN_WEDNESDAY_2026_08_05),
            "same-day and future daily bytes cannot enter the offered stream"
        );
        assert_eq!(
            got.references.last().map(DailyReference::ist_day),
            Some(OPEN_TUESDAY_2026_08_04)
        );
        assert!(
            DAILY_INTEGRITY_NOTE.starts_with("UNVERIFIED"),
            "ordinary reads must never manufacture a scrub receipt"
        );
        assert_eq!(DAILY_REFERENCE_SCHEMA, 1);
        assert_eq!(DAILY_ELIGIBILITY_POLICY, 1);
        // NINE, since 2021-02-24 joined them: the NSE outage day traded 09:15
        // to 10:08, halted, and reopened outside the storable window, so the
        // store holds a 54-minute stub. Left eligible, that stub became the
        // previous-day anchor for 2021-02-25 and moved the whole 44-position
        // pivot ladder for five sessions -- the same mechanism the two
        // disaster-recovery Saturdays were excluded for.
        assert_eq!(CHARTER_NON_REGULAR_IST_DAYS.len(), 9);
    }

    #[test]
    fn an_observed_regular_session_without_its_daily_record_refuses() {
        let daily = daily_span(vec![candle_on_ist_day(OPEN_MONDAY_2026_08_03, 2_500_000)]);
        let signal = [
            candle_on_ist_day(OPEN_TUESDAY_2026_08_04, 2_600_000),
            candle_on_ist_day(OPEN_WEDNESDAY_2026_08_05, 2_600_100),
        ];
        let why = daily_context_from_span(daily, &signal)
            .expect_err("the first signal day is observed but absent from daily storage");
        assert!(
            why.contains(&format!("day {OPEN_TUESDAY_2026_08_04} traded")),
            "{why}"
        );
        assert!(
            why.contains("older anchor was not silently reused"),
            "{why}"
        );
    }

    #[test]
    fn a_non_regular_observed_day_is_not_invented_as_an_eligible_anchor() {
        let non_regular = CHARTER_NON_REGULAR_IST_DAYS[0];
        let prior_regular = accepted_open_before(non_regular);
        let next_regular = accepted_open_after(non_regular);
        let daily = daily_span(vec![candle_on_ist_day(prior_regular, 2_500_000)]);
        let signal = [
            candle_on_ist_day(non_regular, 2_600_000),
            candle_on_ist_day(next_regular, 2_600_100),
        ];
        let got = daily_context_from_span(daily, &signal)
            .expect("the verified non-regular day must be skipped, not required as daily anchor");
        assert_eq!(got.eligibility, vec![1]);
    }

    #[test]
    fn a_stray_daily_row_on_a_measured_closed_day_refuses() {
        assert!(matches!(
            pull::calendar::kind_of(CLOSED_SUNDAY_2026_08_02),
            DayKind::Closed
        ));
        let daily = daily_span(vec![
            candle_on_ist_day(CLOSED_SUNDAY_2026_08_02, 2_400_000),
            candle_on_ist_day(OPEN_MONDAY_2026_08_03, 2_500_000),
        ]);
        let signal = [candle_on_ist_day(OPEN_TUESDAY_2026_08_04, 2_600_000)];
        let why = daily_context_from_span(daily, &signal)
            .expect_err("a stored Sunday daily row cannot become market evidence");
        assert!(why.contains("measures as closed"), "{why}");
        assert!(why.contains("stray row"), "{why}");
    }

    #[test]
    fn a_single_signal_day_measured_closed_refuses_inside_the_daily_door() {
        let prior_open = accepted_open_before(CLOSED_SUNDAY_2026_08_02);
        let daily = daily_span(vec![candle_on_ist_day(prior_open, 2_500_000)]);
        let signal = [candle_on_ist_day(CLOSED_SUNDAY_2026_08_02, 2_600_000)];
        let why = daily_context_from_span(daily, &signal)
            .expect_err("the standalone typed daily door must classify its only signal day");
        assert!(why.contains("measures as closed"), "{why}");
        assert!(why.contains(&CLOSED_SUNDAY_2026_08_02.to_string()), "{why}");
    }

    #[test]
    fn missing_warmup_and_missing_month_receipts_both_refuse_loudly() {
        let signal = [candle_on_ist_day(OPEN_TUESDAY_2026_08_04, 2_600_000)];
        let no_prior = daily_span(vec![candle_on_ist_day(OPEN_TUESDAY_2026_08_04, 2_500_000)]);
        let why = daily_context_from_span(no_prior, &signal).expect_err("same day is not prior");
        assert!(why.contains("strictly before"), "{why}");

        let mut incomplete = daily_span(vec![candle_on_ist_day(OPEN_MONDAY_2026_08_03, 2_500_000)]);
        incomplete.found = 1;
        incomplete.missing.push((2026, 7));
        let why = daily_context_from_span(incomplete, &signal).expect_err("hole is not a holiday");
        assert!(why.contains("missing 2026-07"), "{why}");
        assert!(why.contains("cannot be called a holiday"), "{why}");
    }

    fn minute_on_ist_day(day: i64, minute: i64, close: i64) -> Candle {
        let mut bar = candle_on_ist_day(day, close);
        bar.ts_micros = bar
            .ts_micros
            .saturating_add(minute.saturating_mul(60_000_000));
        bar
    }

    fn minute_span(bars: Vec<Candle>) -> Span {
        Span {
            bars,
            vendor: Vendor::Zerodha,
            key: InstrumentKey::index(Exchange::Nse, "NIFTY").expect("swept fixture"),
            timeframe: "1min",
            asked: 2,
            found: 2,
            missing: Vec::new(),
            excluded: CalendarExclusion::none(),
        }
    }

    #[test]
    fn exact_minute_context_requires_a_complete_prior_three_bar_session() {
        let signal = [minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000)];
        let complete = minute_span(vec![
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 927, 2_500_000),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 928, 2_500_100),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 929, 2_500_200),
            minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000),
        ]);
        let got = exact_minute_context_from_span(complete, &signal)
            .expect("the exact canonical terminal three seed GapFib");
        assert_eq!(got.prior_session_day, OPEN_MONDAY_2026_08_03);
        assert_eq!(got.prior_session_bars, 3);
        assert_eq!(got.bars.len(), 4);
        assert_eq!(got.found, got.asked);
        assert_eq!(EXACT_MINUTE_GAP_POLICY, 1);
        assert!(EXACT_MINUTE_INTEGRITY_NOTE.starts_with("UNVERIFIED"));

        let short = minute_span(vec![
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 928, 2_500_100),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 929, 2_500_200),
            minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000),
        ]);
        let why = exact_minute_context_from_span(short, &signal)
            .expect_err("two prior minutes cannot prove a three-bar gap");
        assert!(why.contains("only 2 bar(s)"), "{why}");
        assert!(why.contains("final three"), "{why}");
    }

    #[test]
    fn exact_minute_context_refuses_sunday_and_weekday_closed_rows() {
        let cases = [
            (CLOSED_SUNDAY_2026_08_02, OPEN_MONDAY_2026_08_03),
            (
                CLOSED_REPUBLIC_DAY_2026_01_26,
                accepted_open_after(CLOSED_REPUBLIC_DAY_2026_01_26),
            ),
        ];
        for (closed_day, signal_day) in cases {
            assert!(matches!(
                pull::calendar::kind_of(closed_day),
                DayKind::Closed
            ));
            let signal = [minute_on_ist_day(signal_day, 555, 2_600_000)];
            let minute = minute_span(vec![
                minute_on_ist_day(closed_day, 927, 2_500_000),
                minute_on_ist_day(closed_day, 928, 2_500_100),
                minute_on_ist_day(closed_day, 929, 2_500_200),
                minute_on_ist_day(signal_day, 555, 2_600_000),
            ]);
            let why = exact_minute_context_from_span(minute, &signal)
                .expect_err("closed-day minutes cannot impersonate a prior session");
            assert!(why.contains("measured-closed"), "{why}");
            assert!(why.contains(&closed_day.to_string()), "{why}");
        }
    }

    #[test]
    fn exact_minute_context_refuses_three_early_prior_session_bars() {
        let signal = [minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000)];
        let early = minute_span(vec![
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 555, 2_500_000),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 556, 2_500_100),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 557, 2_500_200),
            minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000),
        ]);
        let why = exact_minute_context_from_span(early, &signal)
            .expect_err("three opening bars are not the prior session's terminal three");
        assert!(why.contains("terminal-minute geometry"), "{why}");
        assert!(why.contains("Early or truncated bars"), "{why}");
    }

    #[test]
    fn exact_minute_context_refuses_holes_and_malformed_cadence() {
        let signal = [minute_on_ist_day(OPEN_TUESDAY_2026_08_04, 555, 2_600_000)];
        let mut hole = minute_span(vec![
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 927, 2_500_000),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 928, 2_500_100),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 929, 2_500_200),
        ]);
        hole.found = 1;
        hole.missing.push((2026, 7));
        let why = exact_minute_context_from_span(hole, &signal)
            .expect_err("a missing month is not a holiday");
        assert!(why.contains("missing 2026-07"), "{why}");
        assert!(why.contains("cannot be reconstructed"), "{why}");

        let malformed = minute_span(vec![
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 927, 2_500_000),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 928, 2_500_100),
            minute_on_ist_day(OPEN_MONDAY_2026_08_03, 928, 2_500_200),
        ]);
        let why = exact_minute_context_from_span(malformed, &signal)
            .expect_err("a duplicate exact minute is not an ordered path");
        assert!(why.contains("cadence is malformed"), "{why}");
    }

    fn calendar_minute_v1(day: i64, minute: u16) -> i64 {
        day.checked_mul(MICROS_PER_DAY_V1)
            .and_then(|at| {
                i64::from(minute)
                    .checked_mul(MICROS_PER_MINUTE_V1)
                    .and_then(|within| at.checked_add(within))
            })
            .and_then(|ist| ist.checked_sub(indicators::IST_OFFSET_MICROS))
            .expect("the measured calendar fixture fits epoch micros")
    }

    fn calendar_windows_v1(day: i64, windows: &[(u16, u16)]) -> Vec<i64> {
        let mut timestamps = Vec::new();
        for &(from, to) in windows {
            for minute in from..=to {
                timestamps.push(calendar_minute_v1(day, minute));
            }
        }
        timestamps
    }

    #[test]
    fn calendar_receipt_regular_complete_and_gap_are_distinct() {
        let day = 20_668_i64; // 2026-08-03, a measured regular session.
        let complete = calendar_windows_v1(day, &[(555, 929)]);
        let receipt =
            calendar_receipt_v1(&complete, day, day).expect("the full regular session reconciles");
        assert_eq!(receipt.schema_version(), CALENDAR_RECEIPT_SCHEMA_V1);
        assert_eq!(receipt.policy_version(), CALENDAR_RECEIPT_POLICY_V1);
        assert_eq!(receipt.first_day(), day);
        assert_eq!(receipt.last_day(), day);
        assert_eq!(receipt.offered(), 375);
        assert_eq!(receipt.expected(), 375);
        assert_eq!(receipt.missing(), 0);
        assert_eq!(receipt.unexpected(), 0);
        assert_eq!(receipt.status(), CalendarStatusV1::Complete);
        assert_ne!(receipt.digest(), [0; 32]);

        let gap: Vec<_> = complete
            .iter()
            .copied()
            .filter(|timestamp| *timestamp != calendar_minute_v1(day, 700))
            .collect();
        let receipt =
            calendar_receipt_v1(&gap, day, day).expect("an absent minute is counted, not invented");
        assert_eq!(receipt.offered(), 374);
        assert_eq!(receipt.expected(), 375);
        assert_eq!(receipt.missing(), 1);
        assert_eq!(receipt.unexpected(), 0);
        assert_eq!(receipt.status(), CalendarStatusV1::Incomplete);
    }

    #[test]
    fn calendar_receipt_counts_entire_missing_interior_and_boundary_days() {
        let first_day = 20_668_i64;
        let missing_day = 20_669_i64;
        let last_day = 20_670_i64;
        for day in [first_day, missing_day, last_day] {
            assert!(
                matches!(pull::calendar::kind_of(day), DayKind::Open(_)),
                "fixture IST day {day} must be a measured open session"
            );
        }
        let mut offered = calendar_windows_v1(first_day, &[(555, 929)]);
        offered.extend(calendar_windows_v1(last_day, &[(555, 929)]));
        let receipt = calendar_receipt_v1(&offered, first_day, last_day)
            .expect("an absent measured day is a counted gap, not corruption");
        assert_eq!(receipt.first_day(), first_day);
        assert_eq!(receipt.last_day(), last_day);
        assert_eq!(receipt.offered(), 750);
        assert_eq!(receipt.expected(), 1_125);
        assert_eq!(receipt.missing(), 375);
        assert_eq!(receipt.status(), CalendarStatusV1::Incomplete);

        let boundary_only = calendar_windows_v1(missing_day, &[(555, 929)]);
        let receipt = calendar_receipt_v1(&boundary_only, first_day, last_day)
            .expect("missing requested boundary days remain in the denominator");
        assert_eq!(receipt.offered(), 375);
        assert_eq!(receipt.expected(), 1_125);
        assert_eq!(receipt.missing(), 750);
        assert_eq!(receipt.status(), CalendarStatusV1::Incomplete);
    }

    #[test]
    fn calendar_receipt_split_and_short_sessions_use_their_exact_windows() {
        let split_day = 19_784_i64;
        let split = calendar_windows_v1(split_day, &[(555, 599), (690, 749)]);
        let receipt = calendar_receipt_v1(&split, split_day, split_day)
            .expect("both measured split windows are complete");
        assert_eq!(receipt.offered(), 105);
        assert_eq!(receipt.expected(), 105);
        assert_eq!(receipt.missing(), 0);
        assert_eq!(receipt.status(), CalendarStatusV1::Complete);

        let short_day = 20_382_i64;
        let short = calendar_windows_v1(short_day, &[(825, 884)]);
        let receipt = calendar_receipt_v1(&short, short_day, short_day)
            .expect("the measured one-hour session is complete");
        assert_eq!(receipt.offered(), 60);
        assert_eq!(receipt.expected(), 60);
        assert_eq!(receipt.missing(), 0);
        assert_eq!(receipt.status(), CalendarStatusV1::Complete);
    }

    #[test]
    fn calendar_receipt_refuses_closed_days_and_outside_window_bars() {
        let closed = calendar_minute_v1(20_479, 555); // 2026-01-26, measured closed.
        let why = calendar_receipt_v1(&[closed], 20_479, 20_479)
            .expect_err("a closed day cannot carry a bar");
        assert!(why.contains("measured closed IST day 20479"), "{why}");

        let between_split_windows = calendar_minute_v1(19_784, 630);
        let why = calendar_receipt_v1(&[between_split_windows], 19_784, 19_784)
            .expect_err("the split-session break was not an offered minute");
        assert!(
            why.contains("outside every measured session window"),
            "{why}"
        );

        let before_short_session = calendar_minute_v1(20_382, 555);
        let why = calendar_receipt_v1(&[before_short_session], 20_382, 20_382)
            .expect_err("a regular open is outside the measured short session");
        assert!(
            why.contains("outside every measured session window"),
            "{why}"
        );
    }

    #[test]
    fn calendar_receipt_keeps_both_unmeasured_day_kinds_digest_bound() {
        let outside_day = 16_436_i64; // 2015-01-01, before measured coverage.
        let outside = calendar_receipt_v1(
            &[calendar_minute_v1(outside_day, 555)],
            outside_day,
            outside_day,
        )
        .expect("an unmeasured day is a non-claim, not a refusal");
        assert_eq!(outside.status(), CalendarStatusV1::Unmeasured);
        assert_eq!(outside.first_day(), outside_day);
        assert_eq!(outside.last_day(), outside_day);
        assert_eq!(outside.offered(), 1);
        assert_eq!(outside.expected(), 0);
        assert_eq!(outside.missing(), 0);

        let unknown_length_day = 20_028_i64;
        let unknown_length = calendar_receipt_v1(
            &[calendar_minute_v1(unknown_length_day, 825)],
            unknown_length_day,
            unknown_length_day,
        )
        .expect("a proved open day with unknown length remains a non-claim");
        assert_eq!(unknown_length.status(), CalendarStatusV1::Unmeasured);
        assert_eq!(unknown_length.expected(), 0);
        assert_eq!(unknown_length.missing(), 0);
        assert_ne!(
            outside.digest(),
            unknown_length.digest(),
            "out-of-range and open-length-unmeasured are different bound decisions"
        );
    }

    #[test]
    fn calendar_receipt_empty_slices_use_the_requested_calendar_span() {
        let open = calendar_receipt_v1(&[], 20_668, 20_668)
            .expect("empty is a valid observation of a known open day");
        assert_eq!(open.offered(), 0);
        assert_eq!(open.expected(), 375);
        assert_eq!(open.missing(), 375);
        assert_eq!(open.status(), CalendarStatusV1::Incomplete);

        let closed =
            calendar_receipt_v1(&[], 20_479, 20_479).expect("a known closed day owes no bars");
        assert_eq!(closed.offered(), 0);
        assert_eq!(closed.expected(), 0);
        assert_eq!(closed.missing(), 0);
        assert_eq!(closed.status(), CalendarStatusV1::Complete);

        let unmeasured = calendar_receipt_v1(&[], 16_436, 16_436)
            .expect("an empty unmeasured day remains a non-claim");
        assert_eq!(unmeasured.offered(), 0);
        assert_eq!(unmeasured.expected(), 0);
        assert_eq!(unmeasured.missing(), 0);
        assert_eq!(unmeasured.status(), CalendarStatusV1::Unmeasured);
    }

    #[test]
    fn calendar_receipt_refuses_off_grid_duplicate_backward_and_bad_spans() {
        let base = calendar_minute_v1(20_668, 555);
        let why = calendar_receipt_v1(&[base.saturating_add(1)], 20_668, 20_668)
            .expect_err("one microsecond is not an exact minute");
        assert!(why.contains("off the exact one-minute grid"), "{why}");

        let why = calendar_receipt_v1(&[base, base], 20_668, 20_668)
            .expect_err("duplicates are corruption");
        assert!(why.contains("duplicate"), "{why}");

        let next = base.saturating_add(MICROS_PER_MINUTE_V1);
        let why = calendar_receipt_v1(&[next, base], 20_668, 20_668)
            .expect_err("time cannot run backward");
        assert!(why.contains("backward"), "{why}");

        let outside = calendar_minute_v1(20_669, 555);
        let why = calendar_receipt_v1(&[outside], 20_668, 20_668)
            .expect_err("offered bounds cannot widen requested bounds");
        assert!(why.contains("outside requested inclusive span"), "{why}");

        let why = calendar_receipt_v1(&[], 20_669, 20_668)
            .expect_err("a requested span cannot run backward");
        assert!(why.contains("runs backward"), "{why}");

        let too_many =
            i64::try_from(MAX_CALENDAR_RECEIPT_DAYS_V1).expect("the month-derived cap fits i64");
        let why = calendar_receipt_v1(&[], 0, too_many)
            .expect_err("inclusive length is one beyond the existing span contract");
        assert!(why.contains("1200-month span contract"), "{why}");
        assert!(why.contains("37200"), "{why}");
    }

    #[test]
    fn calendar_receipt_digest_binds_which_exact_minute_is_missing() {
        let day = 20_668_i64;
        let full = calendar_windows_v1(day, &[(555, 929)]);
        let missing_600: Vec<_> = full
            .iter()
            .copied()
            .filter(|timestamp| *timestamp != calendar_minute_v1(day, 600))
            .collect();
        let missing_601: Vec<_> = full
            .iter()
            .copied()
            .filter(|timestamp| *timestamp != calendar_minute_v1(day, 601))
            .collect();
        let left = calendar_receipt_v1(&missing_600, day, day).expect("one named gap");
        let repeated =
            calendar_receipt_v1(&missing_600, day, day).expect("same bytes, same receipt");
        let right = calendar_receipt_v1(&missing_601, day, day).expect("a different named gap");
        assert_eq!(left, repeated, "receipt construction is byte-deterministic");
        assert_eq!(left.offered(), right.offered());
        assert_eq!(left.expected(), right.expected());
        assert_eq!(left.missing(), right.missing());
        assert_eq!(left.status(), right.status());
        assert_ne!(
            left.digest(),
            right.digest(),
            "equal counts cannot alias different timestamp-membership decisions"
        );
    }

    const CALENDAR_RUNGS_V2: [(u32, u64); 8] = [
        (60, 375),
        (120, 188),
        (180, 125),
        (300, 75),
        (600, 38),
        (900, 25),
        (1_800, 13),
        (3_600, 7),
    ];

    fn calendar_bucket_indices_v2(
        day: i64,
        rung_seconds: u32,
        intervals: &[(i64, i64)],
    ) -> Vec<i64> {
        let width = i64::from(rung_seconds / 60);
        let mut timestamps = Vec::new();
        for &(first, last) in intervals {
            for bucket in first..=last {
                let minute = NSE_OPEN_MINUTE_V2
                    .checked_add(bucket.checked_mul(width).expect("fixture bucket fits"))
                    .expect("fixture minute fits");
                timestamps.push(calendar_minute_v1(
                    day,
                    u16::try_from(minute).expect("fixture minute is within one IST day"),
                ));
            }
        }
        timestamps
    }

    fn regular_buckets_v2(day: i64, rung_seconds: u32, expected: u64) -> Vec<i64> {
        calendar_bucket_indices_v2(
            day,
            rung_seconds,
            &[(
                0,
                i64::try_from(expected)
                    .expect("regular denominator fits i64")
                    .saturating_sub(1),
            )],
        )
    }

    fn split_intervals_v2(rung_seconds: u32) -> &'static [(i64, i64)] {
        match rung_seconds {
            60 => &[(0, 44), (135, 194)],
            120 => &[(0, 22), (67, 97)],
            180 => &[(0, 14), (45, 64)],
            300 => &[(0, 8), (27, 38)],
            600 => &[(0, 4), (13, 19)],
            900 => &[(0, 2), (9, 12)],
            1_800 => &[(0, 1), (4, 6)],
            3_600 => &[(0, 0), (2, 3)],
            _ => panic!("fixture names only a signal rung"),
        }
    }

    fn short_intervals_v2(rung_seconds: u32) -> &'static [(i64, i64)] {
        match rung_seconds {
            60 => &[(270, 329)],
            120 => &[(135, 164)],
            180 => &[(90, 109)],
            300 => &[(54, 65)],
            600 => &[(27, 32)],
            900 => &[(18, 21)],
            1_800 => &[(9, 10)],
            3_600 => &[(4, 5)],
            _ => panic!("fixture names only a signal rung"),
        }
    }

    #[test]
    fn calendar_receipt_v2_regular_session_is_complete_on_all_eight_rungs() {
        let day = 20_668_i64;
        for (rung_seconds, expected) in CALENDAR_RUNGS_V2 {
            let offered = regular_buckets_v2(day, rung_seconds, expected);
            let receipt = calendar_receipt_v2(&offered, rung_seconds, day, day)
                .expect("the pinned regular-session rung geometry reconciles");
            assert_eq!(receipt.schema_version(), CALENDAR_RECEIPT_SCHEMA_V2);
            assert_eq!(receipt.policy_version(), CALENDAR_RECEIPT_POLICY_V2);
            assert_eq!(receipt.rung_seconds(), rung_seconds);
            assert_eq!(receipt.first_day(), day);
            assert_eq!(receipt.last_day(), day);
            assert_eq!(receipt.offered(), expected, "rung {rung_seconds}");
            assert_eq!(receipt.expected(), expected, "rung {rung_seconds}");
            assert_eq!(receipt.missing(), 0, "rung {rung_seconds}");
            assert_eq!(receipt.unexpected(), 0, "rung {rung_seconds}");
            assert_eq!(
                receipt.status(),
                CalendarStatusV1::Complete,
                "rung {rung_seconds}"
            );
            assert_ne!(receipt.digest(), [0; 32], "rung {rung_seconds}");

            let complete = receipt
                .require_complete()
                .expect("a complete receipt projects into the capability");
            assert_eq!(complete.rung_seconds(), rung_seconds);
            assert_eq!(complete.first_day(), day);
            assert_eq!(complete.last_day(), day);
            assert_eq!(complete.digest(), receipt.digest());
            assert_eq!(
                receipt,
                calendar_receipt_v2(&offered, rung_seconds, day, day)
                    .expect("same evidence makes the same receipt"),
                "rung {rung_seconds} is byte-deterministic"
            );
        }
    }

    #[test]
    fn calendar_receipt_v2_gap_location_is_bound_on_all_eight_rungs() {
        let day = 20_668_i64;
        for (rung_seconds, expected) in CALENDAR_RUNGS_V2 {
            let full = regular_buckets_v2(day, rung_seconds, expected);
            let mut early = full.clone();
            early.remove(1);
            let mut late = full;
            let late_index = late.len().saturating_sub(2);
            late.remove(late_index);

            let left = calendar_receipt_v2(&early, rung_seconds, day, day)
                .expect("a gap is measured, never synthesized");
            let right = calendar_receipt_v2(&late, rung_seconds, day, day)
                .expect("a different gap is also measured");
            for receipt in [left, right] {
                assert_eq!(receipt.offered(), expected.saturating_sub(1));
                assert_eq!(receipt.expected(), expected);
                assert_eq!(receipt.missing(), 1);
                assert_eq!(receipt.unexpected(), 0);
                assert_eq!(receipt.status(), CalendarStatusV1::Incomplete);
                let why = receipt
                    .require_complete()
                    .expect_err("a missing bucket cannot become a complete capability");
                assert!(why.contains("is not complete"), "{why}");
                assert!(why.contains("missing 1"), "{why}");
            }
            assert_ne!(
                left.digest(),
                right.digest(),
                "rung {rung_seconds}: equal counts cannot alias different gaps"
            );
        }
    }

    /// Every split and short exception day is WITHHELD, on all eight rungs,
    /// and the receipt still names the size of what it withheld.
    ///
    /// # What this replaced, and why the old assertion had to go
    ///
    /// It used to assert that both days reconcile COMPLETE with
    /// `expected == offered == 105` (and 60). That was true under calendar
    /// policy 2 and is a contradiction under policy 3: the loader now removes
    /// those bars, so offering them proves the loader and the calendar
    /// disagree, and the receipt refuses.
    ///
    /// # Both halves of the agreement are pinned here
    ///
    /// A test that only checked the empty case would pass against a build that
    /// silently ACCEPTED a withheld day's bars, which is the failure mode
    /// `CLAUDE.md` §4 calls a fallback that hides a failure. So the refusal is
    /// asserted first and the completeness second.
    ///
    /// # The counts are the old ones, deliberately
    ///
    /// `withheld_buckets` must equal what `expected` was before the policy
    /// changed — the day's real geometry is still measured, it is merely not
    /// demanded. Reusing the exact numbers is what proves that.
    ///
    /// All four `pull::calendar::IRREGULAR` entries are charter days, so after
    /// policy 3 no split or short session is reachable from production input at
    /// all. These two days are the whole population of that geometry.
    #[test]
    fn calendar_receipt_v2_withholds_split_and_short_exceptions_on_all_eight_rungs() {
        let split_day = 19_784_i64;
        let short_day = 20_382_i64;
        let split_counts = [105_u64, 54, 35, 21, 12, 7, 5, 3];
        let short_counts = [60_u64, 30, 20, 12, 6, 4, 2, 2];
        let nothing: [i64; 0] = [];

        for (index, (rung_seconds, _)) in CALENDAR_RUNGS_V2.into_iter().enumerate() {
            let split = calendar_bucket_indices_v2(
                split_day,
                rung_seconds,
                split_intervals_v2(rung_seconds),
            );
            let short = calendar_bucket_indices_v2(
                short_day,
                rung_seconds,
                short_intervals_v2(rung_seconds),
            );

            for (day, offered, expected_withheld) in [
                (split_day, &split, split_counts[index]),
                (short_day, &short, short_counts[index]),
            ] {
                let why = calendar_receipt_v2(offered, rung_seconds, day, day)
                    .expect_err("a withheld day's own bars cannot be offered");
                assert!(
                    why.contains("withholds from every swept series"),
                    "rung {rung_seconds} day {day}: {why}"
                );
                assert!(
                    why.contains("the series and the calendar disagree"),
                    "rung {rung_seconds} day {day}: {why}"
                );

                let receipt = calendar_receipt_v2(&nothing, rung_seconds, day, day)
                    .expect("a withheld day expects nothing and therefore reconciles");
                assert_eq!(receipt.offered(), 0);
                assert_eq!(receipt.expected(), 0, "a withheld day demands no bucket");
                assert_eq!(receipt.missing(), 0);
                assert_eq!(receipt.unexpected(), 0);
                assert_eq!(receipt.status(), CalendarStatusV1::Complete);
                assert_eq!(receipt.withheld_days(), 1);
                assert_eq!(
                    receipt.withheld_buckets(),
                    expected_withheld,
                    "rung {rung_seconds} day {day} must state the size of what it withheld"
                );
                receipt
                    .require_complete()
                    .expect("a withheld day is admissible evidence, not a hole");
            }
        }

        let two_minute_split = calendar_bucket_indices_v2(split_day, 120, split_intervals_v2(120));
        assert!(
            two_minute_split.contains(&calendar_minute_v1(split_day, 689)),
            "the bucket intersecting the second window is canonically stamped one minute before it"
        );
        let hourly_short = calendar_bucket_indices_v2(short_day, 3_600, short_intervals_v2(3_600));
        assert!(
            hourly_short.contains(&calendar_minute_v1(short_day, 795)),
            "the first hourly bucket intersecting the 13:45 exception is stamped 13:15"
        );
    }

    #[test]
    fn calendar_receipt_v2_refuses_malformed_and_closed_bars_on_every_rung() {
        let open_day = 20_668_i64;
        let closed_day = 20_479_i64;
        for (rung_seconds, _) in CALENDAR_RUNGS_V2 {
            let width_minutes = u16::try_from(rung_seconds / 60).expect("rung fits u16");
            let first = calendar_minute_v1(open_day, 555);
            let second = calendar_minute_v1(open_day, 555 + width_minutes);

            let why = calendar_receipt_v2(&[first, first], rung_seconds, open_day, open_day)
                .expect_err("duplicates are corruption");
            assert!(why.contains("duplicate"), "rung {rung_seconds}: {why}");

            let why = calendar_receipt_v2(&[second, first], rung_seconds, open_day, open_day)
                .expect_err("time cannot run backward");
            assert!(why.contains("backward"), "rung {rung_seconds}: {why}");

            let why =
                calendar_receipt_v2(&[first.saturating_add(1)], rung_seconds, open_day, open_day)
                    .expect_err("a sub-minute timestamp is never a bar boundary");
            assert!(
                why.contains("off the exact one-minute grid"),
                "rung {rung_seconds}: {why}"
            );

            if rung_seconds > 60 {
                let minute_but_not_rung = calendar_minute_v1(open_day, 556);
                let why =
                    calendar_receipt_v2(&[minute_but_not_rung], rung_seconds, open_day, open_day)
                        .expect_err("an exact minute can still miss the signal rung grid");
                assert!(
                    why.contains("open-anchored grid"),
                    "rung {rung_seconds}: {why}"
                );
            }

            let prior_bucket_minute = 555_u16
                .checked_sub(width_minutes)
                .expect("the widest prior bucket remains within the IST day");
            let outside_window = calendar_minute_v1(open_day, prior_bucket_minute);
            let why = calendar_receipt_v2(&[outside_window], rung_seconds, open_day, open_day)
                .expect_err("an aligned bucket that intersects no session is unexpected");
            assert!(
                why.contains("intersects no measured session window"),
                "rung {rung_seconds}: {why}"
            );

            let closed = calendar_minute_v1(closed_day, 555);
            let why = calendar_receipt_v2(&[closed], rung_seconds, closed_day, closed_day)
                .expect_err("a measured closed day cannot carry a bar");
            assert!(
                why.contains("measured closed IST day 20479"),
                "rung {rung_seconds}: {why}"
            );
        }
    }

    #[test]
    fn calendar_receipt_v2_unmeasured_and_input_bounds_fail_closed() {
        let unmeasured_day = 16_436_i64;
        let receipt = calendar_receipt_v2(
            &[calendar_minute_v1(unmeasured_day, 555)],
            900,
            unmeasured_day,
            unmeasured_day,
        )
        .expect("an unmeasured day is retained as a non-claim");
        assert_eq!(receipt.status(), CalendarStatusV1::Unmeasured);
        assert_eq!(receipt.expected(), 0);
        assert_eq!(receipt.missing(), 0);
        assert!(
            receipt.require_complete().is_err(),
            "unmeasured evidence never becomes a completeness capability"
        );

        let why = calendar_receipt_v2(&[], 1, 20_668, 20_668)
            .expect_err("one second is stored but is not one of the eight signal rungs");
        assert!(why.contains("not one of the exact signal rungs"), "{why}");

        let why = calendar_receipt_v2(&[], 86_400, 20_668, 20_668)
            .expect_err("daily coverage is a different receipt contract");
        assert!(why.contains("not one of the exact signal rungs"), "{why}");

        let why = calendar_receipt_v2(&[], 60, 20_669, 20_668)
            .expect_err("the requested span cannot run backward");
        assert!(why.contains("runs backward"), "{why}");

        let outside = calendar_minute_v1(20_669, 555);
        let why = calendar_receipt_v2(&[outside], 60, 20_668, 20_668)
            .expect_err("offered timestamps cannot widen requested bounds");
        assert!(why.contains("outside requested inclusive span"), "{why}");
    }

    #[test]
    fn calendar_receipt_v2_digest_binds_rung_and_requested_boundaries() {
        let closed_day = 20_673_i64; // 2026-08-08, Saturday.
        let next_closed_day = 20_674_i64; // 2026-08-09, Sunday.
        assert!(
            matches!(pull::calendar::kind_of(closed_day), DayKind::Closed),
            "fixture day is measured closed"
        );
        assert!(
            matches!(pull::calendar::kind_of(next_closed_day), DayKind::Closed),
            "fixture Sunday is measured closed"
        );

        let mut prior_digest = None;
        for (rung_seconds, _) in CALENDAR_RUNGS_V2 {
            let receipt = calendar_receipt_v2(&[], rung_seconds, closed_day, closed_day)
                .expect("an empty measured closed day is complete");
            assert_eq!(receipt.status(), CalendarStatusV1::Complete);
            receipt
                .require_complete()
                .expect("zero of zero expected buckets is complete");
            if let Some(prior) = prior_digest {
                assert_ne!(
                    receipt.digest(),
                    prior,
                    "the rung is a digest term even when counts are identical"
                );
            }
            prior_digest = Some(receipt.digest());
        }

        let one_day = calendar_receipt_v2(&[], 60, closed_day, closed_day)
            .expect("one closed day reconciles");
        let two_days = calendar_receipt_v2(&[], 60, closed_day, next_closed_day)
            .expect("two closed days reconcile");
        assert_eq!(one_day.offered(), two_days.offered());
        assert_eq!(one_day.expected(), two_days.expected());
        assert_ne!(
            one_day.digest(),
            two_days.digest(),
            "equal counts over different requested boundaries never alias"
        );
    }

    #[test]
    fn calendar_policy_digest_v2_is_canonical_and_not_a_coverage_receipt() {
        let policy = calendar_policy_digest_v2();
        assert_ne!(
            policy, [0; 32],
            "the canonical policy is never an absent identity"
        );
        assert_eq!(
            policy,
            calendar_policy_digest_v2(),
            "the complete measured-calendar policy is byte-deterministic"
        );

        for (production, (fixture, _)) in
            CALENDAR_POLICY_RUNGS_V2.into_iter().zip(CALENDAR_RUNGS_V2)
        {
            assert_eq!(
                production, fixture,
                "the policy identity and coverage receipt must name the same eight rungs"
            );
        }

        let closed_day = 20_673_i64;
        let one_minute_coverage = calendar_receipt_v2(&[], 60, closed_day, closed_day)
            .expect("an empty measured closed day is complete");
        let one_hour_coverage = calendar_receipt_v2(&[], 3_600, closed_day, closed_day)
            .expect("the same closed day is complete at every supported rung");
        assert_ne!(
            policy,
            one_minute_coverage.digest(),
            "a global policy identity cannot alias one requested-span receipt"
        );
        assert_ne!(
            policy,
            one_hour_coverage.digest(),
            "policy and coverage remain separate at every rung"
        );
    }

    #[test]
    fn candle_calendar_projection_is_exactly_the_timestamp_receipt_without_a_side_vector() {
        let open_day = 20_668_i64;
        let timestamps = calendar_bucket_indices_v2(open_day, 3_600, &[(0, 6)]);
        let bars: Vec<Candle> = timestamps
            .iter()
            .copied()
            .map(|ts_micros| Candle {
                ts_micros,
                open: 100,
                high: 100,
                low: 100,
                close: 100,
                volume: 0,
                open_interest: i64::MIN,
            })
            .collect();
        let from_timestamps = calendar_receipt_v2(&timestamps, 3_600, open_day, open_day)
            .expect("the regular hour-rung session is complete");
        let from_bars = calendar_receipt_v2_for_bars(&bars, 3_600, open_day, open_day)
            .expect("the exact same candle timestamps are complete");
        assert_eq!(
            from_bars, from_timestamps,
            "projecting exact candle timestamps cannot change any calendar fact or digest"
        );
        from_bars
            .require_complete()
            .expect("all seven regular-session hour buckets were supplied");
    }

    /// The multi-month accumulator grows amortised, not exactly.
    ///
    /// # Why a source-shape test
    ///
    /// `try_reserve_exact` and `try_reserve` produce the IDENTICAL span — same
    /// bars, same order, same bytes — so no behavioural test can tell them
    /// apart. The only difference is how many times each bar is memcpy'd on the
    /// way in: exact reservation leaves `capacity == len` after every month, so
    /// the next month reallocates and copies everything accumulated, giving
    /// `B x M / 2` bytes moved instead of `B`.
    ///
    /// A timing test would be the direct proof and is not available: this
    /// machine has been at load average 40-104 on 14 cores throughout, and the
    /// difference is memory bandwidth, which is exactly what a loaded machine
    /// destroys. So the shape is pinned instead, with the reason attached.
    ///
    /// This also guards the `Cost` section's claim of "O(M + B) time", which
    /// exact reservation made false.
    ///
    /// Scoped to the accumulating loop. The module's other two
    /// `try_reserve_exact` calls are correct — each reserves a capacity computed
    /// once for a vector that is then filled — and must not fail this.
    #[test]
    fn the_multi_month_span_accumulator_reserves_amortised_and_not_exactly() {
        let source = include_str!("stored.rs");
        let anchor = "let mut bars: Vec<Candle> = Vec::new();";
        let at = source
            .find(anchor)
            .expect("the span accumulator must still start from an empty Vec");
        let rest = source.get(at..).unwrap_or_default();
        let end = rest
            .find("if bars.is_empty() {")
            .expect("the accumulating loop must still end at the emptiness check");
        let body = rest.get(..end).unwrap_or_default();

        assert!(
            body.len() > 800,
            "the scan found a {}-byte body, so the anchors moved and this test \
             would pass over nothing",
            body.len()
        );
        assert!(
            body.contains("bars.try_reserve(one.bars.len())"),
            "the accumulator must reserve with amortised growth"
        );
        assert!(
            !body.contains("bars.try_reserve_exact("),
            "an exact reservation inside this loop leaves capacity == len after \
             every month, so the next month memcpys the whole span again -- \
             B x M / 2 bytes moved instead of B, and it makes this function's \
             own stated O(M + B) cost false"
        );
    }
}
