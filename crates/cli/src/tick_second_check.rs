//! `cli tick-to-second-check [YEAR] --out DIR [--workers N] [--symbols SYM ...]
//! [--archive ZIP]`: every stored GDFL share-day rebuilt from the exact source
//! its landing recorded, under two rules, and compared with the store,
//! read-only (D-5180 to D-5186).
//!
//! # Which days, and against what
//!
//! The days are the store's own: every day a share's GDFL one-second month
//! holds a bar on, found by one indexed probe per calendar day of the month
//! (`BarFile::first_at_or_after`), for every month of YEAR, or of every year
//! the store holds when YEAR is not given. A day already answered by a row
//! of an earlier run (its newest version or a segment folded since) is not
//! read again, unless the month's `.prv` now names another landing for it:
//! a run checks only what is new, and is resumable.
//!
//! Each day is compared against **the source its landing recorded** in the
//! month's `.prv` sidecar (D-3334, `store::provenance::read`):
//!
//! - an **archive member** (`<archive>!<day zip>!<entry>`): the archive is
//!   opened READ-ONLY at the recorded path through `pull::gdfl_archive`, the
//!   day zip read in place and the member inflated in memory, never
//!   extracted. Before a byte of it is read, the member's whole name must hash
//!   (BLAKE3) to the record's, and its size and CRC-32 from the day zip's
//!   directory must be the record's: otherwise `read_failed:source_changed`.
//! - a **tick-store day** (`cm/STOCKS/<yyyy>/<MON_yyyy>/<day>.bts`): read
//!   through `pull::gdfl_tickstore::TickStore` under the root
//!   `BRUTEX_GDFL_TICKSTORE` names (the importer's variable); the day file's
//!   name and length must be the record's.
//! - **no record** (a month landed before D-3334): `read_failed:
//!   source_unrecorded`, unless `--archive` names an archive to compare with;
//!   that row is then labelled `archive_not_the_landing_source`, which is
//!   what it is.
//!
//! `--archive` adds one more comparison for every day whose landing is not a
//! member of that archive, each row labelled the same way (role `extra`).
//!
//! # One parser, one fold: the importer's own
//!
//! Every row reaches the rebuild through `pull::gdfl_cm::read_listed`, the
//! import's reader (length and CRC-32 checked against the listing, the day's
//! session, refused rows named and counted, closing-auction prints diverted),
//! under the scope a `cli gdfl-import stocks` run files
//! (`pull::gdfl_run::scope::of`). The seconds are folded by
//! `pull::gdfl_import::convert`, the import's fold, and kept or dropped by
//! `pull::session::Window::verdict`, the session authority `pull::ingest`'s
//! filter applies before a bar is stored. The bad-row limit is the reader's
//! own (`MAX_BAD_ROWS` and `BAD_ROWS_PER` today; PREC-5's two-way rule when it
//! lands there): a file it refuses whole is `read_failed:TooManyBadRows`, and
//! its counts are printed either way, with what the two-way rule would say.
//!
//! - **Rule T (trades only)** is `convert` under `ImportKind::Stocks`: a
//!   second exists when at least one row of it traded (LTQ > 0); open is the
//!   first traded LTP, high the largest, low the smallest, close the last,
//!   volume the sum of LTQ. It is what the store holds today.
//! - **Rule A (every row)** keeps exactly rule T's seconds, opens, closes
//!   and volumes, and widens each second's high and low by every row placed
//!   in it, untraded (LTQ = 0) rows included, from `convert` under
//!   `ImportKind::Indices`, the importer's every-row rule. No bar is invented
//!   for a second with no trade.
//! - **Rule R** (every row, every second) is reported only as a minute
//!   high/low match against Zerodha, never as a status.
//! - **Folded** ([`with_prints`]) is rule T with the closing-auction prints
//!   the decoder diverts (D-3320) put back in file order: the bars bar
//!   definitions 2 and 3 built. A store day equal to it and to no rule is
//!   `differs:auction_prints_folded`.
//! - **Unfiled** ([`unfiled_prints`]) is rule T with only the prints the
//!   store's own `.auc` sidecar does not hold put back: a definition that
//!   filed some of a day's prints in the sidecar and folded the others. A
//!   store day equal to it alone is `differs:auction_prints_folded` too, with
//!   `compare` `exact_T_with_unfiled_auction_prints`.
//!
//! # What a row says
//!
//! One CSV row per (share-day, role): the status (`exact_T`, `exact_A`,
//! `differs:<fields>`, `differs:auction_prints_folded`, `raw_only`,
//! `store_only`, `special_session`, `read_failed:<reason>`), the source and
//! landing compared, the counts by reason, the constant-size signatures
//! ([`Sig`]), the **fill-worsening measure** (stored seconds whose rule-A high
//! is above, or low below, the stored bar: how often a fill priced at a
//! second's extreme would be worse under rule A), and the share of Zerodha
//! one-minute bars whose high and low the rebuilt seconds match exactly
//! under T, A and R. Zerodha's history is corporate-action adjusted and GDFL
//! raw. DATA-1's per-share-day factor is merged
//! (`feed_reconcile::FactorIndex::factor_of`) but this check does not apply
//! it (NOT DONE, D-5188), so the minute match is on the RAW basis and every
//! row says so (`zerodha_basis`). A row outside
//! the day band is NOT counted: no dated, charter-sourced per-share-day band
//! exists (`outside_day_band` is `UNVERIFIED` on every row).
//!
//! # Append-only output
//!
//! Each day a run checks becomes a **segment**,
//! `tick-to-second-<year>.s<NNNNNN>.csv`, staged, made durable and linked to
//! a name that did not exist. At the end of the run every segment since the
//! year's newest version is folded with it into the next version:
//! `tick-to-second-<year>.v<NNN>.csv` (every row, sorted), `.summary.txt`
//! (what the rows say), `.timings.txt` (what this run measured) and `.idx`
//! (the fixed-stride index [`index::SecondChecks::status_of`] reads), the
//! index linked last. No file is ever replaced or rewritten. The CSV, the
//! index and the summary are byte-identical at any worker count.
//!
//! # Cost
//!
//! Per row: one decoder step, one census step and a constant number of
//! state updates; per second: one verdict and one digest update; per stored
//! second: one record read. Measured per share-day and per row in every
//! run's timings and on the real archive in `docs/06-limits.md`. Whole files
//! are held in memory, as the importer holds them.

#[path = "tick_second_index.rs"]
pub mod index;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsString;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use brutex_core::blake3::Hasher;
use brutex_core::instrument::{Exchange, InstrumentKey};
use brutex_core::price::Paisa;
use brutex_core::vendor::Vendor;
use pull::gdfl_archive::Archive;
use pull::gdfl_cm::{CmKind, CmRefusal, CmScope, CmSource, DayListing, ListedFile, read_listed};
use pull::gdfl_import::{ImportKind, Tick, convert};
use pull::gdfl_tickstore::TickStore;
use pull::session::{Day, DropReason, IST_OFFSET_SECS, Window};
use pull::vendor::Granularity;
use store::file::{BarFile, StoreError};
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};
use store::provenance::{Found, SourceKind};

use self::index::{SecondStatus, Source, Status};
use crate::{FAILED, MISUSED, OK};

/// The log target of every event this verb writes.
const TARGET: &str = "cli.tick_second_check";

/// The verb's usage line.
pub(crate) const USAGE: &str = "cli tick-to-second-check [YEAR] --out DIR [--workers N] \
                                [--symbols SYM ...] [--archive ZIP]";

/// The row count from which a share-day's fixed cost (two store opens,
/// one source member) is small beside its rows, so its time per row is the
/// per-row cost: the timings' second per-row line. A reporting split, never
/// a verdict.
const AMORTISED_ROWS: u64 = 10_000;

/// The one lock file of an output directory: two runs never fold at once.
const LOCK_NAME: &str = "tick-to-second.lock";

/// Why the minute match is on the raw basis, as every summary states it
/// (D-5188, amending D-5182). DATA-1's per-share-day factor
/// ([`crate::feed_reconcile::FactorIndex::factor_of`]) is merged; this check
/// does not apply it. A summary is append-only, so this sentence must be true
/// of every build that writes it.
const RAW_BASIS_WHY: &str = "DATA-1's per-share-day factor is merged but not applied by this \
                             check (NOT DONE, D-5188)";

/// What the verb was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ask {
    /// The calendar year whose stored days are checked; `None` is every
    /// year the store holds.
    pub(crate) year: Option<u16>,
    /// Where the run's files are written.
    pub(crate) out: PathBuf,
    /// A fixed worker count, or `None` for the doubling rule ([`schedule`]).
    pub(crate) workers: Option<usize>,
    /// The store symbols to check; `None` is every one.
    pub(crate) symbols: Option<HashSet<String>>,
    /// An archive to compare with beside the landing source.
    pub(crate) archive: Option<PathBuf>,
}

/// The most workers a host offers: `std::thread::available_parallelism`,
/// one when the host will not say.
pub(crate) fn host_cores() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get)
}

/// Reads the verb's words.
///
/// # Errors
///
/// The words a refusal shows: a year that is not one, a flag without its
/// value, an unknown flag, a worker count outside `1..=` the host's cores,
/// or no `--out`.
pub(crate) fn ask(args: &[&str]) -> Result<Ask, String> {
    let (year, rest) = match args.split_first() {
        Some((first, rest)) if !first.starts_with("--") => {
            let year: u16 = first
                .parse()
                .ok()
                .filter(|y| (1970..=9999).contains(y))
                .ok_or_else(|| format!("{first:?} is not a year (1970 to 9999)"))?;
            (Some(year), rest)
        }
        _ => (None, args),
    };
    let mut out = None;
    let mut workers = None;
    let mut archive = None;
    let mut symbols: Option<HashSet<String>> = None;
    let mut words = rest.iter().peekable();
    while let Some(word) = words.next() {
        match *word {
            "--out" => {
                let dir = words.next().ok_or("--out takes a directory")?;
                out = Some(PathBuf::from(dir));
            }
            "--archive" => {
                let zip = words.next().ok_or("--archive takes a zip")?;
                archive = Some(PathBuf::from(zip));
            }
            "--workers" => {
                let n = words.next().ok_or("--workers takes a count")?;
                let cores = host_cores();
                let n: usize = n
                    .parse()
                    .ok()
                    .filter(|n| (1..=cores).contains(n))
                    .ok_or_else(|| format!("{n:?} is not a worker count (1 to {cores})"))?;
                workers = Some(n);
            }
            "--symbols" => {
                let set = symbols.get_or_insert_with(|| HashSet::with_capacity(16));
                while let Some(next) = words.next_if(|w| !w.starts_with("--")) {
                    for name in next.split(',').filter(|n| !n.is_empty()) {
                        set.insert(name.to_owned());
                    }
                }
                if set.is_empty() {
                    return Err("--symbols takes at least one symbol".to_owned());
                }
            }
            other => return Err(format!("{other:?} is not a flag of tick-to-second-check")),
        }
    }
    let out = out.ok_or("--out DIR is required: it names where the run's files go")?;
    Ok(Ask {
        year,
        out,
        workers,
        symbols,
        archive,
    })
}

/// A constant-size signature of one share-day's seconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Sig {
    /// Seconds.
    pub(crate) seconds: u64,
    /// Sum of opens, paisa.
    pub(crate) open: i128,
    /// Sum of highs, paisa.
    pub(crate) high: i128,
    /// Sum of lows, paisa.
    pub(crate) low: i128,
    /// Sum of closes, paisa.
    pub(crate) close: i128,
    /// Sum of volumes.
    pub(crate) volume: i128,
    /// First stamp, microseconds (0 when none).
    pub(crate) first: i64,
    /// Last stamp, microseconds (0 when none).
    pub(crate) last: i64,
    /// BLAKE3 over every record in order.
    pub(crate) hash: [u8; 32],
}

/// Builds a [`Sig`] one record at a time: one digest update per record.
pub(crate) struct SigBuilder {
    sig: Sig,
    hasher: Hasher,
}

impl SigBuilder {
    /// An empty signature.
    pub(crate) fn new() -> Self {
        Self {
            sig: Sig::default(),
            hasher: Hasher::new(),
        }
    }

    /// One second record, in time order.
    pub(crate) fn push(&mut self, bar: &Bar) {
        let s = &mut self.sig;
        if s.seconds == 0 {
            s.first = bar.ts_micros;
        }
        s.last = bar.ts_micros;
        s.seconds = s.seconds.saturating_add(1);
        s.open = s.open.saturating_add(i128::from(bar.open));
        s.high = s.high.saturating_add(i128::from(bar.high));
        s.low = s.low.saturating_add(i128::from(bar.low));
        s.close = s.close.saturating_add(i128::from(bar.close));
        s.volume = s.volume.saturating_add(i128::from(bar.volume));
        let mut record = [0_u8; 56];
        let fields = [
            bar.ts_micros,
            bar.open,
            bar.high,
            bar.low,
            bar.close,
            bar.volume,
            bar.open_interest,
        ];
        for (slot, value) in record.chunks_exact_mut(8).zip(fields) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        self.hasher.update(&record);
    }

    /// The signature.
    pub(crate) fn finish(self) -> Sig {
        Sig {
            hash: self.hasher.finalize(),
            ..self.sig
        }
    }
}

/// The signature of `bars`, in their order.
pub(crate) fn sig_of(bars: &[Bar]) -> Sig {
    let mut b = SigBuilder::new();
    for bar in bars {
        b.push(bar);
    }
    b.finish()
}

/// Which fields of two signatures differ, joined by `+`.
pub(crate) fn differing(a: &Sig, b: &Sig) -> String {
    let checks = [
        ("seconds", a.seconds == b.seconds),
        ("open", a.open == b.open),
        ("high", a.high == b.high),
        ("low", a.low == b.low),
        ("close", a.close == b.close),
        ("volume", a.volume == b.volume),
        ("span", a.first == b.first && a.last == b.last),
        ("hash", a.hash == b.hash),
    ];
    let names: Vec<&str> = checks
        .iter()
        .filter(|(_, same)| !same)
        .map(|(name, _)| *name)
        .collect();
    names.join("+")
}

/// The comparison of the store with the two rebuilds: `exact_T`, `exact_A`,
/// `raw_only` (the raw file traded, the store holds nothing that day), or
/// `differs:<fields against rule T>`.
pub(crate) fn compare(t: &Sig, a: &Sig, store: &Sig) -> String {
    if store == t {
        "exact_T".to_owned()
    } else if store == a {
        "exact_A".to_owned()
    } else if t.seconds > 0 && store.seconds == 0 {
        "raw_only".to_owned()
    } else {
        format!("differs:{}", differing(t, store))
    }
}

/// The comparison of a store day equal, in every field and the digest, to
/// rule T with the closing-auction prints folded back in ([`with_prints`]):
/// the bars bar definitions 2 and 3 built, which definition 4 restates
/// (D-4878 item 10).
pub(crate) const FOLDED_COMPARE: &str = "exact_T_with_auction_prints";

/// The status of such a day: it differs from what this commit's importer
/// builds, and the one thing it differs by is named.
pub(crate) const FOLDED_STATUS: &str = "differs:auction_prints_folded";

/// The comparison of a store day equal, in every field and the digest, to
/// rule T with only the closing-auction prints the store's own `.auc`
/// sidecar does NOT hold folded back in ([`unfiled_prints`]): a bar
/// definition that diverted some of the day's prints to the sidecar and
/// folded the rest into the seconds. Its status is [`FOLDED_STATUS`] too.
pub(crate) const UNFILED_COMPARE: &str = "exact_T_with_unfiled_auction_prints";

/// The calendar's word for `day`: `regular`, `special:<kind>` from
/// `pull::calendar::special_session`, or what `kind_of` says otherwise.
pub(crate) fn session_word(day: Day) -> String {
    let epoch = i64::from(day.days_from_epoch());
    if let Some(special) = pull::calendar::special_session(epoch) {
        return format!("special:{:?}", special.kind);
    }
    match pull::calendar::kind_of(epoch) {
        pull::calendar::DayKind::Open(_) => "regular".to_owned(),
        other => format!("calendar:{other:?}")
            .split(['(', ' ', '{'])
            .next()
            .unwrap_or("calendar")
            .to_owned(),
    }
}

/// The status a row shows: a special session whose comparison was exact is
/// `special_session` (its `compare` says which rule), anything else its
/// comparison.
pub(crate) fn status_word(compare: &str, session: &str) -> String {
    let exact = compare == "exact_T" || compare == "exact_A";
    if compare == FOLDED_COMPARE || compare == UNFILED_COMPARE {
        FOLDED_STATUS.to_owned()
    } else if exact && session.starts_with("special:") {
        "special_session".to_owned()
    } else {
        compare.to_owned()
    }
}

/// The variant word of a `Debug` text: its leading identifier.
pub(crate) fn word_of(debug: &str) -> String {
    debug
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .next()
        .filter(|w| !w.is_empty())
        .unwrap_or("unnamed")
        .to_owned()
}

/// What this check counts of a file's bytes beyond what the decoder keeps:
/// one pass over the lines, each line split into at most ten fields. It
/// never makes a bar; the bars come from the importer's decoder alone. The
/// quotes are read with `Paisa::from_rupee_text_half_up`, the one half-up
/// reader, and only compared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Census {
    /// Lines after the header (an unterminated last line included).
    pub(crate) lines: u64,
    /// Lines byte-identical to the line before (the CR is not compared).
    pub(crate) identical: u64,
    /// Ten-field lines whose LTP carries a minus sign (the decoder refuses
    /// them as malformed).
    pub(crate) negative_ltp: u64,
    /// Ten-field lines whose `BuyQty`, `SellQty`, `LTQ` or `OpenInterest` carries a
    /// minus sign (the decoder refuses them as malformed).
    pub(crate) negative_qty: u64,
    /// Ten-field lines whose bid and ask are both positive and bid > ask.
    pub(crate) crossed: u64,
    /// Ten-field lines with exactly one of bid and ask positive.
    pub(crate) one_sided: u64,
    /// Steps taken: one per line. The census's own work counter, which its
    /// test reads.
    pub(crate) steps: u64,
}

/// The census of one file's bytes (see [`Census`]).
pub(crate) fn census(bytes: &[u8]) -> Census {
    let mut c = Census::default();
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    let mut lines = body.split(|&b| b == b'\n');
    let _header = lines.next();
    let mut previous: Option<&[u8]> = None;
    for raw in lines {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        c.steps = c.steps.saturating_add(1);
        c.lines = c.lines.saturating_add(1);
        if previous == Some(line) {
            c.identical = c.identical.saturating_add(1);
        }
        previous = Some(line);
        let mut fields = [&b""[..]; 10];
        let mut count = 0_usize;
        for field in line.split(|&b| b == b',') {
            if let Some(slot) = fields.get_mut(count) {
                *slot = field;
            }
            count = count.saturating_add(1);
        }
        if count != fields.len() {
            continue;
        }
        let [_, _, _, ltp, bid, bid_qty, ask, ask_qty, ltq, oi] = fields;
        let negative = |f: &[u8]| f.first() == Some(&b'-');
        if negative(ltp) {
            c.negative_ltp = c.negative_ltp.saturating_add(1);
        }
        if [bid_qty, ask_qty, ltq, oi].into_iter().any(negative) {
            c.negative_qty = c.negative_qty.saturating_add(1);
        }
        let quote = |f: &[u8]| {
            std::str::from_utf8(f)
                .ok()
                .and_then(|t| Paisa::from_rupee_text_half_up(t).ok())
                .map(Paisa::raw)
        };
        if let (Some(b), Some(a)) = (quote(bid), quote(ask)) {
            if b > 0 && a > 0 && b > a {
                c.crossed = c.crossed.saturating_add(1);
            }
            if (b > 0) != (a > 0) {
                c.one_sided = c.one_sided.saturating_add(1);
            }
        }
    }
    c
}

/// Every count of one share-day's row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    /// Data lines in the file.
    pub(crate) raw_rows: u64,
    /// Rows the decoder kept.
    pub(crate) kept_rows: u64,
    /// Decoder refusals: zero LTP.
    pub(crate) zero_ltp: u64,
    /// Decoder refusals: LTP off the two-place grid or below a tick.
    pub(crate) off_grid: u64,
    /// Decoder refusals: malformed (not ten fields, a sign, a bad time...).
    pub(crate) malformed: u64,
    /// Rows the reused reader refused when it refused the whole file
    /// (`TooManyBadRows`); 0 when it kept the file.
    pub(crate) refused_whole: u64,
    /// Census: a negative LTP.
    pub(crate) negative_ltp: u64,
    /// Census: a negative quantity.
    pub(crate) negative_qty: u64,
    /// Census: bid > ask, both positive.
    pub(crate) crossed_quote: u64,
    /// Census: one side of the quote missing.
    pub(crate) one_sided_quote: u64,
    /// Census: byte-identical consecutive lines.
    pub(crate) identical_consecutive: u64,
    /// Kept rows stamped below the running maximum stamp.
    pub(crate) out_of_order: u64,
    /// Traded rows the import placed late (`Placement::late_rows`).
    pub(crate) late_traded: u64,
    /// Kept rows with LTQ = 0.
    pub(crate) ltq0_rows: u64,
    /// In-session LTQ = 0 rows whose LTP differs from the last traded LTP.
    pub(crate) ltq0_new_price: u64,
    /// Closing-auction prints the decoder diverted.
    pub(crate) auction_prints: u64,
    /// The traded quantity of those prints, every identical row counted: what
    /// an earlier bar definition (2 or 3) folded into the seconds.
    pub(crate) auction_volume: u64,
    /// Of those prints, the ones the store's own `.auc` sidecar holds for
    /// the day ([`unfiled_prints`]): what the store's bar definition
    /// diverted rather than folded. 0 when the decoder diverted none.
    pub(crate) auction_in_sidecar: u64,
    /// Traded rows stamped in session.
    pub(crate) trades_in_session: u64,
    /// Rule-T seconds the session filter dropped before the open.
    pub(crate) seconds_before_open: u64,
    /// Rule-T seconds the session filter dropped at or after the close.
    pub(crate) seconds_after_close: u64,
    /// Rule-T seconds dropped for any other reason.
    pub(crate) seconds_dropped_other: u64,
    /// Rule-A seconds whose high rose above rule T's.
    pub(crate) a_high_widened: u64,
    /// Rule-A seconds whose low fell below rule T's.
    pub(crate) a_low_widened: u64,
}

impl Counts {
    /// The counts in [`COUNT_COLUMNS`] order.
    pub(crate) const fn values(&self) -> [u64; 22] {
        [
            self.raw_rows,
            self.kept_rows,
            self.zero_ltp,
            self.off_grid,
            self.malformed,
            self.refused_whole,
            self.negative_ltp,
            self.negative_qty,
            self.crossed_quote,
            self.one_sided_quote,
            self.identical_consecutive,
            self.out_of_order,
            self.late_traded,
            self.ltq0_rows,
            self.ltq0_new_price,
            self.auction_prints,
            self.auction_volume,
            self.auction_in_sidecar,
            self.trades_in_session,
            self.seconds_before_open,
            self.seconds_after_close,
            self.seconds_dropped_other,
        ]
    }
}

/// The count columns, in [`Counts::values`] order.
pub(crate) const COUNT_COLUMNS: [&str; 22] = [
    "raw_rows",
    "kept_rows",
    "zero_ltp",
    "off_grid",
    "malformed",
    "refused_whole",
    "negative_ltp",
    "negative_qty",
    "crossed_quote",
    "one_sided_quote",
    "identical_consecutive",
    "out_of_order",
    "late_traded",
    "ltq0_rows",
    "ltq0_new_price",
    "auction_prints",
    "auction_volume",
    "auction_in_sidecar",
    "trades_in_session",
    "seconds_before_open",
    "seconds_after_close",
    "seconds_dropped_other",
];

/// One rule's minute match against Zerodha's one-minute bars.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Match {
    /// Zerodha minutes the rule also has a second in.
    pub(crate) compared: u64,
    /// Of those, minutes whose high is equal.
    pub(crate) high: u64,
    /// Of those, minutes whose low is equal.
    pub(crate) low: u64,
}

/// The minute matches of one share-day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Minutes {
    /// Zerodha one-minute bars stored for the day.
    pub(crate) zerodha: u64,
    /// Rule T.
    pub(crate) t: Match,
    /// Rule A.
    pub(crate) a: Match,
    /// Rule R (every row, every second).
    pub(crate) r: Match,
}

/// The fill-worsening measure of one share-day: rule A against the stored
/// bars, second by second.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Worse {
    /// Stored seconds rule A also has.
    pub(crate) compared: u64,
    /// Of those, seconds whose rule-A high is above the stored high.
    pub(crate) high: u64,
    /// Of those, seconds whose rule-A low is below the stored low.
    pub(crate) low: u64,
    /// Of those, seconds with either.
    pub(crate) either: u64,
}

/// Which comparison a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Role {
    /// Against the landing's recorded source (or its absence).
    Landing,
    /// Against the archive `--archive` named, which is not the landing's
    /// recorded source.
    Extra,
}

impl Role {
    /// The CSV word.
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::Landing => "landing",
            Self::Extra => "extra",
        }
    }
}

/// What a row was compared against, as it says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Label {
    /// The comparison.
    pub(crate) role: Role,
    /// The source's kind.
    pub(crate) source: Source,
    /// The source's whole name: `<archive>!<day zip>!<entry>` or the
    /// tick-store day's path below its root.
    pub(crate) source_name: String,
    /// The `.prv` landing the day's bars came from: `prv:<ordinals>@g<the bar
    /// file's generation>:def<bar definition>`, empty when none is recorded.
    pub(crate) landing: String,
}

/// One share-day's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Row {
    /// The store symbol.
    pub(crate) symbol: String,
    /// The IST day.
    pub(crate) day: Day,
    /// What the row compared against.
    pub(crate) label: Label,
    /// The vendor file's name, empty when none was read.
    pub(crate) file: String,
    /// The status.
    pub(crate) status: String,
    /// The comparison behind the status.
    pub(crate) compare: String,
    /// The calendar's word for the day.
    pub(crate) session: String,
    /// The counts.
    pub(crate) counts: Counts,
    /// Rule T's signature.
    pub(crate) t: Sig,
    /// Rule A's signature.
    pub(crate) a: Sig,
    /// The store's signature.
    pub(crate) store: Sig,
    /// Rule T with the closing-auction prints folded back in, as bar
    /// definitions 2 and 3 built it ([`with_prints`]); `None` when the
    /// decoder diverted no print.
    pub(crate) folded: Option<Sig>,
    /// Rule T with only the prints the store's `.auc` sidecar does not hold
    /// folded back in ([`unfiled_prints`]); `None` unless the sidecar holds
    /// some of the day's prints and not all of them (otherwise it is rule T
    /// or [`Self::folded`]), or when the sidecar could not be read (the row's
    /// `detail` then says why).
    pub(crate) unfiled: Option<Sig>,
    /// The fill-worsening measure.
    pub(crate) worse: Worse,
    /// The minute matches.
    pub(crate) minutes: Minutes,
    /// Wall time this share-day took, nanoseconds (timings only, never in
    /// the CSV).
    pub(crate) elapsed_ns: u64,
    /// Words about a failure, commas removed.
    pub(crate) detail: String,
}

impl Row {
    /// A row with nothing in it yet.
    pub(crate) fn blank(symbol: &str, day: Day, file: &str, label: Label) -> Self {
        Self {
            symbol: symbol.to_owned(),
            day,
            label,
            file: file.to_owned(),
            status: String::new(),
            compare: String::new(),
            session: session_word(day),
            counts: Counts::default(),
            t: Sig::default(),
            a: Sig::default(),
            store: Sig::default(),
            folded: None,
            unfiled: None,
            worse: Worse::default(),
            minutes: Minutes::default(),
            elapsed_ns: 0,
            detail: String::new(),
        }
    }

    /// This row failed for `reason`, with `detail`.
    pub(crate) fn failed(mut self, reason: &str, detail: &str) -> Self {
        self.status = format!("read_failed:{reason}");
        "n/a".clone_into(&mut self.compare);
        self.detail = clean(detail);
        self
    }
}

/// `text` with every comma, CR and LF replaced, so it is one CSV field.
pub(crate) fn clean(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            ',' => ';',
            '\r' | '\n' => ' ',
            other => other,
        })
        .collect()
}

/// The first and one-past-last UTC microsecond of IST `day`.
pub(crate) fn day_span(day: Day) -> (i64, i64) {
    let start = (i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS) * 1_000_000;
    (start, start + 86_400 * 1_000_000)
}

/// What the session filter says of a bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Kept.
    Kept,
    /// Dropped, and why.
    Dropped(DropReason),
    /// The session authority refused to judge it.
    Refused,
}

/// The session filter `pull::ingest` applies to one-second bars of a share:
/// the import's window (one day), rung and venue.
pub(crate) struct Filter {
    window: Window,
    cadence: pull::session::Cadence,
    venue: pull::vendor::Venue,
    day_start_secs: i64,
}

impl Filter {
    /// The filter for `day`.
    ///
    /// # Errors
    ///
    /// Never for a real day: a one-day window cannot run backwards.
    pub(crate) fn of(day: Day) -> Result<Self, String> {
        let window = Window::new(day, day).map_err(|w| w.to_string())?;
        Ok(Self {
            window,
            cadence: Granularity::Second1.cadence(),
            venue: ImportKind::Stocks.listing().venue(),
            day_start_secs: i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS,
        })
    }

    /// The verdict on a bar stamped `ts_micros`.
    pub(crate) fn verdict(&self, ts_micros: i64) -> Verdict {
        match self
            .window
            .verdict(ts_micros.div_euclid(1_000_000), self.cadence, self.venue)
        {
            Ok(None) => Verdict::Kept,
            Ok(Some(why)) => Verdict::Dropped(why),
            Err(_) => Verdict::Refused,
        }
    }

    /// Whether a row stamped at second `sod` of the day is in session.
    pub(crate) fn in_session_sod(&self, sod: u32) -> bool {
        let secs = self.day_start_secs + i64::from(sod);
        matches!(
            self.window.verdict(secs, self.cadence, self.venue),
            Ok(None)
        )
    }
}

/// `bars` the session filter keeps, counting the rest into `counts` when it
/// is given.
pub(crate) fn keep(filter: &Filter, bars: &[Bar], mut counts: Option<&mut Counts>) -> Vec<Bar> {
    let mut kept = Vec::with_capacity(bars.len());
    for bar in bars {
        match filter.verdict(bar.ts_micros) {
            Verdict::Kept => kept.push(*bar),
            dropped => {
                if let Some(c) = counts.as_deref_mut() {
                    let slot = match dropped {
                        Verdict::Dropped(DropReason::BeforeSessionOpen) => {
                            &mut c.seconds_before_open
                        }
                        Verdict::Dropped(DropReason::AtOrAfterSessionClose) => {
                            &mut c.seconds_after_close
                        }
                        _ => &mut c.seconds_dropped_other,
                    };
                    *slot = slot.saturating_add(1);
                }
            }
        }
    }
    kept
}

/// Rule A from rule T's kept seconds and every row's kept seconds (both in
/// time order; every traded second is also an every-row second): T's open,
/// close and volume, the high and low widened by every row.
pub(crate) fn rule_a(t: &[Bar], every: &[Bar], counts: &mut Counts) -> Vec<Bar> {
    let mut out = Vec::with_capacity(t.len());
    let mut rows = every.iter().peekable();
    for bar in t {
        while rows.next_if(|r| r.ts_micros < bar.ts_micros).is_some() {}
        let mut a = *bar;
        if let Some(r) = rows.peek().filter(|r| r.ts_micros == bar.ts_micros) {
            if r.high > a.high {
                a.high = r.high;
                counts.a_high_widened = counts.a_high_widened.saturating_add(1);
            }
            if r.low < a.low {
                a.low = r.low;
                counts.a_low_widened = counts.a_low_widened.saturating_add(1);
            }
        }
        out.push(a);
    }
    out
}

/// The fill-worsening measure: one merge of the stored seconds with rule A's
/// (both in time order). A second is compared when both hold it; it is worse
/// when rule A's high is above the stored high or its low below the stored
/// low.
pub(crate) fn fill_worse(stored: &[Bar], a: &[Bar]) -> Worse {
    let mut w = Worse::default();
    let mut rule = a.iter().peekable();
    for s in stored {
        while rule.next_if(|r| r.ts_micros < s.ts_micros).is_some() {}
        if let Some(r) = rule.peek().filter(|r| r.ts_micros == s.ts_micros) {
            w.compared = w.compared.saturating_add(1);
            let high = r.high > s.high;
            let low = r.low < s.low;
            if high {
                w.high = w.high.saturating_add(1);
            }
            if low {
                w.low = w.low.saturating_add(1);
            }
            if high || low {
                w.either = w.either.saturating_add(1);
            }
        }
    }
    w
}

/// One minute's extremes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Extremes {
    /// Minute number since the epoch.
    pub(crate) minute: i64,
    /// Highest high.
    pub(crate) high: i64,
    /// Lowest low.
    pub(crate) low: i64,
}

/// The per-minute extremes of `bars` (in time order).
pub(crate) fn minutes_of(bars: &[Bar]) -> Vec<Extremes> {
    let mut out: Vec<Extremes> = Vec::with_capacity(bars.len().min(1_440));
    for bar in bars {
        let minute = bar.ts_micros.div_euclid(60_000_000);
        match out.last_mut() {
            Some(m) if m.minute == minute => {
                m.high = m.high.max(bar.high);
                m.low = m.low.min(bar.low);
            }
            _ => out.push(Extremes {
                minute,
                high: bar.high,
                low: bar.low,
            }),
        }
    }
    out
}

/// How many of `zerodha`'s minutes `rule` also holds, and how many of those
/// match high and low exactly: one merge of two time-ordered lists.
pub(crate) fn match_minutes(zerodha: &[Bar], rule: &[Extremes]) -> Match {
    let mut m = Match::default();
    let mut ours = rule.iter().peekable();
    for z in zerodha {
        let minute = z.ts_micros.div_euclid(60_000_000);
        while ours.next_if(|e| e.minute < minute).is_some() {}
        if let Some(e) = ours.peek().filter(|e| e.minute == minute) {
            m.compared = m.compared.saturating_add(1);
            if e.high == z.high {
                m.high = m.high.saturating_add(1);
            }
            if e.low == z.low {
                m.low = m.low.saturating_add(1);
            }
        }
    }
    m
}

/// The store path of `key`'s month at `timeframe` of `vendor`, and the
/// symbol id the store files it under.
///
/// # Errors
///
/// A month or path the store cannot name.
fn month_path(
    (vendor, timeframe): (Vendor, Timeframe),
    key: &InstrumentKey,
    ym: YearMonth,
) -> Result<(StorePath<'_>, u32), String> {
    let path = StorePath::for_key(vendor, key, timeframe, ym, FileKind::Bars)
        .map_err(|w| format!("store_path: {w}"))?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the id IS the low 32 bits of the FNV-1a hash, as `pull::ingest` writes it \
                  and `cli::stored::load` asks for it"
    )]
    let symbol_id = brutex_core::universe::fnv1a(key.underlying.as_str()) as u32;
    Ok((path, symbol_id))
}

/// `store_<variant>: <words>` for a store refusal.
fn store_named(why: &StoreError) -> String {
    format!("store_{}: {why}", word_of(&format!("{why:?}")))
}

/// Reads `key`'s bars of `vendor` at `timeframe` for IST `day` through the
/// store's read door, handing each to `each` in order. `Ok(false)` when the
/// month does not exist.
///
/// # Errors
///
/// Any other refusal of the store, as `store_<variant>: <words>`.
pub(crate) fn store_day(
    root: &Path,
    (vendor, timeframe): (Vendor, Timeframe),
    key: &InstrumentKey,
    day: Day,
    mut each: impl FnMut(&Bar),
) -> Result<bool, String> {
    let ym = YearMonth::new(day.year(), day.month()).map_err(|w| w.to_string())?;
    let (path, symbol_id) = month_path((vendor, timeframe), key, ym)?;
    let file = match BarFile::open_existing(root, path, symbol_id) {
        Ok(file) => file,
        Err(StoreError::Missing { .. }) => return Ok(false),
        Err(why) => return Err(store_named(&why)),
    };
    let (start, end) = day_span(day);
    let mut index = file.first_at_or_after(start).map_err(|w| store_named(&w))?;
    while index < file.records() {
        let bar = file.read_record(index).map_err(|w| store_named(&w))?;
        if bar.ts_micros >= end {
            break;
        }
        each(&bar);
        index = index.saturating_add(1);
    }
    Ok(true)
}

/// A source that keeps the bytes of the last file it fetched, so the census
/// reads exactly the bytes `read_listed` checked and decoded.
pub(crate) struct Keeping<'a, S: CmSource> {
    inner: &'a S,
    kept: RefCell<Vec<u8>>,
}

impl<'a, S: CmSource> Keeping<'a, S> {
    /// `inner`, keeping.
    pub(crate) const fn new(inner: &'a S) -> Self {
        Self {
            inner,
            kept: RefCell::new(Vec::new()),
        }
    }

    /// The last fetched bytes, taken (empty when nothing was fetched since
    /// the last take).
    pub(crate) fn take(&self) -> Vec<u8> {
        self.kept.take()
    }
}

impl<S: CmSource> CmSource for Keeping<'_, S> {
    type Locator = S::Locator;

    fn day(&self, kind: CmKind, day: Day) -> Result<Option<DayListing<Self::Locator>>, CmRefusal> {
        self.inner.day(kind, day)
    }

    fn scope(&self) -> Arc<CmScope> {
        self.inner.scope()
    }

    fn fetch(
        &self,
        listing: &DayListing<Self::Locator>,
        file: &ListedFile<Self::Locator>,
    ) -> Result<Vec<u8>, CmRefusal> {
        let bytes = self.inner.fetch(listing, file)?;
        self.kept.replace(bytes.clone());
        Ok(bytes)
    }
}

/// The traded quantity of closing-auction prints, each identical row
/// counted: what bar definitions 2 and 3 folded into the seconds and
/// definition 4 files in the `.auc` sidecar (D-4878 item 10).
pub(crate) fn auction_volume(prints: &[pull::gdfl_cm::auction::Print]) -> u64 {
    prints.iter().fold(0_u64, |sum, p| {
        sum.saturating_add(p.quantity.saturating_mul(u64::from(p.rows)))
    })
}

/// A decoded row as the fold's input: a share carries no open interest.
pub(crate) const fn tick_of(r: &pull::gdfl_cm::CmRow) -> Tick {
    Tick {
        sod: r.sod,
        ltp: r.ltp,
        ltq: r.ltq,
        oi: None,
    }
}

/// The fold's input as bar definitions 2 and 3 saw the file (D-4878 item 10):
/// the decoder's kept rows and its diverted closing-auction prints, each
/// print's identical rows restored, merged back into file order by line.
/// One merge of two lists already in file order; the fold that consumes it
/// is the importer's `convert`, so the only difference from rule T is the
/// rows definition 4 sends to the `.auc` sidecar.
pub(crate) fn with_prints(
    rows: &[pull::gdfl_cm::CmRow],
    prints: &[pull::gdfl_cm::auction::Print],
) -> Vec<Tick> {
    let extra = prints.iter().fold(0_usize, |n, p| {
        n.saturating_add(usize::try_from(p.rows).unwrap_or(usize::MAX))
    });
    let mut out = Vec::with_capacity(rows.len().saturating_add(extra));
    let mut rest = rows.iter().peekable();
    for print in prints {
        while let Some(r) = rest.next_if(|r| r.line < print.line) {
            out.push(tick_of(r));
        }
        for _ in 0..print.rows {
            out.push(Tick {
                sod: print.sod,
                ltp: print.price,
                ltq: print.quantity,
                oi: None,
            });
        }
    }
    out.extend(rest.map(tick_of));
    out
}

/// The day's diverted `prints` the store's own `.auc` sidecar does NOT hold
/// (each print kept unless a record of the sidecar has its second, price,
/// quantity and row count), and how many of them it does hold. A bar
/// definition that filed some prints in the sidecar folded the rest into
/// the seconds; the rest are what a rebuild of its bars must put back. One
/// sidecar read per share-day with a print, and one set probe per print.
///
/// # Errors
///
/// The sidecar's refusal (`pull::gdfl_cm::auction::read`), or a path the
/// store cannot name, as words: the caller says so on the row.
pub(crate) fn unfiled_prints(
    store: &Path,
    key: &InstrumentKey,
    day: Day,
    prints: &[pull::gdfl_cm::auction::Print],
) -> Result<(Vec<pull::gdfl_cm::auction::Print>, u64), String> {
    use pull::gdfl_cm::auction;
    let path = auction::path(store, key, day)?;
    let Some(sidecar) = auction::read(&path).map_err(|w| w.to_string())? else {
        return Ok((prints.to_vec(), 0));
    };
    let filed: HashSet<(i64, i64, u64, u32)> = sidecar
        .records
        .iter()
        .map(|r| (r.t_micros, r.price_paisa, r.quantity, r.rows))
        .collect();
    let mut held = 0_u64;
    let mut rest = Vec::with_capacity(prints.len());
    for print in prints {
        let r = auction::Record::of(day, print);
        if filed.contains(&(r.t_micros, r.price_paisa, r.quantity, r.rows)) {
            held = held.saturating_add(1);
        } else {
            rest.push(*print);
        }
    }
    Ok((rest, held))
}

/// The row-level counts of a decoded file: one pass over its kept rows.
pub(crate) fn row_counts(rows: &[pull::gdfl_cm::CmRow], filter: &Filter, c: &mut Counts) {
    let mut high: Option<u32> = None;
    let mut last_trade: Option<i64> = None;
    let mut cached: Option<(u32, bool)> = None;
    for row in rows {
        if high.is_some_and(|max| row.sod < max) {
            c.out_of_order = c.out_of_order.saturating_add(1);
        } else {
            high = Some(row.sod);
        }
        if row.ltq == 0 {
            c.ltq0_rows = c.ltq0_rows.saturating_add(1);
        }
        let in_session = match cached {
            Some((sod, verdict)) if sod == row.sod => verdict,
            _ => {
                let verdict = filter.in_session_sod(row.sod);
                cached = Some((row.sod, verdict));
                verdict
            }
        };
        if !in_session {
            continue;
        }
        if row.ltq > 0 {
            c.trades_in_session = c.trades_in_session.saturating_add(1);
            last_trade = Some(row.ltp);
        } else if last_trade.is_some_and(|p| p != row.ltp) {
            c.ltq0_new_price = c.ltq0_new_price.saturating_add(1);
        }
    }
}

/// What one share-day's rebuild needs beside its file: the store root and
/// the day's filter.
pub(crate) struct DayContext<'a> {
    /// The store root.
    pub(crate) store: &'a Path,
    /// The day.
    pub(crate) day: Day,
    /// The day's session filter.
    pub(crate) filter: Filter,
}

/// One share-day: the raw file rebuilt under T, A and R, the store read, the
/// fill-worsening measure taken, the minutes matched.
pub(crate) fn check_file<S: CmSource>(
    source: &Keeping<'_, S>,
    listing: &DayListing<S::Locator>,
    (key, file): (&InstrumentKey, &str),
    ctx: &DayContext<'_>,
    label: Label,
) -> Row {
    let started = Instant::now();
    let symbol = key.underlying.as_str();
    let mut row = Row::blank(symbol, ctx.day, file, label);
    let mut stored: Vec<Bar> = Vec::with_capacity(1_024);
    let held = store_day(
        ctx.store,
        (Vendor::Gdfl, Timeframe::SECOND_1),
        key,
        ctx.day,
        |b| stored.push(*b),
    );
    row.store = sig_of(&stored);
    let mut row = match (rebuild(source, listing, key, ctx, &mut row), held) {
        (Err((reason, detail)), _) => row.failed(&reason, &detail),
        (Ok(_), Err(why)) => row.failed("store", &why),
        (Ok(a), Ok(_)) => {
            row.worse = fill_worse(&stored, &a);
            row.compare = compare(&row.t, &row.a, &row.store);
            if row.compare.starts_with("differs:") && row.folded == Some(row.store) {
                FOLDED_COMPARE.clone_into(&mut row.compare);
            } else if row.compare.starts_with("differs:") && row.unfiled == Some(row.store) {
                UNFILED_COMPARE.clone_into(&mut row.compare);
            }
            row.status = status_word(&row.compare, &row.session);
            row
        }
    };
    row.elapsed_ns = elapsed(started);
    row
}

/// The words a whole-file `TooManyBadRows` refusal earns: the reused
/// reader's verdict and what PREC-5's two-way rule (a file is refused whole
/// only when its malformed rows are at least its well-formed rows, with at
/// least one malformed) would say of the same counts.
pub(crate) fn two_way_words(refused: u32, rows: u32) -> String {
    let well = rows.saturating_sub(refused);
    let verdict = if refused >= 1 && refused >= well {
        "refuse"
    } else {
        "admit"
    };
    format!(
        "{refused} of {rows} rows refused; the reused reader refuses the file whole (more than \
         {} and more than one in every {}); PREC-5's two-way rule would {verdict} it ({refused} \
         malformed against {well} well-formed)",
        pull::gdfl_cm::MAX_BAD_ROWS,
        pull::gdfl_cm::BAD_ROWS_PER,
    )
}

/// The raw half of [`check_file`]: decoded, census taken, folded, filtered,
/// matched against Zerodha. `Ok(rule A's seconds)`, or `Err((reason,
/// detail))` when the file is refused; the census counts are filled either
/// way when the bytes were fetched.
fn rebuild<S: CmSource>(
    source: &Keeping<'_, S>,
    listing: &DayListing<S::Locator>,
    key: &InstrumentKey,
    ctx: &DayContext<'_>,
    row: &mut Row,
) -> Result<Vec<Bar>, (String, String)> {
    let _stale = source.take();
    let read = read_listed(source, listing, key);
    let bytes = source.take();
    let cen = census(&bytes);
    let c = &mut row.counts;
    c.raw_rows = cen.lines;
    c.negative_ltp = cen.negative_ltp;
    c.negative_qty = cen.negative_qty;
    c.crossed_quote = cen.crossed;
    c.one_sided_quote = cen.one_sided;
    c.identical_consecutive = cen.identical;
    let decoded = match read {
        Ok(Some(day_file)) => day_file.file,
        Ok(None) => return Err(("not_located".to_owned(), "the listing names no file".into())),
        Err(CmRefusal::TooManyBadRows { refused, rows }) => {
            c.refused_whole = u64::from(refused);
            return Err(("TooManyBadRows".to_owned(), two_way_words(refused, rows)));
        }
        Err(why) => return Err((word_of(&format!("{why:?}")), why.to_string())),
    };
    c.kept_rows = u64::try_from(decoded.rows.len()).unwrap_or(u64::MAX);
    c.zero_ltp = u64::from(decoded.refused.zero_ltp);
    c.off_grid = u64::from(decoded.refused.off_grid);
    c.malformed = u64::from(decoded.refused.malformed);
    c.auction_prints = u64::try_from(decoded.auction.len()).unwrap_or(u64::MAX);
    c.auction_volume = auction_volume(&decoded.auction);
    let ticks: Vec<Tick> = decoded.rows.iter().map(tick_of).collect();
    let fold = |kind: ImportKind, ticks: &[Tick]| {
        convert(kind, ctx.day, ticks).map_err(|why| {
            (
                format!("fold_{}", word_of(&format!("{why:?}"))),
                why.to_string(),
            )
        })
    };
    let traded = fold(ImportKind::Stocks, &ticks)?;
    let every = fold(ImportKind::Indices, &ticks)?;
    c.late_traded = u64::try_from(traded.placement.late_rows).unwrap_or(u64::MAX);
    let t = keep(&ctx.filter, &traded.seconds, Some(c));
    let r = keep(&ctx.filter, &every.seconds, None);
    let a = rule_a(&t, &r, c);
    if !decoded.auction.is_empty() {
        let restored = with_prints(&decoded.rows, &decoded.auction);
        let folded = fold(ImportKind::Stocks, &restored)?;
        row.folded = Some(sig_of(&keep(&ctx.filter, &folded.seconds, None)));
        match unfiled_prints(ctx.store, key, ctx.day, &decoded.auction) {
            Ok((rest, held)) => {
                row.counts.auction_in_sidecar = held;
                if held > 0 && !rest.is_empty() {
                    let restored = with_prints(&decoded.rows, &rest);
                    let unfiled = fold(ImportKind::Stocks, &restored)?;
                    row.unfiled = Some(sig_of(&keep(&ctx.filter, &unfiled.seconds, None)));
                }
            }
            Err(why) => {
                row.detail = format!(
                    "the store's .auc sidecar could not be read ({why}); only the every-print \
                     fold was compared"
                );
            }
        }
    }
    row_counts(&decoded.rows, &ctx.filter, &mut row.counts);
    row.t = sig_of(&t);
    row.a = sig_of(&a);
    let mut zerodha = Vec::with_capacity(400);
    let _held = store_day(
        ctx.store,
        (Vendor::Zerodha, Timeframe::MINUTE_1),
        key,
        ctx.day,
        |b| zerodha.push(*b),
    )
    .map_err(|why| ("zerodha".to_owned(), why))?;
    row.minutes = Minutes {
        zerodha: u64::try_from(zerodha.len()).unwrap_or(u64::MAX),
        t: match_minutes(&zerodha, &minutes_of(&t)),
        a: match_minutes(&zerodha, &minutes_of(&a)),
        r: match_minutes(&zerodha, &minutes_of(&r)),
    };
    Ok(a)
}

// ---------------------------------------------------------------------------
// The plan: which share-days the store holds, and what their landings say.
// ---------------------------------------------------------------------------

/// One `.prv` landing, as the check needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Landed {
    /// The landing's key ([`Label::landing`]).
    pub(crate) key: String,
    /// The source's kind.
    pub(crate) kind: SourceKind,
    /// The name as the record keeps it (its first bytes when
    /// [`Self::truncated`]).
    pub(crate) name: String,
    /// Whether [`Self::name`] is only a prefix of the source's name: the
    /// sidecar keeps at most 128 bytes of it (D-3334) and its BLAKE3 whole.
    pub(crate) truncated: bool,
    /// BLAKE3 of the whole name.
    pub(crate) name_hash: [u8; 32],
    /// The source's size in bytes.
    pub(crate) bytes: u64,
    /// The member's CRC-32, 0 for a tick-store day.
    pub(crate) crc32: u32,
}

impl Landed {
    /// The archive a member's record names: the part of its name before the
    /// day zip `day` is filed under, `<archive>!<day zip>!<entry>`. A name
    /// the record keeps whole must hold that day zip in full. A name kept
    /// as a prefix may end anywhere after the archive's `!`: the archive is
    /// the part before the first `!` whose remainder is the start of
    /// `<day zip>!`. Either way it is only a candidate: the whole name built
    /// from it must hash to the record's BLAKE3, or the row is
    /// `read_failed:source_changed`. `None` when the name does not reach
    /// past the archive's path.
    pub(crate) fn archive(&self, day: Day) -> Option<&str> {
        let zip = pull::gdfl_archive::day_zip_name(CmKind::Stocks, day);
        if let Some(at) = self.name.find(&format!("!{zip}!")) {
            return self.name.get(..at);
        }
        if !self.truncated {
            return None;
        }
        let tail = format!("{zip}!");
        self.name.match_indices('!').find_map(|(at, _)| {
            let rest = self.name.get(at + 1..)?;
            tail.starts_with(rest).then(|| self.name.get(..at))?
        })
    }
}

/// The archive part of a member's whole name (`<archive>!STOCKS/...`), as a
/// row's `source_name` carries it; `None` for any other name.
pub(crate) fn archive_of_name(name: &str) -> Option<&str> {
    name.find("!STOCKS/").and_then(|at| name.get(..at))
}

/// What a stored share-day's landing was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DaySource {
    /// One source, recorded.
    Recorded(Landed),
    /// Records naming different sources for one day.
    Ambiguous(String),
    /// No record names the day.
    Unrecorded,
    /// The month's `.prv` could not be read.
    Unreadable(String),
}

/// One stored share-day the plan found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    /// The store symbol.
    pub(crate) symbol: String,
    /// The day.
    pub(crate) day: Day,
    /// Its landing.
    pub(crate) source: DaySource,
}

/// The landings of a month's `.prv`, by day: `yyyymmdd` to every record of
/// that day, in ordinal order.
///
/// # Errors
///
/// What `store::provenance::read` refuses, named.
pub(crate) fn landings(
    store: &Path,
    path: StorePath<'_>,
) -> Result<BTreeMap<u32, Vec<store::provenance::Record>>, String> {
    let mut by_day: BTreeMap<u32, Vec<store::provenance::Record>> = BTreeMap::new();
    match store::provenance::read(store, path).map_err(|w| w.to_string())? {
        Found::Absent | Found::Unborn { .. } => {}
        Found::Present(sidecar) => {
            for record in sidecar.records() {
                by_day
                    .entry(record.day.yyyymmdd())
                    .or_default()
                    .push(record.clone());
            }
        }
    }
    Ok(by_day)
}

/// What a day's records say its source was.
pub(crate) fn day_source(records: Option<&Vec<store::provenance::Record>>) -> DaySource {
    let Some(records) = records.filter(|r| !r.is_empty()) else {
        return DaySource::Unrecorded;
    };
    let Some(last) = records.last() else {
        return DaySource::Unrecorded;
    };
    if records.iter().any(|r| {
        r.source_name_blake3 != last.source_name_blake3 || r.source_kind != last.source_kind
    }) {
        let names: Vec<String> = records
            .iter()
            .map(|r| format!("{}:{}", r.ordinal, r.source_name_blake3_hex()))
            .collect();
        return DaySource::Ambiguous(names.join(" "));
    }
    let ordinals: Vec<String> = records.iter().map(|r| r.ordinal.to_string()).collect();
    DaySource::Recorded(Landed {
        key: format!(
            "prv:{}@g{}:def{}",
            ordinals.join("+"),
            last.bar_file_generation,
            last.bar_definition
        ),
        kind: last.source_kind,
        name: last.source_name.clone(),
        truncated: last.source_name_truncated,
        name_hash: last.source_name_blake3,
        bytes: last.source_bytes,
        crc32: last.source_crc32,
    })
}

/// Every calendar day of a month.
pub(crate) fn days_of_month(ym: YearMonth) -> Vec<Day> {
    (1..=31_u8)
        .filter_map(|d| Day::new(ym.year(), ym.month(), d).ok())
        .collect()
}

/// The stored share-days of `symbol` in the months `months`, each with its
/// landing; a month that cannot be read is one failed row, on its first day.
/// Per month: one open, one `.prv` read and one indexed probe per calendar
/// day.
pub(crate) fn plan_symbol(
    store: &Path,
    symbol: &str,
    months: &[YearMonth],
) -> (Vec<Item>, Vec<Row>) {
    let mut items = Vec::new();
    let mut failed = Vec::new();
    let label = || Label {
        role: Role::Landing,
        source: Source::Unrecorded,
        source_name: String::new(),
        landing: String::new(),
    };
    let Ok(key) = InstrumentKey::cash(Exchange::Nse, symbol) else {
        if let Some(day) = months
            .first()
            .and_then(|m| days_of_month(*m).first().copied())
        {
            failed.push(
                Row::blank(symbol, day, "", label())
                    .failed("store_symbol", "not a share symbol the store files"),
            );
        }
        return (items, failed);
    };
    for &ym in months {
        let days = days_of_month(ym);
        let Some(&first) = days.first() else { continue };
        let fail = |why: &str| Row::blank(symbol, first, "", label()).failed("store_month", why);
        let (path, id) = match month_path((Vendor::Gdfl, Timeframe::SECOND_1), &key, ym) {
            Ok(p) => p,
            Err(why) => {
                failed.push(fail(&why));
                continue;
            }
        };
        let file = match BarFile::open_existing(store, path, id) {
            Ok(file) => file,
            Err(StoreError::Missing { .. }) => continue,
            Err(why) => {
                failed.push(fail(&store_named(&why)));
                continue;
            }
        };
        let records = landings(store, path);
        for day in days {
            let (start, end) = day_span(day);
            let held = file.first_at_or_after(start).and_then(|at| {
                if at < file.records() {
                    file.read_record(at).map(|b| b.ts_micros < end)
                } else {
                    Ok(false)
                }
            });
            match held {
                Ok(false) => {}
                Ok(true) => {
                    let source = match &records {
                        Ok(by_day) => {
                            let packed = u32::from(day.year()) * 10_000
                                + u32::from(day.month()) * 100
                                + u32::from(day.day());
                            day_source(by_day.get(&packed))
                        }
                        Err(why) => DaySource::Unreadable(why.clone()),
                    };
                    items.push(Item {
                        symbol: symbol.to_owned(),
                        day,
                        source,
                    });
                }
                Err(why) => failed
                    .push(Row::blank(symbol, day, "", label()).failed("store", &store_named(&why))),
            }
        }
    }
    (items, failed)
}

/// The store symbols of the GDFL cash tree, in name order, filtered.
///
/// # Errors
///
/// A tree that cannot be listed, named. A tree that does not exist holds
/// none.
pub(crate) fn store_symbols(
    store: &Path,
    symbols: Option<&HashSet<String>>,
) -> Result<Vec<String>, String> {
    let tree = store.join("bars").join("gdfl").join("NSE").join("CASH");
    let entries = match std::fs::read_dir(&tree) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", tree.display())),
    };
    let mut names = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", tree.display()))?;
        if let Ok(name) = entry.file_name().into_string()
            && symbols.is_none_or(|set| set.contains(&name))
        {
            names.insert(name);
        }
    }
    Ok(names.into_iter().collect())
}

/// The months of `symbol`'s GDFL one-second tree, from the store's own
/// listing of its `<yyyy-mm>.bin` and sealed `<yyyy-mm>.sbar` files
/// (D-5899), limited to `year` when given.
///
/// # Errors
///
/// A directory that cannot be listed, named.
pub(crate) fn store_months(
    store: &Path,
    symbol: &str,
    year: Option<u16>,
) -> Result<Vec<YearMonth>, String> {
    if let Some(year) = year {
        return Ok((1..=12_u8)
            .filter_map(|m| YearMonth::new(year, m).ok())
            .collect());
    }
    let dir = store
        .join("bars")
        .join("gdfl")
        .join("NSE")
        .join("CASH")
        .join(symbol)
        .join("1s");
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", dir.display())),
    };
    let mut months = BTreeSet::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", dir.display()))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        // Both forms of a month (D-5899): a sealed `.sbar` whose `.bin` a
        // retirement removed is still a month the store holds, and the read
        // door opens it; listing `.bin` alone would drop it without a word.
        let stem = name
            .strip_suffix(".bin")
            .or_else(|| name.strip_suffix(".sbar"));
        if let Some((y, m)) = stem.and_then(|s| s.split_once('-'))
            && let (Ok(y), Ok(m)) = (y.parse::<u16>(), m.parse::<u8>())
            && let Ok(ym) = YearMonth::new(y, m)
        {
            months.insert((y, m, ym));
        }
    }
    Ok(months.into_iter().map(|(_, _, ym)| ym).collect())
}

// ---------------------------------------------------------------------------
// The sources a run reads.
// ---------------------------------------------------------------------------

/// The archive `--archive` named: compared beside the landing source, never
/// as it.
pub(crate) struct Extra {
    /// Its path as given.
    pub(crate) path: String,
    /// The archive, opened under the run's scope.
    pub(crate) archive: Archive<std::fs::File>,
    /// The recorded archive paths that resolve to this same file: a landing
    /// from one of them is not compared twice.
    pub(crate) same_as: HashSet<String>,
}

/// Every source a run may read, opened once before any day.
pub(crate) struct Sources {
    /// Each archive a landing record names, by its recorded path.
    pub(crate) archives: HashMap<String, Result<Archive<std::fs::File>, String>>,
    /// The archive `--archive` named.
    pub(crate) extra: Option<Extra>,
    /// The tick store `BRUTEX_GDFL_TICKSTORE` names, with its root.
    pub(crate) tickstore: Result<(PathBuf, TickStore), String>,
}

impl Sources {
    /// Opens every archive the items name (each once), the extra archive and
    /// the tick store, all under `scope`. An archive a record names that
    /// cannot be opened is that record's rows' failure, not the run's.
    ///
    /// # Errors
    ///
    /// The extra archive refused: it was asked for by name.
    pub(crate) fn open(
        items: &[Item],
        extra: Option<(&Path, &HashSet<String>)>,
        tickstore: Result<PathBuf, String>,
        scope: &Arc<CmScope>,
    ) -> Result<Self, String> {
        let mut archives = HashMap::new();
        for item in items {
            if let DaySource::Recorded(landed) = &item.source
                && landed.kind == SourceKind::ArchiveMember
                && let Some(path) = landed.archive(item.day)
                && !archives.contains_key(path)
            {
                let opened = Archive::open(Path::new(path))
                    .map(|a| a.with_scope(Arc::clone(scope)))
                    .map_err(|w| format!("{path}: {w}"));
                archives.insert(path.to_owned(), opened);
            }
        }
        let extra = match extra {
            Some((path, same_as)) => {
                let archive = Archive::open(path)
                    .map_err(|w| format!("--archive {}: {w}", path.display()))?
                    .with_scope(Arc::clone(scope));
                Some(Extra {
                    path: path.display().to_string(),
                    archive,
                    same_as: same_as.clone(),
                })
            }
            None => None,
        };
        let tickstore = tickstore.map(|root| {
            (
                root.clone(),
                TickStore::new(&root).with_scope(Arc::clone(scope)),
            )
        });
        Ok(Self {
            archives,
            extra,
            tickstore,
        })
    }
}

/// The file `key` has in `listing`, by the stem the listing's scope files it
/// under that day: one slot index.
///
/// # Errors
///
/// [`CmRefusal::NotSwept`] when the scope does not file `key`, and what
/// [`DayListing::locate`] refuses.
pub(crate) fn locate<'l, L>(
    listing: &'l DayListing<L>,
    key: &InstrumentKey,
) -> Result<Option<&'l ListedFile<L>>, CmRefusal> {
    let (_, stem) = listing
        .scope()
        .stem_for(key, listing.day())
        .ok_or(CmRefusal::NotSwept)?;
    Ok(listing.locate(&stem)?.map(|(file, _)| file))
}

// ---------------------------------------------------------------------------
// One day.
// ---------------------------------------------------------------------------

/// What one day produced.
#[derive(Debug, Default)]
pub(crate) struct DayOut {
    /// One row per (share-day, role).
    pub(crate) rows: Vec<Row>,
    /// Listing entries the scope does not file (indices, series beyond the
    /// scope, off-surface shares), the filter does not ask for, or another
    /// entry of the same share already answered.
    pub(crate) skipped: u64,
    /// Wall time of the day, nanoseconds.
    pub(crate) elapsed_ns: u64,
}

impl DayOut {
    /// The raw rows the day read, plus one: the work a round's throughput
    /// is measured in (never zero, so an empty day still counts).
    pub(crate) fn work(&self) -> u64 {
        self.rows
            .iter()
            .fold(1_u64, |n, r| n.saturating_add(r.counts.raw_rows))
    }
}

/// One day's job: every stored share-day of it, and which of them an
/// earlier row already answers.
#[derive(Debug, Clone, Default)]
pub(crate) struct DayJob {
    /// Every stored share-day of the day, with whether its landing row and
    /// its extra row are already answered.
    pub(crate) items: Vec<(Item, bool, bool)>,
    /// Whether any extra row of the day is already answered (the extra
    /// archive's raw-only shares are then not looked for again).
    pub(crate) extra_seen: bool,
}

/// What a day's check reads besides its sources.
pub(crate) struct DayInputs<'a> {
    /// The store root.
    pub(crate) store: &'a Path,
    /// The symbol filter.
    pub(crate) symbols: Option<&'a HashSet<String>>,
}

/// A group of the day's items by the source their landing names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Group {
    /// Members of the archive at this recorded path.
    Archive(String),
    /// The tick store.
    Tickstore,
    /// No usable record: the reason, or `None` for none recorded.
    Unusable(Option<(String, String)>),
}

/// The day `day`: every uncovered stored share-day compared against its
/// landing source, the extra archive where asked, and every share a read
/// listing files that the store holds nothing of.
#[expect(
    clippy::too_many_lines,
    reason = "one day in order: its groups by landing source, each read once, then the extra archive"
)]
pub(crate) fn check_day(
    sources: &Sources,
    inputs: &DayInputs<'_>,
    day: Day,
    job: &DayJob,
) -> DayOut {
    let started = Instant::now();
    let mut out = DayOut::default();
    let ctx = match Filter::of(day) {
        Ok(filter) => DayContext {
            store: inputs.store,
            day,
            filter,
        },
        Err(why) => {
            for (item, _, _) in &job.items {
                out.rows.push(
                    Row::blank(&item.symbol, day, "", unrecorded_label()).failed("filter", &why),
                );
            }
            return out;
        }
    };
    let stored: HashSet<&str> = job
        .items
        .iter()
        .map(|(i, _, _)| i.symbol.as_str())
        .collect();
    let mut groups: BTreeMap<Group, Vec<&Item>> = BTreeMap::new();
    for (item, landing_done, _) in &job.items {
        if !*landing_done {
            groups.entry(group_of(item, day)).or_default().push(item);
        }
    }
    let mut seen: HashSet<(String, Role)> = HashSet::new();
    for (group, items) in &groups {
        match group {
            Group::Archive(path) => match sources.archives.get(path) {
                Some(Ok(archive)) => {
                    let zip = pull::gdfl_archive::day_zip_name(CmKind::Stocks, day);
                    let namer = |f: &ListedFile<_>, l: Option<&Landed>| {
                        let name = format!("{path}!{zip}!{}", f.entry);
                        let fits = l.is_none_or(|landed| {
                            brutex_core::blake3::hash(name.as_bytes()) == landed.name_hash
                                && f.len == landed.bytes
                                && f.crc32 == landed.crc32
                        });
                        (name, fits)
                    };
                    landing_group(
                        (archive, Source::ArchiveMember),
                        (&ctx, inputs),
                        (items, &stored),
                        (&mut out, &mut seen),
                        &namer,
                    );
                }
                Some(Err(why)) => {
                    for item in items {
                        out.rows.push(
                            Row::blank(
                                &item.symbol,
                                day,
                                "",
                                landing_label(item, Source::ArchiveMember),
                            )
                            .failed("source_missing", why),
                        );
                    }
                }
                None => {
                    for item in items {
                        out.rows.push(
                            Row::blank(
                                &item.symbol,
                                day,
                                "",
                                landing_label(item, Source::ArchiveMember),
                            )
                            .failed(
                                "source_not_opened",
                                "the run did not open this recorded archive",
                            ),
                        );
                    }
                }
            },
            Group::Tickstore => match &sources.tickstore {
                Ok((root, store)) => tickstore_group(
                    (root, store),
                    (&ctx, inputs),
                    (items, &stored),
                    (&mut out, &mut seen),
                ),
                Err(why) => {
                    for item in items {
                        out.rows.push(
                            Row::blank(
                                &item.symbol,
                                day,
                                "",
                                landing_label(item, Source::TickstoreDay),
                            )
                            .failed("tickstore_unset", why),
                        );
                    }
                }
            },
            Group::Unusable(reason) => {
                if reason.is_none() && sources.extra.is_some() {
                    continue;
                }
                let (word, detail) = reason.clone().unwrap_or_else(|| {
                    (
                        "source_unrecorded".to_owned(),
                        "no .prv record names this day's source, and no --archive was given to \
                         compare with"
                            .to_owned(),
                    )
                });
                for item in items {
                    out.rows.push(
                        Row::blank(
                            &item.symbol,
                            day,
                            "",
                            landing_label(item, Source::Unrecorded),
                        )
                        .failed(&word, &detail),
                    );
                }
            }
        }
    }
    if let Some(extra) = &sources.extra {
        // A landing group read from this same archive already looked for
        // the shares it files that the store holds nothing of.
        let scanned = groups
            .keys()
            .any(|g| matches!(g, Group::Archive(p) if extra.same_as.contains(p)));
        extra_pass(extra, &ctx, (job, scanned), (&stored, inputs), &mut out);
    }
    out.elapsed_ns = elapsed(started);
    out
}

/// A tick-store landing's items of the day. The day file is the landing's
/// source as a whole: its name and length are held to each record before it
/// is opened, so a file that changed is `read_failed:source_changed`, never
/// read; the rest go to [`landing_group`].
fn tickstore_group(
    (root, store): (&Path, &TickStore),
    (ctx, inputs): (&DayContext<'_>, &DayInputs<'_>),
    (items, stored): (&[&Item], &HashSet<&str>),
    (out, seen): (&mut DayOut, &mut HashSet<(String, Role)>),
) {
    let day = ctx.day;
    let rel = TickStore::new(Path::new(""))
        .day_path(CmKind::Stocks, day)
        .display()
        .to_string();
    let len = std::fs::metadata(root.join(&rel)).map(|m| m.len()).ok();
    let name_hash = brutex_core::blake3::hash(rel.as_bytes());
    let mut fitting: Vec<&Item> = Vec::with_capacity(items.len());
    for item in items {
        match &item.source {
            DaySource::Recorded(landed)
                if landed.name_hash != name_hash || len != Some(landed.bytes) =>
            {
                let mut label = landing_label(item, Source::TickstoreDay);
                label.source_name.clone_from(&rel);
                out.rows
                    .push(Row::blank(&item.symbol, day, "", label).failed(
                        "source_changed",
                        &format!(
                            "the tick-store day file is {} bytes; the landing recorded {} bytes of \
                         the file whose name hashes to {}",
                            len.map_or_else(|| "absent, 0".to_owned(), |n| n.to_string()),
                            landed.bytes,
                            hex8(&landed.name_hash)
                        ),
                    ));
            }
            _ => fitting.push(item),
        }
    }
    let namer = |_: &ListedFile<_>, _: Option<&Landed>| (rel.clone(), true);
    landing_group(
        (store, Source::TickstoreDay),
        (ctx, inputs),
        (&fitting, stored),
        (out, seen),
        &namer,
    );
}

/// The source group a share-day's landing puts it in.
fn group_of(item: &Item, day: Day) -> Group {
    match &item.source {
        DaySource::Recorded(landed) => match landed.kind {
            SourceKind::ArchiveMember => match landed.archive(day) {
                Some(path) => Group::Archive(path.to_owned()),
                None => Group::Unusable(Some((
                    "source_name_unparsed".to_owned(),
                    format!(
                        "the record's name {} does not hold this day's day zip",
                        landed.name
                    ),
                ))),
            },
            SourceKind::TickstoreDay => Group::Tickstore,
            SourceKind::VendorEndpoint => Group::Unusable(Some((
                "source_kind_unexpected".to_owned(),
                "a GDFL share month's landing names a vendor endpoint".to_owned(),
            ))),
        },
        DaySource::Ambiguous(names) => Group::Unusable(Some((
            "source_ambiguous".to_owned(),
            format!("the day's records name different sources: {names}"),
        ))),
        DaySource::Unreadable(why) => Group::Unusable(Some(("provenance".to_owned(), why.clone()))),
        DaySource::Unrecorded => Group::Unusable(None),
    }
}

/// A landing row's label before any source was read: its source kind and
/// its landing.
fn landing_label(item: &Item, source: Source) -> Label {
    Label {
        role: Role::Landing,
        source,
        source_name: String::new(),
        landing: match &item.source {
            DaySource::Recorded(landed) => landed.key.clone(),
            _ => String::new(),
        },
    }
}

/// The label of a row with no source.
fn unrecorded_label() -> Label {
    Label {
        role: Role::Landing,
        source: Source::Unrecorded,
        source_name: String::new(),
        landing: String::new(),
    }
}

/// How a landing source names a listed file, and whether it is the recorded
/// one: given the file and the landing (`None` for a share the store holds
/// nothing of), the source's whole name and whether the name, size and CRC
/// fit the record.
type Namer<'a, L> = dyn Fn(&ListedFile<L>, Option<&Landed>) -> (String, bool) + 'a;

/// One landing source's items of the day: each located, held to its record
/// (`read_failed:source_changed` otherwise) and checked; then every share
/// the listing files that the store holds nothing of.
fn landing_group<S: CmSource>(
    (source, kind): (&S, Source),
    (ctx, inputs): (&DayContext<'_>, &DayInputs<'_>),
    (items, stored): (&[&Item], &HashSet<&str>),
    (out, seen): (&mut DayOut, &mut HashSet<(String, Role)>),
    namer: &Namer<'_, S::Locator>,
) {
    let day = ctx.day;
    let label = |name: String, landing: String| Label {
        role: Role::Landing,
        source: kind,
        source_name: name,
        landing,
    };
    let listing = match source.day(CmKind::Stocks, day) {
        Ok(Some(listing)) => listing,
        Ok(None) => {
            for item in items {
                out.rows.push(
                    Row::blank(&item.symbol, day, "", landing_label(item, kind)).failed(
                        "source_changed",
                        "the recorded source holds no file for this day",
                    ),
                );
            }
            return;
        }
        Err(why) => {
            for item in items {
                out.rows.push(
                    Row::blank(&item.symbol, day, "", landing_label(item, kind))
                        .failed("source_listing", &why.to_string()),
                );
            }
            return;
        }
    };
    let keeping = Keeping::new(source);
    for item in items {
        let DaySource::Recorded(landed) = &item.source else {
            continue;
        };
        let Ok(key) = InstrumentKey::cash(Exchange::Nse, &item.symbol) else {
            out.rows.push(
                Row::blank(&item.symbol, day, "", landing_label(item, kind))
                    .failed("store_symbol", "not a share symbol the store files"),
            );
            continue;
        };
        seen.insert((item.symbol.clone(), Role::Landing));
        match locate(&listing, &key) {
            Ok(Some(file)) => {
                let (name, fits) = namer(file, Some(landed));
                let label = label(name, landed.key.clone());
                if fits {
                    out.rows.push(check_file(
                        &keeping,
                        &listing,
                        (&key, file.name()),
                        ctx,
                        label,
                    ));
                } else {
                    out.rows
                        .push(Row::blank(&item.symbol, day, file.name(), label).failed(
                        "source_changed",
                        &format!(
                            "the listed file ({} bytes; CRC-32 {:08x}) is not the one the landing \
                             recorded ({} bytes; CRC-32 {:08x}; whole-name BLAKE3 {})",
                            file.len,
                            file.crc32,
                            landed.bytes,
                            landed.crc32,
                            hex8(&landed.name_hash)
                        ),
                    ));
                }
            }
            Ok(None) => out.rows.push(
                Row::blank(&item.symbol, day, "", landing_label(item, kind)).failed(
                    "source_changed",
                    "the recorded source no longer lists this share's file",
                ),
            ),
            Err(why) => out.rows.push(
                Row::blank(&item.symbol, day, "", landing_label(item, kind))
                    .failed(&word_of(&format!("{why:?}")), &why.to_string()),
            ),
        }
    }
    raw_only_scan(
        &keeping,
        &listing,
        (stored, inputs),
        (ctx, Role::Landing),
        (out, seen),
        &|f| label(namer(f, None).0, String::new()),
    );
}

/// Every share `listing` files that the store holds nothing of on the day
/// (and that the filter asks for), rebuilt and compared against nothing:
/// `raw_only` when it traded, `exact_T` when it did not.
fn raw_only_scan<S: CmSource>(
    keeping: &Keeping<'_, S>,
    listing: &DayListing<S::Locator>,
    (stored, inputs): (&HashSet<&str>, &DayInputs<'_>),
    (ctx, role): (&DayContext<'_>, Role),
    (out, seen): (&mut DayOut, &mut HashSet<(String, Role)>),
    label_of: &dyn Fn(&ListedFile<S::Locator>) -> Label,
) {
    let day = ctx.day;
    let wanted = |s: &str| inputs.symbols.is_none_or(|set| set.contains(s));
    let folder = format!("{}/", listing.folder());
    for file in listing.entries() {
        let stem = file
            .entry
            .strip_prefix(folder.as_str())
            .and_then(|n| pull::gdfl_cm::split_name(n).ok())
            .map(|(stem, _)| stem);
        let key = match stem.map(|s| listing.filing(s)) {
            Some(Ok(Some(key))) => key,
            Some(Err(why)) => match why.filed_as() {
                Some(symbol)
                    if wanted(symbol)
                        && !stored.contains(symbol)
                        && seen.insert((symbol.to_owned(), role)) =>
                {
                    out.rows.push(
                        Row::blank(symbol, day, file.name(), label_of(file))
                            .failed("filing", &why.to_string()),
                    );
                    continue;
                }
                _ => {
                    out.skipped = out.skipped.saturating_add(1);
                    continue;
                }
            },
            _ => {
                out.skipped = out.skipped.saturating_add(1);
                continue;
            }
        };
        let symbol = key.underlying.as_str().to_owned();
        if !wanted(&symbol) || stored.contains(symbol.as_str()) || !seen.insert((symbol, role)) {
            out.skipped = out.skipped.saturating_add(1);
            continue;
        }
        out.rows.push(check_file(
            keeping,
            listing,
            (&key, file.name()),
            ctx,
            label_of(file),
        ));
    }
}

/// The extra archive's rows of the day: every stored share-day whose landing
/// is not a member of that archive and has no extra row yet, compared with
/// it and labelled `archive_not_the_landing_source`; `store_only` where it
/// lists no file of the share; then, unless the day already has extra rows,
/// every share it files that the store holds nothing of.
#[expect(
    clippy::too_many_lines,
    reason = "one pass over one listing: its targets, its refusals, its raw-only shares"
)]
fn extra_pass(
    extra: &Extra,
    ctx: &DayContext<'_>,
    (job, scanned): (&DayJob, bool),
    (stored, inputs): (&HashSet<&str>, &DayInputs<'_>),
    out: &mut DayOut,
) {
    let day = ctx.day;
    let zip = pull::gdfl_archive::day_zip_name(CmKind::Stocks, day);
    let label = |entry: &str, landing: &str| Label {
        role: Role::Extra,
        source: Source::NotTheLandingSource,
        source_name: if entry.is_empty() {
            String::new()
        } else {
            format!("{}!{zip}!{entry}", extra.path)
        },
        landing: landing.to_owned(),
    };
    let targets: Vec<&Item> = job
        .items
        .iter()
        .filter(|(item, _, extra_done)| {
            !*extra_done
                && match &item.source {
                    DaySource::Recorded(landed) => {
                        landed.kind != SourceKind::ArchiveMember
                            || landed
                                .archive(day)
                                .is_none_or(|p| !extra.same_as.contains(p))
                    }
                    _ => true,
                }
        })
        .map(|(item, _, _)| item)
        .collect();
    let scan = !job.extra_seen && !scanned;
    if targets.is_empty() && !scan {
        return;
    }
    let landing_of = |item: &Item| match &item.source {
        DaySource::Recorded(landed) => landed.key.clone(),
        _ => String::new(),
    };
    let listing = match extra.archive.day(CmKind::Stocks, day) {
        Ok(Some(listing)) => listing,
        Ok(None) => {
            for item in &targets {
                out.rows.push(store_only_row(
                    ctx,
                    item,
                    label("", &landing_of(item)),
                    &format!("the archive holds no day zip for {day}"),
                ));
            }
            return;
        }
        Err(why) => {
            for item in &targets {
                out.rows.push(
                    Row::blank(&item.symbol, day, "", label("", &landing_of(item)))
                        .failed("source_listing", &why.to_string()),
                );
            }
            return;
        }
    };
    let keeping = Keeping::new(&extra.archive);
    let mut seen: HashSet<(String, Role)> = HashSet::new();
    for item in &targets {
        seen.insert((item.symbol.clone(), Role::Extra));
        let Ok(key) = InstrumentKey::cash(Exchange::Nse, &item.symbol) else {
            out.rows.push(
                Row::blank(&item.symbol, day, "", label("", &landing_of(item)))
                    .failed("store_symbol", "not a share symbol the store files"),
            );
            continue;
        };
        match locate(&listing, &key) {
            Ok(Some(file)) => out.rows.push(check_file(
                &keeping,
                &listing,
                (&key, file.name()),
                ctx,
                label(&file.entry, &landing_of(item)),
            )),
            Ok(None) => out.rows.push(store_only_row(
                ctx,
                item,
                label("", &landing_of(item)),
                "the archive's day zip lists no file of this share",
            )),
            Err(why) => out.rows.push(
                Row::blank(&item.symbol, day, "", label("", &landing_of(item)))
                    .failed(&word_of(&format!("{why:?}")), &why.to_string()),
            ),
        }
    }
    if scan {
        raw_only_scan(
            &keeping,
            &listing,
            (stored, inputs),
            (ctx, Role::Extra),
            (out, &mut seen),
            &|f| label(&f.entry, ""),
        );
    }
}

/// A stored share-day the compared source does not file: `store_only`, with
/// the store's signature.
fn store_only_row(ctx: &DayContext<'_>, item: &Item, label: Label, why: &str) -> Row {
    let started = Instant::now();
    let mut row = Row::blank(&item.symbol, ctx.day, "", label);
    let Ok(key) = InstrumentKey::cash(Exchange::Nse, &item.symbol) else {
        return row.failed("store_symbol", "not a share symbol the store files");
    };
    let mut sig = SigBuilder::new();
    match store_day(
        ctx.store,
        (Vendor::Gdfl, Timeframe::SECOND_1),
        &key,
        ctx.day,
        |b| sig.push(b),
    ) {
        Ok(_) => {
            row.store = sig.finish();
            "store_only".clone_into(&mut row.compare);
            "store_only".clone_into(&mut row.status);
            row.detail = clean(why);
        }
        Err(why) => row = row.failed("store", &why),
    }
    row.elapsed_ns = elapsed(started);
    row
}

// ---------------------------------------------------------------------------
// Workers.
// ---------------------------------------------------------------------------

/// Runs `job` over `0..n` on `workers` threads, each taking the next index;
/// the answers in index order. A worker that panicked is named.
///
/// # Errors
///
/// `"a worker stopped"` when a worker thread panicked.
pub(crate) fn parallel<T: Send>(
    n: usize,
    workers: usize,
    job: impl Fn(usize) -> T + Sync,
) -> Result<Vec<T>, String> {
    let next = AtomicUsize::new(0);
    let mut slots: Vec<Option<T>> = (0..n).map(|_| None).collect();
    let answers = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers.clamp(1, n.max(1)))
            .map(|_| {
                scope.spawn(|| {
                    let mut mine = Vec::new();
                    loop {
                        let at = next.fetch_add(1, Ordering::Relaxed);
                        if at >= n {
                            break;
                        }
                        mine.push((at, job(at)));
                    }
                    mine
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().map_err(|_| "a worker stopped".to_owned()))
            .collect::<Result<Vec<_>, String>>()
    })?;
    for (at, answer) in answers.into_iter().flatten() {
        if let Some(slot) = slots.get_mut(at) {
            *slot = Some(answer);
        }
    }
    Ok(slots.into_iter().flatten().collect())
}

/// One round of [`schedule`]: its workers, its jobs, their work and its wall
/// time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Round {
    /// Workers.
    pub(crate) workers: usize,
    /// Jobs.
    pub(crate) jobs: usize,
    /// Work done ([`DayOut::work`]).
    pub(crate) work: u64,
    /// Wall time, nanoseconds.
    pub(crate) wall_ns: u64,
}

/// Whether round `a` did more work per nanosecond than round `b`: exact, by
/// cross-multiplication.
pub(crate) fn faster(a: &Round, b: &Round) -> bool {
    u128::from(a.work) * u128::from(b.wall_ns.max(1))
        > u128::from(b.work) * u128::from(a.wall_ns.max(1))
}

/// Runs `job` over `0..n` and returns the answers in index order, with the
/// rounds it took. With `fixed`, one round at that many workers. Otherwise
/// the operator's rule (16 Oct 17:15 IST, DATA-1 item 5, days as the unit):
/// the ceiling is `ceiling` (the host's cores) and `n`, whichever is fewer;
/// start at one worker and double while the work per second of the last
/// full round (one job per worker) still rises; the rest runs at the best
/// count measured. The answers do not depend on the count.
///
/// # Errors
///
/// What [`parallel`] refuses.
pub(crate) fn schedule<T: Send>(
    n: usize,
    fixed: Option<usize>,
    ceiling: usize,
    work: impl Fn(&T) -> u64 + Sync,
    job: impl Fn(usize) -> T + Sync,
) -> Result<(Vec<T>, Vec<Round>), String> {
    let mut answers = Vec::with_capacity(n);
    let mut rounds = Vec::new();
    let run = |from: usize, count: usize, workers: usize, answers: &mut Vec<T>| {
        let started = Instant::now();
        let got = parallel(count, workers, |i| job(from + i))?;
        let round = Round {
            workers,
            jobs: count,
            work: got.iter().fold(0_u64, |s, t| s.saturating_add(work(t))),
            wall_ns: elapsed(started),
        };
        answers.extend(got);
        Ok::<Round, String>(round)
    };
    if let Some(workers) = fixed {
        if n > 0 {
            rounds.push(run(0, n, workers, &mut answers)?);
        }
        return Ok((answers, rounds));
    }
    let top = ceiling.min(n).max(1);
    let mut workers = 1_usize;
    let mut best: Option<Round> = None;
    let mut at = 0_usize;
    while at < n {
        let count = workers.min(n - at);
        let round = run(at, count, workers, &mut answers)?;
        rounds.push(round);
        at += count;
        if count < workers {
            break;
        }
        let rising = best.as_ref().is_none_or(|b| faster(&round, b));
        if rising {
            best = Some(round);
            if workers < top {
                // The ceiling is a count the doubling reaches, never one it
                // skips: 8 doubles to the host's 14, not past it and not
                // stopping short of it.
                workers = workers.saturating_mul(2).min(top);
                continue;
            }
        } else if let Some(b) = &best {
            workers = b.workers;
        }
        if at < n {
            rounds.push(run(at, n - at, workers, &mut answers)?);
        }
        break;
    }
    Ok((answers, rounds))
}

/// Nanoseconds since `started`.
fn elapsed(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

// ---------------------------------------------------------------------------
// Rows as text.
// ---------------------------------------------------------------------------

/// The CSV header.
pub(crate) const HEADER: &str = "symbol,day,role,source,status,compare,session,file,source_name,\
landing,checked_by,raw_rows,kept_rows,zero_ltp,off_grid,malformed,refused_whole,negative_ltp,\
negative_qty,crossed_quote,one_sided_quote,identical_consecutive,out_of_order,late_traded,\
ltq0_rows,ltq0_new_price,auction_prints,auction_volume,auction_in_sidecar,trades_in_session,\
seconds_before_open,seconds_after_close,seconds_dropped_other,outside_day_band,t_seconds,\
t_volume,t_hash,a_seconds,a_high_widened,a_low_widened,a_hash,folded_seconds,folded_volume,\
folded_hash,unfiled_seconds,unfiled_volume,unfiled_hash,store_seconds,store_volume,store_hash,\
fill_compared,fill_high_worse,fill_low_worse,fill_either_worse,zerodha_basis,zerodha_minutes,\
t_compared,t_high_match,t_low_match,a_compared,a_high_match,a_low_match,r_compared,r_high_match,\
r_low_match,detail";

/// The first eight bytes of a digest, as hex.
pub(crate) fn hex8(hash: &[u8; 32]) -> String {
    let mut s = String::with_capacity(16);
    for b in hash.iter().take(8) {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// One row as a CSV line (no newline), checked by build `commit`.
pub(crate) fn csv_line(r: &Row, commit: &str) -> String {
    let c = &r.counts;
    let m = &r.minutes;
    let basis = if m.zerodha == 0 {
        "none"
    } else {
        "raw_unadjusted"
    };
    let mut line = format!(
        "{},{},{},{},{},{},{},{},{},{},{}",
        r.symbol,
        r.day,
        r.label.role.word(),
        r.label.source.word(),
        r.status,
        r.compare,
        r.session,
        clean(&r.file),
        clean(&r.label.source_name),
        clean(&r.label.landing),
        commit
    );
    for n in c.values() {
        let _ = write!(line, ",{n}");
    }
    let triple = |s: Option<Sig>| {
        s.map_or_else(
            || ",,".to_owned(),
            |f| format!("{},{},{}", f.seconds, f.volume, hex8(&f.hash)),
        )
    };
    let folded = triple(r.folded);
    let unfiled = triple(r.unfiled);
    let w = &r.worse;
    let _ = write!(
        line,
        ",UNVERIFIED,{},{},{},{},{},{},{},{folded},{unfiled},{},{},{},{},{},{},{},{basis},{},{},{},{},{},{},{},{},{},{},{}",
        r.t.seconds,
        r.t.volume,
        hex8(&r.t.hash),
        r.a.seconds,
        c.a_high_widened,
        c.a_low_widened,
        hex8(&r.a.hash),
        r.store.seconds,
        r.store.volume,
        hex8(&r.store.hash),
        w.compared,
        w.high,
        w.low,
        w.either,
        m.zerodha,
        m.t.compared,
        m.t.high,
        m.t.low,
        m.a.compared,
        m.a.high,
        m.a.low,
        m.r.compared,
        m.r.high,
        m.r.low,
        clean(&r.detail),
    );
    line
}

/// A CSV line's cells by header name (the header is [`HEADER`]).
pub(crate) struct Cells<'a> {
    cells: Vec<&'a str>,
}

/// The column of `name` in [`HEADER`].
fn column(name: &str) -> Option<usize> {
    HEADER.split(',').position(|h| h == name)
}

impl<'a> Cells<'a> {
    /// `line`'s cells.
    ///
    /// # Errors
    ///
    /// A line with another cell count than the header's.
    pub(crate) fn of(line: &'a str) -> Result<Self, String> {
        let cells: Vec<&str> = line.split(',').collect();
        let want = HEADER.split(',').count();
        if cells.len() == want {
            Ok(Self { cells })
        } else {
            Err(format!(
                "a row of {} cells, not {want}: {line}",
                cells.len()
            ))
        }
    }

    /// The cell of `name`, empty when the header has no such column.
    pub(crate) fn get(&self, name: &str) -> &'a str {
        column(name)
            .and_then(|at| self.cells.get(at).copied())
            .unwrap_or("")
    }

    /// The cell of `name` as a count, 0 when it is not one.
    pub(crate) fn n(&self, name: &str) -> u64 {
        self.get(name).parse().unwrap_or(0)
    }
}

/// A `YYYY-MM-DD` cell as a day.
pub(crate) fn parse_day(text: &str) -> Option<Day> {
    let mut parts = text.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    Day::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()
}

// ---------------------------------------------------------------------------
// The output directory: segments, versions, the fold.
// ---------------------------------------------------------------------------

/// A row's key in a year's ledger: day, symbol, role.
pub(crate) type RowKey = (String, String, String);

/// What a year's output files already hold.
#[derive(Debug, Default)]
pub(crate) struct YearLedger {
    /// The newest version's number.
    pub(crate) version: Option<u32>,
    /// The highest number any version file of the year carries (a version a
    /// dead run left without its index included).
    pub(crate) highest_version: u32,
    /// The highest segment sequence the newest version folded.
    pub(crate) through: u64,
    /// The highest segment sequence on disk.
    pub(crate) last_segment: u64,
    /// Segments not yet folded.
    pub(crate) unfolded: u64,
    /// Every row: the newest version's, then each unfolded segment's in
    /// sequence order, a later row replacing an earlier one of its key.
    pub(crate) lines: BTreeMap<RowKey, String>,
}

impl YearLedger {
    /// Whether the year already holds `line`, byte for byte, as the row of
    /// its key: one map probe.
    pub(crate) fn holds(&self, line: &str) -> bool {
        Cells::of(line).is_ok_and(|c| {
            let key = (
                c.get("day").to_owned(),
                c.get("symbol").to_owned(),
                c.get("role").to_owned(),
            );
            self.lines.get(&key).is_some_and(|held| held == line)
        })
    }
}

/// The segment sequences of `year` in `dir`, from one listing.
///
/// # Errors
///
/// A directory that cannot be listed.
pub(crate) fn segments_in(dir: &Path, year: u16) -> Result<Vec<u64>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(why) => return Err(format!("{}: {why}", dir.display())),
    };
    let prefix = format!("tick-to-second-{year}.s");
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|why| format!("{}: {why}", dir.display()))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if let Some(n) = name
            .strip_prefix(&prefix)
            .and_then(|r| r.strip_suffix(".csv"))
            .filter(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            .and_then(|n| n.parse::<u64>().ok())
        {
            out.push(n);
        }
    }
    out.sort_unstable();
    Ok(out)
}

/// The rows of a CSV text into `lines`, each replacing an earlier one of its
/// key.
///
/// # Errors
///
/// A text whose header is not [`HEADER`] (another layout's file), or a row
/// that is not one of this layout's, named by `path`.
fn absorb(text: &str, path: &Path, lines: &mut BTreeMap<RowKey, String>) -> Result<(), String> {
    let mut rows = text.lines();
    if rows.next() != Some(HEADER) {
        return Err(format!(
            "{} is not a file of this check's layout (its header differs)",
            path.display()
        ));
    }
    for line in rows {
        let cells = Cells::of(line).map_err(|w| format!("{}: {w}", path.display()))?;
        let key = (
            cells.get("day").to_owned(),
            cells.get("symbol").to_owned(),
            cells.get("role").to_owned(),
        );
        lines.insert(key, line.to_owned());
    }
    Ok(())
}

/// What `dir` holds of `year`: the newest version (its CSV held to the
/// digest its index records) and every segment after it.
///
/// # Errors
///
/// A file that cannot be read, a CSV that is not the one its index names,
/// or a file of another layout.
pub(crate) fn load_year(dir: &Path, year: u16) -> Result<YearLedger, String> {
    let mut ledger = YearLedger::default();
    for kind in ["csv", "idx", "summary.txt", "timings.txt"] {
        if let Some(&v) = index::versions_in(dir, year, kind)?.last() {
            ledger.highest_version = ledger.highest_version.max(v);
        }
    }
    if let Some(newest) = index::newest(dir, year)? {
        let path = dir.join(format!(
            "tick-to-second-{year}.v{:03}.csv",
            newest.version()
        ));
        let bytes = std::fs::read(&path).map_err(|w| format!("{}: {w}", path.display()))?;
        if brutex_core::blake3::hash(&bytes) != newest.csv_digest() {
            return Err(format!(
                "{} is not the CSV its index {} names (BLAKE3 differs)",
                path.display(),
                newest.path().display()
            ));
        }
        let text =
            String::from_utf8(bytes).map_err(|_| format!("{} is not text", path.display()))?;
        absorb(&text, &path, &mut ledger.lines)?;
        ledger.version = Some(newest.version());
        ledger.through = newest.segments_through();
    }
    let segments = segments_in(dir, year)?;
    ledger.last_segment = segments.last().copied().unwrap_or(0).max(ledger.through);
    for seq in segments.into_iter().filter(|s| *s > ledger.through) {
        let path = dir.join(segment_name(year, seq));
        let text =
            std::fs::read_to_string(&path).map_err(|w| format!("{}: {w}", path.display()))?;
        absorb(&text, &path, &mut ledger.lines)?;
        ledger.unfolded = ledger.unfolded.saturating_add(1);
    }
    Ok(ledger)
}

/// A segment's file name.
pub(crate) fn segment_name(year: u16, seq: u64) -> String {
    format!("tick-to-second-{year}.s{seq:06}.csv")
}

/// Writes `bytes` to a staging name in `dir` and makes them durable.
///
/// # Errors
///
/// The staging refusal or the host's, named.
fn stage(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    stage_with(dir, name, &mut |file| {
        file.write_all(bytes).and_then(|()| file.sync_all())
    })
}

/// [`stage`] with the filling passed in. A filling the host refuses (a full
/// disk, a failed `fsync`) takes the staging name back off: the name carries
/// this process's id, so no later run's [`store::staging::create`] would ever
/// clear it, and a refused run left a `.tmp` beside its versions for good
/// (verify-DATA-2-r1 follow-up). The removal is of the one name this call
/// created, never of a version; when it is refused too, both refusals are
/// named.
///
/// # Errors
///
/// The staging refusal, or the filling's, named with the removal's outcome.
fn stage_with(
    dir: &Path,
    name: &str,
    fill: &mut dyn FnMut(&mut std::fs::File) -> std::io::Result<()>,
) -> Result<PathBuf, String> {
    let path = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    let mut staged = store::staging::create(&path).map_err(|w| format!("{}: {w}", w.word()))?;
    match fill(&mut staged.file) {
        Ok(()) => Ok(path),
        Err(why) => {
            drop(staged);
            Err(match std::fs::remove_file(&path) {
                Ok(()) => format!("{}: {why} (the staging name was removed)", path.display()),
                Err(left) => format!(
                    "{}: {why}; the staging name could not be removed either: {left}",
                    path.display()
                ),
            })
        }
    }
}

/// Publishes `bytes` at `dir/name`, which must not exist: staged, made
/// durable, linked with `hard_link` (which never replaces a file), the
/// staging name removed and the directory made durable. A reader sees the
/// whole file or no file.
///
/// # Errors
///
/// A name that exists already, or the host's refusal, named.
pub(crate) fn publish_new(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|w| format!("{}: {w}", dir.display()))?;
    let tmp = stage(dir, name, bytes)?;
    let at = dir.join(name);
    let linked = std::fs::hard_link(&tmp, &at).map_err(|w| format!("{}: {w}", at.display()));
    std::fs::remove_file(&tmp).map_err(|w| format!("{}: {w}", tmp.display()))?;
    linked?;
    std::fs::File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(|w| format!("{}: {w}", dir.display()))?;
    Ok(at)
}

/// The index cells of a version's lines: per share-day the landing row, unless
/// it says only that no source was recorded and an extra row exists (a
/// landing row that names a source ranks 2, an extra row 1, a landing row
/// with no source 0; the higher rank answers).
///
/// # Errors
///
/// A row whose status, source or day this layout does not know.
pub(crate) fn cells_of(lines: &[&str]) -> Result<Vec<(String, Day, SecondStatus)>, String> {
    let mut chosen: BTreeMap<(String, String), (String, Day, SecondStatus, u8)> = BTreeMap::new();
    for (at, line) in lines.iter().enumerate() {
        let cells = Cells::of(line)?;
        let day = parse_day(cells.get("day")).ok_or_else(|| format!("not a day: {line}"))?;
        let status = Status::of_word(cells.get("status"))
            .ok_or_else(|| format!("not a status of this layout: {line}"))?;
        // A special session compared exact under rule A holds rule A's bars,
        // not rule T's: the index says `ExactA`, so `trusted()` never claims
        // the rule-T seconds of a day that does not hold them. The CSV keeps
        // `special_session` and its `compare`.
        let status = if status == Status::SpecialSession && cells.get("compare") == "exact_A" {
            Status::ExactA
        } else {
            status
        };
        let source = Source::of_word(cells.get("source"))
            .ok_or_else(|| format!("not a source of this layout: {line}"))?;
        let sat = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
        let answer = SecondStatus {
            status,
            source,
            store_seconds: sat(cells.n("store_seconds")),
            fill_compared: sat(cells.n("fill_compared")),
            fill_worse: sat(cells.n("fill_either_worse")),
            row: u32::try_from(at.saturating_add(1)).unwrap_or(u32::MAX),
        };
        let rank = match (cells.get("role") == Role::Landing.word(), source) {
            (true, Source::Unrecorded) => 0,
            (false, _) => 1,
            (true, _) => 2,
        };
        let key = (cells.get("symbol").to_owned(), cells.get("day").to_owned());
        if chosen.get(&key).is_none_or(|(_, _, _, old)| rank >= *old) {
            chosen.insert(key, (cells.get("symbol").to_owned(), day, answer, rank));
        }
    }
    Ok(chosen
        .into_values()
        .map(|(symbol, day, answer, _)| (symbol, day, answer))
        .collect())
}

/// Counts of values, for exact percentiles without a sort.
#[derive(Debug, Default)]
pub(crate) struct Histogram {
    counts: BTreeMap<u64, u64>,
    n: u64,
}

impl Histogram {
    /// One value.
    pub(crate) fn add(&mut self, v: u64) {
        let slot = self.counts.entry(v).or_insert(0);
        *slot = slot.saturating_add(1);
        self.n = self.n.saturating_add(1);
    }

    /// The value at per-mille `q` (500 the median, 990 the p99): the smallest
    /// value with at least `ceil(q * n / 1000)` values at or below it.
    pub(crate) fn at(&self, q: u64) -> u64 {
        let rank = (self.n.saturating_mul(q)).div_ceil(1_000).max(1);
        let mut seen = 0_u64;
        for (&v, &k) in &self.counts {
            seen = seen.saturating_add(k);
            if seen >= rank {
                return v;
            }
        }
        0
    }

    /// The largest value.
    pub(crate) fn max(&self) -> u64 {
        self.counts.keys().next_back().copied().unwrap_or(0)
    }

    /// How many values.
    pub(crate) const fn len(&self) -> u64 {
        self.n
    }

    /// `n=… p50=… p99=… max=…`.
    pub(crate) fn line(&self) -> String {
        format!(
            "n={} p50={} p99={} max={}",
            self.n,
            self.at(500),
            self.at(990),
            self.max()
        )
    }
}

/// `part / whole` in per-mille, `0` when `whole` is.
pub(crate) fn per_mille(part: u64, whole: u64) -> u64 {
    part.saturating_mul(1_000).checked_div(whole).unwrap_or(0)
}

/// A version's summary: what its rows say, from the rows alone, so the same
/// rows give the same bytes.
///
/// # Errors
///
/// A row of another layout.
#[expect(
    clippy::too_many_lines,
    reason = "one report in reading order: the tables, the sums, the fill measure, the minutes"
)]
pub(crate) fn summary(
    year: u16,
    version: u32,
    lines: &[&str],
    identity: &[(&str, String)],
) -> Result<String, String> {
    let mut s = String::new();
    let _ = writeln!(s, "tick-to-second-check {year} version {version}");
    for (k, v) in identity {
        let _ = writeln!(s, "{k:<22}{v}");
    }
    let _ = writeln!(s, "{:<22}{}", "rows", lines.len());
    let mut by: [BTreeMap<String, u64>; 4] = Default::default();
    let mut sums: BTreeMap<&str, u64> = BTreeMap::new();
    let mut share_t = Histogram::default();
    let mut share_a = Histogram::default();
    let mut worse = Histogram::default();
    let summed: Vec<&str> = COUNT_COLUMNS
        .iter()
        .copied()
        .chain([
            "a_high_widened",
            "a_low_widened",
            "fill_compared",
            "fill_high_worse",
            "fill_low_worse",
            "fill_either_worse",
            "zerodha_minutes",
            "t_compared",
            "t_high_match",
            "t_low_match",
            "a_compared",
            "a_high_match",
            "a_low_match",
            "r_compared",
            "r_high_match",
            "r_low_match",
        ])
        .collect();
    for line in lines {
        let cells = Cells::of(line)?;
        for (slot, name) in by.iter_mut().zip(["role", "source", "status", "compare"]) {
            *slot.entry(cells.get(name).to_owned()).or_insert(0) += 1;
        }
        for name in &summed {
            let v = sums.entry(name).or_insert(0);
            *v = v.saturating_add(cells.n(name));
        }
        if cells.n("t_compared") > 0 {
            share_t.add(per_mille(cells.n("t_high_match"), cells.n("t_compared")));
            share_a.add(per_mille(cells.n("a_high_match"), cells.n("a_compared")));
        }
        if cells.n("fill_compared") > 0 {
            worse.add(per_mille(
                cells.n("fill_either_worse"),
                cells.n("fill_compared"),
            ));
        }
    }
    for (title, table) in ["role", "source", "status", "compare"].iter().zip(&by) {
        let _ = writeln!(s, "\n{title}");
        for (k, v) in table {
            let _ = writeln!(s, "  {k:<44} {v}");
        }
    }
    let _ = writeln!(s, "\ncounts over every row");
    for name in &summed {
        let _ = writeln!(s, "  {name:<26}{}", sums.get(name).copied().unwrap_or(0));
    }
    let _ = writeln!(
        s,
        "  {:<26}not counted: UNVERIFIED (no dated, charter-sourced per-share-day band)",
        "outside_day_band"
    );
    let get = |name: &str| sums.get(name).copied().unwrap_or(0);
    let _ = writeln!(
        s,
        "\nfill worsening (rule A against the stored bar, per second)\n  compared {} high worse {} \
         low worse {} either {} ({} per mille)\n  per share-day either-worse per mille: {}",
        get("fill_compared"),
        get("fill_high_worse"),
        get("fill_low_worse"),
        get("fill_either_worse"),
        per_mille(get("fill_either_worse"), get("fill_compared")),
        worse.line()
    );
    let _ = writeln!(
        s,
        "\nminutes vs Zerodha 1min (RAW basis: Zerodha is corporate-action adjusted, GDFL raw; \
         {RAW_BASIS_WHY})\n  zerodha minutes       {}",
        get("zerodha_minutes")
    );
    for name in ["t", "a", "r"] {
        let (c, h, l) = (
            get(&format!("{name}_compared")),
            get(&format!("{name}_high_match")),
            get(&format!("{name}_low_match")),
        );
        let _ = writeln!(
            s,
            "  rule {}: compared {c} high equal {h} ({} per mille) low equal {l} ({} per mille)",
            name.to_uppercase(),
            per_mille(h, c),
            per_mille(l, c)
        );
    }
    let _ = writeln!(
        s,
        "  per share-day high-match per mille: T median {} A median {} (n={})",
        share_t.at(500),
        share_a.at(500),
        share_t.len()
    );
    Ok(s)
}

/// What one run measured, for its timings file.
#[derive(Debug, Default)]
pub(crate) struct Measured {
    /// The plan's wall time, nanoseconds.
    pub(crate) plan_ns: u64,
    /// The days' wall time, nanoseconds.
    pub(crate) days_ns: u64,
    /// Each checked day's wall time.
    pub(crate) day_ns: Vec<u64>,
    /// The rounds the workers took.
    pub(crate) rounds: Vec<Round>,
    /// The rows this run made: (`raw_rows`, `elapsed_ns`).
    pub(crate) rows: Vec<(u64, u64)>,
}

/// The timings text of one version: this run's measures and the index
/// lookups timed on the version just written.
pub(crate) fn timings(m: &Measured, lookups: &Histogram, fold_ns: u64) -> String {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "timings (wall, measured by the run that wrote this version)"
    );
    let mut ns_row = Histogram::default();
    let mut ns_row_big = Histogram::default();
    let mut us_day = Histogram::default();
    let mut ms_day = Histogram::default();
    for &(raw, ns) in &m.rows {
        if let Some(per_row) = ns.checked_div(raw) {
            ns_row.add(per_row);
            us_day.add(ns / 1_000);
            if raw >= AMORTISED_ROWS {
                ns_row_big.add(per_row);
            }
        }
    }
    for &ns in &m.day_ns {
        ms_day.add(ns / 1_000_000);
    }
    let _ = writeln!(
        s,
        "  ns per raw row, per share-day        {}",
        ns_row.line()
    );
    let _ = writeln!(
        s,
        "  ns per raw row, files of >= {AMORTISED_ROWS} rows {}",
        ns_row_big.line()
    );
    let _ = writeln!(
        s,
        "  us per share-day (a file read)       {}",
        us_day.line()
    );
    let _ = writeln!(
        s,
        "  ms per day                           {}",
        ms_day.line()
    );
    let _ = writeln!(
        s,
        "  ns per status_of lookup (this index) {}",
        lookups.line()
    );
    let _ = writeln!(
        s,
        "  phases ms                            plan {} days {} fold {}",
        m.plan_ns / 1_000_000,
        m.days_ns / 1_000_000,
        fold_ns / 1_000_000
    );
    let _ = writeln!(s, "  rounds (workers, days, work, ms)");
    for r in &m.rounds {
        let _ = writeln!(
            s,
            "    {} {} {} {}",
            r.workers,
            r.jobs,
            r.work,
            r.wall_ns / 1_000_000
        );
    }
    s
}

// ---------------------------------------------------------------------------
// The verb.
// ---------------------------------------------------------------------------

/// The two forms of a path that [`inside`] compares (D-5187).
///
/// Both start from the path made absolute against the current directory
/// (`std::path::absolute`), the same base the host resolves a relative
/// `--out` against when the run makes it.
///
/// - `lexical` drops each `.` and lets each `..` take off the component before
///   it, as written: it catches an `--out` that names the store through one of
///   the store's own links.
/// - `physical` walks the same components, and after each one that exists
///   takes the host's own answer for where it is (`canonicalize`: links
///   followed, the case held on disk). A `..` after an existing component
///   therefore leaves the place that component really is, and a `..` after a
///   missing one cancels it, exactly as `create_dir_all` makes the missing
///   component a real directory and then steps back out of it.
///
/// The form this replaces kept a relative name relative (the parent of a bare
/// name is the empty path, which neither exists nor resolves) and stopped at a
/// `..` after a missing component, so both compared raw text with the store's
/// resolved path and the run made its directory, lock and files inside the
/// store (verify-DATA-2-r1).
#[derive(Debug, PartialEq, Eq)]
struct Resolved {
    lexical: PathBuf,
    physical: PathBuf,
}

/// `path` in both forms.
///
/// # Errors
///
/// A path that cannot be made absolute (an empty one, or no current
/// directory), or a component the host can neither resolve nor report missing
/// (a refused permission, a component under a file): where the path lies is
/// then unknown, and the run is refused rather than guessed (CLAUDE.md §4).
fn resolved(path: &Path) -> Result<Resolved, String> {
    use std::path::Component;
    let absolute = std::path::absolute(path)
        .map_err(|why| format!("{} cannot be made absolute: {why}", path.display()))?;
    let mut lexical = PathBuf::new();
    let mut physical = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::Prefix(_) | Component::RootDir => {
                lexical.push(part);
                physical.push(part);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                lexical.pop();
                physical.pop();
            }
            Component::Normal(name) => {
                lexical.push(name);
                physical.push(name);
                match std::fs::canonicalize(&physical) {
                    Ok(real) => physical = real,
                    Err(why) if why.kind() == std::io::ErrorKind::NotFound => {}
                    Err(why) => {
                        return Err(format!(
                            "{}: where {} lies cannot be told: {why}",
                            physical.display(),
                            path.display()
                        ));
                    }
                }
            }
        }
    }
    Ok(Resolved { lexical, physical })
}

/// Whether `inner` lies inside `outer`: in either form of [`Resolved`], so a
/// relative name, a `..`, a link or a case variant cannot carry an `--out`
/// into a root the check never writes.
///
/// # Errors
///
/// Either path's [`resolved`] refusal.
fn inside(inner: &Path, outer: &Path) -> Result<bool, String> {
    let (inner, outer) = (resolved(inner)?, resolved(outer)?);
    Ok(inner.physical.starts_with(&outer.physical) || inner.lexical.starts_with(&outer.lexical))
}

/// What a run did, for its report.
#[derive(Debug, Default)]
pub(crate) struct Report {
    /// Share-days the plan found in the store.
    pub(crate) planned: usize,
    /// Of those, already answered by an earlier row.
    pub(crate) covered: usize,
    /// Of those, share-days no `.prv` record names a landing for (a month
    /// landed before D-3334): they can be compared only with `--archive`,
    /// labelled not the landing source, or are `read_failed:
    /// source_unrecorded`. Said out loud so a run over such months never
    /// reads as a landing-source check (D-5185).
    pub(crate) unrecorded: usize,
    /// Days checked.
    pub(crate) days: usize,
    /// Rows this run made.
    pub(crate) rows: usize,
    /// Of those, rows that are not exact.
    pub(crate) not_exact: usize,
    /// Each year's new version files (the CSV path, the index path).
    pub(crate) versions: Vec<(u16, PathBuf, PathBuf)>,
    /// The `--symbols` names the store's GDFL share tree holds no share of,
    /// sorted: said out loud, never a silent empty run (a typo checks
    /// nothing). Not refused: with `--archive` such a name may still be one
    /// of the archive's files, compared as `raw_only`.
    pub(crate) unknown_symbols: Vec<String>,
    /// Years with nothing new to fold.
    pub(crate) unchanged: Vec<u16>,
}

/// The verb, with its environment and store root passed in.
pub(crate) fn command_with(
    args: &[&str],
    env: &dyn Fn(&str) -> Option<OsString>,
    store: &dyn Fn() -> Result<PathBuf, String>,
    out: &mut String,
) -> u8 {
    let ask = match ask(args) {
        Ok(ask) => ask,
        Err(why) => {
            let _ = writeln!(out, "refused: {why}\nusage: {USAGE}");
            return MISUSED;
        }
    };
    match run(&ask, env, store) {
        Ok(report) => {
            let _ = writeln!(
                out,
                "tick-to-second-check: {} stored share-days, {} already answered, {} days \
                 checked, {} rows made",
                report.planned, report.covered, report.days, report.rows
            );
            if report.unrecorded > 0 {
                let _ = writeln!(
                    out,
                    "  {} of the stored share-days have no .prv landing record (landed before \
                     D-3334): compared only with --archive, as not the landing source, or \
                     read_failed:source_unrecorded",
                    report.unrecorded
                );
            }
            for (year, csv, idx) in &report.versions {
                let _ = writeln!(out, "  {year} rows   {}", csv.display());
                let _ = writeln!(out, "  {year} index  {}", idx.display());
            }
            for name in &report.unknown_symbols {
                let _ = writeln!(
                    out,
                    "  --symbols {name}: the store's GDFL share tree holds no such share"
                );
            }
            for year in &report.unchanged {
                let _ = writeln!(out, "  {year} nothing new to fold");
            }
            if report.not_exact == 0 {
                OK
            } else {
                let _ = writeln!(
                    out,
                    "  NOT EVERY SHARE-DAY IS EXACT: {} of this run's rows are not; see the \
                     summary's status table",
                    report.not_exact
                );
                FAILED
            }
        }
        Err(why) => {
            let _ = writeln!(out, "TICK-TO-SECOND CHECK REFUSED: {why}");
            crate::note(
                &telemetry::Event::error(TARGET, "tick-to-second check refused")
                    .with("why", why.as_str()),
            );
            FAILED
        }
    }
}

/// Whether a status is exact.
fn exact(status: &str) -> bool {
    matches!(status, "exact_T" | "exact_A" | "special_session")
}

/// Opens and takes the run's lock at `path` (D-5188), before anything is
/// read. The name is opened by the importer's own lock door,
/// [`pull::gdfl_run::locks::open_lock_path`]: `O_NOFOLLOW | O_NONBLOCK`, so a
/// symbolic link at the name is refused by the host and nothing is made
/// where it leads, and a pipe cannot hold the open; then the opened file must
/// be a regular file with exactly one name, so a hard link to another file (a
/// store month's writer lock) is refused before any lock is taken on it.
///
/// # Errors
///
/// The refusal, naming `path` and what is at it; or `path` held by another
/// run.
fn take_lock(path: &Path) -> Result<std::fs::File, String> {
    let lock = pull::gdfl_run::locks::open_lock_path(path, LOCK_NAME).map_err(|fault| {
        format!(
            "{} is refused ({fault}): the lock name is refused before anything is read, and \
             nothing is made or locked through it (D-5188)",
            path.display()
        )
    })?;
    lock.try_lock().map_err(|w| {
        format!(
            "{} is held: another tick-to-second-check writes this directory ({w})",
            path.display()
        )
    })?;
    Ok(lock)
}

/// The run behind [`command_with`].
#[expect(
    clippy::too_many_lines,
    reason = "one run in order: roots, lock, plan, sources, days, fold; each step is a call"
)]
fn run(
    ask: &Ask,
    env: &dyn Fn(&str) -> Option<OsString>,
    store: &dyn Fn() -> Result<PathBuf, String>,
) -> Result<Report, String> {
    let store = store()?;
    if inside(&ask.out, &store)? {
        return Err(format!(
            "--out {} lies inside the store {}; this check never writes the store",
            ask.out.display(),
            store.display()
        ));
    }
    let tick_root =
        crate::gdfl_import_command::root(crate::gdfl_import_command::TICKSTORE_VAR, env);
    if let Ok(ticks) = &tick_root
        && inside(&ask.out, ticks)?
    {
        return Err(format!(
            "--out {} lies inside the tick store {}; this check never writes a source it \
             compares with",
            ask.out.display(),
            ticks.display()
        ));
    }
    std::fs::create_dir_all(&ask.out).map_err(|w| format!("{}: {w}", ask.out.display()))?;
    let lock = take_lock(&ask.out.join(LOCK_NAME))?;
    let commit = crate::commit_stamp().unwrap_or("unstamped").to_owned();
    let started = Instant::now();
    crate::note(
        &telemetry::Event::info(TARGET, "tick-to-second check started")
            .with("year", ask.year.map_or(0, u64::from))
            .with("out", ask.out.display().to_string().as_str()),
    );
    // The plan: the store's own days, each with its landing.
    let names = store_symbols(&store, ask.symbols.as_ref())?;
    let unknown_symbols: Vec<String> = ask.symbols.as_ref().map_or_else(Vec::new, |asked| {
        let mut missing: Vec<String> = asked
            .iter()
            .filter(|name| names.binary_search(*name).is_err())
            .cloned()
            .collect();
        missing.sort_unstable();
        missing
    });
    let cores = host_cores();
    let listing_workers = ask.workers.unwrap_or(cores).min(names.len().max(1));
    let planned = parallel(names.len(), listing_workers, |at| {
        names.get(at).map_or_else(
            || Ok((Vec::new(), Vec::new())),
            |name| store_months(&store, name, ask.year).map(|m| plan_symbol(&store, name, &m)),
        )
    })?;
    let mut items: Vec<Item> = Vec::new();
    let mut plan_failed: Vec<Row> = Vec::new();
    for answer in planned {
        let (i, f) = answer?;
        items.extend(i);
        plan_failed.extend(f);
    }
    let mut measured = Measured {
        plan_ns: elapsed(started),
        ..Measured::default()
    };
    // What earlier runs answered.
    let mut years: BTreeSet<u16> = items.iter().map(|i| i.day.year()).collect();
    years.extend(plan_failed.iter().map(|r| r.day.year()));
    if let Some(year) = ask.year {
        years.insert(year);
    }
    let mut ledgers: BTreeMap<u16, YearLedger> = BTreeMap::new();
    for &year in &years {
        ledgers.insert(year, load_year(&ask.out, year)?);
    }
    let row_of = |l: &YearLedger, item: &Item, role: Role| {
        l.lines
            .get(&(
                item.day.to_string(),
                item.symbol.clone(),
                role.word().to_owned(),
            ))
            .and_then(|line| {
                Cells::of(line).ok().map(|c| {
                    (
                        c.get("landing").to_owned(),
                        c.get("status").starts_with("read_failed:"),
                    )
                })
            })
    };
    let extra = ask.archive.is_some();
    // The recorded archives that are the --archive file itself: a landing
    // from one of them is never compared with it twice. One resolution per
    // distinct recorded path.
    let mut same_paths: HashSet<String> = HashSet::new();
    if let Some(path) = &ask.archive {
        let real = std::fs::canonicalize(path)
            .map_err(|w| format!("--archive {}: {w}", path.display()))?;
        let recorded: BTreeSet<&str> = items
            .iter()
            .filter_map(|i| match &i.source {
                DaySource::Recorded(l) if l.kind == SourceKind::ArchiveMember => l.archive(i.day),
                _ => None,
            })
            .collect();
        for p in recorded {
            if std::fs::canonicalize(p).is_ok_and(|r| r == real) {
                same_paths.insert(p.to_owned());
            }
        }
    }
    let is_same = |item: &Item| match &item.source {
        DaySource::Recorded(l) if l.kind == SourceKind::ArchiveMember => {
            l.archive(item.day).is_some_and(|p| same_paths.contains(p))
        }
        _ => false,
    };
    // The days whose --archive files were already looked at: a row against
    // it, or a landing row read from it. One pass over each year's rows.
    let extra_days: HashSet<String> = ledgers
        .values()
        .flat_map(|l| {
            l.lines.iter().filter_map(|((day, _, role), line)| {
                let seen = role == Role::Extra.word()
                    || Cells::of(line).is_ok_and(|c| {
                        archive_of_name(c.get("source_name"))
                            .is_some_and(|p| same_paths.contains(p))
                    });
                seen.then(|| day.clone())
            })
        })
        .collect();
    let mut jobs: BTreeMap<u32, DayJob> = BTreeMap::new();
    let mut report = Report {
        planned: items.len(),
        unrecorded: items
            .iter()
            .filter(|i| i.source == DaySource::Unrecorded)
            .count(),
        unknown_symbols,
        ..Report::default()
    };
    for item in items {
        let Some(ledger) = ledgers.get(&item.day.year()) else {
            continue;
        };
        let landing_now = match &item.source {
            DaySource::Recorded(landed) => landed.key.clone(),
            _ => String::new(),
        };
        // An unrecorded day compared with --archive is answered by its extra
        // row; a recorded one by a landing row of the same landing (a new
        // landing of the day is checked again).
        // A `read_failed` row answers nothing: the day is read again on the
        // next run (a source that was missing may be back), and a row that
        // comes out the same is not filed again.
        let landing_done = (extra && item.source == DaySource::Unrecorded)
            || row_of(ledger, &item, Role::Landing)
                .is_some_and(|(l, failed)| l == landing_now && !failed);
        let extra_done = !extra
            || is_same(&item)
            || row_of(ledger, &item, Role::Extra).is_some_and(|(_, failed)| !failed);
        if landing_done && extra_done {
            report.covered += 1;
        }
        let job = jobs.entry(item.day.days_from_epoch()).or_default();
        job.items.push((item, landing_done, extra_done));
    }
    if extra {
        // A day the store holds nothing of may still hold the extra
        // archive's files: every day of the years in play.
        for &year in &years {
            if let Ok((first, days)) = index::year_span(year) {
                for d in first..first + days {
                    jobs.entry(d).or_default();
                }
            }
        }
        for (d, job) in &mut jobs {
            job.extra_seen =
                Day::from_days(*d).is_ok_and(|day| extra_days.contains(&day.to_string()));
        }
    }
    jobs.retain(|_, job| job.items.iter().any(|(_, l, e)| !l || !e) || (extra && !job.extra_seen));
    plan_failed.retain(|row| {
        let line = csv_line(row, &commit);
        let key = (
            row.day.to_string(),
            row.symbol.clone(),
            row.label.role.word().to_owned(),
        );
        ledgers
            .get(&row.day.year())
            .is_none_or(|l| l.lines.get(&key) != Some(&line))
    });
    // The sources, each opened once.
    let scope = Arc::new(pull::gdfl_run::scope::of(ImportKind::Stocks, &store)?.0);
    let all_items: Vec<Item> = jobs
        .values()
        .flat_map(|j| j.items.iter().map(|(i, _, _)| i.clone()))
        .collect();
    let sources = Sources::open(
        &all_items,
        ask.archive.as_deref().map(|p| (p, &same_paths)),
        tick_root,
        &scope,
    )?;
    let inputs = DayInputs {
        store: &store,
        symbols: ask.symbols.as_ref(),
    };
    let next: BTreeMap<u16, AtomicU64> = ledgers
        .iter()
        .map(|(y, l)| (*y, AtomicU64::new(l.last_segment.saturating_add(1))))
        .collect();
    let day_list: Vec<(u32, DayJob)> = jobs.into_iter().collect();
    report.days = day_list.len();
    let days_started = Instant::now();
    let (outs, rounds) = schedule(
        day_list.len(),
        ask.workers,
        cores,
        |d: &Result<DayOut, String>| d.as_ref().map_or(1, DayOut::work),
        |at| {
            let Some((d, job)) = day_list.get(at) else {
                return Ok(DayOut::default());
            };
            let day = Day::from_days(*d).map_err(|w| w.to_string())?;
            let done = check_day(&sources, &inputs, day, job);
            note_day(day, &done);
            // A row byte-identical to the one the year already holds for its
            // key (a retried failure that failed the same way) is not filed
            // again: a run that learns nothing makes no version.
            let held = ledgers.get(&day.year());
            let mut lines: Vec<String> = done
                .rows
                .iter()
                .map(|r| csv_line(r, &commit))
                .filter(|line| held.is_none_or(|l| !l.holds(line)))
                .collect();
            if !lines.is_empty() {
                let seq = next
                    .get(&day.year())
                    .map_or(1, |n| n.fetch_add(1, Ordering::Relaxed));
                let mut text = String::with_capacity(lines.len().saturating_mul(320));
                text.push_str(HEADER);
                text.push('\n');
                lines.sort_unstable();
                for line in lines {
                    text.push_str(&line);
                    text.push('\n');
                }
                publish_new(&ask.out, &segment_name(day.year(), seq), text.as_bytes())?;
            }
            Ok(done)
        },
    )?;
    measured.days_ns = elapsed(days_started);
    measured.rounds = rounds;
    let mut this_run: Vec<Row> = Vec::new();
    for out in outs {
        let out = out?;
        measured.day_ns.push(out.elapsed_ns);
        this_run.extend(out.rows);
    }
    if !plan_failed.is_empty() {
        let mut by_year: BTreeMap<u16, Vec<&Row>> = BTreeMap::new();
        for row in &plan_failed {
            by_year.entry(row.day.year()).or_default().push(row);
        }
        for (year, rows) in by_year {
            let seq = next
                .get(&year)
                .map_or(1, |n| n.fetch_add(1, Ordering::Relaxed));
            let mut text = String::from(HEADER);
            text.push('\n');
            let mut lines: Vec<String> = rows.iter().map(|r| csv_line(r, &commit)).collect();
            lines.sort_unstable();
            for line in lines {
                text.push_str(&line);
                text.push('\n');
            }
            publish_new(&ask.out, &segment_name(year, seq), text.as_bytes())?;
        }
        this_run.extend(plan_failed);
    }
    measured.rows = this_run
        .iter()
        .map(|r| (r.counts.raw_rows, r.elapsed_ns))
        .collect();
    report.rows = this_run.len();
    report.not_exact = this_run.iter().filter(|r| !exact(&r.status)).count();
    note_months(&this_run);
    // The fold: each year with unfolded segments becomes its next version.
    for &year in &years {
        let fold_started = Instant::now();
        let ledger = load_year(&ask.out, year)?;
        if ledger.unfolded == 0 {
            report.unchanged.push(year);
            continue;
        }
        let version = ledger.highest_version.saturating_add(1);
        let lines: Vec<&str> = ledger.lines.values().map(String::as_str).collect();
        let mut csv = String::with_capacity(lines.len().saturating_mul(320));
        csv.push_str(HEADER);
        csv.push('\n');
        for line in &lines {
            csv.push_str(line);
            csv.push('\n');
        }
        let csv_digest = brutex_core::blake3::hash(csv.as_bytes());
        let cells = cells_of(&lines)?;
        let refs: Vec<index::Cell<'_>> =
            cells.iter().map(|(s, d, a)| (s.as_str(), *d, *a)).collect();
        let idx = index::encode(year, &refs, ledger.last_segment, csv_digest)?;
        let identity = [
            (
                "layout",
                format!(
                    "rows header-blake3 {} index format {}",
                    hex8(&brutex_core::blake3::hash(HEADER.as_bytes())),
                    index::FORMAT
                ),
            ),
            ("csv blake3", hex32(&csv_digest)),
            ("segments through", ledger.last_segment.to_string()),
            ("rule T", "pull::gdfl_import::convert(Stocks) + Window::verdict".to_owned()),
            ("rule A", "rule T widened by convert(Indices) every-row extremes".to_owned()),
            (
                "fills",
                "read the stored rule-T one-second bars until a rule-A file version exists (D-5180)"
                    .to_owned(),
            ),
        ];
        let text = summary(year, version, &lines, &identity)?;
        let stem = format!("tick-to-second-{year}.v{version:03}");
        let csv_path = publish_new(&ask.out, &format!("{stem}.csv"), csv.as_bytes())?;
        publish_new(&ask.out, &format!("{stem}.summary.txt"), text.as_bytes())?;
        let decoded =
            index::YearIndex::decode(&idx, &ask.out.join(format!("{stem}.idx")), version)?;
        let mut lookups = Histogram::default();
        if let Ok((first, days)) = index::year_span(year) {
            for symbol in decoded.symbols() {
                for d in first..first + days {
                    if let Ok(day) = Day::from_days(d) {
                        let t = Instant::now();
                        let answer = decoded.status_of(symbol, day);
                        lookups.add(elapsed(t));
                        std::hint::black_box(answer.ok());
                    }
                }
            }
        }
        let fold_ns = elapsed(fold_started);
        publish_new(
            &ask.out,
            &format!("{stem}.timings.txt"),
            timings(&measured, &lookups, fold_ns).as_bytes(),
        )?;
        let idx_path = publish_new(&ask.out, &format!("{stem}.idx"), &idx)?;
        report.versions.push((year, csv_path, idx_path));
    }
    crate::note(
        &telemetry::Event::info(TARGET, "tick-to-second check finished")
            .with("share_days", report.planned)
            .with("covered", report.covered)
            .with("unrecorded", report.unrecorded)
            .with("days", report.days)
            .with("rows", report.rows)
            .with("not_exact", report.not_exact)
            .with("wall_ms", elapsed(started) / 1_000_000),
    );
    drop(lock);
    Ok(report)
}

/// A whole digest, as hex.
pub(crate) fn hex32(hash: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in hash {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// One event per day checked: the structural boundary of this verb.
fn note_day(day: Day, done: &DayOut) {
    let exact_rows = done.rows.iter().filter(|r| exact(&r.status)).count();
    let text = day.to_string();
    crate::note(
        &telemetry::Event::info(TARGET, "tick-to-second day checked")
            .with("day", text.as_str())
            .with("rows", done.rows.len())
            .with("exact", exact_rows)
            .with("other", done.rows.len().saturating_sub(exact_rows))
            .with("skipped", done.skipped)
            .with("elapsed_ms", done.elapsed_ns / 1_000_000),
    );
}

/// One event per instrument-month: its rows and how many were exact.
fn note_months(rows: &[Row]) {
    let mut months: BTreeMap<(&str, u16, u8), (u64, u64)> = BTreeMap::new();
    for r in rows {
        let slot = months
            .entry((r.symbol.as_str(), r.day.year(), r.day.month()))
            .or_insert((0, 0));
        slot.0 += 1;
        if exact(&r.status) {
            slot.1 += 1;
        }
    }
    for ((symbol, year, month), (days, exact_rows)) in months {
        let ym = format!("{year:04}-{month:02}");
        crate::note(
            &telemetry::Event::info(TARGET, "tick-to-second instrument-month")
                .with("symbol", symbol)
                .with("month", ym.as_str())
                .with("rows", days)
                .with("exact", exact_rows),
        );
    }
}

/// `cli tick-to-second-check`: see the module header.
pub(crate) fn command(args: &[&str], out: &mut String) -> u8 {
    command_with(args, &|v| std::env::var_os(v), &crate::store_root, out)
}

#[cfg(test)]
#[path = "tick_second_check_tests.rs"]
mod tests;
