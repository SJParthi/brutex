//! The bench rows of the lattice (FIB-6). A stub until FIB-6 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the lattice has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "lattice: not built yet: it arrives with FIB-6");
    true
}
