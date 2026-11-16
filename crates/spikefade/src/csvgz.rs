//! Reading the staged Tickvault CSV.gz files: `gzip -dc` as a child process (no new crate), quote-aware field split, CRLF or LF, empty
//! field = NULL, prices to paisa (at most 2 decimals; more are counted and rounded half up), signed integers.

use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};

/// A line reader over `gzip -dc <file>`.
pub struct GzLines {
    child: Child,
    rd: BufReader<ChildStdout>,
    buf: Vec<u8>,
}

impl GzLines {
    /// Opens the file through `/usr/bin/gzip -dc`.
    ///
    /// # Errors
    /// The child could not be started.
    pub fn open(p: &Path) -> Result<Self, String> {
        let mut child = Command::new(crate::kit::tool("gzip")).arg("-dc").arg(p).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| format!("gzip -dc {}: {e}", p.display()))?;
        let out = child.stdout.take().ok_or("gzip: no stdout")?;
        Ok(Self { child, rd: BufReader::with_capacity(1 << 22, out), buf: Vec::with_capacity(512) })
    }

    /// The next line without its line end (CR stripped too), or None at the end.
    ///
    /// # Errors
    /// A read error.
    pub fn next_line(&mut self) -> Result<Option<&[u8]>, String> {
        self.buf.clear();
        let n = self.rd.read_until(b'\n', &mut self.buf).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(None);
        }
        while matches!(self.buf.last(), Some(b'\n' | b'\r')) {
            self.buf.pop();
        }
        Ok(Some(&self.buf))
    }

    /// Waits for the child; Err when gzip failed.
    ///
    /// # Errors
    /// gzip exited with an error (a damaged file).
    pub fn finish(mut self) -> Result<(), String> {
        let mut rest = Vec::new();
        let _ = self.rd.read_to_end(&mut rest);
        let st = self.child.wait().map_err(|e| e.to_string())?;
        if st.success() { Ok(()) } else { Err(format!("gzip exited {st}")) }
    }
}

/// Splits one CSV line into field ranges (quotes stripped; a doubled quote inside quotes is kept as two bytes, never needed here).
pub fn split(line: &[u8], out: &mut Vec<(usize, usize)>) {
    out.clear();
    let mut i = 0;
    let n = line.len();
    loop {
        if i >= n {
            out.push((n, n));
            break;
        }
        if line.get(i) == Some(&b'"') {
            let s = i + 1;
            let mut j = s;
            while j < n {
                if line.get(j) == Some(&b'"') {
                    if line.get(j + 1) == Some(&b'"') {
                        j += 2;
                        continue;
                    }
                    break;
                }
                j += 1;
            }
            out.push((s, j.min(n)));
            // skip the closing quote and the comma
            i = j + 1;
            if line.get(i) == Some(&b',') {
                i += 1;
                if i >= n {
                    out.push((n, n));
                    break;
                }
                continue;
            }
            break;
        }
        let s = i;
        while i < n && line.get(i) != Some(&b',') {
            i += 1;
        }
        out.push((s, i));
        if i >= n {
            break;
        }
        i += 1;
        if i >= n {
            out.push((n, n));
            break;
        }
    }
}

/// The bytes of field `k`.
#[must_use]
pub fn field<'a>(line: &'a [u8], f: &[(usize, usize)], k: usize) -> &'a [u8] {
    f.get(k).and_then(|&(a, b)| line.get(a..b)).unwrap_or(&[])
}

/// A signed integer; empty = None.
#[must_use]
pub fn int(b: &[u8]) -> Option<i64> {
    if b.is_empty() {
        return None;
    }
    let (neg, d) = if b.first() == Some(&b'-') { (true, b.get(1..).unwrap_or(&[])) } else { (false, b) };
    if d.is_empty() {
        return None;
    }
    let mut v: i64 = 0;
    for &c in d {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v.checked_mul(10)?.checked_add(i64::from(c - b'0'))?;
    }
    Some(if neg { -v } else { v })
}

/// A decimal price to paisa: (paisa, had more than 2 decimals). Empty = None.
#[must_use]
pub fn paisa(b: &[u8]) -> Option<(i64, bool)> {
    if b.is_empty() {
        return None;
    }
    let (neg, d) = if b.first() == Some(&b'-') { (true, b.get(1..).unwrap_or(&[])) } else { (false, b) };
    let mut whole: i64 = 0;
    let mut frac: i64 = 0;
    let mut nfrac = 0_u32;
    let mut extra = false;
    let mut round_up = false;
    let mut seen_dot = false;
    let mut any = false;
    for &c in d {
        if c == b'.' {
            if seen_dot {
                return None;
            }
            seen_dot = true;
            continue;
        }
        if !c.is_ascii_digit() {
            return None;
        }
        any = true;
        let x = i64::from(c - b'0');
        if seen_dot {
            if nfrac < 2 {
                frac = frac * 10 + x;
                nfrac += 1;
            } else {
                if !extra && x >= 5 {
                    round_up = true;
                }
                if x != 0 {
                    extra = true;
                }
            }
        } else {
            whole = whole.checked_mul(10)?.checked_add(x)?;
        }
    }
    if !any {
        return None;
    }
    while nfrac < 2 {
        frac *= 10;
        nfrac += 1;
    }
    let mut v = whole.checked_mul(100)?.checked_add(frac)?;
    if round_up {
        v += 1;
    }
    Some((if neg { -v } else { v }, extra))
}

/// "2026-11-11T09:15:00.000000Z" -> (yyyymmdd, second of the day, whole second?).
#[must_use]
pub fn ts(b: &[u8]) -> Option<(u32, i64, bool)> {
    if b.len() < 19 {
        return None;
    }
    let num = |a: usize, n: usize| -> Option<i64> { int(b.get(a..a + n)?) };
    let y = num(0, 4)?;
    let m = num(5, 2)?;
    let d = num(8, 2)?;
    let hh = num(11, 2)?;
    let mm = num(14, 2)?;
    let ss = num(17, 2)?;
    if b.get(4) != Some(&b'-') || b.get(10) != Some(&b'T') || !(0..24).contains(&hh) || !(0..60).contains(&mm) || !(0..61).contains(&ss) {
        return None;
    }
    let frac_zero = b.get(19..).is_none_or(|r| r.iter().all(|&c| c == b'.' || c == b'0' || c == b'Z'));
    let ymd = u32::try_from(y * 10_000 + m * 100 + d).ok()?;
    Some((ymd, hh * 3_600 + mm * 60 + ss, frac_zero))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_vectors() {
        let l = br#""2026-11-11T09:15:00.000000Z","2 seconds","355 milliseconds","57 seconds","dhan","NSE_FNO",45438,"NIFTY-17Nov2026-26050-CE",38.0,52.0,32.55,46.05,-325390,276380,73,123.0,21.18,1702285,41145,2267962576,355423296,57376614128"#;
        let mut f = Vec::new();
        split(l, &mut f);
        assert_eq!(f.len(), 22);
        assert_eq!(field(l, &f, 5), b"NSE_FNO");
        assert_eq!(int(field(l, &f, 6)), Some(45_438));
        assert_eq!(paisa(field(l, &f, 10)), Some((3_255, false)));
        assert_eq!(paisa(b"2792.0"), Some((279_200, false)));
        assert_eq!(paisa(b"0.585"), Some((59, true)));
        assert_eq!(paisa(b"-0.6"), Some((-60, false)));
        assert_eq!(int(field(l, &f, 12)), Some(-325_390));
        assert_eq!(ts(field(l, &f, 0)), Some((20_261_111, 33_300, true)));
        let e = b"a,,\"x,y\",";
        split(e, &mut f);
        assert_eq!(f.len(), 4);
        assert_eq!(field(e, &f, 1), b"");
        assert_eq!(field(e, &f, 2), b"x,y");
        assert_eq!(field(e, &f, 3), b"");
    }
}
