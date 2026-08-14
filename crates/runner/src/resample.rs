//! One-minute candles folded into a coarser timeframe.
//!
//! # Why this exists, and what was a lie without it
//!
//! `CLAUDE.md` §3 rule 3 makes `timeframe` a term in the run identity, so
//! `(NIFTY, 1min)` and `(NIFTY, 5min)` are two runs with two different digests.
//! Nothing in this workspace produced the second one. A caller could pass
//! `"5min"` to `identity()` today, get a distinct digest, and have swept the
//! same one-minute bars — a five-minute answer nobody computed, which is exactly
//! the §4 breach of reporting a result that was never measured.
//!
//! # The trap, and it is §3 rule 7
//!
//! A five-minute bar's `close` is the close of its LAST minute. Emit the bucket
//! before that minute has arrived and the bar depends on data from the future;
//! emit it at the first minute and every field but `open` is wrong. So a bucket
//! is emitted only once it is **provably complete** — when a bar belonging to a
//! later bucket has been seen, or the input has ended.
//!
//! The trailing bucket **is** emitted, and the distinction is the whole of it:
//! it is complete *for the data given*. The input has ended, so no later bar can
//! revise it. Twelve minutes at five-minute resolution yield three bars, the
//! last covering two minutes.
//!
//! What that bar is **not** is complete for a session still trading. A caller
//! streaming live bars must drop the final bar of each batch, because this
//! function cannot tell a finished day from a paused feed — and inventing that
//! distinction would mean consulting a calendar, which is exactly what the
//! section below explains this module must never do.
//!
//! # Buckets are keyed on the CLOCK, never on position in the slice
//!
//! Grouping every five consecutive elements would make the answer depend on
//! where the slice happens to start: the same market data sliced from 09:16
//! instead of 09:15 would produce different bars with the same identity, and
//! §3 rule 5's byte-for-byte reproducibility would be false. The bucket is
//! `(ts_micros + IST offset) / period`, floored -- so a bar's bucket is a
//! property of the bar, on the same IST grid `crates/pull/src/fold.rs` and
//! `crates/store` use. Anchoring on the bare epoch puts every edge at UTC
//! midnight; fold.rs shipped that once and the store held 20 records stamped on
//! a SUNDAY.
//!
//! # Sessions need no special case, and that is the point
//!
//! An overnight gap spans thousands of buckets, so the last bar of one day and
//! the first of the next land in different buckets by arithmetic alone. No
//! calendar, no session table, no exchange rule — which means nothing here can
//! disagree with `crates/pull`'s calendar, because it does not consult one.

use indicators::{Candle, IST_OFFSET_MICROS, OI_NULL};

/// Microseconds in one minute — the resolution every stored bar is at.
const MINUTE_MICROS: i64 = 60_000_000;

/// A coarser timeframe, expressed in whole minutes.
///
/// A newtype rather than a bare `u32` so a caller cannot pass a bar count where
/// a period belongs. Zero and one are refused at construction: one is the input
/// resolution and folding it is a copy, zero is not a duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Period(u32);

impl Period {
    /// A period of `minutes`, or `None` for zero and one.
    ///
    /// One is refused rather than treated as identity because a caller asking to
    /// resample to the resolution the data is already at has made a mistake, and
    /// silently returning a copy would hide it.
    #[must_use]
    pub const fn minutes(minutes: u32) -> Option<Self> {
        if minutes < 2 {
            None
        } else {
            Some(Self(minutes))
        }
    }

    /// The period in whole minutes.
    #[must_use]
    pub const fn as_minutes(self) -> u32 {
        self.0
    }

    /// The label this period contributes to a run identity — `"5min"`.
    ///
    /// Built here rather than by a caller so two runs at the same period cannot
    /// disagree about their own name and therefore about their digest.
    #[must_use]
    pub fn label(self) -> String {
        format!("{}min", self.0)
    }

    /// The period in microseconds.
    const fn micros(self) -> i64 {
        (self.0 as i64).saturating_mul(MINUTE_MICROS)
    }
}

/// Which bucket a timestamp belongs to, on the **IST** grid.
///
/// Floored division, and `div_euclid` rather than `/` so a negative timestamp
/// floors downward instead of toward zero. No bar before 1970 exists in this
/// store, but a truncating divide would silently merge the buckets either side
/// of the epoch, and a bucket key that is wrong only for one input is worse than
/// one that is wrong for all of them.
///
/// # The anchor, and the incident that names it
///
/// This was `ts_micros.div_euclid(..)` on the bare epoch, which anchors every
/// edge at **UTC** midnight. `crates/pull/src/fold.rs` had exactly that bug and
/// records what it cost: *every daily bar moved back one calendar day*, and the
/// store held **20 records stamped on a SUNDAY** on an exchange that trades
/// Monday to Friday. `store::path::Timeframe` states as fact that "the fold grid
/// is anchored at IST midnight".
///
/// The two grids agree only when the period divides the 330-minute offset —
/// 5, 15 and 30 minutes do, so the fixtures here never saw it. **60 does not**,
/// and 60 is a `Timeframe::KNOWN` rung, so an hourly resample here would have
/// disagreed with the store's own fold on the same bars. Found by an
/// adversarial audit, not by a test.
///
/// [`indicators::IST_OFFSET_MICROS`] is used rather than a second copy of
/// 19,800: one definition across three crates is what stops them drifting.
const fn bucket_of(ts_micros: i64, period: Period) -> i64 {
    ts_micros
        .saturating_add(IST_OFFSET_MICROS)
        .div_euclid(period.micros())
}

/// The UTC timestamp at which `bucket` begins.
///
/// The inverse of [`bucket_of`], and it must carry the same anchor or a bar
/// would be stamped on a grid it was not bucketed on.
const fn bucket_start(bucket: i64, period: Period) -> i64 {
    bucket
        .saturating_mul(period.micros())
        .saturating_sub(IST_OFFSET_MICROS)
}

/// Folds one-minute `bars` into `period` bars.
///
/// Input is assumed to be in ascending timestamp order, which is what
/// `indicators::column::Column` already refuses to sweep without — a
/// non-increasing timestamp is counted as a refusal there, so a caller that
/// resamples unsorted bars has a defect this function cannot see and the column
/// will.
///
/// # Cost
///
/// One pass, no allocation per bar, no lookahead buffer: the accumulator holds
/// one bucket. UNVERIFIED as a measured figure — no bench row covers this yet,
/// and `crates/runner/benches/ratio.rs` would need a `C-R-04` to claim one.
#[must_use]
pub fn resample(bars: &[Candle], period: Period) -> Vec<Candle> {
    // The output cannot be longer than the input divided by the period, plus one
    // for a leading partial. Reserving the ceiling avoids the doubling growth
    // gate 11 rule 3 is about.
    // `Period` refuses anything below two, so the divisor is never zero and
    // `div_ceil` cannot panic. A `checked_div(..).unwrap_or(..)` here left an
    // arm no input could reach, which is the coverage hole §9 refuses and,
    // worse, a fallback nobody has ever seen taken.
    let per = period.as_minutes() as usize;
    let mut out: Vec<Candle> = Vec::with_capacity(bars.len().div_ceil(per));
    let mut open: Option<(i64, Candle)> = None;

    for bar in bars {
        let key = bucket_of(bar.ts_micros, period);
        match open {
            // A bar in the bucket being accumulated.
            Some((k, ref mut acc)) if k == key => {
                acc.high = acc.high.max(bar.high);
                acc.low = acc.low.min(bar.low);
                acc.close = bar.close;
                acc.volume = acc.volume.saturating_add(bar.volume);
                acc.open_interest = fold_open_interest(acc.open_interest, bar.open_interest);
            }
            // A bar in a LATER bucket. The one being accumulated is now provably
            // complete -- nothing after this point can belong to it, because the
            // input ascends -- so it is emitted, and this bar opens the next.
            Some((_, acc)) => {
                out.push(acc);
                open = Some((key, opened(bar, period)));
            }
            None => open = Some((key, opened(bar, period))),
        }
    }
    // THE TRAILING BUCKET IS EMITTED, and this is the one place a caller must
    // read the doc. It is complete for the data GIVEN, which is the strongest
    // statement available: the input has ended, so no later bar can revise it.
    // What it is not is complete for a session still trading -- a caller
    // streaming live bars must therefore drop the final bar of each batch, and
    // that is the caller's to know because this function cannot tell a finished
    // day from a paused feed.
    if let Some((_, acc)) = open {
        out.push(acc);
    }
    out
}

/// A fresh accumulator opened at `bar`, timestamped to its BUCKET.
///
/// The stamp is the bucket's start, not the first bar's own time. A 5-minute bar
/// built from 09:17..09:19 is stamped 09:15 — otherwise two runs over data that
/// began at different minutes would produce bars with different timestamps from
/// the same market, and `data_digest` would disagree across them.
fn opened(bar: &Candle, period: Period) -> Candle {
    Candle {
        ts_micros: bucket_start(bucket_of(bar.ts_micros, period), period),
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.volume,
        open_interest: bar.open_interest,
    }
}

/// Open interest across a bucket: the LAST known value, never a sum.
///
/// Open interest is a level, not a flow — adding five minutes of it would report
/// five times the contracts outstanding. `CLAUDE.md` §7 reserves `i64::MIN` as
/// the null sentinel and says zero means zero, so an absent value must not
/// overwrite a known one and a genuine zero must.
const fn fold_open_interest(accumulated: i64, incoming: i64) -> i64 {
    if incoming == OI_NULL {
        accumulated
    } else {
        incoming
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{MINUTE_MICROS, Period, bucket_of, bucket_start, resample};
    use indicators::{Candle, IST_OFFSET_MICROS, OI_NULL};

    /// A one-minute bar at `minute` past the epoch.
    fn bar(minute: i64, open: i64, high: i64, low: i64, close: i64, volume: i64) -> Candle {
        Candle {
            ts_micros: minute.saturating_mul(MINUTE_MICROS),
            open,
            high,
            low,
            close,
            volume,
            open_interest: OI_NULL,
        }
    }

    fn five() -> Period {
        Period::minutes(5).expect("five is a valid period")
    }

    #[test]
    fn a_period_below_two_minutes_is_refused() {
        assert_eq!(Period::minutes(0), None, "zero is not a duration");
        assert_eq!(
            Period::minutes(1),
            None,
            "one minute is the input resolution; folding it is a copy, and a \
             caller who asked for it has made a mistake worth surfacing"
        );
        assert_eq!(Period::minutes(2).map(Period::as_minutes), Some(2));
        assert_eq!(five().label(), "5min");
    }

    #[test]
    fn ohlc_takes_first_last_max_min_and_volume_sums() {
        // Five minutes, deliberately not monotone, so first/last cannot be
        // confused with min/max.
        let bars = [
            bar(0, 100, 110, 95, 105, 10),
            bar(1, 105, 130, 104, 120, 20),
            bar(2, 120, 121, 80, 90, 30),
            bar(3, 90, 100, 88, 99, 40),
            bar(4, 99, 101, 97, 100, 50),
        ];
        let out = resample(&bars, five());
        assert_eq!(out.len(), 1);
        let b = out.first().expect("one bar");
        assert_eq!(b.open, 100, "the FIRST open, not the lowest");
        assert_eq!(b.close, 100, "the LAST close, not the highest");
        assert_eq!(b.high, 130, "the maximum high across the bucket");
        assert_eq!(b.low, 80, "the minimum low across the bucket");
        assert_eq!(b.volume, 150, "volumes sum");
        assert_eq!(b.ts_micros, 0, "stamped at the bucket start");
    }

    #[test]
    fn buckets_are_keyed_on_the_clock_and_not_on_position() {
        // The SAME market minutes, offered starting at minute 3 instead of 0.
        // Position-based grouping would put minutes 3..7 in one bar; clock-based
        // grouping splits them at minute 5, which is where the exchange does.
        let bars = [
            bar(3, 10, 10, 10, 10, 1),
            bar(4, 20, 20, 20, 20, 1),
            bar(5, 30, 30, 30, 30, 1),
            bar(6, 40, 40, 40, 40, 1),
            bar(7, 50, 50, 50, 50, 1),
        ];
        let out = resample(&bars, five());
        assert_eq!(out.len(), 2, "the bucket boundary is at minute 5");
        let first = out.first().expect("two bars");
        let second = out.get(1).expect("two bars");
        assert_eq!(first.ts_micros, 0);
        assert_eq!(first.close, 20, "minutes 3 and 4 only");
        assert_eq!(second.ts_micros, 5 * MINUTE_MICROS);
        assert_eq!(second.open, 30, "minute 5 opens the second bucket");
    }

    #[test]
    fn an_overnight_gap_needs_no_calendar_to_split_a_bar() {
        // Two bars a day apart. Nothing here consults a session table; the gap
        // spans thousands of buckets, so arithmetic alone separates them.
        let bars = [bar(0, 10, 10, 10, 10, 1), bar(1_440, 20, 20, 20, 20, 1)];
        let out = resample(&bars, five());
        assert_eq!(
            out.len(),
            2,
            "a day's gap cannot be folded into one five-minute bar"
        );
    }

    #[test]
    fn a_bucket_is_emitted_only_once_no_later_bar_can_revise_it() {
        // Seven minutes at five-minute resolution: one full bucket and a partial.
        // The partial IS emitted -- the input ended, so nothing can revise it --
        // and its close is minute 6's, never minute 4's.
        let bars: Vec<Candle> = (0..7)
            .map(|m| bar(m, m * 10, m * 10, m * 10, m * 10, 1))
            .collect();
        let out = resample(&bars, five());
        assert_eq!(out.len(), 2);
        let full = out.first().expect("two");
        let partial = out.get(1).expect("two");
        assert_eq!(full.close, 40, "minute 4 closes the first bucket");
        assert_eq!(partial.open, 50, "minute 5 opens the second");
        assert_eq!(partial.close, 60, "and minute 6 closes it -- not minute 9");
        assert_eq!(partial.volume, 2, "two minutes, not five");
    }

    #[test]
    fn open_interest_is_the_last_known_level_and_never_a_sum() {
        let mut bars = [
            bar(0, 10, 10, 10, 10, 1),
            bar(1, 10, 10, 10, 10, 1),
            bar(2, 10, 10, 10, 10, 1),
        ];
        if let Some(b) = bars.first_mut() {
            b.open_interest = 500;
        }
        if let Some(b) = bars.get_mut(1) {
            b.open_interest = OI_NULL; // absent, and must not erase 500
        }
        if let Some(b) = bars.get_mut(2) {
            b.open_interest = 0; // a genuine zero, and must overwrite
        }
        let out = resample(&bars, five());
        assert_eq!(
            out.first().map(|b| b.open_interest),
            Some(0),
            "zero means zero -- §7 -- so a real zero replaces a real 500"
        );

        // And an absent value at the end leaves the last known one standing.
        let mut tail = bars;
        if let Some(b) = tail.get_mut(2) {
            b.open_interest = OI_NULL;
        }
        assert_eq!(
            resample(&tail, five()).first().map(|b| b.open_interest),
            Some(500),
            "an absent reading must not erase a known level"
        );
    }

    #[test]
    fn an_empty_input_produces_no_bars_and_a_single_bar_produces_one() {
        assert!(resample(&[], five()).is_empty());
        let one = resample(&[bar(7, 10, 12, 8, 11, 3)], five());
        assert_eq!(one.len(), 1);
        assert_eq!(
            one.first().map(|b| b.ts_micros),
            Some(5 * MINUTE_MICROS),
            "minute 7 belongs to the bucket starting at minute 5"
        );
    }

    #[test]
    fn a_negative_timestamp_floors_downward_rather_than_toward_zero() {
        // Anchored, not bare: a timestamp is negative on the IST grid only below
        // -19,800 s. There, `div_euclid` must floor DOWNWARD rather than toward
        // zero, or the two buckets either side of IST midnight 1970 merge.
        assert_eq!(
            bucket_of(-IST_OFFSET_MICROS, five()),
            0,
            "IST midnight on 1 Jan 1970 is bucket zero"
        );
        assert_eq!(
            bucket_of(-IST_OFFSET_MICROS - 1, five()),
            -1,
            "one microsecond earlier floors DOWN, not toward zero"
        );
        assert_eq!(
            bucket_of(-IST_OFFSET_MICROS - 5 * MINUTE_MICROS, five()),
            -1
        );
        assert_eq!(
            bucket_of(-IST_OFFSET_MICROS - 6 * MINUTE_MICROS, five()),
            -2
        );
    }

    /// The grid is IST's, and an hourly fold is where that starts to matter.
    ///
    /// A UTC-anchored grid agrees with an IST one exactly when the period
    /// divides the 330-minute offset. 5, 15 and 30 do — which is why every other
    /// fixture in this file passed while the anchor was wrong. **60 does not**,
    /// and 60 is a `Timeframe::KNOWN` store rung, so an hourly resample on the
    /// bare epoch would have disagreed with `pull::fold` on the same bars.
    #[test]
    fn the_hourly_grid_is_anchored_on_ist_and_not_on_utc() {
        let hour = Period::minutes(60).expect("sixty is a valid period");
        // 09:15 IST on day zero, expressed in UTC micros.
        let open_ist = 555 * MINUTE_MICROS - IST_OFFSET_MICROS;
        let start = bucket_start(bucket_of(open_ist, hour), hour);

        // On the IST grid an hourly bucket begins on the IST hour: 09:00 IST.
        let ist_minutes_into_day = (start + IST_OFFSET_MICROS)
            .div_euclid(MINUTE_MICROS)
            .rem_euclid(1_440);
        assert_eq!(
            ist_minutes_into_day, 540,
            "an hourly bucket containing 09:15 IST must begin at 09:00 IST \
             (540 minutes), not at an offset thirty minutes away -- which is \
             exactly what a UTC-anchored grid produces, because 330 is not a \
             multiple of 60"
        );

        // And the five-minute grid is unaffected, because 5 divides 330 -- the
        // reason this defect survived every other test in this file.
        let five_start = bucket_start(bucket_of(open_ist, five()), five());
        assert_eq!(
            (five_start + IST_OFFSET_MICROS)
                .div_euclid(MINUTE_MICROS)
                .rem_euclid(1_440),
            555,
            "09:15 IST is itself a five-minute edge"
        );
    }

    /// The saturating arms, which no market data reaches and which are therefore
    /// the arms nobody has ever seen taken.
    ///
    /// Every one of them exists because `overflow-checks = true` in release
    /// makes an unguarded add a panic, and a panic is the loud death
    /// `CLAUDE.md` §4 will not accept. An arm that guards against a panic and is
    /// never exercised is a guard nobody has tested.
    #[test]
    fn extreme_values_saturate_rather_than_panicking() {
        // A period so long its microsecond span overflows i64. u32::MAX minutes
        // is roughly 8,000 years; the multiply saturates instead of wrapping.
        let huge = Period::minutes(u32::MAX).expect("u32::MAX is above two");
        assert_eq!(
            bucket_of(i64::MAX, huge),
            i64::MAX / huge.micros().max(1),
            "a saturated period still yields a defined bucket rather than a trap"
        );

        // Volumes that would overflow when summed. Two bars in ONE bucket, each
        // carrying nearly the whole i64 range.
        let mut a = bar(0, 10, 10, 10, 10, i64::MAX);
        let mut b = bar(1, 10, 10, 10, 10, i64::MAX);
        a.open_interest = 1;
        b.open_interest = 2;
        let out = resample(&[a, b], five());
        assert_eq!(out.len(), 1, "both minutes are in the same bucket");
        assert_eq!(
            out.first().map(|c| c.volume),
            Some(i64::MAX),
            "the sum saturates at the ceiling instead of wrapping to a negative \
             volume, which is a corruption the column would then refuse"
        );
        assert_eq!(
            out.first().map(|c| c.open_interest),
            Some(2),
            "and open interest is still the last level, not a sum"
        );

        // A timestamp at the very top of the range. The bucket-start multiply
        // saturates rather than wrapping to a negative stamp, which would make
        // the output non-monotone and the column refuse every bar after it.
        let top = resample(&[bar(i64::MAX / MINUTE_MICROS, 5, 5, 5, 5, 1)], five());
        assert_eq!(top.len(), 1);
        assert!(
            top.first().is_some_and(|c| c.ts_micros > 0),
            "a saturated stamp must stay positive"
        );
    }

    #[test]
    fn folding_a_real_session_reduces_the_bar_count_by_the_period() {
        let day = crate::synthetic::sessions(1);
        let out = resample(&day, five());
        let expected = day.len().div_ceil(5);
        assert_eq!(
            out.len(),
            expected,
            "375 one-minute bars fold to 75 five-minute bars"
        );
        // Every field of the folded stream is still a valid candle.
        for b in &out {
            assert!(b.high >= b.low, "high below low at {}", b.ts_micros);
            assert!(b.open <= b.high && b.open >= b.low);
            assert!(b.close <= b.high && b.close >= b.low);
        }
    }
}
