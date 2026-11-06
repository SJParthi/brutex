//! F57 `report::row_stats` and F60 `report::head_compare` (PREVIEW-SPEC section 3.10).

use std::collections::BTreeMap;

use crate::account::MonthRow;
use crate::book::View;
use crate::sizing::ACCOUNT;

/// Every number of one row that is not a month or a trade list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowStats {
    /// Months in the window.
    pub months: usize,
    /// Months at or above +2.5%.
    pub months_pass: usize,
    /// Months with net above 0.
    pub months_positive: usize,
    /// Worst month bp (months with positive capital only).
    pub worst_bp: Option<i128>,
    /// The lower median month bp: the `ceil(m / 2)`-th smallest.
    pub median_bp: Option<i128>,
    /// Best month bp.
    pub best_bp: Option<i128>,
    /// Sum of trade nets, paisa.
    pub total_net: i128,
    /// `(10_000 x total).div_euclid(200,000,000)` bp.
    pub total_bp: i128,
    /// Admitted trades that filled.
    pub trades: u64,
    /// Trades with net above 0.
    pub wins: u64,
    /// Trades with net below 0.
    pub losses: u64,
    /// Wins per trade, permille floored (`None`: no trades).
    pub win_rate: Option<i128>,
    /// Sum of winners' net / wins, floored (`None`: no wins).
    pub avg_win: Option<i128>,
    /// Sum of losers' net / losses, floored (`None`: no losses).
    pub avg_loss: Option<i128>,
    /// Gross win / gross loss in thousandths, floored (`None`: no loss).
    pub profit_factor: Option<i128>,
    /// Sum of headlines, paisa.
    pub charges: i128,
}

/// F57: the row's statistics from its months and its trades' nets and headlines.
#[must_use]
pub fn row_stats(months: &[MonthRow], nets: &[i128], charges: i128) -> RowStats {
    let mut bps: Vec<i128> = months.iter().filter_map(|m| m.bp).collect();
    bps.sort_unstable();
    let median_bp = bps
        .len()
        .checked_sub(1)
        .and_then(|i| bps.get(i / 2))
        .copied();
    let total_net: i128 = nets.iter().sum();
    let wins: Vec<i128> = nets.iter().copied().filter(|n| *n > 0).collect();
    let losses: Vec<i128> = nets.iter().copied().filter(|n| *n < 0).collect();
    let count = |v: &[i128]| u64::try_from(v.len()).unwrap_or(u64::MAX);
    let trades = u64::try_from(nets.len()).unwrap_or(u64::MAX);
    let gross_win: i128 = wins.iter().sum();
    let gross_loss: i128 = -losses.iter().sum::<i128>();
    let per = |sum: i128, n: usize| -> Option<i128> {
        let n = i128::try_from(n).ok().filter(|n| *n > 0)?;
        Some(sum.div_euclid(n))
    };
    RowStats {
        months: months.len(),
        months_pass: months.iter().filter(|m| m.pass).count(),
        months_positive: months.iter().filter(|m| m.net > 0).count(),
        worst_bp: bps.first().copied(),
        median_bp,
        best_bp: bps.last().copied(),
        total_net,
        total_bp: (10_000 * total_net).div_euclid(i128::from(ACCOUNT)),
        trades,
        wins: count(&wins),
        losses: count(&losses),
        win_rate: per(1_000 * i128::from(count(&wins)), nets.len()),
        avg_win: per(gross_win, wins.len()),
        avg_loss: per(-gross_loss, losses.len()),
        profit_factor: (gross_loss > 0).then(|| (1_000 * gross_win).div_euclid(gross_loss)),
        charges,
    }
}

/// F60: `over / base` in permille, floored; `None` when the base is 0 or negative
/// (printed as such).
#[must_use]
pub fn permille(over: i128, base: i128) -> Option<i128> {
    (base > 0).then(|| (1_000 * over).div_euclid(base))
}

/// One measure of the head: a permille, or the base itself when it is 0 or negative
/// (printed as such, never divided).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Per {
    /// `over / base` in permille, floored.
    Permille(i128),
    /// The zero or negative base.
    Base(i128),
}

/// `over` against `base`: [`permille`], or the base when it cannot divide.
#[must_use]
pub fn per(over: i128, base: i128) -> Per {
    permille(over, base).map_or(Per::Base(base), Per::Permille)
}

/// The measures of one row the head compares: trades, total net, and the max drawdown
/// in both of its readings (bp and paisa, section 3.9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Measures {
    /// Admitted trades that filled.
    pub trades: i128,
    /// Total net, paisa.
    pub net: i128,
    /// Max drawdown, bp.
    pub dd_bp: i128,
    /// Max drawdown, paisa.
    pub dd: i128,
}

/// One row as the head reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeadRow {
    /// Index into [`crate::ruleset::RULE_SETS`].
    pub set: usize,
    /// The timeframe in minutes.
    pub k: u32,
    /// The account.
    pub view: View,
    /// Its measures.
    pub measures: Measures,
    /// Its floor, the worst month bp (`None`: no month has a bp).
    pub floor: Option<i128>,
}

/// One comparison of the head: rule set `over` against rule set `base` on one timeframe
/// and account.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeadLine {
    /// The timeframe.
    pub k: u32,
    /// The account.
    pub view: View,
    /// The rule set compared.
    pub over: usize,
    /// The rule set it is compared against.
    pub base: usize,
    /// Trades.
    pub trades: Per,
    /// Total net.
    pub net: Per,
    /// Max drawdown, bp.
    pub dd_bp: Per,
    /// Max drawdown, paisa.
    pub dd: Per,
}

/// The head of a run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Head {
    /// Every comparison, timeframe and account in the rows' order, then the pairs' order.
    pub lines: Vec<HeadLine>,
    /// The nearest met floor: the highest floor of any row (even a negative one) and the
    /// first row holding it; `None` when no row has a floor.
    pub nearest: Option<(usize, i128)>,
}

/// The head's pairs (over, base) as indices into [`crate::ruleset::RULE_SETS`]:
/// AUTHOR-V2 over VIDEO-ORIGINAL (PREVIEW-SPEC 3.10), VIDEO-SCREEN over VIDEO-ORIGINAL
/// (TEST-PLAN T-F60-1), and VIDEO-SCREEN over AUTHOR-V2 (Q10's other reading).
pub const PAIRS: [(usize, usize); 3] = [(1, 0), (2, 0), (2, 1)];

/// F60: per timeframe and account (in the order the rows first name them), each pair whose
/// two rows are present compared on every measure; and the nearest met floor over all
/// rows. O(r log r) once per run.
#[must_use]
pub fn head_compare(rows: &[HeadRow], pairs: &[(usize, usize)]) -> Head {
    let mut by_key: BTreeMap<(u32, View, usize), &HeadRow> = BTreeMap::new();
    let mut keys: Vec<(u32, View)> = Vec::new();
    for r in rows {
        if !keys.contains(&(r.k, r.view)) {
            keys.push((r.k, r.view));
        }
        by_key.entry((r.k, r.view, r.set)).or_insert(r);
    }
    let mut lines = Vec::new();
    for (k, view) in keys {
        for &(over, base) in pairs {
            let o = by_key.get(&(k, view, over));
            let b = by_key.get(&(k, view, base));
            if let (Some(o), Some(b)) = (o, b) {
                let (o, b) = (o.measures, b.measures);
                lines.push(HeadLine {
                    k,
                    view,
                    over,
                    base,
                    trades: per(o.trades, b.trades),
                    net: per(o.net, b.net),
                    dd_bp: per(o.dd_bp, b.dd_bp),
                    dd: per(o.dd, b.dd),
                });
            }
        }
    }
    let nearest = rows
        .iter()
        .enumerate()
        .filter_map(|(i, r)| r.floor.map(|f| (i, f)))
        .fold(None, |best: Option<(usize, i128)>, (i, f)| {
            if best.is_none_or(|(_, b)| f > b) {
                Some((i, f))
            } else {
                best
            }
        });
    Head { lines, nearest }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
