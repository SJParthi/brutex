//! The three fixed rule sets as data (TEST-PLAN section 0 item 8; ADD): one table each,
//! run by one code path. VIDEO-SCREEN is AUTHOR-V2 with the two C6 levels mirrored.

use crate::readings::Kind;

/// One condition of a rule set, with its fixed parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cond {
    /// C1: narrow CPR at this many basis points of the pivot.
    NarrowCpr {
        /// The width cap, basis points (20 = 0.20%).
        bp: u16,
    },
    /// C2: the close beyond the previous session's R1 (long) or S1 (short).
    BeyondR1S1,
    /// C3: the Supertrend 10 / factor is settled and on the side.
    Supertrend {
        /// The factor in thousandths (2,000 or 3,000).
        factor_milli: u32,
    },
    /// C4: the close beyond the Bollinger 20 band at this width.
    Bollinger {
        /// The width z in thousandths (2,000 or 3,000).
        width_milli: u32,
    },
    /// C5: a faithful RSI 14 beyond the levels.
    Rsi {
        /// Long when above, ppm.
        long_ppm: u32,
        /// Short when below, ppm.
        short_ppm: u32,
    },
    /// C6: Williams %R 14 beyond the levels (LEVEL form), not flat.
    WilliamsR {
        /// Long when above, ppm of the 0..-100 scale.
        long_ppm: i32,
        /// Short when below, ppm.
        short_ppm: i32,
    },
    /// C7: CCI 20 beyond the levels, not flat.
    Cci {
        /// Long when above, thousandths of a point.
        long_milli: i32,
        /// Short when below, thousandths of a point.
        short_milli: i32,
    },
    /// C8: the close beyond a warm EMA 200.
    Ema,
}

impl Cond {
    /// The reading the condition gates on, if any (C1 and C2 read the previous session).
    #[must_use]
    pub const fn reading(self) -> Option<Kind> {
        match self {
            Self::NarrowCpr { .. } | Self::BeyondR1S1 => None,
            Self::Supertrend { .. } => Some(Kind::Supertrend),
            Self::Bollinger { .. } => Some(Kind::Bollinger),
            Self::Rsi { .. } => Some(Kind::Rsi),
            Self::WilliamsR { .. } => Some(Kind::WilliamsR),
            Self::Cci { .. } => Some(Kind::Cci),
            Self::Ema => Some(Kind::Ema),
        }
    }

    /// `C1`..`C8`.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::NarrowCpr { .. } => "C1",
            Self::BeyondR1S1 => "C2",
            Self::Supertrend { .. } => "C3",
            Self::Bollinger { .. } => "C4",
            Self::Rsi { .. } => "C5",
            Self::WilliamsR { .. } => "C6",
            Self::Cci { .. } => "C7",
            Self::Ema => "C8",
        }
    }
}

/// One rule set: a name and its conjunction of conditions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RuleSet {
    /// `VIDEO-ORIGINAL`, `AUTHOR-V2` or `VIDEO-SCREEN`.
    pub name: &'static str,
    /// Short form for tables: `VO`, `AV2`, `VS`.
    pub short: &'static str,
    /// The conditions, all on the same closed candle.
    pub conds: &'static [Cond],
}

const RSI: Cond = Cond::Rsi { long_ppm: 700_000, short_ppm: 300_000 };
const CCI: Cond = Cond::Cci { long_milli: 200_000, short_milli: -200_000 };

/// VIDEO-ORIGINAL = C2 + C3(10, 3.000) + C4(20, 2.000) + C5.
pub const VIDEO_ORIGINAL: RuleSet = RuleSet {
    name: "VIDEO-ORIGINAL",
    short: "VO",
    conds: &[Cond::BeyondR1S1, Cond::Supertrend { factor_milli: 3_000 }, Cond::Bollinger { width_milli: 2_000 }, RSI],
};

/// AUTHOR-V2 = C1 (20 bp) + C2 + C3(10, 2.000) + C4(20, 3.000) + C5 + C6 (long above -85,
/// short below -15) + C7 + C8.
pub const AUTHOR_V2: RuleSet = RuleSet {
    name: "AUTHOR-V2",
    short: "AV2",
    conds: &[
        Cond::NarrowCpr { bp: 20 },
        Cond::BeyondR1S1,
        Cond::Supertrend { factor_milli: 2_000 },
        Cond::Bollinger { width_milli: 3_000 },
        RSI,
        Cond::WilliamsR { long_ppm: -850_000, short_ppm: -150_000 },
        CCI,
        Cond::Ema,
    ],
};

/// VIDEO-SCREEN = AUTHOR-V2 with C6 mirrored: long above -15, short below -85 (ADD).
pub const VIDEO_SCREEN: RuleSet = RuleSet {
    name: "VIDEO-SCREEN",
    short: "VS",
    conds: &[
        Cond::NarrowCpr { bp: 20 },
        Cond::BeyondR1S1,
        Cond::Supertrend { factor_milli: 2_000 },
        Cond::Bollinger { width_milli: 3_000 },
        RSI,
        Cond::WilliamsR { long_ppm: -150_000, short_ppm: -850_000 },
        CCI,
        Cond::Ema,
    ],
};

/// The three rule sets, in report order.
pub const RULE_SETS: [RuleSet; 3] = [VIDEO_ORIGINAL, AUTHOR_V2, VIDEO_SCREEN];

/// Why a rule-set table was refused at load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No condition: never "always true".
    Empty,
    /// One condition kind twice.
    Repeated(&'static str),
    /// A known name whose table differs from SPEC section 3.1 (the first differing id).
    NotTheSpec(&'static str),
}

/// Checks a table: non-empty, no condition kind twice, and a known name's table equal to
/// the spec's.
///
/// # Errors
/// The first [`Refusal`].
pub fn check(set: &RuleSet) -> Result<(), Refusal> {
    if set.conds.is_empty() {
        return Err(Refusal::Empty);
    }
    for (i, c) in set.conds.iter().enumerate() {
        if set.conds.iter().skip(i + 1).any(|d| d.id() == c.id()) {
            return Err(Refusal::Repeated(c.id()));
        }
    }
    let spec = RULE_SETS.iter().find(|s| s.name == set.name);
    if let Some(spec) = spec
        && spec.conds != set.conds
    {
        let first = spec.conds.iter().zip(set.conds.iter()).find(|(a, b)| a != b).map_or("length", |(a, _)| a.id());
        return Err(Refusal::NotTheSpec(first));
    }
    Ok(())
}

/// The supertrend factor a rule set reads (C3's), if it has C3.
#[must_use]
pub fn supertrend_factor(set: &RuleSet) -> Option<u32> {
    set.conds.iter().find_map(|c| match c {
        Cond::Supertrend { factor_milli } => Some(*factor_milli),
        _ => None,
    })
}

#[cfg(test)]
#[path = "ruleset_tests.rs"]
mod tests;
