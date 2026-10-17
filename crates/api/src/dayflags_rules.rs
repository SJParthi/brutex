//! The day band's read-time rules (R3-13): what `/dayflags.json` decides about
//! a day from facts it reads at request time, kept out of the record builder so
//! a later package changes a rule here and the 7,000-line route not at all.
//!
//! # What lives here
//!
//! - [`feed_rungs`]: the band's peer feed and each feed's finest rung, ONE
//!   function (R3-34 points it at the feed table; until then it is the two
//!   feeds the operator uses, answer 7).
//! - [`CasFlags`]: a share's dated closing-auction flag for one day, read from
//!   that day's NSE master exactly as `/bars/window.json` reads it
//!   (`bars_change::Rules::eligibility`), so the two routes cannot give one
//!   minute two phases (SC-CLOSE-8).
//! - [`expiry_of`]: a contract's last session, so a day after it is `expired`
//!   and owes nothing, as `/gaps.json` already says (SC-CLOSE-12).
//! - [`held_month_before`]: the newest month before a day that holds a `1day`
//!   file, found by `metadata` calls rather than opens, so a scale switch is
//!   judged against a previous bar however many months back it lies (DB-031).
//!
//! # Cost
//!
//! Every rule is O(1) a day or bounded by a named constant: [`CasFlags::of`] is
//! one hash probe a day once a master is loaded (one read of the day's master
//! per request otherwise, and only from 2026-08-03 on); [`expiry_of`] parses
//! ten bytes; [`held_month_before`] makes at most [`MONTHS_BACK`] `metadata`
//! calls, once per page, never per day.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use pull::calendar::CasEligibility;
use pull::session::Day;
use store::path::{Timeframe, YearMonth};

// ══ The band's rungs ════════════════════════════════════════════════════════

/// What a band reads for one feed: the feed's finest rung, the other feed it
/// counts beside its own, and that feed's finest rung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FeedRungs {
    /// The feed's finest stored rung: GDFL's one-second bars, Zerodha's
    /// one-minute bars.
    pub(super) finest: Timeframe,
    /// The other feed, whose bars a record counts as its `peer`.
    pub(super) peer: Vendor,
    /// The peer's finest stored rung.
    pub(super) peer_finest: Timeframe,
}

/// A feed's finest stored rung.
const fn finest_of(feed: Vendor) -> Timeframe {
    match feed {
        Vendor::Gdfl => Timeframe::SECOND_1,
        _ => Timeframe::MINUTE_1,
    }
}

/// THE ONE PLACE the band's peer and finest rungs are decided (R3-13 goal 5).
/// Every reader in `dayflags` asks this, so pointing it at the feed table is
/// one edit here.
pub(super) const fn feed_rungs(feed: Vendor) -> FeedRungs {
    let peer = match feed {
        Vendor::Gdfl => Vendor::Zerodha,
        _ => Vendor::Gdfl,
    };
    FeedRungs {
        finest: finest_of(feed),
        peer,
        peer_finest: finest_of(peer),
    }
}

/// The rung a band is asked to count, from `timeframe=` (the grid's rung):
/// `Ok(None)` when the parameter is absent, so the feed's finest is counted.
///
/// # Errors
///
/// A word that is not a rung this store files, as one sentence for a 400.
pub(super) fn asked_rung(word: &str) -> Result<Option<Timeframe>, String> {
    if word.is_empty() {
        return Ok(None);
    }
    Timeframe::KNOWN
        .iter()
        .find(|known| known.as_str() == word)
        .copied()
        .map(Some)
        .ok_or_else(|| {
            format!(
                "timeframe={word:?} is not a rung this store files: one of {}",
                Timeframe::KNOWN
                    .iter()
                    .map(|known| known.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

// ══ A share's dated closing-auction flag ═════════════════════════════════════

/// Each day's master THAT LOADED, kept once: a receipted master is immutable.
/// A failed load is never kept here (AF-03): only the request that met it
/// remembers it, with its reason, and the next request reads the day again.
static MASTERS: OnceLock<Mutex<MasterMemo>> = OnceLock::new();

/// Loaded masters by their directory and IST day.
type MasterMemo = HashMap<(PathBuf, u32), Arc<pull::cash_session_cache::VerifiedLifecycleMaster>>;

/// The process's loaded masters.
fn masters() -> &'static Mutex<MasterMemo> {
    MASTERS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Where a day's closing-auction flag came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CasBasis {
    /// Not a share, or a day before the closing auction began (2026-08-03):
    /// no flag is needed, and none is read.
    NotRequired,
    /// The day's dated master holds the share's flag.
    DatedMaster,
    /// The share has no ISIN in the universe, so no master row can name it.
    NoIdentity,
    /// The day's master could not be read, or holds no row for the share, in
    /// the reader's own words.
    Unread(String),
}

/// A share's closing-auction flag on one day, and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CasFlag {
    /// The flag the calendar's phases are built with.
    pub(super) eligibility: CasEligibility,
    /// Why it is what it is.
    pub(super) basis: CasBasis,
}

impl CasFlag {
    /// The record's `calendar.cas` (SC-CLOSE-8, SC-CLOSE-9): `eligible` is
    /// `true` (flag 1), `false` (flag 0) or `null` (no flag held, `why` says
    /// which); `null` as a whole where no flag is required.
    pub(super) fn json(&self) -> serde_json::Value {
        let eligible = match self.eligibility {
            CasEligibility::Eligible => Some(true),
            CasEligibility::NotEligible => Some(false),
            CasEligibility::Unknown => None,
        };
        match &self.basis {
            CasBasis::NotRequired => serde_json::Value::Null,
            CasBasis::DatedMaster => serde_json::json!({
                "eligible": eligible, "basis": "dated_master", "why": null,
            }),
            CasBasis::NoIdentity => serde_json::json!({
                "eligible": null, "basis": "unknown",
                "why": "no ISIN for this symbol in the universe, so no master row can name it",
            }),
            CasBasis::Unread(why) => serde_json::json!({
                "eligible": null, "basis": "unknown", "why": why,
            }),
        }
    }
}

/// One request's reads of the dated masters for one instrument: a day is read
/// at most once per request, a loaded master is kept for the process.
pub(super) struct CasFlags {
    /// `(symbol, ISIN)` for a share the universe names; `None` otherwise.
    identity: Option<(String, String)>,
    /// Whether the instrument is a share at all.
    share: bool,
    /// `<store>/session-masters`.
    dir: PathBuf,
    /// This request's refused loads, by IST day, with their reasons.
    refused: HashMap<u32, String>,
}

impl CasFlags {
    /// The flags of `symbol` on segment `share` (true for `CASH`) in the
    /// store at `root`.
    pub(super) fn of(root: &Path, symbol: &str, share: bool) -> Self {
        let identity = share
            .then(|| brutex_core::universe::nse_isin(symbol))
            .flatten()
            .map(|isin| (symbol.to_owned(), isin.as_str().to_owned()));
        Self {
            identity,
            share,
            dir: root.join("session-masters"),
            refused: HashMap::new(),
        }
    }

    /// The flag on `day`, read from that day's master where one is needed.
    pub(super) fn on(&mut self, day: Day) -> CasFlag {
        let unknown = |basis| CasFlag {
            eligibility: CasEligibility::Unknown,
            basis,
        };
        if !self.share || !pull::vendor::cash_auction_eligibility_required(day) {
            return unknown(CasBasis::NotRequired);
        }
        let Some((symbol, isin)) = self.identity.clone() else {
            return unknown(CasBasis::NoIdentity);
        };
        let days = day.days_from_epoch();
        let master = match self.master(day) {
            Ok(master) => master,
            Err(why) => return unknown(CasBasis::Unread(why)),
        };
        match master.cas_eligibility(&symbol, &isin) {
            Ok(eligibility) => CasFlag {
                eligibility,
                basis: CasBasis::DatedMaster,
            },
            Err(why) => {
                let why = format!("{day}: {why}");
                let _kept = self.refused.insert(days, why.clone());
                unknown(CasBasis::Unread(why))
            }
        }
    }

    /// `day`'s master: the process memo's when it loaded before, else read
    /// now; a refusal is kept for this request only.
    fn master(
        &mut self,
        day: Day,
    ) -> Result<Arc<pull::cash_session_cache::VerifiedLifecycleMaster>, String> {
        let days = day.days_from_epoch();
        let key = (self.dir.clone(), days);
        if let Some(kept) = masters()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
        {
            return Ok(Arc::clone(kept));
        }
        if let Some(why) = self.refused.get(&days) {
            return Err(why.clone());
        }
        match pull::cash_session_cache::read_local_lifecycle(&self.dir, day) {
            Ok(loaded) => {
                let loaded = Arc::new(loaded);
                let _was = masters()
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(key, Arc::clone(&loaded));
                Ok(loaded)
            }
            Err(why) => {
                let why = format!("{day}: {why}");
                let _kept = self.refused.insert(days, why.clone());
                Err(why)
            }
        }
    }

    /// The days this request could not read a flag for, and why: at most one
    /// line a day, in day order.
    pub(super) fn refusals(&self) -> Vec<String> {
        let mut out: Vec<(u32, String)> = self
            .refused
            .iter()
            .map(|(day, why)| (*day, why.clone()))
            .collect();
        out.sort();
        out.into_iter().map(|(_, why)| why).collect()
    }
}

// ══ A contract's expiry ═════════════════════════════════════════════════════

/// A contract's last session: the expiry date its segment opens with
/// (`YYYY-MM-DD-…`), or `None` for a spot series. The same reading
/// `/gaps.json` makes (`gaps_route::expiry_of`), so the two routes agree on
/// which days a contract owes.
pub(super) fn expiry_of(contract: Option<Contract>) -> Option<Day> {
    let contract = contract?;
    let text = contract.as_str().get(..10)?;
    let mut parts = text.split('-');
    let year = parts.next()?.parse::<u16>().ok()?;
    let month = parts.next()?.parse::<u8>().ok()?;
    let day = parts.next()?.parse::<u8>().ok()?;
    Day::new(year, month, day).ok()
}

// ══ The month before ════════════════════════════════════════════════════════

/// How many months back [`held_month_before`] looks: twelve years, the whole
/// span of a Zerodha `1day` series in this store (2015-01..2026-09) and more.
pub(super) const MONTHS_BACK: u32 = 144;

/// The month `steps` before `month`, or `None` before year 0.
fn month_back(month: YearMonth, steps: u32) -> Option<YearMonth> {
    let index = u32::from(month.year())
        .checked_mul(12)?
        .checked_add(u32::from(month.month()))?
        .checked_sub(1)?
        .checked_sub(steps)?;
    YearMonth::new(
        u16::try_from(index / 12).ok()?,
        u8::try_from(index % 12 + 1).ok()?,
    )
    .ok()
}

/// The newest month before `month` for which `held` says yes, at most
/// [`MONTHS_BACK`] months back. `held` is one `metadata` call in the route,
/// so a series whose previous held `1day` bar lies years back — NIFTYGS
/// indices re-scaled after a long hole (DB-031) — is found for that many
/// `stat`s and ONE open, where twelve opens found nothing before.
pub(super) fn held_month_before(
    month: YearMonth,
    mut held: impl FnMut(YearMonth) -> bool,
) -> Option<YearMonth> {
    (1..=MONTHS_BACK)
        .map_while(|steps| month_back(month, steps))
        .find(|probe| held(*probe))
}

#[cfg(test)]
#[path = "dayflags_rules_tests.rs"]
mod tests;
