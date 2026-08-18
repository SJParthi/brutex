//! Bars generated in-process, which is the only kind this crate has ever run on.
//!
//! # Why generated and not stored
//!
//! The operator's rule is that a vendor pull is forbidden **and so are the bars
//! already on disk**. That is not a limitation this module works around; it is
//! the reason it is better:
//!
//! * **Reproducible.** Every field is a function of `(day, minute)` alone, so a
//!   run is byte-identical on any machine, forever. `CLAUDE.md` §3 rule 5 asks
//!   for exactly that, and a stored month cannot give it — it makes a test
//!   depend on what happens to be on a disk.
//! * **Complete.** Real bars are a *sample*. Two of the evaluator's refusal
//!   arms — an overflowing bar range and an oversized volume accumulator —
//!   cannot be produced by any bar a market prints: the widest Indian index span
//!   is about 10^7 paisa against an `i64` ceiling of 9.2 x 10^18. On stored data
//!   those branches are unreachable forever and 100% coverage is unreachable
//!   with them. Here they are one call away.
//! * **Shaped on demand.** A one-bar session, a year-long gap, a flat session, a
//!   session that straddles a month boundary — each is a parameter rather than a
//!   search through history for an example.
//!
//! # What it is NOT
//!
//! It is not a market model and nothing here is a claim about prices. The drift
//! and the wobble are arbitrary integers chosen to move the extremes around so
//! the condition families have something to decide. `CLAUDE.md` §3 rule 1 forbids
//! inventing a vendor or an instrument fact, and this module invents no fact —
//! it makes well-formed records whose only guaranteed property is that
//! `Candle::check` accepts them.

use indicators::{Candle, OI_NULL};

/// 09:15 IST expressed as microseconds past midnight UTC.
///
/// 555 minutes into the IST day, less the 330-minute offset. Written as the
/// arithmetic rather than as a constant so the two halves are visible: a reader
/// checking it does not have to trust a magic number.
pub const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;

/// One day, in microseconds.
pub const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;

/// One minute, in microseconds.
pub const MINUTE_MICROS: i64 = 60 * 1_000_000;

/// Bars in a regular NSE session: 09:15 to 15:29 inclusive, at one minute.
pub const BARS_PER_SESSION: usize = 375;

/// A price to build around, in paisa. 25,000.00 index points.
pub const BASE: i64 = 2_500_000;

/// One bar, determined entirely by `(day, minute)`.
///
/// Same arguments, same bar, forever — which is what makes a run over these
/// reproducible byte for byte.
#[must_use]
pub fn bar(day: i64, minute: usize) -> Candle {
    let m = i64::try_from(minute).unwrap_or(0);
    // Integer only. §7 forbids a float here as firmly as in shipping code, and a
    // generator that used one would be unable to produce the same bar on two
    // machines.
    let drift = day.saturating_mul(400).saturating_add(m.saturating_mul(3));
    let wobble = ((m % 7) - 3).saturating_mul(25);
    let close = BASE.saturating_add(drift).saturating_add(wobble);
    let open = close.saturating_sub(wobble);
    let high = open.max(close).saturating_add(60);
    let low = open.min(close).saturating_sub(60);
    Candle::new(
        day.saturating_mul(DAY_MICROS)
            .saturating_add(IST_OPEN_UTC_MICROS)
            .saturating_add(m.saturating_mul(MINUTE_MICROS)),
        open,
        high,
        low,
        close,
        1_000 + (m % 11),
        OI_NULL,
    )
}

/// `count` consecutive regular sessions, one bar per minute.
///
/// Timestamps increase strictly across the whole slice, and each session starts
/// at its own 09:15 — so the evaluator's rollover fires exactly `count - 1`
/// times, which is what fills the five-session ladder.
#[must_use]
pub fn sessions(count: i64) -> Vec<Candle> {
    session_of(count, BARS_PER_SESSION)
}

/// `count` consecutive sessions of `bars_each` bars.
///
/// `bars_each` is a parameter because session length is a real variable: the
/// 2025 Muhurat session was sixty bars, not 375, and a family that assumes 375
/// is wrong on it.
#[must_use]
pub fn session_of(count: i64, bars_each: usize) -> Vec<Candle> {
    let total = usize::try_from(count.max(0))
        .unwrap_or(0)
        .saturating_mul(bars_each);
    let mut out = Vec::with_capacity(total);
    for day in 0..count.max(0) {
        for minute in 0..bars_each {
            out.push(bar(day, minute));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{BARS_PER_SESSION, DAY_MICROS, bar, session_of, sessions};

    #[test]
    fn the_same_arguments_give_the_same_bar() {
        assert_eq!(bar(3, 17), bar(3, 17));
        assert_ne!(bar(3, 17), bar(3, 18));
        assert_ne!(bar(3, 17), bar(4, 17));
    }

    #[test]
    fn every_generated_bar_is_a_bar() {
        // The one property this module guarantees: the evaluator accepts them.
        for day in 0..3 {
            for minute in 0..BARS_PER_SESSION {
                let c = bar(day, minute);
                assert!(c.check().is_ok(), "day {day} minute {minute} is not a bar");
                assert!(c.high >= c.low);
                assert!(c.open <= c.high && c.open >= c.low);
                assert!(c.close <= c.high && c.close >= c.low);
            }
        }
    }

    #[test]
    fn timestamps_increase_strictly_across_sessions() {
        let bars = sessions(4);
        assert_eq!(bars.len(), 4 * BARS_PER_SESSION);
        // Zipped against its own tail rather than matched out of `windows(2)`:
        // the match arm for "a window of two that is not two" cannot happen, and
        // an arm that cannot happen is a region llvm-cov counts forever. Zip has
        // no such arm.
        for (a, b) in bars.iter().zip(bars.iter().skip(1)) {
            assert!(
                b.ts_micros > a.ts_micros,
                "a receding timestamp would be refused by the evaluator"
            );
        }
    }

    #[test]
    fn each_session_starts_a_new_ist_day() {
        let bars = sessions(3);
        let first = bars.first().map_or(0, |c| c.ts_micros);
        let second = bars.get(BARS_PER_SESSION).map_or(0, |c| c.ts_micros);
        assert_eq!(second - first, DAY_MICROS, "one calendar day apart");
    }

    #[test]
    fn a_short_session_is_a_parameter_not_an_accident() {
        // The 2025 Muhurat session was one hour, not a full day.
        let muhurat = session_of(1, 60);
        assert_eq!(muhurat.len(), 60);
        assert!(muhurat.iter().all(|c| c.check().is_ok()));
    }

    #[test]
    fn a_zero_or_negative_count_gives_no_bars() {
        assert!(sessions(0).is_empty());
        assert!(sessions(-1).is_empty());
        assert!(session_of(2, 0).is_empty());
    }
}

/// One bar of a TWO-SIDED series: it falls as readily as it rises.
///
/// # Why the trending generator is not enough
///
/// [`bar`] computes `close = BASE + day*400 + minute*3`. It only ever goes up,
/// and that single fact makes four separate questions unanswerable:
///
/// * A stop loss can only ever HURT on a monotone rise, so "does a stop help"
///   has a trivial answer here and no bearing on a market.
/// * Measured on it, two of the three candidate ranking keys chose the no-stop
///   variant **400 times out of 400**, with a total drawdown of 180 paisa
///   across all 400 candidates. `docs/06-limits.md` records why that table
///   cannot decide the key.
/// * No bar ever reaches both a stop and a target, so `pessimistic` equals
///   `optimistic` on all 19,300 cells a fixture produces and the two-case fill
///   model is never exercised where the cases differ.
/// * Romano–Wolf's stepdown never reaches a second round, so its monotonicity
///   guard compares `0 >= 0`.
///
/// # The shape, and why each piece is here rather than another
///
/// A **mean-reverting** level with a **regime that flips sign**, both driven by
/// the same integer hash of `(day, minute)` that [`bar`] uses. Reversion gives
/// two-sided moves within a session so a stop can be right or wrong on the same
/// day; the regime flip gives stretches where a long is systematically wrong,
/// which is what makes a stop's value vary between combinations rather than
/// being a constant.
///
/// The **range is deliberately wide relative to the drift** — 15x the trending
/// generator's — so a single bar can span both a stop and a target rung and the
/// ambiguity path is reachable without a hand-patched fixture.
///
/// Integer arithmetic only, and no clock: `CLAUDE.md` §7 forbids a float here as
/// firmly as in shipping code, and §3 rule 5 requires the same bar on every
/// machine forever. Same `(day, minute)`, same bar.
#[must_use]
pub fn two_sided_bar(day: i64, minute: usize) -> Candle {
    let m = i64::try_from(minute).unwrap_or(0);

    // A cheap integer hash. Deterministic, no state, and it decorrelates
    // adjacent minutes so the series is not a visible sawtooth.
    let mix = |x: i64| -> i64 {
        // `cast_signed()` rather than `as i64`: the wrap is intended — these are
        // the SplitMix64 odd constants, whose top bits are set by construction —
        // and `cast_possible_wrap` is denied workspace-wide. `cast_signed` says
        // "reinterpret these bits" where `as` says "convert this number", which
        // is the distinction the lint exists to force.
        let h = x
            .wrapping_mul(0x9E37_79B9_7F4A_7C15_u64.cast_signed())
            .rotate_left(29)
            .wrapping_mul(0xBF58_476D_1CE4_E5B9_u64.cast_signed());
        h.rem_euclid(2_000).saturating_sub(1_000)
    };

    // THE REGIME. Roughly every 90 minutes the sign flips, so a long is
    // systematically right for a stretch and systematically wrong for the next.
    // Without this a stop's value is the same for every combination and the
    // grid has nothing to choose between.
    let regime = if (day.saturating_mul(5).saturating_add(m / 90)) % 2 == 0 {
        1
    } else {
        -1
    };

    // MEAN REVERSION toward the day's anchor rather than a trend away from it.
    // The pull is proportional to the displacement, which is what keeps the
    // series two-sided instead of wandering off.
    let anchor = BASE.saturating_add(day.saturating_mul(120));
    let wander = mix(day.saturating_mul(1_000).saturating_add(m));
    let pull = wander / 4;
    let close = anchor
        .saturating_add(wander.saturating_mul(regime))
        .saturating_sub(pull);
    let open = anchor.saturating_add(
        mix(day
            .saturating_mul(1_000)
            .saturating_add(m)
            .saturating_sub(1))
        .saturating_mul(regime),
    );

    // A WIDE range, 15x the trending generator's 60. A bar has to be able to
    // span two ladder rungs for the ambiguity path to exist at all.
    let reach = 900_i64;
    let high = open.max(close).saturating_add(reach);
    let low = open.min(close).saturating_sub(reach);

    Candle::new(
        day.saturating_mul(DAY_MICROS)
            .saturating_add(IST_OPEN_UTC_MICROS)
            .saturating_add(m.saturating_mul(MINUTE_MICROS)),
        open,
        high,
        low,
        close,
        1_000 + (m % 11),
        OI_NULL,
    )
}

/// `count` consecutive sessions of [`two_sided_bar`].
///
/// The drop-in counterpart to [`sessions`] for any measurement whose answer
/// would be an artefact of a one-way market. Same length, same timestamps, same
/// session boundaries — only the price path differs.
#[must_use]
pub fn two_sided_sessions(count: i64) -> Vec<Candle> {
    let mut out = Vec::with_capacity(
        usize::try_from(count.max(0))
            .unwrap_or(0)
            .saturating_mul(BARS_PER_SESSION),
    );
    for day in 0..count {
        for minute in 0..BARS_PER_SESSION {
            out.push(two_sided_bar(day, minute));
        }
    }
    out
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod two_sided_tests {
    use super::{sessions, two_sided_bar, two_sided_sessions};

    #[test]
    fn the_two_sided_series_falls_as_readily_as_it_rises() {
        // THE ONE PROPERTY THE TRENDING GENERATOR CANNOT HAVE, and the reason
        // four separate questions were unanswerable without this.
        //
        // MEASURED over twelve sessions:
        //
        //   trending    up 3,852   down   647
        //   two-sided   up 2,251   down 2,248
        //
        // On a monotone rise a stop loss can only ever hurt, so "does a stop
        // help" has a trivial answer that says nothing about a market.
        let bars = two_sided_sessions(12);
        // `first()`/`last()` rather than `w[0]`/`w[1]`: `indexing_slicing` is
        // denied workspace-wide and applies to test code too. Both sides are
        // `Some` for every `windows(2)` pane, and `Option<i64>` orders by its
        // contents, so the comparison is the same one without the panic path.
        // Same idiom as `excursion.rs:88` and `grid.rs:1300`.
        let up = bars
            .windows(2)
            .filter(|w| w.last().map(|c| c.close) > w.first().map(|c| c.close))
            .count();
        let down = bars
            .windows(2)
            .filter(|w| w.last().map(|c| c.close) < w.first().map(|c| c.close))
            .count();
        assert!(up > 0 && down > 0, "the series moved in one direction only");

        // Within a factor of two of each other. Not exactly equal -- that would
        // be a suspiciously engineered series -- but neither side dominant.
        let (lo, hi) = if up < down { (up, down) } else { (down, up) };
        assert!(
            hi <= lo.saturating_mul(2),
            "the series is {hi} one way against {lo} the other, which is not two-sided"
        );

        // And it is genuinely different from the trending one, so a test that
        // meant to use it cannot silently get the old behaviour.
        let trending = sessions(12);
        assert_eq!(
            bars.len(),
            trending.len(),
            "the two must be interchangeable"
        );
        assert!(
            bars.iter().zip(&trending).any(|(a, b)| a.close != b.close),
            "the two generators produced identical prices"
        );
    }

    #[test]
    fn the_same_arguments_give_the_same_bar_forever() {
        // CLAUDE.md section 3 rule 5. A generator that varied would make every
        // measurement taken on it unreproducible, and the measurements are the
        // only reason it exists.
        assert_eq!(two_sided_bar(3, 100), two_sided_bar(3, 100));
        assert_ne!(two_sided_bar(3, 100), two_sided_bar(3, 101));
        assert_ne!(two_sided_bar(3, 100), two_sided_bar(4, 100));
        // A bar's own range must contain its open and close, or it is not a bar.
        for day in 0..4 {
            for minute in [0_usize, 1, 90, 187, 374] {
                let b = two_sided_bar(day, minute);
                assert!(b.high >= b.open.max(b.close), "high below the body");
                assert!(b.low <= b.open.min(b.close), "low above the body");
                assert!(b.low > 0, "a price fell to or below zero");
            }
        }
    }
}
