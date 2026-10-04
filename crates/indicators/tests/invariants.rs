//! The invariants `docs/04-invariants.md` names for this crate.
//!
//! Rows V-02 … V-05 each named a test that existed in **zero files**. CI gate 10
//! skipped them while `crates/indicators` was not a workspace member and goes red
//! now that it is, so the choice was to write them or to delete the claims. These
//! are the tests, written as ordinary exhaustive `#[test]`s rather than `proptest`
//! — a dependency this workspace has twice declined, and one these properties do
//! not need, because the input space that matters here is a real bar slice that can
//! be walked in full.

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes — see \
              tests/shared_core_doc.rs beside this file: a test that cannot panic \
              cannot fail. `expect` and not the `let ... else { unreachable!() }` this \
              file used to spell, because `unreachable!` expands to a panic inside the \
              crate under test and is therefore a coverage region no green run can ever \
              execute, while `expect` panics inside the standard library and leaves no \
              such region behind."
)]

use indicators::Candle;
use indicators::CurDayFib;
use indicators::daily::DailyLevels;
use indicators::pattern::{Patterns, Thresholds};
use indicators::session::SessionState;
use vocab::{ConditionMask, Tolerance};

/// One synthetic session, deterministic and shaped like a real one.
///
/// Prices wander in a fixed pattern so bars differ from one another — a slice of
/// identical bars would satisfy every property below without exercising anything.
fn session(day: i64, n: i64) -> Vec<Candle> {
    const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
    const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;
    (0..n)
        .map(|m| {
            let drift = (m * 137) % 811 - 405;
            let mid = 2_500_000 + drift * 7;
            Candle {
                ts_micros: day * DAY_MICROS + IST_OPEN_UTC_MICROS + m * 60 * 1_000_000,
                open: mid,
                high: mid + 300 + (m % 11) * 20,
                low: mid - 300 - (m % 7) * 20,
                close: mid + ((m % 5) - 2) * 40,
                volume: 0,
                open_interest: i64::MIN,
            }
        })
        .collect()
}

fn tol() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("the pinned fib tolerance is valid")
}

/// Drive every module that takes a bar at a time and union what they emit.
///
/// This is also the closest thing the crate has to an aggregate evaluator, and
/// writing it here rather than in the library is deliberate: `crates/indicators`
/// has no such function, the audit named that as the missing integration layer, and
/// a test is not the place to introduce one.
fn bits_over(bars: &[Candle]) -> Vec<ConditionMask> {
    let mut fib = CurDayFib::new();
    let mut pat = Patterns::new(Thresholds::default());
    let mut sess = SessionState::default();
    bars.iter()
        .map(|b| {
            let f = fib.step(b, tol()).unwrap_or(ConditionMask::ZERO);
            let p = pat.step(b).unwrap_or(ConditionMask::ZERO);
            // `previous: None` on purpose: the gap family needs yesterday's close, and
            // supplying it would make these tests depend on a second session.
            let s = sess.step(b, None, tol()).unwrap_or(ConditionMask::ZERO);
            f.union(&p).union(&s)
        })
        .collect()
}

/// **V-02.** At bar *i* the evaluator reads no bar `> i`.
///
/// Proved by construction rather than by inspection: run the whole slice, then run
/// every prefix of it, and require the prefix's output to be a prefix of the full
/// run's output. If any module consulted a later bar, a prefix would disagree with
/// the full run at its own last position.
///
/// This is stronger than the accessor `CLAUDE.md` §3 rule 7 asks for. Every
/// evaluator in this crate is a streaming fold — `step(&mut self, bar: &Candle, ..)` —
/// so it never *holds* a future bar and a look-ahead read is not expressible. The
/// test pins that property against a future refactor that hands a module a slice.
#[test]
fn no_lookahead() {
    let bars = session(20_000, 90);
    let full = bits_over(&bars);
    for cut in 1..=bars.len() {
        let prefix = bars.get(..cut).expect("cut is within the slice");
        let got = bits_over(prefix);
        let expected = full.get(..cut).expect("cut is within the slice");
        // Named before the assertion, not computed inside its message: an argument that
        // is only evaluated on failure is a region only a red run reaches.
        let last = cut - 1;
        assert_eq!(
            got, expected,
            "prefix of length {cut} disagreed with the full run; some module read \
             past bar {last}"
        );
    }
}

/// **V-03.** Bits `0..=i` are identical whether bars `i+1..` are absent, mutated,
/// or extreme.
///
/// The complement of V-02: instead of truncating the future, this *corrupts* it.
/// Every bar after the cut is replaced with a pathological one — `i64::MAX` /
/// `i64::MIN` prices, a far-future timestamp — and the bits at and before the cut
/// must not move.
#[test]
fn suffix_independence() {
    let bars = session(20_100, 60);
    let baseline = bits_over(&bars);

    for cut in 1..bars.len() {
        let mut mutated = bars.clone();
        for (offset, slot) in mutated.iter_mut().enumerate().skip(cut) {
            let extreme = i64::from(u32::try_from(offset % 3).unwrap_or(0));
            slot.high = i64::MAX / 4 - extreme;
            slot.low = -(i64::MAX / 4) + extreme;
            slot.open = 0;
            slot.close = extreme;
            slot.volume = i64::MAX;
        }
        let got = bits_over(&mutated);
        let (a, b) = got
            .get(..cut)
            .zip(baseline.get(..cut))
            .expect("cut is within both slices");
        let last = cut - 1;
        assert_eq!(
            a, b,
            "mutating every bar from {cut} onward changed the bits at or before {last}"
        );
    }
}

/// The VWAP family is cleared, not guessed, when volume is unavailable.
///
/// `docs/03-vocabulary.md` §4: a bit that cannot be evaluated evaluates **false**,
/// and never "probably". Measured fact this defends: 0 of 1,222,791 one-minute
/// index bars carry volume, so `Availability::Absent` is the real answer on the
/// timeframe the engine sweeps — and all 20 positions must stay false rather than
/// emitting a plain average wearing a VWAP label.
///
/// This is the `Absent` half of VWAP clearing on one-minute bars and drives
/// `Vwap` alone. It was V-04's only proof and never built a daily bar
/// (P12-01); the daily timeframe is
/// `a_daily_bar_series_sets_no_time_of_day_or_vwap_bit` below.
#[test]
fn daily_mask_clears() {
    use indicators::vwap::{Availability, Vwap};

    let bars = session(20_200, 120);
    let mut v = Vwap::for_slice(Availability::Absent);
    for b in &bars {
        let mask = v.step(b, tol()).expect("a sane bar");
        for p in indicators::vwap::positions() {
            assert!(
                !mask.get(u32::from(p)),
                "position {p} was set with Availability::Absent"
            );
        }
        assert_eq!(v.value(), None, "no VWAP without volume");
        assert_eq!(v.sigma(), None, "no sigma without volume");
    }
}

/// **V-04.** Time-of-day (44-47) and every VWAP position (52-53 among them)
/// are clear on a daily timeframe, through the production `Evaluator` and
/// `Column::build`. P12-01.
///
/// A daily-rung bar is stamped at 00:00 IST (pinned in `crates/pull` by
/// `the_daily_rung_is_anchored_at_ist_midnight_and_not_at_the_open`). That is
/// before the 09:15 open, so `orb::minutes_since_open` is `None` and no
/// time-of-day window can hold; and a daily bar is the only bar of its IST
/// session, while a present-volume VWAP needs two contributing bars in the
/// current session before it can answer. So the bits are clear BY THOSE TWO
/// MECHANISMS, with volume present and with it absent. In the knowledge mask
/// they differ, measured: every VWAP position is UNKNOWN (zero known bits),
/// while 44-47 are KNOWN false, a midnight bar being in no window. Nothing in
/// the evaluator knows the word "daily"; the test pins that the two
/// mechanisms still produce the documented answer.
///
/// The control re-stamps the same prices at two intraday minutes of each day
/// and requires a time-of-day bit and a VWAP side bit to appear, so the
/// assertion is not passing on a fold that never sets these positions at all.
#[test]
fn a_daily_bar_series_sets_no_time_of_day_or_vwap_bit() {
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::vwap::Availability;

    const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;
    const TEN_IST_UTC_MICROS: i64 = (600 - 330) * 60 * 1_000_000;
    const DAYS: i64 = 260;

    let candle = |ts_micros: i64, d: i64| {
        let mid = 2_500_000 + ((d * 137) % 811 - 405) * 70;
        Candle {
            ts_micros,
            open: mid,
            high: mid + 9_000 + (d % 11) * 200,
            low: mid - 9_000 - (d % 7) * 200,
            close: mid + ((d % 5) - 2) * 400,
            volume: 1_000 + d,
            open_interest: i64::MIN,
        }
    };
    let daily: Vec<Candle> = (0..DAYS)
        .map(|d| candle((20_000 + d) * DAY_MICROS - indicators::IST_OFFSET_MICROS, d))
        .collect();
    let intraday: Vec<Candle> = (0..DAYS)
        .flat_map(|d| {
            let ten = (20_000 + d) * DAY_MICROS + TEN_IST_UTC_MICROS;
            [candle(ten, d), candle(ten + 60 * 1_000_000, d + 1)]
        })
        .collect();

    let mut cleared: Vec<u16> = vec![44, 45, 46, 47];
    cleared.extend(indicators::vwap::positions());
    let any_set = |rows: &[vocab::ConditionMask], positions: &[u16]| {
        rows.iter()
            .any(|row| positions.iter().any(|&p| row.get(u32::from(p))))
    };
    let widths = Widths::pinned().expect("both pinned widths are valid");

    for availability in [Availability::Present, Availability::Absent] {
        let mut ev = Evaluator::new(widths, availability, Thresholds::CLASSICAL);
        let column = Column::build(&daily, &mut ev);
        assert!(
            !column.is_empty(),
            "the daily series must warm and emit rows, or the clearing is vacuous"
        );
        assert!(
            !any_set(column.bits(), &cleared),
            "a daily-rung bar set a time-of-day or VWAP position"
        );
        // VWAP is UNKNOWN on a daily bar, so a negation cannot turn it into a
        // signal; time of day is a KNOWN false (a midnight bar is in no window).
        assert!(
            !any_set(column.known(), &indicators::vwap::positions()),
            "a daily-rung bar certified a VWAP answer"
        );
        assert!(
            column
                .known()
                .iter()
                .all(|row| [44_u32, 45, 46, 47].iter().all(|&p| row.get(p))),
            "a daily-rung bar left time of day uncertified"
        );
    }

    let mut ev = Evaluator::new(widths, Availability::Present, Thresholds::CLASSICAL);
    let control = Column::build(&intraday, &mut ev);
    assert!(
        any_set(control.bits(), &[44, 45, 46, 47]),
        "the intraday control set no time-of-day bit"
    );
    assert!(
        any_set(control.bits(), &[52, 53]),
        "the intraday control set no VWAP side bit"
    );
}

/// **V-05.** The daily pivot ladder agrees with the design document's
/// recurrence on five sessions. It is not a differential test of the whole
/// evaluator on random input, which is what V-05 claimed until P12-01.
///
/// The reference is rebuilt here from the recurrence in `docs/09-design-sources.md`
/// §1 — read off the source document, not off `daily.rs` — because a reference
/// derived from the implementation shares the implementation's misreadings. Plain
/// `i128`, one level at a time, no shared helpers.
#[test]
fn differential_vs_naive() {
    for (h, l, c) in [
        (2_500_000_i64, 2_400_000_i64, 2_490_000_i64),
        (2_500_000, 2_400_000, 2_410_000), // inverted CPR: bc > tc
        (2_500_000, 2_500_000, 2_500_000), // zero range
        (1, 0, 0),
        (9_000_000, 100, 4_500_000),
    ] {
        // The session is named in a `String` built before the call, because `expect`
        // takes a `&str` and the three prices are what identifies a failing row. A
        // `format!` on the failure path only would be a region a green run cannot
        // reach; built here it runs on every row.
        let session = format!("sane session ({h}, {l}, {c})");
        let fast = DailyLevels::from_previous_session(h, l, c).expect(&session);

        // The naive reference, straight from the document.
        let sum = i128::from(h) + i128::from(l) + i128::from(c);
        let pivot = sum.div_euclid(3);
        let bc = (i128::from(h) + i128::from(l)).div_euclid(2);
        let tc = 2 * pivot - bc;
        let range = i128::from(h) - i128::from(l);
        let r1 = 2 * pivot - i128::from(l);
        let r2 = pivot + range;
        let r3 = r1 + range;
        let r4 = r3 + r2 - r1;
        let r5 = r4 + r3 - r2;
        let s1 = 2 * pivot - i128::from(h);
        let s2 = pivot - range;
        let s3 = s1 - range;
        let s4 = s3 + s2 - s1;
        let s5 = s4 + s3 - s2;

        assert_eq!(i128::from(fast.pivot()), pivot, "pivot ({h},{l},{c})");
        assert_eq!(i128::from(fast.bc()), bc, "bc ({h},{l},{c})");
        assert_eq!(i128::from(fast.tc()), tc, "tc ({h},{l},{c})");
        // `Some(want)` rather than an unwrap and a compare: it asserts the level EXISTS
        // in the same breath as its value, and it removes an `else` arm that a correct
        // `DailyLevels` can never take.
        for (n, want) in [(1, r1), (2, r2), (3, r3), (4, r4), (5, r5)] {
            assert_eq!(
                fast.resistance(n).map(i128::from),
                Some(want),
                "R{n} ({h},{l},{c})"
            );
        }
        for (n, want) in [(1, s1), (2, s2), (3, s3), (4, s4), (5, s5)] {
            assert_eq!(
                fast.support(n).map(i128::from),
                Some(want),
                "S{n} ({h},{l},{c})"
            );
        }
        // The pivot is the exact midpoint of [bc, tc] because tc = 2*pivot - bc.
        // This identity is what retired position 6 and what makes the CPR's own
        // band exactly [bc, tc] (D-0079).
        let (low, high) = fast.cpr_span();
        assert_eq!(
            i128::from(low) + i128::from(high),
            2 * pivot,
            "pivot is not the midpoint of the CPR ({h},{l},{c})"
        );
    }
}

/// Every module's emission is a function of the bars alone — running twice gives
/// byte-identical output.
///
/// `CLAUDE.md` §3 rule 5 asserts idempotence and nothing in the repository ran one
/// computation twice and compared. This does, in one process and over a whole
/// session.
#[test]
fn two_runs_of_one_slice_agree_exactly() {
    let bars = session(20_300, 200);
    let first = bits_over(&bars);
    for _ in 0..4 {
        assert_eq!(
            bits_over(&bars),
            first,
            "a rerun disagreed with the first run"
        );
    }
}
