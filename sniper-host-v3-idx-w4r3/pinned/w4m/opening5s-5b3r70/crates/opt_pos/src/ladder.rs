//! The listed ladder (`LISTED-CONTRACTS-IDX.tsv`, sha16 1d7be966f0314fee: the first two expiries strictly after each
//! bhavcopy file's date, strikes as arithmetic runs, UDiFF lots or the old form's lot candidates) and the listing of every
//! expiry per file (`LISTED-EXPIRIES-IDX.tsv`, via `idx_cal::listing`).

use std::collections::BTreeMap;

use idx_cal::day::{self, Day};

/// One (file, index, expiry) row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LadderRow {
    /// UDiFF form.
    pub udiff: bool,
    /// CE strikes (paisa), ascending.
    pub ce: Vec<i64>,
    /// PE strikes.
    pub pe: Vec<i64>,
    /// The stated lot (UDiFF `NewBrdLotQty`), when the form states one.
    pub lot: Option<i64>,
    /// The old form's lot candidates (the GCDs of the file's expiries with enough open interest).
    pub cands: Vec<i64>,
}

/// Every row, keyed by (index, file date, expiry).
#[derive(Clone, Debug, Default)]
pub struct Ladder {
    rows: BTreeMap<(String, Day, Day), LadderRow>,
}

fn runs(s: &str) -> Result<Vec<i64>, String> {
    let mut v = Vec::new();
    for r in s.split(',') {
        match r.split_once('+') {
            None => v.push(r.trim().parse::<i64>().map_err(|_| format!("run {r}"))?),
            Some((a, rest)) => {
                let (step, n) = rest.split_once('*').ok_or_else(|| format!("run {r}"))?;
                let (a, step, n): (i64, i64, i64) = (a.parse().map_err(|_| format!("run {r}"))?, step.parse().map_err(|_| format!("run {r}"))?, n.parse().map_err(|_| format!("run {r}"))?);
                for k in 0..n {
                    v.push(a + k * step);
                }
            }
        }
    }
    v.sort_unstable();
    Ok(v)
}

impl Ladder {
    /// Parses `LISTED-CONTRACTS-IDX.tsv`.
    ///
    /// # Errors
    /// A malformed row.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rows = BTreeMap::new();
        for (i, l) in text.lines().enumerate() {
            if l.is_empty() || l.starts_with('#') || l.starts_with("file_date") {
                continue;
            }
            let c: Vec<&str> = l.split('\t').collect();
            let g = |k: usize| c.get(k).copied().unwrap_or("");
            let file = day::parse(g(0)).ok_or_else(|| format!("ladder line {}", i + 1))?;
            let expiry = day::parse(g(3)).ok_or_else(|| format!("ladder line {}", i + 1))?;
            let row = LadderRow {
                udiff: g(1) == "udiff",
                ce: runs(g(4))?,
                pe: runs(g(5))?,
                lot: g(6).parse().ok(),
                cands: if g(9) == "-" || g(9).is_empty() { Vec::new() } else { g(9).split(',').filter_map(|x| x.parse().ok()).collect() },
            };
            rows.insert((g(2).to_string(), file, expiry), row);
        }
        Ok(Self { rows })
    }

    /// The row of (index, file, expiry).
    #[must_use]
    pub fn get(&self, index: &str, file: Day, expiry: Day) -> Option<&LadderRow> {
        self.rows.get(&(index.to_string(), file, expiry))
    }

    /// Whether the file dated `file` lists `expiry` with strikes for `index` (F10).
    #[must_use]
    pub fn has(&self, index: &str, file: Day, expiry: Day) -> bool {
        self.get(index, file, expiry).is_some_and(|r| !r.ce.is_empty() && !r.pe.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_expand() {
        assert_eq!(runs("740000+10000*3,915000").unwrap_or_default(), vec![740_000, 750_000, 760_000, 915_000]);
        let t = "file_date\tform\tindex\texpiry\tce_strikes\tpe_strikes\tlots\toi_gcd\toi_rows\tfile_cands\n2018-09-03\told\tNIFTY\t2018-09-27\t740000+10000*2\t740000\t-\t75\t133\t25,75\n";
        let l = Ladder::parse(t).unwrap_or_default();
        let r = l.get("NIFTY", day::parse("2018-09-03").unwrap_or(0), day::parse("2018-09-27").unwrap_or(0));
        assert_eq!(r.map(|r| r.cands.clone()), Some(vec![25, 75]));
        assert_eq!(r.and_then(|r| r.lot), None);
    }
}

/// C27: the bhavcopy file a session's ladder is read from: the latest file dated STRICTLY BEFORE the session (never the
/// session's own file: K11). Mutant `m_c27_listed` reads the file dated on the session (look-ahead); test
/// `ix_c27_listed` must turn red under it.
#[must_use]
pub fn file_before(files: &[Day], d: Day) -> Option<Day> {
    let k = if cfg!(feature = "m_c27_listed") { files.partition_point(|&f| f <= d) } else { files.partition_point(|&f| f < d) };
    k.checked_sub(1).and_then(|i| files.get(i)).copied()
}

#[cfg(test)]
mod c27 {
    use super::*;

    /// v2 F1 `ix_c27_listed` (red first): on 2023-06-28 the ladder comes from the file of 2023-06-27, which lists 2023-06-29
    /// (the holiday-shifted series) and not yet 2023-06-28; a contract first listed by the session's own file is never a
    /// candidate that session.
    #[test]
    fn ix_c27_listed() {
        let d = |s: &str| day::parse(s).unwrap_or(0);
        let files = [d("2023-06-26"), d("2023-06-27"), d("2023-06-28"), d("2023-06-30")];
        assert_eq!(file_before(&files, d("2023-06-28")), Some(d("2023-06-27")));
        assert_eq!(file_before(&files, d("2023-06-30")), Some(d("2023-06-28")));
        assert_eq!(file_before(&files, d("2023-06-26")), None);
    }
}
