//! The bench rows of the walker (FIB-5). A stub until FIB-5 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the walker has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "walk: not built yet: it arrives with FIB-5");
    true
}
