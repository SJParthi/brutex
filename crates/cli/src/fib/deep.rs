//! `cli fib deep`: the deep re-walk of stage-A survivors on GDFL seconds at the worst price (FIB-9). A stub until FIB-9 fills it: it refuses by name,
//! exits FAILED and writes nothing (`CLAUDE.md` §4).

/// Refuses `fib deep` until FIB-9 builds it.
pub(super) fn command(_arguments: &[&str], out: &mut String) -> u8 {
    super::not_built("deep", "FIB-9", out)
}
