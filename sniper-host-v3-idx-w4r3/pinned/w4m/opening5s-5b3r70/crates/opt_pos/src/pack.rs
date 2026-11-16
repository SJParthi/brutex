//! S5 (wave 4; SNIPER-IDXOPT-PLAN-v3.1 N4 / N9, s6.3): the index option PACK's data parts, read only on every input.
//!
//! * **DP-IDXITM**: the A2 / s23a ITM-fit contract of every second (every instant t a 1-s decision candle can end at: 09:15:01 ..
//!   15:29:59 for the current expiry; on a true expiry day also 15:00:00 .. 15:29:59 for the NEXT expiry, the roll window of the
//!   sequential t2), chosen by THE one rule [`crate::chain::itm_fit`] (the function `itm-pick` and G-EQ3 use) on the store's 1-s
//!   bars, each candidate's lot C17 / D9 kept INCREMENTALLY ([`LotState`]: the divisibility of every print before t), as
//!   run-length rows: a run holds while the pick (or its refusal) and its depth, skips, lot and one-order maximum stay the same.
//! * **DP-IDXLEAD**: the busiest contract and the runner-up of every second (the comparison column; the ordering of
//!   [`crate::chain::leader`]: units traded in [09:15:00, t) desc, nearest the money, lower strike), over the same candidate set
//!   (the listed strikes of the current expiry in their series' final folder), as runs.
//! * **DP-IDXNEED**: the exact contract-day list the rules can touch: every DP-IDXITM pick of the day (intraday), each picked
//!   contract's LATER sessions to its expiry (positional), the roll window's next-expiry picks, every DP-IDXLEAD leader and
//!   runner-up.
//! * **F8, the one-order gate**: every pick's Rs 5,00,000 closest quantity is at or below that day's one-order maximum (asserted
//!   on every second; a failure is counted and named).
//!
//! Runs are written as TSV and compressed with zstd (ruzstd `Fastest`, about zstd level 1; plan s6.3 "compact"), one frame per
//! index file, so the host decodes them with the decoder the store reader already links.

use common_fill::limits::size_units_closest;

use crate::chain::{self, Cand, ItmCand};
use crate::ladder::LadderRow;
use crate::secs::Sec;

/// The A2 target (A1: Rs 5,00,000 a trade = N 4).
pub const TARGET: i64 = 50_000_000;

/// C17 / D9 kept incrementally for one candidate: the stated UDiFF lot, else the lot candidates still dividing EVERY print seen
/// so far (a bit per candidate), so the lot at t equals [`chain::lot_at`] over the prints before t (test `lot_state_equals_lot_at`).
#[derive(Clone, Debug)]
pub struct LotState {
    stated: Option<i64>,
    cands: Vec<i64>,
    divides: Vec<bool>,
    prints: u64,
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

impl LotState {
    /// A new state for a ladder row's lot data.
    #[must_use]
    pub fn new(row: &LadderRow) -> Self {
        Self { stated: row.lot, cands: row.cands.clone(), divides: vec![true; row.cands.len()], prints: 0 }
    }

    /// One more print (a second with volume > 0) before t.
    pub fn push(&mut self, vol: u64) {
        self.prints += 1;
        let v = i64::try_from(vol).unwrap_or(1);
        for (d, &x) in self.divides.iter_mut().zip(&self.cands) {
            if x <= 0 || v % x != 0 {
                *d = false;
            }
        }
    }

    /// The lot at t (0: unknown at t, never picked), as [`chain::lot_at`].
    #[must_use]
    pub fn lot(&self) -> i64 {
        if let Some(l) = self.stated {
            return l;
        }
        if self.cands.len() == 1 && self.prints == 0 {
            return self.cands.first().copied().unwrap_or(0);
        }
        let div: Vec<i64> = self.cands.iter().zip(&self.divides).filter(|(x, d)| **d && **x > 0).map(|(x, _)| *x).collect();
        let Some(&largest) = div.iter().max() else { return 0 };
        let mut l = 1i64;
        for x in &div {
            l = l / gcd(l, *x) * x;
        }
        if l == largest { largest } else { l }
    }
}

/// One candidate contract of a (session, side, expiry) with its rows and incremental state.
#[derive(Clone, Debug)]
pub struct Live {
    /// Strike, paisa.
    pub strike: i64,
    /// Its store folder name.
    pub folder: String,
    /// The contract-day's 1-s rows (valid prints, derivatives halts cut).
    pub rows: Vec<Sec>,
    /// Rows with `ts < t`.
    pub cursor: usize,
    /// Units traded in [09:15:00, t).
    pub units: u64,
    /// C17 / D9 at t.
    pub lot: LotState,
}

impl Live {
    /// Advances to instant `t` (folding every row with `ts < t`).
    pub fn advance(&mut self, t: i64) {
        while let Some(r) = self.rows.get(self.cursor) {
            if r.ts >= t {
                break;
            }
            self.units += r.vol;
            if r.vol > 0 {
                self.lot.push(r.vol);
            }
            self.cursor += 1;
        }
    }
}

/// The pick of one second, as stored in a run: (folder or the refusal word, strike, depth, skipped no-trade, skipped big, lot,
/// one-order maximum, the Rs 5 L quantity).
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Pick {
    /// The contract folder, or `no_itm_fit` / `no_spot` / `no_freeze_row`.
    pub what: String,
    /// Strike (0 when refused).
    pub strike: i64,
    /// ITM depth.
    pub depth: u32,
    /// Strikes skipped for no trade in [09:15:00, t).
    pub skip_nt: u32,
    /// Strikes skipped as too big for one order (or lot unknown).
    pub skip_big: u32,
    /// The pick's lot.
    pub lot: u64,
    /// The one-order maximum for that lot.
    pub max_order: i64,
}

/// The A2 pick at instant `t` over `live` (advanced to t), with the dated freeze units `freeze` (0: no freeze row, refused by
/// name), through [`chain::itm_fit`]. Also returns the Rs 5 L quantity (for the F8 gate).
///
/// RESUMED WAVE 4 (check-build-idx-S1-S6 F-e): `ghosts` = the LISTED strikes of the expiry with no store folder (or no rows that
/// day): they never traded, so the rule skips them as "no trade in [09:15:00, t)" and COUNTS them in the depth and the no-trade
/// skips, as the plan text says (the busiest rule's candidate set stays the store folders: G-EQ1 unchanged).
///
/// # Errors
/// A C11 host error.
pub fn pick_at(live: &[Live], ghosts: &[i64], ce: bool, spot: Option<i64>, t: i64, freeze: i64) -> Result<(Pick, u64), String> {
    let Some(sp) = spot else { return Ok((Pick { what: "no_spot".into(), ..Pick::default() }, 0)) };
    if freeze <= 0 {
        return Ok((Pick { what: "no_freeze_row".into(), ..Pick::default() }, 0));
    }
    let mut cands: Vec<ItmCand<'_>> = live.iter().map(|c| (c.strike, c.folder.as_str(), c.rows.get(..c.cursor).unwrap_or(&[]), u64::try_from(c.lot.lot().max(0)).unwrap_or(0))).collect();
    if !cfg!(feature = "m_fe_drop_ghosts") {
        cands.extend(ghosts.iter().map(|k| (*k, "", &[] as &[Sec], 0u64)));
    }
    match chain::itm_fit(&cands, ce, sp, t, TARGET, None, freeze) {
        Ok(p) => Ok((Pick { what: p.folder, strike: p.strike, depth: p.depth, skip_nt: p.skipped_no_trade, skip_big: p.skipped_big, lot: p.lot, max_order: p.max_order }, p.qty)),
        Err(e) if e == "no_itm_fit" => Ok((Pick { what: e, ..Pick::default() }, 0)),
        Err(e) => Err(e),
    }
}

/// F8: the Rs 5 L closest quantity of a pick fits one order (whole lots, at or below the maximum). Recomputed here from the
/// pick's own reference so the gate does not trust `itm_fit`.
#[must_use]
pub fn f8_ok(p: &Pick, live: &[Live]) -> bool {
    if p.strike == 0 {
        return true;
    }
    let Some(c) = live.iter().find(|c| c.folder == p.what) else { return false };
    let Some(r) = c.cursor.checked_sub(1).and_then(|k| c.rows.get(k)) else { return false };
    match size_units_closest(TARGET, Some(r.high), p.lot, None) {
        Ok((q, _)) => q >= 1 && p.lot > 0 && q % p.lot == 0 && i64::try_from(q).unwrap_or(i64::MAX) <= p.max_order,
        Err(_) => false,
    }
}

/// The reusable leader buffer of `live` (one [`Cand`] per candidate, its units updated in place each second).
#[must_use]
pub fn lead_buf(live: &[Live]) -> Vec<Cand> {
    live.iter().map(|c| Cand { folder: c.folder.clone(), strike: c.strike, units: vec![c.units] }).collect()
}

/// The busiest contract and the runner-up at t ([`chain::leader`]'s ordering) over `live` (advanced to t), through the buffer.
#[must_use]
pub fn lead_at(buf: &mut [Cand], live: &[Live], spot: i64) -> (Option<String>, Option<String>) {
    for (b, c) in buf.iter_mut().zip(live) {
        if let Some(u) = b.units.first_mut() {
            *u = c.units;
        }
    }
    let (a, b) = chain::leader(buf, 0, spot);
    (a.map(|x| x.folder.clone()), b.map(|x| x.folder.clone()))
}

/// Run-length rows of one (session, side, job): `(from_sod, to_sod exclusive, value)`.
#[derive(Clone, Debug, Default)]
pub struct Runs<T: PartialEq + Clone> {
    /// The runs.
    pub v: Vec<(i64, i64, T)>,
}

impl<T: PartialEq + Clone> Runs<T> {
    /// Appends the value of second `sod` (seconds are pushed in order, one apart).
    pub fn push(&mut self, sod: i64, x: T) {
        if let Some(last) = self.v.last_mut()
            && last.1 == sod
            && last.2 == x
        {
            last.1 = sod + 1;
            return;
        }
        self.v.push((sod, sod + 1, x));
    }
}

/// zstd-compresses `text` (ruzstd `Fastest`).
#[must_use]
pub fn zst(text: &[u8]) -> Vec<u8> {
    ruzstd::encoding::compress_to_vec(text, ruzstd::encoding::CompressionLevel::Fastest)
}

/// Decompresses a zstd frame (the pack's reader).
///
/// # Errors
/// A malformed frame.
pub fn unzst(b: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let mut d = ruzstd::decoding::StreamingDecoder::new(b).map_err(|e| format!("zstd: {e}"))?;
    let mut out = Vec::new();
    d.read_to_end(&mut out).map_err(|e| format!("zstd: {e}"))?;
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, reason = "tests")]
mod tests {
    use super::*;

    fn row(lot: Option<i64>, cands: &[i64]) -> LadderRow {
        LadderRow { lot, cands: cands.to_vec(), ..LadderRow::default() }
    }

    /// The incremental lot equals `chain::lot_at` over the prints before t, print by print (C17 / D9 incl. F7's LCM).
    #[test]
    fn lot_state_equals_lot_at() {
        for (r, prints) in [
            (row(Some(65), &[]), vec![130u64, 65]),
            (row(None, &[25, 75]), vec![150, 225, 75, 25]),
            (row(None, &[75, 50]), vec![150, 300, 450, 50]),
            (row(None, &[40]), vec![]),
            (row(None, &[40]), vec![30]),
            (row(None, &[15, 25, 30]), vec![150, 300, 75, 45]),
        ] {
            let mut s = LotState::new(&r);
            assert_eq!(s.lot(), chain::lot_at(&r, &[]).0);
            for k in 0..prints.len() {
                s.push(prints[k]);
                assert_eq!(s.lot(), chain::lot_at(&r, &prints[..=k]).0, "{r:?} after {:?}", &prints[..=k]);
            }
        }
    }

    /// F-e (RESUMED WAVE 4): a LISTED strike nearer the money with no store folder (a ghost) is a no-trade skip counted in the
    /// depth; the pick itself is unchanged (BANKNIFTY 2019-08-01 14:49 / 2026-03-30 15:03 are the real cases G-EQ3 found).
    #[test]
    fn fe_ghost_listed_strike_is_counted_in_the_depth() {
        let r = row(Some(75), &[]);
        let t = 100 * 1_000_000;
        let mut c = Live { strike: 1_985_000, folder: "fit".into(), rows: vec![crate::secs::Sec { ts: 2_000_000, high: 27_778, low: 27_778, close: 27_778, vol: 75 }], cursor: 0, units: 0, lot: LotState::new(&r) };
        c.advance(t);
        let live = vec![c];
        let (p, q) = pick_at(&live, &[1_995_000], true, Some(2_000_000), t, 1_800).unwrap_or_default();
        assert_eq!((p.what.as_str(), p.depth, p.skip_nt, q), ("fit", 1, 1, 1_800), "the ghost is counted, the pick unchanged");
        let (p0, _) = pick_at(&live, &[], true, Some(2_000_000), t, 1_800).unwrap_or_default();
        assert_eq!((p0.depth, p0.skip_nt), (0, 0));
        let (pf, _) = pick_at(&live, &[1_980_000], true, Some(2_000_000), t, 1_800).unwrap_or_default();
        assert_eq!(pf.depth, 0, "a ghost deeper than the pick is never reached");
    }

    /// Runs merge equal consecutive seconds only.
    #[test]
    fn runs_merge_consecutive_equal_seconds() {
        let mut r: Runs<u8> = Runs::default();
        for (s, x) in [(10, 1u8), (11, 1), (12, 2), (13, 2), (14, 1), (16, 1)] {
            r.push(s, x);
        }
        assert_eq!(r.v, vec![(10, 12, 1), (12, 14, 2), (14, 15, 1), (16, 17, 1)]);
        let z = zst(b"abcabcabcabc");
        assert_eq!(unzst(&z).unwrap_or_default(), b"abcabcabcabc");
    }
}

/// One corrected second as the pack stores it: (seconds of day, open, high, low, close, volume); volume 0 = a price-only second.
pub type NewSec = (i32, i64, i64, i64, i64, u64);

fn put_var(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let b = u8::try_from(v & 0x7f).unwrap_or(0);
        v >>= 7;
        if v == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}

fn get_var(b: &[u8], at: &mut usize) -> Result<u64, String> {
    let mut v = 0u64;
    let mut shift = 0u32;
    loop {
        let x = *b.get(*at).ok_or("newpx block: short varint")?;
        *at += 1;
        v |= u64::from(x & 0x7f).checked_shl(shift).ok_or("newpx block: varint too long")?;
        if x & 0x80 == 0 {
            return Ok(v);
        }
        shift += 7;
        if shift > 63 {
            return Err("newpx block: varint too long".into());
        }
    }
}

const fn zz(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)).cast_unsigned()
}

const fn unzz(v: u64) -> i64 {
    (v >> 1).cast_signed() ^ -((v & 1).cast_signed())
}

/// DP-NEWPX-IDXOPT's compact block of one contract-day (before zstd): the contract name, the count, then per second the varints
/// of (sod - previous sod, open - previous close (zigzag), high - open, open - low, close - low, volume). Lossless
/// ([`dec_block`] round-trips it; the builder asserts that on every block).
#[must_use]
pub fn enc_block(contract: &str, secs: &[NewSec]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + secs.len() * 6);
    put_var(&mut out, u64::try_from(contract.len()).unwrap_or(0));
    out.extend_from_slice(contract.as_bytes());
    put_var(&mut out, u64::try_from(secs.len()).unwrap_or(0));
    let (mut ps, mut pc) = (0i64, 0i64);
    for &(sod, o, h, l, c, v) in secs {
        let s = i64::from(sod);
        put_var(&mut out, zz(s - ps));
        put_var(&mut out, zz(o - pc));
        put_var(&mut out, zz(h - o));
        put_var(&mut out, zz(o - l));
        put_var(&mut out, zz(c - l));
        put_var(&mut out, v);
        (ps, pc) = (s, c);
    }
    out
}

/// Decodes one block from `b` at `at` (advanced past it): (contract, seconds).
///
/// # Errors
/// A malformed block.
pub fn dec_block(b: &[u8], at: &mut usize) -> Result<(String, Vec<NewSec>), String> {
    let n = usize::try_from(get_var(b, at)?).map_err(|e| e.to_string())?;
    let name = b.get(*at..*at + n).ok_or("newpx block: short name")?;
    let name = String::from_utf8(name.to_vec()).map_err(|e| e.to_string())?;
    *at += n;
    let k = usize::try_from(get_var(b, at)?).map_err(|e| e.to_string())?;
    let mut v = Vec::with_capacity(k);
    let (mut ps, mut pc) = (0i64, 0i64);
    for _ in 0..k {
        let s = ps + unzz(get_var(b, at)?);
        let o = pc + unzz(get_var(b, at)?);
        let h = o + unzz(get_var(b, at)?);
        let l = o - unzz(get_var(b, at)?);
        let c = l + unzz(get_var(b, at)?);
        let vol = get_var(b, at)?;
        v.push((i32::try_from(s).map_err(|e| e.to_string())?, o, h, l, c, vol));
        (ps, pc) = (s, c);
    }
    Ok((name, v))
}

#[cfg(test)]
mod block_tests {
    use super::*;

    /// The compact block round-trips exactly, including a price-only second (volume 0) and large jumps.
    #[test]
    fn newpx_block_round_trips() {
        let secs: Vec<NewSec> = vec![(33_300, 12_345, 12_400, 12_300, 12_390, 75), (33_301, 12_390, 12_390, 12_390, 12_390, 0), (55_799, 5, 9_999_995, 5, 500_000, 1_234_567)];
        let b = enc_block("2024-06-13-2310000-PE", &secs);
        let mut at = 0;
        let (n, v) = dec_block(&b, &mut at).unwrap_or_default();
        assert_eq!((n.as_str(), v, at), ("2024-06-13-2310000-PE", secs, b.len()));
    }
}
