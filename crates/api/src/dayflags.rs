//! `GET /dayflags.json?feed=<gdfl|zerodha>&instrument=<key>&from=<day|month>&to=<day|month>`
//! -- one record per (feed, instrument, day), every record carrying the same
//! keys, paged by days. DB-016, DB-025, DB-026, DB-029, DB-030, DB-031,
//! DB-053, DB-G37, DB-G47, DB-G69, DB-G87, DB-G96. D-3335 fixes the record and
//! both ledgers' line formats; D-3336 the day code; D-3319 and D-3320 the
//! special sessions, halts and phases; D-3317 the import journal; D-3315 the
//! refused fronts.
//!
//! # Why this route exists
//!
//! A day on `/db` was a row or a hole, and nothing said which kind of hole: a
//! weekend, a disaster-recovery drill the GDFL import refused, a day whose
//! index file held zero-LTP rows and was refused whole, a suspected
//! circuit-breaker halt, a Zerodha minute gap on a day GDFL traded, or a
//! Zerodha daily row priced at zero. The reasons existed -- in the import
//! journal, in the `cli` log, in `pull::calendar`, in the bars themselves --
//! and no route put them beside the day. This one does, one record per day,
//! so a page draws its day band from one request (WP-54).
//!
//! # What each record merges
//!
//! | key | source | feeds |
//! |---|---|---|
//! | `calendar`, `events` | `pull::calendar::record_of` and `phases_of`, the calendar of record | both |
//! | `import` | the import journal, read by `/imports.json`'s own reader with the importer's logged refusals | gdfl |
//! | `quality`, `degraded` | `<store>/imports/gdfl.dayflags` (D-3335 §1) | gdfl |
//! | `sampled` | whether the index (the instrument's own, or an option's underlying) is a minute-sampled day: the day ledger's line where one exists, else the bars held, said as a suspicion (WP-11c, DB-A04) | gdfl |
//! | `events.halt.held` | the feed's bars inside the calendar's halt seconds for the instrument's segment (WP-11c, DB-A07) | both |
//! | `gaps`, `refused_front` | `<store>/audit/zerodha.gaps` (D-3335 §2): always the Zerodha line | both |
//! | `pull`, `refused_front` | `<store>/audit/pull.journal`: Zerodha's member failures that refused a window before append (D-3315) or a dated day's rows (D-3344), and where the series' `1day` bars start | zerodha spot |
//! | `junk`, `day_bar` | the day's stored Zerodha `1day` bar, and the bar counts | zerodha |
//! | `held`, `peer` | the day's bars at this feed's finest rung, and the other feed's | both |
//! | `says` | the above as short phrases, in a fixed order | both |
//!
//! # Built against the format, before the writers
//!
//! WP-11b writes `gdfl.dayflags` (at import, and `cli gdfl-dayflags` for days
//! already imported) and WP-13b writes `zerodha.gaps` as each member lands.
//! This reader is built against D-3335's line formats and fixtures in them and
//! does not wait for either. A ledger that is not there yet is an answer --
//! `present: false` under `sources`, the field `null` on every record, and a
//! note naming the file -- never an error and never an empty object a reader
//! could take for "nothing was wrong". A line this reader cannot name is
//! counted and quoted with its reason, never dropped in silence, never guessed.
//!
//! # Cost
//!
//! O(days in the page), and a page holds at most [`PAGE_DAYS`]:
//! - the calendar: O(1) a day (`record_of`, `phases_of`: bounded walks of
//!   compile-time tables);
//! - the journal: `/imports.json`'s reader (a tail of the journal and of the
//!   `cli` ring), then O(rows in the page's days);
//! - each ledger: a tail -- one `metadata` call and the bytes appended since
//!   the last read -- then one `BTreeMap` probe a day, O(log n);
//! - the bars: per feed, one open per month the page touches and one
//!   `first_at_or_after` per day boundary: through the month's `.tix` at most
//!   one entry and one bar read, by the D-1434 bisection O(log n) reads (the
//!   store logs which); two more on a day the calendar halts (three days in
//!   all), and for an option on a one-second feed the same again for its
//!   underlying index's month (WP-11c);
//! - Zerodha's `1day` months: at most [`MAX_DAY_BARS`] records a month, and
//!   for the page's first day the last bar before it, at most
//!   [`LOOKBACK_MONTHS`] month opens back; where none is found, at most
//!   [`FRONT_PROBE_MONTHS`] `metadata` calls each way to find the series'
//!   first month, and one open;
//! - the pull journal, for a Zerodha spot page only: through the journal's
//!   own index (WP-33c; `crate::audit::Journal::read_index`), O(runs) for the
//!   run spans and O(refusals of the symbol) for the records it names, then
//!   O(refusals of the symbol) a day. A journal the index does not cover yet
//!   (one written before this build, until its first append or the startup
//!   build indexes it) is walked once per process, outside any lock, and the
//!   answer that pays for it says so in `notes` and `sources.pull_journal.via`,
//!   with the time it took and why (4,217,027 records, 1 GiB, on the
//!   operator's store). Since WP-36 (DB-N19) the served process walks such a
//!   journal at start, off the request path, and a request that arrives
//!   during that walk is answered without the journal and says why, rather
//!   than paying 0.7 s.
//!
//! Measured p50/p99 are in the commit that built this route and go to
//! `docs/06-limits.md` with WP-99; none is claimed here.
//!
//! UNVERIFIED as a bound: `the_real_days_are_labelled_and_the_answer_is_timed`
//! times the answer (n = 200) and asserts no time.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use brutex_core::instrument::{Contract, Exchange, Segment};
use brutex_core::vendor::Vendor;
use pull::calendar::{CasEligibility, DayKind, DayRecord, Phase, SpecialKind};
use pull::gdfl_import::ImportKind;
use pull::session::Day;
use pull::vendor::Venue;
use serde_json::{Value, json};
use store::file::{BarFile, StoreError};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{Loaded, param};

/// The most days one page holds: a quarter, so a month is one page and a
/// coverage calendar of a quarter is one request.
pub(crate) const PAGE_DAYS: u32 = 92;

/// The most refusals a record names by file; the rest are counted.
const MAX_REFUSALS_NAMED: usize = 4;

/// The most faulty lines one ledger read keeps by number; the rest are
/// counted.
const MAX_NAMED_FAULTS: usize = 16;

/// The most records a Zerodha `1day` month may hold for this reader to read
/// it whole. A month has at most 31 days, so a file holding more is one this
/// reader does not understand, and it says so rather than reading it.
const MAX_DAY_BARS: u64 = 64;

/// How many months before a page's first day the previous `1day` bar is
/// looked for, so `impossible_jump` can be judged on that day.
const LOOKBACK_MONTHS: u32 = 12;

/// `impossible_jump`: a `1day` close at least `JUMP_NUM / JUMP_DEN` (2.5)
/// times the previous held close, or at most its inverse. Chosen, not
/// sourced: no index moves by 60% in a session, and no F&O share in this
/// store's span closed at 2.5 times or 0.4 of its previous close, while the
/// vendor's junk rows (NIFTY100EQLWGT 2018-03-27, a fall of 93%; a NIFTYGS
/// index re-scaled elevenfold) are far beyond it (DB-031). A flag, never a
/// refusal: the bar is served unchanged.
const JUMP_NUM: i128 = 5;
/// See [`JUMP_NUM`].
const JUMP_DEN: i128 = 2;

/// IST's offset from UTC in seconds, as a late row's `max_back_s` would read
/// it: `pull::session::IST_OFFSET_SECS`, which is 5 h 30 min.
const IST_OFFSET_BACK_S: u64 = pull::session::IST_OFFSET_SECS.unsigned_abs();

/// Microseconds in a day.
const DAY_MICROS: i64 = 86_400 * 1_000_000;

/// The content type every answer here carries.
fn json_header() -> [(axum::http::HeaderName, &'static str); 1] {
    [(
        axum::http::header::CONTENT_TYPE,
        "application/json; charset=utf-8",
    )]
}

// ══ What a request asks ═════════════════════════════════════════════════════

/// One instrument as the census names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Instrument {
    /// The exchange.
    pub(crate) exchange: Exchange,
    /// The store segment.
    pub(crate) segment: Segment,
    /// The store symbol: the underlying, for a contract.
    pub(crate) symbol: String,
    /// The contract, for a future or an option.
    pub(crate) contract: Option<Contract>,
}

impl Instrument {
    /// The census key: `<EXCHANGE>-<SEGMENT>-<SYMBOL>`, then `-<contract>`
    /// for a contract. The Zerodha ledger keys its lines by it (D-3335 §2).
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

    /// The GDFL import kind whose journal and ledger lines cover this
    /// instrument.
    const fn kind(&self) -> ImportKind {
        match self.segment {
            Segment::Index => ImportKind::Indices,
            Segment::Cash => ImportKind::Stocks,
            Segment::Fno => ImportKind::Options,
        }
    }

    /// The venue whose phases a day of this instrument has.
    const fn venue(&self) -> Venue {
        match self.segment {
            Segment::Index => Venue::NseIndex,
            Segment::Cash => Venue::NseCash,
            Segment::Fno => Venue::NseDerivatives,
        }
    }
}

/// What a request asked for, read and checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Asked {
    /// `gdfl` or `zerodha`.
    pub(crate) feed: Vendor,
    /// The instrument.
    pub(crate) instrument: Instrument,
    /// The first day of the range.
    pub(crate) from: Day,
    /// The last day of the range.
    pub(crate) to: Day,
    /// Newest day first when `order=desc`; oldest first otherwise.
    pub(crate) newest_first: bool,
    /// The page, from 0.
    pub(crate) page: u32,
}

/// Reads a query string, or names the parameter it cannot read.
///
/// `feed` and `instrument` are required and never defaulted: a day's flags
/// are a feed's, and a default would answer for a feed the reader did not
/// pick. `from` and `to` are a day (`YYYY-MM-DD`) or a month (`YYYY-MM`,
/// its first day for `from` and its last for `to`); `to` defaults to the end
/// of `from`'s month when `from` is a month and to `from` when it is a day.
///
/// # Errors
///
/// The parameter and why, as one sentence for a 400.
pub(crate) fn asked(query: &str) -> Result<Asked, String> {
    let feed = feed_of(&param(query, "feed"))?;
    let instrument = instrument_of(&param(query, "instrument"), &param(query, "contract"))?;
    let from_text = param(query, "from");
    if from_text.is_empty() {
        return Err(
            "/dayflags.json needs from=YYYY-MM-DD or from=YYYY-MM and never guesses a range"
                .to_owned(),
        );
    }
    let from = bound(&from_text, "from", false)?;
    let to_text = param(query, "to");
    let to = if to_text.is_empty() {
        if from_text.len() == 7 {
            from.end_of_month()
        } else {
            from
        }
    } else {
        bound(&to_text, "to", true)?
    };
    if from > to {
        return Err(format!("from={from} is after to={to}"));
    }
    let newest_first = match param(query, "order").as_str() {
        "" | "asc" => false,
        "desc" => true,
        other => return Err(format!("order={other:?} is not asc or desc")),
    };
    let page_word = param(query, "page");
    let page = if page_word.is_empty() {
        0
    } else {
        page_word
            .parse::<u32>()
            .ok()
            .filter(|page| page.to_string() == page_word)
            .ok_or_else(|| format!("page={page_word:?} is not a whole number"))?
    };
    Ok(Asked {
        feed,
        instrument,
        from,
        to,
        newest_first,
        page,
    })
}

/// The feed a word names: Zerodha or GDFL, the operator's two (answer 7).
fn feed_of(word: &str) -> Result<Vendor, String> {
    let (gdfl, zerodha) = (Vendor::Gdfl, Vendor::Zerodha);
    if word.is_empty() {
        return Err(format!(
            "/dayflags.json needs ?feed={} or ?feed={} and never guesses one",
            gdfl.as_str(),
            zerodha.as_str()
        ));
    }
    if word.eq_ignore_ascii_case(gdfl.as_str()) {
        Ok(gdfl)
    } else if word.eq_ignore_ascii_case(zerodha.as_str()) {
        Ok(zerodha)
    } else {
        Err(format!(
            "feed {word:?} has no day flags: this route reads {} and {} only",
            gdfl.as_str(),
            zerodha.as_str()
        ))
    }
}

/// The instrument a census key names, with the contract either inside the
/// key (`NSE-FNO-NIFTY-2019-03-28-1150000-CE`, the census's own spelling) or
/// in `contract=`. Both, disagreeing, is refused; a contract on a spot
/// segment is refused.
fn instrument_of(text: &str, contract_text: &str) -> Result<Instrument, String> {
    let shape = "is not <EXCHANGE>-<SEGMENT>-<SYMBOL>, for example NSE-INDEX-NIFTY, \
                 NSE-CASH-RELIANCE or NSE-FNO-NIFTY-2019-03-28-1150000-CE";
    if text.is_empty() {
        return Err(format!("/dayflags.json needs instrument=<key>: it {shape}"));
    }
    let mut parts = text.splitn(3, '-');
    let (Some(exchange), Some(segment), Some(rest)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("instrument={text:?} {shape}"));
    };
    let exchange = Exchange::parse(exchange)
        .map_err(|_| format!("instrument={text:?}: {exchange:?} is not an exchange"))?;
    let segment = Segment::parse(segment)
        .map_err(|_| format!("instrument={text:?}: {segment:?} is not one of INDEX, CASH, FNO"))?;
    let (symbol, embedded) = if segment == Segment::Fno {
        split_contract(rest)
    } else {
        (rest, None)
    };
    let symbol_ok = brutex_core::symbol::Symbol::new(symbol)
        .ok()
        .is_some_and(|parsed| parsed.as_str() == symbol);
    if !symbol_ok {
        return Err(format!(
            "instrument={text:?}: {symbol:?} is not a store symbol as the census spells it"
        ));
    }
    let asked_contract = if contract_text.is_empty() {
        None
    } else {
        Some(Contract::parse(contract_text).ok_or_else(|| {
            format!("contract={contract_text:?} is not a contract segment this store can name")
        })?)
    };
    let contract = match (embedded, asked_contract) {
        (Some(inside), Some(asked)) if inside != asked => {
            return Err(format!(
                "instrument={text:?} names contract {} and contract={contract_text:?} another",
                inside.as_str()
            ));
        }
        (inside, asked) => inside.or(asked),
    };
    if contract.is_some() && segment != Segment::Fno {
        return Err(format!(
            "instrument={text:?} is {} and a contract belongs to FNO only",
            segment.as_str()
        ));
    }
    Ok(Instrument {
        exchange,
        segment,
        symbol: symbol.to_owned(),
        contract,
    })
}

/// `<SYMBOL>` and the contract after it, when the text after a `-` is a
/// contract segment (`YYYY-MM-DD-<strike>-<CE|PE>` or `YYYY-MM-DD-FUT`). A
/// symbol may hold a `-` (`BAJAJ-AUTO`), so the first split whose remainder
/// is a contract wins. Bounded by the key's length.
fn split_contract(rest: &str) -> (&str, Option<Contract>) {
    for (at, _) in rest.match_indices('-') {
        let (Some(head), Some(tail)) = (rest.get(..at), rest.get(at.saturating_add(1)..)) else {
            continue;
        };
        if contract_shaped(tail)
            && let Some(contract) = Contract::parse(tail)
        {
            return (head, Some(contract));
        }
    }
    (rest, None)
}

/// Whether `text` opens `YYYY-MM-DD-` and closes `-CE`, `-PE` or `-FUT`.
fn contract_shaped(text: &str) -> bool {
    let bytes = text.as_bytes();
    let digits = |from: usize, to: usize| {
        bytes
            .get(from..to)
            .is_some_and(|run| run.iter().all(u8::is_ascii_digit))
    };
    let dash = |at: usize| bytes.get(at) == Some(&b'-');
    digits(0, 4)
        && dash(4)
        && digits(5, 7)
        && dash(7)
        && digits(8, 10)
        && dash(10)
        && (text.ends_with("-CE") || text.ends_with("-PE") || text.ends_with("-FUT"))
}

/// A day, or a month read as its first (`end: false`) or last day.
fn bound(text: &str, name: &str, end: bool) -> Result<Day, String> {
    if let Some(day) = super::import_journal::parse_day(text) {
        return Ok(day);
    }
    let month = text.split_once('-').and_then(|(year, month)| {
        let canonical = year.len() == 4 && month.len() == 2;
        let day = Day::new(year.parse().ok()?, month.parse().ok()?, 1).ok()?;
        canonical.then_some(day)
    });
    match month {
        Some(first) if end => Ok(first.end_of_month()),
        Some(first) => Ok(first),
        None => Err(format!(
            "{name}={text:?} is neither a day as YYYY-MM-DD nor a month as YYYY-MM"
        )),
    }
}

// ══ The route ═══════════════════════════════════════════════════════════════

/// `GET /dayflags.json`.
pub(super) async fn dayflags_json(
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
        Ok(body) => match being_written(&body) {
            Some((status, said)) => (status, json_header(), said),
            None => (axum::http::StatusCode::OK, json_header(), body.to_string()),
        },
        Err(why) => {
            let (status, body) = crate::detail::admission_refused(
                "day flags read",
                crate::detail::MAX_STORE_READ_CONCURRENT,
                &why,
            );
            (status, json_header(), body)
        }
    }
}

/// The "being written, ask again" 503 for an [`answer`] whose EVERY month
/// fault is a writer holding the month past the read's patience — the same
/// small body every bar route answers a writer with, so `count_route` adds
/// `Retry-After`, counts it as being written, and the page asks again by
/// itself (R3-08, RA-117). Before this a page beside a writer-held month was
/// 200 with every day's cell `"unreadable"` (30 of 30 measured on a copy of
/// gdfl NIFTY 1s 2019-11 held by `lockf(1)`) and nothing asked again. `None`
/// when nothing failed, or when anything failed for another reason: real
/// damage beside a busy month is answered as damage, its writer-held cells
/// saying `being_written`.
pub(crate) fn being_written(body: &Value) -> Option<(axum::http::StatusCode, String)> {
    let faults: Vec<String> = body
        .get("month_faults")?
        .as_array()?
        .iter()
        .filter_map(|fault| fault.as_str().map(str::to_owned))
        .collect();
    crate::detail::being_written_answer(&faults)
}

/// The page's days: `(first, last)` of page `page` (clamped), and the count
/// of pages over the asked range.
fn page_span(asked: &Asked) -> (Day, Day, u32, u32) {
    let from = asked.from.days_from_epoch();
    let to = asked.to.days_from_epoch();
    let total = to.saturating_sub(from).saturating_add(1);
    let pages = total.div_ceil(PAGE_DAYS).max(1);
    // CLAMPED, NOT REFUSED: a page past the end is a stale bookmark, and the
    // answer says which page it is.
    let page = asked.page.min(pages.saturating_sub(1));
    let skip = page.saturating_mul(PAGE_DAYS);
    let (first, last) = if asked.newest_first {
        let last = to.saturating_sub(skip);
        (
            last.saturating_sub(PAGE_DAYS.saturating_sub(1)).max(from),
            last,
        )
    } else {
        let first = from.saturating_add(skip);
        (first, first.saturating_add(PAGE_DAYS - 1).min(to))
    };
    let day = |days: u32, fallback: Day| Day::from_days(days).unwrap_or(fallback);
    (day(first, asked.from), day(last, asked.to), page, pages)
}

/// The other feed: the one a record counts beside its own.
const fn peer_of(feed: Vendor) -> Vendor {
    match feed {
        Vendor::Gdfl => Vendor::Zerodha,
        _ => Vendor::Gdfl,
    }
}

/// A feed's finest rung: GDFL's one-second bars, Zerodha's one-minute bars.
const fn finest(feed: Vendor) -> Timeframe {
    match feed {
        Vendor::Gdfl => Timeframe::SECOND_1,
        _ => Timeframe::MINUTE_1,
    }
}

/// The whole answer over the store at `root`. Never an error: every source
/// that cannot be read is named under `sources` and in `notes`, and the
/// other sources still answer.
pub(crate) fn answer(root: &Path, asked: &Asked) -> Value {
    let (first, last, page, pages) = page_span(asked);
    let days = days_between(first, last);
    let inst = &asked.instrument;
    let peer_feed = peer_of(asked.feed);
    let mut notes: Vec<String> = Vec::new();
    let journal = (asked.feed == Vendor::Gdfl).then(|| journal_rows(root, inst, first, last));
    let quality =
        (asked.feed == Vendor::Gdfl).then(|| read_ledger(quality_tails(), &quality_path(root)));
    let gaps = read_ledger(gap_tails(), &gaps_path(root));
    // A ZERODHA SHARE'S ZERO-VOLUME MINUTES ARE COUNTED (DB-B16): a candle
    // for a minute nobody traded is held but not traded. An index has no
    // volume at all (D-3322), and GDFL holds no bar for a second with no trade.
    let fills = asked.feed == Vendor::Zerodha && inst.segment == Segment::Cash;
    let underlying = sampled_series(asked.feed, inst);
    let held = PageHeld::count(root, asked, &days, (fills, underlying.as_ref()));
    notes.extend(held.notes());
    let mut month_faults: Vec<String> = held.faults().collect();
    let day_bars = (asked.feed == Vendor::Zerodha && inst.segment != Segment::Fno)
        .then(|| day_bars(root, inst, &days));
    if let Some(bars) = &day_bars {
        notes.extend(bars.notes.iter().cloned());
        month_faults.extend(bars.faults.iter().cloned());
    }
    // THE PULL JOURNAL IS READ FOR A ZERODHA SPOT PAGE ONLY. Zerodha pulls
    // spot, so an option has no record there; a GDFL page carries Zerodha's
    // gap line as its peer's and does not pay the journal's first walk (see
    // `read_pull_journal`), which is the one cost here that is not per day.
    let zerodha_spot = asked.feed == Vendor::Zerodha && inst.segment != Segment::Fno;
    let pull = zerodha_spot.then(|| read_pull_journal(root, inst.symbol.as_str()));
    let pull_ok = pull.as_ref().and_then(|p| p.as_ref().ok());
    // WHERE THE SERIES STARTS decides whether a day can be a refused front
    // (D-3315). A `1day` bar before the page, which `day_bars` already found,
    // settles it with no probe; otherwise `series_start` probes months.
    let start = zerodha_spot.then(|| {
        let held_before = day_bars
            .as_ref()
            .and_then(|bars| bars.prev_close.first().copied().flatten())
            .is_some();
        match days.first() {
            _ if held_before => SeriesStart::HeldBefore,
            Some(first) => series_start(root, inst, *first),
            None => SeriesStart::NoneHeld,
        }
    });
    let ctx = Context {
        asked,
        sampled_of: underlying.as_ref(),
        journal: journal.as_ref().and_then(|j| j.as_ref().ok()),
        quality: quality.as_ref().and_then(|q| q.as_ref().ok()),
        gaps: gaps.as_ref().ok(),
        pull: pull_ok,
        start,
    };
    let mut records: Vec<Value> = days
        .iter()
        .enumerate()
        .map(|(at, day)| {
            let bar = day_bars.as_ref().map(|bars| bars.at(at));
            ctx.record(*day, &held.day(at), bar)
        })
        .collect();
    if asked.newest_first {
        records.reverse();
    }
    notes.extend(source_notes(
        asked,
        journal.as_ref(),
        quality.as_ref(),
        &gaps,
    ));
    notes.extend(pull_notes(pull.as_ref()));
    json!({
        "feed": asked.feed.as_str(),
        "instrument": inst.key(),
        "kind": inst.kind().as_str(),
        "asked": {
            "from": asked.from.to_string(),
            "to": asked.to.to_string(),
            "order": if asked.newest_first { "desc" } else { "asc" },
        },
        "page": page,
        "pages": pages,
        "page_days": PAGE_DAYS,
        "page_from": first.to_string(),
        "page_to": last.to_string(),
        "days_in_range": asked.to.days_from_epoch().saturating_sub(asked.from.days_from_epoch()).saturating_add(1),
        "rungs": {
            "held": { "feed": asked.feed.as_str(), "rung": finest(asked.feed).as_str() },
            "peer": { "feed": peer_feed.as_str(), "rung": finest(peer_feed).as_str() },
            "day_bar": day_bars.as_ref().map(|_| Timeframe::DAY_1.as_str()),
        },
        "zerodha_1day_start": start.map(SeriesStart::json),
        "days": records,
        "sources": sources_json(journal.as_ref(), quality.as_ref(), &gaps, pull.as_ref()),
        "derive": derive_json(gaps.as_ref().ok(), inst, first, last),
        "lineage": lineage_json(root, asked.feed, inst),
        "rules": rules_json(),
        "notes": notes,
        // EVERY MONTH THAT WOULD NOT OPEN, whatever the cause (R3-08, RA-117):
        // when a writer is the only cause the route answers "being written,
        // ask again" from these, and a reader of the body sees them here.
        "month_faults": month_faults,
    })
}

/// Every day of `first..=last`, oldest first.
fn days_between(first: Day, last: Day) -> Vec<Day> {
    (first.days_from_epoch()..=last.days_from_epoch())
        .filter_map(|days| Day::from_days(days).ok())
        .collect()
}

/// The lineage ledger, read for one contract's records: by census key, the
/// records naming it, and the lines that did not parse.
#[derive(Debug, Default)]
struct LineageIndex {
    /// Each key's records, as the answer carries them.
    by_key: HashMap<String, Vec<Value>>,
    /// Each underlying's `event` records (D-3355), in ledger order: what
    /// moved on an ex-date, its lots, and how many keys it paired and
    /// stopped. The last record of an ex-date is the one in force.
    events: HashMap<String, Vec<Value>>,
    /// Lines that are none of `alias`, `lineage`, `stitched`, `event`,
    /// `stopped` and the re-key's `superseded` and `retired` records.
    unreadable: usize,
}

/// The ledger's file stamp: length, mtime seconds and nanoseconds.
type FileStamp = (u64, i64, i64);

/// The lineage ledger by path, kept under its file stamp: a request that
/// finds it unchanged is one map probe, whatever the ledger's length.
static LINEAGE: OnceLock<Mutex<LineageMemo>> = OnceLock::new();

/// Each lineage ledger read, by path: its stamp when read, and its index.
type LineageMemo = HashMap<PathBuf, (FileStamp, Arc<LineageIndex>)>;

/// One ledger text, indexed by the census key each record is about. A
/// `lineage` record is filed under both keys (the old key `continues_as` the
/// new one, the new key is `renamed_from` the old), a `stitched` record under
/// its key, a re-key `superseded` record under both (`superseded_by`,
/// `supersedes`) and a `retired` record under its key (D-3331; WP-12c).
/// `alias` records are the decoder's (D-3331 item 1), not a key's.
fn index_lineage(text: &str) -> LineageIndex {
    let mut index = LineageIndex::default();
    let key = pull::gdfl_lineage::option_key;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        if line.starts_with("alias ") {
            continue;
        }
        if let Some(record) = pull::gdfl_lineage::LineageRecord::parse(line) {
            let (old, new) = (
                key(record.underlying, record.old),
                key(record.underlying, record.new),
            );
            let factor = format!("{}/{}", record.factor.0, record.factor.1);
            let ex = record.ex_date.to_string();
            index.by_key.entry(old.clone()).or_default().push(json!({
                "kind": "continues_as", "key": new, "ex": ex, "factor": factor,
                "lot_old": record.lot_old, "lot_new": record.lot_new,
            }));
            index.by_key.entry(new).or_default().push(json!({
                "kind": "renamed_from", "key": old, "ex": ex, "factor": factor,
                "lot_old": record.lot_old, "lot_new": record.lot_new,
            }));
        } else if let Some(record) = pull::gdfl_lineage::StitchedRecord::parse(line) {
            let other = |contract: Option<Contract>| contract.map(|c| key(record.underlying, c));
            index
                .by_key
                .entry(key(record.underlying, record.key))
                .or_default()
                .push(json!({
                    "kind": "stitched", "ex": record.ex_date.to_string(),
                    "before": other(record.before), "after": other(record.after),
                }));
        } else if let Some(record) = pull::gdfl_lineage::SupersededRecord::parse(line) {
            // THE RE-KEY'S RECORDS ARE READ, never "unreadable" (D-3331;
            // WP-12c, DB-A17): the stated key is `superseded_by` the key the
            // decoder files it under, which `supersedes` it with a copy of its
            // bars, the count and BLAKE3 as the ledger states them.
            let (old, new) = (
                key(record.underlying, record.old),
                key(record.underlying, record.new),
            );
            let (why, digest) = (record.kind.as_str(), record.digest_hex());
            index.by_key.entry(old.clone()).or_default().push(json!({
                "kind": "superseded_by", "key": new, "why": why,
                "months": record.months, "bars": record.bars, "blake3": digest,
            }));
            index.by_key.entry(new).or_default().push(json!({
                "kind": "supersedes", "key": old, "why": why,
                "months": record.months, "bars": record.bars, "blake3": digest,
            }));
        } else if let Some(record) = pull::gdfl_lineage::RetiredRecord::parse(line) {
            // AND A KEY THE DECODER REFUSES ON EVERY DAY (DB-A18), with the
            // sentence every reader refuses it by.
            index
                .by_key
                .entry(key(record.underlying, record.key))
                .or_default()
                .push(json!({
                    "kind": "retired", "why": record.why.as_str(),
                    "day": record.day.to_string(), "median": record.median,
                    "days": record.days, "bars": record.bars,
                    "refusal": record.refusal(),
                }));
        } else if let Some(record) = pull::gdfl_lineage::StoppedRecord::parse(line) {
            // A PRE-EX KEY AN EVENT RETIRED WITH NO PARTNER (D-3355, DB-B13):
            // the exchange contract under it ended the day before `ex`.
            index
                .by_key
                .entry(key(record.underlying, record.key))
                .or_default()
                .push(json!({ "kind": "stopped", "ex": record.ex_date.to_string() }));
        } else if let Some(record) = pull::gdfl_lineage::EventRecord::parse(line) {
            // THE EVENT ITSELF, by underlying: a re-listing pairs nothing, so
            // this is the only line that says what happened to its keys.
            index
                .events
                .entry(record.underlying.as_str().to_owned())
                .or_default()
                .push(json!({
                    "ex": record.ex_date.to_string(), "kind": record.kind.as_str(),
                    "shift_paisa": record.kind.shift(),
                    "lot_old": record.lot_old, "lot_new": record.lot_new,
                    "paired": record.paired, "stopped": record.stopped,
                }));
        } else {
            index.unreadable = index.unreadable.saturating_add(1);
        }
    }
    index
}

/// The ledger at `path`, read once per stamp: `Ok(None)` when no ledger is
/// written yet, `Err` naming why one that exists would not read.
fn read_lineage(path: &Path) -> Result<Option<Arc<LineageIndex>>, String> {
    let Ok(meta) = std::fs::metadata(path) else {
        return Ok(None);
    };
    let stamp = (
        meta.len(),
        std::os::unix::fs::MetadataExt::mtime(&meta),
        std::os::unix::fs::MetadataExt::mtime_nsec(&meta),
    );
    let memo = LINEAGE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some((kept, index)) = memo
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(path)
        && *kept == stamp
    {
        return Ok(Some(Arc::clone(index)));
    }
    let text = std::fs::read_to_string(path)
        .map_err(|why| format!("cannot read {}: {why}", path.display()))?;
    let index = Arc::new(index_lineage(&text));
    memo.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(path.to_path_buf(), (stamp, Arc::clone(&index)));
    Ok(Some(index))
}

/// WHERE A CONTRACT'S SERIES CONTINUES, from the import's lineage ledger
/// (D-3331 item 4, DB-G72). At a share's bonus or split the vendor renames
/// every open contract to its adjusted strike: BRITANNIA's 2018-12-27 6000
/// CE stops on 2018-11-28 and the same contract trades on as the 3000 CE
/// from 2018-11-29. The ledger links the two keys; this names the link on
/// either key, so /db says "renamed at the ex-date, continues as <key>"
/// instead of a life that seems to stop weeks before expiry.
///
/// `null` for anything but a GDFL option or future. A ledger not written yet
/// is an answer (`present: false`, with the reason), never an empty list a
/// reader could take for "never renamed". O(1) a request once the ledger is
/// indexed: one stamp, one probe.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
fn lineage_json(root: &Path, feed: Vendor, inst: &Instrument) -> Value {
    if feed != Vendor::Gdfl || inst.segment != Segment::Fno || inst.contract.is_none() {
        return Value::Null;
    }
    let path = pull::gdfl_lineage::lineage_path(root);
    match read_lineage(&path) {
        Ok(None) => json!({
            "present": false,
            "records": [],
            "why": format!(
                "no lineage ledger at {} yet: the GDFL options import writes it as it detects \
                 an ex-date (D-3331), and days imported before it are covered by a rescan \
                 (`cli gdfl-dayflags options`, WP-80)",
                path.display()
            ),
        }),
        Ok(Some(index)) => json!({
            "present": true,
            "records": index.by_key.get(&inst.key()).cloned().unwrap_or_default(),
            "events": index
                .events
                .get(inst.symbol.as_str())
                .cloned()
                .unwrap_or_default(),
            "unreadable_lines": index.unreadable,
            "why": null,
        }),
        Err(why) => json!({ "present": true, "records": [], "why": why }),
    }
}

// ══ The import journal, through /imports.json's reader ══════════════════════

/// The journal's rows for the page's days and the instrument's kind, keyed
/// by day, and the last day the journal names for that kind.
struct JournalRows {
    /// One row a day: of the rows whose filter NAMES the instrument's symbol
    /// (a one-name filter, or a comma list holding the whole name), the one
    /// with the newest closing line; else the whole tree's (`*`). AF-02,
    /// ingest-6: see [`Names`].
    by_day: HashMap<String, Value>,
    /// The last day any line names for this kind.
    last_day: Option<Day>,
    /// The journal file's own state, as `/imports.json` gives it.
    journal: Value,
    /// Every `run` line (D-3317 item 4): its id, the second it started and
    /// the commit it was built from, oldest first. A logged refusal is
    /// attributed to the run that was running when it was logged (DB-B45).
    runs: Vec<RunLine>,
    /// Each day's closing runs: the `run=` of every row of the kind and day,
    /// under ANY filter, the instrument's or not. A logged refusal is a
    /// close's only before the next of these started (AF-02 repair 2): a later
    /// list close's refusal is never an earlier close's.
    closers: HashMap<String, Vec<u64>>,
}

/// One `run` line of the import journal, as a refusal's attribution needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunLine {
    /// The run's id (`run=` on its closing lines).
    id: u64,
    /// Unix seconds the run started (`started=`), its id where absent.
    started: u64,
    /// The commit the run was built from, where the line names one.
    commit: Option<String>,
}

/// The run lines of an `/imports.json` answer, oldest first.
fn runs_of(body: &Value) -> Vec<RunLine> {
    let mut runs: Vec<RunLine> = body
        .get("runs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|run| {
            let id = run.get("id").and_then(Value::as_u64)?;
            let fields = run.get("fields");
            let field = |name: &str| fields.and_then(|f| f.get(name)).and_then(Value::as_str);
            Some(RunLine {
                id,
                started: field("started")
                    .and_then(|text| text.parse::<u64>().ok())
                    .unwrap_or(id),
                commit: field("commit").map(str::to_owned),
            })
        })
        .collect();
    runs.sort_by_key(|run| (run.started, run.id));
    runs
}

/// The journal rows of `first..=last` for `inst`'s kind, read by
/// `/imports.json`'s own reader so the two routes cannot disagree about a day.
fn journal_rows(
    root: &Path,
    inst: &Instrument,
    first: Day,
    last: Day,
) -> Result<JournalRows, String> {
    let kind = inst.kind();
    let mut by_day: HashMap<String, Value> = HashMap::with_capacity(PAGE_DAYS as usize);
    // Each kept row's rank: its closing line, then whether its filter names
    // the symbol. A higher rank replaces a lower one: the newest close that
    // covers the instrument is where its day stands.
    let mut ranks: HashMap<String, (u64, bool)> = HashMap::with_capacity(PAGE_DAYS as usize);
    let mut names = Names::new(inst.symbol.as_str());
    let mut page = 0usize;
    let mut journal = Value::Null;
    let mut last_day = None;
    let mut runs = Vec::new();
    let mut closers: HashMap<String, Vec<u64>> = HashMap::with_capacity(PAGE_DAYS as usize);
    loop {
        let asked = super::imports::Asked {
            from: Some(first),
            to: Some(last),
            kind: Some(kind),
            newest_first: false,
            page,
            ..super::imports::Asked::whole()
        };
        let body = super::imports::answer(root, &asked)?;
        if page == 0 {
            journal = body.get("journal").cloned().unwrap_or(Value::Null);
            runs = runs_of(&body);
            last_day = body
                .get("kinds")
                .and_then(Value::as_array)
                .and_then(|kinds| {
                    kinds
                        .iter()
                        .find_map(|k| (k.get("kind")? == kind.as_str()).then(|| k.get("last_day")))
                        .flatten()
                })
                .and_then(Value::as_str)
                .and_then(super::import_journal::parse_day);
        }
        for row in body
            .get("rows")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let (Some(day), Some(filter)) = (
                row.get("day").and_then(Value::as_str),
                row.get("filter").and_then(Value::as_str),
            ) else {
                continue;
            };
            // EVERY CLOSE BOUNDS THE ONE BEFORE IT, whichever filter wrote it
            // (AF-02 repair 2): at most the handful of runs that closed a day.
            if let Some(run) = row.get("run").and_then(Value::as_u64) {
                let ids = closers.entry(day.to_owned()).or_default();
                if !ids.contains(&run) {
                    ids.push(run);
                }
            }
            // THE NEWEST CLOSE THAT COVERS THE INSTRUMENT IS WHERE IT STANDS.
            // A run given symbols journals their day under its list of names
            // (`BANKNIFTY,NIFTY`, the 277-name key), and a list naming this
            // symbol is this instrument's close (AF-02, ingest-6); `*` is the
            // whole tree's, and covers it too. Of the closes that cover it,
            // the newest line wins, whichever filter wrote it: the importer
            // skips a day by kind, day AND filter (`pull::gdfl_import::admit`),
            // so a `*` run re-imports a day a list closed, and that later
            // close is the day's current state, clean or failed. A naming
            // list beats `*` only on an equal line (none in a journal; the
            // tie is the log-only row's line 0). A filter that does not name
            // the symbol is another underlying's and says nothing about this
            // one.
            let own = names.names(filter);
            if !own && filter != "*" {
                continue;
            }
            let rank = (row.get("line").and_then(Value::as_u64).unwrap_or(0), own);
            if ranks.get(day).is_none_or(|kept| rank > *kept) {
                let _was = ranks.insert(day.to_owned(), rank);
                let _replaced = by_day.insert(day.to_owned(), row.clone());
            }
        }
        let pages = body.get("pages").and_then(Value::as_u64).unwrap_or(1);
        page = page.saturating_add(1);
        if u64::try_from(page).unwrap_or(u64::MAX) >= pages {
            break;
        }
    }
    Ok(JournalRows {
        by_day,
        last_day,
        journal,
        runs,
        closers,
    })
}

/// Whether a journal filter names one symbol, decided once per distinct
/// filter string per request (AF-02, ingest-6).
///
/// # Whole names, not text
///
/// A filter is the importer's scope: `*`, one name, or names joined by `,`
/// (`BANKNIFTY,NIFTY`, the 277-name key the operator's batch writes). It
/// names a symbol when one of its comma-separated names IS the symbol:
/// `NIFTY` is named by `BANKNIFTY,NIFTY` and not by `NIFTYNXT50,BANKNIFTY`,
/// whose text merely contains it. `*` names no symbol: it is the whole tree,
/// and covers every symbol; of the closes that cover one, the newest line
/// wins ([`journal_rows`]).
///
/// # Cost
///
/// A page of [`PAGE_DAYS`] days reads a handful of keys a day, and the
/// operator's journal holds five distinct filters for options (`*`, the
/// two-name list, two 277-name lists). The split, O(bytes of the filter), is
/// paid once per distinct filter of the request; every other row is one hash
/// probe of its filter text. A per-row split would cost the 2,200-byte list
/// on every row of every page.
struct Names<'s> {
    /// The symbol asked about.
    symbol: &'s str,
    /// Each distinct filter seen in this request, and whether it names the
    /// symbol.
    seen: HashMap<String, bool>,
}

impl<'s> Names<'s> {
    fn new(symbol: &'s str) -> Self {
        Self {
            symbol,
            seen: HashMap::with_capacity(8),
        }
    }

    /// Whether `filter` names the symbol: memoised.
    fn names(&mut self, filter: &str) -> bool {
        if let Some(named) = self.seen.get(filter) {
            return *named;
        }
        #[cfg(test)]
        SPLITS.with(|splits| splits.set(splits.get().saturating_add(1)));
        let named = filter != "*" && filter.split(',').any(|name| name == self.symbol);
        let _first = self.seen.insert(filter.to_owned(), named);
        named
    }
}

#[cfg(test)]
thread_local! {
    /// Filters split on this thread, for the test that pins the memo.
    static SPLITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What a logged refusal is about, as this instrument sees it.
fn about_this(about: &str, inst: &Instrument, day: Day) -> bool {
    // A DAY-LEVEL REFUSAL is about every instrument of the kind: the calendar
    // declined the day, or the census refused it.
    let decisive = [
        pull::gdfl_import::Stage::Calendar.as_str(),
        pull::gdfl_import::Stage::Census.as_str(),
    ];
    if about.is_empty() || decisive.into_iter().any(|word| word == about) {
        return true;
    }
    match inst.segment {
        Segment::Index | Segment::Cash => {
            let tree = if inst.segment == Segment::Index {
                pull::gdfl_cm::CmKind::Indices
            } else {
                pull::gdfl_cm::CmKind::Stocks
            };
            let stem = pull::gdfl_cm::split_name(about).map_or(about, |(stem, _)| stem);
            matches!(
                pull::gdfl_cm::resolve(tree, stem),
                pull::gdfl_cm::Resolution::Swept(key) if key.underlying.as_str() == inst.symbol
            )
        }
        Segment::Fno => {
            let ticker = about.strip_suffix(".NFO").unwrap_or(about);
            pull::gdfl_nfo::decode_ticker(ticker, day).is_ok_and(|decoded| {
                decoded.underlying.as_str() == inst.symbol
                    && inst
                        .contract
                        .is_none_or(|contract| contract == decoded.contract)
            })
        }
    }
}

/// The refusals the importer logged for one journal row, as one instrument
/// sees them.
#[derive(Debug, Default)]
struct Refusals {
    /// Distinct `(about, why)` about this instrument, the first
    /// [`MAX_REFUSALS_NAMED`].
    this: Vec<(String, String)>,
    /// Distinct refusals about this instrument.
    this_count: usize,
    /// Distinct refusals about another file of the same kind-day.
    other_count: usize,
    /// The first of those, so a day incomplete for another file says which
    /// (DB-G69).
    other_first: Option<(String, String)>,
    /// Distinct refusals an EARLIER run logged for this kind-day, the first
    /// [`MAX_REFUSALS_NAMED`]: history, never the current reason (DB-B45).
    superseded: Vec<Superseded>,
    /// How many distinct refusals earlier runs logged.
    superseded_count: usize,
    /// How many distinct refusals a LATER run that closed the same kind-day
    /// logged: that run's, never this close's (AF-02 repair 2).
    later_count: usize,
    /// How many of those are about this instrument: logged in a later
    /// closing run's window whose filter does not name it, so no close of
    /// the day owns them (`/imports.json` keeps their log-only `*` row).
    later_this_count: usize,
    /// The first of those.
    later_this_first: Option<(String, String)>,
}

/// The second run `id` started, as its `run` line names it, else the id
/// itself (a run's id is the second it started, D-3317).
fn start_of(runs: &[RunLine], id: u64) -> u64 {
    runs.iter()
        .find(|run| run.id == id)
        .map_or(id, |run| run.started)
}

/// One refusal an earlier run logged, attributed to its run where the
/// journal records one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Superseded {
    /// The run that was running when it was logged, or `None` for a run
    /// before the journal recorded `run` lines (D-3317).
    run: Option<u64>,
    /// That run's commit, where its line names one.
    commit: Option<String>,
    /// When it was logged, epoch milliseconds.
    at_ms: Option<u64>,
    /// The file or stage it is about.
    about: String,
    /// The importer's words.
    why: String,
}

/// The refusals the importer logged for a row, split into those about this
/// instrument and those about another file of the same kind-day, and those
/// logged by an EARLIER run than the one whose line closes the row (DB-B45).
/// Each distinct refusal is decided once.
///
/// # Grouped by run
///
/// A day attempted by several runs logged each run's refusals, and the
/// importer's events carry no run id. The row's closing line does
/// (`run=`, D-3317), and every `run` line names the second it started: a
/// refusal logged at or after the closing run's start is that run's, and
/// one logged before it is an earlier run's, attributed to the latest run
/// started before it, or to none for a run before the journal recorded run
/// lines. 2024-03-02's indices were refused by a build that called the drill
/// "not a regular session", then admitted and refused at filing by run
/// 1791602895: the current reason is the filing, and the calendar refusal is
/// history. A row with no `run=` (a line written before D-3317) keeps every
/// refusal current: nothing says which run logged it.
///
/// # Bounded by the next close (AF-02 repair 2)
///
/// A refusal logged at or after `next_s`, the start of the next run that
/// closed the same kind-day under any filter ([`JournalRows::closers`]), is
/// that run's and never this close's: a later `ABCAPITAL,RELIANCE` run's
/// refusal is not the first of an earlier `BANKNIFTY,NIFTY` close. It is
/// counted as `later_count` and named nowhere else. The placing is
/// `/imports.json`'s own ([`super::imports::placed`]), so the two routes
/// cannot disagree about whose a refusal is.
fn refusals_of(
    row: &Value,
    inst: &Instrument,
    day: Day,
    runs: &[RunLine],
    next_s: Option<u64>,
) -> Refusals {
    let mut distinct: std::collections::BTreeSet<(String, String)> =
        std::collections::BTreeSet::new();
    let mut history: std::collections::BTreeSet<(String, String)> =
        std::collections::BTreeSet::new();
    let mut later: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
    let mut out = Refusals::default();
    let started_s = row
        .get("run")
        .and_then(Value::as_u64)
        .map(|id| start_of(runs, id));
    for logged in row
        .get("logged")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let text = |key: &str| {
            logged
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        };
        let pair = (text("about"), text("why"));
        let at_ms = logged.get("at_ms").and_then(Value::as_u64);
        let placed = super::imports::placed(at_ms, started_s, next_s);
        if placed == super::imports::Placed::After {
            if later.insert(pair.clone()) {
                out.later_count = out.later_count.saturating_add(1);
                // ONE ABOUT THIS INSTRUMENT is said, never dropped: a later
                // run closed the day for other files, and this refusal is in
                // its window (an attempt that left no line naming this one).
                if about_this(&pair.0, inst, day) {
                    out.later_this_count = out.later_this_count.saturating_add(1);
                    if out.later_this_first.is_none() {
                        out.later_this_first = Some(pair);
                    }
                }
            }
            continue;
        }
        if placed == super::imports::Placed::Before {
            if !history.insert(pair.clone()) {
                continue;
            }
            out.superseded_count = out.superseded_count.saturating_add(1);
            if out.superseded.len() < MAX_REFUSALS_NAMED {
                let by = at_ms.and_then(|at| {
                    runs.iter()
                        .rev()
                        .find(|run| run.started.saturating_mul(1_000) <= at)
                });
                out.superseded.push(Superseded {
                    run: by.map(|run| run.id),
                    commit: by.and_then(|run| run.commit.clone()),
                    at_ms,
                    about: pair.0,
                    why: pair.1,
                });
            }
            continue;
        }
        // DISTINCT, NOT LOG LINES: a day attempted by two runs logs each
        // refusal twice. `distinct` holds every distinct pair of one row:
        // bounded by the files of one kind-day.
        if !distinct.insert(pair.clone()) {
            continue;
        }
        if about_this(&pair.0, inst, day) {
            out.this_count = out.this_count.saturating_add(1);
            if out.this.len() < MAX_REFUSALS_NAMED {
                out.this.push(pair);
            }
        } else {
            out.other_count = out.other_count.saturating_add(1);
            if out.other_first.is_none() {
                out.other_first = Some(pair);
            }
        }
    }
    out
}

/// The record's `import`: the journal row's standing, or `none` with whether
/// the import has reached the day.
fn import_json(rows: &JournalRows, inst: &Instrument, day: Day) -> Value {
    let Some(row) = rows.by_day.get(&day.to_string()) else {
        let reached = rows.last_day.is_some_and(|last| day <= last);
        return json!({
            "outcome": "none",
            "reached": reached,
            "stage": null, "reason": null, "files": null, "failures": null, "run": null,
            "from": null, "line": null, "late": null, "max_back_s": null,
            "refusals": {
                "this": [], "this_count": 0, "other_count": 0, "other_first": null,
                "superseded": [], "superseded_count": 0, "later_count": 0,
                "later_this_count": 0, "later_this_first": null,
            },
        });
    };
    let field = |key: &str| row.get(key).cloned().unwrap_or(Value::Null);
    let recounted = row.get("recounted").and_then(Value::as_bool) == Some(true);
    // THE CLOSE'S OWN WINDOW: from its run's start to the start of the next
    // run that closed the same kind-day, under any filter (AF-02 repair 2).
    let next_s = row.get("run").and_then(Value::as_u64).and_then(|id| {
        let ids = rows.closers.get(&day.to_string())?;
        super::imports::next_start(
            start_of(&rows.runs, id),
            ids.iter().map(|other| start_of(&rows.runs, *other)),
        )
    });
    let refusals = refusals_of(row, inst, day, &rows.runs, next_s);
    // A DAY THE LOG ALONE NAMES, AND NOTHING IN IT ABOUT THIS INSTRUMENT:
    // a run refused other files of the day and left no journal line (DB-025's
    // shape), and nothing says this instrument's day was refused. It reads
    // `none`, from the log, with the other files' refusals beside it (AF-02,
    // ingest-6: a refusal about another underlying never marks this one
    // refused). A day-level refusal (the calendar, the census) is about every
    // instrument (`about_this`) and keeps `refused`.
    let others_only =
        field("outcome_from") == "log" && refusals.this_count == 0 && refusals.other_count > 0;
    let outcome = if recounted {
        Value::from("recounted")
    } else if others_only {
        Value::from("none")
    } else {
        field("outcome")
    };
    let pair = |(about, why): &(String, String)| json!({ "about": about, "why": why });
    let unless_others_only = |key: &str| {
        if others_only { Value::Null } else { field(key) }
    };
    json!({
        "outcome": outcome,
        "reached": true,
        "stage": unless_others_only("stage"),
        "reason": unless_others_only("reason"),
        "files": field("files"),
        "failures": unless_others_only("failures"),
        "run": field("run"),
        "from": field("outcome_from"),
        "line": field("line"),
        "late": field("late"),
        "max_back_s": field("max_back_s"),
        "refusals": {
            "this": refusals.this.iter().map(pair).collect::<Vec<_>>(),
            "this_count": refusals.this_count,
            "other_count": refusals.other_count,
            "other_first": refusals.other_first.as_ref().map(pair),
            "superseded": refusals.superseded.iter().map(|old| json!({
                "run": old.run,
                "commit": old.commit,
                "at_ms": old.at_ms,
                "about": old.about,
                "why": old.why,
            })).collect::<Vec<_>>(),
            "superseded_count": refusals.superseded_count,
            "later_count": refusals.later_count,
            "later_this_count": refusals.later_this_count,
            "later_this_first": refusals.later_this_first.as_ref().map(pair),
        },
    })
}

// ══ Append-only ledgers, read by their tails ════════════════════════════════

/// One append-only ledger's parse, folded a line at a time. D-3317's line
/// rules: one event a line, fields after the key `name=value`, the last line
/// of a key wins.
trait Ledger: Default + Clone {
    /// Folds one whole line in. `number` counts from 1.
    fn line(&mut self, number: u64, text: &str);
}

/// What a ledger read keeps between calls for one path.
struct Tail<L> {
    /// The file's device and inode when read: a replaced file is read again.
    identity: Option<(u64, u64)>,
    /// Bytes parsed, through the last newline.
    upto: u64,
    /// Whole lines parsed.
    lines: u64,
    parsed: Arc<L>,
}

/// One read of a ledger: what it holds, and how the file stood.
struct LedgerRead<L> {
    path: PathBuf,
    /// An absent ledger is one its writer has not written yet: a fact.
    present: bool,
    bytes: u64,
    /// Bytes after the last newline: a line being written. Never parsed.
    unterminated: u64,
    /// Bytes this call read: the whole file the first time, the new bytes
    /// after that.
    read_now: u64,
    lines: u64,
    parsed: Arc<L>,
}

/// Every path read so far, per ledger kind. Keyed by path, so a test's store
/// and the served store never share a parse.
type Tails<L> = Mutex<HashMap<PathBuf, Tail<L>>>;

/// The GDFL day ledgers read so far.
fn quality_tails() -> &'static Tails<QualityLedger> {
    static TAILS: OnceLock<Tails<QualityLedger>> = OnceLock::new();
    TAILS.get_or_init(|| Mutex::new(HashMap::with_capacity(4)))
}

/// The Zerodha gap ledgers read so far.
fn gap_tails() -> &'static Tails<GapLedger> {
    static TAILS: OnceLock<Tails<GapLedger>> = OnceLock::new();
    TAILS.get_or_init(|| Mutex::new(HashMap::with_capacity(4)))
}

/// `<store>/imports/gdfl.dayflags`, beside the import journal (D-3335 §1).
fn quality_path(root: &Path) -> PathBuf {
    pull::gdfl_import::journal_path(root).with_file_name("gdfl.dayflags")
}

/// `<store>/audit/zerodha.gaps` (D-3335 §2).
fn gaps_path(root: &Path) -> PathBuf {
    root.join("audit")
        .join(format!("{}.gaps", Vendor::Zerodha.as_str()))
}

/// The position just past the last newline in `bytes`, if one is there.
fn whole_lines_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, byte)| (*byte == b'\n').then_some(at))
}

/// The ledger at `path`, read through its tail: one `metadata` call, and the
/// bytes appended since the last read. A file that shrank or was replaced is
/// read again from the start.
///
/// # Errors
///
/// A file that is there and cannot be measured or read, in the failure's own
/// words. An absent file is not an error.
fn read_ledger<L: Ledger>(tails: &Tails<L>, path: &Path) -> Result<LedgerRead<L>, String> {
    use std::io::{Read as _, Seek as _};
    use std::os::unix::fs::MetadataExt as _;
    let meta = match std::fs::metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LedgerRead {
                path: path.to_path_buf(),
                present: false,
                bytes: 0,
                unterminated: 0,
                read_now: 0,
                lines: 0,
                parsed: Arc::new(L::default()),
            });
        }
        Err(e) => return Err(format!("{}: {e}", path.display())),
        Ok(meta) => meta,
    };
    let identity = Some((meta.dev(), meta.ino()));
    let len = meta.len();
    let mut held = tails.lock().unwrap_or_else(PoisonError::into_inner);
    let fresh_tail = || Tail {
        identity,
        upto: 0,
        lines: 0,
        parsed: Arc::new(L::default()),
    };
    let tail = held.entry(path.to_path_buf()).or_insert_with(fresh_tail);
    if tail.identity != identity || len < tail.upto {
        *tail = fresh_tail();
    }
    let mut read_now = 0u64;
    if len > tail.upto {
        let fail = |e: std::io::Error| format!("{}: {e}", path.display());
        let mut file = std::fs::File::open(path).map_err(fail)?;
        file.seek(std::io::SeekFrom::Start(tail.upto))
            .map_err(fail)?;
        let mut fresh = Vec::new();
        file.take(len.saturating_sub(tail.upto))
            .read_to_end(&mut fresh)
            .map_err(fail)?;
        read_now = u64::try_from(fresh.len()).unwrap_or(u64::MAX);
        if let Some(end) = whole_lines_end(&fresh) {
            let parsed = Arc::make_mut(&mut tail.parsed);
            for line in fresh.get(..end).unwrap_or_default().split(|&b| b == b'\n') {
                tail.lines = tail.lines.saturating_add(1);
                // NOT UTF-8 IS NOT A LINE ANY LEDGER WRITES: decoded lossily
                // so the parser counts it as foreign and quotes it.
                parsed.line(tail.lines, &String::from_utf8_lossy(line));
            }
            tail.upto = tail
                .upto
                .saturating_add(u64::try_from(end).unwrap_or(u64::MAX))
                .saturating_add(1);
        }
    }
    Ok(LedgerRead {
        path: path.to_path_buf(),
        present: true,
        bytes: len,
        unterminated: len.saturating_sub(tail.upto),
        read_now,
        lines: tail.lines,
        parsed: Arc::clone(&tail.parsed),
    })
}

/// A count as a ledger writes one: decimal, no sign, no leading zero but `0`.
fn canonical_count(text: &str) -> Option<u64> {
    let value: u64 = text.parse().ok()?;
    (value.to_string() == text).then_some(value)
}

/// `HH:MM:SS` (or `HH:MM` when `seconds` is false) as seconds of the day,
/// `Ok(None)` for `-`, or an error for anything else.
fn clock(text: &str, seconds: bool) -> Result<Option<u32>, &'static str> {
    let refused = if seconds {
        "not HH:MM:SS or -"
    } else {
        "not HH:MM or -"
    };
    if text == "-" {
        return Ok(None);
    }
    let parts: Vec<&str> = text.split(':').collect();
    let want = if seconds { 3 } else { 2 };
    if parts.len() != want || parts.iter().any(|part| part.len() != 2) {
        return Err(refused);
    }
    let number = |at: usize, below: u32| {
        parts
            .get(at)
            .and_then(|part| part.parse::<u32>().ok())
            .filter(|n| *n < below)
            .ok_or(refused)
    };
    let hours = number(0, 24)?;
    let minutes = number(1, 60)?;
    let secs = if seconds { number(2, 60)? } else { 0 };
    Ok(Some(hours * 3_600 + minutes * 60 + secs))
}

/// Seconds of the day as `HH:MM:SS`.
fn hms(seconds: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3_600,
        seconds % 3_600 / 60,
        seconds % 60
    )
}

/// Minutes of the day as `HH:MM`.
fn hm(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// The faults one ledger read found: the first [`MAX_NAMED_FAULTS`] by line
/// number, with their first bytes and the reason, and all of them counted.
#[derive(Debug, Clone, Default)]
struct Faults {
    named: Vec<(u64, String, String)>,
    count: u64,
}

impl Faults {
    /// Notes one line this reader cannot name.
    fn note(&mut self, number: u64, text: &str, why: &str) {
        self.count = self.count.saturating_add(1);
        if self.named.len() < MAX_NAMED_FAULTS {
            let shown: String = text.chars().take(80).collect();
            self.named.push((number, shown, why.to_owned()));
        }
    }

    fn json(&self) -> Value {
        json!({
            "count": self.count,
            "lines": self
                .named
                .iter()
                .map(|(line, text, why)| json!({ "line": line, "text": text, "why": why }))
                .collect::<Vec<_>>(),
        })
    }
}

/// A field's shape in the GDFL day ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// A decimal count.
    Count,
    /// `HH:MM:SS`, or `-` when there is none.
    Clock,
}

/// The fields every kind's line carries (D-3335 §1).
const EVERY_KIND: [(&str, Shape); 5] = [
    ("member_bytes", Shape::Count),
    ("late", Shape::Count),
    ("max_back_s", Shape::Count),
    ("deferred_s", Shape::Count),
    ("rows_refused", Shape::Count),
];

/// An index line's own fields.
const INDICES_FIELDS: [(&str, Shape); 10] = [
    ("rows", Shape::Count),
    ("seconds", Shape::Count),
    ("session_s", Shape::Count),
    ("ss_distinct", Shape::Count),
    ("holes_5s", Shape::Count),
    ("hole_max_s", Shape::Count),
    ("hole_max_at", Shape::Clock),
    ("dual_spread_s", Shape::Count),
    ("dual_spread_max_bps", Shape::Count),
    ("zero_ltp_rows", Shape::Count),
];

/// A stocks line's own fields.
const STOCKS_FIELDS: [(&str, Shape); 8] = [
    ("files", Shape::Count),
    ("files_traded", Shape::Count),
    ("first_trade_p50", Shape::Clock),
    ("shared_holes", Shape::Count),
    ("shared_hole_max_s", Shape::Count),
    ("shared_hole_max_at", Shape::Clock),
    ("ltq0_new_ltp_rows", Shape::Count),
    ("auction_prints", Shape::Count),
];

/// An options line's own fields.
const OPTIONS_FIELDS: [(&str, Shape); 7] = [
    ("files", Shape::Count),
    ("files_traded", Shape::Count),
    ("zero_quote_files", Shape::Count),
    ("ltq1_lead_files", Shape::Count),
    ("ltq1_lead_rows", Shape::Count),
    ("ltq0_new_ltp_rows", Shape::Count),
    ("first_trade_p50", Shape::Clock),
];

/// The fields a later writer added to every kind's line (D-4928; WP-11c): the
/// in-session seconds the calendar of record halts for the kind's segment,
/// and the placed seconds inside them. **Optional**: a line written before
/// them stays valid, and a missing one is `null`, never `0`.
const LATER_EVERY: [(&str, Shape); 2] = [("halt_s", Shape::Count), ("halt_held_s", Shape::Count)];

/// An index line's later fields (D-4928; WP-11c): the longest run of
/// in-session rows carrying one unchanged value, and its first stamp.
/// Optional, as above.
const LATER_INDICES: [(&str, Shape); 2] =
    [("flat_max_s", Shape::Count), ("flat_max_at", Shape::Clock)];

/// A kind's later, optional fields.
fn later_fields(kind: ImportKind) -> Vec<(&'static str, Shape)> {
    let own: &[(&str, Shape)] = if kind == ImportKind::Indices {
        &LATER_INDICES
    } else {
        &[]
    };
    LATER_EVERY.iter().chain(own).copied().collect()
}

/// A kind's own fields.
const fn own_fields(kind: ImportKind) -> &'static [(&'static str, Shape)] {
    match kind {
        ImportKind::Indices => &INDICES_FIELDS,
        ImportKind::Stocks => &STOCKS_FIELDS,
        ImportKind::Options => &OPTIONS_FIELDS,
    }
}

/// A kind as a sortable key part, in import order.
const fn kind_slot(kind: ImportKind) -> u8 {
    match kind {
        ImportKind::Indices => 0,
        ImportKind::Stocks => 1,
        ImportKind::Options => 2,
    }
}

/// One field's value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldValue {
    Count(u64),
    /// Seconds of the day, or `None` for `-`.
    Clock(Option<u32>),
}

impl FieldValue {
    fn json(self) -> Value {
        match self {
            Self::Count(n) => Value::from(n),
            Self::Clock(Some(at)) => Value::from(hms(at)),
            Self::Clock(None) => Value::Null,
        }
    }
}

/// One `flags` line of the GDFL day ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
struct QualityLine {
    kind: ImportKind,
    day: Day,
    scope: String,
    src: String,
    definition: u64,
    /// Every field of the kind, the five every kind carries first, in
    /// D-3335's order.
    fields: Vec<(&'static str, FieldValue)>,
    /// The kind's later fields ([`later_fields`]), `None` where the line was
    /// written before them.
    later: Vec<(&'static str, Option<FieldValue>)>,
    line: u64,
}

impl QualityLine {
    /// Every field the line carries, the later ones it has among them.
    fn carried(&self) -> impl Iterator<Item = (&'static str, FieldValue)> + '_ {
        self.fields.iter().copied().chain(
            self.later
                .iter()
                .filter_map(|(name, value)| value.map(|value| (*name, value))),
        )
    }

    /// A count field's value; `None` for a later field the line lacks.
    fn count(&self, name: &str) -> Option<u64> {
        self.carried().find_map(|(field, value)| match value {
            FieldValue::Count(n) if field == name => Some(n),
            _ => None,
        })
    }

    /// A clock field's value: seconds of the day, `None` for `-` or absent.
    fn clock(&self, name: &str) -> Option<u32> {
        self.carried().find_map(|(field, value)| match value {
            FieldValue::Clock(at) if field == name => at,
            _ => None,
        })
    }

    fn json(&self) -> Value {
        let mut fields = serde_json::Map::with_capacity(self.fields.len().saturating_add(4));
        fields.insert("scope".to_owned(), Value::from(self.scope.clone()));
        fields.insert("src".to_owned(), Value::from(self.src.clone()));
        fields.insert("definition".to_owned(), Value::from(self.definition));
        fields.insert("line".to_owned(), Value::from(self.line));
        for (name, value) in &self.fields {
            fields.insert((*name).to_owned(), value.json());
        }
        // A LATER FIELD THE LINE LACKS IS `null`, never 0: the key is always
        // there, and a line written before the field says nothing about it.
        for (name, value) in &self.later {
            fields.insert(
                (*name).to_owned(),
                value.map_or(Value::Null, FieldValue::json),
            );
        }
        Value::Object(fields)
    }
}

/// The GDFL day ledger: the last line of each `(day, kind, scope)`.
#[derive(Debug, Clone, Default)]
struct QualityLedger {
    rows: BTreeMap<(u32, u8, String), QualityLine>,
    faults: Faults,
}

impl QualityLedger {
    /// The line for one day, kind and scope. O(log n).
    fn get(&self, day: Day, kind: ImportKind, scope: &str) -> Option<&QualityLine> {
        self.rows
            .get(&(day.days_from_epoch(), kind_slot(kind), scope.to_owned()))
    }
}

impl Ledger for QualityLedger {
    fn line(&mut self, number: u64, text: &str) {
        if text.is_empty() {
            return;
        }
        match parse_quality(number, text) {
            Ok(line) => {
                let key = (
                    line.day.days_from_epoch(),
                    kind_slot(line.kind),
                    line.scope.clone(),
                );
                self.rows.insert(key, line);
            }
            Err(why) => self.faults.note(number, text, &why),
        }
    }
}

/// One `flags <kind> <day> <scope> v=1 src=<import|rescan> definition=<n>
/// <fields>` line, every field of its kind present once, or why not.
fn parse_quality(number: u64, text: &str) -> Result<QualityLine, String> {
    let mut words = text.split(' ');
    if words.next() != Some("flags") {
        return Err("not a flags line".to_owned());
    }
    let kind_word = words.next().unwrap_or_default();
    let kind = ImportKind::parse(kind_word).map_err(|_| format!("{kind_word:?} is not a kind"))?;
    let day_word = words.next().unwrap_or_default();
    let day = super::import_journal::parse_day(day_word)
        .ok_or_else(|| format!("{day_word:?} is not a day as YYYY-MM-DD"))?;
    let scope = words.next().unwrap_or_default();
    let scope_ok = match kind {
        ImportKind::Indices => !scope.is_empty() && scope != "*" && !scope.contains('='),
        ImportKind::Stocks | ImportKind::Options => scope == "*",
    };
    if !scope_ok {
        return Err(format!(
            "scope {scope:?} is not an index symbol for indices or `*` for stocks and options"
        ));
    }
    let spec: Vec<(&'static str, Shape)> =
        EVERY_KIND.iter().chain(own_fields(kind)).copied().collect();
    let mut found: Vec<Option<FieldValue>> = vec![None; spec.len()];
    let later_spec = later_fields(kind);
    let mut later: Vec<Option<FieldValue>> = vec![None; later_spec.len()];
    let (mut version, mut src, mut definition) = (None, None, None);
    for word in words {
        let (name, value) = word
            .split_once('=')
            .ok_or_else(|| format!("{word:?} is not name=value"))?;
        match name {
            "v" => version = Some(value),
            "src" => src = Some(value),
            "definition" => {
                definition = Some(
                    canonical_count(value)
                        .ok_or_else(|| format!("definition={value:?} is not a count"))?,
                );
            }
            _ => {
                let slots = found.iter_mut().chain(later.iter_mut());
                for ((field, shape), slot) in spec.iter().chain(&later_spec).zip(slots) {
                    if *field != name {
                        continue;
                    }
                    let read = match shape {
                        Shape::Count => canonical_count(value).map(FieldValue::Count),
                        Shape::Clock => clock(value, true).ok().map(FieldValue::Clock),
                    }
                    .ok_or_else(|| format!("{name}={value:?} is not a {shape:?}"))?;
                    if slot.replace(read).is_some() {
                        return Err(format!("{name} appears twice"));
                    }
                }
                // A FIELD A LATER WRITER ADDS IS KEPT OUT OF THE RECORD, not a
                // reason to drop the line: the version says what this reader
                // reads, and every field it needs is checked below.
            }
        }
    }
    if version != Some("1") {
        return Err(format!(
            "v={} is not 1, the version this reader reads",
            version.unwrap_or("(none)")
        ));
    }
    let src = src
        .filter(|s| matches!(*s, "import" | "rescan"))
        .ok_or("src is not import or rescan".to_owned())?;
    let definition = definition.ok_or("no definition=".to_owned())?;
    let mut fields = Vec::with_capacity(spec.len());
    for ((name, _), value) in spec.iter().zip(found) {
        fields.push((*name, value.ok_or_else(|| format!("{name} is missing"))?));
    }
    Ok(QualityLine {
        kind,
        day,
        scope: scope.to_owned(),
        src: src.to_owned(),
        definition,
        fields,
        later: later_spec
            .iter()
            .map(|(name, _)| *name)
            .zip(later)
            .collect(),
        line: number,
    })
}

/// One `gap` line of the Zerodha ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GapLine {
    rung: String,
    owed: u64,
    held: u64,
    missing: u64,
    /// Minutes of the day.
    first_missing: Option<u32>,
    runs: u64,
    longest: u64,
    /// Minutes of the day.
    longest_at: Option<u32>,
    why: String,
    run: u64,
    line: u64,
    /// Whether a missing minute may be one nobody traded (D-3321): the line
    /// says `absence=may_be_no_trade`, or its key names a segment where a
    /// minute has a bar only when somebody trades
    /// (`pull::ingest::repair_plan::absence_may_be_no_trade`, the fold's one
    /// rule). The store cannot tell such a minute from a vendor hole.
    may_be_no_trade: bool,
}

impl GapLine {
    fn json(&self) -> Value {
        json!({
            "feed": Vendor::Zerodha.as_str(),
            "rung": self.rung,
            "owed": self.owed,
            "held": self.held,
            "missing": self.missing,
            "first_missing": self.first_missing.map(|m| hm(m / 60)),
            "runs": self.runs,
            "longest": self.longest,
            "longest_at": self.longest_at.map(|m| hm(m / 60)),
            "why": self.why,
            "run": self.run,
            "line": self.line,
            "absence": self.may_be_no_trade.then_some("may_be_no_trade"),
        })
    }
}

/// One `derive` line of the Zerodha ledger (D-3321).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeriveLine {
    month: YearMonth,
    days: u64,
    buckets: u64,
    withheld: u64,
    partial: u64,
    run: u64,
    line: u64,
}

/// The Zerodha ledger: the last `gap` line of each `(key, day)` and the last
/// `derive` line of each `(key, month, rung)`.
#[derive(Debug, Clone, Default)]
struct GapLedger {
    gaps: BTreeMap<(String, u32), GapLine>,
    derives: BTreeMap<(String, u32, String), DeriveLine>,
    faults: Faults,
}

impl GapLedger {
    /// The gap line for one instrument and day. O(log n).
    fn get(&self, key: &str, day: Day) -> Option<&GapLine> {
        self.gaps.get(&(key.to_owned(), day.days_from_epoch()))
    }
}

impl Ledger for GapLedger {
    fn line(&mut self, number: u64, text: &str) {
        if text.is_empty() {
            return;
        }
        let parsed = if text.starts_with("gap ") {
            parse_gap(number, text).map(|(key, day, line)| {
                self.gaps.insert((key, day), line);
            })
        } else if text.starts_with("derive ") {
            parse_derive(number, text).map(|(key, rung, line)| {
                let month = month_index(line.month);
                self.derives.insert((key, month, rung), line);
            })
        } else {
            Err("not a gap or derive line".to_owned())
        };
        if let Err(why) = parsed {
            self.faults.note(number, text, &why);
        }
    }
}

/// A month as one sortable number.
fn month_index(month: YearMonth) -> u32 {
    u32::from(month.year())
        .saturating_mul(12)
        .saturating_add(u32::from(month.month()))
}

/// The `name=value` fields after a ledger line's key, every name in `want`
/// present exactly once and every other name ignored.
fn named_fields<'t>(
    words: impl Iterator<Item = &'t str>,
    want: &[&str],
) -> Result<Vec<&'t str>, String> {
    let mut found: Vec<Option<&str>> = vec![None; want.len()];
    for word in words {
        let (name, value) = word
            .split_once('=')
            .ok_or_else(|| format!("{word:?} is not name=value"))?;
        for (at, field) in want.iter().enumerate() {
            if *field == name
                && let Some(slot) = found.get_mut(at)
                && slot.replace(value).is_some()
            {
                return Err(format!("{name} appears twice"));
            }
        }
    }
    want.iter()
        .zip(found)
        .map(|(name, value)| value.ok_or_else(|| format!("{name} is missing")))
        .collect()
}

/// A count field, or why not.
fn count_field(name: &str, value: &str) -> Result<u64, String> {
    canonical_count(value).ok_or_else(|| format!("{name}={value:?} is not a count"))
}

/// One `gap <key> <day> v=1 rung= owed= held= missing= first_missing= runs=
/// longest= longest_at= why= run=` line, or why not.
fn parse_gap(number: u64, text: &str) -> Result<(String, u32, GapLine), String> {
    let mut words = text.split(' ').skip(1);
    let key = words
        .next()
        .filter(|key| !key.is_empty() && !key.contains('='))
        .ok_or("no instrument key")?;
    let day_word = words.next().unwrap_or_default();
    let day = super::import_journal::parse_day(day_word)
        .ok_or_else(|| format!("{day_word:?} is not a day as YYYY-MM-DD"))?;
    let want = [
        "v",
        "rung",
        "owed",
        "held",
        "missing",
        "first_missing",
        "runs",
        "longest",
        "longest_at",
        "why",
        "run",
    ];
    let got = named_fields(words, &want)?;
    let value = |at: usize| got.get(at).copied().unwrap_or_default();
    if value(0) != "1" {
        return Err(format!(
            "v={} is not 1, the version this reader reads",
            value(0)
        ));
    }
    let minute = |at: usize| {
        clock(value(at), false).map_err(|why| {
            format!(
                "{}={:?} is {why}",
                want.get(at).copied().unwrap_or_default(),
                value(at)
            )
        })
    };
    let why = value(9);
    if why.is_empty() || !why.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
        return Err(format!("why={why:?} is not a snake-case code"));
    }
    let line = GapLine {
        rung: value(1).to_owned(),
        owed: count_field("owed", value(2))?,
        held: count_field("held", value(3))?,
        missing: count_field("missing", value(4))?,
        first_missing: minute(5)?,
        runs: count_field("runs", value(6))?,
        longest: count_field("longest", value(7))?,
        longest_at: minute(8)?,
        why: why.to_owned(),
        run: count_field("run", value(10))?,
        line: number,
        may_be_no_trade: text
            .split(' ')
            .any(|word| word == "absence=may_be_no_trade")
            || key
                .split('-')
                .nth(1)
                .is_some_and(pull::ingest::repair_plan::absence_may_be_no_trade),
    };
    Ok((key.to_owned(), day.days_from_epoch(), line))
}

/// One `derive <key> <rung> <YYYY-MM> v=1 days= buckets= withheld= partial=
/// run=` line, or why not.
fn parse_derive(number: u64, text: &str) -> Result<(String, String, DeriveLine), String> {
    let mut words = text.split(' ').skip(1);
    let key = words
        .next()
        .filter(|key| !key.is_empty() && !key.contains('='))
        .ok_or("no instrument key")?;
    let rung = words
        .next()
        .filter(|rung| Timeframe::KNOWN.iter().any(|known| known.as_str() == *rung))
        .ok_or("no rung this store files")?;
    let month_word = words.next().unwrap_or_default();
    let month = month_word
        .split_once('-')
        .filter(|(year, month)| year.len() == 4 && month.len() == 2)
        .and_then(|(year, month)| YearMonth::new(year.parse().ok()?, month.parse().ok()?).ok())
        .ok_or_else(|| format!("{month_word:?} is not a month as YYYY-MM"))?;
    let want = ["v", "days", "buckets", "withheld", "partial", "run"];
    let got = named_fields(words, &want)?;
    let value = |at: usize| got.get(at).copied().unwrap_or_default();
    if value(0) != "1" {
        return Err(format!(
            "v={} is not 1, the version this reader reads",
            value(0)
        ));
    }
    let line = DeriveLine {
        month,
        days: count_field("days", value(1))?,
        buckets: count_field("buckets", value(2))?,
        withheld: count_field("withheld", value(3))?,
        partial: count_field("partial", value(4))?,
        run: count_field("run", value(5))?,
        line: number,
    };
    Ok((key.to_owned(), rung.to_owned(), line))
}

/// The derive lines for `inst` in the months `first..=last` touches, oldest
/// first. O(log n + lines in those months).
fn derive_json(
    gaps: Option<&LedgerRead<GapLedger>>,
    inst: &Instrument,
    first: Day,
    last: Day,
) -> Value {
    let (Some(gaps), Ok(from), Ok(to)) = (gaps, first.year_month(), last.year_month()) else {
        return Value::Null;
    };
    let key = inst.key();
    let low = (key.clone(), month_index(from), String::new());
    let high = (key, month_index(to).saturating_add(1), String::new());
    let lines: Vec<Value> = gaps
        .parsed
        .derives
        .range(low..high)
        .map(|((_, _, rung), line)| {
            json!({
                "rung": rung,
                "month": format!("{:04}-{:02}", line.month.year(), line.month.month()),
                "days": line.days,
                "buckets": line.buckets,
                "withheld": line.withheld,
                "partial": line.partial,
                "run": line.run,
                "line": line.line,
            })
        })
        .collect();
    Value::from(lines)
}

// ══ The Zerodha pull journal's refusals ═════════════════════════════════════
//
// D-3335 puts a refused front in the Zerodha ledger (`why=refused_front`), and
// no writer writes that line yet: WP-13b's minute audit writes the days it
// finds short, never a `refused_front` line. The one durable record of a
// refused front on the operator's store is the pull journal
// (`audit/pull.journal`): the member failures NIFTYOILANDGAS's 1day pulls
// appended, "conflicting vendor candles at 1578853800; refused before append"
// (D-3315; records 15, 16 and 28 of 4,217,027, read 9 Oct 2026). The same
// journal keeps the rows a pull refused on a dated day: a Muhurat of unmeasured
// length (D-3344, "<day>: 60 minute row(s) refused: ..."), and a timestamp the
// fetch refused ("row 7499: timestamp 1605357900 refused -- ..."), 1,573
// records on that store, every one on a Muhurat. So this reads those records,
// by the journal's own decoder, beside the ledger -- never instead of it.
//
// A record is attributed to the run before it, as `/audit.json` attributes it
// (`audit_json::vendor_of_source`), and only Zerodha's are kept.

// The note prefixes of a Zerodha member whose WHOLE window was refused before
// append (`pull::ingest`): nothing of the window landed. Matched on the
// prefix, because the journal cuts a note at 60 bytes and keeps its start.
// Kept in `audit` since WP-33c, so the journal's index files the same records
// this reader keeps.
use crate::audit::WHOLE_WINDOW_NOTES;

/// The most refusals one record names; the rest are counted.
const MAX_PULL_NAMED: usize = 4;

/// How many months either side of a day a refused front's held months are
/// looked for: twelve years, the whole span of a Zerodha `1day` series in this
/// store (2015-01..2026-09) and more. Bounded, so a page costs at most this
/// many `metadata` calls per refused span it shows, and only then.
const FRONT_PROBE_MONTHS: u32 = 144;

/// One member failure of the pull journal that names a refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PullRefusal {
    /// The record's number in the journal, from 1.
    record: u64,
    /// When the record was written, epoch seconds.
    at_unix_secs: i64,
    /// The member's window, days from the epoch.
    from_days: u32,
    to_days: u32,
    /// The note, as journalled; `cut` when the journal kept only its start.
    note: String,
    cut: bool,
    /// The IST day the note names, when it names one: `YYYY-MM-DD: ...`, a
    /// `timestamp <epoch>` or a conflicting candle's `at <epoch>`.
    names_day: Option<u32>,
    /// The member's whole window was refused before append.
    whole_window: bool,
}

impl PullRefusal {
    fn json(&self) -> Value {
        let day =
            |days: u32| Day::from_days(days).map_or_else(|_| days.to_string(), |d| d.to_string());
        json!({
            "record": self.record,
            "at_unix_secs": self.at_unix_secs,
            "window": { "from": day(self.from_days), "to": day(self.to_days) },
            "note": self.note,
            "cut": self.cut,
            "names_day": self.names_day.map(day),
            "whole_window": self.whole_window,
        })
    }

    /// The note as a phrase quotes it: marked when the journal kept only its
    /// start.
    fn shown(&self) -> String {
        if self.cut {
            format!("{}...", self.note)
        } else {
            self.note.clone()
        }
    }

    /// The IST day the record was written, as `YYYY-MM-DD`.
    fn run_day(&self) -> String {
        ist_day_of(self.at_unix_secs)
            .and_then(|days| Day::from_days(days).ok())
            .map_or_else(|| self.at_unix_secs.to_string(), |day| day.to_string())
    }

    /// Whether this refusal is about `day`: it names the day, or it refused a
    /// whole window holding it.
    fn about(&self, day: u32) -> bool {
        if self.whole_window {
            self.from_days <= day && day <= self.to_days
        } else {
            self.names_day == Some(day)
        }
    }
}

/// The Zerodha refusals of one pull journal, by the member's source (the
/// store symbol it pulled).
#[derive(Debug, Clone, Default)]
struct PullRefusals {
    by_symbol: HashMap<String, Vec<PullRefusal>>,
    /// Records that would not decode, or members no run before them names.
    faults: Faults,
    /// Member failures read whose run is not Zerodha's (or names no vendor).
    other_vendor: u64,
}

/// What one pull-journal read keeps between calls for one path.
struct PullTail {
    identity: Option<(u64, u64)>,
    /// Bytes parsed: a whole number of records.
    upto: u64,
    /// The vendor the last run record named: its member failures follow it.
    vendor: Option<Vendor>,
    parsed: Arc<PullRefusals>,
    /// When the served process began walking this journal at start, while it
    /// walks ([`warm_pull_journal`]). A request that finds it set and records
    /// to read is answered without them rather than walking beside it.
    warming: Option<std::time::Instant>,
}

impl PullTail {
    /// Nothing read yet of the journal whose `stat` gave `identity`.
    fn fresh(identity: Option<(u64, u64)>) -> Self {
        Self {
            identity,
            upto: 0,
            vendor: None,
            parsed: Arc::new(PullRefusals::default()),
            warming: None,
        }
    }

    /// Read the records past `upto` up to `whole` bytes from `file`, folding
    /// each into `parsed`, in 4 MiB reads. The records read.
    ///
    /// # Errors
    ///
    /// A read that failed, in its own words.
    fn advance(&mut self, path: &Path, whole: u64) -> Result<u64, String> {
        use std::io::{Read as _, Seek as _};
        let stride = crate::audit::RECORD_LEN_U64;
        let mut read_now = 0u64;
        if whole <= self.upto {
            return Ok(0);
        }
        let fail = |e: std::io::Error| format!("{}: {e}", path.display());
        let mut file = std::fs::File::open(path).map_err(fail)?;
        file.seek(std::io::SeekFrom::Start(self.upto))
            .map_err(fail)?;
        let parsed = Arc::make_mut(&mut self.parsed);
        let chunk_records: u64 = 16_384;
        let mut buffer = Vec::new();
        while self.upto < whole {
            let want = whole.saturating_sub(self.upto).min(chunk_records * stride);
            buffer.clear();
            (&mut file)
                .take(want)
                .read_to_end(&mut buffer)
                .map_err(fail)?;
            if buffer.is_empty() {
                break;
            }
            let first = self.upto / stride;
            for (at, image) in buffer.chunks_exact(crate::audit::RECORD_LEN).enumerate() {
                let number = first
                    .saturating_add(u64::try_from(at).unwrap_or(u64::MAX))
                    .saturating_add(1);
                pull_record(parsed, &mut self.vendor, number, image);
            }
            let got = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
            let kept = got - got % stride;
            self.upto = self.upto.saturating_add(kept);
            read_now = read_now.saturating_add(kept / stride);
            if kept < got {
                break;
            }
        }
        Ok(read_now)
    }
}

/// Why a request's read of the pull journal gave no records.
#[derive(Debug)]
enum PullUnread {
    /// It is there and could not be measured or read, in its own words.
    Failed(String),
    /// The served process is walking it at start ([`warm_pull_journal`]),
    /// and a request does not walk it beside it (DB-N19).
    Warming {
        /// The journal.
        path: PathBuf,
        /// How long ago the walk began.
        since: std::time::Duration,
        /// Whole records the journal holds.
        records: u64,
    },
}

impl PullUnread {
    /// The sentence an answer's notes give.
    fn note(&self) -> String {
        match self {
            Self::Failed(why) => format!(
                "The Zerodha pull journal could not be read, so no record names a pull refusal \
                 and refused_front is judged from the gap ledger alone: {why}"
            ),
            Self::Warming {
                path,
                since,
                records,
            } => format!(
                "The Zerodha pull journal at {} is being walked by this server's start \
                 ({records} record(s), begun {} ms ago; the journal keeps no index by \
                 instrument, so its first read walks it whole), and this answer did not wait \
                 for it: no record names a pull refusal yet and refused_front is judged from \
                 the gap ledger alone. Ask again in a moment.",
                path.display(),
                since.as_millis()
            ),
        }
    }

    /// Its `sources.pull_journal` entry.
    fn json(&self) -> Value {
        match self {
            Self::Failed(why) => json!({ "present": null, "error": why }),
            Self::Warming {
                path,
                since,
                records,
            } => json!({
                "path": path.display().to_string(),
                "present": null,
                "state": "walking at start",
                "since_ms": u64::try_from(since.as_millis()).unwrap_or(u64::MAX),
                "records": records,
                "error": null,
            }),
        }
    }
}

/// One read of the pull journal.
struct PullRead {
    path: PathBuf,
    present: bool,
    bytes: u64,
    records: u64,
    /// Bytes past the last whole record: a record being written, or torn.
    torn: u64,
    /// Records this call read: every one the first time, the new ones after.
    read_now: u64,
    /// How long this call's read took, in microseconds: the first read of a
    /// long journal is the one non-constant cost of this route, so it is said.
    read_micros: u64,
    parsed: Arc<PullRefusals>,
    /// How the refusals were found.
    via: Via,
}

/// How one pull-journal read found its refusals (WP-33c).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Via {
    /// No journal.
    Absent,
    /// Through the journal's index, which covered this many records and held
    /// this many runs; only the asked instrument's refusals were read.
    Index { covered: u64, runs: u64 },
    /// By a walk of the journal, every instrument's refusals, because the
    /// index could not be used, for this reason.
    Walk { why: String },
}

/// The pull journals read so far.
fn pull_tails() -> &'static Mutex<HashMap<PathBuf, PullTail>> {
    static TAILS: OnceLock<Mutex<HashMap<PathBuf, PullTail>>> = OnceLock::new();
    TAILS.get_or_init(|| Mutex::new(HashMap::with_capacity(4)))
}

/// Whether `needle` is inside `haystack`.
fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// The byte of a pull-journal record that holds its kind. `audit.rs` keeps the
/// layout private; both versions put the kind here and a version is never
/// re-laid (`CLAUDE.md` §3 rule 8), and
/// `the_pull_journal_offsets_are_the_records_own` pins it to
/// [`crate::audit::Record::image`].
const PULL_KIND_AT: usize = 118;

/// The bytes either record version keeps the note in: v1's from 184, v2's
/// from 192, both ending at the checksum (252). Pinned by the same test.
const PULL_NOTE_BYTES: std::ops::Range<usize> = 184..252;

/// Whether a member failure's note may name a refusal, before decoding it.
fn may_refuse(image: &[u8]) -> bool {
    let Some(note) = image.get(PULL_NOTE_BYTES) else {
        return false;
    };
    holds(note, b"refused")
        || WHOLE_WINDOW_NOTES
            .iter()
            .any(|prefix| holds(note, prefix.as_bytes().get(..16).unwrap_or_default()))
}

/// The IST day of an epoch second, as days from the epoch.
fn ist_day_of(epoch_secs: i64) -> Option<u32> {
    pull::session::IstMoment::from_epoch_secs(epoch_secs)
        .ok()
        .map(|moment| moment.day().days_from_epoch())
}

/// The decimal number after `marker` in `text`.
fn number_after(text: &str, marker: &str) -> Option<i64> {
    let (_, rest) = text.split_once(marker)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// One decoded member failure as a refusal, or `None` when its note names
/// none this reader knows.
fn pull_refusal(number: u64, record: &crate::audit::Record) -> Option<PullRefusal> {
    let note = record.note.as_str();
    let whole_window = WHOLE_WINDOW_NOTES
        .iter()
        .any(|prefix| note.starts_with(prefix));
    let dated = note
        .get(..10)
        .filter(|_| note.get(10..11) == Some(":"))
        .and_then(super::import_journal::parse_day)
        .map(Day::days_from_epoch);
    let names_day = dated
        .or_else(|| number_after(note, "timestamp ").and_then(ist_day_of))
        .or_else(|| {
            whole_window
                .then(|| number_after(note, " at ").and_then(ist_day_of))
                .flatten()
        });
    if !(whole_window || note.contains("refused") && names_day.is_some()) {
        return None;
    }
    Some(PullRefusal {
        record: number,
        at_unix_secs: record.at_unix_secs,
        from_days: record.from_days,
        to_days: record.to_days,
        note: note.to_owned(),
        cut: record.note_was_cut(),
        names_day,
        whole_window,
    })
}

/// The pull journal at `root`, for `symbol`'s refusals: through the journal's
/// index when it covers the journal ([`crate::audit::Journal::read_index`]),
/// else by a walk that says why.
///
/// # Cost, stated (WP-33c; DB-A34)
///
/// Through the index: one read each of `ident`, `cover` and `runs` (O(runs)),
/// one read of `symbol`'s `refusals` file and one positional read per record
/// it names (O(refusals of the symbol): 20 for NIFTY on the operator's store),
/// and a decode of the records past the cover (none after an append). No lock
/// is held and nothing grows with the journal.
///
/// Without an index (a journal the first append under this build has not yet
/// indexed, or an index of another file), through its tail: the first read in
/// a process walks every record, 4,217,027 of them (1,079,558,912 bytes) on the
/// operator's store on 9 Oct 2026, and later reads the records appended since.
/// The walk runs OUTSIDE the tails' lock (DB-A34), so one walk never stalls
/// every other page's read; the served process walks it at start, off the
/// request path ([`warm_pull_journal`], DB-N19), and a request that arrives
/// while that walk runs answers without it and says so. The answer names the
/// walk, its time and why the index could not be used.
///
/// # Errors
///
/// [`PullUnread::Failed`]: a file that is there and cannot be measured or
/// read, in its own words. [`PullUnread::Warming`]: the walk at start is
/// still running and there are records this read would have had to walk.
fn read_pull_journal(root: &Path, symbol: &str) -> Result<PullRead, PullUnread> {
    use std::os::unix::fs::MetadataExt as _;
    let started = std::time::Instant::now();
    let journal = crate::audit::Journal::at(root);
    let path = journal.path.clone();
    let meta = match std::fs::metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PullRead {
                path,
                present: false,
                bytes: 0,
                records: 0,
                torn: 0,
                read_now: 0,
                read_micros: 0,
                parsed: Arc::new(PullRefusals::default()),
                via: Via::Absent,
            });
        }
        Err(e) => return Err(PullUnread::Failed(format!("{}: {e}", path.display()))),
        Ok(meta) => meta,
    };
    let len = meta.len();
    let stride = crate::audit::RECORD_LEN_U64;
    let whole = len - len % stride;
    let (parsed, read_now, via) = match journal.read_index(whole / stride) {
        Ok(indexed) => {
            let (parsed, read_now) = pull_indexed(&journal, &indexed, symbol, whole / stride)
                .map_err(PullUnread::Failed)?;
            let via = Via::Index {
                covered: indexed.covered,
                runs: crate::audit::as_u64(indexed.runs.len()),
            };
            (parsed, read_now, via)
        }
        Err(why) => {
            let (parsed, read_now) = pull_walked(&path, Some((meta.dev(), meta.ino())), whole)?;
            (parsed, read_now, Via::Walk { why })
        }
    };
    Ok(PullRead {
        path,
        present: true,
        bytes: len,
        records: whole / stride,
        torn: len.saturating_sub(whole),
        read_now,
        read_micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
        parsed,
        via,
    })
}

/// `symbol`'s Zerodha pull refusals as JSON, oldest record first, and how they
/// were found: the reader `/ingest/refusals.json` shares with this route, so a
/// month cell on /ingest and a day on /db name the same records (WP-61,
/// DB-G23). One read of the symbol's index file, O(its refusals).
///
/// # Errors
///
/// The journal is there and cannot be read, or the served process is still
/// walking it at start, in the sentence this route's notes give.
pub(crate) fn zerodha_refusals_of(
    root: &Path,
    symbol: &str,
) -> Result<(Vec<Value>, Value), String> {
    let read = read_pull_journal(root, symbol).map_err(|unread| unread.note())?;
    let refusals = read
        .parsed
        .by_symbol
        .get(symbol)
        .map(|found| found.iter().map(PullRefusal::json).collect())
        .unwrap_or_default();
    let via = match read.via {
        Via::Absent => json!({ "kind": "absent" }),
        Via::Index { covered, runs } => {
            json!({ "kind": "index", "covered": covered, "runs": runs })
        }
        Via::Walk { ref why } => json!({ "kind": "walk", "why": why }),
    };
    Ok((
        refusals,
        json!({
            "path": read.path.display().to_string(),
            "present": read.present,
            "records": read.records,
            "read_now": read.read_now,
            "read_micros": read.read_micros,
            "via": via,
        }),
    ))
}

/// `symbol`'s Zerodha refusals through the index: the records its `refusals`
/// file names, each filed under the run before it, then the records past the
/// cover, decoded in order from the vendor of the last run the index holds.
/// Answers the refusals and how many records were read.
fn pull_indexed(
    journal: &crate::audit::Journal,
    indexed: &crate::audit::Indexed,
    symbol: &str,
    total: u64,
) -> Result<(Arc<PullRefusals>, u64), String> {
    let mut parsed = PullRefusals::default();
    let ordinals = journal.refusal_ordinals(symbol, indexed.covered)?;
    let mut read_now = crate::audit::as_u64(ordinals.len());
    for (ordinal, decoded) in journal.records_at(&ordinals)? {
        let number = ordinal.saturating_add(1);
        let record = match decoded {
            Err(why) => {
                parsed.faults.note(number, "", &format!("{why:?}"));
                continue;
            }
            Ok(record) => record,
        };
        // AN ENTRY THAT NAMES ANOTHER RECORD is a fault of the index, named,
        // never read as the symbol's.
        if record.kind != crate::audit::Kind::MemberFailure || record.source != symbol {
            parsed.faults.note(
                number,
                &record.source,
                "the journal's index names this record under another source or kind",
            );
            continue;
        }
        if indexed.vendor_before(ordinal) != Some(Vendor::Zerodha) {
            parsed.other_vendor = parsed.other_vendor.saturating_add(1);
            continue;
        }
        if let Some(refusal) = pull_refusal(number, &record) {
            parsed
                .by_symbol
                .entry(record.source.clone())
                .or_default()
                .push(refusal);
        }
    }
    // THE RECORDS PAST THE COVER, which no append has indexed yet: read in
    // order, from the vendor of the last run the index holds.
    if total > indexed.covered {
        let mut vendor = indexed.vendor_before(indexed.covered);
        let tail: Vec<u64> = (indexed.covered..total).collect();
        read_now = read_now.saturating_add(crate::audit::as_u64(tail.len()));
        let mut rest = PullRefusals::default();
        for (ordinal, decoded) in journal.records_at(&tail)? {
            let image = match decoded {
                Ok(ref record) => record.image(),
                Err(ref why) => {
                    rest.faults
                        .note(ordinal.saturating_add(1), "", &format!("{why:?}"));
                    continue;
                }
            };
            pull_record(&mut rest, &mut vendor, ordinal.saturating_add(1), &image);
        }
        if let Some(found) = rest.by_symbol.remove(symbol) {
            parsed
                .by_symbol
                .entry(symbol.to_owned())
                .or_default()
                .extend(found);
        }
        parsed.faults.count = parsed.faults.count.saturating_add(rest.faults.count);
        parsed.faults.named.extend(rest.faults.named);
        parsed.other_vendor = parsed.other_vendor.saturating_add(rest.other_vendor);
    }
    Ok((Arc::new(parsed), read_now))
}

/// Every symbol's Zerodha refusals by a walk of the journal through its tail:
/// the records appended since the last walk, or all of them the first time.
/// The walk runs OUTSIDE the tails' lock (DB-A34): the tail is copied under
/// the lock (an `Arc` and three numbers), the lock released, the records read,
/// and the result installed only if no other walk got further first. A walk
/// at start under way ([`warm_pull_journal`]) is never walked beside.
fn pull_walked(
    path: &Path,
    identity: Option<(u64, u64)>,
    whole: u64,
) -> Result<(Arc<PullRefusals>, u64), PullUnread> {
    let stride = crate::audit::RECORD_LEN_U64;
    let mut walk = {
        let mut held = pull_tails().lock().unwrap_or_else(PoisonError::into_inner);
        let tail = held
            .entry(path.to_path_buf())
            .or_insert_with(|| PullTail::fresh(identity));
        if tail.identity != identity || whole < tail.upto {
            *tail = PullTail::fresh(identity);
        }
        // THE WALK AT START IS NOT REPEATED BESIDE IT (DB-N19): it reads every
        // record once, and a request that walked too would pay the whole walk.
        if let Some(began) = tail.warming
            && whole > tail.upto
        {
            return Err(PullUnread::Warming {
                path: path.to_path_buf(),
                since: began.elapsed(),
                records: whole / stride,
            });
        }
        PullTail {
            identity: tail.identity,
            upto: tail.upto,
            vendor: tail.vendor,
            parsed: Arc::clone(&tail.parsed),
            warming: None,
        }
    };
    let read_now = walk.advance(path, whole).map_err(PullUnread::Failed)?;
    let parsed = Arc::clone(&walk.parsed);
    let mut held = pull_tails().lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(tail) = held.get_mut(path)
        && tail.identity == walk.identity
        && tail.upto < walk.upto
    {
        tail.upto = walk.upto;
        tail.vendor = walk.vendor;
        tail.parsed = walk.parsed;
    }
    Ok((parsed, read_now))
}

/// What [`warm_pull_journal`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PullWarmed {
    /// Whether there is a journal.
    pub present: bool,
    /// The records the walk read.
    pub read: u64,
    /// How long it took, in microseconds.
    pub micros: u64,
}

/// Walk the pull journal at `root` once, off the request path: what the
/// served process does at start, so no Zerodha day-flags request pays the
/// first walk (683-730 ms on the operator's store, DB-N19).
///
/// The walk runs without the tails' lock, into a tail of its own, and is
/// installed under it when done; while it runs, the tail is marked, and a
/// request that finds records to read answers without them and says why
/// ([`PullUnread::Warming`]). Mark and install are under the lock, so a
/// request sees either the mark or the installed tail, never neither.
///
/// # Errors
///
/// A journal that is there and could not be measured or read.
pub(crate) fn warm_pull_journal(root: &Path) -> Result<PullWarmed, String> {
    use std::os::unix::fs::MetadataExt as _;
    let started = std::time::Instant::now();
    let path = crate::audit::journal_path(root);
    let meta = match std::fs::metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PullWarmed {
                present: false,
                read: 0,
                micros: 0,
            });
        }
        Err(e) => return Err(format!("{}: {e}", path.display())),
        Ok(meta) => meta,
    };
    let identity = Some((meta.dev(), meta.ino()));
    let len = meta.len();
    let whole = len - len % crate::audit::RECORD_LEN_U64;
    // A JOURNAL ITS INDEX COVERS IS NOT WALKED (WP-33c): a request reads it
    // through the index, so a walk at start would read 1 GiB for nobody.
    if crate::audit::Journal::at(root)
        .read_index(whole / crate::audit::RECORD_LEN_U64)
        .is_ok()
    {
        return Ok(PullWarmed {
            present: true,
            read: 0,
            micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
        });
    }
    let mut walk = {
        let mut held = pull_tails().lock().unwrap_or_else(PoisonError::into_inner);
        let tail = held
            .entry(path.clone())
            .or_insert_with(|| PullTail::fresh(identity));
        if tail.identity != identity || whole < tail.upto {
            *tail = PullTail::fresh(identity);
        }
        if tail.upto >= whole || tail.warming.is_some() {
            return Ok(PullWarmed {
                present: true,
                read: 0,
                micros: 0,
            });
        }
        tail.warming = Some(started);
        PullTail {
            identity,
            upto: tail.upto,
            vendor: tail.vendor,
            parsed: Arc::clone(&tail.parsed),
            warming: None,
        }
    };
    // THE MARK IS TAKEN OFF WHATEVER HAPPENS, a failed or unwound walk
    // included: a mark left behind would answer every request "walking" for
    // the life of the process.
    let unmark = Unmark(&path);
    let read = walk.advance(&path, whole);
    {
        let mut held = pull_tails().lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(tail) = held.get_mut(&path)
            && read.is_ok()
            && tail.identity == walk.identity
            && tail.upto <= walk.upto
        {
            *tail = walk;
        }
    }
    drop(unmark);
    Ok(PullWarmed {
        present: true,
        read: read?,
        micros: u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX),
    })
}

/// Takes the walk-at-start mark off a journal's tail when dropped.
struct Unmark<'a>(&'a Path);

impl Drop for Unmark<'_> {
    fn drop(&mut self) {
        if let Some(tail) = pull_tails()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(self.0)
        {
            tail.warming = None;
        }
    }
}

/// Folds one 256-byte record into `parsed`: a run sets the vendor its member
/// failures belong to; a Zerodha member failure naming a refusal is kept.
fn pull_record(parsed: &mut PullRefusals, vendor: &mut Option<Vendor>, number: u64, image: &[u8]) {
    let Ok(image) = <&[u8; crate::audit::RECORD_LEN]>::try_from(image) else {
        return;
    };
    let kind = image.get(PULL_KIND_AT).copied();
    let member = kind == Some(crate::audit::Kind::MemberFailure.code());
    if member && !may_refuse(image) {
        // NOT DECODED, NOT A FAULT: a member failure whose note names no
        // refusal (`request minute coverage UNVERIFIED ...`, most of them).
        return;
    }
    if !member && kind != Some(crate::audit::Kind::Run.code()) {
        parsed
            .faults
            .note(number, "", "a record kind this reader does not know");
        return;
    }
    match crate::audit::Record::decode(image) {
        Err(why) => {
            parsed.faults.note(number, "", &format!("{why:?}"));
            if !member {
                // A RUN THAT WILL NOT DECODE names no vendor: its members are
                // counted apart, never given the previous run's vendor.
                *vendor = None;
            }
        }
        Ok(record) if record.kind == crate::audit::Kind::Run => {
            *vendor = crate::audit_json::vendor_of_source(&record.source);
        }
        Ok(record) => {
            if *vendor != Some(Vendor::Zerodha) {
                parsed.other_vendor = parsed.other_vendor.saturating_add(1);
                return;
            }
            if let Some(refusal) = pull_refusal(number, &record) {
                parsed
                    .by_symbol
                    .entry(record.source.clone())
                    .or_default()
                    .push(refusal);
            }
        }
    }
}

/// The refusals of `symbol`'s that a record of `day` names: one that names the
/// day, and one that refused a whole window holding it -- but that one only
/// where no `1day` bar of the series is held up to the day (before its first,
/// or none held near the page). A window one run refused and another landed is
/// no hole, and naming its refusal on every held day would say one.
///
/// O(refusals of the symbol): 20 for NIFTY on the operator's store.
fn pull_view<'a>(
    parsed: &'a PullRefusals,
    symbol: &str,
    day: Day,
    start: Option<SeriesStart>,
) -> Vec<&'a PullRefusal> {
    let on = day.days_from_epoch();
    let front_open = start.and_then(|start| start.before(day)) != Some(false);
    parsed
        .by_symbol
        .get(symbol)
        .map(|refusals| {
            refusals
                .iter()
                .filter(|r| r.about(on) && (!r.whole_window || front_open))
                .collect()
        })
        .unwrap_or_default()
}

/// The sentences an answer owes about the pull journal.
fn pull_notes(pull: Option<&Result<PullRead, PullUnread>>) -> Vec<String> {
    match pull {
        Some(Err(unread)) => vec![unread.note()],
        Some(Ok(read)) if !read.present => vec![format!(
            "No Zerodha pull journal at {}: no record names a pull refusal, and a day before \
             the series' first 1day bar is not judged a refused front by it.",
            read.path.display()
        )],
        // A WALK IS SAID, with its measured time and why the index could not
        // be used, whenever this answer was the one that paid for it.
        Some(Ok(read)) if read.read_now > FIRST_WALK_RECORDS => vec![format!(
            "This answer read {} pull-journal record(s) in {} ms by a walk of the journal, \
             because its index could not be used ({}); a process walks it once, and later \
             reads cost the records appended since.",
            read.read_now,
            read.read_micros / 1_000,
            match read.via {
                Via::Walk { ref why } => why.as_str(),
                Via::Index { .. } | Via::Absent => "the index covers only part of it",
            }
        )],
        Some(Ok(_)) | None => Vec::new(),
    }
}

/// A read of more pull-journal records than this is named in the answer's
/// notes: a process's first walk, not a tail.
const FIRST_WALK_RECORDS: u64 = 100_000;

/// Whether Zerodha holds `inst`'s `1day` month `month`: one `metadata` call.
fn day_month_held(root: &Path, inst: &Instrument, month: YearMonth) -> bool {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: inst.exchange.as_str(),
        segment: inst.segment.as_str(),
        symbol: &inst.symbol,
        contract: inst.contract,
        timeframe: Timeframe::DAY_1,
        month,
        file: FileKind::Bars,
    })
    .is_ok_and(|path| path.to_path_buf(root).is_file())
}

/// The month `steps` months from `month` (forward when `forward`).
fn step_month(month: YearMonth, steps: u32, forward: bool) -> Option<YearMonth> {
    let index = u32::from(month.year())
        .checked_mul(12)?
        .checked_add(u32::from(month.month()))?
        .checked_sub(1)?;
    let moved = if forward {
        index.checked_add(steps)?
    } else {
        index.checked_sub(steps)?
    };
    YearMonth::new(
        u16::try_from(moved / 12).ok()?,
        u8::try_from(moved % 12 + 1).ok()?,
    )
    .ok()
}

/// Where a Zerodha `1day` series starts, seen from a page's first month.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SeriesStart {
    /// A month before the page's is held: no day of the page is a front.
    HeldBefore,
    /// The series' first held `1day` bar.
    FirstBar(Day),
    /// No month is held within [`FRONT_PROBE_MONTHS`] of the page, or the
    /// first held one could not be read: nothing to be in front of.
    NoneHeld,
}

impl SeriesStart {
    fn json(self) -> Value {
        match self {
            Self::HeldBefore => json!({ "state": "held_before", "first_bar": null }),
            Self::FirstBar(day) => {
                json!({ "state": "first_bar", "first_bar": day.to_string() })
            }
            Self::NoneHeld => json!({ "state": "none_held", "first_bar": null }),
        }
    }

    /// Whether `day` is before the series' first held `1day` bar; `None` when
    /// nothing of the series is held near the page, so there is no first bar
    /// to be before.
    fn before(self, day: Day) -> Option<bool> {
        match self {
            Self::HeldBefore => Some(false),
            Self::FirstBar(first) => Some(day < first),
            Self::NoneHeld => None,
        }
    }
}

/// Where `inst`'s Zerodha `1day` series starts, seen from `page_first`'s
/// month: at most [`FRONT_PROBE_MONTHS`] `metadata` calls back, as many
/// forward, and one month opened for its first bar. Called only when a
/// whole-window refusal covers a day of the page.
fn series_start(root: &Path, inst: &Instrument, page_first: Day) -> SeriesStart {
    let Ok(month) = page_first.year_month() else {
        return SeriesStart::NoneHeld;
    };
    let held = |probe: &YearMonth| day_month_held(root, inst, *probe);
    if (1..=FRONT_PROBE_MONTHS)
        .map_while(|steps| step_month(month, steps, false))
        .any(|probe| held(&probe))
    {
        return SeriesStart::HeldBefore;
    }
    let Some(first) = (0..=FRONT_PROBE_MONTHS)
        .map_while(|steps| step_month(month, steps, true))
        .find(held)
    else {
        return SeriesStart::NoneHeld;
    };
    let Opened::File(file) = open_month(root, Vendor::Zerodha, inst, Timeframe::DAY_1, first)
    else {
        return SeriesStart::NoneHeld;
    };
    file.read_record(0)
        .ok()
        .and_then(|bar| ist_day_of(bar.ts_micros.div_euclid(1_000_000)))
        .and_then(|days| Day::from_days(days).ok())
        .map_or(SeriesStart::NoneHeld, SeriesStart::FirstBar)
}

// ══ Bars: counts per day, and Zerodha's day bars ════════════════════════════

/// What one feed holds on one day at one rung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    /// The month's file is there and holds this many bars that day.
    Bars(u64),
    /// No file for the month: the feed holds nothing there at this rung.
    NoFile,
    /// The file is there and could not be read; a note names why.
    Unreadable,
    /// A writer (an import worker, a month repair, an index build) held the
    /// month past the read's patience: nothing is wrong with the file, and
    /// the count is there once the writer lets go (R3-08, RA-117). Never
    /// "unreadable", the word a damaged file earns.
    BeingWritten,
    /// The instrument is an underlying's options as a kind (`NSE-FNO-NIFTY`)
    /// and names no contract, so there is no one series to count.
    NoContract,
}

impl Held {
    fn json(self, feed: Vendor, rung: Timeframe) -> Value {
        let (bars, why) = match self {
            Self::Bars(n) => (Value::from(n), Value::Null),
            Self::NoFile => (Value::Null, Value::from("not_held")),
            Self::Unreadable => (Value::Null, Value::from("unreadable")),
            Self::BeingWritten => (Value::Null, Value::from("being_written")),
            Self::NoContract => (Value::Null, Value::from("no_contract")),
        };
        json!({ "feed": feed.as_str(), "rung": rung.as_str(), "bars": bars, "why": why })
    }

    const fn bars(self) -> Option<u64> {
        match self {
            Self::Bars(n) => Some(n),
            _ => None,
        }
    }
}

/// One feed's counts over the page, and what could not be read.
struct Counts {
    per_day: Vec<Held>,
    /// Per day, how many of the held bars carry zero volume — a candle for a
    /// minute with no trade in it (DB-B16) — where counted: a Zerodha share's
    /// one-minute bars only, `None` elsewhere.
    fills: Vec<Option<u64>>,
    /// Per day, how many of the held bars open inside the halt the calendar
    /// of record holds for the day and the instrument's segment (WP-11c,
    /// DB-A07): `None` on a day with no recorded halt, or where the day's
    /// bars were not counted.
    halted: Vec<Option<u64>>,
    notes: Vec<String>,
    /// Every month and lookup that could not be read, one sentence each,
    /// whatever the cause: what [`answer`] lists as `month_faults`, and what
    /// the route answers "being written, ask again" from when a writer is
    /// the only cause (R3-08, RA-117).
    faults: Vec<String>,
}

/// One month that would not open, said once in `notes` and once in `faults`:
/// a writer's hold as being written (its cells [`Held::BeingWritten`]),
/// anything else as could not be read (its cells [`Held::Unreadable`]).
/// Whether it is a writer's hold.
fn month_refused(what: &str, why: &str, notes: &mut Vec<String>, faults: &mut Vec<String>) -> bool {
    let written = crate::detail::is_being_written(why);
    let said = if written {
        format!("{what}: {why}")
    } else {
        format!("{what}: the month could not be read: {why}")
    };
    notes.push(said.clone());
    faults.push(said);
    written
}

impl Counts {
    /// An underlying's options as a kind, which names no contract: nothing
    /// to count on any of `days` days, and nothing that failed.
    fn no_contract(days: usize) -> Self {
        Self {
            per_day: vec![Held::NoContract; days],
            fills: vec![None; days],
            halted: vec![None; days],
            notes: Vec::new(),
            faults: Vec::new(),
        }
    }
}

/// The most bars one day's zero-volume count reads: a whole session of
/// one-minute bars is 375, so a day holding more is not counted (it is not a
/// one-minute share's day) and the count says `None` rather than scanning.
const MAX_FILL_SCAN: u64 = 1_440;

/// A month file, opened for reading, or why not.
enum Opened {
    File(Box<BarFile>),
    Absent,
    Refused(String),
}

/// The symbol id `pull::ingest` stamps into a month's header, which the store
/// verifies on every open.
fn symbol_id(symbol: &str) -> u32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id is a cross-check the store verifies on reopen, never an index -- the \
                  low half of a 64-bit FNV-1a is what pull::ingest stamps in, and a different \
                  fold would fail to open a file this repository wrote"
    )]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    id
}

/// `inst`'s month at `rung` on `feed`, opened read-only: never created.
fn open_month(
    root: &Path,
    feed: Vendor,
    inst: &Instrument,
    rung: Timeframe,
    month: YearMonth,
) -> Opened {
    let parts = PathParts {
        vendor: feed,
        exchange: inst.exchange.as_str(),
        segment: inst.segment.as_str(),
        symbol: &inst.symbol,
        contract: inst.contract,
        timeframe: rung,
        month,
        file: FileKind::Bars,
    };
    let path = match StorePath::new(parts) {
        Ok(path) => path,
        Err(why) => return Opened::Refused(format!("{} {month:?}: {why}", inst.key())),
    };
    // A KEY THE RE-KEY RETIRED IS NOT COUNTED (D-3331; WP-12c, DB-A18): its
    // files are kept, and this reader refuses it by the ledger's sentence, as
    // every bar route does with 410. One `stat` and one hash probe.
    if feed == Vendor::Gdfl
        && inst.segment == Segment::Fno
        && inst.contract.is_some()
        && let Some(why) = crate::bars::retired_refusal(root, feed, &inst.key())
    {
        return Opened::Refused(why);
    }
    match BarFile::open_existing(root, path, symbol_id(&inst.symbol)) {
        Ok(file) => Opened::File(Box::new(file)),
        Err(StoreError::Missing { path: missing, .. }) if missing == path.to_path_buf(root) => {
            Opened::Absent
        }
        Err(why) => Opened::Refused(why.to_string()),
    }
}

/// IST midnight opening `day`, in epoch microseconds.
fn ist_midnight_micros(day: Day) -> i64 {
    i64::from(day.days_from_epoch())
        .saturating_mul(86_400)
        .saturating_sub(pull::session::IST_OFFSET_SECS)
        .saturating_mul(1_000_000)
}

/// The run of `days` from `at` that share `days[at]`'s month.
fn month_run(days: &[Day], at: usize) -> usize {
    let month = days.get(at).map(|day| (day.year(), day.month()));
    days.get(at..)
        .unwrap_or_default()
        .iter()
        .take_while(|day| Some((day.year(), day.month())) == month)
        .count()
}

/// The bars `feed` holds for `inst` at `rung` on each of `days` (oldest
/// first, consecutive): one open per month and one lookup per day boundary.
///
/// With `fills`, each day's bars are also read once to count those with zero
/// volume (DB-B16): at most [`MAX_FILL_SCAN`] records a day, so a page costs
/// at most 92 x 1,440 record reads — a Zerodha share's one-minute session is
/// 375. Every other series is not read past its day boundaries.
fn counts(
    root: &Path,
    feed: Vendor,
    inst: &Instrument,
    rung: Timeframe,
    days: &[Day],
    fills: bool,
) -> Counts {
    let mut per_day = Vec::with_capacity(days.len());
    let mut zero = Vec::with_capacity(days.len());
    let mut halted = Vec::with_capacity(days.len());
    let mut notes = Vec::new();
    let mut faults = Vec::new();
    if inst.segment == Segment::Fno && inst.contract.is_none() {
        return Counts::no_contract(days.len());
    }
    let mut at = 0usize;
    while let Some(first) = days.get(at) {
        let run = month_run(days, at).max(1);
        let month_days = days.get(at..at.saturating_add(run)).unwrap_or_default();
        let opened = match first.year_month() {
            Ok(month) => open_month(root, feed, inst, rung, month),
            Err(why) => Opened::Refused(why.to_string()),
        };
        match opened {
            Opened::Absent => per_day.extend(month_days.iter().map(|_| Held::NoFile)),
            Opened::Refused(why) => {
                let what = format!("{} {} {} {first}", feed.as_str(), inst.key(), rung.as_str());
                let held = if month_refused(&what, &why, &mut notes, &mut faults) {
                    Held::BeingWritten
                } else {
                    Held::Unreadable
                };
                per_day.extend(month_days.iter().map(|_| held));
            }
            Opened::File(file) => {
                let mut lower: Option<u64> = None;
                for day in month_days {
                    let start = ist_midnight_micros(*day);
                    let counted = lower
                        .map_or_else(|| file.first_at_or_after(start), Ok)
                        .and_then(|lo| {
                            let hi = file.first_at_or_after(start.saturating_add(DAY_MICROS))?;
                            Ok((lo, hi))
                        });
                    match counted {
                        Ok((lo, hi)) => {
                            let held = hi.saturating_sub(lo);
                            per_day.push(Held::Bars(held));
                            halted.push(halted_of(&file, (feed, inst, rung), *day, &mut notes));
                            zero.push(
                                (fills && held <= MAX_FILL_SCAN)
                                    .then(|| {
                                        (lo..hi).try_fold(0_u64, |n, at| {
                                            file.read_record(at)
                                                .map(|bar| n + u64::from(bar.volume == 0))
                                        })
                                    })
                                    .and_then(|counted| match counted {
                                        Ok(n) => Some(n),
                                        Err(why) => {
                                            notes.push(format!(
                                                "{} {} {} {day}: zero-volume bars could not be counted: {why}",
                                                feed.as_str(),
                                                inst.key(),
                                                rung.as_str()
                                            ));
                                            None
                                        }
                                    }),
                            );
                            lower = Some(hi);
                        }
                        Err(why) => {
                            let said = format!(
                                "{} {} {} {day}: a lookup failed: {why}",
                                feed.as_str(),
                                inst.key(),
                                rung.as_str()
                            );
                            notes.push(said.clone());
                            faults.push(said);
                            // ALL THREE ANSWERS, or every later day of the
                            // month reads the next day's halt count.
                            per_day.push(Held::Unreadable);
                            zero.push(None);
                            halted.push(None);
                            lower = None;
                        }
                    }
                }
            }
        }
        zero.resize(per_day.len(), None);
        halted.resize(per_day.len(), None);
        at = at.saturating_add(run);
    }
    Counts {
        per_day,
        fills: zero,
        halted,
        notes,
        faults,
    }
}

/// [`bars_in_halt`] for one day of `inst`, a failed lookup named in `notes`
/// and answered `None`: never a count the store did not give.
fn halted_of(
    file: &BarFile,
    (feed, inst, rung): (Vendor, &Instrument, Timeframe),
    day: Day,
    notes: &mut Vec<String>,
) -> Option<u64> {
    match bars_in_halt(file, inst.venue(), day)? {
        Ok(inside) => Some(inside),
        Err(why) => {
            notes.push(format!(
                "{} {} {} {day}: the bars inside the halt could not be counted: {why}",
                feed.as_str(),
                inst.key(),
                rung.as_str()
            ));
            None
        }
    }
}

/// The bars of `file` that open inside the halt the calendar of record holds
/// for `day` and `venue` -- the segment's own seconds, `from` up to `resume`
/// -- or `None` on a day with no recorded halt (WP-11c, DB-A07). Two
/// `first_at_or_after` lookups on a halt day, none on any other: the
/// calendar's halts are three days, so a page pays for at most three.
fn bars_in_halt(file: &BarFile, venue: Venue, day: Day) -> Option<Result<u64, StoreError>> {
    let halt = pull::calendar::halt_on(i64::from(day.days_from_epoch()))?;
    let seconds = halt.seconds(venue);
    let at = |sod: u32| {
        file.first_at_or_after(
            ist_midnight_micros(day).saturating_add(i64::from(sod).saturating_mul(1_000_000)),
        )
    };
    Some(at(seconds.from).and_then(|lo| Ok(at(seconds.resume)?.saturating_sub(lo))))
}

/// One Zerodha `1day` bar's values, paisa and shares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DayBar {
    open: i64,
    high: i64,
    low: i64,
    close: i64,
    volume: i64,
}

impl DayBar {
    const fn of(bar: &store::format::Bar) -> Self {
        Self {
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
        }
    }
}

/// A day's `1day` standing: its bar, no bar in a held month, a month not
/// held, or a month that could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DayBarAt {
    Bar(DayBar),
    None,
    NotHeld,
    Unreadable,
    /// A writer held the month past the read's patience ([`Held::BeingWritten`]).
    BeingWritten,
}

/// The page's `1day` bars and, for `impossible_jump`, the close before each.
struct DayBars {
    per_day: Vec<DayBarAt>,
    /// The previous held bar's close for each day: the last bar before it,
    /// in the page or, for the page's first bars, before the page.
    prev_close: Vec<Option<i64>>,
    notes: Vec<String>,
    /// Every month that could not be read, as [`Counts::faults`].
    faults: Vec<String>,
}

impl DayBars {
    fn at(&self, at: usize) -> (DayBarAt, Option<i64>) {
        (
            self.per_day.get(at).copied().unwrap_or(DayBarAt::NotHeld),
            self.prev_close.get(at).copied().flatten(),
        )
    }
}

/// Every bar of a `1day` month, with its IST day, or why not.
fn month_day_bars(file: &BarFile) -> Result<Vec<(Day, DayBar)>, String> {
    let n = file.records();
    if n > MAX_DAY_BARS {
        return Err(format!(
            "it holds {n} records, more than the {MAX_DAY_BARS} a month of days can"
        ));
    }
    let mut out = Vec::with_capacity(usize::try_from(n).unwrap_or(0));
    for index in 0..n {
        let bar = file.read_record(index).map_err(|why| why.to_string())?;
        let day = pull::session::IstMoment::from_epoch_secs(bar.ts_micros.div_euclid(1_000_000))
            .map_err(|why| why.to_string())?
            .day();
        out.push((day, DayBar::of(&bar)));
    }
    Ok(out)
}

/// The last `1day` bar before `day`, in the months before it, at most
/// [`LOOKBACK_MONTHS`] opens back.
fn bar_before(root: &Path, inst: &Instrument, day: Day) -> Option<DayBar> {
    let mut probe = day;
    for _ in 0..LOOKBACK_MONTHS {
        probe = Day::from_days(
            probe
                .days_from_epoch()
                .checked_sub(u32::from(probe.day()))?,
        )
        .ok()?;
        let month = probe.year_month().ok()?;
        match open_month(root, Vendor::Zerodha, inst, Timeframe::DAY_1, month) {
            Opened::Absent => {}
            Opened::Refused(_) => return None,
            Opened::File(file) => {
                let n = file.records();
                if n == 0 {
                    continue;
                }
                return file.read_record(n - 1).ok().map(|bar| DayBar::of(&bar));
            }
        }
    }
    None
}

/// The page's Zerodha `1day` bars, one month file read whole at a time.
fn day_bars(root: &Path, inst: &Instrument, days: &[Day]) -> DayBars {
    let mut per_day = Vec::with_capacity(days.len());
    let mut notes = Vec::new();
    let mut faults = Vec::new();
    let mut before_page: Option<DayBar> = None;
    let mut at = 0usize;
    while let Some(first) = days.get(at) {
        let run = month_run(days, at).max(1);
        let month_days = days.get(at..at.saturating_add(run)).unwrap_or_default();
        let opened = match first.year_month() {
            Ok(month) => open_month(root, Vendor::Zerodha, inst, Timeframe::DAY_1, month),
            Err(why) => Opened::Refused(why.to_string()),
        };
        let held = match opened {
            Opened::File(file) => month_day_bars(&file).map(Some),
            Opened::Absent => Ok(None),
            Opened::Refused(why) => Err(why),
        };
        match held {
            Ok(None) => per_day.extend(month_days.iter().map(|_| DayBarAt::NotHeld)),
            Err(why) => {
                let what = format!("zerodha {} 1day {first}", inst.key());
                let held = if month_refused(&what, &why, &mut notes, &mut faults) {
                    DayBarAt::BeingWritten
                } else {
                    DayBarAt::Unreadable
                };
                per_day.extend(month_days.iter().map(|_| held));
            }
            Ok(Some(bars)) => {
                if at == 0 {
                    before_page = bars
                        .iter()
                        .filter(|(day, _)| day < first)
                        .map(|(_, bar)| *bar)
                        .next_back();
                }
                for day in month_days {
                    let bar = bars
                        .iter()
                        .find_map(|(held, bar)| (held == day).then_some(*bar));
                    per_day.push(bar.map_or(DayBarAt::None, DayBarAt::Bar));
                }
            }
        }
        at = at.saturating_add(run);
    }
    if before_page.is_none()
        && let Some(first) = days.first()
    {
        before_page = bar_before(root, inst, *first);
    }
    let mut prev = before_page.map(|bar| bar.close);
    let prev_close = per_day
        .iter()
        .map(|held| {
            let this = prev;
            if let DayBarAt::Bar(bar) = held {
                prev = Some(bar.close);
            }
            this
        })
        .collect();
    DayBars {
        per_day,
        prev_close,
        notes,
        faults,
    }
}

// ══ One record ══════════════════════════════════════════════════════════════

/// What every record of one answer reads from.
struct Context<'a> {
    asked: &'a Asked,
    /// The index whose sampling a record names ([`sampled_series`]): the
    /// instrument itself, an option's underlying, or none.
    sampled_of: Option<&'a Instrument>,
    journal: Option<&'a JournalRows>,
    quality: Option<&'a LedgerRead<QualityLedger>>,
    gaps: Option<&'a LedgerRead<GapLedger>>,
    /// The Zerodha pull journal, for a Zerodha spot page.
    pull: Option<&'a PullRead>,
    /// Where the Zerodha `1day` series starts, for a Zerodha spot page.
    start: Option<SeriesStart>,
}

/// What the store holds of a page's days: this feed's bars, the other
/// feed's, and the sampled series' where that is another series.
struct PageHeld {
    own: Counts,
    peer: Counts,
    /// AN OPTION'S UNDERLYING INDEX, COUNTED BESIDE IT (WP-11c, DB-A04): a
    /// contract's spot-derived cells are only as dense as the index it
    /// reads, so the day says when that index is minute-sampled. Asked of
    /// the store, never of a list: an underlying that is a share holds no
    /// index month and answers `not_held`. One open a month and one lookup a
    /// day boundary, only where the rung can show sampling at all.
    under: Option<Counts>,
}

impl PageHeld {
    /// Counts `days` for `asked`; `fills` also counts the zero-volume bars,
    /// and `series` is the page's sampled series ([`sampled_series`]).
    fn count(
        root: &Path,
        asked: &Asked,
        days: &[Day],
        (fills, series): (bool, Option<&Instrument>),
    ) -> Self {
        let (inst, rung) = (&asked.instrument, finest(asked.feed));
        let peer_feed = peer_of(asked.feed);
        Self {
            own: counts(root, asked.feed, inst, rung, days, fills),
            peer: counts(root, peer_feed, inst, finest(peer_feed), days, false),
            under: series
                .filter(|index| index.segment != inst.segment)
                .map(|index| counts(root, asked.feed, index, rung, days, false)),
        }
    }

    /// What could not be read, of all three.
    fn notes(&self) -> impl Iterator<Item = String> + '_ {
        [Some(&self.own), Some(&self.peer), self.under.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(|counts| counts.notes.iter().cloned())
    }

    /// Every month and lookup of all three that could not be read.
    fn faults(&self) -> impl Iterator<Item = String> + '_ {
        [Some(&self.own), Some(&self.peer), self.under.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(|counts| counts.faults.iter().cloned())
    }

    /// The page's day `at`.
    fn day(&self, at: usize) -> DayHeld {
        let held = |counts: &Counts| counts.per_day.get(at).copied().unwrap_or(Held::NoFile);
        DayHeld {
            own: held(&self.own),
            fills: self.own.fills.get(at).copied().flatten(),
            halted: self.own.halted.get(at).copied().flatten(),
            peer: held(&self.peer),
            series: held(self.under.as_ref().unwrap_or(&self.own)),
        }
    }
}

/// What the store holds of one day, counted before its record is made.
#[derive(Debug, Clone, Copy)]
struct DayHeld {
    /// This feed's bars at its finest rung.
    own: Held,
    /// Of them, the zero-volume ones, where counted (DB-B16).
    fills: Option<u64>,
    /// Of them, those inside the day's halt, on a halt day (DB-A07).
    halted: Option<u64>,
    /// The other feed's bars at its finest rung.
    peer: Held,
    /// The bars of the series whose sampling the record names
    /// ([`sampled_series`]): `own` for an index, the underlying's for an
    /// option.
    series: Held,
}

/// The index series whose sampling a record of `inst` names (D-4928; WP-11c,
/// DB-A04), or `None` where the feed's finest rung cannot show sampling or
/// the instrument is a share.
///
/// An index names itself. **An option names the index the sourced alias
/// table gives its symbol** (`brutex_core::alias::option_underlying`, D-3332;
/// D-4928 item 5) -- the one answer `/spot.json` and the bar routes read, so
/// a FINNIFTY option reads `NSE-INDEX-NIFTYFINSERVICE`, the key its index
/// bars and its day-ledger line are filed under, never an index of the
/// option's own symbol. Where the table says the underlying is a share, the
/// option names none and no month is opened for it: a share's own seconds
/// are trades and are sparse by nature, so a share names none either. The
/// rung must hold more than [`SAMPLED_SS_DISTINCT`] bars a minute, or every
/// series would read as sampled: a one-minute feed is never asked.
///
/// [`SAMPLED_SS_DISTINCT`]: pull::gdfl_import::quality::SAMPLED_SS_DISTINCT
fn sampled_series(feed: Vendor, inst: &Instrument) -> Option<Instrument> {
    let secs = u64::from(finest(feed).secs());
    if secs == 0 || 60 / secs <= pull::gdfl_import::quality::SAMPLED_SS_DISTINCT {
        return None;
    }
    match inst.segment {
        Segment::Index => Some(inst.clone()),
        Segment::Fno => {
            let underlying = brutex_core::alias::option_underlying(&inst.symbol);
            (underlying.segment == Segment::Index).then(|| Instrument {
                exchange: inst.exchange,
                segment: Segment::Index,
                symbol: underlying.key.to_owned(),
                contract: None,
            })
        }
        Segment::Cash => None,
    }
}

/// `held` of `of` as a percentage to one decimal, half up; `None` of nothing.
fn percent(held: u64, of: u64) -> Option<String> {
    let tenths = held
        .saturating_mul(1_000)
        .saturating_add(of / 2)
        .checked_div(of)?;
    Some(format!("{}.{}", tenths / 10, tenths % 10))
}

/// The record's `sampled` (D-4928; WP-11c, DB-A04): whether `series` is a
/// minute-sampled day, with the numbers that say so and where they are from.
///
/// **THE DAY LEDGER'S LINE IS THE AUTHORITY** (D-3335): where the
/// import or a rescan measured the vendor's rows, the rule is the ledger's
/// own (`pull::gdfl_import::quality::is_sampled`: rows on at most two
/// distinct seconds of the minute) and the standing is `measured`.
///
/// **WHERE NO LINE EXISTS THE STORE'S OWN BARS ANSWER, AND SAY THEY ARE A
/// SUSPICION.** Most stored days were imported before the ledger was written
/// (2019-05-30 among them): a series whose rows use at most two seconds of
/// the minute holds at most two bars a minute, so a day holding no more than
/// that is `suspected` -- it cannot be told from a day mostly missing until
/// `cli gdfl-dayflags` measures its rows. Never silent, never claimed as
/// measured. O(1): the counts the record already holds.
///
/// `null` where no series is named, the day is not a session, or neither
/// source can say.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
fn sampled_json(
    own: &Instrument,
    series: Option<&Instrument>,
    line: Option<&QualityLine>,
    held: Held,
    day: Day,
) -> Value {
    use pull::gdfl_import::quality;
    let Some(series) = series else {
        return Value::Null;
    };
    let of = if series.segment == own.segment {
        "own"
    } else {
        "underlying"
    };
    if let Some(line) = line {
        let (rows, distinct) = (
            line.count("rows").unwrap_or(0),
            line.count("ss_distinct").unwrap_or(0),
        );
        return json!({
            "is": quality::is_sampled(rows, distinct),
            "of": of,
            "series": series.key(),
            "basis": "day_ledger",
            "standing": "measured",
            "held_s": line.count("seconds"),
            "session_s": line.count("session_s"),
            "ss_distinct": distinct,
            "line": line.line,
        });
    }
    let (Some(bars), Ok(session)) = (held.bars(), pull::gdfl_cm::session_of(day)) else {
        return Value::Null;
    };
    let session_s = quality::session_seconds(&session);
    let most = quality::SAMPLED_SS_DISTINCT.saturating_mul(session_s / 60);
    json!({
        "is": bars > 0 && bars <= most,
        "of": of,
        "series": series.key(),
        "basis": "held_bars",
        "standing": "suspected",
        "held_s": bars,
        "session_s": session_s,
        "ss_distinct": null,
        "line": null,
    })
}

/// D-3336's day code, and the day's kind as a word.
///
/// THE CODE IS `/calendar.json`'S OWN, from
/// [`crate::calendar_of::record::record_code`], never a second reading of the
/// calendar: a day band drawn from this route and a coverage calendar drawn
/// from that one must not give one day two codes. A day the record does not
/// size is `-` here, where `/calendar.json` may still observe it from NIFTY's
/// and BANKNIFTY's bars: this route answers one series and votes nothing.
fn day_code(epoch_day: i64) -> (char, &'static str) {
    use crate::calendar_of::record as code;
    let (byte, word) = match code::record_code(epoch_day) {
        None => (code::OUTSIDE, "outside_calendar"),
        Some(code::CLOSED) => (code::CLOSED, "closed"),
        Some(code::UNSIZED) => (code::UNSIZED, "unmeasured_length"),
        Some(code::HALTED) => (code::HALTED, "halt_suspected"),
        Some(code::SPECIAL) => (code::SPECIAL, "special"),
        Some(code::AUCTION) => (code::AUCTION, "regular_with_closing_auction"),
        Some(code::REGULAR) => (code::REGULAR, "regular"),
        // A CODE THIS ROUTE HAS NO WORD FOR is passed through as the record
        // gave it and named as such, never read as a regular day.
        Some(other) => (other, "unnamed_code"),
    };
    (char::from(byte), word)
}

/// A special session's kind on the wire.
const fn special_word(kind: SpecialKind) -> &'static str {
    match kind {
        SpecialKind::Muhurat => "muhurat",
        SpecialKind::DisasterRecoveryDrill => "disaster_recovery_drill",
        SpecialKind::SystemsOutage => "systems_outage",
        SpecialKind::BudgetWeekend => "budget_weekend",
        SpecialKind::WeekendFullSession => "weekend_full_session",
    }
}

/// A phase on the wire: D-3320's words, `cas` for the closing auction.
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

/// The record's `calendar` and `events`; `halted` is the feed's bars inside
/// the day's halt, where counted (`events.halt.held`).
fn calendar_json(
    record: &DayRecord,
    venue: Venue,
    (feed, halted): (Vendor, Option<u64>),
) -> (Value, Value) {
    let (code, kind) = day_code(record.day);
    let phases = pull::calendar::phases_of(venue, record.day, CasEligibility::Unknown);
    let open = matches!(record.kind, DayKind::Open(_));
    let special = record.special.map(|special| {
        let windows: Vec<Value> = special
            .session
            .windows
            .iter()
            .take(usize::from(special.session.count))
            .map(|w| json!({ "from": hm(u32::from(w.from)), "to": hm(u32::from(w.to)) }))
            .collect();
        json!({
            "kind": special_word(special.kind),
            "label": special.kind.label(),
            "market_day": special.kind.is_market_day(),
            "windows": windows,
            "evidence": special.evidence,
        })
    });
    let calendar = json!({
        "code": code,
        "kind": kind,
        "weekday": pull::calendar::weekday_of(record.day).short(),
        "special": special,
        "holiday": record.holiday.map(|h| json!({ "name": h.name, "circular": h.circular })),
        "owed_minutes": open.then(|| phases.minutes(Phase::Continuous)),
        "halted_minutes": phases.minutes(Phase::Halted).saturating_add(phases.minutes(Phase::ReopeningAuction)),
        "unverified_minutes": open.then(|| phases.minutes(Phase::Unverified)),
        // A MEASURED, UNEXPLAINED SHORT DAY (WP-62 repair 3; DB-G47): the
        // vendors' own answers lack minutes the session owes. It classifies
        // nothing -- `owed_minutes` is unchanged -- and says what was read.
        // THE OBSERVATION IS THE MARKET'S, NOT THIS INSTRUMENT'S (the fourth
        // check's follow-up): the minutes were found missing from NIFTY's
        // answers, and the same object is on every instrument's day, a stock's
        // and an option's too. `observed_on` says whose minutes they were.
        "short_day": record.short.map(|short| json!({
            "standing": "measured; cause UNVERIFIED; the minutes stay owed",
            "observed_on": "market-wide: NIFTY's minutes (NSE-INDEX-NIFTY), not this instrument's own",
            "nifty_absent": { "from": hm(u32::from(short.nifty_absent.from)), "to": hm(u32::from(short.nifty_absent.to)) },
            "gdfl_checked": short.gdfl_checked,
            "evidence": short.evidence,
        })),
    });
    let halt = record.halt.map(|halt| {
        let seconds = halt.seconds(venue);
        json!({
            "standing": if halt.evidence.is_suspected() { "suspected" } else { "sourced" },
            "halted": { "from": hm(u32::from(halt.halted.from)), "to": hm(u32::from(halt.halted.to)) },
            "reopening": halt.reopening.map(|w| json!({ "from": hm(u32::from(w.from)), "to": hm(u32::from(w.to)) })),
            "seconds": { "from": hms(seconds.from), "resume": hms(seconds.resume) },
            // THE BARS HELD INSIDE IT (D-4928; WP-11c, DB-A07): kept as the vendor
            // printed them and counted here, so a flat run of an index's
            // repeated value is a labelled halt and not an ordinary hour.
            "held": halted.map(|bars| json!({
                "feed": feed.as_str(),
                "rung": finest(feed).as_str(),
                "bars": bars,
                "of_s": seconds.resume.saturating_sub(seconds.from),
            })),
            "evidence": halt.evidence.text(),
        })
    });
    let bands: Vec<Value> = phases
        .bands()
        .iter()
        .map(|band| {
            json!({
                "phase": phase_word(band.phase),
                "from": hm(u32::from(band.from)),
                "to": hm(u32::from(band.to)),
            })
        })
        .collect();
    (calendar, json!({ "halt": halt, "phases": bands }))
}

/// The sentence of a `sampled` that is one, or `None`.
fn sampled_says(sampled: &Value) -> Option<String> {
    if sampled.get("is") != Some(&Value::Bool(true)) {
        return None;
    }
    let text = |key: &str| sampled.get(key).and_then(Value::as_str).unwrap_or_default();
    let count = |key: &str| sampled.get(key).and_then(Value::as_u64);
    let who = if text("of") == "underlying" {
        format!("underlying index {}", text("series"))
    } else {
        "index".to_owned()
    };
    let held = match (count("held_s"), count("session_s")) {
        (Some(held), Some(of)) => percent(held, of).map_or_else(String::new, |p| {
            format!(", {held} of {of} session second(s) held ({p}%)")
        }),
        _ => String::new(),
    };
    Some(if text("basis") == "day_ledger" {
        format!(
            "{who}: minute-sampled{held}, its rows on {} distinct second(s) of the minute",
            count("ss_distinct").unwrap_or(0)
        )
    } else {
        format!(
            "{who}: minute-sampled or mostly missing (suspected){held}, at most {} a minute; \
             no day-ledger line measures its rows yet",
            pull::gdfl_import::quality::SAMPLED_SS_DISTINCT
        )
    })
}

/// The rule a `sampled` reads from the day ledger (D-3335's display rule;
/// D-4928 item 5), worded from the one threshold the import also reads.
fn sampled_rule() -> String {
    format!(
        "ss_distinct is {} or less, on a day with rows",
        pull::gdfl_import::quality::SAMPLED_SS_DISTINCT
    )
}

/// The rule a `sampled` reads from the held bars, where no line exists
/// (D-4928 item 6).
fn sampled_rule_held() -> String {
    let most = pull::gdfl_import::quality::SAMPLED_SS_DISTINCT;
    format!(
        "no day-ledger line, and the series holds at most {most} bars a session minute at a rung \
         of more than {most} a minute"
    )
}

/// The record's `degraded`: D-3335's display rules over the day's ledger
/// lines, each with the numbers it read. An index's rule is `sampled`'s
/// (WP-11c): the day ledger's line where there is one, the held bars where
/// there is none, for the instrument's own series or its underlying's.
fn degraded_json(lines: &[(ImportKind, Option<&QualityLine>)], sampled: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(says) = sampled_says(sampled) {
        let ledger = sampled.get("basis").and_then(Value::as_str) == Some("day_ledger");
        out.push(json!({
            "kind": ImportKind::Indices.as_str(),
            "flag": "sampled",
            "rule": if ledger { sampled_rule() } else { sampled_rule_held() },
            "says": says,
        }));
    }
    for (kind, line) in lines {
        let Some(line) = line else { continue };
        let count = |name: &str| line.count(name).unwrap_or(0);
        match kind {
            ImportKind::Options => {
                let traded = count("files_traded");
                let zero = count("zero_quote_files");
                let lead = count("ltq1_lead_files");
                if traded > 0
                    && (zero.saturating_mul(2) >= traded || lead.saturating_mul(2) >= traded)
                {
                    out.push(json!({
                        "kind": kind.as_str(),
                        "rule": "zero_quote_files or ltq1_lead_files is at least half of files_traded",
                        "says": format!("options: {zero} of {traded} traded files carry zero bid and ask, {lead} open with an LTQ=1 row"),
                    }));
                }
            }
            // AN INDEX DAY IS `sampled`'S, pushed above.
            ImportKind::Indices => {}
            ImportKind::Stocks => {
                if let Some(at) = line.clock("first_trade_p50")
                    && at >= 10 * 3_600 + 15 * 60
                {
                    out.push(json!({
                        "kind": kind.as_str(),
                        "rule": "first_trade_p50 is 10:15:00 or later",
                        "says": format!("stocks: the median first trade is at {}", hms(at)),
                    }));
                }
            }
        }
    }
    out
}

/// The junk codes on a Zerodha day (D-3335 §3), from its `1day` bar, the
/// previous held close and the bar counts. Never applied to a GDFL day.
fn junk_codes(
    record: &DayRecord,
    bar: DayBarAt,
    prev_close: Option<i64>,
    held: Held,
    peer: Held,
    segment: Segment,
) -> Vec<&'static str> {
    let mut codes = Vec::new();
    let day_bar = match bar {
        DayBarAt::Bar(bar) => Some(bar),
        _ => None,
    };
    if day_bar.is_some_and(|b| b.open <= 0 || b.high <= 0 || b.low <= 0 || b.close <= 0) {
        codes.push("zero_price");
    }
    let held_any = day_bar.is_some() || held.bars().is_some_and(|n| n > 0);
    if record.kind == DayKind::Closed && held_any {
        codes.push("off_calendar");
    }
    if let (Some(bar), Some(prev)) = (day_bar, prev_close)
        && bar.close > 0
        && prev > 0
    {
        let (close, prev) = (i128::from(bar.close), i128::from(prev));
        if close * JUMP_DEN >= prev * JUMP_NUM || prev * JUMP_DEN >= close * JUMP_NUM {
            codes.push("impossible_jump");
        }
    }
    if segment == Segment::Cash
        && day_bar.is_some_and(|b| b.volume == 0)
        && peer.bars().is_some_and(|n| n > 0)
    {
        codes.push("zero_volume_peer_traded");
    }
    codes
}

/// A junk code as a phrase.
const fn junk_phrase(code: &str) -> &'static str {
    match code.as_bytes() {
        b"zero_price" => "zero price",
        b"off_calendar" => "a bar on a day the exchange was closed",
        b"impossible_jump" => {
            "impossible jump: the close is at least 2.5 times, or at most 0.4 of, the previous close"
        }
        _ => "zero volume while GDFL traded",
    }
}

/// One day's standing against a refused front, and the pull refusals it names.
struct Front<'a> {
    /// The pull journal's refusals the record names.
    pulled: Vec<&'a PullRefusal>,
    /// The pull journal's word on a refused front.
    journal: Option<bool>,
    /// The record's `refused_front`.
    refused: Option<bool>,
    /// The record's `pull`.
    json: Option<Value>,
}

impl<'a> Context<'a> {
    /// `day`'s refused front, from the gap ledger and the pull journal.
    fn front(&self, day: Day, gap: Option<&GapLine>) -> Front<'a> {
        let symbol = self.asked.instrument.symbol.as_str();
        let pulled: Vec<&'a PullRefusal> = self
            .pull
            .map(|read| pull_view(&read.parsed, symbol, day, self.start))
            .unwrap_or_default();
        let before_first_bar = self.start.and_then(|start| start.before(day));
        // THE LEDGER'S WORD: unknown, not false, when the ledger is not there --
        // a ledger its writer has not written says nothing about the day.
        let ledger = self
            .gaps
            .filter(|read| read.present)
            .map(|_| gap.is_some_and(|g| g.why == "refused_front"));
        // THE PULL JOURNAL'S WORD (D-3315): a day before the series' first held
        // `1day` bar that a window refused before append holds. A day at or
        // after the first bar is never a front; before it, the journal must be
        // there to say whether the vendor's span was refused or never served.
        let present = self.pull.filter(|read| read.present);
        let journal = match before_first_bar {
            Some(false) => Some(false),
            Some(true) => present.map(|_| pulled.iter().any(|r| r.whole_window)),
            None => present.map(|_| false),
        };
        let refused = if self.asked.feed == Vendor::Zerodha {
            match (ledger, journal) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (None, None) => None,
                _ => Some(false),
            }
        } else {
            None
        };
        let json = self.pull.map(|read| {
            json!({
                "journal_present": read.present,
                "refusals": pulled
                    .iter()
                    .take(MAX_PULL_NAMED)
                    .map(|r| r.json())
                    .collect::<Vec<_>>(),
                "count": pulled.len(),
                "before_first_bar": before_first_bar,
            })
        });
        Front {
            pulled,
            journal,
            refused,
            json,
        }
    }
}

impl Context<'_> {
    /// One day's `sampled` ([`sampled_json`]), from the sampled series' own
    /// line: the index's, which for an option is its underlying's and is
    /// read for this alone -- `quality.indices` stays the instrument's own.
    /// A closed day is not sampled: `session_of` refuses it, and a line
    /// exists only for an imported day.
    fn sampled(&self, day: Day, series: Held) -> Value {
        let line = self.quality.zip(self.sampled_of).and_then(|(q, index)| {
            q.parsed
                .get(day, ImportKind::Indices, index.symbol.as_str())
        });
        sampled_json(&self.asked.instrument, self.sampled_of, line, series, day)
    }

    /// The day ledger's lines `day`'s record reads, by kind: the index's own
    /// for an index, and the whole kind-day's for stocks and options. Empty
    /// where the ledger was not read.
    fn quality_lines(&self, day: Day) -> Vec<(ImportKind, Option<&QualityLine>)> {
        let inst = &self.asked.instrument;
        self.quality
            .map(|q| {
                let symbol_scope = if inst.segment == Segment::Index {
                    Some(inst.symbol.as_str())
                } else {
                    None
                };
                vec![
                    (
                        ImportKind::Indices,
                        symbol_scope.and_then(|s| q.parsed.get(day, ImportKind::Indices, s)),
                    ),
                    (
                        ImportKind::Stocks,
                        q.parsed.get(day, ImportKind::Stocks, "*"),
                    ),
                    (
                        ImportKind::Options,
                        q.parsed.get(day, ImportKind::Options, "*"),
                    ),
                ]
            })
            .unwrap_or_default()
    }

    /// One day's record.
    fn record(&self, day: Day, held: &DayHeld, bar: Option<(DayBarAt, Option<i64>)>) -> Value {
        let (own, fills, peer) = (&held.own, held.fills, &held.peer);
        let asked = self.asked;
        let inst = &asked.instrument;
        let record = pull::calendar::record_of(i64::from(day.days_from_epoch()));
        let (calendar, events) = calendar_json(&record, inst.venue(), (asked.feed, held.halted));
        let import = self.journal.map(|rows| import_json(rows, inst, day));
        let quality_lines = self.quality_lines(day);
        let quality = self.quality.map(|_| {
            let line = |kind: ImportKind| {
                quality_lines
                    .iter()
                    .find_map(|(k, line)| (*k == kind).then_some(*line))
                    .flatten()
                    .map_or(Value::Null, QualityLine::json)
            };
            json!({
                "indices": line(ImportKind::Indices),
                "stocks": line(ImportKind::Stocks),
                "options": line(ImportKind::Options),
            })
        });
        let sampled = self.sampled(day, held.series);
        let degraded = degraded_json(&quality_lines, &sampled);
        let gap = self.gaps.and_then(|g| g.parsed.get(&inst.key(), day));
        let front = self.front(day, gap);
        let (junk, day_bar) = match bar {
            Some((at, prev)) => {
                let codes = junk_codes(&record, at, prev, *own, *peer, inst.segment);
                (Some(codes), Some(day_bar_json(at)))
            }
            None => (None, None),
        };
        let says = Says {
            record: &record,
            calendar: &calendar,
            import: import.as_ref(),
            quality: &quality_lines,
            degraded: &degraded,
            gap,
            junk: junk.as_deref(),
            pulled: &front.pulled,
            journal_present: self.pull.map(|read| read.present),
            journal_front: front.journal,
            start: self.start,
            day,
            segment: inst.segment,
            own: *own,
            fills,
            halted: held.halted,
            peer: *peer,
            feed: asked.feed,
        }
        .phrases();
        let mut held = own.json(asked.feed, finest(asked.feed));
        if let Some(fields) = held.as_object_mut() {
            fields.insert(
                "zero_volume".to_owned(),
                fills.map_or(Value::Null, Value::from),
            );
        }
        json!({
            "feed": asked.feed.as_str(),
            "instrument": inst.key(),
            "day": day.to_string(),
            "calendar": calendar,
            "events": events,
            "import": import,
            "quality": quality,
            "degraded": degraded,
            "sampled": sampled,
            "gaps": gap.map(GapLine::json),
            "junk": junk,
            "day_bar": day_bar,
            "refused_front": front.refused,
            "pull": front.json,
            "held": held,
            "peer": peer.json(peer_of(asked.feed), finest(peer_of(asked.feed))),
            "says": says,
        })
    }
}

/// A day's `1day` standing on the wire.
fn day_bar_json(at: DayBarAt) -> Value {
    match at {
        DayBarAt::Bar(bar) => json!({
            "held": true, "why": null,
            "o": bar.open, "h": bar.high, "l": bar.low, "c": bar.close, "v": bar.volume,
        }),
        DayBarAt::None => json!({ "held": false, "why": "no_bar" }),
        DayBarAt::NotHeld => json!({ "held": false, "why": "not_held" }),
        DayBarAt::Unreadable => json!({ "held": null, "why": "unreadable" }),
        DayBarAt::BeingWritten => json!({ "held": null, "why": "being_written" }),
    }
}

/// Everything a record's phrases are made from.
struct Says<'a> {
    record: &'a DayRecord,
    calendar: &'a Value,
    import: Option<&'a Value>,
    quality: &'a [(ImportKind, Option<&'a QualityLine>)],
    degraded: &'a [Value],
    gap: Option<&'a GapLine>,
    junk: Option<&'a [&'static str]>,
    /// The Zerodha pull journal's refusals this day's record names.
    pulled: &'a [&'a PullRefusal],
    /// Whether the pull journal is there; `None` when it was not read.
    journal_present: Option<bool>,
    /// The pull journal's word on a refused front (D-3315).
    journal_front: Option<bool>,
    start: Option<SeriesStart>,
    day: Day,
    segment: Segment,
    own: Held,
    /// The own feed's zero-volume bars that day, where counted (DB-B16).
    fills: Option<u64>,
    /// The own feed's bars inside the day's halt, where counted (DB-A07).
    halted: Option<u64>,
    peer: Held,
    feed: Vendor,
}

/// The longest phrase a refusal's own words are cut to.
const MAX_PHRASE_CHARS: usize = 140;

/// `text` cut to [`MAX_PHRASE_CHARS`] characters, marked when cut.
fn cut(text: &str) -> String {
    if text.chars().count() <= MAX_PHRASE_CHARS {
        return text.to_owned();
    }
    let mut kept: String = text.chars().take(MAX_PHRASE_CHARS).collect();
    kept.push_str("...");
    kept
}

/// The first refusal about another file of the day, as `about: why`, cut:
/// what a day the import failed or refused for other files names (DB-G69;
/// AF-02).
fn other_first(import: &Value) -> Option<String> {
    import
        .get("refusals")
        .and_then(|r| r.get("other_first"))
        .filter(|first| !first.is_null())
        .map(|first| {
            let field = |key: &str| first.get(key).and_then(Value::as_str).unwrap_or_default();
            cut(&format!("{}: {}", field("about"), field("why")))
        })
}

/// The directories this route reads the importer's refusals from, as a
/// sentence names them: `logs/cli`, then each GDFL kind's and the store
/// lock's (`crate::logs::import_dirs`).
fn logs_read() -> String {
    crate::logs::import_dirs()
        .map(|dir| format!("logs/{dir}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A close's `failures` against the refusals the logs this route reads
/// place in its own window ([`refusals_of`]): this instrument's and other
/// files'.
///
/// AF-02 repair 2: "none about this instrument" is said only when those
/// number at least the close's `failures`. A `cli` given `BRUTEX_LOG_DIR`
/// logs where it was told (`logs/cli-batch` for the operator's batch
/// script), so a close can count refusals that no directory read holds;
/// then the sentence says how many it holds and where it looked, and never
/// that none was this instrument's.
#[derive(Debug, Clone, Copy)]
struct Tally {
    /// The close's own count.
    failures: u64,
    /// Distinct refusals in its window about this instrument.
    this: u64,
    /// Distinct refusals in its window about another file of the kind-day.
    other: u64,
}

impl Tally {
    fn of(import: &Value) -> Self {
        let refusals = |key: &str| {
            import
                .get("refusals")
                .and_then(|r| r.get(key))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        Self {
            failures: import.get("failures").and_then(Value::as_u64).unwrap_or(0),
            this: refusals("this_count"),
            other: refusals("other_count"),
        }
    }

    /// The refusals the logs read place in the close's window.
    fn seen(self) -> u64 {
        self.this.saturating_add(self.other)
    }

    /// Whether the logs read hold every one the close counts.
    fn whole(self) -> bool {
        self.seen() >= self.failures
    }

    /// The ones the close counts that no directory read holds, and where it
    /// looked.
    fn unseen(self) -> String {
        let missing = self.failures.saturating_sub(self.seen());
        format!(
            "{missing} of them {} in none of the logs this route reads ({})",
            if missing == 1 { "is" } else { "are" },
            logs_read()
        )
    }
}

/// A refusal about this instrument logged in a LATER closing run's window
/// (AF-02 repair 2): never this close's, and never dropped. `None` when the
/// later windows hold none about it.
fn later_phrase(import: &Value) -> Option<String> {
    let refusals = import.get("refusals")?;
    let count = refusals
        .get("later_this_count")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0)?;
    let field = |key: &str| {
        refusals
            .get("later_this_first")
            .and_then(|first| first.get(key))
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    Some(format!(
        "the importer's log holds {count} refusal(s) about this instrument logged after a \
         later run closed this day for other files; the first is {}",
        cut(&format!("{}: {}", field("about"), field("why")))
    ))
}

/// What a `done` or `done_with_refusals` close says of its refusals.
///
/// * A CLOSE WHOSE REFUSALS ARE ALL ANOTHER FILE'S says so, as an incomplete
///   one does (DB-G69): the 277-name key's ten refusals on 2026-04-16 were
///   about other underlyings, and NIFTY's own contracts were imported
///   (AF-02, ingest-6). Only when the logs read hold all of them ([`Tally`]).
/// * A CLOSE WITH REFUSALS OF THIS INSTRUMENT'S OWN names how many and the
///   first, as a refused or incomplete one does: the phantom `*` row used to
///   name it, and the close it gave way to must not say less (AF-02). When
///   the logs read hold fewer than the close counts, the count is "at least".
/// * A CLOSE WHOSE REFUSALS THE LOGS READ DO NOT HOLD says how many are
///   missing and where it looked (AF-02 repair 2), never "none about this
///   instrument", and never another run's refusal as its first: the window
///   ends at the next closing run's start ([`refusals_of`]).
/// * A CLEAN CLOSE NEVER LEAVES THIS INSTRUMENT'S OWN REFUSALS UNSAID (AF-02
///   repair round 1): a refusal of the day about this instrument that no
///   other run's window holds (a close with no run id cannot place it
///   elsewhere) is said, never left in the JSON only.
fn closed_phrase(import: &Value, outcome: &str, first_refusal: Option<&str>) -> Option<String> {
    let tally = Tally::of(import);
    if outcome == "done" {
        return first_refusal.map(|first| {
            format!(
                "closed done, yet the importer's log holds {} refusal(s) of this day about this \
                 instrument not placed in another run; the first is {first}",
                tally.this
            )
        });
    }
    let head = format!("imported with {} vendor refusal(s)", tally.failures);
    Some(match (first_refusal, other_first(import)) {
        (Some(first), _) if tally.whole() => format!(
            "{head}, {} about this instrument; the first is {first}",
            tally.this
        ),
        (Some(first), _) => format!(
            "{head}, at least {} about this instrument; the first is {first}; {}",
            tally.this,
            tally.unseen()
        ),
        (None, Some(other)) if tally.whole() => {
            format!("{head}, none about this instrument; the first is {other}")
        }
        (None, Some(other)) => format!(
            "{head}; {} of them in the logs this route reads, none of those about this \
             instrument; the first is {other}; {}",
            tally.seen(),
            tally.unseen()
        ),
        (None, None) if tally.whole() => head,
        (None, None) => format!("{head}; {}", tally.unseen()),
    })
}

impl Says<'_> {
    /// The record's phrases, in a fixed order: the calendar, the import, the
    /// vendor's quality, Zerodha's ledger and junk, then the bars.
    fn phrases(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.calendar_phrases(&mut out);
        self.import_phrases(&mut out);
        self.quality_phrases(&mut out);
        for degraded in self.degraded {
            if let Some(says) = degraded.get("says").and_then(Value::as_str) {
                out.push(format!("degraded vendor day -- {says}"));
            }
        }
        self.gap_phrases(&mut out);
        self.pull_phrases(&mut out);
        for code in self.junk.unwrap_or_default() {
            out.push(junk_phrase(code).to_owned());
        }
        self.bar_phrases(&mut out);
        out
    }

    fn calendar_phrases(&self, out: &mut Vec<String>) {
        let record = self.record;
        match record.kind {
            DayKind::Closed => out.push(record.holiday.map_or_else(
                || {
                    if pull::calendar::weekday_of(record.day).is_weekend() {
                        "closed: a weekend".to_owned()
                    } else {
                        "closed".to_owned()
                    }
                },
                |h| format!("closed: {}", h.name),
            )),
            DayKind::Unmeasured => out.push("outside the calendar's span".to_owned()),
            DayKind::OpenLengthUnmeasured => out.push("a session of unmeasured length".to_owned()),
            DayKind::Open(_) => {}
        }
        if let Some(special) = record.special {
            let windows = self
                .calendar
                .get("special")
                .and_then(|s| s.get("windows"))
                .and_then(Value::as_array)
                .map(|windows| {
                    windows
                        .iter()
                        .map(|w| {
                            format!(
                                "{}-{}",
                                w.get("from").and_then(Value::as_str).unwrap_or_default(),
                                w.get("to").and_then(Value::as_str).unwrap_or_default()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" and ")
                })
                .unwrap_or_default();
            // THE LABEL IS THE CALENDAR'S, and some already end in "session"
            // ("Muhurat session"): the word is added only where it does not.
            let label = special.kind.label();
            let noun = if label.ends_with("session") {
                ""
            } else {
                " session"
            };
            out.push(format!("{label}{noun}, {windows}"));
        }
        if let Some(halt) = record.halt {
            let standing = if halt.evidence.is_suspected() {
                "suspected"
            } else {
                "sourced"
            };
            out.push(format!(
                "halt ({standing}) {}-{}",
                hm(u32::from(halt.halted.from)),
                hm(u32::from(halt.halted.to))
            ));
            self.halt_phrases(halt, out);
        }
    }

    /// What is held inside the day's halt (WP-11c, DB-A07): the feed's bars
    /// in this segment's own halt seconds, and what the import measured of
    /// the vendor's rows there. The bars are kept as printed; this labels
    /// them.
    fn halt_phrases(&self, halt: pull::calendar::Halt, out: &mut Vec<String>) {
        let venue = match self.segment {
            Segment::Index => Venue::NseIndex,
            Segment::Cash => Venue::NseCash,
            Segment::Fno => Venue::NseDerivatives,
        };
        let seconds = halt.seconds(venue);
        let window = format!("{} until {}", hms(seconds.from), hms(seconds.resume));
        if let Some(bars) = self.halted {
            let rung = finest(self.feed).as_str();
            out.push(if bars == 0 {
                format!("no {rung} bar held inside the halt, {window}")
            } else {
                format!(
                    "{bars} {rung} bar(s) held inside the halt, {window}: kept as printed, \
                     labelled, never dropped"
                )
            });
        }
        // THE IMPORT'S OWN MEASURE, from the line of this instrument's kind.
        let own_kind = match self.segment {
            Segment::Index => ImportKind::Indices,
            Segment::Cash => ImportKind::Stocks,
            Segment::Fno => ImportKind::Options,
        };
        let Some(line) = self.line(own_kind) else {
            return;
        };
        if let (Some(halt_s), Some(held_s)) = (line.count("halt_s"), line.count("halt_held_s"))
            && halt_s > 0
        {
            out.push(if own_kind == ImportKind::Indices {
                format!("the import placed {held_s} of the halt's {halt_s} session second(s)")
            } else {
                format!(
                    "{}: the day's files hold {held_s} second(s) inside the halt's {halt_s}",
                    own_kind.as_str()
                )
            });
        }
        if own_kind == ImportKind::Indices
            && let (Some(flat_s), Some(at)) = (line.count("flat_max_s"), line.clock("flat_max_at"))
        {
            out.push(format!(
                "the index value stood unchanged for {flat_s} s from {}",
                hms(at)
            ));
        }
    }

    fn import_phrases(&self, out: &mut Vec<String>) {
        let Some(import) = self.import else { return };
        let text = |key: &str| import.get(key).and_then(Value::as_str);
        let first_refusal = import
            .get("refusals")
            .and_then(|r| r.get("this"))
            .and_then(Value::as_array)
            .and_then(|this| this.first())
            .map(|first| {
                let about = first
                    .get("about")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let why = first.get("why").and_then(Value::as_str).unwrap_or_default();
                cut(&format!("{about}: {why}"))
            });
        let zero_ltp = self
            .line(ImportKind::Indices)
            .and_then(|line| line.count("zero_ltp_rows"))
            .filter(|n| *n > 0);
        let stage = text("stage").map(|s| format!(" ({s})")).unwrap_or_default();
        match text("outcome").unwrap_or("none") {
            "refused" => out.push(match (zero_ltp, first_refusal) {
                (Some(rows), _) => format!("refused at import{stage}: {rows} zero-LTP row(s)"),
                (None, Some(first)) if text("stage") != Some("calendar") => {
                    format!("refused at import{stage}: {first}")
                }
                _ => format!("refused at import{stage}"),
            }),
            "incomplete" => {
                // WHAT FAILED, AND WHETHER IT WAS THIS INSTRUMENT'S FILE: a day
                // incomplete for another contract still holds this one's bars
                // (DB-G69).
                // "NONE ABOUT THIS INSTRUMENT" ONLY WHEN THE LOGS READ HOLD
                // EVERY FAILURE, each placed in this close's own window (AF-02
                // repair 2, [`Tally`]).
                let other_first = other_first(import);
                let tally = Tally::of(import);
                let mut parts: Vec<String> =
                    text("reason").map(str::to_owned).into_iter().collect();
                match (first_refusal, other_first) {
                    (Some(first), _) => parts.push(first),
                    (None, Some(other)) if tally.whole() => parts.push(format!(
                        "{} failure(s), none about this instrument; the first is {other}",
                        tally.failures
                    )),
                    (None, Some(other)) => parts.push(format!(
                        "{} failure(s); {} of them in the logs this route reads, none of those \
                         about this instrument; the first is {other}; {}",
                        tally.failures,
                        tally.seen(),
                        tally.unseen()
                    )),
                    (None, None) => {}
                }
                let detail = if parts.is_empty() {
                    String::new()
                } else {
                    format!(": {}", parts.join("; "))
                };
                out.push(format!("incomplete at import{stage}{detail}"));
            }
            outcome @ ("done_with_refusals" | "done") => {
                out.extend(closed_phrase(import, outcome, first_refusal.as_deref()));
            }
            "open" => out.push("import begun and not closed".to_owned()),
            "recounted" => out.push("closed by the census recount".to_owned()),
            "none"
                if matches!(
                    self.record.kind,
                    DayKind::Open(_) | DayKind::OpenLengthUnmeasured
                ) =>
            {
                out.push(
                    if import.get("reached").and_then(Value::as_bool) == Some(true) {
                        "no import journal line for this day".to_owned()
                    } else {
                        "not reached by the import yet".to_owned()
                    },
                );
                // THE LOG NAMED THE DAY, AND ONLY OTHER FILES (AF-02): said,
                // never as this instrument's refusal.
                if let Some(other) = other_first(import) {
                    out.push(format!(
                        "the importer's log refused {} other file(s) of this day, none about \
                         this instrument; the first is {other}",
                        import
                            .get("refusals")
                            .and_then(|r| r.get("other_count"))
                            .and_then(Value::as_u64)
                            .unwrap_or(0)
                    ));
                }
            }
            _ => {}
        }
        out.extend(later_phrase(import));
        Self::history_and_late_phrases(import, out);
    }

    /// The import's phrases after its outcome: an earlier run's refusals,
    /// said as history (DB-B45), and the late rows deferred (DB-029).
    fn history_and_late_phrases(import: &Value, out: &mut Vec<String>) {
        let count = |key: &str| import.get(key).and_then(Value::as_u64);
        // AN EARLIER RUN'S REFUSALS ARE HISTORY (DB-B45): counted and said
        // as such, never as the day's current reason.
        if let Some(earlier) = import
            .get("refusals")
            .and_then(|r| r.get("superseded_count"))
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
        {
            out.push(match count("run") {
                Some(run) => format!(
                    "{earlier} refusal(s) logged by an earlier run are history, superseded by run {run}"
                ),
                None => format!("{earlier} refusal(s) logged by an earlier run are history"),
            });
        }
        if let Some(late) = count("late").filter(|n| *n > 0) {
            let back = count("max_back_s").unwrap_or(0);
            // 19,800 s IS IST'S OFFSET FROM UTC, to the second (DB-029): said as
            // the arithmetic fact it is, never as a diagnosis of the vendor.
            let offset = if back == IST_OFFSET_BACK_S {
                "; 19800 s is exactly IST's offset from UTC"
            } else {
                ""
            };
            out.push(format!(
                "{late} late row(s) deferred to a later second, at most {back} s back{offset}"
            ));
        }
    }

    /// The day's ledger line of one kind.
    fn line(&self, kind: ImportKind) -> Option<&QualityLine> {
        self.quality
            .iter()
            .find_map(|(k, line)| (*k == kind).then_some(*line))
            .flatten()
    }

    fn quality_phrases(&self, out: &mut Vec<String>) {
        if let Some(line) = self.line(ImportKind::Indices) {
            let n = |name: &str| line.count(name).unwrap_or(0);
            if n("holes_5s") > 0 {
                out.push(format!(
                    "{} in-session index hole(s) over 5 s, the longest {} s at {}",
                    n("holes_5s"),
                    n("hole_max_s"),
                    line.clock("hole_max_at")
                        .map_or_else(|| "-".to_owned(), hms)
                ));
            }
            if n("dual_spread_s") > 0 {
                out.push(format!(
                    "{} second(s) holding two values at least 10 bps apart, up to {} bps",
                    n("dual_spread_s"),
                    n("dual_spread_max_bps")
                ));
            }
        }
        if let Some(line) = self.line(ImportKind::Stocks) {
            let n = |name: &str| line.count(name).unwrap_or(0);
            if n("shared_holes") > 0 {
                out.push(format!(
                    "{} hole(s) of 300 s or more shared by 90% of traded shares, the longest {} s at {}",
                    n("shared_holes"),
                    n("shared_hole_max_s"),
                    line.clock("shared_hole_max_at").map_or_else(|| "-".to_owned(), hms)
                ));
            }
            if n("auction_prints") > 0 {
                out.push(format!(
                    "{} closing-auction print(s), kept apart from trades",
                    n("auction_prints")
                ));
            }
        }
        for (kind, line) in self.quality {
            let Some(line) = line else { continue };
            let n = |name: &str| line.count(name).unwrap_or(0);
            if n("rows_refused") > 0 {
                out.push(format!(
                    "{}: {} row(s) refused at import",
                    kind.as_str(),
                    n("rows_refused")
                ));
            }
            if n("ltq0_new_ltp_rows") > 0 {
                out.push(format!(
                    "{}: {} LTQ=0 row(s) carry a new price (flagged; bars unchanged)",
                    kind.as_str(),
                    n("ltq0_new_ltp_rows")
                ));
            }
        }
    }

    fn gap_phrases(&self, out: &mut Vec<String>) {
        if let Some(gap) = self.gap {
            // A SHARE'S MISSING MINUTE MAY BE ONE NOBODY TRADED (D-3321), said
            // wherever the line is shown: the store cannot tell it from a
            // vendor hole, and a repair lands only what the feed serves.
            let caveat = if gap.may_be_no_trade {
                "; these may be minutes nobody traded: the store cannot tell a no-trade minute from a vendor hole (D-3321), and a repair or a pull lands only the minutes the feed serves"
            } else {
                ""
            };
            out.push(match gap.why.as_str() {
                "vendor_empty" => format!(
                    "Zerodha minute gap: {} of {} owed minute(s) missing, the longest run {} from {}{caveat}",
                    gap.missing,
                    gap.owed,
                    gap.longest,
                    gap.longest_at.map_or_else(|| "-".to_owned(), |m| hm(m / 60))
                ),
                // A LINE FOUND FROM THE STORED MONTH, NOT FROM A PULL (R3-18,
                // `cli zerodha-gaps`): the store was read, the vendor was not,
                // so no vendor reason is claimed.
                "stored_short" => format!(
                    "Zerodha minute gap found in the stored month (no vendor call, R3-18): {} of {} owed minute(s) missing, the longest run {} from {}{}",
                    gap.missing,
                    gap.owed,
                    gap.longest,
                    gap.longest_at.map_or_else(|| "-".to_owned(), |m| hm(m / 60)),
                    if gap.may_be_no_trade {
                        caveat
                    } else {
                        "; a versioned repair or a pull fills it"
                    }
                ),
                "withheld_unmeasured_session" => {
                    "Zerodha withheld the day: a session of unmeasured length".to_owned()
                }
                "refused_day" => "Zerodha's pull refused the day".to_owned(),
                "refused_front" => {
                    "refused front: the vendor served this span and the pull refused it (D-3315)"
                        .to_owned()
                }
                other => format!("Zerodha ledger: {other}"),
            });
        }
    }

    /// The pull journal's refusals, and where the day stands against the
    /// series' first held `1day` bar (D-3315, D-3344). DB-053: a refused front
    /// and a span before the first bar with no refusal are named apart.
    fn pull_phrases(&self, out: &mut Vec<String>) {
        let first = match self.start {
            Some(SeriesStart::FirstBar(first)) => Some(first),
            _ => None,
        };
        let whole = self.pulled.iter().find(|r| r.whole_window);
        let before = self.start.and_then(|start| start.before(self.day));
        match (whole, first) {
            (Some(refusal), Some(first)) if self.journal_front == Some(true) => out.push(format!(
                "refused front of Zerodha's 1day series: a Zerodha pull refused its whole window \
                 before append ({}), and the first held 1day bar is {first} (D-3315)",
                refusal.shown()
            )),
            (_, Some(first)) if before == Some(true) => {
                out.push(if self.journal_present == Some(true) {
                    format!(
                        "before Zerodha's first held 1day bar, {first}; no pull refusal is \
                         recorded for this day"
                    )
                } else {
                    format!(
                        "before Zerodha's first held 1day bar, {first}; no pull journal could \
                         be read, so whether the vendor served this span is unknown"
                    )
                });
            }
            (Some(refusal), None) => out.push(format!(
                "a Zerodha pull refused a window holding this day before append ({}), and \
                 nothing of the series is held near it",
                refusal.shown()
            )),
            _ => {}
        }
        let mut dated = self.pulled.iter().filter(|r| !r.whole_window);
        if let Some(refusal) = dated.next() {
            let more = dated.count();
            out.push(format!(
                "a Zerodha pull refused rows of this day, run of {}: {}{}",
                refusal.run_day(),
                refusal.shown(),
                if more > 0 {
                    format!(" (and {more} more record(s))")
                } else {
                    String::new()
                }
            ));
        }
    }

    fn bar_phrases(&self, out: &mut Vec<String>) {
        let session = matches!(
            self.record.kind,
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured
        );
        let name = |feed: Vendor| {
            if feed == Vendor::Gdfl {
                "GDFL"
            } else {
                "Zerodha"
            }
        };
        // A GDFL ONE-SECOND OPTION OR SHARE IS TRADE-DRIVEN (D-3336 §2): a bar
        // exists for a traded second only, so a session without one is a day
        // nothing traded, not a hole, and is said so.
        // Said only on a day the import took: a day it refused or has not
        // reached holds no bar for that reason, already named above.
        let imported = self
            .import
            .and_then(|import| import.get("outcome"))
            .and_then(Value::as_str)
            .is_some_and(|outcome| !matches!(outcome, "refused" | "none" | "open"));
        let trade_driven = self.feed == Vendor::Gdfl && self.segment != Segment::Index && imported;
        match self.own {
            Held::Bars(0) if session && trade_driven => out.push(
                "no GDFL bar on this session: a one-second option or share bar exists only for \
                 a traded second (D-3336 §2)"
                    .to_owned(),
            ),
            Held::Bars(0) if session => {
                out.push(format!("no {} bar on this session", name(self.feed)));
            }
            Held::NoFile if session => {
                out.push(format!("{} holds no file for this month", name(self.feed)));
            }
            Held::Unreadable => out.push(format!("{}'s month could not be read", name(self.feed))),
            Held::BeingWritten => out.push(format!(
                "{}'s month is being written; ask again once the writer lets go",
                name(self.feed)
            )),
            _ => {}
        }
        // HELD IS NOT TRADED (DB-B16): a zero-volume candle is a minute
        // nobody traded in, and a day of them reads complete by count alone.
        if let (Held::Bars(held), Some(zero)) = (self.own, self.fills)
            && zero > 0
        {
            out.push(format!(
                "{zero} of {held} {} bar(s) carry zero volume: held, but no trade in that minute",
                name(self.feed)
            ));
        }
        let peer = peer_of(self.feed);
        match self.peer {
            Held::Bars(n) if n > 0 || session => {
                out.push(format!("{} holds {n} bar(s)", name(peer)));
            }
            Held::Unreadable => out.push(format!("{}'s month could not be read", name(peer))),
            Held::BeingWritten => out.push(format!(
                "{}'s month is being written; ask again once the writer lets go",
                name(peer)
            )),
            _ => {}
        }
    }
}

// ══ The envelope ════════════════════════════════════════════════════════════

/// One ledger read's state on the wire.
fn ledger_json<L>(
    read: &Result<LedgerRead<L>, String>,
    faults: impl Fn(&L) -> Value,
    writer: &str,
) -> Value {
    match read {
        Ok(read) => json!({
            "path": read.path.display().to_string(),
            "present": read.present,
            "bytes": read.bytes,
            "lines": read.lines,
            "read_now": read.read_now,
            "unterminated": read.unterminated,
            "faults": faults(&read.parsed),
            "writer": writer,
            "error": null,
        }),
        Err(why) => json!({ "present": null, "writer": writer, "error": why }),
    }
}

/// Where each source stood.
fn sources_json(
    journal: Option<&Result<JournalRows, String>>,
    quality: Option<&Result<LedgerRead<QualityLedger>, String>>,
    gaps: &Result<LedgerRead<GapLedger>, String>,
    pull: Option<&Result<PullRead, PullUnread>>,
) -> Value {
    let journal = journal.as_ref().map(|read| match read {
        Ok(rows) => {
            json!({ "route": "/imports.json?feed=gdfl", "journal": rows.journal, "error": null })
        }
        Err(why) => json!({ "route": "/imports.json?feed=gdfl", "journal": null, "error": why }),
    });
    let pull = pull.map(|read| match read {
        Ok(read) => json!({
            "path": read.path.display().to_string(),
            "present": read.present,
            "bytes": read.bytes,
            "records": read.records,
            "torn_bytes": read.torn,
            "read_now": read.read_now,
            "read_ms": read.read_micros / 1_000,
            "via": match read.via {
                Via::Absent => json!({ "kind": "absent" }),
                Via::Index { covered, runs } => json!({
                    "kind": "index",
                    "covered": covered,
                    "runs": runs,
                    "refusals_of": "the asked instrument only",
                }),
                Via::Walk { ref why } => json!({
                    "kind": "walk",
                    "why": why,
                    "refusals_of": "every instrument",
                }),
            },
            "zerodha_refusals": read.parsed.by_symbol.values().map(Vec::len).sum::<usize>(),
            "other_vendor_members": read.parsed.other_vendor,
            "faults": read.parsed.faults.json(),
            "kept": "Zerodha member failures whose note names a refused day or a window refused before append",
            "error": null,
        }),
        Err(unread) => unread.json(),
    });
    json!({
        "journal": journal,
        "quality": quality.as_ref().map(|read| ledger_json(read, |l: &QualityLedger| l.faults.json(), "WP-11b: the GDFL import, and `cli gdfl-dayflags` for days already imported")),
        "gaps": ledger_json(gaps, |l: &GapLedger| l.faults.json(), "WP-13b: the Zerodha minute pull, as each member lands"),
        "pull_journal": pull,
        "calendar": "pull::calendar::record_of and phases_of, the calendar of record (D-3319, D-3320)",
    })
}

/// The sentences an answer owes its reader about what it could not read.
fn source_notes(
    asked: &Asked,
    journal: Option<&Result<JournalRows, String>>,
    quality: Option<&Result<LedgerRead<QualityLedger>, String>>,
    gaps: &Result<LedgerRead<GapLedger>, String>,
) -> Vec<String> {
    let mut notes = Vec::new();
    if let Some(Err(why)) = journal {
        notes.push(format!(
            "The import journal could not be read, so every record's import is null: {why}"
        ));
    }
    match quality {
        Some(Ok(read)) if !read.present => notes.push(format!(
            "No GDFL day ledger at {} yet: WP-11b's import and `cli gdfl-dayflags` write it (WP-80 \
             step 6), so every record's quality is null -- not measured, not clean.",
            read.path.display()
        )),
        Some(Err(why)) => notes.push(format!(
            "The GDFL day ledger could not be read, so every record's quality is null: {why}"
        )),
        _ => {}
    }
    match gaps {
        Ok(read) if !read.present => notes.push(format!(
            "No Zerodha gap ledger at {} yet: WP-13b's pull writes it as each member lands, so \
             every record's gaps is null, and refused_front is judged from the pull journal \
             alone -- not measured, not clean.",
            read.path.display()
        )),
        Err(why) => notes.push(format!(
            "The Zerodha gap ledger could not be read, so every record's gaps is null: {why}"
        )),
        Ok(_) => {}
    }
    for (name, faults) in [
        (
            "GDFL day ledger",
            quality
                .as_ref()
                .and_then(|q| q.as_ref().ok())
                .map(|q| q.parsed.faults.count),
        ),
        (
            "Zerodha gap ledger",
            gaps.as_ref().ok().map(|g| g.parsed.faults.count),
        ),
    ] {
        if let Some(count) = faults.filter(|n| *n > 0) {
            notes.push(format!(
                "{count} line(s) of the {name} are not lines this reader names; each is listed with \
                 its reason under sources and left out of every record."
            ));
        }
    }
    if asked.feed == Vendor::Zerodha {
        notes.push(
            "Zerodha is pulled, not imported: its records carry no import journal (import is \
             null); a day its pull refused is the gap ledger's why=refused_day."
                .to_owned(),
        );
    } else {
        notes.push(
            "junk and day_bar are Zerodha's checks of its stored 1day bars and are null on a \
             GDFL record: GDFL's rows are checked at import (D-3318), and what it refused is \
             under import and quality. pull, Zerodha's pull-journal refusals, is read for a \
             feed=zerodha page only and is null here."
                .to_owned(),
        );
    }
    if asked.instrument.segment == Segment::Cash {
        notes.push(
            "A share's 15:15-15:29 reads unverified from 2026-08-03: this route does not read the \
             dated security master's closing-auction flag. /bars/window.json labels each bar's \
             phase (D-3320)."
                .to_owned(),
        );
    }
    notes
}

/// The display rules a page names beside what they flag.
fn rules_json() -> Value {
    json!({
        "degraded": {
            "options": "zero_quote_files or ltq1_lead_files is at least half of files_traded",
            "indices": "the record's `sampled` is one (see `sampled`)",
            "stocks": "first_trade_p50 is 10:15:00 or later",
            "source": "D-3335: display rules, chosen, not sourced",
        },
        "sampled": {
            "day_ledger": sampled_rule(),
            "held_bars": sampled_rule_held(),
            "of": "own: the instrument is the index; underlying: the instrument is an option and the index is the one the alias table gives its symbol (D-3332: NSE-INDEX-NIFTYFINSERVICE for a FINNIFTY option), read under that store key",
            "standing": "measured from the vendor's rows by the import or `cli gdfl-dayflags`; suspected from the bars held, which cannot tell a sampled day from one mostly missing",
            "null": format!(
                "a share, an option whose underlying is a share (any symbol the alias table does not name as an index), a feed whose finest rung holds at most {} bars a minute, a day that is not a session, or an index with neither a day-ledger line nor a month the store holds",
                pull::gdfl_import::quality::SAMPLED_SS_DISTINCT
            ),
            "source": "D-3335's index rule, named by D-4928 (WP-11c, DB-A04); chosen, not sourced",
        },
        "halt_held": "events.halt.held counts the feed's bars that open inside the halt's seconds for the instrument's own segment (from, up to resume); bars are kept as printed; null where the month is not held, or where the count could not be read and a note says why (D-4928; WP-11c, DB-A07)",
        "junk": {
            "zero_price": "the day's Zerodha 1day bar has an open, high, low or close at or below zero",
            "off_calendar": "Zerodha holds a bar on a day the calendar of record says was closed",
            "impossible_jump": "the 1day close is at least 2.5 times, or at most 0.4 of, the previous held 1day close",
            "zero_volume_peer_traded": "a share's Zerodha 1day bar has volume 0 and GDFL holds 1s bars that day (a GDFL share bar is a traded second)",
            "source": "chosen, not sourced; flags only, the bars are served unchanged (DB-031)",
        },
        "calendar_code": {
            ".": "closed", "R": "regular session", "X": "regular, with the closing auction for eligible shares",
            "P": "special session with recorded windows", "U": "session of unmeasured length",
            "H": "session with a suspected halt", "-": "outside the calendar's span",
            "source": "D-3336",
        },
        "refused_front": {
            "true": "the Zerodha gap ledger says why=refused_front, or the day is before the series' first held 1day bar and a Zerodha pull refused a window holding it before append (D-3315)",
            "false": "the day is at or after the series' first held 1day bar, or a source that would name the refusal was read and names none",
            "null": "a GDFL record, or neither the gap ledger nor the pull journal is there to say",
        },
        "pull": "a Zerodha member failure is named on a day it dates, and a whole-window refusal only on a day no 1day bar of the series is held up to",
        "windows": "from and to are minute opens, both inclusive",
    })
}

/// AF-02 (ingest-6): a journal row is an instrument's when its filter names
/// the symbol. Its own file, so the packages appending to `tests` merge
/// beside it.
#[cfg(test)]
#[path = "dayflags_filter_tests.rs"]
mod filter_tests;

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;
    use crate::server::tests::{agreeing, site};
    use std::fmt::Write as _;
    use store::format::{Bar, OI_NULL};

    // EVERY FIXTURE BELOW IS INVENTED. Prices are a few rupees, volumes are
    // small round numbers, and no row is a vendor's: the shapes are the
    // findings', the numbers are not the market's.

    /// **A MEASURED SHORT DAY IS SAID, AND STILL OWES ITS MINUTES** (WP-62
    /// repair 3; DB-G47): 2019-08-02 and 2022-03-07 read as plain sessions
    /// with no word. The day's calendar object now carries the calendar of
    /// record's entry and its evidence; `owed_minutes` is the full session's.
    #[test]
    fn a_measured_short_day_is_on_the_calendar_object_and_still_owes_its_minutes() {
        let day = |y, m, d| i64::from(Day::new(y, m, d).expect("a day").days_from_epoch());
        let (calendar, _) = calendar_json(
            &pull::calendar::record_of(day(2019, 8, 2)),
            Venue::NseIndex,
            (Vendor::Zerodha, None),
        );
        assert_eq!(calendar["short_day"]["nifty_absent"]["from"], "13:59");
        assert_eq!(calendar["short_day"]["gdfl_checked"], true);
        assert_eq!(calendar["owed_minutes"], 375);
        // Whose minutes they were, on any instrument's day.
        assert!(
            calendar["short_day"]["observed_on"]
                .as_str()
                .is_some_and(|said| said.starts_with("market-wide: NIFTY's minutes")),
            "{calendar}"
        );
        let (calendar, _) = calendar_json(
            &pull::calendar::record_of(day(2022, 3, 7)),
            Venue::NseIndex,
            (Vendor::Zerodha, None),
        );
        assert_eq!(calendar["short_day"]["nifty_absent"]["to"], "10:24");
        assert_eq!(calendar["short_day"]["gdfl_checked"], true);
        assert_eq!(calendar["short_day"]["nifty_absent"]["from"], "10:22");
        assert_eq!(calendar["owed_minutes"], 375);
        let (calendar, _) = calendar_json(
            &pull::calendar::record_of(day(2022, 3, 8)),
            Venue::NseIndex,
            (Vendor::Zerodha, None),
        );
        assert_eq!(calendar["short_day"], Value::Null);
    }

    /// A store root of its own.
    fn root_of(name: &str) -> PathBuf {
        let root = crate::scratch::path(&format!("dayflags-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("a scratch root");
        root
    }

    fn day(text: &str) -> Day {
        super::super::import_journal::parse_day(text).expect("a day")
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }

    fn append(path: &Path, text: &str) {
        use std::io::Write as _;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("open for append");
        file.write_all(text.as_bytes()).expect("append");
    }

    /// The importer's `refused` events, as the `cli` ring holds them.
    fn cli_log(root: &Path, refusals: &[(&str, &str, &str, &str)]) {
        let mut text = String::new();
        for (seq, (kind, on, about, why)) in refusals.iter().enumerate() {
            let _ = writeln!(
                text,
                "{{\"seq\":{seq},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{},\"level\":\"error\",\
                 \"target\":\"gdfl.import\",\"msg\":\"refused\",\"fields\":{{\"kind\":\"{kind}\",\
                 \"day\":\"{on}\",\"about\":\"{about}\",\"why\":\"{why}\"}}}}",
                1_791_171_078_967_u64 + u64::try_from(seq).unwrap()
            );
        }
        write(&root.join("logs").join("cli").join("events.ndjson"), &text);
    }

    /// One bar at `second` of the IST day `on`.
    fn bar(on: &str, second: i64, ohlc: [i64; 4], volume: i64) -> Bar {
        Bar {
            ts_micros: ist_midnight_micros(day(on)) + second * 1_000_000,
            open: ohlc[0],
            high: ohlc[1],
            low: ohlc[2],
            close: ohlc[3],
            volume,
            open_interest: OI_NULL,
        }
    }

    /// `n` unchanged-price one-minute bars from 09:15 IST on `on`.
    fn minutes(on: &str, n: i64) -> Vec<Bar> {
        (0..n)
            .map(|i| bar(on, 9 * 3_600 + 15 * 60 + i * 60, [500; 4], 0))
            .collect()
    }

    /// Files `bars` for `key` on `feed` at `rung`, one month file per month,
    /// oldest first.
    fn file_bars(root: &Path, feed: Vendor, key: &str, rung: Timeframe, mut bars: Vec<Bar>) {
        let inst = instrument_of(key, "").expect("a key");
        bars.sort_by_key(|bar| bar.ts_micros);
        let mut at = 0;
        while at < bars.len() {
            let on = pull::session::IstMoment::from_epoch_secs(bars[at].ts_micros / 1_000_000)
                .unwrap()
                .day();
            let month = on.year_month().unwrap();
            let run = bars[at..]
                .iter()
                .take_while(|bar| {
                    pull::session::IstMoment::from_epoch_secs(bar.ts_micros / 1_000_000)
                        .unwrap()
                        .day()
                        .year_month()
                        .unwrap()
                        == month
                })
                .count();
            let path = StorePath::new(PathParts {
                vendor: feed,
                exchange: inst.exchange.as_str(),
                segment: inst.segment.as_str(),
                symbol: &inst.symbol,
                contract: inst.contract,
                timeframe: rung,
                month,
                file: FileKind::Bars,
            })
            .expect("a legal path");
            let mut file =
                BarFile::open_or_create(root, path, symbol_id(&inst.symbol)).expect("a bar file");
            file.append(&bars[at..at + run]).expect("the bars append");
            at += run;
        }
    }

    fn ask(query: &str) -> Asked {
        asked(query).unwrap_or_else(|why| panic!("{query}: {why}"))
    }

    /// The record of `on` in an answer.
    fn record<'a>(got: &'a Value, on: &str) -> &'a Value {
        got["days"]
            .as_array()
            .expect("days")
            .iter()
            .find(|record| record["day"] == on)
            .unwrap_or_else(|| panic!("{on} in {got:#}"))
    }

    fn says(record: &Value) -> Vec<String> {
        record["says"]
            .as_array()
            .expect("says")
            .iter()
            .map(|s| s.as_str().expect("a phrase").to_owned())
            .collect()
    }

    fn says_has(record: &Value, phrase: &str) {
        let said = says(record);
        assert!(
            said.iter().any(|s| s == phrase),
            "{phrase:?} not in {said:#?}"
        );
    }

    /// One complete `indices` line of the GDFL day ledger.
    fn index_flags(on: &str, symbol: &str, set: &[(&str, &str)]) -> String {
        flags_line("indices", on, symbol, &INDICES_FIELDS, set)
    }

    fn flags_line(
        kind: &str,
        on: &str,
        scope: &str,
        own: &[(&str, Shape)],
        set: &[(&str, &str)],
    ) -> String {
        let mut line = format!("flags {kind} {on} {scope} v=1 src=rescan definition=2");
        for (name, shape) in EVERY_KIND.iter().chain(own) {
            let value = set
                .iter()
                .find_map(|(n, v)| (n == name).then_some(*v))
                .unwrap_or(if *shape == Shape::Clock { "-" } else { "0" });
            let _ = write!(line, " {name}={value}");
        }
        line.push('\n');
        line
    }

    // ── The operator's checks, on fixtures in D-3335's shape ──────────────

    /// **2022-03-07: REFUSED AT IMPORT, FOUR ZERO-LTP ROWS, AND THE OTHER
    /// FEED'S COUNT.** The day left no journal line (DB-025): it is read from
    /// the importer's log, its refusal about NIFTY's own file is named and
    /// BANKNIFTY's is counted apart, the ledger's `zero_ltp_rows` gives the
    /// phrase, GDFL holds no bar and Zerodha 372. The month around it reads
    /// done, closed on the weekend, and counts both feeds. DB-016, DB-025.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one fixture and every assertion about it, read top to bottom"
    )]
    fn the_refused_index_day_names_its_zero_ltp_rows_and_the_zerodha_count() {
        let root = root_of("refused-index");
        write(
            &pull::gdfl_import::journal_path(&root),
            "begin indices 2022-03-04 *\n\
             done indices 2022-03-04 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
             begin indices 2022-03-08 *\n\
             done indices 2022-03-08 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
        );
        let nifty_why =
            "line 9: LTP is not a decimal of at most two decimal places and at least one tick";
        let bank_why =
            "line 7: LTP is not a decimal of at most two decimal places and at least one tick";
        cli_log(
            &root,
            &[
                ("indices", "2022-03-07", "NIFTY 50.NSE_IDX.csv", nifty_why),
                ("indices", "2022-03-07", "NIFTY BANK.NSE_IDX.csv", bank_why),
                // A SECOND RUN refused the same files the same way.
                ("indices", "2022-03-07", "NIFTY 50.NSE_IDX.csv", nifty_why),
                ("indices", "2022-03-07", "NIFTY BANK.NSE_IDX.csv", bank_why),
            ],
        );
        write(
            &quality_path(&root),
            &index_flags(
                "2022-03-07",
                "NIFTY",
                &[
                    ("zero_ltp_rows", "4"),
                    ("rows", "90"),
                    ("ss_distinct", "60"),
                ],
            ),
        );
        let mut zerodha = minutes("2022-03-04", 375);
        zerodha.extend(minutes("2022-03-07", 372));
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-INDEX-NIFTY",
            Timeframe::MINUTE_1,
            zerodha,
        );
        let gdfl: Vec<Bar> = (0..3)
            .map(|i| bar("2022-03-04", 9 * 3_600 + 15 * 60 + i, [500; 4], 0))
            .collect();
        file_bars(
            &root,
            Vendor::Gdfl,
            "NSE-INDEX-NIFTY",
            Timeframe::SECOND_1,
            gdfl,
        );

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03&to=2022-03"),
        );
        assert_eq!(got["days"].as_array().unwrap().len(), 31, "{got:#}");
        assert_eq!(got["page_from"], "2022-03-01");
        assert_eq!(got["page_to"], "2022-03-31");
        let refused = record(&got, "2022-03-07");
        assert_eq!(refused["calendar"]["code"], "R");
        assert_eq!(refused["import"]["outcome"], "refused");
        assert_eq!(refused["import"]["from"], "log");
        assert_eq!(refused["import"]["refusals"]["this_count"], 1);
        assert_eq!(refused["import"]["refusals"]["other_count"], 1);
        assert_eq!(
            refused["import"]["refusals"]["this"][0]["about"],
            "NIFTY 50.NSE_IDX.csv"
        );
        assert_eq!(refused["quality"]["indices"]["zero_ltp_rows"], 4);
        assert_eq!(refused["held"]["bars"], 0);
        assert_eq!(refused["peer"]["feed"], "zerodha");
        assert_eq!(refused["peer"]["rung"], "1min");
        assert_eq!(refused["peer"]["bars"], 372);
        assert_eq!(refused["junk"], Value::Null, "junk is Zerodha's");
        says_has(refused, "refused at import: 4 zero-LTP row(s)");
        says_has(refused, "no GDFL bar on this session");
        says_has(refused, "Zerodha holds 372 bar(s)");

        let done = record(&got, "2022-03-04");
        assert_eq!(done["import"]["outcome"], "done");
        assert_eq!(done["import"]["from"], "journal");
        assert_eq!(done["held"]["bars"], 3);
        assert_eq!(done["peer"]["bars"], 375);
        assert!(says(done).iter().all(|s| !s.starts_with("refused")));

        let weekend = record(&got, "2022-03-05");
        assert_eq!(weekend["calendar"]["code"], ".");
        assert_eq!(weekend["calendar"]["owed_minutes"], Value::Null);
        assert_eq!(says(weekend), ["closed: a weekend"]);

        // A DAY AFTER THE JOURNAL'S LAST is not reached; one inside it with no
        // line is a hole the journal does not explain.
        says_has(record(&got, "2022-03-09"), "not reached by the import yet");
        // The same day for BANKNIFTY names BANKNIFTY's own file, without the
        // NIFTY ledger line, and no zero-LTP count it does not have.
        let bank = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-BANKNIFTY&from=2022-03-07"),
        );
        let bank = record(&bank, "2022-03-07");
        assert_eq!(
            bank["import"]["refusals"]["this"][0]["about"],
            "NIFTY BANK.NSE_IDX.csv"
        );
        assert_eq!(bank["quality"]["indices"], Value::Null);
        says_has(
            bank,
            &format!("refused at import: NIFTY BANK.NSE_IDX.csv: {bank_why}"),
        );
        assert_eq!(bank["peer"]["why"], "not_held");
    }

    /// **2024-03-02 READS "DR DRILL, REFUSED" WITH ZERODHA'S 105**, and
    /// **2020-03-13 READS "HALT (SUSPECTED)"**: the calendar of record names
    /// the drill's two windows and the halt's minutes, the import's calendar
    /// refusal settles its stage, and the halt's minutes are not owed.
    /// DB-016, DB-G47, DB-G87.
    #[test]
    fn the_drill_day_is_special_and_refused_and_the_march_halt_is_suspected() {
        let root = root_of("drill-halt");
        write(
            &pull::gdfl_import::journal_path(&root),
            "begin indices 2020-03-13 *\n\
             done indices 2020-03-13 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0\n\
             begin indices 2024-03-04 *\n\
             done indices 2024-03-04 * files=2 seconds=3 failures=0 late=0 late_unresolved=0 max_back_s=0\n",
        );
        cli_log(
            &root,
            &[(
                "indices",
                "2024-03-02",
                "calendar",
                "the calendar says Open(two windows), not a regular session",
            )],
        );
        let mut drill = minutes("2024-03-02", 45);
        drill
            .extend((0..60).map(|i| bar("2024-03-02", 11 * 3_600 + 30 * 60 + i * 60, [500; 4], 0)));
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-INDEX-NIFTY",
            Timeframe::MINUTE_1,
            drill,
        );

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-03-02"),
        );
        assert_eq!(
            got["days"].as_array().unwrap().len(),
            1,
            "a day is one record"
        );
        let drill = record(&got, "2024-03-02");
        assert_eq!(drill["calendar"]["code"], "P");
        assert_eq!(
            drill["calendar"]["special"]["kind"],
            "disaster_recovery_drill"
        );
        assert_eq!(drill["calendar"]["owed_minutes"], 105);
        assert_eq!(drill["import"]["outcome"], "refused");
        assert_eq!(drill["import"]["stage"], "calendar");
        assert_eq!(drill["peer"]["bars"], 105);
        says_has(
            drill,
            "disaster-recovery drill session, 09:15-09:59 and 11:30-12:29",
        );
        let muhurat = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2025-10-21"),
        );
        says_has(
            record(&muhurat, "2025-10-21"),
            "Muhurat session, 13:45-14:44",
        );
        says_has(drill, "refused at import (calendar)");
        says_has(drill, "Zerodha holds 105 bar(s)");

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-03-13"),
        );
        let halt = record(&got, "2020-03-13");
        assert_eq!(halt["calendar"]["code"], "H");
        assert_eq!(halt["events"]["halt"]["standing"], "suspected");
        assert_eq!(halt["events"]["halt"]["halted"]["from"], "09:22");
        assert_eq!(
            halt["events"]["halt"]["seconds"]["from"], "09:21:10",
            "the index's own seconds"
        );
        assert_eq!(halt["calendar"]["owed_minutes"], 375 - 59);
        assert_eq!(halt["calendar"]["halted_minutes"], 59);
        assert_eq!(halt["import"]["outcome"], "done");
        says_has(halt, "halt (suspected) 09:22-10:20");
        let phases: Vec<&str> = halt["events"]["phases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|band| band["phase"].as_str().unwrap())
            .collect();
        assert!(
            phases.contains(&"halted") && phases.contains(&"reopening_auction"),
            "{phases:?}"
        );
    }

    /// **AN EARLIER RUN'S REFUSAL IS HISTORY, NOT THE DAY'S REASON** (DB-B45).
    /// 2024-03-02's indices were refused by a build that called the drill "not
    /// a regular session", then admitted and refused at filing by run
    /// 1791602895, which closed the day `incomplete`. The importer's events
    /// carry no run id; the run line's `started` splits them: the filing
    /// refusal is current, the calendar one is history with no run line of its
    /// own (logged before the journal recorded runs), and the phrases lead with
    /// the filing.
    #[test]
    fn an_earlier_runs_refusal_is_history_and_the_newest_run_is_current() {
        let root = root_of("runs");
        write(
            &pull::gdfl_import::journal_path(&root),
            "run 1791602895 source=zip archive=/tmp/a.zip archive_bytes=7 commit=abc started=1791602895 archive_cd_blake3=00\n\
             begin indices 2024-03-02 *\n\
             incomplete indices 2024-03-02 * definition=2 files=8 seconds=0 failures=2 late=0 late_unresolved=0 max_back_s=0 stage=file reason=filing run=1791602895\n",
        );
        let event = |seq: u64, ms: u64, about: &str, why: &str| {
            format!(
                "{{\"seq\":{seq},\"ts\":\"2026-10-05T03:31:18.967Z\",\"ms\":{ms},\"level\":\"error\",\
                 \"target\":\"gdfl.import\",\"msg\":\"refused\",\"fields\":{{\"kind\":\"indices\",\
                 \"day\":\"2024-03-02\",\"about\":\"{about}\",\"why\":\"{why}\"}}}}\n"
            )
        };
        write(
            &root.join("logs").join("cli").join("events.ndjson"),
            &format!(
                "{}{}{}",
                event(
                    1,
                    1_791_171_132_660,
                    "calendar",
                    "the calendar says Open(two windows), not a regular session"
                ),
                event(
                    2,
                    1_791_271_117_373,
                    "calendar",
                    "the calendar says Open(two windows), not a regular session"
                ),
                event(
                    3,
                    1_791_602_900_000,
                    "NIFTY 50.NSE_IDX.csv",
                    "the store refused to file the bars"
                ),
            ),
        );
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-03-02"),
        );
        let drill = record(&got, "2024-03-02");
        assert_eq!(drill["import"]["outcome"], "incomplete");
        assert_eq!(drill["import"]["run"], 1_791_602_895_u64);
        let refusals = &drill["import"]["refusals"];
        assert_eq!(refusals["this_count"], 1, "{refusals:#}");
        assert_eq!(refusals["this"][0]["about"], "NIFTY 50.NSE_IDX.csv");
        assert_eq!(
            refusals["superseded_count"], 1,
            "two log lines, one distinct refusal"
        );
        assert_eq!(refusals["superseded"][0]["about"], "calendar");
        assert_eq!(
            refusals["superseded"][0]["run"],
            Value::Null,
            "logged before the journal recorded any run line"
        );
        assert_eq!(refusals["superseded"][0]["at_ms"], 1_791_171_132_660_u64);
        let said = says(drill);
        assert!(
            said.iter()
                .any(|s| s.starts_with("incomplete at import (file): filing; NIFTY 50.NSE_IDX.csv")),
            "{said:#?}"
        );
        assert!(
            !said.iter().any(|s| s.contains("not a regular session")),
            "the earlier build's reason is not the current one: {said:#?}"
        );
        says_has(
            drill,
            "1 refusal(s) logged by an earlier run are history, superseded by run 1791602895",
        );
    }

    /// **A REFUSAL LOGGED BEFORE THE CLOSING RUN STARTED IS ATTRIBUTED TO THE
    /// RUN THAT WAS RUNNING** (DB-B45): the latest run line started before
    /// it, with that run's commit; a row with no `run=` keeps every refusal
    /// current, since nothing says which run logged it.
    #[test]
    fn an_earlier_refusal_names_the_run_that_logged_it() {
        let runs = vec![
            RunLine {
                id: 10,
                started: 10,
                commit: Some("aaa".to_owned()),
            },
            RunLine {
                id: 20,
                started: 20,
                commit: None,
            },
        ];
        let row = json!({
            "run": 20,
            "logged": [
                { "about": "calendar", "why": "old", "at_ms": 15_000 },
                { "about": "calendar", "why": "new", "at_ms": 21_000 },
            ],
        });
        let inst = instrument_of("NSE-INDEX-NIFTY", "").expect("a key");
        let split = refusals_of(&row, &inst, day("2024-03-02"), &runs, None);
        assert_eq!(split.this, vec![("calendar".to_owned(), "new".to_owned())]);
        assert_eq!(split.superseded.len(), 1);
        assert_eq!(split.superseded[0].run, Some(10));
        assert_eq!(split.superseded[0].commit.as_deref(), Some("aaa"));
        // A row with no `run=` keeps every refusal current: nothing says which run logged it.
        let unrun = json!({ "logged": [{ "about": "calendar", "why": "old", "at_ms": 15_000 }] });
        let kept = refusals_of(&unrun, &inst, day("2024-03-02"), &runs, None);
        assert_eq!(kept.this_count, 1);
        assert_eq!(kept.superseded_count, 0);
    }

    /// **A CONTRACT RENAMED AT AN EX-DATE NAMES WHERE IT CONTINUES** (DB-G72,
    /// D-3331 item 4). The import's lineage ledger links the pre-ex key to the
    /// adjusted one; the answer names the link on both keys, a `stitched`
    /// key as stitched, and a ledger not written yet as absent with its
    /// reason, never as "never renamed". An index or a Zerodha page carries
    /// no lineage at all. The keys are invented (`ZZBRIT`), the shape is
    /// BRITANNIA 2018-11-29's: strikes halved, the lot doubled.
    #[test]
    fn a_renamed_contract_names_the_key_it_continues_as() {
        let root = root_of("lineage");
        let old_key = "NSE-FNO-ZZBRIT-2018-12-27-600000-CE";
        let new_key = "NSE-FNO-ZZBRIT-2018-12-27-300000-CE";
        let asked_old = ask(&format!("feed=gdfl&instrument={old_key}&from=2018-11-28"));
        let none = answer(&root, &asked_old);
        assert_eq!(none["lineage"]["present"], false);
        assert!(
            none["lineage"]["why"]
                .as_str()
                .is_some_and(|why| why.contains("no lineage ledger")),
            "{:#}",
            none["lineage"]
        );
        let (under, old) = pull::gdfl_lineage::parse_option_key(old_key).expect("an option key");
        let (_, new) = pull::gdfl_lineage::parse_option_key(new_key).expect("an option key");
        let (_, stitched) =
            pull::gdfl_lineage::parse_option_key("NSE-FNO-ZZBRIT-2018-12-27-310000-CE")
                .expect("an option key");
        let lineage = pull::gdfl_lineage::LineageRecord {
            underlying: under,
            old,
            new,
            ex_date: day("2018-11-29"),
            factor: (2, 1),
            lot_old: 100,
            lot_new: 200,
            oi_unadjusted_until: 0,
        };
        let stitch = pull::gdfl_lineage::StitchedRecord {
            underlying: under,
            key: stitched,
            ex_date: day("2018-11-29"),
            before: None,
            after: Some(old),
        };
        write(
            &pull::gdfl_lineage::lineage_path(&root),
            &format!("{}\n{}\nnot a record\n", lineage.line(), stitch.line()),
        );
        let got = answer(&root, &asked_old);
        let records = &got["lineage"]["records"];
        assert_eq!(got["lineage"]["present"], true);
        assert_eq!(got["lineage"]["unreadable_lines"], 1);
        assert_eq!(records[0]["kind"], "continues_as", "{records:#}");
        assert_eq!(records[0]["key"], new_key);
        assert_eq!(records[0]["ex"], "2018-11-29");
        assert_eq!(records[0]["factor"], "2/1");
        let back = answer(
            &root,
            &ask(&format!("feed=gdfl&instrument={new_key}&from=2018-11-29")),
        );
        assert_eq!(back["lineage"]["records"][0]["kind"], "renamed_from");
        assert_eq!(back["lineage"]["records"][0]["key"], old_key);
        let joined = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-FNO-ZZBRIT-2018-12-27-310000-CE&from=2018-11-29"),
        );
        assert_eq!(joined["lineage"]["records"][0]["kind"], "stitched");
        assert_eq!(joined["lineage"]["records"][0]["after"], old_key);
        assert_eq!(joined["lineage"]["records"][0]["before"], Value::Null);
        let index = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2018-11-29"),
        );
        assert_eq!(index["lineage"], Value::Null, "an index is never renamed");
        let zerodha = answer(
            &root,
            &ask(&format!(
                "feed=zerodha&instrument={old_key}&from=2018-11-28"
            )),
        );
        assert_eq!(
            zerodha["lineage"],
            Value::Null,
            "only GDFL keeps a lineage ledger"
        );
    }

    /// **AN EVENT AND ITS STOPPED KEYS ARE READ AND SHOWN** (D-3355;
    /// DB-B13): a re-listing pairs nothing, so the `event` record of its
    /// underlying and the `stopped` record of each retired key are the only
    /// lines that say what happened. Neither is an unreadable line; the key
    /// carries `stopped`, and every key of the underlying carries the event.
    #[test]
    fn an_event_and_its_stopped_keys_are_read_and_shown() {
        let root = root_of("lineage-event");
        let stopped = "NSE-FNO-ZZIDX-2020-06-25-1000000-CE";
        let other = "NSE-FNO-ZZIDX-2020-06-25-1100000-CE";
        write(
            &pull::gdfl_lineage::lineage_path(&root),
            &format!(
                "event NSE-FNO-ZZIDX ex=2020-06-16 kind=relisted shift=0 lot_old=75 lot_new=75 \
                 paired=0 stopped=1 v=1\nstopped {stopped} ex=2020-06-16 v=1\n"
            ),
        );
        let lineage_of = |key: &str| {
            answer(
                &root,
                &ask(&format!("feed=gdfl&instrument={key}&from=2020-06-15")),
            )["lineage"]
                .clone()
        };
        let on_key = lineage_of(stopped);
        assert_eq!(on_key["unreadable_lines"], 0, "{on_key:#}");
        assert_eq!(
            on_key["records"],
            json!([{ "kind": "stopped", "ex": "2020-06-16" }])
        );
        let event = json!([{
            "ex": "2020-06-16", "kind": "relisted", "shift_paisa": 0,
            "lot_old": 75, "lot_new": 75, "paired": 0, "stopped": 1,
        }]);
        assert_eq!(on_key["events"], event);
        let beside = lineage_of(other);
        assert_eq!(beside["records"], json!([]));
        assert_eq!(beside["events"], event, "the event is the underlying's");
        let elsewhere = lineage_of("NSE-FNO-ZZOTHER-2020-06-25-1000000-CE");
        assert_eq!(elsewhere["events"], json!([]));
    }

    /// **THE RE-KEY'S RECORDS ARE READ AND NAMED ON EACH KEY** (D-3331;
    /// WP-12c, DB-A17, DB-A18): the stated key is `superseded_by` the
    /// decoded one, the decoded key `supersedes` it, and a retired key says
    /// so with its refusal -- never counted as unreadable ledger lines.
    #[test]
    fn the_rekeys_records_are_read_and_named_on_each_key() {
        let root = root_of("lineage-rekey");
        let stated = "NSE-FNO-ZZIDX-2023-06-29-1000000-PE";
        let settled = "NSE-FNO-ZZIDX-2023-06-28-1000000-PE";
        let refused = "NSE-FNO-ZZIDX-2020-06-25-191250000-CE";
        let parse = |key: &str| pull::gdfl_lineage::parse_option_key(key).expect("an option key");
        let (under, old) = parse(stated);
        let moved = pull::gdfl_lineage::SupersededRecord {
            underlying: under,
            old,
            new: parse(settled).1,
            kind: pull::expiry_alias::AliasKind::VendorRedated,
            months: 2,
            bars: 3,
            digest: [7; 32],
        };
        let retired = pull::gdfl_lineage::RetiredRecord {
            underlying: under,
            key: parse(refused).1,
            why: pull::gdfl_lineage::RetiredWhy::StrikeImplausible,
            day: day("2019-05-30"),
            median: Some(1_170_000),
            days: 1,
            bars: 5,
        };
        write(
            &pull::gdfl_lineage::lineage_path(&root),
            &format!("{}\n{}\n", moved.line(), retired.line()),
        );
        let lineage_of = |key: &str, from: &str| {
            answer(
                &root,
                &ask(&format!("feed=gdfl&instrument={key}&from={from}")),
            )["lineage"]
                .clone()
        };
        let old_side = lineage_of(stated, "2020-06-15");
        assert_eq!(old_side["unreadable_lines"], 0, "{old_side:#}");
        let record = &old_side["records"][0];
        assert_eq!(record["kind"], "superseded_by");
        assert_eq!(record["key"], settled);
        assert_eq!(record["why"], "vendor_redated");
        assert_eq!(
            (record["months"].clone(), record["bars"].clone()),
            (json!(2), json!(3))
        );
        assert_eq!(record["blake3"], moved.digest_hex());
        let new_side = lineage_of(settled, "2020-06-15");
        assert_eq!(new_side["records"][0]["kind"], "supersedes");
        assert_eq!(new_side["records"][0]["key"], stated);
        let gone = lineage_of(refused, "2019-05-30");
        assert_eq!(gone["records"][0]["kind"], "retired");
        assert_eq!(gone["records"][0]["why"], "strike_implausible");
        assert_eq!(gone["records"][0]["refusal"], retired.refusal());
    }

    /// **A ZERODHA SHARE'S ZERO-VOLUME MINUTES ARE HELD, NOT TRADED** (DB-B16).
    /// A circuit-locked or illiquid share's day reads complete by count: 375
    /// minute bars, most with no trade in them. The record counts them apart;
    /// an index (no volume at all) and GDFL are not counted.
    #[test]
    fn a_shares_zero_volume_minutes_are_counted_apart() {
        let root = root_of("fills");
        let mut bars = minutes("2020-02-03", 375);
        for bar in bars.iter_mut().take(75) {
            bar.volume = 100;
        }
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-CASH-ZZFILL",
            Timeframe::MINUTE_1,
            bars,
        );
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-INDEX-ZZIDX",
            Timeframe::MINUTE_1,
            minutes("2020-02-03", 375),
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-CASH-ZZFILL&from=2020-02-03"),
        );
        let day = record(&got, "2020-02-03");
        assert_eq!(day["held"]["bars"], 375);
        assert_eq!(day["held"]["zero_volume"], 300);
        says_has(
            day,
            "300 of 375 Zerodha bar(s) carry zero volume: held, but no trade in that minute",
        );
        let index = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZIDX&from=2020-02-03"),
        );
        assert_eq!(
            record(&index, "2020-02-03")["held"]["zero_volume"],
            Value::Null,
            "an index has no volume to count"
        );
        let gdfl = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-CASH-ZZFILL&from=2020-02-03"),
        );
        assert_eq!(
            record(&gdfl, "2020-02-03")["held"]["zero_volume"],
            Value::Null
        );
    }

    /// **ZERODHA'S JUNK DAILY ROWS ARE NAMED, AND SERVED UNCHANGED.** A zero
    /// price (MAZDOCK 2018-01-01's shape), a bar on a Saturday (BHARATBOND's),
    /// an impossible jump down and back (NIFTY100EQLWGT 2018-03-27's, judged
    /// against the bar before the page, and against the month before it), and
    /// a share's zero volume on a day GDFL traded -- but not on a day it did
    /// not. DB-031, DB-053.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one fixture and every assertion about it, read top to bottom"
    )]
    fn zerodha_junk_rows_are_named_and_never_altered() {
        let root = root_of("junk");
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-CASH-ZZDOCK",
            Timeframe::DAY_1,
            vec![
                bar("2018-01-01", 0, [0; 4], 670),
                bar("2018-02-05", 0, [0; 4], 243),
            ],
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-CASH-ZZDOCK&from=2018-01"),
        );
        let zero = record(&got, "2018-01-01");
        assert_eq!(zero["junk"], json!(["zero_price"]));
        assert_eq!(zero["day_bar"]["c"], 0, "the bar is served as stored");
        assert_eq!(zero["day_bar"]["v"], 670);
        assert_eq!(
            zero["import"],
            Value::Null,
            "Zerodha is pulled, not imported"
        );
        says_has(zero, "zero price");
        let none = record(&got, "2018-01-02");
        assert_eq!(none["junk"], json!([]));
        assert_eq!(none["day_bar"]["why"], "no_bar");

        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-CASH-ZZBOND",
            Timeframe::DAY_1,
            vec![
                bar("2019-11-29", 0, [800, 810, 790, 805], 10),
                bar("2019-11-30", 0, [805, 805, 805, 805], 10),
            ],
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-CASH-ZZBOND&from=2019-11-29&to=2019-12-01"),
        );
        assert_eq!(record(&got, "2019-11-29")["junk"], json!([]));
        assert_eq!(record(&got, "2019-11-30")["junk"], json!(["off_calendar"]));
        assert_eq!(record(&got, "2019-12-01")["junk"], json!([]));

        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-INDEX-ZZEQLWGT",
            Timeframe::DAY_1,
            vec![
                bar("2018-02-28", 0, [1_000, 1_000, 1_000, 1_000], 0),
                bar("2018-03-26", 0, [1_000, 1_010, 990, 1_000], 0),
                bar("2018-03-27", 0, [1_000, 1_000, 70, 70], 0),
                bar("2018-03-28", 0, [1_000, 1_005, 995, 1_001], 0),
                bar("2018-04-02", 0, [1_001, 1_002, 999, 1_000], 0),
            ],
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZEQLWGT&from=2018-03-27&to=2018-03-28"),
        );
        assert_eq!(
            record(&got, "2018-03-27")["junk"],
            json!(["impossible_jump"]),
            "against the bar before the page"
        );
        assert_eq!(
            record(&got, "2018-03-28")["junk"],
            json!(["impossible_jump"]),
            "and back"
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZEQLWGT&from=2018-04-02"),
        );
        assert_eq!(
            record(&got, "2018-04-02")["junk"],
            json!([]),
            "against March's last bar, 1001"
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZEQLWGT&from=2018-03-26"),
        );
        assert_eq!(
            record(&got, "2018-03-26")["junk"],
            json!([]),
            "against February's last bar"
        );

        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-CASH-ZZSHARE",
            Timeframe::DAY_1,
            vec![
                bar("2018-09-05", 0, [600; 4], 0),
                bar("2018-09-06", 0, [600; 4], 0),
            ],
        );
        file_bars(
            &root,
            Vendor::Gdfl,
            "NSE-CASH-ZZSHARE",
            Timeframe::SECOND_1,
            vec![bar("2018-09-05", 34_000, [600; 4], 25)],
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-CASH-ZZSHARE&from=2018-09-05&to=2018-09-06"),
        );
        let traded = record(&got, "2018-09-05");
        assert_eq!(traded["junk"], json!(["zero_volume_peer_traded"]));
        assert_eq!(traded["peer"]["bars"], 1);
        says_has(traded, "zero volume while GDFL traded");
        assert_eq!(
            record(&got, "2018-09-06")["junk"],
            json!([]),
            "GDFL did not trade"
        );
    }

    /// **THE VENDOR'S DEGRADED DAYS, LATE ROWS, HOLES AND DUAL VALUES ARE
    /// NAMED FROM THE LEDGER AND THE JOURNAL**: 30 May 2019's minute-stamped
    /// index, lead LTQ=1 option files and late first share trade by D-3335's
    /// display rules; 28 Mar 2019's same-second dual values; 2022-03-07's
    /// shared share hole; 2021-05-21's late rows from the journal's own
    /// `late` and `max_back_s`. DB-026, DB-029, DB-030, DB-G96.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one fixture and every assertion about it, read top to bottom"
    )]
    fn degraded_days_late_rows_holes_and_dual_values_are_named() {
        let root = root_of("quality");
        write(
            &pull::gdfl_import::journal_path(&root),
            "begin indices 2021-05-21 *\n\
             done indices 2021-05-21 * files=2 seconds=3 failures=0 late=703 late_unresolved=0 max_back_s=126\n\
             begin indices 2021-05-24 *\n\
             done indices 2021-05-24 * files=2 seconds=3 failures=0 late=63 late_unresolved=0 max_back_s=19800\n",
        );
        let mut ledger = index_flags(
            "2019-05-30",
            "NIFTY",
            &[
                ("rows", "376"),
                ("seconds", "375"),
                ("session_s", "22500"),
                ("ss_distinct", "1"),
            ],
        );
        ledger.push_str(&flags_line(
            "options",
            "2019-05-30",
            "*",
            &OPTIONS_FIELDS,
            &[
                ("files", "12"),
                ("files_traded", "10"),
                ("zero_quote_files", "10"),
                ("ltq1_lead_files", "9"),
                ("first_trade_p50", "09:15:11"),
            ],
        ));
        ledger.push_str(&flags_line(
            "stocks",
            "2019-05-30",
            "*",
            &STOCKS_FIELDS,
            &[
                ("files", "5"),
                ("files_traded", "5"),
                ("first_trade_p50", "11:57:48"),
            ],
        ));
        ledger.push_str(&index_flags(
            "2019-03-28",
            "NIFTY",
            &[
                ("rows", "900"),
                ("ss_distinct", "60"),
                ("dual_spread_s", "3"),
                ("dual_spread_max_bps", "20"),
            ],
        ));
        ledger.push_str(&flags_line(
            "stocks",
            "2022-03-07",
            "*",
            &STOCKS_FIELDS,
            &[
                ("files", "5"),
                ("files_traded", "5"),
                ("first_trade_p50", "09:15:02"),
                ("shared_holes", "1"),
                ("shared_hole_max_s", "1359"),
                ("shared_hole_max_at", "09:16:44"),
            ],
        ));
        write(&quality_path(&root), &ledger);

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-05-30"),
        );
        let degraded = record(&got, "2019-05-30");
        let kinds: Vec<&str> = degraded["degraded"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["indices", "stocks", "options"], "{degraded:#}");
        says_has(
            degraded,
            "degraded vendor day -- index: minute-sampled, 375 of 22500 session second(s) held \
             (1.7%), its rows on 1 distinct second(s) of the minute",
        );
        assert_eq!(degraded["degraded"][0]["flag"], "sampled");
        assert_eq!(
            degraded["sampled"],
            json!({
                "is": true, "of": "own", "series": "NSE-INDEX-NIFTY",
                "basis": "day_ledger", "standing": "measured",
                "held_s": 375, "session_s": 22500, "ss_distinct": 1, "line": 1,
            })
        );
        says_has(
            degraded,
            "degraded vendor day -- options: 10 of 10 traded files carry zero bid and ask, 9 open with an LTQ=1 row",
        );
        says_has(
            degraded,
            "degraded vendor day -- stocks: the median first trade is at 11:57:48",
        );
        assert_eq!(
            degraded["quality"]["options"]["first_trade_p50"],
            "09:15:11"
        );
        assert_eq!(
            degraded["quality"]["indices"]["hole_max_at"],
            Value::Null,
            "`-` is null"
        );

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2019-03-28"),
        );
        let dual = record(&got, "2019-03-28");
        assert_eq!(dual["degraded"], json!([]));
        assert_eq!(dual["sampled"]["is"], false, "sixty seconds of the minute");
        assert_eq!(dual["sampled"]["basis"], "day_ledger");
        says_has(
            dual,
            "3 second(s) holding two values at least 10 bps apart, up to 20 bps",
        );

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2022-03-07"),
        );
        let hole = record(&got, "2022-03-07");
        assert_eq!(
            hole["quality"]["indices"],
            Value::Null,
            "a share reads no index line"
        );
        says_has(
            hole,
            "1 hole(s) of 300 s or more shared by 90% of traded shares, the longest 1359 s at 09:16:44",
        );

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2021-05-21"),
        );
        let late = record(&got, "2021-05-21");
        assert_eq!(late["import"]["late"], 703);
        says_has(
            late,
            "703 late row(s) deferred to a later second, at most 126 s back",
        );
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2021-05-24"),
        );
        says_has(
            record(&got, "2021-05-24"),
            "63 late row(s) deferred to a later second, at most 19800 s back; 19800 s is exactly \
             IST's offset from UTC",
        );
    }

    /// **A STORED-MONTH GAP LINE IS A REASON, NOT "ZERODHA LEDGER: <CODE>"**
    /// (R3-18): `cli zerodha-gaps` writes `why=stored_short` with `fill=` and
    /// `source=` after D-3335's fields; the day band parses it, ignores the
    /// two trailing fields by name, and says the store was read, not the
    /// vendor.
    #[test]
    fn a_stored_month_gap_line_is_said_as_a_reason() {
        let root = root_of("stored-short");
        write(
            &gaps_path(&root),
            "gap NSE-INDEX-NIFTY 2019-09-23 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=stored_short run=0 fill=interior source=store\n",
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2019-09-23"),
        );
        let gap = record(&got, "2019-09-23");
        assert_eq!(gap["gaps"]["why"], "stored_short", "{got:#}");
        assert_eq!(gap["gaps"]["missing"], 14);
        says_has(
            gap,
            "Zerodha minute gap found in the stored month (no vendor call, R3-18): 14 of 375 owed minute(s) missing, the longest run 14 from 15:15; a versioned repair or a pull fills it",
        );
    }

    /// **A SHARE'S STORED-MONTH GAP CARRIES THE NO-TRADE CAVEAT** (D-3321,
    /// verify-R3-18-r1): an `NSE-CASH` line, labelled or not, never says a
    /// repair "fills it" — the store cannot tell a minute nobody traded from a
    /// vendor hole, and a repair lands only what the feed serves. An index's
    /// line keeps its plain sentence and carries no label.
    #[test]
    fn a_shares_stored_month_gap_says_the_minutes_may_be_untraded() {
        let root = root_of("stored-short-cash");
        write(
            &gaps_path(&root),
            "gap NSE-CASH-ZZSHARE 2019-05-02 v=1 rung=1min owed=375 held=0 missing=375 first_missing=09:15 runs=1 longest=375 longest_at=09:15 why=stored_short run=0 fill=interior source=store absence=may_be_no_trade\n\
             gap NSE-CASH-ZZOTHER 2019-05-02 v=1 rung=1min owed=375 held=370 missing=5 first_missing=10:00 runs=1 longest=5 longest_at=10:00 why=vendor_empty run=4\n\
             gap NSE-INDEX-NIFTY 2019-05-02 v=1 rung=1min owed=375 held=361 missing=14 first_missing=15:15 runs=1 longest=14 longest_at=15:15 why=stored_short run=0 fill=interior source=store\n",
        );
        let caveat = "these may be minutes nobody traded: the store cannot tell a no-trade minute from a vendor hole (D-3321), and a repair or a pull lands only the minutes the feed serves";
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-CASH-ZZSHARE&from=2019-05-02"),
        );
        let gap = record(&got, "2019-05-02");
        assert_eq!(gap["gaps"]["absence"], "may_be_no_trade", "{got:#}");
        says_has(
            gap,
            &format!(
                "Zerodha minute gap found in the stored month (no vendor call, R3-18): 375 of 375 owed minute(s) missing, the longest run 375 from 09:15; {caveat}"
            ),
        );
        assert!(
            !gap["says"].to_string().contains("fills it"),
            "a share's line never promises a fill: {got:#}"
        );
        // A vendor line for a share, unlabelled: the key's segment says it.
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-CASH-ZZOTHER&from=2019-05-02"),
        );
        let gap = record(&got, "2019-05-02");
        assert_eq!(gap["gaps"]["absence"], "may_be_no_trade", "{got:#}");
        says_has(
            gap,
            &format!(
                "Zerodha minute gap: 5 of 375 owed minute(s) missing, the longest run 5 from 10:00; {caveat}"
            ),
        );
        // An index: computed every minute, no caveat, no label.
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2019-05-02"),
        );
        let gap = record(&got, "2019-05-02");
        assert!(gap["gaps"]["absence"].is_null(), "{got:#}");
        says_has(
            gap,
            "Zerodha minute gap found in the stored month (no vendor call, R3-18): 14 of 375 owed minute(s) missing, the longest run 14 from 15:15; a versioned repair or a pull fills it",
        );
    }

    /// **ZERODHA'S GAP LEDGER NAMES ITS MINUTE GAPS AND REFUSED FRONTS**, and
    /// a GDFL record carries Zerodha's line for the same day as its peer's;
    /// the derive lines of the page's months are in the envelope. DB-G37,
    /// DB-053.
    #[test]
    fn the_zerodha_gap_ledger_names_gaps_and_refused_fronts() {
        let root = root_of("gaps");
        write(
            &gaps_path(&root),
            "gap NSE-INDEX-NIFTY 2020-11-14 v=1 rung=1min owed=60 held=0 missing=60 first_missing=18:15 runs=1 longest=60 longest_at=18:15 why=vendor_empty run=7\n\
             gap NSE-INDEX-ZZOIL 2020-01-13 v=1 rung=1min owed=375 held=0 missing=375 first_missing=09:15 runs=1 longest=375 longest_at=09:15 why=refused_front run=8\n\
             derive NSE-INDEX-NIFTY 5min 2020-11 v=1 days=18 buckets=1350 withheld=12 partial=0 run=9\n\
             derive NSE-INDEX-NIFTY 5min 2020-12 v=1 days=22 buckets=1650 withheld=0 partial=0 run=9\n",
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2020-11-14"),
        );
        let gap = record(&got, "2020-11-14");
        assert_eq!(gap["gaps"]["missing"], 60);
        assert_eq!(gap["gaps"]["first_missing"], "18:15");
        assert_eq!(gap["gaps"]["why"], "vendor_empty");
        assert_eq!(gap["refused_front"], false);
        says_has(
            gap,
            "Zerodha minute gap: 60 of 60 owed minute(s) missing, the longest run 60 from 18:15",
        );
        assert_eq!(
            got["derive"].as_array().unwrap().len(),
            1,
            "November's only: {got:#}"
        );
        assert_eq!(got["derive"][0]["withheld"], 12);

        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-11-14"),
        );
        let peer = record(&got, "2020-11-14");
        assert_eq!(peer["gaps"]["feed"], "zerodha");
        assert_eq!(
            peer["refused_front"],
            Value::Null,
            "a Zerodha span, not GDFL's"
        );

        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2020-01-13"),
        );
        let front = record(&got, "2020-01-13");
        assert_eq!(front["refused_front"], true);
        says_has(
            front,
            "refused front: the vendor served this span and the pull refused it (D-3315)",
        );
    }

    /// **AN ABSENT LEDGER IS AN ANSWER, NOT AN ERROR**: the field is null on
    /// every record, `sources` says the file is not there, and a note says the
    /// day is not measured rather than clean.
    #[test]
    fn an_absent_ledger_is_named_and_its_fields_are_null() {
        let root = root_of("absent");
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03-07"),
        );
        assert_eq!(got["sources"]["quality"]["present"], false);
        assert_eq!(got["sources"]["gaps"]["present"], false);
        let one = record(&got, "2022-03-07");
        assert_eq!(one["quality"]["indices"], Value::Null);
        assert_eq!(one["gaps"], Value::Null);
        assert_eq!(one["import"]["outcome"], "none");
        assert_eq!(one["held"]["why"], "not_held");
        let notes = got["notes"].as_array().unwrap();
        for wanted in ["No GDFL day ledger", "No Zerodha gap ledger"] {
            assert!(
                notes.iter().any(|n| n.as_str().unwrap().contains(wanted)),
                "{wanted}: {notes:#?}"
            );
        }
        let zerodha = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2022-03-07"),
        );
        assert_eq!(
            zerodha["sources"]["quality"],
            Value::Null,
            "quality is GDFL's"
        );
        assert_eq!(
            record(&zerodha, "2022-03-07")["refused_front"],
            Value::Null,
            "unknown, not false"
        );
    }

    // ── The ledgers' reader ────────────────────────────────────────────────

    /// **A LEDGER IS READ BY ITS TAIL AND THE LAST LINE OF A KEY WINS**: a
    /// second read with nothing appended reads nothing, an append reads only
    /// its bytes, a line still being written is not parsed, and a replaced
    /// file is read again from its start.
    #[test]
    fn a_ledger_is_read_by_its_tail_and_the_last_line_of_a_key_wins() {
        let root = root_of("tail");
        let path = quality_path(&root);
        let first = index_flags("2022-03-07", "NIFTY", &[("zero_ltp_rows", "3")]);
        let second = index_flags("2022-03-07", "NIFTY", &[("zero_ltp_rows", "4")]);
        write(&path, &format!("{first}{second}"));
        let read = read_ledger(quality_tails(), &path).unwrap();
        assert!(read.present);
        assert_eq!(read.lines, 2);
        assert_eq!(read.read_now, read.bytes);
        let line = read
            .parsed
            .get(day("2022-03-07"), ImportKind::Indices, "NIFTY")
            .unwrap();
        assert_eq!(line.count("zero_ltp_rows"), Some(4), "the last line wins");
        assert_eq!(line.line, 2);

        let again = read_ledger(quality_tails(), &path).unwrap();
        assert_eq!(again.read_now, 0, "nothing new, nothing read");

        let third = index_flags("2022-03-08", "NIFTY", &[]);
        let torn = "flags indices 2022-03-09 NIFTY v=1";
        append(&path, &format!("{third}{torn}"));
        let tail = read_ledger(quality_tails(), &path).unwrap();
        assert_eq!(
            tail.read_now,
            u64::try_from(third.len() + torn.len()).unwrap()
        );
        assert_eq!(tail.unterminated, u64::try_from(torn.len()).unwrap());
        assert_eq!(tail.lines, 3);
        assert!(
            tail.parsed
                .get(day("2022-03-08"), ImportKind::Indices, "NIFTY")
                .is_some()
        );
        assert!(
            tail.parsed
                .get(day("2022-03-09"), ImportKind::Indices, "NIFTY")
                .is_none()
        );

        // REPLACED: another inode is another file.
        let replaced = path.with_file_name("gdfl.dayflags.new");
        write(&replaced, &first);
        std::fs::rename(&replaced, &path).unwrap();
        let fresh = read_ledger(quality_tails(), &path).unwrap();
        assert_eq!(fresh.lines, 1);
        let line = fresh
            .parsed
            .get(day("2022-03-07"), ImportKind::Indices, "NIFTY")
            .unwrap();
        assert_eq!(line.count("zero_ltp_rows"), Some(3));
    }

    /// **A LINE THIS READER CANNOT NAME IS COUNTED AND QUOTED WITH ITS
    /// REASON**, never dropped in silence and never guessed; the good lines
    /// around it still read.
    #[test]
    fn a_line_this_reader_cannot_name_is_counted_with_its_reason() {
        let root = root_of("faults");
        let good = index_flags("2022-03-07", "NIFTY", &[]);
        let bad = [
            good.replace("v=1", "v=2"),
            good.replace(" zero_ltp_rows=0", ""),
            good.replace("rows=0", "rows=07"),
            good.replace(" NIFTY ", " * "),
            good.replace("flags indices", "flags futures"),
            "begin indices 2022-03-07 *\n".to_owned(),
            good.replace("hole_max_at=-", "hole_max_at=9:15"),
            good.replace("src=rescan", "src=guess"),
        ];
        let text: String = bad.concat() + &good;
        write(&quality_path(&root), &text);
        let read = read_ledger(quality_tails(), &quality_path(&root)).unwrap();
        let faults = &read.parsed.faults;
        assert_eq!(faults.count, 8, "{faults:#?}");
        let reasons: Vec<&str> = faults
            .named
            .iter()
            .map(|(_, _, why)| why.as_str())
            .collect();
        for wanted in [
            "v=2 is not 1",
            "zero_ltp_rows is missing",
            "rows=\"07\" is not a Count",
            "scope \"*\"",
            "\"futures\" is not a kind",
            "not a flags line",
            "hole_max_at=\"9:15\" is not a Clock",
            "src is not import or rescan",
        ] {
            assert!(
                reasons.iter().any(|why| why.contains(wanted)),
                "{wanted}: {reasons:#?}"
            );
        }
        assert_eq!(read.parsed.rows.len(), 1, "the good line reads");

        let gaps = root.join("audit").join("zerodha.gaps");
        write(
            &gaps,
            "gap NSE-INDEX-NIFTY 2020-11-14 v=1 rung=1min owed=60 held=0 missing=60 first_missing=18:15:00 runs=1 longest=60 longest_at=18:15 why=vendor_empty run=7\n\
             gap NSE-INDEX-NIFTY 2020-11-14 v=1 rung=1min owed=60 held=0 missing=60 first_missing=18:15 runs=1 longest=60 longest_at=18:15 why=Vendor run=7\n\
             derive NSE-INDEX-NIFTY 7min 2020-11 v=1 days=1 buckets=1 withheld=0 partial=0 run=1\n\
             note something\n",
        );
        let read = read_ledger(gap_tails(), &gaps).unwrap();
        assert_eq!(read.parsed.faults.count, 4);
        assert!(read.parsed.gaps.is_empty());
    }

    // ── The calendar code, the parameters, the page ────────────────────────

    /// **D-3336'S DAY CODE**, one day of each code this calendar can reach.
    #[test]
    fn the_calendar_code_follows_d3336() {
        for (on, code) in [
            ("2018-09-08", "."),
            ("2022-03-07", "R"),
            ("2026-08-03", "X"),
            ("2024-03-02", "P"),
            ("2018-11-07", "P"),
            ("2021-02-24", "P"),
            ("2020-03-13", "H"),
            ("2020-03-23", "H"),
            ("2017-12-29", "-"),
            // The record runs to 2026-12-31 (DB-B41): the first weekday past
            // its old end is a session, and the first past its new end is not
            // answered.
            ("2026-10-05", "X"),
            ("2027-01-04", "-"),
        ] {
            let epoch_day = i64::from(day(on).days_from_epoch());
            assert_eq!(day_code(epoch_day).0.to_string(), code, "{on}");
        }
    }

    /// **EVERY PARAMETER IS READ OR REFUSED BY NAME.**
    #[test]
    fn every_parameter_is_read_or_refused_by_name() {
        for (query, said) in [
            (
                "instrument=NSE-INDEX-NIFTY&from=2022-03",
                "needs ?feed=gdfl",
            ),
            (
                "feed=dhan&instrument=NSE-INDEX-NIFTY&from=2022-03",
                "gdfl and zerodha only",
            ),
            ("feed=gdfl&from=2022-03", "needs instrument="),
            (
                "feed=gdfl&instrument=NIFTY&from=2022-03",
                "is not <EXCHANGE>",
            ),
            (
                "feed=gdfl&instrument=NSE-SPOT-NIFTY&from=2022-03",
                "not one of INDEX",
            ),
            (
                "feed=gdfl&instrument=NSE-INDEX-nifty&from=2022-03",
                "not a store symbol",
            ),
            ("feed=gdfl&instrument=NSE-INDEX-NIFTY", "needs from="),
            (
                "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-3",
                "neither a day",
            ),
            (
                "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03-02&to=2022-03-01",
                "is after",
            ),
            (
                "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03&order=up",
                "order=",
            ),
            (
                "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03&page=-1",
                "page=",
            ),
            (
                "feed=gdfl&instrument=NSE-INDEX-NIFTY&contract=2019-03-28-1150000-CE&from=2022-03",
                "FNO only",
            ),
            (
                "feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&contract=2019-03-28-1150000-PE&from=2019-03",
                "another",
            ),
        ] {
            let why = asked(query).expect_err(query);
            assert!(why.contains(said), "{query}: {why}");
        }
        let month = ask("feed=ZERODHA&instrument=NSE-INDEX-NIFTY&from=2024-02");
        assert_eq!(month.feed, Vendor::Zerodha);
        assert_eq!(
            (month.from, month.to),
            (day("2024-02-01"), day("2024-02-29"))
        );
        let one = ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-02-10");
        assert_eq!((one.from, one.to), (day("2024-02-10"), day("2024-02-10")));
        let span =
            ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-01&to=2024-03&order=desc&page=1");
        assert_eq!((span.from, span.to), (day("2024-01-01"), day("2024-03-31")));
        assert!(span.newest_first);
        assert_eq!(span.page, 1);
    }

    /// **A CENSUS KEY NAMES ITS CONTRACT INSIDE IT**, a symbol may hold a
    /// dash, and `contract=` agreeing with the key is the same contract.
    #[test]
    fn a_census_key_names_its_contract_and_a_symbol_may_hold_a_dash() {
        let key = "NSE-FNO-BAJAJ-AUTO-2019-03-28-300000-CE";
        let inst = instrument_of(key, "").unwrap();
        assert_eq!(inst.symbol, "BAJAJ-AUTO");
        assert_eq!(inst.contract.unwrap().as_str(), "2019-03-28-300000-CE");
        assert_eq!(inst.key(), key);
        assert_eq!(instrument_of(key, "2019-03-28-300000-CE").unwrap(), inst);
        let split = instrument_of("NSE-FNO-NIFTY", "2019-03-28-1150000-PE").unwrap();
        assert_eq!(split.key(), "NSE-FNO-NIFTY-2019-03-28-1150000-PE");
        let fut = instrument_of("NSE-FNO-NIFTY-2019-03-28-FUT", "").unwrap();
        assert!(fut.contract.unwrap().is_future());
        let kind = instrument_of("NSE-FNO-NIFTY", "").unwrap();
        assert_eq!(kind.contract, None);
        assert_eq!(kind.kind(), ImportKind::Options);
        assert_eq!(
            instrument_of("NSE-CASH-BAJAJ-AUTO", "").unwrap().symbol,
            "BAJAJ-AUTO"
        );
    }

    /// **A RANGE LONGER THAN A PAGE IS PAGED BY DAYS**, oldest first or
    /// newest first, and a page past the end is the last one.
    #[test]
    fn a_long_range_is_paged_by_days_either_way() {
        let root = root_of("pages");
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-01-01&to=2024-12-31"),
        );
        assert_eq!(got["pages"], 4);
        assert_eq!(got["days_in_range"], 366);
        assert_eq!(got["days"].as_array().unwrap().len(), 92);
        assert_eq!(got["days"][0]["day"], "2024-01-01");
        assert_eq!(got["page_to"], "2024-04-01");
        let last = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-01-01&to=2024-12-31&page=9"),
        );
        assert_eq!(last["page"], 3);
        assert_eq!(last["page_from"], "2024-10-03");
        assert_eq!(last["days"].as_array().unwrap().len(), 90);
        let desc = answer(
            &root,
            &ask(
                "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-01-01&to=2024-12-31&order=desc&page=3",
            ),
        );
        assert_eq!(desc["days"][0]["day"], "2024-03-30");
        assert_eq!(desc["page_from"], "2024-01-01");
        assert_eq!(desc["days"].as_array().unwrap().len(), 90);
        let newest = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-12&order=desc"),
        );
        assert_eq!(newest["days"][0]["day"], "2024-12-31");
        assert_eq!(newest["days"][30]["day"], "2024-12-01");
    }

    /// **AN OPTIONS REFUSAL IS ABOUT THIS CONTRACT ONLY WHEN ITS TICKER
    /// DECODES TO IT**; a kind-level options instrument owns every refusal of
    /// its underlying and counts no bars. DB-G69.
    #[test]
    fn an_options_refusal_is_about_the_contract_its_ticker_names() {
        let root = root_of("options");
        write(
            &pull::gdfl_import::journal_path(&root),
            "begin options 2019-03-20 *\n\
             done options 2019-03-20 * definition=2 files=9 seconds=9 failures=2 late=0 late_unresolved=0 max_back_s=0 vendor_refused=2 reason=malformed_row\n\
             begin options 2019-03-22 *\n\
             incomplete options 2019-03-22 * definition=2 files=9 seconds=9 failures=1 late=0 late_unresolved=0 max_back_s=0\n",
        );
        cli_log(
            &root,
            &[
                (
                    "options",
                    "2019-03-20",
                    "NIFTY28MAR1911500CE",
                    "line 4: not ten well-formed fields",
                ),
                (
                    "options",
                    "2019-03-20",
                    "NIFTY28MAR1911600CE",
                    "line 5: not ten well-formed fields",
                ),
                (
                    "options",
                    "2019-03-22",
                    "ZZBANK28MAR1912000PE",
                    "line 6: not ten well-formed fields",
                ),
            ],
        );
        // THE CONTRACT TRADED ON THE 19TH, so its month is held and the 20th
        // holds no bar.
        file_bars(
            &root,
            Vendor::Gdfl,
            "NSE-FNO-NIFTY-2019-03-28-1150000-CE",
            Timeframe::SECOND_1,
            vec![bar("2019-03-19", 34_000, [500; 4], 75)],
        );
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&from=2019-03-20"),
        );
        let one = record(&got, "2019-03-20");
        assert_eq!(one["held"]["bars"], 0);
        assert_eq!(one["import"]["outcome"], "done_with_refusals");
        says_has(
            one,
            "no GDFL bar on this session: a one-second option or share bar exists only for a \
             traded second (D-3336 §2)",
        );
        assert_eq!(one["import"]["refusals"]["this_count"], 1, "{one:#}");
        assert_eq!(one["import"]["refusals"]["other_count"], 1);
        assert_eq!(
            one["import"]["refusals"]["this"][0]["about"],
            "NIFTY28MAR1911500CE"
        );
        // The close names how many of its refusals are this contract's and
        // the first (AF-02, D-4970).
        says_has(
            one,
            "imported with 2 vendor refusal(s), 1 about this instrument; the first is \
             NIFTY28MAR1911500CE: line 4: not ten well-formed fields",
        );
        assert_eq!(
            one["import"]["refusals"]["other_first"]["about"],
            "NIFTY28MAR1911600CE"
        );
        // A DAY THE IMPORT DID NOT TAKE is not explained as a day without a
        // trade: the journal has no line for 2019-03-18, a regular Monday
        // before its last day.
        let untaken = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&from=2019-03-18"),
        );
        let untaken = record(&untaken, "2019-03-18");
        assert_eq!(untaken["held"]["bars"], 0);
        says_has(untaken, "no GDFL bar on this session");
        says_has(untaken, "no import journal line for this day");
        // INCOMPLETE FOR ANOTHER CONTRACT'S FILE, said as such (DB-G69).
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&from=2019-03-22"),
        );
        let other = record(&got, "2019-03-22");
        assert_eq!(other["import"]["outcome"], "incomplete");
        assert_eq!(other["import"]["refusals"]["this_count"], 0);
        says_has(
            other,
            "incomplete at import: 1 failure(s), none about this instrument; the first is \
             ZZBANK28MAR1912000PE: line 6: not ten well-formed fields",
        );
        let kind = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-FNO-NIFTY&from=2019-03-20"),
        );
        let kind = record(&kind, "2019-03-20");
        assert_eq!(kind["import"]["refusals"]["this_count"], 2);
        assert_eq!(kind["held"]["why"], "no_contract");
        assert_eq!(kind["peer"]["why"], "no_contract");
    }

    #[test]
    fn a_key_the_rekey_retired_is_refused_by_the_ledgers_sentence() {
        // D-3331; WP-12c, DB-A18. An INVENTED strike far past any ladder,
        // one bar, and a neighbour the ledger never names.
        let root = root_of("rekey-retired");
        let retired_key = "NSE-FNO-NIFTY-2019-03-28-90000000-CE";
        let kept_key = "NSE-FNO-NIFTY-2019-03-28-1150000-CE";
        for key in [retired_key, kept_key] {
            file_bars(
                &root,
                Vendor::Gdfl,
                key,
                Timeframe::SECOND_1,
                vec![bar("2019-03-19", 34_000, [500; 4], 75)],
            );
        }
        let asked = |key: &str| ask(&format!("feed=gdfl&instrument={key}&from=2019-03-19"));
        // BEFORE THE RE-KEY RECORDS IT, the bar is counted like any other.
        let before = answer(&root, &asked(retired_key));
        assert_eq!(record(&before, "2019-03-19")["held"]["bars"], 1);
        let record_line = pull::gdfl_lineage::RetiredRecord {
            underlying: brutex_core::symbol::Symbol::new("NIFTY").expect("a symbol"),
            key: Contract::parse("2019-03-28-90000000-CE").expect("a contract"),
            why: pull::gdfl_lineage::RetiredWhy::StrikeImplausible,
            day: day("2019-03-19"),
            median: Some(1_150_000),
            days: 1,
            bars: 1,
        };
        write(
            &pull::gdfl_lineage::lineage_path(&root),
            &format!("{}\n", record_line.line()),
        );
        // AFTER: not counted, and the ledger's own sentence says why.
        let after = answer(&root, &asked(retired_key));
        let one = record(&after, "2019-03-19");
        assert_eq!(one["held"]["bars"], Value::Null, "{one:#}");
        assert_eq!(one["held"]["why"], "unreadable");
        let notes: Vec<&str> = after["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let refusal = record_line.refusal();
        assert!(
            notes.iter().any(|note| note.ends_with(&refusal)),
            "{refusal:?} not in {notes:#?}"
        );
        assert!(pull::gdfl_lineage::is_contract_refused(&refusal));
        // A KEY THE LEDGER DOES NOT NAME is counted as before.
        let kept = answer(&root, &asked(kept_key));
        assert_eq!(record(&kept, "2019-03-19")["held"]["bars"], 1);
    }

    // ── The Zerodha pull journal ──────────────────────────────────────────

    /// A pull run whose source names Zerodha by its policy, as the operator's
    /// runs do, and the member failures appended after it.
    fn pull_run(root: &Path, source: &str, members: &[(&str, &str, &str, &str)]) {
        let journal = crate::audit::Journal::at(root);
        let at = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_266_622);
        journal
            .append(&crate::audit::Record::refused(
                crate::audit::Scope::Spot,
                crate::audit::Outcome::Failed,
                at,
                source,
                "fixture run",
            ))
            .expect("a run record");
        for (symbol, from, to, note) in members {
            journal
                .append(&crate::audit::Record::member_failure(
                    crate::audit::Scope::Spot,
                    at,
                    symbol,
                    pull::session::Window::new(day(from), day(to)).expect("a window"),
                    note,
                ))
                .expect("a member failure");
        }
    }

    /// **THE TWO OFFSETS THIS READER SKIPS BY ARE THE RECORD'S OWN**: the kind
    /// byte and the note bytes, pinned to `audit::Record::image` for a run and
    /// a member failure, so a skipped record is one the decoder would skip too.
    #[test]
    fn the_pull_journal_offsets_are_the_records_own() {
        let at = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_266_622);
        let window = pull::session::Window::new(day("2018-01-01"), day("2026-09-30")).unwrap();
        let note = "conflicting vendor candles at 1578853800; refused before append";
        let member = crate::audit::Record::member_failure(
            crate::audit::Scope::Spot,
            at,
            "ZZOIL",
            window,
            note,
        );
        let image = member.image();
        assert_eq!(
            image[PULL_KIND_AT],
            crate::audit::Kind::MemberFailure.code()
        );
        assert!(may_refuse(&image), "the note bytes hold the note");
        assert!(holds(
            &image[PULL_NOTE_BYTES],
            b"conflicting vendor candles at"
        ));
        let run = crate::audit::Record::refused(
            crate::audit::Scope::Spot,
            crate::audit::Outcome::Failed,
            at,
            "zerodha_nse_checked fixture",
            "fixture",
        );
        assert_eq!(run.image()[PULL_KIND_AT], crate::audit::Kind::Run.code());
        let quiet = crate::audit::Record::member_failure(
            crate::audit::Scope::Spot,
            at,
            "ZZOIL",
            window,
            "request minute coverage UNVERIFIED on 2018-11-07",
        );
        assert!(!may_refuse(&quiet.image()), "no refusal, not decoded");
    }

    /// **A REFUSED FRONT IS NAMED APART FROM A SPAN BEFORE THE FIRST BAR**
    /// (DB-053, D-3315): a series whose `1day` bars start 2023-06-26, a
    /// Zerodha run that refused its whole 2018-2026 window before append, a
    /// dated Muhurat refusal, and another vendor's member for the same symbol
    /// that is counted, never kept. Before the first bar the day is a refused
    /// front; from it, no day is, and the whole-window refusal is not named on
    /// a held day. Without the journal the same day is only "before the first
    /// bar", and unknown.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one fixture and every assertion about it, read top to bottom"
    )]
    fn a_refused_front_is_named_apart_from_a_span_before_the_first_bar() {
        let root = root_of("refused-front");
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-INDEX-ZZOIL",
            Timeframe::DAY_1,
            vec![
                bar("2023-06-26", 0, [900, 910, 890, 905], 0),
                bar("2023-07-03", 0, [905, 915, 900, 910], 0),
            ],
        );
        // BEFORE ANY JOURNAL: before the first bar, and unknown.
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2020-01-13"),
        );
        assert_eq!(got["zerodha_1day_start"]["state"], "first_bar");
        assert_eq!(got["zerodha_1day_start"]["first_bar"], "2023-06-26");
        let unknown = record(&got, "2020-01-13");
        assert_eq!(unknown["refused_front"], Value::Null, "{unknown:#}");
        assert_eq!(unknown["pull"]["journal_present"], false);
        assert_eq!(unknown["pull"]["before_first_bar"], true);
        says_has(
            unknown,
            "before Zerodha's first held 1day bar, 2023-06-26; no pull journal could be read, so \
             whether the vendor served this span is unknown",
        );

        let whole = "conflicting vendor candles at 1578853800; refused before append";
        let dated = "2020-11-14: 60 minute row(s) refused: the exchange calendar records a session";
        pull_run(
            &root,
            "Reference indices",
            &[("ZZOIL", "2018-01-01", "2026-09-30", whole)],
        );
        pull_run(
            &root,
            "zerodha_nse_checked fixture",
            &[
                ("ZZOIL", "2018-01-01", "2026-09-30", whole),
                ("ZZOIL", "2018-01-01", "2026-09-30", dated),
                ("ZZOTHER", "2018-01-01", "2026-09-30", whole),
            ],
        );
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2020-01-13"),
        );
        // THROUGH THE INDEX (WP-33c): the asked instrument's refusals only.
        assert_eq!(got["sources"]["pull_journal"]["zerodha_refusals"], 2);
        assert_eq!(got["sources"]["pull_journal"]["other_vendor_members"], 1);
        assert_eq!(got["sources"]["pull_journal"]["via"]["kind"], "index");
        let front = record(&got, "2020-01-13");
        assert_eq!(front["refused_front"], true, "{front:#}");
        assert_eq!(front["pull"]["count"], 1);
        assert_eq!(front["pull"]["refusals"][0]["whole_window"], true);
        assert_eq!(front["pull"]["refusals"][0]["cut"], true);
        assert_eq!(front["pull"]["refusals"][0]["names_day"], "2020-01-13");
        says_has(
            front,
            "refused front of Zerodha's 1day series: a Zerodha pull refused its whole window \
             before append (conflicting vendor candles at 1578853800; refused before app...), and \
             the first held 1day bar is 2023-06-26 (D-3315)",
        );

        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2020-11-14"),
        );
        let muhurat = record(&got, "2020-11-14");
        assert_eq!(
            muhurat["pull"]["count"], 2,
            "the window's, and the day's own"
        );
        assert!(
            says(muhurat)
                .iter()
                .any(|s| s.starts_with("a Zerodha pull refused rows of this day, run of 2026-10-06: 2020-11-14: 60 minute row(s) refused")),
            "{:#?}",
            says(muhurat)
        );

        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2023-06-25&to=2023-06-26"),
        );
        assert_eq!(
            record(&got, "2023-06-25")["refused_front"],
            true,
            "the span's last day"
        );
        let held = record(&got, "2023-06-26");
        assert_eq!(held["refused_front"], false, "the first bar is no front");
        assert_eq!(
            held["pull"]["count"], 0,
            "a held day names no window refusal"
        );
        assert!(says(held).iter().all(|s| !s.contains("refused front")));

        // A PAGE WHOSE SERIES IS HELD BEFORE IT is settled by the bar before
        // the page: no front, and no month probe.
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2023-07-03"),
        );
        assert_eq!(got["zerodha_1day_start"]["state"], "held_before");
        assert_eq!(record(&got, "2023-07-03")["refused_front"], false);

        // A SERIES HOLDING NOTHING is refused, not a front of anything.
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOTHER&from=2020-01-13"),
        );
        assert_eq!(got["zerodha_1day_start"]["state"], "none_held");
        let nothing = record(&got, "2020-01-13");
        assert_eq!(nothing["refused_front"], false);
        assert!(
            says(nothing)
                .iter()
                .any(|s| s.contains("nothing of the series is held near it")),
            "{:#?}",
            says(nothing)
        );

        // A GDFL PAGE does not read Zerodha's pull journal.
        let gdfl = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-ZZOIL&from=2020-01-13"),
        );
        assert_eq!(gdfl["sources"]["pull_journal"], Value::Null);
        assert_eq!(record(&gdfl, "2020-01-13")["pull"], Value::Null);
        assert_eq!(record(&gdfl, "2020-01-13")["refused_front"], Value::Null);
    }

    /// **A JOURNAL WITH NO INDEX IS WALKED, BY ITS TAIL, AND SAYS SO**: the
    /// shape of the operator's journal before WP-33c's first append. A second
    /// read reads nothing new, an append reads only its records, a torn record
    /// is not read, and the answer names the walk and why.
    #[test]
    fn a_journal_with_no_index_is_walked_by_its_tail_and_says_so() {
        let root = root_of("pull-tail");
        pull_run(
            &root,
            "zerodha_nse_checked fixture",
            &[(
                "ZZOIL",
                "2018-01-01",
                "2026-09-30",
                "conflicting vendor candles at 1578853800; refused before append",
            )],
        );
        let journal = crate::audit::Journal::at(&root);
        std::fs::remove_dir_all(journal.index_dir()).expect("the append built an index");
        let first = read_pull_journal(&root, "ZZOIL").unwrap();
        assert!(first.present);
        assert_eq!((first.records, first.read_now), (2, 2));
        assert!(
            matches!(first.via, Via::Walk { ref why } if why.contains("no index")),
            "{:?}",
            first.via
        );
        let again = read_pull_journal(&root, "ZZOIL").unwrap();
        assert_eq!(again.read_now, 0);
        // A TORN TAIL AFTER A RECORD WRITTEN WITH NO INDEX: still walked.
        let at = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_266_622);
        let image = crate::audit::Record::refused(
            crate::audit::Scope::Spot,
            crate::audit::Outcome::Failed,
            at,
            "zerodha_nse_checked fixture",
            "fixture run",
        )
        .image();
        let mut bytes = std::fs::read(&journal.path).unwrap();
        bytes.extend_from_slice(&image);
        bytes.extend_from_slice(b"torn");
        std::fs::write(&journal.path, bytes).unwrap();
        let tail = read_pull_journal(&root, "ZZOIL").unwrap();
        assert_eq!((tail.records, tail.read_now, tail.torn), (3, 1, 4));
        assert_eq!(tail.parsed.by_symbol["ZZOIL"].len(), 1);
    }

    /// **THROUGH THE INDEX THE READER READS THE ASKED SYMBOL'S RECORDS AND
    /// NOTHING ELSE** (WP-33c, DB-A34): the journal's own appends keep the
    /// index, so a read touches O(runs) entries and the one symbol's refusal
    /// records, files each under the run before it, never reads another
    /// symbol's, and decodes a record past the index's cover in order.
    #[test]
    fn through_the_index_only_the_asked_symbols_records_are_read() {
        let root = root_of("pull-index");
        let whole = "conflicting vendor candles at 1578853800; refused before append";
        pull_run(
            &root,
            "Reference indices",
            &[("ZZOIL", "2018-01-01", "2026-09-30", whole)],
        );
        pull_run(
            &root,
            "zerodha_nse_checked fixture",
            &[
                ("ZZOIL", "2018-01-01", "2026-09-30", whole),
                ("ZZOTHER", "2018-01-01", "2026-09-30", whole),
                (
                    "ZZOIL",
                    "2018-01-01",
                    "2026-09-30",
                    "request minute coverage UNVERIFIED",
                ),
            ],
        );
        let read = read_pull_journal(&root, "ZZOIL").unwrap();
        assert_eq!(read.records, 6);
        assert_eq!(
            read.via,
            Via::Index {
                covered: 6,
                runs: 2
            }
        );
        assert_eq!(read.read_now, 2, "the two ZZOIL refusals, and nothing else");
        assert_eq!(read.parsed.by_symbol["ZZOIL"].len(), 1, "Zerodha's only");
        assert_eq!(read.parsed.other_vendor, 1, "the reference run's member");
        assert!(!read.parsed.by_symbol.contains_key("ZZOTHER"));
        // A RECORD PAST THE COVER (written by a build with no index) is read
        // in order, under the run before it.
        let journal = crate::audit::Journal::at(&root);
        let at = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_791_266_622);
        let late = crate::audit::Record::member_failure(
            crate::audit::Scope::Spot,
            at,
            "ZZOIL",
            pull::session::Window::new(day("2018-01-01"), day("2026-09-30")).unwrap(),
            whole,
        );
        let mut bytes = std::fs::read(&journal.path).unwrap();
        bytes.extend_from_slice(&late.image());
        std::fs::write(&journal.path, bytes).unwrap();
        let read = read_pull_journal(&root, "ZZOIL").unwrap();
        assert_eq!(read.records, 7);
        assert_eq!(
            read.via,
            Via::Index {
                covered: 6,
                runs: 2
            }
        );
        assert_eq!(
            read.read_now, 3,
            "the two indexed, and the one past the cover"
        );
        assert_eq!(read.parsed.by_symbol["ZZOIL"].len(), 2);
    }

    /// Put the walk-at-start mark on `path`'s tail, with nothing read: a walk
    /// at start under way.
    fn mark_walking(path: &Path) {
        use std::os::unix::fs::MetadataExt as _;
        let meta = std::fs::metadata(path).expect("the journal");
        let mut tail = PullTail::fresh(Some((meta.dev(), meta.ino())));
        tail.warming = Some(std::time::Instant::now());
        pull_tails()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path.to_path_buf(), tail);
    }

    /// Whether `path`'s tail carries the walk-at-start mark.
    fn walking(path: &Path) -> bool {
        pull_tails()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(path)
            .is_some_and(|tail| tail.warming.is_some())
    }

    /// **THE PULL JOURNAL IS WALKED ONCE AT START, OFF THE REQUEST PATH, AND A
    /// REQUEST THAT ARRIVES DURING THAT WALK DOES NOT WALK IT BESIDE IT: IT IS
    /// ANSWERED WITHOUT THE JOURNAL AND SAYS WHY.** DB-N19: the first Zerodha
    /// spot page after a start paid the whole walk, 0.73 s on a shadow of the
    /// operator's store.
    #[test]
    fn the_pull_journal_is_walked_at_start_and_a_request_during_the_walk_says_so() {
        let root = root_of("pull-warm");
        assert_eq!(
            warm_pull_journal(&root),
            Ok(PullWarmed {
                present: false,
                read: 0,
                micros: 0,
            }),
            "no journal: nothing to walk"
        );
        pull_run(
            &root,
            "zerodha_nse_checked fixture",
            &[(
                "ZZOIL",
                "2018-01-01",
                "2026-09-30",
                "conflicting vendor candles at 1578853800; refused before append",
            )],
        );
        let path = crate::audit::journal_path(&root);
        // A JOURNAL ITS INDEX COVERS IS NOT WALKED AT START (WP-33c): a
        // request reads it through the index.
        assert_eq!(warm_pull_journal(&root).map(|walked| walked.read), Ok(0));
        // A JOURNAL WRITTEN BEFORE THE INDEX (the operator's, until its first
        // append under WP-33c) is walked at start, and a request during the
        // walk does not walk beside it.
        std::fs::remove_dir_all(crate::audit::Journal::at(&root).index_dir())
            .expect("the appends built an index");

        // THE WALK AT START IS UNDER WAY: its mark is on the tail, and nothing
        // is installed yet.
        mark_walking(&path);
        match read_pull_journal(&root, "ZZOIL") {
            Err(PullUnread::Warming { records, .. }) => assert_eq!(records, 2),
            Err(PullUnread::Failed(why)) => panic!("the journal is there: {why}"),
            Ok(read) => panic!(
                "a request does not walk beside the start; it read {} record(s)",
                read.read_now
            ),
        }
        let got = answer(
            &root,
            &ask("feed=zerodha&instrument=NSE-INDEX-ZZOIL&from=2020-01-13"),
        );
        assert_eq!(
            got["sources"]["pull_journal"]["state"], "walking at start",
            "{:#}",
            got["sources"]
        );
        assert_eq!(got["sources"]["pull_journal"]["records"], 2);
        assert!(
            got["notes"]
                .as_array()
                .expect("notes")
                .iter()
                .any(|note| note
                    .as_str()
                    .is_some_and(|n| n.contains("being walked by this server's start"))),
            "{:#}",
            got["notes"]
        );
        assert_eq!(
            record(&got, "2020-01-13")["pull"],
            Value::Null,
            "no journal word on the day, rather than a word read from nothing"
        );

        // THE WALK ITSELF: a second walk while one is marked reads nothing.
        assert_eq!(
            warm_pull_journal(&root).map(|walked| walked.read),
            Ok(0),
            "one walk at a time"
        );
        pull_tails()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&path);
        let walked = warm_pull_journal(&root).expect("the walk at start");
        assert!(walked.present);
        assert_eq!(walked.read, 2, "every record, once");
        let after = read_pull_journal(&root, "ZZOIL").expect("a tail read");
        assert_eq!(
            (after.records, after.read_now),
            (2, 0),
            "the request after the walk reads only what was appended since"
        );
        assert_eq!(
            after.parsed.by_symbol["ZZOIL"].len(),
            1,
            "the walk was installed"
        );
        assert!(!walking(&path), "and its mark is off");
        assert_eq!(
            warm_pull_journal(&root).map(|walked| walked.read),
            Ok(0),
            "walked already: nothing to read"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE ROUTE, OVER THE REAL ROUTER STATE**: 200 with one record a day,
    /// and 400 naming the parameter it could not read.
    #[tokio::test]
    async fn the_route_answers_a_page_and_refuses_by_name() {
        let dir = agreeing("dayflags-route");
        let mut built = site("dayflags-route", &dir);
        built.store_root = root_of("route");
        let loaded: Loaded = std::sync::Arc::new(built);
        let (status, headers, body) = dayflags_json(
            axum::extract::State(std::sync::Arc::clone(&loaded)),
            "/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-03-01&to=2020-03-31"
                .parse::<axum::http::Uri>()
                .unwrap(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::OK, "{body}");
        assert_eq!(headers[0].1, "application/json; charset=utf-8");
        let got: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(got["days"].as_array().unwrap().len(), 31);
        assert_eq!(record(&got, "2020-03-13")["calendar"]["code"], "H");
        let (status, _, body) = dayflags_json(
            axum::extract::State(loaded),
            "/dayflags.json?feed=gdfl&symbol=NIFTY&from=2020-03-01"
                .parse::<axum::http::Uri>()
                .unwrap(),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert!(body.contains("instrument"), "{body}");
    }

    // ── WP-11c: the sampled day and the bars inside a halt (DB-A04, DB-A07) ─

    /// `n` one-second bars on `on`, one a minute at second :59 from 09:15.
    fn minute_stamped(on: &str, n: i64) -> Vec<Bar> {
        (0..n)
            .map(|m| bar(on, 9 * 3_600 + 15 * 60 + m * 60 + 59, [700 + m; 4], 0))
            .collect()
    }

    /// **A SAMPLED DAY WITH NO LEDGER LINE IS STILL NAMED, AS A SUSPICION**
    /// (DB-A04): 30 May 2019 was imported before the day ledger existed, so
    /// the store's own count answers -- 375 one-second bars against a
    /// 22,500-second session is at most two a minute -- and says it cannot
    /// tell a sampled day from one mostly missing. The edge is two bars a
    /// session minute; an ordinary day, a closed day, a share, an index the
    /// store does not hold and a one-minute feed are each not named.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one fixture and every assertion about it, read top to bottom"
    )]
    fn a_sampled_day_without_a_ledger_line_is_suspected_from_the_bars_held() {
        let root = root_of("sampled-held");
        let mut bars = minute_stamped("2019-05-30", 375);
        // 2019-05-29 holds 750 (the edge: two a minute), 2019-05-28 holds 751.
        for (on, n) in [("2019-05-29", 750_i64), ("2019-05-28", 751)] {
            bars.extend((0..n).map(|s| bar(on, 9 * 3_600 + 15 * 60 + s, [600; 4], 0)));
        }
        file_bars(
            &root,
            Vendor::Gdfl,
            "NSE-INDEX-NIFTY",
            Timeframe::SECOND_1,
            bars,
        );
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-INDEX-NIFTY",
            Timeframe::MINUTE_1,
            minutes("2019-05-30", 375),
        );
        let page = |feed: &str, instrument: &str| {
            answer(
                &root,
                &ask(&format!(
                    "feed={feed}&instrument={instrument}&from=2019-05-25&to=2019-05-31"
                )),
            )
        };
        let got = page("gdfl", "NSE-INDEX-NIFTY");
        let sampled = record(&got, "2019-05-30");
        assert_eq!(
            sampled["sampled"],
            json!({
                "is": true, "of": "own", "series": "NSE-INDEX-NIFTY",
                "basis": "held_bars", "standing": "suspected",
                "held_s": 375, "session_s": 22500, "ss_distinct": null, "line": null,
            })
        );
        assert_eq!(sampled["degraded"][0]["flag"], "sampled");
        assert_eq!(sampled["degraded"][0]["rule"], sampled_rule_held());
        says_has(
            sampled,
            "degraded vendor day -- index: minute-sampled or mostly missing (suspected), 375 of \
             22500 session second(s) held (1.7%), at most 2 a minute; no day-ledger line measures \
             its rows yet",
        );
        let edge = record(&got, "2019-05-29");
        assert_eq!(edge["sampled"]["is"], true, "750 is two a session minute");
        let over = record(&got, "2019-05-28");
        assert_eq!(
            over["sampled"]["is"], false,
            "751 is more than two a minute"
        );
        assert_eq!(over["sampled"]["held_s"], 751);
        assert_eq!(over["degraded"], json!([]));
        // A SESSION HOLDING NOTHING is "no GDFL bar", not a sampled day.
        let empty = record(&got, "2019-05-27");
        assert_eq!(empty["sampled"]["is"], false);
        assert_eq!(empty["sampled"]["held_s"], 0);
        assert_eq!(empty["degraded"], json!([]));
        // A CLOSED DAY has no session to be sampled in.
        assert_eq!(record(&got, "2019-05-25")["sampled"], Value::Null);
        // A ONE-MINUTE FEED cannot show it and is never asked.
        let zerodha = page("zerodha", "NSE-INDEX-NIFTY");
        assert_eq!(record(&zerodha, "2019-05-30")["sampled"], Value::Null);
        assert_eq!(record(&zerodha, "2019-05-30")["degraded"], json!([]));
        // A SHARE'S OWN SECONDS ARE TRADES: none is named.
        let share = page("gdfl", "NSE-CASH-RELIANCE");
        assert_eq!(record(&share, "2019-05-30")["sampled"], Value::Null);
        // AN OPTION NAMES ITS UNDERLYING INDEX, a contract and the kind alike.
        for instrument in ["NSE-FNO-NIFTY-2019-05-30-1190000-CE", "NSE-FNO-NIFTY"] {
            let option = page("gdfl", instrument);
            let rec = record(&option, "2019-05-30");
            assert_eq!(rec["sampled"]["is"], true, "{instrument}");
            assert_eq!(rec["sampled"]["of"], "underlying");
            assert_eq!(rec["sampled"]["series"], "NSE-INDEX-NIFTY");
            assert_eq!(rec["sampled"]["held_s"], 375);
            assert_eq!(rec["quality"]["indices"], Value::Null, "its own lines only");
            says_has(
                rec,
                "degraded vendor day -- underlying index NSE-INDEX-NIFTY: minute-sampled or \
                 mostly missing (suspected), 375 of 22500 session second(s) held (1.7%), at most \
                 2 a minute; no day-ledger line measures its rows yet",
            );
            assert_eq!(record(&option, "2019-05-28")["sampled"]["is"], false);
        }
        // AN UNDERLYING THAT IS A SHARE holds no index month: nothing is said.
        let stock_option = page("gdfl", "NSE-FNO-RELIANCE-2019-05-30-130000-CE");
        assert_eq!(record(&stock_option, "2019-05-30")["sampled"], Value::Null);
        assert_eq!(record(&stock_option, "2019-05-30")["degraded"], json!([]));
        // THE LEDGER'S LINE, ONCE WRITTEN, IS THE AUTHORITY: measured, for
        // the index and for the option that reads it.
        write(
            &quality_path(&root),
            &index_flags(
                "2019-05-30",
                "NIFTY",
                &[
                    ("rows", "376"),
                    ("seconds", "375"),
                    ("session_s", "22500"),
                    ("ss_distinct", "1"),
                ],
            ),
        );
        for (instrument, of) in [
            ("NSE-INDEX-NIFTY", "own"),
            ("NSE-FNO-NIFTY-2019-05-30-1190000-CE", "underlying"),
        ] {
            let got = page("gdfl", instrument);
            let rec = record(&got, "2019-05-30");
            assert_eq!(rec["sampled"]["basis"], "day_ledger", "{instrument}");
            assert_eq!(rec["sampled"]["standing"], "measured");
            assert_eq!(rec["sampled"]["of"], of);
            assert_eq!(rec["sampled"]["ss_distinct"], 1);
            assert_eq!(rec["degraded"][0]["rule"], sampled_rule());
            // A day the ledger has no line for is still the store's to say.
            assert_eq!(record(&got, "2019-05-29")["sampled"]["basis"], "held_bars");
        }
        assert_eq!(got["rules"]["sampled"]["day_ledger"], sampled_rule());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE BARS INSIDE A HALT ARE COUNTED AND LABELLED, NEVER DROPPED**
    /// (DB-A07): each segment by its own halt seconds on 2020-03-13 (the
    /// index from 09:21:10 up to 10:21:01, the shares from 09:21:09, the
    /// index options from 09:21:45, both up to 10:21:00), a one-minute feed
    /// by its minute opens, a halt day holding nothing inside it, a halt day
    /// with no month, and a day with no halt.
    #[test]
    fn the_bars_inside_a_halt_are_counted_by_the_segments_own_seconds() {
        let root = root_of("halt-held");
        let at = |h: i64, m: i64, s: i64| h * 3_600 + m * 60 + s;
        let on = "2020-03-13";
        let seconds =
            |list: &[i64]| -> Vec<Bar> { list.iter().map(|&s| bar(on, s, [900; 4], 0)).collect() };
        let edges = [
            at(9, 21, 8),
            at(9, 21, 9),
            at(9, 21, 10),
            at(9, 21, 44),
            at(9, 21, 45),
            at(10, 20, 59),
            at(10, 21, 0),
            at(10, 21, 1),
        ];
        let option = "NSE-FNO-NIFTY-2020-03-19-950000-PE";
        for key in ["NSE-INDEX-NIFTY", "NSE-CASH-RELIANCE", option] {
            file_bars(
                &root,
                Vendor::Gdfl,
                key,
                Timeframe::SECOND_1,
                seconds(&edges),
            );
        }
        // BANKNIFTY holds the day and nothing inside the halt.
        file_bars(
            &root,
            Vendor::Gdfl,
            "NSE-INDEX-BANKNIFTY",
            Timeframe::SECOND_1,
            seconds(&[at(9, 15, 0), at(15, 29, 59)]),
        );
        // Zerodha's minutes: 09:21 opens before the index's halt, 09:22 inside.
        file_bars(
            &root,
            Vendor::Zerodha,
            "NSE-INDEX-NIFTY",
            Timeframe::MINUTE_1,
            seconds(&[at(9, 21, 0), at(9, 22, 0), at(10, 21, 0), at(10, 22, 0)]),
        );
        let get = |feed: &str, instrument: &str, day: &str| {
            let got = answer(
                &root,
                &ask(&format!("feed={feed}&instrument={instrument}&from={day}")),
            );
            record(&got, day).clone()
        };
        for (instrument, inside, window) in [
            ("NSE-INDEX-NIFTY", 5, "09:21:10 until 10:21:01"),
            ("NSE-CASH-RELIANCE", 5, "09:21:09 until 10:21:00"),
            (option, 2, "09:21:45 until 10:21:00"),
        ] {
            let rec = get("gdfl", instrument, on);
            assert_eq!(rec["held"]["bars"], 8, "{instrument}: nothing is dropped");
            assert_eq!(
                rec["events"]["halt"]["held"]["bars"], inside,
                "{instrument}"
            );
            assert_eq!(rec["events"]["halt"]["held"]["feed"], "gdfl");
            assert_eq!(rec["events"]["halt"]["held"]["rung"], "1s");
            says_has(
                &rec,
                &format!(
                    "{inside} 1s bar(s) held inside the halt, {window}: kept as printed, \
                     labelled, never dropped"
                ),
            );
        }
        let index = get("gdfl", "NSE-INDEX-NIFTY", on);
        assert_eq!(index["events"]["halt"]["held"]["of_s"], 3_591);
        let none = get("gdfl", "NSE-INDEX-BANKNIFTY", on);
        assert_eq!(none["events"]["halt"]["held"]["bars"], 0);
        says_has(
            &none,
            "no 1s bar held inside the halt, 09:21:10 until 10:21:01",
        );
        let minutes = get("zerodha", "NSE-INDEX-NIFTY", on);
        assert_eq!(
            minutes["events"]["halt"]["held"]["bars"], 2,
            "09:22 and 10:21"
        );
        assert_eq!(minutes["events"]["halt"]["held"]["rung"], "1min");
        // A HALT DAY WHOSE MONTH IS NOT HELD: the halt is named, the count is
        // null and no phrase claims one.
        let absent = get("gdfl", "NSE-INDEX-NIFTYIT", on);
        assert_eq!(absent["events"]["halt"]["standing"], "suspected");
        assert_eq!(absent["events"]["halt"]["held"], Value::Null);
        assert!(
            says(&absent).iter().all(|s| !s.contains("inside the halt")),
            "{absent:#}"
        );
        // A DAY WITH NO HALT has none to count in.
        let quiet = get("gdfl", "NSE-INDEX-NIFTY", "2020-03-12");
        assert_eq!(quiet["events"]["halt"], Value::Null);
        assert!(
            says(&quiet).iter().all(|s| !s.contains("halt")),
            "{quiet:#}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **AN OPTION'S UNDERLYING INDEX IS THE ALIAS TABLE'S, NOT THE OPTION'S
    /// OWN SYMBOL** (WP-11c repair 1; D-3332, D-4928 item 5): a FINNIFTY
    /// option's index is filed under `NIFTYFINSERVICE`, and so is its
    /// day-ledger line. Every index the table files under a key other than
    /// its F&O symbol is asked, from the table itself, as a contract and as
    /// the kind, by the bars held and by the ledger's line; a share's option
    /// -- renamed or not -- names no index and opens none.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one fixture and every assertion about it, read top to bottom"
    )]
    fn an_option_reads_the_sampled_day_of_the_index_the_alias_table_names() {
        let root = root_of("sampled-alias");
        let (on, before) = ("2022-03-07", "2022-03-04");
        let renamed: Vec<(&str, &str)> = brutex_core::alias::INDEX_ROWS
            .iter()
            .filter_map(|row| Some((row.fno?, row.store_key)))
            .filter(|(fno, key)| fno != key)
            .collect();
        assert!(
            renamed.contains(&("FINNIFTY", "NIFTYFINSERVICE")),
            "the table files FINNIFTY's index under another key: {renamed:?}"
        );
        for (_, key) in &renamed {
            let mut bars = minute_stamped(on, 375);
            bars.extend((0..751).map(|s| bar(before, 9 * 3_600 + 15 * 60 + s, [600; 4], 0)));
            file_bars(
                &root,
                Vendor::Gdfl,
                &format!("NSE-INDEX-{key}"),
                Timeframe::SECOND_1,
                bars,
            );
        }
        let page = |instrument: &str| {
            answer(
                &root,
                &ask(&format!(
                    "feed=gdfl&instrument={instrument}&from={before}&to={on}"
                )),
            )
        };
        for (fno, key) in &renamed {
            let series = format!("NSE-INDEX-{key}");
            for instrument in [
                format!("NSE-FNO-{fno}-2022-03-08-1700000-CE"),
                format!("NSE-FNO-{fno}"),
            ] {
                let got = page(&instrument);
                let rec = record(&got, on);
                assert_eq!(
                    rec["sampled"],
                    json!({
                        "is": true, "of": "underlying", "series": series,
                        "basis": "held_bars", "standing": "suspected",
                        "held_s": 375, "session_s": 22500, "ss_distinct": null, "line": null,
                    }),
                    "{instrument}"
                );
                assert_eq!(rec["degraded"][0]["flag"], "sampled", "{instrument}");
                says_has(
                    rec,
                    &format!(
                        "degraded vendor day -- underlying index {series}: minute-sampled or \
                         mostly missing (suspected), 375 of 22500 session second(s) held (1.7%), \
                         at most 2 a minute; no day-ledger line measures its rows yet"
                    ),
                );
                let full = record(&got, before);
                assert_eq!(full["sampled"]["is"], false, "{instrument}");
                assert_eq!(full["sampled"]["held_s"], 751);
                assert_eq!(full["degraded"], json!([]));
            }
            // THE INDEX ITSELF, asked under its store key, is its own series.
            let own = page(&series);
            assert_eq!(record(&own, on)["sampled"]["of"], "own", "{series}");
            assert_eq!(record(&own, on)["sampled"]["series"], series.as_str());
        }
        // A SHARE'S OPTION NAMES NO INDEX, under its own symbol or one a dated
        // change renamed: null, nothing degraded, and no month looked for.
        for instrument in [
            "NSE-FNO-RELIANCE-2022-03-31-240000-CE",
            "NSE-FNO-TATAMOTORS-2022-03-31-45000-CE",
        ] {
            let got = page(instrument);
            assert_eq!(record(&got, on)["sampled"], Value::Null, "{instrument}");
            assert_eq!(record(&got, on)["degraded"], json!([]), "{instrument}");
        }
        // THE LEDGER'S LINE IS FILED UNDER THE STORE KEY TOO, and is read there.
        let lines: String = renamed
            .iter()
            .map(|(_, key)| {
                index_flags(
                    on,
                    key,
                    &[
                        ("rows", "376"),
                        ("seconds", "375"),
                        ("session_s", "22500"),
                        ("ss_distinct", "1"),
                    ],
                )
            })
            .collect();
        write(&quality_path(&root), &lines);
        for (at, (fno, key)) in renamed.iter().enumerate() {
            let got = page(&format!("NSE-FNO-{fno}-2022-03-08-1700000-CE"));
            let rec = record(&got, on);
            assert_eq!(
                rec["sampled"],
                json!({
                    "is": true, "of": "underlying", "series": format!("NSE-INDEX-{key}"),
                    "basis": "day_ledger", "standing": "measured",
                    "held_s": 375, "session_s": 22500, "ss_distinct": 1, "line": at + 1,
                }),
                "{fno}"
            );
            assert_eq!(rec["degraded"][0]["rule"], sampled_rule());
            says_has(
                rec,
                &format!(
                    "degraded vendor day -- underlying index NSE-INDEX-{key}: minute-sampled, \
                     375 of 22500 session second(s) held (1.7%), its rows on 1 distinct \
                     second(s) of the minute"
                ),
            );
            assert_eq!(
                record(&got, before)["sampled"]["basis"],
                "held_bars",
                "a day with no line is still the store's to say"
            );
            let rules = &got["rules"]["sampled"];
            assert!(
                rules["of"].as_str().unwrap().contains("D-3332"),
                "{rules:#}"
            );
            assert!(
                rules["null"]
                    .as_str()
                    .unwrap()
                    .contains("an option whose underlying is a share"),
                "{rules:#}"
            );
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The path of `key`'s month file of `kind` holding `on`.
    fn month_file(
        root: &Path,
        (feed, key, rung): (Vendor, &str, Timeframe),
        on: &str,
        kind: FileKind,
    ) -> PathBuf {
        let inst = instrument_of(key, "").expect("a key");
        StorePath::new(PathParts {
            vendor: feed,
            exchange: inst.exchange.as_str(),
            segment: inst.segment.as_str(),
            symbol: &inst.symbol,
            contract: inst.contract,
            timeframe: rung,
            month: day(on).year_month().unwrap(),
            file: kind,
        })
        .expect("a legal path")
        .to_path_buf(root)
    }

    /// Spoils record `row` of a sealed month file, so every read of its
    /// checksum block is refused, and takes the month's time index away, so
    /// each lookup is the plain bisection whose probes the tests below count.
    fn spoil_block_of(root: &Path, series: (Vendor, &str, Timeframe), on: &str, row: u64) {
        // A month under the small-month threshold has none to take away
        // (D-4933); one past it does.
        match std::fs::remove_file(month_file(root, series, on, FileKind::TimeIndex)) {
            Ok(()) => {}
            Err(gone) if gone.kind() == std::io::ErrorKind::NotFound => {}
            Err(other) => panic!("the month's time index: {other}"),
        }
        let path = month_file(root, series, on, FileKind::Bars);
        let mut bytes = std::fs::read(&path).expect("the month's bars");
        let at = usize::try_from(store::format::HEADER_LEN + row * store::format::RECORD_STRIDE)
            .expect("an offset");
        for byte in &mut bytes[at + 8..at + 40] {
            *byte ^= 0xA5;
        }
        std::fs::write(&path, bytes).expect("the spoiled bars");
    }

    /// **A DAY WHOSE LOOKUP FAILS DOES NOT SHIFT THE HALT COUNTS OF THE DAYS
    /// AFTER IT** (WP-11c repair 1): the three per-day answers are pushed
    /// together. The month holds 584 one-second bars (eight checksum blocks
    /// of 73): 100 on the 10th, 192 on the 11th, 292 on the halt day. Block 1
    /// (records 73..146) is spoiled. Bisecting for the 11th's opening probes
    /// 292, 146, then 73 -- refused; the lookups for the 12th, the 13th and
    /// the halt's two edges probe blocks 2 to 7 only. Before the repair the
    /// 13th read the 14th's count: `held` null, with no reason.
    ///
    /// **AND A HALT WHOSE OWN COUNT FAILS SAYS SO**: BANKNIFTY's day holds
    /// 1,168 bars, the halt opens at record 400 and block 5 (365..438) is
    /// spoiled; the day's two edges probe blocks 0, 1, 2, 4, 8 and 12 to 15.
    #[test]
    fn a_failed_lookup_keeps_each_later_days_halt_count_on_its_own_day() {
        let root = root_of("halt-failed-lookup");
        let at = |h: i64, m: i64, s: i64| h * 3_600 + m * 60 + s;
        let run = |on: &str, from: i64, n: i64| -> Vec<Bar> {
            (0..n).map(|i| bar(on, from + i, [900; 4], 0)).collect()
        };
        let nifty = (Vendor::Gdfl, "NSE-INDEX-NIFTY", Timeframe::SECOND_1);
        let mut bars = run("2020-03-10", at(9, 15, 0), 100);
        bars.extend(run("2020-03-11", at(9, 15, 0), 192));
        // 09:20:00 to 09:24:51: the index's halt opens at 09:21:10, 70 in.
        bars.extend(run("2020-03-13", at(9, 20, 0), 292));
        file_bars(&root, nifty.0, nifty.1, nifty.2, bars);
        spoil_block_of(&root, nifty, "2020-03-13", 100);
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-03-11&to=2020-03-13"),
        );
        let notes: Vec<&str> = got["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(record(&got, "2020-03-11")["held"]["why"], "unreadable");
        assert_eq!(record(&got, "2020-03-11")["held"]["bars"], Value::Null);
        assert!(
            notes
                .iter()
                .any(|n| n.starts_with("gdfl NSE-INDEX-NIFTY 1s 2020-03-11: a lookup failed: ")),
            "{notes:#?}"
        );
        assert_eq!(record(&got, "2020-03-12")["held"]["bars"], 0);
        assert_eq!(record(&got, "2020-03-12")["events"]["halt"], Value::Null);
        let halt = record(&got, "2020-03-13");
        assert_eq!(halt["held"]["bars"], 292);
        assert_eq!(
            halt["events"]["halt"]["held"],
            json!({ "feed": "gdfl", "rung": "1s", "bars": 222, "of_s": 3_591 }),
            "the halt day's own count, not the day after's: {notes:#?}"
        );
        says_has(
            halt,
            "222 1s bar(s) held inside the halt, 09:21:10 until 10:21:01: kept as printed, \
             labelled, never dropped",
        );

        let bank = (Vendor::Gdfl, "NSE-INDEX-BANKNIFTY", Timeframe::SECOND_1);
        let mut bars = run("2020-03-13", at(9, 10, 0), 400);
        bars.extend(run("2020-03-13", at(9, 21, 10), 500));
        bars.extend(run("2020-03-13", at(10, 30, 0), 268));
        file_bars(&root, bank.0, bank.1, bank.2, bars);
        spoil_block_of(&root, bank, "2020-03-13", 400);
        let got = answer(
            &root,
            &ask("feed=gdfl&instrument=NSE-INDEX-BANKNIFTY&from=2020-03-13&to=2020-03-13"),
        );
        let rec = record(&got, "2020-03-13");
        assert_eq!(rec["held"]["bars"], 1_168, "the day itself is counted");
        assert_eq!(rec["events"]["halt"]["standing"], "suspected");
        assert_eq!(
            rec["events"]["halt"]["held"],
            Value::Null,
            "never a count the store did not give"
        );
        assert!(
            says(rec).iter().all(|s| !s.contains("inside the halt")),
            "{rec:#}"
        );
        let said: Vec<&str> = got["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .filter_map(Value::as_str)
            .filter(|n| n.contains("inside the halt"))
            .collect();
        assert_eq!(said.len(), 1, "{:#}", got["notes"]);
        assert!(
            said[0].starts_with(
                "gdfl NSE-INDEX-BANKNIFTY 1s 2020-03-13: the bars inside the halt could not be \
                 counted: "
            ),
            "{said:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE IMPORT'S HALT AND FLAT-RUN FIELDS ARE READ WHERE A LINE HAS
    /// THEM, AND ARE `null` WHERE IT DOES NOT** (WP-11c): a line written
    /// before the fields stays valid and never reads as zero; one that
    /// carries them is said; one that carries a field twice, or malformed,
    /// is counted as a fault with its reason.
    #[test]
    fn the_halt_fields_of_a_ledger_line_are_optional_and_never_read_as_zero() {
        let root = root_of("halt-fields");
        let later = |line: String, more: &str| format!("{} {more}\n", line.trim_end());
        let mut ledger = index_flags("2020-03-13", "BANKNIFTY", &[("rows", "9")]);
        ledger.push_str(&later(
            index_flags(
                "2020-03-13",
                "NIFTY",
                &[("rows", "22501"), ("ss_distinct", "60")],
            ),
            "halt_s=3591 halt_held_s=3591 flat_max_s=2692 flat_max_at=09:21:09",
        ));
        ledger.push_str(&later(
            flags_line(
                "stocks",
                "2020-03-13",
                "*",
                &STOCKS_FIELDS,
                &[("files", "3")],
            ),
            "halt_s=3591 halt_held_s=13",
        ));
        ledger.push_str(&later(
            index_flags("2020-03-23", "NIFTY", &[("rows", "5")]),
            "halt_s=3532 halt_s=3532",
        ));
        ledger.push_str(&later(
            index_flags("2020-03-23", "BANKNIFTY", &[("rows", "5")]),
            "flat_max_at=25:00:00",
        ));
        write(&quality_path(&root), &ledger);
        let get = |instrument: &str, day: &str| {
            let got = answer(
                &root,
                &ask(&format!("feed=gdfl&instrument={instrument}&from={day}")),
            );
            (record(&got, day).clone(), got)
        };
        let (with, _) = get("NSE-INDEX-NIFTY", "2020-03-13");
        assert_eq!(with["quality"]["indices"]["halt_s"], 3_591);
        assert_eq!(with["quality"]["indices"]["halt_held_s"], 3_591);
        assert_eq!(with["quality"]["indices"]["flat_max_s"], 2_692);
        assert_eq!(with["quality"]["indices"]["flat_max_at"], "09:21:09");
        says_has(
            &with,
            "the import placed 3591 of the halt's 3591 session second(s)",
        );
        says_has(
            &with,
            "the index value stood unchanged for 2692 s from 09:21:09",
        );
        assert_eq!(with["quality"]["stocks"]["halt_held_s"], 13);
        assert_eq!(
            with["quality"]["stocks"]["flat_max_s"],
            Value::Null,
            "an index's field, not a share day's: the key is not there"
        );
        let (before, _) = get("NSE-INDEX-BANKNIFTY", "2020-03-13");
        assert_eq!(before["quality"]["indices"]["rows"], 9, "the line is read");
        for name in ["halt_s", "halt_held_s", "flat_max_s", "flat_max_at"] {
            assert!(
                before["quality"]["indices"]
                    .as_object()
                    .unwrap()
                    .contains_key(name),
                "{name} is a key of every index line"
            );
            assert_eq!(before["quality"]["indices"][name], Value::Null, "{name}");
        }
        assert!(
            says(&before)
                .iter()
                .all(|s| !s.starts_with("the import placed")
                    && !s.starts_with("the index value stood")),
            "{before:#}"
        );
        let (share, _) = get("NSE-CASH-RELIANCE", "2020-03-13");
        says_has(
            &share,
            "stocks: the day's files hold 13 second(s) inside the halt's 3591",
        );
        let (twice, got) = get("NSE-INDEX-NIFTY", "2020-03-23");
        assert_eq!(
            twice["quality"]["indices"],
            Value::Null,
            "the line is refused"
        );
        let faults = got["sources"]["quality"]["faults"].to_string();
        assert!(faults.contains("halt_s appears twice"), "{faults}");
        assert!(faults.contains("flat_max_at="), "{faults}");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **THE REAL DAYS, AND THE ANSWER'S COST** (WP-11c; DB-A04, DB-A07): over
    /// a store root named by `BRUTEX_DAYFLAGS_STORE` (a shadow of the real
    /// store; read-only), 2019-05-30 is labelled sampled and the two March
    /// 2020 halts count the bars held inside them, on NIFTY, BANKNIFTY, a
    /// share and option contracts; then one 92-day page is answered 200 times
    /// for an index, a share and an option and the p50, p99 and max are
    /// printed. Run with `-- --ignored --nocapture`.
    #[test]
    #[ignore = "requires BRUTEX_DAYFLAGS_STORE (a store root holding the GDFL days); read-only"]
    #[allow(
        clippy::print_stdout,
        clippy::too_many_lines,
        reason = "a probe prints what it read, and reads top to bottom"
    )]
    fn the_real_days_are_labelled_and_the_answer_is_timed() {
        let root = PathBuf::from(
            std::env::var_os("BRUTEX_DAYFLAGS_STORE")
                .expect("BRUTEX_DAYFLAGS_STORE names a store root"),
        );
        let get = |instrument: &str, on: &str| -> Value {
            let got = answer(
                &root,
                &ask(&format!(
                    "feed=gdfl&instrument={instrument}&from={on}&to={on}"
                )),
            );
            let rec = record(&got, on).clone();
            println!(
                "{instrument} {on}: held={} code={} sampled={} halt_held={} degraded={} says={}",
                rec["held"]["bars"],
                rec["calendar"]["code"],
                rec["sampled"],
                rec["events"]["halt"]["held"],
                rec["degraded"],
                rec["says"]
            );
            rec
        };
        let sampled_day = [
            "NSE-INDEX-NIFTY",
            "NSE-INDEX-BANKNIFTY",
            "NSE-FNO-NIFTY-2019-05-30-1190000-CE",
            "NSE-FNO-NIFTY-2019-05-30-1300000-PE",
            "NSE-CASH-RELIANCE",
        ]
        .map(|instrument| get(instrument, "2019-05-30"));
        let ordinary = get("NSE-INDEX-NIFTY", "2019-05-29");
        let halts = [
            ("NSE-INDEX-NIFTY", "2020-03-13"),
            ("NSE-INDEX-NIFTY", "2020-03-23"),
            ("NSE-INDEX-BANKNIFTY", "2020-03-13"),
            ("NSE-FNO-NIFTY-2020-03-19-950000-PE", "2020-03-13"),
            ("NSE-FNO-NIFTY-2020-03-26-1250000-CE", "2020-03-23"),
            ("NSE-FNO-BANKNIFTY-2020-03-19-2500000-PE", "2020-03-13"),
            ("NSE-CASH-RELIANCE", "2020-03-13"),
        ]
        .map(|(instrument, on)| get(instrument, on));
        for query in [
            "feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-01-01&to=2020-03-31",
            "feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2020-01-01&to=2020-03-31",
            "feed=gdfl&instrument=NSE-FNO-NIFTY-2020-03-26-950000-PE&from=2020-01-01&to=2020-03-31",
            "feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2020-01-01&to=2020-03-31",
        ] {
            let asked = ask(query);
            let _warm = answer(&root, &asked);
            let mut micros: Vec<u128> = (0..200)
                .map(|_| {
                    let at = std::time::Instant::now();
                    let got = answer(&root, &asked);
                    let took = at.elapsed().as_micros();
                    assert_eq!(got["days"].as_array().map(Vec::len), Some(91), "{query}");
                    took
                })
                .collect();
            micros.sort_unstable();
            println!(
                "TIMED n=200 {query}: p50={} us p99={} us max={} us",
                micros[99], micros[197], micros[199]
            );
        }
        // THE SAMPLED DAY: both indices, and each NIFTY option through its
        // underlying; the share's own series is not an index and says nothing.
        for rec in &sampled_day[..2] {
            assert_eq!(rec["sampled"]["is"], true, "{rec:#}");
            assert_eq!(rec["sampled"]["of"], "own", "{rec:#}");
            assert!(
                says(rec).iter().any(|s| s.contains("minute-sampled")),
                "{rec:#}"
            );
            assert!(!rec["degraded"].as_array().unwrap().is_empty(), "{rec:#}");
        }
        for rec in &sampled_day[2..4] {
            assert_eq!(rec["sampled"]["is"], true, "{rec:#}");
            assert_eq!(rec["sampled"]["of"], "underlying", "{rec:#}");
        }
        assert_eq!(sampled_day[4]["sampled"], Value::Null);
        assert_eq!(ordinary["sampled"]["is"], false, "{ordinary:#}");
        // THE HALTS: every series counts the bars it holds inside its own
        // segment's halt seconds.
        for rec in &halts {
            assert!(rec["events"]["halt"]["held"]["bars"].is_u64(), "{rec:#}");
            assert!(
                says(rec).iter().any(|s| s.contains("inside the halt")),
                "{rec:#}"
            );
        }
    }
}
