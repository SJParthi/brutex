//! The host's streams and day facts: Zerodha candles of one timeframe (undone from Kite's back-adjustment, K13), the
//! 4-minute fold of Zerodha's 1-minute candles, the red source (the store's GDFL 1-s bars, LTQ0-HL-1 present), the
//! per-session facts (official H / L / C, the first and last trade) and the levels of every session (daily, weekly,
//! monthly pivots and CPR through K13 X2's factors, only events with u < e <= t).

use std::collections::BTreeMap;
use std::path::Path;

use brutex_core::vendor::Vendor;
use common_feat::ca::{CaBook, through, undo_factor};
use common_feat::cal::{ist_day, month_key, week_key};
use common_feat::fold::{Candle, Fold, Pushed};
use common_feat::levels::{Hlc, Pivots};
use common_feat::num::Ratio;

use crate::data::{Bhav, Session, tick_on};
use crate::zstore::{read_range, tf_of};

/// Everything the host reads besides the bars.
pub struct Ctx {
    /// The dated session table.
    pub sessions: Vec<Session>,
    /// Corporate actions.
    pub ca: CaBook,
    /// Tick sizes.
    pub ticks: Vec<(i32, i64)>,
    /// Official daily rows.
    pub bhav: BTreeMap<i32, Bhav>,
    /// The first session any candle reading reads (the CONT start, 2018-09-07: after the demerger week).
    pub cont_start: i32,
}

impl Ctx {
    /// The session index of an IST day.
    #[must_use]
    pub fn index_of(&self, day: i32) -> Option<usize> {
        self.sessions.binary_search_by_key(&day, |s| s.day).ok()
    }
}

/// One timeframe's candles with their session index, oldest first.
#[derive(Clone, Debug, Default)]
pub struct Stream {
    /// Candle length, seconds.
    pub tf_s: i64,
    /// The source label printed on every result.
    pub source: String,
    /// (session index, candle).
    pub candles: Vec<(usize, Candle)>,
}

/// What loading a stream counted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadStats {
    /// Bars read.
    pub bars: u64,
    /// Bars on a day that is not a session of the table (named in `notes`).
    pub not_session: u64,
    /// Bars whose interval overlaps no segment of their session (refused).
    pub outside_segment: u64,
    /// Bars that START outside every segment (before a special session's open, in a DR gap) but overlap one: KEPT,
    /// placed on the segment they overlap (REPAIR 1, B2; counted, never dropped).
    pub straddle_kept: u64,
    /// Bars before the CONT start.
    pub before_start: u64,
    /// Bars undone from a Kite back-adjustment.
    pub undone: u64,
    /// Bars that failed the candle sanity check after the undo (refused).
    pub insane: u64,
    /// Notes (missing months, not-a-session days).
    pub notes: Vec<String>,
    /// S4 (r3): intraday candles of a MUHURAT session dropped from a signal stream whose read row says
    /// `muhurat_signals excluded` (plan v3.1 s4.4 / Q39: no entry, no flip, no indicator update). Always 0 on the CASH rows.
    pub muhurat_excluded: u64,
}

/// S4 (r3, wave 4; plan v3.1 N5): WHAT a Zerodha stream reads, as data (the instrument rows' `segment`, `zerodha_symbol`, read span
/// and `muhurat_signals`, `crate::instr`), never hard-coded. [`zerodha_stream`] is this read with ADANIENT's CASH row
/// (`CASH`, 2018-01..2026-11, Muhurat candles kept), so every ADANIENT reading stays byte-identical.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZRead<'a> {
    /// The store segment (`CASH`, `INDEX`).
    pub segment: &'a str,
    /// The store symbol (`ADANIENT`, `NIFTY`, `BANKNIFTY`, `INDIAVIX`).
    pub symbol: &'a str,
    /// The first month read.
    pub from: (u16, u8),
    /// The last month read.
    pub to: (u16, u8),
    /// Q39: drop the intraday candles of MUHURAT sessions (index rows); the 1day candles are no signal stream and stay.
    pub muhurat_excluded: bool,
}

impl<'a> ZRead<'a> {
    /// ADANIENT's (and every r2 caller's) read: CASH, 2018-01..2026-11, Muhurat candles kept.
    #[must_use]
    pub const fn cash(symbol: &'a str) -> Self {
        Self { segment: "CASH", symbol, from: (2018, 1), to: (2026, 11), muhurat_excluded: false }
    }
}

fn place(ctx: &Ctx, c: Candle, tf_s: i64, st: &mut LoadStats, seen_bad: &mut BTreeMap<i32, u64>) -> Option<(usize, Candle)> {
    let day = ist_day(c.start);
    let Some(i) = ctx.index_of(day) else {
        st.not_session += 1;
        *seen_bad.entry(day).or_insert(0) += 1;
        return None;
    };
    if day < ctx.cont_start {
        st.before_start += 1;
        return None;
    }
    let s = ctx.sessions.get(i)?;
    // REPAIR 1 (B2): placed by OVERLAP with a segment, so a real Zerodha candle that starts before a special session's
    // open or in a DR gap is kept (k from the session open: `common_feat::cal::candle_index`). Its END is clamped to
    // the segment it overlaps. The named host mutant `m_drop_straddle` restores v1's drop (placement by its START).
    let seg = if cfg!(feature = "m_drop_straddle") {
        s.segs.iter().find(|g| c.start >= g.0 && c.start < g.1).copied()
    } else {
        s.seg_overlapping(c.start, c.start + tf_s)
    };
    let Some((seg_start, seg_end)) = seg else {
        st.outside_segment += 1;
        return None;
    };
    if c.start < seg_start {
        st.straddle_kept += 1;
    }
    Some((i, Candle { end: (c.start + tf_s).min(seg_end), ..c }))
}

/// The Zerodha candles of `tfname` (`1min` ... `60min`, `1day`), 2018-01..2026-11, placed on the session table,
/// with Kite's back-adjustment undone (`undo`): a price stamped before a re-based event maps back to the raw tick grid.
#[must_use]
pub fn zerodha_stream(root: &Path, symbol: &str, tfname: &str, ctx: &Ctx, undo: bool) -> (Stream, LoadStats) {
    zerodha_stream_read(root, &ZRead::cash(symbol), tfname, ctx, undo)
}

/// S4 (r3): [`zerodha_stream`] for any read row ([`ZRead`]: segment, symbol, span, Muhurat rule as data). The CASH row gives
/// r2's stream byte for byte (the same reads, placement and source label).
#[must_use]
pub fn zerodha_stream_read(root: &Path, r: &ZRead<'_>, tfname: &str, ctx: &Ctx, undo: bool) -> (Stream, LoadStats) {
    let mut st = LoadStats::default();
    let Some(tf) = tf_of(tfname) else {
        st.notes.push(format!("unknown timeframe {tfname}"));
        return (Stream::default(), st);
    };
    let tf_s = i64::from(tf.secs());
    let (bars, notes) = read_range(root, Vendor::Zerodha, r.segment, r.symbol, tf, r.from, r.to);
    st.notes = notes;
    let mut out = Vec::with_capacity(bars.len());
    let mut bad = BTreeMap::new();
    for b in bars {
        st.bars += 1;
        let day = ist_day(b.start);
        if r.muhurat_excluded && tfname != "1day" && !cfg!(feature = "m_muhurat_kept") && ctx.index_of(day).and_then(|i| ctx.sessions.get(i)).is_some_and(|s| s.kind == "MUHURAT") {
            st.muhurat_excluded += 1;
            continue;
        }
        let mut c = b;
        if undo && let Some(k) = ctx.ca.kite_undo(day) {
            let t = tick_on(&ctx.ticks, day);
            c.open = undo_factor(c.open, k, t);
            c.high = undo_factor(c.high, k, t);
            c.low = undo_factor(c.low, k, t);
            c.close = undo_factor(c.close, k, t);
            st.undone += 1;
        }
        if undo && let Some(kv) = ctx.ca.kite_vol_undo(day) {
            c.volume = common_feat::ca::undo_volume(c.volume, kv);
        }
        if !c.sane() {
            st.insane += 1;
            continue;
        }
        if tfname == "1day" {
            if ctx.index_of(day).is_some() {
                out.push((ctx.index_of(day).unwrap_or(0), c));
            } else {
                st.not_session += 1;
            }
            continue;
        }
        if let Some(x) = place(ctx, c, tf_s, &mut st, &mut bad) {
            out.push(x);
        }
    }
    for (d, n) in bad {
        st.notes.push(format!("{} {tfname}: {n} bars on a day that is not a session of the table", common_feat::cal::ymd(d)));
    }
    let mut source = format!("zerodha {tfname} (store shadow/fib-frozen-2, BarFile generation-aware){}", if undo { ", Kite back-adjustment undone" } else { ", RAW vendor values" });
    if *r != ZRead::cash(r.symbol) {
        let _ = std::fmt::Write::write_fmt(
            &mut source,
            format_args!(
                "; read row {} {} {}-{:02}..{}-{:02}, Muhurat candles {}",
                r.segment,
                r.symbol,
                r.from.0,
                r.from.1,
                r.to.0,
                r.to.1,
                if r.muhurat_excluded { "EXCLUDED (Q39)" } else { "kept" }
            ),
        );
    }
    (Stream { tf_s, source, candles: out }, st)
}

/// Folds a stream into `tf_s`-second candles anchored at each session's open (the 4-minute family from Zerodha's
/// 1-minute candles; the seconds families from the corrected 1-s bars). O(1) a bar.
#[must_use]
pub fn fold_stream(src: &Stream, tf_s: i64, ctx: &Ctx, label: &str) -> Stream {
    let mut out = Vec::new();
    let mut f = Fold::new(tf_s);
    let mut cur_sess: Option<usize> = None;
    let mut cur_seg_end = 0i64;
    for &(i, b) in &src.candles {
        let Some(s) = ctx.sessions.get(i) else { continue };
        let Some(seg_end) = s.seg_end_of(b.start) else { continue };
        if cur_sess != Some(i) || seg_end != cur_seg_end {
            if let (Some(ci), Some(done)) = (cur_sess, f.segment(s.open(), s.open(), seg_end)) {
                out.push((ci, done));
            }
            cur_sess = Some(i);
            cur_seg_end = seg_end;
        }
        if let (Pushed::Completed(done), _) = f.push(&b)
            && let Some(ci) = cur_sess
        {
            out.push((ci, done));
        }
    }
    if let (Some(ci), Some(done)) = (cur_sess, f.flush()) {
        out.push((ci, done));
    }
    Stream { tf_s, source: format!("{label} folded to {tf_s} s from {}", src.source), candles: out }
}

/// One session day's corrected 1-s bars ([`session_seconds`]).
#[derive(Clone, Debug, Default)]
pub struct SessionSeconds {
    /// The kept in-segment corrected 1-s candles `[t, t + 1)`, in time order.
    pub candles: Vec<Candle>,
    /// Tick rows of the member.
    pub tick_rows: u64,
    /// Rows refused by the reader.
    pub bad_rows: u64,
    /// Rows stamped out of order (folded into their own second).
    pub out_of_order: u64,
    /// Kept price-only seconds (only new-price LTQ = 0 rows: volume 0) among `candles`.
    pub price_only: u64,
    /// In-segment seconds inside a market-wide halt `[from, resume)` of the session with traded volume 0: DROPPED, never a
    /// candle (REPAIR 2), counted.
    pub halt_dropped: u64,
}

/// THE corrected session-seconds read (REPAIR 2, check-build-sec.md B1 = check-build-feat-v1-r2 N1; plan 2.1 HALTS, K11,
/// SN-X35), the ONE read of every seconds family (`feat-run seconds` and the sniper host's `sec::day_seconds` both call it):
/// the member `<sym>.NSE.csv` of the session's tick-store day file ([`crate::ticks::read_member`]), the corrected 1-s fold
/// [`crate::ticks::fold_seconds`] over the whole day (LTQ = 0 new-price rows kept, DP-NEWPX), every second whose instant lies
/// inside a segment of the session as a candle `[t, t + 1)`. A second whose instant lies inside one of the session's
/// market-wide halts `[from, resume)` and whose traded volume is 0 (the re-opening call auction's INDICATIVE price) is DROPPED
/// and counted; a traded second inside a halt keeps its place, so the grid continues across the halt (SN-X35). `Ok(None)`: the
/// day holds no such member.
///
/// # Errors
/// A malformed day file (refused by the reader).
pub fn session_seconds(ticks: &Path, sym: &str, s: &Session) -> Result<Option<SessionSeconds>, String> {
    let (y, m, d) = common_feat::cal::civil(s.day);
    let member = format!("{sym}.NSE.csv");
    let Some((rows, info)) = crate::ticks::read_member(&crate::ticks::day_path(ticks, y, m, d), &member)? else { return Ok(None) };
    let (secs, out_of_order) = crate::ticks::fold_seconds(&rows, 0, 86_400);
    let d0 = common_feat::cal::at(s.day, 0, 0, 0);
    let mut out = SessionSeconds { candles: Vec::with_capacity(secs.len()), tick_rows: info.rows, bad_rows: info.bad_rows, out_of_order, ..SessionSeconds::default() };
    for x in secs {
        let t = d0 + i64::from(x.sod);
        if s.seg_end_of(t).is_none() {
            continue;
        }
        if halt_indicative(s, t, &x) {
            out.halt_dropped += 1;
            continue;
        }
        out.price_only += u64::from(x.price_only());
        out.candles.push(Candle { start: t, end: t + 1, open: x.open, high: x.high, low: x.low, close: x.close, volume: x.volume });
    }
    Ok(Some(out))
}

/// The second at instant `t` lies inside a market-wide halt `[from, resume)` of the session and holds no traded volume (its
/// only trade rows are new-price LTQ = 0 rows: the re-opening call auction's indicative price). The named mutant
/// `m_halt0_folded` folds it again (v1); `tests/repair2.rs` catches it.
fn halt_indicative(s: &Session, t: i64, x: &crate::ticks::Sec) -> bool {
    !cfg!(feature = "m_halt0_folded") && x.price_only() && s.in_halt(t)
}

/// The red source: the store's GDFL 1-s bars (traded rows only, LTQ0-HL-1 present) folded to `tf_s`.
#[must_use]
pub fn gdfl_store_stream(root: &Path, symbol: &str, tf_s: i64, ctx: &Ctx) -> (Stream, LoadStats) {
    let mut st = LoadStats::default();
    let (bars, notes) = read_range(root, Vendor::Gdfl, "CASH", symbol, store::path::Timeframe::SECOND_1, (2018, 9), (2026, 10));
    st.notes = notes;
    let mut bad = BTreeMap::new();
    let mut one = Vec::with_capacity(bars.len());
    for b in bars {
        st.bars += 1;
        if let Some(x) = place(ctx, b, 1, &mut st, &mut bad) {
            one.push(x);
        }
    }
    let s1 = Stream { tf_s: 1, source: "gdfl store 1s (traded rows only: LTQ0-HL-1 present)".into(), candles: one };
    (fold_stream(&s1, tf_s, ctx, "RED"), st)
}

/// One session's facts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DayFact {
    /// The first trade of the session (the first 1-minute candle's open), paisa.
    pub open: Option<i64>,
    /// The last trade (the last 1-minute candle's close).
    pub last: Option<i64>,
}

/// The per-session first and last trade from a 1-minute stream.
#[must_use]
pub fn day_facts(one_min: &Stream, ctx: &Ctx) -> Vec<DayFact> {
    let mut v = vec![DayFact::default(); ctx.sessions.len()];
    for &(i, c) in &one_min.candles {
        if let Some(f) = v.get_mut(i) {
            if f.open.is_none() {
                f.open = Some(c.open);
            }
            f.last = Some(c.close);
        }
    }
    v
}

/// The levels of one session (decision session t), each `None` when unknown, with the reason flags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Levels {
    /// Daily pivots from the previous session's official H / L / C.
    pub piv_d: Option<Pivots>,
    /// Daily pivots with the previous session's LAST trade as the close (beside).
    pub piv_d_last: Option<Pivots>,
    /// Weekly pivots from the previous calendar week.
    pub piv_w: Option<Pivots>,
    /// Monthly pivots from the previous calendar month.
    pub piv_m: Option<Pivots>,
    /// The previous session's official close (through the factors).
    pub pc_off: Option<i64>,
    /// The previous session's last trade.
    pub pc_last: Option<i64>,
    /// The previous session's official high and low.
    pub pdh: Option<i64>,
    /// See `pdh`.
    pub pdl: Option<i64>,
    /// This session's first trade.
    pub open: Option<i64>,
    /// The daily / weekly / monthly window spans an unproven corporate action (K13: unknown, counted).
    pub span_unknown: [bool; 3],
}

fn period_hlc(ctx: &Ctx, idx: &[usize], t: i32) -> (Option<Hlc>, bool) {
    let mut h: Option<i64> = None;
    let mut l: Option<i64> = None;
    let mut c: Option<i64> = None;
    for &j in idx {
        let Some(s) = ctx.sessions.get(j) else { return (None, false) };
        let Some(f) = ctx.ca.factor(s.day, t) else { return (None, true) };
        let Some(b) = ctx.bhav.get(&s.day) else { return (None, false) };
        let (bh, bl, bc) = (through(b.high, f), through(b.low, f), through(b.close, f));
        h = Some(h.map_or(bh, |x| x.max(bh)));
        l = Some(l.map_or(bl, |x| x.min(bl)));
        c = Some(bc);
    }
    match (h, l, c) {
        (Some(h), Some(l), Some(c)) => (Some(Hlc { h, l, c }), false),
        _ => (None, false),
    }
}

/// The levels of every session (indexed like the session table).
#[must_use]
pub fn levels(ctx: &Ctx, facts: &[DayFact]) -> Vec<Levels> {
    let n = ctx.sessions.len();
    let mut out = vec![Levels::default(); n];
    // period membership
    let wk: Vec<i32> = ctx.sessions.iter().map(|s| week_key(s.day)).collect();
    let mk: Vec<i32> = ctx.sessions.iter().map(|s| month_key(s.day)).collect();
    for i in 1..n {
        let (Some(s), Some(p)) = (ctx.sessions.get(i), ctx.sessions.get(i - 1)) else { continue };
        let t = s.day;
        let lv = out.get_mut(i);
        let Some(lv) = lv else { continue };
        lv.open = facts.get(i).and_then(|f| f.open);
        match ctx.ca.factor(p.day, t) {
            None => lv.span_unknown[0] = true,
            Some(f) => {
                if let Some(b) = ctx.bhav.get(&p.day) {
                    let hlc = Hlc { h: through(b.high, f), l: through(b.low, f), c: through(b.close, f) };
                    lv.piv_d = Some(Pivots::of(hlc));
                    lv.pc_off = Some(hlc.c);
                    lv.pdh = Some(hlc.h);
                    lv.pdl = Some(hlc.l);
                    if let Some(last) = facts.get(i - 1).and_then(|x| x.last) {
                        lv.piv_d_last = Some(Pivots::of(Hlc { c: through(last, f), ..hlc }));
                    }
                }
                lv.pc_last = facts.get(i - 1).and_then(|x| x.last).map(|x| through(x, f));
            }
        }
        // the previous calendar week / month: the latest key below this session's key
        for (slot, keys) in [(1usize, &wk), (2usize, &mk)] {
            let Some(&key) = keys.get(i) else { continue };
            let mut j = i;
            while j > 0 && keys.get(j - 1) == Some(&key) {
                j -= 1;
            }
            if j == 0 {
                continue;
            }
            let Some(&pkey) = keys.get(j - 1) else { continue };
            let mut a = j - 1;
            while a > 0 && keys.get(a - 1) == Some(&pkey) {
                a -= 1;
            }
            let idx: Vec<usize> = (a..j).collect();
            let (hlc, unk) = period_hlc(ctx, &idx, t);
            if let Some(f) = lv.span_unknown.get_mut(slot) {
                *f = unk;
            }
            let piv = hlc.map(Pivots::of);
            if slot == 1 {
                lv.piv_w = piv;
            } else {
                lv.piv_m = piv;
            }
        }
    }
    out
}

/// A ratio in basis points of `x` against `base`: 10^4 (x - base) / base.
#[must_use]
pub fn bp(x: i64, base: i64) -> Option<Ratio> {
    Ratio::new(10_000 * (i128::from(x) - i128::from(base)), i128::from(base))
}
