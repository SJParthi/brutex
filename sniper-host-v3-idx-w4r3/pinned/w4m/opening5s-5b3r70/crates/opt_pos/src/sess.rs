//! The dated session facts as DATA (check-build-idx-S1-S6 D5; plan F13), for the positional engine [`crate::pos`]: the F&O end of
//! every session from S1's `FNO-END-ROWS.tsv` (15:30:00; 15:40:00 from 2026-08-03; a day with NO row is REFUSED BY NAME, never
//! extended), the index close (entries strictly before it) and the per-segment halt windows (derivatives / index) from the
//! sha-pinned `HALT-WINDOWS-IDX-v1.tsv`. The constant functions of [`crate::secs`] stay only as the idx1s comparison rows of
//! G-EQ2 (D5: "keep the idx1s values as the G-EQ2 comparison rows"); a unit test pins the data file equal to them.
//!
//! W4 REPAIR (check-wave4-S9 D-W4-2; plan v3.1 s4.4 / Q39; S1 `MUHURAT-LIST.tsv`: "positions are held through"): the Muhurat sessions
//! are DATA too ([`Sessions::with_muhurat`]). A carried exit rest goes to [`Sessions::next_carry`], the next session that is NOT a
//! Muhurat session; the other special sessions (the Saturday DR / budget sessions 2024-03-02, 2024-05-18, ...) stay carry targets:
//! exits are allowed where the option prints (lane B Q16).

use std::collections::{BTreeMap, BTreeSet};

use idx_cal::day::{self, Day};
use idx_cal::storeio::at_sod;

fn hms(s: &str) -> Option<i64> {
    let p: Vec<i64> = s.split(':').map(|x| x.trim().parse().ok()).collect::<Option<Vec<i64>>>()?;
    match p.as_slice() {
        [h, m, s] if (0..24).contains(h) && (0..60).contains(m) && (0..60).contains(s) => Some(h * 3600 + m * 60 + s),
        _ => None,
    }
}

/// The session facts of the run.
#[derive(Clone, Debug, Default)]
pub struct Sessions {
    /// The sessions with an F&O-end row, ascending.
    pub days: Vec<Day>,
    fno_end: BTreeMap<Day, i64>,
    index_close: BTreeMap<Day, i64>,
    halt_der: BTreeMap<Day, (i64, i64)>,
    halt_idx: BTreeMap<Day, (i64, i64)>,
    /// W4 REPAIR (D-W4-2): the Muhurat sessions of S1's `MUHURAT-LIST.tsv` (held through: never a carry target).
    muhurat: BTreeSet<Day>,
}

impl Sessions {
    /// Parses `FNO-END-ROWS.tsv` (`day fno_end index_close source`) and `HALT-WINDOWS-IDX-v1.tsv`
    /// (`day segment from resume source`, segment `derivatives` | `index`).
    ///
    /// # Errors
    /// A malformed row.
    pub fn parse(fno_rows: &str, halts: &str) -> Result<Self, String> {
        let mut s = Self::default();
        for (i, l) in fno_rows.lines().enumerate() {
            if l.starts_with('#') || l.starts_with("day\t") || l.trim().is_empty() {
                continue;
            }
            let c: Vec<&str> = l.split('\t').collect();
            let d = c.first().and_then(|x| day::parse(x)).ok_or_else(|| format!("FNO-END-ROWS line {}: day", i + 1))?;
            let e = c.get(1).and_then(|x| hms(x)).ok_or_else(|| format!("FNO-END-ROWS line {}: fno_end", i + 1))?;
            let x = c.get(2).and_then(|x| hms(x)).ok_or_else(|| format!("FNO-END-ROWS line {}: index_close", i + 1))?;
            s.fno_end.insert(d, e);
            s.index_close.insert(d, x);
        }
        for (i, l) in halts.lines().enumerate() {
            if l.starts_with('#') || l.starts_with("day\t") || l.trim().is_empty() {
                continue;
            }
            let c: Vec<&str> = l.split('\t').collect();
            let d = c.first().and_then(|x| day::parse(x)).ok_or_else(|| format!("HALT-WINDOWS line {}: day", i + 1))?;
            let a = c.get(2).and_then(|x| hms(x)).ok_or_else(|| format!("HALT-WINDOWS line {}: from", i + 1))?;
            let z = c.get(3).and_then(|x| hms(x)).ok_or_else(|| format!("HALT-WINDOWS line {}: resume", i + 1))?;
            match c.get(1).copied() {
                Some("derivatives") => s.halt_der.insert(d, (a, z)),
                Some("index") => s.halt_idx.insert(d, (a, z)),
                x => return Err(format!("HALT-WINDOWS line {}: segment {x:?}", i + 1)),
            };
        }
        s.days = s.fno_end.keys().copied().collect();
        Ok(s)
    }

    /// W4 REPAIR (D-W4-2): the same facts with S1's `MUHURAT-LIST.tsv` (`day weekday segments source`; `#` lines and the header
    /// skipped) loaded as data. Every listed day must be a session of the F&O-end rows (fail closed: a list that names a day the
    /// session table does not hold is refused by name).
    ///
    /// # Errors
    /// A malformed row, or a Muhurat day with no F&O-end row.
    pub fn with_muhurat(mut self, text: &str) -> Result<Self, String> {
        for (i, l) in text.lines().enumerate() {
            if l.starts_with('#') || l.starts_with("day\t") || l.trim().is_empty() {
                continue;
            }
            let d = l.split('\t').next().and_then(day::parse).ok_or_else(|| format!("MUHURAT-LIST line {}: day", i + 1))?;
            if !self.fno_end.contains_key(&d) {
                return Err(format!("MUHURAT-LIST line {}: {} is not a session of the F&O-end rows (refused by name)", i + 1, day::fmt(d)));
            }
            self.muhurat.insert(d);
        }
        Ok(self)
    }

    /// Whether `d` is a Muhurat session (S1 `MUHURAT-LIST.tsv`).
    #[must_use]
    pub fn is_muhurat(&self, d: Day) -> bool {
        self.muhurat.contains(&d)
    }

    /// The Muhurat sessions loaded, ascending.
    #[must_use]
    pub fn muhurat_days(&self) -> Vec<Day> {
        self.muhurat.iter().copied().collect()
    }

    /// W4 REPAIR (D-W4-2; plan s4.4 / Q39 "a position is held through a Muhurat session, and its exit waits for the next regular
    /// session's candles"): the session a carried exit rest fills from = the first session after `d` that is NOT a Muhurat session.
    /// The other special sessions stay carry targets (exits allowed where the option prints, lane B Q16).
    #[must_use]
    pub fn next_carry(&self, d: Day) -> Option<Day> {
        let i = self.days.partition_point(|&x| x <= d);
        self.days.get(i..).unwrap_or(&[]).iter().copied().find(|x| !self.muhurat.contains(x))
    }

    /// The dated F&O end of `d` (exclusive, UTC micros). F13: a day with no row is refused by name.
    ///
    /// # Errors
    /// `no_fno_end_row <day>`.
    pub fn fno_end(&self, d: Day) -> Result<i64, String> {
        self.fno_end.get(&d).map(|&s| at_sod(d, s)).ok_or_else(|| format!("no_fno_end_row {} (F13: refused by name)", day::fmt(d)))
    }

    /// The index close of `d` (UTC micros): an entry is decided strictly before it.
    ///
    /// # Errors
    /// `no_fno_end_row <day>`.
    pub fn index_close(&self, d: Day) -> Result<i64, String> {
        self.index_close.get(&d).map(|&s| at_sod(d, s)).ok_or_else(|| format!("no_fno_end_row {} (F13: refused by name)", day::fmt(d)))
    }

    /// The derivatives halt window of `d` [from, resume), UTC micros.
    #[must_use]
    pub fn halt_der(&self, d: Day) -> Option<(i64, i64)> {
        self.halt_der.get(&d).map(|&(a, z)| (at_sod(d, a), at_sod(d, z)))
    }

    /// The index halt window of `d` [from, resume), UTC micros.
    #[must_use]
    pub fn halt_idx(&self, d: Day) -> Option<(i64, i64)> {
        self.halt_idx.get(&d).map(|&(a, z)| (at_sod(d, a), at_sod(d, z)))
    }

    /// The session after `d`.
    #[must_use]
    pub fn next(&self, d: Day) -> Option<Day> {
        let i = self.days.partition_point(|&x| x <= d);
        self.days.get(i).copied()
    }

    /// Whether `d` is a session.
    #[must_use]
    pub fn is_session(&self, d: Day) -> bool {
        self.fno_end.contains_key(&d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HALTS: &str = "day\tsegment\tfrom\tresume\tsource\n2020-03-13\tderivatives\t09:21:45\t10:21:00\tidx1s\n2020-03-13\tindex\t09:21:10\t10:21:01\tidx1s\n2020-03-23\tderivatives\t09:58:01\t10:57:00\tidx1s\n2020-03-23\tindex\t09:58:09\t10:57:01\tidx1s\n2021-02-24\tderivatives\t11:40:00\t15:45:00\tSO\n2021-02-24\tindex\t11:40:00\t15:45:00\tSO\n";

    /// D5 / F13: the F&O end comes from the rows (15:40 from 2026-08-03); a day without a row is refused by name; the halt file
    /// equals the idx1s comparison constants of `secs` on every halt day.
    #[test]
    fn d5_session_facts_are_data() {
        let rows = "day\tfno_end\tindex_close\tsource\n2026-07-31\t15:30:00\t15:30:00\tregular\n2026-08-04\t15:40:00\t15:30:00\tregular\n2020-03-13\t15:30:00\t15:30:00\tregular\n2020-03-23\t15:30:00\t15:30:00\tregular\n2021-02-24\t17:00:00\t17:00:00\tspecial\n";
        let s = Sessions::parse(rows, HALTS).unwrap_or_default();
        let d = |x: &str| day::parse(x).unwrap_or(0);
        assert_eq!(s.fno_end(d("2026-08-04")), Ok(at_sod(d("2026-08-04"), 15 * 3600 + 40 * 60)));
        assert_eq!(s.fno_end(d("2026-07-31")), Ok(at_sod(d("2026-07-31"), 15 * 3600 + 30 * 60)));
        assert!(s.fno_end(d("2026-11-02")).is_err_and(|e| e.starts_with("no_fno_end_row")), "F13: no row, refused by name");
        for x in ["2020-03-13", "2020-03-23", "2021-02-24"] {
            assert_eq!(s.halt_der(d(x)), crate::secs::halt_der(d(x)), "{x} derivatives");
            assert_eq!(s.halt_idx(d(x)), crate::secs::halt_idx(d(x)), "{x} index");
        }
        assert_eq!(s.next(d("2026-07-31")), Some(d("2026-08-04")));
    }

    /// W4 REPAIR (D-W4-2): S1's MUHURAT-LIST rows are data; a carry from 2025-10-20 skips the 2025-10-21 Muhurat session (and the
    /// 10-22 holiday) and lands on 2025-10-23; the special Saturday DR session 2024-03-02 is NOT a Muhurat session and stays a carry
    /// target; a Muhurat day the session table does not hold is refused by name; `next` itself is unchanged (rolls, the S8b reads).
    #[test]
    fn w4r_muhurat_rows_are_data_and_carries_skip_them() {
        let rows = "day\tfno_end\tindex_close\tsource\n2024-03-01\t15:30:00\t15:30:00\tregular\n2024-03-02\t12:30:00\t15:30:00\tspecial session segment\n2024-03-04\t15:30:00\t15:30:00\tregular\n2025-10-20\t15:30:00\t15:30:00\tregular\n2025-10-21\t14:45:00\t15:30:00\tspecial session segment\n2025-10-23\t15:30:00\t15:30:00\tregular\n";
        let muh = "# Muhurat sessions (Q39)\nday\tweekday\tsegments\tsource\n2025-10-21\tTue\t13:45:00-14:45:00\tSESSIONS-SPECIAL-v1.tsv\n";
        let d = |x: &str| day::parse(x).unwrap_or(0);
        let s = Sessions::parse(rows, HALTS).and_then(|s| s.with_muhurat(muh)).unwrap_or_default();
        assert!(s.is_muhurat(d("2025-10-21")) && !s.is_muhurat(d("2024-03-02")));
        assert_eq!(s.muhurat_days(), vec![d("2025-10-21")]);
        assert_eq!(s.next(d("2025-10-20")), Some(d("2025-10-21")), "next: the session table as read");
        assert_eq!(s.next_carry(d("2025-10-20")), Some(d("2025-10-23")), "a carry is held through the Muhurat session");
        assert_eq!(s.next_carry(d("2024-03-01")), Some(d("2024-03-02")), "the DR Saturday stays a carry target (lane B Q16)");
        assert_eq!(s.next_carry(d("2025-10-23")), None, "no session after the data end");
        let bad = Sessions::parse(rows, HALTS).and_then(|s| s.with_muhurat("2025-10-22\tWed\t-\tx\n"));
        assert!(bad.is_err_and(|e| e.contains("not a session")), "a listed day outside the session table is refused by name");
        assert!(Sessions::parse(rows, HALTS).and_then(|s| s.with_muhurat("21-10-2025\tTue\n")).is_err(), "a malformed row is refused");
    }
}
