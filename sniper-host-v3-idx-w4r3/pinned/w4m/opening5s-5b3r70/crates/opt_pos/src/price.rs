//! The shared option pricer's INTRADAY path (plan s2 N6, s3.1 (4)-(6)): one trade on the option's own seconds with crate
//! v2.1's block walk (1-s blocks for index options, REQ 73b / 73c; 2-s for stock options), both protection paths (IDX-R3 (i):
//! the worse net is booked), REQ 70a pay from the reserve, the exit, the 15:10:00 square-off to the dated F&O end, the
//! leftovers, and the charges by `costs::option_clock` (Dhan's estimator at the dearer of the dated and today's set).
//! The XC exit (s20: the Supertrend flip OR 5 minutes after the option entry fill, with the B1 cut at 15:08:00) is planned
//! by [`xc_plan`]. Positional legs (the sequential roll, `close_split`, carry, settlement after a roll) live in the positional engine
//! (`pos.rs` of the S8 tree), which reuses [`walk1`].
//!
//! WAVE 4 (S4-S6 builder, 15 Nov 2026; check-build-idx-S1-S6 D1 and D3):
//! * **D1.** On an expiry day the leftover is valued two ways and the WORSE is booked (a tie books the settlement, as the
//!   positional engine does). The S15 bullet 1 reading sells the leftover at the worst traded price since W0 by its OWN sale
//!   order (Rs 20 + GST: `exit_orders + 1`); the settlement reading carries no such order but `REM_SETTLED` (Rs 20 on the
//!   exercised contract, the dated exercise STT, the SEBI fee on the notional). The leftover row prints the BOOKED valuation:
//!   [`Leg::Settled`] at the intrinsic value per unit when the settlement is booked, [`Leg::Unclosable`] at the bullet 1 price
//!   otherwise. The intrinsic value comes from crate v2.1 `limits::settle_intrinsic` (via `costs::option_clock::settle`: one
//!   common runtime). Named mutant `m_d1_lo_order` drops the bullet 1 order again.
//! * **D3.** The per-leg reading slices every order by `common_fill::legs::freeze_slices(units, max_order)` with
//!   `max_order` = the dated one-order maximum floor(freeze / lot) x lot (`FreezeTable::max_one_order`, R2), never the raw freeze.
//!   Named mutant `m_d3_raw_freeze` slices by the raw freeze again.

use common_fill::std70::{BlockFill, Kind, LevelFill, Order, Why, walk};
use common_fill::tape::{SecIndex, Seg, Tape};
use common_fill::{Dial, LevelMask, MAXL, Side, US_S, limits};
use costs::option_clock::{self as oc, Reading, Trip};
use idx_cal::day::Day;
use idx_cal::storeio::at_sod;

use crate::secs::{self, Sec};

/// Which order a fill belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Leg {
    /// The entry buy.
    Entry,
    /// The strategy exit sell.
    Exit,
    /// The 15:10 square-off sell.
    SquareOff,
    /// Units still open at the dated F&O end (valued, never a fill): the idx1s reading (lowest LOW since E0) or, on an index
    /// expiry day, S15 bullet 1 when it is the worse reading (sold by its own order at the worst traded price since W0).
    Unclosable,
    /// Units still open at the dated F&O end of the index expiry day, SETTLED at intrinsic value against the official close
    /// (crate S15 bullet 3) because the settlement is the worse reading (D1): the row prints the intrinsic value per unit.
    Settled,
}

/// One filled block: (block start, units, block volume, price, known instant).
pub type Fill = (Leg, i64, u64, u64, i64, i64);

/// How units still open at the F&O end are valued.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leftover {
    /// The idx1s reading: at the lowest traded LOW since E0 (5 paisa when none), counted `unclosable`.
    WorstSinceE0,
    /// The index-option expiry day (crate S15): the WORSE of the settlement at intrinsic against the official close
    /// (`settle`, with the dated exercise STT) and S15 bullet 1 (the worst traded price in [W0, F&O end)).
    Settle {
        /// The official index close of the day (S1's INDEX-CLOSE file), paisa.
        close: i64,
        /// The strike, paisa.
        strike: i64,
        /// CALL?
        ce: bool,
        /// W0 (B6): the window's start (E0 for an intraday trade).
        w0: i64,
    },
}

/// One trade's plan.
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    /// The session.
    pub d: Day,
    /// The entry anchor (the decision candle's END).
    pub t: i64,
    /// The lot.
    pub lot: u64,
    /// The quantity.
    pub qty: u64,
    /// The buy's protection limit (protection-kept path).
    pub buy_lim: i64,
    /// The payable exposure (REQ 70a reserve), both paths.
    pub reserve: i64,
    /// The decided exit anchor, if any (an exit decided at or after the square-off start is not one).
    pub exit: Option<i64>,
    /// The share dial, basis points.
    pub bp: u16,
    /// The block length, seconds (1: index options; 2: stock options).
    pub block_s: i64,
    /// The leftover rule.
    pub leftover: Leftover,
    /// The raw dated freeze quantity: ONLY for check-idx1s-early-A TABLE 1's per-leg reading beside (ceil(units / freeze)).
    pub freeze: u64,
    /// The dated one-order maximum for this contract's lot (`FreezeTable::max_one_order`: floor(freeze / lot) x lot, R2): the
    /// per-leg reading slices every order by `freeze_slices(units, max_order)` (D3). 0 is refused by name (no freeze row, F13).
    pub max_order: u64,
}

/// One protection path's outcome.
#[derive(Clone, Debug, Default)]
pub struct PathOut {
    /// Protection kept?
    pub kept: bool,
    /// Every filled block, in time order.
    pub fills: Vec<Fill>,
    /// Units bought.
    pub units_in: u64,
    /// Premium paid.
    pub value_in: i64,
    /// Units sold by the exit / the square-off / left at the end.
    pub xu: u64,
    /// Square-off units.
    pub squ: u64,
    /// Units left at the F&O end.
    pub un_u: u64,
    /// Their valuation price per unit.
    pub un_px: i64,
    /// Proceeds (exit + square-off + leftover valuation).
    pub proceeds: i64,
    /// REQ 70a units cancelled.
    pub unpaid: u64,
    /// E0 of the entry.
    pub e0: i64,
    /// The booked net (estimator, dearer of the dated and today's set).
    pub net: i64,
    /// The dated net (estimator at the dated set).
    pub net_dated: i64,
    /// The per-leg net of the BOOKED reading (estimator with Rs 20 per executed whole-lot order slice, D3).
    pub net_per_leg: i64,
    /// The per-leg net with slices of the RAW freeze (check-idx1s-early-A TABLE 1 only; beside).
    pub net_per_leg_raw: i64,
    /// The S15 bullet 1 net on an expiry day (the leftover sold by its own order at the worst traded price since W0); otherwise
    /// the net of the leftover rule in force. 0 when nothing was bought.
    pub net_bullet1: i64,
    /// The settlement net (0 when the leftover rule is not the expiry-day settlement or nothing is left).
    pub net_settle: i64,
    /// The intrinsic value per unit of the leftover under the settlement reading (paisa; 0 when OTM or no settlement).
    pub intrinsic: i64,
    /// Settled (expiry-day leftovers settled at intrinsic, the worse reading).
    pub settled: bool,
    /// RESUMED WAVE 4: an order of the trip spans a DATA-law hole (the production source's; never on the store source).
    pub hole: bool,
}

/// Both paths and the booked one (index 0 kept, 1 off; booked = the worse net, kept on a tie).
#[derive(Clone, Debug, Default)]
pub struct Priced {
    /// [kept, off].
    pub paths: [PathOut; 2],
    /// The booked path's index.
    pub booked: usize,
}

pub(crate) fn walk1(tape: &Tape<'_, Sec>, o: &Order, want: u64, money: Option<i64>, bp: u16, leg: Leg) -> Result<(Vec<Fill>, LevelFill), String> {
    let dial = Dial::new(&[bp]).map_err(|e| format!("{e:?}"))?;
    let mut w = [0u64; MAXL];
    let mut m: [Option<i64>; MAXL] = [None; MAXL];
    if let (Some(x), Some(y)) = (w.first_mut(), m.first_mut()) {
        *x = want;
        *y = money;
    }
    let mut out = [LevelFill::default(); MAXL];
    let mut v: Vec<Fill> = Vec::new();
    let mut rec = |_l: usize, bf: &BlockFill| {
        if bf.why == Why::Filled && bf.units > 0 {
            v.push((leg, bf.block.at, bf.units, bf.block.vol, bf.px, bf.block.known));
        }
    };
    walk(tape, o, &dial, LevelMask::one(0), &mut w, Some(&m), None, &mut out, Some(&mut rec)).map_err(|e| format!("walk {e:?}"))?;
    Ok((v, out.first().copied().unwrap_or_default()))
}

/// The session segments of a day: [09:15:00, F&O end) split around a derivatives halt.
#[must_use]
pub fn segments(d: Day) -> Vec<Seg> {
    let open = at_sod(d, 9 * 3600 + 15 * 60);
    let end = secs::fno_end(d);
    match secs::halt_der(d) {
        Some((a, z)) => vec![Seg { open, last_sec: a - US_S, auction_open: false }, Seg { open: z, last_sec: end - US_S, auction_open: false }],
        None => vec![Seg { open, last_sec: end - US_S, auction_open: false }],
    }
}

/// The order slices of `units` under the one-order maximum `max_order` (crate S18 `legs::freeze_slices`; R2 / D3): the fewest
/// whole-lot orders at or below the maximum (`max_order` is a whole number of lots). 0 for no units.
#[must_use]
pub fn leg_count(units: u64, max_order: u64) -> u32 {
    u32::try_from(common_fill::legs::freeze_slices(units, max_order)).unwrap_or(u32::MAX)
}

/// Prices one intraday trade on both protection paths.
///
/// # Errors
/// A tape refusal (bad rows), a walk error, or a refused rate day.
pub fn price_intraday(rows: &[Sec], p: &Plan) -> Result<Priced, String> {
    price_intraday_holes(rows, &[], p)
}

/// RESUMED WAVE 4: [`price_intraday`] on the PRODUCTION source's rows with its DATA-law holes ([`crate::newpx::day_rows`]):
/// every order whose blocks span a hole sets the path's `hole` flag (the fills are those of the worse-of-two rows).
///
/// # Errors
/// As [`price_intraday`].
pub fn price_intraday_holes(rows: &[Sec], holes: &[(i64, i64)], p: &Plan) -> Result<Priced, String> {
    let open = at_sod(p.d, 9 * 3600 + 15 * 60);
    let end = secs::fno_end(p.d);
    let s0 = secs::sq0(p.d);
    let segs = segments(p.d);
    let ix = SecIndex::new(rows, open, end).map_err(|e| format!("index {e:?}"))?;
    let tape = Tape::new(rows, &ix, None).and_then(|t| t.with_segments(&segs)).and_then(|t| t.with_block(p.block_s)).map_err(|e| format!("tape {e:?}"))?.with_holes(holes);
    let decided = p.exit.filter(|&x| x < s0);
    let cancel = decided.unwrap_or(s0);
    let (rd, rt) = (oc::rates_on(i64::from(p.d))?, oc::rates_on(oc::today())?);
    let mut out = Priced::default();
    for (pi, kept) in [true, false].into_iter().enumerate() {
        let mut g = PathOut { kept, ..PathOut::default() };
        let en = Order { side: Side::Buy, anchor: p.t, stop: cancel, unit: p.lot, limit: kept.then_some(p.buy_lim), kind: Kind::Entry };
        let (ef, el) = walk1(&tape, &en, p.qty, Some(p.reserve), p.bp, Leg::Entry)?;
        g.fills.extend(ef);
        g.hole = el.hole_span;
        g.units_in = el.units;
        g.value_in = el.value;
        g.unpaid = el.unpaid;
        g.e0 = el.e0;
        let mut rest = el.units;
        let (mut xv, mut sqv) = (0i64, 0i64);
        if let Some(xt) = decided
            && rest > 0
        {
            let sl = secs::last_before(rows, xt).map(|r| limits::protection(r.close, Side::Sell, 1_000, 5));
            let x = Order { side: Side::Sell, anchor: xt, stop: s0, unit: p.lot, limit: if kept { sl } else { None }, kind: Kind::SignalExit };
            let (xf, xl) = walk1(&tape, &x, rest, None, p.bp, Leg::Exit)?;
            g.fills.extend(xf);
            g.hole = g.hole || xl.hole_span;
            g.xu = xl.units;
            xv = xl.value;
            rest -= xl.units;
        }
        if rest > 0 {
            let q = Order { side: Side::Sell, anchor: s0, stop: end, unit: p.lot, limit: None, kind: Kind::SquareOff };
            let (qf, ql) = walk1(&tape, &q, rest, None, p.bp, Leg::SquareOff)?;
            g.fills.extend(qf);
            g.hole = g.hole || ql.hole_span;
            g.squ = ql.units;
            sqv = ql.value;
            rest -= ql.units;
        }
        let worst = |from: i64| rows.iter().filter(|r| r.ts >= from && r.ts < end).map(|r| r.low).min().unwrap_or(5);
        // (intrinsic value x units, notional) of the leftover under the settlement reading (crate S15 bullet 3)
        let mut settle_trip: Option<(i64, i64)> = None;
        if rest > 0 {
            g.un_u = rest;
            match p.leftover {
                Leftover::WorstSinceE0 => g.un_px = worst(g.e0),
                Leftover::Settle { close, strike, ce, w0 } => {
                    // the worse of the settlement (with its exercise charges) and S15 bullet 1, decided on the nets below
                    g.un_px = worst(w0);
                    let u = i64::try_from(rest).unwrap_or(0);
                    let (sv, sn) = oc::settle(close, strike, ce, u);
                    g.intrinsic = if u > 0 { sv / u } else { 0 };
                    settle_trip = Some((sv, sn));
                }
            }
            g.fills.push((Leg::Unclosable, end - US_S, rest, 0, g.un_px, end));
        }
        let rest_value = i64::try_from(g.un_u).unwrap_or(0) * g.un_px;
        g.proceeds = xv + sqv + rest_value;
        // D1: the leftover's own sale order (S15 bullet 1, and the idx1s "unclosable" reading) is an executed exit order; the
        // settlement reading has no such order (REM_SETTLED charges the exercised contract instead).
        // S7 BUILDER wave 4 (MAIN-RULINGS-S7-S10 item 3, D2): the 15:10:00 square-off is the strategy's OWN order (Tickvault places it
        // before Dhan's auto square-off): an executed exit order at Rs 20 + the statutory charges, exactly as the estimator; never Dhan's
        // auto square-off fee. MUTANT m_d2_broker_sq charges it as the BROKER's order (the square-off fee on top: the robot's old reading).
        let (own_sq, broker_sq) = if cfg!(feature = "m_d2_broker_sq") { (0, u32::from(g.squ > 0)) } else { (u32::from(g.squ > 0), 0) };
        let closed_orders = u32::from(g.xu > 0) + own_sq;
        let lo_order = u32::from(g.un_u > 0 && !(cfg!(feature = "m_d1_lo_order") && settle_trip.is_some()));
        // D3: slices of the dated one-order maximum (whole lots); the raw freeze only for TABLE 1 beside
        let slice_by = if cfg!(feature = "m_d3_raw_freeze") { p.freeze } else { p.max_order };
        let sl = |u: u64| leg_count(u, slice_by);
        let sl_raw = |u: u64| leg_count(u, p.freeze);
        if g.units_in > 0 {
            if p.max_order == 0 {
                return Err(format!("no one-order maximum on {} (no dated freeze row: refused by name, F13)", idx_cal::day::fmt(p.d)));
            }
            let qty = i64::try_from(g.units_in).unwrap_or(0);
            let base = Trip { qty, buy_value: g.value_in, sell_value: g.proceeds, entry_orders: 1, exit_orders: closed_orders + lo_order, broker_orders: broker_sq, ..Trip::default() };
            let leg_trip = Trip { entry_orders: sl(g.units_in), exit_orders: sl(g.xu) + sl(g.squ) + sl(g.un_u), ..base };
            let leg_trip_raw = Trip { entry_orders: sl_raw(g.units_in), exit_orders: sl_raw(g.xu) + sl_raw(g.squ) + sl_raw(g.un_u), ..base };
            let net_of = |t: &Trip| -> (i64, i64) {
                let (tot, _, _) = oc::charged(&rd, &rt, t, Reading::Estimator);
                let (b, s, _) = oc::charges_est(&rd, t);
                (t.sell_value - t.buy_value - tot, t.sell_value - t.buy_value - b - s)
            };
            let (n1, d1) = net_of(&base);
            (g.net, g.net_dated, g.net_bullet1) = (n1, d1, n1);
            g.net_per_leg = net_of(&leg_trip).0;
            g.net_per_leg_raw = net_of(&leg_trip_raw).0;
            if let Some((sv, sn)) = settle_trip {
                let st = Trip { sell_value: xv + sqv + sv, exit_orders: closed_orders, rem_kind: oc::REM_SETTLED, settle_value: sv, settle_notional: sn, ..base };
                let st_leg = Trip { entry_orders: sl(g.units_in), exit_orders: sl(g.xu) + sl(g.squ), ..st };
                let st_leg_raw = Trip { entry_orders: sl_raw(g.units_in), exit_orders: sl_raw(g.xu) + sl_raw(g.squ), ..st };
                let (n2, d2) = net_of(&st);
                g.net_settle = n2;
                // the WORSE is booked; a tie books the settlement (what NSE does with an open contract at expiry; pos.rs too)
                if n2 <= n1 {
                    (g.net, g.net_dated, g.settled) = (n2, d2, true);
                    g.proceeds = xv + sqv + sv;
                    g.un_px = g.intrinsic;
                    g.net_per_leg = net_of(&st_leg).0;
                    g.net_per_leg_raw = net_of(&st_leg_raw).0;
                    if let Some(row) = g.fills.last_mut()
                        && row.0 == Leg::Unclosable
                    {
                        *row = (Leg::Settled, end - US_S, g.un_u, 0, g.intrinsic, end);
                    }
                }
            }
        }
        if let Some(slot) = out.paths.get_mut(pi) {
            *slot = g;
        }
    }
    let (n0, n1) = (out.paths.first().map_or(0, |x| x.net), out.paths.get(1).map_or(0, |x| x.net));
    out.booked = usize::from(n1 < n0);
    Ok(out)
}

/// The XC exit plan of one level (s3.1 (4); CHANGES-xc + CHANGES-b1-fix): the flip anchor `flip` (the END of the first
/// index candle closing across Supertrend 10 x 3; None = no flip before the square-off) against the clock 300 s after E0
/// (the START of the level's first filling option block). The clock fires only when E0 + 300 s is strictly before the flip
/// anchor; its anchor is max(E0 + 300 s, E1) (E1 = the first filling block's known end); a clock anchored at or after
/// 15:08:00 is never sent (the B1 cut: the level stays on its signal plan, the 15:10 square-off). Returns (exit anchor or
/// None for the square-off, true when the clock won).
#[must_use]
pub fn xc_plan(d: Day, e0: i64, e1: i64, flip: Option<i64>) -> (Option<i64>, bool) {
    let cut = at_sod(d, 15 * 3600 + 8 * 60);
    if e0 == 0 {
        return (flip, false);
    }
    let clock = e0 + 300 * US_S;
    let a = clock.max(e1);
    let before_flip = flip.is_none_or(|f| clock < f);
    if before_flip && a < cut {
        return (Some(a), true);
    }
    (flip.filter(|&f| f < cut), false)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unreachable, reason = "tests")]
mod tests {
    use super::*;
    use idx_cal::day;

    /// s3.5 (8): an XC clock anchored at exactly 15:08:00 is never sent (B1); at 15:07:59 it is; a flip before the clock wins.
    #[test]
    fn xc_clock_and_the_b1_cut() {
        let d = day::from_civil(2024, 5, 2);
        let at = |h: i64, m: i64, s: i64| at_sod(d, h * 3600 + m * 60 + s);
        // E0 15:03:00, E1 15:03:01 -> clock 15:08:00: at the cut, never sent; no flip -> square-off
        assert_eq!(xc_plan(d, at(15, 3, 0), at(15, 3, 1), None), (None, false));
        // E0 15:02:59 -> clock 15:07:59 < 15:08:00: sent
        assert_eq!(xc_plan(d, at(15, 2, 59), at(15, 3, 0), None), (Some(at(15, 7, 59)), true));
        // a flip at 10:04:00 before the clock (E0 10:00:00 -> clock 10:05:00): the flip
        assert_eq!(xc_plan(d, at(10, 0, 0), at(10, 0, 1), Some(at(10, 4, 0))), (Some(at(10, 4, 0)), false));
        // the clock exactly at the flip anchor does not fire (strictly before)
        assert_eq!(xc_plan(d, at(10, 0, 0), at(10, 0, 1), Some(at(10, 5, 0))), (Some(at(10, 5, 0)), false));
        // E1 later than E0 + 300 s: the anchor is E1
        assert_eq!(xc_plan(d, at(10, 0, 0), at(10, 6, 0), None), (Some(at(10, 6, 0)), true));
    }

    fn sec(d: Day, h: i64, m: i64, s: i64, high: i64, low: i64, vol: u64) -> Sec {
        Sec { ts: at_sod(d, h * 3600 + m * 60 + s), high, low, close: low, vol }
    }

    /// The D1 shape: 1,000 units bought at 10:00:00 at Rs 100.00, no exit decided, the square-off sells 100 at Rs 50.00 at
    /// 15:15:00, so 900 units are left at the F&O end; S15 bullet 1 = the worst LOW since W0 = E0 = Rs 50.00.
    fn d1(d: Day, leftover: Leftover) -> (Vec<Sec>, Plan) {
        let rows = vec![sec(d, 10, 0, 0, 10_000, 9_900, 1_000), sec(d, 15, 15, 0, 5_000, 5_000, 100)];
        let p = Plan { d, t: at_sod(d, 10 * 3600), lot: 50, qty: 1_000, buy_lim: 20_000, reserve: 10_000_000_000, exit: None, bp: 10_000, block_s: 1, leftover, freeze: 1_800, max_order: 1_800 };
        (rows, p)
    }

    fn net_of(d: Day, t: &Trip) -> i64 {
        let (rd, rt) = (oc::rates_on(i64::from(d)).unwrap_or_else(|_| oc::rates_on(oc::today()).unwrap_or_else(|_| unreachable!())), oc::rates_on(oc::today()).unwrap_or_else(|_| unreachable!()));
        t.sell_value - t.buy_value - oc::charged(&rd, &rt, t, Reading::Estimator).0
    }

    fn settle_at(d: Day, close: i64, strike: i64) -> Leftover {
        Leftover::Settle { close, strike, ce: true, w0: at_sod(d, 10 * 3600) }
    }

    /// D1 (a): an ITM leftover on 2019-08-29 (exercise STT on the settlement NOTIONAL): the settlement is the worse reading and
    /// is booked; the leftover row prints the intrinsic value; the settlement trip carries no extra sale order (REM_SETTLED).
    #[test]
    fn d1_itm_leftover_2019_08_29_notional_stt_settlement_worse() {
        let d = day::from_civil(2019, 8, 29);
        assert!(oc::rates_on(i64::from(d)).is_ok_and(|r| r.stt_ex_on_notional), "2019-08-29: exercise STT on the notional");
        let (rows, p) = d1(d, settle_at(d, 1_100_000, 1_099_500));
        let pr = price_intraday(&rows, &p).unwrap_or_default();
        let g = &pr.paths[pr.booked];
        assert_eq!((g.units_in, g.squ, g.un_u), (1_000, 100, 900));
        assert!(g.settled, "settlement (Rs 5.00 intrinsic, STT on Rs 99 lakh notional) is worse than bullet 1 (Rs 50.00)");
        assert_eq!(g.intrinsic, 500);
        assert_eq!(g.fills.last().copied(), Some((Leg::Settled, at_sod(d, 15 * 3600 + 30 * 60) - US_S, 900, 0, 500, at_sod(d, 15 * 3600 + 30 * 60))));
        let st = Trip { qty: 1_000, buy_value: 10_000_000, sell_value: 500_000 + 450_000, entry_orders: 1, exit_orders: 1, rem_kind: oc::REM_SETTLED, settle_value: 450_000, settle_notional: 990_000_000, ..Trip::default() };
        assert_eq!(g.net, net_of(d, &st));
        assert_eq!(g.net, g.net_settle);
        assert!(g.net_settle < g.net_bullet1);
    }

    /// D1 (b): an OTM leftover settles at 0 (+ Rs 20 on the expired contract): worse than bullet 1, booked, the row at 0.
    #[test]
    fn d1_otm_leftover_settles_at_zero_and_is_booked() {
        let d = day::from_civil(2024, 5, 2);
        let (rows, p) = d1(d, settle_at(d, 1_990_000, 2_000_000));
        let pr = price_intraday(&rows, &p).unwrap_or_default();
        let g = &pr.paths[pr.booked];
        assert!(g.settled);
        assert_eq!((g.intrinsic, g.un_px), (0, 0));
        assert_eq!(g.fills.last().map(|f| (f.0, f.2, f.4)), Some((Leg::Settled, 900, 0)));
        let st = Trip { qty: 1_000, buy_value: 10_000_000, sell_value: 500_000, entry_orders: 1, exit_orders: 1, rem_kind: oc::REM_SETTLED, ..Trip::default() };
        assert_eq!(g.net, net_of(d, &st));
    }

    /// D1 (c): bullet 1 is the worse reading (intrinsic Rs 300.00 against Rs 50.00): booked, and it carries the leftover's OWN
    /// sale order (Rs 20 + 18% GST = Rs 23.60 more than without it). Red under `m_d1_lo_order`.
    #[test]
    fn d1_bullet1_worse_carries_its_own_sale_order() {
        let d = day::from_civil(2024, 5, 2);
        let (rows, p) = d1(d, settle_at(d, 2_030_000, 2_000_000));
        let pr = price_intraday(&rows, &p).unwrap_or_default();
        let g = &pr.paths[pr.booked];
        assert!(!g.settled);
        assert_eq!(g.fills.last().map(|f| (f.0, f.2, f.4)), Some((Leg::Unclosable, 900, 5_000)));
        let b1 = Trip { qty: 1_000, buy_value: 10_000_000, sell_value: 500_000 + 4_500_000, entry_orders: 1, exit_orders: 2, ..Trip::default() };
        let without = Trip { exit_orders: 1, ..b1 };
        assert_eq!(net_of(d, &without) - net_of(d, &b1), 2_360, "Rs 20 + GST");
        assert_eq!(g.net, net_of(d, &b1), "the bullet 1 reading sells the leftover by its own order");
        assert_eq!(g.net, g.net_bullet1);
        assert!(g.net_settle > g.net_bullet1);
    }

    /// D1 (d): an expiry day after 2026-04-01: the exercise STT at 0.15% on the intrinsic value; the settlement booked.
    #[test]
    fn d1_settlement_2026_04_07_at_the_dated_0_15_pct() {
        let d = day::from_civil(2026, 4, 7);
        let r = oc::rates_on(i64::from(d)).unwrap_or_else(|_| unreachable!());
        let r0 = oc::rates_on(i64::from(day::from_civil(2026, 3, 31))).unwrap_or_else(|_| unreachable!());
        assert_eq!((r.stt_ex, r0.stt_ex, r.stt_ex_on_notional), (1_500_000, 1_250_000, false));
        let (rows, p) = d1(d, settle_at(d, 2_400_200, 2_400_000));
        let pr = price_intraday(&rows, &p).unwrap_or_default();
        let g = &pr.paths[pr.booked];
        assert!(g.settled);
        assert_eq!(g.intrinsic, 200);
        let st = Trip { qty: 1_000, buy_value: 10_000_000, sell_value: 500_000 + 180_000, entry_orders: 1, exit_orders: 1, rem_kind: oc::REM_SETTLED, settle_value: 180_000, settle_notional: 2_160_180_000, ..Trip::default() };
        assert_eq!(g.net, net_of(d, &st));
    }

    /// D3 (R2): the per-leg reading slices by the whole-lot one-order maximum: NIFTY lot 65 at freeze 1,800 (max 1,755):
    /// 3,575 units = 3 orders (2 by the raw freeze); BANKNIFTY lot 35 at freeze 900 (max 875): 1,785 units = 3 (2 raw).
    /// Red under `m_d3_raw_freeze` (the per-leg net then equals the raw-freeze net).
    #[test]
    fn d3_per_leg_slices_by_the_one_order_maximum() {
        assert_eq!((leg_count(3_575, 1_755), leg_count(3_575, 1_800)), (3, 2));
        assert_eq!((leg_count(1_785, 875), leg_count(1_785, 900)), (3, 2));
        assert_eq!((leg_count(0, 875), leg_count(875, 875), leg_count(876, 875)), (0, 1, 2));
        let d = day::from_civil(2025, 9, 15);
        let rows = vec![sec(d, 10, 0, 0, 50_000, 49_000, 2_000), sec(d, 11, 0, 0, 52_000, 51_000, 2_000)];
        let p = Plan { d, t: at_sod(d, 10 * 3600), lot: 35, qty: 1_785, buy_lim: 60_000, reserve: 10_000_000_000, exit: Some(at_sod(d, 11 * 3600)), bp: 10_000, block_s: 1, leftover: Leftover::WorstSinceE0, freeze: 900, max_order: 875 };
        let pr = price_intraday(&rows, &p).unwrap_or_default();
        let g = &pr.paths[pr.booked];
        assert_eq!((g.units_in, g.xu, g.un_u), (1_785, 1_785, 0));
        // 3 + 3 order slices against 2 + 2: two more Rs 20 + GST
        assert_eq!(g.net_per_leg_raw - g.net_per_leg, 2 * 2_360);
        // the real dated table: BANKNIFTY 900 from 2025-09-01 with lot 35 -> 875; NIFTY 1,800 with lot 65 -> 1,755
        let text = std::fs::read_to_string("/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/nse-freeze-20261114/FREEZE-TABLE-NIFTY-BANKNIFTY.tsv").unwrap_or_default();
        let f = crate::freeze::FreezeTable::parse(&text).unwrap_or_default();
        assert_eq!(f.max_one_order("BANKNIFTY", d, 35), Some(875));
        assert_eq!(f.max_one_order("BANKNIFTY", day::from_civil(2025, 7, 1), 35), Some(595));
        assert_eq!(f.max_one_order("NIFTY", day::from_civil(2026, 1, 5), 65), Some(1_755));
        // a plan without a one-order maximum is refused by name (F13)
        assert!(price_intraday(&rows, &Plan { max_order: 0, ..p }).is_err());
    }

    /// s3.5 (12) / B5: the square-off runs to 15:40:00 from 2026-08-03 and to 15:30:00 before.
    #[test]
    fn dated_fno_end() {
        assert_eq!(secs::fno_end(day::from_civil(2026, 8, 4)) - at_sod(day::from_civil(2026, 8, 4), 0), (15 * 3600 + 40 * 60) * US_S);
        assert_eq!(secs::fno_end(day::from_civil(2026, 7, 31)) - at_sod(day::from_civil(2026, 7, 31), 0), (15 * 3600 + 30 * 60) * US_S);
    }
}
