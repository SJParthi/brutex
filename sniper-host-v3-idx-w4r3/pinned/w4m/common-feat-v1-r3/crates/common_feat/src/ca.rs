//! Corporate actions (K13; REQ72-PLAN A.7 DP-CA, amendments A1 / A2).
//!
//! - Every event is a data row: its ex-date session `e`, its KIND, its price factor f and volume factor g (exact
//!   rationals), its proof status, and Kite's MEASURED re-basing of the Zerodha series at that event.
//! - **Causal read (RF11):** a reading at decision session t applies only events with `u < e <= t`; a factor dated
//!   after the decision session is never applied (red test `ca_future_factor_never_applied`, mutant m20).
//! - **A back-adjusted series is never a price source (K10, K13):** the Zerodha history Kite re-based at a LATER event
//!   is undone first ([`CaBook::kite_undo`], [`undo_factor`]), so no reading sees a future event.
//! - **X3:** CONT streams reset at a factor ex-date ([`CaBook::cont_reset`]) and are cold until their `need`.
//! - An event whose f is not PROVEN gives no factor: a reading whose window spans it is `unknown`, and the stock-level
//!   refusal span of K13 (`ex_date_unknown`: from e through `e + n_max - 1`) is the host's, from [`CaBook::unknown_events`].

use crate::num::{Ratio, mul_snap};

/// The kind of a corporate action (K13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Bonus a:b.
    Bonus,
    /// Split.
    Split,
    /// Consolidation.
    Consolidation,
    /// Rights (handled as the company-ratio kinds until DP-CA measures Kite on its own rights events).
    Rights,
    /// Demerger.
    Demerger,
    /// Scheme of arrangement.
    Scheme,
    /// Capital reduction.
    CapitalReduction,
    /// Dividend (no factor; O2).
    Dividend,
}

/// The proof status of an event's price factor (A.7 pass rule).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// f PROVEN: readings across e read earlier prices through f.
    Proven,
    /// `ex_date_unknown`: f has one source or none; every window that spans e is `unknown` (and K13's stock span).
    ExDateUnknown,
}

/// One event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    /// The ex-date session (IST day number).
    pub day: i32,
    /// Kind.
    pub kind: Kind,
    /// Price factor f = num / den (1/1 when none applies).
    pub f: (i128, i128),
    /// Volume factor g = num / den.
    pub g: (i128, i128),
    /// Proof status of f.
    pub status: Status,
    /// Kite's MEASURED step at e: the Zerodha prices BEFORE e equal the raw prices times `num / den` (rounded to the
    /// vendor's tick); `None` = "Kite: not re-based".
    pub kite: Option<(i128, i128)>,
    /// Kite's MEASURED volume step at e: the Zerodha volumes BEFORE e equal the raw volumes times `num / den` (rounded);
    /// `None` = not re-based.
    pub kite_vol: Option<(i128, i128)>,
}

/// The dated corporate actions of one series, sorted by day.
#[derive(Clone, Debug, Default)]
pub struct CaBook {
    events: Vec<Event>,
}

impl CaBook {
    /// A book from rows (sorted here).
    #[must_use]
    pub fn new(mut events: Vec<Event>) -> Self {
        events.sort_by_key(|e| e.day);
        Self { events }
    }

    /// The events.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// The cumulative price factor F(u .. t] of the PROVEN factor events with `u < e <= t`, or `None` when an
    /// event in that span is not PROVEN (the reading is `unknown`). Dividends carry no factor (O2's raw default).
    /// O(events of the series): the named bounded exception (a few a member).
    #[must_use]
    pub fn factor(&self, u: i32, t: i32) -> Option<Ratio> {
        let mut num: i128 = 1;
        let mut den: i128 = 1;
        for e in &self.events {
            let inside = if cfg!(feature = "m_ca_future") { e.day > u } else { e.day > u && e.day <= t };
            if !inside || e.kind == Kind::Dividend {
                continue;
            }
            if e.status != Status::Proven {
                return None;
            }
            num *= e.f.0;
            den *= e.f.1;
        }
        Ratio::new(num, den)
    }

    /// The cumulative volume factor G(u .. t] (X4), with the same rule as [`CaBook::factor`].
    #[must_use]
    pub fn vfactor(&self, u: i32, t: i32) -> Option<Ratio> {
        let mut num: i128 = 1;
        let mut den: i128 = 1;
        for e in &self.events {
            if !(e.day > u && e.day <= t) || e.kind == Kind::Dividend {
                continue;
            }
            if e.status != Status::Proven {
                return None;
            }
            num *= e.g.0;
            den *= e.g.1;
        }
        Ratio::new(num, den)
    }

    /// A factor event is dated `day`: CONT streams reset there (X3, `ex_date_reset_cold`).
    #[must_use]
    pub fn cont_reset(&self, day: i32) -> bool {
        self.events.iter().any(|e| e.day == day && e.kind != Kind::Dividend)
    }

    /// Kite's cumulative re-basing of a Zerodha price stamped on session `day`: the product of the steps of every event
    /// AFTER `day` (Kite re-bases history at later ex-dates). `None` when no later event re-based it.
    #[must_use]
    pub fn kite_undo(&self, day: i32) -> Option<(i128, i128)> {
        let mut num: i128 = 1;
        let mut den: i128 = 1;
        let mut any = false;
        for e in &self.events {
            if e.day > day
                && let Some((a, b)) = e.kite
            {
                num *= a;
                den *= b;
                any = true;
            }
        }
        any.then_some((num, den))
    }

    /// Kite's cumulative VOLUME re-basing of a Zerodha volume stamped on session `day` (the steps of every later event).
    #[must_use]
    pub fn kite_vol_undo(&self, day: i32) -> Option<(i128, i128)> {
        let mut num: i128 = 1;
        let mut den: i128 = 1;
        let mut any = false;
        for e in &self.events {
            if e.day > day
                && let Some((a, b)) = e.kite_vol
            {
                num *= a;
                den *= b;
                any = true;
            }
        }
        any.then_some((num, den))
    }

    /// The events whose f is not PROVEN (the host's K13 `ex_date_unknown` spans).
    pub fn unknown_events(&self) -> impl Iterator<Item = &Event> {
        self.events.iter().filter(|e| e.status != Status::Proven && e.kind != Kind::Dividend)
    }
}

/// Undoes a back-adjustment: a vendor price `z = round(raw x num / den)` maps back to the raw grid of `tick` paisa,
/// `raw = nearest multiple of tick to z x den / num` (half up). The one rounding of the undo; its exactness against
/// the raw feed is measured by the host's reconciliation (mismatches beyond one tick counted).
#[must_use]
pub fn undo_factor(z: i64, kite: (i128, i128), tick: i64) -> i64 {
    mul_snap(z, kite.1, kite.0, tick)
}

/// Undoes a volume back-adjustment: `v = round(raw x num / den)` maps back to `raw = round(v x den / num)` (half up).
#[must_use]
pub fn undo_volume(v: u64, kite_vol: (i128, i128)) -> u64 {
    let x = i64::try_from(v).unwrap_or(i64::MAX);
    u64::try_from(mul_snap(x, kite_vol.1, kite_vol.0, 1)).unwrap_or(v)
}

/// An earlier price `x` read at a later decision through the factor `f` (X2): `floor(x f)` to the paisa, the one
/// rounding named (REQ72-PLAN K13 X2: "the product floored to paisa once").
#[must_use]
pub fn through(x: i64, f: Ratio) -> i64 {
    i64::try_from((i128::from(x) * f.num).div_euclid(f.den)).unwrap_or(x)
}
