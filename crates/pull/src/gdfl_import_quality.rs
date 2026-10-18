//! The GDFL day ledger, `<store>/imports/gdfl.dayflags` (D-3335 §1): what
//! the vendor gave each day, counted per kind-day (stocks, options) or per
//! index (indices), at import and by `cli gdfl-dayflags` for days already
//! imported. Findings DB-026, DB-029, DB-030, DB-056, DB-G96.
//!
//! # What it is for
//!
//! 30 May 2019 is a degraded vendor day: the options rows carry zero bid and
//! ask and lead with LTQ = 1 marker rows, the index files hold one row per
//! minute, and the first share prints carry a cumulative morning volume. Late
//! rows deferred under D-2802 leave holes and merged bars (2021-05-21, and the
//! October 2020 bursts stamped in UTC), 2022-03-07 has a 23-minute hole every
//! share shares, and three seconds of NIFTY on 2019-03-28 hold two values
//! 22.75 points apart. The bars are right under the agreed definition, and
//! none of this was recorded anywhere a page could read. **The bars are never
//! altered by any of it** (the operator's answer: flagged only); the ledger
//! counts, it never decides, and `/dayflags.json` (WP-33b) reads it.
//!
//! # The line (D-3335's names, in its order)
//!
//! `flags <kind> <day> <scope> v=1 src=<import|rescan> definition=<n>
//! <fields> max_defer_s=<n> member_src=<zip|none>`, D-3317's line rules (one space between
//! fields, every field after the key `name=value`), the last line of a key
//! wins. `<scope>` is the index's store symbol for `indices` and `*` for the
//! whole kind-day of `stocks` or `options`. Every field of a kind is always
//! present:
//!
//! - every kind: `member_bytes` (the day zip's size as its archive's
//!   directory states it; `0` when the source holds no zip, the tick store,
//!   and `member_src=none` says so: no real zip is 0 bytes), `late` (kept
//!   rows deferred), `max_back_s`, `deferred_s` (placed seconds that took at
//!   least one late row), `rows_refused` (D-3318; `0` for options, whose
//!   reader refuses a file whole, never a row);
//! - `indices`: `rows`, `seconds` (session seconds held), `session_s`,
//!   `ss_distinct`, `holes_5s`, `hole_max_s`, `hole_max_at` (the first
//!   missing second), `dual_spread_s`, `dual_spread_max_bps`,
//!   `zero_ltp_rows`;
//! - `stocks`: `files`, `files_traded`, `first_trade_p50`, `shared_holes`,
//!   `shared_hole_max_s`, `shared_hole_max_at`, `ltq0_new_ltp_rows`,
//!   `auction_prints`;
//! - `options`: `files`, `files_traded`, `zero_quote_files`,
//!   `ltq1_lead_files`, `ltq1_lead_rows`, `ltq0_new_ltp_rows`,
//!   `first_trade_p50`.
//!
//! `max_defer_s` and `member_src` are this writer's additions, which
//! D-3335's reader keeps out of the record ("a field a later writer adds"):
//! the most seconds a kept row was deferred by (its placed second less its
//! stamp; DB-029's "rows up to 171 s late" is this measure, where
//! `max_back_s` is how far a row stepped back from the running maximum), and
//! what `member_bytes` measured.
//!
//! # The halt and the sampled day (D-4928; WP-11c; DB-A04, DB-A07)
//!
//! Two more of this writer's additions sit before `max_defer_s`, on every
//! kind's line: `halt_s`, the in-session seconds the calendar of record
//! ([`crate::calendar::HALTS`], the kind's own segment) marks halted on the
//! day -- `0` on a day with no recorded halt -- and `halt_held_s`, the placed
//! seconds inside them (an index's own; summed over the files of a stocks or
//! options day). An index line carries two beside them: `flat_max_s` and
//! `flat_max_at`, the longest run of in-session rows, in file order, that
//! carry one unchanged value, as its length in seconds (last stamp less first,
//! plus one) and its first stamp. On 2020-03-13 the vendor repeats the index
//! value every second of the circuit-breaker halt: the bars are kept as
//! printed (the operator's answer: flagged only) and these fields are the
//! label. A halt's window is the calendar's, suspected until a circular is
//! recorded (D-3320); nothing here decides that a day halted.
//!
//! A **minute-sampled** index day needs no new field: [`is_sampled`] reads
//! D-3335's own `rows` and `ss_distinct` (2019-05-30: one row a minute, all
//! at second :59, 375 of 22,500 session seconds held), and the import says so
//! out loud as it files the day ([`DayLedger::sampled`]). A line written
//! before these fields has none of them, and a reader never takes a missing
//! one for `0`.
//!
//! # How each count is read (this module's reading of D-3335, said once)
//!
//! - A file is **traded** when one of its rows has LTQ above zero; its
//!   **first trade** is the first such row in file order, as stamped.
//!   `first_trade_p50` is the LOWER median over traded files, or `-`.
//! - **In session** is a second inside one of the day's windows
//!   ([`crate::gdfl_cm::session_of`], D-3319): minutes `from..=to`.
//! - An index **hole** is a run of in-session seconds holding no bar, the
//!   window's edges included, longer than [`INDEX_HOLE_S`] seconds.
//! - A **shared hole** is a run of at least [`SHARED_HOLE_S`] in-session
//!   seconds during which at least [`SHARED_HOLE_PERCENT`]% of the day's
//!   traded files are each inside a gap of at least [`SHARED_HOLE_S`] seconds
//!   between their traded seconds (the window's edges included).
//! - An **LTQ = 0 row with a new LTP** is an untraded row whose LTP is
//!   neither zero nor the LTP of the file's last traded row before it; rows
//!   before the file's first trade have no last traded LTP and are not
//!   counted. What GDFL means by these rows is UNVERIFIED (DB-056).
//! - **LTQ = 1 marker rows** lead a file: the traded rows of LTQ 1 before its
//!   first larger traded quantity, as `gdfl_lineage::LotEvidence` leaves them
//!   out of the lot.
//!
//! The thresholds are D-3335's, chosen, not sourced.
//!
//! # Cost
//!
//! Per file one pass over its rows and one over its bars, O(rows + bars);
//! a stocks day adds one difference array over the 86,400 seconds of the
//! day, O(1) in the files, and one O(seconds) scan. The median is one
//! `select_nth_unstable` over the traded files. Per day one append of a
//! bounded number of lines and one `fsync`. The ledger is read once per run
//! (O(its lines)) so a line equal to the key's last one, `src` aside, is not
//! written again: a re-run that changes nothing writes nothing.

use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use super::{BAR_DEFINITION, ImportKind, Placement, Tick};
use crate::calendar::Session;
use crate::gdfl_cm::DualSpread;
use crate::session::Day;

/// An index hole is counted when it is longer than this many seconds
/// (D-3335, chosen).
pub const INDEX_HOLE_S: u32 = 5;

/// A shared share hole is at least this many seconds (D-3335, chosen).
pub const SHARED_HOLE_S: u32 = 300;

/// A shared hole is shared by at least this percentage of the day's traded
/// share files (D-3335, chosen).
pub const SHARED_HOLE_PERCENT: u64 = 90;

/// An index day is minute-sampled when its in-session rows use at most this
/// many distinct seconds of the minute (D-3335's display rule for a degraded
/// index day, chosen, not sourced; D-4928 item 5 names it, and item 6 reads
/// the same number as bars a session minute where no line exists).
pub const SAMPLED_SS_DISTINCT: u64 = 2;

/// Whether an index line reads as a minute-sampled day: it has rows, and they
/// use at most [`SAMPLED_SS_DISTINCT`] distinct seconds of the minute. The one
/// rule the import's event and `/dayflags.json` both read (DB-A04).
#[must_use]
pub const fn is_sampled(rows: u64, ss_distinct: u64) -> bool {
    rows > 0 && ss_distinct <= SAMPLED_SS_DISTINCT
}

/// The seconds of a day.
const DAY_SECONDS: usize = 86_400;

/// The day ledger's path under a store root, beside the import journal.
#[must_use]
pub fn dayflags_path(store_root: &Path) -> PathBuf {
    store_root.join("imports").join("gdfl.dayflags")
}

/// Who wrote a line: the import, or `cli gdfl-dayflags` over days already
/// imported (D-3335's `src=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Src {
    /// The import, as it filed the day.
    Import,
    /// A read-only rescan of the vendor's files.
    Rescan,
}

impl Src {
    /// The word `src=` writes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Rescan => "rescan",
        }
    }
}

/// What a file's placement deferred (D-2802's late rows): how many placed
/// seconds took at least one late kept row, and the most seconds one was
/// deferred by (its placed second less its stamp).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Deferred {
    /// D-3335's `deferred_s`.
    pub seconds: u64,
    /// This writer's `max_defer_s`: how late, at most, a kept row was filed.
    pub max_s: u32,
}

/// What a reader counted about one file beyond its rows (D-3318, D-3320,
/// D-3335): constant size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReaderCounts {
    /// Rows the reader refused by name and did not keep (D-3318).
    pub rows_refused: u64,
    /// Of them, the rows whose LTP was zero.
    pub zero_ltp_rows: u64,
    /// An index file's same-second dual values (DB-G96).
    pub dual: DualSpread,
    /// Closing-auction prints diverted from the trades (D-3320).
    pub auction_prints: u64,
    /// For an options file, its rows whose bid or ask is not zero; `None`
    /// for a capital-market file, which is never judged on quotes.
    pub quoted_rows: Option<u64>,
}

/// The seconds `kind`'s one-second series spends in the halt the calendar of
/// record holds for `day`, as `[from, resume)`, or `None` on a day with no
/// recorded halt. O(1): one walk of the calendar's three-entry table.
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
#[must_use]
pub fn halt_window(kind: ImportKind, day: Day) -> Option<(u32, u32)> {
    let venue = match kind {
        ImportKind::Indices => crate::vendor::Venue::NseIndex,
        ImportKind::Stocks => crate::vendor::Venue::NseCash,
        ImportKind::Options => crate::vendor::Venue::NseDerivatives,
    };
    crate::calendar::halt_on(i64::from(day.days_from_epoch())).map(|halt| {
        let seconds = halt.seconds(venue);
        (seconds.from, seconds.resume)
    })
}

/// The seconds of `session`'s windows: D-3335's `session_s`.
#[must_use]
pub fn session_seconds(session: &Session) -> u64 {
    windows(session)
        .iter()
        .map(|&(from, to)| u64::from(to - from))
        .sum()
}

/// The parts of `halt` inside `spans`, each `[from, to)`: a span that only
/// touches the halt shares no second with it and is no part. `pub(super)` so
/// the import's tests can ask it directly (GF-1): its two callers add up
/// lengths, to which an empty part adds nothing, so only its own answer shows
/// whether an empty part is kept.
pub(super) fn halted_spans(spans: &[(u32, u32)], halt: Option<(u32, u32)>) -> Vec<(u32, u32)> {
    let Some((from, resume)) = halt else {
        return Vec::new();
    };
    spans
        .iter()
        .map(|&(a, b)| (a.max(from), b.min(resume)))
        .filter(|&(a, b)| a < b)
        .collect()
}

/// The seconds `[from, to)` of the day's session windows.
fn windows(session: &Session) -> Vec<(u32, u32)> {
    session
        .windows
        .iter()
        .take(usize::from(session.count))
        .map(|w| (u32::from(w.from) * 60, (u32::from(w.to) + 1) * 60))
        .collect()
}

/// The runs of seconds inside `[from, to)` that hold none of `held` (sorted,
/// deduplicated), each as `(first missing second, length)`, the window's
/// edges included. O(held).
fn gaps_in(held: &[u32], (from, to): (u32, u32)) -> Vec<(u32, u32)> {
    let start = held.partition_point(|&s| s < from);
    let mut out = Vec::new();
    let mut next_expected = from;
    for &s in held.get(start..).unwrap_or_default() {
        if s >= to {
            break;
        }
        if s > next_expected {
            out.push((next_expected, s - next_expected));
        }
        next_expected = s + 1;
    }
    if to > next_expected {
        out.push((next_expected, to - next_expected));
    }
    out
}

/// What one file of a day showed: counts of constant size, and for a share
/// file its in-session gaps of at least [`SHARED_HOLE_S`] for the day's
/// shared-hole count.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileQuality {
    /// The rows the import read (kept rows; a refused row is not here).
    pub rows: u64,
    /// Whether a row traded.
    pub traded: bool,
    /// The first traded row's stamp, in file order.
    pub first_trade: Option<u32>,
    /// Kept rows deferred, the largest back-step, and the placed seconds
    /// that took a late row.
    pub late: u64,
    /// See [`Self::late`].
    pub max_back_s: u32,
    /// See [`Self::late`].
    pub deferred_s: u64,
    /// The most seconds a kept row was deferred by.
    pub max_defer_s: u32,
    /// The reader's own counts.
    pub reader: ReaderCounts,
    /// LTQ = 0 rows carrying a new LTP (module doc).
    pub ltq0_new_ltp_rows: u64,
    /// Leading LTQ = 1 traded rows (options), and whether the first trade is
    /// one.
    pub ltq1_lead_rows: u64,
    /// See [`Self::ltq1_lead_rows`].
    pub ltq1_lead: bool,
    /// An index file: distinct seconds of the minute among session rows.
    pub ss_distinct: u32,
    /// An index file: in-session seconds held, and its holes over
    /// [`INDEX_HOLE_S`].
    pub seconds_held: u64,
    /// See [`Self::seconds_held`].
    pub holes: u64,
    /// The longest hole, and its first missing second.
    pub hole_max: Option<(u32, u32)>,
    /// A share file: its in-session gaps of at least [`SHARED_HOLE_S`], as
    /// `(first second, length)`.
    pub share_gaps: Vec<(u32, u32)>,
    /// The placed seconds inside the day's halt (module doc; WP-11c).
    pub halt_held_s: u64,
    /// An index file: the longest run of in-session rows carrying one
    /// unchanged value, as `(its first stamp, its length in seconds)`.
    pub flat_max: Option<(u32, u32)>,
}

/// A run of in-session index rows, in file order, carrying one value.
#[derive(Debug, Clone, Copy)]
struct FlatRun {
    ltp: i64,
    first: u32,
    last: u32,
}

impl FlatRun {
    /// Takes one in-session row: it extends `open` where its value is that
    /// run's, and otherwise closes that run into `longest` and opens its own.
    fn take(open: &mut Option<Self>, tick: &Tick, longest: &mut Option<(u32, u32)>) {
        match open {
            Some(run) if run.ltp == tick.ltp => {
                run.first = run.first.min(tick.sod);
                run.last = run.last.max(tick.sod);
            }
            _ => {
                if let Some(run) = open.take() {
                    run.close(longest);
                }
                *open = Some(Self {
                    ltp: tick.ltp,
                    first: tick.sod,
                    last: tick.sod,
                });
            }
        }
    }

    /// Folds this run into `longest`: the first of the longest wins.
    fn close(self, longest: &mut Option<(u32, u32)>) {
        let len = self.last - self.first + 1;
        if longest.is_none_or(|(_, max)| len > max) {
            *longest = Some((self.first, len));
        }
    }
}

/// One file's quality, from its rows in file order, the second of every bar
/// it built (ascending), what its placement counted and the reader's counts,
/// against the day's session and its halt ([`halt_window`]), if it has one.
#[must_use]
pub fn measure_file(
    kind: ImportKind,
    (session, halt): (&Session, Option<(u32, u32)>),
    ticks: &[Tick],
    bar_seconds: &[u32],
    (placement, deferred): (Placement, Deferred),
    reader: ReaderCounts,
) -> FileQuality {
    let spans = windows(session);
    let in_session = |sod: u32| spans.iter().any(|&(a, b)| sod >= a && sod < b);
    let mut quality = FileQuality {
        rows: u64::try_from(ticks.len()).unwrap_or(u64::MAX),
        late: u64::try_from(placement.late_rows).unwrap_or(u64::MAX),
        max_back_s: placement.max_back_s,
        deferred_s: deferred.seconds,
        max_defer_s: deferred.max_s,
        reader,
        ..FileQuality::default()
    };
    for (from, to) in halted_spans(&spans, halt) {
        let lo = bar_seconds.partition_point(|&s| s < from);
        let hi = bar_seconds.partition_point(|&s| s < to);
        quality.halt_held_s += u64::try_from(hi.saturating_sub(lo)).unwrap_or(0);
    }
    let mut last_traded: Option<i64> = None;
    let mut leading = true;
    let mut minute_seconds: u64 = 0;
    let mut flat: Option<FlatRun> = None;
    for tick in ticks {
        if kind == ImportKind::Indices {
            if in_session(tick.sod) {
                minute_seconds |= 1 << (tick.sod % 60);
                FlatRun::take(&mut flat, tick, &mut quality.flat_max);
            }
            continue;
        }
        if tick.ltq > 0 {
            if !quality.traded {
                quality.traded = true;
                quality.first_trade = Some(tick.sod);
                quality.ltq1_lead = tick.ltq == 1;
            }
            if leading {
                if tick.ltq == 1 {
                    quality.ltq1_lead_rows += 1;
                } else {
                    leading = false;
                }
            }
            last_traded = Some(tick.ltp);
        } else if let Some(last) = last_traded
            && tick.ltp != 0
            && tick.ltp != last
        {
            quality.ltq0_new_ltp_rows += 1;
        }
    }
    if let Some(run) = flat {
        run.close(&mut quality.flat_max);
    }
    match kind {
        ImportKind::Indices => {
            quality.traded = !ticks.is_empty();
            quality.ss_distinct = minute_seconds.count_ones();
            for span in &spans {
                let lo = bar_seconds.partition_point(|&s| s < span.0);
                let hi = bar_seconds.partition_point(|&s| s < span.1);
                quality.seconds_held += u64::try_from(hi.saturating_sub(lo)).unwrap_or(0);
                for (at, len) in gaps_in(bar_seconds, *span) {
                    if len > INDEX_HOLE_S {
                        quality.holes += 1;
                        if quality.hole_max.is_none_or(|(_, max)| len > max) {
                            quality.hole_max = Some((at, len));
                        }
                    }
                }
            }
            // An index row carries no quantity: none of the traded-row
            // counts apply to it.
            quality.ltq1_lead_rows = 0;
        }
        ImportKind::Stocks => {
            quality.ltq1_lead_rows = 0;
            quality.ltq1_lead = false;
            if quality.traded {
                for span in &spans {
                    quality.share_gaps.extend(
                        gaps_in(bar_seconds, *span)
                            .into_iter()
                            .filter(|&(_, len)| len >= SHARED_HOLE_S),
                    );
                }
            }
        }
        ImportKind::Options => {}
    }
    quality
}

/// `HH:MM:SS` of a second of the day, or `-`.
fn clock(sod: Option<u32>) -> String {
    sod.map_or_else(
        || "-".to_owned(),
        |s| format!("{:02}:{:02}:{:02}", s / 3_600, s / 60 % 60, s % 60),
    )
}

/// The counts every kind's line carries first.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Every {
    late: u64,
    max_back_s: u32,
    deferred_s: u64,
    rows_refused: u64,
    max_defer_s: u32,
}

impl Every {
    fn take(&mut self, file: &FileQuality) {
        self.late = self.late.saturating_add(file.late);
        self.max_back_s = self.max_back_s.max(file.max_back_s);
        self.deferred_s = self.deferred_s.saturating_add(file.deferred_s);
        self.max_defer_s = self.max_defer_s.max(file.max_defer_s);
        self.rows_refused = self.rows_refused.saturating_add(file.reader.rows_refused);
    }
}

/// One index's line.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct IndexLine {
    every: Every,
    rows: u64,
    seconds: u64,
    ss_distinct: u32,
    holes: u64,
    hole_max: Option<(u32, u32)>,
    dual: DualSpread,
    zero_ltp_rows: u64,
    halt_held_s: u64,
    flat_max: Option<(u32, u32)>,
}

/// An index of a day whose line reads as minute-sampled ([`is_sampled`]),
/// with the numbers that say so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sampled {
    /// The index's store symbol, the line's scope.
    pub scope: String,
    /// The rows the import read.
    pub rows: u64,
    /// The session seconds held, of [`Self::session_s`].
    pub seconds: u64,
    /// The seconds of the day's session windows.
    pub session_s: u64,
    /// The distinct seconds of the minute its rows use.
    pub ss_distinct: u32,
}

/// Of `sampled`, the indices whose line is among `news` -- what
/// [`Ledger::news`] answered and the run appended (D-4928 item 7): a rescan
/// names a sampled day when it writes the line that says so, and a later
/// rescan of the same rows, which writes nothing, names nothing again. At
/// most a day's indices against a day's lines.
#[must_use]
pub fn sampled_among(
    kind: ImportKind,
    day: Day,
    sampled: Vec<Sampled>,
    news: &[String],
) -> Vec<Sampled> {
    let written: Vec<String> = news
        .iter()
        .filter_map(|line| keyed(line).map(|(key, _)| key))
        .collect();
    sampled
        .into_iter()
        .filter(|one| written.contains(&format!("{} {day} {}", kind.as_str(), one.scope)))
        .collect()
}

/// The day of one kind, as its files are measured: per index, or the whole
/// kind-day of stocks or options.
#[derive(Debug, Clone)]
pub struct DayLedger {
    kind: ImportKind,
    day: Day,
    session: Session,
    member_bytes: Option<u64>,
    indices: BTreeMap<String, IndexLine>,
    every: Every,
    files: u64,
    files_traded: u64,
    first_trades: Vec<u32>,
    ltq0_new_ltp_rows: u64,
    auction_prints: u64,
    zero_quote_files: u64,
    ltq1_lead_files: u64,
    ltq1_lead_rows: u64,
    /// The day's halt for this kind's segment ([`halt_window`]).
    halt: Option<(u32, u32)>,
    /// A stocks or options day's placed seconds inside it, over its files.
    halt_held_s: u64,
    /// A stocks day's difference array of files in a long gap, by second.
    shared: Vec<i32>,
}

impl DayLedger {
    /// An empty day of `kind` on `day` under `session`.
    #[must_use]
    pub fn new(kind: ImportKind, day: Day, session: Session) -> Self {
        Self {
            kind,
            day,
            session,
            member_bytes: None,
            indices: BTreeMap::new(),
            every: Every::default(),
            files: 0,
            files_traded: 0,
            first_trades: Vec::new(),
            ltq0_new_ltp_rows: 0,
            auction_prints: 0,
            zero_quote_files: 0,
            ltq1_lead_files: 0,
            ltq1_lead_rows: 0,
            halt: halt_window(kind, day),
            halt_held_s: 0,
            shared: if kind == ImportKind::Stocks {
                vec![0; DAY_SECONDS + 1]
            } else {
                Vec::new()
            },
        }
    }

    /// The day this ledger counts.
    #[must_use]
    pub const fn day(&self) -> Day {
        self.day
    }

    /// The kind this ledger counts.
    #[must_use]
    pub const fn kind(&self) -> ImportKind {
        self.kind
    }

    /// The session the day is measured against.
    #[must_use]
    pub const fn session(&self) -> &Session {
        &self.session
    }

    /// The day's halt for this kind's segment, as `[from, resume)` seconds
    /// of the day: what [`measure_file`] measures a file against.
    #[must_use]
    pub const fn halt(&self) -> Option<(u32, u32)> {
        self.halt
    }

    /// The in-session seconds of the day's halt: the line's `halt_s`.
    fn halt_s(&self) -> u64 {
        halted_spans(&windows(&self.session), self.halt)
            .iter()
            .map(|&(from, to)| u64::from(to - from))
            .sum()
    }

    /// The day's indices that read as minute-sampled, by scope: what the
    /// import and the rescan say out loud as they write the day's lines
    /// (DB-A04). Empty for a stocks or options day, whose files are trades
    /// and are sparse by nature.
    #[must_use]
    pub fn sampled(&self) -> Vec<Sampled> {
        let session_s = session_seconds(&self.session);
        self.indices
            .iter()
            .filter(|(_, line)| is_sampled(line.rows, u64::from(line.ss_distinct)))
            .map(|(scope, line)| Sampled {
                scope: scope.clone(),
                rows: line.rows,
                seconds: line.seconds,
                session_s,
                ss_distinct: line.ss_distinct,
            })
            .collect()
    }

    /// Records the day zip's size, where the source read one.
    pub const fn set_member_bytes(&mut self, bytes: Option<u64>) {
        self.member_bytes = bytes;
    }

    /// How many files were taken.
    #[must_use]
    pub const fn files(&self) -> u64 {
        self.files
    }

    /// Takes one file of the day, filed under `symbol` (an index's scope).
    pub fn take(&mut self, symbol: &str, file: &FileQuality) {
        self.files += 1;
        if self.kind == ImportKind::Indices {
            let line = self.indices.entry(symbol.to_owned()).or_default();
            line.every.take(file);
            line.rows = line.rows.saturating_add(file.rows);
            line.seconds = line.seconds.saturating_add(file.seconds_held);
            line.ss_distinct = line.ss_distinct.max(file.ss_distinct);
            line.holes = line.holes.saturating_add(file.holes);
            if let Some((at, len)) = file.hole_max
                && line.hole_max.is_none_or(|(_, max)| len > max)
            {
                line.hole_max = Some((at, len));
            }
            line.dual.seconds = line.dual.seconds.saturating_add(file.reader.dual.seconds);
            if file.reader.dual.max_bps > line.dual.max_bps {
                line.dual.max_bps = file.reader.dual.max_bps;
                line.dual.max_at = file.reader.dual.max_at;
            }
            line.zero_ltp_rows = line.zero_ltp_rows.saturating_add(file.reader.zero_ltp_rows);
            line.halt_held_s = line.halt_held_s.saturating_add(file.halt_held_s);
            if let Some((at, len)) = file.flat_max
                && line.flat_max.is_none_or(|(_, max)| len > max)
            {
                line.flat_max = Some((at, len));
            }
            return;
        }
        self.every.take(file);
        self.halt_held_s = self.halt_held_s.saturating_add(file.halt_held_s);
        self.ltq0_new_ltp_rows = self
            .ltq0_new_ltp_rows
            .saturating_add(file.ltq0_new_ltp_rows);
        self.auction_prints = self
            .auction_prints
            .saturating_add(file.reader.auction_prints);
        if file.reader.quoted_rows == Some(0) && file.rows > 0 {
            self.zero_quote_files += 1;
        }
        if !file.traded {
            return;
        }
        self.files_traded += 1;
        if let Some(first) = file.first_trade {
            self.first_trades.push(first);
        }
        if file.ltq1_lead {
            self.ltq1_lead_files += 1;
        }
        self.ltq1_lead_rows = self.ltq1_lead_rows.saturating_add(file.ltq1_lead_rows);
        for &(at, len) in &file.share_gaps {
            // A gap lies inside the day, so both ends index the array; one
            // that does not is no gap of this day and is not counted.
            let from = usize::try_from(at).unwrap_or(usize::MAX);
            let to = usize::try_from(at.saturating_add(len)).unwrap_or(usize::MAX);
            if to < self.shared.len() {
                if let Some(slot) = self.shared.get_mut(from) {
                    *slot += 1;
                }
                if let Some(slot) = self.shared.get_mut(to) {
                    *slot -= 1;
                }
            }
        }
    }

    /// The lower median of the traded files' first trades.
    fn first_trade_p50(&mut self) -> Option<u32> {
        if self.first_trades.is_empty() {
            return None;
        }
        let at = (self.first_trades.len() - 1) / 2;
        let (_, median, _) = self.first_trades.select_nth_unstable(at);
        Some(*median)
    }

    /// The day's shared holes: count, and the longest as `(first second,
    /// length)`.
    fn shared_holes(&self) -> (u64, Option<(u32, u32)>) {
        if self.files_traded == 0 || self.shared.is_empty() {
            return (0, None);
        }
        // At least SHARED_HOLE_PERCENT% of the traded files, rounded up.
        let need = (self.files_traded * SHARED_HOLE_PERCENT).div_ceil(100);
        let mut cover = vec![0_i64; DAY_SECONDS];
        let mut running = 0_i64;
        for (second, slot) in cover.iter_mut().enumerate() {
            running += i64::from(self.shared.get(second).copied().unwrap_or(0));
            *slot = running;
        }
        let mut count = 0_u64;
        let mut longest: Option<(u32, u32)> = None;
        for (from, to) in windows(&self.session) {
            let mut run: Option<u32> = None;
            for second in from..=to {
                let covered = second < to
                    && cover
                        .get(second as usize)
                        .is_some_and(|&n| u64::try_from(n).unwrap_or(0) >= need);
                match (covered, run) {
                    (true, None) => run = Some(second),
                    (false, Some(start)) => {
                        let len = second - start;
                        if len >= SHARED_HOLE_S {
                            count += 1;
                            if longest.is_none_or(|(_, max)| len > max) {
                                longest = Some((start, len));
                            }
                        }
                        run = None;
                    }
                    _ => {}
                }
            }
        }
        (count, longest)
    }

    /// The day's lines, by scope, without `src=`: D-3335's fields in its
    /// order, then the halt's (and an index's unchanged-price run), `max_defer_s` and
    /// `member_src`. `whole` is false for a run
    /// filtered to some instruments: a stocks or options line counts the
    /// WHOLE kind-day, so a filtered run writes none. A day none of whose
    /// files was read clean (its listing refused, or every file) has nothing
    /// to count and writes no line: the journal names why.
    #[must_use]
    pub fn lines(&mut self, whole: bool) -> Vec<(String, String)> {
        let member = |bytes: Option<u64>| format!("member_bytes={}", bytes.unwrap_or(0));
        let every = |e: &Every, member_bytes: Option<u64>| {
            format!(
                "{} late={} max_back_s={} deferred_s={} rows_refused={}",
                member(member_bytes),
                e.late,
                e.max_back_s,
                e.deferred_s,
                e.rows_refused
            )
        };
        let src_of = |bytes: Option<u64>| if bytes.is_some() { "zip" } else { "none" };
        let (kind, day) = (self.kind, self.day);
        let head = move |scope: &str| format!("flags {} {day} {scope} v=1", kind.as_str());
        let p50 = self.first_trade_p50();
        let session_s = session_seconds(&self.session);
        let halt_s = self.halt_s();
        let mut out = Vec::new();
        match self.kind {
            ImportKind::Indices => {
                for (scope, line) in &self.indices {
                    let mut text = every(&line.every, self.member_bytes);
                    let _infallible = write!(
                        text,
                        " rows={} seconds={} session_s={session_s} ss_distinct={} holes_5s={} \
                         hole_max_s={} hole_max_at={} dual_spread_s={} dual_spread_max_bps={} \
                         zero_ltp_rows={} halt_s={halt_s} halt_held_s={} flat_max_s={} \
                         flat_max_at={} max_defer_s={} member_src={}",
                        line.rows,
                        line.seconds,
                        line.ss_distinct,
                        line.holes,
                        line.hole_max.map_or(0, |(_, len)| len),
                        clock(line.hole_max.map(|(at, _)| at)),
                        line.dual.seconds,
                        line.dual.max_bps,
                        line.zero_ltp_rows,
                        line.halt_held_s,
                        line.flat_max.map_or(0, |(_, len)| len),
                        clock(line.flat_max.map(|(at, _)| at)),
                        line.every.max_defer_s,
                        src_of(self.member_bytes),
                    );
                    out.push((head(scope), text));
                }
            }
            ImportKind::Stocks if whole && self.files > 0 => {
                let (holes, longest) = self.shared_holes();
                let mut text = every(&self.every, self.member_bytes);
                let _infallible = write!(
                    text,
                    " files={} files_traded={} first_trade_p50={} shared_holes={holes} \
                     shared_hole_max_s={} shared_hole_max_at={} ltq0_new_ltp_rows={} \
                     auction_prints={} halt_s={halt_s} halt_held_s={} max_defer_s={} \
                     member_src={}",
                    self.files,
                    self.files_traded,
                    clock(p50),
                    longest.map_or(0, |(_, len)| len),
                    clock(longest.map(|(at, _)| at)),
                    self.ltq0_new_ltp_rows,
                    self.auction_prints,
                    self.halt_held_s,
                    self.every.max_defer_s,
                    src_of(self.member_bytes),
                );
                out.push((head("*"), text));
            }
            ImportKind::Options if whole && self.files > 0 => {
                let mut text = every(&self.every, self.member_bytes);
                let _infallible = write!(
                    text,
                    " files={} files_traded={} zero_quote_files={} ltq1_lead_files={} \
                     ltq1_lead_rows={} ltq0_new_ltp_rows={} first_trade_p50={} \
                     halt_s={halt_s} halt_held_s={} max_defer_s={} member_src={}",
                    self.files,
                    self.files_traded,
                    self.zero_quote_files,
                    self.ltq1_lead_files,
                    self.ltq1_lead_rows,
                    self.ltq0_new_ltp_rows,
                    clock(p50),
                    self.halt_held_s,
                    self.every.max_defer_s,
                    src_of(self.member_bytes),
                );
                out.push((head("*"), text));
            }
            ImportKind::Stocks | ImportKind::Options => {}
        }
        out
    }
}

/// One whole flags line: `head`, `v=1` already in it, then `src=`,
/// `definition=` and the fields.
#[must_use]
pub fn render(head: &str, src: Src, fields: &str) -> String {
    format!(
        "{head} src={} definition={BAR_DEFINITION} {fields}",
        src.as_str()
    )
}

/// The key of a flags line (`<kind> <day> <scope>`) and the line less its
/// `src=` field, or `None` for a line that is not a flags line.
fn keyed(line: &str) -> Option<(String, String)> {
    let mut words = line.split(' ');
    if words.next() != Some("flags") {
        return None;
    }
    let (kind, day, scope) = (words.next()?, words.next()?, words.next()?);
    let unsourced: Vec<&str> = line
        .split(' ')
        .filter(|word| !word.starts_with("src="))
        .collect();
    Some((format!("{kind} {day} {scope}"), unsourced.join(" ")))
}

/// The day ledger as one run holds it: the last line of every key, its
/// `src=` taken out, so a line that says nothing new is not written again.
#[derive(Debug, Default)]
pub struct Ledger {
    path: PathBuf,
    last: HashMap<String, String>,
}

impl Ledger {
    /// Reads the ledger at `path`; a missing one is empty. A line this
    /// writer cannot key (a torn last line, closed by the next append) is
    /// left for the reader to count; it never stops a run.
    ///
    /// # Errors
    ///
    /// The path and why, when it exists and cannot be read.
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let text = match std::fs::read(&path) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(why) => return Err(format!("{}: {why}", path.display())),
        };
        let mut last = HashMap::new();
        for line in text.lines() {
            if let Some((key, unsourced)) = keyed(line) {
                last.insert(key, unsourced);
            }
        }
        Ok(Self { path, last })
    }

    /// The lines of `fresh` that say something the ledger's last line of
    /// their key does not (`src=` aside), each rendered with `src`, and
    /// noted as the key's last.
    pub fn news(&mut self, fresh: Vec<(String, String)>, src: Src) -> Vec<String> {
        let mut out = Vec::new();
        for (head, fields) in fresh {
            let line = render(&head, src, &fields);
            let Some((key, unsourced)) = keyed(&line) else {
                continue;
            };
            if self.last.get(&key) == Some(&unsourced) {
                continue;
            }
            self.last.insert(key, unsourced);
            out.push(line);
        }
        out
    }

    /// Appends `lines`, durably.
    ///
    /// # Errors
    ///
    /// The path and why.
    pub fn append(&self, lines: &[String]) -> Result<(), String> {
        super::append_lines(&self.path, lines)
    }
}
