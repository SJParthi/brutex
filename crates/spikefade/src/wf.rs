//! v3: MANY WINDOWS FROM ONE SWEEP (EXITS-DYNAMIC-DESIGN 5.3; WINDOWS-MWF-v1, 21 honest rows over 9 year pieces, MULTI-WF-DESIGN as signed).
//!
//! The sweep keeps per account and year piece: net, trades, wins, the max drawdown inside the piece and the piece's highest and lowest
//! realized level (`kern::Piece`). A window's figures over pieces a..=b compose EXACTLY: net adds; the drawdown of A then B is max(dd_A, dd_B,
//! up_A - net_A - low_B); up and low shift by net_A. Growing rows start at the hull's first piece, so they read the running accumulator.
//! Sliding rows start later: a FRESH start there differs from the continuous walk only through the cash rule (free = Rs 20 L + min(0,
//! realized) - margin in use; R4 never reads realized; positions never cross a day, so nothing else carries over). With R0 = the continuous
//! walk's realized at the row's start, a cash check of slack s (refused iff s < 0) is refused by the fresh walk iff s < R0. The continuous
//! walk and the fresh walk therefore decide identically unless (R0 > 0 and some passed check has s < R0) or (R0 < 0 and some refused check
//! has s >= R0) inside the row's pieces: then the account-row is RE-WALKED from the row's start (exact). The re-walks are counted.
//!
//! Pieces (spike-fade cubes start 2018-10-01): piece y - 2018 for y = 2018 (Oct-Dec) .. 2026 (to the last staged day): 9 pieces.

use crate::kern::Piece;

/// The first year piece.
pub const Y0: i64 = 2018;
/// The pieces.
pub const NPIECE: usize = 9;

/// One window row: learning pieces a..=b, test pieces b+1..=last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinRow {
    pub id: String,
    pub learn: (usize, usize),
    pub test: (usize, usize),
    pub growing: bool,
}

/// The 21 rows of WINDOWS-MWF-v1 (generated from the rule text: 6 growing, 15 sliding), pieces by year - 2018, the test to 2026.
#[must_use]
pub fn rows21() -> Vec<WinRow> {
    let p = |y: i64| usize::try_from(y - Y0).unwrap_or(0);
    let last = p(2026);
    let mut v = Vec::new();
    for (id, e) in [("WF", 2019), ("AN2020", 2020), ("AN2021", 2021), ("TT", 2022), ("AN2023", 2023), ("AN2024", 2024)] {
        v.push(WinRow { id: id.to_owned(), learn: (0, p(e)), test: (p(e) + 1, last), growing: true });
    }
    for (l, e0) in [(1_i64, 2019_i64), (2, 2020), (3, 2021)] {
        for e in e0..=2024 {
            v.push(WinRow { id: format!("R{l}-{e}"), learn: (p(e - l + 1), p(e)), test: (p(e) + 1, last), growing: false });
        }
    }
    v
}

/// The composed figures of pieces a..=b.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Fig {
    pub net: i64,
    pub mdd: i64,
    pub up: i64,
    pub low: i64,
    pub trades: u32,
    pub wins: u32,
    /// Every piece in a..=b net positive.
    pub all_pos: bool,
    /// REPAIR A (WF19): months >= Rs 2,00,000, positive weeks and positive sessions in a..=b (the pieces' counters).
    pub mo_ok: u32,
    pub wk_pos: u32,
    pub d_pos: u32,
    /// v3.4 CAP20 (matrix X20): trading minutes above Rs 20 L (margin in use + running MTM loss) in a..=b (0 with the switch off).
    pub cap: u32,
}

/// Composes pieces a..=b (exact).
#[must_use]
pub fn compose(p: &[Piece], a: usize, b: usize) -> Fig {
    let mut f = Fig { all_pos: true, ..Fig::default() };
    for k in a..=b {
        let Some(x) = p.get(k) else {
            f.all_pos = false;
            continue;
        };
        let mdd = f.mdd.max(x.mdd).max(f.up - f.net - x.low);
        let up = f.up.max(f.net + x.up);
        let low = f.low.min(f.net + x.low);
        f = Fig { net: f.net + x.net, mdd, up, low, trades: f.trades + x.trades, wins: f.wins + x.wins, all_pos: f.all_pos && x.net > 0, mo_ok: f.mo_ok + u32::from(x.mo_ok), wk_pos: f.wk_pos + u32::from(x.wk_pos), d_pos: f.d_pos + u32::from(x.d_pos), cap: f.cap + u32::from(x.cap) };
    }
    f
}

/// The slack check of a row starting at piece `s` and reading pieces s..=b: true = a fresh start decides exactly as the continuous walk.
#[must_use]
pub fn fresh_equal(p: &[Piece], s: usize, b: usize) -> bool {
    if cfg!(feature = "m_sf3_window_no_rewalk") {
        // MUTANT m_sf3_window_no_rewalk: the composed window always trusted
        return true;
    }
    let r0: i64 = p.iter().take(s).map(|x| x.net).sum();
    for k in s..=b {
        let Some(x) = p.get(k) else { continue };
        if (r0 > 0 && x.min_pass < r0) || (r0 < 0 && x.max_ref >= r0) {
            return false;
        }
    }
    true
}

/// The learning key of common_wf R73 (bigger is better): every learning piece positive, then the smallest learning max drawdown, then the
/// most learning net. REPAIR A (check-engine-2 S2, WF19): the addendum 15 / 19 order on the learning months instead: the most months >= Rs
/// 2,00,000, then the most positive weeks, then the most positive sessions, then the smallest max drawdown, then the most net (v3.1's key
/// as (0 / 1, -DD, net) otherwise: the same order as its (bool, -DD, net)).
#[must_use]
pub fn learn_key(f: &Fig) -> (i64, i64, i64) {
    // MUTANT m_v32r_wf_learn_key_v31: the year rule kept under WF19
    if crate::sw::on(crate::sw::WF19) && !cfg!(feature = "m_v32r_wf_learn_key_v31") {
        (i64::from(f.mo_ok) * 10_000_000_000 + i64::from(f.wk_pos) * 100_000 + i64::from(f.d_pos), -f.mdd, f.net)
    } else {
        (i64::from(f.all_pos), -f.mdd, f.net)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acct::SEv;
    use crate::kern::{CSet, walk_account};
    use crate::stage::{EvX, NR, Tab3};
    use crate::trade::{EvT, NL, Out};

    #[test]
    fn rows_are_the_signed_21() {
        let r = rows21();
        assert_eq!(r.len(), 21);
        assert_eq!(r.iter().filter(|x| x.growing).count(), 6);
        let wf = r.iter().find(|x| x.id == "WF").cloned().unwrap_or_else(|| panic!("WF"));
        assert_eq!((wf.learn, wf.test), ((0, 1), (2, 8)));
        let r3 = r.iter().find(|x| x.id == "R3-2021").cloned().unwrap_or_else(|| panic!("R3"));
        assert_eq!((r3.learn, r3.test), ((1, 3), (4, 8)));
        assert!(r.iter().all(|x| x.growing || x.learn.0 >= 1), "no sliding row starts on the 2018 piece");
    }

    /// Synthetic accounts over 9 "years" (one day each): every row's composed figures equal a fresh walk from the row's start whenever the
    /// slack check passes; a forced cash-rule case fails the check and the re-walk equals the fresh walk.
    #[test]
    fn sf3_window_compose_equals_fresh_walk() {
        // 9 days = 9 pieces; each day: events at 10:00 .. 10:50 on distinct contracts, one lot of margin m, net per event from a pattern
        let mut evs: Vec<SEv> = Vec::new();
        let mut ex: Vec<EvX> = Vec::new();
        let mut tab = Tab3::default();
        let of_day: Vec<u8> = (0..9).collect();
        let nets = [[50_000_000_i32, -70_000_000, 1_000, 2_000], [-20_000, 30_000, -40_000, 5_000], [-150_000_000, 10_000, 10_000, 10_000], [7_000, -1_000, 3_000, -9_000], [100, 200, -300, 400], [-5_000, -5_000, 60_000, 1], [9_000, 9_000, -1, -1], [-80_000_000, 20_000, 30_000, -10_000], [1_000, 2_000, 3_000, 4_000]];
        for d in 0..9_u8 {
            for k in 0..4_u32 {
                let t = 36_000 + k * 600;
                let mut et = EvT { lots: [1; 4], q_of: [0; 4], qs: vec![1], margin_lot: 60_000_000, filled: [true; NL], ..EvT::default() };
                et.refuse = crate::trade::Refuse::None;
                let x = EvX { r0: u32::try_from(tab.anc.len()).unwrap_or(0), t0: 0, nq: 1, tf: Vec::new(), fill: Vec::new(), mark1: 0 };
                for _l in 0..NL {
                    let mut a = [crate::stage::ABSENT; NR];
                    a[0] = crate::stage::NEVER3;
                    tab.anc.push(a);
                    tab.os.push([0; NR]);
                    tab.obase.push(u32::try_from(tab.outs.len()).unwrap_or(0));
                    let net = nets[usize::from(d)][k as usize];
                    tab.outs.push(Out { net, exit: u16::try_from(i64::from(t) + 300 - crate::trade::BASE_S).unwrap_or(0), flag: 1, ..Out::default() });
                }
                evs.push(SEv { di: u16::from(d), ci: k + 10 * u32::from(d), t, skey: k + 10 * u32::from(d), tab: et });
                ex.push(x);
            }
        }
        let list: Vec<u32> = (0..u32::try_from(evs.len()).unwrap_or(0)).collect();
        let keys = crate::kern::list_keys(&evs, &list);
        let cs = CSet { rules: vec![(false, 0)], sl: 0 };
        let (_, pc, _) = walk_account(&evs, &ex, &tab, &list, &keys, &cs, 0, 0, None, true, false, &of_day, NPIECE, 9);
        let mut checked = 0;
        let mut rewalked = 0;
        let mut truly = 0;
        for row in rows21() {
            for (a, b) in [row.learn, row.test] {
                let f = compose(&pc.p, a, b);
                // a fresh walk from piece `a` over pieces a..=b
                let sub: Vec<u32> = list.iter().copied().filter(|&i| { let d = usize::from(evs[i as usize].di); d >= a && d <= b }).collect();
                let sk = crate::kern::list_keys(&evs, &sub);
                let (fs, fp, _) = walk_account(&evs, &ex, &tab, &sub, &sk, &cs, 0, 0, None, true, false, &of_day, NPIECE, 9);
                let ff = compose(&fp.p, a, b);
                if fresh_equal(&pc.p, a, b) {
                    assert_eq!((f.net, f.mdd, f.trades, f.wins, f.all_pos), (ff.net, ff.mdd, ff.trades, ff.wins, ff.all_pos), "row {} pieces {a}..{b}", row.id);
                    assert_eq!(ff.net, fs.net);
                    checked += 1;
                } else {
                    rewalked += 1;
                    if (f.net, f.mdd, f.trades) != (ff.net, ff.mdd, ff.trades) {
                        truly += 1;
                    }
                }
            }
        }
        assert!(checked >= 10, "checked {checked}");
        assert!(truly > 0, "at least one flagged account-row really differs from its fresh walk (the check is not vacuous)");
        // the forced case: the 2018 piece loses Rs 20 L - 1.5 L...: day 2 has a -Rs 15 L trade, so the cash rule bites later in the
        // continuous walk but not in a fresh one: the slack check must catch some rows
        assert!(rewalked > 0, "the forced cash-rule case must be re-walked (rewalked {rewalked})");
    }

    #[test]
    fn compose_is_exact_on_concatenation() {
        let a = Piece { net: 100, trades: 2, wins: 1, mdd: 30, up: 120, low: -10, ..Piece::default() };
        let b = Piece { net: -50, trades: 1, wins: 0, mdd: 70, up: 20, low: -50, ..Piece::default() };
        let f = compose(&[a, b], 0, 1);
        // equity path: A rises to 120 and ends at 100; B goes up 20 (to 120) then down to 50: dd = max(30, 70, 120 - 100 + 50 = 70) = 70
        assert_eq!((f.net, f.mdd, f.up, f.low, f.all_pos), (50, 70, 120, -10, false));
    }
}
