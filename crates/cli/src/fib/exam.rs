//! `cli fib exam`: the sealed exam, opened once after the operator's yes (FIB-13). A stub until FIB-13 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib exam` until FIB-13 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("exam", "FIB-13", out)
}
