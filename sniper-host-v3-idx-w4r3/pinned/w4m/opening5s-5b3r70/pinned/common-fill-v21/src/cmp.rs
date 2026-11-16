//! The ONE old-rule engine kept in the crate (design 5.2): the per-second REQ 58b CORE, `req58b-core@cf2`. It is the red build's reading
//! (under `--features red` the walk family carries it: 1-second steps, the price below, 5-paisa tick, no decision reference) and a labelled
//! comparison beside the headline. It is NOT the lanes' full old laws (AFR-7's rest at the limit, AFR-10's remainder, FU9's minute-worst
//! square-off, the 60-s forced sale): each lane's old-rule column comes from its own sha-frozen old code (design 5.2, D20).
//!
//! The core: from the anchor T, every PRINTED second s before the stop, in time order; a buy fills at the highest of s's HIGH, the next
//! printed second's HIGH (s + 1 or the next printed second standing in) and the decision reference, plus one tick; a sale at the lowest of
//! their LOWs less one tick (never below one paisa); each dial level takes floor(level x the SECOND's volume) in whole units; Dhan's
//! protection and the HARD cap skip a second; REQ 70a's payable cut as the standard's; a fill is known at its second's end.

use crate::gaps::cap_units;
use crate::std70::{BlockFill, LevelFill, Order, Recorder, Why};
use crate::tape::{Block, Row, Tape, valid};
use crate::{Dial, FillErr, LevelMask, MAXL, Paisa, Side, US_S, Units};

/// The name of this comparison.
pub const LAW_REQ58B_CORE: &str = "req58b-core@cf2";

/// What the old rule needs and the standard does not (design 5.2 `CmpIn`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CmpIn {
    /// The decision reference price (REQ 58a: the decision second's HIGH for a buy, LOW for a sale), when the host gives it.
    pub ref_px: Option<Paisa>,
    /// The tick added against the trader (the dated tick; the red build's fixed 5 paise).
    pub tick: Paisa,
}

impl CmpIn {
    /// The red build's reading: no decision reference, the fixed 5-paisa tick.
    pub const RED: Self = Self { ref_px: None, tick: 5 };
}

/// The per-second core for every level in `mask` in one pass (the arguments as [`crate::walk`]'s).
///
/// # Errors
/// [`FillErr::NotWholeSecond`], [`FillErr::DialInvalid`], [`FillErr::LotMismatch`], [`FillErr::Overflow`] as [`crate::walk`].
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one order at every level in one pass")]
pub fn walk_seconds<R: Row>(tape: &Tape<'_, R>, o: &Order, dial: &Dial, mask: LevelMask, want: &mut [Units; MAXL], money: Option<&[Option<Paisa>; MAXL]>, cap: Option<Paisa>, out: &mut [LevelFill; MAXL], mut rec: Option<Recorder<'_>>, cin: &CmpIn) -> Result<(), FillErr> {
    if o.anchor.rem_euclid(US_S) != 0 {
        return Err(FillErr::NotWholeSecond);
    }
    let nl = dial.len();
    if (nl..MAXL).any(|l| mask.has(l)) {
        return Err(FillErr::DialInvalid);
    }
    let unit = o.unit.max(1);
    if (0..nl).any(|l| mask.has(l) && want.get(l).is_some_and(|&w| w % unit != 0)) {
        return Err(FillErr::LotMismatch);
    }
    let rows = tape.rows();
    let mut i = tape.first_at(o.anchor);
    let live = |want: &[Units; MAXL], l: usize| mask.has(l) && want.get(l).is_some_and(|&w| w > 0);
    while let Some(s) = rows.get(i) {
        if !(0..nl).any(|l| live(want, l)) {
            break;
        }
        let ts = s.ts();
        if ts >= o.stop {
            break;
        }
        // The second's rows (a repeated stamp merges into it).
        let mut j = i;
        let (mut hi, mut lo, mut vol, mut n) = (i64::MIN, i64::MAX, 0_u64, 0_u32);
        while let Some(r) = rows.get(j) {
            if r.ts() != ts {
                break;
            }
            if valid(r) {
                hi = hi.max(r.high());
                lo = lo.min(r.low());
                vol = vol.checked_add(r.vol()).ok_or(FillErr::Overflow)?;
                n += 1;
            }
            j += 1;
        }
        i = j;
        if n == 0 {
            continue;
        }
        let next = rows.get(j..).and_then(|w| w.iter().find(|r| valid(*r)));
        let mut px = match o.side {
            Side::Buy => hi.max(next.map_or(i64::MIN, Row::high)),
            Side::Sell => lo.min(next.map_or(i64::MAX, Row::low)),
        };
        if let Some(r) = cin.ref_px {
            px = match o.side {
                Side::Buy => px.max(r),
                Side::Sell => px.min(r),
            };
        }
        px = match o.side {
            Side::Buy => px.saturating_add(cin.tick),
            Side::Sell => px.saturating_sub(cin.tick).max(1),
        };
        let past = o.limit.is_some_and(|lim| match o.side {
            Side::Buy => px > lim,
            Side::Sell => px < lim,
        });
        let capped = !past && cap.is_some_and(|c| px > c);
        let to = ts.checked_add(US_S).ok_or(FillErr::Overflow)?;
        let b = Block { at: ts, to, clipped: false, n, n_secs: 1, one_price: u8::from(hi == lo), high: hi, low: lo, high_pre: hi, low_pre: lo, vol, known: to };
        let k = u32::try_from((ts - o.anchor) / US_S).map_err(|_| FillErr::Overflow)?;
        for l in 0..nl {
            if !live(want, l) {
                continue;
            }
            let (Some(f), Some(w), Some(bp)) = (out.get_mut(l), want.get_mut(l), dial.bp(l)) else { continue };
            f.steps.blocks = f.steps.blocks.saturating_add(1);
            if past || capped {
                if past {
                    f.past_limit = f.past_limit.saturating_add(1);
                } else {
                    f.past_cap = f.past_cap.saturating_add(1);
                }
                if let Some(r) = rec.as_mut() {
                    r(l, &BlockFill { k, block: b, units: 0, px, cum: f.units, why: if past { Why::PastLimit } else { Why::PastCap } });
                }
                continue;
            }
            let capu = cap_units(vol, bp, unit)?;
            if capu == 0 {
                f.zero_take_blocks = f.zero_take_blocks.saturating_add(1);
            }
            let mut take = capu.min(*w);
            let mut cut = false;
            if let Some(p) = money.and_then(|m| m.get(l).copied().flatten())
                && take > 0
            {
                let room = p.checked_sub(f.value).ok_or(FillErr::Overflow)?.max(0);
                let fit = Units::try_from(room / px.max(1)).map_err(|_| FillErr::Overflow)? / unit * unit;
                if fit < take {
                    take = fit;
                    cut = true;
                }
            }
            if take > 0 {
                let units = f.units.checked_add(take).ok_or(FillErr::Overflow)?;
                let add = i64::try_from(take).map_err(|_| FillErr::Overflow)?.checked_mul(px).ok_or(FillErr::Overflow)?;
                f.value = f.value.checked_add(add).ok_or(FillErr::Overflow)?;
                let bf = BlockFill { k, block: b, units: take, px, cum: units, why: Why::Filled };
                if f.blocks_filled == 0 {
                    f.first = Some(bf);
                    f.e0 = ts;
                    f.e1 = to;
                }
                f.blocks_filled = f.blocks_filled.saturating_add(1);
                f.last = Some(bf);
                f.known_at = f.known_at.max(to);
                f.filled_secs = f.filled_secs.saturating_add(1);
                f.one_price_secs = f.one_price_secs.saturating_add(u32::from(hi == lo));
                f.units = units;
                *w -= take;
                if let Some(r) = rec.as_mut() {
                    r(l, &bf);
                }
            } else if let Some(r) = rec.as_mut() {
                r(l, &BlockFill { k, block: b, units: 0, px, cum: f.units, why: if cut { Why::Unpaid } else { Why::ZeroTake } });
            }
            if cut {
                f.unpaid = f.unpaid.checked_add(*w).ok_or(FillErr::Overflow)?;
                *w = 0;
            }
        }
    }
    for (l, f) in out.iter_mut().enumerate().take(nl) {
        if mask.has(l) {
            f.rest_at_stop = want.get(l).copied().unwrap_or(0);
        }
    }
    Ok(())
}
