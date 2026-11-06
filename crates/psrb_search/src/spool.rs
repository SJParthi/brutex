//! The spool (SEARCH-SPEC 6.1): phase A's records, one file per (timeframe, exit lane,
//! stock), fixed-size little-endian records in decision order. Phase B and the merge
//! read them back; nothing here ever deletes a file.

use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use crate::common::{ExitKind, LANE_NAMES, Status, TFS, US_S, ist_day_of, window_index};
use crate::filters::FW;
use crate::phase_a::Sym;

/// One would-be trade of a record (WITH or FADE).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Trade {
    pub(crate) status: Status,
    pub(crate) kind: ExitKind,
    pub(crate) fallback: bool,
    pub(crate) late: bool,
    pub(crate) at_once: bool,
    /// Rs 2,50,000 buys at least one share (SIZE AFTER A FALL can trade it).
    pub(crate) half_ok: bool,
    pub(crate) entry_us: i64,
    pub(crate) entry_px: i64,
    pub(crate) qty: i64,
    pub(crate) exit_us: i64,
    pub(crate) exit_px: i64,
    pub(crate) stop0: i64,
    pub(crate) gross: i64,
    pub(crate) charges: i64,
    pub(crate) net: i64,
    pub(crate) gross_h: i64,
    pub(crate) charges_h: i64,
    pub(crate) net_h: i64,
}

/// One record: a signal candle where at least one core holds on this exit lane.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Record {
    /// The signal candle's close instant T.
    pub(crate) t: i64,
    pub(crate) stock: u16,
    /// The signal side: 0 long, 1 short.
    pub(crate) side: u8,
    /// Bit k: core K(k+1) holds.
    pub(crate) cores: u8,
    /// Candle codes of t-2, t-1, t on the signal side's meaning (`u8::MAX`: none).
    pub(crate) codes: [u8; 3],
    /// Filter bits on the signal side's meaning.
    pub(crate) fbits: [u64; FW],
    /// The monthly trade cap's strength score (relative volume, ppm).
    pub(crate) strength: i64,
    /// The signal candle's open, high, low, close (Zerodha, adjusted).
    pub(crate) sig: [i64; 4],
    /// [WITH, FADE].
    pub(crate) tr: [Trade; 2],
}

const TRADE_BYTES: usize = 8 + 12 * 8;
/// Bytes of one record on disk.
pub(crate) const REC_BYTES: usize = 16 + FW * 8 + 8 + 32 + 2 * TRADE_BYTES;

fn status_u8(s: Status) -> u8 {
    match s {
        Status::Filled => 0,
        Status::EntryNotFilled => 1,
        Status::ZeroQty => 2,
        Status::NoRatio => 3,
    }
}

fn status_of(x: u8) -> Status {
    match x {
        0 => Status::Filled,
        2 => Status::ZeroQty,
        3 => Status::NoRatio,
        _ => Status::EntryNotFilled,
    }
}

fn kind_of(x: u8) -> ExitKind {
    match x {
        1 => ExitKind::Flip,
        2 => ExitKind::Time,
        3 => ExitKind::SquareOff,
        4 => ExitKind::Unclosable,
        _ => ExitKind::Stop,
    }
}

fn kind_u8(k: ExitKind) -> u8 {
    u8::try_from(k.index()).unwrap_or(0)
}

fn put64(buf: &mut Vec<u8>, x: i64) {
    buf.extend_from_slice(&x.to_le_bytes());
}

fn get64(b: &[u8], at: usize) -> i64 {
    let mut a = [0_u8; 8];
    if let Some(s) = b.get(at..at + 8) {
        a.copy_from_slice(s);
    }
    i64::from_le_bytes(a)
}

fn getu64(b: &[u8], at: usize) -> u64 {
    u64::from_ne_bytes(get64(b, at).to_ne_bytes())
}

fn byte(b: &[u8], at: usize) -> u8 {
    b.get(at).copied().unwrap_or(0)
}

impl Record {
    /// Append this record's bytes.
    pub(crate) fn encode(&self, buf: &mut Vec<u8>) {
        put64(buf, self.t);
        buf.extend_from_slice(&self.stock.to_le_bytes());
        buf.push(self.side);
        buf.push(self.cores);
        buf.extend_from_slice(&self.codes);
        buf.push(0);
        for w in &self.fbits {
            buf.extend_from_slice(&w.to_le_bytes());
        }
        put64(buf, self.strength);
        for x in &self.sig {
            put64(buf, *x);
        }
        for tr in &self.tr {
            let flags = u8::from(tr.fallback) | (u8::from(tr.late) << 1) | (u8::from(tr.at_once) << 2) | (u8::from(tr.half_ok) << 3);
            buf.extend_from_slice(&[status_u8(tr.status), kind_u8(tr.kind), flags, 0, 0, 0, 0, 0]);
            for x in [
                tr.entry_us, tr.entry_px, tr.qty, tr.exit_us, tr.exit_px, tr.stop0, tr.gross, tr.charges, tr.net, tr.gross_h,
                tr.charges_h, tr.net_h,
            ] {
                put64(buf, x);
            }
        }
    }

    /// Decode one record from `REC_BYTES` bytes.
    pub(crate) fn decode(b: &[u8]) -> Self {
        let mut r = Self {
            t: get64(b, 0),
            stock: u16::from_le_bytes([byte(b, 8), byte(b, 9)]),
            side: byte(b, 10),
            cores: byte(b, 11),
            codes: [byte(b, 12), byte(b, 13), byte(b, 14)],
            ..Self::default()
        };
        let mut at = 16;
        for w in &mut r.fbits {
            *w = getu64(b, at);
            at += 8;
        }
        r.strength = get64(b, at);
        at += 8;
        for x in &mut r.sig {
            *x = get64(b, at);
            at += 8;
        }
        for tr in &mut r.tr {
            let flags = byte(b, at + 2);
            tr.status = status_of(byte(b, at));
            tr.kind = kind_of(byte(b, at + 1));
            tr.fallback = flags & 1 != 0;
            tr.late = flags & 2 != 0;
            tr.at_once = flags & 4 != 0;
            tr.half_ok = flags & 8 != 0;
            at += 8;
            let mut v = [0_i64; 12];
            for x in &mut v {
                *x = get64(b, at);
                at += 8;
            }
            let [a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11] = v;
            tr.entry_us = a0;
            tr.entry_px = a1;
            tr.qty = a2;
            tr.exit_us = a3;
            tr.exit_px = a4;
            tr.stop0 = a5;
            tr.gross = a6;
            tr.charges = a7;
            tr.net = a8;
            tr.gross_h = a9;
            tr.charges_h = a10;
            tr.net_h = a11;
        }
        r
    }
}

/// One trade as the search's accounts read it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HotTrade {
    pub(crate) exit_us: i64,
    pub(crate) net: i64,
    pub(crate) gross: i64,
    pub(crate) charges: i64,
    pub(crate) net_h: i64,
    pub(crate) gross_h: i64,
    pub(crate) charges_h: i64,
    /// Exit fill second - entry fill second.
    pub(crate) hold: u32,
    /// 0 filled, 1 entry not filled, 2 zero quantity, 3 no ratio.
    pub(crate) status: u8,
    /// The window index of the exit's month (`u8::MAX` outside the window).
    pub(crate) month: u8,
    pub(crate) half_ok: bool,
    /// The exit kind (common::ExitKind::index).
    pub(crate) kind: u8,
}

/// One record as phase B and the merge read it.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Hot {
    pub(crate) t: i64,
    pub(crate) stock: u16,
    pub(crate) side: u8,
    pub(crate) cores: u8,
    pub(crate) codes: [u8; 3],
    /// The window index of T's month.
    pub(crate) month_t: u8,
    /// The IST day number of T.
    pub(crate) day: u32,
    pub(crate) strength: i64,
    pub(crate) fbits: [u64; FW],
    pub(crate) tr: [HotTrade; 2],
}

impl Hot {
    pub(crate) fn of(r: &Record) -> Self {
        let tr = r.tr.map(|t| {
            let filled = t.status == Status::Filled;
            HotTrade {
                exit_us: t.exit_us,
                net: t.net,
                gross: t.gross,
                charges: t.charges,
                net_h: t.net_h,
                gross_h: t.gross_h,
                charges_h: t.charges_h,
                hold: if filled { u32::try_from((t.exit_us - t.entry_us).div_euclid(US_S).max(0)).unwrap_or(u32::MAX) } else { 0 },
                status: status_u8(t.status),
                month: if filled { window_index(t.exit_us).and_then(|m| u8::try_from(m).ok()).unwrap_or(u8::MAX) } else { u8::MAX },
                half_ok: t.half_ok,
                kind: kind_u8(t.kind),
            }
        });
        Self {
            t: r.t,
            stock: r.stock,
            side: r.side,
            cores: r.cores,
            codes: r.codes,
            month_t: window_index(r.t).and_then(|m| u8::try_from(m).ok()).unwrap_or(u8::MAX),
            day: u32::try_from(ist_day_of(r.t)).unwrap_or(0),
            strength: r.strength,
            fbits: r.fbits,
            tr,
        }
    }
}

/// The folder of one (timeframe, lane).
pub(crate) fn lane_dir(out: &Path, ti: usize, lane: usize) -> PathBuf {
    let tf = TFS.get(ti).copied().unwrap_or(0);
    out.join("spool").join(format!("tf{tf}")).join(LANE_NAMES.get(lane).copied().unwrap_or("X?"))
}

/// The file of one stock in one (timeframe, lane).
pub(crate) fn stock_file(out: &Path, ti: usize, lane: usize, sym: &Sym) -> PathBuf {
    lane_dir(out, ti, lane).join(format!("{:03}-{}.bin", sym.rank, sym.name))
}

/// Write one stock's records of one (timeframe, lane).
pub(crate) fn write_stock(out: &Path, ti: usize, lane: usize, sym: &Sym, recs: &[Record]) -> Result<u64, String> {
    let dir = lane_dir(out, ti, lane);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let p = stock_file(out, ti, lane, sym);
    let mut buf = Vec::with_capacity(recs.len() * REC_BYTES);
    for r in recs {
        r.encode(&mut buf);
    }
    let mut f = std::fs::File::create(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    f.write_all(&buf).map_err(|e| format!("{}: {e}", p.display()))?;
    u64::try_from(buf.len()).map_err(|_| "size".to_owned())
}

/// Every record of one (timeframe, lane), in admission order (T, then symbol rank).
pub(crate) fn load_full(out: &Path, ti: usize, lane: usize, syms: &[Sym]) -> Result<Vec<Record>, String> {
    let mut v: Vec<Record> = Vec::new();
    for s in syms {
        let p = stock_file(out, ti, lane, s);
        let mut f = match std::fs::File::open(&p) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", p.display())),
        };
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| format!("{}: {e}", p.display()))?;
        if buf.len() % REC_BYTES != 0 {
            return Err(format!("{}: {} bytes is not a whole number of {REC_BYTES}-byte records", p.display(), buf.len()));
        }
        for ch in buf.chunks_exact(REC_BYTES) {
            v.push(Record::decode(ch));
        }
    }
    v.sort_by_key(|r| (r.t, r.stock));
    Ok(v)
}

/// The hot view of one (timeframe, lane), in admission order.
pub(crate) fn load_hot(out: &Path, ti: usize, lane: usize, syms: &[Sym]) -> Result<Vec<Hot>, String> {
    let mut v: Vec<Hot> = Vec::new();
    for s in syms {
        let p = stock_file(out, ti, lane, s);
        let mut f = match std::fs::File::open(&p) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("{}: {e}", p.display())),
        };
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| format!("{}: {e}", p.display()))?;
        if buf.len() % REC_BYTES != 0 {
            return Err(format!("{}: {} bytes is not a whole number of {REC_BYTES}-byte records", p.display(), buf.len()));
        }
        v.reserve(buf.len() / REC_BYTES);
        for ch in buf.chunks_exact(REC_BYTES) {
            v.push(Hot::of(&Record::decode(ch)));
        }
    }
    v.sort_by_key(|r| (r.t, r.stock));
    Ok(v)
}
