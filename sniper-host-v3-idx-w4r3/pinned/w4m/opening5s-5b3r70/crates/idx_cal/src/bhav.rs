//! The raw NSE F&O bhavcopy zips (one CSV member each), read only: the zip's central directory is parsed here and the
//! member inflated with DEFLATE (or read as stored). The OPTIDX rows of one index give each contract's expiry, strike,
//! side, UDiFF `FinInstrmId` (the contract identity across re-dates) and UDiFF `UndrlygPric`.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::day::{self, Day, MONTHS};

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]))
}
fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([*b.get(at)?, *b.get(at + 1)?, *b.get(at + 2)?, *b.get(at + 3)?]))
}

/// The bytes of the zip's FIRST member (the bhavcopy zips hold one CSV each; a zip with another count is refused).
///
/// # Errors
/// An unreadable file, a malformed zip, a member count other than 1, an unknown compression method, or a length that
/// differs from the directory's.
pub fn read_single_member(path: &Path) -> Result<Vec<u8>, String> {
    let b = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let n = b.len();
    // End of central directory: the last 0x06054b50 within the final 64 KiB + 22 bytes.
    let lo = n.saturating_sub(65_557);
    let mut eocd = None;
    let mut i = n.saturating_sub(22);
    loop {
        if u32_at(&b, i) == Some(0x0605_4b50) {
            eocd = Some(i);
            break;
        }
        if i <= lo {
            break;
        }
        i -= 1;
    }
    let e = eocd.ok_or_else(|| format!("{}: no end of central directory", path.display()))?;
    let entries = u16_at(&b, e + 10).ok_or("eocd short")?;
    if entries != 1 {
        return Err(format!("{}: {entries} members (1 expected)", path.display()));
    }
    let cd = usize::try_from(u32_at(&b, e + 16).ok_or("eocd short")?).map_err(|e| e.to_string())?;
    if u32_at(&b, cd) != Some(0x0201_4b50) {
        return Err(format!("{}: bad central directory", path.display()));
    }
    let method = u16_at(&b, cd + 10).ok_or("cd short")?;
    let csize = usize::try_from(u32_at(&b, cd + 20).ok_or("cd short")?).map_err(|e| e.to_string())?;
    let usize_ = usize::try_from(u32_at(&b, cd + 24).ok_or("cd short")?).map_err(|e| e.to_string())?;
    let lho = usize::try_from(u32_at(&b, cd + 42).ok_or("cd short")?).map_err(|e| e.to_string())?;
    if u32_at(&b, lho) != Some(0x0403_4b50) {
        return Err(format!("{}: bad local header", path.display()));
    }
    let nlen = usize::from(u16_at(&b, lho + 26).ok_or("lh short")?);
    let xlen = usize::from(u16_at(&b, lho + 28).ok_or("lh short")?);
    let start = lho + 30 + nlen + xlen;
    let data = b.get(start..start + csize).ok_or_else(|| format!("{}: member past the end", path.display()))?;
    let out = match method {
        0 => data.to_vec(),
        8 => {
            let mut v = Vec::with_capacity(usize_);
            flate2::read::DeflateDecoder::new(data).read_to_end(&mut v).map_err(|e| format!("{}: inflate {e}", path.display()))?;
            v
        }
        m => return Err(format!("{}: compression method {m}", path.display())),
    };
    if out.len() != usize_ {
        return Err(format!("{}: member {} bytes, directory says {usize_}", path.display(), out.len()));
    }
    Ok(out)
}

/// The zip of the file dated `d` in its form, searched in `roots` in order.
#[must_use]
pub fn bhav_path(roots: &[PathBuf], d: Day, udiff: bool) -> Option<PathBuf> {
    let (y, m, dd) = day::to_civil(d);
    let name = if udiff {
        format!("BhavCopy_NSE_FO_0_0_0_{y:04}{m:02}{dd:02}_F_0000.csv.zip")
    } else {
        format!("fo{dd:02}{}{y:04}bhav.csv.zip", MONTHS.get(usize::try_from(m).ok()?.checked_sub(1)?)?)
    };
    roots.iter().map(|r| r.join(&name)).find(|p| p.is_file())
}

/// One OPTIDX contract row of an index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Row {
    /// Listed expiry.
    pub expiry: Day,
    /// Strike, paisa.
    pub strike: i64,
    /// CALL (true) or PUT.
    pub ce: bool,
    /// UDiFF `FinInstrmId` (`None` in the old form).
    pub fin_id: Option<u64>,
    /// UDiFF `UndrlygPric`, paisa (`None` in the old form).
    pub underlying: Option<i64>,
    /// Lot (UDiFF `NewBrdLotQty`; `None` in the old form).
    pub lot: Option<i64>,
}

/// A decimal text in rupees to paisa (`21400.00`, `9100`, `1234.5`).
#[must_use]
pub fn paisa(s: &str) -> Option<i64> {
    let t = s.trim();
    let (a, b) = t.split_once('.').unwrap_or((t, ""));
    let neg = a.starts_with('-');
    let a = a.trim_start_matches('-');
    if a.is_empty() || !a.bytes().all(|c| c.is_ascii_digit()) || !b.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let r: i64 = a.parse().ok()?;
    let mut frac = String::from(b);
    if frac.len() > 2 {
        if frac.bytes().skip(2).any(|c| c != b'0') {
            return None;
        }
        frac.truncate(2);
    }
    while frac.len() < 2 {
        frac.push('0');
    }
    let p = r.checked_mul(100)?.checked_add(frac.parse::<i64>().ok()?)?;
    Some(if neg { -p } else { p })
}

/// The OPTIDX rows of `index` in one bhavcopy CSV (either form, told by its header).
///
/// # Errors
/// A missing header column or a malformed row of the index.
pub fn index_rows(csv: &[u8], index: &str) -> Result<Vec<Row>, String> {
    let text = String::from_utf8_lossy(csv);
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().ok_or("empty csv")?.split(',').map(str::trim).collect();
    let col = |n: &str| head.iter().position(|h| *h == n);
    let mut v = Vec::new();
    if let Some(sym) = col("TckrSymb") {
        let (tp, id, xp, sk, ot, up, lot) = (
            col("FinInstrmTp").ok_or("no FinInstrmTp")?,
            col("FinInstrmId").ok_or("no FinInstrmId")?,
            col("XpryDt").ok_or("no XpryDt")?,
            col("StrkPric").ok_or("no StrkPric")?,
            col("OptnTp").ok_or("no OptnTp")?,
            col("UndrlygPric").ok_or("no UndrlygPric")?,
            col("NewBrdLotQty").ok_or("no NewBrdLotQty")?,
        );
        for l in lines {
            let c: Vec<&str> = l.split(',').collect();
            if c.get(tp).copied() != Some("IDO") || c.get(sym).copied() != Some(index) {
                continue;
            }
            let ce = match c.get(ot).copied() {
                Some("CE") => true,
                Some("PE") => false,
                o => return Err(format!("udiff option type {o:?}")),
            };
            v.push(Row {
                expiry: c.get(xp).and_then(|x| day::parse(x)).ok_or_else(|| format!("udiff expiry in {l}"))?,
                strike: c.get(sk).and_then(|x| paisa(x)).ok_or_else(|| format!("udiff strike in {l}"))?,
                ce,
                fin_id: Some(c.get(id).and_then(|x| x.trim().parse().ok()).ok_or_else(|| format!("udiff id in {l}"))?),
                underlying: c.get(up).and_then(|x| paisa(x)),
                lot: c.get(lot).and_then(|x| x.trim().parse().ok()),
            });
        }
    } else {
        let (ins, sym, xp, sk, ot) = (
            col("INSTRUMENT").ok_or("no INSTRUMENT")?,
            col("SYMBOL").ok_or("no SYMBOL")?,
            col("EXPIRY_DT").ok_or("no EXPIRY_DT")?,
            col("STRIKE_PR").ok_or("no STRIKE_PR")?,
            col("OPTION_TYP").ok_or("no OPTION_TYP")?,
        );
        for l in lines {
            let c: Vec<&str> = l.split(',').collect();
            if c.get(ins).copied() != Some("OPTIDX") || c.get(sym).copied() != Some(index) {
                continue;
            }
            let ce = match c.get(ot).copied() {
                Some("CE") => true,
                Some("PE") => false,
                o => return Err(format!("old option type {o:?}")),
            };
            v.push(Row {
                expiry: c.get(xp).and_then(|x| day::parse_dmy(x)).ok_or_else(|| format!("old expiry in {l}"))?,
                strike: c.get(sk).and_then(|x| paisa(x)).ok_or_else(|| format!("old strike in {l}"))?,
                ce,
                fin_id: None,
                underlying: None,
                lot: None,
            });
        }
    }
    v.sort();
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paisa_texts() {
        assert_eq!(paisa("21400.00"), Some(2_140_000));
        assert_eq!(paisa("9100"), Some(910_000));
        assert_eq!(paisa("1234.5"), Some(123_450));
        assert_eq!(paisa("1.050"), Some(105));
        assert_eq!(paisa("1.055"), None);
        assert_eq!(paisa("x"), None);
    }

    #[test]
    fn rows_of_both_forms() {
        let old = b"INSTRUMENT,SYMBOL,EXPIRY_DT,STRIKE_PR,OPTION_TYP,OPEN\nOPTIDX,NIFTY,04-Apr-2019,9100,CE,0\nOPTIDX,BANKNIFTY,04-Apr-2019,9100,CE,0\nFUTIDX,NIFTY,25-Apr-2019,0,XX,1\n";
        let r = index_rows(old, "NIFTY").unwrap_or_default();
        assert_eq!(r.len(), 1);
        let u = b"TradDt,FinInstrmTp,FinInstrmId,TckrSymb,XpryDt,StrkPric,OptnTp,UndrlygPric,NewBrdLotQty\n2024-01-01,IDO,72884,NIFTY,2024-02-29,21400.00,CE,21741.90,50\n2024-01-01,STO,1,NIFTY,2024-02-29,1.00,CE,1,1\n";
        let r = index_rows(u, "NIFTY").unwrap_or_default();
        assert_eq!(r.len(), 1);
        assert_eq!(r.first().and_then(|x| x.fin_id), Some(72_884));
        assert_eq!(r.first().and_then(|x| x.underlying), Some(2_174_190));
        assert_eq!(r.first().and_then(|x| x.lot), Some(50));
    }
}
