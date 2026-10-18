//! The generated exit set and fill set (FIB-5; D-6380; operator 16 Oct 13:00,
//! 13:05 and 17:15 IST).
//!
//! An exit is a COMPOSITION of optional components, each present or absent:
//! a stop loss, a take-profit, a trailing take-profit, a trailing stop and a
//! time exit; the 15:09 forced exit is always last and is a fact, not a
//! choice. No component value is typed here: every value comes from the
//! direction's own past through [`seed_exit_set`] (its structural "none" plus
//! the median of its own-past distribution on the learning months `[0, W0)`
//! only, FM-F049), and later refinements append compositions through
//! [`ExitSet::extended`]. A set is versioned and append-only: a variant keeps
//! its id forever, so an outcome record names its variant by id.
//!
//! The entry fill window and the participation cap are deep-only components
//! (17:15 items 3 and 4) and live in their own generated [`FillSet`], so a
//! stage-A walk never repeats itself once per deep-only value.
//!
//! A larger set runs in more passes: [`ExitSet::passes`] cuts the ids into
//! chunks of the measured [`PassBudget`], each pass identical in cost; the
//! bytes a walk writes never depend on the budget (FM-F032). Proved by
//! `crates/runner/tests/fib_walk.rs`.

use std::collections::HashSet;

use brutex_core::blake3::{OUT_LEN, hash};
use vocab::fib_cube::{PERMILLE_PER_UNIT, SESSION_FORCED_WINDOW_START_SECS, Search, VariantSet};

/// The stop-loss component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StopLoss {
    /// No stop-loss order: time, target and forced exits only, with R still
    /// measured from the planned stop.
    Absent,
    /// A stop beyond the base: the planned stop (the base's far edge offset by
    /// exactly one tick) and `extra_ticks` more ticks of the walk's tick.
    BeyondBase {
        /// Further ticks beyond the planned stop, from the own-past far-edge
        /// excursions.
        extra_ticks: u32,
    },
}

/// The take-profit component. Every target is frozen at the decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TakeProfit {
    /// No target.
    Absent,
    /// `entry_ref + milli_r / 1000 x planned risk`, from the own-past run-ups.
    RMultiple {
        /// The multiple, in thousandths of R.
        milli_r: u32,
    },
    /// A Fibonacci rung of the day so far (`indicators::fib::RUNGS`), measured
    /// in the trade's direction from the plan's level 0.
    FibRung {
        /// The rung, in permille.
        rung_permille: u16,
    },
    /// The first rung of `indicators::fib::RUNGS` strictly beyond the entry
    /// reference.
    NextRung,
}

/// The trailing take-profit component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrailingTakeProfit {
    /// No trailing take-profit.
    Absent,
    /// Exit when the price gives back `permille` of the open profit
    /// (the best price since entry less the entry reference), from the
    /// own-past pullbacks.
    GiveBack {
        /// The share given back, in permille, in `1..=1000`.
        permille: u16,
    },
}

/// The trailing stop component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrailingStop {
    /// No trailing stop.
    Absent,
    /// Under each new confirmed swing of the plan's own candles: a low below
    /// the `width` candles on each side, confirmed at the close of the
    /// `width`-th candle after it, raises the stop to that low less one tick
    /// and less `buffer_paisa` (decision basis), never lowered.
    Swing {
        /// Candles on each side; one is the definition of a swing.
        width: u16,
        /// The own-past poke-through depth, in decision-basis paisa.
        buffer_paisa: u32,
    },
    /// Each full step of `milli_r / 1000 x planned risk` of best price beyond
    /// the entry reference raises the stop by one step.
    RStep {
        /// The step, in thousandths of R.
        milli_r: u32,
    },
}

/// The time-exit component. An instant at or after 15:09:00 collapses into
/// the forced exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TimeExit {
    /// No time exit.
    Absent,
    /// At the first real bar at or after this clock second (after IST
    /// midnight), from the own-past last-peak clock seconds.
    Clock {
        /// The instant, in seconds after IST midnight.
        second: u32,
    },
    /// This many seconds after the first fill, from the own-past time to best.
    Hold {
        /// The hold, in seconds.
        seconds: u32,
    },
}

/// One exit variant: a composition of the five optional components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExitVariant {
    /// The stop loss.
    pub stop: StopLoss,
    /// The take-profit.
    pub take_profit: TakeProfit,
    /// The trailing take-profit.
    pub trailing_take_profit: TrailingTakeProfit,
    /// The trailing stop.
    pub trailing_stop: TrailingStop,
    /// The time exit.
    pub time_exit: TimeExit,
}

impl ExitVariant {
    /// The reference walk (17:15 item 0): no stop-loss order, no target and no
    /// time exit, so only the forced exit closes it.
    pub const REFERENCE: Self = Self {
        stop: StopLoss::Absent,
        take_profit: TakeProfit::Absent,
        trailing_take_profit: TrailingTakeProfit::Absent,
        trailing_stop: TrailingStop::Absent,
        time_exit: TimeExit::Absent,
    };

    /// Refuses a value no own-past distribution can produce: a zero multiple,
    /// step, width or hold, a give-back outside `1..=1000` permille, or a rung
    /// that is not one of `indicators::fib::RUNGS`.
    ///
    /// # Errors
    ///
    /// [`ExitSetRefusal::InvalidValue`] naming the component.
    pub fn validate(&self) -> Result<(), ExitSetRefusal> {
        let bad = |component| Err(ExitSetRefusal::InvalidValue { component });
        if let TakeProfit::RMultiple { milli_r: 0 } = self.take_profit {
            return bad("take_profit_r_multiple");
        }
        if let TakeProfit::FibRung { rung_permille } = self.take_profit
            && !indicators::fib::RUNGS
                .iter()
                .any(|rung| i64::from(*rung) == i64::from(rung_permille))
        {
            return bad("take_profit_fib_rung");
        }
        if let TrailingTakeProfit::GiveBack { permille } = self.trailing_take_profit
            && (permille == 0 || u64::from(permille) > PERMILLE_PER_UNIT)
        {
            return bad("trailing_take_profit_give_back");
        }
        match self.trailing_stop {
            TrailingStop::Swing { width: 0, .. } => return bad("trailing_stop_swing_width"),
            TrailingStop::RStep { milli_r: 0 } => return bad("trailing_stop_r_step"),
            _ => {}
        }
        if let TimeExit::Hold { seconds: 0 } = self.time_exit {
            return bad("time_exit_hold");
        }
        Ok(())
    }

    /// The variant's canonical bytes: one code byte and the value of each
    /// component, little-endian, in component order.
    fn put(&self, out: &mut Vec<u8>) {
        let (code, value) = match self.stop {
            StopLoss::Absent => (StopCode::Absent, 0),
            StopLoss::BeyondBase { extra_ticks } => (StopCode::BeyondBase, u64::from(extra_ticks)),
        };
        put_component(out, code as u8, value, 0);
        let (code, value) = match self.take_profit {
            TakeProfit::Absent => (TargetCode::Absent, 0),
            TakeProfit::RMultiple { milli_r } => (TargetCode::RMultiple, u64::from(milli_r)),
            TakeProfit::FibRung { rung_permille } => {
                (TargetCode::FibRung, u64::from(rung_permille))
            }
            TakeProfit::NextRung => (TargetCode::NextRung, 0),
        };
        put_component(out, code as u8, value, 0);
        let (code, value) = match self.trailing_take_profit {
            TrailingTakeProfit::Absent => (GiveBackCode::Absent, 0),
            TrailingTakeProfit::GiveBack { permille } => {
                (GiveBackCode::GiveBack, u64::from(permille))
            }
        };
        put_component(out, code as u8, value, 0);
        let (code, value, second) = match self.trailing_stop {
            TrailingStop::Absent => (TrailCode::Absent, 0, 0),
            TrailingStop::Swing {
                width,
                buffer_paisa,
            } => (TrailCode::Swing, u64::from(width), u64::from(buffer_paisa)),
            TrailingStop::RStep { milli_r } => (TrailCode::RStep, u64::from(milli_r), 0),
        };
        put_component(out, code as u8, value, second);
        let (code, value) = match self.time_exit {
            TimeExit::Absent => (TimeCode::Absent, 0),
            TimeExit::Clock { second } => (TimeCode::Clock, u64::from(second)),
            TimeExit::Hold { seconds } => (TimeCode::Hold, u64::from(seconds)),
        };
        put_component(out, code as u8, value, 0);
    }
}

/// One component's bytes: its code, its value and its second value.
fn put_component(out: &mut Vec<u8>, code: u8, value: u64, second: u64) {
    out.push(code);
    out.extend_from_slice(&value.to_le_bytes());
    out.extend_from_slice(&second.to_le_bytes());
}

/// The stop component's byte codes, append-only.
#[derive(Clone, Copy)]
enum StopCode {
    Absent,
    BeyondBase,
}

/// The take-profit component's byte codes, append-only.
#[derive(Clone, Copy)]
enum TargetCode {
    Absent,
    RMultiple,
    FibRung,
    NextRung,
}

/// The trailing take-profit component's byte codes, append-only.
#[derive(Clone, Copy)]
enum GiveBackCode {
    Absent,
    GiveBack,
}

/// The trailing stop component's byte codes, append-only.
#[derive(Clone, Copy)]
enum TrailCode {
    Absent,
    Swing,
    RStep,
}

/// The time-exit component's byte codes, append-only.
#[derive(Clone, Copy)]
enum TimeCode {
    Absent,
    Clock,
    Hold,
}

/// The entry fill window (17:15 item 3), deep only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntryWindow {
    /// Reading 1: fill until the next close of the plan's timeframe.
    NextClose,
    /// Reading 2, the operator's FILLS rule with no window: fill until the
    /// order is filled, an exit fires, or 15:09:00.
    UntilForced,
    /// A deadline this many seconds after the decision, from the own-past
    /// seconds to full fill (R2); capped at 15:09:00.
    Deadline {
        /// The deadline, in seconds after the decision instant.
        seconds: u32,
    },
}

/// The participation cap (17:15 item 4), deep only, on entry fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Participation {
    /// The operator's rule: a second supplies its whole volume.
    NoCap,
    /// A second supplies `floor(permille x volume / 1000)` shares, from the
    /// own-past per-second participation.
    Cap {
        /// The cap, in permille of the second's volume, in `1..=1000`.
        permille: u16,
    },
}

/// One fill variant: the entry window and the participation cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FillVariant {
    /// The entry fill window.
    pub window: EntryWindow,
    /// The participation cap.
    pub participation: Participation,
}

impl FillVariant {
    /// Stage A's only fill: the next close of the plan's timeframe, the whole
    /// quantity at the minute's worst printed price (FM-F039).
    pub const STAGE_A: Self = Self {
        window: EntryWindow::NextClose,
        participation: Participation::NoCap,
    };

    /// The reference walk's fill (17:15 item 0): no window and no cap, so the
    /// participation it measures is the uncapped one.
    pub const REFERENCE: Self = Self {
        window: EntryWindow::UntilForced,
        participation: Participation::NoCap,
    };

    /// Refuses a zero deadline or a cap outside `1..=1000` permille.
    ///
    /// # Errors
    ///
    /// [`ExitSetRefusal::InvalidValue`] naming the component.
    pub fn validate(&self) -> Result<(), ExitSetRefusal> {
        if let EntryWindow::Deadline { seconds: 0 } = self.window {
            return Err(ExitSetRefusal::InvalidValue {
                component: "entry_window_deadline",
            });
        }
        if let Participation::Cap { permille } = self.participation
            && (permille == 0 || u64::from(permille) > PERMILLE_PER_UNIT)
        {
            return Err(ExitSetRefusal::InvalidValue {
                component: "participation_cap",
            });
        }
        Ok(())
    }

    /// The variant's canonical bytes.
    fn put(&self, out: &mut Vec<u8>) {
        let (code, value) = match self.window {
            EntryWindow::NextClose => (WindowCode::NextClose, 0),
            EntryWindow::UntilForced => (WindowCode::UntilForced, 0),
            EntryWindow::Deadline { seconds } => (WindowCode::Deadline, u64::from(seconds)),
        };
        put_component(out, code as u8, value, 0);
        let (code, value) = match self.participation {
            Participation::NoCap => (CapCode::NoCap, 0),
            Participation::Cap { permille } => (CapCode::Cap, u64::from(permille)),
        };
        put_component(out, code as u8, value, 0);
    }
}

/// The entry window's byte codes, append-only.
#[derive(Clone, Copy)]
enum WindowCode {
    NextClose,
    UntilForced,
    Deadline,
}

/// The participation cap's byte codes, append-only.
#[derive(Clone, Copy)]
enum CapCode {
    NoCap,
    Cap,
}

/// Why a set was refused, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitSetRefusal {
    /// A component value no own-past distribution can produce.
    InvalidValue {
        /// The component.
        component: &'static str,
    },
    /// The same composition twice in one set.
    DuplicateVariant {
        /// The second occurrence's id.
        id: usize,
    },
    /// More variants than the exit row's `u16` radix holds.
    TooManyVariants {
        /// How many were offered.
        count: usize,
    },
    /// A set with no variant at all.
    Empty,
    /// A sample list that is not sorted by value (the seed reads medians by
    /// rank).
    SamplesNotSorted {
        /// The distribution.
        distribution: &'static str,
    },
    /// A version past `u32::MAX`.
    VersionExhausted,
}

impl ExitSetRefusal {
    /// The refusal's canonical name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::InvalidValue { .. } => "exit_value_invalid",
            Self::DuplicateVariant { .. } => "exit_variant_duplicate",
            Self::TooManyVariants { .. } => "exit_variants_exceed_u16",
            Self::Empty => "exit_set_empty",
            Self::SamplesNotSorted { .. } => "seed_samples_not_sorted",
            Self::VersionExhausted => "exit_set_version_exhausted",
        }
    }
}

/// How many variants one walker pass walks: the measured bench budget that
/// keeps one bar's work O(1) at p99 (13:05 item 4; rows C-F5-01 to C-F5-03 of
/// `crates/runner/benches/fib.rs`). Recorded in the run ledger, never in the
/// identity: the bytes do not depend on it (FM-F032).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PassBudget(u16);

impl PassBudget {
    /// A budget of `variants` per pass, or `None` for zero.
    #[must_use]
    pub const fn new(variants: u16) -> Option<Self> {
        if variants == 0 {
            None
        } else {
            Some(Self(variants))
        }
    }

    /// Variants per pass.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// The run's generated, versioned exit set for one direction (16:04 IST: no
/// value is shared across directions).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExitSet {
    /// The direction whose own past generated it.
    direction: Search,
    /// The set's version: one more than the version it extends.
    version: u32,
    /// The walk-forward step this version was born at (R1).
    birth_step: u32,
    /// The variants, by id.
    variants: Vec<ExitVariant>,
    /// Every distinct swing width the set uses, ascending.
    widths: Vec<u16>,
}

impl ExitSet {
    /// A set of `variants`, ids in the order given.
    ///
    /// # Errors
    ///
    /// [`ExitSetRefusal::Empty`], [`ExitSetRefusal::TooManyVariants`],
    /// [`ExitSetRefusal::DuplicateVariant`] or a variant's
    /// [`ExitSetRefusal::InvalidValue`].
    pub fn new(
        direction: Search,
        version: u32,
        birth_step: u32,
        variants: Vec<ExitVariant>,
    ) -> Result<Self, ExitSetRefusal> {
        if variants.is_empty() {
            return Err(ExitSetRefusal::Empty);
        }
        if u16::try_from(variants.len()).is_err() {
            return Err(ExitSetRefusal::TooManyVariants {
                count: variants.len(),
            });
        }
        let mut seen = HashSet::with_capacity(variants.len());
        let mut widths: Vec<u16> = Vec::new();
        for (id, variant) in variants.iter().enumerate() {
            variant.validate()?;
            if !seen.insert(*variant) {
                return Err(ExitSetRefusal::DuplicateVariant { id });
            }
            if let TrailingStop::Swing { width, .. } = variant.trailing_stop {
                insert_sorted(&mut widths, width);
            }
        }
        Ok(Self {
            direction,
            version,
            birth_step,
            variants,
            widths,
        })
    }

    /// This set with `more` appended as the next version, born at
    /// `birth_step`: every existing variant keeps its id (13:05 item 5, a new
    /// variant is computed only for the trades that lack it).
    ///
    /// # Errors
    ///
    /// Every refusal of [`Self::new`], and
    /// [`ExitSetRefusal::VersionExhausted`].
    pub fn extended(&self, birth_step: u32, more: &[ExitVariant]) -> Result<Self, ExitSetRefusal> {
        let version = self
            .version
            .checked_add(1)
            .ok_or(ExitSetRefusal::VersionExhausted)?;
        let mut variants = self.variants.clone();
        variants.extend_from_slice(more);
        Self::new(self.direction, version, birth_step, variants)
    }

    /// The direction.
    #[must_use]
    pub const fn direction(&self) -> Search {
        self.direction
    }

    /// The version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// The variants, by id.
    #[must_use]
    pub fn variants(&self) -> &[ExitVariant] {
        &self.variants
    }

    /// The variant with id `id`.
    #[must_use]
    pub fn get(&self, id: u16) -> Option<&ExitVariant> {
        self.variants.get(usize::from(id))
    }

    /// How many variants.
    #[must_use]
    pub fn count(&self) -> u16 {
        u16::try_from(self.variants.len()).unwrap_or(u16::MAX)
    }

    /// Every distinct swing width, ascending.
    #[must_use]
    pub fn widths(&self) -> &[u16] {
        &self.widths
    }

    /// The canonical bytes: tag, direction, version, birth step, count and
    /// every variant in id order.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"BTXFIBX1");
        out.push(self.direction as u8);
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&self.birth_step.to_le_bytes());
        out.extend_from_slice(&self.count().to_le_bytes());
        for variant in &self.variants {
            variant.put(&mut out);
        }
        out
    }

    /// The BLAKE3 of [`Self::bytes`].
    #[must_use]
    pub fn digest(&self) -> [u8; OUT_LEN] {
        hash(&self.bytes())
    }

    /// The set as the cube's exit row reads it (17:15 item f, FM-F006): its
    /// version, digest and count.
    #[must_use]
    pub fn variant_set(&self) -> VariantSet {
        VariantSet {
            version: self.version,
            digest: self.digest(),
            count: self.count(),
        }
    }

    /// The ids, cut into passes of `budget` variants, in id order: a larger
    /// set runs in `ceil(variants / budget)` passes, each identical in cost.
    #[must_use]
    pub fn passes(&self, budget: PassBudget) -> Vec<core::ops::Range<u16>> {
        let count = self.count();
        let mut out = Vec::with_capacity(usize::from(count.div_ceil(budget.get())));
        let mut start = 0;
        while start < count {
            let end = start.saturating_add(budget.get()).min(count);
            out.push(start..end);
            start = end;
        }
        out
    }
}

/// The run's generated, versioned fill set for one direction, deep only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FillSet {
    /// The direction whose own past generated it.
    direction: Search,
    /// The set's version.
    version: u32,
    /// The variants, by id.
    variants: Vec<FillVariant>,
}

impl FillSet {
    /// A set of `variants`, ids in the order given.
    ///
    /// # Errors
    ///
    /// [`ExitSetRefusal::Empty`], [`ExitSetRefusal::TooManyVariants`],
    /// [`ExitSetRefusal::DuplicateVariant`] or
    /// [`ExitSetRefusal::InvalidValue`].
    pub fn new(
        direction: Search,
        version: u32,
        variants: Vec<FillVariant>,
    ) -> Result<Self, ExitSetRefusal> {
        if variants.is_empty() {
            return Err(ExitSetRefusal::Empty);
        }
        if u16::try_from(variants.len()).is_err() {
            return Err(ExitSetRefusal::TooManyVariants {
                count: variants.len(),
            });
        }
        let mut seen = HashSet::with_capacity(variants.len());
        for (id, variant) in variants.iter().enumerate() {
            variant.validate()?;
            if !seen.insert(*variant) {
                return Err(ExitSetRefusal::DuplicateVariant { id });
            }
        }
        Ok(Self {
            direction,
            version,
            variants,
        })
    }

    /// The variants, by id.
    #[must_use]
    pub fn variants(&self) -> &[FillVariant] {
        &self.variants
    }

    /// The canonical bytes.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"BTXFIBF1");
        out.push(self.direction as u8);
        out.extend_from_slice(&self.version.to_le_bytes());
        let count = u16::try_from(self.variants.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&count.to_le_bytes());
        for variant in &self.variants {
            variant.put(&mut out);
        }
        out
    }
}

/// `value` into the ascending, duplicate-free `list`.
fn insert_sorted(list: &mut Vec<u16>, value: u16) {
    let mut at = list.len();
    for (index, existing) in list.iter().enumerate() {
        if *existing == value {
            return;
        }
        if *existing > value {
            at = index;
            break;
        }
    }
    list.insert(at, value);
}

/// One own-past observation: its learning month and its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    /// The month's index on the run's span (month 0 is the first).
    pub month: u32,
    /// The measured value.
    pub value: u32,
}

/// The own-past distributions the seed reads, each sorted by value, measured
/// on one direction's reference walks (17:15 item 0) and FIB-2's measures. A
/// month at or after `W0` is never read (FM-F049).
#[derive(Clone, Copy, Debug)]
pub struct SeedSamples<'a> {
    /// Ticks the price went beyond the base's far edge (stop offsets k).
    pub far_edge_excursion_ticks: &'a [Sample],
    /// Run-ups of the reference walks, in thousandths of R (take-profit).
    pub run_up_milli_r: &'a [Sample],
    /// Open-profit pullbacks, in permille of the open profit (trailing
    /// take-profit).
    pub giveback_permille: &'a [Sample],
    /// Pullback lengths, in the plan's candles (swing widths).
    pub pullback_candles: &'a [Sample],
    /// Poke-through depths below a swing low, in decision-basis paisa (the
    /// swing buffer).
    pub poke_through_paisa: &'a [Sample],
    /// Swing sizes, in thousandths of R (the R-step).
    pub swing_size_milli_r: &'a [Sample],
    /// Last-peak clock seconds of reference trades still open at the forced
    /// window (clock exits).
    pub last_peak_clock_seconds: &'a [Sample],
    /// Time to the best price, in seconds after the first fill (hold exits).
    pub time_to_best_seconds: &'a [Sample],
}

/// The median of `samples` on the months before `w0`, or `None` when none is
/// there.
///
/// # Errors
///
/// [`ExitSetRefusal::SamplesNotSorted`] for a list not sorted by value.
fn learning_median(
    distribution: &'static str,
    samples: &[Sample],
    w0: u32,
) -> Result<Option<u32>, ExitSetRefusal> {
    let mut previous: Option<u32> = None;
    for sample in samples {
        if previous.is_some_and(|value| value > sample.value) {
            return Err(ExitSetRefusal::SamplesNotSorted { distribution });
        }
        previous = Some(sample.value);
    }
    let learning: Vec<u32> = samples
        .iter()
        .filter(|sample| sample.month < w0)
        .map(|sample| sample.value)
        .collect();
    Ok(lower_median(&learning))
}

/// The lower median of a sorted sample: the value at 1-based rank
/// `ceil(n / 2)`, R2's seed rule (`vocab::fib_cube::lower_nearest_rank` at one
/// half, which `seed_rule_is_the_lower_nearest_rank_median` pins), written
/// with a shift so no quantile is typed.
fn lower_median(sorted: &[u32]) -> Option<u32> {
    let rank = sorted.len().checked_add(1)? >> 1;
    sorted.get(rank.checked_sub(1)?).copied()
}

/// The coarse start of the exit set (17:15 item 2): every component at its
/// structural "none" plus one value at the median of its own-past
/// distribution on `[0, w0)`, and the swing of width one, which defines a
/// swing. The variants are every composition of those values, in component
/// order. A distribution with no learning sample, or whose median is not a
/// valid value, contributes "none" only.
///
/// # Errors
///
/// [`ExitSetRefusal::SamplesNotSorted`] and every refusal of [`ExitSet::new`].
pub fn seed_exit_set(
    direction: Search,
    birth_step: u32,
    w0: u32,
    samples: &SeedSamples<'_>,
) -> Result<ExitSet, ExitSetRefusal> {
    let mut stops = vec![StopLoss::Absent, StopLoss::BeyondBase { extra_ticks: 0 }];
    if let Some(k) = learning_median(
        "far_edge_excursion_ticks",
        samples.far_edge_excursion_ticks,
        w0,
    )? && k > 0
    {
        stops.push(StopLoss::BeyondBase { extra_ticks: k });
    }
    let mut targets = vec![TakeProfit::Absent];
    if let Some(m) = learning_median("run_up_milli_r", samples.run_up_milli_r, w0)? {
        push_valid_target(&mut targets, TakeProfit::RMultiple { milli_r: m });
    }
    let mut givebacks = vec![TrailingTakeProfit::Absent];
    if let Some(g) = learning_median("giveback_permille", samples.giveback_permille, w0)?
        && let Ok(permille) = u16::try_from(g)
    {
        let value = TrailingTakeProfit::GiveBack { permille };
        if probe(TrailingStop::Absent, value).is_ok() {
            givebacks.push(value);
        }
    }
    let mut trails = vec![
        TrailingStop::Absent,
        TrailingStop::Swing {
            width: 1,
            buffer_paisa: 0,
        },
    ];
    if let Some(w) = learning_median("pullback_candles", samples.pullback_candles, w0)?
        && let Ok(width) = u16::try_from(w)
        && width > 1
    {
        trails.push(TrailingStop::Swing {
            width,
            buffer_paisa: 0,
        });
    }
    if let Some(b) = learning_median("poke_through_paisa", samples.poke_through_paisa, w0)?
        && b > 0
    {
        trails.push(TrailingStop::Swing {
            width: 1,
            buffer_paisa: b,
        });
    }
    if let Some(s) = learning_median("swing_size_milli_r", samples.swing_size_milli_r, w0)?
        && s > 0
    {
        trails.push(TrailingStop::RStep { milli_r: s });
    }
    let mut times = vec![TimeExit::Absent];
    if let Some(c) = learning_median(
        "last_peak_clock_seconds",
        samples.last_peak_clock_seconds,
        w0,
    )? && u64::from(c) < SESSION_FORCED_WINDOW_START_SECS
    {
        times.push(TimeExit::Clock { second: c });
    }
    if let Some(h) = learning_median("time_to_best_seconds", samples.time_to_best_seconds, w0)?
        && h > 0
    {
        times.push(TimeExit::Hold { seconds: h });
    }
    let mut variants = Vec::with_capacity(
        stops.len() * targets.len() * givebacks.len() * trails.len() * times.len(),
    );
    for stop in &stops {
        for take_profit in &targets {
            for trailing_take_profit in &givebacks {
                for trailing_stop in &trails {
                    for time_exit in &times {
                        variants.push(ExitVariant {
                            stop: *stop,
                            take_profit: *take_profit,
                            trailing_take_profit: *trailing_take_profit,
                            trailing_stop: *trailing_stop,
                            time_exit: *time_exit,
                        });
                    }
                }
            }
        }
    }
    ExitSet::new(direction, 1, birth_step, variants)
}

/// Pushes `target` when it is a valid value.
fn push_valid_target(targets: &mut Vec<TakeProfit>, target: TakeProfit) {
    let variant = ExitVariant {
        take_profit: target,
        ..ExitVariant::REFERENCE
    };
    if variant.validate().is_ok() {
        targets.push(target);
    }
}

/// Validates one trailing take-profit value beside a trailing stop.
fn probe(trailing_stop: TrailingStop, value: TrailingTakeProfit) -> Result<(), ExitSetRefusal> {
    ExitVariant {
        trailing_take_profit: value,
        trailing_stop,
        ..ExitVariant::REFERENCE
    }
    .validate()
}

/// The coarse start of the deep fill set (17:15 items 3 and 4): both entry
/// window readings, and no cap plus a cap at the median of the own-past
/// per-second participation on `[0, w0)`. Deadlines arrive by R2 refinement.
///
/// # Errors
///
/// [`ExitSetRefusal::SamplesNotSorted`] and every refusal of [`FillSet::new`].
pub fn seed_fill_set(
    direction: Search,
    w0: u32,
    participation_permille: &[Sample],
) -> Result<FillSet, ExitSetRefusal> {
    let mut caps = vec![Participation::NoCap];
    if let Some(c) = learning_median("participation_permille", participation_permille, w0)?
        && let Ok(permille) = u16::try_from(c)
    {
        let cap = Participation::Cap { permille };
        let probe = FillVariant {
            window: EntryWindow::NextClose,
            participation: cap,
        };
        if probe.validate().is_ok() {
            caps.push(cap);
        }
    }
    let mut variants = Vec::new();
    for window in [EntryWindow::NextClose, EntryWindow::UntilForced] {
        for participation in &caps {
            variants.push(FillVariant {
                window,
                participation: *participation,
            });
        }
    }
    FillSet::new(direction, 1, variants)
}
