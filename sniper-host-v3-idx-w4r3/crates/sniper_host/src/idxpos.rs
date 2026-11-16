//! S7 BUILDER (wave 4, main 5b507203, 15 Nov 2026): the INDEX POSITIONAL mode (SNIPER-IDXOPT-PLAN-v3.1 s4; plan s8 S7 "positional mode =
//! lane B step 6 as repaired"), NIFTY / BANKNIFTY options held overnight:
//!
//! * the GROUPS are lane B's (`crate::posgroups`, copied byte for byte): the Supertrend 10 x 3 stream of the decision timeframe over every
//!   index candle, the exit rules XS (headline) / FLIP (beside) across nights, P-MONO, FIRST per group = one position per side at a time;
//!   the index has no K13 event and no CA close (no caps but the data end);
//! * the ENTRIES: index candle ENDs strictly before the index close 15:30:00 (B5; `cands::build` with that cut), the Supertrend ready
//!   (`pos_st_unready` a DECIDE refusal, as lane B's R4);
//! * the MONEY: `opt_pos::pos::run` per candidate, one stream, at every N of the 15-value grid, every share level and both protection paths
//!   (the worse booked, IDX-R3 (i)): the A2 / s23a entry, the 15:00:00 roll on the TRUE expiry day with the SEQUENTIAL open (A3), close_split
//!   (B1), carry, settlement with the B6 window (S8b, check-build-idx-S7-S10 D-S8-1 / D-S8-2 fixed), on the S5 pack's production seconds;
//! * the REFUSAL CLASSES (CRITIC-POSITIONAL B2, plan s4.3a): DECIDE refusals (known at the entry: `no_itm_fit` at T, `pos_st_unready`) may
//!   move FIRST; LATER facts (`roll_no_itm`, `roll_refused_cash`, `roll_close_incomplete`, `roll_t2_at_index_close`, a HOLE, the data end)
//!   NEVER remove the row: they are flags (mutant `m_pos_future_refusal` removes such a row: look-ahead);
//! * the BOOKING (6.1, s4.3a B3): a group books at the running maximum over groups of the latest session any of its priced rows is last
//!   known (every N and level); `Outcome.mk` = that session's month (mutant `m_pos_mk`: the entry month);
//! * the OUT-POS row: lane B's layout (`record_width_pos(15, 5)` = 792 B): nets i64 and run-downs u16 per (N, level), the last-known second,
//!   the flag word u32, the entry's first block, the exits per level, rolls, nights, the entry and exit groups;
//! * downstream: `Layout::new_pos` (B4: leaves from the groups' booking months, counted months from the sessions), `Row.sess` = the group,
//!   `Row.k` = the ordinal (u32, B5), the sweep with the group count as its sessions (FU-10), the re-walk, common_wf under
//!   WF-RULES-SNIPER-POS-v1.tsv (ARRIVAL, carried yes, marks none: lane B's file), the twin per group, the HARD gate at 1x / 10x / 100x.
//!
//! The DATA END (plan s4.3a B2 / s4.4 / s4.7 (15); W4 REPAIR check-wave4-S9 D-W4-1): a group with no exit before the data end is priced
//! with NO exit decision (the sentinel `opt_pos::pos::DATA_END`; the final4 stand-in exit at the last index close is gone): every unit
//! still held after the last session is `open_at_data_end` in `opt_pos::pos`, valued WHOLE at the worst print of the last session (W0 =
//! E0 when bought that session, else its first block), one sale order charged, flagged OPEN_AT_DATA_END, never refused, never an error;
//! an exit or carry on the last session whose rest cannot sell keeps the same valuation. Every positional pick line (SN-PICKS / SN-REPORT)
//! prints the data-end test-year word (2026) WITH and WITHOUT the open_at_data_end trades.
//!
//! W4 REPAIR D-W4-2: S1's MUHURAT-LIST is a data row (`muhurat_file`): a carried exit rest is held through a Muhurat session (s4.4 / Q39).
//! W4 REPAIR D-W4-3: the slot rule is `SlotRule::default()` = base / N (MAIN-RULINGS-S7-S10 item 2; the one rule of `opt_pos::pos` and
//! `opt_pos::acct`), named on SN1 and on every pick line; the fixed Rs 20 L / N slot is the beside reading (G-ROLL beside run).
//!
//! W4 REPAIR r3 (check-wave4-S9-r2.md; MAIN ruling of 16 Nov 2026 ~14:3x IST, main a3019c01):
//! * D-R2-1: a data-end valuation that read a DATA-law hole (`opt_pos::pos::TripOut::hole`, e.g. the 2026-10-30 contract-days that
//!   DP-NEWPX-FAILS lists `unchecked`) is a hole trade: `level_of` sets HOLE from it as from a leg, so the pick line's holes count and
//!   common_wf's `Trade.hole` (the AFR-2 second reading: winning hole trades left out) carry it; the count of such valuations prints beside
//!   the "legs on a DATA-law hole" line. Mutant `m_idx_data_end_nohole` (the legs only + opt_pos's span test skipped).
//! * D-W4-3 item 3: the FIXED-SLOT reading is PRINTED BESIDE every positional pick (never the headline): the pick's own rows re-priced at
//!   its N and level under `SlotRule::Fixed` on both protection paths, re-walked through common_wf, "fixed-slot beside (fixed Rs 20 L / N):
//!   <the pick's test-year words>" on the pick's lines, three SN-PICKS columns, every re-priced cell in FIXED-BESIDE-POS-* and the SN1
//!   row `slot_rule_beside`.
//!
//! Not built here (named on every run): lane B's DETAIL mode / P7, the netted view, the MTM drawdown beside, the R16 account walk, the
//! FLIP comparison of XS picks, the shared-contract split (s26 only).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use common_fill::US_S;
use common_wf::views::{Role, Ticket, Tickets, crit_val};
use common_wf::{Acc, Code as WfCode, Partition, PickWin, ReadSet, Trade, Verdict, ViewId};
use opt_pos::pos::{self as opos, OpenRule, RunOut, SlotRule, Spec, Stream};
use sniper_kernel::code::Code;
use sniper_kernel::line::Row;
use wf_sweep::acc::{Layout, Outcome};
use wf_sweep::lists::Lists;

use crate::cands::Family;
use crate::esn1::Inputs;
use crate::idx::IdxEnv;
use crate::posgroups::{Exit, Group, Rule, StC};
use crate::search::{Combo, CtxOut, Decoder, RowOut, RunSpec, Slice, Table, combo_rows, pct_of, test_figures};

/// The positional flag word (u32; the low 9 bits are the cash bits, so `Trade.hole` reads bit 4 as on the cash rows).
pub mod pflag {
    /// Units left at an expiry close valued at S15 bullet 1 (not settled).
    pub const UNCLOSABLE: u32 = 2;
    /// An order spans a DATA-law hole (AFR-2).
    pub const HOLE: u32 = 4;
    /// The entry filled 0 units.
    pub const ZERO: u32 = 8;
    /// REQ 70a units cancelled on an entry or roll-open.
    pub const PROTECTED: u32 = 128;
    /// C11 cut by free cash (`cash_short`).
    pub const ACCOUNT_CAP: u32 = 256;
    /// Leftovers SETTLED at intrinsic (the worse reading).
    pub const SETTLED: u32 = 512;
    /// The protection-OFF path was booked.
    pub const OFF_BOOKED: u32 = 1024;
    /// The slot buys no whole lot at this N (no trade at this N; net 0).
    pub const ZERO_QTY: u32 = 2048;
    /// A roll's open leg found no ITM fit (`roll_no_itm`; the trade ends at the roll close).
    pub const ROLL_NO_ITM: u32 = 1 << 12;
    /// A roll's open leg refused for cash.
    pub const ROLL_REFUSED_CASH: u32 = 1 << 13;
    /// A roll close unfinished at the index close (no open leg).
    pub const ROLL_CLOSE_INCOMPLETE: u32 = 1 << 14;
    /// A close leg split by the one-order maximum (B1).
    pub const CLOSE_SPLIT: u32 = 1 << 15;
    /// The data end (plan s4.3a B2; W4 REPAIR D-W4-1): units still held after the last session were valued at its worst print
    /// (`opt_pos::pos` `open_at_data_end`: the whole position of a group with no exit before the data end, or the unsold rest of an
    /// exit or carry on the last session). Set from the pricer's outcome only (a data-end group's row closed earlier, e.g. at a roll
    /// with no ITM fit, is a closed trade).
    pub const OPEN_AT_DATA_END: u32 = 1 << 16;
    /// An exit's unfilled rest carried to the next session.
    pub const CARRY: u32 = 1 << 17;
    /// A buy order cut by the exit decision (D-S8-1) or a pending open cancelled by the flip.
    pub const CUT_BY_EXIT: u32 = 1 << 18;
    /// The joined run-down left the u16 bp width and reads `u16::MAX` (lane B's RD_SATURATED: a roll trip larger than the first entry
    /// can lose more than 655.35 % of the FIRST premium).
    pub const RD_SATURATED: u32 = 1 << 19;
    /// The cash-bit part of a flag word.
    #[must_use]
    pub const fn cash_bits(f: u32) -> u16 {
        (f & 0x0FFF) as u16
    }
}

/// The refusal class of a priced positional row (CRITIC-POSITIONAL B2): `Decide` rows are refused (known at the entry); everything
/// else is `Keep` (LATER facts are flags, never a refusal). Mutant `m_pos_future_refusal` refuses a row carrying a LATER flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Refused by name, before any money (known at T): the row may move FIRST.
    Decide(&'static str),
    /// Priced and kept (LATER facts flagged).
    Keep,
}

/// The LATER flags of a stream's counted facts (s4.3a B2).
pub const LATER_FLAGS: [&str; 7] = ["roll_no_itm", "roll_refused_cash", "roll_close_incomplete", "roll_t2_at_index_close", "roll_wait_close_incomplete", "open_after_run", "open_at_data_end"];

/// The leftover bits of a stream outcome: UNCLOSABLE (an expiry leftover valued at S15 bullet 1, not settled), SETTLED (settled at
/// intrinsic, the worse reading), OPEN_AT_DATA_END (units held after the last session, valued at its worst print: never UNCLOSABLE,
/// never SETTLED).
#[must_use]
pub fn leftover_bits(o: &opos::StreamOut) -> u32 {
    let mut f = 0u32;
    if o.trips.iter().any(|t| t.leftover > 0 && !t.settled && !t.data_end) {
        f |= pflag::UNCLOSABLE;
    }
    if o.trips.iter().any(|t| t.leftover > 0 && t.settled) {
        f |= pflag::SETTLED;
    }
    if o.trips.iter().any(|t| t.data_end) || o.flags.get("open_at_data_end").copied().unwrap_or(0) > 0 {
        f |= pflag::OPEN_AT_DATA_END;
    }
    f
}

/// The class of one stream outcome (at the proxy N and level): an entry refused at T is DECIDE; a LATER flag keeps the row (the mutant
/// refuses it, moving FIRST to a later candle).
#[must_use]
pub fn classify(o: &opos::StreamOut, hole: bool) -> Class {
    if o.legs.first().is_some_and(|l| l.kind == "entry_refused") {
        return Class::Decide("no_itm_fit");
    }
    if cfg!(feature = "m_pos_future_refusal") && (hole || LATER_FLAGS.iter().any(|k| o.flags.get(k).copied().unwrap_or(0) > 0)) {
        // MUTANT m_pos_future_refusal: a fact known only AFTER the entry removes the row (look-ahead: FIRST moves to a later candle).
        return Class::Decide("later_fact");
    }
    Class::Keep
}

/// The booking sessions (6.1): per group the running maximum, in group order, of the latest session any of its priced rows is last known
/// (`bg`, a session index per group) and its own entry session (`entry_sess`). Mutant `m_pos_mk`: each group books at its ENTRY session.
#[must_use]
pub fn booking(bg: &[usize], entry_sess: &[usize]) -> Vec<usize> {
    let mut out = Vec::with_capacity(bg.len());
    let mut run_max = 0usize;
    for (g, &b) in bg.iter().enumerate() {
        let e = entry_sess.get(g).copied().unwrap_or(0);
        if cfg!(feature = "m_pos_mk") {
            out.push(e);
            continue;
        }
        run_max = run_max.max(b.max(e));
        out.push(run_max);
    }
    out
}

/// The OUT-POS record width (lane B's `record_width_pos`): `nn` N x `nlev` levels x (i64 net + u16 run-down) + 39 B, padded to 8.
#[must_use]
pub const fn record_width_pos(nn: usize, nlev: usize) -> usize {
    (nn * nlev * 10 + 39).div_ceil(8) * 8
}

/// One (N, level) of a positional row, booked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PLevel {
    /// The booked net (sum of the trips' estimator nets), paisa.
    pub net: i64,
    /// The run-down, bp of the first entry value.
    pub rd_bp: u16,
    /// The last fill known (the trade's money released).
    pub last_known: i64,
    /// Flags ([`pflag`]).
    pub flags: u32,
    /// Units of the first entry.
    pub units: u64,
    /// Its value.
    pub entry_value: i64,
    /// Every sale's value (+ leftover valuations).
    pub exit_value: i64,
    /// Rolls made.
    pub rolls: u8,
    /// Nights held.
    pub nights: u16,
}

fn encode_pos(lv: &[Vec<PLevel>], entry_first: i64, origin: i64, gids: (u32, u32)) -> Result<Vec<u8>, String> {
    let nlev = lv.first().map_or(0, Vec::len);
    let mut b = Vec::with_capacity(record_width_pos(lv.len(), nlev));
    for lvs in lv {
        for x in lvs {
            b.extend_from_slice(&x.net.to_le_bytes());
        }
    }
    for lvs in lv {
        for x in lvs {
            b.extend_from_slice(&x.rd_bp.to_le_bytes());
        }
    }
    let secs = |u: i64| -> Result<u32, String> { if u == 0 { Ok(0) } else { u32::try_from((u - origin) / US_S).map_err(|_| "OUT-POS seconds".to_string()) } };
    let first = lv.first();
    let last = first.map_or(0, |v| v.iter().map(|x| x.last_known).max().unwrap_or(0));
    b.extend_from_slice(&secs(last)?.to_le_bytes());
    let flags = lv.iter().flatten().fold(0u32, |a, x| a | x.flags);
    b.extend_from_slice(&flags.to_le_bytes());
    b.extend_from_slice(&secs(entry_first)?.to_le_bytes());
    for l in 0..5 {
        let x = first.and_then(|v| v.get(l)).copied().unwrap_or_default();
        let rel = if x.last_known == 0 || entry_first == 0 { 0 } else { u32::try_from(((x.last_known - entry_first) / US_S).max(0)).map_err(|_| "OUT-POS exit seconds")? };
        b.extend_from_slice(&rel.to_le_bytes());
    }
    let x0 = first.and_then(|v| v.first()).copied().unwrap_or_default();
    b.push(x0.rolls);
    b.extend_from_slice(&x0.nights.to_le_bytes());
    let id16 = |g: u32| u16::try_from(g).unwrap_or(u16::MAX);
    b.extend_from_slice(&id16(gids.0).to_le_bytes());
    b.extend_from_slice(&id16(gids.1).to_le_bytes());
    while b.len() % 8 != 0 {
        b.push(0);
    }
    Ok(b)
}

/// A range-min over one contract-day's LOWs (the run-down's per-day reading; a sparse table, O(1) a query after O(n log n) once).
struct DayMin {
    ts: Vec<i64>,
    tab: Vec<Vec<i64>>,
}

impl DayMin {
    fn new(rows: &[opt_pos::secs::Sec]) -> Self {
        let ts: Vec<i64> = rows.iter().map(|r| r.ts).collect();
        let mut tab = vec![rows.iter().map(|r| r.low).collect::<Vec<i64>>()];
        let mut w = 1usize;
        while 2 * w <= ts.len() {
            let prev = tab.last().cloned().unwrap_or_default();
            // prev holds the windows of length w (prev.len() = n - w + 1): the windows of 2w are min(prev[i], prev[i + w]), n - 2w + 1 of them
            let next: Vec<i64> = (0..prev.len().saturating_sub(w)).map(|i| prev.get(i).copied().unwrap_or(i64::MAX).min(prev.get(i + w).copied().unwrap_or(i64::MAX))).collect();
            tab.push(next);
            w *= 2;
        }
        Self { ts, tab }
    }

    /// The lowest LOW of the seconds starting in [a, b).
    fn min_in(&self, a: i64, b: i64) -> Option<i64> {
        let i = self.ts.partition_point(|&t| t < a);
        let j = self.ts.partition_point(|&t| t < b);
        if j <= i {
            return None;
        }
        let n = j - i;
        let k = usize::try_from(usize::BITS - 1 - n.leading_zeros()).unwrap_or(0);
        let row = self.tab.get(k)?;
        Some(row.get(i).copied().unwrap_or(i64::MAX).min(row.get(j - (1 << k)).copied().unwrap_or(i64::MAX)))
    }
}

/// One trip of the joined run-down: (premium paid, units bought, the trip's realised GROSS result = its sale and leftover values minus
/// its premium = booked net + booked charges, the lowest LOW of its contract over its held seconds when one was printed).
pub(crate) type RdTrip = (i64, u64, i64, Option<i64>);

/// Lane B's S5 run-down (the ONE positional engine, POSITIONAL-SEARCH-DESIGN s9 / opt_pos::pos of lane B, kept for the index): cumulative
/// over every trip (the entry, then each roll's open leg): the worst mark of the held trip (units x the lowest LOW over its held seconds
/// minus its premium) PLUS the realised gross result of the trips before it, below 0, in bp of the FIRST entry's premium, a SATURATING
/// u16: `(bp, saturated)`; `saturated` = the joined loss left the width (flag RD_SATURATED, the reading is `u16::MAX`). A trip with no
/// print in its held window adds its realised result only.
#[must_use]
pub(crate) fn join_rundown(trips: &[RdTrip]) -> (u16, bool) {
    let first = trips.first().map_or(0, |t| t.0);
    let mut realised: i128 = 0;
    let mut worst: i128 = 0;
    for &(value_in, units_in, gross, lo) in trips {
        if let Some(lo) = lo {
            let mark = realised + i128::from(units_in) * i128::from(lo) - i128::from(value_in);
            worst = worst.min(mark);
        }
        realised += i128::from(gross);
    }
    if cfg!(feature = "m_pos_rd_alone") {
        // MUTANT m_pos_rd_alone: each trip read alone against its own premium (the roll legs NOT joined, the realised result dropped).
        let alone = trips.iter().filter_map(|&(v, u, _, lo)| lo.filter(|_| v > 0).map(|lo| ((i128::from(v) - i128::from(u) * i128::from(lo)).max(0) * 10_000) / i128::from(v))).max().unwrap_or(0);
        return (u16::try_from(alone).unwrap_or(u16::MAX), false);
    }
    let bp = (-worst).max(0) * 10_000 / i128::from(first.max(1));
    match u16::try_from(bp) {
        Ok(v) => (v, false),
        Err(_) => (u16::MAX, true),
    }
}

/// The run-down of a stream ([`join_rundown`]): per trip, the lowest LOW of the held contract from its buy's E0 to the trip's money
/// release, day by day on that contract's own seconds (per contract-day sparse tables, O(1) a query; the production source), joined over
/// the rolls.
fn rundown(book: &opos::Book, o: &opos::StreamOut, sessions: &[i32], cache: &mut BTreeMap<(String, i32), DayMin>) -> Result<(u16, bool), String> {
    let mut trips: Vec<RdTrip> = Vec::with_capacity(o.trips.len());
    for tr in &o.trips {
        let gross = tr.net + tr.charges;
        let e0 = o.legs.iter().find(|l| (l.kind == "entry" || l.kind == "roll_open") && l.folder == tr.folder && l.day == tr.buy_day).map_or(0, |l| l.e0);
        if e0 == 0 || tr.units_in == 0 {
            trips.push((tr.value_in, tr.units_in, gross, None));
            continue;
        }
        let until = if tr.closed_at > 0 { tr.closed_at } else { e0 };
        let (d0, _) = idx_cal::storeio::ist_of(e0);
        let (d1, _) = idx_cal::storeio::ist_of(until);
        let mut lo = i64::MAX;
        for &d in sessions.iter().filter(|&&d| d >= d0 && d <= d1) {
            let key = (tr.folder.clone(), d);
            if !cache.contains_key(&key) {
                let rows = book.opt_rows(&tr.folder, d)?.unwrap_or_default();
                cache.insert(key.clone(), DayMin::new(&rows));
            }
            if let Some(m) = cache.get(&key).and_then(|m| m.min_in(if d == d0 { e0 } else { i64::MIN }, if d == d1 { until } else { i64::MAX })) {
                lo = lo.min(m);
            }
        }
        trips.push((tr.value_in, tr.units_in, gross, (lo < i64::MAX).then_some(lo)));
    }
    Ok(join_rundown(&trips))
}

/// The booked (N, level) of one candidate from its two protection paths' outcomes.
fn level_of(book: &opos::Book, kept: &RunOut, off: &RunOut, sessions: &[i32], cache: &mut BTreeMap<(String, i32), DayMin>, zero_qty: bool) -> Result<(PLevel, Class), String> {
    let booked_off = off.net < kept.net;
    let r = if booked_off { off } else { kept };
    let o = r.streams.first().ok_or("pos::run returned no stream")?;
    // W4 REPAIR r3 (check-wave4-S9-r2.md D-R2-1): a data-end valuation that read a DATA-law hole (`TripOut.hole`, opt_pos::pos) makes the
    // row a hole trade like an order spanning one: pflag::HOLE, the pick line's holes count and common_wf's Trade.hole (bit 4: the AFR-2
    // second reading leaves a winning one out) carry it. Mutant m_idx_data_end_nohole: the legs only (the w4r reading).
    let hole = o.legs.iter().any(|l| l.hole) || (!cfg!(feature = "m_idx_data_end_nohole") && o.trips.iter().any(|t| t.data_end && t.hole));
    let class = classify(o, hole);
    if zero_qty {
        return Ok((PLevel { flags: pflag::ZERO_QTY | pflag::ZERO, ..PLevel::default() }, Class::Keep));
    }
    let fl = |k: &str| o.flags.get(k).copied().unwrap_or(0);
    let mut f = 0u32;
    if hole {
        f |= pflag::HOLE;
    }
    if fl("entry_zero_fill") > 0 || o.trips.is_empty() {
        f |= pflag::ZERO;
    }
    f |= leftover_bits(o);
    if o.legs.iter().any(|l| l.unpaid > 0) {
        f |= pflag::PROTECTED;
    }
    if fl("cash_short") > 0 {
        f |= pflag::ACCOUNT_CAP;
    }
    if booked_off {
        f |= pflag::OFF_BOOKED;
    }
    if fl("roll_no_itm") > 0 {
        f |= pflag::ROLL_NO_ITM;
    }
    if fl("roll_refused_cash") > 0 {
        f |= pflag::ROLL_REFUSED_CASH;
    }
    if fl("roll_close_incomplete") + fl("roll_t2_at_index_close") + fl("roll_wait_close_incomplete") > 0 {
        f |= pflag::ROLL_CLOSE_INCOMPLETE;
    }
    if fl("close_split") > 0 {
        f |= pflag::CLOSE_SPLIT;
    }
    if fl("carry") > 0 {
        f |= pflag::CARRY;
    }
    if fl("entry_cut_by_exit") + fl("open_cut_by_flip") + fl("roll_cancelled_by_flip") > 0 {
        f |= pflag::CUT_BY_EXIT;
    }
    let last_known = o.trips.iter().map(|t| t.closed_at).chain(o.legs.iter().map(|l| l.known_at)).max().unwrap_or(0);
    let first = o.trips.first();
    let nights = {
        let d0 = first.map_or(0, |t| t.buy_day);
        let d1 = idx_cal::storeio::ist_of(last_known.max(1)).0;
        u16::try_from(sessions.iter().filter(|&&d| d > d0 && d <= d1).count()).unwrap_or(u16::MAX)
    };
    let (rd_bp, saturated) = rundown(book, o, sessions, cache)?;
    if saturated {
        f |= pflag::RD_SATURATED;
    }
    let lv = PLevel {
        net: o.net,
        rd_bp,
        last_known,
        flags: f,
        units: first.map_or(0, |t| t.units_in),
        entry_value: first.map_or(0, |t| t.value_in),
        exit_value: o.trips.iter().flat_map(|t| t.sells.iter().map(|s| s.1)).sum::<i64>(),
        rolls: u8::try_from(fl("rolls")).unwrap_or(u8::MAX),
        nights,
    };
    Ok((lv, class))
}

/// One order leg of a priced positional row at the proxy N and the last share level, protection kept (the IDX-POS-LEGS table: plan s8
/// S7 "the one-order gate on every headline entry and roll-open of the canaries, with close_split counted (F8)"; s5.1 the contract on
/// every trade row).
#[derive(Clone, Debug, Default)]
pub(crate) struct LegFact {
    pub(crate) kind: &'static str,
    pub(crate) day: i32,
    pub(crate) decided: i64,
    pub(crate) folder: String,
    pub(crate) expiry: i32,
    pub(crate) lot: u64,
    pub(crate) freeze: i64,
    pub(crate) max_order: i64,
    /// The A2 (Rs 5 L) quantity of a buy leg's pick (0 on a close leg).
    pub(crate) qty_a2: u64,
    pub(crate) want: u64,
    pub(crate) units: u64,
    /// Orders sent (freeze_slices; `close_split k` when k > 1 on a close leg).
    pub(crate) slices: u64,
    pub(crate) depth: u32,
    pub(crate) skipped_no_trade: u32,
    pub(crate) skipped_big: u32,
    pub(crate) reference: i64,
    pub(crate) ref_age_s: i64,
    pub(crate) spot: i64,
    pub(crate) hole: bool,
}

impl LegFact {
    fn of(l: &opos::LegOut) -> Self {
        let p = l.pick.as_ref();
        Self {
            kind: l.kind,
            day: l.day,
            decided: l.decided,
            folder: l.folder.clone(),
            expiry: l.expiry,
            lot: l.lot,
            freeze: l.freeze,
            max_order: l.max_order,
            qty_a2: p.map_or(0, |x| x.qty_a2),
            want: l.want,
            units: l.units,
            slices: l.slices,
            depth: p.map_or(0, |x| x.depth),
            skipped_no_trade: p.map_or(0, |x| x.skipped_no_trade),
            skipped_big: p.map_or(0, |x| x.skipped_big),
            reference: p.map_or(0, |x| x.reference),
            ref_age_s: p.map_or(0, |x| x.ref_age_s),
            spot: p.map_or(0, |x| x.spot),
            hole: l.hole,
        }
    }

    /// A BUY leg sent to the exchange with an A2 pick (an entry or a roll-open; a refused roll-open sends nothing).
    fn is_buy(&self) -> bool {
        self.kind == "entry" || self.kind == "roll_open"
    }
}

/// The one-order gate F8 of a buy leg: its A2 (Rs 5 L) quantity is whole lots, above 0 and at or below that day's one-order maximum
/// (plan s3.1 (3) / A5: every headline ENTRY and every ROLL-OPEN fits one order). Mutant `m_pos_f8`: the maximum is not checked.
#[must_use]
pub(crate) fn f8_ok(l: &LegFact) -> bool {
    let fits = cfg!(feature = "m_pos_f8") || i64::try_from(l.qty_a2).unwrap_or(i64::MAX) <= l.max_order;
    fits && l.qty_a2 > 0 && l.lot > 0 && l.qty_a2.is_multiple_of(l.lot)
}

/// The positional table of one (side, rule).
pub(crate) struct PosTable {
    pub(crate) t: Table,
    pub(crate) grp: Vec<u32>,
    pub(crate) ord: Vec<u32>,
    pub(crate) book_day: Vec<i32>,
    pub(crate) flx: Vec<Vec<u32>>,
    pub(crate) runs: u64,
    pub(crate) pos_counts: BTreeMap<&'static str, u64>,
    /// Per kept row (in table order): (candidate, group, its legs at the proxy N and the last level, kept path).
    pub(crate) legs: Vec<(usize, u32, Vec<LegFact>)>,
    /// Pricing wall per candidate (every N, level and path), microseconds (p50 / p99 printed: O(1) measured).
    pub(crate) cand_us: Vec<u64>,
    /// Kept rows whose money is last known on a session the common_wf calendar does not walk (a special session) or on no session:
    /// booked at the next walked session ([`booking_session`]).
    pub(crate) book_moved: u64,
}

/// The booking session of a positional row (plan 6.1 / s4.3a B3, index into `sess_days`): the first WALKED session (the common_wf
/// calendar's rule, `walked_ix` = the entry sessions; common_wf walks a SPECIAL session only under a quoted ruling, MAIN R11) at or
/// after the day `d` its money is last known. An exit or carry that fills on a special session (a Saturday budget or DR session, a
/// Sunday budget session, a Muhurat evening: lane B Q16 allows the exit there) books at the next walked session, never earlier than
/// known; None when no walked session follows (never on this data: the last session is regular). Mutant `m_pos_book_unwalked`: the
/// latest session at or before `d` (the final3 reading: a special session, which common_wf refuses as `TradeOnUnwalkedSession`).
#[must_use]
pub(crate) fn booking_session(sess_days: &[i32], walked_ix: &[usize], d: i32) -> Option<usize> {
    if cfg!(feature = "m_pos_book_unwalked") {
        return Some(sess_days.partition_point(|&x| x <= d).saturating_sub(1));
    }
    let s0 = sess_days.partition_point(|&x| x < d);
    walked_ix.get(walked_ix.partition_point(|&i| i < s0)).copied()
}

/// One candidate to price: (candidate index, entry instant, exit decision instant; `opt_pos::pos::DATA_END` = no exit before the data
/// end, W4 REPAIR D-W4-1).
type PosCand = (usize, i64, i64);

/// Microseconds as milliseconds with 3 decimals (integer arithmetic only).
#[must_use]
pub(crate) fn ms3(us: u64) -> String {
    format!("{}.{:03}", us / 1000, us % 1000)
}

/// A year unit's word exactly as common_wf's report prints its items (W02 / W05 / W10 / W11; `report::item` is private there): "LABEL
/// no data for this instrument", "LABEL no session in the data", "LABEL no trade" or "LABEL Rs NET" (W4 REPAIR D-W4-1: the data-end
/// test-year word with and without the open_at_data_end trades).
#[must_use]
pub(crate) fn unit_word(units: &[common_wf::windows::Unit], u: usize, f: &common_wf::Facts) -> String {
    let label = units.get(u).map_or("", |x| x.label.as_str());
    let bit = u32::try_from(u).ok().and_then(|s| 1u64.checked_shl(s)).unwrap_or(0);
    let c = f.units.get(u).copied().unwrap_or(common_wf::UnitCell::EMPTY);
    if f.nodata & bit != 0 {
        format!("{label} no data for this instrument")
    } else if f.missing & bit != 0 {
        format!("{label} no session in the data")
    } else if c.trades == 0 {
        format!("{label} no trade")
    } else {
        format!("{label} {}", common_wf::words::fmt_rs(c.net))
    }
}

/// Prices every kept candidate of a (side, rule) at every N, level and protection path through `opt_pos::pos::run`, classifies, books.
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one positional table")]
fn price_all_pos_idx(env: &IdxEnv, fam: &Family, ctx: &feat_io::host::Ctx, cands: &[PosCand], grp_of: &[(u32, u32)], groups: &[Group], long: bool, slots: &[i64], levels: &[u16], account: i64, hull_to: i32, threads: usize, proxy_ni: usize) -> Result<PosTable, String> {
    let nn = slots.len();
    let nlev = levels.len();
    let origin = crate::price::at_sod(common_feat::cal::parse_ymd("2018-01-01").ok_or("origin")?, 0);
    let sess_days: Vec<i32> = ctx.sessions.iter().map(|s| s.day).collect();
    // the sessions the common_wf calendar walks (wfio::calendar: walked = an entry session; a special session only under a ruling, R11)
    let walked_ix: Vec<usize> = ctx.sessions.iter().enumerate().filter(|(_, s)| s.entry).map(|(i, _)| i).collect();
    // month chunks of entries (a fresh book per chunk and worker: bounded caches)
    let mut by_month: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (j, c) in cands.iter().enumerate() {
        let mk = fam.cands.get(c.0).map_or(0, |x| x.mk);
        by_month.entry(mk).or_default().push(j);
    }
    let chunks: Vec<Vec<usize>> = by_month.into_values().collect();
    let next = AtomicUsize::new(0);
    type Priced = (usize, Result<(Vec<Vec<PLevel>>, i64, i64, Class, BTreeMap<&'static str, u64>, Vec<LegFact>, u64), String>);
    let priced: Mutex<Vec<Priced>> = Mutex::new(Vec::new());
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let runs = AtomicUsize::new(0);
    let tp = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let mut local: Vec<Priced> = Vec::new();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(ch) = chunks.get(k) else { break };
                    let book = env.book();
                    let mut cache: BTreeMap<(String, i32), DayMin> = BTreeMap::new();
                    for &j in ch {
                        let tc = Instant::now();
                        let Some(&(ci, t, exit_at)) = cands.get(j) else { continue };
                        if let Some(code) = env.entry_refusal(t) {
                            // DECIDE (known at T): the run cannot even look the entry up; refused by name, never an error
                            local.push((j, Ok((Vec::new(), 0, 0, Class::Decide(code), BTreeMap::new(), Vec::new(), 0))));
                            continue;
                        }
                        let mut lv: Vec<Vec<PLevel>> = Vec::with_capacity(nn);
                        let mut class = Class::Keep;
                        let mut entry_first = 0i64;
                        let mut legs: Vec<LegFact> = Vec::new();
                        let mut counts: BTreeMap<&'static str, u64> = BTreeMap::new();
                        let mut err = None;
                        'n: for (ni, &slot) in slots.iter().enumerate() {
                            let mut row = Vec::with_capacity(nlev);
                            for (li, &bp) in levels.iter().enumerate() {
                                let stream = Stream { book: 0, ce: long, entry: t, exit: exit_at, slot, label: format!("{} {}", env.index, if long { "CE" } else { "PE" }) };
                                // W4 REPAIR D-W4-3: the ONE slot rule of the account (SlotRule::default() = base / N, MAIN ruling 2)
                                let spec = |kept: bool| Spec { streams: vec![stream.clone()], account, bp, kept, open_rule: OpenRule::Sequential, target: env.target, slot_rule: SlotRule::default() };
                                let (rk, ro) = match (opos::run(std::slice::from_ref(&book), &spec(true)), opos::run(std::slice::from_ref(&book), &spec(false))) {
                                    (Ok(a), Ok(b)) => (a, b),
                                    (Err(e), _) | (_, Err(e)) => {
                                        err = Some(e);
                                        break 'n;
                                    }
                                };
                                runs.fetch_add(2, Ordering::Relaxed);
                                let zero_qty = rk.streams.first().is_some_and(|o| o.flags.get("entry_cash_short_zero").copied().unwrap_or(0) > 0 && o.trips.is_empty());
                                // W4 REPAIR D-W4-1: OPEN_AT_DATA_END comes from the pricer's outcome only (leftover_bits)
                                let (x, c) = match level_of(&book, &rk, &ro, &sess_days, &mut cache, zero_qty) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        err = Some(e);
                                        break 'n;
                                    }
                                };
                                if ni == proxy_ni && li == nlev.saturating_sub(1) {
                                    class = c;
                                    if let Some(o) = rk.streams.first() {
                                        legs = o.legs.iter().map(LegFact::of).collect();
                                        for (k2, v) in &o.flags {
                                            *counts.entry(k2).or_insert(0) += u64::from(*v);
                                        }
                                    }
                                }
                                if entry_first == 0 {
                                    entry_first = rk.streams.first().and_then(|o| o.legs.iter().find(|l| l.kind == "entry")).map_or(0, |l| l.e0);
                                }
                                row.push(x);
                            }
                            lv.push(row);
                        }
                        if let Some(e) = err {
                            if let Ok(mut g) = errors.lock() {
                                g.push(format!("candidate {ci}: {e}"));
                            }
                            continue;
                        }
                        let last = lv.iter().flatten().map(|x| x.last_known).max().unwrap_or(0);
                        let us = u64::try_from(tc.elapsed().as_micros()).unwrap_or(u64::MAX);
                        local.push((j, Ok((lv, entry_first, last, class, counts, legs, us))));
                    }
                }
                if let Ok(mut g) = priced.lock() {
                    g.extend(local);
                }
            });
        }
    });
    let errors = errors.into_inner().map_err(|_| "a pricing worker panicked")?;
    if let Some(e) = errors.first() {
        return Err(format!("positional index pricing error (never a refusal; {} in all): {e}", errors.len()));
    }
    let mut priced = priced.into_inner().map_err(|_| "a pricing worker panicked")?;
    priced.sort_by_key(|x| x.0);
    let price_s = tp.elapsed().as_secs_f64();
    // booking (6.1): B(g) = the latest last-known session over every KEPT priced row of g at every N and level; running max over groups.
    // The session of a row is the first WALKED session at or after the day its money is last known (booking_session).
    let ng = groups.len();
    let mut bg: Vec<usize> = vec![0; ng];
    let mut book_moved = 0u64;
    for (j, r) in &priced {
        let Ok((_, _, last, class, _, _, _)) = r else { continue };
        if *class != Class::Keep {
            continue;
        }
        let Some(&(g, _)) = grp_of.get(*j) else { continue };
        let d = idx_cal::storeio::ist_of((*last).max(1)).0;
        let si = booking_session(&sess_days, &walked_ix, d).ok_or_else(|| format!("booking: no walked session at or after {} (candidate row {j}; never booked earlier than known)", idx_cal::day::fmt(d)))?;
        if *last > 0 && sess_days.get(si) != Some(&d) {
            book_moved += 1;
        }
        if let Some(b) = bg.get_mut(g as usize) {
            *b = (*b).max(si);
        }
    }
    let entry_sess: Vec<usize> = (0..ng).map(|g| groups.get(g).and_then(|gr| cands.get(gr.first)).and_then(|c| fam.cands.get(c.0)).map_or(0, |c| c.sess as usize)).collect();
    let book = booking(&bg, &entry_sess);
    let mk_of = |d: i32| {
        let (y, m, _) = common_feat::cal::civil(d);
        y * 12 + i32::try_from(m).unwrap_or(1) - 1
    };
    let mut pt = PosTable { t: Table::empty(nn, price_s), grp: Vec::new(), ord: Vec::new(), book_day: Vec::new(), flx: vec![Vec::new(); nn], runs: runs.load(Ordering::Relaxed) as u64, pos_counts: BTreeMap::new(), legs: Vec::new(), cand_us: Vec::new(), book_moved };
    let mut seen_test = false;
    for (j, r) in priced {
        let (lv, entry_first, _last, class, counts, legs, us) = match r {
            Err(e) => {
                *pt.t.refusals.entry(e).or_insert(0) += 1;
                continue;
            }
            Ok(x) => x,
        };
        if us > 0 {
            pt.cand_us.push(us);
        }
        if let Class::Decide(code) = class {
            *pt.t.refusals.entry(code.to_string()).or_insert(0) += 1;
            continue;
        }
        for (k, v) in counts {
            *pt.pos_counts.entry(k).or_insert(0) += v;
        }
        let Some(&(ci, _, _)) = cands.get(j) else { continue };
        let Some(&(g, ord)) = grp_of.get(j) else { continue };
        let Some(c) = fam.cands.get(ci) else { continue };
        let bs = book.get(g as usize).copied().unwrap_or(0);
        let bday = sess_days.get(bs).copied().unwrap_or(0);
        let mk = mk_of(bday);
        if mk > hull_to {
            seen_test = true;
        } else if seen_test {
            return Err("a learning row after a test row: the booking months are not a prefix (6.1)".to_string());
        } else {
            pt.t.nl += 1;
        }
        let fno_end = env.fno_end_sod(bday);
        let close_s = u32::try_from(fno_end - 1).unwrap_or(0);
        let gids = (g, groups.get(g as usize).map_or(0, |_| g));
        let rec = encode_pos(&lv, entry_first, origin, gids)?;
        let mut cells = Vec::with_capacity(nn * 5);
        let mut qv = Vec::with_capacity(nn * 5);
        let mut sized = Vec::with_capacity(nn);
        for (ni, row) in lv.iter().enumerate() {
            for l in 0..5 {
                let x = row.get(l).copied().unwrap_or_default();
                cells.push((x.net, x.rd_bp, close_s, pflag::cash_bits(x.flags)));
                qv.push((x.units, x.entry_value, x.exit_value));
                if let Some(fx) = pt.flx.get_mut(ni) {
                    fx.push(x.flags);
                }
            }
            let q = row.first().map_or(0, |x| x.units);
            sized.push(crate::size::Sized { qty: q, rule_qty: q, capped: row.first().is_some_and(|x| x.flags & pflag::ACCOUNT_CAP != 0) });
        }
        let ro = RowOut { rec, cells, rewalks: 0, entry_first, refpx: 0, sized, qv };
        pt.t.push_row(g, mk, ci, &ro, nn);
        pt.grp.push(g);
        pt.ord.push(ord);
        pt.book_day.push(bday);
        pt.legs.push((ci, g, legs));
        let _ = c;
    }
    Ok(pt)
}

/// One cell of the fixed-slot beside: (table row, N index, level index).
type FxCell = (usize, usize, usize);

/// W4 REPAIR r3 (check-wave4-S9-r2.md section 3 = D-W4-3 item 3; MAIN ruling of 16 Nov 2026 ~14:3x IST, main a3019c01): the FIXED-SLOT
/// reading BESIDE the picks, never the headline (base / N stays the headline, D-W4-3; lost money is never reinvested). Each requested cell
/// (a pick's own row at the pick's N and level) is re-priced through `opt_pos::pos::run` under `SlotRule::Fixed` (the fixed Rs 20 L / N
/// slot) on BOTH protection paths, the worse booked exactly as the headline books it ([`level_of`]), on the same Book (the production
/// source); month chunks of entries on `threads` workers, a fresh Book per chunk (bounded caches, as [`price_all_pos_idx`]). Returns the
/// booked level per cell and each cell's wall in microseconds (p50 / p99 printed: O(1) measured).
#[allow(clippy::too_many_arguments, reason = "one beside pricing")]
fn price_fixed_beside(env: &IdxEnv, fam: &Family, sess_days: &[i32], cells: &BTreeMap<FxCell, PosCand>, long: bool, slots: &[i64], levels: &[u16], account: i64, threads: usize) -> Result<(BTreeMap<FxCell, PLevel>, Vec<u64>), String> {
    let mut by_month: BTreeMap<i32, Vec<(FxCell, PosCand)>> = BTreeMap::new();
    for (k, c) in cells {
        let mk = fam.cands.get(c.0).map_or(0, |x| x.mk);
        by_month.entry(mk).or_default().push((*k, *c));
    }
    let chunks: Vec<Vec<(FxCell, PosCand)>> = by_month.into_values().collect();
    let next = AtomicUsize::new(0);
    let got: Mutex<Vec<(FxCell, PLevel, u64)>> = Mutex::new(Vec::new());
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let mut local: Vec<(FxCell, PLevel, u64)> = Vec::new();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(ch) = chunks.get(k) else { break };
                    let book = env.book();
                    let mut cache: BTreeMap<(String, i32), DayMin> = BTreeMap::new();
                    for &((j, ni, col), (ci, t, exit_at)) in ch {
                        let tc = Instant::now();
                        let r = match (slots.get(ni), levels.get(col)) {
                            (Some(&slot), Some(&bp)) => {
                                let stream = Stream { book: 0, ce: long, entry: t, exit: exit_at, slot, label: format!("{} {}", env.index, if long { "CE" } else { "PE" }) };
                                let fixed = |kept: bool| Spec { streams: vec![stream.clone()], account, bp, kept, open_rule: OpenRule::Sequential, target: env.target, slot_rule: SlotRule::Fixed };
                                opos::run(std::slice::from_ref(&book), &fixed(true)).and_then(|rk| opos::run(std::slice::from_ref(&book), &fixed(false)).map(|ro| (rk, ro))).and_then(|(rk, ro)| {
                                    let zero_qty = rk.streams.first().is_some_and(|o| o.flags.get("entry_cash_short_zero").copied().unwrap_or(0) > 0 && o.trips.is_empty());
                                    level_of(&book, &rk, &ro, sess_days, &mut cache, zero_qty)
                                })
                            }
                            _ => Err(format!("no N / level for the cell (row {j}, N index {ni}, level {col})")),
                        };
                        match r {
                            Ok((x, _)) => local.push(((j, ni, col), x, u64::try_from(tc.elapsed().as_micros()).unwrap_or(u64::MAX))),
                            Err(e) => {
                                if let Ok(mut g) = errors.lock() {
                                    g.push(format!("candidate {ci}: {e}"));
                                }
                            }
                        }
                    }
                }
                if let Ok(mut g) = got.lock() {
                    g.extend(local);
                }
            });
        }
    });
    let errors = errors.into_inner().map_err(|_| "a fixed-slot beside worker panicked")?;
    if let Some(e) = errors.first() {
        return Err(format!("fixed-slot beside pricing error (never a refusal; {} in all): {e}", errors.len()));
    }
    let got = got.into_inner().map_err(|_| "a fixed-slot beside worker panicked")?;
    let mut us = Vec::with_capacity(got.len());
    let mut map = BTreeMap::new();
    for (k, x, u) in got {
        us.push(u);
        map.insert(k, x);
    }
    Ok((map, us))
}

impl IdxEnv {
    /// The dated F&O end (seconds of the IST day) of session `d` (S1 FNO-END-ROWS; 15:30:00 when the row is missing).
    #[must_use]
    pub fn fno_end_sod(&self, d: i32) -> i64 {
        self.book_sess_end(d).unwrap_or(15 * 3600 + 30 * 60)
    }
}

/// One twin slot: (group, calendar year, decision candles of the group).
type TwinSlot = (u32, i32, u32);

/// The twin's draws per GROUP (FU-9; lane B's `twin_draws_pos`, copied): u32 candle counts.
fn twin_draws_pos(seed: u64, world: u32, slots: &[TwinSlot], per_year: &BTreeMap<i32, u32>) -> Vec<(u32, u32)> {
    let mut by_year: BTreeMap<i32, Vec<(u64, TwinSlot)>> = BTreeMap::new();
    for s in slots {
        by_year.entry(s.1).or_default().push((crate::twin::key(seed, world, s.0), *s));
    }
    let mut out = Vec::new();
    for (y, mut v) in by_year {
        let m = per_year.get(&y).copied().unwrap_or(0) as usize;
        v.sort_unstable_by_key(|x| (x.0, x.1.0));
        for (k, s) in v.into_iter().take(m) {
            let c = 1 + u32::try_from(sniper_kernel::scale::splitmix(k) % u64::from(s.2.max(1))).unwrap_or(0);
            out.push((s.0, c));
        }
    }
    out.sort_unstable();
    out
}

/// The index positional exit rule of an exit id (`POS-XS10-30` headline, `POS-FLIP10-30` beside).
fn rule_of(id: &str) -> Result<Rule, String> {
    match id {
        "POS-XS10-30" | "POS-ST10-30" => Ok(Rule::Xs),
        "POS-FLIP10-30" => Ok(Rule::Flip),
        x => Err(format!("positional exit {x}: POS-XS10-30 (headline) | POS-FLIP10-30 (beside)")),
    }
}

/// Runs an index POSITIONAL family.
///
/// # Errors
/// A held gate, a refused input, an I/O error, a pricing error.
#[allow(clippy::too_many_lines, reason = "the positional family's pipeline in one place")]
pub fn run_family_idx_pos(inp: &Inputs, spec: &RunSpec) -> Result<Vec<String>, String> {
    let run = &inp.run;
    let mut lines = Vec::new();
    let gate = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, 8)?;
    lines.push(format!("{} gate ok={} hold={:?} pressure={:?} swap_used={:?} swap_free={:?} disk_free={:?}", crate::ist_now(), gate.ok, gate.hold, gate.pressure, gate.swap_used_mb, gate.swap_free_mb, gate.disk_free_gib));
    if !gate.ok {
        return Err(format!("launch gate HOLDS: {:?}", gate.hold));
    }
    let slice = spec.slice.as_deref().map(Slice::load).transpose()?;
    let db = crate::db::Db::open(&spec.out.join("db"), crate::VERSION)?;
    let t0 = Instant::now();
    let te = Instant::now();
    let env = IdxEnv::load(&inp.instr)?;
    lines.push(format!("index option inputs ({}): the production source {}, target Rs {} a trade (A2); {:.1} s", env.index, env.pack_note, env.target / 100, te.elapsed().as_secs_f64()));
    // B5: positional entries at index candle ENDs strictly before the index close 15:30:00
    let cut = 15 * 3600 + 30 * 60;
    let rd = crate::idx::readings(inp, spec, slice.as_ref(), cut)?;
    lines.extend(rd.lines.iter().cloned());
    let sst = crate::posgroups::st_stream(&rd.ctx, &rd.stream)?;
    lines.push(format!("Supertrend 10 x 3 stream (lane B's posgroups on the index candles): {} candles, {} ready", sst.len(), sst.iter().filter(|c| c.up.is_some()).count()));
    let cal = crate::wfio::calendar(&rd.ctx.sessions)?;
    let (hull_from, hull_to) = inp.windows.hull();
    let levels: Vec<u16> = run.list("levels_bp")?;
    let account: i64 = run.num("account_paisa")?;
    let n_grid = crate::idx::rows_list(inp, "n_grid")?;
    let proxy_n = crate::idx::rows_list(inp, "proxy_n")?.first().copied().ok_or("IDXOPT row proxy_n empty")?;
    let slots: Vec<i64> = n_grid.iter().map(|n| account / n.max(&1)).collect();
    let twin_draws: u32 = run.num("twin_draws")?;
    let twin_seed: u64 = run.num("twin_seed")?;
    let wf_data = PathBuf::from(run.get("windows_file")?).parent().map(Path::to_path_buf).ok_or("windows_file has no folder")?;
    let part_tpl = crate::rows::read_with_sha(&spec.data.join("PART-SNIPER-v1.tsv").to_string_lossy())?;
    let labels = inp.words.standing(1, "printed by the tool").join(" | ");
    let mut total = CtxOut { lines: Vec::new(), searches: 0, wf_passed: 0, tt_passed: 0, both: 0, mismatches: 0 };
    let sess_mk: Vec<i32> = rd
        .ctx
        .sessions
        .iter()
        .map(|s| {
            let (y, m, _) = common_feat::cal::civil(s.day);
            y * 12 + i32::try_from(m).unwrap_or(1) - 1
        })
        .collect();
    // W4 REPAIR r3 (D-W4-3 item 3): each context's fixed-slot beside summary, for the SN1 row slot_rule_beside
    let mut fx_all: Vec<String> = Vec::new();
    for &long in &spec.sides {
        for exit_id in &spec.exits {
            let rule = rule_of(exit_id)?;
            let pc = PosCtx { inp, spec, db: &db, fam: &rd.fam, ctx: &rd.ctx, sst: &sst, env: &env, long, rule, slots: &slots, n_grid: &n_grid, proxy_n, cal: &cal, wf_data: &wf_data, part_tpl: &part_tpl.0, slice: slice.as_ref(), twin: (twin_seed, twin_draws), labels: &labels, hull: (hull_from, hull_to), levels: &levels, account, sess_mk: &sess_mk };
            let (c, fx) = context_pos(&pc)?;
            fx_all.push(fx);
            total.searches += c.searches;
            total.wf_passed += c.wf_passed;
            total.tt_passed += c.tt_passed;
            total.both += c.both;
            total.mismatches += c.mismatches;
            for l in &c.lines {
                println!("{l}");
            }
        }
    }
    lines.extend(total.lines);
    lines.push(format!(
        "SUMMARY {} {} INDEX OPTIONS positional: {} searches; walk-forward picks PASSED {} of {}; train/test picks PASSED {} of {}; both {} of {}; re-walk key mismatches {}",
        inp.instr.underlying(),
        spec.tf,
        total.searches,
        total.wf_passed,
        total.searches,
        total.tt_passed,
        total.searches,
        total.both,
        total.searches,
        total.mismatches
    ));
    let mut sn1 = vec![
        vec![spec.run_id.clone(), "version".to_string(), crate::VERSION.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "laws".to_string(), crate::law_line(), labels.clone()],
        vec![spec.run_id.clone(), "pricer".to_string(), format!("opt_pos::pos::run (S8b + wave 4) per candidate; crate {} {}", cf21::VERSION, cf21::STEP), labels.clone()],
        vec![spec.run_id.clone(), "threads".to_string(), spec.threads.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "gate".to_string(), format!("{gate:?}").replace('\n', " "), labels.clone()],
        vec![spec.run_id.clone(), "stream_source".to_string(), rd.stream.source.clone(), labels.clone()],
        vec![spec.run_id.clone(), "n_grid".to_string(), format!("{n_grid:?} (proxy N {proxy_n})"), labels.clone()],
        vec![spec.run_id.clone(), "outcome_row_bytes".to_string(), record_width_pos(n_grid.len(), levels.len()).to_string(), labels.clone()],
        vec![spec.run_id.clone(), "not_built".to_string(), "DETAIL / P7, netted view, MTM drawdown beside, R16 account walk, FLIP comparison of XS picks, shared-contract split (s26)".to_string(), labels.clone()],
        vec![spec.run_id.clone(), "slot_rule".to_string(), format!("{} (opt_pos::pos::SlotRule::default(), the one rule of opt_pos::pos and opt_pos::acct; W4 REPAIR D-W4-3); beside: {} on G-ROLL (g-roll --slot-rule fixed) and on every positional pick line (row slot_rule_beside)", SlotRule::default().name(), SlotRule::Fixed.name()), labels.clone()],
        vec![
            spec.run_id.clone(),
            "slot_rule_beside".to_string(),
            format!(
                "{} BESIDE on every positional pick line, never the headline (W4 REPAIR r3: check-wave4-S9-r2 section 3 = D-W4-3 item 3, MAIN ruling 16 Nov 2026 ~14:3x IST; base / N stays the headline, lost money never reinvested): each pick's own rows re-priced at its N and level under opt_pos::pos::SlotRule::Fixed on both protection paths (the worse booked), the same Book (the production source); each booked at the headline's session, or at the first walked session at or after the day the fixed reading's money is last known when that is later (never earlier than known; a running maximum keeps the order); re-walked through common_wf; printed 'fixed-slot beside (fixed Rs 20 L / N): <the pick's test-year words>' after the pick's data-end line (SN-REPORT) and in SN-PICKS (fixed_slot_beside_test_years, _nets_differ, _booked_later); every re-priced cell beside the headline's own cell in FIXED-BESIDE-POS-<tf>-<side>-<exit>.tsv; {}",
                SlotRule::Fixed.name(),
                fx_all.join(" | ")
            ),
            labels.clone(),
        ],
        vec![spec.run_id.clone(), "data_end".to_string(), "a group with no exit before the data end passes no exit decision: every unit still held after the last session is open_at_data_end, valued whole at the worst print of the last session (W0 = E0 when bought that session, else 09:15:00), one sale order charged, flagged OPEN_AT_DATA_END, booked at that session (plan s4.3a B2; W4 REPAIR D-W4-1); a valuation whose window [W0, F&O end) overlaps a DATA-law hole (e.g. a contract-day DP-NEWPX-FAILS lists 'unchecked') is a hole trade: HOLE on its row, so the AFR-2 second reading leaves a winning one out (W4 REPAIR r3 D-R2-1); every pick line prints the data-end test year with and without those trades".to_string(), labels.clone()],
        vec![spec.run_id.clone(), "muhurat_carry".to_string(), format!("a carried exit rest is held through the Muhurat sessions (S1 MUHURAT-LIST, {} sessions: {}); it fills from the next session that is not a Muhurat session (plan s4.4 / Q39; W4 REPAIR D-W4-2)", env.muhurat_days().len(), env.muhurat_days().iter().map(|d| idx_cal::day::fmt(*d)).collect::<Vec<_>>().join(", ")), labels.clone()],
        vec![spec.run_id.clone(), "wall_s".to_string(), format!("{:.1}", t0.elapsed().as_secs_f64()), labels.clone()],
    ];
    for (f, s) in inp.shas.iter().chain(env.shas.iter()) {
        sn1.push(vec![spec.run_id.clone(), format!("sha256 {f}"), s.clone(), labels.clone()]);
    }
    for (k, v) in &inp.instr.rows {
        sn1.push(vec![spec.run_id.clone(), format!("instrument {k}"), v.clone(), labels.clone()]);
    }
    for (f, v) in &rd.sn1 {
        sn1.push(vec![spec.run_id.clone(), f.clone(), v.clone(), labels.clone()]);
    }
    db.write(&format!("SN1-POS-{}.tsv", spec.tf), &["run", "field", "value", "labels"], &sn1)?;
    let n = db.verify()?;
    lines.push(format!("DB: {} tables audited, chain verified; run folder {}", n, spec.out.display()));
    for l in inp.words.standing(1, "printed by the tool") {
        lines.push(l);
    }
    lines.push(format!("{} done in {:.1} s", crate::ist_now(), t0.elapsed().as_secs_f64()));
    Ok(lines)
}

/// One positional context's inputs.
struct PosCtx<'a> {
    inp: &'a Inputs,
    spec: &'a RunSpec,
    db: &'a crate::db::Db,
    fam: &'a Family,
    ctx: &'a feat_io::host::Ctx,
    sst: &'a [StC],
    env: &'a IdxEnv,
    long: bool,
    rule: Rule,
    slots: &'a [i64],
    n_grid: &'a [i64],
    proxy_n: i64,
    cal: &'a common_wf::Calendar,
    wf_data: &'a Path,
    part_tpl: &'a str,
    slice: Option<&'a Slice>,
    twin: (u64, u32),
    labels: &'a str,
    hull: (i32, i32),
    levels: &'a [u16],
    account: i64,
    sess_mk: &'a [i32],
}

/// One positional context: candidates, groups, pricing, booking, the sweep at every N, the re-walk through common_wf, the report, the
/// twin, the DB rows; with the context's fixed-slot beside summary (W4 REPAIR r3, D-W4-3 item 3: the SN1 row `slot_rule_beside`).
#[allow(clippy::too_many_lines, reason = "one positional context's pipeline in one place")]
fn context_pos(pc: &PosCtx<'_>) -> Result<(CtxOut, String), String> {
    let (inp, spec, db, fam, long, rule) = (pc.inp, pc.spec, pc.db, pc.fam, pc.long, pc.rule);
    let side = if long { "LONG" } else { "SHORT" };
    let cname = format!("{}|{side}|{}", spec.tf, rule.id());
    let mut lines = vec![format!("=== {} positional context {cname} ({}) ===", crate::ist_now(), inp.instr.underlying())];
    let ncol = 5_usize;
    let levels = pc.levels;
    // --- candidates: stream index of each; pos_st_unready is a DECIDE refusal (R4) ---
    let end_ix: BTreeMap<i64, usize> = pc.sst.iter().enumerate().map(|(i, c)| (c.end, i)).collect();
    let mut refused: BTreeMap<String, u64> = BTreeMap::new();
    let mut kept: Vec<(usize, usize)> = Vec::new();
    for (ci, c) in fam.cands.iter().enumerate() {
        let Some(&si) = end_ix.get(&c.end) else {
            *refused.entry("not_in_stream".to_string()).or_insert(0) += 1;
            continue;
        };
        if pc.sst.get(si).is_none_or(|x| x.up.is_none()) {
            *refused.entry("pos_st_unready".to_string()).or_insert(0) += 1;
            continue;
        }
        kept.push((ci, si));
    }
    let exits = crate::posgroups::exits_all(pc.sst, long, rule);
    let cand_si: Vec<usize> = kept.iter().map(|x| x.1).collect();
    let (groups, grp_of) = crate::posgroups::groups(pc.sst, &cand_si, &exits, &[])?;
    lines.push(format!(
        "groups (lane B 5.2, P-MONO asserted): {} candidates kept of {} ({:?} DECIDE-refused before pricing), {} groups ({} one-candle), largest group {} candles; exits: {} candle, {} data end",
        kept.len(),
        fam.cands.len(),
        refused,
        groups.len(),
        groups.iter().filter(|g| g.n == 1).count(),
        groups.iter().map(|g| g.n).max().unwrap_or(0),
        groups.iter().filter(|g| matches!(g.exit, Exit::Candle(_))).count(),
        groups.iter().filter(|g| g.exit == Exit::DataEnd).count()
    ));
    let fmt_ist = |t: i64| {
        let (d, sod) = idx_cal::storeio::ist_of(t);
        format!("{} {}", idx_cal::day::fmt(d), idx_cal::storeio::hms(sod))
    };
    let mut cand_rows = Vec::with_capacity(kept.len());
    let mut pcands: Vec<PosCand> = Vec::with_capacity(kept.len());
    for (&(ci, si), &(g, o)) in kept.iter().zip(&grp_of) {
        let (Some(c), Some(gr)) = (fam.cands.get(ci), groups.get(g as usize)) else { continue };
        let ex = match gr.exit {
            Exit::Candle(_) | Exit::CaClose(_) => fmt_ist(gr.exit_at),
            Exit::DataEnd => "data end".to_string(),
        };
        let f17 = rule == Rule::Xs && pc.sst.get(si).is_some_and(|x| x.up == Some(!long));
        let t = c.end * US_S;
        cand_rows.push(vec![spec.run_id.clone(), ci.to_string(), idx_cal::day::fmt(c.day), c.k.to_string(), idx_cal::storeio::hms(idx_cal::storeio::ist_of(t).1), g.to_string(), o.to_string(), ex, if f17 { "yes" } else { "no" }.to_string(), format!("labels: SN1-POS-{} row labels", spec.tf)]);
        // W4 REPAIR (check-wave4-S9 D-W4-1; plan s4.3a B2): a group with no exit before the data end passes NO exit decision (the
        // sentinel); the final4 stand-in exit (the last index close, or 1 s after the entry) is removed
        let exit_at = if gr.exit == Exit::DataEnd { opos::DATA_END } else { gr.exit_at };
        pcands.push((ci, t, exit_at));
    }
    db.write(&format!("CAND-POS-{}-{side}-{}.tsv", spec.tf, rule.id()), &["run", "cand", "entry_day", "candle", "entry_decision_ist", "group", "ordinal", "exit_decision", "f17", "labels"], &cand_rows)?;
    // --- pricing ---
    let proxy_ni = pc.n_grid.iter().position(|&n| n == pc.proxy_n).unwrap_or(0);
    let pt = price_all_pos_idx(pc.env, fam, pc.ctx, &pcands, &grp_of, &groups, long, pc.slots, levels, pc.account, pc.hull.1, spec.threads, proxy_ni)?;
    let t = &pt.t;
    // --- the one-order gate F8 on every entry and roll-open, close_split counted (IDX-POS-LEGS: the proxy N, the 100% level, kept) ---
    let (mut f8_ok_n, mut f8_fail_n, mut buys, mut entries, mut opens, mut closes, mut split_legs, mut split_orders, mut hole_legs) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let mut leg_rows = Vec::new();
    for (ci, g, legs) in &pt.legs {
        let c = fam.cands.get(*ci);
        for (k, l) in legs.iter().enumerate() {
            let verdict = if l.is_buy() {
                buys += 1;
                if l.kind == "entry" {
                    entries += 1;
                } else {
                    opens += 1;
                }
                if f8_ok(l) {
                    f8_ok_n += 1;
                    "fits one order"
                } else {
                    f8_fail_n += 1;
                    "F8 FAIL"
                }
            } else if l.kind == "roll_close" || l.kind == "exit" || l.kind == "carry" {
                closes += 1;
                if l.slices > 1 {
                    split_legs += 1;
                    split_orders += l.slices;
                    "close_split"
                } else {
                    "one order"
                }
            } else {
                "-"
            };
            hole_legs += u64::from(l.hole);
            leg_rows.push(vec![
                spec.run_id.clone(),
                cname.clone(),
                ci.to_string(),
                g.to_string(),
                c.map_or(0, |c| c.day).to_string(),
                c.map_or(0, |c| c.k).to_string(),
                k.to_string(),
                l.kind.to_string(),
                if l.day == 0 { "-".to_string() } else { idx_cal::day::fmt(l.day) },
                if l.decided == 0 { "-".to_string() } else { fmt_ist(l.decided) },
                l.folder.clone(),
                if l.expiry == 0 { "-".to_string() } else { idx_cal::day::fmt(l.expiry) },
                l.lot.to_string(),
                l.freeze.to_string(),
                l.max_order.to_string(),
                l.qty_a2.to_string(),
                l.want.to_string(),
                l.units.to_string(),
                l.slices.to_string(),
                verdict.to_string(),
                l.depth.to_string(),
                l.skipped_no_trade.to_string(),
                l.skipped_big.to_string(),
                l.reference.to_string(),
                l.ref_age_s.to_string(),
                l.spot.to_string(),
                u8::from(l.hole).to_string(),
                format!("labels: SN1-POS-{} row labels", spec.tf),
            ]);
        }
    }
    db.write(
        &format!("IDX-POS-LEGS-{}-{side}-{}.tsv", spec.tf, rule.id()),
        &["run", "context", "cand", "group", "entry_day", "candle", "leg", "kind", "leg_session", "decided_ist", "contract", "expiry_true", "lot", "freeze", "max_one_order", "qty_a2", "want", "units", "orders", "f8_or_close", "itm_depth", "skipped_no_trade", "skipped_big", "size_ref_paisa", "size_ref_age_s", "spot_paisa", "hole", "labels"],
        &leg_rows,
    )?;
    let mut us = pt.cand_us.clone();
    us.sort_unstable();
    let q = |p: usize| us.get((us.len().saturating_sub(1) * p) / 1000).copied().unwrap_or(0);
    // W4 REPAIR r3 (D-R2-1): the data-end valuations that read a DATA-law hole (not orders, so not in IDX-POS-LEGS; HOLE on their rows),
    // counted by the pricer at the same reading (the proxy N, the 100% level, protection kept)
    let (de_hole, de_hole_units) = (pt.pos_counts.get("data_end_hole").copied().unwrap_or(0), pt.pos_counts.get("data_end_hole_units").copied().unwrap_or(0));
    lines.push(format!(
        "index contracts (positional, the proxy N {} at the 100% level, protection kept): {} kept rows, {buys} buy legs ({entries} entries + {opens} roll-opens): one-order gate F8 at the A2 (Rs 5 L) quantity {f8_ok_n} fit one order, {f8_fail_n} fail; {closes} close legs (exits, roll closes, carries): close_split {split_legs} legs sent as {split_orders} orders; {hole_legs} legs on a DATA-law hole (IDX-POS-LEGS-{}-{side}-{}.tsv); {de_hole} data-end valuations on a DATA-law hole ({de_hole_units} units; a valuation is no order: HOLE on its row, W4 REPAIR r3 D-R2-1); pricing per candidate ({} N x {} levels x 2 paths) p50 / p99 / max {} / {} / {} ms over {} candidates",
        pc.proxy_n,
        pt.legs.len(),
        spec.tf,
        rule.id(),
        pc.n_grid.len(),
        levels.len(),
        ms3(q(500)),
        ms3(q(990)),
        ms3(us.last().copied().unwrap_or(0)),
        us.len()
    ));
    if f8_fail_n > 0 {
        return Err(format!("{cname}: the one-order gate F8 failed on {f8_fail_n} positional buy legs (never silent)"));
    }
    let osha = db.write_bin(&format!("OUT-POS-{}-{side}-{}.bin", spec.tf, rule.id()), &t.records)?;
    lines.push(format!(
        "pricing (opt_pos::pos::run, {} runs = rows x {} N x {} levels x 2 protection paths): {} rows priced ({} learning, {} test); refused {:?}; {:.1} s wall on {} threads; OUT-POS rows {} B, sha {osha}; positional facts at the proxy N {} and the 100% level: {:?}; rows whose money is last known on a session the calendar does not walk (special sessions), booked at the next walked session: {}",
        pt.runs,
        pc.n_grid.len(),
        levels.len(),
        t.kidx.len(),
        t.nl,
        t.kidx.len() - t.nl,
        t.refusals,
        t.price_s,
        spec.threads,
        record_width_pos(pc.n_grid.len(), levels.len()),
        pc.proxy_n,
        pt.pos_counts,
        pt.book_moved
    ));
    // --- the kernel rows: Row.sess = the GROUP, Row.k = the ordinal (u32, B5) ---
    let s = usize::from(!long);
    let krows: Vec<Row> = t
        .kidx
        .iter()
        .zip(pt.grp.iter().zip(&pt.ord))
        .filter_map(|(&i, (&g, &o))| fam.cands.get(i).map(|c| {
            let b = c.core.get(s).copied().unwrap_or_default();
            Row { sess: g, k: o, rb: b[0], cb: b[1], wb: b[2] }
        }))
        .collect();
    let (mut values, wiring) = crate::cands::outer_values(fam, &t.kidx, long, &inp.dials, None);
    if let Some(sl) = pc.slice {
        values.retain(|v| inp.dials.get(v.dial).is_some_and(|d| sl.keeps(&d.id, &v.label)));
    }
    let walked_values = values.iter().filter(|v| !v.empty).count() as u64;
    let core = crate::cands::core_of(if long { &inp.core.0 } else { &inp.core.1 });
    let pos = |id: &str| inp.dials.iter().position(|d| d.id == id).unwrap_or(0);
    let core_pos = [pos("M-RSI"), pos("M-CCI"), pos("M-WR")];
    let dec = Decoder::new(&wiring.radix, core_pos, &values, pc.n_grid.len());
    let nw_code = dec.n_weight().ok_or("the N digit leaves 256 bits")?;
    let members = (1 + walked_values).saturating_mul(core.cells()).saturating_mul(pc.n_grid.len() as u64);
    let ng = usize::try_from(pt.grp.iter().max().map_or(0, |g| g + 1)).unwrap_or(0);
    // B4: Layout::new_pos (leaves = the groups' booking months, counted months = the sessions')
    let mut leaf_mk = vec![i32::MAX; ng];
    for (j, &g) in pt.grp.iter().enumerate() {
        if let (Some(x), Some(o)) = (leaf_mk.get_mut(g as usize), t.outs.first().and_then(|v| v.get(j))) {
            *x = o.mk;
        }
    }
    let wins: Vec<(String, i32, i32)> = inp.windows.rows.iter().map(|r| (r.0.clone(), r.1, r.2)).collect();
    let lay = Layout::new_pos(&leaf_mk, pc.sess_mk, pc.hull, &wins).map_err(|e| e.to_string())?;
    lines.push(format!("lattice: {} outer values walked + S0, core {} cells, {} N: {} combos declared; groups {} (the sweep's sessions, FU-10)", walked_values, core.cells(), pc.n_grid.len(), members, ng));
    let lrows = krows.get(..t.nl).unwrap_or(&[]);
    let run = &inp.run;
    let top: u64 = run.num("stage2_top")?;
    let mut merged: Vec<Lists> = Vec::new();
    for v in &inp.views {
        for _ in 0..ncol {
            merged.push(Lists::new(v.k.unwrap_or(top)));
        }
    }
    let mut cells_offered = 0_u64;
    let ts = Instant::now();
    let swept: Vec<usize> = pc.n_grid.iter().enumerate().filter(|(_, n)| !spec.n_sweep.as_ref().is_some_and(|l| !l.contains(n))).map(|(ni, _)| ni).collect();
    let swept_outs: Vec<&[Outcome]> = swept.iter().map(|&ni| t.outs.get(ni).map(Vec::as_slice).ok_or("outcome table")).collect::<Result<_, _>>()?;
    let sctx0 = crate::sweep::Ctx { rows: lrows, out: swept_outs.first().copied().unwrap_or(&[]), lay: &lay, views: &inp.views, cols: ncol, core, sessions: ng, top, radix: &wiring.radix, core_pos, filter: None };
    let mut all = if swept_outs.is_empty() { Vec::new() } else { crate::sweep::run_all(&sctx0, &swept_outs, &values, spec.threads)? }.into_iter();
    let mut sn4 = Vec::new();
    for (ni, &n) in pc.n_grid.iter().enumerate() {
        if spec.n_sweep.as_ref().is_some_and(|l| !l.contains(&n)) {
            continue;
        }
        let res = all.next().ok_or("outcome table")?;
        if !res.refused.is_empty() {
            return Err(format!("{cname} N={n}: the sweep refused {:?}", res.refused.iter().take(3).collect::<Vec<_>>()));
        }
        let shift = nw_code.mul_add(ni as u64, 0).ok_or("N digit overflow")?;
        for (m, l) in merged.iter_mut().zip(&res.lists) {
            m.merge(&l.shifted(shift.0).ok_or("shifted code overflow")?);
        }
        cells_offered += res.walk.cells_offered;
        sn4.push(vec![spec.run_id.clone(), cname.clone(), "S0+S1 (groups)".to_string(), n.to_string(), res.values_walked.to_string(), res.walk.cells.to_string(), res.walk.runs_offered.to_string(), format!("{:?}", res.not_eligible), pc.labels.to_string()]);
    }
    lines.push(format!("sweep at the swept N ({} of {}): {:.1} s wall; cells offered {cells_offered}", swept.len(), pc.n_grid.len(), ts.elapsed().as_secs_f64()));
    // --- the HARD gate (plan 8.4; s4.3a B5): per-event step counts identical at 1x / 10x / 100x, S0 on the WF-learning rows at the proxy N ---
    let wf_to = inp.windows.rows.first().map_or(pc.hull.1, |r| r.2);
    let pout = t.outs.get(proxy_ni).ok_or("proxy outcomes")?;
    let base_rows: Vec<Row> = lrows.iter().zip(pout).filter(|(_, o)| o.mk <= wf_to).map(|(r, _)| *r).collect();
    let base_out: Vec<Outcome> = pout.iter().take(t.nl).filter(|o| o.mk <= wf_to).copied().collect();
    let mut sig = Vec::new();
    let mut steps_rows = Vec::new();
    for times in [1_u16, 10, 100] {
        let (rt, src) = sniper_kernel::scale::replicate(&base_rows, times, 20_261_113);
        let ot: Vec<Outcome> = src.iter().enumerate().filter_map(|(j, &k)| base_out.get(k as usize).map(|o| Outcome { id: j as u64, ..*o })).collect();
        let gctx = crate::sweep::Ctx { rows: &rt, out: &ot, lay: &lay, views: &inp.views, cols: ncol, core, sessions: ng, top: 1000, radix: &wiring.radix, core_pos, filter: None };
        let g = crate::sweep::run(&gctx, &[], 1)?;
        let sg = (g.walk.hist_enter.max(), g.walk.hist_line.max(), g.hist_acc.max(), g.hist_key.max());
        sig.push(sg);
        steps_rows.push(vec![spec.run_id.clone(), cname.clone(), times.to_string(), rt.len().to_string(), format!("{sg:?}"), pc.labels.to_string()]);
    }
    let hard_ok = sig.windows(2).all(|w| w.first() == w.get(1));
    lines.push(format!("HARD gate (per-event step counts identical at 1x / 10x / 100x; S0 on the WF-learning rows at N = {}; Row.k u32): {} {sig:?}", pc.proxy_n, if hard_ok { "PASS" } else { "FAIL" }));
    db.write(&format!("SN4-STEPS-POS-{}-{side}-{}.tsv", spec.tf, rule.id()), &["run", "context", "scale", "rows", "max_enter_line_acc_key", "labels"], &steps_rows)?;
    db.write(&format!("SN4-POS-{}-{side}-{}.tsv", spec.tf, rule.id()), &["run", "context", "stage", "n", "outer_values_walked", "core_cells", "runs_offered", "not_eligible", "labels"], &sn4)?;
    // --- common_wf under the positional RULES ---
    let parts: Vec<Vec<String>> = levels
        .iter()
        .enumerate()
        .map(|(c, bp)| {
            crate::wfio::partition_row(
                pc.part_tpl,
                u32::try_from(c + 1).unwrap_or(1),
                &[("tf", spec.tf.clone()), ("side", if long { "long only".to_string() } else { "short only".to_string() }), ("exit", rule.id().to_string()), ("level", format!("{}", bp / 100)), ("family", "minutes".to_string())],
            )
        })
        .collect();
    let wf = crate::wfio::load_rules(&spec.data, "WF-RULES-SNIPER-POS-v1.tsv", pc.wf_data, pc.cal, &parts, members)?;
    let set = wf.ws.sets.get(wf.set).ok_or("window set")?;
    let mut cl = common_wf::Lists::new(&wf.views, set, &wf.rules, 0);
    let mut walked: BTreeMap<(usize, [u64; 4]), (Combo, Vec<usize>)> = BTreeMap::new();
    let mut mismatches = 0_u64;
    let trades_of = |cb: &Combo, rows: &[usize], col: usize| -> Vec<Trade> {
        let out = t.outs.get(cb.ni);
        let close = t.close.get(cb.ni);
        let fl = t.fl.get(cb.ni);
        rows.iter()
            .filter_map(|&j| {
                let o = out?.get(j)?;
                let c = fam.cands.get(*t.kidx.get(j)?)?;
                let x = j * ncol + col;
                Some(Trade {
                    day: c.day,
                    exit_day: if cfg!(feature = "m_pos_mk") { c.day } else { pt.book_day.get(j).copied().unwrap_or(c.day) },
                    close_s: close?.get(x).copied().unwrap_or(0),
                    net: o.net.get(col).copied().unwrap_or(0),
                    net_alt: 0,
                    rd_bp: i32::from(o.rd_bp.get(col).copied().unwrap_or(0)),
                    hole: fl?.get(x).copied().unwrap_or(0) & 4 != 0,
                    id: o.id,
                    mark_prev: 0,
                })
            })
            .collect()
    };
    let walk = |trades: &[Trade]| -> Result<Acc, String> {
        let mut acc = Acc::new(set, &wf.views, &wf.rules);
        acc.reset();
        for tr in trades {
            acc.add(tr).map_err(|e| format!("acc add: {e}"))?;
        }
        acc.finish().map_err(|e| format!("acc finish: {e}"))?;
        Ok(acc)
    };
    for (vi, v) in inp.views.iter().enumerate() {
        let cv = wf.views.views.iter().position(|x| x.id == v.id).ok_or_else(|| format!("view {} is not a common_wf VIEWS-STD row", v.id))?;
        let cview = wf.views.views.get(cv).ok_or("view")?;
        for col in 0..ncol {
            let Some(l) = merged.get(vi * ncol + col) else { continue };
            for (key, code, n) in l.entries() {
                let cb = dec.decode(code)?;
                let kept_bits = cb.outer.and_then(|i| values.get(i)).map(|x| &x.kept);
                let rows = combo_rows(&krows, kept_bits, core, &cb);
                let trades = trades_of(&cb, &rows, col);
                let acc = walk(&trades)?;
                let lr = acc.learn(cview.win).map_err(|e| format!("learn: {e}"))?;
                let vals: Vec<i64> = cview.key.iter().map(|c| crit_val(c, &lr, &acc, wf.rules.base_capital, false, &[]).unwrap_or(i64::MIN)).collect();
                if key.0.get(..vals.len()) != Some(vals.as_slice()) {
                    mismatches += 1;
                }
                if walked.contains_key(&(col, code)) {
                    continue;
                }
                let mut tk = Tickets::new();
                tk.push(Ticket { grid: 0, stage: u8::from(cb.outer.is_some()), set: ReadSet::EMPTY }, &wf.rules).map_err(|e| format!("ticket: {e}"))?;
                let kmax = wf.views.views.iter().map(|x| x.k as u64).max().unwrap_or(1);
                let pid = Partition(u32::try_from(col + 1).unwrap_or(1));
                for i in 0..n.min(kmax) {
                    let c2 = Code(code).plus(i).ok_or("code overflow")?;
                    cl.offer(&wf.views, pid, &tk, WfCode(c2.0), &acc).map_err(|e| format!("offer: {e}"))?;
                }
                walked.insert((col, code), (cb, rows));
            }
        }
    }
    lines.push(format!("re-walk: {} distinct (level, combo) entries walked over the whole period and offered to common_wf (positional RULES: ARRIVAL, carried yes, marks none); rewalk_equal mismatches {mismatches}", walked.len()));
    // --- the report ---
    let combo_words = |cb: &Combo| -> String {
        let lad = if long { &inp.core.0 } else { &inp.core.1 };
        let lv = |li: usize, j: u16| if j == 0 { "OFF".to_string() } else { lad.l.get(li).map_or_else(String::new, |x| x.level_text(usize::from(j))) };
        let outer = cb.outer.and_then(|i| values.get(i)).map_or_else(|| "every other dial OFF".to_string(), |x| format!("{} (every other dial OFF)", x.label));
        format!("RSI 14 {}; CCI 20 {}; %R 14 {}; {}; N = {}", lv(0, cb.rsi), lv(1, cb.cci), lv(2, cb.wr), outer, pc.n_grid.get(cb.ni).copied().unwrap_or(0))
    };
    let mut pick_rows = Vec::new();
    let mut report_rows = Vec::new();
    let mut twin_rows = Vec::new();
    let mut co = CtxOut { lines: Vec::new(), searches: 0, wf_passed: 0, tt_passed: 0, both: 0, mismatches };
    let honest = wf.ws.honest();
    let sniper_ids: Vec<&str> = inp.views.iter().map(|v| v.id.as_str()).collect();
    // W4 REPAIR D-W4-1 (plan s4.3a B2 "the 2026 test-year word prints with it and without it"): the year unit holding the data end (the
    // last session's month)
    let de_unit = pc.sess_mk.last().and_then(|&mk| set.tab.units.iter().position(|u| u.from <= mk && mk <= u.to));
    let de_label = de_unit.and_then(|u| set.tab.units.get(u)).map_or_else(|| "data-end".to_string(), |u| u.label.clone());
    let slot_tag = SlotRule::default().tag();
    // twin slots: every GROUP with a priced row (FU-9)
    let mut grp_rows: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for (j, r) in krows.iter().enumerate() {
        grp_rows.entry(r.sess).or_default().push(j);
    }
    let tslots: Vec<TwinSlot> = grp_rows.iter().filter_map(|(g, v)| fam.cands.get(*t.kidx.get(*v.first()?)?).map(|c| (*g, common_feat::cal::civil(c.day).0, u32::try_from(v.len()).unwrap_or(u32::MAX)))).collect();
    // --- W4 REPAIR r3 (check-wave4-S9-r2.md section 3 = D-W4-3 item 3; MAIN ruling 16 Nov ~14:3x IST): the FIXED-SLOT reading BESIDE every
    // pick (never the headline: base / N stays it). Every pick's own rows at its N and level, collected first (the picks below are the same
    // kept tops), re-priced once each under SlotRule::Fixed on both protection paths (the same Book), then re-walked per pick ---
    let sess_days: Vec<i32> = pc.ctx.sessions.iter().map(|s| s.day).collect();
    let walked_ix: Vec<usize> = pc.ctx.sessions.iter().enumerate().filter(|(_, s)| s.entry).map(|(i, _)| i).collect();
    let pcand_of: BTreeMap<usize, PosCand> = pcands.iter().map(|c| (c.0, *c)).collect();
    let mut fx_want: BTreeMap<FxCell, PosCand> = BTreeMap::new();
    for col in 0..levels.len() {
        let pid = u32::try_from(col + 1).unwrap_or(1);
        for (vi, v) in wf.views.views.iter().enumerate() {
            if !set.has(v.win) || !sniper_ids.contains(&v.id.as_str()) || v.role == Role::Miss {
                continue;
            }
            let Some(top) = cl.kept(Partition(pid), ViewId(u16::try_from(vi).unwrap_or(0))).first() else { continue };
            let cb = dec.decode(top.key.code.0)?;
            let rows = walked.get(&(col, top.key.code.0)).map(|x| x.1.clone()).unwrap_or_else(|| {
                let kept_bits = cb.outer.and_then(|i| values.get(i)).map(|x| &x.kept);
                combo_rows(&krows, kept_bits, core, &cb)
            });
            for &j in &rows {
                let ci = *t.kidx.get(j).ok_or("fixed-slot beside: table row")?;
                let pcand = *pcand_of.get(&ci).ok_or("fixed-slot beside: a pick row with no priced candidate")?;
                fx_want.insert((j, cb.ni, col), pcand);
            }
        }
    }
    let fx_t0 = Instant::now();
    let (fx_lv, mut fx_us) = price_fixed_beside(pc.env, fam, &sess_days, &fx_want, long, pc.slots, levels, pc.account, spec.threads)?;
    let fx_wall = fx_t0.elapsed().as_secs_f64();
    fx_us.sort_unstable();
    let fq = |p: usize| fx_us.get((fx_us.len().saturating_sub(1) * p) / 1000).copied().unwrap_or(0);
    let fx_cells_differ = fx_lv.iter().filter(|((j, ni, col), x)| t.outs.get(*ni).and_then(|v| v.get(*j)).is_none_or(|o| o.net.get(*col).copied() != Some(x.net))).count();
    let fx_rows: std::collections::BTreeSet<usize> = fx_want.keys().map(|k| k.0).collect();
    let fx_line = format!(
        "fixed-slot beside (fixed Rs 20 L / N, SlotRule::Fixed; D-W4-3 item 3, MAIN ruling 16 Nov; never the headline: base / N stays it): {} pick cells ({} distinct rows x the picks' N and levels) re-priced through opt_pos::pos::run on both protection paths (the worse booked), the same Book; {:.1} s wall on {} threads; per cell p50 / p99 / max {} / {} / {} ms; {fx_cells_differ} cells' nets differ from the headline base / N",
        fx_lv.len(),
        fx_rows.len(),
        fx_wall,
        spec.threads,
        ms3(fq(500)),
        ms3(fq(990)),
        ms3(fx_us.last().copied().unwrap_or(0))
    );
    lines.push(fx_line.clone());
    // the beside cells as rows (proof: every re-priced cell, its fixed reading beside the headline's own cell; never read back)
    let fx_db: Vec<Vec<String>> = fx_lv
        .iter()
        .map(|(&(j, ni, col), x)| {
            let head = t.outs.get(ni).and_then(|v| v.get(j));
            vec![
                spec.run_id.clone(),
                t.kidx.get(j).map_or_else(|| "-".to_string(), ToString::to_string),
                j.to_string(),
                pc.n_grid.get(ni).copied().unwrap_or(0).to_string(),
                levels.get(col).copied().unwrap_or(0).to_string(),
                x.net.to_string(),
                x.rd_bp.to_string(),
                format!("0x{:x}", x.flags),
                if x.last_known > 0 { fmt_ist(x.last_known) } else { "-".to_string() },
                head.and_then(|o| o.net.get(col)).map_or_else(|| "-".to_string(), ToString::to_string),
                head.and_then(|o| o.rd_bp.get(col)).map_or_else(|| "-".to_string(), ToString::to_string),
                format!("labels: SN1-POS-{} row labels", spec.tf),
            ]
        })
        .collect();
    db.write(
        &format!("FIXED-BESIDE-POS-{}-{side}-{}.tsv", spec.tf, rule.id()),
        &["run", "cand", "table_row", "n", "level_bp", "net_fixed", "rd_bp_fixed", "flags_fixed", "last_known_fixed_ist", "net_headline", "rd_bp_headline", "labels"],
        &fx_db,
    )?;
    let mut fx_later_all = 0u64;
    for (col, bp) in levels.iter().enumerate() {
        let pid = u32::try_from(col + 1).unwrap_or(1);
        let p = wf.rules.partition(pid).map_err(|e| e.to_string())?.clone();
        let search = format!("{cname}|{}%", bp / 100);
        co.searches += 1;
        let mut out: Vec<String> = vec![format!("--- search {search} ({}; its own {} index candles; {}; N slots of Rs 20 L / N, headline N {}; 2018-09..2026-10; run {}) ---", inp.instr.underlying(), spec.tf, inp.words.line("INSTR", &[]), inp.instr.rows.get("headline_n").map_or("?", String::as_str), spec.run_id)];
        let mut verdicts: BTreeMap<String, bool> = BTreeMap::new();
        for (vi, v) in wf.views.views.iter().enumerate() {
            let vid = ViewId(u16::try_from(vi).unwrap_or(0));
            if !set.has(v.win) || !sniper_ids.contains(&v.id.as_str()) {
                continue;
            }
            let cn = cl.counts(Partition(pid), vid);
            let kept_l = cl.kept(Partition(pid), vid);
            let row = wf.ws.row(v.win);
            if v.role == Role::Miss {
                out.push(format!("{} closest misses ({} kept of {} offered):", v.id, kept_l.len(), cn.offered));
                for k in kept_l.iter().take(10) {
                    let cb = dec.decode(k.key.code.0)?;
                    out.push(format!("  MISS {} [{}]: {}", Code(k.key.code.0).hex(), combo_words(&cb), common_wf::report::miss_checks(set, v.win, &k.facts).join("; ")));
                }
                continue;
            }
            let Some(top) = kept_l.first() else {
                out.push(format!("{} W07: {}", v.id, common_wf::report::no_pick_line(&wf.words, v, row, &cn, &p, &wf.rules).map_err(|e| e.to_string())?));
                continue;
            };
            let learn = set.learn_range(v.win).unwrap_or((0, -1));
            let pick = PickWin { set: set.set_id(), win: v.win, learn, kind: top.kind, code: top.key.code };
            let cb = dec.decode(top.key.code.0)?;
            let rows = walked.get(&(col, top.key.code.0)).map(|x| x.1.clone()).unwrap_or_else(|| {
                let kept_bits = cb.outer.and_then(|i| values.get(i)).map(|x| &x.kept);
                combo_rows(&krows, kept_bits, core, &cb)
            });
            let trades = trades_of(&cb, &rows, col);
            let acc = walk(&trades)?;
            if acc.facts(Partition(pid)) != top.facts {
                return Err(format!("{search} {}: the report re-walk's facts differ from the kept facts (rewalk_equal)", v.id));
            }
            out.push(format!("{} PICK {} [{}] key {:?}", v.id, Code(top.key.code.0).hex(), combo_words(&cb), top.key.parts.get(..usize::from(top.key.n)).unwrap_or(&[])));
            for h in &honest {
                let facts = acc.facts(Partition(pid));
                let rv = common_wf::verdict(set, *h, pick, &facts, &wf.rules).map_err(|e| e.to_string())?;
                for ln in common_wf::report::reading_lines(&wf.words, set, *h, &rv, &pick, row, &facts, &acc, &wf.rules, &p, "", None).map_err(|e| e.to_string())? {
                    out.push(format!("{} {}: {ln}", v.id, wf.ws.row(*h).id));
                }
            }
            out.push(format!("{} s12: {}", v.id, inp.words.line("S12", &[])));
            // the positional facts of the pick's own trades at its N and level
            let fx = |bit: u32| rows.iter().filter(|&&j| pt.flx.get(cb.ni).and_then(|f| f.get(j * ncol + col)).is_some_and(|x| x & bit != 0)).count();
            out.push(format!(
                "{} positional: {} trades; close_split {}; roll_refused_cash {}; roll_no_itm {}; roll_close_incomplete {}; settled {}; unclosable {}; holes {}; cash_short {}; carried {}; open_at_data_end {}; slot rule {slot_tag} (base / N, MAIN-RULINGS-S7-S10 item 2; fixed Rs 20 L / N beside on G-ROLL)",
                v.id,
                trades.len(),
                fx(pflag::CLOSE_SPLIT),
                fx(pflag::ROLL_REFUSED_CASH),
                fx(pflag::ROLL_NO_ITM),
                fx(pflag::ROLL_CLOSE_INCOMPLETE),
                fx(pflag::SETTLED),
                fx(pflag::UNCLOSABLE),
                fx(pflag::HOLE),
                fx(pflag::ACCOUNT_CAP),
                fx(pflag::CARRY),
                fx(pflag::OPEN_AT_DATA_END)
            ));
            // W4 REPAIR D-W4-1 (plan s4.3a B2): the data-end test-year word WITH and WITHOUT the pick's open_at_data_end trades (the
            // same common_wf re-walk with those rows left out; the word is common_wf's year item)
            let is_ode = |j: usize| pt.flx.get(cb.ni).and_then(|f| f.get(j * ncol + col)).is_some_and(|x| x & pflag::OPEN_AT_DATA_END != 0);
            let rows_wo: Vec<usize> = rows.iter().copied().filter(|&j| !is_ode(j)).collect();
            let trades_wo = trades_of(&cb, &rows_wo, col);
            let acc_wo = walk(&trades_wo)?;
            let n_ode = rows.len() - rows_wo.len();
            let net_ode = trades.iter().map(|x| x.net).sum::<i64>() - trades_wo.iter().map(|x| x.net).sum::<i64>();
            let (w_with, w_wo) = match de_unit {
                Some(u) => (unit_word(&set.tab.units, u, &acc.facts(Partition(pid))), unit_word(&set.tab.units, u, &acc_wo.facts(Partition(pid)))),
                None => ("no year unit holds the data end".to_string(), "no year unit holds the data end".to_string()),
            };
            out.push(format!("{} data end (plan s4.3a B2): the {de_label} test-year word with the open_at_data_end trades: {w_with}; without them ({n_ode} trades, net {} left out): {w_wo}", v.id, common_wf::words::fmt_rs(net_ode)));
            // W4 REPAIR r3 (D-W4-3 item 3; MAIN ruling 16 Nov): the FIXED-SLOT reading BESIDE (never the headline): the pick's own rows at
            // its N and level under SlotRule::Fixed (both protection paths, priced above), each booked at the headline's session, or at the
            // first walked session at or after the day the fixed reading's money is last known when that is later (never earlier than
            // known; a running maximum keeps the booking order), re-walked through common_wf; the words are the pick's own test years
            let (w_fx, fx_differ, fx_later) = {
                let out_n = t.outs.get(cb.ni).ok_or("outcomes")?;
                let close_n = t.close.get(cb.ni).ok_or("closes")?;
                let mut tr_fx: Vec<Trade> = Vec::with_capacity(rows.len());
                let (mut differ, mut later, mut run_day) = (0u64, 0u64, i32::MIN);
                for &j in &rows {
                    let (Some(o), Some(c)) = (out_n.get(j), t.kidx.get(j).and_then(|&ci| fam.cands.get(ci))) else { continue };
                    let x = fx_lv.get(&(j, cb.ni, col)).copied().ok_or_else(|| format!("{search} {}: the fixed-slot beside has no level for row {j}", v.id))?;
                    let head_day = if cfg!(feature = "m_pos_mk") { c.day } else { pt.book_day.get(j).copied().unwrap_or(c.day) };
                    let known_day = if x.last_known > 0 {
                        let d = idx_cal::storeio::ist_of(x.last_known).0;
                        booking_session(&sess_days, &walked_ix, d).and_then(|si| sess_days.get(si).copied()).ok_or_else(|| format!("fixed-slot beside: no walked session at or after {} (row {j})", idx_cal::day::fmt(d)))?
                    } else {
                        head_day
                    };
                    let day = head_day.max(known_day).max(run_day);
                    run_day = day;
                    let close_s = if day == head_day { close_n.get(j * ncol + col).copied().unwrap_or(0) } else { u32::try_from(pc.env.fno_end_sod(day) - 1).unwrap_or(0) };
                    later += u64::from(day != head_day);
                    differ += u64::from(o.net.get(col).copied() != Some(x.net));
                    tr_fx.push(Trade { day: c.day, exit_day: day, close_s, net: x.net, net_alt: 0, rd_bp: i32::from(x.rd_bp), hole: pflag::cash_bits(x.flags) & 4 != 0, id: o.id, mark_prev: 0 });
                }
                let f_fx = walk(&tr_fx)?.facts(Partition(pid));
                let words = set.spec(v.win).map_or_else(|| "no test years in this window".to_string(), |s| s.test_list.iter().map(|&u| unit_word(&set.tab.units, usize::from(u), &f_fx)).collect::<Vec<_>>().join("; "));
                (words, differ, later)
            };
            fx_later_all += fx_later;
            out.push(format!("{} fixed-slot beside (fixed Rs 20 L / N): {w_fx}", v.id));
            let (_, vd) = common_wf::verdict(set, v.win, pick, &top.facts, &wf.rules).map_err(|e| e.to_string())?;
            let passed = matches!(vd, Verdict::Passed { .. });
            if v.role == Role::Headline && row.honest {
                verdicts.insert(row.id.clone(), passed);
                let test = (row.test_from.unwrap_or(0), row.test_to.unwrap_or(-1));
                let own: Vec<(i32, i64)> = trades.iter().map(|x| (common_wf::date::mk_of_day(x.exit_day), x.net)).collect();
                let (own_all, own_net, own_n) = test_figures(&own, test);
                let mut per_year: BTreeMap<i32, u32> = BTreeMap::new();
                for x in &trades {
                    *per_year.entry(common_feat::cal::civil(x.day).0).or_insert(0) += 1;
                }
                let out_n = t.outs.get(cb.ni).ok_or("outcomes")?;
                let mut nets = Vec::with_capacity(pc.twin.1 as usize);
                let (mut k_net, mut k_all) = (0_u32, 0_u32);
                for world in 0..pc.twin.1 {
                    let d = twin_draws_pos(pc.twin.0, world, &tslots, &per_year);
                    let tw: Vec<(i32, i64)> = d
                        .iter()
                        .filter_map(|(g, c)| {
                            let j = *grp_rows.get(g)?.get(usize::try_from(c.saturating_sub(1)).ok()?)?;
                            let o = out_n.get(j)?;
                            Some((o.mk, o.net.get(col).copied().unwrap_or(0)))
                        })
                        .collect();
                    let (all_pos, net, _) = test_figures(&tw, test);
                    k_net += u32::from(net >= own_net);
                    k_all += u32::from(all_pos);
                    nets.push(net);
                }
                nets.sort_unstable();
                out.push(format!(
                    "{} twin: random-entry twin per group ({} draws, seed {}; TEST months only): random entries did as well in {k_net} of {}; every test year positive in {k_all} of {}; twin test net p5 / p50 / p95 {} / {} / {}; the pick: {own_n} test trades, test net {}, every test year positive: {}",
                    v.id,
                    pc.twin.1,
                    pc.twin.0,
                    pc.twin.1,
                    pc.twin.1,
                    common_wf::words::fmt_rs(pct_of(&nets, 50)),
                    common_wf::words::fmt_rs(pct_of(&nets, 500)),
                    common_wf::words::fmt_rs(pct_of(&nets, 950)),
                    common_wf::words::fmt_rs(own_net),
                    if own_all { "yes" } else { "no" }
                ));
                twin_rows.push(vec![spec.run_id.clone(), search.clone(), v.id.clone(), pc.twin.1.to_string(), k_net.to_string(), k_all.to_string(), own_net.to_string(), own_n.to_string(), pc.labels.to_string()]);
            }
            pick_rows.push(vec![
                spec.run_id.clone(),
                search.clone(),
                v.id.clone(),
                format!("{:?}", v.role),
                Code(top.key.code.0).hex(),
                pc.n_grid.get(cb.ni).copied().unwrap_or(0).to_string(),
                combo_words(&cb),
                format!("{:?}", top.key.parts.get(..usize::from(top.key.n)).unwrap_or(&[])),
                trades.len().to_string(),
                common_wf::report::verdict_code(&vd),
                slot_tag.to_string(),
                n_ode.to_string(),
                w_with.clone(),
                w_wo.clone(),
                w_fx.clone(),
                fx_differ.to_string(),
                fx_later.to_string(),
                pc.labels.to_string(),
            ]);
        }
        let (w_ok, t_ok) = (verdicts.get("WF").copied().unwrap_or(false), verdicts.get("TT").copied().unwrap_or(false));
        co.wf_passed += u64::from(w_ok);
        co.tt_passed += u64::from(t_ok);
        co.both += u64::from(w_ok && t_ok);
        for (i, l) in out.iter().enumerate() {
            report_rows.push(vec![spec.run_id.clone(), search.clone(), i.to_string(), l.clone(), pc.labels.to_string()]);
        }
        co.lines.extend(out);
    }
    db.write(
        &format!("SN-PICKS-POS-{}-{side}-{}.tsv", spec.tf, rule.id()),
        &["run", "search", "view", "role", "code", "n", "combo", "key", "trades", "verdict_code", "slot_rule", "open_at_data_end_trades", "data_end_year_with_open_at_data_end", "data_end_year_without_open_at_data_end", "fixed_slot_beside_test_years", "fixed_slot_beside_nets_differ", "fixed_slot_beside_booked_later", "labels"],
        &pick_rows,
    )?;
    db.write(&format!("SN-REPORT-POS-{}-{side}-{}.tsv", spec.tf, rule.id()), &["run", "search", "line_no", "line", "labels"], &report_rows)?;
    db.write(&format!("SN9-POS-{}-{side}-{}.tsv", spec.tf, rule.id()), &["run", "search", "view", "draws", "k_net_at_least", "k_every_test_year_pos", "pick_test_net", "pick_test_trades", "labels"], &twin_rows)?;
    let fx_last = format!("fixed-slot beside: {fx_later_all} pick rows booked later than the headline's session over every pick line of {cname} (never earlier than the fixed reading's money is known)");
    let fx_sn1 = format!("{cname}: {fx_line}; {fx_later_all} pick rows booked later than the headline's session");
    let mut all_lines = lines;
    all_lines.extend(co.lines);
    all_lines.push(fx_last);
    co.lines = all_lines;
    Ok((co, fx_sn1))
}

#[cfg(test)]
mod tests {
    //! Named cases of the index positional wiring (red first: m_pos_future_refusal, m_pos_mk).
    use super::*;
    use opt_pos::pos::{LegOut, StreamOut};

    fn stream_with(kind0: &'static str, flags: &[(&'static str, u32)]) -> StreamOut {
        let mut o = StreamOut { legs: vec![LegOut { kind: kind0, ..LegOut::default() }], ..StreamOut::default() };
        for &(k, v) in flags {
            o.flags.insert(k, v);
        }
        o
    }

    /// CRITIC-POSITIONAL B2 / plan s4.3a: an entry refused at T (no_itm_fit) is a DECIDE refusal; every LATER fact (a roll with no ITM
    /// fit, a roll refused for cash, a roll close unfinished, a hole, open after the run) keeps the row (flagged). Red under
    /// `m_pos_future_refusal` (a later fact removes the row: FIRST would move to a later candle).
    #[test]
    fn s7w4_later_facts_never_remove_a_positional_row() {
        assert_eq!(classify(&stream_with("entry_refused", &[]), false), Class::Decide("no_itm_fit"));
        assert_eq!(classify(&stream_with("entry", &[]), false), Class::Keep);
        for k in LATER_FLAGS {
            assert_eq!(classify(&stream_with("entry", &[(k, 1)]), false), Class::Keep, "{k} is a LATER fact: kept");
        }
        assert_eq!(classify(&stream_with("entry", &[]), true), Class::Keep, "a hole under a later leg: kept (AFR-2 flag)");
    }

    /// 6.1 / s4.3a B3: the booking session is the running maximum, in group order, of each group's latest last-known session (never
    /// before its own entry). Red under `m_pos_mk` (each group booked at its entry session).
    #[test]
    fn s7w4_booking_is_the_running_max_of_last_known_sessions() {
        // group 0: entered session 3, last known 10; group 1: entered 5 (inside group 0's hold), last known 7 -> books at 10, not 7
        assert_eq!(booking(&[10, 7, 12], &[3, 5, 11]), vec![10, 10, 12]);
    }

    /// OUT-POS: lane B's layout, 792 B for the 15-value grid x 5 levels.
    #[test]
    fn s7w4_out_pos_row_is_792_bytes() {
        assert_eq!(record_width_pos(15, 5), 792);
        let lv = vec![vec![PLevel { net: -5, rd_bp: 7, last_known: 2_000_000, flags: pflag::CLOSE_SPLIT, rolls: 1, nights: 3, ..PLevel::default() }; 5]; 15];
        let b = encode_pos(&lv, 1_000_000, 0, (4, 4)).unwrap();
        assert_eq!(b.len(), 792);
        assert_eq!(i64::from_le_bytes(b[0..8].try_into().unwrap()), -5);
    }

    /// Lane B's S5 run-down, the roll legs JOINED (the positional canaries of 15 Nov 16:08 / 16:10 stopped on a u16 overflow): the worst
    /// mark of the held trip plus the realised gross of the trips before it, in bp of the FIRST premium, saturating with a flag. Red under
    /// `m_pos_rd_alone` (each trip alone against its own premium: the earlier loss dropped, never saturating).
    #[test]
    fn s7w4_rundown_joins_the_roll_legs_lane_b_s5() {
        // one trip: 100 units bought for Rs 1,000 (1,00,000 paisa), lowest LOW 600 paisa: mark -40,000 -> 4,000 bp
        assert_eq!(join_rundown(&[(100_000, 100, 20_000, Some(600))]), (4_000, false));
        // trip 1 lost 50,000 gross (low 900: its own mark -10,000); the roll trip paid 3,00,000 for 300 units, low 500: -50,000 + 1,50,000
        // - 3,00,000 = -2,00,000 -> 20,000 bp of the FIRST premium (alone it would read 5,000 bp of its own premium)
        assert_eq!(join_rundown(&[(100_000, 100, -50_000, Some(900)), (300_000, 300, -10_000, Some(500))]), (20_000, false));
        // EXTREME: the roll trip sized 10x the first premium and worthless at its low: -50,000 - 10,00,000 = -10,50,000 = 1,05,000 bp > u16
        assert_eq!(join_rundown(&[(100_000, 100, -50_000, Some(900)), (1_000_000, 1_000, -1_000_000, Some(0))]), (u16::MAX, true));
        // an earlier PROFIT offsets the roll trip's mark: +80,000 realised, then 1,00,000 for 100 units at low 500: +30,000 (no run-down);
        // the worst is trip 1's own mark (low 900: -10,000 = 1,000 bp)
        assert_eq!(join_rundown(&[(100_000, 100, 80_000, Some(900)), (100_000, 100, 5_000, Some(500))]), (1_000, false));
        // a trip with no print in its held window adds its realised result only; no trips at all reads 0
        assert_eq!(join_rundown(&[(100_000, 100, -30_000, None), (100_000, 100, 0, Some(800))]), (5_000, false));
        assert_eq!(join_rundown(&[]), (0, false));
        // never negative: a trip whose low stays above its average price reads 0
        assert_eq!(join_rundown(&[(100_000, 100, 10_000, Some(1_000))]), (0, false));
    }

    /// Plan s8 S7 F8 on the positional legs: an entry or roll-open passes when its A2 quantity is whole lots, above 0 and at or below
    /// that day's one-order maximum; exactly AT the maximum is one order (s3.5 (11)); one lot above it fails. Red under `m_pos_f8`.
    #[test]
    fn s7w4_positional_f8_gate_on_entries_and_roll_opens() {
        let leg = |kind: &'static str, qty_a2: u64, lot: u64, max_order: i64| LegFact { kind, qty_a2, lot, max_order, ..LegFact::default() };
        // NIFTY 2026-10-05 era: freeze 3,510 = 54 lots of 65 -> max 3,510; BANKNIFTY 900 / 30 -> max 900
        assert!(f8_ok(&leg("entry", 3_510, 65, 3_510)), "exactly at the maximum: one order");
        assert!(f8_ok(&leg("roll_open", 900, 30, 900)), "a roll-open exactly at the maximum");
        assert!(!f8_ok(&leg("entry", 3_575, 65, 3_510)), "one lot above the maximum fails F8");
        assert!(!f8_ok(&leg("roll_open", 930, 30, 900)), "a roll-open one lot above fails F8");
        assert!(!f8_ok(&leg("entry", 100, 75, 1_800)), "not whole lots");
        assert!(!f8_ok(&leg("entry", 0, 75, 1_800)), "0 units is never a pass");
        assert!(leg("entry", 75, 75, 1_800).is_buy() && leg("roll_open", 75, 75, 1_800).is_buy());
        assert!(!leg("roll_open_refused_cash", 75, 75, 1_800).is_buy() && !leg("exit", 0, 75, 1_800).is_buy(), "a refused open sends nothing; closes are counted as close_split");
        assert_eq!(ms3(1_234_567), "1234.567");
        assert_eq!(ms3(5), "0.005");
    }

    /// The run-down's range-min over a contract-day (the sparse table equals a scan on every window).
    #[test]
    fn s7w4_day_min_equals_a_scan() {
        let rows: Vec<opt_pos::secs::Sec> = (0..200).map(|i| opt_pos::secs::Sec { ts: i * 1_000_000, high: 0, low: 1_000 + (i * 7919) % 97, close: 0, vol: 1 }).collect();
        let m = DayMin::new(&rows);
        for a in (0..200).step_by(7) {
            for b in (a..210).step_by(11) {
                let scan = rows.iter().filter(|r| r.ts >= a * 1_000_000 && r.ts < b * 1_000_000).map(|r| r.low).min();
                assert_eq!(m.min_in(a * 1_000_000, b * 1_000_000), scan, "[{a}, {b})");
            }
        }
    }

    /// Plan s4.3a B2 / s4.7 (15) (S7 resumed again): units held after the last session (`TripOut.data_end`, the pricer's
    /// `open_at_data_end` count) set OPEN_AT_DATA_END, never UNCLOSABLE (an expiry leftover at bullet 1) and never SETTLED; an expiry
    /// leftover keeps its bits; open_at_data_end is a LATER fact (the row is kept).
    #[test]
    fn s7w4_open_at_data_end_is_flagged_never_unclosable() {
        let mut o = stream_with("entry", &[("open_at_data_end", 1)]);
        o.trips.push(opt_pos::pos::TripOut { leftover: 600, data_end: true, ..opt_pos::pos::TripOut::default() });
        assert_eq!(leftover_bits(&o), pflag::OPEN_AT_DATA_END);
        let mut e = stream_with("entry", &[]);
        e.trips.push(opt_pos::pos::TripOut { leftover: 600, ..opt_pos::pos::TripOut::default() });
        assert_eq!(leftover_bits(&e), pflag::UNCLOSABLE);
        e.trips.push(opt_pos::pos::TripOut { leftover: 5, settled: true, ..opt_pos::pos::TripOut::default() });
        assert_eq!(leftover_bits(&e), pflag::UNCLOSABLE | pflag::SETTLED);
        assert!(LATER_FLAGS.contains(&"open_at_data_end"));
        assert_eq!(classify(&o, false), Class::Keep);
    }

    /// Plan 6.1 / s4.3a B3 with common_wf's calendar (walked = an entry session; a SPECIAL session only under a ruling, R11): the booking
    /// session is the first WALKED session at or after the day the money is last known. The NIFTY 60m LONG positional canary (final3)
    /// had a carry that filled on the Saturday budget session 2020-02-01 (a special session): booked at 2020-02-03, never on 2020-02-01
    /// (common_wf refused that as TradeOnUnwalkedSession) and never earlier than known. Red under `m_pos_book_unwalked`.
    #[test]
    fn s7w4_booking_session_is_the_next_walked_session() {
        let d = |s: &str| common_feat::cal::parse_ymd(s).unwrap_or(0);
        // sessions: 2020-01-31 (regular), 2020-02-01 (special, Saturday budget), 2020-02-03, 2020-02-04 (regular)
        let days = [d("2020-01-31"), d("2020-02-01"), d("2020-02-03"), d("2020-02-04")];
        let walked = [0usize, 2, 3];
        assert_eq!(booking_session(&days, &walked, d("2020-02-01")), Some(2), "a special session books at the next walked session");
        assert_eq!(booking_session(&days, &walked, d("2020-02-02")), Some(2), "a day with no session books at the next walked session, never earlier");
        assert_eq!(booking_session(&days, &walked, d("2020-01-31")), Some(0));
        assert_eq!(booking_session(&days, &walked, d("2020-02-04")), Some(3));
        assert_eq!(booking_session(&days, &walked, d("2020-02-05")), None, "no walked session follows: refused by name by the caller");
    }

    /// W4 REPAIR D-W4-1 (plan s4.3a B2 "the 2026 test-year word prints with it and without it"): the data-end year word reads exactly
    /// like common_wf's year items (the W10 figures line prints "2026 -Rs 31,232.02" / "2026 no trade"), so the pick line's "with"
    /// word equals the figures line's own 2026 item; OPEN_AT_DATA_END comes from the pricer's outcome only (a closed trade never has it).
    #[test]
    fn w4r_data_end_year_word_reads_like_common_wf_items() {
        let units = vec![common_wf::windows::Unit { from: 2025 * 12, to: 2025 * 12 + 11, label: "2025".into() }, common_wf::windows::Unit { from: 2026 * 12, to: 2026 * 12 + 9, label: "2026".into() }];
        let cell = |net: i64, trades: u64| common_wf::UnitCell { net, trades, ..common_wf::UnitCell::EMPTY };
        let f = common_wf::Facts { set: common_wf::SetId(0), partition: common_wf::Partition(1), ident: 0, units: vec![cell(5, 1), cell(-3_123_202, 4)], booked: 1, second: 1, touched: 3, nodata: 0, missing: 0 };
        assert_eq!(unit_word(&units, 1, &f), "2026 -Rs 31,232.02");
        assert_eq!(unit_word(&units, 0, &f), "2025 Rs 0.05");
        let none = common_wf::Facts { units: vec![cell(0, 0), cell(0, 0)], ..f.clone() };
        assert_eq!(unit_word(&units, 1, &none), "2026 no trade");
        let miss = common_wf::Facts { missing: 2, ..none.clone() };
        assert_eq!(unit_word(&units, 1, &miss), "2026 no session in the data");
        let nod = common_wf::Facts { nodata: 2, ..none };
        assert_eq!(unit_word(&units, 1, &nod), "2026 no data for this instrument");
        assert_eq!(unit_word(&units, 70, &f), " no trade", "a unit past the table reads empty, never a panic");
        let mut closed = stream_with("entry", &[]);
        closed.trips.push(opt_pos::pos::TripOut { leftover: 0, ..opt_pos::pos::TripOut::default() });
        assert_eq!(leftover_bits(&closed) & pflag::OPEN_AT_DATA_END, 0, "a closed trade of a data-end group is not open_at_data_end");
    }

    /// W4 REPAIR D-W4-1 / D-W4-3 at the host seam: the sentinel is `opt_pos::pos::DATA_END` (no exit decision: i64::MAX, after every
    /// candle end) and the slot rule the positional mode passes is the one headline rule (base / N).
    #[test]
    fn w4r_positional_seam_sentinel_and_slot_rule() {
        assert_eq!(opos::DATA_END, i64::MAX);
        assert_eq!(SlotRule::default(), SlotRule::BaseOverN);
        assert_eq!(SlotRule::default().tag(), "base_over_n");
    }

    /// W4 REPAIR r3 (check-wave4-S9-r2.md D-R2-1; the NIFTY 60m LONG positional canary's 6 data-end rows): a data-end valuation that read
    /// a DATA-law hole (`TripOut { data_end, hole }`, the pricer's outcome) sets HOLE on the booked level although NO leg spans a hole; its
    /// cash bits carry bit 4 (common_wf's `Trade.hole`: the AFR-2 second reading leaves a winning one out) and OPEN_AT_DATA_END stays; the
    /// value is untouched. A clean data-end valuation sets no HOLE, and only a data-end valuation's hole counts (`t.data_end && t.hole`).
    /// Red under `m_idx_data_end_nohole` (the legs only: the w4r reading).
    #[test]
    fn w4r3_level_of_sets_hole_for_a_data_end_valuation_on_a_hole() {
        let book = opos::Book::new("NIFTY", BTreeMap::new(), Vec::new(), opt_pos::ladder::Ladder::default(), opt_pos::chain::Folders::default(), opt_pos::freeze::FreezeTable::default(), BTreeMap::new(), opt_pos::sess::Sessions::default(), None);
        let run_of = |data_end: bool, hole: bool| {
            let mut o = stream_with("entry", &[("open_at_data_end", u32::from(data_end))]);
            o.trips.push(opt_pos::pos::TripOut { units_in: 975, value_in: 48_750_000, leftover: 975, data_end, hole, net: 30_083_455, charges: 2_000, ..opt_pos::pos::TripOut::default() });
            o.net = 30_083_455;
            RunOut { streams: vec![o], net: 30_083_455 }
        };
        let mut cache: BTreeMap<(String, i32), DayMin> = BTreeMap::new();
        let on_hole = run_of(true, true);
        assert!(on_hole.streams[0].legs.iter().all(|l| !l.hole), "no leg spans a hole: only the valuation read one");
        let (lv, class) = level_of(&book, &on_hole, &on_hole, &[], &mut cache, false).unwrap();
        assert_ne!(lv.flags & pflag::HOLE, 0, "a data-end valuation on a DATA-law hole is a hole trade (HOLE)");
        assert_ne!(pflag::cash_bits(lv.flags) & 4, 0, "common_wf's Trade.hole reads bit 4 of the cash bits");
        assert_ne!(lv.flags & pflag::OPEN_AT_DATA_END, 0, "still open_at_data_end");
        assert_eq!((lv.net, class), (30_083_455, Class::Keep), "the hole labels the trade: the net is untouched, the row kept");
        let clean = run_of(true, false);
        let (lc, _) = level_of(&book, &clean, &clean, &[], &mut cache, false).unwrap();
        assert_eq!(lc.flags & pflag::HOLE, 0, "a clean data-end valuation: no HOLE");
        assert_eq!(lc.flags & !pflag::HOLE, lv.flags & !pflag::HOLE, "HOLE is the only difference");
        let not_de = run_of(false, true);
        let (ln, _) = level_of(&book, &not_de, &not_de, &[], &mut cache, false).unwrap();
        assert_eq!(ln.flags & pflag::HOLE, 0, "only a data-end valuation's hole counts (t.data_end && t.hole)");
    }
}
