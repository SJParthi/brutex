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
//! Every `cli` verb, by name ([`WRITERS`]; the classification test fails when a
//! verb of `COMMANDS` is on neither side). The API asks the same question
//! through [`unstamped_write_refusal`]: an unstamped `api` answers 403 to
//! every request that is not a GET or a HEAD, grounds its autopilot and does
//! not resume a spot recovery at start (`api::server::WriteGate`), and names
//! itself on every response (`x-brutex-build: unstamped`).

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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
///
/// `tickstore-write` (R4-08) is one: it writes the tick store's day files and
/// its log, which the importer reads as the vendor's days, and each log line
/// names the commit that wrote them.
///
/// `alerts` is one in two of its forms (IA-11a, merged in beside this table):
/// `--ack` appends an acknowledgement to `audit/alerts` and `--replace-damaged`
/// puts a new queue in its place. Its listings read and write nothing.
pub(crate) const WRITERS: [&str; 13] = [
    "alerts",
    "census-migrate",
    "census-recount",
    "gdfl-dayflags",
    "gdfl-import",
    "gdfl-rekey",
    "msum-build",
    "scrub-blocks",
    "srt-build",
    "tickstore-write",
    "tix-rebuild",
    "zerodha-gaps",
    "zerodha-repair",
];

/// Whether `verb`, invoked with `arguments`, writes the store.
///
/// A verb in [`WRITERS`] writes unless its arguments choose its read-only
/// form: `tix-rebuild ... --dry-run`, `srt-build ... --dry-run`,
/// `msum-build ... --dry-run`, `gdfl-rekey` without `--apply`, `zerodha-gaps`
/// without `--apply`, and `zerodha-repair` without `--from-store` each state
/// in their own module that they write nothing; `alerts` writes only as
/// `alerts --ack <n>` and `alerts --replace-damaged`, the flag first, which is
/// the one place its own parse takes it.
#[must_use]
pub(crate) fn writes_the_store(verb: &str, arguments: &[&str]) -> bool {
    match verb {
        "alerts" => matches!(arguments.first(), Some(&("--ack" | "--replace-damaged"))),
        "msum-build" | "srt-build" | "tix-rebuild" => !arguments.contains(&"--dry-run"),
        "gdfl-rekey" | "zerodha-gaps" => arguments.contains(&"--apply"),
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
    let writes = writes_the_store(verb, arguments);
    if writes && let Some(why) = unstamped_write_refusal(commit, allow) {
        return Err(why);
    }
    let root = root()?;
    if writes {
        writable(&root)?;
    }
    Ok(root)
}

/// Refuses, by name, a store root no verb may write: one whose `bars` is a
/// symbolic link (`linked_store_root`: a test server's shadow, whose `bars`
/// is the real store's, RA-298) or is not a directory. Asked once, after the
/// stamp and before the verb opens anything, by [`root_with`] for every
/// writing invocation and by `gdfl-import` for its own root; a read-only
/// form is never refused by it, because a shadow exists to be read. The
/// rule is `store::safe_open::refuse_linked_store_root` (R3-16); the month
/// writer's own refusal of any link below the root (CE-62) stays behind it.
pub(crate) fn writable(root: &Path) -> Result<(), String> {
    store::safe_open::refuse_linked_store_root(root).map_err(|why| why.to_string())
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
            // WP-13d's verb, merged in beside this table: it judges stored
            // minutes against stored day bars and "writes nothing" (its
            // header); it opens no file for writing.
            "series-identity",
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
            (
                "srt-build",
                &["gdfl", "NSE/INDEX/NIFTY", "1s", "2019-11", "2019-11"],
                true,
            ),
            (
                "srt-build",
                &[
                    "gdfl",
                    "NSE/FNO/NIFTY/*",
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
                "zerodha-gaps",
                &["NIFTY", "2024-01-01", "2024-01-31"],
                false,
            ),
            (
                "zerodha-gaps",
                &["ALL", "2024-01-01", "2024-01-31", "--apply"],
                true,
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
            ("alerts", &[], false),
            ("alerts", &["--older"], false),
            ("alerts", &["--before", "5"], false),
            ("alerts", &["--file", "/x/alerts.damaged-1"], false),
            ("alerts", &["--older", "--ack"], false),
            ("alerts", &["--ack", "3"], true),
            ("alerts", &["--replace-damaged"], true),
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

    /// **EVERY WRITING VERB REFUSES A STORE WHOSE BARS FOLDER IS A LINK**
    /// (R3-16, RA-298, FM-198): the shape `state/fix/shadow.sh` makes. Every
    /// verb of [`WRITERS`], in its writing form and with a stamp, is refused
    /// `linked_store_root` naming the link; the read-only forms are given the
    /// root, because a shadow exists to be read; and the same verbs are given
    /// a root whose `bars` is its own. Nothing is created through the link.
    #[test]
    fn every_writing_verb_refuses_a_store_whose_bars_folder_is_a_link() {
        let scratch =
            std::env::temp_dir().join(format!("brutex-cli-linked-root-{}", std::process::id()));
        let real = scratch.join("real");
        let shadow = scratch.join("shadow");
        std::fs::create_dir_all(real.join("bars")).unwrap();
        std::fs::create_dir_all(&shadow).unwrap();
        let link = shadow.join("bars");
        if std::fs::symlink_metadata(&link).is_err() {
            std::os::unix::fs::symlink(real.join("bars"), &link).unwrap();
        }
        let shadowed = || Ok(shadow.clone());
        let owned = || Ok(real.clone());
        // `tickstore-write` (R4-08) never asks for this root: it writes the
        // tick store under its own ROOT argument, and its own refusal of a
        // linked folder there is `pull::tickstore_write`'s `linked_folder`
        // (`a_tick_store_folder_that_is_a_link_is_never_written_through`).
        // It is listed so the table stays every writer, once.
        let writing: [(&str, &[&str]); 13] = [
            ("alerts", &["--ack", "1"]),
            ("census-migrate", &["gdfl"]),
            ("census-recount", &["gdfl"]),
            ("gdfl-dayflags", &[]),
            ("gdfl-import", &[]),
            ("gdfl-rekey", &["--apply"]),
            ("msum-build", &[]),
            ("scrub-blocks", &[]),
            ("srt-build", &[]),
            ("tickstore-write", &["ZIP", "ROOT"]),
            ("tix-rebuild", &[]),
            ("zerodha-gaps", &["--apply"]),
            ("zerodha-repair", &["--from-store"]),
        ];
        assert_eq!(writing.map(|(verb, _)| verb), WRITERS, "every writer, once");
        for (verb, arguments) in writing {
            let refused = root_with(verb, arguments, Some(STAMP), None, &shadowed)
                .expect_err("a linked store is refused");
            assert!(
                refused.starts_with("linked_store_root: "),
                "{verb}: {refused}"
            );
            assert!(
                refused.contains(&link.display().to_string()),
                "{verb}: {refused}"
            );
            assert_eq!(
                root_with(verb, arguments, Some(STAMP), None, &owned),
                Ok(real.clone()),
                "{verb}: a store with its own bars is written"
            );
        }
        for (verb, arguments) in [
            ("alerts", &[][..]),
            ("gdfl-rekey", &["--plan"][..]),
            ("msum-build", &["--dry-run"][..]),
            ("srt-build", &["--dry-run"][..]),
            ("tix-rebuild", &["--dry-run"][..]),
            ("zerodha-gaps", &[][..]),
            ("zerodha-repair", &[][..]),
            ("top", &[][..]),
        ] {
            assert_eq!(
                root_with(verb, arguments, Some(STAMP), None, &shadowed),
                Ok(shadow.clone()),
                "{verb} {arguments:?} reads, and a shadow is there to be read"
            );
        }
        assert_eq!(
            std::fs::read_dir(real.join("bars")).unwrap().count(),
            0,
            "nothing was created through the link"
        );
        assert!(writable(&shadow).unwrap_err().contains("linked_store_root"));
        let _ = std::fs::remove_dir_all(&scratch);
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
