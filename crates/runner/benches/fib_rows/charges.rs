//! The bench rows of the precise charges (FIB-11). A stub until FIB-11 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the precise charges has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "charges: not built yet: it arrives with FIB-11");
    true
}
