//! Pricing one (row, side, exit set) through the frozen common fill crate v2 (plan 5), the template being billions
//! `walk/stk70.rs` (shares route): no law here, only call sites.
//!
//! - Entry: one market order anchored at T = the decision candle's END (REQ 70c), sized at the decision by crate S11 (the last
//!   traded second strictly before T: its HIGH for a buy, its LOW for a sale), qty = the RUN row `qty_rule`'s count of slot x 5 /
//!   reference (SNIPER s16: `closest`, half way the lower count; or `floor`, crate S11's `size_units`), cut to the largest count
//!   whose money in fits the account (base Q7, [`crate::size`]); Dhan's market protection limit (REQ 70a); every share level in ONE
//!   `walk`.
//! - Signal exits (XS, XE): the exit anchor is known from candles before any fill; the entry is walked with stop = that anchor.
//! - Stop and timer exits (XQ-A, XQ-G): `path::scan_one` per level on the entry's filling blocks gives the anchor A; **S2: the
//!   host NEVER links `FillPath::cut_at`; where a stop or timer wins at A for a level, that level's entry is RE-WALKED with stop A**
//!   (at most one re-walk per winning level, so at most 5 an order, counted). A 0-unit entry sends no exit order (counted).
//! - SNIPER s20 combined exit (XC, [`Plan::ClockOrSignal`]): the signal anchor (the XS flip of the same setting, or the square-off)
//!   and the crate's plain time exit at E0 + secs through the same `scan_one` path as the timer exits; the earlier wins per level
//!   (the clock only when E0 + secs is strictly before the signal anchor); a winning clock re-walks the entry with its anchor (S2).
//!   check-build-xc B1 (REQ 70 S14): a clock anchored at or after the 15:08:00 own-order cut is never sent; the level waits for the
//!   15:10:00 square-off (its signal plan), with no re-walk.
//! - The 15:10:00 square-off by 2-second blocks to the segment end (crate S14); units still open at the segment end are
//!   "unclosable in session", valued at the worst traded price since the square-off, else since E0 (crate S15), counted.
//! - Charges: Dhan's equity-intraday estimator (`costs::equity_clock::price_trade`, the dearer of the dated and today's clocks):
//!   net only.

use common_fill::path::{EntryView, Rule, runs, scan_one};
use common_fill::{BlockFill, Dial, FillErr, Kind, LevelFill, LevelMask, MAXL, Order, Paisa, RangeMm, SecIn, SecIndex, Seg, Side, Tape, US_S, Units, Us, Why, walk};
use costs::equity_clock::{Fill as CFill, Order as COrder, TradeFills, price_trade};
use costs::rate::OrderSide;

use crate::size::QtyRule;

/// IST offset, seconds.
pub const IST_S: i64 = 19_800;

/// The UTC instant (microseconds) of IST day `day` (days since 1970-01-01) at `sod` seconds after IST midnight.
#[must_use]
pub const fn at_sod(day: i32, sod: i64) -> Us {
    (day as i64 * 86_400 + sod - IST_S) * US_S
}

/// A named refusal of one priced row (counted per reason, never a silent skip).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No traded second before the decision to size from (crate S11).
    NoReference,
    /// The slot buys no whole share (SNIPER s16 closest: one share costs at least twice the target; floor: more than the target).
    ZeroQty,
    /// The account base (Q7) leaves no whole share (the rule's count is above 0, the account's money buys none).
    AccountZero,
    /// The anchor is at or after the broker's own-order cut (crate S14).
    AfterCut,
    /// The crate refused (named by its error).
    Fill(String),
    /// The charges estimator refused.
    Charges(String),
    /// A net or a run-down left its stored width (checked conversion: S4, F32).
    StoreOverflow,
}

impl From<FillErr> for Refusal {
    fn from(e: FillErr) -> Self {
        Self::Fill(e.to_string())
    }
}

/// One instrument-day for the crate (stk70's `Day70`).
#[derive(Clone, Debug)]
pub struct Day {
    /// The IST day.
    pub day: i32,
    /// The rows (corrected GDFL seconds: LTQ = 0 new-price rows kept; a price-only second has volume 0, crate S2).
    pub rows: Vec<SecIn>,
    ix: SecIndex,
    mm: RangeMm,
    segs: Vec<Seg>,
    /// The seconds whose only trade rows are new-price LTQ = 0 rows (F14), as UTC microseconds.
    pub price_only: Vec<Us>,
}

impl Day {
    /// The day of `rows` with its session segments `segs` ([open, end) epoch seconds; the first segment of a REGULAR session
    /// opens with the call auction).
    ///
    /// # Errors
    /// The crate's refusal of the rows, or no segment.
    pub fn new(day: i32, rows: Vec<SecIn>, segs: &[(i64, i64)], regular: bool, price_only: Vec<Us>) -> Result<Self, String> {
        let segs: Vec<Seg> = segs
            .iter()
            .enumerate()
            .map(|(i, &(o, e))| Seg { open: o * US_S, last_sec: (e - 1) * US_S, auction_open: regular && i == 0 })
            .collect();
        let end = segs.last().map(Seg::end).ok_or("no session segment")?;
        let first = segs.first().map_or(0, |s| s.open);
        let t0 = first.min(rows.first().map_or(Us::MAX, |r| r.ts));
        let t0 = t0 - t0.rem_euclid(US_S);
        let t1 = end.max(rows.last().map_or(0, |r| r.ts));
        let ix = SecIndex::new(&rows, t0, t1).map_err(|e| e.to_string())?;
        let mm = RangeMm::new(&rows);
        Ok(Self { day, rows, ix, mm, segs, price_only })
    }

    /// The crate's view.
    ///
    /// # Errors
    /// The crate's refusal.
    pub fn tape(&self) -> Result<Tape<'_>, FillErr> {
        Tape::new(&self.rows, &self.ix, Some(&self.mm))?.with_segments(&self.segs)
    }

    /// Whether a price-only second lies in [a, b).
    #[must_use]
    pub fn price_only_in(&self, a: Us, b: Us) -> bool {
        let i = self.price_only.partition_point(|&t| t < a);
        self.price_only.get(i).is_some_and(|&t| t < b)
    }
}

/// The pricing rows (all from SNIPER-RUN-v1.tsv).
#[derive(Clone, Copy, Debug)]
pub struct Cfg {
    /// The share dial.
    pub dial: Dial,
    /// Leverage num / den.
    pub lev: (u32, u32),
    /// Dhan's protection, bp.
    pub protection_bp: u32,
    /// The protection limit's tick.
    pub limit_tick: Paisa,
    /// The square-off, seconds after IST midnight.
    pub square_off_sod: i64,
    /// The broker's own-order cut: no order anchored at or after it, seconds after IST midnight.
    pub cut_sod: i64,
    /// SNIPER s16: the sizing rule (RUN row `qty_rule`, SNIPER-RUN-v5.tsv).
    pub qty_rule: QtyRule,
    /// The account (RUN row `account_paisa`): the most money free at any entry (base Q7: profits never added, one position at a
    /// time); the pick's own losses lower it further in the trade rows (`size::account_walk`).
    pub account: Paisa,
}

/// How one exit set ends a trade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    /// A signal exit at `anchor` (a candle END), or the square-off when `anchor` is at or after it.
    Signal {
        /// The exit anchor.
        anchor: Us,
        /// F17: the entry candle already closed on the exit side (exit at the next candle's END).
        immediate: bool,
    },
    /// Hold to the square-off (X-1510).
    Clock,
    /// REQ 47's small stop at `x` paisa from R(s) (XQ-A).
    Stop {
        /// The distance.
        x: Paisa,
    },
    /// The go-positive timer (XQ-G).
    Timer {
        /// Seconds after E0.
        secs: u32,
    },
    /// A composite (XSA / XEA, S11): the stop or the signal, whichever comes first (a tie goes to the stop).
    StopOrSignal {
        /// The distance.
        x: Paisa,
        /// The signal anchor.
        anchor: Us,
        /// F17, as for the signal part.
        immediate: bool,
    },
    /// SNIPER s20 (XC, the ONE combined exit): the signal (the Supertrend flip's candle END, or the square-off when no flip comes
    /// before it) or a plain clock exit `secs` after E0, whichever comes first. The clock is the crate's `Rule::TimeExit` through
    /// `scan_one`, the timer exits' path: it fires only when E0 + `secs` is strictly before the signal anchor; its exit is anchored
    /// at max(E0 + `secs`, E1); where it wins at A (A at or before the signal anchor) that level's entry is RE-WALKED with stop A
    /// (S2, never `FillPath::cut_at`) and the exit order is anchored at A. Otherwise the level is exactly the signal plan's.
    ClockOrSignal {
        /// The clock, seconds after E0.
        secs: u32,
        /// The signal anchor (already clamped: the square-off when no flip comes before it).
        anchor: Us,
        /// F17, as for the signal part (flagged only on a level whose exit the signal part decided).
        immediate: bool,
    },
}

/// Flags of a priced level (the stored u16).
pub mod flag {
    /// A block clipped by an order's stop.
    pub const CLIPPED: u16 = 1;
    /// Units unclosable in session (crate S15).
    pub const UNCLOSABLE: u16 = 2;
    /// An order spanning a suspected feed hole (crate S3).
    pub const HOLE: u16 = 4;
    /// The entry filled 0 units (no exit order sent).
    pub const ZERO: u16 = 8;
    /// A filling block holds a price-only second (F14).
    pub const PRICE_ONLY: u16 = 16;
    /// F17: the immediate Supertrend exit.
    pub const IMMEDIATE: u16 = 32;
    /// S2: the entry was re-walked with the stop / timer anchor.
    pub const REWALKED: u16 = 64;
    /// REQ 70a / S3 (HOST REPAIR B2): Dhan's market protection HELD units: a block of the booked entry order or of the signal /
    /// stop exit order printed past its protection LIMIT while the level still had units to fill (`common_fill::LevelFill::
    /// past_limit` > 0; the crate skips that block, the rest moves to the next block). The square-off carries no limit. Note:
    /// [`CLIPPED`] is a different fact (the crate's `LevelFill::clipped`: a block cut by the order's own STOP).
    pub const PROTECTED: u16 = 128;
    /// SNIPER s16 + base Q7: the rule's count was cut to the largest count the account's money fits (never under the floor rule,
    /// whose count never needs more than the slot; named on the trade row).
    pub const ACCOUNT_CAP: u16 = 256;
}

/// HOST REPAIR B2 (S3: "Dhan's market protection ... counted per pick"): over a pick's trades' flags at its own (N, level), the
/// trades on which the protection held units ([`flag::PROTECTED`]) and, beside, the trades with a block clipped by an order's stop
/// ([`flag::CLIPPED`]). Mutant `m_b2` counts the stop clip as the protection hold (the check text's field): the matrix test
/// SN-B2 catches it.
#[must_use]
pub fn held_counts(flags: impl Iterator<Item = u16>) -> (u32, u32) {
    let prot = if cfg!(feature = "m_b2") { flag::CLIPPED } else { flag::PROTECTED };
    flags.fold((0, 0), |(p, c), f| (p + u32::from(f & prot != 0), c + u32::from(f & flag::CLIPPED != 0)))
}

/// One level of one N, priced.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelOut {
    /// Net after every Dhan charge, paisa.
    pub net: i64,
    /// The worst run-down, bp of the entry value (S4).
    pub rd_bp: u16,
    /// The exit anchor (the stop / timer trigger's, the signal's, or the square-off's).
    pub exit_anchor: Us,
    /// The last fill's known instant (the trade's exit second).
    pub exit_known: Us,
    /// Units entered.
    pub units: Units,
    /// Units unclosable in session.
    pub unclosed: Units,
    /// The flags.
    pub flags: u16,
    /// SNIPER s16 trade rows: the entry fills' value (sum of units x price), paisa.
    pub entry_value: Paisa,
    /// SNIPER s16 trade rows: the closing fills' value (signal / stop exit + square-off + unclosed units at the worst price), paisa.
    pub exit_value: Paisa,
}

/// One row priced at every N asked and every level of the dial.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Priced {
    /// Per N (in the order asked), per level.
    pub lv: Vec<[LevelOut; MAXL]>,
    /// The entry's first filling block's start at the first N (0 = none).
    pub entry_first: Us,
    /// Re-walks done (S2), counted.
    pub rewalks: u32,
    /// Crate steps (blocks read) over every order: the SC-PRICE counter.
    pub steps: u64,
    /// SNIPER s16: the size reference (crate S11 `size_ref`), paisa.
    pub refpx: Paisa,
    /// SNIPER s16: per N (in the order asked) the sized count (the count taken, the rule's own count, cut by the account base).
    pub sized: Vec<crate::size::Sized>,
}

#[allow(clippy::too_many_arguments, reason = "one order: the crate walk's own arguments (stk70 `leg`)")]
fn leg(tape: &Tape<'_>, side: Side, anchor: Us, stop: Us, limit: Option<Paisa>, kind: Kind, dial: &Dial, mask: LevelMask, want: &[Units; MAXL]) -> Result<([LevelFill; MAXL], Vec<Vec<BlockFill>>), FillErr> {
    let mut w = *want;
    let mut out = [LevelFill::default(); MAXL];
    let mut fills: Vec<Vec<BlockFill>> = vec![Vec::new(); MAXL];
    if stop <= anchor {
        return Ok((out, fills));
    }
    let o = Order { side, anchor, stop, unit: 1, limit, kind };
    {
        let mut rec = |l: usize, b: &BlockFill| {
            if b.why == Why::Filled
                && let Some(v) = fills.get_mut(l)
            {
                v.push(*b);
            }
        };
        walk(tape, &o, dial, mask, &mut w, None, None, &mut out, Some(&mut rec))?;
    }
    Ok((out, fills))
}

fn corder(side: Side, by_broker: bool, fills: &[BlockFill]) -> Result<Option<COrder>, Refusal> {
    let mut v: Vec<CFill> = Vec::with_capacity(fills.len());
    for b in fills {
        if b.units == 0 {
            continue;
        }
        let q = i64::try_from(b.units).map_err(|_| Refusal::StoreOverflow)?;
        v.push(CFill { second: b.block.at / US_S, quantity: q, price: brutex_core::price::Paisa::from_raw(b.px) });
    }
    if v.is_empty() {
        return Ok(None);
    }
    Ok(Some(COrder { side: if side == Side::Buy { OrderSide::Buy } else { OrderSide::Sell }, by_broker, fills: v }))
}

fn ymd(day: i32) -> (i64, i64, i64) {
    // days since 1970-01-01 -> civil date (Howard Hinnant's algorithm).
    let z = i64::from(day) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Dhan's net of one level's trade: entry, exit, square-off and unclosed orders (the unclosed units as a broker order at the
/// segment end at the worst price, crate S15).
#[allow(clippy::too_many_arguments, reason = "one trade's orders")]
fn net_of(day: i32, side_in: Side, entry: &[BlockFill], exit: &[BlockFill], square: &[BlockFill], unclosed: Units, worst: Paisa, end: Us) -> Result<i64, Refusal> {
    let side_out = side_in.other();
    let mut orders = Vec::new();
    if let Some(o) = corder(side_in, false, entry)? {
        orders.push(o);
    } else {
        return Ok(0);
    }
    if let Some(o) = corder(side_out, false, exit)? {
        orders.push(o);
    }
    if let Some(o) = corder(side_out, true, square)? {
        orders.push(o);
    }
    if unclosed > 0 {
        let b = BlockFill { block: common_fill::Block { at: end, ..common_fill::Block::default() }, units: unclosed, px: worst, ..BlockFill::default() };
        if let Some(o) = corder(side_out, true, &[b])? {
            orders.push(o);
        }
    }
    let (y, m, d) = ymd(day);
    let td = costs::day::TradeDay::new(u16::try_from(y).map_err(|_| Refusal::StoreOverflow)?, u8::try_from(m).map_err(|_| Refusal::StoreOverflow)?, u8::try_from(d).map_err(|_| Refusal::StoreOverflow)?)
        .map_err(|e| Refusal::Charges(format!("{e:?}")))?;
    let p = price_trade(&TradeFills { orders }, td).map_err(|e| Refusal::Charges(format!("{e:?}")))?;
    Ok(p.net().raw())
}

/// Prices one row: decision candle END `t`, `long` or short, the exit plan, the slots (paisa) of each N asked.
///
/// # Errors
/// A named refusal (counted by the caller).
#[allow(clippy::too_many_lines, reason = "one round trip at every N and level: entry, stop scan, S2 re-walks, exit, square-off, charges")]
pub fn price(d: &Day, cfg: &Cfg, t: Us, long: bool, plan: Plan, slots: &[Paisa]) -> Result<Priced, Refusal> {
    let tape = d.tape()?;
    let sq = at_sod(d.day, cfg.square_off_sod);
    let cut = at_sod(d.day, cfg.cut_sod);
    if t >= cut {
        return Err(Refusal::AfterCut);
    }
    let side_in = if long { Side::Buy } else { Side::Sell };
    let side_out = side_in.other();
    let n = cfg.dial.len();
    let mask = LevelMask::all(n);
    let end = tape.session_end();
    let refpx = common_fill::limits::size_ref(&tape, t, side_in).ok_or(Refusal::NoReference)?;
    let limit_in = common_fill::limits::protection(refpx, side_in, cfg.protection_bp, cfg.limit_tick);
    let mut res = Priced::default();
    for (ni, &slot) in slots.iter().enumerate() {
        // SNIPER s16: the RUN row's rule (closest | floor), cut by the account base (Q7); the crate's floor stays the floor rule.
        let sz = crate::size::size(cfg.qty_rule, slot, cfg.account, cfg.lev, refpx, 1);
        if sz.rule_qty == 0 {
            return Err(Refusal::ZeroQty);
        }
        if sz.qty == 0 {
            return Err(Refusal::AccountZero);
        }
        let qty = sz.qty;
        res.refpx = refpx;
        res.sized.push(sz);
        let mut want = [0_u64; MAXL];
        for w in want.iter_mut().take(n) {
            *w = qty;
        }
        let signal = match plan {
            Plan::Signal { anchor, .. } | Plan::StopOrSignal { anchor, .. } => anchor.min(sq),
            // MUTANT m_xc_flip only: the combined exit ignores the Supertrend flip (caught by matrix sn20_*).
            Plan::ClockOrSignal { .. } if cfg!(feature = "m_xc_flip") => sq,
            Plan::ClockOrSignal { anchor, .. } => anchor.min(sq),
            Plan::Clock | Plan::Stop { .. } | Plan::Timer { .. } => sq,
        };
        // SNIPER s20: the F17 flag of the combined exit goes only on a level whose exit the signal part decided.
        let xc_immediate = matches!(plan, Plan::ClockOrSignal { immediate: true, .. });
        // The entry, walked to the signal anchor (or the square-off), every level in one pass.
        let (ent, ent_fills) = leg(&tape, side_in, t, signal, Some(limit_in), Kind::Entry, &cfg.dial, mask, &want)?;
        res.steps += ent.iter().map(|f| u64::from(f.steps.blocks)).sum::<u64>();
        if ni == 0 {
            res.entry_first = ent.iter().take(n).filter(|f| f.units > 0).map(|f| f.e0).min().unwrap_or(0);
        }
        let mut out = [LevelOut::default(); MAXL];
        for l in 0..n {
            let (Some(e), Some(ef)) = (ent.get(l), ent_fills.get(l)) else { continue };
            let mut lo = LevelOut::default();
            if sz.capped {
                lo.flags |= flag::ACCOUNT_CAP;
            }
            if e.clipped > 0 {
                lo.flags |= flag::CLIPPED;
            }
            if e.hole_span {
                lo.flags |= flag::HOLE;
            }
            if matches!(plan, Plan::Signal { immediate: true, .. } | Plan::StopOrSignal { immediate: true, .. }) {
                lo.flags |= flag::IMMEDIATE;
            }
            if e.units == 0 {
                // A 0-unit entry sends no exit order (S2), counted.
                lo.flags |= flag::ZERO;
                if e.past_limit > 0 {
                    lo.flags |= flag::PROTECTED;
                }
                if xc_immediate {
                    lo.flags |= flag::IMMEDIATE;
                }
                lo.exit_anchor = signal;
                if let Some(x) = out.get_mut(l) {
                    *x = lo;
                }
                continue;
            }
            // The level's exit anchor: a stop / timer trigger (scan on its filling blocks) or the signal / square-off.
            let rule = match plan {
                Plan::Stop { x } | Plan::StopOrSignal { x, .. } => Some(Rule::Stop { x }),
                Plan::Timer { secs } => Some(Rule::GoPositive { secs }),
                // MUTANT m_xc_clock only: the combined exit's clock is the go-positive timer (caught by matrix sn20_*).
                Plan::ClockOrSignal { secs, .. } if cfg!(feature = "m_xc_clock") => Some(Rule::GoPositive { secs }),
                // SNIPER s20: the plain clock (the crate's time exit at E0 + secs).
                Plan::ClockOrSignal { secs, .. } => Some(Rule::TimeExit { secs }),
                Plan::Signal { .. } | Plan::Clock => None,
            };
            let mut anchor = signal;
            let mut entry_fill = *e;
            let mut entry_blocks: Vec<BlockFill> = ef.clone();
            // Whether the rule (stop / timer / SNIPER s20 clock) won this level.
            let mut won = false;
            if let Some(r) = rule {
                let ev = EntryView { e0: e.e0, e1: e.e1, fills: ef };
                let mut st = 0_u32;
                let (trig, _facts) = scan_one(&tape, &ev, signal, long, &[r], &[], &mut st);
                res.steps += u64::from(st);
                // check-build-xc B1 (REQ 70 S14, MAIN ruling 14 Nov 15:1x): the combined exit's clock never sends an own order
                // anchored at or after the broker's 15:08:00 cut; the level stays on the signal plan (then always the 15:10:00
                // square-off: no flip lies in [15:08, 15:10)), no re-walk. Every other plan as before.
                // MUTANT m_xc_cut only: the cut ignored (caught by matrix sn20_the_clock_at_or_after_the_cut_waits_for_the_square_off).
                if let Some(Some(tr)) = trig.first()
                    && tr.anchor <= signal
                    && (cfg!(feature = "m_xc_cut") || !(matches!(plan, Plan::ClockOrSignal { .. }) && tr.anchor >= cut))
                {
                    anchor = tr.anchor;
                    won = true;
                    // S2: re-walk this level's entry with stop = the trigger's anchor (never FillPath::cut_at).
                    let mut w1 = [0_u64; MAXL];
                    if let Some(x) = w1.get_mut(l) {
                        *x = qty;
                    }
                    let (re, rf) = leg(&tape, side_in, t, anchor, Some(limit_in), Kind::Entry, &cfg.dial, LevelMask::one(l), &w1)?;
                    res.rewalks += 1;
                    lo.flags |= flag::REWALKED;
                    entry_fill = re.get(l).copied().unwrap_or_default();
                    entry_blocks = rf.get(l).cloned().unwrap_or_default();
                    res.steps += u64::from(entry_fill.steps.blocks);
                }
            }
            lo.units = entry_fill.units;
            lo.entry_value = entry_fill.value;
            lo.exit_anchor = anchor;
            if xc_immediate && !won {
                lo.flags |= flag::IMMEDIATE;
            }
            // B2: the protection hold of the BOOKED entry (the re-walked one when S2 re-walked it).
            if entry_fill.past_limit > 0 {
                lo.flags |= flag::PROTECTED;
            }
            if entry_fill.units == 0 {
                lo.flags |= flag::ZERO;
                if let Some(x) = out.get_mut(l) {
                    *x = lo;
                }
                continue;
            }
            let mut held = [0_u64; MAXL];
            if let Some(x) = held.get_mut(l) {
                *x = entry_fill.units;
            }
            let kind = match rule {
                Some(Rule::Stop { .. }) if anchor < signal || matches!(plan, Plan::Stop { .. }) => Kind::Stop,
                Some(Rule::GoPositive { .. }) => Kind::GoPositive,
                Some(Rule::TimeExit { .. }) if won => Kind::TimeExit,
                _ => Kind::SignalExit,
            };
            let (xf, xb) = if anchor < sq {
                let lim = common_fill::limits::size_ref(&tape, anchor, side_out).map(|p| common_fill::limits::protection(p, side_out, cfg.protection_bp, cfg.limit_tick));
                leg(&tape, side_out, anchor, sq, lim, kind, &cfg.dial, LevelMask::one(l), &held)?
            } else {
                ([LevelFill::default(); MAXL], vec![Vec::new(); MAXL])
            };
            let xl = xf.get(l).copied().unwrap_or_default();
            res.steps += u64::from(xl.steps.blocks);
            let mut rest = [0_u64; MAXL];
            if let Some(x) = rest.get_mut(l) {
                *x = entry_fill.units.saturating_sub(xl.units);
            }
            let (sf, sb) = leg(&tape, side_out, sq, end, None, Kind::SquareOff, &cfg.dial, LevelMask::one(l), &rest)?;
            let sl = sf.get(l).copied().unwrap_or_default();
            res.steps += u64::from(sl.steps.blocks);
            let unclosed = rest.get(l).copied().unwrap_or(0).saturating_sub(sl.units);
            let sell = side_out == Side::Sell;
            let worst = tape.worst_between(sq, end, sell).or_else(|| tape.worst_between(t, end, sell)).unwrap_or(0);
            if unclosed > 0 {
                lo.flags |= flag::UNCLOSABLE;
            }
            lo.unclosed = unclosed;
            let unclosed_value = i64::try_from(i128::from(unclosed) * i128::from(worst)).map_err(|_| Refusal::StoreOverflow)?;
            lo.exit_value = xl.value.checked_add(sl.value).and_then(|v| v.checked_add(unclosed_value)).ok_or(Refusal::StoreOverflow)?;
            if xl.clipped > 0 || sl.clipped > 0 {
                lo.flags |= flag::CLIPPED;
            }
            if xl.past_limit > 0 || sl.past_limit > 0 {
                lo.flags |= flag::PROTECTED;
            }
            if xl.hole_span || sl.hole_span {
                lo.flags |= flag::HOLE;
            }
            let all_blocks = entry_blocks.iter().chain(xb.get(l).into_iter().flatten()).chain(sb.get(l).into_iter().flatten());
            if all_blocks.clone().any(|b| d.price_only_in(b.block.at, b.block.to)) {
                lo.flags |= flag::PRICE_ONLY;
            }
            lo.exit_known = all_blocks.map(|b| b.block.known).max().unwrap_or(0);
            let xbl = xb.get(l).map_or(&[][..], Vec::as_slice);
            let sbl = sb.get(l).map_or(&[][..], Vec::as_slice);
            lo.net = net_of(d.day, side_in, &entry_blocks, xbl, sbl, unclosed, worst, end - US_S)?;
            let exit_to = if lo.exit_known > 0 { lo.exit_known } else { end };
            let ((_, _), (down, _)) = runs(&tape, entry_fill.e0, exit_to, long, entry_fill.value, entry_fill.units);
            let bp = i128::from(down) * 10_000 / i128::from(entry_fill.value.max(1));
            lo.rd_bp = u16::try_from(bp).map_err(|_| Refusal::StoreOverflow)?;
            if let Some(x) = out.get_mut(l) {
                *x = lo;
            }
        }
        res.lv.push(out);
    }
    Ok(res)
}

/// The derived composite (S11, plan 5 item 6): per level the earlier of the two parts' exit anchors wins (a tie goes to the stop),
/// and the composite's level is the winning part's priced level. FIRST mode only (the same entry fills); red test G-COMP
/// asserts it equals the composite priced through crate v2.
#[must_use]
pub fn derive_composite(signal_part: &Priced, stop_part: &Priced) -> Priced {
    let mut out = signal_part.clone();
    for (ni, lvs) in out.lv.iter_mut().enumerate() {
        for (l, x) in lvs.iter_mut().enumerate() {
            let s = stop_part.lv.get(ni).and_then(|v| v.get(l)).copied().unwrap_or_default();
            if s.exit_anchor <= x.exit_anchor {
                *x = s;
            }
        }
    }
    out.rewalks = 0;
    out.steps = 0;
    out
}

/// The stored outcome row's width (the layout of [`encode`]) for `nn` N and `nlev` levels, padded to 8 bytes: 112 B for 3 N x 5
/// levels, 352 B for the 11-N grid x 5 levels (every OUT store this host writes). HOST REPAIR B3: `sniper-q` reads a partition with
/// it, never with a width guessed from the part's name.
#[must_use]
pub const fn record_width(nn: usize, nlev: usize) -> usize {
    (nn * nlev * 6 + 20).div_ceil(8) * 8
}

/// The stored outcome row (S4 with S11; RECHECK-1b X3's layout): `cols` nets (i32 paisa) and run-downs (u16 bp), the exit second
/// (u32 since the period origin), flags (u16, OR over levels), the entry block's first second (u32), the exit anchor per level at
/// the first N (5 x u16 seconds after the entry), padded to 8 bytes. 112 B for 3 N x 5 levels; 352 B for 11 N x 5 levels.
///
/// # Errors
/// `StoreOverflow` for any field outside its width (checked conversion: never a wrap).
pub fn encode(p: &Priced, nlev: usize, origin: Us) -> Result<Vec<u8>, Refusal> {
    let mut b = Vec::with_capacity(352);
    let ov = || Refusal::StoreOverflow;
    for lvs in &p.lv {
        for x in lvs.iter().take(nlev) {
            b.extend_from_slice(&i32::try_from(x.net).map_err(|_| ov())?.to_le_bytes());
        }
    }
    for lvs in &p.lv {
        for x in lvs.iter().take(nlev) {
            b.extend_from_slice(&x.rd_bp.to_le_bytes());
        }
    }
    let first = p.lv.first();
    let exit = first.map_or(0, |v| v.iter().take(nlev).map(|x| x.exit_known).max().unwrap_or(0));
    let secs = |u: Us| -> Result<u32, Refusal> { if u == 0 { Ok(0) } else { u32::try_from((u - origin) / US_S).map_err(|_| ov()) } };
    b.extend_from_slice(&secs(exit)?.to_le_bytes());
    let flags = p.lv.iter().flat_map(|v| v.iter().take(nlev)).fold(0_u16, |a, x| a | x.flags);
    b.extend_from_slice(&flags.to_le_bytes());
    b.extend_from_slice(&secs(p.entry_first)?.to_le_bytes());
    for l in 0..5 {
        let a = first.and_then(|v| v.get(l)).map_or(0, |x| x.exit_anchor);
        let rel = if a == 0 || p.entry_first == 0 { 0 } else { u16::try_from(((a - p.entry_first) / US_S).max(0)).map_err(|_| ov())? };
        b.extend_from_slice(&rel.to_le_bytes());
    }
    while b.len() % 8 != 0 {
        b.push(0);
    }
    Ok(b)
}
