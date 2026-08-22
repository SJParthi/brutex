//! Mapping a signal series onto the series a position is actually taken on.
//!
//! # The rule, and where it comes from
//!
//! `docs/00-charter.md` §3 stamps a bar at the OPEN of its interval, so the bar
//! stamped `t` covers `[t, t + length)` and its close prints at `t + length`.
//! A condition decided on that bar is therefore not knowable until `t + length`,
//! and the earliest bar it can be acted on is the first EXECUTION bar that opens
//! at or after that instant.
//!
//! That single sentence is the whole module. Everything else here is arithmetic
//! and refusals.
//!
//! # Why this exists at all
//!
//! Until it did, `runner::trade::walk` and `runner::grid::evaluate` each took
//! ONE bar slice, so a condition found on a fifteen-minute bar was also filled
//! on fifteen-minute bars and its stop was checked at fifteen-minute
//! resolution. The entry INSTANT was already right — the next fifteen-minute bar
//! opens exactly when the signal bar closes — so what this changes is not when a
//! position is opened but **how finely the path afterwards is measured**:
//!
//! * a stop and a target both inside one bar's range have no order the data can
//!   settle, and a fifteen-minute bar hides fifteen minutes of that path;
//! * a trailing order tracks a running peak that updates 25 times a session on
//!   fifteen-minute bars and 375 times on one-minute bars. The trail is the
//!   figure that moves most, because the coarse series never saw the peak;
//! * `Horizon` is counted in BARS, so "15" means fifteen minutes on a
//!   one-minute series and three hours forty-five minutes on a fifteen-minute
//!   one. Executing on one-minute bars makes the horizon mean minutes on every
//!   signal timeframe, which is the only reading under which nine timeframes are
//!   comparable at all.
//!
//! # No look-ahead is preserved, not merely hoped for
//!
//! `Column::reproject` copies the mask a bar produced and changes only the index
//! it is filed under. No indicator is evaluated on a series it was not built
//! for. The mask came from bars `0..=s` of the signal series and the index it
//! now carries points at a bar opening at or after that signal bar CLOSED, so
//! `CLAUDE.md` §3 rule 7 holds by the same argument it always did.
//!
//! # Cost
//!
//! One merge pass over two sorted series: `O(signals + execution)` total and
//! `O(1)` amortised per bar, with the execution cursor moving forward only.
//! There is no binary search per signal and no scan. Run once per run, never per
//! candidate.

use indicators::Candle;

/// Where a signal series lands on an execution series, and what did not land.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alignment {
    /// For each column position, the execution index to act on, or `None`.
    pub onto: Vec<Option<usize>>,
    /// Signals with no execution bar at or after their close.
    ///
    /// The session's last bars produce these, and so does a signal whose close
    /// falls past the end of the execution series. Counted rather than hidden:
    /// a projection that quietly shortened the column would make a smaller
    /// sample read like a whole one.
    pub unreachable: u64,
}

/// The instant a bar stamped `ts` stops being incomplete.
///
/// `ts + length`, saturating. A bar stamped at the open covers `[ts, ts+len)`,
/// so this is the first instant its mask is knowable.
const fn closes_at(ts_micros: i64, length_micros: i64) -> i64 {
    ts_micros.saturating_add(length_micros)
}

/// Maps every position of a signal column onto an execution series.
///
/// `sources` is `Column::sources()` — the signal-series index each column
/// position came from. `signal` and `execution` must each be sorted ascending
/// by `ts_micros`, which `stored::load_span` guarantees and `Column::build`
/// requires.
///
/// # What is NOT done here
///
/// No bar is skipped for being in a different session, and none is required to
/// be. A signal at 15:09 whose close is 15:10 finds the 15:10 execution bar and
/// `runner::trade`'s square-off rule then refuses it as an entry — that rule
/// lives there, is already tested, and duplicating it here would be a second
/// spelling of one policy.
///
/// # Errors
///
/// `None` when `length_micros` is not positive, or when a `sources` entry does
/// not index `signal`. Both are caller bugs rather than data conditions, so they
/// refuse rather than being absorbed into `unreachable` — a count that mixed
/// "no bar to trade" with "the caller passed the wrong slice" could not be read.
#[must_use]
pub fn onto_execution(
    signal: &[Candle],
    sources: &[usize],
    execution: &[Candle],
    length_micros: i64,
) -> Option<Alignment> {
    if length_micros <= 0 {
        return None;
    }
    let mut onto: Vec<Option<usize>> = Vec::with_capacity(sources.len());
    let mut unreachable: u64 = 0;
    // THE CURSOR ONLY MOVES FORWARD, which is what makes this a merge and not a
    // search. `sources` is ascending because `Column::build` walks the slice in
    // order, so each signal's close is at or after the previous one's and the
    // execution bar for it cannot be earlier than the last one found.
    let mut cursor: usize = 0;
    for &s in sources {
        let bar = signal.get(s)?;
        let deadline = closes_at(bar.ts_micros, length_micros);
        while execution
            .get(cursor)
            .is_some_and(|c| c.ts_micros < deadline)
        {
            cursor = cursor.saturating_add(1);
        }
        if cursor < execution.len() {
            onto.push(Some(cursor));
        } else {
            onto.push(None);
            unreachable = unreachable.saturating_add(1);
        }
    }
    Some(Alignment { onto, unreachable })
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes: a test \
              that cannot panic cannot fail. `clippy::panic` is deliberately NOT \
              in this list -- nothing here writes `panic!` and an unfulfilled \
              expectation is itself a warning, so listing it would trade one \
              lint for another"
)]
mod tests {
    use super::{Alignment, onto_execution};
    use indicators::Candle;

    /// A bar stamped `minute` minutes past an arbitrary epoch anchor.
    fn bar(minute: i64) -> Candle {
        Candle {
            ts_micros: minute * 60_000_000,
            open: 2_500_000,
            high: 2_500_100,
            low: 2_499_900,
            close: 2_500_050,
            volume: 0,
            open_interest: i64::MIN,
        }
    }

    /// `n` one-minute bars from minute zero.
    fn minutes(n: i64) -> Vec<Candle> {
        (0..n).map(bar).collect()
    }

    /// `n` bars of `step` minutes each, from minute zero.
    fn coarse(step: i64, n: i64) -> Vec<Candle> {
        (0..n).map(|i| bar(i * step)).collect()
    }

    const FIFTEEN_MIN: i64 = 15 * 60 * 1_000_000;
    const ONE_MIN: i64 = 60 * 1_000_000;

    #[test]
    fn a_signal_lands_on_the_first_execution_bar_at_or_after_its_close() {
        // THE WHOLE RULE. A fifteen-minute bar stamped at minute 0 covers
        // [0, 15) and closes at 15, so the earliest one-minute bar that may act
        // on it is the one stamped 15 -- not 14, which opened while the signal
        // bar was still incomplete and is therefore look-ahead.
        let signal = coarse(15, 4);
        let exec = minutes(60);
        let got = onto_execution(&signal, &[0, 1, 2, 3], &exec, FIFTEEN_MIN).expect("aligned");
        assert_eq!(
            got.onto,
            vec![Some(15), Some(30), Some(45), None],
            "each signal maps to its own close instant, and the last has no bar \
             after it"
        );
        assert_eq!(got.unreachable, 1);
    }

    #[test]
    fn a_one_minute_signal_on_one_minute_bars_is_simply_the_next_bar() {
        // The degenerate case, and it must stay degenerate: when the signal
        // series IS the execution series, this is exactly "enter on the next
        // bar", which is what the engine did before this module existed. If this
        // ever disagreed, every existing one-minute result would have moved.
        let bars = minutes(6);
        let got = onto_execution(&bars, &[0, 1, 2, 3, 4, 5], &bars, ONE_MIN).expect("aligned");
        assert_eq!(
            got.onto,
            vec![Some(1), Some(2), Some(3), Some(4), Some(5), None]
        );
        assert_eq!(got.unreachable, 1, "the last bar has no next bar");
    }

    #[test]
    fn an_execution_series_with_holes_takes_the_first_bar_past_the_close() {
        // A one-minute series is not dense in practice: a halt, a missing month,
        // or a session boundary leaves gaps. The rule is "first bar at or after
        // the close", so a gap moves the entry LATER and never earlier.
        let signal = coarse(15, 2);
        // Minutes 0..10 then a jump to 40. The first signal closes at 15 and
        // there is no bar at 15, so it must take 40.
        let mut exec: Vec<Candle> = (0..10).map(bar).collect();
        exec.push(bar(40));
        let got = onto_execution(&signal, &[0, 1], &exec, FIFTEEN_MIN).expect("aligned");
        assert_eq!(
            got.onto,
            vec![Some(10), Some(10)],
            "both signals close before minute 40, so both take that bar"
        );
        assert_eq!(exec[10].ts_micros, 40 * 60_000_000);
        assert_eq!(got.unreachable, 0);
    }

    #[test]
    fn a_signal_whose_close_is_past_every_execution_bar_is_unreachable() {
        let signal = coarse(15, 2);
        let exec = minutes(5);
        let got = onto_execution(&signal, &[0, 1], &exec, FIFTEEN_MIN).expect("aligned");
        assert_eq!(
            got,
            Alignment {
                onto: vec![None, None],
                unreachable: 2
            },
            "no execution bar exists at or after either close, and that is \
             COUNTED rather than mapped to the last bar"
        );
    }

    #[test]
    fn the_cursor_never_moves_backwards_so_the_pass_stays_linear() {
        // This is the property that makes the module O(signals + execution)
        // rather than O(signals * execution). Asserted on the RESULT because
        // that is observable: a non-decreasing output is exactly what a
        // forward-only cursor produces, and a per-signal search could produce
        // the same values while costing a scan each time.
        let signal = coarse(3, 100);
        let exec = minutes(400);
        let sources: Vec<usize> = (0..100).collect();
        let got = onto_execution(&signal, &sources, &exec, 3 * ONE_MIN).expect("aligned");
        let mut last = 0;
        for slot in &got.onto {
            let at = slot.expect("every signal has a bar in this fixture");
            assert!(at >= last, "the mapping must be non-decreasing");
            last = at;
        }
        assert_eq!(got.onto.first().copied(), Some(Some(3)));
    }

    #[test]
    fn a_non_positive_bar_length_is_refused_rather_than_looping() {
        // A length of zero makes every deadline equal to the signal's own stamp,
        // which would map a signal onto a bar that opened BEFORE it closed --
        // look-ahead, silently. A negative one is worse. Neither is a data
        // condition, so both refuse.
        let bars = minutes(4);
        assert!(onto_execution(&bars, &[0], &bars, 0).is_none());
        assert!(onto_execution(&bars, &[0], &bars, -60_000_000).is_none());
    }

    #[test]
    fn a_source_index_outside_the_signal_series_refuses() {
        // A caller bug, not a data condition: absorbing it into `unreachable`
        // would mix "no bar to trade" with "the wrong slice was passed" in one
        // number that could then not be read.
        let signal = minutes(3);
        let exec = minutes(10);
        assert!(onto_execution(&signal, &[0, 99], &exec, ONE_MIN).is_none());
    }

    #[test]
    fn an_empty_signal_set_aligns_to_nothing_without_refusing() {
        let exec = minutes(10);
        let got =
            onto_execution(&[], &[], &exec, ONE_MIN).expect("nothing to align is not an error");
        assert!(got.onto.is_empty());
        assert_eq!(got.unreachable, 0);
    }
}
