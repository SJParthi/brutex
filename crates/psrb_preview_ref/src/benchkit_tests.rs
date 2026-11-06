#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;

#[test]
fn a_distribution_reads_by_nearest_rank() {
    let d = Dist::of((1..=100).rev().collect());
    assert_eq!((d.p50(), d.p99(), d.max(), d.n()), (50, 99, 100, 100));
    assert_eq!(d.at(0), 1);
    let empty = Dist::of(Vec::new());
    assert_eq!((empty.p50(), empty.p99(), empty.max(), empty.n()), (0, 0, 0, 0));
    assert_eq!(Dist::of(vec![7]).p99(), 7);
}

#[test]
fn the_ratio_passes_at_the_ceiling_and_breaches_past_it() {
    assert_eq!(ratio(0, 5), Verdict::Unmeasurable);
    assert_eq!(ratio(1_000, 3_000), Verdict::Flat(3_000));
    assert_eq!(ratio(1_000, 3_001), Verdict::Breach(3_001));
    assert_eq!(ratio(7, 7), Verdict::Flat(1_000));
    let flat = [Dist::of(vec![10; 4]), Dist::of(vec![10; 4]), Dist::of(vec![20; 4])];
    let (line, ok) = row("flat", &flat);
    assert!(ok && line.starts_with("flat: 1x p50 10"));
    let breach = [Dist::of(vec![10; 4]), Dist::of(vec![10; 4]), Dist::of(vec![31; 4])];
    assert!(!row("breach", &breach).1);
    let zero = [Dist::of(vec![0; 4]), Dist::of(vec![10; 4]), Dist::of(vec![10; 4])];
    assert!(!row("zero", &zero).1);
    let first = [Dist::of(vec![10; 4]), Dist::of(vec![31; 4]), Dist::of(vec![10; 4])];
    assert!(!row("first", &first).1);
}

#[test]
fn the_measured_steps_run_at_tiny_sizes() {
    assert_eq!(per_candle(5, 3).n(), 3);
    assert_eq!(per_second(5, 3).n(), 3);
    assert_eq!(time_calls(2, |_| {}).n(), 2);
    let c = candle(97);
    assert_eq!(c.close, 100_000 + 5 * 97 + 300);
    assert_eq!(candle(300).close, 100_000 + 5 * 100);
}
