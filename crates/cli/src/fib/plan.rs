//! `cli fib plan`: the measured projection of a run, refused past its budget (FIB-12). A stub until FIB-12 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib plan` until FIB-12 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("plan", "FIB-12", out)
}
