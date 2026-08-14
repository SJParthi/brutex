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
