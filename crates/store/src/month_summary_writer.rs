//! The writer's half of the month summary: the `.sum` sibling kept in step
//! after every commit and when a writer opens, and the one library call that
//! rebuilds it. R3-06, D-4980.
//!
//! A child of `crate::file` so it reads committed rows through the verified
//! path, under the same handle, and cannot drift from what a reader reads.
//!
//! # The writer's step, and why it never fails an append
//!
//! [`BarFile::keep_month_summary`] runs after a writer opens a month and after
//! every commit, after the sort index's step. The bars are already committed
//! when it runs, so it cannot refuse them: a failure is a `store.sum` warning
//! naming the reason, the summary left beside the month describes an older
//! generation, and every reader refuses it as `stale` by a header comparison
//! and reads the bars itself, saying so. The next writer open tries again.
//!
//! A summary that describes the generation committed before this one (the
//! same file, fewer rows, its last row the stamp it names) is EXTENDED: the
//! rows committed since are read back and folded in, `O(batch)`. Anything else
//! — no summary, a torn or foreign one, a repaired month, a month whose seconds
//! the density rule now wants — is built WHOLE from every committed row, and
//! the `store.sum` log names that write and why (a month's first included).
//! Either way the new bytes are written to `<yyyy-mm>.new.sum` (a new inode,
//! [`crate::staging`]) and renamed over `<yyyy-mm>.sum`, so a reader never
//! sees a file change under it.
//!
//! # Not flushed, and why that is safe
//!
//! The summary is derived from bars that are already durable, and every read
//! checks its magic, length, CRC and the header it describes. A crash that
//! leaves it torn, empty or older is refused by name and rebuilt by the next
//! writer open; it can never be read as these bars' summary. A flush per
//! commit would buy nothing a reader relies on.
//!
//! # The small-month rule (D-4933)
//!
//! A month under [`INDEX_MIN_ROWS`](crate::time_index::INDEX_MIN_ROWS) bars
//! with no `.sum` is given none, as it is given no `.tix` and no `.srt`: a
//! reader builds its summary from its at most 1,023 bars. The commit that
//! reaches the threshold writes it whole. 552,686 of 852,757 months are under
//! the threshold; the rest pay one 4 KiB block and one inode each until they
//! are sealed, when the bytes move into the `.sbar` section and need neither.

use std::fs;
use std::path::{Path, PathBuf};

use super::{Action, BarFile, StoreError, classify, fault, refuse_link, write_fully};
use crate::format::Bar;
use crate::month_summary::{MonthSummary, Refused, path_beside};
use crate::path::{FileKind, StorePath};

/// Where a bar file's month summary is.
#[derive(Debug, Clone)]
pub(super) struct SummarySidecar {
    /// `<yyyy-mm>.sum`, beside the bar file the handle opened.
    pub(super) path: PathBuf,
}

impl SummarySidecar {
    /// The name a new image is written under before the rename.
    fn staging(&self) -> PathBuf {
        let stem = self
            .path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("month");
        self.path.with_file_name(format!("{stem}.new.sum"))
    }
}

/// What [`BarFile::month_summary_existing`] found beside one month, and did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Summarized {
    /// A summary already described exactly these bars: nothing was written.
    Kept,
    /// The summary was written whole, for the reason named.
    Built(String),
    /// The month commits no rows.
    Empty,
    /// Under the small-month rule and holding no `.sum`: none is made.
    BelowThreshold,
}

/// What the writer's step decided.
enum Plan {
    /// The summary describes every committed row.
    Kept,
    /// Under the threshold, and no summary: none is made.
    BelowThreshold,
    /// Extend this summary with rows `[from, n_valid)`.
    Extend(MonthSummary, u64),
    /// Build from every row, because of the reason named.
    Whole(String),
}

impl BarFile {
    /// Attaches the month summary beside this writer's bar file and runs the
    /// writer's step once.
    pub(super) fn attach_month_summary(&mut self, path: StorePath<'_>) {
        if path.file() != FileKind::Bars {
            return;
        }
        self.msum = Some(SummarySidecar {
            path: path_beside(&self.bars_path),
        });
        self.keep_month_summary();
    }

    /// The writer's month-summary step. Never refuses the caller; a failure
    /// is a `store.sum` warning. See the module header.
    pub(super) fn keep_month_summary(&self) {
        let Some(sidecar) = self.msum.as_ref() else {
            return;
        };
        if self.header.n_valid == 0 {
            return;
        }
        let kept = self.untouched(|| match self.plan_month_summary() {
            Plan::Kept | Plan::BelowThreshold => Ok(()),
            Plan::Extend(summary, from) => self.extend_summary(sidecar, summary, from),
            Plan::Whole(because) => self
                .whole_summary()
                .map_err(|why| why.to_string())
                .and_then(|summary| write_summary(sidecar, &summary))
                .map(|()| {
                    // EVERY WHOLE WRITE IS NAMED, a month's first one
                    // included (rule 3: every state change is logged). One
                    // line a month when it first reaches the threshold, as
                    // gate 17's one event per instrument-month allows; the
                    // per-commit extension below stays silent upkeep.
                    note_summary_built(&sidecar.path, &because, self.header.n_valid);
                }),
        });
        if let Err(why) = kept {
            note_summary_lags(&sidecar.path, &why, self.header.n_valid);
        }
    }

    fn plan_month_summary(&self) -> Plan {
        let header = self.header;
        let month = self.admission.month;
        // ONE READ OF THE KEPT FILE, through the reader's own no-follow,
        // non-blocking, regular-file-only door: decided from these bytes,
        // never a second open that could find other ones.
        let summary = match crate::month_summary::read_kept(&self.bars_path, header.n_valid) {
            Ok(summary) => summary,
            Err(Refused::BelowThreshold) => return Plan::BelowThreshold,
            Err(why) => return Plan::Whole(why.to_string()),
        };
        let refused = match summary.describes(&header, month) {
            Ok(()) => return Plan::Kept,
            Err(why) => why,
        };
        let Refused::Stale { .. } = refused else {
            return Plan::Whole(refused.to_string());
        };
        // STALE BY APPENDS ONLY: an older generation of THIS file, fewer
        // rows, and its last row still the stamp it names.
        let prefix = summary.generation < header.generation
            && summary.n_valid > 0
            && summary.n_valid < header.n_valid
            && summary.counts.folded == summary.n_valid
            && summary.first_ts_micros == header.first_ts_micros
            && self
                .read_row::<Bar>(summary.n_valid - 1)
                .is_ok_and(|bar| bar.ts_micros == summary.last_ts_micros);
        if prefix {
            let from = summary.n_valid;
            Plan::Extend(summary, from)
        } else {
            Plan::Whole(refused.to_string())
        }
    }

    fn extend_summary(
        &self,
        sidecar: &SummarySidecar,
        mut summary: MonthSummary,
        from: u64,
    ) -> Result<(), String> {
        for row in from..self.header.n_valid {
            summary.fold(&self.read_row::<Bar>(row).map_err(|why| why.to_string())?);
        }
        summary.bind(&self.header);
        if summary.wants_seconds() {
            // DENSE NOW, AND ITS SECONDS WERE NOT KEPT: the month is read
            // whole, O(n_valid), and the log says so. This runs on EVERY
            // commit that takes the month back over the half line, so a
            // month whose density moves back and forth across it pays it on
            // each such commit (the bound in `crate::month_summary`).
            summary = self.whole_summary().map_err(|why| why.to_string())?;
            write_summary(sidecar, &summary)?;
            note_summary_built(&sidecar.path, DENSE_AGAIN, self.header.n_valid);
            return Ok(());
        }
        write_summary(sidecar, &summary)
    }

    /// The month's summary from every committed row, through the verified
    /// read path. `O(n_valid)`.
    fn whole_summary(&self) -> Result<MonthSummary, StoreError> {
        let mut summary = MonthSummary::new(
            self.header.symbol_id,
            self.header.timeframe_secs,
            self.admission.month,
        );
        for row in 0..self.header.n_valid {
            summary.fold(&self.read_row::<Bar>(row)?);
        }
        summary.bind(&self.header);
        Ok(summary)
    }

    /// Builds, or confirms, one month's `.sum` from every committed row and
    /// changes nothing else: the ONE library call that rebuilds a summary
    /// (`store_writes_needed` names the run that walks it over the store).
    ///
    /// Opened as [`Self::sort_index_existing`] opens a month: read-only bars,
    /// the month's lock EXCLUSIVE and non-blocking, no link below the root
    /// followed, the served generation's bar file. A summary that already
    /// describes these bars is [`Summarized::Kept`] and no byte changes; a
    /// month under the threshold with none is [`Summarized::BelowThreshold`].
    ///
    /// # Errors
    ///
    /// Every refusal of the open ([`StoreError::Missing`],
    /// [`StoreError::Locked`], [`StoreError::NotABarPath`], ...), a row the
    /// verified path refuses, and the host's refusal of the write.
    pub fn month_summary_existing(
        root: &Path,
        path: StorePath<'_>,
        symbol_id: u32,
    ) -> Result<Summarized, StoreError> {
        let (mut file, _served) = Self::open_to_sort(root, path, symbol_id)?;
        let sidecar = SummarySidecar {
            path: path_beside(&file.bars_path),
        };
        if file.header.n_valid == 0 {
            return Ok(Summarized::Empty);
        }
        let because = match crate::month_summary::read_beside(
            &file.bars_path,
            &file.header,
            file.admission.month,
        ) {
            Ok(_) => return Ok(Summarized::Kept),
            Err(Refused::BelowThreshold) => return Ok(Summarized::BelowThreshold),
            Err(why) => why.to_string(),
        };
        file.msum = Some(sidecar.clone());
        let summary = file.whole_summary()?;
        write_image(&sidecar, &summary.file_image())?;
        note_summary_built(&sidecar.path, &because, file.header.n_valid);
        Ok(Summarized::Built(because))
    }
}

/// Writes `summary` as the month's `.sum`, by staging and rename.
fn write_summary(sidecar: &SummarySidecar, summary: &MonthSummary) -> Result<(), String> {
    write_image(sidecar, &summary.file_image()).map_err(|why| why.to_string())
}

/// `image` into `<yyyy-mm>.new.sum` (a new inode, never one already at the
/// name) and renamed over `<yyyy-mm>.sum`.
fn write_image(sidecar: &SummarySidecar, image: &[u8]) -> Result<(), StoreError> {
    let staging = sidecar.staging();
    refuse_link(&sidecar.path)?;
    // A NAME HELD BY A DIRECTORY, A PIPE OR A DEVICE is left as it is and the
    // step refused BEFORE a staging file is made: a rename over it would fail
    // after the write and leave the staging name behind.
    if let Ok(meta) = fs::symlink_metadata(&sidecar.path)
        && !meta.is_file()
    {
        return Err(StoreError::Io {
            path: sidecar.path.clone(),
            action: Action::Write,
            kind: std::io::ErrorKind::AlreadyExists,
            code: None,
        });
    }
    let staged = crate::staging::create(&staging).map_err(|why| match why {
        crate::staging::StagingError::Io { source, .. } => {
            classify(&staging, Action::Open, &source)
        }
        // A NAME HELD BY SOMETHING THAT IS NOT A DEAD WRITER'S LEFTOVER: a
        // second name of a file, a link, a directory, a live writer. Nothing
        // was removed, cut or written through it.
        _ => StoreError::Io {
            path: staging.clone(),
            action: Action::Open,
            kind: std::io::ErrorKind::AlreadyExists,
            code: None,
        },
    })?;
    write_fully(&staged.file, &staging, 0, image)?;
    drop(staged);
    fault(
        fs::rename(&staging, &sidecar.path),
        &sidecar.path,
        Action::Write,
    )
}

/// `store.sum`, WARN: a writer could not keep the month's summary in step.
/// The bars are committed; readers refuse the older summary by name and read
/// the bars, and the next writer open tries again.
fn note_summary_lags(path: &Path, why: &str, n_valid: u64) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("store.sum", "month summary lags the bars")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("reason", telemetry::Value::Str(why))
            .with("n_valid", telemetry::Value::Uint(n_valid)),
    );
}

/// Why a month that was sparse was read whole at this commit.
const DENSE_AGAIN: &str = "dense_again: the month holds at least half the seconds it spans \
     again and its seconds were not kept, so it was read whole to keep them (D-4980)";

/// `store.sum`, INFO: a month's summary was written whole (its first, one
/// replaced, one rebuilt by the door, or a month dense again), and why.
fn note_summary_built(path: &Path, why: &str, n_valid: u64) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info("store.sum", "month summary written whole")
            .with("file", telemetry::Value::Str(&path.display().to_string()))
            .with("reason", telemetry::Value::Str(why))
            .with("n_valid", telemetry::Value::Uint(n_valid)),
    );
}
