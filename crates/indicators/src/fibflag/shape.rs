//! The base shape scores behind the `base_shape` row (FIB-3; D-6343, D-6345).
//!
//! Each score is an integer in permille of the base's width, computed in the
//! base's own direction's view (the short view is the long view's code on
//! negated prices), from the [`Window`] the detector already holds: O(1) per
//! base. The names are LABELS for regions of the measurements, never
//! thresholds; the SHAPE LABEL is chosen later by the caller, as the argmax of
//! each score's own-past permille within its kind's family (FIB-2's histogram
//! bank), ties to the lower index. `indicators::pattern`'s predicates are not
//! reused: their thresholds are fixed classical conventions labelled
//! UNVERIFIED, which nothing typed forbids here.
//!
//! | Family | Score | Permille of the width `W` |
//! |---|---|---|
//! | rest | `u` | mean close of the first and last thirds minus the middle third's, at least 0 |
//! | rest | `inverted_u` | the same, negated, at least 0 |
//! | rest | `flat` | `1000 - (mean(high edge - high) + mean(low - low edge)) / 2W` |
//! | rest | `triangle` | `1000 -` the squeeze (last half's mean range over the first's), at least 0 |
//! | rest | `inside_candles` | the share of candles inside the one before |
//! | rest | `tight_closes` | `1000 - (highest close - lowest close) / W` |
//! | rest | `rising_lows` | the share of consecutive higher lows |
//! | rest | `shakeout_reclaim` | the deepest poke below the day low that closed back above it |
//! | micro | `v_bottom` | three candles: the middle low below both neighbours, its depth; else 0 |
//! | micro | `partial_u` | `1000 - |last low - first low| / W` |
//! | micro | `inside_pair` | 1000 when the last candle lies inside the one before, else 0 |
//! | micro | `stair` | the share of higher highs and higher lows |
//!
//! A permille is rounded toward zero. A rest of fewer than three candles has
//! no thirds: its `u` and `inverted_u` are 0.
//!
//! # A zero width (FM-F024)
//!
//! `W == 0` never divides: `flat`, `tight_closes` and `partial_u` are 1000,
//! `v_bottom`, `shakeout_reclaim`, `u` and `inverted_u` are 0. The ranges are
//! then zero too, so the squeeze is `0 / 0` = 1000 and `triangle` is 0.

use vocab::fib_cube::{BASE_SHAPE_ROW, PERMILLE_PER_UNIT, SHAPE_HALVES, SHAPE_THIRDS};

use super::measure::{Bar, Ratio, Window, ratio};
use super::types::BaseKind;

/// How many shapes the `base_shape` row has.
pub const SHAPES: usize = BASE_SHAPE_ROW.labels.len();

/// A shape, by its `base_shape` value index. The declaration order is the
/// row's, proved by `the_shapes_are_the_cube_s`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Shape {
    /// Rest: ends above the middle.
    U,
    /// Rest: the middle above the ends.
    InvertedU,
    /// Rest: highs and lows hug the edges.
    Flat,
    /// Rest: the ranges contract.
    Triangle,
    /// Rest: candles inside the one before.
    InsideCandles,
    /// Rest: the closes bunch together.
    TightCloses,
    /// Rest: consecutive higher lows.
    RisingLows,
    /// Rest: a poke below the day low that closed back above it.
    ShakeoutReclaim,
    /// Micro: a middle low below both neighbours.
    VBottom,
    /// Micro: the last low back at the first.
    PartialU,
    /// Micro: the last candle inside the one before.
    InsidePair,
    /// Micro: higher highs and higher lows.
    Stair,
}

impl Shape {
    /// Every shape, in the row's order.
    pub const ALL: [Self; SHAPES] = [
        Self::U,
        Self::InvertedU,
        Self::Flat,
        Self::Triangle,
        Self::InsideCandles,
        Self::TightCloses,
        Self::RisingLows,
        Self::ShakeoutReclaim,
        Self::VBottom,
        Self::PartialU,
        Self::InsidePair,
        Self::Stair,
    ];

    /// The rest family's scores.
    pub const REST: &[Self] = &[
        Self::U,
        Self::InvertedU,
        Self::Flat,
        Self::Triangle,
        Self::InsideCandles,
        Self::TightCloses,
        Self::RisingLows,
        Self::ShakeoutReclaim,
    ];

    /// The micro family's scores.
    pub const MICRO: &[Self] = &[Self::VBottom, Self::PartialU, Self::InsidePair, Self::Stair];

    /// The shape's value index in the `base_shape` row.
    #[must_use]
    pub const fn cube_value(self) -> u8 {
        self as u8
    }

    /// The family of scores a kind of base is labelled from.
    #[must_use]
    pub const fn family(kind: BaseKind) -> &'static [Self] {
        match kind {
            BaseKind::Rest => Self::REST,
            BaseKind::Micro2 | BaseKind::Micro3 => Self::MICRO,
        }
    }
}

/// One base's shape scores, in permille of its width; a score outside the
/// base's family is 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShapeScores {
    /// The scores, by shape.
    scores: [u16; SHAPES],
}

impl ShapeScores {
    /// One shape's score.
    #[must_use]
    pub fn get(&self, shape: Shape) -> u16 {
        self.scores
            .iter()
            .zip(Shape::ALL)
            .find(|(_, at)| *at == shape)
            .map_or(0, |(score, _)| *score)
    }

    /// Every score, in the row's order.
    #[must_use]
    pub const fn all(&self) -> &[u16; SHAPES] {
        &self.scores
    }

    /// Sets one shape's score.
    fn set(&mut self, shape: Shape, score: u16) {
        for (slot, at) in self.scores.iter_mut().zip(Shape::ALL) {
            if at == shape {
                *slot = score;
            }
        }
    }
}

/// One whole, in permille.
fn whole() -> u16 {
    u16::try_from(PERMILLE_PER_UNIT).unwrap_or(u16::MAX)
}

/// `num / den` in permille, rounded toward zero, for `0 <= num <= den`;
/// total: a pair the detector never builds gets the nearest of 0 and 1000.
fn permille(num: i128, den: i128) -> u16 {
    match ratio(num.max(0), den.max(0), PERMILLE_PER_UNIT) {
        Ratio::Value(value) => u16::try_from(value).unwrap_or(u16::MAX).min(whole()),
        Ratio::OverZero => whole(),
    }
}

/// `1000 - num / den` in permille, for `0 <= num <= den`.
fn complement(num: i128, den: i128) -> u16 {
    whole() - permille(num, den)
}

/// The scores of a base of `kind` from its window.
#[must_use]
pub fn scores(w: &Window, kind: BaseKind) -> ShapeScores {
    let mut out = ShapeScores {
        scores: [0; SHAPES],
    };
    let width = i128::from(w.high) - i128::from(w.low);
    match kind {
        BaseKind::Rest => rest(w, width, &mut out),
        BaseKind::Micro2 | BaseKind::Micro3 => micro(w, width, &mut out),
    }
    out
}

/// The rest family.
fn rest(w: &Window, width: i128, out: &mut ShapeScores) {
    let n = i128::from(w.candles);
    let pairs = n - 1;
    let third = n / i128::from(SHAPE_THIRDS);
    let middle = n - third - third;
    let (u, inverted) = if third == 0 || width == 0 {
        (0, 0)
    } else {
        let ends = w.close_first_third + w.close_last_third;
        let mid = w.sum_close - ends;
        let diff = ends * middle - mid * (third + third);
        let den = (third + third) * middle * width;
        (permille(diff, den), permille(-diff, den))
    };
    out.set(Shape::U, u);
    out.set(Shape::InvertedU, inverted);
    let (flat, tight, shakeout) = if width == 0 {
        (whole(), whole(), 0)
    } else {
        let deviation = (n * i128::from(w.high) - w.sum_high) + (w.sum_low - n * i128::from(w.low));
        (
            complement(deviation, (n + n) * width),
            complement(i128::from(w.close_high) - i128::from(w.close_low), width),
            permille(i128::from(w.poke), width),
        )
    };
    out.set(Shape::Flat, flat);
    out.set(Shape::TightCloses, tight);
    out.set(Shape::ShakeoutReclaim, shakeout);
    let first_half = n / i128::from(SHAPE_HALVES);
    let triangle = match ratio(
        w.range_last_half * first_half,
        w.range_first_half * (n - first_half),
        PERMILLE_PER_UNIT,
    ) {
        Ratio::Value(squeeze) => whole().saturating_sub(u16::try_from(squeeze).unwrap_or(u16::MAX)),
        Ratio::OverZero => 0,
    };
    out.set(Shape::Triangle, triangle);
    out.set(
        Shape::InsideCandles,
        permille(i128::from(w.inside_pairs), pairs),
    );
    out.set(
        Shape::RisingLows,
        permille(i128::from(w.rising_lows), pairs),
    );
}

/// Whether `later` lies inside `earlier`.
const fn inside(later: Bar, earlier: Bar) -> bool {
    later.high <= earlier.high && later.low >= earlier.low
}

/// The micro family.
fn micro(w: &Window, width: i128, out: &mut ShapeScores) {
    let (first, last) = (w.first, w.last);
    let before_last = w.middle.unwrap_or(first);
    let v_bottom = w
        .middle
        .filter(|mid| mid.low < first.low && mid.low < last.low)
        .map_or(0, |mid| {
            permille(
                i128::from(first.low.min(last.low)) - i128::from(mid.low),
                width,
            )
        });
    out.set(Shape::VBottom, v_bottom);
    let partial_u = if width == 0 {
        whole()
    } else {
        complement((i128::from(last.low) - i128::from(first.low)).abs(), width)
    };
    out.set(Shape::PartialU, partial_u);
    out.set(
        Shape::InsidePair,
        if inside(last, before_last) {
            whole()
        } else {
            0
        },
    );
    let step = |a: Bar, b: Bar| i128::from(b.high > a.high) + i128::from(b.low > a.low);
    let rises = match w.middle {
        Some(mid) => step(first, mid) + step(mid, last),
        None => step(first, last),
    };
    let pairs = i128::from(w.candles) - 1;
    out.set(Shape::Stair, permille(rises, pairs + pairs));
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail."
)]
mod tests {
    use super::*;
    use vocab::fib_cube::RowId;

    /// A window of `candles` between `low` and `high` with every sum zero.
    fn window(candles: u32, high: i64, low: i64) -> Window {
        let bar = Bar { high, low };
        Window {
            candles,
            first_index: 0,
            high,
            low,
            close_high: high,
            close_low: low,
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
            first: bar,
            middle: None,
            last: bar,
        }
    }

    #[test]
    fn the_shapes_are_the_cube_s_and_each_kind_has_its_family() {
        for shape in Shape::ALL {
            let label = BASE_SHAPE_ROW.labels[usize::from(shape.cube_value())];
            assert_eq!(RowId::BaseShape.value_of(label), Some(shape.cube_value()));
        }
        assert_eq!(Shape::ALL.len(), Shape::REST.len() + Shape::MICRO.len());
        assert_eq!(Shape::family(BaseKind::Rest), Shape::REST);
        assert_eq!(Shape::family(BaseKind::Micro2), Shape::MICRO);
        assert_eq!(Shape::family(BaseKind::Micro3), Shape::MICRO);
        let scores = scores(&window(2, 10, 0), BaseKind::Micro2);
        assert_eq!(scores.all().len(), SHAPES);
        assert_eq!(scores.get(Shape::InsidePair), scores.all()[10]);
    }

    #[test]
    fn a_u_and_an_inverted_u_need_thirds() {
        // Three candles, closes 9 / 1 / 9 in a width of 10: ends 9, middle 1.
        let mut u = window(3, 10, 0);
        u.close_first_third = 9;
        u.close_last_third = 9;
        u.sum_close = 19;
        let s = scores(&u, BaseKind::Rest);
        assert_eq!((s.get(Shape::U), s.get(Shape::InvertedU)), (800, 0));
        u.sum_close = 18 + 9;
        u.close_first_third = 1;
        u.close_last_third = 1;
        u.sum_close = 11;
        let s = scores(&u, BaseKind::Rest);
        assert_eq!((s.get(Shape::U), s.get(Shape::InvertedU)), (0, 800));
        // Two candles have no thirds.
        let two = window(2, 10, 0);
        assert_eq!(scores(&two, BaseKind::Rest).get(Shape::U), 0);
    }

    #[test]
    fn a_zero_width_rest_has_the_stated_limits() {
        let flat = window(4, 50, 50);
        let s = scores(&flat, BaseKind::Rest);
        for (shape, limit) in [
            (Shape::Flat, 1_000),
            (Shape::TightCloses, 1_000),
            (Shape::ShakeoutReclaim, 0),
            (Shape::U, 0),
            (Shape::InvertedU, 0),
            (Shape::Triangle, 0),
        ] {
            assert_eq!(s.get(shape), limit, "{shape:?}");
        }
        for shape in Shape::MICRO {
            assert_eq!(s.get(*shape), 0, "outside the family");
        }
    }

    #[test]
    fn a_ranged_rest_scores_its_edges_closes_ranges_and_pokes() {
        let mut w = window(4, 100, 0);
        // Highs 100, 90, 90, 80 and lows 0, 10, 10, 20: deviations 40 + 40.
        w.sum_high = 360;
        w.sum_low = 40;
        w.close_high = 70;
        w.close_low = 30;
        w.range_first_half = 180;
        w.range_last_half = 140;
        w.inside_pairs = 3;
        w.rising_lows = 2;
        w.poke = 25;
        let s = scores(&w, BaseKind::Rest);
        assert_eq!(s.get(Shape::Flat), 900, "1000 - 80 x 1000 / 800");
        assert_eq!(s.get(Shape::TightCloses), 600);
        assert_eq!(s.get(Shape::Triangle), 223, "1000 - 140 x 1000 / 180");
        assert_eq!(s.get(Shape::InsideCandles), 1_000);
        assert_eq!(s.get(Shape::RisingLows), 666);
        assert_eq!(s.get(Shape::ShakeoutReclaim), 250);
        // A flat first half under a ranged last half: x / 0, no triangle.
        w.range_first_half = 0;
        assert_eq!(scores(&w, BaseKind::Rest).get(Shape::Triangle), 0);
        // An expanding range: the squeeze passes 1000 and the triangle is 0.
        w.range_first_half = 100;
        w.range_last_half = 300;
        assert_eq!(scores(&w, BaseKind::Rest).get(Shape::Triangle), 0);
        // A window the detector never builds (one candle, one pair) is still
        // scored without a panic: a share over zero pairs is a whole.
        let mut one = window(1, 100, 0);
        one.inside_pairs = 1;
        assert_eq!(
            scores(&one, BaseKind::Rest).get(Shape::InsideCandles),
            1_000
        );
    }

    #[test]
    fn micro_scores_read_their_two_or_three_candles() {
        let mut v = window(3, 100, 0);
        v.first = Bar { high: 100, low: 40 };
        v.middle = Some(Bar { high: 60, low: 0 });
        v.last = Bar { high: 80, low: 30 };
        let s = scores(&v, BaseKind::Micro3);
        assert_eq!(
            s.get(Shape::VBottom),
            300,
            "30 below the shallower neighbour"
        );
        assert_eq!(s.get(Shape::PartialU), 900);
        assert_eq!(s.get(Shape::InsidePair), 0, "80 is above the middle's 60");
        // first -> middle: neither higher; middle -> last: both higher.
        assert_eq!(s.get(Shape::Stair), 500);
        for shape in Shape::REST {
            assert_eq!(s.get(*shape), 0, "outside the family");
        }
        let mut pair = window(2, 100, 0);
        pair.first = Bar { high: 100, low: 0 };
        pair.last = Bar { high: 90, low: 10 };
        let s = scores(&pair, BaseKind::Micro2);
        assert_eq!(s.get(Shape::VBottom), 0, "two candles have no middle");
        assert_eq!(s.get(Shape::InsidePair), 1_000);
        assert_eq!(s.get(Shape::Stair), 500, "a higher low, a lower high");
        let flat = window(2, 70, 70);
        let s = scores(&flat, BaseKind::Micro2);
        assert_eq!(
            (s.get(Shape::PartialU), s.get(Shape::InsidePair)),
            (1_000, 1_000)
        );
        assert_eq!(s.get(Shape::Stair), 0);
    }
}
