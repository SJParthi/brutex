//! `cli tix-rebuild` — the verb is registered, and refuses by name until it is
//! built (WP-15, D-3340).
//!
//! # Why a verb that does nothing exists
//!
//! The fix plan gives the `.tix` rebuild to WP-15 and the census verbs to WP-10,
//! and both touch this crate's one command table. Registering the word here, in
//! the package that owns the table, means WP-15 changes ONE file — this one —
//! and never the dispatch, the usage or `COMMANDS`, which three tests hold in
//! step with each other.
//!
//! Until then the verb answers what it is and that it did nothing, and exits
//! [`crate::FAILED`]: a script that called it must not read a refusal as a
//! rebuilt index. `CLAUDE.md` §4 — refuse and name the reason, never return
//! success over work that was not done.

use std::fmt::Write as _;

use crate::FAILED;

/// The refusal, naming the package and the decision that will build it.
pub(crate) const NOT_BUILT: &str = "`cli tix-rebuild` is registered for WP-15 (D-3340: .tix for \
     every index one-second month) and is not built in this binary";

/// Runs the verb: refuses, interpreting no argument and touching no file.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    let _ = writeln!(
        out,
        "TIX REBUILD REFUSED: {NOT_BUILT}. Nothing was read and nothing was \
         written; the {} argument(s) given were not interpreted.",
        arguments.len()
    );
    FAILED
}

#[cfg(test)]
mod tests {
    use super::{NOT_BUILT, command};
    use crate::FAILED;

    /// The stub refuses by name, says nothing was done, and fails the exit
    /// code — whatever it is handed.
    #[test]
    fn the_registered_verb_refuses_by_name_and_does_nothing() {
        for arguments in [&["NIFTY"][..], &["a", "b", "c"][..]] {
            let mut out = String::new();
            assert_eq!(command(arguments, &mut out), FAILED);
            assert!(out.contains(NOT_BUILT), "{out}");
            assert!(
                out.contains("Nothing was read and nothing was written"),
                "{out}"
            );
            assert!(
                out.contains(&format!("{} argument(s)", arguments.len())),
                "{out}"
            );
        }
    }
}
