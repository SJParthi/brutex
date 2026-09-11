//! Dated NSE MII cash-auction eligibility, parsed once from supplied CSV bytes.
//!
//! The caller authenticates the source and binds this value to the master date;
//! this module does not fetch, decompress, select dates, or infer membership.
//! NSE/CMTR/73845 describes the eligibility indicator; NSE/CMTR/74466 names the
//! dated MII master. Only exact EQ symbol/ISIN mappings answer a query. Missing
//! evidence is an error, never an inferred `false` or permission to trade.
//!
//! Parsing is O(input bytes), bounded by the limits below. Retained space is
//! O(EQ identities); lookup uses a `HashMap` (expected O(1), plus key-byte cost).
//! CSV supports quoted fields/escaped quotes, BOM and CRLF, but refuses quoted
//! multiline records rather than guessing how a broken physical row continues.
//!
//! Lifecycle columns are optional, independent snapshot assertions. Their
//! absence, zero values and disagreements cannot exempt a historical gap.
//! A dated master is not a point-in-time identity or complete lifecycle ledger.

use std::borrow::Cow;
use std::collections::{HashMap, hash_map::Entry};

use crate::session::Day;

/// Maximum decompressed master size accepted by this parser: 64 MiB.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;
/// Maximum data records, including non-EQ rows and identical duplicates.
pub const MAX_ROWS: usize = 250_000;
const MAX_RECORD_BYTES: usize = 16 * 1024;
const MAX_FIELDS: usize = 256;
const MAX_IDENTITY_BYTES: usize = 256;
const REQUIRED: [&str; 5] = [
    "FinInstrmId",
    "TckrSymb",
    "SctySrs",
    "ISIN",
    "ElgbltyClsgAuctnSsn",
];
const LIFECYCLE_COLUMNS: [&str; 3] = ["ListgDt", "RmvlDt", "RadmssnDt"];

/// The three lifecycle assertions carried by a security-master row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleField {
    /// `ListgDt`; not necessarily an IPO or the start of this ISIN's history.
    Listing,
    /// `RmvlDt`; its absence does not prove uninterrupted trading.
    Removal,
    /// `RadmssnDt`; its absence does not prove there was no readmission.
    Readmission,
}

/// Why a supplied lifecycle value could not be interpreted as a civil date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleDateError {
    /// Encoding not observed/supported, including unrecognised sentinel text.
    UnsupportedEncoding,
    /// Outside the nonnegative signed-LONG date range.
    OutOfRange,
    /// Contains a time of day; never rounded to manufacture a date.
    NotMidnight,
}

/// One optional lifecycle field, preserving uncertainty rather than inventing
/// a date or treating a missing removal as proof that removal never happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleDate {
    /// Older CAS fixtures/masters can omit the column entirely.
    HeaderAbsent,
    /// The column exists but its cell is blank.
    Empty,
    /// Literal `0`, observed in the real master; no historical claim is made.
    Zero,
    /// A decoded assertion in the source row, NOT a verified historical event.
    Date {
        /// Exact unsigned decimal seconds supplied by the master.
        seconds_since_1980: u32,
        /// Date decoded from the NSE epoch, without a timezone shift.
        day: Day,
    },
    /// Supplied evidence is retained, but is not a usable date.
    Invalid {
        /// Exact bounded CSV field, without trimming/coercion.
        raw: String,
        /// Typed reason for withholding interpretation.
        reason: LifecycleDateError,
    },
    /// Duplicate exact identities disagree on this field. No row wins.
    Conflicting,
}

impl LifecycleDate {
    /// The source's decoded date, if any. Callers must also inspect the full
    /// assessment against the master date; this grants no historical exemption.
    #[must_use]
    pub const fn date(&self) -> Option<Day> {
        match self {
            Self::Date { day, .. } => Some(*day),
            _ => None,
        }
    }

    fn parse(raw: Option<&str>) -> Self {
        let Some(raw) = raw else {
            return Self::HeaderAbsent;
        };
        match raw {
            "" => return Self::Empty,
            "0" => return Self::Zero,
            _ => {}
        }
        match lifecycle_day(raw) {
            Ok((seconds_since_1980, day)) => Self::Date {
                seconds_since_1980,
                day,
            },
            Err(reason) => Self::Invalid {
                raw: raw.to_owned(),
                reason,
            },
        }
    }

    fn merge(&mut self, other: &Self) {
        if *self != *other {
            *self = Self::Conflicting;
        }
    }
}

/// Snapshot metadata for one exact EQ identity. Even three consistent dates
/// cannot prove that there were no other listings, removals, renames or halts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleMetadata {
    /// Source `ListgDt` assertion.
    pub listing: LifecycleDate,
    /// Source `RmvlDt` assertion.
    pub removal: LifecycleDate,
    /// Source `RadmssnDt` assertion.
    pub readmission: LifecycleDate,
}

/// Qualification of the snapshot, never a coverage or trading verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleQuality {
    /// No date can be read from the supplied fields.
    Unavailable,
    /// Some dates exist; at least one other assertion is absent/unspecified.
    Partial,
    /// Every supplied date is ordered and not after the master date. Still
    /// requires historical corroboration; this is not complete-history proof.
    SnapshotOnly,
    /// Invalid encoding or a future-dated assertion needs corroboration.
    Unverified,
    /// Duplicate rows or the supplied chronology disagree.
    Contradictory,
}

/// A bounded, typed diagnostic for lifecycle metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleIssue {
    /// Inspect the corresponding field to distinguish absent/blank/zero.
    Unspecified(LifecycleField),
    /// An uninterpretable field; raw value/reason remain on the metadata.
    Invalid(LifecycleField),
    /// Different exact-identity rows disagree; no first/last-row preference.
    Conflicting(LifecycleField),
    /// A future date cannot be used as an event proved by this snapshot.
    AfterMaster(LifecycleField),
    /// The removal assertion predates the listing assertion.
    RemovalBeforeListing,
    /// The readmission assertion predates the listing assertion.
    ReadmissionBeforeListing,
    /// Readmission is not strictly after the supplied removal.
    ReadmissionNotAfterRemoval,
    /// A readmission exists without a usable removal date; no interval implied.
    ReadmissionWithoutRemoval,
}

/// Assessment against a caller-bound master date. At most seven diagnostics
/// can be emitted; no historical date range is scanned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleAssessment {
    /// Overall qualification, with contradictions taking precedence.
    pub quality: LifecycleQuality,
    /// Fixed-order diagnostics; empty still means only snapshot consistency.
    pub issues: Vec<LifecycleIssue>,
}

impl LifecycleMetadata {
    /// Assess the three source assertions without deciding whether any
    /// historical day was tradable or whether a missing candle is permissible.
    #[must_use]
    pub fn assess(&self, master_day: Day) -> LifecycleAssessment {
        let mut issues = Vec::new();
        let mut dates = 0;
        let mut unverified = false;
        let mut contradictory = false;
        for (field, value) in [
            (LifecycleField::Listing, &self.listing),
            (LifecycleField::Removal, &self.removal),
            (LifecycleField::Readmission, &self.readmission),
        ] {
            match value {
                LifecycleDate::Date { day, .. } => {
                    dates += 1;
                    if *day > master_day {
                        issues.push(LifecycleIssue::AfterMaster(field));
                        unverified = true;
                    }
                }
                LifecycleDate::Invalid { .. } => {
                    issues.push(LifecycleIssue::Invalid(field));
                    unverified = true;
                }
                LifecycleDate::Conflicting => {
                    issues.push(LifecycleIssue::Conflicting(field));
                    contradictory = true;
                }
                _ => issues.push(LifecycleIssue::Unspecified(field)),
            }
        }
        let listing = self.listing.date();
        let removal = self.removal.date();
        let readmission = self.readmission.date();
        for (earlier, later, issue, equal_conflicts) in [
            (
                listing,
                removal,
                LifecycleIssue::RemovalBeforeListing,
                false,
            ),
            (
                listing,
                readmission,
                LifecycleIssue::ReadmissionBeforeListing,
                false,
            ),
            (
                removal,
                readmission,
                LifecycleIssue::ReadmissionNotAfterRemoval,
                true,
            ),
        ] {
            if let (Some(earlier), Some(later)) = (earlier, later)
                && (later < earlier || (equal_conflicts && later == earlier))
            {
                issues.push(issue);
                contradictory = true;
            }
        }
        if readmission.is_some() && removal.is_none() {
            issues.push(LifecycleIssue::ReadmissionWithoutRemoval);
        }
        let quality = if contradictory {
            LifecycleQuality::Contradictory
        } else if unverified {
            LifecycleQuality::Unverified
        } else {
            match dates {
                0 => LifecycleQuality::Unavailable,
                3 => LifecycleQuality::SnapshotOnly,
                _ => LifecycleQuality::Partial,
            }
        };
        LifecycleAssessment { quality, issues }
    }

    fn merge(&mut self, other: &Self) {
        self.listing.merge(&other.listing);
        self.removal.merge(&other.removal);
        self.readmission.merge(&other.readmission);
    }
}

// The actual 2026-09-04 MII EQ rows contain nonnegative decimal LONGs;
// all 3,830 nonzero ListgDt values are multiples of 86,400. No date-string
// variant was observed, so YYYYMMDD, ISO dates and arbitrary sentinels are NOT
// guessed. NSE Masters Data v1.6 §2.3 (p7) states seconds from 01-Jan-1980;
// §3.1.1 (p9) names listing/expulsion/readmission as LONG dates:
// https://nsearchives.nseindia.com/web/sites/default/files/inline-files/NSE-Masters%20Data-v1.6.pdf
// This decodes a source assertion, not the event's historical applicability.
fn lifecycle_day(raw: &str) -> Result<(u32, Day), LifecycleDateError> {
    if raw.starts_with('0') || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(LifecycleDateError::UnsupportedEncoding);
    }
    let seconds = raw
        .parse::<i32>()
        .map_err(|_| LifecycleDateError::OutOfRange)?;
    let seconds = i64::from(seconds);
    if seconds % crate::session::SECS_PER_DAY != 0 {
        return Err(LifecycleDateError::NotMidnight);
    }
    let epoch = Day::new(1980, 1, 1).map_err(|_| LifecycleDateError::OutOfRange)?;
    let days = u32::try_from(seconds / crate::session::SECS_PER_DAY)
        .map_err(|_| LifecycleDateError::OutOfRange)?;
    let day = Day::from_days(epoch.days_from_epoch() + days)
        .map_err(|_| LifecycleDateError::OutOfRange)?;
    Ok((
        u32::try_from(seconds).map_err(|_| LifecycleDateError::OutOfRange)?,
        day,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Identity {
    native_id: String,
    isin: String,
    eligible: bool,
    lifecycle: LifecycleMetadata,
}

/// Borrowed metadata after the exact symbol/ISIN check. The parser does not
/// authenticate or date its input; use the receipted-cache API for that binding.
#[derive(Debug, Clone, Copy)]
pub struct LifecycleEvidence<'a> {
    /// Exact EQ symbol, never an alias inferred from the ISIN.
    pub symbol: &'a str,
    /// Exact matched ISIN.
    pub isin: &'a str,
    /// Native ID of the unique EQ row.
    pub native_id: &'a str,
    /// All three assertions, including absent/invalid/conflicting states.
    pub metadata: &'a LifecycleMetadata,
}

/// One independently documented NSE trading interruption. This is distinct
/// from a current master's incomplete lifecycle fields and contains no inferred
/// pre-listing interval. Sources name the exact symbol and ISIN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InactiveInterval {
    /// First inactive date, inclusive.
    pub from: Day,
    /// Last inactive date, inclusive.
    pub through: Day,
    /// NSE withdrawal circular establishing the start boundary.
    pub withdrawal_source: &'static str,
    /// Company filing containing the NSE readmission circular/end boundary.
    pub readmission_source: &'static str,
}

/// Inspect independently sourced inactivity for exactly this symbol/ISIN/date.
/// `None` means NO applicable independent evidence, not "active" or "complete".
/// No alias, case folding, ISIN substitution, current listing-date inference,
/// or calendar-day approximation is allowed. Constant comparisons only.
///
/// `docs/00-charter.md` §9, "FORCEMOT NSE interruption": NSE CML58560
/// withdraws FORCEMOT / INE451A01017 effective 2023-10-26 after the 25th's
/// close; the company filing containing CML60618 admits that same EQ identity
/// from 2024-02-14. The certified inactive interval is therefore inclusive
/// 2023-10-26..2024-02-13. It says nothing about BSE or any other period.
#[must_use]
pub fn independently_inactive(symbol: &str, isin: &str, day: Day) -> Option<InactiveInterval> {
    if symbol != "FORCEMOT" || isin != "INE451A01017" {
        return None;
    }
    let interval = InactiveInterval {
        from: Day::new(2023, 10, 26).ok()?,
        through: Day::new(2024, 2, 13).ok()?,
        withdrawal_source: "https://archives.nseindia.com/content/circulars/CML58560.pdf",
        readmission_source: "https://www.forcemotors.com/wp-content/uploads/2025/02/Announcement-under-Regulation-30-for-Listing-on-NSE.pdf",
    };
    (day >= interval.from && day <= interval.through).then_some(interval)
}

/// Exact EQ identities from one caller-selected, dated NSE MII master.
///
/// There is no default/empty constructor that can masquerade as parsed proof.
#[derive(Debug, Clone)]
pub struct DailyEligibility {
    by_symbol: HashMap<String, Identity>,
}

impl DailyEligibility {
    /// Parse decompressed UTF-8 CSV once, with strict schema checks.
    ///
    /// Unrelated columns may appear in any order. Every record must have the
    /// header's width; EQ records require an explicit `0` or `1` flag. Non-EQ
    /// series are checked structurally, not for eligibility, and never become
    /// an EQ lookup. Identical selected identities
    /// may repeat; any disagreement in symbol, native ID, ISIN or flag refuses
    /// the entire master. No partial map escapes an error.
    ///
    /// # Errors
    /// Returns an `UNVERIFIED` reason for bad schema/records, exceeded
    /// bounds, conflicting identities or a master with no EQ mappings.
    pub fn parse(text: &str) -> Result<Self, String> {
        if text.len() > MAX_BYTES {
            return Err(unknown("master exceeds the 64 MiB decompressed byte limit"));
        }
        let mut lines = text.strip_prefix('\u{feff}').unwrap_or(text).lines();
        let header = fields(lines.next().ok_or_else(|| unknown("missing header"))?)?;
        let columns = Columns::parse(&header)?;
        let mut by_symbol = HashMap::new();
        let mut by_native_id = HashMap::new();
        for (index, line) in lines.enumerate() {
            if index >= MAX_ROWS {
                return Err(unknown("master exceeds the data-record limit"));
            }
            let parsed = fields(line)
                .and_then(|row| columns.identity(&row))
                .map_err(|why| format!("line {}: {why}", index + 2))?;
            if let Some((symbol, identity)) = parsed {
                insert_identity(&mut by_symbol, &mut by_native_id, symbol, identity)
                    .map_err(|why| format!("line {}: {why}", index + 2))?;
            }
        }
        if by_symbol.is_empty() {
            return Err(unknown("master contains no EQ identities"));
        }
        Ok(Self { by_symbol })
    }

    /// Read the explicit flag for exactly this EQ symbol and ISIN.
    ///
    /// No case folding, whitespace trimming, symbol aliases or ISIN borrowing.
    /// `Ok(false)` means an explicit zero, not absent evidence.
    ///
    /// # Errors
    /// Returns `UNVERIFIED` for missing symbols, wrong ISINs or oversized keys.
    pub fn eligibility(&self, symbol: &str, isin: &str) -> Result<bool, String> {
        self.identity(symbol, isin)
            .map(|(_, identity)| identity.eligible)
    }

    /// Inspect lifecycle assertions for the exact EQ symbol and ISIN.
    /// Missing lifecycle headers are represented, not a failure of CAS parsing.
    /// Invalid/contradictory lifecycle fields do not change the CAS flag.
    ///
    /// # Errors
    /// Returns `UNVERIFIED` for missing symbols, wrong ISINs or malformed keys.
    pub fn lifecycle(&self, symbol: &str, isin: &str) -> Result<LifecycleEvidence<'_>, String> {
        let (symbol, identity) = self.identity(symbol, isin)?;
        Ok(LifecycleEvidence {
            symbol,
            isin: &identity.isin,
            native_id: &identity.native_id,
            metadata: &identity.lifecycle,
        })
    }

    fn identity(&self, symbol: &str, isin: &str) -> Result<(&str, &Identity), String> {
        identity_word(symbol, "lookup symbol")?;
        identity_word(isin, "lookup ISIN")?;
        let (stored_symbol, identity) = self
            .by_symbol
            .get_key_value(symbol)
            .ok_or_else(|| unknown(&format!("no EQ mapping for symbol {symbol:?}")))?;
        if identity.isin != isin {
            return Err(unknown(&format!("ISIN mismatch for EQ symbol {symbol:?}")));
        }
        Ok((stored_symbol, identity))
    }
}

/// Dated continuous-session closes for exactly one instrument.
///
/// The loader owns the instrument, authenticated source and master-date binding.
/// Do not reuse a schedule across instruments or carry the last known flag
/// forward. This type records no auction prices and grants no trading approval.
#[derive(Debug, Default, Clone)]
pub struct Schedule {
    closes: HashMap<u32, u16>,
}

impl Schedule {
    /// Insert the flag verified for this instrument in this day's master.
    /// Identical repetitions are idempotent; contradictory flags are refused.
    ///
    /// # Errors
    /// Returns `UNVERIFIED` if the date already carries a different flag. Its
    /// existing value is preserved; the caller must propagate the refusal.
    pub fn insert(&mut self, day: Day, eligible: bool) -> Result<(), String> {
        let close = if eligible { 15 * 60 + 15 } else { 15 * 60 + 30 };
        match self.closes.entry(day.days_from_epoch()) {
            Entry::Occupied(entry) if *entry.get() != close => {
                Err(unknown(&format!("conflicting dated eligibility on {day}")))
            }
            Entry::Occupied(_) => Ok(()),
            Entry::Vacant(entry) => {
                entry.insert(close);
                Ok(())
            }
        }
    }

    /// Exclusive continuous-session close in IST minutes: 930 before the CAS
    /// change; afterward 915 for an explicit eligible flag, otherwise 930.
    ///
    /// # Errors
    /// Returns `UNVERIFIED` after the change if this exact date is absent.
    pub fn close(&self, day: Day) -> Result<u16, String> {
        if !crate::vendor::cash_auction_eligibility_required(day) {
            return Ok(15 * 60 + 30);
        }
        self.closes
            .get(&day.days_from_epoch())
            .copied()
            .ok_or_else(|| unknown(&format!("no dated cash-auction eligibility on {day}")))
    }
}

struct Columns {
    width: usize,
    positions: [usize; 5],
    lifecycle: [Option<usize>; 3],
}

impl Columns {
    fn parse(header: &[Cow<'_, str>]) -> Result<Self, String> {
        let mut seen = std::collections::HashSet::new();
        for name in header {
            if name.is_empty() || !seen.insert(name.as_ref()) {
                return Err(unknown("empty or duplicate header name"));
            }
        }
        let mut positions = [0; 5];
        for (slot, required) in positions.iter_mut().zip(REQUIRED) {
            *slot = header
                .iter()
                .position(|name| name == required)
                .ok_or_else(|| unknown(&format!("missing required header {required}")))?;
        }
        Ok(Self {
            width: header.len(),
            positions,
            lifecycle: LIFECYCLE_COLUMNS
                .map(|wanted| header.iter().position(|name| name == wanted)),
        })
    }

    fn identity(&self, row: &[Cow<'_, str>]) -> Result<Option<(String, Identity)>, String> {
        if row.len() != self.width {
            return Err(unknown("record width differs from header"));
        }
        let [id_at, symbol_at, series_at, isin_at, flag_at] = self.positions;
        let at = |position| {
            row.get(position)
                .map(Cow::as_ref)
                .ok_or_else(|| unknown("required field position is absent"))
        };
        if at(series_at)? != "EQ" {
            return Ok(None);
        }
        let flag = match at(flag_at)? {
            "0" => false,
            "1" => true,
            _ => return Err(unknown("ElgbltyClsgAuctnSsn must be exactly 0 or 1")),
        };
        let native_id = at(id_at)?;
        identity_word(native_id, "FinInstrmId")?;
        if !native_id.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(unknown("FinInstrmId is not an unsigned decimal identifier"));
        }
        let symbol = at(symbol_at)?;
        let isin = at(isin_at)?;
        identity_word(symbol, "TckrSymb")?;
        identity_word(isin, "ISIN")?;
        Ok(Some((
            symbol.to_owned(),
            Identity {
                native_id: native_id.to_owned(),
                isin: isin.to_owned(),
                eligible: flag,
                lifecycle: {
                    let [listing, removal, readmission] = self.lifecycle.map(|position| {
                        LifecycleDate::parse(position.and_then(|at| row.get(at).map(Cow::as_ref)))
                    });
                    LifecycleMetadata {
                        listing,
                        removal,
                        readmission,
                    }
                },
            },
        )))
    }
}

fn insert_identity(
    symbols: &mut HashMap<String, Identity>,
    native_ids: &mut HashMap<String, String>,
    symbol: String,
    identity: Identity,
) -> Result<(), String> {
    if let Some(previous) = native_ids.get(&identity.native_id)
        && previous != &symbol
    {
        return Err(unknown("conflicting EQ symbols for FinInstrmId"));
    }
    match symbols.entry(symbol.clone()) {
        Entry::Occupied(previous)
            if previous.get().native_id != identity.native_id
                || previous.get().isin != identity.isin
                || previous.get().eligible != identity.eligible =>
        {
            Err(unknown(&format!(
                "conflicting EQ identity/flag for symbol {symbol:?}"
            )))
        }
        Entry::Occupied(mut previous) => {
            previous.get_mut().lifecycle.merge(&identity.lifecycle);
            Ok(())
        }
        Entry::Vacant(slot) => {
            native_ids.insert(identity.native_id.clone(), symbol);
            slot.insert(identity);
            Ok(())
        }
    }
}

fn identity_word(word: &str, field: &str) -> Result<(), String> {
    if word.is_empty()
        || word.len() > MAX_IDENTITY_BYTES
        || word.chars().any(char::is_whitespace)
        || word.chars().any(char::is_control)
    {
        return Err(unknown(&format!("missing, oversized or malformed {field}")));
    }
    Ok(())
}

fn unknown(why: &str) -> String {
    format!("cash-auction eligibility UNVERIFIED: {why}")
}

/// Physical records are bounded before allocating fields. Plain MII rows borrow
/// their values; quoting takes the slower owned path without changing identity.
fn fields(line: &str) -> Result<Vec<Cow<'_, str>>, String> {
    if line.is_empty() || line.len() > MAX_RECORD_BYTES || line.contains(['\r', '\n', '\0']) {
        return Err(unknown("empty, oversized or malformed CSV record"));
    }
    let mut result = Vec::new();
    if !line.contains('"') {
        for field in line.split(',') {
            if result.len() >= MAX_FIELDS {
                return Err(unknown("CSV field limit exceeded"));
            }
            result.push(Cow::Borrowed(field));
        }
        return Ok(result);
    }
    let mut input = line.chars().peekable();
    loop {
        if result.len() >= MAX_FIELDS {
            return Err(unknown("CSV field limit exceeded"));
        }
        let mut value = String::new();
        if input.peek() == Some(&'"') {
            input.next();
            loop {
                match input.next() {
                    Some('"') if input.peek() == Some(&'"') => {
                        input.next();
                        value.push('"');
                    }
                    Some('"') => break,
                    Some(c) => value.push(c),
                    None => return Err(unknown("unterminated quoted CSV field")),
                }
            }
        } else {
            while input.peek().is_some_and(|c| *c != ',') {
                match input.next() {
                    Some('"') => return Err(unknown("quote inside unquoted CSV field")),
                    Some(c) => value.push(c),
                    None => break,
                }
            }
        }
        result.push(Cow::Owned(value));
        match input.next() {
            Some(',') => {}
            None => return Ok(result),
            _ => return Err(unknown("unexpected text after quoted CSV field")),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test assertions"
)]
mod tests {
    use super::*;

    const HEADER: &str = "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn";
    const ABB: &str = "13,ABB,EQ,INE117A01022,1";

    fn parse(rows: &str) -> Result<DailyEligibility, String> {
        DailyEligibility::parse(&format!("{HEADER}\n{rows}\n"))
    }

    fn lifecycle_rows(rows: &str) -> DailyEligibility {
        DailyEligibility::parse(&format!("{HEADER},ListgDt,RmvlDt,RadmssnDt\n{rows}\n")).unwrap()
    }

    #[test]
    fn independent_forcemot_inactivity_has_exact_identity_and_inclusive_boundaries() {
        let from = Day::new(2023, 10, 26).unwrap();
        let through = Day::new(2024, 2, 13).unwrap();
        for epoch in from.days_from_epoch()..=through.days_from_epoch() {
            let interval =
                independently_inactive("FORCEMOT", "INE451A01017", Day::from_days(epoch).unwrap())
                    .unwrap();
            assert_eq!((interval.from, interval.through), (from, through));
            assert_eq!(
                interval.withdrawal_source,
                "https://archives.nseindia.com/content/circulars/CML58560.pdf"
            );
            assert_eq!(
                interval.readmission_source,
                "https://www.forcemotors.com/wp-content/uploads/2025/02/Announcement-under-Regulation-30-for-Listing-on-NSE.pdf"
            );
        }
        for date in [
            Day::new(2023, 10, 25).unwrap(),
            Day::new(2024, 2, 14).unwrap(),
        ] {
            assert_eq!(
                independently_inactive("FORCEMOT", "INE451A01017", date),
                None
            );
        }
        for (symbol, isin) in [
            ("FORCEMOT", "WRONG"),
            ("forcemot", "INE451A01017"),
            ("FORCEMOT ", "INE451A01017"),
            ("OTHER", "INE451A01017"),
            ("FORCEMOT", "INE451A01017 "),
            ("", ""),
        ] {
            assert_eq!(independently_inactive(symbol, isin, from), None);
        }
    }

    #[test]
    fn snapshot_listing_and_zero_removal_cannot_create_or_erase_independent_evidence() {
        let daily = lifecycle_rows(
            "2029,IRFC,EQ,INE053F01010,1,1296345600,0,0\n11573,FORCEMOT,EQ,INE451A01017,1,1250640000,0,0",
        );
        let irfc = daily.lifecycle("IRFC", "INE053F01010").unwrap();
        let earlier = Day::new(2019, 12, 1).unwrap();
        assert!(irfc.metadata.listing.date().unwrap() > earlier);
        assert_eq!(
            independently_inactive("IRFC", "INE053F01010", earlier),
            None
        );
        let force = daily.lifecycle("FORCEMOT", "INE451A01017").unwrap();
        assert_eq!(force.metadata.removal, LifecycleDate::Zero);
        assert_eq!(force.metadata.readmission, LifecycleDate::Zero);
        assert!(
            independently_inactive("FORCEMOT", "INE451A01017", Day::new(2023, 11, 1).unwrap())
                .is_some()
        );
    }

    fn lifecycle_date(year: u16, month: u8, day: u8) -> String {
        let date = Day::new(year, month, day).unwrap();
        let epoch = Day::new(1980, 1, 1).unwrap();
        (i64::from(date.days_from_epoch() - epoch.days_from_epoch()) * crate::session::SECS_PER_DAY)
            .to_string()
    }

    #[test]
    fn lifecycle_absent_headers_do_not_break_existing_cas_fixtures() {
        let daily = parse(ABB).unwrap();
        assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
        let evidence = daily.lifecycle("ABB", "INE117A01022").unwrap();
        assert_eq!(
            (evidence.symbol, evidence.isin, evidence.native_id),
            ("ABB", "INE117A01022", "13")
        );
        assert_eq!(
            evidence.metadata,
            &LifecycleMetadata {
                listing: LifecycleDate::HeaderAbsent,
                removal: LifecycleDate::HeaderAbsent,
                readmission: LifecycleDate::HeaderAbsent,
            }
        );
        assert_eq!(
            evidence.metadata.assess(Day::new(2026, 9, 4).unwrap()),
            LifecycleAssessment {
                quality: LifecycleQuality::Unavailable,
                issues: vec![
                    LifecycleIssue::Unspecified(LifecycleField::Listing),
                    LifecycleIssue::Unspecified(LifecycleField::Removal),
                    LifecycleIssue::Unspecified(LifecycleField::Readmission),
                ],
            }
        );
    }

    #[test]
    fn lifecycle_optional_columns_are_independent_quoted_and_reorderable() {
        let raw = "1296345600";
        for (header, field) in LIFECYCLE_COLUMNS.into_iter().zip([
            LifecycleField::Listing,
            LifecycleField::Removal,
            LifecycleField::Readmission,
        ]) {
            let daily = DailyEligibility::parse(&format!(
                "\u{feff}\"{header}\",{HEADER}\r\n\"{raw}\",{ABB}\r\n"
            ))
            .unwrap();
            let data = daily.lifecycle("ABB", "INE117A01022").unwrap();
            let expected = LifecycleDate::Date {
                seconds_since_1980: 1_296_345_600,
                day: Day::new(2021, 1, 29).unwrap(),
            };
            for (actual_field, actual) in [
                (LifecycleField::Listing, &data.metadata.listing),
                (LifecycleField::Removal, &data.metadata.removal),
                (LifecycleField::Readmission, &data.metadata.readmission),
            ] {
                if actual_field == field {
                    assert_eq!(actual, &expected);
                } else {
                    assert_eq!(actual, &LifecycleDate::HeaderAbsent);
                }
            }
            assert_eq!(
                data.metadata.assess(Day::new(2026, 9, 4).unwrap()).quality,
                LifecycleQuality::Partial
            );
            assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
        }
        for name in LIFECYCLE_COLUMNS {
            let why = DailyEligibility::parse(&format!("{HEADER},{name},{name}\n{ABB},0,0\n"))
                .unwrap_err();
            assert!(why.contains("duplicate header"), "{why}");
        }
    }

    #[test]
    fn lifecycle_zero_empty_and_unrecognised_sentinels_never_become_dates() {
        let daily = lifecycle_rows(&format!("{ABB},,0,-1"));
        let data = daily.lifecycle("ABB", "INE117A01022").unwrap();
        assert_eq!(data.metadata.listing, LifecycleDate::Empty);
        assert_eq!(data.metadata.removal, LifecycleDate::Zero);
        assert_eq!(
            data.metadata.readmission,
            LifecycleDate::Invalid {
                raw: "-1".to_owned(),
                reason: LifecycleDateError::UnsupportedEncoding,
            }
        );
        assert_eq!(
            data.metadata.assess(Day::new(2026, 9, 4).unwrap()).quality,
            LifecycleQuality::Unverified
        );
        let zero = lifecycle_rows(&format!("{ABB},0,0,0"));
        let zero = zero.lifecycle("ABB", "INE117A01022").unwrap();
        assert_eq!(
            zero.metadata.assess(Day::new(2026, 9, 4).unwrap()).quality,
            LifecycleQuality::Unavailable
        );
        for raw in [
            "00000000",
            "0000-00-00",
            "-1",
            "NULL",
            "N/A",
            " ",
            " 0",
            "0 ",
            "01",
            "+86400",
        ] {
            let value = LifecycleDate::parse(Some(raw));
            assert_eq!(
                value,
                LifecycleDate::Invalid {
                    raw: raw.to_owned(),
                    reason: LifecycleDateError::UnsupportedEncoding,
                }
            );
            assert_eq!(value.date(), None);
        }
    }

    #[test]
    fn lifecycle_actual_decimal_dates_use_the_nse_1980_epoch_not_unix() {
        // These raw EQ listing fields were inspected in the local 2026-09-04
        // master. These tests prove decoding, not the companies' full history.
        for (raw, year, month, day) in [
            ("1296345600", 2021, 1, 29),
            ("1250640000", 2019, 8, 19),
            ("502070400", 1995, 11, 29),
            ("86400", 1980, 1, 2),
        ] {
            let decoded = LifecycleDate::parse(Some(raw));
            assert_eq!(
                decoded,
                LifecycleDate::Date {
                    seconds_since_1980: raw.parse().unwrap(),
                    day: Day::new(year, month, day).unwrap(),
                }
            );
            assert_eq!(decoded.date(), Some(Day::new(year, month, day).unwrap()));
        }
    }

    #[test]
    fn lifecycle_does_not_guess_unobserved_formats_round_times_or_overflow() {
        for raw in [
            "2021-01-29",
            "29/01/2021",
            "29-JAN-2021",
            "20210129",
            "1296345600000",
            "1.2963456E9",
            "1296345600.0",
        ] {
            assert!(
                matches!(
                    LifecycleDate::parse(Some(raw)),
                    LifecycleDate::Invalid { .. }
                ),
                "{raw}"
            );
        }
        assert_eq!(
            LifecycleDate::parse(Some("1296345601")),
            LifecycleDate::Invalid {
                raw: "1296345601".to_owned(),
                reason: LifecycleDateError::NotMidnight,
            }
        );
        for raw in ["2147483648", "4294967295", "18446744073709551616"] {
            assert_eq!(
                LifecycleDate::parse(Some(raw)),
                LifecycleDate::Invalid {
                    raw: raw.to_owned(),
                    reason: LifecycleDateError::OutOfRange,
                }
            );
        }
        let last_midnight = (i64::from(i32::MAX) / crate::session::SECS_PER_DAY
            * crate::session::SECS_PER_DAY)
            .to_string();
        assert!(matches!(
            LifecycleDate::parse(Some(&last_midnight)),
            LifecycleDate::Date { .. }
        ));
    }

    #[test]
    fn lifecycle_listing_removal_readmission_are_only_ordered_snapshot_assertions() {
        let listing = lifecycle_date(2020, 1, 1);
        let removal = lifecycle_date(2023, 10, 26);
        let readmission = lifecycle_date(2024, 2, 14);
        let daily = lifecycle_rows(&format!("{ABB},{listing},{removal},{readmission}"));
        let data = daily.lifecycle("ABB", "INE117A01022").unwrap();
        assert_eq!(
            data.metadata.assess(Day::new(2026, 9, 4).unwrap()),
            LifecycleAssessment {
                quality: LifecycleQuality::SnapshotOnly,
                issues: vec![],
            }
        );
        let future = data.metadata.assess(Day::new(2023, 10, 26).unwrap());
        assert_eq!(future.quality, LifecycleQuality::Unverified);
        assert_eq!(
            future.issues,
            [LifecycleIssue::AfterMaster(LifecycleField::Readmission)]
        );
        let future = data.metadata.assess(Day::new(2019, 12, 31).unwrap());
        assert_eq!(
            future.issues,
            [
                LifecycleIssue::AfterMaster(LifecycleField::Listing),
                LifecycleIssue::AfterMaster(LifecycleField::Removal),
                LifecycleIssue::AfterMaster(LifecycleField::Readmission),
            ]
        );
        let partial = lifecycle_rows(&format!("{ABB},{listing},0,{readmission}"));
        let partial = partial
            .lifecycle("ABB", "INE117A01022")
            .unwrap()
            .metadata
            .assess(Day::new(2026, 9, 4).unwrap());
        assert_eq!(partial.quality, LifecycleQuality::Partial);
        assert_eq!(
            partial.issues,
            [
                LifecycleIssue::Unspecified(LifecycleField::Removal),
                LifecycleIssue::ReadmissionWithoutRemoval,
            ]
        );
    }

    #[test]
    fn lifecycle_contradictory_chronology_is_never_a_validated_interval() {
        let early = lifecycle_date(2020, 1, 1);
        let later = lifecycle_date(2024, 2, 14);
        for (listing, removal, readmission, expected) in [
            (&later, &early, &later, LifecycleIssue::RemovalBeforeListing),
            (
                &later,
                &early,
                &early,
                LifecycleIssue::ReadmissionBeforeListing,
            ),
            (
                &early,
                &later,
                &early,
                LifecycleIssue::ReadmissionNotAfterRemoval,
            ),
            (
                &early,
                &later,
                &later,
                LifecycleIssue::ReadmissionNotAfterRemoval,
            ),
        ] {
            let daily = lifecycle_rows(&format!("{ABB},{listing},{removal},{readmission}"));
            let report = daily
                .lifecycle("ABB", "INE117A01022")
                .unwrap()
                .metadata
                .assess(Day::new(2026, 9, 4).unwrap());
            assert_eq!(report.quality, LifecycleQuality::Contradictory);
            assert!(report.issues.contains(&expected));
            assert!(report.issues.len() <= 7);
            assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
        }
        let equal = lifecycle_rows(&format!("{ABB},{early},{early},{later}"));
        assert_eq!(
            equal
                .lifecycle("ABB", "INE117A01022")
                .unwrap()
                .metadata
                .assess(Day::new(2026, 9, 4).unwrap())
                .quality,
            LifecycleQuality::SnapshotOnly
        );
    }

    #[test]
    fn lifecycle_duplicate_disagreement_is_order_independent_and_cas_is_preserved() {
        let original = format!("{ABB},1296345600,0,0");
        for (changed, field) in [
            (format!("{ABB},1250640000,0,0"), LifecycleField::Listing),
            (
                format!("{ABB},1296345600,1250640000,0"),
                LifecycleField::Removal,
            ),
            (
                format!("{ABB},1296345600,0,1250640000"),
                LifecycleField::Readmission,
            ),
            (format!("{ABB},,0,0"), LifecycleField::Listing),
        ] {
            let mut previous = None;
            for rows in [
                format!("{original}\n{changed}\n{original}"),
                format!("{changed}\n{original}\n{changed}"),
            ] {
                let daily = lifecycle_rows(&rows);
                assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
                let metadata = daily.lifecycle("ABB", "INE117A01022").unwrap().metadata;
                let report = metadata.assess(Day::new(2026, 9, 4).unwrap());
                assert_eq!(report.quality, LifecycleQuality::Contradictory);
                assert!(report.issues.contains(&LifecycleIssue::Conflicting(field)));
                if let Some(previous) = &previous {
                    assert_eq!(metadata, previous);
                }
                previous = Some(metadata.clone());
            }
        }
        let daily = lifecycle_rows(&format!("{original}\n{original}"));
        assert_eq!(daily.by_symbol.len(), 1);
        assert_eq!(
            daily
                .lifecycle("ABB", "INE117A01022")
                .unwrap()
                .metadata
                .assess(Day::new(2026, 9, 4).unwrap())
                .quality,
            LifecycleQuality::Partial
        );
    }

    #[test]
    fn lifecycle_identity_mismatch_and_aliases_never_borrow_another_row() {
        let daily = lifecycle_rows(&format!(
            "{ABB},1296345600,0,0\n14,OTHER,BE,OTHERISIN,1,0,0,0"
        ));
        for (symbol, isin) in [
            ("ABB", "WRONG"),
            ("OTHER", "OTHERISIN"),
            ("abb", "INE117A01022"),
            ("ABB ", "INE117A01022"),
            ("ALIAS", "INE117A01022"),
            ("ABB", ""),
        ] {
            assert!(
                daily
                    .lifecycle(symbol, isin)
                    .unwrap_err()
                    .contains("UNVERIFIED")
            );
        }
        for conflicting in [
            "13,ABB,EQ,WRONG,1,0,0,0",
            "14,ABB,EQ,INE117A01022,1,0,0,0",
            "13,OTHER,EQ,OTHERISIN,1,0,0,0",
            "13,ABB,EQ,INE117A01022,0,0,0,0",
        ] {
            assert!(
                DailyEligibility::parse(&format!(
                    "{HEADER},ListgDt,RmvlDt,RadmssnDt\n{ABB},0,0,0\n{conflicting}\n"
                ))
                .unwrap_err()
                .contains("conflicting")
            );
        }
    }

    #[test]
    fn explicit_flags_are_distinct_from_missing_evidence() {
        let daily = parse(&format!("{ABB}\n14,OTHER,EQ,OTHERISIN,0")).unwrap();
        assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
        assert_eq!(daily.eligibility("OTHER", "OTHERISIN"), Ok(false));
        assert!(
            daily
                .eligibility("ABSENT", "OTHERISIN")
                .unwrap_err()
                .contains("UNVERIFIED")
        );
    }

    #[test]
    fn bom_crlf_reordered_quoted_headers_and_extra_columns_work() {
        let text = "\u{feff}\"ISIN\",SctySrs,FinInstrmId,Note,ElgbltyClsgAuctnSsn,TckrSymb\r\nINE117A01022,EQ,13,\"an \"\"exact\"\" name, here\",1,ABB\r\n";
        let daily = DailyEligibility::parse(text).unwrap();
        assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
    }

    #[test]
    fn all_required_headers_are_mandatory_and_duplicates_refuse() {
        for removed in REQUIRED {
            let header = REQUIRED
                .into_iter()
                .filter(|name| *name != removed)
                .collect::<Vec<_>>()
                .join(",");
            assert!(
                DailyEligibility::parse(&header)
                    .unwrap_err()
                    .contains(removed)
            );
        }
        for header in [format!("{HEADER},ISIN"), format!("{HEADER},")] {
            assert!(
                DailyEligibility::parse(&header)
                    .unwrap_err()
                    .contains("header")
            );
        }
    }

    #[test]
    fn missing_bad_or_padded_flags_refuse_the_whole_master() {
        for flag in ["", "2", "-1", "true", "01", "1 ", " 0"] {
            let why = parse(&format!("{ABB}\n14,OTHER,EQ,OTHERISIN,{flag}")).unwrap_err();
            assert!(why.contains("must be exactly 0 or 1"), "{why}");
        }
        assert!(
            parse("13,ABB,EQ,INE117A01022")
                .unwrap_err()
                .contains("width")
        );
    }

    #[test]
    fn identical_duplicates_are_idempotent() {
        let daily = parse(&format!("{ABB}\n{ABB}")).unwrap();
        assert_eq!(daily.by_symbol.len(), 1);
        assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
    }

    #[test]
    fn unrelated_series_flags_and_identity_blanks_do_not_refuse_eq_evidence() {
        let daily = parse(&format!(
            "{ABB}\n,OTHER,BE,,\n13,ABB,BE,DIFFERENTISIN,x\n,UNLISTED,UNKNOWN,,not-a-flag\n,, , ,\n14,OTHER,,OTHERISIN,"
        )).unwrap();
        assert_eq!(daily.by_symbol.len(), 1);
        assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
        assert!(daily.eligibility("OTHER", "OTHERISIN").is_err());
        assert!(daily.eligibility("UNLISTED", "OTHERISIN").is_err());
    }

    #[test]
    fn duplicate_flag_isin_native_id_and_reverse_id_conflicts_refuse() {
        for other in [
            "13,ABB,EQ,INE117A01022,0",
            "13,ABB,EQ,DIFFERENTISIN,1",
            "14,ABB,EQ,INE117A01022,1",
            "13,OTHER,EQ,OTHERISIN,1",
        ] {
            for rows in [format!("{ABB}\n{other}"), format!("{other}\n{ABB}")] {
                assert!(parse(&rows).unwrap_err().contains("conflicting"));
            }
        }
    }

    #[test]
    fn exact_symbol_isin_and_eq_series_are_required_for_lookup() {
        let daily = parse(&format!(
            "{ABB}\n14,OTHER,BE,OTHERISIN,1\n15,ABB,BE,DIFFERENTISIN,0"
        ))
        .unwrap();
        for (symbol, isin) in [
            ("ABB", "WRONG"),
            ("OTHER", "OTHERISIN"),
            ("abb", "INE117A01022"),
            ("ABB ", "INE117A01022"),
            ("ABB", "INE117A01022 "),
            ("ABB", ""),
        ] {
            assert!(
                daily.eligibility(symbol, isin).is_err(),
                "{symbol:?} {isin:?}"
            );
        }
        assert_eq!(daily.eligibility("ABB", "INE117A01022"), Ok(true));
        assert!(
            parse("14,OTHER,BE,OTHERISIN,1")
                .unwrap_err()
                .contains("no EQ")
        );
        assert!(parse("13,ABB,,INE117A01022,1").is_err());
    }

    #[test]
    fn missing_or_invalid_eq_identity_fields_refuse() {
        for row in [
            ",ABB,EQ,INE117A01022,1",
            "not-id,ABB,EQ,INE117A01022,1",
            "13,,EQ,INE117A01022,1",
            "13,ABB,EQ,,1",
            "13, ABB,EQ,INE117A01022,1",
        ] {
            assert!(parse(row).is_err(), "{row}");
        }
        assert!(
            parse(&format!(
                "13,{},EQ,ISIN,1",
                "X".repeat(MAX_IDENTITY_BYTES + 1)
            ))
            .is_err()
        );
    }

    #[test]
    fn malformed_csv_and_empty_input_fail_closed() {
        for body in ["", HEADER, "<html>not a master</html>"] {
            assert!(DailyEligibility::parse(body).is_err(), "{body}");
        }
        for row in [
            "13,\"ABB,EQ,ISIN,1",
            "13,A\"BB,EQ,ISIN,1",
            "13,\"ABB\"x,EQ,ISIN,1",
            "13,\"AB\nB\",EQ,ISIN,1",
            "13,ABB,EQ,ISIN,1,extra",
            "13,ABB,EQ,ISIN\0,1",
            "",
        ] {
            assert!(parse(row).is_err(), "{row:?}");
        }
    }

    #[test]
    fn size_row_and_field_limits_are_enforced() {
        assert!(
            DailyEligibility::parse(&"x".repeat(MAX_BYTES + 1))
                .unwrap_err()
                .contains("byte limit")
        );
        assert!(
            fields(&"x".repeat(MAX_RECORD_BYTES + 1))
                .unwrap_err()
                .contains("record")
        );
        assert!(
            fields(&vec!["x"; MAX_FIELDS + 1].join(","))
                .unwrap_err()
                .contains("field limit")
        );
        assert!(
            fields(&vec!["\"x\""; MAX_FIELDS + 1].join(","))
                .unwrap_err()
                .contains("field limit")
        );
        let many = format!("{HEADER}\n{}", format!("{ABB}\n").repeat(MAX_ROWS + 1));
        assert!(
            DailyEligibility::parse(&many)
                .unwrap_err()
                .contains("data-record limit")
        );
    }

    #[test]
    fn schedule_switches_on_the_dated_boundary_without_carry_forward() {
        let before = Day::new(2026, 8, 2).unwrap();
        let first = Day::new(2026, 8, 3).unwrap();
        let second = Day::new(2026, 8, 4).unwrap();
        let missing = Day::new(2026, 8, 5).unwrap();
        let mut schedule = Schedule::default();
        assert_eq!(schedule.close(before), Ok(930));
        assert!(schedule.close(first).unwrap_err().contains("UNVERIFIED"));
        schedule.insert(first, true).unwrap();
        schedule.insert(second, false).unwrap();
        assert_eq!(schedule.close(first), Ok(915));
        assert_eq!(schedule.close(second), Ok(930));
        assert!(schedule.close(missing).unwrap_err().contains("no dated"));
        schedule.insert(before, true).unwrap();
        assert_eq!(schedule.close(before), Ok(930));
    }

    #[test]
    fn schedule_repeats_are_idempotent_conflicts_preserve_prior_value() {
        let day = Day::new(2026, 9, 4).unwrap();
        for eligible in [true, false] {
            let mut schedule = Schedule::default();
            schedule.insert(day, eligible).unwrap();
            schedule.insert(day, eligible).unwrap();
            let original = schedule.close(day).unwrap();
            assert!(
                schedule
                    .insert(day, !eligible)
                    .unwrap_err()
                    .contains("conflicting")
            );
            assert_eq!(schedule.close(day), Ok(original));
            assert_eq!(schedule.closes.len(), 1);
            assert_eq!(schedule.clone().close(day), Ok(original));
        }
    }

    fn read_actual_master(path: &std::path::Path) -> DailyEligibility {
        use std::io::Read;

        let file = std::fs::File::open(path).unwrap();
        let mut compressed = Vec::new();
        file.take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut compressed)
            .unwrap();
        assert!(compressed.len() <= 4 * 1024 * 1024);
        let mut decoder = flate2::bufread::GzDecoder::new(compressed.as_slice());
        let mut text = String::new();
        (&mut decoder)
            .take(u64::try_from(MAX_BYTES + 1).unwrap())
            .read_to_string(&mut text)
            .unwrap();
        assert!(text.len() <= MAX_BYTES);
        assert!(decoder.get_ref().is_empty(), "unconsumed gzip bytes");
        let header = fields(text.lines().next().unwrap()).unwrap();
        assert_eq!(header.len(), 120, "actual NSE MII header width");
        DailyEligibility::parse(&text).unwrap()
    }

    /// Read-only local evidence audit, independent of the calendar's measured
    /// horizon. A mismatch is reported, never replaced by an alias or ISIN join.
    /// Run explicitly with `BRUTEX_NSE_CASH_SAMPLE_DIR` naming the supplied root.
    #[test]
    #[ignore = "requires BRUTEX_NSE_CASH_SAMPLE_DIR containing the 25 official dated gzip samples"]
    fn actual_dated_masters_match_every_current_fno_cash_identity() {
        use brutex_core::universe::{FNO_UNDERLYINGS, NTM_INDEX, nse_isin};

        let root = std::path::PathBuf::from(
            std::env::var_os("BRUTEX_NSE_CASH_SAMPLE_DIR").expect("local sample directory"),
        );
        let members: Vec<_> = FNO_UNDERLYINGS
            .into_iter()
            .filter(|symbol| NTM_INDEX.contains(symbol))
            .collect();
        assert_eq!(members.len(), 208, "current F&O/total-market intersection");
        let first = Day::new(2026, 8, 3).unwrap().days_from_epoch();
        let last = Day::new(2026, 9, 4).unwrap().days_from_epoch();
        let mut days = 0;
        let mut mismatches = Vec::new();
        for number in first..=last {
            let day = Day::from_days(number).unwrap();
            let path = root.join(format!(
                "NSE_CM_security_{:02}{:02}{:04}.csv.gz",
                day.day(),
                day.month(),
                day.year()
            ));
            if !path.try_exists().unwrap() {
                continue;
            }
            days += 1;
            let master = read_actual_master(&path);
            let mut eligible = 0;
            let mut ineligible = 0;
            for symbol in &members {
                let isin = nse_isin(symbol).expect("core cash identity must carry an ISIN");
                match master.eligibility(symbol, isin.as_str()) {
                    Ok(true) => eligible += 1,
                    Ok(false) => {
                        ineligible += 1;
                        eprintln!("{day} {symbol} ISIN={isin}: explicit eligibility=0");
                    }
                    Err(why) => {
                        let same_symbol = master.by_symbol.get(*symbol);
                        let mut same_isin: Vec<_> = master
                            .by_symbol
                            .iter()
                            .filter(|(_, identity)| identity.isin == isin.as_str())
                            .map(|(name, identity)| format!("{name}: {identity:?}"))
                            .collect();
                        same_isin.sort_unstable();
                        mismatches.push(format!("{day} {symbol} expected_ISIN={isin}: {why}; same_symbol={same_symbol:?}; same_ISIN_rows={same_isin:?}"));
                    }
                }
            }
            eprintln!(
                "{day}: exact_eligible={eligible} exact_ineligible={ineligible} mismatches={}",
                members.len() - eligible - ineligible
            );
        }
        assert_eq!(days, 25, "all supplied dated masters must be inspected");
        for mismatch in &mismatches {
            eprintln!("{mismatch}");
        }
        assert!(
            mismatches.is_empty(),
            "{} dated current-basket identities remain UNVERIFIED",
            mismatches.len()
        );
    }
}
