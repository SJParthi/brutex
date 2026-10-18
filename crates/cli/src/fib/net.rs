//! `cli fib net`: every trade priced net of the precise charges (FIB-11). A stub until FIB-11 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib net` until FIB-11 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("net", "FIB-11", out)
}
