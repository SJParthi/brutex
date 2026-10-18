//! The FIB bench harness's own tests (FIB-1; D-6318; FM-F001, FM-F002), run
//! under `cargo test` from the same file the four FIB bench targets include.

#[path = "../benches/fib_rows/harness.rs"]
mod harness;

#[test]
fn harness_detects_amortised_spike() {
    assert_eq!(harness::harness_detects_amortised_spike(), Ok(()));
}

#[test]
fn harness_prints_timer_granularity() {
    assert_eq!(harness::harness_prints_timer_granularity(), Ok(()));
    assert!(harness::timer_granularity_ns() > 0);
}

#[test]
fn harness_refuses_row_without_class() {
    assert_eq!(harness::harness_refuses_row_without_class(), Ok(()));
}

#[test]
fn every_row_declares_axis_and_scale() {
    assert_eq!(harness::every_row_declares_axis_and_scale(), Ok(()));
}

#[test]
fn a_data_row_runs_at_one_and_at_the_production_worker_count() {
    assert_eq!(
        harness::a_data_row_runs_at_one_and_at_the_production_worker_count(),
        Ok(())
    );
}

#[test]
fn the_work_histogram_is_exact_below_its_threshold_and_bounded_above() {
    let mut h = harness::WorkHistogram::default();
    for value in [24, 24, 24, 25, 70_000] {
        h.record(value);
    }
    assert_eq!(h.quantile(1, 2), 24);
    assert_eq!(h.quantile(4, 5), 25);
    assert_eq!(
        h.quantile(1, 1),
        70_000,
        "a bucket above the exact range is capped at the max"
    );
    assert_eq!(h.max(), 70_000);
    assert_eq!(h.count(), 5);
    assert_eq!(h.mean_milli(), 14_019_400);
    let mut wide = harness::WorkHistogram::new();
    for value in [70_000, 70_000, 200_000] {
        wide.record(value);
    }
    assert_eq!(
        wide.quantile(1, 2),
        131_071,
        "the power-of-two bucket's upper bound"
    );
    let mut huge = harness::WorkHistogram::new();
    huge.record(u64::MAX);
    assert_eq!(huge.quantile(1, 2), u64::MAX);
}

#[test]
fn every_class_is_labelled() {
    assert_eq!(harness::Class::WorstCase.label(), "WORST-CASE");
    assert_eq!(harness::Class::Expected.label(), "EXPECTED");
    assert_eq!(
        harness::Class::Amortised { period: 7 }.label(),
        "AMORTISED (period 7 calls)"
    );
    assert_eq!(
        harness::Class::Bounded { bound: "O(n)" }.label(),
        "BOUNDED (O(n))"
    );
    assert!(harness::Verdict::Fail(String::new()).fails());
    assert!(harness::Verdict::Refused(String::new()).fails());
    assert!(!harness::Verdict::Pass.fails());
    assert!(!harness::Verdict::TimeAbove(String::new()).fails());
}
