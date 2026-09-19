#![cfg(test)]
//! Generated observations exercise persistence; none are market performance.
#![expect(clippy::unwrap_used, reason = "named persistence invariant assertions")]
use super::*;
use crate::index_consistency::{Eligibility, eligibility_of, evaluate};
use brutex_core::instrument::{Exchange, InstrumentKey};
use pull::session::Day;
use runner::research_family::ResearchFamilyV1;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

const BUDGET: u64 = 1_000_000;
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-index-consistency-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn day(month: u8, date: u8) -> i64 {
    i64::from(Day::new(2025, month, date).unwrap().days_from_epoch())
}
fn record(gap: bool, no_trades: bool) -> Record {
    let family =
        ResearchFamilyV1::new(InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()).unwrap();
    let training_first = day(1, 6);
    let training_last = day(1, 10);
    let later_first = day(1, if gap { 20 } else { 13 });
    let later_last = day(1, if gap { 24 } else { 17 });
    let sessions: Vec<_> = (training_first..=training_last)
        .chain(later_first..=later_last)
        .filter(|&day| eligibility_of(day) == Eligibility::Eligible)
        .map(|day| Session {
            day,
            pessimistic_paisa: if no_trades { 0 } else { 100 },
            optimistic_paisa: if no_trades { 0 } else { 100 },
            trades: if no_trades { 0 } else { 3 },
        })
        .collect();
    Record {
        binding: Binding {
            original: [1; 32],
            later: [2; 32],
            later_identity: [3; 32],
            later_completion: [4; 32],
            source: [5; 32],
            family: 0,
            coordinate: 0,
        },
        evaluation: evaluate(
            Policy::V1,
            family,
            training_first,
            later_last,
            &sessions,
            true,
        ),
        training: evaluate(
            Policy::V1,
            family,
            training_first,
            training_last,
            sessions.get(..5).unwrap(),
            true,
        ),
        later: evaluate(
            Policy::V1,
            family,
            later_first,
            later_last,
            sessions.get(5..).unwrap(),
            true,
        ),
        training_session_count: 5,
        sessions,
    }
}
fn write(temp: &Temp, value: Record) -> Reader {
    produce(&temp.0, [8; 32], [9; 32], vec![value], BUDGET, || Ok(())).unwrap()
}

#[test]
fn receipt_reopens_exactly_reuses_bytes_and_keeps_independent_admission() {
    let temp = Temp::new();
    let expected = record(false, false);
    let first = write(&temp, expected.clone());
    let receipt = first.receipt();
    assert_eq!(first.record([9; 32], 0).unwrap(), &expected);
    assert_eq!(expected.outcome(), Outcome::Passed);
    assert!(expected.combined_qualifies(AdmissionStatusV1::Admitted));
    for status in [
        AdmissionStatusV1::Rejected,
        AdmissionStatusV1::Refused,
        AdmissionStatusV1::Unmeasured,
    ] {
        assert!(!expected.combined_qualifies(status));
    }
    drop(first);
    let again = write(&temp, expected.clone());
    assert_eq!(again.receipt(), receipt);
    let reopened = Reader::open(&temp.0, [8; 32], [9; 32], Policy::V1, BUDGET, 1).unwrap();
    assert_eq!(reopened.record([9; 32], 0).unwrap(), &expected);
    assert!(reopened.record([10; 32], 0).is_err());
    assert!(reopened.record([9; 32], 1).is_err());
    assert!(Reader::open(&temp.0, [8; 32], [10; 32], Policy::V1, BUDGET, 1).is_err());
    assert!(Reader::open(&temp.0, [8; 32], [9; 32], Policy::V1, BUDGET, 2).is_err());
}

#[test]
fn full_span_does_not_hide_missing_gap_between_individually_passing_periods() {
    let temp = Temp::new();
    let value = record(true, false);
    assert_eq!(value.training.outcome, Outcome::Passed);
    assert_eq!(value.later.outcome, Outcome::Passed);
    assert_eq!(value.outcome(), Outcome::Refused);
    assert!(value.evaluation.summary.missing_days > 0);
    assert!(!value.combined_qualifies(AdmissionStatusV1::Admitted));
    assert_eq!(
        write(&temp, value.clone()).record([9; 32], 0).unwrap(),
        &value
    );
}

#[test]
fn zero_trade_denominator_is_durable_and_never_promoted_by_institutional_pass() {
    let temp = Temp::new();
    let value = record(false, true);
    assert_eq!(value.outcome(), Outcome::Failed);
    assert_eq!(value.evaluation.summary.no_trade_days, 10);
    assert_eq!(value.evaluation.summary.observed_days, 10);
    assert_eq!(value.evaluation.summary.trades, 0);
    assert!(!value.combined_qualifies(AdmissionStatusV1::Admitted));
    assert_eq!(
        write(&temp, value.clone()).record([9; 32], 0).unwrap(),
        &value
    );
}

#[test]
fn receipt_missing_corrupt_changed_and_truncated_states_fail_closed() {
    let temp = Temp::new();
    assert!(Reader::open(&temp.0, [8; 32], [9; 32], Policy::V1, BUDGET, 1).is_err());
    let reader = write(&temp, record(false, false));
    let path = temp
        .0
        .join(NAMESPACE)
        .join(crate::identity_hex(&reader.receipt().identity));
    let body = path.join("body.bin");
    let original = std::fs::read(&body).unwrap();
    let mut corrupt = original.clone();
    *corrupt.get_mut(72).unwrap() ^= 1;
    std::fs::write(&body, &corrupt).unwrap();
    assert!(reader.require_current().is_err());
    assert!(Reader::open(&temp.0, [8; 32], [9; 32], Policy::V1, BUDGET, 1).is_err());
    std::fs::write(&body, &original).unwrap();
    std::fs::remove_file(path.join("complete.bin")).unwrap();
    assert!(Reader::open(&temp.0, [8; 32], [9; 32], Policy::V1, BUDGET, 1).is_err());
    for length in [0, 7, 71, 143, original.len() - 1] {
        assert!(
            decode(
                original.get(..length).unwrap(),
                [8; 32],
                [9; 32],
                Policy::V1,
                1
            )
            .is_err()
        );
    }
    let mut extra = original;
    extra.push(0);
    assert!(decode(&extra, [8; 32], [9; 32], Policy::V1, 1).is_err());
}

#[test]
fn conflicting_reuse_budget_and_parent_failure_do_not_publish_success() {
    let temp = Temp::new();
    let initial = write(&temp, record(false, false)).receipt();
    let changed = record(false, true);
    assert!(produce(&temp.0, [8; 32], [9; 32], vec![changed], BUDGET, || Ok(())).is_err());
    assert_eq!(
        Reader::open(&temp.0, [8; 32], [9; 32], Policy::V1, BUDGET, 1)
            .unwrap()
            .receipt(),
        initial
    );
    assert!(encode([8; 32], [9; 32], &[record(false, false)], 100).is_err());
    assert!(Reader::open(&temp.0, [8; 32], [9; 32], Policy::V1, 112, 1).is_err());
    let failed = Temp::new();
    assert!(
        produce(
            &failed.0,
            [8; 32],
            [9; 32],
            vec![record(false, false)],
            BUDGET,
            || Err("source changed".into())
        )
        .is_err()
    );
    assert!(!failed.0.join(NAMESPACE).exists());
}

#[test]
fn loss_streak_crosses_training_boundary_and_flat_days_do_not_reset_it() {
    let mut value = record(false, false);
    for index in [3, 4, 5] {
        value.sessions.get_mut(index).unwrap().pessimistic_paisa = -100;
        value.sessions.get_mut(index).unwrap().optimistic_paisa = -100;
    }
    value.sessions.get_mut(6).unwrap().pessimistic_paisa = 0;
    value.sessions.get_mut(6).unwrap().optimistic_paisa = 0;
    value.sessions.get_mut(6).unwrap().trades = 0;
    let family = value.evaluation.family;
    value.training = evaluate(
        Policy::V1,
        family,
        day(1, 6),
        day(1, 10),
        value.sessions.get(..5).unwrap(),
        true,
    );
    value.later = evaluate(
        Policy::V1,
        family,
        day(1, 13),
        day(1, 17),
        value.sessions.get(5..).unwrap(),
        true,
    );
    value.evaluation = evaluate(
        Policy::V1,
        family,
        day(1, 6),
        day(1, 17),
        &value.sessions,
        true,
    );
    assert_eq!(value.training.summary.longest_losing_streak, 2);
    assert_eq!(value.later.summary.longest_losing_streak, 1);
    assert_eq!(value.evaluation.summary.longest_losing_streak, 3);
    assert_eq!(value.outcome(), Outcome::Failed);
    let temp = Temp::new();
    assert_eq!(
        write(&temp, value.clone()).record([9; 32], 0).unwrap(),
        &value
    );
}

/// A mixed-policy set is refused, and the refusal is AUDITED.
///
/// # What this pins that the previous shape could not
///
/// The store named `Policy::V1` at four sites, so it could only ever hold the
/// version it was written with. Deriving the policy from the evidence fixed
/// that but moved the check ahead of `sweep_evidence::begin`, which silently
/// stopped this refusal producing an attempt row while every other refusal in
/// `produce` still made one. Keying on the first record restores it.
///
/// The empty case is deliberately NOT asserted to be audited: a set with no
/// evaluations declares no policy, therefore has no content address, therefore
/// has no identity for an attempt to be opened against. It refuses without a
/// row and that is structural. D-0602.
#[test]
fn a_mixed_policy_set_is_refused_and_an_empty_one_cannot_even_be_addressed() {
    let temp = Temp::new();
    let mut mixed = record(false, false);
    // One evaluation under a different policy than its two siblings.
    mixed.later.policy = Policy::V2;
    let mixed_result = produce(&temp.0, [8; 32], [9; 32], vec![mixed], BUDGET, || Ok(()));
    assert!(
        mixed_result
            .as_ref()
            .err()
            .is_some_and(|why| why.contains("one exact policy")),
        "a mixed-policy set must refuse naming the policy"
    );
    // An empty set names no policy at all, so it cannot be addressed.
    let empty_result = produce(&temp.0, [8; 32], [9; 32], Vec::new(), BUDGET, || Ok(()));
    assert!(
        empty_result
            .as_ref()
            .err()
            .is_some_and(|why| why.contains("at least one evaluated coordinate")),
        "an empty set declares no policy and cannot be addressed"
    );

    // A coherent set still round-trips, so neither refusal is overreach.
    let good = record(false, false);
    assert_eq!(
        write(&temp, good.clone()).record([9; 32], 0).unwrap(),
        &good
    );
}
