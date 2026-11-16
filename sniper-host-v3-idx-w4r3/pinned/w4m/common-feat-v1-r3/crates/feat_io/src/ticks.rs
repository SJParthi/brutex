//! The GDFL tick store reader (spec `/Volumes/WD_BLACK/brutex/tickstore-data/FORMAT.md` v1, `BRTXTS01`), read
//! strictly as the specification states (the project's `pull::gdfl_tickstore` reads the same spec; its zstd decoder,
//! `ruzstd`, is the one used here). Every member file is rebuilt byte for byte and held to the index's size and CRC-32
//! ("a reader must refuse otherwise", spec section 4).
//!
//! **LTQ0-HL-1 (heart piece):** the store's 1-s bars keep only LTQ > 0 rows. A row with LTQ = 0 whose LTP differs from
//! the row before it is a REAL trade (its volume sits in a sibling row): it is KEPT here. A second's OPEN / HIGH / LOW /
//! CLOSE are over its trade rows (LTQ > 0, or LTQ = 0 with a new price); its VOLUME is the sum of LTQ. A second whose
//! only trade rows are new-price LTQ = 0 rows has volume 0 and is flagged `price_only` (crate v2's S2 reads it as no
//! print; counted).

use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::crc::crc32;

const MAGIC: &[u8; 8] = b"BRTXTS01";
const FOOTER_MAGIC: &[u8; 8] = b"BRTXTSE1";
const MAX_WINDOW: u64 = 128 << 20;

/// One tick row of a member file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickRow {
    /// Seconds since IST midnight.
    pub sod: i32,
    /// LTP, paisa.
    pub ltp: i64,
    /// LTQ, shares.
    pub ltq: u64,
}

/// One second of the corrected (new-price) fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sec {
    /// Seconds since IST midnight.
    pub sod: i32,
    /// Open over the trade rows, paisa.
    pub open: i64,
    /// High over the trade rows.
    pub high: i64,
    /// Low over the trade rows.
    pub low: i64,
    /// Close over the trade rows.
    pub close: i64,
    /// Sum of LTQ.
    pub volume: u64,
    /// The store rule (LTQ > 0 rows only): (open, high, low, close), `None` when the second has no LTQ > 0 row.
    pub traded: Option<(i64, i64, i64, i64)>,
}

impl Sec {
    /// Only new-price LTQ = 0 rows: volume 0 (crate v2 S2 reads it as no print).
    #[must_use]
    pub const fn price_only(&self) -> bool {
        self.volume == 0
    }
}

fn unzstd(frame: &[u8], want: u64) -> Result<Vec<u8>, String> {
    let dec = ruzstd::decoding::StreamingDecoder::new_with_max_window_size(frame, MAX_WINDOW)
        .map_err(|e| format!("zstd frame: {e}"))?;
    let mut out = Vec::new();
    std::io::Read::take(dec, want.saturating_add(1)).read_to_end(&mut out).map_err(|e| format!("zstd: {e}"))?;
    if u64::try_from(out.len()).unwrap_or(u64::MAX) == want {
        Ok(out)
    } else {
        Err(format!("zstd length {} != {want}", out.len()))
    }
}

struct Cur<'a> {
    b: &'a [u8],
    at: usize,
}

impl Cur<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.at.checked_add(n).ok_or("overflow")?;
        let s = self.b.get(self.at..end).ok_or_else(|| format!("short read at {} + {n}", self.at))?;
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(*self.take(1)?.first().ok_or("short")?)
    }
    fn u16(&mut self) -> Result<u16, String> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes(s.try_into().map_err(|_| "u16")?))
    }
    fn u32(&mut self) -> Result<u32, String> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes(s.try_into().map_err(|_| "u32")?))
    }
    fn u64(&mut self) -> Result<u64, String> {
        let s = self.take(8)?;
        Ok(u64::from_le_bytes(s.try_into().map_err(|_| "u64")?))
    }
}

fn read_at(f: &mut File, off: u64, len: u64) -> Result<Vec<u8>, String> {
    use std::io::{Seek, SeekFrom};
    f.seek(SeekFrom::Start(off)).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; usize::try_from(len).map_err(|_| "len")?];
    f.read_exact(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

/// One index entry.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The zip entry name.
    pub name: String,
    /// 0 dir, 1 columnar, 2 raw.
    pub kind: u8,
    off: u64,
    len: u64,
    size: u64,
    crc: u32,
    rows: u64,
}

/// Reads a day file's index (footer, zstd index, CRC).
///
/// # Errors
///
/// Any rule of spec section 3 broken.
pub fn read_index(path: &Path) -> Result<(File, Vec<Entry>), String> {
    let mut f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let len = f.metadata().map_err(|e| e.to_string())?.len();
    if len < 48 || read_at(&mut f, 0, 8)? != MAGIC {
        return Err("no BRTXTS01 magic".into());
    }
    let foot = read_at(&mut f, len - 40, 40)?;
    let mut c = Cur { b: &foot, at: 0 };
    let (ioff, ilen, iraw, icrc, n) = (c.u64()?, c.u64()?, c.u64()?, c.u32()?, c.u32()?);
    if c.take(8)? != FOOTER_MAGIC {
        return Err("no footer magic".into());
    }
    if ioff.checked_add(ilen).and_then(|x| x.checked_add(40)) != Some(len) || ioff < 8 {
        return Err("index not at the end".into());
    }
    let index = unzstd(&read_at(&mut f, ioff, ilen)?, iraw)?;
    if crc32(&index) != icrc {
        return Err("index CRC".into());
    }
    let mut c = Cur { b: &index, at: 0 };
    let mut v = Vec::new();
    for _ in 0..n {
        let nl = usize::from(c.u16()?);
        let name = String::from_utf8_lossy(c.take(nl)?).into_owned();
        let kind = c.u8()?;
        let off = c.u64()?;
        let blen = c.u64()?;
        let size = c.u64()?;
        let crc = c.u32()?;
        let rows = c.u64()?;
        let _dos = c.u32()?;
        if kind != 0 && (off < 8 || off.checked_add(blen).is_none_or(|e| e > ioff)) {
            return Err(format!("block range of {name}"));
        }
        v.push(Entry { name, kind, off, len: blen, size, crc, rows });
    }
    Ok((f, v))
}

/// The basename of an entry name.
#[must_use]
pub fn basename(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// A column decoded: text fields or numeric `(shape, v)` pairs.
enum Col {
    Text(Vec<String>),
    Num(Vec<(u8, i64)>),
}

fn render(shape: u8, v: i64) -> String {
    if shape & 0x20 != 0 {
        return format!("{:02}:{:02}:{:02}", v / 3600, (v / 60) % 60, v % 60);
    }
    let d = usize::from(shape & 0x0f);
    let mut s = format!("{:0w$}", v.unsigned_abs(), w = d + 1);
    if d > 0 {
        s.insert(s.len() - d, '.');
    }
    if shape & 0x10 != 0 { format!("-{s}") } else { s }
}

fn decode_column(c: &mut Cur<'_>, rows: usize) -> Result<Col, String> {
    let tag = c.u8()?;
    let plen = c.u64()?;
    let _tlen = c.u64()?;
    let zlen = usize::try_from(c.u64()?).map_err(|_| "zlen")?;
    let payload = unzstd(c.take(zlen)?, plen)?;
    if tag == 0 {
        let text = String::from_utf8_lossy(&payload).into_owned();
        let f: Vec<String> = if rows == 0 { Vec::new() } else { text.split('\n').map(str::to_owned).collect() };
        if f.len() != rows {
            return Err(format!("text column rows {} != {rows}", f.len()));
        }
        return Ok(Col::Text(f));
    }
    let shape = payload.get(..rows).ok_or("shape")?;
    let delta = *payload.get(rows).ok_or("delta")?;
    let w = usize::from(*payload.get(rows + 1).ok_or("w")?);
    let planes = payload.get(rows + 2..rows + 2 + w * rows).ok_or("planes")?;
    let mut out = Vec::with_capacity(rows);
    let mut prev: i64 = 0;
    for i in 0..rows {
        let mut x: u64 = 0;
        for k in 0..w {
            let byte = *planes.get(k * rows + i).ok_or("plane")?;
            x |= u64::from(byte) << (8 * k);
        }
        let z = i64::from_ne_bytes(((x >> 1) ^ (x & 1).wrapping_neg()).to_ne_bytes());
        let v = if delta == 1 { prev.wrapping_add(z) } else { z };
        prev = v;
        out.push((*shape.get(i).ok_or("shape")?, v));
    }
    Ok(Col::Num(out))
}

fn field(col: &Col, i: usize) -> Option<String> {
    match col {
        Col::Text(t) => t.get(i).cloned(),
        Col::Num(n) => n.get(i).map(|&(s, v)| render(s, v)),
    }
}

/// A price field's value in paisa (decimals <= 2, or a value whose extra decimals are zero).
fn paisa(col: &Col, i: usize) -> Option<i64> {
    match col {
        Col::Num(n) => {
            let &(s, v) = n.get(i)?;
            if s & 0x20 != 0 {
                return None;
            }
            let d = u32::from(s & 0x0f);
            let mag = i64::try_from(v.unsigned_abs()).ok()?;
            let mag = if d <= 2 {
                mag.checked_mul(10i64.pow(2 - d))?
            } else {
                let q = 10i64.pow(d - 2);
                if mag % q != 0 {
                    return None;
                }
                mag / q
            };
            Some(if s & 0x10 != 0 { -mag } else { mag })
        }
        Col::Text(t) => parse_paisa(t.get(i)?),
    }
}

/// `"1825.5"` -> 182550 paisa (at most two decimals, or zeros beyond).
#[must_use]
pub fn parse_paisa(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let i: i64 = if int.is_empty() { 0 } else { int.parse().ok()? };
    let mut f = frac.to_owned();
    while f.len() > 2 && f.ends_with('0') {
        f.pop();
    }
    if f.len() > 2 || !f.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    while f.len() < 2 {
        f.push('0');
    }
    let v = i.checked_mul(100)?.checked_add(f.parse::<i64>().ok()?)?;
    Some(if neg { -v } else { v })
}

fn sod_of(col: &Col, i: usize) -> Option<i32> {
    match col {
        Col::Num(n) => {
            let &(s, v) = n.get(i)?;
            if s & 0x20 == 0 {
                return None;
            }
            i32::try_from(v).ok()
        }
        Col::Text(t) => {
            let x = t.get(i)?;
            let mut it = x.split(':');
            let h: i32 = it.next()?.parse().ok()?;
            let m: i32 = it.next()?.parse().ok()?;
            let s: i32 = it.next()?.parse().ok()?;
            Some(h * 3600 + m * 60 + s)
        }
    }
}

/// What reading a member file found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemberInfo {
    /// Data rows.
    pub rows: u64,
    /// The rebuilt file equalled the index's size and CRC-32.
    pub crc_ok: bool,
    /// Rows refused (an unparseable time, price or quantity).
    pub bad_rows: u64,
}

/// Reads the rows of the member whose basename is exactly `member` (e.g. `ADANIENT.NSE.csv`; the block-deal member
/// `ADANIENT.BL.NSE.csv` is a different name and is never read). `Ok(None)` when the day holds no such member.
///
/// # Errors
///
/// A malformed file, or a rebuilt member whose size or CRC-32 differs from the index (refused, never read).
pub fn read_member(path: &Path, member: &str) -> Result<Option<(Vec<TickRow>, MemberInfo)>, String> {
    let (mut f, entries) = read_index(path)?;
    let Some(e) = entries.iter().find(|e| basename(&e.name) == member) else {
        return Ok(None);
    };
    let blk = read_at(&mut f, e.off, e.len)?;
    let mut c = Cur { b: &blk, at: 0 };
    let kind = c.u8()?;
    let (header, cols, rows, crlf, trailing): (String, Vec<Col>, usize, bool, bool) = match kind {
        1 => {
            let crlf = c.u8()? == 1;
            let trailing = c.u8()? == 1;
            let hl = usize::try_from(c.u32()?).map_err(|_| "hl")?;
            let header = String::from_utf8_lossy(c.take(hl)?).into_owned();
            let ncols = c.u32()?;
            let rows = usize::try_from(c.u64()?).map_err(|_| "rows")?;
            if u64::try_from(rows).ok() != Some(e.rows) {
                return Err(format!("{member}: rows {rows} != index {}", e.rows));
            }
            let mut cols = Vec::new();
            for _ in 0..ncols {
                cols.push(decode_column(&mut c, rows)?);
            }
            (header, cols, rows, crlf, trailing)
        }
        2 => {
            let raw = unzstd(c.take(blk.len().saturating_sub(1))?, e.size)?;
            if crc32(&raw) != e.crc {
                return Err(format!("{member}: raw CRC"));
            }
            let text = String::from_utf8_lossy(&raw).into_owned();
            let mut lines = text.lines();
            let header = lines.next().unwrap_or("").to_owned();
            let mut tcols: Vec<Vec<String>> = vec![Vec::new(); header.split(',').count()];
            let mut rows = 0usize;
            for l in lines.filter(|l| !l.is_empty()) {
                for (j, fv) in l.split(',').enumerate() {
                    if let Some(col) = tcols.get_mut(j) {
                        col.push(fv.trim_end_matches('\r').to_owned());
                    }
                }
                rows += 1;
            }
            (header, tcols.into_iter().map(Col::Text).collect(), rows, false, false)
        }
        _ => return Err(format!("{member}: entry kind {kind}")),
    };
    let names: Vec<&str> = header.split(',').collect();
    let at = |n: &str| names.iter().position(|x| *x == n);
    let (Some(it), Some(ip), Some(iq)) = (at("Time"), at("LTP"), at("LTQ")) else {
        return Err(format!("{member}: header without Time / LTP / LTQ: {header}"));
    };
    let mut info = MemberInfo { rows: u64::try_from(rows).unwrap_or(0), crc_ok: kind == 2, bad_rows: 0 };
    if kind == 1 {
        // Rebuild (spec section 4) and hold it to the index's size and CRC-32.
        let term: &[u8] = if crlf { b"\r\n" } else { b"\n" };
        let mut buf: Vec<u8> = Vec::with_capacity(usize::try_from(e.size).unwrap_or(0));
        buf.extend_from_slice(header.as_bytes());
        for i in 0..rows {
            buf.extend_from_slice(term);
            for (j, col) in cols.iter().enumerate() {
                if j > 0 {
                    buf.push(b',');
                }
                buf.extend_from_slice(field(col, i).unwrap_or_default().as_bytes());
            }
        }
        if trailing {
            buf.extend_from_slice(term);
        }
        if u64::try_from(buf.len()).ok() != Some(e.size) || crc32(&buf) != e.crc {
            return Err(format!("{member}: rebuilt size {} / CRC differs from the index (refused)", buf.len()));
        }
        info.crc_ok = true;
    }
    let (Some(ct), Some(cp), Some(cq)) = (cols.get(it), cols.get(ip), cols.get(iq)) else {
        return Err("columns".into());
    };
    let mut out = Vec::with_capacity(rows);
    for i in 0..rows {
        let (Some(sod), Some(ltp), Some(q)) = (sod_of(ct, i), paisa(cp, i), paisa(cq, i)) else {
            info.bad_rows += 1;
            continue;
        };
        if q < 0 || q % 100 != 0 || ltp <= 0 {
            info.bad_rows += 1;
            continue;
        }
        out.push(TickRow { sod, ltp, ltq: u64::try_from(q / 100).unwrap_or(0) });
    }
    Ok(Some((out, info)))
}

/// The corrected 1-s fold of one day's rows over `[from_sod, to_sod)`: trade rows are LTQ > 0 rows and LTQ = 0 rows
/// whose LTP differs from the row before (in file order, every row counted for "the row before"). Returns the seconds
/// with any trade row, in time order, and the count of rows stamped out of order (folded into their own second).
#[must_use]
pub fn fold_seconds(rows: &[TickRow], from_sod: i32, to_sod: i32) -> (Vec<Sec>, u64) {
    let span = usize::try_from((to_sod - from_sod).max(0)).unwrap_or(0);
    let mut slots: Vec<Option<Sec>> = vec![None; span];
    let mut prev_ltp: Option<i64> = None;
    let mut last_sod = i32::MIN;
    let mut out_of_order = 0u64;
    for r in rows {
        let is_new = prev_ltp != Some(r.ltp);
        prev_ltp = Some(r.ltp);
        if r.sod < last_sod {
            out_of_order += 1;
        }
        last_sod = last_sod.max(r.sod);
        if r.sod < from_sod || r.sod >= to_sod {
            continue;
        }
        let trade = r.ltq > 0 || is_new;
        let Some(slot) = usize::try_from(r.sod - from_sod).ok().and_then(|i| slots.get_mut(i)) else {
            continue;
        };
        if !trade {
            if let Some(s) = slot.as_mut() {
                s.volume += r.ltq;
            }
            continue;
        }
        match slot {
            Some(s) => {
                s.high = s.high.max(r.ltp);
                s.low = s.low.min(r.ltp);
                s.close = r.ltp;
                s.volume += r.ltq;
            }
            None => {
                *slot = Some(Sec { sod: r.sod, open: r.ltp, high: r.ltp, low: r.ltp, close: r.ltp, volume: r.ltq, traded: None });
            }
        }
        if r.ltq > 0
            && let Some(s) = slot.as_mut()
        {
            s.traded = Some(match s.traded {
                None => (r.ltp, r.ltp, r.ltp, r.ltp),
                Some((o, h, l, _)) => (o, h.max(r.ltp), l.min(r.ltp), r.ltp),
            });
        }
    }
    (slots.into_iter().flatten().collect(), out_of_order)
}

/// The tick store day file of an IST day (`TS/cm/STOCKS/<YYYY>/<MON_YYYY>/GFDLCM_STOCK_TICK_<DDMMYYYY>.bts`).
#[must_use]
pub fn day_path(root: &Path, y: i32, m: u32, d: u32) -> std::path::PathBuf {
    const MON: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
    let mon = usize::try_from(m.saturating_sub(1)).ok().and_then(|i| MON.get(i)).copied().unwrap_or("JAN");
    root.join("cm")
        .join("STOCKS")
        .join(format!("{y:04}"))
        .join(format!("{mon}_{y:04}"))
        .join(format!("GFDLCM_STOCK_TICK_{d:02}{m:02}{y:04}.bts"))
}
