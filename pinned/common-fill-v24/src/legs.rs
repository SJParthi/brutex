//! Hedged option pairs (design 4.2 `legs`, S18, D14): two legs on their own contracts' tapes, ONE anchor (one lattice), walked block by
//! block together. The LEAD leg fills first in each block; the FOLLOW leg is never above the lead's cumulative units at the end of the
//! same block. Opening a hedged sale: lead = the hedge (bought), follow = the sold leg. Closing is reversed: lead = the sold leg (bought
//! back), follow = the hedge (sold only up to what was bought back). Each leg is priced at ITS OWN block's worst (a buy its HIGH, a sale its
//! LOW), with its own lot as the unit and its own protection limit; the take is the walk's take ([`crate::walk`]). A leg left short of the
//! other at the pair's stop is the host's to unwind (a decided exit, counted). The sold leg's money is margin: the ACCOUNT module's (D26).

use crate::gaps::cap_units;
use crate::std70::{BlockFill, LevelFill, Order, Why};
use crate::tape::{Row, Tape};
use crate::{BLOCK_US, Dial, FillErr, LevelMask, MAXL, Side, US_S, Units, Us};

/// The pair recorder: (GLOBAL level, leg 0 = lead / 1 = follow, row).
pub type PairRecorder<'r> = &'r mut dyn FnMut(usize, u8, &BlockFill);

/// One leg's take in one block at one level (the walk's take; `bound` caps it for the follow leg's pairing).
#[allow(clippy::too_many_arguments, reason = "one leg's take")]
fn leg_take(f: &mut LevelFill, w: &mut Units, b: &crate::tape::Block, k: u32, px: crate::Paisa, bp: u16, unit: Units, bound: Option<Units>) -> Result<(Units, Why), FillErr> {
    let capu = cap_units(b.vol, bp, unit)?;
    if capu == 0 {
        f.zero_take_blocks = f.zero_take_blocks.saturating_add(1);
    }
    let mut take = capu.min(*w);
    let mut why = Why::ZeroTake;
    if let Some(bd) = bound
        && bd < take
    {
        take = bd / unit * unit;
        why = Why::Paired;
    }
    if take == 0 {
        return Ok((0, if capu == 0 { Why::ZeroTake } else { why }));
    }
    let units = f.units.checked_add(take).ok_or(FillErr::Overflow)?;
    let add = i64::try_from(take).map_err(|_| FillErr::Overflow)?.checked_mul(px).ok_or(FillErr::Overflow)?;
    let value = f.value.checked_add(add).ok_or(FillErr::Overflow)?;
    let bf = BlockFill { k, block: *b, units: take, px, cum: units, why: Why::Filled };
    if f.blocks_filled == 0 {
        f.first = Some(bf);
        f.e0 = b.at;
        f.e1 = b.known;
    }
    f.blocks_filled = f.blocks_filled.checked_add(1).ok_or(FillErr::Overflow)?;
    f.last = Some(bf);
    f.known_at = f.known_at.max(b.known);
    f.filled_secs = f.filled_secs.saturating_add(u32::from(b.n_secs));
    f.one_price_secs = f.one_price_secs.saturating_add(u32::from(b.one_price));
    f.units = units;
    f.value = value;
    *w -= take;
    Ok((take, Why::Filled))
}

/// S18 + D14: the pair's blocks for every level in `mask` in ONE pass: per block the lead leg first, then the follow leg bounded by the
/// lead's cumulative units at the end of the block. `want[l]` is the pair's units per level (both legs; out: the FOLLOW leg's rest, the
/// pair being complete only when the follow leg is); `out_lead` / `out_follow` the legs' accumulated summaries. Both orders must share
/// the anchor and the unit.
///
/// # Errors
/// [`FillErr::NotWholeSecond`] for an anchor off whole seconds; [`FillErr::PairMismatch`] for legs of different anchors or units (CHANGE
/// C10: or of different block lengths, [`crate::Tape::with_block`]);
/// [`FillErr::DialInvalid`] for a mask outside the dial; [`FillErr::LotMismatch`] for a want not a whole number of units;
/// [`FillErr::Overflow`] when a sum leaves its type.
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "two legs at every level in one pass")]
pub fn walk_pair<R: Row>(lead: (&Tape<'_, R>, &Order), follow: (&Tape<'_, R>, &Order), dial: &Dial, mask: LevelMask, want: &mut [Units; MAXL], out_lead: &mut [LevelFill; MAXL], out_follow: &mut [LevelFill; MAXL], mut rec: Option<PairRecorder<'_>>) -> Result<(), FillErr> {
    let (ta, oa) = lead;
    let (tb, ob) = follow;
    // CHANGE FA1: the paired legs read recorded volumes: refused under a capacity reading.
    if ta.cap_mode() != crate::tape::CapMode::Row || tb.cap_mode() != crate::tape::CapMode::Row {
        return Err(FillErr::CapReadNotServed);
    }
    // CHANGE K1: the paired legs read per-block capacities: refused on a carrying view.
    if ta.carry() || tb.carry() {
        return Err(FillErr::CarryNotServed);
    }
    // CHANGE C10 (crate v2.1): the pair walks ONE lattice of the legs' common block length (C9 refused any length but 2 s); legs on two
    // block lengths share no lattice and are refused by name.
    let blk = ta.block_us();
    if tb.block_us() != blk || blk <= 0 {
        return Err(FillErr::PairMismatch);
    }
    if oa.anchor.rem_euclid(US_S) != 0 {
        return Err(FillErr::NotWholeSecond);
    }
    if oa.anchor != ob.anchor || oa.unit.max(1) != ob.unit.max(1) {
        return Err(FillErr::PairMismatch);
    }
    let nl = dial.len();
    if (nl..MAXL).any(|l| mask.has(l)) {
        return Err(FillErr::DialInvalid);
    }
    let unit = oa.unit.max(1);
    if (0..nl).any(|l| mask.has(l) && want.get(l).is_some_and(|&w| w % unit != 0)) {
        return Err(FillErr::LotMismatch);
    }
    let mut wa = *want;
    let mut wb = *want;
    let stop = oa.stop.max(ob.stop);
    let mut at = oa.anchor;
    let (mut ia, mut ib) = (ta.first_at(at), tb.first_at(at));
    let live = |wa: &[Units; MAXL], wb: &[Units; MAXL], l: usize| mask.has(l) && (wa.get(l).is_some_and(|&w| w > 0) || wb.get(l).is_some_and(|&w| w > 0));
    while at < stop {
        if !(0..nl).any(|l| live(&wa, &wb, l)) {
            break;
        }
        let na = ta.rows().get(ia).map(Row::ts).filter(|&t| t < oa.stop);
        let nb = tb.rows().get(ib).map(Row::ts).filter(|&t| t < ob.stop);
        let Some(next) = [na, nb].into_iter().flatten().min() else { break };
        let end = at.checked_add(blk).ok_or(FillErr::Overflow)?;
        if next >= end {
            let k = (next - at) / blk;
            at = at.checked_add(k.checked_mul(blk).ok_or(FillErr::Overflow)?).ok_or(FillErr::Overflow)?;
            let kk = u32::try_from(k).unwrap_or(u32::MAX);
            for l in 0..nl {
                if live(&wa, &wb, l) {
                    for f in [out_lead.get_mut(l), out_follow.get_mut(l)].into_iter().flatten() {
                        f.no_trade = f.no_trade.saturating_add(kk);
                        f.steps.jumps = f.steps.jumps.saturating_add(1);
                    }
                }
            }
            continue;
        }
        let (ba, nia) = if at < oa.stop { ta.block_from(ia, at, oa.stop)? } else { (crate::tape::Block { at, to: at, ..crate::tape::Block::default() }, ia) };
        let (bb, nib) = if at < ob.stop { tb.block_from(ib, at, ob.stop)? } else { (crate::tape::Block { at, to: at, ..crate::tape::Block::default() }, ib) };
        ia = nia;
        ib = nib;
        let k = u32::try_from((at - oa.anchor) / blk).map_err(|_| FillErr::Overflow)?;
        let pxa = ba.px(oa.side);
        let pxb = bb.px(ob.side);
        let past = |o: &Order, px: crate::Paisa| o.limit.is_some_and(|lim| match o.side {
            Side::Buy => px > lim,
            Side::Sell => px < lim,
        });
        let (pa, pb) = (past(oa, pxa), past(ob, pxb));
        for l in 0..nl {
            if !mask.has(l) {
                continue;
            }
            let Some(bp) = dial.bp(l) else { continue };
            // THE LEAD LEG.
            if let (Some(f), Some(w)) = (out_lead.get_mut(l), wa.get_mut(l))
                && *w > 0
                && ba.n > 0
            {
                f.steps.blocks = f.steps.blocks.saturating_add(1);
                if pa {
                    f.past_limit = f.past_limit.saturating_add(1);
                    if let Some(r) = rec.as_mut() {
                        r(l, 0, &BlockFill { k, block: ba, units: 0, px: pxa, cum: f.units, why: Why::PastLimit });
                    }
                } else {
                    let (t, why) = leg_take(f, w, &ba, k, pxa, bp, unit, None)?;
                    if let Some(r) = rec.as_mut() {
                        r(l, 0, &BlockFill { k, block: ba, units: t, px: pxa, cum: f.units, why });
                    }
                }
            }
            // THE FOLLOW LEG: never above the lead's cumulative units at the end of this block.
            let lead_cum = out_lead.get(l).map_or(0, |f| f.units);
            if let (Some(f), Some(w)) = (out_follow.get_mut(l), wb.get_mut(l))
                && *w > 0
                && bb.n > 0
            {
                f.steps.blocks = f.steps.blocks.saturating_add(1);
                if pb {
                    f.past_limit = f.past_limit.saturating_add(1);
                    if let Some(r) = rec.as_mut() {
                        r(l, 1, &BlockFill { k, block: bb, units: 0, px: pxb, cum: f.units, why: Why::PastLimit });
                    }
                } else {
                    let bound = lead_cum.saturating_sub(f.units);
                    let (t, why) = leg_take(f, w, &bb, k, pxb, bp, unit, Some(bound))?;
                    if let Some(r) = rec.as_mut() {
                        r(l, 1, &BlockFill { k, block: bb, units: t, px: pxb, cum: f.units, why });
                    }
                }
            }
        }
        at = end;
    }
    for l in 0..nl {
        if mask.has(l) {
            if let (Some(f), Some(&w)) = (out_lead.get_mut(l), wa.get(l)) {
                f.rest_at_stop = w;
            }
            if let (Some(f), Some(&w)) = (out_follow.get_mut(l), wb.get(l)) {
                f.rest_at_stop = w;
            }
        }
    }
    // The pair is complete only when the follow leg is: want out = the follow leg's rest per level.
    *want = wb;
    Ok(())
}

/// The freeze-quantity slices of an order of `units` (S18: slices sent together share each block's capacity; they change no fill and are
/// counted for charges): ceil(units / freeze), 0 for no units.
#[must_use]
pub const fn freeze_slices(units: Units, freeze: Units) -> Units {
    if units == 0 {
        0
    } else if freeze == 0 {
        1
    } else {
        units.div_ceil(freeze)
    }
}

/// CHANGE C10: the first lattice instant of `anchor` at or after `t` on the block length `blk` (microseconds; the tape's,
/// [`crate::Tape::block_us`]): `anchor` itself when `t` is not after it. None for a length not above 0 or a result outside i64 (refused,
/// never wrapped). At [`BLOCK_US`] it equals [`lattice_at_or_after`].
#[must_use]
pub const fn lattice_at_or_after_blk(anchor: Us, t: Us, blk: Us) -> Option<Us> {
    if blk <= 0 {
        return None;
    }
    if t <= anchor {
        return Some(anchor);
    }
    let Some(d) = t.checked_sub(anchor) else { return None };
    let Some(d1) = d.checked_add(blk - 1) else { return None };
    let Some(off) = (d1 / blk).checked_mul(blk) else { return None };
    anchor.checked_add(off)
}

/// The instant `t` as a whole-second lattice instant of `anchor` at or after it.
#[must_use]
pub const fn lattice_at_or_after(anchor: Us, t: Us) -> Us {
    if t <= anchor {
        return anchor;
    }
    let k = (t - anchor + BLOCK_US - 1) / BLOCK_US;
    anchor + k * BLOCK_US
}
