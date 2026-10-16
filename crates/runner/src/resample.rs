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
//! `(ts_micros + anchor) / period`, floored -- so a bar's bucket is a property
//! of the bar, on the same grid `crates/pull/src/fold.rs` files into the store.
//!
//! # The grid starts at the 09:15 IST OPEN, not at IST midnight (D-1430)
//!
//! An intraday period is anchored at the NSE open, 555 minutes past IST
//! midnight, exactly as `pull::fold` anchors every intraday rung, and its grid
//! restarts at every 09:15 so a period that does not divide a day cannot drift
//! (see `bucket_start`). A period of a day or longer keeps the IST-midnight
//! anchor, again as `pull::fold` does.
//!
//! This module used to anchor every period at IST midnight, and its docs said
//! the bars began at the open. They did only when the period divides 555: for 2,
//! 10, 30 and 60 minutes the first bar of each session was stamped BEFORE the
//! open (09:14, 09:10, 09:00, 09:00) and held only part of a period, and a
//! period that does not divide a day (7, 75 minutes) drifted to a different
//! clock offset every day, as it still would on `pull::fold`'s continuous grid
//! (the store files no such rung). `runner` cannot depend on `pull`, so the rule is
//! restated here rather than imported; `crates/cli/tests/resample_matches_fold.rs`
//! holds the two to the same answer for every period that divides a day.
//!
//! What anchoring at the open leaves is a SHORT LAST bar when the period does
//! not divide the session: 375 minutes at 60 ends with 15:15-15:30. That bar is
//! stamped correctly and holds only the trades in it, which is the same
//! trailing-stub statement `pull::fold` makes.
//!
//! # Sessions need no special case, and that is the point
//!
//! An overnight gap spans thousands of buckets, so the last bar of one day and
//! the first of the next land in different buckets by arithmetic alone. No
//! calendar, no session table, no exchange rule — which means nothing here can
//! disagree with `crates/pull`'s calendar, because it does not consult one.
//!
//! # The `i64` edges refuse; they do not saturate (D-1431)
//!
//! A timestamp whose anchored key or bucket start does not fit in `i64`, and a
//! bucket whose summed volume does not, are refused by name as
//! [`ResampleError`]. These arms used to saturate, which stamped a bar outside
//! the bucket it was folded into and clamped a volume to `i64::MAX` as if it had
//! been measured — a fallback that hides a failure, which `CLAUDE.md` §4 bans.

use indicators::{Candle, IST_OFFSET_MICROS, OI_NULL};

/// Microseconds in one minute — the resolution every stored bar is at.
const MINUTE_MICROS: i64 = 60_000_000;

/// Minutes from IST midnight to the NSE open, 09:15.
///
/// The same value as `store::path::Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT`,
/// which `pull::fold` anchors on. `runner` may not name `store`, so the number
/// is restated, and the cross-crate test named in the module doc is what stops
/// the two drifting.
const OPEN_MINUTES_PAST_IST_MIDNIGHT: i64 = 9 * 60 + 15;

/// Minutes in a day: a period this wide or wider is a daily-class rung, and
/// keeps the IST-midnight anchor exactly as `pull::fold` does for any bucket of
/// 86,400 seconds or more.
const DAY_MINUTES: u32 = 24 * 60;

/// Microseconds in one day, the cycle an intraday grid restarts on.
const DAY_MICROS: i64 = 1_440 * MINUTE_MICROS;

/// The anchor of a daily-class grid: IST midnight.
const MIDNIGHT_ANCHOR_MICROS: i64 = IST_OFFSET_MICROS;

/// The anchor of an intraday grid: the 09:15 IST open.
///
/// MINUS, as in `pull::fold`: an edge is where `t + A ≡ 0 (mod w)`, IST
/// midnight is at `t ≡ -19,800 s`, and the open is 33,300 s later, so the anchor
/// moves the same distance the other way. The result, −13,500 s, is negative,
/// and `rem_euclid` handles it correctly.
const OPEN_ANCHOR_MICROS: i64 =
    MIDNIGHT_ANCHOR_MICROS - OPEN_MINUTES_PAST_IST_MIDNIGHT * MINUTE_MICROS;

/// The largest period's span fits in `i64` with room to spare, so the period
/// multiply needs no guard at all: `u32::MAX` minutes is about 2.6 × 10^17 µs.
const _: () = assert!((u32::MAX as i64).checked_mul(MINUTE_MICROS).is_some());

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

    /// The period in microseconds. Exact: the `const` assertion above proves
    /// no `u32` minute count overflows it.
    const fn micros(self) -> i64 {
        (self.0 as i64) * MINUTE_MICROS
    }

    /// The grid's anchor: the open for an intraday period, IST midnight for a
    /// day or longer. The same split `pull::fold` makes.
    const fn anchor(self) -> i64 {
        if self.0 >= DAY_MINUTES {
            MIDNIGHT_ANCHOR_MICROS
        } else {
            OPEN_ANCHOR_MICROS
        }
    }
}

/// Why a resample was refused. Each names the input position and the value
/// that could not be represented; nothing is clamped in its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResampleError {
    /// The bar at `at` is so near an `i64` edge that its anchored bucket key,
    /// or its bucket's start stamp, does not fit in `i64`. Saturating instead
    /// stamped the bar outside the bucket it was folded into.
    GridOverflow {
        /// Input position of the refused bar.
        at: usize,
        /// Its timestamp, microseconds since the Unix epoch.
        ts_micros: i64,
    },
    /// Adding the bar at `at` to its bucket's volume overflows `i64`.
    /// Saturating instead reported `i64::MAX` as a measured volume.
    VolumeOverflow {
        /// Input position of the bar whose volume overflowed the sum.
        at: usize,
        /// The start stamp of the bucket whose sum overflowed.
        bucket_micros: i64,
    },
}

impl core::fmt::Display for ResampleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::GridOverflow { at, ts_micros } => write!(
                f,
                "resample refused: bar {at} at {ts_micros} µs has no representable \
                 bucket on the open-anchored grid"
            ),
            Self::VolumeOverflow { at, bucket_micros } => write!(
                f,
                "resample refused: bar {at} overflows the i64 volume of the bucket \
                 starting at {bucket_micros} µs"
            ),
        }
    }
}

impl std::error::Error for ResampleError {}

/// The UTC microsecond at which the bucket holding `ts_micros` starts, or
/// `None` when the anchored timestamp or the start does not fit in `i64`.
///
/// The start names the bucket: it is unique per bucket and ascends with it, so
/// no separate bucket index is kept.
///
/// An intraday period walks a grid that RESTARTS at every 09:15 IST: the
/// offset into the bucket is `((t + A) mod day) mod period`, with `A` the open
/// anchor. For every period that divides a day — every rung the store files —
/// restarting at each open is the same grid `pull::fold` walks continuously
/// from its anchor, edge for edge. For one that does not (7, 75 minutes) a
/// continuous grid moves to a different clock offset each day; restarting
/// keeps the first bar of every session at 09:15, and the bucket cut short is
/// the one ending at the next 09:15, overnight. A period of a day or more keeps
/// `pull::fold`'s continuous IST-midnight grid.
///
/// `rem_euclid`, never `%`: the intraday anchor is negative, so the anchored
/// value is negative for any instant before 09:15 IST on 1 Jan 1970, and a
/// truncating remainder there would stamp the bucket AFTER the bar.
///
/// # The anchor, and the incidents that name it
///
/// This was first `ts_micros.div_euclid(..)` on the bare epoch, which anchors
/// every edge at **UTC** midnight; `crates/pull/src/fold.rs` had the same bug
/// and the store held **20 records stamped on a SUNDAY**. It then moved to IST
/// midnight, which is right for a day and wrong for an intraday period that
/// does not divide 555 minutes — see the module doc and D-1430.
///
/// [`indicators::IST_OFFSET_MICROS`] is used rather than a second copy of
/// 19,800: one definition across three crates is what stops them drifting.
const fn bucket_start(ts_micros: i64, period: Period) -> Option<i64> {
    let Some(shifted) = ts_micros.checked_add(period.anchor()) else {
        return None;
    };
    let into_bucket = if period.0 >= DAY_MINUTES {
        shifted.rem_euclid(period.micros())
    } else {
        shifted.rem_euclid(DAY_MICROS).rem_euclid(period.micros())
    };
    ts_micros.checked_sub(into_bucket)
}

/// Folds one-minute `bars` into `period` bars.
///
/// Input is assumed to be in ascending timestamp order, which is what
/// `indicators::column::Column` already refuses to sweep without — a
/// non-increasing timestamp is counted as a refusal there, so a caller that
/// resamples unsorted bars has a defect this function cannot see and the column
/// will.
///
/// # Errors
///
/// [`ResampleError::GridOverflow`] for a bar whose bucket or bucket start is
/// not representable in `i64`, and [`ResampleError::VolumeOverflow`] for a
/// bucket whose summed volume is not. No market timestamp or volume reaches
/// either; they exist so that an impossible input is refused rather than
/// clamped into a plausible-looking bar.
///
/// # Cost
///
/// One pass, no allocation per bar, no lookahead buffer: the accumulator holds
/// one bucket, and each bar costs a fixed number of checked operations, O(1).
/// UNVERIFIED as a measured figure — no bench row covers this yet, and
/// `crates/runner/benches/ratio.rs` would need a `C-R-04` to claim one.
pub fn resample(bars: &[Candle], period: Period) -> Result<Vec<Candle>, ResampleError> {
    // A HINT, not a bound, and the comment here used to claim otherwise.
    //
    // It said the output "cannot be longer than the input divided by the period,
    // plus one" and then reserved no plus-one -- and the bound itself is false
    // on any multi-session slice. Buckets are keyed on the clock, so an
    // overnight gap splits a bucket WITHOUT consuming a full period of input
    // bars; this module's own `an_overnight_gap_needs_no_calendar_to_split_a_bar`
    // is a direct counterexample. Reserving the common case still avoids most of
    // the doubling growth gate 11 rule 3 is about, and a `Vec` that grows past a
    // hint is correct where a comment that lies is not.
    // `Period` refuses anything below two, so the divisor is never zero and
    // `div_ceil` cannot panic.
    let per = period.as_minutes() as usize;
    let mut out: Vec<Candle> = Vec::with_capacity(bars.len().div_ceil(per));
    let mut open: Option<(i64, Candle)> = None;

    for (at, bar) in bars.iter().enumerate() {
        let grid_overflow = ResampleError::GridOverflow {
            at,
            ts_micros: bar.ts_micros,
        };
        let key = bucket_start(bar.ts_micros, period).ok_or(grid_overflow)?;
        match open {
            // A bar in the bucket being accumulated.
            Some((k, ref mut acc)) if k == key => {
                acc.high = acc.high.max(bar.high);
                acc.low = acc.low.min(bar.low);
                acc.close = bar.close;
                acc.volume =
                    acc.volume
                        .checked_add(bar.volume)
                        .ok_or(ResampleError::VolumeOverflow {
                            at,
                            bucket_micros: acc.ts_micros,
                        })?;
                acc.open_interest = fold_open_interest(acc.open_interest, bar.open_interest);
            }
            // A bar in a LATER bucket. The one being accumulated is now provably
            // complete -- nothing after this point can belong to it, because the
            // input ascends -- so it is emitted, and this bar opens the next.
            Some((_, acc)) => {
                out.push(acc);
                open = Some((key, opened(bar, key)));
            }
            None => open = Some((key, opened(bar, key))),
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
    Ok(out)
}

/// A fresh accumulator opened at `bar`, timestamped to its bucket's `start`.
///
/// The stamp is the bucket's start, not the first bar's own time. A 5-minute bar
/// built from 09:17..09:19 is stamped 09:15 — otherwise two runs over data that
/// began at different minutes would produce bars with different timestamps from
/// the same market, and `data_digest` would disagree across them.
const fn opened(bar: &Candle, start: i64) -> Candle {
    Candle {
        ts_micros: start,
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
    use super::{MINUTE_MICROS, OPEN_ANCHOR_MICROS, Period, ResampleError, bucket_start, resample};
    use indicators::{Candle, IST_OFFSET_MICROS, OI_NULL};

    /// A one-minute bar at `minute` past the epoch.
    fn bar(minute: i64, open: i64, high: i64, low: i64, close: i64, volume: i64) -> Candle {
        Candle {
            ts_micros: minute * MINUTE_MICROS,
            open,
            high,
            low,
            close,
            volume,
            open_interest: OI_NULL,
        }
    }

    /// A flat one-minute bar at `ist_minute` past IST midnight of IST `day`.
    fn ist(day: i64, ist_minute: i64, volume: i64) -> Candle {
        Candle {
            ts_micros: (day * 1_440 + ist_minute) * MINUTE_MICROS - IST_OFFSET_MICROS,
            open: 10,
            high: 10,
            low: 10,
            close: 10,
            volume,
            open_interest: OI_NULL,
        }
    }

    /// Minutes past IST midnight of `ts_micros`.
    fn ist_minute_of(ts_micros: i64) -> i64 {
        (ts_micros + IST_OFFSET_MICROS)
            .div_euclid(MINUTE_MICROS)
            .rem_euclid(1_440)
    }

    fn period(minutes: u32) -> Period {
        Period::minutes(minutes).expect("at least two minutes")
    }

    fn five() -> Period {
        period(5)
    }

    fn run(bars: &[Candle], p: Period) -> Vec<Candle> {
        resample(bars, p).expect("ordinary market values never overflow")
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

    /// D-1194: the first bar of EVERY session starts at 09:15 and covers whole
    /// periods from the open, for every intraday period that divides the day,
    /// on two consecutive days. Under the IST-midnight anchor a 60-minute first
    /// bar was stamped 09:00. (D-1194 also refused a period that does not
    /// divide the day; the merged tree keeps D-1430's per-session restart
    /// instead, under which such a period opens at 09:15 every day too.)
    #[test]
    fn every_session_opens_with_a_bar_stamped_at_the_open() {
        let open = |day: i64| (day * 1_440 + 555) * MINUTE_MICROS - IST_OFFSET_MICROS;
        for minutes in [
            2_u32, 3, 4, 5, 6, 8, 10, 15, 20, 30, 45, 60, 90, 120, 180, 360, 720,
        ] {
            let p = period(minutes);
            for day in [19_723_i64, 19_724] {
                let session: Vec<Candle> = (0..375)
                    .map(|m| Candle {
                        ts_micros: open(day) + m * MINUTE_MICROS,
                        ..bar(0, 10, 10, 10, 10, 1)
                    })
                    .collect();
                let out = run(&session, p);
                assert_eq!(
                    out.first().map(|c| c.ts_micros),
                    Some(open(day)),
                    "{minutes}-minute first bar on day {day}"
                );
                let per = i64::from(minutes);
                assert_eq!(
                    out.first().map(|c| c.volume),
                    Some(per.min(375)),
                    "{minutes}-minute first bar holds a whole period"
                );
                assert_eq!(out.len(), 375_usize.div_ceil(minutes as usize));
            }
        }
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
        let out = run(&bars, five());
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
        let out = run(&bars, five());
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
        let out = run(&bars, five());
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
        let out = run(&bars, five());
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
        let out = run(&bars, five());
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
            run(&tail, five()).first().map(|b| b.open_interest),
            Some(500),
            "an absent reading must not erase a known level"
        );
    }

    #[test]
    fn an_empty_input_produces_no_bars_and_a_single_bar_produces_one() {
        assert_eq!(resample(&[], five()), Ok(Vec::new()));
        let one = run(&[bar(7, 10, 12, 8, 11, 3)], five());
        assert_eq!(one.len(), 1);
        assert_eq!(
            one.first().map(|b| b.ts_micros),
            Some(5 * MINUTE_MICROS),
            "minute 7 belongs to the bucket starting at minute 5"
        );
    }

    #[test]
    fn a_negative_anchored_value_floors_downward_rather_than_toward_zero() {
        // 09:15 IST on 1 Jan 1970 is `-OPEN_ANCHOR_MICROS` in UTC micros, where
        // the anchored value is zero. One microsecond earlier it is negative,
        // and `rem_euclid` must place the bar in the bucket that started at
        // 09:10, not in one starting after the bar.
        let zero = -OPEN_ANCHOR_MICROS;
        assert_eq!(bucket_start(zero, five()), Some(zero));
        assert_eq!(
            bucket_start(zero - 1, five()),
            Some(zero - 5 * MINUTE_MICROS)
        );
        assert_eq!(
            bucket_start(zero - 5 * MINUTE_MICROS, five()),
            Some(zero - 5 * MINUTE_MICROS)
        );
        assert_eq!(
            bucket_start(zero - 6 * MINUTE_MICROS, five()),
            Some(zero - 10 * MINUTE_MICROS)
        );
        // And on the 1,440-minute grid, IST midnight 1970 is its own edge.
        assert_eq!(
            bucket_start(-IST_OFFSET_MICROS - 1, period(1_440)),
            Some(-IST_OFFSET_MICROS - 1_440 * MINUTE_MICROS)
        );
    }

    /// D-1430. Every intraday bucket of a session starts at the 09:15 open and
    /// then every `period` minutes after it — the rule `pull::fold` files the
    /// store's rungs on. On a midnight-anchored grid 2, 10, 30 and 60 opened the
    /// day with a bar stamped 09:14, 09:10, 09:00 and 09:00.
    #[test]
    fn every_intraday_period_opens_the_session_at_09_15() {
        let bars: Vec<Candle> = (555..=630).map(|m| ist(20_000, m, 1)).collect();
        for (minutes, starts) in [
            (2_u32, (555..=630).step_by(2).collect::<Vec<i64>>()),
            (3, (555..=630).step_by(3).collect()),
            (5, (555..=630).step_by(5).collect()),
            (10, vec![555, 565, 575, 585, 595, 605, 615, 625]),
            (15, vec![555, 570, 585, 600, 615, 630]),
            (30, vec![555, 585, 615]),
            (60, vec![555, 615]),
            (75, vec![555, 630]),
        ] {
            let out = run(&bars, period(minutes));
            let got: Vec<i64> = out.iter().map(|c| ist_minute_of(c.ts_micros)).collect();
            assert_eq!(got, starts, "{minutes}min bucket starts, IST minutes");
            let volume: i64 = out.iter().map(|c| c.volume).sum();
            assert_eq!(volume, 76, "{minutes}min: every minute lands somewhere");
            assert_eq!(
                out.first().map(|c| c.volume),
                Some(i64::from(minutes.min(76))),
                "{minutes}min: the first bar is a WHOLE period, not a stub"
            );
        }
    }

    /// The hourly case, named: midnight-anchored it starts 09:00, open-anchored
    /// it starts 09:15, and that is the whole defect D-1430 fixes.
    #[test]
    fn the_hourly_grid_starts_at_the_open_and_not_on_the_ist_hour() {
        let hour = period(60);
        let open = ist(0, 555, 1).ts_micros;
        assert_eq!(
            bucket_start(open, hour),
            Some(open),
            "09:15 IST is itself an hourly edge"
        );

        // A full regular session at sixty minutes: six whole hours and a
        // fifteen-minute stub at 15:15, the period not dividing 375.
        let day: Vec<Candle> = (555..930).map(|m| ist(0, m, 1)).collect();
        let out = run(&day, hour);
        let starts: Vec<i64> = out.iter().map(|c| ist_minute_of(c.ts_micros)).collect();
        assert_eq!(starts, vec![555, 615, 675, 735, 795, 855, 915]);
        let volumes: Vec<i64> = out.iter().map(|c| c.volume).collect();
        assert_eq!(volumes, vec![60, 60, 60, 60, 60, 60, 15], "short LAST bar");
    }

    /// A period that does not divide a day stays on the open every day; on a
    /// midnight grid a seven-minute bucket drifted by 1,440 mod 7 = 5 minutes a
    /// day.
    #[test]
    fn a_period_that_does_not_divide_a_day_does_not_drift_between_days() {
        let seven = period(7);
        for day in 0..14 {
            let out = run(&[ist(day, 555, 1), ist(day, 560, 1)], seven);
            assert_eq!(
                out.iter()
                    .map(|c| ist_minute_of(c.ts_micros))
                    .collect::<Vec<_>>(),
                vec![555],
                "day {day}: 09:15 and 09:20 share the bucket opened at 09:15"
            );
        }
    }

    /// Day boundary, and bars before the open: the grid extends backwards from
    /// 09:15 by whole periods, the same arithmetic `pull::fold` uses, and the
    /// last bar of one day never shares a bucket with the next day's first.
    #[test]
    fn pre_open_bars_and_the_day_boundary_stay_on_the_open_grid() {
        let thirty = period(30);
        let bars = [
            ist(3, 540, 1), // 09:00, pre-open
            ist(3, 554, 1), // 09:14
            ist(3, 555, 1), // 09:15, the open
            ist(3, 929, 1), // 15:29, the last minute
            ist(4, 555, 1), // next day's open
        ];
        let out = run(&bars, thirty);
        let stamps: Vec<(i64, i64)> = out
            .iter()
            .map(|c| (indicators::ist_day(c.ts_micros), ist_minute_of(c.ts_micros)))
            .collect();
        assert_eq!(
            stamps,
            vec![(3, 525), (3, 555), (3, 915), (4, 555)],
            "09:00 and 09:14 fold into 08:45; 09:15 opens a new bucket; \
             15:29 sits in 15:15; the next day opens at 09:15 again"
        );
        assert_eq!(out.first().map(|c| c.volume), Some(2));
    }

    /// A period of a day or more keeps the IST-midnight anchor, as `pull::fold`
    /// does: an open-anchored day would span two calendar dates.
    #[test]
    fn a_daily_period_keeps_the_ist_midnight_anchor() {
        let day = period(1_440);
        let out = run(&[ist(7, 555, 1), ist(7, 929, 1), ist(8, 555, 1)], day);
        let stamps: Vec<i64> = out.iter().map(|c| c.ts_micros).collect();
        assert_eq!(stamps, vec![ist(7, 0, 0).ts_micros, ist(8, 0, 0).ts_micros]);
    }

    /// D-1431. Summed volume past `i64::MAX` is refused by name; it used to
    /// saturate and report `i64::MAX` as if measured.
    #[test]
    fn a_volume_sum_past_i64_max_is_refused_and_not_clamped() {
        let a = ist(1, 555, i64::MAX);
        let b = ist(1, 556, 1);
        assert_eq!(
            resample(&[a, b], five()),
            Err(ResampleError::VolumeOverflow {
                at: 1,
                bucket_micros: a.ts_micros,
            })
        );
        // Exactly the ceiling is a representable sum, and is kept.
        let edge = run(&[ist(1, 555, i64::MAX - 1), b], five());
        assert_eq!(edge.first().map(|c| c.volume), Some(i64::MAX));
        // A bar in a LATER bucket starts a fresh sum, so no refusal there.
        let split = run(&[a, ist(1, 560, i64::MAX)], five());
        assert_eq!(split.len(), 2);
    }

    /// D-1431. A timestamp whose anchored key or bucket start is not an `i64`
    /// is refused by name; it used to saturate and stamp the bar outside the
    /// bucket it was folded into.
    #[test]
    fn a_timestamp_at_the_i64_edge_is_refused_and_not_saturated() {
        let at = |ts_micros: i64| Candle {
            ts_micros,
            ..ist(0, 555, 1)
        };
        // The anchored key overflows: the intraday anchor is negative.
        assert_eq!(
            resample(&[at(i64::MIN)], five()),
            Err(ResampleError::GridOverflow {
                at: 0,
                ts_micros: i64::MIN,
            })
        );
        // The anchored value fits but the start does not: on the daily grid the
        // anchor is positive, so `i64::MIN` shifts in range while its bucket
        // starts before `i64::MIN`.
        let into = (i128::from(i64::MIN) + i128::from(IST_OFFSET_MICROS))
            .rem_euclid(1_440 * i128::from(MINUTE_MICROS));
        assert!(into > 0, "the fixture must start below i64::MIN");
        assert_eq!(
            resample(&[at(i64::MIN)], period(1_440)),
            Err(ResampleError::GridOverflow {
                at: 0,
                ts_micros: i64::MIN,
            })
        );
        // Against an i128 model near the bottom of the intraday range: a bar is
        // refused exactly when its anchored value or its start leaves `i64`, and
        // otherwise stamped where the model says.
        let mut refused_on_start = 0;
        for minutes in [2_u32, 7, 60, 1_439] {
            let p = period(minutes);
            for step in 0..64_i64 {
                let ts = i64::MIN - OPEN_ANCHOR_MICROS - 1 + step * 1_000_003_007;
                let shifted = i128::from(ts) + i128::from(OPEN_ANCHOR_MICROS);
                let model = (shifted >= i128::from(i64::MIN)).then(|| {
                    i128::from(ts)
                        - shifted
                            .rem_euclid(1_440 * i128::from(MINUTE_MICROS))
                            .rem_euclid(i128::from(minutes) * i128::from(MINUTE_MICROS))
                });
                let expected = model.and_then(|start| i64::try_from(start).ok());
                if model.is_some() && expected.is_none() {
                    refused_on_start += 1;
                }
                assert_eq!(bucket_start(ts, p), expected, "{minutes}min step {step}");
            }
        }
        assert!(
            refused_on_start > 0,
            "the start-overflow arm must be reached"
        );
        // The daily anchor is positive, so it overflows at the other edge, and
        // the refusal names the position of the offending bar, not the first.
        assert_eq!(
            resample(&[ist(1, 555, 1), at(i64::MAX)], period(1_440)),
            Err(ResampleError::GridOverflow {
                at: 1,
                ts_micros: i64::MAX,
            })
        );
        // And i64::MAX on the intraday grid is representable: its bucket is
        // stamped at or before it and within one period, never clamped.
        let top = run(&[at(i64::MAX)], five());
        let stamp = top.first().map(|c| c.ts_micros).expect("one bar");
        assert!(i64::MAX - stamp < 5 * MINUTE_MICROS, "stamped {stamp}");

        assert!(
            ResampleError::GridOverflow {
                at: 3,
                ts_micros: 9
            }
            .to_string()
            .contains("bar 3 at 9")
        );
        assert!(
            ResampleError::VolumeOverflow {
                at: 4,
                bucket_micros: 8
            }
            .to_string()
            .contains("bar 4 overflows")
        );
    }

    /// Same input, same output, byte for byte; and a slice that starts mid-way
    /// through a bucket yields the same later bars as the whole session.
    #[test]
    fn a_rerun_is_identical_and_a_later_start_changes_no_later_bar() {
        let day = crate::synthetic::sessions(2);
        for minutes in [2_u32, 7, 10, 30, 60, 75] {
            let first = run(&day, period(minutes));
            assert_eq!(first, run(&day, period(minutes)), "{minutes}min rerun");
            let cut = run(day.get(1..).expect("a session"), period(minutes));
            assert_eq!(
                first.get(1..),
                cut.get(1..),
                "{minutes}min: dropping 09:15 only changes the bucket it was in"
            );
        }
    }

    #[test]
    fn folding_a_real_session_reduces_the_bar_count_by_the_period() {
        let day = crate::synthetic::sessions(1);
        let out = run(&day, five());
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
