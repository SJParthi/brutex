//! The walker's records and their codec (FIB-5; D-6385; 17:15 item 7 and
//! FM-F031).
//!
//! Two record kinds, each fixed-stride, little-endian and versioned, with no
//! byte cap:
//!
//! * [`DecisionRecord`] v1 (the specification's `CandidateRecord` v1): ONE per
//!   decision — its plan, its coordinates, and every measured row's raw
//!   measurement with its own-past mid-rank position, so a later generation
//!   re-buckets it without re-simulating anything. It carries no outcome, so
//!   no outcome can ever feed a coordinate.
//! * [`OutcomeRecord`]: ONE per (decision ordinal, exit variant, fill
//!   variant), with one slot per trade size, in its own file per variant-set
//!   version and resolution. Adding a variant appends outcome records only.
//!   v2 adds each exit order's last fill second, filled quantity and exit
//!   value (FM-F125); v1 stays decodable, its orders reported merged.
//!
//! A file starts with a header naming its version, its stride and its slot
//! count; a reader takes the stride from the header, never from this code
//! (`estimate_uses_header_stride`). Every stride is a sum of field widths
//! checked at compile time; the goldens in `crates/runner/tests/fib_walk.rs`
//! pin the bytes.
//!
//! No I/O: bytes in, values out (gate 22).

use core::mem::size_of;

use indicators::fibflag::types::{BaseKind, EntryStyle};
use vocab::fib_cube::{Coords, ROW_COUNT, ROWS, Search, Source};

use super::types::{
    ExitOrder, ExitReason, Resolution, SlotOutcome, SlotRefusal, SlotStatus, TradeFlags,
    UnfilledReason, UnpricedReason,
};
use super::walk::{ClockLag, Plan};

/// The decision record's first version.
pub const DECISION_RECORD_V1: u16 = 1;
/// The outcome record's first version: no exit orders.
pub const OUTCOME_RECORD_V1: u16 = 1;
/// The outcome record with each exit order (FM-F125).
pub const OUTCOME_RECORD_V2: u16 = OUTCOME_RECORD_V1 + 1;

/// How many rows of the schema are measured: one measurement per row.
pub const MEASURED_ROWS: usize = measured_rows();

/// Counts the measured rows of `vocab::fib_cube::ROWS`.
const fn measured_rows() -> usize {
    let mut count = 0;
    let mut rest: &[vocab::fib_cube::Row] = &ROWS;
    while let [row, tail @ ..] = rest {
        if matches!(row.source, Source::Measured) {
            count += 1;
        }
        rest = tail;
    }
    count
}

/// One measured row of a decision: its raw value and its exact own-past
/// mid-rank position `num / den` (R1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Measurement {
    /// The raw measurement.
    pub raw: i64,
    /// The position's numerator.
    pub position_num: u64,
    /// The position's denominator; zero when the row has no own past yet.
    pub position_den: u64,
}

/// One decision, as stage A wrote it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DecisionRecord {
    /// The decision's ordinal in its stock-month: outcome record `i` of a file
    /// belongs to decision `i`.
    pub ordinal: u32,
    /// The stock's slot in the day's point-in-time pool.
    pub stock_slot: u16,
    /// The IST day number.
    pub ist_day: i64,
    /// The plan the walker reads.
    pub plan: Plan,
    /// The base kind.
    pub kind: BaseKind,
    /// The cell.
    pub coords: Coords,
    /// Every measured row, in row order.
    pub measurements: [Measurement; MEASURED_ROWS],
}

/// One outcome record: a decision under one exit variant and one fill
/// variant, one slot per trade size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutcomeRecord {
    /// The decision's ordinal.
    pub ordinal: u32,
    /// The exit variant's id.
    pub variant: u16,
    /// The fill variant's id.
    pub fill: u16,
    /// DATA-1's Zerodha close (one at stage A).
    pub zerodha_close: i64,
    /// DATA-1's GDFL close (one at stage A).
    pub gdfl_close: i64,
    /// The stock-day's clock lag (FM-F043).
    pub clock_lag: ClockLag,
    /// One outcome per size, in the run's size order.
    pub slots: Vec<SlotOutcome>,
}

/// A decoded outcome record and how its exit orders read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedOutcome {
    /// The record.
    pub record: OutcomeRecord,
    /// True for a v1 record: it has no exit orders, so a reader books every
    /// order at the slot's exit second, labelled "exit orders merged
    /// (`OutcomeRecord` v1)".
    pub orders_merged: bool,
}

/// One file's header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Header {
    /// The record kind's magic: decisions or outcomes.
    pub kind: RecordKind,
    /// The record version.
    pub version: u16,
    /// The walk's resolution (outcomes; decisions are stage A's).
    pub resolution: Resolution,
    /// Slots per record (zero for decisions).
    pub slot_count: u16,
    /// Bytes per record.
    pub stride: u32,
    /// The set version the records belong to (the exit set's for outcomes,
    /// the schema's for decisions).
    pub set_version: u32,
    /// That set's digest.
    pub set_digest: [u8; DIGEST],
}

/// Which records a file holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordKind {
    /// [`DecisionRecord`]s.
    Decisions,
    /// [`OutcomeRecord`]s.
    Outcomes,
}

impl RecordKind {
    /// The kind's eight magic bytes.
    const fn magic(self) -> &'static [u8; MAGIC] {
        match self {
            Self::Decisions => b"BTXFIBD1",
            Self::Outcomes => b"BTXFIBO1",
        }
    }
}

/// Bytes of a digest.
const DIGEST: usize = vocab::fib_cube::DIGEST_BYTES;
/// Bytes of a magic.
const MAGIC: usize = size_of::<u64>();

/// Why bytes were refused, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecRefusal {
    /// Fewer bytes than the layout needs, or more.
    Length {
        /// The bytes the layout needs.
        expected: usize,
        /// The bytes given.
        got: usize,
    },
    /// A magic that is neither kind's.
    Magic,
    /// A version this build does not know.
    Version {
        /// The version read.
        version: u16,
    },
    /// A code that names no value of its field.
    Code {
        /// The field.
        field: &'static str,
        /// The code read.
        code: u8,
    },
    /// A record with a different slot count from its header's.
    SlotCount,
}

impl CodecRefusal {
    /// The refusal's canonical name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Length { .. } => "record_length",
            Self::Magic => "record_magic",
            Self::Version { .. } => "record_version",
            Self::Code { .. } => "record_code",
            Self::SlotCount => "record_slot_count",
        }
    }
}

/// The header's width: magic, version, resolution, slot count, stride, set
/// version, set digest.
pub const HEADER_BYTES: usize = MAGIC
    + size_of::<u16>()
    + size_of::<u8>()
    + size_of::<u16>()
    + size_of::<u32>()
    + size_of::<u32>()
    + DIGEST;

/// A plan's width.
const PLAN_BYTES: usize = size_of::<u8>()
    + size_of::<u8>()
    + size_of::<u16>()
    + size_of::<u32>()
    + size_of::<u32>()
    + size_of::<i64>()
    + size_of::<i64>()
    + size_of::<i64>()
    + size_of::<i64>()
    + size_of::<i64>()
    + size_of::<u16>();

/// One measurement's width.
const MEASUREMENT_BYTES: usize = size_of::<i64>() + size_of::<u64>() + size_of::<u64>();

/// The decision record's stride (v1).
pub const DECISION_RECORD_BYTES: usize = size_of::<u32>()
    + size_of::<u16>()
    + size_of::<i64>()
    + PLAN_BYTES
    + size_of::<u8>()
    + ROW_COUNT
    + MEASURED_ROWS * MEASUREMENT_BYTES;

/// The outcome record's fixed part: ordinal, variant, fill, factor pair,
/// clock lag (state and seconds).
pub const OUTCOME_FIXED_BYTES: usize = size_of::<u32>()
    + size_of::<u16>()
    + size_of::<u16>()
    + size_of::<i64>()
    + size_of::<i64>()
    + size_of::<u8>()
    + size_of::<i32>();

/// One v1 slot's width.
pub const SLOT_V1_BYTES: usize = size_of::<u8>() // status
    + size_of::<u8>() // status reason
    + size_of::<u8>() // exit reason
    + size_of::<u32>() // flags
    + size_of::<u64>() // filled qty
    + size_of::<i128>() // entry value
    + size_of::<i128>() // exit value
    + size_of::<i64>() // risk per share
    + size_of::<i128>() // risk numerator
    + size_of::<i64>() // risk denominator
    + size_of::<u32>() // first entry second
    + size_of::<u32>() // last entry second
    + size_of::<u32>() // exit second
    + size_of::<i64>() // mfe to exit
    + size_of::<i64>() // mae to exit
    + size_of::<i64>() // mfe to horizon
    + size_of::<i64>() // mae to horizon
    + size_of::<u64>() // decision qty
    + size_of::<u64>() // planned qty
    + size_of::<i64>() // first entry price
    + size_of::<u32>() // mfe second
    + size_of::<u32>() // last peak second
    + size_of::<u32>() // entry wait seconds
    + size_of::<u32>() // exit spill seconds
    + size_of::<u32>(); // max participation

/// One exit order's width: present, last fill second, qty, value.
const ORDER_BYTES: usize =
    size_of::<u8>() + size_of::<u32>() + size_of::<u64>() + size_of::<i128>();

/// One v2 slot's width: v1 and the own and forced exit orders.
pub const SLOT_V2_BYTES: usize = SLOT_V1_BYTES + ORDER_BYTES + ORDER_BYTES;

// The strides fit the header's u32 field for every slot count a u16 holds.
const _: () =
    assert!(OUTCOME_FIXED_BYTES + (u16::MAX as usize) * SLOT_V2_BYTES <= u32::MAX as usize);
const _: () = assert!(DECISION_RECORD_BYTES <= u32::MAX as usize);
const _: () = assert!(SLOT_V2_BYTES > SLOT_V1_BYTES);

/// One outcome record's stride at `version` with `slot_count` slots, or
/// `None` for an unknown version.
#[must_use]
pub const fn outcome_stride(version: u16, slot_count: u16) -> Option<usize> {
    let slot = match version {
        OUTCOME_RECORD_V1 => SLOT_V1_BYTES,
        OUTCOME_RECORD_V2 => SLOT_V2_BYTES,
        _ => return None,
    };
    Some(OUTCOME_FIXED_BYTES + (slot_count as usize) * slot)
}

/// The bytes a file of `records` records takes, read from its header's stride
/// (FIB-12's estimate).
#[must_use]
pub fn estimate_bytes(header: &Header, records: u64) -> u128 {
    u128::from(records) * u128::from(header.stride) + header_bytes_wide()
}

/// [`HEADER_BYTES`] widened.
fn header_bytes_wide() -> u128 {
    u128::try_from(HEADER_BYTES).unwrap_or(u128::MAX)
}

impl Header {
    /// The header of an outcome file at `version`.
    ///
    /// # Errors
    ///
    /// [`CodecRefusal::Version`] for an unknown version.
    pub fn outcomes(
        version: u16,
        resolution: Resolution,
        slot_count: u16,
        set_version: u32,
        set_digest: [u8; DIGEST],
    ) -> Result<Self, CodecRefusal> {
        let stride =
            outcome_stride(version, slot_count).ok_or(CodecRefusal::Version { version })?;
        Ok(Self {
            kind: RecordKind::Outcomes,
            version,
            resolution,
            slot_count,
            stride: u32::try_from(stride).unwrap_or(u32::MAX),
            set_version,
            set_digest,
        })
    }

    /// The header of a decision file.
    #[must_use]
    pub fn decisions(schema_version: u32, schema_digest: [u8; DIGEST]) -> Self {
        Self {
            kind: RecordKind::Decisions,
            version: DECISION_RECORD_V1,
            resolution: Resolution::Minute,
            slot_count: 0,
            stride: u32::try_from(DECISION_RECORD_BYTES).unwrap_or(u32::MAX),
            set_version: schema_version,
            set_digest: schema_digest,
        }
    }

    /// The header's bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_BYTES);
        out.extend_from_slice(self.kind.magic());
        out.extend_from_slice(&self.version.to_le_bytes());
        out.push(self.resolution as u8);
        out.extend_from_slice(&self.slot_count.to_le_bytes());
        out.extend_from_slice(&self.stride.to_le_bytes());
        out.extend_from_slice(&self.set_version.to_le_bytes());
        out.extend_from_slice(&self.set_digest);
        out
    }

    /// A header from its bytes.
    ///
    /// # Errors
    ///
    /// [`CodecRefusal::Length`], [`CodecRefusal::Magic`],
    /// [`CodecRefusal::Version`] or a resolution [`CodecRefusal::Code`].
    pub fn decode(bytes: &[u8]) -> Result<Self, CodecRefusal> {
        let mut r = Reader::new(bytes, HEADER_BYTES)?;
        let magic = r.array::<MAGIC>();
        let kind = if &magic == RecordKind::Decisions.magic() {
            RecordKind::Decisions
        } else if &magic == RecordKind::Outcomes.magic() {
            RecordKind::Outcomes
        } else {
            return Err(CodecRefusal::Magic);
        };
        let version = r.u16();
        let resolution = resolution_of(r.u8())?;
        let slot_count = r.u16();
        let stride = r.u32();
        let set_version = r.u32();
        let set_digest = r.array::<DIGEST>();
        let known = match kind {
            RecordKind::Decisions => version == DECISION_RECORD_V1,
            RecordKind::Outcomes => outcome_stride(version, slot_count).is_some(),
        };
        if !known {
            return Err(CodecRefusal::Version { version });
        }
        Ok(Self {
            kind,
            version,
            resolution,
            slot_count,
            stride,
            set_version,
            set_digest,
        })
    }
}

/// A resolution from its code.
fn resolution_of(code: u8) -> Result<Resolution, CodecRefusal> {
    [Resolution::Minute, Resolution::Second]
        .into_iter()
        .find(|r| *r as u8 == code)
        .ok_or(CodecRefusal::Code {
            field: "resolution",
            code,
        })
}

/// A little-endian reader over exactly the bytes a layout needs.
struct Reader<'a> {
    /// What is left.
    rest: &'a [u8],
}

impl<'a> Reader<'a> {
    /// A reader of exactly `expected` bytes.
    fn new(bytes: &'a [u8], expected: usize) -> Result<Self, CodecRefusal> {
        if bytes.len() != expected {
            return Err(CodecRefusal::Length {
                expected,
                got: bytes.len(),
            });
        }
        Ok(Self { rest: bytes })
    }

    /// The next `N` bytes. The length was checked once, so this never runs
    /// short; a short read would answer zeros, never past the end.
    fn array<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0; N];
        let (head, tail) = self.rest.split_at(N.min(self.rest.len()));
        for (slot, byte) in out.iter_mut().zip(head) {
            *slot = *byte;
        }
        self.rest = tail;
        out
    }

    /// The next `u8`.
    fn u8(&mut self) -> u8 {
        u8::from_le_bytes(self.array())
    }

    /// The next `u16`.
    fn u16(&mut self) -> u16 {
        u16::from_le_bytes(self.array())
    }

    /// The next `u32`.
    fn u32(&mut self) -> u32 {
        u32::from_le_bytes(self.array())
    }

    /// The next `i32`.
    fn i32(&mut self) -> i32 {
        i32::from_le_bytes(self.array())
    }

    /// The next `u64`.
    fn u64(&mut self) -> u64 {
        u64::from_le_bytes(self.array())
    }

    /// The next `i64`.
    fn i64(&mut self) -> i64 {
        i64::from_le_bytes(self.array())
    }

    /// The next `i128`.
    fn i128(&mut self) -> i128 {
        i128::from_le_bytes(self.array())
    }
}

/// A value's code in its field, append-only: its position in `all`.
fn code_of<T: PartialEq>(all: &[T], value: &T) -> u8 {
    let mut code = 0;
    for (at, candidate) in all.iter().enumerate() {
        if candidate == value {
            code = u8::try_from(at).unwrap_or(u8::MAX);
        }
    }
    code
}

/// The value at `code` in its field.
fn value_of<T: Copy>(field: &'static str, all: &[T], code: u8) -> Result<T, CodecRefusal> {
    all.get(usize::from(code))
        .copied()
        .ok_or(CodecRefusal::Code { field, code })
}

/// The directions, by code.
const DIRECTIONS: &[Search] = &[Search::Long, Search::Short];
/// The entry styles, by code (the cube's entry-style order).
const STYLES: &[EntryStyle] = &[EntryStyle::Close, EntryStyle::Break, EntryStyle::Retest];
/// The base kinds, by code (the cube's base-kind order).
const KINDS: &[BaseKind] = &[BaseKind::Micro2, BaseKind::Micro3, BaseKind::Rest];
/// The exit reasons, by code (their precedence order).
const REASONS: &[ExitReason] = &[
    ExitReason::StopLoss,
    ExitReason::TrailingStopLoss,
    ExitReason::TrailingTakeProfit,
    ExitReason::TakeProfit,
    ExitReason::Time,
    ExitReason::Forced,
];
/// The unfilled reasons, by code.
const UNFILLED: &[UnfilledReason] = &[
    UnfilledReason::WindowExpired,
    UnfilledReason::TargetBeforeFill,
    UnfilledReason::FeedHole,
    UnfilledReason::NeverTriggered,
];
/// The unpriced reasons, by code.
const UNPRICED: &[UnpricedReason] = &[
    UnpricedReason::ForcedThin,
    UnpricedReason::ForcedLockedLong,
    UnpricedReason::ForcedLockedShort,
    UnpricedReason::ForcedCheap,
    UnpricedReason::ExecutionFeedHole,
    UnpricedReason::CorruptBarInTrade,
    UnpricedReason::Halt,
];
/// The slot refusals, by code.
const REFUSALS: &[SlotRefusal] = &[
    SlotRefusal::SizeBelowOneShare,
    SlotRefusal::NonpositivePlannedRisk,
];

/// A status's two codes: its kind and its reason.
fn status_codes(status: SlotStatus) -> (u8, u8) {
    match status {
        SlotStatus::Priced => (StatusCode::Priced as u8, 0),
        SlotStatus::EntryUnfilled(why) => {
            (StatusCode::EntryUnfilled as u8, code_of(UNFILLED, &why))
        }
        SlotStatus::Unpriceable(why) => (StatusCode::Unpriceable as u8, code_of(UNPRICED, &why)),
        SlotStatus::Refused(why) => (StatusCode::Refused as u8, code_of(REFUSALS, &why)),
        SlotStatus::NotTradable => (StatusCode::NotTradable as u8, 0),
    }
}

/// A status from its two codes.
fn status_of(kind: u8, reason: u8) -> Result<SlotStatus, CodecRefusal> {
    let refused = CodecRefusal::Code {
        field: "status",
        code: kind,
    };
    let kinds = [
        StatusCode::Priced,
        StatusCode::EntryUnfilled,
        StatusCode::Unpriceable,
        StatusCode::Refused,
        StatusCode::NotTradable,
    ];
    match kinds
        .into_iter()
        .find(|k| *k as u8 == kind)
        .ok_or(refused)?
    {
        StatusCode::Priced => Ok(SlotStatus::Priced),
        StatusCode::EntryUnfilled => Ok(SlotStatus::EntryUnfilled(value_of(
            "unfilled_reason",
            UNFILLED,
            reason,
        )?)),
        StatusCode::Unpriceable => Ok(SlotStatus::Unpriceable(value_of(
            "unpriced_reason",
            UNPRICED,
            reason,
        )?)),
        StatusCode::Refused => Ok(SlotStatus::Refused(value_of(
            "slot_refusal",
            REFUSALS,
            reason,
        )?)),
        StatusCode::NotTradable => Ok(SlotStatus::NotTradable),
    }
}

/// A status kind's code, append-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StatusCode {
    Priced,
    EntryUnfilled,
    Unpriceable,
    Refused,
    NotTradable,
}

/// A clock lag's state code and seconds.
fn lag_codes(lag: ClockLag) -> (u8, i32) {
    match lag {
        ClockLag::Measured(seconds) => (LagCode::Measured as u8, seconds),
        ClockLag::Unmeasured => (LagCode::Unmeasured as u8, 0),
        ClockLag::TrustUnchecked => (LagCode::TrustUnchecked as u8, 0),
    }
}

/// A clock lag from its codes.
fn lag_of(code: u8, seconds: i32) -> Result<ClockLag, CodecRefusal> {
    match [
        LagCode::Measured,
        LagCode::Unmeasured,
        LagCode::TrustUnchecked,
    ]
    .into_iter()
    .find(|k| *k as u8 == code)
    {
        Some(LagCode::Measured) => Ok(ClockLag::Measured(seconds)),
        Some(LagCode::Unmeasured) => Ok(ClockLag::Unmeasured),
        Some(LagCode::TrustUnchecked) => Ok(ClockLag::TrustUnchecked),
        None => Err(CodecRefusal::Code {
            field: "clock_lag",
            code,
        }),
    }
}

/// A clock lag state's code, append-only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LagCode {
    Measured,
    Unmeasured,
    TrustUnchecked,
}

impl DecisionRecord {
    /// The record's bytes: exactly [`DECISION_RECORD_BYTES`].
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(DECISION_RECORD_BYTES);
        out.extend_from_slice(&self.ordinal.to_le_bytes());
        out.extend_from_slice(&self.stock_slot.to_le_bytes());
        out.extend_from_slice(&self.ist_day.to_le_bytes());
        let p = &self.plan;
        out.push(code_of(DIRECTIONS, &p.direction));
        out.push(code_of(STYLES, &p.style));
        out.extend_from_slice(&p.tf_minutes.to_le_bytes());
        out.extend_from_slice(&p.decision_second.to_le_bytes());
        out.extend_from_slice(&p.entry_window_end.to_le_bytes());
        for price in [
            p.entry_ref,
            p.base_edge,
            p.trigger_edge,
            p.fib_zero,
            p.fib_one,
        ] {
            out.extend_from_slice(&price.to_le_bytes());
        }
        out.extend_from_slice(&p.trail.to_le_bytes());
        out.push(code_of(KINDS, &self.kind));
        out.extend_from_slice(&self.coords);
        for m in &self.measurements {
            out.extend_from_slice(&m.raw.to_le_bytes());
            out.extend_from_slice(&m.position_num.to_le_bytes());
            out.extend_from_slice(&m.position_den.to_le_bytes());
        }
        out
    }

    /// A record from its bytes.
    ///
    /// # Errors
    ///
    /// [`CodecRefusal::Length`] or a [`CodecRefusal::Code`].
    pub fn decode(bytes: &[u8]) -> Result<Self, CodecRefusal> {
        let mut r = Reader::new(bytes, DECISION_RECORD_BYTES)?;
        let ordinal = r.u32();
        let stock_slot = r.u16();
        let ist_day = r.i64();
        let direction = value_of("direction", DIRECTIONS, r.u8())?;
        let style = value_of("entry_style", STYLES, r.u8())?;
        let plan = Plan {
            direction,
            style,
            tf_minutes: r.u16(),
            decision_second: r.u32(),
            entry_window_end: r.u32(),
            entry_ref: r.i64(),
            base_edge: r.i64(),
            trigger_edge: r.i64(),
            fib_zero: r.i64(),
            fib_one: r.i64(),
            trail: r.u16(),
        };
        let kind = value_of("base_kind", KINDS, r.u8())?;
        let coords = r.array::<ROW_COUNT>();
        let mut measurements = [Measurement {
            raw: 0,
            position_num: 0,
            position_den: 0,
        }; MEASURED_ROWS];
        for m in &mut measurements {
            m.raw = r.i64();
            m.position_num = r.u64();
            m.position_den = r.u64();
        }
        Ok(Self {
            ordinal,
            stock_slot,
            ist_day,
            plan,
            kind,
            coords,
            measurements,
        })
    }
}

impl OutcomeRecord {
    /// The record's bytes at `version`: exactly its [`outcome_stride`].
    ///
    /// # Errors
    ///
    /// [`CodecRefusal::Version`] for an unknown version, and
    /// [`CodecRefusal::SlotCount`] for more slots than a `u16` counts.
    pub fn encode(&self, version: u16) -> Result<Vec<u8>, CodecRefusal> {
        let slots = u16::try_from(self.slots.len()).map_err(|_| CodecRefusal::SlotCount)?;
        let stride = outcome_stride(version, slots).ok_or(CodecRefusal::Version { version })?;
        let mut out = Vec::with_capacity(stride);
        out.extend_from_slice(&self.ordinal.to_le_bytes());
        out.extend_from_slice(&self.variant.to_le_bytes());
        out.extend_from_slice(&self.fill.to_le_bytes());
        out.extend_from_slice(&self.zerodha_close.to_le_bytes());
        out.extend_from_slice(&self.gdfl_close.to_le_bytes());
        let (state, seconds) = lag_codes(self.clock_lag);
        out.push(state);
        out.extend_from_slice(&seconds.to_le_bytes());
        for slot in &self.slots {
            put_slot(&mut out, slot);
            if version == OUTCOME_RECORD_V2 {
                put_order(&mut out, slot.own_exit);
                put_order(&mut out, slot.forced_exit);
            }
        }
        Ok(out)
    }

    /// A record from its bytes, as the file's header describes them.
    ///
    /// # Errors
    ///
    /// [`CodecRefusal::Version`], [`CodecRefusal::Length`] or a
    /// [`CodecRefusal::Code`].
    pub fn decode(header: &Header, bytes: &[u8]) -> Result<DecodedOutcome, CodecRefusal> {
        let stride =
            outcome_stride(header.version, header.slot_count).ok_or(CodecRefusal::Version {
                version: header.version,
            })?;
        let mut r = Reader::new(bytes, stride)?;
        let ordinal = r.u32();
        let variant = r.u16();
        let fill = r.u16();
        let zerodha_close = r.i64();
        let gdfl_close = r.i64();
        let state = r.u8();
        let clock_lag = lag_of(state, r.i32())?;
        let mut slots = Vec::with_capacity(usize::from(header.slot_count));
        for _ in 0..header.slot_count {
            let mut slot = take_slot(&mut r)?;
            if header.version == OUTCOME_RECORD_V2 {
                slot.own_exit = take_order(&mut r);
                slot.forced_exit = take_order(&mut r);
            }
            slots.push(slot);
        }
        Ok(DecodedOutcome {
            record: Self {
                ordinal,
                variant,
                fill,
                zerodha_close,
                gdfl_close,
                clock_lag,
                slots,
            },
            orders_merged: header.version == OUTCOME_RECORD_V1,
        })
    }
}

/// A slot's v1 bytes.
fn put_slot(out: &mut Vec<u8>, s: &SlotOutcome) {
    let (kind, reason) = status_codes(s.status);
    out.push(kind);
    out.push(reason);
    out.push(code_of(REASONS, &s.exit_reason));
    out.extend_from_slice(&s.flags.0.to_le_bytes());
    out.extend_from_slice(&s.filled_qty.to_le_bytes());
    out.extend_from_slice(&s.entry_value_paisa.to_le_bytes());
    out.extend_from_slice(&s.exit_value_paisa.to_le_bytes());
    out.extend_from_slice(&s.risk_per_share_paisa.to_le_bytes());
    out.extend_from_slice(&s.risk_num.to_le_bytes());
    out.extend_from_slice(&s.risk_den.to_le_bytes());
    for second in [s.first_entry_second, s.last_entry_second, s.exit_second] {
        out.extend_from_slice(&second.to_le_bytes());
    }
    for excursion in [
        s.mfe_to_exit_paisa,
        s.mae_to_exit_paisa,
        s.mfe_to_horizon_paisa,
        s.mae_to_horizon_paisa,
    ] {
        out.extend_from_slice(&excursion.to_le_bytes());
    }
    out.extend_from_slice(&s.decision_qty.to_le_bytes());
    out.extend_from_slice(&s.planned_qty.to_le_bytes());
    out.extend_from_slice(&s.first_entry_price_paisa.to_le_bytes());
    for count in [
        s.mfe_second,
        s.last_peak_second,
        s.entry_wait_seconds,
        s.exit_spill_seconds,
        s.max_participation_permille,
    ] {
        out.extend_from_slice(&count.to_le_bytes());
    }
}

/// A slot from its v1 bytes, with no exit orders.
fn take_slot(r: &mut Reader<'_>) -> Result<SlotOutcome, CodecRefusal> {
    let kind = r.u8();
    let reason = r.u8();
    let status = status_of(kind, reason)?;
    let exit_reason = value_of("exit_reason", REASONS, r.u8())?;
    Ok(SlotOutcome {
        status,
        exit_reason,
        flags: TradeFlags(r.u32()),
        filled_qty: r.u64(),
        entry_value_paisa: r.i128(),
        exit_value_paisa: r.i128(),
        risk_per_share_paisa: r.i64(),
        risk_num: r.i128(),
        risk_den: r.i64(),
        first_entry_second: r.u32(),
        last_entry_second: r.u32(),
        exit_second: r.u32(),
        mfe_to_exit_paisa: r.i64(),
        mae_to_exit_paisa: r.i64(),
        mfe_to_horizon_paisa: r.i64(),
        mae_to_horizon_paisa: r.i64(),
        decision_qty: r.u64(),
        planned_qty: r.u64(),
        first_entry_price_paisa: r.i64(),
        mfe_second: r.u32(),
        last_peak_second: r.u32(),
        entry_wait_seconds: r.u32(),
        exit_spill_seconds: r.u32(),
        max_participation_permille: r.u32(),
        own_exit: None,
        forced_exit: None,
    })
}

/// An exit order's bytes: a presence byte, then its fields (zeros when
/// absent).
fn put_order(out: &mut Vec<u8>, order: Option<ExitOrder>) {
    let o = order.unwrap_or(ExitOrder {
        last_fill_second: 0,
        filled_qty: 0,
        exit_value_paisa: 0,
    });
    out.push(u8::from(order.is_some()));
    out.extend_from_slice(&o.last_fill_second.to_le_bytes());
    out.extend_from_slice(&o.filled_qty.to_le_bytes());
    out.extend_from_slice(&o.exit_value_paisa.to_le_bytes());
}

/// An exit order from its bytes.
fn take_order(r: &mut Reader<'_>) -> Option<ExitOrder> {
    let present = r.u8() != 0;
    let order = ExitOrder {
        last_fill_second: r.u32(),
        filled_qty: r.u64(),
        exit_value_paisa: r.i128(),
    };
    present.then_some(order)
}
