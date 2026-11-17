//! v3.3 FIX SWITCHES (integrate:v33; MAIN-RULINGS-ENGINE-v22 Addendum 28.2: "every BLOCKING item of the audits and re-checks"): the MUST
//! items of the independent re-checks of round B, `v32/check-engine-1r.md` (M1r, M2r, M3r) and `v32/check-engine-2r.md` (M1, M2, M3, M4),
//! 16 Nov 2026.
//!
//! Unlike the v3.3 speed switches (`sw33`) these CHANGE results. Each defaults to OFF, so with every one off the binary replays round B's
//! FINAL binary (bf337da9...) byte for byte (proof P0, with or without the speed switches). `run3 --fix33` turns every one on; each has its
//! own flag. Their words go to RUN.log, to the RESULTS header and to the day-major checkpoint fingerprint (a resume never mixes two readings).
//! Process-wide in the binary (set once from the command line before any work); under `cfg(test)` each test thread has its own copy.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::screen::{Levels, TFS};

/// A v3.3 fix switch (an index into the fix table).
pub type Fx = usize;
/// check-engine-2 M1 / check-engine-1 M2r: CAUSAL screener levels. Round B's min-volume levels L1-L4 and min-value levels V1-V3 are
/// percentiles of the top-50 members over EVERY candle end of EVERY run day (a look-ahead into every learning window). Under this switch day
/// d's levels are those percentiles (the same samples, the same percentiles, the same rounding) over the run's previous `W` sessions that hold
/// the segment (`--levels-window`, default 60; fewer at the run's start): known before day d's open. The run's first session of a segment has
/// no earlier session: WARM-UP (no level is known, so no event is listed; counted and printed).
pub const LV_CAUSAL: Fx = 0;
/// check-engine-1 M1r (addendum 17 "hard at every instant"; item 4): the kernel refuses a sale that would pass the cap, but nothing acts
/// between sales (no RMS model, matrix X20). Until an RMS is modelled, every account a pick or the verdict view can read is checked MINUTE BY
/// MINUTE (margin in use + the running MTM loss against the account base Rs 20 L + min(0, realized)); an account above it in any minute is
/// EXCLUDED from those views (the next checked account takes its place), an s26 portfolio above it is not admitted, and checklist item 7
/// prints PARTIAL.
pub const CAP_MIN: Fx = 1;
/// check-engine-1 M3r: the dated SEBI fee row (SEBI/LAD-NRO/GN/2020/011 = NSE FA44514: 50% of the fee from 2020-06-01 to 2021-03-31), and the
/// pre-2020-07-01 stamp duty (0.02% on both sides, no source on disk) LABELLED "UNVERIFIED (conservative)" on every pick and in item 8
/// (PARTIAL when the run holds such a session).
pub const CHG33: Fx = 2;
/// check-engine-2 M2: the split / side-by-side words print the segment's own slot-count grid (index N 15 / 20, not v3.1's 20 / 40), and the
/// label block names the data vendor and period of the run's days from each day's cube manifest (never "Tickvault" for a GDFL day).
pub const WORDS33: Fx = 3;
/// check-engine-2 M3: the day-major stage B samples its account-day walks (every 64th per worker thread, wall clock, with their steps; a
/// bounded uniform subsample) so the all-years mode measures its per-step p50 / p99; zero samples print "NOT MEASURED", never 0.000.
pub const STEP_DM: Fx = 4;
const N: usize = 5;
/// The number of fix switches (tests reset every one).
pub const COUNT: usize = N;
/// LV_CAUSAL: the trailing window in sessions.
pub static LV_WINDOW: AtomicUsize = AtomicUsize::new(60);
/// CAP_MIN: the accounts checked per (group, view) a pick or the verdict view reads.
pub static CAP_K: AtomicUsize = AtomicUsize::new(20);

#[cfg(not(test))]
static FLAGS: [std::sync::atomic::AtomicBool; N] = [const { std::sync::atomic::AtomicBool::new(false) }; N];
#[cfg(test)]
thread_local! {
    static FLAGS: [std::cell::Cell<bool>; N] = const { [const { std::cell::Cell::new(false) }; N] };
}

/// One fix switch's value.
#[inline]
#[must_use]
pub fn on(s: Fx) -> bool {
    #[cfg(not(test))]
    {
        FLAGS.get(s).is_some_and(|f| f.load(Ordering::Relaxed))
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| f.get(s).is_some_and(std::cell::Cell::get))
    }
}

/// Sets one fix switch.
pub fn set(s: Fx, v: bool) {
    #[cfg(not(test))]
    {
        if let Some(f) = FLAGS.get(s) {
            f.store(v, Ordering::Relaxed);
        }
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| {
            if let Some(c) = f.get(s) {
                c.set(v);
            }
        });
    }
}

/// Whether any fix switch is on.
#[must_use]
pub fn any() -> bool {
    (0..N).any(on)
}

/// The fix words (RUN.log, the RESULTS header, the checkpoint fingerprint; empty when none is on, so every round B print stays byte for byte).
#[must_use]
pub fn words() -> String {
    if !any() {
        return String::new();
    }
    let w = |s: Fx, a: String, b: &str| if on(s) { a } else { b.to_owned() };
    [
        w(LV_CAUSAL, format!("screener levels CAUSAL (each day's L1-L4 / V1-V3 from the previous {} sessions only; first session = warm-up) (check-2r M1, check-1r M2r)", LV_WINDOW.load(Ordering::Relaxed)), "screener levels over every run day (round B)"),
        w(CAP_MIN, format!("Rs 20 L cap checked minute by minute on the first {} accounts of every pick's view and the verdict view (margin + running MTM loss vs Rs 20 L + min(0, realized)); breaching accounts and portfolios EXCLUDED; the summary tables top-v3.tsv / splits18.tsv not checked (check-1r M1r)", CAP_K.load(Ordering::Relaxed)), "per-minute cap printed only (round B)"),
        w(CHG33, "dated SEBI 50% fee 2020-06-01..2021-03-31; stamp duty before 2020-07-01 labelled UNVERIFIED (check-1r M3r)".to_owned(), "SEBI fee and stamp as round B"),
        w(WORDS33, "split / side words with the segment's N grid; data vendor and period from the day manifests (check-2r M2)".to_owned(), "words as round B"),
        w(STEP_DM, "day-major step timing from sampled account-day walks (check-2r M3)".to_owned(), "no day-major step timing (round B)"),
    ]
    .join("; ")
}

/// The fix part of the day-major checkpoint fingerprint (empty when none is on: round B's and v3.3's fingerprints unchanged).
#[must_use]
pub fn fp_part() -> String {
    if any() { format!("; fix33 {}", words()) } else { String::new() }
}

/// Reads the fix switches (`--fix33` = every one; each flag may override).
///
/// # Errors
/// A switch value that is not one of its words, or a window / K outside its range.
pub fn from_args(a: &[String]) -> Result<(), String> {
    let all = a.iter().any(|x| x == "--fix33");
    let flag = |k: &str, yes: &str, no: &str| -> Result<Option<bool>, String> {
        match a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str) {
            None => Ok(None),
            Some(v) if v == yes => Ok(Some(true)),
            Some(v) if v == no => Ok(Some(false)),
            Some(v) => Err(format!("{k} {yes}|{no}, not {v}")),
        }
    };
    for (s, k, yes, no) in [(LV_CAUSAL, "--levels", "causal", "run"), (CAP_MIN, "--cap-minutes", "strict", "print"), (CHG33, "--charges33", "on", "off"), (WORDS33, "--words33", "on", "off"), (STEP_DM, "--step-dm", "sampled", "off")] {
        set(s, flag(k, yes, no)?.unwrap_or(all));
    }
    if let Some(v) = a.iter().position(|x| x == "--levels-window").and_then(|i| a.get(i + 1)) {
        let w = v.parse::<usize>().map_err(|_| format!("--levels-window {v}"))?;
        if !(1..=1_000).contains(&w) {
            return Err(format!("--levels-window {v}: 1..1000 sessions"));
        }
        LV_WINDOW.store(w, Ordering::Relaxed);
    }
    if let Some(v) = a.iter().position(|x| x == "--cap-k").and_then(|i| a.get(i + 1)) {
        let k = v.parse::<usize>().map_err(|_| format!("--cap-k {v}"))?;
        if !(1..=200).contains(&k) {
            return Err(format!("--cap-k {v}: 1..200 accounts"));
        }
        CAP_K.store(k, Ordering::Relaxed);
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------ LV_CAUSAL (check-2r M1, check-1r M2r)

/// One segment-day's screener level samples: per timeframe (member lots, member values), each SORTED ascending (round B's
/// `report::level_samples`, then sorted once).
pub type DaySmp = Vec<(Vec<u64>, Vec<u64>)>;

/// Sorts one day's samples (each timeframe's lots and values ascending).
#[must_use]
pub fn sort_samples(mut s: DaySmp) -> DaySmp {
    for (a, b) in &mut s {
        a.sort_unstable();
        b.sort_unstable();
    }
    s
}

/// The k-th smallest value (0-based, k < the total) of the union of sorted slices: the smallest x with count(<= x) > k (binary search on the
/// value; each count is a sum of partition points).
fn kth_union(parts: &[&[u64]], k: u64) -> u64 {
    let lo0 = parts.iter().filter_map(|p| p.first()).copied().min().unwrap_or(0);
    let hi0 = parts.iter().filter_map(|p| p.last()).copied().max().unwrap_or(0);
    let (mut lo, mut hi) = (lo0, hi0);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let c: u64 = parts.iter().map(|p| u64::try_from(p.partition_point(|&v| v <= mid)).unwrap_or(0)).sum();
        if c > k {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

/// Round B's percentile (`report::pctile`: sort, then index (n - 1) x q / 100) over the union of sorted slices, exactly; 0 when empty.
#[must_use]
pub fn pct_union(parts: &[&[u64]], q: u64) -> u64 {
    let n: u64 = parts.iter().map(|p| u64::try_from(p.len()).unwrap_or(0)).sum();
    if n == 0 {
        return 0;
    }
    kth_union(parts, (n - 1) * q / 100)
}

/// Round B's `report::levels_from` over the union of several days' sorted samples: the same percentiles (L 25/50/75/90, V 25/50/75), the same
/// rounding to two significant digits, the same sample counts.
#[must_use]
pub fn levels_of(days: &[&DaySmp]) -> Levels {
    let mut lv = Levels::default();
    for ti in 0..TFS.len() {
        let lots: Vec<&[u64]> = days.iter().filter_map(|d| d.get(ti)).map(|x| x.0.as_slice()).collect();
        let vals: Vec<&[u64]> = days.iter().filter_map(|d| d.get(ti)).map(|x| x.1.as_slice()).collect();
        if let Some(s) = lv.samples.get_mut(ti) {
            *s = lots.iter().map(|p| u64::try_from(p.len()).unwrap_or(0)).sum();
        }
        let q = |p: &[&[u64]], x: u64| crate::report::r2(pct_union(p, x));
        if let (Some(a), Some(b)) = (lv.minvol.get_mut(ti), lv.minval.get_mut(ti)) {
            *a = [q(&lots, 25), q(&lots, 50), q(&lots, 75), q(&lots, 90)];
            *b = [0, q(&vals, 25), q(&vals, 50), q(&vals, 75)];
        }
    }
    lv
}

/// The WARM-UP levels (no earlier session in the run, so no level is known): no member can meet a minimum volume, so the screener lists no
/// event that day (counted as a warm-up session).
#[must_use]
pub fn warm_levels() -> Levels {
    Levels { minvol: [[u64::MAX; 4]; 10], minval: [[0, u64::MAX, u64::MAX, u64::MAX]; 10], samples: [0; 10] }
}

/// The CAUSAL levels of one segment, per RUN day: `days[d]` = that day's sorted samples when the segment is present on run day d. Returns per
/// run day Some((levels, sessions used)) for a present day with at least one earlier present day (its levels from the `w` present days just
/// before it, never itself or a later day), None for an absent day or the warm-up day (the first present day). Days are computed in parallel
/// (each from its own window).
#[must_use]
pub fn causal_levels(days: &[Option<DaySmp>], w: usize, threads: usize) -> Vec<Option<(Levels, usize)>> {
    let present: Vec<usize> = days.iter().enumerate().filter(|(_, d)| d.is_some()).map(|(i, _)| i).collect();
    let mut jobs: Vec<(usize, Vec<usize>)> = Vec::with_capacity(present.len());
    for (k, &d) in present.iter().enumerate() {
        if k == 0 {
            continue;
        }
        // MUTANT m_v33f_levels_whole_run: every day's levels from EVERY present day of the run (round B's look-ahead)
        let win: Vec<usize> = if cfg!(feature = "m_v33f_levels_whole_run") { present.clone() } else { present.get(k.saturating_sub(w.max(1))..k).map(<[usize]>::to_vec).unwrap_or_default() };
        jobs.push((d, win));
    }
    let mut out: Vec<Option<(Levels, usize)>> = vec![None; days.len()];
    let nt = threads.clamp(1, 16);
    let per = jobs.len().div_ceil(nt).max(1);
    let res: Vec<Vec<(usize, Levels, usize)>> = std::thread::scope(|s| {
        let hs: Vec<_> = jobs
            .chunks(per)
            .map(|ch| {
                s.spawn(move || {
                    ch.iter()
                        .map(|(d, win)| {
                            let refs: Vec<&DaySmp> = win.iter().filter_map(|&i| days.get(i).and_then(Option::as_ref)).collect();
                            (*d, levels_of(&refs), refs.len())
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    for (d, lv, n) in res.into_iter().flatten() {
        if let Some(o) = out.get_mut(d) {
            *o = Some((lv, n));
        }
    }
    out
}

/// The per-run-day levels one segment's screener uses under LV_CAUSAL (absent days and the warm-up day get [`warm_levels`]), and the counts
/// (present days, warm-up days, days with fewer than `w` earlier sessions).
#[must_use]
pub fn day_levels(cl: &[Option<(Levels, usize)>], present: &[bool], w: usize) -> (Vec<Levels>, (usize, usize, usize)) {
    let warm = warm_levels();
    let (mut np, mut nw, mut short) = (0, 0, 0);
    let v = cl
        .iter()
        .enumerate()
        .map(|(d, x)| {
            let pr = present.get(d).copied().unwrap_or(false);
            np += usize::from(pr);
            match x {
                Some((lv, n)) => {
                    short += usize::from(*n < w);
                    lv.clone()
                }
                None => {
                    nw += usize::from(pr);
                    warm.clone()
                }
            }
        })
        .collect();
    (v, (np, nw, short))
}

/// The words of a config's level dials under LV_CAUSAL (the per-day values are in `levels-causal.tsv`).
#[must_use]
pub fn level_words(mv: usize, mval: usize) -> (String, String) {
    let w = LV_WINDOW.load(Ordering::Relaxed);
    let pv = [25, 50, 75, 90];
    let pl = [0, 25, 50, 75];
    (
        format!("L{} (causal p{}, previous {w} sessions)", mv + 1, pv.get(mv).copied().unwrap_or(0)),
        if mval == 0 { "off".to_owned() } else { format!("V{mval} (causal p{}, previous {w} sessions)", pl.get(mval).copied().unwrap_or(0)) },
    )
}

// ------------------------------------------------------------------------------------------------ CAP_MIN (check-1r M1r)

/// One account's (or portfolio's) minute-by-minute cap reading: minutes above the account base, minutes above the flat Rs 20 L, the largest
/// excess over the base (paisa) with its day and minute index (09:15 + b), and the minutes read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapRes {
    pub above_base: u32,
    pub above_flat: u32,
    pub excess: i64,
    pub day: i64,
    pub minute: u16,
    pub minutes: u32,
}

impl CapRes {
    /// Whether the cap held in every minute (against the account base, which is never above the flat Rs 20 L).
    #[must_use]
    pub const fn ok(&self) -> bool {
        self.above_base == 0
    }
}

/// check-engine-1 M1r: per trading minute (09:15-15:30, 375 a session) of every run session, the margin in use + the running MTM loss of the
/// open positions (round B's per-minute reading: a trade is open from its decision minute to the minute holding its close; its loss at the
/// kernel's mark of that minute, losses only) against the ACCOUNT BASE = Rs 20 L + min(0, the realized net of the trades closed in an
/// earlier minute) and against the flat Rs 20 L. `trades` = (entry us, close us, margin, net, day) with the mark reading.
#[must_use]
pub fn cap_minutes(trades: &[(i64, i64, i64, i64, i64, crate::v32::PMark)], marks: &[i32], days: &[i64]) -> CapRes {
    let mut r = CapRes::default();
    let mut idx: Vec<usize> = (0..trades.len()).collect();
    idx.sort_by_key(|&i| trades.get(i).map_or(0, |t| t.4));
    let mut realized = 0_i64;
    let mut j = 0_usize;
    let mut prev_day = i64::MIN;
    for &d in days {
        // trades of earlier days (or days outside the run's list) are realized before this day
        while let Some(t) = idx.get(j).and_then(|&i| trades.get(i)) {
            if t.4 >= d {
                break;
            }
            realized += t.3;
            j += 1;
        }
        let _ = prev_day;
        prev_day = d;
        let (a, b) = (crate::cal::at_sod(d, crate::cal::OPEN_S), crate::cal::at_sod(d, crate::cal::CLOSE_S));
        let mut v = [0_i64; 375];
        let mut add = [0_i64; 376];
        let mut k = j;
        while let Some(t) = idx.get(k).and_then(|&i| trades.get(i)) {
            if t.4 != d {
                break;
            }
            k += 1;
            let (te, tk, m, net, _, pm) = *t;
            let i1 = usize::try_from(((tk.min(b) - a + 60 * crate::cal::US_S - 1) / (60 * crate::cal::US_S)).max(0)).unwrap_or(0).min(375);
            if let Some(x) = add.get_mut(i1) {
                *x += net;
            }
            if tk <= a || te >= b {
                continue;
            }
            let i0 = usize::try_from(((te.max(a) - a) / (60 * crate::cal::US_S)).max(0)).unwrap_or(0);
            for (bi, x) in v.iter_mut().enumerate().take(i1).skip(i0) {
                *x += m + crate::v32::loss_at(crate::v32::mark_in_minute(marks, pm.mark1, bi), pm.units, pm.sold);
            }
        }
        j = k;
        let mut real_d = realized;
        for (bi, &x) in v.iter().enumerate() {
            real_d += add.get(bi).copied().unwrap_or(0);
            // MUTANT m_v33f_cap_flat_only: the account base read as the flat Rs 20 L (the realized losses never taken off)
            let base = crate::trade::ACCOUNT + if cfg!(feature = "m_v33f_cap_flat_only") { 0 } else { real_d.min(0) };
            r.minutes += 1;
            if x > crate::trade::ACCOUNT {
                r.above_flat += 1;
            }
            if x > base {
                r.above_base += 1;
                if x - base > r.excess {
                    r.excess = x - base;
                    r.day = d;
                    r.minute = u16::try_from(bi).unwrap_or(0);
                }
            }
        }
        realized = real_d + add.get(375).copied().unwrap_or(0);
    }
    r
}

/// The words of a cap reading.
#[must_use]
pub fn cap_words(c: &CapRes) -> String {
    if c.ok() {
        return format!("per-minute cap (margin + running MTM loss vs Rs 20 L + min(0, realized)): held in all {} minutes", c.minutes);
    }
    format!(
        "per-minute cap (margin + running MTM loss vs Rs 20 L + min(0, realized)): ABOVE in {} of {} minutes ({} above the flat Rs 20 L); largest excess Rs {} on {} at {}: EXCLUDED (check-1r M1r)",
        c.above_base,
        c.minutes,
        c.above_flat,
        crate::cal::fmt_rs(c.excess),
        crate::cal::fmt_day(c.day),
        crate::cal::fmt_sod(crate::cal::OPEN_S + 60 * i64::from(c.minute))
    )
}

// ------------------------------------------------------------------------------------------------ CHG33 (check-1r M3r)

/// The first session with a SOURCED stamp duty (the Indian Stamp Act amendment, uniform 0.003% on the buy side of options from 2020-07-01).
#[must_use]
pub const fn stamp_sourced_from() -> i64 {
    crate::cal::days_from_civil(2020, 7, 1)
}

/// Whether the SEBI fee of day `d` is the dated 50% (SEBI/LAD-NRO/GN/2020/011, NSE FA44514: June 01, 2020 to March 31, 2021).
#[must_use]
pub fn sebi_half(d: i64) -> bool {
    // MUTANT m_v33f_sebi_half_missing: the 50% window left out (round B's single 2019-04-01 step)
    !cfg!(feature = "m_v33f_sebi_half_missing") && on(CHG33) && d >= crate::cal::days_from_civil(2020, 6, 1) && d <= crate::cal::days_from_civil(2021, 3, 31)
}

// ------------------------------------------------------------------------------------------------ STEP_DM (check-2r M3)

/// The kept sampled blocks of one segment's day-major walk (a bounded uniform subsample: at [`STEP_CAP`] blocks every second one is dropped
/// and the stride doubles).
#[derive(Clone, Debug)]
pub struct StepAcc {
    pub stride: u64,
    pub blocks: Vec<(u64, u64)>,
    /// The blocks are the day-major walk's (set when the day-major stage B sampled under STEP_DM).
    pub dm: bool,
}

/// The most sampled blocks kept per segment.
pub const STEP_CAP: usize = 1 << 17;

impl Default for StepAcc {
    fn default() -> Self {
        Self { stride: 64, blocks: Vec::new(), dm: false }
    }
}

impl StepAcc {
    /// Adds one worker thread's blocks (taken at the current stride).
    pub fn merge(&mut self, b: Vec<(u64, u64)>) {
        self.blocks.extend(b);
        while self.blocks.len() > STEP_CAP {
            let v: Vec<(u64, u64)> = self.blocks.iter().step_by(2).copied().collect();
            self.blocks = v;
            self.stride = self.stride.saturating_mul(2);
        }
    }
}

/// One worker thread's sampler for one day: every `stride`-th account-day walk is timed.
#[derive(Clone, Debug)]
pub struct StepSampler {
    n: u64,
    stride: u64,
    on: bool,
    pub blocks: Vec<(u64, u64)>,
}

impl StepSampler {
    /// A sampler at the segment's current stride (inactive unless STEP_DM is on).
    #[must_use]
    pub fn new(stride: u64) -> Self {
        Self { n: 0, stride: stride.max(1), on: on(STEP_DM), blocks: Vec::new() }
    }

    /// Whether this account-day walk is timed.
    #[inline]
    pub fn want(&mut self) -> bool {
        if !self.on {
            return false;
        }
        self.n += 1;
        // MUTANT m_v33f_step_dm_unsampled: no day-major walk is ever sampled
        !cfg!(feature = "m_v33f_step_dm_unsampled") && self.n % self.stride == 0
    }

    /// Records one timed walk (walks with no step are not a block).
    #[inline]
    pub fn push(&mut self, ns: u64, steps: u64) {
        if steps > 0 {
            self.blocks.push((ns, steps));
        }
    }
}

// ------------------------------------------------------------------------------------------------ WORDS33 (check-2r M2)

/// The data vendor of one cube day from its manifest's builder / source lines ("GDFL" or "Tickvault"; "unknown" when no manifest).
#[must_use]
pub fn vendor_of(cdir: &std::path::Path, day: &str) -> &'static str {
    let Ok(t) = std::fs::read_to_string(cdir.join(format!("{day}.manifest.tsv"))) else { return "unknown (no manifest)" };
    vendor_of_text(&t)
}

/// The vendor named by a day manifest's first lines (its builder / source lines).
#[must_use]
pub fn vendor_of_text(t: &str) -> &'static str {
    let head: String = t.lines().take(3).collect::<Vec<_>>().join("\n");
    // MUTANT m_v33f_vendor_fixed: every day labelled Tickvault (round B's fixed words)
    if cfg!(feature = "m_v33f_vendor_fixed") {
        return "Tickvault";
    }
    if head.contains("GDFL") {
        "GDFL"
    } else if head.contains("tickvault") || head.contains("spikefade-cube v1 (") {
        "Tickvault"
    } else {
        "unknown"
    }
}

/// The run's data in words: per vendor, its sessions and first .. last day.
#[must_use]
pub fn vendor_words(cdir: &std::path::Path, days: &[String]) -> String {
    let mut by: std::collections::BTreeMap<&'static str, (usize, String, String)> = std::collections::BTreeMap::new();
    for d in days {
        let v = vendor_of(cdir, d);
        let e = by.entry(v).or_insert((0, d.clone(), d.clone()));
        e.0 += 1;
        e.2.clone_from(d);
    }
    by.iter().map(|(v, (n, a, b))| format!("{v} {n} sessions {a} .. {b}")).collect::<Vec<_>>().join("; ")
}

/// check-engine-2 M2: the slot count N printed in the split / side words: the segment's own grid (index 5 / 10 / 15 / 20 under NGRID) under
/// WORDS33; round B printed v3.1's 5 / 10 / 20 / 40 for both segments (`acct::n_of`).
#[must_use]
pub fn n_words(seg: usize, ni: usize) -> i64 {
    // MUTANT m_v33f_words_n_v31: v3.1's grid in the words (round B's report3 `n_of`)
    if on(WORDS33) && !cfg!(feature = "m_v33f_words_n_v31") { crate::acct::n_of_s(seg, ni) } else { crate::acct::n_of(ni) }
}

/// WORDS33: the run's vendor words (set once by `run3` from the day manifests).
pub static VENDOR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// CHG33: the run's sessions before 2020-07-01 (the unverified stamp duty reading), set once by `run3`.
pub static PRE_STAMP: AtomicUsize = AtomicUsize::new(0);

/// CAP_MIN: accounts checked minute by minute and accounts excluded, over every segment of the run.
pub static CAP_CHECKED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static CAP_EXCLUDED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// CAP_MIN: s26 portfolio additions refused for a minute above the account base.
pub static CAP_PORT_REFUSED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The CAP_MIN counts in words (checklist item 7).
#[must_use]
pub fn cap_line() -> String {
    format!(
        "this run: {} accounts checked minute by minute, {} EXCLUDED for a minute above the base, {} s26 portfolio additions refused for it",
        CAP_CHECKED.load(Ordering::Relaxed),
        CAP_EXCLUDED.load(Ordering::Relaxed),
        CAP_PORT_REFUSED.load(Ordering::Relaxed)
    )
}

/// check-engine-1 M3r: a pick's charges words under CHG33: its trades on SEBI's dated 50% window and its trades before 2020-07-01, whose stamp
/// duty is the UNVERIFIED conservative reading (0.02% on both sides; the state rates of that time are not sourced on disk).
#[must_use]
pub fn charges_words(trade_days: &[i64]) -> String {
    let half = trade_days.iter().filter(|&&d| sebi_half(d)).count();
    // MUTANT m_v33f_stamp_label_off: the unverified stamp duty not named
    let pre = if cfg!(feature = "m_v33f_stamp_label_off") { 0 } else { trade_days.iter().filter(|&&d| d < stamp_sourced_from()).count() };
    let mut s = format!("- SEBI fee at the DATED 50% (SEBI/LAD-NRO/GN/2020/011, NSE FA44514, 2020-06-01..2021-03-31) on {half} of {} trades.\n", trade_days.len());
    if pre > 0 {
        s.push_str(&format!("- Stamp duty UNVERIFIED on {pre} of {} trades (sessions before 2020-07-01): the conservative 0.02% on BOTH sides is in their net (no source for the state rates of that time on disk); checklist item 8 is PARTIAL for these sessions.\n", trade_days.len()));
    }
    s
}

/// check-engine-2 M3: the per-step timing words of a segment under STEP_DM (p50 / p99 x 1,000 from `kern::step_weighted`): "NOT MEASURED"
/// with zero sampled blocks, never 0.000.
#[must_use]
pub fn step_words(p50: u64, p99: u64, ns: usize, acc: &StepAcc) -> String {
    if ns == 0 {
        return "NOT MEASURED (0 sampled step blocks; check-engine-2 M3)".to_owned();
    }
    let src = if acc.dm { format!("sampled account-day walks of the DAY-MAJOR stage B (every {}th per worker thread, a bounded uniform subsample)", acc.stride) } else { "sampled step blocks of the stage B kernel".to_owned() };
    format!("p50 {}.{:03} / p99 {}.{:03} ns (step-weighted over {ns} {src}, wall clock, MEASURED)", p50 / 1_000, p50 % 1_000, p99 / 1_000, p99 % 1_000)
}
