//! Which trading days are a CASH security's own: its listing day and, when it
//! was expelled, the day its history ends. R4-17; findings Z5 and COR-07.
//!
//! # The source
//!
//! NSE's dated security master, `NSE_CM_security_ddmmyyyy.csv.gz`, carries
//! three lifecycle dates per EQ identity — `ListgDt`, `RmvlDt` and
//! `RadmssnDt`, seconds from 1980-01-01 (NSE Masters Data v1.6 §2.3 and
//! §3.1.1; `docs/00-charter.md`, "NSE Masters Data v1.6"). They are decoded
//! once, by [`crate::cash_auction`], and read here through the receipted cache
//! ([`crate::cash_session_cache::read_local_lifecycle`]): no second decoder,
//! no second epoch.
//!
//! # The key
//!
//! `(exchange, symbol, ISIN)`, exactly: a symbol whose master row carries
//! another ISIN is not that row's security, and a symbol reused for another
//! company is a different key. The landing knows a share's ISIN from
//! [`brutex_core::universe::nse_isin`], the same table the closing-auction
//! schedule is built from, so this holds no second symbol→ISIN map.
//!
//! # What is refused on its word, and what is not
//!
//! - A day before a master's listing day is not that security's
//!   ([`Span::before_listing`]): a bar on it is refused by name (D-5839).
//! - A day strictly after a master's removal day, with no readmission after
//!   it, is past the end of its history ([`Span::history_ended`]): a vendor's
//!   empty answer or 404 there is the end of history, not a failure (D-4991).
//!   The removal day itself is never past it (whether `RmvlDt` is the last day
//!   traded is UNVERIFIED, charter).
//! - A listing day the master does not state — `0` (TATACONSUM, TMPV), blank,
//!   absent, invalid, after the master's own date, or two rows disagreeing — is
//!   UNKNOWN, and nothing is refused on it ([`Span::first`] is `Err`).
//! - A name in no master on disk (DHFL, JETAIRWAYS: delisted before any master
//!   this store holds) gets a span INFERRED from the vendor's own day rung
//!   ([`Source::InferredFromVendorSpan`]). It is a label: an inferred span never
//!   refuses anything, because the vendor's own span cannot accuse the vendor's
//!   own rows.
//! - A share whose ISIN [`brutex_core::universe::nse_isin`] does not hold
//!   cannot be keyed, so no master is asked: it is labelled
//!   [`Source::NoIsin`], never "inferred" (which says a master was asked and
//!   omits it), and nothing is refused on it (R4-17 repair 5).
//!
//! # Which master
//!
//! The newest receipted one under `<store>/session-masters`. A newer master
//! whose receipt or bytes do not check is passed over BY NAME
//! ([`Listing::passed_over`]) and the next newest is used. A store with no
//! master that checks answers every probe "not in any master", and its shares
//! are labelled [`Source::NoCheckedMaster`], never
//! [`Source::InferredFromVendorSpan`]: with no master nothing is known of any
//! share's listing, which is not the same as a share every master omits.
//! Nothing is downloaded, created or repaired here.
//!
//! # Said out loud (R4-17 repair 3)
//!
//! [`for_store`] — the one door every landing and every scan takes — puts one
//! `warn` `pull.listing` event on `/logs` for each master it passed over,
//! naming the file, why, and the master that answers instead (or `none`), and
//! one `warn` when no master checks, saying the listing rule is off for every
//! CASH share. A table that loaded a master is remembered, so these are said
//! once per store per process; a table with no master is loaded again on each
//! call (AF-03) and says them again only when what it found differs from what
//! it last said for that store, so a pull of many answers is not one warn per
//! answer. Every `pull.identity` event names the master in use and its
//! SHA-256 ([`Listing::in_use`]), and `cli series-identity` prints both and
//! every master passed over.
//!
//! # Cost
//!
//! [`Listing::span`] is one expected-O(1) hash probe into the master's index
//! (parsed once at load) plus bounded key bytes. Loading lists the master directory once and parses one master: done
//! once per store root per process by [`for_store`], and only remembered when
//! a master was loaded (AF-03: a failed load is never remembered). Measured by
//! `crates/pull/benches/ratio/identity.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::cash_auction::{LifecycleDate, LifecycleIssue};
use crate::cash_session_cache::{VerifiedLifecycleMaster, read_local_lifecycle};
use crate::session::Day;

/// The directory under a store root that holds the dated masters.
pub const MASTER_DIR: &str = "session-masters";

/// The code a refusal of a bar before its security's listing carries.
pub const BEFORE_LISTING: &str = "before_listing";

/// The code an answer past the end of a delisted security's history carries.
pub const HISTORY_ENDS: &str = "history_ends_delisted";

/// Where a span came from. Every span names it; a receipt and a log line
/// carry [`Source::code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source<'a> {
    /// A receipted NSE security master.
    NseSecurityMaster {
        /// The master's file name under [`MASTER_DIR`].
        file: &'a str,
        /// The SHA-256 its receipt binds (of the compressed bytes).
        sha256: &'a [u8; 32],
    },
    /// A master checked and holds no row for this name: the vendor's own day
    /// rung, labelled.
    InferredFromVendorSpan,
    /// No master under the store checks (none on disk, the directory will not
    /// list, or every one failed its receipt): nothing is known of any share's
    /// listing, so the listing rule is off for every share, said by a `warn`
    /// ([`for_store`]). Never a span's source; the label of every share then.
    NoCheckedMaster,
    /// A master checks, but [`brutex_core::universe::nse_isin`] holds no ISIN
    /// for the share: the key `(exchange, symbol, ISIN)` cannot be formed, so
    /// no master is asked and nothing is known of its listing (R4-17 repair
    /// 5). It was labelled [`Source::InferredFromVendorSpan`], which says a
    /// master was asked and omits the share — untrue when the master may hold
    /// it. Never a span's source; a label, it refuses nothing.
    NoIsin,
}

impl Source<'_> {
    /// The code a receipt line and a log event carry.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NseSecurityMaster { .. } => "nse_security_master",
            Self::InferredFromVendorSpan => "inferred_from_vendor_span",
            Self::NoCheckedMaster => NO_CHECKED_MASTER,
            Self::NoIsin => NO_ISIN,
        }
    }
}

/// The label of every CASH share of a store with no master that checks.
pub const NO_CHECKED_MASTER: &str = "no_checked_master";

/// The label of a CASH share whose ISIN the universe does not hold, beside a
/// master that checks ([`Source::NoIsin`]).
pub const NO_ISIN: &str = "no_isin";

/// The target of the events [`for_store`] says a master's fate on.
pub const EVENT_TARGET: &str = "pull.listing";

/// Lower-case hex of a master's SHA-256, as a log line and a report print it.
#[must_use]
pub fn sha256_hex(sha256: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in sha256 {
        // `write!` to a `String` cannot fail.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Why a listing day is unknown. Nothing is refused on an unknown day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unknown {
    /// The master says `0` (TATACONSUM, TMPV): no date.
    EpochZero,
    /// The cell is blank.
    Blank,
    /// The master has no `ListgDt` column.
    ColumnAbsent,
    /// The cell is not a date this build reads.
    Invalid,
    /// Two rows of the same identity disagree.
    Conflicting,
    /// The date is after the master's own day: not an event this snapshot proves.
    AfterMaster,
    /// The master's dates contradict each other (a removal before the listing).
    Contradictory,
}

impl Unknown {
    /// The code a receipt line and a log event carry.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::EpochZero => "listing_epoch_zero",
            Self::Blank => "listing_blank",
            Self::ColumnAbsent => "listing_column_absent",
            Self::Invalid => "listing_invalid",
            Self::Conflicting => "listing_conflicting",
            Self::AfterMaster => "listing_after_master",
            Self::Contradictory => "listing_contradictory",
        }
    }
}

/// One security's span of trading days, as one source states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span<'a> {
    /// The first trading day, or why it is not known.
    pub first: Result<Day, Unknown>,
    /// The day history ends: the master's removal day with no readmission
    /// after it, or the vendor's last day bar for an inferred span.
    pub last: Option<Day>,
    /// Where it came from.
    pub source: Source<'a>,
}

impl Span<'_> {
    /// A span inferred from the vendor's own day rung: first and last served
    /// day bars. A label only; it refuses nothing.
    #[must_use]
    pub const fn inferred(first: Day, last: Day) -> Span<'static> {
        Span {
            first: Ok(first),
            last: Some(last),
            source: Source::InferredFromVendorSpan,
        }
    }

    /// Whether a bar on `day` is refused as before this security's listing:
    /// only on a master's stated listing day, never on an unknown or inferred one.
    #[must_use]
    pub fn before_listing(&self, day: Day) -> bool {
        matches!(self.source, Source::NseSecurityMaster { .. })
            && matches!(self.first, Ok(first) if day < first)
    }

    /// The master's listing day, when a refusal may rest on it.
    #[must_use]
    pub const fn listed_on(&self) -> Option<Day> {
        match (self.source, self.first) {
            (Source::NseSecurityMaster { .. }, Ok(first)) => Some(first),
            _ => None,
        }
    }

    /// The removal day when `day` lies strictly after it on a master's word:
    /// the security's history has ended. Never on an inferred span.
    #[must_use]
    pub fn history_ended(&self, day: Day) -> Option<Day> {
        match (self.source, self.last) {
            (Source::NseSecurityMaster { .. }, Some(last)) if day > last => Some(last),
            _ => None,
        }
    }
}

/// The listing table of one store: the newest receipted master that checks,
/// and every newer one it passed over.
#[derive(Debug)]
pub struct Listing {
    master: Option<(String, VerifiedLifecycleMaster)>,
    passed_over: Vec<(String, String)>,
}

impl Listing {
    /// The newest master under `masters` (a store's [`MASTER_DIR`]) that checks.
    /// Never fails: a directory that does not list, or holds no master that
    /// checks, is a table with no master, and every master passed over is
    /// named with why — a name shaped like a master's that states no calendar
    /// day included ([`UNDATED_MASTER`]), which is never read.
    #[must_use]
    pub fn load(masters: &Path) -> Self {
        // `None` for a master-shaped name that states no day: it sorts after
        // every dated one, so ONE sort orders both (gate 11, docs/06).
        let mut found: Vec<(Option<Day>, String)> = match std::fs::read_dir(masters) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().into_string().ok()?;
                    match master_day(&name) {
                        Some(day) => Some((Some(day), name)),
                        None if master_shaped(&name) => Some((None, name)),
                        None => None,
                    }
                })
                .collect(),
            Err(why) => {
                return Self {
                    master: None,
                    passed_over: vec![(masters.display().to_string(), why.to_string())],
                };
            }
        };
        // NEWEST FIRST, and one name per day: the directory order is not.
        found.sort_unstable_by(|a, b| b.cmp(a));
        let mut master = None;
        let mut passed_over = Vec::new();
        for (day, name) in found {
            match (day, &master) {
                // A NAME THAT STATES NO DAY IS NAMED, NEVER SKIPPED UNSAID
                // (R4-17 repair 4): it was dropped before any read, so a
                // master saved under a mistyped date answered nothing and no
                // line said so.
                (None, _) => passed_over.push((
                    name,
                    format!(
                        "{UNDATED_MASTER}: the name states no calendar day \
                         (NSE_CM_security_ddmmyyyy.csv.gz); it is never read"
                    ),
                )),
                // Older than the master in use: not passed over, not read.
                (Some(_), Some(_)) => {}
                (Some(day), None) => match read_local_lifecycle(masters, day) {
                    Ok(checked) => master = Some((name, checked)),
                    Err(why) => passed_over.push((name, why)),
                },
            }
        }
        Self {
            master,
            passed_over,
        }
    }

    /// The master in use: its file name and the day its receipt binds.
    #[must_use]
    pub fn master(&self) -> Option<(&str, Day)> {
        self.master
            .as_ref()
            .map(|(name, master)| (name.as_str(), master.provenance().master_day))
    }

    /// The master in use: its file name and the SHA-256 its receipt binds.
    #[must_use]
    pub fn in_use(&self) -> Option<(&str, &[u8; 32])> {
        self.master
            .as_ref()
            .map(|(name, master)| (name.as_str(), &master.provenance().sha256))
    }

    /// Newer masters that did not check, each with why, newest first; then
    /// every master-shaped name that states no day ([`UNDATED_MASTER`]).
    #[must_use]
    pub fn passed_over(&self) -> &[(String, String)] {
        &self.passed_over
    }

    /// The label of a share this table holds no span for, asked with the
    /// `isin` [`Listing::span`] was asked with:
    /// [`Source::InferredFromVendorSpan`] when a master checked and omits it,
    /// [`Source::NoIsin`] when a master checked but the share has no ISIN to
    /// ask it with, [`Source::NoCheckedMaster`] when no master checked at all.
    #[must_use]
    pub const fn absent(&self, isin: Option<&str>) -> Source<'static> {
        match (self.master.is_some(), isin.is_some()) {
            (false, _) => Source::NoCheckedMaster,
            (true, false) => Source::NoIsin,
            (true, true) => Source::InferredFromVendorSpan,
        }
    }

    /// One `warn` per master passed over, naming it, why, and the master
    /// that answers instead; and one `warn` when none checks. Silent for a
    /// table whose newest master checked.
    fn say(&self, masters: &Path) {
        let dir = masters.display().to_string();
        let answering = self.in_use().map_or("none", |(file, _)| file);
        for (file, why) in &self.passed_over {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn(
                    EVENT_TARGET,
                    "a security master was passed over: its receipt or bytes do not check; \
                     the next newest that checks answers instead (none: the listing rule is off)",
                )
                .with("masters", telemetry::Value::Str(&dir))
                .with("master", telemetry::Value::Str(file))
                .with("why", telemetry::Value::Str(why))
                .with("in_use", telemetry::Value::Str(answering)),
            );
        }
        if self.master.is_none() {
            let _dropped_when_filtered = telemetry::emit(
                &telemetry::Event::warn(
                    EVENT_TARGET,
                    "no security master under the store checks: the listing rule is OFF for \
                     every CASH share (no bar is refused before its listing and no history \
                     ends on delisting) until a master that checks is installed",
                )
                .with("masters", telemetry::Value::Str(&dir))
                .with("label", telemetry::Value::Str(NO_CHECKED_MASTER))
                .with(
                    "passed_over",
                    telemetry::Value::Uint(
                        u64::try_from(self.passed_over.len()).unwrap_or(u64::MAX),
                    ),
                ),
            );
        }
    }

    /// The span of the NSE security `symbol` with `isin`, when the master holds
    /// a row for exactly that identity; `None` otherwise, and the caller labels
    /// the vendor's own span with [`Span::inferred`]. Any exchange but NSE has
    /// no span here.
    #[must_use]
    pub fn span(&self, exchange: &str, symbol: &str, isin: Option<&str>) -> Option<Span<'_>> {
        if exchange != "NSE" {
            return None;
        }
        if let (Some((file, master)), Some(isin)) = (&self.master, isin)
            && let Ok(evidence) = master.inspect(symbol, isin)
        {
            let source = Source::NseSecurityMaster {
                file,
                sha256: &evidence.provenance.sha256,
            };
            let metadata = evidence.identity.metadata;
            let master_day = evidence.provenance.master_day;
            // A CHRONOLOGY THE MASTER CONTRADICTS states neither end. A cell two
            // rows disagree on is that cell's own `Conflicting`, below.
            let contradicted = evidence.assessment.issues.iter().any(|issue| {
                matches!(
                    issue,
                    LifecycleIssue::RemovalBeforeListing
                        | LifecycleIssue::ReadmissionBeforeListing
                        | LifecycleIssue::ReadmissionNotAfterRemoval
                )
            });
            if contradicted {
                return Some(Span {
                    first: Err(Unknown::Contradictory),
                    last: None,
                    source,
                });
            }
            return Some(Span {
                first: listing_day(&metadata.listing, master_day),
                last: removal_day(&metadata.removal, &metadata.readmission, master_day),
                source,
            });
        }
        None
    }
}

/// The code a master-shaped name that states no calendar day is passed over
/// with: `NSE_CM_security_31022026.csv.gz`, a name with seven digits, or one
/// with a letter among them. It is never read.
pub const UNDATED_MASTER: &str = "undated_master_name";

/// The name a master is saved under, before its date is read.
const MASTER_PREFIX: &str = "NSE_CM_security_";
const MASTER_SUFFIX: &str = ".csv.gz";

/// Whether `name` has a master's prefix and suffix, whatever lies between.
fn master_shaped(name: &str) -> bool {
    name.len() >= MASTER_PREFIX.len() + MASTER_SUFFIX.len()
        && name.starts_with(MASTER_PREFIX)
        && name.ends_with(MASTER_SUFFIX)
}

/// The day a master names in its file name, `NSE_CM_security_ddmmyyyy.csv.gz`.
fn master_day(name: &str) -> Option<Day> {
    let digits = name
        .strip_prefix(MASTER_PREFIX)?
        .strip_suffix(MASTER_SUFFIX)?;
    if digits.len() != 8 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let number = |range: std::ops::Range<usize>| digits.get(range)?.parse::<u16>().ok();
    let day = u8::try_from(number(0..2)?).ok()?;
    let month = u8::try_from(number(2..4)?).ok()?;
    Day::new(number(4..8)?, month, day).ok()
}

/// A listing assertion as a day a refusal may rest on, or why not.
fn listing_day(field: &LifecycleDate, master_day: Day) -> Result<Day, Unknown> {
    match field {
        LifecycleDate::Date { day, .. } if *day <= master_day => Ok(*day),
        LifecycleDate::Date { .. } => Err(Unknown::AfterMaster),
        LifecycleDate::Zero => Err(Unknown::EpochZero),
        LifecycleDate::Empty => Err(Unknown::Blank),
        LifecycleDate::HeaderAbsent => Err(Unknown::ColumnAbsent),
        LifecycleDate::Invalid { .. } => Err(Unknown::Invalid),
        LifecycleDate::Conflicting => Err(Unknown::Conflicting),
    }
}

/// The removal day, when the master states one on or before its own day and
/// no readmission. A readmission on or before the removal is a contradiction
/// [`Listing::span`] has already answered, so any readmission date here came
/// after it: the history did not end. A removal two rows disagree on states no
/// day, so it ends nothing.
fn removal_day(
    removal: &LifecycleDate,
    readmission: &LifecycleDate,
    master_day: Day,
) -> Option<Day> {
    removal
        .date()
        .filter(|day| *day <= master_day && readmission.date().is_none())
}

/// The listing table of the store at `store_root`, loaded once per process
/// and remembered only when a master was loaded: a store whose master is
/// installed later is read again on the next call (AF-03). Each load says
/// every master it passed over, and a store with none that checks, on
/// `/logs` (module doc, "Said out loud").
#[must_use]
pub fn for_store(store_root: &Path) -> Arc<Listing> {
    static TABLES: OnceLock<Mutex<HashMap<PathBuf, Arc<Listing>>>> = OnceLock::new();
    // PRE-SIZED FOR ONE ROOT: a process serves one store; a test process
    // that names more grows the table, amortised, once per root.
    let tables = TABLES.get_or_init(|| Mutex::new(HashMap::with_capacity(1)));
    if let Some(held) = tables
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(store_root)
    {
        return Arc::clone(held);
    }
    let masters = store_root.join(MASTER_DIR);
    let loaded = Arc::new(Listing::load(&masters));
    say_once(store_root, &masters, &loaded);
    if loaded.master.is_some() {
        tables
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(store_root.to_path_buf(), Arc::clone(&loaded));
    }
    loaded
}

/// [`Listing::say`], unless this store's last load in this process found
/// exactly the same: a table with no master is loaded on every call, and
/// the same finding said once per answer would bury `/logs`.
fn say_once(store_root: &Path, masters: &Path, table: &Listing) {
    type Said = (bool, Vec<(String, String)>);
    static SAID: OnceLock<Mutex<HashMap<PathBuf, Said>>> = OnceLock::new();
    if table.master.is_some() && table.passed_over.is_empty() {
        return;
    }
    let now: Said = (table.master.is_some(), table.passed_over.clone());
    {
        // PRE-SIZED FOR ONE ROOT, as `for_store`'s own table.
        let said = SAID.get_or_init(|| Mutex::new(HashMap::with_capacity(1)));
        let mut said = said.lock().unwrap_or_else(PoisonError::into_inner);
        if said.get(store_root) == Some(&now) {
            return;
        }
        said.insert(store_root.to_path_buf(), now);
    }
    table.say(masters);
}

#[cfg(test)]
#[path = "listing_tests.rs"]
mod tests;
