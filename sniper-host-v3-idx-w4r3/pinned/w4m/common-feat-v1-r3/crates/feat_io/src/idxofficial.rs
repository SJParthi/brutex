//! S4 (r3, wave 4; plan v3.1 N5, v2 s4 L-CPR / C-GAP rows): the dated official daily rows of an INDEX (NIFTY, BANKNIFTY), the
//! index's `Ctx.bhav` (pivots and CPR, C-GAP / P-CHG, G-52H / G-52L, the daily ATR of G-DRNG all read it, exactly as ADANIENT's
//! read the NSE CM bhavcopy). Read only on every input. One row per session of the dated table:
//!
//! * **close** = S1's dated official close (`INDEX-CLOSE-<IX>.tsv` column `official`: the tick store's day-end value, else the
//!   Zerodha 1day close, as S1 checked it against NSE's final settlement prices);
//! * **high / low** = the store's GDFL index seconds inside the session's segments and OUTSIDE every HALT window of the session
//!   (`ix_data.rs:3`'s rule; the generation-aware door). Where those seconds have a hole (a segment with none, or a gap above
//!   [`MAX_GAP_S`] inside a segment, counted from the segment's open and to its end, the HALT windows not counted; or kept seconds
//!   in fewer than half of the session's second slots) and the
//!   session's Zerodha 1-minute candles exist, the UNION of both (`union_gdfl_zerodha`: each covers part of the day); Zerodha
//!   alone when no second is kept (`zerodha_1min`); the holey seconds alone when no candle exists (`gdfl_1s_holey`: the Muhurat
//!   evenings); the official close alone with neither (`close_only`). Every row names its source; the other source's high / low print beside;
//! * **open** = Zerodha's FIRST 1-minute open of the session (v2 s4 C-GAP: the 1day open is 1 paisa off on 51 days), else the
//!   first GDFL second's open, else the close (named);
//! * a high / low that leaves the official close outside [low, high] is widened to it (`close_widened`, counted): the true day
//!   range always holds the close.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use brutex_core::vendor::Vendor;
use common_feat::cal::{ist_day, ymd};
use common_feat::fold::Candle;

use crate::data::Session;
use crate::host::{Ctx, ZRead, zerodha_stream_read};
use crate::zstore::{Month, read_month};

/// The largest gap (seconds) between consecutive GDFL index seconds inside a segment for which the seconds still give the day's
/// high / low; above it the Zerodha 1-minute candles do (named, counted).
pub const MAX_GAP_S: i64 = 300;

/// One session's official row with its sources.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OffRow {
    /// IST day.
    pub day: i32,
    /// Open, paisa (`None`: no source).
    pub open: Option<i64>,
    /// High.
    pub high: Option<i64>,
    /// Low.
    pub low: Option<i64>,
    /// The official close (S1).
    pub close: Option<i64>,
    /// The previous session's official close.
    pub prevclose: Option<i64>,
    /// `zerodha_1min` | `gdfl_1s` | `close_only` | `-`.
    pub open_src: &'static str,
    /// `gdfl_1s` | `zerodha_1min` | `close_only` | `-`.
    pub hl_src: &'static str,
    /// GDFL index seconds kept (inside the segments, outside the halts).
    pub gdfl_rows: u64,
    /// The largest gap inside a segment (seconds; segment edges included).
    pub max_gap_s: i64,
    /// The GDFL seconds' high / low (beside).
    pub g_hl: Option<(i64, i64)>,
    /// The Zerodha 1-minute candles' high / low (beside).
    pub z_hl: Option<(i64, i64)>,
    /// The close lay outside the source's [low, high] and the range was widened to it.
    pub close_widened: bool,
    /// S1's close source word.
    pub close_src: String,
}

/// Parses S1's `INDEX-CLOSE-<IX>.tsv`: day -> (official close, source word).
///
/// # Errors
/// A malformed row.
pub fn parse_close(text: &str) -> Result<BTreeMap<i32, (i64, String)>, String> {
    let mut m = BTreeMap::new();
    let mut head: Vec<&str> = Vec::new();
    for l in text.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        if c.first() == Some(&"day") {
            head = c;
            continue;
        }
        let col = |n: &str| head.iter().position(|h| *h == n).ok_or_else(|| format!("INDEX-CLOSE: no column {n}"));
        let (ci, cs) = (col("official")?, col("src")?);
        let d = common_feat::cal::parse_ymd(c.first().copied().unwrap_or("")).ok_or_else(|| format!("INDEX-CLOSE day {l}"))?;
        let v: i64 = c.get(ci).and_then(|x| x.parse().ok()).ok_or_else(|| format!("INDEX-CLOSE official {l}"))?;
        m.insert(d, (v, c.get(cs).copied().unwrap_or("").to_owned()));
    }
    Ok(m)
}

#[derive(Default)]
struct Acc {
    rows: u64,
    hl: Option<(i64, i64)>,
    first_open: Option<i64>,
    last_t: Option<i64>,
    max_gap: i64,
    /// Bit k set: segment k of the session holds a kept second.
    seen: u64,
}

/// Seconds of [a, b) inside the session's HALT windows.
fn halt_overlap(s: &Session, a: i64, b: i64) -> i64 {
    s.halts.iter().map(|h| (b.min(h.1) - a.max(h.0)).max(0)).sum()
}

fn kept_second(s: &Session, t: i64) -> Option<(i64, i64)> {
    if s.in_halt(t) {
        return None;
    }
    s.segs.iter().find(|g| t >= g.0 && t < g.1).copied()
}

/// Builds the official rows of index `symbol` over every session of `ctx` (the index ctx: its session table and halts).
///
/// # Errors
/// A refused store month (named).
pub fn build(root: &Path, symbol: &str, ctx: &Ctx, close: &BTreeMap<i32, (i64, String)>) -> Result<(Vec<OffRow>, Vec<String>), String> {
    let mut notes = Vec::new();
    // the Zerodha 1-minute candles (RAW: an index has no corporate action), Muhurat sessions KEPT (an official row is no signal stream)
    let zr = ZRead { segment: "INDEX", symbol, from: (2018, 1), to: (2026, 11), muhurat_excluded: false };
    let (z1, zst) = zerodha_stream_read(root, &zr, "1min", ctx, false);
    notes.extend(zst.notes.iter().map(|n| format!("zerodha: {n}")));
    let mut z_open: BTreeMap<usize, i64> = BTreeMap::new();
    let mut z_hl: BTreeMap<usize, (i64, i64)> = BTreeMap::new();
    for &(i, c) in &z1.candles {
        z_open.entry(i).or_insert(c.open);
        let e = z_hl.entry(i).or_insert((c.high, c.low));
        e.0 = e.0.max(c.high);
        e.1 = e.1.min(c.low);
    }
    // the GDFL index seconds, one month at a time (the generation-aware door)
    let mut acc: BTreeMap<usize, Acc> = BTreeMap::new();
    let (first, last) = (ctx.sessions.first().map_or(0, |s| s.day), ctx.sessions.last().map_or(0, |s| s.day));
    let (fy, fm, _) = common_feat::cal::civil(first);
    let (ly, lm, _) = common_feat::cal::civil(last);
    let (mut y, mut m) = (u16::try_from(fy).map_err(|e| e.to_string())?, u8::try_from(fm).map_err(|e| e.to_string())?);
    let end = (u16::try_from(ly).map_err(|e| e.to_string())?, u8::try_from(lm).map_err(|e| e.to_string())?);
    while (y, m) <= end {
        match read_month(root, Vendor::Gdfl, "INDEX", symbol, store::path::Timeframe::SECOND_1, y, m) {
            Month::Bars(v) => fold_month(ctx, &v, &mut acc),
            Month::Missing => notes.push(format!("gdfl {symbol} 1s {y}-{m:02}: missing")),
            Month::Refused(w) => return Err(format!("gdfl {symbol} 1s {y}-{m:02} refused: {w}")),
        }
        if m == 12 {
            y += 1;
            m = 1;
        } else {
            m += 1;
        }
    }
    let mut out = Vec::with_capacity(ctx.sessions.len());
    let mut prev: Option<i64> = None;
    for (i, s) in ctx.sessions.iter().enumerate() {
        let a = acc.remove(&i).unwrap_or_default();
        let Some((c, csrc)) = close.get(&s.day).cloned() else {
            // no official close (S1's range starts with the file of the first session): no official row, named
            notes.push(format!("{}: no S1 official close: no official row", ymd(s.day)));
            out.push(OffRow { day: s.day, open_src: "-", hl_src: "-", gdfl_rows: a.rows, g_hl: a.hl, z_hl: z_hl.get(&i).copied(), close_src: "-".to_owned(), ..OffRow::default() });
            prev = None;
            continue;
        };
        // the edge gap to the session's last segment end
        let tail = a.last_t.map_or(0, |t| s.segs.iter().find(|g| t >= g.0 && t < g.1).map_or(0, |g| g.1 - 1 - t - halt_overlap(s, t + 1, g.1)));
        // a segment without one kept second is a gap of its whole length
        let empty_seg = s.segs.iter().enumerate().filter(|(k, _)| a.seen & (1_u64 << (k % 64)) == 0).map(|(_, g)| g.1 - g.0).max().unwrap_or(0);
        let max_gap = a.max_gap.max(tail).max(empty_seg);
        let slots: i64 = s.segs.iter().map(|g| g.1 - g.0 - halt_overlap(s, g.0, g.1)).sum();
        let zhl = z_hl.get(&i).copied();
        let (hl, hl_src) = match (a.hl, zhl) {
            // dense enough too: at least half of the session's second slots (segments less halts) hold a kept second (2019-05-30
            // carries one second a minute: no gap above the limit, but the minute's extremes are not in it)
            (Some(g), _) if max_gap <= MAX_GAP_S && 2 * i64::try_from(a.rows).unwrap_or(i64::MAX) >= slots => (Some(g), "gdfl_1s"),
            // holey seconds and Zerodha candles each cover part of the day: their union (the true range holds both)
            (Some(g), Some(z)) => (Some((g.0.max(z.0), g.1.min(z.1))), "union_gdfl_zerodha"),
            (None, Some(z)) => (Some(z), "zerodha_1min"),
            (Some(g), None) => (Some(g), "gdfl_1s_holey"),
            (None, None) => (Some((c, c)), "close_only"),
        };
        let (open, open_src) = match (z_open.get(&i).copied(), a.first_open) {
            (Some(o), _) => (Some(o), "zerodha_1min"),
            (None, Some(o)) => (Some(o), "gdfl_1s"),
            (None, None) => (Some(c), "close_only"),
        };
        let (mut h, mut l) = hl.unwrap_or((c, c));
        let widened = c > h || c < l;
        h = h.max(c);
        l = l.min(c);
        out.push(OffRow {
            day: s.day,
            open,
            high: Some(h),
            low: Some(l),
            close: Some(c),
            prevclose: prev,
            open_src,
            hl_src,
            gdfl_rows: a.rows,
            max_gap_s: max_gap,
            g_hl: a.hl,
            z_hl: zhl,
            close_widened: widened,
            close_src: csrc,
        });
        prev = Some(c);
    }
    Ok((out, notes))
}

fn fold_month(ctx: &Ctx, v: &[Candle], acc: &mut BTreeMap<usize, Acc>) {
    let mut cur: Option<(i32, usize)> = None;
    for b in v {
        let day = ist_day(b.start);
        let i = match cur {
            Some((d, i)) if d == day => i,
            _ => match ctx.index_of(day) {
                Some(i) => {
                    cur = Some((day, i));
                    i
                }
                None => continue,
            },
        };
        let Some(s) = ctx.sessions.get(i) else { continue };
        let Some(seg) = kept_second(s, b.start) else { continue };
        if b.high <= 0 || b.low <= 0 || b.low > b.high {
            continue;
        }
        let a = acc.entry(i).or_default();
        // the gap from the previous kept second (or the segment's open) inside this segment; a new segment first closes the
        // previous segment's tail (its last kept second to its end)
        let from = match a.last_t {
            Some(t) if t >= seg.0 => t,
            Some(t) => {
                let tail = s.segs.iter().find(|g| t >= g.0 && t < g.1).map_or(0, |g| g.1 - 1 - t - halt_overlap(s, t + 1, g.1));
                a.max_gap = a.max_gap.max(tail);
                seg.0 - 1
            }
            None => seg.0 - 1,
        };
        // a HALT window inside the gap is no hole (its seconds are left out on purpose)
        a.max_gap = a.max_gap.max(b.start - from - 1 - halt_overlap(s, from + 1, b.start));
        a.rows += 1;
        if let Some(k) = s.segs.iter().position(|g| *g == seg) {
            a.seen |= 1_u64 << (k % 64);
        }
        a.first_open.get_or_insert(b.open);
        a.hl = Some(a.hl.map_or((b.high, b.low), |(h, l)| (h.max(b.high), l.min(b.low))));
        a.last_t = Some(b.start);
    }
}

/// The rows as the TSV `data::bhav_index` reads (paisa; `-` for none), with the counts in the header.
#[must_use]
pub fn to_tsv(symbol: &str, rows: &[OffRow], notes: &[String], stamp: &str) -> String {
    let n = |src: &str| rows.iter().filter(|r| r.hl_src == src).count();
    let o = |src: &str| rows.iter().filter(|r| r.open_src == src).count();
    let mut s = String::new();
    let _ = writeln!(
        s,
        "# INDEX-OFFICIAL-{symbol}-v1 (S4, wave 4, feat-run idx-official, written {stamp}): the dated official daily rows of {symbol} (paisa). close = S1 INDEX-CLOSE official; high / low = GDFL index seconds inside the segments and outside every HALT window (max gap {MAX_GAP_S} s inside a segment), else the Zerodha 1-minute candles, else the close alone; open = Zerodha's first 1-minute open, else the first GDFL second, else the close; a range that leaves the close outside is widened to it."
    );
    let _ = writeln!(
        s,
        "# rows {}; hl_src gdfl_1s {} / union_gdfl_zerodha {} / zerodha_1min {} / gdfl_1s_holey {} / close_only {}; open_src zerodha_1min {} / gdfl_1s {} / close_only {}; close_widened {}",
        rows.len(),
        n("gdfl_1s"),
        n("union_gdfl_zerodha"),
        n("zerodha_1min"),
        n("gdfl_1s_holey"),
        n("close_only"),
        o("zerodha_1min"),
        o("gdfl_1s"),
        o("close_only"),
        rows.iter().filter(|r| r.close_widened).count()
    );
    for x in notes {
        let _ = writeln!(s, "# note: {x}");
    }
    let _ = writeln!(s, "day\topen\thigh\tlow\tclose\tprevclose\topen_src\thl_src\tgdfl_rows\tmax_gap_s\tg_high\tg_low\tz_high\tz_low\tclose_widened\tclose_src");
    let f = |v: Option<i64>| v.map_or_else(|| "-".to_owned(), |x| x.to_string());
    for r in rows {
        let _ = writeln!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            ymd(r.day),
            f(r.open),
            f(r.high),
            f(r.low),
            f(r.close),
            f(r.prevclose),
            r.open_src,
            r.hl_src,
            r.gdfl_rows,
            r.max_gap_s,
            f(r.g_hl.map(|x| x.0)),
            f(r.g_hl.map(|x| x.1)),
            f(r.z_hl.map(|x| x.0)),
            f(r.z_hl.map(|x| x.1)),
            u8::from(r.close_widened),
            r.close_src
        );
    }
    s
}
