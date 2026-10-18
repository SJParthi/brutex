//! `cli fib`: the router of the Fib Flag Breakout engine's sub-verbs (FIB-1;
//! D-6306, D-6307).
//!
//! # The sub-verbs and their owners
//!
//! | Sub-verb | File | Owner |
//! |---|---|---|
//! | `day-check` | `day.rs` | FIB-8 |
//! | `run` | `run.rs` | FIB-9 |
//! | `deep` | `deep.rs` | FIB-9 |
//! | `sample` | `sample.rs` | FIB-9 |
//! | `verify` | `verify.rs` | FIB-9 |
//! | `net` | `net.rs` | FIB-11 |
//! | `plan` | `plan.rs` | FIB-12 |
//! | `wf` | `wf.rs` | FIB-13 |
//! | `exam` | `exam.rs` | FIB-13 |
//! | `report` | `report.rs` | FIB-14 |
//!
//! `authorities.rs`, `reference.rs` and `files.rs` are FIB-8's and `wire.rs`
//! FIB-10's. A sub-verb that is not built refuses by name and exits FAILED: a
//! refusal, never a silent no-op (`CLAUDE.md` §4). An unknown sub-verb is a
//! misuse.
//!
//! # Where a FIB run writes (D-6306)
//!
//! Only `--out-store` writes the store, under `<store>/results/fib-v1/<run-id>/`
//! ([`results_dir`]) and through `store_write::root_for`, so an unstamped build
//! is refused before it opens anything. Every other form writes only to an
//! `--out` directory OUTSIDE the store root; an `--out` inside it is refused by
//! name ([`refuse_out_inside_store`]).
//!
//! # One direction per search (FM-F007)
//!
//! `fib run` requires `--direction long` or `--direction short`: every mix is
//! one direction (operator 16 Oct 16:04 IST), and the run identity's direction
//! is never Undirected.
//!
//! Equities are labelled research, gross of every charge until FIB-11 wires
//! the charges, and never Selection V6 or execution authority (`CLAUDE.md` §1).

mod authorities;
mod day;
mod deep;
mod exam;
mod files;
mod net;
mod plan;
mod reference;
mod report;
mod run;
mod sample;
mod verify;
mod wf;
mod wire;

use std::path::{Path, PathBuf};

use vocab::fib_cube::Search;

/// Every sub-verb, with the package that builds it.
pub(crate) const SUBVERBS: &[(&str, &str)] = &[
    ("day-check", "FIB-8"),
    ("run", "FIB-9"),
    ("deep", "FIB-9"),
    ("sample", "FIB-9"),
    ("verify", "FIB-9"),
    ("net", "FIB-11"),
    ("plan", "FIB-12"),
    ("wf", "FIB-13"),
    ("exam", "FIB-13"),
    ("report", "FIB-14"),
];

/// The store folder every FIB result lives under, below `results/`.
pub(crate) const RESULTS_FOLDER: &str = "fib-v1";

/// `cli fib SUBVERB ...`: one sub-verb, by its first word.
pub(crate) fn command(arguments: &[&str], out: &mut String) -> u8 {
    match arguments {
        ["day-check", rest @ ..] => day::command(rest, out),
        ["run", rest @ ..] => match direction(rest) {
            Ok(direction) => run::command(direction, rest, out),
            Err(why) => crate::refuse(out, &why),
        },
        ["deep", rest @ ..] => deep::command(rest, out),
        ["sample", rest @ ..] => sample::command(rest, out),
        ["verify", rest @ ..] => verify::command(rest, out),
        ["net", rest @ ..] => net::command(rest, out),
        ["plan", rest @ ..] => plan::command(rest, out),
        ["wf", rest @ ..] => wf::command(rest, out),
        ["exam", rest @ ..] => exam::command(rest, out),
        ["report", rest @ ..] => report::command(rest, out),
        [word, ..] => crate::refuse(out, &unknown(word)),
        [] => crate::refuse(out, "`fib` needs a sub-verb"),
    }
}

/// The refusal of a word that is no sub-verb, naming every one.
fn unknown(word: &str) -> String {
    let verbs: Vec<&str> = SUBVERBS.iter().map(|(verb, _)| *verb).collect();
    format!(
        "`fib {word}` is not a fib sub-verb; the sub-verbs are {}",
        verbs.join(", ")
    )
}

/// The refusal of a sub-verb whose package has not landed: by name, exit
/// FAILED, nothing written.
fn not_built(verb: &str, owner: &str, out: &mut String) -> u8 {
    out.push_str("refused: fib ");
    out.push_str(verb);
    out.push_str(" is not built yet: it arrives with ");
    out.push_str(owner);
    out.push('\n');
    crate::FAILED
}

/// The direction a `fib run` names: `--direction long` or `--direction short`
/// (FM-F007).
///
/// # Errors
///
/// `direction_required`, naming the two spellings, when the flag is missing,
/// has no value, or has any other value.
fn direction(arguments: &[&str]) -> Result<Search, String> {
    let named = arguments
        .iter()
        .position(|word| *word == "--direction")
        .and_then(|at| arguments.get(at + 1));
    match named {
        Some(&"long") => Ok(Search::Long),
        Some(&"short") => Ok(Search::Short),
        _ => Err("direction_required: `fib run` needs `--direction long` or \
                  `--direction short`; every search is one direction and no run is \
                  Undirected (operator 16 Oct 16:04 IST, FM-F007)"
            .to_owned()),
    }
}

/// The folder a stamped `fib ... --out-store` writes one run's results to:
/// `<store>/results/fib-v1/<run-id>/`.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "FIB-9's run writes through it; until then only the tests call it"
    )
)]
pub(crate) fn results_dir(store_root: &Path, run_id_hex: &str) -> PathBuf {
    store_root
        .join("results")
        .join(RESULTS_FOLDER)
        .join(run_id_hex)
}

/// Refuses an `--out` directory inside the store root: only `--out-store`
/// writes the store. Both paths are made absolute against the working
/// directory and compared by component, without touching the disk.
///
/// # Errors
///
/// `out_inside_store`, naming both paths, when `out` is the store root or
/// below it; `out_unresolved` when either path cannot be made absolute.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "FIB-9's run checks its --out through it; until then only the tests call it"
    )
)]
pub(crate) fn refuse_out_inside_store(out: &Path, store_root: &Path) -> Result<(), String> {
    let lexical = |path: &Path| -> Result<PathBuf, String> {
        let absolute = std::path::absolute(path)
            .map_err(|why| format!("out_unresolved: {}: {why}", path.display()))?;
        // An absolute path's `components()` already drops every `.`, so only
        // `..` is left to fold.
        let mut clean = PathBuf::new();
        for part in absolute.components() {
            if part == std::path::Component::ParentDir {
                clean.pop();
            } else {
                clean.push(part);
            }
        }
        Ok(clean)
    };
    let (out_path, store) = (lexical(out)?, lexical(store_root)?);
    if out_path.starts_with(&store) {
        return Err(format!(
            "out_inside_store: --out {} is inside the store root {}; only \
             --out-store writes the store (D-6306)",
            out.display(),
            store_root.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use crate::{FAILED, MISUSED};

    fn routed(words: &[&str]) -> (u8, String) {
        let mut out = String::new();
        let code = command(words, &mut out);
        (code, out)
    }

    /// EVERY SUB-VERB REFUSES BY NAME, EXITS FAILED AND NAMES ITS PACKAGE.
    #[test]
    fn each_fib_sub_verb_refuses_by_name() {
        for (verb, owner) in SUBVERBS {
            let words: Vec<&str> = if *verb == "run" {
                vec![verb, "--direction", "long", "--out", "/tmp/x"]
            } else {
                vec![verb, "anything"]
            };
            let (code, out) = routed(&words);
            assert_eq!(code, FAILED, "{verb}");
            assert_eq!(
                out,
                format!("refused: fib {verb} is not built yet: it arrives with {owner}\n")
            );
        }
        let (code, out) = routed(&["run", "--direction", "short"]);
        assert_eq!(code, FAILED);
        assert!(out.contains("fib run is not built yet: it arrives with FIB-9"));
        for (verb, _) in SUBVERBS {
            let (code, _) = routed(&[verb]);
            assert_eq!(
                code,
                if *verb == "run" { MISUSED } else { FAILED },
                "{verb} alone"
            );
        }
    }

    #[test]
    fn an_unknown_sub_verb_is_a_misuse_naming_every_sub_verb() {
        let (code, out) = routed(&["sweep"]);
        assert_eq!(code, MISUSED);
        assert!(
            out.starts_with("refused: `fib sweep` is not a fib sub-verb"),
            "{out}"
        );
        for (verb, _) in SUBVERBS {
            assert!(out.contains(verb), "{verb}");
        }
        assert!(out.contains(crate::USAGE), "a misuse prints the usage");
        let (code, out) = routed(&[]);
        assert_eq!(code, MISUSED);
        assert!(out.contains("`fib` needs a sub-verb"));
    }

    /// FM-F007: a run without a direction is refused by name.
    #[test]
    fn fib_run_without_direction_is_refused_by_name() {
        for words in [
            &["run"][..],
            &["run", "--out-store"][..],
            &["run", "--direction"][..],
            &["run", "--direction", "both"][..],
            &["run", "--direction", "undirected"][..],
            &["run", "long"][..],
        ] {
            let (code, out) = routed(words);
            assert_eq!(code, MISUSED, "{words:?}");
            assert!(out.starts_with("refused: direction_required:"), "{out}");
        }
        assert_eq!(direction(&["--direction", "long"]), Ok(Search::Long));
        assert_eq!(
            direction(&["x", "--direction", "short", "y"]),
            Ok(Search::Short)
        );
    }

    #[test]
    fn the_results_folder_is_under_the_store_s_results() {
        assert_eq!(
            results_dir(Path::new("/s"), "ab12"),
            PathBuf::from("/s/results/fib-v1/ab12")
        );
    }

    /// AN `--out` INSIDE THE STORE ROOT IS REFUSED BY NAME; one outside is not.
    #[test]
    fn an_out_inside_the_store_is_refused_and_one_outside_is_not() {
        let store = Path::new("/data/store");
        for inside in [
            "/data/store",
            "/data/store/results/fib-v1/x",
            "/data/store/./bars",
            "/data/other/../store/x",
        ] {
            let refused = refuse_out_inside_store(Path::new(inside), store).unwrap_err();
            assert!(
                refused.starts_with("out_inside_store: "),
                "{inside}: {refused}"
            );
        }
        for outside in ["/data/store-scratch", "/data/scratch", "/tmp/fib", "/data"] {
            assert_eq!(
                refuse_out_inside_store(Path::new(outside), store),
                Ok(()),
                "{outside}"
            );
        }
        let relative = refuse_out_inside_store(Path::new("x/y"), Path::new("x"));
        assert!(relative.unwrap_err().starts_with("out_inside_store"));
        let dotted = refuse_out_inside_store(Path::new("./x/./y"), Path::new("x/."));
        assert!(dotted.unwrap_err().starts_with("out_inside_store"));
        let empty = refuse_out_inside_store(Path::new(""), store);
        assert!(empty.unwrap_err().starts_with("out_unresolved: "));
    }

    /// `fib` is a command in the list, the usage, the dispatch and the sweep
    /// set; `cli fib` alone is a misuse naming it as a command; it writes the
    /// store only with `--out-store`.
    #[test]
    fn fib_is_listed_dispatched_and_writes_the_store_only_with_out_store() {
        assert!(crate::COMMANDS.contains(&"fib"));
        assert!(crate::USAGE.contains("cli fib SUBVERB"));
        assert!(crate::is_sweep_command("fib"));
        let mut out = String::new();
        let code = crate::run(&["fib".to_owned()], &mut out);
        assert_eq!(code, MISUSED);
        assert!(
            out.contains("`fib` is a command, but not with 0 arguments"),
            "{out}"
        );
        let mut out = String::new();
        let code = crate::run(&["fib".to_owned(), "plan".to_owned()], &mut out);
        assert_eq!(code, FAILED);
        assert!(out.contains("fib plan is not built yet: it arrives with FIB-12"));
        assert!(crate::store_write::writes_the_store(
            "fib",
            &["run", "--out-store"]
        ));
        assert!(!crate::store_write::writes_the_store(
            "fib",
            &["run", "--out", "/scratch"]
        ));
    }

    /// Until FIB-10 every new-work authority word is refused by name, and a
    /// word that is none of them is refused too.
    #[test]
    fn every_new_work_authority_word_is_refused_until_built() {
        assert_eq!(wire::WORDS, ["cal1", "pit", "data1", "data2", "of1"]);
        // The refusal's membership test is a match over the same five words.
        for word in wire::WORDS {
            assert!(wire::register(word).unwrap_err().contains("not built yet"));
        }
        for word in wire::WORDS {
            assert_eq!(
                wire::register(word),
                Err(format!(
                    "authority `{word}` is not built yet: it arrives with FIB-10"
                ))
            );
        }
        let other = wire::register("interim").unwrap_err();
        assert!(other.starts_with("`interim` is not a new-work authority word"));
    }
}
