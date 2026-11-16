//! The inputs as text: `LISTED-EXPIRIES-IDX.tsv` (one row per bhavcopy file and index, every listed expiry), the dated
//! session table (`nse-sessions-tickstore.txt`), the special-session rows (`SESSIONS-SPECIAL-v1.tsv`).

use std::collections::{BTreeMap, BTreeSet};

use crate::day::{self, Day};

/// One listed expiry of one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Listed {
    /// The listed expiry date.
    pub expiry: Day,
    /// The UDiFF "actual expiry" when it differs from the listed one (`listed>actual` in the file).
    pub actual: Option<Day>,
    /// CE rows / PE rows of that expiry in the file.
    pub ce: u32,
    /// PE rows.
    pub pe: u32,
}

/// One bhavcopy file of one index.
#[derive(Clone, Debug)]
pub struct FileRow {
    /// The file's date (= the session it reports).
    pub file: Day,
    /// The form: true = UDiFF, false = the old form.
    pub udiff: bool,
    /// Every listed expiry, ascending.
    pub listed: Vec<Listed>,
}

/// The listing per index, files ascending.
pub type Listing = BTreeMap<String, Vec<FileRow>>;

/// Parses `LISTED-EXPIRIES-IDX.tsv`.
///
/// # Errors
/// A malformed row (named by its line number).
pub fn parse_listing(text: &str) -> Result<Listing, String> {
    let mut out: Listing = BTreeMap::new();
    for (i, l) in text.lines().enumerate() {
        if l.is_empty() || l.starts_with('#') || l.starts_with("file_date") {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let (Some(fd), Some(form), Some(ix), Some(ex)) = (c.first(), c.get(1), c.get(2), c.get(3)) else {
            return Err(format!("listing line {}: {} columns", i + 1, c.len()));
        };
        let file = day::parse(fd).ok_or_else(|| format!("listing line {}: file date {fd}", i + 1))?;
        let udiff = match *form {
            "udiff" => true,
            "old" => false,
            _ => return Err(format!("listing line {}: form {form}", i + 1)),
        };
        let mut listed = Vec::new();
        for item in ex.split(',') {
            let mut p = item.split(':');
            let dates = p.next().unwrap_or("");
            let ce: u32 = p.next().and_then(|x| x.parse().ok()).ok_or_else(|| format!("listing line {}: {item}", i + 1))?;
            let pe: u32 = p.next().and_then(|x| x.parse().ok()).ok_or_else(|| format!("listing line {}: {item}", i + 1))?;
            let (a, b) = match dates.split_once('>') {
                Some((a, b)) => (a, Some(b)),
                None => (dates, None),
            };
            let expiry = day::parse(a).ok_or_else(|| format!("listing line {}: expiry {a}", i + 1))?;
            let actual = match b {
                Some(b) => Some(day::parse(b).ok_or_else(|| format!("listing line {}: actual {b}", i + 1))?),
                None => None,
            };
            listed.push(Listed { expiry, actual, ce, pe });
        }
        listed.sort_by_key(|x| x.expiry);
        out.entry((*ix).to_string()).or_default().push(FileRow { file, udiff, listed });
    }
    for v in out.values_mut() {
        v.sort_by_key(|r| r.file);
        if v.windows(2).any(|w| w.first().map(|a| a.file) == w.get(1).map(|b| b.file)) {
            return Err("listing: a file date appears twice for one index".to_string());
        }
    }
    Ok(out)
}

/// Parses a plain list of `YYYY-MM-DD` lines.
///
/// # Errors
/// A line that is not a date.
pub fn parse_days(text: &str) -> Result<BTreeSet<Day>, String> {
    let mut s = BTreeSet::new();
    for (i, l) in text.lines().enumerate() {
        let t = l.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        s.insert(day::parse(t).ok_or_else(|| format!("day list line {}: {t}", i + 1))?);
    }
    Ok(s)
}

/// One row of `SESSIONS-SPECIAL-v1.tsv`.
#[derive(Clone, Debug)]
pub struct Special {
    /// The date.
    pub day: Day,
    /// Its kind (`MUHURAT`, `DR`, `WEEKEND`, `OUTAGE`, `HALT`, `DEFECT_D9`, ...).
    pub kind: String,
    /// Its segments column as written.
    pub segments: String,
}

/// Parses `SESSIONS-SPECIAL-v1.tsv`.
///
/// # Errors
/// A malformed row.
pub fn parse_specials(text: &str) -> Result<Vec<Special>, String> {
    let mut v = Vec::new();
    for (i, l) in text.lines().enumerate() {
        if l.is_empty() || l.starts_with('#') || l.starts_with("date\t") {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let (Some(d), Some(k), Some(s)) = (c.first(), c.get(1), c.get(2)) else {
            return Err(format!("specials line {}", i + 1));
        };
        v.push(Special { day: day::parse(d).ok_or_else(|| format!("specials line {}: {d}", i + 1))?, kind: (*k).to_string(), segments: (*s).to_string() });
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn listing_rows_parse_with_actual_dates() {
        let t = "# c\nfile_date\tform\tindex\texpiries\n2018-09-03\told\tNIFTY\t2018-09-27:104:104,2018-10-25:73:73\n2025-01-02\tudiff\tBANKNIFTY\t2025-01-30>2025-01-31:5:6\n";
        let l = parse_listing(t).unwrap_or_default();
        assert_eq!(l.len(), 2);
        let b = l.get("BANKNIFTY").and_then(|v| v.first()).cloned();
        assert!(b.as_ref().is_some_and(|r| r.udiff && r.listed.len() == 1));
        assert_eq!(b.and_then(|r| r.listed.first().and_then(|x| x.actual)), day::parse("2025-01-31"));
        assert!(parse_listing("2018-09-03\tnew\tNIFTY\t2018-09-27:1:1\n").is_err());
    }
}
