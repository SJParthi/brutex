//! `cli fib sample`: a sampled configuration timed for the plan (FIB-9). A stub until FIB-9 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib sample` until FIB-9 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("sample", "FIB-9", out)
}
