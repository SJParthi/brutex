//! `cli fib wf`: the walk-forward on unseen months (FIB-13). A stub until FIB-13 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib wf` until FIB-13 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("wf", "FIB-13", out)
}
