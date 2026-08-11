//! Every way reading the lake can refuse, each named.
//!
//! `docs/00-charter.md` prohibition 6 and `CLAUDE.md` §4: degrade loudly and
//! name the reason, or refuse — never both silently. There is no variant here
//! meaning "something was wrong, here is a default anyway". A corrupt file, an
//! unexpected schema, a missing column and an unsupported codec are four
//! different refusals with four different names, because an operator staring
//! at 40 GB of irreplaceable history needs to know which one happened.
//!
//! These are plain enums with a hand-written [`std::fmt::Display`], the same
//! shape `crates/core` uses. A derive macro would be a proc-macro dependency
//! this crate does not need.

use core::fmt;

use brutex_core::error::PriceError;

/// The physical column types this reader understands, for error messages.
///
/// This is a *reporting* type, not a decoding one. It exists so a type
/// mismatch can say what it wanted and what it found without leaking a
/// `parquet` type into this crate's public surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnType {
    /// Parquet `INT32`.
    Int32,
    /// Parquet `INT64`.
    Int64,
    /// Parquet `DOUBLE`.
    Double,
    /// Anything else the lake is not expected to contain.
    Other,
}

impl fmt::Display for ColumnType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Int32 => "INT32",
            Self::Int64 => "INT64",
            Self::Double => "DOUBLE",
            Self::Other => "an unsupported physical type",
        };
        f.write_str(s)
    }
}

/// A lake file could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub enum LakeError {
    /// The file could not be opened or read at all.
    Io {
        /// The underlying operating-system error, rendered.
        reason: String,
    },

    /// The file does not begin and end with the Parquet magic `PAR1`.
    ///
    /// Refused by name rather than handed to the footer parser, because the
    /// parser's own failure on a JPEG is an unhelpful thrift error and an
    /// operator needs to know the file is simply not Parquet.
    NotParquet {
        /// The first four bytes actually found.
        head: [u8; 4],
        /// The last four bytes actually found.
        tail: [u8; 4],
    },

    /// The file is shorter than the smallest possible Parquet file.
    ///
    /// A Parquet file is at minimum `PAR1` + a footer length + `PAR1`. Any
    /// shorter and there is nothing to parse; this is the shape a truncated
    /// download or an interrupted write takes.
    Truncated {
        /// How many bytes the file actually holds.
        len: usize,
        /// The smallest number of bytes a Parquet file can occupy.
        minimum: usize,
    },

    /// The thrift footer would not parse.
    FooterUnreadable {
        /// What the metadata parser said.
        reason: String,
    },

    /// A column is compressed with a codec this reader does not implement.
    ///
    /// Named, never skipped. `CLAUDE.md` §2 forbids the C zstd binding, so the
    /// set of codecs this crate can decode is deliberately small: a file using
    /// another one is a refusal an operator must see, not a column silently
    /// dropped from a bar.
    UnknownCodec {
        /// Which column carried it.
        column: String,
        /// The codec, as the file names it.
        codec: String,
    },

    /// A column the layout requires is absent from the file.
    MissingColumn {
        /// The column that should have been there.
        name: &'static str,
    },

    /// A column is present but holds a different physical type than expected.
    ColumnTypeMismatch {
        /// The column.
        name: &'static str,
        /// What this reader requires.
        want: ColumnType,
        /// What the file actually holds.
        got: ColumnType,
    },

    /// The file's column set matches no layout this crate knows.
    ///
    /// The lake holds exactly two shapes — a 7-column cash/index bar and a
    /// 17-column F&O bar that is a strict superset of it. A third shape is a
    /// writer change, and guessing which columns still mean what would be the
    /// silent fallback `CLAUDE.md` §4 bans.
    UnexpectedSchema {
        /// How many leaf columns the file holds.
        columns: usize,
        /// Their names, in file order.
        names: Vec<String>,
    },

    /// A leaf column is nested or repeated, and this reader will not guess.
    ///
    /// Every leaf in every lake file is an unnested `OPTIONAL` primitive:
    /// maximum definition level 1, maximum repetition level 0. Measured
    /// directly in the schema descriptors of 2,401 real files — every leaf of
    /// every one.
    ///
    /// This refusal exists because the shape is **invisible to the name
    /// check**. A `ColumnDescriptor`'s `name()` is the *leaf* name, so an
    /// `open_interest` wrapped in one optional group still presents as
    /// `open_interest` with the right physical type, and the file is accepted.
    /// Its definition levels then run 0..=2 rather than 0..=1, and a reader
    /// that treats "present" as the single level 1 decodes every row to null.
    ///
    /// It is not decoded because a level-2 leaf carries a distinction
    /// [`crate::bar::Bar`] cannot hold — a null *group* and a null *leaf
    /// inside a present group* are different facts and both would collapse to
    /// one `None`. On `open_interest` that `None` is `i64::MIN`, the
    /// `CLAUDE.md` §7 sentinel, so the collapse would invent a vendor report.
    /// `CLAUDE.md` §3 rule 1 forbids guessing which the file meant. The limit
    /// is written down in `docs/06-limits.md`.
    UnsupportedColumnShape {
        /// The column.
        name: &'static str,
        /// The leaf's maximum definition level. An unnested optional leaf has 1.
        max_def_level: i16,
        /// The leaf's maximum repetition level. An unrepeated leaf has 0.
        max_rep_level: i16,
    },

    /// A page could not be decoded.
    PageDecode {
        /// Which column's pages.
        column: String,
        /// What the page reader said.
        reason: String,
    },

    /// A column chunk stopped short of what the file declares it holds.
    ///
    /// **This is the refusal that replaced a fabrication.** The reader used to
    /// walk `0..num_rows` and push `None` for every row the decoded levels did
    /// not reach, so a chunk delivering 400 of 2,480 rows was *accepted* with
    /// 2,080 nulls this crate invented. On `open_interest` an invented null is
    /// an invented `i64::MIN` — `CLAUDE.md` §7's vendor-reported-none sentinel
    /// — and nothing downstream can tell it from one the vendor really sent.
    /// That is the fallback that hides a failure, banned by `CLAUDE.md` §4.
    ///
    /// Two file faults reach here and both are byte loss rather than a vendor
    /// gap: a `ColumnMetaData.num_values` short of the row group's `num_rows`,
    /// and a chunk whose bytes end on an exact page boundary before its pages
    /// are exhausted. A bad sector, an interrupted write and a partial object
    /// body all leave one of those two shapes behind.
    ///
    /// Deliberately **not** [`Self::UnexpectedNull`]. That one says "the file
    /// has a null here"; this one says "the chunk ran out here". They are
    /// different faults with different remedies — one is a vendor gap, the
    /// other is damage — and naming the wrong one sends an operator looking in
    /// the wrong place.
    ShortColumnChunk {
        /// The column that ran out.
        column: &'static str,
        /// The first row it could not deliver.
        row: usize,
        /// How many were expected.
        expected: usize,
        /// How many actually arrived.
        arrived: usize,
    },

    /// A column that must never be null was null.
    ///
    /// Measured across 170,547 F&O rows and 78,448 cash/index rows: timestamp,
    /// open, high, low, close and volume are null in none of them. This
    /// refusal therefore fires on no file in the lake today — and it exists
    /// precisely so that a writer change which starts emitting a null price is
    /// a loud refusal rather than a zero that looks like a real quote.
    UnexpectedNull {
        /// The column.
        column: &'static str,
        /// Which row of the row group.
        row: usize,
    },

    /// A rupee value could not become a paisa integer.
    ///
    /// The lake stores prices as IEEE doubles because Polars wrote them that
    /// way. `CLAUDE.md` §7 fixes money at paisa integers, so the value has to
    /// cross once — and a value that cannot cross is refused here rather than
    /// saturated into a plausible-looking extreme price.
    NotRepresentable {
        /// The column.
        column: &'static str,
        /// Which row of the row group.
        row: usize,
        /// Why `core` refused it.
        source: PriceError,
    },

    /// Some of the eight greeks were present on a row and some were not.
    ///
    /// Measured across 120 real F&O files and 170,547 rows, the greeks block
    /// is null as one unit: every row observed has all eight or none. A mixed
    /// row therefore contradicts the model this reader is built on, and is
    /// refused rather than resolved — returning `None` would silently discard
    /// the greeks that *were* present, and filling the gaps with zero would
    /// invent values the vendor never sent.
    PartialGreeks {
        /// Which row of the row group.
        row: usize,
        /// How many of the eight were present.
        present: u8,
    },

    /// A row group index was past the end of the file.
    NoSuchRowGroup {
        /// The index asked for.
        asked: usize,
        /// How many the file holds.
        held: usize,
    },

    /// A count in the file did not fit the machine's `usize`, or a length was
    /// negative where the format forbids it.
    ///
    /// Separate from [`Self::FooterUnreadable`] because the footer parsed
    /// perfectly well; it is the *values* that are impossible.
    ImpossibleLength {
        /// What was being sized.
        what: &'static str,
        /// The value the file gave.
        value: i64,
    },
}

impl fmt::Display for LakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { reason } => write!(f, "lake file could not be read: {reason}"),
            Self::NotParquet { head, tail } => write!(
                f,
                "not a parquet file: expected magic PAR1 at both ends, found {head:02x?} and {tail:02x?}"
            ),
            Self::Truncated { len, minimum } => write!(
                f,
                "truncated parquet file: {len} bytes, the minimum is {minimum}"
            ),
            Self::FooterUnreadable { reason } => {
                write!(f, "parquet footer would not parse: {reason}")
            }
            Self::UnknownCodec { column, codec } => write!(
                f,
                "column `{column}` uses compression codec {codec}, which this reader does not implement; refusing rather than skipping the column"
            ),
            Self::MissingColumn { name } => {
                write!(f, "required column `{name}` is absent from the file")
            }
            Self::ColumnTypeMismatch { name, want, got } => write!(
                f,
                "column `{name}` should be {want} but the file holds {got}"
            ),
            Self::UnexpectedSchema { columns, names } => write!(
                f,
                "unrecognised lake schema: {columns} column(s) {names:?}; this crate knows only the 7-column cash bar and the 17-column F&O bar"
            ),
            Self::UnsupportedColumnShape {
                name,
                max_def_level,
                max_rep_level,
            } => write!(
                f,
                "column `{name}` is a leaf at definition level {max_def_level} and repetition level {max_rep_level}; every lake column is a flat OPTIONAL leaf at definition level 1 and repetition level 0, and a nested or repeated one is refused rather than decoded to nulls"
            ),
            Self::PageDecode { column, reason } => {
                write!(f, "column `{column}` page decode refused: {reason}")
            }
            Self::ShortColumnChunk {
                column,
                row,
                expected,
                arrived,
            } => write!(
                f,
                "column `{column}` does not cover its row group: {expected} expected, {arrived} arrived, diverging at row {row}; a chunk that runs out is damage rather than a tail of nulls, and it is refused rather than filled with nulls this reader invented"
            ),
            Self::UnexpectedNull { column, row } => write!(
                f,
                "column `{column}` is null at row {row}, and a null there has no meaning; refusing rather than substituting a value"
            ),
            Self::NotRepresentable {
                column,
                row,
                source,
            } => write!(
                f,
                "column `{column}` at row {row} cannot be represented as paisa: {source}"
            ),
            Self::PartialGreeks { row, present } => write!(
                f,
                "row {row} carries {present} of the 8 greeks; the block is null as a unit in every row measured, so a mixed row is refused rather than guessed at"
            ),
            Self::NoSuchRowGroup { asked, held } => {
                write!(f, "row group {asked} asked for, file holds {held}")
            }
            Self::ImpossibleLength { what, value } => {
                write!(f, "{what} is {value}, which is not a possible length")
            }
        }
    }
}

impl std::error::Error for LakeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NotRepresentable { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A lake contract directory name could not be parsed.
///
/// Separate from [`LakeError`] because parsing a name touches no file: a
/// caller walking 116,086 directories wants to know a name is malformed
/// without that being confusable with the file behind it being corrupt.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ContractError {
    /// The name is longer than this parser is willing to examine.
    ///
    /// Not a claim about the name's *content*: it is a refusal to spend
    /// unbounded time deciding. Measured before the bound existed, rejecting a
    /// 4 KiB name cost 33.6x what parsing a real 26-byte one costs. The same
    /// reasoning as `crates/core`'s `InstrumentError::FieldTooWide`, D-0033.
    TooLong {
        /// How many bytes the name actually held.
        len: usize,
    },
    /// The name did not split into the expected number of `-` separated parts.
    ///
    /// The lake writes exactly two shapes: five parts for an option
    /// (`NSE-NIFTY-01Apr20-10000-CE`) and four for a future
    /// (`NSE-BANKNIFTY-24Apr24-FUT`).
    WrongPartCount {
        /// How many `-` separated parts the name actually had.
        found: usize,
    },
    /// The leading exchange segment was not one this engine reads.
    UnknownExchange {
        /// What was there instead.
        found: String,
    },
    /// The underlying symbol was empty or not a valid symbol.
    BadUnderlying {
        /// What was there instead.
        found: String,
    },
    /// The expiry token was not `DDMonYY`.
    BadExpiryShape {
        /// What was there instead.
        found: String,
    },
    /// The three-letter month was not one of the twelve.
    BadMonth {
        /// What was there instead.
        found: String,
    },
    /// The day, month and year parsed but do not name a real date.
    ///
    /// Refused rather than normalised: 31 February would otherwise become
    /// 3 March and file a contract under a month it never traded in.
    ImpossibleDate {
        /// The day of month.
        day: u8,
        /// The month, 1..=12.
        month: u8,
        /// The four-digit year.
        year: u16,
    },
    /// The strike was not a positive whole number of rupees.
    BadStrike {
        /// What was there instead.
        found: String,
    },
    /// The strike is a whole number but does not fit in `i64` paisa.
    StrikeNotRepresentable {
        /// The rupee strike that overflowed.
        rupees: u64,
    },
    /// The final token was neither `CE`, `PE` nor `FUT`.
    UnknownSide {
        /// What was there instead.
        found: String,
    },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { len } => write!(
                f,
                "contract name is {len} bytes; this parser refuses to examine one that long"
            ),
            Self::WrongPartCount { found } => write!(
                f,
                "contract name has {found} `-` separated part(s); an option has 5 and a future 4"
            ),
            Self::UnknownExchange { found } => {
                write!(f, "unknown exchange segment `{found}`")
            }
            Self::BadUnderlying { found } => write!(f, "unusable underlying `{found}`"),
            Self::BadExpiryShape { found } => {
                write!(f, "expiry `{found}` is not DDMonYY, for example 01Apr20")
            }
            Self::BadMonth { found } => write!(f, "`{found}` is not a month"),
            Self::ImpossibleDate { day, month, year } => {
                write!(f, "{year:04}-{month:02}-{day:02} is not a real date")
            }
            Self::BadStrike { found } => {
                write!(
                    f,
                    "strike `{found}` is not a positive whole number of rupees"
                )
            }
            Self::StrikeNotRepresentable { rupees } => {
                write!(f, "strike {rupees} rupees does not fit in i64 paisa")
            }
            Self::UnknownSide { found } => {
                write!(f, "`{found}` is neither CE, PE nor FUT")
            }
        }
    }
}

impl std::error::Error for ContractError {}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::{ColumnType, ContractError, LakeError};
    use brutex_core::error::PriceError;

    /// The faults that mean the FILE cannot be read at all, paired with a
    /// substring only a correct rendering of each can contain.
    fn file_level_cases() -> Vec<(LakeError, &'static str)> {
        vec![
            (
                LakeError::Io {
                    reason: "permission denied".to_owned(),
                },
                "permission denied",
            ),
            (
                LakeError::NotParquet {
                    head: [0xFF, 0xD8, 0xFF, 0xE0],
                    tail: [1, 2, 3, 4],
                },
                "ff",
            ),
            (
                LakeError::Truncated {
                    len: 7,
                    minimum: 12,
                },
                "7 bytes",
            ),
            (
                LakeError::FooterUnreadable {
                    reason: "thrift ran out".to_owned(),
                },
                "thrift ran out",
            ),
            (LakeError::NoSuchRowGroup { asked: 9, held: 2 }, "9"),
            (
                LakeError::ImpossibleLength {
                    what: "num_rows",
                    value: -3,
                },
                "num_rows",
            ),
        ]
    }

    /// The faults that mean the file reads but holds something this crate
    /// refuses — a wrong column, a wrong type, a value it cannot represent.
    fn content_level_cases() -> Vec<(LakeError, &'static str)> {
        vec![
            (
                LakeError::UnknownCodec {
                    column: "delta".to_owned(),
                    codec: "BROTLI".to_owned(),
                },
                "BROTLI",
            ),
            (LakeError::MissingColumn { name: "vega" }, "vega"),
            (
                LakeError::ColumnTypeMismatch {
                    name: "greeks_provenance_id",
                    want: ColumnType::Int32,
                    got: ColumnType::Int64,
                },
                "greeks_provenance_id",
            ),
            (
                LakeError::UnexpectedSchema {
                    columns: 9,
                    names: vec!["a".to_owned(), "b".to_owned()],
                },
                "9 column",
            ),
            (
                LakeError::UnsupportedColumnShape {
                    name: "open_interest",
                    max_def_level: 2,
                    max_rep_level: 0,
                },
                "open_interest",
            ),
            (
                LakeError::PageDecode {
                    column: "close".to_owned(),
                    reason: "zstd frame refused".to_owned(),
                },
                "zstd frame refused",
            ),
            (
                LakeError::ShortColumnChunk {
                    column: "high",
                    row: 41,
                    expected: 100,
                    arrived: 3,
                },
                "high",
            ),
            (
                LakeError::UnexpectedNull {
                    column: "timestamp",
                    row: 17,
                },
                "timestamp",
            ),
            (
                LakeError::NotRepresentable {
                    column: "low",
                    row: 5,
                    source: PriceError::NotFinite,
                },
                "low",
            ),
            (
                LakeError::PartialGreeks {
                    row: 88,
                    present: 4,
                },
                "88",
            ),
        ]
    }

    /// **EVERY VARIANT RENDERS, AND RENDERS ITS OWN FIELDS.**
    ///
    /// Both `Display` impls in this file were dark: a coverage run on
    /// 2026-08-10 put `lake/src/error.rs` at **24.05% lines**, the worst file in
    /// the workspace, entirely because nothing ever formatted an error although
    /// every variant is constructed somewhere.
    ///
    /// A never-formatted `Display` is not a cosmetic gap. These arms are a long
    /// `match` of near-identical `write!` calls, which is the ideal habitat for
    /// a copy-pasted arm that interpolates the previous variant's field — and
    /// the result is an error message that confidently names the wrong column
    /// or the wrong row while the code around it is correct. Nobody discovers
    /// that until an incident, which is the moment the message is load-bearing.
    ///
    /// So this asserts three things, and the second and third are the ones with
    /// teeth:
    ///   1. every variant renders something non-empty;
    ///   2. every rendering is DISTINCT from every other — a duplicated arm
    ///      fails here;
    ///   3. each rendering contains its OWN distinguishing field value — an arm
    ///      that interpolates a neighbour's field fails here.
    #[test]
    fn every_error_variant_renders_and_names_its_own_fields() {
        // (the error, a substring only a CORRECT rendering of it can contain)
        let mut cases = file_level_cases();
        cases.extend(content_level_cases());

        let mut rendered: Vec<String> = Vec::new();
        for (error, must_contain) in cases {
            let text = error.to_string();
            assert!(!text.is_empty(), "{error:?} rendered nothing");
            assert!(
                text.contains(must_contain),
                "{error:?} rendered {text:?}, which does not name its own \
                 {must_contain:?} — the arm is interpolating the wrong field"
            );
            rendered.push(text);
        }
        let mut unique = rendered.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            rendered.len(),
            "two variants render identically, so one arm was copied and not \
             edited: {rendered:#?}"
        );
    }

    /// The contract parser's errors, held to the same standard as [`LakeError`].
    ///
    /// Separate from the test above because a shared loop over two unrelated
    /// enums would have to erase both to `Box<dyn Display>` and would then stop
    /// naming which enum failed.
    #[test]
    fn every_contract_error_variant_renders_and_names_its_own_fields() {
        let cases: Vec<(ContractError, &str)> = vec![
            (ContractError::TooLong { len: 999 }, "999"),
            (ContractError::WrongPartCount { found: 3 }, "3"),
            (
                ContractError::UnknownExchange {
                    found: "LSE".to_owned(),
                },
                "LSE",
            ),
            (
                ContractError::BadUnderlying {
                    found: "nifty!".to_owned(),
                },
                "nifty!",
            ),
            (
                ContractError::BadExpiryShape {
                    found: "31XYZ25".to_owned(),
                },
                "31XYZ25",
            ),
            (
                ContractError::BadMonth {
                    found: "XYZ".to_owned(),
                },
                "XYZ",
            ),
            (
                ContractError::ImpossibleDate {
                    day: 31,
                    month: 2,
                    year: 2025,
                },
                "31",
            ),
            (
                ContractError::BadStrike {
                    found: "26k".to_owned(),
                },
                "26k",
            ),
            (
                ContractError::StrikeNotRepresentable {
                    rupees: 999_999_999,
                },
                "999999999",
            ),
            (
                ContractError::UnknownSide {
                    found: "XX".to_owned(),
                },
                "XX",
            ),
        ];

        let mut rendered: Vec<String> = Vec::new();
        for (error, must_contain) in cases {
            let text = error.to_string();
            assert!(!text.is_empty(), "{error:?} rendered nothing");
            assert!(
                text.contains(must_contain),
                "{error:?} rendered {text:?}, which does not name its own \
                 {must_contain:?}"
            );
            rendered.push(text);
        }
        let mut unique = rendered.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            rendered.len(),
            "two variants render identically: {rendered:#?}"
        );
    }

    /// The small reporting enum both of the above interpolate.
    #[test]
    fn every_column_type_renders_distinctly() {
        let all = [
            ColumnType::Int32,
            ColumnType::Int64,
            ColumnType::Double,
            ColumnType::Other,
        ];
        let mut seen: Vec<String> = all.iter().map(ToString::to_string).collect();
        assert!(seen.iter().all(|s| !s.is_empty()));
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), all.len(), "two column types render the same");
    }
}
