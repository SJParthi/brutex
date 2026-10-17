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
//!   ([`Span::before_listing`]): a bar on it is refused by name (D-4990).
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
//!
//! # Which master
//!
//! The newest receipted one under `<store>/session-masters`. A newer master
//! whose receipt or bytes do not check is passed over BY NAME
//! ([`Listing::passed_over`]) and the next newest is used; a store with no
//! master that checks answers every probe "not in any master". Nothing is
//! downloaded, created or repaired here.
//!
//! # Cost
//!
//! [`Listing::span`] is one expected-O(1) hash probe into the master's index
//! (parsed once at load) plus bounded key bytes, or one into the inferred
//! table. Loading lists the master directory once and parses one master: done
//! once per store root per process by [`for_store`], and only remembered when
//! a master was loaded (AF-03: a failed load is never remembered). Measured by
//! `crates/pull/benches/ratio/identity.rs`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::cash_auction::{LifecycleDate, LifecycleQuality};
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
    /// No master on disk holds this name: the vendor's own day rung, labelled.
    InferredFromVendorSpan,
}

impl Source<'_> {
    /// The code a receipt line and a log event carry.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NseSecurityMaster { .. } => "nse_security_master",
            Self::InferredFromVendorSpan => "inferred_from_vendor_span",
        }
    }
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
    /// The span is inferred from the vendor, which cannot accuse itself.
    Inferred,
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
            Self::Inferred => "listing_inferred",
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
/// and the spans inferred for names in none.
#[derive(Debug)]
pub struct Listing {
    master: Option<(String, VerifiedLifecycleMaster)>,
    passed_over: Vec<(String, String)>,
    inferred: HashMap<String, (Day, Day)>,
}

impl Listing {
    /// The newest master under `masters` (a store's [`MASTER_DIR`]) that checks.
    /// Never fails: a directory that does not list, or holds no master that
    /// checks, is a table with no master, and every master passed over is
    /// named with why.
    #[must_use]
    pub fn load(masters: &Path) -> Self {
        let mut found: Vec<(Day, String)> = match std::fs::read_dir(masters) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().into_string().ok()?;
                    master_day(&name).map(|day| (day, name))
                })
                .collect(),
            Err(why) => {
                return Self {
                    master: None,
                    passed_over: vec![(masters.display().to_string(), why.to_string())],
                    inferred: HashMap::new(),
                };
            }
        };
        // NEWEST FIRST, and one name per day: the directory order is not.
        found.sort_unstable_by(|a, b| b.cmp(a));
        let mut passed_over = Vec::new();
        for (day, name) in found {
            match read_local_lifecycle(masters, day) {
                Ok(master) => {
                    return Self {
                        master: Some((name, master)),
                        passed_over,
                        inferred: HashMap::new(),
                    };
                }
                Err(why) => passed_over.push((name, why)),
            }
        }
        Self {
            master: None,
            passed_over,
            inferred: HashMap::new(),
        }
    }

    /// The master in use: its file name and the day its receipt binds.
    #[must_use]
    pub fn master(&self) -> Option<(&str, Day)> {
        self.master
            .as_ref()
            .map(|(name, master)| (name.as_str(), master.provenance().master_day))
    }

    /// Newer masters that did not check, each with why, newest first.
    #[must_use]
    pub fn passed_over(&self) -> &[(String, String)] {
        &self.passed_over
    }

    /// Record the vendor's own span for `symbol`, used only when no master
    /// row answers for it.
    pub fn infer(&mut self, symbol: &str, first: Day, last: Day) {
        self.inferred.insert(symbol.to_owned(), (first, last));
    }

    /// The span of the NSE security `symbol` with `isin`, when known: the
    /// master's row for exactly that identity, else a span inferred for the
    /// symbol, else `None`. Any exchange but NSE has no span here.
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
            if evidence.assessment.quality == LifecycleQuality::Contradictory {
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
        self.inferred
            .get(symbol)
            .map(|(first, last)| Span::inferred(*first, *last))
    }
}

/// The day a master names in its file name, `NSE_CM_security_ddmmyyyy.csv.gz`.
fn master_day(name: &str) -> Option<Day> {
    let digits = name
        .strip_prefix("NSE_CM_security_")?
        .strip_suffix(".csv.gz")?;
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
/// no readmission after it.
fn removal_day(removal: &LifecycleDate, readmission: &LifecycleDate, master_day: Day) -> Option<Day> {
    let removed = removal.date().filter(|day| *day <= master_day)?;
    match readmission.date() {
        Some(back) if back > removed => None,
        _ => Some(removed),
    }
}

/// The listing table of the store at `store_root`, loaded once per process
/// and remembered only when a master was loaded: a store whose master is
/// installed later is read again on the next call (AF-03).
#[must_use]
pub fn for_store(store_root: &Path) -> Arc<Listing> {
    static TABLES: OnceLock<Mutex<HashMap<PathBuf, Arc<Listing>>>> = OnceLock::new();
    let tables = TABLES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(held) = tables
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(store_root)
    {
        return Arc::clone(held);
    }
    let loaded = Arc::new(Listing::load(&store_root.join(MASTER_DIR)));
    if loaded.master.is_some() {
        tables
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(store_root.to_path_buf(), Arc::clone(&loaded));
    }
    loaded
}

#[cfg(test)]
#[path = "listing_tests.rs"]
mod tests;
