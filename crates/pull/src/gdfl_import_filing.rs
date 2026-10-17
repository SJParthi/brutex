//! The importer's filing (R3-35, a move-only split of `gdfl_import.rs`):
//! one day's files read and filed on the workers, the begin gate, the
//! provenance records and the day's writes made durable.

use std::path::Path;

use brutex_core::vendor::Vendor;
use store::format::Bar;

use crate::fetch::BarRequest;
use crate::ingest::{Filed as Landed, Ingested, from_rows, repair_rows};
use crate::manifest::Held;
use crate::session::Day;

use super::causes::{ImportRefusal, Stage};
use super::day::{Ctx, Mark, Staged, note_staged};
use super::journal::BAR_DEFINITION;
#[cfg(test)]
use super::tests;
use super::{
    Ask, Converted, DaySource, Durability, FLUSH_THREADS, Handed, ImportKind, Origin, Placement,
    Refused, Report, Run, TARGET, TickFile, convert_noted, day_source, failure_on, lineage, plan,
    quality, second_of,
};

/// The store's symbol id of `symbol`: the cross-check its bar header
/// carries, derived exactly as `ingest` derives it.
pub(super) fn symbol_id_of(symbol: &str) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the store's own fold of the name into its 32-bit cross-check: \
                  the low half of the 64-bit FNV-1a, exactly as `ingest` takes it"
    )]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    id
}

/// The UTC microsecond now; 0 before 1970, which a record refuses by name.
fn now_micros() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_micros()).unwrap_or(i64::MAX)
        })
}

/// The bar file of the month `held` names, in the GDFL tree.
///
/// # Errors
///
/// The store's words for a key that is not a path.
fn month_path(held: &Held) -> Result<store::path::StorePath<'_>, String> {
    let key = &held.entry.key;
    store::path::StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: key.exchange.as_str(),
        segment: key.segment.as_str(),
        symbol: key.symbol.as_str(),
        contract: key.contract,
        timeframe: key.timeframe,
        month: key.month,
        file: store::path::FileKind::Bars,
    })
    .map_err(|why| why.to_string())
}

/// Names the month `held` names to `batch`, so every file it has and every
/// directory down to it is flushed for itself when the batch is made durable
/// (WP-16, D-3353; `store::file::DurableBatch::confirm_month`). `Ok(false)`
/// only for a batch already sealed, which the day's never is while it files.
///
/// # Errors
///
/// The path or the store's refusal, in its words.
fn confirm_month(
    store_root: &Path,
    held: &Held,
    batch: &store::file::DurableBatch,
) -> Result<bool, String> {
    batch
        .confirm_month(store_root, month_path(held)?)
        .map_err(|why| why.to_string())
}

/// Records one landing beside the month it appended to (D-3334): the month
/// `held` names, whose append committed its last `committed` bars. The bar
/// file is opened read-only, never written, and the record is refused unless
/// that file's commit counter is still `held`'s, so a record never names bars
/// another writer appended. `Ok(false)` when nothing was appended.
///
/// # Errors
///
/// The path or the store's refusal, in its words.
fn record_provenance(
    store_root: &Path,
    held: &Held,
    committed: usize,
    landing: &store::provenance::Landing<'_>,
) -> Result<bool, String> {
    let key = held.entry.key;
    let path = month_path(held)?;
    let bars =
        store::file::BarFile::open_existing(store_root, path, symbol_id_of(key.symbol.as_str()))
            .map_err(|why| why.to_string())?;
    let n_valid = held.entry.rows;
    let first_index = u64::try_from(committed)
        .ok()
        .and_then(|committed| n_valid.checked_sub(committed))
        .ok_or_else(|| format!("{committed} bars committed and {n_valid} held"))?;
    let appended = store::file::Appended::Committed {
        first_index,
        n_valid,
    };
    match store::provenance::record(store_root, &bars, path, appended, landing) {
        Ok(store::provenance::Recorded::Written(_)) => Ok(true),
        Ok(store::provenance::Recorded::NothingAppended) => Ok(false),
        Err(why) => Err(why.to_string()),
    }
}

// ═══ One day's files on W workers (WP-16, DB-N25) ═════════════════════════
//
// A day used to file its files one after another on one thread: an options
// day of 5,641 files took 94.5 s on the operator's drive for 36 s of CPU,
// most of the rest waiting on 46,373 flushes (2020-09-15 into a scratch
// store, measured by a counting build). The files of a day now run on
// [`Workers`]: each file is one job (read, verify and decode it; convert,
// fold and file its seconds; record its landing), taken in listing order by
// whichever worker is free, and what each job did comes back as a [`Filed`]
// that the coordinating thread folds into the day IN LISTING ORDER. So the
// report, its failures, the day ledger, the census batch and the lineage's
// contract-days are exactly what one worker builds.
//
// One contract-month per file: an option file is one contract, a stocks file
// one share, an index file one index, and a listing names each at most once
// (a second name is a skip or a refusal, D-2814), so every month file and
// every sidecar beside it is written by exactly one worker.
//
// ONE WRITER OF JOURNAL LINES (the design's seam 2). Workers never append to
// the journal. A day's `begin` is still written before its first write and
// only when the day has a second to write (D-2803): the first worker that
// has one asks the [`BeginGate`], the coordinating thread writes the line and
// answers its number, and every later file of the day reads it from the gate.

/// The day's `begin` line, written by the coordinating thread when the first
/// worker with a second to file asks for it (WP-16).
///
/// An attempt the journal refuses is answered to every worker that asked
/// before it finished, each of which names it as its own file's failure and
/// files nothing, exactly as a single worker did; the next worker that asks
/// makes a new attempt.
#[derive(Debug, Default)]
pub(super) struct BeginGate {
    state: std::sync::Mutex<GateState>,
    changed: std::sync::Condvar,
}

/// What the [`BeginGate`] knows.
#[derive(Debug, Default)]
pub(super) struct GateState {
    /// The `begin` line's number in the journal, once written.
    pub(super) line: Option<u64>,
    /// A worker waits for an attempt.
    asked: bool,
    /// Attempts finished.
    attempts: u64,
    /// The last attempt's refusal.
    pub(super) refused: Option<ImportRefusal>,
    /// Workers that have not finished.
    working: usize,
}

impl BeginGate {
    /// A gate for a day whose `begin` is `line` (written by an earlier batch
    /// of the same day), for `working` workers.
    pub(super) fn new(line: Option<u64>, working: usize) -> Self {
        Self {
            state: std::sync::Mutex::new(GateState {
                line,
                working,
                ..GateState::default()
            }),
            changed: std::sync::Condvar::new(),
        }
    }

    /// The state, whatever a panicking worker left: every field is written
    /// whole under the lock, so a poisoned state is still a correct one.
    pub(super) fn lock(&self) -> std::sync::MutexGuard<'_, GateState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The `begin` line's number, asked by a worker with a second to file.
    ///
    /// # Errors
    ///
    /// The journal's refusal of the attempt this call waited for.
    pub(super) fn line(&self) -> Result<u64, ImportRefusal> {
        let mut state = self.lock();
        if let Some(line) = state.line {
            return Ok(line);
        }
        let asked_at = state.attempts;
        state.asked = true;
        self.changed.notify_all();
        loop {
            if let Some(line) = state.line {
                return Ok(line);
            }
            if state.attempts > asked_at {
                return Err(state
                    .refused
                    .clone()
                    .unwrap_or_else(|| ImportRefusal::Journal {
                        why: "the day's begin line was not written".to_owned(),
                    }));
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// The coordinating thread's side: writes `begin` through `write` each
    /// time a worker asks, until every worker has finished.
    fn serve(&self, write: &mut dyn FnMut() -> Result<u64, ImportRefusal>) {
        let mut state = self.lock();
        loop {
            if state.asked {
                state.asked = false;
                // Written under the lock: a worker that needs the line waits
                // for it anyway, and no other worker touches the gate.
                let written = write();
                state.attempts = state.attempts.saturating_add(1);
                match written {
                    Ok(line) => state.line = Some(line),
                    Err(why) => state.refused = Some(why),
                }
                self.changed.notify_all();
                continue;
            }
            if state.working == 0 {
                return;
            }
            state = self
                .changed
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// `count` workers finished, or will never start.
    fn finished(&self, count: usize) {
        let mut state = self.lock();
        state.working = state.working.saturating_sub(count);
        self.changed.notify_all();
    }

    /// The line, if it was written.
    fn written(&self) -> Option<u64> {
        self.lock().line
    }
}

/// Tells the gate a worker finished when it drops, so a worker that panics
/// still lets the coordinating thread stop serving and join it.
struct Finishing<'a>(&'a BeginGate);

impl Drop for Finishing<'_> {
    fn drop(&mut self) {
        self.0.finished(1);
    }
}

/// Runs `job` over every one of `jobs` on up to `workers` threads, the
/// coordinating thread serving the [`BeginGate`] meanwhile, and answers each
/// job's result at its index. `Err` when no thread could be started, with
/// why; a worker that panics is a panic of the run, resumed here.
pub(super) fn on_workers<J: Sync, T: Send>(
    workers: usize,
    jobs: &[J],
    gate: &BeginGate,
    write_begin: &mut dyn FnMut() -> Result<u64, ImportRefusal>,
    job: &(dyn Fn(&J) -> T + Sync),
) -> Result<Vec<T>, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let want = workers.min(jobs.len()).max(1);
    let next = AtomicUsize::new(0);
    let mut slots: Vec<Option<T>> = Vec::with_capacity(jobs.len());
    slots.resize_with(jobs.len(), || None);
    let refused = std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(want);
        let mut refused = None;
        for at in 0..want {
            let spawned = std::thread::Builder::new()
                .name(format!("gdfl-file-{at}"))
                .spawn_scoped(scope, || {
                    let _finishing = Finishing(gate);
                    let mut done = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(one) = jobs.get(index) else {
                            break;
                        };
                        done.push((index, job(one)));
                    }
                    done
                });
            match spawned {
                Ok(handle) => handles.push(handle),
                Err(why) => {
                    refused = Some(why.to_string());
                    break;
                }
            }
        }
        // A worker that never started never finishes: the gate is told now.
        gate.finished(want - handles.len());
        if handles.is_empty() {
            return refused.or_else(|| Some("no worker started".to_owned()));
        }
        if let Some(why) = refused.as_deref() {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn(TARGET, "fewer workers than asked for")
                    .with("asked", want)
                    .with("started", handles.len())
                    .with("why", why),
            );
        }
        gate.serve(write_begin);
        for handle in handles {
            match handle.join() {
                Ok(done) => {
                    for (index, result) in done {
                        if let Some(slot) = slots.get_mut(index) {
                            *slot = Some(result);
                        }
                    }
                }
                Err(panicked) => std::panic::resume_unwind(panicked),
            }
        }
        None
    });
    match refused {
        Some(why) => Err(why),
        None => Ok(slots.into_iter().flatten().collect()),
    }
}

/// What every worker of a day shares while it files (WP-16): read-only.
pub(super) struct Filing<'a> {
    pub(super) run: &'a Run<'a>,
    pub(super) day: Day,
    pub(super) request: &'a BarRequest,
    /// The day's session, when it keeps a day ledger (D-3335).
    pub(super) session: Option<&'a crate::calendar::Session>,
    /// The run's measured origin (D-3317 item 4).
    pub(super) origin: Option<&'a Origin>,
    /// Where the day's files came from (D-3334), or why it is not known.
    pub(super) source: Option<&'a Result<DaySource, String>>,
    /// The day's batch, when an earlier run left the day open (WP-16,
    /// D-3353): every month a file of the day is filed in is confirmed in
    /// it, so what that run wrote and never made durable is flushed for
    /// itself before this run closes the day.
    pub(super) confirm: Option<&'a store::file::DurableBatch>,
    /// The run's id, which a back-fill's reason and event name (D-3350).
    pub(super) id: u64,
}

/// What a file's landing left beside the bars it committed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Receipt {
    /// Nothing: no bar committed, or a run that names no origin.
    #[default]
    None,
    /// The provenance record of its append (D-3334): counted in
    /// [`Report::provenance_records`].
    Provenance,
    /// A new generation of its month (D-3350; DB-N26, WP-12d), journaled in
    /// `<store>/audit/gdfl.repairs`: counted in [`Report::months_repaired`].
    /// A revision carries no provenance record.
    NewGeneration,
}

/// One file as a worker filed it, folded into its day in listing order by
/// [`DayFiler`] (WP-16): every count the file adds and every failure it
/// named, in the order it named them.
#[derive(Debug, Default)]
pub(super) struct Filed {
    /// The fold refused the file: counted refused, and never also read
    /// (D-2814).
    pub(super) refused: bool,
    /// What its placement counted, for a file the fold took.
    pub(super) placement: Option<Placement>,
    /// Its quality (D-3335) and the symbol it is filed under.
    pub(super) quality: Option<(String, quality::FileQuality)>,
    /// Bars the venue's session declined.
    pub(super) outside_session: usize,
    /// Bars offered to the store.
    pub(super) seconds: usize,
    /// Bars the store wrote.
    pub(super) committed: usize,
    /// What its landing left beside its bars.
    receipt: Receipt,
    /// Whether its month was confirmed in the day's batch (D-3353): the day
    /// had been left open by an earlier run.
    confirmed: bool,
    /// The census row of the month it filed.
    pub(super) held: Option<Held>,
    /// Its failures, in the order they arose.
    pub(super) failures: Vec<Staged>,
}

/// A file's quality (D-3335), from its rows, the bars it built and what its
/// placement counted.
pub(super) fn measure(
    kind: ImportKind,
    (day, session): (Day, &crate::calendar::Session),
    handed: &Handed,
    converted: &Converted,
    deferred: quality::Deferred,
) -> quality::FileQuality {
    let seconds: Vec<u32> = converted
        .seconds
        .iter()
        .map(|bar| second_of(bar.ts_micros, day))
        .collect();
    quality::measure_file(
        kind,
        (session, quality::halt_window(kind, day)),
        &handed.file.ticks,
        &seconds,
        (converted.placement, deferred),
        handed.facts.reader,
    )
}

/// A file's quality (D-3335), folded into `ledger`.
pub(super) fn take_quality(
    kind: ImportKind,
    ledger: &mut quality::DayLedger,
    handed: &Handed,
    converted: &Converted,
    deferred: quality::Deferred,
) {
    let measured = measure(
        kind,
        (ledger.day(), ledger.session()),
        handed,
        converted,
        deferred,
    );
    ledger.take(&handed.file.symbol, &measured);
}

/// The store's door a file is filed through (WP-16 with WP-12d).
///
/// A day's files run on workers inside the day's batch, where no writer
/// flushes for itself (D-3353). The versioned month repair (D-3350) is not
/// such a writer: its own flushes order a revision's bars before its receipt
/// and its promotion, it journals to one ledger, `<store>/audit/gdfl.repairs`,
/// and it costs O(month). So a worker files through the append door alone,
/// and a file that door refuses outright is filed again through the
/// back-fill door by the coordinating thread, in listing order and outside
/// the batch: the repair keeps every flush it was written with, and the
/// repair journal's lines are in the order one worker writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Door {
    /// `ingest::from_rows`: a tail, a new month, a day already held.
    Append,
    /// `ingest::repair_rows`: the append door again, then the repair for a
    /// day its month lacks where no append can land it.
    BackFill,
}

impl Filing<'_> {
    /// Converts and files one instrument file, measures its quality and
    /// records its landing: what one file of a day does, on whichever worker
    /// took it. `begin` answers the day's `begin` line, written the first time
    /// a file has a second to file.
    ///
    /// The flag is `true` only through [`Door::Append`], when the append door
    /// refused the file and committed nothing: the answer is then not the
    /// file's, nothing of it is to be counted, and the caller files it again
    /// through [`Door::BackFill`], whose answer is always the file's.
    pub(super) fn file(
        &self,
        handed: &Handed,
        begin: &dyn Fn() -> Result<u64, ImportRefusal>,
        door: Door,
    ) -> (Filed, bool) {
        let run = self.run;
        let file = &handed.file;
        let mut filed = Filed::default();
        // A file the fold refuses is counted refused and never also read
        // (D-2814): files, skips and refusals add up to the day's entries.
        let (converted, deferred) = match convert_noted(run.kind, self.day, &file.ticks) {
            Ok(two) => two,
            Err(why) => {
                filed.refused = true;
                filed
                    .failures
                    .push(note_staged(run, self.day, &file.name, Stage::File, &why));
                return (filed, false);
            }
        };
        if let Some(session) = self.session {
            let measured = measure(run.kind, (self.day, session), handed, &converted, deferred);
            filed.quality = Some((file.symbol.clone(), measured));
        }
        let p = converted.placement;
        filed.placement = Some(p);
        if converted.seconds.is_empty() {
            return (filed, false);
        }
        let begin_line = match begin() {
            Ok(line) => line,
            Err(why) => {
                filed
                    .failures
                    .push(note_staged(run, self.day, "journal", Stage::Journal, &why));
                return (filed, false);
            }
        };
        let origin = format!("gdfl {} {} {}", run.kind.as_str(), self.day, file.name);
        // A DAY ITS MONTH LACKS, WHERE NO APPEND CAN LAND IT, LANDS AS A NEW
        // GENERATION OF THE MONTH (D-3350; DB-N26). The append-only month
        // refuses it, and was the end of it: the 12 index days WP-12a
        // admitted were refused for every NIFTY and BANKNIFTY file on 10 Oct
        // 2026. The back-fill door tries the append door first, so a tail, a
        // new month and a day already held land exactly as before; it
        // repairs only a month that holds no bar inside the file's span and
        // cannot be appended to, never in place, and every other refusal
        // stands as it was. The reason names the run, whose `run` line names
        // the archive.
        let Some((done, landed)) = self.through(door, file, &converted.seconds, &origin) else {
            return (filed, true);
        };
        filed.outside_session = usize::try_from(done.census.total()).unwrap_or(0);
        filed.seconds = done.bars_stored;
        filed.committed = done.bars_committed;
        if done.bars_committed > 0
            && let Some(held) = done.pending.as_ref()
        {
            match landed {
                Landed::Appended => {
                    match self.provenance(handed, held, done.bars_committed, begin_line) {
                        Ok(true) => filed.receipt = Receipt::Provenance,
                        Ok(false) => {}
                        Err(refused) => filed.failures.push(refused),
                    }
                }
                Landed::NewGeneration => {
                    filed.receipt = Receipt::NewGeneration;
                    self.repaired(handed, held, done.bars_committed);
                }
            }
        }
        // A DAY AN EARLIER RUN LEFT OPEN: whatever that run wrote of this
        // month was never made durable, and found present it is not written
        // again, so it is named to the day's batch, which flushes the month's
        // files and directories for themselves (D-3353). A month that cannot
        // be named keeps the day open, by name.
        if let (Some(batch), Some(held)) = (self.confirm, done.pending.as_ref()) {
            match confirm_month(run.store_root, held, batch) {
                Ok(confirmed) => filed.confirmed = confirmed,
                Err(why) => {
                    let refusal = ImportRefusal::Filing { why };
                    filed.failures.push(note_staged(
                        run,
                        self.day,
                        &file.name,
                        Stage::File,
                        &refusal,
                    ));
                }
            }
        }
        filed.held = done.pending;
        for failure in done.failures {
            let refusal = ImportRefusal::Filing { why: failure.why };
            filed.failures.push(note_staged(
                run,
                self.day,
                &file.name,
                Stage::File,
                &refusal,
            ));
        }
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::debug(TARGET, "filed")
                .with("file", file.name.as_str())
                .with("rows", p.rows)
                .with("seconds", converted.seconds.len()),
        );
        (filed, false)
    }

    /// Offers a file's bars to the store through `door`: what the store
    /// answered and how the bars landed, or `None` when [`Door::Append`]
    /// refused them and committed nothing.
    fn through(
        &self,
        door: Door,
        file: &TickFile,
        seconds: &[Bar],
        origin: &str,
    ) -> Option<(Ingested, Landed)> {
        let run = self.run;
        let month = || plan(run.kind, self.request, file.contract);
        match door {
            Door::Append => {
                let done = from_rows(seconds, &[], &file.symbol, origin, run.store_root, month());
                (done.bars_committed > 0 || done.failures.is_empty())
                    .then_some((done, Landed::Appended))
            }
            Door::BackFill => {
                let reason = format!(
                    "gdfl-import {} {} {} run:{}: a day the month lacks where no append can land it",
                    run.kind.as_str(),
                    self.day,
                    file.name,
                    self.id
                );
                // OUTSIDE THE DAY'S BATCH, whichever thread this is: the
                // repair's flushes are its own (D-3350), and so are those of
                // an append the door takes this second time.
                let _outside_the_batch = store::file::DurableBatch::suspend();
                Some(repair_rows(
                    seconds,
                    &file.symbol,
                    origin,
                    run.store_root,
                    month(),
                    &reason,
                ))
            }
        }
    }

    /// Writes the provenance record of a file's landing (D-3334) when the
    /// run names its origin and the source named the file's member, and says
    /// whether it wrote one. A record the sidecar refuses is a named failure
    /// of the day, never silence: the bars stay committed and stay
    /// unrecorded, which `/provenance.json` labels reconstructed.
    fn provenance(
        &self,
        handed: &Handed,
        held: &Held,
        committed: usize,
        begin_line: u64,
    ) -> Result<bool, Staged> {
        let run = self.run;
        let (Some(origin), Some(member), Some(source)) =
            (self.origin, handed.facts.member.as_ref(), self.source)
        else {
            return Ok(false);
        };
        let recorded = source.clone().and_then(|source| {
            let (kind, name, bytes, crc32) = match source {
                DaySource::Members { prefix } => (
                    store::provenance::SourceKind::ArchiveMember,
                    format!("{prefix}{}", member.entry),
                    member.bytes,
                    member.crc32,
                ),
                DaySource::TickstoreDay { name, bytes } => {
                    (store::provenance::SourceKind::TickstoreDay, name, bytes, 0)
                }
            };
            let day =
                store::provenance::Day::new(self.day.year(), self.day.month(), self.day.day())
                    .ok_or_else(|| format!("{} is not a provenance day", self.day))?;
            let landing = store::provenance::Landing {
                source: store::provenance::Source {
                    kind,
                    name: &name,
                    bytes,
                    crc32,
                },
                day,
                bar_definition: BAR_DEFINITION,
                journal_ordinal: begin_line,
                commit: origin.commit.as_deref(),
                landed_at_micros: now_micros(),
            };
            record_provenance(run.store_root, held, committed, &landing)
        });
        recorded.map_err(|why| {
            let refusal = ImportRefusal::Provenance { why };
            note_staged(run, self.day, &handed.file.name, Stage::File, &refusal)
        })
    }

    /// A file whose day its month lacked inside what it holds, landed as a
    /// NEW GENERATION of the month (D-3350; DB-N26): logged with the month's
    /// rows now served, and counted in [`Report::months_repaired`] when the
    /// file is folded into its day. The repair door journaled it in `<store>/audit/gdfl.repairs`
    /// with this run's id.
    ///
    /// **No `.prv` record is written, and the event says so.** A sidecar
    /// record covers the bars ONE append committed at the end of its file,
    /// at that file's commit generation (`store::provenance::record`); the
    /// added bars sit inside the new generation, and its records would index
    /// bars the original never held, so a revision carries no `.prv`
    /// (`crates/store/REPAIR.md`). `/provenance.json` reads beside the
    /// generation every reader opens and rebuilds the month's answer from
    /// this journal's day lines, labelled reconstructed; the original's
    /// `.prv`, if any, is left byte-identical. When the run names its origin
    /// — so an append would have been recorded — the event is a `warn`.
    fn repaired(&self, handed: &Handed, held: &Held, added: usize) {
        let unrecorded =
            self.origin.is_some() && handed.facts.member.is_some() && self.source.is_some();
        let (day, month) = (self.day.to_string(), held.entry.key.month.to_string());
        let event = if unrecorded {
            telemetry::Event::warn(
                TARGET,
                "day back-filled as a new generation of its month; no .prv record: a revision \
                 carries none (D-3350), the repair journal and this journal's day line name it",
            )
        } else {
            telemetry::Event::info(TARGET, "day back-filled as a new generation of its month")
        };
        let _dropped_when_filtered = telemetry::emit(
            &event
                .with("file", handed.file.name.as_str())
                .with("day", day.as_str())
                .with("month", month.as_str())
                .with("added", added)
                .with("rows", held.entry.rows)
                .with("run", self.id),
        );
    }
}

/// What reading one file of a day came to, answered to its source in listing
/// order (WP-16): read and filed, with the contract-day the lineage folds
/// when the source asked for it, or refused by the reader.
#[derive(Debug)]
pub(super) enum JobRead {
    /// Read; filed by [`Filing::file`].
    Read(Option<lineage::DayContract>),
    /// The source or the reader refused it.
    Refused(Refused),
}

/// What a worker answers for one job.
#[derive(Debug)]
enum JobDone {
    /// Read and filed.
    Filed(Box<Filed>, Option<lineage::DayContract>),
    /// Read, and refused outright by the append door: the coordinating
    /// thread files it through the back-fill door ([`Door::BackFill`]).
    BackFill(Box<Handed>, Option<lineage::DayContract>),
    /// The source or the reader refused it.
    Refused(Refused),
}

/// One file read by a source for a day's job: its rows and facts, and the
/// contract-day the lineage folds, when asked.
pub(super) type ReadJob = Result<(Handed, Option<lineage::DayContract>), Refused>;

/// One job of a day once every file is filed: the file as it was filed and
/// the contract-day the lineage folds, or the read's refusal.
type FiledJob = Result<(Box<Filed>, Option<lineage::DayContract>), Refused>;

/// THE BACK-FILL PASS (D-3350 under WP-16): every file the append door refused
/// outright, filed through the back-fill door ([`Door::BackFill`]), one after
/// the other in listing order on the calling thread. Each had asked for the
/// day's `begin` line before it reached a door, so `begun` holds it.
fn back_fill(filing: &Filing<'_>, begun: Option<u64>, done: Vec<JobDone>) -> Vec<FiledJob> {
    let begin_again = || {
        begun.ok_or_else(|| ImportRefusal::Journal {
            why: "the day's begin line was not written".to_owned(),
        })
    };
    done.into_iter()
        .map(|one| match one {
            JobDone::Filed(filed, contract) => Ok((filed, contract)),
            JobDone::BackFill(handed, contract) => {
                let (filed, _never_through_this_door) =
                    filing.file(&handed, &begin_again, Door::BackFill);
                Ok((Box::new(filed), contract))
            }
            JobDone::Refused(refused) => Err(refused),
        })
        .collect()
}

/// One admitted day's filing (WP-16): the source hands it the day's wanted
/// files, it files them on the run's workers, and it folds what each file
/// did into the day in listing order.
pub(super) struct DayFiler<'a> {
    pub(super) ctx: &'a Ctx<'a>,
    pub(super) key: &'a str,
    pub(super) day: Day,
    pub(super) request: &'a BarRequest,
    /// Where the day's files came from (D-3334), when the run names an
    /// origin.
    pub(super) source: Option<Result<DaySource, String>>,
    pub(super) workers: usize,
    /// The day's `begin` line, once written.
    begin: Option<u64>,
    pub(super) report: &'a mut Report,
    pub(super) held: Vec<Held>,
    pub(super) failures: Vec<Staged>,
    /// The day's largest back-step, for its journal line.
    pub(super) max_back_s: u32,
    /// The day's quality fields (D-3335), when the day has a session.
    pub(super) ledger: Option<quality::DayLedger>,
    /// The day's batch (WP-16, D-3353), which `drive_core` makes durable
    /// once the last file is filed: the one every write of the day's files
    /// is left to when the run makes a day durable together, and the one the
    /// day's months are confirmed in when an earlier run left the day open.
    pub(super) batch: Option<store::file::DurableBatch>,
    /// Whether each job enters the batch on the thread that runs it, so its
    /// writers leave their flushes to it ([`Durability::Day`]). A run that
    /// flushes each write as it is written has a batch only to confirm in.
    enters: bool,
    /// Whether an earlier run left the day open ([`Journal::left_open`]):
    /// every month a file of the day is filed in is confirmed in the batch.
    pub(super) confirm: bool,
    /// The months confirmed so far.
    pub(super) confirmed: usize,
    /// Which files the day's reader is asked for (D-4878).
    pub(super) ask: Ask,
}

impl<'a> DayFiler<'a> {
    /// The filer of one admitted day: its quality ledger when the day has a
    /// session, where its files came from when the run names an origin, and
    /// its batch.
    ///
    /// A DAY AN EARLIER RUN BEGAN AND NEVER CLOSED was cut off after it had
    /// written some of the day's months and before it made them durable.
    /// Read again they are found present and nothing is written, so this run
    /// confirms each month the day files in the day's batch (D-3353),
    /// whatever its own durability: a run that flushes each write as it is
    /// written takes a batch for the confirmations alone.
    pub(super) fn of_day(
        ctx: &'a Ctx<'a>,
        (key, day): (&'a str, Day),
        request: &'a BarRequest,
        report: &'a mut Report,
    ) -> Self {
        let (kind, options) = (ctx.run.kind, ctx.options);
        let left_open = ctx.journal.left_open(kind, day);
        Self {
            ctx,
            key,
            day,
            request,
            source: options.origin.map(|origin| day_source(kind, day, origin)),
            workers: options.workers.get(),
            begin: None,
            report,
            held: Vec::new(),
            failures: Vec::new(),
            max_back_s: 0,
            ledger: crate::gdfl_cm::session_of(day)
                .ok()
                .map(|session| quality::DayLedger::new(kind, day, session)),
            batch: options.durability.batch(left_open),
            enters: options.durability != Durability::EachWrite,
            confirm: left_open,
            confirmed: 0,
            ask: Ask::All,
        }
    }
}

impl DayFiler<'_> {
    /// Reads every one of `jobs` through `read` and files what it reads, on
    /// the run's workers; folds each file into the day in `jobs` order and
    /// answers how each read went, in that order.
    ///
    /// With one worker, or one job, nothing is started: the jobs run on this
    /// thread, as before WP-16. A run that cannot start a single worker
    /// thread says so at `warn` and runs the day here too, never silently.
    pub(super) fn file_all<J: Sync>(
        &mut self,
        jobs: &[J],
        read: &(dyn Fn(&J) -> ReadJob + Sync),
    ) -> Vec<JobRead> {
        let session = self.ledger.as_ref().map(|ledger| *ledger.session());
        let batch = self.batch.as_ref().filter(|_| self.enters);
        let filing = Filing {
            run: self.ctx.run,
            day: self.day,
            request: self.request,
            session: session.as_ref(),
            origin: self.ctx.options.origin,
            source: self.source.as_ref(),
            confirm: self.batch.as_ref().filter(|_| self.confirm),
            id: self.ctx.id,
        };
        let mut done: Option<Vec<JobDone>> = None;
        if self.workers > 1 && jobs.len() > 1 {
            let gate = BeginGate::new(self.begin, self.workers.min(jobs.len()));
            let (ctx, key) = (self.ctx, self.key);
            let threaded = on_workers(
                self.workers,
                jobs,
                &gate,
                &mut || ctx.append(&format!("begin {key}")),
                &|job| {
                    let _in_the_day = batch.map(store::file::DurableBatch::enter);
                    match read(job) {
                        Ok((handed, contract)) => {
                            match filing.file(&handed, &|| gate.line(), Door::Append) {
                                (_, true) => JobDone::BackFill(Box::new(handed), contract),
                                (filed, false) => JobDone::Filed(Box::new(filed), contract),
                            }
                        }
                        Err(refused) => JobDone::Refused(refused),
                    }
                },
            );
            self.begin = gate.written();
            match threaded {
                Ok(all) => done = Some(all),
                Err(why) => {
                    let day_text = self.day.to_string();
                    let _dropped_when_filtered = telemetry::emit(
                        &telemetry::Event::warn(
                            TARGET,
                            "no worker thread started; the day's files run on one thread",
                        )
                        .with("day", day_text.as_str())
                        .with("why", why.as_str()),
                    );
                }
            }
        }
        let done = done.unwrap_or_else(|| {
            let line = std::cell::Cell::new(self.begin);
            let (ctx, key) = (self.ctx, self.key);
            let begin = || {
                if let Some(written) = line.get() {
                    return Ok(written);
                }
                let written = ctx.append(&format!("begin {key}"))?;
                line.set(Some(written));
                Ok(written)
            };
            let _in_the_day = batch.map(store::file::DurableBatch::enter);
            let all = jobs
                .iter()
                .map(|job| match read(job) {
                    Ok((handed, contract)) => match filing.file(&handed, &begin, Door::Append) {
                        (_, true) => JobDone::BackFill(Box::new(handed), contract),
                        (filed, false) => JobDone::Filed(Box::new(filed), contract),
                    },
                    Err(refused) => JobDone::Refused(refused),
                })
                .collect();
            self.begin = line.get();
            all
        });
        let done = back_fill(&filing, self.begin, done);
        done.into_iter()
            .map(|one| match one {
                Ok((filed, contract)) => {
                    self.fold(*filed);
                    JobRead::Read(contract)
                }
                Err(refused) => JobRead::Refused(refused),
            })
            .collect()
    }

    /// Folds one filed file into the day: exactly what filing it did to the
    /// report, the ledger, the census batch and the failures when the day ran
    /// on one thread.
    pub(super) fn fold(&mut self, filed: Filed) {
        if filed.refused {
            self.report.files_refused += 1;
        } else {
            self.report.files += 1;
        }
        if let (Some(ledger), Some((symbol, quality))) = (self.ledger.as_mut(), &filed.quality) {
            ledger.take(symbol, quality);
        }
        if let Some(p) = filed.placement {
            self.report.rows += p.rows;
            self.report.ltq_zero_dropped += p.ltq_zero_dropped;
            self.report.late_rows += p.late_rows;
            self.report.late_unresolved += p.late_unresolved;
            self.report.max_back_s = self.report.max_back_s.max(p.max_back_s);
            self.max_back_s = self.max_back_s.max(p.max_back_s);
        }
        self.report.outside_session += filed.outside_session;
        self.report.seconds += filed.seconds;
        self.report.seconds_committed += filed.committed;
        match filed.receipt {
            Receipt::None => {}
            Receipt::Provenance => self.report.provenance_records += 1,
            Receipt::NewGeneration => self.report.months_repaired += 1,
        }
        self.confirmed += usize::from(filed.confirmed);
        self.held.extend(filed.held);
        self.failures.extend(filed.failures);
    }
}

/// Makes the day's batch durable (WP-16, D-3353), on [`FLUSH_THREADS`], and
/// says so at `info`, once a day beside "day imported": what it held, the
/// flushes it took and how long they took, so the cost of a day's durability
/// is in the log the operator reads.
pub(super) fn make_durable(
    batch: &store::file::DurableBatch,
    (kind, day): (ImportKind, Day),
    left_open: Option<usize>,
) -> Result<(), store::file::StoreError> {
    let started = std::time::Instant::now();
    let made = batch.make_durable_by(FLUSH_THREADS, &|file, path| {
        #[cfg(test)]
        tests::before_a_day_flush(path)?;
        let _ = path;
        file.sync_all()
    })?;
    let day_text = day.to_string();
    let ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    // A DAY AN EARLIER RUN LEFT OPEN SAYS SO, at `warn`: what that run wrote
    // and never made durable was flushed for itself here, and how much.
    if let Some(months) = left_open {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(
                TARGET,
                "an earlier run left this day open; its months were confirmed durable",
            )
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("months", months)
            .with("files", made.confirmed),
        );
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "day made durable")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("files", made.files)
            .with("directories", made.directories)
            .with("confirmed", made.confirmed)
            .with("flushes", made.flushes)
            .with("ms", ms),
    );
    Ok(())
}

/// A day whose writes could not be made durable (WP-16, D-3353): named in
/// the report, said at `error`, and the run stopped there. The day is left
/// begun and not closed, so the next run reads it again; nothing of its
/// ledger, census or closing line is written over writes that are not
/// durable.
pub(super) fn stop_at_durability(
    ctx: &Ctx<'_>,
    day: Day,
    (before, mut report): (Mark, Report),
    failures: Vec<Staged>,
    why: &store::file::StoreError,
) -> ImportRefusal {
    let kind = ctx.run.kind;
    let words = format!(
        "the day's writes could not be made durable, so nothing of its ledger, census or \
         closing line was written and the day stays open: {why}"
    );
    let day_text = day.to_string();
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "day not made durable; run stopped")
            .with("kind", kind.as_str())
            .with("day", day_text.as_str())
            .with("why", words.as_str())
            .with("days_imported", report.days_imported)
            .with("files", report.files - before.files),
    );
    report
        .failures
        .extend(failures.into_iter().map(|one| one.failure));
    report
        .failures
        .push(failure_on(day, &format!("{day} durability"), words.clone()));
    ImportRefusal::Durability {
        kind,
        day,
        why: words,
        done: Box::new(report),
    }
}
