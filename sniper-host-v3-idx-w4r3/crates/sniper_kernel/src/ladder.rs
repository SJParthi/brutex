//! A dial's levels as DATA rows (SNIPER-ADANIENT-PLAN 3.1: "every level is a data row"; the dials file's `values` and `mirror`
//! columns), and the exact bucket of a RAW reading (F24: "value >= 70" decided exactly on the module's state, never on a float).
//!
//! A ladder holds its rungs in PASS order, loosest first: a long's `GE` ladder ascending (RSI 70, 71 .. 100), a short's `LE`
//! ladder descending (RSI 30, 29 .. 0). Passing rung i implies passing every rung before it (nested sets, CODE-SCOUT 4.1 (a)).
//! The bucket of a reading is the number of rungs it passes (0 = it passes only OFF). Value index 0 is OFF; value index j >= 1
//! is rung j; a row is kept by value j exactly when its bucket is at least j.
//!
//! **S8 (the OPEN last rung):** the strictest rung reads "at or beyond the last written level", so a reading 10x the last level
//! still passes it (red test G-OPEN). Mutant `m_s19` closes it ("between" the last two levels).

/// The comparator of a ladder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cmp {
    /// value >= level (a long's high extreme).
    Ge,
    /// value <= level (a short's low extreme, the mirror).
    Le,
}

/// One dial's rungs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ladder {
    /// The dial id (`M-RSI`, ...).
    pub id: String,
    /// The comparator.
    pub cmp: Cmp,
    /// The levels in PASS order (loosest first), integers in the row's unit.
    pub levels: Vec<i64>,
}

/// A ladder refusal, named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LadderErr {
    /// The `values` cell does not parse (`CMP=range:lo:hi:step`).
    BadValues(String),
    /// The `mirror` cell does not parse (`LE a-x`, `LE -x`).
    BadMirror(String),
    /// More rungs than a u16 bucket holds, or none.
    BadSize(String),
}

impl std::fmt::Display for LadderErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadValues(s) => write!(f, "ladder values cell refused: {s}"),
            Self::BadMirror(s) => write!(f, "ladder mirror cell refused: {s}"),
            Self::BadSize(s) => write!(f, "ladder size refused: {s}"),
        }
    }
}

impl std::error::Error for LadderErr {}

fn range(spec: &str) -> Result<Vec<i64>, LadderErr> {
    let bad = || LadderErr::BadValues(spec.to_string());
    let mut it = spec.split(':');
    if it.next() != Some("range") {
        return Err(bad());
    }
    let lo: i64 = it.next().and_then(|x| x.trim().parse().ok()).ok_or_else(bad)?;
    let hi: i64 = it.next().and_then(|x| x.trim().parse().ok()).ok_or_else(bad)?;
    let step: i64 = it.next().and_then(|x| x.trim().parse().ok()).ok_or_else(bad)?;
    if it.next().is_some() || step <= 0 || hi < lo {
        return Err(bad());
    }
    let mut v = Vec::new();
    let mut x = lo;
    while x <= hi {
        v.push(x);
        x = x.checked_add(step).ok_or_else(bad)?;
        if v.len() > usize::from(u16::MAX) - 1 {
            return Err(LadderErr::BadSize(spec.to_string()));
        }
    }
    Ok(v)
}

impl Ladder {
    /// The LONG side's ladder from the dials file's `values` cell, which must hold exactly one `GE=range:lo:hi:step` term
    /// (the momentum core: M-RSI, M-CCI, M-WR).
    ///
    /// # Errors
    /// A cell that is not one `GE=range` term.
    pub fn long_of(id: &str, values: &str) -> Result<Self, LadderErr> {
        let (cmp, spec) = values.split_once('=').ok_or_else(|| LadderErr::BadValues(values.to_string()))?;
        if cmp.trim() != "GE" || spec.contains(';') {
            return Err(LadderErr::BadValues(values.to_string()));
        }
        let levels = range(spec.trim())?;
        if levels.is_empty() {
            return Err(LadderErr::BadSize(values.to_string()));
        }
        Ok(Self { id: id.to_string(), cmp: Cmp::Ge, levels })
    }

    /// The SHORT side's ladder: the long ladder through the `mirror` cell (`LE 100-x (30..0)`, `LE -x (...)`,
    /// `LE -100-x (...)`): level' = a - x, comparator LE, so the pass order stays loosest first.
    ///
    /// # Errors
    /// A mirror cell of another form.
    pub fn short_of(long: &Self, mirror: &str) -> Result<Self, LadderErr> {
        let bad = || LadderErr::BadMirror(mirror.to_string());
        let m = mirror.trim();
        let rest = m.strip_prefix("LE ").ok_or_else(bad)?;
        let expr = rest.split_whitespace().next().ok_or_else(bad)?;
        let a: i64 = if expr == "-x" {
            0
        } else {
            let head = expr.strip_suffix("-x").ok_or_else(bad)?;
            head.parse().map_err(|_| bad())?
        };
        let levels = long.levels.iter().map(|&x| a.checked_sub(x).ok_or_else(bad)).collect::<Result<Vec<_>, _>>()?;
        Ok(Self { id: long.id.clone(), cmp: Cmp::Le, levels })
    }

    /// Values including OFF.
    #[must_use]
    pub fn values(&self) -> usize {
        self.levels.len() + 1
    }

    /// Does the exact reading `num / den` (`den > 0`) pass `level`?
    #[must_use]
    pub fn passes(&self, num: i128, den: i128, level: i64) -> bool {
        let l = i128::from(level).saturating_mul(den);
        match self.cmp {
            Cmp::Ge => num >= l,
            Cmp::Le => num <= l,
        }
    }

    /// The bucket of a RAW reading: the number of rungs it passes (0 = only OFF). `None` (an unready reading, K6) passes no
    /// rung: never true, never false, it is kept only by OFF. O(log rungs), computed once per row at load, never in the sweep.
    #[must_use]
    pub fn bucket(&self, reading: Option<(i128, i128)>) -> u16 {
        let Some((num, den)) = reading else { return 0 };
        if den <= 0 {
            return 0;
        }
        let (mut lo, mut hi) = (0_usize, self.levels.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.levels.get(mid).is_some_and(|&l| self.passes(num, den, l)) {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let b = u16::try_from(lo).unwrap_or(u16::MAX);
        if cfg!(feature = "m_s19") && lo == self.levels.len() && lo >= 2 {
            // MUTANT m-S19: a closed last rung ("between" the last two written levels): beyond the last level plus one step fails it.
            if let (Some(&a), Some(&z)) = (self.levels.get(lo - 2), self.levels.get(lo - 1)) {
                let beyond = z.saturating_add(z.saturating_sub(a));
                if self.passes(num, den, beyond) && beyond != z {
                    return b.saturating_sub(1);
                }
            }
        }
        b
    }

    /// The level text of value index `j` (0 = OFF).
    #[must_use]
    pub fn level_text(&self, j: usize) -> String {
        if j == 0 {
            return "OFF".to_string();
        }
        let op = match self.cmp {
            Cmp::Ge => ">=",
            Cmp::Le => "<=",
        };
        self.levels.get(j - 1).map_or_else(|| "?".to_string(), |l| format!("{op} {l}"))
    }
}
