//! `cli fib day-check`: one stock-month's day input checked against every authority, read-only (FIB-8). A stub until FIB-8 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib day-check` until FIB-8 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("day-check", "FIB-8", out)
}
