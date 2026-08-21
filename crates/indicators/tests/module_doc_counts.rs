//! Every module header that quotes a position count is checked against the count.
//!
//! # The drift this closes, measured
//!
//! `crates/indicators/tests/shared_core_doc.rs` is a working doc-count gate and
//! it covers exactly one document: `docs/10-shared-core.md`. Every other quoted
//! count in this crate lives in a MODULE DOC COMMENT, and nothing checked any of
//! them. An adversarial audit measured two that had drifted:
//!
//! | Module | Header claimed | `positions()` returned |
//! |---|---:|---:|
//! | `daily.rs` | 41 | **44** |
//! | `fib.rs` | 26 | **27** |
//!
//! `fib.rs` was the clearer of the two: its header said 26 and **its own table,
//! two lines below, listed 9 + 7 + 11**. The document contradicted itself and
//! shipped that way. `daily.rs` conflated two real numbers — `plan()` has 41
//! entries and `positions()` returns those 41 plus three CPR-width states.
//!
//! # Why a test and not a careful edit
//!
//! Correcting the numbers fixes today. This fixes the class: the header is now
//! read from disk and compared against the array width the compiler enforces, so
//! a module that grows a position and does not say so fails the build.
//!
//! # What this does NOT cover
//!
//! Only the leading `**N vocabulary positions**` claim of each module listed
//! below. A count quoted mid-file, or in a different phrasing, is still
//! unchecked — and `crates/runner/src/rank.rs` quotes struct sizes that no test
//! reads. Those are named in `docs/11-findings.md` rather than silently left.

// The exception every test in this workspace takes, and here for the reason
// the crate's own test modules give at length: `panic!` expands to a panic
// written IN THIS CRATE, which `cargo llvm-cov` counts as a region no passing
// run can enter. `expect` panics inside core, which is not instrumented -- the
// same failure, the same message, and no dead region left behind.
#![allow(
    clippy::expect_used,
    reason = "a test that cannot fail cannot check anything, and `expect` leaves \
              no uncoverable region behind the way `panic!` does."
)]

use std::fs;

/// One module, the file to read, and the count its own `positions()` returns.
///
/// The right-hand number is NOT a literal repeated from the source. It is the
/// length of the array `positions()` returns, which the compiler enforces
/// against the ladders that build it — so this table cannot drift from the code
/// even if the header does.
fn modules() -> Vec<(&'static str, &'static str, usize)> {
    vec![
        (
            "daily",
            "src/daily.rs",
            indicators::daily::positions().len(),
        ),
        ("fib", "src/fib.rs", indicators::fib::positions().len()),
        ("orb", "src/orb.rs", indicators::orb::positions().len()),
        (
            "pattern",
            "src/pattern.rs",
            indicators::pattern::positions().len(),
        ),
        (
            "session",
            "src/session.rs",
            indicators::session::positions().len(),
        ),
        ("vwap", "src/vwap.rs", indicators::vwap::positions().len()),
    ]
}

/// The number a module's header claims, if it makes the claim in the checked
/// form.
///
/// Reads the leading `//!` block only. A module that does not state a count is
/// reported as `None` and handled by the caller, rather than defaulted to zero —
/// "did not say" and "said zero" are different facts and the second would pass
/// silently.
fn claimed(source: &str) -> Option<usize> {
    for line in source.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("//!") {
            // The header block has ended. Stop rather than scanning the whole
            // file: a count quoted inside a function's doc is a different claim
            // about a different thing.
            if trimmed.is_empty() {
                continue;
            }
            break;
        }
        let Some(at) = trimmed.find("**") else {
            continue;
        };
        let rest = trimmed.get(at.saturating_add(2)..)?;
        let Some(end) = rest.find(' ') else { continue };
        let word = rest.get(..end)?;
        if let Ok(n) = word.parse::<usize>()
            && rest.contains("vocabulary position")
        {
            return Some(n);
        }
    }
    None
}

#[test]
fn every_module_header_states_the_position_count_it_actually_emits() {
    let mut checked = 0_usize;
    for (name, path, actual) in modules() {
        let source = fs::read_to_string(path)
            .expect("every listed module must be readable from the crate root");
        let stated = claimed(&source);
        assert!(
            stated.is_some(),
            "{name}: the header of {path} no longer states a position count in the \
             form `**N vocabulary positions**`. Either restore the claim or remove \
             this module from the list -- a header that quietly stops making the \
             claim is how the check stops checking."
        );
        let stated = stated.expect("asserted to be Some on the line above");
        assert_eq!(
            stated, actual,
            "{name}: the header of {path} claims {stated} vocabulary positions and \
             `positions()` returns {actual}. This is the drift that shipped twice: \
             daily.rs said 41 against 44, and fib.rs said 26 against 27 while its \
             own table listed 9 + 7 + 11."
        );
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked, 6,
        "six modules state a count and all six must be read; a module dropped from \
         the list is a module nobody checks"
    );
}
