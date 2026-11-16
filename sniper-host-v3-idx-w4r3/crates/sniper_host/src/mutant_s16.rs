//! MUTANT m-S16 (S2): this file exists only to prove the build test catches a host that references `FillPath::cut_at`. It is
//! compiled only under `--features m_s16`, and the binary calls it, so the symbol reaches the binary; `tests/no_cut_at.rs` must
//! then FAIL. It is never part of a standard build.

use common_fill::{FillPath, SecIn, Tape};

/// Calls the forbidden function.
///
/// # Errors
/// The crate's refusal.
pub fn mutant_entry(p: &FillPath, tape: &Tape<'_, SecIn>) -> Result<u64, common_fill::FillErr> {
    let mut steps = 0_u32;
    let f = p.cut_at(tape, 0, 1, 0, &mut steps)?;
    Ok(f.units)
}
