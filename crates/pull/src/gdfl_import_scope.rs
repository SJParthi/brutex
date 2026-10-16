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
//! # What a run tells of it (repair 1, 12 Oct 2026)
//!
//! - A member key's close is the MEMBER's event: "member imported", "member
//!   incomplete" or "member refused", naming `member=` and its journal
//!   `key=`. It is never "day imported": that message is the day's, told once
//!   after the day's last member key, and the import's pace is measured from
//!   the gaps between its stamps.
//! - The done days of a run's first month that lie BEFORE its range and owe
//!   its scope a member are read first, in order, under a request of their
//!   own, and the run says so at `warn` (`scope_reach_back`). So no member's
//!   month is made from a later day while an earlier day of it is owed: on
//!   10 Oct 2026 that made six series from special sessions alone, and every
//!   later landing in such a month is a new full copy of it. At most 30
//!   days; a day no run closed owes nothing and is not read.
//! - `retry_refused` reads again a member key closed done with refusals, as
//!   it does a base key.
//! - The run's `run` line is in the journal before an `examined` line names
//!   the run's id.
//! - A torn line of this ledger is said at `warn` (`scope_ledger_torn`).
//!
//! # Whose close a run asks (repair 2, 12 Oct 2026)
//!
//! The reach-back above asked only the run's own key, so a run under a
//! filter (`cli`'s `--stored`, one named symbol) met days closed `done`
//! under `*`, found its own key closed nowhere, and made each new member's
//! month from its one later day, unsaid. A day before the range that the
//! run's own key did not close is now asked of EVERY key that closed it
//! `done`. Such a key owes the members its close's scope never covered that
//! its own filter names; the run reads the ones its own filter wants, under
//! that key's member keys, so the key's own later run finds them done. The
//! examination is remembered here under `<that filter>/<the run's filter>`
//! in the `<filter>` place of an `examined` line (`/` is in no symbol), and
//! never under the key itself: its own run still reads the members this
//! filter did not want. A member two keys owe is read once and the day is
//! told once.
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

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "a test that cannot panic cannot fail")]
mod cloud_mutant_tests {
    //! Mutant kills for the scope ledger (cloud mutation round, 2026-10-16).
    //! Every member name is invented.

    use super::{Ledger, Record, scopes_path};
    use crate::gdfl_fixtures::scratch;

    /// The two swept index members, the digest `printf 'NSE-INDEX-BANKNIFTY\n
    /// NSE-INDEX-NIFTY\n' | sha256sum` begins with.
    const SWEPT_DIGEST: &str = "0d28ae1eaf0dd31f";

    fn swept() -> Record {
        Record::of(vec![
            "NSE-INDEX-NIFTY".to_owned(),
            "NSE-INDEX-BANKNIFTY".to_owned(),
            "NSE-INDEX-NIFTY".to_owned(),
        ])
    }

    /// The digest is the FIRST 64 bits of the SHA-256 of the sorted,
    /// deduplicated list, as sixteen hex digits: exactly what a close's
    /// `scope=` carries. Kills `DIGEST_HEX / 2` read as `% 2` (an empty
    /// digest) or `* 2` (the whole 64-digit hash).
    #[test]
    fn a_digest_is_sixteen_hex_digits_of_the_sorted_list() {
        let record = swept();
        assert_eq!(record.digest(), SWEPT_DIGEST);
        assert_eq!(
            record.members(),
            ["NSE-INDEX-BANKNIFTY", "NSE-INDEX-NIFTY"],
            "sorted and without repeats"
        );
        assert_eq!(record.set().len(), 2);
        assert!(record.set().contains("NSE-INDEX-NIFTY"));
        // The empty list is a scope too: SHA-256 of nothing.
        let none = Record::of(Vec::new());
        assert_eq!(none.digest(), "e3b0c44298fc1c14");
        assert!(none.members().is_empty() && none.set().is_empty());
        // A name holding `&` (an `M&M`-shaped member) hashes as its bytes.
        assert_eq!(
            Record::of(vec!["B&C".to_owned(), "A".to_owned()]).digest(),
            "08f0f05e70c80f78"
        );
    }

    /// The ledger is `<store>/imports/gdfl.scopes`, beside the journal.
    #[test]
    fn the_ledger_lies_beside_the_import_journal() {
        let root = std::path::Path::new("/store-root");
        assert_eq!(
            scopes_path(root),
            std::path::PathBuf::from("/store-root/imports/gdfl.scopes")
        );
    }

    /// A ledger that cannot be READ (a directory at its path) refuses by
    /// name with the path; only a missing ledger is empty. Kills the
    /// `NotFound` guard read as `true`.
    #[test]
    fn a_ledger_that_cannot_be_read_refuses_and_only_a_missing_one_is_empty() {
        let root = scratch("cloud-scope-unreadable");
        let path = scopes_path(&root);
        let missing = Ledger::load(path.clone()).unwrap();
        assert_eq!(missing.torn(), 0);
        assert!(missing.members_of(SWEPT_DIGEST).is_none());
        std::fs::create_dir_all(&path).unwrap();
        let refused = Ledger::load(path.clone()).unwrap_err();
        assert!(refused.contains("gdfl.scopes"), "{refused}");
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A `scope` line is read only when its count AND its digest are its
    /// list's: a wrong count with the right digest, the right count with a
    /// wrong digest, and a count that is not a number each refuse the load.
    /// Kills `||` read as `&&` and either `!=` read as `==`.
    #[test]
    fn a_scope_line_whose_count_or_digest_is_not_its_lists_refuses() {
        let root = scratch("cloud-scope-count");
        let path = scopes_path(&root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let list = "NSE-INDEX-BANKNIFTY,NSE-INDEX-NIFTY";
        let good = format!("scope {SWEPT_DIGEST} indices n=2 {list} .\n");
        std::fs::write(&path, &good).unwrap();
        let loaded = Ledger::load(path.clone()).unwrap();
        let held = loaded.members_of(SWEPT_DIGEST).unwrap();
        assert_eq!(held.len(), 2);
        assert!(held.contains("NSE-INDEX-BANKNIFTY") && held.contains("NSE-INDEX-NIFTY"));
        for bad in [
            format!("scope {SWEPT_DIGEST} indices n=3 {list} ."),
            format!("scope {SWEPT_DIGEST} indices n=1 {list} ."),
            format!("scope 0000000000000000 indices n=2 {list} ."),
            format!("scope {SWEPT_DIGEST} indices n=two {list} ."),
            format!("scope {SWEPT_DIGEST} indices 2 {list} ."),
        ] {
            std::fs::write(&path, format!("{bad}\n")).unwrap();
            let why = Ledger::load(path.clone()).unwrap_err();
            assert!(why.contains("not a scope ledger line"), "{bad}: {why}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// An `examined` line is read only with numeric `run=` and `opened=` and
    /// a `scope=`; any other line, or a line of another shape, refuses.
    #[test]
    fn an_examined_line_is_read_whole_or_refuses() {
        let root = scratch("cloud-scope-examined");
        let path = scopes_path(&root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let good = "examined indices 2031-03-03 * scope=abc run=7 opened=2 .\n";
        std::fs::write(&path, good).unwrap();
        let loaded = Ledger::load(path.clone()).unwrap();
        assert_eq!(loaded.examined_under("indices 2031-03-03 *"), Some("abc"));
        assert_eq!(loaded.examined_under("indices 2031-03-04 *"), None);
        for bad in [
            "examined indices 2031-03-03 * scope=abc run=x opened=2 .",
            "examined indices 2031-03-03 * scope=abc run=7 opened=-1 .",
            "examined indices 2031-03-03 * abc run=7 opened=2 .",
            "examined indices 2031-03-03 * scope=abc 7 opened=2 .",
            "examined indices 2031-03-03 scope=abc run=7 opened=2 .",
            "something else .",
        ] {
            std::fs::write(&path, format!("{bad}\n")).unwrap();
            assert!(Ledger::load(path.clone()).is_err(), "{bad}");
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    /// Every fragment without the ` .` end is stepped over AND counted, one
    /// each, wherever it lies; an empty line is not a fragment. Kills the
    /// count's `+=` read as `-=` or `*=` and the empty-line filter's `!`
    /// deleted.
    #[test]
    fn every_torn_fragment_is_counted_once_and_whole_lines_still_read() {
        let root = scratch("cloud-scope-torn");
        let path = scopes_path(&root);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let text = format!(
            "scope {SWEPT_DIGEST} ind\n\nexamined indices 2031-03-03 * scope={SWEPT_DIGEST} run=1 opened=0 .\n\
             scope {SWEPT_DIGEST} indices n=2 NSE-INDEX-BANKNIFTY,NSE-INDEX-NIFTY .\nexamined ind"
        );
        std::fs::write(&path, text).unwrap();
        let loaded = Ledger::load(path.clone()).unwrap();
        assert_eq!(loaded.torn(), 2, "two fragments, one empty line");
        assert!(loaded.members_of(SWEPT_DIGEST).is_some());
        assert_eq!(
            loaded.examined_under("indices 2031-03-03 *"),
            Some(SWEPT_DIGEST)
        );
        // Zero-length and one-fragment files.
        std::fs::write(&path, b"").unwrap();
        assert_eq!(Ledger::load(path.clone()).unwrap().torn(), 0);
        std::fs::write(&path, b"scope").unwrap();
        assert_eq!(Ledger::load(path.clone()).unwrap().torn(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// `record` writes a digest's list once and holds it at once; `examine`
    /// writes its line and holds it at once; a reload reads both back, in
    /// the shape this module writes. Kills `record` and `examine` read as
    /// `Ok(())`, and `members`/`set` answering anything but the record's.
    #[test]
    fn record_and_examine_write_their_lines_once_and_a_reload_reads_them() {
        let root = scratch("cloud-scope-write");
        let path = scopes_path(&root);
        let mut ledger = Ledger::load(path.clone()).unwrap();
        let record = swept();
        ledger.record("indices", &record).unwrap();
        ledger.record("indices", &record).unwrap();
        assert_eq!(
            ledger
                .members_of(SWEPT_DIGEST)
                .map(|set| set.contains("NSE-INDEX-NIFTY")),
            Some(true)
        );
        ledger
            .examine("indices 2031-03-03 *", SWEPT_DIGEST, (9, 3))
            .unwrap();
        assert_eq!(
            ledger.examined_under("indices 2031-03-03 *"),
            Some(SWEPT_DIGEST)
        );
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            text,
            format!(
                "scope {SWEPT_DIGEST} indices n=2 NSE-INDEX-BANKNIFTY,NSE-INDEX-NIFTY .\n\
                 examined indices 2031-03-03 * scope={SWEPT_DIGEST} run=9 opened=3 .\n"
            ),
            "one scope line however often it is recorded"
        );
        let reread = Ledger::load(path).unwrap();
        let held = reread.members_of(SWEPT_DIGEST).unwrap();
        assert_eq!(held.len(), 2);
        assert_eq!(
            reread.examined_under("indices 2031-03-03 *"),
            Some(SWEPT_DIGEST)
        );
        assert_eq!(reread.torn(), 0);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A ledger that cannot be appended to (its directory is a file) refuses
    /// `record` and `examine` by name and holds nothing it did not write.
    #[test]
    fn a_ledger_that_cannot_be_appended_to_refuses_and_holds_nothing_new() {
        let root = scratch("cloud-scope-blocked");
        let mut ledger = Ledger::load(scopes_path(&root)).unwrap();
        std::fs::write(root.join("imports"), b"not a directory").unwrap();
        assert!(ledger.record("indices", &swept()).is_err());
        assert!(ledger.members_of(SWEPT_DIGEST).is_none());
        assert!(ledger.examine("k", SWEPT_DIGEST, (1, 0)).is_err());
        assert_eq!(ledger.examined_under("k"), None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
