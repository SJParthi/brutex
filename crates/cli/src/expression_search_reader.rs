//! Read-only, bounded views of an observed expression-search checkpoint chain.
//! This verifies saved receipts and grammar transitions, not the original market
//! inputs or nine-term identity construction. Continuations are learned only by
//! walking a previously admitted snapshot; arbitrary predecessor tokens refuse.

use super::{CHECKPOINT_MAX, Candidate, State, add, candidate_path, debug};
use crate::search_checkpoint::{Saved, Snapshot};
use runner::expression::Summary;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use vocab::expression_search::{Cursor, Step};

/// Maximum checkpoint links visited in a page; some links contain no candidate.
pub const MAX_LINKS: usize = 256;
const MAX_CONTINUATIONS: usize = 4096;
// Each immutable checkpoint reads its <=8192-byte payload file and its
// 32-byte completion marker twice. Charge the full admitted bytes before I/O.
const CHECKPOINT_READ_MAX: u64 = CHECKPOINT_MAX + 64;
// Exact-attempt reads touch lifecycle, immutable start and two optional detail
// files. Expression children do not publish depth/frontier history.
const ATTEMPT_FILE_MAX: u64 = 4096;
// Search pricing writes one tier/two sides, whose catalog/start both fit here.
// A larger independent capture is refused explicitly by this search view.
const CAPTURE_FILE_MAX: u64 = 8192;

/// Exact sealed position, never a mutable latest alias.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Anchor {
    /// Immutable reservation sequence.
    pub sequence: u64,
    /// Full payload/header seal.
    pub seal: [u8; 32],
}
/// Observed progress from one admitted snapshot.
#[derive(Clone, Debug)]
pub struct Progress {
    /// Exact search identity named by the checkpoint envelope.
    pub identity: [u8; 32],
    /// Initial checkpoint, absent before the first complete marker.
    pub checkpoint: Option<Anchor>,
    /// Recorded grammar-node work.
    pub work: u64,
    /// Recorded evaluated expression count.
    pub candidates: u64,
    /// Recorded support-qualifying count; threshold not re-proven here.
    pub qualifying: u64,
    /// Recorded signal-row observations.
    pub rows: u64,
    /// Cursor proves exhaustion of its versioned language.
    pub exhausted: bool,
    /// Owner lock was held when observed; not a continuing liveness guarantee.
    pub writer_observed: bool,
    /// Reservations with no completion marker when observed.
    pub interrupted: u64,
}
/// One exact expression child whose signal receipt was verified.
#[derive(Clone, Debug)]
pub struct Row {
    /// One-based evaluated candidate ordinal in this search.
    pub ordinal: u64,
    /// Saved child's own identity, distinct from the search identity.
    pub identity: [u8; 32],
    /// Child attempt required by the exact-trade endpoint.
    pub attempt: u64,
    /// Canonical complete expression text from the vocabulary formatter.
    pub expression: String,
    /// Referenced bits, not an AND interpretation.
    pub mask_words: [u64; 6],
    /// Reconciled exact saved signal counts.
    pub summary: Summary,
    /// Sealed pricing catalog if present; trade bodies verify when opened.
    pub capture_digest: Option<[u8; 32]>,
}
/// A bounded reverse-chronological page through checkpoint links.
pub struct Page {
    /// Exact candidate receipts, possibly empty on a progress-only page.
    pub rows: Vec<Row>,
    /// Number of checkpoint links actually verified.
    pub links: usize,
    /// Next learned predecessor, including through a page with zero candidates.
    pub next: Option<Anchor>,
}
/// Read-only snapshot reader with bounded, verified continuation authorities.
pub struct Reader {
    root: PathBuf,
    snapshot: Snapshot,
    progress: Progress,
    admitted: HashSet<Anchor>,
}
impl Reader {
    /// Opens existing checkpoints without creating directories or owning the run.
    ///
    /// # Errors
    /// Unknown/corrupt/reserved paths, changed seals, or admission overflow.
    pub fn open(root: &Path, identity: [u8; 32], max_bytes: u64) -> Result<Option<Self>, String> {
        let Some(snapshot) = Snapshot::open(root, "expression-search-v1", identity)? else {
            return Ok(None);
        };
        let saved = snapshot
            .latest
            .map(|sequence| snapshot.read(sequence, max_bytes.min(CHECKPOINT_MAX)))
            .transpose()?;
        let state = saved
            .as_ref()
            .map(|saved| State::decode(&saved.payload))
            .transpose()?;
        let checkpoint = saved.as_ref().map(|saved| Anchor {
            sequence: saved.sequence,
            seal: saved.seal,
        });
        let progress = Progress {
            identity,
            checkpoint,
            work: state.as_ref().map_or(0, |value| value.work),
            candidates: state.as_ref().map_or(0, |value| value.candidates),
            qualifying: state.as_ref().map_or(0, |value| value.qualifying),
            rows: state.as_ref().map_or(0, |value| value.rows),
            exhausted: state.as_ref().is_some_and(|value| value.exhausted),
            writer_observed: snapshot.writer_observed,
            interrupted: snapshot.interrupted,
        };
        Ok(Some(Self {
            root: root.to_path_buf(),
            snapshot,
            progress,
            admitted: checkpoint.into_iter().collect(),
        }))
    }
    /// Immutable observed counters. These are not a live process census.
    #[must_use]
    pub const fn progress(&self) -> &Progress {
        &self.progress
    }

    /// Visits at most `limit` checkpoint links, sharing one byte budget.
    ///
    /// # Errors
    /// Refuses unlearned cursors, damaged/missing children, inconsistent history,
    /// changed original snapshot, or a page/continuation admission limit.
    pub fn page(
        &mut self,
        cursor: Option<Anchor>,
        limit: usize,
        max_bytes: u64,
    ) -> Result<Page, String> {
        if limit == 0 || limit > MAX_LINKS {
            return Err("search page limit must be 1..=256 checkpoint links".to_owned());
        }
        let cursor = cursor.or(self.progress.checkpoint);
        if cursor.is_some_and(|anchor| !self.admitted.contains(&anchor)) {
            return Err("search cursor is not linked to this admitted snapshot".to_owned());
        }
        let mut remaining = max_bytes;
        if let Some(anchor) = self.progress.checkpoint {
            self.budgeted(anchor, &mut remaining)?;
        }
        let mut page = Page {
            rows: Vec::new(),
            links: 0,
            next: cursor,
        };
        while page.links < limit {
            let Some(anchor) = page.next else {
                break;
            };
            let saved = self.budgeted(anchor, &mut remaining)?;
            let current = State::decode(&saved.payload)?;
            let prior = if current.previous.0 == 0 {
                None
            } else {
                if current.previous.0 >= anchor.sequence {
                    return Err("cyclic search checkpoint history".to_owned());
                }
                Some(self.budgeted(
                    Anchor {
                        sequence: current.previous.0,
                        seal: current.previous.1,
                    },
                    &mut remaining,
                )?)
            };
            let before = match &prior {
                Some(saved) => State::decode(&saved.payload)?,
                None => {
                    State::new(Cursor::decode(&current.cursor.initial_descriptor()).map_err(debug)?)
                }
            };
            let child = current
                .last
                .as_ref()
                .map(|candidate| self.child(candidate, current.candidates, &mut remaining))
                .transpose()?;
            transition(&before, &current, child.as_ref())?;
            if let Some(child) = child {
                page.rows.push(child);
            }
            page.links += 1;
            page.next = prior.map(|saved| Anchor {
                sequence: saved.sequence,
                seal: saved.seal,
            });
        }
        if let Some(anchor) = self.progress.checkpoint {
            self.budgeted(anchor, &mut remaining)?;
        }
        if let Some(next) = page.next {
            if !self.admitted.contains(&next) && self.admitted.len() >= MAX_CONTINUATIONS {
                return Err(
                    "search snapshot continuation limit reached; open a new snapshot".to_owned(),
                );
            }
            self.admitted.try_reserve(1).map_err(debug)?;
            self.admitted.insert(next);
        }
        Ok(page)
    }
    fn require(&self, anchor: Anchor, max_bytes: u64) -> Result<Saved, String> {
        let saved = self
            .snapshot
            .read(anchor.sequence, max_bytes.min(CHECKPOINT_MAX))?;
        if saved.seal != anchor.seal {
            return Err("search checkpoint seal changed".to_owned());
        }
        Ok(saved)
    }
    fn budgeted(&self, anchor: Anchor, remaining: &mut u64) -> Result<Saved, String> {
        charge(remaining, CHECKPOINT_READ_MAX)?;
        self.require(anchor, CHECKPOINT_MAX)
    }
    fn child(
        &self,
        candidate: &Candidate,
        ordinal: u64,
        remaining: &mut u64,
    ) -> Result<Row, String> {
        charge(remaining, 4 * ATTEMPT_FILE_MAX)?;
        let evidence = crate::sweep_evidence::read_attempt(
            &self.root,
            candidate.identity,
            candidate.attempt,
            ATTEMPT_FILE_MAX,
        )?
        .ok_or("search child has no exact attempt")?;
        if evidence.operation != crate::sweep_evidence::Operation::Expression
            || evidence.completion != crate::sweep_evidence::Completion::Completed
        {
            return Err("search child has no completed expression receipt".to_owned());
        }
        let path = candidate_path(&self.root, candidate);
        let file = crate::readonly_file::open(&path).map_err(debug)?;
        file.try_lock_shared()
            .map_err(|why| format!("search signal receipt is busy or cannot be read: {why}"))?;
        let before = crate::result_set::file_generation(&file, &path)?;
        let size = std::fs::metadata(&path).map_err(debug)?.len();
        charge(
            remaining,
            size.checked_mul(3)
                .ok_or("search signal verification byte cost overflow")?,
        )?;
        let (identity, expression, summary) =
            crate::expression::read_saved(&path, size, |_| Ok(())).map_err(debug)?;
        let after = crate::result_set::file_generation(&file, &path)?;
        crate::result_set::require_generation_unchanged(before, after, &path)?;
        if identity != candidate.identity || summary != candidate.summary {
            return Err("search child differs from its recorded signal summary".to_owned());
        }
        charge(remaining, 2 * CAPTURE_FILE_MAX)?;
        let capture = crate::candidate_trades::read_model(
            &self.root,
            identity,
            candidate.attempt,
            crate::candidate_trades::Model::Expression,
            CAPTURE_FILE_MAX,
        )?;
        if capture
            .as_ref()
            .is_some_and(|saved| saved.expression() != Some(&expression))
        {
            return Err(
                "search child pricing predicate differs from its signal predicate".to_owned(),
            );
        }
        Ok(Row {
            ordinal,
            identity,
            attempt: candidate.attempt,
            expression: expression.to_string(),
            mask_words: expression.referenced().words(),
            summary,
            capture_digest: capture.map(|saved| saved.digest),
        })
    }
}
fn charge(remaining: &mut u64, bytes: u64) -> Result<(), String> {
    *remaining = remaining
        .checked_sub(bytes)
        .ok_or("search page byte admission exhausted; no prefix exposed")?;
    Ok(())
}
fn transition(before: &State, current: &State, child: Option<&Row>) -> Result<(), String> {
    let delta = current
        .work
        .checked_sub(before.work)
        .filter(|n| *n <= 4096)
        .ok_or("search work transition exceeds its bound")?;
    if current.candidates != add(before.candidates, u64::from(child.is_some()))?
        || current.rows != add(before.rows, child.map_or(0, |row| row.summary.evaluated))?
        || current.qualifying < before.qualifying
        || current.qualifying - before.qualifying > u64::from(child.is_some())
    {
        return Err("search checkpoint counters do not reconcile".to_owned());
    }
    let mut cursor = before.cursor.clone();
    let mut work = before.work;
    let step = cursor.advance(delta, &mut work).map_err(debug)?;
    let expected = match &step {
        Step::Candidate(expression) => Some(expression.to_string()),
        Step::Paused | Step::Exhausted => None,
    };
    if work != current.work
        || cursor.encode() != current.cursor.encode()
        || expected.as_deref() != child.map(|row| row.expression.as_str())
        || matches!(step, Step::Exhausted) != current.exhausted
    {
        return Err("search checkpoint does not prove its exact grammar transition".to_owned());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search_checkpoint::{Journal, tests::Scratch};

    fn recorded(root: &Path) -> Result<([u8; 32], Journal), String> {
        use indicators::evaluator::{Calendar, Evaluator, Widths};
        let bars = runner::synthetic::sessions(8);
        let mut evaluator = Evaluator::with_calendar(
            Widths::pinned().map_err(debug)?,
            indicators::vwap::Availability::Absent,
            indicators::pattern::Thresholds::CLASSICAL,
            Calendar::all_regular(),
        );
        let column = indicators::column::Column::build(&bars, &mut evaluator);
        let cursor = Cursor::new(&[0, 369]).map_err(debug)?;
        let key = brutex_core::instrument::InstrumentKey::index(
            brutex_core::instrument::Exchange::Nse,
            "NIFTY",
        )
        .map_err(debug)?;
        let run = runner::identity::Run {
            mask: cursor.alphabet(),
            direction: runner::identity::Direction::Undirected,
            instrument: &key,
            timeframe: "synthetic",
            params: runner::identity::Params::of(engine::Ladder::with_min_hits(1)),
            data_digest: [8; 32],
            commit: "reader-fixture",
            feed: "synthetic",
        };
        let identity = runner::expression::search_identity(&run, &cursor);
        let mut journal = Journal::open(root, "expression-search-v1", identity)?;
        super::super::execute(
            root,
            &run,
            &column,
            &bars,
            &mut journal,
            State::new(cursor),
            (0, [0; 32]),
            (3, 10000),
            None,
        )?;
        Ok((identity, journal))
    }
    #[test]
    fn active_search_is_read_without_ownership_and_zero_candidate_pages_continue()
    -> Result<(), String> {
        let scratch = Scratch::new().map_err(debug)?;
        assert!(Reader::open(&scratch.0, [1; 32], 64 * 1024 * 1024)?.is_none());
        assert!(!scratch.0.join("expression-search-v1").exists());
        let (identity, journal) = recorded(&scratch.0)?;
        let mut reader = Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?.ok_or("reader")?;
        assert!(reader.progress().writer_observed);
        assert_eq!(reader.progress().candidates, 3);
        let first = reader.page(None, 1, 64 * 1024 * 1024)?;
        assert_eq!(first.links, 1);
        assert!(first.rows.is_empty());
        assert!(first.next.is_some());
        assert!(
            Journal::open(&scratch.0, "expression-search-v1", identity).is_err(),
            "reader did not steal ownership"
        );
        let mut next = first.next;
        let mut ordinals = Vec::new();
        while let Some(cursor) = next {
            let page = reader.page(Some(cursor), 1, 64 * 1024 * 1024)?;
            assert!(page.links <= 1 && page.rows.len() <= 1);
            for row in page.rows {
                assert!(row.summary.evaluated > 0 && row.summary.reconciles());
                assert!(row.capture_digest.is_none());
                assert!(!row.expression.is_empty());
                ordinals.push(row.ordinal);
            }
            next = page.next;
        }
        assert_eq!(ordinals, vec![3, 2, 1]);
        drop(journal);
        assert!(
            !Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?
                .ok_or("paused reader")?
                .progress()
                .writer_observed
        );
        Ok(())
    }
    #[test]
    fn unlearned_missing_changed_and_over_budget_pages_never_return_a_prefix() -> Result<(), String>
    {
        let scratch = Scratch::new().map_err(debug)?;
        let (identity, journal) = recorded(&scratch.0)?;
        let mut reader = Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?.ok_or("reader")?;
        assert!(reader.page(None, 257, 64 * 1024 * 1024).is_err());
        assert!(
            reader
                .page(
                    Some(Anchor {
                        sequence: 1,
                        seal: [1; 32]
                    }),
                    1,
                    64 * 1024 * 1024
                )
                .is_err()
        );
        assert!(reader.page(None, 256, 1).is_err());
        let first = reader.page(None, 1, 64 * 1024 * 1024)?;
        let cursor = first.next.ok_or("child cursor")?;
        let current = State::decode(&journal.read(cursor.sequence, CHECKPOINT_MAX)?.payload)?;
        let child = current.last.ok_or("candidate")?;
        std::fs::remove_file(candidate_path(&scratch.0, &child)).map_err(debug)?;
        assert!(reader.page(Some(cursor), 256, 64 * 1024 * 1024).is_err());
        let latest = journal.latest(CHECKPOINT_MAX)?.ok_or("latest")?;
        let path = scratch
            .0
            .join("expression-search-v1")
            .join(super::super::hex(&identity))
            .join(format!("{:016x}", latest.sequence))
            .join("payload");
        let mut bytes = std::fs::read(&path).map_err(debug)?;
        *bytes.last_mut().ok_or("footer")? ^= 1;
        std::fs::write(&path, bytes).map_err(debug)?;
        assert!(reader.page(None, 1, 64 * 1024 * 1024).is_err());
        Ok(())
    }
    #[test]
    fn newly_sealed_but_inconsistent_counter_transition_is_refused() -> Result<(), String> {
        let scratch = Scratch::new().map_err(debug)?;
        let (identity, mut journal) = recorded(&scratch.0)?;
        let latest = journal.latest(CHECKPOINT_MAX)?.ok_or("latest")?;
        let mut state = State::decode(&latest.payload)?;
        state.previous = (latest.sequence, latest.seal);
        state.candidates += 1;
        journal.publish(&state.encode(), CHECKPOINT_MAX)?;
        let mut reader = Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?.ok_or("reader")?;
        assert!(reader.page(None, 256, 64 * 1024 * 1024).is_err());
        Ok(())
    }

    fn busy_refuses(
        path: &Path,
        read: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> Result<(), String> {
        let owner = std::fs::File::open(path).map_err(debug)?;
        owner.lock().map_err(debug)?;
        let (send, receive) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || send.send(read()));
        let answer = receive.recv_timeout(std::time::Duration::from_millis(500));
        owner.unlock().map_err(debug)?;
        worker
            .join()
            .map_err(|_| "reader panicked")?
            .map_err(debug)?;
        let why = answer
            .map_err(debug)?
            .err()
            .ok_or("busy evidence must refuse")?;
        assert!(why.contains("busy"), "{why}");
        Ok(())
    }

    #[test]
    fn busy_checkpoint_and_signal_receipts_refuse_without_stranding_a_reader() -> Result<(), String>
    {
        let scratch = Scratch::new().map_err(debug)?;
        let (identity, journal) = recorded(&scratch.0)?;
        let mut reader = Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?.ok_or("reader")?;
        let first = reader.page(None, 1, 64 * 1024 * 1024)?;
        let cursor = first.next.ok_or("child cursor")?;
        let current = State::decode(&journal.read(cursor.sequence, CHECKPOINT_MAX)?.payload)?;
        let child = current.last.ok_or("candidate")?;
        busy_refuses(&candidate_path(&scratch.0, &child), move || {
            reader.page(Some(cursor), 1, 64 * 1024 * 1024).map(|_| ())
        })?;
        let latest = journal.latest(CHECKPOINT_MAX)?.ok_or("latest")?;
        let checkpoint_path = scratch
            .0
            .join("expression-search-v1")
            .join(super::super::hex(&identity))
            .join(format!("{:016x}", latest.sequence))
            .join("payload");
        let root = scratch.0.clone();
        busy_refuses(&checkpoint_path, move || {
            Reader::open(&root, identity, 64 * 1024 * 1024).map(|_| ())
        })?;
        assert!(
            Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?
                .ok_or("reopened reader")?
                .page(None, 4, 64 * 1024 * 1024)
                .is_ok()
        );
        Ok(())
    }

    #[test]
    fn failed_multi_child_page_learns_nothing_and_the_same_cursor_can_retry() -> Result<(), String>
    {
        let scratch = Scratch::new().map_err(debug)?;
        let (identity, journal) = recorded(&scratch.0)?;
        let mut reader = Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?.ok_or("reader")?;
        let cursor = reader
            .page(None, 1, 64 * 1024 * 1024)?
            .next
            .ok_or("third candidate")?;
        let third = State::decode(&journal.read(cursor.sequence, CHECKPOINT_MAX)?.payload)?;
        let second = State::decode(&journal.read(third.previous.0, CHECKPOINT_MAX)?.payload)?;
        let unlearned = Anchor {
            sequence: second.previous.0,
            seal: second.previous.1,
        };
        let child = second.last.ok_or("second candidate")?;
        let path = candidate_path(&scratch.0, &child);
        let bytes = std::fs::read(&path).map_err(debug)?;
        std::fs::write(&path, b"damaged receipt").map_err(debug)?;
        assert!(reader.page(Some(cursor), 2, 64 * 1024 * 1024).is_err());
        let why = reader
            .page(Some(unlearned), 1, 64 * 1024 * 1024)
            .err()
            .ok_or("failed page leaked its continuation authority")?;
        assert!(why.contains("not linked"));
        std::fs::write(&path, bytes).map_err(debug)?;
        let retried = reader.page(Some(cursor), 2, 64 * 1024 * 1024)?;
        assert_eq!(
            retried
                .rows
                .iter()
                .map(|row| row.ordinal)
                .collect::<Vec<_>>(),
            vec![3, 2]
        );
        assert_eq!(retried.next, Some(unlearned));
        assert_eq!(
            reader
                .page(retried.next, 1, 64 * 1024 * 1024)?
                .rows
                .first()
                .map(|row| row.ordinal),
            Some(1)
        );
        Ok(())
    }

    #[test]
    fn complete_page_admission_charges_both_signal_passes_and_all_child_receipts()
    -> Result<(), String> {
        let scratch = Scratch::new().map_err(debug)?;
        let (identity, journal) = recorded(&scratch.0)?;
        let mut reader = Reader::open(&scratch.0, identity, 64 * 1024 * 1024)?.ok_or("reader")?;
        let cursor = reader
            .page(None, 1, 64 * 1024 * 1024)?
            .next
            .ok_or("third candidate")?;
        let third = State::decode(&journal.read(cursor.sequence, CHECKPOINT_MAX)?.payload)?;
        let second = State::decode(&journal.read(third.previous.0, CHECKPOINT_MAX)?.payload)?;
        // Initial/final anchor and current/prior checkpoint for each of two
        // candidates; each child reserves all documented cold-read passes.
        let mut expected = 6 * CHECKPOINT_READ_MAX;
        for state in [third, second] {
            let child = state.last.ok_or("candidate")?;
            let bytes = std::fs::metadata(candidate_path(&scratch.0, &child))
                .map_err(debug)?
                .len();
            expected += 4 * ATTEMPT_FILE_MAX + 3 * bytes + 2 * CAPTURE_FILE_MAX;
        }
        let why = reader
            .page(Some(cursor), 2, expected - 1)
            .err()
            .ok_or("one-byte-short admission exposed a page")?;
        assert!(why.contains("byte admission"));
        let page = reader.page(Some(cursor), 2, expected)?;
        assert_eq!(page.links, 2);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows.first().map(|row| row.ordinal), Some(3));
        assert_eq!(page.rows.last().map(|row| row.ordinal), Some(2));
        Ok(())
    }
}
