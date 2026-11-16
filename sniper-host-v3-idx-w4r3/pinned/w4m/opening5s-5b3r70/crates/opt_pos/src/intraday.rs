//! S7 BUILDER (wave 4, main 5b507203, 15 Nov 2026): the INTRADAY index trade the host prices for every decision candle (plan v3.1
//! s3.1), as ONE call into the shared pricer. Nothing here is a new law: every reading is an existing function of this crate.
//!
//! * The CONTRACT ([`pick_intraday`]): the A2 / s23a ITM-fit of the CURRENT expiry as known (DP-IDXCAL `known_listed` in the file read
//!   before the session), by `pos::pick` (the same function the positional entries and G-ROLL use; decisions read the store's 1-s bars,
//!   as DP-IDXITM does). INTRADAY never switches to the next expiry at 15:00 (F3): an entry decided at 15:05 on an expiry day buys the
//!   EXPIRING contract and is squared off from 15:10. The pick is made ONCE at the Rs 5,00,000 target (R5: the contract is the A2 pick).
//! * The QUANTITY of each N: C11 `size_units_closest(slot, reference, lot, free = the account)`; a slot that buys no whole lot carries
//!   no trade at that N (`ZERO_QTY`, net 0; the other N stay priced: the row is never dropped for it); a count the account's money cuts
//!   is `ACCOUNT_CAP`.
//! * Each share level: `price::price_intraday_holes` on the PRODUCTION rows (DP-NEWPX-IDXOPT under the DATA law, `Book::opt_rows`) with
//!   the XC exit planned PER PROTECTION PATH by `price::xc_plan` (the clock 300 s after that path's E0, strictly before the flip, never
//!   at or after the 15:08:00 cut, B1): a path whose clock wins is priced again with the exit at the clock's anchor, so its entry is
//!   RE-WALKED with that stop (S2, never a cut of a walked path). The worse path is booked (IDX-R3 (i); kept on a tie).
//! * The expiry-day leftovers: the WORSE of the settlement at intrinsic (the dated exercise law) and S15 bullet 1 over [W0, F&O end),
//!   W0 = the path's E0 (an intraday trade's window is its own session from E0, plan s3.1 (6)).
//! * D2 (MAIN-RULINGS-S7-S10 item 3): the 15:10:00 square-off is the strategy's OWN order (price.rs; mutant `m_d2_broker_sq`).
//! * The run-down (REQ 48): crate `path::runs` on the option's own seconds over [E0, the last fill known), bp of the entry value.
//!
//! Refusals are DECIDE-class only (known at the decision: `no_cal_row`, `no_series`, `no_ladder_row`, `no_spot`, `no_freeze_row`,
//! `no_itm_fit`, `no_print_before_t`); a hole under the trade (a LATER fact) is never a refusal: the `HOLE` flag (AFR-2).

use common_fill::limits::{self, SizeWhy, size_units_closest};
use common_fill::tape::{RangeMm, SecIndex, Tape};
use common_fill::{Side, US_S};
use idx_cal::day::Day;
use idx_cal::storeio::{self, at_sod};

use crate::pos::{self, Book, PickInfo};
use crate::price::{self, Leftover, Leg, PathOut, Plan};
use crate::secs::{self, Sec};

/// The flag bits of an intraday level (the host's 16-bit cash flag word: the shared bits keep the cash meanings).
pub mod flag {
    /// Units left at the F&O end, valued at S15 bullet 1 (not settled).
    pub const UNCLOSABLE: u16 = 2;
    /// An order of the booked path spans a DATA-law hole (AFR-2).
    pub const HOLE: u16 = 4;
    /// The entry filled 0 units (no exit order sent).
    pub const ZERO: u16 = 8;
    /// F17: the immediate Supertrend exit decided the level.
    pub const IMMEDIATE: u16 = 32;
    /// S2: the XC clock won and the entry was re-walked with its anchor.
    pub const REWALKED: u16 = 64;
    /// REQ 70a: units of the booked path's entry were cancelled for want of payable money.
    pub const PROTECTED: u16 = 128;
    /// C11: the count was cut to what the account's free money buys.
    pub const ACCOUNT_CAP: u16 = 256;
    /// Expiry-day leftovers SETTLED at intrinsic (the worse reading).
    pub const SETTLED: u16 = 512;
    /// The protection-OFF path was booked (it was the worse).
    pub const OFF_BOOKED: u16 = 1024;
    /// The slot buys no whole lot at this N (no trade at this N; net 0).
    pub const ZERO_QTY: u16 = 2048;
}

/// The contract of one intraday decision.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IPick {
    /// The contract folder.
    pub folder: String,
    /// Strike, paisa.
    pub strike: i64,
    /// CALL (a long signal) or PUT.
    pub ce: bool,
    /// The held series' TRUE expiry day.
    pub expiry: Day,
    /// The pick's lot (C17 / D9 at t).
    pub lot: u64,
    /// That day's freeze quantity.
    pub freeze: i64,
    /// That day's one-order maximum for the lot.
    pub max_order: i64,
    /// The A2 facts (spot, depth, reference, its age, skips, the Rs 5 L quantity, the alias and file read).
    pub info: PickInfo,
    /// The contract expires this session (its leftovers settle).
    pub expiring_today: bool,
    /// The official index close of the session (the settlement close; 0 when not an expiry day).
    pub close: i64,
    /// The buy's protection limit (protection kept): 10% above the last print before t, on the 5-paisa tick.
    pub buy_limit: i64,
}

/// One (N, share level) of an intraday trade, booked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ILevel {
    /// The booked net (estimator, the dearer of the dated and today's set), paisa.
    pub net: i64,
    /// The dated net.
    pub net_dated: i64,
    /// The per-leg net (every whole-lot order slice an order).
    pub net_per_leg: i64,
    /// The run-down, bp of the entry value.
    pub rd_bp: u16,
    /// Units bought.
    pub units: u64,
    /// Premium paid.
    pub entry_value: i64,
    /// Proceeds (exit + square-off + leftover valuation).
    pub exit_value: i64,
    /// The exit anchor (the clock's, the flip's, or the 15:10 square-off).
    pub exit_anchor: i64,
    /// The last fill's known instant (the F&O end for a leftover).
    pub exit_known: i64,
    /// E0 of the booked path.
    pub e0: i64,
    /// Flags ([`flag`]).
    pub flags: u16,
    /// REQ 70a units cancelled.
    pub unpaid: u64,
    /// Units left at the F&O end.
    pub leftover: u64,
}

/// One decision priced at every N and share level.
#[derive(Clone, Debug, Default)]
pub struct IPriced {
    /// The contract.
    pub pick: IPick,
    /// Per N: the count taken (0 = no whole lot).
    pub qty: Vec<u64>,
    /// Per N: the count before the account's cut.
    pub uncapped: Vec<u64>,
    /// Per N, per level.
    pub lv: Vec<Vec<ILevel>>,
    /// E0 at the first N priced with units (0 = none).
    pub entry_first: i64,
    /// `price_intraday_holes` calls made (the cost counter).
    pub calls: u32,
}

/// The A2 / s23a contract of an intraday decision at `t` (CALL for a long signal), the CURRENT expiry as known (F3).
///
/// # Errors
/// A store read error or a C11 host error (a named DECIDE refusal is `Ok(Err(code))`).
pub fn pick_intraday(b: &Book, ce: bool, t: i64, target: i64) -> Result<Result<IPick, &'static str>, String> {
    let (d, _) = storeio::ist_of(t);
    let Some(row) = b.cal.get(&d).copied() else { return Ok(Err("no_cal_row")) };
    // F3: the current expiry as known, never the next one (the 15:00:00 switch is POSITIONAL only)
    let (alias, file) = (row.known_listed, row.file_read);
    let Some(sr) = b.series_of(alias, file).cloned() else { return Ok(Err("no_series")) };
    let (p, info) = match pos::pick(b, d, file, alias, sr.final_date(), ce, t, target, false) {
        Ok(x) => x,
        Err(e) => {
            return if e == "no_itm_fit" {
                Ok(Err("no_itm_fit"))
            } else if e.starts_with("no spot") {
                Ok(Err("no_spot"))
            } else if e.starts_with("no_freeze_row") {
                Ok(Err("no_freeze_row"))
            } else if e.starts_with("no ladder row") {
                Ok(Err("no_ladder_row"))
            } else {
                Err(e)
            };
        }
    };
    let rows = b.opt_rows(&p.folder, d)?.unwrap_or_default();
    let Some(last) = secs::last_before(&rows, t) else { return Ok(Err("no_print_before_t")) };
    let freeze = b.freeze.units(&b.index, d).unwrap_or(0);
    let max_order = b.freeze.max_one_order(&b.index, d, i64::try_from(p.lot).unwrap_or(0)).unwrap_or(0);
    let expiring_today = sr.true_last == d;
    let close = if expiring_today { b.close.get(&d).copied().ok_or_else(|| format!("no official index close on the expiry day {} (S1 INDEX-CLOSE)", idx_cal::day::fmt(d)))? } else { 0 };
    Ok(Ok(IPick { folder: p.folder, strike: p.strike, ce, expiry: sr.true_last, lot: p.lot, freeze, max_order, info, expiring_today, close, buy_limit: limits::protection(last.close, Side::Buy, 1_000, 5) }))
}

/// The first entry fill's known end (E1) of a path.
fn e1_of(g: &PathOut) -> i64 {
    g.fills.iter().find(|f| f.0 == Leg::Entry).map_or(0, |f| f.5)
}

/// Prices one level of one N: the XC plan per protection path, the re-walk of a path whose clock wins, W0 = E0 for an expiry-day
/// leftover, the worse path booked. Returns the level and the pricer calls made.
#[allow(clippy::too_many_arguments, reason = "one level of one trade")]
fn price_level(rows: &[Sec], holes: &[(i64, i64)], pk: &IPick, t: i64, qty: u64, flip: Option<i64>, immediate: bool, bp: u16, account: i64, tape: &Tape<'_, Sec>) -> Result<(ILevel, u32), String> {
    let (d, _) = storeio::ist_of(t);
    let s0 = secs::sq0(d);
    let end = secs::fno_end(d);
    let leftover = |w0: i64| if pk.expiring_today { Leftover::Settle { close: pk.close, strike: pk.strike, ce: pk.ce, w0 } } else { Leftover::WorstSinceE0 };
    let plan = |exit: Option<i64>, w0: i64| Plan {
        d,
        t,
        lot: pk.lot,
        qty,
        buy_lim: pk.buy_limit,
        reserve: account,
        exit,
        bp,
        block_s: 1,
        leftover: leftover(w0),
        freeze: u64::try_from(pk.freeze).unwrap_or(0),
        max_order: u64::try_from(pk.max_order).unwrap_or(0),
    };
    let first = price::price_intraday_holes(rows, holes, &plan(flip, t))?;
    let mut calls = 1u32;
    let mut chosen: Vec<(PathOut, i64, bool)> = Vec::with_capacity(2);
    for pi in 0..2 {
        let g0 = first.paths.get(pi).cloned().unwrap_or_default();
        let (anchor, won) = price::xc_plan(d, g0.e0, e1_of(&g0), flip);
        let (mut g, exit) = if won {
            // S2: the clock won on this path: its entry is re-walked with the clock's anchor as its stop
            let again = price::price_intraday_holes(rows, holes, &plan(anchor, t))?;
            calls += 1;
            (again.paths.get(pi).cloned().unwrap_or_default(), anchor)
        } else {
            (g0, flip)
        };
        if pk.expiring_today && g.un_u > 0 && g.e0 > 0 && g.e0 != t {
            // plan s3.1 (6): an intraday trade's S15 bullet 1 window starts at its own E0 (same session)
            let again = price::price_intraday_holes(rows, holes, &plan(exit, g.e0))?;
            calls += 1;
            g = again.paths.get(pi).cloned().unwrap_or_default();
        }
        chosen.push((g, exit.unwrap_or(s0), won));
    }
    let net = |i: usize| chosen.get(i).map_or(0, |c| c.0.net);
    let booked = usize::from(net(1) < net(0));
    let (g, anchor, won) = chosen.get(booked).cloned().unwrap_or_default();
    let mut f = 0u16;
    if g.hole {
        f |= flag::HOLE;
    }
    if g.units_in == 0 {
        f |= flag::ZERO;
    }
    if g.un_u > 0 && !g.settled {
        f |= flag::UNCLOSABLE;
    }
    if g.settled {
        f |= flag::SETTLED;
    }
    if won {
        f |= flag::REWALKED;
    }
    if g.unpaid > 0 {
        f |= flag::PROTECTED;
    }
    if booked == 1 {
        f |= flag::OFF_BOOKED;
    }
    if immediate && !won {
        f |= flag::IMMEDIATE;
    }
    let exit_known = g.fills.iter().map(|x| x.5).max().unwrap_or(0);
    let rd_bp = if g.units_in > 0 && g.value_in > 0 {
        let to = if exit_known > 0 { exit_known } else { end };
        let ((_, _), (down, _)) = common_fill::path::runs(tape, g.e0, to, true, g.value_in, g.units_in);
        u16::try_from(i128::from(down) * 10_000 / i128::from(g.value_in)).map_err(|_| "a run-down left its u16 bp width".to_string())?
    } else {
        0
    };
    let lv = ILevel {
        net: g.net,
        net_dated: g.net_dated,
        net_per_leg: g.net_per_leg,
        rd_bp,
        units: g.units_in,
        entry_value: g.value_in,
        exit_value: g.proceeds,
        exit_anchor: anchor,
        exit_known,
        e0: g.e0,
        flags: f,
        unpaid: g.unpaid,
        leftover: g.un_u,
    };
    Ok((lv, calls))
}

/// Prices one intraday decision at every N (`slots`, paisa) and share level (`bps`): the contract `pk` (from [`pick_intraday`]), the
/// decision `t` (the candle END), the flip anchor `flip` (the END of the first index candle closing across Supertrend 10 x 3 before
/// the cut; None = the 15:10 square-off), `immediate` (F17), the account's free money `account` (the sweep rows: the whole account).
///
/// # Errors
/// A tape or walk error, a refused rate day, a C11 host error, a run-down outside its width.
#[allow(clippy::too_many_arguments, reason = "one decision at every N and level")]
pub fn price_intraday_itm(b: &Book, pk: &IPick, t: i64, flip: Option<i64>, immediate: bool, slots: &[i64], bps: &[u16], account: i64) -> Result<IPriced, String> {
    let (d, _) = storeio::ist_of(t);
    let rows = b.opt_rows(&pk.folder, d)?.unwrap_or_default();
    let holes = b.holes_of(&pk.folder, d);
    let s0 = secs::sq0(d);
    let flip = flip.filter(|&f| f < s0);
    let open = at_sod(d, 9 * 3600 + 15 * 60);
    let end = secs::fno_end(d);
    let ix = SecIndex::new(&rows, open, end).map_err(|e| format!("index {e:?}"))?;
    let mm = RangeMm::new(&rows);
    let tape = Tape::new(&rows, &ix, Some(&mm)).map_err(|e| format!("tape {e:?}"))?;
    let mut out = IPriced { pick: pk.clone(), ..IPriced::default() };
    for &slot in slots {
        let (qty, why) = size_units_closest(slot, Some(pk.info.reference), pk.lot, Some(account)).map_err(|e| format!("C11 {e:?}"))?;
        let uncapped = size_units_closest(slot, Some(pk.info.reference), pk.lot, None).map_or(0, |x| x.0);
        let zero = qty == 0 || why.refused() || matches!(why, SizeWhy::CashShortZero { .. });
        let mut lvs = Vec::with_capacity(bps.len());
        for &bp in bps {
            if zero {
                lvs.push(ILevel { flags: flag::ZERO_QTY | flag::ZERO, exit_anchor: flip.unwrap_or(s0), ..ILevel::default() });
                continue;
            }
            let (mut lv, calls) = price_level(&rows, &holes, pk, t, qty, flip, immediate, bp, account, &tape)?;
            out.calls += calls;
            if qty < uncapped {
                lv.flags |= flag::ACCOUNT_CAP;
            }
            if out.entry_first == 0 && lv.e0 > 0 {
                out.entry_first = lv.e0;
            }
            lvs.push(lv);
        }
        out.qty.push(if zero { 0 } else { qty });
        out.uncapped.push(uncapped);
        out.lv.push(lvs);
    }
    let _ = US_S;
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic, clippy::expect_used, reason = "tests")]
mod tests {
    //! Named extreme cases of the host's intraday index trade (plan v3.1 s3.5), on synthetic books (the real-day proofs are the S7
    //! canary contexts and the F8 gate over their picks).
    use std::collections::BTreeMap;

    use costs::option_clock::{self as oc, Reading, Trip};
    use idx_cal::day::{self, from_civil};

    use super::*;
    use crate::chain::Folders;
    use crate::freeze::FreezeTable;
    use crate::ladder::Ladder;
    use crate::pos::{CalRow, SeriesRow};
    use crate::sess::Sessions;

    fn d(m: u32, dd: u32) -> Day {
        from_civil(2025, m, dd)
    }

    fn sec(day: Day, h: i64, m: i64, s: i64, px: i64, vol: u64) -> Sec {
        Sec { ts: at_sod(day, h * 3600 + m * 60 + s), high: px, low: px, close: px, vol }
    }

    fn at(day: Day, h: i64, m: i64, s: i64) -> i64 {
        at_sod(day, h * 3600 + m * 60 + s)
    }

    const FREEZE: &str = "index\teffective_from\tfreeze_qty_units\tcircular_number\tcircular_date\nNIFTY\t2025-01-01\t1800\tA\t2024-12-30\n";

    /// Sessions 2025-08-26 (D2) and 2025-08-28 (D3, the expiry E1 = 08-28; next E2 = 09-02); the file read = the previous session.
    fn book() -> Book {
        let sessions = [d(8, 26), d(8, 28)];
        let files = [d(8, 25), d(8, 26)];
        let mut cal = BTreeMap::new();
        for (k, &s) in sessions.iter().enumerate() {
            cal.insert(s, CalRow { file_read: files[k], known_listed: d(8, 28), known_eff: d(8, 28), series: 1, next_eff: Some(d(9, 2)), d_exp: s == d(8, 28) });
        }
        let series = vec![SeriesRow { id: 1, aliases: vec![(d(8, 28), d(8, 22), d(8, 28))], true_last: d(8, 28) }, SeriesRow { id: 2, aliases: vec![(d(9, 2), d(8, 22), d(9, 2))], true_last: d(9, 2) }];
        let mut ladder = String::from("file_date\tform\tindex\texpiry\tce_strikes\tpe_strikes\tlots\toi_gcd\toi_rows\tfile_cands\n");
        for f in files {
            for e in [d(8, 28), d(9, 2)] {
                ladder.push_str(&format!("{}\tudiff\tNIFTY\t{}\t2400000\t2600000\t75\t-\t-\t-\n", day::fmt(f), day::fmt(e)));
            }
        }
        let mut srows = String::from("day\tfno_end\tindex_close\tsource\n");
        for s in sessions {
            srows.push_str(&format!("{}\t15:30:00\t15:30:00\tregular\n", day::fmt(s)));
        }
        let sess = Sessions::parse(&srows, "day\tsegment\tfrom\tresume\tsource\n").unwrap();
        let mut close = BTreeMap::new();
        for s in sessions {
            close.insert(s, 2_450_000);
        }
        let b = Book::new("NIFTY", cal, series, Ladder::parse(&ladder).unwrap(), Folders::default(), FreezeTable::parse(FREEZE).unwrap(), close, sess, None);
        for s in sessions {
            b.put_spot(s, vec![sec(s, 9, 30, 0, 2_450_000, 0), sec(s, 14, 59, 59, 2_450_000, 0)]);
        }
        b
    }

    const E1: &str = "2025-08-28-2400000-CE";
    const E2: &str = "2025-09-02-2400000-CE";

    fn net_est(day: Day, t: &Trip) -> i64 {
        let rd = oc::rates_on(i64::from(day)).unwrap();
        let rt = oc::rates_on(oc::today()).unwrap();
        t.sell_value - t.buy_value - oc::charged(&rd, &rt, t, Reading::Estimator).0
    }

    /// D2 (MAIN-RULINGS-S7-S10 item 3): a trade with no exit decided is squared off from 15:10:00 by the strategy's OWN order: the
    /// booked net is the estimator's with the square-off counted as one executed exit order and NO broker square-off fee; the broker
    /// reading (the robot's old one) is dearer by exactly Dhan's square-off fee + its GST. Red under `m_d2_broker_sq`.
    #[test]
    fn d2_the_1510_square_off_is_the_strategys_own_order() {
        let b = book();
        let day2 = d(8, 26);
        let mut r: Vec<Sec> = vec![sec(day2, 9, 30, 0, 50_000, 75)];
        r.extend((0..5).map(|s| sec(day2, 10, 0, s, 50_000, 100_000)));
        r.extend((0..5).map(|s| sec(day2, 15, 10, s, 51_000, 100_000)));
        b.put_opt(E1, day2, Some(r));
        let t = at(day2, 10, 0, 0);
        let pk = pick_intraday(&b, true, t, 50_000_000).unwrap().unwrap();
        assert_eq!((pk.folder.as_str(), pk.lot, pk.info.qty_a2, pk.expiring_today), (E1, 75, 975, false));
        let p = price_intraday_itm(&b, &pk, t, None, false, &[50_000_000], &[10_000], 200_000_000).unwrap();
        let lv = p.lv[0][0];
        assert_eq!((lv.units, lv.entry_value, lv.exit_value), (975, 975 * 50_000, 975 * 51_000));
        assert_eq!(lv.exit_anchor, at(day2, 10, 5, 0), "the XC clock 300 s after E0 won (no flip): exit at 10:05:00, re-walked");
        // the clock exit at 10:05:00 finds no print before 15:10: the exit order rests, the square-off sells: one own order either way
        let own = Trip { qty: 975, buy_value: 975 * 50_000, sell_value: 975 * 51_000, entry_orders: 1, exit_orders: 1, ..Trip::default() };
        let broker = Trip { exit_orders: 0, broker_orders: 1, ..own };
        assert_eq!(lv.net, net_est(day2, &own), "D2: the square-off is the strategy's own order (no broker square-off fee)");
        assert_eq!(net_est(day2, &own) - net_est(day2, &broker), 2_360, "the broker reading is dearer by Rs 20 + 18% GST");
    }

    /// F3 / s3.5 (10) + s3.1 (6): an entry decided at 15:05:00 on the expiry day buys the EXPIRING contract (never the next expiry);
    /// no clock is sent at or after the 15:08 cut (E0 15:05:00: clock 15:10:00), so it is squared off from 15:10:00; one 75-unit print
    /// at 15:12:00 leaves 900 units at the F&O end, valued the WORSE of the settlement at intrinsic (close Rs 24,500 against strike
    /// Rs 24,000: Rs 500 a unit, the dated exercise STT) and S15 bullet 1 over [W0 = E0, F&O end) with its own sale order:
    /// (a) the 15:12 print at Rs 490: bullet 1 is the worse (booked, UNCLOSABLE); (b) at Rs 510: the settlement is (SETTLED).
    /// F8: the Rs 5 L quantity of the pick fits one order.
    #[test]
    fn f3_entry_1505_on_the_expiry_day_uses_the_expiring_contract_and_its_leftover_is_valued_the_worse_way() {
        for (px, settled) in [(49_000_i64, false), (51_000_i64, true)] {
            let b = book();
            let day3 = d(8, 28);
            let mut r: Vec<Sec> = vec![sec(day3, 9, 30, 0, 52_000, 75), sec(day3, 15, 4, 59, 52_000, 75)];
            r.extend((0..5).map(|s| sec(day3, 15, 5, s, 52_000, 100_000)));
            r.push(sec(day3, 15, 12, 0, px, 75));
            b.put_opt(E1, day3, Some(r));
            b.put_opt(E2, day3, Some(vec![sec(day3, 15, 4, 59, 60_000, 75)]));
            let t = at(day3, 15, 5, 0);
            let pk = pick_intraday(&b, true, t, 50_000_000).unwrap().unwrap();
            assert_eq!(pk.folder, E1, "F3: the expiring contract, not {E2}");
            assert!(pk.expiring_today && pk.close == 2_450_000 && pk.expiry == day3);
            assert!(i64::try_from(pk.info.qty_a2).unwrap() <= pk.max_order, "F8: the A2 quantity fits one order");
            let p = price_intraday_itm(&b, &pk, t, None, false, &[50_000_000], &[10_000], 200_000_000).unwrap();
            let lv = p.lv[0][0];
            assert_eq!(lv.units, 975, "C11 closest of Rs 5 L at Rs 520 with lot 75: 13 lots");
            assert_eq!(lv.exit_anchor, secs::sq0(day3), "no clock at or after the 15:08 cut (B1): the square-off");
            assert_eq!(lv.leftover, 900, "75 units sold at 15:12:00, 900 left at the F&O end");
            assert_eq!(lv.flags & flag::SETTLED != 0, settled, "print {px}: the worse reading is booked");
            assert_eq!(lv.flags & flag::UNCLOSABLE != 0, !settled);
            assert_eq!(lv.exit_known, at(day3, 15, 30, 0), "a leftover's valuation row is known at the F&O end");
        }
    }

    /// s3.5 (8) on the option tape: an XC clock whose anchor would be at 15:08:00 is never sent (B1); one at 15:07:59 is, and its
    /// entry is re-walked (S2); a flip before the clock wins.
    #[test]
    fn xc_clock_b1_cut_and_rewalk_on_the_option_tape() {
        let b = book();
        let day2 = d(8, 26);
        let mut r: Vec<Sec> = vec![sec(day2, 9, 30, 0, 50_000, 75)];
        r.extend((0..4).map(|s| sec(day2, 15, 2, 59 + s, 50_000, 100_000)));
        r.extend((0..30).map(|s| sec(day2, 15, 7, 30 + s, 50_500, 100_000)));
        r.extend((0..5).map(|s| sec(day2, 15, 10, s, 50_200, 100_000)));
        b.put_opt(E1, day2, Some(r));
        // entry at 15:02:59: E0 15:02:59, clock 15:07:59 < 15:08:00: sent and re-walked
        let t1 = at(day2, 15, 2, 59);
        let pk1 = pick_intraday(&b, true, t1, 50_000_000).unwrap().unwrap();
        let p1 = price_intraday_itm(&b, &pk1, t1, None, false, &[50_000_000], &[10_000], 200_000_000).unwrap();
        assert_eq!(p1.lv[0][0].exit_anchor, at(day2, 15, 7, 59));
        assert!(p1.lv[0][0].flags & flag::REWALKED != 0);
        // entry at 15:03:00: clock 15:08:00 = the cut: never sent; the square-off
        let t2 = at(day2, 15, 3, 0);
        let pk2 = pick_intraday(&b, true, t2, 50_000_000).unwrap().unwrap();
        let p2 = price_intraday_itm(&b, &pk2, t2, None, false, &[50_000_000], &[10_000], 200_000_000).unwrap();
        assert_eq!(p2.lv[0][0].exit_anchor, secs::sq0(day2));
        assert_eq!(p2.lv[0][0].flags & flag::REWALKED, 0);
        // a flip at 15:05:00 before the 15:07:59 clock: the flip
        let p3 = price_intraday_itm(&b, &pk1, t1, Some(at(day2, 15, 5, 0)), false, &[50_000_000], &[10_000], 200_000_000).unwrap();
        assert_eq!(p3.lv[0][0].exit_anchor, at(day2, 15, 5, 0));
    }

    /// The 15-value N grid: a slot that buys no whole lot (Rs 15,000 against a 75-unit lot at Rs 500 = Rs 37,500, more than twice the
    /// slot: 0 by the closest count) is ZERO_QTY at that N (net 0) while every other N is priced: the row is never dropped for it.
    #[test]
    fn n_without_a_whole_lot_is_zero_qty_and_the_row_stays() {
        let b = book();
        let day2 = d(8, 26);
        let mut r: Vec<Sec> = vec![sec(day2, 9, 30, 0, 50_000, 75)];
        r.extend((0..5).map(|s| sec(day2, 10, 0, s, 50_000, 100_000)));
        r.extend((0..5).map(|s| sec(day2, 15, 10, s, 50_000, 100_000)));
        b.put_opt(E1, day2, Some(r));
        let t = at(day2, 10, 0, 0);
        let pk = pick_intraday(&b, true, t, 50_000_000).unwrap().unwrap();
        let p = price_intraday_itm(&b, &pk, t, None, false, &[50_000_000, 1_500_000], &[10_000], 200_000_000).unwrap();
        assert_eq!(p.qty, vec![975, 0]);
        assert!(p.lv[1][0].flags & flag::ZERO_QTY != 0 && p.lv[1][0].net == 0);
        assert!(p.lv[0][0].units == 975);
    }
}
