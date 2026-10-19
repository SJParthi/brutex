#![cfg(test)]
//! `sweep-stored`'s exit code, lifecycle phase and invocation-audit phase
//! agree with the sweep evidence it wrote (repro-2, extends D-2720).
//!
//! Generated stores only (`crate::audited_stored::with_warmed_store`): invented
//! constant-price sessions, never a vendor row.
#![allow(clippy::expect_used)]

use super::*;
use std::cell::RefCell;
use std::path::PathBuf;

thread_local! {
    /// The generated store the fn-pointer command below sweeps. A
    /// `stored_month_arm` command is a plain `fn`, so it cannot capture one.
    static ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// `sweep_stored` with the generated store in place of `$BRUTEX_STORE` and a
/// generated commit in place of the build stamp: the same kernel, the same
/// rendering of a refusal.
fn generated_sweep_stored(
    vendor: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> String {
    let root = ROOT
        .with(|root| root.borrow().clone())
        .expect("a generated root is set");
    let request = StoredSweepRequest {
        root,
        vendor: parse_vendor(vendor).expect("a known feed"),
        underlying,
        rung,
        year,
        month,
        min_hits,
        commit: "generated-sweep-exit-fixture",
    };
    match sweep_stored_kernel(request) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// The same for `audit-stored`, which shares the arm.
fn generated_audit_stored(
    vendor: &str,
    underlying: &str,
    rung: &str,
    year: u16,
    month: u8,
    min_hits: u64,
) -> String {
    let root = ROOT
        .with(|root| root.borrow().clone())
        .expect("a generated root is set");
    let request = StoredSweepRequest {
        root,
        vendor: parse_vendor(vendor).expect("a known feed"),
        underlying,
        rung,
        year,
        month,
        min_hits,
        commit: "generated-audit-exit-fixture",
    };
    match audit_stored_kernel(request) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

/// One arm run over a generated store: (exit code, page, latest evidence).
fn arm_over(
    root: &std::path::Path,
    command: fn(&str, &str, &str, u16, u8, u64) -> String,
    min_hits: &str,
) -> (u8, String, Option<sweep_evidence::Evidence>) {
    ROOT.with(|slot| *slot.borrow_mut() = Some(root.to_path_buf()));
    let mut page = String::new();
    let code = stored_month_arm(
        &mut page,
        command,
        ("zerodha", "NIFTY", "1min"),
        ("2025", "5", min_hits),
    );
    ROOT.with(|slot| *slot.borrow_mut() = None);
    let evidence = sweep_evidence::latest(root, 1_048_576).expect("the evidence reads");
    (code, page, evidence)
}

/// A HALTED `sweep-stored` WALK EXITS FAILED, AND EVERY RECORD SAYS SO.
///
/// A ceiling of one candidate halts the generated month at k=1. Before the
/// fix the arm exited `OK`: the verdict row read `NO closure is incomplete ...`
/// and `untrustworthy` compared the whole rest with `NO`, so the lifecycle
/// event said `completed` and the invocation audit `Completed` while the
/// evidence said `Halted` (repro-2). `audit-stored` over the same store
/// already exited `FAILED` and still does: the siblings now agree.
#[test]
fn a_halted_sweep_stored_walk_exits_failed() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::knobs::set("BRUTEX_CEILING", "1");
    crate::audited_stored::with_warmed_store(|root| {
        let (code, page, evidence) = arm_over(root, generated_sweep_stored, "1");
        let evidence = evidence.expect("the walk recorded an attempt");
        assert_eq!(
            evidence.completion,
            sweep_evidence::Completion::Halted,
            "premise: a one-candidate ceiling halts the walk:\n{page}"
        );
        assert!(!carries_refusal(&page), "premise: no refusal line:\n{page}");
        assert!(untrustworthy(&page), "{page}");
        assert_eq!(code, FAILED, "{page}");
        assert_ne!(lifecycle_phase(code), "completed");
        assert_eq!(lifecycle_phase(code), "refused");
        assert_ne!(audit_phase(code), operation_audit::Phase::Completed);

        let (audit_code, audit_page, audit_evidence) = arm_over(root, generated_audit_stored, "1");
        assert_eq!(audit_code, FAILED, "{audit_page}");
        assert_eq!(
            audit_evidence
                .expect("the audit recorded an attempt")
                .completion,
            sweep_evidence::Completion::Halted,
            "{audit_page}"
        );
    });
    crate::knobs::clear_all();
}

/// A COMPLETE walk over the same generated month still exits `OK`, and its
/// lifecycle and audit phases say completed: the predicate moved only the
/// halted case.
#[test]
fn a_complete_sweep_stored_walk_still_exits_ok() {
    let _knobs = crate::knobs::serially();
    crate::knobs::clear_all();
    crate::audited_stored::with_warmed_store(|root| {
        let (code, page, evidence) = arm_over(root, generated_sweep_stored, "18446744073709551615");
        assert_eq!(
            evidence.expect("the walk recorded an attempt").completion,
            sweep_evidence::Completion::Completed,
            "{page}"
        );
        assert_eq!(code, OK, "{page}");
        assert_eq!(lifecycle_phase(code), "completed");
        assert_eq!(audit_phase(code), operation_audit::Phase::Completed);
    });
    crate::knobs::clear_all();
}

/// `untrustworthy` reads the verdict cell -- the first word after the label --
/// so the halted row with its closure note counts, and prose does not.
#[test]
fn untrustworthy_reads_the_first_word_after_its_label() {
    assert!(untrustworthy(
        "  trustworthy as a whole answer            NO  closure is incomplete on the halted frontier"
    ));
    assert!(untrustworthy("  trustworthy as a whole answer   NO"));
    assert!(!untrustworthy(
        "  trustworthy as a whole answer           yes  "
    ));
    assert!(!untrustworthy(
        "trustworthy as a whole answer NO, in prose here"
    ));
    assert!(!untrustworthy("trustworthy as a whole answer NOT here"));
    assert!(!untrustworthy("  trustworthy as a whole answer"));
    assert!(!untrustworthy(
        "  untrustworthy as a whole answer         NO"
    ));
    assert_eq!(stored_month_exit("refused: no month\n"), FAILED);
    assert_eq!(
        stored_month_exit("  trustworthy as a whole answer   yes\n"),
        OK
    );
}
