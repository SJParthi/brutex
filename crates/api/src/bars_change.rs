//! Why a bar's change is not a number: the D-3325 refusals, decided per
//! series from its segment, its underlying, the exchange calendar, the
//! session phases and the import's lineage ledger.
//!
//! # What was wrong
//!
//! The change fold applied to every segment alike. GDFL BAJFINANCE 1s
//! 2025-06-16 09:15:01 served `chg = −8976` bps — the fake crash D-0018 forbids
//! — while the census refused the same month as `corporate_action_unverified`
//! (DB-010, DB-G43, DB-G74). Zerodha NMDC 1day 2022-10-27 printed +85.71% at a
//! demerger (DB-032). A stock-option series stitched across an ex-date read
//! +900% (DB-G71). Zerodha NIFTY 1min measured +15.97% across a two-month hole
//! (DB-012). The 2020-03-13 halt and the closing auction were measured across
//! as if trading had been continuous (DB-028, DB-034).
//!
//! # The order (D-3325), the first that applies wins
//!
//! 1. `not_a_derivative` — open interest of an `INDEX` or `CASH` bar (D-3322).
//! 2. `contract_adjusted` — the first bar of a contract at or after a lineage
//!    ex-date, from `<store>/imports/gdfl.lineage` (D-3331).
//! 3. `corporate_action_unverified` — the first bar of an IST session on a
//!    segment that can be re-based: `CASH` on every feed, and `FNO` whose
//!    underlying is a share. An index option is not refused here: an index
//!    never splits (D-0018).
//! 4. `previous_session_not_adjacent` — the previous stored bar is in a
//!    session earlier than the calendar session immediately before this one.
//! 5. `halt_suspected` — a suspected halt (D-3320) lies between the two bars.
//! 6. `phase_boundary` — the two bars are in different phases of one session.
//! 7. `previous_bar_not_adjacent` — on an `INDEX` series at a rung of one
//!    minute or coarser, the previous bar is not the previous bucket: inside
//!    a session the bucket one rung earlier, and across two adjacent sessions
//!    the previous session's last bucket before this session's first. A
//!    share's missing minute is a minute with no trade (D-3321) and is not
//!    refused.
//! 8. `vendor_oi_unadjusted` — open interest the vendor carried unscaled past
//!    an ex-date (D-3331).
//!
//! Rules 5, 6 and 7 are intraday: a day bar spans its session, so a halt or a
//! phase inside it is not "between" two day bars.
//!
//! # Cost
//!
//! Per bar, O(1) and no allocation: two day numbers, at most ten calendar
//! lookups walking back over closed days, the three-entry halt table, two
//! phase lookups and this series' lineage events (one or two). The lineage
//! ledger is read once per change of its stamp; a closing-auction eligibility
//! master once per day it is asked about when it LOADS (both bounded memos),
//! and once per day per request when it is refused, because a refusal is not
//! remembered past its request (AF-03, D-4963).
//!
//! UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
//! "Cost claims of the /db fix round that name no proof").

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use brutex_core::instrument::Segment;
use brutex_core::vendor::Vendor;
use pull::calendar::{self, CasEligibility, DayKind, Phase};
use pull::vendor::Venue;
use store::format::Bar;
use store::path::Timeframe;

use super::series::{MICROS, Memo, SeriesKey, day_of, day_start};

/// Calendar days a session-adjacency check walks back over closed days
/// before it gives up and measures. NSE has never been closed ten days in a
/// row in the sourced span; a walk that reached the bound says nothing.
const MAX_CLOSED_WALK: i64 = 10;

/// One lineage event of one contract key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LineageEvent {
    /// 00:00 IST of the ex-date, UTC micros.
    pub(super) ex_start: i64,
    /// Up to this instant (inclusive, UTC micros) the key's open interest is
    /// the vendor's unscaled pre-ex value; `0` when none is.
    pub(super) oi_until: i64,
}

/// What the re-key's records (D-3331; WP-12c, `pull::gdfl_rekey`) say of
/// one stored key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeyStatus {
    /// The decoder files this key under `by`; the re-key published its bars
    /// there. The key stays readable, and every read of it carries the
    /// pointer.
    Superseded {
        /// The census key it continues as.
        by: String,
        /// Why its expiry moved (`closed_day` or `vendor_redated`).
        kind: &'static str,
    },
    /// The decoder refuses every day of this key; no reader serves it. The
    /// sentence every reader refuses it with.
    Retired(String),
}

/// One stored key whose bars the re-key copied INTO the asked key: the
/// reverse of [`KeyStatus::Superseded`], so a reader of the decoded key can
/// audit where its copied bars came from (D-3331; WP-12c).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Predecessor {
    /// The census key the bars were copied from (still readable, kept).
    pub(crate) of: String,
    /// Why its expiry moved (`closed_day` or `vendor_redated`).
    pub(crate) kind: &'static str,
    /// Month files and bars copied, and the BLAKE3 of those bars' records in
    /// write order, as the ledger line states them.
    pub(crate) months: u64,
    /// Bars copied.
    pub(crate) bars: u64,
    /// The digest, 64 lower-case hex digits.
    pub(crate) blake3: String,
}

/// The lineage ledger, parsed: events by census key, the re-key's statuses
/// by census key, its predecessors by the key they were copied into, and the
/// lines that did not parse.
#[derive(Debug, Clone, Default)]
struct Ledger {
    events: HashMap<String, Vec<LineageEvent>>,
    status: HashMap<String, KeyStatus>,
    predecessors: HashMap<String, Vec<Predecessor>>,
    unreadable: usize,
}

/// The parsed ledger, by its path and stamp.
static LEDGERS: Memo<PathBuf, (u64, i64, i64, Arc<Ledger>)> = Memo::new();

/// Each day's closing-auction master THAT LOADED, kept once: a receipted
/// master is immutable, so a later read could only repeat it. A day that
/// needs no master (before 2026-08-03) is kept as `None`.
///
/// A FAILED LOAD IS NEVER KEPT HERE (AF-03, p99-2). Kept as `None` it read
/// exactly like a day with no master, so one refusal — a lock held by a
/// concurrent reader, or a master not yet installed — left that day's
/// closing-auction eligibility `Unknown` until the process restarted: the
/// fallback CLAUDE.md §4 bans. A refusal is kept, with its reason, only by
/// the request that met it ([`Rules`]'s `refused`), and the next request
/// reads the day again.
static MASTERS: Memo<
    (PathBuf, u32),
    Option<Arc<pull::cash_session_cache::VerifiedLifecycleMaster>>,
> = Memo::new();

/// The D-3325 rules for one series.
pub(super) struct Rules {
    segment: Option<Segment>,
    rebase: bool,
    venue: Option<Venue>,
    rung_micros: i64,
    intraday: bool,
    index_adjacency: bool,
    lineage: Vec<LineageEvent>,
    cash_identity: Option<(String, String)>,
    masters: PathBuf,
    /// This request's refused master loads, by IST day, with their reasons:
    /// a day is read at most once per request, and again by the next one.
    refused: Mutex<HashMap<u32, String>>,
}

impl Rules {
    /// The rules of `key` on `vendor` at `timeframe`, and the ledger fault,
    /// if its ledger has lines that do not parse.
    pub(super) fn of(
        root: &Path,
        timeframe: Timeframe,
        key: &SeriesKey,
        lineage: &Lineage,
    ) -> Self {
        let segment = Segment::parse(&key.segment).ok();
        let rung_secs = i64::from(timeframe.secs());
        let intraday = rung_secs < 86_400;
        let rebase = match segment {
            Some(Segment::Cash) => true,
            Some(Segment::Fno) => !super::underlying_is_index(&key.symbol),
            Some(Segment::Index) | None => false,
        };
        let venue = segment.map(|segment| match segment {
            Segment::Index => Venue::NseIndex,
            Segment::Cash => Venue::NseCash,
            Segment::Fno => Venue::NseDerivatives,
        });
        let lineage = if key.contract.is_some() {
            lineage.events_of(&key.text())
        } else {
            Vec::new()
        };
        let cash_identity = (segment == Some(Segment::Cash))
            .then(|| brutex_core::universe::nse_isin(&key.symbol))
            .flatten()
            .map(|isin| (key.symbol.clone(), isin.as_str().to_owned()));
        Self {
            segment,
            rebase,
            venue,
            rung_micros: rung_secs * MICROS,
            intraday,
            index_adjacency: segment == Some(Segment::Index) && intraday && rung_secs >= 60,
            lineage,
            cash_identity,
            masters: root.join("session-masters"),
            refused: Mutex::new(HashMap::new()),
        }
    }

    /// Whether open interest is not applicable: an `INDEX` or `CASH` bar on
    /// every feed (D-3322).
    pub(super) fn not_a_derivative(&self) -> bool {
        matches!(self.segment, Some(Segment::Index | Segment::Cash))
    }

    /// Whether volume is not traded: an `INDEX` bar (D-2802, D-3322).
    pub(super) fn index_no_volume(&self) -> bool {
        self.segment == Some(Segment::Index)
    }

    /// Rules 2 to 7, which refuse both change columns: the first that
    /// applies, or `None` when the bar before is the comparable one.
    pub(super) fn judge(&self, before: &Bar, bar: &Bar) -> Option<&'static str> {
        if self
            .lineage
            .iter()
            .any(|event| before.ts_micros < event.ex_start && event.ex_start <= bar.ts_micros)
        {
            return Some("contract_adjusted");
        }
        let (was, is) = (day_of(before.ts_micros), day_of(bar.ts_micros));
        if self.rebase && was != is {
            return Some("corporate_action_unverified");
        }
        // THE CALENDAR IS NSE'S, so it judges only a series on an NSE venue.
        self.venue?;
        if was != is && session_before(is).is_some_and(|previous| was < previous) {
            return Some("previous_session_not_adjacent");
        }
        if !self.intraday {
            return None;
        }
        if self.halted_between(before.ts_micros, bar.ts_micros) {
            return Some("halt_suspected");
        }
        if was == is && self.phases_differ(was, before.ts_micros, bar.ts_micros) {
            return Some("phase_boundary");
        }
        if self.index_adjacency && !self.adjacent_buckets(was, is, before.ts_micros, bar.ts_micros)
        {
            return Some("previous_bar_not_adjacent");
        }
        None
    }

    /// Whether `before` is the bucket immediately before `at` on this INDEX
    /// rung (D-3325 rule 7).
    ///
    /// Inside one session, the two stamps are one rung apart. Across sessions
    /// (rule 4 has already refused a session that is not the adjacent one),
    /// `at` must open its session and `before` must close its own: a bucket of
    /// continuous trading one rung before `at` on its day, or one rung after
    /// `before` on its day, is a bucket the store does not hold: a session
    /// whose minute file stops early (D-3324 measured Zerodha NIFTY 1min on
    /// 2021-02-24 stopping at 10:08) would otherwise give the next session's
    /// first bar an "overnight" change from mid-morning. A phase the calendar
    /// does not know is not a bucket and refuses nothing.
    fn adjacent_buckets(&self, was: i64, is: i64, before: i64, at: i64) -> bool {
        if was == is {
            return at - before == self.rung_micros;
        }
        let continuous = |day: i64, ts: i64| {
            day_of(ts) == day && self.phase_at(day, ts) == Some(Phase::Continuous)
        };
        !continuous(is, at - self.rung_micros) && !continuous(was, before + self.rung_micros)
    }

    /// Rule 8: the bar's open interest is the vendor's unscaled pre-ex value.
    pub(super) fn oi_unadjusted(&self, bar: &Bar) -> bool {
        self.lineage.iter().any(|event| {
            event.oi_until > 0 && event.ex_start <= bar.ts_micros && bar.ts_micros <= event.oi_until
        })
    }

    /// Whether a suspected halt's seconds on this series' venue lie between
    /// the two bars' CLOSES.
    ///
    /// A bar's close is the last instant of its bucket, so a one-minute bar
    /// stamped 10:21 closes after a 10:21:01 resume and the bar after it is
    /// measured, while the 10:21 bar itself — whose predecessor closed inside
    /// the halt — is refused. On one-second bars the close is the stamp.
    fn halted_between(&self, before: i64, at: i64) -> bool {
        let Some(venue) = self.venue else {
            return false;
        };
        let close = |ts: i64| ts.saturating_add(self.rung_micros - 1);
        calendar::HALTS.iter().any(|halt| {
            let seconds = halt.seconds(venue);
            let midnight = day_start(halt.day);
            let from = midnight + i64::from(seconds.from) * MICROS;
            let resume = midnight + i64::from(seconds.resume) * MICROS;
            halt.evidence.is_suspected() && close(before) < resume && close(at) >= from
        })
    }

    /// Whether the two instants of IST day `day` are in different phases,
    /// both of them known.
    fn phases_differ(&self, day: i64, before: i64, at: i64) -> bool {
        match (self.phase_at(day, before), self.phase_at(day, at)) {
            (Some(was), Some(is)) => {
                was != is && was != Phase::Unverified && is != Phase::Unverified
            }
            _ => false,
        }
    }

    /// The phase of instant `ts` on IST day `day`, for this series' venue.
    ///
    /// A rung finer than a minute is labelled to the SECOND
    /// (`calendar::phase_at_second`, DB-B43): a halt begins mid-minute
    /// (09:21:10 for the index on 2020-03-13) and its unchanged-price seconds are
    /// halted, never `continuous`.
    fn phase_at(&self, day: i64, ts: i64) -> Option<Phase> {
        let venue = self.venue?;
        let since = ts - day_start(day);
        let minute = u16::try_from(since.div_euclid(60 * MICROS)).ok()?;
        // ELIGIBILITY IS READ ONLY WHERE THE PHASE DEPENDS ON IT: a share,
        // from the closing auction's first minute on.
        let cas = if venue == Venue::NseCash && minute >= calendar::CAS_CONTINUOUS_CLOSE {
            self.eligibility(day)
        } else {
            CasEligibility::Unknown
        };
        if self.rung_micros < 60 * MICROS {
            let second = u32::try_from(since.div_euclid(MICROS)).ok()?;
            return Some(calendar::phase_at_second(venue, day, second, cas));
        }
        Some(calendar::phase_at(venue, day, minute, cas))
    }

    /// The wire word for `bar`'s phase, or `None` on a day rung (a day bar
    /// spans its session) or a series with no venue.
    pub(super) fn phase_word(&self, bar: &Bar) -> Option<&'static str> {
        if !self.intraday {
            return None;
        }
        let day = day_of(bar.ts_micros);
        self.phase_at(day, bar.ts_micros).map(phase_word)
    }

    /// This share's closing-auction flag on IST day `day`, from that day's
    /// dated NSE master (charter §9), or `Unknown` — never `NotEligible` for
    /// a master or a row that is not there.
    fn eligibility(&self, day: i64) -> CasEligibility {
        let Some((symbol, isin)) = &self.cash_identity else {
            return CasEligibility::Unknown;
        };
        let Ok(days) = u32::try_from(day) else {
            return CasEligibility::Unknown;
        };
        self.master(days)
            .and_then(|master| master.cas_eligibility(symbol, isin).ok())
            .unwrap_or(CasEligibility::Unknown)
    }

    /// IST day `days`'s master: the process-wide memo's when it loaded
    /// before, else read now. A refusal is remembered for this request only,
    /// with its reason (AF-03). Expected O(1) per bar: a hit in either memo
    /// is one hash probe, and a miss reads the day once per request.
    fn master(&self, days: u32) -> Option<Arc<pull::cash_session_cache::VerifiedLifecycleMaster>> {
        let memo = (self.masters.clone(), days);
        if let Some(kept) = MASTERS.get(&memo) {
            return kept;
        }
        if self
            .refused
            .lock()
            .is_ok_and(|refused| refused.contains_key(&days))
        {
            return None;
        }
        let Some(civil) = pull::session::Day::from_days(days)
            .ok()
            .filter(|civil| pull::vendor::cash_auction_eligibility_required(*civil))
        else {
            MASTERS.put(memo, None);
            return None;
        };
        match pull::cash_session_cache::read_local_lifecycle(&self.masters, civil) {
            Ok(loaded) => {
                let loaded = Arc::new(loaded);
                MASTERS.put(memo, Some(Arc::clone(&loaded)));
                Some(loaded)
            }
            Err(why) => {
                if let Ok(mut refused) = self.refused.lock() {
                    refused.insert(days, why);
                }
                None
            }
        }
    }

    /// Why this request could not load IST day `days`'s master, if it could
    /// not: the reason its `Unknown` eligibility stands on.
    #[cfg(test)]
    fn refusal(&self, days: u32) -> Option<String> {
        self.refused.lock().ok()?.get(&days).cloned()
    }
}

/// The wire word for a phase.
const fn phase_word(phase: Phase) -> &'static str {
    match phase {
        Phase::Closed => "closed",
        Phase::BeforeOpen => "before_open",
        Phase::PreOpen => "pre_open",
        Phase::Continuous => "continuous",
        Phase::Halted => "halted",
        Phase::ReopeningAuction => "reopening_auction",
        Phase::Break => "break",
        Phase::ClosingAuction => "cas",
        Phase::PostClose => "post_close",
        Phase::Unverified => "unverified",
    }
}

/// The calendar session immediately before IST day `day`, or `None` when the
/// calendar cannot say (outside its sourced span, a day of unmeasured length
/// on the way, or ten closed days in a row).
fn session_before(day: i64) -> Option<i64> {
    let mut at = day - 1;
    while at >= day - MAX_CLOSED_WALK {
        match calendar::exchange_kind_of(at) {
            DayKind::Open(_) => return Some(at),
            DayKind::Closed => at -= 1,
            DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => return None,
        }
    }
    None
}

/// The import's lineage ledger as one request reads it: once, whatever the
/// number of series, so a request over a 137-contract chain `stat`s the
/// ledger once and not 137 times.
pub(super) struct Lineage {
    ledger: Option<Arc<Ledger>>,
    /// The ledger's fault, said once on the answer: it would not read, or
    /// some of its lines do not parse.
    pub(super) fault: Option<String>,
}

impl Lineage {
    /// The ledger of `vendor` under `root`: GDFL's
    /// `<store>/imports/gdfl.lineage` (D-3331); no other feed keeps one.
    pub(super) fn read(root: &Path, vendor: Vendor) -> Self {
        let none = Self {
            ledger: None,
            fault: None,
        };
        if vendor != Vendor::Gdfl {
            return none;
        }
        let path = pull::gdfl_lineage::lineage_path(root);
        let Ok(meta) = std::fs::metadata(&path) else {
            return none;
        };
        let stamp = (
            meta.len(),
            std::os::unix::fs::MetadataExt::mtime(&meta),
            std::os::unix::fs::MetadataExt::mtime_nsec(&meta),
        );
        let ledger = match LEDGERS.get(&path) {
            Some((len, secs, nanos, ledger)) if (len, secs, nanos) == stamp => ledger,
            _ => {
                // A LEDGER THAT EXISTS AND WILL NOT READ IS SAID, NOT READ AS
                // EMPTY.
                let text = match std::fs::read_to_string(&path) {
                    Ok(text) => text,
                    Err(why) => {
                        return Self {
                            ledger: None,
                            fault: Some(format!(
                                "cannot read {}: {why}; no lineage was applied",
                                path.display()
                            )),
                        };
                    }
                };
                let ledger = Arc::new(parse_ledger(&text));
                LEDGERS.put(
                    path.clone(),
                    (stamp.0, stamp.1, stamp.2, Arc::clone(&ledger)),
                );
                ledger
            }
        };
        let fault = (ledger.unreadable > 0).then(|| {
            format!(
                "{} line(s) of {} do not parse as a lineage record and were not applied",
                ledger.unreadable,
                path.display()
            )
        });
        Self {
            ledger: Some(ledger),
            fault,
        }
    }

    /// What the re-key's records say of census key `key`, if anything. One
    /// hash probe.
    pub(crate) fn status_of(&self, key: &str) -> Option<&KeyStatus> {
        self.ledger
            .as_ref()
            .and_then(|ledger| ledger.status.get(key))
    }

    /// The stored keys whose bars the re-key copied into census key `key`,
    /// in ledger order; empty for every other key. One hash probe.
    pub(crate) fn predecessors_of(&self, key: &str) -> &[Predecessor] {
        self.ledger
            .as_ref()
            .and_then(|ledger| ledger.predecessors.get(key))
            .map_or(&[], Vec::as_slice)
    }

    /// The events of census key `key`.
    fn events_of(&self, key: &str) -> Vec<LineageEvent> {
        self.ledger
            .as_ref()
            .and_then(|ledger| ledger.events.get(key).cloned())
            .unwrap_or_default()
    }
}

/// The ledger's `lineage` and `stitched` records, by the key whose first
/// post-ex bar they label, and the re-key's `superseded` and `retired`
/// records by the key they name. `alias` records are the decoder's, not a
/// read's.
///
/// A superseded key's events also label the key it continues as: its bars
/// are there now, at the same instants (D-3331; WP-12c).
fn parse_ledger(text: &str) -> Ledger {
    let mut ledger = Ledger {
        events: HashMap::with_capacity(64),
        status: HashMap::with_capacity(16),
        predecessors: HashMap::with_capacity(16),
        unreadable: 0,
    };
    let mut moved: Vec<(String, String)> = Vec::with_capacity(16);
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if line.starts_with("alias ") {
            continue;
        }
        if let Some(record) = pull::gdfl_lineage::SupersededRecord::parse(line) {
            let old = pull::gdfl_lineage::option_key(record.underlying, record.old);
            let by = pull::gdfl_lineage::option_key(record.underlying, record.new);
            moved.push((old.clone(), by.clone()));
            ledger
                .predecessors
                .entry(by.clone())
                .or_default()
                .push(Predecessor {
                    of: old.clone(),
                    kind: record.kind.as_str(),
                    months: record.months,
                    bars: record.bars,
                    blake3: record.digest_hex(),
                });
            let _ = ledger.status.insert(
                old,
                KeyStatus::Superseded {
                    by,
                    kind: record.kind.as_str(),
                },
            );
            continue;
        }
        if let Some(record) = pull::gdfl_lineage::RetiredRecord::parse(line) {
            let _ = ledger.status.insert(
                pull::gdfl_lineage::option_key(record.underlying, record.key),
                KeyStatus::Retired(record.refusal()),
            );
            continue;
        }
        if let Some(record) = pull::gdfl_lineage::LineageRecord::parse(line) {
            ledger
                .events
                .entry(pull::gdfl_lineage::option_key(
                    record.underlying,
                    record.new,
                ))
                .or_default()
                .push(LineageEvent {
                    ex_start: day_start(i64::from(record.ex_date.days_from_epoch())),
                    oi_until: record.oi_unadjusted_until.saturating_mul(MICROS),
                });
        } else if let Some(record) = pull::gdfl_lineage::StitchedRecord::parse(line) {
            ledger
                .events
                .entry(pull::gdfl_lineage::option_key(
                    record.underlying,
                    record.key,
                ))
                .or_default()
                .push(LineageEvent {
                    ex_start: day_start(i64::from(record.ex_date.days_from_epoch())),
                    oi_until: 0,
                });
        } else if pull::gdfl_lineage::EventRecord::parse(line).is_none()
            && pull::gdfl_lineage::StoppedRecord::parse(line).is_none()
        {
            // An `event` and a `stopped` record (D-3355) are read and label
            // no bar: a stopped key's next contract, if one ever trades
            // under it, is a `stitched` record, which does.
            ledger.unreadable = ledger.unreadable.saturating_add(1);
        }
    }
    for (old, by) in moved {
        if let Some(events) = ledger.events.get(&old).cloned() {
            ledger.events.entry(by).or_default().extend(events);
        }
    }
    ledger
}

/// The refusal a reader answers a RETIRED key of `vendor` with (D-3331;
/// WP-12c), or `None` for every other key: one `stat` of the ledger and one
/// hash probe, through the same memo the window reads it by. A ledger that
/// will not read refuses nothing here; the window route names its fault.
pub(in crate::bars) fn retired_refusal(root: &Path, vendor: Vendor, key: &str) -> Option<String> {
    match Lineage::read(root, vendor).status_of(key) {
        Some(KeyStatus::Retired(why)) => Some(why.clone()),
        Some(KeyStatus::Superseded { .. }) | None => None,
    }
}

/// The sentence an AUDIT reader (`/provenance.json`) adds about a key the
/// re-key moved, copied into or retired (D-3331; WP-12c), or `None` for any
/// other key: the provenance of a copied month is the import of the key it
/// was copied from, and a reader must be told so rather than read the import
/// days as this key's own. One `stat` of the ledger and two hash probes.
pub(in crate::bars) fn rekey_note(root: &Path, vendor: Vendor, key: &str) -> Option<String> {
    let lineage = Lineage::read(root, vendor);
    let mut said: Vec<String> = Vec::with_capacity(2);
    match lineage.status_of(key) {
        Some(KeyStatus::Superseded { by, kind }) => said.push(format!(
            "{key} is superseded by {by} ({kind}, D-3331): `cli gdfl-rekey` copied its bars \
             there as new month files; these files are kept exactly as they were"
        )),
        Some(KeyStatus::Retired(why)) => said.push(why.clone()),
        None => {}
    }
    for one in lineage.predecessors_of(key) {
        said.push(format!(
            "the bars of {} were copied into {key} by `cli gdfl-rekey` ({}, D-3331; {} bar(s) in \
             {} month file(s), BLAKE3 {} in imports/gdfl.lineage): the import days below are \
             the days {} was imported on, and the copy landed when this file was last modified",
            one.of, one.kind, one.bars, one.months, one.blake3, one.of
        ));
    }
    (!said.is_empty()).then(|| said.join("; "))
}

/// What the window's tests read of the ledger parse.
#[cfg(test)]
pub(super) mod for_tests {
    /// How many keys carry events, and how many lines did not parse.
    pub(in crate::bars) fn ledger_counts(text: &str) -> (usize, usize) {
        let ledger = super::parse_ledger(text);
        (ledger.events.len(), ledger.unreadable)
    }

    /// The re-key status of `key` in a ledger text, and the events it labels.
    pub(in crate::bars) fn ledger_status(
        text: &str,
        key: &str,
    ) -> (Option<super::KeyStatus>, usize) {
        let ledger = super::parse_ledger(text);
        (
            ledger.status.get(key).cloned(),
            ledger.events.get(key).map_or(0, Vec::len),
        )
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "tests fail immediately when fixture setup fails"
)]
mod tests {
    use super::*;

    /// A gzip member of one stored (uncompressed) deflate block: `api` has
    /// no compressor, and the master reader only needs a valid member.
    fn stored_gzip(text: &[u8]) -> Vec<u8> {
        let mut crc = !0u32;
        for &byte in text {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        let len = u16::try_from(text.len()).expect("one stored block");
        let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 1];
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(text);
        out.extend_from_slice(&(!crc).to_le_bytes());
        out.extend_from_slice(&u32::try_from(text.len()).expect("small").to_le_bytes());
        out
    }

    /// **A FAILED MASTERS LOAD IS NOT REMEMBERED (AF-03, p99-2).** A share's
    /// 15:20 minute on 2026-08-03 reads `unverified` while that day's master
    /// is not there, and the request keeps the refusal with its reason, so
    /// it reads the day once and not once per bar. The master then lands, as
    /// the pull installs it: the NEXT request reads the day again and carries
    /// the closing-auction phase. The process-wide memo kept the refusal as
    /// `None`, and the day stayed `unverified` until a restart.
    #[test]
    fn a_failed_masters_load_is_not_remembered() {
        let root = std::env::temp_dir().join(format!(
            "brutex-af03-masters-{}-{}",
            std::process::id(),
            line!()
        ));
        let masters = root.join("session-masters");
        std::fs::create_dir_all(&masters).expect("a scratch store");
        let key = SeriesKey::new("NSE", "CASH", "RELIANCE", None);
        let rules = || {
            Rules::of(
                &root,
                Timeframe::MINUTE_1,
                &key,
                &Lineage::read(&root, Vendor::Zerodha),
            )
        };
        let on = pull::session::Day::new(2026, 8, 3).expect("a CAS day");
        let days = on.days_from_epoch();
        let minute = |hh: i64, mm: i64| Bar {
            ts_micros: day_start(i64::from(days)) + (hh * 3_600 + mm * 60) * MICROS,
            open: 100,
            high: 100,
            low: 100,
            close: 100,
            volume: 1,
            open_interest: i64::MIN,
        };
        let auction = minute(15, 20);

        let first = rules();
        assert_eq!(first.phase_word(&auction), Some("unverified"));
        let why = first.refusal(days).expect("the refusal is kept, named");
        assert!(
            why.contains("UNVERIFIED") && why.contains("absent"),
            "{why}"
        );

        let isin = brutex_core::universe::nse_isin("RELIANCE").expect("an F&O share");
        let csv = format!(
            "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n7,RELIANCE,EQ,{},1\n",
            isin.as_str()
        );
        pull::cash_session_cache::install(&masters, on, &stored_gzip(csv.as_bytes()))
            .expect("the master lands");

        // The same request keeps its refusal: one read of the day per request.
        assert_eq!(first.phase_word(&auction), Some("unverified"));
        // The next request reads the day again, and the phase is carried.
        let second = rules();
        assert_eq!(second.phase_word(&auction), Some("cas"));
        assert_eq!(second.phase_word(&minute(15, 10)), Some("continuous"));
        assert_eq!(second.refusal(days), None);
        // And it is kept: the master is immutable once receipted.
        assert_eq!(rules().phase_word(&auction), Some("cas"));
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// Tests written to kill mutants a cloud mutation pass left alive in this
/// file (`reports/cloud-mut-api-a-20261016.md`). Each names the mutant.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "tests fail immediately when fixture setup fails"
)]
mod cloud_mutant_tests {
    use super::*;

    /// A root no test writes to: the rules read nothing from it but the
    /// session masters, and those are absent.
    fn nowhere() -> PathBuf {
        PathBuf::from("/nonexistent-cloud-mutant-root")
    }

    fn rules(segment: &str, symbol: &str, timeframe: Timeframe) -> Rules {
        let root = nowhere();
        Rules::of(
            &root,
            timeframe,
            &SeriesKey::new("NSE", segment, symbol, None),
            &Lineage::read(&root, Vendor::Dhan),
        )
    }

    fn bar_at(ts_micros: i64) -> Bar {
        Bar {
            ts_micros,
            open: 100,
            high: 100,
            low: 100,
            close: 100,
            volume: 1,
            open_interest: 10,
        }
    }

    /// 2020-03-13, the first recorded (suspected) halt, and the seconds the
    /// index spent in it.
    fn index_halt() -> (i64, pull::calendar::HaltSeconds) {
        let halt = pull::calendar::halt_on(18_334).expect("the 2020-03-13 halt");
        (halt.day, halt.seconds(Venue::NseIndex))
    }

    /// **A BAR STAMPED AT THE EX-DATE'S OWN MIDNIGHT IS ALREADY POST-EX**:
    /// the change from it to the next bar crosses no ex-date and is measured.
    /// Kills `< with <=` in `judge`'s lineage test, which called that pair
    /// `contract_adjusted`. A day bar may be stamped at 00:00 IST.
    #[test]
    fn a_bar_at_the_ex_dates_midnight_is_already_post_ex() {
        let mut daily = rules("FNO", "NIFTY", Timeframe::DAY_1);
        let ex_start = day_start(20_000);
        daily.lineage = vec![LineageEvent {
            ex_start,
            oi_until: 0,
        }];
        let on_ex = bar_at(ex_start);
        let next = bar_at(ex_start + 86_400 * MICROS);
        assert_ne!(daily.judge(&on_ex, &next), Some("contract_adjusted"));
        // And a pair that does cross it is refused, so the event is live.
        let before_ex = bar_at(ex_start - 86_400 * MICROS);
        assert_eq!(daily.judge(&before_ex, &on_ex), Some("contract_adjusted"));
    }

    /// **AN EVENT THAT CARRIED NO UNSCALED OPEN INTEREST (`oi_until == 0`)
    /// LABELS NO BAR, EVEN ONE STAMPED AT OR BEFORE THE EPOCH.** Kills
    /// `> with >=` in `oi_unadjusted`, which read `0` as "unscaled up to the epoch"
    /// and labelled a January-1970 bar (the store's months start at
    /// 1970-01-01 00:00 IST, before the epoch).
    #[test]
    fn an_event_with_no_unscaled_window_labels_no_bar() {
        let mut option = rules("FNO", "NIFTY", Timeframe::MINUTE_1);
        let ex_start = day_start(0);
        option.lineage = vec![LineageEvent {
            ex_start,
            oi_until: 0,
        }];
        assert!(!option.oi_unadjusted(&bar_at(-1)));
        assert!(!option.oi_unadjusted(&bar_at(ex_start)));
        // An event that names a window labels the bars inside it.
        option.lineage = vec![LineageEvent {
            ex_start,
            oi_until: 5 * MICROS,
        }];
        assert!(option.oi_unadjusted(&bar_at(-1)));
        assert!(!option.oi_unadjusted(&bar_at(6 * MICROS)));
    }

    /// **A BAR'S CLOSE IS THE LAST MICROSECOND OF ITS BUCKET**: on one-second
    /// index bars, the bar one second before the resume closes inside the
    /// halt, so the bar after it is refused `halt_suspected`. Kills `- with +`
    /// and `- with /` in `halted_between`'s `rung_micros - 1`, which pushed
    /// that close one or two microseconds past the resume.
    #[test]
    fn the_bar_before_the_resume_closes_inside_the_halt() {
        let (day, seconds) = index_halt();
        let second = |s: u32| day_start(day) + i64::from(s) * MICROS;
        let index = rules("INDEX", "NIFTY", Timeframe::SECOND_1);
        let before = bar_at(second(seconds.resume - 1));
        let at = bar_at(second(seconds.resume));
        assert_eq!(index.judge(&before, &at), Some("halt_suspected"));
    }

    /// **TWO BARS BEFORE THE HALT BEGAN ARE MEASURED**: nothing halted lies
    /// between 09:17:00 and 09:17:01 on 2020-03-13. Kills `+ with -`, `* with
    /// +` and `* with /` in `halted_between`'s `from`, each of which moved the
    /// halt's start to (or before) midnight.
    #[test]
    fn two_bars_before_the_halt_began_are_measured() {
        let (day, seconds) = index_halt();
        let second = |s: u32| day_start(day) + i64::from(s) * MICROS;
        assert!(
            9 * 3_600 + 17 * 60 + 1 < seconds.from,
            "both before the halt"
        );
        let index = rules("INDEX", "NIFTY", Timeframe::SECOND_1);
        let before = bar_at(second(9 * 3_600 + 17 * 60));
        let at = bar_at(second(9 * 3_600 + 17 * 60 + 1));
        assert_eq!(index.judge(&before, &at), None);
    }

    /// **A SHARE'S MASTER IS READ ONLY FROM THE CLOSING AUCTION'S FIRST
    /// MINUTE ON**: a 10:00 bar on a day whose phases need the master reads
    /// none, so no refusal is kept for it. Kills `&& with ||` in `phase_at`,
    /// which read (and here refused) the master for every minute of a share.
    #[test]
    fn a_shares_morning_bar_reads_no_master() {
        let share = rules("CASH", "RELIANCE", Timeframe::MINUTE_1);
        let on = pull::session::Day::new(2026, 8, 3).expect("a CAS day");
        let days = on.days_from_epoch();
        let ten = day_start(i64::from(days)) + 10 * 3_600 * MICROS;
        assert_eq!(share.phase_word(&bar_at(ten)), Some("continuous"));
        assert_eq!(share.refusal(days), None, "nothing was read");
        // The auction minute does read it, and keeps the refusal.
        let auction = day_start(i64::from(days)) + (15 * 3_600 + 20 * 60) * MICROS;
        assert_eq!(share.phase_word(&bar_at(auction)), Some("unverified"));
        assert!(share.refusal(days).is_some());
    }

    /// **THE SESSION BEFORE A DAY IS THE CALENDAR'S PREVIOUS OPEN DAY, NEVER
    /// THE DAY ITSELF**: Tuesday's is Monday, Monday's is the Friday before.
    /// Kills `- with /` in `session_before`, which started the walk at the day
    /// itself and answered it.
    #[test]
    fn the_session_before_a_day_is_the_previous_open_day() {
        let day = |d: u8| {
            i64::from(
                pull::session::Day::new(2025, 5, d)
                    .expect("a day")
                    .days_from_epoch(),
            )
        };
        // 2025-05-06 is a Tuesday, 05-05 a Monday, 05-02 a Friday.
        assert_eq!(session_before(day(6)), Some(day(5)));
        assert_eq!(session_before(day(5)), Some(day(2)));
    }

    /// **TWO DAY BARS OF ADJACENT SESSIONS ARE MEASURED**: a day bar spans its
    /// session, so the intraday rules (halt, phase, bucket) do not apply to
    /// it. Runs `judge`'s day-rung return, which no test reached.
    #[test]
    fn two_day_bars_of_adjacent_sessions_are_measured() {
        let daily = rules("INDEX", "NIFTY", Timeframe::DAY_1);
        let midnight = |d: u8| {
            day_start(i64::from(
                pull::session::Day::new(2025, 5, d)
                    .expect("a day")
                    .days_from_epoch(),
            ))
        };
        assert_eq!(
            daily.judge(&bar_at(midnight(5)), &bar_at(midnight(6))),
            None
        );
        assert_eq!(
            daily.phase_word(&bar_at(midnight(6))),
            None,
            "no phase on a day rung"
        );
    }

    /// **A SERIES ON NO NSE VENUE HAS NO HALT AND NO PHASE BETWEEN ANY TWO
    /// BARS.** `judge` stops before asking for such a series; these are the
    /// helpers' own answers, which no test reached.
    #[test]
    fn a_series_on_no_venue_has_no_halt_and_no_phase() {
        let other = rules("TESTSEG", "WINDOWTEST", Timeframe::SECOND_1);
        let (day, seconds) = index_halt();
        let second = |s: u32| day_start(day) + i64::from(s) * MICROS;
        assert!(!other.halted_between(second(seconds.from), second(seconds.resume)));
        assert!(!other.phases_differ(day, second(33_300), second(seconds.from)));
        assert_eq!(other.phase_word(&bar_at(second(33_300))), None);
    }

    /// **A SHARE THE UNIVERSE NAMES NO ISIN FOR HAS NO CLOSING-AUCTION FLAG**,
    /// and neither does a day before the epoch: both are `Unknown`, never
    /// `NotEligible`. Runs `eligibility`'s two early returns.
    #[test]
    fn eligibility_is_unknown_without_an_identity_or_a_day() {
        let unknown = rules("CASH", "NOSUCHSHARE", Timeframe::MINUTE_1);
        assert!(unknown.cash_identity.is_none());
        assert_eq!(unknown.eligibility(20_000), CasEligibility::Unknown);
        let known = rules("CASH", "RELIANCE", Timeframe::MINUTE_1);
        assert!(known.cash_identity.is_some());
        assert_eq!(known.eligibility(-1), CasEligibility::Unknown);
    }

    /// **EVERY PHASE HAS ITS OWN WIRE WORD.** Runs the four arms of
    /// `phase_word` no stored fixture reached.
    #[test]
    fn every_phase_has_its_own_wire_word() {
        let words = [
            (Phase::Closed, "closed"),
            (Phase::BeforeOpen, "before_open"),
            (Phase::PreOpen, "pre_open"),
            (Phase::Continuous, "continuous"),
            (Phase::Halted, "halted"),
            (Phase::ReopeningAuction, "reopening_auction"),
            (Phase::Break, "break"),
            (Phase::ClosingAuction, "cas"),
            (Phase::PostClose, "post_close"),
            (Phase::Unverified, "unverified"),
        ];
        for (phase, word) in words {
            assert_eq!(phase_word(phase), word, "{phase:?}");
        }
    }

    /// **A LEDGER THAT EXISTS AND WILL NOT READ IS SAID, NOT READ AS EMPTY.**
    /// A directory where `imports/gdfl.lineage` should be passes the `stat`
    /// and refuses the read: the fault names the path and says no lineage was
    /// applied. Runs `Lineage::read`'s read-refusal arm.
    #[test]
    fn a_ledger_that_will_not_read_is_said_and_applies_nothing() {
        let root = std::env::temp_dir().join(format!(
            "brutex-cloud-ledger-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(pull::gdfl_lineage::lineage_path(&root))
            .expect("a directory where the ledger should be");
        let lineage = Lineage::read(&root, Vendor::Gdfl);
        let _ = std::fs::remove_dir_all(&root);
        let fault = lineage.fault.clone().expect("the refusal is said");
        assert!(
            fault.starts_with("cannot read ") && fault.ends_with("no lineage was applied"),
            "{fault}"
        );
        assert!(fault.contains("gdfl.lineage"), "{fault}");
        assert!(lineage.ledger.is_none());
        assert_eq!(lineage.status_of("NSE-FNO-NIFTY"), None);
    }
}
