//! The bench rows of the Winner Gate (FIB-7). A stub until FIB-7 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the Winner Gate has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "gate: not built yet: it arrives with FIB-7");
    true
}
