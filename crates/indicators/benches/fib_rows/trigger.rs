//! The bench rows of the trigger and the plan (FIB-4). A stub until FIB-4 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the trigger and the plan has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "trigger: not built yet: it arrives with FIB-4");
    true
}
