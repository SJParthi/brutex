//! The day extremes a base is measured against, folded one closed candle at a
//! time, and the trade-frame ladder over them (FIB-3; D-6340, D-6341, D-6342,
//! D-6346).
//!
//! # A sibling of `CurDayFib`, not an edit of it
//!
//! [`DayExtremes`] keeps the semantics of [`crate::CurDayFib`]: the strict `>`
//! and `<` fold (the first candle reaching an extreme keeps it), the reset on
//! [`crate::ist_day`], and the reset, emit, fold order fused into one step so a
//! caller cannot transpose them. It adds what a base needs: the index and the
//! close instant of the candle that made each extreme. `CurDayFib` is not
//! touched (vocabulary bits 121 to 131 must not change); on the same stream the
//! two hold the same endpoints.
//!
//! A record [`Candle::check_evaluable`] refuses is counted and never folded.
//!
//! # Extremes from a withheld bucket (FM-F025)
//!
//! `pull::fold` withholds an incomplete bucket, so a 3, 5 or 15-minute stream
//! can miss the minute that made the day's real high or low. The caller passes
//! the present minutes' high and low as a [`WithheldExtremes`] update: it is
//! folded here and nowhere else (never a base candle, a trigger or a decision),
//! and the ladder says `extreme_from_withheld_bucket` while that extreme
//! stands.
//!
//! # The ladder (D-6341, D-6342)
//!
//! After candle `t` folds, `R = H - L`. A ladder exists only when its rungs are
//! distinguishable on the price grid: the smallest gap between adjacent rungs of
//! [`crate::fib::RUNGS`] (118 of 1000 parts of `R`, computed, never typed) is at
//! least one observed tick, `R x gap >= 1000 x tick`. There is no typed minimum
//! range.
//!
//! A direction's TRADE FRAME puts level 0 at the day extreme in its direction
//! (long: `H`; short: `L`) and level 1 at the other. A rung `r` of `RUNGS`, in
//! parts of 1000, lies `r x R / 1000` from level 0 towards level 1 when `r` is
//! at most 1000, and an extension `r` above 1000 lies `(r - 1000) x R / 1000`
//! beyond level 0 in the trade's direction. Every level is the exact rational
//! `numerator / 1000` (FM-F026), never rounded.
//!
//! The short view is the long view on negated prices: its level 0 is `-L`.

use vocab::fib_cube::PERMILLE_PER_UNIT;

use super::types::TradeDir;
use crate::fib::RUNGS;
use crate::{Candle, Corrupt, OI_NULL, ist_day};

/// The smallest gap between adjacent rungs of [`RUNGS`], in parts of 1000 of
/// the day range: the ladder rule's grid (D-6342).
pub const MIN_RUNG_GAP: i32 = smallest_gap(&RUNGS);

/// The smallest difference between adjacent entries of an ascending list.
const fn smallest_gap(rungs: &[i32]) -> i32 {
    let [first, rest @ ..] = rungs else {
        return i32::MAX;
    };
    let [second, ..] = rest else {
        return i32::MAX;
    };
    let here = *second - *first;
    let further = smallest_gap(rest);
    if here < further { here } else { further }
}

/// One day extreme and the candle that made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Extreme {
    /// The price, in paisa.
    pub price: i64,
    /// The day index of the candle that made it; `None` when an extremes-only
    /// update of a withheld bucket made it (FM-F025), which is never a base
    /// candle.
    pub index: Option<u32>,
    /// The close instant of the candle, or the withheld bucket, that made it.
    pub close_ts_micros: i64,
    /// Whether the print that made it was marked suspect (FM-F096).
    pub suspect_print: bool,
}

impl Extreme {
    /// Whether a withheld bucket's extremes-only update made this extreme.
    #[must_use]
    pub const fn from_withheld_bucket(&self) -> bool {
        self.index.is_none()
    }
}

/// The day's extremes before a candle folded: what a step emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prior {
    /// The day high before the candle, in paisa.
    pub high: i64,
    /// The day low before the candle, in paisa.
    pub low: i64,
}

/// An extremes-only update (FM-F025): the high and the low of the minutes
/// present in a bucket `pull::fold` withheld, corrupt minutes excluded by the
/// caller, at the bucket's close instant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WithheldExtremes {
    /// The bucket's open instant (its IST day is the day it folds into).
    pub open_ts_micros: i64,
    /// The bucket's close instant.
    pub close_ts_micros: i64,
    /// The present minutes' high, in paisa.
    pub high: i64,
    /// The present minutes' low, in paisa.
    pub low: i64,
    /// Whether one of those minutes was marked suspect (FM-F096).
    pub suspect_print: bool,
}

/// The day high and low, folded one candle at a time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DayExtremes {
    /// The IST day being folded.
    day: i64,
    /// The high and the low, once a candle folded today.
    extremes: Option<(Extreme, Extreme)>,
    /// Records refused and never folded.
    refused: u64,
}

impl DayExtremes {
    /// No day yet.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            day: i64::MIN,
            extremes: None,
            refused: 0,
        }
    }

    /// Reset (when the candle's IST day is a new one), emit (the extremes
    /// before the candle, `None` for the day's first), fold. `index` is the
    /// candle's day index and `close_ts_micros` its close instant.
    ///
    /// # Errors
    ///
    /// The [`Corrupt`] reason of a record [`Candle::check_evaluable`]
    /// refuses: counted, and nothing changes.
    pub fn step(
        &mut self,
        candle: &Candle,
        index: u32,
        close_ts_micros: i64,
        suspect_print: bool,
    ) -> Result<Option<Prior>, Corrupt> {
        self.admit(candle)?;
        self.reset_on(ist_day(candle.ts_micros));
        let prior = self.prior();
        let made = |price| Extreme {
            price,
            index: Some(index),
            close_ts_micros,
            suspect_print,
        };
        self.fold(made(candle.high), made(candle.low));
        Ok(prior)
    }

    /// Folds an extremes-only update of a withheld bucket (FM-F025), with the
    /// same reset and the same strict rule. It never makes a base candle.
    ///
    /// # Errors
    ///
    /// The [`Corrupt`] reason when the high is below the low or the low is
    /// not positive: counted, and nothing changes.
    pub fn step_withheld(&mut self, update: &WithheldExtremes) -> Result<(), Corrupt> {
        self.admit(&Candle {
            ts_micros: update.open_ts_micros,
            open: update.high,
            high: update.high,
            low: update.low,
            close: update.low,
            volume: 0,
            open_interest: OI_NULL,
        })?;
        self.reset_on(ist_day(update.open_ts_micros));
        let made = |price| Extreme {
            price,
            index: None,
            close_ts_micros: update.close_ts_micros,
            suspect_print: update.suspect_print,
        };
        self.fold(made(update.high), made(update.low));
        Ok(())
    }

    /// Refuses, and counts, a record that is not a bar.
    fn admit(&mut self, candle: &Candle) -> Result<(), Corrupt> {
        candle.check_evaluable().inspect_err(|_| {
            self.refused = self.refused.saturating_add(1);
        })
    }

    /// Starts a new day when `day` is not the one being folded.
    fn reset_on(&mut self, day: i64) {
        if day != self.day {
            self.day = day;
            self.extremes = None;
        }
    }

    /// The extremes so far, as a step emits them.
    fn prior(&self) -> Option<Prior> {
        self.extremes.map(|(high, low)| Prior {
            high: high.price,
            low: low.price,
        })
    }

    /// The strict fold: only a strictly higher high or a strictly lower low
    /// moves an extreme, so the first candle reaching it keeps it.
    fn fold(&mut self, high: Extreme, low: Extreme) {
        self.extremes = Some(match self.extremes {
            None => (high, low),
            Some((was_high, was_low)) => (
                if high.price > was_high.price {
                    high
                } else {
                    was_high
                },
                if low.price < was_low.price {
                    low
                } else {
                    was_low
                },
            ),
        });
    }

    /// The IST day being folded, once a record folded.
    #[must_use]
    pub fn day(&self) -> Option<i64> {
        self.extremes.map(|_| self.day)
    }

    /// The day high and the candle that made it.
    #[must_use]
    pub fn high(&self) -> Option<Extreme> {
        self.extremes.map(|(high, _)| high)
    }

    /// The day low and the candle that made it.
    #[must_use]
    pub fn low(&self) -> Option<Extreme> {
        self.extremes.map(|(_, low)| low)
    }

    /// Records refused and never folded.
    #[must_use]
    pub const fn refused(&self) -> u64 {
        self.refused
    }

    /// The ladder over today's extremes at the observed tick, or `None`: no
    /// candle yet, a tick that is not positive, or a range whose adjacent
    /// rungs are less than one tick apart (`no_ladder`, D-6342).
    #[must_use]
    pub fn ladder(&self, tick: i64) -> Option<Ladder> {
        let (high, low) = self.extremes?;
        let ladder = Ladder { high, low };
        let spread = i128::from(ladder.range()) * i128::from(MIN_RUNG_GAP);
        let grid = i128::from(PERMILLE_PER_UNIT) * i128::from(tick);
        (tick > 0 && spread >= grid).then_some(ladder)
    }
}

/// The day's two extremes after a candle folded, when they make a ladder. Only
/// [`DayExtremes::ladder`] builds one, so its range is always positive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Ladder {
    /// The day high.
    high: Extreme,
    /// The day low.
    low: Extreme,
}

impl Ladder {
    /// The day high and the candle that made it.
    #[must_use]
    pub const fn high(&self) -> Extreme {
        self.high
    }

    /// The day low and the candle that made it.
    #[must_use]
    pub const fn low(&self) -> Extreme {
        self.low
    }

    /// `R = H - L`, in paisa. Both are positive prices, so it fits.
    #[must_use]
    pub const fn range(&self) -> i64 {
        self.high.price - self.low.price
    }

    /// Level 0 of a direction's frame in its own view: `H` for a long, `-L`
    /// for a short (the long view on negated prices).
    #[must_use]
    pub const fn level0_view(&self, direction: TradeDir) -> i64 {
        match direction {
            TradeDir::Long => self.high.price,
            TradeDir::Short => -self.low.price,
        }
    }

    /// Rung `rung` (parts of 1000) of a direction's frame, in that direction's
    /// view, times 1000: exact, never rounded (FM-F026). A rung of
    /// [`RUNGS`] is non-negative, so `rung.unsigned_abs()` passes it exactly.
    #[must_use]
    pub fn rung_view(&self, direction: TradeDir, rung: u64) -> i128 {
        let unit = i128::from(PERMILLE_PER_UNIT);
        let level0 = unit * i128::from(self.level0_view(direction));
        let range = i128::from(self.range());
        let rung = i128::from(rung);
        if rung <= unit {
            level0 - rung * range
        } else {
            level0 + (rung - unit) * range
        }
    }

    /// Rung `rung` of a direction's frame as a price, times 1000 (FM-F026):
    /// the view's level, un-negated for a short.
    #[must_use]
    pub fn rung_price(&self, direction: TradeDir, rung: u64) -> i128 {
        let view = self.rung_view(direction, rung);
        match direction {
            TradeDir::Long => view,
            TradeDir::Short => -view,
        }
    }

    /// The stamps a base measured against this ladder carries.
    #[must_use]
    pub const fn stamp(&self) -> LadderStamp {
        LadderStamp {
            high: self.high.price,
            low: self.low.price,
            extreme_from_withheld_bucket: self.high.from_withheld_bucket()
                || self.low.from_withheld_bucket(),
            frame_from_suspect_print: self.high.suspect_print || self.low.suspect_print,
        }
    }
}

/// The ladder a base was measured against, as its snapshot carries it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LadderStamp {
    /// The day high at the base's last candle, in paisa.
    pub high: i64,
    /// The day low at the base's last candle, in paisa.
    pub low: i64,
    /// An extreme of the frame came from a withheld bucket (FM-F025).
    pub extreme_from_withheld_bucket: bool,
    /// An extreme of the frame came from a print marked suspect (FM-F096).
    pub frame_from_suspect_print: bool,
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

    const DAY: i64 = 20_000 * 86_400 * 1_000_000;

    fn bar(minute: i64, high: i64, low: i64) -> Candle {
        Candle {
            ts_micros: DAY + minute * 60_000_000,
            open: low,
            high,
            low,
            close: high,
            volume: 1,
            open_interest: OI_NULL,
        }
    }

    #[test]
    fn the_rung_grid_is_118_of_1000_and_computed() {
        assert_eq!(MIN_RUNG_GAP, 118);
        assert_eq!(smallest_gap(&[]), i32::MAX);
        assert_eq!(smallest_gap(&[5]), i32::MAX);
        assert_eq!(smallest_gap(&[0, 9, 12, 40]), 3);
        assert_eq!(smallest_gap(&[0, 2, 9]), 2);
    }

    #[test]
    fn the_first_candle_reaching_an_extreme_keeps_it() {
        let mut day = DayExtremes::empty();
        assert_eq!(day.day(), None);
        assert_eq!(day.high(), None);
        assert_eq!(day.ladder(1), None);
        assert_eq!(day.step(&bar(0, 1_100, 1_000), 0, 7, false), Ok(None));
        assert_eq!(
            day.step(&bar(1, 1_100, 1_000), 1, 8, true),
            Ok(Some(Prior {
                high: 1_100,
                low: 1_000
            }))
        );
        let high = day.high().unwrap();
        assert_eq!(
            (high.price, high.index, high.close_ts_micros),
            (1_100, Some(0), 7)
        );
        assert!(!high.suspect_print);
        day.step(&bar(2, 1_200, 990), 2, 9, true).unwrap();
        let (high, low) = (day.high().unwrap(), day.low().unwrap());
        assert_eq!((high.price, high.index), (1_200, Some(2)));
        assert_eq!((low.price, low.index), (990, Some(2)));
        assert!(high.suspect_print && low.suspect_print);
        assert_eq!(day.day(), Some(ist_day(DAY)));
    }

    #[test]
    fn a_corrupt_record_is_counted_and_never_folded() {
        let mut day = DayExtremes::empty();
        day.step(&bar(0, 1_100, 1_000), 0, 7, false).unwrap();
        let before = day;
        let inverted = Candle {
            low: 1_200,
            ..bar(1, 1_100, 1_000)
        };
        assert_eq!(day.step(&inverted, 1, 8, false), Err(Corrupt::HighBelowLow));
        let zero = bar(2, 1_100, 0);
        assert_eq!(day.step(&zero, 1, 8, false), Err(Corrupt::PriceNotPositive));
        let withheld = WithheldExtremes {
            open_ts_micros: DAY,
            close_ts_micros: 9,
            high: 5,
            low: 6,
            suspect_print: false,
        };
        assert_eq!(day.step_withheld(&withheld), Err(Corrupt::HighBelowLow));
        assert_eq!(day.refused(), 3);
        assert_eq!(day.high(), before.high());
        assert_eq!(day.low(), before.low());
    }

    #[test]
    fn a_new_ist_day_resets_and_a_withheld_bucket_stamps_its_extreme() {
        let mut day = DayExtremes::empty();
        day.step(&bar(0, 1_100, 1_000), 0, 7, false).unwrap();
        let update = WithheldExtremes {
            open_ts_micros: DAY + 60_000_000,
            close_ts_micros: 99,
            high: 1_300,
            low: 1_050,
            suspect_print: true,
        };
        day.step_withheld(&update).unwrap();
        let high = day.high().unwrap();
        assert!(high.from_withheld_bucket() && high.suspect_print);
        assert_eq!((high.price, high.close_ts_micros), (1_300, 99));
        assert!(!day.low().unwrap().from_withheld_bucket());
        let stamp = day.ladder(1).unwrap().stamp();
        assert!(stamp.extreme_from_withheld_bucket && stamp.frame_from_suspect_print);
        // The next day starts from its own first record, withheld or not.
        let next = WithheldExtremes {
            open_ts_micros: DAY + 86_400 * 1_000_000,
            ..update
        };
        day.step_withheld(&next).unwrap();
        assert_eq!(day.day(), Some(ist_day(DAY) + 1));
        assert_eq!(day.low().unwrap().price, 1_050);
        let tomorrow = bar(24 * 60, 1_000, 900);
        assert_eq!(
            day.step(&tomorrow, 0, 5, false),
            Ok(Some(Prior {
                high: 1_300,
                low: 1_050
            }))
        );
        let stamp = day.ladder(1).unwrap().stamp();
        assert!(stamp.extreme_from_withheld_bucket, "the high still stands");
        assert!(!day.low().unwrap().from_withheld_bucket());
    }

    #[test]
    fn a_ladder_needs_one_tick_between_adjacent_rungs() {
        let mut day = DayExtremes::empty();
        // R = 1,000 paisa: 1,000 x 118 >= 1,000 x 118 at tick 118, not at 119.
        day.step(&bar(0, 2_000, 1_000), 0, 7, false).unwrap();
        assert!(day.ladder(118).is_some());
        assert!(day.ladder(119).is_none());
        assert!(day.ladder(0).is_none(), "no observed tick, no ladder");
        let mut flat = DayExtremes::empty();
        flat.step(&bar(0, 1_000, 1_000), 0, 7, false).unwrap();
        assert!(flat.ladder(1).is_none(), "a zero range has no ladder");
    }

    #[test]
    fn rungs_are_exact_and_extensions_lie_beyond_level_zero() {
        let mut day = DayExtremes::empty();
        day.step(&bar(0, 3_001, 2_000), 0, 7, false).unwrap();
        let ladder = day.ladder(1).unwrap();
        assert_eq!(ladder.range(), 1_001);
        // Long: 382 lies 382.382 paisa below H, exactly.
        assert_eq!(ladder.rung_price(TradeDir::Long, 382), 3_001_000 - 382_382);
        assert_eq!(ladder.rung_price(TradeDir::Long, 0), 3_001_000);
        assert_eq!(ladder.rung_price(TradeDir::Long, 1_000), 2_000_000);
        assert_eq!(
            ladder.rung_price(TradeDir::Long, 1_272),
            3_001_000 + 272_272
        );
        // Short: level 0 at L, rungs rise towards H, extensions fall below L.
        assert_eq!(ladder.level0_view(TradeDir::Short), -2_000);
        assert_eq!(ladder.rung_price(TradeDir::Short, 382), 2_000_000 + 382_382);
        assert_eq!(
            ladder.rung_price(TradeDir::Short, 1_618),
            2_000_000 - 618_618
        );
        assert_eq!(ladder.rung_view(TradeDir::Short, 0), -2_000_000);
        assert_eq!((ladder.high().price, ladder.high().index), (3_001, Some(0)));
        assert_eq!(
            (ladder.low().price, ladder.low().close_ts_micros),
            (2_000, 7)
        );
        let stamp = ladder.stamp();
        assert_eq!((stamp.high, stamp.low), (3_001, 2_000));
        assert!(!stamp.extreme_from_withheld_bucket && !stamp.frame_from_suspect_print);
    }
}
