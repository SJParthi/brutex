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
//! one hash probe a day once the share's flag is kept (otherwise one read of
//! the day's master, shared by every request asking at once, on at most
//! [`LOADS_AT_ONCE`] threads at a time, and only from 2026-08-03 on; a cold
//! master read is NOT O(1) and is bounded and measured in `docs/06-limits.md`
//! L-R3-13-4); [`expiry_of`] parses
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

/// A loaded, receipt-checked NSE master.
type Master = pull::cash_session_cache::VerifiedLifecycleMaster;

/// One share's flag on one day as its day's master answered it: the flag, or
/// the master's own refusal to name the share (no row, another ISIN). A
/// receipted master is immutable, so a later read could only repeat either.
type Answer = Result<CasEligibility, String>;

/// A flag's key: the masters directory, the IST day, the share's symbol.
type FlagKey = (PathBuf, u32, String);

/// THE FLAGS ARE KEPT, NEVER THE MASTERS (verify-R3-13-r1, blocking 2;
/// D-7135 item 20). A parsed master held for the process cost 18-25 MB of
/// resident memory per auction-era day, never freed (+744 MB for 42 days),
/// because `pull::cash_auction::DailyEligibility::parse` reserves its map for
/// every CSV line (37,237 on 2026-09-01) to keep the 4,428 EQ rows. A page
/// needs one share's flag a day, so that answer is all that is kept: its key
/// (the directory's path, the day, the symbol) and the answer, about 200
/// bytes by the sizes of their parts (an estimate; the process's memory is
/// measured in L-R3-13-4), in two generations of at most [`FLAG_CAP`] / 2
/// entries each.
/// A FAILED LOAD IS NEVER KEPT HERE (AF-03): only the request that met it
/// remembers it, with its reason, and the next request reads the day again.
static FLAGS: OnceLock<Mutex<FlagGenerations>> = OnceLock::new();

/// The most flags [`FLAGS`] holds across its two generations: 213 F&O shares
/// for 153 auction-era sessions in one generation. A flag turned out is one
/// master read again on the next cold page that needs it, never a wrong flag.
pub(super) const FLAG_CAP: usize = 65_536;

/// The two generations behind [`FLAGS`]: a put and a hit go to `young`; when
/// `young` is full and the key is new, `old` is dropped whole and `young`
/// becomes `old`, so at most the older half is dropped at once and a flag in
/// use survives the turn.
#[derive(Default)]
struct FlagGenerations {
    young: HashMap<FlagKey, Answer>,
    old: HashMap<FlagKey, Answer>,
}

impl FlagGenerations {
    /// The flag kept for `key`; a hit in `old` moves it to `young`.
    fn get(&mut self, key: &FlagKey) -> Option<Answer> {
        if let Some(answer) = self.young.get(key) {
            return Some(answer.clone());
        }
        let (key, answer) = self.old.remove_entry(key)?;
        self.put(key, answer.clone());
        Some(answer)
    }

    /// Keeps `answer` for `key`, turning the generations when `young` is full.
    fn put(&mut self, key: FlagKey, answer: Answer) {
        let _was = self.old.remove(&key);
        if !self.young.contains_key(&key) && self.young.len() >= FLAG_CAP / 2 {
            self.old = std::mem::take(&mut self.young);
        }
        let _was = self.young.insert(key, answer);
    }

    /// Flags held across both generations.
    #[cfg(test)]
    fn len(&self) -> usize {
        self.young.len() + self.old.len()
    }
}

/// The process's kept flags.
fn flags() -> std::sync::MutexGuard<'static, FlagGenerations> {
    FLAGS
        .get_or_init(|| Mutex::new(FlagGenerations::default()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// One load of one day's master, shared by every reader that asks while it
/// runs: the master, or the load's refusal.
type Flight = Arc<OnceLock<Result<Arc<Master>, String>>>;

/// THE LOADS IN FLIGHT, by masters directory and IST day (SINGLE FLIGHT,
/// verify-R3-13-r1 blocking 2): ten cold pages asking one day at once parse
/// its master once. A flight is dropped as soon as its first reader is done,
/// and the master with its last reader, so no master outlives the requests
/// that read it.
static FLIGHTS: OnceLock<Mutex<HashMap<(PathBuf, u32), Flight>>> = OnceLock::new();

/// The process's loads in flight.
fn flights() -> std::sync::MutexGuard<'static, HashMap<(PathBuf, u32), Flight>> {
    FLIGHTS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

/// The most masters the process parses at once, over every request: each
/// parse holds its expanded CSV (11 MB on 2026-09-01) and the reserved map
/// beside it until it is done, so this bounds the memory cold pages can take
/// together, whatever their number. A page's own cold days are read on up to
/// this many threads ([`CasFlags::on_sessions`]).
pub(super) const LOADS_AT_ONCE: usize = 4;

/// The parses running now, against [`LOADS_AT_ONCE`].
static LOADING: (Mutex<usize>, std::sync::Condvar) = (Mutex::new(0), std::sync::Condvar::new());

/// Reads `day`'s master from `dir` under one of the [`LOADS_AT_ONCE`] slots,
/// waiting for one where all are taken.
fn load(dir: &Path, day: Day) -> Result<Arc<Master>, String> {
    let _slot = Slot::take();
    #[cfg(test)]
    read_counted(dir, day);
    pull::cash_session_cache::read_local_lifecycle(dir, day).map(Arc::new)
}

/// One of the [`LOADS_AT_ONCE`] slots, given back when dropped, a panicking
/// read included, so a slot is never lost.
struct Slot;

impl Slot {
    /// Waits for a free slot and takes it.
    fn take() -> Self {
        let (count, freed) = &LOADING;
        let mut running = count.lock().unwrap_or_else(PoisonError::into_inner);
        while *running >= LOADS_AT_ONCE {
            running = freed.wait(running).unwrap_or_else(PoisonError::into_inner);
        }
        *running += 1;
        Self
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let (count, freed) = &LOADING;
        let mut running = count.lock().unwrap_or_else(PoisonError::into_inner);
        *running = running.saturating_sub(1);
        drop(running);
        freed.notify_one();
    }
}

/// `symbol`'s flag on `day` from the masters in `dir`: the kept answer where
/// one is, else the day's master read once however many ask at once. `Err`
/// is the load's refusal, never kept; `Ok` is the master's answer, kept.
fn flag_of(dir: &Path, day: Day, (symbol, isin): (&str, &str)) -> Result<Answer, String> {
    let key: FlagKey = (dir.to_path_buf(), day.days_from_epoch(), symbol.to_owned());
    if let Some(kept) = flags().get(&key) {
        return Ok(kept);
    }
    let at = (key.0.clone(), key.1);
    let flight = {
        let mut flying = flights();
        // LOOKED FOR AGAIN UNDER THE FLIGHTS' LOCK: a flight's reader keeps
        // its flag before it ends the flight, so a reader that finds no flight
        // here finds the flag, and one share's day is never read twice at once.
        if let Some(kept) = flags().get(&key) {
            return Ok(kept);
        }
        Arc::clone(flying.entry(at.clone()).or_default())
    };
    let loaded = flight
        .get_or_init(|| {
            let master = load(dir, day)?;
            keep_underlyings(dir, day, &master);
            Ok(master)
        })
        .clone();
    let answer = loaded.map(|master| master.cas_eligibility(symbol, isin));
    if let Ok(kept) = &answer {
        flags().put(key, kept.clone());
    }
    let mut flying = flights();
    if flying
        .get(&at)
        .is_some_and(|held| Arc::ptr_eq(held, &flight))
    {
        let _ended = flying.remove(&at);
    }
    answer
}

/// FROM A MASTER JUST READ, EVERY F&O UNDERLYING'S FLAG ON ITS DAY IS KEPT
/// (the repair's measurement: with only the asking share's flag kept, every
/// further share's first auction-era page parsed the same 42 masters again,
/// about 1 s each). The 213 shares of `brutex_core::universe::FNO_UNDERLYINGS`
/// (CLAUDE.md §1's equity surface) are each the master's own answer for that
/// symbol and its universe ISIN, the lookup [`CasFlags::on`] makes: 213 hash
/// probes against a parse of the whole file, and about 213 x 200 bytes a day
/// kept. A share outside the list keeps only its own flag, as it asks.
fn keep_underlyings(dir: &Path, day: Day, master: &Master) {
    let days = day.days_from_epoch();
    let answers: Vec<(FlagKey, Answer)> = brutex_core::universe::FNO_UNDERLYINGS
        .iter()
        .filter_map(|symbol| {
            let isin = brutex_core::universe::nse_isin(symbol)?;
            Some((
                (dir.to_path_buf(), days, (*symbol).to_owned()),
                master.cas_eligibility(symbol, isin.as_str()),
            ))
        })
        .collect();
    let mut kept = flags();
    for (key, answer) in answers {
        kept.put(key, answer);
    }
}

/// Masters read so far, by directory and IST day: the proof that a day's
/// master is read once however many ask at once.
#[cfg(test)]
static READS: OnceLock<Mutex<HashMap<(PathBuf, u32), usize>>> = OnceLock::new();

/// Counts one read of `day`'s master in `dir`.
#[cfg(test)]
fn read_counted(dir: &Path, day: Day) {
    let reads = READS.get_or_init(|| Mutex::new(HashMap::new()));
    *reads
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry((dir.to_path_buf(), day.days_from_epoch()))
        .or_default() += 1;
}

/// How many times `day`'s master in `dir` has been read.
#[cfg(test)]
pub(super) fn reads_of(dir: &Path, day: Day) -> usize {
    READS
        .get()
        .and_then(|reads| {
            reads
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .get(&(dir.to_path_buf(), day.days_from_epoch()))
                .copied()
        })
        .unwrap_or(0)
}

/// Loads of `dir`'s masters still in flight: none once the requests that
/// read them are done.
#[cfg(test)]
pub(super) fn flights_of(dir: &Path) -> usize {
    flights().keys().filter(|(of, _)| of == dir).count()
}

/// Whether `symbol`'s flag on `day` from `dir` is kept, and how many flags
/// are kept in all.
#[cfg(test)]
pub(super) fn kept_flag(dir: &Path, day: Day, symbol: &str) -> (bool, usize) {
    let mut kept = flags();
    let key = (dir.to_path_buf(), day.days_from_epoch(), symbol.to_owned());
    (kept.get(&key).is_some(), kept.len())
}

/// Drops every flag kept from `dir`, so a timing run reads cold; flags from
/// other directories, other tests' stores, are left as they are.
#[cfg(test)]
pub(super) fn forget_flags(dir: &Path) {
    let mut kept = flags();
    kept.young.retain(|(of, _, _), _| of != dir);
    kept.old.retain(|(of, _, _), _| of != dir);
}

/// Where a day's closing-auction flag came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum CasBasis {
    /// Not a share, or a day before the closing auction began (2026-08-03):
    /// no flag is needed, and none is read.
    NotRequired,
    /// A session after today (IST): no master is dated it yet, and nothing is
    /// read or claimed.
    NotYet,
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
            CasBasis::NotYet => serde_json::json!({
                "eligible": null, "basis": "not_yet",
                "why": "a session after today: no master is dated it yet",
            }),
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
/// at most once per request, and its answer, never its master, is kept for the
/// process ([`FLAGS`]).
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

    /// The flag on `day` as a page reads it: nothing is read for a day that is
    /// not an `open` session (a closed day has no closing auction), and a
    /// session after today (`future`) has no master dated it yet — said so,
    /// never read and never a refusal (R3-13 review). Otherwise [`Self::on`].
    pub(super) fn on_session(&mut self, day: Day, open: bool, future: bool) -> CasFlag {
        let required = self.share && pull::vendor::cash_auction_eligibility_required(day);
        let basis = match (required && open, future) {
            (false, _) => CasBasis::NotRequired,
            (true, true) => CasBasis::NotYet,
            (true, false) => return self.on(day),
        };
        CasFlag {
            eligibility: CasEligibility::Unknown,
            basis,
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
        let Some((symbol, isin)) = self.identity.as_ref() else {
            return unknown(CasBasis::NoIdentity);
        };
        let days = day.days_from_epoch();
        if let Some(why) = self.refused.get(&days) {
            return unknown(CasBasis::Unread(why.clone()));
        }
        match flag_of(&self.dir, day, (symbol.as_str(), isin.as_str())) {
            Ok(Ok(eligibility)) => CasFlag {
                eligibility,
                basis: CasBasis::DatedMaster,
            },
            Ok(Err(why)) | Err(why) => {
                let why = format!("{day}: {why}");
                let _kept = self.refused.insert(days, why.clone());
                unknown(CasBasis::Unread(why))
            }
        }
    }

    /// [`Self::on_session`] for each of a page's `(day, open, future)`, in
    /// order, ITS COLD DAYS READ FIRST ON UP TO [`LOADS_AT_ONCE`] THREADS
    /// (verify-R3-13-r1 blocking 2): an auction-era page of 42 cold days read
    /// one master after another took 2.6-3.15 s. A day whose flag is kept
    /// spawns nothing, and a page with one cold day reads it on its own
    /// thread. A load a thread could not make is this request's refusal, so
    /// no day is read twice in one request.
    pub(super) fn on_sessions(&mut self, asks: &[(Day, bool, bool)]) -> Vec<CasFlag> {
        if let Some((symbol, isin)) = self.identity.as_ref() {
            let cold: Vec<Day> = asks
                .iter()
                .filter(|(day, open, future)| {
                    self.share
                        && *open
                        && !*future
                        && pull::vendor::cash_auction_eligibility_required(*day)
                        && !self.refused.contains_key(&day.days_from_epoch())
                        && flags()
                            .get(&(self.dir.clone(), day.days_from_epoch(), symbol.clone()))
                            .is_none()
                })
                .map(|(day, _, _)| *day)
                .collect();
            if cold.len() > 1 {
                let next = std::sync::atomic::AtomicUsize::new(0);
                let refused = Mutex::new(Vec::new());
                let dir = self.dir.as_path();
                // THE SCOPE JOINS EVERY THREAD and re-raises a panic in one.
                std::thread::scope(|scope| {
                    for _ in 0..cold.len().min(LOADS_AT_ONCE) {
                        let _thread = scope.spawn(|| {
                            while let Some(day) =
                                cold.get(next.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                            {
                                if let Err(why) =
                                    flag_of(dir, *day, (symbol.as_str(), isin.as_str()))
                                {
                                    refused
                                        .lock()
                                        .unwrap_or_else(PoisonError::into_inner)
                                        .push((day.days_from_epoch(), format!("{day}: {why}")));
                                }
                            }
                        });
                    }
                });
                self.refused
                    .extend(refused.into_inner().unwrap_or_else(PoisonError::into_inner));
            }
        }
        asks.iter()
            .map(|(day, open, future)| self.on_session(*day, *open, *future))
            .collect()
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

/// What a GDFL share's `.auc` sidecar (D-3320) says of ONE day: a claim the
/// band makes only from the sidecar's own records (SC-CLOSE-9).
///
/// PER DAY, NEVER PER MONTH (the Mac finish of R3-13; `docs/06-limits.md`
/// L-AH03-2): a month gains its sidecar on the first day imported under
/// D-3320's rule, so 2026-09-01..24 of every share whose 2026-09 month gained
/// one from the 25-30 Sep append sit in a month that "holds a sidecar" while
/// their prints were imported as ordinary bars. A month's file is therefore
/// read once and each day answered by whether a record is stamped on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SidecarDay {
    /// The month's sidecar files at least one print stamped on this day.
    Filed,
    /// The month holds a sidecar, and none of its prints is stamped on this
    /// day.
    NoneThatDay,
    /// The month holds no sidecar.
    NoFile,
}

impl SidecarDay {
    /// The record's `auction_sidecar`: `true` only where the day's print is
    /// filed apart.
    pub(super) const fn filed(self) -> bool {
        matches!(self, Self::Filed)
    }
}

/// One month's sidecar as a request read it: `None` for no file, else the
/// IST days its records are stamped on (at most 31 distinct, sorted).
type SidecarMonth = Option<Vec<u32>>;

/// Whether a GDFL share's month keeps each day's closing-auction print
/// apart, read from the `.auc` sidecar beside the one-second month: one
/// read of the file a month a request touches (a small file: one 32-byte
/// record a print), kept for the request.
pub(super) struct AuctionSidecars {
    /// The share's store key, for a GDFL share; `None` otherwise.
    key: Option<brutex_core::instrument::InstrumentKey>,
    /// `<store>`.
    root: PathBuf,
    /// Each month asked, by `YYYY * 12 + MM`: what it holds, or `None` where
    /// it could not be read (named in `refused`).
    seen: HashMap<u32, Option<SidecarMonth>>,
    /// The months whose sidecar could not be read, in the reader's words (one
    /// line a month): never said "holds no sidecar", which is a claim.
    refused: Vec<String>,
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
            refused: Vec::new(),
        }
    }

    /// The months whose sidecar could not be read, and why.
    pub(super) fn refusals(&self) -> &[String] {
        &self.refused
    }

    /// What `day`'s month's sidecar says of `day`: `None` where none could
    /// exist (not a GDFL share, or a day before the closing auction began),
    /// or where it could not be read (the path cannot be built, the file
    /// will not open, lock or parse): that is named in [`Self::refusals`],
    /// never answered as "no sidecar".
    pub(super) fn on(&mut self, day: Day) -> Option<SidecarDay> {
        let key = self.key.as_ref()?;
        if !pull::vendor::cash_auction_eligibility_required(day) {
            return None;
        }
        let month = u32::from(day.year())
            .saturating_mul(12)
            .saturating_add(u32::from(day.month()));
        // A MONTH ALREADY READ IS ONE PROBE, AND NOTHING IS COPIED per day.
        if let Some(held) = self.seen.get(&month) {
            return held.as_ref().map(|held| said_of(held.as_deref(), day));
        }
        let held = match read_month(&self.root, key, day) {
            Ok(held) => Some(held),
            Err(why) => {
                self.refused.push(format!(
                    "gdfl NSE-CASH-{} 1s {:04}-{:02}: the month's auction sidecar could not be \
                     read, so whether a print is kept apart is not said: {why}",
                    key.underlying,
                    day.year(),
                    day.month()
                ));
                None
            }
        };
        let said = held.as_ref().map(|held| said_of(held.as_deref(), day));
        let _was = self.seen.insert(month, held);
        said
    }
}

/// What a month's sidecar, as read (`None`: no file; else the IST days its
/// records are stamped on, sorted), says of `day`: one binary search.
fn said_of(month: Option<&[u32]>, day: Day) -> SidecarDay {
    match month {
        None => SidecarDay::NoFile,
        Some(days) if days.binary_search(&day.days_from_epoch()).is_ok() => SidecarDay::Filed,
        Some(_) => SidecarDay::NoneThatDay,
    }
}

/// The IST days `key`'s sidecar for `day`'s month stamps its records on, or
/// `None` where the month holds none. A failed read is NOT "no file" (CLAUDE.md
/// §4): only `pull::gdfl_cm::auction::read`'s `Ok(None)`, a file not found, is.
fn read_month(
    root: &Path,
    key: &brutex_core::instrument::InstrumentKey,
    day: Day,
) -> Result<SidecarMonth, String> {
    let path = pull::gdfl_cm::auction::path(root, key, day)?;
    let Some(sidecar) = pull::gdfl_cm::auction::read(&path).map_err(|refused| refused.why)? else {
        return Ok(None);
    };
    let mut days: Vec<u32> = sidecar
        .records
        .iter()
        .filter_map(|record| {
            let seconds = record
                .t_micros
                .div_euclid(1_000_000)
                .saturating_add(pull::session::IST_OFFSET_SECS);
            u32::try_from(seconds.div_euclid(86_400)).ok()
        })
        .collect();
    days.sort_unstable();
    days.dedup();
    Ok(Some(days))
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

/// THE CASH CLOSING AUCTION'S WINDOW ON AN INDEX DAY, `[from, to)` seconds
/// since IST midnight (the Mac finish of DB-B20): from 2026-08-03 every share
/// an NSE index is computed from leaves continuous trading at the CAS-eligible
/// close (`pull::vendor::Venue::share_hours_on`, D-3320), while the index's
/// own session runs on to its close (`Venue::hours_on`). An unchanged index
/// value there is the auction's, never a frozen feed's.
///
/// MEASURED, NOT ASSUMED (`docs/00-charter.md`): read through a shadow of the
/// store on 2026-10-20, Zerodha's NIFTY and BANKNIFTY `1min` hold exactly one
/// unchanged run a day from 15:15, of 780 or 840 s, on every one of the 42
/// sessions 2026-08-03..2026-09-30 and on none of 2026-07-20..31; GDFL's day
/// ledger measures NIFTY unchanged from 15:15:00 for 799 to 864 s on each of
/// 2026-09-25..30. Without this window every index session since 2026-08-03
/// read "frozen in session".
///
/// `None` before 2026-08-03, for any venue but the index, and where either
/// session row carries no verified hours: then nothing is set aside, so a run
/// there is judged and flagged, never hidden.
pub(super) fn auction_window(venue: pull::vendor::Venue, day: Day) -> Option<(u32, u32)> {
    use pull::vendor::Venue;
    if venue != Venue::NseIndex || !pull::vendor::cash_auction_eligibility_required(day) {
        return None;
    }
    let from = Venue::NseCash
        .share_hours_on(day, CasEligibility::Eligible)
        .ok()?
        .close_minute();
    let to = venue.hours_on(day).ok()?.close_minute();
    // An empty or inverted window sets nothing aside: [`in_window`] and the
    // overlap of a run with it are then nothing, so no guard is needed here.
    Some((from.saturating_mul(60), to.saturating_mul(60)))
}

/// Whether `since` (seconds since IST midnight) is inside `window`.
pub(super) fn in_window(window: Option<(u32, u32)>, since: i64) -> bool {
    window.is_some_and(|(from, to)| since >= i64::from(from) && since < i64::from(to))
}

/// The run `[start, end)`'s seconds outside every one of `windows`
/// (`[from, to)` each): its longest single stretch, and all its stretches
/// together. A window that is empty or inverted sets nothing aside, and two
/// that meet are counted once. Two windows, so at most three stretches.
///
/// THE LONGEST STRETCH IS WHAT A RUN IS JUDGED BY, as Zerodha's minutes are
/// ([`fold_minutes`]): a bar inside a set-aside window ends the run there, so
/// 100 s before a halt and 100 s after it are two runs of 100 s, never one of
/// 200 s.
fn outside_of((start, end): (u64, u64), windows: [Option<(u32, u32)>; 2]) -> (u64, u64) {
    let mut cuts: Vec<(u64, u64)> = windows
        .iter()
        .flatten()
        .map(|&(from, to)| (u64::from(from), u64::from(to)))
        .filter(|(from, to)| from < to)
        .collect();
    cuts.sort_unstable();
    let (mut cursor, mut longest, mut total) = (start, 0_u64, 0_u64);
    for (from, to) in cuts {
        let stretch = from.min(end).saturating_sub(cursor);
        longest = longest.max(stretch);
        total = total.saturating_add(stretch);
        cursor = cursor.max(to);
    }
    let last = end.saturating_sub(cursor);
    (longest.max(last), total.saturating_add(last))
}

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
    /// Times an unchanged value went on past one or more absent minutes: the
    /// run before them ended there, so each part was measured alone.
    pub(super) broken: u32,
}

/// Each day's scan by month file, the file's inode and the day's first row,
/// with the day's end row it was read up to. The store appends in time order,
/// so the rows `[lo, hi)` of a file never change: a day whose end row is the
/// one kept is one probe, a day that grew since is read again and REPLACES its
/// entry, and a new generation is a new inode, read afresh. So the memo holds
/// at most one entry per (month file, day) the process has shown, however
/// often a month being written grows.
static SCANS: OnceLock<Mutex<ScanMemo>> = OnceLock::new();

/// The process's kept scans.
fn scans() -> &'static Mutex<ScanMemo> {
    SCANS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Scans by the month file's device and inode and the day's first row:
/// `(end row, scan)`. No path is hashed or allocated per day.
type ScanMemo = HashMap<(u64, u64, u64), (u64, DayScan)>;

/// Folds one day's one-minute bars, oldest first: the extremes, and the runs
/// of consecutive minutes (no minute missing between them) each printing one
/// unchanged value, `O = H = L = C`, the same as the minute before. A run is
/// measured as its minutes times 60 s. `still` says whether a bar's stamp is
/// set aside: inside the calendar's recorded halt, or inside the cash closing
/// auction's window an index is computed through ([`auction_window`]), where
/// an unchanged value is the halt's or the auction's and not a frozen feed.
pub(super) fn fold_minutes(
    bars: impl Iterator<Item = store::format::Bar>,
    still: impl Fn(i64) -> bool,
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
        // O = H = L = C IS `high == low` FOR A STORED BAR: the store refuses,
        // on every append, a bar whose open or close lies outside its high and
        // low (`store::file` `survey`, `Bar::ohlc_is_sane`).
        let flat = bar.high == bar.low;
        let halted = still(bar.ts_micros);
        run = match run {
            Some((first, last, value))
                if flat && !halted && value == bar.close && bar.ts_micros - last == MINUTE =>
            {
                Some((first, bar.ts_micros, value))
            }
            other => {
                // A RUN AN ABSENT MINUTE ENDED, where the next minute held
                // prints the same value still, is counted (the check's
                // follow-up: BANKNIFTY 2022-03-07 09:54-09:57), so a run
                // measured as its parts says why.
                if let Some((_, last, value)) = other
                    && flat
                    && !halted
                    && value == bar.close
                    && bar.ts_micros - last > MINUTE
                {
                    scan.broken = scan.broken.saturating_add(1);
                }
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
    still: impl Fn(i64) -> bool,
) -> Result<Option<DayScan>, String> {
    if hi.saturating_sub(lo) > MAX_SCAN_BARS {
        return Ok(None);
    }
    let key = identity.map(|(dev, ino)| (dev, ino, lo));
    let memo = scans();
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
    let scan = fold_minutes(bars.into_iter(), still);
    if let Some(key) = key {
        let _was = memo
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, (hi, scan));
    }
    Ok(Some(scan))
}

/// The days of `spans` a month's scan has still to read: those that hold bars
/// and that the memo does not hold at their end row, under the month file's
/// `(dev, ino)`. One lock of the memo a month.
pub(super) fn cold_days((dev, ino): (u64, u64), spans: &[(Day, u64, u64)]) -> Vec<(Day, u64, u64)> {
    let memo = scans().lock().unwrap_or_else(PoisonError::into_inner);
    spans
        .iter()
        .filter(|(_, lo, hi)| {
            hi > lo && memo.get(&(dev, ino, *lo)).is_none_or(|(end, _)| end != hi)
        })
        .copied()
        .collect()
}

/// Scans `days` of `file` in order and keeps each ([`scan_day`]); a day that
/// will not read is left for its own [`scan_day`] to read again and name.
/// Returns how many days were read.
pub(super) fn scan_days<Still: Fn(i64) -> bool>(
    file: &store::file::BarFile,
    identity: (u64, u64),
    days: &[(Day, u64, u64)],
    still: impl Fn(Day) -> Still,
) -> usize {
    for &(day, lo, hi) in days {
        let _scanned = scan_day(file, Some(identity), (lo, hi), still(day));
    }
    days.len()
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
    /// The cash closing auction's `[from, to)` seconds of the day, where the
    /// index is computed through one ([`auction_window`]).
    pub(super) auction: Option<(u32, u32)>,
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
                         recorded halt{}",
                        scan.frozen_runs,
                        FROZEN_MIN_S / 60,
                        seconds / 60,
                        clock_of(self.midnight, from),
                        clock_of(self.midnight, to),
                        self.aside(),
                    ));
                }
                if scan.broken > 0 {
                    says.push(format!(
                        "index value unchanged across absent minutes {} time(s): a minute the \
                         file does not hold ends a run, so each part is measured alone ({rung})",
                        scan.broken
                    ));
                }
                serde_json::json!({
                    "basis": "minutes_scanned",
                    "rung": rung,
                    "flagged": scan.frozen_runs > 0,
                    "runs": scan.frozen_runs,
                    "broken_by_absent": scan.broken,
                    "longest_s": longest.map(|(_, s)| s),
                    "longest_from": longest.map(|(from, _)| clock_of(self.midnight, from)),
                    "auction_window": self.auction_json(),
                    "line": null,
                })
            }
            (None, Some(ledger)) => self.ledger_frozen(ledger, &mut says),
            (None, None) => serde_json::Value::Null,
        };
        let outside = match (self.scan, self.day_bar) {
            (Some(scan), Some((day_high, day_low))) => {
                let above = scan.high.filter(|(high, _)| *high > day_high);
                let below = scan.low.filter(|(low, _)| *low < day_low);
                // A VALUE OUTSIDE THE RANGE INSIDE A RECORDED HALT IS STILL
                // FLAGGED, AND SAYS WHERE IT IS (R3-13, the Mac's re-check):
                // 2020-03-13's Zerodha 10:07 low lies inside the halt the
                // band names on the same day. No cause is claimed for it.
                let where_ = |at: i64| {
                    if self.in_halt(at) {
                        ", inside the recorded halt"
                    } else {
                        ""
                    }
                };
                if let Some((high, at)) = above {
                    says.push(format!(
                        "intraday high {} points above the day bar's high, at {} ({rung}){}",
                        points(high - day_high),
                        clock_of(self.midnight, at),
                        where_(at)
                    ));
                }
                if let Some((low, at)) = below {
                    says.push(format!(
                        "intraday low {} points below the day bar's low, at {} ({rung}){}",
                        points(day_low - low),
                        clock_of(self.midnight, at),
                        where_(at)
                    ));
                }
                serde_json::json!({
                    "rung": rung,
                    "flagged": above.is_some() || below.is_some(),
                    "day_high": day_high,
                    "day_low": day_low,
                    "above_by": above.map(|(high, _)| high - day_high),
                    "above_at": above.map(|(_, at)| clock_of(self.midnight, at)),
                    "above_in_halt": above.map(|(_, at)| self.in_halt(at)),
                    "below_by": below.map(|(low, _)| day_low - low),
                    "below_at": below.map(|(_, at)| clock_of(self.midnight, at)),
                    "below_in_halt": below.map(|(_, at)| self.in_halt(at)),
                })
            }
            _ => serde_json::Value::Null,
        };
        (
            serde_json::json!({ "frozen": frozen, "outside_day_range": outside }),
            says,
        )
    }

    /// The ledger's frozen run (GDFL): the record's `frozen` from the day
    /// ledger's `flat_max_s` and its stamp, and the phrase it says.
    ///
    /// THE LEDGER HOLDS ONE RUN A DAY, THE LONGEST (verify-R3-13-r1, blocking
    /// 1; D-7135 item 19): `flatagg` keeps the day's longest unchanged run and
    /// nothing shorter. So `flagged` is `false` only where that run is under
    /// [`FROZEN_MIN_S`], when no run of the day can be frozen; `true` where its
    /// longest stretch outside the recorded halt and the cash closing
    /// auction's window reaches [`FROZEN_MIN_S`]; and `null` — never "not
    /// frozen" — where the run reaches it but lies in those windows, or has no
    /// stamp to place it against them on a day that has one. A shorter run
    /// elsewhere in that session could be frozen and is not measured, and the
    /// phrase says so (UNVERIFIED). On every auction-era day the ledger has
    /// measured (2026-09-25..30), the run is the auction's, from 15:15:00.
    fn ledger_frozen(
        &self,
        (seconds, at, line): (u64, Option<u32>, u64),
        says: &mut Vec<String>,
    ) -> serde_json::Value {
        let rung = finest_of(self.feed).as_str();
        // ONLY THE RUN'S SECONDS OUTSIDE THE RECORDED HALT AND THE CASH
        // CLOSING AUCTION'S WINDOW COUNT (the R3-13 review; the Mac finish):
        // 2020-03-13's run starts at 09:21:09, a second before the index
        // halt's 09:21:10, and is the halt itself; from 2026-08-03 every
        // session's run from 15:15:00 is the auction's ([`auction_window`]).
        // The two windows are counted once where they meet, and the run is
        // judged by its longest stretch outside them ([`outside_of`]).
        let windowed = self.halt.is_some() || self.auction.is_some();
        let outside = match at {
            Some(at) => {
                let start = u64::from(at);
                let run = (start, start.saturating_add(seconds));
                Some(outside_of(run, [self.halt, self.auction]))
            }
            None if windowed => None,
            None => Some((seconds, seconds)),
        };
        let judged_s = outside.map(|(longest, _)| longest);
        let flagged = match judged_s {
            Some(judged) if judged >= FROZEN_MIN_S => Some(true),
            _ if seconds < FROZEN_MIN_S => Some(false),
            _ => None,
        };
        let clock = |at: u32| format!("{:02}:{:02}:{:02}", at / 3_600, at % 3_600 / 60, at % 60);
        match (flagged, outside) {
            (Some(true), Some((judged, total))) => {
                let stretch = if judged < total {
                    " in one stretch"
                } else {
                    ""
                };
                let part = match (judged < seconds, self.auction.is_some()) {
                    (true, false) => {
                        format!(", {judged} s of it{stretch} outside the recorded halt")
                    }
                    (true, true) => format!(
                        ", {judged} s of it{stretch} outside any recorded halt and the cash \
                         closing auction's window"
                    ),
                    (false, _) => format!(", outside any recorded halt{}", self.aside()),
                };
                says.push(format!(
                    "index value frozen in session: unchanged for {seconds} s from {} (the \
                     import's measure of the vendor's rows){part}",
                    at.map_or_else(|| "-".to_owned(), clock)
                ));
            }
            (None, _) => says.push(self.unjudged(seconds, at.map(clock), judged_s)),
            _ => {}
        }
        serde_json::json!({
            "basis": "day_ledger",
            "rung": rung,
            "flagged": flagged,
            "runs": null,
            "longest_s": seconds,
            "judged_s": judged_s,
            "longest_from": at.map(clock),
            "auction_window": self.auction_json(),
            "line": line,
        })
    }

    /// The phrase of a ledger run that cannot judge the day (`flagged` null):
    /// what the ledger measures, where its one run lies, and that a shorter
    /// run elsewhere is not measured. `from` is the run's stamp, `judged` its
    /// longest stretch outside the windows where it could be placed.
    fn unjudged(&self, seconds: u64, from: Option<String>, judged: Option<u64>) -> String {
        let windows = match (self.halt.is_some(), self.auction.is_some()) {
            (true, false) => "the recorded halt",
            (false, true) => "the cash closing auction's window",
            _ => "the recorded halt and the cash closing auction's window",
        };
        let placed = match (from, judged) {
            (Some(from), Some(0)) => format!("{seconds} s from {from}, all of it inside {windows}"),
            (Some(from), Some(judged)) => format!(
                "{seconds} s from {from}, only {judged} s of it in one stretch outside {windows}"
            ),
            _ => format!("{seconds} s, with no stamp, so it cannot be placed against {windows}"),
        };
        format!(
            "index frozen not judged: the day ledger measures only the day's longest unchanged \
             run, {placed}; a shorter run of {FROZEN_MIN_S} s or more elsewhere in the session \
             is not measured (UNVERIFIED)"
        )
    }

    /// Whether the stamp `ts` (epoch microseconds) lies inside the day's
    /// recorded halt `[from, resume)`, the window [`in_window`] reads.
    fn in_halt(&self, ts: i64) -> bool {
        in_window(
            self.halt,
            ts.saturating_sub(self.midnight).div_euclid(1_000_000),
        )
    }

    /// The phrase that names the auction's window beside the halt, where the
    /// day has one.
    const fn aside(&self) -> &'static str {
        if self.auction.is_some() {
            " and the cash closing auction's window"
        } else {
            ""
        }
    }

    /// The auction's window on the wire, `HH:MM:SS-HH:MM:SS`, or `null`.
    fn auction_json(&self) -> serde_json::Value {
        let clock = |at: u32| format!("{:02}:{:02}:{:02}", at / 3_600, at % 3_600 / 60, at % 60);
        self.auction.map_or(serde_json::Value::Null, |(from, to)| {
            serde_json::Value::from(format!("{}-{}", clock(from), clock(to)))
        })
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
