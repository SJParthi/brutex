//! The trade's whole count (SNIPER-REQUIREMENTS section 16, operator Sat 14 Nov 2026 ~11:2x IST: "Yes, closest count"; recorded
//! by MAIN 5b507203 at 11:26 IST). Host-side by design: the pinned common fill crate is never edited; its floor function
//! (`common_fill::limits::size_units`, crate S11) stays the `floor` rule, and this module adds the `closest` rule beside it.
//!
//! - **The rule is a RUN data row** (`qty_rule` = `closest` | `floor`, SNIPER-RUN-v5.tsv); a missing row or any other value is a
//!   named refusal (fail closed, never a default). Every result line that names sizing and every result DB table carries it.
//! - **closest**: the whole count (a multiple of `unit`: 1 share for cash, the lot for options) whose value at the reference price is
//!   CLOSEST to the target (cash: slot x leverage), even when slightly above it; exactly half way: the LOWER count. Exact integer
//!   arithmetic on the rational target slot x lev_num / lev_den (no float, no intermediate floor). It is 0 only when one unit costs
//!   at least twice the target (at exactly twice, 0 and 1 are equally close and the lower count wins); 0 is refused, named, as today.
//! - **floor**: `common_fill::limits::size_units` itself (the count before the 14 Nov ruling).
//! - **The account base (base Q7)**: never more money than the account has free at that instant (no borrowing, lost money never
//!   reused): the count is cut to the largest count whose money in (`common_fill::money::money_in`, ceil of value / leverage) fits
//!   the free money, and the cut is named.

use common_fill::{Paisa, Units};

/// The sizing rule of a run (RUN row `qty_rule`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum QtyRule {
    /// SNIPER s16: the closest whole count (half way: the lower count). The default of every NEW run's RUN file.
    Closest,
    /// floor(slot x leverage / reference) (crate S11), the rule before the 14 Nov ruling.
    Floor,
}

impl QtyRule {
    /// Parses the RUN row's value.
    ///
    /// # Errors
    /// Any value other than `closest` or `floor` (named; never a default).
    pub fn parse(v: &str) -> Result<Self, String> {
        match v.trim() {
            "closest" => Ok(Self::Closest),
            "floor" => Ok(Self::Floor),
            x => Err(format!("RUN row qty_rule: '{x}' is neither closest nor floor (SNIPER-REQUIREMENTS s16)")),
        }
    }

    /// The row value (`closest` / `floor`), written in the DB `qty_rule` columns.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Closest => "closest",
            Self::Floor => "floor",
        }
    }

    /// The words printed on every result line that names sizing (the floor words are SNIPER s16's own).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Closest => "sizing: closest whole count (SNIPER s16, 14 Nov ruling; half way: the lower count)",
            Self::Floor => "sizing: floor (before 14 Nov ruling)",
        }
    }
}

/// SNIPER s16's closest count: the multiple of `unit` whose value at `refpx` is nearest the target slot x lev_num / lev_den;
/// exactly half way: the lower count. 0 when the reference, the slot or the leverage denominator is not above 0 (as the floor rule).
#[must_use]
pub fn closest_units(slot: Paisa, lev_num: u32, lev_den: u32, refpx: Paisa, unit: Units) -> Units {
    let u = unit.max(1);
    if refpx <= 0 || slot <= 0 || lev_den == 0 {
        return 0;
    }
    // In lev_den-scaled paisa: the target is a = slot x lev_num, one unit's value is b = refpx x lev_den x unit (both exact).
    let a = i128::from(slot) * i128::from(lev_num);
    let b = i128::from(refpx) * i128::from(lev_den) * i128::from(u);
    let q = a / b;
    let r = a - q * b;
    // The count q is r below the target, q + 1 is b - r above it: q + 1 only when strictly closer (2r > b); a tie keeps q.
    // MUTANT m_s16q only: a tie takes the upper count.
    let up = if cfg!(feature = "m_s16q") { 2 * r >= b } else { 2 * r > b };
    let lots = if up { q + 1 } else { q };
    Units::try_from(lots.max(0)).map_or(Units::MAX, |l| l.saturating_mul(u))
}

/// The largest multiple of `unit` whose money in (`common_fill::money::money_in`: ceil(value x lev_den / lev_num), the whole value
/// when lev_num is 0) at `refpx` is at most `free`. 0 when nothing fits.
#[must_use]
pub fn fit_units(free: Paisa, lev_num: u32, lev_den: u32, refpx: Paisa, unit: Units) -> Units {
    let u = unit.max(1);
    if free <= 0 || refpx <= 0 {
        return 0;
    }
    // ceil(v x d / n) <= F  <=>  v x d <= F x n (integers); v = q x refpx.  With n = 0 the money in is v itself.
    let (n, d) = if lev_num == 0 { (1_i128, 1_i128) } else { (i128::from(lev_num), i128::from(lev_den.max(1))) };
    let lots = (i128::from(free) * n) / (i128::from(refpx) * d * i128::from(u));
    Units::try_from(lots.max(0)).map_or(Units::MAX, |l| l.saturating_mul(u))
}

/// The money in of `qty` units at `refpx` (`common_fill::money::money_in`, rounded against us).
#[must_use]
pub fn money_in(qty: Units, refpx: Paisa, lev_num: u32, lev_den: u32) -> Paisa {
    let v = i128::from(qty) * i128::from(refpx);
    common_fill::money::money_in(i64::try_from(v).unwrap_or(i64::MAX), lev_num, lev_den)
}

/// One trade's sized count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Sized {
    /// The count the trade takes (the rule's count, cut by the account base when it binds).
    pub qty: Units,
    /// The rule's own count (closest or floor) before the account base.
    pub rule_qty: Units,
    /// The account base cut the rule's count (named on the trade row).
    pub capped: bool,
}

/// The count of one trade: the rule's count for `slot` at `refpx`, cut to the largest count that fits `free` (base Q7).
#[must_use]
pub fn size(rule: QtyRule, slot: Paisa, free: Paisa, lev: (u32, u32), refpx: Paisa, unit: Units) -> Sized {
    let rule_qty = match rule {
        QtyRule::Floor => common_fill::limits::size_units(slot, lev.0, lev.1, refpx, unit),
        QtyRule::Closest => closest_units(slot, lev.0, lev.1, refpx, unit),
    };
    let fit = fit_units(free, lev.0, lev.1, refpx, unit);
    // MUTANT m_s16a only: the account base is never applied.
    if rule_qty > fit && !cfg!(feature = "m_s16a") { Sized { qty: fit, rule_qty, capped: true } } else { Sized { qty: rule_qty, rule_qty, capped: false } }
}

/// The account-base walk of one pick's trades in time order (base Q7 with one position at a time: no other trade is open at an
/// entry, so the money free at that instant is the account less the losses so far; profits are never added to it, and lost money
/// is never reused): per trade (money free before it, money in of its count, the largest count that fits when the count does not).
#[must_use]
pub fn account_walk(account: Paisa, trades: &[(Units, Paisa, i64)], lev: (u32, u32)) -> Vec<(Paisa, Paisa, Option<Units>)> {
    let mut cum = 0_i64;
    let mut out = Vec::with_capacity(trades.len());
    for &(qty, refpx, net) in trades {
        let free = account.saturating_add(cum.min(0)).max(0);
        let need = money_in(qty, refpx, lev.0, lev.1);
        let cut = if need > free && !cfg!(feature = "m_s16a") { Some(fit_units(free, lev.0, lev.1, refpx, 1)) } else { None };
        out.push((free, need, cut));
        cum = cum.saturating_add(net);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{QtyRule, account_walk, closest_units, fit_units, money_in, size};

    /// The plain definition: the multiple of `u` minimising |q x p x d - s x n| (scaled), the lower one on a tie, by search.
    fn brute(s: i64, n: u32, d: u32, p: i64, u: u64) -> u64 {
        let t = i128::from(s) * i128::from(n);
        let mut best = (i128::MAX, 0_u64);
        let mut q = 0_u64;
        loop {
            let v = i128::from(q) * i128::from(p) * i128::from(d);
            let diff = (v - t).abs();
            if diff < best.0 {
                best = (diff, q);
            }
            if v > t {
                break;
            }
            q += u;
        }
        best.1
    }

    #[test]
    fn closest_equals_the_plain_definition_and_never_below_floor() {
        let mut x = 0x9e37_79b9_u64;
        for _ in 0..20_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let s = i64::try_from(x % 4_000_000).unwrap() + 1;
            let p = i64::try_from((x >> 22) % 400_000).unwrap() + 1;
            let n = u32::try_from((x >> 41) % 7).unwrap() + 1;
            let d = u32::try_from((x >> 50) % 3).unwrap() + 1;
            let u = [1_u64, 1, 1, 25, 75][usize::try_from((x >> 55) % 5).unwrap()];
            let c = closest_units(s, n, d, p, u);
            assert_eq!(c, brute(s, n, d, p, u), "s {s} n {n} d {d} p {p} u {u}");
            let f = common_fill::limits::size_units(s, n, d, p, u);
            assert!(c == f || c == f + u, "closest is the floor or one unit above: s {s} p {p} f {f} c {c}");
        }
    }

    #[test]
    fn half_way_takes_the_lower_count_and_zero_only_at_twice_the_target() {
        // slot 100 x 5 = a target of 500 paisa; exact halves first:
        assert_eq!(closest_units(100, 5, 1, 200, 1), 2, "target 500 at 200: 2 (400, 100 below) vs 3 (600, 100 above): tie -> lower 2");
        assert_eq!(closest_units(100, 5, 1, 199, 1), 3, "target 500 at 199: 3 (597, 97 above) beats 2 (398, 102 below)");
        assert_eq!(closest_units(100, 5, 1, 201, 1), 2);
        assert_eq!(closest_units(100, 5, 1, 1000, 1), 0, "one share at exactly twice the target: 0 and 1 equally close, lower = 0");
        assert_eq!(closest_units(100, 5, 1, 999, 1), 1, "just under twice the target: 1 share");
        assert_eq!(closest_units(100, 5, 1, 2000, 1), 0);
        assert_eq!(closest_units(0, 5, 1, 10, 1), 0);
        assert_eq!(closest_units(100, 5, 0, 10, 1), 0);
        assert_eq!(closest_units(100, 5, 1, 0, 1), 0);
        // lots: target 500, lot of 3 at 40 = 120 a lot: 4 lots = 480 (20 below) vs 5 = 600 (100 above): 12 units.
        assert_eq!(closest_units(100, 5, 1, 40, 3), 12);
        // the ADANIENT shape: Rs 1,00,000 slot x 5 at Rs 2,345.65: 213.16 shares -> 213; at Rs 2,341.00: 213.58 -> 214.
        assert_eq!(closest_units(10_000_000, 5, 1, 234_565, 1), 213);
        assert_eq!(closest_units(10_000_000, 5, 1, 234_100, 1), 214);
        assert_eq!(common_fill::limits::size_units(10_000_000, 5, 1, 234_100, 1), 213);
    }

    #[test]
    fn the_account_base_cuts_to_the_largest_count_that_fits() {
        assert_eq!(fit_units(1000, 5, 1, 2500, 1), 2, "free 1000 x 5 = 5000 of exposure at 2500: 2");
        assert_eq!(money_in(2, 2500, 5, 1), 1000);
        assert_eq!(money_in(3, 2501, 5, 1), 1501, "ceil(7503 / 5)");
        assert_eq!(fit_units(1500, 5, 1, 2501, 1), 2, "3 x 2501 needs 1501 > 1500");
        assert_eq!(fit_units(0, 5, 1, 10, 1), 0);
        let s = size(QtyRule::Closest, 1000, 1001, (5, 1), 1999, 1);
        assert_eq!(s, super::Sized { qty: 2, rule_qty: 3, capped: true }, "closest 3 (5997) needs 1200 > free 1001: cut to 2");
        let f = size(QtyRule::Floor, 1000, 200_000, (5, 1), 1999, 1);
        assert_eq!(f, super::Sized { qty: 2, rule_qty: 2, capped: false });
        assert_eq!(QtyRule::parse("closest"), Ok(QtyRule::Closest));
        assert_eq!(QtyRule::parse(" floor "), Ok(QtyRule::Floor));
        assert!(QtyRule::parse("nearest").is_err() && QtyRule::parse("").is_err(), "fail closed");
        assert_eq!(QtyRule::Floor.label(), "sizing: floor (before 14 Nov ruling)");
        // the walk: account 10,000; a loss of 9,500 leaves 500 free; the next trade's money in 1,000 does not fit: cut to 1.
        let w = account_walk(10_000, &[(2, 2500, -9_500), (2, 2500, 300), (1, 2500, 0)], (5, 1));
        assert_eq!(w[0], (10_000, 1000, None));
        assert_eq!(w[1], (500, 1000, Some(1)));
        assert_eq!(w[2], (800, 500, None), "profits refill the free money only up to the losses; never above the account");
        let up = account_walk(10_000, &[(1, 2500, 5_000), (1, 2500, 0)], (5, 1));
        assert_eq!(up[1].0, 10_000, "profits are never added to the account base");
    }
}
