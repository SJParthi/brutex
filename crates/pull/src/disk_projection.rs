//! The shared disk projection and its free-space floor (PERF-13, R4-07).
//!
//! # What it answers
//!
//! "How many bytes will this plan write, and may it start?" — for every step
//! that writes many files: the GDFL Dropbox fetch ([`crate::gdfl_dropbox`],
//! which checks it before every file), the ingest chain (R4-11), the Zerodha
//! plan (R4-23) and the GDFL preflight and `/disk.json` (R4-29). One module,
//! so the bytes a day, the median rule and the floor have one answer.
//!
//! # Measured, never invented
//!
//! A [`Totals`] is filled by its caller from what the store already counts:
//! the census's bar bytes and held instrument-days per feed, kind and rung
//! (the sums `/disk.json` computes), and each option underlying's stored
//! bytes and held days. Bytes a day is `bytes / days`, rounded up. An option
//! underlying with no stored months (FINNIFTY, MIDCPNIFTY, NIFTYNXT50 and
//! every other unimported one) is projected from the MEDIAN bytes a day of
//! the held underlyings of its class (index or stock options), and its line
//! says so: [`Basis::Extrapolated`] names the class and how many underlyings
//! the median is of (`CLAUDE.md` §3 rule 6: an extrapolation is labelled).
//! A unit nothing measures is [`Basis::Unmeasured`] and is never read as
//! zero: [`check_plan`] refuses it by name.
//!
//! # The floor refuses, and it refuses first
//!
//! [`floor_check`] refuses `disk_floor` when the free bytes less the
//! projected bytes would fall below the floor, BEFORE any file is read or
//! written. This supersedes, for the floor only, the design's "an
//! extrapolation never refuses" (ingest-automation.md, free space against the
//! projected growth): a projection may be wrong by its tolerance, and a full
//! disk mid-write is the one failure the store cannot undo (a writable
//! mapping raises a signal on a full disk, `CLAUDE.md` §4). D-4987.
//!
//! The floor is the `disk.floor` setting ([`store::settings::Floor`], one
//! reader: [`store::settings::disk_floor`]); unset, it is [`DEFAULT_FLOOR`],
//! decided by measurement ([`DEFAULT_FLOOR_SOURCE`]).
//!
//! # Cost
//!
//! [`Totals::seal`] sorts each class's bytes a day once, O(u log u) over the
//! held underlyings; every projected unit after that is one hash probe or one
//! cached median, O(1), so a plan is O(its units) and independent of how many
//! kinds and underlyings the totals hold (bench C-DP-01, run by
//! `crates/pull/benches/ratio.rs`, at 1x, 10x and 100x; R407-07).
//!
//! The free space comes from the caller's [`FreeSpace`]: this crate starts no
//! program (gate 0, D-1603). The app's meter is the `/bin/df -Pk` that
//! `/disk.json` already runs (`crates/api/src/disk_route.rs`), whose output
//! [`parse_df`] reads; wiring it to the fetch is the ingest chain's (R4-11).

use std::collections::HashMap;
use std::path::Path;

pub use store::settings::Floor;

/// The floor when `disk.floor` is unset: 15 percent of the store's
/// filesystem.
pub const DEFAULT_FLOOR: Floor = Floor::Percent(15);

/// Where [`DEFAULT_FLOOR`] came from, in words.
pub const DEFAULT_FLOOR_SOURCE: &str = "built-in default (R4-07, D-4987): 15% of the store's \
     filesystem, measured on 16 Oct 2026 with df -Pk on the store's drive (7,813,821,744 KiB, \
     1,770,133,020 KiB free, 22.6%): 15% keeps 1.09 TiB free and leaves 0.56 TiB to write; \
     the operator's 1 TB rule for the equities scope is the byte floor 1TB";

/// The tolerance a fixture projection must land within (percent of the
/// actual bytes).
pub const FIXTURE_TOLERANCE_PERCENT: u64 = 10;

/// The tolerance the real options resume's first 30 days are checked
/// against (percent of the actual bytes), recorded by the store-steps check.
pub const RESUME_TOLERANCE_PERCENT: u64 = 20;

/// One measured kind: a feed, a kind of instrument and a rung.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KindKey {
    /// The feed's wire word (`gdfl`, `zerodha`).
    pub feed: String,
    /// The kind (`index`, `stock`, `option`, or a census kind word).
    pub kind: String,
    /// The rung (`1s`, `1min`, `1day`, or `ticks`).
    pub rung: String,
}

impl KindKey {
    /// A key of three words.
    #[must_use]
    pub fn new(feed: &str, kind: &str, rung: &str) -> Self {
        Self {
            feed: feed.to_owned(),
            kind: kind.to_owned(),
            rung: rung.to_owned(),
        }
    }
}

impl core::fmt::Display for KindKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}/{}/{}", self.feed, self.kind, self.rung)
    }
}

/// Which median an unmeasured option underlying is projected from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OptionClass {
    /// An index's options.
    Index,
    /// A share's options.
    Stock,
}

impl OptionClass {
    /// The class's word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Index => "index_options",
            Self::Stock => "stock_options",
        }
    }

    const fn slot(self) -> usize {
        match self {
            Self::Index => 0,
            Self::Stock => 1,
        }
    }
}

/// Bytes the store holds for a unit and the instrument-days they cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measured {
    /// The bytes.
    pub bytes: u64,
    /// The instrument-days those bytes cover; never zero.
    pub days: u64,
}

impl Measured {
    /// Bytes a day, rounded up so a projection never promises less.
    #[must_use]
    pub const fn per_day(self) -> u64 {
        self.bytes.div_ceil(self.days)
    }
}

/// Why a sample was not taken into a [`Totals`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleRefusal {
    /// A sample covering no day has no bytes a day; it is not read as zero.
    NoDays {
        /// The unit, in words.
        unit: String,
    },
    /// The unit was measured twice; the first is kept.
    Duplicate {
        /// The unit, in words.
        unit: String,
    },
}

impl SampleRefusal {
    /// The refusal's code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NoDays { .. } => "sample_holds_no_day",
            Self::Duplicate { .. } => "sample_duplicate",
        }
    }
}

impl core::fmt::Display for SampleRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoDays { unit } => write!(
                f,
                "sample_holds_no_day: {unit} covers no day, so it has no bytes a day"
            ),
            Self::Duplicate { unit } => {
                write!(
                    f,
                    "sample_duplicate: {unit} was measured twice; the first is kept"
                )
            }
        }
    }
}

/// The measured totals a projection reads, before [`Totals::seal`].
#[derive(Debug, Clone, Default)]
pub struct Totals {
    kinds: HashMap<KindKey, Measured>,
    underlyings: HashMap<String, (OptionClass, Measured)>,
}

impl Totals {
    /// No samples.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// One kind's bytes and held instrument-days. O(1): one hash insert, not
    /// timed on its own (UNVERIFIED, docs/06 L-R407-11).
    ///
    /// # Errors
    ///
    /// [`SampleRefusal::NoDays`] for zero days, [`SampleRefusal::Duplicate`]
    /// for a kind already measured.
    pub fn add_kind(&mut self, key: KindKey, bytes: u64, days: u64) -> Result<(), SampleRefusal> {
        if days == 0 {
            return Err(SampleRefusal::NoDays {
                unit: key.to_string(),
            });
        }
        if self.kinds.contains_key(&key) {
            return Err(SampleRefusal::Duplicate {
                unit: key.to_string(),
            });
        }
        self.kinds.insert(key, Measured { bytes, days });
        Ok(())
    }

    /// One option underlying's stored bytes and held days. O(1): one hash
    /// insert, not timed on its own (UNVERIFIED, docs/06 L-R407-11).
    ///
    /// # Errors
    ///
    /// As [`Totals::add_kind`].
    pub fn add_underlying(
        &mut self,
        name: &str,
        class: OptionClass,
        bytes: u64,
        days: u64,
    ) -> Result<(), SampleRefusal> {
        if days == 0 {
            return Err(SampleRefusal::NoDays {
                unit: name.to_owned(),
            });
        }
        if self.underlyings.contains_key(name) {
            return Err(SampleRefusal::Duplicate {
                unit: name.to_owned(),
            });
        }
        self.underlyings
            .insert(name.to_owned(), (class, Measured { bytes, days }));
        Ok(())
    }

    /// The totals with each class's median bytes a day computed once,
    /// O(u log u) over the held underlyings.
    #[must_use]
    pub fn seal(self) -> Sealed {
        let mut per_class: [Vec<u64>; 2] = [Vec::new(), Vec::new()];
        for (class, measured) in self.underlyings.values() {
            if let Some(list) = per_class.get_mut(class.slot()) {
                list.push(measured.per_day());
            }
        }
        let medians = per_class.map(|mut list| {
            list.sort_unstable();
            median(&list).map(|value| (value, list.len()))
        });
        Sealed {
            kinds: self.kinds,
            underlyings: self.underlyings,
            medians,
        }
    }
}

/// The median of sorted `list`: the middle value, or the two middle values'
/// mean rounded up. `None` for an empty list.
fn median(list: &[u64]) -> Option<u64> {
    let n = list.len();
    if n == 0 {
        return None;
    }
    let high = *list.get(n / 2)?;
    if n % 2 == 1 {
        return Some(high);
    }
    let low = *list.get(n / 2 - 1)?;
    Some(low + (high - low).div_ceil(2))
}

/// Measured totals with their medians: what [`project`] reads.
#[derive(Debug, Clone)]
pub struct Sealed {
    kinds: HashMap<KindKey, Measured>,
    underlyings: HashMap<String, (OptionClass, Measured)>,
    medians: [Option<(u64, usize)>; 2],
}

impl Sealed {
    /// How many kinds and underlyings were measured.
    #[must_use]
    pub fn len(&self) -> usize {
        self.kinds.len() + self.underlyings.len()
    }

    /// Whether nothing was measured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The median bytes a day of `class`'s held underlyings and how many
    /// they are, or `None` when none is held.
    #[must_use]
    pub fn median_of(&self, class: OptionClass) -> Option<(u64, usize)> {
        self.medians.get(class.slot()).copied().flatten()
    }

    /// One unit's bytes a day and how that number was reached. O(1), timed
    /// through [`project`] by C-DP-01 in `crates/pull/benches/ratio.rs`.
    #[must_use]
    pub fn per_day(&self, unit: &Unit) -> (u64, Basis) {
        match unit {
            Unit::Kind(key) => self.kinds.get(key).map_or((0, Basis::Unmeasured), |m| {
                (m.per_day(), Basis::Measured(*m))
            }),
            Unit::Underlying { name, class } => {
                if let Some((_, measured)) = self.underlyings.get(name) {
                    return (measured.per_day(), Basis::Measured(*measured));
                }
                self.median_of(*class)
                    .map_or((0, Basis::Unmeasured), |(value, of)| {
                        (
                            value,
                            Basis::Extrapolated {
                                class: *class,
                                median_of: of,
                            },
                        )
                    })
            }
        }
    }
}

/// What a plan line projects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unit {
    /// A measured kind: its bytes a day are per instrument-day.
    Kind(KindKey),
    /// One option underlying's chain.
    Underlying {
        /// The underlying's store symbol.
        name: String,
        /// Its class, whose median stands in when it holds no month.
        class: OptionClass,
    },
}

impl core::fmt::Display for Unit {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Kind(key) => key.fmt(f),
            Self::Underlying { name, class } => write!(f, "{name} ({})", class.as_str()),
        }
    }
}

/// How a line's bytes a day were reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// From the unit's own stored bytes and days.
    Measured(Measured),
    /// EXTRAPOLATION: the median of the held underlyings of `class`.
    Extrapolated {
        /// The class whose median was taken.
        class: OptionClass,
        /// How many held underlyings the median is of.
        median_of: usize,
    },
    /// Nothing measures this unit: NOT KNOWN, never zero.
    Unmeasured,
}

/// One unit to write: `days` days of `instruments` instruments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanItem {
    /// What is written.
    pub unit: Unit,
    /// The days left to write.
    pub days: u64,
    /// The instruments a day (1 for an underlying's chain).
    pub instruments: u64,
}

/// One projected line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The unit.
    pub unit: Unit,
    /// Bytes a day.
    pub per_day: u64,
    /// Days.
    pub days: u64,
    /// Instruments a day.
    pub instruments: u64,
    /// `days x instruments x per_day`, saturating.
    pub bytes: u64,
    /// How `per_day` was reached.
    pub basis: Basis,
}

/// A plan's projection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Projection {
    /// Every line, in the plan's order.
    pub lines: Vec<Line>,
    /// The projected bytes, saturating.
    pub bytes: u64,
    /// The units projected by extrapolation, named.
    pub extrapolated: Vec<String>,
    /// The units nothing measures, named.
    pub unmeasured: Vec<String>,
}

impl Projection {
    /// One line of words for a page: the bytes, then every extrapolated and
    /// unmeasured unit by name.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut text = format!(
            "projected {} bytes over {} lines",
            self.bytes,
            self.lines.len()
        );
        if !self.extrapolated.is_empty() {
            text.push_str("; EXTRAPOLATION (median of the held underlyings of their class) for ");
            text.push_str(&self.extrapolated.join(", "));
        }
        if !self.unmeasured.is_empty() {
            text.push_str("; NOT KNOWN (nothing measures) for ");
            text.push_str(&self.unmeasured.join(", "));
        }
        text
    }
}

/// `plan` projected over `totals`: `days x instruments x bytes a day` for
/// each item. O(items), each one hash probe.
#[must_use]
pub fn project(totals: &Sealed, plan: &[PlanItem]) -> Projection {
    let mut out = Projection {
        lines: Vec::with_capacity(plan.len()),
        ..Projection::default()
    };
    for item in plan {
        let (per_day, basis) = totals.per_day(&item.unit);
        let bytes = item
            .days
            .saturating_mul(item.instruments)
            .saturating_mul(per_day);
        match basis {
            Basis::Extrapolated { .. } => out.extrapolated.push(item.unit.to_string()),
            Basis::Unmeasured => out.unmeasured.push(item.unit.to_string()),
            Basis::Measured(_) => {}
        }
        out.bytes = out.bytes.saturating_add(bytes);
        out.lines.push(Line {
            unit: item.unit.clone(),
            per_day,
            days: item.days,
            instruments: item.instruments,
            bytes,
            basis,
        });
    }
    out
}

/// Whether `projected` lands within `percent` of `actual`:
/// `|projected - actual| * 100 <= percent * actual`. Integer only.
#[must_use]
pub fn within(projected: u64, actual: u64, percent: u64) -> bool {
    let gap = u128::from(projected.abs_diff(actual)) * 100;
    gap <= u128::from(percent) * u128::from(actual)
}

/// A filesystem's size and free bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    /// The filesystem's total bytes.
    pub total: u64,
    /// The bytes available to this process.
    pub free: u64,
}

/// Where free space is measured.
pub trait FreeSpace: Send + Sync {
    /// The filesystem `path` is on, measured now.
    ///
    /// # Errors
    ///
    /// The sentence naming why it could not be measured.
    fn space(&self, path: &Path) -> Result<Space, String>;
}

/// The filesystem in POSIX `df -Pk` output: a header line, then
/// `<name> <1024-blocks> <used> <available> <capacity%> <mounted on>`; the
/// capacity column is found by its `%`, so a mount point with spaces reads.
///
/// # Errors
///
/// The sentence naming what in the output could not be read.
pub fn parse_df(text: &str) -> Result<Space, String> {
    let line = text
        .lines()
        .rfind(|line| !line.trim().is_empty())
        .ok_or_else(|| "df printed nothing".to_owned())?;
    let words: Vec<&str> = line.split_whitespace().collect();
    let at = words
        .iter()
        .rposition(|word| word.ends_with('%'))
        .filter(|at| *at >= 4)
        .ok_or_else(|| format!("df line {line:?} has no capacity column"))?;
    let kib = |offset: usize| -> Result<u64, String> {
        words
            .get(at - offset)
            .and_then(|word| word.parse::<u64>().ok())
            .map(|blocks| blocks.saturating_mul(1024))
            .ok_or_else(|| format!("df line {line:?} has no number {offset} before its capacity"))
    };
    Ok(Space {
        total: kib(3)?,
        free: kib(1)?,
    })
}

/// What a passed floor check leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Headroom {
    /// The filesystem as measured.
    pub space: Space,
    /// The floor in bytes.
    pub floor_bytes: u64,
    /// The projected bytes.
    pub projected: u64,
    /// The free bytes after the projection is written.
    pub left_after: u64,
}

/// Why a plan may not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FloorRefusal {
    /// Writing the projection would leave less than the floor free.
    DiskFloor {
        /// The filesystem as measured.
        space: Space,
        /// The floor in bytes.
        floor_bytes: u64,
        /// The projected bytes.
        projected: u64,
        /// How many bytes short of the floor the plan would leave it.
        short_by: u64,
    },
    /// A unit of the plan has no measured bytes a day: its cost is NOT
    /// KNOWN, so no floor can be checked for it.
    ProjectionUnknown {
        /// The units, named.
        units: Vec<String>,
    },
    /// The free space could not be measured.
    SpaceUnknown {
        /// Why.
        why: String,
    },
}

impl FloorRefusal {
    /// The refusal's code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::DiskFloor { .. } => "disk_floor",
            Self::ProjectionUnknown { .. } => "projection_unknown",
            Self::SpaceUnknown { .. } => "free_space_unknown",
        }
    }
}

impl core::fmt::Display for FloorRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DiskFloor {
                space,
                floor_bytes,
                projected,
                short_by,
            } => write!(
                f,
                "disk_floor: {projected} bytes to write with {} free of {} would leave \
                 {short_by} bytes less than the floor of {floor_bytes}; nothing was read or written",
                space.free, space.total
            ),
            Self::ProjectionUnknown { units } => write!(
                f,
                "projection_unknown: nothing measures {}, so the floor cannot be checked; \
                 nothing was read or written",
                units.join(", ")
            ),
            Self::SpaceUnknown { why } => write!(
                f,
                "free_space_unknown: {why}; the floor cannot be checked, so nothing was read or \
                 written"
            ),
        }
    }
}

/// Whether writing `projected` bytes on `space` keeps `floor` free. O(1):
/// integer arithmetic, not timed (UNVERIFIED, docs/06 L-R407-11).
///
/// # Errors
///
/// [`FloorRefusal::DiskFloor`] when it would not.
pub fn floor_check(space: Space, projected: u64, floor: Floor) -> Result<Headroom, FloorRefusal> {
    // A meter answer no filesystem gives would make the floor 0 (a percent
    // of a 0 total) and pass: refused by name, never checked against.
    if space.total == 0 || space.free > space.total {
        return Err(FloorRefusal::SpaceUnknown {
            why: format!(
                "the free-space meter answered {} bytes free of {} total, which no filesystem has",
                space.free, space.total
            ),
        });
    }
    let floor_bytes = floor.bytes_of(space.total);
    let need = projected.saturating_add(floor_bytes);
    if space.free < need {
        return Err(FloorRefusal::DiskFloor {
            space,
            floor_bytes,
            projected,
            short_by: need - space.free,
        });
    }
    Ok(Headroom {
        space,
        floor_bytes,
        projected,
        left_after: space.free - projected,
    })
}

/// [`floor_check`] for a whole projection, with the free space measured by
/// `meter` at `path`: an unmeasured unit or an unmeasurable disk refuses by
/// name, never passes. O(1) beyond the measurement: one [`floor_check`],
/// not timed (UNVERIFIED, docs/06 L-R407-11).
///
/// # Errors
///
/// [`FloorRefusal::ProjectionUnknown`], [`FloorRefusal::SpaceUnknown`] or
/// [`FloorRefusal::DiskFloor`].
pub fn check_plan(
    meter: &dyn FreeSpace,
    path: &Path,
    projection: &Projection,
    floor: Floor,
) -> Result<Headroom, FloorRefusal> {
    if !projection.unmeasured.is_empty() {
        return Err(FloorRefusal::ProjectionUnknown {
            units: projection.unmeasured.clone(),
        });
    }
    let space = meter
        .space(path)
        .map_err(|why| FloorRefusal::SpaceUnknown { why })?;
    floor_check(space, projection.bytes, floor)
}

/// The floor `value` names, or [`DEFAULT_FLOOR`] when it is unset, with the
/// words saying which.
///
/// # Errors
///
/// The sentence [`store::settings::disk_floor`] refuses a value with: a
/// malformed floor is a refusal, never the default (`CLAUDE.md` §4).
pub fn floor_of(value: Option<&str>) -> Result<(Floor, &'static str), String> {
    match value {
        None => Ok((DEFAULT_FLOOR, DEFAULT_FLOOR_SOURCE)),
        Some(text) => {
            store::settings::disk_floor(text).map(|floor| (floor, "the disk.floor setting"))
        }
    }
}

#[cfg(test)]
#[path = "disk_projection_tests.rs"]
mod tests;
