//! `CLAUDE.md` §3 rule 7 for exit levels (D-1514): a hole AFTER a level exit,
//! a refused record or a missing minute, must not un-price that exit or move
//! any trade that closed before it.
//!
//! Until D-1514 `trade::walk_core` marked a whole path block-only when any bar
//! up to its TIME exit was a hole, so a stop that had already closed the
//! position vanished from every exit-grid cell and held the next signal to the
//! time exit: a bar after the exit decided whether the exit counted.
//!
//! Generated OHLCV fixtures; no market execution or profit assurance.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "fixture failures name the broken premise"
)]

use indicators::Candle;
use indicators::column::Column;
use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use runner::excursion::{Ladder, Ladders, Side};
use runner::grid::{Chosen, TradeRow, per_trade};
use runner::outcome::Horizon;
use vocab::ConditionMask;

const BARS_PER_SESSION: usize = 375;
const HOLD: u32 = 30;

fn evaluator() -> Evaluator {
    Evaluator::new(
        Widths::pinned().expect("pinned widths are valid"),
        Availability::Absent,
        Thresholds::CLASSICAL,
    )
}

/// Eight generated sessions with a seven-minute sawtooth laid over them, so
/// a 300 ppm stop fires inside a hold and some holds run to their time exit.
fn choppy() -> Vec<Candle> {
    let mut bars = runner::synthetic::sessions(8);
    for (index, bar) in bars.iter_mut().enumerate() {
        let tooth = i64::try_from(index % 7).expect("small") - 3;
        let shift = tooth.saturating_mul(400);
        bar.open = bar.open.saturating_add(shift);
        bar.high = bar.high.saturating_add(shift);
        bar.low = bar.low.saturating_add(shift);
        bar.close = bar.close.saturating_add(shift);
    }
    bars
}

fn ladder(ppm: i64) -> Ladder {
    Ladder::new(vec![ppm]).expect("an ascending ladder")
}

/// Every row `bars` trades under `chosen`, long, every bar a signal.
fn rows(bars: &[Candle], chosen: Chosen) -> Vec<TradeRow> {
    let (stops, targets, trails) = (ladder(300), ladder(900_000), ladder(900_000));
    let column = Column::build(bars, &mut evaluator());
    per_trade(
        bars,
        &column,
        &ConditionMask::ZERO,
        Horizon::bars(HOLD).expect("a non-zero horizon"),
        Side::Long,
        Ladders {
            stops: &stops,
            targets: &targets,
            trails: &trails,
        },
        chosen,
    )
    .map_or_else(Vec::new, |(_, rows)| rows)
}

const STOP: Chosen = Chosen {
    stop: Some(0),
    target: None,
    tsl: None,
    ttp: None,
};

const TIME: Chosen = Chosen {
    stop: None,
    target: None,
    tsl: None,
    ttp: None,
};

/// Stop exits that closed at least two bars before their time exit, early
/// enough in their session that the time exit is `entry + HOLD`, so the bar
/// after the stop lies on the path the walk used to block on.
fn early_stops(clean: &[TradeRow]) -> Vec<TradeRow> {
    clean
        .iter()
        .copied()
        .filter(|row| {
            row.entry_bar % BARS_PER_SESSION < 300
                && row.exit_bar.saturating_add(2) < row.entry_bar.saturating_add(HOLD as usize)
                && row.exit_bar > row.entry_bar
        })
        .take(6)
        .collect()
}

/// The rows that exited before `hole`: bars `0..hole` decided them, so a hole
/// at `hole` may not move one of them.
fn decided_before(rows: &[TradeRow], hole: usize) -> Vec<TradeRow> {
    rows.iter()
        .copied()
        .filter(|row| row.exit_bar < hole)
        .collect()
}

fn check(damage: impl Fn(&mut Vec<Candle>, usize), what: &str) {
    let bars = choppy();
    let clean = rows(&bars, STOP);
    let probes = early_stops(&clean);
    assert!(
        probes.len() >= 3,
        "the fixture must hold stop exits well inside their hold: {}",
        probes.len()
    );
    for probe in probes {
        let hole = probe.exit_bar.saturating_add(1);
        let mut damaged = bars.clone();
        damage(&mut damaged, hole);
        let after = rows(&damaged, STOP);
        assert!(
            after.contains(&probe),
            "{what} at {hole}, after a stop that closed at {}: the stop must stay priced, \
             unchanged",
            probe.exit_bar
        );
        assert_eq!(
            decided_before(&after, hole),
            decided_before(&clean, hole),
            "{what} at {hole}: every trade that closed before it is unchanged"
        );
        // The time exit of the same path is AT or after the hole, so it is
        // still unpriceable: no time-exit row may cross the hole.
        let timed = rows(&damaged, TIME);
        assert!(
            timed
                .iter()
                .all(|row| !(row.entry_bar < hole && hole <= row.exit_bar)),
            "{what} at {hole}: a time exit read across the hole"
        );
    }
}

#[test]
fn a_refused_record_after_a_stop_leaves_the_stop_priced() {
    check(
        |bars, at| {
            let bar = bars.get_mut(at).expect("inside the slice");
            bar.close = bar.high.saturating_add(1);
        },
        "a refused record",
    );
}

#[test]
fn a_missing_minute_after_a_stop_leaves_the_stop_priced() {
    check(
        |bars, at| {
            bars.remove(at);
        },
        "a missing minute",
    );
}

/// The hole ON the stop bar: the exit itself cannot be read, so the stop is
/// not priced from it.
#[test]
fn a_refused_stop_bar_is_never_priced() {
    let bars = choppy();
    let clean = rows(&bars, STOP);
    let probe = *early_stops(&clean).first().expect("a stop exit");
    let mut damaged = bars.clone();
    let bar = damaged.get_mut(probe.exit_bar).expect("inside the slice");
    bar.close = bar.high.saturating_add(1);
    let after = rows(&damaged, STOP);
    assert!(
        after
            .iter()
            .all(|row| !(row.entry_bar <= probe.exit_bar && probe.exit_bar <= row.exit_bar)),
        "no priced row may read the refused stop bar"
    );
}
