//! The run directories: the one table of where a run process logs, beneath
//! `<store>/logs` (D-4897, which continues D-4862).
//!
//! # Why it lives here
//!
//! A process holds one sink ([`crate::install`] refuses a second), so a log
//! directory belongs to the one lock that makes its writer alone there. The
//! writers choose their directory in three crates (`cli` for a verb, `api`
//! for the child it starts, `pull` for a verification run) and the readers
//! (`/logs`, `/imports.json`, `/dayflags.json`'s day band, `/store`) are in a
//! fourth. Before this table each of them spelt the directories itself: four
//! hand-kept copies of one list, and a fifth the day a mode was added.
//! `telemetry` is the one crate every one of them already names, and it
//! depends on nothing, so the table is here: a writer asks [`for_gdfl`],
//! [`VERIFY_ZERODHA`] or [`VERIFY_MERGE`] where to log, and a reader reads
//! [`ALL`], and neither can name a directory the other does not know.
//!
//! # What it does not do
//!
//! Nothing here opens, creates or locks anything: these are names. The lock
//! paths are the ones the ingest automation design gives (its §4, "the
//! locks"); the locks themselves are taken by the packages that start runs
//! (IA-1 and IA-9), and at the time this table landed none of them was built.
//! Until IA-1's directory switch every run still logs to [`CLI`].
//!
//! The three kinds are `pull::gdfl_import::ImportKind::ALL`'s words. This
//! crate may not depend on `pull` (it depends on nothing), so the tie is a
//! test where both are in scope: `api::logs`'s
//! `the_run_directories_name_every_import_kind`.

use std::path::{Path, PathBuf};

/// The directory, beneath a store root, that every log directory is under.
pub const LOGS: &str = "logs";

/// The directory, beneath [`LOGS`], that every `cli` verb logs to, and every
/// run until the directory switch. Not one of [`ALL`]: no lock makes a
/// process alone there, which is why runs move out of it.
pub const CLI: &str = "cli";

/// One directory a run process logs to, and the lock that makes that process
/// the only one there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunDir {
    /// The directory beneath `<store>/logs`, `/`-separated.
    pub sub: &'static str,
    /// The lock file beneath `<store>` whose exclusive holder is the one
    /// process that logs here.
    pub lock: &'static str,
    /// For a GDFL kind's own directory, the kind's word; `None` for the
    /// others.
    pub gdfl_kind: Option<&'static str>,
}

impl RunDir {
    /// Where the directory is beneath `store_root`.
    #[must_use]
    pub fn beneath(&self, store_root: &Path) -> PathBuf {
        store_root.join(LOGS).join(self.sub)
    }

    /// The directory as an answer names it: `logs/<sub>`.
    #[must_use]
    pub fn name(&self) -> String {
        format!("{LOGS}/{}", self.sub)
    }
}

/// The census layout a GDFL run found, as far as it decides the run's lock:
/// on v3 one store lock serialises every run, on v4 each kind has its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GdflLayout {
    /// One census for the store: every GDFL run holds the store lock.
    V3,
    /// One census partition per kind: a run holds its kind's lock.
    V4,
}

/// Where the runs that hold the GDFL store lock log: `cli census-migrate
/// gdfl`, `cli census-recount gdfl`, and on v3 every GDFL run.
pub const GDFL_STORE: RunDir = RunDir {
    sub: "gdfl/store",
    lock: "imports/gdfl.lock",
    gdfl_kind: None,
};

/// Where a v4 run of each GDFL kind logs, in import order.
pub const GDFL_KINDS: [RunDir; 3] = [
    RunDir {
        sub: "gdfl/indices",
        lock: "imports/gdfl.indices.lock",
        gdfl_kind: Some("indices"),
    },
    RunDir {
        sub: "gdfl/stocks",
        lock: "imports/gdfl.stocks.lock",
        gdfl_kind: Some("stocks"),
    },
    RunDir {
        sub: "gdfl/options",
        lock: "imports/gdfl.options.lock",
        gdfl_kind: Some("options"),
    },
];

/// Where a Zerodha cold-verification run logs.
pub const VERIFY_ZERODHA: RunDir = RunDir {
    sub: "verify/zerodha",
    lock: "verify/zerodha.lock",
    gdfl_kind: None,
};

/// Where every `verify-merge` process logs.
pub const VERIFY_MERGE: RunDir = RunDir {
    sub: "verify/merge",
    lock: "verify/merge.lock",
    gdfl_kind: None,
};

/// Every run directory: the closed set. With `<store>/logs` itself (the
/// server's own) and [`CLI`] these are the 8 directories a reader reads. A
/// new kind of run is a row here, with its lock, and
/// `docs/06-limits.md` L-IA-13's bound recomputed.
pub const ALL: [RunDir; 6] = [
    GDFL_KINDS[0],
    GDFL_KINDS[1],
    GDFL_KINDS[2],
    GDFL_STORE,
    VERIFY_ZERODHA,
    VERIFY_MERGE,
];

/// The directory a GDFL run of `kind` logs to on `layout`: the store lock's
/// on v3, the kind's own on v4. `None` for a word that is not a kind of this
/// table, on either layout: a run that cannot name its directory is refused
/// by its caller, never logged somewhere a reader does not look.
#[must_use]
pub fn for_gdfl(kind: &str, layout: GdflLayout) -> Option<RunDir> {
    let own = GDFL_KINDS
        .iter()
        .copied()
        .find(|dir| dir.gdfl_kind == Some(kind))?;
    Some(match layout {
        GdflLayout::V3 => GDFL_STORE,
        GdflLayout::V4 => own,
    })
}

/// The directories a GDFL run of `kind` can have logged to, in the order a
/// reader reads them: [`CLI`] (every run before the directory switch), the
/// kind's own (v4) and the store lock's (v3, and the census verbs). Each as
/// its path beneath [`LOGS`]. A word that is not a kind has no directory of
/// its own and gets the other two.
pub fn gdfl_evidence(kind: &str) -> impl Iterator<Item = &'static str> + '_ {
    gdfl_evidence_all().filter(move |sub| {
        GDFL_KINDS
            .iter()
            .find(|dir| dir.sub == *sub)
            .is_none_or(|dir| dir.gdfl_kind == Some(kind))
    })
}

/// Every directory any GDFL run can have logged to: [`CLI`], each kind's own
/// and the store lock's, each as its path beneath [`LOGS`]. Five.
pub fn gdfl_evidence_all() -> impl Iterator<Item = &'static str> {
    std::iter::once(CLI)
        .chain(GDFL_KINDS.iter().map(|dir| dir.sub))
        .chain(std::iter::once(GDFL_STORE.sub))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    /// The table is a set: no directory twice, no lock twice, and no entry
    /// inside another (a sink's files in a directory that is also another
    /// sink's parent would be read as that one's).
    #[test]
    fn every_run_directory_and_lock_is_named_once() {
        for (n, dir) in ALL.iter().enumerate() {
            assert!(!dir.sub.is_empty() && !dir.lock.is_empty());
            assert_ne!(dir.sub, CLI, "cli is not a run directory");
            for other in ALL.iter().skip(n + 1) {
                assert_ne!(dir.sub, other.sub);
                assert_ne!(dir.lock, other.lock);
                assert!(
                    !other.sub.starts_with(&format!("{}/", dir.sub))
                        && !dir.sub.starts_with(&format!("{}/", other.sub)),
                    "{} and {} nest",
                    dir.sub,
                    other.sub
                );
            }
        }
        assert_eq!(ALL.len(), GDFL_KINDS.len() + 3);
    }

    /// A v3 run of any kind logs under the store lock, a v4 run under its
    /// own, and both answers are rows of the table. A word that is not a
    /// kind has no directory on either layout.
    #[test]
    fn a_gdfl_run_logs_where_its_lock_is() {
        for own in GDFL_KINDS {
            let kind = own.gdfl_kind.expect("a kind's directory names its kind");
            assert_eq!(for_gdfl(kind, GdflLayout::V3), Some(GDFL_STORE));
            assert_eq!(for_gdfl(kind, GdflLayout::V4), Some(own));
            assert!(ALL.contains(&own));
            assert_eq!(own.sub, format!("gdfl/{kind}"));
            assert_eq!(own.lock, format!("imports/gdfl.{kind}.lock"));
        }
        assert!(ALL.contains(&GDFL_STORE));
        for garbage in ["", "store", "Indices", "indices ", "gdfl/indices", "../cli"] {
            assert_eq!(for_gdfl(garbage, GdflLayout::V3), None, "{garbage:?}");
            assert_eq!(for_gdfl(garbage, GdflLayout::V4), None, "{garbage:?}");
        }
    }

    /// A reader of one kind's evidence reads exactly the directories a run
    /// of that kind can have written, and all five when it reads every kind.
    #[test]
    fn a_kinds_evidence_is_in_its_three_directories() {
        assert_eq!(
            gdfl_evidence_all().collect::<Vec<_>>(),
            [
                "cli",
                "gdfl/indices",
                "gdfl/stocks",
                "gdfl/options",
                "gdfl/store"
            ]
        );
        for own in GDFL_KINDS {
            let kind = own.gdfl_kind.expect("a kind");
            let read: Vec<&str> = gdfl_evidence(kind).collect();
            assert_eq!(read, [CLI, own.sub, GDFL_STORE.sub]);
            for layout in [GdflLayout::V3, GdflLayout::V4] {
                let written = for_gdfl(kind, layout).expect("a kind has a directory");
                assert!(read.contains(&written.sub), "{kind} on {layout:?}");
            }
        }
        assert_eq!(
            gdfl_evidence("nonsense").collect::<Vec<_>>(),
            [CLI, GDFL_STORE.sub]
        );
    }

    /// Where a directory is, and how an answer names it.
    #[test]
    fn a_run_directory_is_beneath_the_stores_logs() {
        let root = Path::new("/a/store");
        assert_eq!(
            VERIFY_MERGE.beneath(root),
            Path::new("/a/store/logs/verify/merge")
        );
        assert_eq!(VERIFY_ZERODHA.name(), "logs/verify/zerodha");
    }
}
