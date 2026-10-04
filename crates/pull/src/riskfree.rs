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

/// The rate that applies on one day, and the auction it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AsOf {
    /// Continuously compounded, `Calendar365`, cited as [`CHARTER_CITE`].
    pub rate: Rate,
    /// The auction whose release is the latest strictly before the day asked.
    pub auction: Auction,
}

/// The loaded series, indexed by day.
#[derive(Debug, Clone)]
pub struct Series {
    /// The first day a lookup can answer: the day after the first release.
    first: i32,
    /// One slot per day from `first`: the index of the applicable auction plus
    /// one, or zero where the latest release is older than [`MAX_AGE_DAYS`].
    by_day: Vec<u32>,
    auctions: Vec<(Auction, Rate)>,
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
                 days old or absent, so an auction is missing from the series; \
                 no rate is carried forward over the gap"
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

/// One data row, checked.
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
        let mut auctions: Vec<(Auction, Rate)> = Vec::new();
        for (offset, line) in lines.enumerate() {
            let previous = auctions.last().map(|(a, _)| a.released);
            let parsed = row(line, previous).map_err(|fault| SeriesError::Row {
                line: offset + 2,
                fault,
            })?;
            auctions.push(parsed);
        }
        let (Some((head, _)), Some((tail, _))) = (auctions.first(), auctions.last()) else {
            return Err(SeriesError::Empty);
        };
        let (first_release, last_release) = (head.released, tail.released);
        let first = first_release.ordinal() + 1;
        let end = last_release.ordinal() + MAX_AGE_DAYS;
        let mut by_day = Vec::new();
        let mut current = 0_usize;
        for ordinal in first..=end {
            while auctions
                .get(current + 1)
                .is_some_and(|(next, _)| next.released.ordinal() < ordinal)
            {
                current += 1;
            }
            let fresh = auctions
                .get(current)
                .is_some_and(|(a, _)| ordinal - a.released.ordinal() <= MAX_AGE_DAYS);
            // `current + 1` fits: there is at most one release per day of a
            // window of about 40,560 days.
            by_day.push(if fresh {
                u32::try_from(current + 1).unwrap_or(u32::MAX)
            } else {
                0
            });
        }
        Ok(Self {
            first,
            by_day,
            auctions,
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
        Ok(AsOf { rate, auction })
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

    /// How many auctions the table holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.auctions.len()
    }

    /// Always false: a table with no auction is refused at load.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.auctions.is_empty()
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
        assert_eq!(whole.len(), 1);
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
        assert_eq!(s.len(), 3);
        assert!(!s.is_empty());
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
        assert_eq!(read.len(), 3);

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
}
