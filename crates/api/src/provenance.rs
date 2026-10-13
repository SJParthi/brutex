//! `GET /provenance.json?feed=<feed>&instrument=<key>&timeframe=<rung>&month=<YYYY-MM>[&t=<unix s>]`
//! -- where one stored instrument-month came from. Findings DB-078 (no number
//! on `/db` could be audited back to its source) and DB-G56 (nothing in the
//! store told a zip-built month from a tick-store-built one). Decision D-3334;
//! the sidecar format is WP-38a's `store::provenance` (`docs/02-store-format.md`
//! §8.3).
//!
//! # What it answers
//!
//! 1. **The bars.** One `metadata` call on the month's `.bin` (the path
//!    probe): an absent month is a `404` naming the path, never an empty
//!    answer. Then the bar file's header: how many bars are committed, and its
//!    commit **generation** -- how many landings appended to it -- with the
//!    file's modification time.
//! 2. **The sidecar** (`<month>.prv`), when one exists: every record, or with
//!    `t` the one record whose bars hold `t`'s bar. Each record names the
//!    source kind (`archive_member`, `tickstore_day`, `vendor_endpoint`), its
//!    name, size, CRC-32 and name digest, the day, the bars it appended, the bar
//!    definition, the journal record it wrote, the generation and the build
//!    commit. Each record is cross-read against its journal: the GDFL import
//!    journal's row for its day, or the pull journal's record at its ordinal.
//!    A sidecar the store refuses (damaged, foreign, past the bars) is named
//!    under `sidecar.error` and never served as a record.
//! 3. **What no record covers** (`unrecorded`): bars written before the month
//!    had a sidecar, or by a landing that died between its commit and its
//!    record. For those -- and for a month with no sidecar at all, which is
//!    every month written before D-3334 -- the answer is **rebuilt from the
//!    journals and labelled `"reconstructed": true` and "reconstructed, member
//!    digest unknown"** (D-3334 item 3):
//!    - **GDFL** from `imports/gdfl.journal`: the import-day lines of the
//!      month (or of `t`'s day) for the instrument's import kind -- the
//!      deciding line, its outcome and counts -- each beside the bars of the
//!      month stamped on that day (their index range and count), so every bar
//!      is tied to the line of its day and a bar on a day the journal does not
//!      name is counted apart; and the `run` line a close names, which since
//!      D-3317 records the source (zip or tick store), the archive path, size
//!      and digest, and the commit. A line written before D-3317 names no run,
//!      and the answer says the archive was not recorded rather than guessing
//!      one: it names only the vendor day folder, the day zip inside the
//!      capital-market archive and the tick-store day file as THIS build's
//!      reader names them, with the stem of the instrument's file.
//!    - **Every other feed** from `audit/pull.journal`: the run records
//!      attributed to the feed (by the endpoint or the cash-identity policy
//!      their `source` names, `crate::audit_json::vendor_of_source`) whose
//!      window covers the month and that offered bars. A pull record names its
//!      window and its request, never the instrument-months it wrote, so the
//!      candidates are listed oldest first, and one is named as the writer only
//!      when the bar file committed exactly once (generation 1) and its
//!      modification time falls inside exactly one candidate's run. A run's
//!      opening record (D-3341) is paired with its close; one never closed
//!      (its process stopped) is listed, and blocks naming any writer once it
//!      began before the file's last write.
//! 4. **The operator's window** (D-3338): a Zerodha month outside
//!    2018-01-01..2026-09-30 is labelled "outside your window", with its pull
//!    run.
//!
//! # Cost
//!
//! Per request: one `metadata` call and one bar-file open (its header and
//! checksum); with `t`, one `first_at_or_after` -- through the month's `.tix`
//! one entry and one bar read, else the D-1434 bisection, O(log n) reads,
//! which the answer names; one `open` and one read of the `.prv`, at most
//! `store::provenance::MAX_FILE_LEN` bytes, and a bounded walk of its at most
//! `store::provenance::MAX_RECORDS` records; then the journal probe:
//!
//! - **GDFL**: the import journal's tail (one `metadata` call and the bytes
//!   appended since the last read, `import_journal::snapshot`), then the rows
//!   of the month's days: O(log n + rows of those days). A close's `run=` is
//!   one hash probe into an index of the journal's run lines, rebuilt only
//!   after the journal grew. Each day's bars cost at most two time lookups,
//!   one when the previous day's end is already this day's start (one bar
//!   read decides): through `.tix` one entry each, else the D-1434 bisection,
//!   O(log n) reads each. **This is O(days in the month), not O(1)**: at most
//!   31 days, so at most 62 lookups.
//! - **A pulled feed**: the pull journal's tail (one `metadata` call and the
//!   records appended since), then one hash probe of an index from month to the
//!   runs whose window covers it, and a walk of those runs. **The first read in
//!   a process walks the whole journal**, which keeps no index of its own
//!   (4,217,027 records, 1,079,558,912 bytes, 12 of them runs, on the
//!   operator's store, 9 Oct 2026); the answer that pays for it says so in
//!   `notes`, with the time it took. A run whose window spans more than
//!   [`MAX_INDEXED_MONTHS`] months, or names no calendar day, is kept on a
//!   short list that every request walks, and the answer counts it.
//!
//! Measured p50/p99 are in the commit that built this route and go to
//! `docs/06-limits.md` with WP-99; none is claimed here.
//!
//! UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
//! "Cost claims of the /db fix round that name no proof").

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};

use brutex_core::instrument::{Contract, Exchange, InstrumentKey, Segment};
use brutex_core::vendor::Vendor;
use pull::gdfl_cm::{CmKind, CmScope};
use pull::gdfl_import::ImportKind;
use pull::session::{Day, IstMoment};
use serde_json::{Value, json};
use store::file::{BarFile, StoreError, TimeLookup};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};
use store::provenance::{Found, Record, Sidecar, SourceKind};

use super::import_journal::{DayOutcome, DayRow, Parsed, RunLine};
use super::{Loaded, ist_stamp, param};
use crate::audit;

/// What D-3334 item 3 has a reconstructed answer say.
pub(crate) const RECONSTRUCTED: &str = "reconstructed, member digest unknown";

/// What a month outside the operator's window is labelled (D-3338), in the
/// words `/feeds.json` gives the window's `label_outside`.
pub(crate) const OUTSIDE_WINDOW: &str = "outside your window";

/// What a month that straddles the operator's window is labelled.
pub(crate) const PARTLY_OUTSIDE_WINDOW: &str = "partly outside your window";

/// The widest run window, in months, the pull-journal index files under each
/// month it covers. A wider one (no pull this repository makes spans a
/// century) is kept on a list every request walks, and counted.
pub(crate) const MAX_INDEXED_MONTHS: u32 = 1_200;

/// Byte 118 of a pull-journal record: its kind (`audit::Kind::code`), read
/// before anything is decoded so a member failure -- 4,217,015 of the
/// operator's 4,217,027 records -- is passed over unread.
/// `tests::the_kind_byte_is_where_the_writer_puts_it` pins it to the writer.
const KIND_AT: usize = 118;

/// A read of more pull-journal records than this is named in `notes`: a
/// process's first walk, not a tail.
const FIRST_WALK_RECORDS: u64 = 100_000;

/// Records read per chunk of a pull-journal walk: 4 MiB.
const CHUNK_RECORDS: u64 = 16_384;

/// Microseconds in one second.
const MICROS: i64 = 1_000_000;

/// How the note of a run's opening record begins when the run writes it again
/// because another writer appended in between (`audit::Journal::append_failure`,
/// D-3341): the same run, not another.
const CONTINUES: &str = "continues the run opened at record ";

/// The content type every answer here carries.
fn json_header() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

// ══ What a request asks ═════════════════════════════════════════════════════

/// The instant `t=` names, and its IST day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Instant {
    /// UNIX seconds, as `t=` gave them.
    pub(crate) secs: i64,
    /// The same instant in microseconds, as a bar is stamped.
    pub(crate) micros: i64,
    /// Its IST day.
    pub(crate) day: Day,
}

/// What a request asked for, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asked {
    /// The feed whose month it is.
    pub(crate) feed: Vendor,
    /// The exchange.
    pub(crate) exchange: Exchange,
    /// The store segment.
    pub(crate) segment: Segment,
    /// The store symbol: the underlying, for a contract.
    pub(crate) symbol: String,
    /// The contract, for an option or a future.
    pub(crate) contract: Option<Contract>,
    /// The rung.
    pub(crate) timeframe: Timeframe,
    /// The month.
    pub(crate) month: YearMonth,
    /// The one bar asked about, when `t=` was given.
    pub(crate) t: Option<Instant>,
}

impl Asked {
    /// The census key: `<EXCHANGE>-<SEGMENT>-<SYMBOL>[-<contract>]`.
    pub(crate) fn key(&self) -> String {
        let base = format!(
            "{}-{}-{}",
            self.exchange.as_str(),
            self.segment.as_str(),
            self.symbol
        );
        match self.contract {
            Some(contract) => format!("{base}-{}", contract.as_str()),
            None => base,
        }
    }

    /// The month's path parts for sibling `file`.
    fn parts(&self, file: FileKind) -> PathParts<'_> {
        PathParts {
            vendor: self.feed,
            exchange: self.exchange.as_str(),
            segment: self.segment.as_str(),
            symbol: &self.symbol,
            contract: self.contract,
            timeframe: self.timeframe,
            month: self.month,
            file,
        }
    }

    /// The days the answer is about: `t`'s day, or the whole month.
    fn days(&self) -> Option<(Day, Day)> {
        if let Some(t) = self.t {
            return Some((t.day, t.day));
        }
        let first = Day::new(self.month.year(), self.month.month(), 1).ok()?;
        Some((first, first.end_of_month()))
    }

    /// The echo every answer opens with.
    fn echo(&self) -> Value {
        json!({
            "feed": self.feed.as_str(),
            "instrument": self.key(),
            "timeframe": self.timeframe.as_str(),
            "month": self.month.to_string(),
            "t": self.t.map(|t| t.secs),
        })
    }
}

/// Reads a query string, or names the parameter it cannot read.
///
/// Every parameter but `t` is required and none is defaulted: a month's
/// provenance is one feed's, one instrument's, one rung's, and a default
/// would answer for a file the reader did not pick.
///
/// # Errors
///
/// The parameter and why, as one sentence for a `400`.
pub(crate) fn asked(query: &str) -> Result<Asked, String> {
    let feed = feed_of(&param(query, "feed"))?;
    let (exchange, segment, symbol, contract) = instrument_of(&param(query, "instrument"))?;
    if param(query, "timeframe").is_empty() {
        return Err(
            "/provenance.json needs timeframe=<rung>, for example 1s, 1min or 1day, and never \
             guesses one"
                .to_owned(),
        );
    }
    let timeframe = super::timeframe_param(query)?;
    if param(query, "month").is_empty() {
        return Err("/provenance.json needs month=YYYY-MM and never guesses one".to_owned());
    }
    let month = super::month_param(query, "month")?;
    let t = instant(&param(query, "t"), month)?;
    Ok(Asked {
        feed,
        exchange,
        segment,
        symbol,
        contract,
        timeframe,
        month,
        t,
    })
}

/// The feed a word names. Any feed the store keys bars by: GDFL's months
/// are reconstructed from its import journal, every other feed's from the
/// pull journal.
fn feed_of(word: &str) -> Result<Vendor, String> {
    if word.is_empty() {
        return Err(format!(
            "/provenance.json needs feed=<{}> and never guesses one",
            Vendor::ALL.map(Vendor::as_str).join("|")
        ));
    }
    Vendor::ALL
        .into_iter()
        .find(|vendor| word.eq_ignore_ascii_case(vendor.as_str()))
        .ok_or_else(|| {
            format!(
                "feed={word:?} is not a feed the store keys bars by: {}",
                Vendor::ALL.map(Vendor::as_str).join(", ")
            )
        })
}

/// The instrument a census key names: `NSE-INDEX-NIFTY`, `NSE-CASH-RELIANCE`
/// or `NSE-FNO-NIFTY-2019-03-28-1150000-CE`. A contract belongs to `FNO`
/// only, and an `FNO` key names one: the store files an option's bars under
/// its contract, never under the underlying alone.
fn instrument_of(text: &str) -> Result<(Exchange, Segment, String, Option<Contract>), String> {
    let shape = "is not a census key <EXCHANGE>-<SEGMENT>-<SYMBOL>[-<contract>], for example \
                 NSE-INDEX-NIFTY, NSE-CASH-RELIANCE or NSE-FNO-NIFTY-2019-03-28-1150000-CE";
    if text.is_empty() {
        return Err(format!(
            "/provenance.json needs instrument=<key>: it {shape}"
        ));
    }
    let key = crate::bars::SeriesKey::parse(text)
        .filter(|key| key.text() == text)
        .ok_or_else(|| format!("instrument={text:?} {shape}"))?;
    let exchange = Exchange::parse(&key.exchange)
        .map_err(|_| format!("instrument={text:?}: {:?} is not an exchange", key.exchange))?;
    let segment = Segment::parse(&key.segment).map_err(|_| {
        format!(
            "instrument={text:?}: {:?} is not one of INDEX, CASH, FNO",
            key.segment
        )
    })?;
    match (segment, key.contract) {
        (Segment::Fno, None) => Err(format!(
            "instrument={text:?} names no contract, and an FNO month is filed under its contract"
        )),
        (Segment::Index | Segment::Cash, Some(_)) => Err(format!(
            "instrument={text:?} is {} and a contract belongs to FNO only",
            segment.as_str()
        )),
        _ => Ok((exchange, segment, key.symbol, key.contract)),
    }
}

/// `t=<unix seconds>`, the open of one bar of `month`, or `None` when absent.
fn instant(raw: &str, month: YearMonth) -> Result<Option<Instant>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    let secs = raw
        .parse::<i64>()
        .ok()
        .filter(|secs| secs.to_string() == raw)
        .ok_or_else(|| format!("t={raw:?} is not a whole number of unix seconds"))?;
    let micros = secs
        .checked_mul(MICROS)
        .ok_or_else(|| format!("t={raw} is past the range a bar can be stamped in"))?;
    let at = IstMoment::from_epoch_secs(secs).map_err(|why| format!("t={raw}: {why}"))?;
    let day = at.day();
    let of = day.year_month().map_err(|why| format!("t={raw}: {why}"))?;
    if of != month {
        return Err(format!(
            "t={raw} is {}, in {of}, not in month={month}: a month file holds the bars of its own \
             IST days",
            ist_stamp(secs)
        ));
    }
    Ok(Some(Instant { secs, micros, day }))
}

// ══ The route ═══════════════════════════════════════════════════════════════

/// `GET /provenance.json`.
pub(super) async fn provenance_json(
    axum::extract::State(site): axum::extract::State<Loaded>,
    uri: axum::http::Uri,
) -> (
    axum::http::StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let query = uri.query().unwrap_or("").to_owned();
    let asked = match asked(&query) {
        Ok(asked) => asked,
        // THE REFUSAL IS THE BODY'S, and `logs::note_request` records the path
        // and the 400. No event of its own: this crate counts its emit sites.
        Err(why) => {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                json_header(),
                json!({ "error": why }).to_string(),
            );
        }
    };
    let root = site.store_root.clone();
    match crate::detail::run_store_read(move || answer(&root, &asked)).await {
        Ok(Answer { status, body }) => (status, json_header(), body.to_string()),
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "provenance read",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, json_header(), body)
        }
    }
}

/// A status and the JSON body that goes with it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Answer {
    /// The HTTP status.
    pub(crate) status: axum::http::StatusCode,
    /// The body.
    pub(crate) body: Value,
}

impl Answer {
    /// A refusal: the echo of the request, `error`, and anything else named.
    fn refused(status: axum::http::StatusCode, asked: &Asked, error: &str, extra: Value) -> Self {
        let mut body = asked.echo();
        if let Some(object) = body.as_object_mut() {
            object.insert("error".to_owned(), Value::String(error.to_owned()));
            if let Value::Object(more) = extra {
                object.extend(more);
            }
        }
        Self { status, body }
    }
}

/// The whole answer for one request over the store at `root`.
pub(crate) fn answer(root: &Path, asked: &Asked) -> Answer {
    use axum::http::StatusCode;
    let bars_path = match StorePath::new(asked.parts(FileKind::Bars)) {
        Ok(path) => path,
        Err(why) => {
            return Answer::refused(
                StatusCode::BAD_REQUEST,
                asked,
                &format!("{} {}: {why}", asked.key(), asked.month),
                Value::Null,
            );
        }
    };
    let relative = bars_path.to_string();
    // THE PATH PROBE. An absent month is a fact about the store, and a 404
    // naming the file looked for -- never an empty provenance a reader could
    // take for "nothing is known about these bars".
    let meta = match std::fs::metadata(bars_path.to_path_buf(root)) {
        Ok(meta) => meta,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Answer::refused(
                StatusCode::NOT_FOUND,
                asked,
                &format!("the store holds no bar file at {relative}: this month is not held"),
                json!({ "held": false, "path": relative }),
            );
        }
        Err(why) => {
            return Answer::refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                asked,
                &format!("{relative}: {why}"),
                json!({ "path": relative }),
            );
        }
    };
    let bars = match BarFile::open_existing(root, bars_path, symbol_id(&asked.symbol)) {
        Ok(bars) => bars,
        // A LANDING HOLDS THE MONTH'S LOCK WHILE IT APPENDS, and the store
        // refuses a reader meanwhile. That is the month being written, not a
        // damaged one: a 409 that says so, never a 500 and never a guess.
        Err(why @ StoreError::Locked { .. }) => {
            return Answer::refused(
                StatusCode::CONFLICT,
                asked,
                &format!(
                    "{why}: the month is being written right now; ask again when the landing \
                     commits"
                ),
                json!({ "path": relative }),
            );
        }
        Err(why @ StoreError::Missing { .. }) => {
            return Answer::refused(
                StatusCode::NOT_FOUND,
                asked,
                &format!("{why}: this month is not held"),
                json!({ "held": false, "path": relative }),
            );
        }
        Err(why) => {
            return Answer::refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                asked,
                &why.to_string(),
                json!({ "path": relative }),
            );
        }
    };
    let at_bar = match asked.t {
        None => None,
        Some(t) => match bar_at(&bars, t) {
            Ok(index) => Some(index),
            Err(why) => {
                return Answer::refused(
                    StatusCode::NOT_FOUND,
                    asked,
                    &why,
                    json!({ "path": relative }),
                );
            }
        },
    };
    // THE GENERATION EVERY READER OPENS (D-3350; DB-N26, WP-12d).
    served_answer(root, asked, (bars_path, relative, &meta), &bars, at_bar)
}

/// The answer for the open month `bars` at `bars_path` (below the root,
/// `relative`): resolved to the generation every reader opens
/// ([`served_month`]), its sidecar read beside it, and the provenance built.
fn served_answer(
    root: &Path,
    asked: &Asked,
    (bars_path, relative, meta): (StorePath<'_>, String, &std::fs::Metadata),
    bars: &BarFile,
    at_bar: Option<u64>,
) -> Answer {
    use axum::http::StatusCode;
    let (served, served_root, modified) = match served_month(root, bars_path, meta) {
        Ok(served) => served,
        Err(why) => {
            return Answer::refused(
                StatusCode::INTERNAL_SERVER_ERROR,
                asked,
                &why,
                json!({ "path": relative }),
            );
        }
    };
    let sidecar = read_sidecar(root, bars_path, bars, &served_root);
    let body = provenance(
        root,
        asked,
        &Month {
            relative: bars_path.to_path_buf(&served_root).display().to_string(),
            original: (served > 0).then_some(relative),
            served,
            bars,
            modified,
            at_bar,
        },
        &sidecar,
    );
    Answer {
        status: StatusCode::OK,
        body,
    }
}

/// The generation of `bars_path`'s month every reader opens (D-3350; DB-N26,
/// WP-12d): its ordinal (`0` the original), its root below the store root,
/// and its bar file's modification time. A month served from a repair
/// revision is named by the revision's path and time, never the superseded
/// original's, which no reader opens; `meta` is the original's, already
/// read by the path probe. One pointer read, and one `metadata` call for a
/// revision.
///
/// # Errors
///
/// The store's refusal of the month's pointer, in its words.
fn served_month(
    root: &Path,
    bars_path: StorePath<'_>,
    meta: &std::fs::Metadata,
) -> Result<(u32, PathBuf, Option<i64>), String> {
    let served = store::generation::served(root, bars_path).map_err(|why| why.to_string())?;
    let modified = if served.ordinal() == 0 {
        modified_secs(meta)
    } else {
        std::fs::metadata(bars_path.to_path_buf(&served.root(root)))
            .ok()
            .and_then(|meta| modified_secs(&meta))
    };
    Ok((served.ordinal(), served.root(Path::new("")), modified))
}

/// What the answer knows about the month file itself.
struct Month<'a> {
    /// The served bar file's path below the store root: the original's, or
    /// its repair revision's (D-3350).
    relative: String,
    /// The original's path below the store root, when a revision is served.
    original: Option<String>,
    /// The generation served: `0` the original, else the revision ordinal.
    served: u32,
    /// The bar file, open.
    bars: &'a BarFile,
    /// Its modification time, UNIX seconds, when the host says.
    modified: Option<i64>,
    /// `t`'s bar, when `t` was asked.
    at_bar: Option<u64>,
}

/// The answer once the month is open: bars, sidecar, coverage and, for what
/// no record covers, the reconstruction.
fn provenance(root: &Path, asked: &Asked, month: &Month<'_>, sidecar: &SidecarRead) -> Value {
    let mut notes: Vec<String> = Vec::new();
    // A REFUSED SIDECAR IS SAID AT THE TOP, not only under `sidecar`: what
    // follows is a reconstruction standing in for records the store would not
    // serve, and a reader must not take it for the recorded truth.
    if let SidecarRead::Refused { why, .. } = sidecar {
        notes.push(format!(
            "the sidecar beside this month was refused, so none of its records is served and \
             every bar below is {RECONSTRUCTED}: {why}"
        ));
    }
    // A REPAIRED MONTH IS SAID AT THE TOP TOO (D-3350; DB-N26): its bars are
    // a revision written whole by the repair, so its commits are not landings
    // and it has no sidecar; every bar is answered from the journals.
    if month.served > 0 {
        notes.push(format!(
            "this month is served from repair revision {} (D-3350): its bars, header and \
             modification time are the revision's, which the repair that published it wrote in \
             one commit; the original at {} is kept byte-identical and no reader opens it. A \
             revision carries no .prv sidecar, so every bar is {RECONSTRUCTED}, and \
             audit/{}.repairs names each repair and why",
            month.served,
            month.original.as_deref().unwrap_or("?"),
            asked.feed.as_str()
        ));
    }
    // A KEY THE RE-KEY MOVED, COPIED INTO OR RETIRED IS SAID (D-3331;
    // WP-12c): a copied month's import days are those of the key it was
    // copied from, and the reconstruction below cannot tell that by itself.
    if let Some(note) = crate::bars::rekey_note(root, asked.feed, &asked.key()) {
        notes.push(note);
    }
    let header = month.bars.header();
    let records = sidecar.records();
    let unrecorded = unrecorded(records, header.n_valid);
    let shown: Vec<&Record> = match month.at_bar {
        Some(index) => sidecar.covering(index).into_iter().collect(),
        None => records.iter().collect(),
    };
    // RECONSTRUCTED WHERE NO RECORD SPEAKS: `t`'s bar when no record holds it,
    // and otherwise the month whenever any committed bar is unrecorded.
    let rebuild = match month.at_bar {
        Some(_) => shown.is_empty(),
        None => !unrecorded.is_empty(),
    };
    let journals = Journals::new(root, asked);
    let recorded: Vec<Value> = shown
        .iter()
        .map(|record| record_json(record, &journals, asked))
        .collect();
    let reconstruction = if rebuild {
        match asked.feed {
            Vendor::Gdfl => gdfl_reconstruction(&journals, asked, month.bars, month.served),
            _ => pull_reconstruction(
                root,
                asked,
                (header.generation, month.served),
                month.modified,
                &mut notes,
            ),
        }
    } else {
        Value::Null
    };
    if let Some(TimeLookup::Bisection(why)) = month.at_bar.map(|_| month.bars.time_lookup()) {
        notes.push(format!(
            "t was located by bisection, O(log n) bar reads, because this month has no usable \
             time index: {why:?}"
        ));
    }
    let label = label(rebuild, recorded.is_empty(), header.n_valid);
    let mut body = asked.echo();
    if let Some(object) = body.as_object_mut() {
        object.extend(
            json!({
                "held": true,
                "bars": {
                    "path": month.relative,
                    "served_generation": month.served,
                    "original_path": month.original,
                    "n_valid": header.n_valid,
                    "generation": header.generation,
                    "first_ts_micros": header.first_ts_micros,
                    "last_ts_micros": header.last_ts_micros,
                    "modified_unix": month.modified,
                    "modified": month.modified.map(ist_stamp),
                },
                "bar": month.at_bar.map(|index| json!({ "index": index })),
                "sidecar": sidecar.json(),
                "recorded": recorded,
                "unrecorded": unrecorded
                    .iter()
                    .map(|&(from, to)| json!({ "first_index": from, "end": to }))
                    .collect::<Vec<_>>(),
                "reconstructed": rebuild,
                "label": label,
                "reconstruction": reconstruction,
                "window": window_json(asked),
                "notes": notes,
            })
            .as_object()
            .cloned()
            .unwrap_or_default(),
        );
    }
    body
}

/// The one-line verdict: recorded, reconstructed, or both. With `t`, a
/// record is shown only when it holds `t`'s bar, so `t` is never both.
fn label(rebuild: bool, none_recorded: bool, n_valid: u64) -> String {
    if n_valid == 0 {
        return "the month file holds no committed bar, so there is nothing to attribute"
            .to_owned();
    }
    match (rebuild, none_recorded) {
        (false, _) => "recorded".to_owned(),
        (true, true) => RECONSTRUCTED.to_owned(),
        (true, false) => format!(
            "recorded for the bars the sidecar's records hold; {RECONSTRUCTED}, for the bars \
             listed under unrecorded"
        ),
    }
}

/// The index of the bar stamped exactly `t`, or why there is none.
fn bar_at(bars: &BarFile, t: Instant) -> Result<u64, String> {
    let n_valid = bars.header().n_valid;
    let index = bars
        .first_at_or_after(t.micros)
        .map_err(|why| format!("t={}: {why}", t.secs))?;
    if index >= n_valid {
        return Err(format!(
            "no bar of this month is stamped {} (t={}): every one of its {n_valid} bars is \
             earlier",
            ist_stamp(t.secs),
            t.secs
        ));
    }
    let bar = bars
        .read_record(index)
        .map_err(|why| format!("t={}: bar {index}: {why}", t.secs))?;
    if bar.ts_micros != t.micros {
        let next = bar.ts_micros.div_euclid(MICROS);
        return Err(format!(
            "no bar of this month is stamped {} (t={}); the first bar at or after it is bar \
             {index}, stamped {} (t={next})",
            ist_stamp(t.secs),
            t.secs,
            ist_stamp(next)
        ));
    }
    Ok(index)
}

/// The symbol id a month's header carries and every open checks: the low
/// 32 bits of the symbol's 64-bit FNV-1a, the fold `pull::ingest` stamps in
/// and `crate::bars::open_classified` opens with (written there as `as u32`).
/// `tests::the_symbol_id_is_the_fold_ingest_stamps` pins the two together.
fn symbol_id(symbol: &str) -> u32 {
    let low = brutex_core::universe::fnv1a(symbol) & u64::from(u32::MAX);
    u32::try_from(low).unwrap_or(u32::MAX)
}

/// A file's modification time in UNIX seconds, when the host says.
fn modified_secs(meta: &std::fs::Metadata) -> Option<i64> {
    let since = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?;
    i64::try_from(since.as_secs()).ok()
}

// ══ The sidecar ═════════════════════════════════════════════════════════════

/// What stands beside the month.
#[derive(Debug)]
enum SidecarRead {
    /// No `.prv`.
    Absent { path: String },
    /// A `.prv` whose header never reached the disk.
    Unborn { path: String, bytes: u64 },
    /// A sidecar that describes these bars.
    Present { path: String, sidecar: Sidecar },
    /// A sidecar the store refuses, and why. Its records are never served.
    Refused { path: String, why: String },
}

impl SidecarRead {
    /// The valid records, in bar order; none unless the sidecar is present.
    fn records(&self) -> &[Record] {
        match self {
            Self::Present { sidecar, .. } => sidecar.records(),
            _ => &[],
        }
    }

    /// The record whose bars hold bar `index`, when the sidecar is present
    /// and one does (`Sidecar::covering`, a bounded search of its records).
    fn covering(&self, index: u64) -> Option<&Record> {
        match self {
            Self::Present { sidecar, .. } => sidecar.covering(index),
            _ => None,
        }
    }

    /// As the answer says it.
    fn json(&self) -> Value {
        match self {
            Self::Absent { path } => json!({
                "path": path,
                "state": "absent",
                "records": 0,
                "why": "no landing has recorded itself beside this month: it was written \
                        before D-3334, or by a writer that records nothing yet",
            }),
            Self::Unborn { path, bytes } => json!({
                "path": path,
                "state": "unborn",
                "records": 0,
                "bytes": bytes,
                "why": "its header never reached the disk, so it holds no record; the next \
                        landing writes it again",
            }),
            Self::Present { path, sidecar } => json!({
                "path": path,
                "state": "present",
                "records": sidecar.records().len(),
                "tail_bytes": sidecar.tail_bytes(),
                "tail": (sidecar.tail_bytes() > 0).then_some(
                    "bytes past the last valid record: an append that was interrupted, never \
                     served as a record"
                ),
            }),
            Self::Refused { path, why } => json!({
                "path": path,
                "state": "refused",
                "records": 0,
                "error": why,
            }),
        }
    }
}

/// Reads the sidecar beside `bars_path`'s served generation, whose root below
/// the store root is `served_root` (empty for the original, D-3350), and
/// checks it against `bars`. `store::provenance::read` resolves the same
/// generation; the path named is the one it read.
fn read_sidecar(
    root: &Path,
    bars_path: StorePath<'_>,
    bars: &BarFile,
    served_root: &Path,
) -> SidecarRead {
    let prv = bars_path.with_file(FileKind::Provenance);
    let path = prv.to_path_buf(served_root).display().to_string();
    match store::provenance::read(root, bars_path) {
        Ok(Found::Absent) => SidecarRead::Absent { path },
        Ok(Found::Unborn { bytes }) => SidecarRead::Unborn { path, bytes },
        Ok(Found::Present(sidecar)) => match sidecar.check_bars(&root.join(&path), bars) {
            Ok(()) => SidecarRead::Present { path, sidecar },
            Err(why) => SidecarRead::Refused {
                path,
                why: why.to_string(),
            },
        },
        Err(why) => SidecarRead::Refused {
            path,
            why: why.to_string(),
        },
    }
}

/// The bar ranges `[from, to)` below `n_valid` no record covers: before the
/// first, between two, after the last. Records are in bar order and do not
/// overlap (`store::provenance` refuses a sidecar whose records do), so this
/// is one pass over at most `MAX_RECORDS` records.
fn unrecorded(records: &[Record], n_valid: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::with_capacity(records.len().saturating_add(1));
    let mut at = 0u64;
    for record in records {
        if record.first_index > at {
            out.push((at, record.first_index.min(n_valid)));
        }
        at = at.max(record.end());
    }
    if at < n_valid {
        out.push((at, n_valid));
    }
    out.retain(|&(from, to)| from < to);
    out
}

/// The journals an answer cross-reads, each read at most once per request.
struct Journals {
    /// The GDFL import journal, for a GDFL month.
    gdfl: Option<Result<super::import_journal::Snapshot, String>>,
    /// The pull journal, for every other feed, and its whole records.
    pull: Option<(audit::Journal, u64)>,
}

impl Journals {
    /// The journal `asked.feed` lands through.
    fn new(root: &Path, asked: &Asked) -> Self {
        if asked.feed == Vendor::Gdfl {
            Self {
                gdfl: Some(super::import_journal::snapshot(root)),
                pull: None,
            }
        } else {
            let journal = audit::Journal::at(root);
            let records = journal.look().records();
            Self {
                gdfl: None,
                pull: Some((journal, records)),
            }
        }
    }
}

/// One sidecar record, with the journal record it names read beside it.
fn record_json(record: &Record, journals: &Journals, asked: &Asked) -> Value {
    let landed_secs = record.landed_at_micros.div_euclid(MICROS);
    json!({
        "ordinal": record.ordinal,
        "feed": record.vendor.as_str(),
        "source_kind": record.source_kind.as_str(),
        "source_name": record.source_name,
        "source_name_truncated": record.source_name_truncated,
        "source_name_blake3": record.source_name_blake3_hex(),
        "source_bytes": record.source_bytes,
        "source_crc32": (record.source_kind == SourceKind::ArchiveMember)
            .then(|| format!("{:08x}", record.source_crc32)),
        "day": record.day.to_string(),
        "first_index": record.first_index,
        "n_bars": record.n_bars,
        "end": record.end(),
        "bar_definition": record.bar_definition,
        "journal_ordinal": record.journal_ordinal,
        "bar_file_generation": record.bar_file_generation,
        "commit": record.commit_hex(),
        "commit_stamped": record.commit.is_some(),
        "landed_at_micros": record.landed_at_micros,
        "landed_at": ist_stamp(landed_secs),
        "journal": journal_of_record(record, journals, asked),
    })
}

/// The journal record a sidecar record names: the GDFL import journal's row
/// for its day, or the pull journal's record at its ordinal (one positional
/// read).
fn journal_of_record(record: &Record, journals: &Journals, asked: &Asked) -> Value {
    if let Some(snapshot) = &journals.gdfl {
        let day = Day::new(record.day.year(), record.day.month(), record.day.day()).ok();
        return match (snapshot, day) {
            (Err(why), _) => json!({ "source": "imports/gdfl.journal", "error": why }),
            (Ok(_), None) => json!({
                "source": "imports/gdfl.journal",
                "error": format!("{} is not a day the calendar can name", record.day),
            }),
            (Ok(snapshot), Some(day)) => {
                let kind = import_kind(asked.segment);
                let rows: Vec<Value> = snapshot
                    .parsed
                    .range(day, day)
                    .filter(|row| row.kind == kind && filter_holds(&row.filter, &asked.symbol))
                    .map(|row| day_json(row, asked, None, None))
                    .collect();
                json!({
                    "source": "imports/gdfl.journal",
                    "line_named": record.journal_ordinal,
                    "day_rows": rows,
                })
            }
        };
    }
    let Some((journal, records)) = &journals.pull else {
        return Value::Null;
    };
    if record.journal_ordinal >= *records {
        return json!({
            "source": "audit/pull.journal",
            "error": format!(
                "record {} is past the journal's {records} records",
                record.journal_ordinal
            ),
        });
    }
    match journal.page(record.journal_ordinal.saturating_add(1), 0, 1) {
        Ok(entries) => match entries.first() {
            Some(entry) => match &entry.decoded {
                Ok(found) => json!({
                    "source": "audit/pull.journal",
                    "record": pull_record_json(entry.ordinal, found, None),
                }),
                Err(fault) => json!({
                    "source": "audit/pull.journal",
                    "ordinal": entry.ordinal,
                    "error": fault.to_string(),
                }),
            },
            None => json!({
                "source": "audit/pull.journal",
                "error": format!("record {} was not read", record.journal_ordinal),
            }),
        },
        Err(why) => json!({ "source": "audit/pull.journal", "error": why }),
    }
}

// ══ GDFL: rebuilt from the import journal ══════════════════════════════════

/// The import kind whose journal lines cover a segment.
const fn import_kind(segment: Segment) -> ImportKind {
    match segment {
        Segment::Index => ImportKind::Indices,
        Segment::Cash => ImportKind::Stocks,
        Segment::Fno => ImportKind::Options,
    }
}

/// Whether a journal key's filter (`*`, or the sorted names a run was given)
/// takes `symbol`. Bounded by the filter's own text.
fn filter_holds(filter: &str, symbol: &str) -> bool {
    filter == "*" || filter.split(',').any(|name| name == symbol)
}

/// The month (or `t`'s day) rebuilt from `imports/gdfl.journal`, each import
/// day beside the bars of this month stamped on that day. `served` is the
/// generation served (D-3350): a repair revision is written in one commit,
/// so its commits are not compared with the journal's days.
fn gdfl_reconstruction(journals: &Journals, asked: &Asked, bars: &BarFile, served: u32) -> Value {
    let source = "imports/gdfl.journal";
    let snapshot = match &journals.gdfl {
        Some(Ok(snapshot)) => snapshot,
        Some(Err(why)) => return json!({ "source": source, "error": why }),
        None => return json!({ "source": source, "error": "the import journal was not read" }),
    };
    let Some((from, to)) = asked.days() else {
        return json!({ "source": source, "error": "the month names no calendar day" });
    };
    let header = bars.header();
    let generation = header.generation;
    let kind = import_kind(asked.segment);
    let parsed = &snapshot.parsed;
    let rows: Vec<&DayRow> = parsed
        .range(from, to)
        .filter(|row| row.kind == kind && filter_holds(&row.filter, &asked.symbol))
        .collect();
    let scope = cm_scope(asked);
    // THE BARS EACH IMPORT DAY HOLDS, by two time lookups a day (through the
    // month's `.tix`, one entry each; else the D-1434 bisection), so every bar
    // of the month is tied to the journal line of the day it is stamped on, and
    // a bar on a day the journal does not name is counted, never hidden.
    let mut spans = DaySpans::with_capacity(rows.len());
    let days: Vec<Value> = rows
        .iter()
        .map(|row| {
            let span = spans.of(bars, row.day);
            day_json(row, asked, scope.as_ref(), Some(span))
        })
        .collect();
    let (held_on_days, lookup_errors) = spans.totals();
    let with_bars = spans.days_with_bars();
    let done = rows.iter().filter(|row| row.outcome.is_done()).count();
    let mut runs: Vec<Value> = Vec::with_capacity(rows.len().min(4));
    let mut named: Vec<u64> = Vec::with_capacity(rows.len().min(4));
    let mut unnamed = 0usize;
    let mut missing = 0usize;
    for row in &rows {
        match row.stats.run {
            None => unnamed = unnamed.saturating_add(1),
            Some(id) if named.iter().all(|seen| *seen != id) => {
                named.push(id);
                let line = run_line_json(parsed, id);
                if line.get("error").is_some() {
                    missing = missing.saturating_add(1);
                }
                runs.push(line);
            }
            Some(_) => {}
        }
    }
    let whole_month = asked.t.is_none();
    // A REPAIR REVISION'S ONE COMMIT IS THE REPAIR'S, not an import day's
    // (D-3350; DB-N26): no comparison, and the reason in its place.
    let compared = whole_month && served == 0;
    json!({
        "source": source,
        "journal": {
            "path": source,
            "present": snapshot.present,
            "bytes": snapshot.bytes,
            "lines": parsed.lines,
            "foreign": parsed.foreign_count,
            "torn": parsed.torn_count,
            "unterminated_bytes": snapshot.unterminated,
        },
        "kind": kind.as_str(),
        "days": days,
        "journal_days": rows.len(),
        "journal_days_done": done,
        "journal_days_with_bars": with_bars,
        "bars_on_journal_days": held_on_days,
        "bars_on_no_journal_day": whole_month.then(|| header.n_valid.saturating_sub(held_on_days)),
        "bar_lookup_errors": lookup_errors,
        "bar_file_commits": generation,
        "commits_agree": compared.then(|| u64::try_from(with_bars).ok() == Some(generation)),
        // ONE COMMIT PER IMPORT DAY THAT FILED A BAR, as measured on the
        // operator's store on 10 Oct 2026: GDFL NIFTY 1s 2019-03 is at
        // generation 19, and 19 journal days of that month hold its bars.
        "commits_why": if served > 0 {
            whole_month.then_some(
                "the month is served from a repair revision (D-3350), written whole in one \
                 commit by the repair that published it, so its commits do not count the \
                 import days that landed its bars; audit/gdfl.repairs names each repair",
            )
        } else {
            whole_month.then_some(
                "an import day that files bars for this instrument appends them to the month in \
                 one commit, so the commits equal the journal days holding its bars; a re-run \
                 that appended part of a day again, or bars landed by something other than the \
                 import, makes them differ",
            )
        },
        "archive": {
            "recorded": unnamed == 0 && missing == 0 && !rows.is_empty(),
            "runs": runs,
            "days_naming_no_run": unnamed,
            "runs_named_and_absent": missing,
            "why": (unnamed > 0).then_some(
                "these days' closing lines name no run: they were written before D-3317, whose \
                 run line is what records a run's source (zip or tick store), archive path, \
                 size and digest, and build commit. Which archive, which member and its digest \
                 were not recorded (DB-G56); vendor_files names only where THIS build's reader \
                 finds the day"
            ),
        },
        "member_digest": "unknown",
    })
}

/// One journal row: the deciding line and its counts, and where this build's
/// reader finds the vendor's file of that day.
fn day_json(row: &DayRow, asked: &Asked, scope: Option<&CmScope>, bars: Option<Span>) -> Value {
    let stats = &row.stats;
    let bars = bars.map(|span| match span {
        Ok((first, end)) => json!({
            "first_index": first,
            "end": end,
            "count": end.saturating_sub(first),
        }),
        Err(why) => json!({ "error": why }),
    });
    json!({
        "day": row.day.to_string(),
        "kind": row.kind.as_str(),
        "filter": row.filter,
        "outcome": row.outcome.as_str(),
        "line": row.line,
        "attempts": row.attempts,
        "closes": row.closes,
        "recounted": row.recounted,
        "previous": row.previous.map(DayOutcome::as_str),
        "definition": stats.definition,
        "files": stats.files,
        "seconds": stats.seconds,
        "failures": stats.failures,
        "late": stats.late,
        "late_unresolved": stats.late_unresolved,
        "max_back_s": stats.max_back_s,
        "vendor_refused": stats.vendor_refused,
        "months": stats.months,
        "run": stats.run,
        "stage": row.stage,
        "reason": row.reason,
        "bars": bars,
        "vendor_files": vendor_files(asked, row.kind, row.day, scope),
    })
}

/// `[first, end)` of the bars stamped on one day, or why they could not be
/// found.
type Span = Result<(u64, u64), String>;

/// The bars of a month stamped on each day asked, each day looked up once:
/// the first bar at or after its IST midnight and the first at or after the
/// next day's.
struct DaySpans {
    /// Each day looked up, with its span or why it could not be found.
    seen: Vec<(Day, Span)>,
}

impl DaySpans {
    /// Room for `days` days.
    fn with_capacity(days: usize) -> Self {
        Self {
            seen: Vec::with_capacity(days),
        }
    }

    /// The span of bars stamped on `day`. A day already looked up (two
    /// journal filters closing one day) is answered from the last entry:
    /// rows come in day order.
    ///
    /// THE PREVIOUS DAY'S END IS THIS DAY'S START when the bar there is
    /// stamped on or after this day's midnight: every bar before it is earlier
    /// than the previous day's next midnight, so no bar lies between. One bar
    /// read then stands for one time lookup, which on a month with no `.tix`
    /// is a bisection.
    fn of(&mut self, bars: &BarFile, day: Day) -> Span {
        let previous_end = match self.seen.last() {
            Some((last, span)) if *last == day => return span.clone(),
            Some((last, Ok((_, end)))) if *last < day => Some(*end),
            _ => None,
        };
        let start = ist_midnight_micros(day);
        let first = match previous_end {
            Some(end) if starts_at_or_after(bars, end, start) => Ok(end),
            _ => bars
                .first_at_or_after(start)
                .map_err(|why| format!("{day}: {why}")),
        };
        let span = first.and_then(|first| {
            bars.first_at_or_after(start.saturating_add(86_400 * MICROS))
                .map(|end| (first, end.max(first)))
                .map_err(|why| format!("{day}: {why}"))
        });
        self.seen.push((day, span.clone()));
        span
    }

    /// Bars on the days looked up, and how many days could not be.
    fn totals(&self) -> (u64, u64) {
        self.seen
            .iter()
            .fold((0, 0), |(bars, errors), (_, span)| match span {
                Ok((first, end)) => (bars.saturating_add(end.saturating_sub(*first)), errors),
                Err(_) => (bars, errors.saturating_add(1)),
            })
    }

    /// Days with at least one bar.
    fn days_with_bars(&self) -> usize {
        self.seen
            .iter()
            .filter(|(_, span)| matches!(span, Ok((first, end)) if end > first))
            .count()
    }
}

/// Whether bar `index` is past the month's last bar or stamped at or after
/// `micros`: one bar read, and a bar that cannot be read answers `false`, so
/// the caller looks the time up instead.
fn starts_at_or_after(bars: &BarFile, index: u64, micros: i64) -> bool {
    index >= bars.header().n_valid
        || bars
            .read_record(index)
            .is_ok_and(|bar| bar.ts_micros >= micros)
}

/// IST midnight opening `day`, in epoch microseconds.
fn ist_midnight_micros(day: Day) -> i64 {
    i64::from(day.days_from_epoch())
        .saturating_mul(86_400)
        .saturating_sub(pull::session::IST_OFFSET_SECS)
        .saturating_mul(MICROS)
}

/// The capital-market scope that names an instrument's file stem: the swept
/// surface, the alias table's stored indices and renamed shares, and this
/// share. `None` for an option, whose file is named by its vendor ticker.
fn cm_scope(asked: &Asked) -> Option<CmScope> {
    match asked.segment {
        Segment::Index => CmScope::stored(std::iter::empty::<&str>()).ok(),
        Segment::Cash => CmScope::stored([asked.symbol.as_str()]).ok(),
        Segment::Fno => None,
    }
}

/// Where this build's readers find the vendor's file of `day` for this
/// instrument: the day folder, the day zip inside the capital-market outer
/// archive and the tick-store day file (`pull::gdfl_archive::day_zip_name`,
/// `pull::gdfl_tickstore::TickStore::day_path`), and the file's stem. Names a
/// layout, never a recorded fact: the answer says so.
fn vendor_files(asked: &Asked, kind: ImportKind, day: Day, scope: Option<&CmScope>) -> Value {
    let cm = match kind {
        ImportKind::Indices => CmKind::Indices,
        ImportKind::Stocks => CmKind::Stocks,
        ImportKind::Options => {
            return json!({
                "day_folder": pull::gdfl_nfo::day_folder_name(day),
                "stem": Value::Null,
                "named_by": "this build's options reader (pull::gdfl_nfo); a contract's file is \
                             named by its vendor ticker, which the import did not record",
            });
        }
    };
    let key = match asked.segment {
        Segment::Index => InstrumentKey::index(asked.exchange, &asked.symbol).ok(),
        Segment::Cash => InstrumentKey::cash(asked.exchange, &asked.symbol).ok(),
        Segment::Fno => None,
    };
    let stem = key
        .zip(scope)
        .and_then(|(key, scope)| scope.stem_for(&key, day))
        .map(|(_, stem)| stem);
    json!({
        "day_folder": pull::gdfl_cm::day_folder_name(cm, day),
        "zip_day": pull::gdfl_archive::day_zip_name(cm, day),
        "tickstore_day": pull::gdfl_tickstore::TickStore::new(Path::new(""))
            .day_path(cm, day)
            .display()
            .to_string(),
        "stem": stem,
        "named_by": "this build's capital-market readers (pull::gdfl_archive, \
                     pull::gdfl_tickstore, pull::gdfl_cm); the member is the day folder's \
                     <stem>.csv or <stem>.CSV, and its size and CRC-32 were not recorded",
    })
}

/// The `run` line a close names, by its id, or why it cannot be shown.
fn run_line_json(parsed: &Arc<Parsed>, id: u64) -> Value {
    match run_line(parsed, id) {
        Some(run) => {
            let mut fields = serde_json::Map::with_capacity(run.fields.len());
            for (name, value) in &run.fields {
                fields.insert(name.clone(), Value::String(value.clone()));
            }
            json!({ "id": run.id, "line": run.line, "fields": fields })
        }
        None => json!({
            "id": id,
            "line": Value::Null,
            "error": format!(
                "a closing line names run {id} and the journal holds no run line with that id"
            ),
        }),
    }
}

/// One snapshot's run lines, by id.
struct RunIds {
    /// The snapshot the map was built from. A `Weak` keeps its allocation
    /// from being reused, so a pointer match means the same snapshot.
    of: Weak<Parsed>,
    /// Run id to position in `Parsed::runs`.
    at: HashMap<u64, usize>,
}

/// The run-id index of the last snapshot looked at.
fn run_ids() -> &'static Mutex<Option<RunIds>> {
    static HELD: OnceLock<Mutex<Option<RunIds>>> = OnceLock::new();
    HELD.get_or_init(|| Mutex::new(None))
}

/// The run line with id `id`: one hash probe, after the index is rebuilt
/// when the snapshot is not the one it was built from (the journal grew or
/// was replaced) -- O(run lines) once per such change.
fn run_line(parsed: &Arc<Parsed>, id: u64) -> Option<RunLine> {
    let mut held = run_ids().lock().unwrap_or_else(PoisonError::into_inner);
    let fresh = held
        .as_ref()
        .is_some_and(|ids| std::ptr::eq(ids.of.as_ptr(), Arc::as_ptr(parsed)));
    if !fresh {
        let mut at = HashMap::with_capacity(parsed.runs.len().max(1));
        for (position, run) in parsed.runs.iter().enumerate() {
            at.insert(run.id, position);
        }
        *held = Some(RunIds {
            of: Arc::downgrade(parsed),
            at,
        });
    }
    let position = held.as_ref()?.at.get(&id).copied()?;
    parsed
        .runs
        .get(position)
        .filter(|run| run.id == id)
        .cloned()
}

// ══ A pulled feed: rebuilt from the pull journal ═══════════════════════════

/// One run record of the pull journal, with the feed its source names.
#[derive(Debug, Clone)]
struct PullRun {
    /// Its ordinal in the journal.
    ordinal: u64,
    /// The feed its source names, or `None` (a run refused before any vendor
    /// was asked names none).
    vendor: Option<Vendor>,
    /// The record.
    record: audit::Record,
}

impl PullRun {
    /// The UNIX seconds it started and, at the latest, ended: every writer
    /// stamps a run record with the time taken before the pull began
    /// (`server::recovery_spot`, `autopilot`) and its measured duration.
    fn span(&self) -> (i64, i64) {
        let start = self.record.at_unix_secs;
        let took =
            i64::try_from(self.record.elapsed_micros.div_ceil(1_000_000)).unwrap_or(i64::MAX);
        (start, start.saturating_add(took))
    }

    /// Whether the run was under way at `at`, UNIX seconds; `false` for an
    /// instant the host did not give.
    fn ran_at(&self, at: Option<i64>) -> bool {
        let (start, end) = self.span();
        at.is_some_and(|at| start <= at && at <= end)
    }
}

/// Every run record of one pull journal, and which months each window covers.
#[derive(Debug, Clone)]
struct RunIndex {
    /// Run records, in file order.
    runs: Vec<PullRun>,
    /// Month number (`year * 12 + month - 1`) to positions in `runs`.
    by_month: HashMap<u32, Vec<usize>>,
    /// Positions of runs whose window could not be filed by month: wider than
    /// [`MAX_INDEXED_MONTHS`], reversed, or past the calendar. Every request
    /// walks them.
    unindexed: Vec<usize>,
    /// Runs that name no window (a request refused before it had one).
    unwindowed: u64,
    /// Records whose kind byte names a run and that will not decode, or whose
    /// kind byte this reader does not know.
    faults: u64,
}

impl RunIndex {
    /// An empty index.
    fn empty() -> Self {
        Self {
            runs: Vec::with_capacity(16),
            by_month: HashMap::with_capacity(256),
            unindexed: Vec::with_capacity(1),
            unwindowed: 0,
            faults: 0,
        }
    }

    /// Files one run under every month its window covers.
    fn push(&mut self, run: PullRun) {
        let position = self.runs.len();
        let (from, to) = (run.record.from_days, run.record.to_days);
        if from == 0 && to == 0 {
            self.unwindowed = self.unwindowed.saturating_add(1);
        } else {
            match (month_number_of_days(from), month_number_of_days(to)) {
                (Some(first), Some(last)) if first <= last && last - first < MAX_INDEXED_MONTHS => {
                    for month in first..=last {
                        self.by_month
                            .entry(month)
                            .or_insert_with(|| Vec::with_capacity(1))
                            .push(position);
                    }
                }
                _ => self.unindexed.push(position),
            }
        }
        self.runs.push(run);
    }
}

/// `year * 12 + month - 1` of the day `days` after the epoch.
fn month_number_of_days(days: u32) -> Option<u32> {
    let day = Day::from_days(days).ok()?;
    Some(month_number(day.year(), day.month()))
}

/// `year * 12 + month - 1`.
fn month_number(year: u16, month: u8) -> u32 {
    u32::from(year)
        .saturating_mul(12)
        .saturating_add(u32::from(month))
        .saturating_sub(1)
}

/// What the last read of one pull journal left.
struct RunTail {
    /// The file's device and inode when read: a replaced file is read again.
    identity: Option<(u64, u64)>,
    /// Records read.
    upto: u64,
    /// What they hold.
    index: Arc<RunIndex>,
}

/// Every pull journal read so far, by path: a test's store and the served
/// store never share an index.
fn run_tails() -> &'static Mutex<HashMap<PathBuf, RunTail>> {
    static TAILS: OnceLock<Mutex<HashMap<PathBuf, RunTail>>> = OnceLock::new();
    TAILS.get_or_init(|| Mutex::new(HashMap::with_capacity(4)))
}

/// One read of the pull journal.
struct RunsRead {
    /// Whether the file is there.
    present: bool,
    /// Its length.
    bytes: u64,
    /// Whole records read.
    records: u64,
    /// Bytes past the last whole record: a torn write, named.
    torn: u64,
    /// Records this call read: all of them the first time.
    read_now: u64,
    /// How long this call took, in microseconds.
    read_micros: u64,
    /// The index.
    index: Arc<RunIndex>,
}

/// The pull journal under `root`, read through its tail. See the module
/// header for the cost.
///
/// # Errors
///
/// A file that is there and cannot be measured or read, in its own words.
fn read_runs(root: &Path) -> Result<RunsRead, String> {
    use std::io::{Read as _, Seek as _};
    use std::os::unix::fs::MetadataExt as _;
    let started = std::time::Instant::now();
    let path = audit::journal_path(root);
    let meta = match std::fs::metadata(&path) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RunsRead {
                present: false,
                bytes: 0,
                records: 0,
                torn: 0,
                read_now: 0,
                read_micros: 0,
                index: Arc::new(RunIndex::empty()),
            });
        }
        Err(why) => return Err(format!("{}: {why}", path.display())),
        Ok(meta) => meta,
    };
    let identity = Some((meta.dev(), meta.ino()));
    let len = meta.len();
    let whole = len / audit::RECORD_LEN_U64;
    let mut held = run_tails().lock().unwrap_or_else(PoisonError::into_inner);
    let fresh = || RunTail {
        identity,
        upto: 0,
        index: Arc::new(RunIndex::empty()),
    };
    let tail = held.entry(path.clone()).or_insert_with(fresh);
    // A FILE THAT SHRANK OR IS ANOTHER FILE IS READ AGAIN FROM THE START. The
    // journal is append-only, so neither happens to the one the server
    // writes; a restored copy or a test's fixture is exactly that case.
    if tail.identity != identity || whole < tail.upto {
        *tail = fresh();
    }
    let mut read_now = 0u64;
    if whole > tail.upto {
        let fail = |why: std::io::Error| format!("{}: {why}", path.display());
        let mut file = std::fs::File::open(&path).map_err(fail)?;
        file.seek(std::io::SeekFrom::Start(
            tail.upto.saturating_mul(audit::RECORD_LEN_U64),
        ))
        .map_err(fail)?;
        let index = Arc::make_mut(&mut tail.index);
        let mut buffer = Vec::new();
        while tail.upto < whole {
            let want = whole.saturating_sub(tail.upto).min(CHUNK_RECORDS);
            buffer.clear();
            (&mut file)
                .take(want.saturating_mul(audit::RECORD_LEN_U64))
                .read_to_end(&mut buffer)
                .map_err(fail)?;
            let got = u64::try_from(buffer.len()).unwrap_or(u64::MAX) / audit::RECORD_LEN_U64;
            if got == 0 {
                return Err(format!(
                    "{}: the file ended before record {} of {whole}",
                    path.display(),
                    tail.upto
                ));
            }
            for (at, image) in (tail.upto..).zip(buffer.chunks_exact(audit::RECORD_LEN)) {
                fold_record(index, at, image);
            }
            tail.upto = tail.upto.saturating_add(got);
            read_now = read_now.saturating_add(got);
        }
    }
    Ok(RunsRead {
        present: true,
        bytes: len,
        records: tail.upto,
        torn: len % audit::RECORD_LEN_U64,
        read_now,
        read_micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
        index: Arc::clone(&tail.index),
    })
}

/// Folds one 256-byte record into the index: a run is decoded and filed; a
/// member failure is passed over by its kind byte alone.
fn fold_record(index: &mut RunIndex, ordinal: u64, image: &[u8]) {
    let Ok(image) = <&[u8; audit::RECORD_LEN]>::try_from(image) else {
        return;
    };
    let kind = image.get(KIND_AT).copied();
    if kind == Some(audit::Kind::MemberFailure.code()) {
        return;
    }
    if kind != Some(audit::Kind::Run.code()) {
        index.faults = index.faults.saturating_add(1);
        return;
    }
    match audit::Record::decode(image) {
        Ok(record) if record.kind == audit::Kind::Run => index.push(PullRun {
            ordinal,
            vendor: crate::audit_json::vendor_of_source(&record.source),
            record,
        }),
        _ => index.faults = index.faults.saturating_add(1),
    }
}

/// Why a run covering the month is not a candidate, counted.
#[derive(Debug, Default)]
struct Excluded {
    /// Its source names another feed.
    other_feed: u64,
    /// Its source names no feed: a run refused before any vendor was asked.
    unattributed: u64,
    /// It was the other form: spot for an option, expired F&O for spot.
    other_scope: u64,
    /// Its window misses `t`'s day.
    other_day: u64,
    /// It was refused or never started.
    not_run: u64,
    /// It offered no bar.
    offered_nothing: u64,
}

/// The audit scope a segment is pulled under.
const fn scope_of(segment: Segment) -> audit::Scope {
    match segment {
        Segment::Fno => audit::Scope::Fno,
        Segment::Index | Segment::Cash => audit::Scope::Spot,
    }
}

/// The month (or `t`'s day) rebuilt from `audit/pull.journal`. `generation`
/// is the served bar file's commit count and `served` the generation served
/// (D-3350): a repair revision's one commit and modification time are the
/// repair's, so no pull run is named its writer.
fn pull_reconstruction(
    root: &Path,
    asked: &Asked,
    (generation, served): (u64, u32),
    modified: Option<i64>,
    notes: &mut Vec<String>,
) -> Value {
    let source = "audit/pull.journal";
    let read = match read_runs(root) {
        Ok(read) => read,
        Err(why) => return json!({ "source": source, "error": why }),
    };
    if read.read_now > FIRST_WALK_RECORDS {
        notes.push(format!(
            "the pull journal keeps no index of its runs, so this process's first read of it \
             decoded {} records in {} ms; later requests read only what is appended",
            read.read_now,
            read.read_micros / 1_000
        ));
    }
    let Some((from, to)) = asked.days() else {
        return json!({ "source": source, "error": "the month names no calendar day" });
    };
    let index = &read.index;
    let month = month_number(asked.month.year(), asked.month.month());
    let filed = index.by_month.get(&month).map_or(&[][..], Vec::as_slice);
    let Sorted {
        candidates,
        closing,
        opened,
        continuations,
        excluded,
    } = sorted(index, filed, asked, (from, to));
    let unclosed = unclosed(&opened, &closing);
    let unclosed_before_write = unclosed
        .iter()
        .any(|open| modified.is_some_and(|at| open.record.at_unix_secs <= at));
    let running: Vec<u64> = candidates
        .iter()
        .filter(|run| run.ran_at(modified))
        .map(|run| run.ordinal)
        .collect();
    let attributed = match (
        served,
        generation,
        running.as_slice(),
        unclosed_before_write,
    ) {
        (0, 1, [only], false) => json!({
            "run": only,
            "basis": "the bar file committed once (generation 1), and its modification time \
                      falls inside this run and no other candidate's",
        }),
        _ => Value::Null,
    };
    json!({
        "source": source,
        "journal": {
            "path": source,
            "present": read.present,
            "bytes": read.bytes,
            "records": read.records,
            "torn_bytes": read.torn,
            "runs": index.runs.len(),
            "runs_without_window": index.unwindowed,
            "runs_walked_on_every_request": index.unindexed.len(),
            "undecodable": index.faults,
            "read_now": read.read_now,
        },
        "candidates": candidates
            .iter()
            .map(|run| pull_record_json(run.ordinal, &run.record, Some((run, modified))))
            .collect::<Vec<_>>(),
        "opened_never_closed": unclosed
            .iter()
            .map(|run| pull_record_json(run.ordinal, &run.record, Some((run, modified))))
            .collect::<Vec<_>>(),
        "opened_and_closed": opened.len().saturating_sub(unclosed.len()),
        "opened_again_after_another_writer": continuations,
        "excluded": {
            "other_feed": excluded.other_feed,
            "names_no_feed": excluded.unattributed,
            "other_scope": excluded.other_scope,
            "window_misses_t": excluded.other_day,
            "refused_or_not_started": excluded.not_run,
            "offered_no_bar": excluded.offered_nothing,
        },
        "bar_file_commits": generation,
        "last_write": {
            "modified_unix": modified,
            "modified": modified.map(ist_stamp),
            "inside_runs": running,
        },
        "attributed": attributed,
        "why": "a pull record names its window and its request, never the instrument-months it \
                wrote; these are the runs of this feed and form whose window covers the bars \
                and that offered bars, oldest first. One is named the writer only when the bar \
                file committed once and was last written inside that run alone, and no run \
                opened before that write is left without a closing record; never for a month \
                served from a repair revision (D-3350), whose one commit is the repair's",
        "member_digest": "unknown",
    })
}

/// The runs that cover a month's bars, sorted by what they can say about them.
struct Sorted<'a> {
    /// Closing records of this feed and form that offered bars: who may have
    /// written them.
    candidates: Vec<&'a PullRun>,
    /// Every closing record of this feed and form, whatever its outcome.
    closing: Vec<&'a PullRun>,
    /// Opening records of this feed and form (D-3341), one per run.
    opened: Vec<&'a PullRun>,
    /// Opening records written again after another writer: the same runs.
    continuations: u64,
    /// The rest, counted by why.
    excluded: Excluded,
}

/// Sorts the runs at `filed` and every unfiled one: those whose window misses
/// the days `(from, to)`, another feed's or none's, the other form's; then
/// opening records apart from closing ones, and of the closing ones those that
/// offered bars. Positions come in file order, so every list is oldest first.
fn sorted<'a>(
    index: &'a RunIndex,
    filed: &[usize],
    asked: &Asked,
    (from, to): (Day, Day),
) -> Sorted<'a> {
    let mut out = Sorted {
        candidates: Vec::with_capacity(filed.len()),
        closing: Vec::with_capacity(filed.len()),
        opened: Vec::with_capacity(2),
        continuations: 0,
        excluded: Excluded::default(),
    };
    let excluded = &mut out.excluded;
    for position in ascending(filed, &index.unindexed) {
        let Some(run) = index.runs.get(position) else {
            continue;
        };
        let record = &run.record;
        if record.to_days < from.days_from_epoch() || record.from_days > to.days_from_epoch() {
            excluded.other_day = excluded.other_day.saturating_add(1);
        } else if run.vendor.is_none() {
            excluded.unattributed = excluded.unattributed.saturating_add(1);
        } else if run.vendor != Some(asked.feed) {
            excluded.other_feed = excluded.other_feed.saturating_add(1);
        } else if record.scope != scope_of(asked.segment) {
            excluded.other_scope = excluded.other_scope.saturating_add(1);
        } else if record.outcome == audit::Outcome::Opened {
            // A RUN'S OPENING RECORD IS NOT A VERDICT (D-3341): its counters
            // are zero and its closing record carries what landed. Kept apart
            // and paired with its close. The copy a run writes again when
            // another writer interleaved (`audit::Journal::append_failure`)
            // is the same run, not another.
            if record.note.starts_with(CONTINUES) {
                out.continuations = out.continuations.saturating_add(1);
            } else {
                out.opened.push(run);
            }
        } else {
            out.closing.push(run);
            if matches!(
                record.outcome,
                audit::Outcome::Refused | audit::Outcome::NotStarted
            ) {
                excluded.not_run = excluded.not_run.saturating_add(1);
            } else if record.outcome == audit::Outcome::Empty || record.bars_stored == 0 {
                excluded.offered_nothing = excluded.offered_nothing.saturating_add(1);
            } else {
                out.candidates.push(run);
            }
        }
    }
    out
}

/// The opening records no closing record answers.
///
/// AN OPENED RUN WITH NO CLOSE MAY HAVE LANDED BARS AND SAID NOTHING OF
/// THEM: its process stopped before its closing record. Each open is paired
/// with the first unpaired closing record after it, of the same feed, form
/// and window, that was under way when it was opened (the close is stamped
/// with the time its request arrived, the open a moment later). An unclosed
/// one has no end, so it is listed, never named the writer, and keeps any run
/// from being named once it began before the file's last write. Bounded by
/// the runs that cover the month, both lists.
fn unclosed<'a>(opened: &[&'a PullRun], closing: &[&PullRun]) -> Vec<&'a PullRun> {
    let mut paired = vec![false; closing.len()];
    let mut out: Vec<&PullRun> = Vec::with_capacity(opened.len());
    for open in opened {
        let mut closed = false;
        for (close, taken) in closing.iter().zip(paired.iter_mut()) {
            if !*taken
                && close.ordinal > open.ordinal
                && close.record.from_days == open.record.from_days
                && close.record.to_days == open.record.to_days
                && close.ran_at(Some(open.record.at_unix_secs))
            {
                *taken = true;
                closed = true;
                break;
            }
        }
        if !closed {
            out.push(open);
        }
    }
    out
}

/// The positions of two ascending lists, ascending: a month's filed runs and
/// the runs too wide to file, so candidates come oldest first with no sort.
fn ascending<'a>(filed: &'a [usize], wide: &'a [usize]) -> impl Iterator<Item = usize> + 'a {
    let (mut i, mut j) = (0usize, 0usize);
    std::iter::from_fn(
        move || match (filed.get(i).copied(), wide.get(j).copied()) {
            (Some(a), Some(b)) if a <= b => {
                i = i.saturating_add(1);
                Some(a)
            }
            (_, Some(b)) => {
                j = j.saturating_add(1);
                Some(b)
            }
            (Some(a), None) => {
                i = i.saturating_add(1);
                Some(a)
            }
            (None, None) => None,
        },
    )
}

/// One pull-journal record as the answer shows it; for a candidate, whether
/// the bar file was last written inside its run.
fn pull_record_json(
    ordinal: u64,
    record: &audit::Record,
    candidate: Option<(&PullRun, Option<i64>)>,
) -> Value {
    let started = record.at_unix_secs;
    let ended = started.saturating_add(
        i64::try_from(record.elapsed_micros.div_ceil(1_000_000)).unwrap_or(i64::MAX),
    );
    let day = |days: u32| {
        (days != 0)
            .then(|| Day::from_days(days).ok())
            .flatten()
            .map(|day| day.to_string())
    };
    json!({
        "ordinal": ordinal,
        "kind": record.kind.label(),
        "feed": candidate.and_then(|(run, _)| run.vendor).map(Vendor::as_str),
        "started_unix": started,
        "started": ist_stamp(started),
        "elapsed_micros": record.elapsed_micros,
        "ended": ist_stamp(ended),
        "scope": record.scope.label(),
        "outcome": record.outcome.label(),
        "window": { "from": day(record.from_days), "to": day(record.to_days) },
        "members": record.members,
        "rows_read": record.rows_read,
        "bars_offered": record.bars_stored,
        "failures": record.failures,
        "source": record.source,
        "source_bytes": record.source_bytes,
        "source_cut": record.source_was_cut(),
        "note": record.note,
        "note_cut": record.note_was_cut(),
        "running_at_last_write": candidate.map(|(run, modified)| run.ran_at(modified)),
    })
}

// ══ The operator's window ═══════════════════════════════════════════════════

/// The feed's operator window (D-3338), and whether the month lies in it.
fn window_json(asked: &Asked) -> Value {
    let Some(span) = pull::vendor::Feed::ALL
        .into_iter()
        .find(|feed| feed.store_vendor() == Some(asked.feed))
        .and_then(|feed| feed.descriptor().window)
    else {
        return Value::Null;
    };
    let Some((first, last)) = (Asked {
        t: None,
        ..asked.clone()
    })
    .days() else {
        return Value::Null;
    };
    let inside = span.holds(first) && span.holds(last);
    let touches = first.days_from_epoch() <= span.to.days_from_epoch()
        && last.days_from_epoch() >= span.from.days_from_epoch();
    json!({
        "from": span.from.to_string(),
        "to": span.to.to_string(),
        "standing": match span.standing {
            pull::vendor::SpanStanding::Operator => "operator",
            pull::vendor::SpanStanding::Measured => "measured",
        },
        "source": span.source,
        "inside": inside,
        "label": if inside {
            None
        } else if touches {
            Some(PARTLY_OUTSIDE_WINDOW)
        } else {
            Some(OUTSIDE_WINDOW)
        },
    })
}

#[cfg(test)]
#[path = "provenance_tests.rs"]
mod tests;
