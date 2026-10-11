//! One rule for every verb that writes the store: a build that carries no
//! commit stamp does not, unless told to (DB-B71, WP-81).
//!
//! # The failure
//!
//! A dirty-tree build carries no commit (`build.rs` stamps only a tree that
//! equals HEAD), so every journal line and provenance record it writes says
//! `commit=unstamped`: nobody can later say which code produced those bars.
//! Such a `cli` still imported — a test shadow's `imports/gdfl.journal` held
//! 14 `run ... commit=unstamped` lines, each followed by its days — and
//! nothing stopped it from doing the same to the operator's store.
//!
//! # The rule, in one place
//!
//! Every verb reaches its store root through a closure. [`root_for`] is that
//! closure for the binary: when the invocation WRITES the store
//! ([`writes_the_store`]) and the build is unstamped, it answers the refusal
//! instead of the root, so the verb prints it under its own heading and
//! exits [`crate::FAILED`] after its own argument checks and before it opens
//! anything. A read-only form of a writing verb — a plan, a dry run — is not
//! refused: it writes nothing a commit would have to answer for.
//!
//! `BRUTEX_ALLOW_UNSTAMPED_WRITE=1` lets an unstamped build write on purpose.
//! Its lines still say `unstamped`: allowed, never disguised.
//!
//! # What is and is not covered
//!
//! Every `cli` verb. The API's own writers (a Zerodha pull, the autopilot) are
//! NOT refused here: an unstamped API names itself on every response
//! (`x-brutex-build: unstamped`) and in red on every page, and its provenance
//! records say so, but it still pulls. [`unstamped_write_refusal`] is public
//! so a writer inside the API can ask the same question.

use std::ffi::OsStr;
use std::path::PathBuf;

/// The variable that lets a build with no commit stamp write the store
/// anyway. Set it to `1`, and only for a deliberate one-off.
pub const ALLOW_UNSTAMPED_VAR: &str = "BRUTEX_ALLOW_UNSTAMPED_WRITE";

/// Why a store write by this build is refused, or `None` when it may write.
///
/// A pure function of the stamp and the allowance, so a test reaches both
/// arms in a stamped and an unstamped build alike.
#[must_use]
pub fn unstamped_write_refusal(commit: Option<&str>, allow: Option<&OsStr>) -> Option<String> {
    if commit.is_some() || allow.is_some_and(|value| value == "1") {
        return None;
    }
    Some(format!(
        "this build carries no commit stamp (a dirty working tree or no .git), so \
         nothing could say which code wrote the store; build from a clean commit, \
         or set {ALLOW_UNSTAMPED_VAR}=1 to write as `unstamped` deliberately (DB-B71)"
    ))
}

/// Every verb that can write bars, sidecars, a census, a journal or a ledger
/// of the store, in [`crate::COMMANDS`] order.
///
/// `scrub-blocks` IS one. It writes no bar, sidecar or census byte, but it
/// appends to `audit/scrub-blocks.<vendor>.log` and `.cursor`, and `/scrub.json`,
/// `/disk.json`, `/db` and `/ingest` serve what those say: "every block was read
/// and matched" is a claim about the store that some code made, and a run line
/// saying `commit=unstamped` cannot say which. It was listed as a reader while
/// it wrote those two files.
pub(crate) const WRITERS: [&str; 8] = [
    "census-migrate",
    "census-recount",
    "gdfl-dayflags",
    "gdfl-import",
    "gdfl-rekey",
    "scrub-blocks",
    "tix-rebuild",
    "zerodha-repair",
];

/// Whether `verb`, invoked with `arguments`, writes the store.
///
/// A verb in [`WRITERS`] writes unless its arguments choose its read-only
/// form: `tix-rebuild ... --dry-run`, `gdfl-rekey` without `--apply`, and
/// `zerodha-repair` without `--from-store` each state in their own module
/// that they write nothing.
#[must_use]
pub(crate) fn writes_the_store(verb: &str, arguments: &[&str]) -> bool {
    match verb {
        "tix-rebuild" => !arguments.contains(&"--dry-run"),
        "gdfl-rekey" => arguments.contains(&"--apply"),
        "zerodha-repair" => arguments.contains(&"--from-store"),
        other => WRITERS.contains(&other),
    }
}

/// [`root_for`] over a named stamp, allowance and root, so every arm is
/// driven by a test without touching the process environment.
pub(crate) fn root_with(
    verb: &str,
    arguments: &[&str],
    commit: Option<&str>,
    allow: Option<&OsStr>,
    root: &dyn Fn() -> Result<PathBuf, String>,
) -> Result<PathBuf, String> {
    if writes_the_store(verb, arguments)
        && let Some(why) = unstamped_write_refusal(commit, allow)
    {
        return Err(why);
    }
    root()
}

/// The store root `verb` may use for this invocation: the CLI's own, or the
/// refusal of an unstamped build that would write to it.
pub(crate) fn root_for<'a>(
    verb: &'a str,
    arguments: &'a [&'a str],
) -> impl Fn() -> Result<PathBuf, String> + 'a {
    move || {
        root_with(
            verb,
            arguments,
            crate::commit_stamp(),
            std::env::var_os(ALLOW_UNSTAMPED_VAR).as_deref(),
            &crate::store_root,
        )
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;

    const STAMP: &str = "0123456789abcdef0123456789abcdef01234567";

    /// **AN UNSTAMPED BUILD DOES NOT WRITE THE STORE UNLESS TOLD TO
    /// (DB-B71).** Both arms of the rule, as a function of the stamp and the
    /// allowance; the allowance is exactly `1`.
    #[test]
    fn an_unstamped_build_is_refused_unless_allowed() {
        assert_eq!(unstamped_write_refusal(Some(STAMP), None), None);
        let why = unstamped_write_refusal(None, None).expect("refused");
        assert!(
            why.contains(ALLOW_UNSTAMPED_VAR) && why.contains("DB-B71"),
            "{why}"
        );
        for not_one in ["0", "", "true", " 1", "11"] {
            assert!(
                unstamped_write_refusal(None, Some(OsStr::new(not_one))).is_some(),
                "{not_one:?}"
            );
        }
        assert_eq!(unstamped_write_refusal(None, Some(OsStr::new("1"))), None);
    }

    /// **EVERY VERB IS A WRITER OR IS NOT, BY NAME, AND THE LIST IS THE
    /// BINARY'S.** A verb added to `COMMANDS` must be put on one side here, so
    /// a new store writer cannot arrive ungated by being forgotten.
    #[test]
    fn every_command_is_classified_as_a_store_writer_or_not() {
        let readers = [
            "audit",
            "audit-audited-range",
            "audit-range",
            "audit-stored",
            "auto",
            "auto-stored",
            "boolean-campaign-stored",
            "boolean-catalog-stored",
            "boolean-grammar-campaign-stored",
            "boolean-oos-stored",
            "boolean-qualified-campaign-stored",
            "boolean-qualified-search-stored",
            "checksum-audit-stored",
            "descend",
            "elite",
            "expression-backtest-stored",
            "expression-search-stored",
            "expression-stored",
            "fold-audit",
            "ledger-all",
            "ledger-v6",
            "ledger-v6-replay",
            "policy-check",
            "pool",
            "pool-oos",
            "range-all",
            "range-rung",
            "research-plan",
            "results",
            "screen",
            "sweep",
            "sweep-all",
            "sweep-audited-stored",
            "sweep-stored",
            "top",
            "verify",
        ];
        for verb in crate::COMMANDS {
            assert_ne!(
                WRITERS.contains(&verb),
                readers.contains(&verb),
                "`{verb}` must be exactly one of a store writer and not one"
            );
        }
        for verb in WRITERS.iter().chain(&readers) {
            assert!(crate::COMMANDS.contains(verb), "`{verb}` is no command");
        }
        assert!(
            WRITERS.windows(2).all(|pair| pair.first() < pair.last()),
            "kept sorted"
        );
    }

    /// The read-only form of a writing verb writes nothing and is not gated;
    /// its writing form is. A verb that is no writer never is.
    #[test]
    fn a_plan_or_a_dry_run_is_not_a_write_and_its_applying_form_is() {
        for (verb, arguments, writes) in [
            (
                "gdfl-import",
                &["indices", "zip", "2024-04-01", "2024-04-01"][..],
                true,
            ),
            ("census-migrate", &["gdfl"], true),
            ("census-migrate", &["gdfl", "--seal"], true),
            ("census-recount", &["gdfl", "--from", "2019-11-14"], true),
            (
                "gdfl-dayflags",
                &["indices", "2019-11-01", "2019-11-30"],
                true,
            ),
            (
                "tix-rebuild",
                &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-11", "2019-11"],
                true,
            ),
            (
                "tix-rebuild",
                &[
                    "gdfl",
                    "NSE/INDEX/NIFTY",
                    "1s",
                    "2019-11",
                    "2019-11",
                    "--dry-run",
                ],
                false,
            ),
            ("gdfl-rekey", &["--plan"], false),
            ("gdfl-rekey", &["--apply", "--underlying", "NIFTY"], true),
            (
                "zerodha-repair",
                &["NIFTY", "2024-01-01", "2024-01-31"],
                false,
            ),
            (
                "zerodha-repair",
                &[
                    "NIFTY",
                    "2024-01-01",
                    "2024-01-31",
                    "--from-store",
                    "/x",
                    "--reason",
                    "y",
                ],
                true,
            ),
            ("scrub-blocks", &["gdfl"], true),
            ("top", &[], false),
            ("no-such-verb", &["--apply"], false),
        ] {
            assert_eq!(
                writes_the_store(verb, arguments),
                writes,
                "{verb} {arguments:?}"
            );
        }
    }

    /// The closure a verb is handed: the refusal for a writing invocation of
    /// an unstamped build, the root otherwise — and the root is not even
    /// asked for when the write is refused.
    #[test]
    fn the_root_is_refused_to_an_unstamped_write_and_given_otherwise() {
        let root = || Ok(PathBuf::from("/a/store"));
        let asked = std::cell::Cell::new(false);
        let watched = || -> Result<PathBuf, String> {
            asked.set(true);
            Ok(PathBuf::from("/a/store"))
        };
        let refused = root_with("census-migrate", &["gdfl"], None, None, &watched)
            .expect_err("unstamped write");
        assert!(refused.contains("carries no commit stamp"), "{refused}");
        assert!(!asked.get(), "the root was asked for by a refused write");
        assert_eq!(
            root_with("census-migrate", &["gdfl"], Some(STAMP), None, &root),
            Ok(PathBuf::from("/a/store"))
        );
        assert_eq!(
            root_with(
                "census-migrate",
                &["gdfl"],
                None,
                Some(OsStr::new("1")),
                &root
            ),
            Ok(PathBuf::from("/a/store"))
        );
        assert_eq!(
            root_with("gdfl-rekey", &["--plan"], None, None, &root),
            Ok(PathBuf::from("/a/store")),
            "a plan writes nothing"
        );
        assert_eq!(
            root_with("gdfl-rekey", &["--apply"], None, None, &|| Err(
                "x".to_owned()
            )),
            Err(unstamped_write_refusal(None, None).expect("refused")),
            "the stamp is asked before the root"
        );
        // The binary's own closure, on whatever this test build is: a reader
        // is given the CLI's root or its own refusal, never the stamp's.
        let asked = root_for("top", &[])();
        assert!(
            asked
                .as_ref()
                .err()
                .is_none_or(|why| !why.contains("commit stamp")),
            "{asked:?}"
        );
    }
}
