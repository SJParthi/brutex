//! The SECONDS families (3 / 5 / 10 / 15 / 30 s; SNIPER-REQUIREMENTS s13 (2), REQ 62; RESULTS-ADANIENT-ALL items (a)-(d); MAIN
//! 2026-11-13 20:56 IST: design change (d) SIGNED). Written by the seconds builder (main session 5b507203), 2026-11-14.
//!
//! - **(a) The stream.** DP-NEWPX: each session day's corrected GDFL ticks through `feat_io::ticks::fold_seconds` (LTQ = 0
//!   new-price rows kept), every second inside a segment of the session as a 1-s candle, folded by `feat_io::host::fold_stream`:
//!   exactly `feat-run seconds`. REPAIR ROUND (check-build-sec.md B1 = N1): the read is common_feat repair 2's ONE function
//!   `feat_io::host::session_seconds` (no second copy here), which drops every halt-window second with traded volume 0.
//!   ONE SESSION DAY AT A TIME: the 1-s source is never held whole (days read in parallel chunks,
//!   folded in order); the series of the decision timeframe and of the other seconds timeframes carry their state across days
//!   ([`crate::cands::CandBuilder`], [`crate::cands::HtfBuilder`], [`crate::cands::HtfCursor`]).
//! - **(b) The family.** The H-* dials read the OTHER timeframes of the RUN row `seconds_family` only, never a minute (REQ 62).
//! - **(c) The day facts.** C-GAP's session OPEN is GDFL's first traded print at or after the OPEN (the day's first in-segment
//!   corrected second) and P-CHG's decision price the corrected decision candle's close (plan 2.4, X1 (4)); the previous close,
//!   pivots, CPR and previous high / low stay NSE's official bhavcopy (one source for both families).
//! - **(d) Compact rows.** A candidate is never kept with its whole `Readings` (about 3 KB): each one is folded as before (a full
//!   [`crate::cands::Cand`], transient) and COMPRESSED at once into a fixed part ([`RowFix`], 40 B) and a byte row of
//!   [`RowLayout::stride`] bytes: for every wired ladder dial the boundary of its passing levels (a prefix for GE, a suffix for
//!   LE / LT, VERIFIED on every level of every row by the module's own predicate; a row that breaks it is refused), the zone codes,
//!   the pattern bits and the H-* reads (16 B a timeframe). A value's kept set is computed from these bytes when it is walked
//!   ([`SecFamily::kept`]), never stored for all values at once. SEC-EQ (`sniper-host sec-eq`, and the tests) holds every value's
//!   kept set to `cands::outer_values` on the full candidates, bit for bit.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use common_feat::fold::Candle;
use common_feat::levels::{CprZone, PivZone};
use feat_io::data::Session;
use feat_io::host::{Ctx, DayFact, Levels, Stream};
use sniper_kernel::bits::Bits;
use sniper_kernel::line::Row;

use crate::cands::{self, Cand, CandBuilder, EMA_ROWS, HtfBuilder, HtfCursor, HtfLadders, HtfRead, OuterValue, ST_ROWS, Wiring};
use crate::rows::{CoreLadders, DialRow};

/// The seconds family's timeframe names and lengths (`3s` -> 3).
#[must_use]
pub fn tf_secs(name: &str) -> Option<i64> {
    let n: i64 = name.strip_suffix('s')?.parse().ok()?;
    (1..60).contains(&n).then_some(n)
}

/// One session day's corrected 1-s candles through common_feat REPAIR 2's ONE corrected session-seconds read
/// (`feat_io::host::session_seconds`, the read `feat-run seconds` itself calls; REPAIR ROUND, check-build-sec.md B1 / F2): the
/// member `<SYM>.NSE.csv` of the day file, `fold_seconds(rows, 0, 86_400)`, every second whose instant lies inside a segment of
/// the session as a candle `[t, t + 1)`, EXCEPT a second inside a market-wide halt `[from, resume)` with traded volume 0 (the
/// re-opening auction's indicative price: dropped, counted in [`DayRead::halt_dropped`]; plan 2.1 HALTS, K11); a traded second
/// keeps its place (SN-X35). `Ok(None)`: the day holds no such member.
///
/// # Errors
/// A malformed day file (refused by the reader).
pub fn day_seconds(ticks: &Path, sym: &str, i: usize, s: &Session) -> Result<Option<DayRead>, String> {
    let Some(d) = feat_io::host::session_seconds(ticks, sym, s)? else { return Ok(None) };
    let one: Vec<(usize, Candle)> = d.candles.into_iter().map(|c| (i, c)).collect();
    Ok(Some(DayRead { one, tick_rows: d.tick_rows, bad_rows: d.bad_rows, out_of_order: d.out_of_order, price_only: d.price_only, halt_dropped: d.halt_dropped }))
}

/// The reader threads' results of one chunk: (chunk position, the day's read).
type DayGot = Vec<(usize, Result<Option<DayRead>, String>)>;

/// One day's read.
#[derive(Clone, Debug, Default)]
pub struct DayRead {
    /// The in-segment corrected 1-s candles, in time order.
    pub one: Vec<(usize, Candle)>,
    /// Tick rows of the member.
    pub tick_rows: u64,
    /// Rows refused by the reader.
    pub bad_rows: u64,
    /// Rows stamped out of order (folded into their own second).
    pub out_of_order: u64,
    /// Price-only seconds (only new-price LTQ = 0 rows: volume 0) among the kept in-segment ones.
    pub price_only: u64,
    /// In-segment seconds inside a market-wide halt with traded volume 0: dropped, never a candle (REPAIR ROUND, B1).
    pub halt_dropped: u64,
}

/// The label of the seconds stream (printed on every result).
pub const SEC_SOURCE: &str = "gdfl ticks, corrected 1-s (LTQ = 0 new-price rows kept; DP-NEWPX through feat_io::host::session_seconds = feat_io::ticks::fold_seconds + the session's segments; halt-window volume-0 seconds dropped, common_feat repair 2), read one session day at a time";

/// A ladder term's comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pat {
    /// The passing levels are a PREFIX of the ascending ladder (GE): the byte is their count.
    Prefix,
    /// The passing levels are a SUFFIX (LE, LT): the byte is the index of the first passing level (the ladder length: none).
    Suffix,
    /// No level ever passes (a comparison `cmp_ok` does not know).
    Never,
}

impl Pat {
    fn of_cmp(cmp: &str) -> Self {
        match cmp {
            "GE" => Self::Prefix,
            "LE" | "LT" => Self::Suffix,
            _ => Self::Never,
        }
    }

    /// Level `li` passes under boundary `b`.
    #[must_use]
    pub const fn pass(self, b: u8, li: u8) -> bool {
        match self {
            Self::Prefix => li < b,
            Self::Suffix => li >= b,
            Self::Never => false,
        }
    }
}

/// The boundary byte of one ladder term: evaluates `pass` on EVERY level (the module's own predicate) and verifies the pattern.
fn enc(pat: Pat, lv: &[i64], pass: impl Fn(i64) -> bool) -> Result<u8, String> {
    let b = match pat {
        Pat::Prefix => {
            let b = lv.iter().take_while(|&&x| pass(x)).count();
            if lv.iter().skip(b).any(|&x| pass(x)) {
                return Err("a GE ladder whose passing levels are not a prefix".to_string());
            }
            b
        }
        Pat::Suffix => {
            let b = lv.iter().take_while(|&&x| !pass(x)).count();
            if lv.iter().skip(b).any(|&x| !pass(x)) {
                return Err("an LE / LT ladder whose passing levels are not a suffix".to_string());
            }
            b
        }
        Pat::Never => {
            if lv.iter().any(|&x| pass(x)) {
                return Err("a comparison cmp_ok never passes passed".to_string());
            }
            0
        }
    };
    let b = if cfg!(feature = "m_sec_eq") && b > 0 && pat == Pat::Prefix { b - 1 } else { b };
    u8::try_from(b).map_err(|_| format!("a ladder of {} levels: more than 255 (refused)", lv.len()))
}

/// One wired ladder dial's terms: (pattern, comparison text, levels ascending).
#[derive(Clone, Debug, Default)]
pub struct Terms {
    /// The terms in the row's order.
    pub t: Vec<(Pat, String, Vec<i64>)>,
}

impl Terms {
    fn of(values: &str) -> Result<Self, String> {
        let mut t = Vec::new();
        for (cmp, lv) in cands::terms(values) {
            if lv.windows(2).any(|w| w.first() >= w.get(1)) {
                return Err(format!("levels not strictly ascending in {values} (refused: the boundary bytes need an ascending ladder)"));
            }
            if lv.len() > 255 {
                return Err(format!("more than 255 levels in {values}"));
            }
            t.push((Pat::of_cmp(&cmp), cmp, lv));
        }
        Ok(Self { t })
    }

    fn n(&self) -> usize {
        self.t.len()
    }
}

/// The row-level number dials of `cands::outer_values`' shared arm (two variants, long and short, when the reading is side-bound).
pub const ROW_NUM: [(&str, bool); 8] = [("C-BODY", false), ("C-UW", true), ("C-LW", true), ("C-RNG", false), ("V-RVOL", false), ("P-CHG", false), ("G-PDH", true), ("G-PDL", true)];
/// The session-level number dials (their reading depends on the session's levels only): C-GAP and L-CPRW (shared arm), L-CPRWKW and
/// L-CPRMOW (span-checked).
pub const SESS_NUM: [&str; 4] = ["C-GAP", "L-CPRW", "L-CPRWKW", "L-CPRMOW"];

/// Where every compact field lives in a row's bytes (and in a session's bytes), from the dial rows.
#[derive(Clone, Debug, Default)]
pub struct RowLayout {
    /// Bytes per row.
    pub stride: usize,
    /// Bytes per session.
    pub sess_stride: usize,
    off_st: usize,
    off_es: usize,
    off_misc: usize,
    off_zone: usize,
    off_tst: usize,
    tst: Terms,
    off_tema: usize,
    tema: Terms,
    off_vbb: usize,
    vbb: Terms,
    off_vbbw: usize,
    vbbw: Terms,
    row_num: BTreeMap<String, (usize, bool, Terms)>,
    sess_num: BTreeMap<String, (usize, Terms)>,
    off_htf: usize,
    /// H-* slots per row (the family's other timeframes).
    pub n_htf: usize,
    /// PORTER (secfd): the seven context / calendar / range dials' section, appended AFTER every other field, only when the run
    /// wires them (`--context-dials on`); `None` in the OFF mode, so the OFF layout and stride are secf-a's byte for byte.
    pub ctx: Option<CtxLay>,
}

/// PORTER (secfd): where the context dials live in a row. Number dials (X-NIFTY, X-VIX, G-52H, G-52L, G-DRNG): one boundary byte
/// per term (two variants, long then short, for G-52H / G-52L, whose short side reads the row's own mirror); D-EXP / D-MEND: one
/// flag byte (bit 0 D-EXP known, bit 1 D-EXP on, bit 2 D-MEND known, bit 3 D-MEND on; unknown is never true).
#[derive(Clone, Debug, Default)]
pub struct CtxLay {
    num: BTreeMap<String, (usize, bool, Terms)>,
    off_flags: usize,
}

const HTF_B: usize = 16;

impl RowLayout {
    /// The layout of the wired dials of `dials` with `n_htf` other timeframes.
    ///
    /// # Errors
    /// A ladder the boundary bytes cannot hold (not ascending, more than 255 levels).
    pub fn new(dials: &[DialRow], n_htf: usize) -> Result<Self, String> {
        let live = |id: &str| dials.iter().find(|d| d.id == id && d.status.starts_with("LIVE"));
        let terms = |id: &str| live(id).map_or_else(|| Ok(Terms::default()), |d| Terms::of(&d.values));
        let mut l = Self { n_htf, ..Self::default() };
        let mut at = 0_usize;
        let mut take = |n: usize| {
            let o = at;
            at += n;
            o
        };
        l.off_st = take(9);
        l.off_es = take(5);
        l.off_misc = take(5);
        l.off_zone = take(6);
        l.tst = terms("T-ST")?;
        l.off_tst = take(9 * 2 * l.tst.n());
        l.tema = terms("T-EMA")?;
        l.off_tema = take(5 * 2 * l.tema.n());
        l.vbb = terms("V-BB")?;
        l.off_vbb = take(2 * 2 * l.vbb.n());
        l.vbbw = terms("V-BBW")?;
        l.off_vbbw = take(2 * l.vbbw.n());
        for (id, two) in ROW_NUM {
            if live(id).is_some() {
                let t = terms(id)?;
                let o = take(if two { 2 } else { 1 } * t.n());
                l.row_num.insert(id.to_string(), (o, two, t));
            }
        }
        l.off_htf = take(n_htf * HTF_B);
        l.stride = at;
        let mut s_at = 0_usize;
        for id in SESS_NUM {
            if live(id).is_some() {
                let t = terms(id)?;
                l.sess_num.insert(id.to_string(), (s_at, t.clone()));
                s_at += t.t.len();
            }
        }
        l.sess_stride = s_at;
        Ok(l)
    }

    /// PORTER (secfd): this layout with the context dials' section appended (the LIVE ones of `ctxdials::IDS`).
    ///
    /// # Errors
    /// A ladder the boundary bytes cannot hold.
    pub fn with_ctx(mut self, dials: &[DialRow]) -> Result<Self, String> {
        let live = |id: &str| dials.iter().find(|d| d.id == id && d.status.starts_with("LIVE"));
        let mut at = self.stride;
        let mut num = BTreeMap::new();
        for (id, two) in [("X-NIFTY", false), ("X-VIX", false), ("G-52H", true), ("G-52L", true), ("G-DRNG", false)] {
            if let Some(d) = live(id) {
                let t = Terms::of(&d.values)?;
                let n = if two { 2 } else { 1 } * t.n();
                num.insert(id.to_string(), (at, two, t));
                at += n;
            }
        }
        let off_flags = at;
        at += 1;
        self.stride = at;
        self.ctx = Some(CtxLay { num, off_flags });
        Ok(self)
    }

    /// PORTER (secfd): writes candidate `c`'s context-dial bytes from its readings `r` (each byte verified on every level by the
    /// very predicate of `cands::outer_values`' arm: `cmp_ok` on `ctxdials::reading`). Nothing in the OFF mode.
    ///
    /// # Errors
    /// A boundary pattern broken, or a field that does not fit.
    pub fn compress_ctx(&self, c: &Cand, r: &crate::ctxdials::CtxRead, out: &mut [u8]) -> Result<(), String> {
        let Some(cl) = &self.ctx else { return Ok(()) };
        for (id, (o, two, t)) in &cl.num {
            let n = t.n();
            for v in 0..if *two { 2 } else { 1 } {
                let reading = crate::ctxdials::reading(id, r, c.ohlc[3], v == 0);
                for (ti, (pat, cmp, lv)) in t.t.iter().enumerate() {
                    let b = enc(*pat, lv, |x| cands::cmp_ok(cmp, reading, x))?;
                    *out.get_mut(o + v * n + ti).ok_or("row layout overflow (context dials)")? = b;
                }
            }
        }
        let bit = |known: Option<bool>, k: u8, on: u8| known.map_or(0, |b| k | if b { on } else { 0 });
        *out.get_mut(cl.off_flags).ok_or("row layout overflow (context flags)")? = bit(r.exp, 1, 2) | bit(r.mend, 4, 8);
        Ok(())
    }
}

/// The fixed part of a compact row (40 B).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowFix {
    /// The candle END, epoch seconds.
    pub end: i64,
    /// ATR 14 at S (REQ 47's stop distance), `i64::MIN` when unready.
    pub atr: i64,
    /// The session index.
    pub sess: u32,
    /// The candle number in the session.
    pub k: u16,
    /// Core buckets long (RSI, CCI, %R) then short.
    pub core: [u16; 6],
}

fn zone_code(z: PivZone) -> u8 {
    match z {
        PivZone::AboveR2 => 0,
        PivZone::R1R2 => 1,
        PivZone::PR1 => 2,
        PivZone::S1P => 3,
        PivZone::S2S1 => 4,
        PivZone::BelowS2 => 5,
    }
}

const PIV_ZONES: [PivZone; 6] = [PivZone::AboveR2, PivZone::R1R2, PivZone::PR1, PivZone::S1P, PivZone::S2S1, PivZone::BelowS2];
const CPR_ZONES: [CprZone; 3] = [CprZone::Above, CprZone::Inside, CprZone::Below];

fn cpr_code(z: CprZone) -> u8 {
    match z {
        CprZone::Above => 0,
        CprZone::Inside => 1,
        CprZone::Below => 2,
    }
}

const NO_ZONE: u8 = 255;

/// The H-* read packed into 16 bytes (checked conversions; a rung or code that does not fit refuses the row).
fn htf_pack(h: Option<HtfRead>, out: &mut [u8]) -> Result<(), String> {
    let Some(h) = h else {
        out.iter_mut().for_each(|x| *x = 0);
        return Ok(());
    };
    let u8c = |v: u16| u8::try_from(v).map_err(|_| format!("an H-* rung {v} above 255"));
    let mut b = [0_u8; HTF_B];
    b[0] = u8c(h.rsi[0])?;
    b[1] = u8c(h.rsi[1])?;
    b[2] = u8c(h.wr[0])?;
    b[3] = u8c(h.wr[1])?;
    b[4] = u8c(h.bb[0][0])?;
    b[5] = u8c(h.bb[0][1])?;
    b[6] = u8c(h.bb[1][0])?;
    b[7] = u8c(h.bb[1][1])?;
    b[8..10].copy_from_slice(&h.cci[0].to_le_bytes());
    b[10..12].copy_from_slice(&h.cci[1].to_le_bytes());
    let mut w: u32 = 1 << 31;
    for (j, &s) in h.st.iter().enumerate() {
        if s > 3 {
            return Err("an H-ST code above 3".to_string());
        }
        w |= u32::from(s) << (2 * j);
    }
    for (j, &e) in h.ema.iter().enumerate() {
        if e > 3 {
            return Err("an H-EMA code above 3".to_string());
        }
        w |= u32::from(e) << (18 + 2 * j);
    }
    b[12..16].copy_from_slice(&w.to_le_bytes());
    out.copy_from_slice(&b);
    Ok(())
}

/// The H-* read back from its 16 bytes.
#[must_use]
pub fn htf_unpack(b: &[u8]) -> Option<HtfRead> {
    let g = |i: usize| b.get(i).copied().unwrap_or(0);
    let w = u32::from_le_bytes([g(12), g(13), g(14), g(15)]);
    if w >> 31 == 0 {
        return None;
    }
    let mut h = HtfRead { rsi: [u16::from(g(0)), u16::from(g(1))], wr: [u16::from(g(2)), u16::from(g(3))], bb: [[u16::from(g(4)), u16::from(g(5))], [u16::from(g(6)), u16::from(g(7))]], cci: [u16::from_le_bytes([g(8), g(9)]), u16::from_le_bytes([g(10), g(11)])], ..HtfRead::default() };
    for (j, s) in h.st.iter_mut().enumerate() {
        *s = u8::try_from((w >> (2 * j)) & 3).unwrap_or(0);
    }
    for (j, e) in h.ema.iter_mut().enumerate() {
        *e = u8::try_from((w >> (18 + 2 * j)) & 3).unwrap_or(0);
    }
    Some(h)
}

/// The number reading of a row-level dial (the very expression of `cands::outer_values`' shared arm).
fn row_reading(id: &str, c: &Cand, long: bool) -> Option<(i128, i128)> {
    let a = &c.r.anat;
    match id {
        "C-BODY" => cands::ratio(a.body_pct),
        "C-UW" => cands::ratio(if long { a.upper_pct } else { a.lower_pct }),
        "C-LW" => cands::ratio(if long { a.lower_pct } else { a.upper_pct }),
        "C-RNG" => cands::pct100(c.r.range_atr),
        "V-RVOL" => cands::pct100(c.r.rvol),
        "P-CHG" => c.lv.pc_off.and_then(|p| cands::ratio(feat_io::host::bp(c.ohlc[3], p))),
        "G-PDH" => {
            if long {
                c.lv.pdh.and_then(|p| cands::ratio(feat_io::host::bp(c.ohlc[3], p)))
            } else {
                c.lv.pdl.and_then(|p| feat_io::host::bp(c.ohlc[3], p)).map(|r| (-r.num, r.den))
            }
        }
        "G-PDL" => {
            if long {
                c.lv.pdl.and_then(|p| cands::ratio(feat_io::host::bp(c.ohlc[3], p)))
            } else {
                c.lv.pdh.and_then(|p| feat_io::host::bp(c.ohlc[3], p)).map(|r| (-r.num, r.den))
            }
        }
        _ => None,
    }
}

/// The session-level dials' pass at level `x` (the very expressions of `cands::outer_values`).
fn sess_pass(id: &str, lv: &Levels, cmp: &str, x: i64) -> bool {
    match id {
        "C-GAP" => cands::cmp_ok(cmp, lv.open.zip(lv.pc_off).and_then(|(o, p)| cands::ratio(feat_io::host::bp(o, p))), x),
        "L-CPRW" => cands::cmp_ok(cmp, lv.piv_d.and_then(|p| cands::ratio(p.cpr_width_bp())), x),
        "L-CPRWKW" | "L-CPRMOW" => {
            let span = if id == "L-CPRWKW" { 1 } else { 2 };
            if lv.span_unknown.get(span).copied().unwrap_or(true) {
                return false;
            }
            let p = if span == 1 { lv.piv_w } else { lv.piv_m };
            cands::cmp_ok(cmp, p.and_then(|p| cands::ratio(p.cpr_width_bp())), x)
        }
        _ => false,
    }
}

impl RowLayout {
    /// Compresses one full candidate (its readings, its session's row-level levels) and its H-* reads into `out` (`stride` bytes).
    ///
    /// # Errors
    /// A boundary pattern broken (never expected: the predicates are monotone), or a field that does not fit.
    #[allow(clippy::too_many_lines, reason = "one block per compact field, in the layout's order")]
    pub fn compress(&self, c: &Cand, htf: &[Option<HtfRead>], out: &mut [u8]) -> Result<RowFix, String> {
        let put = |out: &mut [u8], o: usize, v: u8| -> Result<(), String> {
            *out.get_mut(o).ok_or("row layout overflow")? = v;
            Ok(())
        };
        let close_s = i128::from(c.ohlc[3]) * common_feat::S;
        for (j, s) in c.st.iter().enumerate() {
            let v = s.map_or(0, |(up, _, f)| 1 | (u8::from(up) << 1) | (u8::from(f) << 2));
            put(out, self.off_st + j, v)?;
        }
        for (j, e) in c.ema.iter().enumerate() {
            let v = e.map_or(0, |e| match close_s.cmp(&e) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => 2,
                std::cmp::Ordering::Equal => 3,
            });
            put(out, self.off_es + j, v)?;
        }
        let col = match c.ohlc[3].cmp(&c.ohlc[0]) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Less => 2,
            std::cmp::Ordering::Equal => 0,
        };
        let a = &c.r.anat;
        let p8 = |v: u16| u8::try_from(v).map_err(|_| format!("pattern bits {v} above 255"));
        put(out, self.off_misc, col)?;
        put(out, self.off_misc + 1, c.r.weekday)?;
        put(out, self.off_misc + 2, p8(a.pat_cold)?)?;
        put(out, self.off_misc + 3, p8(a.pat_long)?)?;
        put(out, self.off_misc + 4, p8(a.pat_short)?)?;
        let close = c.ohlc[3];
        let su = |s: usize| c.lv.span_unknown.get(s).copied().unwrap_or(true);
        let zones = [
            c.lv.piv_d.map_or(NO_ZONE, |p| zone_code(p.zone(close))),
            c.lv.piv_d.map_or(NO_ZONE, |p| cpr_code(p.cpr_zone(close))),
            if su(1) { NO_ZONE } else { c.lv.piv_w.map_or(NO_ZONE, |p| zone_code(p.zone(close))) },
            if su(1) { NO_ZONE } else { c.lv.piv_w.map_or(NO_ZONE, |p| cpr_code(p.cpr_zone(close))) },
            if su(2) { NO_ZONE } else { c.lv.piv_m.map_or(NO_ZONE, |p| zone_code(p.zone(close))) },
            if su(2) { NO_ZONE } else { c.lv.piv_m.map_or(NO_ZONE, |p| cpr_code(p.cpr_zone(close))) },
        ];
        for (z, v) in zones.iter().enumerate() {
            put(out, self.off_zone + z, *v)?;
        }
        let nt = self.tst.n();
        for j in 0..9 {
            for d in 0..2_usize {
                for (ti, (pat, cmp, lv)) in self.tst.t.iter().enumerate() {
                    let b = enc(*pat, lv, |x| {
                        let Some((_, line, _)) = c.st.get(j).copied().flatten() else { return false };
                        let dist = if d == 0 { close_s - line } else { line - close_s };
                        cands::cmp_ok(cmp, Some((dist * 10_000, close_s)), x)
                    })?;
                    put(out, self.off_tst + (j * 2 + d) * nt + ti, b)?;
                }
            }
        }
        let ne = self.tema.n();
        for j in 0..5 {
            for d in 0..2_usize {
                for (ti, (pat, cmp, lv)) in self.tema.t.iter().enumerate() {
                    let b = enc(*pat, lv, |x| {
                        let Some(e) = c.ema.get(j).copied().flatten() else { return false };
                        let dist = if d == 0 { close_s - e } else { e - close_s };
                        dist >= 0 && cands::cmp_ok(cmp, Some((dist * 10_000, close_s)), x)
                    })?;
                    put(out, self.off_tema + (j * 2 + d) * ne + ti, b)?;
                }
            }
        }
        let nb = self.vbb.n();
        for j in 0..2_usize {
            for d in 0..2_usize {
                for (ti, (_, _, lv)) in self.vbb.t.iter().enumerate() {
                    // V-BB reads every term's levels as "beyond by x" (the cmp cell is not read): monotone, a prefix.
                    let b = enc(Pat::Prefix, lv, |x| c.r.bb.get(j).copied().flatten().is_some_and(|b| if d == 0 { b.above_upper_by(i128::from(x)) } else { b.below_lower_by(i128::from(x)) }))?;
                    put(out, self.off_vbb + (j * 2 + d) * nb + ti, b)?;
                }
            }
        }
        let nw = self.vbbw.n();
        for j in 0..2_usize {
            for (ti, (_, cmp, lv)) in self.vbbw.t.iter().enumerate() {
                let ge = cmp == "GE";
                let b = enc(if ge { Pat::Prefix } else { Pat::Suffix }, lv, |x| c.r.bb.get(j).copied().flatten().is_some_and(|b| b.width_ge(i128::from(x)) == ge))?;
                put(out, self.off_vbbw + j * nw + ti, b)?;
            }
        }
        for (id, (o, two, t)) in &self.row_num {
            let n = t.n();
            for v in 0..if *two { 2 } else { 1 } {
                let reading = row_reading(id, c, v == 0);
                for (ti, (pat, cmp, lv)) in t.t.iter().enumerate() {
                    let b = enc(*pat, lv, |x| cands::cmp_ok(cmp, reading, x))?;
                    put(out, o + v * n + ti, b)?;
                }
            }
        }
        for h in 0..self.n_htf {
            let o = self.off_htf + h * HTF_B;
            htf_pack(htf.get(h).copied().flatten(), out.get_mut(o..o + HTF_B).ok_or("row layout overflow (H-*)")?)?;
        }
        let core = [c.core[0][0], c.core[0][1], c.core[0][2], c.core[1][0], c.core[1][1], c.core[1][2]];
        let atr = match c.atr {
            None => i64::MIN,
            Some(a) => i64::try_from(a).ok().filter(|&x| x != i64::MIN).ok_or("an ATR outside i64 (refused)")?,
        };
        Ok(RowFix { end: c.end, atr, sess: c.sess, k: c.k, core })
    }

    /// One session's bytes (`sess_stride`) from its final levels.
    ///
    /// # Errors
    /// A boundary pattern broken.
    pub fn compress_sess(&self, lv: &Levels, out: &mut [u8]) -> Result<(), String> {
        for (id, (o, t)) in &self.sess_num {
            for (ti, (pat, cmp, levels)) in t.t.iter().enumerate() {
                let b = enc(*pat, levels, |x| sess_pass(id, lv, cmp, x))?;
                *out.get_mut(o + ti).ok_or("session layout overflow")? = b;
            }
        }
        Ok(())
    }
}

/// What building a seconds family counted (printed on the readings line and in SN1).
#[derive(Clone, Debug, Default)]
pub struct SecStats {
    /// Sessions read.
    pub days: u64,
    /// Sessions whose day file holds no member (named).
    pub days_no_member: Vec<String>,
    /// In-segment corrected 1-s bars.
    pub bars_1s: u64,
    /// Of them price-only seconds.
    pub price_only: u64,
    /// Halt-window volume-0 seconds dropped (REPAIR ROUND, B1; never in `bars_1s`).
    pub halt_dropped: u64,
    /// The sessions with a dropped second, `YYYY-MM-DD n`.
    pub halt_dropped_days: Vec<String>,
    /// Tick rows read.
    pub tick_rows: u64,
    /// Tick rows refused by the reader.
    pub bad_rows: u64,
    /// Tick rows stamped out of order.
    pub out_of_order: u64,
    /// Decision-timeframe candles.
    pub candles: u64,
    /// Other timeframes' candles (slot order).
    pub other_candles: Vec<u64>,
    /// Wall seconds reading and folding.
    pub wall_s: f64,
    /// Seconds spent compressing rows.
    pub compress_s: f64,
}

/// One seconds family: compact rows, the session table, the counts.
#[derive(Clone, Debug, Default)]
pub struct SecFamily {
    /// Candle length, seconds.
    pub tf_s: i64,
    /// The source label.
    pub source: String,
    /// The layout.
    pub lay: RowLayout,
    /// The fixed parts, oldest first (rows of one session contiguous).
    pub fix: Vec<RowFix>,
    /// The byte rows (`lay.stride` each).
    pub bytes: Vec<u8>,
    /// The session bytes (`lay.sess_stride` each, every session of the table).
    pub sess_bytes: Vec<u8>,
    /// K_tf.
    pub kmax: u16,
    /// Refused decision candles.
    pub not_entry: u64,
    /// By reason.
    pub refused: BTreeMap<String, u64>,
    /// K13 reset instants.
    pub resets: Vec<i64>,
    /// H-* reads refused across the reset, per other timeframe.
    pub htf_reset_refused: Vec<u64>,
    /// The other timeframes in slot order.
    pub htf_names: Vec<String>,
    /// GDFL day facts (first and last traded print in segment) per session.
    pub facts: Vec<DayFact>,
    /// The final levels (bhavcopy + GDFL open) per session.
    pub lv: Vec<Levels>,
    /// The counts.
    pub stats: SecStats,
    /// PORTER (secfd): the context dials' run lines and SN1 rows (`ctxdials::lines`, as `ctxdials::attach` prints them); empty in
    /// the OFF mode.
    pub ctx_lines: Vec<String>,
    /// See `ctx_lines`.
    pub ctx_sn1: Vec<(String, String)>,
}

/// A seconds family request.
#[derive(Clone, Debug)]
pub struct SecSpec {
    /// The decision timeframe (`30s` ...).
    pub tf: String,
    /// The other timeframes read by the H-* dials (empty: no H-* walked).
    pub others: Vec<String>,
    /// The first and last IST day folded (the CONT start at the earliest).
    pub from: i32,
    /// See `from`.
    pub to: i32,
    /// Reader threads.
    pub threads: usize,
    /// Sessions read per chunk (in parallel), then folded in order.
    pub chunk: usize,
}

impl SecFamily {
    /// Rows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fix.len()
    }

    /// No rows.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fix.is_empty()
    }

    /// Row `i`'s bytes.
    #[must_use]
    pub fn row(&self, i: usize) -> &[u8] {
        let s = self.lay.stride;
        self.bytes.get(i * s..(i + 1) * s).unwrap_or(&[])
    }

    /// Session `i`'s bytes.
    #[must_use]
    pub fn sess_row(&self, i: usize) -> &[u8] {
        let s = self.lay.sess_stride;
        self.sess_bytes.get(i * s..(i + 1) * s).unwrap_or(&[])
    }

    /// Resident bytes of the rows (fixed parts + byte rows + session bytes), for the memory lines.
    #[must_use]
    pub fn resident_bytes(&self) -> usize {
        self.fix.len() * std::mem::size_of::<RowFix>() + self.bytes.len() + self.sess_bytes.len()
    }

    /// The Supertrend flag byte of row `i`, row `j` of [`ST_ROWS`]: bit 0 present (ready), bit 1 up, bit 2 flip.
    #[must_use]
    pub fn st(&self, i: usize, j: usize) -> u8 {
        self.row(i).get(self.lay.off_st + j).copied().unwrap_or(0)
    }

    /// The close against EMA row `j` of [`EMA_ROWS`]: 0 unready, 1 above, 2 below, 3 equal.
    #[must_use]
    pub fn ema_sign(&self, i: usize, j: usize) -> u8 {
        self.row(i).get(self.lay.off_es + j).copied().unwrap_or(0)
    }

    /// The kernel rows of one side over the given row indices.
    #[must_use]
    pub fn kernel_rows(&self, long: bool, idx: &[u32]) -> Vec<Row> {
        let s = if long { 0 } else { 3 };
        idx.iter()
            .filter_map(|&i| self.fix.get(i as usize))
            .map(|f| {
                let g = |o: usize| f.core.get(s + o).copied().unwrap_or(0);
                Row { sess: f.sess, k: u32::from(f.k), rb: g(0), cb: g(1), wb: g(2) }
            })
            .collect()
    }
}

/// Builds one seconds family over `[spec.from, spec.to]` (sessions from the CONT start), one day at a time.
///
/// # Errors
/// A malformed day file, a refused series configuration, a broken boundary pattern, or levels that disagree.
#[allow(clippy::too_many_lines, clippy::too_many_arguments, reason = "the day loop in one place: read, fold five timeframes, decide, compress")]
pub fn build_family(ctx: &Ctx, spec: &SecSpec, ticks: &Path, sym: &str, dials: &[DialRow], core: (&CoreLadders, &CoreLadders), cut_sod: i64, lad: Option<&HtfLadders>) -> Result<SecFamily, String> {
    build_family_ctx(ctx, spec, ticks, sym, dials, core, cut_sod, lad, None)
}

/// PORTER (secfd): [`build_family`] with the seven context dials wired when `ctxp` is given (`--context-dials on`): each candidate's
/// readings are made by `ctxdials::read_one` at its END, with the day's decision-stream range so far, and compressed into the
/// layout's context section. `None` is [`build_family`] (secf-a) exactly.
///
/// # Errors
/// As [`build_family`].
#[allow(clippy::too_many_lines, clippy::too_many_arguments, reason = "the day loop in one place: read, fold five timeframes, decide, compress")]
pub fn build_family_ctx(ctx: &Ctx, spec: &SecSpec, ticks: &Path, sym: &str, dials: &[DialRow], core: (&CoreLadders, &CoreLadders), cut_sod: i64, lad: Option<&HtfLadders>, ctxp: Option<&crate::ctxdials::Prep>) -> Result<SecFamily, String> {
    let t0 = std::time::Instant::now();
    let tf_s = tf_secs(&spec.tf).ok_or_else(|| format!("{}: not a seconds timeframe", spec.tf))?;
    let others: Vec<(String, i64)> = if lad.is_some() {
        spec.others.iter().map(|n| tf_secs(n).map(|s| (n.clone(), s)).ok_or_else(|| format!("{n}: not a seconds timeframe"))).collect::<Result<_, _>>()?
    } else {
        Vec::new()
    };
    let n_sess = ctx.sessions.len();
    // Row-level levels read the bhavcopy only (previous close, pivots, CPR, previous high / low): independent of the day facts.
    let lv_bhav = feat_io::host::levels(ctx, &vec![DayFact::default(); n_sess]);
    let lay = if ctxp.is_some() { RowLayout::new(dials, others.len())?.with_ctx(dials)? } else { RowLayout::new(dials, others.len())? };
    let mut ctx_counts = crate::ctxdials::Counts::default();
    let label = format!("{}s", tf_s);
    let source = format!("{label} folded to {tf_s} s from {SEC_SOURCE}");
    let mut b = CandBuilder::new(tf_s, &source)?;
    let mut hb: Vec<HtfBuilder> = others.iter().map(|(_, s)| HtfBuilder::new(*s)).collect::<Result<_, _>>()?;
    let mut cur: Vec<HtfCursor<HtfRead>> = others.iter().map(|_| HtfCursor::new()).collect();
    let mut fam = SecFamily { tf_s, source: source.clone(), lay: lay.clone(), facts: vec![DayFact::default(); n_sess], ..SecFamily::default() };
    fam.stats.other_candles = vec![0; others.len()];
    let todo: Vec<usize> = (0..n_sess).filter(|&i| ctx.sessions.get(i).is_some_and(|s| s.day >= spec.from && s.day <= spec.to && s.day >= ctx.cont_start)).collect();
    let mut n_resets = 0_usize;
    // Reserve the rows' upper bound once (every second of every session): untouched pages are never resident, and the vectors never
    // grow by copying (a copy would hold two gigabyte buffers at once on 3 s).
    let cap = todo.len() * (usize::try_from(22_500 / tf_s).unwrap_or(0) + 2);
    fam.fix.reserve(cap);
    fam.bytes.reserve(cap * lay.stride);
    let mut buf = vec![0_u8; lay.stride];
    let mut tc = std::time::Duration::ZERO;
    for chunk in todo.chunks(spec.chunk.max(1)) {
        // read the chunk's days in parallel (the tick reader is the slow part), fold them in order
        let next = AtomicUsize::new(0);
        let got: Mutex<DayGot> = Mutex::new(Vec::with_capacity(chunk.len()));
        std::thread::scope(|s| {
            for _ in 0..spec.threads.max(1).min(chunk.len()) {
                s.spawn(|| {
                    let mut local = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(&i) = chunk.get(k) else { break };
                        let Some(se) = ctx.sessions.get(i) else { continue };
                        local.push((k, day_seconds(ticks, sym, i, se)));
                    }
                    if let Ok(mut g) = got.lock() {
                        g.extend(local);
                    }
                });
            }
        });
        let mut got = got.into_inner().map_err(|_| "a reader thread panicked")?;
        got.sort_by_key(|x| x.0);
        for (k, r) in got {
            let i = *chunk.get(k).ok_or("chunk index")?;
            let se = ctx.sessions.get(i).ok_or("session")?;
            let Some(day) = r? else {
                fam.stats.days_no_member.push(common_feat::cal::ymd(se.day));
                continue;
            };
            fam.stats.days += 1;
            fam.stats.bars_1s += day.one.len() as u64;
            fam.stats.price_only += day.price_only;
            if day.halt_dropped > 0 {
                fam.stats.halt_dropped += day.halt_dropped;
                fam.stats.halt_dropped_days.push(format!("{} {}", common_feat::cal::ymd(se.day), day.halt_dropped));
            }
            fam.stats.tick_rows += day.tick_rows;
            fam.stats.bad_rows += day.bad_rows;
            fam.stats.out_of_order += day.out_of_order;
            if let Some(f) = fam.facts.get_mut(i) {
                f.open = day.one.first().map(|x| x.1.open);
                f.last = day.one.last().map(|x| x.1.close);
            }
            let s1 = Stream { tf_s: 1, source: SEC_SOURCE.to_string(), candles: day.one };
            // the other timeframes first: every candle of the day folded and queued (a cursor puts it only once a decision reaches it)
            for (h, (_, ts)) in others.iter().enumerate() {
                let st = feat_io::host::fold_stream(&s1, *ts, ctx, "h");
                if let (Some(bh), Some(ch), Some(lad)) = (hb.get_mut(h), cur.get_mut(h), lad) {
                    for &(si, c) in &st.candles {
                        let (end, v) = bh.push(ctx, si, &c, lad);
                        ch.put(end, v);
                    }
                }
                if let Some(x) = fam.stats.other_candles.get_mut(h) {
                    *x += st.candles.len() as u64;
                }
            }
            let dec = feat_io::host::fold_stream(&s1, tf_s, ctx, &label);
            fam.stats.candles += dec.candles.len() as u64;
            // PORTER (secfd): G-DRNG's range so far, kept over EVERY decision candle of the session (`ctxdials::day_range_so_far`).
            let mut rng: Option<(usize, i64, i64)> = None;
            for &(si, c) in &dec.candles {
                if ctxp.is_some() {
                    rng = Some(match rng {
                        Some((s, h, l)) if s == si => (si, h.max(c.high), l.min(c.low)),
                        _ => (si, c.high, c.low),
                    });
                }
                let idx = u32::try_from(fam.fix.len()).map_err(|_| "more than 2^32 rows")?;
                let cand = b.push(ctx, si, &c, &lv_bhav, core, cut_sod, idx);
                while let Some(&r) = b.fam.resets.get(n_resets) {
                    for ch in &mut cur {
                        ch.reset(r);
                    }
                    n_resets += 1;
                }
                let Some(cand) = cand else { continue };
                let reads: Vec<Option<HtfRead>> = cur.iter_mut().map(|ch| ch.at(cand.end)).collect();
                let tq = std::time::Instant::now();
                let fx = lay.compress(&cand, &reads, &mut buf).map_err(|e| format!("{} {} k {}: {e}", spec.tf, common_feat::cal::ymd(cand.day), cand.k))?;
                if let Some(p) = ctxp {
                    // PORTER (secfd): the candidate IS this stream candle (same session, same END), as `ctxdials::attach` matches it.
                    let Some((_, h, l)) = rng.filter(|x| x.0 == si && cand.sess as usize == si && cand.end == c.end) else {
                        return Err(format!("{} {}: a candidate without its stream candle (the day range so far)", spec.tf, common_feat::cal::ymd(cand.day)));
                    };
                    let r = crate::ctxdials::read_one(p, si, cand.day, cand.end, h, l);
                    ctx_counts.add(cand.day, &r);
                    lay.compress_ctx(&cand, &r, &mut buf).map_err(|e| format!("{} {} k {}: {e}", spec.tf, common_feat::cal::ymd(cand.day), cand.k))?;
                }
                tc += tq.elapsed();
                fam.fix.push(fx);
                fam.bytes.extend_from_slice(&buf);
            }
        }
    }
    if let Some(p) = ctxp {
        let (l, s) = crate::ctxdials::lines(p, &ctx_counts, n_sess);
        fam.ctx_lines = l;
        fam.ctx_sn1 = s;
    }
    fam.kmax = b.fam.kmax;
    fam.not_entry = b.fam.not_entry;
    fam.refused = b.fam.refused;
    fam.resets = b.fam.resets;
    fam.htf_reset_refused = cur.iter().map(HtfCursor::reset_refused).collect();
    fam.htf_names = others.iter().map(|x| x.0.clone()).collect();
    // (c): the final levels with GDFL's day facts; every row-level field must equal the bhavcopy-only levels the rows read.
    let lv = feat_io::host::levels(ctx, &fam.facts);
    for (i, (a, z)) in lv.iter().zip(&lv_bhav).enumerate() {
        if a.piv_d != z.piv_d || a.pc_off != z.pc_off || a.pdh != z.pdh || a.pdl != z.pdl || a.piv_w != z.piv_w || a.piv_m != z.piv_m || a.span_unknown != z.span_unknown {
            return Err(format!("session {i}: the row-level levels depend on the day facts (refused)"));
        }
    }
    let mut sb = vec![0_u8; lay.sess_stride * n_sess];
    for (i, l) in lv.iter().enumerate() {
        let o = i * lay.sess_stride;
        lay.compress_sess(l, sb.get_mut(o..o + lay.sess_stride).ok_or("session bytes")?)?;
    }
    fam.sess_bytes = sb;
    fam.lv = lv;
    fam.stats.wall_s = t0.elapsed().as_secs_f64();
    fam.stats.compress_s = tc.as_secs_f64();
    Ok(fam)
}

/// One walked value of a seconds context, evaluated from the compact bytes (`cands::outer_values`' value, by the same loops).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VSpec {
    /// C-IDX = k.
    Idx(u16),
    /// C-COL: the colour code (1 green, 2 red).
    Col(u8),
    /// K-DOW set.
    Dow(u8),
    /// A boundary byte at `off` passing level `li` (`pat`), and, for T-ST, the Supertrend flag byte's side / flip test.
    Bnd {
        /// Byte offset in the row.
        off: usize,
        /// Level index.
        li: u8,
        /// The pattern.
        pat: Pat,
        /// T-ST: (row j, want_up, flip 0 any / 1 now / 2 not); `None` for the others.
        st: Option<(usize, bool, u8)>,
    },
    /// A session byte at `off` passing level `li`.
    SBnd {
        /// Byte offset in the session bytes.
        off: usize,
        /// Level index.
        li: u8,
        /// The pattern.
        pat: Pat,
    },
    /// A zone byte (row offset) whose code is in the mask.
    Zone {
        /// Byte offset in the row.
        off: usize,
        /// Bit c set: zone code c passes.
        mask: u8,
    },
    /// C-PAT: the bit, read on the side's pattern byte (cold never passes).
    PatBit {
        /// The bit.
        bit: u8,
        /// Long side.
        long: bool,
    },
    /// H-RSI / H-CCI / H-WR (0 / 1 / 2) of slot `h`: rung at least `j`.
    HCore {
        /// Which reading.
        which: u8,
        /// The slot.
        h: usize,
        /// Side (0 long).
        side: usize,
        /// Rung.
        j: u16,
    },
    /// H-BB of slot `h`, band `b`: rung at least `j`.
    HBb {
        /// Slot.
        h: usize,
        /// Side.
        side: usize,
        /// Band row.
        b: usize,
        /// Rung.
        j: u16,
    },
    /// H-ST / H-EMA of slot `h`, row `j`: the code wanted (1 or 2).
    HSide {
        /// 0 = ST, 1 = EMA.
        ema: bool,
        /// Slot.
        h: usize,
        /// Row.
        j: usize,
        /// The code that passes.
        want: u8,
    },
    /// PORTER (secfd): D-EXP / D-MEND: the context flag byte at `off`, known bit `known`, value bit `val`, the value wanted.
    Flag {
        /// Byte offset in the row.
        off: usize,
        /// The known bit.
        known: u8,
        /// The value bit.
        val: u8,
        /// ON (true) or NOT (false).
        want: bool,
    },
}

impl SecFamily {
    fn pass(&self, v: &VSpec, i: usize) -> bool {
        let Some(f) = self.fix.get(i) else { return false };
        let row = self.row(i);
        let g = |o: usize| row.get(o).copied();
        match *v {
            VSpec::Idx(k) => f.k == k,
            VSpec::Col(c) => g(self.lay.off_misc) == Some(c),
            VSpec::Dow(set) => g(self.lay.off_misc + 1).is_some_and(|w| (1..=5).contains(&w) && (set >> (w - 1)) & 1 == 1),
            VSpec::Bnd { off, li, pat, st } => {
                let ok = st.is_none_or(|(j, want_up, flip)| {
                    g(self.lay.off_st + j).is_some_and(|b| {
                        let up = b & 2 != 0;
                        let fl = b & 4 != 0;
                        b & 1 != 0
                            && up == want_up
                            && match flip {
                                1 => fl,
                                2 => !fl,
                                _ => true,
                            }
                    })
                });
                ok && g(off).is_some_and(|b| pat.pass(b, li))
            }
            VSpec::SBnd { off, li, pat } => self.sess_row(f.sess as usize).get(off).is_some_and(|&b| pat.pass(b, li)),
            VSpec::Zone { off, mask } => g(off).is_some_and(|z| z < 8 && (mask >> z) & 1 == 1),
            VSpec::PatBit { bit, long } => {
                let cold = g(self.lay.off_misc + 2).unwrap_or(0xff);
                let side = g(self.lay.off_misc + if long { 3 } else { 4 }).unwrap_or(0);
                bit != 0 && cold & bit == 0 && side & bit != 0
            }
            VSpec::HCore { which, h, side, j } => self.htf(i, h).is_some_and(|r| {
                let b = match which {
                    0 => r.rsi,
                    1 => r.cci,
                    _ => r.wr,
                };
                b.get(side).copied().unwrap_or(0) >= j
            }),
            VSpec::HBb { h, side, b, j } => self.htf(i, h).is_some_and(|r| r.bb.get(side).and_then(|x| x.get(b)).copied().unwrap_or(0) >= j),
            VSpec::HSide { ema, h, j, want } => self.htf(i, h).is_some_and(|r| (if ema { r.ema.get(j) } else { r.st.get(j) }).copied().unwrap_or(0) == want),
            VSpec::Flag { off, known, val, want } => g(off).is_some_and(|b| b & known != 0 && (b & val != 0) == want),
        }
    }

    /// Row `i`'s read of other timeframe `h`.
    #[must_use]
    pub fn htf(&self, i: usize, h: usize) -> Option<HtfRead> {
        let o = self.lay.off_htf + h * HTF_B;
        self.row(i).get(o..o + HTF_B).and_then(htf_unpack)
    }

    /// The kept set of value `v` over the rows `idx` (the context's priced rows, in order; the first `n` of them).
    #[must_use]
    pub fn kept(&self, v: &VSpec, idx: &[u32], n: usize) -> Bits {
        let mut b = Bits::new(n);
        for (r, &i) in idx.iter().take(n).enumerate() {
            if self.pass(v, i as usize) {
                b.set(r);
            }
        }
        b
    }
}

/// The walked values of one seconds context, in `cands::outer_values`' order with its labels, value indices and EMPTY flags (the
/// `kept` sets left empty: [`SecFamily::kept`] makes each one when it is walked), and their compact specs.
#[must_use]
#[allow(clippy::too_many_lines, reason = "one arm per wired dial, mirroring cands::outer_values")]
pub fn values(fam: &SecFamily, long: bool, dials: &[DialRow], xe: Option<u16>) -> (Vec<OuterValue>, Vec<VSpec>, Wiring) {
    let mut out = Vec::new();
    let mut specs = Vec::new();
    let mut wiring = Wiring::default();
    let lay = &fam.lay;
    let side_ix = usize::from(!long);
    for (di, d) in dials.iter().enumerate() {
        wiring.radix.push(cands::radix_of(d, fam.kmax, fam.tf_s >= 60));
        if d.id.starts_with("M-") {
            continue;
        }
        if !d.status.starts_with("LIVE") {
            wiring.held.push(format!("{} ({})", d.id, d.status));
            continue;
        }
        let ts = cands::terms(&d.values);
        let mut vi = 0_u64;
        let mut push = |label: String, spec: VSpec, empty: bool, out: &mut Vec<OuterValue>, specs: &mut Vec<VSpec>| {
            vi += 1;
            out.push(OuterValue { dial: di, value: vi, label, kept: Bits::new(0), empty });
            specs.push(spec);
        };
        let before = out.len();
        let tpos = |t: &Terms, ti: usize| t.t.get(ti).map_or(Pat::Never, |x| x.0);
        match d.id.as_str() {
            "C-IDX" => {
                for k in 1..=fam.kmax {
                    push(format!("C-IDX={k}"), VSpec::Idx(k), false, &mut out, &mut specs);
                }
            }
            "C-COL" => {
                for lab in cands::list_labels(&d.values) {
                    let green = lab == "GREEN";
                    push(format!("C-COL={lab}"), VSpec::Col(if green { 1 } else { 2 }), false, &mut out, &mut specs);
                }
            }
            "K-DOW" => {
                for set in 1_u8..31 {
                    push(format!("K-DOW={set:05b}"), VSpec::Dow(set), false, &mut out, &mut specs);
                }
            }
            "T-ST" => {
                let nt = lay.tst.n();
                for (j, _) in ST_ROWS.iter().enumerate() {
                    for side in 0..2_u8 {
                        for flip in 0..3_u8 {
                            for (ti, (cmp, lv)) in ts.iter().enumerate() {
                                let d_ix = if (side == 0) == long { 0 } else { 1 };
                                let want_up = if side == 0 { long } else { !long };
                                for (li, &x) in lv.iter().enumerate() {
                                    let spec = VSpec::Bnd { off: lay.off_tst + (j * 2 + d_ix) * nt + ti, li: u8::try_from(li).unwrap_or(u8::MAX), pat: tpos(&lay.tst, ti), st: Some((j, want_up, flip)) };
                                    push(format!("T-ST[{j},{side},{flip}] {cmp} {x}"), spec, false, &mut out, &mut specs);
                                }
                            }
                        }
                    }
                }
            }
            "T-EMA" => {
                let nt = lay.tema.n();
                for (j, &p) in EMA_ROWS.iter().enumerate() {
                    for side in 0..2_u8 {
                        let empty = xe == Some(p) && side == 1;
                        let d_ix = if (side == 0) == long { 0 } else { 1 };
                        for (ti, (cmp, lv)) in ts.iter().enumerate() {
                            for (li, &x) in lv.iter().enumerate() {
                                let spec = VSpec::Bnd { off: lay.off_tema + (j * 2 + d_ix) * nt + ti, li: u8::try_from(li).unwrap_or(u8::MAX), pat: tpos(&lay.tema, ti), st: None };
                                push(format!("T-EMA[{p},{side}] {cmp} {x}"), spec, empty, &mut out, &mut specs);
                            }
                        }
                    }
                }
            }
            "V-BB" => {
                let nb = lay.vbb.n();
                for j in 0..2_usize {
                    for (ti, (_, lv)) in ts.iter().enumerate() {
                        for (li, &x) in lv.iter().enumerate() {
                            let spec = VSpec::Bnd { off: lay.off_vbb + (j * 2 + side_ix) * nb + ti, li: u8::try_from(li).unwrap_or(u8::MAX), pat: Pat::Prefix, st: None };
                            push(format!("V-BB[{j}] beyond by {x} bp (of the band, the module's reading)"), spec, false, &mut out, &mut specs);
                        }
                    }
                }
            }
            "C-BODY" | "C-UW" | "C-LW" | "C-RNG" | "V-RVOL" | "C-GAP" | "P-CHG" | "G-PDH" | "G-PDL" | "L-CPRW" => {
                let id = d.id.clone();
                for (ti, (cmp, lv)) in ts.iter().enumerate() {
                    for (li, &x) in lv.iter().enumerate() {
                        let li8 = u8::try_from(li).unwrap_or(u8::MAX);
                        let spec = if let Some((o, t)) = lay.sess_num.get(&id) {
                            VSpec::SBnd { off: o + ti, li: li8, pat: tpos(t, ti) }
                        } else if let Some((o, two, t)) = lay.row_num.get(&id) {
                            let v = if *two && !long { 1 } else { 0 };
                            VSpec::Bnd { off: o + v * t.n() + ti, li: li8, pat: tpos(t, ti), st: None }
                        } else {
                            VSpec::Idx(0)
                        };
                        push(format!("{id} {cmp} {x}"), spec, false, &mut out, &mut specs);
                    }
                }
            }
            "L-PIV" | "L-CPR" => {
                let piv = d.id == "L-PIV";
                for lab in cands::list_labels(&d.values) {
                    let mask = if piv {
                        PIV_ZONES.iter().fold(0_u8, |m, &z| m | (u8::from(cands::piv_label(z, &lab, long)) << zone_code(z)))
                    } else {
                        CPR_ZONES.iter().fold(0_u8, |m, &z| m | (u8::from(cands::cpr_label(z, &lab, long)) << cpr_code(z)))
                    };
                    let off = lay.off_zone + if piv { 0 } else { 1 };
                    push(format!("{}={lab}{}", d.id, if long { "" } else { " (mirror)" }), VSpec::Zone { off, mask }, false, &mut out, &mut specs);
                }
            }
            "L-PIVW" | "L-PIVM" | "L-CPRWK" | "L-CPRMO" => {
                let span = if matches!(d.id.as_str(), "L-PIVW" | "L-CPRWK") { 1 } else { 2 };
                let piv = d.id.starts_with("L-PIV");
                for lab in cands::list_labels(&d.values) {
                    let mask = if piv {
                        PIV_ZONES.iter().fold(0_u8, |m, &z| m | (u8::from(cands::piv_label(z, &lab, long)) << zone_code(z)))
                    } else {
                        CPR_ZONES.iter().fold(0_u8, |m, &z| m | (u8::from(cands::cpr_label(z, &lab, long)) << cpr_code(z)))
                    };
                    let off = lay.off_zone + 2 * span + if piv { 0 } else { 1 };
                    push(format!("{}={lab}{}", d.id, if long { "" } else { " (mirror)" }), VSpec::Zone { off, mask }, false, &mut out, &mut specs);
                }
            }
            "L-CPRWKW" | "L-CPRMOW" => {
                let id = d.id.clone();
                for (ti, (cmp, lv)) in ts.iter().enumerate() {
                    for (li, &x) in lv.iter().enumerate() {
                        let li8 = u8::try_from(li).unwrap_or(u8::MAX);
                        let spec = lay.sess_num.get(&id).map_or(VSpec::Idx(0), |(o, t)| VSpec::SBnd { off: o + ti, li: li8, pat: tpos(t, ti) });
                        push(format!("{} {cmp} {x}", d.id), spec, false, &mut out, &mut specs);
                    }
                }
            }
            "V-BBW" => {
                let nw = lay.vbbw.n();
                for j in 0..2_usize {
                    for (ti, (cmp, lv)) in ts.iter().enumerate() {
                        let pat = if cmp == "GE" { Pat::Prefix } else { Pat::Suffix };
                        for (li, &x) in lv.iter().enumerate() {
                            let spec = VSpec::Bnd { off: lay.off_vbbw + j * nw + ti, li: u8::try_from(li).unwrap_or(u8::MAX), pat, st: None };
                            push(format!("V-BBW[{j}] {cmp} {x}"), spec, false, &mut out, &mut specs);
                        }
                    }
                }
            }
            "C-PAT" => {
                use common_feat::anatomy::pat;
                for lab in cands::list_labels(&d.values) {
                    let bit = match lab.as_str() {
                        "MARUBOZU" => pat::MARUBOZU,
                        "ENGULF" | "ENGULFING" => pat::ENGULFING,
                        "HAMMER" => pat::HAMMER,
                        "INSIDE" => pat::INSIDE,
                        "OUTSIDE" => pat::OUTSIDE,
                        "DOJI" => pat::DOJI,
                        "NR4" => pat::NR4,
                        "NR7" => pat::NR7,
                        _ => 0,
                    };
                    push(format!("C-PAT={lab}{}", if long { "" } else { " (mirror)" }), VSpec::PatBit { bit: u8::try_from(bit).unwrap_or(0), long }, bit == 0, &mut out, &mut specs);
                }
            }
            "H-RSI" | "H-CCI" | "H-WR" | "H-BB" | "H-ST" | "H-EMA" if lay.n_htf > 0 => {
                let side = usize::from(!long);
                for h in 0..lay.n_htf {
                    let hn = fam.htf_names.get(h).cloned().unwrap_or_default();
                    match d.id.as_str() {
                        "H-RSI" | "H-CCI" | "H-WR" => {
                            let n = ts.iter().map(|t| t.1.len()).sum::<usize>();
                            let which = match d.id.as_str() {
                                "H-RSI" => 0,
                                "H-CCI" => 1,
                                _ => 2,
                            };
                            for j in 1..=n {
                                let jj = u16::try_from(j).unwrap_or(u16::MAX);
                                push(format!("{}[{hn}] rung {j}", d.id), VSpec::HCore { which, h, side, j: jj }, false, &mut out, &mut specs);
                            }
                        }
                        "H-BB" => {
                            let n = ts.iter().map(|t| t.1.len()).sum::<usize>();
                            for b in 0..2_usize {
                                for j in 1..=n {
                                    let jj = u16::try_from(j).unwrap_or(u16::MAX);
                                    push(format!("H-BB[{hn},{b}] rung {j}"), VSpec::HBb { h, side, b, j: jj }, false, &mut out, &mut specs);
                                }
                            }
                        }
                        "H-ST" => {
                            for j in 0..9_usize {
                                for (li, _) in cands::list_labels(&d.values).iter().enumerate() {
                                    let agree = li == 0;
                                    let want = if long == agree { 1 } else { 2 };
                                    push(format!("H-ST[{hn},{j}] {}", if agree { "AGREE" } else { "AGAINST" }), VSpec::HSide { ema: false, h, j, want }, false, &mut out, &mut specs);
                                }
                            }
                        }
                        _ => {
                            for j in 0..5_usize {
                                for li in 0..cands::list_labels(&d.values).len() {
                                    let agree = li == 0;
                                    let want = if long == agree { 1 } else { 2 };
                                    push(format!("H-EMA[{hn},{}] {}", EMA_ROWS.get(j).copied().unwrap_or(0), if agree { "AGREE" } else { "AGAINST" }), VSpec::HSide { ema: true, h, j, want }, false, &mut out, &mut specs);
                                }
                            }
                        }
                    }
                }
            }
            "X-NIFTY" | "X-VIX" | "G-52H" | "G-52L" | "G-DRNG" if lay.ctx.is_some() => {
                // PORTER (secfd): `cands::outer_values`' DIALS BUILD arm: every level of the row's own terms, in the file's order,
                // read from the context section's boundary bytes (the short variant of G-52H / G-52L is the row's mirror).
                let id = d.id.clone();
                let num = lay.ctx.as_ref().and_then(|c| c.num.get(&id));
                for (ti, (cmp, lv)) in ts.iter().enumerate() {
                    for (li, &x) in lv.iter().enumerate() {
                        let li8 = u8::try_from(li).unwrap_or(u8::MAX);
                        let spec = num.map_or(VSpec::Idx(0), |(o, two, t)| {
                            let v = if *two && !long { 1 } else { 0 };
                            VSpec::Bnd { off: o + v * t.n() + ti, li: li8, pat: tpos(t, ti), st: None }
                        });
                        push(format!("{id} {cmp} {x}"), spec, false, &mut out, &mut specs);
                    }
                }
            }
            "D-EXP" | "D-MEND" if lay.ctx.is_some() => {
                // PORTER (secfd): ON / NOT from the row's list; an unknown bit is never true; a label other than ON / NOT is EMPTY.
                let exp = d.id == "D-EXP";
                let off = lay.ctx.as_ref().map_or(0, |c| c.off_flags);
                for lab in cands::list_labels(&d.values) {
                    let want = match lab.as_str() {
                        "ON" => Some(true),
                        "NOT" => Some(false),
                        _ => None,
                    };
                    let spec = want.map_or(VSpec::Idx(0), |w| VSpec::Flag { off, known: if exp { 1 } else { 4 }, val: if exp { 2 } else { 8 }, want: w });
                    push(format!("{}={lab}", d.id), spec, want.is_none(), &mut out, &mut specs);
                }
            }
            _ => {
                wiring.not_wired.push(d.id.clone());
                continue;
            }
        }
        wiring.wired.push((d.id.clone(), (out.len() - before) as u64));
    }
    (out, specs, wiring)
}

/// SEC-EQ: every value of `cands::outer_values` over a full family equals the compact family's kept set (same order, same labels,
/// same EMPTY flags), both sides and every XE context. Returns (values compared, rows, mismatches with their first labels).
#[must_use]
pub fn sec_eq(full: &cands::Family, comp: &SecFamily, dials: &[DialRow]) -> (u64, usize, Vec<String>) {
    let mut bad = Vec::new();
    let mut n = 0_u64;
    let all: Vec<usize> = (0..full.cands.len()).collect();
    let idx: Vec<u32> = (0..u32::try_from(comp.len()).unwrap_or(0)).collect();
    if full.cands.len() != comp.len() {
        bad.push(format!("rows: full {} compact {}", full.cands.len(), comp.len()));
        return (0, comp.len(), bad);
    }
    for (i, (c, f)) in full.cands.iter().zip(&comp.fix).enumerate() {
        let core = [c.core[0][0], c.core[0][1], c.core[0][2], c.core[1][0], c.core[1][1], c.core[1][2]];
        if c.end != f.end || c.sess != f.sess || c.k != f.k || core != f.core || c.atr.and_then(|a| i64::try_from(a).ok()).unwrap_or(i64::MIN) != f.atr {
            bad.push(format!("row {i}: fixed part differs"));
            if bad.len() > 20 {
                return (n, comp.len(), bad);
            }
        }
        for j in 0..9 {
            let want = c.st.get(j).copied().flatten().map_or(0, |(up, _, fl)| 1 | (u8::from(up) << 1) | (u8::from(fl) << 2));
            if comp.st(i, j) != want {
                bad.push(format!("row {i}: ST flags {j}"));
            }
        }
    }
    for long in [true, false] {
        for xe in [None, Some(9_u16), Some(20), Some(50), Some(100), Some(200)] {
            let (a, _) = cands::outer_values(full, &all, long, dials, xe);
            let (b, specs, _) = values(comp, long, dials, xe);
            if a.len() != b.len() {
                bad.push(format!("long {long} xe {xe:?}: {} values vs {}", a.len(), b.len()));
                continue;
            }
            for ((va, vb), s) in a.iter().zip(&b).zip(&specs) {
                n += 1;
                let kb = comp.kept(s, &idx, idx.len());
                if (va.dial != vb.dial || va.value != vb.value || va.label != vb.label || va.empty != vb.empty || (!va.empty && va.kept != kb)) && bad.len() < 20 {
                    bad.push(format!("long {long} xe {xe:?} {}: kept {} vs {}", va.label, va.kept.count(), kb.count()));
                }
            }
        }
    }
    (n, comp.len(), bad)
}

/// The compact family of an existing full family (its candidates, H-* reads and session levels): SEC-EQ on the MINUTES families
/// holds the compact evaluation to `cands::outer_values` on data the seconds path never builds.
///
/// # Errors
/// A boundary pattern broken, or a field that does not fit.
pub fn compress_family(full: &cands::Family, lv: &[Levels], dials: &[DialRow]) -> Result<SecFamily, String> {
    // PORTER (secfd): a full family with the context readings attached (`ctxdials::attach`) gets the context section too.
    let lay = if full.ctxr.is_empty() { RowLayout::new(dials, full.n_htf)? } else { RowLayout::new(dials, full.n_htf)?.with_ctx(dials)? };
    let mut fam = SecFamily { tf_s: full.tf_s, source: full.source.clone(), lay: lay.clone(), kmax: full.kmax, not_entry: full.not_entry, refused: full.refused.clone(), resets: full.resets.clone(), htf_reset_refused: full.htf_reset_refused.clone(), htf_names: full.htf_names.clone(), lv: lv.to_vec(), ..SecFamily::default() };
    let mut buf = vec![0_u8; lay.stride];
    for c in &full.cands {
        let reads: Vec<Option<HtfRead>> = (0..full.n_htf).map(|h| full.htf.get(c.idx as usize * full.n_htf + h).copied().flatten()).collect();
        fam.fix.push(lay.compress(c, &reads, &mut buf)?);
        if !full.ctxr.is_empty() {
            let r = full.ctxr.get(c.idx as usize).ok_or("a candidate without its context readings")?;
            lay.compress_ctx(c, r, &mut buf)?;
        }
        fam.bytes.extend_from_slice(&buf);
    }
    let mut sb = vec![0_u8; lay.sess_stride * lv.len()];
    for (i, l) in lv.iter().enumerate() {
        let o = i * lay.sess_stride;
        lay.compress_sess(l, sb.get_mut(o..o + lay.sess_stride).ok_or("session bytes")?)?;
    }
    fam.sess_bytes = sb;
    Ok(fam)
}

/// The 16-byte H-* packing (for the round-trip test).
#[must_use]
pub fn htf_pack_for_test(h: Option<HtfRead>) -> [u8; HTF_B] {
    let mut b = [0_u8; HTF_B];
    if htf_pack(h, &mut b).is_err() {
        b = [0_u8; HTF_B];
    }
    b
}
