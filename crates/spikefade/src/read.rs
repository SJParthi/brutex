//! v3: the READINGS AT THE DECISION T (EXITS-DYNAMIC-DESIGN 2, every reading from data ended by T: no look-ahead, REQ 56) and the LIQUIDITY
//! DIAL (MAIN 15 Nov 02:5x, from the operator: "we always pick top gainers desc and from there only very very high volume").
//!
//! Per contract-day with at least one event, ONE preparation (O(session seconds), freed after the day): Wilder ATR 14 of the option's own
//! 1-minute bars per minute slot (ATR1), of the 5-second candles per 5-second slot (ATR5, range only: the tape has no close, Q7), the
//! prefix count of traded seconds, the prefix sum of (HIGH - LOW)^2, the empty-run length of every second with a sparse table over it and
//! the next printed second. Every event's reading is then O(1): two or three array reads.
//!
//! The liquidity dial (one screener dimension of the FULL search; chosen on learning months only, like every setting): 7 values, read
//! over the 60 seconds STRICTLY BEFORE T (`[T - 60 s, T)`), from the contract's own seconds tape:
//! 0 off (the old behaviour) / 1-3 at least 30, 45, 55 traded seconds / 4-6 no run of more than 2, 5, 10 seconds without a print (a run
//! touching the window's start counts from the window's start).
//!
//! The calendar of the theta reading (critic R5): BOTH NSE holiday files (2018, 2019, 2026 and 2020-2025); a year with no holiday row
//! between T's day and the expiry refuses the reading (`theta_no_calendar`), never a default; a special session (a loaded day that is no
//! regular trading day, e.g. the Muhurat session 2026-11-08) counts its real minutes (the span of its present option bars).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::cal::{CLOSE_S, END_S, NM, OPEN_S, day_of_ymd, parse_day};
use crate::cube::{Cube, Sec};
use crate::menu::Rd;

/// Session seconds indexed from 09:15:00 to 15:40:00 (23,100) + 1.
pub const NSEC: usize = (END_S - OPEN_S) as usize + 1;
/// 5-second slots of the session.
pub const N5: usize = (END_S - OPEN_S) as usize / 5;
/// The liquidity dial's values (v3.1: 7; v3.2 addendum 14 (b) adds 3 values that need BOTH P prints of the last 60 s AND no silence above
/// G s: P / G = 30 / 10, 45 / 5, 55 / 2; `--liq all` reads [`nliq`]).
pub const NLIQ: usize = 10;
/// The liquidity dial's names.
pub const LIQ_NAMES: [&str; NLIQ] = ["off", ">=30 traded s of the last 60", ">=45 traded s of the last 60", ">=55 traded s of the last 60", "no gap > 2 s in the last 60", "no gap > 5 s in the last 60", "no gap > 10 s in the last 60", ">=30 traded s AND no gap > 10 s in the last 60", ">=45 traded s AND no gap > 5 s in the last 60", ">=55 traded s AND no gap > 2 s in the last 60"];
/// Short names.
pub const LIQ_SHORT: [&str; NLIQ] = ["L0", "C30", "C45", "C55", "G2", "G5", "G10", "C30G10", "C45G5", "C55G2"];

/// v3.2: the liquidity values of `--liq all`: v3.1's 7, or 10 under the LIQ14 switch (addendum 14 (b): P and G together).
#[must_use]
pub fn nliq() -> usize {
    if crate::sw::on(crate::sw::LIQ14) { NLIQ } else { 7 }
}

/// One contract-day's preparation.
#[derive(Clone, Debug, Default)]
pub struct Prep {
    /// ATR1 after the present bar of slot m (paisa x 1,000), None before 14 present bars.
    atr1: Vec<Option<i64>>,
    /// ATR5 after 5-second slot j, None before 14 candles with a print.
    atr5: Vec<Option<i64>>,
    /// cnt[k] = traded seconds in [09:15:00, 09:15:00 + k).
    cnt: Vec<u32>,
    /// sq[k] = sum of (HIGH - LOW)^2 over the traded seconds in [09:15:00, 09:15:00 + k).
    sq: Vec<u128>,
    /// run[k] = the empty seconds ending at second k (0 when k printed).
    run: Vec<u16>,
    /// Sparse max table over `run`.
    st: Vec<Vec<u16>>,
    /// nextp[k] = the first printed second index at or after k (NSEC = none).
    nextp: Vec<u32>,
    /// v3.2 addendum 14 (b): volp[k] = the volume of the printed seconds in [09:15:00, 09:15:00 + k).
    volp: Vec<u64>,
}

#[inline]
fn wilder(prev: Option<(i64, u32, i64)>, tr: i64) -> (i64, u32, i64) {
    // state (atr x 1000 once seeded, count, sum of TR before the seed)
    match prev {
        None => (0, 1, tr),
        Some((atr, n, sum)) => {
            if n < 14 {
                let n2 = n + 1;
                let s2 = sum + tr;
                if n2 == 14 { (s2 * 1_000 / 14, 14, s2) } else { (0, n2, s2) }
            } else {
                ((atr * 13 + tr * 1_000) / 14, n + 1, sum)
            }
        }
    }
}

impl Prep {
    /// The preparation of contract `ci` of cube `c` with its seconds `secs` (rows of [09:15:00, 15:40:00), one per second).
    #[must_use]
    pub fn new(c: &Cube, ci: usize, secs: &[Sec]) -> Self {
        // v3.4 E8: a direct array read of contract ci, counted when it lies outside a loaded segment slice
        c.guard(ci);
        // ATR1 over the present 1-minute bars
        let mut atr1 = vec![None; NM];
        let mut w: Option<(i64, u32, i64)> = None;
        let mut cprev: Option<i64> = None;
        for m in 0..NM {
            let ix = Cube::ix(ci, m);
            if c.flags.get(ix).is_some_and(|f| f & 1 == 1) {
                let h = i64::from(c.high.get(ix).copied().unwrap_or(0));
                let l = i64::from(c.low.get(ix).copied().unwrap_or(0));
                let tr = match cprev {
                    None => h - l,
                    Some(cp) => h.max(cp) - l.min(cp),
                };
                w = Some(wilder(w, tr));
                cprev = Some(i64::from(c.close.get(ix).copied().unwrap_or(0)));
            }
            if let (Some(slot), Some((a, n, _))) = (atr1.get_mut(m), w)
                && n >= 14
            {
                *slot = Some(a);
            }
        }
        // per second: traded flags, (H-L)^2, 5-second candles
        let mut cnt = vec![0_u32; NSEC];
        let mut sq = vec![0_u128; NSEC];
        let mut printed = vec![false; NSEC];
        let mut hl: Vec<(i64, i64)> = vec![(0, 0); NSEC];
        let mut vsec = vec![0_u64; NSEC];
        for s in secs {
            let k = i64::from(s.sod) - OPEN_S;
            let Ok(k) = usize::try_from(k) else { continue };
            if k + 1 >= NSEC {
                continue;
            }
            if s.vol > 0 && s.high > 0 && s.low > 0 && s.high >= s.low {
                if let Some(p) = printed.get_mut(k) {
                    *p = true;
                }
                if let Some(x) = hl.get_mut(k) {
                    *x = (i64::from(s.high), i64::from(s.low));
                }
                if let Some(x) = vsec.get_mut(k) {
                    *x = x.saturating_add(u64::from(s.vol));
                }
            }
        }
        let mut volp = vec![0_u64; NSEC + 1];
        for k in 0..NSEC {
            let v = volp.get(k).copied().unwrap_or(0).saturating_add(vsec.get(k).copied().unwrap_or(0));
            if let Some(x) = volp.get_mut(k + 1) {
                *x = v;
            }
        }
        let mut run = vec![0_u16; NSEC];
        let mut c0 = 0_u32;
        let mut s0 = 0_u128;
        let mut r = 0_u16;
        for k in 0..NSEC {
            if let (Some(a), Some(b)) = (cnt.get_mut(k), sq.get_mut(k)) {
                *a = c0;
                *b = s0;
            }
            let p = printed.get(k).copied().unwrap_or(false);
            if p {
                c0 += 1;
                let (h, l) = hl.get(k).copied().unwrap_or((0, 0));
                let d = u128::try_from(h - l).unwrap_or(0);
                s0 += d * d;
                r = 0;
            } else {
                r = r.saturating_add(1);
            }
            if let Some(x) = run.get_mut(k) {
                *x = r;
            }
        }
        let mut st = vec![run.clone()];
        let mut lv = 1_usize;
        while (1_usize << lv) <= NSEC {
            let half = 1_usize << (lv - 1);
            let prev = st.get(lv - 1).cloned().unwrap_or_default();
            st.push((0..=(NSEC - (1_usize << lv))).map(|i| prev.get(i).copied().unwrap_or(0).max(prev.get(i + half).copied().unwrap_or(0))).collect());
            lv += 1;
        }
        let mut nextp = vec![u32::try_from(NSEC).unwrap_or(u32::MAX); NSEC + 1];
        for k in (0..NSEC).rev() {
            let nx = if printed.get(k).copied().unwrap_or(false) { u32::try_from(k).unwrap_or(u32::MAX) } else { nextp.get(k + 1).copied().unwrap_or(u32::MAX) };
            if let Some(x) = nextp.get_mut(k) {
                *x = nx;
            }
        }
        // ATR5 over 5-second candles with a print
        let mut atr5 = vec![None; N5];
        let mut w5: Option<(i64, u32, i64)> = None;
        for j in 0..N5 {
            let (mut hi, mut lo, mut any) = (i64::MIN, i64::MAX, false);
            for k in (5 * j)..(5 * j + 5) {
                if printed.get(k).copied().unwrap_or(false) {
                    let (h, l) = hl.get(k).copied().unwrap_or((0, 0));
                    hi = hi.max(h);
                    lo = lo.min(l);
                    any = true;
                }
            }
            if any {
                w5 = Some(wilder(w5, hi - lo));
            }
            if let (Some(slot), Some((a, n, _))) = (atr5.get_mut(j), w5)
                && n >= 14
            {
                *slot = Some(a);
            }
        }
        Self { atr1, atr5, cnt, sq, run, st, nextp, volp }
    }

    /// v3.2 addendum 14 (b): the volume traded in [T - 60 s, T) (strictly before the decision; the coverage gate's reading).
    #[must_use]
    pub fn vol60_at(&self, t: i64) -> u64 {
        let (ka, kb) = (Self::k_of(t - 60), Self::k_of(t));
        self.volp.get(kb).copied().unwrap_or(0).saturating_sub(self.volp.get(ka).copied().unwrap_or(0))
    }

    fn k_of(t: i64) -> usize {
        usize::try_from((t - OPEN_S).clamp(0, i64::try_from(NSEC - 1).unwrap_or(0))).unwrap_or(0)
    }

    /// ATR1 at T (the present bars ENDED by T: slots m <= b - 1, T = 09:15 + b minutes).
    #[must_use]
    pub fn atr1_at(&self, t: i64) -> Option<i64> {
        let b = usize::try_from((t - OPEN_S) / 60).unwrap_or(0);
        // MUTANT m_sf3_atr_next: the bar after T (slot b) is read too (look-ahead)
        let m = if cfg!(feature = "m_sf3_atr_next") { b } else { b.checked_sub(1)? };
        self.atr1.get(m).copied().flatten()
    }

    /// ATR5 at T (the last 5-second candle completed by T).
    #[must_use]
    pub fn atr5_at(&self, t: i64) -> Option<i64> {
        let j = usize::try_from((t - OPEN_S) / 5).ok()?.checked_sub(1)?;
        self.atr5.get(j).copied().flatten()
    }

    /// (sum of (HIGH - LOW)^2, traded seconds) over [a, b) (session seconds; clamped to the session).
    #[must_use]
    pub fn sq_between(&self, a: i64, b: i64) -> (u128, u64) {
        let (ka, kb) = (Self::k_of(a), Self::k_of(b));
        let s = self.sq.get(kb).copied().unwrap_or(0).saturating_sub(self.sq.get(ka).copied().unwrap_or(0));
        let m = self.cnt.get(kb).copied().unwrap_or(0).saturating_sub(self.cnt.get(ka).copied().unwrap_or(0));
        (s, u64::from(m))
    }

    /// The liquidity readings at T: (traded seconds in [T - 60 s, T), the longest run of seconds without a print in it).
    #[must_use]
    pub fn liq_at(&self, t: i64) -> (u32, u32) {
        // MUTANT m_sf3_liq_lookahead: the window [T - 59 s, T + 1 s) (the second T itself counted)
        let (a, b) = if cfg!(feature = "m_sf3_liq_lookahead") { (t - 59, t + 1) } else { (t - 60, t) };
        let (ka, kb) = (Self::k_of(a), Self::k_of(b));
        let c = self.cnt.get(kb).copied().unwrap_or(0).saturating_sub(self.cnt.get(ka).copied().unwrap_or(0));
        let first = self.nextp.get(ka).copied().unwrap_or(u32::MAX) as usize;
        let gap = if first >= kb {
            u32::try_from(kb - ka).unwrap_or(u32::MAX)
        } else {
            let lead = u32::try_from(first - ka).unwrap_or(0);
            // the runs inside [first, kb): a range max over run (no run there reaches back past `first`, a printed second)
            let (lo, hi) = (first, kb - 1);
            let len = hi - lo + 1;
            let lvl = usize::try_from(len.ilog2()).unwrap_or(0);
            let t0 = self.st.get(lvl);
            let x = t0.and_then(|r| r.get(lo)).copied().unwrap_or(0).max(t0.and_then(|r| r.get(hi + 1 - (1_usize << lvl))).copied().unwrap_or(0));
            lead.max(u32::from(x))
        };
        (c, gap)
    }

    /// The plain reading of the same (the test reference): every second of the window walked.
    #[must_use]
    pub fn liq_plain(secs: &[Sec], t: i64) -> (u32, u32) {
        let printed = |s: i64| secs.iter().any(|x| i64::from(x.sod) == s && x.vol > 0 && x.high > 0 && x.low > 0 && x.high >= x.low);
        let (mut c, mut gap, mut r) = (0_u32, 0_u32, 0_u32);
        for s in (t - 60)..t {
            if printed(s) {
                c += 1;
                r = 0;
            } else {
                r += 1;
                gap = gap.max(r);
            }
        }
        (c, gap)
    }
}

/// Does an event pass liquidity value `v` with readings (traded seconds, longest gap)?
#[must_use]
pub const fn liq_pass(v: usize, cnt: u32, gap: u32) -> bool {
    match v {
        0 => true,
        1 => cnt >= 30,
        2 => cnt >= 45,
        3 => cnt >= 55,
        4 => gap <= 2,
        5 => gap <= 5,
        6 => gap <= 10,
        7 => cnt >= 30 && gap <= 10,
        8 => cnt >= 45 && gap <= 5,
        9 => cnt >= 55 && gap <= 2,
        _ => false,
    }
}

/// The theta calendar: NSE holidays of both files, the covered years, the special sessions of the loaded days (day -> real minutes from
/// its first present bar to its last present bar's end, as (first slot, last slot)).
#[derive(Clone, Debug, Default)]
pub struct Cal3 {
    pub hol: BTreeSet<i64>,
    pub years: BTreeSet<i64>,
    pub special: BTreeMap<i64, (usize, usize)>,
    pub sources: Vec<String>,
    /// REPAIR A (check-engine-2 M3): the special sessions come from the DATED table (`--special-sessions`), never from the loaded days.
    pub dated: bool,
    /// REPAIR A: the dated table's TRADING minutes of each special session (may be fewer than its span: the 2024 Saturday sessions ran
    /// 09:15-10:00 and 11:30-12:30).
    pub special_min: BTreeMap<i64, i64>,
}

impl Cal3 {
    /// Reads NSE holiday files (`YYYY-MM-DD<TAB>name`; `#` notes).
    ///
    /// # Errors
    /// An unreadable file or no row.
    pub fn load(files: &[&Path]) -> Result<Self, String> {
        let mut c = Self::default();
        for p in files {
            let t = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let mut n = 0;
            for l in t.lines() {
                if l.starts_with('#') || l.trim().is_empty() {
                    continue;
                }
                let d = l.split('\t').next().unwrap_or("");
                if let Some(day) = parse_day(d) {
                    c.hol.insert(day);
                    c.years.insert(crate::cal::civil(day).0);
                    n += 1;
                }
            }
            if n == 0 {
                return Err(format!("{}: no holiday row", p.display()));
            }
            c.sources.push(format!("{} ({n} rows)", p.display()));
        }
        Ok(c)
    }

    /// Records the loaded days that are no regular trading day as special sessions (their present option bars' span).
    pub fn add_specials(&mut self, cubes: &[Cube]) {
        // REPAIR A (check-engine-2 M3): under the dated calendar the loaded days never change it (`dated_check` verifies them)
        // MUTANT m_v32r_sess_loaded: the loaded days' realized spans still written over the dated table
        if self.dated && !cfg!(feature = "m_v32r_sess_loaded") {
            return;
        }
        for c in cubes {
            if self.regular(c.day) {
                continue;
            }
            let (mut a, mut b) = (usize::MAX, 0_usize);
            for (i, f) in c.flags.iter().enumerate() {
                if f & 1 == 1 {
                    let m = i % NM;
                    a = a.min(m);
                    b = b.max(m);
                }
            }
            if a <= b {
                self.special.insert(c.day, (a, b));
                if self.dated {
                    // (only reached under MUTANT m_v32r_sess_loaded: the loaded day's realized span also written over the dated minutes)
                    self.special_min.insert(c.day, i64::try_from(b + 1 - a).unwrap_or(0));
                }
            }
        }
    }

    /// A regular trading day (Monday-Friday, not a holiday; under the dated calendar also not a dated special session, e.g. a weekday
    /// Muhurat session).
    #[must_use]
    pub fn regular(&self, d: i64) -> bool {
        crate::rules::weekday(d) < 5 && !self.hol.contains(&d) && !(self.dated && self.special.contains_key(&d))
    }

    /// REPAIR A (check-engine-2 M3): installs the DATED special-session calendar (every special session NSE held, with its slots or its
    /// minutes), replacing the loaded-day specials; afterwards `add_specials` / the streamed pass 0 only VERIFY the loaded days against it.
    pub fn apply_dated(&mut self, sc: &crate::expiry::SpecialCal) {
        self.special = sc.days.iter().map(|(&d, x)| (d, (x.0, x.1))).collect();
        self.special_min = sc.days.iter().map(|(&d, x)| (d, x.2)).collect();
        self.dated = true;
        self.sources.push(format!("dated special sessions {} (sha256 {}, {} sessions)", sc.path, sc.sha, sc.days.len()));
    }

    /// REPAIR A: a loaded day that is no regular trading day under the dated calendar must be in the dated table (else the run is refused:
    /// the calendar never depends on the loaded days).
    ///
    /// # Errors
    /// The loaded special day missing from the table.
    pub fn dated_check(&self, day: i64) -> Result<(), String> {
        // MUTANT m_v32r_sess_loaded: (see add_specials) the check is skipped
        if !self.dated || self.regular(day) || self.special.contains_key(&day) || cfg!(feature = "m_v32r_sess_loaded") {
            return Ok(());
        }
        Err(format!("{}: a loaded day that is no regular session is missing from the dated special-session table: refused (add it with its NSE source)", crate::cal::fmt_day(day)))
    }

    /// Trading minutes from second `t` of day `d` to 15:30 of the expiry day `e` (375 a full regular session; special sessions their real
    /// minutes); Err(`theta_no_calendar`) when a year between d and e has no holiday row.
    ///
    /// # Errors
    /// The refusal code.
    pub fn minutes(&self, d: i64, t: i64, e: i64) -> Result<i64, &'static str> {
        if e < d {
            return Err("theta_none");
        }
        let (y0, y1) = (crate::cal::civil(d).0, crate::cal::civil(e).0);
        for y in y0..=y1 {
            if !self.years.contains(&y) {
                return Err("theta_no_calendar");
            }
        }
        let mut m = if let Some(&(_, b)) = self.special.get(&d) {
            // a special session: to the end of its last present bar
            (OPEN_S + 60 * i64::try_from(b + 1).unwrap_or(0) - t).max(0) / 60
        } else {
            (CLOSE_S - t).max(0) / 60
        };
        for x in (d + 1)..=e {
            if self.regular(x) {
                m += 375;
            } else if let Some(&(a, b)) = self.special.get(&x) {
                m += match (self.dated, self.special_min.get(&x)) {
                    (true, Some(&tm)) => tm,
                    _ => i64::try_from(b + 1 - a).unwrap_or(0),
                };
            }
        }
        Ok(m)
    }
}

/// The tf-independent readings of an event at T (`b` = T's minute slot end: T = 09:15 + b minutes).
#[must_use]
pub fn base_reading(c: &Cube, ci: usize, t: i64, pr: &Prep, cal: &Cal3) -> Rd {
    let v = pr.sq_between(t - 300, t);
    let v300 = if v.1 >= 30 { Some(v) } else { None };
    let b = usize::try_from((t - OPEN_S) / 60).unwrap_or(0);
    let theta = theta_at(c, ci, t, b, cal);
    Rd { atr1: pr.atr1_at(t), atr5: pr.atr5_at(t), hwin: 0, rise: 0, v300, theta }
}

fn theta_at(c: &Cube, ci: usize, t: i64, b: usize, cal: &Cal3) -> Result<(i64, i64), &'static str> {
    let k = c.cons.get(ci).ok_or("theta_none")?;
    let ct = i64::from(c.close_at(ci, b.checked_sub(1).ok_or("theta_none")?));
    let s = i64::from(c.spot_at(usize::from(k.und), b - 1));
    if s <= 0 || ct <= 0 {
        return Err("theta_none");
    }
    let intrinsic = if k.right == 0 { (s - k.strike).max(0) } else { (k.strike - s).max(0) };
    let tv = (ct - intrinsic).max(0);
    if tv == 0 {
        return Err("theta_none");
    }
    let mt = cal.minutes(c.day, t, day_of_ymd(k.expiry))?;
    if mt <= 0 {
        return Err("theta_none");
    }
    Ok((tv, mt))
}

/// The window readings of an event for timeframe `tf` (minutes; 0 = day so far): (the highest HIGH of the window's present bars, close_T -
/// start), the start as the screener's `member` reads it.
#[must_use]
pub fn window_reading(c: &Cube, ci: usize, t: i64, tf: u32) -> (i64, i64) {
    // v3.4 E8: a direct array read, counted outside a loaded segment slice
    c.guard(ci);
    let b = usize::try_from((t - OPEN_S) / 60).unwrap_or(0);
    let a = if tf == 0 { 0 } else { b.saturating_sub(tf as usize) };
    let mut hi = 0_i64;
    for m in a..b {
        let ix = Cube::ix(ci, m);
        if c.flags.get(ix).is_some_and(|f| f & 1 == 1) {
            hi = hi.max(i64::from(c.high.get(ix).copied().unwrap_or(0)));
        }
    }
    let rise = crate::screen::member(c, ci, a, b, tf == 0).map_or(0, |m| i64::from(m.ct) - i64::from(m.c0));
    (hi, rise)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cal::{days_from_civil, ymd_of};
    use crate::cube::{Contract, Und};

    fn cube_with(bars: &[(usize, i32, i32, i32)]) -> Cube {
        let d = days_from_civil(2026, 11, 4);
        let mut c = Cube { ymd: ymd_of(d), day: d, ..Cube::default() };
        c.unds.push(Und { name: "SBIN".to_owned(), kind: 1, spot_seg: 1, spot_sid: 1, nearest: ymd_of(d + 19), next: 0 });
        c.cons.push(Contract { sid: 7, und: 0, kind: 1, right: 0, expiry: ymd_of(d + 19), lot: 50, strike: 100_000, first_min: 0, first_open: 1_000, symbol: "X".to_owned(), n_sec: 0 });
        c.spot = vec![100_500; NM];
        c.close = vec![0; NM];
        c.flags = vec![0; NM];
        c.high = vec![0; NM];
        c.low = vec![0; NM];
        c.vol = vec![0; NM];
        c.oi = vec![0; NM];
        let mut last = 0;
        for m in 0..NM {
            if let Some(&(_, h, l, cl)) = bars.iter().find(|b| b.0 == m) {
                c.flags[m] = 5;
                c.high[m] = h;
                c.low[m] = l;
                c.close[m] = cl;
                c.vol[m] = 100;
                last = cl;
            } else {
                c.close[m] = last;
                if last > 0 {
                    c.flags[m] = 2;
                }
            }
        }
        c.cumvol = (0..=NM as u64).map(|m| m * 100).collect();
        c.cumval = (0..=NM as u64).map(|m| m * 100 * 1_000).collect();
        c
    }

    #[test]
    fn sf3_sl_atr1_reads_only_bars_ended_by_t() {
        // 20 present bars of range 10 (H 1,010 L 1,000, close 1,005): Wilder ATR 14 = 10.000 paisa
        let bars: Vec<(usize, i32, i32, i32)> = (0..20).map(|m| (m, 1_010, 1_000, 1_005)).collect();
        let c = cube_with(&bars);
        let p = Prep::new(&c, 0, &[]);
        let t = OPEN_S + 20 * 60; // T = the end of slot 19
        assert_eq!(p.atr1_at(t), Some(10_000));
        // every bar AFTER T moved by +-5 ATR: the reading is unchanged
        let mut b2 = bars.clone();
        b2.push((20, 1_060, 950, 1_000));
        b2.push((21, 1_200, 800, 1_000));
        let p2 = Prep::new(&cube_with(&b2), 0, &[]);
        assert_eq!(p2.atr1_at(t), Some(10_000), "no look-ahead: bars after T never move ATR1 at T");
        // fewer than 14 present bars ended by T: unready
        assert_eq!(p.atr1_at(OPEN_S + 13 * 60), None);
        assert_eq!(p.atr1_at(OPEN_S + 14 * 60), Some(10_000));
        // the first bar's TR is its range; later bars use the previous present bar's close
        let gap: Vec<(usize, i32, i32, i32)> = (0..14).map(|m| (m, if m == 13 { 1_100 } else { 1_010 }, 1_000, 1_005)).collect();
        let p3 = Prep::new(&cube_with(&gap), 0, &[]);
        // TRs: 10 x 13 then max(1,100, 1,005) - min(1,000, 1,005) = 100: (130 + 100) / 14 = 16.428 -> 16,428 (x 1,000, floored)
        assert_eq!(p3.atr1_at(OPEN_S + 14 * 60), Some(16_428));
    }

    #[test]
    fn sf3_sl_atr5_range_only_last_completed_candle() {
        let c = cube_with(&[(0, 1_000, 1_000, 1_000)]);
        // 5-second candles from 09:15:00, each with one print of range 20 (14 candles: 09:15:00 .. 09:16:09)
        let mut secs: Vec<Sec> = (0..14).map(|j| Sec { sod: u32::try_from(OPEN_S + 5 * j).unwrap_or(0), high: 1_020, low: 1_000, vol: 10 }).collect();
        let t = OPEN_S + 70;
        let p = Prep::new(&c, 0, &secs);
        assert_eq!(p.atr5_at(t), Some(20_000));
        // a print inside the candle still FORMING at T (09:16:10 .. 09:16:15) never moves it
        secs.push(Sec { sod: u32::try_from(OPEN_S + 71).unwrap_or(0), high: 2_000, low: 900, vol: 10 });
        let p2 = Prep::new(&c, 0, &secs);
        assert_eq!(p2.atr5_at(t), Some(20_000));
        // moving the last COMPLETED candle (09:16:05) changes it: TR 120 instead of 20 -> (20 x 13 + 120) / 14 = 27.142
        let mut s3: Vec<Sec> = (0..14).map(|j| Sec { sod: u32::try_from(OPEN_S + 5 * j).unwrap_or(0), high: 1_020, low: 1_000, vol: 10 }).collect();
        s3.push(Sec { sod: u32::try_from(OPEN_S + 66).unwrap_or(0), high: 1_120, low: 1_000, vol: 10 });
        let p3 = Prep::new(&c, 0, &s3);
        assert_eq!(p3.atr5_at(t), Some(27_142));
        assert_eq!(p.atr5_at(OPEN_S + 65), None, "13 candles: unready");
    }

    #[test]
    fn sf3_sl_vol300_needs_30_seconds() {
        let c = cube_with(&[(0, 1_000, 1_000, 1_000)]);
        let t = OPEN_S + 600;
        let secs: Vec<Sec> = (0..29).map(|i| Sec { sod: u32::try_from(t - 10 - i).unwrap_or(0), high: 1_010, low: 1_000, vol: 5 }).collect();
        let p = Prep::new(&c, 0, &secs);
        assert_eq!(p.sq_between(t - 300, t), (29 * 100, 29));
        let mut s2 = secs.clone();
        s2.push(Sec { sod: u32::try_from(t - 200).unwrap_or(0), high: 1_004, low: 1_000, vol: 5 });
        s2.sort_by_key(|s| s.sod);
        let p2 = Prep::new(&c, 0, &s2);
        assert_eq!(p2.sq_between(t - 300, t), (29 * 100 + 16, 30));
        // a print AT T is not before T
        let mut s3 = s2.clone();
        s3.push(Sec { sod: u32::try_from(t).unwrap_or(0), high: 2_000, low: 1_000, vol: 5 });
        assert_eq!(Prep::new(&c, 0, &s3).sq_between(t - 300, t), (29 * 100 + 16, 30));
    }

    #[test]
    fn sf3_liquidity_dial_counts_and_gaps_before_t_only() {
        let c = cube_with(&[(0, 1_000, 1_000, 1_000)]);
        let t = OPEN_S + 3_600;
        // prints at T-60 .. T-1 except a 7-second hole T-30 .. T-24 and one zero-volume row inside it
        let mut secs: Vec<Sec> = ((t - 60)..t).filter(|s| !((t - 30)..(t - 23)).contains(s)).map(|s| Sec { sod: u32::try_from(s).unwrap_or(0), high: 1_010, low: 1_000, vol: 3 }).collect();
        secs.push(Sec { sod: u32::try_from(t - 27).unwrap_or(0), high: 5_000, low: 100, vol: 0 });
        secs.push(Sec { sod: u32::try_from(t).unwrap_or(0), high: 1_010, low: 1_000, vol: 3 });
        secs.sort_by_key(|s| s.sod);
        let p = Prep::new(&c, 0, &secs);
        assert_eq!(p.liq_at(t), (53, 7));
        assert_eq!(p.liq_at(t), Prep::liq_plain(&secs, t));
        assert!(liq_pass(2, 53, 7) && !liq_pass(3, 53, 7) && !liq_pass(5, 53, 7) && liq_pass(6, 53, 7));
        // a run touching the window's start counts from the window's start; nothing printed at all: 60
        let late: Vec<Sec> = ((t - 20)..t).map(|s| Sec { sod: u32::try_from(s).unwrap_or(0), high: 1_010, low: 1_000, vol: 3 }).collect();
        let p2 = Prep::new(&c, 0, &late);
        assert_eq!(p2.liq_at(t), (20, 40));
        assert_eq!(Prep::new(&c, 0, &[]).liq_at(t), (0, 60));
        // randomized: the O(1) reading equals the plain walk
        let mut g = 12_345_u64;
        for _ in 0..300 {
            let mut v = Vec::new();
            let base = OPEN_S + 120;
            for s in base..(base + 400) {
                g = g.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                if !(g >> 33).is_multiple_of(3) {
                    v.push(Sec { sod: u32::try_from(s).unwrap_or(0), high: 1_010, low: 1_000, vol: u32::try_from((g >> 40) % 4).unwrap_or(0) });
                }
            }
            let p3 = Prep::new(&c, 0, &v);
            for tt in [base + 60, base + 61, base + 200, base + 399, base + 400] {
                assert_eq!(p3.liq_at(tt), Prep::liq_plain(&v, tt));
            }
        }
    }

    #[test]
    fn sf3_tp_theta_needs_time_value_and_the_dated_calendar() {
        let mut cal = Cal3::default();
        let d = days_from_civil(2026, 11, 4);
        for y in [2026_i64, 2027] {
            cal.years.insert(y);
        }
        cal.hol.insert(days_from_civil(2026, 11, 10));
        // T 15:00 on Wed 4 Nov, expiry Mon 9 Nov: 30 + Thu 375 + Fri 375 + Mon 375 = 1,155
        assert_eq!(cal.minutes(d, CLOSE_S - 1_800, d + 5), Ok(1_155));
        // a holiday between them (Tue 10 Nov) when the expiry is Wed 11 Nov: + 0 for the holiday + 375
        assert_eq!(cal.minutes(d, CLOSE_S - 1_800, d + 7), Ok(1_155 + 375));
        // a Muhurat session on Sunday 8 Nov (slots 306 .. 384): + 79 minutes
        cal.special.insert(days_from_civil(2026, 11, 8), (306, 384));
        assert_eq!(cal.minutes(d, CLOSE_S - 1_800, d + 5), Ok(1_155 + 79));
        // T on the expiry day itself: minutes to 15:30
        assert_eq!(cal.minutes(d, CLOSE_S - 600, d), Ok(10));
        // a year with no calendar row refuses
        assert_eq!(cal.minutes(d, CLOSE_S - 600, days_from_civil(2028, 1, 4)), Err("theta_no_calendar"));
        // time value: a CE at strike Rs 1,000 with spot Rs 1,005 and premium Rs 10: intrinsic Rs 5, TV Rs 5
        let c = cube_with(&[(0, 1_000, 1_000, 1_000)]);
        let t = OPEN_S + 60;
        assert_eq!(theta_at(&c, 0, t, 1, &cal).map(|x| x.0), Ok(500));
        // deep in the money (no time value): theta_none
        let mut c2 = c.clone();
        c2.spot = vec![102_000; NM];
        assert_eq!(theta_at(&c2, 0, t, 1, &cal), Err("theta_none"));
    }
}
