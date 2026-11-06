//! F58 `verdict::bar` and F59 `verdict::rank` (PREVIEW-SPEC section 3.10, "Verdict").

use std::cmp::Ordering;

use crate::account::MonthRow;

/// One month below the +2.5% bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gap {
    /// The month's place in the window, from 0.
    pub index: usize,
    /// `250 - month_bp` (`None` when the capital was at or below 0).
    pub bp: Option<i128>,
    /// The exact gap `250 x capital - 10_000 x net`, in paisa x 10,000.
    pub exact: i128,
}

/// A row's verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bar {
    /// Every month is at or above +2.5%.
    pub pass: bool,
    /// Every month below the bar, in window order.
    pub gaps: Vec<Gap>,
    /// The highest monthly floor the row holds: its worst month's bp.
    pub floor: Option<i128>,
}

/// F58: PASS iff every month passes; else every month below with its gap, and the floor.
#[must_use]
pub fn bar(months: &[MonthRow]) -> Bar {
    let gaps: Vec<Gap> = months
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.pass)
        .map(|(index, m)| Gap {
            index,
            bp: m.bp.map(|b| 250 - b),
            exact: 250 * m.capital_start - 10_000 * m.net,
        })
        .collect();
    let floor = months.iter().filter_map(|m| m.bp).min();
    Bar {
        pass: !months.is_empty() && gaps.is_empty(),
        gaps,
        floor,
    }
}

/// What the ranking reads of a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rankable {
    /// PASS.
    pub pass: bool,
    /// Max drawdown, bp.
    pub max_dd_bp: i128,
    /// Total net, paisa.
    pub total_net: i128,
    /// Win rate, permille.
    pub win_rate: i128,
    /// Months below +2.5%.
    pub months_below: usize,
    /// The worst month's bp (`None` sorts lowest).
    pub worst: Option<i128>,
    /// The final deterministic tie: (rule set, timeframe, account) order.
    pub tie: (usize, u32, usize),
}

/// F59's comparator: passers first, by lowest drawdown, then most net, then highest win
/// rate; then misses by fewest months below, then highest worst month, then lowest
/// drawdown, then most net; every full tie by (rule set, timeframe, account).
#[must_use]
pub fn compare(a: &Rankable, b: &Rankable) -> Ordering {
    let head = b.pass.cmp(&a.pass);
    let body = if a.pass && b.pass {
        a.max_dd_bp
            .cmp(&b.max_dd_bp)
            .then(b.total_net.cmp(&a.total_net))
            .then(b.win_rate.cmp(&a.win_rate))
    } else {
        a.months_below
            .cmp(&b.months_below)
            .then(b.worst.cmp(&a.worst))
            .then(a.max_dd_bp.cmp(&b.max_dd_bp))
            .then(b.total_net.cmp(&a.total_net))
    };
    head.then(body).then(a.tie.cmp(&b.tie))
}

/// F59: the rows in rank order (indices into `rows`).
#[must_use]
pub fn rank(rows: &[Rankable]) -> Vec<usize> {
    let mut order: Vec<(usize, &Rankable)> = rows.iter().enumerate().collect();
    order.sort_by(|a, b| compare(a.1, b.1));
    order.into_iter().map(|(i, _)| i).collect()
}

#[cfg(test)]
#[path = "verdict_tests.rs"]
mod tests;
