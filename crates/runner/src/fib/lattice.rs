//! The BUC iceberg lattice of one partition: one-direction mixes grown by
//! trade count to extinction (FIB-6; D-6302, D-6400 to D-6410).
//!
//! # What a row is, and what a partition is
//!
//! A row is one DECISION of one partition: its full cell (a
//! [`vocab::fib_cube::Coords`]), its stock slot, its IST day, its decision
//! second, its decision id and one outcome index per exit variant. A partition
//! is one (direction, entry style, timeframe): the two partition rows besides
//! the exit row are fixed for the whole table, and the exit row is never
//! traversed at all. Every node scores every exit variant of the run as its own
//! mix ([`Node::mix`]), so the traversal runs once per (direction, entry style,
//! timeframe) and not once per exit variant (FM-F054, D-6405): the raw count of
//! a decision is the same under every exit, so the per-exit lattices would be
//! the same lattice V times.
//!
//! # The rows are in canonical time order, and every node keeps them so
//!
//! Canonical order is (day, decision second, stock slot, decision id). A
//! month's decisions arrive in (slot, id) order and are put in canonical order
//! by two stable counting sorts over the month's own (day, second) domains, so
//! a new month is appended and never re-sorts the old ones (FM-F051, D-6404).
//! The table's row index IS the canonical position.
//!
//! A node's rows are a list of row indices in ascending order. A categorical
//! row's children come from ONE stable counting sort of the node's rows into a
//! scratch region, O(rows in node + radix), so every child is ascending too. The
//! cumulative row (`mover_rank`) has PREFIX children (`group <= g`), and each is
//! built by one stable linear scan of the node's rows into its own scratch
//! region (FM-F055, D-6406): a counting sort would put a wider prefix's rows out
//! of time order, and the one-position-per-stock pass ([`super::score`]) needs
//! time order.
//!
//! # Growth by trade count only, to extinction
//!
//! A child is visited iff its RAW trade count reaches `min_support`. Support is
//! anti-monotone, so every branch dies out by itself; the caller derives the
//! floor from the Winner Gate (`MinSupport`, R5) and it is never a flag. The
//! lattice never reads a gate verdict: win rate is not monotone, so a child of
//! a failing parent is still grown (operator 16 Oct 11:10 IST). The number of
//! rows a mix fixes is bounded by the schema's own free rows — 21 of the 24 —
//! a structural bound and not a parameter; no type here carries one
//! (`CLAUDE.md` §6). The [`Tally`] counts every node by the FIXED ROWS IN THE
//! MIX.
//!
//! # Equivalent children are skipped, counted and never scored
//!
//! A child whose raw count equals its parent's holds the parent's trade set:
//! every mix under it is reached from the parent with the same rows, so it is
//! skipped as `equivalent_to_parent` (D-6401), the closed-itemset reasoning of
//! `crate::closed`. A prefix child whose count equals the prefix before it is
//! skipped as `equivalent_to_sibling` (FM-F103, D-6407). Neither is a trial.
//!
//! # Guided mode
//!
//! [`walk_guided`] descends only along a trie of survivor keys, with the
//! equivalence skip and the floor switched off on those paths, so a lone
//! survivor equal to its parent is still scored (FM-F052, D-6408); [`reconcile`]
//! fails when a survivor has no verdict.
//!
//! # Cost
//!
//! Per visited node of `n` rows: one counting sort per free row after the
//! node's last fixed one (O(n + radix) each) and one O(n) scan per prefix child.
//! So the walk is O(sum of visited supports) — O(1) per row-touch — and not O(1)
//! per run; the [`Tally`] counts the row-touches. Memory per worker: the table,
//! the identity order and one scratch arena of 21 times the row count, reserved
//! once and zeroed lazily by the allocator (the path from the root to any node
//! holds at most 21 regions, each no larger than the root's rows). Proved in
//! `crates/runner/tests/fib_lattice.rs` (the row-touch tally against a naive
//! walk) and measured by `crates/runner/benches/fib.rs` (`C-F6-01`).
//!
//! Pure: no I/O, and its loops call nothing but the visitor (gates 17 and 22).

use core::ops::Range;
use std::collections::{HashMap, HashSet};

use rayon::prelude::*;
use vocab::fib_cube::{
    Coords, CubeRefusal, Layout, MixKey, OPEN, ROW_COUNT, RowId, RowKind, SECONDS_PER_DAY, Search,
};

/// One decision of one partition, as its producer hands it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    /// Its full cell. The exit row must hold the table's first exit variant:
    /// every variant is scored at every node, so the cell names the first.
    pub coords: Coords,
    /// The stock's slot in the run's pool.
    pub slot: u32,
    /// The IST civil day number.
    pub day: u32,
    /// The decision second after IST midnight.
    pub second: u32,
    /// The decision id.
    pub id: u64,
}

/// The partition one table holds: a direction, an entry style, a timeframe and
/// the exit variants each node scores (FM-F054).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Partition {
    /// Long or short: every mix is one direction (operator 16 Oct 16:04 IST).
    pub search: Search,
    /// The entry-style value of every row.
    pub entry_style: u8,
    /// The timeframe value of every row.
    pub timeframe: u8,
    /// The exit variants scored at every node, as exit-row value indices.
    pub exits: Range<u8>,
}

/// Why the lattice refused. Every refusal is named; none is silent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LatticeRefusal {
    /// A floor of zero would visit empty mixes.
    MinSupportZero,
    /// A partition value is outside its row's generated values.
    PartitionValueOutOfRange {
        /// The row.
        row: &'static str,
        /// The value given.
        value: u8,
        /// The row's value count.
        values: u8,
    },
    /// The partition names no exit variant, or one past the exit set.
    ExitsInvalid {
        /// The first variant.
        start: u8,
        /// One past the last variant.
        end: u8,
        /// The exit set's size.
        values: u8,
    },
    /// The run has no month.
    MonthsZero,
    /// A month ordinal past the run's months.
    MonthOutOfRange {
        /// The month given.
        month: u32,
        /// The run's month count.
        months: u32,
    },
    /// A month pushed at or before the last one.
    MonthNotAfterLast {
        /// The month given.
        month: u32,
        /// The last month pushed.
        last: u32,
    },
    /// A month's first day is not after the previous month's last day.
    MonthDaysOverlap {
        /// The month given.
        month: u32,
        /// Its first day.
        first_day: u32,
        /// The previous month's last day.
        previous_last_day: u32,
    },
    /// The outcome list does not hold one index per decision per variant.
    OutcomesLength {
        /// Decisions given.
        decisions: usize,
        /// Exit variants per decision.
        variants: usize,
        /// Outcome indices given.
        outcomes: usize,
    },
    /// A cell the layout refuses.
    Cell(CubeRefusal),
    /// A mix the layout refuses.
    Mix(CubeRefusal),
    /// A cell or mix whose side belongs to the other direction.
    SideOutsideSearch {
        /// The table's search.
        search: &'static str,
        /// The side value given.
        side: u8,
    },
    /// A cell's partition row differs from the table's.
    PartitionMismatch {
        /// The row.
        row: &'static str,
        /// The table's value.
        want: u8,
        /// The cell's value.
        have: u8,
    },
    /// A decision second at or past the day's last second.
    SecondOutOfDay {
        /// The second given.
        second: u32,
    },
    /// A month's decisions are not in strictly increasing (slot, id) order.
    InputNotInSlotOrder {
        /// The position of the first decision out of order.
        at: usize,
    },
    /// A table past what a `u32` row index can name.
    RowsExceedU32,
    /// A survivor's exit variant is not one this table scores.
    SurvivorExitOutside {
        /// The variant given.
        variant: u8,
    },
    /// A survivor key given twice.
    SurvivorRepeated {
        /// The key.
        key: MixKey,
    },
    /// A survivor with no verdict (FM-F052: `fib verify` FAILS).
    SurvivorWithoutVerdict {
        /// The key.
        key: MixKey,
    },
    /// A verdict for a key that is not a survivor.
    VerdictWithoutSurvivor {
        /// The key.
        key: MixKey,
    },
    /// Two verdicts for one survivor.
    VerdictRepeated {
        /// The key.
        key: MixKey,
    },
    /// A range of rows past the table, or backwards.
    RangeOutsideTable {
        /// Its first row.
        start: u32,
        /// One past its last row.
        end: u32,
        /// Rows the table holds.
        rows: usize,
    },
    /// Scratch smaller than the range it is asked to expand.
    ScratchTooSmall {
        /// Rows the range holds.
        needed: usize,
        /// Rows the scratch holds.
        have: usize,
    },
}

impl LatticeRefusal {
    /// The refusal's stable name, for a counter or a report.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::MinSupportZero => "min_support_zero",
            Self::PartitionValueOutOfRange { .. } => "partition_value_out_of_range",
            Self::ExitsInvalid { .. } => "exits_invalid",
            Self::MonthsZero => "months_zero",
            Self::MonthOutOfRange { .. } => "month_out_of_range",
            Self::MonthNotAfterLast { .. } => "month_not_after_last",
            Self::MonthDaysOverlap { .. } => "month_days_overlap",
            Self::OutcomesLength { .. } => "outcomes_length",
            Self::Cell(_) => "cell",
            Self::Mix(_) => "mix",
            Self::SideOutsideSearch { .. } => "side_outside_search",
            Self::PartitionMismatch { .. } => "partition_mismatch",
            Self::SecondOutOfDay { .. } => "second_out_of_day",
            Self::InputNotInSlotOrder { .. } => "input_not_in_slot_order",
            Self::RowsExceedU32 => "rows_exceed_u32",
            Self::SurvivorExitOutside { .. } => "survivor_exit_outside",
            Self::SurvivorRepeated { .. } => "survivor_repeated",
            Self::SurvivorWithoutVerdict { .. } => "survivor_without_verdict",
            Self::VerdictWithoutSurvivor { .. } => "verdict_without_survivor",
            Self::VerdictRepeated { .. } => "verdict_repeated",
            Self::RangeOutsideTable { .. } => "range_outside_table",
            Self::ScratchTooSmall { .. } => "scratch_too_small",
        }
    }
}

impl core::fmt::Display for LatticeRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "fib lattice refused ({}): {self:?}", self.name())
    }
}

impl std::error::Error for LatticeRefusal {}

/// The value of row `row` in `coords`.
fn value_at(coords: &Coords, row: RowId) -> u8 {
    coords.get(row.index()).copied().unwrap_or(OPEN)
}

/// `key` with row `row` set to `value`.
fn with_row(key: MixKey, row: usize, value: u8) -> MixKey {
    let mut out = key;
    if let Some(slot) = out.0.get_mut(row) {
        *slot = value;
    }
    out
}

/// The schema rows a mix may leave free or fix: every row but the partition
/// rows, in row order.
fn free_rows() -> [RowId; FREE_ROWS] {
    let mut out = [RowId::Side; FREE_ROWS];
    let free = RowId::ALL
        .into_iter()
        .filter(|id| id.row().kind != RowKind::Partition);
    for (slot, id) in out.iter_mut().zip(free) {
        *slot = id;
    }
    out
}

/// How many schema rows a mix may leave free or fix: the structural bound on
/// the fixed rows of any mix, counted from the schema at compile time.
pub const FREE_ROWS: usize = free_count();

/// Counts the rows that are not partition rows.
const fn free_count() -> usize {
    let mut rest: &[RowId] = &RowId::ALL;
    let mut count = 0;
    while let [first, tail @ ..] = rest {
        if !matches!(first.row().kind, RowKind::Partition) {
            count += 1;
        }
        rest = tail;
    }
    count
}

/// Refuses a table of more rows than a `u32` row index can name.
fn indexable(rows: usize) -> Result<(), LatticeRefusal> {
    u32::try_from(rows)
        .map(|_| ())
        .map_err(|_| LatticeRefusal::RowsExceedU32)
}

/// Writes `value` at the cursor `start` of `out` and moves the cursor on.
fn place(out: &mut [u32], start: &mut usize, value: u32) {
    if let Some(slot) = out.get_mut(*start) {
        *slot = value;
    }
    *start += 1;
}

/// One stable counting sort of `order` by `key`, whose values are all below
/// `domain`: O(order + domain).
fn stable_by(order: &[u32], domain: usize, key: impl Fn(u32) -> usize) -> Vec<u32> {
    let mut starts = vec![0_usize; domain];
    for at in order {
        if let Some(count) = starts.get_mut(key(*at)) {
            *count += 1;
        }
    }
    let mut run = 0;
    for start in &mut starts {
        let width = *start;
        *start = run;
        run += width;
    }
    let mut out = vec![0_u32; order.len()];
    for at in order {
        if let Some(start) = starts.get_mut(key(*at)) {
            place(&mut out, start, *at);
        }
    }
    out
}

/// The rows of one partition, in canonical order, appended a month at a time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowTable {
    /// The generation's layout every cell is checked against.
    layout: Layout,
    /// The partition.
    partition: Partition,
    /// The run's month count: the length of every month vector.
    months: u32,
    /// The last month pushed and its last day.
    last: Option<(u32, u32)>,
    /// Each row's cell.
    coords: Vec<Coords>,
    /// Each row's stock slot.
    slot: Vec<u32>,
    /// Each row's IST day.
    day: Vec<u32>,
    /// Each row's decision second.
    second: Vec<u32>,
    /// Each row's decision id.
    id: Vec<u64>,
    /// Each row's month ordinal.
    month: Vec<u32>,
    /// Each row's outcome index per exit variant, row-major.
    outcomes: Vec<u32>,
    /// The identity order 0, 1, 2, ...: the root's rows, kept so no walk
    /// allocates them.
    order: Vec<u32>,
    /// One past the largest stock slot seen: the pool the scorer sizes for.
    pool: u32,
}

/// One row of a [`RowTable`], read whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowView<'a> {
    /// Its cell.
    pub coords: &'a Coords,
    /// Its stock slot.
    pub slot: u32,
    /// Its IST day.
    pub day: u32,
    /// Its decision second.
    pub second: u32,
    /// Its decision id.
    pub id: u64,
    /// Its month ordinal.
    pub month: u32,
    /// Its outcome index per exit variant.
    pub outcomes: &'a [u32],
}

impl RowTable {
    /// An empty table of one partition, for a run of `months` months.
    ///
    /// # Errors
    ///
    /// [`LatticeRefusal::MonthsZero`],
    /// [`LatticeRefusal::PartitionValueOutOfRange`] and
    /// [`LatticeRefusal::ExitsInvalid`].
    pub fn new(layout: Layout, partition: Partition, months: u32) -> Result<Self, LatticeRefusal> {
        if months == 0 {
            return Err(LatticeRefusal::MonthsZero);
        }
        for (id, value) in [
            (RowId::EntryStyle, partition.entry_style),
            (RowId::Timeframe, partition.timeframe),
        ] {
            let values = layout.radix(id);
            if value >= values {
                return Err(LatticeRefusal::PartitionValueOutOfRange {
                    row: id.row().name,
                    value,
                    values,
                });
            }
        }
        let exits = layout.radix(RowId::Exit);
        if partition.exits.is_empty() || partition.exits.end > exits {
            return Err(LatticeRefusal::ExitsInvalid {
                start: partition.exits.start,
                end: partition.exits.end,
                values: exits,
            });
        }
        Ok(Self {
            layout,
            partition,
            months,
            last: None,
            coords: Vec::new(),
            slot: Vec::new(),
            day: Vec::new(),
            second: Vec::new(),
            id: Vec::new(),
            month: Vec::new(),
            outcomes: Vec::new(),
            order: Vec::new(),
            pool: 0,
        })
    }

    /// Exit variants per row.
    #[must_use]
    pub fn variants(&self) -> usize {
        self.partition.exits.len()
    }

    /// Checks one decision against the layout, the search and the partition.
    fn check(&self, decision: &Decision) -> Result<(), LatticeRefusal> {
        self.layout
            .validate(&decision.coords)
            .map_err(LatticeRefusal::Cell)?;
        if !self.partition.search.admits(&decision.coords) {
            return Err(LatticeRefusal::SideOutsideSearch {
                search: self.partition.search.name(),
                side: value_at(&decision.coords, RowId::Side),
            });
        }
        for (id, want) in [
            (RowId::EntryStyle, self.partition.entry_style),
            (RowId::Timeframe, self.partition.timeframe),
            (RowId::Exit, self.partition.exits.start),
        ] {
            let have = value_at(&decision.coords, id);
            if have != want {
                return Err(LatticeRefusal::PartitionMismatch {
                    row: id.row().name,
                    want,
                    have,
                });
            }
        }
        if u64::from(decision.second) >= SECONDS_PER_DAY {
            return Err(LatticeRefusal::SecondOutOfDay {
                second: decision.second,
            });
        }
        Ok(())
    }

    /// Appends one month's decisions, given in strictly increasing (slot, id)
    /// order, with one outcome index per decision per exit variant (row-major).
    /// Two stable counting sorts over the month's own second and day domains put
    /// them in canonical order: O(decisions + domains), and no earlier month is
    /// touched (FM-F051).
    ///
    /// # Errors
    ///
    /// By name: the month's range and order, its days against the previous
    /// month's, the outcome count, every decision's cell, side, partition rows
    /// and second, the (slot, id) order, and a table past `u32` rows. A refused
    /// month leaves the table as it was.
    pub fn push_month(
        &mut self,
        month: u32,
        decisions: &[Decision],
        outcomes: &[u32],
    ) -> Result<(), LatticeRefusal> {
        if month >= self.months {
            return Err(LatticeRefusal::MonthOutOfRange {
                month,
                months: self.months,
            });
        }
        if let Some((last, _)) = self.last
            && month <= last
        {
            return Err(LatticeRefusal::MonthNotAfterLast { month, last });
        }
        let variants = self.variants();
        if decisions.len().checked_mul(variants) != Some(outcomes.len()) {
            return Err(LatticeRefusal::OutcomesLength {
                decisions: decisions.len(),
                variants,
                outcomes: outcomes.len(),
            });
        }
        indexable(self.coords.len().saturating_add(decisions.len()))?;
        let mut previous: Option<(u32, u64)> = None;
        for (at, decision) in decisions.iter().enumerate() {
            self.check(decision)?;
            let here = (decision.slot, decision.id);
            if previous.is_some_and(|before| before >= here) {
                return Err(LatticeRefusal::InputNotInSlotOrder { at });
            }
            previous = Some(here);
        }
        let first_day = decisions.iter().map(|d| d.day).min();
        let last_day = decisions.iter().map(|d| d.day).max();
        if let (Some(first_day), Some((_, previous_last_day))) = (first_day, self.last)
            && first_day <= previous_last_day
        {
            return Err(LatticeRefusal::MonthDaysOverlap {
                month,
                first_day,
                previous_last_day,
            });
        }
        let order = Self::canonical(decisions);
        let placed = order.iter().filter_map(|at| {
            decisions
                .get(*at as usize)
                .map(|decision| (*at as usize, decision))
        });
        for (at, decision) in placed {
            self.coords.push(decision.coords);
            self.slot.push(decision.slot);
            self.day.push(decision.day);
            self.second.push(decision.second);
            self.id.push(decision.id);
            self.month.push(month);
            let next = u32::try_from(self.order.len()).unwrap_or(u32::MAX);
            self.order.push(next);
            let from = at * variants;
            self.outcomes
                .extend(outcomes.iter().skip(from).take(variants).copied());
            self.pool = self.pool.max(decision.slot.saturating_add(1));
        }
        let kept_last_day = last_day.or(self.last.map(|(_, day)| day)).unwrap_or(0);
        self.last = Some((month, kept_last_day));
        Ok(())
    }

    /// The canonical order of one month's decisions, already in (slot, id)
    /// order: stable by second, then stable by day.
    fn canonical(decisions: &[Decision]) -> Vec<u32> {
        let identity: Vec<u32> = (0..decisions.len())
            .filter_map(|at| u32::try_from(at).ok())
            .collect();
        let field = |pick: fn(&Decision) -> u32| {
            let low = decisions.iter().map(pick).min().unwrap_or(0);
            let high = decisions.iter().map(pick).max().unwrap_or(0);
            let domain = high.abs_diff(low) as usize + 1;
            let key = move |at: u32| {
                decisions
                    .get(at as usize)
                    .map_or(0, |d| (pick(d) - low) as usize)
            };
            (domain, key)
        };
        let (seconds, by_second) = field(|d| d.second);
        let by_seconds = stable_by(&identity, seconds, by_second);
        let (days, by_day) = field(|d| d.day);
        stable_by(&by_seconds, days, by_day)
    }

    /// Rows held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.coords.len()
    }

    /// Whether the table holds no row.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.coords.is_empty()
    }

    /// Every row index in canonical order (0, 1, 2, ...): a block of it is the
    /// rows of a contiguous run of decisions.
    #[must_use]
    pub fn order(&self) -> &[u32] {
        &self.order
    }

    /// The run's month count.
    #[must_use]
    pub const fn months(&self) -> u32 {
        self.months
    }

    /// One past the largest stock slot held.
    #[must_use]
    pub const fn pool(&self) -> u32 {
        self.pool
    }

    /// The partition.
    #[must_use]
    pub const fn partition(&self) -> &Partition {
        &self.partition
    }

    /// The layout.
    #[must_use]
    pub const fn layout(&self) -> &Layout {
        &self.layout
    }

    /// Row `row`, or `None` past the table.
    #[must_use]
    pub fn row(&self, row: u32) -> Option<RowView<'_>> {
        let at = row as usize;
        let variants = self.variants();
        Some(RowView {
            coords: self.coords.get(at)?,
            slot: *self.slot.get(at)?,
            day: *self.day.get(at)?,
            second: *self.second.get(at)?,
            id: *self.id.get(at)?,
            month: *self.month.get(at)?,
            outcomes: self.outcomes.get(at * variants..(at + 1) * variants)?,
        })
    }

    /// The key of the partition's root: every free row open, the entry-style
    /// and timeframe rows fixed, the exit row open (every variant is scored).
    #[must_use]
    pub fn root_key(&self) -> MixKey {
        let key = with_row(
            MixKey([OPEN; ROW_COUNT]),
            RowId::EntryStyle.index(),
            self.partition.entry_style,
        );
        with_row(key, RowId::Timeframe.index(), self.partition.timeframe)
    }
}

/// Which exit variants a visit is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wanted<'a> {
    /// Every variant of the table (an ordinary walk).
    Every,
    /// Only these, ascending (a guided walk's survivors at this node).
    Only(&'a [u8]),
}

/// One visited node: its key, its rows in canonical order and how many rows it
/// fixes.
#[derive(Clone, Copy, Debug)]
pub struct Node<'a> {
    /// The table.
    table: &'a RowTable,
    /// The traversal key: the exit row is open, because every variant is
    /// scored here.
    key: MixKey,
    /// The rows, ascending.
    rows: &'a [u32],
    /// The fixed rows in the mix (the partition rows not counted).
    fixed: usize,
    /// The variants asked for.
    wanted: Wanted<'a>,
}

impl<'a> Node<'a> {
    /// The table.
    #[must_use]
    pub const fn table(&self) -> &'a RowTable {
        self.table
    }

    /// The traversal key, with the exit row open.
    #[must_use]
    pub const fn key(&self) -> &MixKey {
        &self.key
    }

    /// The rows, in canonical order.
    #[must_use]
    pub const fn rows(&self) -> &'a [u32] {
        self.rows
    }

    /// The raw trade count.
    #[must_use]
    pub fn support(&self) -> u64 {
        self.rows.len() as u64
    }

    /// The fixed rows in the mix, the partition rows not counted.
    #[must_use]
    pub const fn fixed_rows(&self) -> usize {
        self.fixed
    }

    /// The variants asked for.
    #[must_use]
    pub const fn wanted(&self) -> Wanted<'a> {
        self.wanted
    }

    /// The mix of exit variant `variant`: the key with the exit row fixed.
    /// `None` for a variant the table does not score.
    #[must_use]
    pub fn mix(&self, variant: u8) -> Option<MixKey> {
        let exits = &self.table.partition.exits;
        (variant >= exits.start && variant < exits.end)
            .then(|| with_row(self.key, RowId::Exit.index(), variant))
    }
}

/// What the walk hands every visited node to. A forked visitor takes a
/// subtree and is absorbed back in visit order (preorder), so a parallel walk's
/// output is the sequential walk's, byte for byte (FM-F053).
pub trait Visitor: Send + Sized {
    /// One visited node.
    fn visit(&mut self, node: &Node<'_>);

    /// A fresh visitor for a subtree walked elsewhere.
    #[must_use]
    fn fork(&self) -> Self;

    /// Appends a later subtree's visitor, in visit order (preorder).
    fn absorb(&mut self, later: Self);
}

/// How a child was classified.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Child {
    /// Its raw count is below the floor: pruned, counted, never a trial.
    Infrequent,
    /// Its raw count equals its parent's: the same trade set.
    EquivalentToParent,
    /// A prefix child whose raw count equals the prefix before it.
    EquivalentToSibling,
    /// Visited.
    Frequent,
}

/// Classifies a child of raw count `count` under a parent of `parent` rows,
/// with the prefix before it holding `sibling` rows when there is one.
const fn classify(count: u64, parent: u64, sibling: Option<u64>, min: u64) -> Child {
    if count < min {
        return Child::Infrequent;
    }
    if count == parent {
        return Child::EquivalentToParent;
    }
    if let Some(sibling) = sibling
        && count == sibling
    {
        return Child::EquivalentToSibling;
    }
    Child::Frequent
}

/// Slots in a [`Tally`] array: one per possible number of fixed rows.
pub const FIXED_SLOTS: usize = ROW_COUNT + 1;

/// What the walk counted, by the fixed rows in the mix. Independent of the
/// schedule: a parallel walk's tally equals the sequential one's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tally {
    /// Nodes visited.
    pub frequent: [u64; FIXED_SLOTS],
    /// Children skipped as equal to their parent.
    pub equivalent_to_parent: [u64; FIXED_SLOTS],
    /// Prefix children skipped as equal to the prefix before them.
    pub equivalent_to_sibling: [u64; FIXED_SLOTS],
    /// Children below the floor.
    pub infrequent: [u64; FIXED_SLOTS],
    /// Row indices read by a counting pass, a scatter or a prefix scan.
    pub row_touches: u64,
}

impl Tally {
    /// Nothing counted.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            frequent: [0; FIXED_SLOTS],
            equivalent_to_parent: [0; FIXED_SLOTS],
            equivalent_to_sibling: [0; FIXED_SLOTS],
            infrequent: [0; FIXED_SLOTS],
            row_touches: 0,
        }
    }

    /// Counts one child.
    fn count(&mut self, child: Child, fixed: usize) {
        let array = match child {
            Child::Infrequent => &mut self.infrequent,
            Child::EquivalentToParent => &mut self.equivalent_to_parent,
            Child::EquivalentToSibling => &mut self.equivalent_to_sibling,
            Child::Frequent => &mut self.frequent,
        };
        if let Some(slot) = array.get_mut(fixed) {
            *slot += 1;
        }
    }

    /// Adds another tally.
    fn add(&mut self, other: &Self) {
        for (mine, theirs) in [
            (&mut self.frequent, &other.frequent),
            (&mut self.equivalent_to_parent, &other.equivalent_to_parent),
            (
                &mut self.equivalent_to_sibling,
                &other.equivalent_to_sibling,
            ),
            (&mut self.infrequent, &other.infrequent),
        ] {
            for (a, b) in mine.iter_mut().zip(theirs) {
                *a += *b;
            }
        }
        self.row_touches += other.row_touches;
    }

    /// Nodes visited, in all.
    #[must_use]
    pub fn nodes(&self) -> u64 {
        self.frequent.iter().sum()
    }

    /// Mixes visited: every visited node scores each of `variants` exit
    /// variants as its own mix, and each is its own trial (FM-F054).
    #[must_use]
    pub fn mixes(&self, variants: usize) -> u64 {
        self.nodes().saturating_mul(variants as u64)
    }
}

/// How the walk is spread over threads. Never changes an output byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Schedule {
    /// One thread, in preorder.
    Sequential,
    /// A node of at least `split_support` rows walks the children of each
    /// free row as rayon tasks, each with its own scratch and its own forked
    /// visitor, absorbed in visit order (preorder) (FM-F053). `split_support` is
    /// derived by the caller from the measured per-task overhead and
    /// recorded; a batch whose task scratch would pass `batch_budget_bytes`
    /// runs inline and is counted.
    Parallel {
        /// The least support a node needs to split.
        split_support: u64,
        /// The most scratch one batch of tasks may hold, in bytes.
        batch_budget_bytes: u64,
    },
}

/// What the schedule did: information, never part of the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spread {
    /// Subtree tasks spawned.
    pub tasks: u64,
    /// Batches run inline because their scratch would pass the budget.
    pub inline_for_budget: u64,
}

/// A finished walk: its tally and its schedule's record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Walked {
    /// What was counted.
    pub tally: Tally,
    /// What the schedule did.
    pub spread: Spread,
}

/// Where one node sits: its rows with their cells side by side, its key, the
/// next free row it may fix and how many rows it fixes.
#[derive(Clone, Copy)]
struct At<'r> {
    /// The node's rows, ascending.
    rows: &'r [u32],
    /// Their cells, in the same order: a node's own region, never a gather
    /// from the table.
    cells: &'r [Coords],
    /// Its key.
    key: MixKey,
    /// The first position in the free-row list it may fix.
    next: usize,
    /// Its fixed rows.
    fixed: usize,
}

/// The counters one subtree adds.
struct Counted {
    /// The tally.
    tally: Tally,
    /// The spread.
    spread: Spread,
}

impl Counted {
    /// Nothing yet.
    const fn zero() -> Self {
        Self {
            tally: Tally::zero(),
            spread: Spread {
                tasks: 0,
                inline_for_budget: 0,
            },
        }
    }

    /// Adds a subtree's counters.
    fn add(&mut self, other: &Self) {
        self.tally.add(&other.tally);
        self.spread.tasks += other.spread.tasks;
        self.spread.inline_for_budget += other.spread.inline_for_budget;
    }
}

/// The width of one count region: one slot per possible value of a row.
const COUNT_STRIDE: usize = OPEN as usize;

/// The zero cell a fresh scratch region holds.
const BLANK: Coords = [0; ROW_COUNT];

/// Where a task's rows come from.
#[derive(Clone, Copy)]
enum Source<'r> {
    /// Already in place: a categorical child's range of the scatter.
    Rows(&'r [u32], &'r [Coords]),
    /// Scanned by the task itself: a prefix child of the parent's rows.
    Prefix {
        /// The parent's rows.
        rows: &'r [u32],
        /// Their cells.
        cells: &'r [Coords],
        /// The cumulative schema row.
        row: usize,
        /// The widest group kept.
        group: u8,
    },
}

/// One child ready for a task: where its rows come from, and its key.
struct Ready<'r> {
    /// Its rows.
    source: Source<'r>,
    /// Its key.
    key: MixKey,
}

/// A scratch arena: row indices and their cells side by side.
struct Arena<'b> {
    /// The row indices.
    rows: &'b mut [u32],
    /// Their cells.
    cells: &'b mut [Coords],
}

impl<'b> Arena<'b> {
    /// Splits off the first `n` entries (fewer when the arena is shorter):
    /// this node's child region, and the rest for the nodes below.
    fn split(self, n: usize) -> (Arena<'b>, Arena<'b>) {
        let (rows, rows_rest) = self.rows.split_at_mut(n.min(self.rows.len()));
        let (cells, cells_rest) = self.cells.split_at_mut(n.min(self.cells.len()));
        (
            Arena { rows, cells },
            Arena {
                rows: rows_rest,
                cells: cells_rest,
            },
        )
    }

    /// Writes one row and its cell at the cursor and moves the cursor on.
    fn put(&mut self, cursor: &mut u32, index: u32, cell: &Coords) {
        let slot = *cursor as usize;
        if let Some(target) = self.rows.get_mut(slot) {
            *target = index;
        }
        if let Some(target) = self.cells.get_mut(slot) {
            *target = *cell;
        }
        *cursor += 1;
    }

    /// A shorter-lived view of the same arena.
    fn reborrow(&mut self) -> Arena<'_> {
        Arena {
            rows: self.rows,
            cells: self.cells,
        }
    }
}

/// The buffers one node writes: its count region, its scratch arena (its
/// children's region first) and the count regions below it.
struct Bufs<'b> {
    /// The node's own count region.
    hist: &'b mut [u32],
    /// The arena: this node's child region, then everything below.
    arena: Arena<'b>,
    /// The count regions of the nodes below.
    deeper: &'b mut [u32],
}

/// Owned scratch: an arena and count regions, sized once.
struct Owned {
    /// Row indices.
    rows: Vec<u32>,
    /// Their cells.
    cells: Vec<Coords>,
    /// Count regions.
    counts: Vec<u32>,
}

impl Owned {
    /// The arena.
    fn arena(&mut self) -> (Arena<'_>, &mut [u32]) {
        (
            Arena {
                rows: &mut self.rows,
                cells: &mut self.cells,
            },
            &mut self.counts,
        )
    }
}

/// The walk over one table.
struct Walker<'t> {
    /// The table.
    table: &'t RowTable,
    /// The floor.
    min: u64,
    /// The free rows, in row order.
    free: [RowId; FREE_ROWS],
    /// The schedule.
    schedule: Schedule,
    /// Whether children are visited and never expanded ([`expand_range`]).
    once: bool,
}

impl Walker<'_> {
    /// Scratch for a subtree of `rows` rows whose next free position is
    /// `next`: one arena region per free row it may still fix, and one count
    /// region per free row plus its own.
    fn owned(&self, rows: usize, next: usize) -> Owned {
        let regions = self.free.len().saturating_sub(next);
        Owned {
            rows: vec![0_u32; rows.saturating_mul(regions)],
            cells: vec![BLANK; rows.saturating_mul(regions)],
            counts: vec![0_u32; COUNT_STRIDE * (regions + 1)],
        }
    }

    /// Visits a node and walks every child it grows.
    fn expand<V: Visitor>(
        &self,
        at: At<'_>,
        arena: Arena<'_>,
        counts: &mut [u32],
        visitor: &mut V,
        out: &mut Counted,
    ) {
        visitor.visit(&Node {
            table: self.table,
            key: at.key,
            rows: at.rows,
            fixed: at.fixed,
            wanted: Wanted::Every,
        });
        if self.once && at.fixed > 0 {
            return;
        }
        let (hist, deeper) = counts.split_at_mut(COUNT_STRIDE.min(counts.len()));
        let mut bufs = Bufs {
            hist,
            arena,
            deeper,
        };
        for (position, id) in self.free.iter().enumerate().skip(at.next) {
            match id.row().kind {
                RowKind::Cumulative => self.prefixes(at, position, *id, &mut bufs, visitor, out),
                RowKind::Categorical | RowKind::Partition => {
                    self.categories(at, position, *id, &mut bufs, visitor, out);
                }
            }
        }
    }

    /// Fills `hist` with the node's value counts on schema row `row`, reading
    /// the node's own cells in order.
    fn histogram(cells: &[Coords], row: usize, hist: &mut [u32], out: &mut Counted) {
        hist.fill(0);
        for cell in cells {
            let value = cell.get(row).copied().unwrap_or(OPEN);
            if let Some(count) = hist.get_mut(usize::from(value)) {
                *count += 1;
            }
        }
        out.tally.row_touches += cells.len() as u64;
    }

    /// The batch budget when a node of `rows` rows with `children` children
    /// to walk splits them into tasks; `None` when it walks them inline.
    const fn splits(&self, rows: usize, children: usize) -> Option<u64> {
        match self.schedule {
            Schedule::Parallel {
                split_support,
                batch_budget_bytes,
            } if rows as u64 >= split_support && children > 1 => Some(batch_budget_bytes),
            Schedule::Sequential | Schedule::Parallel { .. } => None,
        }
    }

    /// The scratch bytes one batch of tasks below free position `next` holds.
    fn batch_bytes(&self, ready: &[Ready<'_>], next: usize) -> usize {
        let below = self.free.len().saturating_sub(next);
        let entry = size_of::<u32>() + size_of::<Coords>();
        ready.iter().fold(0_usize, |sum, child| {
            let (rows, own) = match child.source {
                Source::Rows(rows, _) => (rows.len(), 0),
                Source::Prefix { rows, .. } => (rows.len(), rows.len()),
            };
            sum.saturating_add(
                rows.saturating_mul(below)
                    .saturating_add(own)
                    .saturating_mul(entry),
            )
        })
    }

    /// Walks a batch of children as rayon tasks, each with its own scratch and
    /// its own forked visitor, absorbed in visit order (FM-F053).
    fn tasks<V: Visitor>(
        &self,
        ready: &[Ready<'_>],
        next: usize,
        fixed: usize,
        visitor: &mut V,
        out: &mut Counted,
    ) {
        let forks: Vec<V> = ready.iter().map(|_| visitor.fork()).collect();
        let done: Vec<(V, Counted)> = forks
            .into_par_iter()
            .zip(ready.par_iter())
            .map(|(mut fork, child)| {
                let mut counted = Counted::zero();
                let mut own_rows = Vec::new();
                let mut own_cells = Vec::new();
                if let Source::Prefix {
                    rows,
                    cells,
                    row,
                    group,
                } = child.source
                {
                    own_rows = vec![0_u32; rows.len()];
                    own_cells = vec![BLANK; rows.len()];
                    let mut region = Arena {
                        rows: &mut own_rows,
                        cells: &mut own_cells,
                    };
                    let kept = Self::scan(rows, cells, row, group, &mut region, &mut counted);
                    own_rows.truncate(kept);
                    own_cells.truncate(kept);
                }
                let (rows, cells) = match child.source {
                    Source::Rows(rows, cells) => (rows, cells),
                    Source::Prefix { .. } => (own_rows.as_slice(), own_cells.as_slice()),
                };
                let mut owned = self.owned(rows.len(), next);
                let (arena, counts) = owned.arena();
                let place = At {
                    rows,
                    cells,
                    key: child.key,
                    next,
                    fixed,
                };
                self.expand(place, arena, counts, &mut fork, &mut counted);
                (fork, counted)
            })
            .collect();
        out.spread.tasks += ready.len() as u64;
        for (fork, counted) in done {
            visitor.absorb(fork);
            out.add(&counted);
        }
    }

    /// Whether to walk `ready` as tasks: the batch fits the budget. A batch
    /// over the budget is counted and runs inline.
    fn as_tasks(&self, ready: &[Ready<'_>], next: usize, budget: u64, out: &mut Counted) -> bool {
        if self.batch_bytes(ready, next) as u64 > budget {
            out.spread.inline_for_budget += 1;
            return false;
        }
        true
    }

    /// The children of a categorical row: one stable counting sort of the
    /// node's rows and cells into the first region of the arena.
    fn categories<V: Visitor>(
        &self,
        at: At<'_>,
        position: usize,
        id: RowId,
        bufs: &mut Bufs<'_>,
        visitor: &mut V,
        out: &mut Counted,
    ) {
        let row = id.index();
        let radix = usize::from(self.table.layout.radix(id));
        let parent = at.rows.len() as u64;
        let hist = bufs.hist.get_mut(..radix).unwrap_or_default();
        Self::histogram(at.cells, row, hist, out);
        let mut frequent = 0;
        for count in hist.iter() {
            let child = classify(u64::from(*count), parent, None, self.min);
            out.tally.count(child, at.fixed + 1);
            frequent += usize::from(child == Child::Frequent);
        }
        if frequent == 0 {
            return;
        }
        let mut end = 0;
        for count in hist.iter_mut() {
            let width = *count;
            *count = end;
            end += width;
        }
        let (mut region, mut rest) = bufs.arena.reborrow().split(at.rows.len());
        for (index, cell) in at.rows.iter().zip(at.cells) {
            let value = cell.get(row).copied().unwrap_or(OPEN);
            if let Some(cursor) = hist.get_mut(usize::from(value)) {
                region.put(cursor, *index, cell);
            }
        }
        out.tally.row_touches += parent;
        let next = position + 1;
        let fixed = at.fixed + 1;
        let (region_rows, region_cells) = (&*region.rows, &*region.cells);
        let children = (0_u8..)
            .zip(hist.iter())
            .scan(0_u32, |start, (value, end)| {
                let span = *start as usize..*end as usize;
                let rows = region_rows.get(span.clone()).unwrap_or_default();
                let cells = region_cells.get(span).unwrap_or_default();
                *start = *end;
                Some((value, rows, cells))
            })
            .filter(|(_, rows, _)| {
                classify(rows.len() as u64, parent, None, self.min) == Child::Frequent
            });
        if let Some(budget) = self.splits(at.rows.len(), frequent) {
            let ready: Vec<Ready<'_>> = children
                .map(|(value, rows, cells)| Ready {
                    source: Source::Rows(rows, cells),
                    key: with_row(at.key, row, value),
                })
                .collect();
            if self.as_tasks(&ready, next, budget, out) {
                self.tasks(&ready, next, fixed, visitor, out);
                return;
            }
            for child in ready {
                if let Source::Rows(rows, cells) = child.source {
                    let place = At {
                        rows,
                        cells,
                        key: child.key,
                        next,
                        fixed,
                    };
                    self.expand(place, rest.reborrow(), bufs.deeper, visitor, out);
                }
            }
            return;
        }
        for (value, rows, cells) in children {
            let place = At {
                rows,
                cells,
                key: with_row(at.key, row, value),
                next,
                fixed,
            };
            self.expand(place, rest.reborrow(), bufs.deeper, visitor, out);
        }
    }

    /// The prefix children of the cumulative row: each one stable linear
    /// scan of the node's rows into the first region of the arena (FM-F055).
    fn prefixes<V: Visitor>(
        &self,
        at: At<'_>,
        position: usize,
        id: RowId,
        bufs: &mut Bufs<'_>,
        visitor: &mut V,
        out: &mut Counted,
    ) {
        let row = id.index();
        let radix = usize::from(self.table.layout.radix(id));
        let parent = at.rows.len() as u64;
        let hist = bufs.hist.get_mut(..radix).unwrap_or_default();
        Self::histogram(at.cells, row, hist, out);
        let classes = (0_u8..).zip(hist.iter()).scan(
            (0_u64, None),
            move |(below, sibling), (group, count)| {
                let prefix = *below + u64::from(*count);
                let child = classify(prefix, parent, *sibling, self.min);
                *below = prefix;
                *sibling = Some(prefix);
                Some((group, child))
            },
        );
        let mut frequent = 0;
        for (_, child) in classes.clone() {
            out.tally.count(child, at.fixed + 1);
            frequent += usize::from(child == Child::Frequent);
        }
        let wanted =
            classes.filter_map(|(group, child)| (child == Child::Frequent).then_some(group));
        let next = position + 1;
        let fixed = at.fixed + 1;
        if let Some(budget) = self.splits(at.rows.len(), frequent) {
            let ready: Vec<Ready<'_>> = wanted
                .map(|group| Ready {
                    source: Source::Prefix {
                        rows: at.rows,
                        cells: at.cells,
                        row,
                        group,
                    },
                    key: with_row(at.key, row, group),
                })
                .collect();
            if self.as_tasks(&ready, next, budget, out) {
                self.tasks(&ready, next, fixed, visitor, out);
                return;
            }
            for child in ready {
                if let Source::Prefix { group, .. } = child.source {
                    self.prefix_inline(
                        at,
                        row,
                        group,
                        next,
                        bufs.arena.reborrow(),
                        bufs.deeper,
                        visitor,
                        out,
                    );
                }
            }
            return;
        }
        for group in wanted {
            self.prefix_inline(
                at,
                row,
                group,
                next,
                bufs.arena.reborrow(),
                bufs.deeper,
                visitor,
                out,
            );
        }
    }

    /// Builds one prefix child in the first region of the arena and walks it.
    #[allow(
        clippy::too_many_arguments,
        reason = "one node's place, the prefix it builds, and the buffers and sinks it writes"
    )]
    fn prefix_inline<V: Visitor>(
        &self,
        at: At<'_>,
        row: usize,
        group: u8,
        next: usize,
        arena: Arena<'_>,
        deeper: &mut [u32],
        visitor: &mut V,
        out: &mut Counted,
    ) {
        let (region, rest) = arena.split(at.rows.len());
        let (rows, cells) = (region.rows, region.cells);
        let kept = Self::scan(
            at.rows,
            at.cells,
            row,
            group,
            &mut Arena {
                rows: &mut *rows,
                cells: &mut *cells,
            },
            out,
        );
        let place = At {
            rows: rows.get(..kept).unwrap_or_default(),
            cells: cells.get(..kept).unwrap_or_default(),
            key: with_row(at.key, row, group),
            next,
            fixed: at.fixed + 1,
        };
        self.expand(place, rest, deeper, visitor, out);
    }

    /// Copies the rows (and cells) whose group on schema row `row` is at most
    /// `group` into `region`, in order; returns how many.
    fn scan(
        rows: &[u32],
        cells: &[Coords],
        row: usize,
        group: u8,
        region: &mut Arena<'_>,
        out: &mut Counted,
    ) -> usize {
        let mut kept = 0;
        for (index, cell) in rows.iter().zip(cells) {
            if cell.get(row).copied().unwrap_or(OPEN) <= group {
                if let Some(target) = region.rows.get_mut(kept) {
                    *target = *index;
                }
                if let Some(target) = region.cells.get_mut(kept) {
                    *target = *cell;
                }
                kept += 1;
            }
        }
        out.tally.row_touches += rows.len() as u64;
        kept
    }
}

/// Walks the table's lattice to extinction at `min_support` raw trades,
/// handing every visited node to `visitor` in visit order (preorder).
///
/// # Errors
///
/// [`LatticeRefusal::MinSupportZero`].
pub fn walk<V: Visitor>(
    table: &RowTable,
    min_support: u64,
    schedule: Schedule,
    visitor: &mut V,
) -> Result<Walked, LatticeRefusal> {
    if min_support == 0 {
        return Err(LatticeRefusal::MinSupportZero);
    }
    let walker = Walker {
        table,
        min: min_support,
        free: free_rows(),
        schedule,
        once: false,
    };
    let mut counted = Counted::zero();
    let rows = &table.order;
    if (rows.len() as u64) < min_support {
        counted.tally.count(Child::Infrequent, 0);
    } else {
        counted.tally.count(Child::Frequent, 0);
        let mut owned = walker.owned(rows.len(), 0);
        let (arena, counts) = owned.arena();
        let root = At {
            rows,
            cells: &table.coords,
            key: table.root_key(),
            next: 0,
            fixed: 0,
        };
        walker.expand(root, arena, counts, visitor, &mut counted);
    }
    Ok(Walked {
        tally: counted.tally,
        spread: counted.spread,
    })
}

/// Caller-owned scratch for [`expand_range`]: one child region and two count
/// regions, sized once for the largest range it will expand, so an expansion
/// allocates nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scratch {
    /// Child row indices.
    rows: Vec<u32>,
    /// Their cells.
    cells: Vec<Coords>,
    /// Count regions.
    counts: Vec<u32>,
}

impl Scratch {
    /// Scratch for ranges of up to `rows` rows.
    #[must_use]
    pub fn for_rows(rows: usize) -> Self {
        Self {
            rows: vec![0; rows],
            cells: vec![BLANK; rows],
            counts: vec![0; COUNT_STRIDE + COUNT_STRIDE],
        }
    }

    /// The largest range it can expand.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows.len()
    }
}

/// Expands ONE node: the table's contiguous rows `range` as the root of their
/// own lattice (every free row open), handing the node and each frequent
/// child to `visitor` and expanding no child. The unit of work the walk does
/// per node, on a block of rows whose cells already lie side by side — the
/// shape every child region of a walk has. The bench's row-touch op.
///
/// # Errors
///
/// [`LatticeRefusal::MinSupportZero`], [`LatticeRefusal::RangeOutsideTable`]
/// and [`LatticeRefusal::ScratchTooSmall`].
pub fn expand_range<V: Visitor>(
    table: &RowTable,
    range: Range<u32>,
    min_support: u64,
    scratch: &mut Scratch,
    visitor: &mut V,
) -> Result<Tally, LatticeRefusal> {
    if min_support == 0 {
        return Err(LatticeRefusal::MinSupportZero);
    }
    let span = range.start as usize..range.end as usize;
    let cells = table
        .coords
        .get(span.clone())
        .filter(|_| range.start <= range.end)
        .ok_or(LatticeRefusal::RangeOutsideTable {
            start: range.start,
            end: range.end,
            rows: table.len(),
        })?;
    if span.len() > scratch.rows() {
        return Err(LatticeRefusal::ScratchTooSmall {
            needed: span.len(),
            have: scratch.rows(),
        });
    }
    let rows = table.order.get(span).unwrap_or_default();
    let walker = Walker {
        table,
        min: min_support,
        free: free_rows(),
        schedule: Schedule::Sequential,
        once: true,
    };
    let mut counted = Counted::zero();
    if (rows.len() as u64) < min_support {
        counted.tally.count(Child::Infrequent, 0);
        return Ok(counted.tally);
    }
    counted.tally.count(Child::Frequent, 0);
    let arena = Arena {
        rows: &mut scratch.rows,
        cells: &mut scratch.cells,
    };
    let node = At {
        rows,
        cells,
        key: table.root_key(),
        next: 0,
        fixed: 0,
    };
    walker.expand(node, arena, &mut scratch.counts, visitor, &mut counted);
    Ok(counted.tally)
}

/// One node of the survivor trie.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TrieNode {
    /// Its edges, as a range of [`Survivors::edges`].
    edges: Range<usize>,
    /// The exit variants of the survivors ending here, ascending.
    variants: Vec<u8>,
}

/// One trie edge: fix schema row `row` to `value`, reaching node `child`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Edge {
    /// The parent node.
    parent: u32,
    /// The schema row fixed.
    row: u8,
    /// Its value.
    value: u8,
    /// The child node.
    child: u32,
}

/// The survivor keys of one partition as a trie in the walk's fixing order
/// (FM-F052): a guided walk descends only along it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Survivors {
    /// The nodes; node 0 is the partition's root.
    nodes: Vec<TrieNode>,
    /// Every edge, grouped by parent and ordered by (row, value) inside it.
    edges: Vec<Edge>,
    /// How many survivor keys it holds.
    keys: usize,
}

impl Survivors {
    /// The trie of `keys`, each a full mix of `table`'s partition with its exit
    /// variant fixed.
    ///
    /// # Errors
    ///
    /// [`LatticeRefusal::Mix`] for a key the layout refuses,
    /// [`LatticeRefusal::SideOutsideSearch`],
    /// [`LatticeRefusal::PartitionMismatch`] for another entry style or
    /// timeframe, [`LatticeRefusal::SurvivorExitOutside`] and
    /// [`LatticeRefusal::SurvivorRepeated`].
    pub fn new(table: &RowTable, keys: &[MixKey]) -> Result<Self, LatticeRefusal> {
        let free = free_rows();
        let mut child_of: HashMap<(u32, u8, u8), u32> =
            HashMap::with_capacity(keys.len().saturating_mul(free.len()) + 1);
        let mut variants: Vec<Vec<u8>> = vec![Vec::new()];
        let mut edges = Vec::new();
        let mut distinct: HashSet<MixKey> = HashSet::with_capacity(keys.len() + 1);
        for key in keys {
            Self::check(table, key)?;
            if !distinct.insert(*key) {
                return Err(LatticeRefusal::SurvivorRepeated { key: *key });
            }
            let mut node = 0_u32;
            for id in &free {
                let value = key.0.get(id.index()).copied().unwrap_or(OPEN);
                if value == OPEN {
                    continue;
                }
                let row = u8::try_from(id.index()).unwrap_or(OPEN);
                let fresh = u32::try_from(variants.len()).unwrap_or(u32::MAX);
                let child = *child_of.entry((node, row, value)).or_insert(fresh);
                if child == fresh {
                    variants.push(Vec::new());
                    edges.push(Edge {
                        parent: node,
                        row,
                        value,
                        child,
                    });
                }
                node = child;
            }
            let exit = key.0.get(RowId::Exit.index()).copied().unwrap_or(OPEN);
            if let Some(here) = variants.get_mut(node as usize) {
                here.push(exit);
            }
        }
        let nodes = variants.len();
        let edges = stable_edges(&edges, nodes);
        let mut ranges = vec![0_usize; nodes + 1];
        for edge in &edges {
            if let Some(count) = ranges.get_mut(edge.parent as usize + 1) {
                *count += 1;
            }
        }
        let mut run = 0;
        for slot in &mut ranges {
            run += *slot;
            *slot = run;
        }
        let exits = usize::from(table.layout.radix(RowId::Exit));
        let trie = variants
            .into_iter()
            .zip(ranges.iter().zip(ranges.iter().skip(1)))
            .map(|(list, (start, end))| TrieNode {
                edges: *start..*end,
                variants: ascending(&list, exits),
            })
            .collect();
        Ok(Self {
            nodes: trie,
            edges,
            keys: keys.len(),
        })
    }

    /// Checks one survivor key against the table's layout and partition.
    fn check(table: &RowTable, key: &MixKey) -> Result<(), LatticeRefusal> {
        table
            .layout
            .validate_mix(key)
            .map_err(LatticeRefusal::Mix)?;
        let partition = &table.partition;
        if partition.search.admits_mix(key).is_err() {
            return Err(LatticeRefusal::SideOutsideSearch {
                search: partition.search.name(),
                side: key.0.first().copied().unwrap_or(OPEN),
            });
        }
        for (id, want) in [
            (RowId::EntryStyle, partition.entry_style),
            (RowId::Timeframe, partition.timeframe),
        ] {
            let have = key.0.get(id.index()).copied().unwrap_or(OPEN);
            if have != want {
                return Err(LatticeRefusal::PartitionMismatch {
                    row: id.row().name,
                    want,
                    have,
                });
            }
        }
        let exit = key.0.get(RowId::Exit.index()).copied().unwrap_or(OPEN);
        if exit < partition.exits.start || exit >= partition.exits.end {
            return Err(LatticeRefusal::SurvivorExitOutside { variant: exit });
        }
        Ok(())
    }

    /// How many survivor keys the trie holds.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.keys
    }

    /// Whether it holds none.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.keys == 0
    }
}

/// `list`'s distinct values ascending, each below `values`: O(list + values).
fn ascending(list: &[u8], values: usize) -> Vec<u8> {
    if list.is_empty() {
        return Vec::new();
    }
    let mut seen = vec![false; values];
    for value in list {
        if let Some(slot) = seen.get_mut(usize::from(*value)) {
            *slot = true;
        }
    }
    (0_u8..)
        .zip(seen)
        .filter_map(|(value, here)| here.then_some(value))
        .collect()
}

/// The edges grouped by parent, ordered by (row, value) inside each group:
/// three stable counting sorts, O(edges + values + rows + nodes).
fn stable_edges(edges: &[Edge], nodes: usize) -> Vec<Edge> {
    let order: Vec<u32> = (0..edges.len())
        .filter_map(|at| u32::try_from(at).ok())
        .collect();
    let field = |pick: fn(&Edge) -> usize| move |at: u32| edges.get(at as usize).map_or(0, pick);
    let by_value = stable_by(&order, COUNT_STRIDE, field(|e| usize::from(e.value)));
    let by_row = stable_by(&by_value, ROW_COUNT, field(|e| usize::from(e.row)));
    let by_parent = stable_by(&by_row, nodes, field(|e| e.parent as usize));
    by_parent
        .iter()
        .filter_map(|at| edges.get(*at as usize).copied())
        .collect()
}

/// What a guided walk counted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuidedTally {
    /// Survivor nodes handed to the visitor.
    pub scored: u64,
    /// Nodes walked through on the way to a survivor and not scored.
    pub passed_through: u64,
    /// Row indices read by the child scans.
    pub row_touches: u64,
}

/// The guided walk over one table.
struct Guide<'t> {
    /// The table.
    table: &'t RowTable,
    /// The trie.
    survivors: &'t Survivors,
}

impl Guide<'_> {
    /// Scores trie node `trie` when a survivor ends there, then walks each of
    /// its trie children, with no floor and no equivalence skip.
    fn descend<V: Visitor>(
        &self,
        trie: &TrieNode,
        at: At<'_>,
        arena: Arena<'_>,
        visitor: &mut V,
        tally: &mut GuidedTally,
    ) {
        if trie.variants.is_empty() {
            tally.passed_through += 1;
        } else {
            tally.scored += 1;
            visitor.visit(&Node {
                table: self.table,
                key: at.key,
                rows: at.rows,
                fixed: at.fixed,
                wanted: Wanted::Only(&trie.variants),
            });
        }
        let edges = self
            .survivors
            .edges
            .get(trie.edges.clone())
            .unwrap_or_default();
        let (region, mut rest) = arena.split(at.rows.len());
        for edge in edges {
            let row = usize::from(edge.row);
            let cumulative = RowId::ALL
                .get(row)
                .is_some_and(|id| id.row().kind == RowKind::Cumulative);
            let mut kept = 0;
            for (index, cell) in at.rows.iter().zip(at.cells) {
                let value = cell.get(row).copied().unwrap_or(OPEN);
                let inside = if cumulative {
                    value <= edge.value
                } else {
                    value == edge.value
                };
                if inside {
                    if let Some(target) = region.rows.get_mut(kept) {
                        *target = *index;
                    }
                    if let Some(target) = region.cells.get_mut(kept) {
                        *target = *cell;
                    }
                    kept += 1;
                }
            }
            tally.row_touches += at.rows.len() as u64;
            let child = At {
                rows: region.rows.get(..kept).unwrap_or_default(),
                cells: region.cells.get(..kept).unwrap_or_default(),
                key: with_row(at.key, row, edge.value),
                next: at.next,
                fixed: at.fixed + 1,
            };
            if let Some(next) = self.survivors.nodes.get(edge.child as usize) {
                self.descend(next, child, rest.reborrow(), visitor, tally);
            }
        }
    }
}

/// Walks only along the survivor trie (FM-F052): every survivor node is
/// handed to `visitor` with the variants asked for, whatever its raw count and
/// however equal to its parent. Cost: O(sum of supports along survivor paths).
#[must_use]
pub fn walk_guided<V: Visitor>(
    table: &RowTable,
    survivors: &Survivors,
    visitor: &mut V,
) -> GuidedTally {
    let guide = Guide { table, survivors };
    let rows = &table.order;
    let regions = rows.len().saturating_mul(free_rows().len());
    let mut arena_rows = vec![0_u32; regions];
    let mut arena_cells = vec![BLANK; regions];
    let mut tally = GuidedTally {
        scored: 0,
        passed_through: 0,
        row_touches: 0,
    };
    let root = At {
        rows,
        cells: &table.coords,
        key: table.root_key(),
        next: 0,
        fixed: 0,
    };
    let arena = Arena {
        rows: &mut arena_rows,
        cells: &mut arena_cells,
    };
    if let Some(trie) = survivors.nodes.first() {
        guide.descend(trie, root, arena, visitor, &mut tally);
    }
    tally
}

/// Checks that every survivor has exactly one verdict and no verdict names a
/// key that is not a survivor (FM-F052: `fib verify` FAILS on a mismatch).
/// Expected O(survivors + verdicts): one pre-sized hash map.
///
/// # Errors
///
/// [`LatticeRefusal::SurvivorRepeated`], [`LatticeRefusal::VerdictWithoutSurvivor`],
/// [`LatticeRefusal::VerdictRepeated`] and
/// [`LatticeRefusal::SurvivorWithoutVerdict`], the first found in input order.
pub fn reconcile(survivors: &[MixKey], verdicts: &[MixKey]) -> Result<(), LatticeRefusal> {
    let mut seen: HashMap<MixKey, u32> = HashMap::with_capacity(survivors.len() + 1);
    for key in survivors {
        if seen.insert(*key, 0).is_some() {
            return Err(LatticeRefusal::SurvivorRepeated { key: *key });
        }
    }
    for key in verdicts {
        match seen.get_mut(key) {
            None => return Err(LatticeRefusal::VerdictWithoutSurvivor { key: *key }),
            Some(count) if *count > 0 => {
                return Err(LatticeRefusal::VerdictRepeated { key: *key });
            }
            Some(count) => *count += 1,
        }
    }
    for key in survivors {
        if seen.get(key) == Some(&0) {
            return Err(LatticeRefusal::SurvivorWithoutVerdict { key: *key });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! What an integration test cannot reach: a table past `u32` rows.

    use super::{Child, Counted, FREE_ROWS, LatticeRefusal, Spread, free_count, indexable};

    /// A subtree's counters add into its parent's, every field: the schedule's
    /// record as well as the tally (a task's are absorbed this way).
    #[test]
    fn a_subtrees_counters_add_into_its_parents() {
        let mut parent = Counted::zero();
        parent.spread = Spread {
            tasks: 2,
            inline_for_budget: 3,
        };
        parent.tally.count(Child::Frequent, 1);
        parent.tally.row_touches = 10;
        let mut child = Counted::zero();
        child.spread = Spread {
            tasks: 5,
            inline_for_budget: 7,
        };
        child.tally.count(Child::Frequent, 1);
        child.tally.count(Child::Infrequent, 2);
        child.tally.count(Child::EquivalentToParent, 2);
        child.tally.count(Child::EquivalentToSibling, 3);
        child.tally.row_touches = 4;
        parent.add(&child);
        assert_eq!(
            parent.spread,
            Spread {
                tasks: 7,
                inline_for_budget: 10
            }
        );
        assert_eq!(parent.tally.frequent[1], 2);
        assert_eq!(parent.tally.infrequent[2], 1);
        assert_eq!(parent.tally.equivalent_to_parent[2], 1);
        assert_eq!(parent.tally.equivalent_to_sibling[3], 1);
        assert_eq!(parent.tally.row_touches, 14);
    }

    #[test]
    fn a_table_past_what_a_u32_row_index_names_is_refused() {
        let most = usize::try_from(u32::MAX).unwrap_or(usize::MAX);
        assert_eq!(indexable(most), Ok(()));
        assert_eq!(indexable(most + 1), Err(LatticeRefusal::RowsExceedU32));
        assert_eq!(indexable(0), Ok(()));
    }

    /// The free-row count is evaluated at compile time; run here, it agrees.
    #[test]
    fn the_free_row_count_runs_the_same_at_run_time() {
        assert_eq!(free_count(), FREE_ROWS);
    }
}
