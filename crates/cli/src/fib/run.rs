//! `cli fib run`: stage A over the point-in-time pool, one direction per
//! search (FIB-9). A stub until FIB-9 fills it: it refuses by name, exits
//! FAILED and writes nothing (`CLAUDE.md` §4). The router has already refused a
//! run without `--direction long|short` (FM-F007).

use vocab::fib_cube::Search;

/// Refuses `fib run` until FIB-9 builds it.
pub(super) fn command(_direction: Search, _arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("run", "FIB-9", out)
}
