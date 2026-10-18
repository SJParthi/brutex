//! The bench rows of the base detector (FIB-3). A stub until FIB-3 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the base detector has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "detect: not built yet: it arrives with FIB-3");
    true
}
