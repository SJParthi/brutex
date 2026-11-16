//! v3.2 report pieces (MAIN-RULINGS-ENGINE-v22 addenda 8 / 8b, 10, 12 / 12b / 12c, 13 / 13b, 14, 15, 16): every trade's v3.2 facts
//! (`Row32`), each pick's v3.2 tables (orders and charges under THE NET DEFINITION, liquidity and unfilled exits, the expiry split, the Greek
//! bands, the month and week verdicts), the s26 portfolio layer over the verdict view's accounts, and the dated expiry calendar command.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::cal::{fmt_day, fmt_rs};

/// One trade's v3.2 facts (the trade row's v3.2 columns and every v3.2 table read these).
#[derive(Clone, Debug, Default)]
pub struct Row32 {
    pub day: i64,
    /// The decision instant T (us) and the instant the trade is known (us).
    pub t_us: i64,
    pub known: i64,
    pub net: i64,
    /// The margin blocked at entry (lots x margin per lot + the order's Rs 20), paisa.
    pub margin: i64,
    /// Addendum 10: (sale-side, own buy-side, Dhan square-off) executed orders and the slice (units, 0 = no freeze limit known).
    pub orders: (u32, u32, u32),
    pub slice: u64,
    pub brokerage_gst: i64,
    /// Addendum 14 (d): entry time-to-fill (s, from T to the sale's last fill known), exit time-to-fill (s, from the exit decision to the
    /// last buy-back / square-off fill known; None = nothing to buy back), % of the wanted units sold (bp), prints in the last 60 s, longest
    /// silence (s), exit re-sends (blocks of the own exit before its last fill), never-closed units.
    pub entry_ttf: i64,
    pub exit_ttf: Option<i64>,
    pub filled_bp: i64,
    pub prints60: u32,
    pub gap60: u32,
    pub resends: u32,
    pub nc_units: u64,
    /// Addendum 16: sessions to expiry, its bucket, monthly, moved by a holiday, expiry day.
    pub dte: i64,
    pub bucket: usize,
    pub monthly: bool,
    pub moved: bool,
    /// Addenda 8 / 8b: entry IV and |delta| (None: refused / not computed).
    pub iv: Option<f64>,
    pub delta: Option<f64>,
    pub greeks_ns: u64,
    pub sl_exit: bool,
    /// REPAIR A (check-engine-2 M1 / S7): the segment, the units sold, the value sold (paisa) and 1 + the offset of the contract-day's minute
    /// closes in the segment's `Tab3::marks` (0 = none): the running MTM loss per minute.
    pub seg: u8,
    pub units: u64,
    pub sold: i64,
    pub mark1: u32,
    /// REPAIR A (addendum 19): the trade's (day, week, month) are read from `day`.
    pub expiry_day: bool,
    /// REPAIR A (check-engine-1 M1): the charges in the net (dated rates under CHG_DATED) and the dearer-of reading beside (paisa).
    pub charges: i64,
    pub charges_dearer: i64,
    /// REPAIR A (check-engine-1 M2): the liquidity dial value of the pick's list (0 = the gate OFF).
    pub liq: u8,
}

/// REPAIR A (check-engine-2 M1): one member trade's mark data for the portfolio's MTM rule: units sold, value sold (paisa), 1 + the offset of
/// the contract-day's minute closes in the segment's `Tab3::marks` (0 = no marks: no loss counted, as the kernel).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PMark {
    pub units: u64,
    pub sold: i64,
    pub mark1: u32,
}

/// The kernel's mark of a position at decision instant `t_us` (kern::unrealised): the option's last 1-minute close at or before the decision
/// candle's end (minute slot (T - 09:15) / 60 - 1, forward filled in the table); 0 = no mark.
#[must_use]
pub fn mark_at(marks: &[i32], mark1: u32, t_us: i64) -> i64 {
    if mark1 == 0 {
        return 0;
    }
    let b = (crate::cal::sod_of(t_us) - crate::cal::OPEN_S) / 60;
    if b < 1 {
        return 0;
    }
    let b = usize::try_from(b - 1).unwrap_or(0).min(crate::cal::NM - 1);
    marks.get(mark1 as usize - 1 + b).copied().map_or(0, i64::from)
}

/// The mark of a position in minute slot `b` (the close of that minute, forward filled); 0 = no mark.
#[must_use]
pub fn mark_in_minute(marks: &[i32], mark1: u32, b: usize) -> i64 {
    if mark1 == 0 {
        return 0;
    }
    marks.get(mark1 as usize - 1 + b.min(crate::cal::NM - 1)).copied().map_or(0, i64::from)
}

/// The running MTM LOSS of one short position at mark `mark` (losses only): max(0, mark x units - value sold).
#[inline]
#[must_use]
pub fn loss_at(mark: i64, units: u64, sold: i64) -> i64 {
    if mark <= 0 {
        return 0;
    }
    (mark * i64::try_from(units).unwrap_or(0) - sold).max(0)
}

/// The |delta| bands of addendum 8 (count, net, win rate per band).
pub const DELTA_BANDS: [&str; 6] = ["0.00-0.10", "0.10-0.25", "0.25-0.40", "0.40-0.55", "0.55+", "not solved"];
/// The IV bands.
pub const IV_BANDS: [&str; 6] = ["< 20%", "20-40%", "40-80%", "80-150%", "150%+", "not solved"];

/// The band of an |delta|.
#[must_use]
pub fn delta_band(d: Option<f64>) -> usize {
    match d.map(f64::abs) {
        None => 5,
        Some(x) if x < 0.10 => 0,
        Some(x) if x < 0.25 => 1,
        Some(x) if x < 0.40 => 2,
        Some(x) if x < 0.55 => 3,
        Some(_) => 4,
    }
}

/// The band of an IV (decimal).
#[must_use]
pub fn iv_band(v: Option<f64>) -> usize {
    match v {
        None => 5,
        Some(x) if x < 0.20 => 0,
        Some(x) if x < 0.40 => 1,
        Some(x) if x < 0.80 => 2,
        Some(x) if x < 1.50 => 3,
        Some(_) => 4,
    }
}

/// "+384.21% (Rs 19.00 -> Rs 92.00, source)" or "-" when a price is missing.
#[must_use]
pub fn gain_words(from: i64, to: i64, src: &str) -> String {
    if from <= 0 || to <= 0 {
        return format!("- ({src})");
    }
    let bp = (to - from) * 10_000 / from;
    format!("{}{}.{:02}% (Rs {} -> Rs {}, {src})", if bp < 0 { "-" } else { "+" }, bp.abs() / 100, bp.abs() % 100, fmt_rs(from), fmt_rs(to))
}

/// REPAIR A (check-engine-1 S4): a ratio in basis points rounded HALF UP (half away from zero), not truncated.
#[must_use]
pub fn bp_round(num: i64, den: i64) -> i64 {
    if den == 0 {
        return 0;
    }
    // MUTANT m_v32r_pct_truncate: the old truncation
    if cfg!(feature = "m_v32r_pct_truncate") {
        return num * 10_000 / den;
    }
    let n = i128::from(num) * 10_000;
    let d = i128::from(den);
    let q = (2 * n.abs() + d.abs()) / (2 * d.abs());
    i64::try_from(if (n < 0) != (d < 0) { -q } else { q }).unwrap_or(0)
}

/// REPAIR A (check-engine-1 S4): [`gain_words`] with the percentage rounded half up to 2 decimals.
#[must_use]
pub fn gain_words_r(from: i64, to: i64, src: &str) -> String {
    if from <= 0 || to <= 0 {
        return format!("- ({src})");
    }
    let bp = bp_round(to - from, from);
    format!("{}{}.{:02}% (Rs {} -> Rs {}, {src})", if bp < 0 { "-" } else { "+" }, bp.abs() / 100, bp.abs() % 100, fmt_rs(from), fmt_rs(to))
}

/// REPAIR A (check-engine-1 M3, addendum 12): HOW an exit rule's level was set, in rupees, from the rule's own distance at the trigger and the
/// level it crossed: e.g. "stop = R(s) Rs 18.90 + Rs 0.95 (SL 5% of the first fill price Rs 18.90, rounded up) = Rs 19.85", where R(s) =
/// the worse (lower, for a sale) of the first fill price and the running average sold price at that second.
#[must_use]
pub fn level_words(xr: Option<&common_fill::xrule::XRule>, level: i64, p0: i64, rule_words: &str) -> String {
    use common_fill::xrule::XRule as X;
    let rs = |v: i64| format!("Rs {}", fmt_rs(v));
    let base = format!("R(s) = the worse (lower) of the first fill price Rs {} and the running average sold price", fmt_rs(p0));
    match xr {
        Some(X::Stop { x } | X::QuickCut { x }) => format!("stop = R(s) {} + {} ({rule_words}; first fill Rs {}) = {} ({base})", rs(level - x), rs(*x), fmt_rs(p0), rs(level)),
        Some(X::Target { x }) => format!("target = R(s) {} - {} ({rule_words}; first fill Rs {}) = {} ({base})", rs(level + x), rs(*x), fmt_rs(p0), rs(level)),
        Some(X::Trail { act, dist }) => format!("trailing stop: woke once the price fell {} below R(s); level = the lowest price since then {} + {} = {} ({rule_words})", rs(*act), rs(level - dist), rs(*dist), rs(level)),
        Some(X::TrailPct { act, bp }) => format!("trailing stop: woke once the price fell {} below R(s); level = the lowest price since then + {}.{:02}% of it (at least 1 paisa) = {} ({rule_words})", rs(*act), bp / 100, bp % 100, rs(level)),
        Some(X::ArmedStop { act, off }) => format!("breakeven stop: armed once the price fell {} below R(s); stop = R(s) {} {} {} = {} ({rule_words}; {base})", rs(*act), rs(level - off), if *off < 0 { "-" } else { "+" }, rs(off.abs()), rs(level)),
        Some(X::TimedStop { after, off }) => format!("timed stop: from {after} s after the first fill block; stop = R(s) {} {} {} = {} ({rule_words}; {base})", rs(level - off), if *off < 0 { "-" } else { "+" }, rs(off.abs()), rs(level)),
        Some(X::StopAt { level: lv }) => format!("stop at the fixed level {} ({rule_words})", rs(*lv)),
        Some(X::TargetAt { level: lv }) => format!("target at the fixed level {} ({rule_words})", rs(*lv)),
        Some(X::GoPositive { secs }) => format!("go-positive time stop: {secs} s after the first fill block the price was not below R(s) ({rule_words})"),
        Some(X::TimeExit { secs }) => format!("time limit: {secs} s after the first fill block ({rule_words})"),
        None => format!("level {} ({rule_words})", rs(level)),
    }
}

fn pct_bp(a: i64, b: i64) -> String {
    if b == 0 {
        return "-".to_owned();
    }
    let x = a * 10_000 / b;
    format!("{}.{:02}%", x / 100, (x % 100).abs())
}

/// The ISO-like week id of a day (Monday-based weeks since 1970-01-05).
#[must_use]
pub const fn week_of(d: i64) -> i64 {
    (d - 4).div_euclid(7)
}

/// The month id of a day (year x 12 + month - 1).
#[must_use]
pub fn month_id(d: i64) -> i64 {
    let (y, m, _) = crate::cal::civil(d);
    y * 12 + m - 1
}

/// "YYYY-MM" of a month id.
#[must_use]
pub fn month_name(id: i64) -> String {
    format!("{}-{:02}", id.div_euclid(12), id.rem_euclid(12) + 1)
}

/// ADDENDUM 17: the margin utilization of a set of trades over the run's sessions (09:15-15:30 = 375 trading minutes a session): per minute
/// the margin in use = the margins of the positions open in it (blocked from the decision T to the last fill known); (peak, median, share of
/// minutes above 80% of Rs 20 L in bp, idle share in bp) over every minute of every session of the run. `trades` = (T us, known us, margin).
#[must_use]
pub fn utilization(trades: &[(i64, i64, i64)], days: &[i64]) -> (i64, i64, i64, i64) {
    let mut per: Vec<i64> = Vec::with_capacity(days.len() * 375);
    for &d in days {
        let mut v = vec![0_i64; 375];
        for &(t, k, m) in trades {
            let (a, b) = (crate::cal::at_sod(d, crate::cal::OPEN_S), crate::cal::at_sod(d, crate::cal::CLOSE_S));
            if k <= a || t >= b {
                continue;
            }
            let i0 = usize::try_from(((t.max(a) - a) / (60 * crate::cal::US_S)).max(0)).unwrap_or(0);
            let i1 = usize::try_from(((k.min(b) - a + 60 * crate::cal::US_S - 1) / (60 * crate::cal::US_S)).max(0)).unwrap_or(0).min(375);
            for x in v.iter_mut().take(i1).skip(i0) {
                *x += m;
            }
        }
        per.extend(v);
    }
    if per.is_empty() {
        return (0, 0, 0, 0);
    }
    let n = i64::try_from(per.len()).unwrap_or(1);
    let hi = i64::try_from(per.iter().filter(|&&x| x * 10 > crate::trade::ACCOUNT * 8).count()).unwrap_or(0);
    let idle = i64::try_from(per.iter().filter(|&&x| x == 0).count()).unwrap_or(0);
    let peak = per.iter().copied().max().unwrap_or(0);
    let mut s = per.clone();
    s.sort_unstable();
    (peak, s.get(s.len() / 2).copied().unwrap_or(0), hi * 10_000 / n, idle * 10_000 / n)
}

/// REPAIR A (check-engine-2 S7, addendum 17): the per-minute MARGIN + RUNNING MTM LOSS of a set of trades over the run's sessions (09:15-15:30,
/// 375 minutes a session): per minute b the margins of the positions open in it (from T to the last fill known) plus each one's loss at the
/// close of minute b (the kernel's mark, forward filled). Returns (peak Rs, its session and minute 09:15 + b, minutes above Rs 20 L, minutes).
#[must_use]
pub fn margin_mtm_peak(trades: &[(i64, i64, i64, PMark)], marks: &[i32], days: &[i64]) -> (i64, i64, usize, i64, i64) {
    let (mut peak, mut pk_day, mut pk_min, mut above, mut n) = (0_i64, 0_i64, 0_usize, 0_i64, 0_i64);
    for &d in days {
        let mut v = vec![0_i64; 375];
        for &(t, k, m, pm) in trades {
            let (a, b) = (crate::cal::at_sod(d, crate::cal::OPEN_S), crate::cal::at_sod(d, crate::cal::CLOSE_S));
            if k <= a || t >= b {
                continue;
            }
            let i0 = usize::try_from(((t.max(a) - a) / (60 * crate::cal::US_S)).max(0)).unwrap_or(0);
            let i1 = usize::try_from(((k.min(b) - a + 60 * crate::cal::US_S - 1) / (60 * crate::cal::US_S)).max(0)).unwrap_or(0).min(375);
            for (bi, x) in v.iter_mut().enumerate().take(i1).skip(i0) {
                *x += m + loss_at(mark_in_minute(marks, pm.mark1, bi), pm.units, pm.sold);
            }
        }
        for (bi, &x) in v.iter().enumerate() {
            n += 1;
            if x > crate::trade::ACCOUNT {
                above += 1;
            }
            if x > peak {
                peak = x;
                pk_day = d;
                pk_min = bi;
            }
        }
    }
    (peak, pk_day, pk_min, above, n)
}

/// The words of a (margin + MTM) peak.
#[must_use]
pub fn mtm_peak_words(p: (i64, i64, usize, i64, i64)) -> String {
    let (peak, d, b, above, n) = p;
    if peak == 0 {
        return "per-minute margin + running MTM loss: no position open".to_owned();
    }
    format!("per-minute margin + running MTM loss (minute-close marks, the kernel's rule): peak Rs {} ({}) at {} {}; minutes above Rs 20,00,000: {above} of {n}{}", fmt_rs(peak), pct_bp(peak, crate::trade::ACCOUNT), fmt_day(d), crate::cal::fmt_sod(crate::cal::OPEN_S + 60 * i64::try_from(b).unwrap_or(0)), if above == 0 { " (never above the cap)" } else { " (ABOVE the cap between sales: no new sale was allowed then; Dhan's RMS would act: matrix X20)" })
}

/// REPAIR A (check-engine-2 M1): the s26 portfolio layer with the kernel's MTM rule at every portfolio sale: as [`portfolio`], and a sale is
/// refused when the free money (Rs 20 L + min(0, realized) - margin in use) less the running MTM loss of the portfolio's open positions at
/// the sale's decision instant (minute-close marks) cannot cover its margin (`margin_mtm_short` when the free money alone would; else the cash
/// rule). Returns the [`PRes`] and the MTM-only refusals.
#[must_use]
pub fn portfolio_mtm(members: &[&[(PTrade, PMark)]], marks: &[i32], months_all: &[i64]) -> (PRes, u32) {
    // v3.2 round B (ONE common runtime): the s26 portfolio and the side-by-side account are the SAME walk (`port_walk`); one segment here
    let w: Vec<Vec<WTrade>> = members.iter().map(|m| m.iter().map(|&(p, pm)| WTrade { p, seg: 0, pm }).collect()).collect();
    let refs: Vec<&[WTrade]> = w.iter().map(Vec::as_slice).collect();
    let r = port_walk(&refs, &[marks], months_all);
    if let Ok(mut g) = LAST_TAKEN_MTM.lock() {
        *g = r.taken.iter().map(|x| (x.0, x.1, x.2, x.3)).collect();
    }
    if let Ok(mut g) = LAST_TAKEN_NET.lock() {
        g.clone_from(&r.taken_net);
    }
    ((r.net, r.trades, r.cap + r.cash, r.ruled, r.dd, r.mo), r.mtm)
}

/// v3.2 round B (ONE common runtime, addenda 15 / 17): one member trade of the one-account walk: the s26 trade, its segment (0 index, 1
/// stock: the index into the walk's mark tables), its mark reading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WTrade {
    pub p: PTrade,
    pub seg: u8,
    pub pm: PMark,
}

/// The one-account walk's outcome.
#[derive(Clone, Debug, Default)]
pub struct WalkRes {
    pub net: i64,
    pub trades: u32,
    pub by_seg: [u32; 2],
    /// Refused: the Rs 20 L cap, the cash rule, the running MTM loss alone, the one-account rules.
    pub cap: u32,
    pub cash: u32,
    pub mtm: u32,
    pub ruled: u32,
    pub dd: i64,
    pub mo: BTreeMap<i64, i64>,
    /// The trades taken: (entry us, known us, margin, mark, segment).
    pub taken: Vec<(i64, i64, i64, PMark, u8)>,
    /// v3.3 fix CAP_MIN: the net and day of each taken trade (in `taken` order).
    pub taken_net: Vec<(i64, i64)>,
    /// The largest margin in use + running MTM loss seen at a sale instant (after the sale).
    pub peak_mtm: i64,
    /// Per member trade in the walk's order: (member, trade index in the member, outcome: "taken", "cap_20L", "cash_rule", "mtm_loss",
    /// "one_account_rule").
    /// The realized net per session (the day of each closed trade).
    pub day_net: BTreeMap<i64, i64>,
    pub log: Vec<(usize, usize, &'static str)>,
}

/// v3.2 round B: THE one-account walk of the s26 portfolio AND the side-by-side account (addendum 17): the members' trades in time order
/// (the trade known first, then the segment, then the member order; a stable sort), and at each sale the kernel's rules in its order: the
/// same contract or another strike of the same underlying + side + expiry open IN THE SAME SEGMENT (refused, ruled; keys of different
/// segments are different contracts), margin in use + this margin > Rs 20 L (refused, cap), the free money (Rs 20 L + min(0, realized) -
/// margin in use) less the RUNNING MTM LOSS of every open position at the sale's decision instant (each from its OWN segment's minute-close
/// marks, losses only) below this margin (refused: `mtm` when the free money alone would cover it, else the cash rule).
#[must_use]
pub fn port_walk(members: &[&[WTrade]], marks: &[&[i32]], months_all: &[i64]) -> WalkRes {
    let mut all: Vec<(usize, usize, WTrade)> = Vec::new();
    for (mi, m) in members.iter().enumerate() {
        all.extend(m.iter().enumerate().map(|(ti, t)| (mi, ti, *t)));
    }
    all.sort_by_key(|(mi, _, t)| (t.p.0, t.p.1, t.seg, *mi));
    let mut r = WalkRes { mo: months_all.iter().map(|&m| (m, 0)).collect(), ..WalkRes::default() };
    let mut open: Vec<WTrade> = Vec::new();
    let (mut realized, mut peak, mut in_use) = (0_i64, 0_i64, 0_i64);
    let mark_of = |x: &WTrade, t: i64| -> i64 { marks.get(usize::from(x.seg)).map_or(0, |mk| loss_at(mark_at(mk, x.pm.mark1, t), x.pm.units, x.pm.sold)) };
    for (mi, ti, t) in all {
        open.sort_by_key(|x| x.p.1);
        while let Some(x) = open.first().copied() {
            if x.p.1 > t.p.0 {
                break;
            }
            open.remove(0);
            realized += x.p.3;
            in_use -= x.p.2;
            peak = peak.max(realized);
            r.dd = r.dd.max(peak - realized);
            *r.mo.entry(month_id(x.p.4)).or_insert(0) += x.p.3;
            *r.day_net.entry(x.p.4).or_insert(0) += x.p.3;
        }
        // MUTANT m_v32rb_port_segment_blind: keys of different segments compared as one segment's
        let same_seg = |x: &WTrade| cfg!(feature = "m_v32rb_port_segment_blind") || x.seg == t.seg;
        if open.iter().any(|x| same_seg(x) && (x.p.5 == t.p.5 || x.p.6 == t.p.6)) {
            r.ruled += 1;
            r.log.push((mi, ti, "one_account_rule"));
            continue;
        }
        let m = t.p.2;
        if in_use + m > crate::trade::ACCOUNT {
            r.cap += 1;
            r.log.push((mi, ti, "cap_20L"));
            continue;
        }
        let free0 = crate::trade::ACCOUNT + realized.min(0) - in_use;
        // MUTANT m_v32r_portfolio_no_mtm (repair A) / m_v32b_side_no_mtm (v3.2-B): the open positions' running loss left out of the free money
        let u: i64 = if cfg!(feature = "m_v32r_portfolio_no_mtm") || cfg!(feature = "m_v32b_side_no_mtm") { 0 } else { open.iter().map(|x| mark_of(x, t.p.0)).sum() };
        if free0 - u < m {
            if free0 >= m {
                r.mtm += 1;
                r.log.push((mi, ti, "mtm_loss"));
            } else {
                r.cash += 1;
                r.log.push((mi, ti, "cash_rule"));
            }
            continue;
        }
        open.push(t);
        in_use += m;
        r.peak_mtm = r.peak_mtm.max(in_use + u);
        r.trades += 1;
        if let Some(c) = r.by_seg.get_mut(usize::from(t.seg.min(1))) {
            *c += 1;
        }
        r.taken.push((t.p.0, t.p.1, m, t.pm, t.seg));
        r.taken_net.push((t.p.3, t.p.4));
        r.log.push((mi, ti, "taken"));
    }
    open.sort_by_key(|x| x.p.1);
    for x in open {
        realized += x.p.3;
        peak = peak.max(realized);
        r.dd = r.dd.max(peak - realized);
        *r.mo.entry(month_id(x.p.4)).or_insert(0) += x.p.3;
        *r.day_net.entry(x.p.4).or_insert(0) += x.p.3;
    }
    r.net = realized;
    r
}

/// The trades the last `portfolio_mtm` call took, with their marks.
pub static LAST_TAKEN_MTM: std::sync::Mutex<Vec<(i64, i64, i64, PMark)>> = std::sync::Mutex::new(Vec::new());
/// v3.3 fix CAP_MIN: the (net, day) of the trades the last `portfolio_mtm` call took (in LAST_TAKEN_MTM order).
pub static LAST_TAKEN_NET: std::sync::Mutex<Vec<(i64, i64)>> = std::sync::Mutex::new(Vec::new());

/// REPAIR A (addendum 19, check-engine-2 S1 / S9): the day / week / month verdict of a set of (day, net) trades against the RUN's sessions
/// (a session, week or month with no trade is net 0: not positive, failing): positive days / all, positive weeks / all, months >= Rs 2 L /
/// all, positive months / all, the worst day / week / month with their dates (a week by its Monday), and the failing weeks and months.
#[derive(Clone, Debug, Default)]
pub struct V19 {
    pub days: (usize, usize),
    pub weeks: (usize, usize),
    pub months_ok: (usize, usize),
    pub months_pos: usize,
    pub worst_day: (i64, i64),
    pub worst_week: (i64, i64),
    pub worst_month: (i64, i64),
    pub fail_weeks: Vec<(i64, i64)>,
    pub fail_months: Vec<(i64, i64)>,
}

/// The addendum 19 verdict of trades `(day, net)` over the run's sessions `days` (sorted, distinct).
#[must_use]
pub fn v19_of(trades: &[(i64, i64)], days: &[i64]) -> V19 {
    let mut dn: BTreeMap<i64, i64> = days.iter().map(|&d| (d, 0)).collect();
    for &(d, n) in trades {
        *dn.entry(d).or_insert(0) += n;
    }
    let mut wn: BTreeMap<i64, i64> = BTreeMap::new();
    let mut mn: BTreeMap<i64, i64> = BTreeMap::new();
    let traded: std::collections::BTreeSet<i64> = trades.iter().map(|t| t.0).collect();
    for (&d, &n) in &dn {
        // a week counts against the run's weeks (WK_RUN); its date is its Monday
        // MUTANT m_v32r_weeks_seen: a week without a trade dropped (h6's weeks-seen denominator)
        if !cfg!(feature = "m_v32r_weeks_seen") || traded.contains(&d) {
            *wn.entry(week_of(d)).or_insert(0) += n;
        }
        *mn.entry(month_id(d)).or_insert(0) += n;
    }
    let mut v = V19::default();
    v.days = (dn.values().filter(|&&n| n > 0).count(), dn.len());
    v.weeks = (wn.values().filter(|&&n| n > 0).count(), wn.len());
    v.months_ok = (mn.values().filter(|&&n| crate::kern::month_ok(n)).count(), mn.len());
    v.months_pos = mn.values().filter(|&&n| n > 0).count();
    v.worst_day = dn.iter().map(|(&d, &n)| (n, d)).min().map_or((0, 0), |(n, d)| (n, d));
    v.worst_week = wn.iter().map(|(&w, &n)| (n, w * 7 + 4)).min().map_or((0, 0), |(n, d)| (n, d));
    v.worst_month = mn.iter().map(|(&m, &n)| (n, m)).min().map_or((0, 0), |(n, m)| (n, m));
    v.fail_weeks = wn.iter().filter(|&(_, &n)| n <= 0).map(|(&w, &n)| (w * 7 + 4, n)).collect();
    v.fail_months = mn.iter().filter(|&(_, &n)| !crate::kern::month_ok(n)).map(|(&m, &n)| (m, n)).collect();
    v
}

/// REPAIR A (check-engine-2 M4): the walk-forward / train-test PASSED verdict of a row's TEST period under addenda 15 / 19: at least one test
/// trade, and EVERY test month >= Rs 2,00,000 net, EVERY test week positive and EVERY test session positive (against the run's sessions).
#[must_use]
pub fn a19_passed(v: &V19, trades: usize) -> bool {
    // MUTANT m_v32r_wf_year_rule: the month bar alone (weeks and sessions not read): the defect class of h6's year-rule verdict
    if cfg!(feature = "m_v32r_wf_year_rule") {
        return trades > 0 && v.months_ok.0 == v.months_ok.1;
    }
    trades > 0 && v.months_ok.1 > 0 && v.months_ok.0 == v.months_ok.1 && v.weeks.0 == v.weeks.1 && v.days.0 == v.days.1
}

/// The words of an addendum 19 verdict (counts and the worst day / week / month with dates).
#[must_use]
pub fn v19_words(v: &V19) -> String {
    format!(
        "positive days {} / {}; positive weeks {} / {}; months >= Rs 2,00,000 {} / {} (positive months {} / {}); worst day Rs {} on {}; worst week Rs {} (week of {}); worst month Rs {} ({})",
        v.days.0, v.days.1, v.weeks.0, v.weeks.1, v.months_ok.0, v.months_ok.1, v.months_pos, v.months_ok.1,
        fmt_rs(v.worst_day.0), if v.days.1 == 0 { "-".to_owned() } else { fmt_day(v.worst_day.1) },
        fmt_rs(v.worst_week.0), if v.weeks.1 == 0 { "-".to_owned() } else { fmt_day(v.worst_week.1) },
        fmt_rs(v.worst_month.0), if v.months_ok.1 == 0 { "-".to_owned() } else { month_name(v.worst_month.1) }
    )
}

/// The closest-miss words of an addendum 19 verdict: every failing month's rupee gap to Rs 2,00,000 and every non-positive week's net.
#[must_use]
pub fn v19_miss_words(v: &V19) -> String {
    let mut p: Vec<String> = v.fail_months.iter().map(|(m, n)| format!("{} net Rs {} short by Rs {}", month_name(*m), fmt_rs(*n), fmt_rs(crate::kern::MONTH_MIN - *n))).collect();
    let nw = v.fail_weeks.len();
    p.extend(v.fail_weeks.iter().take(12).map(|(d, n)| format!("week of {} net Rs {} (not positive: needs Rs {} more)", fmt_day(*d), fmt_rs(*n), fmt_rs(1 - *n))));
    if nw > 12 {
        p.push(format!("... {} more non-positive weeks", nw - 12));
    }
    if p.is_empty() { "every month passes and every week is positive".to_owned() } else { p.join("; ") }
}

/// One pick's v3.2 tables (markdown lines).
#[must_use]
pub fn pick_tables(name: &str, rows: &[(Row32, String)], months_all: &[i64]) -> String {
    pick_tables_m(name, rows, months_all, [&[], &[]])
}

/// One pick's v3.2 tables with the segments' minute marks (REPAIR A: the per-minute margin + MTM peak, addendum 19).
#[must_use]
pub fn pick_tables_m(name: &str, rows: &[(Row32, String)], months_all: &[i64], marks: [&[i32]; 2]) -> String {
    let mut s = String::new();
    let n = rows.len();
    let net: i64 = rows.iter().map(|r| r.0.net).sum();
    let _ = writeln!(s, "### {name}: v3.2 tables ({n} trades, net Rs {})", fmt_rs(net));
    let _ = writeln!(s);
    // addendum 10: orders
    let (so, bo, ko): (u64, u64, u64) = rows.iter().fold((0, 0, 0), |a, r| (a.0 + u64::from(r.0.orders.0), a.1 + u64::from(r.0.orders.1), a.2 + u64::from(r.0.orders.2)));
    let sliced = rows.iter().filter(|r| r.0.slice > 0 && r.0.orders.0 > 0).count();
    let _ = writeln!(s, "- Executed orders (addendum 10: every filling block x every NSE freeze slice): sale side {so}, own buy side {bo}, Dhan square-off {ko} = {} orders x Rs 20 + 18% GST; trades priced with a dated freeze slice {sliced} of {n}; brokerage + its GST in the net Rs {}.", so + bo + ko, fmt_rs(rows.iter().map(|r| r.0.brokerage_gst).sum()));
    if crate::sw::on(crate::sw::HL_STRICT) {
        // REPAIR A (check-engine-1 M2): every pick's liquidity gate in words (P / G / F); a headline never has the gate OFF
        let lq = rows.first().map_or(0, |r| usize::from(r.0.liq));
        let _ = writeln!(s, "- Liquidity gate of this pick (addendum 14 (b), a HARD gate: no HEADLINE pick with it OFF): P / G = {} ({}); F = the last-60-s volume at the 25% share covers the whole sale within {} s and the whole buy-back within {} s.", crate::read::LIQ_NAMES.get(lq).copied().unwrap_or("?"), crate::read::LIQ_SHORT.get(lq).copied().unwrap_or("?"), crate::trade::LIQ_COVER_S.load(std::sync::atomic::Ordering::Relaxed), crate::trade::LIQ_EXIT_S.load(std::sync::atomic::Ordering::Relaxed));
    }
    if crate::sw::on(crate::sw::CHG_DATED) {
        // REPAIR A (check-engine-1 M1): the charges at the DATED rates only; the dearer-of reading (dated vs 2026-10-30) beside, never in the net
        let (cd, cx): (i64, i64) = rows.iter().fold((0, 0), |a, r| (a.0 + r.0.charges, a.1 + r.0.charges_dearer));
        let _ = writeln!(s, "- Charges at the DATED NSE / SEBI / stamp rates of each session (addenda 7 / 10; exchange charge before 2024-10-01 = NSE's Rs 3-100 crore premium slab of circulars FA46730 (Rs 50.00 per lakh to 2020-12-31, Rs 53.00 from 2021-01-01), FA56129 (Rs 50.00 from 2023-04-01; IPFT Rs 50 per crore from 2023-04-01) and FA61137 (Rs 49.50 from 2024-04-01), the costs crate's dated rows from 2024-10-01): Rs {} in the net; beside only, the h6 reading at the dearer of the dated and the 2026-10-30 sets: Rs {} (Rs {} more).", fmt_rs(cd), fmt_rs(cx), fmt_rs(cx - cd));
        if crate::fix33::on(crate::fix33::CHG33) {
            // v3.3 fix CHG33 (check-engine-1 M3r): the dated SEBI 50% window and the unverified pre-July-2020 stamp duty, named per pick
            s.push_str(&crate::fix33::charges_words(&rows.iter().map(|r| r.0.day).collect::<Vec<_>>()));
        }
    }
    // addendum 14 (d)
    let we = rows.iter().map(|r| r.0.entry_ttf).max().unwrap_or(0);
    let wx = rows.iter().filter_map(|r| r.0.exit_ttf).max().unwrap_or(0);
    let unf = rows.iter().filter(|r| r.0.nc_units > 0).count();
    let part = rows.iter().filter(|r| r.0.filled_bp < 10_000).count();
    let _ = writeln!(s, "- Liquidity (addendum 14 (d)): worst entry time-to-fill {we} s; worst exit time-to-fill {wx} s; sales not completely filled {part}; UNFILLED EXITS (units valued, not filled) {unf} trades / {} units {}; exit re-sends p50 / max {} / {}.", rows.iter().map(|r| r.0.nc_units).sum::<u64>(), if unf == 0 { "(0: the headline condition holds)" } else { "(NOT 0: not a headline under addendum 14 (d))" }, { let mut v: Vec<u32> = rows.iter().map(|r| r.0.resends).collect(); v.sort_unstable(); v.get(v.len() / 2).copied().unwrap_or(0) }, rows.iter().map(|r| r.0.resends).max().unwrap_or(0));
    // addendum 17: the margin utilization (never above Rs 20 L: the kernel's hard cap with the running MTM loss)
    let mut days: Vec<i64> = rows.iter().map(|r| r.0.day).collect();
    days.sort_unstable();
    days.dedup();
    let run_days: Vec<i64> = UTIL_DAYS.get().cloned().unwrap_or(days);
    let tr: Vec<(i64, i64, i64)> = rows.iter().map(|r| (r.0.t_us, r.0.known, r.0.margin)).collect();
    let (pk, med, hi, idle) = utilization(&tr, &run_days);
    if crate::sw::on(crate::sw::PORT_MTM) {
        // REPAIR A (check-engine-2 S7): the margin line no longer claims "never above"; the margin + running MTM loss per minute is printed
        let _ = writeln!(s, "- Margin utilization (addendum 17; margin in use per trading minute over the run's {} sessions): peak Rs {} ({}), median Rs {}, minutes above 80% of Rs 20 L {}, idle minutes {}.", run_days.len(), fmt_rs(pk), pct_bp(pk, crate::trade::ACCOUNT), fmt_rs(med), pct_bp(hi, 10_000), pct_bp(idle, 10_000));
        let seg = rows.first().map_or(0, |r| usize::from(r.0.seg.min(1)));
        let tm: Vec<(i64, i64, i64, PMark)> = rows.iter().map(|r| (r.0.t_us, r.0.known, r.0.margin, PMark { units: r.0.units, sold: r.0.sold, mark1: r.0.mark1 })).collect();
        let _ = writeln!(s, "- Addendum 17 cap check between sales: {} (each sale itself is refused when margin in use + the running MTM loss would pass the free money: the kernel's rule).", mtm_peak_words(margin_mtm_peak(&tm, marks.get(seg).copied().unwrap_or(&[]), &run_days)));
    } else {
        let _ = writeln!(s, "- Margin utilization (addendum 17; margin in use per trading minute over the run's {} sessions, never above Rs 20,00,000): peak Rs {} ({}), median Rs {}, minutes above 80% of Rs 20 L {}, idle minutes {}.", run_days.len(), fmt_rs(pk), pct_bp(pk, crate::trade::ACCOUNT), fmt_rs(med), pct_bp(hi, 10_000), pct_bp(idle, 10_000));
    }
    // addendum 16: by bucket and by weekly / monthly
    let _ = writeln!(s);
    let _ = writeln!(s, "| days to expiry (sessions) | trades | net Rs | wins | SL exits | worst trade Rs | weekly / monthly trades | weekly / monthly net Rs |");
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|");
    for (b, bn) in crate::expiry::BUCKETS.iter().enumerate() {
        let v: Vec<&(Row32, String)> = rows.iter().filter(|r| r.0.bucket == b).collect();
        if v.is_empty() {
            continue;
        }
        let (w, m): (Vec<&&(Row32, String)>, Vec<&&(Row32, String)>) = v.iter().partition(|r| !r.0.monthly);
        let _ = writeln!(s, "| {bn} | {} | {} | {} | {} | {} | {} / {} | {} / {} |", v.len(), fmt_rs(v.iter().map(|r| r.0.net).sum()), v.iter().filter(|r| r.0.net > 0).count(), v.iter().filter(|r| r.0.sl_exit).count(), fmt_rs(v.iter().map(|r| r.0.net).min().unwrap_or(0)), w.len(), m.len(), fmt_rs(w.iter().map(|r| r.0.net).sum()), fmt_rs(m.iter().map(|r| r.0.net).sum()));
    }
    let moved = rows.iter().filter(|r| r.0.moved).count();
    let _ = writeln!(s);
    let _ = writeln!(s, "Trades on an expiry a holiday moved: {moved}.");
    // addendum 8: Greek bands
    let _ = writeln!(s);
    let _ = writeln!(s, "| entry |delta| band | trades | net Rs | win rate | | entry IV band | trades | net Rs | win rate |");
    let _ = writeln!(s, "|---|---|---|---|---|---|---|---|---|");
    for k in 0..6 {
        let a: Vec<&(Row32, String)> = rows.iter().filter(|r| delta_band(r.0.delta) == k).collect();
        let b: Vec<&(Row32, String)> = rows.iter().filter(|r| iv_band(r.0.iv) == k).collect();
        let wr = |v: &[&(Row32, String)]| if v.is_empty() { "-".to_owned() } else { format!("{}%", v.iter().filter(|r| r.0.net > 0).count() * 100 / v.len()) };
        let _ = writeln!(s, "| {} | {} | {} | {} | | {} | {} | {} | {} |", DELTA_BANDS.get(k).copied().unwrap_or("?"), a.len(), fmt_rs(a.iter().map(|r| r.0.net).sum()), wr(&a), IV_BANDS.get(k).copied().unwrap_or("?"), b.len(), fmt_rs(b.iter().map(|r| r.0.net).sum()), wr(&b));
    }
    let mut gns: Vec<u64> = rows.iter().map(|r| r.0.greeks_ns).filter(|&x| x > 0).collect();
    gns.sort_unstable();
    let q = |p: usize| gns.get(gns.len().saturating_sub(1) * p / 100).copied().unwrap_or(0);
    let _ = writeln!(s);
    let _ = writeln!(s, "Greeks solve time per instant (the app's solver, MEASURED): p50 {} ns / p99 {} ns over {} instants.", q(50), q(99), gns.len());
    // addendum 15: months and weeks of this pick
    let mut mo: BTreeMap<i64, i64> = months_all.iter().map(|&m| (m, 0)).collect();
    let mut wk: BTreeMap<i64, i64> = BTreeMap::new();
    for r in rows {
        *mo.entry(month_id(r.0.day)).or_insert(0) += r.0.net;
        *wk.entry(week_of(r.0.day)).or_insert(0) += r.0.net;
    }
    let ok = mo.values().filter(|&&v| crate::kern::month_ok(v)).count();
    let wpos = wk.values().filter(|&&v| v > 0).count();
    let _ = writeln!(s);
    let mw: Vec<String> = mo.iter().map(|(m, v)| if crate::kern::month_ok(*v) { format!("{} Rs {} passes", month_name(*m), fmt_rs(*v)) } else { format!("{} Rs {} (short by Rs {})", month_name(*m), fmt_rs(*v), fmt_rs(crate::kern::MONTH_MIN - *v)) }).collect();
    if crate::sw::on(crate::sw::WK_RUN) || crate::sw::on(crate::sw::A19) {
        // REPAIR A (check-engine-2 S1 / S9, addendum 19): days, weeks and months against the RUN's sessions (no trade = net 0, not positive)
        let v = v19_of(&rows.iter().map(|r| (r.0.day, r.0.net)).collect::<Vec<_>>(), &run_days);
        let _ = writeln!(s, "Months (addendum 15 verdict 1: every month >= Rs 2,00,000 net): {ok} of {} pass; {}.", mo.len(), mw.join("; "));
        let _ = writeln!(s);
        let _ = writeln!(s, "Addendum 19 (against the run's {} sessions, {} weeks, {} months; a session, week or month without a trade is net 0, not positive): {}. Max drawdown: see the pick's Overview. Closest-miss gaps: {}.", v.days.1, v.weeks.1, v.months_ok.1, v19_words(&v), v19_miss_words(&v));
        if crate::sw::on(crate::sw::A19) {
            // addendum 16 / check-engine-2 S4: the days-to-expiry split EVERY MONTH, and the expiry-day trades apart
            let _ = writeln!(s);
            let _ = writeln!(s, "| month | days to expiry | trades | net Rs | wins | SL exits | worst trade Rs |");
            let _ = writeln!(s, "|---|---|---|---|---|---|---|");
            for &m in months_all {
                for (b, bn) in crate::expiry::BUCKETS.iter().enumerate() {
                    let v2: Vec<&(Row32, String)> = rows.iter().filter(|r| month_id(r.0.day) == m && r.0.bucket == b).collect();
                    if v2.is_empty() {
                        continue;
                    }
                    let _ = writeln!(s, "| {} | {bn} | {} | {} | {} | {} | {} |", month_name(m), v2.len(), fmt_rs(v2.iter().map(|r| r.0.net).sum()), v2.iter().filter(|r| r.0.net > 0).count(), v2.iter().filter(|r| r.0.sl_exit).count(), fmt_rs(v2.iter().map(|r| r.0.net).min().unwrap_or(0)));
                }
            }
            let ed: Vec<&(Row32, String)> = rows.iter().filter(|r| r.0.expiry_day).collect();
            let _ = writeln!(s);
            let _ = writeln!(s, "Expiry-day trades apart (addendum 16): {} trades, net Rs {}, worst trade Rs {}, SL exits {}, unfilled exits {}.", ed.len(), fmt_rs(ed.iter().map(|r| r.0.net).sum()), fmt_rs(ed.iter().map(|r| r.0.net).min().unwrap_or(0)), ed.iter().filter(|r| r.0.sl_exit).count(), ed.iter().filter(|r| r.0.nc_units > 0).count());
        }
    } else {
        let _ = writeln!(s, "Months (addendum 15 verdict 1: every month >= Rs 2,00,000 net): {ok} of {} pass; {}. Weeks with a trade positive: {wpos} of {}; worst week Rs {}.", mo.len(), mw.join("; "), wk.len(), fmt_rs(wk.values().copied().min().unwrap_or(0)));
    }
    let _ = writeln!(s);
    s
}

/// The run's sessions (the utilization denominator), set once by the report.
pub static UTIL_DAYS: std::sync::OnceLock<Vec<i64>> = std::sync::OnceLock::new();

/// One portfolio member's trades: (entry key us, known us, margin, net, day, contract key (day, contract), R1 key (day, underlying + side +
/// expiry)).
pub type PTrade = (i64, i64, i64, i64, i64, u64, u64);

/// The portfolio walk's outcome: (net, trades taken, refused by the shared Rs 20 L cap or the cash rule, refused by the one-account rules
/// (the same contract or the same underlying + side + expiry already open in the portfolio), max DD, month nets).
pub type PRes = (i64, u32, u32, u32, i64, BTreeMap<i64, i64>);

/// The s26 portfolio layer (addendum 15 / 17): several combos together under ONE Rs 20 L account at every instant: each combo's own
/// decisions as walked alone, then the ONE account's rules across all of them: a trade refused when the same contract, or another strike of
/// the same underlying + side + expiry (R1), is already open in the portfolio; or when the margin in use of ALL open positions plus its
/// margin passes Rs 20 L, or the free money (Rs 20 L + min(0, realized) - margin in use) cannot cover it (lost money never reused). Same
/// entry instant: the trade known first is taken first.
#[must_use]
pub fn portfolio(members: &[&[PTrade]], months_all: &[i64]) -> PRes {
    let mut all: Vec<PTrade> = members.iter().flat_map(|m| m.iter().copied()).collect();
    let mut taken: Vec<(i64, i64, i64)> = Vec::new();
    all.sort_by_key(|x| (x.0, x.1));
    // open: (known, margin, net, day, contract key, R1 key)
    let mut open: Vec<(i64, i64, i64, i64, u64, u64)> = Vec::new();
    let (mut realized, mut peak, mut dd, mut in_use) = (0_i64, 0_i64, 0_i64, 0_i64);
    let (mut trades, mut refused, mut ruled) = (0_u32, 0_u32, 0_u32);
    let mut mo: BTreeMap<i64, i64> = months_all.iter().map(|&m| (m, 0)).collect();
    let realize_upto = |t: i64, open: &mut Vec<(i64, i64, i64, i64, u64, u64)>, realized: &mut i64, in_use: &mut i64, peak: &mut i64, dd: &mut i64, mo: &mut BTreeMap<i64, i64>| {
        open.sort_by_key(|x| x.0);
        while let Some(&(k, m, n, d, _, _)) = open.first() {
            if k > t {
                break;
            }
            open.remove(0);
            *realized += n;
            *in_use -= m;
            *peak = (*peak).max(*realized);
            *dd = (*dd).max(*peak - *realized);
            *mo.entry(month_id(d)).or_insert(0) += n;
        }
    };
    for (t, k, m, n, d, ck, rk) in all {
        realize_upto(t, &mut open, &mut realized, &mut in_use, &mut peak, &mut dd, &mut mo);
        if open.iter().any(|x| x.4 == ck || x.5 == rk) {
            ruled += 1;
            continue;
        }
        let free = crate::trade::ACCOUNT + realized.min(0) - in_use;
        // MUTANT m_v32_portfolio_cap_ignored: every member trade taken whatever the shared Rs 20 L cap says
        let capped = !cfg!(feature = "m_v32_portfolio_cap_ignored") && (in_use + m > crate::trade::ACCOUNT || free < m);
        if capped {
            refused += 1;
            continue;
        }
        open.push((k, m, n, d, ck, rk));
        in_use += m;
        trades += 1;
        taken.push((t, k, m));
    }
    realize_upto(i64::MAX, &mut open, &mut realized, &mut in_use, &mut peak, &mut dd, &mut mo);
    if let Ok(mut g) = LAST_TAKEN.lock() {
        *g = taken;
    }
    (realized, trades, refused, ruled, dd, mo)
}

/// The trades the last `portfolio` call took (its utilization is printed beside it).
pub static LAST_TAKEN: std::sync::Mutex<Vec<(i64, i64, i64)>> = std::sync::Mutex::new(Vec::new());

/// `spikefade expiry-cal <out.tsv> --bhav-dir <dir> [--bhav-dir <dir>...] [--threads N]`: the DATED expiry calendar of NIFTY and BANKNIFTY
/// from NSE's own F&O bhavcopies (addendum 16: never today's rules backwards): per session and index, the expiries NSE listed, the nearest,
/// its weekday, weekly / monthly, whether a holiday moved it (its weekday against the most common weekday of the listed expiries), the
/// file; then the periods of each index's regular expiry weekday (the change points), each with its first and last session and file.
///
/// # Errors
/// No bhavcopy directory, an unreadable file.
pub fn cmd_expiry_cal(a: &[String]) -> Result<String, String> {
    let out = a.get(2).ok_or("expiry-cal <out.tsv> --bhav-dir <dir> ...")?;
    crate::bhav::from_args(a)?.ok_or("--bhav-dir <dir> is required")?;
    let b = crate::bhav::BHAV.get().ok_or("bhav")?;
    let threads: usize = a.iter().position(|x| x == "--threads").and_then(|i| a.get(i + 1)).and_then(|x| x.parse().ok()).unwrap_or(8);
    let days: Vec<i64> = b.sessions();
    // the session calendar of the expiry-cal command: the sessions NSE published a bhavcopy for
    let sess: std::collections::BTreeSet<i64> = days.iter().copied().collect();
    let is_sess = |d: i64| sess.contains(&d);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let res: std::sync::Mutex<Vec<(i64, String)>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                loop {
                    let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(&d) = days.get(k) else { break };
                    let line = match b.day(d) {
                        Ok(bd) => {
                            let mut t = String::new();
                            // the regime reference: NIFTY's listed expiries (weekly through every period of the calendar)
                            let reference = bd.expiries.get("NIFTY").cloned().unwrap_or_default();
                            for idx in ["NIFTY", "BANKNIFTY"] {
                                let v = bd.expiries.get(idx).cloned().unwrap_or_default();
                                let near = v.iter().copied().find(|&e| crate::cal::day_of_ymd(e) >= d).unwrap_or(0);
                                let nd = crate::cal::day_of_ymd(near);
                                let (reg, moved) = if near == 0 { (0, false) } else { crate::expiry::regular_and_moved(if reference.is_empty() { &v } else { &reference }, near, &is_sess) };
                                let monthly = near != 0 && crate::expiry::monthly_of(&v, near) == Some(near);
                                let _ = writeln!(t, "{}\t{idx}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", fmt_day(d), if near == 0 { "-".to_owned() } else { fmt_day(nd) }, if near == 0 { "-" } else { crate::expiry::weekday(nd) }, crate::rules::WD.get(reg).copied().unwrap_or("?"), if monthly { "monthly" } else { "weekly" }, if moved { "MOVED (holiday)" } else { "regular" }, v.len(), v.iter().map(|e| crate::cal::fmt_day(crate::cal::day_of_ymd(*e))).collect::<Vec<_>>().join(","), bd.file);
                            }
                            t
                        }
                        Err(e) => format!("{}\t-\t-\t-\t-\t-\t-\t0\t-\tREFUSED: {e}\n", fmt_day(d)),
                    };
                    if let Ok(mut g) = res.lock() {
                        g.push((d, line));
                    }
                }
            });
        }
    });
    let mut v = res.into_inner().unwrap_or_default();
    v.sort_by_key(|x| x.0);
    let mut tsv = String::from("session\tindex\tnearest_expiry\tnearest_weekday\tregular_weekday_of_listed_expiries\tnearest_kind\tmoved\texpiries_listed\texpiries\tbhavcopy_file\n");
    for (_, l) in &v {
        tsv.push_str(l);
    }
    // the periods of the regular weekday of each index (change points)
    let mut per = String::from("index\tregular_weekday\tfrom_session\tto_session\tsessions\n");
    for idx in ["NIFTY", "BANKNIFTY"] {
        let mut cur: Option<(String, String, String, u32)> = None;
        for (_, l) in &v {
            for row in l.lines() {
                let f: Vec<&str> = row.split('\t').collect();
                if f.get(1) != Some(&idx) {
                    continue;
                }
                let (Some(day), Some(reg)) = (f.first(), f.get(4)) else { continue };
                match &mut cur {
                    Some((r, _, to, n)) if r == reg => {
                        *to = (*day).to_owned();
                        *n += 1;
                    }
                    _ => {
                        if let Some((r, fr, to, n)) = cur.take() {
                            let _ = writeln!(per, "{idx}\t{r}\t{fr}\t{to}\t{n}");
                        }
                        cur = Some(((*reg).to_owned(), (*day).to_owned(), (*day).to_owned(), 1));
                    }
                }
            }
        }
        if let Some((r, fr, to, n)) = cur {
            let _ = writeln!(per, "{idx}\t{r}\t{fr}\t{to}\t{n}");
        }
    }
    std::fs::write(out, &tsv).map_err(|e| format!("{out}: {e}"))?;
    let pout = format!("{out}.periods.tsv");
    std::fs::write(&pout, &per).map_err(|e| format!("{pout}: {e}"))?;
    Ok(format!("expiry calendar: {} sessions -> {out}; regular-weekday periods -> {pout}\n{per}", v.len()))
}

/// Months spanned by a run's days (ids), every month counted even without a trade.
#[must_use]
pub fn months_of(days: &[i64]) -> Vec<i64> {
    let mut v: Vec<i64> = days.iter().map(|&d| month_id(d)).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The closest-miss words of one account's month nets (addendum 15: the exact rupee gap per failing month).
#[must_use]
pub fn miss_words(mo: &BTreeMap<i64, i64>) -> String {
    let fails: Vec<String> = mo.iter().filter(|(_, v)| !crate::kern::month_ok(**v)).map(|(m, v)| format!("{} net Rs {} short by Rs {}", month_name(*m), fmt_rs(*v), fmt_rs(crate::kern::MONTH_MIN - *v))).collect();
    if fails.is_empty() { "every month passes".to_owned() } else { fails.join("; ") }
}

/// The percentage words of a part of a whole.
#[must_use]
pub fn share(a: i64, b: i64) -> String {
    pct_bp(a, b)
}

/// REPAIR A (check-engine-1 M3): the EXIT_WHY words of a HELD exit (the rule fired inside 15:08-15:10, where no own order is sent): the
/// rule word, its distances, the TRIGGER second (never the anchor), how the level was set and the level crossed, then the anchor, the
/// cancelled rest, the square-off and the unfilled units.
#[must_use]
pub fn held_why(fam: &str, rule_words: &str, rule_dist: &str, trigger: &str, how: &str, level: i64, anchor: &str, sq: &str, nc: &str) -> String {
    // MUTANT m_v32r_exit_why_held_anchor: h6's words (no rule word, no level, the anchor printed as the firing second)
    if cfg!(feature = "m_v32r_exit_why_held_anchor") {
        return format!(" ({rule_words}) fired at {anchor} inside 15:08-15:10 where no own order is sent: the sale's rest was cancelled there; {sq}; {nc}");
    }
    format!("{fam} ({rule_words}; {rule_dist}) fired at {trigger}: {how}; the price crossed Rs {} with the anchor {anchor} inside 15:08-15:10, where no own order is sent: the sale's rest was cancelled there; {sq}; {nc}", fmt_rs(level))
}
