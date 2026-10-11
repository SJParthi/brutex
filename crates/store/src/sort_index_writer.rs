//! The writer's half of the `.srt` sort index: one run appended after every
//! commit and after a writer opens, and a month rewritten whole as a new file
//! when it must be. D-3349, WP-39.
//!
//! A child of `crate::file` so it shares that module's I/O helpers, its error
//! classification and the bar handle's committed state, and cannot drift from
//! them.
//!
//! # The writer's step, and why it never fails an append
//!
//! [`BarFile::keep_sort_index`] runs after a writer opens a month and after
//! every commit. It appends ONE run ordering the committed rows the index does
//! not cover yet — after a commit, exactly the batch just committed. The bars
//! are already durable when it runs, so it cannot refuse them: a failure is a
//! `store.srt` warning naming the reason, and every reader names the rows the
//! index does not cover and reads them itself under its own cap. The next
//! writer open appends the missing run.
//!
//! A month whose index is absent, damaged, not this month's, does not describe
//! these bars, or would reach [`COMPACT_AT`] runs with the next append is
//! written WHOLE as one run, into a new file renamed over the old name
//! (`Self::write_whole`), so no reader ever sees a file change under it.
//!
//! # Cost
//!
//! An append's run: the batch's rows read back through the verified path and
//! six sorts of them, `O(b log b)` for a batch of `b`. A whole month:
//! `O(n log n)` for its `n` rows, paid when a writer first opens a month that
//! predates D-3349, every [`COMPACT_AT`] appends, or by
//! [`BarFile::sort_index_existing`].

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};

use std::sync::PoisonError;

use super::{
    Access, Action, BarFile, StoreError, VerifiedBlock, fault, fsync_dir, lock_fault, open_read,
    open_rw, refuse_link, refuse_links_below, sync_hooked, table_of, time_index_path, write_fully,
    writer_open,
};
use crate::flock::Flock;
use crate::format::Bar;
use crate::path::{FileKind, StorePath};
use crate::sort_index::{self, COMPACT_AT, HEADER_LEN, RunKind, Shape, Walked, Why};

/// Where a bar file's sort index is and what it must be bound to.
#[derive(Debug, Clone)]
pub(super) struct SortSidecar {
    /// `<yyyy-mm>.srt`, composed by `crate::path`.
    pub(super) path: PathBuf,
    /// The month, rung and symbol it orders.
    pub(super) shape: Shape,
}

impl SortSidecar {
    /// The sort index of the bar file at `path` under `root`.
    pub(super) fn of(
        root: &Path,
        path: StorePath<'_>,
        timeframe_secs: u32,
        symbol_id: u32,
    ) -> Self {
        Self {
            path: path.with_file(FileKind::SortIndex).to_path_buf(root),
            shape: Shape::of(path.month(), timeframe_secs, symbol_id),
        }
    }

    /// The name a whole-month rewrite is written under before it is renamed
    /// over [`Self::path`]: `<yyyy-mm>.new.srt`, still a `.srt` sibling to
    /// every walk of the store, never read as the month's index.
    fn staging(&self) -> PathBuf {
        let stem = self
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("month");
        self.path.with_file_name(format!("{stem}.new.srt"))
    }
}

/// What [`BarFile::sort_index_existing`] found beside one month, and left.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Resorted {
    /// One run already ordered every committed row: nothing was written.
    Kept,
    /// The month was written whole as one run, because of the reason named.
    Built(String),
    /// No run can order these bars, for the reason named; nothing was
    /// written.
    Unsortable(String),
    /// The month commits no rows: there is nothing to order.
    Empty,
}

/// One month through [`BarFile::sort_index_existing`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortIndexed {
    /// The committed bars, from the header.
    pub n_valid: u64,
    /// The runs the index holds afterwards.
    pub runs: usize,
    /// What the step found and did.
    pub outcome: Resorted,
}

/// What the writer's step decided for one month.
enum Plan {
    /// The index orders every committed row.
    Kept,
    /// Append one run over `[from, n_valid)` at byte `at`.
    Append { from: u64, at: u64 },
    /// Write the month whole: absent, damaged, not these bars, or past
    /// [`COMPACT_AT`] runs.
    Whole,
}

impl BarFile {
    /// Attaches the sort index of the bar file at `path` to this writer and
    /// runs the writer's step once. D-3349.
    pub(super) fn attach_sort_index(&mut self, root: &Path, path: StorePath<'_>) {
        if path.file() != FileKind::Bars {
            return;
        }
        self.srt = Some(SortSidecar::of(
            root,
            path,
            self.header.timeframe_secs,
            self.header.symbol_id,
        ));
        self.keep_sort_index();
    }

    /// The writer's sort-index step: append one run over the committed rows
    /// the index does not cover, or write the month whole. Never refuses the
    /// caller; a failure is a `store.srt` warning naming it. See the module
    /// header.
    pub(super) fn keep_sort_index(&self) {
        let Some(sidecar) = self.srt.as_ref() else {
            return;
        };
        if self.header.n_valid == 0 {
            return;
        }
        let kept = self.untouched(|| {
            self.plan_sort_index(sidecar).and_then(|plan| match plan {
                Plan::Kept => Ok(()),
                Plan::Append { from, at } => self.append_sort_run(sidecar, from, at),
                // SILENT ON SUCCESS: a month that predates D-3349, or one past
                // `COMPACT_AT` runs, is the writer's ordinary upkeep, and an
                // import writes no line for it. A failure is the warning below.
                Plan::Whole => self.write_whole(sidecar).map(|_| ()),
            })
        });
        if let Err(why) = kept {
            note_sort_index_lags(&sidecar.path, &why, self.header.n_valid);
        }
    }

    /// Runs `work`, which reads bars through this handle, and then puts the
    /// handle's verified-block cache back as it was.
    ///
    /// The sort step's reads are not the caller's: a writer that appended and
    /// then reads must meet the block cache its own reads left, or a block the
    /// sort step verified would be served from memory after the file was cut
    /// under the handle, where the caller's own read would have been refused
    /// (`store::write`'s short-read tests hold that).
    fn untouched<T>(&self, work: impl FnOnce() -> T) -> T {
        let saved = {
            let held = self.verified.lock().unwrap_or_else(PoisonError::into_inner);
            VerifiedBlock {
                block: held.block,
                n_valid: held.n_valid,
                start: held.start,
                len: held.len,
                bytes: held.bytes,
            }
        };
        let out = work();
        *self.verified.lock().unwrap_or_else(PoisonError::into_inner) = saved;
        out
    }

    /// What the month's index needs: read its header and run headers, and
    /// bind them to these bars.
    fn plan_sort_index(&self, sidecar: &SortSidecar) -> Result<Plan, String> {
        let n_valid = self.header.n_valid;
        let walked = match walk_sidecar(sidecar)? {
            None | Some(Err(_)) => return Ok(Plan::Whole),
            Some(Ok(walked)) => walked,
        };
        let Ok(coverage) = self.bind_sort_index(&walked) else {
            return Ok(Plan::Whole);
        };
        if coverage == n_valid {
            return Ok(Plan::Kept);
        }
        if walked.runs.len() + 1 >= COMPACT_AT {
            return Ok(Plan::Whole);
        }
        Ok(Plan::Append {
            from: coverage,
            at: walked.end,
        })
    }

    /// The rows `walked` covers, when its runs describe these bars.
    fn bind_sort_index(&self, walked: &Walked) -> Result<u64, Why> {
        sort_index::bind(
            &walked.runs,
            self.header.n_valid,
            self.header.first_ts_micros,
            self.header.last_ts_micros,
            |row| {
                self.read_row::<Bar>(row)
                    .map(|bar| bar.ts_micros)
                    .map_err(|why| why.to_string())
            },
        )
    }

    /// The committed bars `[from, to)`, through the verified read path.
    fn committed_rows(&self, from: u64, to: u64) -> Result<Vec<Bar>, String> {
        let mut bars = Vec::with_capacity(usize::try_from(to.saturating_sub(from)).unwrap_or(0));
        for row in from..to {
            bars.push(self.read_row::<Bar>(row).map_err(|why| why.to_string())?);
        }
        Ok(bars)
    }

    /// Appends one run over `[from, n_valid)` at byte `at`: the run, the file
    /// cut to the run's end (dropping only bytes past the last valid run,
    /// which no reader ever used), and ONE sync, which returns before any
    /// later run is begun — so only the last run can be torn by a crash, and a
    /// walk checks that one's body ([`RunKind`]). A run whose write or sync
    /// failed is cut off again, so no later run is appended after a body that
    /// may not be whole.
    fn append_sort_run(&self, sidecar: &SortSidecar, from: u64, at: u64) -> Result<(), String> {
        let bars = self.committed_rows(from, self.header.n_valid)?;
        let run =
            sort_index::build_run(&sidecar.shape, from, &bars).map_err(|why| why.to_string())?;
        refuse_link(&sidecar.path).map_err(|why| why.to_string())?;
        let file =
            writer_open(open_rw(&sidecar.path), &sidecar.path).map_err(|why| why.to_string())?;
        let end = at + run.len() as u64;
        let written = write_fully(&file, &sidecar.path, at, &run)
            .and_then(|()| fault(file.set_len(end), &sidecar.path, Action::Write))
            .and_then(|()| fault(sync_hooked(&file), &sidecar.path, Action::Sync));
        if let Err(why) = written {
            // BEST EFFORT, AND SAID: the failure itself is the warning the
            // caller writes; a cut that also fails leaves a run the next walk
            // checks whole, since it is the last.
            let _cut = file.set_len(at);
            return Err(why.to_string());
        }
        Ok(())
    }

    /// Every committed row as ONE run's bytes, or why no run can order them
    /// (the inner refusal); the outer one is a bar that would not read.
    fn whole_image(&self, sidecar: &SortSidecar) -> Result<Result<Vec<u8>, Why>, String> {
        let bars = self.committed_rows(0, self.header.n_valid)?;
        Ok(sort_index::build_run_as(
            &sidecar.shape,
            0,
            &bars,
            RunKind::Whole,
        ))
    }

    /// Writes every committed row as ONE run into a new file, then renames it
    /// over the month's index. Returns the runs written: one, or none for a
    /// month that commits no rows.
    ///
    /// # Errors
    ///
    /// The run could not be built (named by [`Why`]), or the host refused a
    /// write, a sync or the rename.
    fn write_whole(&self, sidecar: &SortSidecar) -> Result<usize, String> {
        if self.header.n_valid == 0 {
            return Ok(0);
        }
        let run = self.whole_image(sidecar)?.map_err(|why| why.to_string())?;
        write_image(sidecar, &run)?;
        Ok(1)
    }

    /// The month [`Self::sort_index_existing`] sorts, opened read-only under
    /// its EXCLUSIVE lock, and the root its served generation's files live
    /// under.
    ///
    /// THE GENERATION EVERY READER OPENS is the one sorted (D-3350, as
    /// `Self::index_existing` resolves it for the `.tix`): a repaired month's
    /// `.srt` is its revision's, and building the original's would report a
    /// month indexed that no reader reads. The links are still refused from
    /// the store root down.
    fn open_to_sort(
        store_root: &Path,
        path: StorePath<'_>,
        symbol_id: u32,
    ) -> Result<(Self, PathBuf), StoreError> {
        if path.file() != FileKind::Bars {
            return Err(StoreError::NotABarPath { found: path.file() });
        }
        let served = crate::generation::served(store_root, path)?;
        let served_root = served.root(store_root);
        let root = served_root.as_path();
        let bars_path = path.to_path_buf(root);
        let lock_path = path.with_file(FileKind::Lock).to_path_buf(root);
        refuse_links_below(store_root, &bars_path)?;
        let bars = open_read(&bars_path).map_err(|why| why.refusal(&bars_path))?;
        refuse_link(&lock_path)?;
        let lock = Flock::try_lock(
            writer_open(open_rw(&lock_path), &lock_path)?,
            lock_path.clone(),
        )
        .map_err(|refusal| lock_fault(&lock_path, refusal))?;
        let len = fault(bars.metadata(), &bars_path, Action::Measure)?.len();
        let file = Self::validated(
            bars,
            bars_path,
            Some(path.with_file(FileKind::Checksums).to_path_buf(root)),
            time_index_path(root, path),
            Some(lock),
            len,
            symbol_id,
            path.timeframe().secs(),
            path.month(),
            table_of(path.file()),
            Access::Read,
        )?;
        if let crate::generation::Served::Revision(pointer) = served
            && !pointer.describes(&file.header)
        {
            return Err(StoreError::Generation {
                path: crate::generation::pointer_path(store_root, path),
                reason: format!(
                    "revision {} is not the one its promotion recorded; not sorted",
                    pointer.revision.ordinal()
                ),
            });
        }
        Ok((file, served_root))
    }

    /// Builds, or confirms, one month's `.srt` sort index as ONE run over
    /// every committed row, and changes nothing else. D-3349, WP-39.
    ///
    /// The door `cli srt-build` walks, shaped as
    /// [`Self::index_existing`] is for the `.tix`: a month that is not there
    /// is [`StoreError::Missing`] with nothing created; the bar file and its
    /// checksums are opened read-only; the month's lock is taken EXCLUSIVE and
    /// non-blocking, so a writer or a reader mid-request is
    /// [`StoreError::Locked`] and nothing is written; no link below the root
    /// is followed. A month already held as one run that describes these bars
    /// AND whose body matches the checksum its header holds is
    /// [`Resorted::Kept`] — its headers and one read of the file — so a rerun
    /// is safe and changes no byte of it (a `.new.srt` a dead build left
    /// beside it is removed). Anything else — a damaged body among it — is
    /// written whole into a NEW file renamed over the name, never edited in
    /// place.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotABarPath`] for any other file kind;
    /// [`StoreError::Symlinked`] for a link below the root; the open
    /// refusals of the bar file; [`StoreError::Locked`]; whatever
    /// `Self::validated` refuses; a write the host refused, as
    /// [`StoreError::SortIndex`]; and [`StoreError::Generation`] when the
    /// month's served generation cannot be told, or its revision is not the
    /// one its promotion recorded (D-3350). A repaired month is sorted in its
    /// revision, beside the `.bin` every reader opens.
    pub fn sort_index_existing(
        root: &Path,
        path: StorePath<'_>,
        symbol_id: u32,
    ) -> Result<SortIndexed, StoreError> {
        let (mut file, served_root) = Self::open_to_sort(root, path, symbol_id)?;
        let root = served_root.as_path();
        let sidecar = SortSidecar::of(
            root,
            path,
            file.header.timeframe_secs,
            file.header.symbol_id,
        );
        let n_valid = file.header.n_valid;
        let refused = |why: String| StoreError::SortIndex {
            path: sidecar.path.clone(),
            reason: why,
        };
        if n_valid == 0 {
            return Ok(SortIndexed {
                n_valid,
                runs: 0,
                outcome: Resorted::Empty,
            });
        }
        let because = match walk_sidecar(&sidecar).map_err(refused)? {
            None => Why::Absent.to_string(),
            Some(Err(why)) => why.to_string(),
            Some(Ok(walked)) => match file.bind_sort_index(&walked) {
                Ok(coverage)
                    if coverage == n_valid && walked.runs.len() == 1 && walked.torn.is_none() =>
                {
                    // KEPT ONLY WHEN ITS BODY IS WHOLE. The walk read headers:
                    // a flipped byte in a fence table or a block passes it, a
                    // reader that reaches the byte refuses the month and
                    // prints this door's command, and a door that answered
                    // `kept` would be the repair that hides the damage. One
                    // read of the file (about 20 MB a one-second month).
                    match damaged(&sidecar, &walked).map_err(refused)? {
                        None => {
                            clear_staging(&sidecar).map_err(refused)?;
                            return Ok(SortIndexed {
                                n_valid,
                                runs: 1,
                                outcome: Resorted::Kept,
                            });
                        }
                        Some(why) => why.to_string(),
                    }
                }
                Ok(coverage) if coverage < n_valid => {
                    format!("its runs order rows 0..{coverage} of {n_valid}")
                }
                Ok(_) if walked.runs.len() > 1 => {
                    format!("it holds {} runs; a rebuild holds one", walked.runs.len())
                }
                Ok(_) => format!(
                    "bytes past its last run are not a run ({})",
                    walked.torn.as_ref().map_or("", |(_, why)| why.as_str())
                ),
                Err(why) => why.to_string(),
            },
        };
        file.srt = Some(sidecar.clone());
        // ONE BUILD: a month no run can order is named and nothing is
        // written; otherwise the bytes built here are the bytes written.
        let run = match file.whole_image(&sidecar).map_err(refused)? {
            Ok(run) => run,
            Err(why) => {
                return Ok(SortIndexed {
                    n_valid,
                    runs: 0,
                    outcome: Resorted::Unsortable(why.to_string()),
                });
            }
        };
        write_image(&sidecar, &run).map_err(refused)?;
        note_sort_index_built(&sidecar.path, &because, n_valid);
        Ok(SortIndexed {
            n_valid,
            runs: 1,
            outcome: Resorted::Built(because),
        })
    }
}

/// Writes `run` as the month's whole index: the header and the run into
/// `<yyyy-mm>.new.srt`, a sync, then a rename over `<yyyy-mm>.srt` and a sync
/// of the directory — never an edit of the file a reader may hold.
fn write_image(sidecar: &SortSidecar, run: &[u8]) -> Result<(), String> {
    let staging = sidecar.staging();
    let io = |why: StoreError| why.to_string();
    refuse_link(&staging).map_err(io)?;
    refuse_link(&sidecar.path).map_err(io)?;
    // A STAGING FILE LEFT BY A WRITER THAT DIED is never the month's index —
    // only a rename makes one — so it is cut, not trusted.
    let file = writer_open(open_rw(&staging), &staging).map_err(io)?;
    fault(file.set_len(0), &staging, Action::Write).map_err(io)?;
    write_fully(&file, &staging, 0, &sidecar.shape.encode()).map_err(io)?;
    write_fully(&file, &staging, HEADER_LEN, run).map_err(io)?;
    fault(sync_hooked(&file), &staging, Action::Sync).map_err(io)?;
    drop(file);
    fault(
        fs::rename(&staging, &sidecar.path),
        &sidecar.path,
        Action::Write,
    )
    .map_err(io)?;
    fsync_dir(sidecar.path.parent().unwrap_or(&sidecar.path)).map_err(io)
}

/// Why the walked index's bodies are not whole, or `None` when every run's
/// body matches the checksum its header holds
/// ([`sort_index::verify_bodies`]).
fn damaged(sidecar: &SortSidecar, walked: &Walked) -> Result<Option<Why>, String> {
    let file = open_read(&sidecar.path).map_err(|why| why.refusal(&sidecar.path).to_string())?;
    match sort_index::verify_bodies(|at, buf| file.read_exact_at(buf, at), &walked.runs) {
        Ok(()) => Ok(None),
        Err(Why::Unreadable(why)) => Err(format!("{}: {why}", sidecar.path.display())),
        Err(why) => Ok(Some(why)),
    }
}

/// Removes `<yyyy-mm>.new.srt` when a build that died left one beside an
/// index that is kept: the staging name is never a month's index (only a
/// rename makes one), the month's lock is held, and a build that writes
/// would have cut it. Nothing is removed that is not a regular file.
fn clear_staging(sidecar: &SortSidecar) -> Result<(), String> {
    let staging = sidecar.staging();
    match fs::symlink_metadata(&staging) {
        Ok(meta) if meta.is_file() => {
            fs::remove_file(&staging).map_err(|why| format!("{}: {why}", staging.display()))?;
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::info("store.srt", "staging file of a dead build removed")
                    .with(
                        "file",
                        telemetry::Value::Str(&staging.display().to_string()),
                    )
                    .with("bytes", telemetry::Value::Uint(meta.len())),
            );
            Ok(())
        }
        Ok(_) => Err(format!(
            "{} is not a regular file; it is left as it is",
            staging.display()
        )),
        Err(why) if why.kind() == ErrorKind::NotFound => Ok(()),
        Err(why) => Err(format!("{}: {why}", staging.display())),
    }
}

/// The month's index, walked: `None` when there is none, the refusal
/// when it would not walk.
fn walk_sidecar(sidecar: &SortSidecar) -> Result<Option<Result<Walked, Why>>, String> {
    let meta = match fs::symlink_metadata(&sidecar.path) {
        Ok(meta) => meta,
        Err(why) if why.kind() == ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(format!("{}: {why}", sidecar.path.display())),
    };
    if !meta.is_file() {
        return Err(format!(
            "{} is not a regular file; it is left as it is",
            sidecar.path.display()
        ));
    }
    let file = open_read(&sidecar.path).map_err(|why| why.refusal(&sidecar.path).to_string())?;
    Ok(Some(sort_index::walk(
        |at, buf| file.read_exact_at(buf, at),
        meta.len(),
        &sidecar.shape,
    )))
}

/// `store.srt`, WARN: a writer could not keep the month's sort index in step
/// with its committed bars. The bars are committed; every reader names the
/// rows the index does not cover and reads them itself, and the next writer
/// open tries again.
fn note_sort_index_lags(path: &Path, why: &str, n_valid: u64) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.srt", "sort index lags the bars")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("reason", telemetry::Value::Str(why))
            .with("n_valid", telemetry::Value::Uint(n_valid)),
    );
}

/// `store.srt`, INFO: a month's sort index was written whole by the rebuild
/// door, and why it had to be.
fn note_sort_index_built(path: &Path, why: &str, n_valid: u64) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("store.srt", "sort index written whole")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("reason", telemetry::Value::Str(why))
            .with("n_valid", telemetry::Value::Uint(n_valid)),
    );
}

#[cfg(test)]
#[path = "sort_index_writer_tests.rs"]
mod tests;
