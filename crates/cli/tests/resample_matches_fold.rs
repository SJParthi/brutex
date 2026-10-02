//! `runner::resample` and `pull::fold` give the same bars for the same minutes.
//!
//! `runner` may not depend on `pull`, so `runner::resample` restates the grid
//! rule `pull::fold` files the store's rungs on rather than importing it:
//! intraday buckets anchored at the 09:15 IST open, a day or longer at IST
//! midnight (D-1430). `cli` depends on both, which makes it the one place the
//! two can be held to one answer. Before D-1430 the resampler anchored every
//! period at IST midnight, and for 2, 10, 30 and 60 minutes its first bar of
//! each session was stamped before the open where `pull::fold`'s was at 09:15.

#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]

use indicators::{Candle, IST_OFFSET_MICROS, OI_NULL};
use pull::fold::{Bucket, fold};
use runner::resample::{Period, resample};
use store::format::Bar;

const MINUTE_MICROS: i64 = 60_000_000;

/// A one-minute candle at `ist_minute` past IST midnight of IST `day`, with
/// prices that differ minute to minute so OHLC is not trivially equal.
fn minute(day: i64, ist_minute: i64) -> Candle {
    let price = 2_000_000 + (ist_minute * 37 + day * 11) % 500;
    Candle {
        ts_micros: (day * 1_440 + ist_minute) * MINUTE_MICROS - IST_OFFSET_MICROS,
        open: price,
        high: price + 40,
        low: price - 30,
        close: price + 5,
        volume: ist_minute % 9 + 1,
        open_interest: if ist_minute % 4 == 0 {
            OI_NULL
        } else {
            ist_minute
        },
    }
}

const fn as_bar(c: &Candle) -> Bar {
    Bar {
        ts_micros: c.ts_micros,
        open: c.open,
        high: c.high,
        low: c.low,
        close: c.close,
        volume: c.volume,
        open_interest: c.open_interest,
    }
}

/// Two regular sessions with a pre-open minute, a hole, and two night minutes
/// either side of IST midnight.
fn minutes() -> Vec<Candle> {
    let mut out = vec![minute(19_500, 540), minute(19_500, 554)];
    out.extend((555..930).filter(|m| *m != 700).map(|m| minute(19_500, m)));
    // Either side of IST midnight: a 60-minute bucket runs 23:15-00:15, and
    // the resampler's grid restarting at 09:15 must still merge the two.
    out.extend([minute(19_500, 1_430), minute(19_501, 5)]);
    out.extend((555..930).map(|m| minute(19_501, m)));
    out
}

#[test]
fn runner_resample_and_pull_fold_agree_on_every_period() {
    let candles = minutes();
    let bars: Vec<Bar> = candles.iter().map(as_bar).collect();
    for minutes in [2_u32, 3, 5, 10, 15, 30, 60, 1_440] {
        let period = Period::minutes(minutes).expect("at least two minutes");
        let bucket = Bucket::of_secs(minutes * 60).expect("non-zero");
        let resampled: Vec<Bar> = resample(&candles, period)
            .expect("market values resample")
            .iter()
            .map(as_bar)
            .collect();
        let folded = fold(&bars, bucket).expect("ordered minutes fold");
        assert_eq!(resampled, folded, "{minutes}min");
    }
}

#[test]
fn the_first_intraday_bar_of_a_session_is_stamped_at_the_open() {
    let open = minute(19_500, 555).ts_micros;
    let session: Vec<Candle> = (555..=630).map(|m| minute(19_500, m)).collect();
    for minutes in [2_u32, 10, 30, 60] {
        let period = Period::minutes(minutes).expect("at least two minutes");
        let first = resample(&session, period)
            .expect("market values resample")
            .first()
            .map(|c| c.ts_micros);
        assert_eq!(first, Some(open), "{minutes}min must open at 09:15 IST");
    }
}
