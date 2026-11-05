//! SIZE-FIXED-20-RANKED's first key: the learned edge of a stock, side and time of day (D-8634).
//!
//! **Source.** The candidate's trades in the window's learning months from its
//! SIZE-FIXED-20-SEEDED walk under tie-break (e) with the run's first seed: a recipe's own
//! trades or, for a procedure, its picked recipes' trades pooled (an [`EdgeTrade`] carries no
//! recipe). A trade whose entry filled nothing is not a trade and is not here. FNO-RUN walks and
//! hands them in.
//!
//! **Value.** Net per rupee of entry value, `floor(net x 10^6 / value)` parts per million, a
//! floor for a negative net too (`div_euclid`, never `/`, which rounds toward zero). A cell's
//! score is the mean of its values, compared exactly.
//!
//! **Cells, three levels.** (stock, side, time-of-day bucket), then (stock, side), then
//! (side): [`EdgeTable::score`] returns the finest level holding a trade, else no trade. The
//! buckets are generated, never typed: FNO-RUN fits FNO-WFT's form-G quantile ladder over the
//! learning trades' entry minutes and hands the rungs in; a minute's bucket is the number of
//! rungs at or below it.
//!
//! **Learning months only, never the month it judges.** [`EdgeTable::build`] refuses a trade
//! outside `[learn_from, test_from)`. A forward month is ordered by the whole table. When RANKED
//! walks a learning month, [`EdgeTable::judging`] takes that month's own trades out of all three
//! levels and puts the month taken out before back (leave one month out); the bucket edges stay,
//! as they were fitted over entry times only.
//!
//! **Cost.** A score is three array reads. Building is O(trades + cells) and a month boundary
//! is O(trades of the two months): bounded, not O(1), recorded in `docs/06-limits.md` and
//! measured by rows C-FNO-SIZE-02, -05 and -06 of `crates/runner/benches/ratio.rs`.

use core::cmp::Ordering;

use super::{Invalid, PPM, SESSION_MINUTES, Side, StockId, UNIVERSE, read};

/// The minute map's length: one entry per minute of session.
const MINUTES: usize = 375;

/// The (stock, side) pairs: two per stock position.
const PAIRS: usize = UNIVERSE * 2;

/// How fine a score's cell is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    /// The (stock, side, time-of-day bucket) cell.
    Cell,
    /// The (stock, side) cell.
    StockSide,
    /// The (side) cell.
    Side,
    /// No learning trade at any level.
    None,
}

/// A score: the sum of a cell's values in parts per million, and how many trades it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// The level the score was read at.
    pub level: Level,
    /// Parts per million: the sum of the cell's values.
    pub sum: i128,
    /// The trades in the cell; 0 for no trade.
    pub count: u64,
}

impl Edge {
    /// The score of no trade.
    pub const NONE: Self = Self {
        level: Level::None,
        sum: 0,
        count: 0,
    };

    /// The scores ordered by their mean, `sum / count`, exactly: no trade is below every mean,
    /// and two scores with no trade, or with equal means from different counts (2/4 and 1/2),
    /// are equal.
    #[must_use]
    pub fn cmp_mean(&self, other: &Self) -> Ordering {
        MeanKey::of(self).against(&MeanKey::of(other))
    }
}

/// A mean in a form that compares exactly with no wide product: the floor of `sum / count`,
/// the remainder and the count. Two means compare by floor, then by `r1 / c1` against
/// `r2 / c2`, cross-multiplied in `u128`, where each remainder is below its count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MeanKey {
    /// `floor(sum / count)`.
    quotient: i128,
    /// `sum - quotient x count`, in `[0, count)`.
    remainder: u128,
    /// The count; 0 for no trade.
    count: u64,
}

impl MeanKey {
    /// No trade.
    pub(super) const NONE: Self = Self {
        quotient: 0,
        remainder: 0,
        count: 0,
    };

    /// The mean of `edge`.
    pub(super) fn of(edge: &Edge) -> Self {
        let divisor = i128::from(edge.count.max(1));
        Self {
            quotient: edge.sum.div_euclid(divisor),
            remainder: edge.sum.rem_euclid(divisor).unsigned_abs(),
            count: edge.count,
        }
    }

    /// Ascending by mean; no trade is below every mean.
    pub(super) fn against(&self, other: &Self) -> Ordering {
        match (self.count, other.count) {
            (0, 0) => Ordering::Equal,
            (0, _) => Ordering::Less,
            (_, 0) => Ordering::Greater,
            (mine, theirs) => self.quotient.cmp(&other.quotient).then_with(|| {
                // Each remainder is below 2^64 and so is each count: the products are exact.
                (self.remainder * u128::from(theirs)).cmp(&(other.remainder * u128::from(mine)))
            }),
        }
    }
}

/// One learning trade as FNO-RUN hands it in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeTrade {
    /// The trade's month, in the run's month numbering.
    pub month: u16,
    /// The stock.
    pub stock: StockId,
    /// The side.
    pub side: Side,
    /// The entry's minute of session, 0..=374.
    pub minute: u16,
    /// Paise: FNO-NET's headline net under the worse clock.
    pub net: i128,
    /// Paise: the exact sum over the entry's fills of quantity times fill price, above 0.
    pub value: i128,
}

/// One cell: the sum of its values and its count.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    sum: i128,
    count: u64,
}

impl Cell {
    /// No trade.
    const EMPTY: Self = Self { sum: 0, count: 0 };

    /// Adds `value` as one more trade, or takes one trade of `value` out. Every sum the table
    /// can hold is a sum of a subset of its values, and the sum of their magnitudes was checked
    /// against `i128::MAX` when they were inserted, so neither direction can overflow.
    fn apply(&mut self, value: i128, add: bool) {
        if add {
            self.sum += value;
            self.count += 1;
        } else {
            self.sum -= value;
            self.count -= 1;
        }
    }
}

/// One stored trade: its three cells and its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    cell: usize,
    pair: usize,
    side: usize,
    value: i128,
}

/// The learned edge of one (candidate, scheme, window): RANKED's first key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeTable {
    /// Time-of-day buckets: the rungs plus one.
    buckets: u16,
    /// Each minute of session's bucket.
    minute_bucket: [u16; MINUTES],
    /// (stock, side, bucket) cells.
    cells: Vec<Cell>,
    /// (stock, side) cells.
    pairs: Vec<Cell>,
    /// (side) cells.
    sides: [Cell; 2],
    /// The first learning month.
    learn_from: u16,
    /// The first month after the learning months.
    test_from: u16,
    /// The month of the latest trade inserted.
    last_month: u16,
    /// Every trade, in month order.
    entries: Vec<Entry>,
    /// `starts[r]` is the first entry of month `learn_from + r`; one more than the months.
    starts: Vec<usize>,
    /// The learning month taken out, if any.
    out: Option<u16>,
    /// The sum of every value's magnitude, at most `i128::MAX`.
    magnitude: u128,
}

impl Default for EdgeTable {
    fn default() -> Self {
        Self::empty()
    }
}

impl EdgeTable {
    /// A table with no rung, no learning month and no trade.
    pub(super) fn empty() -> Self {
        Self {
            buckets: 1,
            minute_bucket: [0; MINUTES],
            cells: vec![Cell::EMPTY; PAIRS],
            pairs: vec![Cell::EMPTY; PAIRS],
            sides: [Cell::EMPTY; 2],
            learn_from: 0,
            test_from: 0,
            last_month: 0,
            entries: Vec::new(),
            starts: vec![0],
            out: None,
            magnitude: 0,
        }
    }

    /// The table of `trades`, the learning months `[learn_from, test_from)` in month order,
    /// bucketed by `rungs`.
    ///
    /// # Errors
    ///
    /// [`Invalid::BadRungs`] for rungs not strictly ascending or past minute 374; then, per
    /// trade in order, [`Invalid::MonthOutsideLearning`], [`Invalid::MonthsOutOfOrder`],
    /// [`Invalid::MinuteOutOfSession`], [`Invalid::NonPositiveValue`], and
    /// [`Invalid::Overflow`] for a value past `i128` (`"edge value"`) or values whose
    /// magnitudes sum past `i128::MAX` (`"edge sum"`).
    pub fn build(
        rungs: &[u16],
        learn_from: u16,
        test_from: u16,
        trades: &[EdgeTrade],
    ) -> Result<Self, Invalid> {
        let mut table = Self::empty();
        table.reset(rungs, learn_from, test_from)?;
        for trade in trades {
            table.insert(trade)?;
        }
        let months = usize::from(test_from.saturating_sub(learn_from));
        while table.starts.len() <= months {
            table.starts.push(table.entries.len());
        }
        Ok(table)
    }

    /// Clears every cell and trade and rebuilds the minute map from `rungs`, for the learning
    /// months `[learn_from, test_from)`. Nothing changes when the rungs are refused.
    pub(super) fn reset(
        &mut self,
        rungs: &[u16],
        learn_from: u16,
        test_from: u16,
    ) -> Result<(), Invalid> {
        let mut last: Option<u16> = None;
        for &rung in rungs {
            if rung >= SESSION_MINUTES || last.is_some_and(|before| before >= rung) {
                return Err(Invalid::BadRungs);
            }
            last = Some(rung);
        }
        let mut bucket = 0u16;
        let mut ahead = rungs.iter().peekable();
        for (minute, slot) in (0u16..).zip(self.minute_bucket.iter_mut()) {
            while ahead.next_if(|&&rung| rung <= minute).is_some() {
                bucket += 1;
            }
            *slot = bucket;
        }
        self.buckets = bucket + 1;
        self.cells.clear();
        self.cells
            .resize(PAIRS * usize::from(self.buckets), Cell::EMPTY);
        self.pairs.clear();
        self.pairs.resize(PAIRS, Cell::EMPTY);
        self.sides = [Cell::EMPTY; 2];
        self.learn_from = learn_from;
        self.test_from = test_from;
        self.last_month = learn_from;
        self.entries.clear();
        self.starts.clear();
        self.out = None;
        self.magnitude = 0;
        Ok(())
    }

    /// Adds one learning trade, after the trades of earlier months.
    pub(super) fn insert(&mut self, trade: &EdgeTrade) -> Result<(), Invalid> {
        if trade.month < self.learn_from || trade.month >= self.test_from {
            return Err(Invalid::MonthOutsideLearning { month: trade.month });
        }
        if trade.month < self.last_month {
            return Err(Invalid::MonthsOutOfOrder);
        }
        if trade.minute >= SESSION_MINUTES {
            return Err(Invalid::MinuteOutOfSession(trade.minute));
        }
        if trade.value <= 0 {
            return Err(Invalid::NonPositiveValue);
        }
        let value = trade
            .net
            .checked_mul(PPM)
            .ok_or(Invalid::Overflow("edge value"))?
            .div_euclid(trade.value);
        let magnitude = self
            .magnitude
            .checked_add(value.unsigned_abs())
            .filter(|total| *total <= i128::MAX.unsigned_abs())
            .ok_or(Invalid::Overflow("edge sum"))?;
        let month = usize::from(trade.month - self.learn_from);
        while self.starts.len() <= month {
            self.starts.push(self.entries.len());
        }
        let pair = pair_at(trade.stock, trade.side);
        let entry = Entry {
            cell: pair * usize::from(self.buckets) + usize::from(self.bucket(trade.minute)),
            pair,
            side: trade.side.index(),
            value,
        };
        self.entries.push(entry);
        apply(
            &mut self.cells,
            &mut self.pairs,
            &mut self.sides,
            &entry,
            true,
        );
        self.magnitude = magnitude;
        self.last_month = trade.month;
        Ok(())
    }

    /// The finest-level score of a stock, side and minute of session, three array reads.
    #[must_use]
    pub fn score(&self, stock: StockId, side: Side, minute: u16) -> Edge {
        let pair = pair_at(stock, side);
        let at = pair * usize::from(self.buckets) + usize::from(self.bucket(minute));
        let cell = read(&self.cells, at, Cell::EMPTY);
        if cell.count > 0 {
            return Edge {
                level: Level::Cell,
                sum: cell.sum,
                count: cell.count,
            };
        }
        let pair = read(&self.pairs, pair, Cell::EMPTY);
        if pair.count > 0 {
            return Edge {
                level: Level::StockSide,
                sum: pair.sum,
                count: pair.count,
            };
        }
        let whole = read(&self.sides, side.index(), Cell::EMPTY);
        if whole.count > 0 {
            return Edge {
                level: Level::Side,
                sum: whole.sum,
                count: whole.count,
            };
        }
        Edge::NONE
    }

    /// A minute's bucket: the number of rungs at or below it. A minute past the session is past
    /// every rung, so it is in the last bucket.
    #[must_use]
    pub fn bucket(&self, minute: u16) -> u16 {
        read(&self.minute_bucket, usize::from(minute), self.buckets - 1)
    }

    /// The number of time-of-day buckets: the rungs plus one.
    #[must_use]
    pub const fn buckets(&self) -> u16 {
        self.buckets
    }

    /// The learning month taken out, if any.
    #[must_use]
    pub const fn judged(&self) -> Option<u16> {
        self.out
    }

    /// Scores the learning month `month` without its own trades: puts back the month taken out
    /// before, if another, then takes `month`'s trades out of all three levels. O(trades of the
    /// two months).
    ///
    /// # Errors
    ///
    /// [`Invalid::MonthOutsideLearning`] for a month outside the learning months; the table is
    /// unchanged.
    pub fn judging(&mut self, month: u16) -> Result<(), Invalid> {
        if month < self.learn_from || month >= self.test_from {
            return Err(Invalid::MonthOutsideLearning { month });
        }
        if self.out == Some(month) {
            return Ok(());
        }
        self.whole();
        self.shift_month(month, false);
        self.out = Some(month);
        Ok(())
    }

    /// The whole learning table again, for a forward month: puts back the month taken out.
    pub fn whole(&mut self) {
        if let Some(month) = self.out.take() {
            self.shift_month(month, true);
        }
    }

    /// Adds a learning month's trades back, or takes them out.
    fn shift_month(&mut self, month: u16, add: bool) {
        let at = usize::from(month - self.learn_from);
        let (from, to) = (read(&self.starts, at, 0), read(&self.starts, at + 1, 0));
        let Self {
            cells,
            pairs,
            sides,
            entries,
            ..
        } = self;
        for entry in entries.get(from..to).unwrap_or_default() {
            apply(cells, pairs, sides, entry, add);
        }
    }
}

/// The (stock, side) pair's index.
fn pair_at(stock: StockId, side: Side) -> usize {
    stock.at() * 2 + side.index()
}

/// Applies one entry to its three cells.
fn apply(cells: &mut [Cell], pairs: &mut [Cell], sides: &mut [Cell; 2], entry: &Entry, add: bool) {
    let mine = cells
        .iter_mut()
        .skip(entry.cell)
        .take(1)
        .chain(pairs.iter_mut().skip(entry.pair).take(1))
        .chain(sides.iter_mut().skip(entry.side).take(1));
    for cell in mine {
        cell.apply(entry.value, add);
    }
}
