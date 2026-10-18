//! Folding many snapshots into one bar, and the hard-gated ladder that decides
//! what may be pulled next.
//!
//! # Why folding must exist
//!
//! Both archive vendors ship **one-second snapshots**: two to four rows per
//! second, no sub-second field, no tiebreaker. Handed to the store as-is they
//! are refused, and correctly — a format addressed by
//! `base + header + index·stride` cannot hold two records claiming the same
//! instant. A real run against 194 contracts produced 354,675 rows and **zero
//! bars**, every member refused with *"does not follow"*.
//!
//! That refusal is the feature. What was missing is the step that makes a
//! one-second feed storable as one-minute bars at all: **fold every snapshot
//! inside a bucket into one OHLCV bar.**
//!
//! | Field | From the bucket's snapshots |
//! |---|---|
//! | open | the **first** |
//! | high | the **maximum** |
//! | low | the **minimum** |
//! | close | the **last** |
//! | volume | the **sum** |
//! | open interest | the **last** that carried one |
//!
//! First and last are **file order**, never sorted. Rows sharing a second have
//! no recoverable order, so a sort would invent one and quietly change which
//! price became the open.
//!
//! # The ladder
//!
//! The operator's rule, and it holds for every vendor without exception:
//!
//! | | Segment | | Granularity |
//! |---|---|---|---|
//! | 1 | **Spot** | 1 | **Daily** |
//! | 2 | Futures, expired | 2 | One minute |
//! | 3 | Options, expired | | |
//!
//! Nothing advances until the stage before it finished **completely clean** —
//! zero failures. [`Ladder::next`] is the only thing that says what may run,
//! and it refuses to skip.
//!
//! The order is not merely cautious, it is the cheapest place to fail. Spot is
//! two instruments; futures is ~213 underlyings; options is ~11,500 contracts
//! in a single day. Daily is one bar where minute is 375. Each stage is a
//! rehearsal for the next and costs a fraction of it, so a broken credential, a
//! wrong path or a bad session bound surfaces against two instruments rather
//! than against eleven thousand half-written files.

use store::format::Bar;

/// How wide a fold bucket is, in seconds.
///
/// Not a [`store::path::Timeframe`] because folding happens before a path
/// exists, and taking the width as a plain number keeps this module unable to
/// decide where anything is filed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bucket(u32);

impl Bucket {
    /// ONE SECOND — the width at which a bought file's duplicate timestamps
    /// are resolved.
    ///
    /// # What the store does with two rows that share a second
    ///
    /// The operator's rule, 12 Aug 2026: *"for truedata or gdfl also we will
    /// have multiple ticks but anyhow always the timestamp will be mapped as
    /// second."* Measured in `docs/08-vendor-samples.md`: three rows in one
    /// second at `TrueData`, four at GDFL, no sub-second field at either. So a
    /// shared second is EXPECTED INPUT, and refusing it would refuse every
    /// file the operator bought.
    ///
    /// Folded at this width, the second's rows become one record:
    ///
    /// | Field | From the rows sharing the second |
    /// |---|---|
    /// | open | the **first** in file order |
    /// | high / low | the extremes across all of them |
    /// | close | the **last** in file order — LAST WINS |
    /// | volume | the **sum** |
    /// | open interest | the **last** that carried one |
    ///
    /// **Last wins for the price**, which is the single value a consumer reads
    /// as "what it was at that second", and the earlier rows are not discarded
    /// — they are the second's open, its extremes and its volume. Nothing is
    /// dropped and nothing is refused. First-wins would throw away the newest
    /// information in the second; refusing would throw away the file.
    ///
    /// File order is the only order there is: rows sharing a second carry no
    /// tiebreaker, so a sort would invent one and quietly change which price
    /// became the open. `crate::archive::read_dir` orders MEMBERS and never
    /// the rows inside one, for exactly this reason.
    ///
    /// Proven by `pull::folder::a_shared_second_folds_last_price_wins`.
    pub const SECOND: Self = Self(1);
    /// One minute.
    pub const MINUTE: Self = Self(60);
    /// One trading day. Wide enough that a whole session lands in one bucket.
    pub const DAY: Self = Self(86_400);

    /// A bucket of `secs` seconds, or [`None`] for zero or for a width wider
    /// than one day.
    ///
    /// Zero is refused rather than clamped: a zero-width bucket would divide by
    /// zero, and a bucket silently widened to one second is a different answer
    /// to the question that was asked.
    ///
    /// **Wider than [`Self::DAY`] is refused too** (audit-20261003
    /// attackdata-5, D-1532). Such a bucket folds two trading days into one
    /// bar, no store rung is wider than a day, and a `u32::MAX` width stamped a
    /// 2024 snapshot at 1969-12-31. Every width from one second to one day is
    /// still accepted, which is what folding from snapshots promises.
    #[must_use]
    pub const fn of_secs(secs: u32) -> Option<Self> {
        if secs == 0 || secs > Self::DAY.0 {
            None
        } else {
            Some(Self(secs))
        }
    }

    /// The width in seconds.
    #[must_use]
    pub const fn secs(self) -> u32 {
        self.0
    }
}

/// Folds bars into buckets, one output bar per bucket that held anything.
///
/// Input must be in **non-decreasing** timestamp order, which is what every
/// vendor file is and what [`crate::archive`] preserves by never sorting rows.
/// An out-of-order row would open a bucket that was already closed; rather than
/// silently producing two bars for one minute, that is refused.
///
/// # Errors
///
/// [`FoldError::OutOfOrder`] naming the position and both timestamps.
///
/// # Examples
///
/// ```
/// # use pull::fold::{fold, Bucket};
/// # use store::format::Bar;
/// let at = |s: i64| s * 1_000_000;
/// let bar = |ts, p| Bar {
///     ts_micros: ts, open: p, high: p, low: p, close: p,
///     volume: 1, open_interest: i64::MIN,
/// };
/// // Three snapshots inside one minute, two sharing a second.
/// let snaps = vec![bar(at(0), 100), bar(at(0), 130), bar(at(30), 90)];
/// let bars = fold(&snaps, Bucket::MINUTE)?;
///
/// assert_eq!(bars.len(), 1, "one minute in, one bar out");
/// assert_eq!(bars[0].open, 100, "the FIRST in file order");
/// assert_eq!(bars[0].high, 130);
/// assert_eq!(bars[0].low, 90);
/// assert_eq!(bars[0].close, 90, "the LAST in file order");
/// assert_eq!(bars[0].volume, 3, "summed");
/// # Ok::<(), pull::fold::FoldError>(())
/// ```
pub fn fold(snapshots: &[Bar], bucket: Bucket) -> Result<Vec<Bar>, FoldError> {
    // THE GRID'S ORIGIN IS THE IST DAY, NOT THE UTC DAY.
    //
    // A CONSTANT AND NOT A PARAMETER, for the reason `CLAUDE.md` §6 gives for
    // the absent depth parameter: a value that can be set can be set wrongly,
    // and silently. §1 fixes the engine surface at NSE, so there is exactly one
    // trading day this store addresses and it is the IST one. Module-level
    // ([`IST_ANCHOR_MICROS`]) since D-3321, so the completeness fold places a
    // special session's first bucket on this same grid ([`grid_anchor`]).

    // ══ AND FOR AN INTRADAY RUNG THE GRID STARTS AT THE OPEN, NOT AT MIDNIGHT ══
    //
    // THE PROBLEM THIS SOLVES. The NSE open is 555 minutes past IST midnight,
    // so a midnight-anchored rung lands on 09:15 only when its length divides
    // 555. Three, five and fifteen do. TWO, TEN, THIRTY AND SIXTY DO NOT —
    // 277.5, 55.5, 18.5, 9.25 — and each of those opens the session with a bar
    // stamped BEFORE the open holding only part of it. A 30-minute bar stamped
    // 09:00 containing 09:15-09:29 is fifteen minutes of trade in a record
    // whose header says 1,800 seconds, and every later reader takes it as full.
    // That is why `store_timeframe` refused those rungs, and refusing them is
    // what stopped the operator's own ladder — 2, 3, 5, 10, 15, 30, 60 — from
    // existing at all.
    //
    // THE FAULT IS THE ANCHOR, NOT THE RUNG. Anchored at the OPEN, every rung's
    // first bar of the day begins exactly at 09:15. What is left over is a
    // SHORT LAST BAR wherever the VENUE's session length does not divide by the
    // rung — and that is a property of the venue row, not of the rung. The
    // 375-minute index session leaves one at 2, 10, 30 and 60; the 385-minute
    // equity-derivatives session from 2026-08-03 leaves one at 2, 3, 10, 15, 30
    // and 60 (a 1-minute 3min bar and a 10-minute 15min bar at 15:39 and
    // 15:30); the 360-minute continuous session of a cash security eligible
    // for the closing auction leaves none. Pinned against
    // `complete_minutes_for_venue` by
    // `pull::anchor::derived_rung_stub_minutes_follow_the_venue_session`
    // (D-1447). A trailing stub is a different object from a leading one: it
    // is stamped correctly and holds the trades that happened in it. The
    // leading stub was mislabelled. One is a short final bar; the other is a
    // lie.
    //
    // THE DAILY RUNG KEEPS MIDNIGHT, and that is not a special case for its own
    // sake. A day-wide bucket anchored at the open would run 09:15 to 09:15 —
    // one "day" spanning two calendar dates, which is not what `DAY_1` means
    // anywhere else in this repository. `Grid::Daily` is one record covering a
    // whole session, addressed by the IST day it fell on.
    //
    // NOTHING ALREADY WRITTEN MOVES. Computed rather than assumed:
    //
    //   rung    19800 % w    53100 % w    edges
    //   1min        0            0        unchanged
    //   3min        0            0        unchanged
    //   5min        0            0        unchanged
    //   15min       0            0        unchanged
    //   2min        0           60        MOVE — no rung existed to write them
    //   10min       0          300        MOVE — no rung existed
    //   30min       0          900        MOVE — store_timeframe refused it
    //   60min    1800         2700        MOVE — store_timeframe refused it
    //
    // Every rung whose edges move is a rung this build could never file until
    // now, so there are no bars on disk to disagree with. The four rungs that
    // DO have history are unchanged by arithmetic, not by luck: 60, 180, 300
    // and 900 all divide 33,300 as well as 19,800. `CLAUDE.md` §3 rule 8.
    // MINUS, NOT PLUS, AND THE SIGN IS THE WHOLE THING.
    //
    // The edge condition is `t + A ≡ 0 (mod w)`, so the grid lands on instants
    // where `t ≡ -A`. The midnight anchor is `+19,800` because IST midnight IS
    // `-19,800` in this frame. To land on the open — 33,300 seconds LATER — the
    // anchor has to move the same distance the other way.
    //
    // Adding it instead was wrong and the probe caught it at exactly one rung:
    // `(19,800 + 33,300) % 3,600` is 2,700 where 900 is needed, so the hour bar
    // opened 45 minutes BEFORE the session and every other rung looked right.
    // A sign error that is invisible on seven rungs out of eight is the reason
    // this is asserted against a real session rather than reasoned about.
    //
    // Negative is fine: `div_euclid` floors, so a negative anchor shifts the
    // grid without ever rounding toward zero. Every width in `Timeframe::KNOWN`
    // divides 86,400, so adding a day to the anchor would be equivalent — the
    // negative form is written because it is the arithmetic, not a workaround.
    //
    // [`OPEN_ANCHOR_MICROS`] and [`grid_anchor`] hold the constant and the
    // choice, at module level since D-3321.
    let anchor = grid_anchor(bucket);
    let width = i64::from(bucket.secs()) * 1_000_000;

    // THE EXACT BUCKET COUNT, RESERVED ONCE — this was a bare `Vec::new()`.
    //
    // The output is one bar per BUCKET, and the bucket a snapshot falls in is
    // `(ts - anchor).div_euclid(width)`. So the number of buckets the batch can
    // possibly touch is the first and last snapshot's bucket indices,
    // subtracted — O(1) arithmetic on two values already in hand, because the
    // loop below refuses anything out of order and the extremes are therefore
    // the ends.
    //
    // Growing from zero instead reallocated as it went: a month of one-minute
    // bars is ~8,250 pushes, which is roughly thirteen doublings and thirteen
    // copies of an ever-larger buffer. Amortised O(1) per push is the textbook
    // guarantee and it holds — this is not a complexity fix, it is the same
    // "constant work nobody needed" D-0366 removed from `store::file::read_row`
    // and it is invisible to a ratio gate for the same reason: work that is
    // doubled uniformly does not change a ratio.
    //
    // **Capped at `snapshots.len()`, and that cap is the load-bearing half.**
    // The span bound alone is wrong in the direction that costs memory: a
    // one-second archive folded to one minute has 60× more buckets in its span
    // than it has snapshots to fill them if the day is sparse, and a batch that
    // straddles a long gap would reserve for every empty bucket between. The
    // output can never exceed one bar per snapshot, so the smaller of the two
    // is the true bound and neither over-allocates.
    let span_buckets = match (snapshots.first(), snapshots.last()) {
        (Some(first), Some(last)) if width > 0 => {
            let from = first.ts_micros.saturating_sub(anchor).div_euclid(width);
            let to = last.ts_micros.saturating_sub(anchor).div_euclid(width);
            usize::try_from(to.saturating_sub(from).saturating_add(1)).unwrap_or(usize::MAX)
        }
        _ => 0,
    };
    let mut out: Vec<Bar> = Vec::with_capacity(span_buckets.min(snapshots.len()));
    let mut open_at: Option<i64> = None;
    let mut previous: Option<i64> = None;

    for (i, snap) in snapshots.iter().enumerate() {
        // A NEGATIVE COUNT IS REFUSED BEFORE IT CAN BE NETTED. D-1532.
        if snap.volume < 0 {
            return Err(FoldError::NegativeVolume {
                at: i,
                volume: snap.volume,
            });
        }
        if let Some(prev) = previous
            && snap.ts_micros < prev
        {
            return Err(FoldError::OutOfOrder {
                at: i,
                previous: prev,
                found: snap.ts_micros,
            });
        }
        previous = Some(snap.ts_micros);

        // The bucket a snapshot belongs to is arithmetic, not a search:
        // `div_euclid` rather than `/` so a pre-1970 instant floors downward
        // instead of toward zero, which would put it in the bucket after its own.
        //
        // THE ANCHOR IS WHAT THIS LINE WAS MISSING. Bucketing the bare epoch
        // puts every edge at UTC midnight, and the day this engine stores is an
        // IST day: `IST_OFFSET_SECS` is 19,800 and `86_400 % 19_800 != 0`, so a
        // day-wide edge falls 05:30 INSIDE the session it is meant to contain.
        //
        // A vendor stamps a daily candle at 00:00 IST, which is 18:30 UTC the
        // day BEFORE. Floored to the UTC grid it was re-stamped at 00:00 UTC of
        // that earlier day, so EVERY DAILY BAR MOVED BACK ONE CALENDAR DAY.
        // Measured on the store this produced: 86 records, 20 of them stamped
        // on a SUNDAY and none on a Friday, on an exchange that trades Monday
        // to Friday. `crate::ingest`'s month guard caught it only where the
        // shift crossed a month boundary — "bars span 2025-12 to 2026-01" — and
        // stored a wrong answer SILENTLY everywhere else, which is exactly the
        // W1 class `crate::fetch` names and says is undetectable once written.
        //
        // The minute rung is unchanged BY ARITHMETIC, not by luck: 60 divides
        // 19,800, so `(t + A).div_euclid(60M) * 60M - A` reduces exactly to
        // `t.div_euclid(60M) * 60M`. Only a width that does not divide the
        // offset moves, and `DAY_1` is the only such rung in `Timeframe::KNOWN`.
        //
        // Checked at both ends. Saturating here would silently file a bar in
        // the wrong bucket rather than refuse, which `CLAUDE.md` §4 bans.
        let start = bucket_start(snap.ts_micros, bucket).ok_or(FoldError::AnchorOverflow {
            ts_micros: snap.ts_micros,
        })?;

        if open_at == Some(start) {
            let Some(bar) = out.last_mut() else {
                // Unreachable: `open_at` is only ever Some after a push.
                return Err(FoldError::OutOfOrder {
                    at: i,
                    previous: start,
                    found: snap.ts_micros,
                });
            };
            bar.high = bar.high.max(snap.high);
            bar.low = bar.low.min(snap.low);
            // CLOSE IS THE LAST IN FILE ORDER, not the highest timestamp. Rows
            // sharing a second have no tiebreaker, so file order is the only
            // order there is and sorting would invent one.
            bar.close = snap.close;
            // CHECKED, for the reason the stamp above is: a saturated sum is a
            // bar whose volume is i64::MAX however much more traded, filed as if
            // it were measured. Refused instead (ET-bars-candles-store-4,
            // D-0955).
            bar.volume = bar
                .volume
                .checked_add(snap.volume)
                .ok_or(FoldError::VolumeOverflow {
                    at: i,
                    bucket: start,
                })?;
            if snap.open_interest != i64::MIN {
                bar.open_interest = snap.open_interest;
            }
        } else {
            open_at = Some(start);
            out.push(Bar {
                // The bar is stamped at the START of its bucket, which is what
                // `docs/00-charter.md` §3 means by a bar covering its interval.
                ts_micros: start,
                open: snap.open,
                high: snap.high,
                low: snap.low,
                close: snap.close,
                volume: snap.volume,
                open_interest: snap.open_interest,
            });
        }
    }

    Ok(out)
}

/// The IST day's origin on the UTC microsecond line: IST midnight is
/// `-19,800` seconds in this frame, so the grid edge condition `t + A ≡ 0`
/// lands on IST midnights with this `A`. See [`fold`].
const IST_ANCHOR_MICROS: i64 = crate::session::IST_OFFSET_SECS * 1_000_000;

/// The intraday grid's origin: the 09:15 open, 33,300 seconds after IST
/// midnight, subtracted for the sign reason [`fold`] gives.
const OPEN_ANCHOR_MICROS: i64 = IST_ANCHOR_MICROS
    - (store::path::Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT as i64) * 60 * 1_000_000;

/// The anchor a bucket of this width is placed with: IST midnight for a day
/// or wider, the session open for every intraday rung. [`fold`]'s choice,
/// shared so the completeness fold's arithmetic is the same grid (D-3321).
const fn grid_anchor(bucket: Bucket) -> i64 {
    if bucket.secs() >= 86_400 {
        IST_ANCHOR_MICROS
    } else {
        OPEN_ANCHOR_MICROS
    }
}

/// The start of the bucket `ts_micros` falls in, on [`fold`]'s grid, or
/// [`None`] where the shift leaves the `i64` range.
///
/// # The intraday grid restarts at each IST day's 09:15 (FIB-8, D-6446)
///
/// A day-or-wider bucket keeps the IST-midnight grid. An intraday bucket is
/// counted from the 09:15 open of the IST day the instant falls on: `open +
/// floor((t - open) / w) * w`. For every width that divides a day — every rung
/// in `Timeframe::KNOWN`, and every width this fold was ever called with —
/// that is exactly the epoch grid `t + A ≡ 0 (mod w)` with the open anchor,
/// because each day's open lies on it; so no stored bar moves
/// (`every_width_that_divides_a_day_keeps_its_grid`). A width that does not
/// divide a day (7 or 64 minutes, which FIB-1 R7's generated timeframe set
/// can name) drifted on the epoch grid, its first bucket landing at a
/// different minute each day; counted from the day's open it starts at 09:15
/// every day (`a_width_that_does_not_divide_a_day_starts_at_each_open`). No
/// NSE venue trades across IST midnight, so a day's last bucket never meets
/// the next day's first.
fn bucket_start(ts_micros: i64, bucket: Bucket) -> Option<i64> {
    let width = i64::from(bucket.secs()) * 1_000_000;
    if bucket.secs() >= Bucket::DAY.secs() {
        let anchor = grid_anchor(bucket);
        return ts_micros
            .checked_add(anchor)?
            .div_euclid(width)
            .checked_mul(width)?
            .checked_sub(anchor);
    }
    let day = i64::from(Bucket::DAY.secs()) * 1_000_000;
    let midnight = ts_micros
        .checked_add(IST_ANCHOR_MICROS)?
        .div_euclid(day)
        .checked_mul(day)?
        .checked_sub(IST_ANCHOR_MICROS)?;
    let open = midnight.checked_add(IST_ANCHOR_MICROS - OPEN_ANCHOR_MICROS)?;
    open.checked_add(
        ts_micros
            .checked_sub(open)?
            .div_euclid(width)
            .checked_mul(width)?,
    )
}

/// Why snapshots could not be folded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FoldError {
    /// A width narrower than the bars it is being built from.
    ///
    /// **This is an information limit, not an arithmetic one, and it was found
    /// by measurement rather than by reading.** A folded one-minute bar is
    /// stamped at the *start* of its minute, so the whole minute travels with
    /// that single timestamp and *when inside the minute* things happened is
    /// gone. Fold again at a width whose edges do not land on minute
    /// boundaries and the entire minute is attributed to whichever bucket the
    /// stamp falls in — dragging in data from after that bucket closed.
    ///
    /// Measured on the operator's archive, all 12,132 contracts:
    ///
    /// | Width | Bars compared | Wrong | Worst error |
    /// |---|---|---|---|
    /// | 90 s | 2,513,114 | **513,605 (20.4%)** | ₹1,147.90 |
    /// | 100 s | 2,275,830 | **510,889 (22.4%)** | ₹1,147.90 |
    ///
    /// One concrete case: a 100-second bucket `[12:55:00, 12:56:40)` built from
    /// minute bars swallowed a trade stamped **12:56:58** — eighteen seconds of
    /// the future inside a closed bar, which is `CLAUDE.md` §3 rule 7 broken by
    /// arithmetic rather than by an accessor.
    ///
    /// **Nothing is forbidden by this.** Any width at all is exact when folded
    /// from the raw snapshots, which is what [`fold_from_snapshots`] is for.
    /// This refusal names the one combination that loses information.
    NarrowerThanSource {
        /// The width asked for, in seconds.
        want_secs: u32,
        /// The width the input bars already carry, in seconds.
        source_secs: u32,
    },
    /// A timestamp so near an `i64` end that shifting it onto the IST grid
    /// leaves the range.
    ///
    /// Refused rather than saturated: a saturated instant lands in a bucket
    /// that is not its own, which files a bar under the wrong month and is the
    /// silent wrong answer `CLAUDE.md` §4 bans. Unreachable from any real
    /// vendor row — `i64` microseconds span ±292,000 years — and stated anyway,
    /// because the arm that says "this cannot happen" is a refusal and not a
    /// wrap.
    AnchorOverflow {
        /// The snapshot's own stamp, before the shift.
        ts_micros: i64,
    },
    /// A bucket's summed volume leaves the range an `i64` can hold.
    ///
    /// Refused rather than saturated, for the reason [`Self::AnchorOverflow`]
    /// is: a capped sum is a wrong volume filed as a measured one. Unreachable
    /// from any real exchange print and stated anyway (ET-bars-candles-store-4,
    /// D-0955).
    VolumeOverflow {
        /// Zero-based position of the snapshot whose volume overflowed.
        at: usize,
        /// The start of the bucket it was being added to.
        bucket: i64,
    },
    /// A snapshot whose volume is below zero. audit-20261003 attackdata-5,
    /// D-1532.
    ///
    /// The sum was checked for overflow and not for sign, so a negative
    /// snapshot was netted into a plausible positive bucket (10 + -7 = 3) that
    /// then passed the store's count gate, which sees only the sum. A count of
    /// trades cannot be negative; refused here, where the row is still named.
    NegativeVolume {
        /// Zero-based position of the snapshot.
        at: usize,
        /// Its volume as offered.
        volume: i64,
    },
    /// A snapshot's timestamp precedes the one before it.
    ///
    /// Refused rather than sorted. Rows sharing a second carry no tiebreaker,
    /// so a sort would invent an order and quietly change which price became
    /// the open — and nothing downstream could tell.
    OutOfOrder {
        /// Zero-based position in the input.
        at: usize,
        /// The timestamp before it.
        previous: i64,
        /// What was found.
        found: i64,
    },
}

impl core::fmt::Display for FoldError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // A `match`, not a `let ... else`. The single-variant destructure that
        // stood here became `unreachable!()` the moment a second variant
        // existed, and the guard's own test is what fired it — a refusal that
        // panicked instead of printing.
        match *self {
            Self::NarrowerThanSource {
                want_secs,
                source_secs,
            } => write!(
                f,
                "a {want_secs}s bucket cannot be built from {source_secs}s \
                 bars: {want_secs} is not a whole multiple of {source_secs}, so \
                 a bucket edge falls INSIDE a source bar and that bar cannot be \
                 split — the information to split it was discarded when it was \
                 made. Measured cost of allowing it: 20.4% of bars wrong at \
                 90s, worst error Rs 1,147.90. Fold from the raw snapshots \
                 instead, where any width is exact."
            ),
            Self::AnchorOverflow { ts_micros } => write!(
                f,
                "the snapshot stamped {ts_micros} cannot be shifted onto the \
                 IST day grid without leaving the range an i64 can hold. \
                 Refused rather than saturated: a saturated instant lands in a \
                 bucket that is not its own, which files the bar under the \
                 wrong month."
            ),
            Self::VolumeOverflow { at, bucket } => write!(
                f,
                "snapshot {at} takes the volume of the bucket at {bucket} past \
                 what an i64 can hold. Refused rather than saturated: a capped \
                 sum is a wrong volume filed as a measured one."
            ),
            Self::NegativeVolume { at, volume } => write!(
                f,
                "snapshot {at} carries a negative volume of {volume}. A count of \
                 trades cannot be below zero, and summed into its bucket it \
                 would hide as a smaller plausible volume."
            ),
            Self::OutOfOrder {
                at,
                previous,
                found,
            } => write!(
                f,
                "snapshot {at} is stamped {found}, before {previous}. Refused \
                 rather than sorted: rows sharing a second have no tiebreaker, \
                 so a sort invents an order and changes which price became the \
                 open."
            ),
        }
    }
}

impl core::error::Error for FoldError {}

// ───────────────────────────── the ladder ─────────────────────────────

/// Which segment a stage covers, in the order they must be pulled.
///
/// The discriminant **is** the order, so a new segment is inserted at its true
/// position and cannot accidentally sort elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Segment {
    /// Indices and cash. Two swept instruments — the cheapest rehearsal.
    Spot = 0,
    /// Expired futures. ~213 underlyings.
    Futures = 1,
    /// Expired option chains. ~11,500 contracts in one day.
    Options = 2,
}

/// Which granularity a stage covers, in the order they must be pulled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Grain {
    /// One bar per day. Proves the whole chain at 1/375th the volume.
    Daily = 0,
    /// One bar per minute.
    Minute = 1,
    /// The coarser rungs, FOLDED FROM THE ONE-MINUTE BARS RATHER THAN PULLED.
    ///
    /// 2, 3, 5, 10, 15, 30 and 60 minute bars are never asked of a vendor:
    /// they are one-minute bars folded up, which is why `/db` reports nine
    /// rungs held against the two that were ever fetched.
    ///
    /// That makes this a stage with a HARD PREDECESSOR rather than a bonus
    /// pass. Folding a month whose one-minute pull was dirty produces coarse
    /// bars built out of gaps, and nothing downstream can tell them from
    /// complete ones — a month that looks finished and is not, which is the
    /// whole failure this ladder exists to prevent.
    Derived = 2,
    /// The greeks, SOLVED FROM BARS ALREADY ON DISK.
    ///
    /// Options only, and that is physics rather than scheduling: a future has
    /// no optionality and a spot series has neither strike nor expiry, so
    /// there is nothing to solve for either. It appears once in [`LADDER`],
    /// under `Options`, for that reason.
    ///
    /// `crates/greeks` needs the contract's own bar AND the underlying's spot
    /// bar at the SAME minute. Both are guaranteed present by the order below
    /// rather than hoped for: spot's minute pass is stage 2 and cannot be
    /// skipped to reach this one.
    Greeks = 3,
}

/// One rung: a segment at a granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Stage {
    /// Which segment.
    pub segment: Segment,
    /// Which granularity.
    pub grain: Grain,
}

/// Every rung, in the only order they may be attempted.
///
/// Granularity is the **outer** loop: all three segments at daily, then all
/// three at minute. Daily is 1/375th the volume of minute, so finishing the
/// cheap pass across every segment before starting the expensive one surfaces a
/// structural fault — a wrong path, a bad session bound, a dead credential —
/// against the smallest possible amount of work.
pub const LADDER: [Stage; 10] = [
    // ── SPOT, FINISHED ENTIRELY, BEFORE ANY CONTRACT IS ASKED FOR ──────────
    Stage {
        segment: Segment::Spot,
        grain: Grain::Daily,
    },
    Stage {
        segment: Segment::Spot,
        grain: Grain::Minute,
    },
    Stage {
        segment: Segment::Spot,
        grain: Grain::Derived,
    },
    // ── EXPIRED FUTURES ────────────────────────────────────────────────────
    Stage {
        segment: Segment::Futures,
        grain: Grain::Daily,
    },
    Stage {
        segment: Segment::Futures,
        grain: Grain::Minute,
    },
    Stage {
        segment: Segment::Futures,
        grain: Grain::Derived,
    },
    // ── EXPIRED OPTIONS, AND THE ONLY SEGMENT THAT SOLVES GREEKS ───────────
    Stage {
        segment: Segment::Options,
        grain: Grain::Daily,
    },
    Stage {
        segment: Segment::Options,
        grain: Grain::Minute,
    },
    Stage {
        segment: Segment::Options,
        grain: Grain::Derived,
    },
    Stage {
        segment: Segment::Options,
        grain: Grain::Greeks,
    },
];

/// How far a vendor has got, and what it may attempt next.
///
/// One ladder per vendor. Every vendor climbs the same rungs in the same order
/// without exception — a vendor allowed to skip is a vendor whose failures
/// surface at the most expensive rung instead of the cheapest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ladder {
    done: u8,
}

impl Ladder {
    /// A vendor that has pulled nothing.
    #[must_use]
    pub const fn new() -> Self {
        Self { done: 0 }
    }

    /// The stage that may run now, or [`None`] when every rung is complete.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pull::fold::{Ladder, Segment, Grain};
    /// let mut l = Ladder::new();
    /// assert_eq!(l.next().map(|s| s.segment), Some(Segment::Spot));
    /// assert_eq!(l.next().map(|s| s.grain), Some(Grain::Daily));
    ///
    /// // A stage that failed does NOT advance the ladder.
    /// l.record(false);
    /// assert_eq!(l.next().map(|s| s.segment), Some(Segment::Spot), "still spot");
    ///
    /// // SEGMENT-MAJOR: a clean day pass moves to spot's MINUTE, not to
    /// // futures. A segment is finished entirely before the next is touched,
    /// // because expired options cannot have their implied volatility solved
    /// // without the underlying's spot bar at the same minute.
    /// l.record(true);
    /// assert_eq!(l.next().map(|s| s.segment), Some(Segment::Spot), "still spot");
    /// assert_eq!(l.next().map(|s| s.grain), Some(Grain::Minute));
    ///
    /// // Spot's minute, then its derived rungs, and only then futures.
    /// l.record(true);
    /// l.record(true);
    /// assert_eq!(l.next().map(|s| s.segment), Some(Segment::Futures));
    /// ```
    #[must_use]
    pub fn next(self) -> Option<Stage> {
        LADDER.get(self.done as usize).copied()
    }

    /// Records the outcome of the stage [`Ladder::next`] returned.
    ///
    /// **Only a completely clean stage advances.** `clean` must mean zero
    /// failures — not "mostly worked". A partial success that advanced would
    /// carry its gap into a stage 375 times larger, where finding it costs 375
    /// times as much.
    pub const fn record(&mut self, clean: bool) {
        if clean && (self.done as usize) < LADDER.len() {
            self.done += 1;
        }
    }

    /// How many rungs are complete.
    #[must_use]
    pub const fn completed(self) -> usize {
        self.done as usize
    }

    /// Whether every rung is done.
    #[must_use]
    pub const fn finished(self) -> bool {
        self.done as usize >= LADDER.len()
    }
}

// THE COUNT IS PINNED SO A HALF-ADDED STAGE IS A BUILD FAILURE, not a rung the
// ladder silently never reaches. 10 = spot, futures and options at day, minute
// and derived, plus the one greeks stage options alone can have.
const _: () = assert!(LADDER.len() == 10);
// AND THE ORDER IS PINNED, not just the length. A ladder of the right size in
// the wrong order is the defect this const catches: reordering it to
// grain-major would still be ten stages and would still compile.
const _: () = assert!(matches!(LADDER[0].segment, Segment::Spot));
const _: () = assert!(matches!(LADDER[2].grain, Grain::Derived));
const _: () = assert!(matches!(LADDER[3].segment, Segment::Futures));
const _: () = assert!(matches!(LADDER[9].grain, Grain::Greeks));

/// Folds raw snapshots at **any** width. Always exact.
///
/// A snapshot carries its own instant, so no width can misattribute it. This
/// is the entry point a sub-minute timeframe must use, and the reason nothing
/// about the design is locked down: 1 second, 7 seconds, 90 seconds and one
/// day all go through here and all are correct.
///
/// # Errors
///
/// [`FoldError::OutOfOrder`] if the snapshots are not in non-decreasing
/// timestamp order.
pub fn fold_from_snapshots(snapshots: &[Bar], bucket: Bucket) -> Result<Vec<Bar>, FoldError> {
    fold(snapshots, bucket)
}

/// Folds bars that are already `source` wide into `bucket`-wide bars.
///
/// # Errors
///
/// [`FoldError::NarrowerThanSource`] when `bucket` is not a whole multiple of
/// `source`. That is the combination which loses information: see the variant's
/// own documentation for the measured cost of allowing it.
///
/// [`FoldError::OutOfOrder`] as for [`fold`].
///
/// # Examples
///
/// ```
/// # use pull::fold::{fold_from_bars, Bucket, FoldError};
/// let one_min = Bucket::MINUTE;
///
/// // Every whole-minute width is exact, so all of these are legal.
/// for secs in [60u32, 120, 180, 300, 900, 3_600, 86_400] {
///     let wider = Bucket::of_secs(secs).expect("non-zero");
///     assert!(fold_from_bars(&[], wider, one_min).is_ok(), "{secs}s");
/// }
///
/// // 90 seconds is one and a half minutes. Refused BY NAME, not silently
/// // mis-attributed — this is the case measured at 20.4% wrong bars.
/// let ninety = Bucket::of_secs(90).expect("non-zero");
/// assert_eq!(
///     fold_from_bars(&[], ninety, one_min),
///     Err(FoldError::NarrowerThanSource { want_secs: 90, source_secs: 60 }),
/// );
/// ```
pub fn fold_from_bars(bars: &[Bar], bucket: Bucket, source: Bucket) -> Result<Vec<Bar>, FoldError> {
    // WIDER IS ALWAYS SAFE, NARROWER NEVER IS, AND "NOT A WHOLE MULTIPLE" IS
    // NARROWER IN THE ONLY sense that matters: some bucket edge falls inside a
    // source bar, and that bar cannot be split because the information to split
    // it was discarded when it was made.
    if !bucket.secs().is_multiple_of(source.secs()) {
        return Err(FoldError::NarrowerThanSource {
            want_secs: bucket.secs(),
            source_secs: source.secs(),
        });
    }
    fold(bars, bucket)
}

/// Fold minute candles only when every scheduled minute in a bucket exists.
/// The fixed persisted grid is retained, including scheduled closing stubs.
/// Missing buckets are omitted and named; no prices or store bytes are repaired.
/// A special session folds inside its recorded windows on the same grid
/// (D-3321 item 1): a bucket owes exactly the window minutes it overlaps.
/// Entirely absent civil days are not checked: input bars alone do not prove
/// request coverage. Gap checks reset at each observed IST day and inspect only
/// that day's scheduled windows, never the elapsed span between observations.
/// A later observed IST day closes the previous day's tail check at its known
/// session close. The final input day's unobserved tail remains unchecked:
/// there is no request endpoint here to distinguish a gap from a partial read.
///
/// # Errors
/// Returns the ordinary fold refusal for unordered input or incompatible widths.
pub fn complete_minutes(
    bars: &[Bar],
    bucket: Bucket,
) -> Result<(Vec<Bar>, Vec<String>), FoldError> {
    complete_minutes_for_venue(bars, bucket, crate::vendor::Venue::NseIndex)
}

/// Venue-aware minute completeness on the fixed store grid. Regular trading
/// days use the venue's dated continuous hours; special sessions their
/// recorded windows. Cash uses the existing venue schedule, without inferring
/// per-instrument closing-auction eligibility, and folds a bucket over the
/// minutes that traded (D-3321 item 2). See [`complete_minutes`] for the limits
/// on unobserved days and final tails.
///
/// # Errors
/// Returns the ordinary fold refusal for unordered input or incompatible widths.
pub fn complete_minutes_for_venue(
    bars: &[Bar],
    bucket: Bucket,
    venue: crate::vendor::Venue,
) -> Result<(Vec<Bar>, Vec<String>), FoldError> {
    complete_minutes_with_cash_schedule(bars, bucket, venue, None)
}

/// Complete continuous-minute buckets using dated per-security CAS evidence.
/// Missing evidence withholds the affected day; auction rows never complete a
/// continuous bucket.
///
/// # Errors
/// Returns the same order/width/overflow refusals as `complete_minutes`.
pub fn complete_minutes_with_cash_schedule(
    bars: &[Bar],
    bucket: Bucket,
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
) -> Result<(Vec<Bar>, Vec<String>), FoldError> {
    complete_minutes_with_calendar(
        bars,
        bucket,
        venue,
        cash_schedule,
        crate::calendar::Runtime::default(),
    )
}

/// Audit derived buckets with the same runtime calendar as request coverage.
/// Observed calendars cannot authorize a session or erase a static unknown.
///
/// One diagnostic per withheld bucket or range, and one per withheld day:
/// the detailed form `cli::fold_audit` and the bucket tests read. The derive
/// reads [`derive_minutes`], which counts the same notes instead of
/// repeating them (D-3341).
///
/// # Errors
/// Returns the same order/width/overflow refusals as `complete_minutes`.
pub fn complete_minutes_with_calendar(
    bars: &[Bar],
    bucket: Bucket,
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
    runtime: crate::calendar::Runtime<'_>,
) -> Result<(Vec<Bar>, Vec<String>), FoldError> {
    let folded = fold_month(bars, bucket, venue, cash_schedule, runtime)?;
    Ok((
        folded.bars,
        folded.notes.into_iter().map(|note| note.text).collect(),
    ))
}

/// What one derive fold of a month did, counted (D-3321 item 3, D-3335).
///
/// The four numbers of the ledger's `derive` line. Each is a count over the
/// minutes handed to the fold, which the derive reads as the whole committed
/// source month, so a later derive of the same month answers for the whole
/// month again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Distinct IST days with at least one folded bucket: the days the
    /// derived rung covers.
    pub days: u32,
    /// Buckets folded: every complete one and, for a share, every partial one.
    pub buckets: u32,
    /// Buckets the fold refused: a bucket whose minutes do not attest it, a
    /// scheduled bucket with no minute at all on an observed day of a series
    /// that owes every minute, and every candidate bucket of a day the
    /// calendar or the venue cannot attest. A share's bucket with no trade is
    /// not refused; it has no bar.
    pub withheld: u32,
    /// Of [`Self::buckets`], those of a share folded over fewer minutes than
    /// the session scheduled, because some minutes had no trade (D-3321
    /// item 2).
    pub partial: u32,
}

/// Why a derived bucket, or every bucket of a day, was withheld: the reason
/// a repeated diagnostic is counted under (D-3341 item 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Withheld {
    /// A bucket whose minutes do not cover the minutes it owes, or hold one
    /// it does not owe, a duplicate or an off-grid stamp.
    Incomplete,
    /// Scheduled buckets with no minute at all, between observed ones.
    Absent,
    /// Scheduled buckets with no minute at all after a day's last observed one.
    AbsentTail,
    /// A day outside the calendar's sourced spans.
    Unverified,
    /// A traded day whose session length no record states.
    Unsized,
    /// Minute bars on a day the calendar measures closed.
    Closed,
    /// A day whose venue hours, or dated cash eligibility, cannot attest the
    /// stored grid.
    Unattested,
}

impl Withheld {
    /// The reason's code, as an aggregated event's `reason` field carries
    /// it. A code is never renamed or reused.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Incomplete => "incomplete_minutes",
            Self::Absent => "absent_minutes",
            Self::AbsentTail => "absent_tail",
            Self::Unverified => "calendar_unverified",
            Self::Unsized => "session_unsized",
            Self::Closed => "closed_day",
            Self::Unattested => "session_unattested",
        }
    }

    /// What an aggregated line says the counted notes were.
    const fn label(self) -> &'static str {
        match self {
            Self::Incomplete => "incomplete or invalid minute coverage",
            Self::Absent => "scheduled buckets with no minute between observed ones",
            Self::AbsentTail => "scheduled buckets with no minute after a day's last observed one",
            Self::Unverified => "days outside the calendar's authority",
            Self::Unsized => "traded sessions whose length no record states",
            Self::Closed => "minute bars on days the calendar measures closed",
            Self::Unattested => "days whose session hours cannot attest the stored grid",
        }
    }
}

/// One derive fold's answer: the bars, the counts, and one diagnostic per
/// reason rather than per bucket (D-3341 item 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// The folded bars, complete and (for a share) partial.
    pub bars: Vec<Bar>,
    /// The counts the ledger's `derive` line carries.
    pub tally: Tally,
    /// One line per [`Withheld`] reason that occurred, in the order each
    /// first occurred. A reason seen once is its own sentence, unchanged; a
    /// reason seen more often is one line that counts its buckets and days,
    /// names the first and last day, and quotes the first sentence, so a
    /// suffix a caller rewrites still ends the line.
    pub diagnostics: Vec<String>,
}

/// The derive's fold of one month of minutes into `bucket`: the same bars as
/// [`complete_minutes_with_calendar`], counted into a [`Tally`], with the
/// diagnostics aggregated per reason (D-3321, D-3341).
///
/// # Why this exists beside the detailed form
///
/// One line per withheld bucket was one `pull.derive` warning, one receipt
/// failure and one `pull.spot` "member did not land" error per bucket:
/// 31,625 events for BLUESTARCO alone, and a rolling log that covered 8 to 12
/// minutes of a pull (DB-G88). The derive now reads one line per reason per
/// rung-month, which is bounded by the seven reasons, and the counts that
/// the lines summarise go to the ledger.
///
/// # Cost
///
/// [`complete_minutes_with_calendar`]'s, plus one pass over its notes, which
/// are bounded by the buckets and days of the month.
///
/// # Errors
/// Returns the same order/width/overflow refusals as `complete_minutes`.
pub fn derive_minutes(
    bars: &[Bar],
    bucket: Bucket,
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
    runtime: crate::calendar::Runtime<'_>,
) -> Result<Derived, FoldError> {
    let folded = fold_month(bars, bucket, venue, cash_schedule, runtime)?;
    Ok(Derived {
        bars: folded.bars,
        tally: folded.tally,
        diagnostics: aggregate(&folded.notes),
    })
}

/// One withheld bucket, range or day, with the reason it is counted under.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Note {
    why: Withheld,
    /// The IST day, as days since the epoch.
    day: i64,
    /// Buckets this note withheld.
    buckets: u32,
    /// The sentence, as the detailed form returns it.
    text: String,
}

/// [`fold_month`]'s answer.
struct Folded {
    bars: Vec<Bar>,
    notes: Vec<Note>,
    tally: Tally,
}

/// A day's unobserved tail, checked when the next observed day begins.
#[derive(Clone, Copy)]
struct Tail {
    day: i64,
    /// The first instant after the day's last observed bucket, or its open.
    missing: i64,
    /// The day's session close.
    close: i64,
    session: crate::calendar::DayKind,
    midnight: i64,
}

/// The one completeness fold both public forms read.
#[expect(
    clippy::too_many_lines,
    reason = "one pass over the candidate buckets that carries six pieces of \
              per-day state between iterations; split, the state would cross \
              a function boundary as a struct of mutable references"
)]
fn fold_month(
    bars: &[Bar],
    bucket: Bucket,
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
    runtime: crate::calendar::Runtime<'_>,
) -> Result<Folded, FoldError> {
    let candidates = fold_from_bars(bars, bucket, Bucket::MINUTE)?;
    let width = i64::from(bucket.secs()) * 1_000_000;
    // A SHARE'S MINUTE WITH NO CANDLE IS A MINUTE WITH NO TRADE (D-3321 item
    // 2). An index is computed every minute of its session, so a minute
    // missing from it is missing source and its bucket is withheld. A share
    // trades only when somebody trades it: 360ONE 2023-02-08 had six minutes
    // with no candle at Zerodha and no trade at GDFL either, and withholding
    // the enclosing buckets held 360ONE's 60min rung at 74.6% of expected
    // (DB-G36). So a share's bucket folds over the minutes that traded and
    // is counted `partial`, and a bucket or tail with no trade at all is no
    // bar rather than a withheld one. Futures keep the index's rule: nothing
    // records which of their minutes were no trade.
    let no_trade_is_absence = venue == crate::vendor::Venue::NseCash;
    let mut out = Folded {
        bars: Vec::with_capacity(candidates.len()),
        notes: Vec::new(),
        tally: Tally::default(),
    };
    let mut cursor = 0;
    let mut previous_end: Option<i64> = None;
    let mut previous_day: Option<i64> = None;
    let mut previous_tail: Option<Tail> = None;
    // The note that names the current day withheld, if one does, so each of
    // its buckets is counted under it.
    let mut day_note_at: Option<usize> = None;
    let mut counted_day: Option<i64> = None;
    // ONE CALENDAR AND SESSION LOOKUP PER DAY, NOT PER BUCKET (OD-1, D-2370).
    // `(calendar, session)` of the day the previous bucket fell on; a bucket of
    // the same day reuses it instead of re-deriving the venue's dated hours.
    let mut day_state = (
        crate::calendar::DayKind::Unmeasured,
        crate::calendar::DayKind::Unmeasured,
    );
    for bar in candidates {
        let start = bar.ts_micros;
        let end = start
            .checked_add(width)
            .ok_or(FoldError::AnchorOverflow { ts_micros: start })?;
        let (day, midnight) = ist_day_of(start)?;
        let new_day = previous_day != Some(day);
        if new_day {
            day_note_at = None;
            let before = out.notes.len();
            day_state = day_session(day, venue, cash_schedule, runtime, &mut out.notes);
            if out.notes.len() > before {
                day_note_at = Some(before);
            }
            // Flush only the last observed day's scheduled tail before
            // resetting. No candidate exists for an entirely absent closing
            // bucket, and no timetable or request coverage is inferred for
            // intervening days.
            if let Some(tail) = previous_tail.take()
                && !no_trade_is_absence
            {
                flush_tail(tail, bucket, &mut out);
            }
            let (calendar, session) = day_state;
            if let Some(note) = day_note(day, calendar, session, runtime) {
                day_note_at = Some(out.notes.len());
                out.notes.push(note);
            }
        }
        let (calendar, session) = day_state;
        // Entirely absent buckets never become fold candidates. Name those
        // between observed buckets too; do not treat session breaks as gaps.
        let first_open = session_open(session, midnight, bucket)?;
        let gap_start = if new_day { first_open } else { previous_end };
        if let Some(missing) = gap_start.filter(|missing| *missing < start)
            && !no_trade_is_absence
        {
            let expected = scheduled_minutes(session, midnight, missing, start);
            if expected > 0 {
                let buckets = owed_buckets(session, midnight, missing, start, width);
                out.tally.withheld = out.tally.withheld.saturating_add(buckets);
                out.notes.push(Note {
                    why: Withheld::Absent,
                    day,
                    buckets,
                    text: format!("bucket range [{missing}, {start}): absent: observed 0, scheduled {expected}; withheld; historical gap refill requires a versioned store repair"),
                });
            }
        }
        previous_end = Some(end);
        previous_day = Some(day);
        previous_tail = match first_open {
            Some(open) => Some(Tail {
                day,
                missing: end.max(open),
                close: session_close(session, midnight)
                    .ok_or(FoldError::AnchorOverflow { ts_micros: start })?,
                session,
                midnight,
            }),
            None => None,
        };
        let mut count = 0_i64;
        let mut valid = true;
        let mut previous = None;
        while let Some(source) = bars.get(cursor).filter(|source| source.ts_micros < end) {
            let minute = u16::try_from(
                (i128::from(source.ts_micros) - i128::from(midnight))
                    .div_euclid(i128::from(MINUTE_MICROS)),
            )
            .ok();
            valid &= source.ts_micros.rem_euclid(MINUTE_MICROS) == 0
                && previous != Some(source.ts_micros)
                && matches!((session, minute), (crate::calendar::DayKind::Open(s), Some(m)) if s.expects(m));
            previous = Some(source.ts_micros);
            count += 1;
            cursor += 1;
        }
        let expected = scheduled_minutes(session, midnight, start, end);
        // AN UNMEASURED OR WITHHELD DAY IS NAMED ONCE, NOT ONCE PER BUCKET
        // (o1api-44, D-1201; hunt-pull-3, D-1533). The day sentence above
        // already says every derived bucket of it is withheld and why; each
        // bucket is counted under it. The minutes are still consumed above;
        // nothing is certified.
        if calendar == crate::calendar::DayKind::Unmeasured || day_is_withheld(session) {
            out.tally.withheld = out.tally.withheld.saturating_add(1);
            if let Some(note) = day_note_at.and_then(|at| out.notes.get_mut(at)) {
                note.buckets = note.buckets.saturating_add(1);
            }
            continue;
        }
        let complete = valid && expected > 0 && i128::from(count) == expected;
        let partial = no_trade_is_absence
            && valid
            && count > 0
            && expected > 0
            && i128::from(count) < expected;
        if complete || partial {
            out.bars.push(bar);
            out.tally.buckets = out.tally.buckets.saturating_add(1);
            out.tally.partial = out.tally.partial.saturating_add(u32::from(partial));
            if counted_day != Some(day) {
                counted_day = Some(day);
                out.tally.days = out.tally.days.saturating_add(1);
            }
        } else {
            out.tally.withheld = out.tally.withheld.saturating_add(1);
            out.notes.push(Note {
                why: Withheld::Incomplete,
                day,
                buckets: 1,
                text: format!("bucket {start}: incomplete or invalid minute coverage: observed {count}, scheduled {expected}, calendar {session:?}; withheld; historical gap refill requires a versioned store repair"),
            });
        }
    }
    Ok(out)
}

/// One minute, in microseconds.
const MINUTE_MICROS: i64 = 60_000_000;

/// Names, counts and withholds a day's scheduled tail with no minute at all.
fn flush_tail(tail: Tail, bucket: Bucket, out: &mut Folded) {
    if tail.missing >= tail.close {
        return;
    }
    let expected = scheduled_minutes(tail.session, tail.midnight, tail.missing, tail.close);
    if expected <= 0 {
        return;
    }
    let width = i64::from(bucket.secs()) * 1_000_000;
    let buckets = owed_buckets(tail.session, tail.midnight, tail.missing, tail.close, width);
    out.tally.withheld = out.tally.withheld.saturating_add(buckets);
    let (missing, close) = (tail.missing, tail.close);
    out.notes.push(Note {
        why: Withheld::AbsentTail,
        day: tail.day,
        buckets,
        text: format!("bucket range [{missing}, {close}): absent: observed 0, scheduled {expected}; observed-day tail withheld; historical gap refill requires a versioned store repair"),
    });
}

/// The start of the bucket holding the session's first scheduled minute, on
/// [`fold`]'s grid, or [`None`] for a day that is not an open session.
///
/// A regular session's is 09:15 itself, which the intraday grid is anchored
/// at. A special session's need not be: the 2024-11-01 Muhurat opened at
/// 18:00, and its first 60min bucket on the 09:15 grid starts at 17:15 and
/// owes the fifteen minutes 18:00-18:14 (D-3321 item 1, "the same bucket
/// grid as the regular session's").
fn session_open(
    session: crate::calendar::DayKind,
    midnight: i64,
    bucket: Bucket,
) -> Result<Option<i64>, FoldError> {
    let crate::calendar::DayKind::Open(s) = session else {
        return Ok(None);
    };
    let Some(first) = s.windows.first().filter(|_| s.count > 0) else {
        return Ok(None);
    };
    let at = midnight
        .checked_add(i64::from(first.from) * MINUTE_MICROS)
        .ok_or(FoldError::AnchorOverflow {
            ts_micros: midnight,
        })?;
    bucket_start(at, bucket)
        .map(Some)
        .ok_or(FoldError::AnchorOverflow { ts_micros: at })
}

/// The instant after the session's last scheduled minute, or [`None`] for a
/// day that is not an open session or an instant past the `i64` range.
fn session_close(session: crate::calendar::DayKind, midnight: i64) -> Option<i64> {
    let crate::calendar::DayKind::Open(s) = session else {
        return None;
    };
    let last = s.windows.get(usize::from(s.count).checked_sub(1)?)?;
    midnight.checked_add((i64::from(last.to) + 1) * MINUTE_MICROS)
}

/// How many buckets starting in `[from, to)` on the grid owe at least one
/// scheduled minute. `from` is a bucket start; `to` need not be.
///
/// # Cost
///
/// One [`scheduled_minutes`] per bucket in the range, which lies inside one
/// day, so at most a day's buckets at this width. Each one counted is a
/// bucket withheld, so the work is the size of what it reports.
fn owed_buckets(
    session: crate::calendar::DayKind,
    midnight: i64,
    from: i64,
    to: i64,
    width: i64,
) -> u32 {
    let mut owed = 0_u32;
    let mut at = from;
    while at < to {
        let Some(next) = at.checked_add(width) else {
            break;
        };
        if scheduled_minutes(session, midnight, at, next) > 0 {
            owed = owed.saturating_add(1);
        }
        at = next;
    }
    owed
}

/// One line per reason, in first-occurrence order (D-3341 item 2). See
/// [`Derived::diagnostics`].
fn aggregate(notes: &[Note]) -> Vec<String> {
    // At most seven reasons, so the lookup below is a walk of at most seven.
    struct Group<'a> {
        why: Withheld,
        notes: u32,
        buckets: u32,
        days: u32,
        first_day: i64,
        last_day: i64,
        first: &'a str,
    }
    let mut groups: Vec<Group<'_>> = Vec::with_capacity(7);
    'notes: for note in notes {
        for group in &mut groups {
            if group.why == note.why {
                group.notes = group.notes.saturating_add(1);
                group.buckets = group.buckets.saturating_add(note.buckets);
                if note.day != group.last_day {
                    group.days = group.days.saturating_add(1);
                    group.last_day = note.day;
                }
                continue 'notes;
            }
        }
        {
            groups.push(Group {
                why: note.why,
                notes: 1,
                buckets: note.buckets,
                days: 1,
                first_day: note.day,
                last_day: note.day,
                first: &note.text,
            });
        }
    }
    groups
        .into_iter()
        .map(|group| {
            if group.notes == 1 {
                return group.first.to_owned();
            }
            format!(
                "{}: {} bucket(s) withheld on {} day(s), {}..{}, over {} note(s); the first: {}",
                group.why.label(),
                group.buckets,
                group.days,
                civil(group.first_day),
                civil(group.last_day),
                group.notes,
                group.first
            )
        })
        .collect()
}

/// An IST day as `YYYY-MM-DD`, or `day N` where the calendar cannot name it.
fn civil(day: i64) -> String {
    u32::try_from(day)
        .ok()
        .and_then(|days| crate::session::Day::from_days(days).ok())
        .map_or_else(|| format!("day {day}"), |civil| civil.to_string())
}

/// The one sentence a day gets before its buckets, if it gets one.
///
/// A day outside the measured calendar says so, as it always did. The two new
/// arms are ONCE PER DAY, AND NOT A COVERAGE FAULT (GAP12-12, D-0955). An
/// `OpenLengthUnmeasured` day traded for a length the calendar does not state,
/// so no minute count could ever complete a bucket; each bucket used to be
/// reported as incomplete coverage, which `ingest::derive` rewrote into
/// "restore complete minute source", a repair that does not exist, once per
/// bucket. Bars on a measured `Closed` day are a defect in what was stored, not
/// a hole in it. Neither sentence ends in the suffix `derive` rewrites.
fn day_note(
    day: i64,
    calendar: crate::calendar::DayKind,
    session: crate::calendar::DayKind,
    runtime: crate::calendar::Runtime<'_>,
) -> Option<Note> {
    use crate::calendar::DayKind;
    let (why, text) = if calendar == DayKind::Unmeasured {
        (
            Withheld::Unverified,
            format!(
                "day {day}: UNVERIFIED: {}; derived buckets withheld",
                runtime.unverified_reason(day)
            ),
        )
    } else {
        match session {
            DayKind::OpenLengthUnmeasured => (
                Withheld::Unsized,
                format!(
                    "day {day}: session length unmeasured (a traded non-regular session whose minute length the calendar does not state); derived buckets withheld; nothing to repair in the minute source"
                ),
            ),
            DayKind::Closed => (
                Withheld::Closed,
                format!(
                    "day {day}: minute bars on a day the calendar measures as closed; store defect; derived buckets withheld"
                ),
            ),
            DayKind::Open(_) | DayKind::Unmeasured => return None,
        }
    };
    Some(Note {
        why,
        day,
        buckets: 0,
        text,
    })
}

/// Whether [`day_note`] or [`day_session`] already named every bucket of this
/// day. `Unmeasured` is reached here only when [`minute_session`] refused a
/// calendar-open day (OD-1, D-2370): a day the calendar itself leaves
/// unmeasured is skipped before this test.
const fn day_is_withheld(session: crate::calendar::DayKind) -> bool {
    matches!(
        session,
        crate::calendar::DayKind::OpenLengthUnmeasured
            | crate::calendar::DayKind::Closed
            | crate::calendar::DayKind::Unmeasured
    )
}

/// The calendar kind and venue session of one IST day, looked up once for the
/// day rather than once for each of its buckets (OD-1, D-2370). A day the venue
/// cannot attest — NSE cash on or after 2026-08-03 with no dated eligibility
/// schedule, or a schedule missing that day — is named in ONE line and its
/// session is `Unmeasured`, which [`day_is_withheld`] withholds with no
/// per-bucket line. Before D-2370 every bucket of such a day pushed two lines
/// and re-derived the venue's hours, and `cli::fold_audit` counted `withheld`
/// from that inflated total.
///
/// A special session the runtime calendar leaves unsized is folded inside
/// its recorded window ([`crate::session::with_recorded_window`], D-3319,
/// D-3321 item 1), the same answer the landing judged its minutes by.
fn day_session(
    day: i64,
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
    runtime: crate::calendar::Runtime<'_>,
    notes: &mut Vec<Note>,
) -> (crate::calendar::DayKind, crate::calendar::DayKind) {
    let calendar = crate::session::with_recorded_window(day, runtime.kind_of(day));
    let session = match minute_session(day, calendar, venue, cash_schedule) {
        Ok(session) => session,
        Err(why) => {
            notes.push(Note {
                why: Withheld::Unattested,
                day,
                buckets: 0,
                text: format!("day {day}: {why}; derived buckets withheld"),
            });
            crate::calendar::DayKind::Unmeasured
        }
    };
    (calendar, session)
}

/// The IST civil day a bucket starting at `start` falls on, as days since the
/// epoch, and that day's midnight as UTC microseconds.
///
/// Split out of [`complete_minutes_with_calendar`] so that function stays under
/// the workspace's line limit once it skips an unmeasured day's buckets
/// (o1api-44, D-1201). The arithmetic is the one it did inline: checked, and an
/// overflow is the same `AnchorOverflow` naming the bucket's start.
fn ist_day_of(start: i64) -> Result<(i64, i64), FoldError> {
    const DAY: i64 = 86_400_000_000;
    const OFFSET: i64 = crate::session::IST_OFFSET_SECS * 1_000_000;
    let overflow = || FoldError::AnchorOverflow { ts_micros: start };
    let day = start
        .checked_add(OFFSET)
        .ok_or_else(overflow)?
        .div_euclid(DAY);
    let midnight = day
        .checked_mul(DAY)
        .and_then(|t| t.checked_sub(OFFSET))
        .ok_or_else(overflow)?;
    Ok((day, midnight))
}

/// The calendar decides which days are regular; the venue decides their hours.
pub(crate) fn minute_session(
    day: i64,
    calendar: crate::calendar::DayKind,
    venue: crate::vendor::Venue,
    cash_schedule: Option<&crate::cash_auction::Schedule>,
) -> Result<crate::calendar::DayKind, String> {
    use crate::calendar::{DayKind, Session, Window};
    if calendar != DayKind::Open(Session::full()) {
        return Ok(calendar);
    }
    let day = u32::try_from(day)
        .map_err(|why| why.to_string())
        .and_then(|day| crate::session::Day::from_days(day).map_err(|why| why.to_string()))?;
    let hours = venue.hours_on(day).map_err(|why| why.to_string())?;
    if hours.kind() != crate::vendor::SessionKind::Continuous
        || hours.open_minute() != u32::from(crate::calendar::OPEN_MINUTE)
    {
        return Err(format!(
            "{venue}: session hours cannot attest the fixed stored grid"
        ));
    }
    let from = u16::try_from(hours.open_minute()).map_err(|why| why.to_string())?;
    let close = if venue == crate::vendor::Venue::NseCash
        && crate::vendor::cash_auction_eligibility_required(day)
    {
        u32::from(
            cash_schedule
                .ok_or_else(|| {
                    format!("cash session UNVERIFIED on {day}: dated eligibility required")
                })?
                .close(day)?,
        )
    } else {
        hours.close_minute()
    };
    let to = close
        .checked_sub(1)
        .and_then(|last| u16::try_from(last).ok())
        .ok_or_else(|| format!("{venue}: invalid exclusive session close"))?;
    Ok(DayKind::Open(Session {
        windows: [Window { from, to }, Window { from: 0, to: 0 }],
        count: 1,
    }))
}

/// At most `MAX_WINDOWS` intersections, independent of the timestamp span.
/// Wide intermediates keep boundary arithmetic exact at either i64 extreme.
fn scheduled_minutes(
    session: crate::calendar::DayKind,
    midnight: i64,
    start: i64,
    end: i64,
) -> i128 {
    const MINUTE: i128 = 60_000_000;
    match session {
        crate::calendar::DayKind::Open(s) => s
            .windows
            .iter()
            .take(usize::from(s.count))
            .map(|w| {
                let from =
                    (i128::from(midnight) + i128::from(w.from) * MINUTE).max(i128::from(start));
                let to =
                    (i128::from(midnight) + (i128::from(w.to) + 1) * MINUTE).min(i128::from(end));
                (to - from).max(0) / MINUTE
            })
            .sum(),
        _ => 0,
    }
}

// ═══════════════ the open interest of a one-second bar (D-3342) ═══════════════

/// One placed row of a vendor tick file, as a one-second fold under D-3342
/// reads it: a traded row is its second's price and volume, and every row,
/// traded or not, is its second's open interest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// The row at its placed second, a price bar of one row. An untraded
    /// row's prices and volume are never read.
    pub bar: Bar,
    /// Whether the row records a trade (GDFL: LTQ above zero).
    pub traded: bool,
}

/// Folds placed rows into one-second bars whose open interest is the OI of
/// the LAST row placed in the second, traded or not (D-3342, the operator's
/// answer: bar-close semantics).
///
/// # The defect, measured
///
/// D-2802 says a bar's open interest is "the last one of the second", and
/// the import handed [`fold`] its traded rows only, so the OI was the last
/// TRADED row's: of 545,134 GDFL option bars measured, 235 had a later
/// untraded row in the same second with a different OI
/// (BANKNIFTY28MAR1926500CE at 14:14:30: 6,420 against 6,400), and an OI
/// update carried only by untraded rows never reached the store (DB-083).
///
/// # The rule
///
/// - Open, high, low, close and volume are [`fold`]'s over the traded rows
///   of the second, in file order, exactly as before.
/// - The open interest is the last row of the second, in placement order,
///   that carried one ([`i64::MIN`] is the null and carries none), whether
///   or not it traded.
/// - A second with no traded row has no bar, whatever its untraded rows say.
///
/// With every row traded this is `fold(rows, Bucket::SECOND)` bar for bar.
/// Wiring it into the GDFL import changes the bars a file builds, so it
/// lands with `BAR_DEFINITION` 3 and the sidecar that records which
/// definition built which day (D-3342, D-3334): WP-11b's change.
///
/// # Errors
///
/// [`FoldError::OutOfOrder`] for a row stamped before the one before it,
/// [`FoldError::NegativeVolume`] for a traded row below zero and
/// [`FoldError::VolumeOverflow`] for a second's sum past `i64::MAX`, each
/// naming the row's position. An untraded row's volume is never read.
///
/// # Cost
///
/// One pass, O(rows), and one push per second that traded.
pub fn fold_seconds_with_last_oi(rows: &[Placed]) -> Result<Vec<Bar>, FoldError> {
    const SECOND: i64 = 1_000_000;
    let mut out: Vec<Bar> = Vec::with_capacity(rows.len());
    let mut previous: Option<i64> = None;
    // The second being folded, whether a traded row opened its bar, and the
    // last OI any of its rows carried.
    let mut second: Option<i64> = None;
    let mut opened = false;
    let mut oi = i64::MIN;
    for (at, row) in rows.iter().enumerate() {
        let ts = row.bar.ts_micros;
        if let Some(prev) = previous
            && ts < prev
        {
            return Err(FoldError::OutOfOrder {
                at,
                previous: prev,
                found: ts,
            });
        }
        previous = Some(ts);
        let start = ts
            .div_euclid(SECOND)
            .checked_mul(SECOND)
            .ok_or(FoldError::AnchorOverflow { ts_micros: ts })?;
        if second != Some(start) {
            close_second(&mut out, opened, oi);
            second = Some(start);
            opened = false;
            oi = i64::MIN;
        }
        if row.bar.open_interest != i64::MIN {
            oi = row.bar.open_interest;
        }
        if !row.traded {
            continue;
        }
        if row.bar.volume < 0 {
            return Err(FoldError::NegativeVolume {
                at,
                volume: row.bar.volume,
            });
        }
        if let Some(bar) = out.last_mut().filter(|_| opened) {
            bar.high = bar.high.max(row.bar.high);
            bar.low = bar.low.min(row.bar.low);
            bar.close = row.bar.close;
            bar.volume = bar
                .volume
                .checked_add(row.bar.volume)
                .ok_or(FoldError::VolumeOverflow { at, bucket: start })?;
        } else {
            opened = true;
            out.push(Bar {
                ts_micros: start,
                ..row.bar
            });
        }
    }
    close_second(&mut out, opened, oi);
    Ok(out)
}

/// Stamps the second just folded with the last OI any of its rows carried.
fn close_second(out: &mut [Bar], opened: bool, oi: i64) {
    if let Some(bar) = out.last_mut().filter(|_| opened) {
        bar.open_interest = oi;
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod guard {
    use super::{Bucket, FoldError, fold_from_bars, fold_from_snapshots};
    use crate::calendar::{DayKind, Runtime};
    use crate::session::{Day, IST_OFFSET_SECS};
    use crate::vendor::Venue;
    use store::format::Bar;

    /// The first one-minute bar of a regular session on `day`, 09:15 IST, as
    /// a UTC epoch second.
    fn open_of(day: Day) -> i64 {
        i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60
    }

    /// `n` one-minute bars from `from`, a second apart by `step`.
    fn minutes(from: i64, n: i64, step: i64) -> Vec<Bar> {
        (0..n)
            .map(|m| Bar {
                ts_micros: (from + m * step) * 1_000_000,
                open: 100,
                high: 100,
                low: 100,
                close: 100,
                volume: 1,
                open_interest: i64::MIN,
            })
            .collect()
    }

    /// **A DAY OUTSIDE THE CALENDAR IS ONE SENTENCE, AT EVERY WIDTH.**
    /// o1api-44, D-1201.
    ///
    /// Measured on main before the fix: five weekdays from 2026-09-07, 1,800
    /// minute bars, two-minute rung, gave `complete = 0` and 905 diagnostics,
    /// 158,085 bytes: one day sentence each and one "incomplete or invalid
    /// minute coverage" line per bucket, every one a `pull.derive` warning.
    /// Checked here at every derived rung width, after the calendar's last day
    /// and before its first, with a full session, a single bar, misaligned
    /// seconds and duplicate stamps on the unmeasured days, and with measured
    /// days either side whose own coverage faults must still be named.
    ///
    /// The days are past the SOURCED calendar's last day (2026-12-31) and
    /// before its first (2018-01-01): since D-3346 the ingest calendar answers
    /// the D-2805 spans, so 2026-09-07..=11 and 2019-11-29, which this used,
    /// are regular sessions to it now.
    #[test]
    fn an_unmeasured_day_is_one_diagnostic_and_not_one_per_bucket() {
        let after: Vec<Day> = (4..=8).map(|d| Day::new(2027, 1, d).unwrap()).collect();
        for day in &after {
            assert_eq!(
                Runtime::default().kind_of(i64::from(day.days_from_epoch())),
                DayKind::Unmeasured,
                "the premise: {day:?} is past the sourced calendar's last day (D-3346)"
            );
        }
        let mut bars = Vec::new();
        for day in &after {
            bars.extend(minutes(open_of(*day), 375, 60));
        }
        let widths = [60_u32, 120, 180, 300, 900, 2_700, 3_600, 14_400, 86_400];
        for secs in widths {
            let bucket = Bucket::of_secs(secs).unwrap();
            for venue in Venue::ALL {
                let (complete, diagnostics) = super::complete_minutes_with_calendar(
                    &bars,
                    bucket,
                    venue,
                    None,
                    Runtime::default(),
                )
                .unwrap();
                assert!(complete.is_empty(), "{secs}s {venue}: nothing certified");
                assert_eq!(
                    diagnostics.len(),
                    after.len(),
                    "{secs}s {venue}: one sentence per day, not one per bucket: {} \
                     lines, first {:?}",
                    diagnostics.len(),
                    diagnostics.iter().take(7).collect::<Vec<_>>()
                );
                for (why, day) in diagnostics.iter().zip(&after) {
                    assert!(
                        why.starts_with(&format!("day {}: UNVERIFIED", day.days_from_epoch())),
                        "{secs}s {venue}: {why}"
                    );
                }
                let bytes: usize = diagnostics.iter().map(String::len).sum();
                assert!(bytes < 1_000, "{secs}s {venue}: {bytes} bytes");
            }
        }

        // One bar, misaligned seconds and a duplicated stamp on an unmeasured
        // day are each still that day's one sentence, not a coverage fault.
        let day = after[0];
        let mut odd = minutes(open_of(day), 1, 60);
        odd.extend(minutes(open_of(day) + 61, 3, 17));
        odd.extend(minutes(open_of(day) + 600, 2, 0));
        let (complete, diagnostics) = super::complete_minutes_with_calendar(
            &odd,
            Bucket::of_secs(120).unwrap(),
            Venue::NseIndex,
            None,
            Runtime::default(),
        )
        .unwrap();
        assert!(complete.is_empty());
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");

        // Before the calendar's first day as well as after its last.
        let early = Day::new(2017, 12, 29).unwrap();
        assert_eq!(
            Runtime::default().kind_of(i64::from(early.days_from_epoch())),
            DayKind::Unmeasured
        );
        let (complete, diagnostics) = super::complete_minutes_with_calendar(
            &minutes(open_of(early), 375, 60),
            Bucket::of_secs(120).unwrap(),
            Venue::NseIndex,
            None,
            Runtime::default(),
        )
        .unwrap();
        assert!(complete.is_empty());
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    }

    /// **A MEASURED DAY BESIDE AN UNMEASURED ONE KEEPS EVERY LINE OF ITS OWN.**
    /// o1api-44, D-1201: the skip is for the unmeasured day only. A whole
    /// session on 2026-09-04, the calendar's last day, still completes, and a
    /// hole in it is still named bucket by bucket, ahead of the next day's one
    /// sentence. The next day is 2027-01-04, past the sourced span (D-3346).
    #[test]
    fn a_measured_day_beside_an_unmeasured_one_keeps_its_own_lines() {
        let after = [Day::new(2027, 1, 4).unwrap()];
        let last = Day::new(2026, 9, 4).unwrap();
        assert!(matches!(
            Runtime::default().kind_of(i64::from(last.days_from_epoch())),
            DayKind::Open(_)
        ));
        let mut mixed = minutes(open_of(last), 375, 60);
        mixed.extend(minutes(open_of(after[0]), 375, 60));
        let (complete, diagnostics) = super::complete_minutes_with_calendar(
            &mixed,
            Bucket::of_secs(300).unwrap(),
            Venue::NseIndex,
            None,
            Runtime::default(),
        )
        .unwrap();
        assert_eq!(complete.len(), 75, "the measured day completes whole");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        let mut holed = minutes(open_of(last), 375, 60);
        holed.remove(100);
        holed.extend(minutes(open_of(after[0]), 375, 60));
        let (complete, diagnostics) = super::complete_minutes_with_calendar(
            &holed,
            Bucket::of_secs(300).unwrap(),
            Venue::NseIndex,
            None,
            Runtime::default(),
        )
        .unwrap();
        assert_eq!(complete.len(), 74, "only the holed bucket is withheld");
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        assert!(diagnostics[0].contains("incomplete or invalid minute coverage: observed 4"));
        assert!(diagnostics[1].contains("UNVERIFIED"));
    }

    /// Minute bars at `minutes` past IST midnight on `day`.
    fn at_minutes(day: Day, minutes: impl IntoIterator<Item = i64>) -> Vec<Bar> {
        let midnight = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS;
        minutes
            .into_iter()
            .map(|m| Bar {
                ts_micros: (midnight + m * 60) * 1_000_000,
                open: 100,
                high: 101,
                low: 99,
                close: 100,
                volume: 2,
                open_interest: i64::MIN,
            })
            .collect()
    }

    /// The bucket starts of `bars`, as minutes past IST midnight on `day`.
    fn starts(day: Day, bars: &[Bar]) -> Vec<i64> {
        let midnight = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS;
        bars.iter()
            .map(|b| (b.ts_micros / 1_000_000 - midnight) / 60)
            .collect()
    }

    fn derive(bars: &[Bar], secs: u32, venue: Venue) -> super::Derived {
        super::derive_minutes(
            bars,
            Bucket::of_secs(secs).unwrap(),
            venue,
            None,
            Runtime::default(),
        )
        .unwrap()
    }

    /// **A DISASTER-RECOVERY SATURDAY FOLDS INSIDE ITS WINDOWS (D-3321 item
    /// 1, DB-013).** BANKNIFTY 2024-03-02 held 105 minutes, 09:15-09:59 and
    /// 11:30-12:29, and every derived rung of it was withheld ("fixed stored
    /// grid cannot attest session alignment"). On the regular 09:15 grid each
    /// bucket owes the window minutes it overlaps: 21 five-minute bars, and
    /// three hour bars owing 45, 45 and 15 minutes. A minute missing from a
    /// window still withholds its bucket, and nothing in the halt is owed.
    #[test]
    fn a_disaster_recovery_saturday_folds_inside_its_windows() {
        let day = Day::new(2024, 3, 2).unwrap();
        let mut bars = at_minutes(day, 555..600);
        bars.extend(at_minutes(day, 690..750));
        assert_eq!(bars.len(), 105);
        for venue in [Venue::NseIndex, Venue::NseCash] {
            let five = derive(&bars, 300, venue);
            assert_eq!(five.bars.len(), 21, "{venue}: {:?}", five.diagnostics);
            assert!(five.diagnostics.is_empty(), "{:?}", five.diagnostics);
            assert_eq!(
                five.tally,
                super::Tally {
                    days: 1,
                    buckets: 21,
                    withheld: 0,
                    partial: 0
                }
            );
            let hour = derive(&bars, 3_600, venue);
            assert_eq!(starts(day, &hour.bars), [555, 675, 735], "{venue}");
            assert_eq!(hour.bars[0].volume, 90, "45 minutes of volume 2");
            assert_eq!(hour.bars[1].volume, 90, "11:30-12:14");
            assert_eq!(hour.bars[2].volume, 30, "12:15-12:29");
        }
        let mut holed = bars.clone();
        holed.remove(50);
        let five = derive(&holed, 300, Venue::NseIndex);
        assert_eq!(five.bars.len(), 20);
        assert_eq!(five.tally.withheld, 1);
        assert_eq!(five.diagnostics.len(), 1);
        assert!(five.diagnostics[0].contains("observed 4, scheduled 5"));
    }

    /// **EVERY MUHURAT FOLDS INSIDE ITS RECORDED WINDOW (D-3319, D-3321).**
    /// 2020-11-14 traded 18:15-19:14 and the ingest calendar answers it
    /// unsized, so every derived bucket of it was withheld. 2024-11-01 opened
    /// at 18:00, off the hour grid: its first hour bar starts at 17:15 and
    /// owes 18:00-18:14, and its two-minute rung opens with a one-minute
    /// bucket at 17:59, the same stub rule as a regular session's close.
    #[test]
    fn a_muhurat_folds_inside_its_recorded_window() {
        let evening = Day::new(2020, 11, 14).unwrap();
        assert_eq!(
            Runtime::default().kind_of(i64::from(evening.days_from_epoch())),
            DayKind::OpenLengthUnmeasured,
            "the premise: the ingest calendar leaves it unsized"
        );
        let bars = at_minutes(evening, 1_095..1_155);
        let five = derive(&bars, 300, Venue::NseIndex);
        assert_eq!(five.bars.len(), 12, "{:?}", five.diagnostics);
        assert_eq!(
            five.tally,
            super::Tally {
                days: 1,
                buckets: 12,
                withheld: 0,
                partial: 0
            }
        );
        let hour = derive(&bars, 3_600, Venue::NseIndex);
        assert_eq!(starts(evening, &hour.bars), [1_095]);

        let off_grid = Day::new(2024, 11, 1).unwrap();
        let bars = at_minutes(off_grid, 1_080..1_140);
        let hour = derive(&bars, 3_600, Venue::NseIndex);
        assert_eq!(
            starts(off_grid, &hour.bars),
            [1_035, 1_095],
            "{:?}",
            hour.diagnostics
        );
        assert_eq!((hour.bars[0].volume, hour.bars[1].volume), (30, 90));
        let two = derive(&bars, 120, Venue::NseIndex);
        assert_eq!(two.bars.len(), 31, "{:?}", two.diagnostics);
        assert_eq!(starts(off_grid, &two.bars)[..2], [1_079, 1_081]);
        // The first window minute missing withholds its bucket by name, and
        // the bucket before the window owes nothing.
        let late = derive(&bars[1..], 3_600, Venue::NseIndex);
        assert_eq!(starts(off_grid, &late.bars), [1_095]);
        assert_eq!(late.tally.withheld, 1);
        assert!(
            late.diagnostics[0].contains("observed 14, scheduled 15"),
            "{:?}",
            late.diagnostics
        );
    }

    /// **A SHARE'S MINUTE WITH NO CANDLE IS NO TRADE; AN INDEX'S IS MISSING
    /// SOURCE (D-3321 item 2, DB-G36).** One minute missing from a regular
    /// session: the index withholds its five-minute bucket and names it, the
    /// share folds the bucket over its four traded minutes and counts it
    /// partial. A whole bucket with no candle is a named absence for the
    /// index and simply no bar for the share, and so is a session tail.
    #[test]
    fn a_shares_no_trade_minute_folds_partial_and_an_indexs_is_withheld() {
        let day = Day::new(2025, 7, 1).unwrap();
        let mut one_missing = at_minutes(day, 555..930);
        one_missing.remove(5); // 09:20
        let index = derive(&one_missing, 300, Venue::NseIndex);
        assert_eq!(index.bars.len(), 74);
        assert_eq!(
            index.tally,
            super::Tally {
                days: 1,
                buckets: 74,
                withheld: 1,
                partial: 0
            }
        );
        assert!(index.diagnostics[0].contains("observed 4, scheduled 5"));
        let share = derive(&one_missing, 300, Venue::NseCash);
        assert_eq!(share.bars.len(), 75);
        assert!(share.diagnostics.is_empty(), "{:?}", share.diagnostics);
        assert_eq!(
            share.tally,
            super::Tally {
                days: 1,
                buckets: 75,
                withheld: 0,
                partial: 1
            }
        );
        let partial = share.bars[1];
        assert_eq!(partial.volume, 8, "four traded minutes, no invented fifth");
        // The hour bar holding 09:20 is partial too, at every rung.
        let hour = derive(&one_missing, 3_600, Venue::NseCash);
        assert_eq!(hour.tally.partial, 1);
        assert_eq!(hour.bars.len(), 7);

        let mut no_bucket = at_minutes(day, 555..930);
        no_bucket.drain(5..10); // 09:20-09:24
        let index = derive(&no_bucket, 300, Venue::NseIndex);
        assert_eq!(index.tally.withheld, 1);
        assert!(index.diagnostics[0].contains("absent: observed 0, scheduled 5"));
        let share = derive(&no_bucket, 300, Venue::NseCash);
        assert_eq!(
            share.tally,
            super::Tally {
                days: 1,
                buckets: 74,
                withheld: 0,
                partial: 0
            }
        );
        assert!(share.diagnostics.is_empty());

        // A share's minute outside its session still withholds its bucket:
        // only a missing minute is read as no trade.
        let mut stray = at_minutes(day, 555..930);
        stray.extend(at_minutes(day, 935..936));
        let share = derive(&stray, 300, Venue::NseCash);
        assert_eq!(share.tally.withheld, 1, "{:?}", share.diagnostics);
    }

    /// **ONE DIAGNOSTIC PER REASON, COUNTED (D-3341, DB-G88).** Three holes
    /// on one index day are three bucket lines in the detailed form and one
    /// line in the derive's: it counts the buckets and days, names the days,
    /// quotes the first, and still ends with the suffix `ingest::derive`
    /// rewrites. Five days past the calendar are one line of 375 buckets.
    #[test]
    fn the_derive_counts_repeated_notes_instead_of_repeating_them() {
        let day = Day::new(2025, 7, 1).unwrap();
        let mut holed = at_minutes(day, 555..930);
        for at in [300, 200, 100] {
            holed.remove(at);
        }
        let (_, detailed) = super::complete_minutes(&holed, Bucket::of_secs(300).unwrap()).unwrap();
        assert_eq!(detailed.len(), 3);
        let derived = derive(&holed, 300, Venue::NseIndex);
        assert_eq!(
            derived.tally,
            super::Tally {
                days: 1,
                buckets: 72,
                withheld: 3,
                partial: 0
            }
        );
        assert_eq!(derived.diagnostics.len(), 1, "{:?}", derived.diagnostics);
        let line = &derived.diagnostics[0];
        assert!(
            line.starts_with(
                "incomplete or invalid minute coverage: 3 bucket(s) withheld on 1 day(s), 2025-07-01..2025-07-01, over 3 note(s); the first: bucket "
            ),
            "{line}"
        );
        assert!(line.ends_with("; historical gap refill requires a versioned store repair"));

        let after: Vec<Day> = (4..=8).map(|d| Day::new(2027, 1, d).unwrap()).collect();
        let mut bars = Vec::new();
        for day in &after {
            bars.extend(at_minutes(*day, 555..930));
        }
        let derived = derive(&bars, 300, Venue::NseIndex);
        assert!(derived.bars.is_empty());
        assert_eq!(
            derived.tally,
            super::Tally {
                days: 0,
                buckets: 0,
                withheld: 375,
                partial: 0
            }
        );
        assert_eq!(derived.diagnostics.len(), 1, "{:?}", derived.diagnostics);
        assert!(
            derived.diagnostics[0].starts_with(
                "days outside the calendar's authority: 375 bucket(s) withheld on 5 day(s), 2027-01-04..2027-01-08"
            ),
            "{}",
            derived.diagnostics[0]
        );
        assert_eq!(super::Withheld::Unverified.code(), "calendar_unverified");
        assert_eq!(super::Withheld::Incomplete.code(), "incomplete_minutes");
    }

    /// **A DAY WITH NO RECORDED WINDOW IS WITHHELD ONCE, FOR ITS OWN REASON**
    /// (GAP12-12, D-0955). No such day is in the calendar since D-3319, so the
    /// day's sentence and its counting are held here: one note per unsized
    /// day, never a coverage fault `ingest::derive` would rewrite into a
    /// repair, every bucket counted under it, and one counted line per reason
    /// in the derive's form. A closed day's sentence is a store defect; a
    /// regular day has none.
    #[test]
    fn an_unsized_day_is_one_note_for_its_own_reason_and_counted() {
        use crate::calendar::Session;
        let open = DayKind::Open(Session::full());
        let rt = Runtime::default();
        let note = super::day_note(18_580, open, DayKind::OpenLengthUnmeasured, rt).unwrap();
        assert_eq!(note.why, super::Withheld::Unsized);
        assert!(
            note.text
                .starts_with("day 18580: session length unmeasured")
        );
        assert!(
            !note.text.contains("historical gap refill"),
            "{}",
            note.text
        );
        assert!(super::day_is_withheld(DayKind::OpenLengthUnmeasured));
        let closed = super::day_note(18_581, open, DayKind::Closed, rt).unwrap();
        assert_eq!(closed.why, super::Withheld::Closed);
        assert!(closed.text.contains("store defect"));
        assert!(super::day_note(18_582, open, open, rt).is_none());
        let unverified =
            super::day_note(30_000, DayKind::Unmeasured, DayKind::Unmeasured, rt).unwrap();
        assert_eq!(unverified.why, super::Withheld::Unverified);

        let notes: Vec<super::Note> = [18_580, 18_580, 18_587]
            .iter()
            .map(|day| super::Note {
                buckets: 12,
                ..super::day_note(*day, open, DayKind::OpenLengthUnmeasured, rt).unwrap()
            })
            .collect();
        let lines = super::aggregate(&notes);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(
            lines[0].starts_with(
                "traded sessions whose length no record states: 36 bucket(s) withheld on 2 day(s), 2020-11-14..2020-11-21, over 3 note(s); the first: day 18580"
            ),
            "{}",
            lines[0]
        );
        assert_eq!(super::aggregate(&notes[..1]), [notes[0].text.clone()]);
        let codes = [
            super::Withheld::Incomplete,
            super::Withheld::Absent,
            super::Withheld::AbsentTail,
            super::Withheld::Unverified,
            super::Withheld::Unsized,
            super::Withheld::Closed,
            super::Withheld::Unattested,
        ]
        .map(super::Withheld::code);
        let mut unique = codes.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), 7, "a code is never shared: {codes:?}");
        assert_eq!(super::civil(18_580), "2020-11-14");
        assert_eq!(super::civil(-1), "day -1");
    }

    /// One placed row for [`super::fold_seconds_with_last_oi`].
    fn placed(secs: i64, price: i64, volume: i64, oi: i64, traded: bool) -> super::Placed {
        super::Placed {
            bar: Bar {
                ts_micros: secs * 1_000_000,
                open: price,
                high: price,
                low: price,
                close: price,
                volume,
                open_interest: oi,
            },
            traded,
        }
    }

    /// **A SECOND'S OPEN INTEREST IS ITS LAST ROW'S, TRADED OR NOT (D-3342,
    /// DB-083).** The measured case: a traded row at 14:14:30 carrying 6,400
    /// and a later untraded row in the same second carrying 6,420. The bar is
    /// the traded row's price and volume and the untraded row's OI. A second
    /// with only untraded rows has no bar; a null OI carries nothing; and with
    /// every row traded the fold is `fold(.., SECOND)` bar for bar.
    #[test]
    fn a_seconds_open_interest_is_its_last_rows_traded_or_not() {
        let t = 1_553_762_670; // an instant; the values are invented
        let rows = [
            placed(t, 2_000, 15, 6_400, true),
            placed(t, 1_990, 0, 6_420, false),
            placed(t + 1, 1_980, 0, 6_440, false),
            placed(t + 2, 2_010, 30, 6_440, true),
            placed(t + 2, 2_030, 45, i64::MIN, true),
            placed(t + 3, 2_020, 0, i64::MIN, false),
        ];
        let bars = super::fold_seconds_with_last_oi(&rows).unwrap();
        assert_eq!(bars.len(), 2, "no bar for a second that did not trade");
        assert_eq!(
            (bars[0].close, bars[0].volume, bars[0].open_interest),
            (2_000, 15, 6_420),
            "the untraded row's OI, the traded row's price"
        );
        assert_eq!(
            (bars[1].open, bars[1].high, bars[1].close, bars[1].volume),
            (2_010, 2_030, 2_030, 75)
        );
        assert_eq!(bars[1].open_interest, 6_440, "a null OI carries nothing");

        let traded: Vec<super::Placed> = rows
            .iter()
            .map(|r| super::Placed { traded: true, ..*r })
            .collect();
        let plain: Vec<Bar> = traded.iter().map(|r| r.bar).collect();
        assert_eq!(
            super::fold_seconds_with_last_oi(&traded).unwrap(),
            fold_from_snapshots(&plain, Bucket::SECOND).unwrap()
        );
        assert!(super::fold_seconds_with_last_oi(&[]).unwrap().is_empty());

        // The refusals name the row, and an untraded row's volume is never read.
        let backwards = [placed(t + 1, 1, 1, 1, true), placed(t, 1, 1, 1, false)];
        assert!(matches!(
            super::fold_seconds_with_last_oi(&backwards),
            Err(FoldError::OutOfOrder { at: 1, .. })
        ));
        let negative = [placed(t, 1, -3, 1, true)];
        assert_eq!(
            super::fold_seconds_with_last_oi(&negative),
            Err(FoldError::NegativeVolume { at: 0, volume: -3 })
        );
        assert!(
            super::fold_seconds_with_last_oi(&[placed(t, 1, -3, 1, false)])
                .unwrap()
                .is_empty()
        );
        let overflow = [placed(t, 1, i64::MAX, 1, true), placed(t, 1, 1, 1, true)];
        assert!(matches!(
            super::fold_seconds_with_last_oi(&overflow),
            Err(FoldError::VolumeOverflow { at: 1, .. })
        ));
        let edge = [super::Placed {
            bar: Bar {
                ts_micros: i64::MIN,
                ..rows[0].bar
            },
            traded: true,
        }];
        assert!(matches!(
            super::fold_seconds_with_last_oi(&edge),
            Err(FoldError::AnchorOverflow { .. })
        ));
    }

    /// Every timeframe the operator named is a whole number of minutes, and
    /// every one of them is exact from stored one-minute bars.
    #[test]
    fn every_whole_minute_width_is_allowed_from_minute_bars() {
        for (label, secs) in [
            ("1 minute", 60u32),
            ("2 minute", 120),
            ("3 minute", 180),
            ("5 minute", 300),
            ("15 minute", 900),
            ("45 minute", 2_700),
            ("1 hour", 3_600),
            ("4 hour", 14_400),
            ("1 day", 86_400),
        ] {
            let want = Bucket::of_secs(secs).expect("non-zero");
            assert!(
                fold_from_bars(&[], want, Bucket::MINUTE).is_ok(),
                "{label} is {secs}s, a whole multiple of 60 — it must be allowed"
            );
        }
    }

    /// A width that is not a whole multiple is REFUSED, not mis-attributed.
    ///
    /// Measured on the operator's archive across all 12,132 contracts: a 90s
    /// bucket built from minute bars got **20.4% of bars wrong**, worst error
    /// Rs 1,147.90, and one case pulled a trade stamped 12:56:58 into a bucket
    /// that ended 12:56:40 — eighteen seconds of the future inside a closed
    /// bar. That is why this refuses rather than approximates.
    #[test]
    fn a_sub_minute_width_from_minute_bars_is_refused_by_name() {
        for secs in [1u32, 7, 30, 90, 100, 3_607] {
            let want = Bucket::of_secs(secs).expect("non-zero");
            assert_eq!(
                fold_from_bars(&[], want, Bucket::MINUTE),
                Err(FoldError::NarrowerThanSource {
                    want_secs: secs,
                    source_secs: 60,
                }),
                "{secs}s is not a whole multiple of 60"
            );
        }

        let text = fold_from_bars(&[], Bucket::of_secs(90).expect("nz"), Bucket::MINUTE)
            .expect_err("90 is not a multiple of 60")
            .to_string();
        assert!(
            text.contains("snapshots"),
            "the refusal must say where the width IS available — nothing is \
             forbidden, only this one lossy path. Got {text:?}"
        );
    }

    /// From snapshots, EVERY width is exact. Nothing is locked down.
    #[test]
    fn any_width_at_all_is_allowed_from_snapshots() {
        for secs in [1u32, 7, 30, 90, 100, 60, 300, 3_607, 86_400] {
            let want = Bucket::of_secs(secs).expect("non-zero");
            assert!(
                fold_from_snapshots(&[], want).is_ok(),
                "{secs}s from raw snapshots is exact — a snapshot carries its \
                 own instant, so no width can misattribute it"
            );
        }
    }

    /// Folding minute bars into wider ones, from a five-minute source too.
    #[test]
    fn the_guard_is_about_the_source_not_about_minutes() {
        let five = Bucket::of_secs(300).expect("nz");
        // 15m and 1h are whole multiples of 5m — allowed.
        assert!(fold_from_bars(&[], Bucket::of_secs(900).expect("nz"), five).is_ok());
        assert!(fold_from_bars(&[], Bucket::of_secs(3_600).expect("nz"), five).is_ok());
        // 1m is NARROWER than 5m — the information is gone.
        assert!(fold_from_bars(&[], Bucket::MINUTE, five).is_err());
        // 3m is not a multiple of 5m even though both are whole minutes.
        assert!(
            fold_from_bars(&[], Bucket::of_secs(180).expect("nz"), five).is_err(),
            "3 minutes is not a whole multiple of 5 minutes — a bucket edge \
             would fall inside a source bar"
        );
    }
}

/// The intraday grid restarts at each IST day's 09:15 (FIB-8, D-6446).
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]
mod open_grid {
    use super::{Bucket, IST_ANCHOR_MICROS, OPEN_ANCHOR_MICROS, bucket_start, fold};
    use store::format::Bar;

    /// The epoch grid the fold used before D-6446.
    fn epoch_grid(ts: i64, width_secs: u32) -> i64 {
        let width = i64::from(width_secs) * 1_000_000;
        (ts + OPEN_ANCHOR_MICROS).div_euclid(width) * width - OPEN_ANCHOR_MICROS
    }

    /// 09:15 IST of the IST epoch day `day`, in UTC microseconds.
    fn open_of(day: i64) -> i64 {
        day * 86_400_000_000 - IST_ANCHOR_MICROS + (IST_ANCHOR_MICROS - OPEN_ANCHOR_MICROS)
    }

    /// Every width that divides a day keeps the grid it had: every store rung
    /// and every divisor of 86,400 seconds, at instants all through two
    /// years of days, before the open, inside the session and after it.
    #[test]
    fn every_width_that_divides_a_day_keeps_its_grid() {
        let mut widths: Vec<u32> = store::path::Timeframe::KNOWN
            .iter()
            .map(|rung| rung.secs())
            .filter(|secs| *secs < 86_400)
            .collect();
        widths.extend((1..86_400).filter(|secs| 86_400 % secs == 0 && secs % 60 == 0));
        let first = 19_000_i64;
        for width in widths {
            let bucket = Bucket::of_secs(width).unwrap();
            for day in (first..first + 730).step_by(7) {
                for offset_secs in [-33_300_i64, -1, 0, 1, 59, 600, 21_000, 22_440, 52_000] {
                    let ts = open_of(day) + offset_secs * 1_000_000;
                    assert_eq!(
                        bucket_start(ts, bucket),
                        Some(epoch_grid(ts, width)),
                        "width {width}s day {day} offset {offset_secs}s"
                    );
                }
            }
        }
    }

    /// A width that does not divide a day starts at 09:15 every day, where the
    /// epoch grid drifted.
    #[test]
    fn a_width_that_does_not_divide_a_day_starts_at_each_open() {
        let minute = |day: i64, n: i64| Bar {
            ts_micros: open_of(day) + n * 60_000_000,
            open: 100,
            high: 101,
            low: 99,
            close: 100,
            volume: 1,
            open_interest: i64::MIN,
        };
        for width_minutes in [7_u32, 64, 118] {
            let bucket = Bucket::of_secs(width_minutes * 60).unwrap();
            let mut drifted = false;
            for day in 20_000..20_010 {
                let bars: Vec<Bar> = (0..375).map(|n| minute(day, n)).collect();
                let folded = fold(&bars, bucket).unwrap();
                assert_eq!(
                    folded[0].ts_micros,
                    open_of(day),
                    "{width_minutes} day {day}"
                );
                assert_eq!(
                    folded.len(),
                    usize::try_from(375_u32.div_ceil(width_minutes)).unwrap()
                );
                drifted |= epoch_grid(open_of(day), width_minutes * 60) != open_of(day);
            }
            assert!(drifted, "the epoch grid drifts at {width_minutes} minutes");
        }
        // A day bucket keeps IST midnight.
        let day = Bucket::DAY;
        let ts = open_of(20_000);
        assert_eq!(
            bucket_start(ts, day),
            Some(20_000 * 86_400_000_000 - IST_ANCHOR_MICROS)
        );
        assert_eq!(bucket_start(i64::MAX, Bucket::MINUTE), None);
    }
}
