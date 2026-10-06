//! Instrument identity — the one key every vendor resolves to.
//!
//! Five sources spell the same contract five incompatible ways:
//!
//! | Source | The same NIFTY option |
//! |---|---|
//! | primary broker | a symbol string |
//! | secondary broker | a **numeric security id** |
//! | tick vendor A, CSV | `NIFTY03JUL2522800CE.NFO` |
//! | tick vendor A, websocket | `OPTIDX_NIFTY_03JUL2025_CE_22800` |
//! | tick vendor B | a **filename** — its rows carry no identifier at all |
//!
//! [`InstrumentKey`] is what they all decode into. Two vendors naming one
//! contract produce one key and therefore **collide by design** — that
//! collision *is* the deduplication. See `docs/05-decisions.md` D-0015.
//!
//! Every field is fixed-width and every one derives [`Hash`], so a key hashes
//! in a constant number of machine words no matter which vendor it came from.
//! That is what makes the O(1) dedup claim true rather than aspirational — and
//! `core::symbol::hashing_feeds_the_same_number_of_bytes_however_long_the_input_was`
//! is what makes it checked: it builds an [`InstrumentKey`] from a
//! one-character underlying and one from a full-width underlying, and asserts
//! both feed a hasher the identical number of bytes.
//!
//! # Storable is not sweepable
//!
//! `docs/00-charter.md` §1 stores every NSE instrument but sweeps exactly
//! two, both on NSE. See D-0017 and D-0018. [`InstrumentKey::is_sweepable`] is the
//! one place that distinction is decided, so widening it is a visible edit
//! rather than a caller passing a different string.

use crate::error::InstrumentError;
use crate::price::Paisa;
use crate::symbol::Symbol;
use std::fmt;

/// The exchange an instrument trades on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Exchange {
    /// National Stock Exchange.
    Nse,
    /// Bombay Stock Exchange.
    Bse,
}

impl Exchange {
    /// The path segment used by the store layout.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Nse => "NSE",
            Self::Bse => "BSE",
        }
    }

    /// Parses an exchange from its path segment.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::UnknownExchange`] for anything else. There is no
    /// "default exchange": guessing one would file an instrument under a
    /// venue it does not trade on.
    pub fn parse(text: &str) -> Result<Self, InstrumentError> {
        match text {
            "NSE" => Ok(Self::Nse),
            "BSE" => Ok(Self::Bse),
            _ => Err(InstrumentError::UnknownExchange),
        }
    }
}

/// The exchange segment, which is also a directory in the store layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Segment {
    /// Spot indices.
    Index,
    /// Cash equities.
    Cash,
    /// Futures and options.
    Fno,
}

impl Segment {
    /// The path segment used by the store layout.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Index => "INDEX",
            Self::Cash => "CASH",
            Self::Fno => "FNO",
        }
    }

    /// Parses a segment from its path segment.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::Malformed`] for anything else.
    pub fn parse(text: &str) -> Result<Self, InstrumentError> {
        match text {
            "INDEX" => Ok(Self::Index),
            "CASH" => Ok(Self::Cash),
            "FNO" => Ok(Self::Fno),
            _ => Err(InstrumentError::Malformed),
        }
    }
}

/// Which side of an option contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OptionSide {
    /// Call.
    Call,
    /// Put.
    Put,
}

impl OptionSide {
    /// The two-letter suffix every Indian vendor uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Call => "CE",
            Self::Put => "PE",
        }
    }
}

/// A contract expiry date.
///
/// Stored as year, month, day in that field order so the derived [`Ord`] is
/// chronological without a hand-written comparator.
///
/// This is deliberately **not** a general date type. It validates only what an
/// expiry needs, and it refuses an impossible date rather than normalising it
/// — 31 February would otherwise silently become 3 March and file a contract
/// under a month it never traded in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Expiry {
    year: u16,
    month: u8,
    day: u8,
}

impl Expiry {
    /// Builds an expiry, refusing an impossible date.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::Malformed`] if the year is outside 1990..=2100, the
    /// month outside 1..=12, or the day outside the real length of that month
    /// including leap years.
    pub const fn new(year: u16, month: u8, day: u8) -> Result<Self, InstrumentError> {
        if year < 1990 || year > 2100 || month < 1 || month > 12 || day < 1 {
            return Err(InstrumentError::Malformed);
        }
        if day > Self::days_in_month(year, month) {
            return Err(InstrumentError::Malformed);
        }
        Ok(Self { year, month, day })
    }

    /// Days in a given month, honouring the Gregorian leap rule.
    const fn days_in_month(year: u16, month: u8) -> u8 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            // A century is a leap year only when divisible by 400, which is
            // why 1900 was not and 2000 was.
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29
            }
            2 => 28,
            _ => 0,
        }
    }

    /// The year.
    #[must_use]
    pub const fn year(self) -> u16 {
        self.year
    }
    /// The month, 1..=12.
    #[must_use]
    pub const fn month(self) -> u8 {
        self.month
    }
    /// The day of month.
    #[must_use]
    pub const fn day(self) -> u8 {
        self.day
    }
}

impl fmt::Display for Expiry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// What kind of instrument this is, carrying the fields that kind requires.
///
/// Modelling the variants this way makes an illegal combination
/// *unrepresentable*: an index cannot accidentally carry a strike, and an
/// option cannot exist without one. A flat struct with optional fields would
/// have allowed both, and the check would have moved to review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A spot index. No expiry, no strike.
    Index,
    /// A cash equity. No expiry, no strike.
    Equity,
    /// A futures contract.
    Future {
        /// Contract expiry.
        expiry: Expiry,
    },
    /// An options contract.
    Option {
        /// Contract expiry.
        expiry: Expiry,
        /// Strike price, in paisa.
        strike: Paisa,
        /// Call or put.
        side: OptionSide,
    },
}

/// The canonical identity of one tradable instrument.
///
/// Equality and hashing are structural over every field, so this type is
/// directly usable as a `HashMap` key — which is what makes duplicate
/// rejection a single probe rather than a scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstrumentKey {
    /// Trading venue.
    pub exchange: Exchange,
    /// Exchange segment.
    pub segment: Segment,
    /// Underlying symbol — `NIFTY` for a NIFTY option, not the contract name.
    pub underlying: Symbol,
    /// Instrument kind and its kind-specific fields.
    pub kind: Kind,
}

impl InstrumentKey {
    /// UNVERIFIED performance: no named cost test or measured latency bound is established here.
    /// The two INDICES the engine sweeps.
    ///
    /// `docs/00-charter.md` §1. India VIX is deliberately absent: it is stored
    /// and stamped onto trades, but it never enters the condition vocabulary,
    /// ranking, or run identity.
    ///
    /// This is no longer the whole sweep surface. D-0506 widened it to the
    /// F&O cash equities as well, and those are not listed here: they are the
    /// names `crate::universe::FNO_UNDERLYINGS` already holds, probed in
    /// O(1) through `FNO_INDEX`, less the five that are indices and so have
    /// no cash equity -- `crate::universe::FNO_INDEX_UNDERLYINGS`, D-0682.
    /// Both of this table's names are among those five. Copying the shares
    /// into a second list is how two lists drift, and the audit that found
    /// seven stale counts in this crate's own prose is the argument against
    /// it. See [`Self::is_sweepable`].
    pub const SWEPT: [(Exchange, &'static str); 2] =
        [(Exchange::Nse, "NIFTY"), (Exchange::Nse, "BANKNIFTY")];

    /// Builds a spot index key.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::Malformed`] if the symbol is not valid.
    pub fn index(exchange: Exchange, underlying: &str) -> Result<Self, InstrumentError> {
        Ok(Self {
            exchange,
            segment: Segment::Index,
            underlying: Symbol::new(underlying)?,
            kind: Kind::Index,
        })
    }

    /// Builds a cash-equity key: the stock's own price series, not a contract.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::Malformed`] if the symbol is not valid.
    pub fn cash(exchange: Exchange, underlying: &str) -> Result<Self, InstrumentError> {
        Ok(Self {
            exchange,
            segment: Segment::Cash,
            underlying: Symbol::new(underlying)?,
            kind: Kind::Equity,
        })
    }

    /// UNVERIFIED performance: no named cost test or measured latency bound is established here.
    /// Whether the sweep engine may operate on this instrument.
    ///
    /// Storable and sweepable are different questions. Everything is storable.
    /// Sweepable is exactly two shapes, and widening either requires a
    /// `docs/05-decisions.md` entry rather than a different argument here:
    ///
    /// 1. An NSE spot INDEX named in [`Self::SWEPT`] -- NIFTY and BANKNIFTY.
    /// 2. An NSE CASH EQUITY whose symbol is one of the F&O underlyings in
    ///    `crate::universe::FNO_UNDERLYINGS` and is a SHARE: 208 of the 213.
    ///    Added by D-0506: the operator's objective is the rare, massive
    ///    winner, and those moves exist in single stocks and are averaged
    ///    away in an index.
    ///
    /// # The five F&O underlyings that are indices
    ///
    /// `crate::universe::FNO_INDEX_UNDERLYINGS` -- BANKNIFTY, FINNIFTY,
    /// MIDCPNIFTY, NIFTY and NIFTYNXT50 -- are F&O underlyings and have no
    /// cash equity, so a `(NSE, Cash, Equity)` key named after one describes
    /// no instrument. Until D-0682 this arm accepted them, so a made-up
    /// `NSE-FINNIFTY` stock passed as sweepable. It refuses all five now.
    /// NIFTY and BANKNIFTY are unaffected, because they reach the
    /// surface through shape 1 as the indices they are; FINNIFTY, MIDCPNIFTY
    /// and NIFTYNXT50 are in neither shape.
    ///
    /// # What is deliberately NOT sweepable
    ///
    /// Futures and options CONTRACTS, on any underlying. They expire, and
    /// `docs/00-charter.md` §7 records that NSE reuses instrument tokens across
    /// an expiry boundary -- a contract is a moving target and a sweep over one
    /// would stitch two instruments into one series. The cash equity is the
    /// stock's own price, the same thing NIFTY's spot level is for the index,
    /// and it does not expire.
    ///
    /// Every contract, not one of each shape.
    /// `core::instrument::no_contract_is_sweepable_whatever_its_side_expiry_or_strike`
    /// asks both sides, every expiry [`Expiry::new`] admits and both ends of
    /// the strike. The shape walks before it built one call at one expiry, so
    /// an arm that swept every put passed them (AF-56).
    ///
    /// # Cost
    ///
    /// The index arm is two comparisons. The equity arm is one probe into
    /// `FNO_INDEX`, an open-addressed table built at compile time. Over its 213
    /// members the worst case measures seven probes on a hit and ten on a miss,
    /// and the probe tests pin those at no more than eight and twelve. It is
    /// the only true worst-case O(1) membership structure in the workspace. On
    /// a hit there are then at most five string comparisons against
    /// `FNO_INDEX_UNDERLYINGS`, a fixed five-name array walked the way the
    /// index arm walks `SWEPT`'s two. No list is copied, and the 213-name
    /// list is not walked.
    ///
    /// This paragraph used to quote six and eleven, which are `NTM_INDEX`'s
    /// figures over 750 members and not this table's.
    /// `core::universe::the_worst_probes_quoted_in_prose_are_the_measured_ones`
    /// now measures both tables and fails if the sentence above stops naming
    /// `FNO_INDEX`'s own.
    #[must_use]
    pub fn is_sweepable(&self) -> bool {
        match (self.segment, &self.kind) {
            (Segment::Index, Kind::Index) => Self::SWEPT
                .iter()
                .any(|&(ex, sym)| ex == self.exchange && self.underlying.as_str() == sym),
            (Segment::Cash, Kind::Equity) => {
                let symbol = self.underlying.as_str();
                self.exchange == Exchange::Nse
                    && crate::universe::FNO_INDEX.contains(symbol)
                    // None of the five index names. Spelled `all(!=)` because
                    // clippy rewrites `any(==)` to the `.contains(&` gate 11
                    // refuses; the two are the same predicate.
                    && crate::universe::FNO_INDEX_UNDERLYINGS
                        .iter()
                        .all(|&index| index != symbol)
            }
            _ => false,
        }
    }

    /// Refuses an instrument the engine may not sweep.
    ///
    /// # Errors
    ///
    /// [`InstrumentError::NotSweepable`] when [`Self::is_sweepable`] is false.
    /// The error says "storable but not sweepable" rather than "unknown",
    /// because the instrument may be perfectly valid and present.
    pub fn require_sweepable(&self) -> Result<(), InstrumentError> {
        if self.is_sweepable() {
            Ok(())
        } else {
            Err(InstrumentError::NotSweepable)
        }
    }
}

impl fmt::Display for InstrumentKey {
    /// Renders the canonical name used as a store path segment.
    ///
    /// Sorting the names as TEXT sorts by underlying and then expiry, but NOT
    /// by strike: the strike is unpadded paisa, so `500000` sorts after
    /// `1500000` (core-1, D-2609). Order contracts with [`Contract`]'s [`Ord`],
    /// which compares the strike numerically.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.exchange.as_str(), self.underlying)?;
        match self.kind {
            Kind::Index | Kind::Equity => Ok(()),
            Kind::Future { expiry } => write!(f, "-{expiry}-FUT"),
            Kind::Option {
                expiry,
                strike,
                side,
            } => write!(f, "-{}-{}-{}", expiry, strike.raw(), side.as_str()),
        }
    }
}

/// The longest a rendered contract segment can be.
///
/// Deliberately equal to `SYMBOL_CAPACITY`: a contract is a PATH SEGMENT in the
/// store exactly as a symbol is, and `store::path::MAX_SEGMENT_LEN` is pinned
/// to that same number by a `const` assertion. One bound, three places, and a
/// widening in one of them cannot silently outgrow the others.
pub const CONTRACT_CAPACITY: usize = 24;

/// One derivative contract, rendered as the store files it.
///
/// # Why this is a segment of its OWN and not part of the symbol
///
/// D-0019 files a future or an option under its CONTRACT, because `NIFTY`
/// alone would put every expiry of every strike in one directory. The obvious
/// implementation appends the contract to the symbol — and it does not fit.
/// `Symbol` holds 24 bytes and so does a path segment, while the real names run
/// longer than that: `BANKNIFTY-30Sep25-24650-CE` is 26 and `MIDCPNIFTY` is 27.
/// A capacity that holds NIFTY and drops BANKNIFTY is the worst of both, since
/// the failure appears only on some underlyings.
///
/// So the underlying stays the symbol and the contract is the segment BELOW it:
/// `.../FNO/BANKNIFTY/2025-09-30-2465000-CE/1min/2026-08.bin`. Each part is
/// independently inside the bound, the directory tree groups every contract
/// under its own underlying, and neither field had to grow.
///
/// # The strike is PAISA, and it is not a decimal
///
/// `CLAUDE.md` §7: prices are `i64` paisa and never a float. A strike rendered
/// as `24650.00` puts a `.` in a path segment and invites a reader to parse it
/// back as a float; rendered as `2465000` it is the integer the store already
/// holds, and it round-trips exactly.
///
/// # Ordering is by strike NUMERICALLY (core-1, D-2609)
///
/// The strike is unpadded text, so a derived byte order put the ₹5,000 strike
/// (`500000`) after the ₹15,000 one (`1500000`). [`Ord`] compares the
/// `-`-separated parts in turn, an all-digit part by its value and any other
/// part by its bytes, then breaks a remaining tie on the whole text so the
/// order agrees with [`Eq`]. The path format is unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Contract {
    bytes: [u8; CONTRACT_CAPACITY],
    len: u8,
}

impl Ord for Contract {
    /// Reads at most [`CONTRACT_CAPACITY`] bytes on each side.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        fn digits(part: &str) -> bool {
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())
        }
        let (left, right) = (self.as_str(), other.as_str());
        let mut ours = left.split('-');
        let mut theirs = right.split('-');
        loop {
            let order = match (ours.next(), theirs.next()) {
                (None, None) => return left.cmp(right),
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (Some(a), Some(b)) if digits(a) && digits(b) => {
                    let a = a.trim_start_matches('0');
                    let b = b.trim_start_matches('0');
                    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
                }
                (Some(a), Some(b)) => a.cmp(b),
            };
            if order != std::cmp::Ordering::Equal {
                return order;
            }
        }
    }
}

impl PartialOrd for Contract {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Contract {
    /// The contract segment for `kind`, or [`None`] where the kind has none.
    ///
    /// `None` for [`Kind::Index`] and [`Kind::Equity`] is the whole point: a
    /// spot instrument has no expiry and no strike, so it has no contract
    /// directory and its path is one level shallower. That absence is the
    /// signal the path builder branches on, rather than a flag beside it.
    #[must_use]
    pub fn of(kind: Kind) -> Option<Self> {
        match kind {
            Kind::Index | Kind::Equity => None,
            Kind::Future { expiry } => Self::render(expiry, None),
            Kind::Option {
                expiry,
                strike,
                side,
            } => Self::render(expiry, Some((strike, side))),
        }
    }

    /// `YYYY-MM-DD-FUT` or `YYYY-MM-DD-<paisa>-CE`.
    ///
    /// Returns `None` only if the rendering would exceed
    /// [`CONTRACT_CAPACITY`], which admits at most ten strike characters after
    /// the date and separators (9,999,999,999 paisa for a positive strike).
    /// Refused rather than truncated: a truncated
    /// strike names a DIFFERENT contract and would merge two series into one
    /// file, which is the exact failure this type exists to prevent.
    fn render(expiry: Expiry, option: Option<(Paisa, OptionSide)>) -> Option<Self> {
        use std::fmt::Write as _;
        let mut text = String::with_capacity(CONTRACT_CAPACITY);
        // `write!` to a String cannot fail; the capacity check below is the
        // real bound and it is checked explicitly rather than trusted.
        let _ = write!(
            text,
            "{:04}-{:02}-{:02}",
            expiry.year(),
            expiry.month(),
            expiry.day()
        );
        match option {
            None => {
                let _ = write!(text, "-FUT");
            }
            Some((strike, side)) => {
                let _ = write!(text, "-{}-{}", strike.raw(), side.as_str());
            }
        }
        if text.len() > CONTRACT_CAPACITY {
            return None;
        }
        let mut bytes = [0u8; CONTRACT_CAPACITY];
        // `get_mut` and not an index: the capacity check above already refused
        // an over-long rendering, so this cannot be `None` — and writing it as
        // a fallible lookup means a future change to that check cannot turn
        // this line into a panic without the compiler saying so.
        bytes
            .get_mut(..text.len())?
            .copy_from_slice(text.as_bytes());
        Some(Self {
            bytes,
            len: u8::try_from(text.len()).ok()?,
        })
    }

    /// The contract this text names, or [`None`] where it cannot hold it.
    ///
    /// The inverse of [`Self::as_str`], and the reason the census can store a
    /// contract as TEXT rather than as three decoded fields: what goes to disk
    /// is exactly what goes in the path, so the two can never disagree about
    /// which contract a month belongs to.
    ///
    /// Refuses anything over [`CONTRACT_CAPACITY`] or holding a byte a path
    /// segment may not — a `/` here would escape the directory it names, and a
    /// truncated contract is a DIFFERENT contract, so neither is repaired.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        if text.is_empty() || text.len() > CONTRACT_CAPACITY {
            return None;
        }
        if !text
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'-')
        {
            return None;
        }
        let mut bytes = [0u8; CONTRACT_CAPACITY];
        bytes
            .get_mut(..text.len())?
            .copy_from_slice(text.as_bytes());
        Some(Self {
            bytes,
            len: u8::try_from(text.len()).ok()?,
        })
    }

    /// Whether this names a FUTURES contract.
    ///
    /// A future renders `<expiry>-FUT` and an option `<expiry>-<strike>-<side>`,
    /// so the suffix is the whole test and it reads off the same text the store
    /// path uses. No second encoding of the same fact, and therefore nothing
    /// that can disagree with the directory the bars are in.
    #[must_use]
    pub fn is_future(&self) -> bool {
        self.as_str().ends_with("-FUT")
    }

    /// Whether this names an OPTIONS contract.
    #[must_use]
    pub fn is_option(&self) -> bool {
        !self.is_future()
    }

    /// The rendered contract.
    #[must_use]
    pub fn as_str(&self) -> &str {
        let n = usize::from(self.len);
        // Same shape as `Symbol::as_str`, and for the same reason: the
        // constructor admits only ASCII, and ASCII is valid UTF-8. `get` and
        // `unwrap_or` rather than an index and an `expect`, because this runs
        // on the write path for every derivative bar file and a slice index
        // there is a panic in the one place a panic must not be.
        core::str::from_utf8(self.bytes.get(..n).unwrap_or(&[])).unwrap_or("")
    }
}

impl fmt::Debug for Contract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Contract({})", self.as_str())
    }
}

impl fmt::Display for Contract {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {

    #[test]
    fn contracts_order_by_strike_numerically_and_agree_with_equality() {
        // core-1, D-2609: the audit's pair, ₹5,000 against ₹15,000.
        let option = |strike: i64, side: OptionSide| {
            Contract::of(Kind::Option {
                expiry: Expiry::new(2025, 9, 30).expect("a real expiry"),
                strike: Paisa::from_raw(strike),
                side,
            })
            .expect("fits")
        };
        let five = option(500_000, OptionSide::Call);
        let fifteen = option(1_500_000, OptionSide::Call);
        assert_eq!(five.cmp(&fifteen), std::cmp::Ordering::Less);
        assert_eq!(fifteen.cmp(&five), std::cmp::Ordering::Greater);
        assert_eq!(five.cmp(&five), std::cmp::Ordering::Equal);
        let mut chain = [
            option(1_500_000, OptionSide::Put),
            option(5_000, OptionSide::Call),
            option(1_500_000, OptionSide::Call),
            option(500_000, OptionSide::Call),
        ];
        chain.sort();
        assert_eq!(
            chain.map(|contract| contract.as_str().to_owned()),
            [
                "2025-09-30-5000-CE",
                "2025-09-30-500000-CE",
                "2025-09-30-1500000-CE",
                "2025-09-30-1500000-PE",
            ]
            .map(str::to_owned)
        );
        // Two spellings of one value stay distinct and ordered, never Equal.
        let padded = Contract::parse("2025-09-30-0500000-CE").expect("parses");
        assert_ne!(padded.cmp(&five), std::cmp::Ordering::Equal);
        assert_eq!(padded.cmp(&five), five.cmp(&padded).reverse());
        // Expiry still leads the strike.
        let later = Contract::of(Kind::Option {
            expiry: Expiry::new(2025, 10, 28).expect("a real expiry"),
            strike: Paisa::from_raw(5_000),
            side: OptionSide::Call,
        })
        .expect("fits");
        assert_eq!(fifteen.cmp(&later), std::cmp::Ordering::Less);
    }

    #[test]
    fn contract_rendering_accepts_the_exact_capacity_and_refuses_the_next_digit() {
        let expiry = Expiry::new(2025, 9, 30).expect("a real expiry");
        for side in [OptionSide::Call, OptionSide::Put] {
            let kind = Kind::Option {
                expiry,
                strike: crate::price::Paisa::from_raw(9_999_999_999),
                side,
            };
            let contract = Contract::of(kind).expect("exactly 24 contract bytes fit");
            let expected = format!("2025-09-30-9999999999-{}", side.as_str());
            assert_eq!(contract.as_str(), expected);
            assert_eq!(contract.as_str().len(), CONTRACT_CAPACITY);
            assert_eq!(Contract::parse(&expected), Some(contract));
            assert_eq!(
                Contract::of(Kind::Option {
                    expiry,
                    strike: crate::price::Paisa::from_raw(10_000_000_000),
                    side,
                }),
                None,
                "a further digit must refuse without merging contract identities"
            );
        }
    }

    /// EVERY GUARD ON `Contract` REFUSES, and each is asserted separately.
    ///
    /// These are the arms a mutation survives in: a capacity check deleted, a
    /// comparison flipped, a byte filter removed. Each one lets a name through
    /// that would either be TRUNCATED — naming a different contract, merging
    /// two series into one file — or would carry a byte a path segment may not.
    #[test]
    fn a_contract_refuses_what_it_cannot_hold_and_what_a_path_may_not_carry() {
        // AT THE BOUND, and one past it. `parse` is the inverse of `as_str`,
        // so the two must agree about exactly where the edge is.
        let at = "A".repeat(CONTRACT_CAPACITY);
        assert_eq!(
            Contract::parse(&at).map(|c| c.as_str().len()),
            Some(CONTRACT_CAPACITY),
            "{CONTRACT_CAPACITY} bytes is inside"
        );
        assert_eq!(
            Contract::parse(&"A".repeat(CONTRACT_CAPACITY + 1)),
            None,
            "one past it is refused, never truncated: a truncated contract \
             names a DIFFERENT contract"
        );
        assert_eq!(Contract::parse(""), None, "and empty is not a contract");

        // A BYTE A PATH SEGMENT MAY NOT CARRY. `/` would escape the directory
        // it names; lower case and a dot are simply not this grammar.
        for bad in [
            "2025-09-30/FUT",
            "2025-09-30-fut",
            "2025-09-30-24650.5-CE",
            "a b",
        ] {
            assert_eq!(Contract::parse(bad), None, "must refuse: {bad}");
        }

        // AND THE RENDERER'S OWN BOUND. A strike large enough to overflow the
        // segment is refused rather than written short — the same rule from the
        // other direction, and the only way to reach `render`'s length check.
        let huge = Kind::Option {
            expiry: Expiry::new(2025, 9, 30).expect("a real expiry"),
            strike: crate::price::Paisa::from_raw(i64::MAX),
            side: OptionSide::Call,
        };
        assert_eq!(
            Contract::of(huge),
            None,
            "a strike that cannot fit the segment is refused, not truncated"
        );

        // THE ROUND TRIP HOLDS for every shape this build files.
        for kind in [
            Kind::Future {
                expiry: Expiry::new(2025, 9, 30).expect("a real expiry"),
            },
            Kind::Option {
                expiry: Expiry::new(2024, 1, 4).expect("a real expiry"),
                strike: crate::price::Paisa::from_raw(1_920_000),
                side: OptionSide::Put,
            },
        ] {
            let made = Contract::of(kind).expect("it renders");
            assert_eq!(made.is_future(), matches!(kind, Kind::Future { .. }));
            assert_eq!(made.is_option(), matches!(kind, Kind::Option { .. }));
            assert_eq!(made.to_string(), made.as_str());
            assert_eq!(format!("{made:?}"), format!("Contract({})", made.as_str()));
            assert_eq!(
                Contract::parse(made.as_str()),
                Some(made),
                "as_str and parse are inverses"
            );
        }

        // AND SPOT HAS NO CONTRACT AT ALL, which is what the path branches on.
        assert_eq!(Contract::of(Kind::Index), None);
        assert_eq!(Contract::of(Kind::Equity), None);
    }
    use super::*;
    use std::collections::HashMap;

    fn nifty() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "NIFTY").expect("valid")
    }

    #[test]
    fn exchange_and_segment_round_trip_through_path_segments() {
        for e in [Exchange::Nse, Exchange::Bse] {
            assert_eq!(Exchange::parse(e.as_str()), Ok(e));
        }
        for s in [Segment::Index, Segment::Cash, Segment::Fno] {
            assert_eq!(Segment::parse(s.as_str()), Ok(s));
        }
        assert_eq!(
            Exchange::parse("MCX"),
            Err(InstrumentError::UnknownExchange)
        );
        assert_eq!(
            Exchange::parse("nse"),
            Err(InstrumentError::UnknownExchange)
        );
        assert_eq!(Segment::parse("CURRENCY"), Err(InstrumentError::Malformed));
    }

    #[test]
    fn option_side_renders_the_vendor_suffix() {
        assert_eq!(OptionSide::Call.as_str(), "CE");
        assert_eq!(OptionSide::Put.as_str(), "PE");
    }

    #[test]
    fn expiry_refuses_impossible_dates_rather_than_normalising() {
        // 31 February would silently become 3 March under a normalising date
        // type, filing a contract under a month it never traded in.
        assert_eq!(Expiry::new(2025, 2, 31), Err(InstrumentError::Malformed));
        assert_eq!(Expiry::new(2025, 13, 1), Err(InstrumentError::Malformed));
        assert_eq!(Expiry::new(2025, 0, 1), Err(InstrumentError::Malformed));
        assert_eq!(Expiry::new(2025, 1, 0), Err(InstrumentError::Malformed));
        assert_eq!(Expiry::new(2025, 4, 31), Err(InstrumentError::Malformed));
        assert_eq!(Expiry::new(1989, 1, 1), Err(InstrumentError::Malformed));
        assert_eq!(Expiry::new(2101, 1, 1), Err(InstrumentError::Malformed));
    }

    #[test]
    fn an_out_of_range_month_has_zero_days() {
        // `days_in_month` needs a catch-all arm to be exhaustive over u8, and
        // `Expiry::new` validates the month before ever reaching it — so this
        // arm is unreachable through the public API and the coverage gate
        // correctly flagged it as dead.
        //
        // It is kept rather than removed, and tested directly here, because
        // returning 0 makes the arm FAIL SAFE: any future caller that skips
        // the month check gets "no day is valid in this month" instead of a
        // plausible 31. Deleting it to satisfy coverage would trade a proven
        // branch for an unprovable assumption about future callers.
        assert_eq!(Expiry::days_in_month(2025, 0), 0);
        assert_eq!(Expiry::days_in_month(2025, 13), 0);
        assert_eq!(Expiry::days_in_month(2025, u8::MAX), 0);
    }

    #[test]
    fn expiry_honours_the_gregorian_leap_rule() {
        assert!(Expiry::new(2024, 2, 29).is_ok(), "2024 is a leap year");
        assert_eq!(
            Expiry::new(2025, 2, 29),
            Err(InstrumentError::Malformed),
            "2025 is not"
        );
        assert!(Expiry::new(2000, 2, 29).is_ok(), "2000 divisible by 400");
        assert_eq!(
            Expiry::new(2100, 2, 29),
            Err(InstrumentError::Malformed),
            "2100 is divisible by 100 but not 400"
        );
    }

    #[test]
    fn expiry_orders_chronologically_and_renders_iso() {
        let a = Expiry::new(2025, 7, 3).expect("valid");
        let b = Expiry::new(2025, 7, 31).expect("valid");
        let c = Expiry::new(2026, 1, 1).expect("valid");
        assert!(a < b && b < c);
        assert_eq!(a.to_string(), "2025-07-03");
        assert_eq!((a.year(), a.month(), a.day()), (2025, 7, 3));
    }

    #[test]
    fn exactly_two_nse_instruments_are_sweepable() {
        assert!(nifty().is_sweepable());
        assert!(
            InstrumentKey::index(Exchange::Nse, "BANKNIFTY")
                .expect("valid")
                .is_sweepable()
        );
        assert_eq!(InstrumentKey::SWEPT.len(), 2);
        assert!(
            InstrumentKey::SWEPT
                .iter()
                .all(|&(ex, _)| ex == Exchange::Nse),
            "the swept set is NSE-only; see D-0017"
        );
    }

    #[test]
    fn sensex_is_storable_but_no_longer_swept() {
        // D-0017 narrowed the engine surface from three instruments to two.
        // SENSEX bars already on disk are not deleted -- append-only history
        // applies to the store too -- but the engine will not sweep them.
        let sensex = InstrumentKey::index(Exchange::Bse, "SENSEX").expect("valid");
        assert!(!sensex.is_sweepable());
        assert_eq!(
            sensex.require_sweepable(),
            Err(InstrumentError::NotSweepable)
        );
    }

    #[test]
    fn an_nse_equity_sweeps_iff_it_is_an_fno_underlying() {
        // D-0018: every NSE instrument is STORED, and the sweep stayed at the
        // two indices "until a two-instrument sweep earns the widening". This
        // test was what would fail if someone widened the surface silently --
        // and it did fail, on the day D-0506 widened it deliberately. It now
        // pins the widened rule from the same three names: an NSE cash equity
        // sweeps exactly when `FNO_INDEX` holds its symbol, and every one of
        // these three is an F&O underlying. The outsider case, with a
        // self-checking fixture, is in
        // `the_sweep_surface_is_the_two_indices_and_the_fno_cash_equities`.
        for s in ["RELIANCE", "HINDALCO", "TCS"] {
            assert!(
                crate::universe::FNO_INDEX.contains(s),
                "{s} is an F&O underlying, or this fixture is wrong"
            );
            let eq = InstrumentKey {
                exchange: Exchange::Nse,
                segment: Segment::Cash,
                underlying: Symbol::new(s).expect("valid"),
                kind: Kind::Equity,
            };
            assert!(eq.is_sweepable(), "{s} is on the surface since D-0506");
        }
    }

    #[test]
    fn india_vix_is_storable_but_never_sweepable() {
        // The charter stores and stamps VIX but keeps it out of the condition
        // vocabulary, ranking and run identity.
        let vix = InstrumentKey::index(Exchange::Nse, "INDIAVIX").expect("valid");
        assert!(!vix.is_sweepable());
        assert_eq!(vix.require_sweepable(), Err(InstrumentError::NotSweepable));
    }

    #[test]
    fn the_right_symbol_on_the_wrong_exchange_is_not_sweepable() {
        // SENSEX is a BSE index. The same name under NSE is a different thing
        // and must not inherit sweepability from the string alone.
        let wrong = InstrumentKey::index(Exchange::Nse, "SENSEX").expect("valid");
        assert!(!wrong.is_sweepable());
        let also_wrong = InstrumentKey::index(Exchange::Bse, "NIFTY").expect("valid");
        assert!(!also_wrong.is_sweepable());
    }

    #[test]
    fn derivatives_are_storable_but_never_sweepable() {
        let fut = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("NIFTY").expect("valid"),
            kind: Kind::Future {
                expiry: Expiry::new(2025, 7, 31).expect("valid"),
            },
        };
        assert!(!fut.is_sweepable());

        let opt = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("NIFTY").expect("valid"),
            kind: Kind::Option {
                expiry: Expiry::new(2025, 7, 3).expect("valid"),
                strike: Paisa::from_raw(2_280_000),
                side: OptionSide::Call,
            },
        };
        assert!(!opt.is_sweepable());
        assert_eq!(opt.require_sweepable(), Err(InstrumentError::NotSweepable));
    }

    #[test]
    fn two_spellings_of_one_contract_collide_by_design() {
        // This IS the deduplication. One vendor sends lower case, another
        // upper; both must land on a single map entry.
        let from_vendor_a = InstrumentKey::index(Exchange::Nse, "NIFTY").expect("valid");
        let from_vendor_b = InstrumentKey::index(Exchange::Nse, "nifty").expect("valid");
        assert_eq!(from_vendor_a, from_vendor_b);

        let mut seen: HashMap<InstrumentKey, u32> = HashMap::new();
        *seen.entry(from_vendor_a).or_insert(0) += 1;
        *seen.entry(from_vendor_b).or_insert(0) += 1;
        assert_eq!(seen.len(), 1, "one contract must occupy one slot");
        assert_eq!(seen.get(&from_vendor_a), Some(&2));
    }

    #[test]
    fn contracts_differing_in_one_field_never_collide() {
        let base = |expiry, strike, side| InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("NIFTY").expect("valid"),
            kind: Kind::Option {
                expiry,
                strike: Paisa::from_raw(strike),
                side,
            },
        };
        let e1 = Expiry::new(2025, 7, 3).expect("valid");
        let e2 = Expiry::new(2025, 7, 10).expect("valid");

        let keys = [
            base(e1, 2_280_000, OptionSide::Call),
            base(e1, 2_280_000, OptionSide::Put), // side differs
            base(e1, 2_285_000, OptionSide::Call), // strike differs
            base(e2, 2_280_000, OptionSide::Call), // expiry differs
        ];

        let mut seen = std::collections::HashSet::new();
        for k in keys {
            seen.insert(k);
        }
        assert_eq!(seen.len(), 4, "every field must participate in EQUALITY");

        // And in the HASH, which the assertion above cannot see. An audit pointed out
        // that a `Hash` skipping `kind` still yields four distinct set entries, because
        // the derived `Eq` keeps them apart -- they simply all land in ONE BUCKET. That
        // turns a `HashMap<InstrumentKey, _>` lookup into a linear scan of every option
        // on the same underlying, which is exactly the §3 rule 4 defect this test's own
        // doc comment forbids, and it is invisible to a length check.
        //
        // `DefaultHasher` is seeded with zeros rather than randomly, so these digests are
        // reproducible within and across runs -- which is what makes comparing them a
        // test rather than a coin toss.
        let digest = |k: &InstrumentKey| -> u64 {
            use core::hash::{Hash as _, Hasher as _};
            let mut h = std::hash::DefaultHasher::new();
            k.hash(&mut h);
            h.finish()
        };
        let digests: std::collections::BTreeSet<u64> = keys.iter().map(digest).collect();
        assert_eq!(
            digests.len(),
            4,
            "two of these four keys hash alike, so a field is missing from `Hash` while \
             still present in `Eq`. They stay distinct in a HashSet and collide into one \
             bucket, which makes an instrument lookup O(n) in the options on that \
             underlying: {digests:?}"
        );
    }

    #[test]
    fn canonical_names_render_for_the_store_path() {
        assert_eq!(nifty().to_string(), "NSE-NIFTY");

        let fut = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("NIFTY").expect("valid"),
            kind: Kind::Future {
                expiry: Expiry::new(2025, 7, 31).expect("valid"),
            },
        };
        assert_eq!(fut.to_string(), "NSE-NIFTY-2025-07-31-FUT");

        let opt = InstrumentKey {
            exchange: Exchange::Bse,
            segment: Segment::Fno,
            underlying: Symbol::new("SENSEX").expect("valid"),
            kind: Kind::Option {
                expiry: Expiry::new(2025, 7, 3).expect("valid"),
                strike: Paisa::from_raw(8_140_000),
                side: OptionSide::Put,
            },
        };
        assert_eq!(opt.to_string(), "BSE-SENSEX-2025-07-03-8140000-PE");
    }

    /// THE SURFACE IS TWO SHAPES, AND EACH EDGE OF EACH SHAPE IS PINNED.
    ///
    /// This test was `an_equity_is_storable_and_not_sweepable`, built on
    /// HINDALCO, and asserted `!is_sweepable()`. HINDALCO is one of the 213 F&O
    /// underlyings, so D-0506 flipped that assertion -- which is the widening
    /// doing exactly what it says. The old name survives here so a reader
    /// following it from a decision entry lands on the test that replaced it.
    ///
    /// Every fixture proves its own premise first, in the repository's idiom:
    /// a non-member that turned out to be a member would make the refusal
    /// assertion vacuous, so the test checks the index before trusting it.
    #[test]
    fn the_sweep_surface_is_the_two_indices_and_the_fno_cash_equities() {
        // AN F&O CASH EQUITY IS SWEEPABLE.
        let member = InstrumentKey::cash(Exchange::Nse, "HINDALCO").expect("valid");
        assert!(
            crate::universe::FNO_INDEX.contains("HINDALCO"),
            "the fixture must be an F&O member, or the assertion below proves nothing"
        );
        assert!(
            member.is_sweepable(),
            "an F&O cash equity is on the surface"
        );
        assert_eq!(member.require_sweepable(), Ok(()));
        assert_eq!(member.to_string(), "NSE-HINDALCO");

        // A CASH EQUITY OUTSIDE THE F&O UNIVERSE IS NOT. The symbol is shaped
        // like a real one and is checked against the index rather than assumed.
        let outsider = InstrumentKey::cash(Exchange::Nse, "ZZQXNOTFNO").expect("valid shape");
        assert!(
            !crate::universe::FNO_INDEX.contains("ZZQXNOTFNO"),
            "the fixture must be OUTSIDE the F&O universe, or this proves nothing"
        );
        assert!(
            !outsider.is_sweepable(),
            "a non-F&O equity is storable, never swept"
        );
        assert_eq!(
            outsider.require_sweepable(),
            Err(InstrumentError::NotSweepable)
        );

        // AN F&O CONTRACT ON A MEMBER IS STILL NOT SWEEPABLE. The widening is
        // the cash series, never the expiring instrument.
        let contract = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("HINDALCO").expect("valid"),
            kind: Kind::Future {
                expiry: Expiry {
                    year: 2026,
                    month: 9,
                    day: 30,
                },
            },
        };
        assert!(
            !contract.is_sweepable(),
            "a contract expires; the cash series does not"
        );

        // A MEMBER ON THE WRONG EXCHANGE IS NOT. The surface is NSE only.
        let bse = InstrumentKey::cash(Exchange::Bse, "HINDALCO").expect("valid");
        assert!(
            !bse.is_sweepable(),
            "BSE is not swept and not pulled (D-0017)"
        );

        // AND THE TWO INDICES ARE EXACTLY AS THEY WERE.
        assert!(nifty().is_sweepable());
        assert!(
            InstrumentKey::index(Exchange::Nse, "BANKNIFTY")
                .expect("valid")
                .is_sweepable()
        );
        assert!(
            !InstrumentKey::index(Exchange::Nse, "INDIAVIX")
                .expect("valid")
                .is_sweepable(),
            "India VIX is reference only and never enters the sweep"
        );
    }

    /// AF-02. AN F&O INDEX UNDERLYING IS NEVER A SWEEPABLE CASH EQUITY.
    ///
    /// The cash arm accepted every `FNO_INDEX` member, and five of those are
    /// indices, so `InstrumentKey::cash(Nse, "FINNIFTY")` -- a stock that does
    /// not exist -- was on the sweep surface. D-0682. Pinned from both sides:
    /// the five are refused as cash, and EXACTLY the other 208 are accepted,
    /// so the refusal cannot have been bought by shrinking the share side.
    #[test]
    fn an_fno_index_underlying_is_never_a_sweepable_cash_equity() {
        use crate::universe::{FNO_INDEX, FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS};

        for name in FNO_INDEX_UNDERLYINGS {
            assert!(
                FNO_INDEX.contains(name),
                "{name} must be an F&O member, or the refusal below proves nothing"
            );
            let cash = InstrumentKey::cash(Exchange::Nse, name).expect("valid");
            assert!(!cash.is_sweepable(), "{name} is an index and has no stock");
            assert_eq!(
                cash.require_sweepable(),
                Err(InstrumentError::NotSweepable),
                "{name} as a cash equity is refused by name"
            );
        }

        let swept_as_cash: Vec<&str> = FNO_UNDERLYINGS
            .iter()
            .copied()
            .filter(|name| {
                InstrumentKey::cash(Exchange::Nse, name)
                    .expect("every F&O name is a valid symbol")
                    .is_sweepable()
            })
            .collect();
        let shares: Vec<&str> = FNO_UNDERLYINGS
            .iter()
            .copied()
            .filter(|name| !FNO_INDEX_UNDERLYINGS.contains(name))
            .collect();
        assert_eq!(
            swept_as_cash, shares,
            "the cash arm is exactly the F&O list less its index names"
        );
        assert_eq!(swept_as_cash.len(), 208, "213 less 5");

        // NIFTY AND BANKNIFTY STAY, THROUGH THE INDEX ARM; THE OTHER THREE ARE
        // IN NEITHER SHAPE.
        for name in FNO_INDEX_UNDERLYINGS {
            let as_index = InstrumentKey::index(Exchange::Nse, name).expect("valid");
            assert_eq!(
                as_index.is_sweepable(),
                matches!(name, "NIFTY" | "BANKNIFTY"),
                "{name}: only the two SWEPT indices sweep as an index"
            );
        }
    }

    #[test]
    fn a_bad_symbol_is_refused_at_construction() {
        assert_eq!(
            InstrumentKey::index(Exchange::Nse, "NIF TY"),
            Err(InstrumentError::Malformed)
        );
        assert_eq!(
            InstrumentKey::index(Exchange::Nse, ""),
            Err(InstrumentError::Malformed)
        );
    }

    /// Was `require_sweepable_accepts_the_three`, which tested one. It now
    /// tests both indices and an equity from each side of the F&O boundary,
    /// so the name says what the body does.
    #[test]
    fn require_sweepable_accepts_both_indices_and_fno_equities_and_refuses_the_rest() {
        assert_eq!(nifty().require_sweepable(), Ok(()));
        assert_eq!(
            InstrumentKey::index(Exchange::Nse, "BANKNIFTY")
                .expect("valid")
                .require_sweepable(),
            Ok(())
        );
        assert_eq!(
            InstrumentKey::cash(Exchange::Nse, "HINDALCO")
                .expect("valid")
                .require_sweepable(),
            Ok(())
        );
        assert_eq!(
            InstrumentKey::cash(Exchange::Nse, "ZZQXNOTFNO")
                .expect("valid shape")
                .require_sweepable(),
            Err(InstrumentError::NotSweepable)
        );
    }

    /// A writer that fails on first use, to exercise the `?` in `Display`.
    struct AlwaysFails;

    impl fmt::Write for AlwaysFails {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    #[test]
    fn display_propagates_a_writer_error_rather_than_swallowing_it() {
        // `write!(f, ...)?` has an error path that a normal String formatter
        // never takes, so it showed up as the one uncovered region. It is a
        // real path: a Display that discards a writer error would report
        // success while producing a truncated instrument name, and a truncated
        // name in a store path is a different instrument.
        use fmt::Write as _;
        let mut w = AlwaysFails;
        assert!(write!(w, "{}", nifty()).is_err());

        let opt = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("NIFTY").expect("valid"),
            kind: Kind::Option {
                expiry: Expiry::new(2025, 7, 3).expect("valid"),
                strike: Paisa::from_raw(2_280_000),
                side: OptionSide::Call,
            },
        };
        assert!(write!(w, "{opt}").is_err());
    }

    #[test]
    fn every_month_length_is_enforced_rather_than_defaulted() {
        // `days_in_month` has one arm per month length. Nothing pinned the
        // 30-day arm or the common-year February arm, so deleting either one
        // and letting those months fall through to the `_ => 0` default went
        // unnoticed: with 0 days, EVERY date in April or February would be
        // refused, and no test said otherwise.
        for m in [4u8, 6, 9, 11] {
            assert!(Expiry::new(2026, m, 30).is_ok(), "month {m} has 30 days");
            assert_eq!(
                Expiry::new(2026, m, 31),
                Err(InstrumentError::Malformed),
                "month {m} has no 31st"
            );
        }
        for m in [1u8, 3, 5, 7, 8, 10, 12] {
            assert!(Expiry::new(2026, m, 31).is_ok(), "month {m} has 31 days");
        }
        // 2026 is a common year, 2024 a leap year.
        assert!(Expiry::new(2026, 2, 28).is_ok());
        assert_eq!(Expiry::new(2026, 2, 29), Err(InstrumentError::Malformed));
        assert!(Expiry::new(2024, 2, 29).is_ok());
        assert_eq!(Expiry::new(2024, 2, 30), Err(InstrumentError::Malformed));
    }

    #[test]
    fn sweepability_needs_the_segment_and_the_kind_to_agree() {
        // The guard is two checks joined by `||`, and only the segment half
        // was ever exercised. A key whose segment and kind disagree is not
        // sweepable even when the exchange and symbol are both on the swept
        // list -- joining the halves with `&&` instead would let this
        // through, because the segment check alone would no longer refuse.
        let mismatched = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Fno,
            underlying: Symbol::new("NIFTY").expect("valid"),
            kind: Kind::Index,
        };
        assert!(!mismatched.is_sweepable());
        assert_eq!(
            mismatched.require_sweepable(),
            Err(InstrumentError::NotSweepable)
        );

        // And the mirror image: the segment says Index, the kind does not.
        let other_way = InstrumentKey {
            exchange: Exchange::Nse,
            segment: Segment::Index,
            underlying: Symbol::new("NIFTY").expect("valid"),
            kind: Kind::Equity,
        };
        assert!(!other_way.is_sweepable());
    }

    /// The refusal `require_sweepable` owes a key, derived from the answer.
    fn owed(sweepable: bool) -> Result<(), InstrumentError> {
        if sweepable {
            Ok(())
        } else {
            Err(InstrumentError::NotSweepable)
        }
    }

    /// Every `(exchange, segment, kind)` shape over one underlying: 2
    /// exchanges x 3 segments x 4 kinds, 24 keys, exchange-major.
    ///
    /// The near-miss walk and the F&O walk below both ask this product, so
    /// neither can cover fewer shapes than the other.
    fn every_shape_of(underlying: Symbol) -> Vec<InstrumentKey> {
        let expiry = Expiry::new(2026, 9, 29).expect("valid");
        let kinds = [
            Kind::Index,
            Kind::Equity,
            Kind::Future { expiry },
            Kind::Option {
                expiry,
                strike: Paisa::from_raw(2_280_000),
                side: OptionSide::Call,
            },
        ];
        let mut keys = Vec::with_capacity(2 * 3 * kinds.len());
        for exchange in [Exchange::Nse, Exchange::Bse] {
            for segment in [Segment::Index, Segment::Cash, Segment::Fno] {
                for kind in kinds {
                    keys.push(InstrumentKey {
                        exchange,
                        segment,
                        underlying,
                        kind,
                    });
                }
            }
        }
        keys
    }

    /// AF-50. SWEEPABILITY IS JUDGED ON THE CASE-FOLDED SYMBOL, BOTH WAYS.
    ///
    /// `Symbol::new` upper-cases its input, so `finnifty` and `FINNIFTY` are
    /// one key and must get one answer. That folding was pinned only at the
    /// `Symbol` level, and every `is_sweepable` test above spells its names
    /// in upper case, so two regressions were invisible from here. A cash arm
    /// that stopped refusing the five index names would let `finnifty`
    /// through as a stock. A `Symbol` that stopped folding would drop every
    /// lower-case share and index off the surface. The accepted spellings are
    /// asserted beside the refused ones, so the second regression cannot hide
    /// the first by refusing everything.
    #[test]
    fn every_case_spelling_of_an_fno_name_is_judged_as_its_upper_case_self() {
        use crate::universe::{FNO_INDEX, FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS};

        // (spelling, sweepable as an NSE cash equity, sweepable as an NSE index)
        let named = [
            ("finnifty", false, false),
            ("FinNifty", false, false),
            ("fInNiFtY", false, false),
            ("midcpnifty", false, false),
            ("MidCpNifty", false, false),
            ("niftynxt50", false, false),
            ("NiftyNxt50", false, false),
            ("nifty", false, true),
            ("NiFtY", false, true),
            ("banknifty", false, true),
            ("BankNifty", false, true),
            ("reliance", true, false),
            ("Reliance", true, false),
            ("hindalco", true, false),
            ("m&m", true, false),
            ("bajaj-auto", true, false),
        ];
        for (spelling, as_cash, as_index) in named {
            let upper = spelling.to_ascii_uppercase();
            assert_ne!(spelling, upper, "{spelling} must not already be canonical");
            assert!(
                FNO_INDEX.contains(&upper),
                "{upper} must be an F&O name, or the answers below prove nothing"
            );

            let cash = InstrumentKey::cash(Exchange::Nse, spelling).expect("valid");
            assert_eq!(cash.underlying.as_str(), upper, "{spelling} is folded");
            assert_eq!(
                cash,
                InstrumentKey::cash(Exchange::Nse, &upper).expect("valid"),
                "{spelling} and {upper} are one cash key"
            );
            assert_eq!(cash.is_sweepable(), as_cash, "{spelling} as a cash equity");
            assert_eq!(cash.require_sweepable(), owed(as_cash), "{spelling} cash");
            assert_eq!(cash.to_string(), format!("NSE-{upper}"));

            let index = InstrumentKey::index(Exchange::Nse, spelling).expect("valid");
            assert_eq!(index.underlying.as_str(), upper, "{spelling} is folded");
            assert_eq!(index.is_sweepable(), as_index, "{spelling} as an index");
            assert_eq!(
                index.require_sweepable(),
                owed(as_index),
                "{spelling} index"
            );
        }

        // EVERY F&O NAME, LOWER-CASED, GETS ITS UPPER-CASE ANSWER. The expected
        // answers are stated from the two lists, not from `is_sweepable`.
        let mut lower_cash_swept = 0_usize;
        for name in FNO_UNDERLYINGS {
            let lower = name.to_ascii_lowercase();
            let share = !FNO_INDEX_UNDERLYINGS.contains(&name);
            let cash = InstrumentKey::cash(Exchange::Nse, &lower).expect("valid");
            assert_eq!(cash.is_sweepable(), share, "{lower} as a cash equity");
            lower_cash_swept += usize::from(cash.is_sweepable());
            let index = InstrumentKey::index(Exchange::Nse, &lower).expect("valid");
            assert_eq!(
                index.is_sweepable(),
                matches!(name, "NIFTY" | "BANKNIFTY"),
                "{lower} as an index"
            );
        }
        assert_eq!(lower_cash_swept, 208, "213 less the five index names");
    }

    /// AF-50. A NEAR MISS OF A SURFACE NAME IS NEVER SWEEPABLE, IN ANY SHAPE.
    ///
    /// The only outsider fixture above is `ZZQXNOTFNO`, which is far from
    /// every member. These are one character off an F&O name, or a real NSE
    /// or BSE index that merely starts like one (`NIFTYIT`, `NIFTY50`), so a
    /// prefix or truncated comparison in either arm would sweep them. Each
    /// premise is checked against `FNO_INDEX` first, and the exact names they
    /// surround are asserted to sweep, so the refusal is not a table that
    /// refuses everything.
    ///
    /// AF-52. "In any shape" is the whole product, 23 names x 2 exchanges x
    /// 3 segments x 4 kinds, 552 keys. This test first built only the cash
    /// and index keys, 2 of the 12 `(segment, kind)` shapes, so a third arm
    /// that swept `NIFTY50` as a future passed it, and the F&O walk below
    /// never asks a near miss. The two constructor keys are asserted to be
    /// among the 24 each name is asked in.
    #[test]
    fn a_near_miss_of_a_surface_name_is_never_sweepable_in_any_shape() {
        use crate::universe::FNO_INDEX;

        let near = [
            "FINNIFT",
            "FINNIFTYY",
            "NIFTYNXT5",
            "NIFTYNXT500",
            "MIDCPNIFTY-",
            "NIFTY50",
            "NIFTYIT",
            "INDIAVIX",
            "SENSEX",
            "BANKEX",
            "NIFT",
            "NIFTYY",
            "NIFTY_",
            "_NIFTY",
            "BANKNIFT",
            "BANKNIFTYY",
            "RELIANC",
            "RELIANCEE",
            "HINDALC0",
            "M&",
            "M&MM",
            "BAJAJAUTO",
            "BAJAJ_AUTO",
        ];
        let mut asked = 0_usize;
        for name in near {
            assert!(
                !FNO_INDEX.contains(name),
                "{name} must be OUTSIDE the F&O list, or this proves nothing"
            );
            let shapes = every_shape_of(Symbol::new(name).expect("valid shape"));
            for exchange in [Exchange::Nse, Exchange::Bse] {
                for built in [
                    InstrumentKey::cash(exchange, name).expect("valid shape"),
                    InstrumentKey::index(exchange, name).expect("valid shape"),
                ] {
                    assert!(
                        shapes.contains(&built),
                        "{built} is one of the shapes asked below"
                    );
                }
            }
            for key in shapes {
                assert!(!key.is_sweepable(), "{key:?} is a near miss");
                assert_eq!(
                    key.require_sweepable(),
                    Err(InstrumentError::NotSweepable),
                    "{key:?} is refused as storable but not sweepable"
                );
                asked += 1;
            }
        }
        assert_eq!(asked, 23 * 2 * 3 * 4, "every shape of every near miss");

        // THE NAMES THEY SURROUND DO SWEEP, each in its own shape only.
        for (name, as_cash, as_index) in [
            ("NIFTY", false, true),
            ("BANKNIFTY", false, true),
            ("RELIANCE", true, false),
            ("HINDALCO", true, false),
            ("M&M", true, false),
            ("BAJAJ-AUTO", true, false),
        ] {
            let cash = InstrumentKey::cash(Exchange::Nse, name).expect("valid");
            let index = InstrumentKey::index(Exchange::Nse, name).expect("valid");
            assert_eq!(cash.is_sweepable(), as_cash, "{name} as a cash equity");
            assert_eq!(index.is_sweepable(), as_index, "{name} as an index");
        }
    }

    /// AF-50. EVERY (EXCHANGE, SEGMENT, KIND) SHAPE OVER EVERY F&O NAME, ASKED
    /// THREE TIMES.
    ///
    /// The existing shape tests each pin a corner: `(Fno, Index)` and
    /// `(Index, Equity)` on NIFTY, BSE on NIFTY and HINDALCO, contracts on two
    /// names. None builds a `(Cash, Index)` key, and none walks the full
    /// product. This one does: 213 names x 2 exchanges x 3 segments x 4
    /// kinds, 5,112 keys. Exactly 210 sweep, the 208 shares as
    /// `(NSE, Cash, Equity)` and NIFTY and BANKNIFTY as `(NSE, Index, Index)`,
    /// and nothing else. `CLAUDE.md` section 3 rule 5: a key asked twice gets
    /// the same answer, and `require_sweepable` agrees with it every time.
    #[test]
    fn every_shape_over_every_fno_name_sweeps_only_the_two_surface_shapes() {
        use crate::universe::{FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS};

        let mut swept = Vec::new();
        let mut asked = 0_usize;
        for name in FNO_UNDERLYINGS {
            let underlying = Symbol::new(name).expect("every F&O name is a symbol");
            for key in every_shape_of(underlying) {
                let first = key.is_sweepable();
                assert_eq!(key.is_sweepable(), first, "{key} asked twice");
                assert_eq!(key.require_sweepable(), owed(first), "{key}");
                assert_eq!(key.is_sweepable(), first, "{key} asked again");
                asked += 1;
                if first {
                    swept.push((key.exchange, key.segment, key.kind, name));
                }
            }
        }
        assert_eq!(asked, 213 * 2 * 3 * 4, "the whole product was asked");

        // Stated from the lists, in the loop's name-major order.
        let mut expected = Vec::new();
        for name in FNO_UNDERLYINGS {
            if matches!(name, "NIFTY" | "BANKNIFTY") {
                expected.push((Exchange::Nse, Segment::Index, Kind::Index, name));
            }
            if !FNO_INDEX_UNDERLYINGS.contains(&name) {
                expected.push((Exchange::Nse, Segment::Cash, Kind::Equity, name));
            }
        }
        assert_eq!(swept.len(), 210, "208 shares and 2 indices");
        assert_eq!(swept, expected, "exactly the two surface shapes sweep");
    }

    /// Every date `Expiry::new` admits, 1990-01-01 through 2100-12-31, in
    /// chronological order.
    fn every_expiry() -> Vec<Expiry> {
        let mut all = Vec::with_capacity(111 * 366);
        for year in 1990..=2100 {
            for month in 1..=12 {
                for day in 1..=31 {
                    if let Ok(expiry) = Expiry::new(year, month, day) {
                        all.push(expiry);
                    }
                }
            }
        }
        all
    }

    /// The contract kinds the grid walk below asks, 4 + 4 x 6 x 2 = 52: a
    /// future at each of four expiries, and an option at each of those
    /// expiries, at each of six strikes, on both sides.
    ///
    /// The expiries are the two ends of what `Expiry::new` admits, a leap day,
    /// and `every_shape_of`'s own. The strikes are both ends of `i64`, zero and
    /// the values either side of it, and `every_shape_of`'s 22800.00. A key is
    /// judged without being rendered, so a strike too wide for a contract
    /// segment is still a key `is_sweepable` must answer.
    fn every_contract_kind() -> Vec<Kind> {
        let expiries = [
            Expiry::new(1990, 1, 1).expect("the first expiry admitted"),
            Expiry::new(2024, 2, 29).expect("a leap day"),
            Expiry::new(2026, 9, 29).expect("every_shape_of's expiry"),
            Expiry::new(2100, 12, 31).expect("the last expiry admitted"),
        ];
        let strikes = [i64::MIN, -1, 0, 1, 2_280_000, i64::MAX];
        let mut kinds = Vec::with_capacity(expiries.len() * (1 + strikes.len() * 2));
        for expiry in expiries {
            kinds.push(Kind::Future { expiry });
            for strike in strikes {
                for side in [OptionSide::Call, OptionSide::Put] {
                    kinds.push(Kind::Option {
                        expiry,
                        strike: Paisa::from_raw(strike),
                        side,
                    });
                }
            }
        }
        // Its 52 kinds are distinct, and half its options are puts.
        let mut distinct = kinds.clone();
        distinct.sort();
        distinct.dedup();
        assert_eq!(distinct.len(), 52, "4 futures and 48 options, none twice");
        let puts = kinds
            .iter()
            .filter(|kind| {
                matches!(
                    kind,
                    Kind::Option {
                        side: OptionSide::Put,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(puts, 24, "every option is asked on both sides");
        kinds
    }

    /// `kind` on `underlying` in each of the six (exchange, segment) pairs,
    /// exchange-major as `every_shape_of` orders them, whether or not the
    /// pair is one a contract is filed under.
    fn filed_everywhere(underlying: Symbol, kind: Kind) -> [InstrumentKey; 6] {
        [
            (Exchange::Nse, Segment::Index),
            (Exchange::Nse, Segment::Cash),
            (Exchange::Nse, Segment::Fno),
            (Exchange::Bse, Segment::Index),
            (Exchange::Bse, Segment::Cash),
            (Exchange::Bse, Segment::Fno),
        ]
        .map(|(exchange, segment)| InstrumentKey {
            exchange,
            segment,
            underlying,
            kind,
        })
    }

    /// Both answers a contract is owed: it is not sweepable, and
    /// `require_sweepable` refuses it as storable but not sweepable. A failure
    /// is reported at the caller's line, so it names the walk that found it.
    #[track_caller]
    fn assert_refused(key: InstrumentKey) {
        assert!(!key.is_sweepable(), "{key:?} is a contract");
        assert_eq!(
            key.require_sweepable(),
            Err(InstrumentError::NotSweepable),
            "{key:?} is refused as storable but not sweepable"
        );
    }

    /// AF-56. NO CONTRACT IS SWEEPABLE, WHATEVER ITS SIDE, EXPIRY OR STRIKE.
    ///
    /// `every_shape_of` builds one future and one option: expiry 2026-09-29,
    /// strike 22800.00, a call. AF-50 and AF-52 walk that product, so each
    /// contract shape was pinned by one value, and no test in this crate
    /// asked whether a put is sweepable. An arm that swept every F&O put, on
    /// either exchange and any underlying, passed the whole core suite.
    /// `CLAUDE.md` section 1 refuses every contract, not one of each shape.
    ///
    /// Three walks, each over keys whose answer is stated here and not read
    /// off `is_sweepable`. First, a put on NIFTY, BANKNIFTY and RELIANCE on
    /// NSE's F&O segment, beside the index and cash keys that do sweep, so
    /// the refusal is not a table that refuses everything. Second, the 52
    /// kinds of `every_contract_kind` in every (exchange, segment) of the 213
    /// F&O names and five outsiders, 68,016 keys. Third, every expiry
    /// `Expiry::new` admits, as a future and as an option on each side, in
    /// every (exchange, segment) of NIFTY, BANKNIFTY and RELIANCE, the names
    /// that reach the surface in each of its two shapes.
    ///
    /// Not walked: the strike beyond its six values. It is an `i64`.
    #[test]
    fn no_contract_is_sweepable_whatever_its_side_expiry_or_strike() {
        use crate::universe::{FNO_INDEX, FNO_UNDERLYINGS};

        // THE PUT, NAMED. The rendering proves each key is the put it claims.
        let expiry = Expiry::new(2026, 9, 29).expect("valid");
        let strike = Paisa::from_raw(2_280_000);
        for (name, surface, rendered) in [
            (
                "NIFTY",
                InstrumentKey::index(Exchange::Nse, "NIFTY").expect("valid"),
                "NSE-NIFTY-2026-09-29-2280000-PE",
            ),
            (
                "BANKNIFTY",
                InstrumentKey::index(Exchange::Nse, "BANKNIFTY").expect("valid"),
                "NSE-BANKNIFTY-2026-09-29-2280000-PE",
            ),
            (
                "RELIANCE",
                InstrumentKey::cash(Exchange::Nse, "RELIANCE").expect("valid"),
                "NSE-RELIANCE-2026-09-29-2280000-PE",
            ),
        ] {
            assert!(surface.is_sweepable(), "{surface} is on the surface");
            let put = InstrumentKey {
                exchange: Exchange::Nse,
                segment: Segment::Fno,
                underlying: Symbol::new(name).expect("valid"),
                kind: Kind::Option {
                    expiry,
                    strike,
                    side: OptionSide::Put,
                },
            };
            assert_eq!(put.to_string(), rendered);
            assert_refused(put);
        }

        // THE GRID, over every F&O name and five outsiders.
        let kinds = every_contract_kind();
        let outsiders = ["INDIAVIX", "SENSEX", "BANKEX", "NIFTY50", "ZZQXNOTFNO"];
        for name in outsiders {
            assert!(!FNO_INDEX.contains(name), "{name} is outside the F&O list");
        }
        let mut asked = 0_usize;
        for name in FNO_UNDERLYINGS.iter().chain(outsiders.iter()) {
            let underlying = Symbol::new(name).expect("valid");
            for &kind in &kinds {
                for key in filed_everywhere(underlying, kind) {
                    assert_refused(key);
                    asked += 1;
                }
            }
        }
        assert_eq!(asked, (213 + 5) * 2 * 3 * 52, "the whole grid was asked");

        // EVERY EXPIRY THE TYPE ADMITS. Its ends are the domain's ends, and it
        // holds 365 days a year for 111 years plus the 27 leap days, 2000 among
        // them and 2100 not.
        let expiries = every_expiry();
        assert_eq!(expiries.len(), 111 * 365 + 27, "every admitted date");
        assert_eq!(
            expiries.first().map(ToString::to_string).as_deref(),
            Some("1990-01-01")
        );
        assert_eq!(
            expiries.last().map(ToString::to_string).as_deref(),
            Some("2100-12-31")
        );
        assert!(Expiry::new(1989, 12, 31).is_err() && Expiry::new(2101, 1, 1).is_err());
        let mut asked = 0_usize;
        for name in ["NIFTY", "BANKNIFTY", "RELIANCE"] {
            let underlying = Symbol::new(name).expect("valid");
            for &expiry in &expiries {
                for kind in [
                    Kind::Future { expiry },
                    Kind::Option {
                        expiry,
                        strike,
                        side: OptionSide::Call,
                    },
                    Kind::Option {
                        expiry,
                        strike,
                        side: OptionSide::Put,
                    },
                ] {
                    for key in filed_everywhere(underlying, kind) {
                        assert_refused(key);
                        asked += 1;
                    }
                }
            }
        }
        assert_eq!(asked, 3 * expiries.len() * 3 * 2 * 3, "every expiry");
    }
}
