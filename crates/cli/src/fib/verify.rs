//! `cli fib verify`: a run's files re-read and its identity re-derived (FIB-9). A stub until FIB-9 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib verify` until FIB-9 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("verify", "FIB-9", out)
}
