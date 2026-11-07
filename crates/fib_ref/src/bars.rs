//! The two feeds as this crate reads them, and the fold of 1-minute candles
//! into the timeframes 1, 2, 3, 5, 10, 15, 30 and 60 minutes.
//!
//! Decisions read Zerodha 1-minute candles; fills read GDFL 1-second
//! trade bars (PREVIEW-SPEC 3.3). A minute carries what the session-level
//! rules need: whether Zerodha holds it, whether it is vendor-padded, and the
//! last GDFL trade inside it (R1's `G`, PREVIEW-SPEC 6.1).
//!
//! **Hole minute** (PREVIEW-SPEC 3.4, DV-29): Zerodha does not hold it, or it
//! is padded and GDFL printed a trade in it. A padded minute with no GDFL
//! trade is a minute nobody traded and stays held. A candle holding a hole
//! minute is incomplete: no decision on it.

use crate::clock::{CANDLE_END, OPEN};

/// The timeframes a recipe may read, in minutes (PREVIEW-SPEC 4).
pub const TIMEFRAMES: [u32; 8] = [1, 2, 3, 5, 10, 15, 30, 60];

/// One GDFL second: trades only. A second with `v == 0` printed nothing
/// tradable and fills nothing (a halt or an empty second).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sec {
    /// Second of the day.
    pub t: u32,
    /// First print, paise.
    pub o: i64,
    /// Highest print, paise.
    pub h: i64,
    /// Lowest print, paise.
    pub l: i64,
    /// Last print, paise.
    pub c: i64,
    /// Shares traded in the second.
    pub v: i64,
}

impl Sec {
    /// A second with traded volume.
    pub fn traded(&self) -> bool {
        self.v > 0
    }
}

/// One Zerodha 1-minute candle with its day facts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Minute {
    /// Session minute, 0 = 09:15.
    pub m: u32,
    /// Open, paise.
    pub o: i64,
    /// High, paise.
    pub h: i64,
    /// Low, paise.
    pub l: i64,
    /// Close, paise.
    pub c: i64,
    /// Zerodha volume.
    pub v: i64,
    /// Zerodha holds this minute.
    pub held: bool,
    /// Vendor-padded (volume 0, OHLC the close before).
    pub padded: bool,
    /// The last GDFL trade inside this minute, if GDFL printed one.
    pub g: Option<i64>,
}

impl Minute {
    /// A hole minute (see the module doc).
    pub fn hole(&self) -> bool {
        !self.held || (self.padded && self.g.is_some())
    }

    /// R1's `(Z, G)` read at this minute: its close and its last GDFL trade,
    /// or `None` when the minute has no price of its own (a hole, a padded
    /// minute, or no GDFL trade), PREVIEW-SPEC 6.1.
    pub fn r1(&self) -> Option<(i64, i64)> {
        if !self.held || self.padded {
            return None;
        }
        self.g.map(|g| (self.c, g))
    }
}

/// A candle of a recipe's timeframe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Candle {
    /// Index of the candle in its day, 0 = the one starting 09:15.
    pub i: u32,
    /// First session minute it covers.
    pub first_minute: u32,
    /// Last session minute it covers.
    pub last_minute: u32,
    /// The second it closes (and is first known), `min(t + tf, 15:30:00)`.
    pub close_t: u32,
    /// Open, paise (first held minute's open).
    pub o: i64,
    /// High, paise (over held minutes).
    pub h: i64,
    /// Low, paise (over held minutes).
    pub l: i64,
    /// Close, paise (last held minute's close).
    pub c: i64,
    /// Zerodha volume.
    pub v: i64,
    /// Holds a hole minute: no decision on it.
    pub incomplete: bool,
}

impl Candle {
    /// Green: close above open.
    pub fn green(&self) -> bool {
        self.c > self.o
    }

    /// Red: close below open.
    pub fn red(&self) -> bool {
        self.c < self.o
    }

    /// The price mirror: every price negated, high and low swapped. A short's
    /// rule on a candle is the long's rule on its mirror.
    #[must_use]
    pub fn mirror(&self) -> Candle {
        Candle {
            o: -self.o,
            h: -self.l,
            l: -self.h,
            c: -self.c,
            ..*self
        }
    }
}

impl Minute {
    /// The price mirror of a minute (see [`Candle::mirror`]).
    #[must_use]
    pub fn mirror(&self) -> Minute {
        Minute {
            o: -self.o,
            h: -self.l,
            l: -self.h,
            c: -self.c,
            g: self.g.map(|g| -g),
            ..*self
        }
    }
}

impl Sec {
    /// The price mirror of a second (see [`Candle::mirror`]).
    #[must_use]
    pub fn mirror(&self) -> Sec {
        Sec {
            o: -self.o,
            h: -self.l,
            l: -self.h,
            c: -self.c,
            ..*self
        }
    }
}

/// Folds a day's minutes (sorted by `m`, at most one per minute, covering
/// `0..n` without a gap in the list itself: a missing Zerodha minute is a
/// `Minute` with `held == false`) into candles of `tf` minutes. Only candles
/// whose every minute is in the list are formed: the list's end is the data's
/// end.
///
/// A candle with no held minute at all carries the previous candle's close as
/// a flat price (it is incomplete, so no decision reads it).
pub fn fold(minutes: &[Minute], tf: u32) -> Vec<Candle> {
    fold_on(minutes, tf, crate::model::CandleGrid::Session)
}

/// As [`fold`] on a chosen label grid (A04): `Session` labels candles from
/// 09:15; `Clock` labels them at multiples of `tf` minutes from midnight,
/// so the first candle runs from 09:15 to the first multiple after it.
pub fn fold_on(minutes: &[Minute], tf: u32, grid: crate::model::CandleGrid) -> Vec<Candle> {
    let tf = tf.max(1);
    // the first candle's length in minutes
    let first_len = match grid {
        crate::model::CandleGrid::Session => tf,
        crate::model::CandleGrid::Clock => tf - (OPEN / 60) % tf,
    };
    let n = u32::try_from(minutes.len()).unwrap_or(u32::MAX);
    let mut out = Vec::new();
    let mut prev_close: Option<i64> = None;
    let mut first = 0u32;
    let mut index = 0u32;
    while first < n {
        let len = if index == 0 { first_len } else { tf };
        let last = (first + len - 1).min(374);
        if last >= n {
            break;
        }
        let mut candle = Candle {
            i: index,
            first_minute: first,
            last_minute: last,
            close_t: (OPEN + 60 * (last + 1)).min(CANDLE_END),
            o: 0,
            h: i64::MIN,
            l: i64::MAX,
            c: 0,
            v: 0,
            incomplete: false,
        };
        let mut any = false;
        for minute in &minutes[first as usize..=last as usize] {
            if minute.hole() {
                candle.incomplete = true;
            }
            if !minute.held {
                continue;
            }
            if !any {
                candle.o = minute.o;
                any = true;
            }
            candle.h = candle.h.max(minute.h);
            candle.l = candle.l.min(minute.l);
            candle.c = minute.c;
            candle.v += minute.v;
        }
        if !any {
            let flat = prev_close.unwrap_or(0);
            candle.o = flat;
            candle.h = flat;
            candle.l = flat;
            candle.c = flat;
        }
        prev_close = Some(candle.c);
        out.push(candle);
        first = last + 1;
        index += 1;
    }
    out
}

/// The minute index of the last 1-minute candle closed by second `t`
/// (a minute closes at its end), or `None` before 09:16:00.
pub fn last_minute_closed_by(t: u32) -> Option<u32> {
    if t < OPEN + 60 {
        return None;
    }
    Some(((t.min(CANDLE_END) - OPEN) / 60) - 1)
}

/// Index of the first second in `secs` (sorted by `t`) at or after `t`.
pub fn first_at_or_after(secs: &[Sec], t: u32) -> usize {
    secs.partition_point(|s| s.t < t)
}

/// The first traded second at or after `t`, if any.
pub fn first_traded_at_or_after(secs: &[Sec], t: u32) -> Option<Sec> {
    secs[first_at_or_after(secs, t)..]
        .iter()
        .copied()
        .find(Sec::traded)
}

/// The last traded print strictly before `t`: the sizing price and the
/// protection reference (PREVIEW-SPEC 10.2, 11.1).
pub fn last_print_before(secs: &[Sec], t: u32) -> Option<i64> {
    secs[..first_at_or_after(secs, t)]
        .iter()
        .rev()
        .find(|s| s.traded())
        .map(|s| s.c)
}

#[cfg(test)]
#[path = "bars_tests.rs"]
mod tests;
