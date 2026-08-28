//! Vendor CSV rows into [`crate::fetch::RawRow`]s, driven by the descriptor.
//!
//! # Why this is not "a CSV parser"
//!
//! Every shape here was read out of the operator's own purchased archives, not
//! taken from a vendor's documentation. The documentation and the files
//! disagree, and where they do, **the files win**.
//!
//! | Vendor | Segment | Columns | Header | Date |
//! |---|---|---|---|---|
//! | `TrueData` | index | **5** | none | `YYYYMMDD` |
//! | `TrueData` | futures | **9** | none | `YYYYMMDD` |
//! | GDFL | options, futures | **10** | present | **`DD/MM/YYYY`** |
//!
//! Two things in that table are the whole reason this module exists.
//!
//! **The column count varies by segment inside one vendor.** `TrueData` emits
//! five columns for an index and nine for a future, in the same archive on the
//! same day. An index has no volume and no open interest, so those fields are
//! *structurally absent* rather than zero. A single per-vendor layout would
//! mis-parse one of the two, and the failure is silent: a price column read as
//! a volume yields a plausible number.
//!
//! **GDFL dates are `DD/MM/YYYY`.** `01/07/2025` is 1 July, not 7 January.
//! Reading it the other way shifts every bar by months and produces a file that
//! is internally consistent and completely wrong.
//!
//! # `__MACOSX` is not data
//!
//! `GDFL.zip` lists 24,292 entries, of which **12,145 are `__MACOSX`** — the
//! shadow tree macOS writes when it re-zips, one `AppleDouble` stub per real
//! file. They end in `.csv` and they are binary. A reader that globs `*.csv`
//! will open them and try to parse a resource fork as text.
//!
//! This was found by getting a count wrong: `docs/08-vendor-samples.md` said
//! GDFL held 24,264 CSVs when it holds 12,133, because the ghosts were counted
//! as data. [`is_ghost`] is the same rule applied where it matters.
//!
//! # Cost
//!
//! One pass per line, splitting on a byte. No allocation per row: fields are
//! borrowed from the input and parsed into integers in place. The row vector is
//! reserved from a caller-supplied bound — `docs/07-o1-architecture.md` law 2.

use crate::fetch::{FetchError, MAX_ROWS, RawRow};
use crate::vendor::DateFormat;

/// Which columns a vendor's CSV carries, in order.
///
/// One variant per shape actually observed in the operator's archives. A shape
/// nobody has read is not here, and adding one means reading a file first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Columns {
    /// `date, time, price, volume, open_interest` — five fields.
    ///
    /// `TrueData`'s index layout. Volume and open interest are present in the
    /// row and always zero, because an index has neither. Observed:
    /// `20221003,09:07:41,38444.90,0,0`.
    TrueDataIndex,
    /// `date, time, price, volume, open_interest, …` — nine fields.
    ///
    /// `TrueData`'s futures layout. The trailing four carry bid/ask depth on
    /// the `TICK_BA` products.
    TrueDataFutures,
    /// `Ticker, Date, Time, LTP, BuyPrice, BuyQty, SellPrice, SellQty, LTQ,
    /// OpenInterest` — ten fields, with a header row.
    ///
    /// GDFL's layout for both options and futures. `LTQ` is `0` on most rows:
    /// those are **quote** updates, not trades.
    Gdfl,
}

impl Columns {
    /// How many fields a row of this shape has.
    #[must_use]
    pub const fn count(self) -> usize {
        match self {
            Self::TrueDataIndex => 5,
            Self::TrueDataFutures => 9,
            Self::Gdfl => 10,
        }
    }

    /// Whether the file opens with a header row naming the columns.
    #[must_use]
    pub const fn has_header(self) -> bool {
        matches!(self, Self::Gdfl)
    }

    /// The date format this shape carries.
    #[must_use]
    pub const fn date_format(self) -> DateFormat {
        match self {
            Self::TrueDataIndex | Self::TrueDataFutures => DateFormat::CompactYmd,
            Self::Gdfl => DateFormat::SlashedDmy,
        }
    }

    /// Zero-based index of date, time, price, volume and open interest.
    ///
    /// **The last two were missing, and every bar written before this was
    /// wrong.** The decoder read three columns and hardcoded `volume: 0,
    /// open_interest: None`, so a contract that traded 250 lots was stored
    /// asserting it traded none — with a valid checksum, undetectable
    /// afterwards. Verified against `ABB-III.NFO.csv`, whose 11:50 bucket
    /// carries volume 250 and open interest 2,000 and reached disk as `0` and
    /// `i64::MIN`.
    const fn offsets(self) -> Offsets {
        match self {
            Self::TrueDataIndex | Self::TrueDataFutures => Offsets {
                date: 0,
                time: 1,
                price: 2,
                volume: 3,
                open_interest: 4,
            },
            // Ticker, Date, Time, LTP, BuyPrice, BuyQty, SellPrice, SellQty,
            // LTQ, OpenInterest. LTQ is the traded size — column 9.
            Self::Gdfl => Offsets {
                date: 1,
                time: 2,
                price: 3,
                volume: 8,
                open_interest: 9,
            },
        }
    }
}

/// Which column each field sits in, for one layout.
///
/// A struct rather than five positional `usize`: five adjacent numbers of the
/// same type transpose without a compiler complaint, and a volume index read as
/// a price index yields a plausible number. The same reasoning as
/// `ingest::Plan` and `api::render::View`.
#[derive(Debug, Clone, Copy)]
struct Offsets {
    date: usize,
    time: usize,
    price: usize,
    volume: usize,
    open_interest: usize,
}

/// Why a CSV line is not a row.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CsvError {
    /// The line has the wrong number of fields.
    FieldCount {
        /// One-based line number within the file.
        line: usize,
        /// How many fields were found.
        got: usize,
        /// How many the declared shape has.
        want: usize,
    },
    /// A date field is not the declared format.
    DateMalformed {
        /// One-based line number.
        line: usize,
        /// What was there.
        got: String,
        /// The format that was expected.
        format: DateFormat,
    },
    /// A time field is not `HH:MM:SS`.
    TimeMalformed {
        /// One-based line number.
        line: usize,
        /// What was there.
        got: String,
    },
    /// A price is not a decimal this build can put on the paisa grid.
    PriceMalformed {
        /// One-based line number.
        line: usize,
        /// What was there.
        got: String,
    },
    /// An open-interest field carries the value §7 spends on ABSENCE.
    ///
    /// `CLAUDE.md` §7 makes `i64::MIN` the open-interest null sentinel, and
    /// [`crate::fetch::land`] performs that substitution one layer down —
    /// `row.open_interest.unwrap_or(i64::MIN)`. A vendor that literally sends
    /// -9223372036854775808 would therefore land as the null and be
    /// indistinguishable from a vendor that sent no open interest at all,
    /// which is the silent substitution §4 bans. The decoder is the last place
    /// the two are still different values, so it is where they are told apart.
    OpenInterestSentinel {
        /// One-based line number.
        line: usize,
        /// What was there, as the file spells it.
        got: String,
    },
    /// More rows than [`MAX_ROWS`].
    TooManyRows {
        /// How many were found before stopping.
        rows: usize,
        /// The bound.
        cap: usize,
    },
}

impl core::fmt::Display for CsvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::FieldCount { line, got, want } => write!(
                f,
                "line {line}: {got} fields, expected {want}. The column layout \
                 is declared per (vendor, segment) because one vendor emits \
                 five for an index and nine for a future."
            ),
            Self::DateMalformed {
                line,
                ref got,
                format,
            } => write!(f, "line {line}: date {got:?} is not {format:?}"),
            Self::TimeMalformed { line, ref got } => {
                write!(f, "line {line}: time {got:?} is not HH:MM:SS")
            }
            Self::PriceMalformed { line, ref got } => {
                write!(f, "line {line}: price {got:?} is not a decimal")
            }
            Self::OpenInterestSentinel { line, ref got } => write!(
                f,
                "line {line}: open interest {got:?} is the null sentinel — \
                 `CLAUDE.md` §7 spends i64::MIN on an ABSENT open interest, so \
                 a vendor that sends it could not be told apart from one that \
                 sent no open interest at all"
            ),
            Self::TooManyRows { rows, cap } => {
                write!(f, "the file holds at least {rows} rows; the cap is {cap}")
            }
        }
    }
}

impl core::error::Error for CsvError {}

impl From<CsvError> for FetchError {
    fn from(why: CsvError) -> Self {
        Self::TransportFailed {
            detail: why.to_string(),
        }
    }
}

/// Whether an archive member is a macOS resource-fork ghost rather than data.
///
/// `GDFL.zip` holds 12,145 of these against 12,133 real CSVs. They end in
/// `.csv`, they are binary, and a reader that globs by extension will try to
/// parse one as text.
///
/// # Examples
///
/// ```
/// # use pull::csv::is_ghost;
/// assert!(is_ghost("__MACOSX/GFDLNFO/Options/._NIFTY25SEP2525700PE.NFO.csv"));
/// assert!(is_ghost("GFDLNFO/Options/._NIFTY.csv"), "the AppleDouble prefix alone");
/// assert!(!is_ghost("GFDLNFO_TICK_01072025/Options/NIFTY25SEP2525700PE.NFO.csv"));
/// ```
#[must_use]
pub fn is_ghost(member: &str) -> bool {
    member.contains("__MACOSX")
        || member
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("._") || name == ".DS_Store")
}

/// Parses a decimal price into paisa, exactly.
///
/// Rejects a third decimal place rather than rounding it: the tick grid is two
/// places (`CLAUDE.md` §7), so a third digit is the vendor sending something
/// this build does not understand, and rounding it here would be a second
/// snapping site competing with the one at the write boundary.
pub(crate) fn paisa(text: &str) -> Option<i64> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    if frac.len() > 2 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let negative = whole.starts_with('-');
    let digits = whole.strip_prefix('-').unwrap_or(whole);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let rupees: i64 = digits.parse().ok()?;
    // "38444.9" is nine tenths, not nine hundredths — pad on the right.
    let hundredths: i64 = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        _ => frac.parse().ok()?,
    };
    let total = rupees.checked_mul(100)?.checked_add(hundredths)?;
    Some(if negative { -total } else { total })
}

/// Seconds since IST midnight, from `HH:MM:SS`.
fn ist_seconds(text: &str) -> Option<i64> {
    let mut parts = text.split(':');
    let (h, m, s) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || h.len() != 2 || m.len() != 2 || s.len() != 2 {
        return None;
    }
    let (h, m, s): (i64, i64, i64) = (h.parse().ok()?, m.parse().ok()?, s.parse().ok()?);
    if h > 23 || m > 59 || s > 59 {
        return None;
    }
    Some(h * 3_600 + m * 60 + s)
}

/// A calendar date from a vendor's date field.
fn day_of(text: &str, format: DateFormat) -> Option<crate::session::Day> {
    let (y, m, d) = match format {
        // `20221003`
        DateFormat::CompactYmd if text.len() == 8 => (
            text.get(0..4)?.parse().ok()?,
            text.get(4..6)?.parse().ok()?,
            text.get(6..8)?.parse().ok()?,
        ),
        // `2022-10-03`
        DateFormat::DashedYmd if text.len() == 10 => (
            text.get(0..4)?.parse().ok()?,
            text.get(5..7)?.parse().ok()?,
            text.get(8..10)?.parse().ok()?,
        ),
        // `01/07/2025` — DAY first. 1 July, not 7 January.
        DateFormat::SlashedDmy if text.len() == 10 => (
            text.get(6..10)?.parse().ok()?,
            text.get(3..5)?.parse().ok()?,
            text.get(0..2)?.parse().ok()?,
        ),
        // `01072025`
        DateFormat::CompactDmy if text.len() == 8 => (
            text.get(4..8)?.parse().ok()?,
            text.get(2..4)?.parse().ok()?,
            text.get(0..2)?.parse().ok()?,
        ),
        _ => return None,
    };
    crate::session::Day::new(y, m, d).ok()
}

/// What one pass over a body counted, in plain integers.
///
/// A struct rather than two adjacent `u64`, for [`Offsets`]' reason: two
/// numbers of the same type transpose without a compiler complaint, and a
/// skipped count reported as a line count is a plausible number.
///
/// Neither counter is an atomic and neither allocates. They are incremented in
/// the per-row loop and read once, after it, which is the only way a per-row
/// fact reaches the log at all — see [`note_decoded`].
#[derive(Debug, Clone, Copy)]
struct Tally {
    /// Every line the pass looked at, blank ones and the header included.
    lines: u64,
    /// The blank lines, plus the header row when the shape declares one.
    skipped: u64,
    /// Rows whose volume field would not parse and were stored as `0`.
    ///
    /// # Why a count and not a refusal
    ///
    /// `CLAUDE.md` §4 allows a degrade OR a refusal — "degrade loudly and name
    /// the reason, or refuse. Never both silently." The substitution below is
    /// the degrade, and until this counter existed it was the *silently*.
    ///
    /// The substitution is also inconsistent with its own neighbour, which is
    /// how it was found: the comment beside it argues that `None` and `Some(0)`
    /// are different facts for open interest — "zero open interest is a
    /// measurement, an unreadable field is not" — and then stores an unreadable
    /// VOLUME as the measurement `0`. `RawRow::volume` is `i64` and not
    /// `Option<i64>`, so absence cannot be expressed there without a type
    /// change that reaches the store format; making it visible is what this
    /// crate can do without changing what lands.
    ///
    /// Two plain integers in the row loop, emitted once where the file ends —
    /// the same shape gate 17 prescribes and the same one `lines` and `skipped`
    /// already use.
    unreadable_volume: u64,
    /// Rows whose open-interest field would not parse and were stored absent.
    ///
    /// Absence here is CORRECT — see [`Self::unreadable_volume`] — but it is
    /// indistinguishable on the wire from a vendor that simply sends no open
    /// interest, and those are different facts about the feed.
    unreadable_open_interest: u64,
    /// Rows whose volume field parsed as a NEGATIVE count and were skipped.
    ///
    /// # The third door, and it was the one still open
    ///
    /// A volume counts shares traded, so a negative one is not a quantity —
    /// D-0323 on the JSON path, and P-64 turned that refusal from a whole
    /// window into a single row. This decoder had neither: `parse::<i64>()`
    /// accepts a leading minus, so the operator's measured `-125` would have
    /// parsed cleanly here, landed, and died ~1,500 lines downstream at
    /// `store::file::survey` as `ImpossibleCount` — with a batch index that
    /// names no line of the file it came from.
    ///
    /// **Skipped, never stored as `0`.** Its neighbour
    /// [`Self::unreadable_volume`] substitutes a zero because an unreadable
    /// field cannot be expressed as absence in `RawRow::volume`. That reasoning
    /// does not carry here: a field that PARSED and said `-125` is the vendor
    /// stating something impossible, and writing `0` beside it would be
    /// asserting no trade in a minute we have no reading for.
    negative_volume: u64,
}

/// One decoded file, on the rolling log.
///
/// # Why the count is here and not in the loop
///
/// [`decode_rows`] runs once per ROW. A one-minute backfill is millions of
/// them against a sink that keeps 64 MiB, so a line each would roll the run's
/// own beginning out of the window before the run finished — the evidence
/// would destroy itself. [`Tally`] is two plain integers incremented in that
/// pass and this is the single event they pay for, once, where the file ends.
/// The same shape as [`crate::fetch::land`]'s census.
///
/// # Why three counts and not one
///
/// `rows_in`, `rows` and `skipped` must add up. A body whose lines exceed its
/// rows plus its skips is a decoder that dropped something without saying so,
/// and until this line existed there was no way to see that from outside — a
/// file that decoded to half its rows and a file that decoded to all of them
/// were the same silence.
///
/// # `fields` and `header` are the SHAPE, and the shape is what mis-parses
///
/// The three layouts this module knows are five-without-header,
/// nine-without-header and ten-with-header, so those two numbers name which
/// one was applied. A `TrueData` futures file decoded under the index layout
/// is the failure the module doc opens with — a price column read as a volume
/// yields a plausible number — and it is now visible on the line rather than
/// only in the bars weeks later.
fn note_decoded(columns: Columns, tally: Tally, rows: usize) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::debug("pull.csv", "file decoded")
            .with("rows_in", telemetry::Value::Uint(tally.lines))
            .with("rows", telemetry::Value::Uint(rows as u64))
            .with("skipped", telemetry::Value::Uint(tally.skipped))
            .with("fields", telemetry::Value::Uint(columns.count() as u64))
            .with("header", telemetry::Value::Bool(columns.has_header()))
            // THE SUBSTITUTIONS, ON THE SAME LINE AS THE COUNTS THEY QUALIFY. A
            // `rows` figure that includes rows whose volume this build invented
            // is not the same fact as one where every field was read, and until
            // these two appeared there was no way to tell those apart from
            // outside -- which is the silence §4 forbids, not the substitution
            // itself.
            .with(
                "unreadable_volume",
                telemetry::Value::Uint(tally.unreadable_volume),
            )
            .with(
                "unreadable_oi",
                telemetry::Value::Uint(tally.unreadable_open_interest),
            )
            // SKIPPED ROWS ARE NOT SUBSTITUTIONS AND GET THEIR OWN FIELD. The
            // two above qualify a `rows` figure that still counts them; this
            // one explains why `rows` is SHORT of `rows_in`, which is a
            // different question and an operator asking it should not have to
            // subtract the other two to answer it.
            .with(
                "negative_volume",
                telemetry::Value::Uint(tally.negative_volume),
            ),
    );
}

/// One file that did not decode, on the rolling log — at `Warn`.
///
/// # There is no per-row refusal count to report, and that is the design
///
/// A malformed line refuses the **whole file**, so the tally of refused rows
/// is never between zero and all of them: it is one file, and the reason is
/// the decoder's own words, which name the line. `rows_in` says how far the
/// pass got before it stopped, which is the one number the error itself does
/// not carry in a form a consumer can filter on.
///
/// `why` is the [`CsvError`]'s rendering, whose facts are at the front —
/// `line 4: 3 fields, expected 5` — so the 128-byte ceiling on a string field
/// cuts the essay that follows it and never the numbers.
///
/// `Warn` rather than `Error`: whether a refused file ends the run is the
/// caller's decision, not this module's. [`crate::archive::read_dir`] does end
/// it, and says so at `Error` for the whole walk.
fn note_refused(columns: Columns, tally: Tally, why: &CsvError) {
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::warn("pull.csv", "file refused")
            .with("rows_in", telemetry::Value::Uint(tally.lines))
            .with("skipped", telemetry::Value::Uint(tally.skipped))
            .with("fields", telemetry::Value::Uint(columns.count() as u64))
            .with("header", telemetry::Value::Bool(columns.has_header()))
            .with("why", telemetry::Value::Str(&why.to_string())),
    );
}

/// Decodes a whole CSV body into rows.
///
/// Timestamps come out as **UTC epoch seconds**, so the result feeds
/// [`crate::fetch::land`] with [`crate::vendor::TimestampEncoding::EpochSecondsUtc`]
/// — the vendor's IST wall clock is converted here, once, where the format is
/// known, rather than being carried onward as an encoding somebody downstream
/// has to remember.
///
/// # Errors
///
/// Any [`CsvError`]. A malformed line refuses the **whole file**: a file
/// missing an arbitrary subset of its rows is not a shorter file, it is a
/// wrong one, and the manifest would record it as complete.
///
/// # Examples
///
/// ```
/// # use pull::csv::{decode, Columns};
/// // TrueData index: five fields, no header, YYYYMMDD, second resolution.
/// let body = "20221003,09:15:01,38445.65,0,0\n20221003,09:15:02,38419.40,0,0\n";
/// let rows = decode(body, Columns::TrueDataIndex)?;
/// assert_eq!(rows.len(), 2);
/// assert_eq!(rows[0].close, 3_844_565, "38444.65 rupees in paisa");
/// # Ok::<(), pull::csv::CsvError>(())
/// ```
pub fn decode(body: &str, columns: Columns) -> Result<Vec<RawRow>, CsvError> {
    let mut tally = Tally {
        lines: 0,
        skipped: 0,
        unreadable_volume: 0,
        unreadable_open_interest: 0,
        negative_volume: 0,
    };
    // ONE EVENT PER FILE, ON EITHER OUTCOME. The pass below is per row and
    // logs nothing; this is where its two counters are read. A file that
    // decoded and a file that refused were indistinguishable from outside
    // before this, and both are the ordinary case on an archive walk.
    match decode_rows(body, columns, &mut tally) {
        Ok(rows) => {
            note_decoded(columns, tally, rows.len());
            Ok(rows)
        }
        Err(why) => {
            note_refused(columns, tally, &why);
            Err(why)
        }
    }
}

/// [`decode`]'s pass over the body, split out for one reason: the two `note_*`
/// helpers must report what it counted whether it finished or refused, and a
/// `?` inside it cannot do that on the way past.
///
/// The counters are incremented here and read exactly once, by the caller,
/// after the loop has ended.
fn decode_rows(body: &str, columns: Columns, tally: &mut Tally) -> Result<Vec<RawRow>, CsvError> {
    let at = columns.offsets();
    let want = columns.count();
    let mut rows: Vec<RawRow> = Vec::new();

    for (i, raw_line) in body.lines().enumerate() {
        let line_no = i + 1;
        // A plain integer, not an emit. Millions of these on one backfill.
        tally.lines = tally.lines.saturating_add(1);
        // CRLF: `lines()` strips `\n` but leaves `\r`, and a trailing `\r`
        // turns the last field into a number that will not parse. Observed in
        // vendor files, so trimmed rather than assumed absent.
        let line = raw_line.trim_end_matches('\r').trim();
        if line.is_empty() {
            tally.skipped = tally.skipped.saturating_add(1);
            continue;
        }
        if i == 0 && columns.has_header() {
            tally.skipped = tally.skipped.saturating_add(1);
            continue;
        }
        if rows.len() >= MAX_ROWS {
            return Err(CsvError::TooManyRows {
                rows: rows.len(),
                cap: MAX_ROWS,
            });
        }

        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != want {
            return Err(CsvError::FieldCount {
                line: line_no,
                got: fields.len(),
                want,
            });
        }

        let date_text = fields.get(at.date).copied().unwrap_or_default();
        let day =
            day_of(date_text, columns.date_format()).ok_or_else(|| CsvError::DateMalformed {
                line: line_no,
                got: date_text.to_owned(),
                format: columns.date_format(),
            })?;

        let time_text = fields.get(at.time).copied().unwrap_or_default();
        let secs = ist_seconds(time_text).ok_or_else(|| CsvError::TimeMalformed {
            line: line_no,
            got: time_text.to_owned(),
        })?;

        let price_text = fields.get(at.price).copied().unwrap_or_default();
        let price = paisa(price_text).ok_or_else(|| CsvError::PriceMalformed {
            line: line_no,
            got: price_text.to_owned(),
        })?;
        // A NEGATIVE PRICE IS MALFORMED, AND THIS DECODER WAS THE ONE THAT
        // NEVER SAID SO.
        //
        // `paisa` parses a leading minus and hands back the negative — by
        // design, since it is also used where a signed delta is legal — and
        // nothing here tested the sign. The JSON path has refused a negative
        // price since D-0321; `Bar::ohlc_is_sane` refuses one at the append.
        // Between the two sat this decoder, passing it through to die a crate
        // later against a batch index that names no line of this file.
        //
        // Refused rather than skipped, which is deliberate and matches this
        // file's own design: a malformed line refuses the WHOLE file here, and
        // the refusal carries the line number and the text. A row skipped
        // quietly in a format whose every field is positional is more likely a
        // column offset that is wrong than one bad price, and refusing is what
        // makes that discoverable.
        if price < 0 {
            return Err(CsvError::PriceMalformed {
                line: line_no,
                got: price_text.to_owned(),
            });
        }

        // IST wall clock to UTC epoch seconds, converted HERE where the format
        // is known. Carrying the IST-ness onward is exactly the shape of W1.
        let epoch_utc =
            i64::from(day.days_from_epoch()) * 86_400 + secs - crate::session::IST_OFFSET_SECS;

        // OPEN INTEREST IS READ HERE AND NOT IN THE ROW LITERAL BELOW, because
        // one of its two outcomes is a refusal, and a refusal spelled inside a
        // struct field is a `return` in the middle of a struct literal.
        //
        // A field that will not parse is ABSENT rather than zero: `None` and
        // `Some(0)` are different facts, because zero open interest is a
        // measurement and an unreadable field is not. That degrade is counted
        // rather than silent — `Tally::unreadable_open_interest`, §4.
        //
        // THE SENTINEL COLLISION, REFUSED AT THE LAST PLACE THE TWO FACTS ARE
        // STILL TWO. §7 spends `i64::MIN` on "there is no open interest", and
        // [`crate::fetch::land`] performs that substitution one layer down:
        // `row.open_interest.unwrap_or(i64::MIN)`. A vendor that literally
        // sends -9223372036854775808 therefore reaches the bar as the null, and
        // from the bar onward the measurement and the omission are the same
        // eight bytes — `store::format::Bar` holds an `i64` and has no third
        // state to read them back into. This line still holds both, so it is
        // this line or nowhere.
        //
        // WHY A REFUSAL AND NOT THE COUNT-AND-DEGRADE ITS NEIGHBOUR USES. The
        // unreadable degrade records something TRUE — the value is not known,
        // so the field is stored absent, and the counter says how often that
        // happened. Storing a value the vendor DID send as absent records
        // something FALSE, and a counter beside it does not unrecord it: the
        // bar on disk still asserts an omission that did not occur, which is
        // the fallback hiding a failure §4 bans rather than the loud degrade it
        // allows. Volume has no choice in the matter — `RawRow::volume` is
        // `i64` and cannot express absence without a store-format change — and
        // this field has one, for free.
        //
        // It also closes the single hole in a rule already written down. D-0148
        // (`store::format::Bar::counts_are_sane`) refuses every negative open
        // interest EXCEPT `OI_NULL`, so a vendor sending -5 is caught at the
        // store's survey; `i64::MIN` is the one negative value that walks past
        // that check, precisely because it is spelled exactly like the null.
        //
        // WHAT THIS DOES NOT FIX. Rows that reach `fetch::land` by the HTTP
        // path build `open_interest` from a `Vec<i64>` rather than from this
        // decoder (`RawWindow::decode`), and can still carry `i64::MIN` in; the
        // guard for that belongs beside that decode in `crates/pull/src/fetch.rs`
        // and is not this module's to place. Nor does this make an open
        // interest OF `i64::MIN` storable — §7 has spent that value, and an
        // open interest is a contract count, which is never negative at all.
        let open_interest = {
            let text = fields.get(at.open_interest).copied().unwrap_or_default();
            let parsed = text.trim().parse::<i64>().ok();
            if parsed == Some(i64::MIN) {
                return Err(CsvError::OpenInterestSentinel {
                    line: line_no,
                    got: text.trim().to_owned(),
                });
            }
            if parsed.is_none() {
                tally.unreadable_open_interest = tally.unreadable_open_interest.saturating_add(1);
            }
            parsed
        };

        // VOLUME IS READ HERE AND NOT IN THE LITERAL BELOW, for the reason open
        // interest is: one of its outcomes now leaves the row out entirely, and
        // a `continue` spelled inside a struct field is not a thing.
        //
        // A field that will not PARSE is still the counted zero it always was —
        // `RawRow::volume` is `i64` and cannot express absence, and that
        // substitution is `Tally::unreadable_volume`. A field that parses and
        // says `-125` is a different fact: the vendor stated a quantity that
        // cannot exist, and a zero beside it would assert no trade in a minute
        // this build has no reading for. So that row is skipped and counted.
        let volume = {
            let text = fields.get(at.volume).copied().unwrap_or_default();
            match text.trim().parse::<i64>() {
                Ok(n) if n < 0 => {
                    tally.negative_volume = tally.negative_volume.saturating_add(1);
                    continue;
                }
                Ok(n) => n,
                Err(_) => {
                    tally.unreadable_volume = tally.unreadable_volume.saturating_add(1);
                    0
                }
            }
        };

        // A snapshot row carries ONE price, not four. Open, high, low and close
        // are all that price, and that is honest for a snapshot: nothing in the
        // row claims a range, so nothing here invents one.
        rows.push(RawRow {
            timestamp: epoch_utc,
            open: price,
            high: price,
            low: price,
            close: price,
            // THE VENDOR SENT THIS AND IT WAS BEING DISCARDED. A field that
            // will not parse is zero for volume — a count this build could not
            // read is not a trade — and the substitution is counted where it
            // cannot be avoided. Open interest is read above instead, because
            // absence IS expressible there and one of its outcomes refuses.
            volume,
            open_interest,
        });
    }

    Ok(rows)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------
//
// IN THE FILE, because [`day_of`] is private and two of its four arms are
// unreachable through [`decode`]: `Columns::date_format` returns only
// `CompactYmd` and `SlashedDmy`, so the dashed and compact day-first arms — the
// ones an HTTP feed's descriptor selects — have no route in from outside. An
// arm nothing exercises is an arm nobody has read since it was written, and
// this decoder's whole subject is that reading a date the wrong way round is
// silent.
//
// NO DATE IS WRITTEN AS A BARE QUOTED WORD. An eight-digit compact date is
// shaped exactly like a parameter-path segment as far as CI gate 1d is
// concerned, so every fixture below is assembled by `format!` from integers
// instead — and so is every `expect` message that would otherwise spell one.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail, and these lints exist to \
              keep panics out of the crate rather than out of its tests"
)]
mod tests {
    use super::*;

    /// The same date, in each of the four renderings a descriptor can declare.
    fn rendered(year: u16, month: u8, day: u8, format: DateFormat) -> String {
        match format {
            DateFormat::CompactYmd => format!("{year:04}{month:02}{day:02}"),
            DateFormat::DashedYmd => format!("{year:04}-{month:02}-{day:02}"),
            DateFormat::SlashedDmy => format!("{day:02}/{month:02}/{year:04}"),
            DateFormat::CompactDmy => format!("{day:02}{month:02}{year:04}"),
            DateFormat::DashedYmdMidnight => {
                format!("{year:04}-{month:02}-{day:02} 00:00:00")
            }
        }
    }

    /// Every declared date format reads the same day, and the day-first ones
    /// read it day-first.
    #[test]
    fn every_declared_date_format_reads_the_day_it_names() {
        let every = [
            DateFormat::CompactYmd,
            DateFormat::DashedYmd,
            DateFormat::SlashedDmy,
            DateFormat::CompactDmy,
        ];
        let wanted = crate::session::Day::new(2025, 7, 1).expect("a real date");

        for format in every {
            let text = rendered(2025, 7, 1, format);
            assert_eq!(
                day_of(&text, format),
                Some(wanted),
                "{text} read as {format:?} is 1 July 2025"
            );
        }

        // THE FAULT THIS EXISTS TO PREVENT. The same eight and ten bytes read
        // under the other convention are a different month, and the wrong
        // answer is a perfectly ordinary date.
        let day_first = rendered(2025, 7, 1, DateFormat::SlashedDmy);
        assert_eq!(
            day_of(&day_first, DateFormat::SlashedDmy),
            Some(wanted),
            "01/07/2025 is 1 July"
        );
        assert_eq!(
            day_of(&day_first, DateFormat::DashedYmd),
            None,
            "and it is not a dashed year-first date at all — refused rather \
             than read as 7 January"
        );
        let compact_day_first = rendered(2025, 7, 1, DateFormat::CompactDmy);
        assert_eq!(
            day_of(&compact_day_first, DateFormat::CompactYmd),
            None,
            "01072025 is not a year-first date either: year 0107 is before the \
             calendar this build holds"
        );
    }

    /// A date of the wrong length, the wrong alphabet or the wrong calendar is
    /// refused rather than salvaged.
    #[test]
    fn a_date_that_is_not_the_declared_format_is_refused() {
        for format in [
            DateFormat::CompactYmd,
            DateFormat::DashedYmd,
            DateFormat::SlashedDmy,
            DateFormat::CompactDmy,
        ] {
            let text = rendered(2025, 7, 1, format);
            let mut short = text.clone();
            short.pop();
            assert_eq!(
                day_of(&short, format),
                None,
                "{short} is the wrong length for {format:?}"
            );
            let mut long = text.clone();
            long.push('0');
            assert_eq!(day_of(&long, format), None, "and so is {long}");
            assert_eq!(day_of("", format), None, "an empty field names no day");
        }

        // The right length and the wrong alphabet.
        let letters = format!("{}{}", "ABCD", "EFGH");
        assert_eq!(day_of(&letters, DateFormat::CompactYmd), None);
        let separators = format!("{}-{}-{}", "ABCD", "EF", "GH");
        assert_eq!(day_of(&separators, DateFormat::DashedYmd), None);

        // The right length, the right alphabet, and no such day. The calendar
        // rule is `Day::new`'s, and this proves it is actually consulted.
        let unreal = rendered(2025, 2, 30, DateFormat::CompactYmd);
        assert_eq!(
            day_of(&unreal, DateFormat::CompactYmd),
            None,
            "30 February parses and is still not a date"
        );
        let before_the_calendar = rendered(1969, 12, 31, DateFormat::CompactYmd);
        assert_eq!(day_of(&before_the_calendar, DateFormat::CompactYmd), None);
    }

    /// A price whose rupee part is too large to put on the paisa grid is
    /// refused rather than wrapped.
    #[test]
    fn a_price_too_large_for_the_paisa_grid_is_refused() {
        let just_inside = i64::MAX / 100;
        assert_eq!(
            paisa(&just_inside.to_string()),
            Some(just_inside * 100),
            "the largest whole rupee value that fits still converts"
        );
        assert_eq!(
            paisa(&(just_inside + 1).to_string()),
            None,
            "and one rupee more overflows the grid — refused, never wrapped"
        );
        assert_eq!(
            paisa(&format!("{just_inside}.99")),
            None,
            "the hundredths are what tip this one over, and the add is checked \
             as well as the multiply"
        );
        assert_eq!(
            paisa(&format!("-{just_inside}")),
            Some(-just_inside * 100),
            "and the sign is applied after the arithmetic, not before it"
        );
        assert_eq!(
            paisa(&core::iter::repeat_n(char::from(b'9'), 20).collect::<String>()),
            None,
            "twenty digits are all digits and still name no i64 — refused at \
             the parse rather than wrapped"
        );
    }

    /// `HH:MM:SS`, and every way a time field is not that.
    #[test]
    fn a_time_that_is_not_hh_mm_ss_is_refused_rather_than_salvaged() {
        let hms = |h: u32, m: u32, s: u32| format!("{h:02}:{m:02}:{s:02}");
        let good = hms(9, 15, 1);
        assert_eq!(
            ist_seconds(&good),
            Some(9 * 3_600 + 15 * 60 + 1),
            "seconds since IST midnight"
        );
        assert_eq!(ist_seconds(&hms(0, 0, 0)), Some(0));
        assert_eq!(ist_seconds(&hms(23, 59, 59)), Some(23 * 3_600 + 3_599));

        for (text, why) in [
            (good[..5].to_owned(), "no seconds field at all"),
            (
                good.replace(':', ""),
                "no separators, so one field not three",
            ),
            (format!("{good}:{:02}", 0), "a fourth field"),
            (format!("{}:{:02}:{:02}", 9, 15, 1), "a one-digit hour"),
            (format!("{:02}:{}:{:02}", 9, 5, 1), "a one-digit minute"),
            (format!("{:02}:{:02}:{}", 9, 15, 1), "a one-digit second"),
            (
                format!("{}:{:02}:{:02}", "AB", 15, 1),
                "an hour that is letters",
            ),
            (
                format!("{:02}:{}:{:02}", 9, "AB", 1),
                "a minute that is letters",
            ),
            (
                format!("{:02}:{:02}:{}", 9, 15, "AB"),
                "a second that is letters",
            ),
            (hms(24, 0, 0), "an hour past 23"),
            (hms(9, 60, 0), "a minute past 59"),
            (
                hms(9, 15, 60),
                "a second past 59 — no leap second is stored",
            ),
        ] {
            assert_eq!(ist_seconds(&text), None, "{text:?} has {why}");
        }
    }

    /// The byte ranges each format reads its year, month and day from, in the
    /// order [`day_of`] evaluates them.
    const fn ranges(format: DateFormat) -> [(usize, usize); 3] {
        match format {
            DateFormat::CompactYmd => [(0, 4), (4, 6), (6, 8)],
            // The date occupies the same first ten bytes either way; the
            // clock a datetime format adds after it is not a date field.
            DateFormat::DashedYmd | DateFormat::DashedYmdMidnight => [(0, 4), (5, 7), (8, 10)],
            DateFormat::SlashedDmy => [(6, 10), (3, 5), (0, 2)],
            DateFormat::CompactDmy => [(4, 8), (2, 4), (0, 2)],
        }
    }

    /// A field of the right length whose digits are not digits is refused, one
    /// field at a time, in every format.
    #[test]
    fn one_bad_field_is_enough_to_refuse_a_date_in_every_format() {
        for format in [
            DateFormat::CompactYmd,
            DateFormat::DashedYmd,
            DateFormat::SlashedDmy,
            DateFormat::CompactDmy,
        ] {
            let whole = rendered(2025, 7, 1, format);
            for (start, end) in ranges(format) {
                let mut bytes = whole.clone().into_bytes();
                for slot in bytes.iter_mut().take(end).skip(start) {
                    *slot = b'A';
                }
                let corrupted = String::from_utf8(bytes).expect("still ASCII");
                assert_eq!(
                    day_of(&corrupted, format),
                    None,
                    "{corrupted} keeps the shape of {format:?} and one field is \
                     not a number — refused rather than read as whatever the \
                     rest says"
                );
            }
        }
    }

    /// A date field of the right BYTE length whose bytes are not all ASCII is
    /// refused rather than panicking on a slice that is not a char boundary.
    ///
    /// `text.len()` counts bytes and a vendor field is arbitrary input, so the
    /// three slices `day_of` takes are `get` rather than `[..]`. Each case
    /// below puts a two-byte character across exactly one of those boundaries.
    #[test]
    fn a_date_whose_bytes_are_not_all_ascii_is_refused_not_sliced() {
        // Byte layouts, chosen so that the boundary named in the comment is the
        // FIRST one that falls inside the two-byte character.
        let cases = [
            (DateFormat::CompactYmd, format!("202{}003", 'é')),
            (DateFormat::CompactYmd, format!("20221{}0", 'é')),
            (DateFormat::DashedYmd, format!("202{}10-03", 'é')),
            (DateFormat::DashedYmd, format!("2025-1{}01", 'é')),
            (DateFormat::DashedYmd, format!("2025-10{}0", 'é')),
            (DateFormat::SlashedDmy, format!("01/07{}025", 'é')),
            (DateFormat::SlashedDmy, format!("01/0{}2025", 'é')),
            (DateFormat::SlashedDmy, format!("0{}07/2025", 'é')),
            (DateFormat::CompactDmy, format!("012{}025", 'é')),
            (DateFormat::CompactDmy, format!("0{}12025", 'é')),
        ];
        for (format, text) in cases {
            let wanted = match format {
                DateFormat::CompactYmd | DateFormat::CompactDmy => 8,
                DateFormat::DashedYmd | DateFormat::SlashedDmy => 10,
                DateFormat::DashedYmdMidnight => 19,
            };
            assert_eq!(
                text.len(),
                wanted,
                "{text:?} must be the right BYTE length for {format:?}, or the \
                 length guard refuses it before the slice is ever taken"
            );
            assert!(!text.is_char_boundary(4) || !text.is_ascii());
            assert_eq!(
                day_of(&text, format),
                None,
                "{text:?} is refused rather than sliced through a character"
            );
        }
    }

    /// An open interest of exactly `i64::MIN` refuses the file rather than
    /// landing as the §7 null.
    ///
    /// `i64::MIN` is the value [`crate::fetch::land`] substitutes for an
    /// ABSENT open interest, so a row carrying it would be stored as a column
    /// the vendor never filled in and nothing downstream could tell the two
    /// apart again. Every other negative open interest is caught later, by
    /// `Bar::counts_are_sane` at the store's survey (D-0148); this one value
    /// walks past that check because it IS the null's bit pattern, which is
    /// why the refusal has to be at the decoder.
    #[test]
    fn an_open_interest_of_exactly_the_null_sentinel_refuses_the_file() {
        // Assembled, not spelled, for the module note above: a bare quoted
        // eight-digit date is segment-shaped to CI gate 1d and so is a bare
        // quoted count. A whole row carries commas and is neither.
        let date = format!("{:04}{:02}{:02}", 2022, 10, 3);
        let row = |oi: &str| format!("{date},09:15:01,38445.65,250,{oi}\n");

        // THE HAPPY PATH FIRST, so that what refuses below is the value and not
        // the fixture. An ordinary contract count decodes and stays a
        // measurement.
        let ordinary = decode(&row(&2_000.to_string()), Columns::TrueDataIndex)
            .expect("an ordinary contract count is not a sentinel");
        assert_eq!(ordinary.len(), 1);
        assert_eq!(ordinary[0].open_interest, Some(2_000));
        assert_eq!(ordinary[0].volume, 250, "and the volume beside it");

        // ONE PAST THE SENTINEL IS AN ORDINARY NUMBER HERE. The guard is the
        // single value §7 spends, never a range near it: an absurd count is the
        // store's business — D-0148 refuses a negative one at `survey` — and
        // not this decoder's to widen into.
        let near = decode(&row(&(i64::MIN + 1).to_string()), Columns::TrueDataIndex)
            .expect("only the sentinel itself is refused at this boundary");
        assert_eq!(near[0].open_interest, Some(i64::MIN + 1));

        // THE COLLISION. Stored, this row would reach the bar as exactly the
        // value `unwrap_or(i64::MIN)` produces for a field that was never sent.
        let sentinel = i64::MIN.to_string();
        let refused = decode(&row(&sentinel), Columns::TrueDataIndex)
            .expect_err("the null sentinel is not a measurement");
        assert_eq!(
            refused,
            CsvError::OpenInterestSentinel {
                line: 1,
                got: sentinel.clone(),
            },
            "refused by line and by value, never degraded into an absent field"
        );
        let sentence = refused.to_string();
        assert!(
            sentence.contains("line 1"),
            "a per-line refusal names its line — {sentence}"
        );
        assert!(
            sentence.contains(&sentinel),
            "and the value it refused, so the row can be found in the file — {sentence}"
        );

        // AND THE NEIGHBOURING DEGRADE IS UNCHANGED. A field that will not
        // parse at all is still ABSENT and still counted: "this build could not
        // read it" and "the vendor sent the null" are different claims, and
        // only the second would be a lie on disk.
        let unreadable = decode(&row("NOT A COUNT"), Columns::TrueDataIndex)
            .expect("an unreadable count degrades rather than refusing the file");
        assert_eq!(unreadable[0].open_interest, None);
        assert_eq!(
            unreadable[0].volume, 250,
            "and the volume beside it still reads"
        );
    }

    /// One data row in the given layout, its volume and open-interest columns
    /// spelled by the caller, and the one-based line that row lands on.
    ///
    /// The three shipped layouts disagree about two things the sentinel guard
    /// depends on, and neither is visible from a single-layout fixture. Open
    /// interest is field 4 in both `TrueData` shapes and field 9 in GDFL —
    /// `Columns::offsets` — and GDFL opens with a header, so its one data row
    /// is line TWO. A guard that read a column number written into the decoder,
    /// or that reported line 1 because that is where the first fixture put the
    /// row, passes two of the three cases below.
    ///
    /// The dates come from `rendered` rather than being spelled out, for the
    /// module note above: a bare quoted eight-digit date is segment-shaped as
    /// far as CI gate 1d is concerned.
    fn one_row(columns: Columns, volume: &str, open_interest: &str) -> (String, usize) {
        let ymd = rendered(2022, 10, 3, DateFormat::CompactYmd);
        let dmy = rendered(2022, 10, 3, DateFormat::SlashedDmy);
        match columns {
            // date, time, price, volume, open_interest.
            Columns::TrueDataIndex => (
                format!("{ymd},09:15:01,38445.65,{volume},{open_interest}\n"),
                1,
            ),
            // The same five, then the four carrying bid/ask depth.
            Columns::TrueDataFutures => (
                format!("{ymd},09:15:01,38445.65,{volume},{open_interest},0,0,0,0\n"),
                1,
            ),
            // Ticker, Date, Time, LTP, BuyPrice, BuyQty, SellPrice, SellQty,
            // LTQ, OpenInterest — a day-first date, and a header row, so the
            // data lands on line two.
            Columns::Gdfl => (
                format!(
                    "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest\n\
                     NIFTY,{dmy},09:15:01,38445.65,0,0,0,0,{volume},{open_interest}\n"
                ),
                2,
            ),
        }
    }

    /// The sentinel is refused in every declared layout, at that layout's own
    /// open-interest column — and at that column only.
    ///
    /// The refusal reads `at.open_interest`, which is the layout's number and
    /// not the decoder's. Proved by running the same value through all three
    /// shipped layouts, two of which put open interest in one column and one of
    /// which puts it five columns further along behind a header row. See
    /// `one_row` for why a single-layout fixture cannot show this.
    #[test]
    fn the_sentinel_guard_reads_each_layouts_own_open_interest_column() {
        let sentinel = i64::MIN.to_string();
        let ordinary = 2_000.to_string();

        for columns in [
            Columns::TrueDataIndex,
            Columns::TrueDataFutures,
            Columns::Gdfl,
        ] {
            // THE FIXTURE FIRST, in each layout. If this did not decode, what
            // refuses below would be the shape and not the value.
            let (readable, _) = one_row(columns, &ordinary, &ordinary);
            let rows =
                decode(&readable, columns).expect("a well-formed row in every declared layout");
            assert_eq!(rows.len(), 1, "{columns:?} carries one data row");
            assert_eq!(
                rows.first().map(|row| row.open_interest),
                Some(Some(2_000)),
                "{columns:?} reads open interest from its own column"
            );

            let (poisoned, line) = one_row(columns, &ordinary, &sentinel);
            assert_eq!(
                decode(&poisoned, columns),
                Err(CsvError::OpenInterestSentinel {
                    line,
                    got: sentinel.clone(),
                }),
                "{columns:?}: the sentinel in the open-interest column refuses \
                 the file, and the refusal names the line that column was on"
            );

            // AND ONLY THAT COLUMN. §7 spends `i64::MIN` on an absent OPEN
            // INTEREST and on nothing else, so the same value in the VOLUME
            // column is not THIS refusal — widening the sentinel guard to the
            // whole row would be inventing a rule nothing has written down.
            //
            // **What the row meets instead is the volume rule, and that is a
            // change.** This assertion used to read `Some(i64::MIN)` — the
            // value carried past this boundary untouched — on the reasoning
            // that an impossible volume belongs to `Bar::counts_are_sane` at
            // the store's survey. That is where it was caught, and D-0337
            // measured what it cost: `survey` names a batch index that maps to
            // no line of any file, and it refuses the whole batch rather than
            // the row. The volume guard now sits here, beside the sentinel one,
            // and skips the row.
            //
            // Both halves still hold and the test still separates them: the
            // file DECODES (so the sentinel guard is still scoped to the
            // open-interest column) and the row is GONE (so a count that cannot
            // be a count no longer travels).
            let (odd_volume, _) = one_row(columns, &sentinel, &ordinary);
            let carried = decode(&odd_volume, columns)
                .expect("the sentinel guard is scoped to the one field §7 spends the value on");
            assert!(
                carried.is_empty(),
                "{columns:?}: a negative volume skips its row here rather than \
                 travelling to the store to refuse a whole batch: {carried:?}"
            );

            // AND AN ORDINARY NEGATIVE GOES THE SAME WAY, so the rule is about
            // the sign and not about that one value. `-125` is the number the
            // operator's own vendor sent.
            let (minus, _) = one_row(columns, "-125", &ordinary);
            assert!(
                decode(&minus, columns)
                    .expect("still not a file-level refusal")
                    .is_empty(),
                "{columns:?}: -125 is not a count of shares either"
            );
        }
    }

    /// A sentinel on a later line refuses the WHOLE file, and names that line.
    ///
    /// Both halves need a fixture of their own. The first sentinel test puts
    /// the value on line 1, which a hardcoded `line: 1` would satisfy; and
    /// `decode`'s contract is that a malformed line refuses the whole file
    /// rather than returning the rows before it, because a file missing an
    /// arbitrary subset of its rows is not a shorter file — it is a wrong one,
    /// and the manifest would record it as complete.
    #[test]
    fn a_sentinel_on_a_later_line_refuses_the_whole_file_and_names_that_line() {
        let ymd = rendered(2022, 10, 3, DateFormat::CompactYmd);
        let ordinary = 2_000.to_string();
        let sentinel = i64::MIN.to_string();
        let row = |second: u8, oi: &str| format!("{ymd},09:15:{second:02},38445.65,250,{oi}\n");

        // Four readable rows. This is what the refusal below is measured
        // against, so a truncated answer cannot pass for a correct one.
        let clean: String = (1..=4).map(|second| row(second, &ordinary)).collect();
        assert_eq!(
            decode(&clean, Columns::TrueDataIndex).map(|rows| rows.len()),
            Ok(4),
            "every row of the fixture reads before one of them is poisoned"
        );

        let poisoned: String = (1..=4)
            .map(|second| row(second, if second == 3 { &sentinel } else { &ordinary }))
            .collect();
        assert_eq!(
            decode(&poisoned, Columns::TrueDataIndex),
            Err(CsvError::OpenInterestSentinel {
                line: 3,
                got: sentinel.clone(),
            }),
            "the line is counted rather than assumed, and the two readable rows \
             before it do not come back as a shorter file"
        );
    }

    /// The guard compares the parsed VALUE, not the bytes the vendor spelled it
    /// with.
    ///
    /// Three spellings of one number: bare, wrapped in the whitespace the field
    /// trim removes, and written with a leading zero. All three parse to
    /// `i64::MIN` and all three would land as the §7 null. A guard written as a
    /// string comparison against `i64::MIN.to_string()` would refuse the first
    /// and pass the other two — and the two that passed are the dangerous ones,
    /// because nothing downstream would ever see them as anything but absent.
    ///
    /// This is not a claim that a vendor spells it either of the last two ways.
    /// It is the reason the check sits after the parse rather than before it,
    /// pinned so a later edit cannot quietly move it.
    #[test]
    fn the_sentinel_guard_compares_the_parsed_value_not_its_spelling() {
        let ymd = rendered(2022, 10, 3, DateFormat::CompactYmd);
        let row = |oi: &str| format!("{ymd},09:15:01,38445.65,250,{oi}\n");
        let bare = i64::MIN.to_string();
        // `unsigned_abs` rather than a negation: `-i64::MIN` overflows, and
        // `overflow-checks` is on in BOTH profiles, so that would be a panic
        // rather than a wrap.
        let leading_zero = format!("-0{}", i64::MIN.unsigned_abs());

        for spelling in [bare.clone(), format!("  {bare} "), leading_zero] {
            let refused = decode(&row(&spelling), Columns::TrueDataIndex)
                .expect_err("every spelling of the null sentinel is the null sentinel");
            assert_eq!(
                refused,
                CsvError::OpenInterestSentinel {
                    line: 1,
                    got: spelling.trim().to_owned(),
                },
                "{spelling:?} parses to the sentinel and is refused as one — and \
                 the refusal quotes the field as the file spells it, so the row \
                 can be found again"
            );
        }
    }
}
