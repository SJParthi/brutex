//! DP-ZMIN reconciliation, red first (SNIPER-REQUIREMENTS section 13 (3); MAIN-SIGNED-SNIPER S1): every Zerodha
//! candle's OHLC against the CORRECTED GDFL fold (LTQ = 0 new-price rows kept) of the same interval. Every mismatch
//! beyond one tick is counted per month and listed; every minute where GDFL traded and Zerodha has no candle is named
//! (never filled by guess); every Zerodha candle with no GDFL trade is named too. Beside it, the tick decoder is held to
//! the store's own 1-s bars: on every second, the store rule (LTQ > 0 rows only) must equal the store's bar.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use brutex_core::vendor::Vendor;
use common_feat::cal::{civil, ist_day, ist_sod, month_key, ymd};
use common_feat::fold::{Candle, Fold, Pushed};

use crate::data::{Session, tick_on};
use crate::host::{Ctx, Stream};
use crate::ticks::{Sec, day_path, fold_seconds, read_member};
use crate::zstore::read_month;

/// Per (timeframe, month) counts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Candles present in both.
    pub compared: u64,
    /// O / H / L / C beyond one tick.
    pub beyond: [u64; 4],
    /// Any field beyond one tick.
    pub any: u64,
    /// All four fields equal to the paisa.
    pub exact: u64,
    /// Largest |Z - G| per field, paisa.
    pub max_abs: [i64; 4],
    /// GDFL traded in the interval, Zerodha has no candle.
    pub z_missing: u64,
    /// Zerodha has a candle, GDFL has no trade in it.
    pub g_missing: u64,
    /// Volume equal.
    pub vol_equal: u64,
}

/// Per month: the decoder against the store's 1-s bars.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoreCheck {
    /// Seconds with an LTQ > 0 row in the ticks.
    pub tick_traded: u64,
    /// Store 1-s bars.
    pub store_bars: u64,
    /// Equal on open, high, low, close and volume.
    pub equal: u64,
    /// Present in both, differing.
    pub differ: u64,
    /// In the store only.
    pub store_only: u64,
    /// In the ticks only.
    pub tick_only: u64,
    /// Seconds with only new-price LTQ = 0 rows (`price_only`).
    pub price_only: u64,
    /// Seconds whose corrected high or low differs from the store rule's (LTQ0-HL-1 touched).
    pub newpx_changed: u64,
}

/// One day's outcome.
#[derive(Default)]
struct DayOut {
    counts: Vec<Counts>,
    store: StoreCheck,
    mism: String,
    zmiss: String,
    gmiss: String,
    note: String,
    secs: Vec<Sec>,
}

fn fold_day(secs: &[Sec], s: &Session, tf: i64) -> Vec<Candle> {
    let mut f = Fold::new(tf);
    let mut out = Vec::new();
    let day0 = common_feat::cal::at(s.day, 0, 0, 0);
    for &(open, end) in &s.segs {
        if let Some(c) = f.segment(s.open(), open, end) {
            out.push(c);
        }
        for x in secs {
            let t = day0 + i64::from(x.sod);
            if t < open || t >= end {
                continue;
            }
            let b = Candle { start: t, end: t + 1, open: x.open, high: x.high, low: x.low, close: x.close, volume: x.volume };
            if let (Pushed::Completed(c), _) = f.push(&b) {
                out.push(c);
            }
        }
    }
    if let Some(c) = f.flush() {
        out.push(c);
    }
    out
}

#[allow(clippy::too_many_arguments, reason = "one day's join")]
fn compare(tfi: usize, tfname: &str, z: &[(usize, Candle)], g: &[Candle], tick: i64, day: i32, o: &mut DayOut) {
    let mut cnt = Counts::default();
    let (mut a, mut b) = (0usize, 0usize);
    let d = ymd(day);
    let hhmm = |t: i64| {
        let s = ist_sod(t);
        format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    };
    loop {
        let zc = z.get(a).map(|x| x.1);
        let gc = g.get(b).copied();
        match (zc, gc) {
            (None, None) => break,
            (Some(zc), Some(gc)) if zc.start == gc.start => {
                cnt.compared += 1;
                let zf = [zc.open, zc.high, zc.low, zc.close];
                let gf = [gc.open, gc.high, gc.low, gc.close];
                let mut bad = false;
                for k in 0..4 {
                    let diff = (zf.get(k).copied().unwrap_or(0) - gf.get(k).copied().unwrap_or(0)).abs();
                    if let Some(m) = cnt.max_abs.get_mut(k) {
                        *m = (*m).max(diff);
                    }
                    if diff > tick {
                        bad = true;
                        if let Some(x) = cnt.beyond.get_mut(k) {
                            *x += 1;
                        }
                    }
                }
                if zf == gf {
                    cnt.exact += 1;
                }
                if zc.volume == gc.volume {
                    cnt.vol_equal += 1;
                }
                if bad {
                    cnt.any += 1;
                    let _ = writeln!(
                        o.mism,
                        "{tfname}\t{d}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{tick}",
                        hhmm(zc.start),
                        zc.open,
                        zc.high,
                        zc.low,
                        zc.close,
                        gc.open,
                        gc.high,
                        gc.low,
                        gc.close,
                        zc.volume,
                        gc.volume
                    );
                }
                a += 1;
                b += 1;
            }
            (Some(zc), Some(gc)) if zc.start < gc.start => {
                cnt.g_missing += 1;
                let _ = writeln!(o.gmiss, "{tfname}\t{d}\t{}\t{}\t{}\t{}\t{}\t{}", hhmm(zc.start), zc.open, zc.high, zc.low, zc.close, zc.volume);
                a += 1;
            }
            (Some(zc), None) => {
                cnt.g_missing += 1;
                let _ = writeln!(o.gmiss, "{tfname}\t{d}\t{}\t{}\t{}\t{}\t{}\t{}", hhmm(zc.start), zc.open, zc.high, zc.low, zc.close, zc.volume);
                a += 1;
            }
            (_, Some(gc)) => {
                cnt.z_missing += 1;
                let _ = writeln!(o.zmiss, "{tfname}\t{d}\t{}\t{}\t{}\t{}\t{}\t{}", hhmm(gc.start), gc.open, gc.high, gc.low, gc.close, gc.volume);
                b += 1;
            }
        }
    }
    if let Some(x) = o.counts.get_mut(tfi) {
        *x = cnt;
    }
}

/// Per-session candle ranges of a stream.
#[must_use]
pub fn ranges(stream: &Stream, n: usize) -> Vec<(usize, usize)> {
    let mut r = vec![(0usize, 0usize); n];
    let mut k = 0usize;
    while k < stream.candles.len() {
        let i = stream.candles.get(k).map_or(0, |x| x.0);
        let a = k;
        while stream.candles.get(k).is_some_and(|x| x.0 == i) {
            k += 1;
        }
        if let Some(x) = r.get_mut(i) {
            *x = (a, k);
        }
    }
    r
}

/// One traded second: (epoch second, the store rule's (open, high, low, close), volume).
type TradedSec = (i64, (i64, i64, i64, i64), u64);

/// The reconciliation's outputs.
pub struct ReconOut {
    /// (tf index, month key) -> counts.
    pub by_month: BTreeMap<(usize, i32), Counts>,
    /// month key -> the decoder check.
    pub store: BTreeMap<i32, StoreCheck>,
    /// Mismatch rows.
    pub mism: String,
    /// Zerodha-missing rows.
    pub zmiss: String,
    /// GDFL-missing rows.
    pub gmiss: String,
    /// Day notes (no member, CRC, bad rows).
    pub notes: String,
    /// The corrected 1-s bars of the calibration day (for the seconds families' smoke check and CAL-10).
    pub calib_secs: Vec<Sec>,
}

/// Runs the reconciliation over every session, `threads` days at a time.
#[allow(clippy::too_many_lines, reason = "one pass over the months")]
#[must_use]
pub fn reconcile(
    store_root: &Path,
    tick_root: &Path,
    symbol: &str,
    ctx: &Ctx,
    tfs: &[(String, Stream)],
    threads: usize,
    calib_day: i32,
) -> ReconOut {
    let n = ctx.sessions.len();
    let rgs: Vec<Vec<(usize, usize)>> = tfs.iter().map(|(_, s)| ranges(s, n)).collect();
    let member = format!("{symbol}.NSE.csv");
    let mut out = ReconOut {
        by_month: BTreeMap::new(),
        store: BTreeMap::new(),
        mism: String::new(),
        zmiss: String::new(),
        gmiss: String::new(),
        notes: String::new(),
        calib_secs: Vec::new(),
    };
    // sessions grouped by month
    let mut months: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (i, s) in ctx.sessions.iter().enumerate() {
        months.entry(month_key(s.day)).or_default().push(i);
    }
    for (mk, idx) in &months {
        let (y, m, _) = civil(idx.first().and_then(|&i| ctx.sessions.get(i)).map_or(0, |s| s.day));
        let store_bars = match read_month(store_root, Vendor::Gdfl, "CASH", symbol, store::path::Timeframe::SECOND_1, u16::try_from(y).unwrap_or(0), u8::try_from(m).unwrap_or(1)) {
            crate::zstore::Month::Bars(v) => v,
            crate::zstore::Month::Missing => {
                let _ = writeln!(out.notes, "{y}-{m:02}\tstore 1s month missing");
                Vec::new()
            }
            crate::zstore::Month::Refused(w) => {
                let _ = writeln!(out.notes, "{y}-{m:02}\tstore 1s month refused: {w}");
                Vec::new()
            }
        };
        let mut store_by_day: BTreeMap<i32, Vec<Candle>> = BTreeMap::new();
        for b in store_bars {
            store_by_day.entry(ist_day(b.start)).or_default().push(b);
        }
        let chunks: Vec<&[usize]> = idx.chunks(idx.len().div_ceil(threads.max(1)).max(1)).collect();
        let results: Vec<Vec<(usize, DayOut)>> = std::thread::scope(|sc| {
            let hs: Vec<_> = chunks
                .iter()
                .map(|ch| {
                    let store_by_day = &store_by_day;
                    let rgs = &rgs;
                    let member = &member;
                    sc.spawn(move || {
                        let mut v = Vec::new();
                        for &i in *ch {
                            let Some(s) = ctx.sessions.get(i) else { continue };
                            let mut o = DayOut { counts: vec![Counts::default(); tfs.len()], ..DayOut::default() };
                            let (yy, mm, dd) = civil(s.day);
                            let p = day_path(tick_root, yy, mm, dd);
                            let secs = match read_member(&p, member) {
                                Ok(Some((rows, info))) => {
                                    if info.bad_rows > 0 || !info.crc_ok {
                                        let _ = writeln!(o.note, "{}\trows {}\tbad {}\tcrc_ok {}", ymd(s.day), info.rows, info.bad_rows, info.crc_ok);
                                    }
                                    let (secs, ooo) = fold_seconds(&rows, 0, 86_400);
                                    if ooo > 0 {
                                        let _ = writeln!(o.note, "{}\tout_of_order_rows {ooo}", ymd(s.day));
                                    }
                                    secs
                                }
                                Ok(None) => {
                                    let _ = writeln!(o.note, "{}\tno member {member}", ymd(s.day));
                                    Vec::new()
                                }
                                Err(e) => {
                                    let _ = writeln!(o.note, "{}\tREFUSED: {e}", ymd(s.day));
                                    Vec::new()
                                }
                            };
                            // the decoder against the store's 1-s bars (store rule: LTQ > 0 rows only)
                            let day0 = common_feat::cal::at(s.day, 0, 0, 0);
                            let empty = Vec::new();
                            let st = store_by_day.get(&s.day).unwrap_or(&empty);
                            let mut sc = StoreCheck::default();
                            let (mut a, mut b) = (0usize, 0usize);
                            let tr: Vec<TradedSec> = secs
                                .iter()
                                .filter_map(|x| x.traded.map(|t| (day0 + i64::from(x.sod), t, x.volume)))
                                .filter(|x| s.seg_end_of(x.0).is_some())
                                .collect();
                            for x in &secs {
                                if x.price_only() && s.seg_end_of(day0 + i64::from(x.sod)).is_some() {
                                    sc.price_only += 1;
                                }
                                if let Some((_, h, l, _)) = x.traded
                                    && (h != x.high || l != x.low)
                                {
                                    sc.newpx_changed += 1;
                                }
                            }
                            sc.tick_traded = u64::try_from(tr.len()).unwrap_or(0);
                            sc.store_bars = u64::try_from(st.len()).unwrap_or(0);
                            loop {
                                match (tr.get(a), st.get(b)) {
                                    (None, None) => break,
                                    (Some(t), Some(q)) if t.0 == q.start => {
                                        if t.1 == (q.open, q.high, q.low, q.close) && t.2 == q.volume {
                                            sc.equal += 1;
                                        } else {
                                            sc.differ += 1;
                                        }
                                        a += 1;
                                        b += 1;
                                    }
                                    (Some(t), Some(q)) if t.0 < q.start => {
                                        sc.tick_only += 1;
                                        a += 1;
                                    }
                                    (Some(_), None) => {
                                        sc.tick_only += 1;
                                        a += 1;
                                    }
                                    (_, Some(_)) => {
                                        sc.store_only += 1;
                                        b += 1;
                                    }
                                }
                            }
                            o.store = sc;
                            let tick = tick_on(&ctx.ticks, s.day);
                            for (tfi, (name, stream)) in tfs.iter().enumerate() {
                                let g = fold_day(&secs, s, stream.tf_s);
                                let (za, zb) = rgs.get(tfi).and_then(|r| r.get(i)).copied().unwrap_or((0, 0));
                                let z = stream.candles.get(za..zb).unwrap_or(&[]);
                                compare(tfi, name, z, &g, tick, s.day, &mut o);
                            }
                            if s.day == calib_day {
                                o.secs = secs;
                            }
                            v.push((i, o));
                        }
                        v
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
        });
        let mut all: Vec<(usize, DayOut)> = results.into_iter().flatten().collect();
        all.sort_by_key(|x| x.0);
        for (_, o) in all {
            for (tfi, c) in o.counts.iter().enumerate() {
                let e = out.by_month.entry((tfi, *mk)).or_default();
                e.compared += c.compared;
                e.any += c.any;
                e.exact += c.exact;
                e.z_missing += c.z_missing;
                e.g_missing += c.g_missing;
                e.vol_equal += c.vol_equal;
                for k in 0..4 {
                    if let (Some(x), Some(y)) = (e.beyond.get_mut(k), c.beyond.get(k)) {
                        *x += *y;
                    }
                    if let (Some(x), Some(y)) = (e.max_abs.get_mut(k), c.max_abs.get(k)) {
                        *x = (*x).max(*y);
                    }
                }
            }
            let s = out.store.entry(*mk).or_default();
            s.tick_traded += o.store.tick_traded;
            s.store_bars += o.store.store_bars;
            s.equal += o.store.equal;
            s.differ += o.store.differ;
            s.store_only += o.store.store_only;
            s.tick_only += o.store.tick_only;
            s.price_only += o.store.price_only;
            s.newpx_changed += o.store.newpx_changed;
            out.mism.push_str(&o.mism);
            out.zmiss.push_str(&o.zmiss);
            out.gmiss.push_str(&o.gmiss);
            out.notes.push_str(&o.note);
            if !o.secs.is_empty() {
                out.calib_secs = o.secs;
            }
        }
    }
    out
}
