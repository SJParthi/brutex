//! S6 positional half (wave 4; SNIPER-IDXOPT-PLAN-v3.1 s5.2, MAIN-RULINGS-S7-S10 item 2): THE ONE ACCOUNT RE-WALK for every
//! multi-stream account (COMBINED long + short, MIXED NIFTY + BANKNIFTY, later the s26 portfolios), here for INTRADAY streams
//! priced by [`crate::price::price_intraday`] (the positional legs come from the S8b engine through the same ledger rules).
//!
//! **The ledger (MAIN's ruling, 15 Nov 03:36):** lost money is never reused and profits are never added: the BASE is Rs 20 L plus
//! the realised net when it is negative (Rs 20 L minus net losses so far); every stream's SLOT is base / N (W4 REPAIR D-W4-3: read
//! from ONE place, `SlotRule::default()` with the one formula `SlotRule::slot`, the same rule as `opt_pos::pos`). Money is released when
//! a trade's LAST fill is known. At one instant every release is settled FIRST, then the entries are decided in stream order
//! (LONG before SHORT; NIFTY before BANKNIFTY), each against `free = base - premium still held`. An entry's quantity is crate
//! C11 `size_units_closest(slot, reference, lot, free)` (`acct_over_cap = cash_short`: the largest count that fits; a count of 0
//! is refused and counted); its payable exposure (REQ 70a) is `min(qty x buy limit, free)` (Q17: never borrowed).
//!
//! **Reuse or re-price:** a trade whose account quantity, contract and first block equal the stream's STORED leg (the stream
//! priced alone in its own account, [`single`]) reuses the stored rows byte for byte; any other is re-priced through `opt_pos`
//! at the account's quantity (`cash_short n of m`). P-ACCT1: a stream re-walked alone equals its stored rows; P-ACCT2: forced
//! cash_short cases equal an independent node re-walk. The shared-contract split (s5.2 "Shared seconds") never arises in
//! COMBINED (CE against PE) or MIXED (NIFTY against BANKNIFTY); it is NOT built here (named for the s26 design).

use common_fill::limits::size_units_closest;
use idx_cal::day::Day;

use crate::pos::SlotRule;
use crate::price::{self, Leftover, Plan, Priced};
use crate::secs::Sec;

/// One decision of a stream (from a pick's stored decisions).
#[derive(Clone, Debug)]
pub struct Decision {
    /// The trade's number in its stream.
    pub no: String,
    /// The index.
    pub ix: String,
    /// The session.
    pub d: Day,
    /// The decision instant (the entry anchor).
    pub t: i64,
    /// The contract folder.
    pub contract: String,
    /// Its lot.
    pub lot: u64,
    /// The size reference (C4: HIGH of the last traded second before t), paisa.
    pub reference: i64,
    /// The buy protection limit (Dhan 10%, from the last print), paisa.
    pub buy_lim: i64,
    /// The exit anchor (None: the square-off).
    pub exit: Option<i64>,
    /// The share dial, bp.
    pub bp: u16,
    /// The raw dated freeze and the one-order maximum (per-leg readings).
    pub freeze: u64,
    /// See `freeze`.
    pub max_order: u64,
}

/// One booked trade of an account.
#[derive(Clone, Debug, Default)]
pub struct Booked {
    /// The stream.
    pub stream: usize,
    /// The trade number.
    pub no: String,
    /// The slot rule's count (the slot alone, no free-cash cut).
    pub want: u64,
    /// The account's count.
    pub qty: u64,
    /// The count was cut by free cash (`cash_short`).
    pub cash_short: bool,
    /// Refused (0 lots fit).
    pub refused: bool,
    /// The stored rows were reused byte for byte.
    pub reused: bool,
    /// The payable exposure passed to the walk.
    pub reserve: i64,
    /// The priced trade (both paths; the booked one).
    pub priced: Priced,
    /// The instant the trade's money is released (its last fill known).
    pub release: i64,
}

impl Booked {
    /// The booked path's net.
    #[must_use]
    pub fn net(&self) -> i64 {
        self.priced.paths.get(self.priced.booked).map_or(0, |p| p.net)
    }
    /// The booked path's premium paid.
    #[must_use]
    pub fn value_in(&self) -> i64 {
        self.priced.paths.get(self.priced.booked).map_or(0, |p| p.value_in)
    }
}

/// The account: the starting base (Rs 20 L) and each stream's N (slot = base / N).
#[derive(Clone, Debug)]
pub struct Account {
    /// Rs 20,00,000 in paisa.
    pub base0: i64,
    /// N per stream.
    pub n: Vec<i64>,
}

fn release_of(p: &Priced, t: i64) -> i64 {
    p.paths.get(p.booked).map_or(t, |g| g.fills.iter().map(|f| f.5).max().unwrap_or(t))
}

fn price_at(rows: &[Sec], dc: &Decision, qty: u64, reserve: i64) -> Result<Priced, String> {
    let plan = Plan { d: dc.d, t: dc.t, lot: dc.lot, qty, buy_lim: dc.buy_lim, reserve, exit: dc.exit, bp: dc.bp, block_s: 1, leftover: Leftover::WorstSinceE0, freeze: dc.freeze, max_order: dc.max_order };
    price::price_intraday(rows, &plan)
}

/// The ledger state.
#[derive(Clone, Debug, Default)]
struct Ledger {
    realised: i64,
    /// (release instant, premium held, net) of trades not yet released.
    open: Vec<(i64, i64, i64)>,
}

impl Ledger {
    /// Settles every trade whose money is released by `t` (releases first).
    fn settle(&mut self, t: i64) {
        let mut keep = Vec::with_capacity(self.open.len());
        for o in self.open.drain(..) {
            if o.0 <= t {
                self.realised += o.2;
            } else {
                keep.push(o);
            }
        }
        self.open = keep;
    }
    fn base(&self, base0: i64) -> i64 {
        base0 + self.realised.min(0)
    }
    fn free(&self, base0: i64) -> i64 {
        self.base(base0) - self.open.iter().map(|o| o.1).sum::<i64>()
    }
}

/// Decides and prices one entry against the ledger.
fn enter<F>(lg: &mut Ledger, acc: &Account, stream: usize, dc: &Decision, rows_of: &mut F, stored: Option<&Booked>) -> Result<Booked, String>
where
    F: FnMut(&str, &str, Day) -> Result<Vec<Sec>, String>,
{
    lg.settle(dc.t);
    let n = acc.n.get(stream).copied().unwrap_or(1).max(1);
    let free = lg.free(acc.base0).max(0);
    // W4 REPAIR (check-wave4-S9 D-W4-3): THE one slot rule of the account, read from ONE place (`SlotRule::default()` and the one
    // formula `SlotRule::slot`, shared with `opt_pos::pos`): base / N, base = Rs 20 L + min(0, the net released so far)
    let slot = SlotRule::default().slot(acc.base0, n, lg.realised);
    let (want, _) = size_units_closest(slot, Some(dc.reference), dc.lot, None).map_err(|e| format!("C11 {e:?}"))?;
    let (qty, why) = size_units_closest(slot, Some(dc.reference), dc.lot, Some(free)).map_err(|e| format!("C11 {e:?}"))?;
    let mut b = Booked { stream, no: dc.no.clone(), want, qty, cash_short: qty < want, refused: qty == 0 || why.refused(), ..Booked::default() };
    if b.refused {
        b.qty = 0;
        b.release = dc.t;
        return Ok(b);
    }
    b.reserve = (i64::try_from(qty).unwrap_or(i64::MAX).saturating_mul(dc.buy_lim)).min(free);
    match stored {
        Some(s) if s.qty == qty && s.reserve == b.reserve => {
            b.priced = s.priced.clone();
            b.reused = true;
        }
        _ => {
            let rows = rows_of(&dc.ix, &dc.contract, dc.d)?;
            b.priced = price_at(&rows, dc, qty, b.reserve)?;
        }
    }
    b.release = release_of(&b.priced, dc.t);
    lg.open.push((b.release, b.value_in(), b.net()));
    Ok(b)
}

/// The STORED pick of one stream: its decisions priced in order in the stream's OWN account (the same ledger rules, one stream).
///
/// # Errors
/// A store or pricing error.
pub fn single<F>(decisions: &[Decision], base0: i64, n: i64, rows_of: &mut F) -> Result<Vec<Booked>, String>
where
    F: FnMut(&str, &str, Day) -> Result<Vec<Sec>, String>,
{
    let acc = Account { base0, n: vec![n] };
    let mut lg = Ledger::default();
    let mut out = Vec::with_capacity(decisions.len());
    for dc in decisions {
        out.push(enter(&mut lg, &acc, 0, dc, rows_of, None)?);
    }
    Ok(out)
}

/// THE ONE ACCOUNT RE-WALK: every stream's decisions in one account (event order: the instant, then the stream's order), each
/// entry reusing its stream's stored row when the account gives the same count (and payable exposure), else re-priced.
///
/// # Errors
/// A store or pricing error.
pub fn rewalk<F>(streams: &[Vec<Decision>], stored: &[Vec<Booked>], acc: &Account, rows_of: &mut F) -> Result<Vec<Booked>, String>
where
    F: FnMut(&str, &str, Day) -> Result<Vec<Sec>, String>,
{
    let mut ev: Vec<(i64, usize, usize)> = Vec::new();
    for (s, v) in streams.iter().enumerate() {
        for (k, dc) in v.iter().enumerate() {
            ev.push((dc.t, s, k));
        }
    }
    ev.sort_unstable();
    let mut lg = Ledger::default();
    let mut out = Vec::with_capacity(ev.len());
    for (_, s, k) in ev {
        let Some(dc) = streams.get(s).and_then(|v| v.get(k)) else { continue };
        let st = stored.get(s).and_then(|v| v.get(k));
        out.push(enter(&mut lg, acc, s, dc, rows_of, st)?);
    }
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;
    use idx_cal::day;
    use idx_cal::storeio::at_sod;

    fn sec(d: Day, h: i64, m: i64, s: i64, px: i64, vol: u64) -> Sec {
        Sec { ts: at_sod(d, h * 3600 + m * 60 + s), high: px, low: px, close: px, vol }
    }

    fn dc(d: Day, no: &str, h: i64, ix: &str) -> Decision {
        Decision { no: no.into(), ix: ix.into(), d, t: at_sod(d, h * 3600), contract: "C".into(), lot: 50, reference: 10_000, buy_lim: 11_000, exit: Some(at_sod(d, h * 3600 + 1800)), bp: 10_000, freeze: 1_800, max_order: 1_800 }
    }

    /// The ledger rules on a synthetic day: stream 1's entry while stream 0's position is open gets only the free cash
    /// (cash_short); after stream 0's loss is released the base (and the slot) shrink; profits never grow it; a stream re-walked
    /// alone reuses every stored row (P-ACCT1 in miniature).
    #[test]
    fn ledger_base_q7_cash_short_and_reuse() {
        let d = day::from_civil(2024, 5, 2);
        // the contract trades Rs 100 from 10:00, Rs 60 from 10:30 (a loss), one print a second to 12:00
        let mut rows: Vec<Sec> = (0..7200i64)
            .map(|k| {
                let mut s = sec(d, 10, 0, 0, 10_000, 100_000);
                s.ts += k * 1_000_000;
                s
            })
            .collect();
        for s in rows.iter_mut() {
            if s.ts >= at_sod(d, 10 * 3600 + 1800) {
                s.high = 6_000;
                s.low = 6_000;
                s.close = 6_000;
            }
        }
        let mut rows_of = |_: &str, _: &str, _: Day| -> Result<Vec<Sec>, String> { Ok(rows.clone()) };
        let a = vec![dc(d, "1", 10, "NIFTY")];
        let mut b0 = dc(d, "1", 10, "BANKNIFTY");
        b0.t += 1_000_000; // one second later, same position window
        let b = vec![b0];
        let acc = Account { base0: 200_000_000, n: vec![1, 1] };
        let sa = single(&a, acc.base0, 1, &mut rows_of).unwrap();
        let sb = single(&b, acc.base0, 1, &mut rows_of).unwrap();
        assert!(!sa[0].cash_short && sa[0].qty > 0);
        // alone: reused byte for byte
        let alone = rewalk(std::slice::from_ref(&a), std::slice::from_ref(&sa), &Account { base0: acc.base0, n: vec![1] }, &mut rows_of).unwrap();
        assert!(alone[0].reused);
        assert_eq!(alone[0].priced.paths[0].fills, sa[0].priced.paths[0].fills);
        // together at N = 1: the second entry meets the first's premium held: cash_short, re-priced
        let both = rewalk(&[a, b], &[sa, sb.clone()], &acc, &mut rows_of).unwrap();
        assert!(both[0].reused);
        assert!(both[1].cash_short && !both[1].reused && both[1].qty < sb[0].qty, "{:?} vs {:?}", both[1].qty, sb[0].qty);
    }
}
