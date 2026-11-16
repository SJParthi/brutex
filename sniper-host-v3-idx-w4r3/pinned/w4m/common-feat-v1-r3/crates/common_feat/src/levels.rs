//! Levels from an earlier period's high, low and close: traditional pivots and CPR (LV-PIV, LV-CPR; weekly and monthly
//! as LV-PIVWM), held exactly in sixths of a paisa (REQ72-DESIGN LV-PIV / LV-CPR: "every level with its denominator
//! kept"), and the zone of a close against them.
//!
//! Traditional: P = (H + L + C) / 3; R1 = 2P - L; S1 = 2P - H; R2 = P + (H - L); S2 = P - (H - L);
//! CPR: BC = (H + L) / 2, TC = 2P - BC; width = |TC - BC| / P.
//!
//! The H / L / C come from the host through K13 X2's factors (only events with u < e <= t, [`crate::ca`]).

use crate::S;
use crate::num::Ratio;

/// An earlier period's official high, low and close (paisa), already read through the factors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hlc {
    /// High.
    pub h: i64,
    /// Low.
    pub l: i64,
    /// Close.
    pub c: i64,
}

/// Pivots and CPR in sixths of a paisa (`x6`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pivots {
    /// 6 P.
    pub p6: i128,
    /// 6 R1.
    pub r1_6: i128,
    /// 6 R2.
    pub r2_6: i128,
    /// 6 S1.
    pub s1_6: i128,
    /// 6 S2.
    pub s2_6: i128,
    /// 6 BC.
    pub bc6: i128,
    /// 6 TC.
    pub tc6: i128,
}

/// Where a close lies against the pivots (SNIPER-ADANIENT-PLAN L-PIV zones, long-side names; the short side mirrors).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PivZone {
    /// Above R2.
    AboveR2,
    /// Between R1 (inclusive) and R2.
    R1R2,
    /// Between P (inclusive) and R1.
    PR1,
    /// Between S1 (inclusive) and P.
    S1P,
    /// Between S2 (inclusive) and S1.
    S2S1,
    /// Below S2.
    BelowS2,
}

/// Where a close lies against the CPR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CprZone {
    /// Above the top (TC, or the higher of TC and BC).
    Above,
    /// Inside the CPR (edges included).
    Inside,
    /// Below the bottom.
    Below,
}

impl Pivots {
    /// The levels of one period.
    #[must_use]
    pub fn of(x: Hlc) -> Self {
        let (h, l, c) = (i128::from(x.h), i128::from(x.l), i128::from(x.c));
        let p6 = 2 * (h + l + c);
        let bc6 = 3 * (h + l);
        Self {
            p6,
            r1_6: 2 * p6 - 6 * l,
            r2_6: p6 + 6 * (h - l),
            s1_6: 2 * p6 - 6 * h,
            s2_6: p6 - 6 * (h - l),
            bc6,
            tc6: 2 * p6 - bc6,
        }
    }

    /// A level held x6 shown at `S` (paisa x 10^6), floored.
    #[must_use]
    pub const fn at_s(v6: i128) -> i128 {
        (v6 * S).div_euclid(6)
    }

    /// The CPR width as an exact ratio of P in basis points: 10^4 |TC - BC| / P.
    #[must_use]
    pub const fn cpr_width_bp(&self) -> Option<Ratio> {
        Ratio::new(10_000 * (self.tc6 - self.bc6).abs(), self.p6)
    }

    /// The pivot zone of a close (paisa).
    #[must_use]
    pub fn zone(&self, close: i64) -> PivZone {
        let x6 = 6 * i128::from(close);
        if x6 > self.r2_6 {
            PivZone::AboveR2
        } else if x6 >= self.r1_6 {
            PivZone::R1R2
        } else if x6 >= self.p6 {
            PivZone::PR1
        } else if x6 >= self.s1_6 {
            PivZone::S1P
        } else if x6 >= self.s2_6 {
            PivZone::S2S1
        } else {
            PivZone::BelowS2
        }
    }

    /// The CPR zone of a close.
    #[must_use]
    pub fn cpr_zone(&self, close: i64) -> CprZone {
        let x6 = 6 * i128::from(close);
        let top = self.tc6.max(self.bc6);
        let bot = self.tc6.min(self.bc6);
        if x6 > top {
            CprZone::Above
        } else if x6 < bot {
            CprZone::Below
        } else {
            CprZone::Inside
        }
    }

    /// The close's distance from a level (held x6) as an exact ratio in bp of the level: 10^4 (6x - v6) / v6.
    #[must_use]
    pub fn dist_bp(close: i64, v6: i128) -> Option<Ratio> {
        Ratio::new(10_000 * (6 * i128::from(close) - v6), v6)
    }
}

/// The name of a pivot zone.
#[must_use]
pub const fn piv_zone_name(z: PivZone) -> &'static str {
    match z {
        PivZone::AboveR2 => "ABOVE_R2",
        PivZone::R1R2 => "R1_R2",
        PivZone::PR1 => "P_R1",
        PivZone::S1P => "S1_P",
        PivZone::S2S1 => "S2_S1",
        PivZone::BelowS2 => "BELOW_S2",
    }
}

/// The name of a CPR zone.
#[must_use]
pub const fn cpr_zone_name(z: CprZone) -> &'static str {
    match z {
        CprZone::Above => "ABOVE",
        CprZone::Inside => "INSIDE",
        CprZone::Below => "BELOW",
    }
}
