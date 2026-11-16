//! The per-day CUBE (format: state/spikefade-20261114/CUBE-FORMAT.md) and the per-day option SECONDS tape, built from the staged
//! Tickvault files, cached with a manifest of the input sha256s (rebuilt only when a staged input changes), read back in O(size).

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::cal::{NM, OPEN_S, END_S, CLOSE_S, day_of_ymd};
use crate::csvgz::{GzLines, field, int, paisa, split, ts};

/// Builder identity printed in every manifest.
pub const BUILDER: &str = "spikefade-cube v1 (main 5b507203 wf step, 2026-11-14)";
const MAGIC: &[u8; 8] = b"SFCUBE01";
const MAGIC_SEC: &[u8; 8] = b"SFSEC001";

/// One underlying.
#[derive(Clone, Debug, Default)]
pub struct Und {
    pub name: String,
    /// 0 index, 1 stock.
    pub kind: u8,
    /// 0 IDX_I, 1 NSE_EQ, 255 none.
    pub spot_seg: u8,
    pub spot_sid: u32,
    /// The current (nearest) expiry of this underlying on this day among its staged option contracts, yyyymmdd.
    pub nearest: u32,
    /// The next expiry after `nearest`, 0 = none staged.
    pub next: u32,
}

/// One option contract.
#[derive(Clone, Debug, Default)]
pub struct Contract {
    pub sid: u32,
    pub und: u16,
    /// 0 OPTIDX, 1 OPTSTK.
    pub kind: u8,
    /// 0 CE, 1 PE.
    pub right: u8,
    pub expiry: u32,
    pub lot: u32,
    pub strike: i64,
    /// First minute slot with a bar (-1 none).
    pub first_min: i32,
    /// That bar's open (paisa).
    pub first_open: i32,
    pub symbol: String,
    pub n_sec: u32,
}

/// Build statistics (every extreme case counted, never silent).
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub rows_1m: u64,
    pub rows_1m_opt: u64,
    pub rows_1m_spot: u64,
    pub rows_1m_dup: u64,
    pub rows_1m_wrong_day: u64,
    pub rows_1m_off_grid: u64,
    pub rows_1m_not_whole_minute: u64,
    pub rows_1m_after_1530_opt: u64,
    pub rows_1m_neg_vol: u64,
    pub rows_1m_zero_vol_opt: u64,
    pub rows_1m_oi0_opt: u64,
    pub rows_1m_vol_not_lot_multiple: u64,
    pub rows_1m_oi_not_lot_multiple: u64,
    pub rows_1m_price_gt2dec: u64,
    pub rows_1m_bad: u64,
    pub rows_1m_cr: u64,
    pub rows_1s: u64,
    pub rows_1s_opt: u64,
    pub rows_1s_dup: u64,
    pub rows_1s_unsorted: u64,
    pub rows_1s_outside_session: u64,
    pub rows_1s_after_1530: u64,
    pub rows_1s_zero_vol: u64,
    pub rows_1s_bad: u64,
    pub rows_1s_cr: u64,
    pub opt_unmapped_1m: u64,
    pub contracts: u64,
    pub contracts_listed_midday: u64,
    pub contracts_no_bar: u64,
    pub contracts_lot_zero: u64,
    pub price_freeze_minutes: u64,
}

/// The loaded cube of one day.
#[derive(Clone, Debug, Default)]
pub struct Cube {
    pub ymd: u32,
    pub day: i64,
    pub unds: Vec<Und>,
    pub cons: Vec<Contract>,
    /// nu x NM, forward-filled (0 = no print yet).
    pub spot: Vec<i32>,
    /// nc x NM, forward-filled (0 = no print yet).
    pub close: Vec<i32>,
    /// nc x NM: bit0 bar present, bit1 forward-filled, bit2 volume > 0, bit3 OI 0 in a present bar.
    pub flags: Vec<u8>,
    pub high: Vec<i32>,
    pub low: Vec<i32>,
    pub vol: Vec<u32>,
    pub oi: Vec<u32>,
    /// nc x (NM+1): sum of vol over slots < m.
    pub cumvol: Vec<u64>,
    /// nc x (NM+1): sum of vol x close over slots < m (paisa x units).
    pub cumval: Vec<u64>,
    pub stats: Stats,
    /// fix:v33 LOT (standing F1): per contract, the whole multiple k >= 2 of its cube lot that its (underlying, lot) group's traded quantum
    /// is on this day (0 = the lot holds, or the guard is off). Never written to a cube file.
    pub lotbad: Vec<u8>,
    /// v3.4 WICK (matrix X19): per contract, whether its cube day HIGH is below the NSE bhavcopy's day HIGH (refused; empty = not checked).
    /// Never written to a cube file.
    pub wickbad: Vec<bool>,
    /// v3.4 E8 (critic F1): a SEGMENT SLICE's allowed contract ranges (every root merged in: index = the main root's index range AND the extra
    /// root's range); None = a whole cube. Reads outside are counted (`kit::SLICE_VIOLATIONS`) and void a shard. Never written to a file.
    pub lim: Option<Vec<(usize, usize)>>,
}

/// One second row of the option tape.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sec {
    pub sod: u32,
    pub high: i32,
    pub low: i32,
    pub vol: u32,
}

/// The option seconds of one day: rows of contract c are rows[off[c]..off[c+1]].
#[derive(Clone, Debug, Default)]
pub struct SecTape {
    pub off: Vec<u64>,
    pub rows: Vec<Sec>,
    /// v3.4 E8: the slice's allowed contract ranges (None = a whole tape); see [`Cube::lim`].
    pub lim: Option<Vec<(usize, usize)>>,
}

/// v3.4 E8 (critic F1): whether contract `c` lies in one of the allowed ranges (None = a whole cube: every contract).
#[inline]
#[must_use]
pub fn in_lim(lim: Option<&[(usize, usize)]>, c: usize) -> bool {
    lim.is_none_or(|l| l.iter().any(|&(a, b)| a <= c && c < b))
}

/// v3.4 E8: counts a read outside the slice (a shard refuses at its next batch file).
#[inline]
fn lim_guard(lim: Option<&[(usize, usize)]>, c: usize) {
    // MUTANT m_slice_reads_outside: a read outside the slice is not counted (it reads zeros with no error: critic F1)
    if !in_lim(lim, c) && !cfg!(feature = "m_slice_reads_outside") {
        crate::kit::SLICE_VIOLATIONS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

impl SecTape {
    /// The rows of contract `c`.
    #[must_use]
    pub fn of(&self, c: usize) -> &[Sec] {
        lim_guard(self.lim.as_deref(), c);
        let a = self.off.get(c).copied().unwrap_or(0);
        let b = self.off.get(c + 1).copied().unwrap_or(a);
        let (a, b) = (usize::try_from(a).unwrap_or(0), usize::try_from(b).unwrap_or(0));
        self.rows.get(a..b).unwrap_or(&[])
    }
}

impl Cube {
    /// Contract c's slot m index into the nc x NM arrays.
    #[inline]
    #[must_use]
    pub const fn ix(c: usize, m: usize) -> usize {
        c * NM + m
    }
    /// Contract c's prefix index.
    #[inline]
    #[must_use]
    pub const fn px(c: usize, m: usize) -> usize {
        c * (NM + 1) + m
    }
    /// v3.4 E8: whether contract `c`'s arrays are loaded (always, for a whole cube).
    #[inline]
    #[must_use]
    pub fn allowed(&self, c: usize) -> bool {
        in_lim(self.lim.as_deref(), c)
    }
    /// v3.4 E8: counts a direct array read of contract `c` outside the slice (the accessors below count their own).
    #[inline]
    pub fn guard(&self, c: usize) {
        lim_guard(self.lim.as_deref(), c);
    }
    /// Window volume of contract c over slots [a, b): O(1).
    #[inline]
    #[must_use]
    pub fn wvol(&self, c: usize, a: usize, b: usize) -> u64 {
        lim_guard(self.lim.as_deref(), c);
        self.cumvol.get(Self::px(c, b)).copied().unwrap_or(0).saturating_sub(self.cumvol.get(Self::px(c, a)).copied().unwrap_or(0))
    }
    /// Window traded value (sum of 1m volume x 1m close) of contract c over slots [a, b): O(1).
    #[inline]
    #[must_use]
    pub fn wval(&self, c: usize, a: usize, b: usize) -> u64 {
        lim_guard(self.lim.as_deref(), c);
        self.cumval.get(Self::px(c, b)).copied().unwrap_or(0).saturating_sub(self.cumval.get(Self::px(c, a)).copied().unwrap_or(0))
    }
    /// Forward-filled close at slot m (0 = none yet).
    #[inline]
    #[must_use]
    pub fn close_at(&self, c: usize, m: usize) -> i32 {
        lim_guard(self.lim.as_deref(), c);
        self.close.get(Self::ix(c, m)).copied().unwrap_or(0)
    }
    /// Forward-filled spot of underlying u at slot m.
    #[inline]
    #[must_use]
    pub fn spot_at(&self, u: usize, m: usize) -> i32 {
        self.spot.get(u * NM + m).copied().unwrap_or(0)
    }
}

/// The staged input files of one day.
#[must_use]
pub fn inputs(stage: &Path, d: &str) -> Vec<PathBuf> {
    vec![
        stage.join(format!("instruments/{d}/instruments-{d}.csv.gz")),
        stage.join(format!("candles_1m/{d}/candles_1m-{d}.csv.gz")),
        stage.join(format!("candles_1s/{d}/candles_1s-{d}.csv.gz")),
    ]
}

/// sha256 of a file through /usr/bin/shasum (the sell-diag's way; no new crate).
#[must_use]
pub fn sha256(p: &Path) -> String {
    // v3.4 E5: under PORTABLE in-process (no /usr/bin/shasum: absent on a perl-less image, where every margin table would be refused)
    // MUTANT m_kit_shasum_path: the external shasum kept under PORTABLE
    if crate::kit::on(crate::kit::PORTABLE) && !cfg!(feature = "m_kit_shasum_path") {
        return crate::kit::sha256_file(p).unwrap_or_else(|| "?".to_owned());
    }
    std::process::Command::new("/usr/bin/shasum").args(["-a", "256"]).arg(p).output().ok().and_then(|o| String::from_utf8(o.stdout).ok()).and_then(|s| s.split_whitespace().next().map(str::to_owned)).unwrap_or_else(|| "?".to_owned())
}

fn size_mtime(p: &Path) -> (u64, i64) {
    std::fs::metadata(p).map_or((0, 0), |m| {
        let mt = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
        (m.len(), mt)
    })
}

/// Whether the cached cube of `d` is current: the manifest lists the same inputs with the same size and mtime (fast path) or, when
/// size/mtime differ, the same sha256 (re-hashed). Returns (current, reason).
#[must_use]
pub fn is_current(out: &Path, stage: &Path, d: &str) -> (bool, String) {
    let man = out.join(format!("{d}.manifest.tsv"));
    let Ok(text) = std::fs::read_to_string(&man) else { return (false, "no manifest".to_owned()) };
    if !out.join(format!("{d}.bin")).exists() || !out.join(format!("{d}.sec.bin")).exists() {
        return (false, "cube file missing".to_owned());
    }
    let mut seen: HashMap<String, (u64, i64, String)> = HashMap::new();
    for l in text.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        if f.first() == Some(&"input") && f.len() >= 5 {
            seen.insert(f.get(1).copied().unwrap_or("").to_owned(), (f.get(2).and_then(|x| x.parse().ok()).unwrap_or(0), f.get(3).and_then(|x| x.parse().ok()).unwrap_or(0), f.get(4).copied().unwrap_or("").to_owned()));
        }
    }
    for p in inputs(stage, d) {
        let k = p.display().to_string();
        let Some((sz, mt, sh)) = seen.get(&k) else { return (false, format!("input {k} not in manifest")) };
        let (s2, m2) = size_mtime(&p);
        if s2 == *sz && m2 == *mt {
            continue;
        }
        let h = sha256(&p);
        if &h != sh {
            return (false, format!("input {k} changed (sha256 {h} != {sh})"));
        }
    }
    (true, "inputs unchanged".to_owned())
}

struct Inst {
    opts: HashMap<u32, (String, u8, String, u32, i64, u8, u32)>,
    idx_by_sym: HashMap<String, u32>,
    eq_by_sym: HashMap<String, u32>,
}

fn read_instruments(p: &Path) -> Result<Inst, String> {
    let mut g = GzLines::open(p)?;
    let mut f = Vec::new();
    let mut inst = Inst { opts: HashMap::new(), idx_by_sym: HashMap::new(), eq_by_sym: HashMap::new() };
    let mut first = true;
    while let Some(l) = g.next_line()? {
        split(l, &mut f);
        if first {
            first = false;
            let h: Vec<&[u8]> = (0..f.len()).map(|k| field(l, &f, k)).collect();
            if h.get(0) != Some(&&b"segment"[..]) || h.get(5) != Some(&&b"underlying"[..]) || h.get(9) != Some(&&b"lot_size"[..]) {
                return Err(format!("{}: unexpected instrument header", p.display()));
            }
            continue;
        }
        let seg = field(l, &f, 0);
        let sid = int(field(l, &f, 1)).and_then(|x| u32::try_from(x).ok()).ok_or("instrument sid")?;
        let sym = String::from_utf8_lossy(field(l, &f, 3)).into_owned();
        let kind = field(l, &f, 4);
        match seg {
            b"NSE_FNO" if (kind == b"OPTIDX" || kind == b"OPTSTK") => {
                let r = field(l, &f, 8);
                let right = match r {
                    b"CE" => 0,
                    b"PE" => 1,
                    _ => continue,
                };
                let und = String::from_utf8_lossy(field(l, &f, 5)).into_owned();
                let exp = int(field(l, &f, 6)).and_then(|x| u32::try_from(x).ok()).unwrap_or(0);
                let strike = paisa(field(l, &f, 7)).map_or(0, |x| x.0);
                let lot = int(field(l, &f, 9)).and_then(|x| u32::try_from(x).ok()).unwrap_or(0);
                inst.opts.insert(sid, (sym, u8::from(kind == b"OPTSTK"), und, exp, strike, right, lot));
            }
            b"IDX_I" => {
                inst.idx_by_sym.insert(sym, sid);
            }
            b"NSE_EQ" => {
                inst.eq_by_sym.insert(sym, sid);
            }
            _ => {}
        }
    }
    g.finish()?;
    Ok(inst)
}

fn check_header(l: &[u8], f: &[(usize, usize)], p: &Path) -> Result<(), String> {
    let want: [&[u8]; 14] = [b"ts", b"open_latency", b"close_latency", b"window_span_latency", b"feed", b"segment", b"security_id", b"contract", b"open", b"high", b"low", b"close", b"volume", b"oi"];
    for (k, w) in want.iter().enumerate() {
        if field(l, f, k) != *w {
            return Err(format!("{}: header column {k} is {:?}, expected {:?}", p.display(), String::from_utf8_lossy(field(l, f, k)), String::from_utf8_lossy(w)));
        }
    }
    Ok(())
}

/// Builds the cube and the seconds tape of day `d` ("YYYY-MM-DD") from the staged files; writes `<d>.bin`, `<d>.sec.bin` and
/// `<d>.manifest.tsv` into `out` (an existing file is moved aside first, never deleted). Returns the build log.
///
/// # Errors
/// A missing or malformed input (the day is refused by name).
#[allow(clippy::too_many_lines)]
pub fn build(stage: &Path, out: &Path, d: &str) -> Result<String, String> {
    let t0 = std::time::Instant::now();
    let ins = inputs(stage, d);
    let [pi, p1m, p1s] = ins.as_slice() else { return Err("inputs".to_owned()) };
    let ymd: u32 = d.replace('-', "").parse().map_err(|_| format!("day {d}"))?;
    let day = day_of_ymd(ymd);
    let inst = read_instruments(pi)?;
    // Contracts in a stable order.
    let mut keys: Vec<(u8, String, u32, i64, u8, u32)> = inst.opts.iter().map(|(sid, (_, k, u, e, s, r, _))| (*k, u.clone(), *e, *s, *r, *sid)).collect();
    keys.sort();
    let mut und_ix: BTreeMap<String, usize> = BTreeMap::new();
    let mut unds: Vec<Und> = Vec::new();
    let mut cons: Vec<Contract> = Vec::with_capacity(keys.len());
    let mut ci_of: HashMap<u32, usize> = HashMap::with_capacity(keys.len());
    let mut st = Stats::default();
    for (k, u, e, s, r, sid) in &keys {
        let ui = if let Some(&x) = und_ix.get(u) {
            x
        } else {
            let (seg, ssid) = if *k == 0 { inst.idx_by_sym.get(u).map_or((255, 0), |&x| (0, x)) } else { inst.eq_by_sym.get(u).map_or((255, 0), |&x| (1, x)) };
            unds.push(Und { name: u.clone(), kind: *k, spot_seg: seg, spot_sid: ssid, nearest: 0, next: 0 });
            und_ix.insert(u.clone(), unds.len() - 1);
            unds.len() - 1
        };
        let (sym, _, _, _, _, _, lot) = inst.opts.get(sid).cloned().unwrap_or_default();
        if lot == 0 {
            st.contracts_lot_zero += 1;
        }
        ci_of.insert(*sid, cons.len());
        cons.push(Contract { sid: *sid, und: u16::try_from(ui).unwrap_or(u16::MAX), kind: *k, right: *r, expiry: *e, lot, strike: *s, first_min: -1, first_open: 0, symbol: sym, n_sec: 0 });
    }
    // Nearest and next expiry per underlying (expiry >= day).
    for (ui, u) in unds.iter_mut().enumerate() {
        let mut ex: Vec<u32> = cons.iter().filter(|c| usize::from(c.und) == ui && c.expiry >= ymd).map(|c| c.expiry).collect();
        ex.sort_unstable();
        ex.dedup();
        u.nearest = ex.first().copied().unwrap_or(0);
        u.next = ex.get(1).copied().unwrap_or(0);
    }
    let nc = cons.len();
    let nu = unds.len();
    let mut spot_of: HashMap<(u8, u32), usize> = HashMap::new();
    for (ui, u) in unds.iter().enumerate() {
        if u.spot_seg != 255 {
            spot_of.insert((u.spot_seg, u.spot_sid), ui);
        }
    }
    let mut close = vec![0_i32; nc * NM];
    let mut flags = vec![0_u8; nc * NM];
    let mut high = vec![0_i32; nc * NM];
    let mut low = vec![0_i32; nc * NM];
    let mut vol = vec![0_u32; nc * NM];
    let mut oi = vec![0_u32; nc * NM];
    let mut open = vec![0_i32; nc * NM];
    let mut spot = vec![0_i32; nu * NM];
    let mut spot_has = vec![false; nu * NM];
    // ---- candles_1m
    let mut g = GzLines::open(p1m)?;
    let mut f = Vec::new();
    let mut first = true;
    while let Some(l) = g.next_line()? {
        split(l, &mut f);
        if first {
            first = false;
            check_header(l, &f, p1m)?;
            continue;
        }
        st.rows_1m += 1;
        let Some((rymd, sod, whole)) = ts(field(l, &f, 0)) else {
            st.rows_1m_bad += 1;
            continue;
        };
        if rymd != ymd {
            st.rows_1m_wrong_day += 1;
            continue;
        }
        if !whole || sod % 60 != 0 {
            st.rows_1m_not_whole_minute += 1;
            continue;
        }
        let seg = field(l, &f, 5);
        let Some(sid) = int(field(l, &f, 6)).and_then(|x| u32::try_from(x).ok()) else {
            st.rows_1m_bad += 1;
            continue;
        };
        let m = (sod - OPEN_S).div_euclid(60);
        let Ok(m) = usize::try_from(m) else {
            st.rows_1m_off_grid += 1;
            continue;
        };
        if m >= NM {
            st.rows_1m_off_grid += 1;
            continue;
        }
        let pc = paisa(field(l, &f, 11));
        let Some((cl, x1)) = pc else {
            st.rows_1m_bad += 1;
            continue;
        };
        if seg == b"NSE_FNO" {
            let Some(&ci) = ci_of.get(&sid) else {
                st.opt_unmapped_1m += 1;
                continue;
            };
            st.rows_1m_opt += 1;
            let (Some((op, x2)), Some((hi, x3)), Some((lo, x4))) = (paisa(field(l, &f, 8)), paisa(field(l, &f, 9)), paisa(field(l, &f, 10))) else {
                st.rows_1m_bad += 1;
                continue;
            };
            if x1 || x2 || x3 || x4 {
                st.rows_1m_price_gt2dec += 1;
            }
            let v = int(field(l, &f, 12)).unwrap_or(0);
            if v < 0 {
                st.rows_1m_neg_vol += 1;
            }
            let v = v.unsigned_abs();
            let o = int(field(l, &f, 13)).unwrap_or(0).max(0).unsigned_abs();
            let i = Cube::ix(ci, m);
            if flags.get(i).is_some_and(|x| x & 1 == 1) {
                st.rows_1m_dup += 1;
                continue;
            }
            if sod >= CLOSE_S {
                st.rows_1m_after_1530_opt += 1;
            }
            let lot = u64::from(cons.get(ci).map_or(0, |c| c.lot));
            if lot > 0 && v % lot != 0 {
                st.rows_1m_vol_not_lot_multiple += 1;
            }
            if lot > 0 && o % lot != 0 {
                st.rows_1m_oi_not_lot_multiple += 1;
            }
            if v == 0 {
                st.rows_1m_zero_vol_opt += 1;
            }
            let mut fl = 1_u8;
            if v > 0 {
                fl |= 4;
            }
            if o == 0 {
                st.rows_1m_oi0_opt += 1;
                fl |= 8;
            }
            if let (Some(a), Some(b), Some(c2), Some(dd), Some(e), Some(ff), Some(gg)) = (close.get_mut(i), flags.get_mut(i), high.get_mut(i), low.get_mut(i), vol.get_mut(i), oi.get_mut(i), open.get_mut(i)) {
                *a = i32::try_from(cl).unwrap_or(0);
                *b = fl;
                *c2 = i32::try_from(hi).unwrap_or(0);
                *dd = i32::try_from(lo).unwrap_or(0);
                *e = u32::try_from(v).unwrap_or(u32::MAX);
                *ff = u32::try_from(o).unwrap_or(u32::MAX);
                *gg = i32::try_from(op).unwrap_or(0);
            }
        } else if seg == b"IDX_I" || seg == b"NSE_EQ" {
            let sg = u8::from(seg == b"NSE_EQ");
            if let Some(&ui) = spot_of.get(&(sg, sid)) {
                st.rows_1m_spot += 1;
                if let (Some(a), Some(b)) = (spot.get_mut(ui * NM + m), spot_has.get_mut(ui * NM + m)) {
                    *a = i32::try_from(cl).unwrap_or(0);
                    *b = true;
                }
            }
        }
    }
    st.rows_1m_cr = 0;
    g.finish()?;
    // Forward fill, first bar, prefix sums, price freezes (a present bar with volume 0 and the same close as before).
    let mut cumvol = vec![0_u64; nc * (NM + 1)];
    let mut cumval = vec![0_u64; nc * (NM + 1)];
    for (ci, c) in cons.iter_mut().enumerate() {
        let mut last = 0_i32;
        let mut cv = 0_u64;
        let mut cl = 0_u64;
        for m in 0..NM {
            let i = Cube::ix(ci, m);
            let fl = flags.get(i).copied().unwrap_or(0);
            if fl & 1 == 1 {
                if c.first_min < 0 {
                    c.first_min = i32::try_from(m).unwrap_or(-1);
                    c.first_open = open.get(i).copied().unwrap_or(0);
                }
                let cc = close.get(i).copied().unwrap_or(0);
                if fl & 4 == 0 && cc == last {
                    st.price_freeze_minutes += 1;
                }
                last = cc;
                let v = u64::from(vol.get(i).copied().unwrap_or(0));
                cv = cv.saturating_add(v);
                cl = cl.saturating_add(v.saturating_mul(u64::try_from(cc.max(0)).unwrap_or(0)));
            } else if last > 0 {
                if let (Some(a), Some(b)) = (close.get_mut(i), flags.get_mut(i)) {
                    *a = last;
                    *b |= 2;
                }
            }
            if let (Some(a), Some(b)) = (cumvol.get_mut(Cube::px(ci, m + 1)), cumval.get_mut(Cube::px(ci, m + 1))) {
                *a = cv;
                *b = cl;
            }
        }
        if c.first_min < 0 {
            st.contracts_no_bar += 1;
        } else if c.first_min > 0 {
            st.contracts_listed_midday += 1;
        }
    }
    for ui in 0..nu {
        let mut last = 0_i32;
        for m in 0..NM {
            let i = ui * NM + m;
            if spot_has.get(i).copied().unwrap_or(false) {
                last = spot.get(i).copied().unwrap_or(0);
            } else if let Some(a) = spot.get_mut(i) {
                *a = last;
            }
        }
    }
    st.contracts = u64::try_from(nc).unwrap_or(0);
    // ---- candles_1s (options only, [09:15:00, 15:40:00))
    let mut per: Vec<Vec<Sec>> = vec![Vec::new(); nc];
    let mut g = GzLines::open(p1s)?;
    let mut first = true;
    while let Some(l) = g.next_line()? {
        split(l, &mut f);
        if first {
            first = false;
            check_header(l, &f, p1s)?;
            continue;
        }
        st.rows_1s += 1;
        if field(l, &f, 5) != b"NSE_FNO" {
            continue;
        }
        let Some(sid) = int(field(l, &f, 6)).and_then(|x| u32::try_from(x).ok()) else {
            st.rows_1s_bad += 1;
            continue;
        };
        let Some(&ci) = ci_of.get(&sid) else { continue };
        let Some((rymd, sod, whole)) = ts(field(l, &f, 0)) else {
            st.rows_1s_bad += 1;
            continue;
        };
        if rymd != ymd || !whole {
            st.rows_1s_bad += 1;
            continue;
        }
        st.rows_1s_opt += 1;
        if !(OPEN_S..END_S).contains(&sod) {
            st.rows_1s_outside_session += 1;
            continue;
        }
        if sod >= CLOSE_S {
            st.rows_1s_after_1530 += 1;
        }
        let (Some((hi, _)), Some((lo, _))) = (paisa(field(l, &f, 9)), paisa(field(l, &f, 10))) else {
            st.rows_1s_bad += 1;
            continue;
        };
        let v = int(field(l, &f, 12)).unwrap_or(0).unsigned_abs();
        if v == 0 {
            st.rows_1s_zero_vol += 1;
        }
        let Some(rows) = per.get_mut(ci) else { continue };
        let s = u32::try_from(sod).unwrap_or(0);
        if let Some(lst) = rows.last() {
            if lst.sod == s {
                st.rows_1s_dup += 1;
                continue;
            }
            if lst.sod > s {
                st.rows_1s_unsorted += 1;
            }
        }
        rows.push(Sec { sod: s, high: i32::try_from(hi).unwrap_or(0), low: i32::try_from(lo).unwrap_or(0), vol: u32::try_from(v).unwrap_or(u32::MAX) });
    }
    g.finish()?;
    let mut off = Vec::with_capacity(nc + 1);
    let mut rows: Vec<Sec> = Vec::new();
    for (ci, r) in per.iter_mut().enumerate() {
        if st.rows_1s_unsorted > 0 {
            r.sort_by_key(|x| x.sod);
            let before = r.len();
            r.dedup_by_key(|x| x.sod);
            st.rows_1s_dup += u64::try_from(before - r.len()).unwrap_or(0);
        }
        off.push(u64::try_from(rows.len()).unwrap_or(0));
        if let Some(c) = cons.get_mut(ci) {
            c.n_sec = u32::try_from(r.len()).unwrap_or(0);
        }
        rows.append(r);
    }
    off.push(u64::try_from(rows.len()).unwrap_or(0));
    let cube = Cube { ymd, day, unds, cons, spot, close, flags, high, low, vol, oi, cumvol, cumval, stats: st, lotbad: Vec::new(), wickbad: Vec::new(), lim: None };
    let tape = SecTape { off, rows, lim: None };
    // ---- write (move an existing file aside first)
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |x| x.as_secs());
    let pb = out.join(format!("{d}.bin"));
    let ps = out.join(format!("{d}.sec.bin"));
    let pm = out.join(format!("{d}.manifest.tsv"));
    for p in [&pb, &ps, &pm] {
        if p.exists() {
            let aside = PathBuf::from(format!("{}.old-{stamp}", p.display()));
            std::fs::rename(p, &aside).map_err(|e| format!("move aside {}: {e}", p.display()))?;
        }
    }
    let tmp = out.join(format!("{d}.bin.part"));
    std::fs::write(&tmp, encode(&cube)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &pb).map_err(|e| e.to_string())?;
    let tmp = out.join(format!("{d}.sec.bin.part"));
    std::fs::write(&tmp, encode_sec(&tape, ymd)).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &ps).map_err(|e| e.to_string())?;
    let mut man = String::new();
    let _ = writeln!(man, "builder\t{BUILDER}");
    for p in &ins {
        let (s, m) = size_mtime(p);
        let _ = writeln!(man, "input\t{}\t{s}\t{m}\t{}", p.display(), sha256(p));
    }
    for p in [&pb, &ps] {
        let (s, _) = size_mtime(p);
        let _ = writeln!(man, "output\t{}\t{s}\t{}", p.display(), sha256(p));
    }
    let _ = writeln!(man, "build_seconds\t{:.3}", t0.elapsed().as_secs_f64());
    let _ = writeln!(man, "stats\t{:?}", cube.stats);
    std::fs::write(&pm, &man).map_err(|e| e.to_string())?;
    Ok(format!("{d}: built in {:.1} s: {} contracts, {} underlyings, {} 1-s option rows; {:?}", t0.elapsed().as_secs_f64(), cube.cons.len(), cube.unds.len(), tape.rows.len(), cube.stats))
}

fn put_u32(b: &mut Vec<u8>, x: u32) {
    b.extend_from_slice(&x.to_le_bytes());
}
fn put_i32(b: &mut Vec<u8>, x: i32) {
    b.extend_from_slice(&x.to_le_bytes());
}
fn put_u64(b: &mut Vec<u8>, x: u64) {
    b.extend_from_slice(&x.to_le_bytes());
}
fn pad(b: &mut Vec<u8>, a: usize) {
    while b.len() % a != 0 {
        b.push(0);
    }
}

/// v3.3 integrate (the named test of EXTREME-MATRIX-SELLING X61): writes one day's cube and seconds tape in the run's file format (the
/// builder's own encoders), for a streamed store over synthetic days.
///
/// # Errors
/// A write error.
#[cfg(test)]
pub fn write_day_t(dir: &Path, d: &str, c: &Cube, t: &SecTape) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{d}.bin")), encode(c)).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(format!("{d}.sec.bin")), encode_sec(t, c.ymd)).map_err(|e| e.to_string())
}

fn encode(c: &Cube) -> Vec<u8> {
    encode_with(c, None)
}

/// v3.4 E8: the magic of a segment slice of a cube file (the whole header, every record, the strings and every spot series; the arrays of
/// contracts [c0, c1) only).
pub const SLICE_MAGIC: &[u8; 8] = b"SFCUBS01";
/// v3.4 E8: the magic of a segment slice of a seconds tape (the offsets rebuilt for the kept rows; absent contracts empty).
pub const SLICE_MAGIC_SEC: &[u8; 8] = b"SFSECS01";

/// The cube file's bytes; with `slice` = (segment, c0, c1) the segment slice (E8): its own header, then the whole cube's header, records,
/// strings and spot series, then the arrays of contracts [c0, c1) only. `None` writes v1 exactly as before (the builder's encoder).
pub(crate) fn encode_with(c: &Cube, slice: Option<(u8, usize, usize)>) -> Vec<u8> {
    let mut strs: Vec<u8> = Vec::new();
    let mut b = Vec::with_capacity(slice.map_or(c.cons.len(), |s| s.2 - s.1) * NM * 40 + c.unds.len() * NM * 4 + 4096);
    if let Some((seg, c0, c1)) = slice {
        b.extend_from_slice(SLICE_MAGIC);
        put_u32(&mut b, 1);
        put_u32(&mut b, u32::from(seg));
        put_u32(&mut b, u32::try_from(c0).unwrap_or(u32::MAX));
        put_u32(&mut b, u32::try_from(c1).unwrap_or(u32::MAX));
    } else {
        b.extend_from_slice(MAGIC);
    }
    put_u32(&mut b, 1);
    put_u32(&mut b, c.ymd);
    put_u32(&mut b, u32::try_from(NM).unwrap_or(0));
    put_u32(&mut b, u32::try_from(OPEN_S).unwrap_or(0));
    put_u32(&mut b, u32::try_from(c.cons.len()).unwrap_or(0));
    put_u32(&mut b, u32::try_from(c.unds.len()).unwrap_or(0));
    let mut und_rec = Vec::new();
    for u in &c.unds {
        put_u32(&mut und_rec, u32::try_from(strs.len()).unwrap_or(0));
        und_rec.extend_from_slice(&u16::try_from(u.name.len()).unwrap_or(0).to_le_bytes());
        und_rec.push(u.kind);
        und_rec.push(u.spot_seg);
        put_u32(&mut und_rec, u.spot_sid);
        put_u32(&mut und_rec, u.nearest);
        put_u32(&mut und_rec, u.next);
        strs.extend_from_slice(u.name.as_bytes());
    }
    let mut con_rec = Vec::new();
    for k in &c.cons {
        put_u32(&mut con_rec, k.sid);
        con_rec.extend_from_slice(&k.und.to_le_bytes());
        con_rec.push(k.kind);
        con_rec.push(k.right);
        put_u32(&mut con_rec, k.expiry);
        put_u32(&mut con_rec, k.lot);
        con_rec.extend_from_slice(&k.strike.to_le_bytes());
        put_i32(&mut con_rec, k.first_min);
        put_i32(&mut con_rec, k.first_open);
        put_u32(&mut con_rec, u32::try_from(strs.len()).unwrap_or(0));
        con_rec.extend_from_slice(&u16::try_from(k.symbol.len()).unwrap_or(0).to_le_bytes());
        con_rec.extend_from_slice(&0_u16.to_le_bytes());
        put_u32(&mut con_rec, k.n_sec);
        put_u32(&mut con_rec, 0);
        strs.extend_from_slice(k.symbol.as_bytes());
    }
    put_u32(&mut b, u32::try_from(strs.len()).unwrap_or(0));
    put_u32(&mut b, 0);
    b.extend_from_slice(&und_rec);
    b.extend_from_slice(&con_rec);
    b.extend_from_slice(&strs);
    pad(&mut b, 8);
    for x in &c.spot {
        put_i32(&mut b, *x);
    }
    // the per-contract arrays: every contract, or (E8) the rows of contracts [c0, c1) only
    let (r0, r1) = slice.map_or((0, c.cons.len()), |s| (s.1, s.2));
    let rows = |n: usize| (r0 * n, r1 * n);
    let (a, z) = rows(NM);
    for x in c.close.get(a..z).unwrap_or(&[]) {
        put_i32(&mut b, *x);
    }
    b.extend_from_slice(c.flags.get(a..z).unwrap_or(&[]));
    pad(&mut b, 4);
    for x in c.high.get(a..z).unwrap_or(&[]) {
        put_i32(&mut b, *x);
    }
    for x in c.low.get(a..z).unwrap_or(&[]) {
        put_i32(&mut b, *x);
    }
    for x in c.vol.get(a..z).unwrap_or(&[]) {
        put_u32(&mut b, *x);
    }
    for x in c.oi.get(a..z).unwrap_or(&[]) {
        put_u32(&mut b, *x);
    }
    pad(&mut b, 8);
    let (a1, z1) = rows(NM + 1);
    for x in c.cumvol.get(a1..z1).unwrap_or(&[]) {
        put_u64(&mut b, *x);
    }
    for x in c.cumval.get(a1..z1).unwrap_or(&[]) {
        put_u64(&mut b, *x);
    }
    b
}

fn encode_sec(t: &SecTape, ymd: u32) -> Vec<u8> {
    encode_sec_with(t, ymd, None)
}

/// The seconds tape's bytes; with `slice` = (segment, c0, c1) the segment slice (E8): its own header, then v1's header with the kept rows
/// count, the offsets rebuilt so contracts outside [c0, c1) are empty, and the kept rows. `None` writes v1 exactly as before.
pub(crate) fn encode_sec_with(t: &SecTape, ymd: u32, slice: Option<(u8, usize, usize)>) -> Vec<u8> {
    let nc = t.off.len().saturating_sub(1);
    let (o0, o1) = slice.map_or((0, u64::try_from(t.rows.len()).unwrap_or(0)), |s| (t.off.get(s.1).copied().unwrap_or(0), t.off.get(s.2).copied().unwrap_or(0)));
    let kept = t.rows.get(usize::try_from(o0).unwrap_or(0)..usize::try_from(o1).unwrap_or(0)).unwrap_or(&[]);
    let mut b = Vec::with_capacity(kept.len() * 16 + t.off.len() * 8 + 64);
    if let Some((seg, c0, c1)) = slice {
        b.extend_from_slice(SLICE_MAGIC_SEC);
        put_u32(&mut b, 1);
        put_u32(&mut b, u32::from(seg));
        put_u32(&mut b, u32::try_from(c0).unwrap_or(u32::MAX));
        put_u32(&mut b, u32::try_from(c1).unwrap_or(u32::MAX));
    } else {
        b.extend_from_slice(MAGIC_SEC);
    }
    put_u32(&mut b, 1);
    put_u32(&mut b, ymd);
    put_u32(&mut b, u32::try_from(nc).unwrap_or(0));
    put_u32(&mut b, 0);
    put_u64(&mut b, u64::try_from(kept.len()).unwrap_or(0));
    for x in &t.off {
        put_u64(&mut b, if slice.is_some() { (*x).clamp(o0, o1) - o0 } else { *x });
    }
    for r in kept {
        put_u32(&mut b, r.sod);
        put_i32(&mut b, r.high);
        put_i32(&mut b, r.low);
        put_u32(&mut b, r.vol);
    }
    b
}

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}
impl Rd<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let s = self.b.get(self.p..self.p + n).ok_or("cube file truncated")?;
        self.p += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        self.take(1)?.first().copied().ok_or_else(|| "cube".to_owned())
    }
    fn u16(&mut self) -> Result<u16, String> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s.first().copied().unwrap_or(0), s.get(1).copied().unwrap_or(0)]))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        let mut a = [0_u8; 4];
        a.copy_from_slice(s);
        Ok(u32::from_le_bytes(a))
    }
    fn i32(&mut self) -> Result<i32, String> {
        let s = self.take(4)?;
        let mut a = [0_u8; 4];
        a.copy_from_slice(s);
        Ok(i32::from_le_bytes(a))
    }
    fn u64(&mut self) -> Result<u64, String> {
        let s = self.take(8)?;
        let mut a = [0_u8; 8];
        a.copy_from_slice(s);
        Ok(u64::from_le_bytes(a))
    }
    fn i64(&mut self) -> Result<i64, String> {
        let s = self.take(8)?;
        let mut a = [0_u8; 8];
        a.copy_from_slice(s);
        Ok(i64::from_le_bytes(a))
    }
    fn align(&mut self, a: usize) {
        while self.p % a != 0 {
            self.p += 1;
        }
    }
}

/// v3.2 round B (addendum 17): the SECOND cube root of a run (`--cube-extra <dir>`): the cubes of the other NSE indices (FINNIFTY,
/// MIDCPNIFTY, NIFTYNXT50), built apart by the GDFL cube tool's extra-index copy with their own contract ids. Set once per run; a day the
/// extra root does not hold leaves the main cube unchanged.
pub static EXTRA: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// v3.2 round B: the days whose cube received an extra root's cube (counted, printed).
pub static EXTRA_DAYS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// [`load`] plus the extra root's cube of the same day merged in (when `--cube-extra` is set and that day is held there).
///
/// # Errors
/// A missing or bad main cube; a bad extra cube or a merge conflict (another day, an underlying or contract id in both roots).
pub fn load_m(out: &Path, d: &str) -> Result<Cube, String> {
    let mut c = load(out, d)?;
    if let Some(x) = EXTRA.get()
        && extra_has(x, d)
    {
        merge(&mut c, load(x, d)?).map_err(|e| format!("--cube-extra {} day {d}: {e}", x.display()))?;
        EXTRA_DAYS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    Ok(c)
}

/// [`load_sec`] plus the extra root's seconds tape of the same day (the contracts in the order [`load_m`] merges them).
///
/// # Errors
/// A missing or bad tape.
pub fn load_sec_m(out: &Path, d: &str) -> Result<SecTape, String> {
    let mut t = load_sec(out, d)?;
    if let Some(x) = EXTRA.get()
        && extra_has(x, d)
    {
        merge_sec(&mut t, load_sec(x, d)?);
    }
    Ok(t)
}

/// Whether the extra root holds day `d`: its cube file (v3.3's test), or under E9 its transport file.
#[must_use]
pub fn extra_has(x: &Path, d: &str) -> bool {
    if crate::kit::TRANSPORT.load(std::sync::atomic::Ordering::Relaxed) {
        return x.join(format!("{d}.sfz")).exists();
    }
    x.join(format!("{d}.bin")).exists()
}

/// v3.4 E8 (critic F1): the allowed contract ranges of a merged day: None when neither side is a slice (v3.3); else the main side's ranges
/// (its whole [0, ncm) when it is a whole cube) AND the extra side's, shifted by `ncm` behind every main contract (its whole range when
/// whole).
#[must_use]
pub fn merge_lim(a: Option<Vec<(usize, usize)>>, b: Option<Vec<(usize, usize)>>, ncm: usize, nce: usize) -> Option<Vec<(usize, usize)>> {
    if a.is_none() && b.is_none() {
        return None;
    }
    let mut v = a.unwrap_or_else(|| vec![(0, ncm)]);
    // MUTANT m_slice_drops_extra_range: the extra root's contracts left out of the allowed ranges (critic F1's defect: one range only)
    if !cfg!(feature = "m_slice_drops_extra_range") {
        v.extend(b.unwrap_or_else(|| vec![(0, nce)]).into_iter().map(|(x, y)| (x + ncm, y + ncm)));
    }
    Some(v)
}

/// Merges `e` (another root's cube of the SAME day) into `c`: its underlyings and contracts appended (the contracts' underlying index
/// shifted), every per-underlying and per-contract array appended in the same order. Exact: every array of `c` keeps its values and
/// offsets; `e`'s are read at the shifted indexes (named test `v32rb_cube_merge_exact`).
///
/// # Errors
/// Another day, an underlying name or a contract id present in both cubes (the extra root must hold other underlyings only).
pub fn merge(c: &mut Cube, e: Cube) -> Result<(), String> {
    if c.ymd != e.ymd {
        return Err(format!("days differ ({} vs {})", c.ymd, e.ymd));
    }
    if let Some(u) = e.unds.iter().find(|u| c.unds.iter().any(|x| x.name == u.name)) {
        return Err(format!("underlying {} is in both cube roots", u.name));
    }
    let sids: std::collections::HashSet<u32> = c.cons.iter().map(|k| k.sid).collect();
    if let Some(k) = e.cons.iter().find(|k| sids.contains(&k.sid)) {
        return Err(format!("contract id {} ({}) is in both cube roots", k.sid, k.symbol));
    }
    let u0 = u16::try_from(c.unds.len()).map_err(|_| "too many underlyings".to_owned())?;
    if usize::from(u0) + e.unds.len() > usize::from(u16::MAX) {
        return Err("too many underlyings".to_owned());
    }
    // MUTANT m_v32rb_cube_merge_no_shift: the extra contracts keep their own underlying index (they would read the main cube's spot)
    let shift = if cfg!(feature = "m_v32rb_cube_merge_no_shift") { 0 } else { u0 };
    // v3.4 E8 (critic F1): a slice's allowed ranges follow the contracts (index = the main root's range AND the extra root's)
    let (ncm, nce) = (c.cons.len(), e.cons.len());
    c.lim = merge_lim(c.lim.take(), e.lim.clone(), ncm, nce);
    c.unds.extend(e.unds);
    c.cons.extend(e.cons.into_iter().map(|mut k| {
        k.und += shift;
        k
    }));
    c.spot.extend(e.spot);
    c.close.extend(e.close);
    c.flags.extend(e.flags);
    c.high.extend(e.high);
    c.low.extend(e.low);
    c.vol.extend(e.vol);
    c.oi.extend(e.oi);
    c.cumvol.extend(e.cumvol);
    c.cumval.extend(e.cumval);
    Ok(())
}

/// Merges another root's seconds tape of the same day (contracts appended in [`merge`]'s order; offsets shifted).
pub fn merge_sec(t: &mut SecTape, e: SecTape) {
    let base = u64::try_from(t.rows.len()).unwrap_or(0);
    // v3.4 E8: the ranges as in [`merge`]
    let (ncm, nce) = (t.off.len().saturating_sub(1), e.off.len().saturating_sub(1));
    t.lim = merge_lim(t.lim.take(), e.lim.clone(), ncm, nce);
    if t.off.is_empty() {
        t.off.push(0);
    }
    t.off.extend(e.off.iter().skip(1).map(|&o| o + base));
    t.rows.extend(e.rows);
}

/// Reads `<d>.bin` back.
///
/// # Errors
/// A missing, truncated or foreign file.
pub fn load(out: &Path, d: &str) -> Result<Cube, String> {
    let p = out.join(format!("{d}.bin"));
    // v3.4 E9: a shard's transport file (<d>.sfz), decoded in memory (no raw cube on disk); else the cube file as v3.3 reads it
    let b = if crate::kit::TRANSPORT.load(std::sync::atomic::Ordering::Relaxed) { crate::slice::sfz_bin(out, d)? } else { std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))? };
    decode_cube(&b, &p)
}

/// A cube file's bytes back: v1, or a v3.4 E8 segment slice (the same layout after its own header, the arrays of contracts [c0, c1)
/// only, expanded to the full contract count: every other contract's arrays zero and refused on read through `lim` = [(c0, c1)]).
///
/// # Errors
/// A truncated or foreign file, or a slice range outside the contracts.
pub(crate) fn decode_cube(b: &[u8], p: &Path) -> Result<Cube, String> {
    let mut r = Rd { b, p: 0 };
    let magic = r.take(8)?;
    let slice: Option<(usize, usize)> = if magic == MAGIC {
        None
    } else if magic == SLICE_MAGIC {
        let (sv, _seg, c0, c1) = (r.u32()?, r.u32()?, r.u32()?, r.u32()?);
        if sv != 1 {
            return Err(format!("{}: slice version {sv}", p.display()));
        }
        Some((usize::try_from(c0).unwrap_or(usize::MAX), usize::try_from(c1).unwrap_or(usize::MAX)))
    } else {
        return Err(format!("{}: not a cube", p.display()));
    };
    let ver = r.u32()?;
    if ver != 1 {
        return Err(format!("{}: cube version {ver}", p.display()));
    }
    let ymd = r.u32()?;
    let nm = usize::try_from(r.u32()?).unwrap_or(0);
    let gs = i64::from(r.u32()?);
    if nm != NM || gs != OPEN_S {
        return Err(format!("{}: grid {nm} from {gs}", p.display()));
    }
    let nc = usize::try_from(r.u32()?).unwrap_or(0);
    let nu = usize::try_from(r.u32()?).unwrap_or(0);
    let ns = usize::try_from(r.u32()?).unwrap_or(0);
    let _ = r.u32()?;
    if let Some((c0, c1)) = slice
        && (c0 > c1 || c1 > nc)
    {
        return Err(format!("{}: slice [{c0}, {c1}) outside the cube's {nc} contracts", p.display()));
    }
    let mut unds_raw = Vec::with_capacity(nu);
    for _ in 0..nu {
        let off = r.u32()?;
        let len = r.u16()?;
        let kind = r.u8()?;
        let seg = r.u8()?;
        let sid = r.u32()?;
        let nearest = r.u32()?;
        let next = r.u32()?;
        unds_raw.push((off, len, kind, seg, sid, nearest, next));
    }
    let mut cons_raw = Vec::with_capacity(nc);
    for _ in 0..nc {
        let sid = r.u32()?;
        let und = r.u16()?;
        let kind = r.u8()?;
        let right = r.u8()?;
        let expiry = r.u32()?;
        let lot = r.u32()?;
        let strike = r.i64()?;
        let first_min = r.i32()?;
        let first_open = r.i32()?;
        let so = r.u32()?;
        let sl = r.u16()?;
        let _ = r.u16()?;
        let n_sec = r.u32()?;
        let _ = r.u32()?;
        cons_raw.push((Contract { sid, und, kind, right, expiry, lot, strike, first_min, first_open, symbol: String::new(), n_sec }, so, sl));
    }
    let strs = r.take(ns)?.to_vec();
    let s_of = |o: u32, l: u16| -> String {
        let o = usize::try_from(o).unwrap_or(0);
        String::from_utf8_lossy(strs.get(o..o + usize::from(l)).unwrap_or(&[])).into_owned()
    };
    let unds: Vec<Und> = unds_raw.iter().map(|&(o, l, kind, seg, sid, nearest, next)| Und { name: s_of(o, l), kind, spot_seg: seg, spot_sid: sid, nearest, next }).collect();
    let cons: Vec<Contract> = cons_raw.into_iter().map(|(mut c, o, l)| {
        c.symbol = s_of(o, l);
        c
    }).collect();
    r.align(8);
    let mut spot = Vec::with_capacity(nu * NM);
    for _ in 0..nu * NM {
        spot.push(r.i32()?);
    }
    if let Some((c0, c1)) = slice {
        return decode_slice_arrays(r, p, (ymd, unds, cons, spot), nc, c0, c1);
    }
    let mut close = Vec::with_capacity(nc * NM);
    for _ in 0..nc * NM {
        close.push(r.i32()?);
    }
    let flags = r.take(nc * NM)?.to_vec();
    r.align(4);
    let mut rd_i32 = |n: usize| -> Result<Vec<i32>, String> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(r.i32()?);
        }
        Ok(v)
    };
    let high = rd_i32(nc * NM)?;
    let low = rd_i32(nc * NM)?;
    let mut vol = Vec::with_capacity(nc * NM);
    for _ in 0..nc * NM {
        vol.push(r.u32()?);
    }
    let mut oi = Vec::with_capacity(nc * NM);
    for _ in 0..nc * NM {
        oi.push(r.u32()?);
    }
    r.align(8);
    let mut cumvol = Vec::with_capacity(nc * (NM + 1));
    for _ in 0..nc * (NM + 1) {
        cumvol.push(r.u64()?);
    }
    let mut cumval = Vec::with_capacity(nc * (NM + 1));
    for _ in 0..nc * (NM + 1) {
        cumval.push(r.u64()?);
    }
    Ok(Cube { ymd, day: day_of_ymd(ymd), unds, cons, spot, close, flags, high, low, vol, oi, cumvol, cumval, stats: Stats::default(), lotbad: Vec::new(), wickbad: Vec::new(), lim: None })
}

/// v3.4 E8: the per-contract arrays of a segment slice, read for contracts [c0, c1) into full-size arrays (every other contract zero and
/// refused on read through `lim`), so every contract and underlying keeps its index.
fn decode_slice_arrays(mut r: Rd<'_>, p: &Path, head: (u32, Vec<Und>, Vec<Contract>, Vec<i32>), nc: usize, c0: usize, c1: usize) -> Result<Cube, String> {
    let (ymd, unds, cons, spot) = head;
    let (a, z) = (c0 * NM, c1 * NM);
    let mut close = vec![0_i32; nc * NM];
    for x in close.get_mut(a..z).ok_or("slice close")? {
        *x = r.i32()?;
    }
    let mut flags = vec![0_u8; nc * NM];
    flags.get_mut(a..z).ok_or("slice flags")?.copy_from_slice(r.take(z - a)?);
    r.align(4);
    let mut high = vec![0_i32; nc * NM];
    for x in high.get_mut(a..z).ok_or("slice high")? {
        *x = r.i32()?;
    }
    let mut low = vec![0_i32; nc * NM];
    for x in low.get_mut(a..z).ok_or("slice low")? {
        *x = r.i32()?;
    }
    let mut vol = vec![0_u32; nc * NM];
    for x in vol.get_mut(a..z).ok_or("slice vol")? {
        *x = r.u32()?;
    }
    let mut oi = vec![0_u32; nc * NM];
    for x in oi.get_mut(a..z).ok_or("slice oi")? {
        *x = r.u32()?;
    }
    r.align(8);
    let (a1, z1) = (c0 * (NM + 1), c1 * (NM + 1));
    let mut cumvol = vec![0_u64; nc * (NM + 1)];
    for x in cumvol.get_mut(a1..z1).ok_or("slice cumvol")? {
        *x = r.u64()?;
    }
    let mut cumval = vec![0_u64; nc * (NM + 1)];
    for x in cumval.get_mut(a1..z1).ok_or("slice cumval")? {
        *x = r.u64()?;
    }
    if r.p != r.b.len() {
        return Err(format!("{}: {} trailing byte(s) after the slice's arrays", p.display(), r.b.len() - r.p));
    }
    Ok(Cube { ymd, day: day_of_ymd(ymd), unds, cons, spot, close, flags, high, low, vol, oi, cumvol, cumval, stats: Stats::default(), lotbad: Vec::new(), wickbad: Vec::new(), lim: Some(vec![(c0, c1)]) })
}

/// Reads `<d>.sec.bin` back.
///
/// # Errors
/// A missing, truncated or foreign file.
pub fn load_sec(out: &Path, d: &str) -> Result<SecTape, String> {
    let p = out.join(format!("{d}.sec.bin"));
    // v3.4 E9: a shard's transport file (<d>.sfz), decoded in memory; else the tape file as v3.3 reads it
    let b = if crate::kit::TRANSPORT.load(std::sync::atomic::Ordering::Relaxed) { crate::slice::sfz_sec(out, d)? } else { std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))? };
    decode_sec(&b, &p)
}

/// A seconds tape's bytes back: v1, or a v3.4 E8 segment slice (the same layout after its own header; `lim` = [(c0, c1)]).
///
/// # Errors
/// A truncated or foreign file.
pub(crate) fn decode_sec(b: &[u8], p: &Path) -> Result<SecTape, String> {
    let mut r = Rd { b, p: 0 };
    let magic = r.take(8)?;
    let lim = if magic == MAGIC_SEC {
        None
    } else if magic == SLICE_MAGIC_SEC {
        let (sv, _seg, c0, c1) = (r.u32()?, r.u32()?, r.u32()?, r.u32()?);
        if sv != 1 {
            return Err(format!("{}: slice version {sv}", p.display()));
        }
        Some(vec![(usize::try_from(c0).unwrap_or(usize::MAX), usize::try_from(c1).unwrap_or(usize::MAX))])
    } else {
        return Err(format!("{}: not a seconds tape", p.display()));
    };
    let _ver = r.u32()?;
    let _ymd = r.u32()?;
    let nc = usize::try_from(r.u32()?).unwrap_or(0);
    let _ = r.u32()?;
    let nr = usize::try_from(r.u64()?).unwrap_or(0);
    let mut off = Vec::with_capacity(nc + 1);
    for _ in 0..=nc {
        off.push(r.u64()?);
    }
    let mut rows = Vec::with_capacity(nr);
    for _ in 0..nr {
        let sod = r.u32()?;
        let high = r.i32()?;
        let low = r.i32()?;
        let vol = r.u32()?;
        rows.push(Sec { sod, high, low, vol });
    }
    Ok(SecTape { off, rows, lim })
}
