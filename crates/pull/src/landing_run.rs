//! Every Zerodha landing's run: its id from the store-wide runs ledger, the
//! vendor's landing claim, and the keep-and-spool hook both landing doors
//! hand each window to (IA-10c; D-4859, D-4880, D-4881, D-4888, D-6260 to
//! D-6267; made its own part by the design's critic resolution IA-10b-1,
//! which the ledger holds no entry for).
//!
//! # Why this exists
//!
//! IA-8 built the keeper (`crate::response_keep`) and IA-9 the spool and the
//! verifier (`crate::zerodha_verify`), and nothing on the pull path called
//! either: a Zerodha answer was decoded, landed and dropped, so IA-9's cold
//! verification read every minute `no_response_kept`. And a landing had no
//! durable id: its gap and repair lines named the telemetry sink's claimed
//! run, which two overlapping requests of one api share, and which restarts
//! with the process.
//!
//! # What a landing does here
//!
//! 1. **An id** ([`allocate`]): one runs-ledger record per landing request,
//!    mode `pull` (a `/pull/run`, at its claim; its legs carry it),
//!    `pull-spot`, `pull-fno`, `pull-repair` (each request from its route,
//!    after its seat) or `zerodha-repair` (`cli zerodha-repair --from-store`,
//!    before it opens the store). It goes into `ingest::Plan::run`, so every
//!    gap, derive and repair-journal line says it; into the receipt (offset
//!    60); and names the spool directory.
//! 2. **The vendor's claim** ([`claim`]): `imports/<vendor>.landing.lock`, held
//!    SHARED by every api landing request and EXCLUSIVE by `cli
//!    zerodha-repair --from-store`, so the two never land one vendor's months
//!    at once from two processes (FM-163). The holder writes who it is into
//!    the lock file, and a refusal names it.
//! 3. **The hook** ([`RunHook::after_landing`]), called after EITHER door
//!    returns (`ingest::from_window_with_refusals`,
//!    `ingest::repair_window_with_refusals`), in the order land, keep, spool
//!    (D-4859): the answer is kept with a receipt whose rows-refused field is
//!    the landing's own named refusals and whose door byte says which door;
//!    then, for Zerodha, the window's days are verified and their records
//!    appended to the run's spool. A crash between the landing and the keep
//!    leaves bars with no kept body, which the next verification reads
//!    `no_response_kept`: never a receipt for rows that never landed.
//!
//! # Cost
//!
//! [`allocate`]: the runs ledger's O(1) (one or two 128-byte reads, one write
//! and one sync of the id's record, then one write and one sync of its claims
//! line, D-6269; C-GR-08). [`claim`]: one open and one `flock` try per
//! request, and two whole-width notes (at the claim and with its run id,
//! D-6268). [`RunHook::after_landing`]: the keep's O(body) (C-RK-02), one
//! incremental read of the gap ledger's new lines, and the window's
//! verification, O(minutes of the window × rungs) (L-IA-18, C-ZV-01). The
//! first window a process verifies reads the gap ledger whole, once per
//! process (L-IA-20, L-IA10C-03). Cited from ONE measurement, dated:
//! 24 Oct 2026 15:53 IST on WD_BLACK, `tests::ia10c_timings` on the loaded
//! shared machine (`docs/06-limits.md`, L-IA10C-09): an id p99 57.6 ms (two
//! syncs), the claim taken shared and noted p99 65.5 µs, the hook after one
//! landed day p99 16.4 ms.

use std::collections::{BTreeMap, HashMap};
use std::fs::File;
use std::os::unix::fs::FileExt as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use brutex_core::instrument::{Contract, InstrumentKey};
use brutex_core::vendor::Vendor;
use store::flock::Flock;
use store::path::Timeframe;

use crate::fetch::RawRow;
use crate::gdfl_run::ledger::{self, RunsLedgerFault};
use crate::gdfl_run::locks::{self, Hold, LockFault};
use crate::response_keep::{self, Answered, Door, KeepError, Outcome, Pack};
use crate::session::Day;
use crate::vendor::{Granularity, TimestampEncoding};
use crate::zerodha_verify::spool::{Appended, SpoolWriter};
use crate::zerodha_verify::{self, GapLedger, Series};

use super::DoorLanding;

/// The telemetry target every event of this module carries.
pub const TARGET: &str = "pull.landing_run";

/// What a landing allocates its id as (design §5.3, round 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A `/pull/run`, at its claim; its legs carry its id.
    Run,
    /// A `/pull/spot` request (and the autopilot's and recovery's spot runs,
    /// which land through the same broker run).
    Spot,
    /// A `/pull/fno` request.
    Fno,
    /// A `/pull/repair` request.
    Repair,
    /// `cli zerodha-repair --from-store`.
    StoreRepair,
}

impl Mode {
    /// The runs ledger's word for it (`ledger::MODES`).
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Run => "pull",
            Self::Spot => "pull-spot",
            Self::Fno => "pull-fno",
            Self::Repair => "pull-repair",
            Self::StoreRepair => "zerodha-repair",
        }
    }
}

/// Who allocates (`ledger::BY`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum By {
    /// The api.
    Api,
    /// A cli verb.
    Cli,
}

impl By {
    /// The runs ledger's word for it.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Api => "api",
            Self::Cli => "cli",
        }
    }
}

/// The runs ledger's kind for a landing of `vendor`: `zerodha`, or `-` for a
/// vendor the ledger has no word for (it holds GDFL's kinds and Zerodha's).
#[must_use]
pub const fn kind_of(vendor: Vendor) -> &'static str {
    match vendor {
        Vendor::Zerodha => "zerodha",
        _ => "-",
    }
}

/// The clock, UNIX seconds; `0` for a clock before the epoch, which the
/// runs ledger then refuses as `clock_implausible`, never a guessed time.
fn now_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// Takes one runs-ledger id for a landing of `mode` by `by` (D-4880,
/// D-6260), of the ledger's `kind` (`zerodha`, or `-` for a run of no one
/// vendor the ledger names; see [`kind_of`]): never given twice, whatever the
/// clock does, across a restart, and after the ledger's last record rotted,
/// was lost whole or the ledger was put back without it, while the claims'
/// line of that id survives (the ledger writes every id down twice, its
/// record and a CRC-checked claims line, reads the claims' tail on every
/// allocation and raises past them, D-6269; a ledger and claims that both
/// lost the same tail are `docs/06-limits.md` L-IA10C-10), so a receipt, a
/// spool and a ledger line name one landing for ever. Blocks for two syncs
/// (the record and its claims line): the api calls it off its shared
/// workers.
///
/// # Errors
///
/// Every [`RunsLedgerFault`]: the landing does not start, and its refusal
/// names the code (`runs_ledger_damaged`, `clock_implausible`,
/// `lock_stalled`, `runs_ledger_entry` for a kind the ledger has no word
/// for or an id past what a receipt's run field holds, ...). No landing runs
/// without an id, and none under an id its receipt could not carry.
pub fn allocate(store: &Path, mode: Mode, by: By, kind: &str) -> Result<u64, RunsLedgerFault> {
    allocate_at(store, mode, by, kind, now_s())
}

/// [`allocate`] under the clock `now`, UNIX seconds: the seam a test sets
/// the clock with, to the very second the ledger's last id was given or
/// behind it, where an id could only be told apart by what the ledger
/// remembers.
///
/// # Errors
///
/// What [`allocate`] refuses.
pub(crate) fn allocate_at(
    store: &Path,
    mode: Mode,
    by: By,
    kind: &str,
    now: u64,
) -> Result<u64, RunsLedgerFault> {
    let entry = ledger::Entry {
        id: 0,
        mode: mode.word().to_owned(),
        kind: kind.to_owned(),
        by: by.word().to_owned(),
        pid: std::process::id(),
        started: now,
        resumes: 0,
    };
    let id = ledger::allocate(store, &entry)?;
    // AN ID A RECEIPT CANNOT CARRY IS REFUSED BEFORE THE VENDOR IS ASKED
    // (IA-10c repair 3; the second agent's repair-round-3 probe P6): a
    // receipt's run field is 32 bits (offset 60), and a landing under a wider
    // id stored its bars with no kept answer for every window, an error event
    // each, while every later id is wider still. The id stays spent, as any
    // refused id does; the refusal and its error event name it.
    if u32::try_from(id).is_err() {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error(TARGET, "landing run id past a receipt")
                .with("run", telemetry::Value::Uint(id))
                .with("mode", telemetry::Value::Str(mode.word())),
        );
        return Err(RunsLedgerFault::Entry {
            why: format!(
                "run {id} is past {}, the largest run a receipt's 32-bit run field (offset 60) \
                 holds, so its answers could not be kept: no landing runs under it and nothing \
                 is asked of the vendor (the id stays spent, and every later id is past it \
                 too: the runs ledger and its claims need the operator's look)",
                u32::MAX
            ),
        });
    }
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::info(TARGET, "landing run id taken")
            .with("run", telemetry::Value::Uint(id))
            .with("mode", telemetry::Value::Str(mode.word()))
            .with("by", telemetry::Value::Str(by.word()))
            .with("kind", telemetry::Value::Str(kind)),
    );
    Ok(id)
}

/// The vendor's landing lock beneath the store root (FM-163).
#[must_use]
pub fn landing_lock(vendor: Vendor) -> String {
    format!("imports/{}.landing.lock", vendor.as_str())
}

/// The vendor's landing claim, held until dropped.
#[derive(Debug)]
pub struct VendorClaim {
    lock: Flock<File>,
    name: String,
    /// The lock file's device and inode, the key of its live notes in
    /// [`NOTING`]; `None` when the file could not be stat'ed, said at `warn`
    /// (its notes are then written but not remembered).
    file: Option<(u64, u64)>,
    /// This claim's place among the file's live holders, in claim order.
    seq: u64,
}

/// The live notes of this process's landing claims (D-6268, D-6269).
#[derive(Debug)]
struct Notes {
    /// The next claim's sequence number.
    next: u64,
    /// Each lock file's live holders (device and inode → claim sequence →
    /// that holder's latest note record): at most the api's landing requests
    /// in flight per file.
    live: BTreeMap<(u64, u64), BTreeMap<u64, Vec<u8>>>,
}

/// One note at a time in this process (D-6268). Every SHARED holder of a
/// vendor's landing claim is a request of the one api that serves the store
/// (`serve.lock`), and the EXCLUSIVE holder (`cli zerodha-repair
/// --from-store`) shares the file with no one, so serialising the notes of
/// one process is what keeps two holders from splicing theirs: before it,
/// two requests noting at once truncated, then wrote, each over the other,
/// and a refusal could name a holder neither of them wrote (verify-IA-10c-r1:
/// 18 of 30 rounds). It also remembers every live holder's latest note, so
/// a holder that lets go while another still holds writes that one's note
/// back (repair round 2, D-6269): the file never names a holder that has
/// finished while one that has not still holds.
static NOTING: Mutex<Notes> = Mutex::new(Notes {
    next: 0,
    live: BTreeMap::new(),
});

/// `who` as the lock file's one note record: its bytes, cut at a character
/// boundary to [`locks::PROBE_READ`] (the most a probe reads, the most a
/// claim holds), then padded with spaces to exactly that width. One
/// positional write of the whole width replaces the last note with no
/// truncation first, so a taker in another process reading the holder never
/// reads an emptied file between a truncate and a write; the reader
/// ([`locks::head_of`]) drops the padding.
fn note_record(who: &str) -> Vec<u8> {
    // `is_char_boundary(0)` holds for every string, so the walk stops.
    let mut end = who.len().min(locks::PROBE_READ);
    while !who.is_char_boundary(end) {
        end -= 1;
    }
    let mut record = Vec::with_capacity(locks::PROBE_READ);
    record.extend_from_slice(who.get(..end).unwrap_or_default().as_bytes());
    record.resize(locks::PROBE_READ, b' ');
    record
}

impl VendorClaim {
    /// Writes who holds it into the lock file, so a refused taker can name
    /// the holder: one whole-width record ([`note_record`]) under this
    /// process's one note lock, so two holders never splice their notes, and
    /// remembered as this holder's latest note while it holds. A note that
    /// cannot be written is a warning by name; the claim itself still holds.
    pub fn note(&self, who: &str) {
        let record = note_record(who);
        let written = {
            let mut notes = NOTING.lock().unwrap_or_else(PoisonError::into_inner);
            let written = self.lock.write_all_at(&record, 0);
            if let Some(file) = self.file {
                notes.live.entry(file).or_default().insert(self.seq, record);
            }
            written
        };
        if let Err(why) = written {
            self.say_unwritten("landing claim not noted", &why);
        }
    }

    /// Says at `warn`, naming the lock, that a note was not written.
    fn say_unwritten(&self, what: &'static str, why: &std::io::Error) {
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn(TARGET, what)
                .with("lock", telemetry::Value::Str(&self.name))
                .with("why", telemetry::Value::Str(&why.to_string())),
        );
    }
}

impl Drop for VendorClaim {
    /// A holder that lets go while others of this process still hold the
    /// file writes back the note of the newest of them (claim order), still
    /// under its own hold and under [`NOTING`], so a taker refused after it
    /// names a holder that holds: before, the file kept the LAST note
    /// written, and a sharer that had finished was named, with its run and
    /// mode, while an earlier one still held (the second agent's
    /// repair-round-2 follow-up, probe A2). The last holder to let go writes
    /// nothing: the file is free, and its stale note names no holder anyone
    /// is refused by.
    fn drop(&mut self) {
        let Some(file) = self.file else {
            return;
        };
        let written = {
            let mut notes = NOTING.lock().unwrap_or_else(PoisonError::into_inner);
            let newest = notes.live.get_mut(&file).and_then(|holders| {
                holders.remove(&self.seq);
                holders.last_key_value().map(|(_, record)| record.clone())
            });
            if let Some(record) = newest {
                self.lock.write_all_at(&record, 0)
            } else {
                notes.live.remove(&file);
                Ok(())
            }
        };
        if let Err(why) = written {
            self.say_unwritten("landing claim note not restored", &why);
        }
    }
}

/// Takes `vendor`'s landing claim in `store` (FM-163): `Hold::Shared` for an
/// api landing request (one api per store, `serve.lock`; its requests share
/// the claim), `Hold::Exclusive` for `cli zerodha-repair --from-store`, and
/// writes `who` into it. One try, a few retries [`locks::PROBE_RETRY`] apart,
/// then refused by name: a landing never waits for another process's.
///
/// # Errors
///
/// [`LockFault::Busy`] naming the holder as it wrote itself, and the other
/// [`LockFault`]s of the lock file (a link, a gone root, a replaced file).
pub fn claim(
    store: &Path,
    vendor: Vendor,
    hold: Hold,
    who: &str,
) -> Result<VendorClaim, LockFault> {
    use std::os::unix::fs::MetadataExt as _;
    let name = landing_lock(vendor);
    let file = locks::open_lock_file(store, &name)?;
    let lock = locks::take(file, store, &name, hold, locks::PROBE_RETRY)?;
    let identity = lock.metadata().map(|held| (held.dev(), held.ino()));
    let seq = {
        let mut notes = NOTING.lock().unwrap_or_else(PoisonError::into_inner);
        let seq = notes.next;
        notes.next = notes.next.wrapping_add(1);
        seq
    };
    let claim = VendorClaim {
        lock,
        name,
        file: identity.as_ref().ok().copied(),
        seq,
    };
    if let Err(why) = &identity {
        claim.say_unwritten("landing claim not remembered", why);
    }
    claim.note(who);
    Ok(claim)
}

/// One window, as the hook is handed it after a door returned.
#[derive(Debug, Clone, Copy)]
pub struct Landed<'a> {
    /// The instrument key the pack is filed under (the bar path's own).
    pub key: &'a InstrumentKey,
    /// The rung asked.
    pub granularity: Granularity,
    /// The window asked.
    pub asked: crate::session::Window,
    /// The vendor's answer: its body, digest and status (its rows are
    /// [`Self::rows`]; an `Answered` here may carry none).
    pub answered: &'a Answered,
    /// The decoded rows the door was handed.
    pub rows: &'a [RawRow],
    /// What the door did, with its named refusals.
    pub landing: &'a DoorLanding,
    /// Which door.
    pub door: Door,
    /// How the vendor stamps its rows.
    pub encoding: TimestampEncoding,
}

/// Why a window's answer was not kept.
#[derive(Debug)]
pub enum NotKept {
    /// The keeper refused it (already an audited error event, "response not
    /// kept", naming the pack).
    Refused(KeepError),
    /// The rung has no store directory, so the door landed nothing.
    RungUnstored,
    /// The run id does not fit the receipt's 32 bits.
    RunTooWide {
        /// The run.
        run: u64,
    },
}

/// What the hook did with one window.
#[derive(Debug)]
pub struct Hooked {
    /// The keep's outcome; every refusal is an error event by name, never
    /// silent.
    pub kept: Result<Outcome, NotKept>,
    /// The spool append, for a vendor whose verifier exists and a body that
    /// was kept: `None` when nothing was spooled, with the reason in
    /// [`Self::not_spooled`].
    pub spooled: Option<Result<Appended, String>>,
    /// Why the window was not spooled, when it was not.
    pub not_spooled: Option<&'static str>,
}

/// One landing run's hook: its keeper and its one spool writer (critic
/// IA-9-2: one appender per run), the gap ledger it verifies against, read
/// once and then only its new lines, and each series' first served day bar,
/// read once per series per run.
#[derive(Debug)]
pub struct RunHook {
    root: PathBuf,
    run: u64,
    vendor: Vendor,
    writer: Option<SpoolWriter>,
    ledger: SharedLedger,
    first_days: Mutex<HashMap<Series, Option<Day>>>,
}

/// One store's gap ledger, read once and shared by every run of the process.
type SharedLedger = Arc<Mutex<Option<GapLedger>>>;

/// The store roots one process lands into, which the shared-ledger map is
/// pre-sized to (docs/07 law 2; D-6268): ONE. The one caller of
/// [`RunHook::new`] is the api, and an api serves one store root
/// (`Site::store_root`, one api per store by `serve.lock`). A test process
/// lands into one temporary root per test and grows the map past it, one
/// amortised insert per root, never per window.
const STORE_ROOTS: usize = 1;

/// The gap ledger of the store at `root`, read whole once per process and
/// then only its new lines (`GapLedger::refresh`, which starts again when
/// the file shrank): the ledger is store-wide, so every run of a process
/// shares one, and a run's first window pays for the lines appended since the
/// last run's last window, never for the whole file again (9.0 MB on the
/// operator's store, measured by `ls` on 23 Oct 2026). One entry per store
/// root a process lands into ([`STORE_ROOTS`]).
fn shared_ledger(root: &Path) -> SharedLedger {
    static LEDGERS: std::sync::OnceLock<Mutex<HashMap<PathBuf, SharedLedger>>> =
        std::sync::OnceLock::new();
    let mut held = LEDGERS
        .get_or_init(|| Mutex::new(HashMap::with_capacity(STORE_ROOTS)))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    Arc::clone(
        held.entry(root.to_path_buf())
            .or_insert_with(|| Arc::new(Mutex::new(None))),
    )
}

impl RunHook {
    /// The hook of run `run` of `vendor` under `root`, for a landing planned
    /// to land `series` series: its first-day map is pre-sized to them
    /// (docs/07 law 2; D-6268), one entry per series the run lands. The
    /// caller states its plan: a `/pull/spot` or `/pull/repair` request and a
    /// `/pull/run` leg their instruments, a `/pull/fno` walk the
    /// contract-months its discovery wanted, each already a list in memory, so
    /// the map is never larger than what the caller holds. A run that lands
    /// more series than it planned grows the map, one amortised insert per
    /// series, never per window. A vendor whose verifier exists (Zerodha,
    /// `crate::zerodha_verify`) spools its verdicts; every vendor's answers
    /// are kept.
    #[must_use]
    pub fn new(root: &Path, run: u64, vendor: Vendor, series: usize) -> Self {
        Self {
            root: root.to_path_buf(),
            run,
            vendor,
            writer: (vendor == Vendor::Zerodha)
                .then(|| SpoolWriter::new(root, run, store::verdict::Mode::ZerodhaMerge, now_s())),
            ledger: shared_ledger(root),
            first_days: Mutex::new(HashMap::with_capacity(series)),
        }
    }

    /// How many series the first-day map holds room for before it grows: at
    /// least the `series` the run was planned with.
    #[must_use]
    pub fn series_room(&self) -> usize {
        self.first_days
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .capacity()
    }

    /// The run's id.
    #[must_use]
    pub const fn run(&self) -> u64 {
        self.run
    }

    /// Keeps `window`'s answer, then spools its verdicts (D-4859: land,
    /// keep, spool). Blocks: it is called where the landing ran, on the
    /// landing's own blocking thread, never on a shared async worker.
    #[must_use]
    pub fn after_landing(&self, window: &Landed<'_>) -> Hooked {
        let Some(rung) = window.granularity.store_timeframe() else {
            // A rung with no directory never landed: the door refused it by
            // name already, and there is nothing to keep beside no bars.
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::error(TARGET, "response not kept")
                    .with("run", telemetry::Value::Uint(self.run))
                    .with("instrument", telemetry::Value::Str(&window.key.to_string()))
                    .with("why", telemetry::Value::Str("rung_unstored")),
            );
            return Hooked {
                kept: Err(NotKept::RungUnstored),
                spooled: None,
                not_spooled: Some("rung_unstored"),
            };
        };
        let Ok(run) = u32::try_from(self.run) else {
            return self.not_kept_run();
        };
        let (first_stamp, last_stamp) = stamps(window.rows, window.encoding);
        let landing = response_keep::Landing {
            window: window.asked,
            first_stamp,
            last_stamp,
            rows_decoded: u32::try_from(window.rows.len()).unwrap_or(u32::MAX),
            rows_landed: u32::try_from(window.landing.ingested.bars_committed).unwrap_or(u32::MAX),
            rows_refused: window.landing.rows_refused(),
            refused: &window.landing.refused,
            run,
            rung: window.granularity as u8,
            rules_version: super::LANDING_RULES,
            door: window.door,
        };
        let pack = Pack {
            vendor: self.vendor,
            key: window.key,
            timeframe: rung,
        };
        let kept = response_keep::append(
            &self.root,
            pack,
            response_keep::Offered::Answer(window.answered),
            &landing,
        )
        .map_err(NotKept::Refused);
        let (spooled, not_spooled) = match (&kept, &self.writer) {
            (_, None) => (None, Some("no_verifier_for_vendor")),
            (Err(_), Some(_)) => (None, Some("not_kept")),
            (Ok(outcome), Some(_)) if !body_in_pack(outcome) => (None, Some("body_not_in_pack")),
            (Ok(outcome), Some(writer)) => {
                let spooled = self.spool(window, rung, outcome, writer);
                (Some(spooled), None)
            }
        };
        if let Some(Err(why)) = &spooled {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::error(TARGET, "verdicts not spooled")
                    .with("run", telemetry::Value::Uint(self.run))
                    .with("instrument", telemetry::Value::Str(&window.key.to_string()))
                    .with("why", telemetry::Value::Str(why)),
            );
        }
        Hooked {
            kept,
            spooled,
            not_spooled,
        }
    }

    /// A run id past a receipt's 32 bits cannot be written into one: refused
    /// by name, never truncated into another run's id.
    fn not_kept_run(&self) -> Hooked {
        let why = format!(
            "run {} does not fit the receipt's 32-bit run field; the answer was not kept",
            self.run
        );
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::error(TARGET, "response not kept")
                .with("run", telemetry::Value::Uint(self.run))
                .with("why", telemetry::Value::Str(&why)),
        );
        Hooked {
            kept: Err(NotKept::RunTooWide { run: self.run }),
            spooled: None,
            not_spooled: Some("run_too_wide"),
        }
    }

    /// Verifies the window's days against what it landed and appends their
    /// records to the run's spool.
    fn spool(
        &self,
        window: &Landed<'_>,
        rung: Timeframe,
        outcome: &Outcome,
        writer: &SpoolWriter,
    ) -> Result<Appended, String> {
        let series = Series {
            exchange: window.key.exchange,
            segment: window.key.segment,
            symbol: window.key.underlying.as_str().to_owned(),
            contract: Contract::of(window.key.kind),
        };
        let first_day_bar = self.first_day(&series);
        let shape = zerodha_verify::answer_shape(&window.answered.body);
        let receipt = &outcome.first.receipt;
        let input = zerodha_verify::WindowInput {
            series: &series,
            rung,
            rows: window.rows,
            asked: (receipt.asked_from, receipt.asked_to),
            receipt: outcome.first.ordinal,
            refused: &window.landing.refused,
            rows_refused: window.landing.rows_refused(),
            first_day_bar,
            shape: &shape,
        };
        // ONE VERIFICATION AT A TIME PER STORE: the ledger is shared, and is
        // held across the window's check (its cost is C-ZV-01's), so two
        // landing requests of one process verify one after the other while
        // their landings run side by side (docs/06-limits.md, L-IA10C-02).
        let mut held = self.ledger.lock().unwrap_or_else(PoisonError::into_inner);
        // THE LINES THIS WINDOW'S LANDING JUST WROTE ARE READ: the whole
        // ledger once per process, then only what was appended since (the
        // reader never takes a torn last line).
        let ledger: &GapLedger = match &mut *held {
            Some(ledger) => {
                ledger.refresh(&self.root)?;
                ledger
            }
            slot @ None => slot.insert(GapLedger::read(&self.root)?),
        };
        zerodha_verify::verify_window(&self.root, &input, ledger, writer)
    }

    /// The series' first served day bar, read once per series per run; a
    /// read that fails is said by name and the window is verified with none,
    /// which the verifier treats as "no first day known".
    fn first_day(&self, series: &Series) -> Option<Day> {
        let mut known = self
            .first_days
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(day) = known.get(series) {
            return *day;
        }
        let day = match zerodha_verify::first_served_day(&self.root, series) {
            Ok(day) => day,
            Err(refusal) => {
                let _dropped_when_filtered = telemetry::emit(
                    &telemetry::Event::error(TARGET, "first day bar unread")
                        .with("series", telemetry::Value::Str(&series.key_text()))
                        .with("why", telemetry::Value::Str(&refusal.why)),
                );
                None
            }
        };
        known.insert(series.clone(), day);
        day
    }
}

/// Says, as an audited error event, that a window reached its door with no
/// landing run, instrument key or answer carried beside it, so nothing was
/// kept: its minutes read `no_response_kept` at the next verification. A
/// caller's word for a window it cannot hand to [`RunHook::after_landing`],
/// never a silent skip.
pub fn note_unhooked(asked: crate::session::Window) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::error(TARGET, "response not kept")
            .with("from", telemetry::Value::Str(&asked.from().to_string()))
            .with("to", telemetry::Value::Str(&asked.to().to_string()))
            .with(
                "why",
                telemetry::Value::Str(
                    "the window reached its door with no landing run, key or answer carried \
                     beside it",
                ),
            ),
    );
}

/// Whether the window's body is in its pack (kept, or the same body as an
/// earlier receipt): only then are its days verified against it on the
/// window path. A `pack_full` receipt's minutes read `no_response_kept` at
/// the next cold verification instead.
fn body_in_pack(outcome: &Outcome) -> bool {
    matches!(
        outcome.first.receipt.kept,
        response_keep::Kept::Kept | response_keep::Kept::Duplicate
    )
}

/// The first and last decoded row's stamps, UNIX seconds (0 = none).
fn stamps(rows: &[RawRow], encoding: TimestampEncoding) -> (i64, i64) {
    let seconds = |row: &RawRow| {
        i64::try_from(zerodha_verify::utc_millis(row.timestamp, encoding).div_euclid(1_000))
            .unwrap_or(0)
    };
    (
        rows.first().map_or(0, seconds),
        rows.last().map_or(0, seconds),
    )
}

#[cfg(test)]
#[path = "landing_run_tests.rs"]
mod tests;
