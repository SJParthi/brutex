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

/// What a band asks beside [`super::Asked`], which `sources.rs` builds by
/// name and so cannot gain a field: the grid's rung, and the one expiry an
/// option family is narrowed to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Band {
    /// The rung the grid shows; `None` counts the feed's finest.
    pub(crate) rung: Option<Timeframe>,
    /// The expiry the band's option family is narrowed to (`expiry=`): the
    /// days after it are `expired` (SC-CLOSE-12). A contract's own expiry
    /// is its key's.
    pub(crate) expiry: Option<Day>,
}

/// The band `timeframe=` and `expiry=` ask for `inst`.
///
/// # Errors
///
/// A rung this store does not file, an expiry that is not a day, an expiry
/// on a spot instrument, or an expiry other than the one the contract's own
/// key names, each as one sentence for a 400.
pub(super) fn band_of(
    timeframe: &str,
    expiry: &str,
    inst: &super::Instrument,
) -> Result<Band, String> {
    let rung = asked_rung(timeframe)?;
    if expiry.is_empty() {
        return Ok(Band { rung, expiry: None });
    }
    let day = super::super::import_journal::parse_day(expiry)
        .ok_or_else(|| format!("expiry={expiry:?} is not a day as YYYY-MM-DD"))?;
    if inst.segment != brutex_core::instrument::Segment::Fno {
        return Err(format!(
            "expiry={expiry} names an expiry, and {} has none",
            inst.key()
        ));
    }
    if let Some(own) = expiry_of(inst.contract)
        && own != day
    {
        return Err(format!(
            "expiry={expiry} is not the expiry {} names ({own})",
            inst.key()
        ));
    }
    Ok(Band {
        rung,
        expiry: Some(day),
    })
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

// ══ The closing auction's sidecar ═══════════════════════════════════════════

/// Whether a GDFL share month keeps its closing-auction prints apart, in the
/// `.auc` sidecar beside the one-second month (D-3320): a claim the band makes
/// only where the file is there (SC-CLOSE-9). One `stat` per month a request
/// touches, kept for the request.
pub(super) struct AuctionSidecars {
    /// The share's store key, for a GDFL share; `None` otherwise.
    key: Option<brutex_core::instrument::InstrumentKey>,
    /// `<store>`.
    root: PathBuf,
    /// Each month asked, by `YYYY * 12 + MM`, and whether its sidecar is there.
    seen: HashMap<u32, Option<bool>>,
}

impl AuctionSidecars {
    /// The sidecars of `symbol` on `feed`; none are looked for unless the
    /// feed is GDFL and the segment `CASH` (`share`).
    pub(super) fn of(root: &Path, feed: Vendor, symbol: &str, share: bool) -> Self {
        let key = (feed == Vendor::Gdfl && share)
            .then(|| brutex_core::symbol::Symbol::new(symbol).ok())
            .flatten()
            .map(|underlying| brutex_core::instrument::InstrumentKey {
                exchange: brutex_core::instrument::Exchange::Nse,
                segment: brutex_core::instrument::Segment::Cash,
                underlying,
                kind: brutex_core::instrument::Kind::Equity,
            });
        Self {
            key,
            root: root.to_path_buf(),
            seen: HashMap::new(),
        }
    }

    /// Whether `day`'s month holds a sidecar: `None` where none could exist
    /// (not a GDFL share, or a day before the closing auction began) or the
    /// path cannot be built.
    pub(super) fn on(&mut self, day: Day) -> Option<bool> {
        let key = self.key.as_ref()?;
        if !pull::vendor::cash_auction_eligibility_required(day) {
            return None;
        }
        let month = u32::from(day.year())
            .saturating_mul(12)
            .saturating_add(u32::from(day.month()));
        if let Some(held) = self.seen.get(&month) {
            return *held;
        }
        let held = pull::gdfl_cm::auction::path(&self.root, key, day)
            .ok()
            .map(|path| path.is_file());
        let _was = self.seen.insert(month, held);
        held
    }
}

// ══ The bar definition that built a day ═════════════════════════════════════

/// What one bar definition changed, in a sentence, from the import's own
/// definition list (`pull::gdfl_import::BAR_DEFINITION`'s documentation).
pub(super) const fn definition_means(definition: u64) -> &'static str {
    match definition {
        1 => "placement over traded rows only (D-2802 as first written)",
        2 => {
            "placement over every row; an option second's open interest is its last traded row's (D-2812)"
        }
        3 => {
            "an option second's open interest is the last row placed in it, traded or not (D-3342)"
        }
        4 => {
            "from 2026-08-03 an eligible share's closing-auction band goes to the .auc sidecar, not the fold; every other day holds the bars of 3 (D-4878 item 10)"
        }
        _ => "a definition this build does not name",
    }
}

/// Where a day's definition was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DefinitionFrom {
    /// The import journal's deciding line (`definition=`).
    Journal,
    /// The GDFL day ledger's line (`definition=`), where the journal names
    /// none.
    DayLedger,
}

/// THE BAR DEFINITION THAT BUILT A DAY (R3-13 goal 3): its number, what it
/// means, where it was read, and whether this build would build the same bars
/// (`pull::gdfl_import::definition_is_current`, the importer's one rule). A
/// definition-2 option day is labelled until it is restated. `null` where no
/// source names one.
pub(super) fn definition_json(
    kind: pull::gdfl_import::ImportKind,
    day: Day,
    read: Option<(u64, DefinitionFrom)>,
) -> serde_json::Value {
    let Some((definition, from)) = read else {
        return serde_json::Value::Null;
    };
    serde_json::json!({
        "value": definition,
        "means": definition_means(definition),
        "from": match from {
            DefinitionFrom::Journal => "import_journal",
            DefinitionFrom::DayLedger => "day_ledger",
        },
        "current": pull::gdfl_import::definition_is_current(kind, day, definition),
        "this_build": pull::gdfl_import::BAR_DEFINITION,
    })
}

/// The phrase of a day's definition: always for an option day (the band names
/// which definition built it), and for any other day only where the
/// definition is not current.
pub(super) fn definition_says(
    kind: pull::gdfl_import::ImportKind,
    definition: &serde_json::Value,
) -> Option<String> {
    let value = definition.get("value")?.as_u64()?;
    let current = definition.get("current")?.as_bool()?;
    let this = pull::gdfl_import::BAR_DEFINITION;
    if current && kind != pull::gdfl_import::ImportKind::Options {
        return None;
    }
    let tail = if !current {
        format!(
            "; this build builds definition {this}, so the day is labelled until it is restated"
        )
    } else if value == u64::from(this) {
        String::new()
    } else {
        format!("; it holds the bars definition {this} builds")
    };
    Some(format!(
        "bar definition {value} built this day: {}{tail}",
        definition_means(value)
    ))
}

// ══ A frozen index, and a value outside the day's range ═════════════════════

/// The shortest unchanged run of an index in session that is flagged: three
/// minutes (DB-B20's detector). Chosen, not sourced: an index moves every few
/// seconds in session, and 2022-03-07's dissemination stood still for runs of
/// 5 to 25 minutes while options traded.
pub(super) const FROZEN_MIN_S: u64 = 180;

/// The most one-minute bars one day's scan reads: a whole session is 375, so
/// a day holding more is not a one-minute index's day and is not scanned.
pub(super) const MAX_SCAN_BARS: u64 = 1_440;

/// What one scan of an index day's one-minute bars found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct DayScan {
    /// Bars read.
    pub(super) bars: u64,
    /// The highest high and the lowest low of the day's bars, paisa, with the
    /// stamp (epoch microseconds) of the bar that printed each.
    pub(super) high: Option<(i64, i64)>,
    pub(super) low: Option<(i64, i64)>,
    /// Runs of [`FROZEN_MIN_S`] or more with no change, outside any recorded
    /// halt.
    pub(super) frozen_runs: u32,
    /// The longest such run: its first bar's stamp and its length in seconds.
    pub(super) longest: Option<(i64, u64)>,
}

/// Each day's scan by month file, the file's inode and the day's first row,
/// with the day's end row it was read up to. The store appends in time order,
/// so the rows `[lo, hi)` of a file never change: a day whose end row is the
/// one kept is one probe, a day that grew since is read again and REPLACES its
/// entry, and a new generation is a new inode, read afresh. So the memo holds
/// at most one entry per (month file, day) the process has shown, however
/// often a month being written grows.
static SCANS: OnceLock<Mutex<ScanMemo>> = OnceLock::new();

/// Scans by the month file's device and inode and the day's first row:
/// `(end row, scan)`. No path is hashed or allocated per day.
type ScanMemo = HashMap<(u64, u64, u64), (u64, DayScan)>;

/// Folds one day's one-minute bars, oldest first: the extremes, and the runs
/// of consecutive minutes (no minute missing between them) each printing one
/// unchanged value, `O = H = L = C`, the same as the minute before. A run is
/// measured as its minutes times 60 s. `in_halt` says whether a bar's stamp is
/// inside the calendar's recorded halt, where an unchanged value is the halt's
/// and not a frozen feed.
pub(super) fn fold_minutes(
    bars: impl Iterator<Item = store::format::Bar>,
    in_halt: impl Fn(i64) -> bool,
) -> DayScan {
    const MINUTE: i64 = 60_000_000;
    let mut scan = DayScan::default();
    // The run in progress: its first stamp, its last stamp, its value.
    let mut run: Option<(i64, i64, i64)> = None;
    let close_run = |run: Option<(i64, i64, i64)>, scan: &mut DayScan| {
        let Some((first, last, _)) = run else { return };
        let seconds = u64::try_from((last - first) / 1_000_000 + 60).unwrap_or(0);
        if seconds >= FROZEN_MIN_S {
            scan.frozen_runs = scan.frozen_runs.saturating_add(1);
            if scan.longest.is_none_or(|(_, longest)| seconds > longest) {
                scan.longest = Some((first, seconds));
            }
        }
    };
    for bar in bars {
        scan.bars = scan.bars.saturating_add(1);
        if scan.high.is_none_or(|(high, _)| bar.high > high) {
            scan.high = Some((bar.high, bar.ts_micros));
        }
        if scan.low.is_none_or(|(low, _)| bar.low < low) {
            scan.low = Some((bar.low, bar.ts_micros));
        }
        let flat = bar.open == bar.high && bar.high == bar.low && bar.low == bar.close;
        let halted = in_halt(bar.ts_micros);
        run = match run {
            Some((first, last, value))
                if flat && !halted && value == bar.close && bar.ts_micros - last == MINUTE =>
            {
                Some((first, bar.ts_micros, value))
            }
            other => {
                close_run(other, &mut scan);
                (flat && !halted).then_some((bar.ts_micros, bar.ts_micros, bar.close))
            }
        };
    }
    close_run(run, &mut scan);
    scan
}

/// A month file's device and inode, the scan memo's key: one `stat` per
/// month a page opens, never per day. `None` where the host will not say.
pub(super) fn file_identity(file: &store::file::BarFile) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt as _;
    std::fs::metadata(file.path())
        .ok()
        .map(|meta| (meta.dev(), meta.ino()))
}

/// One day's scan of `file`'s rows `[lo, hi)`: at most [`MAX_SCAN_BARS`]
/// record reads, `None` past that. Read once per (file, day, end row) and kept
/// ([`SCANS`]) under the file's `identity` ([`file_identity`], one `stat` a
/// month); a day that grew is read again, and with no identity nothing is kept.
///
/// # Errors
///
/// A record the store would not read, in its own words.
pub(super) fn scan_day(
    file: &store::file::BarFile,
    identity: Option<(u64, u64)>,
    (lo, hi): (u64, u64),
    in_halt: impl Fn(i64) -> bool,
) -> Result<Option<DayScan>, String> {
    if hi.saturating_sub(lo) > MAX_SCAN_BARS {
        return Ok(None);
    }
    let key = identity.map(|(dev, ino)| (dev, ino, lo));
    let memo = SCANS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(key) = &key
        && let Some((end, kept)) = memo
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .copied()
        && end == hi
    {
        return Ok(Some(kept));
    }
    let bars = (lo..hi)
        .map(|at| file.read_record(at).map_err(|why| why.to_string()))
        .collect::<Result<Vec<_>, String>>()?;
    let scan = fold_minutes(bars.into_iter(), in_halt);
    if let Some(key) = key {
        let _was = memo
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, (hi, scan));
    }
    Ok(Some(scan))
}

/// What an index day's rules read: the feed, the day's scan of its minutes
/// (Zerodha), the day bar's high and low, and the day ledger's longest
/// unchanged run (GDFL: `flat_max_s`, `flat_max_at`, its line).
pub(super) struct IndexDay {
    /// The feed whose bars these are.
    pub(super) feed: Vendor,
    /// IST midnight of the day, epoch microseconds.
    pub(super) midnight: i64,
    /// The minutes' scan, where one was read.
    pub(super) scan: Option<DayScan>,
    /// The day bar's `(high, low)`, paisa, where one is held.
    pub(super) day_bar: Option<(i64, i64)>,
    /// The day ledger's `(flat_max_s, flat_max_at, line)`, where it has them.
    pub(super) ledger: Option<(u64, Option<u32>, u64)>,
    /// The recorded halt's `[from, resume)` seconds of the day, where one is.
    pub(super) halt: Option<(u32, u32)>,
}

/// `HH:MM` of an epoch-microsecond stamp on the day opening at `midnight`.
fn clock_of(midnight: i64, ts: i64) -> String {
    let minutes = ts.saturating_sub(midnight).div_euclid(60_000_000);
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

impl IndexDay {
    /// The record's `index` (DB-B20): `frozen` and `outside_day_range`, each
    /// `null` where nothing measured it, and the phrases they say.
    ///
    /// A FROZEN RUN is [`FROZEN_MIN_S`] or more of an unchanged value outside
    /// any recorded halt: from Zerodha's minutes, `O = H = L = C` and equal
    /// minute to minute; from GDFL's day ledger, the import's measure of the
    /// vendor's rows. AN INTRADAY VALUE OUTSIDE THE DAY'S RANGE is a minute's
    /// high above the day bar's high, or its low below the day bar's low:
    /// 2022-03-07 10:07 is 229.85 points above. Both are flags; the bars are
    /// served unchanged.
    pub(super) fn json(&self) -> (serde_json::Value, Vec<String>) {
        let mut says = Vec::new();
        let rung = finest_of(self.feed).as_str();
        let frozen = match (self.scan, self.ledger) {
            (Some(scan), _) => {
                let longest = scan.longest;
                if let Some((from, seconds)) = longest {
                    let to = from.saturating_add(
                        i64::try_from(seconds).unwrap_or(0) * 1_000_000 - 60_000_000,
                    );
                    says.push(format!(
                        "index value frozen in session: {} run(s) of {} minutes or more with no \
                         change, the longest {} minute(s) from {} to {} ({rung}), outside any \
                         recorded halt",
                        scan.frozen_runs,
                        FROZEN_MIN_S / 60,
                        seconds / 60,
                        clock_of(self.midnight, from),
                        clock_of(self.midnight, to),
                    ));
                }
                serde_json::json!({
                    "basis": "minutes_scanned",
                    "rung": rung,
                    "flagged": scan.frozen_runs > 0,
                    "runs": scan.frozen_runs,
                    "longest_s": longest.map(|(_, s)| s),
                    "longest_from": longest.map(|(from, _)| clock_of(self.midnight, from)),
                    "line": null,
                })
            }
            (None, Some((seconds, at, line))) => {
                let in_halt = matches!((at, self.halt), (Some(at), Some((from, resume))) if at >= from && at < resume);
                let flagged = seconds >= FROZEN_MIN_S && !in_halt;
                if flagged {
                    says.push(format!(
                        "index value frozen in session: unchanged for {seconds} s from {} (the \
                         import's measure of the vendor's rows), outside any recorded halt",
                        at.map_or_else(
                            || "-".to_owned(),
                            |at| format!("{:02}:{:02}:{:02}", at / 3_600, at % 3_600 / 60, at % 60)
                        )
                    ));
                }
                serde_json::json!({
                    "basis": "day_ledger",
                    "rung": rung,
                    "flagged": flagged,
                    "runs": null,
                    "longest_s": seconds,
                    "longest_from": at.map(|at| format!("{:02}:{:02}:{:02}", at / 3_600, at % 3_600 / 60, at % 60)),
                    "line": line,
                })
            }
            (None, None) => serde_json::Value::Null,
        };
        let outside = match (self.scan, self.day_bar) {
            (Some(scan), Some((day_high, day_low))) => {
                let above = scan.high.filter(|(high, _)| *high > day_high);
                let below = scan.low.filter(|(low, _)| *low < day_low);
                if let Some((high, at)) = above {
                    says.push(format!(
                        "intraday high {} points above the day bar's high, at {} ({rung})",
                        points(high - day_high),
                        clock_of(self.midnight, at)
                    ));
                }
                if let Some((low, at)) = below {
                    says.push(format!(
                        "intraday low {} points below the day bar's low, at {} ({rung})",
                        points(day_low - low),
                        clock_of(self.midnight, at)
                    ));
                }
                serde_json::json!({
                    "rung": rung,
                    "flagged": above.is_some() || below.is_some(),
                    "day_high": day_high,
                    "day_low": day_low,
                    "above_by": above.map(|(high, _)| high - day_high),
                    "above_at": above.map(|(_, at)| clock_of(self.midnight, at)),
                    "below_by": below.map(|(low, _)| day_low - low),
                    "below_at": below.map(|(_, at)| clock_of(self.midnight, at)),
                })
            }
            _ => serde_json::Value::Null,
        };
        (
            serde_json::json!({ "frozen": frozen, "outside_day_range": outside }),
            says,
        )
    }
}

/// Paisa as index points, two decimals: `22985` is `229.85`.
pub(super) fn points(paisa: i64) -> String {
    let sign = if paisa < 0 { "-" } else { "" };
    let whole = paisa.unsigned_abs();
    format!("{sign}{}.{:02}", whole / 100, whole % 100)
}

// ══ The market's index ══════════════════════════════════════════════════════

/// The index a share's day is judged sampled by (CMP-06): NIFTY, the index
/// the calendar of record already measures market-wide short days on
/// (`calendar.short_day.observed_on`). A share's own one-second bars are
/// trades and cannot show sampling.
pub(super) const MARKET_INDEX: &str = "NIFTY";

// ══ A share's other names ═══════════════════════════════════════════════════

/// Every other symbol a share has been filed under, from the sourced symbol
/// changes (`brutex_core::alias::LINEAGE`, D-3333): newer names first, then
/// older ones. A peer feed files a renamed share's months under ITS name for
/// them, so a zero-volume day is checked against the peer under each (R3-13
/// goal 2, DB-031). At most the table's rows, one probe each.
pub(super) fn other_names(symbol: &str) -> Vec<&'static str> {
    let lineage = &brutex_core::alias::LINEAGE;
    lineage
        .forward(symbol)
        .map(|hop| hop.new)
        .chain(lineage.backward(symbol).map(|hop| hop.old))
        .collect()
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
