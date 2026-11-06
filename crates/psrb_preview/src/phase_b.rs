//! Phase B: one account per (rule set, timeframe, side); slots, months, drawdown.
//!
//! 3.6 to 3.9 of the spec: one position per stock, 20 slots, ties by (time, symbol),
//! month net % on the compounding base, drawdown on closed-trade equity. Integers only.

use std::collections::BTreeSet;

use crate::common::{
    ACCOUNT_PAISA, Cand, N_MONTHS, SLOTS, Status, ist_day_of, window_index,
};

/// What one account did with the candidates it saw.
#[derive(Debug, Default, Clone)]
pub(crate) struct AccountOut {
    pub(crate) signals: u64,
    pub(crate) while_open: u64,
    pub(crate) slots_full: u64,
    pub(crate) not_filled: u64,
    pub(crate) zero_qty: u64,
    pub(crate) no_ratio: u64,
    /// Indices into the group's candidates of the trades it took.
    pub(crate) trades: Vec<usize>,
}

/// `acct` 0 LONG, 1 SHORT, 2 BOTH. `cands` sorted by (T, symbol rank, side).
pub(crate) fn admit(cands: &[Cand], acct: usize) -> AccountOut {
    let mut out = AccountOut::default();
    let mut open: Vec<(i64, u16)> = Vec::with_capacity(SLOTS);
    for (idx, c) in cands.iter().enumerate() {
        if (acct == 0 && c.side != 0) || (acct == 1 && c.side != 1) {
            continue;
        }
        out.signals += 1;
        // A position holds its slot from its signal instant to its exit fill second, inclusive.
        open.retain(|&(exit, _)| exit >= c.t_signal);
        if open.iter().any(|&(_, s)| s == c.stock) {
            out.while_open += 1;
            continue;
        }
        if open.len() >= SLOTS {
            out.slots_full += 1;
            continue;
        }
        match c.status {
            Status::Filled => {
                open.push((c.exit_us, c.stock));
                out.trades.push(idx);
            }
            Status::EntryNotFilled => out.not_filled += 1,
            Status::ZeroQty => out.zero_qty += 1,
            Status::NoRatio => out.no_ratio += 1,
        }
    }
    out
}

/// One calendar month of one account.
#[derive(Debug, Clone, Default)]
pub(crate) struct MonthRow {
    pub(crate) cap_start: i128,
    pub(crate) net: i128,
    pub(crate) trades: u64,
    pub(crate) wins: u64,
    pub(crate) charges: i128,
    pub(crate) eq_end: i128,
    /// `None`: the month started at capital <= 0 ("account at or below zero").
    pub(crate) bp: Option<i128>,
    pub(crate) pass: bool,
}

/// Every figure of 3.10 for one account.
#[derive(Debug, Clone, Default)]
pub(crate) struct Stats {
    pub(crate) months: Vec<MonthRow>,
    pub(crate) months_pass: usize,
    pub(crate) months_pos: usize,
    pub(crate) zero_cap_months: usize,
    /// `None` when a month started at capital <= 0.
    pub(crate) worst_bp: Option<i128>,
    pub(crate) median_bp: Option<i128>,
    pub(crate) best_bp: Option<i128>,
    pub(crate) total_net: i128,
    pub(crate) total_bp: i128,
    pub(crate) max_dd_bp: i128,
    pub(crate) max_dd_paisa: i128,
    pub(crate) dd_peak_ts: i64,
    pub(crate) dd_low_ts: i64,
    pub(crate) trades: u64,
    pub(crate) wins: u64,
    pub(crate) losses: u64,
    pub(crate) win_pm: Option<i128>,
    pub(crate) avg_win: Option<i128>,
    pub(crate) avg_loss: Option<i128>,
    /// `None`: no loss.
    pub(crate) pf_th: Option<i128>,
    pub(crate) gross_win: i128,
    pub(crate) gross_loss: i128,
    pub(crate) charges: i128,
    pub(crate) gross: i128,
    pub(crate) late: u64,
    pub(crate) exits: [u64; 5],
    pub(crate) fallback_trades: u64,
    pub(crate) fallback_stock_days: u64,
    pub(crate) out_of_window: u64,
}

/// 3.8 to 3.10 over the trades an account took.
pub(crate) fn metrics(cands: &[Cand], trades: &[usize], start_ts: i64) -> Stats {
    let mut s = Stats { months: vec![MonthRow::default(); N_MONTHS], ..Stats::default() };
    let mut order: Vec<&Cand> = trades.iter().filter_map(|&i| cands.get(i)).collect();
    order.sort_by_key(|c| (c.exit_us, c.stock, c.t_signal, c.side));
    let mut eq = i128::from(ACCOUNT_PAISA);
    let mut peak = eq;
    let mut peak_ts = start_ts;
    s.dd_peak_ts = start_ts;
    s.dd_low_ts = start_ts;
    let mut fb_days: BTreeSet<(u16, i64)> = BTreeSet::new();
    for c in &order {
        let net = i128::from(c.net);
        match window_index(c.exit_us).and_then(|wi| s.months.get_mut(wi)) {
            Some(m) => {
                m.net += net;
                m.trades += 1;
                m.charges += i128::from(c.charges);
                if c.net > 0 {
                    m.wins += 1;
                }
            }
            None => s.out_of_window += 1,
        }
        s.trades += 1;
        s.total_net += net;
        s.charges += i128::from(c.charges);
        s.gross += i128::from(c.gross);
        if c.net > 0 {
            s.wins += 1;
            s.gross_win += net;
        } else if c.net < 0 {
            s.losses += 1;
            s.gross_loss -= net;
        }
        if c.late {
            s.late += 1;
        }
        if let Some(x) = s.exits.get_mut(c.exit_kind.index()) {
            *x += 1;
        }
        if c.fallback {
            s.fallback_trades += 1;
            fb_days.insert((c.stock, ist_day_of(c.exit_us)));
        }
        eq += net;
        if eq > peak {
            peak = eq;
            peak_ts = c.exit_us;
        } else if peak > 0 {
            let bp = (10_000 * (peak - eq)).div_euclid(peak);
            if bp > s.max_dd_bp {
                s.max_dd_bp = bp;
                s.max_dd_paisa = peak - eq;
                s.dd_peak_ts = peak_ts;
                s.dd_low_ts = c.exit_us;
            }
        }
    }
    s.fallback_stock_days = u64::try_from(fb_days.len()).unwrap_or(u64::MAX);
    let mut cap = i128::from(ACCOUNT_PAISA);
    for m in &mut s.months {
        m.cap_start = cap;
        if cap > 0 {
            m.bp = Some((10_000 * m.net).div_euclid(cap));
            m.pass = 10_000 * m.net >= 250 * cap;
        } else {
            m.bp = None;
            m.pass = false;
        }
        cap += m.net;
        m.eq_end = cap;
    }
    s.months_pass = s.months.iter().filter(|m| m.pass).count();
    s.months_pos = s.months.iter().filter(|m| m.net > 0).count();
    s.zero_cap_months = s.months.iter().filter(|m| m.bp.is_none()).count();
    let mut bps: Vec<i128> = s.months.iter().map(|m| m.bp.unwrap_or(i128::MIN)).collect();
    bps.sort_unstable();
    let some = |x: Option<&i128>| x.copied().filter(|v| *v != i128::MIN);
    s.worst_bp = some(bps.first());
    s.median_bp = some(bps.get(48));
    s.best_bp = some(bps.last());
    s.total_bp = (10_000 * s.total_net).div_euclid(i128::from(ACCOUNT_PAISA));
    let t = i128::from(s.trades);
    s.win_pm = (t > 0).then(|| (1_000 * i128::from(s.wins)).div_euclid(t));
    let w = i128::from(s.wins);
    let l = i128::from(s.losses);
    s.avg_win = (w > 0).then(|| s.gross_win.div_euclid(w));
    s.avg_loss = (l > 0).then(|| (-s.gross_loss).div_euclid(l));
    s.pf_th = (s.gross_loss > 0).then(|| (1_000 * s.gross_win).div_euclid(s.gross_loss));
    s
}

/// Whether a row passes: every one of the 98 months at or above +2.5% (OI:28).
pub(crate) fn passes(s: &Stats) -> bool {
    s.months_pass == N_MONTHS
}

/// The months below +2.5%: (window index, month bp or None, gap in bp).
pub(crate) fn misses(s: &Stats) -> Vec<(usize, Option<i128>, Option<i128>)> {
    s.months
        .iter()
        .enumerate()
        .filter(|(_, m)| !m.pass)
        .map(|(i, m)| (i, m.bp, m.bp.map(|bp| 250 - bp)))
        .collect()
}

/// Ranking (OI:29, :31): passers by lowest max drawdown, then most net, then win rate;
/// then misses by fewest months below +2.5%, then highest worst month, then lowest max
/// drawdown, then most net.
pub(crate) fn rank_key(s: &Stats) -> (u8, i128, i128, i128, i128) {
    if passes(s) {
        (0, s.max_dd_bp, -s.total_net, -s.win_pm.unwrap_or(0), 0)
    } else {
        let below = i128::try_from(N_MONTHS - s.months_pass).unwrap_or(i128::MAX);
        (1, below, -s.worst_bp.unwrap_or(i128::MIN / 4), s.max_dd_bp, -s.total_net)
    }
}

