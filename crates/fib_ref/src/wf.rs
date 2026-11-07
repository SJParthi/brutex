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
mod tests {
    use super::*;
    use crate::model::Status;

    fn t(month: i32, net: i128, hold: u32) -> SeriesTrade {
        SeriesTrade { month, net, hold }
    }

    const BAR_PLUS: i128 = 2_600_000_000 / 100; // Rs 2,60,000

    #[test]
    fn the_gate_matches_the_kits_boundaries() {
        assert!(clears(3, 0, 2_706));
        assert!(!clears(2, 0, 2_706));
        assert!(clears(6, 0, 5_412));
        assert!(!clears(5, 0, 5_412));
        assert!(clears(10, 0, 9_548));
        assert!(!clears(9, 0, 9_548));
        assert!(clears(677_853, 675_147, 5_412));
        assert!(!clears(677_852, 675_148, 5_412));
        assert!(!clears(0, 0, 0));
        assert!(!clears(5, 5, 0));
        assert!(!clears(4, 5, 0));
        assert!(clears(u32::MAX, u32::MAX - 1, 0));
        assert!(!clears(u32::MAX, u32::MAX - 1, 1));
        assert_eq!(ZSQ, [2_706, 5_412, 9_548]);
    }

    #[test]
    fn the_luck_rule_clears_at_49_or_fewer_at_or_above() {
        let best: Vec<u32> = (0..1000).map(|b| if b < 49 { 80 } else { 10 }).collect();
        assert!(luck_clears(70, &best)); // 49 at or above: 50 x 20 = 1000
        let best50: Vec<u32> = (0..1000).map(|b| if b < 50 { 80 } else { 10 }).collect();
        assert!(!luck_clears(70, &best50)); // 51 x 20 = 1020
        assert!(luck_clears(81, &best50));
        assert!(luck_clears(0, &[]));
        assert!(!luck_clears(0, &[0; 1000]));
    }

    #[test]
    fn passers_rank_by_drawdown_then_net() {
        let pass = |id, nets: &[i128]| Series {
            id,
            trades: nets
                .iter()
                .enumerate()
                .map(|(m, &n)| t(i32::try_from(m).unwrap_or(0), n, 60))
                .collect(),
        };
        let a = pass(1, &[BAR_PLUS + 58_882, BAR_PLUS + 58_882]);
        let b = pass(2, &[BAR_PLUS + 10_058_882, BAR_PLUS + 58_882]);
        let r = Readings::HEADLINE;
        let ka = key(&a, 0, 1, &r);
        let kb = key(&b, 0, 1, &r);
        assert!(ka.passer && kb.passer);
        assert_eq!(ka.at_bar, 2);
        // both have no drawdown beyond the data charge: equal tenths, more net wins
        assert_eq!(cmp_ranking(&kb, &ka), Ordering::Less);
        let miss = pass(3, &[BAR_PLUS * 10]);
        let km = key(&miss, 0, 1, &r); // month 1 has no trade
        assert!(!km.passer);
        assert_eq!(cmp_ranking(&ka, &km), Ordering::Less);
        assert_eq!(cmp_ranking(&km, &ka), Ordering::Greater);
    }

    #[test]
    fn misses_rank_by_shortfall_then_worst_month() {
        let r = Readings::HEADLINE;
        let s = |id, trades: Vec<SeriesTrade>| Series { id, trades };
        let a = key(&s(1, vec![t(0, 100_000, 10), t(1, 100_000, 10)]), 0, 1, &r);
        let b = key(&s(2, vec![t(0, 200_000, 10)]), 0, 1, &r);
        // shortfall: a owes (bar - 41,118) x 2, b owes (bar - 141,118) + bar
        assert_eq!(a.shortfall, 2 * (i128::from(BAR) - 100_000 + 58_882));
        assert_eq!(
            b.shortfall,
            i128::from(BAR) - 200_000 + 58_882 + i128::from(BAR)
        );
        // b netted more in all (one data charge, not two): its shortfall is less
        assert_eq!(cmp_ranking(&b, &a), Ordering::Less);
        // equal shortfall: the higher worst month wins
        let c = key(&s(3, vec![t(0, 150_000, 10), t(1, 50_000, 10)]), 0, 1, &r);
        assert_eq!(c.shortfall, a.shortfall);
        assert_eq!(cmp_ranking(&a, &c), Ordering::Less);
        // an empty range is no passer
        let e = key(&s(4, vec![]), 5, 4, &r);
        assert!(!e.passer);
        assert_eq!(e.months, 0);
    }

    #[test]
    fn keys_four_to_eight_break_ties_in_order() {
        let base = Key {
            passer: true,
            dd_tenths: 0,
            dd: 0,
            net: 100,
            wins: 1,
            trades: 2,
            hold: 100,
            shortfall: 0,
            worst: 0,
            at_bar: 1,
            months: 1,
            id: 1,
        };
        let rate = Key {
            wins: 2,
            id: 2,
            ..base
        };
        assert_eq!(cmp_ranking(&rate, &base), Ordering::Less);
        let none = Key {
            wins: 0,
            trades: 0,
            hold: 0,
            id: 3,
            ..base
        };
        assert_eq!(cmp_ranking(&base, &none), Ordering::Less);
        assert_eq!(cmp_ranking(&none, &base), Ordering::Greater);
        let none2 = Key { id: 4, ..none };
        assert_eq!(cmp_ranking(&none, &none2), Ordering::Less); // by id
        let fewer = Key {
            wins: 1,
            trades: 2,
            id: 5,
            ..base
        };
        let more = Key {
            wins: 2,
            trades: 4,
            id: 6,
            ..base
        };
        assert_eq!(cmp_ranking(&fewer, &more), Ordering::Less); // same rate, fewer trades
        let short = Key {
            hold: 50,
            id: 7,
            ..base
        };
        assert_eq!(cmp_ranking(&short, &base), Ordering::Less);
        let per = Key {
            net: 100,
            trades: 2,
            hold: 100,
            id: 8,
            ..base
        };
        assert_eq!(cmp_ranking(&base, &per), Ordering::Less); // all equal but id
        let dd = Key {
            dd_tenths: 1,
            id: 0,
            ..base
        };
        assert_eq!(cmp_ranking(&base, &dd), Ordering::Less);
        let rich = Key {
            net: 200,
            id: 9,
            ..base
        };
        assert_eq!(cmp_ranking(&rich, &base), Ordering::Less);
        // net per trade decides between equal-net... it cannot differ with equal net and trades
        let _ = per;
    }

    #[test]
    fn windows_and_the_chain() {
        let w = windows();
        assert_eq!(w.len(), 7);
        assert_eq!(crate::clock::month_label(w[0].learn_first), "2018-09");
        assert_eq!(crate::clock::month_label(w[0].learn_last), "2019-12");
        assert_eq!(crate::clock::month_label(w[6].unseen_last), "2026-10");
        let unseen: i32 = w.iter().map(|x| x.unseen_last - x.unseen_first + 1).sum();
        assert_eq!(unseen, 82);
        // two candidates: A good before 2022, B after
        let mut a = Series {
            id: 1,
            trades: vec![],
        };
        let mut b = Series {
            id: 2,
            trades: vec![],
        };
        for m in w[0].learn_first..=w[6].unseen_last {
            let early = m < 2022 * 12;
            a.trades
                .push(t(m, if early { 40_000_000 } else { -100_000 }, 30));
            b.trades
                .push(t(m, if early { 1_000 } else { 40_000_000 }, 30));
        }
        let r = Readings::HEADLINE;
        let (picks, months, joined) = chain(&[a, b], &r);
        assert_eq!(picks[0], (2020, Some(1)));
        assert_eq!(picks.len(), 7);
        assert_eq!(months.len(), 82);
        assert_eq!(joined.trades.len(), 82);
        assert!(pick(&[], &w[0], &r).is_none());
        let (p, m, _) = chain(&[], &r);
        assert!(p.iter().all(|x| x.1.is_none()));
        assert!(m.iter().all(|x| x.trades == 0 && x.net == 0));
    }

    fn good_key() -> Key {
        Key {
            passer: true,
            dd_tenths: 0,
            dd: 0,
            net: 1,
            wins: 1,
            trades: 1,
            hold: 1,
            shortfall: 0,
            worst: 0,
            at_bar: 82,
            months: 82,
            id: 1,
        }
    }

    #[test]
    fn the_verdict_rules_in_order() {
        let r = Readings::HEADLINE;
        let months: Vec<ChainMonth> = (0..82)
            .map(|m| ChainMonth {
                month: m,
                trades: 1,
                net: i128::from(BAR),
            })
            .collect();
        let below = Twin {
            key: Key {
                passer: false,
                id: 9,
                ..good_key()
            },
            whole: true,
        };
        let above = Twin {
            key: Key {
                id: 0,
                ..good_key()
            },
            whole: true,
        };
        let mut v = VerdictIn {
            months: None,
            key: good_key(),
            luck: true,
            twins: [None; 3],
            interim: false,
        };
        assert_eq!(verdict(&v, &r), Verdict::No("no headline"));
        let mut low = months.clone();
        low[40].net -= 1;
        v.months = Some(low);
        assert_eq!(verdict(&v, &r), Verdict::No("a month below the bar"));
        let mut empty = months.clone();
        empty[3].trades = 0;
        v.months = Some(empty);
        assert_eq!(verdict(&v, &r), Verdict::No("a month below the bar"));
        v.months = Some(months);
        v.luck = false;
        assert_eq!(verdict(&v, &r), Verdict::No("the sign test does not clear"));
        v.luck = true;
        assert_eq!(
            verdict(&v, &r),
            Verdict::Pending("fewer than three twin seeds have run")
        );
        v.twins[0] = Some(above);
        assert_eq!(
            verdict(&v, &r),
            Verdict::No("twin seed 1 at or above the headline")
        );
        v.twins[0] = Some(Twin {
            whole: false,
            ..above
        });
        assert_eq!(
            verdict(&v, &r),
            Verdict::No("twin seed 1 at or above the headline")
        );
        v.twins[0] = Some(Twin {
            whole: false,
            ..below
        });
        assert_eq!(
            verdict(&v, &r),
            Verdict::Pending("fewer than three twin seeds have run")
        );
        v.twins = [Some(below), Some(below), Some(below)];
        assert_eq!(verdict(&v, &r), Verdict::Yes);
        v.twins = [Some(below), Some(above), Some(below)];
        assert_eq!(
            verdict(&v, &r),
            Verdict::No("twin seed 2 or 3 at or above the headline")
        );
        v.twins = [Some(below), Some(above), None];
        assert_eq!(
            verdict(&v, &r),
            Verdict::Pending("fewer than three twin seeds have run")
        );
        // interim: always pending, or a NO may print
        v.twins = [Some(below), Some(below), Some(below)];
        v.interim = true;
        assert_eq!(verdict(&v, &r), Verdict::Pending("interim"));
        let mut r2 = r;
        r2.interim = InterimLine::NoCanPrint;
        assert_eq!(verdict(&v, &r2), Verdict::Pending("interim"));
        v.luck = false;
        assert_eq!(
            verdict(&v, &r2),
            Verdict::No("the sign test does not clear")
        );
        assert_eq!(verdict(&v, &r), Verdict::Pending("interim"));
        assert_eq!(Verdict::Yes.line(), "YES");
        assert_eq!(Verdict::No("x").line(), "NO (x)");
        assert_eq!(Verdict::Pending("y").line(), "PENDING (y)");
    }

    #[test]
    fn whole_lots_round_down() {
        // Rs 2 lakh premium, lot 650 at Rs 30.00: 19,500 a lot: 10 lots (Rs 1,95,000)
        assert_eq!(option_lots(20_000_000, 650, 3_000), Ok(10));
        assert_eq!(option_lots(19_499_999, 650, 3_000), Ok(9));
        assert_eq!(option_lots(1_950_000, 650, 3_000), Ok(1));
        assert_eq!(
            option_lots(1_949_999, 650, 3_000),
            Err(Status::LotAbovePremium)
        );
        assert_eq!(option_lots(3_000_000, 650, 3_000), Ok(1)); // Rs 30,000 seed: one lot
        assert_eq!(option_lots(0, 1, 1), Err(Status::LotAbovePremium));
        assert_eq!(option_lots(5, 0, 0), Ok(5));
    }
}
