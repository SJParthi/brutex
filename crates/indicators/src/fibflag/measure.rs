//! The base measurements: width, ranges, bodies, slopes, curvature, squeeze,
//! the day-extreme flag, touches and the base's place in its trade frame
//! (FIB-3; D-6341, D-6342, D-6345).
//!
//! # One code, two views
//!
//! Every measurement is computed from a [`Window`]: one base's aggregates in
//! ONE direction's view. The long view is the prices as stored; the short view
//! is the same aggregates over the NEGATED prices (a short's high is minus a
//! low, its level 0 is minus the day low), so nothing here asks which
//! direction it measures (operator 16 Oct 16:04 IST item 3: only the geometry
//! is mirrored). `crates/indicators/tests/fib_base.rs` proves the short view
//! equals the long view's code on negated prices.
//!
//! # Exact, and no division by zero (FM-F024, FM-F026)
//!
//! A level is the rational `numerator / 1000`; a relation, a level distance
//! and a touch are decided by cross-multiplication in `i128`, never on a
//! rounded price. Every denominator is checked before it divides: `0 / 0` in a
//! ratio is one whole (1000 permille), `x / 0` with `x > 0` is
//! [`Ratio::OverZero`], the histogram's top, counted by the detector. A
//! permille stored as an integer is rounded toward zero and says so.
//!
//! # Totality
//!
//! [`measure`] accepts any [`Window`] and never panics. A window the detector
//! never builds (fewer candles than a base, a far edge outside its ladder, sums
//! past `i128`) gets a slope or curvature of `None` and a retracement outside
//! `0..=1000`; the detector's own windows lie inside their ladder by
//! construction (the day range contains every base).

use vocab::fib_cube::{BP_PER_UNIT, PERMILLE_PER_UNIT, SHAPE_HALVES, ZONE_RUNGS, fib_zone_of};

use super::extremes::Ladder;
use super::types::TradeDir;

/// An exact rational with a positive denominator, unreduced: the formula's own
/// numerator and denominator, so the same window always gives the same pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Exact {
    /// The numerator.
    num: i128,
    /// The denominator, positive.
    den: i128,
}

impl Exact {
    /// `num / den`, or `None` unless `den > 0`.
    #[must_use]
    pub const fn new(num: i128, den: i128) -> Option<Self> {
        if den > 0 {
            Some(Self { num, den })
        } else {
            None
        }
    }

    /// The numerator.
    #[must_use]
    pub const fn num(&self) -> i128 {
        self.num
    }

    /// The denominator, positive.
    #[must_use]
    pub const fn den(&self) -> i128 {
        self.den
    }

    /// The largest integer at or below the value: for a histogram observation
    /// only; every decision compares by [`Self::cmp_int`].
    #[must_use]
    pub const fn floor(&self) -> i128 {
        self.num.div_euclid(self.den)
    }

    /// The value against the integer `value`, by cross-multiplication, or
    /// `None` when `value x den` leaves `i128`.
    #[must_use]
    pub fn cmp_int(&self, value: i128) -> Option<core::cmp::Ordering> {
        value
            .checked_mul(self.den)
            .map(|scaled| self.num.cmp(&scaled))
    }
}

/// A non-negative ratio in a unit (permille or basis points), with FM-F024's
/// limits for a zero denominator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ratio {
    /// The ratio times the unit, rounded toward zero.
    Value(u64),
    /// `x / 0` with `x > 0`: the histogram's top bin, counted.
    OverZero,
}

/// `num / den` times `unit`, for `num >= 0` and `den >= 0`: rounded toward
/// zero; `0 / 0` is one whole (`unit`); `x / 0` is [`Ratio::OverZero`]. A
/// negative input (never built by the detector) is read as its magnitude.
#[must_use]
pub fn ratio(num: i128, den: i128, unit: u64) -> Ratio {
    let (num, den) = (num.unsigned_abs(), den.unsigned_abs());
    if den == 0 {
        return if num == 0 {
            Ratio::Value(unit)
        } else {
            Ratio::OverZero
        };
    }
    num.checked_mul(u128::from(unit))
        .map(|scaled| scaled / den)
        .and_then(|value| u64::try_from(value).ok())
        .map_or(Ratio::OverZero, Ratio::Value)
}

/// Where a base sits against the rung nearest its far edge, in its own
/// direction's view. The declaration order is the `level_relation` row's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LevelRelation {
    /// The whole base lies on the level-0 side of the rung.
    Above,
    /// The whole base lies on the level-1 side of the rung.
    Below,
    /// The rung's price lies inside `[base low, base high]`.
    Spans,
}

impl LevelRelation {
    /// The relation's value index in the `level_relation` row.
    #[must_use]
    pub const fn cube_value(self) -> u8 {
        self as u8
    }
}

/// One candle's high and low in a direction's view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Bar {
    /// The view's high (a short's is minus the low).
    pub high: i64,
    /// The view's low (a short's is minus the high).
    pub low: i64,
}

/// One base's aggregates in one direction's view, over candles `first_index`
/// to `first_index + candles - 1` of the day. Sums are of the view's prices;
/// `x` is the candle's day index.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Window {
    /// How many candles.
    pub candles: u32,
    /// The day index of the first.
    pub first_index: u32,
    /// The base high: the largest view high.
    pub high: i64,
    /// The base low, the far edge: the smallest view low.
    pub low: i64,
    /// The largest view close.
    pub close_high: i64,
    /// The smallest view close.
    pub close_low: i64,
    /// The sum of the view highs.
    pub sum_high: i128,
    /// The sum of the view lows.
    pub sum_low: i128,
    /// The sum of the view closes.
    pub sum_close: i128,
    /// The sum of `x` times the view high.
    pub sum_x_high: i128,
    /// The sum of `x` times the view low.
    pub sum_x_low: i128,
    /// The sum of `x` times the view close.
    pub sum_x_close: i128,
    /// The sum of `x` squared times the view close.
    pub sum_xx_close: i128,
    /// The sum of `x`.
    pub sum_x: i128,
    /// The sum of `x` squared.
    pub sum_xx: i128,
    /// The sum of `x` cubed.
    pub sum_xxx: i128,
    /// The sum of `x` to the fourth.
    pub sum_xxxx: i128,
    /// The sum of the candle ranges (`high - low`, the same in both views).
    pub sum_range: i128,
    /// The sum of the candle bodies (`|close - open|`).
    pub sum_body: i128,
    /// The sum of the ranges of the first `candles / 2` candles.
    pub range_first_half: i128,
    /// The sum of the ranges of the other candles, the last half.
    pub range_last_half: i128,
    /// The sum of the view closes of the first `candles / 3` candles.
    pub close_first_third: i128,
    /// The sum of the view closes of the last `candles / 3` candles.
    pub close_last_third: i128,
    /// Consecutive pairs whose later candle lies inside the earlier.
    pub inside_pairs: u32,
    /// Consecutive pairs whose later view low is higher.
    pub rising_lows: u32,
    /// The deepest view poke below the day's low that closed back above it.
    pub poke: i64,
    /// Whether a candle of the base made the day's level-0 extreme (long: the
    /// high; short: the low) at its own close.
    pub holds_level0: bool,
    /// Whether a candle of the base made the level-1 extreme at its close.
    pub holds_level1: bool,
    /// The first candle.
    pub first: Bar,
    /// The middle candle of a three-candle base.
    pub middle: Option<Bar>,
    /// The last candle.
    pub last: Bar,
}

/// Every raw measurement of one base in one direction's view (D-6341).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BaseMeasures {
    /// The base's width, `high - low`, in paisa.
    pub width: i128,
    /// The width in basis points of the base's level-0 edge (a long's high,
    /// a short's low), rounded toward zero.
    pub width_bp: Ratio,
    /// The sum of the candle ranges: the mean range is `range_sum / candles`.
    pub range_sum: i128,
    /// The sum of the candle bodies: the mean body is `body_sum / candles`.
    pub body_sum: i128,
    /// The least-squares slope of the view's highs, paisa per candle.
    pub slope_highs: Option<Exact>,
    /// The least-squares slope of the view's lows, paisa per candle.
    pub slope_lows: Option<Exact>,
    /// The curvature of the view's closes: the `x squared` coefficient of
    /// their least-squares parabola; `None` below three candles.
    pub curvature_closes: Option<Exact>,
    /// The mean range of the last half over the first, in permille.
    pub squeeze: Ratio,
    /// Which day extremes a candle of the base made at its own close.
    pub holds: Holds,
    /// The far edge's retracement from level 0, in permille of the range.
    pub retrace: Exact,
    /// The `fib_zone` value of the rung nearest the far edge.
    pub fib_zone: u8,
    /// `|far edge - that rung|` in permille of the range.
    pub level_distance: Exact,
    /// Where the base sits against that rung.
    pub relation: LevelRelation,
}

impl BaseMeasures {
    /// The base's width is zero (FM-F024): flagged `zero_width_base`.
    #[must_use]
    pub const fn zero_width(&self) -> bool {
        self.width == 0
    }
}

/// Which day extremes a candle of a base made at its own close (measured, not
/// refused: D-6340).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Holds {
    /// The level-0 extreme (a long's day high, a short's day low).
    pub level0: bool,
    /// The level-1 extreme (a long's day low, a short's day high).
    pub level1: bool,
}

impl Holds {
    /// `base_holds_extreme`: either extreme.
    #[must_use]
    pub const fn extreme(&self) -> bool {
        self.level0 || self.level1
    }
}

/// Whether a view high `high` touches the edge `edge` at a tolerance of
/// `tolerance_bp` basis points of the edge (FM-F017): exactly
/// `|high - edge| x 10,000 <= tolerance_bp x |edge|`.
#[must_use]
pub fn touches(high: i64, edge: i64, tolerance_bp: u64) -> bool {
    let gap = (i128::from(edge) - i128::from(high)).unsigned_abs();
    let scale = u128::from(edge.unsigned_abs());
    gap * u128::from(BP_PER_UNIT) <= u128::from(tolerance_bp) * scale
}

/// `i128` arithmetic that becomes `None` once a step leaves `i128` and stays
/// `None`: one checked path for a whole formula instead of a `?` per step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Wide(Option<i128>);

impl Wide {
    /// A value.
    const fn of(value: i128) -> Self {
        Self(Some(value))
    }

    /// The exact rational `self / den`, or `None` past `i128` or unless
    /// `den > 0`.
    fn over(self, den: Self) -> Option<Exact> {
        self.0
            .zip(den.0)
            .and_then(|(num, den)| Exact::new(num, den))
    }
}

impl core::ops::Add for Wide {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0.zip(other.0).and_then(|(a, b)| a.checked_add(b)))
    }
}

impl core::ops::Sub for Wide {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0.zip(other.0).and_then(|(a, b)| a.checked_sub(b)))
    }
}

impl core::ops::Mul for Wide {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        Self(self.0.zip(other.0).and_then(|(a, b)| a.checked_mul(b)))
    }
}

/// The least-squares slope of `y` over `x`, from the window's sums, or `None`
/// below two candles or past `i128`. `y` is shifted by `shift` first, which
/// leaves the slope exactly unchanged and keeps the products small.
fn slope(w: &Window, total: i128, moment: i128, shift: i64) -> Option<Exact> {
    let (n, x, xx) = (
        Wide::of(i128::from(w.candles)),
        Wide::of(w.sum_x),
        Wide::of(w.sum_xx),
    );
    let shift = Wide::of(i128::from(shift));
    let y = Wide::of(total) - n * shift;
    let xy = Wide::of(moment) - x * shift;
    (n * xy - x * y).over(n * xx - x * x)
}

/// `a x b - c x d`.
fn cross(a: Wide, b: Wide, c: Wide, d: Wide) -> Wide {
    a * b - c * d
}

/// The `x squared` coefficient of the least-squares parabola through the
/// view's closes (Cramer's rule on the normal equations), or `None` below
/// three candles or past `i128`. The closes are shifted by the base low,
/// which leaves the coefficient exactly unchanged.
fn curvature(w: &Window) -> Option<Exact> {
    let n = Wide::of(i128::from(w.candles));
    let shift = Wide::of(i128::from(w.low));
    let (s1, s2, s3, s4) = (
        Wide::of(w.sum_x),
        Wide::of(w.sum_xx),
        Wide::of(w.sum_xxx),
        Wide::of(w.sum_xxxx),
    );
    let t0 = Wide::of(w.sum_close) - n * shift;
    let t1 = Wide::of(w.sum_x_close) - s1 * shift;
    let t2 = Wide::of(w.sum_xx_close) - s2 * shift;
    // det [[n, s1, s2], [s1, s2, s3], [s2, s3, s4]]
    let den = n * cross(s2, s4, s3, s3) - s1 * cross(s1, s4, s2, s3) + s2 * cross(s1, s3, s2, s2);
    // the same with the third column replaced by (t0, t1, t2)
    let num = n * cross(s2, t2, s3, t1) - s1 * cross(s1, t2, t1, s2) + t0 * cross(s1, s3, s2, s2);
    num.over(den)
}

/// Every raw measurement of `window` in `direction`'s view against `ladder`.
#[must_use]
pub fn measure(window: &Window, ladder: &Ladder, direction: TradeDir) -> BaseMeasures {
    let unit = i128::from(PERMILLE_PER_UNIT);
    let width = i128::from(window.high) - i128::from(window.low);
    let n = i128::from(window.candles);
    let first_half = n / i128::from(SHAPE_HALVES);
    let range = i128::from(ladder.range());
    let level0 = i128::from(ladder.level0_view(direction));
    let part = level0 - i128::from(window.low);
    // The nearest of the zone rungs (fib_zone semantics). A part outside
    // 0..=R is nearest rung 0 or 1000 exactly, so it is clamped first; the
    // detector never builds one.
    let clamped = i64::try_from(part.clamp(0, range)).unwrap_or(0);
    let fib_zone = fib_zone_of(clamped, ladder.range()).unwrap_or(0);
    let rung = ZONE_RUNGS
        .iter()
        .zip(0_u8..)
        .find(|(_, zone)| *zone == fib_zone)
        .map_or(0, |(rung, _)| *rung);
    let rung_view = ladder.rung_view(direction, rung);
    let edge_low = unit * i128::from(window.low);
    let edge_high = unit * i128::from(window.high);
    let relation = if edge_low > rung_view {
        LevelRelation::Above
    } else if edge_high < rung_view {
        LevelRelation::Below
    } else {
        LevelRelation::Spans
    };
    // The ladder guarantees R > 0, so each `Exact::new` below is `Some`.
    let over_range = |num: i128| Exact::new(num, range).unwrap_or(Exact { num, den: 1 });
    BaseMeasures {
        width,
        width_bp: ratio(width, i128::from(window.high.unsigned_abs()), BP_PER_UNIT),
        range_sum: window.sum_range,
        body_sum: window.sum_body,
        slope_highs: slope(window, window.sum_high, window.sum_x_high, window.low),
        slope_lows: slope(window, window.sum_low, window.sum_x_low, window.low),
        curvature_closes: curvature(window),
        squeeze: ratio(
            window.range_last_half * first_half,
            window.range_first_half * (n - first_half),
            PERMILLE_PER_UNIT,
        ),
        holds: Holds {
            level0: window.holds_level0,
            level1: window.holds_level1,
        },
        retrace: over_range(unit * part),
        fib_zone,
        level_distance: over_range((edge_low - rung_view).abs()),
        relation,
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail."
)]
mod tests {
    use super::*;
    use crate::fibflag::extremes::DayExtremes;
    use crate::{Candle, OI_NULL};
    use core::cmp::Ordering;
    use vocab::fib_cube::RowId;

    fn ladder(high: i64, low: i64) -> Ladder {
        let mut day = DayExtremes::empty();
        let bar = Candle {
            ts_micros: 0,
            open: low,
            high,
            low,
            close: high,
            volume: 0,
            open_interest: OI_NULL,
        };
        day.step(&bar, 0, 1, false).unwrap();
        day.ladder(1).unwrap()
    }

    /// A window over long-view bars `(open, high, low, close)` at day indices
    /// from 0, its sums computed here.
    fn window(bars: &[(i64, i64, i64, i64)]) -> Window {
        let mut w = Window {
            candles: u32::try_from(bars.len()).unwrap(),
            first_index: 0,
            high: bars.iter().map(|b| b.1).max().unwrap(),
            low: bars.iter().map(|b| b.2).min().unwrap(),
            close_high: bars.iter().map(|b| b.3).max().unwrap(),
            close_low: bars.iter().map(|b| b.3).min().unwrap(),
            sum_high: 0,
            sum_low: 0,
            sum_close: 0,
            sum_x_high: 0,
            sum_x_low: 0,
            sum_x_close: 0,
            sum_xx_close: 0,
            sum_x: 0,
            sum_xx: 0,
            sum_xxx: 0,
            sum_xxxx: 0,
            sum_range: 0,
            sum_body: 0,
            range_first_half: 0,
            range_last_half: 0,
            close_first_third: 0,
            close_last_third: 0,
            inside_pairs: 0,
            rising_lows: 0,
            poke: 0,
            holds_level0: false,
            holds_level1: false,
            first: Bar { high: 0, low: 0 },
            middle: None,
            last: Bar { high: 0, low: 0 },
        };
        let half = bars.len() / 2;
        for (x, &(open, high, low, close)) in bars.iter().enumerate() {
            let x = i128::try_from(x).unwrap();
            w.sum_high += i128::from(high);
            w.sum_low += i128::from(low);
            w.sum_close += i128::from(close);
            w.sum_x_high += x * i128::from(high);
            w.sum_x_low += x * i128::from(low);
            w.sum_x_close += x * i128::from(close);
            w.sum_xx_close += x * x * i128::from(close);
            w.sum_x += x;
            w.sum_xx += x * x;
            w.sum_xxx += x * x * x;
            w.sum_xxxx += x * x * x * x;
            w.sum_range += i128::from(high - low);
            w.sum_body += i128::from((close - open).abs());
            if usize::try_from(x).unwrap() < half {
                w.range_first_half += i128::from(high - low);
            } else {
                w.range_last_half += i128::from(high - low);
            }
        }
        w
    }

    #[test]
    fn exact_compares_by_cross_multiplication_and_floors_toward_minus_infinity() {
        assert_eq!(Exact::new(1, 0), None);
        assert_eq!(Exact::new(1, -2), None);
        let third = Exact::new(1, 3).unwrap();
        assert_eq!((third.num(), third.den()), (1, 3));
        assert_eq!(third.floor(), 0);
        assert_eq!(Exact::new(-1, 3).unwrap().floor(), -1);
        assert_eq!(third.cmp_int(0), Some(Ordering::Greater));
        assert_eq!(third.cmp_int(1), Some(Ordering::Less));
        assert_eq!(Exact::new(6, 3).unwrap().cmp_int(2), Some(Ordering::Equal));
        assert_eq!(Exact::new(1, 3).unwrap().cmp_int(i128::MAX), None);
    }

    #[test]
    fn a_zero_denominator_has_the_stated_limits() {
        assert_eq!(ratio(0, 0, 1_000), Ratio::Value(1_000));
        assert_eq!(ratio(5, 0, 1_000), Ratio::OverZero);
        assert_eq!(ratio(1, 3, 1_000), Ratio::Value(333));
        assert_eq!(ratio(-1, 3, 1_000), Ratio::Value(333), "read as magnitude");
        assert_eq!(ratio(i128::MAX, 1, 1_000), Ratio::OverZero, "past u128");
        assert_eq!(
            ratio(i128::from(u64::MAX), 1, 1_000),
            Ratio::OverZero,
            "past u64"
        );
    }

    #[test]
    fn the_relations_are_the_cube_s() {
        for (relation, label) in [
            (LevelRelation::Above, "above"),
            (LevelRelation::Below, "below"),
            (LevelRelation::Spans, "spans"),
        ] {
            assert_eq!(
                RowId::LevelRelation.value_of(label),
                Some(relation.cube_value())
            );
        }
    }

    /// FM-F017's exact touch: 1.9999 paisa allowed, so a gap of 1 touches and
    /// a gap of 2 does not (rounding the tolerance up would let it).
    #[test]
    fn a_touch_is_decided_exactly_in_basis_points_of_the_edge() {
        assert!(touches(19_998, 19_999, 1));
        assert!(!touches(19_997, 19_999, 1));
        assert!(touches(19_999, 19_999, 0), "the edge itself");
        assert!(!touches(19_998, 19_999, 0));
        assert!(touches(-19_998, -19_999, 1), "a short view's negative edge");
        assert!(!touches(-19_997, -19_999, 1));
    }

    /// FM-F026: R = 1,001 and the base low 0.382 paisa above the exact 382
    /// level is ABOVE it; truncating the level to H - 382 would say spans.
    #[test]
    fn the_relation_is_decided_on_the_exact_level() {
        let ladder = ladder(3_001, 2_000);
        let w = window(&[(2_700, 2_800, 2_619, 2_700), (2_700, 2_750, 2_620, 2_700)]);
        let m = measure(&w, &ladder, TradeDir::Long);
        assert_eq!(m.fib_zone, 2, "rung 382");
        assert_eq!(m.relation, LevelRelation::Above);
        assert!(!m.holds.extreme() && !m.zero_width());
        let both = Holds {
            level0: false,
            level1: true,
        };
        assert!(both.extreme());
        assert_eq!(m.retrace, Exact::new(382_000, 1_001).unwrap());
        // |2,619,000 - (3,001,000 - 382,382)| / 1,001 = 382 / 1,001.
        assert_eq!(m.level_distance, Exact::new(382, 1_001).unwrap());
        let below = window(&[(2_610, 2_618, 2_600, 2_610), (2_610, 2_618, 2_601, 2_610)]);
        assert_eq!(
            measure(&below, &ladder, TradeDir::Long).relation,
            LevelRelation::Below
        );
        let spans = window(&[(2_610, 2_630, 2_600, 2_610), (2_610, 2_630, 2_601, 2_610)]);
        assert_eq!(
            measure(&spans, &ladder, TradeDir::Long).relation,
            LevelRelation::Spans
        );
    }

    #[test]
    fn a_window_outside_its_ladder_is_measured_without_a_panic() {
        let ladder = ladder(3_000, 2_000);
        let high = window(&[(3_500, 3_600, 3_400, 3_500), (3_500, 3_600, 3_400, 3_500)]);
        let m = measure(&high, &ladder, TradeDir::Long);
        assert_eq!(m.fib_zone, 0, "beyond level 0 is nearest rung 0");
        assert_eq!(m.retrace.cmp_int(0), Some(Ordering::Less));
        let low = window(&[(1_500, 1_600, 1_400, 1_500), (1_500, 1_600, 1_400, 1_500)]);
        let m = measure(&low, &ladder, TradeDir::Long);
        assert_eq!(m.fib_zone, 6, "beyond level 1 is nearest rung 1000");
        assert_eq!(m.relation, LevelRelation::Below);
        // One candle: no slope, no curvature, and the halves give 0 / 0.
        let one = window(&[(2_500, 2_600, 2_400, 2_500)]);
        let m = measure(&one, &ladder, TradeDir::Long);
        assert_eq!((m.slope_highs, m.curvature_closes), (None, None));
        assert_eq!(m.squeeze, Ratio::Value(1_000));
        // Sums past i128 give no slope and no curvature, never a panic.
        let mut huge = window(&[(2_500, 2_600, 2_400, 2_500), (2_500, 2_600, 2_400, 2_500)]);
        huge.sum_x = i128::MAX;
        huge.sum_xx = i128::MAX;
        let m = measure(&huge, &ladder, TradeDir::Long);
        assert_eq!((m.slope_highs, m.curvature_closes), (None, None));
    }

    #[test]
    fn slopes_and_curvature_are_exact_least_squares() {
        let ladder = ladder(10_000, 1_000);
        // highs 1,000 / 1,010 / 1,040 / 1,090: slope 30 per candle; closes on
        // y = 2,000 + 5 x^2: curvature exactly 5.
        let mut w = window(&[
            (1_900, 2_100, 1_800, 2_000),
            (1_900, 2_110, 1_800, 2_005),
            (1_900, 2_140, 1_800, 2_020),
            (1_900, 2_190, 1_800, 2_045),
        ]);
        let m = measure(&w, &ladder, TradeDir::Long);
        let slope = m.slope_highs.unwrap();
        assert_eq!(slope.num(), 30 * slope.den());
        assert_eq!(m.slope_lows.unwrap().num(), 0);
        let curve = m.curvature_closes.unwrap();
        assert_eq!(curve.num(), 5 * curve.den());
        w = window(&[(1_900, 2_100, 1_800, 2_000), (1_900, 2_110, 1_800, 2_005)]);
        let two = measure(&w, &ladder, TradeDir::Long);
        assert!(
            two.curvature_closes.is_none(),
            "a parabola needs three candles"
        );
        assert_eq!(
            two.slope_highs.unwrap().num(),
            10 * two.slope_highs.unwrap().den()
        );
    }
}
