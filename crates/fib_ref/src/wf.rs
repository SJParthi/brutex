//! THE RANKING, the walk-forward pick, the win-rate gate, the luck test's
//! clearing rule and the verdict (PREVIEW-SPEC 14, 24.R, 24.P R1-R4,
//! 24.P1 Fix 7).
//!
//! * **THE RANKING** over a month range: passers first (every month has a
//!   trade and nets at least the fixed Rs 2,50,000; R1), then (2) the lowest
//!   max drawdown in tenths of a percent of Rs 20 lakh, (3) the most net,
//!   (4) the highest win rate (zero trades below every rate), (5) fewer
//!   trades, (6) shorter average holding, (7) higher net per trade, (8) the
//!   stable id. Misses: the least total shortfall to the fixed bar (a
//!   no-trade month owes all of it), then the highest worst month, then keys
//!   2-8.
//! * **Windows** Y2020..Y2026, anchored: window Y learns on 2018-09 ..
//!   (Y-1)-12 and is judged on year Y (2026 to October): 82 unseen months.
//! * **The gate** clears seed Zsq when `W > L` and `(W - L)^2 x 1000 >= Zsq
//!   x (W + L)`, exact in `u128`.
//! * **The luck test** clears when `(1 + #{b : best_b >= O}) x 20 <= 1001`.
//! * **The verdict**, the only writer of line 2, rules 1-7 of 14.5 as 24.R,
//!   R1 and R3 amend them.

use std::cmp::Ordering;

use crate::account::dd_tenths;
use crate::account::{BAR, DATA_CHARGE};
use crate::model::{DataChargeAt, InterimLine, Readings};

/// One trade of a series, as the ranking reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeriesTrade {
    /// Month index of the trade.
    pub month: i32,
    /// Net, paise.
    pub net: i128,
    /// Holding, seconds.
    pub hold: u32,
}

/// One series: a recipe's trades in close order, from its exact account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Series {
    /// Stable id.
    pub id: u32,
    /// Trades in close order.
    pub trades: Vec<SeriesTrade>,
}

/// THE RANKING's key of a series over a month range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    /// Every month has a trade and nets at least the bar.
    pub passer: bool,
    /// Max drawdown, tenths of a percent of Rs 20 lakh.
    pub dd_tenths: i128,
    /// Max drawdown, paise.
    pub dd: i128,
    /// Net over the range (trade nets less data charges), paise.
    pub net: i128,
    /// Wins (net above 0).
    pub wins: u64,
    /// Trades.
    pub trades: u64,
    /// Sum of holdings, seconds.
    pub hold: u128,
    /// Total shortfall to the bar, paise.
    pub shortfall: i128,
    /// The worst month's net, paise.
    pub worst: i128,
    /// Months in the range at the bar (a trade and net at least the bar).
    pub at_bar: u32,
    /// Months in the range.
    pub months: u32,
    /// Stable id.
    pub id: u32,
}

/// The key of `s` over months `first..=last`.
pub fn key(s: &Series, first: i32, last: i32, r: &Readings) -> Key {
    let span = usize::try_from(last - first + 1).unwrap_or(0);
    let mut net_m = vec![0i128; span];
    let mut count_m = vec![0u32; span];
    let mut k = Key {
        passer: true,
        dd_tenths: 0,
        dd: 0,
        net: 0,
        wins: 0,
        trades: 0,
        hold: 0,
        shortfall: 0,
        worst: 0,
        at_bar: 0,
        months: u32::try_from(span).unwrap_or(0),
        id: s.id,
    };
    let (mut eq, mut peak) = (0i128, 0i128);
    for t in s
        .trades
        .iter()
        .filter(|t| (first..=last).contains(&t.month))
    {
        let ix = usize::try_from(t.month - first).unwrap_or(0);
        if count_m[ix] == 0 {
            net_m[ix] -= i128::from(DATA_CHARGE);
            if r.data_charge_at == DataChargeAt::FirstFill {
                eq -= i128::from(DATA_CHARGE);
                k.dd = k.dd.max(peak - eq);
            }
        }
        count_m[ix] += 1;
        net_m[ix] += t.net;
        eq += t.net;
        peak = peak.max(eq);
        k.dd = k.dd.max(peak - eq);
        k.trades += 1;
        k.wins += u64::from(t.net > 0);
        k.hold += u128::from(t.hold);
    }
    let mut worst: Option<i128> = None;
    for (n, c) in net_m.iter().zip(&count_m) {
        let at = *c > 0 && *n >= i128::from(BAR);
        k.passer &= at;
        k.at_bar += u32::from(at);
        k.net += n;
        k.shortfall += (i128::from(BAR) - n).max(0);
        worst = Some(worst.map_or(*n, |w| w.min(*n)));
    }
    k.passer &= span > 0;
    k.worst = worst.unwrap_or(0);
    k.dd_tenths = dd_tenths(k.dd, r.dd_round);
    k
}

/// Keys 2-8: `Less` when `a` ranks above `b`.
fn keys_2_8(a: &Key, b: &Key) -> Ordering {
    let rate = match (a.trades, b.trades) {
        (0, 0) => Ordering::Equal,
        (0, _) => Ordering::Greater,
        (_, 0) => Ordering::Less,
        (at, bt) => {
            (u128::from(b.wins) * u128::from(at)).cmp(&(u128::from(a.wins) * u128::from(bt)))
        }
    };
    let (at, bt) = (i128::from(a.trades), i128::from(b.trades));
    let hold = (a.hold * u128::from(b.trades)).cmp(&(b.hold * u128::from(a.trades)));
    let per_trade = (b.net * at).cmp(&(a.net * bt));
    a.dd_tenths
        .cmp(&b.dd_tenths)
        .then(b.net.cmp(&a.net))
        .then(rate)
        .then(a.trades.cmp(&b.trades))
        .then(hold)
        .then(per_trade)
        .then(a.id.cmp(&b.id))
}

/// THE RANKING: `Less` when `a` ranks above `b`.
pub fn cmp_ranking(a: &Key, b: &Key) -> Ordering {
    match (a.passer, b.passer) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (true, true) => keys_2_8(a, b),
        (false, false) => a
            .shortfall
            .cmp(&b.shortfall)
            .then(b.worst.cmp(&a.worst))
            .then(keys_2_8(a, b)),
    }
}

/// A walk-forward window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    /// Test year.
    pub year: i32,
    /// First learning month.
    pub learn_first: i32,
    /// Last learning month.
    pub learn_last: i32,
    /// First unseen month.
    pub unseen_first: i32,
    /// Last unseen month.
    pub unseen_last: i32,
}

/// The seven anchored windows of 14.1 over data 2018-09 .. 2026-10.
pub fn windows() -> Vec<Window> {
    let start = 2018 * 12 + 8; // 2018-09
    let end = 2026 * 12 + 9; // 2026-10
    (2020..=2026)
        .map(|y| Window {
            year: y,
            learn_first: start,
            learn_last: y * 12 - 1,
            unseen_first: y * 12,
            unseen_last: (y * 12 + 11).min(end),
        })
        .collect()
}

/// Window Y's pick: the #1 by THE RANKING on its learning months.
pub fn pick(cands: &[Series], w: &Window, r: &Readings) -> Option<u32> {
    cands
        .iter()
        .map(|s| key(s, w.learn_first, w.learn_last, r))
        .min_by(cmp_ranking)
        .map(|k| k.id)
}

/// One month of a chain: trades and net.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainMonth {
    /// Month index.
    pub month: i32,
    /// Trades.
    pub trades: u32,
    /// Net, paise.
    pub net: i128,
}

/// A chain: each window's `(year, pick)`, its unseen months, and its trades
/// joined in order.
pub type Chain = (Vec<(i32, Option<u32>)>, Vec<ChainMonth>, Series);

/// The headline chain at the series level: each window's pick, its unseen
/// months' trades and nets in order (the pick's own figures; see the
/// report's ambiguity on re-running the account).
pub fn chain(cands: &[Series], r: &Readings) -> Chain {
    let mut picks = Vec::new();
    let mut months = Vec::new();
    let mut joined = Series {
        id: u32::MAX,
        trades: Vec::new(),
    };
    for w in windows() {
        let p = pick(cands, &w, r);
        picks.push((w.year, p));
        let s = p.and_then(|id| cands.iter().find(|c| c.id == id));
        for m in w.unseen_first..=w.unseen_last {
            let ts: Vec<SeriesTrade> = s
                .map(|s| s.trades.iter().copied().filter(|t| t.month == m).collect())
                .unwrap_or_default();
            let trades = u32::try_from(ts.len()).unwrap_or(u32::MAX);
            let net: i128 = ts.iter().map(|t| t.net).sum::<i128>()
                - if trades > 0 {
                    i128::from(DATA_CHARGE)
                } else {
                    0
                };
            months.push(ChainMonth {
                month: m,
                trades,
                net,
            });
            joined.trades.extend(ts);
        }
    }
    (picks, months, joined)
}

/// The high-probability gate (R4, 24.P1 Fix 7).
pub fn clears(wins: u32, losses: u32, zsq: u32) -> bool {
    if wins <= losses {
        return false;
    }
    let d = u128::from(wins - losses);
    let n = u128::from(u64::from(wins) + u64::from(losses));
    d * d * 1000 >= u128::from(zsq) * n
}

/// The gate's three seeds (z = 1.645, 2.326, 3.09).
pub const ZSQ: [u32; 3] = [2_706, 5_412, 9_548];

/// The luck test's clearing rule (14.3).
pub fn luck_clears(o: u32, best: &[u32]) -> bool {
    let at_or_above = best.iter().filter(|&&b| b >= o).count();
    (1 + at_or_above) * 20 <= 1001
}

/// Line 2's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Every rule passed.
    Yes,
    /// A rule failed, named.
    No(&'static str),
    /// Waiting, named.
    Pending(&'static str),
}

impl Verdict {
    /// Line 2's word and its reason.
    pub fn line(self) -> String {
        match self {
            Verdict::Yes => "YES".to_owned(),
            Verdict::No(why) => format!("NO ({why})"),
            Verdict::Pending(why) => format!("PENDING ({why})"),
        }
    }
}

/// A twin seed's GLOBAL chain, as the verdict reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Twin {
    /// Its key on the 82 unseen months.
    pub key: Key,
    /// Its whole pipeline ran (else a partial null).
    pub whole: bool,
}

/// The verdict's inputs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerdictIn {
    /// The headline chain's unseen months, or `None` (no chain formed).
    pub months: Option<Vec<ChainMonth>>,
    /// The headline chain's key on its unseen months.
    pub key: Key,
    /// The luck test cleared.
    pub luck: bool,
    /// Twin seeds 1, 2, 3.
    pub twins: [Option<Twin>; 3],
    /// An interim report.
    pub interim: bool,
}

/// The verdict function, the only writer of line 2.
pub fn verdict(v: &VerdictIn, r: &Readings) -> Verdict {
    let core = verdict_core(v);
    if !v.interim {
        return core;
    }
    match (r.interim, core) {
        (InterimLine::AlwaysPending, _)
        | (InterimLine::NoCanPrint, Verdict::Yes | Verdict::Pending(_)) => {
            Verdict::Pending("interim")
        }
        (InterimLine::NoCanPrint, no) => no,
    }
}

fn verdict_core(v: &VerdictIn) -> Verdict {
    let Some(months) = &v.months else {
        return Verdict::No("no headline");
    };
    if months
        .iter()
        .any(|m| m.trades == 0 || m.net < i128::from(BAR))
    {
        return Verdict::No("a month below the bar");
    }
    if !v.luck {
        return Verdict::No("the sign test does not clear");
    }
    let at_or_above = |t: &Twin| cmp_ranking(&t.key, &v.key) != Ordering::Greater;
    if let Some(t1) = &v.twins[0]
        && at_or_above(t1)
    {
        return Verdict::No("twin seed 1 at or above the headline");
    }
    if v.twins.iter().flatten().filter(|t| t.whole).count() < 3 {
        return Verdict::Pending("fewer than three twin seeds have run");
    }
    if v.twins[1..].iter().flatten().any(at_or_above) {
        return Verdict::No("twin seed 2 or 3 at or above the headline");
    }
    Verdict::Yes
}

/// Whole option lots at or under a premium amount, rounding down (25.2.5):
/// `floor(amount / (lot x premium))`.
///
/// # Errors
/// `lot_above_premium` when one lot costs more than the amount.
pub fn option_lots(amount: i64, lot: i64, premium: i64) -> Result<i64, crate::model::Status> {
    let one = i128::from(lot.max(1)) * i128::from(premium.max(1));
    let lots = i128::from(amount) / one;
    if lots == 0 {
        return Err(crate::model::Status::LotAbovePremium);
    }
    Ok(i64::try_from(lots).unwrap_or(i64::MAX))
}

#[cfg(test)]
#[path = "wf_tests.rs"]
mod tests;
