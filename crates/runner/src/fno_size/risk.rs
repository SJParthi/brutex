//! SIZE-RISK's two budgets and its open-risk book (D-8633).
//!
//! **The budgets** come from the learning window only, from the same candidate's
//! SIZE-FIXED-20-SEEDED walk under tie-break (e) with the run's first seed, counting only its
//! trades with a stop at a positive distance: `per_trade` is the median initial risk `q x d`
//! (the lower one of an even count, FNO-WFT's median rule) and `open_cap` the peak open risk
//! that walk held, [`RiskBook::peak`] over it. FNO-RUN computes both; nothing is typed. With no
//! such trade the policy input carries no budget and every eligible row is refused
//! `size_no_learned_risk`.
//!
//! **Open risk** is the sum of the initial risks of the positions in force, each held until
//! FNO-PORTFOLIO would free its slot: through its last fill second, settled at the first call of
//! a later second. A stop moved to break-even frees nothing until the position closes, the
//! reading that admits less. The book keeps one row per stock and a list of the released rows
//! ordered by their last fill second. It takes releases in any order and with a future last
//! fill, as the account does: a release joins the list at its place, found by walking back from
//! the tail, one step in last-fill order and at most 255 otherwise (bounded by the universe,
//! not O(1)). [`RiskBook::settle`] frees from the head each row whose last fill is before the
//! second, each row once, so amortised O(1). Measured by row C-FNO-SIZE-04 of
//! `crates/runner/benches/ratio.rs`; the bound of the walk is recorded in `docs/06-limits.md`.

use super::{Invalid, StockId, put, read};

/// The list's sentinel node, past the 256 stock rows: its `next` is the head of the released
/// rows and its `prev` the tail.
const SENTINEL: u16 = 256;

/// The stock rows and the sentinel.
const NODES: usize = 257;

/// SIZE-RISK's two budgets, both learned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RiskBudget {
    per_trade: i128,
    open_cap: i128,
}

impl RiskBudget {
    /// The budgets in paise: the risk a trade may take and the open risk the account may hold.
    ///
    /// # Errors
    ///
    /// [`Invalid::NonPositiveBudget`] when either is at or below 0. A correct FNO-RUN never
    /// meets it: one trade with a stop at a positive distance makes both at least 1 paisa.
    pub const fn new(per_trade: i128, open_cap: i128) -> Result<Self, Invalid> {
        if per_trade <= 0 || open_cap <= 0 {
            return Err(Invalid::NonPositiveBudget);
        }
        Ok(Self {
            per_trade,
            open_cap,
        })
    }

    /// Paise: the median initial risk of the learning trades.
    #[must_use]
    pub const fn per_trade(self) -> i128 {
        self.per_trade
    }

    /// Paise: the peak open risk of the learning walk.
    #[must_use]
    pub const fn open_cap(self) -> i128 {
        self.open_cap
    }
}

/// Where a stock's row stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// No position.
    Free,
    /// A position in force, not yet released.
    Held,
    /// Released with its last fill second, not yet settled; still in force.
    Released,
}

/// One stock's row, and its links in the released list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Row {
    state: State,
    admitted: i64,
    after: i64,
    risk: i128,
    prev: u16,
    next: u16,
}

impl Row {
    /// A stock with no position, and the sentinel of an empty list.
    const FREE: Self = Self {
        state: State::Free,
        admitted: 0,
        after: 0,
        risk: 0,
        prev: SENTINEL,
        next: SENTINEL,
    };
}

/// SIZE-RISK's open-risk book, one per walk. FNO-RUN calls it beside every account call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskBook {
    rows: Vec<Row>,
    open_risk: i128,
    peak: i128,
    clock: i64,
}

impl Default for RiskBook {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskBook {
    /// A book with nothing in force.
    #[must_use]
    pub fn new() -> Self {
        Self {
            rows: vec![Row::FREE; NODES],
            open_risk: 0,
            peak: 0,
            clock: i64::MIN,
        }
    }

    /// Paise: the initial risks of the positions in force.
    #[must_use]
    pub const fn open_risk(&self) -> i128 {
        self.open_risk
    }

    /// Paise: the largest open risk the book has held.
    #[must_use]
    pub const fn peak(&self) -> i128 {
        self.peak
    }

    /// The node's row.
    fn row(&self, node: u16) -> Row {
        read(&self.rows, usize::from(node), Row::FREE)
    }

    /// Writes the node's row.
    fn set(&mut self, node: u16, row: Row) {
        put(&mut self.rows, usize::from(node), row);
    }

    /// Admits `stock` at second `at` with its initial risk in paise.
    ///
    /// # Errors
    ///
    /// [`Invalid::TimeBackwards`] for a second before the clock, [`Invalid::StockInForce`] for
    /// a stock held or released and not yet settled, [`Invalid::NegativeView`] (`"risk"`) for a
    /// risk below 0, and [`Invalid::Overflow`] (`"open risk"`) past `i128`. Nothing changes.
    pub fn admit(&mut self, at: i64, stock: StockId, risk: i128) -> Result<(), Invalid> {
        if at < self.clock {
            return Err(Invalid::TimeBackwards {
                at,
                clock: self.clock,
            });
        }
        let node = u16::from(stock.0);
        if self.row(node).state != State::Free {
            return Err(Invalid::StockInForce(stock.0));
        }
        if risk < 0 {
            return Err(Invalid::NegativeView("risk"));
        }
        let open = self
            .open_risk
            .checked_add(risk)
            .ok_or(Invalid::Overflow("open risk"))?;
        self.set(
            node,
            Row {
                state: State::Held,
                admitted: at,
                after: at,
                risk,
                ..Row::FREE
            },
        );
        self.clock = at;
        self.open_risk = open;
        self.peak = self.peak.max(open);
        Ok(())
    }

    /// Releases `stock` with its last fill second `after`, which may lie ahead of the clock.
    /// Returns how many released rows the walk from the tail passed: 0 for a release in
    /// last-fill order, at most 255.
    ///
    /// # Errors
    ///
    /// [`Invalid::NotInForce`] for a stock not held and [`Invalid::ReleaseBeforeAdmit`] for a
    /// last fill before its admission. Nothing changes.
    pub fn release(&mut self, stock: StockId, after: i64) -> Result<u16, Invalid> {
        let node = u16::from(stock.0);
        let row = self.row(node);
        if row.state != State::Held {
            return Err(Invalid::NotInForce(stock.0));
        }
        if after < row.admitted {
            return Err(Invalid::ReleaseBeforeAdmit {
                after,
                admitted: row.admitted,
            });
        }
        let mut place = self.row(SENTINEL).prev;
        let mut passed = 0u16;
        while place != SENTINEL && self.row(place).after > after {
            place = self.row(place).prev;
            passed += 1;
        }
        let next = self.row(place).next;
        self.set(
            node,
            Row {
                state: State::Released,
                after,
                prev: place,
                next,
                ..row
            },
        );
        let mut before = self.row(place);
        before.next = node;
        self.set(place, before);
        let mut behind = self.row(next);
        behind.prev = node;
        self.set(next, behind);
        Ok(passed)
    }

    /// Settles at second `at`: frees every released row whose last fill second is before it.
    ///
    /// # Errors
    ///
    /// [`Invalid::TimeBackwards`] for a second before the clock. Nothing changes.
    pub fn settle(&mut self, at: i64) -> Result<(), Invalid> {
        if at < self.clock {
            return Err(Invalid::TimeBackwards {
                at,
                clock: self.clock,
            });
        }
        self.clock = at;
        let mut head = self.row(SENTINEL).next;
        while head != SENTINEL && self.row(head).after < at {
            let row = self.row(head);
            let mut sentinel = self.row(SENTINEL);
            sentinel.next = row.next;
            self.set(SENTINEL, sentinel);
            let mut behind = self.row(row.next);
            behind.prev = SENTINEL;
            self.set(row.next, behind);
            self.set(head, Row::FREE);
            // The open risk holds this row's risk, so the difference is exact.
            self.open_risk -= row.risk;
            head = row.next;
        }
        Ok(())
    }
}
