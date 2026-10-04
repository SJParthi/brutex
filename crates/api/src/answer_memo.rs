//! Route answers kept for one exact census snapshot and one universe parse.
//!
//! # Why this exists
//!
//! `/instruments.json` and `/calendar.json` rebuilt their whole answer on every
//! request even when nothing they read had moved: the instrument list walked
//! the merged universe and every census entry (W1-api5-3), and the calendar
//! collected and sorted the feed's census entries before its first cached
//! calendar probe (W1-api5-5). Both answers are functions of two things this
//! process already versions:
//!
//! * the census snapshot [`crate::server::census_now`] hands out, an `Arc` that
//!   is replaced, never mutated, whenever any manifest stamp moves; and
//! * the parsed universe, whose [`crate::server::Parsed::generation`] moves on
//!   every accepted reparse.
//!
//! So an answer is kept under the snapshot's identity and the generation, and
//! served again while both are the ones the request just observed.
//!
//! # Invalidation
//!
//! The snapshot is compared by `Weak::ptr_eq`. Holding the `Weak` keeps the
//! allocation's address reserved, so a later snapshot can never be allocated
//! at the same address and pass as the old one. A request that observes a
//! different snapshot or generation drops EVERY kept answer before it builds
//! its own, so no answer outlives the inputs it was built from.
//!
//! Only answers the caller marks as keepable are kept: a refusal is rebuilt
//! on every request, so a repaired file is seen at once (the rule
//! `calendar_of::cached` states for an unopened month).
//!
//! # Cost
//!
//! A hit is one uncontended lock, two comparisons, one hash probe of a key
//! bounded by the caller, and a clone of the answer: O(answer bytes), the
//! response itself. A miss builds OUTSIDE the lock, so one slow build never
//! serialises other keys, and two concurrent misses on one key may both build;
//! the second insert replaces the first with an equal answer. D-2285.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, PoisonError, Weak};

use crate::census::VendorCensus;

/// The input a kept answer was built from, compared for identity.
pub(crate) trait Source {
    /// Whether `self` and `other` name the same input.
    fn same(&self, other: &Self) -> bool;
}

/// A census snapshot, by allocation. See the module doc for why the `Weak`.
impl Source for Weak<Vec<VendorCensus>> {
    fn same(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

/// A file's stamp as [`FileStamp::of`] takes it.
impl Source for FileStamp {
    fn same(&self, other: &Self) -> bool {
        self == other
    }
}

/// What one `stat` says about a file: its identity, its length and both of
/// its clocks. Any write moves the status-change time, and a replacement
/// moves the inode, so a file that reads differently cannot keep this stamp
/// through ordinary filesystem mutation. It is not authentication against an
/// actor able to forge filesystem metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileStamp {
    device: u64,
    inode: u64,
    len: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl FileStamp {
    /// One `stat` of `path`, following links as the read that uses it will.
    ///
    /// # Errors
    /// Whatever the host refuses; a caller then keeps nothing.
    pub(crate) fn of(path: &std::path::Path) -> std::io::Result<Self> {
        use std::os::unix::fs::MetadataExt as _;
        let meta = std::fs::metadata(path)?;
        Ok(Self {
            device: meta.dev(),
            inode: meta.ino(),
            len: meta.len(),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
        })
    }
}

/// The reservation a fresh snapshot's map starts with, below a smaller cap.
const INITIAL_ANSWERS: usize = 16;

/// What the memo is keyed under besides the caller's own key.
struct Held<S, K, V> {
    source: S,
    generation: u64,
    answers: HashMap<K, V>,
}

/// One map of kept answers for one input `S` and one universe parse, holding
/// at most `cap` of them.
pub(crate) struct Memo<S, K, V> {
    held: Mutex<Option<Held<S, K, V>>>,
    cap: usize,
    /// Builds run, for the tests that prove a hit builds nothing.
    #[cfg(test)]
    builds: std::sync::atomic::AtomicUsize,
}

impl<S, K, V> Default for Memo<S, K, V> {
    /// Unbounded by count: for a caller whose keys are bounded by the input
    /// itself (a feed, or a name the census holds).
    fn default() -> Self {
        Self::with_cap(usize::MAX)
    }
}

impl<S, K, V> Memo<S, K, V> {
    /// A memo that holds at most `cap` answers: inserting past it drops every
    /// kept answer first, so a caller whose keys come from request text cannot
    /// grow it without bound.
    pub(crate) const fn with_cap(cap: usize) -> Self {
        Self {
            held: Mutex::new(None),
            cap,
            #[cfg(test)]
            builds: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

impl<S, K, V> std::fmt::Debug for Memo<S, K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memo").finish_non_exhaustive()
    }
}

/// The memo of answers built from one census snapshot.
pub(crate) type CensusMemo<K, V> = Memo<Weak<Vec<VendorCensus>>, K, V>;

impl<K: Eq + Hash, V: Clone> CensusMemo<K, V> {
    /// [`Memo::get`] under the snapshot `source`, by allocation.
    pub(crate) fn of_census(
        &self,
        source: &Arc<Vec<VendorCensus>>,
        generation: u64,
        key: K,
        build: impl FnOnce() -> (V, bool),
    ) -> V {
        self.get(Arc::downgrade(source), generation, key, build)
    }
}

impl<S: Source + Clone, K: Eq + Hash, V: Clone> Memo<S, K, V> {
    /// The answer kept for `key` under `source` and `generation`, or the one
    /// `build` returns.
    ///
    /// `build` answers the value and whether it may be kept. A poisoned lock
    /// is read through: the map holds finished answers only, never a half-built
    /// one, because nothing is inserted until `build` has returned.
    pub(crate) fn get(
        &self,
        observed: S,
        generation: u64,
        key: K,
        build: impl FnOnce() -> (V, bool),
    ) -> V {
        {
            let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
            let current = held
                .as_ref()
                .is_some_and(|held| held.source.same(&observed) && held.generation == generation);
            if !current {
                // EVERY ANSWER OF AN OLDER SNAPSHOT OR PARSE GOES, before the
                // build below, so none can be served for inputs it was not
                // built from.
                *held = Some(Held {
                    source: observed.clone(),
                    generation,
                    // PRE-SIZED: a feed per vendor, a held name per series,
                    // or the cap, whichever the caller bounds it by.
                    answers: HashMap::with_capacity(self.cap.min(INITIAL_ANSWERS)),
                });
            }
            if let Some(answer) = held.as_ref().and_then(|held| held.answers.get(&key)) {
                return answer.clone();
            }
        }
        #[cfg(test)]
        self.builds
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (answer, keep) = build();
        if keep {
            let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
            // KEPT ONLY UNDER THE INPUTS IT WAS BUILT FROM: a newer snapshot
            // installed by another request while this one built is not given an
            // answer from the older one.
            if let Some(held) = held
                .as_mut()
                .filter(|held| held.source.same(&observed) && held.generation == generation)
            {
                if held.answers.len() >= self.cap {
                    held.answers.clear();
                }
                held.answers.insert(key, answer.clone());
            }
        }
        answer
    }

    /// How many builds have run, for the tests that prove a hit builds nothing.
    #[cfg(test)]
    pub(crate) fn builds(&self) -> usize {
        self.builds.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// How many answers are kept now, for the tests that bound the map.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map_or(0, |held| held.answers.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn snapshot() -> Arc<Vec<VendorCensus>> {
        Arc::new(Vec::new())
    }

    /// A kept answer is served without a build while the snapshot and the
    /// generation are the ones it was built under, and either moving drops it.
    #[test]
    fn an_answer_is_rebuilt_exactly_when_its_snapshot_or_generation_moves() {
        let memo: CensusMemo<u8, String> = Memo::default();
        let builds = Cell::new(0_u32);
        let build = |text: &str| {
            builds.set(builds.get() + 1);
            (text.to_owned(), true)
        };
        let first = snapshot();
        assert_eq!(memo.of_census(&first, 0, 1, || build("a")), "a");
        assert_eq!(memo.of_census(&first, 0, 1, || build("b")), "a", "a hit");
        assert_eq!(builds.get(), 1);
        assert_eq!(
            memo.of_census(&first, 0, 2, || build("c")),
            "c",
            "another key"
        );
        assert_eq!(memo.len(), 2);
        assert_eq!(
            memo.of_census(&first, 1, 1, || build("d")),
            "d",
            "a new parse"
        );
        assert_eq!(memo.len(), 1, "the older parse's answers are gone");
        let second = snapshot();
        assert_eq!(
            memo.of_census(&second, 1, 1, || build("e")),
            "e",
            "a new census"
        );
        assert_eq!(builds.get(), 4);
        assert_eq!(memo.of_census(&second, 1, 1, || build("f")), "e");
        assert_eq!(builds.get(), 4);
    }

    /// The `Weak` keeps the old snapshot's address reserved, so a new
    /// snapshot, allocated after the old one is dropped, is never mistaken
    /// for it.
    #[test]
    fn a_dropped_snapshot_is_never_matched_by_one_allocated_after_it() {
        let memo: CensusMemo<u8, u32> = Memo::default();
        for round in 0..64_u32 {
            let source = snapshot();
            assert_eq!(memo.of_census(&source, 0, 0, || (round, true)), round);
        }
    }

    /// A file stamp moves with a rewrite of the same length and with a
    /// replacement, and stays for an untouched file.
    #[test]
    fn a_file_stamp_moves_with_any_write_and_stays_without_one() {
        let dir = crate::scratch::path("answer-memo-stamp");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(std::fs::create_dir_all(&dir).is_ok());
        let file = dir.join("catalogue.csv");
        assert!(std::fs::write(&file, b"a,b\n").is_ok());
        let first = FileStamp::of(&file).ok();
        assert!(first.is_some());
        assert_eq!(FileStamp::of(&file).ok(), first, "untouched");
        let memo: Memo<FileStamp, u8, u8> = Memo::default();
        let stamp = |path: &std::path::Path| FileStamp::of(path).unwrap_or_else(|_| unreachable!());
        assert_eq!(memo.get(stamp(&file), 0, 0, || (1, true)), 1);
        assert_eq!(memo.get(stamp(&file), 0, 0, || (2, true)), 1, "a hit");
        std::thread::sleep(std::time::Duration::from_millis(20));
        assert!(std::fs::write(&file, b"c,d\n").is_ok());
        assert_ne!(FileStamp::of(&file).ok(), first, "same length, new bytes");
        assert_eq!(memo.get(stamp(&file), 0, 0, || (3, true)), 3);
        let fresh = dir.join("fresh.csv");
        assert!(std::fs::write(&fresh, b"c,d\n").is_ok());
        assert!(std::fs::rename(&fresh, &file).is_ok());
        assert_eq!(memo.get(stamp(&file), 0, 0, || (4, true)), 4, "replaced");
        assert!(FileStamp::of(&dir.join("absent")).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A capped memo never holds more than its cap.
    #[test]
    fn a_capped_memo_never_holds_more_than_its_cap() {
        let memo: CensusMemo<u8, u8> = Memo::with_cap(2);
        let source = snapshot();
        for key in 0..5_u8 {
            assert_eq!(memo.of_census(&source, 0, key, || (key, true)), key);
            assert!(memo.len() <= 2, "{}", memo.len());
        }
        assert_eq!(memo.len(), 1, "the fifth insert found two and dropped them");
        assert_eq!(memo.of_census(&source, 0, 4, || (9, true)), 4);
    }

    /// An answer marked not keepable is built again on the next request.
    #[test]
    fn a_refusal_is_never_kept() {
        let memo: CensusMemo<u8, &str> = Memo::default();
        let source = snapshot();
        assert_eq!(
            memo.of_census(&source, 0, 0, || ("refused", false)),
            "refused"
        );
        assert_eq!(memo.len(), 0);
        assert_eq!(memo.of_census(&source, 0, 0, || ("served", true)), "served");
        assert_eq!(memo.of_census(&source, 0, 0, || ("other", true)), "served");
    }

    /// A build that finishes after another request installed a newer snapshot
    /// is answered to its own request and not kept under the newer one.
    #[test]
    fn a_build_overtaken_by_a_newer_snapshot_is_not_kept() {
        let memo: CensusMemo<u8, &str> = Memo::default();
        let old = snapshot();
        let new = snapshot();
        let answer = memo.of_census(&old, 0, 0, || {
            assert_eq!(memo.of_census(&new, 0, 0, || ("new", true)), "new");
            ("old", true)
        });
        assert_eq!(answer, "old");
        assert_eq!(memo.of_census(&new, 0, 0, || ("rebuilt", true)), "new");
        assert_eq!(memo.len(), 1);
    }
}
