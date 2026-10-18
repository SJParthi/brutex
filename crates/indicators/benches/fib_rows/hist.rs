//! The bench rows of the own-past histograms (FIB-2). A stub until FIB-2 fills it: it measures
//! nothing and says so.

use std::fmt::Write as _;

/// Reports that the own-past histograms has no row yet.
pub(crate) fn report(_host: &crate::harness::Host, out: &mut String) -> bool {
    let _ = writeln!(out, "hist: not built yet: it arrives with FIB-2");
    true
}
