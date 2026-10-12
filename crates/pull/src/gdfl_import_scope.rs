//! WHICH MEMBERS A DAY'S CLOSE COVERED: the scope ledger (WP-12f, D-4878;
//! DB-B17, DB-B40, DB-B61, DB-B76).
//!
//! The import journal's key is `<kind> <day> <filter>`, and a `done` key was
//! skipped whatever the run's scope had become. On 10 Oct 2026 a run under a
//! scope of eight indices met 1,985 days closed `done` under a scope of two,
//! skipped every one, and filed six new index series from the 12 special
//! sessions alone. A day is done only for the members its close was asked to
//! read, so each close now says which those were:
//!
//! - a capital-market run's `done` and `incomplete` lines carry
//!   `scope=<digest>`, the digest of the run's member list
//!   ([`crate::gdfl_cm::CmScope::members`]);
//! - this ledger, `<store>/imports/gdfl.scopes`, holds each digest's member
//!   list once (`scope <digest> <kind> n=<count> <m1>,<m2>,... .`), and for a
//!   day read again under a wider scope the scope it was last examined under
//!   (`examined <kind> <day> <filter> scope=<digest> run=<id> opened=<n> .`),
//!   so a day whose listing held no file of any new member is not listed
//!   again by the next run.
//!
//! A close that carries no `scope=` was written before scopes were recorded.
//! It is read as built under the swept surface of its kind, the scope every
//! build that ever closed a day filed at least: the members beyond it are
//! owed. Reading again a member an unrecorded wider run did file costs the
//! read and changes nothing, because a bar already held is confirmed and
//! never written twice (`ingest`'s `AlreadyPresent`).
//!
//! # Lines and crashes
//!
//! Append-only (`CLAUDE.md` §1). Every line ends ` .`: a line without that
//! end is the fragment a killed write left, stepped over and counted
//! ([`Ledger::torn`]), never read as a record; a whole line that is not one
//! of the two above refuses the load by name, so a foreign file is never
//! appended to.
//!
//! # Cost
//!
//! Loaded once per run, O(its bytes). A digest's member list and a day's
//! examined scope are each one hash probe. A member list is written once per
//! distinct digest, an `examined` line once per day read again.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::Digest as _;

/// The scope ledger's path under a store root, beside the import journal.
#[must_use]
pub fn scopes_path(store_root: &Path) -> PathBuf {
    store_root.join("imports").join("gdfl.scopes")
}

/// How many hexadecimal digits of the member list's SHA-256 name a scope:
/// 64 bits, far past the handful of scopes one store ever sees.
const DIGEST_HEX: usize = 16;

/// The end of every whole line.
const END: &str = " .";

/// One run's scope as it is recorded: its members, sorted and without
/// repeats, and their digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    digest: String,
    members: Vec<String>,
    set: Arc<HashSet<String>>,
}

impl Record {
    /// The record of `members`, put in order and deduplicated, so the digest
    /// does not depend on how they were listed. O(n log n), once per run.
    #[must_use]
    pub fn of(members: Vec<String>) -> Self {
        let ordered: std::collections::BTreeSet<String> = members.into_iter().collect();
        let members: Vec<String> = ordered.into_iter().collect();
        let mut hasher = sha2::Sha256::new();
        for member in &members {
            hasher.update(member.as_bytes());
            hasher.update(b"\n");
        }
        let mut digest = String::with_capacity(DIGEST_HEX);
        for byte in hasher.finalize().iter().take(DIGEST_HEX / 2) {
            let _infallible = write!(digest, "{byte:02x}");
        }
        let set = Arc::new(members.iter().cloned().collect());
        Self {
            digest,
            members,
            set,
        }
    }

    /// The scope's digest, as a close's `scope=` carries it.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// The members, sorted.
    #[must_use]
    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// The members as a set.
    #[must_use]
    pub fn set(&self) -> &Arc<HashSet<String>> {
        &self.set
    }
}

/// The scope ledger as one run reads and appends it.
#[derive(Debug, Default)]
pub struct Ledger {
    path: PathBuf,
    /// Each recorded digest's members.
    scopes: HashMap<String, Arc<HashSet<String>>>,
    /// The scope each `<kind> <day> <filter>` was last examined under.
    examined: HashMap<String, String>,
    /// How many torn fragments the load stepped over.
    torn: usize,
}

impl Ledger {
    /// Reads the ledger at `path`; a missing one is empty.
    ///
    /// # Errors
    ///
    /// The file cannot be read, or holds a whole line that is not one this
    /// module writes: named, with the path.
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => return Err(format!("{}: {why}", path.display())),
        };
        let mut ledger = Self {
            path,
            ..Self::default()
        };
        for line in text.split('\n').filter(|line| !line.is_empty()) {
            let Some(whole) = line.strip_suffix(END) else {
                ledger.torn += 1;
                continue;
            };
            let fields: Vec<&str> = whole.split(' ').collect();
            ledger.read_line(&fields).ok_or_else(|| {
                format!(
                    "{}: not a scope ledger line: {line:?}",
                    ledger.path.display()
                )
            })?;
        }
        Ok(ledger)
    }

    /// One whole line; `None` for a line this module does not write.
    fn read_line(&mut self, fields: &[&str]) -> Option<()> {
        match fields {
            ["scope", digest, _kind, count, members] => {
                let count: usize = count.strip_prefix("n=")?.parse().ok()?;
                let set: HashSet<String> = members
                    .split(',')
                    .filter(|member| !member.is_empty())
                    .map(str::to_owned)
                    .collect();
                // The list must be the one its digest names: a line edited
                // by hand, or two lists under one digest, is refused.
                let listed: Vec<String> = set.iter().cloned().collect();
                if set.len() != count || Record::of(listed).digest != *digest {
                    return None;
                }
                self.scopes.insert((*digest).to_owned(), Arc::new(set));
            }
            ["examined", kind, day, filter, scope, run, opened] => {
                let digest = scope.strip_prefix("scope=")?;
                let _run: u64 = run.strip_prefix("run=")?.parse().ok()?;
                let _opened: usize = opened.strip_prefix("opened=")?.parse().ok()?;
                self.examined
                    .insert(format!("{kind} {day} {filter}"), digest.to_owned());
            }
            _ => return None,
        }
        Some(())
    }

    /// How many torn fragments the load stepped over: each a line a killed
    /// write left, which the next run writes again whole.
    #[must_use]
    pub const fn torn(&self) -> usize {
        self.torn
    }

    /// The members `digest` names, when the ledger holds its list.
    #[must_use]
    pub fn members_of(&self, digest: &str) -> Option<&Arc<HashSet<String>>> {
        self.scopes.get(digest)
    }

    /// The scope `key` (`<kind> <day> <filter>`) was last examined under.
    #[must_use]
    pub fn examined_under(&self, key: &str) -> Option<&str> {
        self.examined.get(key).map(String::as_str)
    }

    /// Records `record`'s member list for `kind`, once: a digest the ledger
    /// already holds writes nothing.
    ///
    /// # Errors
    ///
    /// The ledger cannot be appended to, named.
    pub fn record(&mut self, kind: &str, record: &Record) -> Result<(), String> {
        if self.scopes.contains_key(record.digest()) {
            return Ok(());
        }
        let line = format!(
            "scope {} {kind} n={} {}{END}",
            record.digest(),
            record.members().len(),
            record.members().join(","),
        );
        super::append_lines(&self.path, &[line])?;
        self.scopes
            .insert(record.digest().to_owned(), Arc::clone(record.set()));
        Ok(())
    }

    /// Records that `key`'s day was examined under `digest` by run `run`,
    /// which opened `opened` member keys on it.
    ///
    /// # Errors
    ///
    /// The ledger cannot be appended to, named.
    pub fn examine(
        &mut self,
        key: &str,
        digest: &str,
        (run, opened): (u64, usize),
    ) -> Result<(), String> {
        let line = format!("examined {key} scope={digest} run={run} opened={opened}{END}");
        super::append_lines(&self.path, &[line])?;
        self.examined.insert(key.to_owned(), digest.to_owned());
        Ok(())
    }
}
