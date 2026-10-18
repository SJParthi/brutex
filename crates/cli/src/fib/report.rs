//! `cli fib report`: the report and its pages, long and short side by side (FIB-14). A stub until FIB-14 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib report` until FIB-14 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("report", "FIB-14", out)
}
