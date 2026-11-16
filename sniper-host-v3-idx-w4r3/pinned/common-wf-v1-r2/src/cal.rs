//! The dated session table (P2): the host's rows (date, kind, walked), fail closed, as one dense day table with a prefix count of
//! walked sessions, so "is this session walked" and "how many walked sessions in [a, b]" are O(1).

use crate::{Day, WfErr};

/// A session's kind (P2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayKind {
    /// A regular session.
    Regular,
    /// A special session (not walked unless a ruling is quoted, MAIN R11).
    Special,
    /// A regular session with a halt (it books like any walked session, OB31).
    Halted,
}

/// One calendar row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalRow {
    /// The IST date.
    pub day: Day,
    /// Its kind.
    pub kind: DayKind,
    /// Is it walked (the host's rule)?
    pub walked: bool,
    /// A ruling allows a walked SPECIAL session.
    pub ruling: bool,
}

/// The dense day table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calendar {
    first: Day,
    walked: Vec<bool>,
    prefix: Vec<u32>,
    hash: u64,
}

impl Calendar {
    /// Builds the table.
    ///
    /// # Errors
    /// `BadRow(C, n, 1)` unsorted or duplicate; `BadRow(C, n, 2)` a walked SPECIAL without a ruling.
    pub fn from_rows(rows: &[CalRow]) -> Result<Self, WfErr> {
        let mut prev: Option<Day> = None;
        let mut hash = 0x5ca1_ab1e_u64;
        for (i, r) in rows.iter().enumerate() {
            let n = u32::try_from(i + 1).unwrap_or(u32::MAX);
            if prev.is_some_and(|p| r.day <= p) {
                return Err(WfErr::BadRow { file: b'C', row: n, rule: 1 });
            }
            if r.kind == DayKind::Special && r.walked && !r.ruling {
                return Err(WfErr::BadRow { file: b'C', row: n, rule: 2 });
            }
            prev = Some(r.day);
            hash = crate::mix(hash ^ u64::from(r.day.unsigned_abs()) ^ (u64::from(r.walked) << 40) ^ ((r.kind as u64) << 44));
        }
        let (Some(a), Some(b)) = (rows.first(), rows.last()) else {
            return Ok(Self { first: 0, walked: Vec::new(), prefix: vec![0], hash });
        };
        let len = usize::try_from(b.day - a.day + 1).map_err(|_| WfErr::BadDay)?;
        let mut walked = vec![false; len];
        for r in rows {
            if let Some(w) = usize::try_from(r.day - a.day).ok().and_then(|i| walked.get_mut(i)) {
                *w = r.walked;
            }
        }
        let mut prefix = Vec::with_capacity(len + 1);
        let mut c = 0u32;
        prefix.push(0);
        for w in &walked {
            c += u32::from(*w);
            prefix.push(c);
        }
        Ok(Self { first: a.day, walked, prefix, hash })
    }
    fn idx(&self, d: Day) -> Option<usize> {
        usize::try_from(d - self.first).ok().filter(|&i| i < self.walked.len())
    }
    /// Is `d` a walked session (false outside the table)? O(1).
    #[must_use]
    pub fn walked(&self, d: Day) -> bool {
        self.idx(d).and_then(|i| self.walked.get(i)).copied().unwrap_or(false)
    }
    /// Walked sessions in `[a, b]` (clipped to the table). O(1).
    #[must_use]
    pub fn walked_in(&self, a: Day, b: Day) -> u32 {
        if b < a || self.walked.is_empty() {
            return 0;
        }
        let last = self.first + Day::try_from(self.walked.len()).unwrap_or(Day::MAX) - 1;
        let (a, b) = (a.max(self.first), b.min(last));
        if b < a {
            return 0;
        }
        let (Some(i), Some(j)) = (self.idx(a), self.idx(b)) else { return 0 };
        let (Some(x), Some(y)) = (self.prefix.get(j + 1), self.prefix.get(i)) else { return 0 };
        x - y
    }
    /// The table's identity hash (its sha16 stands in identity lines; this hash in `Facts` / `Lists`).
    #[must_use]
    pub const fn hash(&self) -> u64 {
        self.hash
    }
}
