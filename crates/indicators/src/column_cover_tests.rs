#![allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail; `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind"
)]
//! BASE-COV-LIBS-LS3 (attempt 5): the one refusal of the exact-minute bridge
//! (`crate::anchored`) that no input reaches, driven here where `Column`'s
//! fields are visible.
//!
//! `OverlayLengthMismatch` fires when the bridge's evidence, one entry per
//! entry of `Column::sources`, is not one entry per row. Every constructor
//! keeps `source`, `bits` and `known` parallel, so only a column whose
//! invariant was broken can reach it; the refusal is what stands between such
//! a column and a half-replaced family. Extreme cases: a column of eight warm
//! sessions with every source dropped, an empty signal, a single exact minute;
//! and (the combination) a column one source LONGER than its rows, which the
//! bridge refuses before it compares lengths, because a source with no signal
//! bar is its own refusal.

use super::Column;
use super::tests::{evaluator, run};
use crate::Candle;
use crate::anchored::{ExactMinuteGapRefusal, overlay_exact_minute_gapfib};
use crate::evaluator::{Calendar, Widths};
use crate::vwap::Availability;

const MINUTE_MICROS: i64 = 60 * 1_000_000;

/// The bridge over `column` with an empty signal and one exact minute.
fn overlay(column: &mut Column, minute: Candle) -> Result<(), ExactMinuteGapRefusal> {
    overlay_exact_minute_gapfib(
        &[],
        &[minute],
        MINUTE_MICROS,
        Widths::pinned().expect("pinned widths"),
        Calendar::charter(),
        |_| None,
        column,
    )
    .map(|_| ())
}

#[test]
fn an_overlay_onto_a_column_whose_sources_and_rows_disagree_is_refused_whole() {
    let bars = run(8);
    let sound = Column::build(&bars, &mut evaluator(Availability::Absent));
    assert!(!sound.is_empty(), "the fixture column holds rows");
    let minute = bars.first().copied().expect("a first bar");

    let mut dropped = sound.clone();
    dropped.source.clear();
    assert_eq!(
        overlay(&mut dropped, minute),
        Err(ExactMinuteGapRefusal::OverlayLengthMismatch)
    );
    assert_eq!(
        (&dropped.bits, &dropped.known),
        (&sound.bits, &sound.known),
        "a refused overlay replaces nothing"
    );

    let mut longer = sound.clone();
    longer.source.push(0);
    assert!(matches!(
        overlay(&mut longer, minute),
        Err(ExactMinuteGapRefusal::SignalSourceOutOfBounds { row: 0, .. })
    ));

    // The sound half: an empty column over an empty signal is joined, with
    // nothing to replace.
    let mut empty = Column::build(&[], &mut evaluator(Availability::Absent));
    assert_eq!(overlay(&mut empty, minute), Ok(()));
}
