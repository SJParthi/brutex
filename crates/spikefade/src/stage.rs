//! v3 STAGE A (EXITS-DYNAMIC-DESIGN 3.4): EVERY rule of the menu on every event and every entry path (share level l, lot count q),
//! exactly once, shared by every account. Per path: the 95 anchors (crate v2.2 `scan_fast`: O(log n) per rule, the trail through the
//! per-second trail lookup), then the DISTINCT exit keys priced once each through the v2.1 trip machinery:
//! - an anchor before 15:08:00: the own buy-back from the anchor (S13 cancels the sale's rest there);
//! - an anchor in [15:08:00, 15:10:00): NO own order (S14); the sale's unfilled rest is cancelled at the anchor (B2, Q3: CUT_HELD +
//!   REWALKED) and the filled units wait for the 15:10:00 square-off;
//! - no anchor (or one at or after 15:10:00): the sale to 15:10:00 and the square-off (v2.1's candidate 13).
//!
//! The watch window runs to 15:10:00 (Q5). The v2 COMPATIBILITY switch (`compat`, critic F1, for proof P0): the watch ends at 15:08:00, no
//! B2 cancel, every anchor at or after 15:08:00 is the square-off: exactly the v2.1 engine's `cand_of` reading.
//!
//! Anchors are u16 seconds after 09:00:00 (v2.1's encoding); [`NEVER3`] = no trigger in the window, [`ABSENT`] = the rule does not exist at
//! this event (an unready reading, a TP floor not below the entry, a theta level at or below 0: counted per rule).

use common_fill::path::EntryView;
use common_fill::xrule::{TrailIndex, XRule, scan_fast, scan_law};
use common_fill::{Dial, Tape};

use crate::cal::{CUT_S, SQ_S, at_sod, sod_of};
use crate::cube::Cube;
use crate::margin::VarRow;
use crate::menu::{ABSENT_WHY, MRule, Rd, rule_at, tf_dependent};
use crate::trade::{BASE_S, BaseErr, EvT, NL, Out, Paths, Refuse, Trip, out_of, record_base, size_event, trip, truncate};

/// The most menu rules a row holds.
pub const NR: usize = 96;
/// The most tf-dependent rules (SL-H 4 + SL-S 4).
pub const NTF: usize = 8;
/// The rule does not exist at this event.
pub const ABSENT: u16 = u16::MAX;
/// No trigger in the watch window.
pub const NEVER3: u16 = u16::MAX - 1;
/// Out flag bit 6: the sale was cut at a HELD anchor (B2: CUT_HELD + REWALKED).
pub const FLAG_CUT_HELD: u8 = 64;

/// The menu as stage A reads it.
#[derive(Clone, Debug, Default)]
pub struct MenuA {
    pub rows: Vec<MRule>,
    /// Per row: its index among the tf-dependent rows (None: tf-independent).
    pub tfdep: Vec<Option<u8>>,
    /// The tf-dependent rows, in order.
    pub tfrows: Vec<usize>,
}

impl MenuA {
    /// From the loaded menu.
    ///
    /// # Errors
    /// More rows than [`NR`] or tf-dependent rows than [`NTF`].
    pub fn new(rows: &[MRule]) -> Result<Self, String> {
        if rows.len() > NR {
            return Err(format!("menu rows {} > {NR}", rows.len()));
        }
        let mut tfdep = vec![None; rows.len()];
        let mut tfrows = Vec::new();
        for (i, r) in rows.iter().enumerate() {
            if tf_dependent(r.kind) {
                if let Some(x) = tfdep.get_mut(i) {
                    *x = Some(u8::try_from(tfrows.len()).map_err(|_| "tf rows")?);
                }
                tfrows.push(i);
            }
        }
        if tfrows.len() > NTF {
            return Err(format!("tf-dependent rows {} > {NTF}", tfrows.len()));
        }
        Ok(Self { rows: rows.to_vec(), tfdep, tfrows })
    }
}

/// The exit key of a distinct anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XKey {
    /// No own order: the sale to 15:10:00 and the square-off.
    None,
    /// An own buy-back anchored at this second of the day (the trigger level beside: the m_sf3_fill_at_level mutant reads it).
    Own(i64, i64),
    /// A HELD anchor in [15:08:00, 15:10:00): the sale's rest cancelled there (B2), the square-off.
    Held(i64),
}

/// The key of anchor `a` (u16 offset) under the cut rule.
#[must_use]
pub fn key_of(a: u16, level: i64, compat: bool) -> XKey {
    if a >= NEVER3 {
        return XKey::None;
    }
    let s = i64::from(a) + BASE_S;
    if s < CUT_S {
        XKey::Own(s, level)
    } else if s < SQ_S && !compat {
        // MUTANT m_sf3_cut_sends: an own order at a held anchor
        if cfg!(feature = "m_sf3_cut_sends") { XKey::Own(s, level) } else { XKey::Held(s) }
    } else {
        XKey::None
    }
}

/// One round trip of an exit key (v2.1's `trip` for None / Own; the held reading for Held).
///
/// # Errors
/// A crate refusal.
pub fn trip3(p: &mut Paths, tape: &Tape<'_>, day: i64, lot: u64, l: usize, qty: u64, key: XKey, dial: &Dial) -> Result<Trip, String> {
    match key {
        XKey::None => trip(p, tape, day, lot, l, qty, None, dial),
        XKey::Own(s, level) => {
            let mut r = trip(p, tape, day, lot, l, qty, Some(s), dial)?;
            if cfg!(feature = "m_sf3_fill_at_level") && r.bb_units > 0 {
                // MUTANT m_sf3_fill_at_level: the buy-back filled AT the trigger level (never a block's worst)
                let v = i64::try_from(r.bb_units).unwrap_or(0) * level;
                r.net += r.bb_value - v;
                r.bb_first_px = level;
                r.bb_value = v;
            }
            Ok(r)
        }
        XKey::Held(s) => {
            if cfg!(feature = "m_sf3_cut_norewalk") {
                // MUTANT m_sf3_cut_norewalk: the held anchor leaves the sale filling to 15:10:00 (v2's reading)
                return trip(p, tape, day, lot, l, qty, None, dial);
            }
            let r0 = held_trip(p, tape, day, l, qty, s, 0, lot, dial)?;
            // integrate:D F7: the held reading's square-off under every reading too, the dearest booked (PROT off: R0 alone)
            let Some(cx) = p.f7 else { return Ok(r0) };
            crate::trade::book_dearest(p, tape, day, lot, l, qty, dial, &cx, r0, |p2, rd| held_trip(p2, tape, day, l, qty, s, rd, lot, dial))
        }
    }
}

/// The held reading (B2): the sale cut at the held anchor `s` (its unfilled rest cancelled there), no own order, the square-off for every
/// filled unit (v2.1's square-off view and S15 valuation, charges as v2.1). integrate:D F7: `rd` = the square-off's reading (0 = R0).
#[allow(clippy::too_many_arguments)]
fn held_trip(p: &mut Paths, tape: &Tape<'_>, day: i64, l: usize, qty: u64, s: i64, rd: usize, lot: u64, dial: &Dial) -> Result<Trip, String> {
    let mut st = 0_u32;
    // integrate:D F7: the strict sale's straddling-block re-read on its own view (PROT off: the same view, byte for byte)
    let stape = p.f7.map_or(*tape, |cx| tape.with_strict(cx.sale_strict()));
    let sale = p.sale.cut_at(&stape, l, qty, at_sod(day, s), &mut st).map_err(|e| e.to_string())?;
    let mut r = Trip { qty, units: sale.units, sold_value: sale.value, e0: sale.e0, e1: sale.e1, p0: sale.first.map_or(0, |b| b.px), limit_in: p.limit_in, sale_known: sale.known_at, sale_blocks: sale.blocks_filled, sale_secs: sale.filled_secs, sale_one_price_secs: sale.one_price_secs, sale_past_limit: sale.past_limit, x: None, ..Trip::default() };
    if sale.units == 0 {
        return Ok(r);
    }
    // v3.2 addendum 10: the held reading's executed orders (the cut sale's blocks, the square-off's, Dhan's for never-closed units)
    let a10 = crate::sw::on(crate::sw::ORDERS_A10);
    if a10 {
        r.slice = p.slice;
        r.sale_orders = p.sale.sliced_orders(l, &sale, p.slice).map_err(|e| e.to_string())?;
    }
    let mut known = sale.known_at;
    if rd > 0 {
        crate::trade::sq_path_rd(p, tape, day, lot, dial, rd)?;
    }
    let sqp = if rd == 0 { &p.sq } else { p.sq_r.get(rd).and_then(Option::as_ref).ok_or("square-off path")? };
    let sq = sqp.view(l, sale.units, &mut st).map_err(|e| e.to_string())?;
    r.sq_units = sq.units;
    r.sq_value = sq.value;
    if a10 {
        r.sq_orders = sqp.sliced_orders(l, &sq, p.slice).map_err(|e| e.to_string())?;
    }
    if sq.units > 0 {
        known = known.max(sq.known_at);
        r.sq_known = sq.known_at;
    }
    r.unclosed = sale.units - sq.units;
    r.fill_after_1530_real = known > at_sod(day, crate::cal::CLOSE_S);
    if r.unclosed > 0 {
        // v3.1 item 3: the exit decision of a HELD anchor is the 15:10:00 square-off (no own order is sent at the held anchor)
        let (w, end, fb) = crate::trade::never_closed_value(tape, day, sale.e0, at_sod(day, SQ_S));
        r.worst = w;
        r.val_fallback = fb;
        known = known.max(end);
        if a10 {
            r.nc_orders = u32::try_from(common_fill::legs::freeze_slices(r.unclosed, p.slice)).unwrap_or(u32::MAX);
        }
    }
    r.after_1530 = known > at_sod(day, crate::cal::CLOSE_S);
    let unv = i64::try_from(i128::from(r.unclosed) * i128::from(r.worst)).map_err(|_| "unclosed value")?;
    r.unv = unv;
    r.buy_value = r.sq_value + unv;
    crate::trade::settle_charges(&mut r, day)?;
    r.known = known;
    Ok(r)
}

/// One event's stage A record (beside v2.1's `EvT`): its rows' offsets and its timeframe slots.
#[derive(Clone, Debug, Default)]
pub struct EvX {
    /// The main row of (l = 0, q = 0): row (l, q) = r0 + l x nq + q.
    pub r0: u32,
    /// The tf row of (slot 0, l 0, q 0): row (slot, l, q) = t0 + (slot x NL + l) x nq + q.
    pub t0: u32,
    pub nq: u8,
    /// The timeframe ids of the tf slots.
    pub tf: Vec<u8>,
    /// Per share level and lot count: the units the sale filled (the "none" key's view) and the units wanted (the liquidity report).
    pub fill: Vec<(u8, u8, u64, u64)>,
    /// v3.1 item 4: 1 + the offset of this contract-day's minute closes in [`Tab3::marks`] (0 = none: the MTM check off).
    pub mark1: u32,
}

impl EvX {
    /// The main row of (l, q).
    #[must_use]
    pub fn row(&self, l: usize, q: usize) -> usize {
        self.r0 as usize + l * usize::from(self.nq) + q
    }
    /// The tf row of (slot, l, q).
    #[must_use]
    pub fn trow(&self, slot: usize, l: usize, q: usize) -> usize {
        self.t0 as usize + (slot * NL + l) * usize::from(self.nq) + q
    }
    /// The slot of timeframe `tf` (None: the event is not in that timeframe).
    #[must_use]
    pub fn slot_of(&self, tf: u8) -> Option<usize> {
        self.tf.iter().position(|&x| x == tf)
    }
}

/// The stage A tables of one segment (or one chunk of events).
#[derive(Clone, Debug, Default)]
pub struct Tab3 {
    pub anc: Vec<[u16; NR]>,
    pub os: Vec<[u8; NR]>,
    pub obase: Vec<u32>,
    pub tanc: Vec<[u16; NTF]>,
    pub tos: Vec<[u8; NTF]>,
    pub outs: Vec<Out>,
    /// v3.1 item 4: the minute closes (forward filled, paisa; 385 per contract-day) of the contract-days priced here (the MTM check).
    pub marks: Vec<i32>,
    /// v3.1: the marks offset shift of the last `append` (the appended chunk's `mark1` values move by it).
    pub marks_shift: u32,
    /// v3.4 CAP20 (matrix X20): the suffix maxima of `marks` per contract-day (index b = the highest mark from minute b to the day's end),
    /// parallel to `marks` (the per-minute cap's O(1) loss bound); empty with the switch off.
    pub smax: Vec<i32>,
}

impl Tab3 {
    /// Appends another chunk's tables, shifting its offsets; returns (row shift, tf row shift).
    ///
    /// # Errors
    /// An offset beyond u32.
    pub fn append(&mut self, o: Self) -> Result<(u32, u32), String> {
        let (dr, dt, dout) = (u32::try_from(self.anc.len()).map_err(|_| "rows")?, u32::try_from(self.tanc.len()).map_err(|_| "tf rows")?, u32::try_from(self.outs.len()).map_err(|_| "outs")?);
        self.marks_shift = u32::try_from(self.marks.len()).map_err(|_| "marks")?;
        self.marks.extend(o.marks);
        self.smax.extend(o.smax);
        self.anc.extend(o.anc);
        self.os.extend(o.os);
        self.obase.extend(o.obase.into_iter().map(|b| b + dout));
        self.tanc.extend(o.tanc);
        self.tos.extend(o.tos);
        self.outs.extend(o.outs);
        Ok((dr, dt))
    }

    /// Bytes held (the memory gate).
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.anc.len() * (NR * 3 + 4) + self.tanc.len() * NTF * 3 + self.outs.len() * std::mem::size_of::<Out>() + self.marks.len() * 4 + self.smax.len() * 4
    }
}

/// Stage A counters (per rule: absent by reason, distance floor bound, fired before 15:08, held, never; anchors; steps).
#[derive(Clone, Debug, Default)]
pub struct Counts {
    pub absent: Vec<[u64; 7]>,
    pub floor: Vec<u64>,
    pub own: Vec<u64>,
    pub held: Vec<u64>,
    pub never: Vec<u64>,
    pub paths: u64,
    pub distinct: Vec<u32>,
    pub steps: Vec<u32>,
    /// P: the R pieces (filled sale blocks) per path.
    pub pieces: Vec<u32>,
    /// v3.4 CARRYSTAT (standing F21): per priced event (counted once), the blocks the sale order's carry walk read (the most of any share
    /// level), and per own buy-back path the blocks it recorded (empty with the switch off).
    pub carry: Vec<u32>,
    pub carry_b: Vec<u32>,
}

impl Counts {
    /// Empty counters for `n` rules.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self { absent: vec![[0; 7]; n], floor: vec![0; n], own: vec![0; n], held: vec![0; n], never: vec![0; n], ..Self::default() }
    }
    /// Adds another set.
    pub fn merge(&mut self, o: &Self) {
        for (a, b) in self.absent.iter_mut().zip(&o.absent) {
            for (x, y) in a.iter_mut().zip(b) {
                *x += y;
            }
        }
        for (a, b) in [(&mut self.floor, &o.floor), (&mut self.own, &o.own), (&mut self.held, &o.held), (&mut self.never, &o.never)] {
            for (x, y) in a.iter_mut().zip(b) {
                *x += y;
            }
        }
        self.paths += o.paths;
        self.distinct.extend_from_slice(&o.distinct);
        self.steps.extend_from_slice(&o.steps);
        self.pieces.extend_from_slice(&o.pieces);
        self.carry.extend_from_slice(&o.carry);
        self.carry_b.extend_from_slice(&o.carry_b);
    }
}

#[inline]
fn anchor_u16(t: Option<common_fill::path::Trigger>) -> (u16, i64) {
    t.map_or((NEVER3, 0), |tr| (u16::try_from((sod_of(tr.anchor) - BASE_S).clamp(0, i64::from(NEVER3) - 1)).unwrap_or(NEVER3 - 1), tr.level))
}

/// The rules of one share level at an event: per menu row Ok((rule, floor bound)) or Err(absence code); the tf-dependent rows per slot.
#[must_use]
pub fn level_rules(m: &MenuA, base: &Rd, tfw: &[(u8, i64, i64)], p0: i64) -> (Vec<Result<(XRule, bool), u8>>, Vec<Vec<Result<(XRule, bool), u8>>>) {
    let main: Vec<Result<(XRule, bool), u8>> = m.rows.iter().enumerate().map(|(i, r)| if m.tfdep.get(i).copied().flatten().is_some() { Err(u8::MAX) } else { rule_at(r, base, p0) }).collect();
    let per: Vec<Vec<Result<(XRule, bool), u8>>> = tfw.iter().map(|&(_, hw, rise)| { let rd = Rd { hwin: hw, rise, ..*base }; m.tfrows.iter().map(|&i| m.rows.get(i).map_or(Err(u8::MAX), |r| rule_at(r, &rd, p0))).collect() }).collect();
    (main, per)
}

/// Builds one event's stage A record, appending its rows and outs to `tab`. `tfw` = the event's timeframe slots (tf id, H_win, Rise);
/// `base` = its tf-independent readings. Returns v2.1's `EvT` (refusals, sizing, fills) and the v3 record.
///
/// # Errors
/// A crate refusal that is not a per-event refusal.
#[allow(clippy::too_many_arguments)]
pub fn price_event3(c: &Cube, ci: usize, t: u32, tape: Option<&Tape<'_>>, ti: Option<&TrailIndex>, var: &std::collections::BTreeMap<String, VarRow>, dial: &Dial, m: &MenuA, base: &Rd, tfw: &[(u8, i64, i64)], compat: bool, v2: bool, tab: &mut Tab3, cnt: &mut Counts, mx: Option<&crate::mtable::MarginAt>, mark1: u32) -> Result<(EvT, EvX), String> {
    let mut ev = crate::trade::size_event_mx(c, ci, t, var, v2, mx);
    let mut x = EvX { mark1, ..EvX::default() };
    if ev.refuse != Refuse::None || ev.qs.is_empty() {
        return Ok((ev, x));
    }
    let Some(k) = c.cons.get(ci) else { return Err("contract".to_owned()) };
    let (Some(tape), Some(ti)) = (tape, ti) else {
        ev.refuse = Refuse::BadTape;
        return Ok((ev, x));
    };
    let lot = u64::from(k.lot);
    let qmax = u64::from(ev.qs.last().copied().unwrap_or(0)) * lot;
    let day = c.day;
    // integrate:D F7 (data:F7 5.8): the contract-day's context (None with PROT off: build:D-high's record_base, byte for byte)
    let f7 = crate::f7::ctx_of(c, ci)?;
    let mut paths = match crate::trade::record_base_f7(tape, day, i64::from(t), lot, qmax, dial, f7) {
        Ok(p) => p,
        Err(BaseErr::NoReference) => {
            ev.refuse = Refuse::NoReference;
            return Ok((ev, x));
        }
        Err(BaseErr::SaleF7) => {
            crate::f7::note_sale_refused(c.day, k.sid, t, true);
            ev.refuse = Refuse::SaleF7;
            return Ok((ev, x));
        }
        Err(BaseErr::Crate(e)) => return Err(format!("{} {}: {e}", c.ymd, k.symbol)),
    };
    if paths.sale_f7 != 0 {
        crate::f7::note_sale_refused(c.day, k.sid, t, false);
    }
    // v3.2 addendum 10: the contract-day's freeze slice (every filling block x every slice is an executed order)
    if crate::sw::on(crate::sw::ORDERS_A10) {
        paths.slice = crate::freeze::slice_units(c, ci);
    }
    ev.limit_in = paths.limit_in;
    let nq = ev.qs.len();
    x.nq = u8::try_from(nq).map_err(|_| "lot counts")?;
    x.tf = tfw.iter().map(|w| w.0).collect();
    x.r0 = u32::try_from(tab.anc.len()).map_err(|_| "rows over u32")?;
    x.t0 = u32::try_from(tab.tanc.len()).map_err(|_| "tf rows over u32")?;
    tab.anc.resize(tab.anc.len() + NL * nq, [ABSENT; NR]);
    tab.os.resize(tab.os.len() + NL * nq, [0; NR]);
    tab.obase.resize(tab.obase.len() + NL * nq, 0);
    tab.tanc.resize(tab.tanc.len() + tfw.len() * NL * nq, [ABSENT; NTF]);
    tab.tos.resize(tab.tos.len() + tfw.len() * NL * nq, [0; NTF]);
    let to = if compat { at_sod(day, CUT_S) } else { at_sod(day, SQ_S) };
    let qs = ev.qs.clone();
    for l in 0..NL {
        let fl = paths.fills.get(l).cloned().unwrap_or_default();
        let Some(b0) = fl.first().copied() else { continue };
        if let (Some(f), Some(p)) = (ev.filled.get_mut(l), ev.p0.get_mut(l)) {
            *f = true;
            *p = b0.px;
        }
        // v2.1's in-block stop facts (printed beside)
        for (kk, &sp) in crate::trade::STOP_PCT.iter().enumerate() {
            if b0.block.high >= b0.px.saturating_add((b0.px * sp + 99) / 100)
                && let Some(mm) = ev.inblock.get_mut(l)
            {
                *mm |= 1 << kk;
            }
        }
        let (main, per) = level_rules(m, base, tfw, b0.px);
        for (i, r) in main.iter().enumerate() {
            match r {
                Err(code) if *code != u8::MAX => {
                    if let Some(a) = cnt.absent.get_mut(i).and_then(|a| a.get_mut(usize::from(*code))) {
                        *a += 1;
                    }
                }
                Ok((_, true)) => {
                    if let Some(f) = cnt.floor.get_mut(i) {
                        *f += 1;
                    }
                }
                _ => {}
            }
        }
        for slot_rules in &per {
            for (j, r) in slot_rules.iter().enumerate() {
                let i = m.tfrows.get(j).copied().unwrap_or(0);
                match r {
                    Err(code) if *code != u8::MAX => {
                        if let Some(a) = cnt.absent.get_mut(i).and_then(|a| a.get_mut(usize::from(*code))) {
                            *a += 1;
                        }
                    }
                    Ok((_, true)) => {
                        if let Some(f) = cnt.floor.get_mut(i) {
                            *f += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        let present: Vec<(usize, XRule)> = main.iter().enumerate().filter_map(|(i, r)| r.as_ref().ok().map(|(x, _)| (i, *x))).collect();
        let xs: Vec<XRule> = present.iter().map(|p| p.1).collect();
        for (q, &lots) in qs.iter().enumerate() {
            let qty = u64::from(lots) * lot;
            let fq = truncate(&fl, qty);
            let Some(f0) = fq.first() else { continue };
            let view = EntryView { e0: f0.block.at, e1: f0.block.known, fills: &fq };
            let mut steps = 0_u32;
            let trig = scan_fast(tape, ti, &view, to, false, &xs, &mut steps);
            cnt.steps.push(steps);
            cnt.pieces.push(u32::try_from(fq.len()).unwrap_or(u32::MAX));
            cnt.paths += 1;
            let ri = x.row(l, q);
            let mut keys: Vec<XKey> = vec![XKey::None];
            let mut row_anc = [ABSENT; NR];
            let mut row_key: Vec<(usize, XKey)> = Vec::with_capacity(present.len());
            for ((i, _), tr) in present.iter().zip(trig.iter()) {
                let (a, lvl) = anchor_u16(*tr);
                if let Some(s) = row_anc.get_mut(*i) {
                    *s = a;
                }
                let kk = key_of(a, lvl, compat);
                match kk {
                    XKey::None => {
                        if a >= NEVER3 {
                            if let Some(n) = cnt.never.get_mut(*i) {
                                *n += 1;
                            }
                        }
                    }
                    XKey::Own(..) => {
                        if let Some(n) = cnt.own.get_mut(*i) {
                            *n += 1;
                        }
                    }
                    XKey::Held(_) => {
                        if let Some(n) = cnt.held.get_mut(*i) {
                            *n += 1;
                        }
                    }
                }
                row_key.push((*i, kk));
            }
            // the tf-dependent rules per slot
            let mut tf_keys: Vec<Vec<(usize, u16, XKey)>> = Vec::with_capacity(per.len());
            for slot_rules in &per {
                let pr: Vec<(usize, XRule)> = slot_rules.iter().enumerate().filter_map(|(j, r)| r.as_ref().ok().map(|(x2, _)| (j, *x2))).collect();
                let xs2: Vec<XRule> = pr.iter().map(|p| p.1).collect();
                let mut s2 = 0_u32;
                let tr2 = scan_fast(tape, ti, &view, to, false, &xs2, &mut s2);
                let mut v = Vec::with_capacity(pr.len());
                for ((j, _), tr) in pr.iter().zip(tr2.iter()) {
                    let (a, lvl) = anchor_u16(*tr);
                    v.push((*j, a, key_of(a, lvl, compat)));
                }
                tf_keys.push(v);
            }
            // distinct keys (by anchor second: the level beside belongs to the first rule reaching that key)
            let same = |a: &XKey, b: &XKey| match (a, b) {
                (XKey::None, XKey::None) => true,
                (XKey::Own(x, _), XKey::Own(y, _)) | (XKey::Held(x), XKey::Held(y)) => x == y,
                _ => false,
            };
            for (_, kk) in &row_key {
                if !keys.iter().any(|k2| same(k2, kk)) {
                    keys.push(*kk);
                }
            }
            for v in &tf_keys {
                for (_, _, kk) in v {
                    if !keys.iter().any(|k2| same(k2, kk)) {
                        keys.push(*kk);
                    }
                }
            }
            if keys.len() > 255 {
                return Err("more than 255 distinct exit keys".to_owned());
            }
            cnt.distinct.push(u32::try_from(keys.len()).unwrap_or(u32::MAX));
            let ob = u32::try_from(tab.outs.len()).map_err(|_| "outs over u32")?;
            for kk in &keys {
                let r = trip3(&mut paths, tape, day, lot, l, qty, *kk, dial)?;
                let mut o = crate::trade::out_of31(&r, day)?;
                if matches!(kk, XKey::Held(_)) && r.units > 0 {
                    o.flag |= FLAG_CUT_HELD;
                }
                if matches!(kk, XKey::None) {
                    x.fill.push((u8::try_from(l).unwrap_or(0), u8::try_from(q).unwrap_or(0), r.units, qty));
                }
                tab.outs.push(o);
            }
            let slot_of = |kk: &XKey| u8::try_from(keys.iter().position(|k2| same(k2, kk)).unwrap_or(0)).unwrap_or(0);
            if let (Some(a), Some(o), Some(b)) = (tab.anc.get_mut(ri), tab.os.get_mut(ri), tab.obase.get_mut(ri)) {
                *a = row_anc;
                for (i, kk) in &row_key {
                    if let Some(s) = o.get_mut(*i) {
                        *s = slot_of(kk);
                    }
                }
                *b = ob;
            }
            for (slot, v) in tf_keys.iter().enumerate() {
                let tri = x.trow(slot, l, q);
                if let (Some(a), Some(o)) = (tab.tanc.get_mut(tri), tab.tos.get_mut(tri)) {
                    for (j, an, kk) in v {
                        if let (Some(s), Some(t2)) = (a.get_mut(*j), o.get_mut(*j)) {
                            *s = *an;
                            *t2 = slot_of(kk);
                        }
                    }
                }
            }
        }
    }
    // v3.4 CARRYSTAT (standing F21, a measurement): the sale walk's blocks and each own buy-back path's recorded blocks (the stage A
    // counters: once per event, whatever the batch partition or a resume)
    if crate::fix34::on(crate::fix34::CARRYSTAT) {
        cnt.carry.push(paths.walk_blocks);
        for (_, fp, _) in &paths.buys {
            cnt.carry_b.push(u32::try_from((0..NL).map(|l| fp.blocks(l)).max().unwrap_or(0)).unwrap_or(u32::MAX));
        }
    }
    Ok((ev, x))
}

/// The first-hit decision of one setting on one path from its rules' anchors (tie order = the rules' order; strict `<` keeps the first):
/// (winner position in `rules`, anchor) or None when nothing fires. Shared by the kernel and the detail.
#[inline]
#[must_use]
pub fn first_hit(anchors: &[u16]) -> Option<(usize, u16)> {
    let mut best = NEVER3;
    let mut bi = usize::MAX;
    for (i, &a) in anchors.iter().enumerate() {
        // MUTANT m_sf3_tie_tp_first: on a tie a LATER rule (TP after SL) takes the label
        let better = if cfg!(feature = "m_sf3_tie_tp_first") { a <= best && a < NEVER3 } else { a < best };
        if better {
            best = a;
            bi = i;
        }
    }
    (bi != usize::MAX).then_some((bi, best))
}

/// One trade recomputed from scratch for a setting (proof P5 and the trade rows): the sale recorded up to `qty`, every rule of the setting
/// evaluated by THE LAW (crate v2.2 `scan_law`, not the fast path), the first hit with the tie order, its key priced by `trip3`. Returns
/// (trip, the winner's position in `rules` (None: no own exit), its anchor, the key, its trigger second and level).
///
/// # Errors
/// A crate refusal.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn trade3_once(tape: &Tape<'_>, day: i64, t: u32, lot: u64, qty: u64, l: usize, rules: &[Result<(XRule, bool), u8>], compat: bool, dial: &Dial) -> Result<Option<(Trip, Option<usize>, u16, XKey, i64, i64)>, String> {
    trade3_once_s(tape, day, t, lot, qty, l, rules, compat, dial, 0, None)
}

/// v3.2: [`trade3_once`] with the contract-day's freeze slice (addendum 10: the recomputed trade counts the same executed orders as stage A).
///
/// # Errors
/// A crate refusal.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn trade3_once_s(tape: &Tape<'_>, day: i64, t: u32, lot: u64, qty: u64, l: usize, rules: &[Result<(XRule, bool), u8>], compat: bool, dial: &Dial, slice: u64, f7: Option<crate::f7::Ctx>) -> Result<Option<(Trip, Option<usize>, u16, XKey, i64, i64)>, String> {
    // integrate:D F7: the same contract-day context as stage A (None with PROT off: record_base, byte for byte)
    let mut paths = match crate::trade::record_base_f7(tape, day, i64::from(t), lot, qty, dial, f7) {
        Ok(p) => p,
        Err(BaseErr::NoReference) => return Ok(None),
        Err(BaseErr::SaleF7) => return Ok(None),
        Err(BaseErr::Crate(e)) => return Err(e),
    };
    paths.slice = slice;
    let fl = paths.fills.get(l).cloned().unwrap_or_default();
    if fl.is_empty() {
        return Ok(None);
    }
    let fq = truncate(&fl, qty);
    let Some(f0) = fq.first() else { return Ok(None) };
    let view = EntryView { e0: f0.block.at, e1: f0.block.known, fills: &fq };
    let to = if compat { at_sod(day, CUT_S) } else { at_sod(day, SQ_S) };
    let xs: Vec<XRule> = rules.iter().filter_map(|r| r.as_ref().ok().map(|x| x.0)).collect();
    let law = scan_law(tape, &view, to, false, &xs);
    let mut anc: Vec<u16> = Vec::with_capacity(rules.len());
    let mut lv: Vec<(i64, i64)> = Vec::with_capacity(rules.len());
    let mut it = law.iter();
    for r in rules {
        if r.is_ok() {
            let tr = it.next().copied().flatten();
            let (a, level) = anchor_u16(tr);
            anc.push(a);
            lv.push((tr.map_or(0, |x| sod_of(x.sec)), level));
        } else {
            anc.push(ABSENT);
            lv.push((0, 0));
        }
    }
    let hit = first_hit(&anc);
    let (key, pos, a, sec, level) = match hit {
        Some((i, a)) => {
            let (s, lvl) = lv.get(i).copied().unwrap_or((0, 0));
            (key_of(a, lvl, compat), Some(i), a, s, lvl)
        }
        None => (XKey::None, None, NEVER3, 0, 0),
    };
    let r = trip3(&mut paths, tape, day, lot, l, qty, key, dial)?;
    Ok(Some((r, pos, a, key, sec, level)))
}

/// The absence words of a code.
#[must_use]
pub fn absent_word(code: u8) -> &'static str {
    ABSENT_WHY.get(usize::from(code)).copied().unwrap_or("?")
}
