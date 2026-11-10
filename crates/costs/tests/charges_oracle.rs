#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test \
              that cannot panic cannot fail"
)]

//! CHARGES ORACLE — an independent, test-only re-derivation of every charge
//! on an NSE intraday trade, from official sources only, compared with this
//! crate's own charge code.
//!
//! The rates are not read from this crate. They come from
//! `tests/fixtures/charges_oracle_rates.toml`, the machine-readable twin of
//! `docs/charges-oracle/RATES.md`: one dated row per rate from 2018-09-01 to
//! 2026-10-31, each with its official URL and the exact quote it rests on.
//! The arithmetic here was written from those sources, not from this crate's
//! modules; the crate is driven only through its public API.
//!
//! Three segments: equity cash intraday, stock options, index options.
//!
//! * [`order_charges`] — one executed order on its own: every line, exactly.
//! * [`day_charges`] — one client's trades in one security on one day, the
//!   way a contract note bills them, exactly and rounded.
//! * [`exercise_stt`] — an index option exercised at expiry.
//!
//! What runs by default (green) is the oracle's own consistency and every
//! place the crate and the oracle agree. Every disagreement is an
//! `#[ignore]`d test named `finding_*`, which fails when run and prints the
//! mismatch in its message:
//!
//! ```text
//! cargo test -p costs --test charges_oracle -- --ignored
//! ```
//!
//! `CHARGES-ORACLE-REPORT.md` at the repository root is the account of what
//! those findings say.

use std::collections::BTreeSet;
use std::fmt;
use std::fmt::Write as _;

// ---------------------------------------------------------------------------
// Dates: a civil date as a day number, so ranges compare as integers.
// ---------------------------------------------------------------------------

/// A calendar date, carried as its day number counted from 1970-01-01.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Date(i64);

impl Date {
    /// The date `year-month-day`, or `None` when no such day exists.
    fn ymd(year: i64, month: i64, day: i64) -> Option<Self> {
        if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
            return None;
        }
        // Days from civil (Howard Hinnant's algorithm), proleptic Gregorian.
        let shifted = if month <= 2 { year - 1 } else { year };
        let era = shifted.div_euclid(400);
        let yoe = shifted - era * 400;
        let mp = (month + 9) % 12;
        let doy = (153 * mp + 2) / 5 + day - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        Some(Self(era * 146_097 + doe - 719_468))
    }

    /// `YYYY-MM-DD`, strictly.
    fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        let num = |range: std::ops::Range<usize>| -> Option<i64> {
            let part = text.get(range)?;
            if part.bytes().all(|b| b.is_ascii_digit()) {
                part.parse().ok()
            } else {
                None
            }
        };
        Self::ymd(num(0..4)?, num(5..7)?, num(8..10)?)
    }

    /// The year, month and day.
    fn civil(self) -> (i64, i64, i64) {
        let shifted = self.0 + 719_468;
        let era = shifted.div_euclid(146_097);
        let doe = shifted - era * 146_097;
        let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = doy - (153 * mp + 2) / 5 + 1;
        let month = if mp < 10 { mp + 3 } else { mp - 9 };
        let year = yoe + era * 400 + i64::from(month <= 2);
        (year, month, day)
    }

    /// The next calendar day.
    const fn next(self) -> Self {
        Self(self.0 + 1)
    }

    /// The previous calendar day.
    const fn prev(self) -> Self {
        Self(self.0 - 1)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (y, m, d) = self.civil();
        write!(f, "{y:04}-{m:02}-{d:02}")
    }
}

/// Days in `month` of `year`, Gregorian.
fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        _ => 28,
    }
}

/// A date the test writes as a literal.
fn date(text: &str) -> Date {
    Date::parse(text).expect("a literal date")
}

// ---------------------------------------------------------------------------
// Exact rationals, in paisa.
// ---------------------------------------------------------------------------

/// An exact rational `n / d` with `d > 0`, kept in lowest terms.
#[derive(Debug, Clone, Copy)]
struct Q {
    n: i128,
    d: i128,
}

/// Greatest common divisor of two non-negative integers.
const fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Q {
    const ZERO: Self = Self { n: 0, d: 1 };

    fn new(n: i128, d: i128) -> Self {
        assert!(d != 0, "a zero denominator");
        let (n, d) = if d < 0 { (-n, -d) } else { (n, d) };
        let g = gcd(n.abs(), d).max(1);
        Self { n: n / g, d: d / g }
    }

    const fn int(n: i128) -> Self {
        Self { n, d: 1 }
    }

    fn plus(self, other: Self) -> Self {
        Self::new(self.n * other.d + other.n * self.d, self.d * other.d)
    }

    fn minus(self, other: Self) -> Self {
        Self::new(self.n * other.d - other.n * self.d, self.d * other.d)
    }

    fn times(self, other: Self) -> Self {
        // Cross-reduce first so the products stay small.
        let g1 = gcd(self.n.abs(), other.d).max(1);
        let g2 = gcd(other.n.abs(), self.d).max(1);
        Self::new(
            (self.n / g1) * (other.n / g2),
            (self.d / g2) * (other.d / g1),
        )
    }

    fn lesser(self, other: Self) -> Self {
        if self.n * other.d <= other.n * self.d {
            self
        } else {
            other
        }
    }

    const fn is_zero(self) -> bool {
        self.n == 0
    }

    const fn same_as(self, other: Self) -> bool {
        self.n * other.d == other.n * self.d
    }

    fn magnitude(self) -> Self {
        Self::new(self.n.abs(), self.d)
    }

    /// Rounded to a multiple of `quantum` paisa, an exact half going up.
    fn half_up(self, quantum: i128) -> i128 {
        // floor((2n + d·q) / (2·d·q)) · q
        let num = 2 * self.n + self.d * quantum;
        let den = 2 * self.d * quantum;
        num.div_euclid(den) * quantum
    }
}

impl fmt::Display for Q {
    /// Paisa as rupees, exact to nine decimals of a rupee when the value has
    /// them, else as a fraction of a paisa.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scale: i128 = 1_000_000_000;
        if (self.n * scale) % self.d == 0 {
            let units = self.n * scale / self.d; // 10^-9 paisa
            let sign = if units < 0 { "-" } else { "" };
            let units = units.abs();
            let rupees = units / (100 * scale);
            let rest = units % (100 * scale);
            let mut frac = format!("{rest:011}");
            while frac.ends_with('0') && frac.len() > 2 {
                frac.pop();
            }
            write!(f, "{sign}Rs {rupees}.{frac}")
        } else {
            write!(f, "{}/{} paisa", self.n, self.d)
        }
    }
}

/// Paisa as rupees with two decimals.
fn rupees(paisa: i128) -> String {
    let sign = if paisa < 0 { "-" } else { "" };
    let magnitude = paisa.abs();
    format!("{sign}Rs {}.{:02}", magnitude / 100, magnitude % 100)
}

// ---------------------------------------------------------------------------
// The fixture: a strict subset of TOML, parsed here so no dependency is added.
// ---------------------------------------------------------------------------

/// The machine-readable rate table, the twin of `docs/charges-oracle/RATES.md`.
const FIXTURE: &str = include_str!("fixtures/charges_oracle_rates.toml");

/// The human-readable rate table.
const RATES_MD: &str = include_str!("../../../docs/charges-oracle/RATES.md");

/// One value in the fixture: a basic string or an integer.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Str(String),
    Int(i128),
}

/// The fixture as parsed: the top-level keys and one key list per `[[rate]]`.
#[derive(Debug, Default)]
struct Parsed {
    header: Vec<(String, Value)>,
    rows: Vec<Vec<(String, Value)>>,
}

/// Parses the fixture's TOML subset: comments, blank lines, `[[rate]]`
/// headers, and `key = "string"` or `key = integer` lines. Anything else is
/// refused with its line number, so the file cannot say more than this
/// reader understands.
fn parse_fixture(text: &str) -> Result<Parsed, String> {
    let mut parsed = Parsed::default();
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line == "[[rate]]" {
            parsed.rows.push(Vec::new());
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {line_no}: not `key = value`: {line}"))?;
        let key = key.trim();
        if key.is_empty()
            || !key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return Err(format!("line {line_no}: bad key {key:?}"));
        }
        let value = parse_value(value.trim()).map_err(|why| format!("line {line_no}: {why}"))?;
        let target = match parsed.rows.last_mut() {
            Some(row) => row,
            None => &mut parsed.header,
        };
        if target.iter().any(|(k, _)| k == key) {
            return Err(format!("line {line_no}: duplicate key {key}"));
        }
        target.push((key.to_owned(), value));
    }
    Ok(parsed)
}

/// A basic string with `\"` and `\\` escapes only, or a decimal integer that
/// may carry `_` separators.
fn parse_value(text: &str) -> Result<Value, String> {
    if let Some(body) = text.strip_prefix('"') {
        let mut out = String::new();
        let mut chars = body.chars();
        loop {
            match chars.next() {
                None => return Err("an unterminated string".to_owned()),
                Some('"') => break,
                Some('\\') => match chars.next() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    other => return Err(format!("an unsupported escape {other:?}")),
                },
                Some(c) => out.push(c),
            }
        }
        if !chars.as_str().trim().is_empty() {
            return Err(format!("text after a string: {:?}", chars.as_str()));
        }
        return Ok(Value::Str(out));
    }
    let digits: String = text.chars().filter(|&c| c != '_').collect();
    let unsigned = digits.strip_prefix('-').unwrap_or(&digits);
    if unsigned.is_empty() || !unsigned.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("not a string or an integer: {text}"));
    }
    digits
        .parse()
        .map(Value::Int)
        .map_err(|e| format!("integer {text}: {e}"))
}

/// The value of `key` in one key list.
fn field<'a>(row: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    row.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// The string value of `key`.
fn text_field(row: &[(String, Value)], key: &str) -> Result<String, String> {
    match field(row, key) {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(Value::Int(_)) => Err(format!("{key} must be a string")),
        None => Err(format!("missing {key}")),
    }
}

/// The integer value of `key`.
fn int_field(row: &[(String, Value)], key: &str) -> Result<i128, String> {
    match field(row, key) {
        Some(Value::Int(n)) => Ok(*n),
        Some(Value::Str(_)) => Err(format!("{key} must be an integer")),
        None => Err(format!("missing {key}")),
    }
}

// ---------------------------------------------------------------------------
// The rate table the fixture describes.
// ---------------------------------------------------------------------------

/// The three segments this oracle prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Segment {
    /// Equity cash, bought and sold the same day (non-delivery).
    EquityIntraday,
    /// Options on a single stock.
    StockOption,
    /// Options on an index.
    IndexOption,
}

impl Segment {
    const ALL: [Self; 3] = [Self::EquityIntraday, Self::StockOption, Self::IndexOption];

    fn parse(text: &str) -> Option<Self> {
        match text {
            "equity_intraday" => Some(Self::EquityIntraday),
            "stock_option" => Some(Self::StockOption),
            "index_option" => Some(Self::IndexOption),
            _ => None,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::EquityIntraday => "equity_intraday",
            Self::StockOption => "stock_option",
            Self::IndexOption => "index_option",
        }
    }

    const fn is_option(self) -> bool {
        matches!(self, Self::StockOption | Self::IndexOption)
    }
}

/// One side of an executed order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Side {
    Buy,
    Sell,
}

/// Which orders a row's rate applies to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Applies {
    /// The buy side only.
    Buy,
    /// The sell side only.
    Sell,
    /// Every executed order, either side.
    Both,
    /// An option exercised at expiry (the holder's side).
    Exercise,
}

impl Applies {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "buy" => Some(Self::Buy),
            "sell" => Some(Self::Sell),
            "both" => Some(Self::Both),
            "exercise" => Some(Self::Exercise),
            _ => None,
        }
    }

    const fn covers(self, side: Side) -> bool {
        matches!(
            (self, side),
            (Self::Both, _) | (Self::Buy, Side::Buy) | (Self::Sell, Side::Sell)
        )
    }
}

/// What a row prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Charge {
    /// Broker: a fraction of the order value, per executed order.
    BrokerageRate,
    /// Broker: the cap on [`Charge::BrokerageRate`], in paisa per order.
    BrokerageCap,
    /// Broker: a fixed amount per executed order, in paisa.
    BrokeragePerOrder,
    /// Exchange transaction charge, each side.
    ExchangeTxn,
    /// Investor Protection Fund Trust contribution, each side.
    Ipft,
    /// Clearing charge, each side.
    Clearing,
    /// SEBI turnover fee, each side.
    SebiFee,
    /// Securities transaction tax on a sale.
    Stt,
    /// Securities transaction tax on an exercised option.
    SttExercise,
    /// Stamp duty, buy side.
    Stamp,
    /// Goods and services tax on the fee base.
    Gst,
    /// 1 when the SEBI fee is part of the GST base, 0 when it is not.
    GstOnSebiFee,
}

impl Charge {
    const ALL: [Self; 12] = [
        Self::BrokerageRate,
        Self::BrokerageCap,
        Self::BrokeragePerOrder,
        Self::ExchangeTxn,
        Self::Ipft,
        Self::Clearing,
        Self::SebiFee,
        Self::Stt,
        Self::SttExercise,
        Self::Stamp,
        Self::Gst,
        Self::GstOnSebiFee,
    ];

    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "brokerage_rate" => Self::BrokerageRate,
            "brokerage_cap" => Self::BrokerageCap,
            "brokerage_per_order" => Self::BrokeragePerOrder,
            "exchange_txn" => Self::ExchangeTxn,
            "ipft" => Self::Ipft,
            "clearing" => Self::Clearing,
            "sebi_fee" => Self::SebiFee,
            "stt" => Self::Stt,
            "stt_exercise" => Self::SttExercise,
            "stamp" => Self::Stamp,
            "gst" => Self::Gst,
            "gst_on_sebi_fee" => Self::GstOnSebiFee,
            _ => return None,
        })
    }

    /// The segments a charge must cover on every day of the window.
    fn segments(self) -> &'static [Segment] {
        match self {
            Self::BrokerageRate | Self::BrokerageCap => &[Segment::EquityIntraday],
            Self::BrokeragePerOrder | Self::SttExercise => {
                &[Segment::StockOption, Segment::IndexOption]
            }
            _ => &Segment::ALL,
        }
    }
}

/// How firmly a row's figure is grounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Status {
    /// The figure is quoted from an official page or document.
    Verified,
    /// The figure is official, but applying it to one client order needs a
    /// stated choice (which slab of a ladder, which first day).
    Assumption,
    /// The figure was reached only through an unofficial page.
    Unverified,
    /// The figure did not exist on these dates (a broker not yet trading);
    /// it is applied as a labelled what-if.
    Counterfactual,
    /// No official figure was reached at all. The row carries no number;
    /// the oracle prices nothing for it and flags the line.
    Unknown,
}

impl Status {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "verified" => Some(Self::Verified),
            "assumption" => Some(Self::Assumption),
            "unverified" => Some(Self::Unverified),
            "counterfactual" => Some(Self::Counterfactual),
            "unknown" => Some(Self::Unknown),
            _ => None,
        }
    }
}

/// One dated rate, with its source.
#[derive(Debug, Clone)]
struct Row {
    id: String,
    charge: Charge,
    segments: Vec<Segment>,
    applies: Applies,
    base: String,
    rate: Q,
    from: Date,
    to: Date,
    status: Status,
    value: String,
    source: String,
    quote: String,
    note: String,
}

/// The whole table, and the window it covers.
#[derive(Debug)]
struct Table {
    window_from: Date,
    window_to: Date,
    rows: Vec<Row>,
}

/// The keys a row may carry.
const ROW_KEYS: [&str; 14] = [
    "id", "charge", "segments", "applies", "base", "num", "den", "from", "to", "status", "value",
    "source", "quote", "note",
];

/// Reads one `[[rate]]` key list into a [`Row`].
fn row_of(fields: &[(String, Value)]) -> Result<Row, String> {
    let id = text_field(fields, "id")?;
    let wrap = |why: String| format!("row {id}: {why}");
    if let Some((key, _)) = fields.iter().find(|(k, _)| !ROW_KEYS.contains(&k.as_str())) {
        return Err(wrap(format!("unknown key {key}")));
    }
    let text = |key: &str| text_field(fields, key).map_err(wrap);
    let charge = text("charge")?;
    let charge = Charge::parse(&charge).ok_or_else(|| wrap(format!("bad charge {charge}")))?;
    let segments = text("segments")?
        .split(',')
        .map(|s| Segment::parse(s.trim()).ok_or_else(|| wrap(format!("bad segment {s}"))))
        .collect::<Result<Vec<_>, _>>()?;
    let applies = text("applies")?;
    let applies = Applies::parse(&applies).ok_or_else(|| wrap(format!("bad applies {applies}")))?;
    let num = int_field(fields, "num").map_err(wrap)?;
    let den = int_field(fields, "den").map_err(wrap)?;
    if num < 0 || den <= 0 {
        return Err(wrap(format!("rate {num}/{den} is not a rate")));
    }
    let day = |key: &str| -> Result<Date, String> {
        let raw = text(key)?;
        Date::parse(&raw).ok_or_else(|| wrap(format!("{key} {raw} is not a date")))
    };
    let from = day("from")?;
    let to = day("to")?;
    if to < from {
        return Err(wrap(format!("ends {to} before it starts {from}")));
    }
    let status = text("status")?;
    let status = Status::parse(&status).ok_or_else(|| wrap(format!("bad status {status}")))?;
    if (status == Status::Unknown)
        != (num == 0 && charge != Charge::Clearing && charge != Charge::GstOnSebiFee)
    {
        return Err(wrap(
            "an unknown row carries 0, and only an unknown row does".to_owned(),
        ));
    }
    Ok(Row {
        charge,
        segments,
        applies,
        base: text("base")?,
        rate: Q::new(num, den),
        from,
        to,
        status,
        value: text("value")?,
        source: text("source")?,
        quote: text("quote")?,
        note: text("note")?,
        id,
    })
}

/// Loads the fixture.
fn load_table() -> Result<Table, String> {
    let parsed = parse_fixture(FIXTURE)?;
    let header_date = |key: &str| -> Result<Date, String> {
        match parsed.header.iter().find(|(k, _)| k == key) {
            Some((_, Value::Str(s))) => Date::parse(s).ok_or_else(|| format!("{key} {s}")),
            _ => Err(format!("the fixture has no {key}")),
        }
    };
    let rows = parsed
        .rows
        .iter()
        .map(|fields| row_of(fields))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Table {
        window_from: header_date("window_from")?,
        window_to: header_date("window_to")?,
        rows,
    })
}

/// The table, or a panic naming what is wrong with the fixture.
fn table() -> Table {
    load_table().unwrap_or_else(|why| panic!("the fixture does not load: {why}"))
}

impl Table {
    /// Every row of `charge` for `segment` whose dates contain `day`.
    fn matching(&self, charge: Charge, segment: Segment, day: Date) -> Vec<&Row> {
        self.rows
            .iter()
            .filter(|row| {
                row.charge == charge
                    && row.segments.contains(&segment)
                    && row.from <= day
                    && day <= row.to
            })
            .collect()
    }

    /// The one row of `charge` in force for `segment` on `day` that covers
    /// `side` (`None`: whatever side the row names); `Ok(None)` when the
    /// charge is levied that day but not on that side. An error when no row
    /// is in force at all, or two are.
    fn row(
        &self,
        charge: Charge,
        segment: Segment,
        side: Option<Side>,
        day: Date,
    ) -> Result<Option<&Row>, String> {
        let rows = self.matching(charge, segment, day);
        if rows.is_empty() {
            return Err(format!("no {charge:?} row for {} on {day}", segment.name()));
        }
        let hits: Vec<&Row> = rows
            .iter()
            .copied()
            .filter(|row| side.is_none_or(|s| row.applies.covers(s)))
            .collect();
        match hits.as_slice() {
            [] => Ok(None),
            [one] => Ok(Some(one)),
            many => Err(format!(
                "{} {charge:?} rows overlap for {} on {day}: {}",
                many.len(),
                segment.name(),
                many.iter()
                    .map(|r| r.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }

    /// The rate of `charge` for `segment` and `side` on `day`: zero when it is
    /// not levied on that side, and zero (flagged by [`Self::unknown`]) when
    /// no official figure was reached.
    fn rate(
        &self,
        charge: Charge,
        segment: Segment,
        side: Option<Side>,
        day: Date,
    ) -> Result<Q, String> {
        Ok(self
            .row(charge, segment, side, day)?
            .map_or(Q::ZERO, |row| row.rate))
    }

    /// Whether the row in force carries no official figure.
    fn unknown(
        &self,
        charge: Charge,
        segment: Segment,
        side: Option<Side>,
        day: Date,
    ) -> Result<bool, String> {
        Ok(self
            .row(charge, segment, side, day)?
            .is_some_and(|row| row.status == Status::Unknown))
    }
}

// ---------------------------------------------------------------------------
// The oracle: one order, one client-security-day, one exercise.
// ---------------------------------------------------------------------------

/// One executed order: a side, a quantity in units, a price in paisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrderIn {
    side: Side,
    quantity: i128,
    price: i128,
}

impl OrderIn {
    const fn new(side: Side, quantity: i128, price: i128) -> Self {
        Self {
            side,
            quantity,
            price,
        }
    }

    /// `lots × lot_size` units.
    const fn lots(side: Side, lots: i128, lot_size: i128, price: i128) -> Self {
        Self::new(side, lots * lot_size, price)
    }

    /// The traded value in paisa: price × quantity (the premium value for an
    /// option).
    const fn value(self) -> i128 {
        self.price * self.quantity
    }
}

/// Bit flags naming the lines the oracle could not price.
const UNKNOWN_STAMP: u8 = 1;
const UNKNOWN_STT_EXERCISE: u8 = 2;

/// Every charge line, exactly, in paisa.
#[derive(Debug, Clone, Copy)]
struct Lines {
    brokerage: Q,
    exchange_txn: Q,
    ipft: Q,
    clearing: Q,
    sebi: Q,
    stt: Q,
    stamp: Q,
    gst: Q,
    /// [`UNKNOWN_STAMP`] when stamp duty has no official figure that day.
    unknown: u8,
}

impl Lines {
    fn total(&self) -> Q {
        [
            self.brokerage,
            self.exchange_txn,
            self.ipft,
            self.clearing,
            self.sebi,
            self.stt,
            self.stamp,
            self.gst,
        ]
        .into_iter()
        .fold(Q::ZERO, Q::plus)
    }
}

/// Rounded lines, in paisa, as a contract note would print them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Note {
    brokerage: i128,
    /// Exchange transaction charge, IPFT and clearing charge, one line.
    exchange: i128,
    sebi: i128,
    stt: i128,
    stamp: i128,
    gst: i128,
    total: i128,
}

/// The brokerage on one executed order of `value` paisa.
fn brokerage(table: &Table, segment: Segment, day: Date, value: i128) -> Result<Q, String> {
    if segment.is_option() {
        return table.rate(Charge::BrokeragePerOrder, segment, None, day);
    }
    let rate = table.rate(Charge::BrokerageRate, segment, None, day)?;
    let cap = table.rate(Charge::BrokerageCap, segment, None, day)?;
    Ok(rate.times(Q::int(value)).lesser(cap))
}

/// Whether the SEBI fee is part of the GST base on `day`.
fn sebi_in_gst_base(table: &Table, segment: Segment, day: Date) -> Result<bool, String> {
    Ok(!table
        .rate(Charge::GstOnSebiFee, segment, None, day)?
        .is_zero())
}

/// Nearest rupee, fifty paise and above going up: rule 4 of the Securities
/// Transaction Tax Rules, 2004 ("rounded off to the nearest rupee"; NSE's
/// copy: "if such part is fifty paise or more, it shall be increased to one
/// rupee and if such part is less than fifty paise it shall be ignored").
fn to_rupee(amount: Q) -> i128 {
    amount.half_up(100)
}

/// The oracle for ONE order on its own: each charge exactly, as the rate in
/// force on `day` levies it on this order's value and side. The STT here is
/// the statutory sale-side levy on this order's own value; the exchange's
/// client-day method is [`day_charges`].
fn order_charges(
    table: &Table,
    segment: Segment,
    day: Date,
    order: OrderIn,
) -> Result<Lines, String> {
    let value = Q::int(order.value());
    let per_side = |charge: Charge| -> Result<Q, String> {
        Ok(table
            .rate(charge, segment, Some(order.side), day)?
            .times(value))
    };
    let brokerage = brokerage(table, segment, day, order.value())?;
    let exchange_txn = per_side(Charge::ExchangeTxn)?;
    let ipft = per_side(Charge::Ipft)?;
    let clearing = per_side(Charge::Clearing)?;
    let sebi = per_side(Charge::SebiFee)?;
    let stt = per_side(Charge::Stt)?;
    let stamp = per_side(Charge::Stamp)?;
    let mut base = brokerage.plus(exchange_txn).plus(ipft).plus(clearing);
    if sebi_in_gst_base(table, segment, day)? {
        base = base.plus(sebi);
    }
    let gst = table.rate(Charge::Gst, segment, None, day)?.times(base);
    let unknown = if table.unknown(Charge::Stamp, segment, Some(order.side), day)? {
        UNKNOWN_STAMP
    } else {
        0
    };
    Ok(Lines {
        brokerage,
        exchange_txn,
        ipft,
        clearing,
        sebi,
        stt,
        stamp,
        gst,
        unknown,
    })
}

/// The day's totals for one client in one security.
#[derive(Debug, Clone, Copy, Default)]
struct DayTotals {
    buy_qty: i128,
    buy_value: i128,
    sell_qty: i128,
    sell_value: i128,
}

impl DayTotals {
    fn of(orders: &[OrderIn]) -> Self {
        let mut t = Self::default();
        for order in orders {
            match order.side {
                Side::Buy => {
                    t.buy_qty += order.quantity;
                    t.buy_value += order.value();
                }
                Side::Sell => {
                    t.sell_qty += order.quantity;
                    t.sell_value += order.value();
                }
            }
        }
        t
    }

    const fn turnover(self) -> i128 {
        self.buy_value + self.sell_value
    }
}

/// One client-security-day priced by the contract-note method, exactly and
/// rounded.
#[derive(Debug, Clone, Copy)]
struct DayOut {
    /// Every line before rounding; GST on the note's rounded base.
    exact: Lines,
    note: Note,
}

/// The value the STT is levied on for one client-security-day (paisa), and
/// the value stamp duty is levied on.
///
/// Options: the sell premium value, and the buy premium value ("each ...
/// option trade is valued at premium", NSE; stamp duty on "the premium paid
/// by the buyer", Indian Stamp Act s.21).
///
/// Equity intraday: the offset quantity at the client's volume-weighted
/// average price for the scrip and day, that price "rounded off to the
/// nearest paisa" (STT Rules 2004, rule 3; NSE/CMTR/5459); stamp duty on the
/// buy value.
fn taxed_values(segment: Segment, t: DayTotals) -> Result<(Q, Q), String> {
    if segment.is_option() {
        return Ok((Q::int(t.sell_value), Q::int(t.buy_value)));
    }
    if t.buy_qty != t.sell_qty || t.buy_qty == 0 {
        return Err(format!(
            "an intraday day must square off: bought {} sold {}",
            t.buy_qty, t.sell_qty
        ));
    }
    let units = t.buy_qty;
    let vwap = Q::new(t.turnover(), 2 * units).half_up(1);
    Ok((Q::int(vwap * units), Q::int(t.buy_value)))
}

/// The oracle for one client-security-day.
///
/// Rounding, each step as `docs/charges-oracle/RATES.md` §5 sources it:
/// brokerage per order, the exchange line (transaction charge + IPFT +
/// clearing) and the SEBI fee on the day's turnover, and GST on the rounded
/// base — each half-up to the paisa (no official rule reached: assumption);
/// the STT's value and the STT itself each to the nearest rupee (STT Rules
/// rule 4); stamp duty to the nearest rupee (no official rule reached:
/// assumption).
fn day_charges(
    table: &Table,
    segment: Segment,
    day: Date,
    orders: &[OrderIn],
) -> Result<DayOut, String> {
    if orders.is_empty() {
        return Err("an empty day".to_owned());
    }
    let t = DayTotals::of(orders);
    let turnover = Q::int(t.turnover());
    let both = |charge: Charge| -> Result<Q, String> {
        Ok(table.rate(charge, segment, None, day)?.times(turnover))
    };
    let mut brokerage_exact = Q::ZERO;
    let mut brokerage_note = 0;
    for order in orders {
        let one = brokerage(table, segment, day, order.value())?;
        brokerage_exact = brokerage_exact.plus(one);
        brokerage_note += one.half_up(1);
    }
    let exchange_txn = both(Charge::ExchangeTxn)?;
    let ipft = both(Charge::Ipft)?;
    let clearing = both(Charge::Clearing)?;
    let sebi = both(Charge::SebiFee)?;
    let (stt_value, stamp_value) = taxed_values(segment, t)?;
    let stt_rate = table.rate(Charge::Stt, segment, Some(Side::Sell), day)?;
    let stamp_rate = table.rate(Charge::Stamp, segment, Some(Side::Buy), day)?;
    let exchange_note = exchange_txn.plus(ipft).plus(clearing).half_up(1);
    let sebi_note = sebi.half_up(1);
    let stt_note = to_rupee(stt_rate.times(Q::int(to_rupee(stt_value))));
    let stamp_note = to_rupee(stamp_rate.times(stamp_value));
    let gst_rate = table.rate(Charge::Gst, segment, None, day)?;
    let with_sebi = sebi_in_gst_base(table, segment, day)?;
    let base_note = brokerage_note + exchange_note + if with_sebi { sebi_note } else { 0 };
    let gst_exact = gst_rate.times(Q::int(base_note));
    let gst_note = gst_exact.half_up(1);
    // A day with no purchase owes no stamp duty, known or not.
    let unknown =
        if t.buy_value > 0 && table.unknown(Charge::Stamp, segment, Some(Side::Buy), day)? {
            UNKNOWN_STAMP
        } else {
            0
        };
    Ok(DayOut {
        exact: Lines {
            brokerage: brokerage_exact,
            exchange_txn,
            ipft,
            clearing,
            sebi,
            stt: stt_rate.times(stt_value),
            stamp: stamp_rate.times(stamp_value),
            gst: gst_exact,
            unknown,
        },
        note: Note {
            brokerage: brokerage_note,
            exchange: exchange_note,
            sebi: sebi_note,
            stt: stt_note,
            stamp: stamp_note,
            gst: gst_note,
            total: brokerage_note + exchange_note + sebi_note + stt_note + stamp_note + gst_note,
        },
    })
}

/// An index option exercised at expiry, held by the client.
#[derive(Debug, Clone, Copy)]
struct Exercise {
    /// The final settlement price, paisa.
    settlement: i128,
    /// The strike, paisa.
    strike: i128,
    quantity: i128,
}

impl Exercise {
    /// The in-the-money amount × quantity: what NSE calls the intrinsic value.
    const fn intrinsic(self) -> i128 {
        (self.settlement - self.strike).abs() * self.quantity
    }

    /// Settlement price × quantity.
    const fn settlement_value(self) -> i128 {
        self.settlement * self.quantity
    }
}

/// The STT on an exercise: exactly, and as the note rounds it (rule 4, value
/// and tax to the rupee). The base is the row's: the settlement value before
/// 2019-09-01, the intrinsic value from then on.
fn exercise_stt(
    table: &Table,
    segment: Segment,
    day: Date,
    exercise: Exercise,
) -> Result<(Q, i128), String> {
    let row = table
        .row(Charge::SttExercise, segment, None, day)?
        .ok_or("no exercise row")?;
    if row.status == Status::Unknown {
        return Err(format!("{}: {}", row.id, row.value));
    }
    let base = if row.base.starts_with("settlement value") {
        exercise.settlement_value()
    } else if row.base.starts_with("intrinsic value") {
        exercise.intrinsic()
    } else {
        return Err(format!(
            "{}: an exercise base this oracle does not know",
            row.id
        ));
    };
    let exact = row.rate.times(Q::int(base));
    Ok((
        exact,
        to_rupee(row.rate.times(Q::int(to_rupee(Q::int(base))))),
    ))
}

// ---------------------------------------------------------------------------
// The product, driven through its public API only.
// ---------------------------------------------------------------------------

mod product {
    use super::{Date, OrderIn, Q, Segment, Side};
    use brutex_core::instrument::Exchange;
    use brutex_core::price::Paisa;
    use brutex_core::symbol::Symbol;
    use costs::day::TradeDay;
    use costs::exact::{Ratio, UNITS_PER_PAISA};
    use costs::fill::{Bar, Direction};
    use costs::rate::{BpsX100, Broker, OrderSide, RATE_SCALE};
    use costs::schedule::{
        Product, RateCard, RateSelector, Settlement, Taxes, TradeKind, dhan_brokerage,
    };
    use costs::stack::{ClientSecurityDay, DayCharges, Order};
    use costs::trip::{Contract, Leg, Outcome, RoundTrip};

    /// The product's kind for an oracle segment. Stock and index options are
    /// one kind there.
    pub(super) const fn kind(segment: Segment) -> TradeKind {
        match segment {
            Segment::EquityIntraday => TradeKind::Equity(Product::Intraday),
            Segment::StockOption | Segment::IndexOption => TradeKind::Options,
        }
    }

    /// The product's day for an oracle date.
    pub(super) fn trade_day(day: Date) -> TradeDay {
        let (y, m, d) = day.civil();
        TradeDay::new(
            u16::try_from(y).expect("a four-digit year"),
            u8::try_from(m).expect("a month"),
            u8::try_from(d).expect("a day"),
        )
        .expect("a real date")
    }

    fn ratio(rate: Ratio) -> Q {
        Q::new(i128::from(rate.numerator()), i128::from(rate.denominator()))
    }

    fn bps(rate: BpsX100) -> Q {
        Q::new(i128::from(rate.get()), i128::from(RATE_SCALE))
    }

    fn paisa(amount: Paisa) -> i128 {
        i128::from(amount.raw())
    }

    fn money(amount: i128) -> Result<Paisa, String> {
        i64::try_from(amount)
            .map(Paisa::from_raw)
            .map_err(|e| e.to_string())
    }

    /// The rates on the product's card for `segment` on `day`.
    #[derive(Debug, Clone, Copy)]
    pub(super) struct Card {
        /// Exchange transaction charge with the IPFT folded in.
        pub(super) exchange: Q,
        pub(super) sebi: Q,
        pub(super) gst: Q,
        pub(super) stt_sell: Q,
        pub(super) stt_exercise: Option<Q>,
        pub(super) stamp_buy: Q,
    }

    /// The card, or the product's refusal as text.
    pub(super) fn card(
        segment: Segment,
        day: Date,
        selector: RateSelector,
    ) -> Result<Card, String> {
        let card = RateCard::resolve(kind(segment), Exchange::Nse, trade_day(day), selector)
            .map_err(|e| e.to_string())?;
        let (stt_sell, stt_exercise, stamp_buy) = match card.taxes() {
            Taxes::Equity {
                stt_non_delivery,
                stamp_non_delivery,
                ..
            } => (stt_non_delivery.rate(), None, stamp_non_delivery.rate()),
            Taxes::Options {
                stt_sell,
                stt_exercise,
                stamp_buy,
            } => (
                stt_sell.rate(),
                Some(ratio(stt_exercise.rate())),
                stamp_buy.rate(),
            ),
            Taxes::Futures { .. } => return Err("a futures card".to_owned()),
        };
        Ok(Card {
            exchange: ratio(card.exchange_charge().rate()),
            sebi: ratio(card.sebi().rate()),
            gst: ratio(card.gst().rate()),
            stt_sell: ratio(stt_sell),
            stt_exercise,
            stamp_buy: ratio(stamp_buy),
        })
    }

    /// The precise stack's lines for one client-security-day: rounded in
    /// paisa, and unrounded.
    #[derive(Debug, Clone, Copy)]
    pub(super) struct Lines {
        pub(super) brokerage: i128,
        pub(super) exchange: i128,
        pub(super) sebi: i128,
        pub(super) stt: i128,
        pub(super) stt_exercise: i128,
        pub(super) stamp: i128,
        pub(super) gst: i128,
        pub(super) total: i128,
        /// brokerage, exchange, SEBI, STT, STT on exercise, stamp, GST.
        pub(super) exact: [Q; 7],
    }

    fn lines_of(c: &DayCharges) -> Lines {
        let unrounded = c.unrounded_lines();
        let exact = |i: usize| Q::new(unrounded[i].1.units(), UNITS_PER_PAISA);
        Lines {
            brokerage: paisa(c.brokerage()),
            exchange: paisa(c.exchange_charge()),
            sebi: paisa(c.sebi()),
            stt: paisa(c.stt()),
            stt_exercise: paisa(c.stt_exercise()),
            stamp: paisa(c.stamp()),
            gst: paisa(c.gst()),
            total: paisa(c.total()),
            exact: [
                exact(0),
                exact(1),
                exact(2),
                exact(3),
                exact(4),
                exact(5),
                exact(6),
            ],
        }
    }

    /// Prices `orders` as one client-security-day under `selector`.
    pub(super) fn day(
        segment: Segment,
        day: Date,
        selector: RateSelector,
        orders: &[OrderIn],
    ) -> Result<Lines, String> {
        let mut book =
            ClientSecurityDay::open(kind(segment), Exchange::Nse, trade_day(day), selector)
                .map_err(|e| e.to_string())?;
        for order in orders {
            let side = match order.side {
                Side::Buy => OrderSide::Buy,
                Side::Sell => OrderSide::Sell,
            };
            let quantity = i64::try_from(order.quantity).map_err(|e| e.to_string())?;
            let placed =
                Order::new(side, quantity, money(order.price)?).map_err(|e| e.to_string())?;
            book.add(placed).map_err(|e| e.to_string())?;
        }
        let charges = book.settle().map_err(|e| e.to_string())?;
        Ok(lines_of(&charges))
    }

    /// Prices one cash-settled exercise worth `intrinsic` paisa, alone on its
    /// day.
    pub(super) fn exercise(
        day: Date,
        selector: RateSelector,
        intrinsic: i128,
    ) -> Result<Lines, String> {
        let mut book =
            ClientSecurityDay::open(TradeKind::Options, Exchange::Nse, trade_day(day), selector)
                .map_err(|e| e.to_string())?;
        book.exercise(money(intrinsic)?, Settlement::Cash)
            .map_err(|e| e.to_string())?;
        let charges = book.settle().map_err(|e| e.to_string())?;
        Ok(lines_of(&charges))
    }

    /// The dated regime's own lookups for NSE options on `day`.
    pub(super) struct Regime {
        pub(super) stt_options: Result<Q, String>,
        pub(super) stt_exercise: Result<Q, String>,
        /// Exchange transaction charge + IPFT, as `trip` adds them.
        pub(super) exchange: Result<Q, String>,
    }

    pub(super) fn regime(day: Date) -> Regime {
        let td = trade_day(day);
        let exchange = costs::regime::exchange_charge_rate(Exchange::Nse, td)
            .and_then(|e| costs::regime::ipft_rate(Exchange::Nse, td).map(|i| bps(e).plus(bps(i))))
            .map_err(|e| e.to_string());
        Regime {
            stt_options: costs::regime::stt_options_rate(td)
                .map(bps)
                .map_err(|e| e.to_string()),
            stt_exercise: costs::regime::stt_exercise_rate(td)
                .map(bps)
                .map_err(|e| e.to_string()),
            exchange,
        }
    }

    /// Dhan's brokerage on one executed order of `value` paisa.
    pub(super) fn brokerage(segment: Segment, value: i128) -> Result<i128, String> {
        dhan_brokerage(kind(segment), money(value)?)
            .map(paisa)
            .map_err(|e| e.to_string())
    }

    /// One quote from the crate's replica of Dhan's public calculator.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) struct Quote {
        pub(super) brokerage: i128,
        pub(super) exchange: i128,
        pub(super) sebi: i128,
        pub(super) stt: i128,
        pub(super) stamp: i128,
        pub(super) gst: i128,
        pub(super) total: i128,
    }

    pub(super) fn dhan_quote(
        segment: Segment,
        quantity: i128,
        buy: i128,
        sell: i128,
    ) -> Result<Quote, String> {
        let q = costs::dhan_calc::quote(
            kind(segment),
            Exchange::Nse,
            i64::try_from(quantity).map_err(|e| e.to_string())?,
            Some(money(buy)?),
            Some(money(sell)?),
        )
        .map_err(|e| e.to_string())?;
        let line = |r: Result<Paisa, costs::charge_error::ChargeError>| -> Result<i128, String> {
            r.map(paisa).map_err(|e| e.to_string())
        };
        Ok(Quote {
            brokerage: line(q.brokerage())?,
            exchange: line(q.exchange_charge())?,
            sebi: line(q.sebi())?,
            stt: line(q.stt())?,
            stamp: line(q.stamp())?,
            gst: line(q.gst())?,
            total: line(q.total())?,
        })
    }

    /// One long NIFTY round trip through `costs::trip::price`, both legs on
    /// `day`, on bars printed at `buy` and `sell` paisa.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) struct Trip {
        pub(super) buy_fill: i128,
        pub(super) sell_fill: i128,
        pub(super) brokerage: i128,
        pub(super) exchange: i128,
        pub(super) ipft: i128,
        pub(super) sebi: i128,
        pub(super) stt: i128,
        pub(super) stamp: i128,
        pub(super) gst: i128,
        pub(super) total: i128,
    }

    pub(super) fn trip(day: Date, quantity: i128, buy: i128, sell: i128) -> Result<Trip, String> {
        let slot = costs::venue::swept_slot(Symbol::new("NIFTY").map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let td = trade_day(day);
        let leg = |price: i128| -> Result<Leg, String> {
            Ok(Leg::new(
                td,
                Bar::flat(money(price)?).map_err(|e| e.to_string())?,
            ))
        };
        let round_trip = RoundTrip::new(
            Contract::new(slot, costs::scope::Segment::IndexOption),
            Direction::Long,
            Broker::Groww,
            Outcome::NormalClose,
            leg(buy)?,
            leg(sell)?,
            i64::try_from(quantity).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let c = costs::trip::price(&round_trip).map_err(|e| e.to_string())?;
        Ok(Trip {
            buy_fill: paisa(c.buy_fill()),
            sell_fill: paisa(c.sell_fill()),
            brokerage: paisa(c.brokerage()),
            exchange: paisa(c.exchange()),
            ipft: paisa(c.ipft()),
            sebi: paisa(c.sebi()),
            stt: paisa(c.stt()),
            stamp: paisa(c.stamp()),
            gst: paisa(c.gst()),
            total: paisa(c.total_charges()),
        })
    }
}

// ---------------------------------------------------------------------------
// Rate-by-rate comparison, on every calendar day of the window.
// ---------------------------------------------------------------------------

/// The oracle's rates for `segment` on `day`, shaped like the product's card.
#[derive(Debug, Clone, Copy)]
struct OracleCard {
    /// Exchange transaction charge + IPFT + clearing, one figure.
    exchange: Q,
    sebi: Q,
    gst: Q,
    stt_sell: Q,
    stt_exercise: Q,
    stamp_buy: Q,
    sebi_in_gst: bool,
    unknown: u8,
}

fn oracle_card(table: &Table, segment: Segment, day: Date) -> Result<OracleCard, String> {
    let any = |charge: Charge| table.rate(charge, segment, None, day);
    let mut unknown = 0;
    if table.unknown(Charge::Stamp, segment, Some(Side::Buy), day)? {
        unknown |= UNKNOWN_STAMP;
    }
    let stt_exercise = if segment.is_option() {
        if table.unknown(Charge::SttExercise, segment, None, day)? {
            unknown |= UNKNOWN_STT_EXERCISE;
        }
        any(Charge::SttExercise)?
    } else {
        Q::ZERO
    };
    Ok(OracleCard {
        exchange: any(Charge::ExchangeTxn)?
            .plus(any(Charge::Ipft)?)
            .plus(any(Charge::Clearing)?),
        sebi: any(Charge::SebiFee)?,
        gst: any(Charge::Gst)?,
        stt_sell: table.rate(Charge::Stt, segment, Some(Side::Sell), day)?,
        stt_exercise,
        stamp_buy: table.rate(Charge::Stamp, segment, Some(Side::Buy), day)?,
        sebi_in_gst: sebi_in_gst_base(table, segment, day)?,
        unknown,
    })
}

/// Which product rates a comparison read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Set {
    /// `RateSelector::CurrentOnEveryDate`: today's rates on every date — the
    /// set the crate documents as the backtest's.
    Current,
    /// `RateSelector::Dated`: the rates in force on the trade date.
    Dated,
    /// The dated regime's own lookups (`costs::regime`), which
    /// `costs::trip::price` reads.
    Regime,
}

impl Set {
    const fn selector(self) -> costs::schedule::RateSelector {
        match self {
            Self::Current => costs::schedule::RateSelector::CurrentOnEveryDate,
            Self::Dated | Self::Regime => costs::schedule::RateSelector::Dated,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Current => "CurrentOnEveryDate",
            Self::Dated => "Dated",
            Self::Regime => "costs::regime",
        }
    }
}

/// What the product answered for one component on one day.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Answer {
    /// The product's rate equals the oracle's.
    Same,
    /// The product priced at a different rate: (product, official).
    Differs(String, String),
    /// The product refused the day.
    Refused,
    /// No official figure was reached; the product's is shown.
    OfficialUnknown(String),
}

/// One run of consecutive days on which a component's answer did not change.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Run {
    set: Set,
    segment: Segment,
    component: &'static str,
    from: Date,
    to: Date,
    answer: Answer,
    /// Product − official, in rupees per crore of the base the component is
    /// levied on; empty unless the answer differs.
    per_crore: String,
}

impl Run {
    const fn differs(&self) -> bool {
        matches!(self.answer, Answer::Differs(..))
    }
}

/// The card components compared rate by rate.
const COMPONENTS: [&str; 7] = [
    "exchange transaction charge incl. IPFT and clearing (each side)",
    "SEBI turnover fee (each side)",
    "GST rate",
    "SEBI fee inside the GST base",
    "STT on the sale",
    "STT on exercise",
    "stamp duty on the purchase",
];

/// The components of `costs::regime` compared rate by rate.
const REGIME_COMPONENTS: [&str; 3] = [
    "regime::stt_options_rate",
    "regime::stt_exercise_rate (rate, and its documented base: intrinsic value)",
    "regime::exchange_charge_rate + regime::ipft_rate",
];

/// A rate as an exact percentage.
fn pct(rate: Q) -> String {
    let units = rate.times(Q::int(100_000_000_000)); // percent × 10^9
    if units.d == 1 {
        let whole = units.n / 1_000_000_000;
        let mut frac = format!("{:09}", units.n % 1_000_000_000);
        while frac.ends_with('0') && frac.len() > 1 {
            frac.pop();
        }
        format!("{whole}.{frac}%")
    } else {
        format!("{}/{}", rate.n, rate.d)
    }
}

/// Rupees per crore for a rate difference.
fn per_crore(diff: Q) -> String {
    format!("{}", diff.times(Q::int(1_000_000_000)))
}

/// Extends the last run of the same key when nothing changed, else starts a
/// new one.
fn push_run(
    runs: &mut Vec<Run>,
    key: (Set, Segment, &'static str),
    day: Date,
    answer: Answer,
    per_crore: String,
) {
    let (set, segment, component) = key;
    if let Some(last) = runs
        .iter_mut()
        .rev()
        .find(|r| r.set == set && r.segment == segment && r.component == component)
        && last.to.next() == day
        && last.answer == answer
        && last.per_crore == per_crore
    {
        last.to = day;
        return;
    }
    runs.push(Run {
        set,
        segment,
        component,
        from: day,
        to: day,
        answer,
        per_crore,
    });
}

/// One product rate against one official rate.
fn pair(product: Q, official: Q) -> (Answer, String) {
    if product.same_as(official) {
        (Answer::Same, String::new())
    } else {
        (
            Answer::Differs(pct(product), pct(official)),
            per_crore(product.minus(official)),
        )
    }
}

/// One card component of one day.
fn compare_component(index: usize, card: &product::Card, oracle: &OracleCard) -> (Answer, String) {
    match index {
        0 => pair(card.exchange, oracle.exchange),
        1 => pair(card.sebi, oracle.sebi),
        2 => pair(card.gst, oracle.gst),
        // The precise stack always puts the SEBI fee in the GST base.
        3 if oracle.sebi_in_gst => (Answer::Same, String::new()),
        3 => (
            Answer::Differs("included".to_owned(), "excluded".to_owned()),
            per_crore(oracle.gst.times(oracle.sebi)),
        ),
        4 => pair(card.stt_sell, oracle.stt_sell),
        5 => match card.stt_exercise {
            None => (Answer::Same, String::new()),
            Some(rate) if oracle.unknown & UNKNOWN_STT_EXERCISE != 0 => {
                (Answer::OfficialUnknown(pct(rate)), String::new())
            }
            Some(rate) => pair(rate, oracle.stt_exercise),
        },
        _ if oracle.unknown & UNKNOWN_STAMP != 0 => {
            (Answer::OfficialUnknown(pct(card.stamp_buy)), String::new())
        }
        _ => pair(card.stamp_buy, oracle.stamp_buy),
    }
}

/// Compares the product's card with the oracle's on every day of the window,
/// for every segment and both selectors, and folds the result into runs.
fn card_runs(table: &Table) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for set in [Set::Current, Set::Dated] {
        for segment in Segment::ALL {
            let mut day = table.window_from;
            while day <= table.window_to {
                let oracle = oracle_card(table, segment, day)
                    .unwrap_or_else(|why| panic!("the oracle has a hole: {why}"));
                let card = product::card(segment, day, set.selector());
                for (index, component) in COMPONENTS.iter().enumerate() {
                    let (answer, diff) = match &card {
                        Err(_) => (Answer::Refused, String::new()),
                        Ok(card) => compare_component(index, card, &oracle),
                    };
                    push_run(&mut runs, (set, segment, component), day, answer, diff);
                }
                day = day.next();
            }
        }
    }
    runs
}

/// Compares the regime's lookups with the oracle's index-option rates on
/// every day of the window.
fn regime_runs(table: &Table) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    let segment = Segment::IndexOption;
    let mut day = table.window_from;
    while day <= table.window_to {
        let oracle = oracle_card(table, segment, day)
            .unwrap_or_else(|why| panic!("the oracle has a hole: {why}"));
        let regime = product::regime(day);
        let exercise_row = table
            .row(Charge::SttExercise, segment, None, day)
            .ok()
            .flatten()
            .expect("an exercise row");
        let base_differs = !exercise_row.base.starts_with("intrinsic value");
        let answers = [
            regime.stt_options.map(|p| pair(p, oracle.stt_sell)),
            regime.stt_exercise.map(|p| {
                if base_differs {
                    (
                        Answer::Differs(
                            format!("{} of the intrinsic value", pct(p)),
                            format!("{} of the settlement value", pct(oracle.stt_exercise)),
                        ),
                        String::new(),
                    )
                } else {
                    pair(p, oracle.stt_exercise)
                }
            }),
            regime.exchange.map(|p| pair(p, oracle.exchange)),
        ];
        for (component, answer) in REGIME_COMPONENTS.iter().zip(answers) {
            let (answer, diff) = answer.unwrap_or((Answer::Refused, String::new()));
            push_run(
                &mut runs,
                (Set::Regime, segment, component),
                day,
                answer,
                diff,
            );
        }
        day = day.next();
    }
    runs
}

impl fmt::Display for Run {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} | {} | {}..{} | ",
            self.set.name(),
            self.segment.name(),
            self.component,
            self.from,
            self.to
        )?;
        match &self.answer {
            Answer::Same => write!(f, "matches"),
            Answer::Refused => write!(f, "product refuses (no rate)"),
            Answer::OfficialUnknown(p) => {
                write!(
                    f,
                    "product {p}; no official figure was reached (unverifiable)"
                )
            }
            Answer::Differs(p, o) if self.per_crore.is_empty() => {
                write!(f, "product {p} vs official {o}")
            }
            Answer::Differs(p, o) => write!(
                f,
                "product {p} vs official {o}; product minus official = {} per crore",
                self.per_crore
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// The grid of orders.
// ---------------------------------------------------------------------------

/// One client-security-day the grid prices.
#[derive(Debug, Clone)]
struct Case {
    name: &'static str,
    segment: Segment,
    orders: Vec<OrderIn>,
}

impl Case {
    /// The one quantity of a one-buy, one-sell day, with its two prices.
    fn round_trip(&self) -> Option<(i128, i128, i128)> {
        match self.orders.as_slice() {
            [b, s] if b.side == Side::Buy && s.side == Side::Sell && b.quantity == s.quantity => {
                Some((b.quantity, b.price, s.price))
            }
            _ => None,
        }
    }
}

const fn buy(quantity: i128, price: i128) -> OrderIn {
    OrderIn::new(Side::Buy, quantity, price)
}

const fn sell(quantity: i128, price: i128) -> OrderIn {
    OrderIn::new(Side::Sell, quantity, price)
}

/// Equity intraday days: every one squares off.
fn equity_cases() -> Vec<Case> {
    let case = |name, orders: Vec<OrderIn>| Case {
        name,
        segment: Segment::EquityIntraday,
        orders,
    };
    vec![
        case(
            "tiny: 1 share at Rs 1.00 / 1.05",
            vec![buy(1, 100), sell(1, 105)],
        ),
        case(
            "tick edges: 7 at Rs 99.95 / 100.05",
            vec![buy(7, 9_995), sell(7, 10_005)],
        ),
        case(
            "odd: 13 at Rs 1,234.55 / 1,234.60",
            vec![buy(13, 123_455), sell(13, 123_460)],
        ),
        case(
            "cap just below: 2 at Rs 33,300.00",
            vec![buy(2, 3_330_000), sell(2, 3_330_000)],
        ),
        case(
            "cap half-paisa edge: 1 at Rs 66,650.00",
            vec![buy(1, 6_665_000), sell(1, 6_665_005)],
        ),
        case(
            "cap hit: 100 at Rs 1,000 / 1,010",
            vec![buy(100, 100_000), sell(100, 101_000)],
        ),
        case(
            "short first: sell 500 at Rs 2,000, buy at 1,980",
            vec![sell(500, 200_000), buy(500, 198_000)],
        ),
        case(
            "VWAP half-paisa: 1 at Rs 100.00 / 100.05",
            vec![buy(1, 10_000), sell(1, 10_005)],
        ),
        case(
            "several orders: buy 30 + 20, sell 50",
            vec![buy(30, 25_010), buy(20, 25_015), sell(50, 25_100)],
        ),
        case(
            "huge: 100,000 at Rs 5,000.00 / 5,000.05",
            vec![buy(100_000, 500_000), sell(100_000, 500_005)],
        ),
    ]
}

/// Option days for `segment`: one-sided days and round trips, quantities as
/// lots × lot size.
fn option_cases(segment: Segment) -> Vec<Case> {
    let case = |name, orders: Vec<OrderIn>| Case {
        name,
        segment,
        orders,
    };
    let lots = OrderIn::lots;
    vec![
        case(
            "tiny buy: 1 lot of 75 at Rs 0.05",
            vec![lots(Side::Buy, 1, 75, 5)],
        ),
        case(
            "tiny sell: 1 lot of 75 at Rs 0.05",
            vec![lots(Side::Sell, 1, 75, 5)],
        ),
        case(
            "reference: 1 lot of 75 at Rs 100 / 120",
            vec![
                lots(Side::Buy, 1, 75, 10_000),
                lots(Side::Sell, 1, 75, 12_000),
            ],
        ),
        case(
            "odd lots: 7 lots of 65 at Rs 99.95 / 100.05",
            vec![
                lots(Side::Buy, 7, 65, 9_995),
                lots(Side::Sell, 7, 65, 10_005),
            ],
        ),
        case(
            "3 lots of 35 at Rs 245.35 / 250.10",
            vec![
                lots(Side::Buy, 3, 35, 24_535),
                lots(Side::Sell, 3, 35, 25_010),
            ],
        ),
        case(
            "stock lot of 1,800 at Rs 3.45 / 3.50",
            vec![
                lots(Side::Buy, 1, 1_800, 345),
                lots(Side::Sell, 1, 1_800, 350),
            ],
        ),
        case(
            "STT half-paisa edge: sell 1 lot of 75 at Rs 4.40",
            vec![lots(Side::Sell, 1, 75, 440)],
        ),
        case(
            "STT half-paisa edge: sell 1 lot of 75 at Rs 6.60",
            vec![lots(Side::Sell, 1, 75, 660)],
        ),
        case(
            "huge: 1,800 lots of 75 at Rs 500 / 505",
            vec![
                lots(Side::Buy, 1_800, 75, 50_000),
                lots(Side::Sell, 1_800, 75, 50_500),
            ],
        ),
        case(
            "several orders: buy 2 x 1 lot, sell 2 lots, lot 50",
            vec![
                lots(Side::Buy, 1, 50, 15_005),
                lots(Side::Buy, 1, 50, 15_010),
                lots(Side::Sell, 2, 50, 15_500),
            ],
        ),
    ]
}

/// Every grid day.
fn all_cases() -> Vec<Case> {
    let mut cases = equity_cases();
    cases.extend(option_cases(Segment::StockOption));
    cases.extend(option_cases(Segment::IndexOption));
    cases
}

/// The reference trade of each segment, for rupee sizes.
fn reference(segment: Segment) -> Case {
    let orders = match segment {
        Segment::EquityIntraday => vec![buy(100, 100_000), sell(100, 101_000)],
        Segment::IndexOption => vec![buy(75, 10_000), sell(75, 12_000)],
        Segment::StockOption => vec![buy(500, 2_000), sell(500, 2_500)],
    };
    let name = match segment {
        Segment::EquityIntraday => "100 shares bought at Rs 1,000 and sold at Rs 1,010",
        Segment::IndexOption => "75 units (1 lot) bought at Rs 100 and sold at Rs 120",
        Segment::StockOption => "500 units (1 lot) bought at Rs 20 and sold at Rs 25",
    };
    Case {
        name,
        segment,
        orders,
    }
}

/// The days the order grid is priced on: every first and last day of every
/// oracle row and of every run the rate comparison found, the day either
/// side of each, and the window's ends.
fn grid_days(table: &Table, runs: &[Run]) -> Vec<Date> {
    let mut days = BTreeSet::new();
    let mut add = |day: Date| {
        for candidate in [day.prev(), day, day.next()] {
            if table.window_from <= candidate && candidate <= table.window_to {
                days.insert(candidate);
            }
        }
    };
    for row in &table.rows {
        add(row.from);
        add(row.to);
    }
    for run in runs {
        add(run.from);
        add(run.to);
    }
    days.into_iter().collect()
}

// ---------------------------------------------------------------------------
// Amount comparison on the grid.
// ---------------------------------------------------------------------------

/// The lines compared amount by amount.
const AMOUNT_LINES: [&str; 7] = [
    "brokerage",
    "exchange charge incl. IPFT",
    "SEBI fee",
    "STT",
    "stamp duty",
    "GST",
    "total",
];

/// One line of one grid day on which the product and the oracle disagree.
#[derive(Debug, Clone)]
struct LineDiff {
    set: Set,
    segment: Segment,
    day: Date,
    case: &'static str,
    line: &'static str,
    /// Product − oracle, paisa.
    diff: Q,
    product: String,
    oracle: String,
}

/// The oracle's lines in [`AMOUNT_LINES`] order, rounded (`exact = false`) or
/// not. GST "exact" is on the rounded base, as a note bills it. A line with
/// no official figure is `None`; the total then excludes it.
fn oracle_lines(out: &DayOut, exact: bool) -> [Option<Q>; 7] {
    let unknown_stamp = out.exact.unknown & UNKNOWN_STAMP != 0;
    let e = out.exact;
    let n = out.note;
    let stamp = |q: Q| (!unknown_stamp).then_some(q);
    if exact {
        let total = e
            .total()
            .minus(if unknown_stamp { e.stamp } else { Q::ZERO });
        [
            Some(e.brokerage),
            Some(e.exchange_txn.plus(e.ipft).plus(e.clearing)),
            Some(e.sebi),
            Some(e.stt),
            stamp(e.stamp),
            Some(e.gst),
            Some(total),
        ]
    } else {
        let total = n.total - if unknown_stamp { n.stamp } else { 0 };
        [
            Some(Q::int(n.brokerage)),
            Some(Q::int(n.exchange)),
            Some(Q::int(n.sebi)),
            Some(Q::int(n.stt)),
            stamp(Q::int(n.stamp)),
            Some(Q::int(n.gst)),
            Some(Q::int(total)),
        ]
    }
}

/// The product's lines in [`AMOUNT_LINES`] order (STT on exercise excluded:
/// the grid exercises nothing).
fn product_lines(p: &product::Lines, exact: bool) -> [Q; 7] {
    if exact {
        let [brokerage, exchange, sebi, stt, _, stamp, gst] = p.exact;
        let lines = [brokerage, exchange, sebi, stt, stamp, gst];
        let total = lines.into_iter().fold(Q::ZERO, Q::plus);
        [brokerage, exchange, sebi, stt, stamp, gst, total]
    } else {
        [
            Q::int(p.brokerage),
            Q::int(p.exchange),
            Q::int(p.sebi),
            Q::int(p.stt),
            Q::int(p.stamp),
            Q::int(p.gst),
            Q::int(p.total),
        ]
    }
}

/// The product's unrounded lines are counted in 10⁻⁷ paisa, half-up; one
/// such unit is the tolerance of an unrounded comparison.
fn tolerance() -> Q {
    Q::new(1, 10_000_000)
}

/// Prices every case on every day under `set` in both the product and the
/// oracle, and returns every line on which they disagree. Days the product
/// refuses are skipped and counted.
fn compare_grid(
    table: &Table,
    set: Set,
    days: &[Date],
    cases: &[Case],
    exact: bool,
) -> (Vec<LineDiff>, usize) {
    let mut diffs = Vec::new();
    let mut refused = 0;
    for &day in days {
        for case in cases {
            let oracle = day_charges(table, case.segment, day, &case.orders)
                .unwrap_or_else(|why| panic!("the oracle cannot price {}: {why}", case.name));
            let Ok(product) = product::day(case.segment, day, set.selector(), &case.orders) else {
                refused += 1;
                continue;
            };
            let ours = oracle_lines(&oracle, exact);
            let theirs = product_lines(&product, exact);
            for (i, line) in AMOUNT_LINES.iter().enumerate() {
                let Some(official) = ours[i] else { continue };
                let mut mine = theirs[i];
                if i == 6 && oracle.exact.unknown & UNKNOWN_STAMP != 0 {
                    mine = mine.minus(theirs[4]);
                }
                let diff = mine.minus(official);
                // Each unrounded product line is half-up to one unit, so the
                // total of six may sit six units away.
                let units = if i == 6 { 6 } else { 1 };
                let off = if exact {
                    diff.magnitude().minus(tolerance().times(Q::int(units))).n > 0
                } else {
                    !diff.is_zero()
                };
                if off {
                    diffs.push(LineDiff {
                        set,
                        segment: case.segment,
                        day,
                        case: case.name,
                        line,
                        diff,
                        product: format!("{mine}"),
                        oracle: format!("{official}"),
                    });
                }
            }
        }
    }
    (diffs, refused)
}

/// Folds line differences into one sentence per (set, segment, line): the
/// date ranges over the grid's own days, the largest difference and an
/// example.
fn summarize(diffs: &[LineDiff], days: &[Date]) -> String {
    let keys: BTreeSet<(Set, Segment, &str)> =
        diffs.iter().map(|d| (d.set, d.segment, d.line)).collect();
    let mut out = String::new();
    for (set, segment, line) in keys {
        let mine: Vec<&LineDiff> = diffs
            .iter()
            .filter(|d| d.set == set && d.segment == segment && d.line == line)
            .collect();
        let hit: BTreeSet<Date> = mine.iter().map(|d| d.day).collect();
        let mut ranges: Vec<(Date, Date)> = Vec::new();
        let mut open: Option<(Date, Date)> = None;
        for &day in days {
            if hit.contains(&day) {
                open = Some(open.map_or((day, day), |(from, _)| (from, day)));
            } else if let Some(range) = open.take() {
                ranges.push(range);
            }
        }
        ranges.extend(open);
        let worst = mine
            .iter()
            .max_by(|a, b| {
                let (x, y) = (a.diff.magnitude(), b.diff.magnitude());
                (x.n * y.d).cmp(&(y.n * x.d))
            })
            .expect("at least one");
        let span = ranges
            .iter()
            .map(|(a, b)| format!("{a}..{b}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            out,
            "[{}] {} | {line} | grid days {span} | {} cases | largest: {} on {} ({}: product {} vs oracle {})",
            set.name(),
            segment.name(),
            mine.len(),
            worst.diff,
            worst.day,
            worst.case,
            worst.product,
            worst.oracle
        );
    }
    out
}

/// The period table for one segment under `set`: on each grid day, the
/// reference trade's total from the product and from the oracle, folded into
/// runs of equal answers; and whether any day's totals differ.
fn period_table(table: &Table, set: Set, segment: Segment, days: &[Date]) -> (String, bool) {
    let case = reference(segment);
    let mut rows: Vec<(Date, Date, String)> = Vec::new();
    let mut differs = false;
    for &day in days {
        let oracle = day_charges(table, segment, day, &case.orders).expect("the oracle prices it");
        let official = oracle_lines(&oracle, false)[6].expect("a total");
        let unknown_stamp = oracle.exact.unknown & UNKNOWN_STAMP != 0;
        let text = match product::day(segment, day, set.selector(), &case.orders) {
            Err(_) => {
                differs = true;
                format!("official {official}; product refuses")
            }
            Ok(p) => {
                let mine = Q::int(p.total - if unknown_stamp { p.stamp } else { 0 });
                let gap = mine.minus(official);
                differs |= !gap.is_zero();
                let tag = if unknown_stamp {
                    " (stamp duty left out of both: no official figure)"
                } else {
                    ""
                };
                format!("official {official} vs product {mine}: product minus official {gap}{tag}")
            }
        };
        match rows.last_mut() {
            Some((_, to, last)) if *last == text => *to = day,
            _ => rows.push((day, day, text)),
        }
    }
    let mut out = format!(
        "[{}] {} reference trade: {}\n",
        set.name(),
        segment.name(),
        case.name
    );
    for (from, to, text) in rows {
        let _ = writeln!(out, "  {from}..{to}: {text}");
    }
    (out, differs)
}

// ---------------------------------------------------------------------------
// Green: the oracle's own consistency.
// ---------------------------------------------------------------------------

#[test]
fn the_fixture_parses_and_every_row_is_complete() {
    let table = table();
    assert_eq!(table.window_from, date("2018-09-01"));
    assert_eq!(table.window_to, date("2026-10-31"));
    assert!(table.rows.len() >= 40, "{} rows", table.rows.len());
    let mut ids = BTreeSet::new();
    for row in &table.rows {
        assert!(ids.insert(row.id.clone()), "duplicate id {}", row.id);
        assert!(
            table.window_from <= row.from && row.to <= table.window_to,
            "{} lies outside the window",
            row.id
        );
        for (what, text) in [
            ("base", &row.base),
            ("value", &row.value),
            ("source", &row.source),
            ("quote", &row.quote),
            ("note", &row.note),
        ] {
            assert!(!text.trim().is_empty(), "{} has an empty {what}", row.id);
        }
        if row.status != Status::Unknown {
            assert!(
                row.source.contains("https://") || row.source.contains("http://"),
                "{} names no URL",
                row.id
            );
        }
        let official_host = [
            "nseindia.com",
            "sebi.gov.in",
            "incometaxindia.gov.in",
            "indiabudget.gov.in",
            "indiacode.nic.in",
            "pib.gov.in",
            "gstcouncil.gov.in",
            "cbic-gst.gov.in",
            "dhan.co",
        ];
        assert!(
            row.status == Status::Unknown
                || official_host.iter().any(|host| row.source.contains(host)),
            "{} cites no official host: {}",
            row.id,
            row.source
        );
    }
}

#[test]
fn every_charge_is_tiled_across_the_window_without_gap_or_overlap() {
    let table = table();
    for charge in Charge::ALL {
        for &segment in charge.segments() {
            let mut sides: Option<BTreeSet<Applies>> = None;
            let mut day = table.window_from;
            while day <= table.window_to {
                let rows = table.matching(charge, segment, day);
                let here: BTreeSet<Applies> = rows.iter().map(|r| r.applies).collect();
                assert_eq!(
                    here.len(),
                    rows.len(),
                    "{charge:?} {} {day}: two rows for one side",
                    segment.name()
                );
                assert!(
                    !rows.is_empty(),
                    "{charge:?} {} {day}: no row",
                    segment.name()
                );
                match &sides {
                    None => sides = Some(here),
                    Some(first) => assert_eq!(
                        first,
                        &here,
                        "{charge:?} {}: the sides change on {day}",
                        segment.name()
                    ),
                }
                day = day.next();
            }
        }
    }
    // And no row names a segment its charge does not cover.
    for row in &table.rows {
        for segment in &row.segments {
            assert!(
                row.charge.segments().contains(segment),
                "{} names {}",
                row.id,
                segment.name()
            );
        }
    }
}

/// The rate of a row as RATES.md shows it.
fn rate_text(row: &Row) -> String {
    match row.charge {
        Charge::BrokerageCap | Charge::BrokeragePerOrder => rupees(row.rate.half_up(1)),
        Charge::GstOnSebiFee if row.rate.is_zero() => "excluded".to_owned(),
        Charge::GstOnSebiFee => "included".to_owned(),
        _ if row.status == Status::Unknown => "UNKNOWN".to_owned(),
        _ => format!("{} ({}/{})", pct(row.rate), row.rate.n, row.rate.d),
    }
}

/// The one line RATES.md carries for a fixture row, every cell from the
/// fixture, so the two cannot drift apart.
fn markdown_row(row: &Row) -> String {
    let cell = |text: &str| text.replace('|', "\\|");
    let segments = row
        .segments
        .iter()
        .map(|s| s.name())
        .collect::<Vec<_>>()
        .join(", ");
    let applies = match row.applies {
        Applies::Buy => "buy",
        Applies::Sell => "sell",
        Applies::Both => "both sides",
        Applies::Exercise => "exercise",
    };
    let status = match row.status {
        Status::Verified => "verified",
        Status::Assumption => "assumption",
        Status::Unverified => "unverified",
        Status::Counterfactual => "counterfactual",
        Status::Unknown => "UNKNOWN",
    };
    format!(
        "| {} | {segments} | {applies} | {} | {} | {} | {} | {} | {status} | {} | \"{}\" | {} |",
        row.id,
        rate_text(row),
        cell(&row.value),
        cell(&row.base),
        row.from,
        row.to,
        cell(&row.source),
        cell(&row.quote),
        cell(&row.note)
    )
}

#[test]
fn rates_md_carries_every_fixture_row_exactly() {
    let table = table();
    let missing: Vec<String> = table
        .rows
        .iter()
        .map(markdown_row)
        .filter(|line| !RATES_MD.lines().any(|md| md == line))
        .collect();
    assert!(
        missing.is_empty(),
        "docs/charges-oracle/RATES.md must carry these lines exactly:\n{}",
        missing.join("\n")
    );
}

#[test]
fn the_fixture_parser_refuses_what_it_does_not_understand() {
    assert!(
        parse_value("\"a \\\"quoted\\\" word\"")
            .is_ok_and(|v| v == Value::Str("a \"quoted\" word".to_owned()))
    );
    assert_eq!(parse_value("1_000"), Ok(Value::Int(1_000)));
    assert_eq!(parse_value("-7"), Ok(Value::Int(-7)));
    for bad in ["\"open", "\"x\" trailing", "1.5", "true", "\"\\n\"", ""] {
        assert!(parse_value(bad).is_err(), "{bad:?} was accepted");
    }
    assert!(parse_fixture("[table]\n").is_err());
    assert!(parse_fixture("Key = 1\n").is_err());
    assert!(parse_fixture("[[rate]]\nid = \"a\"\nid = \"b\"\n").is_err());
    let one = parse_fixture("top = \"x\"\n# comment\n\n[[rate]]\nnum = 3\n").expect("parses");
    assert_eq!(
        one.header,
        vec![("top".to_owned(), Value::Str("x".to_owned()))]
    );
    assert_eq!(one.rows, vec![vec![("num".to_owned(), Value::Int(3))]]);
    assert!(
        row_of(&[
            ("id".to_owned(), Value::Str("x".to_owned())),
            ("extra".to_owned(), Value::Int(1))
        ])
        .is_err()
    );
    assert!(Date::parse("2023-02-29").is_none());
    assert!(Date::parse("2024-02-29").is_some());
    assert!(Date::parse("2024-2-29").is_none());
    assert_eq!(date("2026-10-31").to_string(), "2026-10-31");
    assert_eq!(date("2020-01-01").prev().to_string(), "2019-12-31");
}

#[test]
fn the_oracle_reproduces_hand_worked_examples() {
    let table = table();
    let note = |segment, day: &str, orders: &[OrderIn]| {
        day_charges(&table, segment, date(day), orders)
            .expect("priced")
            .note
    };
    // Index option, 2026-05-15: 75 bought at Rs 100, sold at Rs 120.
    // Exchange: Rs 16,500 x (3,552.99 + 0.01) per crore = Rs 5.86245 -> 5.86.
    // SEBI: Rs 0.0165 -> 0.02. STT: 0.15% x 9,000 = 13.50 -> 14. Stamp:
    // 0.003% x 7,500 = 0.225 -> 0. GST: 18% x 45.88 = 8.2584 -> 8.26.
    let n = note(
        Segment::IndexOption,
        "2026-05-15",
        &[buy(75, 10_000), sell(75, 12_000)],
    );
    assert_eq!(
        n,
        Note {
            brokerage: 4_000,
            exchange: 586,
            sebi: 2,
            stt: 1_400,
            stamp: 0,
            gst: 826,
            total: 6_814
        }
    );
    // Equity intraday, 2026-05-15: 50 at Rs 1,520 / 1,530. VWAP 1,525.00;
    // STT 0.025% x 76,250 = 19.0625 -> 19; stamp 0.003% x 76,000 = 2.28 -> 2;
    // exchange Rs 4.68175 -> 4.68; SEBI 0.1525 -> 0.15; GST 18% x 44.83.
    let n = note(
        Segment::EquityIntraday,
        "2026-05-15",
        &[buy(50, 152_000), sell(50, 153_000)],
    );
    assert_eq!(
        n,
        Note {
            brokerage: 4_000,
            exchange: 468,
            sebi: 15,
            stt: 1_900,
            stamp: 200,
            gst: 807,
            total: 7_390
        }
    );
    // Equity intraday, 2022-01-10: first-slab Rs 3.45 per lakh, SEBI Rs 10 per
    // crore and outside the GST base (before 2022-07-18).
    // Exchange 201,000 x (345/10^7 + 1/10^9) = 6.934701 -> 6.93; GST 18% x
    // 46.93 = 8.4474 -> 8.45; STT 0.025% x 100,500 = 25.125 -> 25.
    let n = note(
        Segment::EquityIntraday,
        "2022-01-10",
        &[buy(100, 100_000), sell(100, 101_000)],
    );
    assert_eq!(
        n,
        Note {
            brokerage: 4_000,
            exchange: 693,
            sebi: 20,
            stt: 2_500,
            stamp: 300,
            gst: 845,
            total: 8_358
        }
    );
}

#[test]
fn the_oracle_reproduces_hand_worked_historical_order_and_exercise_examples() {
    let table = table();
    // Index option, 2019-03-15: STT 0.05% x 9,000 = 4.50 -> 5 (fifty paise
    // go up); SEBI Rs 15 per crore; GST 18% x 48.25 = 8.685 -> 8.69; stamp
    // duty has no official figure that day and is flagged.
    let out = day_charges(
        &table,
        Segment::IndexOption,
        date("2019-03-15"),
        &[buy(75, 10_000), sell(75, 12_000)],
    )
    .expect("priced");
    assert_eq!(out.note.exchange, 825);
    assert_eq!(out.note.sebi, 2);
    assert_eq!(out.note.stt, 500);
    assert_eq!(out.note.gst, 869);
    assert_eq!(out.exact.unknown, UNKNOWN_STAMP);
    // One order on its own: a stock option sold on 2023-06-01, 1,800 at Rs 3.45.
    let lines = order_charges(
        &table,
        Segment::StockOption,
        date("2023-06-01"),
        OrderIn::lots(Side::Sell, 1, 1_800, 345),
    )
    .expect("priced");
    assert!(lines.brokerage.same_as(Q::int(2_000)));
    assert!(
        lines.exchange_txn.same_as(Q::new(3_105, 10)),
        "{}",
        lines.exchange_txn
    );
    assert!(lines.ipft.same_as(Q::new(3_105, 1_000)), "{}", lines.ipft);
    assert!(lines.stt.same_as(Q::new(388_125, 1_000)), "{}", lines.stt);
    assert!(lines.stamp.is_zero());
    assert!(lines.sebi.same_as(Q::new(621, 1_000)), "{}", lines.sebi);
    // GST 18% x (2,000 + 310.5 + 3.105 + 0.621) paisa.
    assert!(
        lines
            .gst
            .same_as(Q::new(18, 100).times(Q::new(2_314_226, 1_000))),
        "{}",
        lines.gst
    );
    assert!(
        lines.total().same_as(
            Q::new(2_000 * 1_000 + 310_500 + 3_105 + 621 + 388_125, 1_000).plus(lines.gst)
        )
    );
    // Exercise: a NIFTY call, strike 11,800, settled at 11,850, 75 units.
    let ex = Exercise {
        settlement: 1_185_000,
        strike: 1_180_000,
        quantity: 75,
    };
    let stt = |day: &str| {
        exercise_stt(&table, Segment::IndexOption, date(day), ex)
            .expect("priced")
            .1
    };
    assert_eq!(stt("2019-06-27"), 111_100); // 0.125% x 8,88,750 = 1,110.9375
    assert_eq!(stt("2019-09-26"), 500); // 0.125% x 3,750 = 4.6875
    assert_eq!(stt("2026-06-25"), 600); // 0.15% x 3,750 = 5.625
    assert!(exercise_stt(&table, Segment::StockOption, date("2026-06-25"), ex).is_err());
}

#[test]
fn the_order_oracle_agrees_with_a_one_order_day() {
    let table = table();
    for case in all_cases().iter().filter(|c| c.segment.is_option()) {
        for day in [date("2019-03-15"), date("2023-06-01"), date("2026-10-30")] {
            let mut sum: Option<Lines> = None;
            for &order in &case.orders {
                let lines = order_charges(&table, case.segment, day, order).expect("priced");
                sum = Some(match sum {
                    None => lines,
                    Some(s) => Lines {
                        brokerage: s.brokerage.plus(lines.brokerage),
                        exchange_txn: s.exchange_txn.plus(lines.exchange_txn),
                        ipft: s.ipft.plus(lines.ipft),
                        clearing: s.clearing.plus(lines.clearing),
                        sebi: s.sebi.plus(lines.sebi),
                        stt: s.stt.plus(lines.stt),
                        stamp: s.stamp.plus(lines.stamp),
                        gst: s.gst.plus(lines.gst),
                        unknown: s.unknown | lines.unknown,
                    },
                });
            }
            let sum = sum.expect("a case has orders");
            let day_out = day_charges(&table, case.segment, day, &case.orders).expect("priced");
            let e = day_out.exact;
            for (name, a, b) in [
                ("brokerage", sum.brokerage, e.brokerage),
                ("exchange", sum.exchange_txn, e.exchange_txn),
                ("ipft", sum.ipft, e.ipft),
                ("sebi", sum.sebi, e.sebi),
                ("stt", sum.stt, e.stt),
                ("stamp", sum.stamp, e.stamp),
            ] {
                assert!(a.same_as(b), "{} {day} {name}: {a} vs {b}", case.name);
            }
            assert_eq!(sum.unknown, e.unknown);
        }
    }
}

// ---------------------------------------------------------------------------
// Green: where the crate and the oracle agree.
// ---------------------------------------------------------------------------

/// The first day every rate the crate holds is in force.
fn current_regime_start() -> Date {
    date("2026-04-01")
}

#[test]
fn the_crate_holds_the_official_rate_for_every_component_from_2026_04_01() {
    let table = table();
    let from = current_regime_start();
    // A figure no official source was reached for (the STT on a stock option
    // settled by delivery) cannot be checked here; it is reported by
    // `finding_current_rates_charge_todays_exercise_stt_on_every_earlier_date`.
    let bad: Vec<String> = card_runs(&table)
        .iter()
        .filter(|r| r.to >= from && matches!(r.answer, Answer::Differs(..) | Answer::Refused))
        .filter(|r| !(r.set == Set::Dated && r.segment == Segment::EquityIntraday))
        .map(ToString::to_string)
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn the_dated_selector_never_prices_a_rate_that_differs_from_the_official_one() {
    let table = table();
    let bad: Vec<String> = card_runs(&table)
        .iter()
        .filter(|r| r.set == Set::Dated && r.differs())
        .map(ToString::to_string)
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn the_precise_stack_matches_the_oracle_line_by_line_unrounded_from_2026_04_01() {
    let table = table();
    let runs = card_runs(&table);
    let days: Vec<Date> = grid_days(&table, &runs)
        .into_iter()
        .filter(|&d| d >= current_regime_start())
        .collect();
    for set in [Set::Current, Set::Dated] {
        let cases: Vec<Case> = all_cases()
            .into_iter()
            .filter(|c| set == Set::Current || c.segment.is_option())
            .collect();
        let (diffs, refused) = compare_grid(&table, set, &days, &cases, true);
        assert_eq!(refused, 0, "{} refused a current-regime day", set.name());
        assert!(diffs.is_empty(), "{}", summarize(&diffs, &days));
    }
}

#[test]
fn the_dated_precise_stack_matches_the_oracle_unrounded_on_every_day_it_prices() {
    let table = table();
    let runs = card_runs(&table);
    let days = grid_days(&table, &runs);
    let cases: Vec<Case> = all_cases()
        .into_iter()
        .filter(|c| c.segment.is_option())
        .collect();
    let (diffs, refused) = compare_grid(&table, Set::Dated, &days, &cases, true);
    assert!(diffs.is_empty(), "{}", summarize(&diffs, &days));
    // It prices from 2024-10-01 and refuses every grid day before.
    let before = days.iter().filter(|&&d| d < date("2024-10-01")).count();
    assert_eq!(refused, before * cases.len());
}

#[test]
fn dhan_brokerage_matches_the_oracle_at_and_around_the_cap() {
    let table = table();
    let day = date("2026-10-30");
    // Order values in paisa: the 0.03% reaches Rs 20 at Rs 66,666.67.
    for value in [
        100,
        9_995,
        6_660_000,
        6_664_995,
        6_665_000,
        6_666_665,
        6_666_667,
        6_666_700,
        100_000_000,
        50_000_000_000,
    ] {
        let oracle = brokerage(&table, Segment::EquityIntraday, day, value)
            .expect("priced")
            .half_up(1);
        let crate_figure = product::brokerage(Segment::EquityIntraday, value).expect("priced");
        assert_eq!(crate_figure, oracle, "order value {}", rupees(value));
    }
    for segment in [Segment::StockOption, Segment::IndexOption] {
        for value in [5, 375, 1_000_000_000] {
            assert_eq!(product::brokerage(segment, value), Ok(2_000));
            assert_eq!(
                brokerage(&table, segment, day, value)
                    .expect("priced")
                    .half_up(1),
                2_000
            );
        }
    }
}

#[test]
fn the_trip_pricers_dated_rates_match_the_oracle_wherever_it_prices() {
    let table = table();
    let bad: Vec<String> = regime_runs(&table)
        .iter()
        .filter(|r| r.from >= date("2024-10-01") && r.answer != Answer::Same)
        .map(ToString::to_string)
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

// ---------------------------------------------------------------------------
// Findings: each fails when run, and its message is the finding.
// ---------------------------------------------------------------------------

/// Every card run of `set` for `component` that does not match, as text.
fn mismatched_runs(runs: &[Run], set: Set, component: usize) -> Vec<String> {
    runs.iter()
        .filter(|r| {
            r.set == set && r.component == COMPONENTS[component] && r.answer != Answer::Same
        })
        .map(ToString::to_string)
        .collect()
}

/// Fails with every run of `component` under today's-rates-on-every-date
/// that differs from the official rate, or cannot be checked.
fn assert_current_component(component: usize) {
    let table = table();
    let bad = mismatched_runs(&card_runs(&table), Set::Current, component);
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
#[ignore = "finding: today's options STT applied to dates before 2026-04-01"]
fn finding_current_rates_charge_todays_option_stt_on_every_earlier_date() {
    assert_current_component(4);
}

#[test]
#[ignore = "finding: today's exchange charge applied to dates before 2024-10-01"]
fn finding_current_rates_charge_todays_exchange_charge_on_every_earlier_date() {
    assert_current_component(0);
}

#[test]
#[ignore = "finding: the SEBI fee was Rs 15 then Rs 5 per crore in parts of the window"]
fn finding_current_rates_charge_rs_10_sebi_fee_where_it_was_15_or_5() {
    assert_current_component(1);
}

#[test]
#[ignore = "finding: GST on the SEBI fee before 2022-07-18"]
fn finding_current_rates_charge_gst_on_the_sebi_fee_before_2022_07_18() {
    assert_current_component(3);
}

#[test]
#[ignore = "finding: today's exercise STT applied to dates before 2026-04-01"]
fn finding_current_rates_charge_todays_exercise_stt_on_every_earlier_date() {
    assert_current_component(5);
}

#[test]
#[ignore = "finding: stamp duty before 2020-07-01 had no uniform official rate"]
fn finding_current_rates_charge_uniform_stamp_duty_before_2020_07_01() {
    assert_current_component(6);
}

#[test]
#[ignore = "finding: what today's rates on every date cost each reference trade, period by period"]
fn finding_current_rates_reference_trades_period_by_period() {
    let table = table();
    let runs = card_runs(&table);
    let days = grid_days(&table, &runs);
    let mut out = String::new();
    let mut differs = false;
    for segment in Segment::ALL {
        let (text, any) = period_table(&table, Set::Current, segment, &days);
        differs |= any;
        out.push_str(&text);
    }
    assert!(!differs, "{out}");
}

#[test]
#[ignore = "finding: every grid day and line on which today's rates on every date miss the official amount"]
fn finding_current_rates_grid_amounts_unrounded() {
    let table = table();
    let runs = card_runs(&table);
    let days = grid_days(&table, &runs);
    let (diffs, refused) = compare_grid(&table, Set::Current, &days, &all_cases(), true);
    assert_eq!(refused, 0);
    assert!(diffs.is_empty(), "{}", summarize(&diffs, &days));
}

#[test]
#[ignore = "finding: the dated selector refuses dates for which official rates exist"]
fn finding_dated_selector_refuses_dates_with_official_rates() {
    let table = table();
    let bad: Vec<String> = card_runs(&table)
        .iter()
        .filter(|r| {
            r.set == Set::Dated && r.answer == Answer::Refused && r.component == COMPONENTS[0]
        })
        .map(ToString::to_string)
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
#[ignore = "finding: the dated regime's own lookups before 2024-10-01"]
fn finding_regime_lookups_differ_from_or_refuse_official_rates() {
    let table = table();
    let bad: Vec<String> = regime_runs(&table)
        .iter()
        .filter(|r| r.answer != Answer::Same)
        .map(ToString::to_string)
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
#[ignore = "finding: exercise STT before 2019-09-01 was on the settlement value"]
fn finding_exercise_stt_before_2019_09_01_was_on_the_settlement_value() {
    let table = table();
    let ex = Exercise {
        settlement: 1_185_000,
        strike: 1_180_000,
        quantity: 75,
    };
    let mut out = String::new();
    for day in [
        "2018-09-27",
        "2019-08-29",
        "2019-09-26",
        "2026-03-26",
        "2026-04-30",
        "2026-10-27",
    ] {
        let day = date(day);
        let (_, official) = exercise_stt(&table, Segment::IndexOption, day, ex).expect("priced");
        for set in [Set::Current, Set::Dated] {
            let text = match product::exercise(day, set.selector(), ex.intrinsic()) {
                Err(_) => "refuses".to_owned(),
                Ok(p) if p.stt_exercise == official => continue,
                Ok(p) => format!(
                    "{} (product minus official {})",
                    rupees(p.stt_exercise),
                    rupees(p.stt_exercise - official)
                ),
            };
            let _ = writeln!(
                out,
                "{day} [{}] NIFTY call 11,800 settled 11,850 x 75: official STT {} | product {text}",
                set.name(),
                rupees(official)
            );
        }
    }
    assert!(out.is_empty(), "{out}");
}

#[test]
#[ignore = "finding: the SEBI fee on an exercised option's notional value is not priced"]
fn finding_sebi_fee_on_exercised_notional_value_is_not_priced() {
    // "in case where the option is exercised or assigned, it shall be
    // additionally computed on the basis of notional value of option
    // contracts exercised or assigned" (SEBI Stock Brokers Regulations
    // schedule). The crate prices no SEBI fee on an exercise.
    let table = table();
    let day = date("2026-06-25");
    let ex = Exercise {
        settlement: 2_500_000,
        strike: 2_490_000,
        quantity: 65,
    };
    let p = product::exercise(day, Set::Current.selector(), ex.intrinsic()).expect("priced");
    let rate = table
        .rate(Charge::SebiFee, Segment::IndexOption, None, day)
        .expect("a rate");
    let on_settlement = rate.times(Q::int(ex.settlement_value()));
    assert!(
        p.sebi > 0,
        "an exercised NIFTY call, 65 units settled at Rs 25,000: the crate's SEBI line is {}; on the settlement value as notional the fee is {} plus 18% GST",
        rupees(p.sebi),
        on_settlement
    );
}

#[test]
#[ignore = "finding: the Dhan calculator replica's lines against the official method"]
fn finding_dhan_calculator_replica_differs_from_the_official_method() {
    let table = table();
    let day = date("2026-10-30");
    let mut diffs = Vec::new();
    for case in all_cases() {
        let Some((qty, buy_price, sell_price)) = case.round_trip() else {
            continue;
        };
        let oracle = day_charges(&table, case.segment, day, &case.orders).expect("priced");
        let quote = product::dhan_quote(case.segment, qty, buy_price, sell_price).expect("quoted");
        let n = oracle.note;
        for (line, mine, official) in [
            ("brokerage", quote.brokerage, n.brokerage),
            ("exchange charge incl. IPFT", quote.exchange, n.exchange),
            ("SEBI fee", quote.sebi, n.sebi),
            ("STT", quote.stt, n.stt),
            ("stamp duty", quote.stamp, n.stamp),
            ("GST", quote.gst, n.gst),
            ("total", quote.total, n.total),
        ] {
            if mine != official {
                diffs.push(LineDiff {
                    set: Set::Current,
                    segment: case.segment,
                    day,
                    case: case.name,
                    line,
                    diff: Q::int(mine - official),
                    product: rupees(mine),
                    oracle: rupees(official),
                });
            }
        }
    }
    let mut out = String::new();
    for d in &diffs {
        let _ = writeln!(
            out,
            "{} | {} | {}: calculator {} vs official {} (calculator minus official {})",
            d.segment.name(),
            d.case,
            d.line,
            d.product,
            d.oracle,
            d.diff
        );
    }
    assert!(
        diffs.is_empty(),
        "Dhan calculator replica (crates/costs/src/dhan_calc.rs) on {day}:\n{out}"
    );
}

#[test]
#[ignore = "finding: the round-trip pricer rounds every levy upward"]
fn finding_trip_pricer_rounds_levies_upward() {
    let table = table();
    let mut out = String::new();
    let mut worst = 0;
    for day in [
        "2024-10-01",
        "2025-06-12",
        "2026-02-27",
        "2026-03-02",
        "2026-04-01",
        "2026-10-30",
    ] {
        let day = date(day);
        for case in option_cases(Segment::IndexOption) {
            let Some((qty, buy_price, sell_price)) = case.round_trip() else {
                continue;
            };
            let t = product::trip(day, qty, buy_price, sell_price).expect("priced");
            let orders = [buy(qty, t.buy_fill), sell(qty, t.sell_fill)];
            let n = day_charges(&table, Segment::IndexOption, day, &orders)
                .expect("priced")
                .note;
            let lines = [
                ("brokerage", t.brokerage, n.brokerage),
                (
                    "exchange charge incl. IPFT",
                    t.exchange + t.ipft,
                    n.exchange,
                ),
                ("SEBI fee", t.sebi, n.sebi),
                ("STT", t.stt, n.stt),
                ("stamp duty", t.stamp, n.stamp),
                ("GST", t.gst, n.gst),
                ("total", t.total, n.total),
            ];
            for (line, mine, official) in lines {
                if mine != official {
                    if line == "total" {
                        worst = worst.max(mine - official);
                    }
                    let _ = writeln!(
                        out,
                        "{day} | {} (fills {} / {}) | {line}: trip {} vs official {}",
                        case.name,
                        rupees(t.buy_fill),
                        rupees(t.sell_fill),
                        rupees(mine),
                        rupees(official)
                    );
                }
            }
        }
    }
    assert!(
        out.is_empty(),
        "largest total overcharge {}:\n{out}",
        rupees(worst)
    );
}

#[test]
#[ignore = "open question: the rounding order of STT and stamp duty at half-paisa edges"]
fn finding_precise_stack_rounds_stt_and_stamp_to_the_paisa_before_the_rupee() {
    // Rounded lines where the rates agree: today's rates from 2026-04-01, and
    // the dated rates on every day they price. Rates and bases are proven
    // equal unrounded by the green tests above, so what remains is rounding.
    let table = table();
    let runs = card_runs(&table);
    let all = grid_days(&table, &runs);
    let current: Vec<Date> = all
        .iter()
        .copied()
        .filter(|&d| d >= current_regime_start())
        .collect();
    let options: Vec<Case> = all_cases()
        .into_iter()
        .filter(|c| c.segment.is_option())
        .collect();
    let (mut diffs, _) = compare_grid(&table, Set::Current, &current, &all_cases(), false);
    diffs.extend(compare_grid(&table, Set::Dated, &all, &options, false).0);
    assert!(diffs.is_empty(), "{}", summarize(&diffs, &all));
}
