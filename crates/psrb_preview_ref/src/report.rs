//! F57 `report::row_stats` and F60 `report::head_compare` (PREVIEW-SPEC section 3.10).

use crate::account::MonthRow;
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
    let median_bp = bps.len().checked_sub(1).and_then(|i| bps.get(i / 2)).copied();
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

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
