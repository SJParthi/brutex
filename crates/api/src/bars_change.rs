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
//! master once per day it is asked about (both bounded memos).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

/// The lineage ledger, parsed: events by census key, and the lines that did
/// not parse.
#[derive(Debug, Clone, Default)]
struct Ledger {
    events: HashMap<String, Vec<LineageEvent>>,
    unreadable: usize,
}

/// The parsed ledger, by its path and stamp.
static LEDGERS: Memo<PathBuf, (u64, i64, i64, Arc<Ledger>)> = Memo::new();

/// Each day's closing-auction master, loaded once.
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
    fn phase_at(&self, day: i64, ts: i64) -> Option<Phase> {
        let venue = self.venue?;
        let minute = u16::try_from((ts - day_start(day)).div_euclid(60 * MICROS)).ok()?;
        // ELIGIBILITY IS READ ONLY WHERE THE PHASE DEPENDS ON IT: a share,
        // from the closing auction's first minute on.
        let cas = if venue == Venue::NseCash && minute >= calendar::CAS_CONTINUOUS_CLOSE {
            self.eligibility(day)
        } else {
            CasEligibility::Unknown
        };
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
        let memo = (self.masters.clone(), days);
        let master = MASTERS.get(&memo).unwrap_or_else(|| {
            let loaded = pull::session::Day::from_days(days)
                .ok()
                .filter(|civil| pull::vendor::cash_auction_eligibility_required(*civil))
                .and_then(|civil| {
                    pull::cash_session_cache::read_local_lifecycle(&self.masters, civil).ok()
                })
                .map(Arc::new);
            MASTERS.put(memo, loaded.clone());
            loaded
        });
        master
            .and_then(|master| master.cas_eligibility(symbol, isin).ok())
            .unwrap_or(CasEligibility::Unknown)
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

    /// The events of census key `key`.
    fn events_of(&self, key: &str) -> Vec<LineageEvent> {
        self.ledger
            .as_ref()
            .and_then(|ledger| ledger.events.get(key).cloned())
            .unwrap_or_default()
    }
}

/// The ledger's `lineage` and `stitched` records, by the key whose first
/// post-ex bar they label. `alias` records are the decoder's, not a read's.
fn parse_ledger(text: &str) -> Ledger {
    let mut ledger = Ledger {
        events: HashMap::with_capacity(64),
        unreadable: 0,
    };
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if line.starts_with("alias ") {
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
        } else {
            ledger.unreadable = ledger.unreadable.saturating_add(1);
        }
    }
    ledger
}

/// What the window's tests read of the ledger parse.
#[cfg(test)]
pub(super) mod for_tests {
    /// How many keys carry events, and how many lines did not parse.
    pub(in crate::bars) fn ledger_counts(text: &str) -> (usize, usize) {
        let ledger = super::parse_ledger(text);
        (ledger.events.len(), ledger.unreadable)
    }
}
