//! The bench rows of the run orchestration (FIB-9). A stub until FIB-9 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the run orchestration has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "run: not built yet: it arrives with FIB-9");
    true
}
