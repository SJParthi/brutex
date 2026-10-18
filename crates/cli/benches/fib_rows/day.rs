//! The bench rows of the day input (FIB-8). A stub until FIB-8 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the day input has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "day: not built yet: it arrives with FIB-8");
    true
}
