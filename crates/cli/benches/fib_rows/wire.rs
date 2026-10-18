//! The bench rows of the new-work authorities (FIB-10). A stub until FIB-10 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the new-work authorities has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "wire: not built yet: it arrives with FIB-10");
    true
}
