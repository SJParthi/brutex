//! One SOLD round trip of an event and the per-event OUTCOME TABLE the account sweep reads in O(1).
//!
//! The trip follows crate v2.1 (THE STANDARD `req70-2s`) and the signed design's rules, read through the crate's own code:
//! - S11 / S8: the sale's protection reference is the last traded second strictly before T (`limits::size_ref`, its LOW); Dhan's market
//!   protection = 10% on the 5-paisa tick (`limits::protection`, the s25 sell-diag's rule, UNVERIFIED for options, inherited); no last
//!   traded second since the segment opened = no trade (`no_reference`).
//! - REQ 70 / REQ 56 / S9: decision T = the event candle's END; the SALE is a crate walk anchored at T (2-second blocks from the candle
//!   end, never inside the signal candle), each block's LOWEST LOW, the share level's part of each block's volume in whole lots, the rest
//!   to the next block, stopped at the exit's anchor or 15:10:00.
//! - S16 (`path::scan_one`, CHANGE C7): per share level and per lot count, the watch starts at E0 (the START of the entry's first filling
//!   block); the reference R(s) = the worse (lower, for a sale) of the first fill price and the running average of the units filled in
//!   blocks starting at or before s; STOP k at distance x = ceil(p0 x k% / 100) above R(s) (HIGH reaches it), TARGET j at distance
//!   floor(p0 x j% / 100) below R(s) (LOW reaches it, a decided exit, C5), TIME exit m at E0 + m minutes; a price trigger's exit anchor =
//!   max(the trigger second's END, E1), a time exit's = max(E0 + m, E1). p0 = the first filling block's price.
//! - S14: an own exit anchored at or after 15:08:00 is rejected; the position waits for the 15:10:00 square-off (one order to the option
//!   session end 15:40:00, no protection limit: Dhan's own order, the sell-diag's rule).
//! - S13: the exit buys back every unit the entry filled up to the exit's anchor; the entry's rest is cancelled there.
//! - The BUY-BACK is a crate walk anchored at the exit's anchor paying each block's HIGHEST HIGH (a gapped stop pays the block's worst),
//!   with Dhan's protection above the last traded second's HIGH before the anchor, until 15:10:00; the rest goes to the square-off.
//! - S15: units still open at the session end are valued at the highest HIGH from E0 to the session end (counted, "unclosable").
//! - Charges: the s25 sell-diag's `charged_sell` (Dhan's estimator at the dearer of the dated and the 2026-10-30 rate sets).

use common_fill::limits::{protection, size_ref, size_units_closest};
use common_fill::path::{EntryView, Rule, scan_one};
use common_fill::{BlockFill, CapMode, Dial, FillPath, Kind, LevelFill, LevelMask, MAXL, Order, RangeMm, SecIn, SecIndex, Seg, Side, Tape, US_S, Why};

use crate::cal::{CLOSE_S, CUT_S, END_S, OPEN_S, SQ_S, at_sod, sod_of};
use crate::cube::{Cube, Sec};
use crate::margin::{VarRow, per_unit_index, per_unit_stock};

/// REQ 70b share dial (bp of each block's volume).
pub const SHARE_BP: [u16; 5] = [1_000, 2_500, 5_000, 7_500, 10_000];
/// v3.2 (addendum 6's exact reduction by rule, `run3 --shares 10,25`): the share levels stage B walks (bit l); default all five (v3.1).
pub static SHARE_MASK: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0x1F);

/// Whether stage B walks share level `l`.
#[inline]
#[must_use]
pub fn share_on(l: usize) -> bool {
    (SHARE_MASK.load(std::sync::atomic::Ordering::Relaxed) >> l) & 1 == 1
}
/// REQ 47 stop dial (% of the first fill price, above R(s)).
pub const STOP_PCT: [i64; 5] = [5, 10, 20, 50, 100];
/// Target dial (% of the first fill price, below R(s)); index 4 = none.
pub const TGT_PCT: [i64; 4] = [30, 50, 80, 90];
/// Time exit (minutes after E0); index 4 = to 15:10.
pub const TIME_MIN: [i64; 4] = [5, 15, 30, 60];
/// Slot count dial.
pub const NSLOT: [i64; 4] = [5, 10, 20, 40];
/// The account: Rs 20 lakh in paisa.
pub const ACCOUNT: i64 = 200_000_000;
/// Exit rules per (level, lot count): 0..5 stop k, 5..9 target j, 9..13 time m.
pub const NRULE: usize = 13;
/// Exit candidates: the 13 rules and 13 = the square-off (no own exit before 15:08:00).
pub const NCAND: usize = 14;
/// "Never" in the u16 second-offset encoding.
pub const NEVER: u16 = u16::MAX;
/// The u16 second offsets count from 09:00:00.
pub const BASE_S: i64 = 9 * 3_600;
/// Dhan's market protection (bp), the s25 sell-diag's 10% (UNVERIFIED for options; inherited).
pub const PROT_BP: u32 = 1_000;
/// The option tick (paisa) for protection limits only (never added to a fill price).
pub const TICK: i64 = 5;
/// The share levels.
pub const NL: usize = 5;

/// The block length of a segment (whole seconds): INDEX options 1 s (REQ 73b / 73c, SNIPER s25 item 4 "stock 2-s, index 1-s",
/// crate v2.1 C9 `Tape::with_block`), STOCK options 2 s (REQ 70, the crate default).
/// The index block length in force (1 s, REQ 73b; `run --index-block 2` = the 2-second comparison run, labelled).
pub static INDEX_BLOCK: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);

#[must_use]
pub fn block_of(kind: u8) -> i64 {
    if kind == 0 { INDEX_BLOCK.load(std::sync::atomic::Ordering::Relaxed) } else { 2 }
}

/// v2.1 (checker F-A): the FILL READING of the run's main grid: 2 = prorate (THE v2.1 READING, the default: a sparse row's volume counts only
/// for its silence's seconds inside the block and at or after the order's send), 1 = anchor (the minimal REQ 56 reading: no volume from
/// before the order's send), 0 = v2 (each row's whole volume in its own second: the replay switch to v2). `run --fill-reading`.
pub static FILL_READING: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(2);

/// The crate reading of a `FILL_READING` code.
#[must_use]
pub const fn cap_mode_of(code: u8) -> CapMode {
    match code {
        0 => CapMode::Row,
        1 => CapMode::Anchor,
        _ => CapMode::Prorate,
    }
}

/// The run's fill reading (the main grid's).
#[must_use]
pub fn run_reading() -> CapMode {
    cap_mode_of(FILL_READING.load(std::sync::atomic::Ordering::Relaxed))
}

/// The printed name of a reading ("v2.1 prorate", "anchor", "v2 row").
#[must_use]
pub const fn reading_name(m: CapMode) -> &'static str {
    match m {
        CapMode::Row => "v2 (row volume all in its second)",
        CapMode::Anchor => "anchor (no volume from before the order is sent)",
        CapMode::Prorate => "v2.1 prorate (row volume spread over its silence; only the block's own seconds at or after the send)",
    }
}

/// The short name of a reading.
#[must_use]
pub const fn reading_short(m: CapMode) -> &'static str {
    match m {
        CapMode::Row => "v2",
        CapMode::Anchor => "anchor",
        CapMode::Prorate => "v21",
    }
}

/// The crate's view of one contract-day's seconds.
pub struct DayTape {
    rows: Vec<SecIn>,
    ix: SecIndex,
    mm: RangeMm,
    segs: Vec<Seg>,
    blk: i64,
    /// v2.1 (FA1): each row's previous PRINT (the last earlier row with volume and a valid price; the day's first print's previous print is
    /// 09:14:59, the second before the segment opens: its volume is spread from the open, the checker's stated reading).
    prev: Vec<i64>,
    /// v2.1: the capacity reading of this tape.
    mode: CapMode,
}

/// v2.1 (FA1): the previous print of every row: the stamp of the last earlier row with volume and a valid price (a repeated stamp shares
/// its second's previous print); the day's first print reads `first_prev`.
#[must_use]
pub fn prev_prints(rows: &[SecIn], first_prev: i64) -> Vec<i64> {
    let mut out = Vec::with_capacity(rows.len());
    let (mut last, mut before_last): (Option<i64>, i64) = (None, first_prev);
    for r in rows {
        let ok = r.vol > 0 && r.high > 0 && r.low > 0 && r.high >= r.low;
        if last == Some(r.ts) {
            out.push(before_last);
            continue;
        }
        out.push(last.unwrap_or(first_prev));
        if ok {
            before_last = last.unwrap_or(first_prev);
            last = Some(r.ts);
        }
    }
    out
}

impl DayTape {
    /// One option contract-day (one segment 09:15:00 .. 15:39:59, no auction: the sell-diag's options `Day`), blocks of `blk` seconds,
    /// read under the run's fill reading ([`FILL_READING`]).
    ///
    /// # Errors
    /// A crate refusal (unsorted rows, a print with volume and an invalid price).
    pub fn new(day: i64, secs: &[Sec], blk: i64) -> Result<Self, String> {
        Self::with_reading(day, secs, blk, run_reading())
    }

    /// The same under an explicit reading (the side-by-side evaluations).
    ///
    /// # Errors
    /// A crate refusal.
    pub fn with_reading(day: i64, secs: &[Sec], blk: i64, mode: CapMode) -> Result<Self, String> {
        let rows: Vec<SecIn> = secs.iter().map(|s| SecIn { ts: at_sod(day, i64::from(s.sod)), high: i64::from(s.high), low: i64::from(s.low), vol: u64::from(s.vol) }).collect();
        let open = at_sod(day, OPEN_S);
        let end = at_sod(day, END_S);
        let segs = vec![Seg { open, last_sec: end - US_S, auction_open: false }];
        let t0 = open.min(rows.first().map_or(i64::MAX, |r| r.ts));
        let t0 = t0 - t0.rem_euclid(US_S);
        let t1 = end.max(rows.last().map_or(0, |r| r.ts));
        let ix = SecIndex::new(&rows, t0, t1).map_err(|e| e.to_string())?;
        let mm = RangeMm::new(&rows);
        let prev = if mode == CapMode::Row { Vec::new() } else { prev_prints(&rows, open - US_S) };
        Ok(Self { rows, ix, mm, segs, blk, prev, mode })
    }

    /// The tape (with this contract-day's block length and capacity reading).
    ///
    /// # Errors
    /// A crate refusal.
    pub fn tape(&self) -> Result<Tape<'_>, String> {
        // v3.1 item 2: the carried fill entitlement (crate v2.3 K1) when the switch is on
        Tape::new(&self.rows, &self.ix, Some(&self.mm)).and_then(|t| t.with_segments(&self.segs)).and_then(|t| t.with_block(self.blk)).and_then(|t| t.with_cap(&self.prev, self.mode)).map(|t| t.with_carry(crate::sw::on(crate::sw::CARRY)).with_resend(if crate::sw::on(crate::sw::RESEND) && !cfg!(feature = "m_v33f_target_exit_no_resend") { Some((PROT_BP, TICK)) } else { None })).map_err(|e| e.to_string())
    }

    /// This tape's reading.
    #[must_use]
    pub const fn mode(&self) -> CapMode {
        self.mode
    }

    /// v2.1: each row's previous print (empty under the v2 row reading).
    #[must_use]
    pub fn prev(&self) -> &[i64] {
        &self.prev
    }

    /// v2.1: the rows (crate view).
    #[must_use]
    pub fn rows(&self) -> &[SecIn] {
        &self.rows
    }

    /// v2.1: each row's previous print under any reading (computed for the v2 row reading, which keeps none).
    #[must_use]
    pub fn prev_any(&self) -> Vec<i64> {
        if self.prev.is_empty() { prev_prints(&self.rows, self.segs.first().map_or(0, |s| s.open) - US_S) } else { self.prev.clone() }
    }
}

/// v2.1 (checker F-A, FIX 3): the sparse-tape facts of one booked trade's FILLED blocks under its reading: (filled blocks, of them those whose
/// first print followed a silence longer than the block, units filled in all, units filled in those, recorded volume of the filled blocks,
/// usable volume of the filled blocks (floored), whether the FIRST sale block's first print followed a print before T by more than one block).
#[must_use]
pub fn gap_facts(dt: &DayTape, ev: &Evid, t_sod: i64, blk: i64) -> (u32, u32, u64, u64, u64, u64, bool) {
    let (mut nb, mut ng, mut u_all, mut u_gap, mut rec, mut usable) = (0_u32, 0_u32, 0_u64, 0_u64, 0_u64, 0_u64);
    let mut first_before_t = false;
    let rows = dt.rows();
    // the previous VALID print of row i (computed here for every reading: the v2 row reading keeps none)
    let prevs = dt.prev_any();
    let mut first_sale = true;
    for (k, list) in [&ev.sale, &ev.bb, &ev.sq].iter().enumerate() {
        for b in list.iter().filter(|b| b.why == Why::Filled) {
            nb += 1;
            u_all += b.units;
            rec += b.block.vol;
            usable += b.block.usable_floor();
            let i0 = rows.partition_point(|r| r.ts < b.block.at);
            let first = rows.get(i0..).unwrap_or(&[]).iter().enumerate().find(|(_, r)| r.ts < b.block.to && r.vol > 0).map(|(j, r)| (r.ts, prevs.get(i0 + j).copied().unwrap_or(r.ts - US_S)));
            if let Some((ts, p)) = first
                && ts - p > blk * US_S
            {
                ng += 1;
                u_gap += b.units;
                if k == 0 && first_sale && sod_of(p) < t_sod {
                    first_before_t = true;
                }
            }
            if k == 0 {
                first_sale = false;
            }
        }
    }
    (nb, ng, u_all, u_gap, rec, usable, first_before_t)
}

/// The share dial.
///
/// # Errors
/// Never for the constant levels.
pub fn dial() -> Result<Dial, String> {
    Dial::new(&SHARE_BP).map_err(|e| e.to_string())
}

/// One sweep outcome.
#[derive(Clone, Copy, Debug, Default)]
pub struct Out {
    /// Net after charges (paisa).
    pub net: i32,
    /// The instant the last fill is known (seconds from 09:00:00).
    pub exit: u16,
    /// bit0 booked (a unit sold), bit1 own exit filled, bit2 square-off used, bit3 unclosed units, bit4 sale partly filled, bit5 a fill
    /// known after 15:30:00.
    pub flag: u8,
    /// v3.1: bit0 a REAL fill known after 15:30 (S-1's split; never-closed units excluded), bit1 the never-closed valuation window held no
    /// print (valued over the entry window instead, counted).
    pub flag2: u8,
    /// v3.1: units the fill rule never closed (valued, not filled; item 3).
    pub nc_units: u32,
    /// v3.1: units sold (the MTM check reads it).
    pub units: u32,
    /// v3.1: the net at the headline gate's stress (+2% adverse on both sides; 0 when the gate is off).
    pub net_s2: i32,
    /// v3.1: the value sold (paisa; the MTM check reads it).
    pub sold: i64,
}

/// Why an event cannot be traded at all (same for every dial).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Refuse {
    #[default]
    None,
    NoUnderlying,
    NoMarginRow,
    StockExpiryDay,
    /// v2 rule R3: a stock option on its own expiry day whose underlying had no next-expiry contract staged that day.
    StockExpiryNoNext,
    AfterCut,
    BadTape,
    NoReference,
    /// v3.1: no NSE margin table for the session nor an earlier one (exact margin).
    NoMarginFile,
    /// v3.1: the contract is not in the session's NSE SPAN table (a new listing): refused and counted.
    NoSpanRow,
    /// v3.1: a stock without its AEL exposure rate that day (no AEL file, or no row): refused and counted.
    NoElmRate,
    /// v3.2 (checklist item 10): a stock option of an underlying in NSE's F&O ban list that session: no fresh position (refused, counted).
    FnoBan,
    /// REPAIR A (check-engine-1 M1): the dated charge rates of the session are refused by the costs tables (refused, counted; never the
    /// 2026-10-30 set instead).
    ChargesRate,
    /// fix:v33 LOT (standing F1): the contract's cube lot is below the day's traded quantum (its (underlying, lot) group trades only in whole
    /// multiples k >= 2 of it: a corporate-action ex-date the cube missed): refused and counted, never sized with a lot no order could carry.
    BadLot,
}

impl Refuse {
    /// The refusal code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::None => "-",
            Self::NoUnderlying => "no_underlying_price",
            Self::NoMarginRow => "no_margin_row",
            Self::StockExpiryDay => "stock_expiry_day",
            Self::StockExpiryNoNext => "stock_expiry_no_next",
            Self::AfterCut => "after_cut_1508",
            Self::BadTape => "bad_tape",
            Self::NoReference => "no_reference",
            Self::NoMarginFile => "no_margin_file",
            Self::NoSpanRow => "no_span_row",
            Self::NoElmRate => "no_elm_rate",
            Self::FnoBan => "fno_ban",
            Self::ChargesRate => "charges_rate_refused",
            Self::BadLot => "lot_below_traded_quantum",
        }
    }
}

/// The R1 key of a contract on its day: (underlying index of the day, side, expiry as days after the trade day). Positions never cross a
/// day (intraday), so a per-day key is exact.
#[must_use]
pub fn side_key(c: &Cube, ci: usize) -> u32 {
    c.cons.get(ci).map_or(u32::MAX, |k| {
        let off = (crate::cal::day_of_ymd(k.expiry) - c.day).clamp(0, 0x7FFF);
        (u32::from(k.und) << 16) | (u32::from(k.right & 1) << 15) | u32::try_from(off).unwrap_or(0x7FFF)
    })
}

/// One event's sweep table.
#[derive(Clone, Debug, Default)]
pub struct EvT {
    pub refuse: Refuse,
    /// The underlying price used by the margin estimate (paisa).
    pub s: i64,
    pub margin_unit: i64,
    pub margin_lot: i64,
    /// Lots per slot count (0 = margin over the slot).
    pub lots: [u16; 4],
    /// The C11 closest count per slot count (printed beside).
    pub closest: [u16; 4],
    /// Index of the slot count's lot count in `qs` (255: none).
    pub q_of: [u8; 4],
    pub qs: Vec<u16>,
    /// The sale's protection limit (paisa).
    pub limit_in: i64,
    /// Per share level: the sale filled at least one lot before 15:10:00.
    pub filled: [bool; NL],
    /// Per share level: the first filling block's price.
    pub p0: [i64; NL],
    /// Per share level: bit k = the entry block's own range alone crossed stop k's level (S16 `inblock_stop`).
    pub inblock: [u8; NL],
    /// Offset of [l][q][cand] in the flat outcome store.
    pub off: u32,
    /// Offset of [l][q] in the flat anchor store.
    pub aoff: u32,
    /// v3.1: the v2.1 proxy margin per lot (printed beside the exact margin).
    pub margin_lot_proxy: i64,
    /// v3.1: the exact margin of the decision (None under the proxy).
    pub mx: Option<crate::mtable::MarginAt>,
    /// v3.1 margin rule v2 (addendum 9): the order's flat add-on (Rs 20), blocked with the lots' margin.
    pub addon: i64,
    /// v3.2 addendum 14 (b)+(c): bit `ni` = at slot count `ni` the last-60-second volume at the headline share covers the WHOLE sale within
    /// F seconds AND the whole buy-back within Fx seconds (`--liq-cover-s`, `--liq-exit-s`); set only under the LIQ14 switch.
    pub cover: u8,
    /// v3.2: the volume traded in the 60 s strictly before T (the coverage test's reading; 0 when not read).
    pub vol60: u64,
    /// REPAIR A (check-engine-2 S3): the segment of the event's contract (0 index, 1 stock): the per-segment slot-count grid.
    pub seg: u8,
}

impl EvT {
    /// v3.2 addendum 14: whether slot count `ni`'s order passes the coverage gate (always true with LIQ14 off).
    #[must_use]
    pub fn cover_ok(&self, ni: usize) -> bool {
        // MUTANT m_v32_liq_cover_off: the coverage gate never refuses
        !crate::sw::on(crate::sw::LIQ14) || cfg!(feature = "m_v32_liq_cover_off") || (self.cover >> ni) & 1 == 1
    }
}

/// v3.2 addendum 14 (b)+(c): the coverage seconds F (entry) and Fx (exit) of the run (`--liq-cover-s` / `--liq-exit-s`, default 60 s each)
/// and the headline share (bp) the reading is taken at (25%).
pub static LIQ_COVER_S: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(60);
pub static LIQ_EXIT_S: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(60);
/// The headline share of addendum 14 (b): 25%.
pub const COVER_SHARE_BP: u64 = 2_500;

/// v3.2 addendum 14 (b)+(c): the coverage bits of an event of lot `lot` whose last-60-second volume is `vol60`: slot count `ni` passes when
/// share x vol60 x F / 60 >= its units (the whole sale within F s) and share x vol60 x Fx / 60 >= its units (the whole buy-back within Fx s).
/// Integer: 25% x vol60 x F >= units x 60 x 100% (in bp). A contract that fails is refused, never half-entered.
#[must_use]
pub fn cover_bits(lots: &[u16; 4], lot: u64, vol60: u64) -> u8 {
    let f = u64::try_from(LIQ_COVER_S.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or(0);
    let fx = u64::try_from(LIQ_EXIT_S.load(std::sync::atomic::Ordering::Relaxed)).unwrap_or(0);
    let cap = |secs: u64| u128::from(vol60) * u128::from(COVER_SHARE_BP) * u128::from(secs);
    let mut b = 0_u8;
    for (ni, &l) in lots.iter().enumerate() {
        let units = u128::from(l) * u128::from(lot);
        let need = units * 60 * 10_000;
        if l > 0 && cap(f) >= need && cap(fx) >= need {
            b |= 1 << ni;
        }
    }
    b
}

impl EvT {
    /// The outcome index of (level, q, cand).
    #[must_use]
    pub fn idx(&self, l: usize, q: usize, cand: usize) -> usize {
        self.off as usize + (l * self.qs.len() + q) * NCAND + cand
    }
    /// The anchor-table index of (level, q).
    #[must_use]
    pub fn aidx(&self, l: usize, q: usize) -> usize {
        self.aoff as usize + l * self.qs.len() + q
    }
}

/// The u16 offset of the 15:08:00 cut.
#[must_use]
pub fn cut_off() -> u16 {
    u16::try_from(CUT_S - BASE_S).unwrap_or(0)
}

/// The exit candidate of (stop k, target j (4 = none), time m (4 = to 15:10)) from the 13 rule anchors: the earliest anchor; none, or
/// one at or after 15:08:00 (S14), is the square-off (13).
#[inline]
#[must_use]
pub fn cand_of(anc: &[u16; NRULE], k: usize, j: usize, m: usize) -> usize {
    let a_s = anc.get(k).copied().unwrap_or(NEVER);
    let a_t = if j < 4 { anc.get(5 + j).copied().unwrap_or(NEVER) } else { NEVER };
    let a_m = if m < 4 { anc.get(9 + m).copied().unwrap_or(NEVER) } else { NEVER };
    let x = a_s.min(a_t).min(a_m);
    if x == NEVER || x >= cut_off() {
        return 13;
    }
    if a_s == x {
        k
    } else if a_t == x {
        5 + j
    } else {
        9 + m
    }
}

/// The exit anchor (seconds of the day) of a candidate (None: the square-off).
#[must_use]
pub fn cand_anchor(anc: &[u16; NRULE], cand: usize) -> Option<i64> {
    let a = if cand < NRULE { anc.get(cand).copied().unwrap_or(NEVER) } else { NEVER };
    if a == NEVER || a >= cut_off() { None } else { Some(i64::from(a) + BASE_S) }
}

/// The kind label of a candidate.
#[must_use]
pub const fn cand_kind(c: usize) -> Kind {
    match c {
        0..=4 => Kind::Stop,
        5..=8 => Kind::Target,
        9..=12 => Kind::TimeExit,
        _ => Kind::SquareOff,
    }
}

/// The 13 exit rules of a level from its first fill price p0 (paisa distances, fixed for the trade).
#[must_use]
pub fn rules_of(p0: i64) -> [Rule; NRULE] {
    let s = |p: i64| Rule::Stop { x: (p0 * p + 99) / 100 };
    let t = |g: i64| Rule::Target { x: p0 * g / 100 };
    let m = |mm: i64| Rule::TimeExit { secs: u32::try_from(mm * 60).unwrap_or(0) };
    [s(STOP_PCT[0]), s(STOP_PCT[1]), s(STOP_PCT[2]), s(STOP_PCT[3]), s(STOP_PCT[4]), t(TGT_PCT[0]), t(TGT_PCT[1]), t(TGT_PCT[2]), t(TGT_PCT[3]), m(TIME_MIN[0]), m(TIME_MIN[1]), m(TIME_MIN[2]), m(TIME_MIN[3])]
}

/// The full facts of one round trip (the sweep keeps net and exit; the trade lists print everything).
#[derive(Clone, Debug, Default)]
pub struct Trip {
    pub qty: u64,
    pub units: u64,
    pub sold_value: i64,
    pub e0: i64,
    pub e1: i64,
    pub p0: i64,
    pub limit_in: i64,
    pub sale_known: i64,
    pub sale_blocks: u32,
    pub sale_secs: u32,
    pub sale_one_price_secs: u32,
    pub sale_past_limit: u32,
    pub x: Option<i64>,
    pub limit_out: i64,
    pub bb_units: u64,
    pub bb_value: i64,
    pub bb_first: i64,
    pub bb_first_px: i64,
    pub bb_blocks: u32,
    pub bb_past_limit: u32,
    pub sq_units: u64,
    pub sq_value: i64,
    pub unclosed: u64,
    pub worst: i64,
    pub buy_value: i64,
    pub charges: i64,
    pub charges_buy: i64,
    pub charges_sell: i64,
    pub today_dearer: bool,
    pub net: i64,
    pub known: i64,
    pub after_1530: bool,
    /// v3.1 (S-1's split): a REAL fill (sale, buy-back or square-off) known after 15:30.
    pub fill_after_1530_real: bool,
    /// v3.1: the never-closed valuation window held no print: valued over the entry window instead (counted).
    pub val_fallback: bool,
    /// v3.1: the never-closed units' valuation (unclosed x worst).
    pub unv: i64,
    /// v3.2 addendum 10: the EXECUTED ORDERS (every filling block x every freeze slice): the sale's, the own buy-backs', the own 15:10
    /// square-off's and Dhan's square-off of the never-closed units (ceil(units / slice)); the slice (units one order may carry, 0 = no freeze
    /// limit known). Filled under the ORDERS_A10 switch only (0 otherwise).
    pub sale_orders: u32,
    pub bb_orders: u32,
    pub sq_orders: u32,
    pub nc_orders: u32,
    pub slice: u64,
    /// v3.2 addendum 14 (d): the instant the own buy-back's last fill is known (0: none) and the square-off's (0: none).
    pub bb_known: i64,
    pub sq_known: i64,
}

impl Trip {
    /// v3.2 addendum 10: the charge orders (sale side, own buy side, Dhan's square-off) under the ORDERS_A10 switch: every filling block x
    /// every freeze slice; the own 15:10 square-off's blocks are own orders under the own-square-off switch, else Dhan's (v2.1's reading).
    /// None with the switch off (v3.1's count).
    #[must_use]
    pub fn orders_a10(&self) -> Option<(u32, u32, u32)> {
        if !crate::sw::on(crate::sw::ORDERS_A10) {
            return None;
        }
        // MUTANT m_v32_orders_one_per_side: one order a side whatever the blocks and slices (v3.1's count under the new switch)
        if cfg!(feature = "m_v32_orders_one_per_side") {
            return Some((u32::from(self.units > 0), u32::from(self.bb_units > 0) + u32::from(self.sq_units > 0), u32::from(self.unclosed > 0)));
        }
        let own_sq = crate::sw::on(crate::sw::OWN_SQUAREOFF);
        let own = self.bb_orders + if own_sq { self.sq_orders } else { 0 };
        let broker = self.nc_orders + if own_sq { 0 } else { self.sq_orders };
        Some((self.sale_orders, own, broker))
    }
}

/// v3.2 addendum 10: Dhan's charges of a sold-first round trip with every executed order counted (sale side `sale_orders`, own buy side
/// `own_orders`, Dhan's square-off `broker_orders`: Rs 20 + the square-off fee each, all with GST), at the dearer of the dated and the
/// 2026-10-30 sets (as v3.1).
///
/// # Errors
/// A rate refusal of both sets.
pub fn charges_orders_n(day: i64, units: u64, buy_value: i64, sell_value: i64, sale_orders: u32, own_orders: u32, broker_orders: u32) -> Result<(i64, i64, i64, bool), String> {
    let trip = crate::charges::Trip { qty: i64::try_from(units).unwrap_or(0), buy_value, sell_value, entry_orders: sale_orders, exit_orders: own_orders, broker_orders, ..crate::charges::Trip::default() };
    charged_trip(day, &trip)
}

/// REPAIR A (check-engine-1 M1): Dhan's charges of a sold-first trip at the DATED rate set only under the CHG_DATED switch (a refused dated
/// set refuses the trip: the sizing already refused such a session's events, `charges_rate_refused`); v3.2 h6's dearer of the dated and the
/// 2026-10-30 sets otherwise. Returns (total, buy side, sell side, today's set was the dearer).
///
/// # Errors
/// A rate refusal.
pub fn charged_trip(day: i64, trip: &crate::charges::Trip) -> Result<(i64, i64, i64, bool), String> {
    // MUTANT m_v32r_charges_dearer: the dearer-of reading kept under the dated switch
    if crate::sw::on(crate::sw::CHG_DATED) && !cfg!(feature = "m_v32r_charges_dearer") {
        let rd = crate::charges::rates_on(day).map_err(|e| format!("charges_rate_refused on day {day}: {e}"))?;
        let (b, s) = crate::charges::charges_est_sell(&rd, trip);
        return Ok((b + s, b, s, false));
    }
    let rt = crate::charges::rates_on(crate::charges::today())?;
    let rd = crate::charges::rates_on(day).unwrap_or(rt);
    Ok(crate::charges::charged_sell(&rd, &rt, trip))
}

/// REPAIR A: the dearer-of (dated vs 2026-10-30) reading of a trip's charges, printed BESIDE the dated net only.
#[must_use]
pub fn charges_dearer_of(day: i64, units: u64, buy_value: i64, sell_value: i64, orders: (u32, u32, u32)) -> i64 {
    let trip = crate::charges::Trip { qty: i64::try_from(units).unwrap_or(0), buy_value, sell_value, entry_orders: orders.0, exit_orders: orders.1, broker_orders: orders.2, ..crate::charges::Trip::default() };
    let Ok(rt) = crate::charges::rates_on(crate::charges::today()) else { return 0 };
    let rd = crate::charges::rates_on(day).unwrap_or(rt);
    crate::charges::charged_sell(&rd, &rt, &trip).0
}

/// The charges of a finished trip: v3.1's count (1 sale order; an own buy-back; the square-off by the own-square-off switch; Dhan's fee for
/// never-closed units) or, under the ORDERS_A10 switch, every filling block x every freeze slice ([`Trip::orders_a10`]).
///
/// # Errors
/// A rate refusal.
pub fn settle_charges(r: &mut Trip, day: i64) -> Result<(), String> {
    let (c, cb, cs, td) = match r.orders_a10() {
        Some((s, o, b)) => charges_orders_n(day, r.units, r.buy_value, r.sold_value, s, o, b)?,
        None => {
            let (own_n, broker) = buy_orders(r.bb_units, r.sq_units, r.unclosed);
            charges_orders(day, r.units, r.buy_value, r.sold_value, own_n, broker)?
        }
    };
    r.charges = c;
    r.charges_buy = cb;
    r.charges_sell = cs;
    r.today_dearer = td;
    r.net = r.sold_value - r.buy_value - c;
    Ok(())
}

/// v3.1 (addendum 7, checklist item 9): the buy side's order counts: (own exit orders, Dhan's square-off): v2.1 = (an own buy-back, the
/// square-off or never-closed units); the own-square-off switch = (an own buy-back + the own 15:10 square-off, never-closed units only).
#[must_use]
pub fn buy_orders(bb_units: u64, sq_units: u64, unclosed: u64) -> (u32, bool) {
    if crate::sw::on(crate::sw::OWN_SQUAREOFF) {
        (u32::from(bb_units > 0) + u32::from(sq_units > 0), unclosed > 0)
    } else {
        (u32::from(bb_units > 0), sq_units > 0 || unclosed > 0)
    }
}

/// v3.1 item 3: the order end (15:30:00 under the switch, else the tape's session end 15:40, v2.1).
#[must_use]
pub fn order_end(tape: &Tape<'_>, day: i64) -> i64 {
    if crate::sw::on(crate::sw::END_1530) && !cfg!(feature = "m_v31_close_1540") { at_sod(day, CLOSE_S).min(tape.session_end()) } else { tape.session_end() }
}

/// v3.1 item 3: the never-closed units' value per unit: the highest HIGH between the EXIT DECISION (`from`: an own exit's anchor, else the
/// 15:10:00 square-off) and the order end (v2.1: from the entry E0 to the session end). A window without a print falls back to the entry
/// window (counted). Returns (value per unit, the window end (the instant the position is known), fell back).
#[must_use]
pub fn never_closed_value(tape: &Tape<'_>, day: i64, e0: i64, exit_decision: i64) -> (i64, i64, bool) {
    let end = order_end(tape, day);
    let from = if crate::sw::on(crate::sw::VALUE_FROM_EXIT) && !cfg!(feature = "m_v31_value_from_entry") { exit_decision } else { e0 };
    // matrix X65: the range table answers each range max in O(1) (the same answer as v2.1's scan)
    match tape.extreme(from, end, true) {
        Some((v, _)) => (v, end, false),
        None => {
            // matrix X02: an empty window is NEVER valued at 0: the higher of the last print at or before the decision and the entry window
            let e0w = tape.extreme(e0, end, true).map_or(0, |x| x.0);
            let last = common_fill::limits::size_ref(tape, from.saturating_add(common_fill::US_S), Side::Buy).unwrap_or(0);
            (e0w.max(if from > e0 { last } else { 0 }), end, true)
        }
    }
}

/// The recorded paths of one event: the sale (every share level at once, up to `upto` units) with its filling blocks, the square-off,
/// and the own buy-backs by anchor second.
pub struct Paths {
    pub sale: FillPath,
    pub fills: [Vec<BlockFill>; NL],
    pub sq: FillPath,
    pub buys: Vec<(i64, FillPath, i64)>,
    pub limit_in: i64,
    pub upto: u64,
    /// v3.2 addendum 10: the units one order may carry under NSE's dated freeze limit (0 = none known: one order a block); set by the caller
    /// after `record_base` (the contract-day's [`crate::freeze::slice_units`]).
    pub slice: u64,
}

/// Why the base paths could not be recorded.
pub enum BaseErr {
    /// No traded second since the segment opened (S11).
    NoReference,
    /// A crate refusal.
    Crate(String),
}

/// Records the sale path (T to 15:10:00, Dhan's protection below the last traded second's LOW) with its filling blocks, and the square-off
/// path (15:10:00 to the session end), up to `upto` units.
///
/// # Errors
/// `NoReference` when nothing traded before T; a crate refusal.
pub fn record_base(tape: &Tape<'_>, day: i64, t: i64, lot: u64, upto: u64, dial: &Dial) -> Result<Paths, BaseErr> {
    let tt = at_sod(day, t);
    let Some(refpx) = size_ref(tape, tt, Side::Sell) else { return Err(BaseErr::NoReference) };
    let lim = protection(refpx, Side::Sell, PROT_BP, TICK);
    let o = Order { side: Side::Sell, anchor: tt, stop: at_sod(day, SQ_S), unit: lot, limit: Some(lim), kind: Kind::Entry };
    let up = [upto; MAXL];
    let mask = LevelMask::all(NL);
    let sale = FillPath::record(tape, &o, dial, mask, &up, None).map_err(|e| BaseErr::Crate(e.to_string()))?;
    let mut fills: [Vec<BlockFill>; NL] = Default::default();
    {
        let mut want = up;
        let mut out = [LevelFill::default(); MAXL];
        let mut rec = |l: usize, b: &BlockFill| {
            if b.why == Why::Filled
                && let Some(v) = fills.get_mut(l)
            {
                v.push(*b);
            }
        };
        common_fill::walk(tape, &o, dial, mask, &mut want, None, None, &mut out, Some(&mut rec)).map_err(|e| BaseErr::Crate(e.to_string()))?;
    }
    // v3.1 item 3: orders end at 15:30 under the switch (no fill after 15:30)
    let so = Order { side: Side::Buy, anchor: at_sod(day, SQ_S), stop: order_end(tape, day).max(at_sod(day, SQ_S)), unit: lot, limit: None, kind: Kind::SquareOff };
    let so = if crate::sw::on(crate::sw::END_1530) { so } else { Order { stop: at_sod(day, END_S), ..so } };
    let sq = FillPath::record(tape, &so, dial, mask, &up, None).map_err(|e| BaseErr::Crate(e.to_string()))?;
    Ok(Paths { sale, fills, sq, buys: Vec::new(), limit_in: lim, upto, slice: 0 })
}

/// The own buy-back path anchored at second `xs` (cached in `p`): Dhan's protection above the last traded second's HIGH before it.
///
/// # Errors
/// A crate refusal.
fn buy_path(p: &mut Paths, tape: &Tape<'_>, day: i64, xs: i64, lot: u64, dial: &Dial) -> Result<usize, String> {
    if let Some(i) = p.buys.iter().position(|b| b.0 == xs) {
        return Ok(i);
    }
    let at = at_sod(day, xs);
    let lim = size_ref(tape, at, Side::Buy).map(|r| protection(r, Side::Buy, PROT_BP, TICK));
    let o = Order { side: Side::Buy, anchor: at, stop: at_sod(day, SQ_S), unit: lot, limit: lim, kind: Kind::SignalExit };
    let fp = FillPath::record(tape, &o, dial, LevelMask::all(NL), &[p.upto; MAXL], None).map_err(|e| e.to_string())?;
    p.buys.push((xs, fp, lim.unwrap_or(0)));
    Ok(p.buys.len() - 1)
}

/// The first `qty` units of a level's filling blocks (the qty-unit entry's blocks, the last one cut).
#[must_use]
pub fn truncate(fills: &[BlockFill], qty: u64) -> Vec<BlockFill> {
    let mut out = Vec::with_capacity(fills.len());
    let mut cum = 0_u64;
    for b in fills {
        if cum >= qty {
            break;
        }
        let take = b.units.min(qty - cum);
        cum += take;
        out.push(BlockFill { units: take, cum, ..*b });
    }
    out
}

/// The 13 rule anchors (u16 offsets from 09:00:00, NEVER = no trigger before 15:08:00) of one entry's filling blocks (S16 via
/// `path::scan_one`, the window [E0, 15:08:00)).
#[must_use]
pub fn anchors(tape: &Tape<'_>, day: i64, fills: &[BlockFill], rules: &[Rule; NRULE]) -> [u16; NRULE] {
    let mut a = [NEVER; NRULE];
    let Some(b0) = fills.first() else { return a };
    let ev = EntryView { e0: b0.block.at, e1: b0.block.known, fills };
    let mut st = 0_u32;
    let (trig, _) = scan_one(tape, &ev, at_sod(day, CUT_S), false, rules, &[], &mut st);
    for (i, t) in trig.iter().enumerate() {
        if let (Some(t), Some(slot)) = (t, a.get_mut(i)) {
            *slot = u16::try_from(sod_of(t.anchor) - BASE_S).unwrap_or(NEVER);
        }
    }
    a
}

/// Dhan's charges of a sold-first round trip at the dearer of the day's set and the 2026-10-30 set (sell-diag `charged_sell`).
///
/// # Errors
/// A rate refusal of both sets.
pub fn charges_of(day: i64, units: u64, buy_value: i64, sell_value: i64, own_exit: bool, broker: bool) -> Result<(i64, i64, i64, bool), String> {
    charges_orders(day, units, buy_value, sell_value, u32::from(own_exit), broker)
}

/// v3.1 (addendum 7): the charges with the own exit orders counted (an own buy-back and, under the own-square-off switch, the 15:10
/// square-off) and Dhan's square-off fee only when `broker`.
///
/// # Errors
/// A rate refusal of both sets.
pub fn charges_orders(day: i64, units: u64, buy_value: i64, sell_value: i64, own_orders: u32, broker: bool) -> Result<(i64, i64, i64, bool), String> {
    let trip = crate::charges::Trip { qty: i64::try_from(units).unwrap_or(0), buy_value, sell_value, entry_orders: 1, exit_orders: own_orders, broker_orders: u32::from(broker), ..crate::charges::Trip::default() };
    charged_trip(day, &trip)
}

/// One round trip at share level `l` for `qty` units with the exit anchored at `x` (None = no own exit: the square-off).
///
/// # Errors
/// A crate refusal.
pub fn trip(p: &mut Paths, tape: &Tape<'_>, day: i64, lot: u64, l: usize, qty: u64, x: Option<i64>, dial: &Dial) -> Result<Trip, String> {
    let mut st = 0_u32;
    let sale = match x {
        Some(xs) => p.sale.cut_at(tape, l, qty, at_sod(day, xs), &mut st),
        None => p.sale.view(l, qty, &mut st),
    }
    .map_err(|e| e.to_string())?;
    let mut r = Trip {
        qty,
        units: sale.units,
        sold_value: sale.value,
        e0: sale.e0,
        e1: sale.e1,
        p0: sale.first.map_or(0, |b| b.px),
        limit_in: p.limit_in,
        sale_known: sale.known_at,
        sale_blocks: sale.blocks_filled,
        sale_secs: sale.filled_secs,
        sale_one_price_secs: sale.one_price_secs,
        sale_past_limit: sale.past_limit,
        x,
        ..Trip::default()
    };
    if sale.units == 0 {
        return Ok(r);
    }
    // v3.2 addendum 10: every filling block x every freeze slice is an executed order (counted under the switch only)
    let a10 = crate::sw::on(crate::sw::ORDERS_A10);
    if a10 {
        r.slice = p.slice;
        r.sale_orders = p.sale.sliced_orders(l, &sale, p.slice).map_err(|e| e.to_string())?;
    }
    let mut known = sale.known_at;
    if let Some(xs) = x {
        let i = buy_path(p, tape, day, xs, lot, dial)?;
        let (_, fp, lim) = p.buys.get(i).ok_or("buy path")?;
        let bb = fp.view(l, sale.units, &mut st).map_err(|e| e.to_string())?;
        r.limit_out = *lim;
        r.bb_units = bb.units;
        r.bb_value = bb.value;
        r.bb_first = bb.e0;
        r.bb_first_px = bb.first.map_or(0, |b| b.px);
        r.bb_blocks = bb.blocks_filled;
        r.bb_past_limit = bb.past_limit;
        if a10 {
            r.bb_orders = fp.sliced_orders(l, &bb, p.slice).map_err(|e| e.to_string())?;
        }
        if bb.units > 0 {
            known = known.max(bb.known_at);
            r.bb_known = bb.known_at;
        }
    }
    let rest = sale.units - r.bb_units;
    let sq = p.sq.view(l, rest, &mut st).map_err(|e| e.to_string())?;
    r.sq_units = sq.units;
    r.sq_value = sq.value;
    if a10 {
        r.sq_orders = p.sq.sliced_orders(l, &sq, p.slice).map_err(|e| e.to_string())?;
    }
    if sq.units > 0 {
        known = known.max(sq.known_at);
        r.sq_known = sq.known_at;
    }
    r.unclosed = rest - sq.units;
    r.fill_after_1530_real = known > at_sod(day, CLOSE_S);
    if r.unclosed > 0 {
        let (w, end, fb) = never_closed_value(tape, day, sale.e0, at_sod(day, x.unwrap_or(SQ_S)));
        r.worst = w;
        r.val_fallback = fb;
        known = known.max(end);
        if a10 {
            r.nc_orders = u32::try_from(common_fill::legs::freeze_slices(r.unclosed, p.slice)).unwrap_or(u32::MAX);
        }
    }
    r.after_1530 = known > at_sod(day, CLOSE_S);
    let unv = i64::try_from(i128::from(r.unclosed) * i128::from(r.worst)).map_err(|_| "unclosed value")?;
    r.unv = unv;
    r.buy_value = r.bb_value + r.sq_value + unv;
    settle_charges(&mut r, day)?;
    r.known = known;
    Ok(r)
}

/// The sweep record of a trip.
///
/// # Errors
/// A net beyond i32 paisa.
pub fn out_of(r: &Trip) -> Result<Out, String> {
    let mut fl = 0_u8;
    if r.units > 0 {
        fl |= 1;
    }
    if r.bb_units > 0 {
        fl |= 2;
    }
    if r.sq_units > 0 {
        fl |= 4;
    }
    if r.unclosed > 0 {
        fl |= 8;
    }
    if r.units > 0 && r.units < r.qty {
        fl |= 16;
    }
    if r.after_1530 {
        fl |= 32;
    }
    let net = i32::try_from(r.net).map_err(|_| format!("net {} over i32 paisa", r.net))?;
    let exit = if r.units > 0 { u16::try_from(sod_of(r.known) - BASE_S).unwrap_or(NEVER) } else { 0 };
    let flag2 = u8::from(r.fill_after_1530_real) | (u8::from(r.val_fallback) << 1);
    Ok(Out { net, exit, flag: fl, flag2, nc_units: u32::try_from(r.unclosed).unwrap_or(u32::MAX), units: u32::try_from(r.units).unwrap_or(u32::MAX), net_s2: 0, sold: r.sold_value })
}

/// v3.1: the sweep record with the headline gate's stress net (decision (c): +2% adverse on both sides, charges recomputed) when the
/// headline switch is on.
///
/// # Errors
/// A net beyond i32 paisa or a charge-rate refusal.
pub fn out_of31(r: &Trip, day: i64) -> Result<Out, String> {
    let mut o = out_of(r)?;
    if crate::sw::on(crate::sw::HEADLINE) && r.units > 0 {
        let (own_orders, broker) = buy_orders(r.bb_units, r.sq_units, r.unclosed);
        let m = crate::stress::Money { day, units: r.units, sold: r.sold_value, bought: r.buy_value, own_orders, broker, orders: r.orders_a10() };
        let s = crate::stress::stressed(&m, crate::sw::GATE_BP.load(std::sync::atomic::Ordering::Relaxed), true)?;
        o.net_s2 = i32::try_from(s).map_err(|_| format!("stress net {s} over i32 paisa"))?;
    }
    Ok(o)
}

/// The margin estimate of a contract at decision second t: (S, per unit, refusal). S = the underlying's last 1-minute close at or
/// before T (forward filled).
#[must_use]
pub fn margin(c: &Cube, ci: usize, t: u32, var: &std::collections::BTreeMap<String, VarRow>) -> (i64, i64, Refuse) {
    let Some(k) = c.cons.get(ci) else { return (0, 0, Refuse::BadTape) };
    let b = usize::try_from((i64::from(t) - OPEN_S) / 60).unwrap_or(0);
    let s = i64::from(c.spot_at(usize::from(k.und), b.saturating_sub(1)));
    if s <= 0 {
        return (0, 0, Refuse::NoUnderlying);
    }
    let call = k.right == 0;
    if k.kind == 0 {
        (s, per_unit_index(s, k.strike, call, k.expiry == c.ymd).total, Refuse::None)
    } else {
        let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
        match var.get(name) {
            Some(v) => (s, per_unit_stock(s, k.strike, call, v).total, Refuse::None),
            None => (s, 0, Refuse::NoMarginRow),
        }
    }
}

/// The sizing part of an event's table (refusals, margin, lots per slot count). `v2`: a stock option on its own expiry day can only be
/// listed when its underlying had no next expiry staged that day (R3), so its refusal is `stock_expiry_no_next`; v1: `stock_expiry_day`.
#[must_use]
pub fn size_event(c: &Cube, ci: usize, t: u32, var: &std::collections::BTreeMap<String, VarRow>, v2: bool) -> EvT {
    size_event_mx(c, ci, t, var, v2, None)
}

/// v3.1 item 1: the sizing with the EXACT margin when `mx` is given (the per-contract-day table, THE margin module of crate v2.3): the lots
/// per slot from the exact margin per lot; the v2.1 proxy printed beside (`margin_lot_proxy`); a contract without a table row, or a stock
/// without its exposure rate, refused and counted (no guess).
#[must_use]
pub fn size_event_mx(c: &Cube, ci: usize, t: u32, var: &std::collections::BTreeMap<String, VarRow>, v2: bool, mx: Option<&crate::mtable::MarginAt>) -> EvT {
    let mut ev = EvT { q_of: [255; 4], ..EvT::default() };
    let Some(k) = c.cons.get(ci) else {
        ev.refuse = Refuse::BadTape;
        return ev;
    };
    ev.seg = k.kind;
    if i64::from(t) >= CUT_S {
        ev.refuse = Refuse::AfterCut;
        return ev;
    }
    if k.kind == 1 && k.expiry == c.ymd {
        ev.refuse = if v2 { Refuse::StockExpiryNoNext } else { Refuse::StockExpiryDay };
        return ev;
    }
    // v3.2 checklist item 10: no fresh stock-option position on an NSE F&O ban day of its underlying (session S reads S's own list)
    if crate::sw::on(crate::sw::BAN) && k.kind == 1 && crate::ban::refuse(c.day, c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str())) {
        ev.refuse = Refuse::FnoBan;
        return ev;
    }
    // REPAIR A (check-engine-1 M1): no trade whose session's DATED charge rates the costs tables refuse (counted, never another set)
    if crate::sw::on(crate::sw::CHG_DATED) && crate::charges::rates_on(c.day).is_err() {
        ev.refuse = Refuse::ChargesRate;
        return ev;
    }
    // fix:v33 LOT (standing F1): no event on a contract whose cube lot is below the day's traded quantum (refused, counted)
    if crate::fixg::lot_refused(c, ci) {
        ev.refuse = Refuse::BadLot;
        return ev;
    }
    let (s, mu, rf) = margin(c, ci, t, var);
    ev.s = s;
    // v3.1 item 1: under the exact margin the proxy is only a reading beside: a stock without its C_VAR1 row is NOT refused for it
    let exact_in = mx.is_some() && !cfg!(feature = "m_v31_margin_proxy");
    if rf != Refuse::None && !(exact_in && rf == Refuse::NoMarginRow) {
        ev.refuse = rf;
        return ev;
    }
    let lot = u64::from(k.lot.max(1));
    ev.margin_unit = mu;
    ev.margin_lot = mu * i64::from(k.lot);
    ev.margin_lot_proxy = ev.margin_lot;
    // MUTANT m_v31_margin_proxy: the exact margin read but the proxy kept for the sizing
    if let Some(m) = mx
        && !cfg!(feature = "m_v31_margin_proxy")
    {
        ev.mx = Some(*m);
        if let Some(r) = m.refuse {
            ev.refuse = r;
            return ev;
        }
        ev.margin_lot = m.lot_exact;
    }
    let mu = if ev.mx.is_some() && !cfg!(feature = "m_v31_margin_proxy") { (ev.margin_lot + i64::from(k.lot.max(1)) - 1) / i64::from(k.lot.max(1)) } else { mu };
    // margin rule v2 (addendum 9): floor((slot - Rs 20) / margin per lot), the Rs 20 blocked with the lots
    let v2rule = ev.mx.is_some_and(|m| m.addon > 0);
    if v2rule {
        ev.addon = common_fill::margin::ORDER_ADDON_PAISA;
    }
    for i in 0..NSLOT.len() {
        // REPAIR A (check-engine-2 S3): the per-segment slot-count grid under the NGRID switch (v3.1's 5 / 10 / 20 / 40 otherwise)
        let n = crate::sw::n_slots(k.kind, i).max(1);
        let slot = ACCOUNT / n;
        let fit = if v2rule { common_fill::margin::lots_fit_v2(slot, ev.margin_lot) } else if ev.margin_lot > 0 { slot / ev.margin_lot } else { 0 };
        if let Some(x) = ev.lots.get_mut(i) {
            *x = u16::try_from(fit).unwrap_or(u16::MAX);
        }
        let cu = size_units_closest(slot, Some(mu), lot, None).map_or(0, |x| x.0);
        if let Some(x) = ev.closest.get_mut(i) {
            *x = u16::try_from(cu / lot).unwrap_or(u16::MAX);
        }
    }
    let mut qs: Vec<u16> = ev.lots.iter().copied().filter(|&x| x > 0).collect();
    qs.sort_unstable();
    qs.dedup();
    for (i, &x) in ev.lots.iter().enumerate() {
        if let Some(q) = qs.iter().position(|&y| y == x)
            && let Some(slot) = ev.q_of.get_mut(i)
        {
            *slot = u8::try_from(q).unwrap_or(255);
        }
    }
    ev.qs = qs;
    ev
}

/// Builds the sweep table of one event (appending its outcomes to `store` and its anchors to `astore`). `tape` = the contract-day's tape.
///
/// # Errors
/// A crate refusal that is not a per-event refusal.
pub fn price_event(c: &Cube, ci: usize, t: u32, tape: Option<&Tape<'_>>, var: &std::collections::BTreeMap<String, VarRow>, dial: &Dial, store: &mut Vec<Out>, astore: &mut Vec<[u16; NRULE]>, v2: bool) -> Result<EvT, String> {
    let mut ev = size_event(c, ci, t, var, v2);
    if ev.refuse != Refuse::None || ev.qs.is_empty() {
        return Ok(ev);
    }
    let Some(k) = c.cons.get(ci) else { return Err("contract".to_owned()) };
    let Some(tape) = tape else {
        ev.refuse = Refuse::BadTape;
        return Ok(ev);
    };
    let lot = u64::from(k.lot);
    let qmax = u64::from(ev.qs.last().copied().unwrap_or(0)) * lot;
    let day = c.day;
    let mut paths = match record_base(tape, day, i64::from(t), lot, qmax, dial) {
        Ok(p) => p,
        Err(BaseErr::NoReference) => {
            ev.refuse = Refuse::NoReference;
            return Ok(ev);
        }
        Err(BaseErr::Crate(e)) => return Err(format!("{} {}: {e}", c.ymd, k.symbol)),
    };
    ev.limit_in = paths.limit_in;
    let nq = ev.qs.len();
    ev.off = u32::try_from(store.len()).map_err(|_| "outcome store over u32")?;
    ev.aoff = u32::try_from(astore.len()).map_err(|_| "anchor store over u32")?;
    store.resize(store.len() + NL * nq * NCAND, Out::default());
    astore.resize(astore.len() + NL * nq, [NEVER; NRULE]);
    let qs = ev.qs.clone();
    for l in 0..NL {
        let fl = paths.fills.get(l).cloned().unwrap_or_default();
        let Some(b0) = fl.first().copied() else { continue };
        if let (Some(f), Some(p)) = (ev.filled.get_mut(l), ev.p0.get_mut(l)) {
            *f = true;
            *p = b0.px;
        }
        let rules = rules_of(b0.px);
        for (kk, r) in rules.iter().take(5).enumerate() {
            if let Rule::Stop { x } = r
                && b0.block.high >= b0.px.saturating_add(*x)
                && let Some(m) = ev.inblock.get_mut(l)
            {
                *m |= 1 << kk;
            }
        }
        for (q, &lots) in qs.iter().enumerate() {
            let qty = u64::from(lots) * lot;
            let fq = truncate(&fl, qty);
            let anc = anchors(tape, day, &fq, &rules);
            let ai = ev.aidx(l, q);
            if let Some(slot) = astore.get_mut(ai) {
                *slot = anc;
            }
            let mut done: Vec<(Option<i64>, Out)> = Vec::with_capacity(NCAND);
            for cand in 0..NCAND {
                let x = cand_anchor(&anc, cand);
                let o = if let Some(&(_, o)) = done.iter().find(|(xx, _)| *xx == x) {
                    o
                } else {
                    let r = trip(&mut paths, tape, day, lot, l, qty, x, dial)?;
                    let o = out_of(&r)?;
                    done.push((x, o));
                    o
                };
                let i = ev.idx(l, q, cand);
                if let Some(slot) = store.get_mut(i) {
                    *slot = o;
                }
            }
        }
    }
    Ok(ev)
}

/// One trade recomputed from scratch (the detail lists, the 1-lot variant and the sweep-vs-detail check): the sale recorded up to `qty`
/// units only, its own triggers, its own exit. Returns (trip, anchors, candidate, in-block stop mask), or None when the sale level never
/// filled (or no reference).
///
/// # Errors
/// A crate refusal.
pub fn trade_once(tape: &Tape<'_>, day: i64, t: u32, lot: u64, qty: u64, l: usize, k: usize, j: usize, m: usize, dial: &Dial) -> Result<Option<(Trip, [u16; NRULE], usize, u8)>, String> {
    let mut paths = match record_base(tape, day, i64::from(t), lot, qty, dial) {
        Ok(p) => p,
        Err(BaseErr::NoReference) => return Ok(None),
        Err(BaseErr::Crate(e)) => return Err(e),
    };
    let fl = paths.fills.get(l).cloned().unwrap_or_default();
    let Some(b0) = fl.first().copied() else { return Ok(None) };
    let rules = rules_of(b0.px);
    let mut inb = 0_u8;
    for (kk, r) in rules.iter().take(5).enumerate() {
        if let Rule::Stop { x } = r
            && b0.block.high >= b0.px.saturating_add(*x)
        {
            inb |= 1 << kk;
        }
    }
    let fq = truncate(&fl, qty);
    let anc = anchors(tape, day, &fq, &rules);
    let cand = cand_of(&anc, k, j, m);
    let x = cand_anchor(&anc, cand);
    let r = trip(&mut paths, tape, day, lot, l, qty, x, dial)?;
    Ok(Some((r, anc, cand, inb)))
}

/// The fill blocks of one trade (the per-trade EVIDENCE): every block the crate's walk read for the trade's share level while it had
/// units to fill (filled, past the protection limit, or a share that floored to 0 whole lots), for the sale (anchored at T, stopped at
/// the exit anchor or 15:10:00), the own buy-back (anchored at the exit, to 15:10:00) and the square-off (15:10:00 to the session end).
pub struct Evid {
    pub sale: Vec<BlockFill>,
    pub bb: Vec<BlockFill>,
    pub sq: Vec<BlockFill>,
    pub lim_in: i64,
    pub lim_out: Option<i64>,
    pub units: u64,
    pub sold: i64,
    pub bb_units: u64,
    pub bb_value: i64,
    pub sq_units: u64,
    pub sq_value: i64,
}

fn walk_rec(tape: &Tape<'_>, o: &Order, dial: &Dial, l: usize, want_units: u64) -> Result<(Vec<BlockFill>, u64, i64), String> {
    let mut want = [0_u64; MAXL];
    if let Some(w) = want.get_mut(l) {
        *w = want_units;
    }
    let mut out = [LevelFill::default(); MAXL];
    let mut v: Vec<BlockFill> = Vec::new();
    {
        let mut rec = |li: usize, b: &BlockFill| {
            if li == l {
                v.push(*b);
            }
        };
        common_fill::walk(tape, o, dial, LevelMask::all(NL), &mut want, None, None, &mut out, Some(&mut rec)).map_err(|e| e.to_string())?;
    }
    let f = out.get(l).copied().unwrap_or_default();
    Ok((v, f.units, f.value))
}

/// The evidence walk of one trade (`x` = its exit anchor, None = the square-off). None when nothing traded before T.
///
/// # Errors
/// A crate refusal.
pub fn evidence(tape: &Tape<'_>, day: i64, t: u32, lot: u64, qty: u64, l: usize, x: Option<i64>, dial: &Dial) -> Result<Option<Evid>, String> {
    let tt = at_sod(day, i64::from(t));
    let Some(refpx) = size_ref(tape, tt, Side::Sell) else { return Ok(None) };
    let lim = protection(refpx, Side::Sell, PROT_BP, TICK);
    let stop = at_sod(day, x.map_or(SQ_S, |xs| xs.min(SQ_S)));
    let o = Order { side: Side::Sell, anchor: tt, stop, unit: lot, limit: Some(lim), kind: Kind::Entry };
    let (sale, units, sold) = walk_rec(tape, &o, dial, l, qty)?;
    let mut ev = Evid { sale, bb: Vec::new(), sq: Vec::new(), lim_in: lim, lim_out: None, units, sold, bb_units: 0, bb_value: 0, sq_units: 0, sq_value: 0 };
    if units == 0 {
        return Ok(Some(ev));
    }
    if let Some(xs) = x {
        let at = at_sod(day, xs);
        let lo = size_ref(tape, at, Side::Buy).map(|r| protection(r, Side::Buy, PROT_BP, TICK));
        let o = Order { side: Side::Buy, anchor: at, stop: at_sod(day, SQ_S), unit: lot, limit: lo, kind: Kind::SignalExit };
        let (bb, u, v) = walk_rec(tape, &o, dial, l, units)?;
        ev.bb = bb;
        ev.bb_units = u;
        ev.bb_value = v;
        ev.lim_out = lo;
    }
    let rest = units - ev.bb_units;
    if rest > 0 {
        let o = Order { side: Side::Buy, anchor: at_sod(day, SQ_S), stop: at_sod(day, END_S), unit: lot, limit: None, kind: Kind::SquareOff };
        let (sq, u, v) = walk_rec(tape, &o, dial, l, rest)?;
        ev.sq = sq;
        ev.sq_units = u;
        ev.sq_value = v;
    }
    Ok(Some(ev))
}

#[cfg(test)]
mod tests {
    //! Extreme cases the staged days do not show (brief: "test each"): a stock option on its own expiry day, an event at the 15:08 cut,
    //! a stop gapped through (the buy-back pays the block's worst), missing seconds inside a fill block (empty blocks jumped).
    use super::*;
    use crate::cal::{NM, days_from_civil, ymd_of};
    use crate::cube::{Contract, Cube, Sec, Und};

    fn day() -> i64 {
        days_from_civil(2026, 11, 4)
    }

    fn cube(kind: u8, expiry: u32) -> Cube {
        let d = day();
        let mut c = Cube { ymd: ymd_of(d), day: d, ..Cube::default() };
        c.unds.push(Und { name: "SBIN".to_owned(), kind, spot_seg: kind, spot_sid: 1, nearest: expiry, next: 0 });
        c.cons.push(Contract { sid: 7, und: 0, kind, right: 0, expiry, lot: 50, strike: 100_000, first_min: 0, first_open: 1_000, symbol: "SBIN-X-CE".to_owned(), n_sec: 0 });
        c.spot = vec![100_000; NM];
        c.close = vec![1_000; NM];
        c.flags = vec![5; NM];
        c.high = vec![1_000; NM];
        c.low = vec![1_000; NM];
        c.vol = vec![500; NM];
        c.oi = vec![0; NM];
        c.cumvol = (0..=NM as u64).map(|m| m * 500).collect();
        c.cumval = (0..=NM as u64).map(|m| m * 500 * 1_000).collect();
        c
    }

    fn var() -> std::collections::BTreeMap<String, VarRow> {
        let mut v = std::collections::BTreeMap::new();
        v.insert("SBIN".to_owned(), VarRow { var_x100: 1_200, elm_x100: 350 });
        v
    }

    #[test]
    fn r1_side_key_is_underlying_side_expiry_not_strike() {
        let d = day();
        let mut c = Cube { ymd: ymd_of(d), day: d, ..Cube::default() };
        c.unds.push(Und { name: "NIFTY".to_owned(), kind: 0, spot_seg: 0, spot_sid: 13, nearest: ymd_of(d + 5), next: 0 });
        c.unds.push(Und { name: "BANKNIFTY".to_owned(), kind: 0, spot_seg: 0, spot_sid: 25, nearest: ymd_of(d + 19), next: 0 });
        let mk = |und: u16, right: u8, exp: i64, strike: i64| Contract { sid: 1, und, kind: 0, right, expiry: ymd_of(d + exp), lot: 65, strike, first_min: 0, first_open: 100, symbol: String::new(), n_sec: 0 };
        c.cons = vec![mk(0, 0, 5, 2_500_000), mk(0, 0, 5, 2_510_000), mk(0, 1, 5, 2_500_000), mk(0, 0, 12, 2_500_000), mk(1, 0, 5, 2_500_000)];
        let k = |i: usize| side_key(&c, i);
        assert_eq!(k(0), k(1), "two CE strikes of one underlying and expiry share the key");
        assert_ne!(k(0), k(2), "CE and PE of one underlying differ");
        assert_ne!(k(0), k(3), "another expiry differs");
        assert_ne!(k(0), k(4), "another underlying differs");
    }

    #[test]
    fn stock_expiry_day_refused() {
        let d = day();
        let t = 36_000_u32;
        assert_eq!(size_event(&cube(1, ymd_of(d)), 0, t, &var(), false).refuse, Refuse::StockExpiryDay);
        assert_eq!(size_event(&cube(1, ymd_of(d)), 0, t, &var(), true).refuse, Refuse::StockExpiryNoNext);
        // an INDEX option on its own expiry day is allowed (sized by the ASSUMED index formula)
        assert_eq!(size_event(&cube(0, ymd_of(d)), 0, t, &var(), true).refuse, Refuse::None);
        // a stock option before its expiry day is allowed
        assert_eq!(size_event(&cube(1, ymd_of(d + 7)), 0, t, &var(), true).refuse, Refuse::None);
    }

    #[test]
    fn after_cut_refused() {
        let d = day();
        assert_eq!(size_event(&cube(1, ymd_of(d + 7)), 0, u32::try_from(CUT_S).unwrap_or(0), &var(), true).refuse, Refuse::AfterCut);
        assert_eq!(size_event(&cube(1, ymd_of(d + 7)), 0, u32::try_from(CUT_S - 60).unwrap_or(0), &var(), true).refuse, Refuse::None);
    }

    fn sec(s: i64, h: i32, l: i32, v: u32) -> Sec {
        Sec { sod: u32::try_from(s).unwrap_or(0), high: h, low: l, vol: v }
    }

    #[test]
    fn gapped_stop_pays_block_worst() {
        let d = day();
        let t = 36_000_i64;
        let mut rows = vec![sec(t - 1, 10_000, 10_000, 1_000)];
        for s in 0..10 {
            rows.push(sec(t + s, 10_000, 10_000, 1_000));
        }
        // the price gaps from Rs 100 to Rs 190-200 at 10:00:10, Rs 200-210 at 10:00:11 and 10:00:12
        rows.push(sec(t + 10, 20_000, 19_000, 1_000));
        rows.push(sec(t + 11, 21_000, 20_000, 1_000));
        rows.push(sec(t + 12, 21_000, 20_500, 1_000));
        let dt = DayTape::new(d, &rows, 2).unwrap_or_else(|e| panic!("{e}"));
        let tape = dt.tape().unwrap_or_else(|e| panic!("{e}"));
        let dial = dial().unwrap_or_else(|e| panic!("{e}"));
        // 1 lot of 50, share 100%, stop +20% (k = 2), no target, no time exit
        let (r, anc, cand, _) = trade_once(&tape, d, u32::try_from(t).unwrap_or(0), 50, 50, 4, 2, 4, 4, &dial).unwrap_or_else(|e| panic!("{e}")).unwrap_or_else(|| panic!("no trade"));
        assert_eq!(r.units, 50);
        assert_eq!(r.p0, 10_000);
        assert_eq!(cand, 2, "the +20% stop decides");
        // trigger second 10:00:10 (HIGH 200 >= 120), exit anchored at its end 10:00:11
        assert_eq!(cand_anchor(&anc, cand), Some(t + 11));
        // the buy-back pays the block [10:00:11, 10:00:13)'s highest HIGH, Rs 210: never the stop level Rs 120
        assert_eq!(r.bb_first_px, 21_000);
        assert_eq!(r.bb_units, 50);
        assert!(r.net < 0);
    }

    #[test]
    fn missing_seconds_inside_fill_blocks_are_jumped() {
        let d = day();
        let t = 36_000_i64;
        // 10:00:00 prints only 10 units (0 whole lots of 50 at 100%), 10:00:02-03 print nothing, 10:00:05 prints 1,000
        let rows = vec![sec(t - 1, 10_000, 10_000, 1_000), sec(t, 10_000, 9_900, 10), sec(t + 5, 10_100, 9_800, 1_000), sec(t + 600, 9_000, 8_000, 1_000)];
        let dt = DayTape::new(d, &rows, 2).unwrap_or_else(|e| panic!("{e}"));
        let tape = dt.tape().unwrap_or_else(|e| panic!("{e}"));
        let dial = dial().unwrap_or_else(|e| panic!("{e}"));
        let (r, _, _, _) = trade_once(&tape, d, u32::try_from(t).unwrap_or(0), 50, 50, 4, 4, 4, 0, &dial).unwrap_or_else(|e| panic!("{e}")).unwrap_or_else(|| panic!("no trade"));
        // the first filling block is [10:00:04, 10:00:06) (2-second lattice from T), at its lowest LOW Rs 98
        assert_eq!(r.e0, at_sod(d, t + 4));
        assert_eq!(r.p0, 9_800);
        assert_eq!(r.units, 50);
        // index options: 1-second blocks: the first filling block is [10:00:05, 10:00:06)
        let dt1 = DayTape::new(d, &rows, 1).unwrap_or_else(|e| panic!("{e}"));
        let tape1 = dt1.tape().unwrap_or_else(|e| panic!("{e}"));
        let (r1, _, _, _) = trade_once(&tape1, d, u32::try_from(t).unwrap_or(0), 50, 50, 4, 4, 4, 0, &dial).unwrap_or_else(|e| panic!("{e}")).unwrap_or_else(|| panic!("no trade"));
        assert_eq!(r1.e0, at_sod(d, t + 5));
    }

    /// v2.1 (FA1): the host's previous print of every row: the day's first print reads 09:14:59 (its volume spread from the open, the
    /// checker's stated reading); rows without volume are never a previous print.
    #[test]
    fn fa1_prev_prints_first_print_and_zero_volume_rows() {
        let d = day();
        let o = OPEN_S;
        let rows = vec![sec(o + 2, 1_000, 1_000, 0), sec(o + 5, 1_000, 900, 40), sec(o + 6, 1_000, 1_000, 0), sec(o + 10, 1_100, 1_000, 70), sec(o + 11, 1_100, 1_100, 5)];
        let dt = DayTape::with_reading(d, &rows, 2, CapMode::Prorate).unwrap_or_else(|e| panic!("{e}"));
        let p: Vec<i64> = dt.prev.iter().map(|&x| sod_of(x)).collect();
        assert_eq!(p.get(1).copied(), Some(o - 1), "the day's first print: 09:14:59");
        assert_eq!(p.get(3).copied(), Some(o + 5), "the print at 09:15:10 follows the print at 09:15:05 (the zero-volume 09:15:06 row is no print)");
        assert_eq!(p.get(4).copied(), Some(o + 10));
        // the v2 replay switch builds no previous prints at all (the crate's row reading)
        let dv2 = DayTape::with_reading(d, &rows, 2, CapMode::Row).unwrap_or_else(|e| panic!("{e}"));
        assert!(dv2.prev.is_empty() && dv2.mode() == CapMode::Row);
    }

    /// v2.1 (checker F-A, REQ 56): a sale decided at T = 10:00:00 on a SPARSE stock tape: the last print before T at 09:59:30 (inside the
    /// signal candle), the next at 10:00:01 carrying 6,000 units traded over the 31 seconds (09:59:30, 10:00:01]. v2 counted all 6,000 in the
    /// first block [10:00:00, 10:00:02); v2.1 counts only its 2 seconds inside the block: 6,000 x 2 / 31 = 387.1 -> 7 lots of 50 at 100%;
    /// the rest of the order waits for the next print (10:00:20, a 1-second silence before it at 1 s: whole).
    #[test]
    fn fa1_sparse_sale_counts_no_volume_from_the_signal_candle() {
        let d = day();
        let t = 36_000_i64;
        let rows = vec![sec(t - 30, 10_000, 10_000, 500), sec(t + 1, 9_800, 9_700, 6_000), sec(t + 19, 9_600, 9_600, 100), sec(t + 20, 9_500, 9_500, 1_000), sec(t + 3_000, 9_000, 9_000, 10_000)];
        let dial = dial().unwrap_or_else(|e| panic!("{e}"));
        let run = |m: CapMode| {
            let dt = DayTape::with_reading(d, &rows, 2, m).unwrap_or_else(|e| panic!("{e}"));
            let tape = dt.tape().unwrap_or_else(|e| panic!("{e}"));
            // 10 lots of 50, share 100% (level 4), stop +100% (k 4), no target, time exit 5 min (m 0)
            trade_once(&tape, d, u32::try_from(t).unwrap_or(0), 50, 500, 4, 4, 4, 0, &dial).unwrap_or_else(|e| panic!("{e}")).unwrap_or_else(|| panic!("no trade")).0
        };
        let (v2, an, pr) = (run(CapMode::Row), run(CapMode::Anchor), run(CapMode::Prorate));
        // v2: the whole 6,000 in the first block: 500 units at its LOW Rs 97, known at 10:00:02
        assert_eq!((v2.units, v2.sold_value, v2.sale_blocks), (500, 500 * 9_700, 1));
        // anchor reading: the 2 seconds (09:59:59, 10:00:01] at or after the send: the same 387 here (the block starts at the send)
        assert_eq!((an.units, an.sale_blocks), (500, 3));
        // v2.1: 350 units (7 lots) in the first block at Rs 97; then the block [t+18, t+20) holds the 10:00:19 print as its SECOND second
        // (g 18, n 2: 100 x 2 / 18 = 11.1 -> 0 lots; the anchor reading counts all 18 seconds: 100 -> 2 lots) and [t+20, t+22) the 10:00:20
        // print (g 1: 1,000 whole): the last 150 units at Rs 95
        assert_eq!((pr.units, pr.sale_blocks), (500, 2));
        assert_eq!(pr.sold_value, 350 * 9_700 + 150 * 9_500);
        assert_eq!(pr.e0, at_sod(d, t));
        assert!(pr.sale_known > v2.sale_known, "the v2.1 sale completes later");
    }

    #[test]
    fn premium_005_sale_respects_protection_floor() {
        // a Rs 0.05 option: the protection limit never goes below one tick (5 paisa); the sale fills at 5 paisa
        let d = day();
        let t = 36_000_i64;
        let rows = vec![sec(t - 1, 5, 5, 100_000), sec(t, 5, 5, 100_000), sec(t + 1, 5, 5, 100_000)];
        let dt = DayTape::new(d, &rows, 1).unwrap_or_else(|e| panic!("{e}"));
        let tape = dt.tape().unwrap_or_else(|e| panic!("{e}"));
        let dial = dial().unwrap_or_else(|e| panic!("{e}"));
        let (r, _, _, _) = trade_once(&tape, d, u32::try_from(t).unwrap_or(0), 50, 50, 4, 0, 4, 4, &dial).unwrap_or_else(|e| panic!("{e}")).unwrap_or_else(|| panic!("no trade"));
        assert_eq!(r.limit_in, 5);
        assert_eq!(r.p0, 5);
        assert_eq!(r.units, 50);
    }
}
