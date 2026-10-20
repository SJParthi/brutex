//! The risk-free rate as a DATED series: the RBI 91-day Treasury Bill
//! cut-off yield, one value per weekly auction. D-2810.
//!
//! # What this adds, and what it leaves alone
//!
//! [`crate::pricing::Rate`] already refuses a rate with no source, and
//! [`crate::pricing::solve_iv`] and [`crate::pricing::greeks_at`] already price
//! with one. What nothing in the workspace could do was answer *which* rate
//! applied on a given day of an eight-year history: the only arm that existed
//! in practice was [`crate::pricing::RateSource::Operator`], one figure per
//! request. This module is that answer and nothing else. It builds a
//! [`crate::pricing::Rate`] through the existing constructor, cited as
//! [`crate::pricing::RateSource::Charter`] with [`CHARTER_CITE`], so
//! `Rate`, `RateSource`, the solver and the store's one-byte provenance are
//! unchanged.
//!
//! # The series is DATA, read at run time from a path the caller supplies
//!
//! The values are not in this repository. `docs/00-charter.md` §4h names the
//! source and the file shape; the operator keeps the table outside the tree and
//! passes its path. There is no default path and no fallback: a missing,
//! unreadable or malformed file is refused by name ([`SeriesError`]). The file's
//! SHA-256 is carried ([`Series::sha256`]) so a priced row can name the exact
//! table it was priced under — `CLAUDE.md` §3 rule 3.
//!
//! # The as-of rule — no look-ahead
//!
//! A trade on day `D` uses the latest auction whose press release is dated
//! **strictly before** `D`. Not "on or before": RBI announces a T-bill result
//! on the auction day itself, after bidding closes, and the table records the
//! release DATE but not its time. A same-day result is unknown for most of that
//! session, so admitting it would price a morning trade with an afternoon
//! number — `CLAUDE.md` §3 rule 7. D-2810.
//!
//! A day on or before the first release is refused as
//! [`LookupError::BeforeSeries`]. A day more than [`MAX_AGE_DAYS`] after the
//! applicable release is refused as [`LookupError::Stale`]: the series is
//! weekly, so a gap that long means an auction is missing from the table, and
//! carrying an old yield forward over it would be an invented value that no
//! report could tell from a real one.
//!
//! # A DOCUMENTED gap is carried, and says so (DB-B44, R3-30)
//!
//! RBI does not set a 91-day cut-off at every scheduled auction: an auction
//! can be cancelled, a result can state `NA` for the 91-day bill, and a
//! monthly listing can hold no auction in a week. DB-B44 replayed the
//! operator's table under the bare 14-day rule and found greeks refused on
//! ten sessions, two of them expiry days, every one inside such a week. So
//! the table may carry a GAP ROW for a scheduled auction that set no cut-off:
//! the same eight columns, `cut_off_price` one of [`GapKind`]'s words
//! (`CANCELLED`, `NA`, `NOT_HELD`), `implicit_yield_pct` empty, `released`
//! the day the gap was published, and `source_url` the RBI page that says so
//! — the row is the documentation, and a gap row without an RBI URL is
//! refused like any other row. A gap row restarts the staleness clock
//! (staleness is measured from the last SCHEDULED auction, not the last
//! result) and a day it covers is answered with the previous result's rate,
//! LABELLED: [`AsOf::carried`] names the gap and [`AsOf::state`] is
//! `rate_carried`. A gap is known only from the day after its release, as a
//! result is. A gap nobody documented has no row, and the 14-day refusal
//! applies to it exactly as before
//! (`a_documented_rate_cancellation_is_carried_and_labelled`,
//! `an_unexplained_rate_gap_is_still_refused`).
//!
//! # The conversion, and its source
//!
//! RBI publishes the *implicit yield at cut-off price*. It is a simple yield on
//! the price, annualised by `365/91` — not a discount-basis yield. That is
//! checked on every row rather than assumed: [`Series::parse`] recomputes
//! `(100 − P) / P × 365/91` from the published cut-off price in integer
//! arithmetic and refuses the row if the published yield disagrees by more
//! than one unit in its fourth decimal ([`RowFault::YieldDisagreesWithPrice`]).
//!
//! `greeks::bsm` discounts continuously, and so does
//! [`crate::pricing::Rate::annual`]. A simple rate `y` over a period of
//! `τ = 91/365` years is converted with
//! `r = ln(1 + y·τ) / τ` — the `m`-times-a-year rule `R_c = m·ln(1 + R_m/m)`
//! of Hull, *Options, Futures, and Other Derivatives*, chapter 4, "Continuous
//! Compounding", with `m = 1/τ = 365/91`. Cited in `docs/00-charter.md` §4h.
//!
//! # Cost
//!
//! [`Series::on`] is O(1): one subtraction, one bounds-checked `Vec` read and
//! one copy, whatever the table holds. The table is DENSE BY DAY — one slot per
//! calendar day from the day after the first release to [`MAX_AGE_DAYS`] after
//! the last — so no lookup searches. Measured at 1×, 10× and 100× the table by
//! `pull::bench::rate_lookup_is_flat`, row C-RF-01.
//! [`Series::parse`] is O(rows + days) once, and the days are bounded by
//! [`TradeDay`]'s own 1990–2100 window, so the table is at most about 40,560
//! four-byte slots.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::pricing::{PricingError, Rate, RateSource, TradeDay};
use crate::tenor::YearBasis;

/// The citation every rate from this series carries.
///
/// Names the charter section, not a value: the values are dated and live in
/// the operator's table, whose digest travels beside the rate.
pub const CHARTER_CITE: &str = "§4h RBI 91-day Treasury Bill implicit yield at cut-off price, as of the latest release before the trade day (D-2810)";

/// The header the table must carry, byte for byte.
///
/// Eight tab-separated columns. Four are read — `released`, `cut_off_price`,
/// `implicit_yield_pct`, `source_url`. `auction_date`, `settlement_date`,
/// `full_result_url` and `announcement_url` are carried for a human checking a
/// row against RBI and are not interpreted: the as-of rule uses the release
/// date alone, because that is when the yield became known.
pub const HEADER: &str = "auction_date\tsettlement_date\treleased\tcut_off_price\timplicit_yield_pct\tsource_url\tfull_result_url\tannouncement_url";

/// Where a row's source must point: RBI's own press-release pages.
pub const SOURCE_PREFIX: &str = "https://www.rbi.org.in/";

/// The oldest a release may be and still apply, in calendar days.
///
/// The auction is weekly, so consecutive releases are seven days apart and a
/// holiday moves one by a day or two. Fourteen admits one such shift with a
/// week to spare and refuses a whole missing week. D-2810.
pub const MAX_AGE_DAYS: i32 = 14;

/// The oldest a CARRIED result may be, in calendar days: two staleness
/// windows.
///
/// A documented gap row restarts the staleness clock, so without this a run
/// of documented gaps would carry one result's rate with no age bound at all
/// (R3-30 repair 1): ten weekly `CANCELLED` rows would price on a rate ten
/// weeks old. Twenty-eight days admits the longest documented runs the
/// operator's table holds -- two consecutive weeks without a 91-day cut-off
/// (2024-09-18 and 2024-09-25; 2026-03-25 and the week of 2026-03-30, DB-B44)
/// -- with the holiday shift [`MAX_AGE_DAYS`] already allows, and refuses a
/// third missing week by name ([`LookupError::CarriedTooLong`]) instead of
/// pricing on it. A bound of the series, not of a vendor fact.
pub const MAX_CARRIED_AGE_DAYS: i32 = 2 * MAX_AGE_DAYS;

/// The largest file this will read. A weekly series for a century is under
/// a megabyte; anything bigger is not this table.
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// The bill's tenor in days, as RBI names it.
const TENOR_DAYS: u64 = 91;
/// The year RBI annualises the implicit yield over.
const YEAR_DAYS: u64 = 365;
/// Par, in ten-thousandths of a rupee.
const PAR_E4: u32 = 1_000_000;

/// One auction, as the table states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Auction {
    /// The date of the press release that published the result.
    pub released: TradeDay,
    /// The cut-off price, in ten-thousandths of a rupee per ₹100 face value.
    pub price_e4: u32,
    /// The implicit yield at cut-off, in ten-thousandths of a percent.
    pub yield_e4: u32,
}

/// Why a scheduled auction set no 91-day cut-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GapKind {
    /// The auction was cancelled (`CANCELLED`).
    Cancelled,
    /// The result states `NA` for the 91-day bill (`NA`).
    NotAvailable,
    /// The listing holds no auction that week (`NOT_HELD`).
    NotHeld,
}

impl GapKind {
    /// Every kind.
    pub const ALL: [Self; 3] = [Self::Cancelled, Self::NotAvailable, Self::NotHeld];

    /// The word the table's `cut_off_price` column carries.
    #[must_use]
    pub const fn column(self) -> &'static str {
        match self {
            Self::Cancelled => "CANCELLED",
            Self::NotAvailable => "NA",
            Self::NotHeld => "NOT_HELD",
        }
    }

    /// The wire word.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::NotAvailable => "na",
            Self::NotHeld => "not_held",
        }
    }

    /// The kind a `cut_off_price` column names, or `None` for a price.
    #[must_use]
    pub fn of_column(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.column() == text)
    }
}

/// A documented gap: a scheduled auction that set no 91-day cut-off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// The day the gap was published; it is known from the day after.
    pub released: TradeDay,
    /// Why there is no cut-off.
    pub kind: GapKind,
}

/// The rate that applies on one day, and the auction it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AsOf {
    /// Continuously compounded, `Calendar365`, cited as [`CHARTER_CITE`].
    pub rate: Rate,
    /// The auction whose release is the latest strictly before the day asked.
    pub auction: Auction,
    /// The documented gap the day is carried over: the latest scheduled
    /// auction before the day set no cut-off, so the rate is the previous
    /// result's, LABELLED. `None` when the latest scheduled auction before
    /// the day is the result itself.
    pub carried: Option<Gap>,
}

impl AsOf {
    /// `rate_fresh`, or `rate_carried` over a documented gap.
    #[must_use]
    pub const fn state(&self) -> &'static str {
        if self.carried.is_some() {
            "rate_carried"
        } else {
            "rate_fresh"
        }
    }
}

/// The loaded series, indexed by day.
#[derive(Debug, Clone)]
pub struct Series {
    /// The first day a lookup can answer: the day after the first release.
    first: i32,
    /// One slot per day from `first`: the index of the applicable auction plus
    /// one, or zero where the latest release is older than [`MAX_AGE_DAYS`].
    by_day: Vec<u32>,
    /// One slot per day from `first`, beside `by_day`: the index of the
    /// documented gap the day is carried over plus one, or zero.
    carried_by: Vec<u32>,
    auctions: Vec<(Auction, Rate)>,
    gaps: Vec<Gap>,
    first_release: TradeDay,
    last_release: TradeDay,
    sha256: [u8; 32],
}

/// Why a row was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum RowFault {
    /// Not eight tab-separated fields.
    FieldCount {
        /// How many there were.
        found: usize,
    },
    /// A date that is not `YYYY-MM-DD` and a real day in [`TradeDay`]'s window.
    Date {
        /// The column.
        field: &'static str,
    },
    /// A number that is not a plain decimal with at most four places.
    Decimal {
        /// The column.
        field: &'static str,
    },
    /// A cut-off price at or above par, or zero. A T-bill is issued at a
    /// discount; anything else is a transposed or corrupt field.
    PriceNotBelowPar {
        /// The price, in ten-thousandths of a rupee.
        price_e4: u32,
    },
    /// The published yield is not `(100 − P)/P × 365/91` of the published
    /// price to within one unit in the fourth decimal.
    YieldDisagreesWithPrice {
        /// The yield the row states, ten-thousandths of a percent.
        published: u32,
        /// The yield its price implies. `u64`: a price near zero implies a
        /// yield past `u32`.
        implied: u64,
    },
    /// The source is not an RBI press-release URL.
    Source,
    /// The release date is not after the previous row's.
    NotAfterPrevious,
    /// A gap row (`cut_off_price` `CANCELLED`, `NA` or `NOT_HELD`) whose
    /// `implicit_yield_pct` is not empty: a gap states no yield.
    GapWithYield,
    /// A gap row before the table's first result: there is no rate to carry.
    GapBeforeAnyResult,
    /// The continuous rate the row converts to was refused by
    /// [`Rate::measured`].
    Rate(PricingError),
}

/// Why the table could not be loaded.
#[derive(Debug, Clone, PartialEq)]
pub enum SeriesError {
    /// The file could not be read.
    Read {
        /// The operating system's reason.
        why: String,
    },
    /// The file is larger than [`MAX_FILE_BYTES`].
    TooLarge {
        /// Its length.
        bytes: u64,
    },
    /// The file is not UTF-8.
    NotText,
    /// The first line is not [`HEADER`].
    Header,
    /// The header is there and no auction follows it.
    Empty,
    /// A row was refused.
    Row {
        /// One-based line number in the file.
        line: usize,
        /// What was wrong with it.
        fault: RowFault,
    },
}

/// Why a day has no rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupError {
    /// The day is on or before the first release in the table.
    BeforeSeries {
        /// The day asked.
        day: TradeDay,
        /// The first release.
        first_release: TradeDay,
    },
    /// The latest release before the day is more than [`MAX_AGE_DAYS`] old —
    /// an auction is missing from the table, or the table ends before the day.
    Stale {
        /// The day asked.
        day: TradeDay,
    },
    /// Documented gaps cover the day, but the result they would carry is more
    /// than [`MAX_CARRIED_AGE_DAYS`] old: a run of gaps carries a rate only so
    /// far, and past it no rate is priced.
    CarriedTooLong {
        /// The day asked.
        day: TradeDay,
        /// The release of the result that would have been carried.
        result: TradeDay,
        /// The documented gap that covers the day.
        gap: Gap,
    },
}

impl std::fmt::Display for SeriesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { why } => write!(f, "the T-bill series could not be read: {why}"),
            Self::TooLarge { bytes } => write!(
                f,
                "the T-bill series is {bytes} bytes, over the {MAX_FILE_BYTES}-byte \
                 bound; a weekly series is far smaller, so this is not that table"
            ),
            Self::NotText => write!(f, "the T-bill series is not UTF-8 text"),
            Self::Header => write!(
                f,
                "the T-bill series does not start with the header docs/00-charter.md §4h names"
            ),
            Self::Empty => write!(f, "the T-bill series holds a header and no auction"),
            Self::Row { line, fault } => {
                write!(f, "the T-bill series refuses line {line}: {fault:?}")
            }
        }
    }
}

impl std::error::Error for SeriesError {}

impl std::fmt::Display for LookupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BeforeSeries { day, first_release } => write!(
                f,
                "no T-bill result was released before {day}; the series starts \
                 with the release of {first_release}, and no earlier rate is invented"
            ),
            Self::Stale { day } => write!(
                f,
                "the latest T-bill result before {day} is more than {MAX_AGE_DAYS} \
                 days old or absent, and no documented gap row explains why, so \
                 an auction is missing from the series; no rate is carried \
                 forward over an unexplained gap"
            ),
            Self::CarriedTooLong { day, result, gap } => write!(
                f,
                "documented gaps cover {day} (the latest, released {}, is {}), but the \
                 result they would carry was released {result}, more than \
                 {MAX_CARRIED_AGE_DAYS} days before; a rate is carried over documented \
                 gaps only that far, and no older rate is priced",
                gap.released,
                gap.kind.word(),
            ),
        }
    }
}

impl std::error::Error for LookupError {}

/// A plain decimal with at most four places, in ten-thousandths.
fn decimal_e4(text: &str) -> Option<u32> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty()
        || frac.len() > 4
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole: u32 = whole.parse().ok()?;
    let mut scaled = whole.checked_mul(10_000)?;
    let mut unit = 1_000;
    for digit in frac.bytes() {
        scaled = scaled.checked_add(u32::from(digit - b'0') * unit)?;
        unit /= 10;
    }
    Some(scaled)
}

/// `YYYY-MM-DD`, as a day [`TradeDay`] accepts.
fn date(text: &str) -> Option<TradeDay> {
    let mut parts = text.split('-');
    let (Some(y), Some(m), Some(d), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return None;
    }
    TradeDay::new(y.parse().ok()?, m.parse().ok()?, d.parse().ok()?).ok()
}

/// The implicit yield a cut-off price states, in ten-thousandths of a percent,
/// rounded half up: `(100 − P)/P × 365/91 × 100`.
///
/// Integer throughout. `price_e4` is below par and above zero, so the divisor
/// is not zero and the numerator is below `10^6 · 365 · 10^6 · 2 < 2^63`.
fn implied_yield_e4(price_e4: u32) -> u64 {
    let discount = u64::from(PAR_E4 - price_e4);
    let numerator = discount * YEAR_DAYS * 1_000_000 * 2;
    let denominator = u64::from(price_e4) * TENOR_DAYS;
    (numerator / denominator).div_ceil(2)
}

/// The continuously compounded rate a simple 91-day yield converts to.
///
/// `r = ln(1 + y·τ) / τ`, `τ = 91/365` — Hull ch. 4, see the module note.
#[expect(
    clippy::float_arithmetic,
    reason = "a rate is not a price. CLAUDE.md §7 keeps floats out of prices; \
              greeks::bsm takes the rate as a float and the conversion is a \
              logarithm, which has no integer form"
)]
fn continuous(yield_e4: u32) -> f64 {
    let tau = 91.0 / 365.0;
    let simple = f64::from(yield_e4) / 1_000_000.0;
    (simple * tau).ln_1p() / tau
}

/// One data row: a result, or a documented gap.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Entry {
    Result(Auction, Rate),
    Gap(Gap),
}

impl Entry {
    const fn released(&self) -> TradeDay {
        match self {
            Self::Result(auction, _) => auction.released,
            Self::Gap(gap) => gap.released,
        }
    }
}

/// One data row, checked: a gap row when its `cut_off_price` is a
/// [`GapKind`] word, a result row otherwise.
fn entry(text: &str, previous: Option<TradeDay>) -> Result<Entry, RowFault> {
    let fields: Vec<&str> = text.split('\t').collect();
    let [_, _, released, price, yield_pct, source, _, _] = fields.as_slice() else {
        return Err(RowFault::FieldCount {
            found: fields.len(),
        });
    };
    let Some(kind) = GapKind::of_column(price) else {
        return row(text, previous).map(|(auction, rate)| Entry::Result(auction, rate));
    };
    let released = date(released).ok_or(RowFault::Date { field: "released" })?;
    if !yield_pct.is_empty() {
        return Err(RowFault::GapWithYield);
    }
    if !source.starts_with(SOURCE_PREFIX) {
        return Err(RowFault::Source);
    }
    if previous.is_some_and(|before| !before.before(released)) {
        return Err(RowFault::NotAfterPrevious);
    }
    Ok(Entry::Gap(Gap { released, kind }))
}

/// One result row, checked.
fn row(text: &str, previous: Option<TradeDay>) -> Result<(Auction, Rate), RowFault> {
    let fields: Vec<&str> = text.split('\t').collect();
    let [
        _auction,
        _settled,
        released,
        price,
        yield_pct,
        source,
        _full,
        _announced,
    ] = fields.as_slice()
    else {
        return Err(RowFault::FieldCount {
            found: fields.len(),
        });
    };
    let released = date(released).ok_or(RowFault::Date { field: "released" })?;
    let price_e4 = decimal_e4(price).ok_or(RowFault::Decimal {
        field: "cut_off_price",
    })?;
    let yield_e4 = decimal_e4(yield_pct).ok_or(RowFault::Decimal {
        field: "implicit_yield_pct",
    })?;
    if price_e4 == 0 || price_e4 >= PAR_E4 {
        return Err(RowFault::PriceNotBelowPar { price_e4 });
    }
    let implied = implied_yield_e4(price_e4);
    if implied.abs_diff(u64::from(yield_e4)) > 1 {
        return Err(RowFault::YieldDisagreesWithPrice {
            published: yield_e4,
            implied,
        });
    }
    if !source.starts_with(SOURCE_PREFIX) {
        return Err(RowFault::Source);
    }
    if previous.is_some_and(|before| !before.before(released)) {
        return Err(RowFault::NotAfterPrevious);
    }
    let rate = Rate::measured(
        continuous(yield_e4),
        YearBasis::Calendar365,
        RateSource::Charter(CHARTER_CITE),
    )
    .map_err(RowFault::Rate)?;
    Ok((
        Auction {
            released,
            price_e4,
            yield_e4,
        },
        rate,
    ))
}

impl Series {
    /// Reads the table at `path`. The path is the caller's; there is no default.
    ///
    /// # Errors
    ///
    /// [`SeriesError::Read`], [`SeriesError::TooLarge`] and
    /// [`SeriesError::NotText`] for the file, and every refusal of
    /// [`Self::parse`].
    pub fn read(path: &Path) -> Result<Self, SeriesError> {
        let read = |why: std::io::Error| SeriesError::Read {
            why: why.to_string(),
        };
        let bytes = std::fs::metadata(path).map_err(read)?.len();
        if bytes > MAX_FILE_BYTES {
            return Err(SeriesError::TooLarge { bytes });
        }
        let raw = std::fs::read(path).map_err(read)?;
        let text = String::from_utf8(raw).map_err(|_| SeriesError::NotText)?;
        Self::parse(&text)
    }

    /// Builds the dense day table from the table's text.
    ///
    /// # Errors
    ///
    /// [`SeriesError::Header`], [`SeriesError::Empty`], [`SeriesError::Row`]
    /// naming the line and the fault, and [`SeriesError::SpanTooLong`].
    pub fn parse(text: &str) -> Result<Self, SeriesError> {
        let mut lines = text.lines();
        if lines.next() != Some(HEADER) {
            return Err(SeriesError::Header);
        }
        let mut entries: Vec<Entry> = Vec::new();
        let mut seen_result = false;
        for (offset, line) in lines.enumerate() {
            let previous = entries.last().map(Entry::released);
            let parsed = entry(line, previous).map_err(|fault| SeriesError::Row {
                line: offset + 2,
                fault,
            })?;
            seen_result |= matches!(parsed, Entry::Result(..));
            if !seen_result {
                return Err(SeriesError::Row {
                    line: offset + 2,
                    fault: RowFault::GapBeforeAnyResult,
                });
            }
            entries.push(parsed);
        }
        let (Some(head), Some(tail)) = (entries.first(), entries.last()) else {
            return Err(SeriesError::Empty);
        };
        let (first_release, last_release) = (head.released(), tail.released());
        // Each row -- a result or a documented gap -- owns the days after its
        // release up to and including the next row's release day (a release
        // is not known on its own day), or up to MAX_AGE_DAYS after it for the
        // last one. Within that run a day is fresh while it is at most
        // MAX_AGE_DAYS after THAT ROW's release, so staleness is measured
        // from the last SCHEDULED auction, and a zero slot otherwise. A day a
        // gap row owns carries the latest result's rate and names the gap.
        // Two bounded ranges, no search: the table is contiguous from the
        // day after the first release.
        let mut auctions: Vec<(Auction, Rate)> = Vec::new();
        let mut gaps: Vec<Gap> = Vec::new();
        let mut by_day = Vec::new();
        let mut carried_by = Vec::new();
        for (index, row) in entries.iter().enumerate() {
            let carried = match row {
                Entry::Result(auction, rate) => {
                    auctions.push((*auction, *rate));
                    0
                }
                Entry::Gap(gap) => {
                    gaps.push(*gap);
                    // `len` fits: at most one row per day of a window of
                    // about 40,560 days.
                    u32::try_from(gaps.len()).unwrap_or(u32::MAX)
                }
            };
            let slot = u32::try_from(auctions.len()).unwrap_or(u32::MAX);
            let released = row.released().ordinal();
            let owns_until = entries
                .get(index + 1)
                .map_or(released + MAX_AGE_DAYS, |next| next.released().ordinal());
            for day in released + 1..=owns_until {
                let fresh = day - released <= MAX_AGE_DAYS;
                by_day.push(if fresh { slot } else { 0 });
                carried_by.push(if fresh { carried } else { 0 });
            }
        }
        Ok(Self {
            first: first_release.ordinal() + 1,
            by_day,
            carried_by,
            auctions,
            gaps,
            first_release,
            last_release,
            sha256: Sha256::digest(text.as_bytes()).into(),
        })
    }

    /// The rate that applies on `day`: the latest release strictly before it.
    ///
    /// # Errors
    ///
    /// [`LookupError::BeforeSeries`] and [`LookupError::Stale`], by name.
    ///
    /// # Cost
    ///
    /// O(1): one subtraction and one bounds-checked read of the dense day
    /// table. Measured flat at 1×, 10× and 100× the table by
    /// `pull::bench::rate_lookup_is_flat` (C-RF-01).
    pub fn on(&self, day: TradeDay) -> Result<AsOf, LookupError> {
        let offset = day.ordinal() - self.first;
        if offset < 0 {
            return Err(LookupError::BeforeSeries {
                day,
                first_release: self.first_release(),
            });
        }
        let slot = usize::try_from(offset)
            .ok()
            .and_then(|at| self.by_day.get(at))
            .copied()
            .unwrap_or(0);
        let (auction, rate) = slot
            .checked_sub(1)
            .and_then(|index| self.auctions.get(usize::try_from(index).ok()?))
            .copied()
            .ok_or(LookupError::Stale { day })?;
        let carried = usize::try_from(offset)
            .ok()
            .and_then(|at| self.carried_by.get(at))
            .and_then(|slot| slot.checked_sub(1))
            .and_then(|index| self.gaps.get(usize::try_from(index).ok()?))
            .copied();
        if let Some(gap) = carried
            && day.ordinal() - auction.released.ordinal() > MAX_CARRIED_AGE_DAYS
        {
            return Err(LookupError::CarriedTooLong {
                day,
                result: auction.released,
                gap,
            });
        }
        Ok(AsOf {
            rate,
            auction,
            carried,
        })
    }

    /// The first release in the table.
    #[must_use]
    pub const fn first_release(&self) -> TradeDay {
        self.first_release
    }

    /// The last release in the table.
    #[must_use]
    pub const fn last_release(&self) -> TradeDay {
        self.last_release
    }

    /// How many days the dense table covers: from the day after the first
    /// release to [`MAX_AGE_DAYS`] after the last. Every day outside it is
    /// refused without a read.
    #[must_use]
    pub fn days(&self) -> usize {
        self.by_day.len()
    }

    /// How many auctions the table holds. Never zero: a table with no
    /// auction is refused at load ([`SeriesError::Empty`]).
    #[must_use]
    pub fn auction_count(&self) -> usize {
        self.auctions.len()
    }

    /// How many documented gap rows the table holds.
    #[must_use]
    pub fn gap_count(&self) -> usize {
        self.gaps.len()
    }

    /// SHA-256 of the table's text, so a priced row can name its table.
    #[must_use]
    pub const fn sha256(&self) -> [u8; 32] {
        self.sha256
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_arithmetic,
    reason = "a test that cannot panic cannot fail; the float arithmetic is \
              the independent recomputation of a rate, not a price"
)]
mod tests {
    use super::*;

    // EVERY RATE BELOW IS INVENTED. The real series is the operator's, kept
    // outside the tree (docs/00-charter.md §4h). Each invented price/yield pair
    // is internally consistent under (100 − P)/P × 365/91, because the parser
    // checks that.
    const URL: &str = "https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=1";

    fn line(released: &str, price: &str, yield_pct: &str) -> String {
        format!("\t\t{released}\t{price}\t{yield_pct}\t{URL}\t\t")
    }

    fn table(rows: &[(&str, &str, &str)]) -> String {
        let mut text = String::from(HEADER);
        for (released, price, yield_pct) in rows {
            text.push('\n');
            text.push_str(&line(released, price, yield_pct));
        }
        text.push('\n');
        text
    }

    fn three() -> String {
        table(&[
            ("2020-01-01", "98.5000", "6.1081"),
            ("2020-01-08", "98.4000", "6.5219"),
            ("2020-01-15", "98.6000", "5.6951"),
        ])
    }

    fn day(y: u16, m: u8, d: u8) -> TradeDay {
        TradeDay::new(y, m, d).expect("a real day")
    }

    fn series(text: &str) -> Series {
        Series::parse(text).expect("a valid invented series")
    }

    fn refusal(text: &str) -> SeriesError {
        Series::parse(text).expect_err("a refused series")
    }

    /// The refusal of a one-row table whose row is `row_text`.
    fn row_fault(row_text: &str) -> SeriesError {
        refusal(&format!("{HEADER}\n{row_text}\n"))
    }

    /// The refusal [`row_fault`] expects: line 2, for this fault.
    fn at2(fault: RowFault) -> SeriesError {
        SeriesError::Row { line: 2, fault }
    }

    #[test]
    fn a_day_uses_the_latest_release_strictly_before_it() {
        let s = series(&three());
        // The release day itself does not see its own result: it is
        // announced after bidding closes, mid-session.
        let on_release = s.on(day(2020, 1, 8)).expect("a rate");
        assert_eq!(on_release.auction.released, day(2020, 1, 1));
        assert_eq!(on_release.auction.yield_e4, 61_081);
        assert_eq!(on_release.auction.price_e4, 985_000);
        let next = s.on(day(2020, 1, 9)).expect("a rate");
        assert_eq!(next.auction.released, day(2020, 1, 8));
        assert_eq!(next.auction.yield_e4, 65_219);
        assert_eq!(
            s.on(day(2020, 1, 16)).expect("a rate").auction.released,
            day(2020, 1, 15)
        );
    }

    #[test]
    fn a_day_on_or_before_the_first_release_is_refused_by_name() {
        let s = series(&three());
        for asked in [day(2020, 1, 1), day(2019, 12, 31), TradeDay::MIN] {
            assert_eq!(
                s.on(asked),
                Err(LookupError::BeforeSeries {
                    day: asked,
                    first_release: day(2020, 1, 1),
                })
            );
        }
        assert_eq!(
            s.on(day(2020, 1, 2))
                .expect("the first answerable day")
                .auction
                .released,
            day(2020, 1, 1)
        );
    }

    #[test]
    fn a_release_older_than_the_bound_is_stale_and_never_carried_forward() {
        let s = series(&three());
        // Fourteen days after the last release still answers; fifteen does not.
        assert_eq!(
            s.on(day(2020, 1, 29)).expect("age 14").auction.released,
            day(2020, 1, 15)
        );
        assert_eq!(
            s.on(day(2020, 1, 30)),
            Err(LookupError::Stale {
                day: day(2020, 1, 30)
            })
        );
        assert_eq!(
            s.on(TradeDay::MAX),
            Err(LookupError::Stale { day: TradeDay::MAX })
        );
        // A missing week INSIDE the series is refused, not bridged.
        let gap = series(&table(&[
            ("2020-01-01", "98.5000", "6.1081"),
            ("2020-01-20", "98.4000", "6.5219"),
        ]));
        // 2020-01-02 ..= 2020-02-03, the gap's zero slots included.
        assert_eq!(gap.days(), 33);
        assert_eq!(
            gap.on(day(2020, 1, 15)).expect("age 14").auction.released,
            day(2020, 1, 1)
        );
        for asked in [day(2020, 1, 16), day(2020, 1, 20)] {
            assert_eq!(gap.on(asked), Err(LookupError::Stale { day: asked }));
        }
        assert_eq!(
            gap.on(day(2020, 1, 21))
                .expect("the next release")
                .auction
                .released,
            day(2020, 1, 20)
        );
    }

    /// The dense table against a brute-force scan, on every day around the
    /// series: the answer is always the latest release STRICTLY before the
    /// day, and never one released on or after it.
    #[test]
    fn every_day_agrees_with_a_scan_and_none_sees_its_own_or_a_later_release() {
        let rows = [
            ("2020-01-01", "98.5000", "6.1081"),
            ("2020-01-07", "98.4000", "6.5219"),
            ("2020-01-15", "98.6000", "5.6951"),
            ("2020-02-05", "98.7500", "5.0772"),
            ("2020-02-12", "98.0000", "8.1857"),
        ];
        let s = series(&table(&rows));
        let releases: Vec<TradeDay> = rows
            .iter()
            .map(|(released, _, _)| date(released).expect("an invented release"))
            .collect();
        let start = day(2019, 12, 20);
        for offset in 0..90 {
            let asked = start.plus_days(offset).expect("a day");
            let scan = releases.iter().rev().find(|r| r.before(asked)).copied();
            let expected = match scan {
                None => Err("before the series"),
                Some(r) if asked.ordinal() - r.ordinal() > MAX_AGE_DAYS => Err("too old"),
                Some(r) => Ok(r),
            };
            let got = s
                .on(asked)
                .map(|a| a.auction.released)
                .map_err(|e| match e {
                    LookupError::BeforeSeries { .. } => "before the series",
                    LookupError::Stale { .. } => "too old",
                    // No gap row in this table, so nothing is carried.
                    LookupError::CarriedTooLong { .. } => "carried too long",
                });
            assert_eq!(got, expected, "on {asked}");
            if let Ok(released) = got {
                assert!(released.before(asked));
            }
        }
    }

    #[test]
    fn the_rate_is_the_continuous_form_of_the_published_simple_yield() {
        let s = series(&three());
        let rate = s.on(day(2020, 1, 2)).expect("a rate").rate;
        // r = (365/91)·ln(1 + y·91/365), y = 6.1081 % — Hull ch. 4.
        let expected = (365.0 / 91.0) * (1.0_f64 + 0.061_081 * 91.0 / 365.0).ln();
        assert!((rate.annual() - expected).abs() < 1e-15);
        // And independently, from the price the yield came from: the two
        // differ only by the yield's fourth-decimal rounding.
        let from_price = (365.0 / 91.0) * (100.0_f64 / 98.5).ln();
        assert!((rate.annual() - from_price).abs() < 1e-6);
        // Continuous is BELOW the simple yield it came from, never equal.
        assert!(rate.annual() < 0.061_081);
        assert!(rate.annual() > 0.060);
        assert_eq!(rate.basis(), YearBasis::Calendar365);
        assert_eq!(rate.source(), RateSource::Charter(CHARTER_CITE));
    }

    #[test]
    fn the_published_yield_must_agree_with_its_price_to_one_unit() {
        // 98.5000 implies 6.1081 exactly (6.10810509… rounded).
        for (published, accepted) in [
            ("6.1081", true),
            ("6.1080", true),
            ("6.1082", true),
            ("6.1079", false),
            ("6.1083", false),
        ] {
            let text = table(&[("2020-01-01", "98.5000", published)]);
            let want = (!accepted).then(|| {
                at2(RowFault::YieldDisagreesWithPrice {
                    published: decimal_e4(published).expect("a decimal"),
                    implied: 61_081,
                })
            });
            assert_eq!(Series::parse(&text).err(), want, "{published}");
        }
        // Rounded HALF UP: 98.0000 implies 8.18569185… -> 8.1857.
        assert_eq!(implied_yield_e4(980_000), 81_857);
        // A yield a 182-day bill's price would carry is refused, which is what
        // keeps a column transposed from another tenor out.
        assert_eq!(
            row_fault(&line("2020-01-01", "97.0000", "6.1081")),
            at2(RowFault::YieldDisagreesWithPrice {
                published: 61_081,
                implied: 124_051
            })
        );
        // The extreme price below par: no overflow.
        assert_eq!(implied_yield_e4(1), 4_010_985_000_000);
        assert_eq!(implied_yield_e4(999_999), 4);
    }

    #[test]
    fn a_header_or_an_empty_body_is_refused() {
        assert_eq!(refusal(""), SeriesError::Header);
        assert_eq!(refusal("auction_date\treleased\n"), SeriesError::Header);
        assert_eq!(
            refusal(&three().replacen("released", "published", 1)),
            SeriesError::Header
        );
        assert_eq!(refusal(&format!("{HEADER}\n")), SeriesError::Empty);
        assert_eq!(refusal(HEADER), SeriesError::Empty);
    }

    #[test]
    fn every_malformed_field_is_refused_by_name() {
        assert_eq!(row_fault("a\tb"), at2(RowFault::FieldCount { found: 2 }));
        assert_eq!(
            row_fault(&format!(
                "{}\textra",
                line("2020-01-01", "98.5000", "6.1081")
            )),
            at2(RowFault::FieldCount { found: 9 })
        );
        for bad in [
            "2020-1-01",
            "20200-01-01",
            "2020-01-1",
            "20200101",
            "2020-01-01-01",
            "2020-02-30",
            "1989-12-31",
            "2020-01-xx",
            "2020-0a-01",
            "abcd-01-01",
            "",
        ] {
            assert_eq!(
                row_fault(&line(bad, "98.5000", "6.1081")),
                at2(RowFault::Date { field: "released" }),
                "{bad:?}"
            );
        }
        for bad in [
            "98.50001",
            "98,5",
            "-98.5",
            ".5",
            "",
            "x",
            "98.5x",
            "4294967.2960",
        ] {
            assert_eq!(
                row_fault(&line("2020-01-01", bad, "6.1081")),
                at2(RowFault::Decimal {
                    field: "cut_off_price"
                }),
                "{bad:?}"
            );
            assert_eq!(
                row_fault(&line("2020-01-01", "98.5000", bad)),
                at2(RowFault::Decimal {
                    field: "implicit_yield_pct"
                }),
                "{bad:?}"
            );
        }
        // Fewer than four places are the same value as four.
        let short = series(&table(&[("2020-01-01", "98.5", "6.1081")]));
        assert_eq!(
            short.on(day(2020, 1, 2)).expect("a rate").auction.price_e4,
            985_000
        );
        let whole = series(&table(&[("2020-01-01", "98.5000", "6.1081")]));
        assert_eq!(whole.auction_count(), 1);
        assert_eq!(decimal_e4("98"), Some(980_000));
        assert_eq!(decimal_e4("98.1234"), Some(981_234));
        assert_eq!(decimal_e4("0.0001"), Some(1));
        assert_eq!(decimal_e4("429496.7295"), Some(u32::MAX));
        assert_eq!(decimal_e4("429496.7296"), None);
    }

    #[test]
    fn a_price_at_par_or_zero_is_refused() {
        for (price, e4) in [("100.0000", 1_000_000), ("100.0001", 1_000_001), ("0", 0)] {
            assert_eq!(
                row_fault(&line("2020-01-01", price, "0")),
                at2(RowFault::PriceNotBelowPar { price_e4: e4 })
            );
        }
        // One ten-thousandth below par is a price.
        assert!(Series::parse(&table(&[("2020-01-01", "99.9999", "0.0004")])).is_ok());
    }

    #[test]
    fn a_source_that_is_not_an_rbi_page_is_refused() {
        let row_text =
            line("2020-01-01", "98.5000", "6.1081").replace(URL, "https://example.com/rbi.org.in/");
        assert_eq!(row_fault(&row_text), at2(RowFault::Source));
        let blank = line("2020-01-01", "98.5000", "6.1081").replace(URL, "");
        assert_eq!(row_fault(&blank), at2(RowFault::Source));
    }

    #[test]
    fn releases_must_strictly_ascend() {
        for second in ["2020-01-01", "2019-12-25"] {
            let text = table(&[
                ("2020-01-01", "98.5000", "6.1081"),
                (second, "98.4000", "6.5219"),
            ]);
            assert_eq!(
                refusal(&text),
                SeriesError::Row {
                    line: 3,
                    fault: RowFault::NotAfterPrevious
                }
            );
        }
        assert!(
            Series::parse(&table(&[
                ("2020-01-01", "98.5000", "6.1081"),
                ("2020-01-02", "98.4000", "6.5219"),
            ]))
            .is_ok()
        );
    }

    #[test]
    fn a_yield_whose_continuous_rate_is_implausible_is_refused_at_load() {
        // 50.0000 implies 401.0989 %, continuous 277.9 %: past the typo screen.
        let annual = continuous(4_010_989);
        assert!(annual > 1.0);
        assert_eq!(
            row_fault(&line("2020-01-01", "50.0000", "401.0989")),
            at2(RowFault::Rate(PricingError::RateImplausible { annual }))
        );
    }

    #[test]
    fn the_series_reports_its_extent_and_its_digest() {
        let text = three();
        let s = series(&text);
        assert_eq!(s.auction_count(), 3);
        // 2020-01-02 ..= 2020-01-29: the day after the first release to
        // fourteen days after the last.
        assert_eq!(s.days(), 28);
        assert_eq!(s.first_release(), day(2020, 1, 1));
        assert_eq!(s.last_release(), day(2020, 1, 15));
        let digest: [u8; 32] = Sha256::digest(text.as_bytes()).into();
        assert_eq!(s.sha256(), digest);
        let other = series(&text.replace("98.6000\t5.6951", "98.7500\t5.0772"));
        assert_ne!(other.sha256(), s.sha256());
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("brutex-riskfree-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir.join(name)
    }

    #[test]
    fn read_takes_the_path_it_is_given_and_refuses_what_it_cannot_use() {
        let good = scratch("good.tsv");
        std::fs::write(&good, three()).expect("written");
        let read = Series::read(&good).expect("a valid file");
        assert_eq!(read.sha256(), series(&three()).sha256());
        assert_eq!(read.auction_count(), 3);

        assert!(matches!(
            Series::read(&scratch("absent.tsv")),
            Err(SeriesError::Read { .. })
        ));

        let binary = scratch("binary.tsv");
        std::fs::write(&binary, [0xff_u8, 0xfe, 0x00]).expect("written");
        assert_eq!(Series::read(&binary).err(), Some(SeriesError::NotText));

        // Exactly the bound is read (and refused for its content); one byte
        // more is refused for its size before it is read.
        let at = scratch("at.tsv");
        std::fs::write(&at, vec![b'x'; 4 * 1024 * 1024]).expect("written");
        assert_eq!(Series::read(&at).err(), Some(SeriesError::Header));
        let over = scratch("over.tsv");
        std::fs::write(&over, vec![b'x'; 4 * 1024 * 1024 + 1]).expect("written");
        assert_eq!(
            Series::read(&over).err(),
            Some(SeriesError::TooLarge {
                bytes: MAX_FILE_BYTES + 1
            })
        );
        for path in [good, binary, at, over] {
            std::fs::remove_file(path).expect("removed");
        }
    }

    #[test]
    fn every_refusal_says_what_it_refused() {
        let cases: [(String, &str); 7] = [
            (SeriesError::Read { why: "gone".into() }.to_string(), "gone"),
            (SeriesError::TooLarge { bytes: 9 }.to_string(), "9 bytes"),
            (SeriesError::NotText.to_string(), "UTF-8"),
            (SeriesError::Header.to_string(), "§4h"),
            (SeriesError::Empty.to_string(), "no auction"),
            (
                SeriesError::Row {
                    line: 7,
                    fault: RowFault::Source,
                }
                .to_string(),
                "line 7: Source",
            ),
            (
                LookupError::BeforeSeries {
                    day: day(2018, 1, 1),
                    first_release: day(2018, 8, 1),
                }
                .to_string(),
                "before 2018-01-01; the series starts with the release of 2018-08-01",
            ),
        ];
        for (said, needle) in cases {
            assert!(said.contains(needle), "{said:?} lacks {needle:?}");
        }
        let stale = LookupError::Stale {
            day: day(2026, 10, 1),
        }
        .to_string();
        assert!(stale.contains("2026-10-01"));
        assert!(stale.contains("14 days"));
        let as_error: &dyn std::error::Error = &SeriesError::Empty;
        assert!(as_error.source().is_none());
        let as_error: &dyn std::error::Error = &LookupError::Stale { day: TradeDay::MIN };
        assert!(as_error.source().is_none());
    }

    #[test]
    fn a_rate_from_the_series_prices_through_the_existing_solver() {
        // The series produces the SAME `Rate` the pricing module already
        // takes; nothing downstream changes. An invented at-the-money quote.
        let rate = series(&three()).on(day(2020, 1, 9)).expect("a rate").rate;
        let contract = greeks::bsm::Contract {
            spot: 2_000_000.0,
            strike: 2_000_000.0,
            years_to_expiry: 30.0 / 365.0,
            rate: rate.annual(),
            carry: 0.0,
        };
        let priced = contract
            .greeks(0.15, greeks::bsm::OptionKind::Call)
            .expect("a priced call");
        let solved = contract
            .implied_volatility(priced.price, greeks::bsm::OptionKind::Call)
            .expect("solved");
        assert!((solved.volatility - 0.15).abs() < 1e-6);
    }

    // ─────────── DB-B44: documented gaps are carried, unexplained ones refused ───────────

    /// A gap row: an invented RBI page, `kind`'s column word, no yield.
    fn gap_line(released: &str, kind: GapKind) -> String {
        format!("\t\t{released}\t{}\t\t{URL}\t\t", kind.column())
    }

    /// Three weekly results, then a three-week hole, then one result; the
    /// hole is documented by `gaps` (released days, in order) when given.
    fn holed(gaps: &[(&str, GapKind)]) -> String {
        let mut text = String::from(HEADER);
        for (released, price, yield_pct) in [
            ("2020-01-01", "98.5000", "6.1081"),
            ("2020-01-08", "98.4000", "6.5219"),
        ] {
            text.push('\n');
            text.push_str(&line(released, price, yield_pct));
        }
        for (released, kind) in gaps {
            text.push('\n');
            text.push_str(&gap_line(released, *kind));
        }
        text.push('\n');
        text.push_str(&line("2020-02-05", "98.6000", "5.6951"));
        text.push('\n');
        text
    }

    #[test]
    fn a_documented_rate_cancellation_is_carried_and_labelled() {
        let documented = series(&holed(&[
            ("2020-01-15", GapKind::Cancelled),
            ("2020-01-22", GapKind::NotAvailable),
            ("2020-01-29", GapKind::NotHeld),
        ]));
        assert_eq!(documented.gap_count(), 3);
        assert_eq!(documented.auction_count(), 3);
        // Before the first gap is known (its own release day), the result is
        // fresh and unlabelled.
        let fresh = documented.on(day(2020, 1, 15)).expect("fresh");
        assert_eq!(fresh.carried, None);
        assert_eq!(fresh.state(), "rate_fresh");
        assert_eq!(fresh.auction.released, day(2020, 1, 8));
        // From the day after each gap, the 8 Jan rate is carried, NAMING the
        // gap, up to day 28 after it -- past MAX_AGE_DAYS, which the bare rule
        // refused.
        for (asked, gap_day, kind) in [
            (day(2020, 1, 16), day(2020, 1, 15), GapKind::Cancelled),
            (day(2020, 1, 23), day(2020, 1, 22), GapKind::NotAvailable),
            (day(2020, 1, 29), day(2020, 1, 22), GapKind::NotAvailable),
            (day(2020, 1, 31), day(2020, 1, 29), GapKind::NotHeld),
            (day(2020, 2, 5), day(2020, 1, 29), GapKind::NotHeld),
        ] {
            let carried = documented.on(asked).expect("carried over a documented gap");
            assert_eq!(carried.state(), "rate_carried", "{asked}");
            assert_eq!(carried.auction.released, day(2020, 1, 8), "{asked}");
            assert_eq!(
                carried.carried,
                Some(Gap {
                    released: gap_day,
                    kind
                }),
                "{asked}"
            );
            assert_eq!(
                carried.rate,
                documented.on(day(2020, 1, 10)).expect("fresh").rate,
                "the carried rate is the last result's, not a new value"
            );
        }
        // The next result ends the carry.
        let after = documented.on(day(2020, 2, 6)).expect("fresh again");
        assert_eq!(after.carried, None);
        assert_eq!(after.auction.released, day(2020, 2, 5));
    }

    #[test]
    fn an_unexplained_rate_gap_is_still_refused() {
        // The same hole with NO gap row: day 15 after the 8 Jan result is
        // refused exactly as before.
        let bare = series(&holed(&[]));
        assert_eq!(bare.gap_count(), 0);
        assert!(bare.on(day(2020, 1, 22)).is_ok(), "day 14 is fresh");
        assert_eq!(
            bare.on(day(2020, 1, 23)),
            Err(LookupError::Stale {
                day: day(2020, 1, 23)
            })
        );
        // A documented gap explains only its own fourteen days: one gap row
        // on 15 Jan carries through 29 Jan and the rest of the hole is still
        // refused, because nobody explained it.
        let partly = series(&holed(&[("2020-01-15", GapKind::Cancelled)]));
        assert_eq!(
            partly.on(day(2020, 1, 29)).map(|as_of| as_of.state()),
            Ok("rate_carried")
        );
        assert_eq!(
            partly.on(day(2020, 1, 30)),
            Err(LookupError::Stale {
                day: day(2020, 1, 30)
            })
        );
        assert!(
            LookupError::Stale {
                day: day(2020, 1, 30)
            }
            .to_string()
            .contains("no documented gap row explains why")
        );
        // The last row a gap: its fourteen days are carried, the fifteenth is
        // refused like the end of any table.
        let mut tail = holed(&[]);
        tail.push_str(&gap_line("2020-02-12", GapKind::Cancelled));
        tail.push('\n');
        let tail = series(&tail);
        assert_eq!(tail.last_release(), day(2020, 2, 12));
        assert_eq!(
            tail.on(day(2020, 2, 26)).map(|as_of| as_of.state()),
            Ok("rate_carried")
        );
        assert_eq!(
            tail.on(day(2020, 2, 27)),
            Err(LookupError::Stale {
                day: day(2020, 2, 27)
            })
        );
    }

    /// A weekly table of `weeks` rows from 1990-01-03 where every third week
    /// is a documented `CANCELLED` row instead of a result.
    fn weekly_holed(weeks: i32) -> Series {
        let start = day(1990, 1, 3);
        let mut text = String::from(HEADER);
        for week in 0..weeks {
            let released = start.plus_days(week * 7).expect("before 2100").to_string();
            text.push('\n');
            if week % 3 == 2 {
                text.push_str(&gap_line(&released, GapKind::Cancelled));
            } else {
                text.push_str(&line(&released, "98.5000", "6.1081"));
            }
        }
        series(&text)
    }

    /// p50, p99 and max nanoseconds per lookup over 200 batches of `days`.
    fn lookup_tail(table: &Series, days: &[TradeDay]) -> (u128, u128, u128) {
        let mut per_call: Vec<u128> = (0..200)
            .map(|_| {
                let start = std::time::Instant::now();
                for asked in days {
                    std::hint::black_box(table.on(*asked).ok());
                }
                start.elapsed().as_nanos() / u128::try_from(days.len().max(1)).unwrap_or(1)
            })
            .collect();
        per_call.sort_unstable();
        let at = |rank: usize| per_call.get(rank).copied().unwrap_or(u128::MAX);
        (at(100), at(198), at(199))
    }

    /// O(1) AT p99 WITH THE GAP RULE (R3-30 repair 1): a lookup that carries
    /// a rate over a documented gap reads one more bounded slot, never a walk,
    /// so its tail is flat from 52 to 5,200 rows. Printed with n and method.
    #[test]
    fn a_carried_lookup_is_flat_in_the_table() {
        let mut tails = Vec::new();
        for weeks in [52, 5_200] {
            let table = weekly_holed(weeks);
            let first = table.first_release().ordinal() + 1;
            let span = table.last_release().ordinal() - first + 1;
            let days: Vec<TradeDay> = (0..1_024)
                .filter_map(|i| TradeDay::from_ordinal(first + span * i / 1_024).ok())
                .collect();
            let carried = days
                .iter()
                .filter(|asked| {
                    table
                        .on(**asked)
                        .is_ok_and(|as_of| as_of.state() == "rate_carried")
                })
                .count();
            assert!(carried > 0, "the sample carries over gaps at {weeks} rows");
            assert_eq!(table.gap_count(), usize::try_from(weeks / 3).unwrap());
            let tail = lookup_tail(&table, &days);
            eprintln!(
                "riskfree::Series::on, {weeks} rows ({} gaps), {} days x 200 batches, \
                 {carried} carried: p50 {} ns p99 {} ns max {} ns per lookup",
                table.gap_count(),
                days.len(),
                tail.0,
                tail.1,
                tail.2
            );
            tails.push(tail);
        }
        assert!(
            matches!(tails.as_slice(), [small, big] if big.1 <= small.1.saturating_mul(10).max(2_000)),
            "a table 100 times larger stays within ten times the p99: {tails:?}"
        );
    }

    #[test]
    fn chained_documented_gaps_carry_a_rate_no_older_than_two_windows() {
        // Four weekly gap rows after the 8 Jan result, then a result on
        // 12 Feb. Each gap restarts the staleness clock, so without a bound on
        // the CARRIED result the 8 Jan rate would price every day to 12 Feb.
        let mut text = String::from(HEADER);
        for (released, price, yield_pct) in [
            ("2020-01-01", "98.5000", "6.1081"),
            ("2020-01-08", "98.4000", "6.5219"),
        ] {
            text.push('\n');
            text.push_str(&line(released, price, yield_pct));
        }
        for released in ["2020-01-15", "2020-01-22", "2020-01-29", "2020-02-05"] {
            text.push('\n');
            text.push_str(&gap_line(released, GapKind::Cancelled));
        }
        text.push('\n');
        text.push_str(&line("2020-02-12", "98.6000", "5.6951"));
        text.push('\n');
        let chained = series(&text);
        assert_eq!(chained.gap_count(), 4);
        // Day 28 after the carried release: still carried, still labelled.
        let edge = chained.on(day(2020, 2, 5)).expect("day 28 is carried");
        assert_eq!(edge.state(), "rate_carried");
        assert_eq!(edge.auction.released, day(2020, 1, 8));
        // Day 29, inside the fourth gap's own fourteen days: refused by name,
        // naming the gap and the result it would have carried.
        let gap = Gap {
            released: day(2020, 2, 5),
            kind: GapKind::Cancelled,
        };
        for asked in [day(2020, 2, 6), day(2020, 2, 12)] {
            assert_eq!(
                chained.on(asked),
                Err(LookupError::CarriedTooLong {
                    day: asked,
                    result: day(2020, 1, 8),
                    gap,
                }),
                "{asked}"
            );
        }
        let said = chained
            .on(day(2020, 2, 6))
            .expect_err("refused")
            .to_string();
        assert!(said.contains("2020-01-08"), "{said}");
        assert!(said.contains("cancelled"), "{said}");
        assert!(said.contains(&MAX_CARRIED_AGE_DAYS.to_string()), "{said}");
        // The next result is fresh again from the day after its release.
        assert_eq!(
            chained.on(day(2020, 2, 13)).map(|as_of| as_of.state()),
            Ok("rate_fresh")
        );
    }

    #[test]
    fn a_malformed_gap_row_is_refused_by_name() {
        // A gap row stating a yield.
        assert_eq!(
            row_fault(&format!("\t\t2020-01-01\tCANCELLED\t6.1081\t{URL}\t\t")),
            at2(RowFault::GapWithYield)
        );
        // A gap row with no RBI page.
        assert_eq!(
            row_fault("\t\t2020-01-01\tNA\t\thttps://example.invalid/\t\t"),
            at2(RowFault::Source)
        );
        // A gap row with a broken date.
        assert_eq!(
            row_fault(&format!("\t\t2020-13-01\tNA\t\t{URL}\t\t")),
            at2(RowFault::Date { field: "released" })
        );
        // A gap before any result: nothing to carry.
        assert_eq!(
            refusal(&format!(
                "{HEADER}\n{}\n{}\n",
                gap_line("2020-01-01", GapKind::Cancelled),
                line("2020-01-08", "98.4000", "6.5219")
            )),
            at2(RowFault::GapBeforeAnyResult)
        );
        // A gap on the same day as the result before it, and out of order.
        let mut same_day = String::from(HEADER);
        same_day.push('\n');
        same_day.push_str(&line("2020-01-08", "98.4000", "6.5219"));
        same_day.push('\n');
        same_day.push_str(&gap_line("2020-01-08", GapKind::NotHeld));
        same_day.push('\n');
        assert_eq!(
            refusal(&same_day),
            SeriesError::Row {
                line: 3,
                fault: RowFault::NotAfterPrevious
            }
        );
        // A lower-case word is not a gap word: it is read as a price and
        // refused as one.
        assert_eq!(
            row_fault(&format!("\t\t2020-01-01\tcancelled\t\t{URL}\t\t")),
            at2(RowFault::Decimal {
                field: "cut_off_price"
            })
        );
        for kind in GapKind::ALL {
            assert_eq!(GapKind::of_column(kind.column()), Some(kind));
            assert!(!kind.word().is_empty());
        }
    }
}
